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
//! (`docs/04-ARCHITECTURE.md` §4). At M6 this compares both across all 15 fixtures and asserts
//! byte-identical agreement on `structure`, `source_binding`, `representation_sha256` and
//! `counts`.
//!
//! **At M0 the last test fails, deliberately and with a diagnostic.** That is the milestone: a
//! harness that reports honestly on a repo with no implementation. The first three tests pass —
//! they check preconditions that are real today.
//!
//! Nothing here skips. An absent oracle binary, a missing corpus, or a hash mismatch is a
//! **failure**, never a quiet pass. A harness that skips is a harness that reports green on a
//! machine where it never ran.

use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

/// Fixtures owned by the Ethos corpus. The M6 oracle criterion counts exactly these.
const ETHOS_OWNED_FIXTURE_COUNT: usize = 15;

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

/// Resolve the Ethos fixture corpus. `ETHOS_FIXTURES` wins; otherwise the manifest default.
fn corpus_root(manifest: &serde_json::Value) -> PathBuf {
    if let Ok(v) = std::env::var("ETHOS_FIXTURES") {
        return PathBuf::from(v);
    }
    let rel = manifest["corpus_root_default"]
        .as_str()
        .expect("manifest declares corpus_root_default");
    repo_root().join(rel)
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
    let mut tried = Vec::new();

    if let Ok(v) = std::env::var("ETHOS_BIN") {
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

/// The manifest describes the corpus the M6 criterion counts.
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
         Ethos-owned fixtures.\n\
         Engine-owned fixtures are additional test assets and must not inflate that count \
         (fixtures/README.md)."
    );
}

/// Every referenced fixture exists and hashes to what the manifest recorded.
///
/// This is what makes "reference, do not copy" safe: a fixture changing under us becomes a
/// visible failure in this repo rather than a silent change of meaning.
#[test]
fn manifest_hashes_verify_against_the_ethos_tree() {
    let manifest = read_manifest();
    let root = corpus_root(&manifest);

    assert!(
        root.is_dir(),
        "Ethos fixture corpus not found at {}\n\
         Set ETHOS_FIXTURES to override. The corpus is used read-only and is never copied \
         into this repo (docs/04-ARCHITECTURE.md §4).",
        root.display()
    );

    let mut failures = Vec::new();

    for f in manifest["fixtures"].as_array().expect("fixtures array") {
        let id = f["id"].as_str().expect("fixture has an id");
        let rel = f["path"].as_str().expect("fixture has a path");
        let want = f["sha256"].as_str().expect("fixture has a sha256");
        let path = root.join(rel);

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
// The oracle comparison. This FAILS at M0, on purpose.
// ---------------------------------------------------------------------------------------------

/// The first oracle agreement, on one fixture.
///
/// **Expected to fail until M6.** The failure is the point of M0: the harness exists, its
/// preconditions are real, and it reports honestly that the thing under test does not exist yet.
///
/// It must fail with the diagnostic below — never with `todo!()`, never with an `unwrap` on
/// `None`, and never by passing vacuously because a binary was missing.
#[test]
fn oracle_agrees_on_simple_text() {
    // Preconditions first, so a genuine environment problem is never misreported as
    // "unimplemented".
    let manifest = read_manifest();
    let root = corpus_root(&manifest);
    let oracle = resolve_ethos_binary().unwrap_or_else(|e| panic!("{e}"));

    let fixture = manifest["fixtures"]
        .as_array()
        .expect("fixtures array")
        .iter()
        .find(|f| f["id"].as_str() == Some(M0_FIXTURE))
        .unwrap_or_else(|| panic!("manifest does not contain the M0 fixture {M0_FIXTURE}"));

    let pdf = root.join(fixture["path"].as_str().expect("fixture path"));
    assert!(pdf.is_file(), "M0 fixture missing at {}", pdf.display());

    // What the engine would have to produce for this comparison to run.
    let engine = env!("CARGO_BIN_EXE_engine");

    panic!(
        "\n\
         ORACLE NOT YET COMPARABLE — this is the expected M0 state.\n\
         \n\
         Ready:\n\
         \x20 fixture   {}\n\
         \x20 oracle    {}\n\
         \x20 engine    {}\n\
         \n\
         Missing — the engine cannot yet produce a grounding artifact to compare:\n\
         \x20 M1  contract types, c14n, integer quanta, artifact identity\n\
         \x20 M2  classify\n\
         \x20 M3  extract\n\
         \x20 M4  capabilities and typed absence\n\
         \x20 M5  DocumentRepresentation v0 emit + ethos.grounding.v1 adapter\n\
         \x20 M6  this comparison, across all {} fixtures\n\
         \n\
         At M6 this test runs `engine grounding-check` and\n\
         `ethos grounding check <file> --source-artifact <pdf>` and asserts byte-identical\n\
         agreement on: structure, source_binding, representation_sha256, counts.\n\
         \n\
         Do not delete or #[ignore] this test to get a green build. It is the definition of\n\
         done (docs/05-MILESTONES.md).\n",
        pdf.display(),
        oracle.display(),
        engine,
        ETHOS_OWNED_FIXTURE_COUNT,
    );
}
