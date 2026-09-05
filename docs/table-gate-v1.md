# The v1 table gate, and how it is computed

> **v1 closed on this document, 2026-08-30.** Decision #18 in
> [`00-NORTH-STAR.md`](00-NORTH-STAR.md) settled what v1 publishes: the **capability plus the
> band**, never the macro on its own. Everything below is the method and the evidence behind that
> decision; it is the source every reader-facing table number is quoted from. The geometric chase
> stays parked, and the 489‰ comparator is still somebody else's score on their own corpus.

**Measured, and MISSED.** Macro cell-slot F1 is **70‰** over twelve documents, against a published
comparator of 489‰. **That comparator is not a live shipping floor** — the chase for it is parked,
and parking it is not a pass.

**70‰ is the least informative true statement about this corpus.** The band is 0‰..590‰, the median
is 0‰, **ten of the twelve score exactly 0‰**, and removing one document drops the macro to 23‰.

**Micro recall is 4‰, and it is the number the headline was missing.** Pool every gold cell slot
instead of giving each document one vote, and the geometric detectors recovered **70 of 15,755**.
The macro reads 70‰ because it averages two documents that draw rules against ten that get one vote
each for zero; the micro reads 4‰ because it counts cells, **and ten documents contributing nothing
to a pool cannot be averaged back up by two that do.** Both are true of the same corpus and the same
slots. The micro is the one number that says *"two documents carrying ten"* without a band.

**The gate stayed geometric on purpose, and v2-S24 is why that matters.** A fourth rule now reads
the tables the documents *declare* in their structure trees rather than the grids they *draw*, which
closes the gap this whole document circles. **The gate above is deliberately unmoved**, because
scoring a tree-derived table against the tree is circular. The number that moves is **combined micro
recall, 4‰ → 502‰**, and fabrication stays 0.

**Fabrication is 0 on all twelve** — the one number with a required value.

This document states the method completely enough to recompute the number. It is written that way
because of what happens otherwise: **two publishers scored the same tool at 0.000 and 0.693 on
tables, differing only by invocation flags. A table score without its method is not a number.**

## Why this is not comparable to the 0.489 it is named after

**70‰ is this engine on twelve tagged PDFs this repository owns. 0.489 is a published score on
somebody else's corpus.** Different unit **and** different exam — and this document said *"same
unit"* until it was checked.

**0.489 is TEDS.** Tree Edit Distance based Similarity, `1 - EditDist(T_gt, T_pred) /
max(|T_gt|, |T_pred|, 1)`, computed over an HTML DOM with the APTED algorithm. It is
`opendataloader`'s own committed result on the DP-Bench corpus — `teds_mean` **0.4887** — and the
identification is not a guess from one number: the same results file gives its hybrid mode an
`overall_mean` of **0.9066**, which is the 0.907 this repository has always quoted beside it. Both
of the figures in these documents resolve, exactly, against the publisher's own file.

**70‰ is macro-averaged cell-slot F1**, `2TP / (2TP + FP + FN)` over expanded `CellSlot{row,
column}`, §5 below. A tree edit distance over a DOM and a set-overlap over grid slots are not one
unit: they disagree on what a table IS before they disagree on how well one was read.

**The correction makes this document's own case stronger, not weaker.** *"Same unit, different
exam"* invited a reader to subtract 70 from 489 and call the difference a gap. There is no
subtraction to do. The two numbers were never on one scale, and the paragraph that said they shared
one was the single most misleading sentence in this file.

It was used here as a floor to clear and **never as a claim to publish**; since the chase was parked
it is not used as a floor either. The number below is computed on documents this project holds, with
an evaluator written in this repository, against ground truth taken from the documents' own tagged
structure trees. **It shares nothing with 0.489 — not the corpus, not the ground truth, not the evaluator, and not the unit.**

There is no bake-off table in this repository and there will not be one. A reader who wants to
compare this engine to another must run both on one corpus with one evaluator, **and neither this
document nor the README does that.**

## Corpus

Twelve documents, across two roots. It was four for twelve slices, and the reason was not labelling
effort — that is zero, because the labels are derived — but that the four lived in a tree this
repository does not own and cannot write to. **A corpus you publish numbers about has to be one
anybody can re-measure**, so `fixtures/gate/` is committed here.

