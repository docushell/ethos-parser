# Open work

**What is pending, and what each item waits on.** A ledger, not a plan and not a decision record.
[`02-ROADMAP.md`](02-ROADMAP.md) owns the versions and [`00-NORTH-STAR.md`](00-NORTH-STAR.md) owns
the decisions; this page only tracks what is not done yet, so that nothing measured and left open is
lost between sessions. When an item moves, move it here in the same commit.

**Written 2026-09-16, against `main` at `fae4c90`.** v0.57.0 is the latest release. `main` carries
unreleased work for 0.58.0: the reader-defect fixes and `char_offsets`, both from
[`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md), and `classify`/`overlay` under the source ceiling.

**How to read the status column.**
- **ready** means nothing but effort stands in the way.
- **owner** means it waits on a decision only the owner makes.
- **evidence** means it waits on a measurement or a corpus nobody has yet.
- **blocked** names the item it waits on.

**Percentages are given only by gate clause**, because the remaining work is unscoped and a figure
by effort would be invented.

---

## 1. In progress

| Item | Status | Detail |
| --- | --- | --- |
| **Cut 0.58.0** | **in progress** | Owner accepted both of its wire changes on 2026-09-16: `capabilities.char_offsets` turns on, and the new typed absence `not_axis_aligned`, which 0.57.0 readers refuse. Remaining: CHANGELOG entry, version bump, `profile_sha256` re-pinned at 0.58.0, preflight, macOS artifacts, then tag and GitHub Release (docs/RELEASING.md §8). **Until it is cut, a build of `main` says `parser_version` 0.57.0 under a different `profile_sha256` from the released 0.57.0.** |

## 2. v2.2 — layout and accessibility (a roadmap version)

**Gate: 1 of 2 clauses met.**
- **Clause one, met at 0.42.0:** a region is emitted wherever the cut divided a page and nowhere
  else.
- **Clause two, not started:** a tag this engine writes is one it can read back and ground against.
  It was reopened by decision #23.

Everything else filed under v2.2 has shipped: D4-S0 to S4, the `v2.2-S0` to `S8` labels (0.43.0 to
0.50.0) and the block cut (0.55.0). None of it moves clause two.

### 2.1 Clause two: auto-tagging

| Item | Status | Waits on |
| --- | --- | --- |
| Scope document for the second half | ready (owner starts it) | North Star §5 calls it "the next thing owed". Nothing technical blocks writing it |
| Private `/O` owner name and the attribute keys and values under `/A` | owner | The scope document. #23 binds the row to that attribute surviving |
| Which wire object says a tag over `Extracted` text was `Computed` | owner | The scope document. `PdfTagged` locators have no slot for it, and a node carries a single derivation |
| Read-back of the attribute | blocked | The two above. `structure.rs`'s `attribute_dicts` does less than the docs say (§6): it reads no `/O`, drops indirect references, and is reached only for `/TD` and `/TH` cells, never for `/P` |
| Milestones document | blocked | The scope document (roadmap: "waits until its second half is scoped") |
| Writer subcommand: structure tree, MCID marking, `/ParentTree`, filling absence only | blocked | The milestones document. #23 makes it its own subcommand, never a side effect of `extract` |
| Read-back test: write a tag, re-extract, confirm it reads as `Computed` and grounds | blocked | The writer |
| #23's re-refusal measurement: does a consumer treat an engine-written `/P` as author structure? | blocked | The writer, plus a named external consumer to test against |

### 2.2 The block cut (shipped 0.55.0, unfinished)

| Item | Status | Detail |
| --- | --- | --- |
| A consumer for `TextRunAttributes.block` | blocked | Auto-tagging is the only consumer any document names. Nothing reads the field outside tests |
| Wire tests for the two hops that carry `block` | ready | `extract.rs` sets it and `represent.rs` carries it into the representation; neither hop is pinned by a test |
| Declare the cut's limits on the artifact | ready | It has no indent branch, and recall was measured on one document only (63.7%). Both are stated only in `blocks.rs` comments |
| A second labellable document with a different paragraph-break pitch | evidence | `19-BLOCK-SUBDIVISION-SCOPE.md` §11.4. It would reopen the 1.60× vs 1.15× choice |
| Re-test the cut's normaliser now that type size is readable (em- vs leading-normalised gaps) | evidence | Needs the item above for more than one document |

### 2.3 The local plan's other v2.2 definition

`.plans/v22-v23-implementation-plan-v3.md` §10 is untracked. It defines "v2.2 done" differently, as
three conditions, and **0 of 3 are met**:
- a refused table candidate emits something a retrieval pipeline can use — refused by the plan's own
  §5.3 T2;
- TEDS re-measured and published as a band — not started, and **ready**;
- the block cut has a consumer — blocked on auto-tagging.

**Owner:** reconcile that definition with the roadmap's two clauses, or retire it.

## 3. v2.3 — **not a roadmap version**

v2.3 exists only in the untracked plan. Decision D4 there, to create a roadmap row, was never
recorded, and `02-ROADMAP.md` says no new version numbers get invented. Its items are tracked here
because they are real work whatever version they end up in.

**The plan's "done": 0 of 3 met.**
1. Prebuilt binaries on three platforms: macOS only.
2. A quote grounded to its own glyphs: needs word boxes, which doc 22 refused. It is unreachable at
   the Ethos v0.6.0 pin without a verifier change outside this repository.
3. Byte-identical on all three: only the instruction-set axis (aarch64 against x86_64 under Rosetta,
   both on macOS) is checked.

| Plan item | Status | What remains, and what it waits on |
| --- | --- | --- |
| 6.1 Cross-OS byte identity | **ready** | CI's `check` job runs the suite on Linux, but no artifact digest is compared across operating systems. Needed: a job that builds on ubuntu, macOS and Windows and compares digests over the gate, engine and conformance fixtures, with a guard against every comparison being a refusal. A separate job need not appear in `ci/gate.sh`. Also check whether the gate harness runs on Windows at all |
| 6.2 Memory ceiling | mostly done | The published sizing rule over-predicts the worst case by 44% (a choice to change it). Whether a caller-settable byte ceiling or a PDF decompression ceiling is wanted (owner). Measure the `--max-pages` floor: the structure tree's share of memory under the page budget (ready). Verify-path wall time: option B (hash the input span) or C (lossless parse, then B) (owner; option A shipped in 0.55.0) |
| 6.3 GitHub Releases, three platforms | **ready** | Linux and Windows release binaries. Runners now run on the public repository. Labelling them `verified` rather than `compiled` depends on 6.1 |
| 6.4 PyPI wheel | owner | D4 and D8. **Irreversible** |
| 6.5 npm platform binaries | owner | D4 and D8. **Irreversible** |
| 6.6 Word-level boxes | refused | `22-WORD-BOXES-SCOPE.md` §7 names what reopens it |
| 6.7 `locate(representation, quote)` in CLI, MCP and SDK | owner | D7 |
| 6.8 crates.io, five crates in order | owner | D4. Last in the ordering. **Irreversible** |
| 6.9 Password, page sets, `continue_on_page_error`, batch, colour | unscoped | Each knob needs an MCP line and an SDK line. Batch sizing leans on a contested reading of liteparse's `batch-parse` |

## 4. Owner decisions pending

| Decision | Blocks |
| --- | --- |
| Start v2.2's second half (the scope document) | §2.1 entirely |
| The `/O` owner name, the `/A` attribute shape, and which wire field carries `Computed` for a written tag | The writer and the read-back |
| **D4:** create a v2.3 roadmap row (amends `02-ROADMAP.md`'s "no new version numbers" and #15's order) | 6.4, 6.5, 6.8 |
| **D8:** may the Python and Node packages ship a platform binary (reverses v1.2-S2)? | 6.4, 6.5 |
| **D7:** does `locate(representation, quote)` cross the verify boundary (`07-VERIFY-BOUNDARY.md`)? | 6.7 |
| **D1:** reverse P14 narrowly for roles on geometric blocks | Plan §5.4 |
| **D2:** reverse L29 to infer headings from size or font name. The plan's default is to defer | — |
| **D3:** publish a head-to-head comparison, which reverses O26. Running one needs no decision | Publication only |
| How a text-run box declares its kind. `docs/01-CONTRACT.md` §5.3 records it as open, and §6's versioned rule for the box is not in the profile | Contract §5.3 and §6 |
| Retire the unruled table rule, and whether `table_detection.unruled` moves off `unruled-align-v1` after the precondition reorder | Table track |
| The geometric table rework (ruled-rects-v4 to v6) was the owner's choice in the plan only. Record it under #18, because `table-gate-v1.md` still says "the geometric chase stays parked" | Governance |
| Record or withdraw the plan's D1 to D8. The plan says "#24 onward", but #24 went to the MCP ledger | Governance |
| Semver for byte-identical but source-breaking type changes: the `Arc` in `StructuralLocator::PdfTagged`, and `GeometryAbsence` growing without `#[non_exhaustive]` | Release policy |
| `font_size` stays the raw `Tf` operand, and `advance` stays in its pre-rotation frame: keep, document, or change | Wire meaning |
| The research memos in the root commit `f04c488`, reachable from public `main` and three immutable releases. Removal means rewriting every SHA | **Irreversible** either way |
| Repository security settings: secret scanning, push protection and dependabot are off, and org-wide 2FA is not required | Settings |

## 5. Ready now — no decision needed

- **6.1:** the cross-OS digest job; then **6.3**, Linux and Windows release binaries.
- **Re-measure the table band on the current build.** The accuracy harness prints macro cell-F1
  **69‰** where `table-gate-v1.md` records **70‰** (cfpb 244‰ against 259‰). The gap is older than
  0.58.0's work. No twelve-document F1 was re-stated after ruled-rects-v4 to v6, although
  `CAPABILITY.md` and `docs/README.md` quote the band.
- **Re-run opendataloader-bench and OmniDocBench** as instruments after 0.58.0. The roadmap row
  still quotes 0.44.0 figures.
- **A geometry digest check in the gate.** No committed golden covers geometry, so a box change is
  caught only by tests that read boxes.
- **CI coverage of the two forbidden-token greps.** They are PR-only jobs, and no PR has run since
  2026-09-05 under the local-merge flow. Move them into `check`, or open PRs.
- **The doc fixes in §7.**

## 6. Known defects — recorded, not fixed

| Defect | Where it is recorded |
| --- | --- |
| `advance` on a CTM- or `/Rotate`-turned page is measured before the rotation, so it disagrees with the box and with the contract's "after page rotation" | `PdfLocator::advance` rustdoc; doc 22 amendments |
| A run dropped at an undecodable code stops advancing the pen at that code, so later runs on the line land left of where they are drawn | Doc 22 amendments (word-spacing item) |
| A code whose `/ToUnicode` destination is empty decodes to no character and can hide a ligature from `scalar_code_mismatch` | `ShownText::code_advances` comment |
| Office markdown and html stamp the PDF default profile's `profile_sha256`, not the profile that produced the representation | `409102d` commit message |
| On `nist-sp-800-53Ar5` page 47, 23 characters of a turned table header (`Assessor /`, `Assessment Team`) are absent from the extract | `docs/measurements/rotated-text/README.md` |
| DOCX text inside `mc:AlternateContent` may be emitted twice, once from `mc:Choice` and once from `mc:Fallback` | A review lead only — confirm with a fixture |
| Bidi / right-to-left text is untested | Add a fixture before deciding anything |

## 7. Documents that say something no longer true

| Where | What is wrong |
| --- | --- |
| `02-ROADMAP.md` (v2.2 questions row) and `19-BLOCK-SUBDIVISION-SCOPE.md` | Say block subdivision is "not scoped", yet `block` shipped at 0.55.0. Neither names the field or the release, and the cut shipped without the scope document the roadmap requires before code |
| `00-NORTH-STAR.md` row 23 | Lists the CLI as five read-and-project subcommands. It has nine, and `overlay` already writes a PDF: an annotated copy with a private catalog dictionary. What does not exist is a tag writer |
| `00-NORTH-STAR.md` row 23, `02-ROADMAP.md` (v2.2 and auto-tagging rows), `16-D4-SCOPE.md` §7, `19-BLOCK-SUBDIVISION-SCOPE.md` | Say `structure.rs` "already parses this exact shape". See §2.1 |
| `00-NORTH-STAR.md` §5 against `16-D4-SCOPE.md` §7 | The first calls the auto-tagging scope document "owed"; the second says it is "allowed" and "nobody has" started |
| `16-D4-SCOPE.md` §9 | The S0 row says "the milestones — done", but its header says no milestones document exists |
| `table-gate-v1.md` header | Says "the geometric chase stays parked" above its own ruled-rects-v4 to v6 sections |
| `RELEASING.md` | Says "on a private repository". Its advice to reverse a release by deleting it has not been checked against immutable releases |
| `measurements/cross-architecture/README.md` | Says CI cannot allocate a runner. That no longer holds for push-triggered jobs |
| `draft-schemas/derivation-class.draft.json` | Still says "ink boxes from font metrics", which contract §6 no longer says |
| `04-ARCHITECTURE.md` | Names `ttf-parser`; the engine reads fonts with `skrifa` |
| `history/11-V11-MILESTONES.md`, `13-V12-MILESTONES.md`, `14-V2-SCOPE.md`, `15-V2-MILESTONES.md` | Still say "v1 is not done", which #18 settled. Only `09-V1-MILESTONES.md` carries the pointer |
| `fixtures/README.md`, `fixtures/manifest.json` (measured-ink-box note), `crates/ethos-parser-pdf/tests/robustness.rs` (the "pinned set is 64" prose) | Stale fixture counts |
