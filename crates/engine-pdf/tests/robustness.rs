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

//! Fixture mutation: every fixture in the manifest, mechanically damaged, six ways.
//!
//! `docs/03-V0-SCOPE.md` §5 requires mutation tests covering every fixture. This is the
//! Anydoc-style version of that — damage the *inputs* and assert the engine's response — not
//! source-level mutation, which asks a different question (are the assertions load-bearing?) and
//! answers it far more slowly.
//!
//! # What a mutant is allowed to do
//!
//! Exactly two things, and the suite asserts the boundary between them:
//!
//! | Outcome | Required |
//! | --- | --- |
//! | **Refused** | A named [`EngineError`] with one of the six taxonomy codes. No artifact, no partial output |
//! | **Read** | The artifact binds to the **mutant's** bytes — `source.sha256` is the mutant's digest, never the original's |
//!
//! What is never allowed is a third outcome: a panic, or a well-formed artifact that reads as
//! though the original document had been parsed. The first is a crash a caller cannot route; the
//! second is worse, because nothing downstream can tell it happened.
//!
//! # Survivors are pinned, not tolerated
//!
//! A mutant that still parses is not a failure — appending junk after `%%EOF` genuinely leaves a
//! readable document, and a byte flipped in an unreferenced region genuinely changes nothing the
//! parser reads. But *which* mutants survive is a property of the engine, so the set is pinned in
//! [`EXPECTED_SURVIVORS`]. A mutant that starts surviving is a fail-closed path that stopped
//! firing, and this suite makes that a red test rather than a quiet drift.
//!
//! # Panic containment
//!
//! Each case runs under `catch_unwind` so one panic names its own fixture and mutant instead of
//! taking down the run at the first bad byte. The release profile is `panic = "abort"`, so this
//! is a test-build affordance for triage, never a runtime strategy: a panic reaching a caller is
//! a release blocker either way (`docs/05-MILESTONES.md` M7).

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use engine_core::{EngineError, Profile};
use engine_pdf::Document;
use serde_json::Value;

// -------------------------------------------------------------------------------------------
// Manifest
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn manifest() -> Value {
    serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest readable"),
    )
    .expect("manifest is valid JSON")
}

fn root_dir(m: &Value, root_name: &str) -> PathBuf {
    let decl = &m["roots"][root_name];
    assert!(!decl.is_null(), "manifest declares no root `{root_name}`");
    match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("root declares a default")),
    }
}

/// One fixture, as the manifest describes it plus its bytes.
struct Fixture {
    id: String,
    root: String,
    bytes: Vec<u8>,
}

/// **Every** fixture the manifest declares, across all three roots.
///
/// Read from the manifest rather than from a list in this file, so a fixture added to the corpus
/// is mutated without anyone remembering to add it here. A missing file is a failure, never a
/// skip — the same rule the oracle harness runs under (`docs/04-ARCHITECTURE.md` §4).
fn all_fixtures() -> Vec<Fixture> {
    let m = manifest();
    let declared = m["fixtures"].as_array().expect("fixtures array");

    let out: Vec<Fixture> = declared
        .iter()
        .map(|f| {
            let id = f["id"].as_str().expect("fixture id").to_string();
            let root = f["root"].as_str().expect("fixture root").to_string();
            let rel = f["path"].as_str().expect("fixture path");
            let path = root_dir(&m, &root).join(rel);
            let bytes = std::fs::read(&path).unwrap_or_else(|e| {
                panic!(
                    "fixture `{id}` unreadable at {}: {e}\nA missing corpus is a failure, never \
                     a skip.",
                    path.display()
                )
            });
            Fixture { id, root, bytes }
        })
        .collect();

    // The manifest's own counts are the tripwire: if a fixture is added to the array without
    // updating `counts`, mutation coverage and the declared corpus size have silently diverged.
    let expected = m["counts"]["conformance_ethos_owned"].as_u64().unwrap()
        + m["counts"]["benchmark"].as_u64().unwrap()
        + m["counts"]["engine_owned"].as_u64().unwrap();
    assert_eq!(
        out.len() as u64,
        expected,
        "the manifest lists {} fixtures but its counts sum to {expected}",
        out.len()
    );
    out
}

