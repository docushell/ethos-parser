# M1 — What bounds peak memory, and why the two figures in the tree do not

**Run 2026-09-10 against `97fa562`** over the eight committed `fixtures/gate` documents, on macOS
arm64 (48 GiB, 12 cores, no swap in use). Peak RSS is read from outside the process by
`/usr/bin/time -l`, medianed over five runs for the corpus table and three for the ladders.
Instruments: [`model.py`](model.py), [`within.py`](within.py), [`intercept.py`](intercept.py).
Raw readings: [`bench-97fa562.tsv`](bench-97fa562.tsv), [`pages.json`](pages.json).

Plan item 6.2 says **"verify the figure first"**. The figure does not survive, and neither does
its replacement. What does survive is a two-term model and a floor nobody had measured.

---

## 1. Why this was run

The tree carries two figures for the same quantity and they cannot both be ceilings:

| source | claim |
| --- | --- |
| `ci/bench.py:30` | peak RSS is "roughly 300x the *input*" |
| `crates/ethos-parser-pdf/src/extract.rs:1043` | "~4.7 MiB per page, and **independent of file size**" |
| `crates/ethos-parser-cli/src/main.rs:321` | repeats the 4.7 MiB/page figure as what `--max-pages` bounds |

The second disproves the first as a bound: `extract.rs`'s own table has two 1.5 MB documents whose
peak differs 4.5x on page count alone. A caller sizing a machine needs to know which figure is a
law and which is an accident of this corpus.

## 2. The corpus, measured

| fixture | pages | input | artifact | peak RSS | /input | /artifact | MiB/page |
| --- | --- | --- | --- | --- | --- | --- | --- |
| irs-f1040sd-2025 | 2 | 0.09M | 0.64M | 15.1M | 162x | 23.66x | 7.57 |
| irs-fw9 | 6 | 0.13M | 0.80M | 19.2M | 143x | 24.04x | 3.20 |
| nist-sp-800-218 | 36 | 0.71M | 28.73M | 228.1M | 323x | 7.94x | 6.34 |
| nist-sp-800-207 | 59 | 0.92M | 42.67M | 248.6M | 270x | 5.83x | 4.21 |
| nist-sp-800-171r3 | 120 | 1.52M | 81.96M | 567.7M | 374x | 6.93x | 4.73 |
| nist-sp-800-37r2 | 183 | 2.17M | 207.66M | 1080.3M | 499x | 5.20x | 5.90 |
| nist-sp-800-161r1 | 327 | 4.62M | 271.55M | 1641.8M | 355x | 6.05x | 5.02 |
| **nist-sp-800-53Ar5** | **733** | **7.12M** | **950.48M** | **6646.5M** | **933x** | 6.99x | **9.07** |

**The 7.12 MiB document needs 6.5 GiB.** `ci/bench.py` only ever extrapolated this one — "needs
several gigabytes" — so the corpus's worst case had never actually been run to completion and
recorded. It has now. An 8 GB machine cannot parse it.

Two prior readings reproduce, which is the evidence that this run is comparable to theirs:
`161r1` at 1641.8 MiB against `bench.py`'s recorded 1.65 GB, and `171r3` at 567.7 MiB against
v2-S15's recorded 558.0-566.1 MiB. So adding `block` to every text run cost essentially nothing in
peak despite growing the artifact — worth knowing, because it was the obvious suspect.

## 3. Neither published figure is a bound

A ceiling needs a coefficient that **holds**, not one that averages nicely. `model.py` reports the
spread of each candidate's implied coefficient:

| model | coefficient range | median | spread | admissible as a ceiling? |
| --- | --- | --- | --- | --- |
| peak = k x input bytes | 143x - 933x | 339x | **6.53x** | yes — caller knows input |
| peak = k x artifact bytes | 5.20x - 24.04x | 6.96x | 4.62x | **no** — unknown until the run it would bound |
| peak = k MiB x pages | 3.20 - 9.07 | 5.46 | **2.84x** | yes — caller knows page count |

**"Roughly 300x the input" is a median wearing a bound's clothing.** Measured, it runs 143x to
933x. Anyone provisioning from 300x under-sizes the worst gate document by a factor of three.

**The 4.7 MiB/page figure has also drifted** — the median is now 5.46 and the range reaches 9.07.

The one quantity that *is* flat is the one a caller cannot use. Dropping the two small documents,
where a ~7 MiB process floor dominates, **peak/artifact across the six real documents is
5.20x-7.94x — a spread of only 1.53x.** Peak memory is a tight multiple of what the engine
*emits*; the scatter in MiB/page is content density, since artifact-per-page itself ranges 0.13 to
1.30 MiB.

