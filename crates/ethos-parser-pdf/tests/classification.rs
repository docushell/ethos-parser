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

//! M2 acceptance (`docs/history/05-MILESTONES.md`).
//!
//! Every bullet in M2's acceptance list is a test here. Fixtures resolve through
//! `fixtures/manifest.json`'s two roots exactly as the oracle harness resolves them; **a missing
//! corpus is a failure, never a skip**.

use std::path::PathBuf;
use std::time::Instant;

use ethos_parser_core::{EngineError, Profile};
use ethos_parser_pdf::exit::{exit_code, COULD_NOT_READ, NEEDS_ATTENTION, SIMPLE};
use ethos_parser_pdf::{Classification, Document, LayoutComplexityReason, OcrNeedReason};

// -------------------------------------------------------------------------------------------
// Fixture resolution — same two roots, same env overrides, same loud failure as oracle.rs.
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

fn root(name: &str) -> PathBuf {
    let m = manifest();
    let decl = &m["roots"][name];
    assert!(!decl.is_null(), "manifest declares no root `{name}`");
    if let Some(v) = decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        return PathBuf::from(v);
    }
    repo_root().join(decl["default"].as_str().expect("root has a default"))
}

fn path_in(root_name: &str, rel: &str) -> PathBuf {
    let p = root(root_name).join(rel);
    assert!(
        p.is_file(),
        "fixture `{rel}` missing from the `{root_name}` corpus at {}. Set {} to override; a \
         missing corpus is a failure, never a skip.",
        p.display(),
        manifest()["roots"][root_name]["env"]
            .as_str()
            .unwrap_or("(none)")
    );
    p
}

fn conformance(rel: &str) -> PathBuf {
    path_in("conformance", rel)
}
fn bench(rel: &str) -> PathBuf {
    path_in("benchmark", rel)
}

/// Open and classify, as the CLI does.
fn run(path: PathBuf, profile: &Profile) -> Result<Classification, EngineError> {
    Document::open(&path, profile).and_then(|d| ethos_parser_pdf::classify(&d, profile))
}

fn run_ok(path: PathBuf) -> Classification {
    run(path, &Profile::default()).expect("document should classify")
}

// -------------------------------------------------------------------------------------------
// 1. Bounded cost — the load-bearing test
// -------------------------------------------------------------------------------------------

/// **The counter proof.** Content scanning stops at N, on a 492-page document.
///
/// This is the exact class of bug pdf-inspector has: it advertises `Sample(8)` and Phase 3
/// re-scans every page, so `Pages(1)` costs the same as `Full` (memo §16.3, measured — 439 ms vs
/// 412 ms vs 434 ms on the same file). A later phase that rescanned the document would move
/// `pages_content_scanned` and fail here, which is why the counter is on the artifact rather than
/// in a comment.
#[test]
fn the_sampler_is_bounded_on_a_492_page_document() {
    let profile = Profile::default();
    let c = run(bench("nist-sp-800-53r5.pdf"), &profile).expect("classifies");

    assert_eq!(
        c.page_count, 492,
        "the corpus document should have 492 pages"
    );
    assert_eq!(c.pages_sampled, profile.classify_sample_pages);
    assert_eq!(
        c.pages_content_scanned, c.pages_sampled,
        "every sampled page is scanned exactly once"
    );
    assert_eq!(
        c.pages_content_scanned, 8,
        "cost must not scale with page count: 492 pages, 8 scanned"
    );
    assert_eq!(c.pages.len(), 8, "one row per scanned page, and no more");
}

#[test]
fn pages_content_scanned_is_always_min_of_n_and_page_count() {
    for (rel, expect_pages) in [
        ("nist-sp-800-53r5.pdf", 492u32),
        ("nist-sp-800-63b.pdf", 80),
        ("irs-form-1040-2025.pdf", 2),
    ] {
        for n in [1u32, 2, 8, 16] {
            let profile = Profile {
                classify_sample_pages: n,
                ..Profile::default()
            };
            let c = run(bench(rel), &profile).expect("classifies");
            assert_eq!(c.page_count, expect_pages, "{rel}");
            assert_eq!(
                c.pages_content_scanned,
                n.min(expect_pages),
                "{rel} at N={n}: scanned must be min(N, page_count)"
            );
        }
    }
}

