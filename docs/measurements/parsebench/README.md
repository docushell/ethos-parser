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
- **Visual grounding.** It reports the engine's own units with their boxes: each `ground` element,
  each table and each image. Labels come only from what the record states: a tagged role path, an
  `inferred_heading` run, a detected table, a drawn image, and `Text` otherwise. The benchmark's
  LiteParse layout adapter and label mapper are reused under the key `ethos`.

**Measured and not used: the engine's blocks as layout units.** Grouping runs by
`TextRunAttributes::block` scored lower than one baseline's ink (element pass rate 0.174 against
0.220). A block often holds several paragraphs and a heading, so attribution fails. The ground
truth is paragraphs, and the engine makes no paragraph on an untagged page.

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
