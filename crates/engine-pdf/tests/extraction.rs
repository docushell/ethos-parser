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
    assert_eq!(a.reading_order_rule, engine_core::READING_ORDER_RULE_V1);
    assert_eq!(a.source.media_type, "application/pdf");
}

/// **The S5 gate.** `two-columns` reads column-major, and the rule id says which rule did it.
///
/// # This golden was reversed, deliberately, and this is the note saying so
///
/// Through v1-S4 this test asserted `["Right top", "Right bottom", "Left top", "Left bottom"]` —
/// the content-stream order — and called it *"visibly wrong reading order, and honest about it"*.
/// That was the right thing to assert while `single-column-v1` was the rule, because the
/// alternative on offer flipped on `min_lines < 15` and a one-line edit reordered a page.
///
/// v1-S5 replaced the honesty with a rule, so the golden reverses. The claim is no longer *"this
/// comes out wrong and we say so"* but *"this comes out right, and here is the rule that did
/// it"*. Nothing was quietly fixed: the profile hash moved, `reading_order_rule` is a different
/// string, and an artifact from either side of the change announces which one it is.
///
/// # Geometry decides, not the strings
///
/// The runs are named "Left" and "Right", and a test that trusted those names would pass on a
/// document whose labels lied. The origins are what is asserted against: the left column really
/// is at the lesser x, and the expected sequence is derived from that.
#[test]
fn two_columns_reads_column_major_under_the_new_rule() {
    let a = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    let r = runs(&a);

    // Geometry first. Two x positions, two y positions, four runs — a 2x2 arrangement whose
    // left column is the lesser x, measured rather than taken from the text.
    let mut xs: Vec<i64> = r.iter().map(|x| x.locator.origin_x).collect();
    xs.sort_unstable();
    xs.dedup();
    assert_eq!(xs.len(), 2, "two columns of origins");
    let (left_x, right_x) = (xs[0], xs[1]);
    // Comfortably past `gutter-columns-v1`'s twelve-point floor, asserted from outside the crate
    // where that constant is not visible. The number is here so the fixture cannot drift into
    // testing nothing — a two-column fixture whose columns crept together would still pass a
    // test that only checked the strings.
    assert!(
        right_x - left_x >= 1_200,
        "the fixture's columns are {left_x} and {right_x}, which is under the rule's floor — \
         this fixture would no longer exercise a column cut"
    );

    // Column-major: within the left band top to bottom, then the right band. Built from the
    // origins, so if the fixture's labels ever stopped matching its geometry this expectation
    // follows the geometry.
    let mut want: Vec<(i64, i64)> = r
        .iter()
        .map(|x| (x.locator.origin_x, x.locator.origin_y))
        .collect();
    want.sort_unstable();
    let got: Vec<(i64, i64)> = r
        .iter()
        .map(|x| (x.locator.origin_x, x.locator.origin_y))
        .collect();
    assert_eq!(got, want, "left column top-to-bottom, then right column");

    // And the strings, so a reader of this file can see what that means.
    let texts: Vec<&str> = r.iter().map(|x| x.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left top", "Left bottom", "Right top", "Right bottom"],
        "the content stream writes the right column FIRST; `gutter-columns-v1` reads the page \
         instead of the stream"
    );

    assert_eq!(a.reading_order_rule, engine_core::READING_ORDER_RULE_V1);
    assert_ne!(a.reading_order_rule, engine_core::READING_ORDER_RULE_V0);

    // Still not a table. S2's discriminator is asserted properly in its own test; this is the
    // cheap guard that nobody "fixed" two-columns by making it a 2x2 grid.
    assert!(a.pages.iter().all(|p| p.tables.is_empty()));
}

/// **The anti-cliff pair, over real PDF bytes** (v1-S5 decision 2).
///
/// pdf-inspector decides multi-column on `min_lines < 15`: fourteen lines on a page come out
/// row-interleaved and fifteen come out column-major, so a one-line edit reorders the whole
/// document (`docs/03-V0-SCOPE.md` §3.2). These two fixtures are that edit. They are the same
/// page but for one line in the left column, they sit on either side of that boundary, and they
/// must read the same way.
///
/// **A port of the line-count rule fails here and nowhere else.** Every other fixture in the
/// corpus is far from fourteen lines; this pair exists to be the place where a tally-based rule
/// gives itself away.
#[test]
fn one_added_line_does_not_reorder_the_page() {
    let read = |name: &str| -> Vec<String> {
        let a = extract_ok(engine_fx(name));
        assert_eq!(a.reading_order_rule, engine_core::READING_ORDER_RULE_V1);
        assert!(
            a.pages.iter().all(|p| p.tables.is_empty()),
            "{name} must not be read as a table, or the runs become one atom and this test \
             stops exercising the column cut"
        );
        runs(&a).iter().map(|r| r.text.clone()).collect()
    };

    // Fourteen lines: seven per column. The stream writes R1..R7 before L1..L7.
    assert_eq!(
        read("two-column-14-lines"),
        vec!["L1", "L2", "L3", "L4", "L5", "L6", "L7", "R1", "R2", "R3", "R4", "R5", "R6", "R7"],
    );

    // Fifteen lines: the same page with `L8` added. Same rule, same shape of answer. A rule that
    // flipped at fifteen would produce a row-interleaved order for exactly one of these two.
    assert_eq!(
        read("two-column-15-lines"),
        vec![
            "L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8", "R1", "R2", "R3", "R4", "R5", "R6",
            "R7"
        ],
    );
}

/// **S2 × S5.** Reordering a two-column page does not make it a table.
///
/// This is the interaction `docs/09-V1-MILESTONES.md` S5 decision 7 names. Before this slice,
/// `two-columns` was refused by the alignment rule because its runs arrived right-column-first,
/// which is not row-major. The reordering could have looked like a fix for that — and it is not
/// one. Column-major is `Left top, Left bottom, Right top, Right bottom`; row-major would be
/// `Left top, Right top, Left bottom, Right bottom`. They are different sequences, the emission
/// is still not row-major, and the precondition still refuses.
///
/// The test would fail if someone "fixed" two-columns by turning it into a 2x2 table.
#[test]
fn a_reordered_two_column_page_is_still_not_a_row_major_table() {
    let a = extract_ok(conformance("synthetic/two-columns/document.pdf"));

    assert!(
        a.pages.iter().all(|p| p.tables.is_empty()),
        "two-columns is geometrically a 2x2 and is still NOT a table: the alignment rule wants \
         row-major emission and this page has never had it"
    );

    // The order really is column-major and really is not row-major. Spelled out because the two
    // are easy to confuse and only one of them satisfies the detector's precondition.
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left top", "Left bottom", "Right top", "Right bottom"],
    );
    assert_ne!(
        texts,
        vec!["Left top", "Right top", "Left bottom", "Right bottom"],
        "that sequence is row-major, and if the runs ever arrived in it the alignment rule would \
         accept this page as a 2x2 table — which would be a fabricated grid, not a reading order"
    );

    // And the refusal is declared rather than silent, exactly as S2 left it.
    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    assert!(
        codes.contains(&engine_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED),
        "the alignment rule built a candidate here and refused it; that refusal is a disclosure \
         and must survive the reordering: {codes:?}"
    );
}

