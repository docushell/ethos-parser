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


## 9. `whitespace-tracks-v4`: tables inside a page's columns (2026-10-04)

Of the 107 ParseBench table pages where `-v3` scored zero and LiteParse did not, 53 have a
vertical gutter down the page that two or fewer line fragments cross, and 25 are divided by the
reading-order rule's own column cut. On those the page's lines interleave: a table in one column
shares no baseline with the other column's prose, so its rows alternate with prose lines, and the
candidate either breaks at the first prose line or takes the prose in as a column — which the
row-order clause then rightly refuses. Six pages of one 10-K scored zero for that alone.

**`-v4` runs the rule again inside each of the page's columns, where one of them is prose.** The
columns are the reading-order rule's first vertical cut (`crate::reading_order::columns`): the
gutter every run left free by a table leaves clear, between bands that sit beside each other —
the cut alone, never its recursion, which would go on to cut a table's own columns apart. A column
is **prose** with three lines or more, at least half of them thirty characters or more. The test
is what keeps a page that is one wide table from being cut at the gutter between its labels and
its values: neither side of that gutter is prose, so the page stays the page-wide pass's.

Measured on a scratch build: nine of the 107 pages gained a table — the six 10-K pages and three
others — and on ParseBench tables rose 0.3224 → 0.3319, eleven pages up and none down; content
and formatting did not move, visual grounding 0.4042 → 0.4043, three pages down and five up.
opendataloader-bench did not move at all. Of the thirteen tables it added on ParseBench's text and
layout pages, most are tables those pages hold; the false ones were a company-facts block read
as one column of the shareholder table beside it, a row of key-figure tiles, and a list of icon
bullets — the icon font mapping its bullet to `Ȟ`, which the two-column label clause did not take
for a label because it accepted a single letter only from ASCII.

**So any single character is a list label.** An icon font's bullet can map to any letter, and a
first column of single characters beside running text is a list whatever the characters are.
That took the bullet list out (visual grounding 0.4043 → 0.4045) and changed nothing else
measured.

With their trees stripped, the gate documents gain three tables from `-v4`: a 4 × 4
security-impact grid and a questionnaire `nist-sp-800-161r1` declares, and a reference list on
`nist-sp-800-37r2`; `nist-sp-800-53Ar5` gains 110 more of the two-column objective lists it
already yields 262 of without the cut, which its tree declares as lists and which this rule never
reads on the tagged original. The gate corpus itself declares its structure, so the rule emits
nothing there and the table gate does not move.

Measured with the shipped build, all 2,078 pages: ParseBench tables **0.3224 → 0.3319**, eleven
pages up and none down; visual grounding 0.4042 → 0.4045, two pages down and five up; semantic
formatting unchanged; content faithfulness unchanged at 0.6613, one page 0.0006 lower — the page
whose table sits in the middle of two columns, now read as a table. Overall 35.94 → 36.14.
opendataloader-bench unchanged on all three measures, no table on any of its 158 documents
without one.


## 10. Measured under `-v4` and not taken (2026-10-04)

Of the 98 ParseBench table pages still at zero under `-v4`, 28 have a candidate of three rows or
more that the row-order clause refuses with no waiver, 18 stop within two rows because the next row
sits more than 4.2 ems below, 14 stop on a cell off every track, and 11 are two columns under the
four-row floor (§8). Two changes were measured for the first two and refused:

- **The two-flow guard only where one flow is prose** (§7's guard narrowed with `-v4`'s prose test):
  opendataloader-bench TEDS 0.4254 → 0.4471 — `01030000000120`'s table, whose label column is
  written as one block, comes back — and ParseBench tables 0.3319 → 0.3343, but visual grounding
  0.4045 → 0.4041: a property-facts block beside a tenant table, and two columns of a contents
  page, are two flows neither of which is prose. Bound 3.
