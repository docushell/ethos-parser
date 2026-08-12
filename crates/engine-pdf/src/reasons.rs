// Copyright 2026 The ethos-engine maintainers
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

//! The two reason axes (`docs/03-V0-SCOPE.md` §1 items 2–3).
//!
//! Reason codes rather than a confidence score, taken from LiteParse (parity checklist L1, L2).
//! The difference between the two is the difference between an observation a caller can write
//! policy against and an uncalibrated number presented as a safety control.
//!
//! # Two axes, two types
//!
//! **Needing OCR and being hard to lay out are independent properties.** A dense financial table
//! in crisp born-digital text needs no OCR at all; a photographed page of running prose needs OCR
//! and has trivial layout. Collapsing them into one list means a caller who routes on "is this
//! complex" sends the first document to an OCR engine that can only make it worse.
//!
//! They are separate Rust types, not one enum with a comment, so the compiler enforces what the
//! contract asserts: `table-likely` cannot be constructed as an OCR-need reason.

use serde::{Deserialize, Serialize};

/// A reason a caller might route this document to OCR.
///
/// Emitting one is an **observation**, never a routing decision. `docs/01-CONTRACT.md` §9: the
/// engine owns the observation, the caller owns the policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum OcrNeedReason {
    /// No extractable text, and raster imagery is present — the shape of a page-scan.
    Scanned,
    /// No text-showing operators at all on the page.
    NoText,
    /// Very little text *alongside* imagery, suggesting content the text layer does not cover.
    ///
    /// Requires the imagery. See [`crate::thresholds`] for why short text on its own is not
    /// sparse text — it is just short.
    SparseText,
    /// Raster imagery is present. On its own this says nothing about whether OCR is needed.
    EmbeddedImages,
    /// Extracted text appears to be mojibake.
    ///
    /// **Never emitted by this profile.** Declared in the artifact's `not_detected` list. See
    /// [`crate::thresholds`] for the measured reason.
    Garbled,
    /// Substantial vector drawing with no text — glyphs may have been converted to curves.
    VectorText,
    /// The page carries annotations, whose text is not page content and must not be confused
    /// with it.
    AnnotationText,
}

impl OcrNeedReason {
    /// The wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scanned => "scanned",
            Self::NoText => "no-text",
            Self::SparseText => "sparse-text",
            Self::EmbeddedImages => "embedded-images",
            Self::Garbled => "garbled",
            Self::VectorText => "vector-text",
            Self::AnnotationText => "annotation-text",
        }
    }

    /// Every member of the vocabulary, in wire order.
    pub const ALL: [Self; 7] = [
        Self::Scanned,
        Self::NoText,
        Self::SparseText,
        Self::EmbeddedImages,
        Self::Garbled,
        Self::VectorText,
        Self::AnnotationText,
    ];
}

/// A reason a caller might expect this document's layout to be hard.
///
/// **None of these implies OCR.** That independence is the point of the split, and it is
/// asserted by a test rather than left to discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LayoutComplexityReason {
    /// Text appears to be laid out in more than one column.
    ///
    /// **Never emitted by this profile.** v0 has no stable multi-column rule — pdf-inspector's
    /// flips on a single line of text (`min_lines < 15`), so a one-line edit reorders the page.
    /// Declared in the artifact's `not_detected` list.
    MultiColumn,
    /// Ruling lines suggest tabular content.
    TableLikely,
    /// Heavy vector drawing.
    DenseGraphics,
}

impl LayoutComplexityReason {
    /// The wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MultiColumn => "multi-column",
            Self::TableLikely => "table-likely",
            Self::DenseGraphics => "dense-graphics",
        }
    }

    /// Every member of the vocabulary, in wire order.
    pub const ALL: [Self; 3] = [Self::MultiColumn, Self::TableLikely, Self::DenseGraphics];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_spellings_are_kebab_case_and_stable() {
        for r in OcrNeedReason::ALL {
            let json = serde_json::to_string(&r).unwrap();
            assert_eq!(json, format!("\"{}\"", r.as_str()));
            assert!(
                !r.as_str().contains('_'),
                "{} must be kebab-case",
                r.as_str()
            );
        }
        for r in LayoutComplexityReason::ALL {
            let json = serde_json::to_string(&r).unwrap();
            assert_eq!(json, format!("\"{}\"", r.as_str()));
            assert!(
                !r.as_str().contains('_'),
                "{} must be kebab-case",
                r.as_str()
            );
        }
    }

    #[test]
    fn the_vocabulary_matches_the_scope_document() {
        let ocr: Vec<&str> = OcrNeedReason::ALL.iter().map(|r| r.as_str()).collect();
        assert_eq!(
            ocr,
            vec![
                "scanned",
                "no-text",
                "sparse-text",
                "embedded-images",
                "garbled",
                "vector-text",
                "annotation-text"
            ]
        );
        let layout: Vec<&str> = LayoutComplexityReason::ALL
            .iter()
            .map(|r| r.as_str())
            .collect();
        assert_eq!(
            layout,
            vec!["multi-column", "table-likely", "dense-graphics"]
        );
    }

    #[test]
    fn the_two_axes_share_no_spellings() {
        // If a spelling appeared on both axes, a caller reading either list would be unable to
        // tell which property it had observed.
        for o in OcrNeedReason::ALL {
            for l in LayoutComplexityReason::ALL {
                assert_ne!(o.as_str(), l.as_str());
            }
        }
    }

    #[test]
    fn ordering_is_total_and_deterministic() {
        // Emitted lists are sorted, so the order must be a property of the type rather than of
        // the order detectors happened to run in.
        let mut a = vec![
            OcrNeedReason::VectorText,
            OcrNeedReason::NoText,
            OcrNeedReason::Scanned,
        ];
        let mut b = vec![
            OcrNeedReason::Scanned,
            OcrNeedReason::VectorText,
            OcrNeedReason::NoText,
        ];
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
        assert_eq!(
            a[0],
            OcrNeedReason::Scanned,
            "declaration order is the sort order"
        );
    }
}
