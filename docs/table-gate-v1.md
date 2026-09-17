# The v1 table gate, and how it is computed

> **v1 closed on this document, 2026-08-30.** Decision #18 in
> [`00-NORTH-STAR.md`](00-NORTH-STAR.md) settled what v1 publishes: the **capability plus the
> band**, never the macro on its own. Everything below is the method and the evidence behind that
> decision; it is the source every reader-facing table number is quoted from. The geometric chase
> stays parked, and the 489‰ comparator is still somebody else's score on their own corpus.
>
> **Amended 2026-09-16.** *"The geometric chase stays parked"* is left as written, and it no longer
> describes the ruled rule. Between 2026-09-10 and 2026-09-13 that rule moved from `ruled-rects-v3`
> through `-v4` and `-v5` to `-v6`, each step measured and recorded in the v2-S22 subsections below,
> and `-v6` is the rule in force. The rework was the owner's choice, recorded in the local
> implementation plan (untracked) and not in [`00-NORTH-STAR.md`](00-NORTH-STAR.md), so decision
> #18's *"what stays parked"* still reads as it did on 2026-08-30. Whether #18 is amended to record
> the rework is an entry pending the owner ([`OPEN-WORK.md`](OPEN-WORK.md) §4); this paragraph
> records the gap and decides nothing. The twelve-document numbers in this document were not
> re-measured after the rework (`OPEN-WORK.md` §5). The comparator sentence beside it still holds.
>
> **Re-measured 2026-09-16, at 0.58.0.** The twelve-document numbers are re-stated in *The band
> re-stated at 0.58.0* below: macro **69‰**, combined micro recall **503‰**, the band unchanged.
> The paragraph above and the 70‰ tables below it are left as written.

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

### The lattice is oversized too, on a second corpus — 2026-09-10

**This extends the section above rather than correcting it, and one sentence in it needs
qualifying.** Everything here is measured on **opendataloader-bench's 200 documents**, not on the
twelve gate documents, and the two corpora are not interchangeable.
Instrument: [`measurements/table-refusals/`](measurements/table-refusals/).

**Corroborated, from a corpus this repository does not own.** The column-gutter floor is the
precondition that refuses, on 199 of 200 documents, and the other three variants fire **zero** times.
And the floor is not a tuning target: column gaps that failed it come out at median 416 centipoints
whether or not the page holds a table, so lowering it to reach the 42 table-bearing documents would
reach the 157 others too. That is this section's *"lowering it buys a fabrication"*, reproduced
elsewhere.

**Qualified: *"turned away four steps earlier"* describes the control flow, not the candidate.** It
is true that step 2 returns before the lattice-size cap is reached, and this section is right that
earlier analysis could not blame a check the rule never runs. But the lattice was never
exonerated — only unmeasured. Rebuilt out-of-band over 194 pages, the candidate's median size is
**4 628 faces on a page holding prose and 6 672 on one holding a table**, against a
[`MAX_FACES`](../crates/ethos-parser-pdf/src/unruled.rs) ceiling of **4 096**. The typical candidate
is past the cap that exists to refuse it, and `LatticeTooLarge` fires zero times only because step 2
answers first.

So two refusals apply to the same page and the reported one is whichever is tested first. As of
2026-09-10 the size cap is tested **before** the gutter floor, so an oversized lattice is refused as
oversized.

**Measured, both sides.** On opendataloader-bench the reported refusal moves from
`gutter_below_floor` 199 / `lattice_too_large` **0**, to `lattice_too_large` **117** /
`gutter_below_floor` 82 — so **117 of 199 documents were being told a word gap when the truthful
answer was an oversized lattice**, and 27 of the 42 table-bearing pages are among them.

**No table changes anywhere.** Over the eight gate documents the emitted tables are byte-identical
across the reorder: 115 tables, same digests. What moves is the assurance record — **25 of 268
artifacts**, being `extract`, `markdown` and `html` on all eight gate PDFs plus `ground` on two.
Zero engine fixtures move, because a synthetic fixture's lattice is nowhere near 4 096 faces; only
real multi-page documents are.

