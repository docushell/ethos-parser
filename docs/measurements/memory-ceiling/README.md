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

## 6. A 30% cut, byte-identical

§3 says peak is a tight multiple of what the engine EMITS. That turned out to be the wrong place to
look for the fix. An adversarial audit of the retained memory refuted 13 of 16 proposals, and the
one that survived was not the artifact buffer `ci/bench.py` blamed — it was one value copied 1.65
million times.

`bind_structure` resolved a run's `(page, mcid)` against the structure tree and deep-cloned the
whole `PdfTaggedLocator` into the run; `to_representation` cloned it again into the node. On
`53Ar5` that is ~21.9M `Vec<String>` elements per copy, held twice at peak, for **30 distinct role
paths over 23 distinct role names**. The dominant path is 15 deep and carried by 1.2M runs. Sharing
one `Arc<PdfTaggedLocator>` per tree binding leaves 5,661 locators.

Interleaved A/B, median of 3, both arms in the same loop:

| document | budget | baseline | patched | delta | bytes |
| --- | --- | --- | --- | --- | --- |
| nist-sp-800-53Ar5 | — | 6647.0M | 4663.7M | **-29.8%** | IDENTICAL |
| nist-sp-800-53Ar5 | 128 | 1368.5M | 1067.3M | -22.0% | IDENTICAL |
| nist-sp-800-37r2 | — | 1083.7M | 909.7M | -16.1% | IDENTICAL |
| nist-sp-800-171r3 | — | 572.9M | 487.8M | -14.9% | IDENTICAL |
| nist-sp-800-161r1 | — | 1642.0M | 1401.8M | -14.6% | IDENTICAL |
| irs-fw9 | — | 18.7M | 17.4M | -6.9% | IDENTICAL |

**Throughput moved too, by 3% to 9%** — and that number needed its own instrument. Two SEQUENTIAL
`bench.py` runs across the change showed wall time down ~36% uniformly, including on a 6-page form
with almost no role paths to share. A real effect here must scale with role-path density, so a flat
36% is the signature of machine state. Interleaved A/B on the engine's own `wall_micros`, median of
5, gives the honest figure: -3.6% on `irs-fw9`, -2.6% on `171r3`, -3.2% on `161r1`, -4.7% on `218`,
**-8.6% on `53Ar5`** — small, and largest on the densest document, which is the shape the mechanism
predicts. This is the trap `ci/bench.py`'s own header describes, and it very nearly published a
36% throughput claim.

**What the audit refuted, each measured rather than argued.** Recorded so nobody spends the slice
twice:

| proposal | why it died |
| --- | --- |
| Stream the artifact instead of one `Vec<u8>` | A real 950 MiB at the print instant — but that instant sits 1.1-2.1 GiB BELOW the high-water mark. Peak does not move. |
| Swap the allocator (mimalloc) | **+1293 MiB (+22%)**. The system allocator tracks live data to within 4.9%; there is no fragmentation to reclaim. |
| Bound rayon's thread count | Saturates at ~97 MiB (1.5%) and costs +47-54% wall clock. One refuter measured it as a +138 MiB regression. |
| Narrow the parallel page fold | The v2-S15 refusal in disguise, exactly as `extract.rs:1030` warns. Ceiling <=21.6 MiB; realizable -4.7 MiB, i.e. worse. |

## 7. The corpus after the cut, and what a caller can be told

Re-measured at `58a1342`, median of 5 — the coefficients after role-path sharing, superseded by
§11 for the engine that ships after the adopt change; §2's table is the baseline that motivated the
change:

| fixture | pages | artifact | peak RSS | /input | /artifact | MiB/page |
| --- | --- | --- | --- | --- | --- | --- |
| irs-f1040sd-2025 | 2 | 0.64M | 13.8M | 148x | 21.53x | 6.89 |
| irs-fw9 | 6 | 0.80M | 17.9M | 133x | 22.42x | 2.98 |
| nist-sp-800-218 | 36 | 28.73M | 199.1M | 282x | 6.93x | 5.53 |
| nist-sp-800-207 | 59 | 42.67M | 214.0M | 232x | 5.02x | 3.63 |
| nist-sp-800-171r3 | 120 | 81.96M | 488.0M | 321x | 5.95x | 4.07 |
| nist-sp-800-37r2 | 183 | 207.66M | 912.3M | 421x | 4.39x | 4.99 |
| nist-sp-800-161r1 | 327 | 271.55M | 1398.2M | 303x | 5.15x | 4.28 |
| nist-sp-800-53Ar5 | 733 | 950.48M | **4665.6M** | 655x | 4.91x | 6.37 |

