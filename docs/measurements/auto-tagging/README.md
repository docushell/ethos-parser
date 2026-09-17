# Auto-tagging, as it was measured

The instruments behind [`../../23-AUTO-TAGGING-SCOPE.md`](../../23-AUTO-TAGGING-SCOPE.md) §7,
committed so the numbers in that document can be re-derived rather than believed. One section per
instrument, each dated when it was run; the round-trip instrument of §7.1 gets its section when
the writer exists.

**These are measurement scripts, not product code.** Nothing imports them and they are not on the
gate. They read artifacts the engine emits and `qpdf --json` of the source, and never reach into
the engine.

## 1. Paragraphs: would a `/Div`, read as a paragraph, cite across a boundary the author drew? (2026-09-17)

**Run 2026-09-17**: `ethos-parser` 0.58.0 as released (the `aarch64-apple-darwin` release build),
qpdf 12.3.2, Python 3.9.6, over the eight gate documents as shipped. Instrument:
[`paragraphs.py`](paragraphs.py).

### What it measures, and why

Scope §7.2 makes decision #23's re-refusal condition measurable. #23 is re-refused by *a measured
case where a consumer treats an engine-written `/P` as author structure and produces a citation
the document does not support*. The writer emits one `/Div` per `(page, region, block)` of
`gutter-columns-v3` (§3.1), never `/P` (§3.2), and never consults an mcid — so the rate at which a
`/Div` read as a paragraph would cite across a boundary the author drew is fixed by the untouched
original, and needs no writer to measure. Two shares:

- **(i)** blocks whose `/P`-bound runs fall under two or more distinct author `/P` elements, over
  the blocks holding at least one such run. A consumer that reads the written `/Div` as a
  paragraph cites across an author's boundary in exactly these blocks.
- **(ii)** author `/P` elements whose runs fall in two or more blocks, over the `/P` elements with
  a bound run. The cut splitting a paragraph.