// -------------------------------------------------------------------------------------------
// Mutations
// -------------------------------------------------------------------------------------------

/// The mechanical damage applied to every fixture.
///
/// Deterministic by construction — no RNG anywhere. A mutation suite whose inputs vary run to run
/// cannot sit in a repository whose central claim is byte identity, and a survivor set that
/// changed with the weather could not be pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mutation {
    /// Zero bytes.
    Empty,
    /// The first 16 bytes and nothing else — a header with no body.
    Truncate16,
    /// One byte inverted in the last tenth of the file, where the xref and trailer live.
    FlipTailByte,
    /// `%PDF` overwritten, so content-based detection has nothing to recognise.
    HeaderOverwritten,
    /// Trailing junk after `%%EOF`.
    JunkAfterEof,
    /// A known operator token replaced, in place, by one outside PDF 32000-1 Table A.1.
    UnknownOperator,
}

impl Mutation {
    const ALL: [Mutation; 6] = [
        Mutation::Empty,
        Mutation::Truncate16,
        Mutation::FlipTailByte,
        Mutation::HeaderOverwritten,
        Mutation::JunkAfterEof,
        Mutation::UnknownOperator,
    ];

    fn name(self) -> &'static str {
        match self {
            Mutation::Empty => "empty",
            Mutation::Truncate16 => "truncate-16",
            Mutation::FlipTailByte => "flip-tail-byte",
            Mutation::HeaderOverwritten => "header-overwritten",
            Mutation::JunkAfterEof => "junk-after-eof",
            Mutation::UnknownOperator => "unknown-operator",
        }
    }

    /// Apply the mutation, or return `None` where it does not apply to this input.
    ///
    /// `None` is a real answer, not a skip: `UnknownOperator` needs a plaintext content stream to
    /// edit, and a fixture whose streams are compressed has none. Those are counted and reported
    /// by `every_fixture_is_mutated_and_the_coverage_is_reported` rather than passing silently.
    fn apply(self, original: &[u8]) -> Option<Vec<u8>> {
        match self {
            Mutation::Empty => Some(Vec::new()),

            Mutation::Truncate16 => {
                if original.len() <= 16 {
                    return None;
                }
                Some(original[..16].to_vec())
            }

            Mutation::FlipTailByte => {
                if original.len() < 4 {
                    return None;
                }
                // The midpoint of the last tenth: deep enough to land in the xref/trailer region
                // on every fixture in the corpus, and a fixed formula rather than a magic index.
                // Verified rather than assumed — on the four fixtures inspected during triage it
                // lands on an xref digit, on `/Root`, on the object number inside `1 0 R`, and on
                // trailer whitespace respectively. Three of those four are load-bearing.
                let idx = original.len() - original.len() / 20 - 1;
                let mut out = original.to_vec();
                out[idx] ^= 0xFF;
                Some(out)
            }

            Mutation::HeaderOverwritten => {
                if original.len() < 8 {
                    return None;
                }
                let mut out = original.to_vec();
                out[..4].copy_from_slice(b"%XYZ");
                Some(out)
            }

            Mutation::JunkAfterEof => {
                let mut out = original.to_vec();
                out.extend_from_slice(b"\nthis is not part of any document\n");
                Some(out)
            }

            Mutation::UnknownOperator => {
                // Same-length substitution, so `/Length` stays honest and the document remains
                // structurally valid. That is the point: the parse must stop on the *operator*,
                // not on a stream whose declared length no longer matches.
                //
                // `Zq` is outside Table A.1. Preferring `Tj` puts the unknown token after font
                // selection and positioning, so the interpreter reaches it having done real work
                // — a fail-closed path that only fires before any state is built proves less.
                //
                // The token is matched **whitespace-delimited**, not as ` Tj `: half the corpus
                // writes operators at the end of a line (`(text) Tj\n`), and a space-delimited
                // search silently missed those fixtures entirely.
                //
                // **Compressed documents are excluded, and that is a triage finding.** Widening
                // the match made this mutation "apply" to the two NIST benchmarks — and it was
                // not applying at all: the hit in `nist-sp-800-63b` is at offset 301887, inside
                // a Flate stream, surrounded by binary. Overwriting it corrupts compressed data
                // and fails on *decompression*, so the test would have gone green while proving
                // nothing about operator handling. A mutation named `unknown-operator` must
                // inject an operator.
                if find(original, b"FlateDecode").is_some() {
                    return None;
                }
                for op in [b"Tj", b"Td", b"Tm", b"Tf"] {
                    if let Some(at) = find_operator(original, op) {
                        let mut out = original.to_vec();
                        out[at..at + 2].copy_from_slice(b"Zq");
                        return Some(out);
                    }
                }
                None
            }
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
        .filter(|_| !needle.is_empty())
}

/// The first offset where `op` appears as a whole content-stream token.
///
/// PDF separates operands from operators by whitespace, so requiring whitespace on both sides is
/// what makes this an operator rather than two letters inside a name or a string.
fn find_operator(haystack: &[u8], op: &[u8; 2]) -> Option<usize> {
    let ws = |b: u8| matches!(b, b' ' | b'\n' | b'\r' | b'\t');
    (1..haystack.len().saturating_sub(op.len())).find(|&i| {
        &haystack[i..i + 2] == op.as_slice() && ws(haystack[i - 1]) && ws(haystack[i + 2])
    })
}

// -------------------------------------------------------------------------------------------
// The outcome of one mutant
// -------------------------------------------------------------------------------------------

/// The six codes `EngineError::code()` can return. A refusal outside this set is not named.
const TAXONOMY: [&str; 6] = [
    "unsupported",
    "malformed",
    "encrypted",
    "resource_limit",
    "missing_part",
    "io",
];

/// What a mutant that was **read** produced.
///
/// Refusal is the `Err` arm of [`run_mutant`], carrying the [`EngineError`] itself, so there is
/// no second spelling of "refused" here that could drift from the taxonomy.
#[derive(Debug)]
struct Read {
    /// The digest the artifact bound itself to.
    bound_sha256: String,
}

/// Run a mutant all the way through, and describe what happened.
///
/// `extract`/`to_representation` run only where the caller asks, because a 492-page benchmark
/// document is minutes of debug-build extraction per mutant and adds no robustness signal the
/// other **fifty-two** small fixtures do not already give. Exactly three fixtures take the
/// shallow pass — the `benchmark` root — and which they are is reported, never silent.
fn run_mutant(bytes: &[u8], deep: bool) -> Result<Read, EngineError> {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile)?;

    let classification = engine_pdf::classify(&doc, &profile)?;
    let canonical = classification.to_canonical_bytes()?;
    let v: Value = serde_json::from_slice(&canonical).expect("canonical bytes are JSON");
    let bound = v["source"]["sha256"]
        .as_str()
        .expect("every artifact binds to a source digest")
        .to_string();

    if deep {
        let extract = engine_pdf::extract(&doc, &profile)?;
        let repr = engine_pdf::to_representation(&extract, &profile)?;
        repr.verify_fingerprint()?;
        let repr_bound = repr.payload().source.sha256.to_string();
        assert_eq!(
            repr_bound, bound,
            "classification and representation must bind to the same bytes"
        );
    }

    Ok(Read {
        bound_sha256: bound,
    })
}

