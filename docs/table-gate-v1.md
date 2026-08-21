# The v1 table gate, and how it is computed

**Status: measured, and MISSED.** Macro cell-F1 is **64‰**. **489‰ is a published comparator, not a
live shipping floor** — the chase for it is **parked** (`00-NORTH-STAR.md` #10, 2026-08-19), and
parking it is not a pass.
(**61‰** at v1-S7b, under `ruled-rects-v2` alone; v1-S8 added `stroke-ruled-v1` — see below.)

**The method below does not change.** 64‰ is still measured, still reruns to the same value, still
runs in CI, and **fabrication is still 0**. What is parked is treating 489‰ as the number the next
slice must beat.

This document states the method completely enough to recompute the number. It is written that way
because `docs/06-STEAL-REFUSE.md` records what happens otherwise: two publishers scored the same
tool at 0.000 and 0.693 on tables, differing only by invocation flags. A table score without its
method is not a number.

## The number is not comparable to the 0.489 it is named after

**64‰** is this engine on **four tagged PDFs this repository owns**. **0.489** is a published
ODL-local table score on **their** corpus. Same unit, different exam. That incomparability is the
whole reason the chase is parked until this repository has a labelled set it owns and chooses to
resume — the paragraph below is the long form of it, and it predates the park.

0.489 is a **published table-cell score from a third-party corpus this repository does not have**.
It was used here as a floor to clear, and never as a claim to publish; since 2026-08-19 it is not
used as a floor either. The number below is computed
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

**Three of the four** have their sha256 digest in `fixtures/manifest.json`, which is the single
place a digest is recorded; restating them here would create a second copy to drift.

`cfpb-home-loan-toolkit.pdf` **is not in the manifest at all.** The four entries whose notes name
it — `background-panel-not-a-grid`, `simple-font-two-byte-tounicode`, `stroke-ruled-worksheet` and
`stroke-ruled-columns-not-drawn` — are engine-owned fixtures derived from it, not the document
itself, and the `benchmark` root holds three entries where this table names four. So the document
carrying the largest single share of the gate number is pinned by nothing, and the corpus could
change underneath the score with every test still green. This sentence claimed otherwise until
v2-S13.1.

**The gap is pinned rather than closed**, by
`the_gate_corpus_is_pinned_except_the_one_document_that_is_not` in `crates/engine-pdf/src/accuracy.rs`,
which asserts exactly which three are pinned and which one is not. Adding the fourth manifest entry
is a corpus decision with a measurement attached — `fixtures/manifest.json`'s `counts` drive
`crates/engine-pdf/tests/robustness.rs`, so a fourth `benchmark` entry moves the mutation corpus off
its pinned 55 fixtures and 318 mutants — and that is not a patch release's to make. The day it is
made, that test fails and brings whoever makes it back to this paragraph.

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

Engine **0.10.0**, profile
`sha256:08c4207d18bee0e64daea093d94f0c64c5b897add33288969488be5e75143c65`, measured 2026-08-15,
under `ruled-rects-v2` + `stroke-ruled-v1` + `unruled-align-v1`.

| Document | TP | FP | FN | cell-F1 |
| --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 44 | 136 | 115 | **259‰** |
| `irs-form-1040-2025.pdf` | 0 | 0 | 40 | **0‰** |
| `nist-sp-800-63b.pdf` | 0 | 0 | 568 | **0‰** |
| `nist-sp-800-53r5.pdf` | 0 | 0 | 6 937 | **0‰** |
| **MACRO over 4 documents** | | | | **64‰** |

**Gate: 64‰ > 489‰ is false. v1-S7 is still not green** — and 489‰ is now a comparator rather than a
floor the next slice must clear. The verdict stands as a miss; the chase is parked, not passed.

Alongside it, on the same run:

| | |
| --- | --- |
| Cells emitted | 180 |
| **Fabricated cells** | **0** |
| Cross-check disagreements | **0** |
| False tables on the gold negatives | 0 |
| Page-level recall (diagnostic only) | 228‰ |
| Page-level precision (diagnostic only) | 928‰ |

Page-level recall is a **diagnostic**. It is not the gate and it is not comparable to 0.489.
228‰ of pages agreeing is not 228‰ of cells right.

### The run before this one, for comparison

Engine 0.9.0, profile `sha256:5593cb1f…07d2d5c`, under `ruled-rects-v2` alone:

| | 0.9.0 (S7b) | 0.10.0 (S8) |
| --- | --- | --- |
| tables detected / matched | 8 / 8 | 14 / 13 |
| page-level precision | 1000‰ | 928‰ |
| cells emitted | 36 | 180 |
| **fabricated** | **0** | **0** |
| cross-check disagreements | **0** | **0** |
| `cfpb-home-loan-toolkit` | 24 TP / 12 FP → **246‰** | 44 TP / 136 FP → **259‰** |
| `irs-form-1040-2025` | 0 tables → 0‰ | **0 tables** → 0‰ |
| **MACRO** | **61‰** | **64‰** |

**The macro rise is CFPB's alone.** 1040 contributes 0‰ on both sides, which is the whole point:
the parked `stroke-ruled-v1` doubled the macro to 125‰ by turning the canary document into four
tables, and that is the number this slice refused to take.

### The one repair that worked, and how it was found

Under `ruled-rects-v1` this read **43‰**, with 77 cells emitted at 900‰ precision and 2 cross-check
disagreements. The difference is a single defect, found by asking why the *ruled* rule scored what
it did instead of assuming the alignment rule was the problem.

`Lattice::build` required every face to be covered by **some** painted rectangle. A page-background
panel answers yes for every face at once. Two hundred lines away, `detect_ruled` discarded that
same panel as *"the table's own border"* rather than emitting it as a cell. One rectangle cannot be
both the only evidence a face exists and not a cell.

`cfpb-home-loan-toolkit` pages 22 and 23 each paint a 351 × 454 pt panel behind scattered highlight
bars. Page 22 emitted a **17 × 13 table holding 12 cells** on a page whose tree declares no table at
all; page 23 emitted a 23 × 8 against a tagged 5 × 3. Between them they supplied **79 of the 91
false-positive cell slots** charged against the ruled rule, and both cross-check disagreements.

Excluding a lattice-spanning rectangle from being a coherence witness removes exactly those two and
**nothing else**: every other detection on the corpus is unchanged and no true positive is lost.
It is a rule-version event (`ruled-rects-v2`), pinned by the engine-owned fixture
`background-panel-not-a-grid` — 1 175 bytes, the first engine fixture whose geometry is *filled*
rather than stroked, which is part of why this went unnoticed for six slices.

The refusal is now **declared**. Before this the ruled rule had no voice at all: every precondition
failure returned an empty vector and said nothing, so a page whose rectangles implied a grid their
own ink did not draw read exactly like a page that painted nothing — on **556 of the corpus's 602
pages**, measured. It reports under `ruled-table-candidate-refused`, the companion to the alignment
rule's existing declaration, grouped by precondition so that `nist-sp-800-53r5`'s 481 refused pages
cost one explanation rather than 481 copies of it.

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
   490 and 76 copies of a single margin rule. A stroke-ruled detector adds nothing there, and on
   `irs-form-1040-2025` it re-opens the 662-cell fabrication surface of v1-S1 — both since
   confirmed by building it. **True of NIST and 1040, and it was never assessed for
   `cfpb-home-loan-toolkit`, where it became the largest remaining lead — see below.**

A fourth, about the measurement rather than the detector: several NIST tagged tables are
**multi-page** — one is 278 rows, another 245 — and the page-granular join cannot match a
multi-page gold table to a per-page detection even in principle. Recorded here rather than fixed by
excluding them, because dropping the documents that score badly is the failure this whole harness
exists to prevent.

## Segmentation was built and measured. It is a fourth dead end.

The obvious repair for finding 3 is to cut the page into candidate bands before building a lattice,
so the existing preconditions judge a table region rather than a page of prose. S7b implemented it
(`unruled-align-v2`: row lines → cells cut by ≥ 12 pt of whitespace between where one run ends and
the next begins → bands of consecutive rows agreeing on their cell starts) and measured two
variants.

**Variant A — bands, columns still folded from raw run origins.**

| | detected | cell-F1 | note |
| --- | --- | --- | --- |
| baseline | 10 | 43‰ | |
| variant A | 11 | **43‰** | `irs-form-1040-2025` gains a false table |

358 of the 363 bands died on the column-gutter floor, because a cell reading
`Digital Identity Guidelines` is three word-level runs at three x-positions and the fold opens
three column lines a few points apart. No gate movement; one false positive on the canary.

**Variant B — the band's own cell starts become its column lines.**

| Document | detected | TP | FP | cell-F1 |
| --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 13 | 28 | 103 | 193‰ |
| `irs-form-1040-2025.pdf` | **6** | **0** | **37** | 0‰ |
| `nist-sp-800-63b.pdf` | 8 | 2 | 52 | 6‰ |
| `nist-sp-800-53r5.pdf` | 114 | 42 | 1 076 | 10‰ |
| **MACRO** | **141** | **72** | **1 268** | **52‰** |

The rule now fires — 141 tables against 10, and 1 302 emitted cells against 77. It buys **+9‰**.
It is right about 5% of the cells it emits.

**It was reverted, and not because 52‰ still misses 489‰.** It was reverted because of what it
cost:

- **`unruled-near-miss` becomes a table.** That fixture exists to be almost-a-table-and-not-one,
  and a gold negative turning positive is a fabrication by this project's own definition.
- **`irs-form-1040-2025` goes from 0 tables to 6**, with 0 correct cells and 37 wrong ones. That
  document is the canary for v1-S1's 662-cell fabrication, and 6 invented grids on a tax form is
  the same failure at a smaller scale.
- **Three `unruled` unit tests fail**, including `a_face_without_text_refuses_the_whole_lattice`
  and `the_gutter_floor_is_exact_to_the_quantum`. Making cell starts the column lines routes the
  gutter floor around itself — so the variant does not merely score badly, it **removes a
  fabrication guard**, which `docs/08-V1-SCOPE.md` §3.3 and this slice's own terms forbid outright.

Fabricated-cell count stayed 0 throughout: every emitted cell's text really was drawn on the page.
That is worth stating precisely because it is not a defence. The cells were real text in invented
grids, which is exactly the failure `fabricated_cells` cannot see and the gold negatives can.

## MCID grouping was built and measured too. Fifth dead end.

The direction the previous section pointed at: merge runs into units by their marked-content id
before any geometry, so a cell reading `Digital Identity Guidelines` is one unit rather than three
word-level runs. `TextRun.mcid` is copied verbatim off `BDC` and exists in the content stream
whether or not the document is tagged, so it is the producer's own chunking rather than the
structure tree.

**Merge alone: no change at all.** 43‰, 10 tables, 77 cells, 0 fabricated — every number identical
to baseline, and all 217 library tests pass. It compresses well (median 2 302 runs → 278 units per
page, 8.4×) and still cannot help, because the lattice is *still* 74 row lines × 157 column lines =
11 470 faces against a 4 096 cap. The candidate is the whole page; merging changes what is in it,
not how big it is.

**Merge across baselines + bands: 14 fabricated cells.** The first non-zero fabrication count in
this slice. Cause: `extract::reorder_page` remaps `DetectedCell::run_indices` after
`gutter-columns-v1` permutes the page, and that remap only preserves a cell's text because a
table's runs are contiguous in the new order. A unit spanning two baselines can be split by the
reordering, and the remapped indices then concatenate to a different string than the detector
built — a cell claiming text it does not contain.

**Merge within one baseline + bands: fabrication back to 0, and the same failure as before.**
104 tables, 1 086 emitted cells, 26 TP against 1 098 FP, macro **45‰**. It breaks
`unruled-near-miss`, and it *loses a disclosure*: the band filter removes an incoherent candidate
before the coherence check sees it, so a near-miss that v1 declared as `FacesWithoutText` now
silently produces nothing at all. That is standing rule 3 — no silent drop — not merely a bad score.

**Attribution, because it matters.** All three gold negatives carry **zero mcids**, so `units()` is
a provable no-op on them. The near-miss failure is the *bands'*, not the merge's. It also means the
gold negatives are structurally blind to mcid merging and cannot certify it; the only control in
the corpus that exercises it is `irs-form-1040-2025`, whose 1 976 runs all carry mcids.

## If an mcid rule is ever revisited, the gate needs republishing first

An adversarial panel was run on whether this metric can score a detector that reads mcids. Both
lenses agree on the facts and split on the verdict, and the disagreement is worth recording.

**5 998 of the 7 704 gold cells — 778‰ — cite exactly one mcid.** For those, a detector that merges
by mcid produces text that is *byte-identical* to the gold by construction: the same runs, in the
same order, concatenated the same way. The text clause of the metric would stop measuring the
detector on more than three-quarters of the corpus and start measuring the producer's chunking
discipline.

What survives is the entire grid half: gold slots come from `/TR` and `/TD` ordinals and
`/RowSpan`/`/ColSpan`, predicted slots from folding unit origins at `ALIGN_TOLERANCE`. No
coordinate reaches the tree walk and no tree field reaches the detector — `extract.rs` builds
`RunOrigin` from `{x, y, text}` and deliberately omits the `PdfTaggedLocator` sitting on the same
`TextRun`.

The strongest defence of the metric is quantitative: hand the detector perfect cell text *and* a
perfect grid and it ceilings at macro **446‰**, below the 489‰ floor. A gate the shared derivation
cannot clear on its own is not a gate the shared derivation is scoring. The strongest attack is
that the tautological subset alone ceilings well above the floor, so all headroom would be
available without any text inference.

Unresolved, and deliberately left that way: no mcid-reading rule shipped, so the question is not
yet load-bearing. If one is ever proposed, this document must first publish the ceiling and a null
control — an mcid-only "detector" with no geometry at all, scored through the same harness and
asserted to stay near zero — before its number may be quoted.

## A known defect in this metric, found by the same review

The gold joins a cell's mcid texts with a **space**; the detector concatenates its runs with
**nothing**. On **210 of 7 704 cells (27‰)** the two conventions give different gold text, so the
join can insert a separator the page never drew and make a cell unmatchable for a reason that is
the metric's rather than the detector's. Small, real, and stated rather than quietly carried. It is
not corrected here because changing it moves the committed labelled set and the published number in
the same commit that reports them, which is the one edit this document exists to make impossible.

## The finding that reframes all five: the unruled rule scores nothing, and never did

**Every table the gate scores is a ruled one.** Checked by reading `rule` off each detected table:

| Document | ruled | `unruled-align-v1` |
| --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 8 | **0** |
| `irs-form-1040-2025.pdf` | 0 | **0** |
| `nist-sp-800-63b.pdf` | 0 | **0** |
| `nist-sp-800-53r5.pdf` | 0 | **0** |

The alignment rule emits **zero tables on the entire gate corpus**, before and after every repair
tried against it. The whole 61‰ was the ruled detector on the rectangles CFPB actually paints —
and at S8 the whole 64‰ is that plus the stroke-ruled rule on the lines it actually strokes. The
alignment rule still emits nothing here.

So the five alignment repairs were tuning a rule that contributes nothing to the number judging
them. That was not unreasonable — the alignment rule is the only candidate for the three documents
that draw no rulings — but the gate never exercised the code being changed. **Asking the other
question instead is what produced the session's one improvement**: `ruled-rects-v1` was scoring 24
of CFPB's 159 cells while emitting 77, and the excess turned out to be two junk tables from a single
inconsistency, fixed above.

What remains on the ruled side, still unexamined:

| Page | tagged | detected | why |
| --- | --- | --- | --- |
| 9, 10, 21, 25 | 2×3, 4×2, 2×2, 2×2 | none | **the page paints no rectangles at all** — not this rule's job |
| 6, 7 | 7×2, 7×2 | none | 1 and 2 rectangles respectively; nothing like a grid is drawn |
| 16, 17 | 7×2, 5×2 | 1×2, 1×2 | the page paints **part** of the table — see below |
| 13, 8 | 8×4, 2×3 | (second table on the page missed) | |
| 11 | 5×4 | 5×4, 18/20 cells right | the one genuinely painted grid, and it works |

### The second tables on pages 8 and 13, and the lead they uncovered

Two different things again, and only one of them is a table.

**Page 8's second tagged table is layout.** A 2 × 3 reading `YOUR CHOICE Check one:` / `¨ I will go
with the credit I have.` / `OR` / `¨ I will wait a few months…`, two of its six cells empty, drawn
with five scattered text underlines and no grid. It is a checkbox block the producer tagged as a
table — precisely the denominator inflation this document warns about above, and nothing anyone
would want extracted as tabular data.

**Page 13's second tagged table is real, and the document draws it.** An 8 × 4 loan-comparison
worksheet — `LOAN OFFER 1/2/3` across the top, `Lender name`, `Loan amount`, `Interest rate` down
the side, blank cells for the reader to fill in. The page strokes it as **32 horizontal rules in a
perfect grid**:

```
8 baselines   y = 511.4, 457.4, 403.4, 349.4, 295.4, 241.4, 187.4, 124.4
4 segments each   x = 54→210, 210→326, 326→442, 442→558
```

One baseline carries three segments instead of four, matching the gold's blank first cell on that
row. That is the tagged 8 × 4 exactly, drawn in ink, and discarded because a two-point stroked
segment is not a rectangle — `stroke-ruled-tables-not-detected`, which until now was an abstract
limitation with no named instance.

### Stroke-ruled is the largest remaining lead on this corpus

Checking every CFPB page that has a tagged table nothing found:

| Page | missed | cells | segments with real extent |
| --- | --- | --- | --- |
| 6 | 7×2 | 14 | 23 |
| 7 | 7×2 | 14 | 34 |
| 8 | 2×3 | 6 | 7 (underlines, not a grid) |
| 9 | 2×3 | 6 | 7 |
| 10 | 4×2 | 8 | 14 |
| 13 | 8×4 | **32** | 53 |
| 21 | 2×2 | 4 | 4 |
| 23 | 5×3 | 15 | 687 |
| 25 | 2×2 | 4 | 134 |
| | | **103** | |

**Not one missed table is on a page that draws nothing.** All 103 cells — **65% of this document's
gold** — sit on pages that stroke segments the detector throws away.

What it would be worth, if a stroke-ruled rule found them and got them right:

| | cfpb cell-F1 | macro |
| --- | --- | --- |
| at S7b | 246‰ | 61‰ |
| page 13 alone, cells perfect | 493‰ | 123‰ |
| all 103 cells, perfect | 852‰ | 213‰ |
| **what S8 actually got** | **259‰** | **64‰** |

Still short of the 489‰ macro floor, because NIST and 1040 stay at zero and each carry a quarter of
the average. **The forecast row assumed perfect capture and this slice did not get it**: page 13
comes back a 7 × 4 against the tagged 8 × 4 because its header row's top edge is not drawn, which
the cell-slot join charges twice, and the three bands that survive on pages 24 and 25 cost 92 more
false positives. Both are accounted for below. But it is the only measured lead left that moves the number at all, and it is a real
detection rule rather than a tolerance: `flush_subpath` requires 4–5 points with two distinct x and
two distinct y, so a two-point segment produces no rectangle and never reaches the lattice.

### It was then built, measured in full, and not shipped at S7b

`stroke-ruled-v1`, as a complete slice: two-point axis-aligned segments captured as their own
evidence (a `PathSegment` beside `PathRect` — a box says *this cell is here*, a line says *this edge
is here*), then

1. **rule-rows** — horizontal segments sharing a baseline within `LATTICE_TOLERANCE`;
2. **a row must tile** — at least two segments running end to end, so scattered underlining implies
   nothing;
3. **a band** — consecutive rows whose every endpoint is already a column line of the first. A row
   may rule fewer cells than the band has columns (a blank cell does) but may not introduce a
   boundary. Requiring exact equality instead split page 13's worksheet into a 4 × 4 and a 3 × 4 at
   its one blank-first-cell row;
4. **rows are the regions between rules** — *n* baselines bound *n − 1* rows. A form ruled under
   each cell does not draw its first row's top edge and this rule will not supply one;
5. **coherence** — every face's own bottom edge must be drawn;
6. **2 × 2 minimum** and the shared face cap, matching both other rules.

| | `ruled-rects-v2` | + `stroke-ruled-v1` |
| --- | --- | --- |
| detected / matched | 8 / 8 | 21 / 15 |
| precision | 1000‰ | 714‰ |
| cells emitted | 36 | 272 |
| **fabricated** | 0 | **0** |
| cross-check disagreements | 0 | 0 |
| `cfpb-home-loan-toolkit` | 24 TP / 12 FP → **246‰** | 41 TP / 189 FP → **210‰** |
| `irs-form-1040-2025` | 0 TP → 0‰ | 12 TP / 30 FP → **292‰** |
| **MACRO** | **61‰** | **125‰** |

**The lead was real.** It finds 17 CFPB cells and 12 1040 cells no rule had ever found, hits page 7
exactly at 7 × 2, and leaves NIST and all three gold negatives at zero.

**But it refuses page 13** — the worksheet the whole lead was named for. Step 5 wants every face's
bottom edge drawn, and one of the eight baselines rules three cells instead of four, so the band is
declined. The nine tables it does emit are on pages 6, 7, 22, 23, 24 and 25, and the false
positives are concentrated on the Closing Disclosure pages 22, 24 and 25 rather than on scattered
furniture.

**It was not shipped at S7b, for three reasons in this order** — and the next section is how all
three were answered:

1. **It regresses the document it was built for**, 246‰ → 210‰: 17 more right cells bought with 177
   more wrong ones. Classified, 156 of the false positives are text appearing in **no gold cell at
   all** — not an index artifact but genuine over-detection, from page furniture and the Closing
   Disclosure pages.
2. **Four canary tests fail.** `irs-form-1040-2025` yields 4 tables where every slice since v1-S1
   has held it at 0. That guard exists because of the 662-cell fabrication, and its own message
   says flipping it *"is a deliberate decision needing its own evidence"*. **The decision was taken
   and it is to keep 1040 at zero tables.**
3. **The macro gain comes entirely from the canary document.** 1040 going 0‰ → 292‰ is what doubles
   the average, while the document where the rule genuinely helps gets worse. A metric that rewards
   flipping the guard is not the thing that should decide whether to flip it.

Fabrication stayed 0 and the cross-check stayed clean throughout, so this is over-detection rather
than invention — a real distinction, and not a defence: 156 cells of real text in regions nobody
tags as tables is the failure the gold negatives exist to catch.

The rule was **parked, not discarded**:
[`docs/attic/stroke-ruled-v1/`](attic/stroke-ruled-v1/) held it as a patch, with its measurement
and its reason. **v1-S8 revived it, fixed both defects and shipped it** — the next section.

## v1-S8: the same rule, one precondition changed, and it ships

**The parked rule's two failures were one defect: it never read the page's vertical ink.**
`extract` filtered non-horizontal segments away before the rule saw them, so "where are the
columns" was answered entirely by *where horizontal rules happen to end*. That reading is at once
too strict and too loose, and it produced both symptoms above.

`stroke-ruled-v1` as shipped replaces step 5 — "every face's own bottom edge is drawn" — with:

> **Every column line INTERIOR to the band must be stroked as vertical ink running the band's full
> height**, collinear segments joined end to end first. The band's outer edges are exempt: an
> interior line separates two cells, so an author who did not draw one did not divide there, while
> an outer edge is only where the ink stops.

That is the precondition the retired `stroke-ruled-tables-not-detected` limitation itself named
back at v1-S2 — *"every face bounded by four edges rather than covered by one rectangle"* — with
one concession stated out loud, because page 13 strokes its three interior column rules at
x = 210, 326 and 442 and neither outer one. Step 4 already makes the same trade on the other axis.

**And a second precondition, for the 1040.** Under the new step 5 alone the tax form yields *five*
tables, because its entry boxes really are a stroked grid with their column rules drawn. What
settles it is not the geometry but **whose rectangle it is**:

| | a face | a widget `/Rect` there |
| --- | --- | --- |
| `irs-form-1040-2025` p1 | 93.3 … 251.6 × 309 … 321 | **145.0 … 251.2 × 309 … 321** |
| `cfpb-home-loan-toolkit` p13 | 210 … 326 × 334.6 … 388.6 | 231.1 … 321.8 × 349.9 … 376.3 |

On the 1040 the face **is** the field's box, edge for edge. On page 13 — *also* a fillable
worksheet, carrying 25 widgets — the field sits inset inside a larger printed cell, less than half
its height. So: a face whose four edges are a form field's four edges is that field's box, within
the same `LATTICE_TOLERANCE` every other edge comparison uses, and one of them refuses the band.
One is enough because a page cannot half-be a form, and it fails closed.

**No new constant, and neither step names a document, a page or a count.**

### What changed, page by page, on `cfpb-home-loan-toolkit`

| Page | `stroke-ruled-v1` parked | shipped | gold there |
| --- | --- | --- | --- |
| 6 | 5 × 2 | 5 × 2 | 7 × 2 |
| 7 | 7 × 2 | 7 × 2 | 7 × 2 |
| 11 | — | — | 5 × 4 |
| **13** | **refused** | **7 × 4** | **8 × 4** |
| 16 | — | — | 7 × 2 |
| 22 | 5 × 4 and 7 × 4 | **refused**, columns not stroked | none |
| 23 | 3 × 3 (and a 65 × 6 refused) | **refused**, columns not stroked | 5 × 3 |
| 24 | 9 × 4 and 7 × 3 | 9 × 4; the 7 × 3 **refused** | none |
| 25 | 4 × 2 and 8 × 6 | 4 × 2 and 8 × 6 | 2 × 2 |

Pages 11 and 16 read `—` on both sides because a stroke-ruled band **is** built on each and
dropped before emission: `ruled-rects-v2` already owns those regions, and a second grid over a
region a rule with stronger evidence has claimed is the thing detection must not emit. That
arbitration is unchanged from the parked rule.

**Page 13 is found.** It reads `Lender name` / `Loan amount` / `Interest rate` /
`Fixed · Adjustable` / `Monthly principal and interest` / `Monthly mortgage insurance` /
`Total Loan Costs`, with `$` and `%` down the three offer columns — the tagged worksheet, in the
ink the page drew.

### It comes back a 7 × 4 against a tagged 8 × 4, and that is the honest answer

Eight baselines bound **seven** rows. The worksheet's header row — `LOAN OFFER 1 / 2 / 3` — has no
top edge drawn anywhere on the page, and step 4 does not supply one. Emitting an 8 × 4 would mean
writing a coordinate no operator in the file produced.

The cost is real and it is charged twice by this metric: every detected row is compared against the
gold row above it, so the labelled cells all sit one slot off. It is declared rather than absorbed
— `undrawn-table-edges-not-supplied`, the profile limitation that replaces
`stroke-ruled-tables-not-detected` — and pinned by the engine fixture `stroke-ruled-worksheet`,
whose five baselines bound four rows for the same reason.

**The decision was taken, and it is to ship the 7 × 4.** S8's terms asked for a shape match to the
tagged 8 × 4, and that is unreachable while the standing rule against invented coordinates holds:
the top edge is not in the file. Anyone proposing the 8 × 4 later is proposing to write a
coordinate no operator produced, and owes their own evidence for it.

### What it still gets wrong, stated rather than tuned away

**136 false-positive cell slots against 12 before**, and the great majority are three bands:
page 24's 9 × 4 and page 25's 4 × 2 and 8 × 6 — Closing Disclosure pages whose structure tree tags
almost nothing. Those bands are **not** the over-detection the new step 5 was built to kill: their
interior column lines really are stroked across the band, so by every reading of the ink they are
grids the author drew. The gold does not tag them.

**They were left, and that too was a decision rather than an omission.** Every discriminator that
would remove them — a minimum band width, a row-height regularity test, a page-region test — is a
threshold fitted to these four documents, which `docs/08-V1-SCOPE.md` §3 forbids, and the honest
reading is that this is denominator inflation running the other way: a producer who does not tag a
table is not evidence that no table is there. The cost is recorded here and in the precision figure
(1000‰ → 928‰) rather than tuned out of the number.

**Two truncated cells**, also recorded: page 13's `onthly principal and interest` and
`Total Loan Costs (ection D …)`. The producer split those strings and the leading `M` and `S` are
drawn at an x *left of* the band's outer column line, so their runs fall outside the table. Cell
assignment is by run **origin** for all three rules, and widening the band to catch them would be
inventing the outer edge step 4 refuses to invent. Not fabrication — `fabricated_cells` is 0 and
the cross-check is `ok` on that table — but a measured loss, and it is why page 13's 18 matching
slots are not more.

**What a future attempt should know.** The band preconditions are sound and reproduce. What is
left on this document is not the detector: it is the header rows nobody drew a box around, and the
untagged Closing Disclosure grids. Neither is reachable without either inventing a coordinate or
fitting a threshold.

## Pages 16 and 17: nothing to fix, and the cross-check already says so

Both were investigated and **neither is a detector defect.** They are also not the same shape,
which an earlier revision of this document got wrong:

- **Page 17** paints two full-height column panels — 165 × 214 pt and 339 × 214 pt, side by side.
  The five rows of text sit inside them, unpainted.
- **Page 16** paints a single **61 pt** two-cell band, behind one row of a seven-row table. Not
  columns at all.

In both cases the ruled rule reconstructs exactly what was painted — a 1 × 2 grid — and the
artifact reports the disagreement with the tags in full:

```
p16  Mismatch { RowCountDiffers { tagged: 7, detected: 1 }, SlotOnlyInTagged ×12 }
p17  Mismatch { RowCountDiffers { tagged: 5, detected: 1 }, SlotOnlyInTagged ×8  }
```

That is `tagged-vs-geometric-v1` doing the job v1-S1 built it for: two derivations that share no
input disagree, and the artifact names every slot the tags have and the geometry does not. Nothing
is invented, nothing is repaired, and a consumer can see precisely what was and was not found.

**And the metric cannot see any of it.** The gate charges these pages 4 false positives and 24
false negatives. Measured, three of those four "false positives" are text the detector extracted
**exactly right**:

| | detected text | matches gold at | scored |
| --- | --- | --- | --- |
| p16 r0 c0 | ✓ | row **5**, col 0 | FP + FN |
| p16 r0 c1 | ✓ | row **5**, col 1 | FP + FN |
| p17 r0 c0 | ✓ | row **2**, col 0 | FP + FN |
| p17 r0 c1 | — | no gold cell | FP |

A partial detection numbers its own rows from zero, so a correctly extracted row 5 is compared
against gold row 0 and charged twice. Crediting a match at any row offset would take
`cfpb-home-loan-toolkit` from 246‰ to roughly 287‰ — and it is **not** done, because a join
loosened until the detector scores better is the failure `docs/08-V1-SCOPE.md` §3 exists to
prevent. It is recorded here instead, as a property of the metric rather than a fault of the
engine.

## What would actually move it

Nothing in the geometric-alignment family, on this evidence. Six repairs aimed at it have been
measured — the gutter constants, band segmentation, mcid merging, mcid merging with bands — and
the ones that move the number do so by emitting an order of magnitude more cells, getting almost
none right, and breaking a gold negative or the 1040 canary each time.

**Both repairs that worked went to rules that read the author's own ink**, and neither came from a
new idea about tables:

- `ruled-rects-v2` (S7b), +18‰ macro, precision to 1000‰, no true positive lost — from reading the
  detector's own two statements about one rectangle and noticing they contradicted each other.
- `stroke-ruled-v1` (S8), +3‰ macro and +13‰ on the one document that draws grids — from noticing
  that the parked rule's *two* opposite failures were one line in `extract` throwing away the
  page's vertical segments, so a precondition about columns was being answered without ever
  looking at whether a column was drawn.

The pattern in both: the defect was a place where the engine had two beliefs about one piece of
ink and had never put them side by side.

Two facts bound what any of them could have achieved:

- **The gold declares no spans at all.** All 7 704 cells are 1 × 1, so gold slots equal gold cells.
- **Coherence forbids an empty cell**, so the unruled family can never emit any of the 1 234 empty
  gold cells, and a full-lattice emission charges every miss to both FP and FN.

Together those give a hard ceiling for the degenerate half of the metric: grant the detector a
perfect grid and free text wherever a single marked-content unit supplies it, and F1 collapses to
TP/G — 72/159, 11/40, 328/568, 3352/6937, **macro 446‰**. Below the 489‰ floor. That is the
quantitative reason the gate is not gameable by an mcid rule, and it is also the reason no amount
of text accuracy alone would have cleared it.
