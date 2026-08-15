# The v1 table gate, and how it is computed

**Status: measured, and MISSED.** Macro cell-F1 is **43‰** against a floor of **489‰**.

This document states the method completely enough to recompute the number. It is written that way
because `docs/06-STEAL-REFUSE.md` records what happens otherwise: two publishers scored the same
tool at 0.000 and 0.693 on tables, differing only by invocation flags. A table score without its
method is not a number.

## The number is not comparable to the 0.489 it is named after

0.489 is a **published table-cell score from a third-party corpus this repository does not have**,
and it is used here as a floor to clear, never as a claim to publish. The number below is computed
on four documents this project happens to hold, with an evaluator written in this repository,
against ground truth taken from the documents' own tagged structure trees. It shares a *unit* with
0.489 and nothing else.

So: this is not ODL-bench. There is no bake-off table in this repository and there will not be
one. A reader who wants to compare this engine to another must run both on one corpus with one
evaluator, and neither this document nor the README does that.

## Corpus

The four real documents. Engine-owned fixtures are **deliberately excluded** from the gate: they
are purpose-built to exercise one detector behaviour each, and scoring against them measures how
well this engine reproduces its own test cases.

| Fixture id | pages | tagged tables | tagged cells |
| --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 28 | 17 | 159 |
| `irs-form-1040-2025.pdf` | 2 | 1 | 40 |
| `nist-sp-800-63b.pdf` | 80 | 13 | 568 |
| `nist-sp-800-53r5.pdf` | 492 | 26 | 6 937 |
| **total** | **602** | **57** | **7 704** |

Their sha256 digests are in `fixtures/manifest.json`, which is the single place they are recorded;
restating them here would create a second copy to drift.

## Ground truth

From each document's **own tagged structure tree** (`pdf-struct-tree`), committed to
`fixtures/labelled/table-truth.json` and frozen — `the_committed_labels_still_match_the_documents`
re-derives and compares on every run, so the truth cannot follow the code under test.

- **Shape** comes from `/Table`, `/TR`, `/TD`, `/TH`, `/RowSpan`, `/ColSpan`.
- **Cell text** comes from the `/MCID`s the tree cites *beneath each cell*, joined against runs on
  that page by `(page object, mcid)` — the same key v1-S3 binds locators with.

No coordinate is read at any step. That is what makes the gold independent of the geometric
detector it scores: matching by bounding box would be scoring the detector against itself.

**A tagged `/Table` is the producer's claim, not verified truth.** Producers tag tables for layout
as well as for data, so some of the 57 are things nobody would want extracted. That inflates the
denominator and makes the score read worse than the detector deserves. It is left in, because
narrowing the set by sampling and *then* reporting the number is how a gate gets talked past.

## The metric

**Macro-averaged cell-slot F1, in integer per-mille.**

```
F1(document) = 2·TP / (2·TP + FP + FN)          over CellSlots, ×1000, integer division
gate         = mean of F1 over documents declaring ≥ 1 table
```

Integer per-mille, not a float, for the reason every number in this project is an integer: a ratio
printed as `0.17241379310344829` has fifteen digits about IEEE-754 rather than about tables, and
two machines can disagree about them.

**Macro, not micro.** `nist-sp-800-53r5` holds 6 937 of the corpus's 7 704 cells. A micro average
would let one producer's output decide the gate. Macro gives each document one vote, so a rule
that works on one and fails on three cannot read as three-quarters right.

A document that declares no table is excluded from the average. A document that declares tables
and detects none scores **0** and stays in.

### Slots, not cells

Both sides are expanded to `CellSlot`s before comparison: a merged cell owns every `(row, column)`
it covers (`docs/01-CONTRACT.md` §5.4). Comparing cell *lists* would score a correctly detected
2-column merge as two misses, because the tagged half declares spans and the unruled rule never
does.

### The join

A labelled table has **no geometry**, so it cannot be matched to a detected table by overlap.

1. **Page.** A detected table can only join a gold table the tree places on the same page.
2. **Shape, greedily.** Among unclaimed gold tables on that page, the one whose `rows × columns`
   is closest, ties broken by lower gold index so the result cannot depend on iteration order.

