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

//! **G2: what the projection does at `ethos.grounding.v1`'s limits, on sealed representations.**
//!
//! Past a limit this engine used to project anyway, into an artifact the verifier refuses, and exit
//! 0. Each test builds the smallest representation `seal` accepts that crosses one limit — and the
//! same representation at the limit, so the boundary is the checker's own — then projects it and
//! requires the degraded artifact to pass the engine's checker. Tables are covered against a real
//! detected table in the CLI crate, where a PDF reader is available to produce one.

use ethos_parser_core::assurance::{codes, Limitation, PageStateEntry};
use ethos_parser_core::{
    c14n::sha256_hex_bytes, ArtifactIdentity, Assurance, Capabilities, CoordinateSystem,
    DerivationClass, DocumentRepresentation, DocxLocator, EngineError, GeometryAbsence,
    GeometryPresence, IdAllocator, IdKind, NativeLocator, Node, NodeAttributes, NodeGeometry,
    NodeId, NodeKind, OfficeRunAttributes, PageRecord, PdfLocator, ProcessingRun,
    ProcessorIdentity, Profile, QRect, RepresentationPayload, Sha256Hex, SourceIdentity,
    StructuralLocator, TextRunAttributes, REPRESENTATION_ARTIFACT_TYPE,
    REPRESENTATION_SCHEMA_VERSION,
};
use ethos_parser_grounding::{GroundingSource, ELEMENTS_OMITTED_OVER_LIMIT};

const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

fn check_valid(artifact: &GroundingSource) {
    let bytes = ethos_parser_grounding::to_canonical_bytes(artifact).expect("canonicalizes");
    let report = ethos_parser_grounding::check::grounding_check(&bytes, None).expect("check runs");
    assert_eq!(
        report.exit_code(),
        0,
        "the degraded artifact must pass the engine's own validator: {:?}",
        report
            .to_canonical_bytes()
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    );
}