- **The 4.2-em reach measured from the last line taken, not from the row's first line**, so a row
  whose cells wrap does not leave the next row out of reach: ParseBench tables 0.3319 → 0.3368, but
  29 pages fell against 26 that rose — tables growing on past their end into what follows — and
  visual grounding 0.4045 → 0.4041; on opendataloader-bench `01030000000090` lost its table
  entirely (TEDS 0.36 → 0). Bound 3.

Both are the same shortfall: a row here is a line and the lines that wrap under it, so a table
whose rows are paragraphs, or whose cells sit on baselines a few points apart, is out of this
rule's reach. Rows read from the whitespace across every column at once is a different rule.


## 11. `whitespace-tracks-v5`: the header above the rows, and a cell's ink (2026-10-04)

Of the ParseBench pages where both this engine and the other parser find a table, the
table-record half of the score — records matched by their header — was zero on 26 that the other
parser scored above 0.3. Most had no header row at all: a header's cells are set centred on a row
of their own and stacked unevenly — `Line` and `Type of Data` on one baseline, `Most Recent
Calendar` and `Year` above and below it over the third column — so no line of the header has a
cell for every column, none opens or joins the table, and the table began at its first row of
values.

**The header band.** After a table is accepted, the lines just above its first row join it as one
row — each column's cells from every line, top to bottom — while each line, climbing:

1. sits within a row and a half's pitch (nine fifths of an em) of the line below it;
2. has every cell on one of the table's tracks, clear of its neighbours, no two on one track — the
   rule `grow` already applies to a sparse row;
3. has no cell of thirty characters or more (`PROSE_LINE_CHARS`): a caption or a sentence ends
   the band;
4. repeats no value its column holds below: a row the table did not take ends the band;

and the band is taken only where it names more than half of the table's tracks — a title centred
over one column names one. No line before an earlier table's end is read.

Each clause was measured on ParseBench tables and visual grounding, against `-v4`'s 0.3319 and
0.4490 (measured on the build before `line-units-v2`):

| band | tables | grounding |
| --- | ---: | ---: |
| clause 2 alone, 3.5 pitches | 0.3351 | 0.4479 |
| and half the tracks named | 0.3518 | 0.4482 |
| at 2 pitches / 1.5 / 1 | 0.3521 / 0.3525 / 0.3315 | 0.4485 / 0.4490 / 0.4490 |
| at 1.5 pitches, every band line bold | 0.3467 | 0.4490 |
| at 3.5 pitches, at most 3 / 2 lines | 0.3493 / 0.3465 | 0.4485 / 0.4489 |
| **at 1.5 pitches, clauses 3 and 4, more than half named** | **0.3504** | **0.4493** |

At 1.5 pitches without clauses 3 and 4, the tree-stripped gate documents' tables took twenty header
rows, among them rows of `nist-sp-800-161r1`'s control matrix the table had not taken, joined into
one cell, and a questionnaire's question tails; with them and at half the tracks, three — a word
of a wrapped question, a question's tail and a line whose glyphs did not decode; at more than half,
none. Bound 2 is why the last clause is "more than half".

**A cell's ink.** A cell's extent was its runs' measured boxes, and a box spans its run's trailing
whitespace: a producer that sets a table with a space drawn wide after each value — the SERFF rate
tables do it from their third row on — carried `3 ` across the gap to the next column, three cells
read as one, the row fell off the tracks, and the table stopped there. Where a run is upright and
its codes are its characters one for one, its extent now ends where its trailing whitespace's
advance begins; the box on the wire is unchanged. Alone it moved ParseBench tables 0.3319 →
0.3342, eight pages up and three down, and nothing on the gate documents.

**Measured with the shipped build**, all 2,078 pages: ParseBench tables **0.3319 → 0.3533**,
visual grounding 0.4676 → 0.4677, content faithfulness and semantic formatting unchanged, overall
37.43 → 37.86. opendataloader-bench TEDS 0.4254 → 0.4283 (`01030000000187` 0.6053 → 0.7241),
NID 0.8835 → 0.8834, MHS unchanged; no table on any of its 158 documents without one, and every
cell's text its runs'. On the tree-stripped gate documents every table and its first four rows
are as under `-v4`.