**The open question this leaves the owner** is whether `table_detection.unruled` moves off
`unruled-align-v1`. The rule detects exactly what it detected before and the tables prove it; the
assurance record differs. That is a contract question about what a rule id promises, and it is not
settled here.

**Corrected on consolidation, 2026-09-10.** Measured on the merged tree the split is
`lattice_too_large` **116** / `gutter_below_floor` **83**, not 117/82. One document moved, and the
cause is a real interaction the two changes had never been tested against each other for: the ruled
coverage rework emits a table on two more pages, and `unruled::detect` runs on `leftover` — the runs
no accepted ruled table already claims (`tables.rs:353`) — so on a page that now emits a ruled
table the alignment rule sees fewer runs, folds a different lattice, and fails a different
precondition first. The ruled side is unchanged by the merge: 7 documents emit, all 7 with a table
in ground truth, zero false positives.

**And no lattice metric separates a table page from a prose page.** Three of five are inverted —
table pages carry *fewer* gutters at or over the floor (48% of pages vs 56%) and a *narrower* widest
gap (median 1 161 vs 1 288). At 180 column lines folded at a 150-centipoint tolerance from every run
on the page, header and footer included, the candidate is a histogram of where words start.

**What that adds to the recommendation below.** This section says recovering the missed slots *"needs
a derivation that reads a table's geometry from something other than drawn grid ink"*. The
measurement narrows it: the missing ingredient is a **bounded region** to build the candidate in. A
page is the wrong scope, and no rule folding every run on a page can be tuned into the right one.
### The ruled coverage rework, and what it actually bought — 2026-09-10

**Measured on `opendataloader-bench`, not on this gate's twelve.** Instruments:
[`measurements/table-refusals/`](measurements/table-refusals/).

Of the 42 documents whose ground truth holds a table, **30 draw rectangles that imply a grid and
every one of those 30 was refused by a single clause** — `a cell the ink does not draw`, at a median
**57%** of implied faces drawn. The cause has a name: a producer laying down row separators and no
column separators has stated exactly where its grid lies while drawing almost none of its cells, and
the precondition counted faces.

`ruled-rects-v4` keeps that test and adds a second: **every row and column boundary carried end to
end by the rectangle edges lying on it**, gaps closed by collinear ink only. Either suffices.

**Neither subsumes the other, which is why it is a widening rather than a swap.** A merged cell
breaks an interior line, so tracing refuses what faces accept — `a_merged_cell_claims_every_slot`
is that case and it caught the first draft, which had replaced the test outright. A rules-only grid
draws no cell, so faces refuse what tracing accepts. **Requiring both would keep all 30 refused.**

**What it bought:** documents emitting a table go **5 → 7**, all 7 with a table in ground truth,
**zero false positives**. Document-level precision 100%, recall 17%.

**Why only two, and it is the same limit twice.** `detect_ruled` builds **one lattice from every
rectangle on the page**, so a line must be traced across the extent of logos, borders and shading as
well as the table's own rules. 53 documents refuse on tracing for that reason. This is §4b's
page-scope finding in the ruled rule rather than the unruled one, and **grouping rectangles into
spatially connected candidate grids is the change that would address it** — named here, not taken.

**`background-panel-not-a-grid` still refuses under both paths**, which is what keeps the widening
honest: the panel is the enclosing border and draws three faces of 49, and while it traces the four
outer lines by definition, three scattered bars cannot span one interior line.

### The band rework: the grid's own rows, not every band its edges imply — 2026-09-10

**Measured on `opendataloader-bench`.** Instruments:
[`measurements/table-refusals/`](measurements/table-refusals/) §4e–§4f.

`ruled-rects-v4` clustered every rectangle edge into lines and treated **every band between them**
as a row or column. A table drawn as separated cell rows has whitespace between those rows, and that
whitespace became a band nothing occupies — so the grid was larger than the page drew and the
missing faces refused it.

`01030000000045.pdf` paints **nine rectangles that are a complete 3 × 3 cell grid**. Its six y edges
clustered into five bands, two of them inter-cell space: 5 × 3 = 15 faces with nine covered, which
is the refusal's own arithmetic. **The rule declined a perfectly drawn grid over two rows it had
invented.** Two more refused documents are the same shape at seven columns.

