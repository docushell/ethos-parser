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

/// An unrecognised operator stops the parse, **through the public API**.
///
/// Rewritten at M7 as part of the API freeze. The earlier version reached into
/// `engine_pdf::ops` and `engine_pdf::content` to drive the interpreter directly, which made two
/// internal modules part of the published surface for the sake of one assertion. The interpreter
/// is now tested where it lives (`content.rs`), and this asserts the property a caller actually
/// depends on: a real document, one operator token overwritten in place, refused with a named
/// error and no artifact.
///
/// The substitution is same-length, so `/Length` stays honest and the document remains
/// structurally valid — the refusal is about the operator and nothing else.
#[test]
fn an_unknown_operator_produces_no_artifact() {
    let original = std::fs::read(engine_fx("measured-ink-box")).expect("fixture readable");
    let at = original
        .windows(4)
        .position(|w| w == b" Tj ")
        .expect("the fixture shows text with a Tj operator");
    let mut mutant = original.clone();
    mutant[at + 1..at + 3].copy_from_slice(b"Zq");

    let profile = Profile::default();

    // The control: unmodified, this fixture extracts.
    assert!(
        Document::open_bytes(&original, &profile)
            .and_then(|d| engine_pdf::extract(&d, &profile))
            .is_ok(),
        "the control must extract, or the mutant proves nothing"
    );

    let doc = Document::open_bytes(&mutant, &profile)
        .expect("the document is still structurally valid — only the operator changed");
    let e =
        engine_pdf::extract(&doc, &profile).expect_err("an unknown operator must stop the parse");

    assert_eq!(e.code(), "unsupported", "got {e}");
    assert!(
        e.to_string().contains("Zq"),
        "the offending token must be named: {e}"
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
fn the_hostile_xref_fixture_is_repaired_and_extracts_its_real_content() {
    let profile = Profile::default();
    let path = conformance("synthetic/table-regular-grid/document.pdf");

    // v0 refused this: 19-byte xref entries where PDF 32000-1 §7.5.4 requires 20. v0.1 repairs
    // that one class (docs/01-CONTRACT.md §12), so the document now yields its real content.
    let doc = Document::open(&path, &profile).expect("v0.1 repairs the 19-byte xref class");
    assert_eq!(doc.xref_entries_padded(), Some(6));

    let a = engine_pdf::extract(&doc, &profile).expect("and then extracts normally");
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Name", "Score", "Alpha", "10", "Beta", "12"],
        "the repaired document must yield the text it actually contains — a repair that \
         produced plausible-but-different content would be far worse than the refusal it replaced"
    );

    // The repair is on the wire, not just in the parser.
    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    assert!(
        codes.contains(&engine_pdf::limitations::XREF_ENTRY_PADDED),
        "a repaired open must declare itself: {codes:?}"
    );

    // This is a table document with NO ruling lines — its grid is text position alone. v1-S1
    // found nothing here and declared `unruled-tables-not-detected`; v1-S2 ships the alignment
    // rule, so the table IS found and that limitation is **gone rather than reworded**. A
    // limitation that outlives the gap it describes is worse than none, because a reader acts
    // on it.
    assert!(
        !codes.contains(&"unruled-tables-not-detected"),
        "v1-S2 detects this document's table, so the ruled-only limitation must be absent: \
         {codes:?}"
    );
    assert!(
        !codes.contains(&engine_core::codes::TABLES_NOT_EXTRACTED),
        "the false-capability partner is gone now that tables are detected: {codes:?}"
    );
    // The leftover that IS still true: a grid stroked as bare ruling lines is not read as ruled.
    assert!(
        codes.contains(&engine_core::codes::STROKE_RULED_TABLES_NOT_DETECTED),
        "the narrowed leftover must still be declared: {codes:?}"
    );

    // And the table itself: the S2 golden. Six `Tm`/`Tj` pairs, zero path operators, 3x2.
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1, "the alignment rule must find this grid");
    let t = tables[0];
    assert_eq!((t.rows, t.columns), (3, 2));
    assert_eq!(
        t.rule,
        engine_core::TABLE_DETECTION_UNRULED_V1,
        "which rule fired is on the table, not inferred from the profile"
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

// -------------------------------------------------------------------------------------------
// v0.1 — broken font encodings are limitations, never mojibake (parity checklist P10)
// -------------------------------------------------------------------------------------------

/// **A font that cannot map a code loses its run, and says so.**
///
/// The failure this replaces was not subtle: through v0 one unmappable glyph anywhere refused the
/// whole document, so a page with a single bad code yielded nothing at all. The failure it
/// *refuses to introduce* is the one every other reader has — emitting the base encoding's
/// characters for codes `/Differences` remapped, producing text the document does not contain
/// inside a perfectly well-formed artifact.
#[test]
fn a_broken_font_encoding_drops_its_run_and_declares_it() {
    let a = extract_ok(engine_fx("broken-font-encoding"));
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();

    // The decodable run survives, exactly.
    assert_eq!(
        texts,
        vec!["Readable"],
        "text the font CAN map must be extracted normally; only the unmappable run is lost"
    );

    // And the mojibake a naive reader would emit is absent. Codes 200/201/202 are
    // E-grave/E-acute/E-circumflex in WinAnsiEncoding, which is what a reader that ignored
    // `/Differences` — or fell back to the base encoding on a glyph-name miss — would produce.
    let all_text = texts.join("");
    for ch in ['\u{c8}', '\u{c9}', '\u{ca}', '\u{fffd}'] {
        assert!(
            !all_text.contains(ch),
            "the artifact contains U+{:04X}, which the document does not say. A substituted \
             character in an evidence artifact is indistinguishable downstream from one the \
             document really carried.",
            ch as u32
        );
    }

    // The loss is declared, with a count.
    let lim = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_pdf::limitations::BROKEN_FONT_ENCODING)
        .expect("a dropped run must be declared, not silently absent");
    assert!(
        lim.detail.contains("1 text run"),
        "the declaration must say how much is missing: {}",
        lim.detail
    );

    // The page was read, so its state is `processed` and the terminal state is `complete`.
    // Deliberate, and worth recording: `partial` in this contract means pages were never
    // attempted, not that some text on a read page was undecodable. The limitation is what
    // carries "the text here is incomplete", and it says so in as many words.
    assert_eq!(
        a.assurance.terminal_state,
        engine_core::ProcessingTerminalState::Complete
    );
    assert!(
        lim.detail.contains("must not infer"),
        "the declaration must warn against reading absence as blankness: {}",
        lim.detail
    );
}

/// A clean document declares no encoding limitation.
///
/// Guards the guard: a limitation attached unconditionally would make the test above pass on any
/// document at all.
#[test]
fn a_document_that_decodes_cleanly_claims_no_encoding_problem() {
    let a = extract_ok(engine_fx("measured-ink-box"));
    assert!(
        !a.assurance
            .limitations
            .iter()
            .any(|l| l.code == engine_pdf::limitations::BROKEN_FONT_ENCODING),
        "a document with no encoding holes must not claim one"
    );
}

/// **A document whose text layer decodes to nothing is refused outright.**
///
/// The line between "declare" and "refuse". An artifact carrying zero runs would be
/// indistinguishable from a genuinely blank page, and that is a difference a caller must be able
/// to see — so this one is `docs/01-CONTRACT.md` §8 territory rather than a limitation.
#[test]
fn a_document_that_decodes_nothing_is_refused_rather_than_emptied() {
    // Built here rather than by editing the fixture: removing a run from a PDF's content stream
    // moves every byte after it, so the xref offsets and `/Length` both go stale and the
    // document fails to *open* — which would have tested the trailer parser, not the encoding.
    let doc = pdf_with_only_unmappable_text();

    let profile = Profile::default();
    let d = Document::open_bytes(&doc, &profile).expect("still a structurally valid PDF");
    let e = engine_pdf::extract(&d, &profile)
        .expect_err("a document with no decodable text has no usable text layer");

    assert_eq!(e.code(), "unsupported", "got {e}");
    assert!(
        e.to_string().contains("text layer is unusable"),
        "the refusal must name what went wrong: {e}"
    );
}

/// One page, one font whose `/Differences` resolve to nothing, one run of unmappable codes.
///
/// The same shape `fixtures/engine/broken-font-encoding` carries, minus the readable run. Offsets
/// are computed, so the document is genuinely well-formed and the only thing wrong with it is the
/// encoding.
fn pdf_with_only_unmappable_text() -> Vec<u8> {
    let widths: String = std::iter::repeat_n("500", 95).collect::<Vec<_>>().join(" ");
    let stream = b"BT /F1 24 Tf 1 0 0 1 72 40 Tm (\\310\\311\\312) Tj ET";

    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] \
           /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        format!("<< /Length {} >>\nstream\n", stream.len())
            .into_bytes()
            .into_iter()
            .chain(stream.iter().copied())
            .chain(b"\nendstream".iter().copied())
            .collect(),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Type /Encoding \
             /BaseEncoding /WinAnsiEncoding /Differences [200 /nonexistentglyphone \
             /nonexistentglyphtwo /nonexistentglyphthree] >> /FirstChar 32 /LastChar 126 \
             /Widths [{widths}] >>"
        )
        .into_bytes(),
    ];

    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_at = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        // Exactly 20 bytes, per PDF 32000-1 §7.5.4 — this document is not exercising the repair.
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

