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

//! v2-S5's coverage, executable: **an ODT paragraph binds, and the page in the file stays out**.
//!
//! The fourth sibling of `docx_representation.rs`, and the first over a container that is not
//! OOXML. Most of its assertions are the ones the first three made. The ones that are new are all
//! about the same fact: this is the only v2 format whose file **contains a page break**, so the
//! tests that `pages` is empty and that the locator has two fields are the ones carrying weight.

use std::collections::BTreeSet;
use std::path::PathBuf;

use engine_core::{
    GeometryAbsence, GeometryPresence, NativeLocator, NodeAttributes, OdfBlockKind, OdtLocator,
    Profile,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("document.odt");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn locators(sealed: &engine_core::DocumentRepresentation) -> Vec<OdtLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Odt(l) => l.clone(),
            other => panic!("an ODF paragraph must carry a text-document address, not {other:?}"),
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The slice's sentence, executable.** A known phrase is on a node, addressed by the file.
#[test]
fn a_paragraph_resolves_to_a_node_addressed_by_the_document_itself() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let payload = sealed.payload();

    let node = payload
        .nodes
        .iter()
        .find(|n| n.text == "Evidence, not extraction.")
        .expect("the phrase is a heading in the document, so it is in the artifact");

    match (&node.native_locator, &node.attributes) {
        (NativeLocator::Odt(l), NodeAttributes::OfficeParagraph(a)) => {
            assert_eq!(l.part, "content.xml");
            assert_eq!(l.paragraph, 1);
            assert_eq!(
                a.block,
                OdfBlockKind::Heading,
                "the element name says heading, and nothing inferred it from a font size"
            );
        }
        other => panic!("expected an ODF paragraph, got {other:?}"),
    }

    assert_eq!(payload.nodes.iter().filter(|n| n.id == node.id).count(), 1);
    assert!(!payload.nodes.iter().any(|n| n.id.as_str() == "s-forged"));
}

/// **The format whose file contains a page, and still declares none** — §3 for this format.
///
/// The fixture carries a `<text:soft-page-break/>`, so this is not vacuous: the element a reader
/// would mint a `PageRecord` from is present in the bytes being read.
#[test]
fn a_text_document_declares_no_pages_and_carries_no_page_number() {
    let bytes = fixture("text-paragraphs");

    // **Read out of the fixture's own `content.xml`**, not out of the generator that wrote it.
    // The first version of this guard grepped `make_fixtures.py` for the literal — which appears
    // there twice, once inside a `#` comment explaining the element, so deleting the real one from
    // the fixture left the assertion passing. It also proved a property of a Python source file
    // rather than of the bytes under test. This inflates the entry and looks.
    let content = String::from_utf8(
        engine_office::zip::read_entry(&bytes, "content.xml")
            .expect("the fixture has a content part"),
    )
    .expect("content.xml is UTF-8");
    assert!(
        content.contains("<text:soft-page-break/>"),
        "the fixture really does contain the producing application's own page break, so the \
         empty `pages` vector below is a refusal rather than an absence: {content}"
    );

    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    let payload = sealed.payload();

    assert!(
        payload.pages.is_empty(),
        "a soft page break is the producing application's layout, not a page this engine read"
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

    // The field set is the check that matters: no page index, no box, and no room for one.
    let wire = serde_json::to_value(&payload.nodes[0].native_locator).expect("serializes");
    let fields: BTreeSet<&str> = wire["odt"]
        .as_object()
        .expect("an object")
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert_eq!(fields, BTreeSet::from(["part", "paragraph"]));

    for smuggled in [
        r#"{"part":"content.xml","paragraph":1,"page":3}"#,
        r#"{"part":"content.xml","paragraph":1,"bbox":[0,0,10,10]}"#,
        r#"{"part":"content.xml","paragraph":1,"x":0}"#,
        r#"{"part":"content.xml","paragraph":1,"soft_page_break":2}"#,
    ] {
        assert!(
            serde_json::from_str::<OdtLocator>(smuggled).is_err(),
            "`deny_unknown_fields` refuses {smuggled} by name"
        );
    }
}

/// **The highest-value test in the slice.** The address is the position in the *file*, so it
/// counts blocks this reader did not read.
///
/// The second fixture's footnote and comment each hold a `<text:p>`, so the blocks that follow
/// them are 3 and 5. A reader that numbered only what it kept would call them 2 and 3, and every
/// citation after a footnote would name the wrong paragraph.
#[test]
fn the_block_count_advances_through_regions_that_are_not_read() {
    let sealed = engine_office::read(&fixture("text-unread-parts")).expect("the fixture reads");

    let addressed: Vec<(u32, u32, &str)> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Odt(l) => (n.ordinal, l.paragraph, n.text.as_str()),
            other => panic!("expected an ODF paragraph, got {other:?}"),
        })
        .collect();

    assert_eq!(
        addressed,
        vec![
            (1, 1, "The body is all this slice reads."),
            (2, 3, "Reviewed and unchanged."),
            (3, 5, "After both, and still block five."),
        ],
        "the ordinal counts nodes in this artifact; the locator counts blocks in the file, and \
         the two are deliberately different numbers"
    );
}

