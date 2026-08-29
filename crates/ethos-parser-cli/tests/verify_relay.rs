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

//! `ethos-parser verify` — relaying, not verifying (`docs/07-VERIFY-BOUNDARY.md` Stage 1).
//!
//! Four properties, and the third is the one the whole design rests on:
//!
//! 1. **Absence is loud.** No verifier ⇒ exit 2, no report, a named error. Never a skip, never a
//!    stub, never a default-pass.
//! 2. **The gate works.** An ungrounded claim with `--fail-on-ungrounded` exits 1 *and* writes
//!    the report. Without the flag the report is still written — never a silent skip.
//! 3. **The bytes are the verifier's.** `ethos-parser verify` stdout is byte-identical to running
//!    `ethos verify` with the same arguments. Not "equivalent", not "the same fields" —
//!    identical, because the engine forwards bytes and forms no opinion of its own.
//! 4. **Two runs agree**, on both sides, so relaying introduces no volatility.
//!
//! # The oracle is required here too
//!
//! These tests spawn the same binary the M6 oracle harness spawns, resolved the same way. Its
//! absence fails them loudly rather than skipping — a verify suite that quietly passed on a
//! machine with no verifier would be asserting nothing at all.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

// -------------------------------------------------------------------------------------------
// Harness
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

/// The verifier, resolved exactly as the oracle harness resolves it.
fn ethos_binary() -> PathBuf {
    if let Some(v) = std::env::var_os("ETHOS_BIN") {
        let p = PathBuf::from(&v);
        assert!(
            p.is_file(),
            "ETHOS_BIN is set to `{}`, which is not a file. An explicit pin is authoritative.",
            p.display()
        );
        return p;
    }
    let sibling = repo_root().join("../ethos/target/release/ethos");
    if sibling.is_file() {
        return sibling;
    }
    if let Ok(out) = Command::new("sh")
        .arg("-c")
        .arg("command -v ethos")
        .output()
    {
        let found = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !found.is_empty() {
            return PathBuf::from(found);
        }
    }
    panic!(
        "the Ethos CLI could not be located. It is a test-time dependency and its absence is a \
         FAILURE, never a skip (docs/04-ARCHITECTURE.md §4). Build it with `cargo build \
         --release` in the Ethos repo, or set ETHOS_BIN."
    );
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn engine_without_verifier(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        // Injected rather than uninstalling anything: the developer's own binary is untouched,
        // and the pin being authoritative means this cannot silently fall through to it.
        .env("ETHOS_BIN", "/nonexistent/verifier-for-this-test")
        .output()
        .expect("the engine binary runs")
}

fn ethos(args: &[&str]) -> Output {
    Command::new(ethos_binary())
        .args(args)
        .output()
        .expect("the verifier runs")
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-v01-verify-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A grounding artifact plus matching citations, grounded and not.
struct Case {
    grounding: PathBuf,
    grounded: PathBuf,
    ungrounded: PathBuf,
}

/// Build a grounding artifact from an engine-owned fixture and two citations files for it.
///
/// **The `document_fingerprint` is the sha256 of the grounding file's raw bytes.** Measured
/// against the pinned verifier, not remembered: it treats a grounding-JSON source's fingerprint
/// as the digest of the file it loaded, and a citations envelope naming anything else — the
/// PDF's own digest, for instance — comes back `stale_fingerprint` rather than grounded. That is
/// the same digest M6 established as `representation_sha256`.
fn build(dir: &Path) -> Case {
    let pdf = repo_root().join("fixtures/engine/measured-ink-box/document.pdf");
    assert!(pdf.is_file(), "the engine-owned fixture must exist");

    let repr = engine(&["extract", pdf.to_str().unwrap()]);
    assert_eq!(
        repr.status.code(),
        Some(0),
        "extract must succeed to set up"
    );
    let repr_path = dir.join("repr.json");
    std::fs::write(&repr_path, &repr.stdout).expect("write representation");

    let grounded_out = engine(&["ground", repr_path.to_str().unwrap()]);
    assert_eq!(grounded_out.status.code(), Some(0), "ground must succeed");
    let grounding = dir.join("grounding.json");
    std::fs::write(&grounding, &grounded_out.stdout).expect("write grounding");

    let bytes = std::fs::read(&grounding).expect("readable");
    let fingerprint = format!("sha256:{}", ethos_parser_core::sha256_hex_bytes(&bytes));

    let write_claims = |name: &str, text: &str| -> PathBuf {
        let p = dir.join(name);
        let body = serde_json::json!({
            "document_fingerprint": fingerprint,
            "claims": [{
                "kind": "quote",
                "text": text,
                "citation": { "page": "p1", "element_id": "e1" }
            }]
        });
        std::fs::write(&p, serde_json::to_vec_pretty(&body).unwrap()).expect("write claims");
        p
    };

    Case {
        grounding,
        // "Measured" is the fixture's only text, so this claim is genuinely supported.
        grounded: write_claims("grounded.json", "Measured"),
        ungrounded: write_claims(
            "ungrounded.json",
            "a sentence this document does not contain",
        ),
    }
}

/// The arguments the engine forwards, so a direct comparison uses the same ones.
fn oracle_args<'a>(case: &'a Case, citations: &'a Path, gate: bool) -> Vec<&'a str> {
    let mut v = vec![
        "verify",
        case.grounding.to_str().unwrap(),
        "--citations",
        citations.to_str().unwrap(),
        "--grounding",
        "ethos-grounding-json",
    ];
    if gate {
        v.push("--fail-on-ungrounded");
    }
    v
}

fn engine_args<'a>(case: &'a Case, citations: &'a Path, gate: bool) -> Vec<&'a str> {
    let mut v = vec![
        "verify",
        case.grounding.to_str().unwrap(),
        "--citations",
        citations.to_str().unwrap(),
    ];
    if gate {
        v.push("--fail-on-ungrounded");
    }
    v
}

// -------------------------------------------------------------------------------------------
// 1. Absence is loud
// -------------------------------------------------------------------------------------------

/// **No verifier ⇒ exit 2, no report, a named error.**
#[test]
fn a_missing_verifier_is_a_named_failure_with_no_report() {
    let dir = scratch("missing");
    let case = build(&dir);

    let out = engine_without_verifier(&engine_args(&case, &case.grounded, true));

    assert_eq!(
        out.status.code(),
        Some(2),
        "a run that did not happen exits 2, not 1 — a caller must be able to tell 'the check \
         failed' from 'the check did not run'"
    );
    assert!(
        out.stdout.is_empty(),
        "no report may reach stdout when no verifier ran. A report here would be one the engine \
         invented, which is the single outcome this subcommand exists to make impossible. Got: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("missing required part") && stderr.contains("named explicitly"),
        "the failure must be named and explain itself: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// 2. The bytes are the verifier's
// -------------------------------------------------------------------------------------------

/// **A grounded claim exits 0, and stdout is byte-identical to the verifier's own.**
///
/// The strongest statement available about a relay: not "the same fields", not "equivalent" —
/// the same bytes. Anything the engine added, dropped, reordered or re-encoded would show here.
#[test]
fn a_grounded_claim_relays_the_verifier_bytes_verbatim() {
    let dir = scratch("grounded");
    let case = build(&dir);

    let mine = engine(&engine_args(&case, &case.grounded, false));
    let theirs = ethos(&oracle_args(&case, &case.grounded, false));

    assert_eq!(mine.status.code(), Some(0), "a grounded claim exits 0");
    assert_eq!(theirs.status.code(), Some(0));
    assert_eq!(
        mine.stdout, theirs.stdout,
        "the relayed report must be the verifier's bytes exactly"
    );
    assert!(
        !mine.stdout.is_empty(),
        "and there must actually be a report"
    );

    // The report really is a verifier's, and really did ground the claim — otherwise this test
    // would pass just as well on two identically-empty outputs.
    let v: Value = serde_json::from_slice(&mine.stdout).expect("the report is JSON");
    assert_eq!(
        v["all_evidence_grounded"], true,
        "the fixture's own text must actually ground: {v}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Two runs on each side agree, so relaying introduces no volatility of its own.
#[test]
fn the_relayed_report_is_byte_identical_across_two_runs() {
    let dir = scratch("double");
    let case = build(&dir);

    let a = engine(&engine_args(&case, &case.grounded, false));
    let b = engine(&engine_args(&case, &case.grounded, false));
    assert_eq!(a.stdout, b.stdout, "two engine runs must agree");

    let c = ethos(&oracle_args(&case, &case.grounded, false));
    let d = ethos(&oracle_args(&case, &case.grounded, false));
    assert_eq!(c.stdout, d.stdout, "two verifier runs must agree");
    assert_eq!(
        a.stdout, c.stdout,
        "and the two sides must agree with each other"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// 3. The gate
// -------------------------------------------------------------------------------------------

/// **An ungrounded claim with `--fail-on-ungrounded` exits 1 and still writes the report.**
///
/// The product gate. Exit 1 rather than 0 is what lets a shell predicate refuse to ship, and the
/// report is still produced because a non-zero exit with no explanation is a caller guessing.
#[test]
fn an_ungrounded_claim_under_the_gate_exits_one_with_a_real_report() {
    let dir = scratch("gate");
    let case = build(&dir);

    let mine = engine(&engine_args(&case, &case.ungrounded, true));
    let theirs = ethos(&oracle_args(&case, &case.ungrounded, true));

    assert_eq!(mine.status.code(), Some(1), "the gate must fire");
    assert_eq!(
        theirs.status.code(),
        Some(1),
        "and the verifier agrees it should"
    );
    assert_eq!(
        mine.stdout, theirs.stdout,
        "the report is still relayed verbatim on the failing path"
    );

    let v: Value = serde_json::from_slice(&mine.stdout).expect("a real report, not an invention");
    assert_eq!(v["all_evidence_grounded"], false);
    assert!(
        v["checks"].as_array().is_some_and(|c| !c.is_empty()),
        "the report carries the verifier's own checks: {v}"
    );

    // And nothing was added. Every key at the top level is one the verifier itself wrote.
    let theirs_v: Value = serde_json::from_slice(&theirs.stdout).unwrap();
    assert_eq!(v, theirs_v, "the engine invented no field and dropped none");

    let _ = std::fs::remove_dir_all(&dir);
}

/// **Without the flag, an ungrounded claim is still reported** — never a silent skip.
#[test]
fn an_ungrounded_claim_without_the_gate_still_writes_its_report() {
    let dir = scratch("nogate");
    let case = build(&dir);

    let mine = engine(&engine_args(&case, &case.ungrounded, false));
    let theirs = ethos(&oracle_args(&case, &case.ungrounded, false));

    assert_eq!(
        mine.status.code(),
        theirs.status.code(),
        "the engine forwards the verifier's own exit status unchanged"
    );
    assert!(
        !mine.stdout.is_empty(),
        "the report is written whether or not the gate was asked for — an ungrounded claim is \
         never a silent skip"
    );
    assert_eq!(mine.stdout, theirs.stdout);

    let v: Value = serde_json::from_slice(&mine.stdout).unwrap();
    assert_eq!(
        v["all_evidence_grounded"], false,
        "the report says what it found even when the exit code does not"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// 4. The library reaches it, and the shim stays a shim
// -------------------------------------------------------------------------------------------

/// **Reachable through the library**, as the thin-shell rule requires of every subcommand.
///
/// `docs/PUBLIC-API.md`'s thin-shell mapping: what the binary does, an embedding caller can do
/// without a process boundary of its own. That table carries **no row for `verify`** — it names
/// `verify` among the subcommands whose mapping is stated nowhere — so this test is the
/// library-only proof the missing row would cite.
///
/// **No ordinal, on v2-S15's rule.** This said *"like the other four subcommands"* until v2-S17,
/// and it was wrong under either reading: there are **nine** subcommands, so not four others, and
/// `verify` is not among the four the table does map, so it could not have been naming those
/// either. Raising the count to eight would have been a new false statement rather than a repair:
/// `mcp` is a server loop rather than a document pass, `ethos-parser-cli` publishes no library target,
/// and `PUBLIC-API.md` records its mapping as stated nowhere too. Whether `mcp` is *reachable* in
/// the sense this sentence claims is exactly the question that document defers, and a count
/// asserted here would answer it by accident. A form with no count in it cannot be re-rotted by a
/// tenth subcommand.
#[test]
fn the_relay_is_reachable_from_the_library() {
    use ethos_parser_core::verifier::{relay, RelayRequest, VerifierBinary};

    let dir = scratch("library");
    let case = build(&dir);

    let binary = VerifierBinary::resolve(
        std::env::var_os("ETHOS_BIN").map(PathBuf::from).as_deref(),
        Some(&repo_root()),
    )
    .expect("the verifier resolves the same way the harness resolves it");

    assert!(
        binary.version().starts_with("ethos"),
        "a pinned binary reports its version: {}",
        binary.version()
    );
    assert!(binary.pin().is_pinned());

    let relayed = relay(
        &binary,
        &RelayRequest {
            grounding: &case.grounding,
            citations: &case.grounded,
            adapter: ethos_parser_core::GROUNDING_ADAPTER,
            fail_on_ungrounded: true,
            config: None,
            out: None,
        },
    )
    .expect("the relay runs");

    assert_eq!(relayed.exit, ethos_parser_core::RELAY_OK);

    // Same bytes the binary produced, so the library path and the CLI path are one path.
    let via_cli = engine(&engine_args(&case, &case.grounded, true));
    assert_eq!(
        relayed.stdout, via_cli.stdout,
        "the library and the CLI must relay the same bytes"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The verifier's identity is pinned by **version and digest**, and both move the profile hash.
#[test]
fn pinning_a_different_verifier_moves_the_profile_hash() {
    use ethos_parser_core::{Profile, Sha256Hex, VerifierPin};

    let base = Profile::default();
    let unpinned = base.profile_sha256().expect("hashes");
    assert_eq!(base.verifier, VerifierPin::NotPinned);

    let with_a = Profile {
        verifier: VerifierPin::pinned(
            "ethos 0.6.0",
            Sha256Hex::from_hex(&"aa".repeat(32)).unwrap(),
        ),
        ..Profile::default()
    };
    let with_b = Profile {
        verifier: VerifierPin::pinned(
            "ethos 0.6.0",
            Sha256Hex::from_hex(&"bb".repeat(32)).unwrap(),
        ),
        ..Profile::default()
    };
    let with_c = Profile {
        verifier: VerifierPin::pinned(
            "ethos 0.7.0",
            Sha256Hex::from_hex(&"aa".repeat(32)).unwrap(),
        ),
        ..Profile::default()
    };

    let a = with_a.profile_sha256().unwrap();
    let b = with_b.profile_sha256().unwrap();
    let c = with_c.profile_sha256().unwrap();

    assert_ne!(unpinned, a, "pinning a verifier is an identity event");
    assert_ne!(
        a, b,
        "two builds of the SAME version must be distinguishable — a version-only pin would not \
         make a rebuild visible"
    );
    assert_ne!(a, c, "and a version change moves it too");
}

/// The engine relays the verifier's own usage refusals rather than papering over them.
///
/// A wrong input is the verifier's answer to give. What matters is that the engine does not turn
/// it into a report, and does not turn it into success.
#[test]
fn a_verifier_usage_refusal_is_relayed_as_could_not_run() {
    let dir = scratch("usage");
    let case = build(&dir);

    // A representation, not a grounding artifact: the adapter the engine declares will refuse it.
    let wrong_input = dir.join("repr.json");
    let out = engine(&[
        "verify",
        wrong_input.to_str().unwrap(),
        "--citations",
        case.grounded.to_str().unwrap(),
    ]);

    assert_eq!(
        out.status.code(),
        Some(2),
        "a usage refusal is 'the check did not run', not 'the check failed'"
    );
    assert!(
        out.stdout.is_empty(),
        "and it produces no report to mistake for one"
    );
    assert!(
        !out.stderr.is_empty(),
        "the verifier's own explanation is forwarded on stderr"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
