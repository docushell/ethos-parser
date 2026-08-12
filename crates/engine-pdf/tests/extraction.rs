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

//! M3 acceptance (`docs/05-MILESTONES.md`).
//!
//! Fixtures resolve through `fixtures/manifest.json`'s three roots exactly as the oracle harness
//! resolves them. **A missing corpus is a failure, never a skip.**

use std::path::PathBuf;

use engine_core::{DerivationClass, GeometryAbsence, GeometryPresence, Profile};
use engine_pdf::{Document, ExtractArtifact, TextRun};

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

fn extract_ok(path: PathBuf) -> ExtractArtifact {
    let profile = Profile::default();
    let doc = Document::open(&path, &profile).expect("document opens");
    engine_pdf::extract(&doc, &profile).expect("document extracts")
}

fn runs(a: &ExtractArtifact) -> Vec<&TextRun> {
    a.runs().collect()
}

// -------------------------------------------------------------------------------------------
// 1. All four show-text operators
// -------------------------------------------------------------------------------------------

/// `'` and `"` produce text, and their text is not lost.
///
/// The `"` operator is absent from pdf-inspector's operator match: its text vanishes, adjacent
/// runs merge with corrupt geometry, and the output stays well-formed — undetectable downstream
/// (parity checklist P6). This is the fixture that would have caught it.
#[test]
fn the_quote_show_text_operators_do_not_lose_text() {
    let a = extract_ok(engine_fx("show-text-quote-operators"));
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();

    assert_eq!(
        texts,
        vec!["plain Tj", "apostrophe op", "quote op"],
        "all three show-text operators must produce their text"
    );

    // Each landed on its own line, so the runs did not merge.
    let ys: Vec<i64> = runs(&a).iter().map(|r| r.locator.origin_y).collect();
    let mut sorted = ys.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        3,
        "each operator moved to its own line: {ys:?}"
    );
}

// -------------------------------------------------------------------------------------------
// 2. Unknown operator fails closed
// -------------------------------------------------------------------------------------------

#[test]
fn an_unknown_operator_produces_no_artifact() {
    use engine_pdf::ops::Operator;

    // Direct assertion on the table: a token outside PDF 32000-1 Table A.1 resolves to nothing,
    // and the interpreter turns that into a hard error rather than a skip.
    assert_eq!(Operator::from_token("UnknownOp"), None);

    let fonts = std::collections::BTreeMap::new();
    let mut interp = engine_pdf::content::Interpreter::new(&fonts);
    let ops = lopdf::content::Content::decode(b"q 1 0 0 1 0 0 cm UnknownOp Q")
        .expect("decodes")
        .operations;

    let e = interp
        .run(&ops)
        .expect_err("an unknown operator must stop the parse");
    assert_eq!(e.code(), "unsupported");
    assert!(
        e.to_string().contains("UnknownOp"),
        "the token must be named: {e}"
    );
    assert!(
        interp.shown.is_empty(),
        "no partial output may survive a fail-closed parse"
    );
}

// -------------------------------------------------------------------------------------------
// 3. Tz applies to the advance
// -------------------------------------------------------------------------------------------

/// The same string at `Tz 100` and `Tz 50` advances in exactly a 2:1 ratio.
///
/// pdf-inspector implements no `Tz` anywhere in its tree, so its advance widths are wrong on any
/// document that uses it.
#[test]
fn horizontal_scaling_changes_the_advance() {
    let a = extract_ok(engine_fx("horizontal-scaling-tz"));
    let rs = runs(&a);
    assert_eq!(rs.len(), 2, "one run per Tz setting");
    assert_eq!(rs[0].text, "AAAA");
    assert_eq!(rs[1].text, "AAAA", "identical text, so only Tz differs");

    let full = rs[0].locator.advance.expect("widths are in the document");
    let half = rs[1].locator.advance.expect("widths are in the document");
    assert_eq!(
        full,
        half * 2,
        "Tz 50 must halve the advance: got {full} and {half}"
    );
}

// -------------------------------------------------------------------------------------------
// 4. No box derived from a font size
// -------------------------------------------------------------------------------------------

