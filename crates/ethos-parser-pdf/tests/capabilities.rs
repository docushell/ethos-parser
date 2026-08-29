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

//! M4 acceptance — the L1 gate (`docs/05-MILESTONES.md` M4, `docs/01-CONTRACT.md` §7).
//!
//! An artifact that does not declare its capabilities has not reached "extracted," regardless of
//! how good its text is. These tests hold the declarations to the same standard as the text:
//! every `true` capability names a test that proves it, every gap is on the wire, and a run with
//! a gap cannot be presented as a complete one.
//!
//! Fixtures resolve through `fixtures/manifest.json`'s three roots exactly as the oracle harness
//! resolves them. **A missing corpus is a failure, never a skip.**

use std::path::PathBuf;

use ethos_parser_core::{
    Capabilities, GeometryPresence, PageBindingResult, PageBudget, PageState,
    ProcessingTerminalState, Profile,
};
use ethos_parser_pdf::{limitations as lim, Classification, Document, ExtractArtifact};

// -------------------------------------------------------------------------------------------
// Fixture resolution
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn manifest() -> serde_json::Value {
    let p = repo_root().join("fixtures/manifest.json");
    serde_json::from_slice(
        &std::fs::read(&p)
            .unwrap_or_else(|e| panic!("manifest unreadable at {}: {e}", p.display())),
    )
    .expect("manifest is valid JSON")
}

