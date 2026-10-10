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

//! **The Adobe Glyph List** (decision #59): a glyph name's characters, as Adobe's list gives them.
//!
//! `vendor/agl/glyphlist.txt` is the list's table version 2.0, unmodified, embedded with
//! `include_str!` and read once, as the Core-14 AFMs are (`vendor/README.md`). It is consulted
//! only for a name this profile's own glyph table does not hold — `/ccaron`, `/minus`,
//! `/quotedblleft` — so every name that table reads reads as it did, `/fi` among them: the list
//! gives the ligature U+FB01, the table the two letters.
//!
//! A name the list gives a private-use or control value — a small capital, an old-style figure,
//! which Adobe placed in the private-use area — reads as nothing, by decision #43's rule: such a
//! value is no character a reader can use.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The list as Adobe publishes it.
const GLYPHLIST: &str = include_str!("../../../vendor/agl/glyphlist.txt");

/// A glyph name's characters by the Adobe Glyph List, or `None` where the list does not carry the
/// name or gives it a private-use or control value.
pub(crate) fn characters(name: &str) -> Option<&'static str> {
    static LIST: OnceLock<HashMap<&'static str, String>> = OnceLock::new();
    LIST.get_or_init(|| {
        GLYPHLIST
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| {
                let (name, values) = line.split_once(';')?;
                let text = values
                    .split(' ')
                    .map(|hex| u32::from_str_radix(hex, 16).ok().and_then(char::from_u32))
                    .collect::<Option<String>>()?;
                crate::font_fallback::plain(&text).then_some((name, text))
            })
            .collect()
    })
    .get(name)
    .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::characters;

    /// **The list reads the names this profile's table does not hold** (decision #59): `/ccaron`,
    /// `/minus`, a name of two characters; nothing for a name the list does not carry, nor for
    /// one it places in the private-use area.
    #[test]
    fn the_list_reads_its_names_and_no_private_use_value() {
        assert_eq!(characters("ccaron"), Some("\u{10D}"));
        assert_eq!(characters("minus"), Some("\u{2212}"));
        assert_eq!(characters("dalethatafpatah"), Some("\u{5D3}\u{5B2}"));
        assert_eq!(characters("zerooldstyle"), None, "U+F730, private use");
        assert_eq!(characters("g123"), None, "no name of the list");
    }
}
