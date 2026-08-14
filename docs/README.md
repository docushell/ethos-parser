# ethos-engine — implementation documentation

**Status:** **v1-S4 shipped, as 0.6.0.** v1 is the DocuShell replacement gate and is seven slices
long (`08-V1-SCOPE.md`); four are done.

**S4 makes a form field's value and an annotation's comment nodes of their own kind.** Neither is
drawn by any content stream, so neither is a text run — and a reader that copied them into the
text layer would make a reviewer's private note indistinguishable from the page's own words. Both
capabilities flipped true, each with a proof covering the found-and-not-found halves. Measured
first: this engine never had that defect, because extraction only ever read page content streams.

**S3 reads the document's own tagged-structure tree.** `/StructTreeRoot` is walked, `/RoleMap` is
applied where the file supplies one, and a run is bound to a role path on **exact `(page, mcid)`
equality** — nothing fuzzy. `capabilities.structural_locators` flipped to true, and what it claims
is that *this profile looks*: an untagged document gets no roles and declares
`untagged-structure-tree-absent`. Its proof test covers both halves, because a test that only
found roles could be satisfied by an engine that invents them.

A node's structural address is now one of four things, and they are four different facts:

| State | What happened |
| --- | --- |
| `pdf_tagged` | the tree cites this `(page, mcid)` — the author placed this text here |
| `pdf_mcid` | the stream gave an id and **no structure element claims it** |
| `pdf_artifact` | the page marked this as furniture, deliberately outside the tree |
| absent | the page marked nothing here |

Artifact runs stay in `nodes`, flagged — deleting running heads is an undeclared edit to the
document, and undetectable downstream.

**S1 and S2 gave tables two rules**, and every table on the wire names the one that produced it:

| Rule | Evidence | Claim |
| --- | --- | --- |
| `ruled-rects-v1` (S1) | rectangles the author painted | the document drew this grid |
| `unruled-align-v1` (S2) | where the author placed text | a detector inferred this grid |

Both are cross-checked two independent ways, and fabrication is 0 under both. S3 adds a **second**
check — `tagged-vs-geometric-v1` — comparing the grid a document's tags declare against the grid a
detector found. **The > 0.489 accuracy gate is S7's**, not any of these slices' — see
`08-V1-SCOPE.md` §3 for why chasing it earlier would be tuning against nobody's number.

Four measurements shaped the table slices and are worth knowing before reading the detectors:

1. No fixture in the Ethos conformance corpus contains a single path operator, so
   `synthetic/table-regular-grid` was an S2 fixture wearing an S1 name. It is now the S2 golden.
2. A first ruled detector fabricated a **662-cell** table on `irs-form-1040-2025`, with a cell
   spanning 75 rows by 45 columns, before the coherence precondition landed.
3. Alignment repeats that mistake with a different input: the same form's text implies **23 276**
   lattice faces on page 1 and **10 848** on page 2. The unruled coherence precondition is what
   refuses both, and the form still yields 0 tables.
4. Growing column groups until a gap appears **chains**, turning two lines of word-split prose
   into a 2 × 3 table. `unruled-align-v1` folds by tolerance instead, which cannot chain.

**Previously: v1-S3, as 0.5.0.** The tagged-structure tree, read and bound to text by exact
`(page, mcid)` equality. **And v1-S2, as 0.4.0.** Unruled tables from text alignment, under their own rule id, with
every table naming the rule that found it. **And v1-S1, as 0.3.0**: ruled tables from vector
paths, `CellSlot` occupancy, and the geometric-versus-structural locator cross-check.

**Previously: v0.1, as 0.2.0.** v0 was frozen at M7 (0.1.0) and its exit criteria have not
moved; this is the roadmap row after it (`02-ROADMAP.md`), and it is the first work that is a
*version* rather than a milestone.

**What v0.1 added**, all three from that roadmap row:

- **Citation verification as a declared capability.** `engine verify` spawns the pinned Ethos CLI
  and relays its report bytes **verbatim** — byte-identical to running `ethos verify` yourself.
  The engine still does not verify: `engine_core::verifier` has no type for a report, a claim or a
  result, so there is nothing that could re-derive one. `--fail-on-ungrounded` exits 1 with the
  report; a missing verifier exits 2 with no report and a named error.
- **Encoding-issue detection.** A font that cannot map a code drops its run and declares
  `broken-font-encoding` with a count, instead of failing the whole document as v0 did — and
  never emits a substitution character. A document that decodes *nothing* is refused outright.
