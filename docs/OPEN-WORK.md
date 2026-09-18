# Open work

**What is pending, and what each item waits on.** A ledger, not a plan and not a decision record.
[`02-ROADMAP.md`](02-ROADMAP.md) owns the versions and [`00-NORTH-STAR.md`](00-NORTH-STAR.md) owns
the decisions; this page only tracks what is not done yet, so that nothing measured and left open is
lost between sessions. When an item moves, move it here in the same commit.

**Written 2026-09-16 against `main` at `fae4c90`; revised 2026-09-17 against the integration
branch for the version after 0.58.0, and again the same day when auto-tagging's round trip and its
measurements (S3, S4) and the writer review's fixes merged into it; and again that evening, when
that branch and the cross-OS jobs merged into local `main` (`89f9390`, `0edbaad`) and the owner's
decisions of the day were recorded as North Star rows #25–#31.** v0.57.0 is the latest published
release. The unpublished 0.58.0 release commit is `b4b4aa9` (preflight green, macOS artifacts built
and verified); local `main` carries it and, after it, everything this page marks *done
2026-09-16/17*, for 0.59.0.

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
| **Publish 0.58.0** | **owner** | **`main` pushed 2026-09-18** at `791f0fe`, carrying the release commit `04bd9f2` and its merge `b4b4aa9` and all the v2.2 and v2.3 work after them; no tag. The first push to carry `release-artifacts.yml` showed the file was invalid, fixed the same day (`docs/RELEASING.md` §8's amendment) — and that the workflow **cannot build 0.58.0 by any route**: `b4b4aa9` predates the file, and a dispatch at that ref runs a script that predates `--native`. So 0.58.0 ships the macOS pair below. Before the push: the release commit sat on local `main`, unpushed. `ci/release-preflight.sh` passed there (gate 9/9); `target/release-artifacts/` holds the `aarch64` and `x86_64` macOS tarballs, both `verified`; the notes are drafted. Remaining, all the owner's: ~~push `main`~~ (done 2026-09-18), tag `v0.58.0` at `b4b4aa9`, create the release. **Until the tag exists, `parser_version` 0.58.0 names a build no release carries.** Every byte-changing commit since `b4b4aa9` belongs to the next version. **The owner decided 2026-09-17:** tag `b4b4aa9` as 0.58.0 as prepared, and all later work is 0.59.0 |
| **Cut 0.59.0** | **in progress** | The work after 0.58.0 — auto-tagging S1 to S4 with both reviews' fixes (§2.1), the block cut's leftovers (§2.2), the memory rule, the release workflow, the cross-OS jobs, the knobs scope, the reader's content-stream guards (§6), the two benchmark re-measures and the documentation repairs — merged into local `main` on 2026-09-17 and is gated once, when the rest of 0.59.0 has landed. It is a MINOR: readers and emitters change, and a `pdf_tagged` locator from either side of it does not parse on the other (`23-AUTO-TAGGING-SCOPE.md` §8) |

## 2. v2.2 — layout and accessibility (a roadmap version)

**Gate: met.** Decision #27 (2026-09-17) makes the roadmap's two clauses the whole gate.
- **Clause one, met at 0.42.0:** a region is emitted wherever the cut divided a page and nowhere
  else.
- **Clause two, met on the fixtures 2026-09-17:** a tag this engine writes is one it can read back
  and ground against. Scoped in [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md), cut into
  slices in [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md); S1 to S4 are all on
  `main`. S3's round-trip tests pass, and S4 published the round trip over 293
  documents: 129 tagged, every one read back as computed, projected and grounded as its untagged
  original ([`measurements/auto-tagging/README.md`](measurements/auto-tagging/README.md) §2; the
  numbers sit in docs/23's amendments block). North Star decisions #25–#27 record the shape.

Everything else filed under v2.2 has shipped: D4-S0 to S4, the `v2.2-S0` to `S8` labels (0.43.0 to
0.50.0) and the block cut (0.55.0).

### 2.1 Clause two: auto-tagging

| Item | Status | Waits on |
| --- | --- | --- |
| Scope document for the second half | **done 2026-09-16** | [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md), revised the same day after a three-lens adversarial review (23 findings folded in) |
| Private `/O` owner name and the attribute keys and values under `/A` | **done 2026-09-17** | `/O /EthosParser`, `/Derivation /Computed`, `/Rule (gutter-columns-v3)`, read under `/A` and through `/ClassMap` (docs/23 §3.3, §4.1). Recorded as North Star row #25 |
| Which wire object says a tag over `Extracted` text was `Computed` | **done (S1)** | `pdf_tagged.derivation` on every tagged locator, always stated (decision #20), and the document-scoped `structure-tree-engine-written`; `struct-tree-v1` → `struct-tree-v2` (docs/23 §4.2) |
| Read-back of the attribute | **done (S1)** | `structure.rs` reads `/A` in every shape and `/C` through `/ClassMap`, binds `Computed` under an engine-written element, refuses a malformed owned object naming the element; four hand-written fixtures in the writer's tree shape plus the `/OBJR` pair hold it |
| Milestones document | **done 2026-09-16** | [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) |
| Writer subcommand: `tag`, the tree, one sequence per block where the stream allows, `/ParentTree`, filling absence only, the self-check | **done (S2)** | The writer review's 14 confirmed findings are fixed or recorded: the self-check compares every page's counters and walks the tree as written (`5c26a72`); new objects never take a number a dangling reference names, stale `/StructParents` keys are refused, and streams are removed only when nothing names them (`8500ab3`); that commit's own adversarial review found its numbering rule wrote a ten-million-entry cross-reference table for one dangling `10000000 0 R`, which qpdf and Ghostscript could not read while the self-check passed, and `0c7a3c9` numbers from the highest held object instead, removes an indirect `/Contents` array with its streams, reads a null key as no key, and checks each element's `/Type` and `/P`; the `/Properties` resolver has its own test (`5e649dc`); the rest are docs/23's amendments. Over 293 documents: 0 self-check, filter, decode or tokeniser refusals |
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
conditions. Their state, and the reconciliation recorded as North Star row #27:
- a refused table candidate emits something a retrieval pipeline can use — retired on the plan's own
  §5.3 T2 finding that there is nothing useful for a refused candidate to emit;
- TEDS re-measured and published as a band — **done 2026-09-16 at 0.58.0**: 0.0000..0.9802, median
  0.0000, 28 of 42 at zero ([`measurements/opendataloader-bench/README.md`](measurements/opendataloader-bench/README.md)),
  an instrument under decision #18, not a gate;
- the block cut has a consumer — auto-tagging (§2.1).

**Recorded 2026-09-17 as row #27:** the plan's definition is retired, and the roadmap's two clauses
are the gate.

## 3. v2.3 — distribution (decision #28)

v2.3 is a roadmap version since 2026-09-17, by North Star decision #28, which amends
`02-ROADMAP.md`'s *no new version numbers* and row 15's order. It has no scope or milestones
document yet (§4).

**The gate: 0 of 2 met, and the machinery for both exists unrun.**
1. Prebuilt binaries on three platforms: macOS only; the workflow that builds and executes the
   other two has never run (6.3).
2. Byte-identical on all three: the instruction-set axis is measured (aarch64 against x86_64 under
   Rosetta, both on macOS); the operating-system jobs are in `ci.yml` and have never run (6.1).

The plan's other condition, *a quote grounded to its own glyphs*, is dropped by #28: it needed word
boxes, which doc 22 refused.

| Item | Status | What remains, and what it waits on |
| --- | --- | --- |
| 6.1 Cross-OS byte identity | **passed on its first run, 2026-09-18** | **Measured:** the first push of `main` to carry the jobs (`791f0fe`, run 35318173062) built the release engine on all three systems and `cross-os-identity` passed — byte-identical digests over 81 documents, 320 of 324 rows non-empty. That is the operating-system axis of v2.3's condition on CI builds; the other half, verified release binaries, waits for 0.59.0 (`RELEASING.md` §8). Before that run: *the jobs exist and have never run, so nothing is proven yet.* `ci.yml`'s `cross-os-digests` builds the release engine on `ubuntu-latest`, `macos-latest` and `windows-latest` and runs `ci/artifact-bytes.py` on each; `cross-os-identity` fails if the three digest lists differ by a byte, if any row records a crash, or if any list carries fewer than 329 non-empty, exit-0 artifacts (332 of 336 since decision #29's three heading fixtures), so a run of refusals cannot pass. Inputs: 84 documents — every engine and office fixture, and six of the eight gate PDFs. **Not covered:** `nist-sp-800-53Ar5` and `nist-sp-800-161r1` (runner time and memory) and the Ethos conformance corpus. Windows fixes made for it: `fixtures/** -text` in `.gitattributes`, because a CRLF checkout made the engine refuse all 42 hand-built ASCII PDFs; and `.exe`, `os.devnull` and LF output in `ci/artifact-bytes.py`. The gate harness on Windows: all workspace targets, tests included, type-check for `x86_64-pc-windows-gnu`, but nothing has been linked or run there, `ci/gate.sh` and the test suite included. ~~**Waits on:** the owner's first push to `main`, then reading that run~~ — read 2026-09-18, green |
| 6.2 Memory ceiling | mostly done | Whether a caller-settable byte ceiling or a PDF decompression ceiling is wanted (owner). Verify-path wall time: option B (hash the input span) or C (lossless parse, then B) (owner; option A shipped in 0.55.0). Closed 2026-09-16 at 0.58.0 (memory-ceiling §15): the sizing rule is restated as 7 MiB + 5.4 MiB per admitted page + 0.33 MiB per document page, every coefficient the worst measured, over by 3.6% at its tightest point where the old rule was over by 44% on the worst case; and the `--max-pages` floor is separated — the structure tree is 28 MiB of the 733-page document's 221 MiB, the other 193 MiB the source bytes and parsed object graph, which a page budget cannot reach |
| 6.3 GitHub Releases, three platforms | **evidence** | The machinery exists and has never run. `.github/workflows/release-artifacts.yml` — on a `v*` tag push or by hand with `ref` and `targets` — builds `ethos-parser-cli` natively on `ubuntu-latest` (x86_64 gnu), `windows-latest` (x86_64 msvc), `macos-latest` (aarch64) and `macos-15-intel` (x86_64), each leg running `ci/release-artifacts.sh --native --tag <tag>`: build, execute over all eight gate PDFs, fingerprint, package. `verify` runs `--assemble` and writes `SHA256SUMS.txt` labelling every target `verified` only if every fingerprint is whole and byte-identical — 6.1's OS axis measured on the release binaries themselves. It publishes nothing (`contents: read`); the owner attaches `release-bundle` by hand (RELEASING.md §8). Known risk: the arm64 macOS runner has 7 GB against a 4.7 GB extract peak; `targets` drops a leg and §8 covers it locally. **Waits on:** the first tag push or dispatch, and reading what it finds |
| 6.4 PyPI wheel | owner | Publishing is the owner's. D8, whether the package ships a platform binary, is deferred by the owner (2026-09-17). **Irreversible** |
| 6.5 npm platform binaries | owner | As 6.4: the owner's, with D8 deferred. **Irreversible** |
| 6.6 Word-level boxes | refused | `22-WORD-BOXES-SCOPE.md` §7 names what reopens it |
| 6.7 `locate(representation, quote)` in CLI, MCP and SDK | **done 2026-09-18, S0 to S3** | Decision #30 bounds it: a representation and a string in, locations out; no verdict, boolean or claim; a match rule of the engine's own. [`26-LOCATE-SCOPE.md`](26-LOCATE-SCOPE.md) settles the shape — `ethos.parser.locations.v0`, the rule `locate-scalar-exact-v1` (code-point-exact on scalars, joining runs inside one block of the cut and never across two), occurrences as node ids, offsets and the record's own geometry, and every verdict-shaped convenience refused by name — and [`27-LOCATE-MILESTONES.md`](27-LOCATE-MILESTONES.md) cuts it into S0 (the documents, done), S1 (the core query, the profile field, the cap measured — **done**: `locate` and `ethos.parser.locations.v0` in `ethos-parser-core`, `locate_rule` on the profile and the profile re-pinned, 23 tests across three crates plus an ignored instrument, and the cap confirmed at 1,000,000 on a measured 127.6 MiB at the ceiling, [`measurements/locate/README.md`](measurements/locate/README.md)), S2 (CLI, MCP and both SDKs in one slice — **done**: the eleventh subcommand with no exit 1 ever, a fourth MCP tool whose summary is counts only, `locate()` in both SDKs with the quote through a file rather than argv, and a `locations.draft.json` with its guard) and S3 (the measurements over `fixtures/gate` — **done**: a citation-length quote occurs exactly once on all eight documents, a short quote's band is 37 to 8 924 with the worst named, one occurrence is 53 node parts on the most shredded, and a call costs 20 ms plus the record at 80 MB/s with the search a rounding error beside it) |
| 6.8 crates.io, five crates in order | owner | Publishing is the owner's. Last in the ordering. **Irreversible** |
| 6.9a Declare the empty-user-password open | **done 2026-09-17** | Decision #31 (proposal 1 of [`25-KNOBS-SCOPE.md`](25-KNOBS-SCOPE.md)): `encrypted-empty-user-password`, document-scoped, on every artifact from such an open, from `extract` and `classify` alike; contract §8 carries the amendment. Of the 311 PDFs in every corpus here, 1 is encrypted and needs a secret, and 0 open on the empty password |
| 6.9b Password knob | deferred (#31) | Reopens on a corpus this repository can pin whose documents need a user password the caller holds; then never on argv or over MCP (docs/25 §3.3) |
| 6.9c Page sets | deferred (#31) | Reopens on a named caller, as a third `PageBudget` variant with no adapter line (docs/25 §4.3) |

## 4. Owner decisions pending

| Decision | Blocks |
| --- | --- |
| **docs/23 §3.6 row 2's reopening count:** 146 of the 200 opendataloader-bench documents (all PyPDF2 page splits) are refused for marked-content ids without a tree, the shape the row says reopens with a corpus count showing it is common | Whether `tag` ever writes around a document's orphaned ids |
| **docs/23 §3.5's reopening condition for an incremental update:** 6 of 4,116 reals outside content streams do not survive `f32`, in 2 PyPDF2 documents, below the ninth significant digit — non-zero on one producer; whether it is *a producer that matters* | The full re-serialisation |
| **D8:** may the Python and Node packages ship a platform binary (reverses v1.2-S2)? **Deferred by the owner 2026-09-17** ("later") | 6.4, 6.5 |
| **Right-to-left text: should an artifact declare that no bidi reordering was applied?** Measured 2026-09-18 on the new `rtl-hebrew-visual-order` fixture: a producer that has resolved bidi draws Hebrew in visual order, so the run's text is the logical word reversed and `char_codes` carries the page's order. Nothing on the artifact says so, and a consumer quoting in logical order cannot tell. A document-scoped declaration, fired where the text holds right-to-left scalars, is one line; the alternative is the fixture and `CAPABILITY.md` saying it and nothing on the wire | Consumers of RTL documents |
| **Reading order: replace the identity fallback?** Measured 2026-09-18 ([`measurements/opendataloader-bench/reading-order-causes.md`](measurements/opendataloader-bench/reading-order-causes.md)): where the first vertical cut finds no gutter, `reading_order.rs` leaves the page in content-stream order, and the 178 bench documents carrying no `region` hold **90.1%** of NID's ordering cost; 126 documents come out in more than one downward sweep and hold 99.1% of it. A downward sweep as the fallback is worth **0.8697 → 0.9162** on that corpus and touches 126 of 200 documents. **It is a decision, not a fix to take quietly:** the rule id moves, every artifact of every affected document moves its node order, and the ordinals a citation names move with it — the same shape of change as decision #19's | Reading order, and every artifact on the documents it touches |
| **D1:** reverse P14 narrowly for roles on geometric blocks. Auto-tagging writes `/Div`, never `/P`, and says what moves if D1 is taken (docs/23 §3.2) | Plan §5.4, the tag role |
| **D3:** publish a head-to-head comparison, which reverses O26. Running one needs no decision | Publication only |
| How a text-run box declares its kind. `docs/01-CONTRACT.md` §5.3 records it as open, and §6's versioned rule for the box is not in the profile. With it, whether the liteparse refusal's Wall 2 (`liteparse_refusal.rs`) and §5.3's "wrong for a citation highlight" should be re-taken, now that both apply to this engine's own box (`d544418` commit message) | Contract §5.3 and §6 |
| Retire the unruled table rule, and whether `table_detection.unruled` moves off `unruled-align-v1` after the precondition reorder. **Measured 2026-09-18** ([`measurements/opendataloader-bench/tables-why-zero.md`](measurements/opendataloader-bench/tables-why-zero.md)): on 11 of the 28 TEDS-zero documents the rule projects *every run on the page* onto two axes, so the candidate holds 4 680–14 941 cells where the truth's tables hold 5–112, and the 4 096-cell ceiling refuses it. Building the candidate per `region` instead is a narrowing, so it cannot buy a fabrication by construction — the strongest lead the table track has | Table track |
| The geometric table rework (ruled-rects-v4 to v6) was the owner's choice in the plan only. `table-gate-v1.md`'s header carries a dated amendment recording the gap; recording it under #18 is the owner's | Governance |
| **v2.3's documents.** `02-ROADMAP.md` gives every version a scope and a milestones document before code; v2.3's items are independent and some already have code (the release and cross-OS workflows). One document pair for the version, or a scope per item that changes the engine | `locate` and headings, before their code |
| **The roadmap home of inferred headings (#29).** The local plan filed D2 under v2.2's steps, whose gate #27 closed; decision #28 did not list it under v2.3 | The roadmap row only |
| The rest of the plan's D1 to D8. D2, D4 and D7 are recorded as North Star #29, #28 and #30, and D5, keeping auto-tagging, as #25. D1 and D3 are above, and D8 is deferred. D6, landing wire changes before a first registry publication, waits with the packages | Governance |
| Semver for byte-identical but source-breaking type changes: the `Arc` in `StructuralLocator::PdfTagged`, and `GeometryAbsence` growing without `#[non_exhaustive]` | Release policy |
| `font_size` stays the raw `Tf` operand, and `advance` stays in its pre-rotation frame: keep, document, or change | Wire meaning |
| The research memos in the root commit `f04c488`, reachable from public `main` and three immutable releases. Removal means rewriting every SHA | **Irreversible** either way |
| Repository security settings: secret scanning, push protection and dependabot are off, and org-wide 2FA is not required | Settings |

## 5. Ready now — no decision needed

- **Headings inferred from font size or font name** (decision #29) — **scoped 2026-09-18**,
  [`28-HEADINGS-SCOPE.md`](28-HEADINGS-SCOPE.md); **S1, the reader, done 2026-09-18**: the rule reads
  the rendered em, sets `inferred_heading` on a line of a document with no author structure, and
  declares `headings-inferred-from-type`; tagged documents are byte-identical but for the profile
  hash, and it fires on none of the 58 pre-existing engine fixtures that extract. S2 (the projections) next. What the scope settles, each on
  a measurement: the signal is the **rendered em**, not the field spelled `font_size`, which carries
  one distinct value on 87 of the 200 bench documents and on all four gate documents measured; the
  unit is the line; one level, because the harness flattens them and all 193 ground-truth headings
  are level 1; the verdict goes on the wire as an attribute absent where false, with
  `heading_inference_rule` on the profile and a document-scoped declaration; and the size clause
  ships alone at a **5% per-document false-positive bound**, with the font clause held as a
  conditional slice because it measures 91.7% recall at up to 15.76%. Which roadmap version carries
  it is pending (§4).

- ~~**Re-run OmniDocBench** as an instrument after 0.58.0~~ — **done 2026-09-18**: the corpus was
  re-fetched (981 files, 538 521 457 bytes, `v1_0`) and the census re-run in 13 s on the build at
  `a18b2b0`; figures, the two codes that moved and the one new hard failure are in
  [`measurements/omnidocbench/README.md`](measurements/omnidocbench/README.md). opendataloader-bench
  was re-run on 2026-09-16 (NID 0.8697, TEDS 0.1704, MHS 0.0000).
- **A committed fixture for the `<mc:AlternateContent>` path.** Neither `docx.rs` nor `pptx.rs`
  has one: both readers' branch rule is held by unit tests on inline XML, and no document in any
  corpus here carries the element, so the mutation suite and the digest lists never reach it.
- **A committed fixture carrying a non-BMP scalar or a combining mark.** No engine fixture holds
  either, checked across `make_fixtures.py` on 2026-09-18, so `locate`'s scalar-unit and
  no-normalisation rules are held by hand-built representation tests
  ([`26-LOCATE-SCOPE.md`](26-LOCATE-SCOPE.md) §9, T10 and T11) and the mutation suite and the digest
  lists never reach them. Authoring one is a font and a `/ToUnicode` map, which
  `simple-font-two-byte-tounicode` and `rtl-hebrew-visual-order` both already demonstrate.
- **The doc fixes in §7.**

## 6. Known defects — recorded, not fixed

| Defect | Where it is recorded |
| --- | --- |
| `advance` on a CTM- or `/Rotate`-turned page is measured before the rotation, so it disagrees with the box and with the contract's "after page rotation" | `PdfLocator::advance` rustdoc; doc 22 amendments |
| Office markdown and html stamp the PDF default profile's `profile_sha256`, not the profile that produced the representation | `409102d` commit message |
| An ODT or ODP heading is on the wire as a heading and projects as a **paragraph**: `<text:h>` sets `OdfBlockKind::Heading`, and `text:outline-level` is unread, so no level exists to emit. Found while scoping inferred headings, where decision #29's rider that a declared heading always wins stands beside a declared heading that wins nothing | [`28-HEADINGS-SCOPE.md`](28-HEADINGS-SCOPE.md) §6.2 |
| On `nist-sp-800-53Ar5` page 47, 23 characters of a turned table header (`Assessor /`, `Assessment Team`) are absent from the extract | `docs/measurements/rotated-text/README.md` |
| A `LZWDecode` or `ASCII85Decode` content stream corrupt part way decodes in part and is accepted: `lopdf`'s decoders for both return their partial output as a success, and the reader's check covers `FlateDecode`. No page of any corpus here carries either filter | `extract.rs::page_operations` |
| Five gate documents cite one `(page, mcid)` pair from two structure elements; the reader keeps the last binding it walked | `measurements/auto-tagging/README.md` §1 |
| `docs/measurements/block-subdivision/structelem.py` drops every `/P` whose `/K` holds more than a bare integer (a paragraph with a link), so `probe3b.py`'s population is 314 of 407 paragraphs | `measurements/auto-tagging/README.md` §1 |

## 7. Documents that say something no longer true

Repaired 2026-09-16/17: row 23's CLI count (dated correction), `16-D4-SCOPE.md` §9, the
`table-gate-v1.md` header (dated amendment), `RELEASING.md` (public repository, immutable
releases), `derivation-class.draft.json`, `04-ARCHITECTURE.md`'s font crate, the history documents'
"v1 is not done", the fixture counts, the block-subdivision rows of `02-ROADMAP.md` and doc 19;
and later on 2026-09-17 the roadmap's `/P` quotation, `extract_budget.rs`'s floor, CAPABILITY's
paragraph row, `make_fixtures.py`'s typed-absence comment, the 70‰ in `06-STEAL-REFUSE.md` and
the opendataloader-bench README, and, with the cross-OS jobs merged, the cross-architecture
README's runner sentence; and, with the owner's decisions recorded, the North Star's v2.2 intent
cell.

| Where | What is wrong |
| --- | --- |
| `00-NORTH-STAR.md` row 23, `16-D4-SCOPE.md` §7 (its earlier text), `19-BLOCK-SUBDIVISION-SCOPE.md` | Say `structure.rs` "already parses this exact shape". It read `/A` only for cell spans until S1 (docs/23 §4.1); the rows are dated records and stand, corrected by that section |
| `17-D1-SCOPE.md:86` | Quotes 70‰ as the macro; the band is re-stated at 69‰. A dated scope record, so it stands |
| `docs/history/03-V0-SCOPE.md:137` | "37 engine-authored CC0 fixtures" — a dated record, left as written |