/// **The timing proof**, measuring the phase whose boundedness is actually the claim.
///
/// The first version of this test compared *total* classify time (open + classify) between a
/// 492-page and an 80-page document and demanded they be within a small factor. It failed at
/// 7.07×, and the failure was informative rather than a bug: `pages_content_scanned` was 8 in
/// both cases, so the page walk was already bounded — what scales is `Document::open`, because
/// `lopdf` parses the whole object graph eagerly. Total time is `O(parse) + O(N × per-page)`, and
/// no amount of bounded sampling makes the parse term disappear.
///
/// So this measures the two phases separately, which is what `docs/history/03-V0-SCOPE.md` §6 actually
/// claims: *"~0.5 ms per sampled page, plus document parse"*. Measured here (release, best of 3):
///
/// | document | pages | MB | open | classify |
/// | --- | --- | --- | --- | --- |
/// | `irs-form-1040-2025` | 2 | 0.2 | 16 ms | 14 ms |
/// | `nist-sp-800-63b` | 80 | 1.4 | 48 ms | 21 ms |
/// | `nist-sp-800-53r5` | 492 | 5.8 | 431 ms | 25 ms |
///
/// **246× the pages, 1.7× the classify time.** That is the bound. The open column is `lopdf`'s
/// eager parse and is a declared characteristic of the backend, not of the sampler.
#[test]
fn classify_cost_is_flat_in_total_page_count() {
    let profile = Profile::default();

    let phase_times = |rel: &'static str| -> (f64, u32) {
        let bytes = std::fs::read(bench(rel)).expect("fixture readable");
        // Warm: first open pays page-cache and allocator costs that belong to neither phase.
        let _ = Document::open_bytes(&bytes, &profile);

        let mut best = f64::INFINITY;
        let mut pages = 0;
        for _ in 0..3 {
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            pages = doc.page_count();
            let t = Instant::now();
            let c = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");
            best = best.min(t.elapsed().as_secs_f64());
            assert_eq!(c.pages_content_scanned, c.pages_sampled);
        }
        (best, pages)
    };

    let (big, big_pages) = phase_times("nist-sp-800-53r5.pdf");
    let (small, small_pages) = phase_times("nist-sp-800-63b.pdf");

    let page_ratio = f64::from(big_pages) / f64::from(small_pages);
    let time_ratio = big / small;

    assert!(
        page_ratio > 6.0,
        "the fixtures should differ ~6x in page count"
    );
    assert!(
        time_ratio < page_ratio / 2.0,
        "classify took {big:.4}s on {big_pages} pages vs {small:.4}s on {small_pages} \
         (time ratio {time_ratio:.2}x against a page ratio of {page_ratio:.2}x). Classification \
         work must not track page count — check pages_content_scanned first, then look for a \
         phase that walks the whole document."
    );
}

// -------------------------------------------------------------------------------------------
// 2. Three exit codes
// -------------------------------------------------------------------------------------------

