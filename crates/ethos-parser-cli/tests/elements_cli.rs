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

//! `ethos-parser elements` (decision #137): the bytes it prints, on PDFs and on office documents.
//!
//! The shell's properties — the printed bytes are the library's, `--source` prints what `extract`
//! piped into it prints, a repeat prints the same bytes — and the artifact's, on real fixtures:
//! every node in exactly one place, each label reached, a DOCX paragraph one element.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ethos_parser_core::{DocumentRepresentation, Profile};
use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

/// A directory of its own per test, removed when the test ends, including when it panics.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(label: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-elements-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    Scratch(dir)
}

/// PDFs that reach each label and each omission the view writes, a two-page PDF, and office
/// documents.
const SOURCES: [&str; 15] = [
    "fixtures/engine/heading-display-line/document.pdf",
    "fixtures/engine/heading-label-line/document.pdf",
    "fixtures/engine/untagged-artifact-furniture/document.pdf",
    "fixtures/engine/ruled-table-grid/document.pdf",
    "fixtures/engine/image-xobject-drawn/document.pdf",
    "fixtures/engine/figure-paths-drawn/document.pdf",
    "fixtures/engine/tagged-structure-roles/document.pdf",
    "fixtures/engine/tagged-list-items/document.pdf",
    "fixtures/engine/absent-font-metrics/document.pdf",
    "fixtures/engine/form-field-value/document.pdf",
    "fixtures/engine/annotation-contents/document.pdf",
    "fixtures/engine/shared-content-stream/document.pdf",
    "fixtures/office/simple-paragraphs/document.docx",
    "fixtures/office/workbook-cells/workbook.xlsx",
    "fixtures/office/deck-slides/deck.pptx",
];

fn path(source: &str) -> String {
    repo_root().join(source).to_str().unwrap().to_string()
}

fn elements_of(source: &str) -> Value {
    let out = engine(&["elements", "--source", &path(source)]);
    assert_eq!(out.status.code(), Some(0), "{source}: {:?}", out.stderr);
    serde_json::from_slice(&out.stdout).expect("canonical JSON")
}

