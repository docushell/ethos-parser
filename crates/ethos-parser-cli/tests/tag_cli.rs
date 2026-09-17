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

//! `ethos-parser tag` end to end: real process, real exit codes, real stdout (auto-tagging S2).
//!
//! The library tests hold the writer; these hold the shell around it — that the binary prints
//! exactly the bytes `write_tags` returns, that a refusal is exit 2 with the reason on stderr and
//! nothing on stdout, and that what it prints is a PDF rather than the canonical JSON every
//! other artifact-producing subcommand prints.

use std::path::PathBuf;
use std::process::{Command, Output};

use ethos_parser_core::Profile;
use ethos_parser_pdf::Document;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

/// An engine-owned CC0 fixture, for the behaviours the Ethos corpus does not carry.
fn engine_fixture(name: &str) -> PathBuf {
    let p = repo_root().join(format!("fixtures/engine/{name}/document.pdf"));
    assert!(p.is_file(), "fixture missing: {}", p.display());
    p
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn code(out: &Output) -> i32 {
    out.status
        .code()
        .expect("process exited normally, not by signal")
}

/// **The CLI's bytes are the library's** (the thin-shell proof `docs/PUBLIC-API.md` names).
#[test]
fn the_cli_bytes_equal_the_librarys() {
    let path = engine_fixture("leading-gap-two-blocks");
    let out = engine(&["tag", path.to_str().unwrap()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty(), "nothing on stderr on success");

    let profile = Profile::default();
    let doc = Document::open_bytes(&std::fs::read(&path).unwrap(), &profile).unwrap();
    let expected = ethos_parser_pdf::write_tags(&doc, &profile).expect("the library tags");
    assert!(
        out.stdout == expected,
        "the CLI printed {} byte(s) and the library returned {}",
        out.stdout.len(),
        expected.len()
    );
}

/// **stdout is a PDF, not JSON** — the second subcommand after `overlay` whose output is not a
/// canonical artifact, and the only one whose output is a document.
#[test]
fn stdout_is_a_pdf_and_not_json() {
    let path = engine_fixture("two-column-15-lines");
    let out = engine(&["tag", path.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    assert!(out.stdout.starts_with(b"%PDF"), "a PDF header");
    assert!(
        serde_json::from_slice::<serde_json::Value>(&out.stdout).is_err(),
        "and not JSON"
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("/StructTreeRoot"),
        "carrying the tree"
    );
}

/// **A tagged document exits 2 with the reason on stderr and nothing on stdout.** A partial
/// PDF on stdout would be a document nobody wrote.
#[test]
fn a_tagged_document_exits_two_with_the_reason_on_stderr() {
    let path = engine_fixture("tagged-structure-roles");
    let out = engine(&["tag", path.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(out.stdout.is_empty(), "nothing on stdout");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.starts_with(
            "engine: unsupported tagging: the catalog declares /StructTreeRoot: a written tag \
             fills absence only"
        ),
        "{stderr}"
    );
    assert!(stderr.trim_end().ends_with("[unsupported]"), "{stderr}");

    // An id in the content stream and no tree is refused the same way, and a missing file is
    // exit 2 too — could not read, kept apart from nothing by the code on stderr.
    let path = engine_fixture("untagged-mcid-no-tree");
    let out = engine(&["tag", path.to_str().unwrap()]);
    assert_eq!(code(&out), 2);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("carries marked-content id 0"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = engine(&["tag", "/nonexistent/document.pdf"]);
    assert_eq!(code(&out), 2);
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("[io]"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// **`--diagnostics` reports under the extract stage**, as `overlay` does, and changes nothing
/// on stdout.
#[test]
fn diagnostics_report_under_the_extract_stage_and_leave_stdout_alone() {
    let path = engine_fixture("leading-gap-two-blocks");
    let plain = engine(&["tag", path.to_str().unwrap()]);
    let with = engine(&["--diagnostics", "tag", path.to_str().unwrap()]);
    assert_eq!(code(&with), 0);
    assert!(plain.stdout == with.stdout, "stdout is the same bytes");
    let line = String::from_utf8_lossy(&with.stderr);
    let d: serde_json::Value = serde_json::from_str(line.trim()).expect("one JSON line");
    assert_eq!(d["stage"], "extract");
}