fn path_in(root_name: &str, rel: &str) -> PathBuf {
    let m = manifest();
    let decl = &m["roots"][root_name];
    assert!(!decl.is_null(), "manifest declares no root `{root_name}`");
    let root = match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    };
    let p = root.join(rel);
    assert!(
        p.is_file(),
        "fixture `{rel}` missing from `{root_name}` at {}. A missing corpus is a failure, never \
         a skip.",
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
fn bench(name: &str) -> PathBuf {
    path_in("benchmark", name)
}

fn extract_with(path: PathBuf, profile: &Profile) -> ExtractArtifact {
    let doc = Document::open(&path, profile).expect("document opens");
    ethos_parser_pdf::extract(&doc, profile).expect("document extracts")
}

fn extract_ok(path: PathBuf) -> ExtractArtifact {
    extract_with(path, &Profile::default())
}

fn classify_with(path: PathBuf, profile: &Profile) -> Classification {
    let doc = Document::open(&path, profile).expect("document opens");
    ethos_parser_pdf::classify(&doc, profile).expect("document classifies")
}

fn codes(limitations: &[ethos_parser_core::Limitation]) -> Vec<&str> {
    limitations.iter().map(|l| l.code.as_str()).collect()
}

// -------------------------------------------------------------------------------------------
// 1. Every declared capability has a passing test
// -------------------------------------------------------------------------------------------

/// The proof table: capability field → the test that proves it, or why it is `false`.
///
/// `05-MILESTONES.md` M4 lists as an **Out**: *any capability declared `true` that is not
/// tested*. This is the enforcement. The destructuring in
/// [`every_true_capability_names_a_proof_test`] means adding a capability without adding a row
/// here is a **compile error**, and the row's named test must actually exist in this crate's test
/// sources — a proof that is only a string is not a proof.
struct Proof {
    field: &'static str,
    claimed: bool,
    /// The test function proving the claim. `None` is only legal when `claimed` is false.
    proof_test: Option<&'static str>,
    /// Why the capability is false. `None` is only legal when `claimed` is true.
    why_not: Option<&'static str>,
}

fn proof_table() -> Vec<Proof> {
    let c = Capabilities::V0;

    // Exhaustiveness gate. A capability added to the type without a row below fails to compile,
    // which is the only way this table cannot silently fall behind the thing it describes.
    let Capabilities {
        spans,
        char_offsets,
        tables,
        measured_ink_boxes,
        multi_column_reading_order,
        structural_locators,
        form_fields,
        annotations,
        images,
        page_screenshots,
        markdown,
        html,
    } = c;

    vec![
        Proof {
            field: "spans",
            claimed: spans,
            proof_test: Some("spans_are_emitted_with_native_locators"),
            why_not: None,
        },
        Proof {
            field: "char_offsets",
            claimed: char_offsets,
            proof_test: None,
            why_not: Some(
                "v0 emits runs and no element/span hierarchy, so there is nothing for an offset \
                 to index into. M5 built the record and left this false: v0 does no line \
                 grouping, so an element and a span are the same object and an offset would \
                 always be 0..len. Ethos's validator also ties the capability to the fields, \
                 so claiming it would oblige every span to carry offsets. Flips at v1.",
            ),
        },
        Proof {
            field: "tables",
            claimed: tables,
            // v1-S1. The claim is "this profile looked", and the proof is a document that drew a
            // grid, whose cells come back with the right spans, the right parents and the right
            // text — including the empty one.
            proof_test: Some("a_ruled_grid_is_reconstructed_with_spans_and_parent_ids"),
            why_not: None,
        },
        Proof {
            field: "measured_ink_boxes",
            claimed: measured_ink_boxes,
            proof_test: Some("measured_ink_boxes_are_measured_and_absence_stays_typed"),
            why_not: None,
        },
        Proof {
            field: "multi_column_reading_order",
            claimed: multi_column_reading_order,
            // v1-S5. The claim is "this profile orders by page geometry", and the proof has to
            // cover both halves of that or it proves half a capability: a document the stream
            // wrote in the wrong order comes out column-major, AND a single-column document is
            // not touched. A reader that only reordered could be reordering everything.
            proof_test: Some("multi_column_order_is_read_and_single_column_is_left_alone"),
            why_not: None,
        },
        Proof {
            field: "images",
            claimed: images,
            // v1-S6. The claim is "this profile looks", so the proof covers both halves: a page
            // that PAINTS an image yields a node with a placement and a digest, and a page that
            // merely declares one in its resources yields none. Those two are the whole of what
            // the flag means, and a proof of either alone would be a proof of half a capability.
            proof_test: Some("an_image_is_a_node_with_a_placement_and_a_digest"),
            why_not: None,
        },
        Proof {
            field: "page_screenshots",
            claimed: page_screenshots,
            proof_test: None,
            why_not: Some(
                "No page raster is produced, at any resolution, and this is a decision rather \
                 than a gap left open. Rendering a page needs a PDF renderer — glyph \
                 rasterization, shadings, blend modes, image filters — and this workspace has \
                 none: PDFium is admitted only caller-provided under an explicit ADR, no AGPL \
                 renderer clears the licence allowlist, and shelling out to an external \
                 converter would put an unpinned binary between the document and the artifact. \
                 `page-raster-not-emitted` is declared on every artifact and `raster_dpi` \
                 records the not-emitted state on the profile, so a renderer arriving later \
                 moves `profile_sha256` instead of silently changing what an artifact means.",
            ),
        },
        Proof {
            field: "markdown",
            claimed: markdown,
            // The proof is the golden, not the serializer: a Markdown exporter is easy and a
            // Markdown exporter whose output can be inverted back to evidence is the claim.
            proof_test: Some("a_markdown_quote_verifies_end_to_end"),
            why_not: None,
        },
        Proof {
            field: "html",
            claimed: html,
            // v1.1-S4, and the proof is the same shape as `markdown`'s for the same reason: an
            // HTML serializer is easy, and an HTML serializer whose bytes invert back to evidence
            // is the claim. This golden runs on `markdown-two-blocks`, which has no table — the
            // merge that only HTML can carry is pinned separately, by
            // `the_two_projections_disagree_about_the_merge_and_say_so`, because that needs a
            // fixture with a merged cell and this one needs measured ink on two runs.
            proof_test: Some("an_html_quote_verifies_end_to_end"),
            why_not: None,
        },
        Proof {
            field: "structural_locators",
            claimed: structural_locators,
            // v1-S3. The claim is "this profile looks", and the proof has to cover both halves
            // of that: a tagged document whose runs come back with the role path its own tree
            // gives them, AND an untagged one that gains nothing. One without the other proves
            // half a capability — a reader that only found roles could be inventing them.
            proof_test: Some("the_structure_tree_supplies_role_paths_and_absence_stays_absent"),
            why_not: None,
        },
        Proof {
            field: "form_fields",
            claimed: form_fields,
            // v1-S4. The claim is "this profile looks", so the proof covers both halves: a
            // document with a form yields its field, and one without yields none while the
            // capability stays true. And the half that matters most for this slice — the value
            // does not appear among the page's text runs.
            proof_test: Some("a_form_fields_value_is_a_node_and_never_a_text_run"),
            why_not: None,
        },
        Proof {
            field: "annotations",
            claimed: annotations,
            proof_test: Some("an_annotations_contents_is_a_node_and_never_a_text_run"),
            why_not: None,
        },
    ]
}

/// Every source line of the workspace's integration tests, for locating a named proof.
///
/// **Every workspace member's `tests/` directory**, read from `Cargo.toml`'s `members`.
///
/// It scanned one directory at M4, because every capability up to then was about reading a PDF
/// and its proof necessarily lived beside the parser. `capabilities.markdown` was not: the
/// projection lives in `ethos-parser-core` — `ethos-parser-pdf` deliberately does not learn Markdown
/// (`docs/04-ARCHITECTURE.md` §1) — and its proof is an end-to-end run of `extract`, `ground`,
/// `markdown` and `verify` against the pinned Ethos CLI, which can only be driven from the CLI
/// crate's tests. So v1.1-S1 added a second entry to a hardcoded pair.
///
/// Widening the scan was the honest fix then and it is the honest fix now; what was wrong was
/// widening it **one crate at a time, after each failure**, while this sentence claimed the scan
/// already covered the workspace. Three members — `ethos-parser-core`, `ethos-parser-office` and
/// `ethos-parser-grounding` — were never in the pair, so a capability whose proof landed in any of them
/// would have been reported as having no proof at all. The alternative to widening was always a
/// thinner proof placed here to satisfy the search, which would make the guard pass while the
/// capability's real evidence sat somewhere the guard could not see.
fn test_sources() -> String {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();

    // **The crate list is read from `Cargo.toml`'s `members`, not written here.** It was
    // `["ethos-parser-pdf", "ethos-parser-cli"]`, added one crate at a time as a proof turned up somewhere
    // the scan could not see — which is a list that grows only after a failure, and the doc above
    // meanwhile said *every* integration test in the workspace. `ethos-parser-core`, `ethos-parser-office`
    // and `ethos-parser-grounding` were all invisible. The same repair, for the same reason, as
    // `no_job_filter_selects_zero_tests` in `crates/ethos-parser-cli/tests/v0_exit_criteria.rs`.
    let manifest =
        std::fs::read_to_string(workspace.join("Cargo.toml")).expect("Cargo.toml is readable");
    let members = manifest
        .split_once("members = [")
        .expect("the `[workspace]` table declares no `members`")
        .1;
    let members = &members[..members.find(']').expect("`members` is unterminated")];
    let members: Vec<&str> = members
        .split('"')
        .skip(1)
        .step_by(2)
        .filter_map(|p| p.rsplit('/').next())
        .collect();

    let mut all = String::new();
    let mut files = 0usize;
    let mut scanned_crates = 0usize;
    for crate_name in &members {
        let dir = workspace.join("crates").join(crate_name).join("tests");
        let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| {
            panic!(
                "{} unreadable: {e}\nEvery workspace member has integration tests, and a member \
                 that stops having them is a decision this scan should not absorb in silence.",
                dir.display()
            )
        });
        scanned_crates += 1;
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == "rs") {
                all.push_str(&std::fs::read_to_string(&path).expect("readable"));
                files += 1;
            }
        }
    }

    // **Three floors, because the old one had stopped being a floor.** `all.len() > 1000` sat
    // against a real 660,000 — six hundred times below the number it was guarding, so it would
    // have passed with every crate but one silently missing. The crate count is the load-bearing
    // one: it is an equality against `Cargo.toml`, so a sixth member cannot be quietly unscanned.
    assert_eq!(
        scanned_crates,
        members.len(),
        "scanned {scanned_crates} of {} workspace member(s); a proof living in an unscanned \
         crate reads as a proof that does not exist",
        members.len()
    );
    assert!(
        files >= 35,
        "only {files} integration test file(s) scanned; forty is the number at v2-S13.1 and this \
         floor sits below the smallest crate, so this says the walk stopped working"
    );
    assert!(
        all.len() > 500_000,
        "the test-source scan found only {} bytes, so the proof check would pass vacuously",
        all.len()
    );
    all
}

