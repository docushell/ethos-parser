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

*The attribution above is a reading of the code; §15 measured the share. The structure tree is
27.9 MiB of `53Ar5`'s 221 MiB floor (12.6%); the rest is the source bytes and the parsed object
graph, which `classify --sample-pages 0` pays too.*

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
| Stream the artifact instead of one `Vec<u8>` | A real 950 MiB at the print instant — but that instant sits 1.1-2.1 GiB BELOW the high-water mark. Peak does not move. **Amended 2026-09-26:** measured at 97fa562, before §11 took ~950 MiB out of `seal`. After §11 the print instant WAS the high-water mark, and nobody re-measured. Streaming it (`DocumentRepresentation::write_canonical_to`, branch `perf/c14n-allocation-free`) takes `nist-sp-800-53Ar5` from 3825.2 to 3059.9 MiB and `nist-sp-800-161r1` from 1161.4 to 875.4, interleaved, median of 3, byte-identical. |
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
provisioning should err. *Restated in §15 with the process floor separated.*

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
  put it at 32.1 MiB of `53Ar5`'s 221 MiB floor and byte-identical; nobody built it. §15 measures
  the share at 27.9 MiB — 12.6% of the floor, 0.75% of the peak — so that is the most bounding it
  could save.

Do not re-propose chunking the parallel page fold, streaming the artifact buffer, a different
allocator, freeing the object graph or the extract sooner, or reserving c14n's output buffer at its
final size. All six are in §6's, §9's and §11's tables with the measurement that killed them.
**Amended 2026-09-26:** streaming the artifact buffer is off this list. §6 refused it on a
measurement §11 made stale; re-measured after §11 it takes 765 MiB off `nist-sp-800-53Ar5`'s
peak, and `extract` streams now (§6's table). The other five stand.

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

**Amended 2026-09-26:** the adopt path is gone (branch `perf/c14n-allocation-free`).
`CanonicalMap` now writes every member straight into the output and, when members arrive out of
key order, moves the largest one within the output and the rest through one reused buffer, so
`nodes` is still never copied out. Against the build that adopted, interleaved, median of 3:
`nist-sp-800-53Ar5` peaks at 3818.3 MiB against 3824.6 and runs 7.31 s against 12.89,
byte-identical.

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
which changes what is published rather than re-measuring it, and is not done here. *Done in §15 at
0.58.0: 7 MiB + 5.4 MiB per admitted page + 0.33 MiB per document page, over by 12.9% on `53Ar5`
and by 3.6% at its tightest point.*

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
throw it away. ~~With `sha2` 0.10 dispatching to the ARMv8 SHA-2 instructions on this machine,
hashing 805 MB should take well under a second~~ — **wrong, see §13: 0.10 never used those
instructions here, and the hash was a measured 1.1 s of it.** Most of the ~10 s is still the
re-serialization; the rest of that split is inferred, not measured.

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

**Option A landed afterwards** (branch `perf/streaming-fingerprint`).
`RepresentationPayload::fingerprint` now writes the payload's eight members straight into a hashing
sink in the order c14n sorts them, serializing each node directly into the sink's 64 KiB buffer, so
verification's peak is a block rather than the document. Measured against the build before it:
`ground` on the largest gate document 4093 → 3299 MiB (−19%), `markdown` −766 MiB, 161r1 −226 MiB,
171r3 −70 MiB, wall time within noise (−1.3% to +1.4%), output byte-identical. A first version that
copied each node through a scratch buffer saved the same memory at 4–7% of wall clock; removing the
copy removed the cost. It saves memory, not time: the node walk is still most of a load's wall
clock, and options B and C remain the only routes to that — at the semantics they cost.

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

**Fixed afterwards** (branch `perf/grounding-check-single-parse`). The tree was the peak, not the two
parses together: it is dropped before the typed parse, and a build that skipped it peaked at 62 MiB
on 171r3. The tree and its two walks are now built only for an artifact that fails to parse. A scan
that allocates nothing applies `strict_value`'s rules while streaming the bytes; if it is clean, a
typed parse that succeeds is the answer, because every type `reject_unknown_fields` walks denies
unknown fields with exactly the keys it allows. Anything else takes the old path unchanged. One
binary pair, [`gcheck.py`](gcheck.py) for memory, [`gcheckab.py`](gcheckab.py) for wall (medians of
5, interleaved):

| document | grounding artifact | peak RSS / footprint | RSS / input | wall |
| --- | --- | --- | --- | --- |
| nist-sp-800-171r3 | 15.3 MiB | 205.9 / 177.8 → **62.2 / 61.0** | 13.5x → **4.1x** | 0.195 → 0.108 s |
| nist-sp-800-37r2 | 37.4 | 491.9 / 425.4 → **142.7 / 141.5** | 13.1x → **3.8x** | 0.440 → 0.230 s |
| nist-sp-800-161r1 | 47.8 | 639.4 / 539.1 → **194.2 / 193.1** | 13.4x → **4.1x** | 0.572 → 0.305 s |
| nist-sp-800-53Ar5 | 151.4 | 1922.8 / 1691.5 → **489.4 / 488.5** | 12.7x → **3.2x** | 1.433 → 0.583 s |

Reports are byte-identical, and not only on these: [`gcdiff.py`](gcdiff.py) ran 3,681 mutated
artifacts through both binaries — each strict-value rule, unknown and repeated keys, two faults at
once, raw byte edits, 13 report codes in all — and exit code, stdout and stderr agreed on every one.
**The cost falls on an artifact that fails to parse**: it pays the scan — and, if that passed, a typed
parse as far as the fault — before the old path, so 161r1 with an unknown key at its root or junk after it peaks where
it did (540 MiB) and takes ~75 ms longer (0.31 → 0.38 s).

**Then the old path became Ethos's walk** (branch `fix/grounding-check-ethos-drift`), and that moved
the invalid side again. The tree is now built only when the scan passed and the typed parse did not,
because only then can it matter; an artifact the scan refuses is answered by a second pass that
builds nothing. Against the build above, same method: valid artifacts within noise (+1-4% wall,
identical memory); 53Ar5 — refused for its 1.6 million spans — 489.4 → **153.9 MiB**, 0.56 → 0.45 s;
161r1 with junk after it 540.3 → **50.2 MiB**, 0.34 → 0.24 s; 161r1 with an unknown key at its root
the same 540 MiB and 0.37 → 0.46 s, which is Ethos's sorted, duplicate-checked tree being built.

**Released as 0.57.0, and measured against 0.56.0 directly** rather than through the build between,
[`gcheckab.py`](gcheckab.py), medians of 5 interleaved:

| grounding artifact | peak RSS | wall |
| --- | --- | --- |
| nist-sp-800-171r3, 15.3 MiB | 205.9 → 62.2 MiB | 0.186 → 0.104 s |
| nist-sp-800-37r2, 37.4 MiB | 492.5 → 142.8 MiB | 0.416 → 0.220 s |
| nist-sp-800-161r1, 47.8 MiB | 639.4 → 194.3 MiB | 0.545 → 0.295 s |
| nist-sp-800-53Ar5, 151.4 MiB, refused for its spans | 1922.8 → 153.9 MiB | 1.357 → 0.452 s |
| 161r1 with an unknown key at its root | 540.3 → 540.4 MiB | 0.301 → 0.451 s |
| 161r1 with junk after it | 540.3 → 50.3 MiB | 0.271 → 0.239 s |

The report counts in 0.57.0's CHANGELOG entry are [`gcdiff.py`](gcdiff.py)'s mutations of
`ruled-table-grid`, `markdown-two-blocks` and `irs-fw9`'s grounding artifacts, **in that order** —
one random generator serves every seed, so another order is another corpus. Of 3,681: 162 valid in both, every report byte-identical; 3,519 invalid in
both, of which 198 changed code or path, 1,352 kept the report byte for byte, and 1,969 kept code and
path but not the message.

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
enforced by the projection; none is reached by this corpus. *Since enforced (G2): an over-long element is omitted, over-long
cells or too many tables withhold the tables, and too many pages or elements — or an artifact over
256 MiB — is refused; see `ethos-parser-grounding`'s crate docs.*

## 13. The hash was running in software, and moving to `sha2` 0.11 is 9–13% of each command measured

**Run 2026-09-13 at `15ebe51` (0.55.0) against the same tree with `sha2 = "0.11"`.** Both release
builds at equal version; interleaved; every run's output hashed.

§12 said `sha2` 0.10 dispatches to the ARMv8 SHA-2 instructions on this machine. It does not. Its
`sha256.rs` selects a backend at compile time: x86 gets runtime SHA-NI detection, but on `aarch64`
the hardware path sits behind `#[cfg(all(feature = "asm", target_arch = "aarch64"))]`, and this
workspace does not enable `asm` (which would pull `sha2-asm` and a `cc` build script). Every
`aarch64` build this repository has shipped hashed SHA-256 with the portable implementation. `sha2`
0.11 detects the extension at run time through `cpufeatures` with no feature flag, and 0.11.0 was
already in `Cargo.lock` through `lopdf` — so the change adds no crate and removes seven:
`sha2` 0.10.9, `digest` 0.10.7, `block-buffer` 0.10.4, `crypto-common` 0.1.7, `generic-array`
0.14.7, `cpufeatures` 0.2.17 and `version_check` 0.9.5.

| command | document | 0.10 wall | 0.11 wall | Δ |
| --- | --- | ---: | ---: | ---: |
| `extract` | 53Ar5 | 9.37 s | 8.18 s | **−12.7%** |
| `extract` | 161r1 | 2.77 s | 2.46 s | −11.3% |
| `extract` | 171r3 | 0.87 s | 0.76 s | −12.8% |
| `ground` | 53Ar5 (950 MiB) | 11.02 s | 9.94 s | **−9.8%** |
| `markdown` | 53Ar5 | 10.42 s | 9.52 s | −8.7% |
| `ground` | 161r1 | 3.21 s | 2.92 s | −9.0% |
| `ground` | 171r3 | 0.97 s | 0.89 s | −8.9% |
| MCP `node_get`, per call | 53Ar5 | 9.06–9.48 s | 7.97–8.22 s | −11% |

Three runs per arm (five for `extract` 171r3); MCP is two alternating sessions of three calls each.
Peak RSS and footprint move by at most 11 MiB either way, which is noise: the hash never allocated
the memory, it only spent the time. `extract` hashes the payload once to seal it and the read
commands once to verify it, and on 53Ar5 both save about the same 1.1–1.2 s, which is what one
software pass over the 805 MiB payload was costing.

**Byte-identical**: `ci/artifact-bytes.py` over all 268 artifacts equal before and after, every A/B
run's output digest equal across arms, and an `x86_64-apple-darwin` build under Rosetta 2 produces
the same bytes as native. `profile_sha256` does not move — no profile field names the hash
implementation.

**What this is not.** It does not change the split §12 inferred: verification's re-serialization
is still most of its cost, and only not rebuilding the payload — or not verifying twice — removes
that. On Intel Macs nothing changes: 0.10 already detected SHA-NI there, and 0.11 does the same.

The MCP figure comes from [`mcpsession.py`](mcpsession.py), which drives one server through N calls
and refuses to report a run in which any call returned a tool error. Its first version did not, and
a mis-split shell argument produced a "baseline" of 0.0 s per call and 2.4 MiB from calls that all
failed instantly; a separate reading of 14 s per call was taken while ten other processes shared
the machine. Neither is published.

## 14. MCP stops re-verifying bytes it already verified: repeat calls 52–66% faster, and one memory line breached

**Run 2026-09-13.** A = `main` at `d22b12c` (0.55.0 with `sha2` 0.11), B = the same tree plus the
verification ledger in `crates/ethos-parser-cli/src/mcp.rs`, both release builds at equal version.
Instruments: [`ledgerab.py`](ledgerab.py) (interleaved sessions, three per arm) and
[`ledgerpace.py`](ledgerpace.py). `docs/00-NORTH-STAR.md` decision 24 records why the server may keep
anything at all.

### How much of a call verification was

A measurement-only arm with `verify_fingerprint` removed from the MCP route — never shipped, built to
bound what any cache could save — against A, per `node_get` call, output identical:

| representation | A | no verification | share |
| --- | ---: | ---: | ---: |
| 53Ar5, 950 MiB | 8.3 s | 2.8 s | 66% |
| 161r1, 272 MiB | 2.3 s | 0.70 s | 70% |
| 171r3, 82 MiB | 0.69 s | 0.21 s | 70% |

A hit still reads, parses and hashes the file; hashing 950 MiB on the SHA-2 instructions takes
0.43 s (§13). That left a ceiling of about 61–64% per repeat call.

### What the ledger does

| workload (one session) | first call, B vs A | later calls, B vs A | replies |
| --- | ---: | ---: | --- |
| `ground` once, 53Ar5 | 15.62 vs 15.31 s, **+2.0%** | — | identical |
| `ground` once, 161r1 | 4.54 vs 4.45 s, +2.0% | — | identical |
| `node_get` ×5, 53Ar5 | +2.3% | **4.86 vs 12.78 s, −61.9%** | identical |
| `node_get` ×8, 161r1 | +2.8% | 1.22 vs 3.53 s, −65.5% | identical |
| `node_get` ×12, 171r3 | +2.5% | 0.37 vs 1.05 s, −65.3% | identical |
| `ground` ×3, 53Ar5 | +1.8% | 7.28 vs 15.19 s, −52.1% | identical |
| truncated 950 MiB file ×2 | −7.0% | −6.6% | identical |
| a PDF passed as a representation ×2 | ~0 | ~0 | identical |

**The first sight of a file costs +2 to +3%** — one SHA-256 over it — against the +6% line set before
the run. **A file that fails to parse costs nothing extra**, because the hash runs after the parse
succeeds. Every reply digest is identical across arms and runs. These session timings run slower
than the single-call figures above (A's `node_get` 12.8 s here against 8.3 s there) because a
session's later calls run on a heap the earlier ones fragmented; the comparison is within each
workload, where both arms run the same sequence.

### The memory line, and why it was breached

Set before the run: B's peak may not exceed A's worst run by more than max(1%, 32 MiB). **On 53Ar5's
five-call session it did:** B peaked at 4823 MiB RSS and 3769 MiB footprint in two runs of three,
against A's 4174 and 3189. `161r1` and `171r3` peak identically in both arms, and `ground` ×3 on 53Ar5
is within the line (+34 MiB RSS, +11 footprint).

The ledger holds 64 digests at most, so this is not retained data, and [`ledgerpace.py`](ledgerpace.py)
shows what it is. Five B calls on 53Ar5 with a pause before each call after the first:

| pause | later calls | peak RSS | peak footprint |
| ---: | ---: | ---: | ---: |
| none | 4.72–4.74 s | 4808–4820 MiB | 3755–3768 MiB |
| 1 s | 4.74–4.77 s | 4820–4822 MiB | 3768–3770 MiB |
| 2 s | 4.75–4.82 s | 4782–4820 MiB | 3730–3768 MiB |
| 4 s | 4.74–4.79 s | 4461–4464 MiB | 3408–3411 MiB |
| 8 s | 4.78 s | **4174 MiB** | **3039 MiB** |
| A, no pause | 12.36–12.50 s | 4173 MiB | 3121 MiB |

**It is the allocator's reclamation running behind the calls.** Each call frees its parsed tree — an
estimated, unmeasured ~2 GiB — and the next reads a 950 MiB file. A spends ~5 s verifying between the two, which is time for macOS's
allocator to hand the freed pages back; B's next call arrives before it has, and the new buffer lands
on top of pages not yet returned. Given 8 s, B's peak is A's exactly and its footprint is 82 MiB
lower. The same effect shows in resting memory two seconds after the last reply — 161r1 rests at
2345 MiB in B against 1802 in A. When resting memory settles was not measured.

**Shipped anyway, by the owner's decision, with this section as the record.** A host that fires
repeat calls at a 950 MiB representation faster than every 4–8 s will see its server peak up to
~650 MiB higher, in exchange for each of those calls taking 4.9 s instead of 12.8. Nothing here
changes a reply. The candidate that could remove it for both arms — parsing from a hashing reader so
no 950 MiB buffer exists — is not measured; serde_json's reader path is slower than `from_slice`,
so it may cost back what it saves.

## 15. 0.58.0 re-measured, the floor separated, and the rule restated

**Run 2026-09-16 at `b4b4aa9` (0.58.0)** over the eight gate documents, on the machine §2 describes
(Apple M4 Pro, 12 cores, 48 GiB, macOS 26.6.2, no swap in use). **Every table in this section was
produced by one binary:
`target/aarch64-apple-darwin/release/ethos-parser`, whose `--version` prints `ethos-parser 0.58.0`**
— the binary inside the aarch64 release artifact `target/release-artifacts/SHA256SUMS.txt` marks
`verified` (sha256 `03b752ed…`, built 18:56 after the 17:45 merge of the release branch). It was
not rebuilt for this section. `target/release/ethos-parser`, the path `ci/bench.py` uses, was NOT
used: it prints `ethos-parser 0.57.0` and dates from 2026-09-15, before the release commit.

The machine was shared with other measurements while this ran: the load average was 2.3–3.8 over
the runs (the raw file records it per invocation). **Every wall time quoted here was taken under
that load and is not comparable to an earlier section's; peak RSS does not respond to load** and
is the quantity this section is about. Instruments: [`floor.py`](floor.py) takes the readings and
[`floorfit.py`](floorfit.py) prints these tables and the rule from them. Raw readings, one row per
process: [`floor-b4b4aa9.tsv`](floor-b4b4aa9.tsv).

The method is `ci/bench.py`'s: `/usr/bin/time -l` around one process, stdout to `/dev/null`,
`maximum resident set size` in bytes, `peak memory footprint` read beside it. `classify`,
`classify --sample-pages 0` and `extract --max-pages 0` ran three times each and the median is
reported, with the spread; the full extract ran ONCE per document, because the machine was shared,
plus bench.py's chunk-counted size pass wrapped in `time` as a second sample whose sink is a pipe.
The ladder points ran once each, and the point that turned out to set the coefficient four times.

### The corpus at 0.58.0, beside §11

| fixture | pages | artifact | peak RSS | §11 | Δ | /input | /artifact | MiB/page | wall (loaded) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 | 2 | 0.64M | 13.5M | 13.2M | +2.6% | 144x | 21.11x | 6.75 | 0.02 s |
| irs-fw9 | 6 | 0.80M | 18.1M | 18.1M | −0.1% | 135x | 22.65x | 3.01 | 0.02 s |
| nist-sp-800-218 | 36 | 28.72M | 171.8M | 169.4M | +1.4% | 243x | 5.98x | 4.77 | 0.38 s |
| nist-sp-800-207 | 59 | 42.67M | **248.2M** | 205.3M | **+20.9%** | 269x | 5.82x | 4.21 | 0.57 s |
| nist-sp-800-171r3 | 120 | 81.96M | 410.6M | 403.7M | +1.7% | 270x | 5.01x | 3.42 | 1.09 s |
| nist-sp-800-37r2 | 183 | 207.73M | 914.8M | 914.9M | −0.0% | 423x | 4.40x | 5.00 | 2.77 s |
| nist-sp-800-161r1 | 327 | 271.65M | 1136.3M | 1129.2M | +0.6% | 246x | 4.18x | 3.48 | 3.57 s |
| **nist-sp-800-53Ar5** | **733** | 950.50M | **3725.9M** | 3736.2M | −0.3% | 523x | 3.92x | 5.08 | 12.29 s |

The second sample of each, through the pipe: 13.5, 17.5, 171.1, 207.1, 407.0, 914.1, 1134.5 and
3727.8 MiB in the table's order.

**Seven documents are within −0.3% to +2.6% of §11**, which is this instrument's noise for single
samples (§4 put two medians 0.2–0.7% apart). The worst case is 3725.9 MiB, 3727.8 on its second
sample, against §11's 3736.2. **`nist-sp-800-207` is the exception, and the exception is one
sample:** 248.2 MiB on the first run, then 207.1, 208.6, 204.1 and 204.7 — four of five inside
204–209 MiB with §11's 205.3 among them. That first sample is a 20% excursion of the kind §11
recorded on `161r1` (bimodal, 1104–1328 MiB). It stays in every table here, it is the sample the
rule below is checked against, and the rule clears it by 39%.

The artifact `extract` writes is within ±0.1% of §11's size on every document — `53Ar5`
996,675,602 bytes against 996,655,081, `161r1` +0.036%, the two forms −0.07% and −0.09%. The
29.9% growth 0.58.0's CHANGELOG records is in the GROUNDING artifact, an offset pair on each of
2.4 million spans, which `ground` writes and `extract` does not; the representation gained only
the rotated-text fix's measured boxes and lost the invented `TJ` spaces. So the peak had no reason
to move, and it did not. Per page it reads 3.01–6.75 MiB (median 4.49, spread 2.24x), peak over
artifact 3.92x–5.98x on the six real documents.

Not claimed: the full extracts ran 12–13% faster than `bench-c14n-adopt.tsv`'s medians on every
document from 36 pages up. A change that flat across documents is the signature §6 identifies as
machine state — two runs on different days under different load — and not a throughput claim.

### The floor, separated

§5 attributed the floor `--max-pages` cannot lower to the structure tree, because `structure::read`
runs before the budget is consulted. That was a reading of the code, not a measurement of the
share. Three commands on the same document separate it:

- **`classify --sample-pages 0`** reads the source under the ceiling, opens it — `lopdf` parses
  the whole object graph in `Document::open_bytes` — and tallies no page. It never calls
  `structure::read` (`classify.rs` does not name the module). Its peak is the source bytes, the
  object graph, and a small artifact.
- **`classify`** does the same and then tallies the operators of eight pages' decoded content
  streams. The OPEN-WORK row proposed it as the control; it is not one, see below.
- **`extract --max-pages 0`** reads and opens the same way, then builds the structure tree and
  `tree_mcids_by_page` over the whole document, runs a rayon fold that admits no page, and seals an
  artifact with no pages.

So `max0 − classify0` is the tree plus whatever else `extract` does document-wide that `classify`
does not. A ninth document sizes that remainder: `fixtures/engine/untagged-shredded-line`, one
page, no `/StructTreeRoot` (its artifact declares `untagged-structure-tree-absent`), so on it the
difference is the remainder alone.

| fixture | pages | `classify --sample-pages 0` | `classify` | `extract --max-pages 0` | full extract | max0 − classify0 | share of max0 | max0 − floor, per document page | full − max0, per admitted page |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 | 2 | 5.7M | 7.7M | 6.5M | 13.5M | +0.8M | 12.8% | 0.000 | 3.51 |
| irs-fw9 | 6 | 6.2M | 7.4M | 7.2M | 18.1M | +1.0M | 14.1% | 0.120 | 1.81 |
| nist-sp-800-218 | 36 | 12.3M | 22.8M | 13.9M | 171.8M | +1.6M | 11.6% | 0.206 | 4.39 |
| nist-sp-800-207 | 59 | 10.9M | 16.4M | 12.2M | 248.2M | +1.4M | 11.1% | 0.097 | 4.00 |
| nist-sp-800-171r3 | 120 | 41.6M | 43.2M | 45.1M | 410.6M | +3.5M | 7.7% | **0.322** | 3.05 |
| nist-sp-800-37r2 | 183 | 53.0M | 59.6M | 58.7M | 914.8M | +5.6M | 9.6% | 0.285 | 4.68 |
| nist-sp-800-161r1 | 327 | 60.8M | 68.2M | 64.4M | 1136.3M | +3.6M | 5.6% | 0.177 | 3.28 |
| nist-sp-800-53Ar5 | 733 | 193.4M | 202.3M | 221.3M | 3727.8M | **+27.9M** | 12.6% | 0.293 | **4.78** |
| control, untagged, 1 page | 1 | 2.8M | 2.9M | 3.4M | 5.0M | +0.6M | — | — | 1.58 |

Medians of three runs; the widest spread of any three is 1.9 MiB (`161r1`, `classify0`). The full
extract column is the worse of each document's two samples. Footprint sits 1.4–1.8 MiB below RSS
on every floor row of the large documents (`53Ar5`: 219.8 against 221.3 at `--max-pages 0`, 192.0
against 193.4 for `classify0`), so the shares are the same in footprint; on the full extracts it
is 14–25% below RSS, as §9 found.

**The floor is not the structure tree. It is the parsed document.** `--max-pages 0` on `53Ar5`
costs 221.3 MiB and `classify --sample-pages 0`, which reads no tree, costs 193.4 of that. The
tree's share of the floor is **5.6% to 14.1% across the corpus** — `161r1` the smallest share,
`irs-fw9` the largest at 1.0 of 7.2 MiB — and **27.9 MiB on `53Ar5`, the largest in bytes**, which
is 12.6% of its floor and 0.75% of its 3725.9 MiB peak. The remainder that is not the tree — the
rayon pool, the id allocator, a page-state entry per page, sealing an empty artifact — is 0.6 MiB
on the one-page control. §8's audit lens estimated the tree at 32.1 MiB of the 221; the
measurement reads 27.9. Bounding the tree by the page budget, the shape §8 left open, could save
at most that on the worst document, and nothing on the other 87% of the floor: the object graph
is built in `Document::open_bytes` before `extract` is called, and no budget consulted inside
`extract` reaches it.

`classify` with its default sample is not a control for this. Its eight-page tally peaks ABOVE
`extract --max-pages 0` on six of eight documents — `218` at 22.8 against 13.9 MiB, 10.5 MiB of
decoded content streams — so `max0 − classify` reads negative there and bounds nothing. The
difference against `--sample-pages 0` is the one used.

**The process floor** is the smallest gate document at `--max-pages 0`: `irs-f1040sd-2025`,
6.5 MiB. The control peaks at 3.4 MiB, so the process itself is at most that and the form's other
3 MiB is its own source and graph. Net of the 6.5, the floor grows at **0.10 to 0.32 MiB per
document page**, worst on `171r3`. §5 gave 0.18–0.32 over `171r3`, `161r1` and `53Ar5`; they read
0.322, 0.177 and 0.293 now, and the four documents §5 did not net out lie at 0.097 (`207`) to
0.285 (`37r2`). Taking the control's 3.4 MiB as the floor instead would read 0.35 on `171r3`; the
rule below is checked against measured peaks, not against the split, so it holds either way.

### What this separates, and what it does not

- It separates the tree's share from the rest of the floor to within the control's 0.6 MiB
  remainder, measured on one page. The remainder was not measured on a large untagged document,
  because the corpus has none; the page-state vector is the only part of it that grows with the
  document.
- It does not separate `structure::read` from `tree_mcids_by_page`: both sit inside the one
  difference.
- It does not separate, inside `classify --sample-pages 0`'s 193 MiB, the 7.1 MiB source buffer
  from the object graph from the process floor. All three are paid by every command that opens
  the document.
- The full extracts are single samples (two counting the pipe sample), and `207` shows what a
  single sample can do.

### Within one document, the chord is not a bound

§4 established that the marginal cost per admitted page is constant enough to make a per-page
coefficient usable. Re-taken at 0.58.0, the ladder shows where a coefficient taken from the
endpoints alone fails:

| fixture | budget | peak RSS | cumulative MiB per admitted page | chord (full − max0)/pages | ratio |
| --- | ---: | ---: | ---: | ---: | ---: |
| nist-sp-800-171r3 | 8 | 67.8M | 2.83 | 3.05 | 0.93x |
| nist-sp-800-171r3 | 32 | 148.1M | 3.22 | 3.05 | 1.06x |
| nist-sp-800-171r3 | 64 | 227.9M | 2.86 | 3.05 | 0.94x |
| nist-sp-800-53Ar5 | 8 | 242.5M | 2.65 | 4.78 | 0.55x |
| nist-sp-800-53Ar5 | 32 | 373.4M | 4.75 | 4.78 | 0.99x |
| nist-sp-800-53Ar5 | 64 | 545.6M | 5.07 | 4.78 | 1.06x |
| **nist-sp-800-53Ar5** | **128** | **907.1M** | **5.36** | 4.78 | **1.12x** |
| nist-sp-800-53Ar5 | 256 | 1477.8M | 4.91 | 4.78 | 1.03x |
| nist-sp-800-53Ar5 | 512 | 2720.7M | 4.88 | 4.78 | 1.02x |

Each point is one run; the 128 point is four (902.1, 902.9, 904.9, 907.1 MiB — the table shows the
worst), because it sets the coefficient. On `53Ar5` the cumulative cost per admitted page rises
from 2.65 at 8 pages to 5.36 at 128 and falls back to 4.78 over all 733: **its first 128 pages
are denser than its average**, the same shape §4 saw at `97fa562` (8.9 at 128 against 8.78
overall) and §6 at `58a1342` (6.61 against 6.06). A rule built on the chord, 4.8 MiB per admitted
page, would predict 863 MiB at `--max-pages 128` against 907.1 measured — under by 4.8% — and sit
under the 256 and 512 points by 0.01% and 0.5%, clearing the 64 point by 1.9%. A bound has to take
the worst cumulative marginal at ANY budget, and that is what the rule below does.

### The rule

    peak RSS <= 7 MiB + 5.4 MiB x admitted pages + 0.33 MiB x pages in the document

**How each coefficient was chosen — the worst observed, then rounded up; no least squares.** The
floor is the smallest gate document's `--max-pages 0` median, 6.48 MiB, rounded to 7. The
per-document-page term is the largest `(max0 − 6.48) / pages` on any gate document, 0.322 on
`171r3`, rounded to 0.33. The per-admitted-page term is the largest `(peak − max0) / admitted` at
any budget on any gate document, every full-extract sample included: 5.36 on `53Ar5` at
`--max-pages 128`, rounded to 5.4. §7's 7 MiB per page was the same method applied to a
two-page form whose total was mostly the process floor; separating the floor is what moves the
coefficient from 7 to 5.4 while the rule gets tighter, not looser.

Checked against every point measured here, each at its worst sample:

| fixture | admitted | measured | rule | over by | §7's rule (7 + 0.35, no floor) | over by |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 | 2 | 13.5M | 18M | +36.7% | 15M | +8.9% |
| irs-fw9 | 6 | 18.1M | 41M | +129.1% | 44M | +144.2% |
| nist-sp-800-218 | 36 | 171.8M | 213M | +24.1% | 265M | +54.0% |
| nist-sp-800-207 | 59 | 248.2M | 345M | +39.1% | 434M | +74.7% |
| nist-sp-800-171r3 | 120 | 410.6M | 695M | +69.2% | 882M | +114.8% |
| nist-sp-800-171r3 (8 of 120) | 8 | 67.8M | 90M | +32.5% | 98M | +44.6% |
| nist-sp-800-171r3 (32 of 120) | 32 | 148.1M | 219M | +48.1% | 266M | +79.6% |
| nist-sp-800-171r3 (64 of 120) | 64 | 227.9M | 392M | +72.1% | 490M | +115.0% |
| nist-sp-800-37r2 | 183 | 914.8M | 1056M | +15.4% | 1345M | +47.0% |
| nist-sp-800-161r1 | 327 | 1136.3M | 1881M | +65.5% | 2403M | +111.5% |
| **nist-sp-800-53Ar5** | 733 | 3727.8M | 4207M | **+12.9%** | 5388M | **+44.5%** |
| nist-sp-800-53Ar5 (8 of 733) | 8 | 242.5M | 292M | +20.4% | 313M | +28.9% |
| nist-sp-800-53Ar5 (32 of 733) | 32 | 373.4M | 422M | +12.9% | 481M | +28.7% |
| nist-sp-800-53Ar5 (64 of 733) | 64 | 545.6M | 594M | +9.0% | 705M | +29.1% |
| **nist-sp-800-53Ar5 (128 of 733)** | 128 | 907.1M | 940M | **+3.6%** | 1153M | +27.1% |
| nist-sp-800-53Ar5 (256 of 733) | 256 | 1477.8M | 1631M | +10.4% | 2049M | +38.6% |
| nist-sp-800-53Ar5 (512 of 733) | 512 | 2720.7M | 3014M | +10.8% | 3841M | +41.2% |

**The rule over-predicts every point, by 3.6% at the tightest — `53Ar5` at `--max-pages 128` —
and by 12.9% on the corpus's worst case, where §7's rule was over by 44.5%.** On the other
documents' full extracts it is over by 15% (`37r2`) to 129% (`irs-fw9`), and that spread is
content density, the same 2.2x §11 measured: a rule that bounds the densest document is loose on
the sparsest by construction. What it does not promise: it is a bound on this corpus at this
build, with 3.6% of margin at its tightest point, and a single sample can move 20% (`207`, above).
A document whose pages are denser than `53Ar5`'s first 128 is outside it.

Published as of this section: the `--max-pages` doc comment in `crates/ethos-parser-cli/src/main.rs`
and the floor comment in `crates/ethos-parser-pdf/src/extract.rs` carry this rule and this
attribution of the floor; §7's and §11's rule paragraphs point here. §8's open items are
unchanged: there is still no ceiling a caller can set, and a page budget cannot reach the object
graph.