/// Every outcome gets its own code, and no two collapse.
///
/// The fixture choices are **measured, not assumed**. `docs/history/05-MILESTONES.md` originally named
/// `irs-form-1040-2025` as the exit-0 case; it is not — it fires `table-likely` and
/// `dense-graphics`, so under the derivation rule it is exit 1. Suppressing a true layout reason
/// to make a doc line come out right would be exactly the tuning this project refuses, so the
/// doc was corrected instead.
#[test]
fn the_three_exit_codes_are_distinguishable() {
    let profile = Profile::default();

    let cases: Vec<(&str, PathBuf, i32)> = vec![
        // 0 — read cleanly, nothing fired on either axis.
        (
            "simple-text",
            conformance("synthetic/simple-text/document.pdf"),
            SIMPLE,
        ),
        (
            "two-lines",
            conformance("synthetic/two-lines/document.pdf"),
            SIMPLE,
        ),
        // 1 — read cleanly, at least one reason fired.
        (
            "image-only (no-text)",
            conformance("failure/image-only-or-blank-page/document.pdf"),
            NEEDS_ATTENTION,
        ),
        (
            "irs-1040 (layout only)",
            bench("irs-form-1040-2025.pdf"),
            NEEDS_ATTENTION,
        ),
        // 2 — could not read, for four different reasons.
        (
            "password-protected",
            conformance("failure/password-protected/document.pdf"),
            COULD_NOT_READ,
        ),
        (
            "invalid-header",
            conformance("failure/invalid-header/document.pdf"),
            COULD_NOT_READ,
        ),
        (
            // v0.1 repairs the 19-byte class, so the exit-2 case here is a malformation that
            // is NOT repairable — see `the_repaired_fixture_classifies_and_declares_it`.
            "corrupt-header-valid (unrepairable xref)",
            conformance("failure/corrupt-header-valid/document.pdf"),
            COULD_NOT_READ,
        ),
        (
            "missing file",
            repo_root().join("no/such/file.pdf"),
            COULD_NOT_READ,
        ),
    ];

    for (label, path, want) in cases {
        let got = exit_code(&run(path, &profile));
        assert_eq!(got, want, "{label} should exit {want}, got {got}");
    }
}

#[test]
fn a_missing_file_is_could_not_read_not_a_panic() {
    let r = run(
        repo_root().join("definitely/absent.pdf"),
        &Profile::default(),
    );
    assert_eq!(exit_code(&r), COULD_NOT_READ);
    assert_eq!(r.unwrap_err().code(), "io");
}

/// Unknown magic fails closed with a named error.
#[test]
fn unknown_magic_fails_closed() {
    let dir = std::env::temp_dir().join("ethos-parser-m2-magic");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("looks-like.pdf");
    std::fs::write(&path, b"<!DOCTYPE html><html>not a pdf at all</html>").expect("write");

    let r = run(path.clone(), &Profile::default());
    assert_eq!(exit_code(&r), COULD_NOT_READ);
    let e = r.unwrap_err();
    assert_eq!(
        e.code(),
        "unsupported",
        "the extension is not authority: a `.pdf` holding HTML is HTML"
    );

    let _ = std::fs::remove_file(&path);
}

// -------------------------------------------------------------------------------------------
// 3. Axis independence
// -------------------------------------------------------------------------------------------

/// A document whose layout is hard but whose text is fine fires **only** layout reasons.
///
/// `irs-form-1040-2025` is a real, measured instance: heavy ruling lines and vector drawing, with
/// crisp born-digital text throughout. A caller routing on a single "complex" list would send it
/// to an OCR engine that could only make it worse.
#[test]
fn layout_reasons_never_leak_onto_the_ocr_axis() {
    let c = run_ok(bench("irs-form-1040-2025.pdf"));

    assert!(
        c.layout_reasons
            .contains(&LayoutComplexityReason::TableLikely),
        "expected table-likely on the layout axis; got {:?}",
        c.layout_reasons
    );
    assert!(
        c.ocr_reasons.is_empty(),
        "a table is not a reason to run OCR; OCR axis should be empty, got {:?}",
        c.ocr_reasons
    );
    assert_eq!(
        c.pages_with_text, c.pages_sampled,
        "every sampled page has text"
    );
    assert!(
        c.needs_attention,
        "layout reasons alone still need attention"
    );
}

/// Statically: the two axes cannot be confused, because they are different types.
#[test]
fn the_axes_are_separate_types_across_every_fixture() {
    for rel in [
        "synthetic/simple-text/document.pdf",
        "synthetic/two-columns/document.pdf",
        "failure/image-only-or-blank-page/document.pdf",
    ] {
        let c = run_ok(conformance(rel));
        // A layout spelling can never appear in the OCR list: the lists hold different types, so
        // this is enforced at compile time. Asserted anyway on the wire, where types are gone.
        let ocr: Vec<&str> = c.ocr_reasons.iter().map(|r| r.as_str()).collect();
        for l in LayoutComplexityReason::ALL {
            assert!(
                !ocr.contains(&l.as_str()),
                "{rel}: {} leaked onto the OCR axis",
                l.as_str()
            );
        }
    }
}

