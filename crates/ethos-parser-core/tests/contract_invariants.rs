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

//! Contract invariants that are easiest to break with good intentions.
//!
//! Each of these corresponds to a rule in `docs/01-CONTRACT.md` that a reasonable engineer
//! would violate while trying to be helpful — adding a "diagnostic only" confidence field,
//! letting one float through "just for the ink box", returning `Option<QRect>` because it is
//! simpler. Prose in a doc does not stop that. A failing test does.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ethos_parser_core::{
    c14n_bytes, ArtifactBinding, ArtifactIdentity, Assurance, Capabilities, CoordinateSystem,
    CoverageSummary, DerivationClass, GeometryAbsence, GeometryPresence, IdAllocator, IdKind,
    Limitation, PageState, PageStateEntry, ProcessingGaps, ProcessingTerminalState, Profile, QRect,
    Sha256Hex,
};
use serde_json::Value;

// -------------------------------------------------------------------------------------------
// Source scanning
// -------------------------------------------------------------------------------------------

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file of `ethos-parser-core`, with the corpus asserted rather than assumed.
///
/// # Why the floor is here and not in each of the five guards
///
/// `no_confidence_anywhere_in_ethos_parser_core_code`, `no_other_quality_summary_vocabulary_leaks_in`,
/// `no_pdf_type_or_import_in_ethos_parser_core`, `no_verification_concept_in_ethos_parser_core` and
/// `floats_appear_only_inside_quantize` all reduce to this walk plus `assert!(hits.is_empty())`,
/// and **not one of them recorded how much it read.** `read_dir(...).unwrap_or_else(panic)`
/// catches a `src/` that vanished; it does not catch a `src/` that shrank, or modules moved under
/// a path the recursion stops reaching. Thirty-seven banned needles return an empty hit list on a
/// scan that read nothing exactly as they do on a scan that read everything, and five contract
/// invariants — the ones this file's header calls *"easiest to break with good intentions"* —
/// would all have printed `ok`.
///
/// This file already guards its *matchers*: `the_comment_stripper_actually_strips`,
/// `the_pdf_type_guard_matches_exactly_and_still_catches`, `the_float_exemption_does_not_cover_qrect`.
/// Nothing guarded its *corpus*. Putting the floor in the one function all five share is what
/// makes a sixth guard inherit it without anyone remembering to.
///
/// The numbers are just below the real ones at v2-S13.1: fifteen files and 14,787 lines.
fn ethos_parser_core_sources() -> Vec<PathBuf> {
    let files = rust_sources(&src_dir());
    let lines: usize = files
        .iter()
        .map(|p| {
            std::fs::read_to_string(p)
                .expect("readable source")
                .lines()
                .count()
        })
        .sum();
    assert!(
        files.len() >= 12,
        "the ethos-parser-core source walk found {} file(s); fifteen is the number at v2-S13.1. A \
         guard that reads nothing passes, so this is asserted before any of them run.",
        files.len()
    );
    assert!(
        lines >= 12_000,
        "the ethos-parser-core source walk found {lines} line(s); 14,787 is the number at v2-S13.1"
    );
    files
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
    for path in ethos_parser_core_sources() {
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
    for path in ethos_parser_core_sources() {
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
fn no_confidence_anywhere_in_ethos_parser_core_code() {
    let hits = code_hits("confidence");
    assert!(
        hits.is_empty(),
        "`confidence` appears in ethos-parser-core code, which docs/01-CONTRACT.md §9 forbids:\n  {}\n\n\
         Prose in comments is fine. A field, variant, parameter, or constant is not — not even \
         \"diagnostic only\". The v4 OCR lane may record processor-reported uncertainty, and \
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
            "`{banned}` appears in ethos-parser-core code; no single field may summarise document \
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
        // The canonical serializer must NAME f32/f64 in order to refuse them:
        // serde's Serializer trait fixes those two method signatures, and each
        // arm's whole body is the same "non-integer number" refusal the Value
        // route produces. A float still cannot reach canonical output — these
        // are precisely the lines that guarantee it. The exemption matches the
        // signatures alone, so a float USED anywhere in c14n.rs still fails here.
        if hit.starts_with("c14n.rs:")
            && (hit.contains("fn serialize_f32(") || hit.contains("fn serialize_f64("))
        {
            continue;
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

/// No PDF **dependency** in `ethos-parser-core` (`docs/04-ARCHITECTURE.md` §1).
///
/// The boundary that matters is the dependency graph, not vocabulary. `ethos-parser-core` must be
/// compilable with no PDF crate in sight, so that a second format is a variant rather than a
/// rewrite.
#[test]
fn ethos_parser_core_has_no_pdf_dependency() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("readable manifest");
    for banned in ["lopdf", "pdfium", "ttf-parser", "pdf-rs", "printpdf"] {
        assert!(
            !manifest.contains(banned),
            "ethos-parser-core declares a PDF dependency `{banned}`. Format-specific work belongs in \
             ethos-parser-pdf (docs/04-ARCHITECTURE.md §1)."
        );
    }
}

/// No PDF **type or import** in `ethos-parser-core`.
///
/// The line this test draws, stated precisely, because it is easy to draw in the wrong place:
///
/// - **Banned:** importing PDF machinery, or naming a Rust item after PDF structure. Those make
///   `ethos-parser-core` know about one format, and the second format becomes a rewrite.
/// - **Allowed:** a *string value* that happens to name something PDF-ish. `BackendIdentity`'s
///   `"lopdf"` exists so a backend swap moves `profile_sha256`. `EngineError::MissingPart {
///   part: "xref table" }` is the format-agnostic taxonomy being *used* — `part` is a `String`,
///   and ethos-parser-pdf will pass PDF words into it precisely because the taxonomy does not know or
///   care what a format calls its parts.
///
/// The distinction is types and imports versus data. A test that banned the vocabulary outright
/// would forbid the error taxonomy from ever being demonstrated, which would make it worse
/// documented without making it more format-agnostic.
///
/// **Banned type names are matched exactly, not by prefix.** The prefix form (`"struct Page"`)
/// read `PageStateEntry` as a PDF page-tree type and failed M4's coverage summary — a type the
/// contract *requires* in `ethos-parser-core`, since `01-CONTRACT.md` §7 mandates per-page processing
/// state for every format. A page is a universal document concept; `lopdf::Page` is a PDF one.
/// The guard has to be able to tell those apart or it forbids the contract it exists to protect.
#[test]
fn no_pdf_type_or_import_in_ethos_parser_core() {
    let mut offenders = Vec::new();
    for banned in [
        "use lopdf",
        "content_stream",
        "page_tree",
        "acroform",
        "font_descriptor",
        "text_matrix",
    ] {
        offenders.extend(code_hits(banned));
    }
    // Type names that would mean this crate had learned about PDF structure, matched as whole
    // identifiers so a format-agnostic `PageState` is not read as a page tree.
    for banned in ["Page", "TextRun", "ContentStream", "Xref", "FontDescriptor"] {
        for keyword in ["struct", "enum", "type"] {
            offenders.extend(declared_item_hits(keyword, banned));
        }
    }
    assert!(
        offenders.is_empty(),
        "a PDF type or import appears in ethos-parser-core:\n  {}",
        offenders.join("\n  ")
    );
}

/// Hits where `<keyword> <name>` declares an item named **exactly** `name`.
///
/// The character after the name must not be an identifier character, so `struct Page` matches
/// `struct Page { … }` and `struct Page;` but not `struct PageStateEntry`.
fn declared_item_hits(keyword: &str, name: &str) -> Vec<String> {
    let needle = format!("{keyword} {name}");
    code_hits(&needle)
        .into_iter()
        .filter(|hit| {
            // `code_hits` returns "file.rs:line: text"; re-find the needle in the text.
            let Some(at) = hit.find(&needle) else {
                return false;
            };
            let after = &hit[at + needle.len()..];
            !after
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
        })
        .collect()
}

/// The exact-match guard still catches what it is for.
///
/// Guards the guard: a prefix relaxation that let `struct Page` through would make the boundary
/// test decorative, and nobody would notice until a page tree was already in `ethos-parser-core`.
#[test]
fn the_pdf_type_guard_matches_exactly_and_still_catches() {
    let banned_shapes = ["struct Page {", "struct Page;", "enum Xref {"];
    for shape in banned_shapes {
        let hit = format!("some.rs:1: {shape}");
        let (keyword, name) = shape.split_once(' ').unwrap();
        let name = name.split(|c: char| !c.is_alphanumeric()).next().unwrap();
        let needle = format!("{keyword} {name}");
        let at = hit.find(&needle).expect("shape contains its own needle");
        let after = &hit[at + needle.len()..];
        assert!(
            !after
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric() || c == '_'),
            "`{shape}` must be recognised as a banned declaration"
        );
    }

    // And the shape that must NOT be caught.
    let allowed = "some.rs:1: pub struct PageStateEntry {";
    let at = allowed.find("struct Page").expect("prefix is present");
    let after = &allowed[at + "struct Page".len()..];
    assert!(
        after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_'),
        "PageStateEntry must not be read as a PDF page type"
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
fn no_verification_concept_in_ethos_parser_core() {
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
        "a verification concept appears in ethos-parser-core; the engine never verifies:\n  {}",
        offenders.join("\n  ")
    );
}

// -------------------------------------------------------------------------------------------
// Behavioural invariants
// -------------------------------------------------------------------------------------------

const DIGEST: &str = "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";

/// One representative value of every public artifact-ish type, as JSON.
/// A constructed sample of the public types that reach an artifact — **not every public type.**
///
/// # The number, because the four tests below say *every*
///
/// Seventeen values over fifteen distinct types, against **188** frozen crate-root exports of
/// `ethos-parser-core` (`crates/ethos-parser-cli/tests/public_api.rs`'s `CORE`) and 111 `pub struct`/`pub
/// enum` declarations in its sources. It is a sample and it cannot stop being one: the property
/// under test is about a *serialized value*, and most of those exports are functions, consts,
/// traits and error types that have no artifact-bearing value to construct. Deriving this list
/// is not available; what is available is saying the number out loud.
///
/// # What proves the universal claim, since this does not
///
/// [`floats_appear_only_inside_quantize`] scans every source line of `ethos-parser-core` and holds the
/// structural half for **all** types, not a sample: no float type exists outside `quantize`, so
/// no public type can carry one. The four tests below are the behavioural half — that the
/// canonicalizer really does refuse what the structure forbids — and a spot-check is the right
/// instrument for that. Nothing here is unverified; what was wrong is that four names said
/// *every* over fifteen, which is the damage v2-S12.1 recorded for
/// `every_profile_is_distinct_from_every_other`: the next reader adds a type, sees green, and
/// never learns the list is one a human has to remember to grow.
fn public_type_samples() -> Vec<(&'static str, Value)> {
    let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
    vec![
        (
            "ArtifactIdentity",
            serde_json::to_value(ArtifactIdentity {
                artifact_type: "ethos.parser.representation.v0".into(),
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
        (
            "Limitation",
            serde_json::to_value(Limitation::page(
                2,
                ethos_parser_core::codes::RESOURCE_LIMIT_PAGES,
                "a configured page budget stopped the run",
            ))
            .unwrap(),
        ),
        (
            "PageState::Processed",
            serde_json::to_value(PageState::Processed).unwrap(),
        ),
        (
            "PageState::Quarantined",
            serde_json::to_value(PageState::Quarantined(
                ethos_parser_core::codes::RESOURCE_LIMIT_PAGES.into(),
            ))
            .unwrap(),
        ),
        (
            "CoverageSummary",
            serde_json::to_value(
                CoverageSummary::from_page_states(
                    1,
                    &[PageStateEntry {
                        index: 1,
                        state: PageState::Processed,
                    }],
                )
                .unwrap(),
            )
            .unwrap(),
        ),
        (
            "ProcessingTerminalState::Partial",
            serde_json::to_value(ProcessingTerminalState::Partial(ProcessingGaps {
                pages_not_processed: 1,
                first_gap_page: 2,
            }))
            .unwrap(),
        ),
        (
            "Assurance",
            serde_json::to_value(
                Assurance::new(
                    Capabilities::V0,
                    1,
                    vec![PageStateEntry {
                        index: 1,
                        state: PageState::Processed,
                    }],
                    Vec::new(),
                )
                .unwrap(),
            )
            .unwrap(),
        ),
    ]
}

/// The sample, pinned and cross-checked against the types that actually exist.
///
/// Two failures this catches that the four tests below cannot: a sample silently deleted, and a
/// type renamed out from under its entry — after which the entry names nothing, the value it
/// builds is still canonicalized, and the reader believes a type is covered that no longer goes
/// by that name.
#[test]
fn the_public_type_sample_names_types_that_exist() {
    let samples = public_type_samples();
    assert_eq!(
        samples.len(),
        17,
        "{} sample(s); seventeen is the number at v2-S13.1, over fifteen distinct types",
        samples.len()
    );

    let mut sources = String::new();
    for path in ethos_parser_core_sources() {
        sources.push_str(&std::fs::read_to_string(&path).expect("readable source"));
    }
    assert!(
        sources.len() > 100_000,
        "the source scan read only {} bytes, so the check below would pass having read nothing",
        sources.len()
    );

    let mut distinct: BTreeSet<&str> = BTreeSet::new();
    for (name, _) in &samples {
        // `GeometryPresence::Measured` samples one variant of a type; the type is the part that
        // has to exist.
        let base = name.split("::").next().unwrap_or(name);
        distinct.insert(base);
        assert!(
            sources.contains(&format!("pub struct {base}"))
                || sources.contains(&format!("pub enum {base}")),
            "the sample names `{base}`, which `ethos-parser-core` no longer declares. A renamed type              leaves its entry building a value nobody can trace back to it."
        );
    }
    assert_eq!(
        distinct.len(),
        15,
        "{} distinct type(s) sampled; fifteen is the number at v2-S13.1",
        distinct.len()
    );
}

/// Every **sampled** public type canonicalizes — which means none of them contains a float.
///
/// *Every* type is held by [`floats_appear_only_inside_quantize`], structurally. This is the
/// behavioural half over the sample [`public_type_samples`] describes.
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

/// Serialize → canonicalize → parse → identical value, for every **sampled** public type.
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

/// Canonical output for every **sampled** public type is byte-stable across repeated runs.
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

/// No canonical output of a **sampled** public type contains a decimal point.
///
/// The name is `no_public_type_emits_a_decimal_number` and stays that way: it is a filter token in
/// the `v0-c14n` CI job. What it does not say on its own is that the sample is fifteen types, so
/// [`public_type_samples`] says it, and [`floats_appear_only_inside_quantize`] holds the claim the
/// name reads like.
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

/// `profile.draft.json`'s example is the **real** canonical profile.
///
/// The DRAFT schemas are documentation, not validated against the code by a test — a deliberate
/// decision recorded in `docs/draft-schemas/README.md`, because adding a JSON Schema validator to
/// prove a draft matches is more machinery than a draft warrants.
///
/// This is the one exception, and it is cheap: the README claims that example *is* the profile
/// `ethos-parser-core` emits, and by M4 it was not — it still carried `"unbound-until-m3"` placeholders
/// two milestones after the backend landed. A claim that specific either holds or should not be
/// made, and `serde_json` equality is enough to keep it holding.
#[test]
fn the_profile_schema_example_is_the_real_profile() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/draft-schemas/profile.draft.json");
    let schema: Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display())),
    )
    .expect("the draft schema is valid JSON");

    let example = schema["examples"][0].clone();
    assert!(
        !example.is_null(),
        "profile.draft.json must carry an example"
    );

    let real = serde_json::to_value(Profile::default()).unwrap();
    assert_eq!(
        example, real,
        "profile.draft.json's example has drifted from the profile ethos-parser-core emits. Update the \
         example — docs/draft-schemas/README.md tells readers it is the real one."
    );

    // And the required list covers every field the real profile carries, so a new knob cannot be
    // added to the type while the schema keeps describing the old shape.
    let required: Vec<&str> = schema["required"]
        .as_array()
        .expect("required is an array")
        .iter()
        .map(|v| v.as_str().expect("string"))
        .collect();
    for key in real.as_object().expect("profile is an object").keys() {
        assert!(
            required.contains(&key.as_str()),
            "`{key}` is on Profile and missing from profile.draft.json's required list"
        );
    }
}

/// The representation draft schema pins the shape version the code actually emits.
///
/// **Found stale while shipping v1-S3**: the schema still said `0.1.0` while the code had said
/// `0.2.0` since v1-S1. `docs/draft-schemas/README.md` tells readers these describe what the
/// engine emits, and a version constant that drifted is exactly the kind of wrong a reader cannot
/// see. The profile schema already had this guard; the representation one did not, which is why
/// only the profile stayed honest.
#[test]
fn the_representation_schema_pins_the_version_the_code_emits() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/draft-schemas/document-representation.draft.json");
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display())),
    )
    .expect("the draft schema is valid JSON");

    assert_eq!(
        schema["properties"]["schema_version"]["const"].as_str(),
        Some(ethos_parser_core::REPRESENTATION_SCHEMA_VERSION),
        "document-representation.draft.json pins a schema_version the code no longer emits. \
         Update the schema in the same commit that bumps the constant — docs/draft-schemas/\
         README.md tells readers this file describes what the engine actually produces."
    );
    assert_eq!(
        schema["properties"]["artifact_type"]["const"].as_str(),
        Some(ethos_parser_core::REPRESENTATION_ARTIFACT_TYPE),
        "and the artifact type with it"
    );
}

