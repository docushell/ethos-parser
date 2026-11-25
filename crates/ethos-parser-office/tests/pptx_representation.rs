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

//! v2-S4's gate, executable: **a slide's text binds, and a slide is not a page**.
//!
//! The third sibling of `docx_representation.rs`. Most of its assertions are the ones the first
//! two made; the ones that are new are all about the same temptation — a presentation is the
//! first v2 format whose parts a person counts out loud, and the tests below are what stop
//! "slide 2" becoming `PageRecord.index`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ethos_parser_core::{
    GeometryAbsence, GeometryPresence, NativeLocator, NodeAttributes, PptxLocator, Profile,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("deck.pptx");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn locators(sealed: &ethos_parser_core::DocumentRepresentation) -> Vec<PptxLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Pptx(l) => l.clone(),
            other => panic!("a slide run must carry a presentation address, not {other:?}"),
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The gate sentence, executable.** A known phrase is on a node, addressed by the deck itself.
#[test]
fn a_slide_run_resolves_to_a_node_addressed_by_the_deck_itself() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let payload = sealed.payload();

    let node = payload
        .nodes
        .iter()
        .find(|n| n.text == "Evidence, not extraction.")
        .expect("the phrase is on a slide, so it is in the artifact");

    match &node.native_locator {
        NativeLocator::Pptx(l) => {
            assert_eq!(l.part, "ppt/slides/slide1.xml");
            assert_eq!((l.shape, l.paragraph, l.run), (1, 1, 1));
        }
        other => panic!("expected a presentation address, got {other:?}"),
    }

    assert_eq!(payload.nodes.iter().filter(|n| n.id == node.id).count(), 1);
    assert!(!payload.nodes.iter().any(|n| n.id.as_str() == "s-forged"));
}

/// **A slide is a part, not a page** — the whole of §3 for this format, in one test.
#[test]
fn a_deck_declares_no_pages_and_carries_no_page_number() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let payload = sealed.payload();

    assert!(
        payload.pages.is_empty(),
        "`p:sldSz` states a size, which is not a page this engine read"
    );
    assert!(!payload.nodes.is_empty(), "and that is not vacuous");

    for (index, node) in payload.nodes.iter().enumerate() {
        assert!(
            node.parent.as_str().starts_with('d'),
            "node {} is parented by `{}`, which is not a part id",
            node.id,
            node.parent
        );
        assert_eq!(
            sealed.geometry()[index].presence,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        );
    }

    // The locator's field set is the check that matters: no slide index can be read off it, and
    // `deny_unknown_fields` is what stops one arriving later.
    let wire = serde_json::to_value(&payload.nodes[0].native_locator).expect("serializes");
    let fields: BTreeSet<&str> = wire["pptx"]
        .as_object()
        .expect("an object")
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(
        fields,
        BTreeSet::from(["part", "shape", "paragraph", "run"]),
        "a slide number, a page or a box would all show up here"
    );
}

/// **The single highest-value test in the slice.** The slide-to-part binding is read from the
/// relationship part, not guessed from position.
///
/// The fixture's second slide is `slide7.xml` behind `rId4`. A reader that assumed
/// `slide{n}.xml` in `<p:sldIdLst>` order would look for `slide2.xml`, which does not exist.
#[test]
fn the_slide_part_is_read_through_the_rels_not_guessed_from_position() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");

    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Read because the deck listed it.")
        .expect("the second slide's only run");

    match &node.native_locator {
        NativeLocator::Pptx(l) => assert_eq!(
            l.part, "ppt/slides/slide7.xml",
            "the part comes from the relationship, and no `slide2.xml` exists"
        ),
        other => panic!("expected a presentation address, got {other:?}"),
    }
}

