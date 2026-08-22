# ethos-engine

An open, high-performance document parser that emits **evidence**: a versioned, fingerprinted
representation in which every node carries a locator back into the source bytes, every capability is
declared on the wire, and everything the parser could not do is stated rather than guessed.

It answers **"what does this document contain, and exactly where?"** It does not answer whether a
claim is true — that is a separate verifier's job. Together they answer the question that matters:
*did this AI claim actually come from this document?*

## Status

**v0 is complete and frozen (0.1.0); v0.1 shipped on top of it as 0.2.0. v1's last slice shipped
as 0.10.0 with S1–S7b and S8 done and S7 open.** Its table number is measured and **missed**: macro
cell-slot F1 is **64‰** with fabrication at **0**, by the method in
[`docs/table-gate-v1.md`](docs/table-gate-v1.md). **The > 0.489 chase is parked** — 64‰ is this
engine on four tagged PDFs this repository owns, 0.489 is a published ODL-local table score on
*their* corpus, and they are the same unit on a different exam. Parking it is not passing it:
**v1 is not done**, and 0.489 is not a number this project publishes as its own.

**What this build can and cannot do, on one page:** [`docs/CAPABILITY.md`](docs/CAPABILITY.md).

**v1.1 is Safe Markdown and it is complete** (S0–S4). **v1.2 is adoption and it is complete at
0.19.0** — the scope document, MCP over stdio, the Python and Node SDKs, LangChain tools over both,
and one adapter measured and **refused**: a `liteparse → ethos.grounding.v1` mapper cannot name its
own producer or declare its box semantics, so it does not ship
([`docs/06-STEAL-REFUSE.md`](docs/06-STEAL-REFUSE.md)).

**v2 is office formats and it reads eight of them at 0.34.0**, with **S10 (CSV)** a
**named refusal** — argued rather than shipped, with its reopening preconditions written down —
and **v2's format row closed and its gate met**. This said *"and v2 **not complete**"* from
**v2-S6** until v2-S14.1, and stopped being true at **v2-S13.4 (0.32.4)**, when the owner settled
the two questions that stood between this repository and the gate as `00-NORTH-STAR.md` decisions
**#16** and **#17**; `docs/CAPABILITY.md` has said so since. **v1 is still not complete** — that is
the sentence below, and it is a different claim. `engine extract` takes a `.docx`, an
`.xlsx`, a `.pptx`, an `.odt`, an `.ods`, an `.odp`, an `.rtf` or an `.epub` and emits the same
record a PDF does — and **no page appears anywhere on that path**. A cell is addressed as its workbook addresses it: sheet, row and column,
with the column kept as the letters the file wrote. An ODT is the sharpest case, because its
`content.xml` **contains an actual page break**: `<text:soft-page-break/>` records where the
producing application's layout fell, so a `PageRecord` would need no arithmetic at all. It is read,
recognised and discarded, because it is a measurement of a word processor rather than of the
document. **An ODS is the sharpest address case**, because OpenDocument writes none: no row number,
no column letter, anywhere. A cell's position is where it sits among its siblings, compressed into
`table:number-columns-repeated="n"` — so the repeat *is* the statement of position, honouring it is
reading, and the column stays a number because the file contains no letters. **And an ODP is the
sharpest refusal**: a presentation lists `<draw:page>` elements — discrete, ordered, named, and
counted out loud by anybody describing a deck — with a master page's `fo:page-width` beside them, so
a `PageRecord` needed **no arithmetic at all**. It is refused anyway, because a draw page is a part
of the presentation's structure rather than a page this engine measured. **And an RTF says the
word out loud** — `\page` is a page break and `\paperw` a paper width — while having no container
at all: no parts, no manifest, no name for itself, so its address is one paragraph number and the
page-less invariant grew a rule rather than the locator acquiring an invented part name. **And an
EPUB is where that law finally had to be argued rather than applied**: §3 has always said *no page
this engine did not read from the file*, and a navigation document's `page-list` really does name
the pages of a print edition. It is refused because a page record is a page with a **width and a
height**, and a publisher's label about somebody else's paper has no geometry for anything to be
validated against. Each row began because the owner asked for it, not because the gate cleared.