## 4. Within one document, peak is linear in pages

The corpus table cannot separate "the engine is superlinear" from "these documents differ", because
every row has different content. `--max-pages` gives the controlled experiment: same document,
same density, only the admitted page count moves.

| budget | 53Ar5 peak | marginal MiB/page | | budget | 171r3 peak | marginal MiB/page |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | 224.2M | — | | 0 | 45.3M | — |
| 8 | 249.0M | 3.10 | | 8 | 73.2M | 3.49 |
| 32 | 438.4M | 7.89 | | 32 | 188.1M | 4.79 |
| 64 | 666.1M | 7.12 | | 64 | 307.4M | 3.73 |
| 128 | 1364.1M | 10.91 | | none (120) | 563.5M | 4.57 |
| 256 | 2459.7M | 8.56 | | | | |
| 512 | 4803.5M | 9.16 | | | | |
| none (733) | 6656.8M | 8.39 | | | | |

**The marginal cost per admitted page is constant** — about 8.5 MiB/page on `53Ar5` and 4.3 on
`171r3`. So there is **no superlinear retention**: the engine is linear in pages within a document,
and the cross-document spread in MiB/page is density. That matters because it is what makes a
per-page coefficient usable at all; a rising marginal would understate every document larger than
the one measured.

The apparent fall in *average* MiB/page as the budget rises (31.12 at 8 pages down to 9.08 at 733)
is the floor amortizing, not the engine improving.

The unbounded readings here differ from §2 by 0.2%-0.7% (6656.8 vs 6646.5; 563.5 vs 567.7) — three
runs versus five. That is the noise floor of this instrument on this machine, and it is the reason
`ci/bench.py` medians rather than samples.

## 5. The floor `--max-pages` cannot lower

`--max-pages 0` is documented as "process none". It is therefore a direct measurement of the fixed
cost, not a fit:

| fixture | total pages | peak at `--max-pages 0` |
| --- | --- | --- |
| irs-fw9 | 6 | 7.1M |
| nist-sp-800-171r3 | 120 | 45.3M |
| nist-sp-800-161r1 | 327 | 65.1M |
| nist-sp-800-53Ar5 | 733 | **224.2M** |

**Admitting zero pages of the 733-page document still costs 224 MiB.** Something document-wide is
built before the budget is consulted: `extract.rs:972` calls `structure::read(doc.inner())` and
builds `tree_mcids_by_page` over the whole tree at line 977, while the budget is not read until
line 1006. Net of the ~7 MiB process floor, that term is 0.18-0.32 MiB per document page and it
does not respond to the flag at all.

So the honest statement about the existing ceiling is: **`--max-pages` bounds the dominant term and
leaves a floor that grows with the document.** The two ceilings in the tree do not jointly bound
peak to any caller-chosen number — `MAX_SOURCE_BYTES` permits a 2 GiB input, and nothing maps
2 GiB of input onto a memory figure.

## 6. What a caller can be told today

Peak is two terms, both measurable before the run:

    peak ~= floor(total_pages) + k x admitted_pages

with, on this corpus, `floor` at 0.18-0.32 MiB per document page and `k` at 4.3-8.6 MiB per
admitted page. For sizing, the worst observed coefficient is the only safe one:

**Budget 10 MiB per admitted page, plus 0.35 MiB per page in the document.** On `53Ar5` that
predicts 7.6 GiB against 6.6 GiB measured — over by 14%, which is the direction an estimate for
provisioning should err.

## 7. What this does not settle

- Whether any of the retained memory can be released **byte-identically**. Peak is 5.2x-7.9x the
  artifact, so most of it is live Rust structures rather than the JSON buffer; `bench.py:30` names
  `canonical_bytes_of` returning one `Vec<u8>` as the cause, and on `53Ar5` that buffer is 950 MiB
  of 6646 MiB — real, but a minority of the peak.
- Whether the floor in §5 can be bounded by the page budget without changing output.
- Whether a ceiling expressed in *bytes* is wanted at all, given `profile.rs:975` already records
  the design position that pages are "the bound that actually governs peak cost ... which is why it
  is the knob rather than a byte ceiling". §5 shows that position is right about the dominant term
  and silent about the floor.

Do not re-propose chunking the parallel page fold: it was built and measured at v2-S15
(558.9 -> 563.6 MiB, nothing, slightly the wrong way) and the reasoning is at
`extract.rs:1030-1053`. The pages are the artifact; bounding how many are live bounds only the
counters.