/// A font with no metrics yields typed absence, never a font-size-shaped box.
#[test]
fn a_font_without_metrics_yields_typed_absence() {
    // The conformance corpus is standard-14 Helvetica with no FontDescriptor.
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    let r = runs(&a)[0];

    assert_eq!(
        r.geometry,
        GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        "no metrics means no box"
    );
    assert!(
        r.font_size > 0,
        "the font size is still recorded — it is a real property"
    );
    assert!(
        r.geometry.measured().is_none(),
        "and it is not reachable as a box"
    );
}

/// The measured path produces a box computed from declared ascent and descent, not from the size.
#[test]
fn measured_metrics_produce_a_box_that_is_not_the_font_size() {
    let a = extract_ok(engine_fx("measured-ink-box"));
    let r = runs(&a)[0];
    let rect = r
        .geometry
        .measured()
        .expect("this fixture carries a FontDescriptor");

    // Ascent 718, descent -207, size 24pt, baseline at 72pt from the top.
    // top    = 72 - 718/1000*24 = 54.768pt -> 5477 centipoints
    // bottom = 72 + 207/1000*24 = 76.968pt -> 7697 centipoints
    assert_eq!(rect.y0(), 5477, "top edge is baseline minus scaled ascent");
    assert_eq!(
        rect.y1(),
        7697,
        "bottom edge is baseline plus scaled descent"
    );

    let height = rect.y1() - rect.y0();
    assert_ne!(
        height, r.font_size,
        "the box height must not equal the font size — that conflation is pdf-inspector's P5 defect"
    );
    assert_eq!(height, 2220, "718 + 207 = 925 thousandths of 24pt");
}