/// **Shape ids repeat in real decks**, so the address is a position and the id is a label.
///
/// The fixture gives two shapes on slide one the same `<p:cNvPr id="2">` — which real Open XML
/// SDK output does and PowerPoint opens. Addressing by that id would have given one address two
/// answers; addressing by position does not.
#[test]
fn two_shapes_may_share_an_id_without_sharing_an_address() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");

    let ids: Vec<u32> = sealed
        .payload()
        .nodes
        .iter()
        .filter_map(|n| match &n.attributes {
            NodeAttributes::OfficeSlideRun(a) => Some(a.shape_id),
            _ => None,
        })
        .collect();
    assert!(
        ids.iter().filter(|id| **id == 2).count() > 1,
        "the fixture really does reuse a shape id: {ids:?}"
    );

    let first = locators(&sealed);
    let addresses: BTreeSet<(String, u32, u32, u32)> = first
        .iter()
        .map(|l| (l.part.clone(), l.shape, l.paragraph, l.run))
        .collect();
    assert_eq!(
        addresses.len(),
        first.len(),
        "every address is distinct even though the ids are not"
    );
}

/// **Groups are shapes.** They appeared on essentially every slide of every real deck measured,
/// so a reader that skipped them would return a fraction of the presentation and say nothing.
#[test]
fn a_shape_inside_a_group_is_read_and_addressed_like_any_other() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Inside a group, and still read.")
        .expect("a grouped shape's text is still the deck's text");

    match (&node.native_locator, &node.attributes) {
        (NativeLocator::Pptx(l), NodeAttributes::OfficeSlideRun(a)) => {
            assert_eq!(l.part, "ppt/slides/slide1.xml");
            assert_eq!(l.shape, 2, "the second `<p:sp>` in the part, group or not");
            assert_eq!(a.shape_name, "Grouped 2");
        }
        other => panic!("expected a slide run, got {other:?}"),
    }
}

/// Paragraphs and runs are numbered inside their shape, in the order the part lists them.
#[test]
fn paragraphs_and_runs_are_numbered_within_their_shape() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let slide_one: Vec<_> = locators(&sealed)
        .into_iter()
        .filter(|l| l.part.ends_with("slide1.xml"))
        .map(|l| (l.shape, l.paragraph, l.run))
        .collect();
    assert_eq!(slide_one, vec![(1, 1, 1), (1, 2, 1), (1, 2, 2), (2, 1, 1)]);
}

// -------------------------------------------------------------------------------------------
// The shape of the artifact
// -------------------------------------------------------------------------------------------

#[test]
fn a_deck_with_two_slides_seals_with_two_parts() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let payload = sealed.payload();

    let parents: BTreeSet<&str> = payload.nodes.iter().map(|n| n.parent.as_str()).collect();
    let parts: BTreeSet<String> = locators(&sealed).iter().map(|l| l.part.clone()).collect();
    assert_eq!(parents.len(), 2, "one part id per slide");
    assert_eq!(parts.len(), 2, "one part name per slide");

    for parent in &parents {
        let ordinals: Vec<u32> = payload
            .nodes
            .iter()
            .filter(|n| n.parent.as_str() == *parent)
            .map(|n| n.ordinal)
            .collect();
        let expected: Vec<u32> = (1..=ordinals.len() as u32).collect();
        assert_eq!(ordinals, expected, "ordinals restart at 1 in part {parent}");
    }
}

#[test]
fn the_artifact_declares_its_own_profile_and_its_own_media_type() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let payload = sealed.payload();

    assert_eq!(
        payload.source.media_type,
        ethos_parser_office::PPTX_MEDIA_TYPE
    );
    assert_eq!(
        payload.identity.profile_sha256,
        Profile::pptx_v0().profile_sha256().expect("a digest")
    );
    for other in [Profile::default(), Profile::docx_v0(), Profile::xlsx_v0()] {
        assert_ne!(
            Profile::pptx_v0().profile_sha256().expect("a digest"),
            other.profile_sha256().expect("a digest"),
            "a deck artifact is comparable with none of the other three"
        );
    }
    assert!(!Profile::pptx_v0().capabilities.measured_ink_boxes);
    // Decision #135: a slide's `<a:tbl>` is a table, read by the rule `table_detection.office`
    // names.
    assert!(Profile::pptx_v0().capabilities.tables);
    assert_eq!(
        Profile::pptx_v0().table_detection.office.as_deref(),
        Some(ethos_parser_core::TABLE_DETECTION_PPTX_TBL_V1)
    );
}

