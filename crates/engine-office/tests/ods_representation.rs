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

//! v2-S6's coverage, executable: **an ODS cell binds at the position the file states.**
//!
//! The fifth sibling of `docx_representation.rs`. Most of its assertions are the four before it's.
//! The ones that are new are all about one fact: **OpenDocument writes no cell address**, so the
//! tests that carry weight here are the ones about `table:number-columns-repeated` — the only thing
//! in the file that says where a cell is.
//!
//! **Every assertion about the fixture's contents inflates the package's own `content.xml`.**
//! v2-S5's review found a guard that passed because it grepped `make_fixtures.py` and matched a `#`
//! comment; a test that asserts the bytes under test says something must read those bytes.

use std::collections::BTreeSet;
use std::path::PathBuf;

use engine_core::{
    GeometryAbsence, GeometryPresence, NativeLocator, NodeAttributes, OdfCellTextSource,
    OdfValueType, OdsLocator, Profile,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("workbook.ods");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The package's own `content.xml`, inflated. **Never the generator's source.**
fn content_of(bytes: &[u8]) -> String {
    let part = engine_office::zip::read_entry(bytes, "content.xml").expect("the part inflates");
    String::from_utf8(part).expect("the part is UTF-8")
}

fn locators(sealed: &engine_core::DocumentRepresentation) -> Vec<OdsLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Ods(l) => l.clone(),
            other => panic!("an ODF cell must carry a spreadsheet address, not {other:?}"),
        })
        .collect()
}

fn text_at(sealed: &engine_core::DocumentRepresentation, row: u32, column: u32) -> Option<String> {
    sealed
        .payload()
        .nodes
        .iter()
        .find(|n| {
            matches!(&n.native_locator, NativeLocator::Ods(l) if l.row == row && l.column == column)
        })
        .map(|n| n.text.clone())
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The slice's sentence, executable.** A known cell is on a node, addressed by the file.
#[test]
fn a_cell_resolves_to_a_node_addressed_by_the_document_itself() {
    let sealed = engine_office::read(&fixture("sheet-cells")).expect("the fixture reads");

    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Evidence, not extraction.")
        .expect("the phrase is a cell in the document, so it is in the artifact");

    match (&node.native_locator, &node.attributes) {
        (NativeLocator::Ods(l), NodeAttributes::OfficeOdfCell(a)) => {
            assert_eq!(l.part, "content.xml");
            assert_eq!(l.table, "Rows & Columns");
            assert_eq!((l.row, l.column), (1, 1));
            assert_eq!(a.value_type, OdfValueType::String);
            assert_eq!(a.text_source, OdfCellTextSource::StoredText);
        }
        other => panic!("expected an ODF cell, got {other:?}"),
    }
}

/// **The repeat is the address, and the fixture proves the reader honours it.**
///
/// `Total` is the fifth cell of its row because the file states three cells between it and the
/// first. The assertion inflates the package to show the attribute is really there, so a reader
/// that ignored it would fail here rather than quietly disagree with every producer.
#[test]
fn a_repeated_run_of_cells_puts_the_next_one_where_the_file_says() {
    let bytes = fixture("sheet-cells");
    let content = content_of(&bytes);
    assert!(
        content.contains(r#"table:number-columns-repeated="3""#),
        "the fixture must really compress a run of three, or the assertion below is vacuous: \
         {content}"
    );

    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 1, 5).as_deref(),
        Some("Total"),
        "three repeated columns sit at 2, 3 and 4, so the next cell is 5"
    );
    assert!(
        text_at(&sealed, 1, 3).is_none(),
        "and the repeated run is empty, so it is no node"
    );
}

/// **The page is in the file, and stays out of the artifact.**
///
/// A `<text:soft-page-break/>` inside a cell's own paragraph, and an `fo:page-width` in the second
/// package's `styles.xml`. Both are asserted present in the bytes, so `pages: []` below is a
/// refusal rather than an absence.
#[test]
fn the_print_break_in_the_file_is_read_and_is_still_not_a_page() {
    let bytes = fixture("sheet-cells");
    let content = content_of(&bytes);
    assert!(
        content.contains("<text:soft-page-break/>"),
        "the fixture really does carry the producing application's own break: {content}"
    );

    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    let payload = sealed.payload();
    assert!(payload.pages.is_empty(), "a print break is not a page");
    assert!(!payload.nodes.is_empty(), "and that is not vacuous");

    assert_eq!(
        text_at(&sealed, 3, 1).as_deref(),
        Some("Split by the producer and rejoined here."),
        "the break contributes no character either — it is a layout mark, not text"
    );

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
}

