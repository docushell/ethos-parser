# ethos-engine — implementation documentation

**Status:** **v1.2 is complete at 0.19.0; v2 has decided its contract at 0.20.0 and written no reader.** v1.1 is complete. v1 is
the DocuShell replacement gate (`08-V1-SCOPE.md`); S1–S6, S7a, S7b and S8 are done and **S7 is
open**. Its gate — table-cell
accuracy above 0.489 — is measured and **missed**: macro cell-slot F1 is **64‰** against a
489‰ floor. The method is [`table-gate-v1.md`](table-gate-v1.md). **v1 is not done**, and v1.1
began because the owner asked for the next roadmap row rather than because the gate cleared.

**v1.1 is Safe Markdown** (`10-V11-SCOPE.md`, `11-V11-MILESTONES.md`). `ethos.markdown.v1` carries
a Markdown string and the **Anchor Map** that inverts every source byte of it back to
representation nodes, plus a coverage census of what did not make it. `docs/01-CONTRACT.md` §12
refused a Markdown projection for the whole of v1 on Workbench rule 8; the map is what makes that
objection payable.

**v1.2 is ADOPTION, and S1 is its first adapter: `engine mcp`, MCP over stdio.** Three tools —
`extract`, `ground`, `node_get` — over newline-delimited JSON-RPC on a pipe, with no HTTP, no
socket and no async runtime, so `deny.toml`'s network bans hold. The whole slice turns on one
sentence from the parser memo §16.7: MCP tools are model-controlled, so **the engine mints every
locator, returns it as an opaque handle, and re-validates it on the way back in** — get that wrong
and MCP is the worst host on the list rather than the best. A forged node id is an error, an edited
representation fails its fingerprint, and no tool argument names a coordinate.
[`12-V12-SCOPE.md`](12-V12-SCOPE.md) states the law; [`13-V12-MILESTONES.md`](13-V12-MILESTONES.md)
records S5 as **done and refused**.

**S2 is the Python SDK, and it wraps the CLI rather than the library.** `packages/python/` is three
functions over `subprocess` with **no runtime dependency** — so byte-identity with the CLI is a
tautology, not a promise, and a test proves it on a whole artifact. No signature names a coordinate;
`node_get` ports c14n v1 to check a fingerprint before any lookup, because it is the one function
with no subcommand behind it. No PyO3, no `capabilities.python`, not on PyPI.

**S3 is that surface again in Node, and it is not a second design.** `packages/node/` spawns the
same binary, exports the same three functions, throws the same six error names, and is pinned to
the same version — S2 is the contract, and where the two differ it is only where the languages
force it. No napi, no TypeScript, no runtime dependency, `private: true`. c14n is ported again,
sorting keys by code point because JavaScript's default sort compares UTF-16 units and disagrees
with Rust above the BMP — and the one thing that language cannot do, tell `1.0` from `1`, is
written down as a named divergence rather than approximated.

**S4 makes both SDKs callable from LangChain, and the split is the whole slice.** Three tools per
language over `response_format="content_and_artifact"`: the artifact carries the SDK object, and
`content` carries MCP's counts and nothing a pipeline would bind to — compared **byte-for-byte**
against what `engine mcp` emits, so the two adapters cannot drift into two sentences about one
document. The argument schemas are MCP's own, verbatim. LangChain is an optional extra and an
optional peer on a subpath, so the default import still pulls nothing. No LangGraph adapter (the
memo refused one), no trust state, no `verify` tool, no `capabilities.langchain`.

**S5 measured the liteparse adapter and refused it, which completes v1.2.** Two walls, both in
`ethos.grounding.v1` itself: their output *"emits `page, width, height, text, text_items` and
nothing else"*, so it cannot name its own producer, and the schema requires one with no way to mark
a field asserted rather than measured; and their boxes are loose em boxes that §5.3 says must be
declared, with nowhere in the schema to declare them. The predicted blocker — an unknown coordinate
origin — **dissolved**: their space and this engine's visible box are the same box. The refusal is
pinned by `engine-grounding/tests/liteparse_refusal.rs` so relaxing either schema fact reopens it.

