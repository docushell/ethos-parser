# 05 — v0 milestones

**Status:** **all seven complete — v0 shipped at v0.1.0** · **This is the code-review map.** Every
v0 PR belonged to exactly one milestone. Kept as the historical record and as the acceptance list
each milestone was actually held to; work after this is versions, not M-numbers
(`02-ROADMAP.md`).

**Start at M0.** Do not start M1 until M0's acceptance tests are green. The order is not a
suggestion — M1 freezes the types M3 emits, M4 declares what M5 projects, and M6 cannot exist before
M5. Skipping ahead means rewriting.

| ID | Milestone | Gist | State |
| --- | --- | --- | --- |
| **M0** | Repo skeleton + toolchain + deny + failing oracle harness | The harness exists and fails honestly | done |
| **M1** | Contract types + c14n / quanta + `schema_version` on the wire | The artifact shape, frozen in code | done |
| **M2** | Classify: reason codes, two axes, counts, three exit codes, bounded sampling | Observation without judgement | done |
| **M3** | Extract: text runs, `NativeLocator`, font ids, fail-closed operators, synthesized flags | The evidence itself | done |
| **M4** | Capabilities + typed absence + explicit multi-column limitation | The L1 gate | done |
| **M5** | `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter | The canonical record and its projection | done |
| **M6** | `grounding-check` validator + double-run byte identity on the corpus | Agreement with the oracle | done |
| **M7** | CLI + library freeze + v0 exit criteria green | v0 | **done** |

---

## M0 — Repo skeleton, toolchain, deny, failing oracle harness

- **Goal:** A workspace that builds, a licence posture that is enforced rather than intended, and a
  test that fails for the right reason. The first commit contains the oracle test and one fixture,
  **failing**. Nothing else belongs in week one.

- **In:** `git init`; `rust-toolchain.toml` pinned to `1.88.0`; workspace `Cargo.toml` (resolver 2,
  MSRV 1.88) with the four empty crates from `04-ARCHITECTURE.md` §1; `deny.toml` (permissive
  licences only, no AGPL, no network crates); Apache-2.0 `LICENSE` + `NOTICE` reserved for the
  vendored CMaps; `fixtures/manifest.json` referencing the Ethos corpus by path and `sha256`;
  `crates/ethos-parser-cli/tests/oracle.rs` with one fixture (`synthetic/simple-text`), failing; CI running
  `build`, `test`, `clippy -D warnings`, `fmt --check`, `cargo deny check`, plus a job that **proves**
  the AGPL gate rejects.

- **Out:** Any parsing. Any type beyond what the harness needs to compile. Vendoring the CMap data
  (that is M3's, when there is something to decode). Fixture *copies* — the manifest references, it
  does not duplicate.

- **Artifacts / APIs:** `fixtures/manifest.json`;
  `crates/ethos-parser-cli/tests/oracle.rs::oracle_agrees_on_simple_text`; the `ethos-parser` binary failing
  closed with exit 2; CI green except the one test that is meant to fail.

- **Acceptance tests:**
  - **The M0 gate, stated exactly** — these two together, because a bare `cargo test --workspace`
    exits non-zero at M0 *by design* and asserting otherwise would be a contradiction:
    - `cargo test --workspace --locked -- --skip oracle_agrees_on_simple_text` is **green**
      *(historical: this exclusion was the M0 gate and was deleted at M6, when the test began
      running the real comparison. The gate today is the bare command.)*
    - the unskipped `oracle_agrees_on_simple_text` **fails** with the named M1–M6 diagnostic
    <br>**Do not resolve this by making the bare command green.** Every mechanical route —
    `#[ignore]`, deleting the test, weakening it to a `return` — destroys the milestone, and the
    test's own diagnostic forbids it in as many words. The exclusion is the gate, and it is the
    same exclusion CI encodes.
  - `cargo build --workspace --locked` succeeds; `clippy -D warnings` and `fmt --check` clean.
  - `cargo deny check` passes, and the AGPL probe **fails with exit code 4 and names
    `ethos-parser-core`** — proving the gate fired for *that* reason, not merely that cargo-deny was
    unhappy. Any non-zero exit would also match a config typo.
  - `crates/ethos-parser-cli/tests/oracle.rs` **fails** with a message naming what is missing (no
    implementation yet), not with a panic, a skip, or `todo!()`.
  - The oracle harness locates the `ethos` binary and **errors loudly if absent** — never skips.
  - **`ETHOS_BIN` is authoritative, not a hint.** Set to a non-existent path, the harness fails hard
    rather than falling back to another binary. Resolving silently to a verifier nobody chose is
    worse than finding none: the operator believes they know which one answered. (The repo build is
    0.6.0 and the binary on `PATH` is 0.5.0 — the fallback would have been *stale*, not merely
    different.)
  - `fixtures/manifest.json` hashes verify against the Ethos tree; a mutated hash fails the check.
  - An absent or wrong fixture corpus fails with a named error, never a skip.

