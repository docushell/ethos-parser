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

//! **Codes a font's declared maps leave unmapped, read through the font itself** — decision #43,
//! `declared-font-codes-v3`.
//!
//! A run's codes become characters through the font's `/ToUnicode` CMap, or, where it has none,
//! its simple encoding. Where neither maps a code, `-v2` omitted the run. This module builds, once
//! per font, the map of those codes that the font itself states, consulted only for a code the
//! declared map leaves unmapped — so nothing the document declared is overridden:
//!
//! 1. **The font's `/Differences` glyph names**, behind its `/ToUnicode` or its base encoding —
//!    PDF 32000-1 §9.10.2's next method after `/ToUnicode`. Never the base table itself: a font
//!    that ships a `/ToUnicode` often names `/MacRomanEncoding` as a formality and draws its own
//!    glyphs at those codes — a Devanagari font's code for `ग` is MacRoman's comma — while a
//!    `/Differences` name (`uni0917`) is the document naming the glyph.
//! 2. **A glyph name, by the Adobe Glyph List specification's rules that need no list**
//!    ([`glyph_text`]): a suffix after the first full stop is dropped, components joined by
//!    underscores are read one by one, and each is `uniXXXX…` (four upper-case hex digits per
//!    character), `uXXXX` to `uXXXXXX`, or a name this profile's glyph table holds. The full list
//!    stays unvendored, so a name only it knows stays unmapped.
//! 3. **An embedded TrueType program's own tables** ([`from_program`]): the glyph a code selects —
//!    a composite font's CID through its `/CIDToGIDMap`, a symbolic simple font's code through the
//!    program's symbol cmap — read back through the program's Unicode cmap, the lowest codepoint
//!    that selects the glyph, or else its `post` glyph name by rule 2.
//!
//! **Never a guess.** No character is produced that the font does not state, a control or
//! private-use value is refused, and a code nothing maps still omits its run.

use std::collections::{BTreeMap, HashMap};

use skrifa::MetadataProvider;

use crate::encoding::SimpleEncoding;

/// How a font program's glyphs are reached from the font's codes.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Glyphs<'a> {
    /// A composite font under `/Identity-H` or `/Identity-V`: the code is the CID, and the CID
    /// is the glyph (`None`, `/CIDToGIDMap /Identity`) or the glyph its map's stream states.
    Cid(Option<&'a [u8]>),
    /// A simple TrueType font: the code selects a glyph through the program's symbol cmap.
    Symbol,
}

/// A glyph name's characters, by the Adobe Glyph List specification's rules that need no list.
pub(crate) fn glyph_text(name: &str) -> Option<String> {
    let base = name.split('.').next().unwrap_or("");
    if base.is_empty() {
        return None;
    }
    let upper_hex = |h: &str| {
        h.bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
    };
    let mut out = String::new();
    for part in base.split('_') {
        if let Some(text) = crate::encoding::glyph_name_to_str(part) {
            out.push_str(text);
        } else if let Some(h) = part
            .strip_prefix("uni")
            .filter(|h| !h.is_empty() && h.len() % 4 == 0 && upper_hex(h))
        {
            for i in (0..h.len()).step_by(4) {
                out.push(scalar(&h[i..i + 4])?);
            }
        } else if let Some(h) = part
            .strip_prefix('u')
            .filter(|h| (4..=6).contains(&h.len()) && upper_hex(h))
        {
            out.push(scalar(h)?);
        } else {
            return None;
        }
    }
    plain(&out).then_some(out)
}

/// A Unicode scalar value from hex digits: never a surrogate, never past U+10FFFF.
fn scalar(hex: &str) -> Option<char> {
    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
}

/// Whether text is characters a reader can use: some, and no control or private-use value.
fn plain(text: &str) -> bool {
    !text.is_empty()
        && !text
            .chars()
            .any(|c| c.is_control() || matches!(u32::from(c), 0xE000..=0xF8FF | 0xF0000..=0x10FFFF))
}

/// What a simple font's `/Differences` names state: each name [`glyph_text`] reads.
pub(crate) fn from_encoding(enc: &SimpleEncoding) -> BTreeMap<u32, String> {
    (0..=255u8)
        .filter_map(|code| {
            let text = enc.difference(code).and_then(glyph_text)?;
            Some((u32::from(code), text))
        })
        .collect()
}