// -------------------------------------------------------------------------------------------
// v1-S1 — ruled tables (docs/09-V1-MILESTONES.md S1)
// -------------------------------------------------------------------------------------------

/// **The ruled golden.** A 3×3 grid the document drew, with one merge and one empty cell.
#[test]
fn a_ruled_grid_is_reconstructed_with_spans_and_parent_ids() {
    let a = extract_ok(engine_fx("ruled-table-grid"));
    let page = &a.pages[0];
    assert_eq!(page.tables.len(), 1, "one drawn grid, one table");

    let t = &page.tables[0];
    assert_eq!((t.rows, t.columns), (3, 3));
    assert_eq!(t.cells.len(), 8, "9 slots, one cell covering two of them");

    // Zero-based, and span 1 means not merged.
    let cell = |row, col| {
        t.cells
            .iter()
            .find(|c| c.position.row == row && c.position.column == col)
            .unwrap_or_else(|| panic!("no cell at ({row}, {col})"))
    };
    assert_eq!(cell(0, 0).text, "Name");
    assert_eq!(cell(0, 1).text, "Q1");
    assert_eq!(cell(0, 2).text, "Q2");
    assert_eq!(cell(1, 0).text, "Alpha");
    assert_eq!(cell(1, 1).text, "10");

    // The merged cell owns both slots it covers.
    let merged = cell(2, 1);
    assert_eq!(merged.position.colspan, 2, "row 2 columns 1-2 are one cell");
    assert_eq!(merged.position.rowspan, 1);
    assert_eq!(merged.text, "n/a");
    assert_eq!(merged.position.slots().len(), 2);

    // Every cell names its parent, and no cell claims a span of zero.
    for c in &t.cells {
        assert_eq!(
            c.position.table_id, t.id,
            "a cell without its table is unaddressable"
        );
        assert!(c.position.is_well_formed(), "{:?}", c.position);
    }
}

