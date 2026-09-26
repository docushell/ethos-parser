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

//! M3 acceptance (`docs/history/05-MILESTONES.md`).
//!
//! Fixtures resolve through `fixtures/manifest.json`'s three roots exactly as the oracle harness
//! resolves them. **A missing corpus is a failure, never a skip.**

use std::path::PathBuf;

use ethos_parser_core::{DerivationClass, GeometryAbsence, GeometryPresence, Profile};
use ethos_parser_pdf::{Document, ExtractArtifact, TextRun};

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
    ethos_parser_pdf::extract(&doc, &profile).expect("document extracts")
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
/// `ethos_parser_pdf::ops` and `ethos_parser_pdf::content` to drive the interpreter directly, which made two
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
            .and_then(|d| ethos_parser_pdf::extract(&d, &profile))
            .is_ok(),
        "the control must extract, or the mutant proves nothing"
    );

    let doc = Document::open_bytes(&mutant, &profile)
        .expect("the document is still structurally valid — only the operator changed");
    let e = ethos_parser_pdf::extract(&doc, &profile)
        .expect_err("an unknown operator must stop the parse");

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
    // Was `synthetic/simple-text` — standard-14 Helvetica with no /FontDescriptor — until
    // decision #22 vendored the Core-14 AFMs and made that document's metrics readable. It is now
    // the fixture nothing can answer for: /ArialMT, no /Widths, no descriptor.
    let a = extract_ok(engine_fx("absent-font-widths"));
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

/// A real document that carries its type size in the text matrix gets real boxes.
///
/// **The regression this exists for.** `ink_box` used to scale ascent and descent by the raw
/// `Tf` operand. `measured_metrics_produce_a_box_that_is_not_the_font_size` above covers only the
/// case where the operand *is* the size, so it passed throughout — and on
/// `nist-sp-800-207`, which sets `Tf /F 1` and carries the size in the text matrix (the population
/// `docs/19-BLOCK-SUBDIVISION-SCOPE.md` §2 describes), **all 82 909 measured boxes came out between
/// 110 and 123 centipoints** — every one of them a hairline about 1.1pt tall, on a document whose
/// body text is 13pt.
///
/// Nothing caught it: the geometry section is not covered by `representation_c14n_sha256`, so the
/// artifact hash was byte-identical before and after the fix.
///
/// The assertion is deliberately loose. It is not pinning a layout; it is asserting that a box on a
/// text-bearing page is a plausible *glyph* height and not a rounding artefact.
#[test]
fn a_matrix_carried_type_size_produces_a_real_box() {
    let path = repo_root().join("fixtures/gate/nist-sp-800-207.pdf");
    if !path.exists() {
        // The gate fixtures are large and not every checkout carries them. Absent is not a pass:
        // say so loudly rather than reporting green on a test that did not run.
        eprintln!("SKIP: {} absent", path.display());
        return;
    }
    let a = extract_ok(path);

    // The height ACROSS each run's own baseline. Since docs/22 §9 items 1 and 2 this document's
    // vertical margin note has boxes too — 2 800 runs, 280 of them a glyph or two long — and for
    // a run drawn up the page `y1 - y0` is its length, not its envelope. A vertical box is the one
    // whose origin sits on a horizontal edge between its x edges.
    let mut heights: Vec<i64> = runs(&a)
        .iter()
        .filter_map(|r| r.geometry.measured().map(|rect| (r, rect)))
        .map(|(r, rect)| {
            let (ox, oy) = (r.locator.origin_x, r.locator.origin_y);
            let vertical = rect.x0() < ox && ox < rect.x1() && (oy == rect.y0() || oy == rect.y1());
            if vertical {
                rect.x1() - rect.x0()
            } else {
                rect.y1() - rect.y0()
            }
        })
        .collect();
    assert!(
        heights.len() > 1_000,
        "expected thousands of measured boxes on this document, got {}",
        heights.len()
    );

    heights.sort_unstable();
    let median = heights[heights.len() / 2];

    // 200 centipoints is 2pt. No glyph on a readable page is 2pt tall, so anything at or below
    // this is the scaling defect and not a small font.
    let hairlines = heights.iter().filter(|&&h| h <= 200).count();
    assert_eq!(
        hairlines,
        0,
        "{hairlines} of {} boxes are 2pt or shorter — ink_box is scaling by the Tf operand again, \
         not by the rendered em (median height {median}cp)",
        heights.len()
    );
    assert!(
        median > 400,
        "median box height {median}cp is under 4pt; the vertical scale is wrong"
    );
}

/// One page drawing `(abcde)` at (50, 50), 10 pt, in a Type 3 font whose five codes all name one
/// CharProc, with the given `/FontMatrix`, `/Widths` entry, descriptor ascent/descent and glyph.
fn type3_page(
    font_matrix: &str,
    widths: &str,
    ascent: i64,
    descent: i64,
    charproc: &str,
) -> Vec<u8> {
    let stream = |data: &str| format!("<< /Length {} >>\nstream\n{data}\nendstream", data.len());
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] \
           /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream("BT /F1 10 Tf 50 50 Td (abcde) Tj ET").into_bytes(),
        format!(
            "<< /Type /Font /Subtype /Type3 /FontBBox [0 {descent} {widths} {ascent}] \
             /FontMatrix [{font_matrix}] /CharProcs << /a 7 0 R >> /Encoding << /Type /Encoding \
             /Differences [97 /a /a /a /a /a] >> /FirstChar 97 /LastChar 101 \
             /Widths [{widths} {widths} {widths} {widths} {widths}] /FontDescriptor 6 0 R \
             /Resources << >> >>"
        )
        .into_bytes(),
        format!(
            "<< /Type /FontDescriptor /FontName /T3 /Flags 4 /FontBBox [0 {descent} {widths} \
             {ascent}] /ItalicAngle 0 /Ascent {ascent} /Descent {descent} /CapHeight {ascent} \
             /StemV 10 >>"
        )
        .into_bytes(),
        stream(charproc).into_bytes(),
    ];
    pdf_from_objects(&objects)
}

/// **A Type 3 box is kept only where its `/FontMatrix` leaves the vertical at the default**
/// (docs/22 §9 item 6).
///
/// The same five glyphs, 7 pt tall at 10 pt, drawn twice: once in a 1000-unit glyph space, once in
/// p20's 100-unit one. At the default both readings of the descriptor agree and the box is the
/// ink. Under `0.01` the specification's reading and LibreOffice 7.5-7.6's disagree by a factor of
/// ten, and nothing in the file says which one it is — so there is no box, and the advance, which
/// the matrix does state, is unchanged.
#[test]
fn a_type3_box_is_kept_only_at_the_default_font_matrix() {
    let profile = Profile::default();
    let run = |pdf: Vec<u8>| {
        let d = Document::open_bytes(&pdf, &profile).expect("opens");
        let a = ethos_parser_pdf::extract(&d, &profile).expect("extracts");
        let rs = runs(&a);
        assert_eq!(rs.len(), 1, "one run");
        assert_eq!(rs[0].text, "aaaaa");
        assert_eq!(
            rs[0].locator.advance,
            Some(2500),
            "5 glyphs of half an em at 10 pt"
        );
        rs[0].geometry
    };

    let default = run(type3_page(
        "0.001 0 0 0.001 0 0",
        "500",
        700,
        -200,
        "500 0 0 -200 500 700 d1 0 0 500 700 re f",
    ));
    let rect = default
        .measured()
        .unwrap_or_else(|| panic!("the default matrix keeps its box, got {default:?}"));
    assert_eq!(
        [rect.x0(), rect.y0(), rect.x1(), rect.y1()],
        [5000, 34300, 7500, 35200],
        "baseline y 350 in the top-left system, 7 pt above and 2 pt below"
    );

    assert_eq!(
        run(type3_page(
            "0.01 0 0 0.01 0 0",
            "50",
            70,
            -20,
            "50 0 0 -20 50 70 d1 0 0 50 70 re f",
        )),
        GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        "under 0.01 the envelope's units are unstated, so neither 0.9 pt nor 9 pt is a measurement"
    );
}

/// Structural: no code path assigns a box dimension from the font size.
#[test]
fn no_source_line_derives_a_box_from_the_font_size() {
    let src_dir = repo_root().join("crates/ethos-parser-pdf/src");
    let mut offenders = Vec::new();
    let mut files = 0usize;
    let mut lines = 0usize;
    for entry in std::fs::read_dir(&src_dir).expect("src readable") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            files += 1;
            let src = std::fs::read_to_string(&path).expect("readable");
            for (n, line) in src.lines().enumerate() {
                lines += 1;
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
    // **A guard that reads nothing passes.** This asserted an empty offender list and nothing
    // else, so a renamed directory, a `src/` reorganised into subdirectories `read_dir` does not
    // descend into, or a walk that simply stopped working would all have come out green while
    // reading no source at all. The floors are what make the empty list mean something; both sit
    // just below the real numbers at v2-S13.1, twenty-eight files and 16,201 lines.
    assert!(
        files >= 25,
        "only {files} source file(s) scanned in {}; the walk is broken, not the source",
        src_dir.display()
    );
    assert!(
        lines >= 14_000,
        "only {lines} source line(s) scanned; the walk is broken, not the source"
    );
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
        ethos_parser_pdf::SynthesisReason::TjGap
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
    // Type 3 /Widths [600 350 500 500 500 250 300] from code 1: codes 1 2 3 4 5 6 3 7 5 sum to
    // 4000 thousandths of 24pt, 96pt. The `fi` code advances once each time it appears; an advance
    // per character would add two more 500s and reach 12000.
    assert_eq!(
        r.locator.advance,
        Some(9600),
        "one advance per code, however many characters it decodes to"
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

    // docs/22 §9 item 2. The box turns with the page, and turned it runs past the page's edge.
    // The pen runs up user x from 36 to 145.044, which /Rotate 90 lays along display y from 36
    // to 145.044 — past a display height of 144. Ethos's own `layout.json` for this fixture has
    // [6824, 3742, 8489, 14488], past 14400 too. So the box is measured and un-emittable, exactly
    // as D4-S5 answers ink the document draws off its page. Through 0.57.0 it was a horizontal
    // box that fit, which was the wrong rectangle.
    assert_eq!(
        r.geometry,
        GeometryPresence::Absent(GeometryAbsence::MeasuredOffPage),
        "the turned box runs past the page, and is not laid along x to fit"
    );
    assert!(
        !r.findings
            .contains(&ethos_parser_core::TextFinding::OffPage),
        "the origin (7200, 3600) is on the page, and the finding tests the origin"
    );
    let repr = ethos_parser_pdf::to_representation(&a, &Profile::default())
        .expect("a turned box past the page still seals");
    let declared = repr
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE)
        .expect("the run is not groundable, so the gap is declared");
    for expected in [
        "1 of 1 text node(s)",
        "**Four reasons",
        "(v1-S6.2, D4-S5, v2.2-S3)",
        "have their origin inside the visible page",
    ] {
        assert!(
            declared.detail.contains(expected),
            "missing `{expected}`: {}",
            declared.detail
        );
    }
    assert!(
        !declared
            .detail
            .contains("its origin and an `off-page-text` finding"),
        "the sentence must not claim a finding this run does not carry: {}",
        declared.detail
    );
}

/// **`/Rotate` turns the side a box lies on, not only the line it lies along.**
///
/// `rotation-90`, the only `/Rotate` text in any corpus, runs past its page, so it pins no box.
/// These two pages show the same upright run, `(AB)` at user (50, 60), 10 pt, 5 pt per glyph,
/// Helvetica's 718/−207. A quarter turn lays the pen down the page with the glyph tops toward +x;
/// a half turn lays it toward −x with the tops toward +y. Carrying the pen through `/Rotate` but
/// not the glyphs' y axis would keep both lines and put both boxes on the far side of them.
#[test]
fn a_rotated_page_puts_the_box_on_the_side_the_glyph_tops_point() {
    let widths: String = std::iter::repeat_n("500", 95).collect::<Vec<_>>().join(" ");
    let stream = b"BT /F1 10 Tf 1 0 0 1 50 60 Tm (AB) Tj ET";
    let page = |rotate: u32| {
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 300] /Rotate {rotate} \
             /Resources << /Font << /F1 6 0 R >> >> /Contents 5 0 R >>"
        )
        .into_bytes()
    };
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        page(90),
        page(180),
        format!("<< /Length {} >>\nstream\n", stream.len())
            .into_bytes()
            .into_iter()
            .chain(stream.iter().copied())
            .chain(b"\nendstream".iter().copied())
            .collect(),
        format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding \
             /FirstChar 32 /LastChar 126 /Widths [{widths}] >>"
        )
        .into_bytes(),
    ];

    let profile = Profile::default();
    let d = Document::open_bytes(&pdf_from_objects(&objects), &profile).expect("opens");
    let a = ethos_parser_pdf::extract(&d, &profile).expect("extracts");
    let rect = |i: usize| {
        let r = &a.pages[i].runs[0];
        assert_eq!(r.text, "AB");
        let b = r
            .geometry
            .measured()
            .unwrap_or_else(|| panic!("page {i} should be measured, got {:?}", r.geometry));
        [b.x0(), b.y0(), b.x1(), b.y1()]
    };

    assert_eq!(a.pages[0].rotation, 90);
    assert_eq!(
        rect(0),
        [5793, 5000, 6718, 6000],
        "down the page from (60, 50), tops toward +x: x 60 − 2.07 to 60 + 7.18"
    );
    assert_eq!(a.pages[1].rotation, 180);
    assert_eq!(
        rect(1),
        [14000, 5793, 15000, 6718],
        "toward −x from (150, 60), tops toward +y: y 60 − 2.07 to 60 + 7.18"
    );
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

    let a = ethos_parser_pdf::extract(&doc, &profile).expect("and then extracts normally");
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
        codes.contains(&ethos_parser_pdf::limitations::XREF_ENTRY_PADDED),
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
        !codes.contains(&ethos_parser_core::codes::TABLES_NOT_EXTRACTED),
        "the false-capability partner is gone now that tables are detected: {codes:?}"
    );
    // The leftover that IS still true: a grid stroked as bare ruling lines is not read as ruled.
    assert!(
        codes.contains(&ethos_parser_core::codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED),
        "the narrowed leftover must still be declared: {codes:?}"
    );

    // And the table itself: the S2 golden. Six `Tm`/`Tj` pairs, zero path operators, 3x2.
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1, "the alignment rule must find this grid");
    let t = tables[0];
    assert_eq!((t.rows, t.columns), (3, 2));
    assert_eq!(
        t.rule,
        ethos_parser_core::TABLE_DETECTION_UNRULED_V1,
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
            ethos_parser_pdf::extract(&doc, &profile),
            ethos_parser_pdf::extract(&doc, &profile),
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
    let mut extracted = 0usize;
    let mut fixtures = 0usize;
    for f in manifest()["fixtures"].as_array().expect("fixtures") {
        fixtures += 1;
        let path = path_in(f["root"].as_str().unwrap(), f["path"].as_str().unwrap());
        let profile = Profile::default();
        // Two deliberate skips: the corpus contains documents this profile refuses to open and
        // documents it opens but cannot extract, and both are correct outcomes rather than
        // failures. What was missing is any record of how many got through — see the floor below.
        let Ok(doc) = Document::open(&path, &profile) else {
            continue;
        };
        let Ok(a) = ethos_parser_pdf::extract(&doc, &profile) else {
            continue;
        };
        extracted += 1;
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

    // **The old floor counted the wrong thing.** `total > 10` counts *runs*, and one small
    // fixture produces more than ten of them — so fifty-four of the fifty-five could have stopped
    // opening or stopped extracting and this gate, which `.github/workflows/ci.yml`'s
    // `v0-locators` names and `docs/history/03-V0-SCOPE.md` §5 cites, would have stayed green on the
    // strength of one document. The quantity that matters is how many fixtures reached the
    // assertions, and forty-four do at v2-S13.1 — the same forty-four
    // `an_injected_unknown_operator_stops_the_parse` counts in `tests/robustness.rs`.
    assert!(
        fixtures >= 50,
        "the manifest listed only {fixtures} fixture(s); fifty-five is the number at v2-S13.1"
    );
    assert!(
        extracted >= 40,
        "only {extracted} of {fixtures} fixture(s) opened and extracted; forty-four is the number \
         at v2-S13.1, and a corpus that quietly stopped extracting is what this floor exists for"
    );
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

    let c = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");
    let e = ethos_parser_pdf::extract(&doc, &profile).expect("extracts from the same handle");

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
    let e2 = ethos_parser_pdf::extract(&doc, &profile).expect("extracts again");
    let c2 = ethos_parser_pdf::classify(&doc, &profile).expect("classifies again");
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
    let src_dir = repo_root().join("crates/ethos-parser-pdf/src");
    // Named rather than derived, because the rule is about the extract path specifically and not
    // about the crate. That makes the list a liability of its own: a module renamed out from
    // under it is a module this stops checking, in silence and with no offender to report. So
    // every name is asserted to still resolve, the same way `NAME_READING_EXEMPTIONS` in
    // `crates/ethos-parser-cli/tests/no_format_cli.rs` asserts every exemption still matches something.
    const EXTRACT_MODULES: [&str; 6] = [
        "extract.rs",
        "nodes.rs",
        "content.rs",
        "fonts.rs",
        "metrics.rs",
        "text_state.rs",
    ];

    let mut hits = Vec::new();
    let mut seen = Vec::new();
    let mut lines = 0usize;
    for entry in std::fs::read_dir(&src_dir).expect("src readable") {
        let path = entry.expect("entry").path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if !EXTRACT_MODULES.contains(&name.as_str()) {
            continue;
        }
        seen.push(name.clone());
        let src = std::fs::read_to_string(&path).expect("readable");
        for (n, line) in src.lines().enumerate() {
            lines += 1;
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.to_ascii_lowercase().contains("confidence") {
                hits.push(format!("{name}:{}", n + 1));
            }
        }
    }

    seen.sort();
    let mut expected: Vec<String> = EXTRACT_MODULES.iter().map(|m| (*m).to_string()).collect();
    expected.sort();
    assert_eq!(
        seen, expected,
        "the extract path this rule covers and the files that exist have diverged. A module \
         named here and missing from `crates/ethos-parser-pdf/src` is a module nothing scans, and an \
         empty hit list says exactly as much about it as a clean one would."
    );
    assert!(
        lines >= 3_800,
        "only {lines} line(s) scanned across the extract modules; 4,267 is the number at \
         v2-S13.1, so this says the files were found and not read"
    );
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
    assert_eq!(
        a.identity.artifact_type,
        ethos_parser_pdf::EXTRACT_ARTIFACT_TYPE
    );
    assert_eq!(
        a.identity.schema_version,
        ethos_parser_pdf::EXTRACT_SCHEMA_VERSION
    );
    assert_eq!(
        a.identity.profile_sha256,
        Profile::default().profile_sha256().unwrap()
    );
    assert_eq!(
        a.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V3
    );
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

    assert_eq!(
        a.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V3
    );
    assert_ne!(
        a.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V0
    );

    // Still not a table. S2's discriminator is asserted properly in its own test; this is the
    // cheap guard that nobody "fixed" two-columns by making it a 2x2 grid.
    assert!(a.pages.iter().all(|p| p.tables.is_empty()));
}

/// **The anti-cliff pair, over real PDF bytes** (v1-S5 decision 2).
///
/// pdf-inspector decides multi-column on `min_lines < 15`: fourteen lines on a page come out
/// row-interleaved and fifteen come out column-major, so a one-line edit reorders the whole
/// document (`docs/history/03-V0-SCOPE.md` §3.2). These two fixtures are that edit. They are the same
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
        assert_eq!(
            a.reading_order_rule,
            ethos_parser_core::READING_ORDER_RULE_V3
        );
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
/// This is the interaction `docs/history/09-V1-MILESTONES.md` S5 decision 7 names. Before this slice,
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
        codes.contains(&ethos_parser_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED),
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

    let rep = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let page: Vec<(u32, &str)> = rep
        .payload()
        .nodes
        .iter()
        .filter(|n| n.kind == ethos_parser_core::NodeKind::TextRun)
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
    // Not the Helvetica document any more: decision #22 answers that one from `vendor/afm/`, so
    // it no longer declares `font-widths-absent` at all.
    let a = extract_ok(engine_fx("absent-font-widths"));
    let codes: Vec<&str> = a
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect();

    assert!(
        codes.contains(&ethos_parser_pdf::limitations::FONT_WIDTHS_ABSENT),
        "/ArialMT carries no /Widths and is not a face `vendor/afm/` may answer for — the gap \
         must be declared: {codes:?}"
    );
    assert!(
        codes.contains(&ethos_parser_pdf::limitations::FORM_XOBJECT_TEXT_NOT_DESCENDED),
        "text inside form XObjects is not descended into, and that is declared: {codes:?}"
    );
    for l in &a.assurance.limitations {
        assert!(l.detail.len() > 40, "`{}` needs a real reason", l.code);
    }
}