/// **Tables are atoms** (v1-S5 decision 6), asserted where it would actually break.
///
/// `table-regular-grid` has two columns of text 108 points apart — comfortably past the reading
/// -order rule's gutter floor. Loose on a page, those runs would be cut into two columns and read
/// down each one. Inside an accepted table they are one object: they keep the order they were
/// concatenated in, so every cell's text still matches the runs the cell names.
#[test]
fn a_detected_table_is_not_shredded_into_columns_by_the_reading_order_rule() {
    for (name, path) in [
        (
            "table-regular-grid",
            conformance("synthetic/table-regular-grid/document.pdf"),
        ),
        ("ruled-table-grid", engine_fx("ruled-table-grid")),
        ("both-table-rules", engine_fx("both-table-rules")),
    ] {
        let a = extract_ok(path);
        for page in &a.pages {
            for t in &page.tables {
                for c in &t.cells {
                    // The indices were remapped through the reordering; this is the check that
                    // they were remapped correctly. A cell pointing at the wrong runs would
                    // claim text it does not contain, and nothing else on the artifact would say
                    // so.
                    let from_runs: String = c
                        .run_indices
                        .iter()
                        .map(|i| page.runs[*i].text.as_str())
                        .collect();
                    assert_eq!(
                        from_runs, c.text,
                        "{name}: cell {:?} names runs whose text is not its text — the reading \
                         -order permutation did not reach `run_indices`",
                        c.position
                    );
                    // Fabrication-0, restated: every character of a cell came from a run.
                    assert!(
                        c.run_indices.iter().all(|i| *i < page.runs.len()),
                        "{name}: a cell points past the end of the run list"
                    );
                }
            }
        }
    }
}

/// **Ordinals are the reading order, not a second opinion about it** (v1-S5 decision 4).
///
/// The projection assigns `ordinal` from array position, so this asserts the thing that would
/// break if anyone reintroduced a parallel index: on the one fixture where array order and
/// stream order genuinely differ, the ordinals still run 1, 2, 3, 4 down the list, and the span
/// ids run with them.
#[test]
fn ordinals_and_ids_follow_the_reading_order_on_a_reordered_page() {
    let a = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    let r = runs(&a);
    let ids: Vec<&str> = r.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["s1", "s2", "s3", "s4"],
        "span ids are laid over the reading order, so `s1` is the first run a human should read \
         — not the first one the content stream drew"
    );

    let rep = engine_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let page: Vec<(u32, &str)> = rep
        .payload()
        .nodes
        .iter()
        .filter(|n| n.kind == engine_core::NodeKind::TextRun)
        .map(|n| (n.ordinal, n.text.as_str()))
        .collect();
    assert_eq!(
        page,
        vec![
            (1, "Left top"),
            (2, "Left bottom"),
            (3, "Right top"),
            (4, "Right bottom")
        ],
        "ordinal is monotone in the node list, and the node list is the reading order"
    );
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

// -------------------------------------------------------------------------------------------
// 13. The tagged-structure tree (v1-S3)
// -------------------------------------------------------------------------------------------

