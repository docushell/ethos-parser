# E2 — why 28 of the 42 table documents score TEDS 0

Measured 2026-09-18 on `opendataloader-bench`, re-derived rather than read off the 0.58.0 record.
**The 28 are the same 28 documents that record names**, and the measurement adds the one fact that
record does not carry: which of them lost a *grid* and which of them lost *text*.

## What ran

| | |
| --- | --- |
| engine | the build at `fceaf98`, `--version` **ethos-parser 0.58.0**, sha256 `bf782a81ccfe2f5046589ec1a554bc15f3df34e937c7a4c2e3d6e133295e2672` |
| | **not a released build.** Workspace 0.58.0 plus this session's B1–B9 fixes. It is not the `0.58.0` tarball the README's run used, and `--version` cannot tell the two apart |
| profile | `profile_sha256` `sha256:da38cc2564…70b7d9`, schema `0.6.0`, `reading_order_rule` `gutter-columns-v3`, table rules `ruled-rects-v6` / `unruled-align-v1` / `stroke-ruled-v1` |
| harness | `~/ethos-external-benchmarks/opendataloader-bench` at commit `7af1d8f4d0c09f51ea1a5c6ba5f66e993286d109` — the same commit the README records |
| evaluator | the harness's own `evaluator_table.evaluate_table`, unmodified, under its own virtualenv (Python 3.13.14, rapidfuzz 3.14.3) |
| adapter | this repository's committed `docs/measurements/opendataloader-bench/pdf_parser_ethos_parser.py`, imported unmodified |
| repository | read at HEAD `fceaf988c9d659469895d6ce00ea65dbd8dcedb4`; nothing in it was modified |

```bash
E=docs/measurements/opendataloader-bench; B=target/release/ethos-parser
export ETHOS_BENCH=~/ethos-external-benchmarks/opendataloader-bench ETHOS_PARSER_BIN=$B

python3 $E/save_artifacts.py  $ETHOS_BENCH/pdfs     # 200 `extract` artifacts, 1 s wall
python3 $E/save_classify.py   $ETHOS_BENCH/pdfs     # 200 `classify` artifacts, 1 s wall
$ETHOS_BENCH/.venv/bin/python $E/score_e23.py       # predictions + per-document TEDS/NID
$ETHOS_BENCH/.venv/bin/python $E/cell_coverage.py   # does the truth's table TEXT reach the prediction
python3 $E/analyse_tables.py                        # one row per document, from the artifacts
python3 $E/e2_causes.py                             # one cause per zero, first match wins
python3 $E/fabrication.py                           # the two fabrication tests
```

`score_e23.py` is `score.py` with three changes, all recorded in its own header: predictions are
kept, every per-document pair is written to `scores.json`, and MHS is not computed. The scored
predictions are the committed adapter's.

## The result reproduces, to four decimals

| metric | this run | the 0.58.0 record |
| --- | ---: | ---: |
| TEDS, mean over the 42 | **0.1704** | 0.1704 |
| documents holding a table in ground truth | **42** | 42 |
| non-zero / at zero | **14 / 28** | 14 / 28 |
| NID, mean over 200 | **0.8697** | 0.8697 |
| empty predictions | **0** | 0 |

**TEDS band 0.0000..0.9802, median 0.0000, mean 0.1704. The worst is a tie of 28 and
`01030000000064` is the first of them by name.** Best `01030000000053` (0.9802).

Two corrections to how the population is usually described. **The ground truth holds HTML
`<table>` elements, not GFM pipe tables** — 0 of the 200 ground-truth files contain a GFM
delimiter row, and all 42 contain `<table>`; the harness's `convert_to_markdown_with_html_tables`
passes HTML through and converts the *prediction's* GFM. And the 42 documents hold **55** tables,
not 42: 32 documents hold one, 7 hold two, 3 hold three. **Every prediction that holds a table
holds exactly one**, so on the 10 multi-table documents the engine is scored one grid against two
or three before any cell is compared.

## The measurement that decides each cause

A TEDS of zero says no grid was scored. It does not say whether the table's characters were in
the file. So the ground truth's own cell strings were looked for in the prediction
(`cell_coverage.py`): every `<td>`/`<th>` the harness's converter finds, reduced on both sides to
lowercase alphanumerics, cells under 3 such characters excluded.

**Reducing to alphanumerics is not tidiness, it is required.** This projection injects spaces
inside words the producer kerned. On `01030000000052` the header cell comes out `RE GIONS` and a
body cell `Cordiller a Autonomous R egion`. A whitespace-collapsing comparison scored 0 of 15
cells present on that document — while TEDS scored it 0.9761, because TEDS's per-cell Levenshtein
pays one edit and moves on. The two comparisons are printed side by side in `cell-coverage.json`
(`gt_cells_found_in_prediction` against `gt_cells_found_whitespace_exact`) so the difference is
visible rather than assumed.

