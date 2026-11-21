# Benchmarks

ethos-parser is scored on three public benchmarks, each with its own evaluator. Every ethos-parser
number here was measured by this project on `31e4f4d` — the assist lane's on `0f6c8a3`, not run
again — and scored as the benchmark's current leaderboard is scored. Every other row is copied from that leaderboard. Higher is better.

Three lanes are reported:

- **ethos-parser** is the engine alone: no OCR and no model. This is the row ranked against the
  other parsers.
- **+ `--ocr`** adds the bundled recognizer, `ethos-parser-ocr`, for scanned pages and images:
  PP-OCRv5's networks and an orientation classifier, built into the binary and run on your machine.
- **+ assist model** adds LightOnOCR-3-0.8B, a model you run, through the assist lane
  ([`assist/README.md`](../assist/README.md)). Its text reaches the Markdown inside a block labelled
  *Proposed*, and is scored as the product writes it.

How each number is produced, run by run, is in
[`measurements/parsebench/`](measurements/parsebench/) (both ParseBench sets) and
[`measurements/opendataloader-bench/`](measurements/opendataloader-bench/).

## ParseBench

[ParseBench](https://github.com/run-llama/ParseBench): about 2,000 human-verified pages from
enterprise documents, scored on five dimensions. Its leaderboard was re-scored on 15–17 November
2026 by its harness at `c61ff10` (parse-bench 2.1.0), the dataset unchanged at `2805a1d`; the other
rows are its `leaderboard.csv` at `d7efdbc`, 157 rows. ethos-parser scores the same under
parse-bench 2.1.1 (`d7efdbc`) as under the earlier `1f5b37a`, dimension by dimension, in every lane.

Against the open-source local parsers on its leaderboard, with the highest overall row for scale:

| Parser | Overall | Tables | Charts | Content | Formatting | Grounding |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| LlamaParse Agentic Plus (highest overall, cloud) | 86.69 | 93.27 | 92.19 | 89.74 | 82.84 | 75.43 |
| **ethos-parser** | **52.15** | 64.53 | 17.30 | 67.52 | 51.40 | 59.99 |
| PyMuPDF4LLM | 50.47 | 56.93 | 1.63 | 75.25 | 56.66 | 61.89 |
| Warp Ingest | 39.33 | 57.50 | 2.45 | 72.24 | 42.94 | 21.50 |
| LiteParse (no OCR) † | 36.90 | 42.40 | 1.80 | 70.00 | 38.50 | 31.80 |
| OpenDataLoader † | 29.40 | 35.18 | 0.92 | 66.06 | 34.07 | 10.77 |
| pdf-inspector † | 26.59 | 26.64 | 5.29 | 56.06 | 35.07 | 9.91 |
| MarkItDown † | 18.63 | 15.77 | 2.02 | 64.54 | 0.91 | 9.90 |
| PyMuPDF (HTML) † | 16.62 | 0.00 | 0.00 | 55.63 | 18.26 | 9.20 |
| PyMuPDF (Text) † | 16.02 | 0.00 | 0.00 | 68.28 | 0.95 | 10.86 |
| pypdf † | 14.87 | 0.00 | 0.00 | 62.50 | 0.91 | 10.92 |

† Not re-scored: the leaderboard holds no published outputs for these rows (`Verified=no`), so they
keep the numbers submitted.

Every lane, placed on the whole leaderboard (158 rows with ethos-parser):

| Lane | Overall | Tables | Charts | Content | Formatting | Grounding | Place |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ethos-parser | 52.15 | 64.53 | 17.30 | 67.52 | 51.40 | 59.99 | 113th |
| + `--ocr` | 55.14 | 64.54 | 17.30 | 77.39 | 53.17 | 63.30 | 107th |
| + assist model, as written | 65.34 | 60.53 | 64.81 | 83.70 | 54.41 | 63.28 | 69th |
| + assist model, quote markers removed before scoring | 71.93 | 85.75 | 66.67 | 84.18 | 59.75 | 63.28 | 31st |

- The two assist rows were measured on `0f6c8a3` and have not been run again on `31e4f4d`.
- **As written** is what the product writes: the model's blocks inside a block quote under their
  *Proposed* label. ParseBench's converter reads no table or title inside a quote, so the last row
  shows what the model's reading scores once the benchmark's adapter removes the quote markers. The
  product never writes that form. LightOnOCR-3-0.8B on its own scores 74.56 on the same
  leaderboard. The product stays below it because, on every page where the engine read at least 30%
  of the words the model would write, it keeps the engine's own text and boxes and adds only the
  model's tables and charts. With the quote markers removed, its tables and charts score above the
  model's own; its content, formatting and grounding score below.
- Each overall is the mean of the five unrounded dimensions, as the leaderboard computes it, so it
  can differ by 0.01 from the mean of the columns as printed.
- On charts, ethos-parser reads only the values a chart prints; it never estimates a value from the
  drawing.

## ParseBench-2

[ParseBench-2](https://huggingface.co/datasets/llamaindex/ParseBench-2): the same project's second
set, 4,317 pages in 13 subsets, scored on six dimensions by the same harness. Its leaderboard,
`leaderboard-v2.csv` at `d7efdbc` (56 rows), is scored at dataset revision `f0757ac` by parse-bench
2.1.0 (`c61ff10`), and so is ethos-parser here. Every row on it is a hosted parsing service or an AI
model.

Around ethos-parser, with the highest overall row and the assist model for scale:

| Parser | Overall | Text | Formatting | Tables | Charts | Grounding | Forms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| LlamaParse (Agentic Plus) (highest overall, cloud) | 90.82 | 90.22 | 93.60 | 94.72 | 92.58 | 88.23 | 85.56 |
| LightOnOCR-3 0.8B (the assist model, on its own) | 77.12 | 81.24 | 80.87 | 78.58 | 61.22 | 82.90 | 77.92 |
| **ethos-parser + `--ocr`** | **57.94** | 62.51 | 78.97 | 51.71 | 17.94 | 66.76 | 69.72 |
| Falcon-OCR | 57.78 | 64.58 | 75.93 | 72.57 | 0.69 | 64.56 | 68.33 |
| **ethos-parser** | **55.64** | 57.58 | 71.39 | 49.98 | 17.94 | 67.25 | 69.72 |
| jina-ocr-v1 | 48.59 | 75.13 | 79.87 | 55.00 | 8.74 | 0.00 | 72.79 |
| Qwen3.5-4B | 42.86 | 77.32 | 74.77 | 12.86 | 6.88 | 16.14 | 69.21 |
| gemma-4-E4B-it | 42.74 | 65.03 | 70.57 | 30.82 | 5.39 | 22.74 | 61.87 |

- Placed alone on the leaderboard (57 rows with it), ethos-parser would be 54th, and 53rd with
  `--ocr`.
- The assist model has not been run through the product on ParseBench-2.
- parse-bench 2.1.1 (`d7efdbc`) changed how forms are scored after the leaderboard was: under it,
  ethos-parser's forms read 69.86, and the two overalls 55.67 and 57.96.
- Overall is the mean of the six dimensions.

## opendataloader-bench

[opendataloader-bench](https://github.com/opendataloader-project/opendataloader-bench): 200
real-world PDFs, scored on reading order (NID), tables (TEDS) and headings (MHS). Pinned at
`7af1d8f`, which is still its latest commit (checked 2026-11-19); the other rows are the 12 engines
in its README table.

| Engine | Overall | Reading order | Tables | Headings | Speed (s/page) |
| --- | ---: | ---: | ---: | ---: | ---: |
| opendataloader [hybrid] | 0.907 | 0.934 | 0.928 | 0.821 | 0.463 |
| nutrient | 0.885 | 0.925 | 0.708 | 0.819 | 0.008 |
| docling | 0.882 | 0.898 | 0.887 | 0.824 | 0.762 |
| marker | 0.861 | 0.890 | 0.808 | 0.796 | 53.932 |
| unstructured [hi_res] | 0.841 | 0.904 | 0.588 | 0.749 | 3.008 |
| edgeparse | 0.837 | 0.894 | 0.717 | 0.706 | 0.036 |
| **ethos-parser** | **0.835** | 0.899 | 0.667 | 0.706 | — |
| opendataloader | 0.831 | 0.902 | 0.489 | 0.739 | 0.015 |
| mineru | 0.831 | 0.857 | 0.873 | 0.743 | 5.962 |
| pymupdf4llm | 0.732 | 0.885 | 0.401 | 0.412 | 0.091 |
| unstructured | 0.686 | 0.882 | 0.000 | 0.388 | 0.077 |
| markitdown | 0.589 | 0.844 | 0.273 | 0.000 | 0.114 |
| liteparse | 0.576 | 0.866 | 0.000 | 0.000 | 1.061 |

- ethos-parser places 7th of 13, above opendataloader and mineru (0.831). Two hybrid variants,
  whose results the benchmark's repository carries but whose README does not list, also score
  above it: 0.877 and 0.845.
- Its adapter asks for `markdown --unstyled` (decision #158): the reading-order metric compares the
  Markdown's raw characters, and the benchmark's ground truth writes no bold or underline, so the
  marks for a run's style would count as wrong characters. The text is as without the flag.
- The liteparse row is the version the benchmark measured. LiteParse's own README reports 0.886 for
  its current version (read 2026-11-18); the benchmark has not re-measured it.
- Overall is the benchmark's own per-document mean, not the mean of the three columns: tables count
  only on the 42 documents that hold one, headings on the 107 that have them.
- Speed is not re-timed for this commit: a timing is quoted only from a quiet machine, and this run
  was not on one. 0.65.0 measured 0.036 s/page. Other engines' speeds are from an Apple M4.
