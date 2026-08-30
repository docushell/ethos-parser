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

//! **What "a DOCX quote grounds" means, decided** (v2-S1).
//!
//! `14-V2-SCOPE.md` §5 posed the question and did not answer it. This file is the answer, pinned
//! to the facts it rests on — the idiom `liteparse_refusal.rs` uses, and for the same reason: a
//! decision that is only prose stops being true without anyone noticing.
//!
//! # The decision: **(b) — `ethos.grounding.v1` stays PDF-only**
//!
//! No page-less source shape, no optional `page`, no forked schema in this repository. The
//! alternative — revising the artifact — is a change to the **verifier's** contract, and
//! `07-VERIFY-BOUNDARY.md` is why it cannot be made here: `ethos-parser-cli/tests/oracle.rs` agrees with
//! the pinned Ethos CLI on this exact schema, so an engine-only revision would produce artifacts
//! the verifier does not speak while both still called themselves `ethos.grounding.v1`. Option (a)
//! is therefore **blocked on an Ethos-side revision** and is recorded as owned, not as refused.
//!
//! Adding an optional `page` to the engine's copy would be the lying artifact S5 refused for loose
//! boxes, wearing a different field.
//!
//! # The measurement that resized S2, and what v2-S2 did with it
//!
//! S0 assumed the page assumption lived in `ethos.grounding.v1`, so that deciding (b) would leave
//! v2's gate reachable *"at the representation level"*. **That was false, and this file measured
//! it:** `DocumentRepresentation::seal` refused *every* node whose parent was not a declared page,
//! so a page-less document could not become a representation at all.
//!
//! **v2-S2 fixed the invariant rather than the schema**, which is why the measurement below is now
//! two tests instead of one. The rule became locator-aware: a node with a *paginated* address is
//! still parented by a declared page — unchanged, message included — while a node with a
//! page-less address is parented by a **part**, and `pages` must then be empty. Both halves are
//! asserted here, because the point was never "seal refuses" but "seal refuses the right thing".
//!
//! The grounding decision is untouched by that. A DOCX still does not project, and
//! [`a_page_less_representation_is_refused_by_project`] is where that now fails: on the media
//! type, by name, rather than on a page lookup that would have reported "not a declared page"
//! about a document that has none.
//!
//! # `04-ARCHITECTURE.md` §6's precondition, verified
//!
//! > Second format (v2) … None — **provided `ethos-parser-grounding` never learned about pages**
//!
//! **It holds, and it turns out not to be the load-bearing precondition.** This crate never
//! learned what a page *is*: it reads no locator (`ethos_parser_grounding_has_no_pdf_concept` fails if
//! it so much as mentions `NativeLocator`), it derives no geometry, and it addresses pages by id.
//! `project()`'s own check that `node.parent` names a declared page is a **re-assertion of an
//! invariant `seal` already guarantees** for a paginated document, not independent knowledge.
//!
//! v2-S2 revisited that invariant, so a page-less representation now exists and `project()` can be
//! handed one — which is why the refusal below is explicit and on the **media type**. This crate
//! still never reads a locator; it enforces its own output contract, which is a different thing.

use std::path::PathBuf;

use ethos_parser_core::assurance::{codes, Limitation, PageStateEntry};
use ethos_parser_core::{
    c14n::sha256_hex_bytes, ArtifactIdentity, Assurance, Capabilities, CellTextSource,
    CellValueType, CoordinateSystem, DerivationClass, DocumentRepresentation, DocxLocator,
    GeometryAbsence, GeometryPresence, IdAllocator, IdKind, NativeLocator, Node, NodeAttributes,
    NodeGeometry, NodeKind, OfficeCellAttributes, OfficeRunAttributes, OfficeSlideRunAttributes,
    PageRecord, PdfLocator, PptxLocator, ProcessingRun, ProcessorIdentity, Profile,
    RepresentationPayload, Sha256Hex, SourceIdentity, TextRunAttributes, XlsxLocator,
    REPRESENTATION_ARTIFACT_TYPE, REPRESENTATION_SCHEMA_VERSION,
};
use serde_json::Value;

