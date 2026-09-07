# Running ethos-parser against OmniDocBench

The instrument behind 0.47.0's numbers, committed so they can be re-derived rather than believed —
the same correction [`../block-subdivision/`](../block-subdivision/) and
[`../opendataloader-bench/`](../opendataloader-bench/) make for their own measurements.

## This is an instrument, not a leaderboard entry

[`06-STEAL-REFUSE.md`](../../06-STEAL-REFUSE.md) O26 refuses *"#1, fastest, or any bake-off claim"*,
and no score is computed here. What is computed is a **census**: which limitation codes fire on how
many documents, and how the Markdown projection assembles blocks. `../opendataloader-bench/` says
why that is the more useful artifact — *"a score says how well and the census says where"* — and
this corpus is the case for it, because the census found two defects and a score would have found
neither.

**A score on this corpus would not be quotable even if one were computed.** The pages here are the
superseded `v1_0` set whose own maintainers advise against scoring them (see below), and until
0.47.0 any text metric would have measured one projection defect and nothing else.

## Getting the corpus, and what it is

**OmniDocBench ships no PDFs today.** The HuggingFace dataset `opendatalab/OmniDocBench` on `main`
(v1.6, 1 651 pages) is page rasters plus annotations; `pdfs/` and `ori_pdfs/` were deleted from
`main` on 2025-09-25 and never restored. The engine reads PDF bytes and has no OCR, so the shipped
release cannot be fed to it at all.

The only born-digital PDFs ever published are on branch **`v1_0`**, commit
`f5f559bddf50e36f7f9899d842d0006f13ce8afc`: `ori_pdfs/`, **981 files, 538 521 457 bytes**, one page
per file. That is 59.4% of the current benchmark, with v1.0-era annotations two revisions behind.

Three things a reader must know before quoting anything from it:

1. **The maintainers advise against scoring these pages.** The `v1_0` README says they differ from
   the evaluation images — 380 of those are masked and `ori_pdfs` are not — and directs readers to
   `pdfs`/`images` instead.
2. **The dataset card states research use only** and carries no `license:` tag. Apache-2.0 covers
   the GitHub code, not the data. **The corpus is deliberately not vendored into this repository**,
   which is also the precedent beside this file: commit the instrument, never the corpus.
3. **34% of it is out of reach by construction, not by defect.** The `notes` family (116 pages) is
   handwritten and photographed — 9 of 116 carry any text — and 104 of `jiaocaineedrop`'s 214 pages
   are image-only scans whose entire text layer is a promotional line. Both belong outside any
   denominator rather than being scored as failures.

## Running it

```bash
cargo build --release
hf download opendatalab/OmniDocBench --revision v1_0 --include 'ori_pdfs/*' --local-dir <corpus>
python3 docs/measurements/omnidocbench/census.py <corpus>/ori_pdfs
```

About 115 s of engine time for all 981, serial; 15 s wall at eight threads.

## What it measured at 0.47.0

| | documents |
| --- | ---: |
| single page | 981 / 981 |
| text layer present | 756 (77.1%) |
| artifact produced | 961 (98.0%) |
| non-empty Markdown | 733 (74.7%) |

### Block assembly — the reason 0.47.0 exists

| | 0.46.1 | 0.47.0 |
| --- | ---: | ---: |
| median characters per block | 2.0 | **3.0** |
| median share of blocks ≤2 chars | 60% | **41%** |
| documents ≥95% such blocks | 163 of 733 | **41** |
| **share of all extracted text in those** | **52%** | **7%** |
| ≥200 chars *and* <50% tiny blocks | 277 | **351** |

`newspaper` (111 documents) is **not** fixed: a median 90% sub-3-character blocks, because CJK
inter-glyph tracking sits above the 12-centipoint epsilon by construction. See the 0.47.0 entry for
the pitch-relative epsilon that would reach it, and why it was measured and declined.

### The limitation census, which is where the defects were

Document-varying codes over the 961 documents that produced an artifact, with
`../opendataloader-bench/`'s 200 for comparison:

| code | docs | share | opendataloader-bench |
| --- | ---: | ---: | --- |
| `geometry-absent-not-groundable` | 887 | 92% | 92% |
| `non-text-nodes-not-projected` | 743 | 77% | 63% |
| `unruled-table-candidate-refused` | 717 | 75% | 100% |
| **`composite-font-codes-from-tounicode`** | **412** | **43%** | **25%** |
| `ruled-table-candidate-refused` | 399 | 42% | 29% |
| `form-xobjects-not-descended` | 256 | 27% | 23% |
| `broken-font-encoding` | 148 | 15% | 13% |
| `mcid-property-list-by-name` | 69 | 7% | — |
| `invisible-render-mode-text` | 66 | 7% | — |
| `off-page-text` | 45 | 5% | — |
| `font-widths-absent` | 36 | 4% | 1 doc |
| `stroke-ruled-table-candidate-refused` | 6 | 1% | — |
| `inline-images-not-emitted` | 3 | 0.3% | — |

**Composite fonts are ~1.7× denser here than on the previous external corpus**, which is why this
one was worth acquiring and why both defects below surfaced on it.

Groundability: **50 918 of 823 340 nodes (6.2%)** carry no measured ink box, against 8.0% on
opendataloader-bench after the 0.46.0 fix.

### Hard failures — 20 documents produce no artifact

| cause | docs |
| --- | ---: |
| predefined CJK CMap not vendored (`/GBK-EUC-H` and kin) | 12 |
| malformed ToUnicode CMap | 5 |
| no font supplied `/ToUnicode` or a mappable encoding | 3 |

Only **1.2%** die on the CJK vendoring refusal. That refusal was expected to cap this corpus and
does not: nearly every Chinese document here embeds its own ToUnicode CMap.

## What running it found, which is the point

1. **The Markdown projection shredded to a median two characters per block on untagged input** —
   fixed at 0.47.0. Invisible from inside this repository: the gate corpus shows the same *block*
   statistic (68–79% tiny) while those blocks hold only 3–7% of its text, against 52% here. The
   same number meaning opposite things on two corpora is what the census makes visible and a score
   would have hidden.
2. **A legal hex string is refused, and it costs the whole document.** PDF 32000-1 §7.3.4.3 says
   white space inside a hexadecimal string shall be ignored; `cmap.rs`'s `hex_of` requires every
   character between the brackets to be a hex digit, so `<0009 000d 0020 00a0>` and the `bfrange`
   array form `[<0066 0066 006C>…]` are read as malformed and `extract` exits 2 with no artifact.
   Confirmed on 2 of 981 — `scihub_s12935-018-0683-z.pdf_0` carries 4 165 bytes of text and a
   `table-likely` layout. **Not fixed at 0.47.0.**

**Use it as a bug-finder, not a scoreboard.** Both were found by running someone else's corpus
through this engine and reading what came out.
