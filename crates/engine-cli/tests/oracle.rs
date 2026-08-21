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

//! The oracle harness.
//!
//! `ethos grounding check` is the external oracle for `engine grounding-check`
//! (`docs/04-ARCHITECTURE.md` §4). It compares both on `structure`, `source_binding`,
//! `representation_sha256` and `counts`.
//!
//! **Live as of M6.** `oracle_agrees_on_simple_text` failed by design from M0 through M5,
//! carrying a diagnostic naming what was missing; it now runs the comparison it always described.
//!
//! **What the corpus can and cannot show.** Of the 15 Ethos-owned fixtures, 11 reach a grounding
//! artifact and 4 cannot be read at all. But all 15 are standard-14 Helvetica with no font
//! descriptor, so every artifact the corpus produces is `1 page / 0 elements / 0 spans` — enough
//! to compare the identity, source, coordinate-system, capability and page rules, and not enough
//! to reach the element, span or table rules at all. `the_oracle_agrees_on_an_artifact_with_real_elements_and_spans`
//! exists for exactly that reason, and uses a benchmark document rather than one of the 15.
//!
//! Nothing here skips. An absent oracle binary, a missing corpus, or a hash mismatch is a
//! **failure**, never a quiet pass. A harness that skips is a harness that reports green on a
//! machine where it never ran.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

/// Fixtures owned by the Ethos corpus. The M6 oracle criterion counts exactly these.
const ETHOS_OWNED_FIXTURE_COUNT: usize = 15;

/// How many of those fixtures reach a grounding artifact and are compared against the oracle.
///
/// Pinned for the same reason the 15 is: **this number is repeated in `CHANGELOG.md`,
/// `docs/07-VERIFY-BOUNDARY.md` and `docs/README.md`**, and without an assertion a regression that
/// made the extractor refuse six more documents would move them quietly into the refused list,
/// leave the suite green, and leave three docs saying 11.
const ORACLE_AGREED_COUNT: usize = 12;

/// The fixture the first oracle comparison runs on (`docs/05-MILESTONES.md` M0).
const M0_FIXTURE: &str = "synthetic/simple-text";

// ---------------------------------------------------------------------------------------------
// Locating things. Every failure path here names what was looked for and where.
// ---------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/crates/engine-cli
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir always has two ancestors")
        .to_path_buf()
}

fn manifest_path() -> PathBuf {
    repo_root().join("fixtures/manifest.json")
}

fn read_manifest() -> serde_json::Value {
    let p = manifest_path();
    let bytes = std::fs::read(&p).unwrap_or_else(|e| {
        panic!(
            "fixture manifest unreadable at {}: {e}\n\
             This file is the only record of which fixtures this repo tests against.",
            p.display()
        )
    });
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("fixture manifest at {} is not valid JSON: {e}", p.display()))
}

/// Resolve one of the manifest's declared corpus roots by name.
///
/// Two roots exist because M2's acceptance names documents the conformance corpus does not
/// contain: the bounded-cost A/B needs a 492-page PDF, and those live in Ethos's benchmark
/// corpus, not `fixtures/`. Keeping them in separate roots stops benchmark documents from
/// inflating the 15-fixture count the M6 oracle criterion is stated over.
fn corpus_root(manifest: &serde_json::Value, root: &str) -> PathBuf {
    let decl = &manifest["roots"][root];
    assert!(
        !decl.is_null(),
        "manifest declares no root named `{root}`; known roots are {:?}",
        manifest["roots"]
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    );

    if let Some(var) = decl["env"].as_str() {
        if let Ok(v) = std::env::var(var) {
            return PathBuf::from(v);
        }
    }
    let rel = decl["default"]
        .as_str()
        .unwrap_or_else(|| panic!("root `{root}` declares no default path"));
    repo_root().join(rel)
}

/// Every root a fixture entry actually references.
fn referenced_roots(manifest: &serde_json::Value) -> Vec<String> {
    let mut roots: Vec<String> = manifest["fixtures"]
        .as_array()
        .expect("fixtures array")
        .iter()
        .filter_map(|f| f["root"].as_str().map(str::to_owned))
        .collect();
    roots.sort();
    roots.dedup();
    roots
}

/// Resolve the Ethos CLI, preferring the repo build over whatever is on `PATH`.
///
/// Order matters and is not cosmetic: at the time of writing, the repo build is 0.6.0 and the
/// binary on `PATH` is 0.5.0. Silently oracling against a stale verifier would produce
/// confident, wrong agreement.
///
/// **`ETHOS_BIN` is authoritative, not a hint.** If it is set and does not name a file, that is
/// a hard error — never a fallback to some other binary. An operator who pinned the oracle
/// explicitly and got a different one silently is in the worst position of all: they believe
/// they know which verifier answered.
fn resolve_ethos_binary() -> Result<PathBuf, String> {
    resolve_ethos_binary_from(std::env::var("ETHOS_BIN").ok())
}