/// **A capability asserted `true` with no test is a build failure.**
#[test]
fn every_true_capability_names_a_proof_test() {
    let sources = test_sources();

    for p in proof_table() {
        if p.claimed {
            let name = p.proof_test.unwrap_or_else(|| {
                panic!(
                    "`capabilities.{}` is claimed true with no proof test named. A capability \
                     without a passing test is exactly what docs/05-MILESTONES.md M4 lists as \
                     out of scope.",
                    p.field
                )
            });
            assert!(
                sources.contains(&format!("fn {name}(")),
                "`capabilities.{}` names `{name}` as its proof, and no such test exists in \
                 crates/ethos-parser-pdf/tests/. A proof that is only a string is not a proof.",
                p.field
            );
            assert!(
                p.why_not.is_none(),
                "`capabilities.{}` is true, so it needs a proof, not an excuse",
                p.field
            );
        } else {
            assert!(
                p.why_not.is_some(),
                "`capabilities.{}` is false and says nothing about why. Every false capability \
                 owes a reason and a declared limitation.",
                p.field
            );
            assert!(
                p.proof_test.is_none(),
                "`capabilities.{}` is false but names a proof test, which cannot be right",
                p.field
            );
        }
    }
}

/// Guard the guard: the proof check would be worthless if it accepted any string.
///
/// The negative control's name is **assembled at runtime**, because a literal spelling of it in
/// this file would make the file contain the very string the check looks for — and the control
/// would pass by describing itself. That is not a hypothetical: the first version of this test
/// did exactly that and failed on its own source.
#[test]
fn a_missing_proof_test_would_be_detected() {
    let sources = test_sources();

    let absent = format!("fn {}_{}_{}(", "a", "proof", "that_does_not_exist");
    assert!(
        !sources.contains(&absent),
        "the negative control `{absent}` must not exist, or the check proves nothing"
    );
    assert!(
        sources.contains("fn every_true_capability_names_a_proof_test("),
        "the positive control must be found by the same mechanism the check uses"
    );
}

/// Proof for `capabilities.spans`.
#[test]
fn spans_are_emitted_with_native_locators() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    let runs: Vec<_> = a.runs().collect();
    assert!(!runs.is_empty(), "the claim is that spans are emitted");
    for r in &runs {
        assert!(r.locator.page >= 1, "1-based page on every span");
        assert!(!r.text.is_empty());
    }
    assert!(a.assurance.capabilities.spans);
}

/// Proof for `capabilities.measured_ink_boxes`.
///
/// Both halves, because the claim is not "boxes exist" — it is that boxes come from *measured*
/// metrics and that the absence path stays typed rather than falling back to a guess.
#[test]
fn measured_ink_boxes_are_measured_and_absence_stays_typed() {
    let measured = extract_ok(engine_fx("measured-ink-box"));
    let boxes: Vec<_> = measured
        .runs()
        .filter_map(|r| r.geometry.measured())
        .collect();
    assert!(
        !boxes.is_empty(),
        "a font with usable metrics must produce at least one measured box"
    );
    assert!(measured.assurance.capabilities.measured_ink_boxes);

    // The other half: no usable metrics produces typed absence, never a fabricated box.
    let absent = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    assert!(
        absent
            .runs()
            .all(|r| matches!(r.geometry, GeometryPresence::Absent(_))),
        "a font with no widths must yield typed absence, not a box derived from the font size"
    );
}