The worst case is **4.56 GiB, down from 6.49**. Per-page is now 2.98-6.89 MiB (median 4.63, spread
2.31x) and the input ratio 133x-655x — still a 4.9x spread, so it is still not a bound and the
withdrawal in §3 stands. Peak/artifact on the six real documents tightens to 4.39x-6.93x.

**The floor did not move, as predicted:** `--max-pages 0` costs 221.4 MiB on `53Ar5` against 224.2
before, 45.2 against 45.3 on `171r3`. A document admitting no pages has no runs whose role path
could be shared, so the term §5 identifies is untouched. That is a check on the mechanism, not just
a repeat reading.

Peak is two terms, both knowable before the run:

    peak ~= floor(total_pages) + k x admitted_pages

with `floor` at 0.18-0.32 MiB per document page and `k` at 3.0-6.9 MiB per admitted page. For
sizing, the worst observed coefficient is the only safe one:

**Budget 7 MiB per admitted page, plus 0.35 MiB per page in the document.** On `53Ar5` that
predicts 5.26 GiB against 4.56 GiB measured — over by 15%, which is the direction an estimate for
provisioning should err.

## 8. What this still does not settle

- **There is no ceiling a caller can set.** This is the part 6.2's title asked for and it remains
  open. `--max-pages` bounds the dominant term but not the floor, and `MAX_SOURCE_BYTES` permits a
  2 GiB input that nothing maps onto a memory figure. Two shapes are unmeasured: a named
  `ResourceLimit` refusal when `total_pages x coefficient` exceeds a caller-supplied ceiling, and a
  decompression ceiling on the PDF load path to match `zip::MAX_INFLATED_BYTES`, which the office
  path has had since v2-S13. Whether a byte-denominated ceiling is wanted at all is a design call:
  `profile.rs:975` records the position that pages are the knob.
- **The 838 MiB disagreement is settled in §9**, and neither side was right about today's engine:
  freeing the lopdf object graph earlier RAISES peak footprint, by 668 MiB on `53Ar5`. Refused.
- **Whether the floor's structure-tree share can be bounded by the page budget.** One audit lens
  put it at 32.1 MiB of `53Ar5`'s 221 MiB floor and byte-identical; nobody built it.

Do not re-propose chunking the parallel page fold, streaming the artifact buffer, a different
allocator, freeing the object graph or the extract sooner, or reserving c14n's output buffer at its
final size. All six are in §6's, §9's and §11's tables with the measurement that killed them.

## 9. Freeing memory earlier made the peak worse, and was refused

§8 left one disagreement open. An auditor claimed that dropping the lopdf object graph before
`to_representation` saved **838 MiB**, measured on a built prototype; its refuter measured the whole
graph at **149 MiB** and called 838 arithmetically impossible. The auditor's own write-up conceded
only ~190 MiB is actually freed and put the rest down to allocator high-water — so the two agreed on
the bytes and disagreed on whether peak memory can fall by more than the bytes freed. Both had
measured the engine BEFORE role-path sharing moved the peak.

Rebuilt from its description (its source was not saved) and measured against today's engine, with a
second arm that also drops the whole extract page graph before `seal` — the only other large value
dead by then, since `to_representation` reads nothing but small fields after its page loop.

Interleaved, 5 runs per arm, every artifact hashed:

| document | arm | peak RSS | Δ | peak **footprint** | Δ |
| --- | --- | --- | --- | --- | --- |
| nist-sp-800-53Ar5 | baseline | 4664.4M | — | 4060.6M | — |
| | drop the object graph | 5007.9M | **+343.5** | 4729.0M | **+668.4** |
| | … and the extract | 5003.0M | +338.6 | 4730.1M | +669.6 |
| nist-sp-800-37r2 | baseline | 911.1M | — | 807.6M | — |
| | drop the object graph | 1121.8M | **+210.7** | 1048.9M | **+241.3** |
| | … and the extract | 1123.4M | +212.3 | 1049.7M | +242.0 |
| nist-sp-800-171r3 | baseline | 490.8M | — | 382.5M | — |
| | drop the object graph | 512.6M | +21.8 | 450.0M | +67.6 |
| | … and the extract | 513.8M | +23.0 | 450.2M | +67.7 |

