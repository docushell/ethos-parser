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
    // Decision #135: each worksheet is a table, read by the rule `table_detection.office` names.
    assert!(Profile::xlsx_v0().capabilities.tables);
    assert_eq!(
        Profile::xlsx_v0().table_detection.office.as_deref(),
        Some(ethos_parser_core::TABLE_DETECTION_XLSX_SHEETS_V1)
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
// What a workbook's cells may carry (review 2026-09-25 F09)
// -------------------------------------------------------------------------------------------

const MAIN: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// A workbook of `(name, target, sheetData)` sheets, each bound by relationship to its target,
/// with a one-entry shared string table when `shared` is given. A target named twice is one part.
fn workbook(sheets: &[(&str, &str, String)], shared: Option<&str>) -> Vec<u8> {
    let mut listed = String::new();
    let mut rels = String::new();
    let mut entries: Vec<(String, String)> = Vec::new();
    for (i, (name, target, cells)) in sheets.iter().enumerate() {
        let n = i + 1;
        listed.push_str(&format!(
            r#"<sheet name="{name}" sheetId="{n}" r:id="rId{n}"/>"#
        ));
        rels.push_str(&format!(
            r#"<Relationship Id="rId{n}" Type="{RELS}/worksheet" Target="{target}"/>"#
        ));
        let part = format!("xl/{target}");
        if !entries.iter().any(|(p, _)| *p == part) {
            let sheet =
                format!(r#"<worksheet xmlns="{MAIN}"><sheetData>{cells}</sheetData></worksheet>"#);
            entries.push((part, sheet));
        }
    }
    if let Some(text) = shared {
        rels.push_str(&format!(
            r#"<Relationship Id="rIdS" Type="{RELS}/sharedStrings" Target="sharedStrings.xml"/>"#
        ));
        entries.push((
            "xl/sharedStrings.xml".into(),
            format!(r#"<sst xmlns="{MAIN}"><si><t>{text}</t></si></sst>"#),
        ));
    }
    entries.push((
        "xl/workbook.xml".into(),
        format!(
            r#"<workbook xmlns="{MAIN}" xmlns:r="{RELS}"><sheets>{listed}</sheets></workbook>"#
        ),
    ));
    entries.push((
        "xl/_rels/workbook.xml.rels".into(),
        format!(r#"<Relationships xmlns="{PACKAGE_RELS}">{rels}</Relationships>"#),
    ));
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_str()))
        .collect();
    build_zip(&borrowed)
}

/// `n` cells down column A from row 1, each citing shared string 0.
fn citing(n: u32) -> String {
    (1..=n)
        .map(|r| format!(r#"<row r="{r}"><c r="A{r}" t="s"><v>0</v></c></row>"#))
        .collect()
}

fn limit_of(error: &ethos_parser_core::EngineError) -> &str {
    match error {
        ethos_parser_core::EngineError::ResourceLimit { limit, .. } => limit,
        other => panic!("expected a resource limit, got {other}"),
    }
}

/// **A workbook's cells share one text budget, whichever sheet they sit on.**
///
/// A shared string is stored once and cloned into every cell citing it: an 8,017-byte workbook
/// citing one 1 MiB string from 1,024 cells made a 1.07 GB artifact. Here 40 cells on one sheet
/// and 25 on another cite a 1 MiB string: each sheet is under the 64 MiB ceiling, the workbook is
/// not. The failure this catches: no text ceiling, one per sheet, or a clone left uncounted.
#[test]
fn a_workbooks_cells_share_one_text_budget_across_sheets() {
    let mib = "x".repeat(1 << 20);
    let past = workbook(
        &[
            ("One", "worksheets/sheet1.xml", citing(40)),
            ("Two", "worksheets/sheet2.xml", citing(25)),
        ],
        Some(&mib),
    );
    let error = ethos_parser_office::read(&past).expect_err("65 MiB of cell text");
    assert_eq!(
        limit_of(&error),
        "text bytes the cells carry in this workbook"
    );
}

/// **Every cell's locator carries its sheet's name, so the name counts once per cell.**
///
/// One 1 MiB name over 64 cells is 64 MiB of locators from one attribute. The failure this
/// catches: a string that multiplies with the cell count and is never counted.
#[test]
fn a_sheet_name_is_counted_once_per_cell_that_carries_it() {
    let name = "N".repeat(1 << 20);
    let cells: String = (1..=64)
        .map(|r| format!(r#"<row r="{r}"><c r="A{r}"><v>1</v></c></row>"#))
        .collect();
    let past = workbook(&[(&name, "worksheets/sheet1.xml", cells)], None);
    let error = ethos_parser_office::read(&past).expect_err("64 MiB of addresses and more");
    assert_eq!(
        limit_of(&error),
        "address bytes the cells carry in this workbook"
    );
}

/// **Two sheets naming one part are refused before either is read.**
///
/// 400 sheets over one part read it 400 times: 1.55 GB before the representation refused the part
/// for carrying two ids — and a part holding no text cell was read for every sheet and accepted.
/// The failure this catches: a part read once per sheet that names it.
#[test]
fn two_sheets_naming_one_part_are_refused_by_name() {
    let one_cell = r#"<row r="1"><c r="A1"><v>1</v></c></row>"#.to_string();
    for cells in [one_cell, String::new()] {
        let twice = workbook(
            &[
                ("One", "worksheets/sheet1.xml", cells.clone()),
                ("Two", "worksheets/sheet1.xml", cells),
            ],
            None,
        );
        let error = ethos_parser_office::read(&twice).expect_err("one part, two sheets");
        let text = error.to_string();
        assert!(
            matches!(error, ethos_parser_core::EngineError::Malformed { .. }),
            "{text}"
        );
        assert!(
            text.contains("`One` and `Two`") && text.contains("`xl/worksheets/sheet1.xml`"),
            "{text}"
        );
    }
}

/// **Two entries of one name are refused, not read as the first** (review 2026-09-26 N26).
///
/// A second `xl/sharedStrings.xml` would repoint every shared cell for a consumer that reads the
/// last. The single table reads, so the refusal is the duplicate's.
#[test]
fn two_entries_of_one_name_are_refused_rather_than_read_as_the_first() {
    let sheets = [("One", "worksheets/sheet1.xml", citing(1))];
    let one = workbook(&sheets, Some("First"));
    let read = ethos_parser_office::read(&one).expect("one table reads");
    assert_eq!(read.payload().nodes[0].text, "First");

    // The same package with a second table: rebuilt from its entries, one more appended.
    let mut entries = zip_entries(&one);
    entries.push((
        "xl/sharedStrings.xml".to_string(),
        format!(r#"<sst xmlns="{MAIN}"><si><t>Second</t></si></sst>"#),
    ));
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    let error = ethos_parser_office::read(&build_zip(&borrowed)).expect_err("two tables");
    let text = error.to_string();
    assert!(
        matches!(error, ethos_parser_core::EngineError::Malformed { .. }),
        "{text}"
    );
    assert!(
        text.contains("`xl/sharedStrings.xml` more than once"),
        "{text}"
    );
}

/// Every entry of a stored archive `build_zip` wrote, in order.
fn zip_entries(archive: &[u8]) -> Vec<(String, String)> {
    let names = ethos_parser_office::zip::entry_names(archive).expect("names");
    names
        .into_iter()
        .map(|name| {
            let body = ethos_parser_office::zip::read_entry(archive, &name).expect("entry");
            (name, String::from_utf8(body).expect("utf-8"))
        })
        .collect()
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

// -------------------------------------------------------------------------------------------
// Decision #135: a sheet is a table, and a number shows as its format displays it
// -------------------------------------------------------------------------------------------

/// The financial set's number formats: a date, lakh grouping, and lakh grouping after `₹`.
const STYLES: &str = r#"<numFmts count="3"><numFmt numFmtId="164" formatCode="dd-mm-yyyy"/><numFmt numFmtId="165" formatCode="[&gt;=10000000]##\,##\,##\,##0.00;[&gt;=100000]##\,##\,##0.00;##,##0.00"/><numFmt numFmtId="166" formatCode="[&gt;=10000000]&quot;₹&quot;##\,##\,##\,##0.00;[&gt;=100000]&quot;₹&quot;##\,##\,##0.00;&quot;₹&quot;##,##0.00"/><numFmt numFmtId="167" formatCode="0.00E+00"/></numFmts><cellXfs count="6"><xf/><xf numFmtId="14"/><xf numFmtId="164"/><xf numFmtId="165"/><xf numFmtId="166"/><xf numFmtId="167"/></cellXfs>"#;

/// One sheet, `Ledger`, with a styles part and a `workbookPr`, and a second, `Notes`.
fn styled(sheet: &str, merges: &str, workbook_pr: &str) -> Vec<u8> {
    let entries = [
        (
            "xl/workbook.xml".to_string(),
            format!(
                r#"<workbook xmlns="{MAIN}" xmlns:r="{RELS}">{workbook_pr}<sheets><sheet name="Ledger" sheetId="1" r:id="rId1"/><sheet name="Notes &amp; terms" sheetId="2" r:id="rId2"/></sheets></workbook>"#
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels".to_string(),
            format!(
                r#"<Relationships xmlns="{PACKAGE_RELS}"><Relationship Id="rId1" Type="{RELS}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{RELS}/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="{RELS}/styles" Target="styles.xml"/></Relationships>"#
            ),
        ),
        (
            "xl/styles.xml".to_string(),
            format!(r#"<styleSheet xmlns="{MAIN}">{STYLES}</styleSheet>"#),
        ),
        (
            "xl/worksheets/sheet1.xml".to_string(),
            format!(
                r#"<worksheet xmlns="{MAIN}"><sheetData>{sheet}</sheetData>{merges}</worksheet>"#
            ),
        ),
        (
            "xl/worksheets/sheet2.xml".to_string(),
            format!(
                r#"<worksheet xmlns="{MAIN}"><sheetData><row r="3"><c r="C3" t="inlineStr"><is><t>Paid by NEFT</t></is></c></row></sheetData></worksheet>"#
            ),
        ),
    ];
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    build_zip(&borrowed)
}

fn inline(r: &str, text: &str) -> String {
    format!(r#"<c r="{r}" t="inlineStr"><is><t>{text}</t></is></c>"#)
}

fn ledger() -> String {
    format!(
        r#"<row r="1">{}</row><row r="2">{}{}{}</row><row r="3"><c r="A3" s="1"><v>46339</v></c>{}<c r="C3" s="3"><v>116000</v></c><c r="D3" s="4"><v>493098.50</v></c></row><row r="4"><c r="A4" s="2"><v>46340</v></c><c r="C4" s="5"><v>12345</v></c><c r="D4"><f>D3</f><v>39000.5</v></c></row>"#,
        inline("A1", "Ledger FY 2026-27"),
        inline("A2", "Date"),
        inline("B2", "Party"),
        inline("C2", "Debit"),
        inline("B3", "Kiran Builders"),
    )
}

const TITLE_MERGE: &str = r#"<mergeCells count="1"><mergeCell ref="A1:D1"/></mergeCells>"#;

fn office_cell(node: &ethos_parser_core::Node) -> &ethos_parser_core::OfficeCellAttributes {
    match &node.attributes {
        NodeAttributes::OfficeCell(a) => a,
        other => panic!("a cell carries cell attributes, not {other:?}"),
    }
}

/// **A number reads as the workbook displays it, and keeps what it stores.** The date is the
/// day numFmtId 14 shows, the amounts carry their `₹` and lakh grouping, and the stored value is
/// on the record beside each. The failure this catches: a serial or a bare float in the text, or a
/// formatted string with the stored value lost.
#[test]
fn a_number_cell_shows_its_format_and_keeps_its_stored_value() {
    let sealed = ethos_parser_office::read(&styled(&ledger(), TITLE_MERGE, "")).expect("reads");
    for (column, row, text, stored) in [
        ("A", 3, "2026-11-13", Some("46339")),
        ("A", 4, "14-11-2026", Some("46340")),
        ("C", 3, "1,16,000.00", Some("116000")),
        ("D", 3, "₹4,93,098.50", Some("493098.50")),
        // A cached formula result is shown as its format shows it too; `General` shows as stored.
        ("D", 4, "39000.5", None),
        ("B", 3, "Kiran Builders", None),
    ] {
        let node = cell_at(&sealed, "Ledger", column, row).expect("the cell is a node");
        assert_eq!(node.text, text, "{column}{row}");
        assert_eq!(
            office_cell(node).stored_value.as_deref(),
            stored,
            "{column}{row}"
        );
        assert_eq!(
            node.derivation,
            ethos_parser_core::DerivationClass::Extracted
        );
    }
    assert_eq!(
        office_cell(cell_at(&sealed, "Ledger", "D", 4).unwrap()).text_source,
        CellTextSource::CachedFormulaResult
    );
}

/// **A format this build does not render is counted, and the cell reads as it stores.**
#[test]
fn a_number_format_this_build_does_not_render_is_counted() {
    let sealed = ethos_parser_office::read(&styled(&ledger(), "", "")).expect("reads");
    let node = cell_at(&sealed, "Ledger", "C", 4).expect("C4 is a node");
    assert_eq!(
        node.text, "12345",
        "a scientific code shows the stored value"
    );
    assert_eq!(office_cell(node).stored_value, None);
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::OFFICE_NUMBER_FORMATS_NOT_RENDERED)
        .expect("the unrendered format is declared");
    assert!(
        limitation.detail.starts_with("1 number cell(s)"),
        "{}",
        limitation.detail
    );

    // Nothing unrendered, nothing declared.
    let clean = ledger().replace(r#"<c r="C4" s="5"><v>12345</v></c>"#, "");
    let sealed = ethos_parser_office::read(&styled(&clean, "", "")).expect("reads");
    assert!(!sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == ethos_parser_core::codes::OFFICE_NUMBER_FORMATS_NOT_RENDERED));
}

/// **`date1904` moves serial 0 to 1904-01-01**, so one serial is a different day.
#[test]
fn the_1904_date_system_is_read_from_the_workbook() {
    let sheet = r#"<row r="1"><c r="A1" s="1"><v>44877</v></c></row>"#;
    let sealed = ethos_parser_office::read(&styled(sheet, "", r#"<workbookPr date1904="1"/>"#))
        .expect("reads");
    assert_eq!(
        cell_at(&sealed, "Ledger", "A", 1).unwrap().text,
        "2026-11-13"
    );
    let sealed = ethos_parser_office::read(&styled(sheet, "", "")).expect("reads");
    assert_eq!(
        cell_at(&sealed, "Ledger", "A", 1).unwrap().text,
        "2022-11-12"
    );
}

/// **A worksheet is one table, each cell at the address the sheet states.** Slots count from the
/// sheet's first row and column holding a cell; the merge the title opens spans four columns; a
/// cell's slot names its node, and its node keeps the address. The failure this catches: cells as
/// paragraphs, a slot counted rather than read, or a merge dropped.
#[test]
fn a_worksheet_is_one_table_at_the_addresses_it_states() {
    let sealed = ethos_parser_office::read(&styled(&ledger(), TITLE_MERGE, "")).expect("reads");
    let payload = sealed.payload();
    assert!(payload.assurance.capabilities.tables);
    assert_eq!(payload.tables.len(), 2, "one table per worksheet");
    // It looked, so it declares no absence; and no ink rule ran, so it declares no ink rule's scope.
    for code in [
        ethos_parser_core::codes::TABLES_NOT_EXTRACTED,
        ethos_parser_core::codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED,
    ] {
        assert!(
            !payload.assurance.limitations.iter().any(|l| l.code == code),
            "{code}"
        );
    }

    let ledger = &payload.tables[0];
    assert_eq!(
        ledger.detection_rule,
        ethos_parser_core::TABLE_DETECTION_XLSX_SHEETS_V1
    );
    assert_eq!(
        ledger.derivation,
        ethos_parser_core::DerivationClass::Extracted
    );
    assert_eq!((ledger.rows, ledger.columns), (4, 4));
    assert!(matches!(
        ledger.geometry,
        GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind)
    ));
    assert!(matches!(
        ledger.locator_check.outcome,
        ethos_parser_core::CheckStatus::NotApplicable { .. }
    ));
    let at = |row: u32, column: u32| {
        ledger
            .cells
            .iter()
            .find(|c| (c.position.row, c.position.column) == (row, column))
            .unwrap_or_else(|| panic!("a cell at ({row}, {column})"))
    };
    let title = at(0, 0);
    assert_eq!((title.position.rowspan, title.position.colspan), (1, 4));
    assert_eq!(title.text, "Ledger FY 2026-27");
    let party = at(2, 1);
    assert_eq!(party.text, "Kiran Builders");
    let node = payload
        .nodes
        .iter()
        .find(|n| n.id == party.node_ids[0])
        .expect("the cell names a node");
    assert_eq!(
        node.native_locator,
        NativeLocator::Xlsx(XlsxLocator {
            part: "xl/worksheets/sheet1.xml".into(),
            sheet: "Ledger".into(),
            row: 3,
            column: "B".into(),
        })
    );
    // Every node is in exactly one cell, and its table is parented where the node is.
    assert_eq!(
        payload.tables.iter().map(|t| t.cells.len()).sum::<usize>(),
        payload.nodes.len()
    );
    assert_eq!(ledger.page, node.parent);

    // The second sheet's one cell, at C3, is its table's (0, 0).
    let notes = &payload.tables[1];
    assert_eq!((notes.rows, notes.columns), (1, 1));
    assert_eq!(notes.cells[0].text, "Paid by NEFT");
}

/// **The Markdown is a table under its sheet's name.** The name is syntax, quoted from the
/// locator every cell carries; the cells are source; the census still balances.
#[test]
fn a_worksheet_projects_as_a_table_under_its_name() {
    let sealed = ethos_parser_office::read(&styled(&ledger(), TITLE_MERGE, "")).expect("reads");
    let profile = Profile::default();
    let md = ethos_parser_core::to_markdown(
        &sealed,
        &profile.parser_version,
        &profile.profile_sha256().unwrap(),
        &profile.markdown_rule,
    )
    .expect("projects");
    assert!(
        md.markdown.starts_with(
            "# Ledger\n\n| Ledger FY 2026-27 |  |  |  |\n| --- | --- | --- | --- |\n| Date | Party | Debit |  |\n| 2026-11-13 | Kiran Builders | 1,16,000.00 | ₹4,93,098.50 |"
        ),
        "{}",
        md.markdown
    );
    assert!(
        md.markdown.contains("# Notes & terms\n\n| Paid by NEFT |"),
        "{}",
        md.markdown
    );
    let heading = md.anchor_map.segments.first().expect("a segment");
    assert_eq!(heading.kind, ethos_parser_core::SegmentKind::Syntax);
    assert!(md.coverage.balances());
    let html = ethos_parser_core::to_html(
        &sealed,
        &profile.parser_version,
        &profile.profile_sha256().unwrap(),
        &profile.html_rule,
    )
    .expect("projects");
    assert!(
        html.html.contains("<h1>Notes &amp; terms</h1>\n<table>"),
        "{}",
        html.html
    );
}

/// **A sheet too wide for a table is counted, and its cells are still read.**
#[test]
fn a_sheet_past_the_slot_ceiling_is_counted_rather_than_tabled() {
    // 2,000 rows by 702 columns is 1,404,000 slots.
    let sheet = format!(
        "{}{}",
        r#"<row r="1"><c r="A1"><v>1</v></c></row>"#,
        r#"<row r="2000"><c r="ZZ2000"><v>2</v></c></row>"#
    );
    let sealed = ethos_parser_office::read(&styled(&sheet, "", "")).expect("reads");
    let payload = sealed.payload();
    assert_eq!(payload.tables.len(), 1, "only the second sheet is a table");
    assert!(cell_at(&sealed, "Ledger", "ZZ", 2000).is_some());
    let limitation = payload
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::OFFICE_TABLES_NOT_BUILT)
        .expect("declared");
    assert!(
        limitation.detail.starts_with("1 worksheet(s)"),
        "{}",
        limitation.detail
    );
}

// -------------------------------------------------------------------------------------------
// Decision #135, after review: one slot budget, covered cells, styles that will not read
// -------------------------------------------------------------------------------------------

fn limitation<'a>(
    sealed: &'a ethos_parser_core::DocumentRepresentation,
    code: &str,
) -> Option<&'a ethos_parser_core::Limitation> {
    sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == code)
}

/// **One slot budget for the whole workbook.** Two sheets of 600,000 slots each: the first is a
/// table, the second would pass the million and is counted. The failure this catches: a ceiling
/// per table, which let twenty such sheets in an 8 KB file project 60 MB.
#[test]
fn a_workbooks_tables_share_one_slot_budget() {
    let sheet =
        r#"<row r="1"><c r="A1"><v>1</v></c></row><row r="600"><c r="ALL600"><v>2</v></c></row>"#;
    let entries = [
        (
            "xl/workbook.xml".to_string(),
            format!(
                r#"<workbook xmlns="{MAIN}" xmlns:r="{RELS}"><sheets><sheet name="A" sheetId="1" r:id="rId1"/><sheet name="B" sheetId="2" r:id="rId2"/></sheets></workbook>"#
            ),
        ),
        (
            "xl/_rels/workbook.xml.rels".to_string(),
            format!(
                r#"<Relationships xmlns="{PACKAGE_RELS}"><Relationship Id="rId1" Type="{RELS}/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="{RELS}/worksheet" Target="worksheets/sheet2.xml"/></Relationships>"#
            ),
        ),
        (
            "xl/worksheets/sheet1.xml".to_string(),
            format!(r#"<worksheet xmlns="{MAIN}"><sheetData>{sheet}</sheetData></worksheet>"#),
        ),
        (
            "xl/worksheets/sheet2.xml".to_string(),
            format!(r#"<worksheet xmlns="{MAIN}"><sheetData>{sheet}</sheetData></worksheet>"#),
        ),
    ];
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    let sealed = ethos_parser_office::read(&build_zip(&borrowed)).expect("reads");
    assert_eq!(sealed.payload().tables.len(), 1);
    assert_eq!(
        sealed.payload().nodes.len(),
        4,
        "every cell is still a node"
    );
    let l =
        limitation(&sealed, ethos_parser_core::codes::OFFICE_TABLES_NOT_BUILT).expect("declared");
    assert!(l.detail.starts_with("1 worksheet(s)"), "{}", l.detail);
}

/// **A cell under another cell's merge takes no slot, and its text is not lost.** Excel shows the
/// merge, not `hidden`; the node stays and is counted. The failure this catches: `| head | hidden |`,
/// a merge the file states drawn as two cells.
#[test]
fn a_cell_under_a_merge_takes_no_slot_and_keeps_its_node() {
    let sheet = format!(
        r#"<row r="1">{}{}</row><row r="2"><c r="A2"><v>1</v></c><c r="B2"><v>2</v></c></row>"#,
        inline("A1", "head"),
        inline("B1", "hidden")
    );
    let merge = r#"<mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells>"#;
    let sealed = ethos_parser_office::read(&styled(&sheet, merge, "")).expect("reads");
    let ledger = &sealed.payload().tables[0];
    let texts: Vec<_> = ledger
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
    assert_eq!(
        texts,
        vec![(0, 0, 2, "head"), (1, 0, 1, "1"), (1, 1, 1, "2")]
    );
    assert!(
        cell_at(&sealed, "Ledger", "B", 1).is_some(),
        "the covered text is a node"
    );
    let l = limitation(
        &sealed,
        ethos_parser_core::codes::OFFICE_COVERED_CELLS_NOT_PLACED,
    )
    .expect("declared");
    assert!(l.detail.starts_with("1 cell(s)"), "{}", l.detail);
}

/// **A styles part that will not read refuses nothing.** A missing part, a `numFmtId` that is no
/// number, and a truncated part each leave the workbook reading, styled numbers showing what they
/// store, counted. The failure this catches: a workbook head1 read refused over its formatting.
#[test]
fn a_styles_part_that_will_not_read_refuses_nothing() {
    let sheet = r#"<row r="1"><c r="A1" s="1"><v>46339</v></c><c r="B1"><v>7</v></c></row>"#;
    let good = styled(sheet, "", "");
    let rebuilt = |styles: Option<&str>| {
        let mut entries = zip_entries(&good);
        entries.retain(|(n, _)| n != "xl/styles.xml");
        if let Some(body) = styles {
            entries.push(("xl/styles.xml".into(), body.to_string()));
        }
        let borrowed: Vec<(&str, &str)> = entries
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_str()))
            .collect();
        build_zip(&borrowed)
    };
    let bad_id = format!(
        r#"<styleSheet xmlns="{MAIN}"><cellXfs count="2"><xf/><xf numFmtId="x"/></cellXfs></styleSheet>"#
    );
    let truncated = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs count="2"><xf/>"#);
    // Review of decision #135: where no styles were read, style zero's format is no more known
    // than style one's, so the cell that names none is counted too; a part that reads, its one
    // unreadable `numFmtId` aside, leaves style zero `General`.
    for (package, counted) in [
        (rebuilt(None), "2 number cell(s)"),
        (rebuilt(Some(&bad_id)), "1 number cell(s)"),
        (rebuilt(Some(&truncated)), "2 number cell(s)"),
    ] {
        let sealed = ethos_parser_office::read(&package).expect("formats refuse nothing");
        assert_eq!(cell_at(&sealed, "Ledger", "A", 1).unwrap().text, "46339");
        assert_eq!(cell_at(&sealed, "Ledger", "B", 1).unwrap().text, "7");
        let l = limitation(
            &sealed,
            ethos_parser_core::codes::OFFICE_NUMBER_FORMATS_NOT_RENDERED,
        )
        .expect("the styled cell is counted");
        assert!(l.detail.starts_with(counted), "{}", l.detail);
    }
}

/// **Style zero is unknown where the styles part will not read** (review of decision #135): a
/// workbook whose numbers all name style zero, or none, read as `General`, uncounted, and the
/// styles part that would not read went undeclared.
#[test]
fn style_zero_is_unknown_where_the_styles_part_will_not_read() {
    let sheet = r#"<row r="1"><c r="A1" s="0"><v>0.5</v></c><c r="B1"><v>7</v></c></row>"#;
    let mut entries = zip_entries(&styled(sheet, "", ""));
    for (name, body) in &mut entries {
        if name == "xl/styles.xml" {
            *body = format!(r#"<styleSheet xmlns="{MAIN}"><cellXfs count="1"><xf/>"#);
        }
    }
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    let sealed = ethos_parser_office::read(&build_zip(&borrowed)).expect("reads");
    assert_eq!(cell_at(&sealed, "Ledger", "A", 1).unwrap().text, "0.5");
    let l = limitation(
        &sealed,
        ethos_parser_core::codes::OFFICE_NUMBER_FORMATS_NOT_RENDERED,
    )
    .expect("declared");
    assert!(l.detail.starts_with("2 number cell(s)"), "{}", l.detail);
    assert!(l.detail.contains("xl/styles.xml"), "{}", l.detail);
}

/// **A merge covers its cells whatever its first cell holds, and wherever the file lists them**
/// (review of decision #135): for `A1:B1` with `A1` empty, `B1`'s text took an ordinary slot,
/// uncounted; listed before its anchor, it took one under the anchor's span.
#[test]
fn a_merge_covers_its_cells_whatever_its_first_cell_holds() {
    let merge = r#"<mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells>"#;
    let row2 =
        r#"<row r="2"><c r="A2"><v>1</v></c><c r="B2"><v>2</v></c><c r="C2"><v>3</v></c></row>"#;
    for (row1, want) in [
        (
            format!("{}{}", inline("B1", "hidden"), inline("C1", "c")),
            vec![
                (0, 2, 1, "c"),
                (1, 0, 1, "1"),
                (1, 1, 1, "2"),
                (1, 2, 1, "3"),
            ],
        ),
        (
            format!(
                "{}{}{}",
                inline("B1", "hidden"),
                inline("A1", "head"),
                inline("C1", "c")
            ),
            vec![
                (0, 0, 2, "head"),
                (0, 2, 1, "c"),
                (1, 0, 1, "1"),
                (1, 1, 1, "2"),
                (1, 2, 1, "3"),
            ],
        ),
    ] {
        let sheet = format!(r#"<row r="1">{row1}</row>{row2}"#);
        let sealed = ethos_parser_office::read(&styled(&sheet, merge, "")).expect("reads");
        let texts: Vec<_> = sealed.payload().tables[0]
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
        assert_eq!(texts, want);
        assert!(cell_at(&sealed, "Ledger", "B", 1).is_some(), "a node still");
        let l = limitation(
            &sealed,
            ethos_parser_core::codes::OFFICE_COVERED_CELLS_NOT_PLACED,
        )
        .expect("declared");
        assert!(l.detail.starts_with("1 cell(s)"), "{}", l.detail);
    }
}

/// **A conditional format's `<numFmt>` styles no cell.** One in `<dxfs>` with the id the cell's
/// style names must not replace the cell's own built-in date.
#[test]
fn a_conditional_formats_number_format_is_not_the_cells() {
    let sheet = r#"<row r="1"><c r="A1" s="1"><v>46339</v></c></row>"#;
    let mut entries = zip_entries(&styled(sheet, "", ""));
    for (name, body) in entries.iter_mut() {
        if name == "xl/styles.xml" {
            *body = format!(
                r#"<styleSheet xmlns="{MAIN}"><cellXfs count="2"><xf/><xf numFmtId="14"/></cellXfs><dxfs count="1"><dxf><numFmt numFmtId="14" formatCode="0.00%"/></dxf></dxfs></styleSheet>"#
            );
        }
    }
    let borrowed: Vec<(&str, &str)> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_str()))
        .collect();
    let sealed = ethos_parser_office::read(&build_zip(&borrowed)).expect("reads");
    assert_eq!(
        cell_at(&sealed, "Ledger", "A", 1).unwrap().text,
        "2026-11-13"
    );
}

/// **A built-in locale date is written ISO and counted**, so a consumer can tell the order was
/// chosen; a custom `dd-mm-yyyy` states its order and is not counted.
#[test]
fn a_locale_date_written_iso_is_counted() {
    let sealed = ethos_parser_office::read(&styled(&ledger(), "", "")).expect("reads");
    let l = limitation(
        &sealed,
        ethos_parser_core::codes::OFFICE_LOCALE_DATES_WRITTEN_ISO,
    )
    .expect("declared");
    assert!(l.detail.starts_with("1 date cell(s)"), "{}", l.detail);
}
