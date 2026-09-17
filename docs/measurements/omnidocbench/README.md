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

**The benchmark's own end2end evaluation** is a second instrument beside this one, run through
[`end2end.py`](end2end.py):

```bash
python3 docs/measurements/omnidocbench/end2end.py <work-dir>
```

It scores **their** metrics against **their** ground truth, so unlike the census the numbers are
comparable to what other projects publish — which is exactly why decision **O26** refuses to
publish a row from it. §"Their metrics, across three releases" below is the record.

## What it measured after 0.58.0 — 2026-09-18

**Re-run on the build at `a18b2b0`** (`--version` 0.58.0; the work that ships as 0.59.0), 981
documents in **13 s wall**. The corpus was re-fetched because it is not kept on this machine: no
`hf` CLI here, so the dataset's own tree API was read and each file pulled from `resolve/v1_0`,
giving **981 files, 538 521 457 bytes** — the size this page already records for that commit, byte
for byte. Nothing is mirrored into the tree.

| | 0.52.0 | this build |
| --- | ---: | ---: |
| single page | 981 / 981 | **981 / 981** |
| text layer present | 756 (77.1%) | **756 (77.1%)** |
| artifact produced | 963 (98.2%) | **962 (98.1%)** |
| non-empty Markdown | 735 (74.9%) | **735 (74.9%)** |

**One document fewer produces an artifact, and that is this session's fail-closed guard firing on
its first corpus example.** `jiaocaineedrop_chap10.pdf_8.pdf` is refused: *unsupported content
stream tokeniser: page 1: byte 3 of 125718: the operation starting there cannot be parsed past byte
4, and lopdf's decoder stops at the same place and drops the remaining 125715 byte(s) without a
word.* Measured with both binaries on this file: 0.58.0 as released exits **0** and writes a
14 961-byte artifact whose two nodes carry **no text at all** — a complete-looking read of a page
whose 125 718-byte content stream was dropped after its fourth byte — and this build exits **2**
with no artifact. Until now that defect had unit tests and no corpus example; this is the example,
and it is a document whose artifact used to read as an empty page.

### Block assembly, four releases on

| | 0.46.1 | 0.47.0 | 0.52.0 | this build |
| --- | ---: | ---: | ---: | ---: |
| median characters per block | 2.0 | 3.0 | — | **4.0** |
| median share of blocks ≤2 chars | 60% | 41% | — | **31%** |
| documents ≥95% such blocks | 163 of 733 | 41 | — | **38**, holding 7% of all text |
| documents ≥50% such blocks | — | — | — | **256**, holding 55% of all text |

The 0.52.0 record printed its own block table for 0.46.1 and 0.47.0 only; the two columns it left
empty are left empty rather than back-filled from a run it did not make.

### The limitation census on this build

Document-varying codes over the 962 documents that produced an artifact:

| code | docs | share | 0.52.0 |
| --- | ---: | ---: | ---: |
| `geometry-absent-not-groundable` | 855 | 89% | 884 (92%) |
| `non-text-nodes-not-projected` | 744 | 77% | 745 (77%) |
| `unruled-table-candidate-refused` | 719 | 75% | 719 (75%) |
| `composite-font-codes-from-tounicode` | 412 | 43% | 412 (43%) |
| `ruled-table-candidate-refused` | 292 | 30% | 399 (41%) |
| `form-xobjects-not-descended` | 256 | 27% | 256 (27%) |
| `broken-font-encoding` | 149 | 15% | 149 (15%) |
| `mcid-property-list-by-name` | 70 | 7% | 70 (7%) |
| `invisible-render-mode-text` | 66 | 7% | 66 (7%) |
| `off-page-text` | 44 | 5% | 44 (5%) |
| `symbolic-font-builtin-encoding-assumed` | 36 | 4% | 36 (4%) |
| `font-widths-absent` | 7 | 1% | 7 (1%) |
| `stroke-ruled-table-candidate-refused` | 6 | 1% | 6 (1%) |
| `inline-images-not-emitted` | 3 | 0% | 3 (0.3%) |

