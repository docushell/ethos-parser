# Tables from whitespace tracks — scope (decision #38, slice D)

**Status: scoped 2026-10-04, before code.** Decision #38 chose four inferences so the engine can
compete with LiteParse on ParseBench; this is the fourth, and the one that decides the result.
Fabrication stays 0: a cell holds only text the page drew, and no table is emitted where the
ground truth holds none.

## 1. Why

On ParseBench, run locally with one scorer, tables are **8.1 of the 12.2 points** between this engine
and LiteParse: `grits_trm_composite` **0.0212** against **0.4241**, because the engine emits a table on
34 of 503 table pages. On opendataloader-bench TEDS is **0.1728**, 28 of 42 documents at zero.

The four rules in force read evidence most producers do not leave. `ruled-rects-v6` and
`stroke-ruled-v1` need a drawn grid; `tagged-tables-v1` needs a structure tree; `unruled-align-v1`
needs every cell's *origin* on a shared column line, and refuses at its 12pt gutter floor on every
real table it has met (`table-gate-v1.md`, "The recommendation, named not taken";
`measurements/opendataloader-bench/tables-why-zero.md`, 2026-10-03): it reads left origins, so a
right-aligned or centred column of numbers opens a column line per width. The 2026-10-03 prototype
of phrases before that fold moved ParseBench 0.0212 → 0.0245 and was not kept.

## 2. The rule, `whitespace-tracks-v1`

Its own profile field, `table_detection.tracks`, beside the four: the evidence differs — *where the
author left whitespace across rows* — and an artifact says which evidence built its table. It runs
last, on runs no other rule claimed, and **not on a page whose structure tree declares a `/Table`**:
there the author's table wins, as a declared heading does, and an inferred one would take its place
in the position pairing `tagged-tables-v1` relies on.

1. **Lines.** Runs whose baselines lie within `blocks::LINE_TOLERANCE` (1.5pt) are one line, page
   wide. Not per `region`: the reading-order cut can divide a table's columns into regions, and a
   row is a row across them. **Only upright runs**: a run's measured box must start at its origin
   and run right of it, with the baseline crossing the box. A document number set up the margin,
   one glyph per baseline, has its box across the page and its baseline at the box's edge, and is
   no column. The box and not the pen advance gives a run's extent: some documents state a zero
   advance for every glyph and draw them at the font program's widths.
2. **Cells of a line.** Its runs in x order, joined while the gap from one run's pen end to the next
   run's origin is at most one rendered em of the line; a wider gap starts a cell. Whitespace runs
   join and never start a cell.
3. **A table starts** at a line of at least **3** cells. Its **tracks** are that line's cells' x
   extents.
4. **It grows down** line by line while the next baseline is at most 3.5 line pitches — 1.2 ems
   each — below the first line of the row above, and the next line is one of:
   - **a row**: as many cells as tracks, at most one of them off its track;
   - **a sparse row**: fewer cells but at least two, each on a distinct track, at least 1.5
     pitches below — the missing cells are empty;
   - **a wrap**: fewer cells, each on a distinct track, closer than that — its runs join the cells
     of the row above.

   A cell is on a track when its centre lies within the track (±6pt), or its left or right edge
   meets the track's (±6pt) — so left-, right- and centre-aligned columns are all columns. **And no
   cell may reach a neighbouring track**: a line of prose under a table is one wide cell whose
   centre lands on some track, and it ends the table instead of joining a cell as a wrap.
5. **At least 3 rows**, and the row pitch varies by at most half its mean.
6. **The content stream wrote it row by row**: every run of row *r* is emitted before any run of
   row *r* + 1. Columns may interleave within a row, because a wrapped cell's second line comes
   after the next cell's first. This is `unruled-align-v1`'s rule 5 — author evidence rather than
   a threshold — taken at row grain. Detection runs before reading order, as that rule's does, so
   it reads the order the content stream produced.
7. **Fabrication 0 by construction.** A cell's text is its runs' text concatenated, in content
   order; no run is in two cells; a run in no track ends the table rather than being dropped.

