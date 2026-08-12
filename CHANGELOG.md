# Changelog

All notable changes to ethos-engine. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This project is pre-release and has no published versions. Entries are grouped by **milestone**
(`docs/05-MILESTONES.md`) rather than by version number, because a milestone is the unit of work
that has acceptance criteria.

## [Unreleased]

### M4 — Capabilities, typed absence, explicit multi-column limitation (the L1 gate)

L1's achievement condition names capability declarations explicitly, so an artifact without them
has not reached "extracted" regardless of how good its text is. Every emitted artifact now
declares what the profile can do, what it could not do, what happened to each page, and how the
run ended.

**Added — `engine-core`**

- `assurance` — `Limitation` (stable kebab code + detail + `Profile | Document | Page(n)` scope),
  `PageState`, `PageStateEntry`, `CoverageSummary`, `ProcessingGaps`, `ProcessingTerminalState`,
  `RefusalCode`, and the `Assurance` envelope both artifacts embed.
- `Assurance::new` **derives** the coverage summary and the terminal state from the page states it
  is given. An artifact that claims `Complete` while carrying an unread page is not a bug this
  type can have — which is the only way to guarantee `docs/01-CONTRACT.md` §7's rule that a
  verification over a partially processed document never renders as a clean verification of the
  whole document.
- `CoverageSummary` reconciles: `pages_authorized == processed + failed + unsupported +
  quarantined + not_attempted`. Six buckets, not five: **`not_attempted` is the one a four-bucket
  summary would have had to lie about.** Bounded classification samples `N` pages and stops, so
  folding the rest into `processed` would claim observations nobody made and folding them into
  `failed` would claim failures that never happened.
- `page_binding_status` — **capability-limited beats negative** (Workbench rule 4). A query bound
  to a failed, quarantined, or unattempted page returns `CapabilityLimited { limitation_code }`,
  never a boolean "not present". The signature is the enforcement: there is no way to express
  "missing", so absence of extractable content cannot become evidence of absence in the source.
  `NotInDocument` is a separate answer, because "we skipped it" and "there is no such page" are
  different facts.
- `Capabilities::declared_limitations` — the mirror of the proof rule: **no capability may be
  `false` without a declared limitation**, derived by exhaustive destructuring so adding one
  without a code is a compile error.
- `PageBudget` on `Profile` — `Unlimited` or `AtMost(n)`, a **declared state rather than an
  optional field**. An `Option` missing from incoming JSON deserializes to `None` and silently
  re-hashes as though it had been there; a declared enum cannot.

**Added — `engine-pdf`**

- `limitations` — the format-specific codes `engine-core` is not allowed to know about:
  `backend-xref-strict-20-byte`, `predefined-cmaps-not-vendored`,
  `form-xobject-text-not-descended`, `font-widths-absent`, `classify-sample-bound`, and the
  derived `*-reason-not-detected` pair.
- The **xref refusal is declared on artifacts for documents it did not refuse.**
  `synthetic/table-regular-grid` exits 2 with no body, so the only place a caller can learn this
  backend turns away roughly one document in twenty-six is an artifact for one it accepted.

**Changed — the wire, deliberately**

- **`not_detected` (M2) and `not_decoded` (M3) are gone.** Both were declared stand-ins. Their
  content is now `assurance.limitations`, carrying M2's and M3's reasons verbatim — a test fails
  if any `NOT_DETECTED` entry loses its declaration in the move, and another fails if either field
  reappears. Two vocabularies on one artifact means a consumer has to work out which to trust, and
  the answer is never written down.
- **`capabilities.char_offsets`: `true` → `false`.** v0 emits runs and no element/span hierarchy,
  so there is nothing for an offset to index into. M4 asked for the proof and there was none.
  Flips at M5 with `DocumentRepresentation v0`, with a test.
- **`profile_sha256` moved** to
  `sha256:f34be6328f858e09c241cb51c7b0dbb0fe065ecbf5f00e9942cd6bbcdd6faf1e` — the honest
  `char_offsets` value plus the new `page_budget` knob. Artifacts from before and after are
  correctly non-comparable, because the profile that produced them really did change.
