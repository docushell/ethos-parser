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

    assert_eq!(payload.source.media_type, ethos_parser_office::PPTX_MEDIA_TYPE);
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
    assert!(!Profile::pptx_v0().capabilities.tables);
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
    let sealed = ethos_parser_office::read(&fixture("deck-unread-parts")).expect("the fixture reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("notes, a layout and a master are three unread parts");

    // Matched on the exact count phrase, not on a digit and a word: `v2-S4 reads slide shape
    // text only` contains both a `2` and `shape`, so a looser assertion passes even when the
    // branch producing the count is deleted. Mutation-checked both ways.
    assert!(
        limitation.detail.contains("3 part(s)"),
        "the part count is in the message: {}",
        limitation.detail
    );
    assert!(
        limitation.detail.contains("2 shape(s)"),
        "and so is the shape count — deleting that branch must fail this: {}",
        limitation.detail
    );

    for absent in [
        "A speaker note nobody read.",
        "A table cell nobody read.",
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
/// declares. Authored inline because no realistic fixture has a table and nothing else.
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
                 <p:graphicFrame><a:tbl><a:tc><a:txBody><a:p><a:r><a:t>unread</a:t></a:r></a:p></a:txBody></a:tc></a:tbl></p:graphicFrame>
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
        let error = ethos_parser_office::read(&full[..cut]).expect_err("a truncated archive is refused");
        assert!(!error.to_string().is_empty());
    }
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