What it does not do: merged cells, header rows beyond the first row's projection as GFM's header
(as every rule's), nested tables, a table split across pages, and a table with fewer than three
columns or rows.

## 3. The prototype, measured (2026-10-04)

A Python prototype of exactly §2 over `extract` output — a scratch build kept the content-stream
order for rule 6 and was discarded — scored with the benchmarks' own evaluators:

| variant | ParseBench pages with a table | ParseBench tables | ODL documents with a table, ground truth has one | **ODL documents with a table, ground truth has none** |
| --- | ---: | ---: | ---: | ---: |
| rules 1–5, two rows, no rule 6 | 361 | 0.2523 | 31 | **19** |
| rule 6 on the reading order, two rows | 235 | 0.1588 | 26 | **2** |
| rule 6 on the content order, two rows | 306 | 0.2124 | 26 | **2** |
| **§2: rule 6 on the content order, three rows** | **263** | **0.1906** | **24** | **0** |

Without rule 6 the false tables are two-column prose (a footnote number makes the third column),
equations with their numbers, and justified lines whose spaces open into gaps. Rule 6 removes all
but two; the third row removes those two, a chart's axis labels and one justified line pair.

## 4. Bounds, set before the code

1. **opendataloader-bench**: no table on any of the 158 documents whose ground truth holds none.
2. **The gate corpus** (`table-gate-v1.md`): fabricated cells stay 0, and every table this rule
   emits there is read and named; one whose cells are the lines of a paragraph is a breach.
3. **ParseBench**: tables measured with `grits_trm_composite`; content faithfulness and visual
   grounding may not fall, since a false table would take text out of the prose.
4. Every cell's text equals the concatenation of its runs — the invariant every rule already holds.

A breach of 1 or 2 is not shipped and is the owner's call.

## 5. The rule, measured (2026-10-04)

The Rust rule first missed tables the prototype found: on `01030000000078` the line
`Source: World Bank and KNOMAD (2021)` under a seven-column table was one cell centred on a track,
joined the last row as a wrap, and stretched its column until the bands inverted and the table was
dropped — the prototype had kept the table with the source line inside a cell. Rule 4's
neighbour clause is the repair. Reading the gate corpus then found two more things, both repaired
before these numbers: 46 inferred tables on tagged pages had paired with the author's `/Table`
tags and displaced them (hence the declared-page clause), and all 47 on `nist-sp-800-53Ar5` owed
their third column to the vertical margin text (hence upright runs). A first form of that test
compared the box with the pen advance, and cost ParseBench 0.0049 of tables on SERFF filings whose
every glyph states a zero advance; the box alone decides now.

**opendataloader-bench**, `score.py`: TEDS **0.1728 → 0.3918**, non-zero on 14 → 26 of 42, band
0.0000..1.0000, median 0.0000 → 0.3579, worst `01030000000088`; **15 documents rose and none fell**.
NID 0.8793 → 0.8849, because a table is now one atom in reading order. MHS 0.4703 → 0.4712, one
document down (`01030000000150`, 0.3131 → 0.3036). **Bound 1: met** — 20 tables on 15 documents,
every one holding a table in ground truth, none on the 158 that hold none.

**The gate corpus**, the eight gate documents and the ten gate-zero ones: 65 tables on five
documents, every one read — control-baseline matrices on `nist-sp-800-161r1` and
`nist-sp-800-53r5`, parameter tables on `nist-sp-800-171r3`, an errata log, author grids — and on
pages whose tree declares no `/Table`. **Bound 2: met**: none is prose, and fabricated cells are 0.
What reading them shows is imperfect, not invented: where a row has fewer cells than the one above
and sits close under it, it is taken as that row's wrap, so two short rows can share one row.

**ParseBench**, the benchmark's own scorer: tables `grits_trm_composite` **0.0212 → 0.1936**, overall
25.11 → 28.70; content faithfulness 0.6619 → 0.6627 and visual grounding 0.2224 → 0.2267. **Bound 3:
met** — neither fell, so no inferred table took text out of the prose. 184 table pages still score
zero where LiteParse scores: most are two-column tables (a glossary, a key and its value) under
the three-column floor, two-row tables under the three-row floor, and drawn forms whose header
words sit on lines of their own. Both floors are fabrication guards, and lowering either is a
measurement of its own.

**Bound 4: met**: every cell's text equals its runs' concatenation, 0 of 962 on
opendataloader-bench, and the unit tests hold it with no run in two cells.

## 6. `whitespace-tracks-v2`: two columns, two rows, and only undeclared documents (2026-10-04)

`-v1` is the rule of §2–§5. Of its 184 zero-scoring ParseBench pages, most were two-column tables —
a glossary, a key and its value — under the three-column floor, or a header and one row under the
three-row floor. `-v2` lowers both floors, each with the guard its false tables called for, measured
on a scratch build reading the floors from the environment, and ships under a new id:

| floors | opendataloader-bench documents with a table, ground truth has none | ParseBench tables |
| --- | ---: | ---: |
| three columns, three rows (`-v1`) | 0 | 0.1936 |
| three columns, two rows | 1 — a chart's axis labels over its legend | — |
| … two rows only with no empty cell | 0 | 0.2174 |
| two columns, three rows | 12 | — |
| two columns, two rows, two-row guard | 20 | 0.2746 |
| **… two columns need three rows, no first column of list labels, no second column of rising page numbers** | **0** | **0.2630** |

The two-column false tables were lists (a bullet, a number or a note number beside its text),
tables of contents (an entry beside page numbers that never fall) and pairs of prose fragments; the
guards are those three, and each is a unit test. A list's labels stand on the left, so a right-hand
column of small numbers stays a column of values.

**Two changes after the full run, both measured again.** The shipped build's first ParseBench run
lifted tables to 0.2631 but cost content faithfulness 0.0002 and visual grounding 0.0016 — bound 3 —
through two-row false tables: a line of authors over its affiliation numbers, and the top of a
page's three columns of prose (headings over the first lines of their paragraphs). LiteParse's own
two-row test answers both: a pair stands only apart from the lines around it, under a first row
whose cells open with a letter or digit that is not lower-case and end with no comma — adapted from
its "upper-case" so a script without case passes. And the engine's gold negative
`unruled-near-miss`, three rows of two columns with one value five points off its column, became a
table: its note refuses it because the alignment rule would have to move a coordinate, which this
rule does not do, but a gold negative is a safety rail and retiring one is the owner's call. So two
columns need **four** rows, and the rail holds.

**And `-v2` runs only where the document declares no author structure.** Read on the gate corpus,
the two-column variant re-read lists the documents declare — reference lists, glossaries, 438
assessment-objective lists on `nist-sp-800-53Ar5` — as tables, which is an inferred structure
overriding a declared one. A tagged document says what is a table; the gate is the one headings and
layout units use. Every ParseBench table page and every opendataloader-bench document declares no
structure, so the gate costs neither benchmark a table, and on the gate corpus this rule now emits
none (bound 2 by construction).

Measured with the shipped build: ParseBench tables **0.1936 → 0.2375**, overall 32.23 → 33.11
(with layout units); content faithfulness 0.6627 unchanged, visual grounding 0.4028 → 0.4030 and no
page of it fell, formatting unchanged — **bound 3 met**. opendataloader-bench reads exactly as
under `-v1` (TEDS 0.3918, NID 0.8849, no table on its 158 documents without one): the two-column
tables it gained under three rows were the ones the four-row floor now refuses. The gate corpus
gets no table from this rule. The two-column, three-row variant (ParseBench 0.2631) waits on the
owner's word on `unruled-near-miss`. **Decided 2026-10-04** (decision #39): the rail stays the
alignment rule's and stops being this rule's, on a re-measurement under `-v3`.


## 7. `whitespace-tracks-v3`: the amount's sign, the row's footnote mark, and row order only where prose could be (2026-10-04)

Of the 158 ParseBench table pages where `-v2` scored zero and LiteParse did not, 23 held a ruled
grid with no cell in it — a defect of the ruled rule, fixed on its own as `ruled-rects-v7`. On the
rest a scratch build logged, for each candidate, the clause that ended it. **49 had a candidate of
three rows or more refused by the row-order clause alone**; 12 a two-column candidate under four
rows; and most of the others stopped within two rows, for three reasons that recur through financial
statements:

- **a `$` set flush left in its column** and the amount flush right are further apart than an em,
  so the line had a cell more than the table had tracks;
- **a footnote mark raised on a row** — `amortization¹` — stood on a baseline of its own between two
  rows, a line of one short cell that ended the table or joined the row above;
- **a header centred over a column of `$` and amount** held no track the next row's bare amount sat
  on, by centre or by either edge, because a track kept its first line's extent.

`-v3` joins a lone currency sign to the cell after it, joins a superscript to the line it is raised
on — set smaller than that line, its baseline above it by less than half that line's em — and
grows each track to hold every full row's cell.

**The row-order clause** is what tells two columns of prose from a table, and what it refuses besides
was measured by waiving it on the scratch build, before the three fixes:

| row-order clause | opendataloader-bench documents with a table, ground truth has none | ParseBench tables |
| --- | ---: | ---: |
| always asked (`-v2`) | 0 | 0.2375 |
| waived | 18 — two or three columns of prose, and a chart's labels | 0.2743 |
| waived where every column's median cell is ≤ 20 characters | 1 — the chart's labels | 0.2559 |
| … ≤ 30 characters | 1 | 0.2605 |
| … ≤ 40 characters | 4 | 0.2669 |
| waived where no column opens a third or more of its cells lower-case | 1 | 0.2700 |
| … a fifth or more | 1 | 0.2687 |
| … and the table has four rows or more | 0 | 0.2647 |

The tables it found were rate tables exported cell by cell, written down their columns or in no
order a reader would recognise, and tables whose multi-line cells interleave rows in the stream.
Length is the weaker signal: prose set in narrow columns runs 31–40 characters a line. **A paragraph's
continuation lines open lower-case**, and every prose column the waived clause took opened two in
five of its lines lower-case or more. The chart's labels — `Company`, `A` and a raised `2` stacked
under four bars — stopped being a table once a superscript joins its line: the pair left is two
rows, and two rows keep row order. A brochure's panel of bullets beside its panel of headings, two
columns written down each panel, is why the waiver asks for **three columns**: two columns written
down the page are the very shape of two columns of prose.

**Then the first full run of that build lost elsewhere.** Tables rose 0.2375 → 0.3322 with
`ruled-rects-v7`, but content faithfulness fell 0.6633 → 0.6614, semantic formatting 0.3623 →
0.3587 and visual grounding 0.4034 → 0.3911, every fallen page a page set in columns: a radio
schedule, a directory of contact cards, a board's names set in four columns, a newspaper in
traditional Chinese. Each column of such a page is its own text written top to bottom, which is
exactly what a rate table exported down its columns looks like — and a script without letter case,
or a column of names, gives no lower-case line away. On ParseBench's 966 text and layout pages the build emitted
189 tables from this rule where `-v2` emitted 92. **What a table of values has and those pages do
not is a column of numbers**, so the waiver asks for one: a column at least half of whose cells hold
a digit and nothing but digits, spaces, a currency sign and the marks a number is written with.

| on top of `ruled-rects-v7` and the three fixes | tables from this rule on text and layout pages (`-v2`: 92) | ParseBench tables | content | formatting | visual grounding | opendataloader-bench TEDS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| no waiver | — | 0.3025 | 0.6633 | — | 0.4053 | 0.4254 |
| waiver, no column of running text | 189 | 0.3322 | 0.6614 | 0.3587 | 0.3911 | — |
| … and a column of numbers | 121 | 0.3304 | 0.6633 | 0.3623 | 0.4031 | 0.4471 |
| **… and no two groups of columns each written row by row** | **109** | **0.3224** | **0.6633** | **0.3623** | **0.4042** | **0.4254** |

`-v2`'s own figures are 0.2375, 0.6633, 0.3623 and 0.4034 on ParseBench, and 0.4254 TEDS with
`ruled-rects-v7`. **The numbers guard leaves content and formatting where `-v2` had them, but visual
grounding still falls 0.0003** — fifteen pages down and six up, the fallen ones annual reports, an
insurance illustration and a datasheet. That is bound 3 breached. The
last guard refuses a grid whose columns divide into two groups each written row by row — a table
beside a column of text, two flows side by side, whatever the text's letters say — and visual
grounding returns above `-v2`'s, ten pages down and four up, with no content or formatting page
moving: **the shipped rule**. What it costs is stated too. A table that writes its label column as one block
before its values row by row has the same order, and is refused with the flows: on
opendataloader-bench that is the whole of the waiver's gain there (TEDS 0.4471 back to 0.4254, one
document from 0.16 to 0.86 among them). The variant without the last guard is measured above, and
taking its bound-3 breach for those tables is the owner's call. **Declined 2026-10-04** (decision
#39); it reopens on a test that tells a label column written as one block from a column of text.

