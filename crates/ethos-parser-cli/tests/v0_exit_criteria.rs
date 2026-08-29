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
//! | `ci/gate.sh` runs exactly what CI runs | A local "green" that the remote would not give |
//!
//! The `--skip` and zero-filter rows are the ways this scheme goes hollow while still looking
//! complete. Six milestones ran with a deliberate `--skip` on the oracle test, which M6 removed;
//! re-adding one deletes the criterion in the process (`docs/05-MILESTONES.md` M6). And a filter
//! naming a test that was since renamed selects nothing at all, so libtest prints `ok. 0 passed`
//! and the job is green — with every box still ticked and every job still present.
//!
//! The last row is v2-S18's, and it answers a failure the rows above it cannot reach, because
//! every one of them assumes CI runs. **It did not, until 0.41.0 gave this repository a remote
//! at `docushell/ethos-parser`.** Before that there was no remote and no tag,
//! so every green any record claims was produced by hand with whichever checks somebody
//! remembered — and `cargo fmt --all --check` was red for four slices underneath four such
//! claims. `ci/gate.sh` is the answer to that, and a convenience script nobody checked against
//! the workflow would only move the problem one file along, so the script is guarded the same way
//! §5 is.
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

/// The workspace's member crate names, from `Cargo.toml`'s `members` array.
///
/// Parsed rather than listed, for the reason `no_job_filter_selects_zero_tests` gives: a list of
/// crates written into a test goes stale the moment a crate is added, and the failure is silent
/// because a scanner that misses a crate still finds plenty to scan. `crates/ethos-parser-office` was
/// missing from that list for twelve slices.
///
/// Deliberately a small string parser rather than `cargo metadata`: this file already reads
/// `ci.yml` and `03-V0-SCOPE.md` as text, the array is four lines of TOML, and shelling out to
/// cargo from inside a cargo test is a cost and a dependency for no extra truth.
fn workspace_members() -> Vec<String> {
    let manifest = read("Cargo.toml");
    let Some(at) = manifest.find("\n[workspace]\n") else {
        panic!("Cargo.toml has no `[workspace]` table");
    };
    let rest = &manifest[at..];
    let Some(open) = rest.find("members = [") else {
        panic!("the `[workspace]` table declares no `members`");
    };
    let body = &rest[open..];
    let Some(close) = body.find(']') else {
        panic!("`members` is unterminated");
    };

    body[..close]
        .split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|p| p.rsplit('/').next())
        .map(str::to_string)
        .collect()
}