/// The resolution rule, with the environment passed in rather than read.
///
/// Split out so the `ETHOS_BIN`-is-authoritative branch is testable without mutating process
/// environment — `set_var` races across Rust's threaded test harness, so a test that set it
/// could corrupt a sibling test rather than prove anything.
fn resolve_ethos_binary_from(explicit: Option<String>) -> Result<PathBuf, String> {
    let mut tried = Vec::new();

    if let Some(v) = explicit {
        let p = PathBuf::from(&v);
        if p.is_file() {
            return Ok(p);
        }
        return Err(format!(
            "ETHOS_BIN is set to `{v}`, which is not a file.\n\n\
             ETHOS_BIN is an explicit pin, so this is a hard error rather than a fallback: \
             resolving\n\
             to a different binary would mean oracling against a verifier nobody chose. Fix the \
             path\n\
             or unset ETHOS_BIN to use the default search order."
        ));
    }
    tried.push("  ETHOS_BIN (unset)".to_string());

    let repo_build = repo_root().join("../ethos/target/release/ethos");
    if repo_build.is_file() {
        return Ok(repo_build);
    }
    tried.push(format!("  {} (absent)", repo_build.display()));

    if let Ok(out) = Command::new("sh")
        .arg("-c")
        .arg("command -v ethos")
        .output()
    {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    tried.push("  `command -v ethos` (not found)".to_string());

    Err(format!(
        "the Ethos CLI oracle could not be located. Tried, in order:\n{}\n\n\
         The oracle is a test-time dependency, not a runtime one — but its absence is a\n\
         FAILURE, never a skip (docs/04-ARCHITECTURE.md §4). Build it with\n\
         `cargo build --release` in the Ethos repo, or set ETHOS_BIN.",
        tried.join("\n")
    ))
}

// ---------------------------------------------------------------------------------------------
// Preconditions. These pass at M0.
// ---------------------------------------------------------------------------------------------

/// Every workspace crate links and is reachable from the CLI.
///
/// Trivial by design — it exists so the `CRATE_NAME` constants have the use their doc comments
/// claim, and so the wiring is asserted rather than assumed.
///
/// # It named four and asserted three, against a workspace of five
///
/// `engine-office` joined the workspace at v2-S1 and was never added, so the name said *every*
/// over three of the four crates that export a `CRATE_NAME` — and the doc said *four-crate*
/// while `Cargo.toml` listed five members. Both are repaired, and the count is now checked
/// against `Cargo.toml` rather than restated here, because restating it is what went stale.
///
/// **Four, not five, and the difference is not an omission.** `engine-cli` is the crate this
/// test lives in; it is a binary with no library target and exports no `CRATE_NAME` to assert.
/// So the assertion is that every workspace member *other than this one* is linked and names
/// itself, which is a property that survives a sixth crate being added — the sixth would trip
/// the count below rather than slipping past a list nobody remembered to grow.
#[test]
fn every_workspace_crate_links() {
    assert_eq!(engine_core::CRATE_NAME, "engine-core");
    assert_eq!(engine_pdf::CRATE_NAME, "engine-pdf");
    assert_eq!(engine_office::CRATE_NAME, "engine-office");
    assert_eq!(engine_grounding::CRATE_NAME, "engine-grounding");

    // Derived, not restated. A sixth member fails here and names itself.
    let manifest = std::fs::read_to_string(repo_root().join("Cargo.toml")).expect("Cargo.toml");
    let members = manifest
        .split_once("members = [")
        .expect("the `[workspace]` table declares no `members`")
        .1;
    let members = &members[..members.find(']').expect("`members` is unterminated")];
    let listed: Vec<&str> = members
        .split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|p| p.rsplit('/').next())
        .collect();
    assert_eq!(
        listed.len(),
        5,
        "Cargo.toml lists {} workspace member(s): {listed:?}. Four of them export a          `CRATE_NAME` this test asserts, and the fifth is `engine-cli`, which is this test's own          binary crate. A new member needs a line above, or a sentence saying why it has none.",
        listed.len()
    );
    for expected in [
        "engine-core",
        "engine-pdf",
        "engine-office",
        "engine-grounding",
        "engine-cli",
    ] {
        assert!(
            listed.contains(&expected),
            "`{expected}` is asserted here but is no longer a workspace member: {listed:?}"
        );
    }
}

/// The manifest describes the corpus the M6 criterion counts.
///
/// Also checks the manifest's own declared counts against the array. Those numbers are repeated
/// across six docs; unvalidated, an added or removed entry leaves every one of them silently
/// wrong while the harness still passes.
#[test]
fn manifest_declares_fifteen_ethos_owned_fixtures() {
    let manifest = read_manifest();
    let fixtures = manifest["fixtures"]
        .as_array()
        .expect("manifest has a fixtures array");

    let ethos_owned = fixtures
        .iter()
        .filter(|f| f["owner"].as_str() == Some("ethos"))
        .count();

    assert_eq!(
        ethos_owned, ETHOS_OWNED_FIXTURE_COUNT,
        "the M6 oracle criterion is stated over exactly {ETHOS_OWNED_FIXTURE_COUNT} \
         Ethos-owned conformance fixtures.\n\
         Benchmark and engine-owned fixtures are additional test assets and must not inflate \
         that count (fixtures/README.md)."
    );

    // The declared counts must match reality, or they are decoration.
    for (key, actual) in [
        (
            "conformance_ethos_owned",
            fixtures
                .iter()
                .filter(|f| {
                    f["root"].as_str() == Some("conformance")
                        && f["owner"].as_str() == Some("ethos")
                })
                .count(),
        ),
        (
            "benchmark",
            fixtures
                .iter()
                .filter(|f| f["root"].as_str() == Some("benchmark"))
                .count(),
        ),
        (
            "engine_owned",
            fixtures
                .iter()
                .filter(|f| f["owner"].as_str() == Some("engine"))
                .count(),
        ),
    ] {
        let declared = manifest["counts"][key].as_u64().unwrap_or_else(|| {
            panic!("manifest[\"counts\"][\"{key}\"] is missing or not a number")
        });
        assert_eq!(
            declared as usize, actual,
            "manifest counts.{key} says {declared} but the fixtures array has {actual}"
        );
    }

    // Every fixture must name a root that actually exists in the roots table.
    for root in referenced_roots(&manifest) {
        assert!(
            !manifest["roots"][&root].is_null(),
            "a fixture references root `{root}`, which the roots table does not declare"
        );
    }
}