/// **Fabrication 0.** A cell whose rectangle encloses no text is empty, not borrowed.
#[test]
fn a_cell_enclosing_no_text_is_empty_rather_than_filled_from_nearby() {
    let a = extract_ok(engine_fx("ruled-table-grid"));
    let t = &a.pages[0].tables[0];
    let empty = t
        .cells
        .iter()
        .find(|c| c.position.row == 1 && c.position.column == 2)
        .expect("row 1 column 2 exists");

    assert_eq!(
        empty.text, "",
        "the fixture puts no text in this cell; anything here came from a neighbour"
    );
    assert!(empty.run_indices.is_empty());
}

/// **Fabrication 0, as a property rather than an example.**
///
/// Every cell's text is the concatenation of the runs assigned to it, in order — so it is a
/// rearrangement of text the document contains and can never be a novel string.
#[test]
fn every_cell_text_is_built_only_from_extracted_runs() {
    for name in ["ruled-table-grid", "ruled-table-overlap"] {
        let a = extract_ok(engine_fx(name));
        for page in &a.pages {
            let run_texts: Vec<&str> = page.runs.iter().map(|r| r.text.as_str()).collect();
            for t in &page.tables {
                for c in &t.cells {
                    let rebuilt: String = c.run_indices.iter().map(|i| run_texts[*i]).collect();
                    assert_eq!(
                        c.text, rebuilt,
                        "{name}: a cell's text must be exactly its assigned runs"
                    );
                    for i in &c.run_indices {
                        assert!(*i < run_texts.len(), "{name}: run index out of range");
                    }
                }
            }
        }
    }
}