**On 25 of the 28 zeros, ≥90% of the ground truth's table cell text is in the prediction.** The
grid was lost, not the text.

## The cause breakdown, one cause per document, summing to 28

Assigned by `e2_causes.py` in this order, first match wins, from measured facts only.

| | cause | documents |
| ---: | --- | ---: |
| 1 | the truth's table is a picture | **2** |
| 2 | a drawn grid was built and refused | **12** |
| 3 | an inferred grid past the 4096-cell ceiling | **11** |
| 4 | an inferred grid under the 1200-centipoint gutter floor | **3** |
| | | **28** |

**Two things no document does.** No zero emitted a table at all — so "emitted a grid that overlaps
the truth's not at all" is a cause with **0** documents, and a zero here is always an absence.
And **no zero is silent**: all 28 declare a named table-candidate refusal. The codes exist for
exactly this and they held.

### Cause 1 — the truth's table is a picture (2 documents)

`01030000000110`, `01030000000122`. Under half the truth's cell text is anywhere in the
prediction, and the artifact carries an image node.

| | `01030000000110` | `01030000000122` |
| --- | ---: | ---: |
| TEDS | 0.0000 | 0.0000 |
| cell text found | 0 of 54 | 4 of 11 |
| image node | 499×278 pt, 987×551 px, **28.6% of the page** | 458×124 pt, 1320×358 px, **11.7% of the page** |
| `classify` | `embedded-images` | `embedded-images` |
| the truth's cells, probed in `extract`'s text nodes | `Temperature` absent | `BamHI`, `Restriction Buffer`, `Suspect 1 DNA` all absent |

The image's rectangle and the truth's table sit in the same place in both files: on
`01030000000122` the table is the fourth block of the ground truth and the image is the strip at
y 72–196 pt; on `01030000000110` the table follows four paragraphs and the image covers
y 350–628 pt. The engine records the picture exactly — page, object number, painted rectangle,
`pixel_width`/`pixel_height`, a sha256 over the stream — and declares
`image-payload-not-embedded` and `page-raster-not-emitted`.

**What a fix would have to do:** read glyphs out of a raster. That is OCR, and this profile does
not have it — `06-STEAL-REFUSE.md` L24/L25 keep OCR opt-in, profile-isolated and confined to where
no text layer exists. **These two documents are not a table defect and no table rule can reach
them.** They are the price of that position, stated.

### Cause 2 — a drawn grid was built and refused (12 documents)

The page painted rectangles that implied a lattice, and `ruled-rects-v6` judged it incoherent.
Cell text is 86–100% present on all twelve.

| document | rects (`classify`) | what the artifact says refused it |
| --- | ---: | --- |
| `01030000000064` | 225 | 52 rects → 153 cells, **column** boundary 1 of 10 not traced |
| `01030000000078` | 76 | 65 rects → 576 cells, **column** boundary 1 of 25 not traced |
| `01030000000088` | 294 | 135 rects → 135 cells, **column** boundary 2 of 16 not traced |
| `01030000000089` | 272 | 135 rects → 135 cells, **column** boundary 2 of 16 not traced |
| `01030000000090` | 309 | 152 rects → 150 cells, **column** boundary 2 of 16 not traced |
| `01030000000119` | 107 | 93 rects → 126 cells, **column** boundary 1 of 8 not traced |
| `01030000000128` | 23 | 23 rects → 112 cells, **column** boundary 1 of 8 not traced |
| `01030000000146` | 263 | 123 rects → 377 cells, **column** boundary 1 of 14 not traced |
| `01030000000147` | 269 | 86 rects → 273 cells, **column** boundary 1 of 14 not traced |
| `01030000000149` | 178 | 83 rects → 162 cells, **column** boundary 1 of 8 not traced |
| `01030000000150` | 254 | 107 rects → 448 cells, **column** boundary 1 of 17 not traced |
| `01030000000200` | 0 | **458 structural and 58 geometric faults** — `geometric-vs-structural-v1` rejected the grid it had reconstructed |

**Eleven of the twelve are one failure, and it is always a column boundary.** Eight of the eleven
name interior boundary 1 — the first one in from the left edge. The refusal's own explanation
already states the producer behaviour: *"a producer laying down row separators and no column
separators has stated exactly where its grid lies while drawing almost none of its cells."* On
this corpus that producer is the majority of ruled tables.

`01030000000200` is a different failure inside the same cause and is named separately for that
reason: its rectangles reconstructed into a lattice whose own indices contradicted themselves 458
times. It is also the one document where `classify` counts 0 `re` operators while the ruled rule
found rectangles — `classify` counts the `re` operator and the rule reconstructs rectangles from
general path construction, so the two counters are not the same question.