Engine-owned *fixtures* are deliberately excluded: they are purpose-built to exercise one detector
behaviour each, and **scoring against them measures how well this engine reproduces its own test
cases.**

| Fixture | Pages | Tagged tables | Tagged cells |
| --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 28 | 17 | 159 |
| `irs-form-1040-2025.pdf` | 2 | 1 | 40 |
| `irs-f1040sd-2025.pdf` | 2 | 2 | 60 |
| `irs-fw9.pdf` | 6 | 4 | 60 |
| `nist-sp-800-63b.pdf` | 80 | 13 | 568 |
| `nist-sp-800-53r5.pdf` | 492 | 26 | 6,937 |
| `nist-sp-800-161r1.pdf` | 327 | 46 | 3,853 |
| `nist-sp-800-171r3.pdf` | 120 | 24 | 1,946 |
| `nist-sp-800-207.pdf` | 59 | 4 | 114 |
| `nist-sp-800-218.pdf` | 36 | 4 | 416 |
| `nist-sp-800-37r2.pdf` | 183 | 20 | 1,035 |
| `nist-sp-800-53Ar5.pdf` | 733 | 11 | 567 |
| **Total** | **2,068** | **172** | **15,755** |

Digests live in `fixtures/manifest.json`, which is the single place a digest is recorded —
**restating them here would create a second copy to drift.**

**Every one is hash-pinned, and one was not until the corpus grew.** The document carrying the
largest single share of the gate number had **no manifest entry at all**, so it was pinned by
nothing and the corpus could have changed underneath the score with every test still green. An
earlier slice pinned that gap rather than closing it, and said the day it closed the guard would fail
and bring whoever closed it back to this paragraph. **That is what happened.**

## What qualifies a document

Until the corpus grew there was no rule — the four were the tagged PDFs that happened to be in the
benchmark root, and "add a fifth" had no criterion to satisfy. **That is why the set stayed at four:
not labelling effort, but the absence of an answer to *which* fifth.**

A document is admitted when **all five** hold:

1. **Public** — published by a government body or standards organisation.
2. **Redistributable** — this repository commits the bytes and publishes numbers about them.
3. **Stable at a URL** — the recorded provenance has to lead somewhere.
4. **Tagged** — it carries a structure tree, so its ground truth is *derived* rather than authored
   here. **This is the load-bearing one.** A document that must be hand-labelled is not cheaper to
   add, it is a different kind of thing, and mixing the two would put authored labels and derived
   labels in one average.
5. **Carries at least one table** — a tagged document declaring none contributes nothing to an
   average that excludes it, so admitting one adds weight without adding signal.

### Personal documents are refused, on two grounds rather than one

A machine scan found 400 PDFs on the developer machine, 99 tagged, and the reachable ones carrying
tables are personal — a loan acknowledgement, résumés, a phone receipt.

**Privacy.** A benchmark corpus is committed, referenced by digest, and has numbers published about
it. **That is publication.** A document that was never meant to be published does not become
publishable because it is convenient, **and a digest is not anonymisation.**

**Representativeness.** The gate exists to measure documents like the ones this will meet. A résumé's
two-column layout is not that. Admitting them would move the number **without anyone being able to
say whether the detector improved or the corpus got easier** — which is the failure this whole
document is written to prevent.

### What this rule does not do

It does not make the corpus representative of *everything*. Every document here is
English-language, born-digital, and produced by one of a handful of US federal publishing pipelines.
**A score on this corpus is a score on well-tagged US government publishing**, not on documents in
general — stated here rather than discovered later.

## Ground truth

From each document's **own tagged structure tree**, committed and frozen. A test re-derives and
compares on every run, **so the truth cannot follow the code under test.**

- **Shape** comes from the table, row and cell elements and their span attributes.
- **Cell text** comes from the marked-content ids the tree cites beneath each cell, joined against
  runs on that page by exact `(page, id)` equality.

**No coordinate is read at any step.** That is what makes the gold independent of the geometric
detector it scores — **matching by bounding box would be scoring the detector against itself.**