/// **The `structural_locators` proof.** Both halves, because one without the other proves nothing.
///
/// A capability that says *this profile looks* is proven by a document where looking finds
/// something **and** a document where it finds nothing. A test that only checked the first could
/// be satisfied by an engine that invents roles; a test that only checked the second could be
/// satisfied by one that never looks at all.
#[test]
fn the_structure_tree_supplies_role_paths_and_absence_stays_absent() {
    use engine_core::StructuralLocator;

    // Half one: a tagged document yields the roles ITS OWN TREE gives.
    let tagged = extract_ok(engine_fx("tagged-structure-roles"));
    let bound: Vec<&engine_core::PdfTaggedLocator> = runs(&tagged)
        .iter()
        .filter_map(|r| match &r.structural {
            Some(StructuralLocator::PdfTagged(t)) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(bound.len(), 2, "two runs are cited by the tree");
    for t in &bound {
        assert_eq!(
            t.role_path,
            vec!["Document".to_string(), "P".to_string()],
            "the path is the one the document wrote, root first"
        );
    }
    assert!(
        tagged.assurance.capabilities.structural_locators,
        "the capability is what this test proves"
    );

    // Half two: an untagged document gains nothing, with the machinery present and running.
    let untagged = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    for r in runs(&untagged) {
        assert!(
            r.structural.is_none(),
            "an untagged document must gain no role: {:?}",
            r.structural
        );
    }
    let codes: Vec<&str> = untagged
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    assert!(
        codes.contains(&engine_core::codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
        "and must say why it found none: {codes:?}"
    );
    assert!(
        !codes.contains(&engine_core::codes::STRUCTURAL_LOCATORS_NOT_CLAIMED),
        "the false-capability partner is retired now that the tree is read: {codes:?}"
    );
}

/// **Four states, four meanings.** A tagged document is not uniformly tagged.
///
/// `tagged-structure-roles` carries one run of each, and collapsing any two would lose something
/// real: "outside the tree", "not marked at all", and "marked as furniture" are three different
/// statements about one page.
#[test]
fn every_structural_locator_state_is_distinguishable() {
    use engine_core::StructuralLocator;

    let a = extract_ok(engine_fx("tagged-structure-roles"));
    let by_text = |t: &str| {
        runs(&a)
            .into_iter()
            .find(|r| r.text == t)
            .unwrap_or_else(|| panic!("no run reading {t:?}"))
            .structural
            .clone()
    };

    match by_text("First paragraph") {
        Some(StructuralLocator::PdfTagged(t)) => {
            assert_eq!(t.mcid, 0);
            assert_eq!(t.role_path, vec!["Document", "P"]);
            assert!(
                t.standard_role_path.is_none(),
                "nothing was remapped, so no remapped path is emitted"
            );
        }
        other => panic!("cited content must bind: {other:?}"),
    }

    // Marked with an id the tree never mentions. A SMALLER claim than a role path, and the
    // difference is preserved rather than smoothed over.
    assert_eq!(
        by_text("Marked but unclaimed"),
        Some(StructuralLocator::PdfMcid(5)),
        "an id no structure element claims stays a bare id"
    );

    // Page furniture. Present in the artifact, flagged — never deleted.
    match by_text("Running head") {
        Some(StructuralLocator::PdfArtifact(_)) => {}
        other => panic!("an /Artifact sequence must be flagged as one: {other:?}"),
    }

    // The page marked nothing here, so there is nothing to report.
    assert_eq!(by_text("Never marked"), None);

    // **The artifact run is still in the node list.** A reader that drops running heads has
    // silently edited the document (parity checklist O21/O22), and the edit is undetectable
    // downstream.
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
    assert!(
        texts.contains(&"Running head"),
        "artifact content is classified, never dropped: {texts:?}"
    );
    assert_eq!(texts.len(), 5, "every run survives: {texts:?}");
}

/// **A run outside marked content does not make the document untagged.**
///
/// Two absences that must never be conflated: this document HAS a tree, and one of its runs sits
/// outside it. Reporting `untagged-structure-tree-absent` here would be false.
#[test]
fn an_unmarked_run_in_a_tagged_document_is_not_an_untagged_document() {
    let a = extract_ok(engine_fx("tagged-structure-roles"));
    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();

    assert!(
        !codes.contains(&engine_core::codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
        "this document carries a tree; only its coverage is partial: {codes:?}"
    );
    assert!(
        codes.contains(&engine_core::codes::STRUCTURE_MCID_UNBOUND),
        "and the partial coverage is what gets declared: {codes:?}"
    );
}

/// **The join is exact.** An id the tree does not cite binds nothing, and binds nothing *nearby*.
#[test]
fn the_mcid_join_is_exact_equality_and_never_nearest_match() {
    use engine_core::StructuralLocator;

    let a = extract_ok(engine_fx("tagged-structure-roles"));
    // The tree cites 0 and 1. The content stream also marks 5. A nearest-match join would give
    // run 5 the role path of the closest cited id; an exact one gives it nothing.
    let unbound = runs(&a)
        .into_iter()
        .find(|r| r.mcid == Some(5))
        .expect("the fixture marks mcid 5");
    assert_eq!(
        unbound.structural,
        Some(StructuralLocator::PdfMcid(5)),
        "5 is not 0 and is not 1, so it binds to neither"
    );

    // And every bound run's locator names its own id, not a neighbour's.
    for r in runs(&a) {
        if let Some(StructuralLocator::PdfTagged(t)) = &r.structural {
            assert_eq!(
                Some(t.mcid),
                r.mcid,
                "a bound locator must carry the id that bound it"
            );
        }
    }
}

/// **`/RoleMap` is read as data; an unmapped custom type is never guessed at.**
#[test]
fn a_role_map_is_applied_and_an_unmapped_role_stays_itself() {
    use engine_core::StructuralLocator;

    let a = extract_ok(engine_fx("tagged-rolemap"));
    let tagged = |text: &str| match runs(&a)
        .into_iter()
        .find(|r| r.text == text)
        .unwrap_or_else(|| panic!("no run reading {text:?}"))
        .structural
        .clone()
    {
        Some(StructuralLocator::PdfTagged(t)) => t,
        other => panic!("{text:?} must be bound: {other:?}"),
    };

    // /Para -> /P, because the document's own /RoleMap says so.
    let mapped = tagged("Mapped to P");
    assert_eq!(
        mapped.role_path,
        vec!["Document", "Para"],
        "the raw path is never laundered: a consumer sees what the file says"
    );
    assert_eq!(
        mapped.standard_role_path.as_deref(),
        Some(["Document".to_string(), "P".to_string()].as_slice()),
        "and the mapped path rides alongside, present because a mapping applied"
    );

    // /Odd maps to nothing, so it stays /Odd. Deciding it "must mean" /P because it sits where a
    // paragraph would is precisely the inference this slice refuses.
    let unmapped = tagged("Not mapped");
    assert_eq!(unmapped.role_path, vec!["Document", "Odd"]);
    assert!(
        unmapped.standard_role_path.is_none(),
        "no mapping applied, so no mapped path is invented: {:?}",
        unmapped.standard_role_path
    );
}

/// **A `/K` cycle is refused by name**, not walked and not half-reported.
#[test]
fn a_cyclic_structure_tree_fails_closed() {
    let path = engine_fx("tagged-cycle");
    let profile = Profile::default();
    let doc = Document::open(&path, &profile).expect("the document itself is well formed");

    let e = engine_pdf::extract(&doc, &profile)
        .expect_err("a tree that does not terminate must not produce an artifact");

    assert_eq!(e.code(), "malformed", "got {e}");
    let msg = e.to_string();
    assert!(
        msg.contains("cycle") || msg.contains("cycles"),
        "the refusal must name what was wrong: {msg}"
    );
}

/// **Tagged and geometric agree**, and the agreement means something because the halves are
/// independent.
#[test]
fn a_tagged_table_that_matches_the_painted_grid_checks_ok() {
    use engine_core::TaggedGridStatus;

    let a = extract_ok(engine_fx("tagged-table-agrees"));
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1);
    let t = tables[0];

    assert_eq!((t.rows, t.columns), (2, 2));
    assert_eq!(t.rule, engine_core::TABLE_DETECTION_V1);

    let check = t
        .tagged_check
        .as_ref()
        .expect("the tree describes a table here, so the check runs");
    assert_eq!(check.check_id, engine_core::TAGGED_GRID_CHECK_V1);
    assert_ne!(
        check.check_id,
        engine_core::LOCATOR_CHECK_V1,
        "this is a second check, not a widening of the first"
    );
    assert_eq!(check.outcome, TaggedGridStatus::Ok, "{check:?}");

    // Both cross-checks pass, and they are asking different questions.
    assert_eq!(t.check.outcome, engine_core::CheckStatus::Ok);
}

/// **Tagged and geometric disagree**: named, counted, and nothing repaired.
#[test]
fn a_tagged_table_that_contradicts_the_painted_grid_reports_a_mismatch() {
    use engine_core::{TaggedGridFault, TaggedGridStatus};

    let a = extract_ok(engine_fx("tagged-table-disagrees"));
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1);
    let t = tables[0];

    // **Nothing was repaired.** The detector found a 2x2 and still reports a 2x2; the tree's
    // claim of a third row is recorded beside it rather than adopted.
    assert_eq!(
        (t.rows, t.columns),
        (2, 2),
        "the geometric grid is untouched by the disagreement"
    );
    assert_eq!(t.cells.len(), 4);

    let check = t.tagged_check.as_ref().expect("the check must run");
    match &check.outcome {
        TaggedGridStatus::Mismatch { faults } => {
            assert!(
                faults.iter().any(|f| matches!(
                    f,
                    TaggedGridFault::RowCountDiffers {
                        tagged: 3,
                        detected: 2
                    }
                )),
                "the row disagreement must be named: {faults:?}"
            );
            assert!(
                faults
                    .iter()
                    .any(|f| matches!(f, TaggedGridFault::SlotOnlyInTagged(_))),
                "and the slots only the tree claims: {faults:?}"
            );
        }
        other => panic!("expected a mismatch, got {other:?}"),
    }

    // The tree cited two content items the page never marked. Counted, never filled with a
    // fabricated run.
    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    assert!(
        codes.contains(&engine_core::codes::STRUCTURE_ITEM_WITHOUT_CONTENT),
        "{codes:?}"
    );
    assert_eq!(
        runs(&a).len(),
        4,
        "no run was invented to satisfy the tree's extra citations"
    );
}

/// **The tagged check is absent where the tree says nothing** — absent, not `Ok`.
#[test]
fn a_table_with_no_tagged_counterpart_carries_no_tagged_check() {
    let a = extract_ok(engine_fx("ruled-table-grid"));
    for page in &a.pages {
        for t in &page.tables {
            assert!(
                t.tagged_check.is_none(),
                "an untagged document's table has nothing to compare against, and `Ok` would \
                 claim an agreement that was never tested"
            );
        }
    }
}

/// **Earlier slices are untouched by reading the tree.**
///
/// S3 attaches addresses. It does not reorder nodes, and it does not let tags nudge a detector:
/// using the tree to "fix" an unruled near miss or to find a table on the 1040 is exactly the
/// scope creep this asserts against.
#[test]
fn reading_the_structure_tree_changes_no_earlier_slices_answer() {
    // S2's golden is still a 3x2 unruled table.
    let golden = extract_ok(conformance("synthetic/table-regular-grid/document.pdf"));
    let t: Vec<_> = golden.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].rows, t[0].columns), (3, 2));
    assert_eq!(t[0].rule, engine_core::TABLE_DETECTION_UNRULED_V1);

    // The near miss is still a near miss; tags did not rescue it (there are none).
    let near = extract_ok(engine_fx("unruled-near-miss"));
    assert!(near.pages.iter().all(|p| p.tables.is_empty()));

    // two-columns is still not a table, and its order is still the reading-order rule's answer
    // rather than the tag tree's.
    //
    // **Updated at v1-S5**, which is what this assertion was always waiting for: the expected
    // sequence used to be the content stream's, because no rule reordered it. It is now
    // column-major, and the point of the test is unchanged — S3 did not do that, the geometric
    // rule did. `/K` order is still not a sorter, and `structure.rs` still contains no sort.
    let two = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    assert_eq!(two.reading_order_rule, engine_core::READING_ORDER_RULE_V1);
    let texts: Vec<&str> = runs(&two).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left top", "Left bottom", "Right top", "Right bottom"],
        "the order came from the page's whitespace; this document is untagged, so the structure \
         tree could not have produced it even if it were consulted"
    );
    assert!(two.pages.iter().all(|p| p.tables.is_empty()));

    // And the 1040 still yields no table.
    let form = extract_ok(path_in("benchmark", "irs-form-1040-2025.pdf"));
    assert_eq!(form.pages.iter().map(|p| p.tables.len()).sum::<usize>(), 0);
}

