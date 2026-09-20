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

//! v2-S7's coverage, executable: **a draw page is not a page.**
//!
//! The sixth sibling of `docx_representation.rs`. Most of its assertions are the five before it's.
//! The ones that carry weight here are all about one refusal: this is the first format from which
//! a `PageRecord` could have been minted with **no arithmetic at all** — `<draw:page>` elements are
//! discrete and ordered, and a `<style:master-page>` states `fo:page-width` beside them — and the
//! artifact still says `pages: []`.
//!
//! **Every assertion about the fixture's contents inflates the package's own `content.xml`.**
//! v2-S5's review found a guard that passed because it grepped `make_fixtures.py` and matched a `#`
//! comment; a test that asserts the bytes under test must read those bytes.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ethos_parser_core::{
    GeometryAbsence, GeometryPresence, NativeLocator, NodeAttributes, OdfBlockKind, OdpLocator,
    Profile,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("presentation.odp");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The package's own `content.xml`, inflated. **Never the generator's source.**
fn content_of(bytes: &[u8]) -> String {
    let part =
        ethos_parser_office::zip::read_entry(bytes, "content.xml").expect("the part inflates");
    String::from_utf8(part).expect("the part is UTF-8")
}

fn locators(sealed: &ethos_parser_core::DocumentRepresentation) -> Vec<OdpLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Odp(l) => l.clone(),
            other => panic!("a presentation block must carry an ODP address, not {other:?}"),
        })
        .collect()
}

fn text_at(
    sealed: &ethos_parser_core::DocumentRepresentation,
    draw_page: u32,
    shape: u32,
    paragraph: u32,
) -> Option<String> {
    sealed
        .payload()
        .nodes
        .iter()
        .find(|n| {
            matches!(&n.native_locator, NativeLocator::Odp(l)
                if (l.draw_page, l.shape, l.paragraph) == (draw_page, shape, paragraph))
        })
        .map(|n| n.text.clone())
}