fn payload(
    nodes: Vec<Node>,
    pages: Vec<PageRecord>,
    limitations: Vec<Limitation>,
) -> RepresentationPayload {
    let profile = Profile::default();
    let authorized = pages.iter().map(|p| p.index).max().unwrap_or(0);
    let states: Vec<PageStateEntry> = pages
        .iter()
        .map(|p| PageStateEntry {
            index: p.index,
            state: ethos_parser_core::assurance::PageState::Processed,
        })
        .collect();
    RepresentationPayload {
        identity: ArtifactIdentity {
            artifact_type: REPRESENTATION_ARTIFACT_TYPE.into(),
            schema_version: REPRESENTATION_SCHEMA_VERSION.into(),
            parser_version: profile.parser_version.clone(),
            profile_sha256: profile.profile_sha256().expect("a profile digest"),
        },
        source: SourceIdentity {
            media_type: "application/pdf".into(),
            sha256: Sha256Hex::from_hex(&sha256_hex_bytes(b"g2")).expect("a digest"),
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

fn pages(alloc: &mut IdAllocator, n: u32) -> Vec<PageRecord> {
    (1..=n)
        .map(|index| PageRecord {
            id: alloc.next(IdKind::Page).expect("an id"),
            index,
            width: 60000,
            height: 80000,
            rotation: 0,
        })
        .collect()
}

/// A measured text run on its own line: `line` sets the baseline, far enough apart that no two
/// runs share a line or a block.
fn run(alloc: &mut IdAllocator, parent: &NodeId, line: i64, text: String) -> (Node, NodeGeometry) {
    let y = 4000 + line * 6000;
    let node = Node {
        id: alloc.next(IdKind::Span).expect("an id"),
        kind: NodeKind::TextRun,
        parent: parent.clone(),
        ordinal: u32::try_from(line + 1).expect("a small line number"),
        native_locator: NativeLocator::Pdf(PdfLocator {
            page: 1,
            origin_x: 7200,
            origin_y: y,
            advance: Some(1000),
        }),
        structural_locator: None,
        derivation: DerivationClass::Extracted,
        attributes: NodeAttributes::TextRun(TextRunAttributes {
            char_codes: text.chars().map(u32::from).collect(),
            scalar_code_mismatch: false,
            synthesized: Vec::new(),
            findings: Vec::new(),
            font_id: "F1".into(),
            font_size: 1000,
            region: None,
            block: None,
        }),
        text,
    };
    let geometry = NodeGeometry {
        node: node.id.clone(),
        presence: GeometryPresence::Measured(
            QRect::new(7200, y - 800, 17200, y + 200).expect("a box"),
        ),
    };
    (node, geometry)
}

/// Three runs on separate lines, the middle one `middle` bytes of two-byte characters, sealed.
fn three_runs(middle_chars: usize) -> DocumentRepresentation {
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
    let pages = pages(&mut alloc, 1);
    let mut nodes = Vec::new();
    let mut geometry = Vec::new();
    for (line, text) in [
        (0, "before".to_string()),
        (1, "\u{e9}".repeat(middle_chars)),
        (2, "after".to_string()),
    ] {
        let (n, g) = run(&mut alloc, &pages[0].id, line, text);
        nodes.push(n);
        geometry.push(g);
    }
    DocumentRepresentation::seal(payload(nodes, pages, Vec::new()), geometry).expect("seals")
}

/// **An element past the string limit is omitted with its spans, and nothing is left dangling.**
///
/// The middle run is 16,386 bytes. Its element goes, its span goes, the next element takes the id
/// the omitted one would have had — so the artifact has no gap and no span names a missing element —
/// and the record says how many of each were dropped.
#[test]
fn an_element_past_the_string_limit_is_omitted_with_its_spans_and_ids_close_up() {
    let control = ethos_parser_grounding::project(&three_runs(4)).expect("projects");
    assert_eq!(
        control.source.elements.len(),
        3,
        "the fixture must give three elements, or the omission below proves nothing"
    );

    let repr = three_runs(8_193);
    let p = ethos_parser_grounding::project(&repr).expect("projects");
    let ids: Vec<&str> = p.source.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["e1", "e2"]);
    assert_eq!(p.source.elements[1].text.as_deref(), Some("after"));
    let spans = p.source.spans.as_ref().expect("spans");
    let nodes = &repr.payload().nodes;
    let span_view: Vec<(&str, Option<&str>)> = spans
        .iter()
        .map(|s| (s.id.as_str(), s.element.as_deref()))
        .collect();
    assert_eq!(
        span_view,
        [
            (nodes[0].id.as_str(), Some("e1")),
            (nodes[2].id.as_str(), Some("e2"))
        ]
    );
    let o = p.elements_omitted.expect("declared");
    assert_eq!(
        (o.elements, o.spans, o.text_limit, o.limitation_code),
        (1, 1, 16_384, ELEMENTS_OMITTED_OVER_LIMIT)
    );
    assert_eq!(
        p.omission.nodes_omitted, 0,
        "the geometry ledger is not this"
    );
    assert_eq!(p.tables_withheld, None);
    check_valid(&p.source);
}

/// **At the limit nothing moves**: 8,192 two-byte characters are exactly 16,384 bytes.
#[test]
fn an_element_at_the_string_limit_is_emitted() {
    let p = ethos_parser_grounding::project(&three_runs(8_192)).expect("projects");
    assert_eq!(p.source.elements.len(), 3);
    assert_eq!(p.elements_omitted, None);
    check_valid(&p.source);
}

/// **A document past the page limit is refused, and one at it projects.**
///
/// Pages cannot be withheld — the schema has no capability for them and a missing page would be a
/// hole in what the artifact claims to ground — so the only honest answers are the whole document
/// or none of it.
#[test]
fn a_document_one_page_past_the_limit_is_refused_and_one_at_it_projects() {
    let seal = |n: u32| {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let pages = pages(&mut alloc, n);
        DocumentRepresentation::seal(payload(Vec::new(), pages, Vec::new()), Vec::new())
            .expect("seals")
    };
    let at = ethos_parser_grounding::project(&seal(5_000)).expect("5,000 pages project");
    assert_eq!(at.source.pages.len(), 5_000);
    check_valid(&at.source);

    match ethos_parser_grounding::project(&seal(5_001)) {
        Err(EngineError::ResourceLimit { limit, configured }) => {
            assert_eq!(configured, "5000");
            assert!(limit.contains("pages") && limit.contains("5001"), "{limit}");
        }
        other => panic!("5,001 pages must be refused, got {other:?}"),
    }
}

/// **A page-less element whose text or locator is past its limit is omitted; its neighbours keep
/// their ids.** A page-less element's id is its node's, so nothing renumbers.
#[test]
fn a_page_less_element_past_a_string_limit_is_omitted() {
    let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
    let document = alloc.next(IdKind::Part).unwrap();
    let other = alloc.next(IdKind::Part).unwrap();
    let node = |alloc: &mut IdAllocator,
                parent: &NodeId,
                ordinal: u32,
                part_name: String,
                text: String| Node {
        id: alloc.next(IdKind::Span).unwrap(),
        kind: NodeKind::TextRun,
        parent: parent.clone(),
        ordinal,
        text,
        native_locator: NativeLocator::Docx(DocxLocator {
            part: part_name,
            paragraph: 1,
            run: 1,
        }),
        structural_locator: None,
        derivation: DerivationClass::Extracted,
        attributes: NodeAttributes::OfficeRun(OfficeRunAttributes {
            space_preserved: false,
        }),
    };
    let doc = || "word/document.xml".to_string();
    // A part id names exactly one part, so the long part name lives under a part of its own.
    let nodes = vec![
        node(&mut alloc, &document, 1, doc(), "kept".into()),
        node(&mut alloc, &document, 2, doc(), "x".repeat(16_385)),
        node(&mut alloc, &document, 3, doc(), "y".repeat(16_384)),
        node(
            &mut alloc,
            &other,
            1,
            "p".repeat(2_100),
            "long locator".into(),
        ),
    ];
    let geometry = nodes
        .iter()
        .map(|n| NodeGeometry {
            node: n.id.clone(),
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        })
        .collect();
    let kept = [nodes[0].id.clone(), nodes[2].id.clone()];
    let limitations = vec![Limitation::document(
        codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        "G2 fixture: page-less nodes carry no box",
    )];
    let mut payload = payload(nodes, Vec::new(), limitations);
    payload.source.media_type = DOCX.into();
    let sealed = DocumentRepresentation::seal(payload, geometry).expect("seals");

    let p = ethos_parser_grounding::project(&sealed).expect("projects");
    let ids: Vec<&str> = p.source.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, [kept[0].as_str(), kept[1].as_str()]);
    let o = p.elements_omitted.expect("declared");
    assert_eq!((o.elements, o.spans, o.locator_limit), (2, 0, 2_048));
    check_valid(&p.source);
}

/// **A paragraph of short runs, joined past the limit, is omitted — the rule measures the element,
/// not its runs.** Three runs of 6,000 bytes share one marked-content id, so the block rule makes
/// them one element of 18,000 bytes; no run alone is over. A space the page drew no ink for sits
/// between two of them: it is part of the element's text but has no span, so the spans dropped are
/// the three inked runs, not the four members.
#[test]
fn a_block_of_short_runs_joined_past_the_limit_is_omitted_whole() {
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
    let pages = pages(&mut alloc, 1);
    let parent = pages[0].id.clone();
    let mut nodes = Vec::new();
    let mut geometry = Vec::new();
    let members = [
        ("a".repeat(6_000), true),
        (" ".to_string(), false),
        ("b".repeat(6_000), true),
        ("c".repeat(6_000), true),
    ];
    for (i, (text, inked)) in members.into_iter().enumerate() {
        let (mut node, mut g) = run(&mut alloc, &parent, i as i64, text);
        node.structural_locator = Some(StructuralLocator::PdfMcid(7));
        if !inked {
            g.presence = GeometryPresence::Absent(GeometryAbsence::NoInkToMeasure);
        }
        nodes.push(node);
        geometry.push(g);
    }
    let (after, after_g) = run(&mut alloc, &parent, 4, "after".into());
    nodes.push(after);
    geometry.push(after_g);
    let limitations = vec![Limitation::document(
        codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        "G2 fixture: a space with no ink",
    )];
    let repr =
        DocumentRepresentation::seal(payload(nodes, pages, limitations), geometry).expect("seals");

    let p = ethos_parser_grounding::project(&repr).expect("projects");
    let texts: Vec<&str> = p
        .source
        .elements
        .iter()
        .filter_map(|e| e.text.as_deref())
        .collect();
    assert_eq!(texts, ["after"], "the joined paragraph is the one omitted");
    let o = p.elements_omitted.expect("declared");
    assert_eq!(
        (o.elements, o.spans),
        (1, 3),
        "three inked runs, not four members"
    );
    assert_eq!(
        p.omission.nodes_omitted, 1,
        "the inkless space stays in the geometry ledger"
    );
    check_valid(&p.source);
}

/// **A page-less locator exactly at 2,048 bytes is kept, and one byte past it is omitted.** The
/// locator is measured as the canonical JSON the artifact carries.
#[test]
fn a_page_less_locator_at_the_limit_is_kept_and_one_past_it_is_omitted() {
    let locator_len = |part: &str| {
        ethos_parser_core::c14n::canonical_bytes_of(&NativeLocator::Docx(DocxLocator {
            part: part.into(),
            paragraph: 1,
            run: 1,
        }))
        .expect("canonical")
        .len()
    };
    let base = locator_len("");
    let at = "p".repeat(2_048 - base);
    let past = "p".repeat(2_049 - base);
    assert_eq!((locator_len(&at), locator_len(&past)), (2_048, 2_049));

    let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
    let mut nodes = Vec::new();
    for part_name in [at, past] {
        let part = alloc.next(IdKind::Part).unwrap();
        nodes.push(Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: part,
            ordinal: 1,
            text: "text".into(),
            native_locator: NativeLocator::Docx(DocxLocator {
                part: part_name,
                paragraph: 1,
                run: 1,
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeRun(OfficeRunAttributes {
                space_preserved: false,
            }),
        });
    }
    let kept = nodes[0].id.clone();
    let geometry = nodes
        .iter()
        .map(|n| NodeGeometry {
            node: n.id.clone(),
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        })
        .collect();
    let limitations = vec![Limitation::document(
        codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        "G2 fixture: page-less nodes carry no box",
    )];
    let mut payload = payload(nodes, Vec::new(), limitations);
    payload.source.media_type = DOCX.into();
    let sealed = DocumentRepresentation::seal(payload, geometry).expect("seals");

    let p = ethos_parser_grounding::project(&sealed).expect("projects");
    let ids: Vec<&str> = p.source.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, [kept.as_str()]);
    assert_eq!(p.elements_omitted.map(|o| o.elements), Some(1));
    check_valid(&p.source);
}