// -------------------------------------------------------------------------------------------
// 14. Forms and annotations (v1-S4)
// -------------------------------------------------------------------------------------------

/// Every string a form field or annotation carries, on one document.
fn object_texts(a: &ExtractArtifact) -> Vec<String> {
    a.pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .map(|o| o.text.clone())
        .filter(|t| !t.is_empty())
        .collect()
}

/// **The `form_fields` proof, and the rule this whole slice exists to enforce.**
///
/// A field's `/V` lives in a dictionary. Nothing in the page's content stream draws it, so if it
/// ever turned up in a `text_run` that would mean this engine had copied it there — the LiteParse
/// defect (checklist L13), where a widget's value becomes indistinguishable from the page's own
/// words and a citation "grounded in the document" is really grounded in a form control.
///
/// The fixture's page draws the *label* and nothing else, so the value has no innocent route into
/// the runs. Both halves of the capability are proved here: found where there is a form, absent
/// where there is not, with the flag true either way.
#[test]
fn a_form_fields_value_is_a_node_and_never_a_text_run() {
    let a = extract_ok(engine_fx("form-field-value"));

    let fields: Vec<_> = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter(|o| matches!(o.attributes, engine_core::NodeAttributes::FormField(_)))
        .collect();
    assert_eq!(
        fields.len(),
        1,
        "one widget is one node, not a field plus a clone"
    );

    let engine_core::NodeAttributes::FormField(attrs) = &fields[0].attributes else {
        unreachable!("filtered above")
    };
    assert_eq!(attrs.field_name.as_deref(), Some("applicant_name"));
    assert_eq!(attrs.field_type.as_deref(), Some("Tx"));
    assert_eq!(
        attrs.value,
        engine_core::FieldValue::Text("Wendell Ashcroft-Byrne".into())
    );
    assert_eq!(fields[0].text, "Wendell Ashcroft-Byrne");
    // `/Ff 2` is bit 2, "required".
    assert_eq!(attrs.flags, vec!["required".to_string()]);

    // **The assertion the slice is for.** No run carries the value, and none contains it.
    for r in runs(&a) {
        assert_ne!(
            r.text, "Wendell Ashcroft-Byrne",
            "a field value is not page text"
        );
        assert!(
            !r.text.contains("Ashcroft"),
            "no fragment of a field value may reach the text layer: {:?}",
            r.text
        );
    }
    // The printed label IS page text and stays one — the rule is about where a value is read
    // from, not a claim that nothing near a widget can be a run.
    assert!(runs(&a).iter().any(|r| r.text == "Applicant name:"));

    // Half two of the capability: a document with no form yields none, flag still true.
    let none = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    assert!(none.assurance.capabilities.form_fields);
    assert!(
        none.pages.iter().all(|p| p.objects.is_empty()),
        "no form means no field nodes — looked and found none, not a missing capability"
    );
}

