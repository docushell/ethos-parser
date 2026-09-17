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

//! `--diagnostics`: opt-in, on stderr, and unable to touch an artifact.
//!
//! `docs/01-CONTRACT.md` §4 permits exactly one home for volatile data and requires that it be
//! off by default and outside every fingerprint. Three properties make that real, and each is a
//! test here rather than a claim:
//!
//! 1. **Off by default** — a default run writes nothing to stderr at all.
//! 2. **stdout is the artifact either way** — the same bytes, with the flag and without.
//! 3. **Nothing volatile is inside** — no diagnostics field *name* appears anywhere in any
//!    artifact, checked by walking keys rather than by grepping bytes, so a nested occurrence
//!    cannot hide.
//!
//! The third is Ethos's own property-test shape. It catches the mistake the first two miss: a
//! future `timings` block added *inside* an artifact would keep stdout deterministic on one
//! machine and break it across two.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ethos_parser_core::diagnostics::Stage;
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

fn path_in(root_name: &str, rel: &str) -> PathBuf {
    let m: Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest"),
    )
    .expect("valid JSON");
    let decl = &m["roots"][root_name];
    let root = match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    };
    let p = root.join(rel);
    assert!(
        p.is_file(),
        "fixture `{rel}` missing at {}. A missing corpus is a failure, never a skip.",
        p.display()
    );
    p
}

fn conformance(rel: &str) -> PathBuf {
    path_in("conformance", rel)
}

fn engine_fx(name: &str) -> PathBuf {
    path_in("engine", &format!("{name}/document.pdf"))
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-m7-diag-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Every object key in a JSON document, at any depth.
fn keys(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, child) in map {
                out.push(k.clone());
                keys(child, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| keys(i, out)),
        _ => {}
    }
}

/// The field names that may only ever live in a diagnostics envelope.
///
/// Both the names this engine emits **and** the ones a well-meaning future change would reach
/// for. `01-CONTRACT.md` §4 names the classes — timings, memory, host, source paths — so the ban
/// is on the class, not on today's spelling of it.
const DIAGNOSTICS_CLASS_KEYS: [&str; 16] = [
    "diagnostics",
    "diagnostics_version",
    "wall_micros",
    "engine_version",
    "host",
    "hostname",
    "arch",
    "resident_bytes",
    "rss",
    "peak_memory",
    "input_path",
    "input_bytes",
    "duration",
    "duration_ms",
    "elapsed",
    "timestamp",
];

/// One covered `Stage`, with an input that reaches it and the stage name the envelope must report.
///
/// **A stage is not a subcommand**, and `Stage`'s own doc comment says so: ten subcommands map
/// onto five stages, because `markdown`, `html`, `overlay` and `tag` report under the phase
/// whose work they project from or write into a file, and `mcp` has no stage at all. Counting subcommands here was wrong twice
/// over — wrong population, wrong number — and v2-S17 replaced the count with a derivation.
struct Case {
    stage: &'static str,
    args: Vec<String>,
}

/// Every `Stage` this file's walk is expected to reach, **derived from the enum** rather than
/// counted by hand.
///
/// The hardcoded `4` this replaced was a floor set one below its own population, and its message
/// said so out loud — *"all four subcommands must be covered"* — while `Stage` had five variants
/// and the CLI had nine subcommands. A guard that reads the wrong thing also passes.
///
/// **`Stage::Verify` is outside the walk, and that is not a gap presented as a success.** `engine
/// verify` writes the verifier's bytes and composes nothing of its own, so there is no
/// engine-owned artifact here to walk;
/// `verify_relay.rs::a_grounded_claim_relays_the_verifier_bytes_verbatim` asserts that its stdout
/// is **byte-identical** to `ethos verify`'s, which is strictly stronger than *"no
/// diagnostics-class key appears in it"* — a byte-identical relay cannot have injected anything
/// at all. Walking those keys here would be asserting a property of the pinned Ethos binary and
/// reporting it as evidence about this engine.
///
/// **The match is exhaustive on purpose**, and it is the part a hardcoded number could never do:
/// a sixth `Stage` stops this file compiling until somebody decides which side of the line it
/// falls on. `EVERY_STAGE` is the one hand-written list, and the match is what forces a revisit
/// of it.
const EVERY_STAGE: [Stage; 5] = [
    Stage::Classify,
    Stage::Extract,
    Stage::Ground,
    Stage::GroundingCheck,
    Stage::Verify,
];

fn walked_here(stage: Stage) -> bool {
    match stage {
        Stage::Classify | Stage::Extract | Stage::Ground | Stage::GroundingCheck => true,
        Stage::Verify => false,
    }
}

fn stages_this_file_walks() -> Vec<&'static str> {
    EVERY_STAGE
        .iter()
        .copied()
        .filter(|stage| walked_here(*stage))
        .map(|stage| stage.as_str())
        .collect()
}