- **The xref decision, written down.** `01-CONTRACT.md` §8.1 chose one bounded, declared repair
  over refusal. `synthetic/table-regular-grid` now reads, and the oracle partition moved from
  11/4 to 12/3 in the open.

The verifier's identity and the repair policy are both `Profile` fields, so `profile_sha256` moved
again — a verifier swap and a repair-policy change are now as fingerprint-visible as a backend
swap.

`engine-pdf` opens a PDF once and both classifies it (M2) and extracts position-aware text runs
from it (M3): an exhaustive operator table that fails closed, `PdfLocator` on every run, measured
or typed-absent ink boxes, synthesized-character flags, and the ligature caveat on the wire. As of
M4 every artifact also carries the **L1 gate** — declared capabilities, named limitations, per-page
processing state, a coverage summary that reconciles, and a terminal state where **partial is not a
degraded success**. As of M5 `engine extract` emits **`DocumentRepresentation v0`** — the canonical
evidence record, with a fingerprint over its own payload and geometry deliberately outside it — and
`engine ground` projects that record into `ethos.grounding.v1`, validated against a pinned snapshot
of Ethos's own schema. As of M6 `engine grounding-check` validates a grounding artifact's structure
and its binding to source bytes, and **agrees with the Ethos CLI on every fixture that reaches
one**.

M7 added no capability. It closed v0 instead:

- **`03-V0-SCOPE.md` §5 is CI.** Fifteen criteria, fifteen named jobs, and
  `v0_exit_criteria.rs` fails if a box is ticked against a job nobody wrote — or if a `--skip`
  reappears anywhere in the workflow.
- **The public API is a list**, not whatever happened to be `pub`. See
  [`PUBLIC-API.md`](PUBLIC-API.md). `engine-pdf`'s parsing machinery is `pub(crate)`, and
  narrowing it exposed dead code the compiler had been unable to see.
- **`--diagnostics`** exists: opt-in, stderr-only, outside every fingerprint.
- **Fuzz and mutation layers.** `cargo-fuzz` on the PDF entry point, and every fixture in the
  manifest damaged six ways with the survivors pinned and triaged.
- **One fail-closed hardening**, found by the mutation triage: the content interpreter kept text
  shown before an unrecognised operator. No artifact was ever wrong — `extract` propagates and
  drops the interpreter — but the guarantee belonged to the call site rather than to the type, and
  the test that was meant to cover it passed for the wrong reason. It is the type's now.

**Next: v1-S5** — multi-column reading order, with a stable versioned rule
(`09-V1-MILESTONES.md`). Not started.

---

## If you are a coding agent, read this first

1. Read `00-NORTH-STAR.md` — what the product is, and the fourteen decisions that are already made
2. Read `01-CONTRACT.md` — the artifact shape. Frozen before implementation, deliberately
3. Read `03-V0-SCOPE.md` — what is in and out of the first release
4. Read `05-MILESTONES.md` — the ordered work with acceptance tests
5. **M0–M7 are done and v0 is frozen; v0.1 and v1-S1 through v1-S4 shipped on top.** Do not re-author the
   workspace, the contract types, the classifier, the extractor, the assurance envelope, the
   representation, the projection, or the checker — and do not widen the public API without
   editing [`PUBLIC-API.md`](PUBLIC-API.md) and the freeze test in the same commit. For v1 work
   read [`08-V1-SCOPE.md`](08-V1-SCOPE.md) and [`09-V1-MILESTONES.md`](09-V1-MILESTONES.md)
   first: v1 is **seven slices**, the next is S5, and the > 0.489 gate belongs to S7 alone

   One thing M7 inspected and deliberately left alone: the `ethos` binary in the sibling tree is
   **older than its own source** (it prints the validation report bare; the ref CI pins wraps it
   in an in-toto Statement). The oracle harness reads both shapes and every oracle test passes, so
   a rebuild there is a deliberate act with its own commit — `ETHOS_ORACLE_REF` is pinned in
   `.github/workflows/ci.yml` precisely so a verifier swap is visible

**Before you touch anything, run the gate** so you know the baseline you inherited:

```bash
cargo test --workspace --locked
```

**No exclusion.** `oracle_agrees_on_simple_text` failed by design from M0 to M5 and carried a
diagnostic naming what was missing; M6 replaced the panic with the comparison it always described.
If you find yourself adding `--skip` to get a green build, stop: that test is the only thing
proving this engine and the verifier read an artifact the same way.

