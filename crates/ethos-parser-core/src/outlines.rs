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

//! The outline a document declares — `docs/29-OUTLINES-SCOPE.md`.
//!
//! A PDF's `/Outlines` tree is a hierarchy **the author wrote down**, which is why it is consumed
//! rather than inferred: it sits beside `O13` tagged-PDF consumption and `P19` the marked-content
//! bridge, on `ethos-parser-pdf`'s own rule, *"Consume, never synthesise."* `P14` refuses role
//! inferred from *presentation*; a `/First`/`/Next` chain is presentation of nothing.
//!
//! # Why this is not a [`crate::Node`]
//!
//! North Star #4 requires a native locator on every node, and **a bookmark title is text no
//! content stream painted**: it has no page, no origin and no advance, because the document wrote
//! it into the catalog rather than drawing it.
//!
//! [`crate::NativeLocator::PdfObject`] is the closest existing shape — it was added at v1-S4 for
//! annotations and form fields, which "are not glyph runs" — and is still wrong here: its `page`
//! is a **required** `u32`, and an entry whose destination does not resolve is exactly the case
//! this record has to carry. So an outline entry carries no [`crate::NativeLocator`] at all, on
//! [`crate::TableRecord`]'s precedent.
//!
//! **The consequence, stated rather than left to be discovered: an outline title is not
//! quotable.** `locate` searches the nodes and grounding builds its elements from them, so a field
//! outside `nodes` is reached by neither. This is evidence about a document's declared structure,
//! not evidence about its text.

use serde::{Deserialize, Serialize};

/// One entry of a document's declared outline, as the document declared it.
///
/// **One `derivation` for the record**, on [`crate::TableRecord`]'s precedent — *"whose statement
/// the grid is"* — rather than one class per field. The hierarchy and the titles are the author's,
/// so the record is [`crate::DerivationClass::Extracted`]; the page below is the one `Computed`
/// part and says so in its own documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutlineRecord {
    /// Whose statement this entry is: always [`crate::DerivationClass::Extracted`] today.
    ///
    /// Carried rather than implied, because a later slice that read an outline some other engine
    /// wrote would need to say so, exactly as `TableRecord::derivation` does for a `/Table`
    /// carrying this engine's owner attribute.
    pub derivation: crate::derivation::DerivationClass,

    /// The `/Title` string, decoded — **absent when it could not be decoded**, never guessed.
    ///
    /// A PDF text string is UTF-16BE behind a byte-order mark and PDFDocEncoding otherwise
    /// (§7.9.2.2). This engine vendors no PDFDocEncoding table for the `0x80`–`0x9F` block, which
    /// is exactly where that encoding, Latin-1 and Windows-1252 disagree — 69 of the 2 273 entries
    /// in this repository's own corpus carry such a byte
    /// ([`measurements/outlines/`](https://github.com/docushell/ethos-parser)). Decoding them as
    /// Latin-1 would put C1 control characters inside a title that still reads as well-formed,
    /// which is worse than absence because it looks like success.
    ///
    /// So the title is absent and counted. `docs/29-OUTLINES-SCOPE.md` §4 records why vendoring
    /// the block is a separate, optional slice rather than a precondition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// The depth the `/First`/`/Next` chain gives, 1 for a top-level entry.
    ///
    /// **Never renumbered.** Where a document skips a level, so does this — a depth that closed a
    /// gap would be a position in this engine's reading of the tree rather than the tree's own
    /// statement, which is the defect `06-STEAL-REFUSE.md`'s `PI-A` row names in PageIndex.
    pub depth: u32,

    /// The outline item's own object id, **both halves**.
    ///
    /// On [`crate::PdfObjectLocator`]'s rule — *"an object id is both halves"* — so this cannot
    /// re-create the half-address that field exists to refuse. It is not a
    /// [`crate::NativeLocator`]: see this module's own documentation for why.
    pub object: u32,
    /// The generation half of the object id. Usually 0.
    pub generation: u16,

    /// The 1-based page the destination resolves to — **absent when it does not resolve**.
    ///
    /// This is the record's one `Computed` value: the document states a destination, and turning
    /// that into a page index is this engine's lookup over the page tree. A destination that names
    /// a page in **another** file (§12.3.2's remote form, whose first array element is an integer)
    /// resolves to nothing here and is absent rather than coerced to a number of this document's.
    ///
    /// **No end is emitted, ever.** An outline entry declares where a section *begins*;
    /// `17-D1-SCOPE.md`:130 refuses a boundary inferred from a bookmark, and this corpus contains
    /// two entries whose target page precedes their predecessor's, where an inferred end would run
    /// backwards.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}