Every line of `docs/03-V0-SCOPE.md` §5 is a named CI job, and the public API is a deliberate list
rather than whatever happened to be `pub` ([`docs/PUBLIC-API.md`](docs/PUBLIC-API.md)).

Nine subcommands, one library, one document load:

```bash
engine classify        document.pdf                      # counts and reason codes  · 0 / 1 / 2
engine extract         document.pdf|.docx|.xlsx|.pptx|.odt|.ods|.odp|.rtf|.epub  # DocumentRepresentation v0 · 0 / 2
engine ground          representation.json               # ethos.grounding.v1        · 0 / 2
engine markdown        representation.json               # ethos.markdown.v1         · 0 / 2
engine html            representation.json               # ethos.html.v1             · 0 / 2
engine grounding-check grounding.json --source-artifact document.pdf   # validation  · 0 / 1 / 2
engine verify          grounding.json --citations claims.json --fail-on-ungrounded  # 0 / 1 / 2
engine overlay         document.pdf                      # an annotated PDF          · 0 / 2
engine mcp                                               # MCP over stdio            · 0 / 2
```

**`markdown` never emits Markdown alone.** `ethos.markdown.v1` carries the string *and* the
**Anchor Map** that inverts every source byte of it back to representation nodes, as fields of one
artifact rather than two files — a companion file is a thing a pipeline strips, and a field is not.
There is no `--md-only`. Every byte is `source` (it inverts to node text) or `syntax` (this exporter
invented it), so **a quote touching a `syntax` byte is not a citation**, mechanically. A coverage
census accounts for every source character that did not make it, by named bucket with a count.
`docs/01-CONTRACT.md` §12 refused a Markdown projection for the whole of v1 on Workbench rule 8 —
a projection between what a retriever ranks and what a citation binds is where a locator dies
silently — and the map is what makes that objection payable rather than lapsed.

**`html` is the second projection, and not a rendering of the first.** It is projected from the
representation, because a Markdown-to-HTML pass would be a second projection whose map nobody
built. What earns it a subcommand rather than a stylesheet is tables: GFM has no `rowspan`, so
`markdown` must expand a merged cell and count the slots that costs, while this emits one
`<td colspan="2">` and carries the merge the document drew. Same four laws, same census — the two
artifacts of one document are asserted to agree character for character.

**`engine mcp` serves the engine to an agent, and it will not take a locator from one.** MCP over
**stdio** — newline-delimited JSON-RPC on a pipe, so no HTTP, no socket, no TLS and no async
runtime; the network bans in `deny.toml` stay in force. Three tools: `extract`, `ground`, and
`node_get`.

The hazard is that MCP tools are **model-controlled** — the model chooses the arguments — so a tool
that accepted a `page` or a `bbox` the engine then trusted would make the model the citation
authority in one step, and the result would look exactly like a citation. The mitigation is
structural rather than a prompt: **the engine mints every locator inside an artifact, hands it back
as an opaque handle, and re-validates it on the way in.** `node_get` takes a node id copied from a
representation, checks that representation's fingerprint, and looks the id up among *that*
artifact's nodes; an id the engine did not mint is an **error**, never a nearest match and never an
empty result. No tool argument anywhere names a coordinate, and a test reads the advertised schemas
to keep it that way. Locators travel in `structuredContent`; the text the model reads is counts.

**The two SDKs are further callers of that binary, not second engines.**
[`packages/python/`](packages/python/) and [`packages/node/`](packages/node/) are the same three
functions — `extract`, `ground`, `node_get` / `nodeGet` — spawning the CLI and parsing its stdout
with the language's own JSON parser. **Runtime dependencies are empty in both**, and there is
neither PyO3 nor napi: a native binding would reach the library by a second path, which is a second
thing that can disagree with the first. Wrapping the CLI makes byte-identity a tautology instead of
a promise, and each suite re-canonicalizes what `extract` returned and compares it against the
CLI's stdout byte for byte.

**Node is Python in a second language, not a second design.** Where the two differ it is only where
the languages force it — `nodeGet` versus `node_get`, classes that throw versus exceptions that
raise — and a test pins both to the workspace version, because two adapters at different versions
over one binary is exactly the disagreement a version string exists to make legible.

