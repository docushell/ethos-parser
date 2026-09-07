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

//! Simple-font encodings (PDF 32000-1 Annex D) and the `/Differences` mechanism.
//!
//! # What is vendored, and what is not
//!
//! **Vendored here, and *here* means this file.** `WinAnsiEncoding` (Windows-1252) and the ASCII
//! range of `StandardEncoding`, plus a small glyph-name table for `/Differences`, are `const fn`
//! builders a few hundred lines below — `build_win_ansi` and `build_standard` — which the
//! compiler evaluates into `.rodata`. Nothing is read from disk and the build never touches the
//! network.
//!
//! There is no `vendor/encodings/` directory, and there never has been. This paragraph said the
//! tables were *"written out as data under `vendor/encodings/`"* until v2-S13.3, and the doc on
//! `WIN_ANSI` said the same thing in fewer words. `vendor/README.md` has been right about it the
//! whole time and says why: a 256-entry lookup table in a separate file is the same bytes with a
//! parser in front. `vendor/` holds one tracked file, that README.
//!
//! **Not vendored:** the Adobe predefined CJK CMaps (`UniJIS-UCS2-H` and its ~167 siblings) and
//! the full Adobe Glyph List. Neither is decoded approximately — `docs/01-CONTRACT.md` §8, because
//! approximate text is worse than no text here: a citation can be verified against it and appear
//! to hold. What that costs a document differs by which one it needs, and this paragraph said
//! *"refused with a named error"* of both until v2-S13.3. A document naming a predefined CMap is
//! genuinely **refused**. A `/Differences` name outside the subset drops **that run** and is
//! counted into `broken-font-encoding`; only a document that decodes nothing is refused outright,
//! which is v0.1's decision that failing a whole document over one glyph was more than the
//! evidence required.
//!
//! This is a declared limitation, not an oversight. It reaches a consumer as the limitation code
//! `predefined-cmaps-not-vendored` in `assurance.limitations`, so the gap is stated rather than
//! inferred from an absence. It used to say `not_decoded`, which was the M3 spelling: M4 absorbed
//! that list into the L1 gate and no artifact has carried the field since.
//!
//! # Precedence
//!
//! A font's `ToUnicode` CMap, when present, is **authoritative** and overrides everything here:
//! it ships inside the document and describes that document's actual mapping. These tables are
//! the fallback for fonts without one.

use std::collections::BTreeMap;

use ethos_parser_core::EngineError;

/// A simple font's base encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseEncoding {
    /// `StandardEncoding`, PDF 32000-1 Annex D.2. Only its ASCII range is carried.
    Standard,
    /// `WinAnsiEncoding`, Annex D.2 — Windows-1252. Carried in full.
    WinAnsi,
    /// `MacRomanEncoding`. **Not carried**; codes above ASCII fail closed.
    MacRoman,
    /// The font's built-in encoding, with no base named in the document.
    ///
    /// Treated as [`Self::Standard`] for simple fonts, which is what PDF 32000-1 §9.6.6.2
    /// specifies when neither the font program nor the dictionary supplies one.
    Builtin,
}

impl BaseEncoding {
    /// Resolve a `/BaseEncoding` or `/Encoding` name.
    pub fn from_name(name: &[u8]) -> Option<Self> {
        match name {
            b"WinAnsiEncoding" => Some(Self::WinAnsi),
            b"MacRomanEncoding" => Some(Self::MacRoman),
            b"StandardEncoding" => Some(Self::Standard),
            _ => None,
        }
    }
}

/// A code-to-text mapping for one simple font.
#[derive(Debug, Clone)]
pub struct SimpleEncoding {
    base: BaseEncoding,
    /// `/Differences` overrides, code → glyph name.
    differences: BTreeMap<u8, String>,
}

impl SimpleEncoding {
    /// Build from a base encoding and an optional `/Differences` array.
    pub fn new(base: BaseEncoding, differences: BTreeMap<u8, String>) -> Self {
        Self { base, differences }
    }

