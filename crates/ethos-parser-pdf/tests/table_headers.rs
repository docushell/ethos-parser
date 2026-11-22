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

//! Decision #167: a table's header cells, as the document or the rule that read the table states
//! them, reach the record and the element view.

use ethos_parser_core::{DocumentRepresentation, Profile};
use ethos_parser_pdf::Document;
use lopdf::{dictionary, Object, Stream};
use std::path::PathBuf;

fn record(bytes: &[u8]) -> DocumentRepresentation {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    ethos_parser_pdf::into_representation(extract, &profile).expect("seals")
}

fn headers(repr: &DocumentRepresentation) -> Vec<(String, bool)> {
    repr.payload()
        .tables
        .iter()
        .flat_map(|t| t.cells.iter())
        .map(|c| (c.text.clone(), c.header))
        .collect()
}

/// A ruled grid of numbers with its column names set above its top rule (decision #74): the
/// names are the table's first row, and its header.
#[test]
fn a_header_set_above_a_ruled_grid_is_its_header_row() {
    let mut doc = lopdf::Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(
        dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" },
    );
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"1 w 40 120 100 40 re S 140 120 100 40 re S 240 120 100 40 re S \
40 80 100 40 re S 140 80 100 40 re S 240 80 100 40 re S \
BT /F1 12 Tf 1 0 0 1 60 166 Tm (Alpha) Tj 1 0 0 1 160 166 Tm (Beta) Tj 1 0 0 1 260 166 Tm (Gamma) Tj \
1 0 0 1 50 134 Tm (11) Tj 1 0 0 1 150 134 Tm (12) Tj 1 0 0 1 250 134 Tm (13) Tj \
1 0 0 1 50 94 Tm (21) Tj 1 0 0 1 150 94 Tm (22) Tj 1 0 0 1 250 94 Tm (23) Tj ET"
            .to_vec(),
    ));
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "MediaBox" => vec![0.into(), 0.into(), 400.into(), 220.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
        ),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("saves");

    let repr = record(&bytes);
    let got = headers(&repr);
    assert_eq!(
        got,
        [
            ("Alpha", true),
            ("Beta", true),
            ("Gamma", true),
            ("11", false),
            ("12", false),
            ("13", false),
            ("21", false),
            ("22", false),
            ("23", false),
        ]
        .map(|(t, h)| (t.to_string(), h))
    );
    // And the element view names each value's column header by its cell id.
    let elements = ethos_parser_core::to_elements(&repr, "test").expect("projects");
    let grid = elements
        .elements
        .iter()
        .find_map(|e| e.table.as_ref())
        .expect("a table");
    let beta = &repr.payload().tables[0].cells[1].id;
    let twenty_two = grid.cells.iter().find(|c| c.text == "22").expect("a cell");
    assert_eq!(twenty_two.column_headers, [beta.clone()]);
}

/// A tagged table whose tree says `/TH` for a cell: that cell is a header on the record.
#[test]
fn a_tagged_th_is_a_header_cell_on_the_record() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/engine/tagged-table-agrees/document.pdf");
    let original = std::fs::read(path).expect("fixture readable");
    let tagged = |retag: bool| {
        let mut doc = lopdf::Document::load_mem(&original).expect("loads");
        if retag {
            let td = doc
                .objects
                .iter()
                .filter(|(_, o)| {
                    o.as_dict()
                        .ok()
                        .and_then(|d| d.get(b"S").ok())
                        .and_then(|s| s.as_name().ok())
                        == Some(b"TD".as_slice())
                })
                .map(|(id, _)| *id)
                .min()
                .expect("the fixture tags its cells /TD");
            doc.get_object_mut(td)
                .and_then(Object::as_dict_mut)
                .expect("a dictionary")
                .set("S", Object::Name(b"TH".to_vec()));
        }
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("saves");
        record(&bytes)
    };
    let count = |repr: &DocumentRepresentation| headers(repr).iter().filter(|(_, h)| *h).count();
    assert_eq!(count(&tagged(false)), 0);
    assert_eq!(count(&tagged(true)), 1, "the retagged cell, and only it");
}

/// A real tagged form: its `/TH` cells are headers on a table read from the tags alone and on a
/// drawn grid its tags agree with, and nowhere else.
#[test]
fn a_real_forms_th_cells_are_headers_on_both_kinds_of_table() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/gate/irs-fw9.pdf");
    let repr = record(&std::fs::read(path).expect("fixture readable"));
    let count = |rule: &str| {
        repr.payload()
            .tables
            .iter()
            .filter(|t| t.detection_rule.starts_with(rule))
            .flat_map(|t| t.cells.iter())
            .filter(|c| c.header)
            .count()
    };
    assert_eq!(count("tagged-tables"), 2, "read from the tags alone");
    assert_eq!(count("stroke-ruled"), 4, "a drawn grid its tags agree with");
}
