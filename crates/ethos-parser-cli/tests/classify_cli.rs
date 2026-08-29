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

//! `ethos-parser classify` end to end: real process, real exit codes, real stdout.
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
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
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
            // v0.1 repairs the 19-byte xref class, so `table-regular-grid` moved OUT of this
            // list and into `the_repaired_xref_fixture_exits_zero_and_declares_it` below.
            // `corrupt-header-valid` takes its place: also a malformed structure, and NOT the
            // repairable class — which is the property worth guarding here.
            "corrupt xref",
            conformance("failure/corrupt-header-valid/document.pdf"),
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

/// **Every subcommand is implemented as of M6**, and none of them claims otherwise.
///
/// This test used to assert the opposite for whichever subcommands had not landed yet, and it
/// shrank by one row per milestone. It is inverted rather than deleted because the property it
/// guarded still matters in the other direction: a subcommand that prints usage and exits 0, or
/// that says "not implemented", would let a script conclude work happened that did not.
#[test]
fn no_subcommand_claims_to_be_unimplemented() {
    let pdf = conformance("synthetic/simple-text/document.pdf");
    for sub in ["classify", "extract", "ground", "grounding-check"] {
        let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
            .arg(sub)
            .arg(&pdf)
            .output()
            .expect("runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !stderr.contains("not implemented"),
            "`{sub}` still reports itself unimplemented: {stderr}"
        );

        // Handed a PDF, the two that expect JSON refuse it by name rather than by milestone.
        if matches!(sub, "ground" | "grounding-check") {
            assert_ne!(out.status.code(), Some(0), "`{sub}` must refuse a PDF");
            assert!(
                !stderr.is_empty() || !out.stdout.is_empty(),
                "`{sub}` must say something about why"
            );
        }
    }
}

/// `ground` is implemented, and refuses input that is not a representation.
///
/// The complement of the test above: an implemented subcommand still exits 2 on bad input, but
/// for a *named* reason rather than "not built yet". Handed a PDF where a representation belongs,
/// it says the representation is malformed instead of trying to make sense of the bytes.
#[test]
fn ground_is_implemented_and_refuses_a_non_representation() {
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("ground")
        .arg(conformance("synthetic/simple-text/document.pdf"))
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "no artifact may be emitted");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("malformed representation"),
        "the refusal must name what was wrong, not a milestone: {stderr}"
    );
    assert!(
        !stderr.contains("not implemented"),
        "`ground` lands at M5 and must stop claiming otherwise: {stderr}"
    );
}

#[test]
fn the_cli_output_matches_the_library() {
    // The CLI is a thin shell: its stdout must be exactly what the library canonicalizes.
    let path = conformance("synthetic/two-columns/document.pdf");
    let out = classify(&[path.to_str().unwrap()]);

    let profile = ethos_parser_core::Profile::default();
    let doc = ethos_parser_pdf::Document::open(&path, &profile).expect("opens");
    let c = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");
    let expected = c.to_canonical_bytes().expect("canonical");

    assert_eq!(
        out.stdout.trim_ascii_end(),
        expected.as_slice(),
        "CLI stdout must be the library's canonical bytes, with only a trailing newline added"
    );
}

/// Both subcommands put the assurance blocks on stdout.
///
/// The library builds them and a library test asserts their content; this asserts the *binary*
/// ships them. An artifact that reached L1 in memory and lost its declarations on the way to the
/// pipe has not reached L1 for the caller, who is the only one it matters to.
#[test]
fn both_subcommands_emit_the_assurance_blocks() {
    let path = conformance("synthetic/two-columns/document.pdf");
    let arg = path.to_str().unwrap();

    let runs = [
        ("classify", classify(&[arg])),
        (
            "extract",
            Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
                .arg("extract")
                .arg(arg)
                .output()
                .expect("the engine binary runs"),
        ),
    ];

    for (sub, out) in runs {
        let s = String::from_utf8(out.stdout).expect("canonical bytes are UTF-8");
        for required in [
            "\"assurance\"",
            "\"capabilities\"",
            "\"limitations\"",
            "\"coverage\"",
            "\"page_states\"",
            "\"terminal_state\"",
            // A profile-scope limitation on the wire, from the binary. This used to be
            // `multi-column-reading-order`, which v1-S5 retired from the default profile along
            // with the defect it described. The point of the assertion was never that *this*
            // code appears — it is that a capability's scope reaches stdout rather than living
            // only in the library — so it now names the limitation that partners the reading
            // -order capability instead of the one that partnered its absence.
            "reading-order-geometric-only",
        ] {
            assert!(
                s.contains(required),
                "`ethos-parser {sub}` stdout is missing {required}"
            );
        }
        // The absorbed stand-ins must not reappear alongside the block they became.
        assert!(
            !s.contains("\"not_detected\""),
            "{sub} carries a second vocabulary"
        );
        assert!(
            !s.contains("\"not_decoded\""),
            "{sub} carries a second vocabulary"
        );
    }
}

/// `extract` still exits 0, and the terminal state — not the exit code — carries completeness.
///
/// Exit-code meanings are unchanged at M4 (0 extracted, 2 could-not-read). The page budget that
/// can produce a partial artifact is a library/profile knob with no CLI flag, so the binary
/// cannot emit a partial artifact at v0 and the question of what exit code one deserves does not
/// arise yet. Pinned here so the answer is a decision on the record rather than an accident.
#[test]
fn extract_exits_zero_and_reports_completeness_on_the_artifact() {
    let path = conformance("synthetic/simple-text/document.pdf");
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("extract")
        .arg(path.to_str().unwrap())
        .output()
        .expect("the engine binary runs");

    assert_eq!(
        code(&out),
        0,
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8(out.stdout).expect("UTF-8");
    assert!(
        s.contains(r#""terminal_state":{"state":"complete"}"#),
        "a clean document reports Complete on the artifact: {s}"
    );
}
