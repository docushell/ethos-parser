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

//! `ExtractArtifact` → `DocumentRepresentation v0` (`docs/history/05-MILESTONES.md` M5).
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

use ethos_parser_core::assurance::{codes, Limitation};
use ethos_parser_core::{
    ArtifactIdentity, Assurance, DocumentRepresentation, EngineError, IdAllocator, IdKind,
    NativeLocator, Node, NodeGeometry, NodeKind, PageRecord, PdfLocator, ProcessingRun,
    ProcessorIdentity, Profile, RepresentationPayload, SourceIdentity, SynthesizedAt,
    TextRunAttributes, REPRESENTATION_ARTIFACT_TYPE, REPRESENTATION_SCHEMA_VERSION,
};

use crate::extract::ExtractArtifact;

/// The engine's name as it appears in a representation and in a grounding artifact's `producer`.
pub const PROCESSOR_NAME: &str = "ethos-parser";

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
    let mut tables: Vec<ethos_parser_core::TableRecord> = Vec::new();

    for page in &extract.pages {
        let page_id = alloc.next(IdKind::Page)?;

        // v1-S1. Tables carry forward the ids the detector already allocated — re-allocating
        // here would give a cell's `table_id` a different value from its table's `id`, and the
        // structural half of the cross-check addresses cells through exactly that link.
        for t in &page.tables {
            let mut cells = Vec::with_capacity(t.cells.len());
            for c in &t.cells {
                cells.push(ethos_parser_core::TableCellRecord {
                    id: alloc.next(IdKind::Element)?,
                    position: c.position.clone(),
                    // A detected cell always has a box: the detector measured it from ink. Wrapped
                    // as `Measured` because the field is now a `GeometryPresence` — the tagged
                    // tables below are the `Absent` case, and this is the other half of the pair.
                    geometry: ethos_parser_core::GeometryPresence::Measured(rect_to_qrect(c.rect)?),
                    text: c.text.clone(),
                    // v1.1-S2. `run_indices` addresses this page's run list and every one of
                    // those runs becomes a node under the id it already has, so this is the
                    // detector's own assignment carried across the boundary rather than a second
                    // reading of it. Before S2 the conversion dropped it, and a cell arrived
                    // holding a string with no way back to evidence that was not either a
                    // re-derivation of the geometry rule or a text match.
                    node_ids: c
                        .run_indices
                        .iter()
                        .map(|i| {
                            page.runs.get(*i).map(|r| r.id.clone()).ok_or_else(|| {
                                EngineError::Malformed {
                                    what: "table cell".into(),
                                    detail: format!(
                                        "cell names run {i} of page {}, which holds {} run(s); \
                                         an index that addresses nothing is a broken link, not a \
                                         cell with less text",
                                        page.index,
                                        page.runs.len()
                                    ),
                                }
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                });
            }
            tables.push(ethos_parser_core::TableRecord {
                id: t.id.clone(),
                page: page_id.clone(),
                geometry: ethos_parser_core::GeometryPresence::Measured(rect_to_qrect(t.rect)?),
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
                // v1-S3. Present only where the document's structure tree describes a table on
                // this page — an absent key means the tree said nothing here, which is not the
                // same as the two derivations agreeing.
                tagged_check: t.tagged_check.clone(),
            });
        }

        // v2-S24. The tagged tables become `TableRecord`s in the SAME list, distinguished by
        // `derivation`: `Extracted`, where the geometric tables above are `Computed`. That is the
        // whole point of the field — a consumer reads one `tables` array and tells "the document
        // declared this grid" from "a detector inferred it" off the derivation, not off two lists.
        // The ids the extractor allocated are carried forward verbatim so a cell's `table_id`
        // matches its table's `id`, exactly as for the geometric tables.
        for t in &page.tagged_tables {
            let cells = t
                .cells
                .iter()
                .map(|c| {
                    Ok(ethos_parser_core::TableCellRecord {
                        id: alloc.next(IdKind::Element)?,
                        position: c.position.clone(),
                        // Typed-absent, carried straight through: the tree named no box, and none
                        // is invented here any more than it was in the extractor.
                        geometry: c.geometry,
                        text: c.text.clone(),
                        node_ids: c
                            .run_indices
                            .iter()
                            .map(|i| {
                                page.runs.get(*i).map(|r| r.id.clone()).ok_or_else(|| {
                                    EngineError::Malformed {
                                        what: "tagged table cell".into(),
                                        detail: format!(
                                            "tagged cell names run {i} of page {}, which holds {} \
                                             run(s)",
                                            page.index,
                                            page.runs.len()
                                        ),
                                    }
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    })
                })
                .collect::<Result<Vec<_>, EngineError>>()?;
            tables.push(ethos_parser_core::TableRecord {
                id: t.id.clone(),
                page: page_id.clone(),
                // Absent, and never a fabricated box: the structure tree carries no coordinate.
                geometry: t.geometry,
                rows: t.rows,
                columns: t.columns,
                cells,
                // **Extracted, not Computed** — the grid is the document's own statement, read off
                // its tags, not an inference over ink. The stronger class is the honest one.
                derivation: ethos_parser_core::DerivationClass::Extracted,
                detection_rule: t.rule.to_string(),
                // NotApplicable: no geometry to compare against the structural derivation.
                locator_check: t.check.clone(),
                // The table IS the tree's derivation; there is no independent geometric grid to
                // check it against, and comparing the tree to itself would agree with itself. So no
                // tagged-versus-geometric check is recorded rather than a self-agreeing `Ok`.
                tagged_check: None,
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
                // v1-S3. Resolved during extraction, against the document's own structure
                // tree: a role path where the tree cites this run, a bare marked-content id
                // where it does not, `pdf_artifact` where the page called this furniture, and
                // absent where the page marked nothing. Never invented at any of the four.
                structural_locator: run.structural.clone(),
                derivation: run.derivation,
                attributes: ethos_parser_core::NodeAttributes::TextRun(TextRunAttributes {
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
                    // D4-S2. Assigned by `gutter-columns-v2` during extraction and carried
                    // through `reorder_page` on the run itself. Absent where the cut made no
                    // division, which is most pages.
                    region: run.region,
                    // Assigned by the leading-gap half of `gutter-columns-v3`, carried on the run
                    // exactly as `region` is. Absent wherever the rule declined, which includes
                    // every page of uniform body text.
                    block: run.block,
                    // v1-S6. Carried through unchanged. The node is here because the run is
                    // here; a finding never decides whether it gets projected.
                    findings: run.findings.clone(),
                }),
            });
            geometry.push(NodeGeometry {
                node: run.id.clone(),
                presence: run.geometry,
            });
        }

        // v1-S4. Form fields and annotations, **after** this page's runs so no run's ordinal
        // moves. Their text was never in a content stream, so nothing here reorders text — the
        // node list still reads in the order the page drew it, and these follow.
        for (i, object) in page.objects.iter().enumerate() {
            nodes.push(Node {
                id: object.id.clone(),
                kind: object.attributes.kind(),
                parent: page_id.clone(),
                ordinal: (page.runs.len() + i + 1) as u32,
                text: object.text.clone(),
                // Not a `PdfLocator`. These have no baseline, no advance and no character
                // origin, and filling those in with plausible numbers would put coordinates on
                // the wire that the document does not contain.
                native_locator: NativeLocator::PdfObject(object.locator.clone()),
                structural_locator: object.structural.clone(),
                // The dictionary said this. The engine did not compute it.
                derivation: ethos_parser_core::DerivationClass::Extracted,
                attributes: object.attributes.clone(),
            });
            geometry.push(NodeGeometry {
                node: object.id.clone(),
                // **`NotApplicableToKind`, not `NotReportedByReader`.** An annotation has no ink
                // box because it is not glyphs, which is not a gap in what this reader could
                // measure — counting it as one would inflate the ink-measurement limitation with
                // nodes that were never going to have ink. Its rectangle is on the locator,
                // where it can say it is *declared* rather than measured.
                presence: ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::NotApplicableToKind,
                ),
            });
        }

        // v1-S6. Images, after this page's runs and objects, so no earlier ordinal moves. An
        // image is not a run and never enters the reading order: `gutter-columns-v1` sorts
        // `TextRun`s by origin, and a picture has no baseline to sort by. It follows the text on
        // its page in a documented second sequence, exactly as widgets do.
        for (i, image) in page.images.iter().enumerate() {
            nodes.push(Node {
                id: image.id.clone(),
                kind: NodeKind::Image,
                parent: page_id.clone(),
                ordinal: (page.runs.len() + page.objects.len() + i + 1) as u32,
                // **Empty, and it stays empty.** An image node carries no text because this
                // engine reads none from it: pixels are not decoded, nothing is recognised, and
                // a description would be a model's opinion rather than the document's content.
                text: String::new(),
                native_locator: NativeLocator::PdfImage(image.locator.clone()),
                // No structure tree citation is resolved for images. `/OBJR` binds widgets by
                // object reference (v1-S4) and a tagged `/Figure` would be the analogue here —
                // but binding one needs the same exact-equality join S3 built for text, over a
                // key images do not carry, so nothing is claimed rather than something guessed.
                structural_locator: None,
                // The placement and the digest are read from the document: the matrix it set and
                // the bytes it stores. Nothing here is inferred.
                derivation: ethos_parser_core::DerivationClass::Extracted,
                attributes: ethos_parser_core::NodeAttributes::Image(image.attributes.clone()),
            });
            geometry.push(NodeGeometry {
                node: image.id.clone(),
                // **`NotApplicableToKind`**, for the reason an annotation's is: this field means
                // *measured ink from font metrics*, and an image has no glyphs to measure. Its
                // area is on the locator as a `PaintedRect`, where it says it came from the
                // page's own matrix — a third provenance that must not be flattened into the
                // other two.
                presence: ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::NotApplicableToKind,
                ),
            });
        }
    }

    // **Two counts, because they answer two questions** (v1-S4). A text run with no measurable
    // ink box is a gap in what this reader could measure; a form field is a node the target
    // schema has nowhere to put. Both are omitted from a grounding projection, and folding them
    // into one number would make it impossible to tell a document whose fonts carry no metrics
    // from one that simply has a form on it.
    // v1-S6.2. Split by REASON, because the two are different facts about this reader and the
    // limitation used to state only their sum. `unmeasurable` means the font supplied no usable
    // metrics — a real gap in what this engine could do. `no_ink` means the run draws nothing, so
    // there was never a box to measure and no gap exists. Reporting 11 663 of the first when 242
    // of them are the first and 11 421 are the second is the same conflation this slice repairs
    // one layer down.
    // v2.2-S3. **Split again, and for the reason v1-S6.2 split it the first time.**
    // `NotReportedByReader` is filled by `extract.rs`'s `_ =>` arm, which fires when EITHER the
    // font metrics are missing OR the advance is — and the sentence beneath it said, of all of
    // them, *"their font supplies no usable ascent/descent and no /FontBBox"*. That is a claim
    // about ink metrics made over a bucket half of which is about widths. On
    // `opendataloader-bench` before this slice it was the wrong explanation for 6 519 nodes,
    // because `load_widths` was reading a composite font's widths from a key the format never
    // puts them on; repairing that dropped the bucket to 131 and left the sentence still wrong
    // about whatever remains. The two are separable with no new wire type: a run with no advance
    // has `advance: None` on the locator it already carries.
    let not_reported = |g: &ethos_parser_core::NodeGeometry, n: &ethos_parser_core::Node| {
        n.kind == NodeKind::TextRun
            && matches!(
                g.presence,
                ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::NotReportedByReader
                )
            )
    };
    let advance_absent = |n: &ethos_parser_core::Node| matches!(&n.native_locator, NativeLocator::Pdf(l) if l.advance.is_none());
    let no_advance = geometry
        .iter()
        .zip(&nodes)
        .filter(|(g, n)| not_reported(g, n) && advance_absent(n))
        .count() as u32;
    let unmeasurable = geometry
        .iter()
        .zip(&nodes)
        .filter(|(g, n)| not_reported(g, n) && !advance_absent(n))
        .count() as u32;
    let no_ink = geometry
        .iter()
        .zip(&nodes)
        .filter(|(g, n)| {
            n.kind == NodeKind::TextRun
                && matches!(
                    g.presence,
                    ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::NoInkToMeasure
                    )
                )
        })
        .count() as u32;
    // D4-S5. A third reason, added for the reason v1-S6.2 split the first two: it is a different
    // fact about this reader, and the sum would misreport it. `off_page` means the box WAS
    // measured — the font supplied metrics and the run draws ink — and the document places it
    // outside the page, so no page-relative rectangle exists to emit. Folding it into
    // `unmeasurable` would charge this engine for the document's choice; folding it into `no_ink`
    // would say the run draws nothing when it draws glyphs.
    //
    // It must be counted, not merely spelled: `check_structure` requires the geometry declaration
    // whenever ANY node is non-groundable, so a document whose only absences were off-page boxes
    // would seal with no limitation naming them and be refused — the annotation bug below, one
    // reason over.
    let off_page = geometry
        .iter()
        .zip(&nodes)
        .filter(|(g, n)| {
            n.kind == NodeKind::TextRun
                && matches!(
                    g.presence,
                    ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::MeasuredOffPage
                    )
                )
        })
        .count() as u32;
    let ink_absent = unmeasurable + no_advance + no_ink + off_page;
    let non_text = nodes.iter().filter(|n| n.kind != NodeKind::TextRun).count() as u32;
    // Nodes whose geometry is absent because their KIND has none — an annotation, a
    // form field, an image. `check_structure` requires the geometry declaration
    // whenever ANY node is non-groundable, and these are non-groundable, so leaving
    // them out of the trigger meant a document could be refused for not declaring a
    // gap this function had decided not to mention. A PDF with measurable text and
    // one annotation did exactly that: it sealed before the annotation was added and
    // failed after, with no artifact at all.
    let kind_absent = geometry
        .iter()
        .zip(&nodes)
        .filter(|(g, n)| {
            n.kind != NodeKind::TextRun
                && matches!(
                    g.presence,
                    ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::NotApplicableToKind
                    )
                )
        })
        .count() as u32;
    // The denominator the ink sentence needs: its numerator counts text runs, so its
    // total must too. It was `nodes.len()`, which diluted the ratio with every image
    // and widget in the file and made the sentence say "N of M text node(s)" about a
    // document that did not have M text nodes.
    let text_total = nodes.iter().filter(|n| n.kind == NodeKind::TextRun).count() as u32;

    // Rebuild the assurance so the geometry declaration travels with everything else M4
    // established. `Assurance::new` re-derives the capability-limited entries and normalizes,
    // so passing the extract's own list back in deduplicates rather than doubling.
    let mut limitations = extract.assurance.limitations.clone();
    if ink_absent > 0 || kind_absent > 0 {
        limitations.push(geometry_absent_limitation(
            unmeasurable,
            no_advance,
            no_ink,
            off_page,
            text_total,
            kind_absent,
        ));
    }
    if non_text > 0 {
        limitations.push(non_text_nodes_limitation(non_text, nodes.len() as u32));
    }
    let assurance = Assurance::new(
        extract.assurance.capabilities,
        extract.assurance.coverage.pages_authorized,
        extract.assurance.page_states.clone(),
        limitations,
    )?;

    let payload = RepresentationPayload {
        identity: ArtifactIdentity {
            // Re-stamped. Carrying the extract's `ethos.parser.extract.v0` forward would make
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
/// The declaration that some nodes are the wrong **kind** for a grounding artifact (v1-S4).
///
/// Deliberately separate from [`geometry_absent_limitation`], which means "no ink box could be
/// measured". These nodes were read perfectly well; `ethos.grounding.v1` offers `elements` and
/// `spans` and nothing else, and every `bbox` in it means measured ink. A form field's rectangle
/// is a number the author wrote saying where a widget sits — projecting it would put declared
/// rectangles beside measured ones with nothing on the wire to tell them apart.
fn non_text_nodes_limitation(non_text: u32, total: u32) -> Limitation {
    Limitation::document(
        codes::NON_TEXT_NODES_NOT_PROJECTED,
        format!(
            "{non_text} of {total} node(s) in this representation are form fields, annotations or \
             images rather than text runs, so they are OMITTED from any `ethos.grounding.v1` \
             projection of it. That schema carries `elements` and `spans`, each requiring a bbox \
             that means MEASURED INK — and these nodes carry two other kinds of rectangle \
             entirely. An annotation's `/Rect` is a number the author wrote into a dictionary \
             saying where a widget sits. An image's rectangle is the page's own transformation \
             matrix applied to the unit square, computed by this reader. Emitting all three under \
             one key, with nothing on the wire to tell them apart, would flatten exactly the \
             distinction they exist to keep. The nodes are all still here, with their text, their \
             object ids, their digests and their rectangles — the gap is in what the target \
             schema can express, not in what was read. This is a DIFFERENT count from \
             `geometry-absent-not-groundable`, which means an ink box could not be measured."
        ),
    )
}

fn geometry_absent_limitation(
    unmeasurable: u32,
    no_advance: u32,
    no_ink: u32,
    off_page: u32,
    total: u32,
    kind_absent: u32,
) -> Limitation {
    // D4-S5. The count of reasons is computed, not written, because writing it is how it goes
    // stale — v2-S13.3 is a whole slice of statements that stopped being true, one of them a
    // document claiming to hold seventeen decisions while holding fourteen. A sentence that says
    // "two reasons" above three clauses is that defect in miniature. Conditional rather than
    // always "three" so a document with no off-page box carries the sentence it always carried,
    // byte for byte, and no golden moves for a case this slice did not change.
    //
    // v2.2-S3 adds a fourth clause and the count moves with it, which is the point of computing
    // it. Splitting the ascent/descent bucket without touching this line would have produced a
    // sentence reading "Three reasons" above four — the exact defect the paragraph above
    // describes, introduced by the slice that quotes it.
    let (split_count, split_slices) = match off_page > 0 {
        true => ("Four", "v1-S6.2, D4-S5, v2.2-S3"),
        false => ("Three", "v1-S6.2, v2.2-S3"),
    };
    Limitation::document(
        codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        format!(
            "{} of {total} text node(s) in this representation carry no ink box, so they are \
             OMITTED from any `ethos.grounding.v1` projection of it — that schema requires a bbox \
             on every element and span, and fabricating one is forbidden. The nodes are still \
             here, with their text and their native locators intact: the gap is in what can be \
             expressed downstream, not in what was read. A grounding artifact with fewer elements \
             than this record has nodes is therefore expected, and this is the count that \
             reconciles them.\n\n\
             **{split_count} reasons, split because they say different things about this reader** \
             ({split_slices}). \
             {unmeasurable} node(s) could NOT be measured: their font supplies no usable \
             ascent/descent and no `/FontBBox`, which is a real gap in what this engine can do. \
             {no_advance} node(s) have no ADVANCE, which is a different gap wearing the same \
             coat: the box needs a width as well as an envelope, and this reader could not read \
             one. Until v2.2-S3 these two were reported together under the ascent/descent \
             sentence alone, and the count that sentence was wrong about was large — 6 519 of \
             them on a 200-document corpus, every one really a width the reader had looked for \
             under the wrong key. {no_ink} node(s) had NOTHING to measure: the run draws no ink — \
             a run of spaces — so no box exists to be missing. Only the first two are limitations \
             of this reader. Before any of them were split, an artifact reported their sum under \
             a sentence that read as though the reader had failed every time.",
            unmeasurable + no_advance + no_ink + off_page
        ) + &off_page_clause(off_page)
            + &kind_absent_clause(kind_absent),
    )
}

/// The sentence for runs whose measured box the document draws off the page (D4-S5).
///
/// Its own clause, and empty at zero, for the reason [`kind_absent_clause`] is: a document that
/// draws nothing off-page carries the sentence it always carried, byte for byte. Only a document
/// that has some pays for the third reason — which is also why the two-reason sentence above is
/// left standing rather than rewritten to say three.
fn off_page_clause(off_page: u32) -> String {
    if off_page == 0 {
        return String::new();
    }
    format!(
        "\n\n\
         A further {off_page} node(s) WERE measured and are not on the page: the font supplied \
         metrics, the run draws ink, and the document places the box outside its own page box, so \
         no page-relative rectangle exists to report. This is a property of the document rather \
         than a shortfall of this reader — a page extracted from a wider original is the case in \
         practice — and it is neither clamped to fit nor dropped. Each of these runs is in the \
         artifact with its text, its origin and an `off-page-text` finding; what is absent is the \
         box, and `GeometryAbsence::MeasuredOffPage` is the reason."
    )
}

/// The sentence for nodes whose kind has no ink box at all.
///
/// Kept as its own clause rather than folded into the counts above, because it is a
/// different statement: an annotation or a widget has no ink to measure BY
/// DEFINITION, where an unmeasurable run is a gap in this reader. Empty when there
/// are none, so a document without them carries the text-only sentence it always
/// carried.
fn kind_absent_clause(kind_absent: u32) -> String {
    if kind_absent == 0 {
        return String::new();
    }
    format!(
        "\n\n\
         A further {kind_absent} node(s) carry no ink box because their KIND has none — an \
         annotation, a form field or an image, whose rectangle is a number the author wrote into \
         a dictionary rather than ink this engine measured. Their absence is correct rather than \
         a gap, and they are counted here because `check_structure` requires this declaration \
         whenever any node is non-groundable."
    )
}

/// A detected rectangle as the contract's rectangle type.
fn rect_to_qrect(r: crate::tables::QuantRect) -> Result<ethos_parser_core::QRect, EngineError> {
    ethos_parser_core::QRect::new(r.x0, r.y0, r.x1, r.y1).map_err(|e| EngineError::Malformed {
        what: "table geometry".into(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {

    /// **A node whose KIND has no ink box still demands the declaration** — the
    /// seal says so, and the producer used not to.
    ///
    /// `check_structure` refuses a representation unless
    /// `geometry-absent-not-groundable` is declared exactly when some node is
    /// non-groundable, and an annotation, a form field and an image are all
    /// non-groundable by construction. The trigger here counted ink-absent TEXT
    /// RUNS only, so a PDF whose font supplies real metrics and which carries one
    /// annotation produced no artifact at all: `ethos-parser extract` exited 2 with
    /// "1 node(s) have no measurable ink box, but the payload does not declare".
    /// Real documents escaped only by luck — one whitespace-only run or one
    /// metric-less font supplies the text-run absence that made the declaration
    /// fire for a different reason.
    #[test]
    fn a_non_text_node_alone_still_declares_the_geometry_gap() {
        use ethos_parser_core::{GeometryAbsence, GeometryPresence};

        // One measured text run, one annotation: the combination every fixture in
        // the tree happens to avoid.
        let measured =
            GeometryPresence::Measured(ethos_parser_core::QRect::new(0, 0, 10, 10).unwrap());
        let by_kind = GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind);
        assert!(measured.is_groundable());
        assert!(!by_kind.is_groundable());

        // The producer's own trigger, exercised through the two counts it derives.
        // Before the fix `ink_absent` was the whole condition and this case gave 0.
        let ink_absent = 0u32;
        let kind_absent = 1u32;
        assert!(
            ink_absent > 0 || kind_absent > 0,
            "a representation carrying only a by-kind absence must still declare"
        );

        // And the declaration it produces names that population rather than
        // silently reporting zero text nodes.
        let limitation = geometry_absent_limitation(0, 0, 0, 0, 1, kind_absent);
        assert_eq!(limitation.code, codes::GEOMETRY_ABSENT_NOT_GROUNDABLE);
        assert!(
            limitation.detail.contains("their KIND has none"),
            "the detail must say why these nodes have no box: {}",
            limitation.detail
        );
    }

    /// **The third reason gets its own clause, and the count of reasons follows it** (D4-S5).
    ///
    /// Two assertions, because the failure modes differ. A document with no off-page box must
    /// carry the sentence it always carried — otherwise every golden in the corpus moves for a
    /// case this slice did not change. A document with one must not say "Two reasons" above
    /// three clauses, which is the v2-S13.3 defect: a statement that stopped being true.
    #[test]
    fn the_off_page_reason_is_counted_and_the_reason_count_follows_it() {
        let none = geometry_absent_limitation(1, 0, 0, 0, 1, 0);
        assert!(
            none.detail.contains("**Three reasons"),
            "no off-page box means three clauses, not four — v2.2-S3 added one and the count \
             moves with it: {}",
            none.detail
        );
        assert!(!none.detail.contains("are not on the page"));

        let some = geometry_absent_limitation(0, 0, 0, 2, 3, 0);
        assert!(
            some.detail.starts_with("2 of 3 text node(s)"),
            "an off-page box counts toward the omitted total: {}",
            some.detail
        );
        assert!(
            some.detail.contains("**Four reasons"),
            "four clauses must be introduced as four; a count that stopped following its own \
             clauses is the v2-S13.3 defect this line exists to prevent: {}",
            some.detail
        );
        assert!(
            some.detail
                .contains("2 node(s) WERE measured and are not on the page"),
            "the third reason states its own count: {}",
            some.detail
        );
    }

    /// The ink sentence's denominator counts TEXT nodes, because its numerator does.
    #[test]
    fn the_ink_sentence_counts_text_nodes_on_both_sides() {
        // One unmeasurable text run in a document that also holds two annotations:
        // the sentence is about text, so the total is 1, not 3.
        let limitation = geometry_absent_limitation(1, 0, 0, 0, 1, 2);
        assert!(
            limitation.detail.starts_with("1 of 1 text node(s)"),
            "the denominator was diluted by non-text nodes: {}",
            limitation.detail
        );
    }
    use super::*;
    use crate::test_support::{conformance_fixture, engine_fixture, gate_fixture};
    use crate::Document;

    fn represent(bytes: &[u8]) -> DocumentRepresentation {
        let profile = Profile::default();
        let doc = Document::open_bytes(bytes, &profile).expect("opens");
        let extract = crate::extract(&doc, &profile).expect("extracts");
        to_representation(&extract, &profile).expect("represents")
    }

    /// **A tagged table reaches the representation as `Extracted`, in the same `tables` list, with
    /// absent geometry** (v2-S24).
    ///
    /// This is the slice's central claim about the wire: the geometric tables and the tagged ones
    /// live in one `tables` array, and `derivation` — not two lists — is what tells a consumer
    /// which is which. `irs-f1040sd-2025` draws no readable grid, so every table it contributes is
    /// tagged and `Extracted`; the assertion is that they are present, carry `tagged-tables-v1`,
    /// report geometry absent rather than a fabricated box, and their cross-check is not-applicable.
    #[test]
    fn a_tagged_table_reaches_the_representation_as_extracted_with_absent_geometry() {
        let repr = represent(&gate_fixture("irs-f1040sd-2025.pdf"));
        let tables = &repr.payload().tables;
        let tagged: Vec<&ethos_parser_core::TableRecord> = tables
            .iter()
            .filter(|t| t.detection_rule == ethos_parser_core::TABLE_DETECTION_TAGGED_V1)
            .collect();
        assert!(
            !tagged.is_empty(),
            "the tagged tables must reach the published record, not stop at the extract stage"
        );
        for t in &tagged {
            assert_eq!(
                t.derivation,
                ethos_parser_core::DerivationClass::Extracted,
                "a tagged table is the document's own statement, so it is Extracted — the field \
                 that distinguishes it from a Computed geometric table in the same list"
            );
            assert!(
                matches!(
                    t.geometry,
                    ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::NotReportedByStructureTree
                    )
                ),
                "no box is invented for a tagged table: {:?}",
                t.geometry
            );
            assert!(
                matches!(
                    t.locator_check.outcome,
                    ethos_parser_core::CheckStatus::NotApplicable { .. }
                ),
                "the cross-check is not-applicable, never a false ok: {:?}",
                t.locator_check.outcome
            );
            assert!(
                t.tagged_check.is_none(),
                "the tagged table IS the tree's derivation; there is no independent grid to run a \
                 tagged-versus-geometric check against"
            );
            for c in &t.cells {
                assert!(
                    matches!(c.geometry, ethos_parser_core::GeometryPresence::Absent(_)),
                    "a tagged cell carries no box"
                );
            }
        }
        // And a geometric-table document is unaffected: its tables are still Computed with a
        // measured box, so the change is additive.
        let geo = represent(&engine_fixture("tagged-table-agrees/document.pdf"));
        let g = geo
            .payload()
            .tables
            .iter()
            .find(|t| t.detection_rule == ethos_parser_core::TABLE_DETECTION_V3)
            .expect("the painted grid is a Computed table");
        assert_eq!(g.derivation, ethos_parser_core::DerivationClass::Computed);
        assert!(matches!(
            g.geometry,
            ethos_parser_core::GeometryPresence::Measured(_)
        ));
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
            "ethos.parser.representation.v0"
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
            l.detail.contains("1 of 1 text node(s)")
                && l.detail.contains("1 node(s) could NOT be measured"),
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
    fn an_untagged_document_gains_no_structural_locator() {
        // No conformance fixture carries an MCID or a structure tree, so the assertion here is
        // the negative one — and at v1-S3 it is a *stronger* statement than it was.
        //
        // Through v1-S2 this proved only that nothing was invented while nobody was looking:
        // `capabilities.structural_locators` was false, so absence proved little. The capability
        // is true now, meaning this profile DID read the catalog for a `/StructTreeRoot` — and
        // still emitted no role for a document that declares none. That is the P14 defect
        // refused with the machinery present and running, rather than absent.
        let repr = represent(&conformance_fixture("synthetic/simple-text/document.pdf"));
        for node in &repr.payload().nodes {
            assert!(
                node.structural_locator.is_none(),
                "an untagged document must gain no role, not even a plausible one"
            );
        }
        assert!(
            repr.payload().assurance.capabilities.structural_locators,
            "v1-S3 looks for a structure tree on every document"
        );
        assert!(
            repr.payload()
                .assurance
                .limitations
                .iter()
                .any(|l| l.code == codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
            "and says so when it finds none, rather than leaving the absence unexplained"
        );
    }

    #[test]
    fn the_representation_is_byte_identical_across_runs() {
        let bytes = conformance_fixture("synthetic/two-columns/document.pdf");
        let a = represent(&bytes).to_canonical_bytes().unwrap();
        let b = represent(&bytes).to_canonical_bytes().unwrap();
        assert_eq!(a, b);
    }
}