**Two codes moved and the rest did not.** `ruled-table-candidate-refused` falls 399 → 292: 0.55.0's
`ruled-rects-v5` made a single-row or single-column grid no candidate at all, so those pages are
neither emitted nor refused — the same cause `../opendataloader-bench/` records for its own 57 → 31.
`geometry-absent-not-groundable` falls 884 → 855, and **this session's work accounts for exactly
one of those**: run over this corpus with both binaries, the build before B1–B9 carries the code on
856 of its 963 artifacts and this build on 855 of 962 — the difference is the document it now
refuses, which produces no artifact to carry anything. The other 28 fell between 0.52.0 and 0.58.0,
where the releases that made 10 698 more nodes citable are the candidate; this page does not
attribute it further, because doing so needs those builds re-run. This session did change what that
declaration *says* where no text node lacks a box — it states the by-kind population instead of
*0 of N text node(s)* — but not when it fires, which is why the count barely moves.

Groundability on this build: **794 890 of 824 218 nodes (96.4%)** carry a measured ink box, and
**29 328 (3.6%)** carry none. Against 0.52.0's 784 192 / 40 028 of 824 220: **10 698 more nodes are
citable**, and the total is two nodes lower, which is consistent with the refused document's old
artifact having carried exactly two.

### Hard failures — 19 documents produce no artifact

| cause | docs |
| --- | ---: |
| unreadable `/Encoding /Identity-H` | 8 |
| unreadable `/Encoding /GBK-EUC-H` (a predefined CJK CMap, not vendored) | 4 |
| unsupported text encoding: the text layer is unreadable | 3 |
| malformed `ToUnicode` CMap: `bfrange` destination runs past the Unicode range | 2 |
| malformed `ToUnicode` CMap: destination is not valid UTF-16 | 1 |
| **unsupported content stream tokeniser** (new on this build, above) | 1 |

The first five causes are the 18 the 0.52.0 record names, unchanged in kind and in count. Decision
#22's reading still holds for the first: `/Identity-H` is the identity per §9.7.4.2, so what is
missing is CID-to-Unicode and vendoring the Adobe CJK set fixes the four `GBK-EUC-H` documents, not
the eight.

## What it measured at 0.52.0

| | documents |
| --- | ---: |
| single page | 981 / 981 |
| text layer present | 756 (77.1%) |
| artifact produced | 963 (98.2%) |
| non-empty Markdown | 735 (74.9%) |

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

Document-varying codes over the 963 documents that produced an artifact, with
`../opendataloader-bench/`'s 200 for comparison. **This line said 961 until 0.52.0 and the
instrument said 963**, one of three stale numbers in the 0.50.0 record found by re-running it — see
the note under the groundability figure:

| code | docs | share | opendataloader-bench |
| --- | ---: | ---: | --- |
| `geometry-absent-not-groundable` | 884 | 92% | 92% |
| `non-text-nodes-not-projected` | 745 | 77% | 63% |
| `unruled-table-candidate-refused` | 719 | 75% | 100% |
| **`composite-font-codes-from-tounicode`** | **412** | **43%** | **25%** |
| `ruled-table-candidate-refused` | 399 | 41% | 29% |
| `form-xobjects-not-descended` | 256 | 27% | 23% |
| `broken-font-encoding` | 149 | 15% | 13% |
| `mcid-property-list-by-name` | 70 | 7% | — |
| `invisible-render-mode-text` | 66 | 7% | — |
| `off-page-text` | 44 | 5% | — |
| **`symbolic-font-builtin-encoding-assumed`** | **36** | **4%** | new at 0.48.0 |
| **`font-widths-absent`** | **7** | **1%** | 1 doc |
| `stroke-ruled-table-candidate-refused` | 6 | 1% | — |
| `inline-images-not-emitted` | 3 | 0.3% | — |

**Composite fonts are ~1.7× denser here than on the previous external corpus**, which is why this
one was worth acquiring and why both defects below surfaced on it.

Groundability: **40 028 of 824 220 nodes (4.9%)** carry no measured ink box, against 8.0% on
opendataloader-bench after the 0.46.0 fix.

**Measured before and after on this corpus, same instrument, by rebuilding 0.50.0 in a worktree:**

