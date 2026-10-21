# ParseBench — run locally as an instrument

**Measured 2026-10-03.** ParseBench (`run-llama/ParseBench` at `afb36bd`, dataset
`llamaindex/ParseBench` at revision `2805a1d`, both Apache-2.0) scores a parser's Markdown on five
dimensions over 2,078 pages: tables, charts, content faithfulness, semantic formatting and visual
grounding.

**This is an instrument, not a leaderboard entry.** It was run to find defects, and it found them:
every engine change it led to is in `CHANGELOG.md` under "ParseBench, run as a diagnostic" and
"Form XObjects are read as page content". `docs/CAPABILITY.md` refuses a benchmark table comparing
this engine with other parsers, and this page holds the engine's own numbers only.

## How to run it

```bash
git clone https://github.com/run-llama/ParseBench ~/ethos-external-benchmarks/ParseBench
cd ~/ethos-external-benchmarks/ParseBench && git checkout afb36bd && uv sync
cp <this checkout>/docs/measurements/parsebench/ethos_bench.py .
uv run parse-bench download
ETHOS_PARSER_BIN=<this checkout>/target/release/ethos-parser uv run python ethos_bench.py run ethos_markdown
```

`--group text_content` (or `table`, `chart`, `layout`, `text_formatting`) runs one dimension;
`--test` runs fifteen pages. A full run takes about three minutes.

## What the adapter does

[`ethos_bench.py`](ethos_bench.py) registers through `parse_bench.extensions`, so the benchmark's
tree is unmodified.

- **Text.** It runs `extract`, then `markdown` on the representation. A refusal is a
  `ProviderPermanentError` and scores zero, as the benchmark intends.
- **Tables.** The table scorer reads only HTML `<table>` blocks, so pipe tables are converted with
  the benchmark's own LiteParse helper, as every local provider there does.
- **Visual grounding.** It reports the engine's own units with their boxes: each `layout_unit`
  (decision #38) — the `ground` elements whose runs share one, boxed by their union — or each
  `ground` element where a run carries none; each table; and each image. Labels come only from what
  the record states: a tagged role path, an `inferred_heading` run, a detected table, a drawn image,
  and `Text` otherwise. The benchmark's LiteParse layout adapter and label mapper are reused under
  the key `ethos`.
- **A table is one item.** A `ground` element whose runs a reported table holds is not reported
  again. Until 2026-10-04 it was, so every table found was counted twice — once as the table, once
  as its cells' lines — and a page held 131.6 items on average against 33.0 in ground truth. Fixed,
  it holds 53.9, text F1 goes 0.3168 -> 0.3400, and the headline element rule pass rate does not
  move (0.4034 either way).

**Measured and not used: the engine's blocks as layout units.** Grouping runs by
`TextRunAttributes::block` scored lower than one baseline's ink (element pass rate 0.174 against
0.220). A block often holds several paragraphs and a heading, so attribution fails. The ground
truth is paragraphs, and the engine makes no paragraph on an untagged page.

## Layout units, measured before the rule was written (2026-10-04)

Visual grounding scores a ground-truth element only where one predicted item covers it, and the
ground truth is paragraphs; a line item covers one line of one. So the rule was first measured as a
merge of the adapter's line items in a copy of a finished run — `ground` elements merged while the
next sits no more than a fraction of its own height below the merged box and overlaps it across —
and then written into the engine as `line-units-v1` with the best variant's clauses:

| merge | `layout_element_rule_pass_rate` |
| --- | ---: |
| none — one item per `ground` element | 0.2264 |
| gap ≤ 0.8 line, list marker opens an item | 0.3947 |
| gap ≤ 1.2 line | 0.3718 |
| gap ≤ 0.3 line | 0.3933 |
| gap ≤ 0.8 line, no list-marker clause | 0.3877 |
| gap ≤ 0.5 line | 0.4061 |
| gap ≤ 0.5 line, overlap ≥ 0.3 instead of 0.5 | 0.4066 |
| **gap ≤ 0.5 line, heading lines of one level merged** | **0.4114** |

The merge in the adapter was a measurement only. The adapter groups by the engine's
`layout_unit` and makes no unit of its own. Written into
the engine, `line-units-v1` scores **0.4028** — a little under the best merge, since its line is the
heading rule's (one band, one baseline) rather than the `ground` element, and a table's runs join
no unit.

## The engine's numbers

At `05f0d17`, all 2,078 pages:

| Dimension | Score | Metric |
| --- | ---: | --- |
| Content faithfulness | 0.6624 | `content_faithfulness`, 506 text pages |
| Semantic formatting | 0.0970 | `semantic_formatting`, 476 pages |
| Tables | 0.0212 | `grits_trm_composite`, 503 pages |
| Charts | 0.0000 | `rule_pass_rate`, 568 pages: data points are drawn, not written |
| Visual grounding | 0.2200 | `layout_element_rule_pass_rate`, 500 pages |

61 pages are refused: 42 of them are images (`.jpg` and `.png`, no PDF to read); the rest are named
in each run's `_errors.json`. At `0.63.0` content faithfulness was 0.2346, and 256 of the 326
refusals then were one defect (`7cd02b8`).

### After decision #38 (2026-10-04)

With bold and italic, `type-size-v4` headings, `whitespace-tracks-v2` tables and layout units
(heading units joined in Markdown), all 2,078 pages:

| Dimension | Score | At `05f0d17` |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6624 |
| Semantic formatting | 0.3623 | 0.0970 |
| Tables | 0.2375 | 0.0212 |
| Charts | 0.0020 | 0.0000 |
| Visual grounding | 0.4034 | 0.2200 |

Each step's measurement, and what was refused on the way, is in `CHANGELOG.md` under decision #38,
`docs/31-TABLE-TRACKS-SCOPE.md` and `../headings/README.md` §9–§10.

### After `ruled-rects-v7` and `whitespace-tracks-v3` (2026-10-04)

A ruled grid drawn in lines gets its cells, and the tracks rule reads a financial statement's columns
and a table written down its columns (`docs/31-TABLE-TRACKS-SCOPE.md` §7); the adapter reports a
table once. All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.3623 | 0.3623 |
| Tables | 0.3224 | 0.2375 |
| Charts | 0.0031 | 0.0020 |
| Visual grounding | 0.4042 | 0.4034 |

No content or formatting page moved; visual grounding moved on fourteen pages, four up.

### After decision #40 (2026-10-04)

Superscripts and subscripts written `<sup>` and `<sub>`. All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6613 | 0.6633 |
| Semantic formatting | 0.4060 | 0.3623 |
| Tables | 0.3224 | 0.3224 |
| Charts | 0.0031 | 0.0031 |
| Visual grounding | 0.4042 | 0.4042 |

219 of 317 superscript checks pass. Content falls because the benchmark's content scorer deletes
`<sup>` text where its ground truth keeps the same marks inline; the owner took that trade.

### After `whitespace-tracks-v4` (2026-10-04)

The tracks rule runs again inside each of a page's columns where one is prose. All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6613 | 0.6613 |
| Semantic formatting | 0.4060 | 0.4060 |
| Tables | 0.3319 | 0.3224 |
| Charts | 0.0031 | 0.0031 |
| Visual grounding | 0.4045 | 0.4042 |

### After decision #41 (2026-10-04)

A line a page sets apart at its top or bottom edge carries `furniture` (`margin-bands-v1`), and the
adapter labels its item `Page-header` or `Page-footer`. All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6627 | 0.6627 |
| Semantic formatting | 0.4061 | 0.4061 |
| Tables | 0.3319 | 0.3319 |
| Charts | 0.0031 | 0.0031 |
| Visual grounding | 0.4490 | 0.4045 |

The ground truth labels 902 of the 14,986 visual-grounding elements `Page-header` or
`Page-footer`; before the rule every one the engine found was labelled `Text`. Visual grounding
is the mean over 500 pages, and the 46 the engine produces no output for — 42 images and four
refused PDFs — count as zero: ParseBench's runner zero-pads a page with no output and skips only a
page whose output holds no layout data.