**`engine-core` is closed to format concepts.** All PDF work lives in `engine-pdf`, which depends
on `engine-core` and never the other way round. A test scans `engine-core`'s sources and fails if a
PDF import, a float outside `quantize`, or the token `confidence` appears.

Then, as needed:

6. `04-ARCHITECTURE.md` — crate layout, CLI surface, fixtures, dependency posture
7. `07-VERIFY-BOUNDARY.md` — **before touching anything that looks like verification**
8. `06-STEAL-REFUSE.md` — **before proposing a feature borrowed from another parser**
9. `02-ROADMAP.md` — only to confirm a v1+ idea has a home and does not belong in v0

---

## The documents

| Doc | What it settles | Read it when |
| --- | --- | --- |
| [`00-NORTH-STAR.md`](00-NORTH-STAR.md) | Product definition · the 14 forced decisions · trust-ladder ownership · relationship to the Ethos repo · anti-goals | First. Always |
| [`01-CONTRACT.md`](01-CONTRACT.md) | Artifact identity · coordinate declaration · c14n and integer quanta · locators and typed absence · derivation classes · capabilities · fail-closed · no confidence · the `ethos.grounding.v1` mapping | Before writing any type that gets serialized |
| [`02-ROADMAP.md`](02-ROADMAP.md) | v0 → v3 in one line each · folded research deltas · what is deliberately not scheduled | When an idea might belong to a later version |
| [`03-V0-SCOPE.md`](03-V0-SCOPE.md) | v0 in/out · the happy path · exit codes · fixtures · exit criteria · performance posture · risks | Before opening any PR |
| [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) | Crate layout and boundaries · CLI surface · single-load rule · profile-as-identity · fixtures and oracle · dependency posture · how later lanes plug in | When deciding where code goes |
| [`05-MILESTONES.md`](05-MILESTONES.md) | **M0–M7**, each with Goal / In / Out / Artifacts / Acceptance tests / Review checklist / Depends on | Every PR. This is the code-review map |
| [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) | The four-way steal formula · TAKE / IMPROVE / REFUSE / DEFER for the decisions that prevent bad PRs | Before borrowing anything from ODL, Anydoc, pdf-inspector, or LiteParse |
| [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) | Engine vs verifier · the staged path · BYO forever · OCR/agent/VLM boundaries · six anti-patterns | Before anything verification-shaped |
| [`08-V1-SCOPE.md`](08-V1-SCOPE.md) | What v1 is and is not · why 0.489 is measured once at S7 · the ruled/unruled split · capability flip plan | Before any v1 work |
| [`09-V1-MILESTONES.md`](09-V1-MILESTONES.md) | **S0–S7**, each with Goal / In / Out / Acceptance / Depends on | Every v1 PR. This is the v1 code-review map |
| [`PUBLIC-API.md`](PUBLIC-API.md) | The frozen v0 export list, per crate · what is internal and why · the CLI↔library thin-shell mapping | Before adding a `pub use`, or when embedding the engine |
| [`draft-schemas/`](draft-schemas/) | DRAFT JSON Schemas for the M1 types and the M2/M3 artifacts. Not a shipped contract | When you need a wire shape |
| [`reference/`](reference/) | The research archive the above was derived from | To check the evidence behind a decision |

---

## Milestones at a glance

