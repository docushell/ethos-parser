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

- **Tables.** Phrases before the unruled fold do not recover them; see
  [`../opendataloader-bench/tables-why-zero.md`](../opendataloader-bench/tables-why-zero.md), the
  2026-10-03 amendment.
- **Fonts that state no character.** `text_multilang__korean2`, `text_multilang__mandarin4` and
  `text_simple__grayson` are still refused after decision #43 (2026-10-05), because none of their
  fonts states one: korean2's is a CID-keyed CFF program under the `Adobe-Korea1` ordering with no
  `/ToUnicode`, which only Adobe's published CID-to-Unicode table decodes and this profile does not
  carry; mandarin4's and grayson's ship a `/ToUnicode` holding a codespace and no mapping, over a
  Type 1C program whose glyph names are indices (`/G21`) and a two-glyph TrueType stub.
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