fn all_text(sealed: &ethos_parser_core::DocumentRepresentation) -> String {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The slice's sentence, executable.** A known title is on a node, addressed by the file.
#[test]
fn a_title_resolves_to_a_node_addressed_by_the_document_itself() {
    let sealed =
        ethos_parser_office::read(&fixture("presentation-pages")).expect("the fixture reads");

    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Evidence, not extraction.")
        .expect("the phrase is a title in the document, so it is in the artifact");

    match (&node.native_locator, &node.attributes) {
        (NativeLocator::Odp(l), NodeAttributes::OfficeOdfShape(a)) => {
            assert_eq!(l.part, "content.xml");
            assert_eq!((l.draw_page, l.shape, l.paragraph), (1, 1, 1));
            assert_eq!(
                a.block,
                OdfBlockKind::Heading,
                "the fixture writes `<text:h>`"
            );
            assert_eq!(a.draw_page_name.as_deref(), Some("Rows & Columns"));
            assert_eq!(a.shape_name.as_deref(), Some("Title 1"));
            assert_eq!(
                a.outline_level, None,
                "the fixture writes a bare `<text:h>`: it is a heading, and it stated no level. \
                 `Some(1)` here would be a level resolved out of the master page this reader \
                 declares it did not open"
            );
        }
        other => panic!("expected a presentation shape, got {other:?}"),
    }
}

/// **The page was free here, and `pages` is still empty.**
///
/// The load-bearing refusal of this slice. The fixture is asserted to contain **two**
/// `<draw:page>` elements and the second package's `styles.xml` to contain an `fo:page-width`, so
/// a `PageRecord` needed no arithmetic — and there is none. Neither assertion is vacuous: the
/// bytes are inflated first.
#[test]
fn a_draw_page_is_listed_in_the_file_and_is_still_not_a_page() {
    let bytes = fixture("presentation-pages");
    let content = content_of(&bytes);
    assert_eq!(
        content.matches("<draw:page ").count(),
        2,
        "the fixture must really list two draw pages, or `pages: []` below asserts nothing: \
         {content}"
    );

    let styles = String::from_utf8(
        ethos_parser_office::zip::read_entry(&fixture("presentation-unread-parts"), "styles.xml")
            .expect("the styles part inflates"),
    )
    .expect("utf-8");
    assert!(
        styles.contains("fo:page-width=") && styles.contains("<style:master-page"),
        "and a master page must really state paper beside them: {styles}"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let payload = sealed.payload();
    assert!(
        payload.pages.is_empty(),
        "a draw page is a part of the presentation's structure, not a page this engine measured"
    );
    assert!(!payload.nodes.is_empty(), "and that is not vacuous");
    assert!(
        locators(&sealed).iter().any(|l| l.draw_page == 2),
        "the second draw page really did produce nodes"
    );

    for (index, node) in payload.nodes.iter().enumerate() {
        assert!(
            !node.native_locator.is_paginated(),
            "a presentation address is page-less: {:?}",
            node.native_locator
        );
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
}

/// The locator's field set is exactly four, and it refuses a slide number **by name**.
#[test]
fn the_locator_carries_four_fields_and_refuses_a_page_by_name() {
    let json = r#"{"part":"content.xml","draw_page":1,"shape":1,"paragraph":1}"#;
    let parsed: OdpLocator = serde_json::from_str(json).expect("the four fields are the shape");
    assert_eq!(parsed.part, "content.xml");
    assert_eq!(
        (parsed.draw_page, parsed.shape, parsed.paragraph),
        (1, 1, 1)
    );

    for forbidden in [
        r#"{"part":"content.xml","draw_page":1,"shape":1,"paragraph":1,"page":1}"#,
        r#"{"part":"content.xml","draw_page":1,"shape":1,"paragraph":1,"bbox":[0,0,1,1]}"#,
        r#"{"part":"content.xml","draw_page":1,"shape":1,"paragraph":1,"x":0}"#,
        r#"{"part":"content.xml","draw_page":1,"shape":1,"paragraph":1,"slide_number":1}"#,
        r#"{"part":"content.xml","draw_page":1,"shape":1,"paragraph":1,"soft_page_break":1}"#,
    ] {
        assert!(
            serde_json::from_str::<OdpLocator>(forbidden).is_err(),
            "`deny_unknown_fields` must refuse {forbidden}"
        );
    }
}

/// **The names are labels, and they are not in the address.**
///
/// v2-S4 measured that a shape id present on every shape is unique only most of the time, and moved
/// it off the address. No corpus of real `.odp` files was available to measure `draw:name`, so it
/// gets the same treatment rather than the benefit of the doubt — asserted here so a later slice
/// cannot quietly promote it.
#[test]
fn the_draw_names_are_attributes_and_never_the_address() {
    let bytes = fixture("presentation-pages");
    assert!(
        content_of(&bytes).contains(r#"draw:name="Title 1""#),
        "the fixture must really name its shapes"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let serialized = serde_json::to_string(&sealed).expect("serializes");
    let locator_shaped = [r#""draw_page_name""#, r#""shape_name""#];
    for field in locator_shaped {
        assert!(
            serialized.contains(field),
            "{field} is carried on the attributes"
        );
    }
    // The address holds only positions, which is what makes the two separable at all.
    for l in locators(&sealed) {
        assert!(l.draw_page >= 1 && l.shape >= 1 && l.paragraph >= 1);
    }
}

/// **The serialization does not move an address.** `<draw:frame/>` is a shape the page contains.
#[test]
fn a_self_closing_shape_still_holds_its_position() {
    let bytes = fixture("presentation-pages");
    assert!(
        content_of(&bytes).contains("<draw:frame/>"),
        "the fixture must really serialize an empty shape the short way"
    );
    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 2, 2, 1).as_deref(),
        Some("The second draw page"),
        "the self-closing frame is shape 1, so the title after it is shape 2"
    );
    assert!(
        text_at(&sealed, 2, 1, 1).is_none(),
        "and the empty shape is no node"
    );
}

// -------------------------------------------------------------------------------------------
// The text, and what never reaches it
// -------------------------------------------------------------------------------------------

/// ODF's stated characters, an entity, and a break that contributes none.
#[test]
fn a_blocks_text_is_what_the_file_states() {
    let bytes = fixture("presentation-pages");
    let content = content_of(&bytes);
    assert!(
        content.contains("<text:soft-page-break/>"),
        "the fixture really does carry the producing application's own break: {content}"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 1, 2, 1).as_deref(),
        Some("Rows & columns bind to a shape\tand never to a page."),
        "`&amp;` survives and `<text:tab/>` is the tab the file states"
    );
    assert_eq!(
        text_at(&sealed, 1, 2, 2).as_deref(),
        Some("Three spaces:   stated, not measured."),
        "`<text:s text:c=\"3\"/>` is three spaces — the count the file wrote"
    );
    assert_eq!(
        text_at(&sealed, 1, 2, 3).as_deref(),
        Some("Split by the producer and rejoined here."),
        "the break contributes no character — it is a layout mark, not text"
    );
    assert_eq!(
        text_at(&sealed, 2, 3, 1).as_deref(),
        Some("Drawn shapes carry their blocks directly."),
        "a `<draw:custom-shape>` holds its blocks with no text box between"
    );
}

/// The ampersand survives in visible text **and** in both names the file writes.
///
/// A `&` dropped from text is wrong text; one dropped from a name is a wrong **label**, which is
/// what a person reading a citation matches against the original package.
#[test]
fn an_entity_survives_in_text_and_in_every_name() {
    let bytes = fixture("presentation-pages");
    let content = content_of(&bytes);
    for written in [
        r#"draw:name="Rows &amp; Columns""#,
        r#"draw:name="Body &amp; bullets""#,
        "<text:p>Rows &amp; columns bind to a shape",
    ] {
        assert!(
            content.contains(written),
            "the fixture must really write the entity there: {written}"
        );
    }

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    assert!(
        text_at(&sealed, 1, 2, 1).is_some_and(|t| t.starts_with("Rows & columns")),
        "the entity survives in displayed text as well as in the names"
    );
    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| matches!(&n.native_locator, NativeLocator::Odp(l) if (l.draw_page, l.shape) == (1, 2)))
        .expect("the body shape is a node");
    match &node.attributes {
        NodeAttributes::OfficeOdfShape(a) => {
            assert_eq!(a.draw_page_name.as_deref(), Some("Rows & Columns"));
            assert_eq!(a.shape_name.as_deref(), Some("Body & bullets"));
        }
        other => panic!("expected a presentation shape, got {other:?}"),
    }
}

/// **Every leak v2-S5 and v2-S6 listed, absent from a block and present in the count.**
///
/// The load-bearing honesty test. Each string is asserted present in the package's own
/// `content.xml` and absent from every node — so it fails both if the reader starts splicing and
/// if the fixture stops containing the construct.
#[test]
fn non_displayed_character_data_is_absent_from_every_block_and_is_declared() {
    let bytes = fixture("presentation-unread-parts");
    let content = content_of(&bytes);
    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let all = all_text(&sealed);

    // `in_file` is what the package must really contain; `in_text` is what must never reach a
    // node. They differ where the construct is an element whose *character data* is the leak — a
    // bare `2.1` or `17` would be found in prose, so the element is what proves the fixture and
    // the full node text is what proves the reader.
    for (what, in_file, in_text) in [
        (
            "an image's title, in a shape with no block at all",
            "<svg:title>IMAGE-TITLE</svg:title>",
            "IMAGE-TITLE",
        ),
        (
            "an image's description",
            "<svg:desc>IMAGE-DESC</svg:desc>",
            "IMAGE-DESC",
        ),
        (
            "an embedded object's base64",
            "<office:binary-data>QkFTRTY0LURBVEE=</office:binary-data>",
            "QkFTRTY0LURBVEE=",
        ),
        (
            "a generated heading number",
            "<text:number>2.1</text:number>",
            "2.1Numbered heading label",
        ),
        (
            "a field's cached page count",
            "<text:page-count>17</text:page-count>",
            "Fields:17",
        ),
        (
            "a field's cached page number",
            "<text:page-number>4</text:page-number>",
            "Fields:174",
        ),
        (
            "ruby guide text",
            "<text:ruby-text>RUBY-GUIDE</text:ruby-text>",
            "RUBY-GUIDE",
        ),
        ("a comment's body", "COMMENT-BODY", "COMMENT-BODY"),
        (
            "a second framed rendition",
            "SECOND-RENDITION",
            "SECOND-RENDITION",
        ),
        (
            "a rendition after a self-closing first",
            "AFTER-AN-EMPTY-FIRST",
            "AFTER-AN-EMPTY-FIRST",
        ),
        ("a speaker-notes body", "SPOKEN-ALOUD", "SPOKEN-ALOUD"),
        (
            "a drawing shape this slice does not name",
            "<draw:rect",
            "RECT-TEXT",
        ),
        ("a foreign XHTML paragraph", "XHTML-STOLEN", "XHTML-STOLEN"),
        (
            "MathML's annotation",
            "MATHML-ANNOTATION",
            "MATHML-ANNOTATION",
        ),
    ] {
        assert!(
            content.contains(in_file),
            "{what}: the fixture must really contain `{in_file}`, or the assertion below is vacuous"
        );
        assert!(
            !all.contains(in_text),
            "{what}: `{in_text}` reached a block's text — all of it was `{all}`"
        );
    }

    // And a master page's text is in the package and not in the artifact either — v2-S4's
    // "no placeholder inheritance is resolved", in ODF's spelling.
    let styles = String::from_utf8(
        ethos_parser_office::zip::read_entry(&bytes, "styles.xml").expect("styles inflates"),
    )
    .expect("utf-8");
    assert!(styles.contains("MASTER-PAGE-TEXT"), "{styles}");
    assert!(!all.contains("MASTER-PAGE-TEXT"), "{all}");

    // And the shapes' own text is not lost with it.
    assert_eq!(text_at(&sealed, 1, 1, 1).as_deref(), Some("Kept sentence."));
    assert_eq!(text_at(&sealed, 2, 1, 4).as_deref(), Some("kanji"));
    assert_eq!(text_at(&sealed, 2, 2, 1).as_deref(), Some("Namespaces."));
}

/// **The frame-alternative rule, measured on a presentation — and it is ODT's answer.**
///
/// v2-S5 read first-rendition-wins off the specification. v2-S6 measured it on a spreadsheet and
/// found the **atom** decides: an ODT's atom is the paragraph, so the first rendition becomes
/// nodes; an ODS's atom is the cell, a frame floats over the sheet, and merging its words into the
/// anchoring cell was a mis-attribution.
///
/// A presentation is a drawing, and there is nothing for a frame to float over — the frame **is**
/// the shape this reader addresses. So the first rendition becomes nodes and the second is one
/// declared erasure, which is ODT's outcome reached by neither ODT's nor ODS's argument. The
/// three-format table is in `docs/history/15-V2-MILESTONES.md` S7.
#[test]
fn the_first_rendition_is_the_shape_and_the_second_is_declared() {
    let bytes = fixture("presentation-unread-parts");
    let content = content_of(&bytes);
    assert_eq!(
        content.matches("<draw:text-box>").count(),
        7,
        "one kept, TWO renditions in one frame, the second after a self-closing first, the \
         notes', and two more on the second page: {content}"
    );
    assert!(
        content.contains("<draw:text-box/>"),
        "and a self-closing first rendition, which v2-S5 and v2-S6 both got wrong when the \
         fixture never nested one: {content}"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 1, 2, 1).as_deref(),
        Some("FIRST-RENDITION"),
        "the displayed rendition is the shape's text — the outcome an ODS cell could not have"
    );
    let all = all_text(&sealed);
    assert!(!all.contains("SECOND-RENDITION"), "{all}");
    assert!(
        !all.contains("AFTER-AN-EMPTY-FIRST"),
        "an empty first rendition still claims the slot: {all}"
    );
}

/// Deleting the construct from the **bytes** moves the count — so the count is not decoration.
#[test]
fn removing_a_rendition_from_the_bytes_moves_the_declared_count() {
    let bytes = fixture("presentation-unread-parts");
    let before = regions_declared(&bytes);

    let content = content_of(&bytes);
    let stripped = content.replace(
        "<draw:text-box><text:p>SECOND-RENDITION</text:p></draw:text-box>",
        "",
    );
    assert_ne!(
        stripped, content,
        "the mutation must actually change the part"
    );

    let mutated = repack(&bytes, &stripped);
    let after = regions_declared(&mutated);
    assert_eq!(
        (before, after),
        (4, 3),
        "removing the second rendition must lower the declared region count by exactly one — an \
         exact pair rather than `after < before`, so a mutation that broke the package into \
         declaring nothing could not pass as a success"
    );
    assert_eq!(
        ethos_parser_office::read(&mutated)
            .expect("the mutated package reads")
            .payload()
            .nodes
            .len(),
        9,
        "and the node count is untouched: nothing was promoted by the removal"
    );
}

/// And so does deleting the speaker notes, which is the erasure this format adds.
#[test]
fn removing_the_speaker_notes_from_the_bytes_moves_the_declared_count() {
    let bytes = fixture("presentation-unread-parts");
    let before = regions_declared(&bytes);

    let content = content_of(&bytes);
    let open = content
        .find("<presentation:notes>")
        .expect("the notes are there");
    let close = content
        .find("</presentation:notes>")
        .expect("and they close")
        + "</presentation:notes>".len();
    let mut stripped = content.clone();
    stripped.replace_range(open..close, "");

    let mutated = repack(&bytes, &stripped);
    let after = regions_declared(&mutated);
    assert_eq!(
        (before, after),
        (4, 3),
        "removing the speaker notes must lower the declared region count by exactly one"
    );
    let sealed = ethos_parser_office::read(&mutated).expect("the mutated package reads");
    assert_eq!(
        sealed.payload().nodes.len(),
        9,
        "and the node count is untouched: nothing was promoted into the slide by their removal"
    );
    assert!(!all_text(&sealed).contains("SPOKEN-ALOUD"));
}

/// The A14 declaration names all three kinds, and the clean package declares none of them.
#[test]
fn the_clean_package_declares_no_erasure_and_the_other_declares_three_kinds() {
    let clean = ethos_parser_office::read(&fixture("presentation-pages")).expect("reads");
    assert!(
        !clean
            .payload()
            .assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ),
        "the clean package consumes every entry it contains, so it erases nothing"
    );

    let dirty = ethos_parser_office::read(&fixture("presentation-unread-parts")).expect("reads");
    let detail = dirty
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .map(|l| l.detail.clone())
        .expect("the second package erases three kinds of thing");
    assert!(detail.contains("entry(ies)"), "unread entries: {detail}");
    assert!(detail.contains("region(s)"), "unread regions: {detail}");
    assert!(detail.contains("shape(s)"), "foreign text: {detail}");
    assert!(
        detail.contains("no shape was open"),
        "text with no address: {detail}"
    );
    assert!(
        detail.contains("Speaker notes"),
        "and the one a caller most needs to know is named: {detail}"
    );
}

