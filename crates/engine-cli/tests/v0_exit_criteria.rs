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

//! **§5 is CI, and that is checked** (`docs/05-MILESTONES.md` M7 acceptance 1).
//!
//! `docs/03-V0-SCOPE.md` §5 opens with *"Every line is a CI job, not a judgement call."* That
//! sentence is itself a judgement call unless something verifies it, so this does:
//!
//! | Assertion | Catches |
//! | --- | --- |
//! | Every §5 line is ticked and names a job | A criterion nobody wired up |
//! | Every named job exists in the workflow | A renamed or deleted job, or a typo in §5 |
//! | Every matrix entry is named by a criterion | A job with nothing behind it in the doc |
//! | No `--skip` anywhere in CI | The cheapest way to make a red criterion green |
//! | No job filter matches zero tests | The *quietest* way — a job that checks nothing and says `ok` |
//!
//! The last two are the ways this scheme goes hollow while still looking complete. Six milestones
//! ran with a deliberate `--skip` on the oracle test, which M6 removed; re-adding one deletes the
//! criterion in the process (`docs/05-MILESTONES.md` M6). And a filter naming a test that was
//! since renamed selects nothing at all, so libtest prints `ok. 0 passed` and the job is green —
//! with every box still ticked and every job still present.
//!
//! # Why it does not parse YAML
//!
//! Because a YAML dependency to read one file the tests never execute is a dependency in
//! `cargo deny`'s graph forever. What matters is whether a job *id* is present, and substring
//! matching answers that exactly — a job named in §5 either appears in the workflow or does not.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} unreadable: {e}", p.display()))
}

/// The `## 5.` … `## 6.` slice of the scope document, checklist only.
fn section_5() -> String {
    let doc = read("docs/03-V0-SCOPE.md");
    let start = doc
        .find("## 5. Exit criteria")
        .expect("03-V0-SCOPE.md must have a §5");
    let rest = &doc[start..];
    let end = rest
        .find("### 5.1")
        .expect("§5 must be followed by the 5.1 job table");
    rest[..end].to_string()
}

/// Every checklist line in §5, as `(ticked, text)`.
///
/// Continuation lines are folded in, because a criterion's `— CI job \`x\`` annotation is usually
/// on the line after the text it belongs to.
fn criteria() -> Vec<(bool, String)> {
    let mut out: Vec<(bool, String)> = Vec::new();
    for line in section_5().lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("- [x] ") {
            out.push((true, rest.to_string()));
        } else if let Some(rest) = t.strip_prefix("- [ ] ") {
            out.push((false, rest.to_string()));
        } else if !t.is_empty() && !t.starts_with('#') && !t.starts_with('`') {
            if let Some(last) = out.last_mut() {
                last.1.push(' ');
                last.1.push_str(t);
            }
        }
    }
    out
}

