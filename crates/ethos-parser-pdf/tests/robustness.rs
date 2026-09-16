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

//! Fixture mutation: every fixture in the manifest, mechanically damaged, six ways.
//!
//! `docs/history/03-V0-SCOPE.md` §5 requires mutation tests covering every fixture. This is the
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
//! a release blocker either way (`docs/history/05-MILESTONES.md` M7).

use std::collections::BTreeSet;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use ethos_parser_core::{EngineError, Profile};
use ethos_parser_pdf::Document;
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
        + m["counts"]["engine_owned"].as_u64().unwrap()
        + m["counts"]["gate"].as_u64().unwrap();
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
    /// One byte inverted in the file trailer's **cross-reference pointer** (v2-S21).
    ///
    /// It said *"in the last tenth of the file, where the xref and trailer live"* until v2-S21,
    /// and that was a claim about a fixed fraction rather than about the trailer. See
    /// [`Mutation::apply`] for what it cost and when it stopped being true.
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
                // **Seek from `startxref`, not from a fixed fraction** (v2-S21).
                //
                // This read `len - len/20 - 1` — the midpoint of the last tenth — and claimed to
                // be *"deep enough to land in the xref/trailer region on every fixture in the
                // corpus"*. **That was never true of a large document.** A fraction of a file's
                // length has nothing to do with where its trailer is, and the two only coincided
                // because every fixture inspected during triage was under three kilobytes.
                // Measured on all sixty-four: the index landed *hundreds of kilobytes before*
                // `startxref` — inside a compressed object stream, an embedded font, or image
                // data — **373 468 bytes before it on `nist-sp-800-53Ar5`**, 303 661 on
                // `nist-sp-800-53r5`, 242 247 on `nist-sp-800-161r1`. Those documents take the
                // shallow pass, which never decompresses that stream, so nothing read the flipped
                // byte at all. Eighteen of them were pinned as survivors, and v2-S19 corrected the
                // *reason* to *"they survive because the mutation missed"* while leaving the
                // mutation missing. A harness that reports coverage it does not have is the
                // v2-S12.1 / v2-S13.1 shape.
                //
                // The trailer is where a reader **enters** the cross-reference region: PDF
                // 32000-1 §7.5.5 makes `startxref <offset> %%EOF` the last thing in the file and
                // that offset the only way to find the xref at all. So the anchor is the final
                // `startxref` keyword and the index is the midpoint of what follows it — still a
                // formula and not a magic index, and now one whose meaning does not depend on the
                // file's size. Measured on all sixty-four: it lands on a **digit of the
                // cross-reference offset** on every fixture that has one, at `startxref + 10` on
                // a 585-byte synthetic and `startxref + 11` on a 7 MB NIST publication.
                //
                // **`None` where there is no `startxref`.** Two header-only failure fixtures — 9
                // and 10 bytes — declare no cross-reference region, so a mutation that damages one
                // has nothing to damage. That is a real answer and it is pinned in
                // `EXPECTED_INAPPLICABLE`; both were refused for their headers before and after,
                // so no coverage is lost.
                let sx = rfind(original, b"startxref")?;
                let idx = sx + (original.len() - sx) / 2;
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

