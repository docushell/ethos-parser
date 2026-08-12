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

//! Classification: counts and named reasons, never a verdict (`docs/03-V0-SCOPE.md` §1).
//!
//! # What this does not produce
//!
//! No confidence. No `Type: TEXT-BASED`. No quality field. The measured case for that rule was
//! produced by the exact feature under consideration — pdf-inspector on
//! `fixtures/synthetic/simple-text`:
//!
//! ```text
//! Type: TEXT-BASED (extractable text)
//! Confidence: 50%
//! Pages with text: 0            <- its own evidence contradicts its verdict
//! ```
//!
//! Two routing rules a reasonable engineer would write against that output, both wrong on that
//! corpus. So this emits the counts and the reasons, and the caller owns the policy.
//!
//! # Bounded
//!
//! At most `profile.classify_sample_pages` pages are content-scanned, and
//! [`Classification::pages_content_scanned`] records how many actually were. pdf-inspector
//! advertises `Sample(8)` and then re-scans every page in a later phase, so asking for one page
//! costs the same as asking for all of them (`docs/reference/…memo.md` §16.3). The counter makes
//! that class of regression a test failure rather than a benchmark surprise.

use serde::{Deserialize, Serialize};

use engine_core::{ArtifactIdentity, EngineError, Profile, Sha256Hex};

use crate::document::Document;
use crate::reasons::{LayoutComplexityReason, OcrNeedReason};
use crate::thresholds as th;

/// Artifact type for a classification. **DRAFT** — see `docs/draft-schemas/`.
pub const CLASSIFICATION_ARTIFACT_TYPE: &str = "ethos.engine.classification.v0";

/// Shape version of the classification artifact. **DRAFT**.
pub const CLASSIFICATION_SCHEMA_VERSION: &str = "0.1.0";

/// What the source was, bound by digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    /// Always `application/pdf` at v0.
    pub media_type: String,
    /// Digest of the exact source bytes.
    pub sha256: Sha256Hex,
}

/// A reason this profile's detectors can never emit, with the reason why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotDetected {
    /// The reason code, in its wire spelling.
    pub reason: String,
    /// Why no detector exists.
    pub detail: String,
}

/// Per-page observations. Page numbers are **1-based**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageClassification {
    /// 1-based page number, as the document numbers it.
    pub index: u32,
    /// Bytes appearing inside text-showing operators.
    ///
    /// An **estimate of extent, not of characters**: these are raw string-operand bytes before
    /// any encoding or CMap decoding, which is M3's work. A CID font may use two bytes per glyph,
    /// so this over-counts for CJK. It is used only for the presence and sparseness signals,
    /// never presented as a character count.
    pub text_bytes: u32,
    /// Text-showing operators seen.
    pub text_operators: u32,
    /// Image XObjects referenced by this page's resources.
    pub image_count: u32,
    /// Path construction and painting operators.
    pub path_operators: u32,
    /// Axis-aligned rectangles — the ruling-line primitive.
    pub rectangles: u32,
    /// Annotations on the page.
    pub annotations: u32,
    /// OCR-need reasons observed on this page, sorted.
    pub ocr_reasons: Vec<OcrNeedReason>,
    /// Layout-hard reasons observed on this page, sorted.
    pub layout_reasons: Vec<LayoutComplexityReason>,
}

/// The classification artifact.
///
/// Carries the standard identity envelope plus counts, per-page rows, and the two reason axes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classification {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    pub identity: ArtifactIdentity,
    /// The bytes this classification describes.
    pub source: SourceRef,
    /// Total pages in the document. **Does not** drive cost.
    pub page_count: u32,
    /// Pages selected for sampling: `min(page_count, classify_sample_pages)`.
    pub pages_sampled: u32,
    /// Pages whose content stream was actually walked.
    ///
    /// The bound, made observable. Must equal [`Self::pages_sampled`] and must never approach
    /// [`Self::page_count`] on a large document. A test asserts it; a later phase that rescanned
    /// the document would move this number and fail.
    pub pages_content_scanned: u32,
    /// Sampled pages carrying at least one text-showing operator.
    pub pages_with_text: u32,
    /// Union of per-page OCR-need reasons, sorted and deduplicated.
    pub ocr_reasons: Vec<OcrNeedReason>,
    /// Union of per-page layout reasons, sorted and deduplicated.
    pub layout_reasons: Vec<LayoutComplexityReason>,
    /// **Derived**, never independently decided: true when either list is non-empty.
    ///
    /// The lists are the truth. Removing this field loses no information, which a property test
    /// asserts.
    pub needs_attention: bool,
    /// Per-page rows for the sampled pages, in page order.
    pub pages: Vec<PageClassification>,
    /// Vocabulary members this profile never emits, with reasons.
    pub not_detected: Vec<NotDetected>,
}