#[test]
fn an_absent_advance_is_absent_not_zero() {
    // /ArialMT, which decision #22 deliberately refuses to supply metrics for. A Helvetica
    // document reaches a MEASURED advance here now, which is the point of that decision.
    let a = extract_ok(engine_fx("absent-font-widths"));
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
        .find(|l| l.code == ethos_parser_pdf::limitations::BROKEN_FONT_ENCODING)
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
        ethos_parser_core::ProcessingTerminalState::Complete
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
            .any(|l| l.code == ethos_parser_pdf::limitations::BROKEN_FONT_ENCODING),
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
    let e = ethos_parser_pdf::extract(&d, &profile)
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
    pdf_from_objects(&objects)
}

/// A well-formed PDF from its objects, numbered from 1 in order, with computed xref offsets.
fn pdf_from_objects(objects: &[Vec<u8>]) -> Vec<u8> {
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
// v1-S1 — ruled tables (docs/history/09-V1-MILESTONES.md S1)
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
    // `ruled-table-overlap` left this list at v2-S20 for the reason given in
    // `no_table_cell_carries_text_the_document_did_not_put_there`: it now emits no table.
    for name in ["ruled-table-grid"] {
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

/// **The cross-check agrees on the golden**, and the hostile fixture is REFUSED rather than
/// emitted with its disagreement noted beside it (v2-S20).
///
/// # What this asserted until v2-S20, and why it changed
///
/// It asserted that `ruled-table-overlap` produced a table carrying a `Mismatch` — the engine
/// saying, on the wire, *"two rectangles claim one face here and I have repaired nothing"*. That
/// was the right posture while it was the only one available, and v2-S20 measured what it was
/// worth: `ethos_parser_core::markdown` and `ethos_parser_core::html` project **every** table the artifact
/// carries and consult no check, so the grid reached a reader of either projection and the
/// disagreement reached nobody. On `nist-sp-800-218` that came to nine phantom grids of up to
/// 103 x 22 and 11 295 false-positive cell slots, every one of them already flagged by this check
/// and none of it acted on.
///
/// So the ruled rule now declines a grid whose **structural** half disagrees, and the fixture's
/// job changes with it: it still proves the engine sees the double claim and still proves nothing
/// is repaired to hide it, but the seeing is now a refusal on the artifact rather than a field
/// beside a grid. `tables::tests::the_cross_check_still_sees_two_rectangles_claiming_one_slot`
/// holds the check itself under test, and
/// `tables::tests::near_edges_fold_into_one_lattice_line` holds the case that still emits with a
/// `Mismatch`, so neither the check nor the wire state has been retired.
#[test]
fn the_locator_cross_check_agrees_on_the_golden_and_the_hostile_grid_is_refused() {
    let good = extract_ok(engine_fx("ruled-table-grid"));
    let t = &good.pages[0].tables[0];
    assert_eq!(
        t.check.outcome,
        ethos_parser_core::CheckStatus::Ok,
        "{:?}",
        t.check
    );
    assert_eq!(t.check.check_id, ethos_parser_core::LOCATOR_CHECK_V1);
    assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_V6);

    // The hostile fixture draws overlapping rectangles. No table — and the artifact says why,
    // rather than saying nothing, which is the distinction `ruled-table-candidate-refused` exists
    // to keep.
    let bad = extract_ok(engine_fx("ruled-table-overlap"));
    let found: usize = bad.pages.iter().map(|p| p.tables.len()).sum();
    assert_eq!(
        found, 0,
        "a grid whose own cross-check rejects it structurally must not reach the artifact"
    );

    let refusal = bad
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::RULED_TABLE_CANDIDATE_REFUSED)
        .expect("the refused candidate must be declared, not silently dropped");
    assert!(
        refusal.detail.contains("contradicts itself"),
        "the refusal must name the precondition that failed: {}",
        refusal.detail
    );
    assert!(
        refusal.detail.contains("structural"),
        "and carry what disagreed: {}",
        refusal.detail
    );
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
        ethos_parser_core::TABLE_DETECTION_UNRULED_V1,
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
        ethos_parser_core::CheckStatus::Ok,
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
        .find(|t| t.rule == ethos_parser_core::TABLE_DETECTION_V6)
        .expect("the painted grid must be found by the ruled rule");
    let unruled = tables
        .iter()
        .find(|t| t.rule == ethos_parser_core::TABLE_DETECTION_UNRULED_V1)
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
            ethos_parser_core::CheckStatus::Ok,
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
        ethos_parser_core::TABLE_DETECTION_V6,
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
        .find(|l| l.code == ethos_parser_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED)
        .expect("a refused candidate must be declared, not silently absent");

    assert_eq!(refused.scope, ethos_parser_core::LimitationScope::Document);
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

/// **A background panel is not evidence that a grid was drawn** (v1-S7b, `ruled-rects-v2`).
///
/// The whole-document proof of the defect `tables::a_background_panel_does_not_make_scattered_bars_a_grid`
/// pins in the unit tests. Under `ruled-rects-v1` this page emitted a 7 × 7 table holding 3 cells:
/// the three bars' edges cluster into a 49-face lattice, the panel covers every face so the
/// coherence precondition passed, and `detect_ruled` then discarded the panel again as "the table's
/// own border". A rectangle cannot be both the only evidence a face exists and not a cell.
///
/// Measured on the real thing before the fix: `cfpb-home-loan-toolkit` pages 22 and 23 paint a
/// 351 × 454 pt panel behind highlight bars and produced a 17 × 13 table holding 12 cells — on a
/// page whose structure tree declares no table at all — and a 23 × 8 against a tagged 5 × 3.
#[test]
fn a_background_panel_with_bars_on_it_is_not_a_table() {
    let a = extract_ok(engine_fx("background-panel-not-a-grid"));

    for page in &a.pages {
        assert!(
            page.tables.is_empty(),
            "a panel with bars on it is not a grid: {:?}",
            page.tables
        );
    }

    // **Refusing the grid must not cost the page its words.** A detector that got quieter by
    // dropping content would trade one defect for a worse one.
    let text: String = a
        .runs()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("Panel text") && text.contains("and right"),
        "the page's runs must survive the refusal: {text:?}"
    );

    // **And the refusal is DECLARED.** Standing rule 3: a page where a grid was implied and
    // judged incoherent must not read the same as a page that painted nothing. Before v1-S7b the
    // ruled rule had no voice at all — every one of its refusals returned an empty vector and
    // said nothing — which went unnoticed because the coherence precondition it reports almost
    // never fired.
    let refused = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::RULED_TABLE_CANDIDATE_REFUSED)
        .expect("a refused ruled candidate must be declared, not silently absent");

    assert_eq!(refused.scope, ethos_parser_core::LimitationScope::Document);
    assert!(
        refused.detail.contains("page 1"),
        "the declaration must say WHERE: {}",
        refused.detail
    );

    // Typed vocabulary, never a score (`docs/01-CONTRACT.md` §9), exactly as for the unruled one.
    let lower = refused.detail.to_ascii_lowercase();
    for scored in ["confidence", "probability", "score of", "likelihood"] {
        assert!(
            !lower.contains(scored),
            "`{scored}` in a refusal detail: {}",
            refused.detail
        );
    }
}

/// **The table v1-S8 exists to find: a grid drawn as bare stroked ruling lines.**
///
/// The whole-document proof of what `stroke_ruled::a_blank_cell_does_not_refuse_the_worksheet_it_sits_in`
/// pins in the unit tests, and the first fixture anywhere whose grid is drawn as two-point `m`/`l`
/// pairs. Every build from v1-S1 to v1-S7b emitted **nothing** here and declared
/// `stroke-ruled-tables-not-detected` while doing it.
///
/// Shaped after `cfpb-home-loan-toolkit` page 13, in the two respects that decided the slice: one
/// baseline rules three cells rather than four — the blank cell `stroke-ruled-v1` refused the whole
/// band over — and the three interior column rules are stroked while the two outer ones are not.
#[test]
fn a_grid_drawn_as_stroked_ruling_lines_is_a_table() {
    let a = extract_ok(engine_fx("stroke-ruled-worksheet"));

    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1, "one band, one table: {tables:?}");
    let t = tables[0];
    assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_STROKE_V1);

    // **Five baselines bound FOUR rows.** The page shows five, and the top one's upper edge was
    // never drawn — step 4 does not supply it. This is `undrawn-table-edges-not-supplied` on a
    // document small enough to read by hand, and it is the same offset the real page 13 has.
    assert_eq!((t.rows, t.columns), (4, 4));
    assert_eq!(t.cells.len(), 16);

    // The heading row sits ABOVE the topmost rule, so it is outside the table entirely rather
    // than pulled into it — nothing is claimed that the ink did not bound.
    let text: String = t.cells.iter().map(|c| c.text.as_str()).collect();
    assert!(
        !text.contains("Item"),
        "the unbounded heading row is not a row: {text:?}"
    );
    assert!(
        text.contains("Lender") && text.contains("Rate") && text.contains("Term"),
        "the bounded rows are: {text:?}"
    );

    // Fabrication 0, the S1 invariant, on the newest rule.
    for page in &a.pages {
        for table in &page.tables {
            for cell in &table.cells {
                let from_runs: String = cell
                    .run_indices
                    .iter()
                    .filter_map(|i| page.runs.get(*i))
                    .map(|r| r.text.as_str())
                    .collect();
                assert_eq!(from_runs, cell.text, "a cell's text is its runs, always");
            }
        }
    }

    // The profile no longer claims it cannot do this.
    assert!(
        !a.assurance
            .limitations
            .iter()
            .any(|l| l.code == "stroke-ruled-tables-not-detected"),
        "a build that emits stroke-ruled tables must not declare it cannot"
    );
}