/// v2-S3 decided `coordinate_system` stays inert on page-less profiles. S4 adds a third and does
/// not reopen it — this is the test that says so.
#[test]
fn the_third_page_less_profile_keeps_the_same_inert_coordinate_system() {
    assert_eq!(
        Profile::pptx_v0().coordinate_system,
        Profile::docx_v0().coordinate_system
    );
    assert_eq!(
        Profile::pptx_v0().coordinate_system,
        Profile::default().coordinate_system,
        "unchanged, so no PDF artifact's profile hash moved for it"
    );

    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    assert!(
        sealed
            .geometry()
            .iter()
            .all(|g| matches!(g.presence, GeometryPresence::Absent(_))),
        "zero measured geometry rows is what keeps the declaration inert rather than wrong"
    );
}

#[test]
fn two_reads_of_one_deck_produce_identical_bytes() {
    let bytes = fixture("deck-slides");
    let first = ethos_parser_office::read(&bytes)
        .expect("reads")
        .to_canonical_bytes()
        .expect("canonical");
    let second = ethos_parser_office::read(&bytes)
        .expect("reads")
        .to_canonical_bytes()
        .expect("canonical");
    assert_eq!(first, second);
}

#[test]
fn text_is_what_the_slide_says_it_is() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    let texts: Vec<&str> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect();
    assert!(
        texts.contains(&"Slides & shapes are S4"),
        "the entity is resolved, not dropped: {texts:?}"
    );
    assert!(
        texts.contains(&" and never a page."),
        "DrawingML whitespace is always significant, so the leading space survives"
    );
}

// -------------------------------------------------------------------------------------------
// Declared erasure, detection, and every failure closed
// -------------------------------------------------------------------------------------------

/// **A14, both halves.** Parts that were not read, and shapes on the slides that were.
#[test]
fn parts_and_shapes_this_slice_does_not_read_are_declared_with_counts() {
    let sealed =
        ethos_parser_office::read(&fixture("deck-unread-parts")).expect("the fixture reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("notes, a layout and a master are three unread parts");

    // Matched on the exact count phrase, not on a digit and a word, so a looser assertion cannot
    // pass when the branch producing the count is deleted. Mutation-checked both ways.
    assert!(
        limitation.detail.contains("3 part(s)"),
        "the part count is in the message: {}",
        limitation.detail
    );
    // Decision #135: the fixture's table is read now, so the slide-number field is the one shape
    // left unread.
    assert!(
        limitation.detail.contains("1 shape(s)"),
        "and so is the shape count — deleting that branch must fail this: {}",
        limitation.detail
    );
    assert!(
        sealed
            .payload()
            .nodes
            .iter()
            .any(|n| n.text == "A table cell nobody read."),
        "the fixture's table cell is read since decision #135, its name notwithstanding"
    );

    for absent in [
        "A speaker note nobody read.",
        "Confidential",
        "Click to edit",
    ] {
        assert!(
            !sealed
                .payload()
                .nodes
                .iter()
                .any(|n| n.text.contains(absent)),
            "`{absent}` really is unread, so the declaration is not decorative"
        );
    }
    // The slide-number field's cached `1` is not evidence and is not in the record.
    assert!(!sealed.payload().nodes.iter().any(|n| n.text == "1"));
}

/// The other half of the guard: a deck with **no** unread parts but an unread shape still
/// declares. Authored inline because no realistic fixture has a chart and nothing else. A chart
/// frame stays unread since decision #135, which reads a frame's table.
#[test]
fn an_unread_shape_alone_is_enough_to_declare() {
    const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let archive = build_zip(&[
        (
            "ppt/presentation.xml",
            &format!(
                r#"<p:presentation xmlns:p="p" xmlns:r="{R}"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#
            ),
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            &format!(
                r#"<Relationships><Relationship Id="rId1" Type="{R}/slide" Target="slides/slide1.xml"/></Relationships>"#
            ),
        ),
        (
            "ppt/slides/slide1.xml",
            r#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree>
                 <p:sp><p:nvSpPr><p:cNvPr id="2" name="T"/></p:nvSpPr><p:txBody><a:p><a:r><a:t>read</a:t></a:r></a:p></p:txBody></p:sp>
                 <p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="Chart 2"/></p:nvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="c" r:id="rId2"/></a:graphicData></a:graphic></p:graphicFrame>
               </p:spTree></p:cSld></p:sld>"#,
        ),
    ]);

    let sealed = ethos_parser_office::read(&archive).expect("the deck reads");
    assert_eq!(sealed.payload().nodes.len(), 1);
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("an unread shape is a declared erasure even with every part read");
    assert!(
        limitation.detail.contains("1 shape(s)"),
        "{}",
        limitation.detail
    );
    assert!(
        !limitation.detail.contains("part(s)"),
        "and it does not claim an unread part it does not have: {}",
        limitation.detail
    );
}

#[test]
fn a_deck_with_nothing_unread_declares_no_erasure() {
    let sealed = ethos_parser_office::read(&fixture("deck-slides")).expect("the fixture reads");
    assert!(!sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ));
}