impl Classification {
    /// Recompute [`Self::needs_attention`] from the reason lists.
    ///
    /// Exists so the derivation can be checked rather than trusted, and so a caller that strips
    /// the boolean can reconstruct it exactly.
    pub fn derive_needs_attention(&self) -> bool {
        !self.ocr_reasons.is_empty() || !self.layout_reasons.is_empty()
    }

    /// Canonical bytes, via `engine-core`'s c14n.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact cannot be canonicalized — unreachable through
    /// the public API, since every field is an integer, string, bool, or enum.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        let value = serde_json::to_value(self).map_err(|e| EngineError::Malformed {
            what: "classification".into(),
            detail: e.to_string(),
        })?;
        engine_core::c14n_bytes(&value).map_err(|e| EngineError::Malformed {
            what: "classification".into(),
            detail: e.to_string(),
        })
    }
}

/// Raw per-page tallies, before any threshold is applied.
///
/// Separated from reason derivation so the counting and the judging can be tested apart: a wrong
/// count and a wrong threshold are different bugs and should not be able to hide each other.
#[derive(Debug, Default, Clone, Copy)]
struct PageTally {
    text_bytes: u32,
    text_operators: u32,
    image_count: u32,
    path_operators: u32,
    rectangles: u32,
    annotations: u32,
}

/// Classify a document under a profile.
///
/// # Errors
///
/// [`EngineError`] if a sampled page's content cannot be read. Note what is **not** an error: a
/// page with no text, no content, or unreadable-but-present resources is an *observation*, and
/// observations are what this function returns.
pub fn classify(doc: &Document, profile: &Profile) -> Result<Classification, EngineError> {
    let page_count = doc.page_count();
    let sample_n = profile.classify_sample_pages;
    let pages_sampled = page_count.min(sample_n);

    let mut rows: Vec<PageClassification> = Vec::with_capacity(pages_sampled as usize);
    let mut scanned = 0u32;

    // The bound. `take` is the whole mechanism: there is no later phase, and adding one would
    // move `pages_content_scanned` and fail `the_sampler_is_bounded_on_a_492_page_document`.
    for &(page_number, page_id) in doc.pages().iter().take(pages_sampled as usize) {
        let tally = tally_page(doc, page_id);
        scanned += 1;
        rows.push(page_row(page_number, tally));
    }

    let mut ocr_reasons: Vec<OcrNeedReason> = rows
        .iter()
        .flat_map(|r| r.ocr_reasons.iter().copied())
        .collect();
    ocr_reasons.sort_unstable();
    ocr_reasons.dedup();

    let mut layout_reasons: Vec<LayoutComplexityReason> = rows
        .iter()
        .flat_map(|r| r.layout_reasons.iter().copied())
        .collect();
    layout_reasons.sort_unstable();
    layout_reasons.dedup();

    let pages_with_text = rows.iter().filter(|r| r.text_operators > 0).count() as u32;
    let needs_attention = !ocr_reasons.is_empty() || !layout_reasons.is_empty();

    Ok(Classification {
        identity: ArtifactIdentity {
            artifact_type: CLASSIFICATION_ARTIFACT_TYPE.to_string(),
            schema_version: CLASSIFICATION_SCHEMA_VERSION.to_string(),
            parser_version: profile.parser_version.clone(),
            profile_sha256: profile
                .profile_sha256()
                .map_err(|e| EngineError::Malformed {
                    what: "profile".into(),
                    detail: e.to_string(),
                })?,
        },
        source: SourceRef {
            media_type: "application/pdf".to_string(),
            sha256: doc.source_sha256().clone(),
        },
        page_count,
        pages_sampled,
        pages_content_scanned: scanned,
        pages_with_text,
        ocr_reasons,
        layout_reasons,
        needs_attention,
        pages: rows,
        not_detected: th::NOT_DETECTED
            .iter()
            .map(|(reason, detail)| NotDetected {
                reason: (*reason).to_string(),
                detail: (*detail).to_string(),
            })
            .collect(),
    })
}

