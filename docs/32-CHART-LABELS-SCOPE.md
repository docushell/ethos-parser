# A bar chart's printed labels as its table — scope (decision #42)

**Status: shipped 2026-10-04 as `bar-labels-v1`; `bar-labels-v2` since 2026-10-09 (§7, decision #57).** A bar chart that prints its numbers states a
table; this rule reads that table back. It never reads a value off a bar's length: a bar that carries
no number leaves its cell empty, and a chart that prints none yields no table.

## 1. Why

ParseBench scores charts by whether each data point's value sits in a table cell beside its labels
(`chart_data_point` rules: a value, and the category and series it belongs to). Before this rule the
engine scored **0.0024** there — it emitted a chart's labels as loose lines, and the score is the
lowest of the five dimensions by two orders of magnitude.

The evidence is already on the page for many charts. Measured on the 566 chart pages before any
code: for **21%** of the 4,864 data points the value and its labels were both present as text in the
engine's output, and averaged per page that caps a perfect pairing at about **0.27**. 95% of the
points name two labels or more — a category and a series — so the series has to be read too: from a
legend swatch of the bar's colour, or a one-colour chart's title.

## 2. The rule, `bar-labels-v1`

Its own profile field, `table_detection.charts`, beside the five other table rules: the evidence —
filled bars, their colours, and the labels set at them — is its own. It runs after
`whitespace-tracks-v6`, on runs no other rule's table holds, only on a document that declares no
author structure (the tracks rule's gate, for the tracks rule's reason), and keeps a table only where
it overlaps none already found.

1. **Fill colour.** The content-stream interpreter now carries the fill colour in its graphics
   state (`g`, `rg`, `k`, `sc`, `scn`, reset by `cs`, saved by `q`), and records each rectangle a
   fill paints with the colour it was painted in. Colours are compared, never converted. A pattern,
   or operands that are not numbers, is a colour this reader does not know, and matches nothing.
   The ruled-table rule reads exactly the rectangles it read before.
2. **Bars.** Filled rectangles in a known colour that is not white, a point or more each way, under
   a quarter of the page; one drawn twice counts once.
3. **Columns.** Bars sharing one span across their direction, set end to end, each a colour the one
   before it is not — a stack. Two touching segments of one colour are a row of shaded table cells
   or a bar drawn in pieces, never one stack.
4. **A chart.** Two or more columns on one base, of one width (within 15%), side by side (no wider
   apart than four widths). Not when every bar is one segment of one length: that is a table's
   banding. Its categories are the columns — or, where the colours repeat with a period of two or
   more and the gaps inside a period are narrower than between periods, each period. Its series are
   the stack's colours, the period's positions, or its one colour.
5. **Labels.** Upright runs (the tracks rule's test), joined along a baseline wherever less than half
   a line height separates one run's ink from the next; a space the page drew as a run of its own,
   inside a label, is the label's.
6. **A value** is the one number whose centre lies on its bar's segment, or — for a bar of one
   segment — the one just beyond its end. A number two cells could read fills neither.
7. **A category** is the one label nearest the base under its column or group (vertical) or beside
   it (horizontal), with the lines a long label wraps onto (only where the first line fills most of
   its slot, or a line ends in a hyphen). Never a number but a year, never a label that opens with a
   value, never a line of prose (opening lower-case and thirty characters or more), never one set on
   a bar.
8. **A series' name** is the one label beside a small swatch of its colour, near the chart and
   nearer it than any other chart or bars of that colour, not running into the next legend entry,
   with its wrapped lines; a legend above the chart only where nothing but the chart's own labels
   stands between. A chart of one colour without one takes the one-line title just above it, where
   that line is the only label over the chart and holds no number but a year.

## 3. What makes it a table and not a coincidence

1. **The numbers agree with the bars in order**: a bar a tenth longer never carries a smaller number,
   and one a third longer carries a larger one. A linear fit was measured first and refused: charts
   are rarely drawn exactly to scale, and it rejected true charts whose labels are rounded.
2. **Labels are systematic**: a series keeps its numbers only where six bars in ten of it carry one.
   An overlaid line's labels, printed over two bars of seven, are not those bars' values — the
   prototype paired exactly that before this clause.
3. **Nothing is read twice**: no label is two categories, no two categories read alike, no run is in
   two tables (where two candidates share labels, the one with more cells stands).

A cell holds exactly its label's runs and its text is their text concatenated; a cell with nothing
printed is not emitted. The table's cells are labels where the page set them, not boxes tiling a grid,
so the lattice cross-check is `NotApplicable` with that reason — as a tagged table's is.

## 4. Bounds

1. **Every cell is its runs' text**, on every corpus run (opendataloader-bench: 0 of 1,042 cells
   differ).
2. **A chart cell counts as invented when it disagrees with the drawing** — not when a document's
   ground truth has no table. The tracks rule's "no table on a document without one" is the wrong
   test for a chart: three opendataloader-bench documents draw real bar charts, get correct tables
   (checked against the rendered pages), and have no table in their ground truth.
3. **No chart table where there is no chart**: ParseBench's 503 table pages and 508 text pages get
   none; the eight tree-stripped gate documents get none, and every other table on them is
   unchanged.
4. **ParseBench content, formatting, tables and visual grounding may not fall.**

## 5. Measured (2026-10-04)

**The prototype** (`target/proto/chart_proto.py`, bars read with PyMuPDF as an instrument, text from
the engine): charts 0.0024 → 0.0930. Against ParseBench's ground truth, 310 cells agreed and 2
disagreed — both the ground truth's fault, checked on the rendered pages: one rule names India's
ride-sharing value as 4.5 where the chart prints 3.6, one omits which of two panels it means. Its
defects, each fixed before the engine rule was written: wrapped labels cut off, rotated labels read
in pieces, an overlaid line's labels paired with bars, a legend applied to another chart of its
colour, tables from rectangles drawn twice.

**The rule**:

| | before | after |
|---|---:|---:|
| ParseBench charts | 0.0024 | 0.0942 |
| ParseBench tables | 0.3667 | 0.3667 |
| ParseBench content faithfulness | 0.6627 | 0.6627 |
| ParseBench semantic formatting | 0.4063 | 0.4063 |
| ParseBench visual grounding | 0.4680 | 0.4680 |
| opendataloader-bench TEDS | 0.4283 | 0.4283 |
| opendataloader-bench NID | 0.8834 | 0.8819 |
| opendataloader-bench MHS | 0.5465 | 0.5458 |

Chart tables on 91 of 566 chart pages (136 tables, 915 rows); 330 cells agree with ParseBench's
ground truth and the 5 that disagree are the two ground-truth errors above and three rules about
another chart on the same page. 26 of the 101 chart pages the prototype read came from tagged
documents, where the rule does not run (§2).

**The trade, recorded.** opendataloader-bench's NID and MHS fall on exactly the three documents
whose charts became tables (`01030000000036`, `-076`, `-077`): its ground truth keeps a chart's
labels as loose lines in drawing order, and the table reads them category by category. The tables
are correct; the falls are the reordering.

**Found and fixed while measuring the rule**: a page of tables whose every cell is shaded by a
rectangle of its own (`CSR-2024-25-Full-Report` p20) read as stacked bars, with a paragraph's lines
beside them as categories — hence §2.3's one-colour rule and §2.7's prose clause; visual grounding
on that page returned to its score before the rule.

## 6. Not done

- **Line charts**: markers and polylines as the witness, measured in the prototype at +0.002 with
  more ways to go wrong; not in this rule.
- **Pie charts, rotated labels, charts in images.**
- **Charts that print no numbers**: reading a value off a bar's length would be estimating, which
  this engine refuses.
- **Tagged documents**: the rule follows the tracks rule's gate. A tagged document's chart is a
  `/Figure`, and whether a figure's printed data may become a table there is its own decision.

## 7. `bar-labels-v2`: corners drawn twice, groups from their gaps, colours by category, a legend beyond the categories (2026-10-09, decision #57)

**What `-v1` missed.** Told "Charts, round 2", the chart data points were read for why each failed.
3,569 of the 4,864 ask for a value read off a bar's length, which this engine refuses; the printed
ones cap a perfect reading at 0.34 against `-v1`'s 0.1035. Of the printed points that failed while
the engine read their value as text, most sat on charts this rule read no table from, and the
reasons were many and each small:

- a producer that names every corner of a bar twice — a rounded corner of radius nought — drew no
  rectangle the interpreter read (`Earnings_Presentation` slides);
- grouped bars with a bar left out — a zero drawn as no bar — or one group highlighted in colours of
  its own matched no period of colours, and bars coloured one per category matched none either;
- a legend set beyond long category labels stood further from the bars than the chart's own size.

**The change.**

- **Corners named twice.** The interpreter drops a point named again at once and reads the
  rectangle that leaves into a list of its own, which only this rule reads: the ruled rule, and
  `ruled-rects-v12`'s padding test, read exactly the rectangles they read before.
- **Groups from their gaps.** Where no period of colours holds, a gap more than twice the widest of
  the narrow ones ends a group: two groups or more, one of them two columns or more. A series is a
  colour two groups or more hold, its place the mean of its first places there; a column of another
  colour takes the series of its place, only in a group holding one column per series; and no group
  holds two columns of one series.
- **Colours by category.** Three columns or more, each a colour none of the others is, are one
  series, and such a chart takes no title as its series' name: its colours key its categories, and
  the line above it is as often a section's heading. Measured with the title, a visual grounding
  page lost its heading to the table's header (`AXP_2023_2024_ESG_Report` p46, 0.34 → 0.31), and no
  chart page needed it.
- **A legend's distance** is measured from the chart and the labels it read, not from its bars
  alone.

**Measured**, with the shipped build on all 2,078 pages, against `8cfffd9`: ParseBench charts
**0.1035 → 0.1179**, 13 pages up and none down (`Earnings_Presentation` FY25 Q4 p19 and FY26 Q1
p19 0 → 1.0, `US_Professional_Services_Partner_Compensation_Survey` p19 0 → 1.0, `She-figures` p105
0 → 0.9); tables, content, formatting and visual grounding unchanged on every page; overall
44.89 → 45.17. No chart table on a table or text page, the gate documents' 539 tables unchanged, and
the heading bounds read as before on all 11 documents. opendataloader-bench TEDS unchanged; two
documents whose charts became tables, both right against their drawings (`01030000000071`,
`-072`), read in a new order — NID 0.8846 → 0.8841, MHS 0.5459 → 0.5457 — the trade decision #42
took, on two documents more.

**Measured and not used.** Rounded bars read from their straight sides (no page up;
`Digital_News-Report_2022` p17 0.75 → 0.5, a rounded mark of a series' colour taking its legend's
name); a highlighted bar inside a run of one colour read as that colour's; a title over the
chart's left half; numbers with a sign after the currency sign, and the units `b`, `t`, `mn`, `tn`;
a label of numbers joined across bars split run by run (one page up, one down). Each moved nothing,
or as much down as up.

**Still not read.** Values drawn one glyph per run that join their neighbours' with no gap
(`ADL_Future_of_automotive_mobility` p13: `3.73.6`); small multiples, whose data points name a panel
as well as a category and a series; categories in two tiers; bars hanging below a base; a line
drawn over the bars; pies and lines (§6).
