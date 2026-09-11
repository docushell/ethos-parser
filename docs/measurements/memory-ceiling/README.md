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

Re-measured at `58a1342`, median of 5 — these are the SHIPPED coefficients; §2's table is the
baseline that motivated the change:

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
predicts 5.3 GiB against 4.56 GiB measured — over by 16%, which is the direction an estimate for
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
allocator, or freeing the object graph or the extract sooner. All five are in §6's and §9's tables
with the measurement that killed them.

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