Nothing unjoined is dropped. An unjoined gold table contributes **all** its slots as false
negatives; an unjoined detected table contributes all of its as false positives.

### The text rule

Applied to **both** sides, then compared exactly:

1. **NFC.**
2. **Trim.**
3. **Collapse** every internal run of Unicode whitespace to a single `U+0020`.

Nothing looser. No case folding, no punctuation stripping, no edit distance, no model as judge. A
cell reading `1,024` where the page drew `1.024` is wrong. The whitespace concession exists because
a cell split across two `Tj`s and one drawn as a single string are the same text by any reading,
and the difference is an artifact of how the producer chunked the stream.

`the_published_whitespace_rule_is_the_one_that_runs` pins each clause.

## The result

Engine **0.9.0**, profile
`sha256:29e4d9acc30e5843905098c70c1493d2b59b07ddbdad6216453caeb73f574ecf`, measured 2026-08-15.

| Document | TP | FP | FN | cell-F1 |
| --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 24 | 91 | 135 | **175‰** |
| `irs-form-1040-2025.pdf` | 0 | 0 | 40 | **0‰** |
| `nist-sp-800-63b.pdf` | 0 | 0 | 568 | **0‰** |
| `nist-sp-800-53r5.pdf` | 0 | 0 | 6 937 | **0‰** |
| **MACRO over 4 documents** | | | | **43‰** |

**Gate: 43‰ > 489‰ is false. v1-S7 is not green.**

Alongside it, on the same run:

| | |
| --- | --- |
| Cells emitted | 77 |
| **Fabricated cells** | **0** |
| Cross-check disagreements | 2 |
| False tables on the gold negatives | 0 |
| Page-level recall (diagnostic only) | 157‰ |
| Page-level precision (diagnostic only) | 900‰ |

Page-level recall is a **diagnostic**. It is not the gate and it is not comparable to 0.489.
157‰ of pages agreeing is not 157‰ of cells right.

### Gold negatives

`synthetic/two-columns`, `synthetic/simple-text` and `unruled-near-miss` draw no grid and imply
none this engine may claim. All three still yield **0** geometric tables. They are a safety rail on
calibration, not part of the average — they are engine-owned or synthetic, and scoring against
grids this project built itself measures nothing.

## Why the number is what it is

Three findings from S7b, each measured rather than inspected. The full evidence is in the CHANGELOG.

1. **The column-gutter floor is not the cliff.** With `unruled::COLUMN_GUTTER_MIN` disabled
   outright — and then with the row floor disabled too — the corpus scores **identically**. No
   value of either constant changes any number here.

2. **The candidate handed to the alignment rule is the whole page.** Every leftover run on a page
   becomes one lattice. A NIST page yields ~96 row lines × ~231 column lines = 22 176 faces from
   1 784 runs, correctly refused. The floor is not wrong; it is unreachable.

3. **The NIST documents draw no table rulings at all.** Their axis-aligned stroked segments are
   490 and 76 copies of a single margin rule. A stroke-ruled detector would add nothing there, and
   on `irs-form-1040-2025` it would re-open the 662-cell fabrication surface of v1-S1.

A fourth, about the measurement rather than the detector: several NIST tagged tables are
**multi-page** — one is 278 rows, another 245 — and the page-granular join cannot match a
multi-page gold table to a per-page detection even in principle. Recorded here rather than fixed by
excluding them, because dropping the documents that score badly is the failure this whole harness
exists to prevent.

## What would move it

Segmenting candidate regions before the lattice, so the existing gutter, coherence and row-major
preconditions judge a table band rather than a page of prose. That is a real detector change with
real fabrication risk, and S7b did not ship it: a probe found that NIST's word-level run splitting
gives each prose line 20–139 distinct x-origins, so consecutive-row column agreement does not
isolate a table there either. Half-enabling it to move this number is the trade
`docs/08-V1-SCOPE.md` §3.3 forbids — fabrication 0 wins when it conflicts with accuracy.