// -------------------------------------------------------------------------------------------
// 4. Boolean derivation
// -------------------------------------------------------------------------------------------

/// `needs_attention` is derived, and the lists are the truth.
#[test]
fn the_boolean_is_derived_across_the_corpus() {
    let all = [
        conformance("synthetic/simple-text/document.pdf"),
        conformance("synthetic/two-lines/document.pdf"),
        conformance("synthetic/two-columns/document.pdf"),
        conformance("synthetic/rotation-90/document.pdf"),
        conformance("failure/image-only-or-blank-page/document.pdf"),
        bench("irs-form-1040-2025.pdf"),
        bench("nist-sp-800-63b.pdf"),
    ];
    for path in all {
        let label = path.display().to_string();
        let c = run_ok(path);
        assert_eq!(
            c.needs_attention,
            c.derive_needs_attention(),
            "{label}: the stored boolean must equal the derivation"
        );
        assert_eq!(
            c.needs_attention,
            !c.ocr_reasons.is_empty() || !c.layout_reasons.is_empty(),
            "{label}: the formula is the definition"
        );
    }
}

/// Removing the boolean loses no information.
#[test]
fn stripping_the_boolean_loses_nothing() {
    for rel in [
        "synthetic/simple-text/document.pdf",
        "failure/image-only-or-blank-page/document.pdf",
    ] {
        let c = run_ok(conformance(rel));
        let mut v = serde_json::to_value(&c).expect("serializes");
        v.as_object_mut().expect("object").remove("needs_attention");

        // Reconstructed from the lists alone.
        let ocr = v["ocr_reasons"].as_array().expect("array").len();
        let layout = v["layout_reasons"].as_array().expect("array").len();
        assert_eq!(
            (ocr > 0 || layout > 0),
            c.needs_attention,
            "{rel}: the boolean must be recoverable from the lists"
        );
    }
}

// -------------------------------------------------------------------------------------------
// 5. Page indexing
// -------------------------------------------------------------------------------------------

#[test]
fn page_indices_are_one_based() {
    let c = run_ok(conformance("synthetic/two-lines/document.pdf"));
    assert_eq!(c.pages[0].index, 1, "the first page is 1, never 0");

    let c = run_ok(bench("nist-sp-800-63b.pdf"));
    assert_eq!(c.pages[0].index, 1);
    let indices: Vec<u32> = c.pages.iter().map(|p| p.index).collect();
    assert_eq!(
        indices,
        (1..=8).collect::<Vec<u32>>(),
        "sampled pages are the first N, numbered from 1"
    );
}

#[test]
fn page_indices_survive_the_wire() {
    let c = run_ok(bench("nist-sp-800-63b.pdf"));
    let bytes = c.to_canonical_bytes().expect("canonical");
    let back: Classification = serde_json::from_slice(&bytes).expect("round trip");
    assert_eq!(back.pages[0].index, 1);
    assert_eq!(back, c);
}

// -------------------------------------------------------------------------------------------
// 6. simple-text golden — recorded, not tuned
// -------------------------------------------------------------------------------------------

