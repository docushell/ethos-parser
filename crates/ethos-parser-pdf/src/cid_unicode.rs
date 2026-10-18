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

//! **Adobe's CID-to-Unicode mappings** for its character collections (decision #78).
//!
//! A composite font under `/Identity-H` draws CIDs as its codes (PDF 32000-1 §9.7.4.2). With no
//! `/ToUnicode`, what those CIDs stand for is stated by the font's character collection — its
//! descendant's `/CIDSystemInfo` — and §9.10.2 reads them through that collection's
//! `Registry-Ordering-UCS2` CMap, which Adobe publishes for each of its own:
//! `vendor/adobe-mapping-resources-pdf/`, unmodified, under Adobe's BSD-3-Clause licence, embedded
//! with `include_str!` as the Adobe Glyph List is. Each mapping is a CMap by its syntax, so it is
//! read by the parser a document's `/ToUnicode` is, once, the first time a document needs it.
//! Not under `/Identity-V`: that font writes down the page (§9.7.4.3), and this profile advances
//! every pen across it, so a run read there would carry a box where its glyphs are not.

use std::sync::{Arc, OnceLock};

use crate::cmap::ToUnicode;

/// Each Adobe collection this profile decodes, by its `/Ordering`, and its mapping.
const COLLECTIONS: [(&str, &str); 5] = [
    (
        "Japan1",
        include_str!("../../../vendor/adobe-mapping-resources-pdf/Adobe-Japan1-UCS2"),
    ),
    (
        "GB1",
        include_str!("../../../vendor/adobe-mapping-resources-pdf/Adobe-GB1-UCS2"),
    ),
    (
        "CNS1",
        include_str!("../../../vendor/adobe-mapping-resources-pdf/Adobe-CNS1-UCS2"),
    ),
    (
        "Korea1",
        include_str!("../../../vendor/adobe-mapping-resources-pdf/Adobe-Korea1-UCS2"),
    ),
    (
        "KR",
        include_str!("../../../vendor/adobe-mapping-resources-pdf/Adobe-KR-UCS2"),
    ),
];

/// The mapping of the Adobe collection a font's `/CIDSystemInfo` names, or `None` for any other
/// registry or ordering.
pub(crate) fn for_collection(registry: &[u8], ordering: &[u8]) -> Option<Arc<ToUnicode>> {
    static PARSED: [OnceLock<Option<Arc<ToUnicode>>>; 5] = [const { OnceLock::new() }; 5];
    if registry != b"Adobe" {
        return None;
    }
    let k = COLLECTIONS
        .iter()
        .position(|(name, _)| name.as_bytes() == ordering)?;
    PARSED[k]
        .get_or_init(|| {
            ToUnicode::parse(COLLECTIONS[k].1.as_bytes())
                .ok()
                .map(Arc::new)
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vendored_mapping_parses_and_reads_its_collection() {
        // CID 1 is the space in every Adobe collection; Japan1's CID 34 is `A`, its CID 61 the yen
        // sign JIS-Roman sets where ASCII has the backslash.
        for (name, _) in COLLECTIONS {
            let map = for_collection(b"Adobe", name.as_bytes()).expect("parses");
            assert_eq!(map.code_bytes(), 2, "{name}");
            assert_eq!(map.get(1), Some(" "), "{name}");
        }
        let japan1 = for_collection(b"Adobe", b"Japan1").expect("parses");
        assert_eq!(
            (japan1.get(34), japan1.get(61)),
            (Some("A"), Some("\u{a5}"))
        );
    }

    #[test]
    fn another_registry_or_ordering_names_no_mapping() {
        assert!(for_collection(b"Adobe", b"Identity").is_none());
        assert!(for_collection(b"Other", b"Japan1").is_none());
    }
}
