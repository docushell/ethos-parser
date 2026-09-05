# Running ethos-parser against `opendataloader-bench`

The adapter and scorer behind the numbers below, committed so they can be re-derived rather than
believed — the same correction [`../block-subdivision/`](../block-subdivision/) makes for its own
measurement, and the one [`17-D1-SCOPE.md`](../../17-D1-SCOPE.md) and
[`18-INTERNING-SCOPE.md`](../../18-INTERNING-SCOPE.md) still need.

## This is an instrument, not a leaderboard entry

[`06-STEAL-REFUSE.md`](../../06-STEAL-REFUSE.md) O26 refuses *"#1, fastest, or any bake-off claim"*,
and [`02-ROADMAP.md`](../../02-ROADMAP.md) lists rankings under **Not scheduled**. Nothing here is a
claim to publish.

What it *is* legitimate for is the thing [`table-gate-v1.md`](../../table-gate-v1.md) §2 says a
reader would have to do to compare two parsers honestly: **run both on one corpus with one
evaluator**, neither of them yours. That is a measurement. Turning it into an ordering is the
separate act that stays refused.

**The units do not travel.** TEDS is a tree edit distance over an HTML DOM; `table-gate-v1.md`
reports macro cell-slot F1. A TEDS number from here **cannot** be placed beside 70‰ — the same
mistake six documents made about 0.489 until it was measured.

## Running it

```bash
git clone https://github.com/opendataloader-project/opendataloader-bench   # Git LFS, 200 PDFs
cd opendataloader-bench && uv sync                                          # its own venv
cargo build --release                                                       # in this repository

cp docs/measurements/opendataloader-bench/pdf_parser_ethos_parser.py <bench>/src/
# register in <bench>/src/engine_registry.py — and do NOT touch the existing `ethos` entry,
# which is the Ethos verifier CLI, a different tool:
#   ENGINES["ethos-parser"] = "0.46.0"
#   _ENGINE_MODULES["ethos-parser"] = "pdf_parser_ethos_parser"

ETHOS_BENCH=<bench> <bench>/.venv/bin/python docs/measurements/opendataloader-bench/score.py
```

## What it measured at 0.46.0

| metric | ethos-parser | what it means here |
| --- | --- | --- |
| **NID** reading order | **0.8490** | 200/200 documents, 0 empty predictions |
| **TEDS** table structure | **0.1038** | 5 of 42 at 0.945–0.980, **37 at zero** |
| **MHS** heading hierarchy | **0.0000** | 0 of 200 documents carry `/StructTreeRoot` |
| speed | **~26 ms/document** | third fastest of fifteen engines measured |

NID was 0.8471 at 0.44.0 and 0.45.0. Timing is quoted loosely on purpose: a run taken while this
machine was loaded reported 34 ms, and an alternating A/B of the two builds over the 25 largest
documents put 0.46.0 within ~2% of 0.45.0. **Quote a speed only from a quiet machine, or A/B it.**

### The corpus is also a limitation census, and that is where the defects are

`extract` over all 200 documents, counting which limitation codes fire on how many, is the most
useful single thing this corpus produces — more useful than any of the three scores, because a
score says *how well* and the census says *where*. At 0.46.0 the document-scoped ones read:

| code | documents | |
| --- | --- | --- |
| `untagged-structure-tree-absent` | 200 (100%) | why MHS is 0, and L29 says it stays 0 |
| `unruled-table-candidate-refused` | 199 | |
| `geometry-absent-not-groundable` | **183** | 8 770 of 109 500 text nodes cannot be quoted — but only **130** for a reason that is this reader's: 5 592 are whitespace runs and 3 047 are drawn off their own page |
| `non-text-nodes-not-projected` | 125 | |
| `ruled-table-candidate-refused` | 57 | the TEDS zeros |
| `composite-font-codes-from-tounicode` | 50 | |
| `form-xobjects-not-descended` | 46 | new at 0.45.0 |
| `broken-font-encoding` | 25 | |
| `font-widths-absent` | **1** | **51 before 0.46.0**, and 50 of those were wrong |

**That census is what found the composite-font defect.** `font-widths-absent` at 51 documents
looked like an honest declaration until the files were checked: 50 of 50 had `/W` or `/DW` and the
reader was looking for `/Widths`, which §9.7.4.3 never puts on a `/Type0`. Ungroundable text nodes
fell 14 683 → 8 770 when that was repaired. See CHANGELOG "0.46.0".

### Two of the three are capped by decisions, not by effort

**MHS cannot move.** Headings come from the tag tree or nowhere
([`markdown.rs`](../../../crates/ethos-parser-core/src/markdown.rs): *"A heading is a heading
because the structure tree said so"*), and no document in this corpus has one. Scoring would mean
inferring headings from font size — checklist **L29**, refused permanently. The number is the price
of that refusal, stated.

**TEDS is the honest band, on somebody else's corpus.** 5 of 42 near-perfect, 37 at zero: where the
producer draws rules the detector is essentially exact, where they do not it emits nothing. That is
[`table-gate-v1.md`](../../table-gate-v1.md)'s bimodality reproduced on documents this repository
does not own — which is the strongest single result here, and it is a *shape*, not a score.
Decision #18 parked the geometric chase, and v2-S20 spent the one concrete lead it offered.

**NID is the one that was work**, and it was a projection defect rather than a reading-order one.

## What running it found, which is the point

Two real defects, neither visible from inside this repository:

1. **Six documents produced no artifact at all** — a measured box outside its own page, refused by
   `seal` on the stated grounds that the transform must be wrong. It was not: the content stream
   drew the text off-canvas. Fixed at 0.42.1, `GeometryAbsence::MeasuredOffPage`. NID 0.7925 →
   0.8109 from that alone.
2. **Every text run was its own block** — 68 112 blocks averaging two characters on
   `nist-sp-800-207`. Fixed at 0.44.0. NID 0.8109 → **0.8471**.
3. **Form XObjects were drawn and counted nowhere** — 46 of 200 documents draw one. Fixed at
   0.45.0. No score moved; the artifact stopped being silent, which is the point.
4. **Every composite font's widths were read from the wrong key** — 50 of 50 documents and 72 of
   72 fonts declared width-absent while the file supplied `/W` or `/DW`. Fixed at 0.46.0.
   Ungroundable text nodes 14 683 → **8 770**; NID 0.8471 → **0.8490** as a side effect.

Three of the four were invisible from inside this repository, and the fourth — the composite-font
one — was invisible to 1 294 tests, because neither owned corpus contained a single CIDFont.

**Use it as a bug-finder, not a scoreboard.** Both defects were found by running someone else's
corpus through this engine and reading what came out; neither was visible in the gate corpus,
because the gate corpus is well-tagged US federal publishing and
[`table-gate-v1.md`](../../table-gate-v1.md) §4 already says a score on it is a score on that.

## A trap this adapter fell into, recorded so it is not repeated

Before 0.44.0 the adapter rebuilt lines itself from `origin_y`/`origin_x`, to work around the
one-block-per-run projection. It lifted NID and **destroyed every table**:

| | NID | TEDS |
| --- | --- | --- |
| adapter joining lines by baseline | 0.8440 | **0.0000** |
| projection verbatim, 0.44.0 | **0.8471** | **0.1038** |

Joining by baseline flattens a GFM table's rows into ordinary lines — 50–65 pipes become none. The
engine now assembles blocks from the document's own marked-content sequences, which the adapter
could not see, and keeps the tables. **The joining variant is deleted rather than kept behind a
flag**: an adapter that second-guesses the projection measures the adapter.