/// `"<fixture id>/<mutation>"` for every mutant that still parses.
///
/// Pinned rather than merely permitted, and triaged into exactly two classes. Both are cases
/// where refusing would be *wrong*, which is why the right response is a pin and not a fix:
///
/// - **`junk-after-eof`, on every fixture that opens at all.** A PDF reader reaches the trailer
///   through `startxref`, so bytes appended past `%%EOF` sit outside every offset the document
///   declares. Nothing reads them, and the document really is intact. The safety property still
///   holds and is asserted separately: the artifact binds to the *mutant's* digest, so a consumer
///   comparing hashes sees a different document, which it is.
///
/// - **`flip-tail-byte` on nine documents.** Inspected during triage rather than assumed. On
///   `synthetic/two-lines` the flipped byte is the `t` of `/Root` in the trailer; on
///   `synthetic/two-columns` it is the `R` of `1 0 R`. `lopdf` recovers by scanning for the
///   catalog instead of trusting the damaged trailer reference, so a genuinely readable document
///   is read. The other **forty-six** fixtures, where this mutation lands on an xref digit or a
///   length, **do** fail closed — and that ratio is what makes this a backend-recovery
///   observation rather than a hole: the same mutation refuses five times more often than it
///   survives. (Four and eleven at M7, when the corpus was fifteen documents. The class did not
///   change as the corpus grew; only the counts did, and this sentence did not grow with them
///   until v2-S12.1.)
///
/// An entry appearing here that is not one of those two classes is a fail-closed path that
/// stopped firing — triage it before pinning it. An entry disappearing is a path that started
/// firing, which is usually good and still wants a commit message.
const EXPECTED_SURVIVORS: [&str; 60] = [
    "absent-font-metrics/junk-after-eof",
    // v1-S4's form and annotation fixtures. Same class as every other `junk-after-eof`: bytes
    // appended past `%%EOF` leave a readable document.
    "annotation-contents/junk-after-eof",
    "background-panel-not-a-grid/junk-after-eof",
    "both-table-rules/junk-after-eof",
    "broken-font-encoding/junk-after-eof",
    "crop-box-smaller-than-media/junk-after-eof",
    "failure/image-only-or-blank-page/junk-after-eof",
    "form-field-value/junk-after-eof",
    "form-orphan-widget/junk-after-eof",
    "form-xfa-stub/junk-after-eof",
    "failure/memory-limit-simulated/junk-after-eof",
    "foreign/opendataloader/real/flip-tail-byte",
    "foreign/opendataloader/real/junk-after-eof",
    "horizontal-scaling-tz/junk-after-eof",
    // v1-S6's four. All survive `junk-after-eof` and nothing else — the same answer every other
    // engine-authored fixture gives, because the parse is driven from the xref table
    // `startxref` names and bytes appended past `%%EOF` are never read.
    "image-declared-not-drawn/junk-after-eof",
    "image-xobject-drawn/junk-after-eof",
    "invisible-render-mode/junk-after-eof",
    "irs-form-1040-2025/flip-tail-byte",
    "irs-form-1040-2025/junk-after-eof",
    // v1.1-S1's Anchor Map golden, v1.1-S2's GFM one and v1.1-S3's hyphen one. Same class as
    // every other engine fixture: bytes appended past `%%EOF` leave a readable document, and each
    // was re-extracted to confirm it yields what it does unmutated rather than pinned on sight —
    // `markdown-hyphen-break` gives back the same two runs with the same MEASURED ink boxes, which
    // is what its golden depends on.
    "markdown-hyphen-break/junk-after-eof",
    "markdown-table-cells/junk-after-eof",
    "markdown-two-blocks/junk-after-eof",
    "measured-ink-box/junk-after-eof",
    "nist-sp-800-53r5/flip-tail-byte",
    "nist-sp-800-53r5/junk-after-eof",
    "nist-sp-800-63b/flip-tail-byte",
    "nist-sp-800-63b/junk-after-eof",
    "off-page-and-offset-box/junk-after-eof",
    "ruled-table-grid/junk-after-eof",
    "ruled-table-overlap/junk-after-eof",
    "ruled-wins-shared-region/junk-after-eof",
    "show-text-quote-operators/junk-after-eof",
    "simple-font-two-byte-tounicode/junk-after-eof",
    // v1-S8's three stroke-ruled fixtures. Same class again, and the class is the point: bytes
    // appended past `%%EOF` leave a readable document, so the mutant is not a fail-closed path
    // that stopped firing. Triaged rather than pinned on sight — each was re-extracted and yields
    // the same tables it does unmutated.
    "stroke-ruled-columns-not-drawn/junk-after-eof",
    "stroke-ruled-field-boxes/junk-after-eof",
    "stroke-ruled-worksheet/junk-after-eof",
    "synthesized-space-tj/junk-after-eof",
    "synthetic/heading-export/flip-tail-byte",
    "synthetic/heading-export/junk-after-eof",
    "synthetic/hyphenated-line-break/flip-tail-byte",
    "synthetic/hyphenated-line-break/junk-after-eof",
    "synthetic/ligature-fi-embedded-font/junk-after-eof",
    "synthetic/list-items/flip-tail-byte",
    "synthetic/list-items/junk-after-eof",
    "synthetic/rotation-90/junk-after-eof",
    "synthetic/simple-text/junk-after-eof",
    "synthetic/table-regular-grid/junk-after-eof",
    "synthetic/two-columns/flip-tail-byte",
    "synthetic/two-columns/junk-after-eof",
    "synthetic/two-lines/flip-tail-byte",
    "synthetic/two-lines/junk-after-eof",
    // v1.1-S2's tagged list, the only document in either corpus whose tree declares an `/L`.
    "tagged-list-items/junk-after-eof",
    // v1-S3's tagged fixtures. `tagged-cycle` is deliberately NOT here: its structure tree does
    // not terminate, so extraction refuses the mutant for the same reason it refuses the
    // original, and the mutant never survives.
    "tagged-rolemap/junk-after-eof",
    "tagged-structure-roles/junk-after-eof",
    "tagged-table-agrees/junk-after-eof",
    "tagged-table-disagrees/junk-after-eof",
    // v1-S5's anti-cliff pair. Both survive `junk-after-eof` and nothing else, which is the same
    // answer every other engine-authored fixture gives: the parse is driven from the xref table
    // `startxref` names, so bytes appended past `%%EOF` are never read. Triaged as the known
    // class rather than pinned on sight.
    "two-column-14-lines/junk-after-eof",
    "two-column-15-lines/junk-after-eof",
    "unruled-near-miss/junk-after-eof",
    "whitespace-past-the-page-edge/junk-after-eof",
];