/// Every name libtest can select, one per line, for every crate in the workspace.
///
/// A test's libtest name is its **module path joined to its function name** —
/// `content::tests::a_clip_path_is_not_a_ruling_line` for a unit test inside `content.rs`'s
/// `mod tests`, and plain `a_page_that_implies_no_grid_reports_an_empty_table_list` for one at the
/// top of an integration test file, because that file is its own crate root. Filters match on that
/// whole string, so that whole string is what this collects.
///
/// # Why bare `fn` names were not it
///
/// This scan used to collect every `fn` name it saw, which made two classes of filter
/// unrepresentable:
///
/// - A **module-path filter** can never occur inside a bare function name. `content::tests`,
///   `tables::` and `accuracy::` are three such filters in `.github/workflows/ci.yml` today, and
///   all three would have been reported dead the moment the guard could see them.
/// - A bare name matched whether or not it belonged to a `#[test]`, so a filter naming a private
///   helper read as live while selecting nothing — the exact failure this file exists to catch.
///
/// Reconstructing the libtest name fixes both, and it is not an approximation of what libtest
/// does: it is the string libtest prints and filters on.
///
/// # The rules, each exact rather than approximate
///
/// A file under `src/` is a module named after its stem, except `lib.rs` and `main.rs`, which are
/// the crate root and contribute no prefix. A file under `tests/` **is** a crate root, so it
/// contributes no prefix either — libtest does not prefix an integration test with its file name.
/// An inline `mod name {` enters a module; a `mod name;` **declaration** does not, because it
/// points at another file this walk reaches on its own, and treating it as an entry would
/// misattribute every test below it in the same file.
///
/// Inline modules are tracked one level deep, which is the whole depth this workspace uses: every
/// `mod name {` under `crates/*/src` and `crates/*/tests` sits at column zero. Should one ever be
/// nested, its tests are attributed without the outer prefix, and the failure mode is a filter
/// reported dead that is not — loud, and not the silence this file exists to break.
///
/// # Two assertions, because one of them is circular on its own
///
/// The reconstructed count is asserted equal to the number of `#[test]` attributes seen, so a
/// parser that quietly stopped understanding a file fails here rather than passing with a shorter
/// haystack. That equality holds trivially at zero, so the floor below it is what makes it mean
/// something: a scan that read nothing would satisfy the equality and report every token dead.
fn workspace_test_paths() -> String {
    // **The crate list is read from `Cargo.toml`'s `members`, not written here.** It used to be
    // written here, and it said four crates while the workspace had five: `ethos-parser-office` joined
    // at v2-S1 and this list did not. The consequence was a false *negative* in a guard — a job
    // filtering on an office test name would have been reported as matching no test at all,
    // because the scanner could not see the crate rather than because the test was missing. A
    // guard that cannot see a fifth of the workspace is a guard whose own comment is untrue.
    let members = workspace_members();
    assert!(
        members.len() >= 5,
        "found {} workspace member(s) in Cargo.toml: {members:?}. The parser is broken, not the \
         workspace — this test scans what it finds, so finding nothing would make it pass \
         having read nothing.",
        members.len()
    );

    let mut out = String::new();
    let mut attributes = 0usize;
    for crate_name in &members {
        for (sub, in_src) in [("src", true), ("tests", false)] {
            let dir = repo_root().join(format!("crates/{crate_name}/{sub}"));
            collect_test_paths(&dir, in_src, &mut out, &mut attributes);
        }
    }

    let found = out.lines().count();
    assert!(
        found > 1000,
        "the test-name scan reconstructed only {found} libtest name(s); it is broken, not the \
         workflow. This floor is what stops the equality below passing on an empty scan."
    );
    assert_eq!(
        found, attributes,
        "the scan reconstructed {found} libtest name(s) from {attributes} `#[test]` \
         attribute(s). Every attribute must yield exactly one name; a gap means the parser \
         stopped understanding a file and the haystack is quietly short."
    );

    out
}

/// Walk one directory, appending `module::path::function` for every `#[test]` found.
fn collect_test_paths(
    dir: &std::path::Path,
    in_src: bool,
    out: &mut String,
    attributes: &mut usize,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_test_paths(&path, in_src, out, attributes);
            continue;
        }
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let base = if in_src && stem != "lib" && stem != "main" {
            stem
        } else {
            String::new()
        };

        let src = std::fs::read_to_string(&path).expect("readable source");
        let mut module = base.clone();
        let mut pending = false;
        for line in src.lines() {
            let t = line.trim_start();
            if t.starts_with("#[test]") {
                pending = true;
                *attributes += 1;
                continue;
            }
            if let Some(name) = inline_module_name(t) {
                module = if base.is_empty() {
                    name
                } else {
                    format!("{base}::{name}")
                };
                continue;
            }
            if let Some(rest) = t.strip_prefix("fn ") {
                if pending {
                    if !module.is_empty() {
                        out.push_str(&module);
                        out.push_str("::");
                    }
                    out.push_str(rest.split(['(', '<']).next().unwrap_or(""));
                    out.push('\n');
                }
                pending = false;
            }
        }
    }
}

