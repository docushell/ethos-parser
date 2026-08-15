# ethos-engine

An open, high-performance document parser that emits **evidence**: a versioned, fingerprinted
representation in which every node carries a locator back into the source bytes, every capability is
declared on the wire, and everything the parser could not do is stated rather than guessed.

It answers **"what does this document contain, and exactly where?"** It does not answer whether a
claim is true — that is a separate verifier's job. Together they answer the question that matters:
*did this AI claim actually come from this document?*

## Status

**v0 is complete and frozen (0.1.0); v0.1 shipped on top of it as 0.2.0. v1 is at 0.9.0 with
S1–S7b done and S7 open.** Its gate — table-cell accuracy above 0.489 — is measured for the first
time and **missed**: macro cell-slot F1 is **61‰** against a 489‰ floor, by the method in
[`docs/table-gate-v1.md`](docs/table-gate-v1.md). **v1 is not done.**

Every line of `docs/03-V0-SCOPE.md` §5 is a named CI job, and the public API is a deliberate list
rather than whatever happened to be `pub` ([`docs/PUBLIC-API.md`](docs/PUBLIC-API.md)).

Six subcommands, one library, one document load:

```bash
engine classify        document.pdf                      # counts and reason codes  · 0 / 1 / 2
engine extract         document.pdf                      # DocumentRepresentation v0 · 0 / 2
engine ground          representation.json               # ethos.grounding.v1        · 0 / 2
engine grounding-check grounding.json --source-artifact document.pdf   # validation  · 0 / 1 / 2
engine verify          grounding.json --citations claims.json --fail-on-ungrounded  # 0 / 1 / 2
engine overlay         document.pdf                      # an annotated PDF          · 0 / 2
```

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
- **The frozen surface:** [`docs/PUBLIC-API.md`](docs/PUBLIC-API.md)
- **v1 is in progress**, one slice at a time
  ([`docs/09-V1-MILESTONES.md`](docs/09-V1-MILESTONES.md)). **S1 through S7b are done, and S7's gate is measured and missed**: ruled
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

Asking the other question found a real defect and fixed it. The ruled detector required every cell of a candidate grid to be covered by some painted rectangle — and a page-background panel answers yes for every cell at once, while the same detector separately threw that panel away as "the table's own border". One rectangle cannot be both the only evidence a cell exists and not a cell. Two CFPB pages that paint a panel behind highlight bars were emitting a 17 × 13 table holding 12 cells, on a page whose own tags declare no table. `ruled-rects-v2` removes exactly those two and nothing else: **detection precision goes from 900‰ to 1000‰, cross-check disagreements from 2 to 0, and the gate from 43‰ to 61‰, with no true positive lost.** S7b also added the gate the harness was missing, a cell-level one: **macro cell-F1 is 61‰ against the 489‰ floor**, computed by the method in [`docs/table-gate-v1.md`](docs/table-gate-v1.md) and **missed**. Fabrication stays 0 and the gold negatives stay clean. **v1 is not done**, and the number is written down rather than talked around. The largest lead left was then named, measured, and built: every table the engine misses on the one document that draws real grids is **stroked** rather than filled, 103 cells behind a limitation the engine has always declared. A complete `stroke-ruled-v1` slice finds 29 of those cells that nothing had found before — and emits 236 more that no tag calls a table, regresses the document it was built for, and makes a tax form yield four tables where every slice since v1-S1 has held it at zero. **It is measured and not shipped**, with the numbers kept. Page rasters are a
  named leftover rather than part of S6: rendering needs a PDF renderer, and this build depends on
  no C++ stack and no AGPL code by decision.

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
| `v0-fixture-mutation` · `v0-fuzz-smoke` | every fixture damaged six ways; `cargo-fuzz` on the PDF entry point |
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