- **Review checklist:**
  - [ ] Toolchain pinned to `1.88.0` in a committed file, not in CI config alone
  - [ ] `deny.toml` denies AGPL **and** network-capable crates, and the denial is tested
  - [ ] No fixture bytes copied into this repo; manifest references + hashes only
  - [ ] The Ethos tree is untouched (`git -C ../ethos status` clean)
  - [ ] The failing test fails with a diagnostic, not a `todo!()` panic
  - [ ] No `src/` file contains logic yet
  - [ ] **Every negative-path acceptance criterion has a committed test**, not prose. The
        `ETHOS_BIN` hard-fail and the mutated-hash detection are branches; an assertion nobody has
        watched fail is an assertion nobody has tested
  - [ ] Env-dependent behaviour is tested through a parameterised function, never by mutating
        process env — `set_var` races the threaded test harness and corrupts sibling tests
  - [ ] The manifest's declared counts are validated against its own array

- **Depends on:** nothing.

---

## M1 — Contract types, c14n / quanta, `schema_version` on the wire

- **Goal:** `01-CONTRACT.md` expressed as Rust types with the canonicalization that makes them
  byte-stable. After M1 the artifact shape stops being negotiable and starts being a compile error.

- **In:** `ethos-parser-core`: artifact identity (`artifact_type`, `schema_version`, `parser_version`,
  `profile_sha256`); the `Profile` type and its hash; `coordinate_system`; c14n v1 (UTF-8, no
  whitespace, keys sorted explicitly at write time, minimal escaping, **integers only**, idempotent);
  `quantize(pts, 100)` with round-half-away-from-zero and `NaN`/`±Inf`/overflow as errors; `QRect` as
  `[x0, y0, x1, y1]`; stable-ID ordering discipline; `DerivationClass`; typed-absence variants; the
  error taxonomy; DRAFT JSON Schemas under `docs/draft-schemas/`.

- **Out:** Production JSON Schemas under a shipped `schemas/` path — DRAFT under `docs/` only, until
  the DocuShell review round closes the `TODO(re-read DocumentRepresentation v0 field list)` markers.
  Any PDF concept. Any grounding projection.

- **Artifacts / APIs:** `ethos_parser_core::{c14n_bytes, quantize, QRect, Profile, DerivationClass,
  ArtifactIdentity, CoordinateSystem}`; `docs/draft-schemas/*.draft.json`.

- **Acceptance tests:**
  - **c14n idempotence**, property-tested: `c14n(parse(c14n(v))) == c14n(v)` over generated values.
  - **Float rejection**: any non-integer number anywhere in a canonical value is a hard error, not a
    rounding. Includes nested arrays and objects.
  - **Key ordering** is by Unicode code point and holds even with `serde_json/preserve_order` enabled
    in the dependency graph — test with the feature forced on. **The test must first prove the
    feature is actually active**, by asserting plain `serde_json` emits insertion order; otherwise it
    passes vacuously under a `BTreeMap` that sorts for free, which is worse than having no test.
  - **Quantize vectors**: `0.005 → 1`, `0.004 → 0`, `-0.005 → -1`, `612.0 → 61200`, `-0.0 → 0`;
    `NaN`, `INFINITY`, and `1e17` all error.
  - **Escaping**: no Unicode normalization; non-ASCII is emitted literally; `U+0000–U+001F` becomes
    lowercase `\u00xx`.
  - **Profile sensitivity**: mutating each profile field in turn changes `profile_sha256`, asserted
    field by field.
  - **`grep -ri confidence`** over `ethos-parser-core`'s public API returns nothing. Enforced as a test
    that scans `src/**` with comments stripped — prose arguing the rule is fine, an identifier is
    not — plus a self-test proving the comment stripper works, so the scan cannot pass vacuously.
  - Round-trip: every artifact type serializes, canonicalizes, and re-parses to an identical value.
  - **The default profile is pinned** by bytes and digest, so a profile change is a deliberate act
    rather than a discovery. Distinct from the sensitivity test: that proves a change is
    *detectable*, this proves it was *intended*.