/// Build one case per covered stage, materializing the intermediate files `ground` and
/// `grounding-check` consume.
fn covered_stages(dir: &Path) -> Vec<Case> {
    let pdf = engine_fx("measured-ink-box");

    let repr = engine(&["extract", pdf.to_str().unwrap()]);
    assert_eq!(
        repr.status.code(),
        Some(0),
        "extract must succeed to set up"
    );
    let repr_path = dir.join("repr.json");
    std::fs::write(&repr_path, &repr.stdout).expect("write representation");

    let grounding = engine(&["ground", repr_path.to_str().unwrap()]);
    assert_eq!(grounding.status.code(), Some(0), "ground must succeed");
    let grounding_path = dir.join("grounding.json");
    std::fs::write(&grounding_path, &grounding.stdout).expect("write grounding");

    vec![
        Case {
            stage: "classify",
            args: vec!["classify".into(), pdf.display().to_string()],
        },
        Case {
            stage: "extract",
            args: vec!["extract".into(), pdf.display().to_string()],
        },
        Case {
            stage: "ground",
            args: vec!["ground".into(), repr_path.display().to_string()],
        },
        Case {
            stage: "grounding-check",
            args: vec![
                "grounding-check".into(),
                grounding_path.display().to_string(),
                "--source-artifact".into(),
                pdf.display().to_string(),
            ],
        },
    ]
}

fn as_args(c: &Case) -> Vec<&str> {
    c.args.iter().map(String::as_str).collect()
}

// -------------------------------------------------------------------------------------------
// 1. Off by default
// -------------------------------------------------------------------------------------------