// -------------------------------------------------------------------------------------------
// Identity
// -------------------------------------------------------------------------------------------

/// The presentation profile is its own, and all seven are mutually distinct.
#[test]
fn the_odp_profile_is_its_own_and_all_seven_are_distinct() {
    let hashes: BTreeSet<String> = [
        Profile::default(),
        Profile::docx_v0(),
        Profile::xlsx_v0(),
        Profile::pptx_v0(),
        Profile::odt_v0(),
        Profile::ods_v0(),
        Profile::odp_v0(),
    ]
    .iter()
    .map(|p| p.profile_sha256().expect("hashes").to_string())
    .collect();
    assert_eq!(
        hashes.len(),
        7,
        "seven readers, seven profiles, seven hashes"
    );

    let profile = Profile::odp_v0();
    assert!(
        !profile.capabilities.tables,
        "this reader emits no TableRecord, and a true capability would say a detector ran"
    );
    assert!(!profile.capabilities.measured_ink_boxes);
    assert!(!profile.capabilities.structural_locators);
}

/// Two runs over one document produce identical bytes.
#[test]
fn two_runs_over_one_presentation_produce_identical_bytes() {
    for name in ["presentation-pages", "presentation-unread-parts"] {
        let bytes = fixture(name);
        let a = ethos_parser_office::read(&bytes).expect("reads");
        let b = ethos_parser_office::read(&bytes).expect("reads again");
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "{name} must be byte-identical across runs"
        );
    }
}