/// **A4: the bytes decide**, and three formats stay apart.
#[test]
fn detection_reads_the_bytes() {
    let pptx = fixture("deck-slides");
    assert!(ethos_parser_office::is_pptx(&pptx));
    assert!(!ethos_parser_office::is_docx(&pptx));
    assert!(!ethos_parser_office::is_xlsx(&pptx));
    assert!(ethos_parser_office::read(&pptx).is_ok());

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office");
    for (path, kind) in [
        ("simple-paragraphs/document.docx", "docx"),
        ("workbook-cells/workbook.xlsx", "xlsx"),
    ] {
        let other = std::fs::read(root.join(path)).expect("a sibling fixture");
        assert!(
            !ethos_parser_office::is_pptx(&other),
            "a {kind} is never claimed as a presentation"
        );
    }

    for not_a_deck in [
        // A legacy `.ppt` is an OLE compound file, not a ZIP, so it is refused at the first
        // question — which is right: it is a different format with a different reader.
        &b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1"[..],
        b"%PDF-1.7\n",
        b"",
        b"PK\x03\x04 but truncated right here",
        b"<p:sld/>",
    ] {
        assert!(!ethos_parser_office::is_pptx(not_a_deck));
    }
}

/// A package claiming to be more than one format is a named refusal, not a coin toss.
#[test]
fn a_package_that_is_two_formats_is_refused_by_name() {
    let archive = build_zip(&[
        ("ppt/presentation.xml", "<p:presentation/>"),
        ("xl/workbook.xml", "<workbook/>"),
    ]);
    assert!(ethos_parser_office::is_pptx(&archive));
    assert!(ethos_parser_office::is_xlsx(&archive));

    let error = ethos_parser_office::read(&archive).expect_err("ambiguous packages are refused");
    let text = error.to_string();
    assert!(text.contains("ppt/presentation.xml"), "{text}");
    assert!(text.contains("xl/workbook.xml"), "{text}");
}

/// A ZIP with none of the three main parts is refused, naming all three.
#[test]
fn a_zip_without_a_presentation_part_is_a_named_refusal() {
    let renamed = build_zip(&[("ppt/notadeck.xml", "<p:presentation/>")]);
    assert!(!ethos_parser_office::is_pptx(&renamed));

    let error = ethos_parser_office::read(&renamed).expect_err("a ZIP that is no office format");
    let text = error.to_string();
    for part in [
        "word/document.xml",
        "xl/workbook.xml",
        "ppt/presentation.xml",
    ] {
        assert!(text.contains(part), "the refusal names {part}: {text}");
    }
}

#[test]
fn a_truncated_deck_is_refused() {
    let full = fixture("deck-slides");
    for cut in [8, full.len() / 2, full.len() - 4] {
        let error =
            ethos_parser_office::read(&full[..cut]).expect_err("a truncated archive is refused");
        assert!(!error.to_string().is_empty());
    }
}

