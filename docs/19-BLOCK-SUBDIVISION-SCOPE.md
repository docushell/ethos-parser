# 19 — Block subdivision: measured, and not yet scoped

**This document is a measurement, not a plan.** It follows [`17-D1-SCOPE.md`](17-D1-SCOPE.md) and
[`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md): a question was asked, an instrument was built, and
what it found is written down before anything is built on it. Unlike those two it does **not** end
in a refusal. It ends in three named probes, because the measurement that was run answered a
question adjacent to the one that matters.

**It is deliberately not called a paragraph cut.** See §6.

---

## 1. The question, and why it is on the critical path

[`02-ROADMAP.md`](02-ROADMAP.md) gives v2.2 two gate clauses by decision #19. The first — geometric
block structure — is met: D4 shipped S0–S4 at 0.42.0. The second is auto-tagging, *"a tag this
engine writes is one it can read back and ground against"*, and decision #19's argument for doing D4
first was that **you cannot write a tag for an untagged document without first deciding where its
blocks are.**

D4 did not decide that, and [`16-D4-SCOPE.md`](16-D4-SCOPE.md) §2 says so — *"the flat-run-of-
paragraphs problem survives D4 for a single-column untagged document."*
[`reading_order.rs`](../crates/ethos-parser-pdf/src/reading_order.rs) says why, at `Regions`:

> A region is what the **vertical** cut separated, and nothing finer … within a band,
> `horizontal_cut` cuts at the widest gap *and every gap tied with it*, and body text set with
> uniform leading has every baseline gap tied. Numbering the leaves would therefore put one region
> on **every line** of a multi-column page.

So D4 delivered **columns**. Auto-tagging needs **blocks**. The prerequisite decision #19 named is
still open, and this document measures whether it can be closed geometrically.

## 2. What was measured

Six tagged gate documents — `nist-sp-800-207`, `-218`, `-171r3` (60pp), `-37r2` (60pp), `irs-fw9`,
`irs-f1040sd-2025` — plus, for the deployment population, 37 of the 200 untagged DP-Bench documents.

Within each `(page, region)` band, consecutive baselines give a gap. Each gap is expressed as a
multiple of **that band's own modal gap** — its leading — because `font_size` is a constant `100` on
every run of these documents (they set `Tf /F 1` and carry the scale in the text matrix), so it is
not the type size and normalising by it divides by a constant. The band's own leading is also the
only scale a real rule could use, since the rule cannot see the point size either.

A line pair is labelled `ROLE_CHANGE` where the structure tree declares different **block-level**
roles either side, and `SAME_ROLE` otherwise.

## 3. Three errors this instrument had, all found before it was believed

Recorded because the pattern is the finding, and it is the one v2-S13.1 and v2-S17 already named:
**a guard that reads its own subject wrongly passes forever.**

1. **The artifact filter did nothing.** A `PdfArtifact` locator serialises as
   `{"pdf_artifact": {}}`, and `{}` is falsy — so the skip never fired, and NIST's vertically-set
   marginal DOI notice, whose every character has its own baseline, was being read as ~50 body lines
   per page. Key presence, not truthiness.
2. **Inline markup was counted as block structure.** `Link` and `Reference` are inline-level
   elements: `P → Link` is a paragraph containing a URL, not a boundary. **69 of the 149** declared
   boundaries that appeared to carry no vertical signal were `P → Link`. Resolving each line to its
   nearest block-level ancestor halved the apparent invisible fraction from 19% to 9.7%.
3. **The corroboration and the sweep counted different populations.** *"560 of them, 42.9% versus
   11.0%"* was computed over three documents while the sweep ran over six; at `T = 1.6` the six give
   **701** firings, the three give 560. The sentence read as though both numbers described one set.

A fourth, of the same family, was found by review and is **not** fixed here: `(page, region)`
interleaves table columns, so **21 of 232 bands (9%)** are not text flows. Five `nist-sp-800-218`
pages report a modal "leading" of 200 centipoints — 2 points — which is a four-column interleave
pitch. Every ratio in those bands is normalised against a fiction. It makes the published numbers
*pessimistic*, which is why it is recorded rather than quietly repaired.

## 4. What the measurement found

**The premise holds: a gap does separate.** Over 676 declared boundaries and 7,233 same-role pairs:

| gap (× the band's own leading) | declared boundary | same-role |
| --- | --- | --- |
| 0.9–1.1× | 7.8% | **67.2%** |
| 1.3–1.6× | 17.8% | 10.8% |
| 1.6–2.0× | **47.8%** | 5.0% |
| 2.0–2.6× | 18.3% | 1.9% |

**And three findings make the obvious conclusion unquotable.**

**4.1 — The labelled set contains none of the population the feature exists for.** A role *change*
cannot be `P → P`; the exclusion is a tautology, and the enumeration confirms it — **0 of 676** pairs
are `P → P`. 446 involve a heading, 255 a list. The flat run of paragraphs that D4 left open is
precisely what these labels cannot contain. *"~1.6× finds ~71% of declared block boundaries"* is a
**heading-and-list-detection** number and must never be quoted as this feature's recall.

**4.2 — There is no portable threshold.** Recall at `T = 1.6`, per document:

| document | recall | | document | recall |
| --- | --- | --- | --- | --- |
| `nist-sp-800-207` | 97.1% | | `nist-sp-800-218` | 53.6% |
| `nist-sp-800-171r3` | 92.0% | | `nist-sp-800-37r2` | 35.5% |
| `irs-f1040sd-2025` | 76.9% | | `irs-fw9` | **28.8%** |

A 68-point band. The pooled 71.2% is the mean of a rule that works and a rule that does not, and the
constant is fitted to the corpus mix. The mechanism is visible: **each document sets its break at a
near-constant pitch of its own** — `irs-fw9`'s break gap has p25, median and p75 all at 1.33×;
`nist-sp-800-207`'s are all at 1.87×. Within a document the signal is nearly a point mass and highly
discriminative. Across documents a fixed multiplier cleaves the corpus in half. **That is the
finding**, and it says the rule must be adaptive rather than tuned.

**4.3 — On the deployment population the dominant convention is one this instrument cannot see.**
Auto-tagging runs on untagged documents; all 200 DP-Bench documents are untagged (0 carry
`/StructTreeRoot`). Of 37 analysed, **16 (43%)** show a secondary left-edge cluster 11–23 pt right of
the body edge — a 1–2 em paragraph **indent**. US federal publishing sets block paragraphs with space
between; the scholarly typesetting that fills DP-Bench sets indented paragraphs with *no extra
leading*. A gap-only rule is blind to that convention by construction, and no gate document is
indent-marked prose, so the branch that would handle it can be calibrated on nothing this repository
owns.

## 5. What may be quoted, and what is discarded

**Quotable, each with its caveat.** The sweep and band construction reproduced bit-identically under
independent re-runs; `SAME_LINE_TOL`, the merge key, leading rounding (1–50 cp), the 5.0× cap, the
minimum-band size and the digit-line drop each move recall by ≤1 pp, so the result is not an artifact
of its constants. The band-wide mode is *required* — local windows lose 5–20 pp. Per-document recall
must be published as a band with the worst document named, per decision #18.

**Discarded.**

- *"~1.6× finds ~71%"* as this feature's recall — §4.1.
- *"T ≈ 1.6"* as a constant — §4.2; per-document optima span 1.05–1.70.
- *"a hard ceiling around 90%"* unqualified — it is 71.7%–89.6% depending on population, and the
  binding figure for auto-tagging is the untagged one.
- **The whole corroboration sentence.** Beyond error 3, the 4× enrichment was *guaranteed by
  construction*: the non-firing pool is ~67% mid-paragraph continuation lines, which cannot end a
  sentence. It measured its own denominator.

## 6. What is refused now, and needs no further measurement

**Naming a geometric block a paragraph.** Decision #19: *"A region is never a heading, a paragraph,
a section or a column — that is P14, and it stays refused."* Whatever this becomes is a **Computed
block subdivision** in D4's shape: an unnamed index, absent where the rule declined.

**Writing `/P`, or any named block, into a Tagged PDF.** This is decision #20's laundering failure in
a new place, and it closes the second half of v2.2 on the books rather than on evidence. `derivation`
is a field on this engine's `Node`; a PDF structure element has no counterpart for it. So a `/P` this
engine writes from a `Computed` cut is read back by
[`structure.rs`](../crates/ethos-parser-pdf/src/structure.rs) as a `PdfTagged` locator on an
`Extracted` run — and `DerivationClass::may_be_overwritten_by` returns `false` for every pair whose
left side is `Extracted`, so the laundered block is not merely mislabelled, it is **uncorrectable**.

**If v2.2's second half is ever opened, it needs an answer to how a written `/P` declares that it was
Computed once it is inside a PDF — and the format has no field for that.** That is a larger obstacle
than anything this document measured.

## 7. Three probes, before anything is scoped

1. ~~**The `StructElem` sibling probe.**~~ **RUN — see §9.** A walk-order counter per structure element in `structure.rs`,
   carried on the binding, would make two consecutive lines in different innermost elements a
   declared `P → P` boundary — the label §4.1 says does not exist today. **Gate question: does a
   producer emit one `/P` per paragraph, or one per line?** `element_id` is never populated and
   `mcid` is line-like — a median of **one** baseline per mcid on five of six gate documents — so
   this is the only route to a real label on documents the repository already owns. Either answer is
   worth having, and it is an afternoon.
2. **A `/Layout` attribute occurrence count over all 45 PDF fixtures.** `/SpaceBefore`, `/TextIndent`
   and `/StartIndent` are the document declaring its own paragraph spacing, which beats an inferred
   threshold on this repository's own grounds. Decisive either way, exactly as `/Collection` was for
   D1: either they exist and declared beats measured, or they are 0 and that branch is cleanly
   refused.
3. **Adaptive versus fixed, on probe 1's labels** — the band's own second mode against a constant,
   with the plausibility guard (modal gap ≥ 600 cp and mode share ≥ 25%, which excludes the 9% of
   bands that are not text flows) and the per-band indent-convention branch, reported per document
   with the worst document named.

**If the probes land, the rule shape they point at** is: a band-wide modal leading as the
normaliser; the band's own **second mode** as the cut, with no fixed multiplier and no cut where no
second mode exists; a per-band indent convention decided from the band's own left-edge distribution
*before* it is applied; and a short-last-line conjunct where `advance` is present, with a declared
branch where it is not.

## 8. Standing rules, carried forward

1. **A block is where, never what.** P14, and decision #19's line.
2. **No fixed multiplier presented as a measurement.** §4.2 is why.
3. **A number is published with the band and the worst document, never as a pooled macro** —
   decision #18, and [`table-gate-v1.md`](table-gate-v1.md)'s method applied to a second question.
4. **A band the rule cannot read says so** rather than being folded into an average.
5. **Nothing is written into a PDF that cannot declare how it was derived.**

---

## 9. Probe 1, run: the `StructElem` sibling probe

Run without touching the engine. `qpdf --qdf --object-streams=disable` exposes the structure tree,
each `/S /P` element names its page and its MCIDs, and the engine's own artifact maps `(page, mcid)`
to baselines — so element identity is recovered by joining the two rather than by putting it on the
wire to find out whether it is worth putting on the wire.
[`measurements/block-subdivision/structelem.py`](measurements/block-subdivision/structelem.py).

### 9.1 — The answer is *producer-dependent*, which neither branch of the gate question allowed for

The probe was framed as "one `/P` per paragraph, or one per line?" It is both, by document. The
operative statistic is what fraction of consecutive line pairs cross a `/P` boundary — near 100%
means per-line and the label is worthless; near `1/(lines per paragraph)` means it is real:

| document | line pairs crossing a `/P` | lines per `/P` | usable? |
| --- | --- | --- | --- |
| `nist-sp-800-207` | **20.3%** | **3.63** | **yes** |
| `irs-fw9` | 60.6% | 2.27 | no — a form; its "paragraphs" are field labels |
| `irs-f1040sd-2025` | 67.8% | 0.92 | no |
| `nist-sp-800-218` | **79.2%** | 1.32 | no — per-line |

**One gate document in four tags paragraphs as paragraphs.** A first pass asked instead how many
`/P` elements span one baseline (62.9%) and concluded per-line; that was the wrong statistic, since
a one-line `/P` is usually a genuinely one-line paragraph. The corrected question is above.

### 9.2 — What the real labels say, and it corrects §4 in both directions

On `nist-sp-800-207`: **134 real P→P boundaries** and 718 mid-paragraph pairs.

| T | recall of real P→P | fires mid-paragraph |
| --- | --- | --- |
| 1.15 | **64.9%** | 1.1% |
| 1.40 | 61.2% | **0.0%** |
| 1.60 | 59.7% | **0.0%** |
| 1.80 | 54.5% | 0.0% |

**Precision, which §5 recorded as unmeasurable, is measured and near-perfect.** No mid-paragraph
pair fires at `T ≥ 1.25`. When the rule fires it is right; the proxy-based worry about false
positives was an artifact of proxy labels.

**And the ceiling is far worse than the heading set implied. 35.1% of real P→P boundaries sit at
≤1.1× leading**, against 9.7% on the role-change set — the heading measurement was optimistic by
3.6×, because headings are given space by template and paragraph breaks frequently are not. The
honest ceiling on this document is **~65%**, not ~90%, and `nist-sp-800-207` is the document where
the gap rule did *best* on the heading set (97.1%). This is the optimistic case.

### 9.3 — Indent, measured on real labels for the first time

Of the 47 invisible boundaries, **12 (25.5%)** indent the following line — against 20.7% of the
visible ones, so on this document indent is only weakly complementary rather than the specific
antidote §4.3 hoped for. Combined reach of `gap > 1.15 OR indent`: **73.9%**.

That is consistent with §4.3 rather than against it: `nist-sp-800-207` is space-marked US federal
publishing, and the DP-Bench documents where 78% of gap-invisible breaks are indented are
indent-marked scholarly typesetting. **The convention is per-document, and a rule that does not
decide which convention a band uses before testing it will be wrong on whichever half it did not
pick.**

### 9.4 — What this changes

- The route **half-lives**. Real P→P labels exist, on one of four gate documents. Enough to correct
  the numbers above; not enough to scope on, and decision #18's discipline forbids quoting a
  one-document result as a corpus result.
- **§4's recall figures are superseded for the population that matters**: ~65% ceiling, ~60% at
  `T = 1.6`, ~0% false-fire — not 71.2%/9.7%.
- Probes 2 and 3 stand. Probe 3 should now run against `nist-sp-800-207`'s real labels rather than
  proxies, and the corpus question is sharper than it was: **the repository owns exactly one
  document that can label this feature**, and acquiring more is now the binding constraint.