- **Review checklist:**
  - [ ] Keys sorted explicitly at write time, not via map iteration order
  - [ ] No `f32`/`f64` in any serialized type — check the derives, not just the fields
  - [ ] `QRect` is `[x0, y0, x1, y1]`, not `[x, y, w, h]`; ordering is validated at construction
  - [ ] Every profile-relevant knob is in `Profile`, and the sensitivity test covers it
  - [ ] Typed absence is a variant, never `Option<T>` standing in for "we did not measure"
  - [ ] **Every** nested object in a hashed type denies unknown fields. `deny_unknown_fields` does
        not recurse, and guarding only the outer struct leaves a dropped nested knob re-hashing to
        the unmodified digest
  - [ ] A rule stated over a pair of values is tested as a matrix, not from one side. A function
        that ignores an argument passes every single-sided test
  - [ ] Schemas are DRAFT, under `docs/draft-schemas/`, and say so in `$comment`
  - [ ] Uncertain field names carry `TODO(re-read DocumentRepresentation v0 field list)`
  - [ ] No public confidence field, score, grade, or quality summary

- **Depends on:** M0.

---

## M2 — Classify: reason codes, two axes, counts, three exit codes, bounded sampling

- **Goal:** A classifier that reports what it saw and refuses to render a verdict. The caller owns
  the policy; the engine owns the observation.

- **In:** Two orthogonal reason enums — OCR-need (`scanned`, `no-text`, `sparse-text`,
  `embedded-images`, `garbled`, `vector-text`, `annotation-text`) and layout-hard (`multi-column`,
  `table-likely`, `dense-graphics`); per-page counts (`pages_with_text` / `pages_sampled`),
  **1-indexed**; the derived boolean; bounded sampling with `N` (default 8) as a pinned profile
  field; the three exit codes; content-based format detection by magic bytes.

- **Out:** Any confidence float. Any single verdict field. Any routing decision — the engine reports,
  the caller routes. OCR itself. Multi-column *handling* (the reason code is emitted; the reading
  order is still single-column until v1).

- **Artifacts / APIs:** `ethos_parser_pdf::classify(&Document, &Profile) -> Classification`;
  `ethos-parser classify <pdf>` emitting the classification artifact.