/// **A self-closing `<text:p/>` is a block the file contains**, so the address does not turn on
/// how the document was serialized — the defect v2-S4 found in a deck, checked here before it
/// could ship.
#[test]
fn an_empty_block_holds_its_position_without_becoming_a_node() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let paragraphs: Vec<u32> = locators(&sealed).iter().map(|l| l.paragraph).collect();
    assert_eq!(
        paragraphs,
        vec![1, 2, 4, 5, 6, 7],
        "block 3 is the `<text:p/>` blank line: no text, so no node, and every later address \
         still names the position the file gives it"
    );
}

/// **Both block kinds are pinned, and the mapping between them.** Only `Heading` was asserted
/// anywhere, so `read_odt`'s `if block.heading` could have been the constant `Heading` and every
/// test in the repo would have passed — shipping five ordinary paragraphs each claiming to be a
/// heading, a structural role the file does not state, at correct addresses.
#[test]
fn both_block_kinds_reach_the_artifact_as_the_file_states_them() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let kinds: Vec<(u32, OdfBlockKind)> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match (&n.native_locator, &n.attributes) {
            (NativeLocator::Odt(l), NodeAttributes::OfficeParagraph(a)) => (l.paragraph, a.block),
            other => panic!("expected an ODF paragraph, got {other:?}"),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            (1, OdfBlockKind::Heading),
            (2, OdfBlockKind::Paragraph),
            (4, OdfBlockKind::Paragraph),
            (5, OdfBlockKind::Paragraph),
            (6, OdfBlockKind::Paragraph),
            (7, OdfBlockKind::Paragraph),
        ],
        "one `<text:h>` and five `<text:p>`, each labelled by its element name"
    );
}

/// **Document order survives nesting.** `read_content` sorts by ordinal because an outer block
/// closes *after* the inner ones it contains, so pop order is not file order — and every fixture
/// and test until now nested only into a region that emits nothing or a block with no text of its
/// own, which made the sort a no-op everywhere it ran.
#[test]
fn a_block_nested_inside_a_kept_block_does_not_reorder_the_artifact() {
    let archive = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            (
                "META-INF/manifest.xml",
                r#"<manifest:manifest xmlns:manifest="m">
                     <manifest:file-entry manifest:full-path="content.xml"/>
                   </manifest:manifest>"#,
            ),
            (
                "content.xml",
                concat!(
                    r#"<office:document-content"#,
                    r#" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0""#,
                    r#" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0""#,
                    r#" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0">"#,
                    r#"<office:body><office:text>"#,
                    // The outer block has text of its own on *both* sides of the frame, so it
                    // survives to be emitted and its close really does come after the inner one's.
                    r#"<text:p>Caption follows<draw:frame><draw:text-box>"#,
                    r#"<text:p>Inside.</text:p>"#,
                    r#"</draw:text-box></draw:frame> end.</text:p>"#,
                    r#"</office:text></office:body></office:document-content>"#
                ),
            ),
        ],
    );

    let sealed = engine_office::read(&archive).expect("the document reads");
    let addressed: Vec<(u32, u32, &str)> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Odt(l) => (n.ordinal, l.paragraph, n.text.as_str()),
            other => panic!("expected an ODF paragraph, got {other:?}"),
        })
        .collect();
    assert_eq!(
        addressed,
        vec![(1, 1, "Caption follows end."), (2, 2, "Inside.")],
        "the anchoring block is block 1 and the text box's is block 2, and the artifact's own \
         reading order agrees — deleting the sort reverses them"
    );
}

