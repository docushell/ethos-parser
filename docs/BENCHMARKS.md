# Benchmarks

ethos-parser is scored on two public benchmarks, each with its own evaluator, at a pinned commit of
that benchmark. Every ethos-parser number here was measured by this project on `26244de`. Every
other row is copied from the benchmark's published results at the same pin. Higher is better.

Three lanes are reported:

- **ethos-parser** is the engine alone: no OCR and no model. This is the row ranked against the
  other parsers.
- **+ `--ocr`** adds the bundled recognizer, `ethos-parser-ocr`, for scanned pages and images:
  PP-OCRv5's networks and an orientation classifier, built into the binary and run on your machine.
- **+ assist model** adds LightOnOCR-3-0.8B, a model you run, through the assist lane
  ([`assist/README.md`](../assist/README.md)). Its text reaches the Markdown inside a block labelled
  *Proposed*, and is scored as the product writes it.

How each number is produced, run by run, is in
[`measurements/parsebench/`](measurements/parsebench/) and
[`measurements/opendataloader-bench/`](measurements/opendataloader-bench/).

## ParseBench

[ParseBench](https://github.com/run-llama/ParseBench): about 2,000 human-verified pages from
enterprise documents, scored on five dimensions. Pinned at `1f5b37a`; the other rows are its
`leaderboard.csv` at that commit, 154 rows.

Against the open-source local parsers on its leaderboard, with the highest overall row for scale:

| Parser | Overall | Tables | Charts | Content | Formatting | Grounding |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| LlamaParse Agentic Plus (highest overall, cloud) | 86.61 | 93.37 | 90.19 | 88.74 | 86.54 | 74.21 |
| PyMuPDF4LLM | 53.49 | 72.00 | 2.19 | 79.34 | 52.04 | 61.89 |
| **ethos-parser** | **52.15** | 64.53 | 17.30 | 67.52 | 51.39 | 59.99 |
| Warp Ingest | 40.18 | 57.45 | 7.04 | 70.81 | 45.81 | 19.78 |
| LiteParse (no OCR) | 36.90 | 42.40 | 1.80 | 70.00 | 38.50 | 31.80 |
| OpenDataLoader | 29.40 | 35.18 | 0.92 | 66.06 | 34.07 | 10.77 |
| pdf-inspector | 26.59 | 26.64 | 5.29 | 56.06 | 35.07 | 9.91 |
| MarkItDown | 18.63 | 15.77 | 2.02 | 64.54 | 0.91 | 9.90 |
| PyMuPDF (HTML) | 16.62 | 0.00 | 0.00 | 55.63 | 18.26 | 9.20 |
| PyMuPDF (Text) | 16.02 | 0.00 | 0.00 | 68.28 | 0.95 | 10.86 |
| pypdf | 14.87 | 0.00 | 0.00 | 62.50 | 0.91 | 10.92 |

Every lane, placed on the whole leaderboard (155 rows with ethos-parser):

| Lane | Overall | Tables | Charts | Content | Formatting | Grounding | Place |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| ethos-parser | 52.15 | 64.53 | 17.30 | 67.52 | 51.39 | 59.99 | 115th |
| + `--ocr` | 54.68 | 64.54 | 17.30 | 75.48 | 52.82 | 63.28 | 109th |
| + assist model, as written | 65.35 | 60.53 | 64.81 | 83.70 | 54.41 | 63.28 | 68th |
| + assist model, quote markers removed before scoring | 71.93 | 85.75 | 66.67 | 84.18 | 59.75 | 63.28 | 34th |

- **As written** is what the product writes: the model's blocks inside a block quote under their
  *Proposed* label. ParseBench's converter reads no table or title inside a quote, so the last row
  shows what the model's reading scores once the benchmark's adapter removes the quote markers. The
  product never writes that form. LightOnOCR-3-0.8B on its own scores 74.6 on the same leaderboard.
  The product stays below it because, on every page where the engine read at least 30% of the words
  the model would write, it keeps the engine's own text and boxes and adds only the model's tables
  and charts. With the quote markers removed, its tables and charts score above the model's own; its
  content, formatting and grounding score below.
- **ParseBench re-scored its leaderboard after this pin** (15–17 November 2026). Some rows moved:
  PyMuPDF4LLM 53.49 → 50.47, Warp Ingest 40.18 → 39.33, LlamaParse Agentic Plus 86.61 → 86.69.
  ethos-parser has not been measured with the re-scored evaluator, so every row here stays at the
  pin.
- Each overall is the mean of the five dimensions as shown, to two decimals.
- On charts, ethos-parser reads only the values a chart prints; it never estimates a value from the
  drawing.

## opendataloader-bench

[opendataloader-bench](https://github.com/opendataloader-project/opendataloader-bench): 200
real-world PDFs, scored on reading order (NID), tables (TEDS) and headings (MHS). Pinned at
`7af1d8f`, which is still its latest commit; the other rows are the 12 engines in its README table.

| Engine | Overall | Reading order | Tables | Headings | Speed (s/page) |
| --- | ---: | ---: | ---: | ---: | ---: |
| opendataloader [hybrid] | 0.907 | 0.934 | 0.928 | 0.821 | 0.463 |
| nutrient | 0.885 | 0.925 | 0.708 | 0.819 | 0.008 |
| docling | 0.882 | 0.898 | 0.887 | 0.824 | 0.762 |
| marker | 0.861 | 0.890 | 0.808 | 0.796 | 53.932 |
| unstructured [hi_res] | 0.841 | 0.904 | 0.588 | 0.749 | 3.008 |
| edgeparse | 0.837 | 0.894 | 0.717 | 0.706 | 0.036 |
| opendataloader | 0.831 | 0.902 | 0.489 | 0.739 | 0.015 |
| mineru | 0.831 | 0.857 | 0.873 | 0.743 | 5.962 |
| **ethos-parser** | **0.813** | 0.891 | 0.667 | 0.637 | — |
| pymupdf4llm | 0.732 | 0.885 | 0.401 | 0.412 | 0.091 |
| unstructured | 0.686 | 0.882 | 0.000 | 0.388 | 0.077 |
| markitdown | 0.589 | 0.844 | 0.273 | 0.000 | 0.114 |
| liteparse | 0.576 | 0.866 | 0.000 | 0.000 | 1.061 |

- ethos-parser places 9th of 13. Two hybrid variants, whose results the benchmark's repository
  carries but whose README does not list, also score above it: 0.877 and 0.845.
- The liteparse row is the version the benchmark measured. LiteParse's own README reports 0.886 for
  its current version (read 2026-11-18); the benchmark has not re-measured it.
- Overall is the benchmark's own per-document mean, not the mean of the three columns: tables count
  only on the 42 documents that hold one, headings on the 107 that have them.
- Speed is not re-timed for this commit: a timing is quoted only from a quiet machine, and this run
  was not on one. 0.65.0 measured 0.036 s/page. Other engines' speeds are from an Apple M4.
