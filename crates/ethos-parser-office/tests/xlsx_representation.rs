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

//! v2-S3's gate, executable: **an XLSX cell binds**.
//!
//! The sibling of `docx_representation.rs`, and the tests it does not have are the interesting
//! ones. A workbook is the first artifact this engine produces with **more than one part**, so
//! the page-less invariant's bijection is exercised here for the first time in both directions;
//! and it is the first format whose addresses could plausibly have been *counted* rather than
//! read, which is what the sparse-row and rels-resolution tests exist to refuse.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ethos_parser_core::{
    CellTextSource, CellValueType, GeometryAbsence, GeometryPresence, NativeLocator,
    NodeAttributes, Profile, XlsxLocator,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("workbook.xlsx");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every cell node's locator, in artifact order.
fn locators(sealed: &ethos_parser_core::DocumentRepresentation) -> Vec<XlsxLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Xlsx(l) => l.clone(),
            other => panic!("a cell must carry a workbook address, not {other:?}"),
        })
        .collect()
}

/// The node at a given sheet and address, if this artifact has one.
fn cell_at<'a>(
    sealed: &'a ethos_parser_core::DocumentRepresentation,
    sheet: &str,
    column: &str,
    row: u32,
) -> Option<&'a ethos_parser_core::Node> {
    sealed
        .payload()
        .nodes
        .iter()
        .find(|n| match &n.native_locator {
            NativeLocator::Xlsx(l) => l.sheet == sheet && l.column == column && l.row == row,
            _ => false,
        })
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The gate sentence, executable.** A known phrase is on a node, addressed by sheet/row/column.
#[test]
fn a_cell_resolves_to_a_node_addressed_by_the_workbook_itself() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let payload = sealed.payload();

    let node = payload
        .nodes
        .iter()
        .find(|n| n.text == "Rows & columns are S3.")
        .expect("the phrase is in the workbook, so it is in the artifact");

    match &node.native_locator {
        NativeLocator::Xlsx(l) => {
            assert_eq!(l.part, "xl/worksheets/sheet1.xml");
            assert_eq!(l.sheet, "Ledger");
            assert_eq!(l.row, 1);
            assert_eq!(l.column, "A");
        }
        other => panic!("a cell must carry a workbook address, not {other:?}"),
    }

    assert_eq!(
        payload.nodes.iter().filter(|n| n.id == node.id).count(),
        1,
        "the id a citation would carry addresses exactly one node"
    );
    assert!(
        !payload.nodes.iter().any(|n| n.id.as_str() == "s-forged"),
        "an id nobody minted is in no artifact"
    );
}

/// **The single highest-value test in the slice.** The sheet-to-part binding is read from
/// `xl/_rels/workbook.xml.rels`, not guessed from position.
///
/// The fixture's second sheet is `xl/worksheets/sheet3.xml` behind `rId7` — the shape a real
/// workbook takes after a sheet is deleted. A reader that assumed `sheet{n}.xml` in `<sheets>`
/// order would look for `sheet2.xml`, and a reader that dropped the `&amp;` would call the sheet
/// `Notes  sources`. Both are wrong *addresses*, which is worse than a missing one.
#[test]
fn the_sheet_name_is_read_through_the_rels_not_guessed_from_position() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");

    let node = cell_at(&sealed, "Notes & sources", "A", 1)
        .expect("the second sheet's only cell resolves through its relationship");
    assert_eq!(node.text, "Read because the workbook listed it.");

    match &node.native_locator {
        NativeLocator::Xlsx(l) => assert_eq!(
            l.part, "xl/worksheets/sheet3.xml",
            "the part comes from the relationship, and no `sheet2.xml` exists"
        ),
        other => panic!("expected a workbook address, got {other:?}"),
    }
}

/// A row the file numbers 12 is row 12, not the third row anybody counted.
#[test]
fn a_row_the_file_numbers_twelve_is_not_the_third_row() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let rows: BTreeSet<u32> = locators(&sealed)
        .iter()
        .filter(|l| l.sheet == "Ledger")
        .map(|l| l.row)
        .collect();

    assert!(rows.contains(&12), "the sheet's third row is numbered 12");
    assert!(
        !rows.contains(&3),
        "no cell is at row 3 — a counted address would have put one there"
    );
    assert_eq!(rows, BTreeSet::from([1, 2, 12]));
}

// -------------------------------------------------------------------------------------------
// The shape of the artifact
// -------------------------------------------------------------------------------------------