The handle law carries over unchanged. **No function signature names a coordinate** — no `page`, no
`bbox`, no `x`/`y`, no row/column pair — because a locator is returned and never accepted as prose.
`node_get` is the only function with no subcommand behind it (MCP already has that tool, and a
third CLI verb would exist for symmetry), so c14n v1 is ported into each and pinned against
`engine-core`'s own parity vectors: a minted id returns the node, a forged one **fails closed**, and
an edited artifact fails at its fingerprint before any lookup happens. `markdown`, `html` and
`verify` are deliberately absent from both. **Neither is published** — not on PyPI, not on npm, and
this version does not put them there.

**The LangChain tools are the same three functions, with the locators kept out of the prose.**
Both SDKs expose `extract`, `ground` and `node_get` on a subpath —
[`ethos_engine.langchain`](packages/python/) and [`ethos-engine/langchain`](packages/node/) — as
tools declaring `response_format="content_and_artifact"`. **The artifact carries the record; the
`content` string carries counts.** A box in `content` is a locator a model can edit and then cite,
which is the whole hazard, so the summaries are MCP's own — compared byte-for-byte against what
`engine mcp` emits, and the argument schemas are read off `tools/list` rather than retyped. `node_get`'s
summary names the node's kind and never its id.

LangChain is an **optional extra** and an **optional peer**, so `import ethos_engine` and
`import "ethos-engine"` still pull nothing; importing the subpath without it is a named failure
carrying the install command. There is no LangGraph adapter — a bindable tool is already what
LangGraph binds — no trust state on any result, and no `verify` tool.

**`extract` reads a DOCX and an XLSX too, and refuses to invent a page for either.** Dispatch is by content, never
by extension — a renamed `report.bin` still reads and a `.docx` full of something else is a named
failure — and the artifact is the **same** `DocumentRepresentation v0`: one type, one canonical
JSON, one fingerprint. The reader is [`crates/engine-office/`](crates/engine-office/), the fifth
crate, which existed as a name in the architecture doc until a second format made it real. A run is addressed by `part` + `paragraph` +
`run` — the positions OOXML states about itself — with no page, no box and no `x`/`y`, because
*where* a Word paragraph falls is a decision a renderer makes from a font stack and a paper size.
`pages` is `[]`, every geometry row is typed absence, and `check_structure` **refuses a measured box
on a page-less node** outright, so this is a property of the artifact rather than a habit of the
reader. Headers, footers and footnotes are not read and are **counted and declared**, so a phrase
absent from the record is not read as absent from the document.

**And it reads a workbook, where the same rule bites harder.** A cell is addressed by `part` +
`sheet` + `row` + `column`, and the column stays as the **letters the file wrote** — `B`, not `2`,
because turning one into the other is arithmetic the workbook never performed. A spreadsheet has
column widths, print areas and page breaks, and every one of them describes a *printing* rather
than the file; none is read. The sheet-to-part binding comes from `xl/_rels/workbook.xml.rels`
rather than from position, because a workbook that has had a sheet deleted has `sheet1.xml` and
`sheet3.xml` and guessing would put the wrong sheet's name on the right cells. A workbook is also
the first artifact with **more than one part** — one per sheet — which the page-less invariant
already allowed.

**And it reads a deck, which is the format that tested the rule.** A slide is a real, countable
thing the package contains — the first v2 format where calling a part "page 12" would not have
felt like inventing anything. It is a *part*: `pages` is `[]`, a run is addressed by
`part` + `shape` + `paragraph` + `run`, and there is **no slide number in the locator at all**,
because `p:sldSz` is a size nothing measured and a position in `<p:sldIdLst>` is display order a
consumer would read as a page. Tables, charts and slide-number fields are **counted, not read** —
a field's text is a cached number that goes stale when the deck is reordered.

`ethos.grounding.v1` stays PDF-only, so `engine ground` on any of them is a **named refusal** —
that was decided at v2-S1 and is why no DOCX, no cell and no slide run ever acquires a bbox. `engine mcp` and
both SDKs were not taught anything: `node_get` resolves a DOCX run and a spreadsheet cell because
there is one IR.

**`overlay` is the one subcommand whose stdout is a PDF rather than canonical JSON.** It draws what
was detected — table boxes, image placements, flagged runs — onto a copy of the document, and adds
a per-page note counting what has **no** rectangle to draw. That last part is the point: an overlay
showing only the boxes it has would make a partly-read page look fully read. It annotates and never
edits; the document's own annotations and content streams are untouched.

