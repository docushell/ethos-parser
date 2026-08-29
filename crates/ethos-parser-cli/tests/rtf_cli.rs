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

//! `ethos-parser extract` reads an RTF, and the format with no container still binds (v2-S8).
//!
//! The seventh sibling of `office_cli.rs`, proving the same claim once more: **one subcommand, one
//! artifact type, one serializer**. There is no `ethos.parser.rtf.v0` and no `ethos-parser extract-rtf`.
//!
//! What is new is the dispatch. Every earlier line of that router asks a *container* a question,
//! and before this slice an `.rtf` answered `false` to all of them and fell through to the PDF
//! reader — to be refused for having no `%PDF-` header, which is the same wrong-cause defect
//! v2-S5 recorded for an `.ods` and v2-S6 fixed for the ODF family.

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
    repo_root().join("fixtures/office/rich-text-paragraphs/document.rtf")
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
fn extract_reads_an_rtf_into_the_same_artifact_type_a_pdf_produces() {
    let (code, stdout, stderr) = extract(&fixture());
    assert_eq!(code, 0, "{stderr}");

    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON on stdout");
    assert_eq!(
        artifact["artifact_type"], "ethos.parser.representation.v0",
        "one artifact type — there is no `ethos.parser.rtf.v0`"
    );
    let payload = &artifact["representation"];
    assert_eq!(payload["source"]["media_type"], "application/rtf");
    assert_eq!(
        payload["pages"],
        json!([]),
        "`\\page` is where the producer broke a page, not a page this engine measured"
    );
    assert_eq!(
        payload["tables"],
        json!([]),
        "and `\\cell` is a terminator the file writes, not a grid a detector inferred"
    );
    assert!(
        payload["nodes"].as_array().expect("nodes").len() >= 6,
        "and that is not vacuous"
    );
}

/// A known phrase arrives at the address the stream states, on the new locator.
#[test]
fn a_known_paragraph_arrives_at_the_position_the_stream_states() {
    let (_, stdout, _) = extract(&fixture());
    let artifact: Value = serde_json::from_slice(&stdout).expect("canonical JSON");
    let nodes = artifact["representation"]["nodes"]
        .as_array()
        .expect("nodes");

    let node = nodes
        .iter()
        .find(|n| n["text"] == "Split by the producer.")
        .expect("the phrase follows the fixture's own `\\page`");
    let locator = &node["native_locator"]["rtf"];
    assert_eq!(locator["paragraph"], json!(4));
    assert!(
        locator.get("page").is_none()
            && locator.get("bbox").is_none()
            && locator.get("part").is_none(),
        "no page, no box, and no part invented for a format that has none: {locator:?}"
    );
    assert_eq!(
        node["attributes"]["rtf_paragraph"]["terminator"], "paragraph",
        "which control word ended it is a fact the stream states"
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
fn two_runs_over_one_stream_produce_identical_bytes() {
    let (_, first, _) = extract(&fixture());
    let (_, second, _) = extract(&fixture());
    assert_eq!(first, second);
}

// -------------------------------------------------------------------------------------------
// What is still not read, and how it fails
// -------------------------------------------------------------------------------------------

/// **A near miss is not claimed, and an OLE `.doc` least of all.**
///
/// `{\rtf` is the whole of detection, so the cases worth pinning are the ones that look close: a
/// bare open brace, a different first control word, and the legacy `.doc` a caller is most likely
/// to hand over by mistake — an OLE compound file, which begins `D0 CF 11 E0`.
#[test]
fn a_near_miss_is_refused_without_naming_a_pdf_header() {
    let dir = tempdir();
    for (name, bytes) in [
        ("brace.rtf", b"{ not rtf }".to_vec()),
        ("other-word.rtf", br"{\ansi text}".to_vec()),
        (
            "legacy.doc",
            vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0, 0, 0, 0],
        ),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write the file");
        let (code, stdout, stderr) = extract(&path);
        assert_eq!(code, 2, "{name} still fails closed: {stderr}");
        assert!(stdout.is_empty(), "a refusal prints no artifact: {name}");
        assert!(
            !stderr.contains("Rich Text"),
            "{name} was not claimed as RTF: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// **CSV is refused**, and it is not told it is something it is not.
///
/// **The EPUB half of this test moved to `epub_cli.rs` at v2-S9**, which reads it. What stays is
/// the format nothing in this engine speaks: a `.csv` cannot be told from prose without a reader,
/// so it takes the true-unknown-bytes path — argued in `docs/15-V2-MILESTONES.md` S10 rather than
/// guessed at here, because a detector that sniffed commas would claim every comma file.
///
/// **Strengthened at v2-S10, not replaced.** The CSV half asserted exit 2 and empty stdout and
/// nothing about stderr, so it would have stayed green straight through the message change it was
/// written to notice — the same trap as `epub_cli.rs`'s. The `.odg` half beside it already
/// asserted its message, which is what made the gap visible.
#[test]
fn the_formats_this_slice_did_not_implement_are_still_refused() {
    let dir = tempdir();

    let csv = dir.join("rows.csv");
    std::fs::write(&csv, b"a,b,c\n1,2,3\n").expect("write the file");
    let (code, stdout, stderr) = extract(&csv);
    assert_eq!(code, 2, "a CSV has no reader yet");
    assert!(stdout.is_empty());
    assert!(
        stderr.contains("these bytes state no format this engine reads"),
        "v2-S10 gave the true-unknown-bytes path its own cause: {stderr}"
    );
    assert!(
        !stderr.contains("%PDF-"),
        "and it is no longer a missing PDF header: {stderr}"
    );

    // And an `.odg` is still refused as OpenDocument, by name — untouched by this slice.
    let odg = dir.join("drawing.odg");
    std::fs::write(
        &odg,
        ocf_package("application/vnd.oasis.opendocument.graphics"),
    )
    .expect("write the package");
    let (_, _, stderr) = extract(&odg);
    assert!(stderr.contains("OpenDocument"), "{stderr}");
    assert!(stderr.contains("graphics"), "{stderr}");
    assert!(!stderr.contains("%PDF-"), "{stderr}");

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The grounding boundary, unmoved
// -------------------------------------------------------------------------------------------

/// **(a) taken at 0.39.0.** `ethos.grounding.v1` 1.1.0 carries the page-less shape, and an RTF
/// stream projects into it — the format with no parts at all, whose locator is the plainest of
/// the eight.
#[test]
fn ground_projects_a_page_less_stream() {
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
    assert_eq!(artifact["pages"].as_array().expect("pages").len(), 0);
    let first = &artifact["elements"][0];
    assert!(
        first["locator"].as_str().is_some_and(|l| !l.is_empty()),
        "the native address travels on the element: {first}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The handle law, over unmodified MCP
// -------------------------------------------------------------------------------------------

/// An RTF paragraph resolves through the same fingerprint-checked handle path a PDF run uses, and
/// a forged id fails closed. `mcp.rs` is **unchanged** by this slice.
#[test]
fn node_get_resolves_a_paragraph_and_fails_closed_on_a_forged_id() {
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

    let locator = &ok["structuredContent"]["native_locator"]["rtf"];
    assert_eq!(
        locator["paragraph"],
        json!(1),
        "the address that came back is the stream's own: {locator:?}"
    );
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
        "ethos-rtf-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}
