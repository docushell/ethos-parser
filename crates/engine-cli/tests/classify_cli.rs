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

//! `engine classify` end to end: real process, real exit codes, real stdout.
//!
//! The library tests assert the mapping; these assert the *binary* honours it. A correct
//! `exit_code()` that the CLI forgets to return is still a caller reading the wrong signal.

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn manifest() -> serde_json::Value {
    serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest"),
    )
    .expect("valid JSON")
}

fn root(name: &str) -> PathBuf {
    let m = manifest();
    let decl = &m["roots"][name];
    if let Some(v) = decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        return PathBuf::from(v);
    }
    repo_root().join(decl["default"].as_str().expect("default"))
}

fn conformance(rel: &str) -> PathBuf {
    root("conformance").join(rel)
}
fn bench(rel: &str) -> PathBuf {
    root("benchmark").join(rel)
}

fn classify(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engine"))
        .arg("classify")
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn code(out: &Output) -> i32 {
    out.status
        .code()
        .expect("process exited normally, not by signal")
}

#[test]
fn exit_zero_on_a_document_with_no_reasons() {
    let out = classify(&[conformance("synthetic/simple-text/document.pdf")
        .to_str()
        .unwrap()]);
    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.stdout.is_empty(), "the artifact goes to stdout");
}

#[test]
fn exit_one_when_a_reason_fires() {
    // Layout reasons only — a table is not a reason to run OCR, but it is a reason to look.
    let out = classify(&[bench("irs-form-1040-2025.pdf").to_str().unwrap()]);
    assert_eq!(
        code(&out),
        1,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(v["needs_attention"], true);
    assert!(v["ocr_reasons"].as_array().expect("array").is_empty());
    assert!(!v["layout_reasons"].as_array().expect("array").is_empty());
}

#[test]
fn exit_two_for_every_could_not_read_case() {
    for (label, path) in [
        (
            "password-protected",
            conformance("failure/password-protected/document.pdf"),
        ),
        (
            "invalid-header",
            conformance("failure/invalid-header/document.pdf"),
        ),
        (
            "19-byte xref",
            conformance("synthetic/table-regular-grid/document.pdf"),
        ),
        ("missing file", repo_root().join("no/such/file.pdf")),
    ] {
        let out = classify(&[path.to_str().unwrap()]);
        assert_eq!(code(&out), 2, "{label} should exit 2");
        assert!(
            out.stdout.is_empty(),
            "{label}: no artifact may reach stdout when the document could not be read — a \
             partial artifact must not be mistaken for a real one"
        );
        assert!(
            !out.stderr.is_empty(),
            "{label}: the reason belongs on stderr"
        );
    }
}

#[test]
fn stdout_is_byte_identical_across_runs() {
    let path = bench("nist-sp-800-63b.pdf");
    let a = classify(&[path.to_str().unwrap()]);
    let b = classify(&[path.to_str().unwrap()]);
    assert_eq!(
        a.stdout, b.stdout,
        "default output must be fingerprint-stable"
    );
}

#[test]
fn stdout_carries_no_path_or_host_data() {
    let out = classify(&[conformance("synthetic/simple-text/document.pdf")
        .to_str()
        .unwrap()]);
    let s = String::from_utf8(out.stdout).expect("utf8");
    for forbidden in ["/Users", "/home", "document.pdf", "elapsed"] {
        assert!(!s.contains(forbidden), "`{forbidden}` leaked into stdout");
    }
}

#[test]
fn the_sample_flag_changes_the_profile_and_the_observation() {
    let path = bench("nist-sp-800-63b.pdf");
    let a = classify(&[path.to_str().unwrap(), "--sample-pages", "1"]);
    let b = classify(&[path.to_str().unwrap(), "--sample-pages", "8"]);

    let va: serde_json::Value = serde_json::from_slice(&a.stdout).expect("json");
    let vb: serde_json::Value = serde_json::from_slice(&b.stdout).expect("json");

    assert_eq!(va["pages_content_scanned"], 1);
    assert_eq!(vb["pages_content_scanned"], 8);
    assert_ne!(
        va["identity"]["profile_sha256"], vb["identity"]["profile_sha256"],
        "a different sample count is a different profile, so the two artifacts are correctly \
         non-comparable"
    );
}

/// Subcommands that do not exist yet exit 2, naming the milestone that owns them.
///
/// `extract` left this list at M3. Exiting 0 with usage text would let a script conclude the
/// work happened.
#[test]
fn unimplemented_subcommands_exit_two_rather_than_pretend() {
    for sub in ["ground", "grounding-check"] {
        let out = Command::new(env!("CARGO_BIN_EXE_engine"))
            .arg(sub)
            .arg(conformance("synthetic/simple-text/document.pdf"))
            .output()
            .expect("runs");
        assert_eq!(
            out.status.code(),
            Some(2),
            "`{sub}` is unimplemented; exiting 0 would let a script conclude the work happened"
        );
        assert!(out.stdout.is_empty(), "`{sub}` must emit no artifact");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains('M'),
            "`{sub}` should name the milestone that owns it; got: {stderr}"
        );
    }
}

#[test]
fn the_cli_output_matches_the_library() {
    // The CLI is a thin shell: its stdout must be exactly what the library canonicalizes.
    let path = conformance("synthetic/two-columns/document.pdf");
    let out = classify(&[path.to_str().unwrap()]);

    let profile = engine_core::Profile::default();
    let doc = engine_pdf::Document::open(&path, &profile).expect("opens");
    let c = engine_pdf::classify(&doc, &profile).expect("classifies");
    let expected = c.to_canonical_bytes().expect("canonical");

    assert_eq!(
        out.stdout.trim_ascii_end(),
        expected.as_slice(),
        "CLI stdout must be the library's canonical bytes, with only a trailing newline added"
    );
}
