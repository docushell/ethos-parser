# 03 — v0 scope

**Status:** bootstrap authority · **Sources:** memo §16.10 (base scope), §18.9 (classify deltas),
§16.11 (risks) · **Build order:** `05-MILESTONES.md`

---

## 1. In

PDF only. One happy path, everything in service of it.

| # | Item | Notes |
| --- | --- | --- |
| 1 | **Text-layer classification emitting counts, not confidence** | `pages_with_text` / `pages_sampled` per page, **1-indexed**. No verdict-plus-score |
| 2 | **Reason codes on two orthogonal axes** | OCR-need (`scanned`, `no-text`, `sparse-text`, `embedded-images`, `garbled`, `vector-text`, `annotation-text`) ∪ layout-hard (`multi-column`, `table-likely`, `dense-graphics`). **Neither axis implies the other**: `table-likely` must never trigger an OCR route |
| 3 | **Boolean derived from the reason list** | `needs_attention = !reasons.is_empty()`. The list is the truth; the boolean is a convenience. Removing it must lose no information |
| 4 | **Three distinct exit codes** | simple / needs-attention / could-not-read. See §3.1 |
| 5 | **Bounded classification** | Sample *N* pages (default 8) and **stop**. Cost must not scale with total page count |
| 6 | **Position-aware text runs** | origin (x, baseline y) + advance + font id + font size + page + `mcid` |
| 7 | **Fail-closed content-stream interpretation** | Operator set enumerated explicitly. An unrecognised operator stops the parse with a named error |
| 8 | **Ink bbox from measured font metrics, or typed absence** | `ttf-parser` over the embedded font program, falling back to FontDescriptor `/Ascent`, `/Descent`, `/FontBBox`. **Never `height = font_size`** |
| 9 | **`synthesized` flags** | Every character the reader invented — inserted spaces above all — flagged at emission |
| 10 | **`char_codes` with the ligature caveat declared** | Glyph codes travel with the text; ligature expansion yields more scalars than codes and the artifact says so |
| 11 | **Single-column reading order + an explicit multi-column limitation** | See §3.2. **Retired at v1-S5**, which shipped `gutter-columns-v1` and flipped the capability |
| 12 | **Content-based format detection** | Magic bytes, not extension. Unknown magic fails closed |
| 13 | **Error taxonomy** | Typed variants, distinguishable by a caller. Not one `ParseError(String)` |
| 14 | **Canonical representation: c14n + integer quanta + profile-pinned ids** | `01-CONTRACT.md` §4 |
| 15 | **Capability and limitation declarations** | The L1 gate, not polish. Plus per-page state and a coverage summary |
| 16 | **`DocumentRepresentation v0` emit** | The canonical record |
| 17 | **`ethos.grounding.v1` adapter** | The projection today's verifier consumes |
| 18 | **`grounding-check` validator** | JSON Schema validation **only**, with the Ethos CLI as a deterministic oracle |
| 19 | **CLI + Rust library** | CLI is a thin shell over the library so the two cannot diverge |
| 20 | **Fuzz + mutation tests** | `cargo-fuzz` on the PDF entry point; mutation testing over every fixture |

## 2. Out

Not "not yet done." Out — a PR adding one of these to v0 is rejected on scope, regardless of quality.