/// `pages` is empty, every geometry row is typed absence, and every parent is a part.
#[test]
fn the_artifact_has_no_pages_no_geometry_and_a_part_for_a_parent() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let payload = sealed.payload();

    assert!(
        payload.pages.is_empty(),
        "a workbook has no pages to declare"
    );
    assert!(
        !payload.nodes.is_empty(),
        "and this is not vacuous: the artifact has nodes"
    );

    for (index, node) in payload.nodes.iter().enumerate() {
        assert!(
            node.parent.as_str().starts_with('d'),
            "node {} is parented by `{}`, which is not a part id",
            node.id,
            node.parent
        );
        // The exact variant, not merely "absent": `NotReportedByReader` would mean the reader
        // tried to measure a cell and failed, and nothing here ever tries.
        assert_eq!(
            sealed.geometry()[index].presence,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            "node {} carries the wrong kind of absence",
            node.id
        );
    }
}

/// **The multi-part case, which no artifact before this one produced.**
///
/// Two sheets are two parts, two part ids and two independent 1-based ordinal sequences. This is
/// what `check_page_less_shape`'s bijection was written for and what v2-S2 could not exercise
/// with a one-part format.
#[test]
fn a_workbook_with_two_sheets_seals_with_two_parts() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let payload = sealed.payload();

    let parents: BTreeSet<&str> = payload.nodes.iter().map(|n| n.parent.as_str()).collect();
    let parts: BTreeSet<String> = locators(&sealed).iter().map(|l| l.part.clone()).collect();
    assert_eq!(parents.len(), 2, "one part id per sheet");
    assert_eq!(parts.len(), 2, "one part name per sheet");

    // Ordinals are contiguous from 1 **within each part**, which is the rule `check_structure`
    // enforces per parent rather than per document.
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

    // And the pairing is a bijection in both directions, which is the integrity a declared-page
    // list buys a PDF and which this format gets from its own locators.
    let mut pairs: Vec<(String, String)> = payload
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Xlsx(l) => (n.parent.as_str().to_string(), l.part.clone()),
            other => panic!("expected a workbook address, got {other:?}"),
        })
        .collect();
    pairs.sort();
    pairs.dedup();
    assert_eq!(pairs.len(), 2, "one part id means exactly one part name");
}

/// The workbook profile is its own, and is neither the PDF one nor the DOCX one.
#[test]
fn the_artifact_declares_its_own_profile_and_its_own_media_type() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let payload = sealed.payload();

    assert_eq!(
        payload.source.media_type,
        ethos_parser_office::XLSX_MEDIA_TYPE
    );
    assert_eq!(
        payload.identity.profile_sha256,
        Profile::xlsx_v0().profile_sha256().expect("a digest")
    );
    assert_ne!(
        Profile::xlsx_v0().profile_sha256().expect("a digest"),
        Profile::default().profile_sha256().expect("a digest"),
        "a workbook artifact is not comparable with a PDF one"
    );
    assert_ne!(
        Profile::xlsx_v0().profile_sha256().expect("a digest"),
        Profile::docx_v0().profile_sha256().expect("a digest"),
        "and it is not comparable with a DOCX one either"
    );
    assert!(
        !Profile::xlsx_v0().capabilities.tables,
        "this slice emits cells, not this engine's table IR — see `Profile::xlsx_v0`"
    );
}

/// **The `coordinate_system` decision of v2-S3, pinned.**
///
/// Both page-less profiles keep the inert `centipoint`/`top-left` declaration rather than
/// acquiring a mode enum, because nothing reads the value for a page-less artifact. The property
/// that makes it safe is the one asserted here: **no artifact under either profile emits a
/// coordinate.**
#[test]
fn the_page_less_profiles_declare_the_same_inert_coordinate_system() {
    assert_eq!(
        Profile::xlsx_v0().coordinate_system,
        Profile::docx_v0().coordinate_system
    );
    assert_eq!(
        Profile::xlsx_v0().coordinate_system,
        Profile::default().coordinate_system,
        "the declaration is unchanged, so no PDF artifact's profile hash moved for it"
    );
    assert!(!Profile::xlsx_v0().capabilities.measured_ink_boxes);
    assert!(!Profile::docx_v0().capabilities.measured_ink_boxes);

    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    assert!(
        sealed
            .geometry()
            .iter()
            .all(|g| matches!(g.presence, GeometryPresence::Absent(_))),
        "zero measured geometry rows is what makes the declaration inert rather than wrong"
    );
}

// -------------------------------------------------------------------------------------------
// The text, and where it came from
// -------------------------------------------------------------------------------------------