/// **The cross-check agrees on the golden**, and disagrees on the hostile fixture.
#[test]
fn the_locator_cross_check_reports_agreement_and_disagreement() {
    use engine_core::CheckStatus;

    let good = extract_ok(engine_fx("ruled-table-grid"));
    let t = &good.pages[0].tables[0];
    assert_eq!(
        t.check.outcome,
        engine_core::CheckStatus::Ok,
        "{:?}",
        t.check
    );
    assert_eq!(t.check.check_id, engine_core::LOCATOR_CHECK_V1);

    // The hostile fixture draws overlapping rectangles. The engine must SAY so — and must not
    // nudge a coordinate to make the grid tile.
    let bad = extract_ok(engine_fx("ruled-table-overlap"));
    let hostile = bad
        .pages
        .iter()
        .flat_map(|p| p.tables.iter())
        .find(|t| !matches!(t.check.outcome, CheckStatus::Ok))
        .expect("the overlap fixture must produce a mismatch");

    match &hostile.check.outcome {
        CheckStatus::Mismatch {
            structural,
            geometric,
        } => {
            assert!(
                !structural.is_empty() || !geometric.is_empty(),
                "a mismatch must name what disagreed"
            );
        }
        other => panic!("expected a mismatch, got {other:?}"),
    }
}

/// **Looked, found none.** A page whose text implies no grid produces an empty table list, not an
/// absent one and not a fabricated table around the page.
///
/// Narrowed at v1-S2. Through S1 this also covered `synthetic/table-regular-grid`, whose 3×2 grid
/// is laid out by text position with no path operators at all — S1 could only look for painted
/// rectangles, so it correctly found nothing there. S2 ships the alignment rule and that document
/// became the golden, so it moved to
/// `the_alignment_rule_finds_the_grid_the_ruled_rule_could_not`. What is left here is the case
/// that must stay empty under *both* rules: prose.
#[test]
fn a_page_that_implies_no_grid_reports_an_empty_table_list() {
    for path in [
        conformance("synthetic/simple-text/document.pdf"),
        conformance("synthetic/two-lines/document.pdf"),
    ] {
        let a = extract_ok(path.clone());
        for page in &a.pages {
            assert!(
                page.tables.is_empty(),
                "{path:?}: a page of prose has no table — and certainly not a 1x1 one around the \
                 page"
            );
        }
    }
}

/// **The S2 golden.** The document S1 measured as un-findable, found.
///
/// `synthetic/table-regular-grid` is six `Tm`/`Tj` pairs and **zero path operators**: its grid is
/// text position alone. v1-S1 reported `tables: []` on it and said so in a limitation. The whole
/// of v1-S2 is that this document now yields its table, with the same guarantees the ruled rule
/// gives — `CellSlot`-complete, cross-checked, and carrying only text the document contains.
#[test]
fn the_alignment_rule_finds_the_grid_the_ruled_rule_could_not() {
    let a = extract_ok(conformance("synthetic/table-regular-grid/document.pdf"));

    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1, "exactly one table, not one per row");
    let t = tables[0];

    assert_eq!((t.rows, t.columns), (3, 2));
    assert_eq!(t.cells.len(), 6, "every face is a cell");
    assert_eq!(
        t.rule,
        engine_core::TABLE_DETECTION_UNRULED_V1,
        "and it says which rule found it, rather than leaving that to be inferred"
    );

    // Zero-based, span 1 = not merged, every cell naming its parent.
    for c in &t.cells {
        assert_eq!((c.position.rowspan, c.position.colspan), (1, 1));
        assert!(!c.position.is_merged());
        assert_eq!(c.position.table_id, t.id, "a cell is addressable alone");
    }
    assert!(t
        .cells
        .iter()
        .any(|c| c.position.row == 0 && c.position.column == 0));

    // The text, cell by cell. This is the acceptance criterion in full: not "a 3x2 grid" but
    // "a 3x2 grid with the right words in the right cells".
    let at = |row: u32, column: u32| {
        t.cells
            .iter()
            .find(|c| c.position.row == row && c.position.column == column)
            .unwrap_or_else(|| panic!("no cell at ({row}, {column})"))
            .text
            .as_str()
    };
    assert_eq!(
        [at(0, 0), at(0, 1), at(1, 0), at(1, 1), at(2, 0), at(2, 1)],
        ["Name", "Score", "Alpha", "10", "Beta", "12"]
    );

    // Both derivations of this grid agree.
    assert_eq!(
        t.check.outcome,
        engine_core::CheckStatus::Ok,
        "{:?}",
        t.check
    );
}

// -------------------------------------------------------------------------------------------
// 12. Which rule fired (v1-S2)
// -------------------------------------------------------------------------------------------