- **Acceptance tests:**
  - **Bounded cost, the load-bearing test**, in two parts:
    - **Counter (mandatory)**: `pages_content_scanned == min(N, page_count)` and never approaches
      `page_count`. Measured on `nist-sp-800-53r5`: **492 pages, 8 scanned.** This is the exact
      class of bug pdf-inspector has at `detector.rs:431-447`, where `Pages(1)` costs the same as
      `Full`.
    - **Timing**: classify time must be flat in total page count. Measured against `nist-sp-800-63b`:
      **246× the pages for 1.7× the classify time.**
    <br>**Corrected after measurement:** this line originally demanded the 492-page document
    complete within 20% of a short control *in total*. It cannot, and the reason is not the
    sampler — `lopdf` parses the whole object graph eagerly, so total cost is
    `O(parse) + O(N × per-page)` and the parse term scales with bytes. Measured (release, best of
    3): open 16 / 48 / 431 ms against classify 14 / 21 / 25 ms for 2 / 80 / 492 pages. The phases
    are therefore timed separately, which is also what `03-V0-SCOPE.md` §6 actually claims:
    *"~0.5 ms per sampled page, plus document parse."*
  - **Three exit codes, one fixture each, all distinguishable**: `synthetic/simple-text` → **0**;
    `failure/image-only-or-blank-page` (fires `no-text`) → **1**; `failure/password-protected` →
    **2**; `failure/invalid-header` → **2**; `synthetic/table-regular-grid` (19-byte xref) → **2**;
    a missing file → **2**. Assert the codes, not the messages.
    <br>**Corrected after measurement:** this line originally named `irs-form-1040-2025` as the
    exit-0 case. It is not — it fires `table-likely` and `dense-graphics`, so under the derivation
    rule it is exit **1**. Suppressing a true layout reason to make a doc line come out right is
    exactly the tuning this project refuses, so the fixture choice changed instead. `irs-form-1040`
    is now the axis-independence case, which it serves better: heavy ruling lines with crisp
    born-digital text, so the layout axis fires and the OCR axis stays empty.
  - **Axis independence**: a fixture producing `table-likely` and no OCR-need reason emits an empty
    OCR-need list. `table-likely` never appears in the OCR-need axis.
  - **Boolean derivation**: `needs_attention == !ocr_reasons.is_empty() || !layout_reasons.is_empty()`,
    and removing the boolean from the artifact loses no information (property test over fixtures).
  - **Page indexing**: a round-trip test pins 1-based indexing. `synthetic/two-lines` and
    `nist-sp-800-63b` both assert page 1 is `1`.
  - **`simple-text` behaviour is recorded, not tuned to match anyone.** pdf-inspector calls it
    TEXT-BASED with zero text pages; LiteParse calls it `no-text` and demands OCR. Neither is the
    target. **Measured here:** one page, one text-showing operator, 11 text bytes, no imagery, and
    therefore **no reasons on either axis** — because short text without competing content is a
    short page, not a sparse one. Golden pins every count.
  - **The `garbled` and `multi-column` reasons are never emitted, and the artifact says so.** No
    sound detector exists for either, so silence would let a caller read an empty list as evidence
    of absence. Both are declared in `not_detected` with the reason why, and an acronym-dense
    document (`nist-sp-800-53r5`, full of `AC-2`/`SC-7`) is asserted **not** to be reported as
    textless — the LiteParse compounding-garble trap.
  - **No confidence**: `grep -ri confidence` over the classify artifact and its types returns nothing.
  - **Unknown magic** fails closed with a named error and exit **2**.

- **Review checklist:**
  - [ ] Sampling actually stops at `N` — no later phase rescans all pages
  - [ ] `N` is a profile field and changing it moves `profile_sha256`
  - [ ] Two axes are separate types, not one enum with a comment
  - [ ] Exit codes 0/1/2 are distinguishable for every failure mode, including missing file
  - [ ] No threshold in the code produces a *routing decision* — thresholds may produce *reason codes*
  - [ ] Classification runs in-process; nothing forks or spawns
  - [ ] Page indices are 1-based everywhere, including internal types

- **Depends on:** M1.

---

## M3 — Extract: text runs, `NativeLocator`, font ids, fail-closed operators, synthesized flags

- **Goal:** The evidence itself. Position-aware text runs whose origins are trustworthy, whose boxes
  are measured or absent, and whose parse stops rather than silently drops content.

- **In:** Content-stream interpretation with the operator set **enumerated explicitly**; text runs
  carrying origin (x, baseline y) + advance + font id + font size + page + `mcid`; `NativeLocator`
  (`PdfLocator` variant) on every run; measured ink boxes via `ttf-parser` over the embedded font
  program with FontDescriptor `/Ascent` `/Descent` `/FontBBox` fallback; vendored Adobe CMaps
  (168 `.bcmap` + NOTICE) for encoding; `synthesized` flags; `char_codes` with the ligature caveat;
  single-column reading order as a versioned `Computed` rule.

- **Out:** Multi-column reading order. Tables. Any box derived from a font size. Any operator handled
  by "ignore and continue." Underline/strikeout inference. Markdown.

- **Artifacts / APIs:** `ethos_parser_pdf::extract(&Document, &Profile) -> Vec<Node>` with typed locators;
  `ethos-parser extract <pdf>`.