| | 0.50.0 | 0.52.0 |
| --- | ---: | ---: |
| measured ink box | 773 237 (93.8%) | **784 192 (95.1%)** |
| no box — omitted from `ethos.grounding.v1` | 50 983 (6.2%) | **40 028 (4.9%)** |
| total nodes | 824 220 | **824 220** |

**10 955 nodes recovered**, by decision #22's vendored Core-14 metrics (0.51.0) and the derived
glyph-name table (0.52.0). The total is **identical on both sides**: no text is gained or lost, only
whether it can be cited. The figure matches the 37-document measurements those releases were built
on — 10 948 + 7 — to the node.

**Three numbers in the 0.50.0 record were stale and are corrected here.** It said *20 documents
produce no artifact* where its own table summed to 18 and its own "artifact produced 963" implied
18; it said the census covered *961* documents where the instrument said 963; and it said *823 340*
nodes where the instrument says 824 220. None came from a behaviour change — the prose around the
tables was written by hand and not regenerated, which is the drift the instrument printing its own
numbers was meant to stop. It stopped the tables and not the sentences.

### Hard failures — 18 documents produce no artifact

| cause | docs |
| --- | ---: |
| `/Identity-H` with no `/ToUnicode` | 8 |
| predefined CJK CMap `/GBK-EUC-H` not vendored | 4 |
| no font supplied `/ToUnicode` or a mappable encoding | 3 |
| malformed ToUnicode CMap | 3 |

**Vendoring the Adobe predefined CJK CMaps would fix 4 of these, not all 12 as the
`predefined-cmaps-not-vendored` limitation implies.** `/Identity-H` is not a predefined CJK CMap:
§9.7.4.2 makes it the identity, so the code *is* the CID, and what is missing is CID-to-Unicode.
0.50.0 corrected the engine's own message, which had said otherwise. Two malformed-ToUnicode
documents were recovered at 0.48.0 by reading white space inside a hex string as §7.3.4.3 requires.

## What running it found, which is the point

1. **The Markdown projection shredded to a median two characters per block on untagged input** —
   fixed at 0.47.0. Invisible from inside this repository: the gate corpus shows the same *block*
   statistic (68–79% tiny) while those blocks hold only 3–7% of its text, against 52% here. The
   same number meaning opposite things on two corpora is what the census makes visible and a score
   would have hidden.
2. **A legal hex string was refused, and it cost the whole document.** §7.3.4.3 says white space
   inside a hexadecimal string shall be ignored; `hex_of` required every character between the
   brackets to be a hex digit. Two documents of 981; `scihub_s12935-018-0683-z.pdf_0` recovered
   4 970 characters from nothing. **Fixed at 0.48.0.**
3. **A symbolic font was decoded through `StandardEncoding` in silence.** §9.6.6.2 gives that
   fallback to a NONSYMBOLIC font; applied to a symbolic one, CMEX10 code 90 arrives as `Z`. The
   decode is unchanged — 42 documents carry such a font and the flag does not separate the TeX
   math fonts from ordinary prose that merely sets the bit — but the artifact now declares it on
   36 documents. **Declared at 0.48.0.**
4. **One refusal answered for two different absences**, sending a reader after data that could not
   help. **Fixed at 0.50.0**, and this instrument's own failure labels were corrected with it.

**Use it as a bug-finder, not a scoreboard.** Both were found by running someone else's corpus
through this engine and reading what came out.

---

## Their metrics, across three releases

**Run privately, for diagnosis, and not quotable** — decision **O26**, and the corpus is
research-use-only. Recorded here because the working directory that produced them was destroyed
twice in one day and re-deriving it cost an afternoon each time.

`Edit_dist` is **lower-is-better**; `TEDS` **higher-is-better**. All three runs used the same
harness, corpus and filtered ground truth — only the engine changed.

| Module | Metric | 0.52.0 | 0.53.0 | 0.54.0 |
| --- | --- | ---: | ---: | ---: |
| **text_block** | Edit_dist | 0.5931 | 0.5674 | **0.4845** |
| **reading_order** | Edit_dist | 0.5916 | 0.5758 | **0.5314** |
| display_formula | Edit_dist | 0.9812 | 0.9696 | 0.9616 |
| table | TEDS | 0.0111 | 0.0111 | 0.0139 |
| table | TEDS structure | 0.0162 | 0.0162 | 0.0202 |

