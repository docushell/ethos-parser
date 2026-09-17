# Running ethos-parser against `opendataloader-bench`

The adapter, scorer and census behind the numbers below, committed so they can be re-derived rather
than believed — the same correction [`../block-subdivision/`](../block-subdivision/) makes for its
own measurement, and the one [`17-D1-SCOPE.md`](../../17-D1-SCOPE.md) and
[`18-INTERNING-SCOPE.md`](../../18-INTERNING-SCOPE.md) still need.

## This is an instrument, not a leaderboard entry

[`06-STEAL-REFUSE.md`](../../06-STEAL-REFUSE.md) O26 refuses *"#1, fastest, or any bake-off claim"*,
and [`02-ROADMAP.md`](../../02-ROADMAP.md) lists rankings under **Not scheduled**. Nothing here is a
claim to publish.

What it *is* legitimate for is the thing [`table-gate-v1.md`](../../table-gate-v1.md) §2 says a
reader would have to do to compare two parsers honestly: **run both on one corpus with one
evaluator**, neither of them yours. That is a measurement. Turning it into an ordering is the
separate act that stays refused.

**The units do not travel.** TEDS is a tree edit distance over an HTML DOM; `table-gate-v1.md`
reports macro cell-slot F1. A TEDS number from here **cannot** be placed beside 69‰ — the same
mistake six documents made about 0.489 until it was measured.

## Running it

```bash
git clone https://github.com/opendataloader-project/opendataloader-bench   # Git LFS, 200 PDFs
cd opendataloader-bench && uv sync                                          # its own venv
cargo build --release                                                       # in this repository

cp docs/measurements/opendataloader-bench/pdf_parser_ethos_parser.py <bench>/src/
# register in <bench>/src/engine_registry.py — and do NOT touch the existing `ethos` entry,
# which is the Ethos verifier CLI, a different tool:
#   ENGINES["ethos-parser"] = "0.58.0"
#   _ENGINE_MODULES["ethos-parser"] = "pdf_parser_ethos_parser"

ETHOS_BENCH=<bench> <bench>/.venv/bin/python docs/measurements/opendataloader-bench/score.py
ETHOS_PARSER_BIN=target/release/ethos-parser python3 docs/measurements/opendataloader-bench/census.py <bench>/pdfs
```

`score.py` imports the harness's three evaluators and this adapter directly, so it needs the
harness's virtualenv and not its registry; the registry entry is for the harness's own driver.
`ETHOS_PARSER_BIN` pins the binary for both scripts — without it the adapter takes
`target/release/ethos-parser` from this checkout, whatever version that build is. `score.py` prints
the band decision #18 asks for beside each macro, and every document whose ground truth holds a
table with its TEDS; `census.py` prints the limitation-code and groundability tables and needs only
the standard library.

## What it measured at 0.58.0

Measured 2026-09-16 with the `aarch64-apple-darwin` release build of 0.58.0 — the binary inside
`ethos-parser-0.58.0-aarch64-apple-darwin.tar.gz`, sha256 `03b752ed…`, `--version` 0.58.0 — the
adapter and `score.py` as committed beside this file, and the harness at its commit `7af1d8f`. The
exact commands, run from this repository's root:

```bash
ETHOS_BENCH=<bench> ETHOS_PARSER_BIN=target/aarch64-apple-darwin/release/ethos-parser \
  <bench>/.venv/bin/python docs/measurements/opendataloader-bench/score.py
ETHOS_PARSER_BIN=target/aarch64-apple-darwin/release/ethos-parser \
  python3 docs/measurements/opendataloader-bench/census.py <bench>/pdfs
```

| metric | ethos-parser | what it means here |
| --- | --- | --- |
| **NID** reading order | **0.8697** | 200/200 documents, 0 empty predictions; band 0.0068..0.9973, median 0.9238 |
| **TEDS** table structure | **0.1704** | 42 documents hold a table in ground truth: 14 non-zero, **28 at zero**, 4 above 0.9 |
| **MHS** heading hierarchy | **0.0000** | 107 documents hold a heading in ground truth; 0 of 200 carry `/StructTreeRoot` |
| speed | 36 ms/document | one run on a machine that was not quiet — load average 1.9–3.5 on 12 cores, other sessions active — and no A/B, so it is not comparable to the 0.46.0 figure or to anything else |