/// **The `annotations` proof.** A comment is not the page's words.
///
/// Same rule from the other side: `/Contents` is a reviewer's text laid *over* a document, and a
/// consumer that cannot tell it from the document's own words cannot safely cite either.
#[test]
fn an_annotations_contents_is_a_node_and_never_a_text_run() {
    let a = extract_ok(engine_fx("annotation-contents"));

    let annots: Vec<_> = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter(|o| matches!(o.attributes, engine_core::NodeAttributes::Annotation(_)))
        .collect();
    assert_eq!(annots.len(), 2);

    let texts = object_texts(&a);
    assert!(texts.contains(&"Check this figure against the appendix".to_string()));

    for r in runs(&a) {
        for t in &texts {
            assert_ne!(&r.text, t, "an annotation's text is never a run");
            assert!(
                !r.text.contains("appendix") && !r.text.contains("Withheld"),
                "no annotation text may reach the text layer: {:?}",
                r.text
            );
        }
    }

    // Subtype and author come through verbatim.
    let engine_core::NodeAttributes::Annotation(first) = &annots[0].attributes else {
        unreachable!()
    };
    assert_eq!(first.subtype, "Text");
    assert_eq!(first.title.as_deref(), Some("Reviewer"));
    assert_eq!(first.name.as_deref(), Some("note-1"));

    // Half two: a document with no annotations, capability still true.
    let none = extract_ok(conformance("synthetic/simple-text/document.pdf"));
    assert!(none.assurance.capabilities.annotations);
    assert!(none.pages.iter().all(|p| p.objects.is_empty()));
}

/// **A hidden annotation is flagged, never deleted.**
///
/// `/F` bit 2 asks a viewer not to draw it. That is a rendering instruction, not permission to
/// remove content from the record — the parity checklist's O21 is exactly this: report, do not
/// drop. A reader that honoured the flag by deleting would produce an artifact that is silently
/// missing text, with nothing to say so.
#[test]
fn a_hidden_annotation_is_still_a_node_carrying_its_flag() {
    let a = extract_ok(engine_fx("annotation-contents"));

    let hidden: Vec<_> = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter(|o| match &o.attributes {
            engine_core::NodeAttributes::Annotation(x) => x.flags.contains(&"hidden".to_string()),
            _ => false,
        })
        .collect();

    assert_eq!(
        hidden.len(),
        1,
        "the hidden annotation must survive the walk"
    );
    assert_eq!(hidden[0].text, "Withheld pending legal review");
}

/// **An unresolvable `/Parent` is declared, not repaired.**
///
/// LiteParse repairs orphaned widgets in memory and always flattens. Here the widget is emitted
/// with whatever it declares about itself, the break is on the wire, and the source bytes are
/// untouched. The visible consequence — a field name shorter than the form intends — is the
/// honest one.
#[test]
fn an_orphan_widget_is_declared_rather_than_repaired() {
    let path = engine_fx("form-orphan-widget");
    let before = std::fs::read(&path).expect("fixture readable");

    let a = extract_ok(path.clone());

    assert!(
        codes_of(&a).contains(&engine_core::codes::FORM_FIELD_PARENT_UNRESOLVED),
        "the broken link must be declared: {:?}",
        codes_of(&a)
    );

    // The widget is still a node, with what it declares about itself.
    let fields: Vec<_> = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter(|o| matches!(o.attributes, engine_core::NodeAttributes::FormField(_)))
        .collect();
    assert_eq!(fields.len(), 1, "an orphan is emitted, never skipped");
    assert_eq!(fields[0].text, "Orphaned value");

    // And the file on disk is byte-identical: no in-memory repair reached it.
    assert_eq!(
        std::fs::read(&path).expect("fixture still readable"),
        before,
        "the source bytes must be untouched"
    );
}

/// **XFA is detected and declared, never parsed.**
///
/// The static AcroForm sibling still reads — which is why this is a limitation rather than a
/// refusal — but a sparse field set on a dynamic form must not read as "this form is blank".
#[test]
fn an_xfa_packet_is_declared_and_its_static_sibling_still_reads() {
    let a = extract_ok(engine_fx("form-xfa-stub"));

    assert!(
        codes_of(&a).contains(&engine_core::codes::XFA_FORMS_NOT_EXTRACTED),
        "{:?}",
        codes_of(&a)
    );
    assert!(
        object_texts(&a).contains(&"Static AcroForm value".to_string()),
        "a static field beside an XFA packet still reads"
    );
    // Nothing tried to read the XML.
    for r in runs(&a) {
        assert!(
            !r.text.contains("xdp"),
            "the packet is not parsed: {:?}",
            r.text
        );
    }
}

fn codes_of(a: &ExtractArtifact) -> Vec<&str> {
    a.assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect()
}

/// **A form's widgets do not become an alignment lattice.**
///
/// v1-S2's detector clusters **run origins**, and a form field is not a run — so 199 widgets on
/// `irs-form-1040-2025` never enter the clustering at all. That is structural exclusion rather
/// than a threshold that happens to reject them, which is the stronger guarantee, and this pins
/// both the mechanism and the outcome.
#[test]
fn form_fields_never_feed_the_table_detectors() {
    let a = extract_ok(path_in("benchmark", "irs-form-1040-2025.pdf"));

    let fields: usize = a
        .pages
        .iter()
        .map(|p| {
            p.objects
                .iter()
                .filter(|o| matches!(o.attributes, engine_core::NodeAttributes::FormField(_)))
                .count()
        })
        .sum();
    assert!(
        fields > 100,
        "this form really does carry widgets: {fields}"
    );

    assert_eq!(
        a.pages.iter().map(|p| p.tables.len()).sum::<usize>(),
        0,
        "a form's widgets are not a table, and reading them must not make one appear"
    );

    // A blank text field reports `Absent`, never an empty string: "left unfilled" and "filled in
    // with nothing" are different facts about a form.
    let blank = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter(|o| match &o.attributes {
            engine_core::NodeAttributes::FormField(f) => f.value == engine_core::FieldValue::Absent,
            _ => false,
        })
        .count();
    assert!(blank > 0, "this form is blank, and says so per field");

    // Its fully-qualified names really are joined from the `/T` path.
    let named = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter_map(|o| match &o.attributes {
            engine_core::NodeAttributes::FormField(f) => f.field_name.clone(),
            _ => None,
        })
        .find(|n| n.contains('.'))
        .expect("a hierarchical field name");
    assert!(named.contains('.'), "{named}");
}

/// **Earlier slices are untouched by reading forms.**
#[test]
fn reading_forms_changes_no_earlier_slices_answer() {
    // S2's golden.
    let golden = extract_ok(conformance("synthetic/table-regular-grid/document.pdf"));
    let t: Vec<_> = golden.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(t.len(), 1);
    assert_eq!((t[0].rows, t[0].columns), (3, 2));
    assert_eq!(t[0].rule, engine_core::TABLE_DETECTION_UNRULED_V1);

    // S1's golden.
    let ruled = extract_ok(engine_fx("ruled-table-grid"));
    let rt: Vec<_> = ruled.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(rt.len(), 1);
    assert_eq!(rt[0].rule, engine_core::TABLE_DETECTION_V1);

    // S3's four locator states, still four.
    let tagged = extract_ok(engine_fx("tagged-structure-roles"));
    assert_eq!(runs(&tagged).len(), 5);
    assert!(runs(&tagged).iter().any(|r| matches!(
        r.structural,
        Some(engine_core::StructuralLocator::PdfArtifact(_))
    )));

    // two-columns: column-major since v1-S5, still no table. Widgets are not runs, so nothing
    // the form walk found could have joined the text order — the four strings here are the four
    // the content stream drew.
    let two = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    let texts: Vec<&str> = runs(&two).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left top", "Left bottom", "Right top", "Right bottom"]
    );
    assert_eq!(two.reading_order_rule, engine_core::READING_ORDER_RULE_V1);
    assert!(two.pages.iter().all(|p| p.tables.is_empty()));
}