/// **A sentence is one node**, not one per `<text:span>` — see `OdtLocator`'s own documentation.
#[test]
fn a_span_does_not_split_a_sentence_into_addresses_a_quote_cannot_bind_to() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let texts: Vec<&str> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect();
    assert!(
        texts.contains(&"A quote binds to a paragraph and never to a page."),
        "the span is inside the sentence, and the whole sentence is what a citation quotes: \
         {texts:?}"
    );
}

/// ODF's own whitespace rule, and the three elements exempt from it, over a real package.
#[test]
fn the_text_is_what_the_document_states_rather_than_how_it_was_indented() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let texts: Vec<&str> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect();

    assert!(
        texts.contains(&"A quote binds to a paragraph and never to a page."),
        "the fixture indents that sentence across three source lines: {texts:?}"
    );
    assert!(
        texts.contains(&"Rows & columns\tare tabbed."),
        "the entity is resolved and `<text:tab/>` is the tab it states: {texts:?}"
    );
    assert!(
        texts.contains(&"Three spaces:   stated, not measured."),
        "`<text:s text:c=\"3\">` is a count the file wrote down: {texts:?}"
    );
    assert!(
        texts.contains(&"Split by the producer and rejoined here."),
        "and the soft page break contributes no character either: {texts:?}"
    );
}

/// A block inside a table cell is a block, addressed like any other.
#[test]
fn a_table_cell_paragraph_is_read_and_no_table_is_claimed() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    assert!(sealed
        .payload()
        .nodes
        .iter()
        .any(|n| n.text == "In a cell, and still a paragraph."));
    assert!(
        sealed.payload().tables.is_empty(),
        "`tables` is this engine's table IR, produced by the PDF detectors; none of them ran"
    );
}

// -------------------------------------------------------------------------------------------
// The shape of the artifact
// -------------------------------------------------------------------------------------------

#[test]
fn a_text_document_seals_as_one_part_with_contiguous_ordinals() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let payload = sealed.payload();

    let parents: BTreeSet<&str> = payload.nodes.iter().map(|n| n.parent.as_str()).collect();
    let parts: BTreeSet<String> = locators(&sealed).iter().map(|l| l.part.clone()).collect();
    assert_eq!(parents.len(), 1, "one content part, one part id");
    assert_eq!(parts, BTreeSet::from(["content.xml".to_string()]));

    let ordinals: Vec<u32> = payload.nodes.iter().map(|n| n.ordinal).collect();
    assert_eq!(ordinals, (1..=ordinals.len() as u32).collect::<Vec<u32>>());
}

#[test]
fn the_artifact_declares_its_own_profile_and_its_own_media_type() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    let payload = sealed.payload();

    assert_eq!(payload.source.media_type, engine_office::ODT_MEDIA_TYPE);
    assert_eq!(
        payload.source.media_type, "application/vnd.oasis.opendocument.text",
        "spelled out, so a rename of the constant cannot change what goes on the wire"
    );
    assert_eq!(
        payload.identity.profile_sha256,
        Profile::odt_v0().profile_sha256().expect("a digest")
    );
    for other in [
        Profile::default(),
        Profile::docx_v0(),
        Profile::xlsx_v0(),
        Profile::pptx_v0(),
    ] {
        assert_ne!(
            Profile::odt_v0().profile_sha256().expect("a digest"),
            other.profile_sha256().expect("a digest"),
            "an ODT artifact is comparable with none of the other four"
        );
    }
    assert!(!Profile::odt_v0().capabilities.measured_ink_boxes);
    assert!(!Profile::odt_v0().capabilities.tables);
}

/// **All five profiles, every pair.** Each earlier slice asserted its own profile against the ones
/// before it, so mutual distinctness held only by transitivity across four test files. Stated once
/// here, so a fifth format cannot collide with a third by nobody having compared them.
#[test]
fn every_pair_of_profiles_is_distinct() {
    let profiles = [
        ("pdf", Profile::default()),
        ("docx", Profile::docx_v0()),
        ("xlsx", Profile::xlsx_v0()),
        ("pptx", Profile::pptx_v0()),
        ("odt", Profile::odt_v0()),
    ];
    for (i, (left_name, left)) in profiles.iter().enumerate() {
        for (right_name, right) in profiles.iter().skip(i + 1) {
            assert_ne!(
                left.profile_sha256().expect("a digest"),
                right.profile_sha256().expect("a digest"),
                "`{left_name}` and `{right_name}` hash the same, so their artifacts would compare \
                 as though one reader produced both"
            );
        }
    }
}

