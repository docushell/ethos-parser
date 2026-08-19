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

//! `engine extract` reads an ODS, and the ODF sibling it does not read says so (v2-S6).
//!
//! The fifth sibling of `office_cli.rs`, proving the same claim once more: **one subcommand, one
//! artifact type, one serializer**. There is no `ethos.engine.ods.v0` and no `engine extract-ods`.
//!
//! What is new here is the refusal. v2-S5 measured that an `.ods` fell past the office branch to
//! the PDF reader and came back with *"expected a PDF header (%PDF-)"* — fail-closed, and naming
//! the wrong cause — and handed the fix to this slice. An `.ods` now reads; an `.odp` and an `.odg`
//! are refused **as OpenDocument**, by the type they declare.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn fixture() -> PathBuf {
    repo_root().join("fixtures/office/sheet-cells/workbook.ods")
}

fn extract(path: &std::path::Path) -> (i32, Vec<u8>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_engine"))
        .args(["extract", path.to_str().expect("utf-8 path")])
        .output()
        .expect("the engine binary runs");
    (
        out.status.code().unwrap_or(-1),
        out.stdout
            .strip_suffix(b"\n")
            .unwrap_or(&out.stdout)
            .to_vec(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// -------------------------------------------------------------------------------------------
// One subcommand, one artifact
// -------------------------------------------------------------------------------------------

#[test]
fn extract_reads_an_ods_into_the_same_artifact_type_a_pdf_produces() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");

    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON on stdout");
    assert_eq!(
        artifact["artifact_type"], "ethos.engine.representation.v0",
        "one artifact type — there is no `ethos.engine.ods.v0`"
    );
    let payload = &artifact["representation"];
    assert_eq!(
        payload["source"]["media_type"],
        "application/vnd.oasis.opendocument.spreadsheet"
    );
    assert_eq!(
        payload["pages"],
        json!([]),
        "a spreadsheet's page is a printer's, in ODF's spelling as in OOXML's"
    );
    assert_eq!(
        payload["tables"],
        json!([]),
        "cells are addresses the file states, not a grid a detector inferred"
    );
    assert!(
        payload["nodes"].as_array().expect("nodes").len() >= 9,
        "and that is not vacuous"
    );
}

/// A known cell arrives at the address the file states, on the new locator.
#[test]
fn a_known_cell_arrives_at_the_position_the_file_states() {
    let (_, stdout, _) = extract(&fixture());
    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    let nodes = artifact["representation"]["nodes"]
        .as_array()
        .expect("nodes");

    let total = nodes
        .iter()
        .find(|n| n["text"] == "Total")
        .expect("`Total` is a cell in the fixture");
    let locator = &total["native_locator"]["ods"];
    assert_eq!(locator["part"], "content.xml");
    assert_eq!(locator["table"], "Rows & Columns");
    assert_eq!(locator["row"], json!(1));
    assert_eq!(
        locator["column"],
        json!(5),
        "three repeated columns sit between it and the first cell: {locator:?}"
    );
    assert!(
        locator.get("page").is_none() && locator.get("bbox").is_none(),
        "and it carries no page and no box: {locator:?}"
    );
}

/// Dispatch is by content, never by extension.
#[test]
fn dispatch_is_by_content_not_by_extension() {
    let dir = tempdir();
    let renamed = dir.join("report.bin");
    std::fs::copy(fixture(), &renamed).expect("copy the fixture");
    let (code, stdout, stderr) = extract(&renamed);
    assert_eq!(
        code, 0,
        "an extension is a claim anybody can make: {stderr}"
    );
    assert!(!stdout.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_runs_over_one_spreadsheet_produce_identical_bytes() {
    let (_, first, _) = extract(&fixture());
    let (_, second, _) = extract(&fixture());
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// The refusal v2-S5 handed to this slice
// -------------------------------------------------------------------------------------------

/// **An unimplemented ODF sibling is refused as OpenDocument, not as a missing PDF header.**
///
/// The defect `docs/15-V2-MILESTONES.md` S5 recorded and deferred here. Before this slice `is_odt`
/// correctly declined an `.odp`, nothing else claimed it, and it fell through to the PDF reader —
/// which refused it with *"expected a PDF header (%PDF-) at byte 0"*. Correct outcome, wrong cause.
///
/// The assertion that carries the weight is the last one: the message must **not** mention `%PDF-`.
#[test]
fn an_unimplemented_opendocument_type_is_refused_by_name_rather_than_as_a_missing_pdf() {
    let dir = tempdir();
    for (extension, declared) in [
        ("odp", "application/vnd.oasis.opendocument.presentation"),
        ("odg", "application/vnd.oasis.opendocument.graphics"),
        ("odf", "application/vnd.oasis.opendocument.formula"),
    ] {
        let path = dir.join(format!("sibling.{extension}"));
        std::fs::write(&path, odf_package(declared)).expect("write the package");

        let (code, stdout, stderr) = extract(&path);
        assert_eq!(code, 2, "an unread format still fails closed: {stderr}");
        assert!(stdout.is_empty(), "a refusal prints no artifact");
        assert!(
            stderr.contains("OpenDocument"),
            "the refusal names the family: {stderr}"
        );
        assert!(
            stderr.contains(declared),
            "and the type the package declares about itself: {stderr}"
        );
        assert!(
            !stderr.contains("%PDF-"),
            "and it does NOT name a PDF header, which is the defect this closes: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The grounding boundary, unmoved
// -------------------------------------------------------------------------------------------

/// **(b) still holds.** `ethos.grounding.v1` is PDF-only, and an ODS is refused by name.
///
/// This passes with **no change to `engine-grounding`**.
#[test]
fn ground_refuses_a_page_less_spreadsheet_by_name() {
    let dir = tempdir();
    let (_, stdout, _) = extract(&fixture());
    let path = dir.join("representation.json");
    std::fs::write(&path, &stdout).expect("write the artifact");

    let out = Command::new(env!("CARGO_BIN_EXE_engine"))
        .args(["ground", path.to_str().expect("utf-8 path")])
        .output()
        .expect("the engine binary runs");

    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "a refusal prints no artifact");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("application/pdf"),
        "the refusal names the only media type the schema admits: {stderr}"
    );
    assert!(
        stderr.contains("14-V2-SCOPE.md"),
        "and the law it is refusing under: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The handle law, over unmodified MCP
// -------------------------------------------------------------------------------------------

/// An ODF cell resolves through the same fingerprint-checked handle path a PDF run uses, and a
/// forged id fails closed. `mcp.rs` is **unchanged** by this slice.
#[test]
fn node_get_resolves_a_cell_and_fails_closed_on_a_forged_id() {
    let (_, stdout, _) = extract(&fixture());
    let representation: Value = serde_json::from_slice(&stdout).expect("canonical JSON");

    let minted = representation["representation"]["nodes"][0]["id"]
        .as_str()
        .expect("extract minted at least one node")
        .to_string();

    let responses = mcp_session(&[
        call(
            "node_get",
            json!({ "representation": representation, "node_id": minted }),
            1,
        ),
        call(
            "node_get",
            json!({ "representation": representation, "node_id": "s-forged" }),
            2,
        ),
    ]);

    let ok = &responses[0]["result"];
    assert_eq!(ok["isError"], json!(false), "{ok:?}");
    assert_eq!(ok["structuredContent"]["id"], json!(minted));

    let locator = &ok["structuredContent"]["native_locator"]["ods"];
    assert_eq!(
        locator["part"], "content.xml",
        "the address that came back is the document's own: {locator:?}"
    );
    assert_eq!(locator["row"], json!(1));
    assert_eq!(locator["column"], json!(1));
    assert!(
        locator.get("page").is_none() && locator.get("bbox").is_none(),
        "and it carries no page and no box: {locator:?}"
    );

    let forged = &responses[1]["result"];
    assert_eq!(
        forged["isError"],
        json!(true),
        "a forged handle fails closed: {forged:?}"
    );
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// A minimal conforming ODF package declaring `media_type`: `mimetype` first and **stored**.
fn odf_package(media_type: &str) -> Vec<u8> {
    build_zip(&[
        ("mimetype", media_type),
        (
            "META-INF/manifest.xml",
            r#"<?xml version="1.0"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"><manifest:file-entry manifest:full-path="content.xml"/></manifest:manifest>"#,
        ),
        ("content.xml", r#"<?xml version="1.0"?><office/>"#),
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

fn call(name: &str, arguments: Value, id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    })
}

fn mcp_session(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engine"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the engine binary runs");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write");
        }
    }
    let out = child.wait_with_output().expect("the server exits");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("one response per line"))
        .collect()
}

/// A scratch directory beside the target dir, removed by each test that makes one.
fn tempdir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ethos-ods-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}
