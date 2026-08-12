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

//! Contract invariants that are easiest to break with good intentions.
//!
//! Each of these corresponds to a rule in `docs/01-CONTRACT.md` that a reasonable engineer
//! would violate while trying to be helpful — adding a "diagnostic only" confidence field,
//! letting one float through "just for the ink box", returning `Option<QRect>` because it is
//! simpler. Prose in a doc does not stop that. A failing test does.

use std::path::{Path, PathBuf};

use engine_core::{
    c14n_bytes, ArtifactBinding, ArtifactIdentity, Capabilities, CoordinateSystem, DerivationClass,
    GeometryAbsence, GeometryPresence, IdAllocator, IdKind, Profile, QRect, Sha256Hex,
};
use serde_json::Value;

// -------------------------------------------------------------------------------------------
// Source scanning
// -------------------------------------------------------------------------------------------

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir:?}: {e}"));
    for entry in entries {
        let path = entry.expect("readable dir entry").path();
        if path.is_dir() {
            out.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Strip `//`-style comments so prose about a banned token is not mistaken for the token.
///
/// Deliberately simple: it does not parse Rust. It handles line comments and skips string
/// literals containing `//`. Block comments are handled by a crude depth counter. If this ever
/// misjudges, it errs toward *reporting* a hit — a false alarm is cheap, a missed violation is
/// the whole point of the test.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut block_depth = 0usize;
    for line in src.lines() {
        let mut code = String::new();
        let bytes: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut in_string = false;
        while i < bytes.len() {
            let c = bytes[i];
            let next = bytes.get(i + 1).copied();
            if block_depth > 0 {
                if c == '*' && next == Some('/') {
                    block_depth -= 1;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }
            if in_string {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                code.push(c);
                i += 1;
                continue;
            }
            if c == '"' {
                in_string = true;
                code.push(c);
                i += 1;
                continue;
            }
            if c == '/' && next == Some('/') {
                break; // line comment: rest of the line is prose
            }
            if c == '/' && next == Some('*') {
                block_depth += 1;
                i += 2;
                continue;
            }
            code.push(c);
            i += 1;
        }
        out.push_str(&code);
        out.push('\n');
    }
    out
}

/// Non-comment lines where `needle` appears as a **whole token**, not a substring.
///
/// Substring matching was a false-positive machine: a sha256 digest containing `...cf32c2e...`
/// tripped the float ban, because `f32` is three hex digits as readily as it is a type name. A
/// guard that cries wolf on a hash is a guard someone will eventually disable.
fn token_hits(needle: &str) -> Vec<String> {
    fn is_ident(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
    let mut hits = Vec::new();
    for path in rust_sources(&src_dir()) {
        let src = std::fs::read_to_string(&path).expect("readable source");
        for (n, line) in strip_comments(&src).lines().enumerate() {
            let mut from = 0;
            while let Some(rel) = line[from..].find(needle) {
                let start = from + rel;
                let end = start + needle.len();
                let before_ok =
                    start == 0 || !line[..start].chars().next_back().is_some_and(is_ident);
                let after_ok =
                    end >= line.len() || !line[end..].chars().next().is_some_and(is_ident);
                if before_ok && after_ok {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    hits.push(format!("{name}:{}: {}", n + 1, line.trim()));
                    break;
                }
                from = end;
            }
        }
    }
    hits
}

/// Non-comment lines mentioning `needle`, case-insensitively, as `path:line: text`.
fn code_hits(needle: &str) -> Vec<String> {
    let needle = needle.to_ascii_lowercase();
    let mut hits = Vec::new();
    for path in rust_sources(&src_dir()) {
        let src = std::fs::read_to_string(&path).expect("readable source");
        for (n, line) in strip_comments(&src).lines().enumerate() {
            if line.to_ascii_lowercase().contains(&needle) {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                hits.push(format!("{name}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    hits
}

#[test]
fn the_comment_stripper_actually_strips() {
    // Guard the guard: if this broke, the scans below would pass vacuously.
    let src = r#"
// confidence in a line comment
/// confidence in a doc comment
/* confidence in a block */
let x = 1; // confidence trailing
let s = "confidence in a string";
let real_confidence = 2;
"#;
    let stripped = strip_comments(src);
    assert!(!stripped.contains("in a line comment"));
    assert!(!stripped.contains("in a doc comment"));
    assert!(!stripped.contains("in a block"));
    assert!(!stripped.contains("trailing"));
    assert!(
        stripped.contains("real_confidence"),
        "code must survive stripping: {stripped}"
    );
    assert!(
        stripped.contains("confidence in a string"),
        "string literals are code and must survive"
    );
}

/// **No public confidence field, score, grade, or quality summary — anywhere.**
///
/// `docs/01-CONTRACT.md` §9, Workbench rule 9. The measured case for this rule was produced by
/// the exact feature under consideration: pdf-inspector reports `TEXT-BASED, Confidence: 50%,
/// Pages with text: 0` — a verdict its own evidence contradicts.
///
/// Prose in a doc comment is fine and expected (this crate argues the rule at length). What is
/// banned is the token appearing in *code*.
#[test]
fn no_confidence_anywhere_in_engine_core_code() {
    let hits = code_hits("confidence");
    assert!(
        hits.is_empty(),
        "`confidence` appears in engine-core code, which docs/01-CONTRACT.md §9 forbids:\n  {}\n\n\
         Prose in comments is fine. A field, variant, parameter, or constant is not — not even \
         \"diagnostic only\". The v2.1 OCR lane may record processor-reported uncertainty, and \
         when it does it goes on a recognition type under its own profile, never on these.",
        hits.join("\n  ")
    );
}

#[test]
fn no_other_quality_summary_vocabulary_leaks_in() {
    // The rule is about the concept, not one spelling. These are the words a well-meaning
    // change would reach for next.
    for banned in ["quality_score", "trust_score", "is_good", "grade"] {
        let hits = code_hits(banned);
        assert!(
            hits.is_empty(),
            "`{banned}` appears in engine-core code; no single field may summarise document \
             quality (docs/00-NORTH-STAR.md §7):\n  {}",
            hits.join("\n  ")
        );
    }
}

/// **No `f32`/`f64` in a serialized type.**
///
/// `quantize` legitimately *takes* an `f64` — that is the one permitted float on the canonical
/// path. What must never happen is a float reaching a struct that derives `Serialize`.
///
/// The exemption is scoped to `quantize`'s own body and the test module, **not** to all of
/// `geom.rs`. Exempting the whole file would be a hole big enough to drive `QRect` through: it
/// lives in `geom.rs`, derives `Serialize`, and a field changed to `f64` would have sailed past
/// a file-level skip.
#[test]
fn floats_appear_only_inside_quantize() {
    let exempt = quantize_and_test_line_ranges();
    let mut offenders = Vec::new();

    for hit in token_hits("f64").into_iter().chain(token_hits("f32")) {
        if let Some(rest) = hit.strip_prefix("geom.rs:") {
            let line: usize = rest
                .split(':')
                .next()
                .and_then(|n| n.parse().ok())
                .expect("hit carries a line number");
            if exempt.iter().any(|(lo, hi)| line >= *lo && line <= *hi) {
                continue;
            }
        }
        offenders.push(hit);
    }

    assert!(
        offenders.is_empty(),
        "floats appear outside `quantize`:\n  {}\n\n\
         Serialized types carry i64, enums, or strings. Floats do not exist in canonical \
         output (docs/01-CONTRACT.md §4).",
        offenders.join("\n  ")
    );
}

/// Line ranges in `geom.rs` where a float is legitimate: `quantize`'s body, and the tests that
/// exercise it. Computed by brace-depth scan rather than hardcoded, so edits do not silently
/// widen the exemption.
fn quantize_and_test_line_ranges() -> Vec<(usize, usize)> {
    let src = std::fs::read_to_string(src_dir().join("geom.rs")).expect("geom.rs is readable");
    let mut ranges = Vec::new();

    let mut start: Option<usize> = None;
    let mut depth = 0i32;
    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        if start.is_none() && line.contains("pub fn quantize(") {
            start = Some(n);
            depth = 0;
        }
        if let Some(s) = start {
            depth += line.matches('{').count() as i32;
            depth -= line.matches('}').count() as i32;
            if depth == 0 && n > s {
                ranges.push((s, n));
                start = None;
            }
        }
        if line.trim_start().starts_with("mod tests") {
            ranges.push((n, src.lines().count()));
        }
    }

    assert!(
        !ranges.is_empty(),
        "could not locate `quantize` in geom.rs — the exemption scan is broken, which would \
         make the float ban either vacuous or unusable"
    );
    ranges
}

/// The exemption must not cover `QRect`. Guards the guard: if the range scan ever swallowed the
/// whole file, this fails.
#[test]
fn the_float_exemption_does_not_cover_qrect() {
    let src = std::fs::read_to_string(src_dir().join("geom.rs")).expect("geom.rs is readable");
    let qrect_line = src
        .lines()
        .position(|l| l.contains("pub struct QRect"))
        .expect("QRect is defined in geom.rs")
        + 1;

    let exempt = quantize_and_test_line_ranges();
    assert!(
        !exempt
            .iter()
            .any(|(lo, hi)| qrect_line >= *lo && qrect_line <= *hi),
        "the float exemption covers QRect's definition at geom.rs:{qrect_line}, so a float \
         field there would go unnoticed. Exempt ranges: {exempt:?}"
    );
}

/// No PDF **dependency** in `engine-core` (`docs/04-ARCHITECTURE.md` §1).
///
/// The boundary that matters is the dependency graph, not vocabulary. `engine-core` must be
/// compilable with no PDF crate in sight, so that a second format is a variant rather than a
/// rewrite.
#[test]
fn engine_core_has_no_pdf_dependency() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("readable manifest");
    for banned in ["lopdf", "pdfium", "ttf-parser", "pdf-rs", "printpdf"] {
        assert!(
            !manifest.contains(banned),
            "engine-core declares a PDF dependency `{banned}`. Format-specific work belongs in \
             engine-pdf (docs/04-ARCHITECTURE.md §1)."
        );
    }
}

/// No PDF **type or import** in `engine-core`.
///
/// The line this test draws, stated precisely, because it is easy to draw in the wrong place:
///
/// - **Banned:** importing PDF machinery, or naming a Rust item after PDF structure. Those make
///   `engine-core` know about one format, and the second format becomes a rewrite.
/// - **Allowed:** a *string value* that happens to name something PDF-ish. `BackendIdentity`'s
///   `"lopdf"` exists so a backend swap moves `profile_sha256`. `EngineError::MissingPart {
///   part: "xref table" }` is the format-agnostic taxonomy being *used* — `part` is a `String`,
///   and engine-pdf will pass PDF words into it precisely because the taxonomy does not know or
///   care what a format calls its parts.
///
/// The distinction is types and imports versus data. A test that banned the vocabulary outright
/// would forbid the error taxonomy from ever being demonstrated, which would make it worse
/// documented without making it more format-agnostic.
#[test]
fn no_pdf_type_or_import_in_engine_core() {
    let mut offenders = Vec::new();
    for banned in [
        "use lopdf",
        "content_stream",
        "page_tree",
        "acroform",
        "font_descriptor",
        "text_matrix",
        "struct Page",
        "struct TextRun",
    ] {
        offenders.extend(code_hits(banned));
    }
    assert!(
        offenders.is_empty(),
        "a PDF type or import appears in engine-core:\n  {}",
        offenders.join("\n  ")
    );
}

/// The backend name is a declared string, and it is hashed.
///
/// Pins the intent behind the exclusion above: the name exists precisely so that adopting or
/// upgrading the backend moves `profile_sha256`.
#[test]
fn the_backend_name_is_declared_and_hash_bearing() {
    let base = Profile::default();
    let base_hash = base.profile_sha256().unwrap();
    assert_eq!(base.backend.name, "lopdf");

    let mut swapped = Profile::default();
    swapped.backend.name = "pdfium".into();
    assert_ne!(
        swapped.profile_sha256().unwrap(),
        base_hash,
        "a backend swap must be fingerprint-visible"
    );
}

/// No verification concept has leaked in (`docs/07-VERIFY-BOUNDARY.md`).
#[test]
fn no_verification_concept_in_engine_core() {
    let mut offenders = Vec::new();
    for banned in [
        "evidence_tier",
        "all_evidence_grounded",
        "is_grounded",
        "verdict",
    ] {
        offenders.extend(code_hits(banned));
    }
    assert!(
        offenders.is_empty(),
        "a verification concept appears in engine-core; the engine never verifies:\n  {}",
        offenders.join("\n  ")
    );
}

// -------------------------------------------------------------------------------------------
// Behavioural invariants
// -------------------------------------------------------------------------------------------

const DIGEST: &str = "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";

/// One representative value of every public artifact-ish type, as JSON.
fn public_type_samples() -> Vec<(&'static str, Value)> {
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
    vec![
        (
            "ArtifactIdentity",
            serde_json::to_value(ArtifactIdentity {
                artifact_type: "ethos.engine.representation.v0".into(),
                schema_version: "0.1.0".into(),
                parser_version: "0.0.0".into(),
                profile_sha256: Sha256Hex::parse(DIGEST).unwrap(),
            })
            .unwrap(),
        ),
        (
            "ArtifactBinding",
            serde_json::to_value(ArtifactBinding {
                source_sha256: Sha256Hex::parse(DIGEST).unwrap(),
                representation_sha256: Sha256Hex::parse(DIGEST).unwrap(),
            })
            .unwrap(),
        ),
        (
            "CoordinateSystem",
            serde_json::to_value(CoordinateSystem::V0).unwrap(),
        ),
        ("Profile", serde_json::to_value(Profile::default()).unwrap()),
        (
            "Capabilities",
            serde_json::to_value(Capabilities::V0).unwrap(),
        ),
        (
            "DerivationClass",
            serde_json::to_value(DerivationClass::Extracted).unwrap(),
        ),
        (
            "QRect",
            serde_json::to_value(QRect::new(1, 2, 3, 4).unwrap()).unwrap(),
        ),
        (
            "GeometryPresence::Measured",
            serde_json::to_value(GeometryPresence::Measured(QRect::new(1, 2, 3, 4).unwrap()))
                .unwrap(),
        ),
        (
            "GeometryPresence::Absent",
            serde_json::to_value(GeometryPresence::Absent(
                GeometryAbsence::NotReportedByReader,
            ))
            .unwrap(),
        ),
        (
            "Sha256Hex",
            serde_json::to_value(Sha256Hex::parse(DIGEST).unwrap()).unwrap(),
        ),
        (
            "NodeId",
            serde_json::to_value(alloc.next(IdKind::Element).unwrap()).unwrap(),
        ),
    ]
}

/// Every public type canonicalizes — which means none of them contains a float.
#[test]
fn every_public_type_survives_canonicalization() {
    for (name, value) in public_type_samples() {
        c14n_bytes(&value).unwrap_or_else(|e| {
            panic!(
                "{name} does not canonicalize: {e}. A float, or an \
                                        integer beyond 2^53-1, reached a serialized type."
            )
        });
    }
}

/// Serialize → canonicalize → parse → identical value, for every public type.
#[test]
fn every_public_type_round_trips_through_c14n() {
    for (name, value) in public_type_samples() {
        let bytes = c14n_bytes(&value).unwrap();
        let reparsed: Value = serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("{name} canonical bytes are not valid JSON: {e}"));
        assert_eq!(reparsed, value, "{name} did not round-trip");

        // And canonicalization is idempotent for it.
        let again = c14n_bytes(&reparsed).unwrap();
        assert_eq!(again, bytes, "{name} canonicalization is not idempotent");
    }
}

/// Canonical output for every public type is byte-stable across repeated runs.
#[test]
fn public_type_canonical_bytes_are_stable() {
    let first: Vec<Vec<u8>> = public_type_samples()
        .iter()
        .map(|(_, v)| c14n_bytes(v).unwrap())
        .collect();
    let second: Vec<Vec<u8>> = public_type_samples()
        .iter()
        .map(|(_, v)| c14n_bytes(v).unwrap())
        .collect();
    assert_eq!(first, second);
}

/// No canonical output of a public type contains a decimal point.
///
/// A blunt instrument, and that is the point: it catches a float that arrives as a string too.
#[test]
fn no_public_type_emits_a_decimal_number() {
    for (name, value) in public_type_samples() {
        let s = String::from_utf8(c14n_bytes(&value).unwrap()).unwrap();
        // Version strings legitimately contain dots ("0.1.0"), so only flag dots that sit
        // between digits outside a quoted string.
        let mut in_string = false;
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            match chars[i] {
                '"' if i == 0 || chars[i - 1] != '\\' => in_string = !in_string,
                '.' if !in_string => panic!(
                    "{name} emitted a bare decimal point at byte {i}, meaning a float reached \
                     canonical output: {s}"
                ),
                _ => {}
            }
        }
    }
}
