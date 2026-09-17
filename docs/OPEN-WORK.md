# Open work

**What is pending, and what each item waits on.** A ledger, not a plan and not a decision record.
[`02-ROADMAP.md`](02-ROADMAP.md) owns the versions and [`00-NORTH-STAR.md`](00-NORTH-STAR.md) owns
the decisions; this page only tracks what is not done yet, so that nothing measured and left open is
lost between sessions. When an item moves, move it here in the same commit.

**Written 2026-09-16 against `main` at `fae4c90`; revised 2026-09-17 against the integration
branch for the version after 0.58.0, and again the same day when auto-tagging's round trip and its
measurements (S3, S4) and the writer review's fixes merged into it.** v0.57.0 is the latest published release. Local `main` at
`b4b4aa9` carries the unpublished 0.58.0 release commit (preflight green, macOS artifacts built and
verified); the integration branch carries everything this page marks *done 2026-09-16/17*.

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
| **Publish 0.58.0** | **owner** | The release commit `04bd9f2` and its merge `b4b4aa9` sit on local `main`, unpushed. `ci/release-preflight.sh` passed there (gate 9/9); `target/release-artifacts/` holds the `aarch64` and `x86_64` macOS tarballs, both `verified`; the notes are drafted. Remaining, all the owner's: push `main`, tag `v0.58.0` at `b4b4aa9`, create the release. **Until the tag exists, `parser_version` 0.58.0 names a build no release carries.** Every byte-changing commit since `b4b4aa9` belongs to the next version, so 0.58.0 stays as prepared |
| **Cut the version after 0.58.0** | **in progress** | This session's work — auto-tagging S1 to S4 with both reviews' fixes (§2.1), the block cut's leftovers (§2.2), the memory rule, the release workflow, the knobs scope, the two benchmark re-measures and the documentation repairs — lands on one integration branch, gated once at the end. It is a MINOR: readers and emitters change, and a `pdf_tagged` locator from either side of it does not parse on the other (`23-AUTO-TAGGING-SCOPE.md` §8) |

## 2. v2.2 — layout and accessibility (a roadmap version)

**Gate: 2 of 2 clauses met on the fixtures; the second's owner rows are pending.**
- **Clause one, met at 0.42.0:** a region is emitted wherever the cut divided a page and nowhere
  else.