/// Detection reads the bytes: an `.odp` is claimed here and by nothing else.
#[test]
fn detection_reads_the_bytes() {
    let bytes = fixture("presentation-pages");
    assert!(ethos_parser_office::is_odp(&bytes));
    assert!(ethos_parser_office::is_opendocument(&bytes));
    assert!(
        !ethos_parser_office::is_odt(&bytes),
        "a presentation is not a text document"
    );
    assert!(
        !ethos_parser_office::is_ods(&bytes),
        "and it is not a spreadsheet either"
    );
    assert!(!ethos_parser_office::is_docx(&bytes));
    assert!(!ethos_parser_office::is_xlsx(&bytes));
    assert!(
        !ethos_parser_office::is_pptx(&bytes),
        "and an OOXML deck is a different format with a different reader"
    );

    // And the reverse, so the three ODF readers do not overlap.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf();
    for (path, kind) in [
        ("fixtures/office/text-paragraphs/document.odt", "odt"),
        ("fixtures/office/sheet-cells/workbook.ods", "ods"),
    ] {
        let sibling = std::fs::read(root.join(path)).expect("the sibling fixture is there");
        assert!(
            !ethos_parser_office::is_odp(&sibling),
            "an {kind} is not a presentation"
        );
        assert!(ethos_parser_office::is_opendocument(&sibling));
    }
}