/// Every `false` capability is matched by a declared limitation, on both artifacts.
#[test]
fn every_false_capability_declares_a_limitation_on_the_wire() {
    let path = conformance("synthetic/simple-text/document.pdf");
    let profile = Profile::default();
    let e = extract_with(path.clone(), &profile);
    let c = classify_with(path, &profile);

    let expected = Capabilities::V0.declared_limitations();
    assert!(
        !expected.is_empty(),
        "v0 has false capabilities, so it owes limitations"
    );

    for artifact_codes in [
        codes(&e.assurance.limitations),
        codes(&c.assurance.limitations),
    ] {
        for l in &expected {
            assert!(
                artifact_codes.contains(&l.code.as_str()),
                "`{}` is false and its limitation is missing from the artifact: {artifact_codes:?}",
                l.code
            );
        }
    }
}

// -------------------------------------------------------------------------------------------
// 2. `synthetic/two-columns` reads column-major, and the limitation it used to carry is gone
// -------------------------------------------------------------------------------------------

/// **The `multi_column_reading_order` proof, both halves** (v1-S5).
///
/// # What this test used to assert
///
/// Through v1-S4 it was `two_columns_declares_the_multi_column_limitation`, and it asserted the
/// *declaration* rather than correct order — that a two-column document came out
/// right-column-first and that the artifact said so. Its own comment read: *"Asserting correct
/// order here would be asserting a capability v0 does not have."*
///
/// v1-S5 gives the profile that capability, so the test asserts the capability. The limitation it
/// used to look for must now be **absent**, and absent rather than reworded: a reader who finds
/// `multi-column-reading-order` on an artifact acts on it, and acting on it here would mean
/// distrusting an order that is correct.
///
/// # Why both halves
///
/// A reader that reordered everything would pass the first half and be worse than useless. So the
/// single-column fixture is checked in the same test: it must come back in exactly the order the
/// content stream drew it, with no reordering at all.
#[test]
fn multi_column_order_is_read_and_single_column_is_left_alone() {
    let a = extract_ok(conformance("synthetic/two-columns/document.pdf"));

    assert!(
        a.assurance.capabilities.multi_column_reading_order,
        "v1-S5 claims multi-column reading order"
    );
    assert_eq!(
        a.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V1
    );

    // Half one: the content stream writes the right column first, and the artifact does not.
    let texts: Vec<&str> = a.runs().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left top", "Left bottom", "Right top", "Right bottom"],
        "column-major: left band top to bottom, then right"
    );

    // The retired declaration. Gone, not reworded.
    assert!(
        !codes(&a.assurance.limitations)
            .contains(&ethos_parser_core::codes::MULTI_COLUMN_READING_ORDER),
        "this profile reads this document in the right order, so it must not carry a limitation \
         saying it does not: {:?}",
        codes(&a.assurance.limitations)
    );

    // And the narrower one that replaced it, which is a real statement about a real leftover.
    let l = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::READING_ORDER_GEOMETRIC_ONLY)
        .unwrap_or_else(|| {
            panic!(
                "the rule's own scope must be declared beside the capability: {:?}",
                codes(&a.assurance.limitations)
            )
        });
    assert_eq!(l.scope, ethos_parser_core::LimitationScope::Profile);
    assert!(
        l.detail.contains("WHITESPACE IN PAGE SPACE"),
        "the declaration must say what the order was built from: {}",
        l.detail
    );

    // Half two: a single-column document is not touched. Same profile, same rule id, and the
    // two runs come back in the order they were drawn.
    let single = extract_ok(conformance("synthetic/two-lines/document.pdf"));
    let single_texts: Vec<&str> = single.runs().map(|r| r.text.as_str()).collect();
    assert_eq!(
        single_texts,
        vec!["First line", "Second line"],
        "no gutter, no reordering — the rule does nothing where it has no evidence"
    );
    assert_eq!(
        single.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V1
    );

    // Classification declares the capability too: it belongs to the profile, not to one stage.
    let c = classify_with(
        conformance("synthetic/two-columns/document.pdf"),
        &Profile::default(),
    );
    assert!(!codes(&c.assurance.limitations)
        .contains(&ethos_parser_core::codes::MULTI_COLUMN_READING_ORDER));
    assert!(codes(&c.assurance.limitations)
        .contains(&ethos_parser_core::codes::READING_ORDER_GEOMETRIC_ONLY));
}

// -------------------------------------------------------------------------------------------
// 3. Partial processing is terminal and visible
// -------------------------------------------------------------------------------------------

/// A gap makes the artifact partial, and no API call can call it complete.
#[test]
fn partial_processing_is_terminal_and_cannot_look_whole() {
    let profile = Profile {
        page_budget: PageBudget::AtMost(1),
        ..Profile::default()
    };

    // Two pages, budget of one. The second page exists, was never read, and says so.
    let a = extract_with(bench("irs-form-1040-2025.pdf"), &profile);

    assert_eq!(a.page_count, 2, "the fixture is the 2-page IRS form");
    assert!(
        !a.is_complete(),
        "an artifact with an unread page must never present as a complete reading"
    );
    assert_eq!(
        a.assurance.coverage.pages_authorized, 2,
        "every page of the document was authorized"
    );
    assert_eq!(a.assurance.coverage.pages_processed, 1);
    assert_eq!(a.assurance.coverage.pages_quarantined, 1);
    assert!(a.assurance.coverage.reconciles());

    match a.assurance.terminal_state {
        ProcessingTerminalState::Partial(gaps) => {
            assert_eq!(gaps.pages_not_processed, 1);
            assert_eq!(gaps.first_gap_page, 2);
        }
        other => panic!("a run with a gap must be Partial, got {other:?}"),
    }

    // The gap names a limitation the artifact actually declares.
    assert!(a.assurance.every_gap_names_a_declared_limitation());
    assert!(
        codes(&a.assurance.limitations).contains(&ethos_parser_core::codes::RESOURCE_LIMIT_PAGES)
    );

    // And the run list is honest about which pages it covers.
    assert_eq!(a.pages.len(), 1);
    assert_eq!(a.pages[0].index, 1);
}