/// What an embedded TrueType program states for the codes `glyphs` reaches. Empty where the
/// program does not parse or states nothing.
pub(crate) fn from_program(bytes: &[u8], glyphs: Glyphs<'_>) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    let Ok(font) = skrifa::FontRef::new(bytes) else {
        return out;
    };
    let charmap = font.charmap();
    let names = font.glyph_names();
    // Each glyph's lowest codepoint, from the program's Unicode cmap.
    let mut unicode: HashMap<u32, char> = HashMap::new();
    if charmap.has_map() && !charmap.is_symbol() {
        for (cp, gid) in charmap.mappings() {
            let Some(c) = char::from_u32(cp).filter(|c| plain(c.encode_utf8(&mut [0; 4]))) else {
                continue;
            };
            unicode
                .entry(gid.to_u32())
                .and_modify(|e| *e = (*e).min(c))
                .or_insert(c);
        }
    }
    let text_of = |gid: u32| -> Option<String> {
        if gid == 0 {
            return None;
        }
        if let Some(c) = unicode.get(&gid) {
            return Some(c.to_string());
        }
        if names.source() == skrifa::GlyphNameSource::Synthesized {
            return None;
        }
        names
            .get(skrifa::GlyphId::new(gid))
            .and_then(|n| glyph_text(n.as_str()))
    };
    match glyphs {
        Glyphs::Cid(None) => {
            for gid in 1..names.num_glyphs() {
                if let Some(text) = text_of(gid) {
                    out.insert(gid, text);
                }
            }
        }
        Glyphs::Cid(Some(map)) => {
            for (cid, pair) in map.chunks_exact(2).enumerate() {
                let gid = u32::from(u16::from_be_bytes([pair[0], pair[1]]));
                if gid < names.num_glyphs() {
                    if let Some(text) = text_of(gid) {
                        out.insert(cid as u32, text);
                    }
                }
            }
        }
        Glyphs::Symbol if charmap.is_symbol() => {
            for code in 0..=255u32 {
                let gid = charmap.map(0xF000 + code).or_else(|| charmap.map(code));
                if let Some(text) = gid.and_then(|g| text_of(g.to_u32())) {
                    out.insert(code, text);
                }
            }
        }
        Glyphs::Symbol => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_names_are_read_by_the_rules_that_need_no_list() {
        assert_eq!(glyph_text("uni092A094D0930").as_deref(), Some("प्र"));
        assert_eq!(glyph_text("uni093F.720").as_deref(), Some("ि"));
        assert_eq!(glyph_text("f_f").as_deref(), Some("ff"));
        assert_eq!(glyph_text("f_i.alt").as_deref(), Some("fi"));
        assert_eq!(glyph_text("u1F600").as_deref(), Some("😀"));
        assert_eq!(glyph_text("A").as_deref(), Some("A"));
        // Lower-case hex, a surrogate, a private-use value, a name only the full list knows, a
        // number, and nothing at all: no characters.
        for no in [
            "uni0e3f", "uniD800", "uniE001", "scedilla", "g123", "1", ".notdef", "", "uni093",
        ] {
            assert_eq!(glyph_text(no), None, "{no}");
        }
    }

    #[test]
    fn only_a_differences_name_speaks_and_never_the_base_table() {
        use crate::encoding::BaseEncoding;
        let mut d = BTreeMap::new();
        d.insert(33u8, "uni0915".to_string());
        d.insert(34u8, "scedilla".to_string());
        let map = from_encoding(&SimpleEncoding::new(BaseEncoding::MacRoman, d));
        assert_eq!(map.get(&33).map(String::as_str), Some("क"));
        assert!(
            !map.contains_key(&34),
            "a name only the full list knows stays unmapped"
        );
        assert!(
            !map.contains_key(&0x2C),
            "the base table is no statement about the glyph"
        );
    }

    /// A TrueType program of `glyphs` glyphs whose Unicode cmap (3,1), format 4, maps each
    /// codepoint of `map` to its glyph — and nothing else: no `post` names.
    fn program(glyphs: u16, map: &[(u16, u16)]) -> Vec<u8> {
        let seg = map.len() as u16 + 1;
        let mut sub: Vec<u16> = vec![4, 0, 0, 2 * seg, 0, 0, 0];
        sub.extend(map.iter().map(|&(cp, _)| cp));
        sub.push(0xFFFF);
        sub.push(0);
        sub.extend(map.iter().map(|&(cp, _)| cp));
        sub.push(0xFFFF);
        sub.extend(map.iter().map(|&(cp, gid)| gid.wrapping_sub(cp)));
        sub.push(1);
        sub.extend(std::iter::repeat_n(0, usize::from(seg)));
        sub[1] = 2 * sub.len() as u16;
        let mut cmap: Vec<u8> = [0u16, 1, 3, 1]
            .iter()
            .flat_map(|v| v.to_be_bytes())
            .collect();
        cmap.extend_from_slice(&12u32.to_be_bytes());
        cmap.extend(sub.iter().flat_map(|v| v.to_be_bytes()));
        let mut maxp = 0x0000_5000u32.to_be_bytes().to_vec();
        maxp.extend_from_slice(&glyphs.to_be_bytes());
        let mut out = vec![0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0];
        let mut offset = 12 + 2 * 16;
        for (tag, table) in [(b"cmap", &cmap), (b"maxp", &maxp)] {
            out.extend_from_slice(tag);
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&(offset as u32).to_be_bytes());
            out.extend_from_slice(&(table.len() as u32).to_be_bytes());
            offset += table.len();
        }
        out.extend_from_slice(&cmap);
        out.extend_from_slice(&maxp);
        out
    }

    #[test]
    fn a_program_s_unicode_cmap_names_each_glyph_and_never_a_private_use_value() {
        // Glyph 1 is क, glyph 2 is A, glyph 3 only a private-use value.
        let font = program(4, &[(0x0041, 2), (0x0915, 1), (0xE000, 3)]);
        let identity = from_program(&font, Glyphs::Cid(None));
        assert_eq!(identity.get(&1).map(String::as_str), Some("क"));
        assert_eq!(identity.get(&2).map(String::as_str), Some("A"));
        assert!(
            !identity.contains_key(&3),
            "a private-use value is no character"
        );
        // Through a /CIDToGIDMap stream: CID 1 draws glyph 2, CID 2 glyph 1.
        let mapped = from_program(&font, Glyphs::Cid(Some(&[0, 0, 0, 2, 0, 1])));
        assert_eq!(mapped.get(&1).map(String::as_str), Some("A"));
        assert_eq!(mapped.get(&2).map(String::as_str), Some("क"));
        // A simple font reaches glyphs only through a symbol cmap, which this program has not.
        assert!(from_program(&font, Glyphs::Symbol).is_empty());
    }

    #[test]
    fn a_program_that_does_not_parse_states_nothing() {
        assert!(from_program(b"not a font", Glyphs::Cid(None)).is_empty());
        assert!(from_program(&[], Glyphs::Symbol).is_empty());
    }
}