**v2 is office formats, and it is scoped rather than started.** Its gate is *a DOCX quote and an
XLSX cell both ground; no synthesised pages*, and the second clause is the whole hazard: a DOCX has
no page, so a page on a Word citation is a measurement of the machine that printed it rather than of
the document. `06-STEAL-REFUSE.md` **L30** already refuses the shortest path — LibreOffice → PDF —
and `01-CONTRACT.md` §5.1 already says a rendered page is never substituted for a native address.
[`14-V2-SCOPE.md`](14-V2-SCOPE.md) turns that into a checkable law and **poses, without answering**,
the question the gate's own wording depends on: `ethos.grounding.v1` is a PDF schema —
`media_type` is `const application/pdf`, every element needs a `page` and a `bbox` — so either it
revises or v2's gate is met at the representation level. [`15-V2-MILESTONES.md`](15-V2-MILESTONES.md)
made that decision S1, ahead of any reader.

**S1 answered it: `ethos.grounding.v1` stays PDF-only.** Revising the artifact is a change to the
**verifier's** contract — the oracle agrees with the pinned Ethos CLI on this exact schema — so an
engine-only revision would fork what the engine does not own. **And the measurement resized S2:**
`DocumentRepresentation::seal` refuses a node whose parent is not a declared page, so a page-less
document cannot become a representation at all. The page assumption is in `engine-core`, not in
grounding, and reaching v2's gate is upstream of the schema question S0 asked.

**S4 adds a SECOND projection, `ethos.html.v1`, under the same four laws.** Not the first one with
angle brackets: GFM has no `rowspan`, so `markdown` must expand a merged cell and count the slots
that costs, while HTML emits one `<td colspan="2">` and carries the merge — on the same fixture the
two artifacts carry different erasure censuses, and a test asserts the difference. Every cell is a
`<td>`, because neither detector reads `/TH` and a header row would be invented. It is projected
from the representation rather than from the Markdown, everything that decides *what* to emit is
shared rather than copied, and one `census` function closes both so the two artifacts **cannot**
disagree about what a document contains. `markdown_rule` did not move. S4 also fixed the
`<` problem entities create: `&lt;` contains no `<` to label, so the whole entity is `source` and
the census counts the character rather than the four bytes.

**S3 joins a hyphenated line break, in the export and nowhere else.** `markdown-blocks-v2` closes
up a word the page drew in two pieces: `hyphen-` + `ated` reads `hyphenated`. The representation is
**untouched** — `extract` still emits both halves with the hyphen verbatim, because telling a soft
break-hyphen from a real compound one needs a dictionary — so the joined word is readable and
**not citable**, and the artifact says which two strings are: the joined bytes are one `source`
segment naming **both** runs, and the removed hyphen is counted in
`hyphenation-rejoin-dropped-v1`. Checklist P15: the export repairs, `element.text` does not.

The golden makes that trade executable, on an authored fixture whose halves both carry measured
ink: the page draws `The rate may be recalcu-` / `lated at closing`, the export reads
`The rate may be recalculated at closing`, and the pinned Ethos CLI **grounds each half** and
returns **`text_mismatch`** for `recalculated` — cited against an element that exists, so the
reason is pinned rather than an `element_not_found` that would prove nothing. Dot-leaders and
drop-caps are not implemented and have no fixture.

**S2 projects BLOCKS.** `markdown-blocks-v1` emitted a GFM table for every table on the
representation and a list item for every run the structure tree places in an `/L` — and, because
GFM has no `rowspan` and no headerless table, it counts what the flattening cost in
`coverage.structural_erasures` rather than footnoting it. `markdown-table-structure-not-projected`
is **deleted**, not reworded; `markdown-table-spans-flattened` is the narrower sentence that is
still true. `TableCellRecord` gains `node_ids`, without which a GFM cell could not be emitted as a
`source` segment at all, so the representation moves to 0.5.0.

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
| `ruled-rects-v2` (S1, S7b) | rectangles the author painted | the document drew this grid |
| `unruled-align-v1` (S2) | where the author placed text | a detector inferred this grid |

Both are cross-checked two independent ways, and fabrication is 0 under both. S3 adds a **second**
check — `tagged-vs-geometric-v1` — comparing the grid a document's tags declare against the grid a
detector found. **The > 0.489 accuracy gate is S7's**, not any of these slices' — see
`08-V1-SCOPE.md` §3 for why chasing it earlier would be tuning against nobody's number.

**S7 has now measured it: macro cell-F1 is 64‰ against the 489‰ floor — a miss.** The method is
[`table-gate-v1.md`](table-gate-v1.md), and it is the only place a table-accuracy number from this
repository should be quoted from. S7b ran seven investigations: the six aimed at the
*alignment* rule were all measured and rejected, and one — `ruled-rects-v2` — shipped, taking
precision from 900‰ to 1000‰ and the gate from 43‰ to 61‰ without losing a true positive, by
refusing to let a page-background panel witness the lattice its own decoration implies.