/// **The same horizontal ink with no vertical rules is not a grid.**
///
/// Byte for byte the rules of `stroke-ruled-worksheet` and none of its uprights, so the band and
/// its column lines are built identically and the only difference is whether the author drew the
/// boundaries. `stroke-ruled-v1` emitted six tables on `cfpb-home-loan-toolkit`'s Closing
/// Disclosure pages on exactly this evidence.
#[test]
fn rules_that_end_at_a_common_x_are_not_a_column_the_author_drew() {
    let a = extract_ok(engine_fx("stroke-ruled-columns-not-drawn"));
    assert!(
        a.pages.iter().all(|p| p.tables.is_empty()),
        "no vertical ink, no columns: {:?}",
        a.pages
            .iter()
            .flat_map(|p| p.tables.iter())
            .collect::<Vec<_>>()
    );

    // Refusing must not cost the page its words.
    let text: String = a
        .runs()
        .map(|r| r.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(text.contains("Lender") && text.contains("Rate"), "{text:?}");

    // **And the refusal is DECLARED** — standing rule 3. The parked `stroke-ruled-v1` had no
    // wiring for this at all and declined bands in silence; that was a precondition of shipping.
    let refused = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::STROKE_RULED_TABLE_CANDIDATE_REFUSED)
        .expect("a refused stroke-ruled candidate must be declared, not silently absent");
    assert_eq!(refused.scope, ethos_parser_core::LimitationScope::Document);
    assert!(
        refused.detail.contains("page 1"),
        "the declaration must say WHERE: {}",
        refused.detail
    );
    let lower = refused.detail.to_ascii_lowercase();
    for scored in ["confidence", "probability", "score of", "likelihood"] {
        assert!(!lower.contains(scored), "`{scored}` in: {}", refused.detail);
    }
}

/// **A form's field boxes are not a table**, and this is why `irs-form-1040-2025` stays at zero.
///
/// A stroked 2 × 2 whose four faces are also four widget `/Rect`s. Under step 5 alone this is a
/// perfectly good grid — its column rule is drawn — and it is refused on whose rectangle it is.
/// The contrast that makes the rule a rule rather than a veto on forms is
/// `stroke-ruled-worksheet`, which has no widgets, and the real `cfpb-home-loan-toolkit` page 13,
/// which carries 25 of them inset inside its printed cells and is still a table.
#[test]
fn a_grid_of_form_field_boxes_is_not_a_table() {
    let a = extract_ok(engine_fx("stroke-ruled-field-boxes"));
    assert!(
        a.pages.iter().all(|p| p.tables.is_empty()),
        "the boxes are the fields' own: {:?}",
        a.pages
            .iter()
            .flat_map(|p| p.tables.iter())
            .collect::<Vec<_>>()
    );

    // The widgets themselves are still nodes — this refuses a TABLE, it does not drop content.
    let fields: usize = a
        .pages
        .iter()
        .map(|p| {
            p.objects
                .iter()
                .filter(|o| {
                    matches!(
                        o.attributes,
                        ethos_parser_core::NodeAttributes::FormField(_)
                    )
                })
                .count()
        })
        .sum();
    assert_eq!(fields, 4, "four widgets, four nodes");

    let refused = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::STROKE_RULED_TABLE_CANDIDATE_REFUSED)
        .expect("the refusal is the informative part of finding nothing here");
    assert!(
        refused.detail.contains("form field"),
        "the declaration must name WHY: {}",
        refused.detail
    );
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
            .any(|l| l.code == ethos_parser_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED),
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
            .any(|l| l.code == ethos_parser_core::codes::UNRULED_TABLE_CANDIDATE_REFUSED),
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
    // `ruled-table-overlap` was on this list until v2-S20 and is not any more: the ruled rule
    // now refuses a grid whose structural cross-check rejects it, so that fixture emits no table
    // and there is no cell here to check. It is not left in with a weakened assertion — a test
    // that tolerates zero tables is a test that would keep passing if the detector stopped
    // working. See `the_locator_cross_check_agrees_on_the_golden_and_the_hostile_grid_is_refused`.
    for fixture in [
        "ruled-table-grid",
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
    use ethos_parser_core::StructuralLocator;

    // Half one: a tagged document yields the roles ITS OWN TREE gives.
    let tagged = extract_ok(engine_fx("tagged-structure-roles"));
    let bound: Vec<&std::sync::Arc<ethos_parser_core::PdfTaggedLocator>> = runs(&tagged)
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
        codes.contains(&ethos_parser_core::codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
        "and must say why it found none: {codes:?}"
    );
    assert!(
        !codes.contains(&ethos_parser_core::codes::STRUCTURAL_LOCATORS_NOT_CLAIMED),
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
    use ethos_parser_core::StructuralLocator;

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
        !codes.contains(&ethos_parser_core::codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
        "this document carries a tree; only its coverage is partial: {codes:?}"
    );
    assert!(
        codes.contains(&ethos_parser_core::codes::STRUCTURE_MCID_UNBOUND),
        "and the partial coverage is what gets declared: {codes:?}"
    );
}

/// **The join is exact.** An id the tree does not cite binds nothing, and binds nothing *nearby*.
#[test]
fn the_mcid_join_is_exact_equality_and_never_nearest_match() {
    use ethos_parser_core::StructuralLocator;

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
    use ethos_parser_core::StructuralLocator;

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

    let e = ethos_parser_pdf::extract(&doc, &profile)
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
    use ethos_parser_core::TaggedGridStatus;

    let a = extract_ok(engine_fx("tagged-table-agrees"));
    let tables: Vec<_> = a.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(tables.len(), 1);
    let t = tables[0];

    assert_eq!((t.rows, t.columns), (2, 2));
    assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_V6);

    let check = t
        .tagged_check
        .as_ref()
        .expect("the tree describes a table here, so the check runs");
    assert_eq!(check.check_id, ethos_parser_core::TAGGED_GRID_CHECK_V1);
    assert_ne!(
        check.check_id,
        ethos_parser_core::LOCATOR_CHECK_V1,
        "this is a second check, not a widening of the first"
    );
    assert_eq!(check.outcome, TaggedGridStatus::Ok, "{check:?}");

    // Both cross-checks pass, and they are asking different questions.
    assert_eq!(t.check.outcome, ethos_parser_core::CheckStatus::Ok);

    // **No double-emission** (v2-S24). The tree's `/Table` was MATCHED by the painted grid, so it
    // paired in the Some arm and is recorded as this geometric table's `tagged_check`. It must NOT
    // also be emitted as a `tagged-tables-v1` table: a gold `/Table` reaches the artifact once,
    // geometrically OR tagged, never both. The tagged emit fires only in the None arm, which this
    // page never reaches.
    let tagged: usize = a.pages.iter().map(|p| p.tagged_tables.len()).sum();
    assert_eq!(
        tagged, 0,
        "the tree's /Table paired with the detected grid, so it must not ALSO be emitted as a \
         tagged table — that is the double-emission the None-arm guard prevents"
    );
}

