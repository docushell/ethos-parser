# ethos-engine — implementation documentation

**Status:** **M5 complete.** `engine-pdf` opens a PDF once and both classifies it (M2) and
extracts position-aware text runs from it (M3): an exhaustive operator table that fails closed,
`PdfLocator` on every run, measured or typed-absent ink boxes, synthesized-character flags, and
the ligature caveat on the wire. As of M4 every artifact also carries the **L1 gate** — declared
capabilities, named limitations, per-page processing state, a coverage summary that reconciles,
and a terminal state where **partial is not a degraded success**. As of M5 `engine extract` emits
**`DocumentRepresentation v0`** — the canonical evidence record, with a fingerprint over its own
payload and geometry deliberately outside it — and `engine ground` projects that record into
`ethos.grounding.v1`, validated against a pinned snapshot of Ethos's own schema. **Next: M6.**

---

## If you are a coding agent, read this first

1. Read `00-NORTH-STAR.md` — what the product is, and the fourteen decisions that are already made
2. Read `01-CONTRACT.md` — the artifact shape. Frozen before implementation, deliberately
3. Read `03-V0-SCOPE.md` — what is in and out of the first release
4. Read `05-MILESTONES.md` — the ordered work with acceptance tests
5. **Implement M6.** M0–M5 are done and committed; do not re-author the workspace, the contract
   types, the classifier, the extractor, the assurance envelope, the representation, or the
   projection. M6 adds `grounding-check` — **JSON Schema validation only**, never verification —
   and the byte-identical oracle agreement across all 15 fixtures. Note that Ethos computes
   `representation_sha256` as a hash of the grounding **file's raw bytes**, which is not the same
   thing as the representation's own `representation_c14n_sha256`

**Before you touch anything, run the gate** so you know the baseline you inherited:

```bash
cargo test --workspace --locked -- --skip oracle_agrees_on_simple_text
```

Green, with `oracle_agrees_on_simple_text` failing when unskipped, is the correct state — not a bug
to fix. That one exclusion holds until M6.

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
| **M6** | `grounding-check` validator + double-run byte identity on the corpus | M5 | **next** |
| **M7** | CLI + library freeze + v0 exit criteria green | M6 | — |

M2 and M3 both depended only on M1; both are done. Everything else is a chain.

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
