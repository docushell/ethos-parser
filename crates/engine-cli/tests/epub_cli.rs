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

//! `engine extract` reads an EPUB, and the page a publisher named stays out (v2-S9).
//!
//! The eighth sibling of `office_cli.rs`, proving the same claim once more: **one subcommand, one
//! artifact type, one serializer**. There is no `ethos.engine.epub.v0` and no `engine extract-epub`.
//!
//! **The CLI's dispatch did not change for this slice**, and that is worth a test rather than a
//! shrug: v2-S8 made the router's last line the *container* question, so every ZIP already reached
//! `engine_office::read`. This file pins that an EPUB gets there, that it is still never told it is
//! OpenDocument (v2-S6's pin, whose home moved here from `rtf_cli.rs`), and that a `.csv` still
//! does not — because that message belongs to S10.

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
    repo_root().join("fixtures/office/book-spine/book.epub")
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
fn extract_reads_an_epub_into_the_same_artifact_type_a_pdf_produces() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");

    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON on stdout");
    assert_eq!(
        artifact["artifact_type"], "ethos.engine.representation.v0",
        "one artifact type — there is no `ethos.engine.epub.v0`"
    );
    let payload = &artifact["representation"];
    assert_eq!(payload["source"]["media_type"], "application/epub+zip");
    assert_eq!(
        payload["pages"],
        json!([]),
        "a `page-list` names the pages of a print edition this engine never measured"
    );
    assert_eq!(
        payload["tables"],
        json!([]),
        "and a cell's text is read as the block it is, not as a grid a detector inferred"
    );
    assert_eq!(
        payload["processing_run"]["reading_order_rule"],
        "epub-spine-then-document-order-v1"
    );
    assert!(
        payload["nodes"].as_array().expect("nodes").len() >= 9,
        "and that is not vacuous"
    );
}

/// A known phrase arrives at the address the package states, on the new locator.
#[test]
fn a_known_block_arrives_at_the_position_the_package_states() {
    let (_, stdout, _) = extract(&fixture());
    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    let nodes = artifact["representation"]["nodes"]
        .as_array()
        .expect("nodes");

    let node = nodes
        .iter()
        .find(|n| n["text"] == "The second spine item")
        .expect("the phrase is a heading in the fixture's second spine document");
    let locator = &node["native_locator"]["epub"];
    assert_eq!(
        locator["part"], "OEBPS/aa-second.xhtml",
        "the part is resolved from the manifest, relative to the package document's directory"
    );
    assert_eq!(locator["block"], json!(2));
    assert!(
        locator.get("page").is_none() && locator.get("bbox").is_none(),
        "and it carries no page and no box: {locator:?}"
    );
    assert_eq!(node["attributes"]["epub_block"]["element"], "h1");
    assert_eq!(node["attributes"]["epub_block"]["linear"], json!(true));
}

/// **The spine states the order, and the archive does not** — visible on the wire.
#[test]
fn the_nodes_arrive_in_spine_order_not_archive_order() {
    let (_, stdout, _) = extract(&fixture());
    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    let mut seen: Vec<String> = Vec::new();
    for node in artifact["representation"]["nodes"]
        .as_array()
        .expect("nodes")
    {
        let part = node["native_locator"]["epub"]["part"]
            .as_str()
            .expect("a part")
            .to_string();
        if seen.last() != Some(&part) {
            seen.push(part);
        }
    }
    assert_eq!(
        seen,
        vec![
            "OEBPS/zz-first.xhtml".to_string(),
            "OEBPS/aa-second.xhtml".to_string(),
            "OEBPS/notes.xhtml".to_string()
        ],
        "the archive stores `aa-second` first and the spine lists `zz-first` first"
    );
}

/// Dispatch is by content, never by extension.
#[test]
fn dispatch_is_by_content_not_by_extension() {
    let dir = tempdir();
    let renamed = dir.join("book.bin");
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
fn two_runs_over_one_publication_produce_identical_bytes() {
    let (_, first, _) = extract(&fixture());
    let (_, second, _) = extract(&fixture());
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// The pins this slice inherited, and the one format it did not implement
// -------------------------------------------------------------------------------------------

/// **An EPUB is never told it is OpenDocument**, now that it reads.
///
/// v2-S6 wrote `is_opendocument` to answer on the *declared type* rather than on the presence of a
/// first, stored `mimetype` entry — precisely so this format would not arrive to be told it is
/// something it is not. That pin lived in `rtf_cli.rs` until v2-S9 gave EPUB a reader; it lives
/// here now, where the format it protects is.
#[test]
fn an_epub_reads_and_is_still_not_opendocument() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");
    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    assert_eq!(
        artifact["representation"]["source"]["media_type"], "application/epub+zip",
        "it is read as what it declares, and what it declares is not OpenDocument"
    );

    // And an `.odg` — a genuine ODF sibling with no reader — still takes the ODF refusal.
    let dir = tempdir();
    let odg = dir.join("drawing.odg");
    std::fs::write(
        &odg,
        ocf_package("application/vnd.oasis.opendocument.graphics"),
    )
    .expect("write the package");
    let (code, _, stderr) = extract(&odg);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("OpenDocument") && stderr.contains("graphics"),
        "{stderr}"
    );
    assert!(!stderr.contains("%PDF-"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// **CSV is still S10**, and this slice did not sniff commas to change that.
#[test]
fn a_csv_still_does_not_extract() {
    let dir = tempdir();
    let csv = dir.join("rows.csv");
    std::fs::write(&csv, b"a,b,c\n1,2,3\n").expect("write the file");
    let (code, stdout, _) = extract(&csv);
    assert_eq!(
        code, 2,
        "comma-separated text cannot be told from prose without a reader, and a detector that \
         guessed would claim every comma file"
    );
    assert!(stdout.is_empty(), "a refusal prints no artifact");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A ZIP that is no format this engine reads still names the **container**, not a PDF header.
#[test]
fn an_unread_zip_is_still_refused_by_container_rather_than_as_a_missing_pdf() {
    let dir = tempdir();
    let path = dir.join("mystery.zip");
    std::fs::write(&path, ocf_package("application/x-nothing-we-read")).expect("write");
    let (code, stdout, stderr) = extract(&path);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(!stderr.contains("%PDF-"), "{stderr}");
    assert!(
        stderr.contains("application/epub+zip"),
        "and the refusal now names EPUB among the formats it is not: {stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The grounding boundary, unmoved
// -------------------------------------------------------------------------------------------

/// **(b) still holds.** `ethos.grounding.v1` is PDF-only, and an EPUB is refused by name.
///
/// This passes with **no change to `engine-grounding`** — which is worth saying for this format
/// above all the others, because an EPUB is the first one whose own file could have supplied the
/// page the grounding schema requires.
#[test]
fn ground_refuses_a_page_less_publication_by_name() {
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

/// An EPUB block resolves through the same fingerprint-checked handle path a PDF run uses, and a
/// forged id fails closed. `mcp.rs` is **unchanged** by this slice.
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

    let locator = &ok["structuredContent"]["native_locator"]["epub"];
    assert_eq!(
        locator["part"], "OEBPS/zz-first.xhtml",
        "the address that came back is the package's own: {locator:?}"
    );
    assert_eq!(locator["block"], json!(2));
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

/// A minimal OCF-shaped package declaring `media_type`: `mimetype` first and **stored**.
///
/// The container rule an ODF package and an `.epub` share, which is exactly why v2-S6 pinned that
/// the ODF family question asks the declared **type** rather than this entry's presence.
fn ocf_package(media_type: &str) -> Vec<u8> {
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
        "ethos-epub-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}