/// Every referenced fixture exists and hashes to what the manifest recorded.
///
/// This is what makes "reference, do not copy" safe: a fixture changing under us becomes a
/// visible failure in this repo rather than a silent change of meaning.
#[test]
fn manifest_hashes_verify_against_the_ethos_tree() {
    let manifest = read_manifest();

    for root_name in referenced_roots(&manifest) {
        let root = corpus_root(&manifest, &root_name);
        let env_hint = manifest["roots"][&root_name]["env"]
            .as_str()
            .unwrap_or("(no env override)");
        assert!(
            root.is_dir(),
            "corpus root `{root_name}` not found at {}\n\
             Set {env_hint} to override. Corpora are used read-only and are never copied into \
             this repo (docs/04-ARCHITECTURE.md §4).",
            root.display()
        );
    }

    let failures = hash_failures(&manifest);

    assert!(
        failures.is_empty(),
        "{} of {} fixtures failed hash verification:\n  {}\n\n\
         Either the Ethos corpus changed (re-pin the manifest deliberately, in its own commit) \
         or the manifest is wrong.",
        failures.len(),
        manifest["fixtures"].as_array().expect("fixtures").len(),
        failures.join("\n  ")
    );
}

/// Hash every fixture the manifest names, returning one message per failure.
///
/// Separated from the test so the detection itself can be tested against a deliberately
/// corrupted manifest — an assertion nobody has watched fail is an assertion nobody has tested.
fn hash_failures(manifest: &serde_json::Value) -> Vec<String> {
    let mut failures = Vec::new();

    for f in manifest["fixtures"].as_array().expect("fixtures array") {
        let id = f["id"].as_str().expect("fixture has an id");
        let rel = f["path"].as_str().expect("fixture has a path");
        let want = f["sha256"].as_str().expect("fixture has a sha256");
        let root_name = f["root"].as_str().expect("fixture declares a root");
        let path = corpus_root(manifest, root_name).join(rel);

        let Ok(bytes) = std::fs::read(&path) else {
            failures.push(format!("{id}: missing at {}", path.display()));
            continue;
        };

        let got = format!("sha256:{:x}", Sha256::digest(&bytes));
        if got != want {
            failures.push(format!(
                "{id}: hash mismatch\n     manifest: {want}\n     on disk:  {got}"
            ));
        }
    }

    failures
}

// ---------------------------------------------------------------------------------------------
// Negative paths. Two M0 acceptance criteria describe behaviour on bad input; without these the
// criteria were prose, and the branches had never executed.
// ---------------------------------------------------------------------------------------------

/// A corrupted manifest hash must be detected, and the message must name the fixture.
#[test]
fn a_mutated_manifest_hash_is_detected() {
    let mut manifest = read_manifest();
    let target = "synthetic/simple-text";

    let entry = manifest["fixtures"]
        .as_array_mut()
        .expect("fixtures array")
        .iter_mut()
        .find(|f| f["id"].as_str() == Some(target))
        .unwrap_or_else(|| panic!("manifest does not contain {target}"));
    entry["sha256"] = serde_json::Value::String(format!("sha256:{}", "0".repeat(64)));

    let failures = hash_failures(&manifest);

    assert_eq!(
        failures.len(),
        1,
        "exactly one fixture was corrupted, so exactly one failure was expected; got: {failures:#?}"
    );
    assert!(
        failures[0].contains(target) && failures[0].contains("hash mismatch"),
        "the failure must name the fixture and the problem; got: {}",
        failures[0]
    );
}

/// `ETHOS_BIN` pointing at a missing file is a hard error, never a fallback.
///
/// This is the branch the docs argue for most strongly, so it is the one that most needs a test:
/// resolving silently to a verifier nobody chose is worse than finding none at all.
#[test]
fn an_explicit_but_missing_ethos_bin_is_a_hard_error() {
    let err = resolve_ethos_binary_from(Some("/nonexistent/definitely/not/ethos".to_string()))
        .expect_err("a non-existent ETHOS_BIN must not resolve to any binary");

    assert!(
        err.contains("ETHOS_BIN") && err.contains("not a file"),
        "the error must say which pin failed and why; got: {err}"
    );

    // The real hazard is a silent fallback. Prove it did not happen: a valid oracle IS
    // resolvable on this machine via the default order, and the explicit pin must not reach it.
    if let Ok(fallback) = resolve_ethos_binary_from(None) {
        assert!(
            !err.contains(&fallback.display().to_string()),
            "the explicit pin fell back to {} instead of failing",
            fallback.display()
        );
    }
}

/// The oracle binary is present and reports a version. Absence fails loudly.
#[test]
fn ethos_oracle_binary_is_available() {
    let bin = resolve_ethos_binary().unwrap_or_else(|e| panic!("{e}"));

    let out = Command::new(&bin)
        .arg("--version")
        .output()
        .unwrap_or_else(|e| panic!("could not execute the oracle at {}: {e}", bin.display()));

    assert!(
        out.status.success(),
        "the oracle at {} exists but `--version` failed with {:?}",
        bin.display(),
        out.status.code()
    );

    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(
        !version.is_empty(),
        "the oracle at {} reported an empty version. Its identity is pinned into the engine \
         profile at v0.1, so an unidentifiable verifier is not usable (docs/07-VERIFY-BOUNDARY.md §4).",
        bin.display()
    );

    eprintln!("oracle: {version} at {}", bin.display());
}

// ---------------------------------------------------------------------------------------------
// The oracle comparison. Real as of M6.
// ---------------------------------------------------------------------------------------------

/// The four fields `docs/01-CONTRACT.md` §11 makes the agreement criterion.
///
/// Compared as parsed values rather than as raw bytes, and that is not a weakening: Ethos wraps
/// its report in an in-toto Statement while the engine emits the report bare, so the *files*
/// cannot be identical by construction. What must be identical is the report, and this is it.
#[derive(Debug, PartialEq, Eq)]
struct Agreement {
    structure: String,
    source_binding: String,
    representation_sha256: Option<String>,
    counts: Option<serde_json::Value>,
}

impl Agreement {
    fn from_report(report: &serde_json::Value, who: &str) -> Self {
        assert_eq!(
            report["artifact_type"].as_str(),
            Some("ethos.grounding_validation.v1"),
            "{who} did not produce a validation report: {report}"
        );
        assert_eq!(report["schema_version"].as_str(), Some("1.0.0"), "{who}");
        Self {
            structure: report["structure"]
                .as_str()
                .unwrap_or_else(|| panic!("{who}: report has no structure: {report}"))
                .to_string(),
            source_binding: report["source_binding"]
                .as_str()
                .unwrap_or_else(|| panic!("{who}: report has no source_binding: {report}"))
                .to_string(),
            representation_sha256: report["representation_sha256"].as_str().map(str::to_string),
            counts: report.get("counts").cloned(),
        }
    }
}