Five of the documents that gained a table on text and layout pages under the numbers guard were read
by hand. Under the shipped rule two lose theirs — a table of contents set in two pairs of columns,
whose section numbers count as a column of numbers, and two tables set side by side read as one.
Three keep theirs: real tables, though another such pair is still read as one; a chart's axis read
with the table beside it; and a list of features beside a footnote, which the three fixes find
without the waiver. They are named here; no measure moved for them.

With their trees stripped, the gate documents gain two tables from `-v3`, both control matrices
`nist-sp-800-161r1` declares, and nothing false; the table gate does not move. opendataloader-bench
reads exactly as under `ruled-rects-v7` alone, with no table on a document without one.

Each of the three fixes was measured by taking it out of the three-column variant before
`ruled-rects-v7`, at 0.2898: without the currency join ParseBench tables fall 0.021, without growing
tracks 0.006, without superscripts 0.002. Growing a track on sparse rows and wraps too added 0.0002
and is not done.

## 8. Two columns at three rows, measured under `-v3` (2026-10-04) — not shipped

Decision #39 let two columns stand at three rows if a re-measurement under `-v3` kept fabrication at
0 and bound 3. It did not, and the four-row floor stays.

- **The floor at three rows alone** put a table on one opendataloader-bench document whose ground
  truth holds none: `01030000000014`, a line of one column's prose over two footnotes, `51` and
  `52`, read across the page's columns.
- **With the two-row test's header clause** — a first row whose cells open with a letter or digit
  that is no lower-case letter — fabrication is 0 again, TEDS 0.4254 → 0.4359, and
  `unruled-near-miss` reads as the 3 × 2 table it shows, the alignment rule still refusing it by
  name. ParseBench tables 0.3224 → 0.3263; content and formatting unchanged; **visual grounding
  0.4046 → 0.4030**, two pages of one performance report falling 0.41 and 0.36: its key-value
  fact boxes (`Investment Type: | Private Equity`, `Vintage: | 2015`) read as tables, and each
  box's third row took the next box's label.

That is bound 3, and 0.0039 of tables does not buy it back.