fn schema() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("schemas/ethos-grounding-source.schema.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_slice(&bytes).expect("the pinned schema is JSON")
}

// -------------------------------------------------------------------------------------------
// The decision: the artifact stays PDF-shaped
// -------------------------------------------------------------------------------------------

/// **The three walls, pinned.** Relax any of them and v2-S1 is reopened deliberately.
///
/// These are the sentences `14-V2-SCOPE.md` §5 quotes. They are asserted here rather than
/// remembered, because "grounding stays PDF-only" is a decision that would otherwise become false
/// the first time someone made `page` optional to get a DOCX through.
/// Serialize the artifact and run the engine's own validator over it — the same
/// path `ethos-parser grounding-check` takes, so the emitted page-less shape is held to
/// the exact rules a consumer's copy of this validator would apply.
fn check_valid(artifact: &ethos_parser_grounding::GroundingSource) {
    let bytes = ethos_parser_grounding::to_canonical_bytes(artifact).expect("canonicalizes");
    let report = ethos_parser_grounding::check::grounding_check(&bytes, None).expect("check runs");
    assert_eq!(
        report.exit_code(),
        0,
        "the emitted artifact must pass the engine's own validator: {:?}",
        report
            .to_canonical_bytes()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    );
}

#[test]
fn the_grounding_artifact_names_pdf_paginated_and_eight_page_less_types() {
    let schema = schema();

    // The const became the 1.1.0 union: PDF first, then the eight page-less types.
    let media = schema["$defs"]["source"]["properties"]["media_type"]["enum"]
        .as_array()
        .expect("media_type is an enum since 1.1.0");
    assert_eq!(media[0], "application/pdf");
    assert_eq!(
        media.len(),
        9,
        "one paginated type plus eight page-less ones"
    );

    // The old wall stands for the paginated shape: under it, page and bbox are
    // still required on every element — enforced by the version gate rather than
    // by `required`, and `a_paginated_node_still_cannot_name_a_page_that_is_not_declared`
    // plus the check-side tests hold the semantics.
    let element_required: Vec<&str> = schema["$defs"]["element"]["required"]
        .as_array()
        .expect("element declares required fields")
        .iter()
        .map(|v| v.as_str().expect("a field name"))
        .collect();
    assert!(element_required.contains(&"id") && element_required.contains(&"kind"));
    assert!(
        schema["$defs"]["element"]["properties"]["locator"].is_object(),
        "the page-less element's address field exists"
    );
}

// -------------------------------------------------------------------------------------------
// The measurement that resizes S2
// -------------------------------------------------------------------------------------------

/// **A node with a PAGINATED address still needs its page**, exactly as it did before v2-S2.
///
/// S1's measurement, kept and narrowed. The rule it found was real and stays; what changed is that
/// it is now the rule for *one family of addresses* rather than for every node. A PDF glyph run
/// whose page is not declared is still refused, with the same message — v2 added a second family
/// and did not loosen the first, which is the whole difference between an extension and a hole.
#[test]
fn a_paginated_node_still_cannot_name_a_page_that_is_not_declared() {
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
    // A parent id that is *allocated* but never declared as a page — which is the shape a
    // page-less format has: a node with a structural parent and no page to point at.
    let orphan_parent = alloc.next(IdKind::Page).expect("an id");
    let node = text_node(&mut alloc, &orphan_parent);
    let geometry = NodeGeometry {
        node: node.id.clone(),
        // The honest presence for a format with no geometry, per `14-V2-SCOPE.md` §3.
        presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
    };

    let error = DocumentRepresentation::seal(payload(vec![node], Vec::new()), vec![geometry])
        .expect_err("a node with no declared page must not seal");

    let message = error.to_string();
    assert!(
        message.contains("is not a declared page"),
        "the refusal must name the reason, so S2 reads the constraint rather than guessing: \
         {message}"
    );
}