The rule was first measured as a relabelling of the adapter's items in a copy of the adapter —
bands of items split wherever the gap is a given share of the page's median span height — and then
written into the engine with the best variant's clauses:

| variant | `layout_element_rule_pass_rate` |
| --- | ---: |
| none | 0.4045 |
| margin 12%, gap 0.6 line, no line taller than 1.15 body lines, images block | 0.4329 |
| the same, margin 8% / 10% / 15% | 0.4323 / 0.4337 / 0.4320 |
| the same, gap 0.4 / 1.0 line | 0.4331 / 0.4324 |
| the same, at most 2 / 3 lines in the band | 0.4205 / 0.4236 |
| the same, no line taller than the body line | 0.4259 |
| the same, no line taller than 1.5 body lines | 0.4394 |
| the same, no size clause | 0.4415 |
| margin 10%, gap 0.5 line, no size clause, images block | 0.4444 |
| the same, margin 8% / 12% | 0.4434 / 0.4419 |
| the same, the band at most 60 / 100 / 160 / 250 characters | 0.4305 / 0.4365 / 0.4407 / 0.4443 |
| margin 10%, gap 0.5 line, no size clause, images in the band allowed | 0.4457 |
| **margin 10%, gap 0.5 line, no size clause, images not counted at all** | **0.4491** |
| that, and tables not counted either | 0.4480 |
| that, and an `/Artifact` run inside the outer 15% is furniture | 0.4497 |

Written into the engine, `margin-bands-v1` scores **0.4490**. The `/Artifact` clause was left
out for +0.0006.

**Measured and not used: furniture out of the Markdown body.** ParseBench reads a page's header
and footer from structured fields where a provider fills them, and its content ground truth
mostly leaves furniture out of the body — 53 of its 585 header and footer strings are mostly in
their page's bag of words. Moving every Markdown block whose runs are all furniture into those
fields still cost content faithfulness 0.6627 -> 0.6465, so the Markdown keeps its furniture, as
the engine's rule is anyway.

### After `line-units-v2` (2026-10-04)

The ground truth's elements are paragraphs, list items and the separate pieces of a key-value line;
a unit that covers several of them matches none, and 1,831 of the `Text` elements visual grounding
failed under `line-units-v1` had a predicted item covering them that was more than five times
their size. The cut was first measured as a split of the adapter's items in a copy of the adapter
— each run of consecutive `ground` elements sharing a unit cut before an element when the element
before it ends short of the run's widest one — and then written into the engine:

| split (adapter) | `layout_element_rule_pass_rate` |
| --- | ---: |
| none | 0.4490 |
| ending 2 / 4 / 6 / 8 line heights short | 0.4416 / 0.4664 / 0.4676 / 0.4697 |
| **ending 10 line heights short** | **0.4711** |
| ending 12 / 16 / 24 line heights short | 0.4698 / 0.4636 / 0.4589 |
| ending 10% / 20% / 30% / 40% / 50% of the width short | 0.4309 / 0.4572 / 0.4637 / 0.4679 / 0.4680 |
| a line indented a line height opening one, alone / with the 4-height cut | 0.4507 / 0.4628 |
| 10 line heights, body units only | 0.4662 |
| 10 line heights, cuts between lines only | 0.4603 |
| 10 line heights, cuts between pieces of one line only | 0.4669 |
| every piece of a line its own unit | 0.4554 |

In the engine a line's pieces are its runs cut at a gap wider than the line's height, the stand-in
for the adapter's `ground` elements, and the cut reads the pieces of one unit that follow each
other in reading order — a unit whose lines cross a page's undivided columns is cut per column, as
the adapter's grouping was. A piece gap of one line height scored 0.4676, above a half (0.4607),
seven tenths (0.4656), one and a half (0.4652) and two (0.4639). Reading the unit's widest piece
over the whole page instead of per stretch scored 0.4469: on a three-column page the cut never
divided into regions, every line of the first column read as ending short of the third column's
edge.

All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6627 | 0.6627 |
| Semantic formatting | 0.4063 | 0.4061 |
| Tables | 0.3319 | 0.3319 |
| Charts | 0.0031 | 0.0031 |
| Visual grounding | 0.4676 | 0.4490 |

### After `whitespace-tracks-v5` (2026-10-04)

A header set on several lines above a table's first row is read as its first row, and a cell ends
at its last inked character (`docs/31-TABLE-TRACKS-SCOPE.md` §11). The table-record half of the
table score matches records by their header, and was zero on 26 pages the other parser scored
above 0.3, most for a header the table had left out. All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6627 | 0.6627 |
| Semantic formatting | 0.4063 | 0.4063 |
| Tables | 0.3533 | 0.3319 |
| Charts | 0.0031 | 0.0031 |
| Visual grounding | 0.4677 | 0.4676 |

### After `whitespace-tracks-v6` (2026-10-04)

A row with cells missing stays a row at the table's pitch, and a first column of bullets is a list
at any width (`docs/31-TABLE-TRACKS-SCOPE.md` §12). All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6627 | 0.6627 |
| Semantic formatting | 0.4063 | 0.4063 |
| Tables | 0.3667 | 0.3533 |
| Charts | 0.0024 | 0.0031 |
| Visual grounding | 0.4680 | 0.4677 |

Charts fall on one page, a table whose region labels wrap onto a second line beside its 2022 rows.

### After decision #42 (2026-10-04)

`bar-labels-v1` reads a bar chart's printed labels back as its table
([`32-CHART-LABELS-SCOPE.md`](../../32-CHART-LABELS-SCOPE.md)). All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6627 | 0.6627 |
| Semantic formatting | 0.4063 | 0.4063 |
| Tables | 0.3667 | 0.3667 |
| Charts | 0.0942 | 0.0024 |
| Visual grounding | 0.4680 | 0.4680 |

Chart tables on 91 of 566 chart pages; 330 of their cells agree with the ground truth, and the 5 that
do not are two errors in it — a value the chart prints as 3.6 named 4.5, a rule that omits which of
two panels it means — and three rules about another chart on the same page. No chart table on any
table, text or layout page but the charts among them.

### After `ruled-rects-v8` (2026-10-04)

Where a page's rectangles as one lattice are refused, each group of them that touch is a candidate
of its own ([`31-TABLE-TRACKS-SCOPE.md`](../../31-TABLE-TRACKS-SCOPE.md) §13). All 2,078 pages:

| Dimension | Score | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6627 | 0.6627 |
| Semantic formatting | 0.4063 | 0.4063 |
| Tables | 0.4092 | 0.3667 |
| Charts | 0.0977 | 0.0942 |
| Visual grounding | 0.4687 | 0.4680 |

46 table pages rise and none falls.

### After decision #43 (2026-10-05)

`declared-font-codes-v3` reads a code the declared map leaves unmapped through what the font itself
states; `Q` restores the text state; `ruled-rects-v9` narrows `-v8`'s unruled-rows clause
([`33-UNMAPPED-CODES-SCOPE.md`](../../33-UNMAPPED-CODES-SCOPE.md)).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6627 |
| Semantic formatting | 0.4080 | 0.4063 |
| Tables | 0.4148 | 0.4092 |
| Charts | 0.0984 | 0.0977 |
| Visual grounding | 0.4695 | 0.4687 |

Omitted runs across the 2,037 documents: 21,600 on 217 -> 10,895 on 143, and one table page refused
whole before is read (GriTS 0 -> 0.1302). Table pages: 10 rise, 2 fall.
Content pages: 22 rise, 11 fall, the largest the Farsi (-0.096) and Hindi (-0.059) pages, whose
characters are now read and are in drawing order, which for those scripts is not reading order.

### After `ruled-rects-v10` (2026-10-05)