    /// The base this encoding resolves through.
    ///
    /// Read by `load_font` for one question only: whether the document NAMED a base, or whether
    /// [`BaseEncoding::Builtin`] is standing in for one it never supplied.
    pub fn base(&self) -> BaseEncoding {
        self.base
    }

    /// The glyph NAME this encoding gives a code, where this profile knows one.
    ///
    /// `/Differences` wins, exactly as it does for decoding — it is the document overriding its
    /// own base. Otherwise only `WinAnsiEncoding` answers, from the derived table in
    /// [`crate::winansi_names`]. `StandardEncoding` deliberately does not: an AFM's own `C`
    /// column IS StandardEncoding, so [`crate::afm`] already reaches those widths by code and a
    /// second route to the same number could only disagree with the first.
    pub(crate) fn glyph_name(&self, code: u8) -> Option<&str> {
        if let Some(name) = self.differences.get(&code) {
            return Some(name);
        }
        match self.base {
            BaseEncoding::WinAnsi => crate::winansi_names::WIN_ANSI_NAMES[code as usize],
            BaseEncoding::Standard | BaseEncoding::Builtin | BaseEncoding::MacRoman => None,
        }
    }

    /// Decode one byte.
    ///
    /// # Errors
    ///
    /// [`EngineError::Unsupported`] when this profile has no mapping for the code — an unknown
    /// glyph name in `/Differences`, or a high code in an encoding whose table is not carried.
    /// **Refused, never substituted**: emitting U+FFFD would put a character in the evidence that
    /// the document does not contain.
    pub fn decode(&self, code: u8) -> Result<&'static str, EngineError> {
        if let Some(name) = self.differences.get(&code) {
            return glyph_name_to_str(name).ok_or_else(|| EngineError::Unsupported {
                what: "glyph name".into(),
                detail: format!(
                    "/Differences maps code {code} to /{name}, which is not in this profile's \
                     glyph table. The full Adobe Glyph List is not vendored; a document needing \
                     it is refused rather than decoded approximately."
                ),
            });
        }

        let table = match self.base {
            BaseEncoding::WinAnsi => WIN_ANSI,
            BaseEncoding::Standard | BaseEncoding::Builtin => STANDARD,
            BaseEncoding::MacRoman => {
                if code < 0x80 {
                    STANDARD
                } else {
                    return Err(EngineError::Unsupported {
                        what: "encoding".into(),
                        detail: format!(
                            "MacRomanEncoding code {code} is above ASCII, and that table is not \
                             vendored in this profile"
                        ),
                    });
                }
            }
        };

        table[code as usize].ok_or_else(|| EngineError::Unsupported {
            what: "encoding".into(),
            detail: format!(
                "code {code} has no mapping in {:?}Encoding in this profile's tables",
                self.base
            ),
        })
    }
}

/// `WinAnsiEncoding` — Windows-1252, built at compile time by `build_win_ansi` in this file.
static WIN_ANSI: &[Option<&'static str>; 256] = &build_win_ansi();

/// `StandardEncoding`, ASCII range only.
static STANDARD: &[Option<&'static str>; 256] = &build_standard();

/// What `WinAnsiEncoding` says a code means, or `None` where the table carries nothing.
///
/// Read by [`crate::afm`]'s tests to prove the derived glyph-name table is populated at exactly
/// the codes this one is — a check with no external source in it. Test-only for that reason: the
/// reader itself goes through [`SimpleEncoding::decode`], which applies `/Differences` first.
#[cfg(test)]
pub(crate) fn win_ansi_code_to_str(code: u8) -> Option<&'static str> {
    WIN_ANSI[code as usize]
}

/// What `StandardEncoding` says a code means, or `None` where this profile does not carry it.
///
/// Read by [`crate::afm`] for one job: an AFM's `C` column is a `StandardEncoding` code, so this
/// turns a vendored metric into the text a decoder would produce for it. The high range is `None`
/// here, which is why standard-14 width recovery stops at ASCII.
pub(crate) fn standard_code_to_str(code: u8) -> Option<&'static str> {
    STANDARD[code as usize]
}

