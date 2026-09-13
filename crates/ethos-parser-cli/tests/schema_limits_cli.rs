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

//! **G2 through the surfaces a caller meets: the library, `ethos-parser ground`, and MCP.**
//!
//! `ethos-parser-grounding`'s own tests pin the rules on hand-built records. These start from a
//! table this engine actually detected — `fixtures/engine/ruled-table-grid` — lengthen one run and
//! one cell past the schema's string limit, re-seal, and follow the result out through every place
//! the degradation is declared, plus the refusal a document over the page limit gets.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use ethos_parser_core::assurance::{PageState, PageStateEntry};
use ethos_parser_core::{
    Assurance, Capabilities, DocumentRepresentation, IdAllocator, IdKind, NodeAttributes,
    PageRecord, Profile,
};
use ethos_parser_grounding::{ELEMENTS_OMITTED_OVER_LIMIT, TABLES_WITHHELD_OVER_LIMIT};
use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

/// A directory of its own per test, removed when the test ends, including when it panics.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("ethos-schema-limits-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        Scratch(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The ruled table fixture, as the engine represents it.
fn table_fixture() -> DocumentRepresentation {
    let profile = Profile::default();
    let path = repo_root().join("fixtures/engine/ruled-table-grid/document.pdf");
    let doc = ethos_parser_pdf::Document::open(&path, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    ethos_parser_pdf::to_representation(&extract, &profile).expect("represents")
}

/// 8,193 two-byte characters: 16,386 bytes, two past `ethos.grounding.v1`'s string limit.
fn over_long() -> String {
    "\u{e9}".repeat(8_193)
}

/// The fixture with the first cell's text lengthened, and — when `run_too` — the run inside it.
fn with_long_strings(run_too: bool) -> DocumentRepresentation {
    let repr = table_fixture();
    let mut payload = repr.payload().clone();
    let cell = &mut payload.tables[0].cells[0];
    cell.text = over_long();
    if run_too {
        let id = cell.node_ids[0].clone();
        let node = payload
            .nodes
            .iter_mut()
            .find(|n| n.id == id)
            .expect("the cell's run");
        node.text = over_long();
        if let NodeAttributes::TextRun(a) = &mut node.attributes {
            a.char_codes = node.text.chars().map(u32::from).collect();
        }
    }
    DocumentRepresentation::seal(payload, repr.geometry().to_vec()).expect("re-seals")
}

/// A record with more pages than the schema admits, and nothing on them.
fn five_thousand_and_one_pages() -> DocumentRepresentation {
    let repr = table_fixture();
    let mut payload = repr.payload().clone();
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().expect("digest"));
    payload.pages = (1..=5_001)
        .map(|index| PageRecord {
            id: alloc.next(IdKind::Page).expect("an id"),
            index,
            width: 61200,
            height: 79200,
            rotation: 0,
        })
        .collect();
    payload.nodes.clear();
    payload.tables.clear();
    let states = (1..=5_001)
        .map(|index| PageStateEntry {
            index,
            state: PageState::Processed,
        })
        .collect();
    payload.assurance =
        Assurance::new(Capabilities::V0, 5_001, states, Vec::new()).expect("assurance");
    DocumentRepresentation::seal(payload, Vec::new()).expect("seals")
}

fn write_repr(dir: &Path, name: &str, repr: &DocumentRepresentation) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, repr.to_canonical_bytes().expect("canonical")).expect("write");
    path
}

fn check_exit(artifact: &[u8]) -> i32 {
    ethos_parser_grounding::check::grounding_check(artifact, None)
        .expect("check runs")
        .exit_code()
}

/// **A detected table holding a cell past the string limit is withheld whole, and nothing else
/// moves.** Elements and spans are byte-for-byte the unedited document's. On this fixture a cell's
/// runs would form the same blocks table-owned or not, so this pins that withholding moves nothing
/// here — it cannot on its own prove the grouping ignores withholding.
#[test]
fn a_detected_table_with_a_cell_past_the_string_limit_is_withheld_whole() {
    let control = ethos_parser_grounding::project(&table_fixture()).expect("projects");
    assert_eq!(control.tables_withheld, None);
    assert!(control
        .source
        .tables
        .as_ref()
        .is_some_and(|t| !t.is_empty()));

    let p = ethos_parser_grounding::project(&with_long_strings(false)).expect("projects");
    let w = p.tables_withheld.expect("withheld");
    assert_eq!(
        (w.tables, w.oversized_cells, w.limitation_code),
        (1, 1, TABLES_WITHHELD_OVER_LIMIT)
    );
    assert!(!p.source.capabilities.tables);
    assert!(
        p.source.tables.is_none(),
        "withheld means absent, not empty"
    );
    assert_eq!(p.source.elements, control.source.elements);
    assert_eq!(p.source.spans, control.source.spans);
    assert_eq!(p.elements_omitted, None);
    let bytes = ethos_parser_grounding::to_canonical_bytes(&p.source).expect("canonical");
    assert_eq!(check_exit(&bytes), 0);
}