// -------------------------------------------------------------------------------------------
// The suite
// -------------------------------------------------------------------------------------------

/// **No mutant panics, and every refusal is named.**
///
/// The load-bearing test. A panic here is a release blocker; an unnamed refusal is a caller that
/// cannot route the failure, which `docs/03-V0-SCOPE.md` §3.1 exists to prevent.
#[test]
fn no_mutant_panics_and_every_refusal_is_named() {
    let mut panics: Vec<String> = Vec::new();
    let mut unnamed: Vec<String> = Vec::new();
    let mut cases = 0usize;

    for fixture in all_fixtures() {
        // Benchmark documents get the shallow pass. See `run_mutant`.
        let deep = fixture.root != "benchmark";

        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            cases += 1;
            let label = format!("{}/{}", fixture.id, mutation.name());

            let result = catch_unwind(AssertUnwindSafe(|| run_mutant(&mutant, deep)));
            match result {
                Err(_) => panics.push(label),
                Ok(Err(e)) => {
                    if !TAXONOMY.contains(&e.code()) {
                        unnamed.push(format!("{label}: code `{}` ({e})", e.code()));
                    }
                }
                Ok(Ok(_)) => {}
            }
        }
    }

    assert!(cases > 100, "only {cases} mutants ran; coverage collapsed");
    assert!(
        panics.is_empty(),
        "{} mutant(s) panicked. A panic is a release blocker (docs/05-MILESTONES.md M7): the \
         engine must refuse damaged input with a named error, never crash on it.\n  {}",
        panics.len(),
        panics.join("\n  ")
    );
    assert!(
        unnamed.is_empty(),
        "{} mutant(s) were refused outside the six-code taxonomy:\n  {}",
        unnamed.len(),
        unnamed.join("\n  ")
    );
}