// -------------------------------------------------------------------------------------------
// 15. Images, findings and the overlay (v1-S6)
// -------------------------------------------------------------------------------------------

/// **The `images` proof, both halves** (v1-S6).
///
/// A page that PAINTS an image yields a node carrying where it was drawn and which bytes it is.
/// A page that merely declares the same image in its resources and never draws it yields none —
/// because `Do` is what makes a placement, and a resource nobody painted is a resource.
///
/// The two fixtures are the same image object in the same dictionary, so the only difference
/// between them is the `Do`. That is what makes the pair a proof rather than two observations.
#[test]
fn an_image_is_a_node_with_a_placement_and_a_digest() {
    let a = extract_ok(engine_fx("image-xobject-drawn"));
    assert!(a.assurance.capabilities.images);

    let images: Vec<_> = a.pages.iter().flat_map(|p| p.images.iter()).collect();
    assert_eq!(images.len(), 1, "one `Do`, one node");
    let img = images[0];

    // The digest covers the stream AS STORED. Recomputed here from the file's own bytes rather
    // than trusted: an image node's whole claim is "these bytes", and a test that only checked
    // the field was well-formed would pass on a digest of the wrong thing.
    let raw = std::fs::read(engine_fx("image-xobject-drawn")).expect("fixture readable");
    let start = find(&raw, b"stream\n", find(&raw, b"/Subtype /Image", 0)) + b"stream\n".len();
    let end = find(&raw, b"\nendstream", start);
    let expected = engine_core::Sha256Hex::of_bytes(&raw[start..end]);
    assert_eq!(
        img.attributes.stream_sha256, expected,
        "the digest must be over the encoded stream the file actually holds"
    );
    assert_eq!(img.attributes.stream_bytes, (end - start) as u64);

    // `/FlateDecode` samples are not a file. Claiming `image/png` because the bytes are deflated
    // would send a consumer to open something as a format it is not.
    assert_eq!(img.attributes.filters, vec!["FlateDecode".to_string()]);
    assert_eq!(
        img.attributes.media_type,
        engine_core::ImageMediaType::PdfEncodedSamples
    );

    // **The placement is the matrix, not the pixel count.** The fixture's image is 2x2 samples
    // and its `cm` paints it into 120x60 points at (40, 60). Those numbers must not be confusable.
    assert_eq!(img.attributes.pixel_width, Some(2));
    assert_eq!(img.attributes.pixel_height, Some(2));
    let rect = img
        .locator
        .rect
        .painted()
        .expect("an axis-aligned `cm` produces a rectangle");
    assert_eq!(
        (rect.x0(), rect.y0(), rect.x1(), rect.y1()),
        (4_000, 8_000, 16_000, 14_000),
        "120x60 points at (40,60) in a 200-point-tall page, in the declared top-left system"
    );
    assert!(
        rect.x1() - rect.x0() != i64::from(img.attributes.pixel_width.unwrap()),
        "the painted width and the sample count must not be the same number, or this fixture \
         cannot tell the two apart"
    );

    // Not a run, and not text. An image node carries no text at all — a description would be a
    // model's opinion and reading pixels would be OCR, and neither is evidence (checklist O20).
    let rep = engine_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let image_nodes: Vec<_> = rep
        .payload()
        .nodes
        .iter()
        .filter(|n| n.kind == engine_core::NodeKind::Image)
        .collect();
    assert_eq!(image_nodes.len(), 1);
    assert!(
        image_nodes[0].text.is_empty(),
        "an image node carries no text"
    );
    assert!(matches!(
        image_nodes[0].native_locator,
        engine_core::NativeLocator::PdfImage(_)
    ));
    assert!(
        !runs(&a).iter().any(|r| r.text.contains("Im1")),
        "an image's resource name is not text and must never appear as one"
    );

    // Half two: the SAME image, declared and never drawn.
    let none = extract_ok(engine_fx("image-declared-not-drawn"));
    assert!(
        none.assurance.capabilities.images,
        "the capability is a property of the profile, not of the document"
    );
    assert_eq!(
        none.pages.iter().map(|p| p.images.len()).sum::<usize>(),
        0,
        "a resource nobody painted is a resource; `Do` is what makes a placement"
    );
}

/// Byte offset of `needle` at or after `from`.
fn find(haystack: &[u8], needle: &[u8], from: usize) -> usize {
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|i| i + from)
        .unwrap_or_else(|| {
            panic!(
                "fixture shape changed: {:?} not found",
                String::from_utf8_lossy(needle)
            )
        })
}

/// **Checklist O21, and the defect it names.**
///
/// Invisible text is REPORTED and KEPT. OpenDataLoader deletes low-contrast text before returning
/// a page, so the page it returns looks clean and its caller cannot tell a scrubbed document from
/// an innocent one. This engine flags and keeps.
///
/// Note what this test would have looked like before v1-S6: the hidden string was **already**
/// present in the artifact — `Tr` was tracked in the text state from v0 and read by nothing — so
/// nothing had to be un-deleted. What was missing is that anyone could tell it apart from prose.
#[test]
fn invisible_text_is_flagged_and_never_removed() {
    let a = extract_ok(engine_fx("invisible-render-mode"));
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Visible sentence", "Hidden instruction", "Visible again"],
        "all three runs are in the artifact, in reading order — the hidden one is not filtered, \
         not moved, and not marked up inside its own text"
    );

    let flagged: Vec<&str> = runs(&a)
        .iter()
        .filter(|r| {
            r.findings
                .contains(&engine_core::TextFinding::InvisibleRenderMode)
        })
        .map(|r| r.text.as_str())
        .collect();
    assert_eq!(
        flagged,
        vec!["Hidden instruction"],
        "exactly the run drawn under `3 Tr`, and not the ones drawn around it"
    );

    // Counted on the wire, so a consumer reading the assurance block learns of it without
    // diffing node lists.
    let l = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::codes::INVISIBLE_RENDER_MODE_TEXT)
        .expect("the observation is declared as well as flagged");
    assert_eq!(l.scope, engine_core::LimitationScope::Document);
    assert!(l.detail.contains("NONE was removed"));

    // The flag survives the projection: a finding that reached the extract and not the record
    // would be invisible to every consumer.
    let rep = engine_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let hidden = rep
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Hidden instruction")
        .expect("the node is in the record");
    match &hidden.attributes {
        engine_core::NodeAttributes::TextRun(t) => assert_eq!(
            t.findings,
            vec![engine_core::TextFinding::InvisibleRenderMode]
        ),
        other => panic!("expected a text run, got {other:?}"),
    }
}