/// What this engine honestly reports for `synthetic/simple-text`.
///
/// **Not tuned to match any competitor.** pdf-inspector calls this document TEXT-BASED while
/// reporting `Pages with text: 0`; LiteParse calls it `no-text` and demands OCR. Both are
/// calibrated for real-world pages and neither is safe on an eleven-byte fixture.
///
/// This engine reports: one page, one text-showing operator, eleven bytes of text, no imagery,
/// and therefore **no reasons on either axis** — because short text without competing content is
/// a short page, not a sparse one. If a future change makes this fire `sparse-text`, that is the
/// LiteParse trap arriving, and this test is where it surfaces.
#[test]
fn simple_text_golden() {
    let c = run_ok(conformance("synthetic/simple-text/document.pdf"));

    assert_eq!(c.page_count, 1);
    assert_eq!(c.pages_sampled, 1);
    assert_eq!(c.pages_content_scanned, 1);
    assert_eq!(c.pages_with_text, 1);

    assert_eq!(c.ocr_reasons, Vec::<OcrNeedReason>::new());
    assert_eq!(c.layout_reasons, Vec::<LayoutComplexityReason>::new());
    assert!(!c.needs_attention);

    let p = &c.pages[0];
    assert_eq!(p.index, 1);
    assert_eq!(p.text_operators, 1);
    assert_eq!(p.text_bytes, 11);
    assert_eq!(p.image_count, 0);
    assert_eq!(p.path_operators, 0);
    assert_eq!(p.rectangles, 0);
    assert_eq!(p.annotations, 0);

    assert_eq!(exit_code(&Ok(c)), SIMPLE);
}

// -------------------------------------------------------------------------------------------
// 7. No confidence
// -------------------------------------------------------------------------------------------

/// No confidence anywhere in the emitted artifact, for any fixture.
#[test]
fn no_confidence_in_any_emitted_classification() {
    for path in [
        conformance("synthetic/simple-text/document.pdf"),
        conformance("failure/image-only-or-blank-page/document.pdf"),
        bench("irs-form-1040-2025.pdf"),
        bench("nist-sp-800-63b.pdf"),
    ] {
        let label = path.display().to_string();
        let c = run_ok(path);
        let json = String::from_utf8(c.to_canonical_bytes().expect("canonical")).expect("utf8");
        let lower = json.to_ascii_lowercase();
        for banned in ["confidence", "\"score\"", "quality", "is_good"] {
            assert!(
                !lower.contains(banned),
                "{label}: `{banned}` appears in the classification artifact, which \
                 docs/01-CONTRACT.md §9 forbids"
            );
        }
    }
}

