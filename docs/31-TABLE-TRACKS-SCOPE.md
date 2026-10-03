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