/// The same gap, seen from the query side: page 2 is indeterminate, not empty.
#[test]
fn a_query_against_an_unprocessed_page_is_capability_limited_not_negative() {
    let profile = Profile {
        page_budget: PageBudget::AtMost(1),
        ..Profile::default()
    };
    let a = extract_with(bench("irs-form-1040-2025.pdf"), &profile);

    assert_eq!(a.assurance.page_binding_status(1), PageBindingResult::Ok);
    assert_eq!(
        a.assurance.page_binding_status(2),
        PageBindingResult::CapabilityLimited {
            limitation_code: ethos_parser_core::codes::RESOURCE_LIMIT_PAGES.into()
        },
        "absence of extractable content is never evidence of absence in the source"
    );
    assert!(!a.assurance.page_binding_status(2).is_determinate());

    // A page that is not in the document is a different answer from a page nobody read.
    assert_eq!(
        a.assurance.page_binding_status(3),
        PageBindingResult::NotInDocument
    );
}

/// Bounded classification produces the same visible gap, under its own reason code.
#[test]
fn bounded_classification_reports_unsampled_pages_as_never_attempted() {
    let profile = Profile {
        classify_sample_pages: 3,
        ..Profile::default()
    };
    let c = classify_with(bench("nist-sp-800-63b.pdf"), &profile);

    assert_eq!(c.page_count, 80);
    assert_eq!(c.pages_content_scanned, 3, "the bound still holds");
    assert!(
        !c.is_complete(),
        "77 unread pages is not a complete reading"
    );
    assert_eq!(c.assurance.coverage.pages_processed, 3);
    assert_eq!(c.assurance.coverage.pages_not_attempted, 77);
    assert_eq!(c.assurance.coverage.pages_quarantined, 0);
    assert!(c.assurance.coverage.reconciles());

    // The state name is the point: "not attempted" cannot be misread as "page has no text".
    let page_40 = c
        .assurance
        .page_states
        .iter()
        .find(|e| e.index == 40)
        .expect("every authorized page has a disposition");
    assert_eq!(
        page_40.state,
        PageState::NotAttempted(lim::CLASSIFY_SAMPLE_BOUND.to_string())
    );
    assert_eq!(
        c.assurance.page_binding_status(40),
        PageBindingResult::CapabilityLimited {
            limitation_code: lim::CLASSIFY_SAMPLE_BOUND.into()
        }
    );
    assert!(c.assurance.every_gap_names_a_declared_limitation());
}

/// A resource ceiling and the sampling bound are different gaps and report differently.
#[test]
fn a_budget_and_a_sample_bound_are_not_confused_with_each_other() {
    let profile = Profile {
        classify_sample_pages: 8,
        page_budget: PageBudget::AtMost(2),
        ..Profile::default()
    };
    let c = classify_with(bench("nist-sp-800-63b.pdf"), &profile);

    assert_eq!(
        c.pages_content_scanned, 2,
        "the tighter of the two bounds decides how many pages are read"
    );
    // Pages 3..=8 were inside the sample window and outside the budget: quarantined, not merely
    // unsampled. Pages 9..=80 were outside both, and the budget is the binding constraint.
    for index in [3u32, 9, 80] {
        let e = c
            .assurance
            .page_states
            .iter()
            .find(|e| e.index == index)
            .expect("disposition present");
        assert_eq!(
            e.state,
            PageState::Quarantined(ethos_parser_core::codes::RESOURCE_LIMIT_PAGES.to_string()),
            "page {index} was stopped by the budget, and the reason must say so"
        );
    }
    assert_eq!(c.assurance.coverage.pages_quarantined, 78);
    assert_eq!(c.assurance.coverage.pages_not_attempted, 0);
    assert!(c.assurance.coverage.reconciles());
}

/// A budget looser than the sample bound is **not** the reason anything went unread.
///
/// The case that a per-page budget test gets wrong. At `N = 8` under a budget of 20, pages 21–80
/// are outside the budget — but at `N = 8` they would not have been read if the budget were
/// lifted entirely, so reporting them `quarantined` would send a caller to raise a ceiling that
/// is not what stopped them. The reason names the binding constraint, uniformly.
#[test]
fn a_budget_that_does_not_bind_is_not_blamed_for_the_sample_bound() {
    let profile = Profile {
        classify_sample_pages: 8,
        page_budget: PageBudget::AtMost(20),
        ..Profile::default()
    };
    let c = classify_with(bench("nist-sp-800-63b.pdf"), &profile);

    assert_eq!(
        c.pages_content_scanned, 8,
        "the sample bound is the tighter one"
    );
    assert_eq!(c.assurance.coverage.pages_not_attempted, 72);
    assert_eq!(
        c.assurance.coverage.pages_quarantined, 0,
        "a budget that was never reached must not be blamed for anything"
    );

    for index in [9u32, 21, 80] {
        let e = c
            .assurance
            .page_states
            .iter()
            .find(|e| e.index == index)
            .expect("disposition present");
        assert_eq!(
            e.state,
            PageState::NotAttempted(lim::CLASSIFY_SAMPLE_BOUND.to_string()),
            "page {index} went unread because of the sample bound, whichever side of the budget \
             it happens to fall on"
        );
    }

    // And the limitation list agrees: no resource-limit declaration, because none applied.
    assert!(
        !codes(&c.assurance.limitations).contains(&ethos_parser_core::codes::RESOURCE_LIMIT_PAGES)
    );
    assert!(c.assurance.every_gap_names_a_declared_limitation());
}