**What a fix would have to do:** accept a lattice whose interior *column* boundary is corroborated
by something other than ink — text alignment on every row it separates would be the obvious
candidate — without supplying an edge nobody drew. That is a new precondition and therefore a new
rule id, not a tuning pass: `undrawn-table-edges-not-supplied` is a standing declaration that no
rule here supplies an undrawn edge, and `table-gate-v1.md` records that `-v5` bought exactly this
kind of relaxation and **fabricated a table on five of eight gate documents**. Twelve documents is
the largest single prize in E2 and it is the one with a fabrication risk attached to it.

### Cause 3 — an inferred grid past the 4096-cell ceiling (11 documents)

No rectangle lattice was implied at all — 0–65 `re` operators, and no ruled or stroke-ruled
candidate was ever built. Only `unruled-align-v1` ran, and the text's own alignment implied more
cells than the ceiling allows.

| document | cells the text implied | ceiling | ground-truth tables | cell text found |
| --- | ---: | ---: | ---: | ---: |
| `01030000000190` | 14 941 | 4 096 | 2 | 69 of 69 |
| `01030000000187` | 14 700 | 4 096 | 1 | 25 of 25 |
| `01030000000189` | 11 856 | 4 096 | 3 | 112 of 112 |
| `01030000000182` | 9 438 | 4 096 | 1 | 15 of 15 |
| `01030000000178` | 8 648 | 4 096 | 1 | 21 of 21 |
| `01030000000117` | 8 621 | 4 096 | 1 | 13 of 13 |
| `01030000000132` | 7 560 | 4 096 | 1 | 9 of 9 |
| `01030000000170` | 6 660 | 4 096 | 2 | 50 of 50 |
| `01030000000165` | 5 250 | 4 096 | 1 | 5 of 5 |
| `01030000000116` | 4 844 | 4 096 | 2 | 22 of 23 |
| `01030000000166` | 4 680 | 4 096 | 1 | 11 of 11 |

**The ceiling is not really the constraint, and the numbers say so.** The truth's tables on these
eleven documents hold 5 to 112 cells. The candidate the rule built holds 4 680 to 14 941. The rule
is projecting *every run on the page* onto two axes and calling the product a candidate, so a page
with one small table and forty lines of body text implies a lattice two orders of magnitude bigger
than the table. Raising the ceiling would let that page-wide lattice through, which is worse than
refusing it.

**What a fix would have to do:** build the candidate over a *region* rather than the page — the
same move `gutter-columns-v3` already makes when it cuts a page into bands, whose `region` the
artifact already carries — and apply the ceiling to that. Eleven documents, and unlike cause 2 the
change is a narrowing rather than a relaxation, so it cannot buy a fabrication by construction.
Whether `unruled-align-v1`'s other preconditions then hold on those regions is **not measured
here** and would need the rule re-run per region, which needs a code change this measurement was
not allowed to make.

### Cause 4 — an inferred grid under the gutter floor (3 documents)

`01030000000130` (gap 339 cp), `01030000000180` (392 cp), `01030000000197` (561 cp), each against
the 1 200-centipoint floor. Cell text is 100% present on all three. The rule's own reason is that
300–600 centipoints is word spacing, not a gutter, and merging two disagreeing alignments means
picking one the document does not pick.

**What a fix would have to do:** distinguish a narrow column gutter from word spacing by something
other than width — a gap that is empty *on every row* is a gutter and a gap that is empty on one
row is a coincidence. Three documents; the smallest prize in E2, and the floor is load-bearing
(`table-gate-v1.md`: *"lowering it buys a fabrication"*).

## Fabrication: still 0, on two independent tests

`table-gate-v1.md` calls fabrication *"the one number with a required value"*. Both tests, over all
200 artifacts and all 400 emitted cells:

| test | result |
| --- | ---: |
| documents where the **prediction's Markdown** holds a table and the ground truth holds none | **0** |
| documents where the **artifact** emits a table and the ground truth holds none | **0** (0 cells) |
| cells whose `text` is not the concatenation of the runs its `node_ids` name | **0 of 400** |

14 documents emit a table, 14 tables, 400 cells, of which 3 carry no `node_ids` — an empty slot,
which is the honest representation of a cell the page drew and left blank. **Every one of the 14
holds a table in ground truth.** The standing claim is confirmed on this corpus: this engine
emitted no cell it could not trace to ink.

## One thing this run found and did not fix

`01030000000121` and `01030000000120` are emitted with `locator_check.outcome.status` of
`not_applicable` and **0 cells** in a `2x2` and a `3x5` grid respectively, and they still score
0.2857 and 0.1579 — a grid with no cells is being compared against a populated one. Ten of the 14
non-zero documents sit between 0.08 and 0.60 and **why each differs from its ground truth is still
not examined**, exactly as the 0.58.0 record says. E2 asked about the zeros; this is the next
question, not this one's answer.