/// **`ground` declares both degradations on stderr and keeps stdout the artifact**; a document
/// past the page limit exits 2 with nothing on stdout and the limit named.
#[test]
fn ground_declares_what_the_limits_took_and_refuses_what_they_cannot() {
    let dir = Scratch::new("cli");
    let long = write_repr(&dir.0, "long.json", &with_long_strings(true));
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["ground", long.to_str().expect("utf-8")])
        .output()
        .expect("runs");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains(ELEMENTS_OMITTED_OVER_LIMIT), "{stderr}");
    assert!(stderr.contains(TABLES_WITHHELD_OVER_LIMIT), "{stderr}");
    assert!(
        stderr.contains("1 element(s) omitted") && stderr.contains("and 1 span(s) with them"),
        "the note carries the counts: {stderr}"
    );
    assert!(
        stderr.contains("1 table(s) withheld") && stderr.contains("1 cell(s) longer"),
        "{stderr}"
    );
    assert!(
        !stderr.contains("  "),
        "a note reads as prose, without runs of spaces: {stderr}"
    );
    assert_eq!(
        check_exit(out.stdout.strip_suffix(b"\n").unwrap_or(&out.stdout)),
        0
    );
    let artifact = String::from_utf8_lossy(&out.stdout);
    assert!(!artifact.contains("omitted") && !artifact.contains("withheld"));

    let pages = write_repr(&dir.0, "pages.json", &five_thousand_and_one_pages());
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["ground", pages.to_str().expect("utf-8")])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "a refusal writes no artifact");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("resource_limit") && stderr.contains("5000"),
        "{stderr}"
    );
}

/// **MCP's `ground` names both degradations in its summary, and a refusal is a tool error.**
#[test]
fn mcp_ground_names_the_degradations_and_refuses_what_cannot_degrade() {
    let dir = Scratch::new("mcp");
    let long = write_repr(&dir.0, "long.json", &with_long_strings(true));
    let pages = write_repr(&dir.0, "pages.json", &five_thousand_and_one_pages());

    let mut child = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("runs");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for (id, path) in [(1, &long), (2, &pages)] {
            let request = json!({
                "jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": { "name": "ground", "arguments": { "representation": path } }
            });
            writeln!(stdin, "{request}").expect("write");
        }
    }
    let out = child.wait_with_output().expect("exits");
    let replies: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("json"))
        .collect();

    let summary = replies[0]["result"]["content"][0]["text"]
        .as_str()
        .expect("summary");
    assert_eq!(replies[0]["result"]["isError"], json!(false));
    assert!(
        summary.contains("element(s) omitted") && summary.contains("table(s) withheld"),
        "{summary}"
    );

    assert_eq!(replies[1]["result"]["isError"], json!(true));
    let refusal = replies[1]["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        refusal.contains("resource limit") && refusal.contains("5000"),
        "{refusal}"
    );
}

/// `irs-fw9`: three measured tables and one tagged table, as the engine represents it.
fn fw9() -> DocumentRepresentation {
    let profile = Profile::default();
    let path = repo_root().join("fixtures/gate/irs-fw9.pdf");
    let doc = ethos_parser_pdf::Document::open(&path, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    ethos_parser_pdf::to_representation(&extract, &profile).expect("represents")
}

fn fw9_with_long_cell(tagged: bool) -> DocumentRepresentation {
    let repr = fw9();
    let mut payload = repr.payload().clone();
    let table = payload
        .tables
        .iter_mut()
        .find(|t| t.geometry.measured().is_none() == tagged)
        .expect("a table of that kind");
    table.cells[0].text = over_long();
    DocumentRepresentation::seal(payload, repr.geometry().to_vec()).expect("re-seals")
}

/// **A tagged table never reaches the artifact, so its cells cannot withhold anything.** The same
/// document with a tagged cell lengthened grounds to the unedited document's bytes exactly.
#[test]
fn an_over_long_cell_in_a_tagged_table_withholds_nothing() {
    let control = ethos_parser_grounding::project(&fw9()).expect("projects");
    let p = ethos_parser_grounding::project(&fw9_with_long_cell(true)).expect("projects");
    assert_eq!(p.tables_withheld, None);
    assert_eq!(
        ethos_parser_grounding::to_canonical_bytes(&p.source).expect("canonical"),
        ethos_parser_grounding::to_canonical_bytes(&control.source).expect("canonical"),
    );
}

/// **One over-long cell withholds every table, never just its own.** `irs-fw9` projects three
/// measured tables; lengthening a cell in one of them withholds all three.
#[test]
fn one_over_long_cell_withholds_every_table_in_the_document() {
    let control = ethos_parser_grounding::project(&fw9()).expect("projects");
    assert_eq!(control.source.tables.as_ref().map(Vec::len), Some(3));
    let p = ethos_parser_grounding::project(&fw9_with_long_cell(false)).expect("projects");
    let w = p.tables_withheld.expect("withheld");
    assert_eq!((w.tables, w.oversized_cells), (3, 1));
    assert!(!p.source.capabilities.tables && p.source.tables.is_none());
    assert_eq!(p.source.elements, control.source.elements);
    let bytes = ethos_parser_grounding::to_canonical_bytes(&p.source).expect("canonical");
    assert_eq!(check_exit(&bytes), 0);
}
