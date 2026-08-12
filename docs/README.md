# ethos-engine — implementation documentation

**Status:** **M2 complete.** `engine-core` owns the contract types (M1); `engine-pdf` now opens
PDFs and classifies them — reason codes on two orthogonal axes, per-page counts, bounded sampling,
three exit codes — and `engine classify` emits a canonical artifact. No text extraction exists.
**Next: M3.**

---

## If you are a coding agent, read this first

1. Read `00-NORTH-STAR.md` — what the product is, and the fourteen decisions that are already made
2. Read `01-CONTRACT.md` — the artifact shape. Frozen before implementation, deliberately
3. Read `03-V0-SCOPE.md` — what is in and out of the first release
4. Read `05-MILESTONES.md` — the ordered work with acceptance tests
5. **Implement M3.** M0, M1 and M2 are done and committed; do not re-author the workspace, the
   contract types, or the classifier. M3 adds extraction to `engine-pdf` using the same
   `Document` handle `classify` already takes — it must not reopen the file

**Before you touch anything, run the gate** so you know the baseline you inherited:

```bash
cargo test --workspace --locked -- --skip oracle_agrees_on_simple_text
```

Green, with `oracle_agrees_on_simple_text` failing when unskipped, is the correct state — not a bug
to fix. That one exclusion holds until M6.

**`engine-core` is closed to format concepts.** M2 and M3 add PDF work in `engine-pdf`, which
depends on `engine-core` and never the other way round. A test scans `engine-core`'s sources and
fails if a PDF import, a float outside `quantize`, or the token `confidence` appears.

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
| [`draft-schemas/`](draft-schemas/) | DRAFT JSON Schemas for the M1 types and the M2 classification artifact. Not a shipped contract | When you need the wire shape of an `engine-core` type |
| [`reference/`](reference/) | The research archive the above was derived from | To check the evidence behind a decision |

---

## Milestones at a glance

| ID | Milestone | Depends on | State |
| --- | --- | --- | --- |
| **M0** | Repo skeleton + toolchain + deny + failing oracle harness | — | **done** |
| **M1** | Contract types + c14n/quanta + `schema_version` on the wire | M0 | **done** |
| **M2** | Classify: reason codes, two axes, counts, three exit codes, bounded sampling | M1 | **done** |
| **M3** | Extract: text runs, `NativeLocator`, font ids, fail-closed operators, synthesized flags | M1 | **next** |
| **M4** | Capabilities + typed absence + explicit multi-column limitation | M3 | — |
| **M5** | `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter | M4 | — |
| **M6** | `grounding-check` validator + double-run byte identity on the corpus | M5 | — |
| **M7** | CLI + library freeze + v0 exit criteria green | M6 | — |

M2 and M3 both depend only on M1. M2 is done; M3 is next. Everything else is a chain.

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
  in a future `ethos.grounding.v1` revision. v0 declares and omits (`01-CONTRACT.md` §11). Open;
  does not block M2–M4.
- ~~`TODO(M3)` — `BackendIdentity::default().version` placeholder~~ **Closed at M2.** `lopdf` is now
  a real dependency and the profile carries its resolved version, `0.44.0`. The pinned profile
  digest moved as a result, which is the mechanism working: a backend change is fingerprint-visible.
- **`not_detected` is M2's stand-in for capability declarations.** The classification artifact
  declares `garbled` and `multi-column` as reasons it never emits, with the reason why. The full
  capability/limitation machinery lands at **M4** and should absorb this list rather than sit
  beside it.
