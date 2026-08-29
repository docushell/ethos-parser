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

//! `ethos-parser extract` reads a PPTX, and every path that would have needed a page refuses (v2-S3).
//!
//! The sibling of `office_cli.rs`, and it exists to prove the same claim one format further
//! along: **one subcommand, one artifact type, one serializer**. There is no
//! `ethos.parser.pptx.v0` and no `ethos-parser extract-pptx`; dispatch is by content; and `node_get`
//! resolves a *slide run* over an MCP server that was never taught what a deck is.

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
    repo_root().join("fixtures/office/deck-slides/deck.pptx")
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
fn extract_reads_a_pptx_into_the_same_artifact_type_a_pdf_produces() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");

    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    assert_eq!(artifact["artifact_type"], "ethos.parser.representation.v0");
    assert!(
        artifact["representation"]["pages"]
            .as_array()
            .expect("pages")
            .is_empty(),
        "a slide is a part, not a page, so there are none to declare"
    );
    // Spelled out rather than taken from the constant, so a rename of the constant cannot
    // silently change what the CLI puts on the wire.
    assert_eq!(
        artifact["representation"]["source"]["media_type"],
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    );
    assert!(!artifact["representation"]["nodes"]
        .as_array()
        .expect("nodes")
        .is_empty());
}

/// **A4 through the CLI.** The bytes decide, in both directions.
#[test]
fn a_renamed_deck_still_reads_and_a_renamed_pdf_does_not_become_one() {
    let dir = tempdir();

    let renamed = dir.join("report.bin");
    std::fs::copy(fixture(), &renamed).expect("copy the fixture");
    let (code, stdout, stderr) = extract(&renamed);
    assert_eq!(
        code, 0,
        "an extension is a claim anybody can make: {stderr}"
    );
    assert!(!stdout.is_empty());

    let liar = dir.join("not-really.pptx");
    std::fs::write(&liar, b"%PDF-1.7\nnot a package at all").expect("write");
    let (code, stdout, stderr) = extract(&liar);
    assert_eq!(code, 2, "and a magic number is one only the file can make");
    assert!(stdout.is_empty(), "a failed read prints no artifact");
    assert!(!stderr.is_empty());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_runs_over_one_deck_produce_identical_bytes() {
    let (_, first, _) = extract(&fixture());
    let (_, second, _) = extract(&fixture());
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// The grounding boundary, unmoved
// -------------------------------------------------------------------------------------------

/// **(b) still holds.** `ethos.grounding.v1` is PDF-only, and a deck is refused by name.
///
/// This passes with **no change to `ethos-parser-grounding`**: `project` refuses on the source media
/// type, which a PPTX artifact fails for the same reason a DOCX one does.
#[test]
fn ground_projects_a_page_less_deck_by_name() {
    let dir = tempdir();
    let (_, stdout, _) = extract(&fixture());
    let path = dir.join("representation.json");
    std::fs::write(&path, &stdout).expect("write the artifact");

    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["ground", path.to_str().expect("utf-8 path")])
        .output()
        .expect("the engine binary runs");

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
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The handle law, over unmodified MCP
// -------------------------------------------------------------------------------------------

/// A slide run resolves through the same fingerprint-checked handle path a PDF run uses, and a
/// forged
/// id fails closed. `mcp.rs` is **unchanged** by this slice.
#[test]
fn node_get_resolves_a_slide_run_and_fails_closed_on_a_forged_id() {
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

    let locator = &ok["structuredContent"]["native_locator"]["pptx"];
    assert_eq!(
        locator["part"], "ppt/slides/slide1.xml",
        "the address that came back is the deck's own: {locator:?}"
    );
    assert_eq!(locator["shape"], json!(1));
    assert_eq!(locator["paragraph"], json!(1));
    assert_eq!(locator["run"], json!(1));
    assert!(
        locator.get("page").is_none()
            && locator.get("bbox").is_none()
            && locator.get("slide").is_none(),
        "and it carries no page, no box and no slide number: {locator:?}"
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
        "ethos-deck-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}