Every artifact byte-identical across all three arms.

**Freeing earlier raised the peak.** It is not noise: two independently built binaries show the same
increase, and the individual runs are tight (37r2's five readings for the first arm span
1120-1123 MiB). Nor is it pages the OS could take back for free, which was the first hypothesis:
macOS's `time -l` reports **peak memory footprint** beside RSS, footprint excludes pages the
allocator has handed back as reusable, and footprint rises MORE than RSS does. The change genuinely
raises the memory macOS charges the process — by far more than the graph it frees weighs.

So the dispute resolves with neither side right on today's engine. The auditor's −838 MiB was a
real measurement of an engine that no longer exists; the same change is **+668 MiB** of footprint
now. The refuter was right that only ~150 MiB is freed and wrong that the effect is bounded by it.
What both missed is that the sign is not stable: this is an allocator-layout effect, it flips
between engine versions, and a change whose effect can reverse on the next unrelated commit is not
one to ship.

Dropping the extract before `seal` adds nothing on top — it frees several hundred MiB of live runs
and the peak does not move. That is the same lesson from the other side. **In this engine, on this
platform, memory is released by not allocating, not by freeing sooner.** Role-path sharing worked
because 1.65M clones were never made; these two free values that were made, and the allocator does
not hand the space to what comes next.

**Refused, with the numbers above.** Do not re-propose either drop without a footprint A/B against
the engine as it stands, because the answer measured here was the opposite of the answer measured
one commit earlier.

### RSS overstates what macOS counts, in the safe direction

A side result worth keeping: baseline footprint runs **13–22% below RSS** — 4060.6 against 4664.4
MiB on `53Ar5`, 382.5 against 490.8 on `171r3`. Footprint is what macOS's jetsam acts on, and every
coefficient in §7 is RSS. So the published sizing rule over-provisions on macOS, which is the
direction an estimate for provisioning should err. Linux reports no footprint through `time -v`;
RSS stays the portable measure and the one this directory publishes.

## 10. MCP parsed every artifact back into a tree, and no longer does

Everything above was measured on the CLI. `extract` over MCP is the same engine and had never been
measured at all — and it was not the same cost. `mcp.rs`'s `tool_extract` built the representation,
serialized it to canonical bytes, then handed those bytes to a `canonical()` helper that ran
`serde_json::from_slice` over the WHOLE artifact so it could sit inside a `json!` response; `serve`
then serialized that response, tree and all, back into one String to print it. The artifact existed
three times over, and the middle copy is a JSON value tree several times the text it came from.
`tool_ground` did the same.

Same binary, one request over stdin, median of 3:

| document | CLI peak | MCP before | MCP after | change |
| --- | --- | --- | --- | --- |
| irs-fw9 | 17.8M | 38.3M | 17.9M | −53% |
| nist-sp-800-218 | 195.7M | 1015.6M | 199.7M | −80% |
| nist-sp-800-171r3 | 490.3M | 2790.3M | 485.7M | −83% |
| nist-sp-800-161r1 | 1399.1M | **8923.2M** | 1128.4M | **−87%** |
| nist-sp-800-53Ar5 | 4668.9M | not run | 3778.0M | — |

**A 4.6 MB PDF needed 8.7 GiB over MCP** — 6.4x its own CLI peak, the multiplier growing as the
document does — on the surface `mcp.rs` itself calls the one that matters most: a long-lived process
handling untrusted documents repeatedly. The 733-page document was deliberately never run through
the old route; on that trend it needed ~30 GB. It now needs 3.7 GiB.

**Not claimed:** MCP reads 0.81x the CLI on the two largest documents. The arms were not
interleaved, and §9 shows heap layout alone moving this engine's peak by hundreds of MiB in either
direction. The claim is that MCP now runs at about the CLI's own peak — not that it is cheaper.

### Byte-identical where it ships, and canonical everywhere now