`-v5` selects bands before anything is asked of the grid, and both acceptance paths then speak of
the rows and columns that exist — tracing especially, since a page must not be required to draw the
gaps it deliberately left.

**And a grid needs two bands on both axes.** That is this section's own *"one face is a box, not a
grid"* carried one step: two faces in a line is two boxes. Selection makes the shape reachable,
because a page of framed form fields collapses to an N × 1. Without the floor the benchmark emits
**17 documents with four false positives, every one single column**; with it, **12 with none**.

| | documents emitting | precision | recall |
| --- | ---: | ---: | ---: |
| before the coverage rework | 5 | 100% | 12% |
| `-v4`, faces or lines | 7 | 100% | 17% |
| **`-v5`, the grid's own bands** | **12** | **100%** | **29%** |

**What is still refused is no longer a lattice problem.** Thirty of the 42 documents draw
rectangles and twelve now emit; the remaining eighteen are the population this section already
named — cell shading and decoration rather than a covered grid, or a partial grid with a column line
left undrawn. Closing those needs ink this engine can read as a boundary, not a different way of
counting the ink it has.

### `-v6`: what `-v5` emitted where the benchmark was not looking — 2026-09-13

**Measured on `opendataloader-bench`, the gate corpus, and the Ethos corpora.** Instrument:
[`measurements/table-refusals/rule_ab.py`](measurements/table-refusals/rule_ab.py), §4g.

**`-v5` fabricated a table on five of the eight gate documents.** The table above says 100%
precision, and on the benchmark it was. On the gate corpus every ruled table `-v5` emitted was
false: NIST's disclaimer, whose every line is shaded by its own full-width rectangle, as a 14 × 5
grid of thirteen cells spanning all five columns, on four documents; and two pairs of empty
full-width bars on a fifth. The columns were never drawn — the lattice is page-wide, and they came
from an underline's ends and ink elsewhere on the page. Band selection had made the stack look
covered.

**And it lost the grids drawn in rules.** A rule thinner than `LATTICE_TOLERANCE` occupies no face,
so band selection by faces kept no band of a rules-only grid, and `-v4`'s two benchmark tables of
that kind disappeared with no refusal.

`-v6` keeps the bands a rule crosses, and requires a grid to be divided inside itself on both axes
by a rectangle that spans a kept band.

| | documents emitting | holding a table | precision | recall |
| --- | ---: | ---: | ---: | ---: |
| `-v4`, faces or lines | 7 | 7 | 100% | 17% |
| `-v5`, the grid's own bands | 12 | 12 | 100% | 29% |
| **`-v6`, rules cross bands; a grid is divided** | **14** | **14** | **100%** | **33%** |

| gate corpus | ruled tables | of them fabricated |
| --- | ---: | ---: |
| `-v4` | 0 | 0 |
| `-v5` | 6 | **6** |
| **`-v6`** | **0** | **0** |

**The lesson is the one this section keeps relearning at a different scale:** a precision measured
on one population is a precision on that population. The gate corpus has no ground truth for
tables, so it cannot score recall, but it can be read — and a table whose cells are the lines of a
paragraph needs no ground truth to be called false.

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

## The band re-stated at 0.58.0 — 2026-09-16

**The twelve-document numbers, re-measured after `ruled-rects-v4` to `-v6`.** The amendment at the
top of this document says they had not been; this section is that measurement. It was run on
2026-09-16 at `main` `b4b4aa9` (workspace 0.58.0, the tree the 0.58.0 release commit describes), as
`cargo test -p ethos-parser-pdf --lib --locked -- --nocapture accuracy::`: exit 0, 14 tests passed.
The cell-slot table under *The result* above is v2-S20's (0.36.0), and v2-S24's run (`d806f83`,
0.37.0, 2026-08-26) printed the same macro, micro and band beside its own combined numbers. No
commit between 0.37.0 and this run records a print.

**Macro cell-slot F1 is 69‰, where *The result* records 70‰.** The harness's cell-slot table, rows
ordered as *The result* orders them (the harness prints corpus order):