/// v2-S3 decided `coordinate_system` stays inert on page-less profiles. S5 adds a fourth and does
/// not reopen it.
#[test]
fn the_fourth_page_less_profile_keeps_the_same_inert_coordinate_system() {
    assert_eq!(
        Profile::odt_v0().coordinate_system,
        Profile::docx_v0().coordinate_system
    );
    assert_eq!(
        Profile::odt_v0().coordinate_system,
        Profile::default().coordinate_system,
        "unchanged, so no PDF artifact's profile hash moved for it"
    );

    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    assert!(
        sealed
            .geometry()
            .iter()
            .all(|g| matches!(g.presence, GeometryPresence::Absent(_))),
        "zero measured geometry rows is what keeps the declaration inert rather than wrong"
    );
}

#[test]
fn two_reads_of_one_document_produce_identical_bytes() {
    let bytes = fixture("text-paragraphs");
    let first = engine_office::read(&bytes)
        .expect("reads")
        .to_canonical_bytes()
        .expect("canonical");
    let second = engine_office::read(&bytes)
        .expect("reads")
        .to_canonical_bytes()
        .expect("canonical");
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// Declared erasure, detection, and every failure closed
// -------------------------------------------------------------------------------------------

/// **A14, both halves.** Package entries that were not read, and regions of the part that was.
#[test]
fn unread_entries_and_unread_regions_are_declared_with_counts() {
    let sealed = engine_office::read(&fixture("text-unread-parts")).expect("the fixture reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("styles, metadata and a picture are three unread entries");

    // Matched on the exact count phrases, not on a digit and a word: the trailing sentence of the
    // message contains a `5` and the word `read`, so a looser assertion would pass with either
    // branch deleted. Mutation-checked both ways.
    assert!(
        limitation.detail.contains("3 entry(ies)"),
        "the entry count is in the message: {}",
        limitation.detail
    );
    assert!(
        limitation.detail.contains("2 region(s)"),
        "and so is the region count — deleting that branch must fail this: {}",
        limitation.detail
    );

    for absent in [
        "A source nobody read.",
        "A remark nobody read.",
        "Confidential",
        "A title nobody read",
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
}

/// The other half of the guard: a package with every entry read but a region that was not still
/// declares. Authored inline, because a realistic package always carries `styles.xml`.
#[test]
fn an_unread_region_alone_is_enough_to_declare() {
    let archive = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            (
                "META-INF/manifest.xml",
                r#"<manifest:manifest xmlns:manifest="m">
                     <manifest:file-entry manifest:full-path="content.xml"/>
                   </manifest:manifest>"#,
            ),
            (
                "content.xml",
                concat!(
                    r#"<office:document-content"#,
                    r#" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0""#,
                    r#" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">"#,
                    r#"<office:body><office:text>"#,
                    r#"<text:p>read<text:note><text:note-body><text:p>unread</text:p>"#,
                    r#"</text:note-body></text:note></text:p>"#,
                    r#"</office:text></office:body></office:document-content>"#
                ),
            ),
        ],
    );

    let sealed = engine_office::read(&archive).expect("the document reads");
    assert_eq!(sealed.payload().nodes.len(), 1);
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("an unread region is a declared erasure even with every entry read");
    assert!(
        limitation.detail.contains("1 region(s)"),
        "{}",
        limitation.detail
    );
    assert!(
        !limitation.detail.contains("entry(ies)"),
        "and it does not claim an unread entry it does not have: {}",
        limitation.detail
    );
}

#[test]
fn a_document_with_nothing_unread_declares_no_erasure() {
    let sealed = engine_office::read(&fixture("text-paragraphs")).expect("the fixture reads");
    assert!(!sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == engine_core::assurance::codes::OFFICE_PARTS_NOT_READ));
}