/// **Tagged and geometric disagree**: named, counted, and nothing repaired.
#[test]
fn a_tagged_table_that_contradicts_the_painted_grid_reports_a_mismatch() {
    use ethos_parser_core::{TaggedGridFault, TaggedGridStatus};

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
        codes.contains(&ethos_parser_core::codes::STRUCTURE_ITEM_WITHOUT_CONTENT),
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
    assert_eq!(t[0].rule, ethos_parser_core::TABLE_DETECTION_UNRULED_V1);

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
    assert_eq!(
        two.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V3
    );
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
        .filter(|o| {
            matches!(
                o.attributes,
                ethos_parser_core::NodeAttributes::FormField(_)
            )
        })
        .collect();
    assert_eq!(
        fields.len(),
        1,
        "one widget is one node, not a field plus a clone"
    );

    let ethos_parser_core::NodeAttributes::FormField(attrs) = &fields[0].attributes else {
        unreachable!("filtered above")
    };
    assert_eq!(attrs.field_name.as_deref(), Some("applicant_name"));
    assert_eq!(attrs.field_type.as_deref(), Some("Tx"));
    assert_eq!(
        attrs.value,
        ethos_parser_core::FieldValue::Text("Wendell Ashcroft-Byrne".into())
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
        .filter(|o| {
            matches!(
                o.attributes,
                ethos_parser_core::NodeAttributes::Annotation(_)
            )
        })
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
    let ethos_parser_core::NodeAttributes::Annotation(first) = &annots[0].attributes else {
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
            ethos_parser_core::NodeAttributes::Annotation(x) => {
                x.flags.contains(&"hidden".to_string())
            }
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
        codes_of(&a).contains(&ethos_parser_core::codes::FORM_FIELD_PARENT_UNRESOLVED),
        "the broken link must be declared: {:?}",
        codes_of(&a)
    );

    // The widget is still a node, with what it declares about itself.
    let fields: Vec<_> = a
        .pages
        .iter()
        .flat_map(|p| p.objects.iter())
        .filter(|o| {
            matches!(
                o.attributes,
                ethos_parser_core::NodeAttributes::FormField(_)
            )
        })
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
        codes_of(&a).contains(&ethos_parser_core::codes::XFA_FORMS_NOT_EXTRACTED),
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
                .filter(|o| {
                    matches!(
                        o.attributes,
                        ethos_parser_core::NodeAttributes::FormField(_)
                    )
                })
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
            ethos_parser_core::NodeAttributes::FormField(f) => {
                f.value == ethos_parser_core::FieldValue::Absent
            }
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
            ethos_parser_core::NodeAttributes::FormField(f) => f.field_name.clone(),
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
    assert_eq!(t[0].rule, ethos_parser_core::TABLE_DETECTION_UNRULED_V1);

    // S1's golden.
    let ruled = extract_ok(engine_fx("ruled-table-grid"));
    let rt: Vec<_> = ruled.pages.iter().flat_map(|p| p.tables.iter()).collect();
    assert_eq!(rt.len(), 1);
    assert_eq!(rt[0].rule, ethos_parser_core::TABLE_DETECTION_V6);

    // S3's four locator states, still four.
    let tagged = extract_ok(engine_fx("tagged-structure-roles"));
    assert_eq!(runs(&tagged).len(), 5);
    assert!(runs(&tagged).iter().any(|r| matches!(
        r.structural,
        Some(ethos_parser_core::StructuralLocator::PdfArtifact(_))
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
    assert_eq!(
        two.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V3
    );
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
    let expected = ethos_parser_core::Sha256Hex::of_bytes(&raw[start..end]);
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
        ethos_parser_core::ImageMediaType::PdfEncodedSamples
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
    let rep = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let image_nodes: Vec<_> = rep
        .payload()
        .nodes
        .iter()
        .filter(|n| n.kind == ethos_parser_core::NodeKind::Image)
        .collect();
    assert_eq!(image_nodes.len(), 1);
    assert!(
        image_nodes[0].text.is_empty(),
        "an image node carries no text"
    );
    assert!(matches!(
        image_nodes[0].native_locator,
        ethos_parser_core::NativeLocator::PdfImage(_)
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

/// **A form XObject is counted on the document, not only refused in the profile** (v2.2-S2).
///
/// The third member of the `Do` family, and the one the pair above did not cover. `image-xobject-
/// drawn` and `image-declared-not-drawn` differ in whether the `Do` is written; this fixture
/// writes the same `Do` and changes the `/Subtype` to `/Form`. The result is neither an image node
/// (this profile emits nodes for `/Image`) nor a text node (this profile does not descend), so the
/// only thing on the artifact that can say the form's text existed is a count — and until v2.2-S2
/// there was none. The `else` arm incremented nothing and the placement was discarded in silence.
///
/// **The two limitations are both here on purpose, and their scopes are the test.** The
/// profile-scoped one rides on every artifact this engine writes, including artifacts for
/// documents containing no XObject at all; asserting only that would pass on a blank page.
#[test]
fn a_drawn_form_xobject_is_counted_on_the_document_that_drew_it() {
    let a = extract_ok(engine_fx("form-xobject-text-drawn"));

    // The page's own sentence is a node. The form's is not — and the fixture draws both through
    // the SAME font object, so "the form was unreadable" is not available as an explanation.
    let texts: Vec<&str> = runs(&a).iter().map(|r| r.text.as_str()).collect();
    assert_eq!(
        texts,
        vec!["Drawn by the page"],
        "the page's text is read and the form's is not; a reader that descended would show both, \
         and a reader that lost the page's would show neither"
    );
    assert_eq!(
        a.pages.iter().map(|p| p.images.len()).sum::<usize>(),
        0,
        "a `/Form` is not a picture: emitting an image node for it would put something on the \
         wire the document never called one"
    );

    let doc_scoped = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::FORM_XOBJECTS_NOT_DESCENDED)
        .expect("the `Do` happened HERE, and the artifact has to say so");
    assert_eq!(
        doc_scoped.scope,
        ethos_parser_core::LimitationScope::Document
    );
    assert!(
        doc_scoped.detail.starts_with("1 form XObject(s)"),
        "the COUNT is the whole content of this limitation, not its prose: {}",
        doc_scoped.detail
    );

    // Present alongside, and different. Losing the distinction is how this defect survived.
    assert!(
        a.assurance.limitations.iter().any(|l| l.code
            == ethos_parser_pdf::limitations::FORM_XOBJECT_TEXT_NOT_DESCENDED
            && l.scope == ethos_parser_core::LimitationScope::Profile),
        "the profile-scoped statement of policy stays where it was"
    );

    // **The negative half, which is what makes the positive one mean anything.** A document-scoped
    // code that appeared on every document would be the profile-scoped one under a second name.
    for name in [
        "image-xobject-drawn",
        "image-declared-not-drawn",
        "markdown-two-blocks",
    ] {
        let other = extract_ok(engine_fx(name));
        assert!(
            !other
                .assurance
                .limitations
                .iter()
                .any(|l| l.code == ethos_parser_core::codes::FORM_XOBJECTS_NOT_DESCENDED),
            "{name} draws no form XObject and must not carry the count. \
             `image-xobject-drawn` is the sharp case: it writes the same `Do`, and the only \
             difference is the `/Subtype`"
        );
        assert!(
            other
                .assurance
                .limitations
                .iter()
                .any(|l| l.code == ethos_parser_pdf::limitations::FORM_XOBJECT_TEXT_NOT_DESCENDED),
            "{name} still carries the PROFILE-scoped one, which is exactly why it could not \
             stand in for the document-scoped one"
        );
    }
}

/// **A composite font's widths come from its descendant, and the CID is the key** (v2.2-S3).
///
/// `load_widths` read `/Widths` and `/FirstChar` — the SIMPLE font shape. PDF 32000-1 §9.7.4.3
/// puts a composite font's widths on the descendant CIDFont as `/W`, with `/DW` as the default,
/// and a `/Type0` dictionary carries no `/Widths` at all. So every composite font fell through to
/// `WidthSource::Absent` and reported an unknown advance while the document supplied a perfectly
/// good one — measured on `opendataloader-bench`: **50 of 50 documents and 72 of 72 composite
/// fonts**, every one of which had `/W` or `/DW` in the file. A 100% false-positive rate.
///
/// **The whole suite passed while that was true**, because neither owned corpus contained a
/// single CIDFont — `grep -l CIDFontType fixtures/` matched nothing. This fixture is the shape
/// that was missing, and its four glyphs take their widths from four different code paths.
#[test]
fn a_composite_font_takes_its_widths_from_the_descendant_cidfont() {
    let a = extract_ok(engine_fx("composite-font-cid-widths"));
    let r = runs(&a);
    assert_eq!(r.len(), 1, "one `Tj`, one run");

    // Codes are two bytes because Identity-H says so, and each is its own CID.
    assert_eq!(r[0].text, "ABCE");
    assert_eq!(r[0].char_codes, vec![1, 2, 3, 5]);

    // **The number that proves all four paths.** `/W [1 [500 750] 5 7 250]` and `/DW 900` give
    // CID 1 -> 500 and CID 2 -> 750 from the ARRAY form, CID 3 -> 900 from the font's own `/DW`
    // because `/W` names it nowhere, and CID 5 -> 250 from the RANGE form. At 12 pt that is
    // 6.00 + 9.00 + 10.80 + 3.00 = 28.80 pt, and no two of the four widths are equal — so a
    // reader that mis-sourced any single one lands on a different total.
    //
    // `/DW` is 900 rather than 1000 for a measured reason: 1000 is also §9.7.4.3's value when the
    // key is absent, so at 1000 a mutant that replaced the default with zero SURVIVED this
    // assertion. `the_spec_default_applies_when_dw_is_absent` in `fonts.rs` covers the omitted
    // case, which a fixture built by this generator cannot express.
    assert_eq!(
        r[0].locator.advance,
        Some(2_880),
        "28.80 pt in centipoints: 500 + 750 + 900 + 250 glyph units at 12 pt"
    );

    // No width means no ink box means no grounding, which is what the defect actually cost.
    let box_ = r[0]
        .geometry
        .measured()
        .expect("with an advance and a descriptor the ink box is MEASURED");
    // The box is as wide as the advance the widths produced. Asserting the number rather than
    // just its presence: a box that existed but measured the wrong width would be the same
    // silent-plausible failure one layer along.
    assert_eq!(
        box_.x1() - box_.x0(),
        2_880,
        "the measured box spans the 28.80 pt the four widths add up to"
    );
    assert!(
        !a.assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_pdf::limitations::FONT_WIDTHS_ABSENT),
        "the document supplies widths; claiming otherwise was the defect"
    );
}

/// **The refusal, and the reason it gives** (v2.2-S3).
///
/// The other half, and it is not optional: a reader that returned widths for every composite font
/// would pass the test above and be wrong wherever the code is not the CID. `/W` is keyed by CID
/// and the code→CID map is the `/Encoding` CMap, which this profile does not parse. Under a
/// non-Identity CMap the CID is unknown, and a width looked up with the wrong key is a plausible
/// number for the wrong glyph — the one failure a consumer cannot detect.
///
/// The fixture is the SAME descendant with the SAME `/W` and `/DW`, so the only difference is the
/// encoding. And the reason matters as much as the refusal: before this slice the message blamed
/// un-vendored standard-14 AFM tables, which cannot apply to an embedded composite subset — it
/// was the wrong explanation on 50 of the 51 documents that printed it.
#[test]
fn a_composite_font_under_an_unparsed_cmap_refuses_rather_than_guessing() {
    let a = extract_ok(engine_fx("composite-font-non-identity-cmap"));
    let r = runs(&a);
    assert_eq!(r.len(), 1);

    // The TEXT still decodes — `/ToUnicode` is authoritative for characters and says nothing
    // about CIDs. Only the advance is unavailable, and only the advance is withheld.
    assert_eq!(r[0].text, "ABCE", "refusing a width never costs the text");
    assert_eq!(
        r[0].locator.advance, None,
        "the code is not the CID under this CMap, and nothing here can say what is"
    );

    let l = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_pdf::limitations::FONT_WIDTHS_ABSENT)
        .expect("refused, and declared");
    assert!(
        l.detail.contains("UniJIS-UCS2-H") && l.detail.contains("code to CID"),
        "the reason must name the encoding it could not read: {}",
        l.detail
    );
    assert!(
        l.detail.contains("not the standard-14 case"),
        "and must say it is NOT the AFM case, which is what it wrongly claimed for every \
         composite font before v2.2-S3: {}",
        l.detail
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
                .contains(&ethos_parser_core::TextFinding::InvisibleRenderMode)
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
        .find(|l| l.code == ethos_parser_core::codes::INVISIBLE_RENDER_MODE_TEXT)
        .expect("the observation is declared as well as flagged");
    assert_eq!(l.scope, ethos_parser_core::LimitationScope::Document);
    assert!(l.detail.contains("NONE was removed"));

    // The flag survives the projection: a finding that reached the extract and not the record
    // would be invisible to every consumer.
    let rep = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let hidden = rep
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Hidden instruction")
        .expect("the node is in the record");
    match &hidden.attributes {
        ethos_parser_core::NodeAttributes::TextRun(t) => assert_eq!(
            t.findings,
            vec![ethos_parser_core::TextFinding::InvisibleRenderMode]
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
        .filter(|x| {
            x.findings
                .contains(&ethos_parser_core::TextFinding::OffPage)
        })
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
        .find(|l| l.code == ethos_parser_core::codes::OFF_PAGE_TEXT)
        .expect("counted as well as flagged");
    assert_eq!(l.scope, ethos_parser_core::LimitationScope::Document);
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
        let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
        ethos_parser_pdf::build_overlay(&doc, &extract, &profile).expect("overlays")
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
        text.contains("ethos-parser: image "),
        "the painted image is marked"
    );
    assert!(
        text.contains("NO rectangle this overlay can draw"),
        "and the count of what could NOT be drawn is on the page too"
    );
    assert!(
        text.contains("EthosParserOverlay"),
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
    assert_eq!(
        two.reading_order_rule,
        ethos_parser_core::READING_ORDER_RULE_V3
    );
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
        r[0].findings
            .contains(&ethos_parser_core::TextFinding::OffPage),
        "text in the cropped-away margin is outside the VISIBLE box and says so"
    );
    assert_eq!(r[0].text, "Near the top");

    // The projection seals, which is the assertion that actually failed before the fix.
    ethos_parser_pdf::to_representation(&a, &Profile::default())
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
        !codes.contains(&ethos_parser_pdf::limitations::BROKEN_FONT_ENCODING),
        "nothing about this font is broken, and the artifact must not say otherwise: {codes:?}"
    );
    assert!(
        !codes.contains(&ethos_parser_core::codes::COMPOSITE_FONT_CODES_FROM_TOUNICODE),
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
        !codes.contains(&ethos_parser_pdf::limitations::BROKEN_FONT_ENCODING),
        "zero runs are dropped now; 8417 were, under a false statement about this document"
    );

    // The composite half is still an interim, and says so on this document rather than being
    // discovered the way the simple half was.
    assert!(
        codes.contains(&ethos_parser_core::codes::COMPOSITE_FONT_CODES_FROM_TOUNICODE),
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

// -------------------------------------------------------------------------------------------
// 17. A run that draws no ink has no ink box (v1-S6.2)
// -------------------------------------------------------------------------------------------

/// **A rectangle around nothing is not a measurement.**
///
/// The fixture's first run is 28 spaces whose advance carries it off the right edge of the page.
/// Before v1-S6.2 the reader built a box for it out of the font's ascent/descent envelope and the
/// run's advance — the box was never ink — and because that rectangle left the page, `seal` refused
/// the **entire document**. That is what made 491 of `nist-sp-800-53r5`'s 492 pages unreadable.
///
/// Both halves are asserted here: the whitespace run reports why there is no box, and the visible
/// run beside it still gets one. A repair that took boxes away from real text would be a different
/// bug wearing this one's clothes.
#[test]
fn a_whitespace_run_reports_no_ink_rather_than_a_box_around_nothing() {
    let a = extract_ok(engine_fx("whitespace-past-the-page-edge"));
    let r = runs(&a);
    assert_eq!(r.len(), 2);

    assert!(r[0].text.trim().is_empty(), "the first run is whitespace");
    assert_eq!(
        r[0].geometry,
        ethos_parser_core::GeometryPresence::Absent(
            ethos_parser_core::GeometryAbsence::NoInkToMeasure
        ),
        "a run of spaces has nothing to measure — and that is NOT the same as a reader that \
         could not measure, which is what `not_reported_by_reader` would claim"
    );

    assert_eq!(r[1].text, "Visible");
    assert!(
        r[1].geometry.measured().is_some(),
        "text that draws ink still gets its box"
    );

    // The absence must not be counted as a limitation of this reader.
    assert!(
        !r[0].geometry.is_declarable_limitation(),
        "nothing failed here, so nothing is declarable"
    );
    assert!(r[1].geometry.is_groundable());

    // And the document seals, which is the assertion that failed before the repair.
    ethos_parser_pdf::to_representation(&a, &Profile::default())
        .expect("a page with trailing whitespace produces a representation");
}

/// **Both NIST benchmarks produce an artifact again.**
///
/// They had not since `check_box_within_page` was written: every one of the 3 452 boxes that
/// tripped it was whitespace, and no run with visible text was ever out of place. Two of the three
/// real benchmark documents were unreadable over content that draws nothing.
#[test]
fn the_documents_that_could_not_be_read_can_be_read() {
    for name in ["nist-sp-800-63b.pdf", "nist-sp-800-53r5.pdf"] {
        let a = extract_ok(path_in("benchmark", name));
        let rep = ethos_parser_pdf::to_representation(&a, &Profile::default())
            .unwrap_or_else(|e| panic!("{name} must seal: {e}"));
        assert!(
            rep.payload().nodes.len() > 1000,
            "{name} produced a representation with real content"
        );

        // The count that used to read as 11 663 reader failures is split by reason. Only the
        // nodes this reader genuinely could not measure are its limitation.
        // On the REPRESENTATION's assurance, not the extract's: the geometry omission is counted
        // during projection, because it is a fact about what the target schema can carry.
        let detail = rep
            .payload()
            .assurance
            .limitations
            .iter()
            .find(|l| l.code == ethos_parser_core::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE)
            .map(|l| l.detail.clone())
            .unwrap_or_default();
        assert!(
            detail.contains("could NOT be measured") && detail.contains("had NOTHING to measure"),
            "the two reasons must be reported apart, not summed: {detail}"
        );
    }
}

/// Five conformance documents keep the geometry they had, which is how a targeted repair proves
/// itself.
///
/// The name said *the conformance corpus* over a hand-picked five of the fifteen, and the
/// five are not a sample that could be widened to the rest: the property below holds only of
/// documents with no whitespace-only run, and the corpus contains documents that have one.
/// Naming the five is the honest version of the same guard.
#[test]
fn five_conformance_documents_keep_every_box_they_had() {
    // **Five named documents, not the corpus.** The name says *the conformance corpus* and this
    // is a hand-picked five of it, which is not an oversight to widen: the property asserted
    // below — that no run resolves to `NoInkToMeasure` — is true only of documents that contain
    // no whitespace-only run, and the corpus does contain such documents. Running it over
    // everything would fail on the corpus being what it is. So the list stays, and what changes
    // is that it is now counted rather than merely written, and every entry is asserted to have
    // produced runs: a fixture that stopped extracting would otherwise satisfy `.all()`
    // vacuously and take its share of the guarantee with it.
    let mut checked = 0usize;
    for name in [
        "synthetic/simple-text/document.pdf",
        "synthetic/two-lines/document.pdf",
        "synthetic/two-columns/document.pdf",
        "synthetic/ligature-fi-embedded-font/document.pdf",
        "synthetic/table-regular-grid/document.pdf",
    ] {
        let a = extract_ok(conformance(name));
        let runs = runs(&a);
        assert!(
            !runs.is_empty(),
            "{name} produced no runs at all, so `all()` below holds vacuously and this document \
             has quietly dropped out of the guarantee"
        );
        assert!(
            runs.iter().all(|r| !matches!(
                r.geometry,
                ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::NoInkToMeasure
                )
            )),
            "{name} has no whitespace-only run, so nothing in it may change"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 5,
        "five conformance documents carry this guarantee; {checked} were checked"
    );
}

// -------------------------------------------------------------------------------------------

/// **A box the document draws off the page is measured, and un-emittable — not a transform bug.**
///
/// `check_box_within_page` refuses an out-of-page box on the stated grounds that it *"means the
/// measurement or the coordinate transform is wrong"*. v1-S6.2 met that error from one direction —
/// a rectangle around whitespace — and answered it by not claiming a box. Six of the two hundred
/// DP-Bench documents meet it from the other: `01030000000029.pdf` sets
/// `9.9626 0 0 9.9626 -435.1181 674.3054 Tm` against `/MediaBox [0 0 510.236 737.008]`, and the
/// engine reported `x0 = -43512` — that number, correctly transformed and quantized. The
/// measurement was right and the document draws off-canvas, and each of the six produced **no
/// artifact at all**.
///
/// Both halves are asserted, for the reason the whitespace test asserts both: a repair that took
/// boxes away from text that IS on the page would be a different bug wearing this one's clothes.
#[test]
fn ink_the_document_draws_off_the_page_reports_why_rather_than_refusing_the_document() {
    let a = extract_ok(engine_fx("ink-past-the-media-box"));
    let r = runs(&a);
    assert_eq!(r.len(), 2);

    assert_eq!(r[0].text, "Off the left edge");
    assert_eq!(
        r[0].geometry,
        ethos_parser_core::GeometryPresence::Absent(
            ethos_parser_core::GeometryAbsence::MeasuredOffPage
        ),
        "the box was measured and the DOCUMENT put it off the page — which is neither \
         `not_reported_by_reader` (the reader did not fail) nor `no_ink_to_measure` (the run \
         draws glyphs)"
    );
    assert!(
        !r[0].geometry.is_declarable_limitation(),
        "the document made this choice, so it is not charged to this reader"
    );
    assert!(
        !r[0].geometry.is_groundable(),
        "no page-relative rectangle exists, so it cannot enter `ethos.grounding.v1`"
    );

    assert_eq!(r[1].text, "On the page");
    assert!(
        r[1].geometry.measured().is_some(),
        "the absence is per-run: content on the canvas still gets its box"
    );

    // And the document seals, which is the assertion that failed before the repair.
    let repr = ethos_parser_pdf::to_representation(&a, &Profile::default())
        .expect("a page that draws off-canvas still produces a representation");

    // Its off-page run starts off the page too, so it carries the finding, and the sentence
    // saying so is 0.57.0's — not the branch for a run whose origin is on the page.
    let declared = repr
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE)
        .expect("the off-page run is not groundable, so the gap is declared");
    assert!(
        declared
            .detail
            .contains("its origin and an `off-page-text` finding"),
        "{}",
        declared.detail
    );
    assert!(
        !declared
            .detail
            .contains("have their origin inside the visible page"),
        "{}",
        declared.detail
    );
}

/// **Nothing is dropped and nothing is clamped — the run is still evidence.**
///
/// The two forbidden answers to an out-of-page box are the reason this variant exists rather than
/// either of them: clamping fabricates a coordinate the document does not contain, and dropping
/// the run is a silent erasure. What the artifact keeps is asserted here, because "we kept it" is
/// the whole claim and an absent box is the only thing that may be missing.
#[test]
fn an_off_page_run_keeps_its_text_its_origin_and_its_finding() {
    let a = extract_ok(engine_fx("ink-past-the-media-box"));
    let r = runs(&a);

    // Found BY its absence, not by index. The preservation claim is only about the run this
    // slice changed, and a test that reached for `r[0]` would keep passing if the absence
    // stopped being produced at all — a guard reading its own subject wrongly, which is the
    // v2-S13.1 defect.
    let off = r
        .iter()
        .find(|x| {
            x.geometry
                == ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::MeasuredOffPage,
                )
        })
        .expect("the fixture has a run whose measured box the document draws off the page");

    assert_eq!(off.text, "Off the left edge");
    assert!(
        off.findings
            .contains(&ethos_parser_core::TextFinding::OffPage),
        "the engine already had a word for this content and raised it before deciding to \
         refuse the document over the box"
    );
    assert!(
        off.locator.origin_x < 0,
        "the origin is negative and is reported as measured, not nudged to zero: clamping \
         would fabricate a coordinate the document does not contain"
    );
    assert!(
        !r[1]
            .findings
            .contains(&ethos_parser_core::TextFinding::OffPage),
        "and the on-page run is not flagged, so the finding still means something"
    );
}

/// **The producer's containment test and the seal's are one invariant, not two spellings.**
///
/// `PageGeometry::contains` restates `check_box_within_page`. Two copies of one rule is exactly
/// what `docs/06-STEAL-REFUSE.md` warns goes stale, so the agreement is pinned rather than
/// assumed: every box this extractor still calls `Measured` must survive the seal, on every
/// fixture in the engine corpus at once. If `contains` were ever loosened relative to the seal,
/// some fixture here would stop producing an artifact and say so.
#[test]
fn every_measured_box_this_reader_emits_survives_the_seal() {
    let profile = Profile::default();
    let root = engine_fx("ink-past-the-media-box")
        .parent()
        .and_then(|p| p.parent())
        .expect("the engine fixture root is two hops up from a document")
        .to_path_buf();

    let mut checked = 0;
    let mut names: Vec<_> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("engine fixture root {}: {e}", root.display()))
        .map(|d| d.expect("fixture dir").path())
        .collect();
    names.sort();
    for dir in names {
        let pdf = dir.join("document.pdf");
        if !pdf.is_file() {
            continue;
        }
        let Ok(doc) = Document::open(&pdf, &profile) else {
            continue;
        };
        let Ok(a) = ethos_parser_pdf::extract(&doc, &profile) else {
            continue;
        };
        ethos_parser_pdf::to_representation(&a, &profile)
            .unwrap_or_else(|e| panic!("{dir:?} extracted but would not seal: {e}"));
        checked += 1;
    }
    assert!(
        checked > 20,
        "the sweep must actually reach the corpus — it checked {checked}"
    );
}

// -------------------------------------------------------------------------------------------
// docs/22 §9 items 1 and 2: turned text
// -------------------------------------------------------------------------------------------

/// **A turned run's box lies along its own baseline, on the side its glyph tops point.**
///
/// Through 0.57.0 the box was built from the advance, which reads only `Tm.e` and scales it by the
/// CTM's length: text turned by its text matrix advanced 0 or less and was typed
/// `no_ink_to_measure`, the absence that says nothing was drawn, and text turned by its CTM got a
/// box laid along x. Each run is found by its text, so a reorder cannot make this pass.
#[test]
fn turned_text_gets_a_box_along_its_baseline() {
    let a = extract_ok(engine_fx("rotated-and-mirrored-text"));
    let r = runs(&a);
    assert_eq!(r.len(), 7);
    let run = |text: &str| {
        *r.iter()
            .find(|x| x.text == text)
            .unwrap_or_else(|| panic!("the fixture draws `{text}`"))
    };
    let rect = |text: &str| {
        let g = run(text).geometry;
        let b = g
            .measured()
            .unwrap_or_else(|| panic!("`{text}` should be measured, got {g:?}"));
        [b.x0(), b.y0(), b.x1(), b.y1()]
    };

    assert_eq!(
        rect("Upright"),
        [2000, 27138, 6200, 28248],
        "the control, as 0.57.0 had it"
    );
    assert_eq!(
        rect("Turned"),
        [3138, 20400, 4248, 24000],
        "up the page, tops toward -x"
    );
    assert_eq!(
        rect("Downward"),
        [7752, 6000, 8862, 10800],
        "down the page, tops toward +x"
    );
    assert_eq!(rect("Inverted"), [20200, 2752, 25000, 3862], "upside down");
    assert_eq!(
        rect("Mirrored"),
        [23200, 9138, 28000, 10248],
        "mirrored, tops still up"
    );
    assert_eq!(
        run("Diagonal").geometry,
        GeometryPresence::Absent(GeometryAbsence::NotAxisAligned),
        "45 degrees has no axis-aligned rectangle, and a bounding box would claim page it does \
         not cover"
    );
    assert_eq!(
        rect("Rolled"),
        [11138, 16400, 12248, 20000],
        "turned by the CTM rather than the text matrix, and turned all the same"
    );

    // `Turned` and `Rolled` are the same picture reached two ways, so their boxes differ by
    // exactly their origins.
    let (t, o) = (&run("Turned").locator, &run("Rolled").locator);
    let (dx, dy) = (o.origin_x - t.origin_x, o.origin_y - t.origin_y);
    let (bt, bo) = (rect("Turned"), rect("Rolled"));
    assert_eq!([bt[0] + dx, bt[1] + dy, bt[2] + dx, bt[3] + dy], bo);

    // The wire advance keeps its 0.57.0 value, measured along the text matrix's x before any
    // rotation. Re-expressing it in the declared frame is left open — a known defect recorded in
    // docs/22 — and not decided here.
    for (text, advance) in [
        ("Upright", 4200),
        ("Turned", 0),
        ("Downward", 0),
        ("Inverted", -4800),
        ("Mirrored", -4800),
        ("Diagonal", 3394),
        ("Rolled", 3600),
    ] {
        assert_eq!(run(text).locator.advance, Some(advance), "{text}");
    }
}

/// **A run along neither axis is counted, declared, and sealed** — not typed as a reader failure.
///
/// The seal refuses a document whose non-groundable nodes the geometry declaration does not
/// count, so an uncounted `NotAxisAligned` would cost the whole artifact. The grounding half —
/// six elements, the diagonal omitted — is asserted where the projection is reachable, in
/// `ethos-parser-cli`'s `grounding.rs`.
#[test]
fn an_off_axis_run_is_counted_and_declared() {
    let a = extract_ok(engine_fx("rotated-and-mirrored-text"));
    let diagonal = runs(&a)
        .into_iter()
        .find(|r| r.text == "Diagonal")
        .expect("the fixture draws `Diagonal`");
    assert!(!diagonal.geometry.is_declarable_limitation());
    assert!(!diagonal.geometry.is_groundable());

    let repr = ethos_parser_pdf::to_representation(&a, &Profile::default())
        .expect("a document with a run along neither axis still seals");
    let declared = repr
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE)
        .expect("the diagonal run is not groundable, so the gap is declared");
    assert!(
        declared.detail.starts_with("1 of 7 text node(s)"),
        "{}",
        declared.detail
    );
    for expected in [
        "**Four reasons",
        "(v1-S6.2, v2.2-S3, 0.58.0)",
        "1 node(s) WERE measured and do not run along an axis",
        "`GeometryAbsence::NotAxisAligned` is the reason",
    ] {
        assert!(
            declared.detail.contains(expected),
            "missing `{expected}`: {}",
            declared.detail
        );
    }
}

// -------------------------------------------------------------------------------------------
// D4: the region reaches the wire
// -------------------------------------------------------------------------------------------

/// Every run's region, paired with its x origin, from a fixture's emitted representation.
fn regions_and_x(fixture: &str) -> Vec<(Option<u32>, i64)> {
    let a = extract_ok(engine_fx(fixture));
    let rep = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    rep.payload()
        .nodes
        .iter()
        .filter_map(|n| match (&n.attributes, &n.native_locator) {
            (
                ethos_parser_core::NodeAttributes::TextRun(t),
                ethos_parser_core::NativeLocator::Pdf(p),
            ) => Some((t.region, p.origin_x)),
            _ => None,
        })
        .collect()
}

/// **A divided page carries the regions the cut made, on the artifact** (D4-S2).
///
/// `arrange_page` computes the regions and `reorder_page` carries them on the run; the third hop —
/// `represent.rs`'s `region: run.region` — puts them on the wire, and **nothing observed it**.
/// Replacing that expression with `region: None` compiled and passed the entire workspace suite:
/// D4's twenty-one in-module ordering tests assert on hand-constructed `RunGeometry` arrays and
/// never reach a representation, and no test in any crate read `TextRunAttributes::region`. A
/// feature shipped at 0.42.0 had one production line that no test could see.
///
/// **The column check is the load-bearing half.** Asserting only that a region is present passes a
/// mutant that emits `Some(1)` for every run, which is a field with no information in it. Two
/// columns must land in *different* regions, and `two-column-14-lines` draws its left column at
/// x=4000 and its right at x=24000, so the partition is checkable against geometry the fixture
/// states rather than against a number this test hardcodes.
#[test]
fn a_divided_page_carries_the_regions_the_cut_made() {
    let runs = regions_and_x("two-column-14-lines");
    assert!(!runs.is_empty(), "the fixture must produce runs");

    let mut xs_by_region: std::collections::BTreeMap<u32, Vec<i64>> = Default::default();
    for (region, x) in &runs {
        let r = region.expect("a page the cut divided gives every run a region");
        xs_by_region.entry(r).or_default().push(*x);
    }
    assert_eq!(
        xs_by_region.len(),
        2,
        "a two-column page is two regions, not {}: {xs_by_region:?}",
        xs_by_region.len()
    );

    // Column-major: every run of the first region sits left of every run of the second. This is
    // what a mutant emitting one constant region cannot satisfy.
    let (first, second) = (&xs_by_region[&1], &xs_by_region[&2]);
    assert!(
        first.iter().max() < second.iter().min(),
        "region 1 must be the left column and region 2 the right: {xs_by_region:?}"
    );
}

/// **A page the cut did not divide says so by absence** (D4-S2).
///
/// The other direction, and it is not redundant — though not for the reason first written here.
/// **Three mutants were run against both tests, and only the third separates them:**
///
/// | mutant | `a_divided_page…` | this test |
/// | --- | --- | --- |
/// | `region: None` — the one that survived the whole suite | **fails** | passes |
/// | `region: Some(1)` | **fails** | **fails** |
/// | `region: run.region.or(Some(1))` | passes | **fails** |
///
/// This comment claimed the second row was the discriminating case; it is not, because one
/// constant region collapses a two-column page to a single region and the divided test catches
/// that on its own. The third is the real one: a region correct wherever the cut divided and
/// invented everywhere else. **That is the mutant this test exists for**, and it is the shape a
/// plausible bug would actually take.
///
/// `16-D4-SCOPE.md` §4 is explicit that absent means *"the cut made no division here"*, so a region
/// on an undivided page is a claim the rule never made — `reading_order.rs` says the same at
/// `Regions::per_run`: *"One region means no division."*
#[test]
fn an_undivided_page_carries_no_region() {
    let runs = regions_and_x("markdown-two-blocks");
    assert!(!runs.is_empty(), "the fixture must produce runs");
    let present: Vec<_> = runs.iter().filter(|(r, _)| r.is_some()).collect();
    assert!(
        present.is_empty(),
        "an undivided page must carry no region at all, found {present:?}"
    );
}

// -------------------------------------------------------------------------------------------
// The block cut: the two hops that carry `block` to the wire
// -------------------------------------------------------------------------------------------

/// Every run's `block` from the extract, in reading order, with the run's id and text.
fn extract_blocks(fixture: &str) -> Vec<(ethos_parser_core::NodeId, String, Option<u32>)> {
    let a = extract_ok(engine_fx(fixture));
    runs(&a)
        .iter()
        .map(|r| (r.id.clone(), r.text.clone(), r.block))
        .collect()
}

/// Every run's `block` from the emitted representation, with its node id, in node order.
fn representation_blocks(fixture: &str) -> Vec<(ethos_parser_core::NodeId, Option<u32>)> {
    let a = extract_ok(engine_fx(fixture));
    let rep = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    rep.payload()
        .nodes
        .iter()
        .filter_map(|n| match &n.attributes {
            ethos_parser_core::NodeAttributes::TextRun(t) => Some((n.id.clone(), t.block)),
            _ => None,
        })
        .collect()
}

/// The partition `leading-gap-two-blocks` is authored to produce: three runs, then three.
const LEADING_GAP_BLOCKS: [Option<u32>; 6] = [Some(1), Some(1), Some(1), Some(2), Some(2), Some(2)];

/// **A wide gap opens a second block, and the extract says which runs are in it** — the first
/// hop, `extract.rs`'s call to `crate::blocks::subdivide`.
///
/// `leading-gap-two-blocks` is authored against `blocks.rs`'s own constants: six single-run
/// lines at baselines 2000, 3400, 4800, 7600, 9000 and 10400 centipoints, so the band's modal
/// leading is 1400, the threshold is 8/5 × 1400 = 2240, and the one 2800 gap is the only cut. The
/// expected partition is arithmetic the fixture states rather than a number this test hardcodes.
///
/// Both lists are asserted. Deleting the `subdivide` call, or zipping its result against the
/// wrong list, leaves every `block` `None`; the texts pin that the reading order is the top-to-
/// bottom order the partition is stated in, so a block numbered on the wrong axis cannot pass by
/// coincidence.
#[test]
fn a_wide_gap_opens_a_second_block_and_the_extract_places_three_runs_in_each() {
    let got = extract_blocks("leading-gap-two-blocks");
    let texts: Vec<&str> = got.iter().map(|(_, t, _)| t.as_str()).collect();
    assert_eq!(
        texts,
        vec![
            "Water finds its level",
            "and stone keeps its shape",
            "through the long season",
            "Wind moves the grass",
            "and light moves the shade",
            "across the open field",
        ],
        "six single-run lines, top to bottom"
    );
    let blocks: Vec<Option<u32>> = got.iter().map(|(_, _, b)| *b).collect();
    assert_eq!(
        blocks,
        LEADING_GAP_BLOCKS.to_vec(),
        "the 2800 gap between the third and fourth baselines is the one cut: {got:?}"
    );
}

/// **The representation carries the block the extract set, on every run** — the second hop,
/// `represent.rs`'s `block: run.block`.
///
/// One production line, and until this test nothing observed it. `region`'s hop was found the
/// same way at D4-S2 (`a_divided_page_carries_the_regions_the_cut_made` above): replacing that
/// expression with `None` compiled and passed the entire workspace suite. So both halves are
/// asserted here — the value on the wire equals the extract's for the same node id, and those
/// values are the fixture's own partition.
///
/// **Five mutants were run against the three block tests, and each is caught:**
///
/// | mutant | extract test | this test | declined test |
/// | --- | --- | --- | --- |
/// | `represent.rs` `block: None` | passes | **fails** | passes |
/// | `represent.rs` `block: Some(1)` | passes | **fails** | **fails** |
/// | `represent.rs` `block: run.region` — a copy of the line above it, `None` on a single-column page | passes | **fails** | passes |
/// | `represent.rs` `block: run.block.or(Some(1))` | passes | passes | **fails** |
/// | `extract.rs` `run.block` never assigned | **fails** | **fails** | passes |
///
/// The second half of this test is what catches the fifth row: with nothing assigned, the wire
/// equals the extract at `None` on every run, and only the partition check says that is wrong.
#[test]
fn the_representation_carries_the_block_the_extract_set_on_every_run() {
    let from_extract = extract_blocks("leading-gap-two-blocks");
    let on_wire = representation_blocks("leading-gap-two-blocks");
    assert_eq!(on_wire.len(), from_extract.len(), "one node per run");
    for ((id, text, extracted), (node, wired)) in from_extract.iter().zip(&on_wire) {
        assert_eq!(id, node, "the node list is the run list, in order");
        assert_eq!(
            wired, extracted,
            "{id:?} ({text:?}) must reach the wire with the block the cut gave it"
        );
    }
    let blocks: Vec<Option<u32>> = on_wire.iter().map(|(_, b)| *b).collect();
    assert_eq!(
        blocks,
        LEADING_GAP_BLOCKS.to_vec(),
        "and those are the fixture's own two blocks, not a constant"
    );
}

/// **A page the rule declined carries no block on either hop, and that is the common case.**
///
/// `markdown-two-blocks` is two runs, 60 points apart, on a page the vertical cut does not divide.
/// Two lines are one gap; that gap is its own modal leading, and a gap never clears 1.6 times
/// itself, so `subdivide` returns the empty vector — the region contract — and `block` stays
/// `None` on both runs, through `extract.rs`'s `zip`, which does no work on an empty list, and
/// through `represent.rs`. This is the mutant the two positive tests cannot see — the fourth row
/// of the table above: `block: run.block.or(Some(1))` is right wherever the rule cut and invented
/// everywhere else, which is the shape a plausible bug takes, and measured, it fails only here.
/// `TextRunAttributes::block`'s rustdoc says absence is the ordinary state, not a corner; this is
/// the test that keeps it one.
#[test]
fn a_page_the_rule_declined_carries_no_block_on_either_hop() {
    let from_extract = extract_blocks("markdown-two-blocks");
    assert_eq!(from_extract.len(), 2, "the fixture is two runs");
    assert!(
        from_extract.iter().all(|(_, _, b)| b.is_none()),
        "two lines are one gap, which cannot clear 1.6x itself: {from_extract:?}"
    );
    let on_wire = representation_blocks("markdown-two-blocks");
    assert_eq!(on_wire.len(), 2, "one node per run");
    assert!(
        on_wire.iter().all(|(_, b)| b.is_none()),
        "absent on the wire too — not `Some(1)` for a page with one block: {on_wire:?}"
    );
}

// -------------------------------------------------------------------------------------------
// A page lopdf would read in part, or panic on, is refused by name
// -------------------------------------------------------------------------------------------

/// How the replacement content stream is written: as it stands, or through a deflater that
/// returns the stream bytes — whole, cut, or with a wrong check.
type Deflater = fn(&[u8]) -> Vec<u8>;

/// `measured-ink-box`'s one page with its content replaced: `content` as one stream, deflated
/// when `deflate` returns the stream bytes to write.
fn with_content(content: &[u8], deflate: Option<Deflater>) -> Vec<u8> {
    let original = std::fs::read(engine_fx("measured-ink-box")).expect("fixture readable");
    let mut doc = lopdf::Document::load_mem(&original).expect("lopdf loads");
    let page = doc.get_pages()[&1];
    let mut dict = lopdf::Dictionary::new();
    let bytes = match deflate {
        Some(deflate) => {
            dict.set("Filter", "FlateDecode");
            deflate(content)
        }
        None => content.to_vec(),
    };
    let stream = doc.add_object(lopdf::Stream::new(dict, bytes));
    doc.get_dictionary_mut(page)
        .expect("the page")
        .set("Contents", lopdf::Object::Reference(stream));
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

const MEASURED: &[u8] = b"BT /F1 24 Tf 72 72 Td (Measured) Tj ET\n";

fn zlib(bytes: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(bytes).expect("in memory");
    z.finish().expect("in memory")
}

fn extracted(bytes: &[u8]) -> Result<ExtractArtifact, ethos_parser_core::EngineError> {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile).expect("the document opens");
    ethos_parser_pdf::extract(&doc, &profile)
}

/// **An inline image `lopdf` panics on is refused by name, and `classify` survives it.**
/// `lopdf` 0.44.0's inline-image parser unwraps a colour-space lookup, so an image that names
/// neither `/CS` nor `/IM true` panics inside `Content::decode`, and the release profile's
/// `panic = "abort"` would end the process — an MCP server with it. The tokeniser refuses the
/// shape before `lopdf` sees it.
#[test]
fn an_inline_image_lopdf_panics_on_is_refused_by_name() {
    let mut content = MEASURED.to_vec();
    content.extend_from_slice(b"q BI /W 8 /H 1 /BPC 1 ID \xaa EI Q\n");
    let bytes = with_content(&content, None);

    let e = extracted(&bytes).expect_err("refused, not read and not a panic");
    assert_eq!(e.code(), "unsupported", "{e}");
    let message = e.to_string();
    assert!(
        message.starts_with("unsupported content stream tokeniser: page 1: byte "),
        "{message}"
    );
    assert!(
        message.contains("neither a colour space nor /IM true"),
        "{message}"
    );

    let profile = Profile::default();
    let doc = Document::open_bytes(&bytes, &profile).expect("opens");
    let c = ethos_parser_pdf::classify(&doc, &profile).expect("classify answers");
    assert_eq!(
        c.pages[0].text_operators, 0,
        "an unreadable content stream yields zero tallies, as a decode failure always has"
    );
}

/// **A tail `lopdf` would drop is refused by name.** Its decoder stops at the first operation it
/// cannot parse and returns what came before as the whole page: here the extract would carry
/// `Measured` and never `lost`, and nothing on the artifact would say a word was missing.
#[test]
fn a_tail_lopdf_would_drop_is_refused_by_name() {
    let mut content = MEASURED.to_vec();
    content.extend_from_slice(b") BT /F1 24 Tf 72 144 Td (lost) Tj ET\n");
    let lenient = lopdf::content::Content::decode(&content).expect("lopdf decodes leniently");
    assert_eq!(
        lenient.operations.len(),
        5,
        "lopdf keeps the first line's five operations and drops the second line without a word"
    );

    let e = extracted(&with_content(&content, None)).expect_err("refused, not read in part");
    assert_eq!(e.code(), "unsupported", "{e}");
    let message = e.to_string();
    assert!(
        message.starts_with("unsupported content stream tokeniser: page 1: byte "),
        "{message}"
    );
    assert!(message.contains("drops the remaining"), "{message}");
}

/// **A content stream `lopdf` did not load is refused by name.** Its lenient loader drops an
/// object whose bytes do not parse, and the page loop skipped the reference that no longer
/// resolved: one stray `)` in the stream's dictionary read as a blank page, `complete`. A
/// reference the cross-reference table never listed is null (§7.3.10), and still draws nothing.
/// Since review 2026-09-26 N08 the open refuses the lost object, before the page loop reads it.
#[test]
fn a_content_stream_lopdf_did_not_load_is_refused_by_name() {
    let page = |contents: &str, stream_dict: &str| {
        pdf_from_objects(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents {contents} >>")
                .into_bytes(),
            format!("<< /Length 3 {stream_dict}>>\nstream\nq Q\nendstream").into_bytes(),
        ])
    };
    let dropped = page("4 0 R", "/X ) ");
    assert!(
        lopdf::Document::load_mem(&dropped)
            .expect("lopdf loads the rest")
            .get_object((4, 0))
            .is_err(),
        "the control: lopdf dropped the stream at load, without a word"
    );

    let e = Document::open_bytes(&dropped, &Profile::default())
        .expect_err("refused, not read as a blank page");
    assert_eq!(e.code(), "malformed", "{e}");
    assert!(
        e.to_string()
            .starts_with("malformed pdf object: object 4 0 R is in use"),
        "{e}"
    );

    extracted(&page("[4 0 R 5 0 R]", "")).expect("an object nothing defines is null");
}

/// **A `/Contents` entry the table does not list at that generation, or an explicit `null`, draws
/// nothing** (review 2026-09-26 N63). PDF 32000-1 §7.3.10 reads both as null, and v0.61.0 read
/// both; the check above compared object numbers alone and did not take a loaded `null` for one,
/// so it refused each page as unreadable.
#[test]
fn a_contents_entry_that_is_null_draws_nothing() {
    for other in ["4 1 R", "5 0 R"] {
        let bytes = pdf_from_objects(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Resources << /Font \
                 << /F1 6 0 R >> >> /Contents [4 0 R {other}] >>"
            )
            .into_bytes(),
            [
                format!("<< /Length {} >>\nstream\n", MEASURED.len()).as_bytes(),
                MEASURED,
                b"endstream",
            ]
            .concat(),
            b"null".to_vec(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        ]);
        let a = extracted(&bytes).unwrap_or_else(|e| panic!("{other}: {e}"));
        assert_eq!(
            runs(&a).iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            ["Measured"],
            "{other}"
        );
    }
}

/// **An object `lopdf` did not load is refused at open, whatever would read it** (review
/// 2026-09-26 N08). The check above guarded `/Contents` alone: an annotation or a `/ToUnicode` lost
/// to one stray `)` vanished without a word, and an image whose `/Length` names no object loaded
/// with no data — each in an artifact marked `complete`, and in what `tag` and `overlay` wrote.
#[test]
fn an_object_lopdf_did_not_load_is_refused_at_open() {
    let stream = |dict: &str, data: &str| {
        format!(
            "<< /Length {} {dict}>>\nstream\n{data}\nendstream",
            data.len()
        )
        .into_bytes()
    };
    let cases: [(&str, Vec<Vec<u8>>, &str); 3] = [
        (
            "/Annots [4 0 R]",
            vec![
                b"<< /Type /Annot /Subtype /Text /Rect [0 0 9 9] /Contents (note) /X ) >>".to_vec(),
            ],
            "object 4 0 R is in use in the cross-reference table and did not load",
        ),
        (
            "/Resources << /Font << /F1 4 0 R >> >> /Contents 6 0 R",
            vec![
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 5 0 R >>".to_vec(),
                stream("/X ) ", "begincmap endcmap"),
                stream("", "BT /F1 24 Tf 72 72 Td (ABC) Tj ET"),
            ],
            "object 5 0 R is in use in the cross-reference table and did not load",
        ),
        (
            "/Resources << /XObject << /Im1 4 0 R >> >> /Contents 5 0 R",
            vec![
                b"<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
                  /BitsPerComponent 8 /Length 99 0 R >>\nstream\n\x80\nendstream"
                    .to_vec(),
                stream("", "q 9 0 0 9 0 0 cm /Im1 Do Q"),
            ],
            "object 4 0 R is a stream whose /Length does not resolve",
        ),
    ];
    for (page, rest, refusal) in cases {
        let mut objects = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] {page} >>").into_bytes(),
        ];
        objects.extend(rest);
        let bytes = pdf_from_objects(&objects);
        lopdf::Document::load_mem(&bytes).expect("the control: lopdf loads it without a word");

        let e = Document::open_bytes(&bytes, &Profile::default()).expect_err(refusal);
        assert_eq!(e.code(), "malformed", "{e}");
        assert!(e.to_string().contains(refusal), "{e}");
    }
}

