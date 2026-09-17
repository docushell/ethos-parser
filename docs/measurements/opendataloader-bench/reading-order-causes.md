# E3 — the reading-order gap, and where it actually is

Measured 2026-09-18 on `opendataloader-bench`. **NID re-measures at 0.8697, the same four decimals
the 0.58.0 record carries.** The gap the task asks about turns out not to be one number: 0.0465 of
it is reading order, 0.0626 is which characters reached the projection at all, and 0.0212 is where
the spaces went. This document separates the three before naming a cause, because a metric that
pays the same for a missing character and a moved one cannot be read as a reading-order result
without doing that.

`liteparse` publishes **0.908** on this benchmark in its own README. That figure is
**published by them and not reproduced here** — no run of theirs was made for this measurement, on
this machine or any other. It appears once, in that sentence, and nothing below is ranked against
it. `06-STEAL-REFUSE.md` O26 refuses *"#1, fastest, or any bake-off claim"*; every number below is
this engine's own.

## What ran

Identical to [`tables-why-zero.md`](tables-why-zero.md): engine
the build at `fceaf98`, `--version` **ethos-parser 0.58.0**, sha256
`bf782a81ccfe2f5046589ec1a554bc15f3df34e937c7a4c2e3d6e133295e2672` — **workspace 0.58.0 plus this
session's B1–B9 fixes, not the released tarball, and `--version` cannot tell them apart**.
`profile_sha256` `sha256:da38cc2564…70b7d9`, `reading_order_rule` `gutter-columns-v3` on **200 of
200** documents. Harness at `7af1d8f4d0c09f51ea1a5c6ba5f66e993286d109`, its own
`evaluator_reading_order.evaluate_reading_order` unmodified under its own virtualenv. Repository
read at HEAD `fceaf988c9d659469895d6ce00ea65dbd8dcedb4`, unmodified.

```bash
E=docs/measurements/opendataloader-bench; B=target/release/ethos-parser
export ETHOS_BENCH=~/ethos-external-benchmarks/opendataloader-bench ETHOS_PARSER_BIN=$B

python3 $E/save_artifacts.py $ETHOS_BENCH/pdfs        # 200 `extract` artifacts
python3 $E/save_classify.py  $ETHOS_BENCH/pdfs        # 200 `classify` artifacts
$ETHOS_BENCH/.venv/bin/python $E/score_e23.py         # per-document NID, band, ten lowest
$ETHOS_BENCH/.venv/bin/python $E/analyse_order.py     # the three-way split, per document
python3 $E/sweeps.py                                  # the shape of each permutation
python3 $E/sweeps.py 01030000000183 01030000000103 …  # one document at a time

```

## The band, with the worst documents named (decision #18)

**NID 0.0068..0.9973, median 0.9238, mean 0.8697 over 200 of 200 documents, 0 empty predictions.**
Worst `01030000000141` (0.0068). Best `01030000000024` (0.9973). Every document is one page.

| NID | documents |
| --- | ---: |
| above 0.95 | 77 |
| 0.90–0.95 | 42 |
| 0.75–0.90 | 45 |
| 0.50–0.75 | 32 |
| 0.50 or below | **4** — `01030000000141`, `01030000000027`, `01030000000107`, `01030000000103` |

The ten lowest, named: `01030000000141` 0.0068, `01030000000027` 0.2289, `01030000000107` 0.4368,
`01030000000103` 0.4628, `01030000000183` 0.5085, `01030000000110` 0.5209, `01030000000085` 0.5238,
`01030000000128` 0.5387, `01030000000149` 0.5490, `01030000000070` 0.5667.

## Splitting NID before reading it

`evaluate_reading_order` is `fuzz.ratio` over the two whole whitespace-normalized strings. That is
indel similarity, so

```
nid = 2 · LCS(gt, pred) / (len(gt) + len(pred))
```

and the **multiset intersection of the two character populations is an upper bound on that LCS**.
So, on lowercase-alphanumeric strings (`squash`, which removes the whitespace question entirely):

```
ceiling   = 2 · |gt ∩ pred| / (len(gt) + len(pred))        the best these two populations can do
1 - nid_sq = (1 - ceiling)        +  (ceiling - nid_sq)
           = population cost      +  ordering cost
```