**A correction.** The ink change was first recorded as refused, for 403 more tables on the
tree-stripped gate documents (51 with a guard against a second column of running text). That
compared against a list of tables made before those documents were stripped again on
2026-10-04, from a far earlier build; against `-v4` on the documents as they are, the change adds
no table there.


## 12. `whitespace-tracks-v6`: a row with cells missing, and a column of bullets (2026-10-04)

**What `-v5` missed.** A line with fewer cells than the table's tracks, closer than a row and a
half's pitch to the row above, was read as that row's wrapped second line. A table set at a tight
pitch with empty cells had its sparse rows folded into the rows above: an invoice's items read
`2 3 | Widget B Widget C | 15 | $4.00 | $60.00 $45.00`, and `nist-sp-800-161r1`'s control matrix
read `MA-5(4) MA-6` as one control wherever a row left a column blank.

**The change.** A line with a cell on the first track and another beside it opens a row however
close it sits — a first column is where a row names itself — unless its first cell opens
lower-case, as a label's second line does. A line with no first cell, or a single cell, closer
than a row and a half's pitch, is still a wrap. Split rows also let a list through: a first column
of bullets beside its items and the lines of the next column's paragraph, three rows of three. So
a grid whose first column is nothing but bullets — single characters, none a letter or a digit —
is no table at any width, as a two-column one already was not.

Measured on ParseBench, against `-v5`'s tables 0.3533 and visual grounding 0.4677: the sparse row
alone 0.3667 and 0.4675 — `lm555`'s feature list beside its description, and two pages like it,
read as tables; with the bullet column 0.3667 and 0.4680. The lower-case clause moved no ParseBench
score and is kept for the wrapped labels of statements, which the gate corpus holds.

**Measured with the shipped build**, all 2,078 pages: ParseBench tables **0.3533 → 0.3667**,
75 pages up and 15 down; visual grounding 0.4677 → 0.4680; content faithfulness and semantic
formatting unchanged; charts 0.0031 → 0.0024 — the table on
`(Web_version)_E-Government_Survey_2024_1392024_p63`, whose region labels wrap onto a second line
level with each region's 2022 row, is no longer read, the wrapped label now opening a row of its
own; overall 37.86 → 38.12. opendataloader-bench unchanged on all three measures, no table on
any of its 158 documents without one. On the tree-stripped gate documents no table is added, and
the tables that change are rows folded together under `-v5` standing apart: `MA-5(4)` and `MA-6`,
`AC-17` and `AC-17(6)`, `PT-04`'s consent rows.


## 13. `ruled-rects-v8`: a grid read among the page's other ink (2026-10-04)

**What the census found.** Of the table pages the engine still scored zero, 75 carried a
`ruled-table-candidate-refused` — and on most the grid was drawn whole. `ruled-rects-v7` reads every
rectangle a page painted as one lattice, so a footer's rule, a logo's box or a second table puts a
line into the grid that nothing traces across it: the European Medicines Agency's research-needs
tables (`203924…`, twelve pages) are ruled in full, every joint filled, and were refused for
*"column boundary 1 of 7 is not traced end to end"* — the boundary at the end of the page's footer
rule.

**The change.** Where the page-wide lattice is refused, each group of rectangles that touch
(within 2pt) is a candidate of its own, built and accepted exactly as a page's was. Where the
page-wide lattice stands, nothing changes, so every table `-v7` accepted is untouched. A group's
grid stands only where it is the table's, each clause measured before it was written:

1. **Not one of three or more grids sharing their column lines.** A table shaded in bands
   (opendataloader-bench `01030000000078`) is one table, and each band read alone is a fragment:
   TEDS on that document fell 0.87 -> 0.40 without this. Two such grids are allowed — two small
   tables one above the other with a caption between (`AZ LIC Rate Tables` p89, p93).
