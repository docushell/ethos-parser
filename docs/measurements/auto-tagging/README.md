# Auto-tagging, as it was measured

The instruments behind [`../../23-AUTO-TAGGING-SCOPE.md`](../../23-AUTO-TAGGING-SCOPE.md) §7,
committed so the numbers in that document can be re-derived rather than believed. One section per
instrument, each dated when it was run.

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
  is the round-trip instrument's to run (§2).
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

## 2. The round trip: does a written tag read back, project and ground as its original does? (2026-09-17)

**Run 2026-09-17**: the writer at `2e70eba` (the `feat/auto-tagging` branch: S1, S2 with both
reviews' fixes, and S3), release build, version string `0.58.0`; the consumer is `ethos-parser`
0.58.0 as released (the `aarch64-apple-darwin` release build); Ethos 0.6.0, the oracle pin;
qpdf 12.3.2; Python 3.9.6; four jobs. Over 293 documents — the 58 engine fixtures, the 35 oracle
fixtures and the 200 `opendataloader-bench` documents — and the eight gate documents for the wire
cost. Instrument: [`roundtrip.py`](roundtrip.py); every document's record is in
[`results/roundtrip.jsonl`](results/roundtrip.jsonl) and the aggregates in
[`results/roundtrip-summary.json`](results/roundtrip-summary.json). **Re-run the same day at
`0c7a3c9`**, after the review of the writer's numbering and removal fixes: every per-document
record is identical to the committed ones apart from wall-clock seconds, so the results stand for
the writer as merged.

### What it measures, and why

Scope §7.1 lists what the round trip counts, and §7.2's first half is the consumer demonstration.
Per document, with the writer's build: `tag`, a refusal bucketed onto the row of scope §3.6 its
message names; the written tree through `qpdf --json` (elements, sequences, sequences per block,
the stamp); a second `tag`, compared byte for byte with the first; `extract` of the tagged bytes
and of the original, then `ground`, `markdown` and `html` of both, compared as canonical JSON once
the members naming the input bytes are removed (`source_sha256`, `representation_sha256`,
`source.sha256`, the assurance blocks); `grounding-check --source-artifact` on the tagged grounding
artifact; `ethos verify` on one claim quoting the first bound run of the first block, against the
tagged and the untagged grounding artifact, each claims file carrying its own artifact's digest;
and the release binary's `extract` of the tagged bytes. Every original is also scanned for reals
outside content streams that do not survive `f32`. The docstring states each rule and what it
cannot see.

### Running it

```bash
M=docs/measurements/auto-tagging
cargo build --release --locked -p ethos-parser-cli
python3 $M/roundtrip.py \
  --branch target/release/ethos-parser \
  --release <the 0.58.0 release binary> \
  --ethos ../ethos-oracle/target/release/ethos \
  --out /tmp/roundtrip --results $M/results --commit "$(git rev-parse --short HEAD)" \
  engine=fixtures/engine oracle=../ethos-oracle/fixtures \
  bench=<opendataloader-bench>/pdfs gate=fixtures/gate
```

The 293 documents took 60.6 s of document work at four jobs; the gate's sixteen extractions,
branch and release, run one at a time after them. Artifacts stay under `--out` and are never
committed; the committed results name binaries and corpora relative to this repository, the
oracle checkout and the bench clone rather than by this machine's paths.

### What it found

**129 of 293 documents are tagged, and every one reads back as written.** The self-check refused
none. The writer's `extract` of the 129 outputs binds 41,208 of their 41,209 runs `pdf_tagged`,
`derivation: computed`, `Document/Div`; the other run is `untagged-artifact-furniture`'s page
furniture, still `pdf_artifact`. `structure-tree-engine-written` is declared on all 129 and
`untagged-structure-tree-absent` on none.

| outcome | §3.6 row | bench | engine | oracle | all |
| --- | --- | ---: | ---: | ---: | ---: |
| tagged | - | 54 | 44 | 31 | 129 |
| refused: marked-content ids inline, no tree | row 2 | 98 | 1 | 0 | 99 |
| refused: `/StructParents` or `/StructParent`, no tree | §3.6, amended | 48 | 0 | 0 | 48 |
| refused: a tree is present | row 1 | 0 | 12 | 0 | 12 |
| refused: not a PDF this engine opens | - | 0 | 0 | 2 | 2 |
| refused: an id by name through `/Properties` | row 3 | 0 | 1 | 0 | 1 |
| refused: no text run to tag | §3.4 | 0 | 0 | 1 | 1 |
| refused: encrypted | row 4 | 0 | 0 | 1 | 1 |
| documents | - | 200 | 58 | 35 | 293 |

**The refusals are one producer's shape.** All 146 refused bench documents are PyPDF2 output:
pages split out of tagged documents, each keeping its marked-content ids and losing the tree. The
48 counted under the stale-key refusal carry inline ids too; that check runs before extraction and
row 2's after it, so they land in the earlier row, and before the check existed all 147 of that
shape (the 146 and the engine fixture `untagged-mcid-no-tree`) were row 2. The 54 bench documents
tagged are 41 PyPDF2, 12 iLovePDF and 1 Adobe PDF Library. No document anywhere was refused for a
filter or a stream that does not decode to its end (row 5), a tokeniser disagreement (row 6), an
operator whose runs the cut placed in two blocks, or the self-check (row 7). The two unopenable
oracle fixtures are refused by `extract` too: one has no `%PDF-` header, one a broken
cross-reference table.

| the tree, over 129 tagged documents | value |
| --- | ---: |
| elements written | 582: 129 `/Document`, 453 `/Div` |
| sequences written | 776, 1.71 per block |
| blocks written as more than one sequence | 74 of 453 (16.3%) |
| most sequences in one block | 60 (`bench/01030000000199`, one page, one block) |
| highest per-document share of multi-sequence blocks | 100.0% (`bench/01030000000070`) |
| `/K` entries that are not bare ids | 0 |
| catalog stamp missing | none |

§7.1 asked for the multi-sequence blocks bucketed by cause. The writer holds each sequence's
placement and cause in its plan and prints neither, so from outside only the distribution can be
counted; the causes need a line of output the writer does not have.

**The projections of every tagged document are its original's.** `ground`, `markdown` and `html`
compare equal on 129 of 129. This instrument's first run, on 2026-09-17 at `5b4b7bb` — the writer
without S3's `group_key` change — found 102, 97 and 97 of 128 different, which is §4.3's
prediction measured: read as a declaration, a written sequence joins runs the original never
joins. `engine/ink-past-the-media-box`'s grounding element read `Off the left edgeOn the page`
tagged against `On the page` untagged, and the verifier grounded a claim on the tagged side that
found no element on the original. With S3's change the two sides are one.

**Grounded and verified.** `grounding-check` on the 129 tagged grounding artifacts: exit 0,
`structure` valid and `source_binding` matched on every one. `ethos verify` ran on 122 and returned
the same report on both sides on 122, all grounded. On the other 7 no element contains the chosen
run on either side (`absent-font-metrics`, `absent-font-widths`,
`composite-font-non-identity-cmap`, `ink-past-the-media-box`, `ligature-fi-embedded-font`,
`rotation-90`, `bench/01030000000030`), which is the same answer twice. The named subset, the
engine fixtures and the first ten bench documents, is in `results/roundtrip-summary.json`.

**A second `tag` is byte-identical to the first on 129 of 129**, bench documents included.

**Another reader opens every output.** `qpdf --check` (12.3.2) exits 0 on all 129 tagged documents
of the run at `0c7a3c9`, as it does on 128 of their originals; the other original,
`bench/01030000000141`, exits 3 with warnings, and its tagged output exits 0. This engine's own
reader re-reading its output is the self-check, and it cannot see a file only other readers reject:
the numbering rule the review replaced passed it while qpdf read its output's page as blank.

| bytes added, over 129 tagged documents | min | median | max |
| --- | ---: | ---: | ---: |
| absolute | -125,323 (`bench/01030000000141`) | 716 | 3,821 (`bench/01030000000193`) |
| relative to the input | -7.8% (`bench/01030000000141`) | 44.3% | 125.3% (`oracle/synthetic/simple-text`) |
| total | 9,108,805 in, 9,076,431 out; 15 documents shrank | | |

The output is a full re-serialisation (§3.5), so a document can shrink. The largest shrink is an
Adobe PDF Library file.

| reals outside content streams, 293 originals | seen | not surviving `f32` | example |
| --- | ---: | ---: | --- |
| `/MediaBox` | 300 | 0 | |
| `/CropBox` | 140 | 0 | |
| `/Rect` | 3,529 | 0 | |
| `/BBox` | 129 | 0 | |
| `/Matrix` | 12 | 2 | `-1.60399354` → `-1.6039935` (`bench/01030000000103`) |
| `/FontMatrix` | 6 | 4 | `0.00100000005` → `0.001` (`bench/01030000000163`) |
| `/Widths` | 0 | 0 | |
| all | 4,116 | 6, in 2 documents, both PyPDF2 | |

§3.5 measured none on the repository's own 54 PDFs, and gave a non-zero count *on a producer that
matters* as one of the two conditions that reopen the incremental update. It is non-zero here on
one producer, below the ninth significant digit, and the text record does not see it (`extract`
reads both sides through `f32`). Whether PyPDF2 matters is the owner's to say, and nothing here
decides it. The scan is partial where stated: 134 `/Widths` arrays reached through a reference and
the one document with object streams (`oracle/foreign/opendataloader/real`) are not read.

### The consumer that ignores `/A`: 0.58.0 (scope §7.2, first half)

The release build's `extract` of the 129 tagged documents binds 41,208 runs `pdf_tagged` under
`Document/Div`, on 129 of 129 documents every run that is not page furniture, and not one of those
locators carries `derivation`: 0.58.0 reads `/A` only on cell spans and has no field to say
*computed*. So a shipped reader with no test-only flag takes every engine-written `/Div` as the
author's structure, the launder decision #21 refused. On `leading-gap-two-blocks`: 6 runs,
`(absent)` against the writer's build's `computed`; on `bench/01030000000007`: 1,310 runs, the
same. The engine that wrote the tree reads the same bytes as computed (above). §7.2's second half,
the misread rate, is §1.

### The wire cost of `derivation` (scope §4.2)

Every gate document is author-tagged, so every `pdf_tagged` locator gains
`"derivation":"extracted",`, 25 bytes. Branch against release, same document, same version string:

| gate document | pages | artifact bytes | `pdf_tagged` locators | `derivation` bytes | share | release artifact bytes | branch minus release |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `irs-f1040sd-2025` | 2 | 699,411 | 1,098 | 27,450 | 3.9% | 670,536 | 28,875 |
| `irs-fw9` | 6 | 865,109 | 1,097 | 27,425 | 3.2% | 836,259 | 28,850 |
| `nist-sp-800-161r1` | 327 | 298,089,238 | 529,859 | 13,246,475 | 4.4% | 284,841,338 | 13,247,900 |
| `nist-sp-800-171r3` | 120 | 90,076,670 | 165,381 | 4,134,525 | 4.6% | 85,940,720 | 4,135,950 |
| `nist-sp-800-207` | 59 | 46,897,683 | 86,098 | 2,152,450 | 4.6% | 44,743,808 | 2,153,875 |
| `nist-sp-800-218` | 36 | 31,613,728 | 59,682 | 1,492,050 | 4.7% | 30,120,253 | 1,493,475 |
| `nist-sp-800-37r2` | 183 | 227,440,212 | 384,835 | 9,620,875 | 4.2% | 217,817,912 | 9,622,300 |
| `nist-sp-800-53Ar5` | 733 | 1,037,913,352 | 1,649,453 | 41,236,325 | 4.0% | 996,675,602 | 41,237,750 |

The field costs 3.2% (`irs-fw9`) to 4.7% (`nist-sp-800-218`) of the artifact, median 4.3%. The
remainder of every difference is the same 1,425 bytes: the profile-scoped
`block-subdivision-leading-gap-only` declaration this branch also adds, with its separating comma.

### What it does not measure

- **Why a block needed more than one sequence.** Stated above.
- **A tagged document through any reader but these two.** The engine-local guarantee (§9 item 1)
  is shown on the one consumer the scope names; other readers are not run.
- **The `f32` narrowing beyond seven keys.** Reals inside other dictionaries, and in `/Widths`
  arrays given by reference, are not scanned.

### Found on the way

1. **The writer let a dangling reference resolve to one of its own objects.** The first run
   refused `engine/form-orphan-widget` in the self-check: its widget names `/Parent 9 0 R` in a
   file holding objects 1 to 7, and the writer's second new object took number 9. Fixed in
   `8500ab3` (new objects are numbered above the highest number any reference names), and the
   fixture tags in the run above.
2. **The first run's projections differed on 97 to 102 of 128 documents**, on a build without S3.
   That is recorded above as §4.3's prediction measured, not as a defect of the shipped writer.