**A tagged table is the producer's claim, not verified truth.** Producers tag tables for layout as
well as for data, so some are things nobody would want extracted. **That inflates the denominator and
makes the score read worse than the detector deserves. It is left in**, because narrowing the set by
sampling and *then* reporting the number is how a gate gets talked past.

## The metric

**Macro-averaged cell-slot F1, in integer per-mille.**

```
F1(document) = 2·TP / (2·TP + FP + FN)   over cell slots, ×1000, integer division
gate         = mean of F1 over documents declaring at least one table
```

**Integer per-mille, not a float**, for the reason every number in this project is an integer: a
ratio printed to fifteen digits is fifteen digits about floating point rather than about tables, **and
two machines can disagree about them.**

**Macro, not micro.** One document holds 6,937 of the corpus's cells, so a micro average would let
one producer's output decide the gate. **Macro gives each document one vote, so a rule that works on
one and fails on three cannot read as three-quarters right.** (The micro is reported alongside it as
the macro's denominator, printed — not as a second gate.)

A document that declares no table is excluded. **A document that declares tables and detects none
scores 0 and stays in.**

### Slots, not cells

Both sides are expanded to slots before comparison: a merged cell owns every position it covers.
**Comparing cell lists would score a correctly detected 2-column merge as two misses**, because the
tagged half declares spans and the alignment rule never does.

### The join

A labelled table has **no geometry**, so it cannot be matched by overlap.

1. **Page.** A detected table can only join a gold table the tree places on the same page.
2. **Shape, greedily.** Among unclaimed gold tables on that page, the one whose dimensions are
   closest, **ties broken by lower gold index so the result cannot depend on iteration order.**

**Nothing unjoined is dropped.** An unjoined gold table contributes all its slots as false negatives;
an unjoined detected table contributes all of its as false positives.

### The text rule

Applied to **both** sides, then compared exactly: NFC, trim, and collapse every internal run of
whitespace to one space.

**Nothing looser.** No case folding, no punctuation stripping, no edit distance, no model as judge.
A cell reading `1,024` where the page drew `1.024` is wrong. The whitespace concession exists because
a cell split across two text operators and one drawn as a single string are the same text by any
reading, **and the difference is an artifact of how the producer chunked the stream.**

A test pins each clause of that rule against the one the projection actually runs.

## The result

| Document | TP | FP | FN | cell-F1 |
| --- | --- | --- | --- | --- |
| `irs-fw9.pdf` | 26 | 2 | 34 | **590‰** |
| `cfpb-home-loan-toolkit.pdf` | 44 | 136 | 115 | **259‰** |
| `irs-form-1040-2025.pdf` | 0 | 0 | 40 | 0‰ |
| `irs-f1040sd-2025.pdf` | 0 | 0 | 60 | 0‰ |
| `nist-sp-800-63b.pdf` | 0 | 0 | 568 | 0‰ |
| `nist-sp-800-53r5.pdf` | 0 | 0 | 6,937 | 0‰ |
| `nist-sp-800-161r1.pdf` | 0 | 0 | 3,853 | 0‰ |
| `nist-sp-800-171r3.pdf` | 0 | 0 | 1,946 | 0‰ |
| `nist-sp-800-207.pdf` | 0 | 0 | 114 | 0‰ |
| `nist-sp-800-218.pdf` | 0 | 0 | 416 | 0‰ |
| `nist-sp-800-37r2.pdf` | 0 | 0 | 1,035 | 0‰ |
| `nist-sp-800-53Ar5.pdf` | 0 | 0 | 567 | 0‰ |
| **MACRO** | | | | **70‰** |

| | 4 documents | 12 documents (before S20) | 12 documents (now) |
| --- | --- | --- | --- |
| Tables detected / matched | 14 / 13 | 26 / 17 | **17 / 16** |
| Detection precision | 928‰ | 653‰ | **941‰** |
| Cells emitted | 180 | 1,236 | **208** |
| **Fabricated cells** | **0** | **0** | **0** |
| Cross-check disagreements | 0 | 9 | **0** |
| False tables on the gold negatives | 0 | 0 | **0** |

### 64‰ "held" at 70‰, and that is the least informative true thing to say about it

The macro moved 64‰ → 70‰ across a threefold corpus. Read alone, that is stability, and it would
license *"64‰ is this engine's honest table number."* **The band says otherwise, and the band is what
matters.**

- **Ten of twelve documents score 0‰.** Not "low" — zero, because the detector emits **no table at
  all** on them. **The median document scores nothing.**
- **Two documents supply all 849 averaged points.**
- **Remove one and the macro falls to 23‰** — a threefold move from one document out of twelve. That
  is the arithmetic of an average over mostly zeros: **it is stable the way a thermometer reading
  mostly zeros is stable**, and its value is set by which one or two documents happen to have drawn
  rules.

So the four-document 64‰ was **not** a property of this engine, and the twelve-document 70‰ is not
one either. What twelve documents establish that four could not is the **shape**, and the shape is
not "weak everywhere". **It is bimodal: the ruled rule works where a producer drew the rules and
produces nothing where it did not** — confirming on twelve documents the finding that reframed five
slices of work, which four documents could have produced by luck.

### The one document where the failure was not silence

Against 4 tagged tables one document detected **9** — phantom grids up to 103 × 22 spanning whole
pages, contributing **11,295 false-positive slots**, more than the entire rest of the corpus produces
in either direction.

Two things about it are easy to get backwards:

- **Fabrication is still 0.** Every cell's text is a concatenation of runs the page actually drew.
  **The detector arranged real text into a grid that is not there; it did not invent text.**
- **The engine already knew.** All nine cross-check disagreements are that document's, and there are
  exactly nine detected tables there — **so the cross-check was rejecting every one of them.**

**They are ruled-rectangle tables, not stroked ones.** That producer draws its rules as thin *filled*
rectangles, so they fold into a lattice with everything else the page paints. **The distinction is not
pedantic — it decides which rule a repair belongs in**, and the sentence describing them was read the
other way at least once.

11,295 false positives against 1,236 cells emitted corpus-wide is not a contradiction: **the emitted
count counts cells and the score counts slots**, and a cell spanning three columns occupies three.

## v2-S20: the grids the engine already rejected

**Should a table whose own cross-check rejects it be emitted?** The answer shipped is **no when the
rejection is structural.**

**All three options were measured on all twelve documents, and the gate cannot tell them apart** —
the macro reads 70‰ under every one. **So the decision is argued from the artifact, not from the
gate:** the Markdown and HTML projections draw **every** table the artifact carries and consult no
check, so keeping the grid and declaring the disagreement **delivered nine grids to a consumer and
delivered the contradiction to nobody.**

**The cost is twelve cell slots and every one is the empty string** — blank faces agreeing with blank
tagged cells. **No character of extracted text is lost anywhere in the corpus.** As a rate: 1,028
cells emitted and 11,307 slots predicted, to get 12 right — **one per 941 wrong.** This repository
had already reverted a variant that was right about 5% of what it emitted.

**Only the structural half gates, and that was measured rather than reasoned.** Gating on the whole
check refused a 2 × 2 whose only defect is one edge sitting a **single centipoint** out — which is
what a 1 pt stroked rule looks like. The two halves are not the same kind of statement: the
structural one is arithmetic on indices the rule assigned and admits no tolerance, while the
geometric one compares **exact** boxes against a lattice built *with* a tolerance, **so it fires on
the slop that tolerance exists to absorb.**

**The gate lives in one rule because it can only fire in one rule.** The other two build a cell for
every face from the same lines the table's box comes from, so their cells tile exactly — **gating
them would be dead code**, and a test asserts it rather than a comment claiming it.

## v2-S22: why ten documents produce nothing

**The one-line finding: every detected table is a ruled or stroke-ruled one. The alignment rule
emitted none.** All 17 detections are on the two documents that draw their grids. **The engine ships
a rule id, a profile field and a slice of machinery that has never produced a table on a real
document**, and a test now asserts that count stays zero so the claim cannot lapse silently.

**And now we know *which* precondition refuses it, on every gold page in all twelve documents:** the
column-gutter floor. Two adjacent text columns sit closer than 12 pt — **word spacing and running
prose, never a table's column gap.** That is step 2 of the rule, so on no gold page does it ever
reach the whole-page lattice earlier analysis blamed; **it is turned away four steps earlier.** The
floor is doing exactly its job: the gold negatives prove that lowering it buys a fabrication.

**What the ten that detect nothing actually draw**, and neither shape is a tuning target:

- **The NIST family** draws its tables with cell shading and decoration rectangles and at most a
  single margin rule — never a covered cell grid and never a stroked lattice. So the ruled rule sees
  rectangles that explain no grid and the stroke rule sees no rows at all.
- **The IRS forms** draw a real but *partial* grid: a column line left undrawn, or a table that is a
  block of form-field widgets. Both are refused correctly.

**In every case the tables are real, but their geometry is in the tags and the text, not in ink that
forms a grid.**

### The recommendation, named not taken

**The alignment rule is not a gap to close in v1.** It has emitted zero tables on 172 real gold
tables and refuses at the gutter floor on every one. Reaching them means lowering a floor whose only
job is to refuse prose. **So the five geometric-repair slices measured against it were tuning a rule
the gate never exercised, and no seventh repair will move the number.** Whether it should be retired
or reworked is a **version-boundary question**, named for the owner rather than decided.

**v1's remaining gap is that its two working rules require the producer to have drawn the grid.**
Recovering the missed slots is not a tolerance move — **it needs a derivation that reads a table's
geometry from something other than drawn grid ink.**

## v2-S24: the tagged tables the documents declare

**This slice took the derivation the section above named.** It reads the tree's shape and cell text
rather than any coordinate.

**The gate is kept geometric on purpose.** The gate scores the geometric **detectors** against the
document's **tags**, which are independent by construction. A tagged table comes *from* the tree, so
scoring it against the tree would be **the same circularity in reverse: recall near 1.0 that means
nothing.**

**What the tagged emit recovers: combined micro recall, 4‰ → 502‰** — 7,924 of 15,755 gold slots.
**157 of the 172 gold tables** are emitted; the other 15 paired with a geometric detection. The ten
documents that read exactly 0‰ geometric now recover between 313‰ and 745‰ of their gold, and one
recovers all of it. **Fabrication stays 0** across 15,593 tagged cells.

**Why it stops at 502‰ and not 1000‰.** Gold and the tagged emit share the tree derivation, so the
**grid** matches by construction. What differs is the **text**: the gold joins a cell's texts with a
space and the tagged emit concatenates the runs it binds with nothing — the same 27‰ separator gap
recorded below, now applied across all the newly recovered cells. **It is not tuned away: closing it
would mean adopting the gold's join convention, which would make the number measure the harness
rather than the extractor.**

**What a tagged table carries, and what it does not:**

- **`Extracted`, where a geometric table is `Computed`.** The document *stated* the grid; the engine
  did not infer it. **This inverts the usual intuition: the tagged table is the stronger claim.**
- **Geometry typed-absent** under a variant meaning *this source has no box*, distinct from *the
  reader could not measure one*. **No box is invented.**
- **A not-applicable cross-check, never `ok`** — it compares two derivations and a tagged table
  supplies only one.
- **Omitted from grounding**, because that schema requires a box, and disclosed by name.

**What this leaves for decision #18:** whether v1's table number is stated as the geometric gate's
70‰/4‰, or as the capability plus 502‰ combined recall. **This slice reports both and settles
neither.**

## Why the number is what it is

1. **The column-gutter floor is not the cliff.** With it disabled entirely the corpus scores
   identically; disabling the row floor too changes nothing either.
2. **The candidate handed to the alignment rule is the whole page.** Every leftover run becomes one
   lattice, so the face check refuses lattices of tens of thousands of faces. **The floor is not
   wrong; it is unreachable.**
3. **The NIST documents draw no table rulings at all.** Their axis-aligned stroked segments are
   dozens of copies of one margin rule. And several of their tagged tables are **multi-page** — one
   is 278 rows — **which the page-granular join cannot match even in principle.**

## The dead ends, measured

**Segmentation into candidate bands.** Two variants. The better one emitted 141 tables against 10 and
got 72 of 1,302 cells right. **It was reverted, and not because 52‰ still misses the comparator** —
it was reverted because a gold negative became a table, the tax form went from 0 to 6 tables with 0
correct cells, and **making cell starts the column lines routes the gutter floor around itself.**
That removes a fabrication guard, which is forbidden outright. The fabricated-cell count stayed 0:
**real text in invented grids, which is the failure that count cannot see and the gold negatives
can.**

**Marked-content grouping.** Merging runs by the producer's own chunking **changed every number by
nothing** — 217 of 217 tests green. Combined with bands it produced the first non-zero fabrication
count in the project's history (14 cells), traced to a unit spanning two baselines being split by the
reading-order rule. Baseline-scoped merging fixed that and then failed as the bands did.

**Attribution, because it matters:** all three gold negatives carry **zero** marked-content ids, so
the merge is a provable no-op on them — **the negatives cannot certify the merge.**

**Clustering.** Measured and refused: macro 63‰ against 70‰, the corpus's best document falling from
590‰ to 490‰, 112 geometric tables against 18, and about 2,000 added false-positive slots across six
documents. Fabrication stayed 0.

### If a marked-content rule is ever revisited, the gate needs republishing first

**778‰ of gold cells cite exactly one marked-content id.** For those, a detector that merges by id
would reproduce the gold's text **by construction** — so the gate would be measuring the harness. It
would need a published ceiling and a null control before its number could be quoted.

### A known defect in this metric

The gold joins a cell's marked-content texts with a space; the extractor concatenates runs with
nothing. **On 27‰ of gold cells the two conventions give different text**, so the score charges the
extractor for a convention difference. Recorded rather than tuned away, for the reason above.

## The finding that reframes all five dead ends

**Every table the gate scores is a ruled one.** Checked by reading the rule id off each detected
table. **The alignment rule emits none at all on this corpus** — before and after every change tried
against it. **Five slices of work went into a rule the gate never exercised.**

## v1-S8: the stroke-ruled rule, and what shipped

**The parked rule's two failures were one defect: it never read the page's vertical ink.** So "where
are the columns" was decided entirely by where horizontal rules happen to end. The fix moves the
coherence step onto the lines — *every column line interior to a band must be stroked across the
band's full height* — and fixes both symptoms at once.

**And a second precondition, for the tax form.** Under the new rule alone it yields five tables,
because its entry boxes are a stroked grid. **A face whose four edges are a form field's four edges
is that field's box.** Measured, a tax-form face and its widget are the same box edge for edge, while
the worksheet page — also fillable, with 25 widgets — insets its fields well inside larger printed
cells. **No new constant, and neither step names a document, a page or a count.**

**It comes back a 7 × 4 against a tagged 8 × 4, and that is the honest answer.** The worksheet's
header row has no top edge drawn. **A shape match is unreachable while the rule against invented
coordinates holds: the top edge is not in the file.** The decision was taken — ship the 7 × 4 and
declare the offset — and a later slice that wants the 8 × 4 is proposing to write a coordinate no
operator produced.

**What it still gets wrong, stated rather than tuned away.** 136 false-positive cell slots against 12
before, mostly three bands whose interior column lines *are* stroked — **grids by every reading of
the ink, which the structure tree simply does not tag.** Every discriminator that would remove them
is a threshold fitted to these documents. **The decision was to keep them and report the cost:** page
precision 1000‰ → 928‰, carried in the number rather than tuned out of it.

## Pages 16 and 17: nothing to fix, and the cross-check already says so

The last ruled lead was investigated and there is no defect. The rule reconstructs what was painted,
and the tagged-versus-geometric check reports the disagreement slot by slot.

**But the metric cannot see any of it.** The gate charges those pages 4 false positives and 24 false
negatives — and **three of the four false positives are text extracted exactly right, at a row index
a partial detection has no way to know.** Crediting row-offset matches would raise that document to
about 287‰ **and is refused as gate-chasing.**

## What would actually move it

**Both repairs that worked went to rules that read the author's own ink**, and neither came from a
tolerance. The remaining gap is not geometric tuning at all: it is that two of the three working
rules require the producer to have drawn a grid, **and the producers that hold most of this corpus's
cells do not draw one.**

## Gold negatives

Three fixtures draw no grid and imply none this engine may claim, and **all three still yield 0
geometric tables.** They are a safety rail on calibration, not part of the average — they are
engine-owned or synthetic, and **scoring against grids this project built itself measures nothing.**
