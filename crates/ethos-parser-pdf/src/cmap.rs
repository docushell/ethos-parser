// Copyright 2026 The ethos-parser maintainers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! `ToUnicode` CMap parsing (PDF 32000-1 §9.10.3).
//!
//! A `ToUnicode` CMap is **embedded in the document**, so it is authoritative and needs no
//! vendored data. It maps character codes to Unicode, and critically it may map **one code to
//! several scalars** — which is where the ligature caveat comes from.
//!
//! Vendored predefined CMaps (the Adobe CJK set) are a separate question; see
//! [`crate::encoding`] for what this profile does and does not carry, and note that a document
//! naming a predefined CMap **fails closed** rather than guessing.

use std::collections::BTreeMap;

use ethos_parser_core::EngineError;

/// A parsed `ToUnicode` mapping.
///
/// Keyed by character code. Values are full scalar sequences, not single characters: `<03>` →
/// `<00660069>` is one code producing `fi`, and flattening that to one character would silently
/// lose a letter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToUnicode {
    map: BTreeMap<u32, String>,
    /// Byte width of a character code, from the codespace range. Almost always 1 or 2.
    code_bytes: usize,
}

impl ToUnicode {
    /// Look up one code.
    pub fn get(&self, code: u32) -> Option<&str> {
        self.map.get(&code).map(String::as_str)
    }

    /// How many bytes make up one character code.
    pub fn code_bytes(&self) -> usize {
        self.code_bytes
    }