/// The name in `mod name {`, or `None`.
///
/// A `mod name;` declaration returns `None` on purpose: it points at another file, which the walk
/// above reaches on its own, and entering it here would misattribute every test below it.
fn inline_module_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line)
        .strip_prefix("mod ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        return None;
    }
    rest[name.len()..]
        .trim_start()
        .starts_with('{')
        .then_some(name)
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
/// So every filter token in every `cargo test` command in the workflow must occur inside some
/// name libtest can select. Substring filters are fine and intended — `v0-c14n` filters on
/// `c14n`, `float` and `quantize` — so the check is that the token occurs within a test's libtest
/// name, which is exactly what libtest matches on. [`workspace_test_paths`] argues what that name
/// is and why reconstructing it beats collecting bare `fn` names.
///
/// # The quoting hole this test was blind to for twelve slices
///
/// The parser required the command to begin `cargo test`. The `v1s1-gates` and `v1s7-table-gate`
/// matrices **quote** their commands — `run: "cargo test …"` — so the command began with a double
/// quote and the whole line was skipped before a single token was read. Five commands and fifteen
/// of the sixty filter tokens were invisible to the guard that exists to check exactly them, and
/// one of the fifteen was dead: `no_ruling_lines`, orphaned by v1-S2's rename of
/// `a_page_with_no_ruling_lines_reports_an_empty_table_list` and unnoticed for twenty-two commits.
///
/// The quoting is not incidental, which is the part worth keeping in mind. Three of those five
/// commands filter on a module path — `content::tests`, `tables::`, `accuracy::` — and a matrix
/// entry whose style quotes its `run:` is the same entry whose filters this scan could not have
/// matched anyway. The two halves of the defect arrived together.
///
/// # Why the repair is a rule and not a number
///
/// Stripping the quotes is one line, and one line is exactly what would leave the next variant —
/// single quotes, a leading `env FOO=bar`, a YAML block scalar — to be found by a human again.
/// So the assertion below is that **every line mentioning `cargo test` was parsed as a command**.
/// That does not rot, because it does not depend on anyone predicting the shape.
#[test]
fn no_job_filter_selects_zero_tests() {
    let test_paths = workspace_test_paths();

    let mut dead = Vec::new();
    let mut checked = 0usize;
    let mut commands = 0usize;
    let mut unparsed: Vec<&str> = Vec::new();

    let wf = workflow();
    for line in wf.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        let parsed = trimmed
            .strip_prefix("run: ")
            .map(|c| c.trim_matches(|q| q == '"' || q == '\''))
            .filter(|c| c.starts_with("cargo test"));

        let Some(cmd) = parsed else {
            if trimmed.contains("cargo test") {
                unparsed.push(trimmed);
            }
            continue;
        };
        commands += 1;

        let Some((_, tail)) = cmd.split_once(" -- ") else {
            continue;
        };
        for token in tail.split_whitespace() {
            // libtest flags, not filters.
            if token.starts_with("--") {
                continue;
            }
            checked += 1;
            if !test_paths.contains(token) {
                dead.push(token.to_string());
            }
        }
    }

    assert!(
        unparsed.is_empty(),
        "{} workflow line(s) run `cargo test` in a form this parser does not read, so every \
         filter on them is unchecked:\n  {}\n\n\
         This is how the guard went blind for twelve slices: two matrices quoted their commands \
         and every token in them was skipped in silence. Teach the parser the new shape rather \
         than letting the line stay invisible.",
        unparsed.len(),
        unparsed.join("\n  ")
    );

    assert!(
        commands >= 20,
        "only {commands} `cargo test` command(s) found in the workflow; twenty-two is the number \
         at v2-S13.1, so this means the matrices stopped being read"
    );

    // **The floor sits one command below the real total, and that is the whole argument for the
    // number.** Sixty tokens are checked at v2-S13.1 across twenty-two commands, and the largest
    // single command carries six, so losing any one job's filters entirely drops the count to
    // fifty-four and trips this. A floor far below the real number has stopped being a floor:
    // `an_injected_unknown_operator_stops_the_parse` carried `>= 12` against a real forty-four
    // while its corpus tripled underneath it, and v2-S12.1 is why that is written down here.
    assert!(
        checked >= 55,
        "only {checked} filter token(s) checked across the workflow; sixty is the number at \
         v2-S13.1 and the floor sits one command below it, so this says a job's filters stopped \
         being read rather than that a job was retired"
    );

    assert!(
        dead.is_empty(),
        "{} CI filter token(s) match no test libtest can select: {dead:?}\n\n\
         A filter that selects nothing makes its job report `ok. 0 passed` — green, and having \
         checked nothing. Rename the filter with the test, or delete it.",
        dead.len()
    );
}