A group's grid whose interior lines stop at merged cells is read with them merged, and from here the
adapter takes a table holding a merged cell from the engine's HTML projection, where GFM flattened
the merge ([`31-TABLE-TRACKS-SCOPE.md`](../../31-TABLE-TRACKS-SCOPE.md), the `-v10` paragraph).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4080 | 0.4080 |
| Tables | 0.4502 | 0.4148 |
| Charts | 0.0984 | 0.0984 |
| Visual grounding | 0.4698 | 0.4695 |

Tables 0.4410 with the adapter unchanged; the adapter alone moves the `-v9` engine to 0.4160. 44
table pages rise and 8 fall; no page falls on any other dimension.

### After `ruled-rects-v11` (2026-10-05)

Within one group's rectangles, a rectangle painted twice is read once. Tables 0.4502 -> 0.4578, five
pages up and none down; content 0.6633, formatting 0.4080, charts 0.0984 and visual grounding
0.4698 unchanged. Overall 41.95.

### After `ruled-rects-v12` (2026-10-05)

On a document that declares no author structure, a group's grid leaves out a fill painted over
paint of its own colour and a rule neither of whose ends meets another rectangle.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4080 | 0.4080 |
| Tables | 0.4833 | 0.4578 |
| Charts | 0.1002 | 0.0984 |
| Visual grounding | 0.4707 | 0.4698 |

18 table pages rise and 1 falls (`SERFF_CA` p2069, 1.0 -> 0.94); no page falls on any other
dimension. Overall 42.51.

### After decision #44 (2026-10-05)

`declared-font-codes-v4`: a `/ToUnicode` entry that names no character leaves its codes unmapped
instead of refusing the map. Four refused pages read; charts 0.1002 -> 0.1019, visual grounding
0.4707 -> 0.4708, every other dimension unchanged and no page falling. Overall 42.55.

### After decision #46 (2026-10-05)

`figure-regions-v1` (`docs/34-FIGURE-REGIONS-SCOPE.md`): the paths a page paints, clustered where
they come within 3 points, are figure regions, and the adapter labels each one `Picture`.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4080 | 0.4080 |
| Tables | 0.4833 | 0.4833 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.4824 | 0.4708 |

73 grounding pages rise and 3 fall, each by at most 0.036: a running head drawn as a band of tabs,
which the ground truth boxes as the page header and a region now covers. No page moves on any other
dimension. Overall 42.78.

### After `line-units-v3` (2026-10-05)

After decision #46, 1,274 of the `Text` elements visual grounding failed still had a predicted item
more than half as tall again as they were: a contents list, a column of labels, a list set without
markers, whose lines end short of their unit's widest by a word or two — too little for `-v2`'s ten
line heights. The cut was measured first as a split of the adapter's items, where a line opens a new
item when its first word, with its space, would have fit some number of times over between where the
line above ends and the item's right edge:

| first word must fit | `layout_element_rule_pass_rate` |
| --- | ---: |
| not cut (`-v2`) | 0.4708 |
| once / **twice** / three / four / six times over | 0.4748 / **0.4822** / 0.4798 / 0.4780 / 0.4743 |

In the engine, cutting every unit scored 0.4816 but parted two-line headings, which the projections
join: formatting 0.4080 -> 0.4072 on three pages. Cutting body text only scored 0.4817 with content
and formatting unchanged on every page, and is what ships.