**S8 shipped the third detection rule**, `stroke-ruled-v1`, for the grid a document draws as
ruling lines rather than filled boxes — the largest measured lead left on this corpus, 103 CFPB
cells. It was built and parked at S7b for regressing the very document it was built for; S8 found
that both its failures were one defect (the page's vertical ink was discarded before the rule saw
it) and shipped the repair: `cfpb-home-loan-toolkit` 246‰ → 259‰, the gate 61‰ → 64‰,
`irs-form-1040-2025` still at 0 tables, fabrication still 0.

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

**Next: closing v1-S7.** The gate is measured and missed at 61‰, and seven investigations in S7b
established that no repair to the *alignment* rule moves it — see
[`table-gate-v1.md`](table-gate-v1.md) for all of them, and [`attic/`](attic/) for the one that was
built and parked. S8 is unstarted.

---

## If you are a coding agent, read this first

1. Read `00-NORTH-STAR.md` — what the product is, and the fourteen decisions that are already made
2. Read `01-CONTRACT.md` — the artifact shape. Frozen before implementation, deliberately
3. Read `03-V0-SCOPE.md` — what is in and out of the first release
4. Read `05-MILESTONES.md` — the ordered work with acceptance tests
5. **M0–M7 are done and v0 is frozen; v0.1 and v1-S1 through v1-S7b shipped on top.** Do not re-author the
   workspace, the contract types, the classifier, the extractor, the assurance envelope, the
   representation, the projection, or the checker — and do not widen the public API without
   editing [`PUBLIC-API.md`](PUBLIC-API.md) and the freeze test in the same commit. For v1 work
   read [`08-V1-SCOPE.md`](08-V1-SCOPE.md) and [`09-V1-MILESTONES.md`](09-V1-MILESTONES.md)
   first: v1 is **seven slices**, S1–S7b are done, the remaining work is closing S7, and the > 0.489 gate belongs to S7 alone

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
| [`09-V1-MILESTONES.md`](09-V1-MILESTONES.md) | **S0–S8**, each with Goal / In / Out / Acceptance / Depends on | Every v1 PR. This is the v1 code-review map |
| [`10-V11-SCOPE.md`](10-V11-SCOPE.md) | What v1.1 is and is not · Workbench rule 8 and why Markdown waited · the four laws of the Anchor Map | Before any v1.1 work |
| [`11-V11-MILESTONES.md`](11-V11-MILESTONES.md) | **v1.1-S0–S4**, all done | Every v1.1 PR. This is the v1.1 code-review map |
| [`12-V12-SCOPE.md`](12-V12-SCOPE.md) | **v1.2 adoption**: what it is, what it is not, and the handle law | Before any adapter |
| [`13-V12-MILESTONES.md`](13-V12-MILESTONES.md) | **v1.2-S0–S5**, all done — S5 **refused** | Every v1.2 PR. This is the v1.2 code-review map |
| [`14-V2-SCOPE.md`](14-V2-SCOPE.md) | **v2 office formats**: what it is, what it is not, the no-synthesised-pages law, and the open grounding question | Before any office-format work |
| [`15-V2-MILESTONES.md`](15-V2-MILESTONES.md) | **v2-S0–S4**, with S0 and S1 done and S2–S4 **not started** | Every v2 PR. This is the v2 code-review map |
| [`table-gate-v1.md`](table-gate-v1.md) | The v1 table gate's **method and result** · corpus · formula · join and text rules · why the number is not comparable to the published 0.489 | Before quoting any table-accuracy number |
| [`PUBLIC-API.md`](PUBLIC-API.md) | The frozen v0 export list, per crate · what is internal and why · the CLI↔library thin-shell mapping | Before adding a `pub use`, or when embedding the engine |
| [`draft-schemas/`](draft-schemas/) | DRAFT JSON Schemas for the M1 types and every artifact through v1.1 — classification, extract, the M5 representation, and `ethos.markdown.v1`. Not a shipped contract | When you need a wire shape |
| [`reference/`](reference/) | The research archive the above was derived from | To check the evidence behind a decision |
| [`attic/`](attic/) | Work that was **built, measured and deliberately not shipped**, as a patch that still applies | Before rebuilding something this project already tried |

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
| Parked work | `attic/` | Built, measured, not shipped. `stroke-ruled-v1` is there with its numbers and its reason |
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