// -------------------------------------------------------------------------------------------
// 5. `failure/memory-limit-simulated`
// -------------------------------------------------------------------------------------------

/// The limit is **configuration**, not the document.
///
/// `failure/memory-limit-simulated/document.pdf` is byte-identical to
/// `synthetic/simple-text/document.pdf` — both `sha256:f2f6ab91…`, asserted below rather than
/// taken on trust. The fixture name means *a limit simulated by configuration*, so a meaningful
/// test has to set the knob explicitly, and on a one-page document the only budget that bites is
/// zero.
///
/// The behaviour is **declared limitation plus coverage gap**, not a hard refusal: the run is
/// honest about having read nothing, and never emits a tree that looks whole.
#[test]
fn the_memory_limit_fixture_declares_a_gap_rather_than_looking_whole() {
    let fixture = conformance("failure/memory-limit-simulated/document.pdf");
    let simple = conformance("synthetic/simple-text/document.pdf");
    assert_eq!(
        std::fs::read(&fixture).unwrap(),
        std::fs::read(&simple).unwrap(),
        "the premise of this test: the bytes are identical, so only configuration differs"
    );

    let profile = Profile {
        page_budget: PageBudget::AtMost(0),
        ..Profile::default()
    };
    let a = extract_with(fixture, &profile);

    assert!(
        !a.is_complete(),
        "nothing was read; this cannot read as done"
    );
    assert_eq!(a.assurance.coverage.pages_authorized, 1);
    assert_eq!(a.assurance.coverage.pages_processed, 0);
    assert_eq!(a.assurance.coverage.pages_quarantined, 1);
    assert!(a.assurance.coverage.reconciles());
    assert!(
        a.pages.is_empty() && a.runs().count() == 0,
        "a budget of zero must not produce a partial tree at all, let alone a whole-looking one"
    );

    let l = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::RESOURCE_LIMIT_PAGES)
        .expect("the gap must be declared, not merely counted");
    assert_eq!(l.scope, ethos_parser_core::LimitationScope::Document);
    assert!(l.detail.contains("quarantined"));

    // The same bytes at the default profile read cleanly — proof the gap came from the knob.
    let unlimited = extract_ok(simple);
    assert!(unlimited.is_complete());
    assert_eq!(unlimited.assurance.coverage.pages_processed, 1);
}

// -------------------------------------------------------------------------------------------
// 4 / 7 / 9 / 10. Absence, hash sensitivity, coverage everywhere, determinism
// -------------------------------------------------------------------------------------------

/// No confidence field, and no implied full value in its place.
///
/// `docs/01-CONTRACT.md` §9.1: *a processor that reports no uncertainty produces an absent
/// field, never an implied `1.0`.* v0 has no uncertainty to report, so the absence is total —
/// there is no field, and no number standing in for one.
#[test]
fn absence_is_never_a_full_value_and_no_confidence_field_exists() {
    let path = conformance("synthetic/simple-text/document.pdf");
    let profile = Profile::default();
    let e = extract_with(path.clone(), &profile);
    let c = classify_with(path, &profile);

    for (label, bytes) in [
        ("extract", e.to_canonical_bytes().unwrap()),
        ("classify", c.to_canonical_bytes().unwrap()),
    ] {
        let s = String::from_utf8(bytes).unwrap();
        let lower = s.to_lowercase();
        for banned in ["confidence", "score", "quality", "grade", "certainty"] {
            assert!(
                !lower.contains(banned),
                "`{banned}` appears in the {label} artifact; no field may summarise quality"
            );
        }
        // No bare decimal anywhere: an implied `1.0` would have to arrive as a number, and c14n
        // refuses non-integers. Scanned outside quoted strings because `schema_version` is
        // legitimately `"0.1.0"` — a substring match on `1.0` flags the version and misses the
        // actual hazard, which is the wrong instrument for this rule.
        let mut in_string = false;
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            match chars[i] {
                '"' if i == 0 || chars[i - 1] != '\\' => in_string = !in_string,
                '.' if !in_string => {
                    panic!("a bare decimal reached the {label} artifact: {s}")
                }
                _ => {}
            }
        }
    }
}