/// **A4: the bytes decide**, and four formats stay apart.
#[test]
fn detection_reads_the_bytes() {
    let odt = fixture("text-paragraphs");
    assert!(engine_office::is_odt(&odt));
    assert!(!engine_office::is_docx(&odt));
    assert!(!engine_office::is_xlsx(&odt));
    assert!(!engine_office::is_pptx(&odt));
    assert!(engine_office::read(&odt).is_ok());

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office");
    for (path, kind) in [
        ("simple-paragraphs/document.docx", "docx"),
        ("workbook-cells/workbook.xlsx", "xlsx"),
        ("deck-slides/deck.pptx", "pptx"),
    ] {
        let other = std::fs::read(root.join(path)).expect("a sibling fixture");
        assert!(
            !engine_office::is_odt(&other),
            "a {kind} is never claimed as an OpenDocument text document"
        );
    }

    for not_a_document in [
        &b"%PDF-1.7\n"[..],
        b"",
        b"PK\x03\x04 but truncated right here",
        b"<office:document-content/>",
        // A legacy `.sxw` is a ZIP with a different mimetype, and so is every other ODF format.
        b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1",
    ] {
        assert!(!engine_office::is_odt(not_a_document));
    }
}

/// **A spreadsheet and a presentation are the same container**, and neither is claimed here.
///
/// The single reason detection asks the `mimetype` entry rather than looking for `content.xml`: an
/// `.ods` has one too, full of `<table:table-cell>`, and reading it with this vocabulary would
/// return a document with no text and no error — a gap presented as a success.
#[test]
fn a_spreadsheet_and_a_presentation_are_not_claimed_as_text() {
    for (media_type, kind) in [
        ("application/vnd.oasis.opendocument.spreadsheet", "ods"),
        ("application/vnd.oasis.opendocument.presentation", "odp"),
        ("application/vnd.oasis.opendocument.graphics", "odg"),
    ] {
        let archive = build_odt(
            media_type,
            &[
                ("META-INF/manifest.xml", "<manifest:manifest/>"),
                ("content.xml", "<office:document-content/>"),
            ],
        );
        assert!(
            !engine_office::is_odt(&archive),
            "an {kind} is a different vocabulary with a different reader"
        );
        assert!(
            engine_office::read(&archive).is_err(),
            "and it is refused rather than read as an empty text document"
        );
    }
}

/// The `mimetype` entry has to be **first** and **stored**, because the package specification says
/// so — and a package that does not do it is not one this reader will identify from its bytes.
#[test]
fn a_mimetype_that_is_not_first_and_stored_is_not_an_odf_package() {
    let conforming = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            ("META-INF/manifest.xml", "<manifest:manifest/>"),
            ("content.xml", "<office:document-content/>"),
        ],
    );
    assert!(engine_office::is_odt(&conforming));

    // The same entries with `mimetype` no longer first.
    let reordered = build_zip(&[
        ("content.xml", "<office:document-content/>"),
        ("mimetype", "application/vnd.oasis.opendocument.text"),
    ]);
    assert!(!engine_office::is_odt(&reordered));
    assert!(engine_office::read(&reordered).is_err());

    // **And the compression half, which had no test at all.** `build_zip` stores every entry, so
    // deleting `&& stored` from the guard left the whole suite green — and a package whose first
    // entry is a *deflated* `mimetype`, which is what a naive repacking tool produces, would be
    // claimed and read despite the specification requiring otherwise.
    let deflated = build_zip_deflating_first(&[
        ("mimetype", "application/vnd.oasis.opendocument.text"),
        ("content.xml", "<office:document-content/>"),
    ]);
    assert!(
        !engine_office::is_odt(&deflated),
        "the package specification requires the mimetype entry to be stored, so that a consumer \
         can identify the document without inflating anything"
    );
    assert!(engine_office::read(&deflated).is_err());
}

/// **The requirement is about the archive's leading bytes, not its index.**
///
/// A conforming package whose central directory happens to be written in name order — which any
/// tool that sorts its index produces — has `META-INF/manifest.xml` listed before `mimetype`. A
/// check that read directory order would decline a genuine `.odt`, and through the CLI it would
/// fall past the office branch to the PDF reader and be refused with a message about a PDF header.
#[test]
fn a_conforming_package_with_a_name_ordered_directory_still_reads() {
    let archive = build_zip_with_sorted_directory(&[
        ("mimetype", "application/vnd.oasis.opendocument.text"),
        (
            "META-INF/manifest.xml",
            r#"<manifest:manifest xmlns:manifest="m">
                 <manifest:file-entry manifest:full-path="content.xml"/>
               </manifest:manifest>"#,
        ),
        (
            "content.xml",
            concat!(
                r#"<office:document-content"#,
                r#" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0""#,
                r#" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">"#,
                r#"<office:body><office:text><text:p>Still a text document.</text:p>"#,
                r#"</office:text></office:body></office:document-content>"#
            ),
        ),
    ]);

    assert!(
        engine_office::is_odt(&archive),
        "mimetype's local header is at offset 0, which is what the specification requires"
    );
    let sealed = engine_office::read(&archive).expect("a conforming package reads");
    assert_eq!(sealed.payload().nodes[0].text, "Still a text document.");
}