/// Job ids named in a criterion, from `` CI job `x` `` / `` CI jobs `x` and `y` ``.
fn jobs_named_in(text: &str) -> Vec<String> {
    let Some(at) = text.find("CI job") else {
        return Vec::new();
    };
    text[at..]
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

fn workflow() -> String {
    read(".github/workflows/ci.yml")
}

// -------------------------------------------------------------------------------------------
// The assertions
// -------------------------------------------------------------------------------------------

/// **Every §5 line is ticked and names at least one CI job.**
#[test]
fn every_exit_criterion_is_ticked_and_names_a_job() {
    let criteria = criteria();

    assert_eq!(
        criteria.len(),
        15,
        "§5 should carry 15 criteria; found {}. If a criterion was added, it needs a job — and \
         if one was removed, say why in the commit.",
        criteria.len()
    );

    let unticked: Vec<&str> = criteria
        .iter()
        .filter(|(ok, _)| !ok)
        .map(|(_, t)| t.as_str())
        .collect();
    assert!(
        unticked.is_empty(),
        "v0 is declared complete, so every §5 box is ticked. Still open:\n  {}",
        unticked.join("\n  ")
    );

    let unnamed: Vec<&str> = criteria
        .iter()
        .filter(|(_, t)| jobs_named_in(t).is_empty())
        .map(|(_, t)| t.as_str())
        .collect();
    assert!(
        unnamed.is_empty(),
        "{} criterion(s) name no CI job. A ticked box with no job behind it is the judgement \
         call §5 exists to replace:\n  {}",
        unnamed.len(),
        unnamed.join("\n  ")
    );
}

/// **Every job §5 names exists in the workflow.**
#[test]
fn every_named_job_exists_in_the_workflow() {
    let wf = workflow();
    let mut missing = Vec::new();

    for (_, text) in criteria() {
        for job in jobs_named_in(&text) {
            // A matrix entry (`- id: v0-…`) or a top-level job key (`  deny-policy-is-enforced:`).
            let as_matrix_entry = format!("id: {job}\n");
            let as_job_key = format!("\n  {job}:\n");
            if !wf.contains(&as_matrix_entry) && !wf.contains(&as_job_key) {
                missing.push(job);
            }
        }
    }

    assert!(
        missing.is_empty(),
        "§5 names {} job(s) that do not exist in .github/workflows/ci.yml: {missing:?}\n\n\
         Either the job was renamed and §5 was not, or a criterion was ticked against a job \
         nobody wrote.",
        missing.len()
    );
}

/// **No job's test filter matches nothing.**
///
/// The quietest failure available to this whole scheme: `cargo test -- a_renamed_test` selects
/// zero tests, libtest reports `ok. 0 passed`, and the job goes green having checked nothing.
/// Every criterion would still be ticked, every job would still exist, and the gate would be
/// hollow.
///
/// So every filter token in every matrix `run` must appear inside some test function's name.
/// Substring filters are fine and intended — `v0-c14n` filters on `c14n`, `float` and `quantize`
/// — so the check is that the token occurs within an `fn` name somewhere, which is exactly what
/// libtest matches on.
#[test]
fn no_job_filter_selects_zero_tests() {
    // Every test function name in the workspace: `#[test]` fns in `tests/`, and unit tests in
    // `src/`, since jobs filter across both.
    let mut fn_names = String::new();
    let mut walk_dirs: Vec<PathBuf> = Vec::new();
    for crate_name in [
        "engine-core",
        "engine-pdf",
        "engine-grounding",
        "engine-cli",
    ] {
        walk_dirs.push(repo_root().join(format!("crates/{crate_name}/src")));
        walk_dirs.push(repo_root().join(format!("crates/{crate_name}/tests")));
    }

    fn collect(dir: &std::path::Path, out: &mut String) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let src = std::fs::read_to_string(&path).expect("readable source");
                for line in src.lines() {
                    if let Some(rest) = line.trim_start().strip_prefix("fn ") {
                        out.push_str(rest.split(['(', '<']).next().unwrap_or(""));
                        out.push('\n');
                    }
                }
            }
        }
    }
    for dir in &walk_dirs {
        collect(dir, &mut fn_names);
    }
    assert!(
        fn_names.len() > 2000,
        "the test-name scan found almost nothing ({} bytes); it is broken, not the workflow",
        fn_names.len()
    );

    let mut dead = Vec::new();
    let mut checked = 0usize;

    for line in workflow().lines() {
        let Some(cmd) = line.trim().strip_prefix("run: ") else {
            continue;
        };
        if !cmd.starts_with("cargo test") {
            continue;
        }
        let Some((_, tail)) = cmd.split_once(" -- ") else {
            continue;
        };
        for token in tail.split_whitespace() {
            // libtest flags, not filters.
            if token.starts_with("--") {
                continue;
            }
            checked += 1;
            if !fn_names.contains(token) {
                dead.push(token.to_string());
            }
        }
    }

    assert!(
        checked >= 30,
        "only {checked} filter token(s) found across the workflow; the matrix is not being read"
    );
    assert!(
        dead.is_empty(),
        "{} CI filter token(s) match no test function: {dead:?}\n\n\
         A filter that selects nothing makes its job report `ok. 0 passed` — green, and having \
         checked nothing. Rename the filter with the test, or delete it.",
        dead.len()
    );
}