/// Every table names the rule that produced it, and a page with both kinds carries both.
///
/// `both-table-rules` paints a 2×2 grid with `re` and, lower down, lays a second 2×2 grid out by
/// text position alone. The profile can say which rules *ran*; only a per-table field can say
/// which one found any given table, and this is the fixture that makes the difference visible.
#[test]
fn a_page_with_both_kinds_of_grid_records_both_rules() {
    let a = extract_ok(engine_fx("both-table-rules"));
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 2, "one grid of each kind: {tables:?}");

    let ruled = tables
        .iter()
        .find(|t| t.rule == engine_core::TABLE_DETECTION_V1)
        .expect("the painted grid must be found by the ruled rule");
    let unruled = tables
        .iter()
        .find(|t| t.rule == engine_core::TABLE_DETECTION_UNRULED_V1)
        .expect("the aligned-text grid must be found by the alignment rule");

    assert_eq!((ruled.rows, ruled.columns), (2, 2));
    assert_eq!((unruled.rows, unruled.columns), (2, 2));

    // The text went where the document put it, and nowhere else. `DetectedTable` is
    // `pub(crate)` machinery rather than published API, so this reads the cells inline instead
    // of naming the type — the M7 freeze is not widened for a test's convenience.
    let mut ruled_text: Vec<&str> = ruled.cells.iter().map(|c| c.text.as_str()).collect();
    let mut unruled_text: Vec<&str> = unruled.cells.iter().map(|c| c.text.as_str()).collect();
    ruled_text.sort_unstable();
    unruled_text.sort_unstable();
    assert_eq!(ruled_text, vec!["R1", "R2", "R3", "R4"]);
    assert_eq!(unruled_text, vec!["Ua", "Ub", "Uc", "Ud"]);

    // Neither derivation claims the other's region.
    assert!(
        !(ruled.rect.x0 < unruled.rect.x1
            && unruled.rect.x0 < ruled.rect.x1
            && ruled.rect.y0 < unruled.rect.y1
            && unruled.rect.y0 < ruled.rect.y1),
        "two tables must not overlap: {:?} vs {:?}",
        ruled.rect,
        unruled.rect
    );

    for t in [ruled, unruled] {
        assert_eq!(
            t.check.outcome,
            engine_core::CheckStatus::Ok,
            "{:?}",
            t.check
        );
    }
}

/// Where both rules could describe one region, the **ruled** one wins and the other is dropped.
///
/// `ruled-wins-shared-region` paints a 2×2 grid whose four runs are also a flawless 2×2
/// alignment. Without arbitration that is two tables claiming the same cells; with it, one.
///
/// Ruled wins because a ruling line is evidence the author left and an alignment cluster is a
/// decision this engine made. The two grids are never averaged either — that would produce a
/// grid neither rule found, under a rule id that describes neither.
#[test]
fn where_both_rules_could_fire_the_ruled_one_wins() {
    let a = extract_ok(engine_fx("ruled-wins-shared-region"));
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();

    assert_eq!(
        tables.len(),
        1,
        "one region, one table — never one per rule: {tables:?}"
    );
    assert_eq!(
        tables[0].rule,
        engine_core::TABLE_DETECTION_V1,
        "the author drew this grid, so the author's derivation is the one kept"
    );
    assert_eq!((tables[0].rows, tables[0].columns), (2, 2));
    assert_eq!(tables[0].cells.len(), 4);
}

/// **The near miss is a non-event, and it is declared.**
///
/// `unruled-near-miss` is three rows and two columns of text — except that on the last row the
/// right-hand value sits five points off the column the other two share. Five points is past the
/// tolerance the rule folds together and under the gutter it requires between real columns, so
/// there is no table here.
///
/// The declaration is the other half. Without it, this page and a page of ordinary prose both
/// say `tables: []`, and only one of them means "a grid was implied here and judged incoherent".
#[test]
fn columns_that_almost_align_produce_no_table_and_say_why() {
    let a = extract_ok(engine_fx("unruled-near-miss"));

    for page in &a.pages {
        assert!(
            page.tables.is_empty(),
            "a near miss is not a table: {:?}",
            page.tables
        );
    }

    let refused = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED)
        .expect("a refused candidate must be declared, not silently absent");

    assert_eq!(refused.scope, engine_core::LimitationScope::Document);
    assert!(
        refused.detail.contains("page 1"),
        "the declaration must say WHERE: {}",
        refused.detail
    );

    // Typed vocabulary, never a score (`docs/01-CONTRACT.md` §9). "Nearly a table" is a
    // confidence field wearing a different hat.
    let lower = refused.detail.to_ascii_lowercase();
    for scored in ["confidence", "probability", "score of", "likelihood"] {
        assert!(
            !lower.contains(scored),
            "`{scored}` in a refusal detail: {}",
            refused.detail
        );
    }
}