/// A package that lists one name twice is refused, because `read_entry` takes the first and a
/// consumer preferring the last would see a different document — with nothing declared either way.
#[test]
fn a_package_that_names_one_entry_twice_is_refused() {
    let archive = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            (
                "META-INF/manifest.xml",
                r#"<manifest:manifest xmlns:manifest="m">
                     <manifest:file-entry manifest:full-path="content.xml"/>
                   </manifest:manifest>"#,
            ),
            ("content.xml", "<office:document-content/>"),
            ("content.xml", "<office:document-content/>"),
        ],
    );
    let error = engine_office::read(&archive).expect_err("duplicate names are refused");
    assert!(error.to_string().contains("more than once"), "{error}");
}

/// …and a manifest that declares one part twice, which would otherwise let an unencrypted
/// declaration written first hide an encrypted one written second.
#[test]
fn a_manifest_that_declares_the_content_part_twice_is_refused() {
    let archive = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            (
                "META-INF/manifest.xml",
                r#"<manifest:manifest xmlns:manifest="m">
                     <manifest:file-entry manifest:full-path="content.xml"/>
                     <manifest:file-entry manifest:full-path="content.xml">
                       <manifest:encryption-data manifest:checksum="x"/>
                     </manifest:file-entry>
                   </manifest:manifest>"#,
            ),
            ("content.xml", "not xml, because it is ciphertext"),
        ],
    );
    let error = engine_office::read(&archive).expect_err("a contradictory manifest is refused");
    assert!(
        error.to_string().contains("more than once"),
        "and it is refused for the contradiction rather than for the ciphertext: {error}"
    );
}

/// The manifest is consulted, and both ways it can refuse are named rather than guessed at.
#[test]
fn a_package_whose_manifest_hides_its_content_is_refused_by_name() {
    let undeclared = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            (
                "META-INF/manifest.xml",
                r#"<manifest:manifest xmlns:manifest="m">
                     <manifest:file-entry manifest:full-path="styles.xml"/>
                   </manifest:manifest>"#,
            ),
            ("content.xml", "<office:document-content/>"),
        ],
    );
    let error = engine_office::read(&undeclared).expect_err("an undeclared content part");
    assert!(
        error.to_string().contains("META-INF/manifest.xml"),
        "the refusal names the part that failed to declare it: {error}"
    );

    let encrypted = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            (
                "META-INF/manifest.xml",
                r#"<manifest:manifest xmlns:manifest="m">
                     <manifest:file-entry manifest:full-path="content.xml">
                       <manifest:encryption-data manifest:checksum="x"/>
                     </manifest:file-entry>
                   </manifest:manifest>"#,
            ),
            ("content.xml", "not xml, because it is ciphertext"),
        ],
    );
    let error = engine_office::read(&encrypted).expect_err("an encrypted content part");
    assert!(
        error.to_string().contains("encrypted"),
        "and names the reason rather than blaming the XML: {error}"
    );

    let no_manifest = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[("content.xml", "<office:document-content/>")],
    );
    let error = engine_office::read(&no_manifest).expect_err("a package with no manifest");
    assert!(
        error.to_string().contains("META-INF/manifest.xml"),
        "{error}"
    );
}

/// A package claiming to be more than one format is a named refusal, not a coin toss.
#[test]
fn a_package_that_is_two_formats_is_refused_by_name() {
    let archive = build_odt(
        "application/vnd.oasis.opendocument.text",
        &[
            ("META-INF/manifest.xml", "<manifest:manifest/>"),
            ("content.xml", "<office:document-content/>"),
            ("word/document.xml", "<w:document/>"),
        ],
    );
    assert!(engine_office::is_odt(&archive));
    assert!(engine_office::is_docx(&archive));

    let error = engine_office::read(&archive).expect_err("ambiguous packages are refused");
    let text = error.to_string();
    assert!(text.contains("word/document.xml"), "{text}");
    assert!(text.contains("opendocument.text"), "{text}");
}

