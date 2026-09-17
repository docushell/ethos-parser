# 19 — Block subdivision: measured, and not yet scoped

**This document is a measurement, not a plan.** It follows [`17-D1-SCOPE.md`](17-D1-SCOPE.md) and
[`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md): a question was asked, an instrument was built, and
what it found is written down before anything is built on it. Unlike those two it does **not** end
in a refusal. It ends in three named probes, because the measurement that was run answered a
question adjacent to the one that matters.

**It is deliberately not called a paragraph cut.** See §6.

**Since this was written.** The body stands as last amended on 2026-09-10 (§11.2b); the title's
"not yet scoped" is still true, and the cut shipped anyway. What followed is recorded here, dated
2026-09-16, with the version each item shipped in, and the body is not rewritten.

- **Shipped at 0.55.0, without a scope document.**
  [`crates/ethos-parser-pdf/src/blocks.rs`](../crates/ethos-parser-pdf/src/blocks.rs), commit
  `e077a7f`, puts `TextRunAttributes.block` on every PDF text run under `gutter-columns-v3`: the
  unnamed `Computed` index §6 permits and nothing more, 1-based in reading order, absent wherever the
  rule declined. The rule is §11's fixed threshold — a gap of at least 1.6 × the band's own modal
  leading, `5·gap ≥ 8·leading` in centipoints — under §5's guard (a modal bin under 600 centipoints,
  or holding under a quarter of the band's gaps, declines the band), with a block bounded by the
  vertical cut as well, and with no indent branch (§4.3). [`02-ROADMAP.md`](02-ROADMAP.md) requires
  a scope document before code and this cut did not get one; the roadmap now says so in as many
  words rather than backdating one. Decision #21 (2026-09-05) removed the consumer the cut was for
  and #23 (2026-09-07) gave it back.
- **The measurement was re-derived at 0.54.0**, by
  [`measurements/block-subdivision/probe3b.py`](measurements/block-subdivision/probe3b.py) — §11.2b:
  135 real P→P boundaries and 719 mid-paragraph pairs on `nist-sp-800-207`, fixed 1.60× at **63.7%
  recall and 100% precision**, where §11.4 had 63.0% over 715. That is the figure the shipped rule,
  the field's rustdoc and the limitation below all quote. The question §11.2b left live — 1.15×
  dominates on the one labellable document — is still live: no second labellable document has
  arrived.
- **Its limits are declared on the artifact**, after 0.58.0 on branch `feat/block-cut-leftovers`.
  Every PDF extract artifact carries the profile-scoped limitation
  `block-subdivision-leading-gap-only` (the constant in `ethos_parser_core::codes`, built in
  [`crates/ethos-parser-pdf/src/limitations.rs`](../crates/ethos-parser-pdf/src/limitations.rs)),
  which states that the index is computed from vertical whitespace against the band's modal leading
  and nothing else; that there is no indent branch, so a paragraph break marked by indentation with
  no extra leading opens no block; that recall was measured on one document, at 63.7% and 100%
  precision; and that a block is not a paragraph and no role may be read from it. Until then those
  four facts stood only in `blocks.rs` comments ([`OPEN-WORK.md`](OPEN-WORK.md) §2.2).
- **The two hops that carry `block` are pinned by tests**, on the same branch, against
  `leading-gap-two-blocks` — the first fixture authored for the cut, six lines at a stated leading
  with one stated gap ([`fixtures/README.md`](../fixtures/README.md)). `extract.rs` sets three runs
  in block 1 and three in block 2, `represent.rs` carries exactly those onto the wire, and
  `markdown-two-blocks` carries none on either hop. Three earlier engine fixtures already came out in
  two blocks by the gap half, each by accident of a layout authored for something else; none stated
  a leading or a gap.
- **The consumer is auto-tagging, and only auto-tagging.** `23-AUTO-TAGGING-SCOPE.md`, being written
  on another branch, is the scope document for it. Nothing reads `block` outside tests today
  (OPEN-WORK §2.2). §6's line — a block is never named a paragraph, and a tag written from one says it
  was computed — is where that document starts.

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

> **Settled 2026-09-05 as [decision #21](00-NORTH-STAR.md), and unsettled again on 2026-09-07 by
> [decision #23](00-NORTH-STAR.md).** What this section argued, the owner accepted: v2.2 closed at
> half with auto-tagging refused on the format. #23 answered row 21's reopening condition — an
> attribute object under `/A` naming a private owner, which `structure.rs` already parses — so the
> refusal is lifted and **row 23 is where the status is now stated**.
>
> **What this section argued is NOT reversed with it.** Naming a geometric block a paragraph stays
> refused: that is P14 and decision #19's line, and #23 leans on exactly that distinction — a
> written tag says a block was *computed* here, never that an author *declared* one. The subdivision
> below is still an unnamed `Computed` index.

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
2. ~~**A `/Layout` attribute occurrence count over all 45 PDF fixtures.**~~ **RUN — see §10.** `/SpaceBefore`, `/TextIndent`
   and `/StartIndent` are the document declaring its own paragraph spacing, which beats an inferred
   threshold on this repository's own grounds. Decisive either way, exactly as `/Collection` was for
   D1: either they exist and declared beats measured, or they are 0 and that branch is cleanly
   refused.
3. ~~**Adaptive versus fixed, on probe 1's labels**~~ **RUN — see §11.** — the band's own second mode against a constant,
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

---

## 10. Probe 2, run: the documents do not declare their own spacing

PDF 32000-1 §14.8.5 gives structure elements layout attributes under the `/Layout` owner —
`/SpaceBefore`, `/SpaceAfter`, `/StartIndent`, `/EndIndent`, `/TextIndent`. Those are the document
*stating* what §4 infers. Over **all 46 PDF fixtures in this repository** — 45 when
[`17-D1-SCOPE.md`](17-D1-SCOPE.md) counted, plus `ink-past-the-media-box` added at 0.42.1 —
after decompression:

| Signal | Occurrences |
| --- | --- |
| `/SpaceBefore` | **0** |
| `/SpaceAfter` | **0** |
| `/StartIndent` | **0** |
| `/EndIndent` | **0** |
| `/TextIndent` | **0** |
| `/ClassMap` | **0** |

**Two ways this probe could have produced a false zero, and neither did.** Structure elements live
in compressed object streams, so a grep over raw bytes finds nothing whatever a document contains —
D1's "raw object occurrences" method is safe for catalog keys and is *not* safe here, so every file
is expanded with `qpdf --qdf --object-streams=disable` first. And an element may name a `/C` class
whose attributes sit in a `/ClassMap`; a probe reading only `/A` reports zero on any producer using
classes. Both paths are counted, and the ClassMap path is empty because no fixture has one.

**The positive control is what makes the zero worth anything.** The probe found **540** `/O /Layout`
attribute dictionaries across six documents, so it demonstrably finds these when present. Every one
carries the identical shape — `/BBox [...] /O /Layout /Placement /Block` — and they sit on exactly
the roles where a bounding box is conventionally required:

| role carrying a Layout dictionary | count | share of that role |
| --- | --- | --- |
| `/Link` | 342 | 23.5% |
| `/Figure` | 123 | 74.5% |
| `/Table` | 72 | 61.5% |
| `/TextBox` | 2 | 2.0% |
| `/DropCap` | 1 | 100% |
| **`/P`** | **0** | **0% of 49,228** |

**So the finding is not "these documents declare no layout." It is that they declare layout for
figures, tables and links, and declare nothing at all for a paragraph.** Not one of 49,228 `/P`
elements carries a declared rectangle, an indent or a space.

### What this settles

**The declared route is refused on evidence, in D1's exact shape.** The honest signal exists in the
specification, and occurs zero times where it would matter — `/Collection`, `/EmbeddedFiles` and
`/Part` all over again. And as with D1 this **strengthens** the measured cut rather than weakening
it: the alternative was not dismissed on principle, it was tried and found absent, so an inferred
boundary is not a shortcut past a declaration that was there for the taking.

**Reopening condition.** A corpus whose producer emits `/Layout` spacing on `/P` elements. InDesign
and LaTeX-derived PDFs are the plausible sources and this repository owns none; the check is one
run of
[`measurements/block-subdivision/layout_attrs.py`](measurements/block-subdivision/layout_attrs.py)
against any new fixture, and it is worth running on every corpus this repository acquires, because a
document that declares its own spacing turns the whole of §4 into a fallback.

---

## 11. Probe 3, run: the corpus cannot answer the question it was built to answer

§4.2 said the fixed multiplier fails because each document sets its break at a pitch of its own, and
that the rule should therefore find each band's **second mode** and cut in the trough beneath it.
Probe 3 tests that. Per band: bin gaps to 10 centipoints, `leading` is the modal bin, candidate
second modes are bins above `1.15 × leading` holding `max(2, 5%)` of the gaps, and the cut sits
midway between. **No second mode, no cut** — the rule declines rather than guessing.

Probe 1 left one document with real labels, and "adaptive beats fixed" is a claim about variation
*across* documents, which one document cannot exhibit. So the probe splits.

### 11.1 — Test A: the mechanism, six documents, role-change labels

A relative comparison against identical labels, so §4.1's subset problem does not invalidate it.
Absolute values remain heading-and-list numbers.

| document | fixed 1.6× | adaptive |
| --- | --- | --- |
| `nist-sp-800-207` | 97.1% / 9.6% | 79.1% / 10.0% |
| `nist-sp-800-171r3` | 92.7% / 7.2% | 92.4% / 28.5% |
| `irs-f1040sd-2025` | 75.0% / 39.1% | 75.0% / 39.1% |
| `nist-sp-800-218` | 53.6% / 7.6% | 71.4% / 13.1% |
| `nist-sp-800-37r2` | 36.6% / 6.1% | 65.6% / 14.8% |
| `irs-fw9` | **33.3%** / 1.9% | **96.1%** / 27.9% |
| **per-document spread** | **33.3–97.1% (64 pts)** | **65.6–96.1% (30 pts)** |

**Adaptive is the variance fix it was predicted to be: the spread halves and the worst document
nearly triples.** It also fires 2.5× more often on non-boundaries pooled (7.2% → 18.0%), and on
role-change labels that pool contains the unlabelled P→P breaks, so whether the extra firing is
right or wrong cannot be read here.

### 11.2 — Test B: the magnitude, real P→P labels, `nist-sp-800-207` only

127 real boundaries, 715 mid-paragraph pairs, guarded bands:

| rule | recall | false-fire | precision |
| --- | --- | --- | --- |
| fixed 1.15× | 66.1% | 1.1% | 91.3% |
| **fixed 1.60×** | **63.0%** | **0.0%** | **100.0%** |
| adaptive | 46.5% | 1.1% | 88.1% |

**Adaptive is worse than fixed here, on both axes.** And fixed at 1.60× is *perfect* on precision:
across 715 mid-paragraph line pairs it never once fires.

### 11.3 — Why these do not contradict, and why that is the finding

They agree. Test A says adaptive helps the documents where fixed fails (`irs-fw9` 33.3 → 96.1,
`nist-sp-800-37r2` 36.6 → 65.6) and *hurts* the one where fixed already excels
(`nist-sp-800-207` 97.1 → 79.1). Test B measures `nist-sp-800-207` — because probe 1 proved it is
the only document that can be labelled — which is precisely the document adaptive hurts.

**The one document that can validate the rule is the one document where the rule under test is least
needed.** The corpus cannot decide this, and no further analysis of these six files will change
that: it is not a question of method, it is an absence of evidence.

### 11.2b — Test B, reconstructed 2026-09-10, because its instrument was never committed

**§11.2's numbers were not reproducible.** `probe3.py` implements Test A only and `structelem.py`
stops at its gate question, so the table above — the one §11.4 chooses fixed over adaptive on, and
the one the implementation plan carries — had no instrument behind it. That is the failure this
document's own header exists to prevent. [`measurements/block-subdivision/probe3b.py`](measurements/block-subdivision/probe3b.py)
now produces it, and it is re-derived here on a tree eight releases newer.

**135 real P→P boundaries and 719 mid-paragraph pairs** (§11.2 had 127 and 715; the small
difference is the inline-child exclusion this reconstruction states explicitly, plus eight releases
of run-set change). No band failed the plausibility guard on this document.

| rule | §11.2 recall / false-fire / precision | reconstructed |
| --- | --- | --- |
| fixed 1.15× | 66.1% / 1.1% / 91.3% | **68.9% / 0.0% / 100.0%** |
| fixed 1.40× | 61.2% / 0.0% / — | 65.9% / 0.0% / 100.0% |
| **fixed 1.60×** | **63.0% / 0.0% / 100.0%** | **63.7% / 0.0% / 100.0%** |
| fixed 1.80× | 54.5% / 0.0% / — | 58.5% / 0.0% / 100.0% |
| adaptive | 46.5% / 1.1% / 88.1% | 51.1% / 0.0% / 100.0% |

**Two things hold and one has changed.**

**Holds — fixed beats adaptive on real labels**, 63.7% against 51.1%, and §11.3's reading of why
Test A and Test B disagree is untouched.

**Holds — precision.** Not one mid-paragraph pair fires at any threshold tested, over 719 chances.
§11.4's *"a rule that never fires wrongly across 715 chances is worth more to a repository that
refuses fabrication"* is if anything stronger now.

**Changed — the reason for preferring 1.60× over 1.15× has evaporated.** §11.2 chose the higher
threshold because 1.15× cost a 1.1% false-fire. On this tree 1.15× fires wrongly **zero** times and
returns **5.2 points more recall**. On this document it dominates.

**And that is exactly the claim §11.4 forbids acting on.** One document cannot carry a threshold —
Test A puts fixed-1.6×'s per-document recall between 32.3% and 97.1%, and a constant fitted to
`nist-sp-800-207` is a constant fitted to the one document that can be labelled. **1.60× stands as
the shipped rule.** What has changed is that its margin over 1.15× is now zero on the evidence
available, so the second labellable document §11.4 asks for would decide a live question rather
than confirm a settled one.

**Test A also drifted slightly** and is restated here: `nist-sp-800-171r3` 92.7% → 91.9% fixed,
`nist-sp-800-37r2` 36.6% → 32.3% fixed and 65.6% → 62.6% adaptive. Pooled fixed 71.2% → 70.5%.
Every other document is unchanged to the decimal. The spread is 65 points where it was 64.

### 11.4 — What this settles, and what it does not

- **Fixed `1.6 × leading` is measured, precise and narrow.** 63.0% recall at **100% precision** on
  real labels. A rule that never fires wrongly across 715 chances is worth more to a repository that
  refuses fabrication than a rule with higher recall and a false-fire rate.
- **Adaptive is unproven where it matters and disproven where it does not.** It must not be scoped
  on test A alone — that would be fitting a rule to proxy labels on documents whose real labels are
  unavailable, which is §4.1's error committed deliberately.
- **The binding constraint is now exact — and decision #21 removed the consumer it was binding
  for, then decision #23 gave it back.** #21 refused auto-tagging, so for two days no tag depended
  on this rule and nothing was blocked by the document below being absent. #23 (2026-09-07)
  reversed that, so the constraint binds again. It stayed recorded through the refusal because a
  block subdivision emitted as `Computed` layout would be worth its own bytes on D4's argument
  even with no tag consumer at all, and because a measurement that named its
  own missing evidence should say what would supply it. What is needed is one labellable document — 3+ lines per
  `/P`, per §9.1 — **whose break pitch differs from `nist-sp-800-207`'s 1.87×**. Probe 1 supplies
  the test for the first half and §4.2's per-document pitch table the second. Until such a document
  exists, `1.6×` stands as the measured rule and adaptive stays a hypothesis with one supporting
  and one contradicting measurement.