- `Classification::pages_sampled` is now `min(page_count, classify_sample_pages, page_budget)`, so
  it keeps meaning "the pages this run intended to read" and stays equal to
  `pages_content_scanned`.

**`failure/memory-limit-simulated`, stated plainly**

Its bytes are **identical to `synthetic/simple-text`** (`sha256:f2f6ab91…`, asserted in the test
rather than taken on trust). The fixture name means *a limit simulated by configuration*, not
*this PDF is huge*, so the test sets `page_budget` explicitly — and on a one-page document the
only budget that bites is zero. The behaviour is a **declared limitation plus a coverage gap**,
not a hard refusal: one authorized page, zero processed, one quarantined, terminal state
`partial`, and no page tree at all. The same bytes under the default profile read cleanly, which
is the proof the gap came from the knob.

**Fixed — two guards that were not guarding**

- The profile sensitivity test's `capabilities.char_offsets` mutation wrote back the value the
  field already held once `V0` flipped, so it proved nothing while passing. Every mutation now
  asserts it actually changed the profile before asking whether the hash moved.
- `no_pdf_type_or_import_in_engine_core` banned the **prefix** `struct Page`, which read the
  format-agnostic `PageStateEntry` as a PDF page-tree type — forbidding a type the contract
  requires. Banned type names are now matched as whole identifiers, with a test proving the
  exact-match form still catches `struct Page {`.
- `profile.draft.json`'s example still carried `"unbound-until-m3"` placeholders two milestones
  after the backend landed, while its README told readers the example *is* the real profile. The
  example is corrected and `the_profile_schema_example_is_the_real_profile` now holds it there.

**Not done, deliberately**

- **No CLI flag for the page budget.** It is a profile knob, so the binary cannot emit a partial
  artifact at v0 and the question of which exit code one deserves does not arise. Exit-code
  meanings are unchanged.
- **`pages_failed` and `pages_unsupported` are always zero in v0 artifacts.** Extraction fails
  closed on a hard error and emits nothing at all, so no page reaches those states through the
  public path. The buckets are declared with honest zeros and the query helper handles them, but
  nothing pretends they are exercised.
- No repair, no invented pagination, no capability claimed without a proof test.

### M3 — Extract: text runs, native locators, fail-closed operators, synthesized flags

Position-aware text runs whose origins are trustworthy, whose boxes are measured or typed-absent,
and whose parse stops rather than silently dropping content.

**Added — `engine-pdf`**

- `ops` — all **73** operators of PDF 32000-1 Table A.1 as an enum. Dispatch is an exhaustive
  match with **no wildcard arm**: adding a variant without handling it is a compile error, and a
  token outside the table is a hard error naming it. Operators that do not move a glyph are
  *named* no-ops, so "we do not interpret `rg`" is a decision in the source rather than an
  accident of a `_ =>`.
- `content` — the interpreter, including all four show-text operators. **`"` and `'` are
  implemented**; `"`'s absence from pdf-inspector's match is the disqualifying defect, where text
  vanishes and adjacent runs merge with corrupt geometry while the output stays well-formed.
- `text_state` — CTM, text and line matrices, `Tc`/`Tw`/`Tz`/`TL`/`Ts`. **`Tz` multiplies the
  whole advance expression**, spacing terms included; pdf-inspector implements it nowhere.
- `fonts`, `metrics` — three independent questions per glyph (what character, how far, what box),
  because they fail independently. Conflating the last two is how `height = font_size` gets
  written.
- `cmap` — `ToUnicode` parsing (`bfchar`, `bfrange`, codespace ranges), the source of the ligature
  caveat: `<03>` → `<00660069>` is one code and two characters.
- `encoding` — `WinAnsiEncoding` in full, `StandardEncoding`'s ASCII range including the two codes
  where it is *not* ASCII (`0x27` is `quoteright`, `0x60` is `quoteleft`), and `/Differences`.