/// **A `FlateDecode` stream cut short, or corrupt, is refused by name**, where `lopdf` inflates
/// what came before the damage and returns it as a success.
#[test]
fn a_flate_stream_that_does_not_reach_its_end_is_refused_by_name() {
    let whole = zlib(MEASURED);
    let cut: Deflater = |bytes| {
        let z = zlib(bytes);
        z[..z.len() / 2].to_vec()
    };
    assert!(whole.len() > 10, "the stream is long enough to cut");

    let e = extracted(&with_content(MEASURED, Some(cut))).expect_err("refused, not read in part");
    assert_eq!(e.code(), "malformed", "{e}");
    let message = e.to_string();
    assert!(
        message.starts_with("malformed content stream: page 1, stream "),
        "{message}"
    );
    assert!(message.contains("without reaching its end"), "{message}");

    // The control: the same page deflated whole reads exactly as the uncompressed page does.
    let texts = |a: &ExtractArtifact| runs(a).iter().map(|r| r.text.clone()).collect::<Vec<_>>();
    let plain = extracted(&with_content(MEASURED, None)).expect("plain reads");
    let deflated = extracted(&with_content(MEASURED, Some(zlib))).expect("deflated reads");
    assert_eq!(texts(&deflated), texts(&plain));
    assert_eq!(texts(&plain), ["Measured"]);
}