fn texts(artifact: &Value) -> Vec<(&str, &str)> {
    artifact["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["label"].as_str().unwrap(), e["text"].as_str().unwrap()))
        .collect()
}

#[test]
fn the_source_path_the_two_step_path_and_a_repeat_print_the_same_bytes() {
    let dir = scratch("paths");
    for source in SOURCES {
        let source = path(source);
        let extracted = engine(&["extract", &source]);
        assert_eq!(extracted.status.code(), Some(0), "extract {source}");
        let record = dir.join("record.json");
        std::fs::write(&record, &extracted.stdout).unwrap();

        let one_step = engine(&["elements", "--source", &source]);
        let two_step = engine(&["elements", record.to_str().unwrap()]);
        let again = engine(&["elements", "--source", &source]);
        assert_eq!(one_step.status.code(), Some(0), "{source}");
        assert_eq!(
            one_step.stdout, two_step.stdout,
            "{source}: the two paths differ"
        );
        assert_eq!(one_step.stdout, again.stdout, "{source}: a repeat differs");

        // The library's bytes, and the newline every artifact ends with.
        let repr: DocumentRepresentation = serde_json::from_slice(&extracted.stdout).unwrap();
        let mut expected =
            ethos_parser_core::to_elements(&repr, &Profile::default().parser_version)
                .unwrap()
                .to_canonical_bytes()
                .unwrap();
        expected.push(b'\n');
        assert_eq!(
            one_step.stdout, expected,
            "{source}: not the library's bytes"
        );
    }
}

/// **Nothing is dropped silently.** Every node of the record is in exactly one place — a text
/// unit's or a table's `node_ids`, a picture's own image node, a form field's node, or `omitted` —
/// each label is reached, the record's own profile is stamped, pages run in order, and stderr says
/// what was omitted exactly when something was.
#[test]
fn every_node_is_in_exactly_one_place_and_every_label_is_reached() {
    let mut labels = BTreeSet::new();
    let mut reasons = BTreeSet::new();
    for source in SOURCES {
        let extracted = engine(&["extract", &path(source)]);
        let record: Value = serde_json::from_slice(&extracted.stdout).unwrap();
        let nodes = record["representation"]["nodes"].as_array().unwrap();
        let kinds: BTreeMap<&str, &str> = nodes
            .iter()
            .map(|n| (n["id"].as_str().unwrap(), n["kind"].as_str().unwrap()))
            .collect();
        let paginated = !record["representation"]["pages"]
            .as_array()
            .unwrap()
            .is_empty();
        let out = engine(&["elements", "--source", &path(source)]);
        let artifact: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            artifact["representation_sha256"],
            record["representation_c14n_sha256"]
        );
        assert_eq!(
            artifact["profile_sha256"], record["representation"]["identity"]["profile_sha256"],
            "{source}: the record's own profile"
        );

        let mut placed: Vec<&str> = Vec::new();
        let mut last_page = 0;
        for e in artifact["elements"].as_array().unwrap() {
            let label = e["label"].as_str().unwrap();
            labels.insert(label.to_string());
            let ids: Vec<&str> = e["node_ids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap())
                .collect();
            match label {
                // A picture holds its own image node; the runs inside it are elements of their own.
                "picture" => placed.extend(ids.first().filter(|id| kinds[*id] == "image")),
                _ => placed.extend(ids),
            }
            // A source with no pages has no page and no box to give.
            assert_eq!(e.get("page").is_some(), paginated, "{source}: {e}");
            if !paginated {
                assert!(e.get("bbox").is_none(), "{source}: {e}");
            }
            let page = e["page"].as_u64().unwrap_or(0);
            assert!(page >= last_page, "{source}: pages out of order");
            last_page = page;
        }
        let omitted = artifact["omitted"].as_array().unwrap();
        for o in omitted {
            reasons.insert(o["reason"].as_str().unwrap().to_string());
            let ids = o["node_ids"].as_array().unwrap();
            if o["reason"] != "figure-not-measured" {
                assert_eq!(o["count"].as_u64(), Some(ids.len() as u64), "{source}");
            }
            placed.extend(ids.iter().map(|id| id.as_str().unwrap()));
        }
        let mut sorted = placed.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), placed.len(), "{source}: a node in two places");
        assert_eq!(
            sorted.into_iter().collect::<BTreeSet<_>>(),
            kinds.keys().copied().collect::<BTreeSet<_>>(),
            "{source}: a node in no place"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            stderr.contains("omitted from the elements"),
            !omitted.is_empty(),
            "{source}: {stderr}"
        );
    }
    let all = [
        "form-field",
        "list-item",
        "page-footer",
        "page-header",
        "picture",
        "section-header",
        "table",
        "text",
    ];
    assert_eq!(labels, all.map(String::from).into(), "labels reached");
    let all = ["annotation", "no-measured-box"];
    assert_eq!(reasons, all.map(String::from).into(), "reasons reached");
}