`population cost` is what no reordering can fix: characters the prediction lacks, or characters it
carries that the ground truth does not. `ordering cost` is permutation and nothing else. It is an
**upper bound on what a reading-order fix can buy**, not a prediction about any particular rule.

Over the 200 documents, measured:

| component | mean | what it is |
| --- | ---: | --- |
| **population cost** | **0.0626** | characters missing from, or extra in, the projection |
| **ordering cost** | **0.0465** | permutation, with the population held fixed |
| **whitespace cost** (`nid_sq − nid`) | **0.0212** | where the spaces went — the injected intra-word space, and block spacing |
| sum | **0.1303** | and `1 − 0.8697 = 0.1303`, to four decimals |

**Reading order is the second-largest of the three, not the first.** Most of the shortfall this
benchmark reports against this engine is not a reading-order result.

## The cause on the ten worst, one cause each

Assigned by which component dominates, then read document by document with `sweeps.py`, which cuts
a new **sweep** wherever the emitted order jumps back up the page by more than that document's own
median line step (floored at 400 centipoints; stated because it is a threshold). **A page in visual
reading order is one sweep.**

| document | NID | ceiling | pop | **ord** | sweeps | `region` | cause |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `01030000000183` | 0.5085 | 0.980 | 0.020 | **0.451** | 11 | none | content-stream order |
| `01030000000103` | 0.4628 | 0.994 | 0.006 | **0.380** | 7 | none | content-stream order |
| `01030000000085` | 0.5238 | 0.932 | 0.068 | **0.326** | 2 | none | content-stream order |
| `01030000000149` | 0.5490 | 0.915 | 0.085 | **0.323** | 3 | none | content-stream order |
| `01030000000070` | 0.5667 | 0.905 | 0.095 | **0.294** | 7 | none | content-stream order |
| `01030000000141` | 0.0068 | 0.006 | **0.994** | 0.000 | 1 | none | the text is not text |
| `01030000000107` | 0.4368 | 0.455 | **0.545** | −0.000 | 1 | none | the truth transcribes a picture |
| `01030000000110` | 0.5209 | 0.647 | **0.353** | 0.000 | 1 | none | the truth transcribes a picture |
| `01030000000128` | 0.5387 | 0.697 | **0.303** | 0.000 | 1 | none | the truth transcribes a picture |
| `01030000000027` | 0.2289 | 0.269 | **0.731** | 0.051 | 4 | `[1, 2]` | text the page does not show |

| cause | documents |
| --- | ---: |
| **content-stream order left in place** | **5** |
| the truth transcribes a picture (or vector art) | **4** |
| the projection carries text the page does not show | **1** |

### The five ordering failures, and what each permutation actually is

All five carry **no `region` on any run**, which is the artifact saying the rule cut nothing.
`reading_order.rs` step 5: *"No gutter, no reordering. A page whose first vertical cut fails comes
out in content-stream order, unchanged, byte for byte."* **So on these five the emitted order is
the producer's, and the permutation is the producer's, left in place.** Three are the same error.

**A running head or foot arrives before the body — 3 documents.**

- `01030000000085`: the prediction's first sweep is the footer — *"The Law Library of Congress,
  Global Legal Research Directorate (202) 707-5080 · law@loc.gov"* — at y 740.0–753.1 pt, the
  bottom of a 612×792 pt page. Sweep 2 is the title block at y 293.0–643.8. The ground truth ends
  with that footer. 134 runs, 2 sweeps.
- `01030000000149`: sweep 1 opens with *"This project has been funded with the support of the
  European Commission…"*, which is the ground truth's **last** block. 443 runs, 3 sweeps.
- `01030000000070`: sweep 1 is the page number `14` at y 803.9, the ground truth's last line.
  443 runs, 7 sweeps.

**Whole blocks arrive in producer order, unrelated to the layout — 2 documents.**