/// The markdown draft schema pins the shape version **and the rule id** the code actually emits.
///
/// The third of these guards, and the reason there is a third: the v1-S3 note above says the
/// representation schema drifted precisely because the profile one had a guard and it did not.
/// `markdown.draft.json` arrived at v1.1-S1 with neither, so its `schema_version` and its
/// `markdown_rule` example could both go stale silently — and `markdown_rule` is the one field on
/// this artifact whose whole job is to say which projection produced it.
///
/// The rule id is checked through `examples` rather than a `const` on purpose: the schema
/// describes the artifact *shape*, and an artifact produced by an older rule is still a valid
/// `ethos.markdown.v1`. What the README promises is that these files describe what the engine
/// emits, so the example has to be a rule the engine can actually emit today.
#[test]
fn the_markdown_schema_pins_the_version_and_rule_the_code_emits() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/draft-schemas/markdown.draft.json");
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display())),
    )
    .expect("the draft schema is valid JSON");

    assert_eq!(
        schema["properties"]["schema_version"]["const"].as_str(),
        Some(ethos_parser_core::MARKDOWN_SCHEMA_VERSION),
        "markdown.draft.json pins a schema_version the code no longer emits. Update the schema in \
         the same commit that bumps the constant — docs/draft-schemas/README.md tells readers this \
         file describes what the engine actually produces."
    );
    assert_eq!(
        schema["properties"]["artifact_type"]["const"].as_str(),
        Some(ethos_parser_core::MARKDOWN_ARTIFACT_TYPE),
        "and the artifact type with it"
    );

    let examples: Vec<&str> = schema["properties"]["markdown_rule"]["examples"]
        .as_array()
        .expect("markdown_rule carries examples")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert!(
        examples.contains(&ethos_parser_core::MARKDOWN_RULE_BLOCKS_V5),
        "markdown.draft.json's `markdown_rule` examples are {examples:?}, none of which is the \
         rule this build emits ({}). A reader takes the example as the current answer, and the id \
         moved at v1.1-S2 and again at v1.1-S3.",
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V5
    );

    // The example artifact is a whole document, so its own `markdown_rule` has to agree too — an
    // example that disagrees with the property beside it is worse than no example.
    if let Some(example) = schema["examples"].as_array().and_then(|a| a.first()) {
        assert_eq!(
            example["markdown_rule"].as_str(),
            Some(ethos_parser_core::MARKDOWN_RULE_BLOCKS_V5),
            "the worked example names a different rule than the schema's own property does"
        );
        assert_eq!(
            example["artifact_type"].as_str(),
            Some(ethos_parser_core::MARKDOWN_ARTIFACT_TYPE)
        );
        assert_eq!(
            example["schema_version"].as_str(),
            Some(ethos_parser_core::MARKDOWN_SCHEMA_VERSION)
        );
    }
}