/// A cell with no `t` is a number, which is SpreadsheetML's own default — never a string.
#[test]
fn a_cell_with_no_type_attribute_is_a_number_not_a_string() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let node = cell_at(&sealed, "Ledger", "B", 2).expect("B2 is in the fixture");

    assert_eq!(node.text, "42");
    match &node.attributes {
        NodeAttributes::OfficeCell(a) => {
            assert_eq!(a.value_type, CellValueType::Number);
            assert_eq!(a.text_source, CellTextSource::StoredValue);
        }
        other => panic!("a cell carries cell facts, not {other:?}"),
    }
}

/// A shared string split across `<r>` runs is one string, because that is what the sheet shows.
#[test]
fn a_shared_string_split_across_runs_is_one_string() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let node = cell_at(&sealed, "Ledger", "A", 2).expect("A2 is in the fixture");

    assert_eq!(
        node.text, "A quote binds to a cell and never to a print range.",
        "the runs join with no separator, and `xml:space=\"preserve\"` keeps the leading space"
    );
    match &node.attributes {
        NodeAttributes::OfficeCell(a) => assert_eq!(a.value_type, CellValueType::SharedString),
        other => panic!("expected cell facts, got {other:?}"),
    }
}

/// An inline string needs no shared table, and says so in its own attributes.
#[test]
fn an_inline_string_is_read_from_the_cell_itself() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let node = cell_at(&sealed, "Ledger", "C", 2).expect("C2 is in the fixture");

    assert_eq!(node.text, "Inline, not shared.");
    match &node.attributes {
        NodeAttributes::OfficeCell(a) => assert_eq!(a.value_type, CellValueType::InlineString),
        other => panic!("expected cell facts, got {other:?}"),
    }
}

/// **Nothing is evaluated.** A formula cell's text is the value the workbook cached, and the
/// artifact says that is where it came from.
#[test]
fn a_formula_cell_carries_its_cached_value_and_says_so() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    let node = cell_at(&sealed, "Ledger", "A", 12).expect("A12 is in the fixture");

    assert_eq!(node.text, "42", "the cached `<v>`, as stored");
    match &node.attributes {
        NodeAttributes::OfficeCell(a) => {
            assert_eq!(a.text_source, CellTextSource::CachedFormulaResult);
            assert_eq!(a.value_type, CellValueType::Number);
        }
        other => panic!("expected cell facts, got {other:?}"),
    }
    assert!(
        !sealed
            .payload()
            .nodes
            .iter()
            .any(|n| n.text.contains("SUM(")),
        "a formula's source is not a cell's text when the workbook cached a value for it"
    );
}

/// A cell with nothing in it is not a node — there was never a character to erase.
#[test]
fn an_empty_cell_is_not_a_node() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    assert!(
        cell_at(&sealed, "Ledger", "B", 12).is_none(),
        "`<c r=\"B12\"/>` holds no text, so it is no node"
    );
    assert_eq!(sealed.payload().nodes.len(), 7);
}

// -------------------------------------------------------------------------------------------
// Declared erasure, detection, and every failure closed
// -------------------------------------------------------------------------------------------

