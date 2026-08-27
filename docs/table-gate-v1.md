# The v1 table gate, and how it is computed

**Status: measured, and MISSED.** Macro cell-F1 is **70‰**, over twelve documents. **489‰ is a
published comparator, not a live shipping floor** — the chase for it is **parked**
(`00-NORTH-STAR.md` #10, 2026-08-19), and parking it is not a pass.
(**61‰** at v1-S7b, under `ruled-rects-v2` alone; v1-S8 added `stroke-ruled-v1`; **64‰** was the
four-document figure from v1-S8 until v2-S19 grew the corpus — see below.)

**This line read `64‰` from v1-S8 until v2-S20, and stopped being true at v2-S19.** S19 measured
twelve documents at **70‰** and wrote that number into `00-NORTH-STAR.md` #10 and
`07-VERIFY-BOUNDARY.md`, and into this document's own §"The result" and §"64‰ 'held' at 70‰" — and
left the headline three lines above them saying something else. A document whose first paragraph
contradicts its own body is worse than one that is merely out of date, because a reader who stops
at the summary is misinformed by the part written to save them the reading. It is repaired here,
in the slice that had to open the file anyway.

**And 70‰ is the least informative true statement about this corpus**, which §"64‰ 'held' at 70‰"
argues at length: the band is 0‰..590‰, the median is 0‰, ten of the twelve score exactly 0‰, and
removing `irs-fw9` alone drops the macro to 23‰. (This said *nine* until v2-S22 — the tenth,
`nist-sp-800-218`, lost its 2‰ at v2-S20 along with the 11 295 false positives that were its
denominator, and the headline lagged the body's §"The result", which had it right.)

**Micro recall is 4‰, and it is the number this headline was missing** (v2-S22). Pool every gold
cell slot in the corpus instead of giving each document one vote, and the detector recovered **70 of
15 755** — the 70 true positives against the 15 685 slots it missed. The macro reads 70‰ because it
averages two documents that draw rules against ten that get one vote each for zero; the micro reads
4‰ because it counts cells, and ten documents contributing nothing to a pool cannot be averaged back
up by two that do. Both are true of the same corpus and the same slots. The micro is the one number
that says *"two documents carrying ten"* without a band, and it is stated here beside the macro for
exactly that reason — not as a second gate, of which this repository has none, but as the macro's
denominator, printed. The headline number is here because a reader expects one, not because it is
the number worth quoting.

**The gate stayed geometric, and v2-S24 is why that matters.** That slice added `tagged-tables-v1`,
a rule that reads the tables the documents *declare* in their structure tree rather than the grids
they *draw*. It closes the gap this whole document circles — the two working geometric rules need a
drawn grid, and the NIST producers do not draw one. But the gate above is unmoved at **70‰/4‰**,
deliberately: scoring a tree-derived table against the tree is circular, so the tagged tables are
measured apart. The number that moves is **combined micro recall, 4‰ → 502‰** (7 924 of 15 755 gold
slots), and fabrication stays 0. See §"v2-S24".

**The method below does not change.** 70‰ is still measured, still reruns to the same value, still
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

**Twelve documents, across two roots.** It was four until v2-S19, and the reason it stayed four for
twelve slices was not labelling effort — that is zero, because the labels are derived — but that
the four lived in a tree this repository does not own and cannot write to. `fixtures/gate/` is the
answer: an engine-owned root, committed here, so the corpus can grow and anybody can re-measure it.

Engine-owned *fixtures* are still **deliberately excluded**: they are purpose-built to exercise one
detector behaviour each, and scoring against them measures how well this engine reproduces its own
test cases. `fixtures/gate/` is a different thing — real documents this repository happens to store.

| Fixture id | root | pages | tagged tables | tagged cells |
| --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | benchmark | 28 | 17 | 159 |
| `irs-form-1040-2025.pdf` | benchmark | 2 | 1 | 40 |
| `nist-sp-800-63b.pdf` | benchmark | 80 | 13 | 568 |
| `nist-sp-800-53r5.pdf` | benchmark | 492 | 26 | 6 937 |
| `irs-f1040sd-2025.pdf` | gate | 2 | 2 | 60 |
| `irs-fw9.pdf` | gate | 6 | 4 | 60 |
| `nist-sp-800-161r1.pdf` | gate | 327 | 46 | 3 853 |
| `nist-sp-800-171r3.pdf` | gate | 120 | 24 | 1 946 |
| `nist-sp-800-207.pdf` | gate | 59 | 4 | 114 |
| `nist-sp-800-218.pdf` | gate | 36 | 4 | 416 |
| `nist-sp-800-37r2.pdf` | gate | 183 | 20 | 1 035 |
| `nist-sp-800-53Ar5.pdf` | gate | 733 | 11 | 567 |
| **total** | | **2 068** | **172** | **15 755** |

Digests are in `fixtures/manifest.json`, which is the single place a digest is recorded; restating
them here would create a second copy to drift.

**Every one of the twelve is now hash-pinned, and one of them was not until this slice.**
`cfpb-home-loan-toolkit.pdf` had no manifest entry at all — the four entries whose notes name it
(`background-panel-not-a-grid`, `simple-font-two-byte-tounicode`, `stroke-ruled-worksheet`,
`stroke-ruled-columns-not-drawn`) are engine-owned fixtures *derived* from it, which are different
files. So the document carrying the largest single share of the gate number was pinned by nothing,
and the corpus could have changed underneath the score with every test still green.

v2-S13.1 pinned that gap rather than closing it, because closing it moves the mutation harness, and
said the day it closed the guard would fail and bring whoever closed it back to this paragraph.
**That is what happened**: S19 had to touch the manifest to grow the corpus, so the gap closed in
the slice that could pay for it. The guard is now
`every_gate_document_is_hash_pinned` — a universal with no exception list, because a
pinned-versus-not comparison has nothing left to say once the second list is empty and would only
invite someone to add a document to the wrong side of it.

**What it cost, stated because the warning that predicted it is above.**
`fixtures/manifest.json`'s `counts` drive `crates/engine-pdf/tests/robustness.rs`, and nine new
entries — the cfpb pin plus eight `gate` documents — moved the mutation corpus from **55 fixtures
and 318 mutants** to **64 fixtures and 363 mutants**. `EXPECTED_INAPPLICABLE` went from 12 pairs to
**21**, and every one of the nine additions is the same `unknown-operator`-on-a-compressed-content-
stream case the array already documented: these are real publications from government typesetting
pipelines, which is the very property that got them admitted. **The count moved and the reason did
not** — no mutation stopped covering anything it used to cover. The `gate`
root takes the **shallow** mutation pass, as `benchmark` does: the eight added documents run to
1 466 pages and deep-mutating them would add hours per run and no signal the small fixtures do not
already give. Naming roots is a proxy for size and a crude one; a `depth` field per manifest entry
would say it directly, and that is a schema change named here rather than taken quietly.

## What qualifies a document for this corpus

Until v2-S19 the corpus was four documents and no rule — they were the tagged PDFs that happened to
be in the benchmark root, and "add a fifth" had no criterion to satisfy. That is why the set stayed
at four for twelve slices: not labelling effort, which is zero, but the absence of an answer to
*which* fifth.

A document is admitted when **all five** hold:

1. **Public.** Published by a government body or a standards organisation, for anyone to download.
2. **Redistributable.** This repository commits the bytes and publishes numbers about them, so a
   licence that permits neither is disqualifying. US Government works are in the public domain.
3. **Stable at a URL.** The provenance recorded in `fixtures/manifest.json` has to lead somewhere.
4. **Tagged.** It carries a `/StructTreeRoot`, so its ground truth is *derived* from the producer's
   own structure tree rather than authored by anyone here. This is the load-bearing one — see
   §"Ground truth". A document that must be hand-labelled is not cheaper to add, it is a different
   kind of thing, and mixing the two would put authored labels and derived labels in one average.
5. **Carries at least one `/Table`.** A tagged document declaring no table contributes nothing to a
   macro average that excludes it (§"The metric"), so admitting one adds weight without adding
   signal.

### Personal documents are refused, on two grounds rather than one

A scan of the developer machine — run **before** this slice, and inherited by it rather than
repeated — found 400 PDFs, 99 of them tagged, and the reachable ones carrying `/Table` are
personal: a loan acknowledgement, résumés, a phone receipt. They are refused twice over, and the
second reason matters as much as the first.

**Privacy.** A benchmark corpus is committed, referenced by digest, and has numbers published about
it. That is publication. A document that was never meant to be published does not become publishable
because it is convenient, and a digest is not anonymisation.

**Representativeness.** The gate exists to measure documents like the ones DocuShell will meet. A
résumé's two-column layout is not that, and a phone receipt's is not either. Admitting them would
move the number without anyone being able to say whether the detector improved or the corpus got
easier — which is the failure this whole document is written to prevent. **A corpus you cannot
publish is also a corpus nobody can check**, and the two objections point the same way.

### What this rule does not do

It does not make the corpus representative of *everything*. Every document here is
English-language, born-digital, and produced by one of a handful of US federal publishing pipelines.
That is a real bound on what the gate number generalises to, and it is stated here rather than
discovered later: **a score on this corpus is a score on well-tagged US government publishing**, not
on documents in general.

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

Engine **0.36.0**, measured on **twelve** documents. The 0.35.0 column is kept beside it because
the comparison is this slice's whole finding, and the four-document run both replace is kept below.

| Document | TP | FP | FN | cell-F1 | at 0.35.0 |
| --- | --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 44 | 136 | 115 | **259‰** | 259‰ |
| `irs-fw9.pdf` | 26 | 2 | 34 | **590‰** | 590‰ |
| `irs-form-1040-2025.pdf` | 0 | 0 | 40 | **0‰** | 0‰ |
| `irs-f1040sd-2025.pdf` | 0 | 0 | 60 | **0‰** | 0‰ |
| `nist-sp-800-63b.pdf` | 0 | 0 | 568 | **0‰** | 0‰ |
| `nist-sp-800-53r5.pdf` | 0 | 0 | 6 937 | **0‰** | 0‰ |
| `nist-sp-800-161r1.pdf` | 0 | 0 | 3 853 | **0‰** | 0‰ |
| `nist-sp-800-171r3.pdf` | 0 | 0 | 1 946 | **0‰** | 0‰ |
| `nist-sp-800-207.pdf` | 0 | 0 | 114 | **0‰** | 0‰ |
| **`nist-sp-800-218.pdf`** | **0** | **0** | **416** | **0‰** | **2‰** (12 / 11 295 / 404) |
| `nist-sp-800-37r2.pdf` | 0 | 0 | 1 035 | **0‰** | 0‰ |
| `nist-sp-800-53Ar5.pdf` | 0 | 0 | 567 | **0‰** | 0‰ |
| **MACRO over 12 documents** | | | | **70‰** | **70‰** |

**Band: 0‰ .. 590‰, median 0‰, and TEN of the twelve now score exactly 0‰** — nine before this
slice, and the tenth is `nist-sp-800-218`, whose 2‰ is gone along with the 11 295 false positives
that were its denominator. §"The nine grids the engine already rejects" is why, and why the macro
is unchanged at 70‰.

Alongside it, on the same run:

| | 4 documents (0.10.0) | 12 documents (0.35.0) | 12 documents (0.36.0) |
| --- | --- | --- | --- |
| Tables detected / matched | 14 / 13 | 26 / 17 | **17 / 16** |
| Detection precision | 928‰ | 653‰ | **941‰** |
| Cells emitted | 180 | 1 236 | **208** |
| **Fabricated cells** | **0** | **0** | **0** |
| Cross-check disagreements | **0** | **9** | **0** |
| False tables on the gold negatives | 0 | 0 | **0** |

### 64‰ "held" at 70‰, and that is the least informative true thing to say about it

The macro moved 64‰ → 70‰ across a threefold corpus. Read alone, that is stability, and it would
license *"64‰ is this engine's honest table number."* **The band says otherwise, and the band is
the number that matters.**

- **Ten of twelve documents score 0‰.** Not "low" — zero, because the detector emits **no table
  at all** on them. The median document scores nothing.
- **The macro is carried by two documents out of twelve.** `irs-fw9` at 590‰ and
  `cfpb-home-loan-toolkit` at 259‰ supply all 849 of the points that get averaged.
- **Remove `irs-fw9` alone and the macro falls to 23‰** — a threefold move from one document out of
  twelve. That is the arithmetic of an average over mostly zeros: it is stable in the way a
  thermometer reading mostly zeros is stable, and its value is set by which one or two documents
  happen to have drawn rules.

So the four-document 64‰ was **not** a property of this engine, and the twelve-document 70‰ is not
one either. What twelve documents establish that four could not is the **shape**, and the shape is
not "weak everywhere". It is bimodal: **the ruled rule works where a producer drew the rules and
produces nothing where it did not** — confirming on twelve documents the finding that reframed five
slices of work, which four documents could have produced by luck.

### The one document where the failure was not silence

`nist-sp-800-218` was the corpus's new information at v2-S19, and it was worse news than a zero.
**v2-S20 acted on it and the section after this one is the argument**; what follows here is what
S19 measured, kept as written because the decision only makes sense beside it.

Against **4** tagged tables it detects **9**, with dimensions of 103 × 22, 108 × 16, 89 × 15,
87 × 14, 86 × 19, 75 × 14, 66 × 17, 58 × 15 and 6 × 14 — phantom grids spanning whole pages, built
from the document's ruling lines rather than from any table. Expanded to slots, they contribute
**11 295 false positives**, more than the entire rest of the corpus produces in either direction.

Two things about it are worth stating precisely, because they are easy to get backwards:

- **Fabrication is still 0.** Every cell's text is a concatenation of runs the page actually drew.
  The detector arranged real text into a grid that is not there; it did not invent text. That
  distinction is the S1 invariant and the metric keeps it visible.
- **The engine's own cross-check flagged them, and the report now says so per document.**
  Cross-check disagreements went from 0 to 9 on this corpus, and the per-document column shows
  **all nine are this document's** — every other of the twelve reads 0. There are exactly nine
  detected tables here, so the locator cross-check is rejecting **every one of them**. Nothing
  acted on that at S19; a rule that declined a table its own cross-check rejected is a **detector
  change**, which v2-S19 was forbidden to make, and it was recorded as the largest concrete lead
  this corpus produced. **v2-S20 is that change** — see below.

- **They are `ruled-rects`, not `stroke-ruled-v1`.** *"Built from the document's ruling lines"*
  above describes the **ink**, not the rule: this producer draws its rules as thin **filled**
  rectangles, so they arrive as `interp.rects` and fold into a lattice with everything else the
  page paints. Read off each detected table's `rule` field, all nine say `ruled-rects-v2`, and
  every stroke-ruled and alignment table in the corpus cross-checks `ok`. The distinction is not
  pedantic — it decides which rule a repair belongs in, and the sentence above was read the other
  way at least once.

FP of 11 295 against 1 236 cells emitted corpus-wide is not a contradiction and is the kind of thing
worth spelling out: `emitted_cells` counts **cells**, the score counts **slots**, and a cell with
`colspan: 3` occupies three of them. §"Slots, not cells" settles why the comparison is done that way.

## v2-S20: the nine grids the engine already rejects

The question this slice answers, in one line: **should a table whose own cross-check rejects it be
emitted?** The answer shipped is **no when the rejection is structural**, and the reasoning below is
longer than the change because the change is four lines and the reasoning is the slice.

### All three options were measured on all twelve documents, and the gate cannot tell them apart

| | keep and declare (0.35.0) | decline on the cross-check | tighten the ruled rule |
| --- | --- | --- | --- |
| **MACRO** | **70‰** | **70‰** | **70‰** |
| `nist-sp-800-218` | 12 TP / 11 295 FP / 404 FN → 2‰ | 0 / 0 / 416 → 0‰ | 0 / 0 / 416 → 0‰ |
| every other document | — | **unchanged** | **unchanged** |
| band | 0‰..590‰, median 0‰, 9/12 zero | 0‰..590‰, median 0‰, **10/12** zero | identical |
| macro without `irs-fw9` | 23‰ | 23‰ | 23‰ |
| cells emitted | 1 236 | 208 | 208 |
| **fabricated cells** | **0** | **0** | **0** |
| cross-check disagreements | 9 | 0 | 0 |
| gold negatives | 0 tables | 0 tables | 0 tables |

*Decline* was implemented as a gate on `CheckStatus::Mismatch` at the point every rule emits.
*Tighten* was implemented as a precondition in `Lattice`'s consumer: refuse a lattice in which two
rectangles claim one slot — the direct cause, since every one of the nine carries
`OwnedMoreThanOnce` faults by the hundred. **The two are byte-identical on all twelve documents and
on all three gold negatives**, so the corpus cannot choose between them and the choice is argued.

**The headline of this table is the first row.** Removing **11 295** false-positive cell slots —
more than the rest of the corpus produces in either direction — moves the published macro by
**exactly nothing**. That is not evidence the change did nothing. It is v2-S19's finding arriving
from the other direction: an average over mostly zeros is insensitive to a document that was
already at 2‰, so a macro of 70‰ survives both the presence and the absence of the worst
over-detection this engine has ever produced. **A number that cannot see this cannot be the number
that decides it.**

### So the decision is argued from the artifact, not from the gate

**Keeping and declaring was the posture until this slice, and what it is worth is measurable.**
The declaration already exists: every table carries its `LocatorCheck`, and on those nine it said
`Mismatch` with the faults enumerated. **No surface this repository ships reads it.**
`engine_core::markdown::plan_tables` and `engine_core::html::plan_tables` project **every** table
in `payload.tables`, branching only on `rows` and `columns`, and consult no check anywhere. So a
consumer of either projection received nine GFM grids of up to 103 × 22 and received no warning at
all. A disclosure that no reader of the thing being disclosed about can see is a disclosure in
name only — the shape v2-S12.1 and v2-S13.1 both exist for.

**Declining is not a new posture.** This engine already refuses candidates and declares the
refusal: `FaceWithoutRectangle`, `LatticeTooLarge`, `ColumnLineNotStroked`, `FaceIsAFormFieldBox`,
and every variant of `unruled::Refusal`. A cross-check failure is the same shape — a precondition
about the evidence that the evidence did not meet — and it goes out through the same channel, as
`ruled-table-candidate-refused`, naming the page and the fault counts. **Nothing is deleted. The
disagreement moves from a field beside a grid to a refusal instead of a grid.**

### The cost, named: twelve cell slots, and all twelve are the empty string

Declining loses `nist-sp-800-218`'s entire true-positive contribution — **12 cell slots**, which is
why its F1 goes 2‰ → 0‰. Every one of them is on **page 14**, inside the **103 × 22** grid the join
pairs with that page's tagged **72 × 4**, and **every one of them is `""`**:

```
p14 r6 c0   ""      p14 r25 c0  ""      p14 r47 c0  ""      p14 r62 c0  ""
p14 r15 c0  ""      p14 r26 c0  ""      p14 r51 c0  ""      p14 r65 c0  ""
p14 r20 c0  ""      p14 r46 c0  ""      p14 r61 c0  ""      p14 r71 c0  ""
```

A blank face of a phantom grid agreeing with a blank tagged cell. **Not one character of extracted
text is lost anywhere in the corpus** — `cfpb-home-loan-toolkit` keeps all 44 of its true positives
and `irs-fw9` all 26, and no other document had any.

Stated as a rate, the twelve are not a capability: that document emitted **1 028 cells** and
predicted **11 307 slots** to get **12** right — **one right slot per 941 wrong**. This document
already records a variant that was *"right about 5% of the cells it emits"* and reverted it. This
was fifty times worse and it shipped.

### The other cost, which is real and is not designed around

**A table that is emitted can no longer carry a structural `Mismatch`.** That is the honest
consequence of gating on a check: the check still runs, still computes both halves, and still puts
its result on every emitted table — but the structural half's disagreement now ends the detection
instead of accompanying it.

Two things keep that from being the retirement of the cross-check, and both are asserted rather
than asserted-about:

1. **Only the structural half gates.** The two halves are not the same kind of statement. The
   structural half (`SlotFault`) is arithmetic on the row/column indices the rule assigned and
   admits no tolerance — a slot owned twice is a contradiction in the rule's own bookkeeping. The
   geometric half (`GeometricFault`) compares **exact** boxes against a lattice built with
   `LATTICE_TOLERANCE`, so it fires on the very slop that tolerance exists to absorb. **Measured:**
   gating on both was built first and it refuses `near_edges_fold_into_one_lattice_line` — a 2 × 2
   whose only defect is one edge sitting a single centipoint out, which is exactly what a 1 pt
   stroked rule looks like. That fixture now emits, carrying a geometric-only `Mismatch`, and the
   test asserts precisely that — so `CheckStatus::Mismatch` remains a state an emitted table can
   be in.
2. **It is the only rule that can fail this check at all.** `stroke-ruled-v1` and
   `unruled-align-v1` build a cell for **every** face of their lattice, from the same lines the
   table's own box comes from, so their cells tile exactly, never overlap and never reach outside.
   `tables::tests::the_other_two_rules_build_a_cell_for_every_face` runs both and asserts it, which
   is why the gate is in `detect_ruled` alone rather than repeated three times as dead code.

**And one shipped fixture changes its job.** `ruled-table-overlap` existed to prove the engine
emits a self-contradicting grid and says so; it now proves the engine refuses one and says why. Its
row in `fixtures/README.md` is rewritten rather than left standing, two tests in
`crates/engine-pdf/tests/extraction.rs` moved with it, and
`tables::tests::the_cross_check_still_sees_two_rectangles_claiming_one_slot` holds the check itself
under test now that no artifact can. That is a real loss of an end-to-end instance and it is
recorded here rather than absorbed.

### Why not the third option

*Tighten the ruled rule* measures identically and was rejected as the narrower spelling of the same
decision. Its precondition — "no two rectangles may claim one slot" — **is** the structural half of
a check the engine already computes two hundred lines later, so shipping it would put two
derivations of one rule in one file, which is the drift a single pinned rule id exists to prevent.
`Lattice::build`'s own comment records the earlier decision it would reverse: *"Overlaps are
deliberately NOT excluded here … it belongs in the cross-check where it is reported rather than in
a precondition where it would be silently dropped."* That reasoning still holds. What v2-S20 changes
is not where the overlap is **found** but what happens after it is **reported**.

### A rule id moves, because rule behaviour moved

`table_detection.ruled` goes **`ruled-rects-v2` → `ruled-rects-v3`**, so `profile_sha256` moves and
an artifact from either side is correctly non-comparable — two builds disagree about whether
`nist-sp-800-218` has nine tables. `stroke-ruled-v1` and `unruled-align-v1` are **byte-identical**
and keep their ids: neither rule's code changed and neither could reach the new precondition.
The prompt for this slice expected `stroke-ruled-v1 → stroke-ruled-v2`; that would have been the
wrong id, for the reason §"The one document where the failure was not silence" now states.

### The gap this metric has, named and not built

**Fabrication is 0 while the engine emits a 103 × 22 grid that does not exist**, and that is
correct by the counter's own definition: every cell's text is a concatenation of runs the page
actually drew, so nothing was invented. **The counter protects against invented text. It says
nothing about invented structure.** Nine phantom grids are the second thing and `fabricated_cells`
cannot see them — it read 0 through all of them, and would have read 0 if there had been ninety.

That is a gap, and standing rule 5 says a gap is never presented as a success. It is named here
and **not** repaired: a structural-fabrication measure is a second metric, this slice is forbidden
to build one, and a metric introduced in the same commit as the detector change it would score is
the edit this document exists to make impossible. What can be said without a new metric is said in
the table above — **detected tables 26 → 17 and detection precision 653‰ → 941‰** — and the gold
negatives remain the only instrument in the harness that can see an invented grid at all, which is
itself worth knowing: they are three documents, all engine-owned or synthetic, and none of them
resembles `nist-sp-800-218`.

### What the corpus report says now, and what it stopped being able to say

`cross_check_disagreements` reads **0** on all twelve documents. That is not the check finding
nothing; it is the check being enforced, and the column is now structurally 0 for emitted tables.
The number that moved instead is `detected` — `nist-sp-800-218` goes **9 → 0** — and the nine
refusals are on the artifact under `ruled-table-candidate-refused`, each naming its page and its
fault counts. A reader of the printed report alone can no longer tell that document apart from the
nine that detect nothing; a reader of the **artifact** can, and that is the trade this section
argues is the right way round.

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

## v2-S22: why ten documents produce nothing

**This slice measures and changes no detector.** The canonical default profile differs from
0.36.1's only in `parser_version`, and `the_default_profile_is_pinned` proves it. What it adds is a
per-gold-table report — `crate::extract::per_page_table_diagnostics`, driven by
`accuracy::tests::the_ten_documents_where_nothing_is_detected` — that walks every one of the corpus's
**172 tagged tables** and records, for the page each sits on: what ink the page carries, which rule
built a candidate, and which precondition rejected it. The report is deliberate-run and never in CI;
it cross-checks its own emitted tables against `extract`'s before reading a single refusal, so a
drift in the mirror is a test failure rather than a wrong number.

### The one-line finding, on twelve documents rather than four

**Every detected table is a ruled or a stroke-ruled one. The alignment rule emitted none.**

```
across the twelve documents: 172 gold tables; emitted 8 ruled, 9 stroke-ruled, 0 alignment
```

All **17** detections are on the two documents that draw their grids: `cfpb-home-loan-toolkit`
(8 ruled, 6 stroke-ruled) and `irs-fw9` (3 stroke-ruled). The other ten detect nothing — not a
wrong table, no table. §"The finding that reframes all five" established this on the four-document
corpus by reading each table's `rule` field; twelve documents make it a much stronger claim, and it
holds: `unruled-align-v1` has produced no table on any real document this repository has measured.
`the_corpus_is_measured` now asserts that count stays zero, so the claim cannot lapse silently.

### And now we know *which* precondition refuses it, on every gold page

The report's most uniform column is the alignment rule's. On **every gold page in all twelve
documents**, `unruled-align-v1` refuses at the same precondition:

```
unruled = GutterBelowFloor { columns: true, gap: <151..1132>, floor: 1200 }
```

Two adjacent text columns sit closer than the 1 200-centipoint (12 pt) column-gutter floor — word
spacing and running prose, never a table's column gap. This is a sharper statement than the S7b
reading in §"Why the number is what it is" #2, and it supersedes it as the *first*-failing gate: the
gutter floor is step 2 of the rule and the whole-page-lattice ceiling is step 6, so on no gold page
does the rule ever reach the 22 176-face lattice that analysis described — it is turned away three
steps earlier, at the gutter. The floor is doing exactly its job: the gold negatives
(`synthetic/two-columns`, a flawless 2 × 2 of prose) prove that lowering it buys a fabrication, and
the report shows the real documents fall on the same side of it as that negative does.

### What the ten that detect nothing actually draw

The ruled and stroke-ruled rules read drawn ink, and the ten produce none they can use:

| Document | gold | what its gold pages draw | why nothing is emitted |
| --- | --- | --- | --- |
| `nist-sp-800-63b` | 13 | many filled rects (20–490/pg), **0 horizontal rules**, one margin upright | `ruled = FaceWithoutRectangle` — the rects cover no coherent lattice; stroke builds no band with no rows |
| `nist-sp-800-53r5` | 26 | 4–1 004 rects/pg, **0 h-rules**, one upright | same: `FaceWithoutRectangle` on every page that paints ≥ 2 faces' worth |
| `nist-sp-800-161r1` | 46 | 1–966 rects/pg, **0 h-rules** | same; two pages (310, 319) paint a single rect and build no candidate at all |
| `nist-sp-800-171r3` | 24 | 118–705 rects/pg, **0 h-rules, 0 uprights** | same |
| `nist-sp-800-207` | 4 | 24–199 rects/pg, no rules | same |
| `nist-sp-800-218` | 4 | 14–217 rects/pg | one page is the v2-S20 case — `ruled = CrossCheckRejected { structural: 1028, geometric: 127 }`, the 72 × 4 phantom refused structurally; the other three `FaceWithoutRectangle` |
| `nist-sp-800-37r2` | 20 | 1–747 rects/pg, one upright | `FaceWithoutRectangle` |
| `nist-sp-800-53Ar5` | 11 | 1–70 rects/pg | `FaceWithoutRectangle`, or a lone rect and no candidate |
| `irs-f1040sd-2025` | 2 | 8 rects, **83 h-rules, 60 uprights**, 45 field boxes | `stroke = ColumnLineNotStroked { interior: 5, stroked: 4 }` — one interior column line is not drawn |
| `irs-form-1040-2025` | 1 | 200 rects, 157 h-rules, 128 field boxes | `ruled = LatticeTooLarge { faces: 6642 }`, `stroke = FaceIsAFormFieldBox` — the 662-cell fabrication surface of v1-S1, correctly refused |

Two shapes, and neither is a tuning target. The **NIST family** draws its tables with cell-shading
and decoration rectangles and at most a single margin rule — never a covered cell grid and never a
stroked lattice — so the ruled rule sees rects that explain no grid (`FaceWithoutRectangle`) and the
stroke rule sees no rows at all. The **IRS forms** draw a real but *partial* grid: a column line
left undrawn, or a table that is a block of form-field widgets. In every case the two working rules
refuse correctly; the tables are real, but their geometry is in the tags and the text, not in ink
that forms a grid.

### The recommendation: v1's remaining gap is not the alignment rule

Stated as a recommendation, not a fix — this slice ships neither.

**The alignment rule is not a gap to close in v1.** It has emitted zero tables on 172 real gold
tables and refuses at the gutter floor on every one, because real-document columns are gutter-close;
reaching them means lowering a floor whose only job is to refuse prose, and the gold negatives prove
that floor is load-bearing. So the five geometric-repair slices measured against it (§"What would
actually move it") were tuning a rule the gate never exercised, and no seventh repair to it will
move the number. Whether `unruled-align-v1` — a rule id, a profile field, and its machinery, none of
which has ever produced a table on a real document — should be retired or reworked is a
**version-boundary question**, not a v1 calibration, and it is named here for the owner rather than
decided.

**v1's remaining gap is that its two working rules require the producer to have drawn the grid.**
The ~15 500 missed slots are almost all NIST, and the report shows why: those producers draw tables
without a grid this engine can read. Recovering them is not a tolerance move on any existing rule —
it needs a derivation that reads a table's geometry from something *other* than drawn grid ink: the
tagged structure tree's own cell bounds, or a column inference the gutter floor exists to forbid.
That is a new derivation class with its own slice and its own gold negatives, and it is the honest
boundary of what v1's three geometric rules can do. **Micro recall states the size of the gap in one
number: 4‰** — 70 of the corpus's 15 755 gold slots — which is the headline the macro's 70‰ was
hiding.

## v2-S24: the tagged tables the documents declare

**This slice took the derivation v2-S22 named.** That section ended by saying the ~15 500 missed
slots need *"a derivation that reads a table's geometry from something other than drawn grid ink:
the tagged structure tree's own cell bounds"* — a new derivation class. `tagged-tables-v1` is that
class, and it reads the tree's shape and cell text rather than any coordinate. **The geometric gate
does not move**, and that is the proof the detector did not: MACRO cell-F1 is still **70‰**, the band
is still 0‰..590‰ with ten of twelve at 0‰, and geometric micro recall is still **4‰** (70 / 15 755).

### The gate is kept geometric on purpose

The gate scores the geometric **detectors** against the document's **tags**, which are independent by
construction — matching by box against the tree would be scoring the detector against itself. A
tagged table comes *from* the tree, so scoring it against the tree would be the same circularity in
reverse: recall of ~1.0 that means nothing. So the tagged tables are emitted into their own list,
scored apart, and the gate — the macro that decides whether the detector cleared 489‰ — is byte-for-
byte the number it was at 0.36.3. Decision #18 is fed, not settled.

### What the tagged emit recovers: combined micro recall, 4‰ → 502‰

Pool every gold slot in the corpus and count what the engine now recovers with its **full** table
output — the geometric detections plus the tables it reads from the tags:

| Document | geo-recall | combined recall | tagged tables | gold tables |
| --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit.pdf` | 276‰ | **452‰** | 5 | 17 |
| `irs-fw9.pdf` | 433‰ | **450‰** | 1 | 4 |
| `irs-form-1040-2025.pdf` | 0‰ | **700‰** | 1 | 1 |
| `irs-f1040sd-2025.pdf` | 0‰ | **1000‰** | 2 | 2 |
| `nist-sp-800-63b.pdf` | 0‰ | **654‰** | 13 | 13 |
| `nist-sp-800-53r5.pdf` | 0‰ | **495‰** | 26 | 26 |
| `nist-sp-800-161r1.pdf` | 0‰ | **437‰** | 46 | 46 |
| `nist-sp-800-171r3.pdf` | 0‰ | **695‰** | 24 | 24 |
| `nist-sp-800-207.pdf` | 0‰ | **745‰** | 4 | 4 |
| `nist-sp-800-218.pdf` | 0‰ | **372‰** | 4 | 4 |
| `nist-sp-800-37r2.pdf` | 0‰ | **458‰** | 20 | 20 |
| `nist-sp-800-53Ar5.pdf` | 0‰ | **313‰** | 11 | 11 |
| **MICRO over every gold slot** | **4‰** (70 / 15 755) | **502‰** (7 924 / 15 755) | **157** | 172 |

The ten documents that read exactly 0‰ geometric now recover most of their gold — the NIST family
between 313‰ and 745‰, and `irs-f1040sd-2025` all of it. **157 of the 172 gold tables** are emitted
as tagged; the other 15 paired with a geometric detection on `cfpb-home-loan-toolkit` and `irs-fw9`
and so ride those documents' geometric numbers instead.

### Why it stops at 502‰ and not 1000‰

Gold and the tagged emit share the tree derivation, so the **grid** matches by construction — same
`/TR`/`/TD` ordinals, same spans. What differs is the **text**. The gold joins a cell's `/MCID`
texts with a space; the tagged emit concatenates the runs it binds with nothing, exactly the 27‰
separator gap §"A known defect in this metric" already records — now applied across all the newly
recovered cells rather than the handful the geometric detector reached. It is a real difference and
it is not tuned away: the tagged cell text is the runs the page drew, and closing the gap would mean
adopting the gold's join convention, which would make the number measure the harness rather than the
extractor. **Fabrication stays 0**: 15 593 tagged cells emitted, every one of them real runs
concatenated, none placed.

### What a tagged table carries, and what it does not

- **`derivation: Extracted`**, where a geometric table is `Computed`. The document *stated* the grid;
  the engine did not infer it. This is the field — not two lists — that tells a consumer which is
  which, and it inverts the usual intuition: the tagged table is the *stronger* claim.
- **Geometry typed-absent**: `GeometryPresence::Absent(NotReportedByStructureTree)`. No box is
  invented. `TableRecord`/`TableCellRecord` carry a `GeometryPresence` rather than a bare `QRect`
  so the absence is expressible; the representation schema moves to `0.6.0` for it.
- **A not-applicable cross-check.** `geometric-vs-structural-v1` compares two derivations of one
  table; a tagged table supplies only the structural one, so the check reports `NotApplicable`,
  never `ok`.
- **Omitted from grounding.** `ethos.grounding.v1` requires a `bbox` on every table and cell, and a
  tagged table has none — so it is left out and disclosed through
  `tagged-table-without-geometric-table`, whose text is repaired to name the emitted absent-geometry
  tables rather than the withheld ones it used to.

### What this leaves for #18

v1's remaining gap was *"the two working rules require the producer to have drawn the grid."* That is
now closed on evidence the documents supply: where the producer drew nothing but tagged everything,
the engine recovers the tables the document declares. Whether v1's table number is stated as the
geometric gate's 70‰/4‰ or as the capability plus 502‰ combined recall is decision #18, and it is
the owner's — this slice reports both and settles neither.

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

## The clustering lead, measured and refused (0.38.1)

The estate audit's highest-confidence ruled-rule recommendation was component clustering:
group a page's rectangles into connected components before building any lattice, judge each
component alone, and the page-global lattice's brutal arithmetic — one stray painted box
refuses a clean grid beside it — goes away. The audit predicted it "should convert a large
share of those 556 refusals into detections without weakening a single precondition." It was
implemented in full — union-find over edge-contact within `LATTICE_TOLERANCE`, components in
reading order, `-v3`'s lattice, coherence, border rule and cross-check unchanged per
component — measured on the twelve-document gate, and **refused**. The numbers:

| measure | `ruled-rects-v3` | clustering |
| --- | --- | --- |
| macro cell-F1 | **70‰** | **63‰** |
| `irs-fw9` (the corpus's best document) | 590‰ | 490‰ |
| `cfpb-home-loan-toolkit` | ~0‰ | 269‰ |
| geometric tables emitted | 18 | 112 |
| false-positive cell slots added | — | ~2 000 across six documents |
| fabrication | 0 | 0 |

Two mechanisms, both invisible until measured:

- **The page-global lattice was load-bearing on the best document.** `irs-fw9`'s form rows
  are boxes separated by more than one tolerance, so clustering splits the W-9's grid into
  five components — every one of them N×1 or 1×N — where the single page-wide lattice had
  unified them into the 2-D grid the truth declares. The defect the audit diagnosed was, on
  this document, the mechanism doing the work.
- **A component is furniture-sized.** `nist-sp-800-53Ar5` emits **70** geometric tables
  against 11 tagged ones — thirty of them 3×3, fourteen 4×3: control-parameter boxes whose
  rectangles genuinely tile a small grid. On `nist-sp-800-161r1`, ten of eleven admitted
  grids are single-column stacks of boxed disclaimer prose, and every one carries a
  `does_not_tile` geometric fault the structural gate cannot see. Coherence was written for
  a page's worth of evidence; against a three-box component it is nearly always satisfied.

Fabrication stayed 0 throughout — the cells carry real text — which is exactly #18's
"worse than a zero" shape: real text arranged into a grid that is not there. A single-
dimension refusal (rows ≥ 2 and columns ≥ 2) was probed against the artifacts before being
written: it clears `nist-sp-800-161r1`'s ten stacks and then deletes `irs-fw9`'s remaining
true positives with the same stroke, because the split components it would refuse are the
real table's fragments. The repair and the regression are the same predicate.

So the lead joins the six measured repairs above rather than the two that shipped, and the
finding it adds to the record is one the next attempt has to answer: **any per-region ruled
rule needs a region-merging step strong enough to reunify a form's rows before it can
afford to judge regions alone** — and that is the v1-S7 chase, which `00-NORTH-STAR.md`
#10 parks and #18 holds for the owner. The code is reverted; this section, the audit row
it answers, and the artifacts sampled for the shape census are the slice's whole output.