const fn empty_table() -> [Option<&'static str>; 256] {
    [None; 256]
}

/// Windows-1252, written out.
///
/// The 0x20–0x7E range is ASCII. 0x80–0x9F are the CP1252 additions. 0xA0–0xFF is Latin-1.
const fn build_win_ansi() -> [Option<&'static str>; 256] {
    let mut t = empty_table();
    // ASCII 0x20..=0x7E is identity; filled at runtime-equivalent const time below.
    t[0x20] = Some(" ");
    t[0x21] = Some("!");
    t[0x22] = Some("\"");
    t[0x23] = Some("#");
    t[0x24] = Some("$");
    t[0x25] = Some("%");
    t[0x26] = Some("&");
    t[0x27] = Some("'");
    t[0x28] = Some("(");
    t[0x29] = Some(")");
    t[0x2A] = Some("*");
    t[0x2B] = Some("+");
    t[0x2C] = Some(",");
    t[0x2D] = Some("-");
    t[0x2E] = Some(".");
    t[0x2F] = Some("/");
    t[0x30] = Some("0");
    t[0x31] = Some("1");
    t[0x32] = Some("2");
    t[0x33] = Some("3");
    t[0x34] = Some("4");
    t[0x35] = Some("5");
    t[0x36] = Some("6");
    t[0x37] = Some("7");
    t[0x38] = Some("8");
    t[0x39] = Some("9");
    t[0x3A] = Some(":");
    t[0x3B] = Some(";");
    t[0x3C] = Some("<");
    t[0x3D] = Some("=");
    t[0x3E] = Some(">");
    t[0x3F] = Some("?");
    t[0x40] = Some("@");
    t[0x41] = Some("A");
    t[0x42] = Some("B");
    t[0x43] = Some("C");
    t[0x44] = Some("D");
    t[0x45] = Some("E");
    t[0x46] = Some("F");
    t[0x47] = Some("G");
    t[0x48] = Some("H");
    t[0x49] = Some("I");
    t[0x4A] = Some("J");
    t[0x4B] = Some("K");
    t[0x4C] = Some("L");
    t[0x4D] = Some("M");
    t[0x4E] = Some("N");
    t[0x4F] = Some("O");
    t[0x50] = Some("P");
    t[0x51] = Some("Q");
    t[0x52] = Some("R");
    t[0x53] = Some("S");
    t[0x54] = Some("T");
    t[0x55] = Some("U");
    t[0x56] = Some("V");
    t[0x57] = Some("W");
    t[0x58] = Some("X");
    t[0x59] = Some("Y");
    t[0x5A] = Some("Z");
    t[0x5B] = Some("[");
    t[0x5C] = Some("\\");
    t[0x5D] = Some("]");
    t[0x5E] = Some("^");
    t[0x5F] = Some("_");
    t[0x60] = Some("`");
    t[0x61] = Some("a");
    t[0x62] = Some("b");
    t[0x63] = Some("c");
    t[0x64] = Some("d");
    t[0x65] = Some("e");
    t[0x66] = Some("f");
    t[0x67] = Some("g");
    t[0x68] = Some("h");
    t[0x69] = Some("i");
    t[0x6A] = Some("j");
    t[0x6B] = Some("k");
    t[0x6C] = Some("l");
    t[0x6D] = Some("m");
    t[0x6E] = Some("n");
    t[0x6F] = Some("o");
    t[0x70] = Some("p");
    t[0x71] = Some("q");
    t[0x72] = Some("r");
    t[0x73] = Some("s");
    t[0x74] = Some("t");
    t[0x75] = Some("u");
    t[0x76] = Some("v");
    t[0x77] = Some("w");
    t[0x78] = Some("x");
    t[0x79] = Some("y");
    t[0x7A] = Some("z");
    t[0x7B] = Some("{");
    t[0x7C] = Some("|");
    t[0x7D] = Some("}");
    t[0x7E] = Some("~");

    // CP1252 additions in 0x80..=0x9F.
    t[0x80] = Some("\u{20AC}"); // euro
    t[0x82] = Some("\u{201A}");
    t[0x83] = Some("\u{0192}");
    t[0x84] = Some("\u{201E}");
    t[0x85] = Some("\u{2026}");
    t[0x86] = Some("\u{2020}");
    t[0x87] = Some("\u{2021}");
    t[0x88] = Some("\u{02C6}");
    t[0x89] = Some("\u{2030}");
    t[0x8A] = Some("\u{0160}");
    t[0x8B] = Some("\u{2039}");
    t[0x8C] = Some("\u{0152}");
    t[0x8E] = Some("\u{017D}");
    t[0x91] = Some("\u{2018}");
    t[0x92] = Some("\u{2019}");
    t[0x93] = Some("\u{201C}");
    t[0x94] = Some("\u{201D}");
    t[0x95] = Some("\u{2022}");
    t[0x96] = Some("\u{2013}");
    t[0x97] = Some("\u{2014}");
    t[0x98] = Some("\u{02DC}");
    t[0x99] = Some("\u{2122}");
    t[0x9A] = Some("\u{0161}");
    t[0x9B] = Some("\u{203A}");
    t[0x9C] = Some("\u{0153}");
    t[0x9E] = Some("\u{017E}");
    t[0x9F] = Some("\u{0178}");

    // 0xA0..=0xFF is Latin-1, i.e. the code point equals the byte value. Written out because a
    // const fn cannot build a &'static str from a computed char.
    t[0xA0] = Some("\u{00A0}");
    t[0xA1] = Some("\u{00A1}");
    t[0xA2] = Some("\u{00A2}");
    t[0xA3] = Some("\u{00A3}");
    t[0xA4] = Some("\u{00A4}");
    t[0xA5] = Some("\u{00A5}");
    t[0xA6] = Some("\u{00A6}");
    t[0xA7] = Some("\u{00A7}");
    t[0xA8] = Some("\u{00A8}");
    t[0xA9] = Some("\u{00A9}");
    t[0xAA] = Some("\u{00AA}");
    t[0xAB] = Some("\u{00AB}");
    t[0xAC] = Some("\u{00AC}");
    t[0xAD] = Some("\u{00AD}");
    t[0xAE] = Some("\u{00AE}");
    t[0xAF] = Some("\u{00AF}");
    t[0xB0] = Some("\u{00B0}");
    t[0xB1] = Some("\u{00B1}");
    t[0xB2] = Some("\u{00B2}");
    t[0xB3] = Some("\u{00B3}");
    t[0xB4] = Some("\u{00B4}");
    t[0xB5] = Some("\u{00B5}");
    t[0xB6] = Some("\u{00B6}");
    t[0xB7] = Some("\u{00B7}");
    t[0xB8] = Some("\u{00B8}");
    t[0xB9] = Some("\u{00B9}");
    t[0xBA] = Some("\u{00BA}");
    t[0xBB] = Some("\u{00BB}");
    t[0xBC] = Some("\u{00BC}");
    t[0xBD] = Some("\u{00BD}");
    t[0xBE] = Some("\u{00BE}");
    t[0xBF] = Some("\u{00BF}");
    t[0xC0] = Some("\u{00C0}");
    t[0xC1] = Some("\u{00C1}");
    t[0xC2] = Some("\u{00C2}");
    t[0xC3] = Some("\u{00C3}");
    t[0xC4] = Some("\u{00C4}");
    t[0xC5] = Some("\u{00C5}");
    t[0xC6] = Some("\u{00C6}");
    t[0xC7] = Some("\u{00C7}");
    t[0xC8] = Some("\u{00C8}");
    t[0xC9] = Some("\u{00C9}");
    t[0xCA] = Some("\u{00CA}");
    t[0xCB] = Some("\u{00CB}");
    t[0xCC] = Some("\u{00CC}");
    t[0xCD] = Some("\u{00CD}");
    t[0xCE] = Some("\u{00CE}");
    t[0xCF] = Some("\u{00CF}");
    t[0xD0] = Some("\u{00D0}");
    t[0xD1] = Some("\u{00D1}");
    t[0xD2] = Some("\u{00D2}");
    t[0xD3] = Some("\u{00D3}");
    t[0xD4] = Some("\u{00D4}");
    t[0xD5] = Some("\u{00D5}");
    t[0xD6] = Some("\u{00D6}");
    t[0xD7] = Some("\u{00D7}");
    t[0xD8] = Some("\u{00D8}");
    t[0xD9] = Some("\u{00D9}");
    t[0xDA] = Some("\u{00DA}");
    t[0xDB] = Some("\u{00DB}");
    t[0xDC] = Some("\u{00DC}");
    t[0xDD] = Some("\u{00DD}");
    t[0xDE] = Some("\u{00DE}");
    t[0xDF] = Some("\u{00DF}");
    t[0xE0] = Some("\u{00E0}");
    t[0xE1] = Some("\u{00E1}");
    t[0xE2] = Some("\u{00E2}");
    t[0xE3] = Some("\u{00E3}");
    t[0xE4] = Some("\u{00E4}");
    t[0xE5] = Some("\u{00E5}");
    t[0xE6] = Some("\u{00E6}");
    t[0xE7] = Some("\u{00E7}");
    t[0xE8] = Some("\u{00E8}");
    t[0xE9] = Some("\u{00E9}");
    t[0xEA] = Some("\u{00EA}");
    t[0xEB] = Some("\u{00EB}");
    t[0xEC] = Some("\u{00EC}");
    t[0xED] = Some("\u{00ED}");
    t[0xEE] = Some("\u{00EE}");
    t[0xEF] = Some("\u{00EF}");
    t[0xF0] = Some("\u{00F0}");
    t[0xF1] = Some("\u{00F1}");
    t[0xF2] = Some("\u{00F2}");
    t[0xF3] = Some("\u{00F3}");
    t[0xF4] = Some("\u{00F4}");
    t[0xF5] = Some("\u{00F5}");
    t[0xF6] = Some("\u{00F6}");
    t[0xF7] = Some("\u{00F7}");
    t[0xF8] = Some("\u{00F8}");
    t[0xF9] = Some("\u{00F9}");
    t[0xFA] = Some("\u{00FA}");
    t[0xFB] = Some("\u{00FB}");
    t[0xFC] = Some("\u{00FC}");
    t[0xFD] = Some("\u{00FD}");
    t[0xFE] = Some("\u{00FE}");
    t[0xFF] = Some("\u{00FF}");
    t
}