/// **A mutant that parses binds to its own bytes, never to the original's.**
///
/// The subtle failure this catches: an engine that opened the mutant but produced an artifact
/// describing the file it was derived from. Nothing downstream — not the oracle, not
/// `grounding-check --source-artifact` — could tell that had happened, because every field would
/// look right.
#[test]
fn a_surviving_mutant_never_claims_to_be_the_original() {
    let mut offenders = Vec::new();
    let mut compared = 0usize;

    for fixture in all_fixtures() {
        let original_digest = format!("sha256:{}", engine_core::sha256_hex_bytes(&fixture.bytes));
        let deep = fixture.root != "benchmark";

        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            let mutant_digest = format!("sha256:{}", engine_core::sha256_hex_bytes(&mutant));

            if let Ok(Ok(Read { bound_sha256 })) =
                catch_unwind(AssertUnwindSafe(|| run_mutant(&mutant, deep)))
            {
                let label = format!("{}/{}", fixture.id, mutation.name());
                compared += 1;
                if bound_sha256 == original_digest {
                    offenders.push(format!("{label}: bound to the ORIGINAL digest"));
                } else if bound_sha256 != mutant_digest {
                    offenders.push(format!(
                        "{label}: bound to {bound_sha256}, which is neither original nor mutant"
                    ));
                }
            }
        }
    }

    // **The comparison only happens for a mutant that parses**, so an empty offender list says
    // nothing on its own: a corpus that went missing, a `run_mutant` that started erroring, or a
    // reader turned fail-closed everywhere would each drive this loop zero times and print `ok`.
    // Sixty mutants survive at v2-S13.1 and [`EXPECTED_SURVIVORS`] pins exactly which, so the
    // count is available and there is no reason to leave it unasserted.
    //
    // The office harness in `crates/engine-office/tests/robustness.rs` was written from this
    // file at v2-S13 and carries this floor. It was not back-ported here, which is why the twin
    // built from the argument ended up holding a guard the original does not — the same shape as
    // v2-S13's `\par`/`\pard` note, arriving from the other direction.
    assert_eq!(
        compared,
        EXPECTED_SURVIVORS.len(),
        "{compared} surviving mutant(s) reached the digest comparison; \
         `EXPECTED_SURVIVORS` pins {}. This test proves nothing about a mutant it never ran.",
        EXPECTED_SURVIVORS.len()
    );

    assert!(
        offenders.is_empty(),
        "an artifact bound to bytes it did not read:\n  {}",
        offenders.join("\n  ")
    );
}

