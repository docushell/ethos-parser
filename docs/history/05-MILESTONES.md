# 05 — v0 milestones

**All seven complete. v0 shipped at 0.1.0 and is frozen.** This is the historical record and the
acceptance list each milestone was actually held to. Work after this is versions, not M-numbers.

The order was not a suggestion: M1 froze the types M3 emits, M4 declared what M5 projects, and M6
could not exist before M5.

| ID | Milestone | Gist |
| --- | --- | --- |
| **M0** | Repo skeleton, toolchain, dependency policy, a failing oracle harness | The harness exists and fails honestly |
| **M1** | Contract types, canonical JSON, quanta, versions on the wire | The artifact shape, frozen in code |
| **M2** | Classify: reason codes, two axes, counts, three exit codes, bounded sampling | Observation without judgement |
| **M3** | Extract: text runs, locators, font ids, fail-closed operators, synthesized flags | The evidence itself |
| **M4** | Capabilities, typed absence, declared limitations | The L1 gate |
| **M5** | The representation, and the grounding projection | The canonical record and what a verifier reads |
| **M6** | The `grounding-check` validator, and double-run byte identity | Agreement with the oracle |
| **M7** | CLI and library freeze, exit criteria green | v0 |

---

## M0 — Skeleton and a failing harness

**Goal:** a workspace that builds, a licence posture that is enforced rather than intended, and a
test that fails for the right reason. **The first commit contained the oracle test and one fixture,
failing.** Nothing else belonged in week one.

**In:** pinned toolchain; the workspace and its empty crates; `deny.toml` (permissive licences only,
no AGPL, no network crates); the fixture manifest referencing the Ethos corpus by path and hash; the
oracle test with one fixture, failing; CI running build, test, clippy, fmt, `cargo deny`, plus a job
that **proves** the AGPL gate rejects.

**Out:** any parsing; any type beyond what the harness needed to compile; fixture *copies*.

**What it was held to:**

- The gate was stated as two conditions, because a bare `cargo test` exited non-zero at M0 *by
  design*: the suite green with the oracle test excluded, **and** that test failing with a named
  diagnostic. **Resolving it by making the bare command green was forbidden** — `#[ignore]`, deleting
  the test, or weakening it to a `return` each destroys the milestone, and the test's own diagnostic
  said so.
- The AGPL probe fails with **exit code 4 naming the crate**, proving the gate fired for *that*
  reason. Any non-zero exit would also match a config typo.
- The oracle harness locates the verifier binary and **errors loudly if absent** — never skips. And
  `ETHOS_BIN` is authoritative rather than a hint: set to a non-existent path, the harness fails hard
  rather than falling back. Resolving silently to a verifier nobody chose is worse than finding none,
  because the operator believes they know which one answered — and the fallback binary was in fact a
  *stale* version, not merely a different one.
- A mutated fixture hash fails the check; an absent corpus fails by name.
- Env-dependent behaviour is tested through a parameterised function, never by mutating process env,
  which races the threaded test harness and corrupts sibling tests.

## M1 — Contract types, canonical JSON, quanta

**Goal:** the contract expressed as Rust types with the canonicalization that makes them byte-stable.
**After M1 the artifact shape stopped being negotiable and started being a compile error.**

**In:** artifact identity; the profile type and its hash; the coordinate system; canonical JSON;
`quantize` with the error cases; the rectangle type; derivation classes; typed-absence variants; the
error taxonomy; draft schemas.

**Out:** production schemas under a shipped path; any PDF concept; any grounding projection.

**What it was held to:**

- **Canonical-JSON idempotence**, property-tested.
- **Float rejection**: any non-integer number anywhere in a canonical value is a hard error, not a
  rounding — including nested in arrays and objects.
- **Key ordering** by code point, holding even with a `preserve_order` feature forced on in the
  dependency graph. **The test first proves the feature is actually active**, by asserting plain
  serialization emits insertion order; otherwise it passes vacuously under a map that sorts for free,
  which is worse than having no test.
- **Quantize vectors**, including `-0.0 → 0`, with `NaN`, infinity and overflow all erroring.
- **Profile sensitivity**: mutating each profile field in turn changes the hash, asserted field by
  field. And separately, **the default profile is pinned by bytes and digest** — the first proves a
  change is *detectable*, the second proves it was *intended*.
- **No confidence token** in the public API, enforced by a scan with comments stripped, plus a
  self-test proving the comment stripper works so the scan cannot pass vacuously.
- **Every nested object in a hashed type denies unknown fields.** The derive does not recurse, and
  guarding only the outer struct leaves a dropped nested knob re-hashing to the unmodified digest.
- **A rule stated over a pair of values is tested as a matrix, not from one side.** A function that
  ignores an argument passes every single-sided test.

## M2 — Classify

**Goal:** a classifier that reports what it saw and refuses to render a verdict. **The caller owns
the policy; the engine owns the observation.**