Each is reported per document and per page as a band with the worst page named (decision #18),
and (i) is also split by whether the block holds only `/Table`-cell paragraphs: a table's cells
share baselines, so a leading-gap cut puts a table's rows in one block by construction.

**The join.** `extract` gives every text run its page (through the page record its `parent`
names), `attributes.text_run.region` and `.block`, and a `structural_locator` — `pdf_tagged` with
the author's `mcid` and `role_path`, `pdf_artifact`, or `pdf_mcid`. `qpdf --json` gives the
structure tree; it is walked from `/StructTreeRoot` through `/K` (references, arrays, `/MCR`
dictionaries, bare integers under the nearest `/Pg`), the `/RoleMap` applied to every `/S`, page
objects mapped through qpdf's `pages` array. Joined on `(page, mcid)`, a run belongs to the
innermost `/P` above it: from the citing element upward, past inline-level elements
(PDF 32000-1 Table 338 — a `/Link` inside a paragraph is part of it), the first block-level
element is the owner, and the run is `/P`-bound when that owner is `/P`. Everything else is
excluded and counted by reason; whitespace-only runs are excluded first, as every instrument
beside this one excludes them. The instrument's docstring records where this differs from
`../block-subdivision/probe3b.py`, `labelled.py` and `structelem.py`.

**Only `nist-sp-800-207` carries labels shown to be real paragraphs.**
[`../../19-BLOCK-SUBDIVISION-SCOPE.md`](../../19-BLOCK-SUBDIVISION-SCOPE.md) §9.1 measured 3.63
lines per `/P` on it and one `/P` per line on the other producers. The same join runs on the
other seven gate documents below, and their two shares describe how those producers write `/P`,
not paragraph recall.

### Running it

```bash
M=docs/measurements/auto-tagging
B=target/aarch64-apple-darwin/release/ethos-parser        # 0.58.0
mkdir -p /tmp/para
for f in nist-sp-800-207 irs-f1040sd-2025 irs-fw9 nist-sp-800-218 nist-sp-800-171r3 \
         nist-sp-800-37r2 nist-sp-800-161r1 nist-sp-800-53Ar5; do
  $B extract fixtures/gate/$f.pdf > /tmp/para/$f.json
done
# The labelled document, with the shipped cut re-scored on probe3b.py's own rows:
python3 $M/paragraphs.py --probe3b /tmp/para/nist-sp-800-207.json fixtures/gate/nist-sp-800-207.pdf
# All eight: one report each, then the summary table and the band across documents:
args=(); for f in nist-sp-800-207 irs-f1040sd-2025 irs-fw9 nist-sp-800-218 nist-sp-800-171r3 \
                  nist-sp-800-37r2 nist-sp-800-161r1 nist-sp-800-53Ar5; do
  args+=(/tmp/para/$f.json fixtures/gate/$f.pdf); done
python3 $M/paragraphs.py "${args[@]}"
```

`paragraphs.py` runs `qpdf --json` itself (`QPDF=` names the executable) and streams the extract,
so the 997 MB artifact of `nist-sp-800-53Ar5` fits: the eight-document run took 38.0 s and peaked
at 1.74 GB resident; `nist-sp-800-207` alone with `--probe3b` takes 2.9 s.

### What it found on `nist-sp-800-207`

**The tree.** 1,226 elements. 448 `/P` after the `/RoleMap`, 41 of them `/S /Artifact` that the
document's own `/RoleMap` maps to `/P`; they cite one `(page, mcid)` each and no node of the
representation carries any of those 41, so none binds a run. 965 citations in all, 4 duplicated
(page 19 mcid 6 under two `/LBody`, page 52 mcid 0 under two `/P`; the first in tree order is
kept), 172 `/OBJR` skipped, no `/P` nested in a `/P`.

**The runs.** 90,817 text runs; 63,306 bound to a `/P`, 1,165 of them through a `/Link` or a
`/Reference` inside it. 27,511 excluded:

| reason | runs |
| --- | ---: |
| under `/LBody` — list item text | 15,313 |
| whitespace-only | 5,108 |
| `pdf_artifact` | 4,089 |
| under `/TOCI` | 2,053 |
| under `/H2`, `/H3`, `/H1`, `/H6` | 506, 205, 129, 31 |
| `pdf_mcid` — an id the tree does not cite | 62 |
| under `/Figure` | 15 |

On all 81,558 tagged runs the walk's owner role equals what the engine's `role_path` gives under
the same nearest-block-level rule, so the join reads the tree the way the engine does. Attributing
a run through its inline parent changes nothing below: direct citations alone give the same 23 of
259 and 12 of 343.

**(i) — 23 of 259 blocks hold two or more author `/P`: 8.9%.**

| blocks holding a `/P`-bound run | blocks | mixed | share | author boundaries inside them |
| --- | ---: | ---: | ---: | ---: |
| holding a `/P` outside any `/Table` | 219 | 11 | 5.0% | 27 |
| holding only `/Table`-cell `/P` | 40 | 12 | 30.0% | 69 |
| all | 259 | 23 | 8.9% | 96 |

Per page the share runs **0.0% .. 100.0%, median 0.0%**; the two pages at 100% are page 36
(1 of 1: the rule declined its one band, and 3 `/P` share it) and page 54 (1 of 1: a table whose
44 cell paragraphs are one block, region 2 block 2). The worst pages by count:

| page | mixed / blocks | `/P` on the page | its worst block |
| ---: | ---: | ---: | --- |
| 51 | 4 / 12 | 20 | region 2, block 3: 6 `/P`, a table |
| 59 | 3 / 5 | 12 | region 2, block 6: 6 `/P`, the references table |
| 1 | 2 / 4 | 8 | region 2, block 7: 4 `/P` |
| 2 | 2 / 4 | 17 | region None, block 4: 11 `/P` |
| 3 | 2 / 8 | 11 | region None, block 7: 3 `/P` |

The 11 prose blocks are on pages 1, 2, 3 (six blocks, 20 of the 27 crossed boundaries: the title
and authority pages, whose paragraphs are one line each; page 2's block 4 is the author block,
whose paragraphs are separated by a whitespace-only run the cut counts as a line, so no gap in it
reaches 1.6×), 11, 20, 37, 44 (one each, 2 to 3 `/P`) and 36 (the declined band). Of the 23, 22
are numbered blocks and 1 is `None`.

**(ii) — 12 of 343 `/P` elements fall in two or more blocks: 3.5%, and all 12 cross a page
break.** Within a page the cut never splits a paragraph: 0 of 343. One of the 12 is a table cell.
Per page 0.0% .. 50.0%, median 0.0%; the maximum is page 34 (2 of its 4 `/P` continue from or onto
another page), then pages 39 and 21 (2 each), 29 and 30 (1 each).

**(iv)** Of the 259 blocks, 258 are numbered and 1 is `None` — the rule declined page 36's band.
13 have region `None`: pages 2, 3 and 36, where the vertical cut made no division.

Lines per `/P` element, distinct baselines among its bound runs: mean 4.12, median 4, one-line
`/P` 37.9%.

### The cross-check against `blocks.rs`'s 63.7% recall at 100% precision

The units differ, and the differences are the whole story. `blocks.rs`'s figure is over
consecutive **line pairs** of a band: a boundary is two consecutive lines under different `/P`,
recall is the boundaries where the simulated gap rule fires. Share (i) is over **blocks**: `k`
consecutive missed boundaries are one block holding `k + 1` paragraphs, so 23 blocks carry 96
author boundaries, 69 of them in table blocks (page 54's alone carries 43), and a consumer citing
one such `/Div` crosses all of that block's boundaries at once. Share (ii) is over **elements**
and is the precision side: a wrong fire splits a `/P`, and 100% precision on line pairs is 0 of
343 elements split within a page here. The 12 split elements are page breaks, which a per-page
line-pair instrument never sees.

The recall itself fell out of the same join, twice, with the shipped cut as the rule — `block`
changing between consecutive lines is a fire:

| rows | boundaries | found | recall | mid-paragraph pairs | cut | precision |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `probe3b.py` at 0.58.0, its own simulation of 1.6× | 135 | 86 | 63.7% | 719 | 0 | 100% |
| `probe3b.py`'s own rows, the shipped cut (`--probe3b`) | 135 | 79 | **58.5%** | 719 | 0 | 100% |
| this instrument's rows, every raw `/P`, the shipped cut | 189 | 129 | **68.3%** | 1,022 | 0 | 100% |

`probe3b.py` re-run at 0.58.0 reproduces §11.2b's 135, 719 and 63.7% exactly. On its own 135
boundaries the shipped cut finds 79. The seven it misses and the simulation finds are all
explained by lines the engine sees and the instrument's rows do not: **5 have a whitespace-only
run between the two lines** (four on page 2, one on page 59 — Word writes an empty paragraph as
a `" "` run, `subdivide` takes every run's baseline, and one 2.0× gap becomes two of 1.0×), and
**2 are on page 36, whose band the rule declined**. Page 36 is the one page where the vertical cut
made no division, so the 86 characters of the vertical `/Artifact` margin note share the body's
band: 69 lines, modal gap 1,380 centipoints, mode share 0.12 against the guard's 0.25. On pages
35 and 37 the note has its own region (declined on its own, modal gap 500) and the body band
passes at 0.68 and 0.51. 373 of the document's 5,108 whitespace-only runs sit on a baseline no
visible run shares.

The third row is the same filters over every raw `/P`. `structelem.py`, which `probe3b.py`
imports, reads the QDF dump with a pattern whose negative lookbehind refuses a digit preceded by a
space, and `qpdf --qdf` prints every array member on its own indented line — so it keeps only the
314 `/P` elements whose `/K` is one bare integer, of 407, and drops a paragraph with a link in it
whole rather than dropping the link's line as its docstring says. The published population is
that subset. Over all 407 the shipped cut finds 129 of 189 boundaries and cuts none of 1,022
mid-paragraph pairs.

So: **63.7% is the rule simulated on cleaned lines over the 314 single-sequence paragraphs;
the shipped rule on the engine's own lines scores 58.5% on those rows and 68.3% on every
paragraph; precision is 100% on all three.** Nothing here moves what `blocks.rs` says stands —
one document carries no threshold — and it does not re-derive the recall so much as bound it from
both sides.

### The other seven gate documents

All eight are tagged, and the same join runs on each. **Only the first row is paragraph recall.**
On the other seven a `/P` is what the producer wrote — one per line on four of them by §9.1's
measurement, and the lines-per-`/P` column here says the same — so a "mixed" block there is a
block holding several lines, which is what a block is meant to hold.

| document | pages | `/P` bound | runs bound | lines per `/P` mean (one-line) | (i) mixed blocks | (i) outside tables | (i) table-only | (ii) split `/P` | blocks `None` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `nist-sp-800-207` | 59 | 343 | 63,306 | 4.12 (37.9%) | 23 / 259 — 8.9% | 11 / 219 — 5.0% | 12 / 40 — 30.0% | 12 / 343 — 3.5% | 1 |
| `irs-f1040sd-2025` | 2 | 101 | 798 | 1.59 (69.3%) | 10 / 15 — 66.7% | 10 / 15 — 66.7% | 0 / 0 | 0 / 101 — 0.0% | 1 |
| `irs-fw9` | 6 | 213 | 779 | 2.70 (38.5%) | 18 / 36 — 50.0% | 16 / 34 — 47.1% | 2 / 2 — 100% | 0 / 213 — 0.0% | 1 |
| `nist-sp-800-218` | 36 | 1,120 | 52,349 | 1.77 (68.6%) | 45 / 65 — 69.2% | 23 / 33 — 69.7% | 22 / 32 — 68.8% | 3 / 1,120 — 0.3% | 26 |
| `nist-sp-800-171r3` | 120 | 2,107 | 112,288 | 1.78 (79.8%) | 321 / 547 — 58.7% | 278 / 499 — 55.7% | 43 / 48 — 89.6% | 4 / 2,107 — 0.2% | 4 |
| `nist-sp-800-37r2` | 183 | 2,118 | 273,408 | 2.90 (52.7%) | 250 / 440 — 56.8% | 207 / 397 — 52.1% | 43 / 43 — 100% | 8 / 2,118 — 0.4% | 4 |
| `nist-sp-800-161r1` | 327 | 4,673 | 380,585 | 2.35 (62.9%) | 349 / 1,151 — 30.3% | 172 / 917 — 18.8% | 177 / 234 — 75.6% | 76 / 4,673 — 1.6% | 86 |
| `nist-sp-800-53Ar5` | 733 | 20,888 | 1,455,284 | 4.21 (67.9%) | 8,930 / 10,923 — 81.8% | 8,895 / 10,884 — 81.7% | 35 / 39 — 89.7% | 22 / 20,888 — 0.1% | 5 |

**Band across the eight, (i): 8.9% (`nist-sp-800-207`) .. 81.8% (`nist-sp-800-53Ar5`), median
58.7% (`nist-sp-800-171r3`). (ii): 0.0% (`irs-f1040sd-2025`, `irs-fw9`) .. 3.5%
(`nist-sp-800-207`), median 0.3% (`nist-sp-800-218`).** The worst page of each:

| document | worst page for (i) | largest block on that page | pages at 100% |
| --- | --- | --- | ---: |
| `nist-sp-800-207` | 51: 4 of 12 blocks | 6 `/P` | 2 |
| `irs-f1040sd-2025` | 2: 9 of 14 | 5 `/P`; page 1 is one `None` block holding all 69 `/P` | 1 |
| `irs-fw9` | 3: 6 of 13 | 12 `/P`; page 1 is one `None` block holding all 46 `/P` | 1 |
| `nist-sp-800-218` | 20: 3 of 5 | 54 `/P` | 28 |
| `nist-sp-800-171r3` | 100: 12 of 12 | 2 `/P` | 21 |
| `nist-sp-800-37r2` | 134: 4 of 5 | 2 `/P` | 101 |
| `nist-sp-800-161r1` | 318: 11 of 17 | 6 `/P` | 147 |
| `nist-sp-800-53Ar5` | 228: 21 of 23 | 2 `/P` | 36 |

What the other rows say about their producers, since the numbers are there: `nist-sp-800-53Ar5`
nests a `/P` inside a `/P` 31,468 times (`nist-sp-800-37r2` 7 times, `nist-sp-800-161r1` once;
the innermost is taken, as the instrument says). Four documents map `/S /Artifact` to `/P`
through their `/RoleMap` — 328 elements on `nist-sp-800-218`, 636 on `-37r2`, 18 on `-161r1`, 54
on `-53Ar5` — and on none of the eight does such an element bind a text run. `(page, mcid)` pairs
cited by two elements: 12 on `-171r3`, 6 on `-37r2`, 16 on `-161r1`, 4 on `-53Ar5`. The split
`/P` that are not page breaks — 1 on `-218`, 4 on `-171r3`, 9 on `-161r1`, 2 on `-53Ar5` — are
elements a producer wrapped around more than a paragraph: `nist-sp-800-171r3`'s pages 93, 94 and
101 each hold one `/P` and 19, 17 and 13 blocks. The same line-pair check on these rows scores the
cut at 7.7% (`irs-fw9`) to 83.3% (`nist-sp-800-53Ar5`) and cuts 56 of `-171r3`'s 1,561
same-`/P` pairs; those are per-line `/P` and page-wrapping `/P`, not paragraphs, and are not
quoted as recall or precision.

### What it does not measure

- **The consumer demonstration.** §7.2's first half — `ethos-parser 0.58.0 extract` on the
  writer's output binding every engine-written `/Div` as author structure — needs the writer, and
  is the round-trip instrument's to run.
- **Paragraph recall on any document but `nist-sp-800-207`.** The seven other rows are producer
  behaviour, stated above.
- **Indent-marked paragraphs.** `blocks.rs` declares the rule blind to them; `nist-sp-800-207` is
  space-marked US federal publishing, so the 8.9% is the space-marked case only.
- **What the 41 `/Artifact`-mapped `/P` elements cite.** No node carries their mcids and the
  `pdf_artifact` locator carries none, so the artifact cannot say.
- **The writer's own boundaries.** That every written `/Div` is exactly one
  `(page, region, block)` is §3.7's check on the writer's output, not this instrument's.

### Found on the way, not fixed

1. **`structelem.py` cannot see an mcid inside a `/K` array**, so `probe3b.py`'s population is
   the 314 single-sequence `/P`, not the 407, and its stated exclusion (a link's line) is wider
   than stated (the whole paragraph). §11.2b's numbers reproduce exactly on the same instrument;
   they are a floor, as its docstring already says, and this is why.
2. **A whitespace-only run is a line to the block cut.** 373 of `nist-sp-800-207`'s 5,108 sit on
   their own baseline, and five of the seven boundaries the simulation finds and the shipped cut
   misses are Word's empty paragraphs halving a gap.
3. **A vertical artifact note declines a band it shares.** Page 36 of `nist-sp-800-207` is the one
   band the rule declined, and the reason is the margin note's 86 per-character baselines in a
   band the vertical cut did not separate.

Each is recorded here as a measurement; none is a decision.