/// The locator's field set is exactly four, and it refuses a print page **by name**.
#[test]
fn the_locator_carries_four_fields_and_refuses_a_page_by_name() {
    let json = r#"{"part":"content.xml","table":"S","row":1,"column":1}"#;
    let parsed: OdsLocator = serde_json::from_str(json).expect("the four fields are the shape");
    assert_eq!(parsed.part, "content.xml");
    assert_eq!((parsed.row, parsed.column), (1, 1));

    for forbidden in [
        r#"{"part":"content.xml","table":"S","row":1,"column":1,"page":1}"#,
        r#"{"part":"content.xml","table":"S","row":1,"column":1,"bbox":[0,0,1,1]}"#,
        r#"{"part":"content.xml","table":"S","row":1,"column":1,"x":0}"#,
        r#"{"part":"content.xml","table":"S","row":1,"column":1,"print_page":1}"#,
        r#"{"part":"content.xml","table":"S","row":1,"column":1,"soft_page_break":1}"#,
    ] {
        assert!(
            serde_json::from_str::<OdsLocator>(forbidden).is_err(),
            "`deny_unknown_fields` must refuse {forbidden}"
        );
    }
}

/// **The column is a number, because the file contains no letters.**
///
/// The one place this locator deliberately differs from `XlsxLocator`, whose column is a `String`
/// because `<c r="B12">` writes `B` down. Asserted so a later slice cannot "improve" it into a
/// spreadsheet application's display convention.
#[test]
fn the_column_is_a_number_and_the_file_is_why() {
    let bytes = fixture("sheet-cells");
    let content = content_of(&bytes);
    for letters in [r#"r="A1""#, r#"r="B2""#, "column-letter"] {
        assert!(
            !content.contains(letters),
            "a `.ods` contains no lettered address, so nothing may read one: found {letters}"
        );
    }
    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    assert!(locators(&sealed).iter().all(|l| l.column >= 1));
}

// -------------------------------------------------------------------------------------------
// The text, and what never reaches it
// -------------------------------------------------------------------------------------------

/// ODF's stated characters, an entity, and two paragraphs in one cell.
#[test]
fn a_cells_text_is_what_the_file_states() {
    let sealed = engine_office::read(&fixture("sheet-cells")).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 2, 3).as_deref(),
        Some("Rows & columns\tare tabbed."),
        "`&amp;` survives and `<text:tab/>` is the tab the file states"
    );
    assert_eq!(
        text_at(&sealed, 2, 4).as_deref(),
        Some("Three spaces:   stated, not measured."),
        "`<text:s text:c=\"3\"/>` is three spaces — the count the file wrote"
    );
    assert_eq!(
        text_at(&sealed, 3, 3).as_deref(),
        Some("First line\nSecond line"),
        "two paragraphs in one cell are two displayed lines of ONE node"
    );
}

/// The ampersand is in the table **name**, where dropping it is a wrong address.
#[test]
fn an_entity_in_the_table_name_is_resolved_because_it_is_an_address() {
    let bytes = fixture("sheet-cells");
    assert!(
        content_of(&bytes).contains(r#"table:name="Rows &amp; Columns""#),
        "the fixture must really write the entity into the name"
    );
    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    assert!(locators(&sealed)
        .iter()
        .all(|l| l.table == "Rows & Columns"));
}

/// A formula cell's text is the cached display form, labelled as such.
#[test]
fn a_formula_cell_is_labelled_rather_than_evaluated() {
    let bytes = fixture("sheet-cells");
    assert!(
        content_of(&bytes).contains("table:formula="),
        "the fixture must really carry a formula"
    );
    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| matches!(&n.native_locator, NativeLocator::Ods(l) if (l.row, l.column) == (2, 5)))
        .expect("the formula cell is a node");
    match &node.attributes {
        NodeAttributes::OfficeOdfCell(a) => {
            assert_eq!(a.text_source, OdfCellTextSource::CachedFormulaText);
            assert_eq!(a.value_type, OdfValueType::Float);
        }
        other => panic!("expected an ODF cell, got {other:?}"),
    }
    assert_eq!(
        node.text, "42",
        "the cached value, never the formula source"
    );
    assert!(
        !sealed
            .payload()
            .nodes
            .iter()
            .any(|n| n.text.contains("SUM")),
        "no formula source reaches any node's text"
    );
}