/// **Off-page text, and the coordinate repair it depends on** (v1-S6).
///
/// The fixture's `/MediaBox` is `[0 20 300 220]` — an origin that is not `(0, 0)`, which not one
/// document in either corpus has — and its `/CropBox` is smaller still. Both facts matter:
///
/// * Through v1-S5 the page transform discarded the box origin, so every coordinate on such a
///   page was shifted by 20 points. The repair is asserted here by the y values themselves.
/// * A run below the crop box is inside the media box. Measuring against `/MediaBox` alone would
///   call it on-page, which is why the visible box is the one the rule uses.
#[test]
fn off_page_text_is_flagged_against_the_visible_box() {
    let a = extract_ok(engine_fx("off-page-and-offset-box"));
    let r = runs(&a);
    assert_eq!(r.len(), 2, "both runs are in the artifact");

    // The origin translation, asserted as a number. User-space y=120 in a box topping out at 220
    // is 100 points down; y=30 is 190 down. Under the pre-repair transform — which used only the
    // box's HEIGHT, 200 — they would have been 80 and 170.
    assert_eq!(r[0].locator.origin_y, 10_000, "220 - 120 = 100pt");
    assert_eq!(r[1].locator.origin_y, 19_000, "220 - 30 = 190pt");

    let flagged: Vec<&str> = r
        .iter()
        .filter(|x| x.findings.contains(&engine_core::TextFinding::OffPage))
        .map(|x| x.text.as_str())
        .collect();
    assert_eq!(
        flagged,
        vec!["Below the crop box"],
        "the run below `/CropBox` is off the visible page; the one inside it is not"
    );

    let l = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == engine_core::codes::OFF_PAGE_TEXT)
        .expect("counted as well as flagged");
    assert_eq!(l.scope, engine_core::LimitationScope::Document);
    assert!(l.detail.contains("none was removed"));
}

/// **The overlay is deterministic, and it does not touch what it draws on** (v1-S6).
///
/// Byte identity holds per fresh build, which is the whole of the trap: lopdf's writer mutates the
/// document it saves, so a cached `Document` saved twice produces two different files. This test
/// builds twice from scratch, which is what the shipped code does.
#[test]
fn the_overlay_is_byte_identical_and_leaves_the_source_alone() {
    let path = engine_fx("image-xobject-drawn");
    let before = std::fs::read(&path).expect("fixture readable");

    let build = || {
        let profile = Profile::default();
        let doc = Document::open(&path, &profile).expect("opens");
        let extract = engine_pdf::extract(&doc, &profile).expect("extracts");
        engine_pdf::build_overlay(&doc, &extract, &profile).expect("overlays")
    };
    let first = build();
    let second = build();
    assert_eq!(first, second, "two independent builds are byte-identical");
    assert!(first.starts_with(b"%PDF-"), "the overlay is a PDF");

    let after = std::fs::read(&path).expect("fixture still readable");
    assert_eq!(
        before, after,
        "the source document's bytes are untouched — this annotates, it does not edit"
    );

    // O10's exit criterion: the overlay distinguishes present geometry from typed absence. An
    // overlay that drew only the boxes it had would make a partly-read page look fully read.
    let text = String::from_utf8_lossy(&first);
    assert!(
        text.contains("ethos-engine: image "),
        "the painted image is marked"
    );
    assert!(
        text.contains("NO rectangle this overlay can draw"),
        "and the count of what could NOT be drawn is on the page too"
    );
    assert!(
        text.contains("EthosEngineOverlay"),
        "the artifact says what it is and what it was drawn from"
    );
}

/// **Earlier slices are untouched by images and findings** (v1-S6).
#[test]
fn observing_images_changes_no_earlier_slices_answer() {
    // S5: two-columns still column-major, still not a table.
    let two = extract_ok(conformance("synthetic/two-columns/document.pdf"));
    let texts: Vec<&str> = runs(&two).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Left top", "Left bottom", "Right top", "Right bottom"]
    );
    assert_eq!(two.reading_order_rule, engine_core::READING_ORDER_RULE_V1);
    assert!(two.pages.iter().all(|p| p.tables.is_empty()));
    assert!(
        two.pages.iter().all(|p| p.images.is_empty()),
        "no `Do` on this page, so no image node"
    );
    assert!(
        runs(&two).iter().all(|r| r.findings.is_empty()),
        "ordinary visible text on an ordinary page carries no findings — a rule that flagged \
         everything would be as useless as one that flagged nothing"
    );

    // S2's and S1's goldens.
    let golden = extract_ok(conformance("synthetic/table-regular-grid/document.pdf"));
    let t: Vec<_> = golden.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!((t.len(), t[0].rows, t[0].columns), (1, 3, 2));
    let ruled = extract_ok(engine_fx("ruled-table-grid"));
    assert_eq!(ruled.pages.iter().flat_map(|p| p.tables.iter()).count(), 1);

    // S4: the 1040 still yields no table, and its widgets are still not runs.
    let form = extract_ok(path_in("benchmark", "irs-form-1040-2025.pdf"));
    assert_eq!(form.pages.iter().map(|p| p.tables.len()).sum::<usize>(), 0);
    let widget_texts: Vec<String> = form
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .map(|o| o.text.clone())
        .filter(|t| !t.is_empty())
        .collect();
    let run_texts: Vec<&str> = runs(&form).iter().map(|r| r.text.as_str()).collect();
    for w in &widget_texts {
        assert!(
            !run_texts.contains(&w.as_str()),
            "a widget's value must never appear as a text run: {w:?}"
        );
    }
}

/// `failure/image-only-or-blank-page` has **no image in it**, and the engine must not claim one.
///
/// The fixture's name is a lie about its content: all 431 bytes of it are an empty content stream
/// and an empty `/Resources`. It is the blank half of "image-only or blank page". Pinned because
/// v1-S6 is exactly the slice where somebody reads the name, expects an image, and "fixes" the
/// classifier until it reports one — which would be a fabricated finding on a document that
/// contains nothing.
#[test]
fn the_image_only_fixture_is_blank_and_is_reported_as_blank() {
    let a = extract_ok(conformance("failure/image-only-or-blank-page/document.pdf"));
    assert!(runs(&a).is_empty(), "a blank page has no runs");
    assert!(
        a.pages.iter().all(|p| p.images.is_empty()),
        "and no images, because there is no image in the file"
    );
    assert!(
        a.pages.iter().all(|p| p.tables.is_empty()),
        "and no fabricated table"
    );
}