The fix never builds the tree. Tools return canonical bytes, and an artifact reply writes every key
in sorted order — `id`, `jsonrpc`, `result`; then `content`, `isError`, `structuredContent`; and the
summary object's `text`, `type` — with the artifact arriving already canonical. In a release build
that is exactly what the old route printed: `serde_json` sorts keys there, the artifact was last at
both levels, and a probe found the old response was literally a 162-byte prefix, the CLI's bytes,
and `}}`. Verified with the final build across **84 MCP sessions** — `extract`, `ground` and
`node_get` over every gate, engine and conformance fixture — with zero differing bytes and zero old
responses that had already drifted from the CLI; and on `nist-sp-800-161r1`, the headline document,
both builds emit the same 284,739,383 bytes (sha256 `fd2e93a1…`).

**The first version of this fix failed the gate, and the failure found an older defect.** Its guard
compared the spliced reply against a `json!` tree serialized by `serde_json`, and passed under
`cargo test -p ethos-parser-cli`. Under `ci/gate.sh` it failed, because the gate runs
`cargo test --workspace`, which builds core's tests — and core enables `serde_json/preserve_order`
in its dev-dependencies, precisely because it is a hazard. Resolver 2 keeps that feature out of
`cargo build`, but in a workspace test build `serde_json` is compiled once with it for every crate,
so the tree printed its keys in INSERTION order. That meant the old route's envelope was sorted in
the build that ships and insertion-ordered in the build the gate tests, for as long as `mcp.rs` has
existed. The artifact inside never differed, because c14n sorts at write time.

So the final reply does not borrow `serde_json`'s order anywhere, including the summary object the
first version still printed through `Display`. `an_artifact_reply_is_canonical_in_every_build`
compares it with `c14n_bytes` of the same envelope — over a nested object, an array, an integer, and
a summary carrying a quote, a backslash and a non-ASCII character — and because the gate runs it
under `preserve_order`, the hazard is exercised rather than described.

## 11. Reserving the buffer bought nothing; adopting it bought 20%

§10 left one candidate: `seal` canonicalizes the payload into a `Vec` that grows while the run sits
at its peak, and `CanonicalMap::finish` knew every entry's length before writing one and pre-sized
nothing. Reading `finish` closely showed two things happening there, of about the same size:

1. **The copy.** Sorting keys means every field is serialized into its own staging buffer first,
   then copied into `out`. The payload's largest staging buffer is `nodes` — 804.8 MiB, 99.9% of the
   payload on the largest gate document — and for the length of that copy it existed twice.
2. **The regrowth.** `out` was sized to fit `nodes` exactly, and the 250 KB of fields that sort
   after it made it regrow to 1.61 GiB.

Pre-sizing — the change that had been proposed — removes only the second. The first needs the
buffer not to be copied at all: at the top level nothing has been written to `out` yet, so `finish`
can build the result AROUND the largest staging buffer — shift its bytes right in place, write the
prefix into the gap, append the suffix. Same bytes, same order, one buffer.

Both were built and measured, interleaved, five runs per arm, every artifact hashed:

| document | pre-size: RSS / footprint | adopt: RSS / footprint | adopt: wall |
| --- | --- | --- | --- |
| nist-sp-800-53Ar5 | +0.4 / −13.7 | **−953.5 / −949.5** | +0.4% |
| nist-sp-800-161r1 | −1.2 / +135.5\* | **−271.3 / −274.1** | +2.1% |
| nist-sp-800-171r3 | +0.8 / +6.6 | **−81.0 / −67.0** | +1.0% |
| nist-sp-800-37r2 | +0.9 / −2.8 | −0.8 / −78.6 | +1.3% |

\* bimodal runs (1104–1328 MiB); not a regression.

The code that ships — the same logic with the experiment's label removed and its tests added — was
then measured again against the baseline, on its own:

| document | shipped adopt: RSS / footprint |
| --- | --- |
| nist-sp-800-53Ar5 | **−947.9 / −910.9** (4664.8 → 3717.0 MiB) |
| nist-sp-800-161r1 | −270.0 / −276.6 |
| nist-sp-800-171r3 | −77.4 / −66.3 |
| nist-sp-800-37r2 | −0.0 / −67.8 |

It reproduces the experimental arm within noise, byte-identical in every run — including the 37r2
anomaly described below, which is therefore a property of the change and not of one run.

**Pre-sizing is refused.** Zero on every axis, including wall time. The regrowth it prevents only
ever added capacity nobody wrote, and an allocation that is never written is not memory — which is
the auditor's own caveat, and the reason a sibling proposal to trim the representation's `Vec`
capacity measured 0 to −22 MiB. It is the same lesson as §9 from the other side.