/// The `garbled` reason is never emitted, and the artifact says so rather than staying silent.
#[test]
fn garbled_is_never_emitted_and_the_absence_is_declared() {
    for path in [
        conformance("synthetic/simple-text/document.pdf"),
        conformance("synthetic/ligature-fi-embedded-font/document.pdf"),
        conformance("failure/image-only-or-blank-page/document.pdf"),
        bench("nist-sp-800-53r5.pdf"),
        bench("nist-sp-800-63b.pdf"),
        bench("irs-form-1040-2025.pdf"),
    ] {
        let label = path.display().to_string();
        let c = run_ok(path);
        assert!(
            !c.ocr_reasons.contains(&OcrNeedReason::Garbled),
            "{label}: no detector for `garbled` exists, so it must never be emitted"
        );
        assert!(
            !c.pages
                .iter()
                .any(|p| p.ocr_reasons.contains(&OcrNeedReason::Garbled)),
            "{label}: nor on any page row"
        );

        // Silence would be dishonest: a caller could read the empty list as evidence the document
        // is not garbled. The artifact declares that no detector exists.
        //
        // M4 moved this declaration out of the ad-hoc `not_detected` list and into
        // `assurance.limitations`, where every other declared gap already lived. One vocabulary,
        // one place to look — the migration `docs/README.md` had open since M2.
        for reason in ["garbled", "multi-column"] {
            let code = ethos_parser_pdf::limitations::undetected_reason_code(reason);
            assert!(
                c.assurance.limitations.iter().any(|l| l.code == code),
                "{label}: `{reason}` must be declared undetected as `{code}`; got {:?}",
                c.assurance
                    .limitations
                    .iter()
                    .map(|l| l.code.as_str())
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// An acronym-dense page is not reported as textless.
///
/// The LiteParse compounding-garble trap: an item-level vowel floor strips items from the length
/// tally, which can then drive `text_length < 20` → `no-text` on a page whose text extracted
/// perfectly. `nist-sp-800-53r5` is the exact shape that triggers it — a control catalogue dense
/// with `AC-2`, `SC-7`, `SI-4`.
#[test]
fn an_acronym_dense_document_is_not_reported_as_textless() {
    let c = run_ok(bench("nist-sp-800-53r5.pdf"));
    assert_eq!(
        c.pages_with_text, c.pages_sampled,
        "every sampled page of a text-bearing control catalogue must count as having text"
    );
    assert!(
        !c.ocr_reasons.contains(&OcrNeedReason::NoText),
        "a document full of AC-2/SC-7 identifiers must not be reported as having no text"
    );
    assert!(!c.ocr_reasons.contains(&OcrNeedReason::SparseText));
}

// -------------------------------------------------------------------------------------------
// 9. Profile sensitivity for N
// -------------------------------------------------------------------------------------------

#[test]
fn changing_n_changes_both_the_profile_hash_and_the_observation() {
    let n1 = Profile {
        classify_sample_pages: 1,
        ..Profile::default()
    };
    let n8 = Profile::default();

    assert_ne!(
        n1.profile_sha256().unwrap(),
        n8.profile_sha256().unwrap(),
        "N is output-affecting, so it must move the profile hash"
    );

    let c1 = run(bench("nist-sp-800-63b.pdf"), &n1).expect("classifies");
    let c8 = run(bench("nist-sp-800-63b.pdf"), &n8).expect("classifies");

    assert_eq!(c1.pages_sampled, 1);
    assert_eq!(c8.pages_sampled, 8);
    assert_eq!(c1.pages_content_scanned, 1);
    assert_eq!(c8.pages_content_scanned, 8);
    assert_eq!(c1.page_count, c8.page_count, "the document did not change");
    assert_ne!(
        c1.identity.profile_sha256, c8.identity.profile_sha256,
        "artifacts from different sample counts are correctly non-comparable"
    );
}

// -------------------------------------------------------------------------------------------
// 10. Determinism
// -------------------------------------------------------------------------------------------

#[test]
fn classification_is_byte_identical_across_runs() {
    let profile = Profile::default();
    for rel in ["nist-sp-800-63b.pdf", "irs-form-1040-2025.pdf"] {
        let a = run(bench(rel), &profile)
            .expect("classifies")
            .to_canonical_bytes()
            .unwrap();
        let b = run(bench(rel), &profile)
            .expect("classifies")
            .to_canonical_bytes()
            .unwrap();
        assert_eq!(a, b, "{rel}: two runs must produce identical bytes");
    }

    for rel in [
        "synthetic/simple-text/document.pdf",
        "synthetic/two-columns/document.pdf",
    ] {
        let a = run(conformance(rel), &profile)
            .unwrap()
            .to_canonical_bytes()
            .unwrap();
        let b = run(conformance(rel), &profile)
            .unwrap()
            .to_canonical_bytes()
            .unwrap();
        assert_eq!(a, b, "{rel}: two runs must produce identical bytes");
    }
}

#[test]
fn the_artifact_carries_a_full_identity_envelope() {
    let c = run_ok(conformance("synthetic/simple-text/document.pdf"));
    assert_eq!(
        c.identity.artifact_type,
        ethos_parser_pdf::CLASSIFICATION_ARTIFACT_TYPE
    );
    assert_eq!(
        c.identity.schema_version,
        ethos_parser_pdf::CLASSIFICATION_SCHEMA_VERSION
    );
    assert_eq!(c.identity.parser_version, Profile::default().parser_version);
    assert_eq!(
        c.identity.profile_sha256,
        Profile::default().profile_sha256().unwrap()
    );
    assert_eq!(c.source.media_type, "application/pdf");
}

#[test]
fn the_artifact_contains_no_host_varying_data() {
    let c = run_ok(conformance("synthetic/simple-text/document.pdf"));
    let json = String::from_utf8(c.to_canonical_bytes().unwrap()).unwrap();
    for forbidden in [
        "/Users",
        "/home",
        "/tmp",
        "elapsed",
        "timestamp",
        "hostname",
        ".pdf",
    ] {
        assert!(
            !json.contains(forbidden),
            "`{forbidden}` appears in the artifact; default output must be fingerprint-stable"
        );
    }
}