/// **Every leak v2-S5's review found, absent from a cell and present in the count.**
///
/// The load-bearing honesty test. Each string is asserted present in the package's own
/// `content.xml` and absent from every node — so this fails both if the reader starts splicing and
/// if the fixture stops containing the construct.
#[test]
fn non_displayed_character_data_is_absent_from_every_cell_and_is_declared() {
    let bytes = fixture("sheet-unread-parts");
    let content = content_of(&bytes);
    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    let all: String = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join("\u{1f}");

    // `in_file` is what the package must really contain; `in_text` is what must never reach a
    // node. They differ where the construct is an element whose *character data* is the leak — a
    // bare `2.1` or `17` would be found in prose, so the element is what proves the fixture and the
    // full node text is what proves the reader.
    for (what, in_file, in_text) in [
        (
            "an image's title",
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
            "a page-anchored shape",
            "FLOATING-SHAPE-TEXT",
            "FLOATING-SHAPE-TEXT",
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
            "{what}: `{in_text}` reached a cell's text — all of it was `{all}`"
        );
    }

    // And the cells' own text is not lost with it.
    assert_eq!(text_at(&sealed, 1, 1).as_deref(), Some("Kept sentence."));
    assert_eq!(text_at(&sealed, 2, 1).as_deref(), Some("Leaks:"));
    assert_eq!(text_at(&sealed, 2, 5).as_deref(), Some("kanji"));
    assert_eq!(text_at(&sealed, 3, 1).as_deref(), Some("Namespaces."));
}

/// **The frame-alternative rule, measured — what v2-S5 owed this slice.**
///
/// v2-S5 wrote first-rendition-wins off the specification and recorded that it had never been
/// measured against a document that nests two renditions, because it had none. This package nests
/// two, in a conforming `.ods`: ODF puts `<draw:frame>` in the `<text:p>` content model for every
/// document type.
///
/// **What the rule looks like here is not what it looks like in an ODT, and that is the result.**
/// An ODT's atom is the paragraph, so a frame's first rendition becomes nodes. A spreadsheet's atom
/// is the **cell**, and a frame floats over the sheet — its words belong to no cell, so there is no
/// address at which "one displayed phrase becomes one node" could be true. Merging them into the
/// anchoring cell was this reader's first behaviour, and it put a phrase at a cell address a person
/// reading the document does not find there.
///
/// So the half that survives the atom change is what this asserts: **the two renditions are
/// accounted separately**, and the cell's own text is untouched by either.
#[test]
fn a_framed_cell_keeps_its_own_text_and_declares_both_renditions() {
    let bytes = fixture("sheet-unread-parts");
    let content = content_of(&bytes);
    assert_eq!(
        content.matches("<draw:text-box>").count(),
        3,
        "two renditions in one frame, plus the page-anchored shape's one: {content}"
    );

    let sealed = engine_office::read(&bytes).expect("the fixture reads");
    let cell = text_at(&sealed, 1, 2).expect("the framed cell is a node");
    assert_eq!(
        cell, "Framed:",
        "the cell keeps its own paragraph and neither rendition joins it"
    );
    let all: String = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join("\u{1f}");
    assert!(!all.contains("FIRST-RENDITION"), "{all}");
    assert!(!all.contains("SECOND-RENDITION"), "{all}");
}