| Document | TP | FP | FN | cell-F1 |
| --- | --- | --- | --- | --- |
| `irs-fw9.pdf` | 26 | 2 | 34 | **590‰** |
| `cfpb-home-loan-toolkit.pdf` | 41 | 135 | 118 | **244‰** |
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
| **MACRO** | | | | **69‰** |

**The band is unchanged: 0‰..590‰, median 0‰, ten of the twelve at 0‰.** The best document is
`irs-fw9` at **590‰**, its row identical to the record's. The worst are the ten that detect
nothing — `irs-form-1040-2025`, `irs-f1040sd-2025`, `nist-sp-800-63b`, `nist-sp-800-53r5`,
`nist-sp-800-161r1`, `nist-sp-800-171r3`, `nist-sp-800-207`, `nist-sp-800-218`, `nist-sp-800-37r2`
and `nist-sp-800-53Ar5` — every gold slot of each a false negative, 15,536 slots between them, as
before. Two documents supply all 834 averaged points, where the record's two supplied 849; remove
`irs-fw9` and the macro is 22‰ (244 / 11), where the record says 23‰. **The one row that moved is
`cfpb-home-loan-toolkit`: 44 / 136 / 115 → 41 / 135 / 118, 259‰ → 244‰.** Its 159 gold slots are
what they were; the detectors predict four fewer slots on it (180 → 176), three of which were true
positives.

**Detection, geometric only**, from the harness's first table. The other ten documents declare 151
tables and detect none:

| Document | declared | detected | matched | recall | precision |
| --- | ---: | ---: | ---: | ---: | ---: |
| `cfpb-home-loan-toolkit.pdf` | 17 | 9 | 8 | 470‰ | 888‰ |
| `irs-fw9.pdf` | 4 | 3 | 3 | 750‰ | 1000‰ |
| **TOTAL, twelve documents** | **172** | **12** | **11** | **63‰** | **916‰** |

| | 12 documents, v2-S20 (*The result*) | 12 documents, 0.58.0 |
| --- | --- | --- |
| Tables detected / matched | 17 / 16 | **12 / 11** |
| Detection precision | 941‰ | **916‰** |
| Cells emitted | 208 | **204** |
| **Fabricated cells** | **0** | **0** |
| Cross-check disagreements | 0 | **0** (structurally, since v2-S20) |
| `unruled-align-v1` tables | 0 | **0** |

The record printed no per-document split of its 17 detections. At 0.58.0 they are cfpb's 9 and
`irs-fw9`'s 3, and the numbers place the five that are gone on cfpb: a detection lost on `irs-fw9`
would have moved its slot row, and it did not.

**Micro recall, geometric only: 4‰ — 67 of 15,755 gold slots, 15,688 missed.** The record says 70
of 15,755; the three are cfpb's.

**Combined micro recall, geometric plus tagged: 503‰ — 7,931 true positives, 7,824 missed**, where
v2-S24 recorded 502‰ and 7,924. **Tagged tables emitted 162, tagged cells 15,641, fabricated 0**,
where v2-S24 recorded 157, 15,593 and 0. All 172 gold tables name a page, and `extract.rs` emits a
tagged table for every declared table on its page that no detection paired with by position, so
172 − 162 = **10 gold tables paired with a geometric detection** (15 at v2-S24): 7 on cfpb and 3 on
`irs-fw9`, read off the tagged column below. Per document, as the harness prints it:

| Document | geo-recall | comb-recall | tagged tables |
| --- | ---: | ---: | ---: |
| `cfpb-home-loan-toolkit.pdf` | 257‰ | 496‰ | 10 |
| `irs-form-1040-2025.pdf` | 0‰ | 700‰ | 1 |
| `nist-sp-800-63b.pdf` | 0‰ | 654‰ | 13 |
| `nist-sp-800-53r5.pdf` | 0‰ | 495‰ | 26 |
| `irs-f1040sd-2025.pdf` | 0‰ | 1000‰ | 2 |
| `irs-fw9.pdf` | 433‰ | 450‰ | 1 |
| `nist-sp-800-161r1.pdf` | 0‰ | 437‰ | 46 |
| `nist-sp-800-171r3.pdf` | 0‰ | 695‰ | 24 |
| `nist-sp-800-207.pdf` | 0‰ | 745‰ | 4 |
| `nist-sp-800-218.pdf` | 0‰ | 372‰ | 4 |
| `nist-sp-800-37r2.pdf` | 0‰ | 458‰ | 20 |
| `nist-sp-800-53Ar5.pdf` | 0‰ | 313‰ | 11 |