/// **A wrong check over whole deflate data still reads.** `lopdf` returns the whole page for it,
/// which is why its inflater tolerates it, so refusing it would refuse a page nothing was lost
/// from. The writer, which re-serialises the stream, is the stricter of the two.
#[test]
fn a_whole_flate_stream_with_a_wrong_check_still_reads() {
    let wrong_check: Deflater = |bytes| {
        let mut z = zlib(bytes);
        let last = z.len() - 1;
        z[last] ^= 0xff;
        z
    };
    let a = extracted(&with_content(MEASURED, Some(wrong_check))).expect("reads");
    assert_eq!(
        runs(&a).iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["Measured"]
    );
}

/// `with_content(stream, None)` with `/Filter` set to `filter` over the stream's bytes as written.
fn under(filter: impl Into<lopdf::Object>, stream: &[u8]) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(&with_content(stream, None)).expect("loads");
    let id = doc.get_page_contents(doc.get_pages()[&1])[0];
    doc.get_object_mut(id)
        .and_then(lopdf::Object::as_stream_mut)
        .expect("the page's stream")
        .dict
        .set("Filter", filter);
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// **A content stream whose filter does not decode is refused by name**, where `lopdf`'s
/// `get_page_content` fell back to the stream's raw bytes: operators under `/ASCIIHexDecode`, a
/// standard filter `lopdf` does not implement, or under a name no standard defines, read as text
/// no viewer draws. A filter that decodes still reads: the whole-deflate control above.
#[test]
fn a_content_stream_whose_filter_does_not_decode_is_refused_by_name() {
    for filter in ["ASCIIHexDecode", "NoSuchDecode"] {
        let e = extracted(&under(filter, MEASURED)).expect_err("refused, not read as raw bytes");
        assert_eq!(e.code(), "unsupported", "{e}");
        assert!(
            e.to_string()
                .contains(&format!("the filter chain [/{filter}] did not decode")),
            "{e}"
        );
    }
}