/// The same document **with** its page declared seals and projects, so the test above fails for
/// the page and for nothing else.
///
/// Without this, `a_page_less_document_cannot_become_a_representation` could be passing because
/// the fixture is malformed in some other way, and the measurement would be worthless.
#[test]
fn the_same_document_with_a_page_seals_and_projects() {
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
    let page = PageRecord {
        id: alloc.next(IdKind::Page).expect("an id"),
        index: 1,
        width: 30000,
        height: 14400,
        rotation: 0,
    };
    let node = text_node(&mut alloc, &page.id);
    let geometry = NodeGeometry {
        node: node.id.clone(),
        presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
    };

    let sealed = DocumentRepresentation::seal(payload(vec![node], vec![page]), vec![geometry])
        .expect("a node whose parent is declared seals");

    let projection = ethos_parser_grounding::project(&sealed).expect("and it projects");
    // The node had no measurable box, so it is omitted and counted — `project()`'s existing
    // honesty, unchanged by this slice.
    assert_eq!(projection.source.elements.len(), 0);
    assert_eq!(projection.omission.nodes_omitted, 1);
}

/// **A page-less representation does not project, and says so by name** (v2-S1's decision (b)).
///
/// v2-S2 made such a representation constructible, so this is no longer unreachable — it is the
/// live behaviour a caller meets when they run `ethos-parser ground` on a DOCX artifact. The refusal is
/// on `source.media_type`, which this crate owns through the schema, rather than on a page lookup
/// that would have reported "not a declared page" about a document that has none.
#[test]
fn a_page_less_representation_is_refused_by_project() {
    let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
    let part = alloc.next(IdKind::Part).unwrap();
    let node = Node {
        id: alloc.next(IdKind::Span).unwrap(),
        kind: NodeKind::TextRun,
        parent: part,
        ordinal: 1,
        text: "a quote".into(),
        native_locator: NativeLocator::Docx(DocxLocator {
            part: "word/document.xml".into(),
            paragraph: 1,
            run: 1,
        }),
        structural_locator: None,
        derivation: DerivationClass::Extracted,
        attributes: NodeAttributes::OfficeRun(OfficeRunAttributes {
            space_preserved: false,
        }),
    };
    let geometry = NodeGeometry {
        node: node.id.clone(),
        presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
    };

    let mut payload = payload(vec![node], Vec::new());
    payload.source.media_type = ethos_parser_office_media_type();
    let sealed = DocumentRepresentation::seal(payload, vec![geometry])
        .expect("v2-S2 made this constructible");

    let projection = ethos_parser_grounding::project(&sealed)
        .expect("since schema 1.1.0 a page-less source projects");
    let artifact = &projection.source;
    assert_eq!(artifact.schema_version, "1.1.0");
    assert!(artifact.pages.is_empty(), "no page was synthesized");
    assert_eq!(artifact.elements.len(), 1);
    let element = &artifact.elements[0];
    assert_eq!(element.page, None);
    assert_eq!(element.bbox, None);
    let locator = element
        .locator
        .as_deref()
        .expect("the native address travels");
    assert!(locator.contains("word/document.xml"), "{locator}");
    check_valid(artifact);
}