/// Deleting the construct from the **bytes** moves the count — so the count is not decoration.
///
/// Mutation-checked rather than asserted: the erasure count is rebuilt from a package with the
/// second rendition removed, and it must fall.
#[test]
fn removing_a_rendition_from_the_bytes_moves_the_declared_count() {
    let bytes = fixture("sheet-unread-parts");
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
    assert!(
        after < before,
        "removing the second rendition must lower the declared region count: {before} -> {after}"
    );
}

/// The A14 declaration names all three kinds, and the clean package declares none of them.
#[test]
fn the_clean_package_declares_no_erasure_and_the_other_declares_three_kinds() {
    let clean = engine_office::read(&fixture("sheet-cells")).expect("reads");
    assert!(
        !clean
            .payload()
            .assurance
            .limitations
            .iter()
            .any(|l| l.code == engine_core::assurance::codes::OFFICE_PARTS_NOT_READ),
        "the clean package consumes every entry it contains, so it erases nothing"
    );

    let dirty = engine_office::read(&fixture("sheet-unread-parts")).expect("reads");
    let detail = dirty
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .map(|l| l.detail.clone())
        .expect("the second package erases three kinds of thing");
    assert!(detail.contains("entry(ies)"), "unread entries: {detail}");
    assert!(detail.contains("region(s)"), "unread regions: {detail}");
    assert!(detail.contains("block(s)"), "foreign text: {detail}");
}

// -------------------------------------------------------------------------------------------
// Identity
// -------------------------------------------------------------------------------------------

/// The spreadsheet profile is its own, and all six are mutually distinct.
#[test]
fn the_ods_profile_is_its_own_and_all_six_are_distinct() {
    let hashes: BTreeSet<String> = [
        Profile::default(),
        Profile::docx_v0(),
        Profile::xlsx_v0(),
        Profile::pptx_v0(),
        Profile::odt_v0(),
        Profile::ods_v0(),
    ]
    .iter()
    .map(|p| p.profile_sha256().expect("hashes").to_string())
    .collect();
    assert_eq!(hashes.len(), 6, "six readers, six profiles, six hashes");

    let profile = Profile::ods_v0();
    assert!(
        !profile.capabilities.tables,
        "a spreadsheet's cells are addresses the file states, not a grid a detector inferred"
    );
    assert!(!profile.capabilities.measured_ink_boxes);
    assert!(!profile.capabilities.structural_locators);
}

/// Two runs over one document produce identical bytes.
#[test]
fn two_runs_over_one_spreadsheet_produce_identical_bytes() {
    for name in ["sheet-cells", "sheet-unread-parts"] {
        let bytes = fixture(name);
        let a = engine_office::read(&bytes).expect("reads");
        let b = engine_office::read(&bytes).expect("reads again");
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "{name} must be byte-identical across runs"
        );
    }
}

/// Detection reads the bytes: an `.ods` is claimed here and by nothing else.
#[test]
fn detection_reads_the_bytes() {
    let bytes = fixture("sheet-cells");
    assert!(engine_office::is_ods(&bytes));
    assert!(engine_office::is_opendocument(&bytes));
    assert!(
        !engine_office::is_odt(&bytes),
        "a spreadsheet is not a text document"
    );
    assert!(!engine_office::is_docx(&bytes));
    assert!(
        !engine_office::is_xlsx(&bytes),
        "and it is not a workbook either"
    );
    assert!(!engine_office::is_pptx(&bytes));

    // And the reverse, so the two ODF readers do not overlap.
    let odt = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office/text-paragraphs/document.odt");
    let odt = std::fs::read(odt).expect("the ODT fixture is there");
    assert!(!engine_office::is_ods(&odt));
    assert!(engine_office::is_opendocument(&odt));
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

fn regions_declared(bytes: &[u8]) -> usize {
    let sealed = engine_office::read(bytes).expect("reads");
    let detail = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::assurance::codes::OFFICE_PARTS_NOT_READ)
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
/// has to honour — first entry, uncompressed — is three lines.
fn repack(original: &[u8], content: &str) -> Vec<u8> {
    let media = String::from_utf8(
        engine_office::zip::read_entry(original, "mimetype").expect("the type entry is there"),
    )
    .expect("utf-8");
    let manifest = String::from_utf8(
        engine_office::zip::read_entry(original, "META-INF/manifest.xml").expect("manifest"),
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