- `01030000000103`, 7 sweeps on a 612×792 pt page. The prediction opens with section **06**'s body
  (y 608.4–622.7), then section 06's heading (y 589.3), then the page title `СREATING SLIDES`
  (y 84.6) **third**, then a 466-run mixed sweep, then the headings `01 - Find Open Educational
  Resources` / `02- Prepare Your Content` / `03- Generate Slides with ChatGPT` grouped together in
  one sweep spanning y 126.0–758.3 — **separated from the bodies they head** — then the footer.
  The ground truth is title, then 01, 02, 03 in order, each heading with its own body.
- `01030000000183`, 11 sweeps on a **960×540 pt slide**. Sweeps 1–3 are the three panel headings,
  all in the y band 159.8–206.9 and in three x bands (63.6–250.5, 353.5–565.1, 656.6–867.9). Sweep
  4 is the page title at y 86.4–117.4, **above all three**. Sweep 5 carries the banner
  `Recommendation Pack: Track Record` at y 50.2. Sweeps 6–11 are chart labels belonging to three
  *different* panels, interleaved: `0.882 0.735` from the right panel, then `CustomerBERT` from the
  centre, then `0.09 0.06 0.03 Personalize AutoEncoder_RecVAE…`, then `AWS Ready 14.3%↑ 1.7X↑
  2.6X↑` spanning x 226.8–587.8, then `0.4048 0.3278 0.23496 0.159` from the left panel. **The
  title is why no cut happened**: it spans x 70.3–789.3 and so crosses every panel gutter, leaving
  no vertical band of the page that no text crosses — which is the only question step 2 asks.

**Why the runs are glyph-by-glyph here, and why it matters.** On `01030000000183` every run is one
or two glyphs (`Co`, `m`, `p`, `a`, `r`…) and **no run carries a `block`** — the whole page has 0
distinct block values, so `block-subdivision-leading-gap-only` found no vertical-whitespace cut to
subdivide on. The projection therefore has no unit larger than a run to reorder, and the injected
intra-word space (`RE GIONS`, `Cordiller a Autonomous R egion`) has the same origin.

### The five population failures, for completeness

- `01030000000141` — **the worst document on this corpus, and its text is not text.** A 1728×2592 pt
  page (24×36 in, a poster). `classify`: **130 657 path operators, 2 text operators, 7 text bytes**,
  reasons `dense-graphics` + `sparse-text`. `extract` finds two runs, `and` and `.org`, against
  2 368 characters of ground truth. Its one image covers **0.6% of the page** — so the 0.58.0
  record's *"one embedded image over a 7-byte text layer"* is not what this measurement sees: the
  type is **drawn as vector paths**, not rastered. Reading it means rendering and OCR, both refused
  (`page-raster-not-emitted`; `06-STEAL-REFUSE.md` L24/L25).
- `01030000000107` (2 rasters, 48.1% of the page), `01030000000110` (28.6%), `01030000000128`
  (14.9%) — the ground truth transcribes chart and table labels that live inside those rasters;
  character recall 0.29, 0.48, 0.54.
- `01030000000027` — **the opposite failure, and the one with a name already.** The artifact
  declares `off-page-text`: **440 of 480 text runs have an origin outside the page's visible box**,
  measured after `/Rotate`. They are projected anyway, so the prediction carries 2 629 squashed
  characters against the ground truth's 454 — character **precision 0.16**, recall 0.91. The
  ground truth transcribes what a renderer shows; this projection emits what the file contains.

## Which single cause, if fixed, would move the most documents

**Content-stream order left in place, by a wide margin, on both counts.**

| cause | documents it touches | headroom on the 200-document mean NID |
| --- | ---: | ---: |
| **content-stream order (pages emitted in more than one sweep)** | **126** | **+0.0461** |
| the truth transcribes a picture (recall < 0.90 and image area > 5%) | 32 | +0.0225 |
| whitespace placement (injected intra-word space, block spacing) | 200 | +0.0212 |
| off-page runs projected | 5 | +0.0101 |
| population cost not in either group above | — | +0.0300 |

The arithmetic, all measured:

- **The rule reorders almost nothing.** 22 of 200 documents carry a `region` on any run; on the
  other **178** the first vertical cut failed and step 5's identity fallback returned content-stream
  order. Those 178 hold **90.1%** of the corpus's total ordering cost.