    /// Number of mappings. Test-only.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the map is empty. Test-only.
    ///
    /// Both this and [`ToUnicode::len`] became `#[cfg(test)]` at M7, when `cmap` stopped being a
    /// public module: nothing outside the tests reads either, and a private module's unused
    /// accessor is dead code the compiler was previously unable to see.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Parse a `ToUnicode` CMap stream.
    ///
    /// Handles `begincodespacerange`, `beginbfchar` and `beginbfrange` — the three constructs
    /// PDF 32000-1 §9.10.3 defines for this purpose.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] when a hex string is unparseable or a range is inverted.
    /// Refusing beats guessing: a mis-parsed `ToUnicode` produces text that looks right and is
    /// wrong, which is worse than no text at all.
    pub fn parse(bytes: &[u8]) -> Result<Self, EngineError> {
        let src = String::from_utf8_lossy(bytes);
        let tokens = tokenize(&src);

        let mut map = BTreeMap::new();
        let mut code_bytes = 1usize;
        // Every `bfchar` entry and every code a `bfrange` covers, counted against one ceiling.
        // The per-range cap below bounds one row; this bounds the rows.
        const MAX_MAPPINGS: u64 = 1 << 20;
        let mut mapped: u64 = 0;
        let mut spend = |codes: u64| -> Result<(), EngineError> {
            mapped = mapped.saturating_add(codes);
            if mapped > MAX_MAPPINGS {
                return Err(EngineError::ResourceLimit {
                    limit: "ToUnicode mappings".into(),
                    configured: MAX_MAPPINGS.to_string(),
                });
            }
            Ok(())
        };
        let mut i = 0;

        while i < tokens.len() {
            match tokens[i].as_str() {
                "begincodespacerange" => {
                    i += 1;
                    while i < tokens.len() && tokens[i] != "endcodespacerange" {
                        if let Some(hex) = hex_of(&tokens[i]) {
                            // Two hex digits per byte.
                            code_bytes = code_bytes.max(hex.len().div_ceil(2));
                        }
                        i += 1;
                    }
                }
                "beginbfchar" => {
                    i += 1;
                    while i < tokens.len() && tokens[i] != "endbfchar" {
                        let (Some(src_hex), Some(dst_hex)) = (
                            hex_of(&tokens[i]),
                            tokens.get(i + 1).and_then(|t| hex_of(t)),
                        ) else {
                            return Err(malformed("bfchar entry is not a pair of hex strings"));
                        };
                        let code = hex_to_u32(&src_hex)?;
                        spend(1)?;
                        map.insert(code, utf16be_hex_to_string(&dst_hex)?);
                        i += 2;
                    }
                }
                "beginbfrange" => {
                    i += 1;
                    while i < tokens.len() && tokens[i] != "endbfrange" {
                        let (Some(lo_hex), Some(hi_hex)) = (
                            hex_of(&tokens[i]),
                            tokens.get(i + 1).and_then(|t| hex_of(t)),
                        ) else {
                            return Err(malformed(
                                "bfrange entry does not start with two hex strings",
                            ));
                        };
                        let lo = hex_to_u32(&lo_hex)?;
                        let hi = hex_to_u32(&hi_hex)?;
                        if hi < lo {
                            return Err(malformed("bfrange upper bound is below its lower bound"));
                        }
                        // Guard against a hostile range exhausting memory before it exhausts
                        // patience. 65536 is the whole two-byte code space.
                        if hi - lo > 65_536 {
                            return Err(EngineError::ResourceLimit {
                                limit: "ToUnicode bfrange span".into(),
                                configured: "65536".into(),
                            });
                        }
                        spend(u64::from(hi - lo) + 1)?;

                        match tokens.get(i + 2) {
                            Some(t) if t == "[" => {
                                // Per-code destinations, one array element each.
                                let mut offset: u32 = 0;
                                let mut j = i + 3;
                                while j < tokens.len() && tokens[j] != "]" {
                                    let code = lo
                                        .checked_add(offset)
                                        .filter(|c| *c <= hi)
                                        .ok_or_else(|| {
                                            malformed(
                                                "bfrange array holds more destinations than its \
                                                 range has codes",
                                            )
                                        })?;
                                    let Some(h) = hex_of(&tokens[j]) else {
                                        return Err(malformed(
                                            "bfrange array holds a non-hex entry",
                                        ));
                                    };
                                    map.insert(code, utf16be_hex_to_string(&h)?);
                                    // At most `hi - lo + 1` ≤ 65 537 elements pass the check above.
                                    offset += 1;
                                    j += 1;
                                }
                                i = j + 1;
                            }
                            Some(t) => {
                                let Some(dst_hex) = hex_of(t) else {
                                    return Err(malformed(
                                        "bfrange destination is not a hex string",
                                    ));
                                };
                                let base = utf16be_hex_to_string(&dst_hex)?;
                                // The destination increments with the code, on its last scalar.
                                for (n, code) in (lo..=hi).enumerate() {
                                    map.insert(code, increment_last_scalar(&base, n as u32)?);
                                }
                                i += 3;
                            }
                            None => return Err(malformed("bfrange entry is truncated")),
                        }
                    }
                }
                _ => i += 1,
            }
        }

        Ok(Self { map, code_bytes })
    }
}

fn malformed(detail: &str) -> EngineError {
    EngineError::Malformed {
        what: "ToUnicode CMap".into(),
        detail: detail.to_string(),
    }
}

/// Split a CMap program into tokens, keeping `<...>` hex strings and `[`/`]` intact.
fn tokenize(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '<' => {
                let mut hex = String::from("<");
                for c in chars.by_ref() {
                    hex.push(c);
                    if c == '>' {
                        break;
                    }
                }
                out.push(hex);
            }
            '[' | ']' => out.push(c.to_string()),
            // A PostScript comment runs to the end of its line and is not program text.
            '%' => {
                for c in chars.by_ref() {
                    if c == '\n' || c == '\r' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            _ => {
                let mut word = String::from(c);
                while let Some(&n) = chars.peek() {
                    if n.is_whitespace() || n == '<' || n == '[' || n == ']' || n == '%' {
                        break;
                    }
                    word.push(n);
                    chars.next();
                }
                out.push(word);
            }
        }
    }
    out
}