/// A fully successful document still carries the coverage object.
///
/// Zeros in the gap buckets are correct output. The *absence* of the object is what is
/// forbidden, because then "no gaps" and "gaps not tracked" look identical on the wire.
#[test]
fn a_fully_processed_document_still_carries_coverage() {
    let path = conformance("synthetic/simple-text/document.pdf");
    let profile = Profile::default();

    let e = extract_with(path.clone(), &profile);
    assert!(e.is_complete());
    assert_eq!(
        e.assurance.terminal_state,
        ProcessingTerminalState::Complete
    );
    assert_eq!(e.assurance.coverage.pages_authorized, 1);
    assert_eq!(e.assurance.coverage.pages_processed, 1);
    assert_eq!(e.assurance.coverage.pages_failed, 0);
    assert_eq!(e.assurance.coverage.pages_not_attempted, 0);

    let s = String::from_utf8(e.to_canonical_bytes().unwrap()).unwrap();
    for required in [
        "\"coverage\"",
        "\"pages_authorized\"",
        "\"pages_failed\":0",
        "\"terminal_state\"",
        "\"capabilities\"",
        "\"limitations\"",
        "\"page_states\"",
    ] {
        assert!(
            s.contains(required),
            "{required} missing from a clean extract"
        );
    }

    // A one-page document classified at N=8 is also complete: the bound did not bite.
    let c = classify_with(path, &profile);
    assert!(c.is_complete());
    assert_eq!(c.assurance.coverage.pages_not_attempted, 0);
}

/// Coverage reconciles, and every gap names a declared limitation, on every fixture that opens.
#[test]
fn coverage_reconciles_across_the_corpus() {
    let openable = [
        "synthetic/heading-export",
        "synthetic/hyphenated-line-break",
        "synthetic/ligature-fi-embedded-font",
        "synthetic/list-items",
        "synthetic/rotation-90",
        "synthetic/simple-text",
        "synthetic/two-columns",
        "synthetic/two-lines",
        "failure/image-only-or-blank-page",
        "failure/memory-limit-simulated",
    ];
    let profile = Profile::default();

    for id in openable {
        let path = conformance(&format!("{id}/document.pdf"));
        let e = extract_with(path.clone(), &profile);
        assert!(
            e.assurance.coverage.reconciles(),
            "{id}: extract coverage does not reconcile: {:?}",
            e.assurance.coverage
        );
        assert_eq!(
            e.assurance.page_states.len() as u32,
            e.page_count,
            "{id}: every page needs exactly one disposition"
        );
        assert_eq!(
            e.pages.len() as u32,
            e.assurance.coverage.pages_processed,
            "{id}: the page list and the processed count must agree"
        );
        assert!(e.assurance.every_gap_names_a_declared_limitation(), "{id}");

        let c = classify_with(path, &profile);
        assert!(
            c.assurance.coverage.reconciles(),
            "{id}: classify coverage does not reconcile"
        );
        assert_eq!(
            c.pages.len() as u32,
            c.assurance.coverage.pages_processed,
            "{id}: scanned rows and the processed count must agree"
        );
        assert!(c.assurance.every_gap_names_a_declared_limitation(), "{id}");
    }
}

/// Changing the capability set changes `profile_sha256`, and the artifact carries the new one.
///
/// Core already proves the hash moves. This proves the *artifact* moves with it — the identity
/// on the wire is the one that makes two artifacts comparable or not.
#[test]
fn a_changed_capability_set_changes_the_artifacts_profile_hash() {
    let path = conformance("synthetic/simple-text/document.pdf");
    let base = extract_with(path.clone(), &Profile::default());

    // v1-S1 inverted this: `tables` is TRUE by default now, so the profile that must be
    // non-comparable is the one that does NOT claim it.
    let not_claiming = Profile {
        capabilities: Capabilities {
            tables: false,
            ..Capabilities::V0
        },
        ..Profile::default()
    };
    let changed = extract_with(path, &not_claiming);

    assert_ne!(
        base.identity.profile_sha256, changed.identity.profile_sha256,
        "a profile that did not look for tables must not be comparable with one that did"
    );
    assert!(!changed.assurance.capabilities.tables);

    // The two limitations swap, which is the whole point of the pair: a profile that did not look
    // declares `tables-not-extracted`, and one that did declares the SCOPE of its looking.
    let unclaimed = codes(&changed.assurance.limitations);
    assert!(
        unclaimed.contains(&ethos_parser_core::codes::TABLES_NOT_EXTRACTED),
        "a false capability owes its own limitation: {unclaimed:?}"
    );
    assert!(
        !unclaimed.contains(&ethos_parser_core::codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED),
        "a profile that never looked must not declare the scope of its looking: {unclaimed:?}"
    );

    let claimed = codes(&base.assurance.limitations);
    assert!(
        claimed.contains(&ethos_parser_core::codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED)
            && !claimed.contains(&ethos_parser_core::codes::TABLES_NOT_EXTRACTED),
        "and the default profile declares the scope rather than the absence: {claimed:?}"
    );
    // v1-S2: the scope narrowed. The blanket "alignment is never inspected" code is retired,
    // because the alignment rule now runs on every document this profile reads.
    assert!(
        !claimed.contains(&"unruled-tables-not-detected"),
        "the ruled-only limitation is retired at v1-S2, not carried forward: {claimed:?}"
    );
}

/// The budget is on the profile, so it is fingerprint-visible.
#[test]
fn a_changed_page_budget_changes_the_artifacts_profile_hash() {
    let path = bench("irs-form-1040-2025.pdf");
    let base = extract_with(path.clone(), &Profile::default());

    let budgeted = Profile {
        page_budget: PageBudget::AtMost(1),
        ..Profile::default()
    };
    let limited = extract_with(path, &budgeted);

    assert_ne!(
        base.identity.profile_sha256, limited.identity.profile_sha256,
        "a budgeted run produced different output and must not claim comparability"
    );
}