/// **Exact, not prefixed.** A template and a drawing share the vocabulary and are not claimed.
///
/// `…presentation-template` would pass a `starts_with`, and an `.odg`'s `content.xml` really is
/// `<draw:page>` — so a prefix match would produce a *plausible* artifact for a format nobody
/// decided to support, which is the worse of the two failures.
#[test]
fn a_template_and_a_drawing_are_not_claimed_as_a_presentation() {
    for (declared, kind) in [
        (
            "application/vnd.oasis.opendocument.presentation-template",
            "otp",
        ),
        ("application/vnd.oasis.opendocument.graphics", "odg"),
        ("application/epub+zip", "epub"),
    ] {
        let archive = build_zip(&[
            ("mimetype", declared),
            ("META-INF/manifest.xml", "<manifest:manifest/>"),
            ("content.xml", "<office:document-content/>"),
        ]);
        assert!(
            !ethos_parser_office::is_odp(&archive),
            "an {kind} is not a presentation"
        );

        let error = ethos_parser_office::read(&archive).expect_err("and none of them reads");
        let text = error.to_string();
        assert!(
            !text.contains("%PDF-"),
            "and none is refused as a missing PDF header: {text}"
        );
        if kind == "epub" {
            assert!(
                !ethos_parser_office::is_opendocument(&archive),
                "an EPUB uses OCF's first-and-stored `mimetype` too, and is NOT OpenDocument"
            );
        } else {
            assert!(
                text.contains("OpenDocument") && text.contains(declared),
                "an unimplemented ODF type is refused by name: {text}"
            );
        }
    }
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

fn regions_declared(bytes: &[u8]) -> usize {
    let sealed = ethos_parser_office::read(bytes).expect("reads");
    let detail = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .map(|l| l.detail.clone())
        .unwrap_or_default();
    detail
        .split_once(" region(s)")
        .and_then(|(head, _)| head.rsplit(' ').next().map(str::to_string))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Rebuild the package with a replaced `content.xml`, keeping `mimetype` first and **stored**.
///
/// A minimal writer rather than a dependency: `deny.toml` bans a ZIP crate, and the ODF rule this
/// has to honour — first entry, uncompressed — is three lines. The manifest is carried over
/// verbatim so the mutated package is still one the reader will consent to read.
fn repack(original: &[u8], content: &str) -> Vec<u8> {
    let media = String::from_utf8(
        ethos_parser_office::zip::read_entry(original, "mimetype")
            .expect("the type entry is there"),
    )
    .expect("utf-8");
    let manifest = String::from_utf8(
        ethos_parser_office::zip::read_entry(original, "META-INF/manifest.xml").expect("manifest"),
    )
    .expect("utf-8");
    build_zip(&[
        ("mimetype", media.as_str()),
        ("META-INF/manifest.xml", manifest.as_str()),
        ("content.xml", content),
    ])
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
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