/// Structural: no code path assigns a box dimension from the font size.
#[test]
fn no_source_line_derives_a_box_from_the_font_size() {
    let src_dir = repo_root().join("crates/engine-pdf/src");
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(&src_dir).expect("src readable") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let src = std::fs::read_to_string(&path).expect("readable");
            for (n, line) in src.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                let squashed: String = line.chars().filter(|c| !c.is_whitespace()).collect();
                for bad in ["height=font_size", "height:font_size", "height=fontsize"] {
                    if squashed.contains(bad) {
                        offenders.push(format!("{}:{}", path.display(), n + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "a box dimension is derived from the font size: {offenders:?}"
    );
}

// -------------------------------------------------------------------------------------------
// 5. Synthesized characters
// -------------------------------------------------------------------------------------------

/// A `TJ` gap wide enough to be a word break inserts a space, flagged at the character.
#[test]
fn a_synthesized_space_is_flagged_where_it_is_created() {
    let a = extract_ok(engine_fx("synthesized-space-tj"));
    let rs = runs(&a);

    let first = rs
        .iter()
        .find(|r| r.text.starts_with("one"))
        .expect("the first run");
    assert_eq!(first.text, "one ", "the gap became a space");
    assert_eq!(first.synthesized.len(), 1, "exactly one inserted character");
    assert_eq!(
        first.synthesized[0].char_index, 3,
        "flagged at the index it occupies in the run's text"
    );
    assert_eq!(
        first.synthesized[0].reason,
        engine_pdf::SynthesisReason::TjGap
    );

    // The document's own characters are not flagged.
    let second = rs.iter().find(|r| r.text == "two").expect("the second run");
    assert!(second.synthesized.is_empty());
}

#[test]
fn synthesized_flags_survive_canonicalization() {
    let a = extract_ok(engine_fx("synthesized-space-tj"));
    let bytes = a.to_canonical_bytes().expect("canonical");
    let s = String::from_utf8(bytes.clone()).expect("utf8");
    assert!(
        s.contains("\"reason\":\"tj-gap\""),
        "the flag reaches the wire"
    );

    let back: ExtractArtifact = serde_json::from_slice(&bytes).expect("round trips");
    assert_eq!(back, a);
}

#[test]
fn text_taken_verbatim_carries_no_synthesis_flags() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    for r in runs(&a) {
        assert!(
            r.synthesized.is_empty(),
            "`{}` was read from the document and must not be flagged as authored",
            r.text
        );
    }
}

// -------------------------------------------------------------------------------------------
// 6. Ligature caveat
// -------------------------------------------------------------------------------------------

/// One code, several scalars — declared, not reconciled.
#[test]
fn the_ligature_fixture_declares_its_scalar_code_mismatch() {
    let a = extract_ok(conformance(
        "synthetic/ligature-fi-embedded-font/document.pdf",
    ));
    let r = runs(&a)[0];

    assert_eq!(r.text, "office file");
    assert_eq!(
        r.char_codes,
        vec![1, 2, 3, 4, 5, 6, 3, 7, 5],
        "nine codes, as the content stream writes them"
    );
    assert_eq!(r.text.chars().count(), 11, "eleven characters");
    assert!(
        r.scalar_code_mismatch,
        "11 scalars from 9 codes must be declared on the wire"
    );
    assert_eq!(
        r.scalar_code_mismatch,
        r.compute_scalar_code_mismatch(),
        "the stored flag must equal the recomputed one"
    );
}

#[test]
fn a_run_without_ligatures_declares_no_mismatch() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    let r = runs(&a)[0];
    assert!(!r.scalar_code_mismatch);
    assert_eq!(r.text.chars().count(), r.char_codes.len());
}

// -------------------------------------------------------------------------------------------
// 7. Hyphenation policy — explicit non-rejoin
// -------------------------------------------------------------------------------------------

/// v0 does **not** rejoin hyphenated line breaks, and the golden records that.
///
/// The alternative would be a `Computed`, reversible rejoin. It is not implemented because
/// distinguishing a soft break-hyphen from a real compound hyphen ("well-known" split across
/// lines) needs a dictionary, and a rule that guesses is the cliff-shaped heuristic this project
/// refuses everywhere else. Silent rejoining is forbidden outright; **not** rejoining is a stated
/// policy with this test as its record.
#[test]
fn hyphenated_line_breaks_are_not_rejoined_and_that_is_the_policy() {
    let a = extract_ok(conformance("synthetic/hyphenated-line-break/document.pdf"));
    let rs = runs(&a);

    assert_eq!(rs.len(), 2, "two runs, one per line");
    assert_eq!(
        rs[0].text, "hyphen-",
        "the trailing hyphen is preserved verbatim"
    );
    assert_eq!(rs[1].text, "ated");

    for r in &rs {
        assert_eq!(
            r.derivation,
            DerivationClass::Extracted,
            "no Computed rejoin node exists, so nothing claims to be derived"
        );
        assert!(r.synthesized.is_empty(), "and nothing was inserted");
    }

    // The source bytes are trivially recoverable: they are the run texts, unmodified.
    let joined: String = rs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(joined, "hyphen-ated");
}

// -------------------------------------------------------------------------------------------
// 8. Rotation
// -------------------------------------------------------------------------------------------

#[test]
fn a_rotated_page_reports_its_rotation_and_transformed_geometry() {
    let a = extract_ok(conformance("synthetic/rotation-90/document.pdf"));
    let page = &a.pages[0];

    assert_eq!(page.rotation, 90, "the page declares its rotation");
    // MediaBox is 144x300; a quarter turn swaps the visible dimensions.
    assert_eq!(page.width, 30000, "display width is the media height");
    assert_eq!(page.height, 14400, "display height is the media width");

    let r = &page.runs[0];
    assert_eq!(r.text, "Rotate Ninety");
    // Content places the text at user (36, 72). Under /Rotate 90 the bottom-left corner becomes
    // the top-left, so (x, y) -> (y, x).
    assert_eq!(
        (r.locator.origin_x, r.locator.origin_y),
        (7200, 3600),
        "geometry is in the declared system after rotation, not raw user space"
    );

    // And it lands inside the page.
    assert!(r.locator.origin_x <= page.width);
    assert!(r.locator.origin_y <= page.height);
}

#[test]
fn unrotated_pages_report_rotation_zero() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    assert_eq!(a.pages[0].rotation, 0);
}

// -------------------------------------------------------------------------------------------
// 9. Hostile xref
// -------------------------------------------------------------------------------------------