/// The **last** offset where `needle` appears.
///
/// Last rather than first, because a document written by incremental update carries several
/// `startxref` keywords and only the final one names the cross-reference section a reader enters
/// through. `foreign/opendataloader/real` is such a file.
fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|w| w == needle)
        .filter(|_| !needle.is_empty())
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
/// small fixtures do not already give. The large real-world documents take the shallow pass —
/// the `benchmark` and `gate` roots — and which they are is reported, never silent.
///
/// **`gate` joined that list at v2-S19 for the same reason `benchmark` was on it, not for
/// convenience.** The roots are separate because of *where the bytes live* — `benchmark` resolves
/// into the Ethos tree, `gate` is committed here — and that has nothing to do with mutation depth.
/// What decides depth is size, and the eleven documents S19 added run to 1,476 pages, one of them
/// alone larger than the 492-page fixture this paragraph was written about. Deep-mutating them
/// would add hours per run and no signal. Naming the roots is still a proxy for size, and a
/// crude one; a `depth` field per manifest entry would say it directly. That is a manifest schema
/// change and this slice already moves the manifest's counts, so it is named here rather than
/// taken quietly.
fn run_mutant(bytes: &[u8], deep: bool) -> Result<Read, EngineError> {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile)?;

    let classification = ethos_parser_pdf::classify(&doc, &profile)?;
    let canonical = classification.to_canonical_bytes()?;
    let v: Value = serde_json::from_slice(&canonical).expect("canonical bytes are JSON");
    let bound = v["source"]["sha256"]
        .as_str()
        .expect("every artifact binds to a source digest")
        .to_string();

    if deep {
        let extract = ethos_parser_pdf::extract(&doc, &profile)?;
        let repr = ethos_parser_pdf::to_representation(&extract, &profile)?;
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
/// Pinned rather than merely permitted, and since v2-S21 there is **one class left**.
///
/// - **`junk-after-eof`, on every fixture that opens at all.** A PDF reader reaches the trailer
///   through `startxref`, so bytes appended past `%%EOF` sit outside every offset the document
///   declares. Nothing reads them, and the document really is intact. The safety property still
///   holds and is asserted separately: the artifact binds to the *mutant's* digest, so a consumer
///   comparing hashes sees a different document, which it is.
///
/// # The eighteen `flip-tail-byte` survivors are gone, and that is the whole of v2-S21
///
/// They were pinned under two headings — small fixtures where `lopdf` recovered from a damaged
/// trailer, and large documents where **the byte landed nowhere load-bearing**. v2-S19 split those
/// two and corrected the second's reason to *"they survive because the mutation missed, not
/// because the reader recovered"*, and left the mutation missing. It named the repair and did not
/// take it: *"making `flip-tail-byte` seek the trailer rather than a fixed fraction would exercise
/// the xref path on large documents for the first time. That is a harness change with a
/// measurement attached."*
///
/// **Here is the measurement.** With the flip landing on a digit of the cross-reference offset,
/// **all sixty-two fixtures that carry a `startxref` refuse**, every one of them `malformed` with
/// the same reason: *"failed parsing cross reference table: invalid start value"*. Survivors go
/// **78 → 60** and not one mutant newly survives.
///
/// The pinned set is **64** at v2.2-S3, and not one of the four additions since v2-S21 is a
/// mutation-behaviour change. `ink-past-the-media-box` (D4-S5), `form-xobject-text-drawn`
/// (v2.2-S2) and the composite-font pair `composite-font-cid-widths` /
/// `composite-font-non-identity-cmap` (v2.2-S3) were each added to the corpus and each survives
/// `junk-after-eof` exactly as every other engine fixture does — bytes appended past `%%EOF`
/// leave the cross-reference table resolving. The v2-S21 measurement above stands as recorded;
/// only the population moved. `rotated-and-mirrored-text` (0.58.0) joins them the same way, and
/// with it and the two fixtures decision #22 and v2.2-S5 added — `absent-font-widths` and
/// `untagged-shredded-line` — the set is **67**. `leading-gap-two-blocks`, the block cut's own
/// fixture (OPEN-WORK §2.2), joins the same way and makes it **68**. The four `engine-tagged-*`
/// fixtures of auto-tagging S1 — the same page under a hand-written structure tree — join the
/// same way and make it **72**: a structure tree changes nothing about how a damaged xref, a
/// truncated stream or an injected operator is refused, and each survives `junk-after-eof` alone.
/// The seven fixtures of auto-tagging S2 — the shapes the writer must refuse or place around,
/// each the leading-gap page with one thing changed — join the same way and make it **79**:
/// a marked-content frame, a shared stream or an inline image in the content changes nothing
/// about how the damage is refused either (83 fixtures, 475 mutants).
///
/// Both old headings dissolve rather than shrink, and neither was quite right:
///
/// - **The large documents were never a reader property at all.** `nist-sp-800-53Ar5`,
///   `nist-sp-800-53r5`, `nist-sp-800-161r1` and the nine others took a flip 373 468, 303 661,
///   242 247 … bytes short of their trailer. Nothing read the byte, so nothing could refuse it.
///   They now refuse, which is the repair working: they were surviving a mutation that never
///   reached them.
/// - **The small ones were a real reader observation, and it did not survive a harder blow.**
///   The flip used to land in the trailer *dictionary* — the `t` of `/Root` on
///   `synthetic/two-lines`, the `R` of `1 0 R` on `synthetic/two-columns` — and `lopdf` recovered
///   by scanning for the catalog instead of trusting the damaged reference. Corrupting the
///   **pointer to the cross-reference table** is a different injury: there is no table to scan
///   *toward*, and the parse stops at `xref` rather than at the catalog. So those five stop
///   surviving too, and the recovery behaviour they documented is still real — it is simply not
///   what this mutation tests any more.
///
/// **What no longer has a home, said rather than left implicit.** `lopdf`'s catalog-scan recovery
/// was covered only by those five survivors and is now covered by nothing. It is a backend
/// behaviour rather than an engine guarantee, no test asserted it, and a mutation weak enough to
/// exercise it is the mutation this slice removed — so it is named here as coverage this corpus
/// stopped having, not quietly dropped.
///
/// **v2-S23 decided it stays that way — argued deletion rather than a fixture.** The choice this
/// slice weighed was a fixture that exercises the recovery again, or refusing one. It refuses:
/// catalog-scan recovery is `lopdf` *salvaging* a document whose trailer dictionary is damaged,
/// which is the exact opposite of what this harness tests — that damage makes the reader fail
/// closed. A fixture pinning it would assert a backend leniency the engine makes no promise about
/// and whose behaviour it does not own, and re-weakening `flip-tail-byte` to resurrect the five
/// survivors would trade v2-S21's real repair for a coverage number. There is nothing
/// engine-owned to delete; the deletion is of the *claim* that this corpus covers it, made here.
/// See `docs/history/15-V2-MILESTONES.md` S23.
///
/// An entry appearing here that is not `junk-after-eof` is a fail-closed path that stopped firing
/// — triage it before pinning it. An entry disappearing is a path that started firing, which is
/// usually good and still wants a commit message.
/// (Four and eleven at M7, when the corpus was fifteen documents; nine and forty-six at v2-S13.1;
/// eighteen and forty-six at v2-S19.)
const EXPECTED_SURVIVORS: [&str; 79] = [
    "absent-font-metrics/junk-after-eof",
    "absent-font-widths/junk-after-eof",
    "annotation-contents/junk-after-eof",
    "background-panel-not-a-grid/junk-after-eof",
    "both-table-rules/junk-after-eof",
    "broken-font-encoding/junk-after-eof",
    "cfpb-home-loan-toolkit/junk-after-eof",
    "composite-font-cid-widths/junk-after-eof",
    "composite-font-non-identity-cmap/junk-after-eof",
    "crop-box-smaller-than-media/junk-after-eof",
    "engine-tagged-blocks/junk-after-eof",
    "engine-tagged-classmap/junk-after-eof",
    "engine-tagged-mixed/junk-after-eof",
    "engine-tagged-nested-frames/junk-after-eof",
    "failure/image-only-or-blank-page/junk-after-eof",
    "failure/memory-limit-simulated/junk-after-eof",
    "foreign/opendataloader/real/junk-after-eof",
    "form-field-value/junk-after-eof",
    "form-orphan-widget/junk-after-eof",
    "form-xfa-stub/junk-after-eof",
    "form-xobject-text-drawn/junk-after-eof",
    "horizontal-scaling-tz/junk-after-eof",
    "image-declared-not-drawn/junk-after-eof",
    "image-xobject-drawn/junk-after-eof",
    "ink-past-the-media-box/junk-after-eof",
    "inline-image-filtered/junk-after-eof",
    "invisible-render-mode/junk-after-eof",
    "irs-f1040sd-2025/junk-after-eof",
    "irs-form-1040-2025/junk-after-eof",
    "irs-fw9/junk-after-eof",
    "leading-gap-nested-frames/junk-after-eof",
    "leading-gap-two-blocks/junk-after-eof",
    "markdown-hyphen-break/junk-after-eof",
    "markdown-table-cells/junk-after-eof",
    "markdown-two-blocks/junk-after-eof",
    "measured-ink-box/junk-after-eof",
    "nist-sp-800-161r1/junk-after-eof",
    "nist-sp-800-171r3/junk-after-eof",
    "nist-sp-800-207/junk-after-eof",
    "nist-sp-800-218/junk-after-eof",
    "nist-sp-800-37r2/junk-after-eof",
    "nist-sp-800-53Ar5/junk-after-eof",
    "nist-sp-800-53r5/junk-after-eof",
    "nist-sp-800-63b/junk-after-eof",
    "off-page-and-offset-box/junk-after-eof",
    "rotated-and-mirrored-text/junk-after-eof",
    "ruled-table-grid/junk-after-eof",
    "ruled-table-overlap/junk-after-eof",
    "ruled-wins-shared-region/junk-after-eof",
    "shared-content-stream/junk-after-eof",
    "show-text-quote-operators/junk-after-eof",
    "simple-font-two-byte-tounicode/junk-after-eof",
    "stroke-ruled-columns-not-drawn/junk-after-eof",
    "stroke-ruled-field-boxes/junk-after-eof",
    "stroke-ruled-worksheet/junk-after-eof",
    "synthesized-space-tj/junk-after-eof",
    "synthetic/heading-export/junk-after-eof",
    "synthetic/hyphenated-line-break/junk-after-eof",
    "synthetic/ligature-fi-embedded-font/junk-after-eof",
    "synthetic/list-items/junk-after-eof",
    "synthetic/rotation-90/junk-after-eof",
    "synthetic/simple-text/junk-after-eof",
    "synthetic/table-regular-grid/junk-after-eof",
    "synthetic/two-columns/junk-after-eof",
    "synthetic/two-lines/junk-after-eof",
    "tagged-list-items/junk-after-eof",
    "tagged-rolemap/junk-after-eof",
    "tagged-structure-roles/junk-after-eof",
    "tagged-table-agrees/junk-after-eof",
    "tagged-table-disagrees/junk-after-eof",
    "two-column-14-lines/junk-after-eof",
    "two-column-15-lines/junk-after-eof",
    "unruled-near-miss/junk-after-eof",
    "untagged-artifact-furniture/junk-after-eof",
    "untagged-mcid-by-name/junk-after-eof",
    "untagged-mcid-no-tree/junk-after-eof",
    "untagged-oc-by-name/junk-after-eof",
    "untagged-shredded-line/junk-after-eof",
    "whitespace-past-the-page-edge/junk-after-eof",
];

// -------------------------------------------------------------------------------------------
// The suite
// -------------------------------------------------------------------------------------------

/// **No mutant panics, and every refusal is named.**
///
/// The load-bearing test. A panic here is a release blocker; an unnamed refusal is a caller that
/// cannot route the failure, which `docs/history/03-V0-SCOPE.md` §3.1 exists to prevent.
#[test]
fn no_mutant_panics_and_every_refusal_is_named() {
    let mut panics: Vec<String> = Vec::new();
    let mut unnamed: Vec<String> = Vec::new();
    let mut cases = 0usize;

    for fixture in all_fixtures() {
        // Benchmark documents get the shallow pass. See `run_mutant`.
        let deep = !matches!(fixture.root.as_str(), "benchmark" | "gate");

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
        "{} mutant(s) panicked. A panic is a release blocker (docs/history/05-MILESTONES.md M7): the \
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
        let original_digest = format!(
            "sha256:{}",
            ethos_parser_core::sha256_hex_bytes(&fixture.bytes)
        );
        let deep = !matches!(fixture.root.as_str(), "benchmark" | "gate");

        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            let mutant_digest = format!("sha256:{}", ethos_parser_core::sha256_hex_bytes(&mutant));

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
    // Sixty mutants survive at v2-S21 and [`EXPECTED_SURVIVORS`] pins exactly which, so the
    // count is available and there is no reason to leave it unasserted. (Sixty at v2-S13.1 too,
    // and seventy-eight in between: v2-S19 grew the corpus and v2-S21 made `flip-tail-byte` land
    // where it always claimed to, which took all eighteen of that mutation's survivors away.)
    //
    // The office harness in `crates/ethos-parser-office/tests/robustness.rs` was written from this
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
        let deep = !matches!(fixture.root.as_str(), "benchmark" | "gate");
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
/// `docs/history/03-V0-SCOPE.md` §1 item 7 is specific: an unrecognised operator stops the parse with a
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
            .and_then(|d| ethos_parser_pdf::extract(&d, &profile))
            .is_ok();
        if !extracts {
            continue;
        }
        checked += 1;

        let e = Document::open_bytes(&mutant, &profile)
            .and_then(|d| ethos_parser_pdf::extract(&d, &profile))
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
///
/// - **`flip-tail-byte`** on the two fixtures that declare no cross-reference region (v2-S21).
///   They are 9 and 10 bytes and carry no `startxref`, so a mutation that damages the trailer's
///   pointer to the xref table has no pointer to damage. **No coverage is lost:** both are refused
///   for their headers with or without the flip, so the mutant they used to produce proved nothing
///   about the tail. `empty`, `header-overwritten` and `junk-after-eof` all still apply to them.
///
/// **v2-S19 moved this from twelve pairs to twenty-one, and every one of the nine is the same
/// case already described above.** The slice pinned `cfpb-home-loan-toolkit` and added the eight
/// `gate` documents, and all nine compress their content streams — they are real publications from
/// government typesetting pipelines, which is exactly the property that got them admitted to the
/// gate corpus. So the count moved and the *reason* did not: no mutation stopped covering anything
/// it used to cover, and the ratio of inapplicable pairs is a fact about how real PDFs are built
/// rather than a gap in the harness.
///
/// **v2-S21 moves it to twenty-three**, and the two additions are the `flip-tail-byte` case above
/// — the first entries here that are not the `FlateDecode` exclusion.
const EXPECTED_INAPPLICABLE: [&str; 23] = [
    "failure/corrupt-header-valid/flip-tail-byte",
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
    "failure/invalid-header/flip-tail-byte",
    "failure/invalid-header/truncate-16",
    "failure/invalid-header/unknown-operator",
    "failure/password-protected/unknown-operator",
    "foreign/opendataloader/real/unknown-operator",
    "irs-form-1040-2025/unknown-operator",
    "nist-sp-800-53r5/unknown-operator",
    "nist-sp-800-63b/unknown-operator",
    // v2-S19: the gate corpus. `cfpb-home-loan-toolkit` was always scored and never pinned; the
    // eight `gate` entries are the documents that took the corpus from four to twelve. All nine
    // compress their content streams, which is the `FlateDecode` case above and not a new one.
    "cfpb-home-loan-toolkit/unknown-operator",
    "irs-f1040sd-2025/unknown-operator",
    "irs-fw9/unknown-operator",
    "nist-sp-800-161r1/unknown-operator",
    "nist-sp-800-171r3/unknown-operator",
    "nist-sp-800-207/unknown-operator",
    "nist-sp-800-218/unknown-operator",
    "nist-sp-800-37r2/unknown-operator",
    "nist-sp-800-53Ar5/unknown-operator",
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
        83,
        "the manifest should declare 83 fixtures across FOUR roots. Auto-tagging S2 moved this \
         from 76 by adding the seven shapes the writer must refuse or place around — \
         `untagged-mcid-no-tree`, `untagged-mcid-by-name`, `untagged-oc-by-name`, \
         `untagged-artifact-furniture`, `shared-content-stream`, `inline-image-filtered` and \
         `leading-gap-nested-frames` — each the leading-gap page with one thing changed, so only \
         that thing is tested: an id in the content stream and no tree, inline and by name; a \
         named list that is a layer; furniture marked /Artifact; one stream two pages share; a \
         filtered inline image; and the untagged twin of `engine-tagged-nested-frames`. \
         Auto-tagging S1 moved this \
         from 72 by adding the four `engine-tagged-*` fixtures — `engine-tagged-blocks`, \
         `engine-tagged-classmap`, `engine-tagged-mixed` and `engine-tagged-nested-frames` — the \
         leading-gap page under the structure tree the writer will emit, written by hand BEFORE \
         the writer exists so the reader is tested against a file no mistake the two might share \
         could have produced: the owner attribute under `/A`, through `/ClassMap`, beside a \
         foreign owner, and inside existing marked-content frames. OPEN-WORK §2.2 moved this from \
         71 by adding `leading-gap-two-blocks`, the first fixture authored for the leading-gap \
         half of the block cut: six lines at a stated leading with one stated gap, so the two \
         blocks it comes out in are checkable against `blocks.rs` by hand, where the three engine \
         fixtures that already came out in two blocks did so by accident of a layout built for \
         something else. 0.58.0 moved this from 70 by \
         adding `rotated-and-mirrored-text`, the only engine fixture whose text does not run along \
         +x: the box was built from the advance's x alone, so text turned by its text matrix was \
         typed as drawing nothing and text turned by its CTM got a box along x, and no engine \
         fixture could see either. Decision #22 moved this from 69 by adding `absent-font-widths`: vendoring Adobe's Core-14 AFMs means a Helvetica document with no `/Widths` is now MEASURED, so `synthetic/simple-text` stopped being able to prove typed absence and five tests were quietly asserting the recovered path instead. `/ArialMT` is the metric substitution that decision refuses, so nothing can answer for it. v2.2-S5 moved this from 68 by \
         adding `untagged-shredded-line`, the only engine fixture whose runs share a baseline — \
         every other one stacks them, so the whole CLI suite was blind to the undeclared join \
         and a rule keyed on `same baseline, next ink along it` passed it unchanged. v2.2-S3 \
         moved this from 66 by \
         adding the composite-font pair — `composite-font-cid-widths` and \
         `composite-font-non-identity-cmap` — the shape NEITHER owned corpus contained: no \
         fixture anywhere held a CIDFont, so the composite-width path was exercised by nothing \
         and read `/Widths` off a dictionary the format never puts it on. v2.2-S2 moved it from \
         65 by adding `form-xobject-text-drawn`: the third member of v1-S6's `Do` pair, and the \
         one placement that produces no node of any kind. v2-S19 moved it from 55: \
         it added the `gate` root — eight tagged public documents committed to `fixtures/gate/` \
         so the table gate could be measured on twelve documents instead of four — and pinned \
         `cfpb-home-loan-toolkit.pdf` in `benchmark`, which had carried the largest share of the \
         gate number while being pinned by nothing. `docs/table-gate-v1.md` predicted this \
         assertion would fail the day that gap closed. The other 55 (23 at M7, plus v0.1's \
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

/// **The tail flip lands in the cross-reference pointer — shown, not asserted** (v2-S21).
///
/// The claim `Mutation::apply` used to make was *"deep enough to land in the xref/trailer region
/// on every fixture in the corpus"*, and nothing checked it. It was false on every large document
/// in the corpus for four slices, and the harness reported coverage it did not have.
///
/// So the claim is a test now. For every fixture the flip applies to, the changed byte must sit
/// **between the final `startxref` keyword and end of file** — the trailer's pointer to the
/// cross-reference table, which is the only route a reader has to find it (PDF 32000-1 §7.5.5).
/// The offset relative to `startxref` is **printed** for the largest fixture and the smallest, so
/// a CI log records the property rather than only that nothing failed: the whole defect was a
/// distance nobody had ever looked at.
#[test]
fn the_tail_flip_lands_in_the_cross_reference_pointer() {
    let mut checked = 0usize;
    let mut extremes: Vec<(usize, String, i64, usize)> = Vec::new();

    for fixture in all_fixtures() {
        let Some(mutant) = Mutation::FlipTailByte.apply(&fixture.bytes) else {
            // Only the two fixtures that declare no cross-reference region at all, and
            // `EXPECTED_INAPPLICABLE` pins exactly which.
            assert!(
                rfind(&fixture.bytes, b"startxref").is_none(),
                "`{}` has a `startxref` but took no tail flip",
                fixture.id
            );
            continue;
        };
        checked += 1;

        let n = fixture.bytes.len();
        assert_eq!(
            mutant.len(),
            n,
            "`{}`: a byte flip changes no length",
            fixture.id
        );
        let changed: Vec<usize> = (0..n).filter(|i| mutant[*i] != fixture.bytes[*i]).collect();
        assert_eq!(
            changed.len(),
            1,
            "`{}`: the flip must change exactly one byte, not {}",
            fixture.id,
            changed.len()
        );
        let idx = changed[0];
        let sx = rfind(&fixture.bytes, b"startxref").expect("applied, so it has one");

        assert!(
            idx > sx && idx < n,
            "`{}`: the flip landed at {idx}, which is {} byte(s) from `startxref` at {sx} — \
             OUTSIDE the trailer's cross-reference pointer. That is the v2-S21 defect returning: \
             a mutation named for the tail must damage the tail on a 7 MB document as it does on \
             a 600-byte one, or the eighteen documents it used to miss go back to surviving for a \
             reason that is about the harness rather than about the reader.",
            fixture.id,
            idx as i64 - sx as i64
        );
        // And it must land on the offset itself rather than on `startxref`'s own letters, or the
        // pointer would still parse and the mutation would be damaging a keyword the reader
        // locates by searching for it.
        assert!(
            fixture.bytes[idx].is_ascii_digit(),
            "`{}`: the flip landed on {:?} at {idx}, not on a digit of the cross-reference offset",
            fixture.id,
            fixture.bytes[idx] as char
        );
        extremes.push((n, fixture.id.clone(), idx as i64 - sx as i64, sx));
    }

    extremes.sort();
    for (label, e) in [("smallest", extremes.first()), ("largest", extremes.last())] {
        if let Some((n, id, delta, sx)) = e {
            println!(
                "tail flip, {label} fixture: {id} is {n} bytes, `startxref` at {sx}, \
                 flip at startxref+{delta}"
            );
        }
    }

    assert!(
        checked >= 60,
        "only {checked} fixture(s) took the tail flip; sixty-two do at v2-S21 and the floor sits \
         just below. A fixture dropping out here is a document that stopped declaring a \
         cross-reference region, which is worth knowing rather than absorbing."
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