/// A ZIP with none of the four formats' evidence is refused, naming all four.
#[test]
fn a_zip_that_is_no_office_format_names_every_one_it_is_not() {
    let renamed = build_zip(&[("content.xml", "<office:document-content/>")]);
    assert!(!engine_office::is_odt(&renamed));

    let error = engine_office::read(&renamed).expect_err("a ZIP that is no office format");
    let text = error.to_string();
    for evidence in [
        "word/document.xml",
        "xl/workbook.xml",
        "ppt/presentation.xml",
        "mimetype",
    ] {
        assert!(
            text.contains(evidence),
            "the refusal names {evidence}: {text}"
        );
    }
}

#[test]
fn a_truncated_document_is_refused() {
    let full = fixture("text-paragraphs");
    for cut in [8, full.len() / 2, full.len() - 4] {
        let error = engine_office::read(&full[..cut]).expect_err("a truncated archive is refused");
        assert!(!error.to_string().is_empty());
    }
}

// -------------------------------------------------------------------------------------------
// Helpers — a minimal stored-entry ZIP, so a negative case needs no fixture file
// -------------------------------------------------------------------------------------------

/// An ODF-shaped package: `mimetype` first, then the entries given.
fn build_odt(media_type: &str, rest: &[(&str, &str)]) -> Vec<u8> {
    let mut entries = vec![("mimetype", media_type)];
    entries.extend_from_slice(rest);
    build_zip(&entries)
}

/// The same archive, but the first entry is **deflated** rather than stored.
///
/// Written by hand because the whole point is a package the specification forbids: `zipfile` and
/// every other writer would happily produce it, and until this existed the `stored` half of ODF
/// detection had no test in the repository.
fn build_zip_deflating_first(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for (index, (name, body)) in entries.iter().enumerate() {
        let offset = out.len() as u32;
        let raw = body.as_bytes();
        let crc = crc32(raw);
        let (method, data) = if index == 0 {
            (8u16, deflate_stored_block(raw))
        } else {
            (0u16, raw.to_vec())
        };

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[10, 0, 0, 0]);
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&data);

        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&[10, 0, 10, 0, 0, 0]);
        directory.extend_from_slice(&method.to_le_bytes());
        directory.extend_from_slice(&[0, 0, 0, 0]);
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }

    finish_zip(out, directory, entries.len())
}

/// The same archive, with the central directory written in **name order** rather than write order.
///
/// A conforming ODF package whose index is sorted — which is what the physical-layout requirement
/// exists to be independent of.
fn build_zip_with_sorted_directory(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut records: Vec<(&str, Vec<u8>)> = Vec::new();

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

        let mut record = Vec::new();
        record.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        record.extend_from_slice(&[10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        record.extend_from_slice(&crc.to_le_bytes());
        record.extend_from_slice(&(data.len() as u32).to_le_bytes());
        record.extend_from_slice(&(data.len() as u32).to_le_bytes());
        record.extend_from_slice(&(name.len() as u16).to_le_bytes());
        record.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        record.extend_from_slice(&offset.to_le_bytes());
        record.extend_from_slice(name.as_bytes());
        records.push((name, record));
    }

    records.sort_by_key(|(name, _)| *name);
    let directory: Vec<u8> = records.into_iter().flat_map(|(_, r)| r).collect();
    finish_zip(out, directory, entries.len())
}

/// A single deflate **stored block** — a valid deflate stream that happens not to compress.
///
/// Enough to make the entry's method 8, which is the property under test; `flate2` inflates it
/// back to the same bytes.
fn deflate_stored_block(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, chunk) in raw.chunks(0xFFFF).enumerate() {
        let last = (index + 1) * 0xFFFF >= raw.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(chunk.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(chunk.len() as u16)).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    if raw.is_empty() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    out
}

fn finish_zip(mut out: Vec<u8>, directory: Vec<u8>, count: usize) -> Vec<u8> {
    let directory_offset = out.len() as u32;
    let directory_size = directory.len() as u32;
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(count as u16).to_le_bytes());
    out.extend_from_slice(&(count as u16).to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

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