#[test]
fn the_hostile_xref_fixture_fails_to_open_with_a_named_error() {
    let profile = Profile::default();
    let e = Document::open(
        &conformance("synthetic/table-regular-grid/document.pdf"),
        &profile,
    )
    .expect_err("19-byte xref entries where PDF 32000-1 §7.5.4 requires 20");

    assert_eq!(e.code(), "malformed");
    assert!(
        !e.to_string().is_empty(),
        "the failure must be named, not a panic"
    );
}

// -------------------------------------------------------------------------------------------
// 10. Determinism
// -------------------------------------------------------------------------------------------

/// Every fixture that opens extracts to identical bytes twice.
#[test]
fn extraction_is_byte_identical_across_runs() {
    let mut checked = 0;
    for f in manifest()["fixtures"].as_array().expect("fixtures") {
        let root = f["root"].as_str().expect("root");
        let rel = f["path"].as_str().expect("path");
        let path = path_in(root, rel);

        let profile = Profile::default();
        let Ok(doc) = Document::open(&path, &profile) else {
            continue; // fixtures that do not open are covered by their own tests
        };
        let (Ok(a), Ok(b)) = (
            engine_pdf::extract(&doc, &profile),
            engine_pdf::extract(&doc, &profile),
        ) else {
            continue;
        };
        assert_eq!(
            a.to_canonical_bytes().unwrap(),
            b.to_canonical_bytes().unwrap(),
            "{rel}: two extracts must produce identical bytes"
        );
        checked += 1;
    }
    assert!(
        checked >= 10,
        "expected to check many fixtures, checked {checked}"
    );
}

// -------------------------------------------------------------------------------------------
// 11. Every run carries a native locator
// -------------------------------------------------------------------------------------------

#[test]
fn every_run_carries_a_native_locator() {
    let mut total = 0;
    for f in manifest()["fixtures"].as_array().expect("fixtures") {
        let path = path_in(f["root"].as_str().unwrap(), f["path"].as_str().unwrap());
        let profile = Profile::default();
        let Ok(doc) = Document::open(&path, &profile) else {
            continue;
        };
        let Ok(a) = engine_pdf::extract(&doc, &profile) else {
            continue;
        };
        for r in a.runs() {
            assert!(r.locator.page >= 1, "page numbers are 1-based");
            assert!(
                r.locator.page <= a.page_count,
                "a locator must point inside the document"
            );
            assert_eq!(r.derivation, DerivationClass::Extracted);
            total += 1;
        }
    }
    assert!(total > 10, "expected runs across the corpus, got {total}");
}

// -------------------------------------------------------------------------------------------
// 12. Single load — classify and extract share one handle
// -------------------------------------------------------------------------------------------

/// One `Document`, two stages, no reopen.
///
/// A correctness rule before a performance one: two loads can disagree, and a classifier that saw
/// a different object graph from the extractor is a silent divergence with no diagnostic.
#[test]
fn classify_and_extract_share_one_open_document() {
    let profile = Profile::default();
    let path = conformance("synthetic/two-lines/document.pdf");

    let doc = Document::open(&path, &profile).expect("opens once");

    let c = engine_pdf::classify(&doc, &profile).expect("classifies");
    let e = engine_pdf::extract(&doc, &profile).expect("extracts from the same handle");

    assert_eq!(
        c.page_count, e.page_count,
        "both stages see the same document"
    );
    assert_eq!(
        c.source.sha256, e.source.sha256,
        "and bind to the same source bytes"
    );
    assert_eq!(c.pages_with_text, 1);
    assert_eq!(e.runs().count(), 2);

    // Order does not matter either.
    let e2 = engine_pdf::extract(&doc, &profile).expect("extracts again");
    let c2 = engine_pdf::classify(&doc, &profile).expect("classifies again");
    assert_eq!(
        e2.to_canonical_bytes().unwrap(),
        e.to_canonical_bytes().unwrap()
    );
    assert_eq!(
        c2.to_canonical_bytes().unwrap(),
        c.to_canonical_bytes().unwrap()
    );
}

// -------------------------------------------------------------------------------------------
// 13. No confidence
// -------------------------------------------------------------------------------------------