- `nodes` — `TextRun` with `PdfLocator`, `SynthesizedChar`, `scalar_code_mismatch`.
- `engine extract <pdf>` — canonical JSON, exit 0 or 2. **No exit 1**: "needs attention" is a
  classify concept, and overloading it would make a caller's `&&` chain mean two different things
  depending on which subcommand ran.

**Honesty, and where it shows**

- **Ink boxes are measured or absent.** The whole conformance corpus is standard-14 Helvetica with
  no `FontDescriptor`, so every run reports `NotReportedByReader` — the typed-absence path, proved
  on real documents rather than a mock. One engine-owned fixture carries a descriptor so the
  measured path is proved too.
- **Absent advance is absent, not zero.** A standard-14 font may omit `/Widths` and expect built-in
  AFM metrics, which this profile does not vendor. The advance is omitted from the wire; the origin
  is unaffected, because it comes from the content stream.
- **Hyphenation is not rejoined**, and that is a stated policy with a golden. Distinguishing a soft
  break-hyphen from a real compound hyphen needs a dictionary, and a rule that guesses is the
  cliff-shaped heuristic refused everywhere else. Silent rejoining is forbidden outright.
- **Reading order is stream order.** `two-columns` comes out right-column-first — visibly wrong,
  and asserted that way, because that is what `single-column-v1` means.

**Changed**

- **`skrifa` replaces `ttf-parser`**, which the milestone named. RUSTSEC-2026-0192 records that
  ttf-parser's author declared it unmaintained with no safe upgrade, and names skrifa as the
  maintained successor. Pinned to 0.39 because 0.44's tree needs Rust 1.89 and this workspace pins
  1.88.
- `Profile.cmap_data_version`: `absent-until-m3` → `annex-d-encodings-1`, naming what is actually
  carried. The pinned profile digest moved accordingly.
- The float-ban guard now matches **whole tokens**. It was matching substrings, and a sha256
  digest containing `…cf32c2e…` tripped it — a guard that cries wolf on a hash is a guard someone
  eventually disables.

**Deviation from the milestone text, stated plainly**

M3 called for vendoring ~168 Adobe `.bcmap` CMaps. They are **not** vendored. Nothing in the corpus
exercises them, they could not be obtained and verified in this pass, and committing binary data no
test touches would be worse than declaring the gap. A document naming a predefined CMap is
**refused** with a named error. The same applies to the Core-14 AFM widths and the full Adobe Glyph
List. All three are declared in `not_decoded` and explained in `vendor/README.md`.

**Not done**

- M4 capability blocks and coverage summaries; M5 `DocumentRepresentation v0` and the grounding
  projection; M6 oracle agreement.

### M2 — Classify: reason codes, two axes, three exit codes, bounded sampling

`engine-pdf` opens a PDF once and reports what it observed. No verdict, no confidence, no quality
field — the engine owns the observation, the caller owns the routing policy.

**Added**

- `engine_pdf::Document` — magic-checked, opened once, shared by every stage. M3's `extract` takes
  the same handle rather than reopening: two loads can disagree, and a classifier that saw a
  different object graph from the extractor is a silent divergence with no diagnostic.
- `engine_pdf::classify` — per-page counts, two orthogonal reason axes, a derived boolean, and
  `pages_content_scanned` as an observable bound.
- `OcrNeedReason` / `LayoutComplexityReason` — **separate types**, so `table-likely` cannot be
  constructed as an OCR-need reason. A dense financial table in crisp born-digital text needs no
  OCR at all; a single "complex" list would send it to an engine that could only make it worse.
- `engine classify <pdf>` — canonical JSON on stdout, exit 0 / 1 / 2. `extract`, `ground` and
  `grounding-check` exit 2 naming the milestone that owns them, rather than printing usage and
  exiting 0 as though the work happened.
- `docs/draft-schemas/classification.draft.json`.

**Measured, and it changed two acceptance lines**

- **`irs-form-1040-2025` is exit 1, not exit 0.** It fires `table-likely` and `dense-graphics`.
  Suppressing a true layout reason to make a doc line come out right is the tuning this project
  refuses, so the fixture choice changed: `synthetic/simple-text` is the exit-0 case, and
  `irs-form-1040` became the axis-independence case, which it serves better.