**`verify` invokes a verifier; it does not verify.** It spawns the pinned Ethos CLI and forwards
its report bytes verbatim — byte-identical to running `ethos verify` with the same arguments. The
engine has no type for a report, a claim or a result, so it cannot re-derive one, and
`ci/forbidden-tokens.sh verification` is the standing proof. A missing verifier exits 2 with **no
report**: never a skip, never a stub, never a default-pass. Which verifier answered is pinned in
the profile by version *and* binary digest, so a swap is fingerprint-visible.

Add `--diagnostics` to any of them for timing, host and input details **on stderr**. stdout is the
artifact and does not change: two runs over the same bytes produce identical files, with the flag
and without.

The artifact contract is Rust (`engine-core`), frozen *before* any parser was written so it is
shaped by what a verifier needs rather than by a parser's accidents. Extraction interprets content
streams against an **exhaustive** operator table — an unrecognised operator stops the parse instead
of being skipped — puts a native locator on every run, and reports an ink box only when it was
measured. `engine-grounding` projects the record into `ethos.grounding.v1` and validates one; the
oracle test compares its answer against the Ethos CLI's on every fixture that reaches an artifact.

- **Start here:** [`docs/README.md`](docs/README.md)
- **What this build can and cannot do:** [`docs/CAPABILITY.md`](docs/CAPABILITY.md)
- **The frozen surface:** [`docs/PUBLIC-API.md`](docs/PUBLIC-API.md)
- **v1 is in progress**, one slice at a time
  ([`docs/09-V1-MILESTONES.md`](docs/09-V1-MILESTONES.md)). **S1 through S8 are done, and S7's gate is measured and missed**: ruled
  tables from vector paths, `CellSlot` occupancy and the locator cross-check (S1); unruled tables
  inferred from text alignment under their own rule id, with every table naming the rule that
  found it (S2); and the document's own tagged-structure tree, read and bound to text by
  `(page, mcid)`, so a node carries the role path its author gave it (S3); and form fields and
  annotations as nodes of their own kind, whose text is never mixed into the page's (S4); and
  multi-column reading order from the page's own whitespace, under `gutter-columns-v1`, so a
  two-column document reads column-major and a single-column one is not touched at all (S5); and
  images as located, fingerprinted nodes, hidden and off-page text reported as findings that never
  remove the text they describe, and an annotated overlay that shows what has **no** box as well as
  what does (S6). S6.1 then repaired a text-loss defect S6's audit surfaced: a simple font's codes were being read two bytes at a time, dropping 8,417 runs from one real document while the artifact blamed that document's fonts. S6.2 then stopped the engine putting rectangles around content that draws nothing — a fabrication that had made two of the three real benchmark documents unreadable outright. S7a then built the measuring instrument: a committed labelled set taken from each document's **own** structure tags — independent of the geometric detector by construction — and a harness reporting recall, precision, fabrication and cross-check disagreement. It reported **157‰ page-level recall and 0 fabricated cells** — an honest number rather than a good one, and a diagnostic rather than the gate. S7b then pointed that harness at the calibration it was scoped to make — and measured that the change does nothing: with the alignment rule's column-gutter floor disabled outright, and then its row floor too, the corpus scores identically. So no detector and no rule id moved, and the real cliff is named instead — the candidate handed to the alignment rule is the whole page. Segmenting the page into candidate bands was then built and measured too: it emits 141 tables against 10 and gets 72 of 1 302 cells right, turns the `unruled-near-miss` gold negative into a table, puts 6 invented grids on the tax form, and routes the gutter floor around itself. Reverted, with the numbers kept. Merging runs into units by their marked-content id — the producer's own chunking of its text — was measured next: on its own it changes **every number by nothing**, and combined with bands it repeats the same failure. Five geometric repairs to the alignment rule are now measured dead ends, and the two that move the score at all do it by emitting an order of magnitude more cells and getting almost none of them right. The finding that reframed all five: **every table the gate scores comes from the *ruled* rule — the alignment rule emits none at all on this corpus**, so five slices of work went into a rule the gate never exercised.

