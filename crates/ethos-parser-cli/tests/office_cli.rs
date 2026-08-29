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

//! `ethos-parser extract` reads a DOCX, and every path that would have needed a page refuses (v2-S2).
//!
//! **One subcommand, one artifact type** (`docs/14-V2-SCOPE.md` §4). There is no
//! `ethos.parser.docx.v0` and no `ethos-parser extract-docx`: dispatch is by content, the record is the
//! same shape, and `node_get` over MCP works on it without having been taught a second format —
//! which is the practical form of the one-IR claim.

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
    repo_root().join("fixtures/office/simple-paragraphs/document.docx")
}

fn extract(path: &std::path::Path) -> (i32, Vec<u8>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
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
fn extract_reads_a_docx_into_the_same_artifact_type_a_pdf_produces() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");

    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    assert_eq!(
        artifact["artifact_type"], "ethos.parser.representation.v0",
        "there is no second canonical JSON per format (14 §4)"
    );
    assert!(
        artifact["representation"]["pages"]
            .as_array()
            .expect("pages")
            .is_empty(),
        "a DOCX has no page and the CLI invented none"
    );
    assert_eq!(
        artifact["representation"]["source"]["media_type"],
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
}

/// **Detection is content-based, so an extension cannot lie its way past it** (Anydoc's **A4**).
#[test]
fn a_renamed_document_still_reads_and_a_renamed_pdf_does_not_become_one() {
    let directory = tempdir();
    let disguised = directory.join("report.bin");
    std::fs::copy(fixture(), &disguised).expect("copy");
    let (code, stdout, stderr) = extract(&disguised);
    assert_eq!(
        code, 0,
        "an extension is a claim anybody can make: {stderr}"
    );
    assert!(!stdout.is_empty());

    // And the converse: a file *named* `.docx` that is not one is refused rather than read as an
    // empty document.
    let liar = directory.join("not-really.docx");
    std::fs::write(&liar, b"%PDF-1.7\nnot a package at all").expect("write");
    let (code, stdout, stderr) = extract(&liar);
    assert_eq!(code, 2, "a lie about the format is a named failure");
    assert!(
        stdout.is_empty(),
        "nothing is emitted for a document that did not read"
    );
    assert!(!stderr.trim().is_empty());
    std::fs::remove_dir_all(&directory).ok();
}

/// Byte-identical across two runs, on the rule every artifact in this repository is under.
#[test]
fn two_runs_over_one_docx_produce_identical_bytes() {
    let (_, first, _) = extract(&fixture());
    let (_, second, _) = extract(&fixture());
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// Under (b), `ground` refuses — and says why
// -------------------------------------------------------------------------------------------

/// **`ethos-parser ground` on a DOCX artifact projects the page-less shape (0.39.0).**
///
/// v2-S1 decided `ethos.grounding.v1` stays PDF-only. The failure names the media type and points
/// at the decision, so a caller learns what happened rather than seeing an empty projection.
#[test]
fn ground_projects_a_page_less_representation_by_name() {
    let directory = tempdir();
    let artifact = directory.join("representation.json");
    let (_, stdout, _) = extract(&fixture());
    std::fs::write(&artifact, &stdout).expect("write");

    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["ground", artifact.to_str().expect("utf-8 path")])
        .output()
        .expect("runs");

    assert_eq!(
        out.status.code(),
        Some(0),
        "since 0.39.0 a page-less representation projects: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let artifact: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("a grounding artifact is printed");
    assert_eq!(artifact["schema_version"], "1.1.0");
    assert_eq!(
        artifact["pages"].as_array().expect("pages").len(),
        0,
        "no page was synthesized"
    );
    let first = &artifact["elements"][0];
    assert!(
        first["locator"].as_str().is_some_and(|l| !l.is_empty()),
        "the native address travels on the element: {first}"
    );
    assert!(
        first.get("page").is_none() && first.get("bbox").is_none(),
        "a page-less element states no page and no bbox: {first}"
    );
    std::fs::remove_dir_all(&directory).ok();
}

// -------------------------------------------------------------------------------------------
// The handle law, over MCP, on an artifact MCP was never taught about
// -------------------------------------------------------------------------------------------

/// **`node_get` resolves a DOCX run by the id the engine minted, and refuses one it did not.**
///
/// This is v2's gate sentence in the form v2-S1's decision (b) leaves available: not
/// `ethos.grounding.v1`, but a quote that resolves to a node through the same fingerprint-checked
/// handle path a PDF uses. `mcp.rs` is **unchanged** by this slice — the one-IR claim is what
/// makes that possible.
#[test]
fn node_get_resolves_a_docx_run_and_fails_closed_on_a_forged_id() {
    let (_, stdout, _) = extract(&fixture());
    let representation: Value = serde_json::from_slice(&stdout).expect("canonical JSON");

    // Read off the artifact rather than hardcoded, so a forged-id assertion cannot pass because
    // the minted one stopped existing.
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
    assert_eq!(
        ok["structuredContent"]["native_locator"]["docx"]["part"], "word/document.xml",
        "the locator that came back is the document's own address"
    );

    let forged = &responses[1]["result"];
    assert_eq!(
        forged["isError"],
        json!(true),
        "a forged handle fails closed: {forged:?}"
    );
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
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
        "ethos-office-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}