/// The new blocks do not break byte identity.
#[test]
fn the_assurance_block_is_byte_identical_across_runs() {
    let profile = Profile::default();
    for id in ["synthetic/two-columns", "synthetic/simple-text"] {
        let path = conformance(&format!("{id}/document.pdf"));
        let first = extract_with(path.clone(), &profile)
            .to_canonical_bytes()
            .unwrap();
        let second = extract_with(path.clone(), &profile)
            .to_canonical_bytes()
            .unwrap();
        assert_eq!(first, second, "{id}: extract is not byte-identical");

        let c1 = classify_with(path.clone(), &profile)
            .to_canonical_bytes()
            .unwrap();
        let c2 = classify_with(path, &profile).to_canonical_bytes().unwrap();
        assert_eq!(c1, c2, "{id}: classify is not byte-identical");
    }
}

// -------------------------------------------------------------------------------------------
// 8. `not_detected` absorbed — one vocabulary, not two
// -------------------------------------------------------------------------------------------

/// The stand-in lists are gone from the wire, and nothing they said was lost.
///
/// `docs/README.md` had this open since M2: *`not_detected` (M2) and `not_decoded` (M3) are
/// stand-ins for capability declarations … M4 should absorb both rather than sit beside them.*
/// Two vocabularies on one artifact means a consumer has to learn which one to trust, and the
/// answer is never written down.
#[test]
fn the_stand_in_vocabularies_are_absorbed_not_duplicated() {
    let path = conformance("synthetic/simple-text/document.pdf");
    let profile = Profile::default();
    let e = extract_with(path.clone(), &profile);
    let c = classify_with(path, &profile);

    for (label, bytes) in [
        ("extract", e.to_canonical_bytes().unwrap()),
        ("classify", c.to_canonical_bytes().unwrap()),
    ] {
        let s = String::from_utf8(bytes).unwrap();
        for gone in ["\"not_detected\"", "\"not_decoded\""] {
            assert!(
                !s.contains(gone),
                "{label} still carries {gone} beside `limitations` — that is the second \
                 vocabulary M4 exists to remove"
            );
        }
    }

    // What they declared still is declared, in the one surviving shape.
    let classify_codes = codes(&c.assurance.limitations);
    assert!(classify_codes.contains(&"garbled-reason-not-detected"));
    assert!(classify_codes.contains(&"multi-column-reason-not-detected"));

    let extract_codes = codes(&e.assurance.limitations);
    assert!(extract_codes.contains(&lim::FONT_WIDTHS_ABSENT));
    assert!(extract_codes.contains(&lim::FORM_XOBJECT_TEXT_NOT_DESCENDED));
}

/// The backend's xref posture is declared on every artifact, repaired or not.
///
/// Rewritten at v0.1. The policy limitation still rides on every artifact — a caller reading one
/// needs to know this backend is strict about entry width and what it does about it — and the
/// *document*-scoped limitation appears only where the repair actually fired. Keeping the two
/// apart is the point: "the repair exists" and "the repair ran here" are different facts.
#[test]
fn the_xref_posture_is_declared_and_the_repair_is_scoped_to_where_it_fired() {
    let profile = Profile::default();

    // A clean document: the policy is declared, the repair is not claimed.
    let clean = conformance("synthetic/simple-text/document.pdf");
    for artifact_codes in [
        codes(&extract_with(clean.clone(), &profile).assurance.limitations),
        codes(&classify_with(clean, &profile).assurance.limitations),
    ] {
        assert!(
            artifact_codes.contains(&lim::BACKEND_XREF_STRICT_20_BYTE),
            "the backend's xref policy must be nameable by callers: {artifact_codes:?}"
        );
        assert!(
            !artifact_codes.contains(&lim::XREF_ENTRY_PADDED),
            "a document that needed no repair must not claim one: {artifact_codes:?}"
        );
    }

    // The repaired document: both, on both artifacts.
    let repaired = conformance("synthetic/table-regular-grid/document.pdf");
    for artifact_codes in [
        codes(
            &extract_with(repaired.clone(), &profile)
                .assurance
                .limitations,
        ),
        codes(&classify_with(repaired, &profile).assurance.limitations),
    ] {
        assert!(
            artifact_codes.contains(&lim::XREF_ENTRY_PADDED),
            "a repaired open declares itself on every artifact it produces: {artifact_codes:?}"
        );
    }

    // A malformation outside the repaired class still refuses, with a named error and no body.
    // This is what says the bounded repair did not become general recovery.
    let unrepairable = conformance("failure/corrupt-header-valid/document.pdf");
    let err = Document::open(&unrepairable, &profile).expect_err("must not open");
    assert_eq!(err.code(), "malformed");
    assert_eq!(
        ethos_parser_core::RefusalCode::of(&err),
        ethos_parser_core::RefusalCode::Malformed,
        "a refusal has a name, even though no body is emitted to put it in"
    );
}

/// Limitations are canonically ordered, so two runs cannot disagree on their order.
#[test]
fn limitations_are_sorted_and_free_of_duplicates() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    let mut sorted = a.assurance.limitations.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        a.assurance.limitations, sorted,
        "the emitted order must already be canonical"
    );
}