/// Strip the angle brackets from a hex-string token.
/// The hex digits of a `<...>` token, or `None` when it is not a hexadecimal string.
///
/// **White space inside the angle brackets is ignored, because PDF 32000-1 §7.3.4.3 says it
/// shall be.** Reading it as a syntax error is not a stricter reading of the specification, it is
/// a wrong one — and it was document-fatal: `extract`'s page fold returns the first page error, so
/// one stray space in one font's `ToUnicode` CMap cost the whole page. Two real documents in 981
/// hit it, `<0009 000d 0020 00a0>` in a `bfchar` and the `bfrange` array form
/// `[<0066 0066 006C>...]`, and one of them carries 4 165 bytes of text and a table.
///
/// PDF white space is the six characters of Table 1 — NUL, TAB, LF, FF, CR and SP — which is
/// `char::is_ascii_whitespace` plus NUL.
fn hex_of(token: &str) -> Option<String> {
    let s = token.strip_prefix('<')?.strip_suffix('>')?;
    let digits: String = s
        .chars()
        .filter(|c| !(c.is_ascii_whitespace() || *c == '\0'))
        .collect();
    digits
        .chars()
        .all(|c| c.is_ascii_hexdigit())
        .then_some(digits)
}

fn hex_to_u32(hex: &str) -> Result<u32, EngineError> {
    if hex.is_empty() || hex.len() > 8 {
        return Err(malformed(
            "character code is empty or wider than four bytes",
        ));
    }
    u32::from_str_radix(hex, 16).map_err(|_| malformed("character code is not hexadecimal"))
}

/// Decode a big-endian UTF-16 hex string into a Rust string.
///
/// This is where one code becomes several scalars: `00660069` is `f` then `i`.
///
/// **An empty destination is accepted and decodes to no character**, which is the document
/// saying this code carries no text. Decided by measurement on 2026-09-18 rather than refused:
/// over the 311 PDFs of every corpus this repository can reach — 40 339 `bfchar` entries and
/// 2 806 `bfrange` rows — **not one destination is empty**, while 698 destinations in 95
/// documents carry several scalars. Refusing would drop a whole run for a code the document
/// chose to give no text, and dropping text that decodes is the larger loss.
///
/// What it costs, stated because nothing else would say it: `TextRunAttributes`'s
/// `scalar_code_mismatch` compares two counts, so an empty destination offsets a code that
/// decodes to two scalars and the flag reads false on a run whose codes are not 1:1 with its
/// characters. That field's own documentation says it is a comparison of counts and not a
/// mapping; `the_empty_destination_offsets_a_ligature` in `extraction.rs` pins the case.
fn utf16be_hex_to_string(hex: &str) -> Result<String, EngineError> {
    if hex.len() % 4 != 0 {
        return Err(malformed(
            "ToUnicode destination is not a whole number of UTF-16 code units",
        ));
    }
    let units: Vec<u16> = (0..hex.len() / 4)
        .map(|i| u16::from_str_radix(&hex[i * 4..i * 4 + 4], 16))
        .collect::<Result<_, _>>()
        .map_err(|_| malformed("ToUnicode destination is not hexadecimal"))?;

    String::from_utf16(&units).map_err(|_| malformed("ToUnicode destination is not valid UTF-16"))
}

