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

//! `ExtractArtifact` → `DocumentRepresentation v0` (`docs/05-MILESTONES.md` M5).
//!
//! # Why this is a conversion and not a replacement
//!
//! `extract()` keeps its signature and its artifact. M3's acceptance suite asserts on
//! `ExtractArtifact` — the `"` operator, `Tz`, synthesized flags, the ligature caveat, rotation —
//! and re-pointing thirty-odd behavioural tests at a new shape would be churn that proves
//! nothing new about the parser. The representation is built *from* the extract, and one test
//! asserts every extracted run appears in it exactly once, which is what makes the stage
//! artifact and the published record impossible to drift apart.
//!
//! # What the conversion adds
//!
//! Identity re-stamped as the representation's own; the processing-run identities; pages as
//! records rather than nodes; parent/ordinal structure; the geometry sidecar; and the declared
//! count of nodes that cannot be projected into a grounding artifact.

use engine_core::assurance::{codes, Limitation};
use engine_core::{
    ArtifactIdentity, Assurance, DocumentRepresentation, EngineError, IdAllocator, IdKind,
    NativeLocator, Node, NodeGeometry, NodeKind, PageRecord, PdfLocator, ProcessingRun,
    ProcessorIdentity, Profile, RepresentationPayload, SourceIdentity, StructuralLocator,
    SynthesizedAt, TextRunAttributes, REPRESENTATION_ARTIFACT_TYPE, REPRESENTATION_SCHEMA_VERSION,
};

use crate::extract::ExtractArtifact;

/// The engine's name as it appears in a representation and in a grounding artifact's `producer`.
pub const PROCESSOR_NAME: &str = "ethos-engine";

/// Build the canonical record from an extract.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the artifact will not satisfy the representation's structural
/// invariants — most usefully, if a **measured box falls outside its page**, which means the
/// measurement or the coordinate transform is wrong and is refused rather than clamped.
/// [`EngineError::ResourceLimit`] if id allocation overflows.
pub fn to_representation(
    extract: &ExtractArtifact,
    profile: &Profile,
) -> Result<DocumentRepresentation, EngineError> {
    let mut alloc = IdAllocator::new(extract.identity.profile_sha256.clone());

    let mut pages = Vec::with_capacity(extract.pages.len());
    let mut nodes = Vec::new();
    let mut geometry: Vec<NodeGeometry> = Vec::new();
    let mut tables: Vec<engine_core::TableRecord> = Vec::new();

    for page in &extract.pages {
        let page_id = alloc.next(IdKind::Page)?;

        // v1-S1. Tables carry forward the ids the detector already allocated — re-allocating
        // here would give a cell's `table_id` a different value from its table's `id`, and the
        // structural half of the cross-check addresses cells through exactly that link.
        for t in &page.tables {
            let mut cells = Vec::with_capacity(t.cells.len());
            for c in &t.cells {
                cells.push(engine_core::TableCellRecord {
                    id: alloc.next(IdKind::Element)?,
                    position: c.position.clone(),
                    bbox: rect_to_qrect(c.rect)?,
                    text: c.text.clone(),
                });
            }
            tables.push(engine_core::TableRecord {
                id: t.id.clone(),
                page: page_id.clone(),
                bbox: rect_to_qrect(t.rect)?,
                rows: t.rows,
                columns: t.columns,
                cells,
                // Never Extracted. The ruling lines and the runs are; the grid over them is not.
                derivation: crate::tables::TABLE_DERIVATION,
                // v1-S2. Carried per table, because one document can hold both kinds and
                // `derivation` is `Computed` for both — only this field distinguishes "the
                // author drew this grid" from "a detector inferred it".
                detection_rule: t.rule.to_string(),
                locator_check: t.check.clone(),
            });
        }
        pages.push(PageRecord {
            id: page_id.clone(),
            // The document's own number, not this record's position. They differ the moment a
            // page is quarantined, and a citation renders against the document's number.
            index: page.index,
            width: page.width,
            height: page.height,
            rotation: page.rotation,
        });

        for (i, run) in page.runs.iter().enumerate() {
            nodes.push(Node {
                // The run's id is reused verbatim. Minting a second id for the same evidence
                // would make the extract and the representation disagree about what to call it,
                // and every consumer would then need a mapping nobody wrote.
                id: run.id.clone(),
                kind: NodeKind::TextRun,
                parent: page_id.clone(),
                ordinal: (i + 1) as u32,
                text: run.text.clone(),
                native_locator: NativeLocator::Pdf(PdfLocator {
                    page: run.locator.page,
                    origin_x: run.locator.origin_x,
                    origin_y: run.locator.origin_y,
                    advance: run.locator.advance,
                }),
                // Captured where the document supplied one, absent where it did not. Never
                // invented, and an absent id is not evidence the document is untagged.
                structural_locator: run.mcid.map(StructuralLocator::PdfMcid),
                derivation: run.derivation,
                attributes: TextRunAttributes {
                    char_codes: run.char_codes.clone(),
                    scalar_code_mismatch: run.scalar_code_mismatch,
                    synthesized: run
                        .synthesized
                        .iter()
                        .map(|s| SynthesizedAt {
                            char_index: s.char_index,
                            // The format owns the vocabulary of reasons; the record owns the
                            // fact that a character was authored. Serialized through the same
                            // kebab spelling the extract uses.
                            reason: serde_json::to_value(s.reason)
                                .ok()
                                .and_then(|v| v.as_str().map(str::to_string))
                                .unwrap_or_else(|| "unknown".into()),
                        })
                        .collect(),
                    font_id: run.font_id.clone(),
                    font_size: run.font_size,
                },
            });
            geometry.push(NodeGeometry {
                node: run.id.clone(),
                presence: run.geometry,
            });
        }
    }

    let not_groundable = geometry
        .iter()
        .filter(|g| !g.presence.is_groundable())
        .count() as u32;

    // Rebuild the assurance so the geometry declaration travels with everything else M4
    // established. `Assurance::new` re-derives the capability-limited entries and normalizes,
    // so passing the extract's own list back in deduplicates rather than doubling.
    let mut limitations = extract.assurance.limitations.clone();
    if not_groundable > 0 {
        limitations.push(geometry_absent_limitation(
            not_groundable,
            nodes.len() as u32,
        ));
    }
    let assurance = Assurance::new(
        extract.assurance.capabilities,
        extract.assurance.coverage.pages_authorized,
        extract.assurance.page_states.clone(),
        limitations,
    )?;

    let payload = RepresentationPayload {
        identity: ArtifactIdentity {
            // Re-stamped. Carrying the extract's `ethos.engine.extract.v0` forward would make
            // the published record announce itself as the stage artifact it was built from.
            artifact_type: REPRESENTATION_ARTIFACT_TYPE.to_string(),
            schema_version: REPRESENTATION_SCHEMA_VERSION.to_string(),
            parser_version: extract.identity.parser_version.clone(),
            profile_sha256: extract.identity.profile_sha256.clone(),
        },
        source: SourceIdentity {
            media_type: extract.source.media_type.clone(),
            sha256: extract.source.sha256.clone(),
        },
        processing_run: ProcessingRun {
            processor: ProcessorIdentity {
                name: PROCESSOR_NAME.to_string(),
                version: extract.identity.parser_version.clone(),
                backend: format!("{} {}", profile.backend.name, profile.backend.version),
            },
            reading_order_rule: extract.reading_order_rule.clone(),
        },
        coordinate_system: profile.coordinate_system,
        pages,
        nodes,
        tables,
        assurance,
    };

    DocumentRepresentation::seal(payload, geometry)
}