/// **The survivor set is exactly what is pinned.**
///
/// Triage, as a test. A new survivor means a fail-closed path stopped firing on damage it used to
/// catch, and the only way to make this green again is to explain the entry in
/// [`EXPECTED_SURVIVORS`] or fix the path.
#[test]
fn the_surviving_mutants_are_the_pinned_ones() {
    let mut survivors: BTreeSet<String> = BTreeSet::new();

    for fixture in all_fixtures() {
        let deep = fixture.root != "benchmark";
        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            if let Ok(Ok(Read { .. })) =
                catch_unwind(AssertUnwindSafe(|| run_mutant(&mutant, deep)))
            {
                survivors.insert(format!("{}/{}", fixture.id, mutation.name()));
            }
        }
    }

    let expected: BTreeSet<String> = EXPECTED_SURVIVORS.iter().map(|s| s.to_string()).collect();
    let new: Vec<&String> = survivors.difference(&expected).collect();
    let gone: Vec<&String> = expected.difference(&survivors).collect();

    assert!(
        new.is_empty() && gone.is_empty(),
        "the survivor set moved.\n\
         newly surviving (a fail-closed path stopped firing — triage before pinning):\n  {}\n\
         no longer surviving (usually good; still say so in the commit):\n  {}\n\n\
         Full current set, for pasting into EXPECTED_SURVIVORS:\n{}",
        new.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
        gone.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
        survivors
            .iter()
            .map(|s| format!("    \"{s}\","))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// **An injected unknown operator stops the parse**, on every fixture that accepts the injection
/// and whose original extracts.
///
/// `docs/03-V0-SCOPE.md` §1 item 7 is specific: an unrecognised operator stops the parse with a
/// named error. The unit test in `extraction.rs` proves the interpreter does that for a
/// hand-built stream; this proves it end to end, on real documents, through the whole open →
/// extract path.
#[test]
fn an_injected_unknown_operator_stops_the_parse() {
    let profile = Profile::default();
    let mut checked = 0usize;

    for fixture in all_fixtures() {
        let Some(mutant) = Mutation::UnknownOperator.apply(&fixture.bytes) else {
            continue;
        };
        // Only fixtures whose original extracts can prove anything here: if the original is
        // refused at open, the mutant is refused for a reason that has nothing to do with the
        // operator.
        let extracts = Document::open_bytes(&fixture.bytes, &profile)
            .and_then(|d| engine_pdf::extract(&d, &profile))
            .is_ok();
        if !extracts {
            continue;
        }
        checked += 1;

        let e = Document::open_bytes(&mutant, &profile)
            .and_then(|d| engine_pdf::extract(&d, &profile))
            .expect_err(&format!(
                "`{}` with an unknown operator injected must not extract",
                fixture.id
            ));
        assert_eq!(
            e.code(),
            "unsupported",
            "`{}`: an unrecognised operator must be refused as unsupported, got {e}",
            fixture.id
        );
    }

    println!("operator injection exercised on {checked} fixture(s)");
    assert!(
        checked >= 40,
        "only {checked} fixture(s) exercised the operator injection; the mutation is not \
         reaching real documents. Forty-four plaintext fixtures extract and take the injection \
         at v2-S12.1; the floor sits just below, so a fixture becoming unreadable is caught here \
         rather than quietly shrinking the coverage this test claims.\n\n\
         The floor said 12 from M7, when fourteen fixtures took the injection. The corpus tripled \
         underneath it and the floor did not move, so by v2-S12.1 three quarters of the corpus \
         could have stopped extracting with this test still green — a floor far below the real \
         number is a floor that has stopped being one."
    );
}

/// Mutation/fixture pairs that cannot be built, and why.
///
/// Pinned rather than tolerated, for the same reason the survivors are: a silent skip is
/// indistinguishable from coverage. Each entry names a mutation that has nothing to work with,
/// not one that was found inconvenient.
///
/// - **`truncate-16`** on the two header-only failure fixtures — they are 9 and 10 bytes, so
///   there is no sixteenth byte to truncate to. `empty`, `flip-tail-byte`, `header-overwritten`
///   and `junk-after-eof` all still apply to them.
/// - **`unknown-operator`** wherever no plaintext `Tj`/`Td`/`Tm`/`Tf` token exists to overwrite:
///   the header-only fixtures have no content stream at all, the encrypted one's is unreadable by
///   construction, and the benchmark and foreign documents compress theirs. A same-length
///   substitution is the only kind that keeps `/Length` honest, so there is nothing to do here
///   without re-encoding the document — which would be authoring a fixture, not mutating one.
///   The `FlateDecode` exclusion is deliberate and was a triage finding; see `Mutation::apply`.
const EXPECTED_INAPPLICABLE: [&str; 12] = [
    "failure/corrupt-header-valid/truncate-16",
    "failure/corrupt-header-valid/unknown-operator",
    "failure/image-only-or-blank-page/unknown-operator",
    // v1-S6's two image fixtures. Their *content* streams are plaintext, but the image XObject
    // they carry declares `/FlateDecode`, and this mutation excludes any document containing that
    // token whole-file. The exclusion is deliberately conservative — see `Mutation::apply` — and
    // widening it to inspect which stream the hit lands in would trade a real safety property for
    // two more cases. The operator-handling path these fixtures do not cover is covered by every
    // other engine-owned fixture.
    "image-declared-not-drawn/unknown-operator",
    "image-xobject-drawn/unknown-operator",
    "failure/invalid-header/truncate-16",
    "failure/invalid-header/unknown-operator",
    "failure/password-protected/unknown-operator",
    "foreign/opendataloader/real/unknown-operator",
    "irs-form-1040-2025/unknown-operator",
    "nist-sp-800-53r5/unknown-operator",
    "nist-sp-800-63b/unknown-operator",
];

/// **Coverage is exact, and reported.**
///
/// Every fixture in the manifest is mutated, and every mutation that did *not* apply is named.
/// A floor ("at least four each") would let a mutation quietly stop applying to half the corpus;
/// pinning the exact inapplicable set will not.
#[test]
fn every_fixture_is_mutated_and_the_coverage_is_reported() {
    let fixtures = all_fixtures();
    let mut report = Vec::new();
    let mut inapplicable: BTreeSet<String> = BTreeSet::new();
    let mut total = 0usize;

    for fixture in &fixtures {
        let mut applied = Vec::new();
        for m in Mutation::ALL {
            match m.apply(&fixture.bytes) {
                Some(_) => applied.push(m.name()),
                None => {
                    inapplicable.insert(format!("{}/{}", fixture.id, m.name()));
                }
            }
        }
        total += applied.len();
        report.push(format!("{:<40} {}", fixture.id, applied.join(", ")));
    }

    // Printed so a CI log records the coverage this job achieved, rather than only that it
    // passed. `--nocapture` shows it; the assertions below hold either way.
    println!(
        "mutation coverage: {} fixtures, {total} mutants",
        fixtures.len()
    );
    for line in &report {
        println!("  {line}");
    }
    println!("not applicable ({}):", inapplicable.len());
    for line in &inapplicable {
        println!("  {line}");
    }

    assert_eq!(
        fixtures.len(),
        55,
        "the manifest should declare 55 fixtures across three roots (23 at M7, plus v0.1's \
         broken-font-encoding, v1-S1's two ruled-table fixtures, v1-S2's three, and v1-S3's \
         five tagged ones, v1-S4's four form/annotation ones — form-field-value, \
         annotation-contents, form-orphan-widget and form-xfa-stub — v1-S5's two-column pair, \
         and v1-S6's four: image-xobject-drawn, image-declared-not-drawn, \
         invisible-render-mode, off-page-and-offset-box crop-box-smaller-than-media, v1-S6.1's simple-font-two-byte-tounicode, v1-S6.2's whitespace-past-the-page-edge, \
         v1-S7b's background-panel-not-a-grid, and v1-S8's three stroke-ruled ones — \
         stroke-ruled-worksheet, stroke-ruled-columns-not-drawn and stroke-ruled-field-boxes, \
         and v1.1-S1's markdown-two-blocks, v1.1-S2's markdown-table-cells and \
         tagged-list-items, and v1.1-S3's markdown-hyphen-break)"
    );

    let expected: BTreeSet<String> = EXPECTED_INAPPLICABLE
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        inapplicable, expected,
        "the set of mutations that cannot be built has moved. Newly inapplicable means a \
         mutation silently stopped covering a fixture; newly applicable means one started, and \
         both belong in EXPECTED_INAPPLICABLE with a reason."
    );

    assert_eq!(
        total,
        Mutation::ALL.len() * fixtures.len() - EXPECTED_INAPPLICABLE.len(),
        "the mutant count must be every fixture times every mutation, less exactly the \
         pinned inapplicable pairs"
    );
}