/// Find the validation report in whatever the oracle printed.
///
/// **Two shapes are accepted, and the reason is a measurement, not defensiveness.** The Ethos
/// binary built in that tree today prints the report bare. Its committed source already wraps it
/// in an in-toto Statement (`_type` / `subject` / `predicateType` / `predicate`), so the binary is
/// *behind its own source* — rebuilding Ethos changes the shape of this output. Accepting both
/// means the agreement criterion survives that rebuild, which is the point of having an oracle at
/// all. Anything that is neither shape is a hard failure, never a guess.
fn extract_report(stdout: &[u8], who: &str) -> serde_json::Value {
    let text = String::from_utf8_lossy(stdout);
    let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap_or_else(|e| {
        panic!("{who} did not print JSON ({e}). stdout was:\n{text}");
    });

    if value["artifact_type"].as_str() == Some("ethos.grounding_validation.v1") {
        return value;
    }
    if let Some(predicate) = value.get("predicate") {
        if predicate["artifact_type"].as_str() == Some("ethos.grounding_validation.v1") {
            return predicate.clone();
        }
    }
    panic!(
        "{who} printed JSON that is neither a bare validation report nor a statement wrapping \
         one. Refusing to guess which field holds the verdict:\n{value}"
    );
}

/// Run a subcommand of the engine binary.
fn run_engine(args: &[&std::ffi::OsStr]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_engine"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

/// Run the Ethos oracle. Its absence is a **failure**, never a skip.
fn run_oracle(args: &[&std::ffi::OsStr]) -> std::process::Output {
    let bin = resolve_ethos_binary().unwrap_or_else(|e| panic!("{e}"));
    Command::new(bin)
        .args(args)
        .output()
        .expect("the ethos binary runs")
}

/// A directory no other call can be using.
///
/// The counter is not decoration. Cargo runs these tests in parallel threads of one process, and
/// two of them walk the same corpus — so a name built from the pid and the fixture id alone
/// collides, and one test deletes the working directory of another mid-run. That surfaced as
/// "the engine printed no report", which reads like a product bug and was a harness bug.
fn scratch(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let unique = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "ethos-engine-oracle-{}-{unique}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Produce a grounding artifact for `pdf`, or say why the document could not get that far.
///
/// `Err` is not a test failure here: the corpus deliberately contains documents this backend
/// refuses, and pretending otherwise is what would make the 15-fixture obligation dishonest.
fn ground_fixture(pdf: &Path, dir: &Path) -> Result<PathBuf, String> {
    let repr = dir.join("repr.json");
    let out = run_engine(&["extract".as_ref(), pdf.as_ref()]);
    if !out.status.success() {
        return Err(format!(
            "extract exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    std::fs::write(&repr, &out.stdout).expect("write representation");

    let grounding = dir.join("grounding.json");
    let out = run_engine(&["ground".as_ref(), repr.as_ref()]);
    if !out.status.success() {
        return Err(format!(
            "ground exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    std::fs::write(&grounding, &out.stdout).expect("write grounding");
    Ok(grounding)
}

/// Both checkers, on one artifact, with the same arguments.
fn compare(grounding: &Path, source: Option<&Path>, label: &str) -> (Agreement, i32, i32) {
    let mut engine_args: Vec<&std::ffi::OsStr> =
        vec!["grounding-check".as_ref(), grounding.as_ref()];
    let mut oracle_args: Vec<&std::ffi::OsStr> =
        vec!["grounding".as_ref(), "check".as_ref(), grounding.as_ref()];
    if let Some(pdf) = source {
        engine_args.push("--source-artifact".as_ref());
        engine_args.push(pdf.as_ref());
        oracle_args.push("--source-artifact".as_ref());
        oracle_args.push(pdf.as_ref());
    }

    let ours = run_engine(&engine_args);
    let theirs = run_oracle(&oracle_args);

    let a = Agreement::from_report(&extract_report(&ours.stdout, "engine"), "engine");
    let b = Agreement::from_report(&extract_report(&theirs.stdout, "ethos"), "ethos");
    assert_eq!(
        a, b,
        "{label}: the engine and the oracle disagree about the same artifact. This is never \
         fixed by relaxing a field — fix the emission or the check semantics until they match."
    );

    (
        a,
        ours.status.code().expect("engine exited normally"),
        theirs.status.code().expect("ethos exited normally"),
    )
}

/// Every Ethos-owned fixture, and what became of it.
struct Coverage {
    agreed: Vec<String>,
    refused: Vec<(String, String)>,
}

fn walk_ethos_owned_fixtures() -> Coverage {
    let manifest = read_manifest();
    let mut agreed = Vec::new();
    let mut refused = Vec::new();

    for fixture in manifest["fixtures"].as_array().expect("fixtures array") {
        if fixture["owner"].as_str() != Some("ethos") {
            continue;
        }
        let id = fixture["id"].as_str().expect("id").to_string();
        let root = corpus_root(&manifest, fixture["root"].as_str().expect("root"));
        let pdf = root.join(fixture["path"].as_str().expect("path"));
        assert!(
            pdf.is_file(),
            "fixture {id} missing at {}. A missing corpus is a failure, never a skip.",
            pdf.display()
        );

        let dir = scratch(&id.replace('/', "-"));
        match ground_fixture(&pdf, &dir) {
            Ok(grounding) => {
                let (_, engine_exit, oracle_exit) = compare(&grounding, Some(&pdf), &id);
                assert_eq!(engine_exit, 0, "{id}: a matched artifact must exit 0");
                assert_eq!(oracle_exit, 0, "{id}: the oracle agrees it is clean");
                agreed.push(id);
            }
            Err(why) => refused.push((id, why)),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    Coverage { agreed, refused }
}

/// **The M0 gate, closed.** One fixture, both checkers, the four fields.
///
/// This test existed from the first commit and failed on purpose for six milestones, carrying a
/// diagnostic that named what was missing. It is the same test, with the panic replaced by the
/// comparison it always described.
#[test]
fn oracle_agrees_on_simple_text() {
    let manifest = read_manifest();
    let fixture = manifest["fixtures"]
        .as_array()
        .expect("fixtures array")
        .iter()
        .find(|f| f["id"].as_str() == Some(M0_FIXTURE))
        .unwrap_or_else(|| panic!("manifest does not contain the M0 fixture {M0_FIXTURE}"));
    let root = corpus_root(&manifest, fixture["root"].as_str().expect("fixture root"));
    let pdf = root.join(fixture["path"].as_str().expect("fixture path"));
    assert!(pdf.is_file(), "M0 fixture missing at {}", pdf.display());

    let dir = scratch("simple-text");
    let grounding = ground_fixture(&pdf, &dir)
        .unwrap_or_else(|e| panic!("{M0_FIXTURE} must reach a grounding artifact: {e}"));

    let (agreement, engine_exit, oracle_exit) = compare(&grounding, Some(&pdf), M0_FIXTURE);

    assert_eq!(agreement.structure, "valid");
    assert_eq!(agreement.source_binding, "matched");
    assert!(agreement
        .representation_sha256
        .as_deref()
        .is_some_and(|h| h.starts_with("sha256:")));
    assert!(agreement.counts.is_some());
    assert_eq!(engine_exit, 0);
    assert_eq!(oracle_exit, 0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// Every Ethos-owned fixture is accounted for — agreed, or refused for a named reason.
///
/// **The 15 is auditable, not asserted.** A fixture this backend cannot open must not quietly
/// vanish from the obligation, so the two lists are required to partition the corpus exactly, and
/// the refusals are printed with their reasons.
#[test]
fn oracle_agrees_on_all_ethos_owned_fixtures() {
    let coverage = walk_ethos_owned_fixtures();

    assert_eq!(
        coverage.agreed.len() + coverage.refused.len(),
        ETHOS_OWNED_FIXTURE_COUNT,
        "every Ethos-owned fixture must be either compared or explicitly refused; agreed={:?} \
         refused={:?}",
        coverage.agreed,
        coverage.refused
    );
    assert_eq!(
        coverage.agreed.len(),
        ORACLE_AGREED_COUNT,
        "the number of fixtures compared against the oracle changed. It is quoted in \
         CHANGELOG.md, docs/07-VERIFY-BOUNDARY.md and docs/README.md — update the constant and \
         those three, in a commit that says why. agreed={:?} refused={:?}",
        coverage.agreed,
        coverage.refused
    );

    eprintln!(
        "oracle: {}/{} Ethos-owned fixtures compared and agreed",
        coverage.agreed.len(),
        ETHOS_OWNED_FIXTURE_COUNT
    );
    for (id, why) in &coverage.refused {
        eprintln!("oracle: {id} produced no grounding artifact — {why}");
    }
}

/// The documents this backend refuses still fail the way the contract says they do.
///
/// Their exclusion from the comparison above is only honest if the exclusion is itself checked:
/// a fixture that started silently producing an empty artifact would otherwise move from
/// "refused" to "agreed" and nobody would notice.
#[test]
fn refused_fixtures_fail_closed_rather_than_producing_an_artifact() {
    let coverage = walk_ethos_owned_fixtures();
    assert!(
        !coverage.refused.is_empty(),
        "the corpus is known to contain documents this backend refuses (a 19-byte xref, an \
         encrypted file, an invalid header). If none refused, either the corpus changed or the \
         engine started accepting what it should not."
    );

    let manifest = read_manifest();
    for (id, _) in &coverage.refused {
        let fixture = manifest["fixtures"]
            .as_array()
            .expect("fixtures")
            .iter()
            .find(|f| f["id"].as_str() == Some(id.as_str()))
            .expect("refused fixture is in the manifest");
        let root = corpus_root(&manifest, fixture["root"].as_str().expect("root"));
        let pdf = root.join(fixture["path"].as_str().expect("path"));

        let out = run_engine(&["extract".as_ref(), pdf.as_ref()]);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{id}: a document that cannot be read exits 2"
        );
        assert!(
            out.stdout.is_empty(),
            "{id}: no artifact may reach stdout for a document that could not be read"
        );
    }
}

/// `matched` / `mismatched` / `not_checked`, one fixture, both checkers.
#[test]
fn the_source_binding_trichotomy_agrees_with_the_oracle() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest, "conformance");
    let pdf = root.join("synthetic/simple-text/document.pdf");
    let other = root.join("synthetic/two-columns/document.pdf");
    assert!(pdf.is_file() && other.is_file());

    let dir = scratch("trichotomy");
    let grounding = ground_fixture(&pdf, &dir).expect("simple-text grounds");

    let (matched, e0, o0) = compare(&grounding, Some(&pdf), "matched");
    assert_eq!(matched.source_binding, "matched");
    assert_eq!((e0, o0), (0, 0));

    let (mismatched, e1, o1) = compare(&grounding, Some(&other), "mismatched");
    assert_eq!(mismatched.source_binding, "mismatched");
    assert_eq!(
        mismatched.structure, "valid",
        "a mismatch is about the source, not about the artifact"
    );
    assert!(
        e1 != 0 && o1 != 0,
        "both must refuse: engine {e1}, ethos {o1}"
    );

    let (not_checked, e2, o2) = compare(&grounding, None, "not_checked");
    assert_eq!(not_checked.source_binding, "not_checked");
    assert_eq!((e2, o2), (0, 0));

    // The artifact is the same file in all three runs, so the digest cannot move.
    assert_eq!(
        matched.representation_sha256,
        mismatched.representation_sha256
    );
    assert_eq!(
        matched.representation_sha256,
        not_checked.representation_sha256
    );
    assert_eq!(matched.counts, not_checked.counts);

    let _ = std::fs::remove_dir_all(&dir);
}

/// A deliberately malformed artifact: both checkers call it invalid, for the same reason.
#[test]
fn an_invalid_artifact_is_refused_by_both_with_the_same_code_and_path() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest, "conformance");
    let pdf = root.join("synthetic/simple-text/document.pdf");
    let dir = scratch("invalid");
    let grounding = ground_fixture(&pdf, &dir).expect("simple-text grounds");

    // One break per rule, each reaching a different code, so agreement is not a coincidence of
    // one shared failure mode.
    let breaks: [(&str, &str, &str); 4] = [
        (
            "artifact_type",
            "\"ethos.grounding.v1\"",
            "\"ethos.grounding.v2\"",
        ),
        ("coordinate unit", "\"centipoint\"", "\"point\""),
        ("page index", "\"index\":1", "\"index\":2"),
        // v1-S1 inverted this break. `tables` is now TRUE and the array is present, so the
        // deliberate fault is claiming the capability is false while the key is still there —
        // the same capability-versus-array mismatch, from the other side.
        ("capabilities", "\"tables\":true", "\"tables\":false"),
    ];

    let original = std::fs::read_to_string(&grounding).expect("read");
    for (label, from, to) in breaks {
        assert!(
            original.contains(from),
            "{label}: `{from}` not in the artifact"
        );
        let broken_path = dir.join(format!("broken-{}.json", label.replace(' ', "-")));
        std::fs::write(&broken_path, original.replacen(from, to, 1)).expect("write");

        let (agreement, engine_exit, oracle_exit) = compare(&broken_path, None, label);
        assert_eq!(agreement.structure, "invalid", "{label}");
        assert_eq!(agreement.source_binding, "not_checked", "{label}");
        assert!(
            agreement.representation_sha256.is_none() && agreement.counts.is_none(),
            "{label}: an invalid artifact carries neither a digest nor counts"
        );
        assert!(engine_exit != 0 && oracle_exit != 0, "{label}");

        // The error object too — agreeing on `invalid` while disagreeing on why is half an
        // agreement, and the code and path are the machine-readable half.
        let ours = run_engine(&["grounding-check".as_ref(), broken_path.as_ref()]);
        let theirs = run_oracle(&["grounding".as_ref(), "check".as_ref(), broken_path.as_ref()]);
        let a = extract_report(&ours.stdout, "engine");
        let b = extract_report(&theirs.stdout, "ethos");
        assert_eq!(
            a["error"]["code"], b["error"]["code"],
            "{label}: error code"
        );
        assert_eq!(
            a["error"]["path"], b["error"]["path"],
            "{label}: error path"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// A non-PDF source artifact is refused by both, with no report at all.
#[test]
fn a_non_pdf_source_artifact_is_refused_by_both() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest, "conformance");
    let pdf = root.join("synthetic/simple-text/document.pdf");
    let dir = scratch("nonpdf");
    let grounding = ground_fixture(&pdf, &dir).expect("simple-text grounds");

    // The grounding artifact itself, handed in where a PDF belongs.
    let ours = run_engine(&[
        "grounding-check".as_ref(),
        grounding.as_ref(),
        "--source-artifact".as_ref(),
        grounding.as_ref(),
    ]);
    let theirs = run_oracle(&[
        "grounding".as_ref(),
        "check".as_ref(),
        grounding.as_ref(),
        "--source-artifact".as_ref(),
        grounding.as_ref(),
    ]);

    assert_eq!(ours.status.code(), Some(2), "the engine refuses to answer");
    assert_ne!(theirs.status.code(), Some(0), "so does the oracle");
    assert!(
        ours.stdout.is_empty() && theirs.stdout.is_empty(),
        "neither writes a report: 'these bytes are not the source' would be a different, and \
         wrong, statement about a file that is not a document at all"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// **Double-run byte identity over the whole path, on files.**
///
/// `classify → extract → ground → grounding-check`, twice, into two directories, comparing the
/// bytes on disk. Not parsed equality: a value comparison would pass while the files differed by
/// key order or a trailing newline, and the contract is about artifacts a consumer stores.
#[test]
fn the_whole_path_is_byte_identical_across_two_runs() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest, "conformance");

    let mut checked = 0;
    for fixture in manifest["fixtures"].as_array().expect("fixtures") {
        if fixture["owner"].as_str() != Some("ethos") {
            continue;
        }
        let id = fixture["id"].as_str().expect("id");
        let pdf = root.join(fixture["path"].as_str().expect("path"));

        let run = |tag: &str| -> Option<Vec<(String, Vec<u8>)>> {
            let dir = scratch(&format!("{}-{tag}", id.replace('/', "-")));
            let classify = run_engine(&["classify".as_ref(), pdf.as_ref()]);
            let extract = run_engine(&["extract".as_ref(), pdf.as_ref()]);
            if !extract.status.success() {
                let _ = std::fs::remove_dir_all(&dir);
                return None;
            }
            let repr = dir.join("repr.json");
            std::fs::write(&repr, &extract.stdout).expect("write");
            let ground = run_engine(&["ground".as_ref(), repr.as_ref()]);
            let grounding = dir.join("grounding.json");
            std::fs::write(&grounding, &ground.stdout).expect("write");
            let check = run_engine(&[
                "grounding-check".as_ref(),
                grounding.as_ref(),
                "--source-artifact".as_ref(),
                pdf.as_ref(),
            ]);

            let files = vec![
                ("classify".to_string(), classify.stdout),
                ("extract".to_string(), std::fs::read(&repr).expect("read")),
                (
                    "ground".to_string(),
                    std::fs::read(&grounding).expect("read"),
                ),
                ("grounding-check".to_string(), check.stdout),
            ];
            let _ = std::fs::remove_dir_all(&dir);
            Some(files)
        };

        let (Some(a), Some(b)) = (run("a"), run("b")) else {
            continue; // a document this backend refuses; covered by the fail-closed test
        };
        for ((stage, x), (_, y)) in a.iter().zip(b.iter()) {
            assert_eq!(
                x, y,
                "{id}: `{stage}` is not byte-identical across two runs"
            );
            assert!(!x.is_empty(), "{id}: `{stage}` produced nothing to compare");
        }
        checked += 1;
    }

    assert_eq!(
        checked, ORACLE_AGREED_COUNT,
        "a floor would let fixtures drop out silently; this is the same set the oracle compares"
    );
}

/// `grounding-check` contains no verification concept.
#[test]
fn the_checker_has_no_verification_semantics() {
    let mut code = String::new();
    for file in [
        "crates/engine-grounding/src/check.rs",
        "crates/engine-cli/src/main.rs",
    ] {
        let src = std::fs::read_to_string(repo_root().join(file)).expect("readable");
        // Cut the unit-test module first. A test that ASSERTS these tokens are absent has to
        // name them, and a scan that cannot tell a rule from its enforcement fires on itself —
        // which is exactly what this one did before the split.
        let production = src
            .split("#[cfg(test)]")
            .next()
            .expect("source before any tests");
        for line in production.lines() {
            // Prose arguing the rule is expected and welcome; an identifier is not.
            if !line.trim_start().starts_with("//") {
                code.push_str(line);
                code.push('\n');
            }
        }
    }
    assert!(
        code.len() > 2000,
        "the scan found almost nothing to check, so it would pass over anything"
    );
    for banned in [
        "evidence_tier",
        "is_grounded",
        "all_evidence_grounded",
        "verdict",
        "quote_window",
        "claim_id",
    ] {
        assert!(
            !code.contains(banned),
            "`{banned}` appears in the check path. The engine validates structure and binding; \
             reimplementing verifier semantics is how a second authority is born by accident."
        );
    }
}

/// The report is found in either shape the oracle can print.
///
/// **Not speculative future-proofing — CI needs the second shape today.** The Ethos binary built
/// in the sibling tree is dated before the commit that introduced the in-toto wrapper, so it
/// prints the report bare; the ref CI pins (`ETHOS_ORACLE_REF`) contains that commit, so the
/// binary CI builds prints it wrapped. One harness has to read both, and the local run alone
/// would never exercise the wrapped path.
#[test]
fn the_report_is_extracted_from_a_bare_or_a_wrapped_oracle_response() {
    let report = serde_json::json!({
        "artifact_type": "ethos.grounding_validation.v1",
        "schema_version": "1.0.0",
        "structure": "valid",
        "source_binding": "matched",
        "representation_sha256": "sha256:".to_string() + &"a".repeat(64),
        "counts": {"pages": 1, "elements": 1, "spans": 1, "tables": 0},
    });

    let bare = serde_json::to_vec(&report).unwrap();
    let wrapped = serde_json::to_vec(&serde_json::json!({
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{"name": "grounding.json", "digest": {"sha256": "b".repeat(64)}}],
        "predicateType": "https://ethos.example/predicate/grounding-validation/v1",
        "predicate": report,
    }))
    .unwrap();

    let from_bare = Agreement::from_report(&extract_report(&bare, "bare"), "bare");
    let from_wrapped = Agreement::from_report(&extract_report(&wrapped, "wrapped"), "wrapped");
    assert_eq!(
        from_bare, from_wrapped,
        "the same report in two envelopes must compare equal"
    );
    assert_eq!(from_bare.structure, "valid");
    assert_eq!(from_bare.counts.as_ref().unwrap()["pages"], 1);
}

/// Anything that is neither shape is a hard failure, never a guess.
#[test]
#[should_panic(expected = "neither a bare validation report nor a statement")]
fn an_unrecognised_oracle_response_is_refused_rather_than_guessed_at() {
    // A statement whose predicate is something else entirely. Hunting for the first object with
    // a `structure` key would "work" here and would be exactly the best-effort parsing of an
    // unrecognised shape that docs/01-CONTRACT.md §8 forbids.
    let odd = serde_json::to_vec(&serde_json::json!({
        "_type": "https://in-toto.io/Statement/v1",
        "predicate": {"artifact_type": "ethos.something_else.v1", "structure": "valid"},
    }))
    .unwrap();
    let _ = extract_report(&odd, "odd");
}

/// A differential corpus: adversarial inputs, both checkers, same verdict **and same reason**.
///
/// The four hand-picked breaks in the test above all happen to agree. That is weaker than it
/// looks — agreement on inputs chosen by the person writing the checker is close to agreement
/// with oneself. These were found by *hunting* for divergence, and five of them originally
/// disagreed: a float reported `invalid_field` where Ethos says `invalid_json`; an integer past
/// `2^53-1` reported `invalid_invariant` at `/pages/0` where Ethos says `limit_exceeded` at `/`;
/// an oversized multibyte string reported the field path where Ethos reports `/`; an unknown
/// field reported `/` where Ethos reports the exact path; a null reported `invalid_field` where
/// Ethos says `invalid_json`.
///
/// Each was a real fidelity gap, and each is fixed rather than tolerated. This test is what stops
/// them coming back.
#[test]
fn adversarial_inputs_agree_on_verdict_code_and_path() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest, "conformance");
    let pdf = root.join("synthetic/simple-text/document.pdf");
    let dir = scratch("differential");
    let grounding = ground_fixture(&pdf, &dir).expect("simple-text grounds");
    let original = std::fs::read_to_string(&grounding).expect("read");

    // (label, how to break it) — raw text edits, so the input can be malformed in ways a typed
    // value could not express.
    /// (label, how to break the artifact's text)
    type Break = (&'static str, Box<dyn Fn(&str) -> String>);
    let cases: Vec<Break> = vec![
        (
            "float where an integer belongs",
            Box::new(|s: &str| s.replacen("\"rotation\":0", "\"rotation\":0.5", 1)),
        ),
        (
            "integer past 2^53-1",
            Box::new(|s: &str| s.replacen("\"rotation\":0", "\"rotation\":9007199254740992", 1)),
        ),
        (
            "null where a string belongs",
            Box::new(|s: &str| s.replacen("\"name\":\"ethos-engine\"", "\"name\":null", 1)),
        ),
        (
            "unknown field at the root",
            Box::new(|s: &str| s.replacen('{', "{\"zzz\":1,", 1)),
        ),
        (
            "unknown field inside a page",
            Box::new(|s: &str| s.replacen("\"rotation\":0", "\"zzz\":1,\"rotation\":0", 1)),
        ),
        (
            "duplicate key",
            Box::new(|s: &str| {
                s.replacen(
                    "\"media_type\":\"application/pdf\"",
                    "\"media_type\":\"application/pdf\",\"media_type\":\"x\"",
                    1,
                )
            }),
        ),
        (
            "trailing bytes after the JSON",
            Box::new(|s: &str| format!("{s} junk")),
        ),
        (
            "an oversized string, measured in bytes",
            Box::new(|s: &str| {
                s.replacen(
                    "\"unit\":\"centipoint\"",
                    &format!("\"unit\":\"{}\"", "é".repeat(8193)),
                    1,
                )
            }),
        ),
        (
            "a byte order mark",
            Box::new(|s: &str| format!("\u{feff}{s}")),
        ),
    ];

    for (label, mutate) in cases {
        let broken = dir.join(format!("d-{}.json", label.replace(' ', "-")));
        std::fs::write(&broken, mutate(&original)).expect("write");

        let ours = run_engine(&["grounding-check".as_ref(), broken.as_ref()]);
        let theirs = run_oracle(&["grounding".as_ref(), "check".as_ref(), broken.as_ref()]);

        let a = extract_report(&ours.stdout, "engine");
        let b = extract_report(&theirs.stdout, "ethos");

        assert_eq!(a["structure"], b["structure"], "{label}: structure");
        assert_eq!(
            a["structure"], "invalid",
            "{label}: the input is broken and both must say so"
        );
        assert_eq!(
            a["error"]["code"], b["error"]["code"],
            "{label}: error code"
        );
        assert_eq!(
            a["error"]["path"], b["error"]["path"],
            "{label}: error path"
        );
        assert_eq!(
            a["source_binding"], b["source_binding"],
            "{label}: source_binding"
        );
    }

    // The control: the unmodified artifact is still valid, so the corpus is measuring breakage
    // and not a checker that refuses everything.
    let (agreement, _, _) = compare(&grounding, None, "differential control");
    assert_eq!(agreement.structure, "valid");

    let _ = std::fs::remove_dir_all(&dir);
}

/// The agreement is measured on an artifact that actually **has** elements and spans.
///
/// **Without this test the M6 criterion is close to vacuous, and that is not a stylistic point.**
/// Every one of the 15 Ethos-owned fixtures is standard-14 Helvetica with no font descriptor, so
/// every node's geometry is typed-absent and every grounding artifact projected from the corpus is
/// `1 page / 0 elements / 0 spans / no tables`. Eleven agreements over eleven copies of the same
/// empty shape exercise the identity, source, coordinate-system, capability and page rules — and
/// none of the element loop, the span loop, the offsets rule, `valid_bbox`, `valid_id` or
/// `valid_kind`, which is most of what the checker is for.
///
/// `irs-form-1040-2025` is a real 2-page form that projects 1975 elements and 1975 spans. It is a
/// benchmark document rather than one of the 15, so it does not touch the oracle count — it just
/// makes the comparison mean something.
#[test]
fn the_oracle_agrees_on_an_artifact_with_real_elements_and_spans() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest, "benchmark");
    let pdf = root.join("irs-form-1040-2025.pdf");
    assert!(
        pdf.is_file(),
        "benchmark corpus missing at {}. A missing corpus is a failure, never a skip.",
        pdf.display()
    );

    let dir = scratch("rich");
    let grounding = ground_fixture(&pdf, &dir).expect("the IRS form grounds");

    let (agreement, engine_exit, oracle_exit) = compare(&grounding, Some(&pdf), "irs-form-1040");
    assert_eq!(agreement.structure, "valid");
    assert_eq!(agreement.source_binding, "matched");
    assert_eq!((engine_exit, oracle_exit), (0, 0));

    // The point of the fixture: the counts are not zero, so the loops actually ran.
    let counts = agreement.counts.as_ref().expect("valid carries counts");
    assert_eq!(counts["pages"], 2, "a two-page document");
    let elements = counts["elements"].as_u64().expect("integer");
    let spans = counts["spans"].as_u64().expect("integer");
    assert!(
        elements > 100 && spans > 100,
        "this test is worthless unless the artifact is populated: {counts}"
    );
    assert_eq!(counts["tables"], 0, "v0 emits no tables");

    // And a break inside the element loop — a region the empty corpus can never reach — is
    // refused by both, with the same code and path.
    let text = std::fs::read_to_string(&grounding).expect("read");
    let broken = dir.join("bad-element.json");
    std::fs::write(
        &broken,
        text.replacen("\"kind\":\"text_run\"", "\"kind\":\"Text_Run\"", 1),
    )
    .expect("write");

    let ours = run_engine(&["grounding-check".as_ref(), broken.as_ref()]);
    let theirs = run_oracle(&["grounding".as_ref(), "check".as_ref(), broken.as_ref()]);
    let a = extract_report(&ours.stdout, "engine");
    let b = extract_report(&theirs.stdout, "ethos");
    assert_eq!(a["structure"], "invalid");
    assert_eq!(a["structure"], b["structure"]);
    assert_eq!(a["error"]["code"], b["error"]["code"]);
    assert_eq!(
        a["error"]["path"], b["error"]["path"],
        "a fault inside the element loop must be located identically"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