2. **Not a grid with a ruled row whose cells each hold three or more lines on shared baselines.**
   A table ruled only round its outside and between its columns is one header row and one body row
   to the lattice, and sixty rows of rates folded into one: `FBLB-134215544` p19 and p84 fell 1.0 ->
   0.02 without this.
3. **Not a grid whose rows mostly run on beside it on their own baselines**, in text no other grid
   holds and within its width of its edge: the grid is part of a wider table. Text another grid
   holds is that grid's — two tables side by side are two tables.
4. **Not a grid with a column that holds no letter or digit in any row**: a row's shading drawn
   piece by piece around a currency sign splits `$` from its amount (`Home Depot 10-k` p33, p58,
   p71).

Ids are given only to grids that stand, so a page where none does keeps every id it had.

**Measured** — the gap swept on ParseBench's table pages: 0.3989 at no gap, 0.4061 at 1pt, 0.4066
from 2pt to 4pt, 0.3998 at 8pt, where neighbouring tables start to join. With the four clauses,
at 2pt:

| | `-v7` | `-v8` |
|---|---:|---:|
| ParseBench tables | 0.3667 | 0.4092 |
| ParseBench charts | 0.0942 | 0.0977 |
| ParseBench visual grounding | 0.4680 | 0.4687 |
| ParseBench content, formatting | unchanged | unchanged |
| opendataloader-bench NID, TEDS, MHS | 0.8819, 0.4283, 0.5458 | unchanged |

46 table pages rise and none falls; no table on an opendataloader-bench document whose ground
truth holds none, and every cell its runs' text. On the eight tree-stripped gate documents 60 grids
`-v7` refused are read — `nist-sp-800-53Ar5`'s boxed assessment procedures, one per control, three
of them replacing a smaller whitespace table over the same text; `nist-sp-800-207`'s acronym list,
one table of 22 rows where the whitespace rule read two; two of `nist-sp-800-161r1`'s.

**`ruled-rects-v9` (2026-10-05).** Clause 2 asked any two cells of a ruled row; once
`declared-font-codes-v3` read the text a producer's `q … Q` had hidden, two cells of wrapped prose
beside a one-line label shared their baselines and the EMA research table lost its grid. It now asks
every cell of the row that holds text ([`33-UNMAPPED-CODES-SCOPE.md`](33-UNMAPPED-CODES-SCOPE.md) §4).

**`ruled-rects-v10` (2026-10-05).** A grid accepted by its lines had to have every line traced end
to end, so a row spanning every column refused its table — the European Medicines Agency's
research-needs tables on eleven ParseBench pages, a focus-area row across each. For one group's
rectangles an undrawn segment of an interior line now joins the faces either side into one cell
(`Lattice::merged_cells`), under four clauses: every other line drawn end to end; every interior
line drawn across one band at least; no segment drawn inside the rectangle a cell so formed spans;
no cell spanning rows with lines on the baselines of the one-row cells beside it in two rows — a
column the page left unruled, as on `FBLB-134215544` p122, where without the clause 63 rows of zip
codes read as one cell. Each band is asked alone, so a gap where a merge crosses a line in one band
says nothing about the next, and a rectangle a fill alone painted white draws no line: on
`text_dense__underline` a white panel behind a source note closed a second row under a boxed
passage and the passage read as a table. **Never on the page-wide lattice**: merged there,
`FBLB-134215544` p19 and p84 folded sixty unruled rows into one, because clause 2 of `standing()`
asks only after a group's grid.

Measured on ParseBench's table pages: 0.4148 -> 0.4410, 34 up and 5 down; 0.4502 with the adapter
reading a table that holds a merged cell from the engine's HTML projection, 44 up and 8 down. The
falls are pages the whitespace rule had read better: `AZ LIC Rate Tables` p48 (a two-line header the
truth holds as one cell), `FBLB-134215544` p16 and p43 (unruled rows under a merged header),
`SERFF_TX` p1051 and p92, `SERFF_CA` p1201, and two of under 0.003. Measured and refused: asking an
interior line to be drawn across most of its bands (-3.0 page units, losing true row spans).