/// **A cropping page still parses, and its coordinates and its dimensions share one frame.**
///
/// Found by probing v1-S6's own change rather than by a test failing: page width and height were
/// briefly taken from the `/CropBox` while every coordinate stayed in the `/MediaBox` frame. Two
/// frames on one page, and `DocumentRepresentation::seal` refuses an artifact whose measured box
/// falls outside its declared page — so a document that crops stopped producing an artifact at
/// all, exiting 2 with *"the measurement or the coordinate transform is wrong"*. It was right.
///
/// No document in either corpus crops (all three benchmark PDFs declare a `/CropBox` equal to
/// their `/MediaBox`), and every other engine fixture supplies no ink metrics, so no measured box
/// existed anywhere that could fall outside a page. This fixture is both at once.
#[test]
fn a_page_that_crops_still_parses_and_keeps_one_coordinate_frame() {
    let a = extract_ok(engine_fx("crop-box-smaller-than-media"));
    let page = &a.pages[0];

    // The MEDIA box, because that is the frame `to_top_left` maps into.
    assert_eq!(
        (page.width, page.height),
        (30_000, 20_000),
        "page dimensions are the media box's, not the crop box's 200x100"
    );

    let r = runs(&a);
    assert_eq!(r.len(), 1);
    let ink = r[0]
        .geometry
        .measured()
        .expect("this fixture's font carries real ascent/descent, which is why it is this fixture");
    assert!(
        ink.x1() <= page.width && ink.y1() <= page.height,
        "the measured box [{}, {}, {}, {}] must fit the declared page [0, 0, {}, {}] — when it \
         did not, `seal` refused the whole document",
        ink.x0(),
        ink.y0(),
        ink.x1(),
        ink.y1(),
        page.width,
        page.height
    );

    // The crop box is still read, and is still what off-page is measured against. This run is
    // outside it, so it carries the finding — and it is still here, with its box, in the artifact.
    assert!(
        r[0].findings.contains(&engine_core::TextFinding::OffPage),
        "text in the cropped-away margin is outside the VISIBLE box and says so"
    );
    assert_eq!(r[0].text, "Near the top");

    // The projection seals, which is the assertion that actually failed before the fix.
    engine_pdf::to_representation(&a, &Profile::default())
        .expect("a cropping document produces a representation");
}

// -------------------------------------------------------------------------------------------
// 16. Code width comes from the font's declared kind (v1-S6.1)
// -------------------------------------------------------------------------------------------

/// **A simple font's codes are one byte, whatever its `/ToUnicode` codespace declares.**
///
/// The fixture is a TrueType — simple, so PDF 32000-1 §9.6 gives it single-byte codes — carrying a
/// `/ToUnicode` whose codespace says two. Real producers emit that combination constantly; **zero
/// fixtures had it before v1-S6.1**, which is why the defect survived six slices.
///
/// Split by the `/ToUnicode` codespace instead of by the font's declared kind, `"Hi there"` fuses
/// into `0x4869`, `0x2074`, … , none of which is in the map, and the entire run is dropped while
/// the artifact declares the *document's* encoding damaged.
#[test]
fn a_simple_fonts_codes_are_one_byte_whatever_its_tounicode_declares() {
    let a = extract_ok(engine_fx("simple-font-two-byte-tounicode"));
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Hi there"],
        "the run decodes; splitting by the CMap's codespace would drop it entirely"
    );

    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    assert!(
        !codes.contains(&engine_pdf::limitations::BROKEN_FONT_ENCODING),
        "nothing about this font is broken, and the artifact must not say otherwise: {codes:?}"
    );
    assert!(
        !codes.contains(&engine_core::codes::COMPOSITE_FONT_CODES_FROM_TOUNICODE),
        "a TrueType is not a composite font"
    );
}

/// **The repair, measured on the document that showed it.**
///
/// `cfpb-home-loan-toolkit` lost 8 417 runs to `broken-font-encoding` under a declaration that
/// blamed its fonts. Its fonts were conformant. This asserts the loss is gone and, more
/// importantly, that the false declaration is gone with it — a misattributing declaration is worse
/// than none, because a reader acts on it.
#[test]
fn a_real_document_stops_losing_text_and_stops_being_blamed_for_it() {
    let a = extract_ok(path_in("benchmark", "cfpb-home-loan-toolkit.pdf"));

    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();
    assert!(
        !codes.contains(&engine_pdf::limitations::BROKEN_FONT_ENCODING),
        "zero runs are dropped now; 8417 were, under a false statement about this document"
    );

    // The composite half is still an interim, and says so on this document rather than being
    // discovered the way the simple half was.
    assert!(
        codes.contains(&engine_core::codes::COMPOSITE_FONT_CODES_FROM_TOUNICODE),
        "this document has Type0 fonts, whose width still comes from the wrong authority: {codes:?}"
    );

    // Prose, not fragments. Before the repair this page read
    // "Choosing the best mortgage for youTTY You’rtartinoooortgagwant to confirm…".
    let page5: String = runs(&a)
        .iter()
        .filter(|r| r.locator.page == 5)
        .map(|r| r.text.as_str())
        .collect();
    assert!(
        page5.contains("You’re starting to look for a mortgage"),
        "the page must read as English: {}",
        &page5[..page5.len().min(200)]
    );
}

/// **The conformance corpus is untouched by the repair**, and that is the proof it was targeted.
///
/// Every fixture there is a Type1 with no `/ToUnicode`, so all nine took the correct path already.
/// A repair that moved their text would be fixing something else — or breaking it.
#[test]
fn the_conformance_corpus_decodes_exactly_as_it_did_before_the_repair() {
    for (name, expected) in [
        ("synthetic/simple-text/document.pdf", vec!["Hello Ethos"]),
        (
            "synthetic/two-lines/document.pdf",
            vec!["First line", "Second line"],
        ),
        (
            "synthetic/two-columns/document.pdf",
            vec!["Left top", "Left bottom", "Right top", "Right bottom"],
        ),
        (
            "synthetic/hyphenated-line-break/document.pdf",
            vec!["hyphen-", "ated"],
        ),
        (
            "synthetic/ligature-fi-embedded-font/document.pdf",
            vec!["office file"],
        ),
    ] {
        let a = extract_ok(conformance(name));
        let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, expected, "{name} decodes as it always has");
    }
}