- **The 20%-total-time bound was unachievable, and the reason was informative.** The counter proved
  the page walk was already bounded — 492 pages, 8 scanned — while `lopdf` parses the whole object
  graph eagerly, so total cost is `O(parse) + O(N × per-page)`. Timing the phases separately shows
  what the claim actually is: **246× the pages for 1.7× the classify time.**

**Honesty about what is not detected**

`garbled` and `multi-column` are in the vocabulary and are **never emitted**, because no sound
detector exists for either. The artifact declares both in `not_detected` with the reason — silence
would let a caller read an empty list as evidence a document is not garbled.

`sparse-text` requires short text **alongside imagery**. LiteParse uses `text_length < 20`
unconditionally and compounds it with a vowel-frequency garble heuristic that strips items from the
tally first, so an acronym-dense page can be reported as `no-text` when its text extracted
perfectly. Neither mechanism exists here, and a test asserts `nist-sp-800-53r5` — a control
catalogue full of `AC-2`/`SC-7` — is not reported as textless.

**Changed**

- `Profile.backend.version` is now the resolved `lopdf` version, `0.44.0`, replacing
  `unbound-until-m3`. The pinned profile digest moved accordingly — the mechanism working as
  designed.
- `deny.toml` gains `BSD-3-Clause` (`lopdf` → `encoding_rs`; also the licence the vendored Adobe
  CMaps will need at M3), plus `Unlicense`, `Zlib` and `0BSD` from the same subtree. Each added in
  the PR that introduced the crate needing it.

**Not done**

- Text extraction: content-stream interpretation, CMaps, ink boxes, `NativeLocator` emit (M3). The
  classifier walks content streams to **count** operators; it decodes no text and interprets
  nothing.

### M1 — Contract types, c14n / quanta, `schema_version` on the wire

`docs/01-CONTRACT.md` is now Rust. The artifact shape stops being negotiable and starts being a
compile error.

**Added — `engine-core`**

- `c14n` — c14n v1, a clean-room implementation of the contract Ethos implements. UTF-8, no
  whitespace, keys sorted **explicitly at write time**, minimal escaping, integers only,
  idempotent. Parity vectors lifted from Ethos's committed tests prove byte compatibility without
  taking a Cargo dependency on Ethos.
- `geom` — `quantize` (round-half-away-from-zero; `NaN`/`±∞`/overflow are errors, never clamps)
  and `QRect` as `[x0, y0, x1, y1]` with construction-time validation.
- `identity` — `ArtifactIdentity`, `ArtifactBinding`, `Sha256Hex`, `CoordinateSystem`.
- `profile` — `Profile`, `Capabilities`, `BackendIdentity`, and `profile_sha256()`.
- `derivation` — `DerivationClass` and `GeometryPresence` / `GeometryAbsence`.
- `ids` — `IdAllocator`, `NodeId`, `IdKind`, `sort_ids`.
- `error` — the six-variant `EngineError` taxonomy.

**Added — docs**

- `docs/draft-schemas/` — six DRAFT JSON Schemas plus an index. Under `docs/` deliberately; there
  is still no production `schemas/` path.
- This changelog.

**Decisions worth knowing**

- **`quantize` uses `f64::round`, not Ethos's `(x + 0.5).floor()`.** That idiom double-rounds: once
  `|x| ≥ 2^52` the sum is unrepresentable and rounds *before* `floor` runs, so an exact integer
  product returns one quantum too large — and `MAX_SAFE_INT`, which the contract declares canonical,
  is refused outright. `f64::round` is IEEE `roundToIntegralTiesToAway`: same rule, computed
  exactly. Measured divergence from Ethos across an exhaustive knife-edge sweep of `[0, 2·10^6)`:
  **exactly one value**, `0.49999999999999994`, where this returns `0` (correct — it is below one
  half) and Ethos returns `1`. Unreachable from decimal text, and across all ten million
  `0.001`-step literals in `[0, 10000)` points the two agree everywhere.
