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

//! `engine extract` reads an ODP, and the page it could have had for free stays out (v2-S7).
//!
//! The sixth sibling of `office_cli.rs`, proving the same claim once more: **one subcommand, one
//! artifact type, one serializer**. There is no `ethos.engine.odp.v0` and no `engine extract-odp`.
//!
//! What is new here is the refusal on the wire. Every earlier v2 format had to argue that its page
//! belonged to somebody else. A presentation does not: `<draw:page>` elements are listed and
//! ordered, and a `<style:master-page>` states paper. The artifact still carries `pages: []`, and
//! the locator that arrives carries no `page` field to put one in.

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
    repo_root().join("fixtures/office/presentation-pages/presentation.odp")
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
fn extract_reads_an_odp_into_the_same_artifact_type_a_pdf_produces() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");

    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON on stdout");
    assert_eq!(
        artifact["artifact_type"], "ethos.engine.representation.v0",
        "one artifact type — there is no `ethos.engine.odp.v0`"
    );
    let payload = &artifact["representation"];
    assert_eq!(
        payload["source"]["media_type"],
        "application/vnd.oasis.opendocument.presentation"
    );
    assert_eq!(
        payload["pages"],
        json!([]),
        "a `<draw:page>` is a part of the presentation's structure, not a page this engine measured"
    );
    assert_eq!(
        payload["tables"],
        json!([]),
        "and no detector ran, so there is no table record either"
    );
    assert!(
        payload["nodes"].as_array().expect("nodes").len() >= 6,
        "and that is not vacuous"
    );
}

/// A known title arrives at the address the file states, on the new locator.
#[test]
fn a_known_title_arrives_at_the_position_the_file_states() {
    let (_, stdout, _) = extract(&fixture());
    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    let nodes = artifact["representation"]["nodes"]
        .as_array()
        .expect("nodes");

    let title = nodes
        .iter()
        .find(|n| n["text"] == "The second draw page")
        .expect("the phrase is a title on the fixture's second page");
    let locator = &title["native_locator"]["odp"];
    assert_eq!(locator["part"], "content.xml");
    assert_eq!(locator["draw_page"], json!(2));
    assert_eq!(
        locator["shape"],
        json!(2),
        "a self-closing `<draw:frame/>` is shape 1, so this is shape 2: {locator:?}"
    );
    assert_eq!(locator["paragraph"], json!(1));
    assert!(
        locator.get("page").is_none()
            && locator.get("bbox").is_none()
            && locator.get("slide_number").is_none(),
        "and it carries no page, no box and no slide number: {locator:?}"
    );

    let attributes = &title["attributes"]["office_odf_shape"];
    assert_eq!(attributes["block"], "heading");
    assert_eq!(
        attributes["draw_page_name"], "Detail",
        "the file's own name is a label on the attributes, never the address: {attributes:?}"
    );
}

/// Dispatch is by content, never by extension.
#[test]
fn dispatch_is_by_content_not_by_extension() {
    let dir = tempdir();
    let renamed = dir.join("deck.bin");
    std::fs::copy(fixture(), &renamed).expect("copy the fixture");
    let (code, stdout, stderr) = extract(&renamed);
    assert_eq!(
        code, 0,
        "an extension is a claim anybody can make: {stderr}"
    );
    assert!(!stdout.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

/// And a package that is two formats at once is a **named refusal**, not a race between two `if`s.
#[test]
fn a_package_claiming_two_formats_is_refused_by_name() {
    let dir = tempdir();
    let path = dir.join("ambiguous.odp");
    std::fs::write(
        &path,
        build_zip(&[
            ("mimetype", "application/vnd.oasis.opendocument.presentation"),
            (
                "META-INF/manifest.xml",
                r#"<?xml version="1.0"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"><manifest:file-entry manifest:full-path="content.xml"/></manifest:manifest>"#,
            ),
            ("content.xml", r#"<?xml version="1.0"?><office/>"#),
            ("ppt/presentation.xml", "<p:presentation/>"),
        ]),
    )
    .expect("write the package");

    let (code, stdout, stderr) = extract(&path);
    assert_eq!(code, 2, "a file claiming to be two documents fails closed");
    assert!(stdout.is_empty(), "a refusal prints no artifact");
    assert!(
        stderr.contains("more than one kind") || stderr.contains("2 main parts"),
        "and it names the ambiguity rather than picking a winner: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn two_runs_over_one_presentation_produce_identical_bytes() {
    let (_, first, _) = extract(&fixture());
    let (_, second, _) = extract(&fixture());
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// The siblings that still have no reader
// -------------------------------------------------------------------------------------------

/// **A drawing shares the vocabulary and is still refused by name.**
///
/// The sharpest case in the ODF family and the reason [`engine_office::is_odp`] matches exactly
/// rather than by prefix: an `.odg`'s `content.xml` really is `<draw:page>`, so a prefix match
/// would have produced a *plausible* artifact for a format nobody decided to support. That is
/// worse than the `%PDF-` message v2-S6 fixed, because it does not look like a failure at all.
#[test]
fn an_unimplemented_opendocument_type_is_still_refused_by_name() {
    let dir = tempdir();
    for (extension, declared) in [
        ("odg", "application/vnd.oasis.opendocument.graphics"),
        (
            "otp",
            "application/vnd.oasis.opendocument.presentation-template",
        ),
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
            "and it does NOT name a PDF header: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The grounding boundary, unmoved
// -------------------------------------------------------------------------------------------

/// **(b) still holds.** `ethos.grounding.v1` is PDF-only, and a presentation is refused by name.
///
/// This passes with **no change to `engine-grounding`** — which matters more for this format than
/// for the five before it, because a presentation is the one whose own file could have supplied
/// the page the grounding schema requires.
#[test]
fn ground_refuses_a_page_less_presentation_by_name() {
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

/// A presentation block resolves through the same fingerprint-checked handle path a PDF run uses,
/// and a forged id fails closed. `mcp.rs` is **unchanged** by this slice.
#[test]
fn node_get_resolves_a_block_and_fails_closed_on_a_forged_id() {
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

    let locator = &ok["structuredContent"]["native_locator"]["odp"];
    assert_eq!(
        locator["part"], "content.xml",
        "the address that came back is the document's own: {locator:?}"
    );
    assert_eq!(locator["draw_page"], json!(1));
    assert_eq!(locator["shape"], json!(1));
    assert_eq!(locator["paragraph"], json!(1));
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
        "ethos-odp-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}