/// **Nothing on stderr without the flag.**
///
/// Not "no diagnostics on stderr" — *nothing*. A default run of a subcommand that succeeded has
/// no reason to write there at all, and asserting emptiness catches a stray `eprintln!` as well
/// as a leaked envelope. (`ground` on a document with unmeasurable nodes legitimately writes its
/// omission note; the fixture here has measurable geometry, so this case is silent.)
#[test]
fn a_default_run_writes_nothing_to_stderr() {
    let dir = scratch("default");
    for case in covered_stages(&dir) {
        let out = engine(&as_args(&case));
        assert!(
            out.stderr.is_empty(),
            "`{}` wrote to stderr without --diagnostics: {}",
            case.stage,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// 2. The flag adds a line and moves no artifact byte
// -------------------------------------------------------------------------------------------

/// The flag is additive: one JSON object on stderr, and stdout byte-for-byte unchanged.
#[test]
fn the_flag_adds_one_stderr_line_and_changes_no_stdout_byte() {
    let dir = scratch("additive");
    for case in covered_stages(&dir) {
        let plain = engine(&as_args(&case));

        let mut with = as_args(&case);
        with.push("--diagnostics");
        let diag = engine(&with);

        assert_eq!(
            plain.stdout, diag.stdout,
            "`{}` produced different artifact bytes under --diagnostics. The flag is an \
             observation, not a mode.",
            case.stage
        );
        assert_eq!(
            plain.status.code(),
            diag.status.code(),
            "`{}` changed its exit code under --diagnostics",
            case.stage
        );

        let stderr = String::from_utf8(diag.stderr.clone()).expect("stderr is UTF-8");
        let line = stderr
            .lines()
            .next_back()
            .unwrap_or_else(|| panic!("`{}` emitted no diagnostics line", case.stage));
        let v: Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!(
                "`{}` diagnostics line is not JSON ({e}): {line}",
                case.stage
            )
        });

        assert_eq!(
            v["stage"], case.stage,
            "the envelope must name the subcommand that produced it"
        );
        assert_eq!(v["engine_version"], env!("CARGO_PKG_VERSION"));
        assert!(v["wall_micros"].is_u64(), "a timing is reported");
        assert!(
            v["input_path"].is_string(),
            "the input path is reported here and nowhere else"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Diagnostics survive the failure path too — including the one that produces no artifact.
///
/// A flag that silently does nothing when a run fails is worse than no flag: the failing run is
/// the one somebody turned it on to investigate.
#[test]
fn a_failing_run_still_reports_its_diagnostics() {
    // `corrupt-header-valid`, not `table-regular-grid`: v0.1 repairs the latter
    // (docs/01-CONTRACT.md §12), and this test needs a document that genuinely fails.
    let pdf = conformance("failure/corrupt-header-valid/document.pdf");
    let out = engine(&["classify", pdf.to_str().unwrap(), "--diagnostics"]);

    assert_eq!(
        out.status.code(),
        Some(2),
        "an unrepairable malformation exits 2"
    );
    assert!(
        out.stdout.is_empty(),
        "a failed run emits no artifact on stdout"
    );

    let stderr = String::from_utf8(out.stderr).expect("UTF-8");
    let line = stderr
        .lines()
        .next_back()
        .expect("a diagnostics line even on failure");
    let v: Value = serde_json::from_str(line).expect("the last stderr line is the envelope");
    assert_eq!(v["stage"], "classify");
    assert!(
        stderr.contains("engine: malformed"),
        "the named error is still reported: {stderr}"
    );
}

// -------------------------------------------------------------------------------------------
// 3. Determinism is unaffected
// -------------------------------------------------------------------------------------------

/// **Double-run byte identity still holds with diagnostics on.**
///
/// The M6 criterion is asserted for the default path; this asserts the flag cannot weaken it.
/// The diagnostics lines themselves are expected to differ, and the test says so rather than
/// leaving it to be discovered.
#[test]
fn two_runs_with_diagnostics_still_produce_identical_artifacts() {
    let dir = scratch("double");
    for case in covered_stages(&dir) {
        let mut with = as_args(&case);
        with.push("--diagnostics");

        let first = engine(&with);
        let second = engine(&with);

        assert_eq!(
            first.stdout, second.stdout,
            "`{}` is not byte-identical across two --diagnostics runs",
            case.stage
        );

        let path_of = |o: &Output| -> String {
            let s = String::from_utf8(o.stderr.clone()).unwrap();
            let line = s.lines().next_back().unwrap().to_string();
            let v: Value = serde_json::from_str(&line).unwrap();
            v["input_path"].as_str().unwrap().to_string()
        };
        assert_eq!(
            path_of(&first),
            path_of(&second),
            "the same input is named the same way"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The identity envelope is untouched: same `profile_sha256`, same representation fingerprint.
#[test]
fn diagnostics_do_not_reach_a_fingerprint() {
    let pdf = engine_fx("measured-ink-box");

    let plain: Value =
        serde_json::from_slice(&engine(&["extract", pdf.to_str().unwrap()]).stdout).unwrap();
    let diag: Value = serde_json::from_slice(
        &engine(&["extract", pdf.to_str().unwrap(), "--diagnostics"]).stdout,
    )
    .unwrap();

    assert_eq!(
        plain["identity"]["profile_sha256"], diag["identity"]["profile_sha256"],
        "observing a run must not change the profile it ran under"
    );
    assert_eq!(
        plain["binding"], diag["binding"],
        "nor the source or representation binding"
    );
    assert_eq!(plain, diag, "nor anything else");
}

// -------------------------------------------------------------------------------------------
// 4. No diagnostics-class name inside any artifact
// -------------------------------------------------------------------------------------------

/// **No volatile field name appears anywhere in any artifact, at any depth.**
///
/// Ethos property-tests its payload projection this way, and the shape is the point: checking
/// only that two runs match would pass for an artifact carrying a `host` field on a machine
/// where the host never changes. Walking keys catches it on one run.
#[test]
fn no_diagnostics_field_name_appears_in_any_artifact() {
    let dir = scratch("keys");
    let mut walked: Vec<&'static str> = Vec::new();

    for case in covered_stages(&dir) {
        let out = engine(&as_args(&case));
        let artifact: Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|e| panic!("`{}` stdout is not JSON: {e}", case.stage));

        let mut found = Vec::new();
        keys(&artifact, &mut found);
        assert!(!found.is_empty(), "`{}` produced no keys", case.stage);

        for banned in DIAGNOSTICS_CLASS_KEYS {
            assert!(
                !found.iter().any(|k| k == banned),
                "`{}` emits a `{banned}` key. Volatile data lives in the --diagnostics envelope \
                 and nowhere else (docs/01-CONTRACT.md §4); a field of that class inside an \
                 artifact breaks byte identity across hosts even when it holds on one.",
                case.stage
            );
        }
        walked.push(case.stage);
    }

    assert_eq!(
        walked,
        stages_this_file_walks(),
        "the walk must reach every `Stage` this file covers, and only those. The expected list is \
         derived from the `Stage` enum rather than counted, so a new variant fails to compile \
         here rather than passing silently against a stale number."
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Guard the guard: the key walk must actually find nested keys.
///
/// Without this, a `keys()` that quietly returned only top-level names would make the scan above
/// pass on every artifact while inspecting almost none of it.
#[test]
fn the_key_walk_reaches_nested_and_array_members() {
    let v: Value =
        serde_json::from_str(r#"{"a": {"b": {"wall_micros": 1}}, "c": [{"host": {"os": "x"}}]}"#)
            .unwrap();
    let mut found = Vec::new();
    keys(&v, &mut found);
    for expect in ["a", "b", "wall_micros", "c", "host", "os"] {
        assert!(found.iter().any(|k| k == expect), "missed `{expect}`");
    }
}