- **`QRect` rejects zero-area rectangles**, which is *stricter* than Ethos, whose `QRect::new`
  rejects only `x0 > x1`. Stricter-on-emission is the only safe direction: the engine may refuse to
  emit what the oracle tolerates, never the reverse.
- **`quantize` rejects `quantum_per_point == 0`**, which would otherwise map every coordinate on
  the page to the origin and return `Ok(0)` while doing it.
- **Typed absence, not `Option<QRect>`.** `None` would collapse "could not measure", "nothing to
  measure", and "not asked to measure" into one value, and only the first is a declarable capability
  limitation.
- **`Sha256Hex` rejects uppercase hex** rather than folding it, so one digest has exactly one
  spelling and a comparison never fails for formatting reasons.
- **The default profile is pinned by test**, bytes and digest. A profile change is now a deliberate
  act — including a crate version bump, since `parser_version` is part of identity by design.
- **`Unicode-3.0` added to `deny.toml`'s allowlist.** M1 is the PR that introduced the crate needing
  it: `serde`'s `derive` feature reaches `unicode-ident`, whose licence is
  `(MIT OR Apache-2.0) AND Unicode-3.0` — the `AND` makes it non-optional. Added on introduction
  rather than in advance, which is what the prior comment asked for.

**Fixed after adversarial review**

- **Unknown-field denial now covers every nested object, not the ones someone remembered.**
  `deny_unknown_fields` is not recursive, and the first pass stopped one level short: a profile
  carrying `coordinate_system.future_knob` parsed cleanly and **re-hashed to the unmodified default
  digest** — measured — so an artifact would claim comparability with a profile it does not match.
  Now on `CoordinateSystem`, plus `ArtifactIdentity`, `ArtifactBinding` and `GeometryPresence`,
  matching the `additionalProperties: false` their draft schemas already declared. The nested test
  derives its field list from the serialized value rather than a hardcoded array, so a nested
  object added later is covered automatically.
- **`DerivationClass::may_be_overwritten_by` implemented only half of §6.** It ignored its second
  argument entirely, so it protected `Extracted` while letting `Proposed` overwrite `Computed`,
  `Recognized`, and other `Proposed` nodes — the path by which a model's suggestion quietly becomes
  the record. Both rules are now enforced and pinned by a full 4×4 matrix, plus a test asserting
  the result actually depends on the overwriter.
- `Profile` now denies unknown fields. Without it a profile from a newer engine deserialized with
  its unknown knob silently dropped, then **re-hashed to a different digest than it arrived with** —
  an artifact claiming comparability it does not have.
- The profile sensitivity gate now destructures to the leaf. A field added to `Capabilities` or
  `BackendIdentity` is as output-affecting as one on `Profile`; a gate stopping at
  `capabilities: _` waved it straight through.
- The float ban was scoped to `quantize`'s body rather than the whole of `geom.rs`. The file-level
  skip was a hole big enough for `QRect` — which lives there, derives `Serialize`, and would have
  passed with an `f64` field. Verified by injecting one and watching the guard fire.
- `sort_ids` put a malformed id **first** while its doc claimed last, because `Option`'s natural
  order sorts `None` first. Now sorts last, with a test.

**Not done, deliberately**

- No PDF parsing, classification, or extraction (M2, M3).
- No grounding projection (M5).
- `oracle_agrees_on_simple_text` still fails with its M1–M6 diagnostic, as designed until M6.

### M0 — Repo skeleton, toolchain, deny policy, failing oracle harness

- Four-crate workspace with enforced boundaries; toolchain pinned to 1.88.0.
- `deny.toml`: allowlist-only licences (no AGPL), no network crates, pinned registry.
- `fixtures/manifest.json` — hash-pinned references into the Ethos corpus across two roots.
- Oracle harness with negative-path tests; `oracle_agrees_on_simple_text` fails by design.
- CI: fmt, clippy, build, test, deny, plus a job that *proves* the AGPL gate rejects.