| ID | Milestone | Depends on | State |
| --- | --- | --- | --- |
| **M0** | Repo skeleton + toolchain + deny + failing oracle harness | — | **done** |
| **M1** | Contract types + c14n/quanta + `schema_version` on the wire | M0 | **done** |
| **M2** | Classify: reason codes, two axes, counts, three exit codes, bounded sampling | M1 | **done** |
| **M3** | Extract: text runs, `NativeLocator`, font ids, fail-closed operators, synthesized flags | M1 | **done** |
| **M4** | Capabilities + typed absence + explicit multi-column limitation | M3 | **done** |
| **M5** | `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter | M4 | **done** |
| **M6** | `grounding-check` validator + double-run byte identity on the corpus | M5 | **done** |
| **M7** | CLI + library freeze + v0 exit criteria green | M6 | **done** |

M2 and M3 both depended only on M1; both are done. Everything else is a chain, and the chain ends
here: **v0 is complete at v0.1.0.** What follows is versions, not milestones — see
[`02-ROADMAP.md`](02-ROADMAP.md).

---

## The rules that get violated under deadline pressure

Repeated from `05-MILESTONES.md`. If you remember nothing else:

1. **No public confidence field, ever**
2. **No box derived from a font size** — typed absence instead
3. **No silent drop** — no text, operator, page, or character disappears without a typed diagnostic
4. **No invented coordinate, identifier, fingerprint, or pagination**
5. **Fail closed, and distinguishably** — three exit codes, named errors
6. **Byte identity is a test, not an aspiration** — two runs, identical files
7. **The Ethos tree is read-only** — contracts, fixtures, and the CLI oracle; never an edit
8. **No verification code in this repo at v0**

---

## External references

| What | Where | Status |
| --- | --- | --- |
| Research archive | `reference/` | Evidence, not roadmap. See [`reference/README.md`](reference/README.md) |
| Ethos product repo | `~/Desktop/Stuff/repo/ethos/` | **Read-only.** Contracts, fixtures, CLI oracle |
| DocuShell repo | `~/Desktop/Stuff/repo/docushell-repo/` | **Read-only.** Workbench rules, `DocumentRepresentation v0` spec |
| Ethos-in-DocuShell parser plan | `~/Desktop/Stuff/repo/ethos-docushell-parser-plan.md` | External. Architecture depth only; superseded where it conflicts with memo §16–§18 |

Never edit the Ethos or DocuShell repos from this project.

---

## Open TODOs in this tree

Grep for `TODO(` to find them. Currently:

- **`TODO(re-read DocumentRepresentation v0 field list)` — re-read at M1, mostly closed.** The
  companion settles the locator names, `TableCellPosition` (zero-indexed), `ProcessingRun`/`StageRun`,
  the node field list, the two-identity rule, and "capability-limited"; all are now used verbatim and
  tabulated in `01-CONTRACT.md` §1. **One divergence remains open and is engine-local:** the
  companion models geometry as an *optional field* and never names a variant for *why* it is absent,
  while this engine carries a typed `GeometryAbsence`. Typed absence is additive and projects down to
  an omitted field cleanly, so nothing is blocked. **Pending DocuShell review** — no live review
  round has happened.
- **`TODO(confirm with Ethos owners...)`** — whether a geometry-absent span should be representable
  in a future `ethos.grounding.v1` revision. v0 declares and omits (`01-CONTRACT.md` §11). **Still
  open, and M5 measured how much it costs**: across the whole Ethos conformance corpus, *zero* runs
  have measurable geometry, so every grounding artifact projected from it is empty of elements and
  spans. The engine reads the text fine — the record holds it, with native locators — but none of
  it can cross into a schema that requires a bbox. That is the strongest argument yet for an
  optional-bbox revision, and it is now a number rather than a hypothesis. Does not block M6.
- ~~`TODO(M3)` — `BackendIdentity::default().version` placeholder~~ **Closed at M2.** `lopdf` is now
  a real dependency and the profile carries its resolved version, `0.44.0`. The pinned profile
  digest moved as a result, which is the mechanism working: a backend change is fingerprint-visible.
- ~~`not_detected` (M2) and `not_decoded` (M3) are stand-ins for capability declarations~~
  **Closed at M4.** Both fields are gone from the wire. Everything they declared is now a
  `Limitation` in `assurance.limitations`, carrying the same reasons verbatim, alongside the
  capability-derived ones. Tests fail if either field reappears, and if any `NOT_DETECTED` entry
  loses its declaration in the move.
- **Two capabilities were narrowed at M4, and both should be revisited.**
  `capabilities.char_offsets` went `true` → `false` because v0 emits runs and no element/span
  hierarchy, so nothing exists for an offset to index into. **M5 built the record and left it
  false**: without line grouping an element and a span are the same object, so an offset would
  always be `0..len`. It flips at **v1** with grouping, with a test.
  `capabilities.structural_locators` stays `false` because an `mcid` captured from `BDC` is not a
  structural address — no role path, and an absent id is not evidence the document is untagged.
  Full structural addressing is v1. Both moved `profile_sha256`, which is the mechanism working.
- **Adobe predefined CMaps, Core-14 AFM widths and the full Adobe Glyph List are not vendored.**
  Each makes the engine fail closed with a named error, and each is declared on the wire. See
  [`vendor/README.md`](../vendor/README.md) for the reasoning and what would change if they land.
- **`skrifa` is pinned to 0.39**, not the current 0.44, because the newer tree requires Rust 1.89
  and this workspace pins 1.88. Revisit when the toolchain moves.
