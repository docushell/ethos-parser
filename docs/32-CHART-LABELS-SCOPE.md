# A bar chart's printed labels as its table — scope (decision #42)

**Status: shipped 2026-10-04 as `bar-labels-v1`; `bar-labels-v2` since 2026-10-09 (§7, decision #57); `bar-labels-v3` and a line chart's rule, `point-labels-v1`, since 2026-10-13 (§8, §9, decision #66); a pie's rule, `slice-labels-v1`, since 2026-10-21 (§10, decision #90).** A bar chart that prints its numbers states a
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
   a quarter of the page; one drawn twice counts once. A bar drawn with its ends or corners rounded
   — curves and straight sides only, two sides or more straight and none on a slant, every point
   on the edge of its box — is its box (decision #88).
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
   value — a range of two values, `$1.00bn–$5.00bn`, is a category (decision #87) — never a line of
   prose (opening lower-case and thirty characters or more), never one set on a bar.
8. **A series' name** is the one label beside a small swatch of its colour — a rectangle, or a dot,
   never a rounded bar (decision #88) — near the chart and nearer it than any other chart or bars
   of that colour, not running into the next legend entry, with its wrapped lines; a legend above
   the chart only where nothing but the chart's own labels stands between. A chart of one colour
   without one takes the one-line title just above it, where that line is the only label over the
   chart — over any part of it, not only its centre (decision #87) — and holds no number but a
   year.

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
  more ways to go wrong; not in this rule. **Read since decision #66** by a rule of their own,
  `point-labels-v1` (§9).
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

## 8. `bar-labels-v3`: the title over a chart names its one series that prints numbers (2026-10-13, decision #66)

`-v2` read the title over a chart as its series' name only where the chart was drawn in one colour.
A chart of bars drawn over a full-length track — a progress bar, one colour printing its numbers
and one printing none — kept two series, and its one column of numbers went unnamed, though the
panel's title over it names it (`(Web_version)_E-Government_Survey_2024` p78, three panels each
titled). `-v3` reads the title for the one series that carries numbers, whatever else is drawn
beside it, and a series with no number is still no column. Measured alone: ParseBench charts
0.1184 → 0.1202, that page 0 → 1.0, nothing else moved.

**Measured beside it and not used:** rounded bars read as bars (one page given a table, and it
still scored nothing without its legend's dots — as `-v2` measured them, §7); panels sharing a
category axis merged into one table (at most fifteen pages and 66 points, each needing the panel's
title and the legend as well).

## 9. `point-labels-v1`: a line chart's printed values (2026-10-13, decision #66)

**Why.** Told "Charts, round 3", the 846 chart data points that print their value and still failed
were read for why: 284 sat on pages where the bar rule found no bars at all — line charts, pies,
charts drawn as images — 328 on pages whose bars it refused, and 234 on pages whose table missed
them, the reasons many and each small. The largest coherent piece left was a line chart that prints
a number at each point: about eight pages. Round 1's prototype read lines and measured +0.002; its
reasons were found and mended before this rule was written (below).

**The rule.** Its own profile field, `table_detection.lines`, beside the bar rule's: the evidence —
stroked lines, small marks, their colours, and the labels set at their points — is its own. It runs
after `bar-labels-v3`, on runs no other rule's table holds, only on a document that declares no
author structure, and keeps a table only where it overlaps none already found.

1. **Strokes and marks.** The interpreter carries the stroke colour as it carries the fill (`G`,
   `RG`, `K`, `SC`, `SCN`, reset by `CS`, saved by `q`), and records each subpath a stroke paints —
   every point a line or a curve ends at — with it, and the box of each filled path a point to
   fifteen points each way and near square, with its fill. Nothing else reads either.
2. **A series** is a stroked line of three points or more, each standing more than half a point
   right of the one before it, not all at one height — a line drawn twice read once — or three
   marks or more of one colour and size so set. A line drawn through a run of marks is that run;
   marks all on a longer line are the line. A line stroked segment by segment, from either end, is
   one line: strokes of one colour joined end to end, each next running on to the right of where
   the last ended (decision #89).
3. **A value** is a number set at one point: centred over or under it within a line and a half, or
   beside it on its height within a line and a fifth. Every number goes to its nearest point over
   every series of the page, and **is no one's where another series' point stands nearly as near**
   (within one and a half times the distance and a third of a point) — round 1 asked each series
   alone, so two lines running close read each other's numbers as ambiguous and neither kept any.
   Numbers are seated so that as many are as can be, each at the nearest point it can take
   (decision #89): a series that sets its numbers under its points, nearer the next line's points
   than its own, had them taken by that line, which then left its own out; each point once kept its
   nearest number. A series' numbers are of one kind: where most of the numbers reaching its points
   end in a percent sign, one that does not is none of its values, and the other way round — a bar's
   `398` set under a line of percentages' last point stood nearer it than the line's `4.6%`
   (decision #89). **A year is never a value**: a timeline's dates set at its marks, or a year axis
   set along a line, read so were a table of dates. A number set on a bar is the bar's.
4. **A series keeps its numbers** only where six points in ten carry one, and **a point clearly
   higher on the page never carries a smaller number** (by half a line).
5. **Categories** are the first row of labels below the series' own lowest point, of the forty rows
   below it — holding one label under eight points in ten, no label twice and at most half of them
   numbers but years. Round 1 looked twelve rows below each series' own lowest point, under which a
   chart's other series print their numbers, and refused a tenor's `3M` as a number; `-v1` then
   looked below the lowest point of every series set at the same places, and three charts stacked
   down a column at the same places read the bottom one's axis (decision #89). A label reaching over
   a point beside its own is set across several — an axis's labels set close run together — and
   names none of them, nor is its point counted against the row (decision #89).
6. **A table** is every series reading one row of categories, a row per category in the order its
   points stand, a column per series, each named by the block of lines beside a legend swatch of its
   colour — a small mark or a short level stroke, its name read line by line up to the next swatch
   — or, alone, by the one-line title over it, measured from its highest number. A legend set in
   rows names each row's swatch by its own line: the label nearest the swatch's height, and no line
   led by another entry's swatch (decision #90). A cell holds exactly its label's runs; an empty
   cell is not emitted; a value is never read off a point's height.

**Measured**, with the shipped build on all 2,078 ParseBench pages and opendataloader-bench's 200,
against decision #65's: ParseBench charts **0.1184 → 0.1248**, seven pages up and none down
(`She-figures` p88 and `E-Government_Survey` p78 0 → 1.0, `2022_SPI_Benchmark_Executive_Summary`
p19 0 → 0.6 and p20 0 → 0.4, `article_thebeatjun2025` p34 0 → 0.3, `Technopak_Industry_Report` p4
0 → 0.2); tables, content, formatting and visual grounding unchanged on every page; overall 46.29
→ 46.42. Seventeen line tables on the chart and layout pages — `She-figures` p88, `article_thebeatjun2025`
p22 and opendataloader-bench `01030000000078` checked against their drawings — none on a table or
text page; the gate documents' 539 tables unchanged; every cell its runs' text.

**The trade, taken by the owner.** opendataloader-bench `01030000000078` draws ASEAN's remittances
as bars and the world's as a line over them; the line now reads as its table, right against the
drawing (`World (left axis)`: 610, 602, 597, 640, 694, 719, 702 for 2014–2020). Its ground truth
keeps the chart's labels as loose lines, so that document's NID falls 0.8786 → 0.8099 and its
TEDS 0.8667 → 0.7086 (the bench's NID 0.8841 → 0.8838, TEDS 0.5339 → 0.5301) — the trade decision
#42 took for bar charts, on one document more, and this time on its table score too.

**Measured and not used.** A series named by a label set on the line itself, at its first or last
point (moved no page); the transposed reader, for markers set down a column with their categories
beside them, was not built.

## 10. `slice-labels-v1`: a pie's printed values (2026-10-21, decision #90)

**Why.** Offered after #89 with a census of the chart rules still failing on its build — 775 of
1,295, sorted by the kind of chart — the owner chose "Pies and donuts": five pages whose pies print
a number at each slice read nothing, all five drawn as vector slices (`FPA-guide-to-data-visualization`
p20's twenty images are not its pies), at most +0.17 overall.

**The rule.** Its own profile field, `table_detection.pies`, beside the bar and line rules': the
evidence — filled slices, their colours, and the labels set at them — is its own. It runs after
`point-labels-v1`, on runs no other rule's table holds, only on a document that declares no author
structure, and keeps a table only where it overlaps none already found.

1. **Slices.** The interpreter records each filled subpath of curves that has one point every other
   point it reaches lies one distance from — its centre, where its two straight sides meet, the path
   drawn from it, ending at it, or both — with the points its arc reaches in order and its fill. A
   circle has no such point: a donut's hole, a legend's dot. Nothing else reads them.
2. **A pie** is two slices or more about one centre, of one radius, to a point, turning a full
   circle between them, to a degree. A slice drawn again, or within a larger slice of its colour, is
   that slice: a donut's producer filled one slice twice.
3. **Its numbers.** Each number on the page goes to the pie whose rim it stands nearest, within the
   pie's radius of it, and one set over a pie's centre is no slice's. A year is never a value. A pie
   takes as many numbers as it has slices, those nearest its rim first — a number set on a slice
   stands at none — all of one kind, percentages or not, as a line's are (decision #89): a stray `5`
   beside a pie whose slice printed no number stood in for that slice's.
4. **Each slice's number.** The numbers, taken round the centre in order, are matched to the slices
   in theirs, at the one turn of the one against the other that sets them nearest: numbers set beside
   a pie on leader lines keep its slices' order, though a small slice's number stands over the next
   slice (`FPA-guide-to-data-visualization` p20). Each stands within a quarter turn of its slice, and
   no second turn sets them as near. A slice a tenth larger than another never carries a smaller
   number, and one a third larger carries a larger one — a number in brackets by its size, for a
   slice is drawn as large for `($60B)` as for `$60B`.
5. **Names.** A slice is named by the block of lines beside a legend swatch of its colour, as a
   line is (§9 item 6), or — where the legend cannot tell two slices of one colour apart, or there
   is none — by the lines just over its number, each centred on it, the nearest first, none a
   number. A pie is named by the label in its hole, or by the line nearest over everything it holds,
   within four lines — its labels over this pie and no other of its chart — where that line is no
   number but a year and ends in no full stop: a number set over a pie is a callout, and a
   sentence's last line is prose.
6. **A table** is the pies set side by side — at one height, sharing a colour — read as one: a row
   per slice, a column per pie. A cell holds exactly its label's runs; an empty cell is not emitted;
   a value is never read off a slice's size.

**Folded in, as no release carries them.** A number may end in `b`, for billions, as `$1,581B` —
the unit §7 measured and did not use, which a pie needs; it reads two bar charts more, each right
against its drawing (`Activate_Consulting_Technology_&_Media_Outlook_2026_(10)` p60,
`natural-catastrophe-and-climate-report-2023` p16), and moves no page's score
(`bar-labels-v3`, `point-labels-v1`). A legend set in rows names each row's swatch by its own line:
a legend of two rows had read each name as both rows together (§9 item 6, `point-labels-v1`).

**Measured**, with the shipped build on every ParseBench page and opendataloader-bench's 200,
against decision #89's: ParseBench charts **0.1363 → 0.1485**, nine pages up and none down
(`FPA-guide-to-data-visualization` p20, `MonthlyTreasuryStatement_202411` p8 and `_202509` p8,
`Technopak_Industry_Report` p18 and p6, `WEF_The_Global_Public_Impact_of_GovTech_2025` p29 0 → 1.0;
`us-global-outsourcing-survey-2024-report` p13 0.6 → 1.0; `MonthlyTreasuryStatement_202509` p7
0 → 0.3; `Renewables2025_1` p48 0 → 0.22); tables, content faithfulness, semantic formatting and
visual grounding unchanged on every page; overall 48.60 → 48.84. Eleven pie tables, ten on chart
pages and one on a layout page, each checked against its drawing, none on a table or text page;
opendataloader-bench unchanged on every document; the gate documents' 541 tables unchanged; every
cell its runs' text.

**Still not read.** A pie whose slices are drawn as polygons, or as ring segments with no centre; a
slice whose number shares a line with its name; a pie with a slice that prints no number.