/// `StandardEncoding`, ASCII range only.
///
/// Identical to ASCII in 0x20–0x7E **except** at two codes, which is exactly the sort of detail
/// that makes "it's basically ASCII" a wrong answer:
///
/// - `0x27` is `quoteright` (U+2019), not the apostrophe
/// - `0x60` is `quoteleft` (U+2018), not the grave accent
///
/// Codes above 0x7E are **not carried**. Annex D.2 defines them, but writing them from memory is
/// the kind of guess this project refuses; a document that uses them is refused instead.
const fn build_standard() -> [Option<&'static str>; 256] {
    let mut t = build_win_ansi();
    // Undo the CP1252 and Latin-1 halves: StandardEncoding's high range is different and is not
    // carried here.
    let mut i = 0x80;
    while i < 256 {
        t[i] = None;
        i += 1;
    }
    t[0x27] = Some("\u{2019}"); // quoteright
    t[0x60] = Some("\u{2018}"); // quoteleft
    t
}

/// Glyph names this profile can resolve, for `/Differences`.
///
/// A deliberately small table: the names the fixture corpus uses plus the obvious Latin set. The
/// full Adobe Glyph List is not vendored, and an unresolvable name is an error rather than a
/// dropped character.
pub(crate) fn glyph_name_to_str(name: &str) -> Option<&'static str> {
    Some(match name {
        "space" => " ",
        "exclam" => "!",
        "quotedbl" => "\"",
        "numbersign" => "#",
        "dollar" => "$",
        "percent" => "%",
        "ampersand" => "&",
        "quotesingle" => "'",
        "quoteright" => "\u{2019}",
        "quoteleft" => "\u{2018}",
        "parenleft" => "(",
        "parenright" => ")",
        "asterisk" => "*",
        "plus" => "+",
        "comma" => ",",
        "hyphen" => "-",
        "period" => ".",
        "slash" => "/",
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        "colon" => ":",
        "semicolon" => ";",
        "less" => "<",
        "equal" => "=",
        "greater" => ">",
        "question" => "?",
        "at" => "@",
        "bracketleft" => "[",
        "backslash" => "\\",
        "bracketright" => "]",
        "asciicircum" => "^",
        "underscore" => "_",
        "grave" => "`",
        "braceleft" => "{",
        "bar" => "|",
        "braceright" => "}",
        "asciitilde" => "~",
        "endash" => "\u{2013}",
        "emdash" => "\u{2014}",
        // Ligatures — one glyph, several characters. The whole reason `char_codes` and the text
        // are counted separately.
        "fi" => "fi",
        "fl" => "fl",
        "ff" => "ff",
        "ffi" => "ffi",
        "ffl" => "ffl",
        n if n.len() == 1 && n.is_ascii() => {
            // Single-letter names are their own character: /a, /Z.
            return single_ascii(n.as_bytes()[0]);
        }
        _ => return None,
    })
}