/// The html draft schema pins the shape version and the rule id the code actually emits.
///
/// The fourth of these guards, added **in the slice that adds the schema** rather than two slices
/// later — which is the rule v1.1-S3 wrote down after finding the representation schema had
/// drifted for exactly that reason. `docs/draft-schemas/README.md` states it: a schema without a
/// guard drifts, and the drift is invisible to the reader the file is for.
#[test]
fn the_html_schema_pins_the_version_and_rule_the_code_emits() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/draft-schemas/html.draft.json");
    let schema: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&path).unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display())),
    )
    .expect("the draft schema is valid JSON");

    assert_eq!(
        schema["properties"]["schema_version"]["const"].as_str(),
        Some(ethos_parser_core::HTML_SCHEMA_VERSION),
        "html.draft.json pins a schema_version the code no longer emits. Update the schema in the \
         same commit that bumps the constant — docs/draft-schemas/README.md tells readers this \
         file describes what the engine actually produces."
    );
    assert_eq!(
        schema["properties"]["artifact_type"]["const"].as_str(),
        Some(ethos_parser_core::HTML_ARTIFACT_TYPE),
        "and the artifact type with it"
    );

    let examples: Vec<&str> = schema["properties"]["html_rule"]["examples"]
        .as_array()
        .expect("html_rule carries examples")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert!(
        examples.contains(&ethos_parser_core::HTML_RULE_BLOCKS_V5),
        "html.draft.json's `html_rule` examples are {examples:?}, none of which is the rule this \
         build emits ({}).",
        ethos_parser_core::HTML_RULE_BLOCKS_V5
    );

    if let Some(example) = schema["examples"].as_array().and_then(|a| a.first()) {
        assert_eq!(
            example["html_rule"].as_str(),
            Some(ethos_parser_core::HTML_RULE_BLOCKS_V5),
            "the worked example names a different rule than the schema's own property does"
        );
        assert_eq!(
            example["artifact_type"].as_str(),
            Some(ethos_parser_core::HTML_ARTIFACT_TYPE)
        );
        assert_eq!(
            example["schema_version"].as_str(),
            Some(ethos_parser_core::HTML_SCHEMA_VERSION)
        );
    }

    // **The two projections must not share a rule id.** One id covering both would make every
    // artifact non-comparable each time either projection moved, which is the opposite of what a
    // rule id is for.
    assert_ne!(
        ethos_parser_core::HTML_RULE_BLOCKS_V5,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V5
    );
}