/// ASCII85 (PDF 32000-1 §7.4.3), closed by its `~>` marker.
fn ascii85(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        let mut n = u32::from_be_bytes(word);
        let mut digits = [0u8; 5];
        for d in digits.iter_mut().rev() {
            *d = b'!' + (n % 85) as u8;
            n /= 85;
        }
        out.extend_from_slice(&digits[..=chunk.len()]);
    }
    out.extend_from_slice(b"~>");
    out
}

/// **`FlateDecode` after another filter is checked as a first one is** (review 2026-09-26 N23).
/// `lopdf` returns a truncated inflate's partial output as a success wherever the filter sits, and
/// only a first filter's deflate data was checked: a page under `[/ASCII85Decode /FlateDecode]`
/// whose deflate data stops at a flush point read as the text before it, `complete`. The same
/// chain whole still reads.
#[test]
fn a_flate_stream_after_another_filter_that_does_not_reach_its_end_is_refused() {
    use std::io::Write;
    // `Measured`, flushed, and nothing after: whole blocks that decode to the line, and no end.
    let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    z.write_all(MEASURED).expect("in memory");
    z.flush().expect("in memory");
    let cut = z.get_ref().clone();
    let chain = || lopdf::Object::Array(vec!["ASCII85Decode".into(), "FlateDecode".into()]);

    let e = extracted(&under(chain(), &ascii85(&cut))).expect_err("refused, not read in part");
    assert_eq!(e.code(), "malformed", "{e}");
    assert!(e.to_string().contains("without reaching its end"), "{e}");

    let whole = extracted(&under(chain(), &ascii85(&zlib(MEASURED)))).expect("reads");
    assert_eq!(
        runs(&whole)
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>(),
        ["Measured"]
    );
}

/// **An empty filter array is no filter** (review 2026-09-26 N09): the stream's own bytes are the
/// page, as Ghostscript and qpdf read them. `lopdf` decodes an empty chain to no bytes at all, and
/// the page read as blank and `complete`.
#[test]
fn an_empty_filter_array_reads_the_streams_own_bytes() {
    let a = extracted(&under(lopdf::Object::Array(Vec::new()), MEASURED)).expect("reads");
    assert_eq!(
        runs(&a).iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["Measured"]
    );
}

// -------------------------------------------------------------------------------------------
// A page tree that is not a tree is refused, not counted
// -------------------------------------------------------------------------------------------

/// **A `/Kids` array that names a page twice, names its own node, or names a kid of no type is
/// refused.** `lopdf`'s `get_pages` keeps no visited set and skips a kid it cannot place, so the
/// first two read as two pages of one, and the third as none — each a page count an artifact
/// stated as `complete`.
#[test]
fn a_page_tree_that_is_not_a_tree_is_refused() {
    for (kids, page_type, refusal, lopdf_pages) in [
        (
            "[3 0 R 3 0 R]",
            "/Type /Page",
            "object 3 0 R is reached twice",
            2,
        ),
        (
            "[3 0 R 2 0 R]",
            "/Type /Page",
            "object 2 0 R is reached twice",
            2,
        ),
        ("[3 0 R]", "", "kid 3 0 R names no /Type", 0),
    ] {
        let bytes = pdf_from_objects(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            format!("<< /Type /Pages /Kids {kids} /Count 1 >>").into_bytes(),
            format!("<< {page_type} /Parent 2 0 R /MediaBox [0 0 300 144] >>").into_bytes(),
        ]);
        let lenient = lopdf::Document::load_mem(&bytes).expect("lopdf loads it");
        assert_eq!(
            lenient.get_pages().len(),
            lopdf_pages,
            "the control, for {kids}"
        );

        let e = Document::open_bytes(&bytes, &Profile::default()).expect_err(refusal);
        assert_eq!(e.code(), "malformed", "{e}");
        assert!(e.to_string().contains(refusal), "{e}");
    }
}

/// **A `/Pages` tree reads as deep as `get_pages` reads it** (review 2026-09-26 N64). The walk
/// bounded levels where `get_pages` bounds pending sibling lists, so a legal 300-level chain, which
/// v0.61.0, Ghostscript and qpdf each read as one page, was refused. The bound now sits where
/// `get_pages` skips a node without a word, and nowhere else.
#[test]
fn a_page_tree_reads_as_deep_as_get_pages_reads_it() {
    // `levels` nested /Pages nodes and a page at the bottom. Each of the first `siblings` nodes
    // also holds a page after the node below it, pending while the walk descends.
    let tree = |levels: usize, siblings: usize| {
        let mut objects = vec![b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()];
        for n in 2..levels + 2 {
            let sibling = if n - 1 <= siblings {
                format!(" {} 0 R", levels + 1 + n)
            } else {
                String::new()
            };
            objects.push(format!("<< /Type /Pages /Kids [{} 0 R{sibling}] >>", n + 1).into());
        }
        for parent in std::iter::once(levels + 1).chain(2..siblings + 2) {
            let page = format!("<< /Type /Page /Parent {parent} 0 R /MediaBox [0 0 300 144] >>");
            objects.push(page.into());
        }
        pdf_from_objects(&objects)
    };
    for (levels, siblings, reads) in [(300, 0, true), (257, 256, true), (258, 256, false)] {
        let bytes = tree(levels, siblings);
        let pages = siblings + 1;
        let lenient = lopdf::Document::load_mem(&bytes).expect("lopdf loads it");
        assert_eq!(
            lenient.get_pages().len() == pages,
            reads,
            "the control, {levels}"
        );

        match Document::open_bytes(&bytes, &Profile::default()) {
            Ok(doc) if reads => assert_eq!(doc.page_count() as usize, pages),
            Err(e) if !reads => assert!(e.to_string().contains("nests past 256 levels"), "{e}"),
            other => panic!("{levels} levels, {siblings} pending: {other:?}"),
        }
    }
}

// -------------------------------------------------------------------------------------------
// A document the empty user password opened says so (decision #31)
// -------------------------------------------------------------------------------------------

/// `measured-ink-box` encrypted RC4-40 (`/V 1 /R 2`) with an owner password and the given user
/// password. With an empty user password `lopdf` authenticates, decrypts and strips `/Encrypt`
/// at load, which is the shape the declaration is for; with a secret it stays locked.
fn encrypted_fixture(user_password: &str) -> Vec<u8> {
    let original = std::fs::read(engine_fx("measured-ink-box")).expect("fixture readable");
    let mut doc = lopdf::Document::load_mem(&original).expect("lopdf loads");
    // The security handler's key derivation reads the first `/ID` string (PDF 32000-1 §7.6.3.3,
    // Algorithm 2), and this fixture's trailer carries none. A fixed one keeps these bytes the
    // same on every run.
    let id = lopdf::Object::String(
        b"0123456789abcdef".to_vec(),
        lopdf::StringFormat::Hexadecimal,
    );
    doc.trailer.set("ID", vec![id.clone(), id]);
    let state = lopdf::EncryptionState::try_from(lopdf::EncryptionVersion::V1 {
        document: &doc,
        owner_password: "owner",
        user_password,
        permissions: lopdf::Permissions::default(),
    })
    .expect("an RC4-40 encryption state");
    doc.encrypt(&state)
        .expect("encrypts every string and stream");
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

fn codes(a: &ExtractArtifact) -> Vec<String> {
    a.assurance
        .limitations
        .iter()
        .map(|l| l.code.clone())
        .collect()
}

/// **An artifact from a document the empty user password opened declares it.** `lopdf`
/// authenticates the empty password, decrypts every object and removes `/Encrypt` before the
/// engine's encryption check runs, so such a document read exactly as a plaintext one and nothing
/// on the artifact said its bytes were ciphertext (`docs/25-KNOBS-SCOPE.md` §3.2, proposal 1).
/// Nothing is withheld, so it is a statement about the source rather than a gap.
#[test]
fn a_document_the_empty_user_password_opened_declares_it() {
    let bytes = encrypted_fixture("");
    let loaded = lopdf::Document::load_mem(&bytes).expect("lopdf loads it");
    assert!(
        loaded.was_encrypted() && !loaded.is_encrypted(),
        "the control: these bytes are ciphertext, and lopdf has already decrypted them"
    );

    let profile = Profile::default();
    let doc = Document::open_bytes(&bytes, &profile).expect("the empty password opened it");
    let a = ethos_parser_pdf::extract(&doc, &profile).expect("and it reads");
    assert!(
        codes(&a)
            .contains(&ethos_parser_pdf::limitations::ENCRYPTED_EMPTY_USER_PASSWORD.to_string()),
        "{:?}",
        codes(&a)
    );
    assert_eq!(
        runs(&a).iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        ["Measured"],
        "and it reads what the plaintext fixture reads — nothing was withheld"
    );

    let c = ethos_parser_pdf::classify(&doc, &profile).expect("classify answers");
    assert!(
        c.assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_pdf::limitations::ENCRYPTED_EMPTY_USER_PASSWORD),
        "classify declares it too, as it declares a repaired open"
    );

    // The two controls: a document with a user password is still refused, and a plaintext
    // document declares nothing.
    let locked = Document::open_bytes(&encrypted_fixture("secret"), &profile)
        .expect_err("a user password is still a refusal");
    assert_eq!(locked.code(), "encrypted", "{locked}");
    let plain = extract_ok(engine_fx("measured-ink-box"));
    assert!(
        !codes(&plain)
            .contains(&ethos_parser_pdf::limitations::ENCRYPTED_EMPTY_USER_PASSWORD.to_string()),
        "{:?}",
        codes(&plain)
    );
}

/// **An encrypted open whose object streams nest is refused.** `lopdf` 0.44.0's encrypted loader
/// fills object streams in `HashMap` order, so where one object stream is stored in another —
/// which PDF 32000-1 §7.5.7 forbids — which object a number resolves to depended on the run: a
/// 1,157-byte file built that way gave two different artifacts over twenty. The cross-reference
/// stream appended here lists object `n + 1` in object stream `n + 2`, and `n + 2` in a third. The
/// fixture's own objects are untouched, so only the refusal stands between it and an artifact.
#[test]
fn an_encrypted_open_whose_object_streams_nest_is_refused() {
    let mut bytes = encrypted_fixture("");
    let loaded = lopdf::Document::load_mem(&bytes).expect("lopdf loads it");
    let root = loaded
        .trailer
        .get(b"Root")
        .and_then(lopdf::Object::as_reference)
        .expect("a /Root");
    let encrypt = loaded
        .encryption_state
        .as_ref()
        .and_then(lopdf::EncryptionState::encrypt_object_id)
        .expect("an /Encrypt dictionary");
    let n = loaded.max_id + 1;
    // Two `/W [1 4 1]` rows of type 2, each naming its container.
    let rows: Vec<u8> = [n + 2, n + 3]
        .iter()
        .flat_map(|container| [&[2][..], &container.to_be_bytes(), &[0]].concat())
        .collect();
    let at = bytes.len();
    // The `/ID` is the one `encrypted_fixture` fixes, which the key derivation reads.
    let dict = format!(
        "<< /Type /XRef /Size {} /Index [{} 2] /W [1 4 1] /Root {} {} R /Encrypt {} {} R \
         /ID [(0123456789abcdef) (0123456789abcdef)] /Prev {} /Length {} >>",
        n + 3,
        n + 1,
        root.0,
        root.1,
        encrypt.0,
        encrypt.1,
        loaded.xref_start,
        rows.len()
    );
    bytes.extend_from_slice(format!("{n} 0 obj\n{dict}\nstream\n").as_bytes());
    bytes.extend_from_slice(&rows);
    bytes.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{at}\n%%EOF\n").as_bytes());

    let e = Document::open_bytes(&bytes, &Profile::default())
        .expect_err("refused, not read in hash order");
    assert_eq!(e.code(), "malformed", "{e}");
    assert!(
        e.to_string().contains(&format!(
            "object {} is stored in object stream {}",
            n + 1,
            n + 2
        )),
        "{e}"
    );
}

// -------------------------------------------------------------------------------------------
// A dropped run still advances the pen (doc 22 amendments, OPEN-WORK §6)
// -------------------------------------------------------------------------------------------