**Measured and not used: the same test the other way.** A line whose first word would *not* have
fit, read as wrapped by its measure, joining the unit above across up to one line height of space
(half is `-v1`'s): 0.4817 -> 0.4771, and across one and a half 0.4699. Separate paragraphs whose
last line happens to run full joined, and lost more than wrapped lines gained.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4080 | 0.4080 |
| Tables | 0.4833 | 0.4833 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.4933 | 0.4824 |

72 grounding pages rise and 49 fall: 332 `Text` elements pass that did not and 87 no longer do.
46 of the 49 lose one or two, mostly where the ground truth keeps a list's items as one element and
the cut parts them — the worst `CSR-2024-25-Full-Report` p59, 0.59 -> 0.46. No page moves on
any other dimension, and opendataloader-bench's 200 Markdown files are byte-identical. Overall
43.00.

### After `page-observations-v4` (2026-10-05)

A run's bold and italic are also read from the program its font embeds — `OS/2`'s weight class from
600, or `head`'s macStyle where there is no `OS/2`, and the italic or oblique bit — and a run painted
fill-then-stroke (rendering mode 2 or 6) is bold. Of the 960 `is_bold` rules whose text we drew but
did not mark, 54 sat in fonts whose dictionary names no weight and whose program states one, and
`text_simple__delinea` set all 26 of its bold labels fill-then-stroke.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4296 | 0.4080 |
| Tables | 0.4833 | 0.4833 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.4933 | 0.4933 |

17 formatting documents rise and none falls — the program alone 13, fill-then-stroke 5, measured
apart — and no page moves on any other dimension. opendataloader-bench: MHS 0.5467 -> 0.5481, NID
0.8839 -> 0.8838 across 12 documents, each a newly bold run gaining `**` or a newly bold title `#`,
TEDS unchanged. The heading bounds of `docs/28-HEADINGS-SCOPE.md` §7.5 are where the owner accepted
them. Overall 43.43.

### After `type-size-v5` (decision #47, 2026-10-06)

On the visual-grounding pages, 1,416 of the elements the ground truth calls section headers were
read as text, and 850 of them are bold lines at the body size set tight over the lines they name —
`Contact person:`, `Loan terms`, `Audience` — which `-v4`'s bold clause leaves alone because they do
not stand apart. `-v5` reads them as **labels**: level 7, below every ranked level, written by both
projections as the bold lines they already were, never with `#`. The adapter labels them
`Section-header`, as it does every inferred heading.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4296 | 0.4296 |
| Tables | 0.4833 | 0.4833 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.5115 | 0.4933 |

111 grounding pages rise and 38 fall; 32 of the 38 lose `Section-header` precision where a bold
line the ground truth keeps as text — a lead-in, a list item's label — is now a label, the worst
`614c519a-7fe2-4057-a7eb-8a7614148818` p30, 0.73 -> 0.53. No page moves on any other dimension,
and opendataloader-bench's 200 Markdown files are byte-identical.

**Measured and not used: a label's lines joined**, as a heading's are. A label set on two lines is
two labels — `ARTICLE 11` over `DIVIDEND EQUIVALENTS` — and joining them moved formatting 0.4296 ->
0.4303 with 8 documents down (`text_simple__dividend` 1.0 -> 0.5) and content 0.6633 -> 0.6632
with 19 pages down. Overall 43.79.

### After a space in a font of spaces alone carries its line (2026-10-06)

Word sets a document's spaces in fonts of their own, and a font with no glyph measured gave such a
space no reach, so the Markdown and HTML ended the line at it: `text_simple__sunnyslope` read
`**IN**`, `**PERSON**`, `**PUBLIC**` one block each. A font that draws only spaces is now measured
by them (folded into `markdown-blocks-v12`). Semantic formatting 0.4296 -> 0.4310, 3 documents up
and none down; content faithfulness 0.6633 unchanged, 6 pages up and 4 down by at most 0.0063,
where a line now read whole is a sentence the ground truth splits; every other dimension unchanged
on every page, and opendataloader-bench byte-identical. Overall 43.82.

### After a label its column wrapped is one line (decision #48, 2026-10-06)

Of the `is_title` rules we failed, 230 hold text we drew but broke across lines, about a hundred of
them written as bold lines one under the other: `**NOTICE OF PUBLIC**` over `**HEARING**` in a
newspaper's narrow column. Both projections now join a label's next line where its column wrapped it: the line
starts where the first does, opens with no number or marker, follows a first line that ends no
sentence, and its first word would not have fit after the first within the text the label heads —
the body lines below it that start where it starts, measured by the widest of them.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4347 | 0.4310 |
| Tables | 0.4833 | 0.4833 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.5115 | 0.5115 |

8 formatting documents rise and none falls; 8 content pages fall by at most 0.001, each a joined
title the scorer reads as a sentence its ground truth lacks. opendataloader-bench byte-identical.
Overall 43.89.

**Measured and not used, on the way there.** Cutting the layout units where a label's author broke
it, so the projections could join a unit's lines, cost visual grounding 0.0010 on 7 pages: the
ground truth boxes a title set on several lines as one element however it was broken. Measuring a
label by the lines around it rather than the text it heads joined two centred lines of
`text_simple__edited` and cost that document 0.11. **Emphasis in table cells** (decision #48):
`**` in GFM cells cost opendataloader-bench TEDS 0.4949 -> 0.4777 on 14 documents, whose scorer
keeps the markers in a cell's text, and `<strong>` in HTML cells alone moved nothing — this adapter
reads pipe tables converted to HTML and the HTML projection's table only where it holds a merged
cell. **A body size read from running text** (decision #48) — the size most lines of 40 characters
or more are set in, where the larger common size sets fewer than ten of them — kept the heading
bounds where they are and moved visual grounding 0.5115 -> 0.5152, but 17 pages fell, the worst
`Apple_Environmental_Progress_Report_2025` p5 0.68 -> 0.30, and opendataloader-bench MHS fell 0.5481
-> 0.5438 where chart values and table lines became headings. Taking that size wherever ten such
lines exist read 500 false headings on `nist-sp-800-218` and 4,480 on `nist-sp-800-53Ar5`.

### After a panel holding prose joins no drawing (2026-10-06)

After decision #48, 433 of the pictures visual grounding failed were covered by a picture box more
than five times their size, three in four of those boxes a figure region a shaded panel had
joined to every drawing set on it. `figure-regions-v1` now leaves out of its clusters a path whose
box holds a line of prose, as its cluster clause already reads one — more than 60 characters
(`docs/34-FIGURE-REGIONS-SCOPE.md` §6.1, where the sweep of that length is).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4347 | 0.4347 |
| Tables | 0.4833 | 0.4833 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.5156 | 0.5115 |

25 grounding pages rise and 3 fall, by at most 0.045, each a Ford report page whose running head,
drawn as a band, is now a region of its own. No page moves on any other dimension, and
opendataloader-bench's 200 Markdown files are byte-identical. Overall 43.98.

**Measured and not used, on tables.** Letting `whitespace-tracks-v6`'s header band reach two
pitches above a table's first row, where it reaches one and a half, moved tables 0.4833 -> 0.4840
with 3 pages down (`Apple 10-k` p28 1.0 -> 0.77) and cost opendataloader-bench NID on the two
documents whose TEDS it raised: a header set on several lines was taken in part. Of the tables
TRM scored with no column matched — 0.287 of the 0.517 the dimension loses — 146 hold their
header text in the lines just above our table, which starts at its first data row.

### After half a thousandth of the page (2026-10-07)

269 of the pictures visual grounding still failed had no region near them, 216 under 0.2% of the
page: line-drawn icons beside lines of text. `figure-regions-v1`'s floor on a region's area moves
from a thousandth of the page to half of one (`docs/34-FIGURE-REGIONS-SCOPE.md` §6.2, where the
sweep is). Visual grounding 0.5156 -> 0.5174, 17 pages up and none down; content, formatting,
tables and charts unchanged on every page; opendataloader-bench byte-identical. Overall 44.01.

### After a header set off its columns (`whitespace-tracks-v7`, decision #49, 2026-10-07)

Of the tables TRM scored with no column matched, 146 hold their header text in the lines just
above our table. `whitespace-tracks-v7` gives each cell of a header line of two cells or more, where
they do not each sit on a track, the track whose centre is nearest its own, in order; lets the
band's first line stand two rows' pitch above the table; and builds the table without a header it
cannot be built with (`docs/31-TABLE-TRACKS-SCOPE.md` §14).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4347 | 0.4347 |
| Tables | 0.4932 | 0.4833 |
| Charts | 0.1035 | 0.1019 |
| Visual grounding | 0.5173 | 0.5174 |

48 table pages rise and 5 fall by at most 0.082, four of them stacked headers now split in two; one
chart page rises 0 -> 0.9; content and formatting move on no page. Two grounding pages fall, where a
caption (`Table II:`) and a key figure set over a table (`2023: 85%`) were taken into headers — a
breach of the tracks scope's bound 3 the owner accepted (decision #49). opendataloader-bench TEDS
0.4949 -> 0.5060, 5 documents up and none down; NID 0.8838 -> 0.8839, one document down 0.8632 ->
0.8283. Overall 44.24.

### After a bold body set as display type (`type-size-v6`, decision #50, 2026-10-07)

Of the elements visual grounding calls section headers and we read as text, 276 sit on documents
whose body em is a size set mostly bold — a statement opening the page, set larger than its text —
which closed the bold clause's guard, and 149 more are bold lines set below such a body em.
`type-size-v6` reads bold lines against the largest common size not set mostly bold there; the size
clause still reads the body em (`docs/measurements/headings/README.md` §12).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4356 | 0.4347 |
| Tables | 0.4932 | 0.4932 |
| Charts | 0.1035 | 0.1035 |
| Visual grounding | 0.5252 | 0.5173 |

32 grounding pages rise and 5 fall, the falls slide decks whose bold names over their job titles now
read as labels; of the elements whose verdict moved, 281 section headers are now read right and 39
text elements are read as headings they are not. 3 formatting documents rise and none falls; 3
content pages fall by at most 0.002. opendataloader-bench MHS 0.5481 -> 0.5454, one document. The
owner shipped it with that trade (decision #50). Overall 44.42.

### After two numbers closer than an em are two cells (`whitespace-tracks-v8`, decision #51, 2026-10-07)

Our tables recover less of the ground truth's content than they add that is wrong — recall 0.76
against precision 0.86 — and `whitespace-tracks-v7`'s stop inside the real table on four clauses
of their growth. `whitespace-tracks-v8` splits a cell holding two numbers set closer than an em
where that lets the line join as a row (`docs/31-TABLE-TRACKS-SCOPE.md` §15).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4356 | 0.4356 |
| Tables | 0.4954 | 0.4932 |
| Charts | 0.1035 | 0.1035 |
| Visual grounding | 0.5252 | 0.5252 |

10 table pages rise and none falls, `Goldman Sachs 10-k` p74 0.14 -> 0.58; no page moves on any
other dimension. opendataloader-bench TEDS 0.5060 -> 0.5114 and NID 0.8839 -> 0.8843, none down.
Overall 44.46.

### After a wider line and a tall row (`whitespace-tracks-v9`, decision #52, 2026-10-07)

`whitespace-tracks-v8`'s tables still stop inside the real table at a line with more cells than the
table has tracks — the table opened on a row leaving a column empty — and at a gap past three and a
half pitches, where a row's cell wraps over several lines. `whitespace-tracks-v9` re-tracks the
table at such a line under rows of data, and lets a row its wrapped cell made tall reach past the
pitch (`docs/31-TABLE-TRACKS-SCOPE.md` §16).

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.6633 |
| Semantic formatting | 0.4356 | 0.4356 |
| Tables | 0.5010 | 0.4954 |
| Charts | 0.1035 | 0.1035 |
| Visual grounding | 0.5253 | 0.5252 |

17 table pages rise and 2 fall by at most 0.031; one grounding page rises and none falls; no page
moves on any other dimension. opendataloader-bench TEDS 0.5114 -> 0.5122, none down. Overall 44.57.

### After a cell of several numbers (`whitespace-tracks-v10`, decision #53, 2026-10-07)

A row of data whose cell runs three or four figures a word space apart kept a table from starting
above it, and its header from being found. `whitespace-tracks-v10` splits such a cell into its numbers
where each sits on a track of its own (`docs/31-TABLE-TRACKS-SCOPE.md` §17). Tables 0.5010 -> 0.5026,
2 pages up and none down; every other dimension unchanged on every page; opendataloader-bench
unchanged. Overall 44.61.

### After a right-to-left line read right to left (`right-to-left-lines-v1`, decision #54, 2026-10-08)

The multilingual pages in Hebrew, Arabic and Farsi scored 0.26 to 0.31 on content: their producers
draw each glyph where it sits, left to right, so every word came out reversed and every line in the
order its runs were drawn. `right-to-left-lines-v1` reads a line whose right-to-left letters
outnumber its left-to-right ones right to left, glyph by glyph — each code's characters whole, so a
lam-alef ligature keeps its two letters' order — with numbers and Latin inside left to right, and
each run carries its reading beside the text it was drawn with; the projections read the reading.

| Dimension | After | Before |
| --- | ---: | ---: |
| Content faithfulness | 0.6657 | 0.6633 |
| Semantic formatting | 0.4365 | 0.4356 |
| Tables | 0.5026 | 0.5026 |
| Charts | 0.1035 | 0.1035 |
| Visual grounding | 0.5253 | 0.5253 |

3 content pages rise and none falls: `text_multilang__arabic` 0.29 -> 0.89, `text_multilang__hebrew`
0.31 -> 0.86, `text_multilang__farsi` 0.26 -> 0.32, whose fonts map several glyphs to Latin letters
and to doubled letters, so its lines often hold more left-to-right letters than right-to-left and
stay as drawn. One formatting page rises, `text_multilang__hebrew` 0.00 -> 0.42; no page moves on
tables, charts or grounding. opendataloader-bench and the tree-stripped gate documents carry no
right-to-left text and are unchanged byte for byte. Overall 44.61 -> 44.67.

Measured before it, at the adapter and not used: reversing each line's characters after the
projection — content 0.6648, Arabic only 0.46, because a ligature's two letters were reversed with
the rest; and moving an Indic vowel sign drawn before its consonant after it, which gained 0.005 on
one Hindi page where the sign was unambiguous and broke a Bengali page already in logical order
(0.71 -> 0.33) where it was moved regardless.

### After leaders, the rows above and a long stub (`whitespace-tracks-v11`, decision #55, 2026-10-08)

The header band stopped most often at a row of data the table had not taken — a statement's dot
leaders running up to its first figure, or a row leaving a column empty — and at a header line whose
stub heading ran past thirty characters. `whitespace-tracks-v11` ends a cell after dot leaders,
joins a lone currency sign to its figure, takes the rows of data above a table, lets a stub heading
run long, and reads titles drawn with leaders over page numbers as contents
(`docs/31-TABLE-TRACKS-SCOPE.md` §18). Tables 0.5026 -> 0.5066, 22 pages up and none down; visual
grounding 0.5253 -> 0.5258, 3 up and none down; content, formatting and charts unchanged on every
page; opendataloader-bench unchanged. Overall 44.67 -> 44.76.

### After a label column written as one block (`whitespace-tracks-v12`, decision #56, 2026-10-08)

On seven table pages scoring nothing the tracks rule had grown the whole table and refused it at its
last guard: the label column was written as one block before the values row by row, the order of a
table beside a column of text. `whitespace-tracks-v12` lets the guard stand aside where the first
column alone is the one group and holds a short label in every row (`docs/31-TABLE-TRACKS-SCOPE.md`
§19). Tables 0.5066 -> 0.5128, 7 pages up and none down; every other dimension unchanged on every
page; opendataloader-bench TEDS 0.5122 -> 0.5339, one document's table back. Overall 44.76 -> 44.89.

### After bars drawn twice-cornered, groups from their gaps and colours by category (`bar-labels-v2`, decision #57, 2026-10-09)

Of the printed chart data points whose value the engine reads as text, most failed on charts the
bar-label rule read no table from. `bar-labels-v2` reads bars drawn with every corner named twice,
groups whose colours repeat in no period (a bar left out, a group highlighted), bars coloured one
per category, and a legend set beyond the categories (`docs/32-CHART-LABELS-SCOPE.md` §7). Charts
0.1035 -> 0.1179, 13 pages up and none down; every other dimension unchanged on every page.
Overall 44.89 -> 45.17.

### After a header's last line read as the first row (`whitespace-tracks-v13`, decision #58, 2026-10-09)

A header set on two lines whose lower line has a cell on every column opened the table itself, and
the band took only the upper line: the header row read `Number of` where the column is `Number of
Shares`. `whitespace-tracks-v13` joins such a first row to the header row
(`docs/31-TABLE-TRACKS-SCOPE.md` §20). Tables 0.5128 -> 0.5280, 31 pages up and one down; every
other dimension unchanged on every page; opendataloader-bench unchanged. Overall 45.17 -> 45.48.

### After the Adobe Glyph List (decision #59, 2026-10-10)

Runs whose font names a glyph the profile's table did not hold — `/minus`, `/ellipsis`, Slovak
`/ccaron` — were dropped. The Adobe Glyph List is vendored and reads them
(`docs/33-UNMAPPED-CODES-SCOPE.md` §6): runs omitted 10,895 on 143 documents -> 10,019 on 73.
Content faithfulness 0.6657 -> 0.6669, visual grounding 0.5258 -> 0.5266, charts 0.1179 -> 0.1184;
tables and formatting unchanged. Overall 45.48 -> 45.53.

### After two flows of numbers read side by side (`whitespace-tracks-v14`, decision #60, 2026-10-10)

Two tables set side by side — a schedule's two panels, a two-column page's tables level with each
other — written one panel and then the other were refused as two flows, the shape of a table beside
a column of text. `whitespace-tracks-v14` reads each flow on its own where each holds a column of
numbers and none is text (`docs/31-TABLE-TRACKS-SCOPE.md` §21). Tables 0.5280 -> 0.5312, 6 pages up
and none down; visual grounding 0.5266 -> 0.5268; every other dimension unchanged on every page;
opendataloader-bench unchanged. Overall 45.53 -> 45.60.

### After a run's advance read along the declared x (decision #61, 2026-10-11)

Told "Content on readable pages", the text pages were scored beside PyMuPDF's plain text, run out
of tree as an instrument (decision #14), page by page. Two pages it read and this engine did not —
`text_simple__slide2` and `text_ocr__bg` — are `/Rotate 90` pages whose producer turns every run
upright with its text matrix: each run advanced 0, and the Markdown projection set each run as a
paragraph of its own. The advance is now the run's travel along the declared x
(`docs/22-WORD-BOXES-SCOPE.md`, decision #61). Content faithfulness 0.6669 -> 0.6696, semantic
formatting 0.4365 -> 0.4402, tables 0.5312 -> 0.5313; no page down on any dimension;
opendataloader-bench unchanged. Overall 45.60 -> 45.73.

### After a script joins the line it is set in (`line-units-v4`, decision #63, 2026-10-11)

Told "Grounding: text blocks", the `Text` elements visual grounding could not place were counted
by why (`target`-side census, the scorer's own per-element numbers): nothing of ours over them
0.052, a bigger item of ours covering them 0.041 (figure regions 0.030 — every variant that dropped
the ones holding text scored lower — tables 0.013, text units 0.013), one item barely over them
0.024, several smaller ones 0.014. On mathematics and chemistry each subscript and superscript was
a unit of its own. `line-units-v4` joins a script to the line it is set in. Visual grounding
0.5268 -> 0.5316, 40 pages up and 10 down (the owner took the trade); content one page down by
0.00003; every other dimension unchanged on every page; opendataloader-bench unchanged. Overall
45.73 -> 45.82.

### After a page states the box a viewer shows (decision #62, 2026-10-11)

Seven visual-grounding pages crop: their `/CropBox` is narrower than their media box, the benchmark
draws and labels the crop box, and this engine's coordinates are in the media box's frame by design.
Each page record now states `visible`, the crop box in the declared system, and the adapter places
its items on it. Visual grounding 0.5316 -> 0.5367, the 7 pages up and none down; every other
dimension unchanged. Overall 45.82 -> 45.92.

### After a weight heavier than the text read as bold (`type-size-v7`, decision #64, 2026-10-11)

Told "Headings on layout pages", the section headers visual grounding finds and we read as text
were counted by what sets them apart: 320 are short lines set within 15% of the text's size in no
bold face, 258 of those in a face other than the text's, commonest the text's own family a weight
heavier (`NewFrank-Medium` over `NewFrank-Light`). `type-size-v7` reads each font's weight class and
counts a run a class or more heavier than its page's text as bold for the heading rule, labels
included (`docs/measurements/headings/README.md` §13). Visual grounding 0.5367 -> 0.5435, 43 pages
up and 18 down (the owner took the trade: 49 text elements now read as headings, navigation tabs
and lists set a weight up); content one page down by 0.028, a heading broken after a hyphen now
two `#` lines; formatting, tables and charts unchanged on every page; opendataloader-bench MHS
0.5457 -> 0.5854, 6 documents up and none down. Overall 45.92 -> 46.06. Not used: the weight read
only where a line stands apart, labels as before (0.5389, 19 pages up and 4 down).

### After a heading's number joined to its title (decision #65, 2026-10-12)

Told "Formatting titles", the title checks on the formatting pages were read for why each fails:
of 1,872, 812 pass and 247 sit on pages with no text layer; a title's words absent or changed in
our text (an OCR layer's errors, a curly apostrophe, a broken ligature) 311; a title set on several
lines whose first only is a heading or a bold line, about 150; a section number and its title set
a tab apart, written as two headings; and 235 on 75 tagged documents whose tree tags every title
`/P` and declares no heading. Both projections now join a heading's title to its section number
across the gap where the heading rule read one line, and write a label's lines in one block.
Semantic formatting 0.4402 -> 0.4512, 20 documents up and none down; content 0.6695 -> 0.6700, 9
up and none down; every other dimension unchanged on every page; opendataloader-bench MHS 0.5854
-> 0.5872. Overall 46.06 -> 46.29. Not used: the join across any gap (46.40; it welds two
columns' headings, folios, column headers and twice-drawn text); headings inferred on tagged
documents that declare none (formatting 0.4573, content down on 15 pages — the owner's call, since
it changes documents that declare structure).

### After a line chart's printed values (`point-labels-v1`, `bar-labels-v3`, decision #66, 2026-10-13)

Told "Charts, round 3", the chart points that print their value and fail were read for why: 284 of
846 on pages whose bars the bar rule never found, 328 on pages whose bars it refused, 234 on pages
whose table missed them, each reason small; the largest coherent piece was a line chart printing a
number at each point. `point-labels-v1` reads one (`docs/32-CHART-LABELS-SCOPE.md` §9), and
`bar-labels-v3` names a progress bar's one numbered series by the title over it (§8). Charts
0.1184 -> 0.1248, 7 pages up and none down; every other dimension unchanged on every page;
opendataloader-bench NID 0.8841 -> 0.8838 and TEDS 0.5339 -> 0.5301 on one document whose line reads
right as a table (the owner took the trade). Overall 46.29 -> 46.42. Round 1's line prototype,
re-run here, measured 0.1192: it judged each series alone and so lost the numbers of lines running
close, and refused a tenor row's `3M` as a number.

### After a line read where the reading order reads it (`line-units-v5`, decision #67, 2026-10-13)

Told "Grounding: text, round 2", the `Text` elements visual grounding could not place were counted
again by how our items sit on them: split across several of ours 811 (0.051 of the score), nothing
of ours over them 520 (0.032 — pages drawn as images, figure regions, panels read as ruled tables),
one item of ours far bigger 483 (0.023), ours covering under half 182 (0.015). The largest split
was a page the reading order did not divide, where a line was every run on its baseline across the
page: a newspaper paragraph sharing its baselines with a byline in the next column was read where
the byline was, and fell into five units. `line-units-v5` reads as a line the pieces of one line
that follow each other in reading order, and reads a space where two runs stand a tenth of the
line's height apart, so a bullet set apart by its position opens an item and a first word is a word
on pages that set their words by moving the pen. Visual grounding 0.5435 -> 0.5575, 119 pages up
and 8 down (the owner took the trade); content one page up by 0.0003; every other dimension
unchanged on every page; opendataloader-bench and the gate documents' tables unchanged. Overall
46.42 -> 46.70. The split bucket fell 811 -> 646 with the lines alone. Measured on the way: the
lines alone 0.5546 (109 up, 12 down — the falls were cuts the across-columns edge made by
accident); with the bullets 0.5561; the space read for the first word too, at a quarter of the
line's height 0.5564, at a tenth 0.5575, at a twentieth 0.5577 (2 pages up and 2 down against a
tenth, so the tenth stands). Left as found: newspaper pages whose columns the stream draws row by
row (the reading order reads across them), and Markdown that writes such a page one word per
block — 71 content pages are mostly one-word blocks.

### After words set apart by position joined into one block (decision #68, 2026-10-15)

Told "Markdown: one word per block", the pages whose Markdown is mostly one-word blocks were read
for why: 71 of the text pages, OCR text layers among them, where the page places each word by
moving its pen and draws no space. Of the 8,587 places two runs of one line fell into two blocks
there, 5,549 had both runs in one layout unit with a word gap between them; 967 were gaps wider
than a line, 956 runs drawn over the one before, 610 runs of two units. Both projections now join
the runs of one unit on one line across the gap, with one space (`layout-unit-gap-joins-v1`).
Semantic formatting 0.4512 -> 0.4572, 15 documents up and none down; content 0.6700 -> 0.6696, 28
up and 87 down (the owner took the trade: content's sentence check reads only lines of two words
or more, so one-word blocks hid OCR misreads and inline mathematics from it); tables, charts and
visual grounding unchanged on every page; opendataloader-bench MHS 0.5872 -> 0.5895. Overall
46.70 -> 46.81. Not used: the join bounded at the line's height (formatting 0.4569; it parts a
10-K's `Item 1.` from `Business`, a tab apart).

### After a header read further off its table (`whitespace-tracks-v15`, decision #69, 2026-10-15)

Told "Tables: census round", the table loss (0.469 of the score's 1) was read by where it falls,
with the scorer's own per-table details: the table-record score's share is 0.246, and of the 518
ground-truth tables whose records fail, the column headers sit just above our table for 148 (0.090),
in our first row with the records failing anyway for 108 (0.048), and 127 are paired with no table
of ours. Marking header rows `<th>` in the adapter alone moved tables by -0.0015 (row 0) and +0.0037
(leading spanned rows too). Of the 148, 131 are `whitespace-tracks` tables whose header band
stopped, for many small reasons; told "Header too far above", its first line's reach was measured:
three ems, two and a half rows' pitch, where two were allowed. Tables 0.5313 -> 0.5326, 5 pages up
and none down; every other dimension unchanged on every page; opendataloader-bench unchanged.
Overall 46.81 -> 46.83. Not used: three and four rows' pitch (0.5324 each, a title read as a
header), and any band line reaching four ems where it sits one cell to a track (0.5312).

### After a table open at its sides read as ruled (`ruled-rects-v13`, decision #70, 2026-10-16)

Every dimension's remaining loss was read again by cause. Formatting: of the bold and title rules
failed on pages with text, the font names bold for 357 and 250, and those split into many shapes
(punctuation the font draws regular after a bold word, a bold phrase broken across blocks, bold in
table cells); content: order rules fail when their anchors are misread, not misordered (2,743 of
2,877), and the worst text pages declare a `/ToUnicode` that maps glyph ids to the wrong letters,
in fonts that carry no `cmap` and no glyph names; grounding: Section and Text confused 0.069 each way
evenly. Tables: PyMuPDF's line-based table finder, scored in our place out of tree (decision #14),
beats ours on 96 pages, and the better of the two on each page reads 0.611. 0.045 of that gap is on
pages whose ruled grid was refused for a line not traced end to end, 0.023 of it an outer one. Told
"Tables: open-sided grids": an outer line of a grid traced whole is drawn across a row whose rules
above and below both run out to it, and a rule other rectangles end on is no underline. Tables
0.5326 -> 0.5405, 7 pages up and none down; every other dimension unchanged on every page;
opendataloader-bench unchanged. Overall 46.83 -> 46.99. Not used: closing the sides of a grid read
through merged cells as well (no page more, opendataloader-bench `01030000000064` TEDS 1.00 ->
0.81), and the grid's bounding box as its sides (0.5398, 6 pages up and 5 down).

### After a run placed by its first glyph (decision #71, 2026-10-16)

Among the ruled tables accepted after #70 that the out-of-tree line-based finder reads better,
several had our shape and half the score: a page that widens the space before a word (`Tw`) draws
` None` from where the figure before it ended, its origin in the column before its ink. Told
"Tables: place a run by its first glyph", the ruled rule places a run by its first inked
character, from the pen's own advances; folded into `ruled-rects-v13`. Tables 0.5405 -> 0.5444, 9
pages up and none down; every other dimension unchanged on every page; opendataloader-bench
unchanged. Overall 46.99 -> 47.07. Not used: the whitespace rule's runs placed the same way
(+0.0005, 4 pages down).

### After rows ruled in single strokes (`stroke-ruled-v2`, decision #72, 2026-10-16)

Three Goldman Sachs 10-K pages draw a table as a box of single full-width strokes divided by
vertical ones, and `stroke-ruled-v1` formed no band from rows of one segment. Told "Tables: stroked
rows split by vertical strokes", a row of one stroke is a rule-row, a band's columns are also the
vertical strokes across it, and such a band is refused where it holds a form widget, unruled rows,
or rows of text running on past its rules. Tables 0.5444 -> 0.5506, 4 pages up and none down;
visual grounding 0.5575 -> 0.5577; content, formatting and charts unchanged on every page;
opendataloader-bench NID and TEDS up on one document each, one heading score 0.5027 -> 0.5025
(the trade taken). Overall 47.07 -> 47.20.

### After a box behind the page and stacked tables (decision #73, 2026-10-16)

A box painted behind the whole page counted, in the page's bands, as covering every face of a
grid drawn in rules, and the page's lattice read tables stacked on their own columns as one grid.
Told "Tables: page box + stacked tables", a rectangle spanning every face of the grid is its border
however far past it it reaches, and a page lattice whose rows break at whitespace into blocks drawn
on different column lines is read group by group. Tables 0.5506 -> 0.5555, 6 pages up and 1 down
(`Coca Cola 10-k` p107 0.095 -> 0.082, the trade taken); every other dimension unchanged on every
page; opendataloader-bench NID 0.8849 -> 0.8874 and TEDS 0.5426 -> 0.5784, three documents up each
and none down. Overall 47.20 -> 47.30.

### After a header set above a ruled table's top rule (decision #74, 2026-10-16)

Ruled tables whose rules start under their header began at their first row of values. Told
"Tables: header above a ruled grid", the lines just above the top rule are the table's first row
where the grid's own first row is no header and the lines are one: set apart, within the table's
columns word by word, labelling two or more. Tables 0.5555 -> 0.5601, 8 pages up and none down;
every other dimension unchanged on every page; opendataloader-bench NID and TEDS up on two
documents each and none down. Overall 47.30 -> 47.39.

### After rules up to three points thick read as one line (decision #75, 2026-10-17)

A rule thicker than the lattice tolerance bounded a sliver column between its edges, and the grid
was refused. Told "Tables: thick rules + a guard", such a rule is read along its middle, a grid no
cell of which is narrower than it has no columns, and a ruled row's cells sharing three baselines
hold rows the page did not rule. Tables 0.5601 -> 0.5631, 3 pages up and none down; visual
grounding 0.5577 -> 0.5582; content, formatting and charts unchanged on every page;
opendataloader-bench unchanged. Overall 47.39 -> 47.46.

### After a line drawn across no band is no line of the grid (decision #76, 2026-10-17)

Ruled grids were refused for a line they draw across none of their bands, mostly a white fill's
edge, and rate tables for a label set beside the rows it spans. Told "Tables: untraced ruled
lines", such a line is dropped and the grid read again where it keeps its outline and no white
fill on one of its panels drew the line, and a label whose text holds half the rows it spans or
fewer is one cell. Tables 0.5631 -> 0.5682, 6 pages up and none down; content, formatting, visual
grounding and charts unchanged on every page; opendataloader-bench unchanged. Overall 47.46 ->
47.56.

### After a heading holds thirty words at most (decision #77, 2026-10-18)

Ledes and opening statements set in display type read as headings, and the layout ground truth
labels them text. Told "Grounding: headings vs text", a unit of heading lines past thirty words is
read as text; at twenty, four formatting pages fell for decks the benchmark reads as titles. Visual
grounding 0.5582 -> 0.5606, 19 pages up and none down; tables, charts, content and formatting
unchanged on every page; opendataloader-bench NID one up and none down. Overall 47.56 -> 47.61.

### After Adobe's CID-to-Unicode tables (decision #78, 2026-10-18)

Text set in Adobe-Japan1, -CNS1 and -Korea1 CID fonts with no `/ToUnicode` was omitted: 1,265 runs
on seven Japanese report pages, 182 on three CJK text pages, and `text_multilang__korean2` refused.
Told "Vendor Adobe CID tables", Adobe's CID-to-Unicode mappings are vendored
([`vendor/README.md`](../../../vendor/README.md)), and every one of those runs is read. Content
faithfulness 0.6696 -> 0.6716, semantic formatting 0.4572 -> 0.4603 and visual grounding 0.5606 ->
0.5642, 4, 3 and 5 pages up and none down; tables and charts unchanged on every page;
opendataloader-bench unchanged. Overall 47.61 -> 47.78.

### After a chart's gridlines are no table (decision #79, 2026-10-18)

Every table `stroke-ruled-v2` emitted on the chart pages was a chart's grid: empty, or holding piles
of the chart's own printed values. Told "Gridlines are no table", a stroke-ruled grid that holds no
text, or whose every rule but one carries a number set on it beside the grid as a value axis's tick
labels are, is refused (`docs/31-TABLE-TRACKS-SCOPE.md` §30). 25 grids gone on 14 pages; visual
grounding 0.5642 -> 0.5644, one page up and none down; tables, charts, content and formatting
unchanged on every page; opendataloader-bench unchanged. Overall 47.78 -> 47.79.

### After a bold line its column wrapped joins the line above (decision #81, 2026-10-19)

A bold phrase set on two lines was written as two bold blocks, and the bold rule finds a phrase only
inside one span. Told "Bold: reuse #48 wrap test", a line of bold body text continues the bold line
above it where decision #48's test says its column wrapped it, but not under a line set in capitals
or after one ending in a hyphen. Semantic formatting 0.4603 -> 0.4646, 7 pages up and none down;
content faithfulness unchanged at 0.6716, 13 pages down by at most 0.0023, a trade the owner took;
tables, charts and visual grounding unchanged on every page. Overall 47.79 -> 47.87.

### After a run set heavier than its page's text is bold (decision #82, 2026-10-19)

Of the bold rules failing on pages with Markdown, one cause had a single fix: faces a weight class
heavier than their page's text — `Amplitude-Medium`, `McKinseySans-Medium` — which the heading rule
read as bold since decision #64 and the projections did not emphasise. Told "Census: unread bold",
such a run is bold. Semantic formatting 0.4646 -> 0.4732, 8 pages up and none down; charts 0.1248
-> 0.1255, two up; content, tables and visual grounding unchanged on every page. Overall 47.87 ->
48.06; opendataloader-bench NID 0.8876 -> 0.8872 and MHS 0.5895 -> 0.5893, a trade the owner took.

### After a mark set right after a bold run stays in its span (decision #83, 2026-10-19)

Of the bold rules failing under text already set bold, 84 lost only punctuation the page set in its
text's weight — `**PAMPLONA** –`, `**necessary**?` — and the bold rule finds a phrase only inside one
span. Told "Punctuation stays in span", a run of punctuation alone set right after a bold run is
written inside the span, unless a letter or a digit follows it directly. Semantic formatting 0.4732
-> 0.4771, 14 pages up and none down; content faithfulness, tables, charts and visual grounding
unchanged on every page. Overall 48.06 -> 48.14; opendataloader-bench NID 0.8872 -> 0.8874, 7
documents up and none down.

### After one paragraph's runs are one block across its marked-content sequences (decision #84, 2026-10-20)

A Word-style tagged page opens a marked-content sequence at every change of style and every line,
and the projections opened a block at each: a bold phrase was cut from its sentence. Told "Group by
paragraph element", the reader names the element holding each tagged run's block of text and the
runs of one element are one block. Semantic formatting 0.4771 -> 0.4819, 15 pages up and one down;
content faithfulness 0.6716 -> 0.6737, 22 up and 31 down by at most 0.018, a trade the owner took;
tables and charts unchanged on every page, visual grounding 0.5644 -> 0.5670 with two pages up. Overall 48.14 -> 48.33.

### After a tagged title's lines read on (decision #85, 2026-10-20)

A producer that tags each line of a title as an element of its own left the lines apart, and the
bold rule finds a phrase only inside one span. Told "Lowercase continuation", a bold line on a
tagged page continues the bold line above it where both are children of one kind of element and
it opens lowercase, or #81's test says its column wrapped it. Semantic formatting 0.4819 -> 0.4848,
10 pages up and none down; content faithfulness unchanged at 0.6737, 10 pages down by at most
0.006, a trade the owner took; tables, charts and visual grounding unchanged on every page. Overall 48.33 -> 48.38.

### After a chart's panel is no bar (decision #86, 2026-10-20)

The bar rule refuses a category label set on a bar, and took a chart's filled panel for one, so a
chart drawn on a grey panel read no categories at all. Told "Category labels", a rectangle drawn
round all of a chart's bars is its panel, not a bar. Charts 0.1255 -> 0.1276, 3 pages up and none
down; every other group unchanged on every page. Overall 48.38 -> 48.43.

### After a range is a category and a title may sit over any part of its chart (decision #87, 2026-10-21)

The bar rule refused a category that opens with a value, which a survey's brackets of firm size
(`$1.00bn–$5.00bn`) do, and read a chart's title only where it crossed the chart's centre. A range
of two values is now a category, and a title may be set over any part of its chart. Charts 0.1276
-> 0.1297, 5 pages up and 1 down — `US_Professional_Services_Partner_Compensation_Survey_2024` p25,
whose `2024` the scorer now finds in a new range cell, a trade the owner took; visual grounding
0.5670 -> 0.5671, 1 page up and none down; tables, content faithfulness and semantic formatting
unchanged on every page. Overall 48.43 -> 48.47.

### After a bar with rounded ends is a bar and a legend's dot is its swatch (decision #88, 2026-10-21)

The bar rule read only rectangles, so a chart whose bars have rounded ends read no bars, and its
legend swatches only as squares, so a legend of dots named no series. A bar drawn with its ends or
corners rounded is now a bar, a legend's dot is a swatch, and a rounded bar is never one — a
rounded bullet beside a paragraph had named a series as near as its legend. Charts 0.1297 ->
0.1337, 4 pages up and none down; tables, content faithfulness, semantic formatting and visual
grounding unchanged on every page. Overall 48.47 -> 48.55.

### After the line reader seats every number it can and reads each chart's own axis (decision #89, 2026-10-21)

The line rule kept each point's nearest number, so a line's numbers set nearer the next line's
points were that line's; it read no line stroked segment by segment; it took a series' categories
from below every series at the same places, so charts stacked down a column read the bottom one's
axis. Numbers are now seated so that as many are as can be, of the series' own kind, a line stroked
in segments is one line, and each series reads the first row below its own points. Charts 0.1337
-> 0.1363, 3 pages up and none down; visual grounding 0.5671 -> 0.5670, one page down where a chart
now reads as its table, a trade the owner took. Overall 48.55 -> 48.60.

### After a pie's printed values read as its table (decision #90, 2026-10-21)

No rule read a pie: its slices are curves, not bars or lines, so a pie that prints a number at each
slice read nothing. A new rule, `slice-labels-v1`, reads a slice from its centre, a pie from slices
turning a full circle about one, and matches the numbers set at it to its slices in their order
round the centre, each slice named by its legend or the lines over its number, each pie by its
hole or the line over it. Charts 0.1363 -> 0.1485, 9 pages up and none down; tables, content
faithfulness, semantic formatting and visual grounding unchanged on every page. Overall 48.60 ->
48.84.

### Ethos + OCR pass (decision #45, 2026-10-05)

Under `ETHOS_BENCH_OCR=1` the adapter reads scans through Tesseract outside the engine (English
only; `gs`, `tesseract` and `qpdf` on PATH). These are not engine numbers and are reported beside
them:

| Dimension | Engine only | Ethos + OCR pass |
| --- | ---: | ---: |
| Content faithfulness | 0.6633 | 0.7747 |
| Semantic formatting | 0.4080 | 0.4167 |
| Tables | 0.4833 | 0.4882 |
| Charts | 0.1019 | 0.1019 |
| Visual grounding | 0.4708 | 0.4922 |

137 pages rise and none falls. Overall 42.55 engine-only, 45.47 with the pass.

## What it found that is not fixed

- **Content on readable pages** (decision #61's census, 2026-10-11). Of the content lost on text
  pages with a text layer, most is order rules whose sentence is not found at all: text no font
  states (`text_misc__marks` draws its words as outlines and an image), OCR layers that are
  themselves wrong, and ground-truth conventions — `text_simple__caldera`'s index entries quote
  their names and write underscores as spaces. Measured and not used: a flagged space at every
  word-sized pen move between two runs on a baseline, the `TJ` rule's 0.18 em applied between
  strings (content +0.0003, 13 pages up and 30 down: a letter-spaced heading, each letter its own
  string, reads `L U N C H T I M E`). Kept: `<sup>` and `<sub>` (decision #40) cost 59 order rules,
  because the scorer turns a tag into a space inside the word it marks. And the scorer reads an
  empty Markdown as no output at all and a page of whitespace as text: a blank page scores 0 here
  and 0.38 for a reader that prints the page's spaces. This engine prints none, and will not print
  whitespace for a score.
- **Tables.** Phrases before the unruled fold do not recover them; see
  [`../opendataloader-bench/tables-why-zero.md`](../opendataloader-bench/tables-why-zero.md), the
  2026-10-03 amendment.
- **Fonts that state no character.** `text_multilang__mandarin4` and `text_simple__grayson` are
  still refused after decision #43 (2026-10-05), because none of their fonts states one: they ship
  a `/ToUnicode` holding a codespace and no mapping, over a Type 1C program whose glyph names are
  indices (`/G21`) and a two-glyph TrueType stub. `text_multilang__korean2`, refused with them
  until decision #78, sets its text in a CID-keyed CFF program under the `Adobe-Korea1` ordering
  with no `/ToUnicode`, and is now read through Adobe's published table for that collection.
- **Two streams `lopdf` does not load**, `text_ocr__mix` and `text_ocr__012-25`: a tab before
  `endstream` that qpdf reads. That is the `lopdf` boundary the 2026-09-25 review named as this
  engine's top risk, and it was left alone.
- **Reading order where a form's text fills a gutter**: opendataloader-bench `01030000000042`,
  `docs/30-FORM-XOBJECTS-SCOPE.md` §12.

## ExtractBench was not run

`run-llama/ExtractBench` scores schema-guided extraction into JSON, which needs a language model.
This engine could only be that model's parse stage, so the score would mostly measure the model.
Its word-level grounding also needs word boxes, which
[`22-WORD-BOXES-SCOPE.md`](../../22-WORD-BOXES-SCOPE.md) refused.