/// One named job's block: from its key to the next top-level job key.
///
/// Scoped structurally rather than by a prefix convention. The workflow has more than one matrix,
/// and "every entry is claimed by a §5 line" is only true of the §5 one; scoping by name would
/// have made that rule quietly depend on nobody choosing an unfortunate job id.
fn job_block(job: &str) -> String {
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

    rest[..end].to_string()
}

/// The `- id:` entries inside one named job's block.
fn matrix_ids_of(job: &str) -> BTreeSet<String> {
    job_block(job)
        .lines()
        .filter_map(|l| l.trim().strip_prefix("- id: "))
        .map(|s| s.trim().to_string())
        .collect()
}

/// Every `run:` step in one job's block, as `(step name, command)`, in file order.
///
/// A step that declares no `name:` yields an empty one; every step this is used on has a name.
/// A block scalar (`run: |`) yields the literal `"|"` rather than its text — this file does not
/// interpret shell, and the two multi-line steps in `check` are identified by name instead.
///
/// The block scalar's body is **consumed by indentation** rather than scanned. Skipping that
/// would let a shell line inside a `run: |` be read as if it were YAML, which is the same class
/// of defect as `ci/forbidden-tokens.sh`'s test-region skip running to end of file: a scanner
/// that misreads one construct still finds plenty elsewhere and looks like it worked.
fn run_steps_of(job: &str) -> Vec<(String, String)> {
    let block = job_block(job);
    let mut out: Vec<(String, String)> = Vec::new();
    let mut name = String::new();
    let mut lines = block.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        // A new step. `- name: x` names it; `- uses: x` opens one that has no name.
        if let Some(rest) = trimmed.strip_prefix("- ") {
            name = rest
                .strip_prefix("name:")
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            continue;
        }

        let Some(cmd) = trimmed.strip_prefix("run:") else {
            continue;
        };
        let cmd = cmd.trim();

        if cmd == "|" {
            while let Some(next) = lines.peek() {
                let t = next.trim_start();
                if t.is_empty() || next.len() - t.len() > indent {
                    lines.next();
                } else {
                    break;
                }
            }
        }
        out.push((name.clone(), cmd.to_string()));
    }
    out
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