**Adopting is shipped.** `nist-sp-800-53Ar5` goes from 4669.8 to 3716.2 MiB, tight across runs
(3712–3718). The corpus's worst case is now **3.65 GiB, down from 6.49 GiB when 6.2 started — 44%**,
in three byte-identical changes: role-path sharing, and this, with MCP brought down to the CLI's
level in between. The cost is one in-place shift of the adopted buffer, 0.4–2.1% of wall time.

One result is not explained, so it is recorded rather than smoothed over: on `nist-sp-800-37r2`
adopting leaves peak RSS unchanged while footprint falls 79 MiB. The likeliest reading is that
37r2's RSS peak falls outside `seal`, so removing a duplicate inside `seal` cannot lower it; that
has not been verified.

### The corpus after the adopt change

Re-measured on the shipped build with `ci/bench.py --repeat 5`
([`bench-c14n-adopt.tsv`](bench-c14n-adopt.tsv)); §7's table is the post-`Arc` record:

| fixture | pages | artifact | peak RSS | /input | /artifact | MiB/page |
| --- | --- | --- | --- | --- | --- | --- |
| irs-f1040sd-2025 | 2 | 0.64M | 13.2M | 141x | 20.56x | 6.58 |
| irs-fw9 | 6 | 0.80M | 18.1M | 135x | 22.65x | 3.01 |
| nist-sp-800-218 | 36 | 28.73M | 169.4M | 240x | 5.90x | 4.70 |
| nist-sp-800-207 | 59 | 42.67M | 205.3M | 223x | 4.81x | 3.48 |
| nist-sp-800-171r3 | 120 | 81.96M | 403.7M | 266x | 4.93x | 3.36 |
| nist-sp-800-37r2 | 183 | 207.66M | 914.9M | 423x | 4.41x | 5.00 |
| nist-sp-800-161r1 | 327 | 271.55M | 1129.2M | 244x | 4.16x | 3.45 |
| nist-sp-800-53Ar5 | 733 | 950.48M | **3736.2M** | 524x | 3.93x | 5.10 |

