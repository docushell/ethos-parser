<div align="center">

# ethos-parser

**Document parsing you can check.**

PDFs, office files and scans in. Markdown, HTML and JSON out,<br>
with every character traced back to where the file drew it.

[![CI](https://github.com/docushell/ethos-parser/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/docushell/ethos-parser/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/docushell/ethos-parser?label=release&color=blue)](https://github.com/docushell/ethos-parser/releases/latest)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust 1.88](https://img.shields.io/badge/Rust-1.88-orange?logo=rust&logoColor=white)](rust-toolchain.toml)
[![Linux, macOS, Windows](https://img.shields.io/badge/Linux%20%7C%20macOS%20%7C%20Windows-supported-lightgrey)](docs/INSTALL.md)

[![Engine: no AI model](https://img.shields.io/badge/engine-no%20AI%20model-success)](#why-ethos-parser)
[![Runs offline](https://img.shields.io/badge/runs-offline-success)](#why-ethos-parser)
[![Output: byte-identical](https://img.shields.io/badge/output-byte--identical-success)](#why-ethos-parser)
[![Dependencies: no AGPL](https://img.shields.io/badge/dependencies-no%20AGPL-success)](deny.toml)
[![MCP server](https://img.shields.io/badge/MCP-server-8A2BE2)](#plug-it-into-your-stack)
[![Python 3.10+](https://img.shields.io/badge/Python-3.10%2B-3776AB?logo=python&logoColor=white)](packages/python/)
[![Node.js 18+](https://img.shields.io/badge/Node.js-18%2B-339933?logo=nodedotjs&logoColor=white)](packages/node/)

[Quick start](#try-it-in-30-seconds) ·
[How it works](#how-it-works) ·
[Benchmarks](#benchmarks) ·
[Commands](#commands) ·
[Install](docs/INSTALL.md) ·
[Docs](docs/README.md)

</div>

Every gap ethos-parser could not read is reported, not hidden, and nothing is invented. Built for
RAG and for any document where a citation has to hold up: contracts, filings, reports.

## Try it in 30 seconds

**1. Install** on macOS or Linux. For Windows, see [`docs/INSTALL.md`](docs/INSTALL.md).

```bash
curl -fsSL https://raw.githubusercontent.com/docushell/ethos-parser/main/install.sh | sh
```

The script picks your platform's archive from the latest release, checks it against
`SHA256SUMS.txt`, checks the binary starts, and installs the binaries into `~/.local/bin`. Nothing
is installed if either check fails. If `~/.local/bin` is not on your `PATH`, the script prints the
line that adds it. The commands below use [`jq`](https://jqlang.org) to pull fields out of the
JSON.

**2. Run it** on a document of your own: a PDF, DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF or EPUB.

```bash
# Markdown. The JSON also holds .anchor_map, from every character back to the page
ethos-parser markdown --source report.pdf | jq -r .markdown

# The same, with invisible text left out and counted
ethos-parser markdown --source report.pdf --drop-hidden | jq .coverage.dropped

# A copy of the PDF with what was found drawn on it
ethos-parser overlay report.pdf > overlay.pdf

# A scan or a photo of a page: read by the bundled recognizer, on your machine
ethos-parser markdown --source scan.pdf --ocr | jq -r .markdown
```

`--drop-hidden` and `--ocr` need 0.66.0 or later. The 0.65.0 release does not have them; until the
next release, [build from source](docs/INSTALL.md).

<details>
<summary><b>No document at hand? Try the sample page</b></summary>

<br>

```bash
curl -fLO https://raw.githubusercontent.com/docushell/ethos-parser/main/docs/sample.pdf
ethos-parser markdown --source sample.pdf | jq -r .markdown
```

It prints the page as Markdown: the heading, the paragraph, the table and the caption.

```markdown
# Quarterly Harbor Report

Seven lanterns guard the quiet northern bay. Traffic rose through the third
quarter, while the old pier stayed closed for repairs.

| Quarter | Ships | Cargo (t) |
| --- | --- | --- |
| Q1 | 14 | 4,085 |
| Q2 | 27 | 5,190 |
| Q3 | 39 | 6,243 |

Figure 1. The northern bay at dusk.

Cargo is in tonnes. The fourth quarter follows in January.

Draft note: figures not yet audited.

Page 1
```

`Draft note: figures not yet audited.` is drawn invisible on the page: a reader never sees it, but
a model fed the text would. The record flags it, and `--drop-hidden` leaves it out of the Markdown
and counts its 36 characters.

</details>

## How it works

<p align="center">
  <img src="docs/images/how-it-works.svg" width="100%" alt="How ethos-parser works. A PDF or an office file enters the engine, which has no AI model. The engine reads text runs with locators, applies layout rules and writes one record. Markdown, HTML, elements, grounding and annotated PDFs are all projected from that record. Scans pass through the optional --ocr recognizer, and an optional assist model adds text labelled Proposed; both sit outside the engine.">
</p>

1. **Read.** The format is decided from the file's first bytes, never its name. The engine reads
   each text run with a locator back into the source. With `--ocr`, the bundled recognizer reads
   scanned pages.
2. **Structure.** Named rules, not a model, find the reading order, headings, lists, tables and
   figures. What a rule could not read becomes a named limitation or a counted gap.
3. **Record.** One record holds every run and every gap, and every output is projected from it.
   So Markdown, HTML, the element list and a citation all point at the same text, and the same
   input always writes the same bytes.

## Why ethos-parser

| Principle | In practice |
| --- | --- |
| **Citations you can check** | Every text run carries a locator: page and position in a PDF; part, paragraph and run in Word; sheet, row and column in a spreadsheet. Markdown and HTML come with an anchor map, so a checker can reject a quote that touches text the exporter added rather than the document. |
| **Gaps reported, not hidden** | Text it could not decode, text drawn invisible, items with no box: each becomes a named limitation or a counted gap. Nothing is dropped silently. |
| **Nothing invented** | The engine never makes up a table cell, a coordinate, an id or a page number. A `.docx` has no pages, so it gets no page number rather than a guessed one. |
| **Deterministic** | There are no floating-point numbers in the JSON it writes. A release ships only when the Linux, macOS and Windows binaries write identical bytes for the gate documents: eight public PDFs kept in [`fixtures/gate/`](fixtures/gate/). |
| **Offline** | The engine and `--ocr` make no network calls and send no telemetry. The engine uses no AI model; `--ocr`'s neural networks are built into `ethos-parser-ocr`, so nothing is downloaded. The one exception is opt-in: the assist lane talks to a model server you name. |
| **Formats read from the bytes** | PDF, DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF and EPUB, plus PNG and JPEG with `--ocr`. The file name is never consulted. |
| **Layout** | Reading order across columns, headings, lists, tables split across pages, strikeout and underline, running heads and page numbers, and pages drawn sideways or upside down. |
| **A licence you can approve** | Apache-2.0, and every dependency is under a permissive licence. For the engine, the recognizer and the assist binary, `deny.toml` enforces that, and CI proves the rule fires by flipping a crate to AGPL and requiring the build to reject it. |

## Benchmarks

Scored on three public benchmarks, each with its own scorer, as its leaderboard is scored today.
Measured on `0ee4b4c`, the `--ocr` column on `dc625f6` (the assist column on `0f6c8a3`). Higher is
better.

| Benchmark | ethos-parser | + `--ocr` | + assist model |
| --- | ---: | ---: | ---: |
| [ParseBench](https://github.com/run-llama/ParseBench), ~2,000 pages, overall | 52.18 | 55.56 | 65.34 |
| [ParseBench-2](https://huggingface.co/datasets/llamaindex/ParseBench-2), ~4,300 pages, overall | 55.90 | 58.76 | — |
| [opendataloader-bench](https://github.com/opendataloader-project/opendataloader-bench), 200 PDFs, overall | 0.835 | — | — |

- **ethos-parser** is the engine alone: no OCR, no model, no GPU.
- **+ `--ocr`** adds the bundled recognizer for scanned pages: neural networks that run on your
  machine.
- **+ assist model** adds LightOnOCR-3-0.8B, a model you run, scored as the product writes it,
  inside its *Proposed* label.

### Among parsers of its own kind

ParseBench's leaderboard lists nine open-source parsers that run locally. This project measured
ethos-parser with the same scorer; placed among them:

| Parser | Overall | Tables | Charts | Content | Formatting | Grounding |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| **ethos-parser** | **52.18** | **64.53** | **17.30** | 67.65 | 51.43 | 59.99 |
| PyMuPDF4LLM | 50.47 | 56.93 | 1.63 | **75.25** | **56.66** | **61.89** |
| Warp Ingest | 39.33 | 57.50 | 2.45 | 72.24 | 42.94 | 21.50 |
| LiteParse (no OCR) † | 36.90 | 42.40 | 1.80 | 70.00 | 38.50 | 31.80 |
| OpenDataLoader † | 29.40 | 35.18 | 0.92 | 66.06 | 34.07 | 10.77 |
| pdf-inspector † | 26.59 | 26.64 | 5.29 | 56.06 | 35.07 | 9.91 |
| MarkItDown † | 18.63 | 15.77 | 2.02 | 64.54 | 0.91 | 9.90 |
| PyMuPDF (HTML) † | 16.62 | 0.00 | 0.00 | 55.63 | 18.26 | 9.20 |
| PyMuPDF (Text) † | 16.02 | 0.00 | 0.00 | 68.28 | 0.95 | 10.86 |
| pypdf † | 14.87 | 0.00 | 0.00 | 62.50 | 0.91 | 10.92 |

Bold marks the highest value in each column. † The leaderboard has not re-scored these rows; they
keep the numbers submitted.

ethos-parser is not on any benchmark's published list; these are this project's own measurements.

### Against AI models and hosted services

The other 148 rows on ParseBench's leaderboard are hosted services and AI models. The highest
scores 86.69. Placed among them:

| Lane | ParseBench overall | Scores above |
| --- | ---: | ---: |
| ethos-parser, no model | 52.18 | 36 of 148 |
| + `--ocr` | 55.56 | 42 of 148 |
| + assist model | 65.34 | 80 of 148 |

- **With a model you run**, the assist lane keeps the model's text apart from the document's own.
  Why its score stays below the model's own 74.56 is in [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md).
- **ParseBench-2** lists only hosted services and AI models. With `--ocr`, ethos-parser scores
  58.76 there on a CPU: above Falcon-OCR (57.78), jina-ocr-v1 (48.59), Qwen3.5-4B (42.86) and
  gemma-4-E4B-it (42.74). The other 52 rows score higher, up to 90.82.

### opendataloader-bench

200 real-world PDFs, scored on reading order, tables and headings. ethos-parser scores 0.835. Placed
among the 12 engines in the benchmark's table it is 7th of 13, within 0.006 of the two above it, at
0.036 s a page ‡.

<details>
<summary><b>The full table</b></summary>

<br>

| Engine | Overall | Reading order | Tables | Headings | Speed (s/page) |
| --- | ---: | ---: | ---: | ---: | ---: |
| opendataloader [hybrid] | 0.907 | 0.934 | 0.928 | 0.821 | 0.463 |
| nutrient | 0.885 | 0.925 | 0.708 | 0.819 | 0.008 |
| docling | 0.882 | 0.898 | 0.887 | 0.824 | 0.762 |
| marker | 0.861 | 0.890 | 0.808 | 0.796 | 53.932 |
| unstructured [hi_res] | 0.841 | 0.904 | 0.588 | 0.749 | 3.008 |
| edgeparse | 0.837 | 0.894 | 0.717 | 0.706 | 0.036 |
| **ethos-parser** | **0.835** | 0.899 | 0.667 | 0.706 | 0.036 ‡ |
| opendataloader | 0.831 | 0.902 | 0.489 | 0.739 | 0.015 |
| mineru | 0.831 | 0.857 | 0.873 | 0.743 | 5.962 |
| pymupdf4llm | 0.732 | 0.885 | 0.401 | 0.412 | 0.091 |
| unstructured | 0.686 | 0.882 | 0.000 | 0.388 | 0.077 |
| markitdown | 0.589 | 0.844 | 0.273 | 0.000 | 0.114 |
| liteparse | 0.576 | 0.866 | 0.000 | 0.000 | 1.061 |

- The other rows are the 12 engines in the benchmark's README table. Two hybrid variants its
  repository carries but its README does not list also score above ethos-parser: 0.877 and 0.845.
- The liteparse row is the version the benchmark measured. LiteParse's own README reports 0.886
  for its current version; the benchmark has not re-measured it.
- ethos-parser's adapter asks for `markdown --unstyled`, since this benchmark's reading order
  counts a bold mark as a wrong character. The text is the same without the flag.

</details>

‡ Timed at 0.65.0 on an Apple M4 Pro and not re-timed since. The other engines' speeds are the
benchmark's own, from an Apple M4.

Where each lane would place on the whole leaderboards, each benchmark's pinned commit and how to
reproduce every number are in [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md).

## Plug it into your stack

| You are building | Start with |
| --- | --- |
| A pipeline or a script | The CLI, as above. `ethos-parser <command> --help` describes every flag |
| An agent in Claude Code | `claude mcp add ethos-parser -- "$(command -v ethos-parser)" mcp`, with `ethos-parser` on your `PATH` |
| A Python app (3.10+) | `pip install "git+https://github.com/docushell/ethos-parser#subdirectory=packages/python"` |
| A Node app (18+) | `git clone --depth 1 https://github.com/docushell/ethos-parser && npm install ./ethos-parser/packages/node` |
| From source | `cargo build --release --locked` |

The Python and Node packages call the `ethos-parser` binary, so install it first. Both ship
LangChain tools behind an optional install: see [`packages/python/`](packages/python/) and
[`packages/node/`](packages/node/).

```python
import ethos_parser

representation = ethos_parser.extract("contract.pdf")
grounding = ethos_parser.ground(representation)
```

## Commands

| Command | What it does |
| --- | --- |
| `extract` | The document as text runs with locators. `--ocr` reads scans and images; `--assist` asks a model you run |
| `markdown` | Markdown plus its anchor map, from a record or straight from a file with `--source` |
| `html` | HTML from a record `extract` wrote, with the same anchor map, keeping merged table cells |
| `elements` | One list of text, headings, page headers and footers, tables and pictures, each with its page and box |
| `locate` | Every place a string occurs, as node ids, offsets and boxes |
| `classify` | Counts and reason codes: is it scanned, is the layout hard |
| `overlay` | An annotated copy of the PDF: what was found, and a count of what has no box |
| `ground` | A record turned into `ethos.grounding.v1`, the citation format |
| `grounding-check` | Checks a grounding file's structure and its link to the source bytes |
| `verify` | Hands a citation check to the pinned [Ethos](https://github.com/docushell/ethos) verifier, a separate program, and relays its answer |
| `tag` | A copy of an untagged PDF with a structure tree written in, marked as computed |
| `mcp` | `extract`, `ground`, `node_get`, `locate` and `markdown` served to an agent over stdio |

Exit codes always mean the same thing: **0** it worked, **1** it ran and the answer is no, **2** it
could not run. `--diagnostics` adds timing to stderr; stdout never changes.

## What the output looks like

A text run and its locator:

```json
{
  "id": "s1",
  "kind": "text_run",
  "text": "Measured",
  "native_locator": {
    "pdf": { "page": 1, "origin_x": 7200, "origin_y": 7200, "advance": 9600 }
  }
}
```

Coordinates are whole centipoints, so `7200` means 72 pt.

Markdown and its anchor map:

```json
{
  "markdown": "Measured\n",
  "anchor_map": {
    "segments": [
      { "start": 0, "end": 8, "kind": "source", "node_ids": ["s1"] },
      { "start": 8, "end": 9, "kind": "syntax" }
    ]
  }
}
```

Every byte is the document's own text (`source`), syntax the exporter added (`syntax`), or, with
the assist lane, a model's text (`proposed`). **A quote that touches anything but `source` bytes is
not a citation.**

## Scans and the assist lane

**`--ocr`** runs the recognizer in `ethos-parser-ocr` on your machine: PP-OCRv5's detection and
recognition networks and an orientation classifier, built into the binary. It reads scanned pages,
PNG and JPEG, and stands a sideways or upside-down scan upright first. It reads English only: its
recognition model and alphabet are PP-OCRv5's English ones, so text in another script, such as
Chinese, Arabic or Cyrillic, is not read correctly. On x86-64 it needs AVX2 and FMA. Scope:
[`docs/35-OCR-SCOPE.md`](docs/35-OCR-SCOPE.md).

**The assist lane** is opt-in. A model server you run reads the pages you choose. Its text reaches
Markdown and HTML inside a block labelled *Proposed*, never as evidence and never citable. No assist
model ships; setup is in [`assist/README.md`](assist/README.md).

## What it will never do

- Emit a confidence score, a grade, or any single field meaning "this document is good."
- Delete hidden text, headers, footers or anything else without saying so.
- Invent a coordinate, an id, a fingerprint or a page number.
- Verify a citation itself. It checks structure and relays a verifier's answer.

## Where things stand

Version 0.66.0-dev.1. PDF and eight office formats read; Markdown, HTML, MCP and both SDKs ship.
The full inventory of what this build can and cannot do is [`docs/CAPABILITY.md`](docs/CAPABILITY.md).

## Tests and contributing

```bash
cargo test --workspace --locked   # nothing skipped; the oracle test needs the Ethos repo
bash ci/gate.sh                   # the whole gate, exactly as CI runs it
```

The oracle test treats Ethos as the reference answer: it runs Ethos's own `ethos grounding check`
beside this engine's `grounding-check` on Ethos's 15 conformance fixtures, and requires the two to
agree. It needs a release build of the
Ethos repo checked out beside this one, at `../ethos`, or `ETHOS_BIN` and `ETHOS_FIXTURES` naming
its binary and its `fixtures/` directory. The gate, `ci/gate.sh`, runs the checks of CI's main job
on your machine: banned-word scans, version strings, formatting, lint, build, tests, licences, the
SDK suites, the recognizer and the assist binary.

Every commit needs a `Signed-off-by` trailer. Run `git config core.hooksPath .githooks` once and
the tracked hooks add and check it. Start with [`docs/README.md`](docs/README.md).

## Licence

Apache-2.0. `NOTICE` records the encoding data, models and dependencies that ship. No AGPL
dependencies: `deny.toml` enforces it for the engine, the recognizer and the assist binary, and CI
proves the rule fires.