/// The declaration that some nodes cannot reach a grounding artifact.
///
/// **This is the "declare" half of omit-plus-count-plus-declare** (`docs/01-CONTRACT.md` §11).
/// It has to live here, in the record, because `ethos.grounding.v1` is
/// `additionalProperties: false` and physically cannot carry a limitation list — so a consumer
/// holding only the grounding artifact must be able to come back to the representation and find
/// out what is missing from it. Putting the count in the record rather than in a log is what
/// makes that possible after the fact.
fn geometry_absent_limitation(not_groundable: u32, total: u32) -> Limitation {
    Limitation::document(
        codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        format!(
            "{not_groundable} of {total} node(s) in this representation have no measurable ink \
             box, so they are OMITTED from any `ethos.grounding.v1` projection of it — that \
             schema requires a bbox on every element and span, and fabricating one is forbidden. \
             The nodes are still here, with their text and their native locators intact: the gap \
             is in what can be expressed downstream, not in what was read. A grounding artifact \
             with fewer elements than this record has nodes is therefore expected, and this is \
             the count that reconciles them."
        ),
    )
}

/// A detected rectangle as the contract's rectangle type.
fn rect_to_qrect(r: crate::tables::QuantRect) -> Result<engine_core::QRect, EngineError> {
    engine_core::QRect::new(r.x0, r.y0, r.x1, r.y1).map_err(|e| EngineError::Malformed {
        what: "table geometry".into(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{conformance_fixture, engine_fixture};
    use crate::Document;

    fn represent(bytes: &[u8]) -> DocumentRepresentation {
        let profile = Profile::default();
        let doc = Document::open_bytes(bytes, &profile).expect("opens");
        let extract = crate::extract(&doc, &profile).expect("extracts");
        to_representation(&extract, &profile).expect("represents")
    }

    #[test]
    fn every_extracted_run_appears_exactly_once() {
        // The bridge. Without it the stage artifact and the published record can drift, and the
        // drift would be invisible: both would still be well-formed.
        let bytes = conformance_fixture("synthetic/two-columns/document.pdf");
        let profile = Profile::default();
        let doc = Document::open_bytes(&bytes, &profile).unwrap();
        let extract = crate::extract(&doc, &profile).unwrap();
        let repr = to_representation(&extract, &profile).unwrap();

        let run_ids: Vec<&str> = extract.runs().map(|r| r.id.as_str()).collect();
        let node_ids: Vec<&str> = repr.payload().nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(run_ids, node_ids, "same runs, same ids, same order");

        let run_text: Vec<&str> = extract.runs().map(|r| r.text.as_str()).collect();
        let node_text: Vec<&str> = repr
            .payload()
            .nodes
            .iter()
            .map(|n| n.text.as_str())
            .collect();
        assert_eq!(run_text, node_text);
    }

    #[test]
    fn the_representation_declares_its_own_artifact_type() {
        let repr = represent(&conformance_fixture("synthetic/simple-text/document.pdf"));
        assert_eq!(
            repr.payload().identity.artifact_type,
            "ethos.engine.representation.v0"
        );
        assert_ne!(
            repr.payload().identity.artifact_type,
            crate::EXTRACT_ARTIFACT_TYPE,
            "the record must not announce itself as the stage artifact it was built from"
        );
        assert_eq!(
            repr.payload().identity.profile_sha256,
            Profile::default().profile_sha256().unwrap()
        );
    }

    #[test]
    fn a_run_is_parented_to_the_page_it_was_drawn_on() {
        let repr = represent(&conformance_fixture("synthetic/two-lines/document.pdf"));
        let pages = &repr.payload().pages;
        for node in &repr.payload().nodes {
            let page = pages
                .iter()
                .find(|p| p.id == node.parent)
                .expect("parent resolves to a declared page");
            // `NativeLocator` is `#[non_exhaustive]`, so even this crate must match rather than
            // destructure — which is the property that keeps a second format from silently
            // compiling into code written for PDF alone.
            match &node.native_locator {
                NativeLocator::Pdf(l) => assert_eq!(
                    page.index, l.page,
                    "the parent page's document number must match the locator's"
                ),
                other => panic!("v0 emits only the PDF locator; got {other:?}"),
            }
        }
    }

    #[test]
    fn ordinals_restart_within_each_page_and_follow_reading_order() {
        let repr = represent(&conformance_fixture("synthetic/two-columns/document.pdf"));
        let ordinals: Vec<u32> = repr.payload().nodes.iter().map(|n| n.ordinal).collect();
        assert_eq!(
            ordinals,
            vec![1, 2, 3, 4],
            "one page, four runs, in stream order"
        );
    }

    #[test]
    fn the_geometry_sidecar_is_aligned_and_outside_the_fingerprint() {
        let repr = represent(&engine_fixture("measured-ink-box/document.pdf"));
        assert_eq!(repr.geometry().len(), repr.payload().nodes.len());
        for (i, node) in repr.payload().nodes.iter().enumerate() {
            assert_eq!(repr.geometry()[i].node, node.id);
        }
        // Sealed artifacts always verify; the point is that the fingerprint covers the payload.
        repr.verify_fingerprint().unwrap();
    }

    #[test]
    fn a_representation_with_no_measurable_geometry_declares_the_count() {
        let repr = represent(&engine_fixture("absent-font-metrics/document.pdf"));
        assert_eq!(repr.nodes_not_groundable(), 1);

        let l = repr
            .payload()
            .assurance
            .limitations
            .iter()
            .find(|l| l.code == codes::GEOMETRY_ABSENT_NOT_GROUNDABLE)
            .expect("the count must be declared in the record");
        assert!(
            l.detail.contains("1 of 1 node(s)"),
            "the declaration carries the count that reconciles the two artifacts: {}",
            l.detail
        );
    }

    #[test]
    fn a_fully_groundable_representation_declares_no_omission() {
        // The mirror. Without it, "declares the count" would pass for an implementation that
        // declares the limitation unconditionally, which would make it noise.
        let repr = represent(&engine_fixture("measured-ink-box/document.pdf"));
        assert_eq!(repr.nodes_not_groundable(), 0);
        assert!(
            !repr
                .payload()
                .assurance
                .limitations
                .iter()
                .any(|l| l.code == codes::GEOMETRY_ABSENT_NOT_GROUNDABLE),
            "a record with nothing omitted must not declare an omission"
        );
    }

    #[test]
    fn the_mcid_becomes_a_structural_locator_where_the_document_supplies_one() {
        // No conformance fixture carries an MCID, so the honest assertion is the negative one:
        // absent means absent, and nothing is invented. `capabilities.structural_locators` is
        // false for exactly this reason.
        let repr = represent(&conformance_fixture("synthetic/simple-text/document.pdf"));
        for node in &repr.payload().nodes {
            assert!(node.structural_locator.is_none());
        }
        assert!(!repr.payload().assurance.capabilities.structural_locators);
    }

    #[test]
    fn the_representation_is_byte_identical_across_runs() {
        let bytes = conformance_fixture("synthetic/two-columns/document.pdf");
        let a = represent(&bytes).to_canonical_bytes().unwrap();
        let b = represent(&bytes).to_canonical_bytes().unwrap();
        assert_eq!(a, b);
    }
}