/// A page that implies nothing grid-shaped declares **no** refusal.
///
/// The other half of the near-miss disclosure. If every document carried it, it would carry no
/// information — which is exactly what was wrong with the blanket `unruled-tables-not-detected`
/// this slice retired.
#[test]
fn a_page_with_no_candidate_declares_no_refusal() {
    let a = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    assert!(
        !a.assurance
            .limitations
            .iter()
            .any(|l| l.code == engine_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED),
        "nothing was refused here, so nothing may say it was"
    );
}

/// **The 1040 does not acquire a fabricated lattice from alignment.**
///
/// v1-S1 measured the rectangle version of this mistake: one lattice built from every rectangle
/// on this form gave a 662-cell table with a cell spanning 75 rows by 45 columns, and Ethos
/// rejected the artifact outright. Alignment is the same mistake with a different input — the
/// form's labels and values imply a lattice of tens of thousands of faces — and the coherence
/// precondition is what refuses it.
///
/// Measured: page 1 implies 23 276 faces from 1 146 runs, page 2 implies 10 848 from 830.
#[test]
fn the_tax_form_does_not_become_an_alignment_lattice() {
    let a = extract_ok(path_in("benchmark", "irs-form-1040-2025.pdf"));

    for page in &a.pages {
        for t in &page.tables {
            // If this form ever does yield a table, it must be a real one — never the 75×45 cell.
            assert!(
                t.rows <= 64 && t.columns <= 64,
                "page {}: a {}x{} lattice on a tax form is the v1-S1 fabrication returning",
                page.index,
                t.rows,
                t.columns
            );
            for c in &t.cells {
                assert!(
                    c.position.rowspan <= 8 && c.position.colspan <= 8,
                    "page {}: a cell spanning {}x{} is not a cell",
                    page.index,
                    c.position.rowspan,
                    c.position.colspan
                );
            }
        }
    }

    let total: usize = a.pages.iter().map(|p| p.tables.len()).sum();
    assert_eq!(
        total, 0,
        "this form implies no coherent grid and must yield no table; if a later rule change \
         makes it yield one, that is a deliberate decision needing its own evidence"
    );

    // And it says it looked and refused, rather than staying silent about it.
    assert!(
        a.assurance
            .limitations
            .iter()
            .any(|l| l.code == engine_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED),
        "the refusal is the informative part of finding nothing here"
    );
}

/// **Fabrication 0, over the unruled goldens too.**
///
/// The v1-S1 property, widened to every table this engine emits regardless of which rule found
/// it: a cell's text is exactly the concatenation of the runs assigned to it, in order, and never
/// a novel string.
#[test]
fn no_table_cell_carries_text_the_document_did_not_put_there() {
    for fixture in [
        "ruled-table-grid",
        "ruled-table-overlap",
        "both-table-rules",
        "ruled-wins-shared-region",
    ] {
        let a = extract_ok(engine_fx(fixture));
        assert_cells_are_concatenations(&a, fixture);
    }
    let a = extract_ok(conformance("synthetic/table-regular-grid/document.pdf"));
    assert_cells_are_concatenations(&a, "table-regular-grid");
}

fn assert_cells_are_concatenations(a: &ExtractArtifact, what: &str) {
    let mut seen_a_table = false;
    for page in &a.pages {
        for t in &page.tables {
            seen_a_table = true;
            for c in &t.cells {
                let expected: String = c
                    .run_indices
                    .iter()
                    .map(|i| page.runs[*i].text.as_str())
                    .collect();
                assert_eq!(
                    c.text, expected,
                    "{what}: cell ({}, {}) under `{}` carries text that is not its runs",
                    c.position.row, c.position.column, t.rule
                );
                if c.run_indices.is_empty() {
                    assert!(
                        c.text.is_empty(),
                        "{what}: a cell enclosing no run must be EMPTY, never borrowed from a \
                         neighbour"
                    );
                }
            }
        }
    }
    assert!(seen_a_table, "{what}: expected at least one table to check");
}