| Out of v0 | Lands at | Why not now |
| --- | --- | --- |
| **Tables** | v1 | The hardest quality item. Rebuild with the locator cross-check as its test, against a 0.489 floor — **as v0 saw it. The 0.489 chase was parked 2026-08-19** (`00-NORTH-STAR.md` #10); the cross-check and fabrication 0 still bind |
| **Markdown / HTML as evidence** | v1.1 | Workbench rule 8 — a projection between what is ranked and what is cited is where a locator dies silently. Ships with the Anchor Map or not at all |
| **OCR, in any form** | v2.1 | v0 ships classification and honest refusal, not recognition |
| **Assist / VLM / agents** | v3 | `Proposed` only, and rule 7 (drafting path ≠ verifying representation) needs machinery v0 does not have |
| **Office formats** | v2 | Anydoc-native, never a LibreOffice→PDF bridge |
| **A second PDF backend** | undecided | One backend, one set of quirks, one declared limitation set |
| **Crops / rendering** | — | Typed absence on some nodes means the engine cannot drive crops. PDFium/Ethos keeps that lane |
| **MCP server** | v1.2 | First adapter after CLI + lib — and its locator-handle discipline must be settled before the first tool exists |
| **Python / Node SDKs** | v1.2 | — |
| **Verification of any kind** | see `07-VERIFY-BOUNDARY.md` | Stage 0: the engine does not verify. The happy path terminates at a *validated* artifact |
| **Multi-column reading order** | v1 | Needed a stable rule and a fixture, not a cliff-shaped heuristic. **Shipped at v1-S5** as `gutter-columns-v1`, with the ±1-line fixture pair as the proof |
| **Forms, annotations, vector paths, screenshots** | v1 | — |
| **WASM / napi bindings** | v2+ | Driven by adopter demand, not by completeness |

## 3. The happy path

```
classify  →  extract  →  ground  →  grounding-check
```

Four commands, one library, one document load. Nothing else is in v0.

- **`classify`** — reason codes, two axes, per-page counts, three exit codes, bounded sampling
- **`extract`** — text runs with native locators, font metrics, `mcid`, synthesized flags →
  `DocumentRepresentation v0`
- **`ground`** — project the representation into `ethos.grounding.v1`
- **`grounding-check`** — validate the grounding artifact against its schema and bind it to the
  source bytes

**Single document load.** `classify` and `extract` in one invocation parse the document once and
share it. Re-opening the file per stage is both slower and a correctness hazard — two loads can
disagree.

### 3.1 Exit codes

| Code | Meaning | Example |
| --- | --- | --- |
| **0** | Simple — no reason codes fired | `irs-form-1040-2025` |
| **1** | Needs attention — one or more reason codes fired, document read successfully | a scanned page, a multi-column layout |
| **2** | Could not read — the document did not open, or fail-closed triggered | `failure/password-protected`, `failure/invalid-header`, an unknown operator |

**These three must never collapse into two.** LiteParse's own README predicate,
`lit is-complex doc.pdf --quiet && lit parse doc.pdf --no-ocr`, cannot distinguish "complex" from "I
could not open this" — password-protected, invalid-header, corrupt-header and *missing file* all
exit 1, identically to "this document is complex" (memo §18.3). Failing closed with an
indistinguishable signal is still a defect. Test it: a fixture per code, asserted per fixture.

### 3.2 The multi-column limitation, stated plainly — and retired at v1-S5

**v0 declared this limitation. v1-S5 shipped the rule that replaced it.** The paragraph below is
kept in the past tense rather than deleted, because it is the reasoning that decided what v1 was
allowed to ship, and a slice that later reached for a line count would be reversing a decision this
section made rather than making a new one.

v0 read single-column. A two-column document was read in the **wrong order**, and the artifact
**declared that** as a capability limitation rather than silently producing interleaved text.

That was deliberate. pdf-inspector flips multi-column detection on `min_lines < 15` at
`layout.rs:1793` — fourteen lines per column produces row-interleaved order, fifteen produces
column-major, measured on generated fixtures. A one-line edit to a document reorders the whole page.
A cliff-shaped heuristic cannot sit under a determinism contract. Multi-column was to ship at v1
when it had a stable rule and a fixture, not before. The fixture through v1-S4 was
`synthetic/two-columns`, whose golden asserted the *declared limitation*, not correct order.

**What shipped (v1-S5, 0.7.0).** `reading_order_rule` moved from `single-column-v1` to
`gutter-columns-v1`, `capabilities.multi_column_reading_order` is `true`, and the
`multi-column-reading-order` limitation is gone from the default profile — gone rather than
reworded, because a stale limitation is acted on. The rule cuts on vertical whitespace in page
space and nothing else: no line count, no run count, no threshold on any tally, and no extent
derived from a font size. Where a page shows no gutter it is not reordered at all, which is what
single-column means and is why every single-column fixture reads exactly as it did at v0.

The `synthetic/two-columns` golden **reversed** in that slice, in the open: it now asserts
`Left top, Left bottom, Right top, Right bottom`. Two engine-owned fixtures sit on either side of
pdf-inspector's boundary — `two-column-14-lines` and `two-column-15-lines`, the same page differing
by one line — and read the same way, which is the property that section was always asking for. The
narrower `reading-order-geometric-only` limitation took the retired one's place and says what the
rule still cannot see: column structure that exists only in a tag tree, and runs whose font supplies
no advance.

## 4. Fixture corpus

15 fixtures, from the Ethos tree, used read-only as a conformance corpus.

| Group | Count | Contents |
| --- | --- | --- |
| `fixtures/synthetic/` | 9 | `heading-export`, `hyphenated-line-break`, `ligature-fi-embedded-font`, `list-items`, `rotation-90`, `simple-text`, `table-regular-grid`, `two-columns`, `two-lines` |
| `fixtures/failure/` | 5 | `corrupt-header-valid`, `image-only-or-blank-page`, `invalid-header`, `memory-limit-simulated`, `password-protected` |
| `fixtures/foreign/opendataloader/real/` | 1 | Foreign-adapter round trip |

**Plus three benchmark documents, in a second root.** M2's acceptance names `nist-sp-800-53r5`
(492 pp, the bounded-cost A/B), `nist-sp-800-63b` (80 pp) and `irs-form-1040-2025` (2 pp, the
exit-code-0 case). None is in `fixtures/` — they live at `ethos/benchmarks/gate-zero/corpus/`, and
the manifest resolves them through a `benchmark` root (`ETHOS_BENCH_CORPUS`). They are hash-pinned
like everything else and **do not count toward the 15**.

Plus **37 engine-authored CC0 fixtures**, the first added at M5: a PDF with unusable font metrics,
exercising the geometry-omission path. Each exists because the Ethos corpus has no case for it, and
the set has grown with every slice that needed one — the count here is the manifest's
`counts.engine_owned`, which `crates/engine-pdf/tests/robustness.rs` asserts against the array
length. **The 15-fixture oracle criterion is unchanged** — it is the Ethos conformance corpus, and
engine-owned fixtures are additional test assets, never part of that count. The manifest says so in
the `engine` root's own note: *"Never counted toward the 15-fixture oracle criterion."*

Two of these are known-hostile and both are load-bearing:

- **`table-regular-grid` did not open under `lopdf`, and now does.** Its xref entries are 19 bytes
  (`0000000015 00000 n\n`) where PDF 32000-1 §7.5.4 requires exactly 20 (`…n \n`, trailing space).
  PDFium repairs it; `lopdf` rejects it. One valid document in ~26 on Ethos's own corpus. **v0
  declared the limitation and exited 2; v0.1 decided repair-or-refuse in favour of one bounded,
  declared repair** (`01-CONTRACT.md` §8.1) and it now reads — carrying `xref-entry-padded` and
  moving the oracle partition to 12 compared / 3 refused. It is still a *table* document read as
  single-column text in stream order: the tables limitation is unchanged, and v0.1 is not v1.
- **`simple-text` breaks both surveyed classifiers, in opposite directions.** pdf-inspector calls it
  TEXT-BASED while reporting `Pages with text: 0` (under-routes); LiteParse calls it `no-text` and
  demands OCR (over-routes). Neither is wrong about the document — both thresholds are calibrated for
  real pages and neither is safe on a 20-character fixture. **The lesson is not "pick the better
  classifier."** It is that the classifier emits counts and reasons and the caller owns the policy.

## 5. Exit criteria — v0 is complete when

**Every line is a CI job, not a judgement call.** Closed at M7, and closed the way the sentence
above always meant: each line below names the job in `.github/workflows/ci.yml` that proves it, so
a reviewer can see *which criterion* is green rather than inferring it from one undifferentiated
`cargo test`. `crates/engine-cli/tests/v0_exit_criteria.rs` asserts that every job named here
exists, that no job exists without a criterion, that no `--skip` appears anywhere in the workflow,
and that **no job's test filter matches zero tests** — a filter naming a renamed test would make
its job print `ok. 0 passed` and go green having checked nothing.

`check` still runs the whole suite as the umbrella gate. It is not the proof — a single tick
cannot tell you the classification bound still holds, only that nothing failed.

- [x] `classify → extract → ground → grounding-check` runs end-to-end on all 15 fixtures
      — CI job `v0-happy-path`
- [x] **Double-run byte identity**: running the full path twice over the corpus produces
      byte-identical artifacts, including file bytes and not merely payloads
      — CI job `v0-double-run`
- [x] **Oracle agreement**: for every fixture, `grounding-check` and
      `ethos grounding check <file> --source-artifact <pdf>` agree byte-identically on `structure`,
      `source_binding`, `representation_sha256` and `counts`
      — CI job `v0-oracle`
- [x] Every artifact carries `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`
      — CI job `v0-artifact-identity`
- [x] Every artifact carrying geometry declares `coordinate_system`
      — CI job `v0-coordinates`
- [x] `grep -ri confidence` over emitted artifacts and their public types returns **nothing**
      — CI job `v0-no-confidence`
- [x] No float appears in any canonical artifact; c14n rejects non-integers as a hard error
      — CI job `v0-c14n`
- [x] Every node carries a `NativeLocator`; no box is derived from a font size
      — CI job `v0-locators`
- [x] Capability + limitation declarations present on every artifact, including the explicit
      multi-column limitation
      — CI job `v0-l1-gate`
- [x] Three exit codes, one asserting fixture each, all distinguishable
      — CI job `v0-exit-codes`
- [x] **Classification bound test**: a 500-page PDF at N=8 completes within **20%** of an 8-page PDF
      at similar bytes/page
      — CI job `v0-classify-bound`
- [x] Unknown operator, unknown magic, and unquantizable number each fail closed with a named,
      deterministic error
      — CI job `v0-fail-closed`
- [x] `cargo-fuzz` target exists and runs clean on the corpus; mutation tests cover every fixture
      — CI jobs `v0-fuzz-smoke`, `v0-fixture-mutation` and `v0-office-mutation`
- [x] `cargo deny` green: no AGPL, no network crates
      — CI job `deny-policy-is-enforced`
- [x] No verification code, type, or field exists anywhere in the tree
      — CI job `v0-no-verify`

### 5.1 What each job actually runs

Named tests, not "the suite". Where a criterion has both a grep and a test behind it, the grep is
the job — `docs/05-MILESTONES.md` M7 asks for a job per line, and a grep over `crates/*/src` is the
cheapest honest form of two of these.

| Job | Runs |
| --- | --- |
| `v0-happy-path` | `oracle_agrees_on_all_ethos_owned_fixtures`, `refused_fixtures_fail_closed_rather_than_producing_an_artifact`, `manifest_declares_fifteen_ethos_owned_fixtures` |
| `v0-double-run` | every `*byte_identical*` test, plus the two diagnostics and library-level double-run tests |
| `v0-oracle` | the whole `engine-cli --test oracle` target, against a built `ethos` binary |
| `v0-artifact-identity` | `the_artifact_carries_a_full_identity_envelope` (×2), `the_default_profile_is_pinned`, `artifact_identity_round_trips_through_c14n`, `the_profile_schema_example_is_the_real_profile` |
| `v0-coordinates` | `every_geometry_bearing_artifact_declares_its_coordinate_system` and the profile/schema literals |
| `v0-no-confidence` | `ci/forbidden-tokens.sh confidence` |
| `v0-c14n` | `engine-core`'s c14n, float-rejection and quantize suites |
| `v0-locators` | `every_run_carries_a_native_locator`, `no_source_line_derives_a_box_from_the_font_size`, and the measured/absent metric pair |
| `v0-l1-gate` | the whole `engine-pdf --test capabilities` target |
| `v0-exit-codes` | the three CLI exit tests, `the_three_exit_codes_are_distinguishable`, and the two library-level distinguishability tests |
| `v0-classify-bound` | `the_sampler_is_bounded_on_a_492_page_document` + the counter and flat-cost tests, `--exact --test-threads=1` |
| `v0-fail-closed` | unknown operator (three tests), unknown magic, and the c14n float refusals |
| `v0-fixture-mutation` | the whole `engine-pdf --test robustness` target, `--nocapture` so the coverage report reaches the log |
| `v0-office-mutation` | the whole `engine-office --test robustness` target, `--nocapture` for the same reason. The sixteen office packages are in no manifest, so `v0-fixture-mutation` cannot reach them (v2-S13) |
| `v0-fuzz-smoke` | `cargo fuzz build` on **all three** targets, then 60s each with `-timeout=10` on the two PDF ones. `office_read` is built and not run — v2-S12 measured a per-push office campaign and declined it, and v2-S12.1 added the build because nothing else compiles it |
| `deny-policy-is-enforced` | `cargo deny check licenses`, then the AGPL probe requiring exit 4 |
| `v0-no-verify` | `ci/forbidden-tokens.sh verification` |

**The bound job's load-bearing assertion is the counter**, not the clock:
`pages_content_scanned == 8` on a 492-page document. The wall-clock comparison
(`classify_cost_is_flat_in_total_page_count`) sits beside it and measures the two phases
separately, because total time is `O(parse) + O(N × per-page)` and no amount of bounded sampling
makes the parse term disappear. A flake in the timing half is visible as this job; the counter
half cannot flake.

## 6. Performance posture

**Bounded, and measured only against itself.** No competitor comparison is published at v0.

| Axis | v0 posture |
| --- | --- |
| **Classification** | Sample *N* pages and stop. Target ≤ **0.5 ms per sampled page** in-process, N=8 default. **Cost must not scale with total page count** — that property is the point |
| **Bound test** | 500-page PDF at N=8 within 20% of an 8-page PDF at similar bytes/page. This is the exact A/B that catches the class of bug below |
| **Extraction** | Measured and recorded, not gated at v0. The Ethos G1 rule (`≥ max(120 pps, 2× remeasured ODL)` ⇒ ≥ 134 pages/sec p50) is a **v1** bar |
| **Process spawn** | ~19–22 ms measured floor for a Rust CLI. Acknowledged, not optimised. At the 20,000 docs/day design target (≈14/minute) it is irrelevant |
| **Public claims** | **None.** No "fastest", "#1", or "best" without a gated harness and a named corpus |

**Do not inherit the "10–50 ms" figure.** It originates as a doc-comment at pdf-inspector
`src/lib.rs:390` and is propagated into five READMEs with no benchmark behind it. Measured on the
same code: a 492-page NIST document takes **447 ms** internally, and — the actual finding — its
advertised `Sample(8)` does **not** bound cost, because a Phase-3 rescan walks every page
(`detector.rs:431-447`). On that file `Pages(vec![1])` costs **439 ms**, `Sample(8)` **412 ms**,
`Full` **434 ms**: asking for one page costs the same as asking for all of them. Cost scales at
roughly 0.4–0.55 ms/page on top of parse.

State ethos-engine's own cost as *"~0.5 ms per sampled page, plus document parse,"* make the sample
count a pinned profile field, and keep classification **in-process** — if it ever shells out,
fork/exec alone consumes the entire budget.

## 7. Risks

| # | Risk | Mitigation |
| --- | --- | --- |
| 1 | **`DocumentRepresentation v0` is a target, not a shipped type.** ethos-engine will be its first implementation, so it risks diverging from what DocuShell eventually needs | Emit it, validate against the companion document's field list, **treat the first implementation as the reference and plan a review round**. Every uncertain field carries `TODO(re-read DocumentRepresentation v0 field list)`. This is an accepted, explicit decision — not something to discover in review (memo §16.13) |
| 2 | **`lopdf`'s ~4% open-failure rate** on a corpus PDFium handles | Declare it; keep PDFium/Ethos available for documents it rejects; **measure the rate on a real corpus**, not 26 fixtures |
| 3 | **Multi-column reading order has no stable rule yet** | v0 shipped single-column plus an explicit limitation rather than a cliff-shaped heuristic. **Closed at v1-S5**: the rule cuts on geometric gutters, and `two-column-14-lines`/`-15-lines` hold it to producing the same order across the one-line edit that moves pdf-inspector's |
| 4 | **Ink-box work is the only unbounded item in v0** | Time-box it. Typed absence is an acceptable v0 answer for hard fonts |
| 5 | **MCP locator-handle discipline is easy to get wrong** | An API-shape decision, settled before the first tool exists. Out of v0 entirely, which is itself the mitigation |
| 6 | **Two engines could drift into two answers** | Stage 0 forbids verification entirely; the oracle test is the guard |
| 7 | **Scope creep from the parity checklist.** ~70 rows of tempting capabilities, most of them genuinely good | `06-STEAL-REFUSE.md` names the target version for each. A capability without a v0 row is out of v0 |

**Unmeasured, and worth an hour each** — none blocks M0:

- Per-invocation `ethos verify` cost
- `lopdf` open-failure rate on a real (non-fixture) corpus
- Ink-box agreement with PDFium on the ADR-0009 five
- The LiteParse compounding-garble spike: item-level `is_likely_garbled` (10% vowel floor) strips
  items from `text_length`, which can drive `text_length < 20` → `no-text` on a page that extracted
  perfectly. Acronym-dense pages (a NIST control table of `AC-2`/`SC-7`) are the trigger shape. One
  generated fixture, ~30 min — worth doing **before** finalising v0's own reason-code thresholds

---

## PR review checklist

- [ ] The change belongs to a numbered row in §1, or it is out of scope
- [ ] No item from §2 has crept in — especially tables, Markdown, OCR, or anything verification-shaped
- [ ] Classification remains bounded; the 500-page A/B still passes
- [ ] No performance figure is claimed that a committed harness does not produce
- [ ] New capability ⇒ new capability declaration ⇒ new profile hash
- [ ] A new limitation is declared in the artifact, not only in a doc comment