- **Acceptance tests:**
  - **`"` and `'` show-text operators are handled**, with a fixture proving text is not lost. This is
    the disqualifying pdf-inspector defect (checklist P6): the `"` operator is absent from its
    operator match, its text vanishes silently, and surrounding runs merge with corrupt geometry.
  - **Unknown operator fails closed** with a named error and exit **2** — a fuzz-generated stream
    containing an undefined operator must not produce a well-formed artifact.
  - **`Tz` (horizontal text scaling) is applied** to advance width, with a fixture. pdf-inspector
    does not implement it anywhere in its tree, so its widths are wrong on any document using it.
  - **No box is derived from a font size**: a test asserts no code path sets height from `font_size`,
    and a font with unavailable metrics produces **typed absence**, never a fallback box.
  - **Synthesized characters flagged**: a fixture where an inter-run space is inserted asserts the
    flag at the character, and the flag survives canonicalization.
  - **Ligature caveat**: `synthetic/ligature-fi-embedded-font` round-trips with the
    scalars ≠ `char_codes` mismatch declared in the artifact, not silently reconciled.
  - **Hyphenation**: `synthetic/hyphenated-line-break` — if a rejoin is performed it is `Computed`,
    reversible, and the source bytes are recoverable.
  - **Rotation**: `synthetic/rotation-90` produces geometry in the declared coordinate system after
    rotation, with `rotation: 90` on the page.
  - **`table-regular-grid` exits 2** with a named xref error (19-byte entries where PDF 32000-1
    §7.5.4 requires 20), and the failure is a **declared limitation**, not a crash.
  - **Determinism**: extracting twice produces byte-identical output for every fixture that opens.

- **Review checklist:**
  - [ ] The operator set is an explicit, exhaustive match — no `_ => continue`, no `_ => {}`
  - [ ] Every text run has a `NativeLocator`
  - [ ] Ink boxes come from measured metrics; the absence path is a type, not a sentinel
  - [ ] Synthesized characters are flagged **where created**, not reconstructed later
  - [ ] Reading-order rule carries a version, and that version is in the profile
  - [ ] CMap data is vendored with the Adobe BSD-3-Clause NOTICE reproduced
  - [ ] No text is dropped anywhere without a typed diagnostic — grep for silent `continue`
  - [ ] `mcid` is captured where present and typed-absent where not