/// **The fuzz scaffolding exists and something compiles it**, so `v0-fuzz-smoke` has something
/// to run and no target can rot outside every gate.
///
/// The fuzz job cannot run here — `cargo-fuzz` needs nightly and a sanitizer — so this asserts
/// the parts that would make it fail for a boring reason: a missing target, a missing seed, an
/// unexecutable seeding script.
///
/// # The target list is read from the directory, and that is the repair v2-S12.1 exists for
///
/// It used to be `["open_and_classify", "open_and_extract"]`, written here by hand. v2-S12 added
/// a third target and did not grow the list — and because `Cargo.toml` excludes `fuzz/` from the
/// workspace, `cargo build --workspace` never compiled it either. `office_read` sat in the tree
/// for a whole slice with **nothing anywhere compiling it**: it could have stopped building
/// against the engine API and every job would have stayed green. That is the same shape as
/// `fuzz/Cargo.lock` sitting two slices stale — a thing outside every gate, found by a human
/// reading rather than by a test.
///
/// So the directory is the list. A fourth target is picked up without anyone remembering this
/// file, and the count below is the tripwire that makes adding one a decision rather than an
/// accident.
///
/// # Each target is asserted to drive *its own* entry point
///
/// Not one entry point for all three. `office_read` drives `ethos_parser_office::read`, and asserting
/// `Document::open_bytes` across the board would either fail here or push a lie into the target
/// to make it pass — which is the failure this test is supposed to catch, arriving through the
/// test.
#[test]
fn the_fuzz_target_and_seed_corpus_are_present() {
    let root = repo_root();
    let dir = root.join("fuzz/fuzz_targets");

    let mut targets: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    targets.sort();

    assert_eq!(
        targets.len(),
        3,
        "found {} fuzz target(s) in {}: {targets:?}. Three is the number this repository has \
         argued for — two on the PDF entry points (v0-M7) and one on the office router \
         (v2-S12). A fourth is a decision: it needs an entry point named below and a \
         `cargo fuzz build` line in `v0-fuzz-smoke`, and this line is what makes someone say so.",
        targets.len(),
        dir.display()
    );

    let wf = workflow();
    let fuzz_manifest = read("fuzz/Cargo.toml");

    for target in &targets {
        let p = dir.join(format!("{target}.rs"));
        let src = std::fs::read_to_string(&p).expect("readable");
        assert!(
            src.contains("fuzz_target!"),
            "{target} is not a libFuzzer target"
        );

        // The engine entry point this target is for. `office_read` reads packages through the
        // office router; everything else drives the PDF one, which is the stricter default and
        // the one `docs/03-V0-SCOPE.md` §5 names.
        let entry = if target == "office_read" {
            "ethos_parser_office::read"
        } else {
            "Document::open_bytes"
        };
        assert!(
            src.contains(entry),
            "{target} must drive `{entry}` (docs/03-V0-SCOPE.md §5); it does not"
        );

        // `cargo fuzz` builds what `fuzz/Cargo.toml` declares as a `[[bin]]`, so a target file
        // with no entry is a file cargo never looks at.
        assert!(
            fuzz_manifest.contains(&format!("name = \"{target}\"")),
            "`fuzz/fuzz_targets/{target}.rs` exists but `fuzz/Cargo.toml` declares no \
             `[[bin]] name = \"{target}\"`, so nothing builds it"
        );

        // **The load-bearing one.** `Cargo.toml` excludes `fuzz/` from the workspace on purpose,
        // so `cargo build --workspace` will never compile a fuzz target. The workflow naming it
        // is the only thing that does.
        assert!(
            wf.contains(&format!("cargo fuzz build {target}")),
            "no CI job builds `{target}`. `cargo build --workspace` excludes `fuzz/`, so a \
             target the workflow does not name is compiled by nothing at all — it can stop \
             building against the engine API and every job stays green. Add \
             `cargo fuzz build {target}` to `v0-fuzz-smoke`'s \"Build the fuzz targets\" step. \
             Building is not running: a target may be built without being given a time budget, \
             and `office_read` is."
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

/// The `check` job steps `ci/gate.sh` deliberately does not run.
///
/// The script's header argues each one; this array is the machine-checked half, and it is
/// asserted **complete in both directions** below — a step named here that no longer exists in
/// the workflow is as much a defect as a step in the workflow that is named neither here nor in
/// the script.
const GATE_SKIPS: [&str; 3] = [
    "Assert the toolchain pin is in force",
    "Build the oracle",
    "Record the oracle identity",
];

/// **`ci/gate.sh` runs what CI runs, and nothing CI does not.**
///
/// `ci.yml` did not execute at all until 0.41.0 gave this repository a remote — no remote, no
/// tag, 85 commits — and `ci/gate.sh` remains the only thing that can make "green" a fact
/// rather than a claim *before* a push, which is when it matters. That makes the
/// script's *fidelity* the whole value: a local gate that runs a subset manufactures exactly the
/// confidence that let `cargo fmt --all --check` stay red across four slices that each recorded
/// a green.
///
/// So this asserts set equality rather than mere presence, in both directions:
///
/// - **CI → script.** Every command the `check` job runs is in the script, unless it is named in
///   `GATE_SKIPS`. A sixth step added to that job is in neither place and fails here.
/// - **script → CI.** Every command the script runs is a command CI runs. Without this the
///   script becomes a second source of truth, and a local green starts meaning something the
///   remote does not enforce.
///
/// The two greps are `v0-exit-criteria` matrix entries rather than `check` steps, and they are
/// the only entries in any matrix that are not subsets of `cargo test --workspace`. They are read
/// out of the workflow rather than assumed, so deleting those jobs fails here too.
///
/// This test runs inside `cargo test --workspace`, which is the script's own step 6. Running the
/// gate therefore proves the gate still matches the workflow.
#[test]
fn the_local_gate_runs_what_ci_runs() {
    let script = repo_root().join("ci/gate.sh");
    assert!(script.is_file(), "ci/gate.sh is missing");
    let gate = read("ci/gate.sh");

    let steps = run_steps_of("check");

    // Guard the guard. If the step parser silently found nothing, the set comparison below would
    // still be a comparison — of two nearly empty sets — and would pass for the wrong reason.
    for expected in ["fmt", "clippy", "build", "test", "deny"] {
        assert!(
            steps.iter().any(|(n, _)| n == expected),
            "the `check` job step parser found no step named `{expected}`; it found {:?}. \
             The parser has drifted from the workflow's shape.",
            steps.iter().map(|(n, _)| n).collect::<Vec<_>>()
        );
    }

    // A skip that names a step which no longer exists excuses nothing, and hides the fact that
    // it excuses nothing.
    for skip in GATE_SKIPS {
        assert!(
            steps.iter().any(|(n, _)| n == skip),
            "GATE_SKIPS names `{skip}`, which is not a step in the `check` job any more. \
             Remove it here and from ci/gate.sh's header, or restore the step."
        );
    }

    // What CI runs that the script is expected to run: every `check` step except the skips …
    let mut want: BTreeSet<String> = steps
        .iter()
        .filter(|(n, _)| !GATE_SKIPS.contains(&n.as_str()))
        .map(|(_, c)| c.clone())
        .collect();

    // … plus the two greps, taken from the workflow rather than written in here.
    for mode in ["confidence", "verification"] {
        let cmd = format!("ci/forbidden-tokens.sh {mode}");
        assert!(
            workflow().contains(&cmd),
            "no CI job runs `{cmd}` any more. The grep criteria are the only gates that are not \
             subsets of `cargo test --workspace`; if one is genuinely gone, remove it from \
             ci/gate.sh too."
        );
        want.insert(cmd);
    }

    // What the script runs. Its commands sit at column 0 — every other line is a comment, the
    // `announce` helper, or shell — so this reads the script exactly as bash will.
    let got: BTreeSet<String> = gate
        .lines()
        .filter(|l| l.starts_with("cargo ") || l.starts_with("ci/"))
        .map(|l| l.trim_end().to_string())
        .collect();

    assert_eq!(
        got,
        want,
        "ci/gate.sh and .github/workflows/ci.yml have drifted.\n  \
         only in ci.yml:   {:?}\n  only in ci/gate.sh: {:?}\n\
         The commands must match character for character — CI is the authority, and a command \
         spelled differently here is a different command. If a `check` step genuinely should not \
         run locally, name it in GATE_SKIPS and argue it in ci/gate.sh's header.",
        want.difference(&got).collect::<Vec<_>>(),
        got.difference(&want).collect::<Vec<_>>(),
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&script)
            .expect("stat")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "ci/gate.sh is not executable ({mode:o}); it is documented as `ci/gate.sh`"
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