/// The `- id:` entries inside one named job's block.
///
/// Scoped structurally — from the job key to the next top-level job key — rather than by a
/// prefix convention. The workflow has more than one matrix, and "every entry is claimed by a §5
/// line" is only true of the §5 one; scoping by name would have made that rule quietly depend on
/// nobody choosing an unfortunate job id.
fn matrix_ids_of(job: &str) -> BTreeSet<String> {
    let wf = workflow();
    let start = wf
        .find(&format!("\n  {job}:\n"))
        .unwrap_or_else(|| panic!("the workflow must define a `{job}` job"));
    let rest = &wf[start + 1..];

    // The next line that is a top-level job key: two spaces, a name, a colon, end of line.
    let end = rest
        .match_indices('\n')
        .find(|(i, _)| {
            let line = rest[i + 1..].lines().next().unwrap_or("");
            line.starts_with("  ")
                && !line.starts_with("   ")
                && line.trim_end().ends_with(':')
                && !line.trim_start().starts_with('#')
                && line.trim_start().split(':').next().is_some_and(|k| {
                    !k.is_empty()
                        && k.chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                })
        })
        .map(|(i, _)| i + 1)
        .unwrap_or(rest.len());

    rest[..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- id: "))
        .map(|s| s.trim().to_string())
        .collect()
}

/// **Every §5 matrix entry is claimed by a criterion.**
///
/// The other direction, and it matters just as much: a job with no criterion behind it is
/// runner time spent on something nobody agreed was a gate, and it drifts unnoticed because
/// nothing points at it.
///
/// Scoped to `v0-exit-criteria`. The workflow also carries `v01-gates`, which proves the roadmap
/// row *after* v0 — those are not §5 criteria, v0's map does not move to accommodate them, and
/// `the_v01_gates_exist` below keeps them from being deleted quietly instead.
#[test]
fn every_matrix_job_is_claimed_by_a_criterion() {
    let matrix_ids = matrix_ids_of("v0-exit-criteria");

    assert!(
        matrix_ids.len() >= 10,
        "found only {} matrix entries; the §5 job matrix is missing or was renamed",
        matrix_ids.len()
    );

    let claimed: BTreeSet<String> = criteria()
        .iter()
        .flat_map(|(_, t)| jobs_named_in(t))
        .collect();

    let orphans: Vec<&String> = matrix_ids.difference(&claimed).collect();
    assert!(
        orphans.is_empty(),
        "{} exit-criteria job(s) are not named by any §5 line: {orphans:?}\n\n\
         Every job in that matrix is there to prove a criterion. One that proves nothing named \
         should either be documented in §5 or removed.",
        orphans.len()
    );
}

/// **The v0.1 gates exist**, and v0's map is untouched by them.
///
/// v0 is frozen: `docs/03-V0-SCOPE.md` §5 is fifteen criteria and stays fifteen. The roadmap row
/// after it — verification by shell-out, encoding detection, the xref decision — gets its own
/// matrix, and this is what stops that matrix quietly emptying out. Without it, deleting a v0.1
/// job would fail nothing at all, because §5 does not mention them and never should.
#[test]
fn the_v01_gates_exist() {
    let ids = matrix_ids_of("v01-gates");
    for expected in ["v01-verify-relay", "v01-encoding", "v01-xref-decision"] {
        assert!(
            ids.contains(expected),
            "the v0.1 gate `{expected}` is missing from the workflow; found {ids:?}"
        );
    }

    // And it really is a separate matrix — a v0.1 gate that drifted into the §5 one would make
    // `every_matrix_job_is_claimed_by_a_criterion` demand a criterion that does not exist.
    let v0 = matrix_ids_of("v0-exit-criteria");
    assert!(
        v0.is_disjoint(&ids),
        "the v0 and v0.1 matrices must not share entries: {:?}",
        v0.intersection(&ids).collect::<Vec<_>>()
    );
    assert_eq!(
        v0.len(),
        14,
        "docs/03-V0-SCOPE.md §5's matrix is fifteen criteria across fourteen entries (fuzz and \
         mutation share a line); v0 is frozen and this number does not move for v0.1 work"
    );
}

/// **No `--skip` anywhere in CI.**
///
/// M6 removed the last one and said why: *"Re-adding a `--skip` here to get a green build would
/// delete the only thing that proves this engine and the verifier read an artifact the same
/// way."* This is that sentence, enforced.
#[test]
fn no_ci_job_skips_a_test() {
    let wf = workflow();
    let offenders: Vec<(usize, &str)> = wf
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with('#'))
        .filter(|(_, l)| l.contains("--skip"))
        .map(|(n, l)| (n + 1, l.trim()))
        .collect();

    assert!(
        offenders.is_empty(),
        "`--skip` appears in CI:\n  {}\n\n\
         A skipped test is a criterion that stopped being checked while still looking green. \
         If a test genuinely cannot run in CI, that is a finding to fix or to declare, not to \
         filter out.",
        offenders
            .iter()
            .map(|(n, l)| format!("ci.yml:{n}: {l}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// **The fuzz scaffolding exists**, so `v0-fuzz-smoke` has something to run.
///
/// The fuzz job cannot run here — `cargo-fuzz` needs nightly and a sanitizer — so this asserts
/// the parts that would make it fail for a boring reason: a missing target, a missing seed, an
/// unexecutable seeding script.
#[test]
fn the_fuzz_target_and_seed_corpus_are_present() {
    let root = repo_root();

    for target in ["open_and_classify", "open_and_extract"] {
        let p = root.join(format!("fuzz/fuzz_targets/{target}.rs"));
        assert!(p.is_file(), "fuzz target missing: {}", p.display());
        let src = std::fs::read_to_string(&p).expect("readable");
        assert!(
            src.contains("fuzz_target!"),
            "{target} is not a libFuzzer target"
        );
        assert!(
            src.contains("Document::open_bytes"),
            "{target} must drive the PDF entry point (docs/03-V0-SCOPE.md §5)"
        );
    }

    let seeds = root.join("fuzz/seeds");
    let count = std::fs::read_dir(&seeds)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", seeds.display()))
        .count();
    assert!(
        count >= 4,
        "only {count} committed fuzz seed(s); libFuzzer starting from nothing spends its whole \
         budget rediscovering `%PDF`"
    );

    let script = root.join("fuzz/seed-corpus.sh");
    assert!(script.is_file(), "fuzz/seed-corpus.sh is missing");

    // The workflow invokes it directly, so the executable bit is part of the gate working.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&script)
            .expect("stat")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "fuzz/seed-corpus.sh is not executable ({mode:o}); the fuzz job runs it as a command"
        );
    }
}

/// **The grep gate exists and is executable**, for the same reason.
#[test]
fn the_forbidden_token_gate_is_runnable() {
    let script = repo_root().join("ci/forbidden-tokens.sh");
    assert!(script.is_file(), "ci/forbidden-tokens.sh is missing");

    let src = std::fs::read_to_string(&script).expect("readable");
    for mode in ["confidence", "verification"] {
        assert!(
            src.contains(mode),
            "the gate does not handle the `{mode}` mode the workflow asks it for"
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&script)
            .expect("stat")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "ci/forbidden-tokens.sh is not executable ({mode:o}); two CI jobs run it directly"
        );
    }
}

/// Guard the guard: the §5 parser must actually find the criteria and their annotations.
#[test]
fn the_scope_parser_reads_the_checklist_it_claims_to() {
    let c = criteria();
    assert!(!c.is_empty(), "the §5 parser found no criteria at all");

    // A known line, folded across its continuation, with its job annotation attached.
    let bound = c
        .iter()
        .find(|(_, t)| t.contains("Classification bound test"))
        .expect("the bound criterion should be found");
    assert!(
        jobs_named_in(&bound.1).contains(&"v0-classify-bound".to_string()),
        "continuation folding is broken — the annotation on the next line was lost: {}",
        bound.1
    );

    // And the one criterion that names two jobs.
    let fuzz = c
        .iter()
        .find(|(_, t)| t.contains("cargo-fuzz"))
        .expect("the fuzz criterion should be found");
    let named = jobs_named_in(&fuzz.1);
    assert!(
        named.contains(&"v0-fuzz-smoke".to_string())
            && named.contains(&"v0-fixture-mutation".to_string()),
        "a criterion naming two jobs must yield both: {named:?}"
    );
}