/// Guard the guard: the mutations must actually change the bytes.
///
/// A `apply` that returned the input unchanged would make every assertion above pass while
/// testing the unmutated corpus.
#[test]
fn every_mutation_actually_mutates() {
    let original = std::fs::read(repo_root().join("fixtures/engine/measured-ink-box/document.pdf"))
        .expect("engine fixture readable");

    for m in Mutation::ALL {
        let mutant = m
            .apply(&original)
            .unwrap_or_else(|| panic!("`{}` must apply to a plaintext engine fixture", m.name()));
        assert_ne!(
            mutant,
            original,
            "`{}` produced the original bytes unchanged",
            m.name()
        );
    }

    // And each one damages what it claims to.
    assert!(Mutation::Empty.apply(&original).unwrap().is_empty());
    assert_eq!(Mutation::Truncate16.apply(&original).unwrap().len(), 16);
    assert!(!Mutation::HeaderOverwritten
        .apply(&original)
        .unwrap()
        .starts_with(b"%PDF"));
    assert!(Mutation::JunkAfterEof.apply(&original).unwrap().len() > original.len());
    assert!(
        find(&Mutation::UnknownOperator.apply(&original).unwrap(), b"Zq").is_some(),
        "the operator injection must land"
    );
    assert_eq!(
        Mutation::FlipTailByte.apply(&original).unwrap().len(),
        original.len(),
        "a byte flip changes no length"
    );
}