- **The sweep count separates the corpus cleanly.** 74 documents come out in **one** sweep: mean
  NID **0.9047**, mean ordering cost **0.0011** — essentially zero, the rule had nothing to fix.
  The other **126** come out in more than one: they hold **99.1%** of all ordering cost.

  | sweeps per page | documents | mean NID | mean ordering cost | share of all ordering cost |
  | --- | ---: | ---: | ---: | ---: |
  | 1 | 74 | 0.9047 | 0.0011 | 0.9% |
  | 2–3 | 83 | 0.8959 | 0.0518 | 46.2% |
  | 4–9 | 31 | 0.7455 | 0.1042 | 34.7% |
  | 10+ | 12 | 0.7928 | 0.1410 | 18.2% |

- **Driving ordering cost to zero takes mean NID from 0.8697 to 0.9162** (0.8697 + 0.0465), and it
  is reachable in principle because the characters are already there: on 66 of 200 documents
  ordering cost already exceeds population cost, and on the five worst ordering documents the
  ceiling is 0.905–0.994 while the score is 0.463–0.567.
- **The next-largest cause is capped by a refusal, not by effort.** "The truth transcribes a
  picture" is worth +0.0225 across 32 documents and every one of them needs glyphs out of a raster
  — OCR, which L24/L25 keep opt-in and profile-isolated, and `page-raster-not-emitted` says no
  raster is produced at any resolution. So the largest *available* cause is the ordering one twice
  over.
- **The components do not double-count.** The document sets overlap — `01030000000070` is both
  multi-sweep and picture-lossy — but population cost and ordering cost are additive **per
  document** by the identity above, so the column of headroom figures sums correctly even where the
  rows share documents.

**What a fix would have to do.** Not "sort by y then x" — `reading_order.rs` argues that at length
and it is right: a global sort reorders the 74 single-sweep documents that are already correct, and
their mean NID is 0.9047. What the measurement points at instead is the **precondition for cutting
at all**. Today a single full-width element — `01030000000183`'s title spanning x 70.3–789.3 —
defeats the page's only vertical cut and sends the whole page down the identity path. A cut that
could set aside a full-width band first, and then look for gutters in what remains, would reach
the 126 multi-sweep documents without touching the 74. **That is a new rule reading new evidence
and it needs its own id**, exactly as `gutter-columns-v2` and `-v3` did.

## What could not be measured, and what it would need

1. **A counterfactual for any of these fixes.** Attempted for the smallest and best-isolated cause:
   drop the runs an artifact already declares `off-page-text` and re-run the committed `markdown`
   projection over what is left. **The engine refused it at four independent invariants** — geometry
   rows must equal nodes (*"a missing row would make absence ambiguous with omission"*), ordinals
   must be contiguous, the assurance block's absence counts must match the payload, and
   `representation_c14n_sha256` must verify. Getting past the fourth would mean rewriting the
   assurance block and recomputing the canonicalization digest, i.e. **fabricating an artifact**, so
   the attempt was stopped. The script and its refusals are in `onpage_counterfactual.py`. Measuring
   any of these fixes needs the fix built in the engine and the corpus re-run; `ordering cost` is
   the ceiling, not a promise.
2. **Which of the four 0.47.0–0.55.0 block-assembly changes moved NID on this corpus**, still not
   measured per release — the 0.58.0 record already says so and this run does not change it.
3. **`01030000000072`**: character recall 0.61 with 17.2% image area, and no limitation naming a
   text loss — no `form-xobjects-not-descended`, no `off-page-text`, no `broken-font-encoding`. It
   is not in the ten lowest so it was not chased. Where its missing 39% went is **unknown**, and it
   is the one document in this measurement whose loss no declared code explains.
4. **Whether the ground truth is complete.** On `01030000000028`, `01030000000029` and
   `01030000000031` the prediction carries roughly twice the ground truth's characters at recall
   1.00 and precision 0.47–0.52 — all three declare `off-page-text` on 649, 789 and 703 runs, so
   this is most likely the same cause as `01030000000027`. Confirming it would mean rendering each
   page and reading it, which needs a renderer this engine does not have.