**In:** two independent reason enums — OCR-need and layout-hard; per-page counts, 1-indexed; the
derived boolean; bounded sampling with the sample count as a pinned profile field; three exit codes;
content-based format detection.

**Out:** any confidence float; any single verdict field; any routing decision; OCR itself;
multi-column *handling* — the reason code exists, the reading order stayed single-column until v1.

**What it was held to:**

- **Bounded cost, the load-bearing test, in two parts.** A counter asserting pages scanned equals the
  sample bound, measured on a 492-page document as **8 scanned**. And timing that is flat in page
  count: 246× the pages for 1.7× the classify time.

  The two phases are timed **separately**, and that is a measurement rather than a concession: the
  backend parses the whole object graph eagerly, so total cost is parse plus sample-count times
  per-page, and the parse term scales with bytes. Measured: open 16/48/431 ms against classify
  14/21/25 ms for 2/80/492 pages.
- **Three exit codes, one fixture each, all distinguishable** — including a missing file. Assert the
  codes, not the messages.
- **Axis independence**: a fixture producing a layout reason and no OCR reason emits an empty
  OCR-need list. The chosen fixture is a tax form — heavy ruling lines with crisp born-digital text —
  which is also why it is *not* the exit-0 case: it fires layout reasons, so it exits 1. **Suppressing
  a true layout reason to make a document line up with a doc example is exactly the tuning this
  project refuses**, so the fixture choice changed instead.
- **Boolean derivation** from the reason lists, with a property test showing the boolean carries no
  information the lists do not.
- **The simplest fixture's behaviour is recorded, not tuned to match anyone.** One competitor calls it
  text-based with zero text pages; another calls it text-free and demands OCR. Neither is the target.
  Measured here: one page, one text operator, 11 text bytes, no imagery, **no reasons on either axis**
  — because short text without competing content is a short page, not a sparse one.
- **The reasons with no sound detector are never emitted, and the artifact says so.** Silence would
  let a caller read an empty list as evidence of absence. An acronym-dense document is asserted
  **not** to be reported as textless — the compounding-garble trap.
- **Unknown magic bytes fail closed** with a named error and exit 2.

## M3 — Extract

**Goal:** the evidence itself. Position-aware text runs whose origins are trustworthy, whose boxes are
measured or absent, and whose parse **stops rather than silently drops content**.

**In:** content-stream interpretation with the operator set **enumerated explicitly**; text runs
carrying origin, advance, font id, font size, page and marked-content id; a native locator on every
run; measured ink boxes from the embedded font program with a descriptor fallback; encoding tables;
synthesized flags; glyph codes with the ligature caveat; single-column reading order as a versioned
computed rule.

**Out:** multi-column reading order; tables; any box derived from a font size; any operator handled by
"ignore and continue"; Markdown.

**What it was held to:**

- **Both quote-form show-text operators are handled**, with a fixture proving text is not lost. This
  is the disqualifying defect in a surveyed parser: the operator is absent from its match, the text
  vanishes silently, and surrounding runs merge with corrupt geometry.
- **An unknown operator fails closed** with a named error and exit 2. A stream containing an undefined
  operator must not produce a well-formed artifact.
- **Horizontal scaling is applied** to advance width, with a fixture. A surveyed parser does not
  implement it anywhere, so its widths are wrong on any document that uses it.
- **No box is derived from a font size.** A font with unavailable metrics produces typed absence,
  never a fallback box.
- **Synthesized characters are flagged where created**, and the flag survives canonicalization.
- **The ligature caveat is declared, not silently reconciled** — expansion yields more scalars than
  codes, so the arrays are not 1:1 and the artifact says so.
- **Determinism**: extracting twice produces byte-identical output for every fixture that opens.

## M4 — Capabilities and the L1 gate

**Goal:** close the L1 gate. **An artifact that does not declare its capabilities has not reached
"extracted", regardless of how good its text is.**

**In:** capability declarations per profile; limitation declarations per document; per-page processing
state; the coverage summary; the partial-processing terminal state.

**Out:** any capability declared `true` that is not tested; any limitation that exists only in a doc
comment; repairing anything.

**What it was held to:**

- **Every declared capability has a passing test.** A capability asserted `true` with no test is a
  build failure.
- **Partial processing is terminal and visible.** A document with one failed page produces a coverage
  summary showing the gap, and the artifact is **not** presentable as complete.
- **Absence is never `1.0`.** A processor reporting no uncertainty emits an absent field.
- **Capability-limited beats negative.** A query against an unprocessed page returns
  capability-limited, never "not present" — **absence of extractable content is never evidence of
  absence in the source.**
- Changing the capability set changes the profile hash.

## M5 — The representation and the grounding projection

**Goal:** the canonical record, and the projection a verifier consumes. This is where the engine's
output first became something another system can read.

