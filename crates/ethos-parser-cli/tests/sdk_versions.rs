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

//! **The SDK version strings are the workspace version, asserted from the side that always runs.**
//!
//! Both SDKs already carry this assertion — `packages/node/test/cli-surface.test.js` and
//! `packages/python/tests/test_cli_surface.py` — and both say so in a comment. At v2-S15 the four
//! numbers were:
//!
//! | file | value |
//! | --- | --- |
//! | `Cargo.toml` `[workspace.package]` | `0.42.1` |
//! | `packages/python/src/ethos_parser/__init__.py` | `0.36.1` |
//! | `packages/node/src/index.js` | `0.36.1` |
//! | `packages/node/package.json` | `0.36.3` |
//!
//! Six minor versions of drift behind three guards, and the node one disagreed with its own
//! `package.json` inside a single package. The guards were not wrong — they were never executed,
//! because nothing in `.github/workflows/ci.yml` had ever run `node --test` or `pytest`.
//!
//! That job now exists. This file is the second half of the fix and the reason it is in Rust: a
//! guard in a suite that CI does not run is indistinguishable from no guard, and the Rust suite is
//! the one this repository has never failed to execute. If the SDK job is ever removed, retargeted
//! at the wrong directory, or silently skipped because a runner has no Python, THIS still fails.
//!
//! It reads the four files as text rather than importing anything, so it needs no Node, no Python,
//! and no built binary — which is precisely what makes it unskippable.

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

/// The one value every other version string has to equal.
///
/// Matched on a line of its own under `[workspace.package]`, the same shape both SDK suites match
/// (`/^version = "([^"]+)"$/m`). Asserting the match is unique is not pedantry: a second top-level
/// `version = "…"` line appearing in this file would silently change which number three guards and
/// two SDKs are pinned to.
fn workspace_version() -> String {
    let cargo = read("Cargo.toml");
    let hits: Vec<&str> = cargo
        .lines()
        .filter_map(|l| l.strip_prefix("version = \""))
        .filter_map(|r| r.strip_suffix('"'))
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "Cargo.toml must carry exactly one top-level `version = \"…\"` line; found {}: {hits:?}. \
         Both SDK suites match this with the same anchored regex, so more than one makes the \
         number they pin to depend on line order.",
        hits.len()
    );
    hits[0].to_string()
}

/// Pull one quoted value out of a file, by the exact line prefix that declares it.
fn declared(rel: &str, prefix: &str) -> String {
    let text = read(rel);
    let hit = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix(prefix))
        .unwrap_or_else(|| panic!("{rel} has no line beginning `{prefix}`"));
    hit.split('"')
        .nth(1)
        .unwrap_or_else(|| panic!("{rel}: `{prefix}` line carries no quoted value: {hit}"))
        .to_string()
}

/// **Every SDK version string is the workspace version.**
///
/// One assertion per file rather than a single combined one, so a failure names the file that
/// drifted instead of reporting that some unspecified pair disagrees.
#[test]
fn every_sdk_version_is_the_workspace_version() {
    let want = workspace_version();

    let python = declared(
        "packages/python/src/ethos_parser/__init__.py",
        "__version__ = ",
    );
    assert_eq!(
        python, want,
        "packages/python/src/ethos_parser/__init__.py says {python}, the workspace says {want}. \
         An SDK claiming a version the engine does not is the same class of lie `parser_version` \
         exists to prevent."
    );

    let node_src = declared("packages/node/src/index.js", "export const version = ");
    assert_eq!(
        node_src, want,
        "packages/node/src/index.js says {node_src}, the workspace says {want}."
    );

    let node_manifest = declared("packages/node/package.json", "\"version\": ");
    assert_eq!(
        node_manifest, want,
        "packages/node/package.json says {node_manifest}, the workspace says {want}. This one \
         disagreed with `src/index.js` INSIDE THE SAME PACKAGE at v2-S15, which is the shape a \
         release bump leaves when it touches the manifest and not the source."
    );
}

/// **The SDK suites are wired into CI**, which is the half that was actually missing.
///
/// The version strings above are a symptom; an unrun guard is the disease. This asserts the
/// workflow still reaches both suites, so deleting them fails here rather than going quiet for
/// six minor versions the way it did before.
///
/// Two links in the chain, because the command lives in a script: `ci.yml` must invoke
/// `ci/sdk-suites.sh`, and that script must run both suites. Asserting only the first would pass
/// against a script that had been emptied.
#[test]
fn ci_runs_both_sdk_suites() {
    // Only lines that EXECUTE count. A first draft of this test matched the whole workflow for
    // `node --test` and passed with the step deleted, because the comment above that step quotes
    // the command — a guard reading its own prose as evidence, which is the exact failure class
    // this file is about. So: strip comments before looking.
    let executable = |text: &str| -> String {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let wf = executable(&read(".github/workflows/ci.yml"));
    assert!(
        wf.contains("ci/sdk-suites.sh"),
        "no non-comment line of .github/workflows/ci.yml invokes `ci/sdk-suites.sh`. Both SDKs \
         carry a version guard that is worth exactly nothing unless CI runs it — that is how \
         0.36.1 and 0.42.0 coexisted behind three assertions."
    );

    // `ci/gate.sh` too: `the_local_gate_runs_what_ci_runs` already asserts the two carry the same
    // commands, so this is belt and braces — but a local gate that skipped the SDK suites would
    // let the drift come back and be green locally right up until the push.
    let gate = executable(&read("ci/gate.sh"));
    assert!(
        gate.contains("ci/sdk-suites.sh"),
        "ci/gate.sh does not run `ci/sdk-suites.sh`, so a local green would not cover the SDK \
         version guards."
    );

    let script = executable(&read("ci/sdk-suites.sh"));
    for (what, needle) in [
        ("the node SDK suite", "node --test"),
        ("the python SDK suite", "-m pytest"),
    ] {
        assert!(
            script.contains(needle),
            "ci/sdk-suites.sh no longer runs {what} (looked for `{needle}` on a non-comment \
             line). The workflow invoking an empty script is the same unrun guard in a new place."
        );
    }
}