### The bands, with the worst document named (decision #18)

**NID 0.0068..0.9973, median 0.9238.** Worst `01030000000141`: its page is one embedded image over
a 7-byte text layer — `classify` reports `sparse-text` and `embedded-images`, and `extract` finds
two text runs, `and` and `.org` — so the 2 374 characters of its ground truth are in the picture
and not in the file's text. Best `01030000000024`, a page of body text with no image and no table.

**TEDS 0.0000..0.9802, median 0.0000, 28 of 42 at exactly zero.** Best `01030000000053`. The worst
is a tie of 28, all named. Every document whose ground truth holds a table:

| TEDS | document |
| ---: | --- |
| 0.9802 | `01030000000053` |
| 0.9761 | `01030000000052` |
| 0.9552 | `01030000000082` |
| 0.9351 | `01030000000084` |
| 0.6040 | `01030000000081` |
| 0.5828 | `01030000000083` |
| 0.4389 | `01030000000046` |
| 0.4342 | `01030000000047` |
| 0.3496 | `01030000000045` |
| 0.2857 | `01030000000121` |
| 0.2318 | `01030000000051` |
| 0.1579 | `01030000000120` |
| 0.1458 | `01030000000188` |
| 0.0802 | `01030000000127` |
| 0.0000 | the other 28: `01030000000` followed by 064, 078, 088, 089, 090, 110, 116, 117, 119, 122, 128, 130, 132, 146, 147, 149, 150, 165, 166, 170, 178, 180, 182, 187, 189, 190, 197, 200 |

The 14 non-zero documents are exactly the 14 on which `extract` emits a table — one table each —
and every one of the 14 holds a table in ground truth: no document emits a table its ground truth
lacks. The 28 at zero emit no table, and score zero for that and not for a wrong grid. The shape
the 0.46.0 record described still holds — accepted grids score, refused pages score nothing — but
the accepted set is no longer only near-perfect: ten of the 14 sit between 0.08 and 0.60, and why
each of those differs from its ground truth was not examined here.

### What moved since 0.46.0, and the reason where one is recorded

| | 0.46.0 | 0.58.0 | reason, from the CHANGELOG |
| --- | ---: | ---: | --- |
| NID | 0.8490 | **0.8697** | block assembly on untagged input changed four times: 0.47.0 joins runs along one baseline, 0.53.0 widens the reach cap by one glyph, 0.54.0 stops a cursor-moved word gap breaking a block, 0.55.0 adds the leading-gap block cut. Every document here is untagged, so all four apply; which of them moved NID was not measured per release on this corpus |
| TEDS | 0.1038 | **0.1704** | 0.55.0's `ruled-rects-v3` → `-v6`, measured there on this corpus with `rule_ab.py`: documents emitting a table 5 → 14, every one holding a table in ground truth |
| TEDS non-zero / at zero | 5 / 37 | **14 / 28** | the same. The 0.46.0 record has no per-document list, so which of its five near-perfect documents are among today's four above 0.9 cannot be said from the record |
| MHS | 0.0000 | 0.0000 | L29 stands; 0 of 200 carry a structure tree |
| speed | ~26 ms, quiet | 36 ms, loaded | not compared — the 0.46.0 note on timing applies |

### The limitation census at 0.58.0

`census.py` over all 200 documents: 200 artifacts, 14 emitting a table. Ten codes fire on every
artifact and are named rather than counted — `backend-xref-strict-20-byte`,
`form-xobject-text-not-descended`, `image-payload-not-embedded`, `low-contrast-not-detected`,
`markdown-table-spans-flattened`, `page-raster-not-emitted`, `predefined-cmaps-not-vendored`,
`reading-order-geometric-only`, `undrawn-table-edges-not-supplied` and, on this corpus only because
no document has a structure tree, `untagged-structure-tree-absent`. The document-scoped ones, beside
the 0.46.0 table above:

| code | 0.46.0 | 0.58.0 | |
| --- | ---: | ---: | --- |
| `untagged-structure-tree-absent` | 200 | 200 | why MHS is 0 |
| `unruled-table-candidate-refused` | 199 | 199 | |
| `geometry-absent-not-groundable` | 183 | 183 | **157** documents carry a text node with no ink box; on the other 26 the detail reads `0 of N text node(s)`, and the absent geometry is an image's or an annotation's (`not_applicable_to_kind`), which `non-text-nodes-not-projected` already declares |
| `non-text-nodes-not-projected` | 125 | 125 | |
| `composite-font-codes-from-tounicode` | 50 | 50 | |
| `form-xobjects-not-descended` | 46 | 46 | |
| `ruled-table-candidate-refused` | 57 | **31** | 0.55.0, two causes it records: nine more documents emit a table (5 → 14), and since `-v5` a single-row or single-column grid is no candidate, so it is neither emitted nor refused. Which of the 26 fewer refusals fell to which was not measured here |
| `broken-font-encoding` | 25 | 25 | |
| `mcid-property-list-by-name` | not in that table | 11 | a marked-content property list supplied by name through `/Properties`, which this profile does not resolve |
| `off-page-text` | not in that table | 7 | runs whose origin is outside the page's visible box, a code older than 0.46.0 (the 0.42.1 entry already names it); six of the seven carry `measured_off_page` nodes, and on `01030000000199` the one such run is whitespace, typed `no_ink_to_measure` |
| `inline-images-not-emitted` | not in that table | 4 | `BI … ID … EI` images, counted and not emitted |
| `stroke-ruled-table-candidate-refused` | not in that table | 4 | the stroke-ruled rule refused a grid the page's ruling lines implied |
| `font-widths-absent` | 1 | **0** | 0.51.0 holds the standard-14 metrics and 0.52.0 the WinAnsi glyph names. Which document carried the code at 0.46.0 was not recorded, so this reason is read off the CHANGELOG, not measured |
| `symbolic-font-builtin-encoding-assumed` | did not exist | 0 | added at 0.48.0; fires on no document here |

Whether the four codes the 0.46.0 table does not list fired then is not in the record.

| text nodes with no measured ink box | 0.46.0 | 0.58.0 |
| --- | ---: | ---: |
| of 109 500 text nodes | 8 770 (8.0%) | **8 696 (7.9%)** |
| `no_ink_to_measure` | 5 592, "whitespace runs" | 5 519 — every one whitespace-only, counted this time rather than inferred from the name |
| `measured_off_page` | 3 047 | 3 047 |
| `not_reported_by_reader` — the only reason that is this reader's | 130 | 130 |

The 74 fewer are the 74 runs the 0.58.0 CHANGELOG counts on this corpus going `no_ink_to_measure`
→ measured under the turned-text fix: text turned by its text matrix advanced 0 or less and was
typed as drawing nothing. The 0.46.0 record's three reasons sum to 8 769 against its 8 770, so one
node's reason went unrecorded then; the three above sum to 8 696.

### What this run found

One thing, recorded and not judged: `geometry-absent-not-groundable` fires on 26 documents whose
every text node has a box. The trigger is any absent geometry entry, an image's or an annotation's
included, while the detail counts text nodes and reads `0 of N` — a limitation declaring an
omission of zero nodes. Not fixed here. The census's 183 is the code's count; 157 is the count of
documents the limitation is about.

## What it measured at 0.46.0

| metric | ethos-parser | what it means here |
| --- | --- | --- |
| **NID** reading order | **0.8490** | 200/200 documents, 0 empty predictions |
| **TEDS** table structure | **0.1038** | 5 of 42 at 0.945–0.980, **37 at zero** |
| **MHS** heading hierarchy | **0.0000** | 0 of 200 documents carry `/StructTreeRoot` |
| speed | **~26 ms/document** | third fastest of fifteen engines measured |

NID was 0.8471 at 0.44.0 and 0.45.0. Timing is quoted loosely on purpose: a run taken while this
machine was loaded reported 34 ms, and an alternating A/B of the two builds over the 25 largest
documents put 0.46.0 within ~2% of 0.45.0. **Quote a speed only from a quiet machine, or A/B it.**

### The corpus is also a limitation census, and that is where the defects are

`extract` over all 200 documents, counting which limitation codes fire on how many, is the most
useful single thing this corpus produces — more useful than any of the three scores, because a
score says *how well* and the census says *where*. At 0.46.0 the document-scoped ones read:

| code | documents | |
| --- | --- | --- |
| `untagged-structure-tree-absent` | 200 (100%) | why MHS is 0, and L29 says it stays 0 |
| `unruled-table-candidate-refused` | 199 | |
| `geometry-absent-not-groundable` | **183** | 8 770 of 109 500 text nodes cannot be quoted — but only **130** for a reason that is this reader's: 5 592 are whitespace runs and 3 047 are drawn off their own page |
| `non-text-nodes-not-projected` | 125 | |
| `ruled-table-candidate-refused` | 57 | the TEDS zeros |
| `composite-font-codes-from-tounicode` | 50 | |
| `form-xobjects-not-descended` | 46 | new at 0.45.0 |
| `broken-font-encoding` | 25 | |
| `font-widths-absent` | **1** | **51 before 0.46.0**, and 50 of those were wrong |

**That census is what found the composite-font defect.** `font-widths-absent` at 51 documents
looked like an honest declaration until the files were checked: 50 of 50 had `/W` or `/DW` and the
reader was looking for `/Widths`, which §9.7.4.3 never puts on a `/Type0`. Ungroundable text nodes
fell 14 683 → 8 770 when that was repaired. See CHANGELOG "0.46.0".

### Two of the three are capped by decisions, not by effort

**MHS cannot move.** Headings come from the tag tree or nowhere
([`markdown.rs`](../../../crates/ethos-parser-core/src/markdown.rs): *"A heading is a heading
because the structure tree said so"*), and no document in this corpus has one. Scoring would mean
inferring headings from font size — checklist **L29**, refused permanently. The number is the price
of that refusal, stated.

**Reversed 2026-09-17** by North Star decision #29: a heading may be inferred from font size or
font name where a document declares no structure, as `Computed`. The 0.0000 above stands as
measured at 0.58.0 until the rule ships and MHS is re-measured.

**TEDS is the honest band, on somebody else's corpus.** 5 of 42 near-perfect, 37 at zero: where the
producer draws rules the detector is essentially exact, where they do not it emits nothing. That is
[`table-gate-v1.md`](../../table-gate-v1.md)'s bimodality reproduced on documents this repository
does not own — which is the strongest single result here, and it is a *shape*, not a score.
Decision #18 parked the geometric chase, and v2-S20 spent the one concrete lead it offered.

**NID is the one that was work**, and it was a projection defect rather than a reading-order one.

## What running it found, which is the point

Two real defects, neither visible from inside this repository:

1. **Six documents produced no artifact at all** — a measured box outside its own page, refused by
   `seal` on the stated grounds that the transform must be wrong. It was not: the content stream
   drew the text off-canvas. Fixed at 0.42.1, `GeometryAbsence::MeasuredOffPage`. NID 0.7925 →
   0.8109 from that alone.
2. **Every text run was its own block** — 68 112 blocks averaging two characters on
   `nist-sp-800-207`. Fixed at 0.44.0. NID 0.8109 → **0.8471**.
3. **Form XObjects were drawn and counted nowhere** — 46 of 200 documents draw one. Fixed at
   0.45.0. No score moved; the artifact stopped being silent, which is the point.
4. **Every composite font's widths were read from the wrong key** — 50 of 50 documents and 72 of
   72 fonts declared width-absent while the file supplied `/W` or `/DW`. Fixed at 0.46.0.
   Ungroundable text nodes 14 683 → **8 770**; NID 0.8471 → **0.8490** as a side effect.

Three of the four were invisible from inside this repository, and the fourth — the composite-font
one — was invisible to 1 294 tests, because neither owned corpus contained a single CIDFont.

**Use it as a bug-finder, not a scoreboard.** Both defects were found by running someone else's
corpus through this engine and reading what came out; neither was visible in the gate corpus,
because the gate corpus is well-tagged US federal publishing and
[`table-gate-v1.md`](../../table-gate-v1.md) §4 already says a score on it is a score on that.

## A trap this adapter fell into, recorded so it is not repeated

Before 0.44.0 the adapter rebuilt lines itself from `origin_y`/`origin_x`, to work around the
one-block-per-run projection. It lifted NID and **destroyed every table**:

| | NID | TEDS |
| --- | --- | --- |
| adapter joining lines by baseline | 0.8440 | **0.0000** |
| projection verbatim, 0.44.0 | **0.8471** | **0.1038** |

Joining by baseline flattens a GFM table's rows into ordinary lines — 50–65 pipes become none. The
engine now assembles blocks from the document's own marked-content sequences, which the adapter
could not see, and keeps the tables. **The joining variant is deleted rather than kept behind a
flag**: an adapter that second-guesses the projection measures the adapter.