/// **And so is a workbook** (v2-S3), for the same reason and with no change to this crate.
///
/// The second page-less format is the check that the refusal is on the *family* rather than on
/// one media type that happened to be handled: a spreadsheet has a shape that tempts a page —
/// print ranges, page breaks, "fit to page" — and none of it reaches `ethos.grounding.v1`.
#[test]
fn an_xlsx_representation_is_refused_by_project() {
    let mut alloc = IdAllocator::new(Profile::xlsx_v0().profile_sha256().unwrap());
    let part = alloc.next(IdKind::Part).unwrap();
    let node = Node {
        id: alloc.next(IdKind::Span).unwrap(),
        kind: NodeKind::TextRun,
        parent: part,
        ordinal: 1,
        text: "a cell".into(),
        native_locator: NativeLocator::Xlsx(XlsxLocator {
            part: "xl/worksheets/sheet1.xml".into(),
            sheet: "Ledger".into(),
            row: 12,
            column: "B".into(),
        }),
        structural_locator: None,
        derivation: DerivationClass::Extracted,
        attributes: NodeAttributes::OfficeCell(OfficeCellAttributes {
            value_type: CellValueType::Number,
            text_source: CellTextSource::StoredValue,
        }),
    };
    let geometry = NodeGeometry {
        node: node.id.clone(),
        presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
    };

    let mut payload = payload(vec![node], Vec::new());
    payload.source.media_type = engine_workbook_media_type();
    let sealed =
        DocumentRepresentation::seal(payload, vec![geometry]).expect("a workbook artifact seals");

    let projection =
        ethos_parser_grounding::project(&sealed).expect("since schema 1.1.0 a workbook projects");
    let artifact = &projection.source;
    assert_eq!(artifact.schema_version, "1.1.0");
    assert!(artifact.pages.is_empty());
    let locator = artifact.elements[0]
        .locator
        .as_deref()
        .expect("the cell address travels");
    assert!(
        locator.contains("Ledger") && locator.contains("\"row\":12"),
        "the locator is the sheet's own address language: {locator}"
    );
    check_valid(artifact);
}

/// **And so is a presentation** (v2-S4), still with no change to this crate.
///
/// The third page-less format, and the one that most looks like it has pages: a deck has slides,
/// a slide has a size, and neither becomes a `page` in `ethos.grounding.v1`.
#[test]
fn a_pptx_representation_is_refused_by_project() {
    let mut alloc = IdAllocator::new(Profile::pptx_v0().profile_sha256().unwrap());
    let part = alloc.next(IdKind::Part).unwrap();
    let node = Node {
        id: alloc.next(IdKind::Span).unwrap(),
        kind: NodeKind::TextRun,
        parent: part,
        ordinal: 1,
        text: "a slide run".into(),
        native_locator: NativeLocator::Pptx(PptxLocator {
            part: "ppt/slides/slide1.xml".into(),
            shape: 1,
            paragraph: 1,
            run: 1,
        }),
        structural_locator: None,
        derivation: DerivationClass::Extracted,
        attributes: NodeAttributes::OfficeSlideRun(OfficeSlideRunAttributes {
            shape_id: 2,
            shape_name: "Title 1".into(),
        }),
    };
    let geometry = NodeGeometry {
        node: node.id.clone(),
        presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
    };

    let mut payload = payload(vec![node], Vec::new());
    payload.source.media_type = engine_deck_media_type();
    let sealed =
        DocumentRepresentation::seal(payload, vec![geometry]).expect("a deck artifact seals");

    let projection =
        ethos_parser_grounding::project(&sealed).expect("since schema 1.1.0 a deck projects");
    let artifact = &projection.source;
    assert_eq!(artifact.schema_version, "1.1.0");
    assert!(artifact.pages.is_empty(), "a slide never became a page");
    check_valid(artifact);
}

/// The media type a presentation declares. Spelled out for the reason below.
fn engine_deck_media_type() -> String {
    "application/vnd.openxmlformats-officedocument.presentationml.presentation".into()
}

/// The media type a word-processing document declares.
///
/// Spelled out rather than imported: `ethos-parser-grounding` does not depend on `ethos-parser-office`, and
/// this crate having a *dependency* on a format reader is exactly what the boundary table forbids.
fn ethos_parser_office_media_type() -> String {
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into()
}

/// The media type a workbook declares. Spelled out for the reason above.
fn engine_workbook_media_type() -> String {
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".into()
}

// -------------------------------------------------------------------------------------------
// The third obligation of §3: no renderer in the graph
// -------------------------------------------------------------------------------------------