/// Map a single ASCII byte to a static string, without allocating.
fn single_ascii(b: u8) -> Option<&'static str> {
    const TABLE: [Option<&str>; 256] = build_win_ansi();
    if b.is_ascii_alphanumeric() {
        TABLE[b as usize]
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(base: BaseEncoding) -> SimpleEncoding {
        SimpleEncoding::new(base, BTreeMap::new())
    }

    #[test]
    fn win_ansi_is_ascii_across_the_printable_range() {
        let e = enc(BaseEncoding::WinAnsi);
        for b in 0x20u8..=0x7E {
            let got = e
                .decode(b)
                .unwrap_or_else(|_| panic!("code {b:#x} unmapped"));
            assert_eq!(got, (b as char).to_string(), "code {b:#x}");
        }
    }

    #[test]
    fn win_ansi_carries_the_cp1252_additions_and_latin1() {
        let e = enc(BaseEncoding::WinAnsi);
        assert_eq!(e.decode(0x80).unwrap(), "\u{20AC}", "euro");
        assert_eq!(e.decode(0x92).unwrap(), "\u{2019}", "right single quote");
        assert_eq!(e.decode(0xE9).unwrap(), "é");
        assert_eq!(e.decode(0xFF).unwrap(), "ÿ");
    }

    /// The two codes where "StandardEncoding is basically ASCII" is wrong.
    #[test]
    fn standard_encoding_differs_from_ascii_at_two_codes() {
        let e = enc(BaseEncoding::Standard);
        assert_eq!(
            e.decode(0x27).unwrap(),
            "\u{2019}",
            "0x27 is quoteright, not apostrophe"
        );
        assert_eq!(
            e.decode(0x60).unwrap(),
            "\u{2018}",
            "0x60 is quoteleft, not grave"
        );

        // Everything else in the printable range is ASCII.
        for b in 0x20u8..=0x7E {
            if b == 0x27 || b == 0x60 {
                continue;
            }
            assert_eq!(e.decode(b).unwrap(), (b as char).to_string(), "code {b:#x}");
        }
    }

    #[test]
    fn standard_encoding_refuses_its_uncarried_high_range() {
        let e = enc(BaseEncoding::Standard);
        for b in [0x80u8, 0xA1, 0xE9, 0xFF] {
            let err = e.decode(b).unwrap_err();
            assert_eq!(
                err.code(),
                "unsupported",
                "code {b:#x} must be refused, not substituted"
            );
        }
    }

    #[test]
    fn an_unmapped_code_is_refused_rather_than_replaced() {
        // U+FFFD in the evidence would be a character the document does not contain.
        let e = enc(BaseEncoding::Standard);
        let err = e.decode(0x00).unwrap_err();
        assert!(!err.to_string().contains('\u{FFFD}'));
        assert_eq!(err.code(), "unsupported");
    }

    #[test]
    fn differences_override_the_base() {
        let mut d = BTreeMap::new();
        d.insert(0x41u8, "space".to_string());
        let e = SimpleEncoding::new(BaseEncoding::WinAnsi, d);
        assert_eq!(
            e.decode(0x41).unwrap(),
            " ",
            "/Differences wins over the base table"
        );
        assert_eq!(e.decode(0x42).unwrap(), "B", "other codes are untouched");
    }

    #[test]
    fn differences_can_name_a_ligature() {
        let mut d = BTreeMap::new();
        d.insert(0x03u8, "fi".to_string());
        let e = SimpleEncoding::new(BaseEncoding::WinAnsi, d);
        let s = e.decode(0x03).unwrap();
        assert_eq!(s, "fi");
        assert_eq!(s.chars().count(), 2, "one code, two characters");
    }

    #[test]
    fn an_unknown_glyph_name_is_refused_with_a_reason() {
        let mut d = BTreeMap::new();
        d.insert(0x41u8, "afii57636".to_string()); // a real AGL name we do not carry
        let e = SimpleEncoding::new(BaseEncoding::WinAnsi, d);
        let err = e.decode(0x41).unwrap_err();
        assert_eq!(err.code(), "unsupported");
        assert!(
            err.to_string().contains("afii57636"),
            "the error must name the glyph so the gap is actionable: {err}"
        );
    }

    #[test]
    fn macroman_is_declared_rather_than_approximated() {
        let e = enc(BaseEncoding::MacRoman);
        assert_eq!(e.decode(b'A').unwrap(), "A", "ASCII is shared");
        let err = e.decode(0xA5).unwrap_err();
        assert_eq!(err.code(), "unsupported");
        assert!(err.to_string().contains("MacRoman"));
    }

    #[test]
    fn encoding_names_resolve() {
        assert_eq!(
            BaseEncoding::from_name(b"WinAnsiEncoding"),
            Some(BaseEncoding::WinAnsi)
        );
        assert_eq!(
            BaseEncoding::from_name(b"MacRomanEncoding"),
            Some(BaseEncoding::MacRoman)
        );
        assert_eq!(
            BaseEncoding::from_name(b"StandardEncoding"),
            Some(BaseEncoding::Standard)
        );
        assert_eq!(
            BaseEncoding::from_name(b"UniJIS-UCS2-H"),
            None,
            "predefined CJK CMaps are not encodings we carry"
        );
    }

    #[test]
    fn single_letter_glyph_names_resolve() {
        let mut d = BTreeMap::new();
        d.insert(0x01u8, "a".to_string());
        d.insert(0x02u8, "Z".to_string());
        let e = SimpleEncoding::new(BaseEncoding::WinAnsi, d);
        assert_eq!(e.decode(0x01).unwrap(), "a");
        assert_eq!(e.decode(0x02).unwrap(), "Z");
    }
}
