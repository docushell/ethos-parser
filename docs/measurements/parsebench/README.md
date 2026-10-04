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

## What it found that is not fixed

- **Tables.** Phrases before the unruled fold do not recover them; see
  [`../opendataloader-bench/tables-why-zero.md`](../opendataloader-bench/tables-why-zero.md), the
  2026-10-03 amendment.
- **Fonts with neither `/ToUnicode` nor a readable encoding.** `text_multilang__korean2`,
  `text_multilang__mandarin4` and `text_simple__grayson` are refused because nothing decodes;
  another reader recovers them through the embedded font program's own `cmap`, which this engine
  does not read.
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