Asking the other question found a real defect and fixed it. The ruled detector required every cell of a candidate grid to be covered by some painted rectangle — and a page-background panel answers yes for every cell at once, while the same detector separately threw that panel away as "the table's own border". One rectangle cannot be both the only evidence a cell exists and not a cell. Two CFPB pages that paint a panel behind highlight bars were emitting a 17 × 13 table holding 12 cells, on a page whose own tags declare no table. `ruled-rects-v2` removes exactly those two and nothing else: **detection precision goes from 900‰ to 1000‰, cross-check disagreements from 2 to 0, and the gate from 43‰ to 61‰, with no true positive lost.** S7b also added the gate the harness was missing, a cell-level one: **macro cell-F1 was 61‰ against the 489‰ comparator**, computed by the method in [`docs/table-gate-v1.md`](docs/table-gate-v1.md) and **missed**. Fabrication stays 0 and the gold negatives stay clean. **v1 is not done**, and the number is written down rather than talked around. The largest lead left was then named, measured, and built: every table the engine misses on the one document that draws real grids is **stroked** rather than filled, 103 cells behind a limitation the engine has always declared. A complete `stroke-ruled-v1` slice finds 29 of those cells that nothing had found before — and emits 236 more that no tag calls a table, regresses the document it was built for, and makes a tax form yield four tables where every slice since v1-S1 has held it at zero. **S7b measured it and did not ship it**, with the numbers kept — and **S8 then found why**: both of its failures were one defect, an `extract` filter that discarded the page's vertical segments before the rule saw them, so "where are the columns" was answered by where horizontal rules happen to end. Reading that ink makes the coherence test ask that every column line interior to a band be stroked across it, which finds page 13's worksheet and refuses the Closing Disclosure furniture in the same change; a second precondition — a cell whose four edges are a form field's four edges is that field's box — holds the tax form at zero. **Shipped as `stroke-ruled-v1` at 0.10.0: CFPB 246‰ → 259‰, the gate 61‰ → 64‰, fabrication still 0, the canary still silent.** Page rasters are a
  named leftover rather than part of S6: rendering needs a PDF renderer, and this build depends on
  no C++ stack and no AGPL code by decision.
- **v1.1 is Safe Markdown** ([`docs/10-V11-SCOPE.md`](docs/10-V11-SCOPE.md),
  [`docs/11-V11-MILESTONES.md`](docs/11-V11-MILESTONES.md)), and it is the version that pays
  §12's objection rather than lifting it. **S1** shipped `ethos.markdown.v1` — the string, the
  Anchor Map, and a character census — with a golden that runs the four real binaries and watches
  the pinned Ethos CLI ground a quote lifted from a `source` segment and **refuse** one spanning
  the blank line between two paragraphs, which is real text in the Markdown that the page never
  drew. **S2** projected blocks: a GFM table for every table and a list item for every run the
  tree places in an `/L`. GFM has no `rowspan` and no headerless table, so what the flattening
  costs is *counted* — `coverage.structural_erasures`, `{code, count}` — rather than footnoted,
  and `markdown-table-structure-not-projected` was **deleted** rather than reworded, because a
  reader acts on a stale limitation. **S3** joins a hyphenated line break **in the export only**:
  the page draws `recalcu-` / `lated`, the Markdown reads `recalculated`, and `extract` still holds
  both halves with the hyphen verbatim. So the joined word reads perfectly and **does not ground** —
  the golden pins each half as grounded and the joined word as `text_mismatch` — while the map
  names the two strings that *are* citable and the removed hyphen sits in a counted bucket.
  Two clauses of that rule were found by measuring: without a baseline test it welded
  `non-` + `escr` into `nonescr` on a real corpus document, and without a furniture test it welded
  a running head onto body text. **S4 (HTML) shipped at 0.14.0**, under the same four laws.

## Building

```bash
cargo test --workspace --locked
```

No exclusion, and no `--skip` anywhere in CI — a test asserts that
(`v0_exit_criteria.rs::no_ci_job_skips_a_test`). `oracle_agrees_on_simple_text` failed by design
from M0 through M5 and runs the real comparison as of M6; filtering it out to get a green build
would delete the only thing proving this engine and the verifier read an artifact the same way.