/// **A14.** Parts that carry text and were not read are counted and named.
#[test]
fn parts_this_slice_does_not_read_are_declared_with_a_count() {
    let sealed =
        ethos_parser_office::read(&fixture("workbook-unread-parts")).expect("the fixture reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("a chart, a drawing and a comment list are three unread text parts");

    assert!(
        limitation.detail.contains('3'),
        "the count is in the message: {}",
        limitation.detail
    );
    for absent in ["Revenue nobody read", "A remark nobody read."] {
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

/// **A listed sheet that is not a worksheet is declared too — the other half of A14.**
///
/// Authored inline rather than as a fixture because the case needs a package with a dialog sheet
/// and *no* chart, drawing, comment or pivot-cache part: a real chart sheet drags `xl/charts/`
/// and `xl/drawings/` along, so the unread-parts count would fire anyway and this branch would
/// stay unproven. Deleting `|| non_worksheets > 0` from the reader must fail this test.
#[test]
fn a_listed_sheet_that_is_not_a_worksheet_is_declared_with_a_count() {
    const NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let archive = build_zip(&[
        (
            "xl/workbook.xml",
            &format!(
                r#"<workbook xmlns:r="{NS}"><sheets>
                     <sheet name="Data" sheetId="1" r:id="rId1"/>
                     <sheet name="Old macro tab" sheetId="2" r:id="rId2"/>
                   </sheets></workbook>"#
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            &format!(
                r#"<Relationships>
                     <Relationship Id="rId1" Type="{NS}/worksheet" Target="worksheets/sheet1.xml"/>
                     <Relationship Id="rId2" Type="{NS}/dialogsheet" Target="dialogsheets/sheet1.xml"/>
                   </Relationships>"#
            ),
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>read</t></is></c></row></sheetData></worksheet>"#,
        ),
        ("xl/dialogsheets/sheet1.xml", r#"<dialogsheet/>"#),
    ]);

    let sealed = ethos_parser_office::read(&archive).expect("the workbook reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("a listed sheet this reader passed over is a declared erasure");

    assert!(
        limitation.detail.contains('1') && limitation.detail.contains("sheet"),
        "the count and what it counts are both in the message: {}",
        limitation.detail
    );
    assert_eq!(
        sealed.payload().nodes.len(),
        1,
        "the worksheet is still read; only the dialog sheet is passed over"
    );
}

/// The clean workbook declares no erasure, so the limitation means something when it appears.
#[test]
fn a_workbook_with_nothing_unread_declares_no_erasure() {
    let sealed = ethos_parser_office::read(&fixture("workbook-cells")).expect("the fixture reads");
    assert!(!sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ));
}

/// Two reads of one workbook produce identical bytes.
#[test]
fn two_reads_of_one_workbook_produce_identical_bytes() {
    let bytes = fixture("workbook-cells");
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

/// **A4: the bytes decide**, and a workbook is not a document.
#[test]
fn detection_reads_the_bytes() {
    let xlsx = fixture("workbook-cells");
    assert!(ethos_parser_office::is_xlsx(&xlsx));
    assert!(
        !ethos_parser_office::is_docx(&xlsx),
        "a workbook lists no `word/document.xml`, so it is never claimed as a document"
    );
    assert!(ethos_parser_office::read(&xlsx).is_ok());

    let docx_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office/simple-paragraphs/document.docx");
    let docx = std::fs::read(&docx_path).expect("the DOCX fixture");
    assert!(
        !ethos_parser_office::is_xlsx(&docx),
        "and a document is never claimed as a workbook"
    );
    assert!(ethos_parser_office::is_docx(&docx));

    for not_a_workbook in [
        &b"%PDF-1.7\n"[..],
        b"",
        b"PK\x03\x04 but truncated right here",
        b"<worksheet/>",
    ] {
        assert!(!ethos_parser_office::is_xlsx(not_a_workbook));
    }
}

/// A package claiming to be both formats at once is a named refusal, not a coin toss.
#[test]
fn a_package_that_is_both_formats_is_refused_by_name() {
    // Authored here rather than as a fixture: it is not a document anybody would produce, and a
    // file in `fixtures/` implies something a real writer emits.
    let mut archive = Vec::new();
    {
        let both = build_zip(&[
            (
                "word/document.xml",
                "<w:document xmlns:w=\"x\"><w:body/></w:document>",
            ),
            ("xl/workbook.xml", "<workbook/>"),
        ]);
        archive.extend_from_slice(&both);
    }
    assert!(ethos_parser_office::is_docx(&archive));
    assert!(ethos_parser_office::is_xlsx(&archive));

    let error = ethos_parser_office::read(&archive).expect_err("ambiguous packages are refused");
    let text = error.to_string();
    assert!(text.contains("word/document.xml"), "{text}");
    assert!(text.contains("xl/workbook.xml"), "{text}");
}

/// A ZIP with neither main part is refused, and the message names both parts it looked for.
#[test]
fn a_zip_without_a_workbook_part_is_a_named_refusal() {
    let renamed = build_zip(&[("xl/notabook.xml", "<workbook/>")]);
    assert!(!ethos_parser_office::is_xlsx(&renamed));
    assert!(!ethos_parser_office::is_docx(&renamed));

    let error = ethos_parser_office::read(&renamed).expect_err("a ZIP that is neither format");
    let text = error.to_string();
    assert!(text.contains("word/document.xml"), "{text}");
    assert!(text.contains("xl/workbook.xml"), "{text}");
}

/// A truncated package is refused rather than read as far as it goes.
#[test]
fn a_truncated_workbook_is_refused() {
    let full = fixture("workbook-cells");
    for cut in [8, full.len() / 2, full.len() - 4] {
        let error =
            ethos_parser_office::read(&full[..cut]).expect_err("a truncated archive is refused");
        assert!(!error.to_string().is_empty());
    }
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// A minimal stored-entry ZIP, so a negative case can be authored without a fixture file.
fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for (name, body) in entries {
        let offset = out.len() as u32;
        let data = body.as_bytes();
        let crc = crc32(data);

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // version, flags, method, time
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