- **Depends on:** M1. (M2 and M3 can proceed in parallel after M1; both need M1's types.)

---

## M4 — Capabilities, typed absence, explicit multi-column limitation

- **Goal:** Close the **L1 gate**. An artifact that does not declare its capabilities has not reached
  "extracted," regardless of how good its text is.

- **In:** Capability declarations per profile; limitation declarations per document; per-page
  processing state; the coverage summary reconciling authorized / processed / failed / unsupported /
  quarantined pages; the partial-processing terminal state; the **explicit multi-column limitation**;
  the `lopdf` xref limitation with its declared failure mode.

- **Out:** Any capability declared `true` that is not tested. Any limitation that exists only in a doc
  comment. Repairing anything.

- **Artifacts / APIs:** `ethos_parser_core::{Capabilities, Limitation, PageState, CoverageSummary}`;
  capability + limitation blocks in every emitted artifact.

- **Acceptance tests:**
  - **Every declared capability has a passing test.** A capability asserted `true` with no test is a
    build failure — enumerate capabilities in a test that maps each to its proof.
  - **`synthetic/two-columns` declares the multi-column limitation**, and its golden asserts the
    *declaration*, not correct reading order.
  - **Partial processing is terminal and visible**: a document with one failed page produces a
    coverage summary showing the gap, and the artifact is **not** presentable as complete.
  - **Absence is never `1.0`**: a processor reporting no uncertainty emits an **absent field**, never
    an implied full-confidence value. Asserted by schema and by test.
  - **`failure/memory-limit-simulated`** produces a declared limitation plus a coverage gap, not a
    partial artifact silently presented as whole.
  - **Capability-limited beats negative**: a query against an unprocessed page returns
    capability-limited, never "not present." Absence of extractable content is never evidence of
    absence in the source.
  - Changing the capability set changes `profile_sha256`.

- **Review checklist:**
  - [ ] No capability is `true` without a named test proving it
  - [ ] Limitations appear **in the artifact**, not only in docs
  - [ ] Coverage summary is present on every representation, including fully successful ones
  - [ ] Partial processing has its own terminal state and cannot be mistaken for success
  - [ ] Typed absence is used everywhere a measurement was not taken
  - [ ] No repair, normalization, or fill-in happens without being declared

- **Depends on:** M3.

---

## M5 — `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter

- **Goal:** The canonical record and the projection a verifier consumes. This is where the engine's
  output first becomes something another system can read.

- **In:** `DocumentRepresentation v0` — schema version, source `ArtifactRef` + fingerprint + media
  type, processing-run and processor/profile identities, representation fingerprint,
  capability/limitation declarations, ordered typed nodes with stable IDs, required `NativeLocator`,
  optional structural locator, optional geometry, per-page state, coverage summary, diagnostics; the
  `ethos-parser-grounding` adapter projecting it to `ethos.grounding.v1`; the geometry-absent omission rule
  with its declared count; **one engine-authored CC0 fixture with unusable font metrics**, the only
  addition to the corpus beyond the Ethos manifest, because Ethos has no fixture for this case.

- **Out:** `tables` (v0 emits `capabilities.tables: false` and no table array). Any grounding field
  outside the schema — it is `additionalProperties: false`. Any PDF concept inside
  `ethos-parser-grounding`.

- **Artifacts / APIs:** `ethos-parser extract` → representation JSON; `ethos-parser ground <representation>` →
  `ethos.grounding.v1` JSON.

- **Acceptance tests:**
  - **Schema conformance**: emitted grounding artifacts validate against
    `ethos/schemas/ethos-grounding-source.schema.json` with **no additional properties**.
  - **Field-exact projection**: `artifact_type == "ethos.grounding.v1"`,
    `schema_version == "1.0.0"`, `source.media_type == "application/pdf"`,
    `source.sha256` matches the fixture bytes, `coordinate_system == {centipoint, top-left}`,
    `capabilities.tables == false`.
  - **`bbox` is `[x0, y0, x1, y1]`** in integer centipoints, with `x1 > x0` and `y1 > y0`. A
    zero-area box is a hard error **in the engine** — note this is *stricter* than the oracle, which
    accepts `x0 == x1` (`ethos-core/src/geom.rs:87` rejects only `x0 > x1`). Stricter-on-emission is
    the safe direction and the only one permitted; see `01-CONTRACT.md` §11.
  - **Pages are 1-indexed**, and `rotation ∈ {0, 90, 180, 270}` with `synthetic/rotation-90`
    asserting `90`.
  - **Geometry-absent nodes are omitted from the grounding artifact and counted** in a declared
    limitation. No node is emitted with a fabricated box — assert the count is non-zero on a fixture
    that triggers it, and that the representation still contains the node with its `NativeLocator`.
  - **A dedicated absent-metrics fixture exists and exercises the omit-plus-count path.** A PDF whose
    font provides no usable ascent/descent and no `/FontBBox`. `simple-text` does not test this and
    must not be the fixture that stands in for it. Authored under CC0 into this repo's own fixture
    set — this is the one case where the corpus grows beyond the Ethos manifest, because Ethos has no
    fixture for it.
  - **Omission is only ever for missing measurable geometry.** Asserted structurally: the omission
    path is reachable only from the typed-absence variant, and a test proves that a node with
    measured geometry is never omitted for any other reason — not a classifier reason code, not a
    quality judgement, not a page state. `grep` the omission call site: it takes a typed absence, not
    a boolean.
  - **Lossiness is asserted, not assumed**: a test documents that derivation class, `mcid`,
    structural locator, synthesized flags, and coverage state do **not** survive the projection —
    so nobody later mistakes a grounding round-trip for proof the representation is intact.
  - **Determinism**: representation and grounding artifacts are byte-identical across two runs for
    every fixture.

- **Review checklist:**
  - [ ] Grounding output contains exactly the schema's fields and nothing else
  - [ ] No PDF type reachable from `ethos-parser-grounding`
  - [ ] Node IDs are stable within a profile and documented as **not** globally stable
  - [ ] `TODO(re-read DocumentRepresentation v0 field list)` markers resolved or still explicitly open
  - [ ] Representation fingerprint and source fingerprint are distinct fields with distinct meanings
  - [ ] Geometry omission is declared with a count, never silent
  - [ ] The omission call site takes a **typed absence**, not a boolean or a reason code — so it
        cannot be reached by a quality judgement
  - [ ] The absent-metrics fixture is CC0, authored here, and documented as engine-owned in
        `fixtures/manifest.json`

- **Depends on:** M4.

---

## M6 — `grounding-check` validator + double-run byte identity

- **Goal:** Agreement with the oracle. The engine's validator and Ethos's read the same artifact and
  say the same thing, byte for byte, across the whole corpus.

- **In:** `grounding-check` implementing **structure and source-byte binding only**, via
  `--source-artifact`. *(Corrected at M6: this line said "JSON Schema validation only". The schema
  is necessary and not sufficient — Ethos's parser enforces id uniqueness, reference resolution,
  page ordering, boxes inside their page, capability/array agreement and offset validity, none of
  which JSON Schema expresses, and a schema-only checker would disagree with the oracle it is
  required to match. The engine mirrors the parser. This is still nowhere near verification.)*; the `ethos.grounding_validation.v1` report shape; `crates/ethos-parser-cli/tests/oracle.rs` extended to
  all 15 fixtures; the double-run byte-identity harness.

- **Out:** **Any verification semantics whatsoever.** `grounding-check` validates structure and
  binding. It does not check claims, does not emit `grounded`, does not compute `evidence_tier`, and
  does not re-derive anything from an Ethos report. Reimplementing verifier semantics is how a second
  authority is born by accident.

- **Artifacts / APIs:** `ethos-parser grounding-check <file> [--source-artifact <pdf>]` emitting
  `ethos.grounding_validation.v1`: `structure` (`valid`|`invalid`), `source_binding`
  (`matched`|`mismatched`|`not_checked`), `representation_sha256`, `counts`
  (`{pages, elements, spans, tables}`).

- **Acceptance tests:**
  - **Oracle agreement across all 15 fixtures**: for each, `ethos-parser grounding-check` and
    `ethos grounding check <file> --source-artifact <pdf>` agree **byte-identically** on `structure`,
    `source_binding`, `representation_sha256`, and `counts`. Any disagreement fails CI with a diff.
  - **`source_binding` trichotomy**: `matched` with the correct PDF; `mismatched` with a different
    PDF; `not_checked` with no `--source-artifact`. One test each.
  - **`structure: invalid`** on a deliberately malformed grounding artifact, with the engine and
    Ethos agreeing it is invalid.
  - **Double-run byte identity**: the full `classify → extract → ground → grounding-check` path over
    the corpus, twice, produces byte-identical **files** — not merely identical payloads. Volatile
    diagnostics are off by default.
  - **Oracle absence is loud**: with the `ethos` binary unavailable, the test **fails** with a named
    error. It never skips, never passes vacuously, never emits a stub report.
  - Exit codes: **0** valid *(and matched, or not checked)*, **1** invalid structure *or a source
    that does not bind*, **2** could-not-read. Ethos returns 2 for the middle two; the engine keeps
    them apart and both agree on zero-versus-non-zero, which is what a shell predicate reads.

- **Review checklist:**
  - [ ] `grounding-check` contains no verification logic — grep for claim, verdict, `grounded`,
        `evidence_tier`
  - [ ] Oracle failure is loud and never degrades to a skip
  - [ ] Byte identity asserted on files, not on parsed values
  - [ ] All 15 fixtures are in the harness, including the ones that exit 2
  - [ ] Nothing is re-derived from an Ethos report — reports would be relayed verbatim if relayed at
        all (and at v0, they are not)

- **Depends on:** M5.

---

## M7 — CLI, library freeze, v0 exit criteria green

- **Goal:** v0. The public surface is what it will be, the exit criteria are green as CI jobs, and
  the fuzz and mutation layers are running.

- **In:** The four subcommands finalized with their exit-code mapping; the library API surface
  reviewed and frozen; `--diagnostics` as opt-in with volatile data excluded from fingerprints;
  `cargo-fuzz` target on the PDF entry point; mutation testing over every fixture; the classification
  bound test wired into CI; README and `docs/` cross-links updated; `03-V0-SCOPE.md` §5 checklist
  green line by line.

- **Out:** Any new capability. Any performance claim. Any published benchmark table. SDKs, MCP, WASM.

- **Artifacts / APIs:** `ethos-parser {classify|extract|ground|grounding-check}`; the frozen
  `ethos-parser-core` / `ethos-parser-pdf` / `ethos-parser-grounding` public API; a tagged v0.

- **Acceptance tests:**
  - **Every line of `03-V0-SCOPE.md` §5 is a green CI job.** Not a review judgement — a job.
  - **CLI is a thin shell**: every subcommand behaviour is reachable through the library, proved by
    library-level tests that do not invoke the binary.
  - **`cargo-fuzz`** runs clean over the corpus seed set; a discovered panic is a release blocker.
  - **Mutation testing** covers every fixture; surviving mutants are triaged, and any survivor
    indicating an assertion that passes for the wrong reason is fixed.
  - **Classification bound test** in CI: 500-page at N=8 within 20% of 8-page at similar bytes/page.
  - **`grep -ri confidence`** over the whole tree's public surface returns nothing.
  - **No verification code exists** anywhere in the tree — asserted by a grep test in CI, not by
    inspection.
  - **`cargo deny check`** green.
  - Double-run byte identity green across the corpus.

- **Review checklist:**
  - [x] Public API reviewed deliberately — every exported item is intended to be supported.
        `docs/PUBLIC-API.md` is the list; `public_api.rs` fails if the two disagree
  - [x] No performance number appears in README or docs without a committed harness producing it.
        The README's only quantitative claim is the bound-test counter, which
        `v0-classify-bound` prints
  - [x] No competitor comparison, ranking, or bake-off table anywhere
  - [x] Every `03-V0-SCOPE.md` §5 item maps to a named CI job — and `v0_exit_criteria.rs` checks
        the mapping in both directions
  - [x] Version and `parser_version` are consistent at **0.1.0**. **Not tagged:** tagging is a
        decider step, so the freeze is in-tree and the tag is a separate deliberate act
  - [x] `07-VERIFY-BOUNDARY.md` still describes reality — re-read at M7; nothing
        verification-shaped shipped, and the `v0-no-verify` grep is now a job

- **What M7 actually changed**, for the record, since "freeze and prove" reads like a no-op:
  - `--diagnostics`, opt-in, stderr-only, structurally outside every fingerprint
  - `ethos-parser-pdf`'s parsing machinery narrowed to `pub(crate)`; the narrowing surfaced five dead
    items the compiler could not previously see, two of which were genuinely unread state
  - a fixture-mutation suite over all 23 manifest fixtures, with survivors pinned and triaged —
    one triage finding was a mutation that "applied" to the two NIST benchmarks by matching bytes
    inside a Flate stream, which would have gone green while proving nothing
  - `Interpreter::run` now discards partial output on refusal. Nothing downstream was ever wrong,
    but the guarantee was the call site's rather than the type's, and the test meant to cover it
    passed for the wrong reason
  - workspace 0.0.0 → 0.1.0, which moves `profile_sha256` by design

- **Depends on:** M6.

---

## Cross-milestone standing rules

These apply to every PR in every milestone. They are repeated here because they are the ones that get
violated under deadline pressure.

1. **No public confidence field, ever** — `01-CONTRACT.md` §9
2. **No box derived from a font size** — typed absence instead
3. **No silent drop** — no text, no operator, no page, no character disappears without a typed
   diagnostic
4. **No invented coordinate, identifier, fingerprint, or pagination** — Workbench rule 3
5. **Fail closed, and distinguishably** — three exit codes, named errors
6. **Byte identity is a test, not an aspiration** — two runs, identical files
7. **The Ethos tree is read-only** — contracts, fixtures, and the CLI oracle; never an edit
8. **No verification code in this repo at v0** — `07-VERIFY-BOUNDARY.md`