### The average is the wrong statistic, and the band says why

**Of the 921 pages the harness scores** — 60 of the 981 carry no scorable text block, per the
filtered-ground-truth note below — **684 yield something to compare and 237 score exactly 1.0** in
every run. There is no OCR, by decision #11, so a scan yields nothing to compare, and averaging
those in describes neither population.

**The denominator is 921, not 981**, and this sentence said 981 until 0.54.0: 684 + 237 = 921, and
981 - 684 = 297, which is nobody's number. The census table above counts a different thing again —
**756 pages carry text objects** (`census.py`, `pages_with_text > 0`) — and the 72-page gap against
684 is not a contradiction: a page can carry text objects and still score 1.0 when what it yields is
unusable. Three populations, three denominators; each is now named where it is used.

| Scored pages that yield a comparison (684 of 921) | 0.52.0 | 0.53.0 | 0.54.0 |
| --- | ---: | ---: | ---: |
| mean | 0.4521 | 0.4175 | **0.3059** |
| **median** | 0.2763 | 0.2111 | **0.0786** |
| near-perfect, < 0.10 | 244 (35.7%) | 274 (40.1%) | **365 (53.4%)** |
| failing, ≥ 0.75 | 259 (37.9%) | 238 (34.8%) | **162 (23.7%)** |

**The median page went 0.2763 → 0.0786 across two releases**, and more than half of all
text-bearing pages are now near-perfect.

### What moved it

**0.53.0** — `ink_reach` capped a run's reach at the font's **median** advance per glyph, so half
of all runs were truncated by construction and a 12-centipoint epsilon then read a gap that was not
there. Words split mid-token.

**0.54.0** — a word gap the page opened by **moving the cursor** rather than drawing a space glyph
was measured twice: the reader wrote the space into the run's text, and the block rule then
measured the same gap again and called it a break. Pages emitted one word per block. This is the
larger of the two by a wide margin.

Neither introduced a threshold. A word-space gap epsilon was refused at 0.47.0 and re-measured over
**749 409 same-baseline pairs**: for Latin pairs the distribution decays monotonically from its
word-space mode with no empty band, so any ceiling would be a tuned knob. There is none.

### Where the remaining failures are, which is the finding to revisit

| 162 pages still failing, by class | 0.52.0 | 0.54.0 |
| --- | ---: | ---: |
| English | 118 | **26** |
| Chinese / mixed | 141 | **136** |
| — `newspaper` alone | 69 | **69** |
| — academic literature, English | 84 | **13** |

**The English half is 78% solved and the CJK half has barely moved.** That is not a shortfall, it
is the mechanism: both fixes key on spaces the reader synthesized from cursor moves, and CJK draws
no word spaces, so no join is ever offered. `newspaper` alone is 69 of the 162.

**A hypothesis for whoever picks this up, untested.** Latin needs a rule that distinguishes a word
space from a column jump, and the measurement above says no such boundary exists in the data. CJK
has *no word boundaries at all* — every same-line gap is tracking — so the question may collapse
from *"is this a space?"* to *"is this a column jump?"*, which is strictly easier and may not need
the trough Latin lacks. Measure before building.

### Read the numbers with these

- **`Overall` is not computed.** Its formula needs CDM, which needs TeX Live, Ghostscript and
  ImageMagick, and OmniDocBench's own `pyproject.toml` calls for Linux. Only the text term exists:
  `(1 − 0.4845) × 100 = 51.6` at 0.54.0, against 40.7 at 0.52.0.
- **981 of the benchmark's 1 651 ground-truth pages.** Only the `v1_0` subset ships as PDFs, and
  the ground truth is filtered to pages predictions exist for — scoring the full set would report
  670 missing predictions as total failures.
- **Table figures are a side effect, not a detection change.** Nothing in 0.53.0 or 0.54.0 goes
  near table detection. OmniDocBench lifts `<table>` out of a prediction and matches its text, so
  better-assembled blocks match more often. Do not read 0.0111 → 0.0139 as tables improving.
- **921 of 981 pages carried scorable text blocks**; 60 had none.
