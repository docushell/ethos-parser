# A bar chart's values read off its axis — scope (decision #122)

**Status: accepted 2026-11-11; S1 built, measured at no gain and parked the same day (§7).** The
owner chose "Chart reader, then assist Phase 0", then "≈ before the number" and "Opt-in flag"; told
what S1 measured, "Park #122, start assist Phase 0". Nothing of it is in the tree. This scope amends one sentence of
[`32-CHART-LABELS-SCOPE.md`](32-CHART-LABELS-SCOPE.md) §6 — *"reading a value off a bar's length would
be estimating, which this engine refuses"* — and only behind a flag: without it, every output is
what it is today.

## 1. Why

The owner's release gate: no release until ethos-parser's ParseBench place is 49th or better. On
2026-11-11 the 49th entry's overall is 69.12 and ethos-parser's 53.94 (109th of 157). Charts are the
widest gap: 17.26 against 35 to 68 for the entries around 49th, worth 15 overall points by
themselves.

- **What the rule refuses is most of the dimension.** Of ParseBench's 4,864 chart rules, 3,569 ask
  for a value read off a bar's length (`32-CHART-LABELS-SCOPE.md` §7); the printed values cap a
  perfect reading at 0.34.
- **The charts are drawings.** 431 of the 568 chart pages draw their charts as paths (77% of the
  rules); 67 hold them as images, which this scope leaves to assist's Phase 0.
- **The answers allow for reading off a drawing.** 1,792 rules accept a value within 5%, 1,118
  within 10%, 301 within 20%: the annotators read them off the bars too.
- **Gaps are not where the points are.** Only 22 failing rules sit on an empty cell of a table the
  engine reads, because `bar-labels` reads no table at all from a series that prints no numbers
  (six bars in ten must carry one). This rule reads those charts; it does not fill holes.

- **The engine already finds the charts.** On the 568 pages `bar-labels` finds 726 charts and reads a
  table from 181. Of the 545 it does not read, 239 set three numbers or more along the side a value
  axis takes: this rule's first targets, against the 181 read today.

A throwaway reader outside the engine (bars only) moved the dimension 0.1658 ->
0.1863 with no rule lost; the engine's own chart reader already does what that prototype lacked —
stacks, groups, legends, titles, wrapped categories — so the measurement that counts is S1's.

## 2. The rule, `bar-lengths-v1` (with `--measure-charts` only)

**Where it reads.** A chart `bar-labels` finds — its bars, columns, groups, stacks, series,
categories, legend and title, all found as today — whose series print no numbers. A series that
prints its numbers keeps them, read as today; a measured value never replaces a printed one.

**The value axis.** The numbers set in a line beside the bars' base side — left or right of
vertical bars, below or above horizontal ones — on no bar, and read as no category or legend:

1. at least three of them, all different, in order along the axis;
2. one straight line through their positions and values, each within 1% of their span;
3. the bars' common base reads the axis's zero, or its lowest number, within the same 1%.

**A value** is the bar's length on that line — for a stack, each segment's — rounded to one decimal
more than the axis's numbers carry, and written with `≈` before it: `≈0.42`.

**Refused by name**, the chart then reading no table as today: no axis; fewer than three numbers;
numbers out of order; a line further than 1% from any of them; a base that is neither zero nor the
lowest number (a broken axis); numbers that grow by a factor rather than a step (a log axis).

## 3. What it writes

- **A measured cell** holds no runs. It carries `measured: "bar_length"`, a new optional field, and
  its text is `≈` and the number. Contract §6: a measured value is `Computed` from the drawing and is
  never text the page states, so `locate` and `ground` never bind one.
- **The seal** checks it: a measured cell holds no runs and its text is `≈` and a number; any other
  cell holds exactly its runs' text, as today.
- **A limitation**, `chart-values-measured`, document-scoped, counts the measured cells. It is
  written only under the flag.
- **The profile** gains `chart_values`: `printed` by default, `measured` under the flag. The default
  profile's hash moves, as decision #120's did; the flag's differs from it.
- **Markdown and HTML** print the cell's text, `≈` included: ParseBench reads `≈0.42` as 0.42
  (checked), and a reader sees at once the number was measured. In the anchor map a measured value
  is a `Syntax` segment — bytes the exporter wrote, naming no node, which a quote cannot invert —
  because the document drew a bar there, not these characters. No new segment kind, no schema
  change; `Syntax`'s description gains the case.

## 4. Bounds

1. Without the flag, the gate's eight documents and opendataloader-bench's 200 extract
   byte-identical but the profile hash.
2. With it, no printed cell changes, and a table appears only where this rule reads one.
3. ParseBench with the flag: no page scores lower than without it. The target for the dimension is
   set from S1's measurement, and recorded here before S2.

## 5. Slices

- **S1** — bars and stacks, upright and sideways: the axis, the measured cell, the seal's rule, the
  limitation, the profile field, `--measure-charts` on `extract` and `markdown --source`, the
  adapter's switch. Measured on ParseBench.
- **S2** — line charts: a point's height on the same axis, for the charts `point-labels` finds.
- **S3** — pies: a slice's angle as its share of the whole.

## 6. Not done

Log axes, broken axes, a second axis for some of the series, charts drawn in 3-D, and charts held
as images (assist's Phase 0 measures those).

## 7. Measured, and parked (2026-11-11)

S1 was built as §2 to §5 say — the profile's `chart_values`, the measured cell and its seal rule,
`≈` written as `Syntax` in Markdown and HTML, the limitation, the axis, `--measure-charts` and the
adapter's switch — and measured on ParseBench before anything was committed.

- **With measured values read inside the chart reader**, 4 pages moved: 3 up, and one printed table
  lost (`WEF_Navigating_Global_Financial_System_Fragmentation_2025_p12`, 0.75 -> 0), because a
  measured reading changed which of the reader's readings stood. That breaks §4.2.
- **With the printed table read first and kept**, and a chart measured only where it reads none,
  0 of 2,553 examples scored differently, and not one measured cell was written anywhere.
- **Why.** Of the 726 charts `bar-labels` finds on the 568 chart pages, a value axis fits on many
  of those it does not read, but their rows never form: the reader finds no category under their
  bars. Rotated category labels account for part of it (52 pages, 229 of the 1,938 rules whose value
  is not printed); the rest is a long tail — labels another rule's table holds, labels set further
  off than the reader looks, axes it does not read.

So the value is not what stops these charts, and reading it adds nothing until their categories
are found. The design above stands for the day it is resumed; the owner moved the rank path to
assist's Phase 0.