/// **Two entries of one name are refused, not read as the first** (review 2026-09-26).
///
/// The deck's second slide renamed to its first in both headers — the same length, so every offset
/// holds — leaves two `ppt/slides/slide1.xml`, the second of which a consumer preferring the last
/// would show. The fixture reads, so the refusal is the duplicate's.
#[test]
fn two_entries_of_one_name_are_refused_rather_than_read_as_the_first() {
    let mut bytes = fixture("deck-slides");
    ethos_parser_office::read(&bytes).expect("the fixture reads");
    let (from, to) = (b"ppt/slides/slide7.xml", b"ppt/slides/slide1.xml");
    let mut renamed = 0;
    let mut at = 0;
    while at + from.len() <= bytes.len() {
        if &bytes[at..at + from.len()] == from {
            bytes[at..at + from.len()].copy_from_slice(to);
            renamed += 1;
        }
        at += 1;
    }
    assert_eq!(renamed, 2, "the local header and the directory");

    let error = ethos_parser_office::read(&bytes).expect_err("two entries of one name");
    let text = error.to_string();
    assert!(
        matches!(error, ethos_parser_core::EngineError::Malformed { .. }),
        "{text}"
    );
    assert!(
        text.contains("`ppt/slides/slide1.xml` more than once"),
        "{text}"
    );
}

// -------------------------------------------------------------------------------------------
// Helpers — a minimal stored-entry ZIP, so a negative case needs no fixture file
// -------------------------------------------------------------------------------------------

fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for (name, body) in entries {
        let offset = out.len() as u32;
        let data = body.as_bytes();
        let crc = crc32(data);

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&[10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }

    let directory_offset = out.len() as u32;
    let directory_size = directory.len() as u32;
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

// -------------------------------------------------------------------------------------------
// Decision #135: a slide's `<a:tbl>` is a table
// -------------------------------------------------------------------------------------------

/// A one-slide deck whose spTree is `body`.
fn deck(body: &str) -> Vec<u8> {
    const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    build_zip(&[
        (
            "ppt/presentation.xml",
            &format!(
                r#"<p:presentation xmlns:p="p" xmlns:r="{R}"><p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst></p:presentation>"#
            ),
        ),
        (
            "ppt/_rels/presentation.xml.rels",
            &format!(
                r#"<Relationships><Relationship Id="rId1" Type="{R}/slide" Target="slides/slide1.xml"/></Relationships>"#
            ),
        ),
        (
            "ppt/slides/slide1.xml",
            &format!(
                r#"<p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree>{body}</p:spTree></p:cSld></p:sld>"#
            ),
        ),
    ])
}

fn tc(attributes: &str, text: &str) -> String {
    let body = if text.is_empty() {
        "<a:p/>".to_string()
    } else {
        format!("<a:p><a:r><a:t>{text}</a:t></a:r></a:p>")
    };
    format!("<a:tc{attributes}><a:txBody>{body}</a:txBody></a:tc>")
}

/// A title shape, then a frame holding a 3 x 3 table: a heading cell spanning three columns, a
/// cell spanning two rows, and an empty cell.
fn results_deck() -> Vec<u8> {
    let frame = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="4" name="Table 3"/></p:nvGraphicFramePr><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblGrid><a:gridCol/><a:gridCol/><a:gridCol/></a:tblGrid><a:tr>{}{}{}</a:tr><a:tr>{}{}{}</a:tr><a:tr>{}{}{}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        tc(r#" gridSpan="3""#, "Results"),
        tc(r#" hMerge="1""#, ""),
        tc(r#" hMerge="1""#, ""),
        tc(r#" rowSpan="2""#, "Q1"),
        tc("", "212"),
        tc("", ""),
        tc(r#" vMerge="1""#, ""),
        tc("", "248"),
        "<a:tc><a:txBody><a:p><a:r><a:t>up</a:t></a:r><a:r><a:t> 17%</a:t></a:r></a:p></a:txBody></a:tc>",
    );
    deck(&format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title 1"/></p:nvSpPr><p:txBody><a:p><a:r><a:t>Q2 results</a:t></a:r></a:p></p:txBody></p:sp>{frame}"#
    ))
}

/// **A slide table is one table, its cells at the rows and columns the slide states.** A cell
/// holds its runs; a merge's covered cells open no slot; a cell's run is addressed by frame, row
/// and column. The failure this catches: a table counted as unread, a covered cell given a slot,
/// or a cell's run addressed as a shape's.
#[test]
fn a_slide_table_is_one_table_at_the_rows_and_columns_it_states() {
    let sealed = ethos_parser_office::read(&results_deck()).expect("reads");
    let payload = sealed.payload();
    assert_eq!(payload.tables.len(), 1);
    let table = &payload.tables[0];
    assert_eq!(
        table.detection_rule,
        ethos_parser_core::TABLE_DETECTION_PPTX_TBL_V1
    );
    assert_eq!(
        table.derivation,
        ethos_parser_core::DerivationClass::Extracted
    );
    assert_eq!((table.rows, table.columns), (3, 3));
    let spans: Vec<_> = table
        .cells
        .iter()
        .map(|c| {
            (
                c.position.row,
                c.position.column,
                c.position.rowspan,
                c.position.colspan,
                c.text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        spans,
        vec![
            (0, 0, 1, 3, "Results"),
            (1, 0, 2, 1, "Q1"),
            (1, 1, 1, 1, "212"),
            (1, 2, 1, 1, ""),
            (2, 1, 1, 1, "248"),
            (2, 2, 1, 1, "up 17%"),
        ]
    );
    let up = &table.cells[5];
    assert_eq!(up.node_ids.len(), 2, "a cell holds each run read in it");

    // The title shape is shape 1; the table's runs are frame 1, by row and column.
    let at: Vec<_> = locators(&sealed)
        .into_iter()
        .map(|l| (l.shape, l.row, l.column, l.paragraph, l.run))
        .collect();
    assert_eq!(
        at,
        vec![
            (1, None, None, 1, 1),
            (1, Some(1), Some(1), 1, 1),
            (1, Some(2), Some(1), 1, 1),
            (1, Some(2), Some(2), 1, 1),
            (1, Some(3), Some(2), 1, 1),
            (1, Some(3), Some(3), 1, 1),
            (1, Some(3), Some(3), 1, 2),
        ]
    );
    let run = payload
        .nodes
        .iter()
        .find(|n| n.text == "212")
        .expect("a cell's run is a node");
    match &run.attributes {
        NodeAttributes::OfficeSlideRun(a) => {
            assert_eq!((a.shape_id, a.shape_name.as_str()), (4, "Table 3"))
        }
        other => panic!("{other:?}"),
    }
    assert!(!payload
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ));
}

/// **The Markdown is the grid**, after the title, and the census balances.
#[test]
fn a_slide_table_projects_as_a_grid() {
    let sealed = ethos_parser_office::read(&results_deck()).expect("reads");
    let profile = Profile::default();
    let md = ethos_parser_core::to_markdown(
        &sealed,
        &profile.parser_version,
        &profile.profile_sha256().unwrap(),
        &profile.markdown_rule,
    )
    .expect("projects");
    assert_eq!(
        md.markdown,
        "Q2 results\n\n| Results |  |  |\n| --- | --- | --- |\n| Q1 | 212 |  |\n|  | 248 | up 17% |\n"
    );
    assert!(md.coverage.balances());
}

/// **A locator for a cell's run round-trips, and a shape's run has no row or column on the wire.**
#[test]
fn a_cell_run_locator_carries_row_and_column_and_a_shape_run_does_not() {
    let sealed = ethos_parser_office::read(&results_deck()).expect("reads");
    let wire = serde_json::to_value(sealed.payload()).expect("json");
    let nodes = wire["nodes"].as_array().expect("nodes");
    assert_eq!(
        nodes[0]["native_locator"]["pptx"],
        serde_json::json!({"part": "ppt/slides/slide1.xml", "shape": 1, "paragraph": 1, "run": 1})
    );
    assert_eq!(
        nodes[2]["native_locator"]["pptx"],
        serde_json::json!({"part": "ppt/slides/slide1.xml", "shape": 1, "paragraph": 1, "run": 1, "row": 2, "column": 1})
    );
}

// -------------------------------------------------------------------------------------------
// Decision #135, after review: spans inside the grid, covered cells, a cell's paragraphs
// -------------------------------------------------------------------------------------------

fn frame(id: u32, rows: &str) -> String {
    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{id}" name="Table {id}"/></p:nvGraphicFramePr><a:graphic><a:graphicData><a:tbl>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    )
}

/// **A span reaches no further than the grid the table states.** Ten one-cell tables each
/// spanning 999 x 1,000 were ten million slots from a 921-byte deck.
#[test]
fn a_span_reaches_no_further_than_the_stated_grid() {
    let body: String = (1..=10)
        .map(|i| {
            frame(
                i,
                &format!(
                    "<a:tr>{}</a:tr>",
                    tc(r#" rowSpan="999" gridSpan="1000""#, "A")
                ),
            )
        })
        .collect();
    let sealed = ethos_parser_office::read(&deck(&body)).expect("reads");
    let tables = &sealed.payload().tables;
    assert_eq!(tables.len(), 10);
    for t in tables {
        assert_eq!((t.rows, t.columns), (1, 1));
        let p = &t.cells[0].position;
        assert_eq!((p.rowspan, p.colspan), (1, 1));
    }
}

/// **A cell under a span takes no slot, and a covered cell's text is kept and counted.** `B`
/// follows a `gridSpan="2"` with no `hMerge`; `ghost` is marked `hMerge`. Both stay nodes.
#[test]
fn a_covered_cell_takes_no_slot_and_its_text_is_counted() {
    let body = frame(
        4,
        &format!(
            "<a:tr>{}{}</a:tr><a:tr>{}{}</a:tr>",
            tc(r#" gridSpan="2""#, "A"),
            tc("", "B"),
            tc("", "C"),
            tc(r#" hMerge="1""#, "ghost"),
        ),
    );
    let sealed = ethos_parser_office::read(&deck(&body)).expect("reads");
    let t = &sealed.payload().tables[0];
    let cells: Vec<_> = t
        .cells
        .iter()
        .map(|c| {
            (
                c.position.row,
                c.position.column,
                c.position.colspan,
                c.text.as_str(),
            )
        })
        .collect();
    assert_eq!(cells, vec![(0, 0, 2, "A"), (1, 0, 1, "C")]);
    for text in ["B", "ghost"] {
        assert!(
            sealed.payload().nodes.iter().any(|n| n.text == text),
            "{text}"
        );
    }
    let l = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::OFFICE_COVERED_CELLS_NOT_PLACED)
        .expect("declared");
    assert!(l.detail.starts_with("2 cell(s)"), "{}", l.detail);
}

/// **A cell's runs of one paragraph join as they stand** (review of decision #135): formatting cut
/// `Revenue` into `Rev` and `enue`, and the cell read `Rev enue` in the record, in Markdown and in
/// HTML. Its paragraphs still join with a space.
#[test]
fn a_cells_runs_of_one_paragraph_join_as_they_stand() {
    let body = frame(
        4,
        "<a:tr><a:tc><a:txBody><a:p><a:r><a:t>Rev</a:t></a:r><a:r><a:t>enue</a:t></a:r></a:p><a:p><a:r><a:t>FY</a:t></a:r><a:r><a:t>2026</a:t></a:r></a:p></a:txBody></a:tc></a:tr>",
    );
    let sealed = ethos_parser_office::read(&deck(&body)).expect("reads");
    assert_eq!(sealed.payload().tables[0].cells[0].text, "Revenue FY2026");
    let profile = Profile::default();
    let sha = profile.profile_sha256().unwrap();
    let md = ethos_parser_core::to_markdown(
        &sealed,
        &profile.parser_version,
        &sha,
        &profile.markdown_rule,
    )
    .expect("projects");
    assert_eq!(md.markdown, "| Revenue FY2026 |\n| --- |\n");
    let html =
        ethos_parser_core::to_html(&sealed, &profile.parser_version, &sha, &profile.html_rule)
            .expect("projects");
    assert!(html.html.contains(">Revenue FY2026<"), "{}", html.html);
}

/// **A line break parts a cell's runs** (review of decision #135): `North<a:br/>America` in one
/// paragraph read `NorthAmerica` in the record, in Markdown and in HTML, once the runs of a
/// paragraph joined as they stand. A shape's runs are blocks of their own, as before.
#[test]
fn a_line_break_in_a_cell_parts_its_runs() {
    let para = "<a:p><a:r><a:t>North</a:t></a:r><a:br/><a:r><a:t>America</a:t></a:r></a:p>";
    let body = format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Title 1"/></p:nvSpPr><p:txBody>{para}</p:txBody></p:sp>{}"#,
        frame(
            4,
            &format!("<a:tr><a:tc><a:txBody>{para}</a:txBody></a:tc></a:tr>")
        )
    );
    let sealed = ethos_parser_office::read(&deck(&body)).expect("reads");
    assert_eq!(sealed.payload().tables[0].cells[0].text, "North America");
    let breaks: Vec<_> = sealed
        .payload()
        .nodes
        .iter()
        .filter_map(|n| match &n.attributes {
            NodeAttributes::OfficeSlideRun(a) => Some((n.text.as_str(), a.line_break_before)),
            _ => None,
        })
        .collect();
    assert_eq!(
        breaks,
        vec![
            ("North", false),
            ("America", false),
            ("North", false),
            ("America", true)
        ]
    );
    let profile = Profile::default();
    let sha = profile.profile_sha256().unwrap();
    let md = ethos_parser_core::to_markdown(
        &sealed,
        &profile.parser_version,
        &sha,
        &profile.markdown_rule,
    )
    .expect("projects");
    assert_eq!(
        md.markdown,
        "North\n\nAmerica\n\n| North America |\n| --- |\n"
    );
    let html =
        ethos_parser_core::to_html(&sealed, &profile.parser_version, &sha, &profile.html_rule)
            .expect("projects");
    assert!(html.html.contains(">North America<"), "{}", html.html);
}

/// **A row in a branch passed over is no row of the table** (review of decision #135): an
/// `<mc:Fallback>` row beside the `<mc:Choice>` one read added an empty row. The locators still
/// count it, as they count every `<a:tr>` the file holds.
#[test]
fn a_row_in_a_branch_passed_over_is_no_row_of_the_table() {
    let row = |a: &str, b: &str| format!("<a:tr>{}{}</a:tr>", tc("", a), tc("", b));
    let body = frame(
        4,
        &format!(
            "{}<mc:AlternateContent xmlns:mc=\"mc\"><mc:Choice Requires=\"a14\">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>{}",
            row("h1", "h2"),
            row("new1", "new2"),
            row("old1", "old2"),
            row("x1", "x2")
        ),
    );
    let sealed = ethos_parser_office::read(&deck(&body)).expect("reads");
    let t = &sealed.payload().tables[0];
    assert_eq!((t.rows, t.columns), (3, 2));
    let cells: Vec<_> = t
        .cells
        .iter()
        .map(|c| (c.position.row, c.position.column, c.text.as_str()))
        .collect();
    assert_eq!(
        cells,
        vec![
            (0, 0, "h1"),
            (0, 1, "h2"),
            (1, 0, "new1"),
            (1, 1, "new2"),
            (2, 0, "x1"),
            (2, 1, "x2"),
        ]
    );
    let rows: Vec<_> = locators(&sealed).into_iter().map(|l| l.row).collect();
    assert_eq!(
        rows,
        vec![Some(1), Some(1), Some(2), Some(2), Some(4), Some(4)],
        "the fallback row keeps its place in the count"
    );
}

/// **A cell's paragraphs are joined as the projections join them**: `Revenue FY2026`, not
/// `RevenueFY2026`.
#[test]
fn a_cells_paragraphs_join_with_a_space() {
    let body = frame(
        4,
        "<a:tr><a:tc><a:txBody><a:p><a:r><a:t>Revenue</a:t></a:r></a:p><a:p><a:r><a:t>FY2026</a:t></a:r></a:p></a:txBody></a:tc></a:tr>",
    );
    let sealed = ethos_parser_office::read(&deck(&body)).expect("reads");
    assert_eq!(sealed.payload().tables[0].cells[0].text, "Revenue FY2026");
}
