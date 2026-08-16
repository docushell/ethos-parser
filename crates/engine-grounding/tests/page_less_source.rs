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
//! `07-VERIFY-BOUNDARY.md` is why it cannot be made here: `engine-cli/tests/oracle.rs` agrees with
//! the pinned Ethos CLI on this exact schema, so an engine-only revision would produce artifacts
//! the verifier does not speak while both still called themselves `ethos.grounding.v1`. Option (a)
//! is therefore **blocked on an Ethos-side revision** and is recorded as owned, not as refused.
//!
//! Adding an optional `page` to the engine's copy would be the lying artifact S5 refused for loose
//! boxes, wearing a different field.
//!
//! # And the measurement that resizes S2
//!
//! S0 assumed the page assumption lived in `ethos.grounding.v1`, so that deciding (b) would leave
//! v2's gate reachable *"at the representation level"*. **That is false, and this file measures
//! it.** `DocumentRepresentation::seal` refuses a node whose parent is not a declared page —
//! `check_structure`, which runs on **both** construction paths, so it is not reachable around.
//! A page-less document cannot become a representation at all, let alone a grounding artifact.
//!
//! So v2's gate sentence — *"a DOCX quote and an XLSX cell both ground"* — is **not reachable by
//! deciding (b) alone**, and the work it implies is upstream of grounding: the representation's
//! own page-parent invariant. That is S2's problem and this slice does not touch it, but S2 now
//! knows it is a v2 design decision about the IR rather than a schema question.
//!
//! # `04-ARCHITECTURE.md` §6's precondition, verified
//!
//! > Second format (v2) … None — **provided `engine-grounding` never learned about pages**
//!
//! **It holds, and it turns out not to be the load-bearing precondition.** This crate never
//! learned what a page *is*: it reads no locator (`engine_grounding_has_no_pdf_concept` fails if
//! it so much as mentions `NativeLocator`), it derives no geometry, and it addresses pages by id.
//! `project()`'s own check that `node.parent` names a declared page is a **re-assertion of an
//! invariant `seal` already guarantees**, not independent knowledge — which is exactly what
//! [`a_page_less_document_cannot_become_a_representation`] shows, by proving `project()` can never
//! be handed such a representation in the first place.
//!
//! The assumption that every node has a page parent lives in `engine-core`. That is permitted by
//! the M5 line in `04-ARCHITECTURE.md` — it is a contract invariant, not format machinery, and
//! nothing there can parse anything — but it is the sentence v2 has to revisit, and §6 pointed at
//! the wrong crate.

use std::path::PathBuf;

use engine_core::assurance::{codes, Limitation, PageStateEntry};
use engine_core::{
    c14n::sha256_hex_bytes, ArtifactIdentity, Assurance, Capabilities, CoordinateSystem,
    DerivationClass, DocumentRepresentation, GeometryAbsence, GeometryPresence, IdAllocator,
    IdKind, NativeLocator, Node, NodeAttributes, NodeGeometry, NodeKind, PageRecord, PdfLocator,
    ProcessingRun, ProcessorIdentity, Profile, RepresentationPayload, Sha256Hex, SourceIdentity,
    TextRunAttributes, REPRESENTATION_ARTIFACT_TYPE, REPRESENTATION_SCHEMA_VERSION,
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
#[test]
fn the_grounding_artifact_can_only_name_a_pdf() {
    let schema = schema();

    assert_eq!(
        schema["$defs"]["source"]["properties"]["media_type"]["const"], "application/pdf",
        "wall 1: a page-less source cannot even be NAMED as the source of this artifact"
    );

    let element_required: Vec<&str> = schema["$defs"]["element"]["required"]
        .as_array()
        .expect("element declares required fields")
        .iter()
        .map(|v| v.as_str().expect("a field name"))
        .collect();
    for field in ["page", "bbox"] {
        assert!(
            element_required.contains(&field),
            "wall 2: `{field}` is required on every element, and a DOCX run has neither"
        );
    }

    let page_required: Vec<&str> = schema["$defs"]["page"]["required"]
        .as_array()
        .expect("page declares required fields")
        .iter()
        .map(|v| v.as_str().expect("a field name"))
        .collect();
    for field in ["index", "width", "height", "rotation"] {
        assert!(
            page_required.contains(&field),
            "wall 3: `{field}` is required on every page, and there is no page geometry to put there"
        );
    }
    assert_eq!(
        schema["$defs"]["page"]["properties"]["width"]["minimum"], 1,
        "a zero-width page is not a way to spell `this document has no pages`"
    );
}

// -------------------------------------------------------------------------------------------
// The measurement that resizes S2
// -------------------------------------------------------------------------------------------

/// **A page-less document cannot become a representation**, so it never reaches grounding.
///
/// This is the finding S1 exists to produce. `14-V2-SCOPE.md` §3 says the empty `pages` vector is
/// the spelling of *"this document has no pages"* — that part is true of the **type**. It is not
/// true of the **invariant**: `check_structure` requires every node's parent to be a declared
/// page, so the empty vector is only legal for a document with no nodes either.
///
/// The failure is **named**, which is what makes it usable: S2 will read this exact message the
/// first time it tries to seal a DOCX, rather than discovering the constraint by surprise
/// somewhere inside `engine ground`.
#[test]
fn a_page_less_document_cannot_become_a_representation() {
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

    let projection = engine_grounding::project(&sealed).expect("and it projects");
    // The node had no measurable box, so it is omitted and counted — `project()`'s existing
    // honesty, unchanged by this slice.
    assert_eq!(projection.source.elements.len(), 0);
    assert_eq!(projection.omission.nodes_omitted, 1);
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

fn text_node(alloc: &mut IdAllocator, parent: &engine_core::NodeId) -> Node {
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
            state: engine_core::assurance::PageState::Processed,
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
                name: "ethos-engine".into(),
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
