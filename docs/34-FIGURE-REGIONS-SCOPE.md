# Figure regions — scope

**Status: built 2026-10-05 as `figure-regions-v1`, decision #46.** The owner, shown a prototype's
measurement, chose "Write scope, then build"; §6 gives the built rule's numbers beside the
prototype's.

## 1. Why

Of the 15,187 elements ParseBench's layout group scores, 2,202 are `Picture`. Ethos boxes a picture
only where the page paints a raster image with `Do` — an `Image` node, a placement of bytes — so a
figure **drawn** with paths has no element at all. At `2309010`, 1,334 of the 2,202 fail
localization: **819 have no element of ours overlapping them at all** — more than every other class
together, 236 — and 515 overlap one too little. Those 1,334 are 8.8% of every element scored.

## 2. The rule — `figure-regions-v1`

A **figure region** is a box this engine infers from the paths a page paints. Per page:

1. **Every painted path's box.** The box of each path a fill or stroke operator paints — lines,
   rectangles and curves alike, a curve's control points included — on the page and inside the
   forms it draws, which is where an included figure usually is. A path ended with `n` (a clip, or
   nothing) paints nothing and is not read. A path whose box covers 80% of the page or more is the
   page's background and is not read either.
2. **Clustered by touch**: two boxes are one cluster where the gap between them is at most 3 points
   across and at most 3 points down.
3. **Kept** where the cluster:
   - holds **3 paths or more** — a lone rectangle is a panel or a rule, not a figure;
   - covers **at least 0.1% and at most 80%** of the page — a speck, or a page background;
   - has **less than half its area inside any one table** a table rule found on the page;
   - holds **no line of prose**: no baseline inside it carries more than 60 characters other than
     whitespace (runs whose origins lie in the box, baselines within the same 1.5 points read as
     one) — a shaded panel behind text is a text box, and a figure's labels are short.
4. **Only on a document that declares no author structure**, as the whitespace and chart rules
   run: a tagged document says what is a figure (`/Figure`), and an inferred one would stand
   beside it.

**Never text.** A region claims no run, carries no caption and no alt text, and moves nothing:
the runs inside it stay in their units, in reading order. It is a box and the rule that drew it.

## 3. On the artifact

A tenth payload member, `figures`, beside `tables` and `outlines` and like them always written:
each record an `id`, its `page`, its box as `geometry` (`Measured`), `detection_rule:
"figure-regions-v1"` and `derivation: "computed"` — the per-record rule id is what says the box is
this engine's inference, as it is for a table. **An empty array means the rule found none, or did
not run on a document that declares author structure**, exactly as the heading, unit and tracks
rules do not; `capabilities.figures` says whether the profile runs it at all, and where it does not —
the office profiles, whose formats draw no paths this engine reads — `figures-not-detected` says so.

Every artifact moves, as at the outlines slice: `schema_version` 0.7.0 → 0.8.0, one `figures` key,
one capability, and on the office profiles one limitation. The profile gains `figure_detection` —
`figure-regions-v1` for PDF, `not-run-for-this-format` for the office profiles — so
`profile_sha256` moves.

**Not projected.** Markdown and HTML project no figure, as they project no image node today
(`non-text-nodes-not-projected`), and the grounding artifact reads `nodes`, so it is unchanged. The
ParseBench adapter labels each record `Picture`, as it labels each image node.

## 4. Why not an `Image` node

An `Image` node states *these bytes were drawn here*: extracted, with a digest of the stream. A
figure region states *this engine read these paths as one figure*: computed, with no bytes behind
it. Folding the second into the first would make a consumer unable to tell a placement the
document made from a grouping this engine made, which is the distinction `DerivationClass` exists
to keep.

## 5. Bounds

1. **Fabrication 0**, by construction: no text, no cell, no run.
2. **ParseBench**: visual grounding rises; no page falls by more than 0.05; content, formatting,
   tables and charts unchanged on every page (a region changes no text).
3. **opendataloader-bench**: NID, TEDS, MHS unchanged on every document (nothing is projected).
4. **The gate documents**: text, tables and limitations unchanged; their tree-stripped copies'
   region counts reported.

## 6. Measured on the prototype (2026-10-05)

Painted-path boxes clustered as §2 states — 3 points, 3 paths, 0.1% — emitted to the ParseBench
adapter as `Picture`, against `2309010`'s visual grounding of 0.4708. The prototype predates
decision #44, so one page that build refuses is out of both grounding runs; the built rule is
measured again on top of it.

| Longest line allowed inside | Pictures newly found | Other elements a region could take | Visual grounding |
| ---: | ---: | ---: | ---: |
| none (prose not checked) | 379 | 181 | — |
| 100 characters | 372 | 160 | — |
| **60 characters** | **345** | **127** | **0.4708 → 0.4824**, 73 pages up, 3 down by 0.013–0.036 |
| 30 characters | 296 | 71 | 0.4708 → 0.4810, 63 up, 2 down |

The gap (0–6 points) and the floor on paths (1 or 3) were swept too: a single path finds about
150 more pictures and more than doubles the elements put at risk.

**Built** (`figure-regions-v1`, on top of decision #44): visual grounding **0.4708 → 0.4824**, 73
pages up and 3 down — `sbkk_integrated_report_2020_section4_en` p6 by 0.036 and
`938c3dc8-b424-40fc-836b-101415d323cd` p18 and p19 by 0.021 and 0.013, inside §5's 0.05 — each a
running head drawn as a band of tabs, which the ground truth boxes as the page header and a region
now covers. Content, formatting, tables and charts are unchanged on every page
(`docs/measurements/parsebench/README.md`), and opendataloader-bench's 200 Markdown files are
byte-identical.

The eight gate documents declare author structure, so the rule runs on none of them and their text,
tables and limitations are as they were. Their tree-stripped copies, where it does run, carry 208
regions over 1,466 pages: `irs-f1040sd-2025` 0, `irs-fw9` 1, `nist-sp-800-161r1` 64,
`nist-sp-800-171r3` 1, `nist-sp-800-207` 13, `nist-sp-800-218` 10, `nist-sp-800-37r2` 9 and
`nist-sp-800-53Ar5` 110.

## 7. Not done

- **Icons in rows.** Ground truth boxes each small icon; a row of them touching one rule clusters
  as one region, which matches none.
- **A figure's text.** Ground truth sometimes attributes the labels inside a figure to it; this
  rule claims no text.
- **Captions** and the link from a figure to one.
- **A raster image and the paths drawn over it**, read as one figure.
