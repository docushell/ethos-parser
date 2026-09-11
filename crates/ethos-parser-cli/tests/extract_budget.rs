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

//! **`extract --max-pages` — the only bound a caller has on what one extract costs** (v2-S15).
//!
//! `ExtractArgs` carried exactly one field, the path. `classify` has had `--sample-pages` since
//! v0, so classification was bounded and caller-tunable while extraction — the stage that
//! actually retains every page — was neither, and `profile.page_budget` defaults to
//! `PageBudget::Unlimited`. A host handing this untrusted input could not lower it.
//!
//! That matters because peak memory tracks PAGE COUNT, not file size. Measured at v2-S15 on the
//! release binary, median of three:
//!
//! | pages | file | peak RSS |
//! | --- | --- | --- |
//! | 6 | 138K | 17.6 MiB |
//! | 28 | 1.5M | 124.2 MiB |
//! | 80 | 1.4M | 411.6 MiB |
//! | 120 | 1.5M | 566.1 MiB |
//!
//! Two 1.5 MB documents, 4.5x apart on page count alone — about 4.7 MiB per page. And with the
//! flag, on the 120-page document: 558.0 MiB unbounded, 299.1 at `--max-pages 64`, 184.7 at 32,
//! 70.7 at 8.
//!
//! Re-measured at 97fa562, the same document reads 563.5 MiB unbounded, 307.4 at 64, 188.1 at 32
//! and 73.2 at 8 — so the ladder reproduces, and the per-page figure is a corpus median whose
//! real range was 3.20 to 9.07 MiB/page. The flag's floor is what the v2-S15 table could not show:
//! `--max-pages 0` costs 45.3 MiB here and 224.2 MiB on a 733-page document, because the
//! structure tree is read before the budget is consulted.
//!
//! Every figure above is PRE-Arc. Role-path sharing (58a1342) then took the same 120-page document
//! to 488.0 MiB unbounded and the 733-page one to 4665.6, byte-identically, without moving the
//! floor: 45.2 and 221.4 MiB at `--max-pages 0`. The flag's arithmetic is unchanged; only the
//! coefficient it multiplies is smaller. c14n then stopped copying the payload's largest field,
//! taking the two documents to 403.7 and 3736.2 MiB with the floor still at 44.9 and 222.1.
//! `docs/measurements/memory-ceiling/`.
//!
//! The mechanism is not new — `PageBudget::AtMost` has always quarantined the pages past the
//! budget and declared `resource-limit-pages`. What was missing was any way to reach it. So these
//! tests assert the wiring and, more importantly, that a bounded run is **legible**: fewer pages
//! is not the same claim as fewer pages in the document, and an artifact that shipped the first
//! while implying the second would be the exact failure this engine's limitation vocabulary
//! exists to prevent.
//!
//! `irs-fw9.pdf` is six pages and committed in-tree, so this suite is cheap and needs no large
//! document to make its point.

use std::path::PathBuf;
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn root(name: &str) -> PathBuf {
    let m: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest"),
    )
    .expect("valid JSON");
    let decl = &m["roots"][name];
    if let Some(v) = decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        return PathBuf::from(v);
    }
    repo_root().join(decl["default"].as_str().expect("default"))
}

/// Six pages, committed here rather than in the Ethos tree.
fn fixture() -> PathBuf {
    root("gate").join("irs-fw9.pdf")
}

const PAGES_IN_FIXTURE: usize = 6;

fn extract(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("extract")
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn representation(out: &Output) -> serde_json::Value {
    assert_eq!(
        out.status.code(),
        Some(0),
        "extract failed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    v["representation"].clone()
}

fn page_count(r: &serde_json::Value) -> usize {
    r["pages"].as_array().expect("pages").len()
}

fn limitation_codes(r: &serde_json::Value) -> Vec<String> {
    r["assurance"]["limitations"]
        .as_array()
        .expect("limitations")
        .iter()
        .map(|l| l["code"].as_str().expect("code").to_string())
        .collect()
}

const BUDGET_CODE: &str = "resource-limit-pages";

/// **The default is unchanged**, which is the assertion that lets the flag be additive.
#[test]
fn without_the_flag_every_page_is_read_and_nothing_is_declared() {
    let r = representation(&extract(&[fixture().to_str().unwrap()]));
    assert_eq!(
        page_count(&r),
        PAGES_IN_FIXTURE,
        "an unbounded extract must read the whole document"
    );
    assert!(
        !limitation_codes(&r).contains(&BUDGET_CODE.to_string()),
        "a run that hit no budget must not declare one; codes: {:?}",
        limitation_codes(&r)
    );
}

/// **A bound is honoured, and it is declared rather than silently applied.**
#[test]
fn a_budget_stops_the_run_and_says_so() {
    let r = representation(&extract(&["--max-pages", "2", fixture().to_str().unwrap()]));

    assert_eq!(page_count(&r), 2, "the budget must actually bind");

    let codes = limitation_codes(&r);
    assert!(
        codes.contains(&BUDGET_CODE.to_string()),
        "a bounded run must declare `{BUDGET_CODE}`. Emitting two pages of a six-page document \
         without saying so would be indistinguishable from a two-page document, which is the \
         one reading a consumer must never be given; codes: {codes:?}"
    );

    // The detail must name both numbers, or a reader cannot tell how much was left out.
    let detail = r["assurance"]["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["code"] == BUDGET_CODE)
        .and_then(|l| l["detail"].as_str())
        .expect("the limitation carries a detail")
        .to_string();
    assert!(
        detail.contains('2') && detail.contains(&PAGES_IN_FIXTURE.to_string()),
        "the detail must name the budget and the document's page count so the omission is \
         quantified, not merely flagged; got: {detail}"
    );
}

/// **A bounded artifact and an unbounded one are not comparable, and the identity says so.**
///
/// `page_budget` is inside the profile, so bounding the run moves `profile_sha256` — the same
/// discipline `classify --sample-pages` follows. Without this a consumer could diff two artifacts
/// of the same document and read a budget difference as a content difference.
#[test]
fn a_budget_moves_the_profile_hash() {
    let bounded = representation(&extract(&["--max-pages", "2", fixture().to_str().unwrap()]));
    let unbounded = representation(&extract(&[fixture().to_str().unwrap()]));

    let hash = |r: &serde_json::Value| {
        r["identity"]["profile_sha256"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_ne!(
        hash(&bounded),
        hash(&unbounded),
        "a bounded run must not claim the profile an unbounded one produced"
    );
}

/// A budget that cannot bind changes nothing — including the page count and the limitation set.
///
/// The interesting half is that this is NOT the same artifact as the unbounded one: the profile
/// carries `AtMost(99)` rather than `Unlimited`, so the hash moves even though no page was
/// dropped. That is correct and worth pinning: the identity describes the profile that ran, not
/// the outcome it happened to produce.
#[test]
fn a_budget_larger_than_the_document_drops_nothing_but_is_still_a_different_profile() {
    let r = representation(&extract(&[
        "--max-pages",
        "99",
        fixture().to_str().unwrap(),
    ]));
    assert_eq!(page_count(&r), PAGES_IN_FIXTURE);
    assert!(
        !limitation_codes(&r).contains(&BUDGET_CODE.to_string()),
        "a budget that never bit must not declare that it did"
    );

    let unbounded = representation(&extract(&[fixture().to_str().unwrap()]));
    assert_ne!(
        r["identity"]["profile_sha256"], unbounded["identity"]["profile_sha256"],
        "the identity describes the profile that ran, not the outcome"
    );
}