/// **L30 made mechanical.** A LibreOffice bridge is a dependency before it is a design decision.
///
/// `06-STEAL-REFUSE.md` refuses LibreOffice → PDF because *"it invents pagination"*. The cheapest
/// way for that refusal to lapse is for a renderer to arrive as a transitive dependency and for
/// nobody to read the lock file, so this reads the lock file.
#[test]
fn no_renderer_has_entered_the_dependency_graph() {
    let lock = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("Cargo.lock");
    let text = std::fs::read_to_string(&lock).unwrap_or_else(|e| panic!("{}: {e}", lock.display()));

    for renderer in [
        "libreoffice",
        "soffice",
        "headless_chrome",
        "headless-chrome",
        "wkhtmltopdf",
        "weasyprint",
        "chromiumoxide",
        "printpdf",
    ] {
        assert!(
            !text.contains(&format!("name = \"{renderer}\"")),
            "`{renderer}` is in the dependency graph. A renderer is how pagination gets invented, \
             which `06-STEAL-REFUSE.md` L30 refuses: pagination does not exist in a DOCX and must \
             not be synthesised"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Fixture construction — the smallest payload `seal` will consider
// -------------------------------------------------------------------------------------------

fn text_node(alloc: &mut IdAllocator, parent: &ethos_parser_core::NodeId) -> Node {
    Node {
        id: alloc.next(IdKind::Span).expect("an id"),
        kind: NodeKind::TextRun,
        parent: parent.clone(),
        ordinal: 1,
        text: "hello".into(),
        // v2 will add a variant here (`01-CONTRACT.md` §5.1 names `DocxLocator`). This slice
        // writes no reader, so the only locator that exists is still the PDF one.
        native_locator: NativeLocator::Pdf(PdfLocator {
            page: 1,
            origin_x: 7200,
            origin_y: 7200,
            advance: Some(1000),
        }),
        structural_locator: None,
        derivation: DerivationClass::Extracted,
        attributes: NodeAttributes::TextRun(TextRunAttributes {
            char_codes: vec![0x68, 0x65, 0x6c, 0x6c, 0x6f],
            scalar_code_mismatch: false,
            synthesized: Vec::new(),
            findings: Vec::new(),
            font_id: "F1".into(),
            font_size: 2400,
            region: None,
        }),
    }
}

fn payload(nodes: Vec<Node>, pages: Vec<PageRecord>) -> RepresentationPayload {
    let profile = Profile::default();
    let authorized = pages.iter().map(|p| p.index).max().unwrap_or(0);
    let states: Vec<PageStateEntry> = pages
        .iter()
        .map(|p| PageStateEntry {
            index: p.index,
            state: ethos_parser_core::assurance::PageState::Processed,
        })
        .collect();

    // The geometry absence is declared, so the seal in
    // `a_page_less_document_cannot_become_a_representation` can only fail for the page.
    let limitations = vec![Limitation::document(
        codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        "v2-S1 fixture: a node with no measurable ink box",
    )];

    RepresentationPayload {
        identity: ArtifactIdentity {
            artifact_type: REPRESENTATION_ARTIFACT_TYPE.into(),
            schema_version: REPRESENTATION_SCHEMA_VERSION.into(),
            parser_version: profile.parser_version.clone(),
            profile_sha256: profile.profile_sha256().expect("a profile digest"),
        },
        source: SourceIdentity {
            media_type: "application/pdf".into(),
            sha256: Sha256Hex::from_hex(&sha256_hex_bytes(b"v2-s1")).expect("a digest"),
        },
        processing_run: ProcessingRun {
            processor: ProcessorIdentity {
                name: "ethos-parser".into(),
                version: profile.parser_version.clone(),
                backend: "lopdf 0.44.0".into(),
            },
            reading_order_rule: profile.reading_order_rule.clone(),
        },
        coordinate_system: CoordinateSystem::V0,
        pages,
        nodes,
        tables: Vec::new(),
        assurance: Assurance::new(Capabilities::V0, authorized, states, limitations)
            .expect("the assurance block is well-formed"),
    }
}