/// **A form field is an element of its own** (decision #163), its name, type and value copied
/// from its node, after its page's text, with no box: the rectangle a field declares is not a
/// measured one.
#[test]
fn a_form_field_is_an_element_with_its_facts_and_no_box() {
    let artifact = elements_of("fixtures/engine/form-field-value/document.pdf");
    let field = artifact["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["label"] == "form-field")
        .expect("the field is an element");
    assert_eq!(field["text"], "Wendell Ashcroft-Byrne");
    assert_eq!(
        field["bbox"],
        Value::Null,
        "a declared rectangle is no bbox"
    );
    assert_eq!(field["node_ids"].as_array().unwrap().len(), 1);
    assert_eq!(
        field["form_field"],
        serde_json::json!({
            "name": "applicant_name",
            "field_type": "Tx",
            "value": {"kind": "text", "value": "Wendell Ashcroft-Byrne"},
        })
    );
    let last = artifact["elements"].as_array().unwrap().last().unwrap();
    assert_eq!(last["label"], "form-field", "after the page's text");
    assert!(artifact["omitted"].as_array().unwrap().is_empty());
}

/// A lone label over its text, in a document nothing else heads, is its heading (decision #159) and
/// a `section-header`, as both projections write it `#`; the label tier, level 7, stays `text`
/// (`ethos_parser_core::elements`' own test). The nine lines the reader measured off the page are
/// counted.
#[test]
fn a_lone_label_heads_its_document_and_its_unmeasured_lines_are_counted() {
    let artifact = elements_of("fixtures/engine/heading-label-line/document.pdf");
    assert_eq!(texts(&artifact)[0], ("section-header", "Loan terms"));
    let omitted = artifact["omitted"].as_array().unwrap();
    assert_eq!(omitted.len(), 1, "{artifact}");
    assert_eq!(omitted[0]["reason"], "no-measured-box");
    assert_eq!(omitted[0]["count"], 9);

    let empty = elements_of("fixtures/engine/absent-font-metrics/document.pdf");
    assert!(empty["elements"].as_array().unwrap().is_empty());
    assert!(
        empty["omitted"][0]["count"].as_u64().unwrap() > 0,
        "{empty}"
    );
}

/// A DOCX paragraph is one element, its text as the projections write it (decision #130), and a
/// line break at its head joins it rather than standing alone.
#[test]
fn a_docx_paragraph_is_one_element() {
    let artifact = elements_of("fixtures/office/simple-paragraphs/document.docx");
    assert_eq!(
        texts(&artifact),
        [
            ("text", "Evidence, not extraction."),
            ("text", "A quote binds to a run and never to a page."),
            ("text", "Rows & columns are S3."),
        ]
    );
    assert_eq!(
        artifact["elements"][2]["node_ids"],
        serde_json::json!(["s4", "s5"])
    );
    assert!(artifact["omitted"].as_array().unwrap().is_empty());
}

/// **A heading an EPUB or an ODT declares is a `section-header`** (review of decision #137), as
/// both projections write it `#`; it was `text`.
#[test]
fn a_heading_an_epub_or_odt_declares_is_a_section_header() {
    for source in [
        "fixtures/office/book-spine/book.epub",
        "fixtures/office/text-paragraphs/document.odt",
    ] {
        let first = &elements_of(source)["elements"][0];
        assert_eq!(
            (
                first["label"].as_str(),
                first["level"].as_u64(),
                first["text"].as_str()
            ),
            (
                Some("section-header"),
                Some(1),
                Some("Evidence, not extraction.")
            ),
            "{source}"
        );
    }
}

#[test]
fn a_painted_image_and_a_figure_region_are_pictures_with_boxes() {
    for (source, first_kind) in [
        ("fixtures/engine/image-xobject-drawn/document.pdf", "image"),
        (
            "fixtures/engine/figure-paths-drawn/document.pdf",
            "text_run",
        ),
    ] {
        let record: Value =
            serde_json::from_slice(&engine(&["extract", &path(source)]).stdout).unwrap();
        let kinds: BTreeMap<&str, &str> = record["representation"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| (n["id"].as_str().unwrap(), n["kind"].as_str().unwrap()))
            .collect();
        let artifact = elements_of(source);
        let pictures: Vec<&Value> = artifact["elements"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["label"] == "picture")
            .collect();
        assert_eq!(pictures.len(), 1, "{source}: {artifact}");
        assert!(pictures[0]["bbox"].is_array(), "{source}");
        if let Some(id) = pictures[0]["node_ids"][0].as_str() {
            assert_eq!(kinds[id], first_kind, "{source}");
        }
    }
}

#[test]
fn a_workbooks_sheet_is_a_named_table_element_with_its_cells_nodes() {
    let artifact = elements_of("fixtures/office/workbook-cells/workbook.xlsx");
    let tables: Vec<&Value> = artifact["elements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["label"] == "table")
        .collect();
    assert!(!tables.is_empty(), "a sheet is a table: {artifact}");
    for t in tables {
        assert!(
            t["table"]["name"].as_str().is_some_and(|n| !n.is_empty()),
            "{t}"
        );
        let from_cells: Vec<&Value> = t["table"]["cells"]
            .as_array()
            .expect("its grid")
            .iter()
            .flat_map(|c| c["node_ids"].as_array().unwrap())
            .collect();
        assert_eq!(
            t["node_ids"].as_array().unwrap().iter().collect::<Vec<_>>(),
            from_cells
        );
    }
}

#[test]
fn a_record_that_does_not_hash_to_its_digest_is_refused() {
    let dir = scratch("tampered");
    let extracted = engine(&["extract", &path(SOURCES[0])]);
    let tampered =
        String::from_utf8(extracted.stdout)
            .unwrap()
            .replacen("\"text\":\"", "\"text\":\"X", 1);
    let record = dir.join("tampered.json");
    std::fs::write(&record, tampered).unwrap();
    let out = engine(&["elements", record.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "no artifact on a refusal");
}