/// Walk one page's content stream and resources, counting.
///
/// Counting only. No text is decoded, no position computed, no operator interpreted — that is
/// M3. A page whose content stream fails to decode yields zero tallies rather than an error: an
/// unreadable content stream is an observation about the document, and turning it into a hard
/// failure would make one bad page abort a classification the caller could still route on.
fn tally_page(doc: &Document, page_id: lopdf::ObjectId) -> PageTally {
    let mut t = PageTally::default();

    let content = doc.inner().get_page_content(page_id);
    if let Ok(decoded) = lopdf::content::Content::decode(&content) {
        for op in &decoded.operations {
            let name = op.operator.as_str();
            if th::TEXT_SHOWING_OPERATORS.contains(&name) {
                t.text_operators += 1;
                t.text_bytes = t
                    .text_bytes
                    .saturating_add(operand_text_bytes(&op.operands));
            }
            if th::PATH_OPERATORS.contains(&name) {
                t.path_operators += 1;
            }
            if name == th::RECTANGLE_OPERATOR {
                t.rectangles += 1;
            }
        }
    }

    if let Ok(dict) = doc.inner().get_dictionary(page_id) {
        t.annotations = dict
            .get(b"Annots")
            .ok()
            .and_then(|o| o.as_array().ok())
            .map(|a| a.len() as u32)
            .unwrap_or(0);

        t.image_count = count_image_xobjects(doc, dict);
    }

    t
}

/// Total bytes of string operands, including strings nested in a `TJ` array.
fn operand_text_bytes(operands: &[lopdf::Object]) -> u32 {
    let mut total = 0u32;
    for o in operands {
        match o {
            lopdf::Object::String(bytes, _) => {
                total = total.saturating_add(bytes.len() as u32);
            }
            lopdf::Object::Array(items) => {
                // `TJ` interleaves strings with kerning numbers; only the strings are text.
                total = total.saturating_add(operand_text_bytes(items));
            }
            _ => {}
        }
    }
    total
}

/// Count image XObjects reachable from a page's resource dictionary.
fn count_image_xobjects(doc: &Document, page_dict: &lopdf::Dictionary) -> u32 {
    let Ok(resources) = page_dict.get(b"Resources") else {
        return 0;
    };
    let Ok(resources) = resolve_dict(doc, resources) else {
        return 0;
    };
    let Ok(xobjects) = resources.get(b"XObject") else {
        return 0;
    };
    let Ok(xobjects) = resolve_dict(doc, xobjects) else {
        return 0;
    };

    let mut count = 0;
    for (_, value) in xobjects.iter() {
        let stream_dict = match value {
            lopdf::Object::Reference(id) => doc
                .inner()
                .get_object(*id)
                .ok()
                .and_then(|o| o.as_stream().ok())
                .map(|s| &s.dict),
            lopdf::Object::Stream(s) => Some(&s.dict),
            _ => None,
        };
        if let Some(d) = stream_dict {
            if d.get(b"Subtype")
                .and_then(|o| o.as_name())
                .is_ok_and(|n| n == b"Image")
            {
                count += 1;
            }
        }
    }
    count
}

/// Follow a reference to a dictionary, or read one inline.
fn resolve_dict<'a>(
    doc: &'a Document,
    object: &'a lopdf::Object,
) -> Result<&'a lopdf::Dictionary, ()> {
    match object {
        lopdf::Object::Dictionary(d) => Ok(d),
        lopdf::Object::Reference(id) => doc
            .inner()
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .ok_or(()),
        _ => Err(()),
    }
}