- **Clause two, met on the fixtures 2026-09-17:** a tag this engine writes is one it can read back
  and ground against. Scoped in [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md), cut into
  slices in [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md); S1 to S4 are all on
  the integration branch. S3's round-trip tests pass, and S4 published the round trip over 293
  documents: 129 tagged, every one read back as computed, projected and grounded as its untagged
  original ([`measurements/auto-tagging/README.md`](measurements/auto-tagging/README.md) §2; the
  numbers sit in docs/23's amendments block). The owner's rows #25–#27 (docs/23 §12) record the
  shape.

Everything else filed under v2.2 has shipped: D4-S0 to S4, the `v2.2-S0` to `S8` labels (0.43.0 to
0.50.0) and the block cut (0.55.0).

### 2.1 Clause two: auto-tagging

| Item | Status | Waits on |
| --- | --- | --- |
| Scope document for the second half | **done 2026-09-16** | [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md), revised the same day after a three-lens adversarial review (23 findings folded in) |
| Private `/O` owner name and the attribute keys and values under `/A` | **decided in the scope, owner to record** | `/O /EthosParser`, `/Derivation /Computed`, `/Rule (gutter-columns-v3)`, read under `/A` and through `/ClassMap` (docs/23 §3.3, §4.1). Proposed as North Star row #25 |
| Which wire object says a tag over `Extracted` text was `Computed` | **done (S1)** | `pdf_tagged.derivation` on every tagged locator, always stated (decision #20), and the document-scoped `structure-tree-engine-written`; `struct-tree-v1` → `struct-tree-v2` (docs/23 §4.2) |
| Read-back of the attribute | **done (S1)** | `structure.rs` reads `/A` in every shape and `/C` through `/ClassMap`, binds `Computed` under an engine-written element, refuses a malformed owned object naming the element; four hand-written fixtures in the writer's tree shape plus the `/OBJR` pair hold it |
| Milestones document | **done 2026-09-16** | [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) |
| Writer subcommand: `tag`, the tree, one sequence per block where the stream allows, `/ParentTree`, filling absence only, the self-check | **done (S2)** | The writer review's 14 confirmed findings are fixed or recorded: the self-check compares every page's counters and walks the tree as written (`5c26a72`); new objects never take a number a dangling reference names, stale `/StructParents` keys are refused, and streams are removed only when nothing names them (`8500ab3`); the `/Properties` resolver has its own test (`5e649dc`); the rest are docs/23's amendments. Over 293 documents: 0 self-check, filter, decode or tokeniser refusals |
| Read-back test: write a tag, re-extract, confirm it reads as `Computed` and grounds; the projections equal the untagged original's | **done (S3)** | `group_key` reads a computed sequence as no declaration; `tag_roundtrip.rs` holds four acceptances on four fixtures, and S4 measured the projections equal on 129 of 129 tagged documents |
| #23's re-refusal measurement | **measured** | The misread rate needs no twin: on `nist-sp-800-207`, 8.9% of blocks hold two or more author `/P` (5.0% outside tables), 3.5% of `/P` are split, all at page breaks (README §1). The consumer: 0.58.0 binds all 41,208 runs of the 129 written trees as author structure, no `derivation` on any (README §2) |
| §7.1's multi-sequence blocks by cause | evidence | 74 of 453 blocks are more than one sequence, but the writer holds each sequence's cause in its plan and prints it nowhere, so the buckets §7.1 asked for are uncounted (docs/23 amendments) |

### 2.2 The block cut (shipped 0.55.0)

| Item | Status | Detail |
| --- | --- | --- |
| A consumer for `TextRunAttributes.block` | **done** | Auto-tagging writes one element per `(page, region, block)` (docs/23 §3.1) |
| Wire tests for the two hops that carry `block` | **done 2026-09-16** | `leading-gap-two-blocks`, the first fixture authored for the leading-gap cut, and three tests in `extraction.rs` (`7620bd2`) |
| Declare the cut's limits on the artifact | **done 2026-09-16** | Profile-scoped `block-subdivision-leading-gap-only` on every PDF extract artifact (`475a069`): whitespace only, no indent branch, 63.7% recall at 100% precision on one document, never a paragraph |
| A second labellable document with a different paragraph-break pitch | evidence | `19-BLOCK-SUBDIVISION-SCOPE.md` §11.4. It would reopen the 1.60× vs 1.15× choice |
| Re-test the cut's normaliser now that type size is readable (em- vs leading-normalised gaps) | evidence | Needs the item above for more than one document |

### 2.3 The local plan's other v2.2 definition

`.plans/v22-v23-implementation-plan-v3.md` §10 is untracked. It defines "v2.2 done" as three
conditions. Their state, and the reconciliation docs/23 §12 proposes as row #27:
- a refused table candidate emits something a retrieval pipeline can use — retired on the plan's own
  §5.3 T2 finding that there is nothing useful for a refused candidate to emit;
- TEDS re-measured and published as a band — **done 2026-09-16 at 0.58.0**: 0.0000..0.9802, median
  0.0000, 28 of 42 at zero ([`measurements/opendataloader-bench/README.md`](measurements/opendataloader-bench/README.md)),
  an instrument under decision #18, not a gate;
- the block cut has a consumer — auto-tagging (§2.1).

**Owner:** record row #27, or retire the plan's definition; the two come to the same.

## 3. v2.3 — **not a roadmap version**

v2.3 exists only in the untracked plan. Decision D4 there, to create a roadmap row, was never
recorded, and `02-ROADMAP.md` says no new version numbers get invented. Its items are tracked here
because they are real work whatever version they end up in.

**The plan's "done": 0 of 3 met, and the machinery for two of them now exists unrun.**
1. Prebuilt binaries on three platforms: macOS only; the workflow that builds and executes the
   other two has never run (6.3).
2. A quote grounded to its own glyphs: needs word boxes, which doc 22 refused. It is unreachable at
   the Ethos v0.6.0 pin without a verifier change outside this repository. **Owner:** reword or
   drop the condition.
3. Byte-identical on all three: the instruction-set axis is measured (aarch64 against x86_64 under
   Rosetta, both on macOS); the operating-system jobs exist on an unmerged branch and have never run
   (6.1).

| Plan item | Status | What remains, and what it waits on |
| --- | --- | --- |
| 6.1 Cross-OS byte identity | **evidence, owner** | Branch `ci/cross-os-digests` (`53f8622`, unmerged) adds `cross-os-digests` (ubuntu, macOS, Windows) and `cross-os-identity`; they run only on GitHub and have never run. Merging it onto the integration branch conflicts in two files, both trivially: `.gitattributes` (its `fixtures/** -text` line beside the release workflow's `*.sh text eol=lf`) and this page. **Waits on:** the owner pushing the branch or a PR, then reading the first run |
| 6.2 Memory ceiling | mostly done | Whether a caller-settable byte ceiling or a PDF decompression ceiling is wanted (owner). Verify-path wall time: option B (hash the input span) or C (lossless parse, then B) (owner; option A shipped in 0.55.0). Closed 2026-09-16 at 0.58.0 (memory-ceiling §15): the sizing rule is restated as 7 MiB + 5.4 MiB per admitted page + 0.33 MiB per document page, every coefficient the worst measured, over by 3.6% at its tightest point where the old rule was over by 44% on the worst case; and the `--max-pages` floor is separated — the structure tree is 28 MiB of the 733-page document's 221 MiB, the other 193 MiB the source bytes and parsed object graph, which a page budget cannot reach |
| 6.3 GitHub Releases, three platforms | **evidence** | The machinery exists and has never run. `.github/workflows/release-artifacts.yml` — on a `v*` tag push or by hand with `ref` and `targets` — builds `ethos-parser-cli` natively on `ubuntu-latest` (x86_64 gnu), `windows-latest` (x86_64 msvc), `macos-latest` (aarch64) and `macos-15-intel` (x86_64), each leg running `ci/release-artifacts.sh --native --tag <tag>`: build, execute over all eight gate PDFs, fingerprint, package. `verify` runs `--assemble` and writes `SHA256SUMS.txt` labelling every target `verified` only if every fingerprint is whole and byte-identical — 6.1's OS axis measured on the release binaries themselves. It publishes nothing (`contents: read`); the owner attaches `release-bundle` by hand (RELEASING.md §8). Known risk: the arm64 macOS runner has 7 GB against a 4.7 GB extract peak; `targets` drops a leg and §8 covers it locally. **Waits on:** the first tag push or dispatch, and reading what it finds |
| 6.4 PyPI wheel | owner | D4 and D8. **Irreversible** |
| 6.5 npm platform binaries | owner | D4 and D8. **Irreversible** |
| 6.6 Word-level boxes | refused | `22-WORD-BOXES-SCOPE.md` §7 names what reopens it |
| 6.7 `locate(representation, quote)` in CLI, MCP and SDK | owner | D7 |
| 6.8 crates.io, five crates in order | owner | D4. Last in the ordering. **Irreversible** |
| 6.9 Password, page sets, `continue_on_page_error`, batch, colour | **scoped 2026-09-16, owner** | [`25-KNOBS-SCOPE.md`](25-KNOBS-SCOPE.md), measured over 297 PDFs: 1 encrypted (a user password; owner-password-only files are read silently today, a finding in its own right), 7 documents over 50 pages, 0 page-local failures anywhere, a sequential loop against four-way parallel at 1.98–2.28×, and 91 of 294 documents declaring a colour space that needs resolution. Its recommendations — defer the password knob and declare the silent open; defer page sets; refuse `continue_on_page_error` on measurement; refuse a batch mode; refuse colour as an emitted field — are proposals for the owner (§8 there) |

## 4. Owner decisions pending

| Decision | Blocks |
| --- | --- |
| **Rows #25–#27** as proposed in [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md) §12: the auto-tagging scope, no MCP/SDK exposure of the writer, and the reconciliation of the plan's v2.2 definition | Clause two's status on the North Star; §2.3 |
| **docs/23 §3.6 row 2's reopening count:** 146 of the 200 opendataloader-bench documents (all PyPDF2 page splits) are refused for marked-content ids without a tree, the shape the row says reopens with a corpus count showing it is common | Whether `tag` ever writes around a document's orphaned ids |
| **docs/23 §3.5's reopening condition for an incremental update:** 6 of 4,116 reals outside content streams do not survive `f32`, in 2 PyPDF2 documents, below the ninth significant digit — non-zero on one producer; whether it is *a producer that matters* | The full re-serialisation |
| **0.58.0's shape:** tag `b4b4aa9` as v0.58.0 as prepared, with this session's work as the next version (recommended); or re-cut 0.58.0 over everything, rewriting the release commit, rebuilding the artifacts and re-running preflight | §1 |
| The proposals of [`25-KNOBS-SCOPE.md`](25-KNOBS-SCOPE.md) §8, one per knob | 6.9 |
| **D4:** create a v2.3 roadmap row (amends `02-ROADMAP.md`'s "no new version numbers" and #15's order) | 6.4, 6.5, 6.8 |
| **D8:** may the Python and Node packages ship a platform binary (reverses v1.2-S2)? | 6.4, 6.5 |
| **D7:** does `locate(representation, quote)` cross the verify boundary (`07-VERIFY-BOUNDARY.md`)? | 6.7 |
| **D1:** reverse P14 narrowly for roles on geometric blocks. Auto-tagging writes `/Div`, never `/P`, and says what moves if D1 is taken (docs/23 §3.2) | Plan §5.4, the tag role |
| **D2:** reverse L29 to infer headings from size or font name. The plan's default is to defer | — |
| **D3:** publish a head-to-head comparison, which reverses O26. Running one needs no decision | Publication only |
| The plan's v2.3 "done" condition 2, *a quote grounded to its own glyphs*: reword or drop | §3 |
| How a text-run box declares its kind. `docs/01-CONTRACT.md` §5.3 records it as open, and §6's versioned rule for the box is not in the profile. With it, whether the liteparse refusal's Wall 2 (`liteparse_refusal.rs`) and §5.3's "wrong for a citation highlight" should be re-taken, now that both apply to this engine's own box (`d544418` commit message) | Contract §5.3 and §6 |
| Retire the unruled table rule, and whether `table_detection.unruled` moves off `unruled-align-v1` after the precondition reorder | Table track |
| The geometric table rework (ruled-rects-v4 to v6) was the owner's choice in the plan only. `table-gate-v1.md`'s header carries a dated amendment recording the gap; recording it under #18 is the owner's | Governance |
| Record or withdraw the plan's D1 to D8. The plan says "#24 onward", but #24 went to the MCP ledger; docs/23 proposes #25–#27 | Governance |
| Semver for byte-identical but source-breaking type changes: the `Arc` in `StructuralLocator::PdfTagged`, and `GeometryAbsence` growing without `#[non_exhaustive]` | Release policy |
| `font_size` stays the raw `Tf` operand, and `advance` stays in its pre-rotation frame: keep, document, or change | Wire meaning |
| The research memos in the root commit `f04c488`, reachable from public `main` and three immutable releases. Removal means rewriting every SHA | **Irreversible** either way |
| Repository security settings: secret scanning, push protection and dependabot are off, and org-wide 2FA is not required | Settings |

## 5. Ready now — no decision needed

- **A geometry digest check in the gate.** No committed golden covers geometry, so a box change is
  caught only by tests that read boxes.
- **CI coverage of the two forbidden-token greps.** They are PR-only jobs, and no PR has run since
  2026-09-05 under the local-merge flow. Move them into `check`, or open PRs.
- **Re-run OmniDocBench** as an instrument after 0.58.0; opendataloader-bench was re-run on
  2026-09-16 (NID 0.8697, TEDS 0.1704 as the band above, MHS 0.0000). The OmniDocBench PDFs were not
  on this machine on 2026-09-16 (9 GiB free), so the run needs the corpus fetched again.
- **Attribute cfpb's 259‰ → 244‰.** The table band was re-stated at 0.58.0 (macro 69‰, combined
  micro recall 503‰, 162 of 172 gold tables emitted from their tags; `table-gate-v1.md` "The band
  re-stated at 0.58.0"), and cfpb's move (17 → 12 geometric detections) lies between the v2-S24 run
  and v0.57.0 with no commit attributed; the harness run at `4da0674`, `2a53416`, `7bd1a79` and
  `b742566` would settle it.
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
| `extract` reads page content through `lopdf`'s lenient decoder: a stream tail after the first unlexable operation is dropped silently, with no limitation; and a content stream whose Flate data is corrupt decodes partially and is accepted. Measured 0 such pages over 1 567 (the writer refuses both by name; the reader does not) | `tagging.rs` module header; docs/23 §3.5 |
| `lopdf` 0.44.0 panics on an inline image with neither `/CS` nor `/IM true` (`image_data_stream`'s `unwrap`), so `extract` would crash on such a page; the writer's tokeniser refuses it instead | `tagging.rs` tests |
| `geometry-absent-not-groundable` fires with a `0 of N text node(s)` detail on 26 of the 200 bench documents, where the absent geometry is an image's or an annotation's, which `non-text-nodes-not-projected` already declares | `measurements/opendataloader-bench/README.md` (0.58.0 census) |
| `classify` exits 0 on `tagged-cycle` and on a page-error document that `extract` refuses: it reads neither the structure tree nor the content streams | `25-KNOBS-SCOPE.md` proposal 5 |
| An owner-password-only encrypted PDF is read today with no limitation mentioning encryption: `lopdf` opens it with the empty user password and removes `/Encrypt` before the engine looks | `25-KNOBS-SCOPE.md` §3 |
| Five gate documents cite one `(page, mcid)` pair from two structure elements; the reader keeps the last binding it walked | `measurements/auto-tagging/README.md` §1 |
| `docs/measurements/block-subdivision/structelem.py` drops every `/P` whose `/K` holds more than a bare integer (a paragraph with a link), so `probe3b.py`'s population is 314 of 407 paragraphs | `measurements/auto-tagging/README.md` §1 |

## 7. Documents that say something no longer true

Repaired 2026-09-16/17: row 23's CLI count (dated correction), `16-D4-SCOPE.md` §9, the
`table-gate-v1.md` header (dated amendment), `RELEASING.md` (public repository, immutable
releases), `derivation-class.draft.json`, `04-ARCHITECTURE.md`'s font crate, the history documents'
"v1 is not done", the fixture counts, and the block-subdivision rows of `02-ROADMAP.md` and doc 19.

| Where | What is wrong |
| --- | --- |
| `00-NORTH-STAR.md` row 23, `16-D4-SCOPE.md` §7 (its earlier text), `19-BLOCK-SUBDIVISION-SCOPE.md` | Say `structure.rs` "already parses this exact shape". It read `/A` only for cell spans until S1 (docs/23 §4.1); the rows are dated records and stand, corrected by that section |
| `00-NORTH-STAR.md` §5, the v2.2 row's Intent cell | Says "auto-tagging reopened and unstarted"; the Done-when cell beside it carries the dated correction |
| `02-ROADMAP.md`, the not-scheduled auto-tagging row | Quotes #23's "another reader still sees a plain `/P`"; the writer emits `/Div` (docs/23 §3.2) |
| `measurements/cross-architecture/README.md` | Says CI cannot allocate a runner. Corrected on the unmerged branch `ci/cross-os-digests` |
| `crates/ethos-parser-cli/tests/extract_budget.rs` doc header | Attributes the `--max-pages` floor to the structure tree; memory-ceiling §15 measured it at 28 of 221 MiB |
| `docs/CAPABILITY.md`, the Cannot row "a paragraph boundary on an untagged page" | Worded as if no block cut existed; the Can table's "Layout blocks" row and the limitation code say what does |
| `fixtures/engine/make_fixtures.py` lines ~349–351 | The generator's own "every other one exercises typed absence" comment, stale as the manifest note was |
| `06-STEAL-REFUSE.md:230`, `17-D1-SCOPE.md:86`, `measurements/opendataloader-bench/README.md:20` | Quote 70‰ as current; the band is re-stated at 69‰ |
| `docs/history/03-V0-SCOPE.md:137` | "37 engine-authored CC0 fixtures" — a dated record, left as written |
