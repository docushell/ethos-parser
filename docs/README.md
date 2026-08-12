# ethos-engine — implementation documentation

**Status:** docs bootstrap. No code exists yet. **Start at milestone M0.**

---

## If you are a coding agent, read this first

1. Read `00-NORTH-STAR.md` — what the product is, and the fourteen decisions that are already made
2. Read `01-CONTRACT.md` — the artifact shape. Frozen before implementation, deliberately
3. Read `03-V0-SCOPE.md` — what is in and out of the first release
4. Read `05-MILESTONES.md` — the ordered work with acceptance tests
5. **Implement M0 only.** Do not start M1 until M0's acceptance tests are green

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
| [`reference/`](reference/) | The research archive the above was derived from | To check the evidence behind a decision |

---

## Milestones at a glance

| ID | Milestone | Depends on |
| --- | --- | --- |
| **M0** | Repo skeleton + toolchain + deny + failing oracle harness | — |
| **M1** | Contract types + c14n/quanta + `schema_version` on the wire | M0 |
| **M2** | Classify: reason codes, two axes, counts, three exit codes, bounded sampling | M1 |
| **M3** | Extract: text runs, `NativeLocator`, font ids, fail-closed operators, synthesized flags | M1 |
| **M4** | Capabilities + typed absence + explicit multi-column limitation | M3 |
| **M5** | `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter | M4 |
| **M6** | `grounding-check` validator + double-run byte identity on the corpus | M5 |
| **M7** | CLI + library freeze + v0 exit criteria green | M6 |

M2 and M3 can run in parallel once M1 lands. Everything else is a chain.

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

- `TODO(re-read DocumentRepresentation v0 field list)` — exact field names and the typed-absence
  variant spelling, in `01-CONTRACT.md` §5.2 and §11. Closed at **M1**, against the companion spec
  and a DocuShell review round
- `TODO(confirm with Ethos owners...)` — whether a geometry-absent span should be representable in a
  future `ethos.grounding.v1` revision. v0 declares and omits (`01-CONTRACT.md` §11)