/// `broken-font-encoding` with one text object showing `first` then `Kept`, and its
/// `/Differences` remapping code 65 — `A`, inside `/Widths`, so its width is declared while its
/// glyph name resolves to no character. The fixture's own codes 200 to 202 are outside
/// `/FirstChar`..`/LastChar`, so no width is declared for them and the pen could not travel over
/// them even in principle; this is the case where it can.
fn undecodable_then_kept(first: &[u8]) -> Vec<u8> {
    let original = std::fs::read(engine_fx("broken-font-encoding")).expect("fixture readable");
    let mut doc = lopdf::Document::load_mem(&original).expect("lopdf loads");
    let font = doc
        .objects
        .iter()
        .find(|(_, o)| matches!(o, lopdf::Object::Dictionary(d) if d.has(b"BaseFont")))
        .map(|(id, _)| *id)
        .expect("the fixture has a font");
    let encoding = lopdf::Dictionary::from_iter([
        ("Type", lopdf::Object::Name(b"Encoding".to_vec())),
        (
            "BaseEncoding",
            lopdf::Object::Name(b"WinAnsiEncoding".to_vec()),
        ),
        (
            "Differences",
            lopdf::Object::Array(vec![
                lopdf::Object::Integer(65),
                lopdf::Object::Name(b"nonexistentglyphone".to_vec()),
            ]),
        ),
    ]);
    doc.get_dictionary_mut(font)
        .expect("the font dictionary")
        .set("Encoding", encoding);

    let mut content = b"BT /F1 24 Tf 72 100 Td (".to_vec();
    content.extend_from_slice(first);
    content.extend_from_slice(b") Tj (Kept) Tj ET\n");
    let page = doc.get_pages()[&1];
    let stream = doc.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), content));
    doc.get_dictionary_mut(page)
        .expect("the page")
        .set("Contents", lopdf::Object::Reference(stream));
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// **A run dropped at an undecodable code still advances the pen**, so a later run in the same
/// text object sits where the document draws it.
///
/// The pen used to stop at the refused code, and every run after it in that text object was
/// reported its width to the left. Measured against an independent reader: Ghostscript 10.06
/// renders `Kept` in the two documents below at exactly the same place — ink from 85.92pt in
/// both, at 300 dpi — because a glyph name that resolves to no character is still a glyph whose
/// width the font declares. The engine put it at 72pt, left of any ink on the page.
#[test]
fn a_dropped_run_still_advances_the_pen() {
    let kept_box = |bytes: &[u8]| -> (String, Vec<i64>) {
        let a = extracted(bytes).expect("reads");
        let run = runs(&a)
            .into_iter()
            .find(|r| r.text == "Kept")
            .expect("the second string is kept");
        let box_of = match &run.geometry {
            GeometryPresence::Measured(r) => vec![r.x0(), r.y0(), r.x1(), r.y1()],
            GeometryPresence::Absent(reason) => panic!("`Kept` must be measured: {reason:?}"),
        };
        (
            a.assurance
                .limitations
                .iter()
                .filter(|l| l.code == ethos_parser_pdf::limitations::BROKEN_FONT_ENCODING)
                .count()
                .to_string(),
            box_of,
        )
    };

    // `B` decodes through WinAnsi; `A` is the remapped code and is dropped.
    let (declared_control, control) = kept_box(&undecodable_then_kept(b"B"));
    let (declared_dropped, dropped) = kept_box(&undecodable_then_kept(b"A"));

    assert_eq!(declared_control, "0", "the control drops nothing");
    assert_eq!(
        declared_dropped, "1",
        "and the other declares its dropped run"
    );
    assert_eq!(
        dropped, control,
        "`Kept` must sit where the document draws it, dropped run or not"
    );
    // The pen travelled one 24pt glyph at 500/1000 em: 12 points, 1 200 centipoints.
    assert_eq!(dropped[0], 8400, "72pt + 12pt, in centipoints: {dropped:?}");
}

// -------------------------------------------------------------------------------------------
// An empty `/ToUnicode` destination, and what `scalar_code_mismatch` can and cannot say
// -------------------------------------------------------------------------------------------

/// `measured-ink-box` with a `/ToUnicode` mapping code 65 to nothing and code 66 to `fi`, and
/// one text object showing both.
fn empty_destination_and_ligature() -> Vec<u8> {
    let cmap = b"/CIDInit /ProcSet findresource begin\n\
        12 dict begin\n\
        begincmap\n\
        /CMapName /EthosEmptyDestination def\n\
        /CMapType 2 def\n\
        1 begincodespacerange\n\
        <41> <42>\n\
        endcodespacerange\n\
        2 beginbfchar\n\
        <41> <>\n\
        <42> <00660069>\n\
        endbfchar\n\
        endcmap\n\
        CMapName currentdict /CMap defineresource pop\n\
        end\n\
        end";
    let original = std::fs::read(engine_fx("measured-ink-box")).expect("fixture readable");
    let mut doc = lopdf::Document::load_mem(&original).expect("lopdf loads");
    let font = doc
        .objects
        .iter()
        .find(|(_, o)| matches!(o, lopdf::Object::Dictionary(d) if d.has(b"BaseFont")))
        .map(|(id, _)| *id)
        .expect("the fixture has a font");
    let stream = doc.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), cmap.to_vec()));
    doc.get_dictionary_mut(font)
        .expect("the font dictionary")
        .set("ToUnicode", lopdf::Object::Reference(stream));

    let page = doc.get_pages()[&1];
    let content = doc.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        b"BT /F1 24 Tf 72 100 Td (AB) Tj ET\n".to_vec(),
    ));
    doc.get_dictionary_mut(page)
        .expect("the page")
        .set("Contents", lopdf::Object::Reference(content));
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("saves");
    out
}

/// **An empty destination offsets a ligature, and `scalar_code_mismatch` reads false.**
///
/// The flag is a comparison of two counts, by its own definition in the contract, not a mapping
/// from codes to characters: here two codes produce two characters — one of them none, the other
/// two — and the counts agree while the run is not 1:1. Decided by measurement on 2026-09-18
/// rather than refused or re-defined: across 311 PDFs, 40,339 `bfchar` entries and 2,806
/// `bfrange` rows, **no destination is empty**, while 698 destinations in 95 documents carry
/// several scalars. Refusing an empty destination would drop a whole run for a code the document
/// chose to give no text. This test exists so the case is visible rather than discovered.
#[test]
fn the_empty_destination_offsets_a_ligature() {
    let a = extracted(&empty_destination_and_ligature()).expect("reads");
    let run = runs(&a);
    assert_eq!(run.len(), 1, "one string, one run");
    assert_eq!(
        run[0].text, "fi",
        "code 65 gives nothing, code 66 gives `fi`"
    );
    assert_eq!(
        run[0].char_codes,
        vec![65, 66],
        "and both codes are on the artifact, so a reader can see the run is not 1:1"
    );
    assert!(
        !run[0].scalar_code_mismatch,
        "two codes, two characters: the counts agree and the flag cannot see the offset"
    );
}

// -------------------------------------------------------------------------------------------
// Right-to-left text: what this engine does with it, measured (B9)
// -------------------------------------------------------------------------------------------

/// **Right-to-left text is reported in the order the page draws it, and nothing is reordered.**
///
/// `rtl-hebrew-visual-order` draws the four letters of `שלום` — ש ל ו ם in logical order, read
/// right to left — as `<0004000300020001>`, which is the order their glyphs sit on the page for a
/// producer whose layout engine has already resolved bidi. Every such producer writes it this way,
/// and this is what the engine says about it: one run, the codes in the page's order, and the text
/// the logical word reversed.
///
/// **This is a statement, not a defect.** The engine reports what the document draws; applying the
/// Unicode bidi algorithm would reorder characters the document did not, and a reordered string is
/// not what any byte of the page says. The consequence is real and belongs to the consumer: a
/// quote copied out of a viewer matches this run, and a quote typed in logical order does not.
/// **The artifact declares that as of 2026-09-23** — `right-to-left-not-reordered`, asserted
/// below, which is the owner decision `docs/OPEN-WORK.md` §4 held open until then.
#[test]
fn right_to_left_text_is_reported_in_the_order_the_page_draws_it() {
    let a = extract_ok(engine_fx("rtl-hebrew-visual-order"));
    let run = runs(&a);
    assert_eq!(run.len(), 1, "one string, one run");

    let logical = "\u{5E9}\u{5DC}\u{5D5}\u{5DD}";
    assert_eq!(
        run[0].text,
        logical.chars().rev().collect::<String>(),
        "the text is the logical word reversed, because that is the order the page draws it"
    );
    assert_eq!(
        run[0].char_codes,
        vec![4, 3, 2, 1],
        "and the codes say which glyphs those were, in the same order"
    );
    assert!(
        !run[0].scalar_code_mismatch,
        "four codes, four scalars: nothing about right-to-left text sets this flag"
    );

    // The four glyphs are 500 glyph units each at 12 pt: 6 pt, 2 400 centipoints in total, and
    // the box grows to the RIGHT from the origin, as it does for left-to-right text — the page's
    // own geometry, not a reading direction this engine inferred.
    assert_eq!(run[0].locator.advance, Some(2400));
    match &run[0].geometry {
        GeometryPresence::Measured(r) => {
            assert_eq!(r.x0(), run[0].locator.origin_x);
            assert_eq!(r.x1() - r.x0(), 2400);
        }
        GeometryPresence::Absent(reason) => panic!("the fixture has metrics: {reason:?}"),
    }

    // And the artifact says so. Before this code the consequence above lived only in
    // `CAPABILITY.md` and in this test — true, and unreadable by anything consuming the artifact.
    let declared: Vec<_> = a
        .assurance
        .limitations
        .iter()
        .filter(|l| l.code == ethos_parser_core::codes::RIGHT_TO_LEFT_NOT_REORDERED)
        .collect();
    assert_eq!(declared.len(), 1, "declared once: {declared:?}");
    assert_eq!(
        declared[0].scope,
        ethos_parser_core::LimitationScope::Document,
        "document-scoped: the condition is measurable, so it is a fact about THIS document"
    );
    assert!(
        declared[0].detail.starts_with("1 run(s)"),
        "the count is runs, and this fixture has one: {}",
        declared[0].detail
    );
}

/// **A char offset counts SCALARS — not UTF-8 bytes, not UTF-16 units — and nothing is normalised.**
///
/// `scalar-units-non-bmp` is the first document in either corpus that can tell those apart. Its one
/// run draws three CIDs: U+1D11E MUSICAL SYMBOL G CLEF, then `e`, then COMBINING ACUTE ACCENT. That
/// is **3 scalars, 4 UTF-16 units and 7 UTF-8 bytes** — three different numbers, which is the whole
/// reason the fixture exists. `scalar_code_mismatch` compares the scalar count against the code
/// count, so a reader counting units or bytes reports a mismatch on this run and a reader counting
/// scalars does not.
///
/// The combining sequence is the second half. `e` + U+0301 is canonically equivalent to the single
/// scalar U+00E9, so any reader applying NFC turns three scalars into two — which is what
/// `locate`'s no-normalisation rule forbids and what `26-LOCATE-SCOPE.md` §9 could hold only with
/// hand-built structs until this document existed.
#[test]
fn a_char_offset_counts_scalars_and_nothing_is_normalised() {
    let a = extract_ok(engine_fx("scalar-units-non-bmp"));
    let run = runs(&a);
    assert_eq!(run.len(), 1, "one string, one run");

    // Built from its scalars rather than written as a literal, so the three counts below are
    // read off the same characters this names.
    let expected: String = ['\u{1D11E}', 'e', '\u{301}'].iter().collect();
    assert_eq!(
        run[0].text, expected,
        "the three scalars the /ToUnicode map names"
    );

    // The three counts that differ, stated as the numbers they are.
    assert_eq!(run[0].text.chars().count(), 3, "scalars");
    assert_eq!(
        run[0].text.encode_utf16().count(),
        4,
        "UTF-16 units — the clef is a surrogate pair"
    );
    assert_eq!(run[0].text.len(), 7, "UTF-8 bytes");

    assert_eq!(
        run[0].char_codes,
        vec![1, 2, 3],
        "one code per scalar, in the page's order"
    );
    assert!(
        !run[0].scalar_code_mismatch,
        "three codes and three SCALARS agree. A reader counting UTF-16 units would see 4 against \
         3 here and set this flag, which is what makes this fixture discriminate"
    );

    // Not normalised: the combining sequence survives as two scalars, as the document wrote it.
    let composed = '\u{E9}';
    assert!(
        !run[0].text.contains(composed),
        "NFC would compose `e` + U+0301 into U+00E9 and turn three scalars into two: {:?}",
        run[0].text
    );
    assert_eq!(
        run[0].text.chars().nth(2),
        Some('\u{301}'),
        "the combining mark is still its own scalar"
    );
}

/// **A document that draws no right-to-left scalar declares nothing**, which is what makes the
/// declaration above worth reading. The negative control for the test above: without it, a code
/// that rode every artifact would pass that assertion just as well.
#[test]
fn a_document_without_right_to_left_text_declares_nothing_about_it() {
    let a = extract_ok(engine_fx("markdown-two-blocks"));
    assert!(
        !a.assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_core::codes::RIGHT_TO_LEFT_NOT_REORDERED),
        "no right-to-left scalar is drawn here, so nothing is declared about one"
    );
}

/// **An astral right-to-left scalar is declared too** (review 2026-09-26 N44). The block test read
/// three BMP ranges, so Adlam, Hanifi Rohingya, Imperial Aramaic or the Arabic mathematical
/// alphabet declared nothing, where the declaration's absence reads as *no right-to-left text*.
#[test]
fn an_astral_right_to_left_scalar_is_declared() {
    let stream = |data: &[u8]| {
        [
            format!("<< /Length {} >>\nstream\n", data.len()).as_bytes(),
            data,
            b"\nendstream",
        ]
        .concat()
    };
    let bytes = pdf_from_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Resources << /Font << /F1 5 0 R \
           >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream(b"BT /F1 24 Tf 72 72 Td (A) Tj ET"),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 6 0 R >>".to_vec(),
        // U+1E900, ADLAM CAPITAL LETTER ALIF, as its UTF-16 surrogate pair.
        stream(
            b"begincmap 1 begincodespacerange <00> <FF> endcodespacerange 1 beginbfchar \
              <41> <D83ADD00> endbfchar endcmap",
        ),
    ]);
    let a = extracted(&bytes).expect("reads");
    assert_eq!(runs(&a)[0].text, "\u{1E900}");
    assert!(
        a.assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_core::codes::RIGHT_TO_LEFT_NOT_REORDERED),
        "an Adlam run is right-to-left text drawn in page order, like a Hebrew one"
    );
}

// -------------------------------------------------------------------------------------------
// Outline titles
// -------------------------------------------------------------------------------------------

/// **An outline `/Title` held in an indirect object is read where it points** (review 2026-09-26
/// N17). PDF 32000-1 §7.3.10 lets any value be indirect, and the reader counted such a title as one
/// holding a byte it would not decode.
#[test]
fn an_indirect_outline_title_is_read() {
    let bytes = pdf_from_objects(&[
        b"<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] >>".to_vec(),
        b"<< /Type /Outlines /First 5 0 R /Last 5 0 R /Count 1 >>".to_vec(),
        b"<< /Title 6 0 R /Parent 4 0 R /Dest [3 0 R /Fit] >>".to_vec(),
        b"(Chapter one)".to_vec(),
    ]);
    let a = extracted(&bytes).expect("reads");
    assert_eq!(
        a.outlines
            .iter()
            .map(|o| o.title.as_deref())
            .collect::<Vec<_>>(),
        [Some("Chapter one")]
    );
}