/// Add `n` to the final scalar of `base`, as `bfrange` requires.
fn increment_last_scalar(base: &str, n: u32) -> Result<String, EngineError> {
    if n == 0 {
        return Ok(base.to_string());
    }
    let mut chars: Vec<char> = base.chars().collect();
    let Some(last) = chars.pop() else {
        return Err(malformed("bfrange destination is empty"));
    };
    let next = char::from_u32(last as u32 + n)
        .ok_or_else(|| malformed("bfrange destination runs past the Unicode range"))?;
    chars.push(next);
    Ok(chars.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real `ToUnicode` from `synthetic/ligature-fi-embedded-font`.
    const LIGATURE_CMAP: &str = r"/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /EthosLigatureFiType3 def
/CMapType 2 def
1 begincodespacerange
<01> <07>
endcodespacerange
7 beginbfchar
<01> <006F>
<02> <0066>
<03> <00660069>
<04> <0063>
<05> <0065>
<06> <0020>
<07> <006C>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end";

    #[test]
    fn the_ligature_fixture_cmap_parses() {
        let m = ToUnicode::parse(LIGATURE_CMAP.as_bytes()).expect("parses");
        assert_eq!(m.len(), 7);
        assert_eq!(m.code_bytes(), 1);
        assert_eq!(m.get(0x01), Some("o"));
        assert_eq!(m.get(0x02), Some("f"));
        assert_eq!(m.get(0x04), Some("c"));
        assert_eq!(m.get(0x05), Some("e"));
        assert_eq!(m.get(0x06), Some(" "));
        assert_eq!(m.get(0x07), Some("l"));
    }

    /// **An empty destination decodes to no character, and is not an error.** The parser's
    /// own end of the case `extraction.rs` pins end to end: the code is in the map, and it
    /// answers with the empty string rather than being absent, so the run keeps the code in
    /// `char_codes` and gains no character for it.
    #[test]
    fn an_empty_destination_decodes_to_no_character() {
        let src = b"begincmap\nbeginbfchar\n<41><>\n<42><00660069>\nendbfchar\nendcmap";
        let m = ToUnicode::parse(src).expect("an empty destination is legal here");
        assert_eq!(m.get(0x41), Some(""), "in the map, and empty");
        assert_eq!(m.get(0x42), Some("fi"), "beside a code that decodes to two");
        assert_eq!(m.len(), 2);
    }

    /// **White space inside a hex string is ignored, because §7.3.4.3 says it shall be.**
    ///
    /// Both forms occur in real documents and both were document-fatal before v2.2-S6: a
    /// `bfchar` destination written `<0009 000d 0020 00a0>`, and the `bfrange` array form whose
    /// entries carry internal spaces. `extract`'s page fold returns the first page error, so one
    /// space cost the whole page.
    #[test]
    fn white_space_inside_a_hex_string_is_ignored() {
        let src = b"begincmap\n\
            beginbfchar\n<29><0009 000d 0020 00a0>\nendbfchar\n\
            beginbfrange\n<1a> <1c> [<0066 0066 006C><0066 006C><0066 0069>]\nendbfrange\n\
            endcmap";
        let m = ToUnicode::parse(src).expect("white space in a hex string is legal");
        assert_eq!(m.get(0x29), Some("\u{9}\u{d}\u{20}\u{a0}"));
        assert_eq!(m.get(0x1a), Some("ffl"));
        assert_eq!(m.get(0x1b), Some("fl"));
        assert_eq!(m.get(0x1c), Some("fi"));
    }

    /// **A comment is not a mapping** (review 2026-09-26 N38). PostScript's `%` runs to the end of
    /// its line. Read as program text, a comment after an entry failed a legal CMap, and a
    /// commented-out block remapped `<01>` to `X` where Ghostscript draws `A`.
    #[test]
    fn a_comment_is_not_a_mapping() {
        let src = b"begincmap\nbeginbfchar\n<01> <0041> % capital A\nendbfchar\n\
            % beginbfchar <01> <0058> endbfchar\nendcmap";
        let m = ToUnicode::parse(src).expect("a comment is legal");
        assert_eq!(m.get(0x01), Some("A"));
    }

    /// **A genuinely non-hex entry is still refused.** Ignoring white space must not become
    /// ignoring the check.
    #[test]
    fn a_non_hex_entry_is_still_refused_after_white_space_is_stripped() {
        let src = b"begincmap\nbeginbfchar\n<29><00zz>\nendbfchar\nendcmap";
        assert!(ToUnicode::parse(src).is_err());
    }

    /// **Every mapping counts against one ceiling.** The span cap bounds one `bfrange` row, and
    /// nothing bounded the rows: seventeen of 65,536 codes each pass it, and together they are
    /// past 2^20.
    #[test]
    fn mappings_past_the_ceiling_are_refused() {
        let rows = "<0000> <FFFF> <D800DC00>\n".repeat(17);
        let src = format!("begincmap\nbeginbfrange\n{rows}endbfrange\nendcmap");
        let e = ToUnicode::parse(src.as_bytes()).expect_err("refused at the ceiling");
        assert_eq!(e.code(), "resource_limit", "{e}");
        assert!(e.to_string().contains("ToUnicode mappings"), "{e}");
    }

    /// **A `bfrange` array maps only the codes its range names.** An element past `hi` mapped
    /// the code after it, and one past `0xFFFFFFFF` wrapped to code 0 in a release build.
    #[test]
    fn a_bfrange_array_maps_no_code_past_its_range() {
        for row in [
            "<01> <02> [<0041> <0042> <0043>]",
            "<FFFFFFFF> <FFFFFFFF> [<0041> <0042>]",
        ] {
            let src = format!("begincmap\nbeginbfrange\n{row}\nendbfrange\nendcmap");
            let e = ToUnicode::parse(src.as_bytes()).expect_err(row);
            assert_eq!(e.code(), "malformed", "{e}");
            assert!(
                e.to_string()
                    .contains("more destinations than its range has codes"),
                "{e}"
            );
        }
    }

    /// One code, two scalars. This is the ligature caveat at its source.
    #[test]
    fn one_code_can_map_to_several_scalars() {
        let m = ToUnicode::parse(LIGATURE_CMAP.as_bytes()).unwrap();
        let fi = m.get(0x03).expect("code 3 is mapped");
        assert_eq!(fi, "fi");
        assert_eq!(
            fi.chars().count(),
            2,
            "the fi ligature is one glyph and two characters; flattening it to one would lose a \
             letter"
        );
    }

    #[test]
    fn bfrange_with_a_base_destination_increments() {
        let src = "begincmap
1 begincodespacerange <00> <ff> endcodespacerange
1 beginbfrange
<41> <45> <0061>
endbfrange
endcmap";
        let m = ToUnicode::parse(src.as_bytes()).unwrap();
        assert_eq!(m.get(0x41), Some("a"));
        assert_eq!(m.get(0x42), Some("b"));
        assert_eq!(m.get(0x45), Some("e"));
        assert_eq!(m.get(0x46), None);
    }

    #[test]
    fn bfrange_with_an_array_maps_each_code_separately() {
        let src = "begincmap
1 beginbfrange
<01> <03> [<0041> <00420043> <0044>]
endbfrange
endcmap";
        let m = ToUnicode::parse(src.as_bytes()).unwrap();
        assert_eq!(m.get(0x01), Some("A"));
        assert_eq!(
            m.get(0x02),
            Some("BC"),
            "an array entry may also be multi-scalar"
        );
        assert_eq!(m.get(0x03), Some("D"));
    }

    #[test]
    fn two_byte_codespace_is_detected() {
        let src = "begincmap
1 begincodespacerange <0000> <ffff> endcodespacerange
1 beginbfchar <0041> <0061> endbfchar
endcmap";
        let m = ToUnicode::parse(src.as_bytes()).unwrap();
        assert_eq!(m.code_bytes(), 2);
        assert_eq!(m.get(0x0041), Some("a"));
    }

    #[test]
    fn malformed_input_is_refused_not_guessed() {
        // Odd-length UTF-16 destination.
        assert!(ToUnicode::parse(b"beginbfchar <01> <006> endbfchar").is_err());
        // Non-hex destination.
        assert!(ToUnicode::parse(b"beginbfchar <01> <zzzz> endbfchar").is_err());
        // Inverted range.
        assert!(ToUnicode::parse(b"beginbfrange <45> <41> <0061> endbfrange").is_err());
        // Truncated pair.
        assert!(ToUnicode::parse(b"beginbfchar <01> endbfchar").is_err());
    }

    #[test]
    fn a_hostile_range_hits_a_resource_limit_rather_than_memory() {
        let src = "beginbfrange <00000000> <00ffffff> <0041> endbfrange";
        let e = ToUnicode::parse(src.as_bytes()).unwrap_err();
        assert_eq!(e.code(), "resource_limit");
    }

    #[test]
    fn an_empty_cmap_is_empty_not_an_error() {
        let m = ToUnicode::parse(b"begincmap endcmap").unwrap();
        assert!(m.is_empty());
    }
}