/// Apply the thresholds. Each produces a reason to *report*; none produces a decision.
fn page_row(index: u32, t: PageTally) -> PageClassification {
    let mut ocr: Vec<OcrNeedReason> = Vec::new();
    let mut layout: Vec<LayoutComplexityReason> = Vec::new();

    let has_text = t.text_operators > 0;
    let has_images = t.image_count > 0;

    if !has_text {
        ocr.push(OcrNeedReason::NoText);
        if has_images {
            // No text plus imagery is the shape of a page-scan.
            ocr.push(OcrNeedReason::Scanned);
        }
        if t.path_operators >= th::VECTOR_TEXT_MIN_PATH_OPS {
            ocr.push(OcrNeedReason::VectorText);
        }
    } else if t.text_bytes < th::SPARSE_TEXT_MAX_BYTES && has_images {
        // Short text ALONGSIDE imagery. Short text on its own is a short page, not a sparse one —
        // see `thresholds::SPARSE_TEXT_MAX_BYTES` for the trap this avoids.
        ocr.push(OcrNeedReason::SparseText);
    }

    if has_images {
        ocr.push(OcrNeedReason::EmbeddedImages);
    }
    if t.annotations > 0 {
        ocr.push(OcrNeedReason::AnnotationText);
    }

    if t.rectangles >= th::TABLE_LIKELY_MIN_RECTANGLES {
        layout.push(LayoutComplexityReason::TableLikely);
    }
    if t.path_operators >= th::DENSE_GRAPHICS_MIN_PATH_OPS {
        layout.push(LayoutComplexityReason::DenseGraphics);
    }

    ocr.sort_unstable();
    ocr.dedup();
    layout.sort_unstable();
    layout.dedup();

    PageClassification {
        index,
        text_bytes: t.text_bytes,
        text_operators: t.text_operators,
        image_count: t.image_count,
        path_operators: t.path_operators,
        rectangles: t.rectangles,
        annotations: t.annotations,
        ocr_reasons: ocr,
        layout_reasons: layout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(t: PageTally) -> PageClassification {
        page_row(1, t)
    }

    #[test]
    fn short_text_without_imagery_is_not_sparse() {
        // The trap both surveyed classifiers fall into on `synthetic/simple-text`. A twenty-byte
        // page with nothing else on it is a short page.
        let r = row(PageTally {
            text_bytes: 5,
            text_operators: 1,
            ..Default::default()
        });
        assert!(
            r.ocr_reasons.is_empty(),
            "short text alone must fire nothing; got {:?}",
            r.ocr_reasons
        );
    }

    #[test]
    fn short_text_with_imagery_is_sparse() {
        let r = row(PageTally {
            text_bytes: 5,
            text_operators: 1,
            image_count: 1,
            ..Default::default()
        });
        assert!(r.ocr_reasons.contains(&OcrNeedReason::SparseText));
        assert!(r.ocr_reasons.contains(&OcrNeedReason::EmbeddedImages));
    }

    #[test]
    fn no_text_with_imagery_reads_as_scanned() {
        let r = row(PageTally {
            image_count: 1,
            ..Default::default()
        });
        assert!(r.ocr_reasons.contains(&OcrNeedReason::NoText));
        assert!(r.ocr_reasons.contains(&OcrNeedReason::Scanned));
    }

    #[test]
    fn no_text_without_imagery_is_only_no_text() {
        let r = row(PageTally::default());
        assert_eq!(r.ocr_reasons, vec![OcrNeedReason::NoText]);
    }

    #[test]
    fn table_likely_never_lands_on_the_ocr_axis() {
        let r = row(PageTally {
            text_bytes: 500,
            text_operators: 50,
            rectangles: th::TABLE_LIKELY_MIN_RECTANGLES,
            path_operators: th::TABLE_LIKELY_MIN_RECTANGLES,
            ..Default::default()
        });
        assert_eq!(r.layout_reasons, vec![LayoutComplexityReason::TableLikely]);
        assert!(
            r.ocr_reasons.is_empty(),
            "a table is not a reason to run OCR; got {:?}",
            r.ocr_reasons
        );
    }

    #[test]
    fn thresholds_are_inclusive_at_the_boundary() {
        let below = row(PageTally {
            text_operators: 1,
            text_bytes: 100,
            rectangles: th::TABLE_LIKELY_MIN_RECTANGLES - 1,
            ..Default::default()
        });
        assert!(below.layout_reasons.is_empty());

        let at = row(PageTally {
            text_operators: 1,
            text_bytes: 100,
            rectangles: th::TABLE_LIKELY_MIN_RECTANGLES,
            ..Default::default()
        });
        assert_eq!(at.layout_reasons, vec![LayoutComplexityReason::TableLikely]);
    }

    #[test]
    fn vector_text_needs_the_absence_of_text() {
        let with_text = row(PageTally {
            text_operators: 10,
            text_bytes: 200,
            path_operators: th::VECTOR_TEXT_MIN_PATH_OPS,
            ..Default::default()
        });
        assert!(!with_text.ocr_reasons.contains(&OcrNeedReason::VectorText));

        let without = row(PageTally {
            path_operators: th::VECTOR_TEXT_MIN_PATH_OPS,
            ..Default::default()
        });
        assert!(without.ocr_reasons.contains(&OcrNeedReason::VectorText));
    }

    #[test]
    fn reason_lists_are_sorted_and_deduplicated() {
        let r = row(PageTally {
            image_count: 3,
            annotations: 2,
            path_operators: th::DENSE_GRAPHICS_MIN_PATH_OPS,
            rectangles: th::TABLE_LIKELY_MIN_RECTANGLES,
            ..Default::default()
        });
        let mut sorted = r.ocr_reasons.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(r.ocr_reasons, sorted, "emitted order must be canonical");

        let mut ls = r.layout_reasons.clone();
        ls.sort_unstable();
        ls.dedup();
        assert_eq!(r.layout_reasons, ls);
    }

    #[test]
    fn tj_array_strings_are_counted_and_kerning_numbers_are_not() {
        let operands = vec![lopdf::Object::Array(vec![
            lopdf::Object::String(b"Hello".to_vec(), lopdf::StringFormat::Literal),
            lopdf::Object::Integer(-250),
            lopdf::Object::String(b"World".to_vec(), lopdf::StringFormat::Literal),
        ])];
        assert_eq!(operand_text_bytes(&operands), 10);
    }
}