The oracle harness needs the Ethos repo for its fixture corpus and CLI. It resolves
`../ethos/fixtures` and `../ethos/target/release/ethos` by default; override with `ETHOS_FIXTURES`,
`ETHOS_BENCH_CORPUS` and `ETHOS_BIN`. Absence is a failure, never a skip.

Each SDK is its own suite, and both need a built binary rather than a build of their own:

```bash
pip install -e 'packages/python[dev]' && pytest packages/python
```

```bash
node --test packages/node
```

Node has nothing to install for the core suite: no runtime dependency means no lockfile and no
`npm install`. The LangChain tests need the optional peer and skip with the install command
without it — `npm install --no-save @langchain/core` from `packages/node` runs them. Python's dev
extra already includes `langchain-core`, so its LangChain tests always run.

Both suites refuse a binary that is not this workspace's: they read `engine --version` and compare
it to `Cargo.toml`. A stale `target/release/engine` would otherwise be preferred over nothing and
answer every question plausibly, and a byte-identity check that compares the SDK against the CLI
using the same stale binary is self-consistent — green, and about the wrong engine.

Both packages read `ETHOS_ENGINE` first and **authoritatively** — a path named there and not present
is an error, not a reason to go looking for some other build — then `engine` on `PATH`. Both suites
additionally fall back to the workspace `target/`, so a plain `cargo build` is enough to run them,
and both use the same in-tree fixture PDF, so neither depends on anything outside this repository.
A missing binary fails the run by name; it is never a skip, for the reason the oracle's absence is
never a skip.

## The v0 exit criteria, as CI

`docs/03-V0-SCOPE.md` §5 is fifteen lines, and each one names the job that proves it — so a
reviewer sees *which* criterion is green, not just that nothing failed:

| | |
| --- | --- |
| `v0-happy-path` | the full path over all 15 conformance fixtures |
| `v0-double-run` | byte-identical **files** across two runs, not merely payloads |
| `v0-oracle` | agreement with `ethos grounding check`, fixture by fixture |
| `v0-artifact-identity` · `v0-coordinates` | identity envelope, and a declared frame wherever there is geometry |
| `v0-no-confidence` · `v0-no-verify` | greps over `crates/*/src`; both are jobs, not review items |
| `v0-c14n` · `v0-locators` · `v0-l1-gate` | integers only, a locator on every node, capabilities and limitations on the wire |
| `v0-exit-codes` · `v0-fail-closed` | three distinguishable codes; unknown operator, magic and quantum each refused by name |
| `v0-classify-bound` | 492 pages at N=8 still scans 8 |
| `v0-fixture-mutation` · `v0-fuzz-smoke` · `v0-office-mutation` | every PDF fixture damaged six ways and every office package twelve; `cargo-fuzz` on the PDF entry points and the office router |
| `deny-policy-is-enforced` | `cargo deny`, plus a probe that proves the AGPL rule actually fires |

v0.1's own gates live in a separate `v01-gates` matrix, so v0's map does not move to accommodate
later work: `v01-verify-relay` (bytes relayed verbatim, absence loud), `v01-encoding` (a limitation
or a refusal, never mojibake) and `v01-xref-decision` (one bounded repair, everything outside it
still refused).

`crates/engine-cli/tests/v0_exit_criteria.rs` checks that mapping in both directions, so a renamed
job or an unticked box is a red test rather than a stale document.

## Performance posture

**Bounded, and measured only against itself.** The claim v0 makes is that classification cost does
not scale with page count, and the proof is an instrumented counter rather than a stopwatch: on a
492-page document at N=8, `pages_content_scanned` is 8.

No competitor comparison, ranking, or bake-off table appears anywhere in this repository, and no
performance figure is published that a committed harness does not produce. See
`docs/03-V0-SCOPE.md` §6 for why — the widely-copied figure in this space traces back to a doc
comment with no benchmark behind it.

## What it will never do

Emit a confidence float or any single field meaning "this document is good." Delete hidden text,
headers, footers, or low-confidence content without a record. Invent a coordinate, identifier,
fingerprint, or pagination. Verify a citation. Publish a competitor bake-off table.

## Licence

Apache-2.0, matching Ethos. `NOTICE` is reserved for the vendored Adobe CMap data (BSD-3-Clause). No
AGPL dependencies, enforced by `deny.toml` and proved by the `deny-policy-is-enforced` job.