**In:** the full representation — identities, processing run, ordered typed nodes with stable ids, a
required native locator, optional structural locator and geometry, per-page state, coverage; the
grounding adapter; the geometry-absent omission rule with its declared count; **one engine-authored
CC0 fixture with unusable font metrics**, because the upstream corpus has no fixture for that case.

**Out:** tables; any grounding field outside the schema; any PDF concept inside the grounding crate.

**What it was held to:**

- **Schema conformance with no additional properties**, and a field-exact projection.
- **A zero-area box is a hard error in the engine** — stricter than the oracle, which accepts a
  degenerate one. **Stricter-on-emission is the safe direction and the only one permitted.**
- **Geometry-absent nodes are omitted from the grounding artifact and counted** in a declared
  limitation, with the representation still holding the node and its locator. No node is ever emitted
  with a fabricated box.
- **A dedicated absent-metrics fixture exercises that path.** The simplest fixture does not test it
  and must not stand in for it.
- **Omission is only ever for missing measurable geometry**, and this is structural rather than
  reviewed: the omission path is reachable only from the typed-absence variant. **The call site takes
  a typed absence, not a boolean** — so it cannot be reached from a quality judgement.
- **Lossiness is asserted, not assumed.** A test documents that derivation class, marked-content id,
  structural locator, synthesized flags and coverage state do **not** survive the projection, so
  nobody later mistakes a grounding round trip for proof the representation is intact.

## M6 — The validator, and agreement with the oracle

**Goal:** the engine's validator and the verifier's read the same artifact and say the same thing,
byte for byte, across the whole corpus.

**In:** `grounding-check` implementing **structure and source-byte binding only**; the report shape;
the oracle harness extended to all 15 fixtures; the double-run byte-identity harness.

The scope line originally said "JSON Schema validation only" and that was corrected here: the schema
is necessary and not sufficient. Id uniqueness, reference resolution, page ordering, boxes inside
their page, capability agreement and offset validity are none of them expressible in JSON Schema, and
**a schema-only checker would disagree with the oracle it is required to match.** The engine mirrors
the parser. This is still nowhere near verification.

**Out:** **any verification semantics whatsoever.** No claims, no `grounded`, no evidence tier,
nothing re-derived from a verifier report.

**What it was held to:**

- **Oracle agreement across all 15 fixtures**, byte-identical on structure, source binding,
  representation hash and counts. Any disagreement fails CI with a diff.
- **The source-binding trichotomy**: matched, mismatched, and not-checked, one test each.
- **Double-run byte identity** over the whole path, producing identical **files** rather than merely
  identical payloads.
- **Oracle absence is loud.** With the verifier unavailable the test fails by name — never skips,
  never passes vacuously, never emits a stub report.
- **Exit codes keep "invalid" and "could not read" apart**, where the verifier collapses them. Both
  agree on zero versus non-zero, which is what a shell predicate reads.

## M7 — Freeze

**Goal:** v0. The public surface is what it will be, the exit criteria are green as CI jobs, and the
fuzz and mutation layers are running.

**In:** the four subcommands finalized with their exit-code mapping; the library API reviewed and
frozen; `--diagnostics` as opt-in with volatile data outside every fingerprint; a fuzz target; mutation
testing over every fixture; the scope document's exit-criteria checklist green line by line.

**Out:** any new capability; any performance claim; any published benchmark table.

**What it was held to:**

- **Every line of the v0 exit criteria is a green CI job.** Not a review judgement — a job, and a test
  checks the mapping in both directions.
- **The CLI is a thin shell**: every subcommand behaviour is reachable through the library, proved by
  library-level tests that never invoke the binary.
- **The public API is a deliberate list**, not whatever happened to be public. A test fails if the list
  and the exports disagree.
- **No performance number appears anywhere without a committed harness producing it.**

**What M7 actually changed**, since "freeze and prove" reads like a no-op:

- `--diagnostics`: opt-in, stderr-only, structurally outside every fingerprint.
- The PDF crate's parsing machinery narrowed to crate-private — which surfaced five dead items the
  compiler could not previously see, two of them genuinely unread state.
- A fixture-mutation suite with survivors pinned and triaged. One triage finding: a mutation that
  "applied" to two benchmark documents by matching bytes inside a compressed stream, **which would
  have gone green while proving nothing**.
- The content interpreter now discards partial output on refusal. Nothing downstream was ever wrong,
  but the guarantee belonged to the call site rather than to the type, and **the test meant to cover
  it passed for the wrong reason**.

---

## Standing rules

These applied to every PR in every milestone, and they are the ones that get violated under deadline
pressure.

1. **No public confidence field, ever.**
2. **No box derived from a font size** — typed absence instead.
3. **No silent drop.** No text, operator, page or character disappears without a typed diagnostic.
4. **No invented coordinate, identifier, fingerprint or page number.**
5. **Fail closed, and distinguishably** — three exit codes, named errors.
6. **Byte identity is a test, not an aspiration.**
7. **The Ethos tree is read-only.**
8. **No verification code in this repository.**