The ten documents at 0‰ geometric recover between 313‰ (`nist-sp-800-53Ar5`) and 745‰
(`nist-sp-800-207`) of their gold through the tags, and `irs-f1040sd-2025` recovers all of it — the
range v2-S24 recorded. The 27‰ separator gap that keeps the combined number short of 1000‰ is what
it was, and is still not tuned away.

**The verdict line reads `gate is > 489‰: MISS`**, as the harness prints it. The comparator is
parked ([`00-NORTH-STAR.md`](00-NORTH-STAR.md) row 18) and is TEDS on somebody else's corpus (§2
above); the printed verdict stays because parking a chase is not passing it.

### What moved, and what is attributed

Everything that moved is on `cfpb-home-loan-toolkit`, in both directions at once: five geometric
detections gone (17 → 12 corpus-wide, 208 → 204 cells), three geometric true positives with them
(70 → 67), and five gold tables emitted from their tags instead (157 → 162, 15,593 → 15,641 tagged
cells), which recover seven more slots than the detections did (7,924 → 7,931). One change in output
takes the macro 70‰ → 69‰ and the combined recall 502‰ → 503‰.

**Not 0.58.0's work, on its own measurements.** The two 0.58.0 changes that touch cfpb are the ones
[`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md) §9 items 3 and 4 record. Item 3 (`ea82c1e`, `Tw`
reaching a composite font's `<0020>`): cfpb page 17's `=` run advances 0.187 pt further, and its
tables and grounding counts are unchanged. Item 4 (`f8d861d`, the `TJ`-gap space): two cfpb tagged
cells lose their invented spaces ('“I f I lock' → '“If I lock' on page 15; '“C an you', '“H ow is'
and '“H ow does' on page 20), `fixtures/labelled/table-truth.json` was regenerated by exactly those
four deleted spaces, and the commit records the whole printed measurement identical before and
after — and already reading 69‰, cfpb 244‰ and 503‰ (7,931) on the branch's parent. That branch is
cut from `e1032a3`, v0.57.0. So the move lies between v2-S24's run (`d806f83`, 0.37.0, 2026-08-26)
and v0.57.0 (2026-09-14).

**Not attributed to a commit.** No commit message in that window records a run of this harness. A
text or box change can move a slot between true positive and false negative but cannot on its own
remove a detection — the ruled rule builds its lattice from rectangles and the stroke rule its bands
from stroked segments, and both read runs only to fill cells by origin — and four predicted slots
are gone, so the change is in what the geometric rules accept. `stroke_ruled.rs` changed in the
window only under the 0.41.0 rename (`46414a3`). The commits that change what the ruled rule emits
are the four shipped in 0.55.0: `4da0674` (an oversized lattice refused as oversized; "no table
changes anywhere" on the eight `fixtures/gate/` documents), `2a53416` (`-v4`, a widening — either
shape of evidence suffices, so nothing `-v3` emitted is lost), `7bd1a79` (`-v5`: a band no rectangle
occupies is dropped, and a grid needs two bands on both axes — recorded costing "one true 1x3, a
lone header row" on opendataloader-bench) and `b742566` (`-v6`: a grid must be divided by the ink,
and a rule keeps the bands it crosses; both `-v5` floors stay). Their measurements are
opendataloader-bench, the engine fixtures and the eight `fixtures/gate/` documents, and cfpb is in
none of those — it lives in the gate-zero benchmark root (`fixtures/README.md`,
`ETHOS_BENCH_CORPUS`). The `-v6` commit also compared `-v6` against `-v5` over "41 more PDFs from
the Ethos corpora" and found it removes one more disclaimer and adds nothing; the 41 are not named,
and `-v5` against `-v3` was not measured there. So `-v5`'s floors are the candidate the records
point at, and a candidate is not an attribution: naming the commit needs this harness run at each of
the four and at their parent, which this section does not do.

**What this section does not change.** The macro is still the least informative true statement
about this corpus, for the reason the header gives. The reader-facing quotes — `CAPABILITY.md`,
`README.md` and `docs/README.md` — now carry 69‰, 503‰ and 162 of 172 with the same band. Decision
#18's clauses stand as written, and its numbers are the 2026-08-30 record, not rewritten here.

### The cfpb move, attributed — 2026-09-17

**The harness at each commit.** The run the section above says it does not do:
`cargo test -p ethos-parser-pdf --lib --locked -- --nocapture accuracy::` (`-p engine-pdf` at
`d806f83`, before the 0.41.0 rename) in a scratch worktree at each commit below. Every run exited 0
with 14 tests passed and 2 ignored. `d806f83` reproduces the record exactly: cfpb 259‰, macro 70‰,
combined 7,924.

| Commit | What it is | cfpb TP / FP / FN | cfpb cell-F1 | cfpb detected / matched | macro |
| --- | --- | --- | --- | --- | --- |
| `d806f83` | v2-S24, 0.37.0 | 44 / 136 / 115 | 259‰ | 14 / 13 | 70‰ |
| `f6274fc` | the parent of `737d55b` | 44 / 136 / 115 | 259‰ | 14 / 13 | 70‰ |
| `737d55b` | a composite font's widths read from `/W` and `/DW`, 2026-09-06 | 45 / 135 / 114 | **265‰** | 14 / 13 | **71‰** |
| `0517a5e` | the parent of `4da0674` | 45 / 135 / 114 | 265‰ | 14 / 13 | 71‰ |
| `6c3319e` | the parent of `7bd1a79`, after `4da0674` and `2a53416` | 45 / 135 / 114 | 265‰ | 14 / 13 | 71‰ |
| `7bd1a79` | `ruled-rects-v5`, 2026-09-10 | 41 / 135 / 118 | **244‰** | **9 / 8** | **69‰** |

**Two steps, and only one of them among the four candidates named above.**

- **`737d55b` added one true positive.** Before it, text in a composite font had no widths, and
  worksheet cells on pages 6, 7 and 13 came out with letters missing. After it, page 7's
  stroke-ruled 7x2 cell (0,0) reads `Total monthly income after taxes` where it read
  `Thy income after taxes`, and matches its gold. That commit's "no text is gained or lost" holds
  for the extracted runs, not for what lands in a table cell.
- **`7bd1a79` removed four.** `-v5`'s floor in `Lattice::build`, that a grid needs two bands on
  both axes, drops six single-row or single-column ruled tables on cfpb:
  - Four 2x1 boxes, on pages 5, 13, 15 and 20, which each matched one gold slot and missed one:
    1 TP / 1 FP each.
  - The 1x2 on page 17: 2 FP.
  - The 1x2 on page 16: 2 FP. It had also claimed its region ahead of the stroke rule, which now
    emits a 4x2 there with 8 FP.
  - So false positives net to zero at 135; four true positives become false negatives; detections
    go 14 → 9 and cells 208 → 204.
  - The five gold tables no detection pairs with any more (pages 5, 13, 15, 17 and 20) are emitted
    from their tags instead, 5 → 10 on cfpb.

The per-table account comes from `extract` on cfpb at `f6274fc`, `737d55b`, `6c3319e` and `7bd1a79`,
scored by a re-implementation of the harness's matching. It reproduces the four harness rows
exactly, and it is a scratch instrument, not committed.

**What this corrects above.** The loss is `-v5`'s floor, as the records pointed, but the net "three
true positives" is `737d55b`'s +1 and `7bd1a79`'s −4. The macro was not a steady 70‰: it was 71‰
from `737d55b` to `7bd1a79`. `4da0674` and `2a53416` do not move cfpb's row. `b742566` was not
run, and `b4b4aa9`'s recorded row equals `7bd1a79`'s. At `7bd1a79` the NIST documents also
carry seven more detections, 19 against 11 matched corpus-wide, each disagreeing with its document's
tagged table, and `b742566` (`-v6`) removed them. Tests exited 0 through all of it.

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