#[test]
fn no_confidence_in_any_extract_artifact() {
    for path in [
        conformance("synthetic/simple-text/document.pdf"),
        conformance("synthetic/ligature-fi-embedded-font/document.pdf"),
        engine_fx("measured-ink-box"),
    ] {
        let a = extract_ok(path);
        let json = String::from_utf8(a.to_canonical_bytes().unwrap()).unwrap();
        let lower = json.to_ascii_lowercase();
        for banned in ["confidence", "\"score\"", "quality", "is_good"] {
            assert!(
                !lower.contains(banned),
                "`{banned}` appears in an extract artifact"
            );
        }
    }
}

#[test]
fn no_confidence_in_the_extract_source_modules() {
    let src_dir = repo_root().join("crates/engine-pdf/src");
    let mut hits = Vec::new();
    for entry in std::fs::read_dir(&src_dir).expect("src readable") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if !matches!(
            name.as_str(),
            "extract.rs" | "nodes.rs" | "content.rs" | "fonts.rs" | "metrics.rs" | "text_state.rs"
        ) {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("readable");
        for (n, line) in src.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.to_ascii_lowercase().contains("confidence") {
                hits.push(format!("{name}:{}", n + 1));
            }
        }
    }
    assert!(hits.is_empty(), "`confidence` in extract code: {hits:?}");
}

// -------------------------------------------------------------------------------------------
// 14. c14n round trip and identity
// -------------------------------------------------------------------------------------------

#[test]
fn the_artifact_round_trips_through_c14n() {
    let a = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    let bytes = a.to_canonical_bytes().expect("canonical");
    let back: ExtractArtifact = serde_json::from_slice(&bytes).expect("reparses");
    assert_eq!(back, a);
    assert_eq!(back.to_canonical_bytes().unwrap(), bytes, "idempotent");
}

#[test]
fn the_artifact_carries_a_full_identity_envelope() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    assert_eq!(a.identity.artifact_type, engine_pdf::EXTRACT_ARTIFACT_TYPE);
    assert_eq!(
        a.identity.schema_version,
        engine_pdf::EXTRACT_SCHEMA_VERSION
    );
    assert_eq!(
        a.identity.profile_sha256,
        Profile::default().profile_sha256().unwrap()
    );
    assert_eq!(a.reading_order_rule, "single-column-v1");
    assert_eq!(a.source.media_type, "application/pdf");
}

/// Reading order is stream order at v0, and `two-columns` shows what that costs.
#[test]
fn reading_order_is_stream_order_and_the_limitation_is_visible() {
    let a = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();

    // The content stream writes the right column first. v0 does not reorder, so the right
    // column comes out first — visibly wrong reading order, and honest about it. Multi-column
    // ships at v1 with a stable rule; pdf-inspector's flips on a single line of text.
    assert_eq!(
        texts,
        vec!["Right top", "Right bottom", "Left top", "Left bottom"],
        "v0 emits stream order; the multi-column limitation is real and declared"
    );
    assert_eq!(a.reading_order_rule, "single-column-v1");
}

/// The M3 `not_decoded` gaps, now carried as limitations.
///
/// M4 absorbed the list rather than leaving it beside the capability block: a reviewer reading an
/// artifact should find every declared gap in one place and one shape, not two vocabularies whose
/// relationship they have to work out.
#[test]
fn undecodable_gaps_are_declared_rather_than_silent() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();

    assert!(
        codes.contains(&engine_pdf::limitations::FONT_WIDTHS_ABSENT),
        "standard-14 Helvetica carries no /Widths, and this profile does not vendor the AFM \
         tables — the gap must be declared: {codes:?}"
    );
    assert!(
        codes.contains(&engine_pdf::limitations::FORM_XOBJECT_TEXT_NOT_DESCENDED),
        "text inside form XObjects is not descended into, and that is declared: {codes:?}"
    );
    for l in &a.assurance.limitations {
        assert!(l.detail.len() > 40, "`{}` needs a real reason", l.code);
    }
}

#[test]
fn an_absent_advance_is_absent_not_zero() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    let r = runs(&a)[0];
    assert_eq!(
        r.locator.advance, None,
        "no /Widths means the advance is unknown, and unknown is what it says"
    );
    // The origin is unaffected — it comes from the content stream, not the font.
    assert_eq!(r.locator.origin_x, 7200);
    assert_eq!(r.locator.origin_y, 7200);
}