Per-page is now **3.01–6.58 MiB** (median 4.09, spread 2.18x). The input ratio is 135x–524x, still
a 3.9x spread, so §3's withdrawal stands. Peak/artifact on the six real documents is 3.93x–5.90x, a
spread of 1.50x. **The worst case is 3.65 GiB**, 44% below the 6.49 GiB §2 measured. (3736.2 MiB
here against 3717.0 in the A/B above: two separate runs 0.5% apart, inside this instrument's noise.)

The floor did not move — 222.1 MiB at `--max-pages 0` on `53Ar5`, 44.9 on `171r3` — which is the
prediction: a document admitting no pages has a payload too small for adopting its largest field to
matter.

**The sizing rule stays 7 MiB per admitted page plus 0.35 per document page, and it is now loose.**
Its per-page term is the worst coefficient observed, and that is 6.58 on `irs-f1040sd-2025` — a
two-page form, where the ~13 MiB process floor dominates and adopting has nothing to adopt. On
`53Ar5` the rule now predicts 5.26 GiB against 3.65 measured, over by 44%. Loose is the safe
direction for provisioning. Tightening it would mean a two-term rule with a separate process floor,
which changes what is published rather than re-measuring it, and is not done here.

## 12. Reading an artifact back in costs more than producing it

Everything above measured the WRITE side — `extract`, and MCP's `extract`. Nothing had measured the
commands that READ a representation back in: `ground`, `markdown`, `html`, and MCP's `ground` and
`node_get`. They share one load path: the whole file into one buffer, `serde_json::from_slice` into
the typed `DocumentRepresentation`, `verify_fingerprint`, then project and emit. `grounding-check`
has a different one — it reads a grounding artifact, not a representation.

Instruments: [`abload.py`](abload.py), [`mcpload.py`](mcpload.py), [`gcheck.py`](gcheck.py).

### The baseline

| document | representation | `ground` RSS / footprint | MCP `node_get` (load + verify only) | `extract` |
| --- | --- | --- | --- | --- |
| nist-sp-800-171r3 | 82.0 MiB | 344.6 / 343.0 | 286.4 | 403.7 |
| nist-sp-800-37r2 | 207.7 | 798.3 / 797.0 | 702.9 | 914.9 |
| nist-sp-800-161r1 | 271.5 | 1102.0 / 1082.5 | 926.0 | 1129.2 |
| **nist-sp-800-53Ar5** | **950.5** | **4267.1 / 4216.6** | **3683.4** | 3736.2 |

**On the largest document, `ground` peaks 531 MiB above `extract`.** MCP's `node_get` is given an id
that does not exist, so it pays the load and the verify and fails closed: its peak is the load
path's cost with no projection on top, and on `53Ar5` that alone is 3683 MiB — what producing the
artifact cost. MCP's `ground` matches the CLI's (4264.8 against 4267.1), so §10's fix holds.

Footprint sits within 1–2% of RSS here, unlike `extract`, where it ran 13–22% lower. The load path
is dominated by a few huge allocations, which macOS charges in full.

### Where the load path's memory and time go

Measurement-only arms, built in a scratch worktree and never shipped, against the baseline,
interleaved, three runs per arm, output hashed on every run:

| `ground` on `53Ar5` | peak RSS | footprint | wall |
| --- | --- | --- | --- |
| baseline | 4265.7 MiB | 4216.9 | 18.38 s |
| A1 — skip `verify_fingerprint` | **−804.8** | −806.9 | **8.28 s (−54.9%)** |
| A2 — free the file buffer once parsed | −0.9 | −0.9 | −0.5% |
| A3 — both | −805.3 | −805.6 | −54.9% |

The same arms across every read command and document, output byte-identical in all eight cases:

| case | baseline | A1 skip verify: RSS / footprint / wall | A2 free buffer: RSS | A3 both: RSS |
| --- | --- | --- | --- | --- |
| `markdown` 53Ar5 | 3975.5M, 16.38 s | −763.6 / −795.9 / **−61.3%** | −8.1 | −797.5 |
| `html` 53Ar5 | 4129.7M, 17.28 s | −667.9 / −742.1 / **−58.9%** | +11.8 | −793.3 |
| `ground` 161r1 | 1103.7M, 4.92 s | −226.2 / −226.2 / **−60.0%** | −0.6 | −228.4 |
| `markdown` 161r1 | 1041.8M, 4.54 s | −200.4 / −223.4 / **−64.3%** | −3.6 | −221.9 |
| `html` 161r1 | 1077.3M, 4.78 s | −215.3 / −226.9 / **−61.4%** | −1.0 | −226.6 |
| `ground` 37r2 | 797.2M, 3.70 s | −100.1 / −118.5 / **−60.8%** | +0.0 | −173.6 |
| `ground` 171r3 | 344.0M, 1.46 s | −68.4 / −68.4 / **−59.2%** | +0.2 | −68.5 |

Freeing the buffer alone never helps (−8 to +12 MiB). Once verification stops allocating it sometimes
adds a little — 74 MiB more on 37r2, 125 more on `html` 53Ar5 — because the peak moves to an instant
where the buffer is still counted. An interaction, not a fix.

**Verification is the load path's dominant cost: 55–64% of the wall time on every read command at
every size, and about the canonical payload's size in memory — 805 MiB on 53Ar5.** The
805 MiB is the canonical payload's size exactly — `fingerprint()` rebuilds all of it to hash it and
throw it away. With `sha2` 0.10 dispatching to the ARMv8 SHA-2 instructions on this machine,
hashing 805 MB should take well under a second, so most of the ~10 s is the re-serialization; that
split is inferred, not measured.

**Freeing the 950 MB file buffer early does nothing** — §9's lesson a third time. In this engine,
memory comes down by not allocating, not by freeing sooner.

### Why a streaming hash would not help, and what would

The obvious fix — hash as a stream instead of into a `Vec` — saves nothing here. c14n sorts keys by
staging each field of an object in its own buffer, so the payload's `nodes` field is materialized
in full, 805 MiB, before a single byte could reach the hasher. §11 removed the COPY of that buffer;
it did not remove the buffer.

What would work is not re-serializing at all: the file is canonical JSON, so its `representation`
member IS the canonical payload, and hashing that byte span in place costs neither the buffer nor
the walk. But it is **not verdict-identical**, and an audit of the representation types says why:

- unknown fields are refused everywhere — every payload struct, and the envelope through
  `RepresentationWire` — so that route is closed;
- but 21 `skip_serializing_if` fields parse an explicit `null` or empty value exactly as their
  absence, and 4 `serde(default)` sites fill a missing field — among them `tables`, which is always
  emitted. Around 25 places where two different byte strings parse to the same value.

So hashing the input span would accept a narrow class of hand-crafted payloads today's check
rejects — explicit empty arrays written into the JSON with the digest recomputed over those raw
bytes — and the engine would then speak for a record whose declared fingerprint is not the
canonical encoding of what it parsed. The options, which are an owner's decision and not a
measurement's:

| option | memory | the ~10 s | semantics |
| --- | --- | --- | --- |
| A. a payload-aware streaming canonical hash | −805 MiB | mostly stays | identical |
| B. hash the input span; fall back to today's check on mismatch | −805 MiB | mostly gone | accepts crafted inputs today's check rejects |
| C. make parsing lossless for canonical input, then B | −805 MiB | mostly gone | a stricter reader — a MINOR, and more work |

### `grounding-check` has the worst ratio in the engine

| document | grounding artifact | peak RSS / footprint | RSS / input |
| --- | --- | --- | --- |
| nist-sp-800-171r3 | 15.3 MiB | 205.9 / 177.9 | 13.5x |
| nist-sp-800-37r2 | 37.4 | 492.5 / 425.5 | 13.2x |
| nist-sp-800-161r1 | 47.8 | 639.5 / 539.2 | 13.4x |
| nist-sp-800-53Ar5 | 151.4 | 1922.8 / 1691.6 | 12.7x |

`grounding_check` parses the artifact twice — into a `serde_json::Value` tree (`check.rs:483`) and
then into the typed `GroundingSource` (`check.rs:511)` — the tree-multiplier shape §10 removed from
MCP. Not changed here.

### Found on the way: `ground` emits an artifact its own checker rejects

On the largest gate document, `ground` writes a grounding artifact that `ethos.grounding.v1` rejects
— and the Ethos verifier refuses it too. `grounding-check` exited 1 on it while this section was
being measured, and three independent checks then agreed:

| check | `nist-sp-800-53Ar5`'s grounding artifact |
| --- | --- |
| the engine's `grounding-check` | exit 1 — `invalid`, `limit_exceeded` at `/` |
| exact sizes, [`gcount.py`](gcount.py) | **1,619,510 spans** against a 1,000,000 limit; its 50,329 elements and longest string (2,094 bytes) are within theirs |
| the Ethos verifier, `ethos grounding check` 0.6.0 | exit 2 — `invalid`, `limit_exceeded` at `/`: "reduce the submitted artifact within the measured limits" |

The other three gate documents are valid under both checkers, carrying 159,594 to 498,561 spans.
`ethos.grounding.v1` caps `elements` and `spans` at 1,000,000 each (`check.rs`, `mod limits`,
differential-tested against Ethos down to verdict, code and path). Every node with a measurable ink
box becomes one span, and the four documents carry 1,330 to 2,210 spans a page — so the cap is
reached somewhere between about 450 and 750 pages of text. `ground`'s projection enforces none of the
schema's limits, so above that size it emits an artifact the verifier refuses outright: the grounding
this engine exists to provide fails, silently, for every sufficiently large document, while `ground`
itself exits 0.

The gate is green because no test grounds a document this large and then checks what it wrote.

**Not fixed here.** What `ground` should do past the cap is a contract decision. The schema already
makes `spans` optional (`capabilities.spans`), which suggests one shape — emit the elements alone and
declare the omission, keeping block-level grounding for large documents — beside the plainer one of
refusing by name, as `MAX_SOURCE_BYTES` does. Either is better than an artifact the verifier rejects.

**Fixed afterwards with option A** (branch `fix/grounding-span-cap`). Past the cap `ground` now
keeps every element and withholds the spans — all of them, never truncated — declared by
`capabilities.spans: false` in the artifact, a stderr note, and MCP's `ground` summary. On the
733-page document both checkers now accept the artifact — the engine's `grounding-check` and `ethos
grounding check` each exit 0, where they returned 1 and 2 — and it carries its 50,329 elements, no
spans, and is 6.6 MiB instead of 151.4. `ground` on the three next-largest gate documents is
byte-identical to before. Under the cap nothing changes. The schema's other limits are still not
enforced by the projection; none is reached by this corpus.
