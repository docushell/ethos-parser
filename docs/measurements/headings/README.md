# C1 S3 — the heading rule's false positives, where the author's tree can say

**Run 2026-09-18** with the release build of `b06b2fc`+`bf44152` (`type-size-v1`, S1 and S2
landed, not pushed), on macOS 26.6.2, Mac16,8 arm64. Instrument: [`falsepos.py`](falsepos.py).
Raw readings: [`falsepos.json`](falsepos.json). 249 s wall, 12.5 GB peak RSS — the instrument
parses two gigabyte-scale artifacts whole, which is what the memory is.

```
cargo build --release
python3 docs/measurements/headings/falsepos.py /tmp/headings-s3
```

**The short version: §7.5's bound is met by its letter and not by its purpose.** The band over the
nine bounded documents is **0.00%..4.61%, worst `cfpb-home-loan-toolkit`**, under the 5% bound. And
across the eleven documents the rule fires on 3,266 lines and is right on 165: two documents carry
**2,979 false headings against 68 declared ones**. The rule does not ship on this measurement until
the owner decides §7.5's reserved question, and the cause is measured below.

---

## 1. Method

`docs/28-HEADINGS-SCOPE.md` §7.3's, applied to the **shipped rule** rather than a proxy. The rule's
inputs do not depend on the structure tree, but its gate keeps the verdict off a tagged document's
wire, so each of §7.1.1's eleven documents is measured twice from one file: the untouched original,
whose `pdf_tagged` locators carry the author's labels, and a copy with `/StructTreeRoot` removed from
its catalog by `qpdf` (checked by `qpdf --check`), on which the gate opens. The two extracts are
joined node by node after the instrument checks they hold the same runs in the same order — and
`crates/ethos-parser-pdf/tests/headings.rs`'s
`a_gate_document_stripped_of_its_tree_is_where_the_rule_fires` holds that join valid in the test
suite, stripping `irs-fw9` through `lopdf`.

A **line** is the rule's own unit (page, band, `/Artifact` state, baseline). Whitespace-only and
`/Artifact` lines are excluded. A line is **labelled** when a run of it carries a declared
block-level role — the innermost role of its locator walking up past inline-level elements,
`paragraphs.py`'s `block_role`. A **false positive** is a line the rule fired on whose declared role
is not `H`/`H1`..`H6`; the **FP rate** is false positives over labelled lines.

## 2. What was measured

| document | heading items | labelled lines | declared heading lines | fired | true | **false** | FP rate | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 *(counts only)* | 8 | 120 | 9 | 3 | 1 | **2** | 1.67% | 11.1% |
| irs-fw9 | 28 | 697 | 28 | 26 | 26 | **0** | 0.00% | 92.9% |
| nist-sp-800-218 *(counts only)* | 7 | 1,848 | 7 | 540 | 7 | **533** | 28.84% | 100.0% |
| nist-sp-800-207 | 56 | 1,882 | 57 | 5 | 0 | **5** | 0.27% | 0.0% |
| nist-sp-800-171r3 | 193 | 4,075 | 180 | 10 | 6 | **4** | 0.10% | 3.3% |
| nist-sp-800-37r2 | 1,341 | 7,924 | 957 | 38 | 16 | **22** | 0.28% | 1.7% |
| nist-sp-800-161r1 | 451 | 12,780 | 454 | 13 | 0 | **13** | 0.10% | 0.0% |
| **nist-sp-800-53Ar5** | 52 | 80,090 | 61 | 2,506 | 58 | **2,446** | 3.05% | 95.1% |
| nist-sp-800-53r5 | 699 | 21,513 | 389 | 41 | 10 | **31** | 0.14% | 2.6% |
| **cfpb-home-loan-toolkit** | 96 | 911 | 83 | 61 | 19 | **42** | **4.61%** | 22.9% |
| irs-form-1040-2025 | 15 | 233 | 24 | 23 | 22 | **1** | 0.43% | 91.7% |

**By §7.5's letter: met.** The nine bounded documents sit at **0.00%..4.61%, worst
`cfpb-home-loan-toolkit`**. The two counts-only documents, reported beside their rates as §7.5
requires: `irs-f1040sd-2025` **2** false positives of 120 labelled lines; `nist-sp-800-218` **533** of
1,848.

**On the two forms the rule is what it was meant to be**: `irs-fw9` finds 26 of its 28 declared
headings with no false positive, and `irs-form-1040-2025` 22 of 24 with one.

## 3. What the letter hides, measured

**Two documents carry 2,979 false headings against 68 declared ones.** The mechanism is the same on
both, and it is the reference, not the cut:

| document | body em the rule measured | the population that set it | what the rule then flagged |
| --- | --- | --- | --- |
| nist-sp-800-218 | **900** centipoints | the document's small type, which dominates its characters | `TT1`, **28.4%** of all characters, flagged on **94.4%** of its own — and ordinary `TT0` prose ("This publication has been developed by NIST…") beside it |
| nist-sp-800-53Ar5 | **850** centipoints | `F66`, **59.5%** of all characters — the assessment-procedure text | `TT0`, the prose font, **10.3%** of characters, flagged on **69.5%** of its own |

The char-weighted mode is the size the most *characters* are set in, and on these two documents that
is dense small type, not the prose. Ordinary prose then sits above six fifths of the "body" and every
line of it is a heading. **Excluding table runs from the reference would not have helped**: the
detector finds **0** geometric tables on `nist-sp-800-53Ar5`.

**Why the scope did not see this.** §7.3's measurement of the size clause was a *per-font*
normalisation — each run's box height over its own font's modal — which by construction cannot see
two fonts set at different sizes. The shipped rule normalises by the *document*. The proxy's 4.84% on
`nist-sp-800-218` was therefore not a prediction of the shipped rule on that document, and the
measurement above is the first of the shipped rule anywhere.

**Why `nist-sp-800-218`'s counts-only status does not hold on this definition.** §7.5 exempts it
because "one false positive moves `-218`'s by tens of points". That is true of *precision* over a
document with seven declared headings; it is not true of the FP rate §7.3 defines, whose denominator
is labelled lines. One false positive moves `-218`'s rate by **0.054 points**, and 28.84% is a stable
measurement of 533 body lines called headings.

**Why the bound does not bound fabrication on a long document.** `nist-sp-800-53Ar5` passes at 3.05%
with **2,446** false headings against **61** declared, because its denominator is 80,090 lines. §7.5's
own reason for refusing 10% — *"a rule that can fabricate more headings than the document declares is
not evidence"* — is violated here at 3.05%, forty times over. A rate over every line is the wrong
unit for that sentence on a document this long.

## 4. What this decides, and what it leaves to the owner

**It decides nothing about shipping by itself**, because §7.5 reserves exactly this: *"a breach on one
of them is an owner decision rather than an automatic refusal."* The rule is committed locally — S1
`b06b2fc`, S2 `bf44152` — and not pushed, so nothing has shipped.

**Candidate repairs, none measured**, each a new rule id with its own run of this instrument:

1. **The body is the largest common size, not the most common one.** Take the reference as the
   largest em bin carrying at least some share of the body characters (10%, say), so a dense
   small-type population cannot make the prose look like display type. On both documents above the
   prose font carries 10%–28% of the characters and would become the reference.
2. **A line in a common size is not a heading**: refuse a line whose em bin carries more than a small
   share of the body characters, since headings are rare by nature and a size holding 28% of a
   document's text is a body size.
3. **Bound the count, not only the rate**: fired lines against declared heading lines per document,
   which is what §7.5's "more headings than the document declares" means.

The first two change the rule; the third changes the bound, and both kinds are the owner's to take.

---

## 5. `type-size-v2`, re-measured — the repair the owner chose

**The owner chose "repair, then re-measure" on 2026-09-18.** Two changes, both measured here with the
same instrument over the same eleven documents — 262 s, 10.9 GB peak RSS — with the raw readings in
[`falsepos-v2.json`](falsepos-v2.json) (`falsepos.json` stays `-v1`'s, as measured):

- **The rule's reference is the larger of the most common size and the largest *common* size** —
  common meaning at least a twentieth of the body characters, on at least ten lines. Dense small type
  is common; it is not the body. The line floor is what keeps a short page's title from becoming the
  body (one 24pt line over five short ones is 9.5% of that page's characters). Taking the larger of
  the two means `-v2`'s cut is never below `-v1`'s, so **`-v2` can only remove headings `-v1` found,
  never add one**. Both constants sit inside the gaps the evidence leaves — every size above the body
  holds at most 4.2% of the characters on at most eight lines, and the smallest prose size that must
  count as body holds 10.4% on 2,418 — at the end that errs toward refusing.
- **A count bound beside the rate**: on every one of the eleven, no more false headings than the author
  declared — §7.5's own reason for refusing 10%, made checkable.

| document | declared | fired v1 → **v2** | right v1 → **v2** | false v1 → **v2** | FP rate **v2** |
| --- | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 *(counts only)* | 9 | 3 → **3** | 1 → **1** | 2 → **2** | 1.67% |
| irs-fw9 | 28 | 26 → **26** | 26 → **26** | 0 → **0** | 0.00% |
| nist-sp-800-218 *(counts only)* | 7 | 540 → **10** | 7 → **0** | 533 → **10** | 0.54% |
| nist-sp-800-207 | 57 | 5 → **5** | 0 → **0** | 5 → **5** | 0.27% |
| nist-sp-800-171r3 | 180 | 10 → **10** | 6 → **6** | 4 → **4** | 0.10% |
| nist-sp-800-37r2 | 957 | 38 → **38** | 16 → **16** | 22 → **22** | 0.28% |
| nist-sp-800-161r1 | 454 | 13 → **13** | 0 → **0** | 13 → **13** | 0.10% |
| nist-sp-800-53Ar5 | 61 | 2,506 → **43** | 58 → **9** | 2,446 → **34** | 0.04% |
| nist-sp-800-53r5 | 389 | 41 → **21** | 10 → **7** | 31 → **14** | 0.07% |
| cfpb-home-loan-toolkit | 83 | 61 → **61** | 19 → **19** | 42 → **42** | **4.61%** |
| irs-form-1040-2025 | 24 | 23 → **23** | 22 → **22** | 1 → **1** | 0.43% |
| **all eleven** | | **3,266 → 253** | **165 → 106** | **3,097 → 147** | |

**False headings fall 95%, and precision against the authors' tags rises from 5% to 42%.** The
reference moved on exactly the three documents the tally said it would — `nist-sp-800-218` 9 → 12pt,
`-53Ar5` 8.5 → 11pt, `-53r5` 10 → 11pt — and every other document reads exactly as it did.

**The rate bound: met**, 0.00%..4.61% over the nine, worst `cfpb-home-loan-toolkit`.

**The count bound: met on ten, breached on one — `nist-sp-800-218`, 10 false headings against 7
declared — and all ten are the document's own title.** "NIST Special Publication 800-218 / Secure
Software Development Framework (SSDF) Version 1.1: / Recommendations for Mitigating / the Risk of
Software Vulnerabilities", five lines on the cover and the same five on the title page, each tagged
`/P` by the producer. §7.5's "Why not 0%" paragraph anticipated exactly this: *"such a producer
labels a real display line `/P`"*.

**What `-v2` cost.** Recall on the three documents whose reference moved: `-218` 7 → 0, `-53Ar5`
58 → 9, `-53r5` 10 → 7. `-218`'s headings are 14pt over 12pt prose — 1.17×, under the 1.20× cut —
and `-v1` cleared them only because it measured the body at 9pt, which is also why it called 533 body
lines headings. `-v1`'s recall on those documents was a by-product of calling every prose line one.

**What the 147 are, read line by line on the four documents that carry most of them:**

- **Real headings the producer did not tag as headings** — the bulk. Titles on cover and title pages,
  "Executive Summary", "Acknowledgements", "Table of Contents", "Errata", "Patent Disclosure Notice",
  "4.1 ACCESS CONTROL" (all `/P` or `/TOCI`), and `cfpb`'s numbered step titles ("1. Define what
  affordable means to you", tagged as list items). The rule is right and the label is the producer's.
- **Decorative large glyphs** — rule error. Private-use icon characters and a lone `$` on `cfpb`,
  single letters on their own line on `nist-sp-800-37r2` (drop caps, figure letters). A line with no
  letters is not a heading; refusing such lines is a further change, **not built**, and would need
  its own measurement.
- **One large-type lead paragraph** — rule error. Six lines of `cfpb`'s page 5 set as a lead-in in
  display type and tagged `/P`.

**What this leaves to the owner:** whether `nist-sp-800-218`'s breach — its title, on two pages —
is accepted, since every other bound holds. S1, S2, S3 and this repair are local and unpushed.

**Amended 2026-09-18: accepted by the owner.** The ten lines are the document's title in display
type, which a reader calls a heading and the producer tagged `/P`; no clause removes them without
also removing real headings.

---

## 6. C1 S4 — what the repaired rule scores on the bench

`type-size-v2` over opendataloader-bench's 200 documents, through the bench's own evaluators —
the appendix of [`../opendataloader-bench/README.md`](../opendataloader-bench/README.md) has the
method and the NID side, and [`bench-mhs-nid.json`](bench-mhs-nid.json) the per-document readings
of both builds.

**MHS 0.0000 → 0.3321** over the 107 documents whose ground truth holds a heading; **band
0.0000..0.9986, median 0.1490, worst `01030000000001` (first by name of the 50 still at 0.0000),
best `01030000000179`**; 57 rose. None fell, but none could: every one stood at 0.0000, which is why
`28-HEADINGS-SCOPE.md` §7.5 calls bar 2's second half vacuous at this first measurement and binding
at every one after. NID moved −0.0003 because a `# ` is text to its evaluator, not because any run
moved: all 67 changed Markdown files are identical to their predecessors once the markers and
whitespace are removed.

**What the bench does not measure**, stated so the band is not read as more than it is. **Level is
invisible to it twice over**: the evaluator (`src/evaluator_heading_level.py`) flattens every
heading to one tag — its docstring: a section tree that "treats all heading levels as equivalent" —
and the ground truth's 193 headings, over the 107 documents that have any, are all level one
(`28-HEADINGS-SCOPE.md` §3.5, which is why the rule has one level). So MHS scores *which* lines are
headings and the text between them, and cannot tell this rule's one level from a right hierarchy or
a wrong one. **And the bench carries no author tags**, so it scores what the rule finds and cannot
see what it fabricates — which is why S3's false-positive measurement on the eleven tagged documents
is the bound the rule ships on, and this is its recall side.

---

## 7. S5 — the font-weight clause, built and refused on its own measurement (2026-09-20)

`28-HEADINGS-SCOPE.md`'s S5 held the font clause as conditional on "its own measurement on the
shipped signal, with its own bound set before its code". This is that measurement. **It was built,
measured on both sides, and is not shipped**: the numbers are here, the code is not in the tree.

**The signal, narrower than §7.3's proxy.** §7.3 measured "a font the body text is not set in",
which is why its band reached 15.76%. What was built instead reads the font's own declaration and
nothing else: `/FontDescriptor /Flags` **ForceBold** (bit 19), or a `/BaseFont` name containing
`Bold`. No `/StemV` and no `/FontWeight` — both are numbers that would need a threshold nobody
measured. A line is bold when every run of it carrying text is, and the clause fires only where the
document's body is **not** itself bold, measured at the body's own size by characters, so a deck or
a form whose prose is bold withdraws the clause rather than reading every line as a heading.

**What it bought**, over opendataloader-bench's 200 documents, the harness's own evaluator: MHS
**0.3353 → 0.5198**, and the documents scoring zero **49 → 15**. NID did not move (0.8714 → 0.8711).
It is a real signal: it is exactly the signal those documents use, and they set their headings at
body size in bold.

**What it cost**, over the eleven documents whose authors declare headings, tree stripped, by
[`falsepos.py`](falsepos.py) unchanged — per-document readings in
[`falsepos-weight-refused.json`](falsepos-weight-refused.json):

| document | declared | fired | true | false | FP rate | recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `irs-fw9` | 28 | 26 | 26 | 0 | 0.00% | 92.86% |
| `irs-form-1040-2025` | 24 | 23 | 22 | 1 | 0.43% | 91.67% |
| `nist-sp-800-53Ar5` | 61 | 78 | 26 | 52 | 0.06% | 42.62% |
| `nist-sp-800-53r5` | 389 | 39 | 10 | 29 | 0.13% | 2.57% |
| `nist-sp-800-207` | 57 | 27 | 9 | 18 | 0.96% | 15.79% |
| `nist-sp-800-37r2` | 957 | 353 | 55 | 298 | 3.76% | 5.75% |
| `nist-sp-800-171r3` | 180 | 372 | 169 | **203** | 4.98% | 93.89% |
| `cfpb-home-loan-toolkit` | 83 | 75 | 23 | 52 | **5.71%** | 27.71% |
| `nist-sp-800-161r1` | 454 | 1416 | 118 | **1297** | **10.15%** | 25.99% |
| `irs-f1040sd-2025` (counts only) | 9 | 3 | 1 | 2 | — | 11.11% |
| `nist-sp-800-218` (counts only) | 7 | 35 | 7 | **28** | — | 100.00% |

**Both bounds fail, and not narrowly.** The rate band over the nine bounded documents is
**0.00%..10.15%**, worst `nist-sp-800-161r1`, with `cfpb-home-loan-toolkit` also over at 5.71%
against §7.5's 5%. The count bound — no more false headings than the author declared — is breached
on three: `nist-sp-800-161r1` 1297 against 454, `nist-sp-800-171r3` 203 against 180, and
`nist-sp-800-218` 28 against 7, which is the document whose earlier breach of 10 the owner accepted.
The instrument's own verdict line reads `ALL BOUNDS: NOT MET`.

**Why it fails, which is the part worth keeping.** The clause is not weak — where weight means
*heading*, it is the best signal this rule has had: `irs-fw9` 92.86% recall at 0.00%,
`nist-sp-800-171r3` 93.89% at 4.98%. It collapses where a document uses weight for something else,
and the long NIST standards do exactly that: bold defined terms, bold table headers, bold inline
emphasis. **A signal that is excellent on four documents and unbounded on two is not a bounded
rule**, and §7.5's bound exists to say so before the code ships rather than after.

**What would reopen it.** A guard that withdraws the clause where weight is *common* in the
document, the way `body_is_bold` withdraws it where the body is bold — a share rather than a
majority. That is a new threshold, and it would need what `BODY_SHARE_DEN`'s 1/20 got: a measured
gap in the evidence to sit in, plus a re-run of both instruments. Nothing here sets one.

### 7.1 And the guard that would have saved it does not exist — measured 2026-09-20

§7 named what would reopen the clause: a guard that withdraws it where weight is *common*, the way
`body_is_bold` withdraws it where the body is bold. A share needs a gap in the evidence to sit in,
the way `BODY_SHARE_DEN`'s 1/20 was set from one. **There is no such gap.** Measured on the same
eleven documents, tree stripped, with the same build that produced §7's table — per-document
readings in [`bold-share.json`](bold-share.json):

| document | bold at body size | bold, all sizes | all-bold candidate lines | bounds |
| --- | ---: | ---: | ---: | --- |
| `irs-fw9` | 0.0% | 0.0% | 0.0% | ok |
| `irs-form-1040-2025` | 0.0% | 0.0% | 0.0% | ok |
| `irs-f1040sd-2025` | 0.0% | 0.0% | 0.0% | ok |
| `nist-sp-800-37r2` | 3.3% | 4.2% | 6.9% | ok |
| `nist-sp-800-207` | 3.6% | 5.7% | 6.3% | ok |
| `nist-sp-800-53Ar5` | 4.8% | **26.2%** | 7.8% | ok |
| `nist-sp-800-53r5` | **10.0%** | 14.8% | **17.5%** | ok |
| `cfpb-home-loan-toolkit` | **1.3%** | 7.4% | 11.5% | **breach** |
| `nist-sp-800-171r3` | 4.0% | 5.7% | 15.5% | **breach** |
| `nist-sp-800-218` | 4.1% | 11.6% | 5.4% | **breach** |
| `nist-sp-800-161r1` | 9.7% | 8.8% | 15.4% | **breach** |

**The two sets interleave on every metric.** `nist-sp-800-53r5` stays inside both bounds with more
bold than any breaching document — 10.0% at body size, 17.5% of its candidate lines — and
`cfpb-home-loan-toolkit` breaches with the least, 1.3%. `nist-sp-800-53Ar5` carries 26.2% bold
overall and is the second-best document in §7's table. No threshold on any of these three columns
separates the documents where the clause is bounded from the documents where it is not, so a
share-based guard is not a rule that could be set here; it would be a number chosen to fit four
documents and refuted by the fifth.

**Why the share cannot work, read off the same table.** How much bold a document contains is not
what decides whether bold means *heading* in it. `53Ar5` bolds a quarter of its text and the clause
stays bounded because those runs are not lines of their own; `cfpb` bolds almost nothing and
breaches because it declares only 83 headings, so a handful of bold captions exceeds the count
bound. The quantity is not the signal, and nothing measured here is.

**So S5 stays unbuilt**, now on two measurements rather than one: the clause itself is outside the
bound (§7), and the guard that would bound it has no gap to stand on (this section). Reopening it
needs evidence this repository does not have — a signal that separates a bold heading from bold
prose *within* a document, rather than a property of the document as a whole.

## 8. PI-C — the cross-page recurrence gate, refused on its own measurement (2026-09-22)

[`06-STEAL-REFUSE.md`](../../06-STEAL-REFUSE.md) row **PI-C** proposed a seventh clause, taken from
PageIndex's header detection: *a line whose normalized text appears on N or more distinct pages is
not an inferred heading*. Its motive is a real gap — `type-size-v2`'s only page-furniture exclusion
is `/Artifact` marked content, a **tagged**-PDF convention, and the rule fires only where a document
declares no structure, so the guard does not fire on the population the rule runs on. A running head
set at 1.20× body em fires on every page.

The clause is **subtractive**: it can only withhold a heading the rule already emitted, so it cannot
fabricate one, and §7.5's bar 1 is safe by construction. **Bar 2 is the whole question** — *"MHS
rises, and falls on no document"*, binding since §6's first measurement — and "falls on no document"
means the clause must withhold **zero** author-declared headings anywhere.

Measured with [`recurrence.py`](recurrence.py), which reuses [`falsepos.py`](falsepos.py)'s machinery
unchanged: each of the eleven §7.1.1 documents read twice from one file, the original carrying the
author's labels and a `/StructTreeRoot`-stripped copy carrying the shipped rule's verdict, joined
node by node under the same three guards. Data: [`recurrence.json`](recurrence.json).

### 8.1 The sweep

`w / tT` is *w lines withheld, of which t were author-declared headings*.

| N | withheld | **declared headings withheld** | documents breaching bar 2 | digits folded |
| ---: | ---: | ---: | ---: | --- |
| 2 | 71 | **14** | 3 | 78 / **19** declared |
| 3 | 12 | **4** | 1 | 12 / 4 |
| 4 | 12 | **4** | 1 | 12 / 4 |
| 5 | 5 | **4** | 1 | 5 / 4 |
| 10 | 1 | **0** | **0** | 1 / 0 |

**Only N=10 clears bar 2, and there the clause withholds one line across all eleven documents.** It
is safe exactly where it does nothing, and does work exactly where it destroys headings. A rule id
move costs `profile_sha256` and makes every artifact before and after non-comparable; one withheld
line does not buy that.

**Folding digits is worse, not better.** It takes N=2's breach from 14 declared headings to 19,
because numbered siblings collapse to one key and a whole run of real section headings goes with
them. The crude normalization is the better of the two, and both fail.

### 8.2 Why it fails, which is more useful than the number

At N=2 the population splits cleanly, and not by page count:

| | documents | shape |
| --- | ---: | --- |
| **pure gain** — withholds only false positives | **6** | `nist-sp-800-218` removes all 10 of its fired lines, every one a false positive; `nist-sp-800-161r1` 12 of 13; `nist-sp-800-53r5` 6, all false; also `-207`, `-53Ar5`, `cfpb-home-loan-toolkit` |
| **breach** — withholds real headings | **3** | `nist-sp-800-37r2` removes 7 declared of 22; `nist-sp-800-171r3` 6 of 10; `irs-fw9` 1 — and that document's rule is currently perfect, 26 fired against 26 declared |
| no effect | 2 | `irs-f1040sd-2025`, `irs-form-1040-2025` |

**The clause is strongly right where a document's false headings are running heads, and destructive
where its recurrent lines are genuinely repeated section headings.** What separates those two
populations is **position** — a running head sits in a margin band and a repeated section heading
does not — and position is the one signal decision #29's rider forbids this rule to read, enforced
by `headings.rs`'s `the_rule_reads_no_region_and_no_block`.

**That is the finding.** PageIndex's own detector agrees: `header_footer.py:296-298` gates on a
top-20%/bottom-20% band **first** and uses recurrence only to *confirm* candidates position already
selected. Its design is right for it. The half this engine is permitted to take is precisely the
half that does not work alone.

### 8.3 What would reopen it

Not a different N — the sweep covers the useful range and the failure is structural, not a tuning
miss. It reopens on **a signal that separates a running head from a repeated heading without reading
position**, which this repository does not have and this measurement does not suggest. The nearest
candidate — the document's own `/Artifact` marks — is the guard that already exists and is exactly
what these untagged documents do not carry.

The gap PI-C aimed at is therefore still open and still real, and §7.5's bars are why this is not
shipped rather than an argument that the gap does not matter.

## 9. `type-size-v3` — levels, and the bold clause with the signal §7.1 lacked (2026-10-03)

**Decision #38.** The owner chose "headings from more signals: boldness, isolation and numbering as
well as size". `type-size-v3` keeps `-v2`'s size clause unchanged, so every line `-v2` read as a
heading it reads as one, and adds three things:

- **Levels.** The sizes the headings are set in rank them, largest first; within one size, the depth
  of the heading's own section number (`2` above `2.1` above `2.1.1`). Levels 1 to 6 travel as
  `inferred_heading_level`, absent at level 1.
- **The bold clause, with isolation.** A line every run of which is bold, at the body em or larger,
  of 2 to 80 non-whitespace characters at least half of them letters, neither starting lower-case
  nor ending with a full stop, in a document whose body is not bold, is a heading **where the
  leading-gap cut put it in a block of its own**. That is §7.1's missing signal: bold prose runs on
  in its paragraph, and a bold heading stands apart. Where the cut declined on a page, nothing there
  stands apart.
- A bold heading is the level below the smallest heading size.

Same instrument, unchanged, over the same eleven documents; raw readings in
[`falsepos-v3.json`](falsepos-v3.json).

| document | declared | fired v2 → **v3** | right v2 → **v3** | false v2 → **v3** | FP rate **v3** | recall v2 → **v3** |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 *(counts only)* | 9 | 3 → **3** | 1 → **1** | 2 → **2** | 1.67% | 11.11% → **11.11%** |
| irs-fw9 | 28 | 26 → **26** | 26 → **26** | 0 → **0** | 0.00% | 92.86% → **92.86%** |
| nist-sp-800-218 *(counts only)* | 7 | 10 → **10** | 0 → **0** | 10 → **10** | 0.54% | 0.00% → **0.00%** |
| nist-sp-800-207 | 57 | 5 → **18** | 0 → **6** | 5 → **12** | 0.64% | 0.00% → **10.53%** |
| nist-sp-800-171r3 | 180 | 10 → **169** | 6 → **164** | 4 → **5** | 0.12% | 3.33% → **91.11%** |
| nist-sp-800-37r2 | 957 | 38 → **47** | 16 → **16** | 22 → **31** | 0.39% | 1.67% → **1.67%** |
| nist-sp-800-161r1 | 454 | 13 → **158** | 0 → **71** | 13 → **87** | 0.68% | 0.00% → **15.64%** |
| nist-sp-800-53Ar5 | 61 | 43 → **68** | 9 → **25** | 34 → **43** | 0.05% | 14.75% → **40.98%** |
| nist-sp-800-53r5 | 389 | 21 → **62** | 7 → **39** | 14 → **23** | 0.11% | 1.80% → **10.03%** |
| cfpb-home-loan-toolkit | 83 | 61 → **65** | 19 → **19** | 42 → **42** | **4.61%** | 22.89% → **22.89%** |
| irs-form-1040-2025 | 24 | 23 → **23** | 22 → **22** | 1 → **1** | 0.43% | 91.67% → **91.67%** |
| **all eleven** | | **253 → 649** | **106 → 389** | **147 → 256** | | |

**The rate bound: met**, 0.00%..4.61% over the nine, worst `cfpb-home-loan-toolkit`, as under `-v2`.
**The count bound: breached where it was breached before and nowhere else** — `nist-sp-800-218`, 10
false against 7 declared, its own title, which the owner accepted for `-v2` on 2026-09-18; `-v3`
adds none there. Right headings rise 3.7×, false ones 1.7×, and precision against the authors' tags
rises from 42% to 60%.

**Why it holds where §7's clause did not.** The same documents that broke §7 are bounded here:
`nist-sp-800-161r1` 1,297 false headings under §7, 87 now; `nist-sp-800-171r3` 203 under §7, 5 now,
with 164 of its 180 declared headings found. Bold defined terms, bold table headers and bold inline
emphasis are not lines of their own in a block of their own.

**A looser isolation was measured and not taken**: a bold line whose block continues into another
bold line — a heading wrapped onto two lines, or a caption. It found one more right heading on five
documents and 66 more false ones, 56 of them on `nist-sp-800-161r1`, and on opendataloader-bench it
read a two-line bold table caption as half a heading. Half of either as a heading is worse than none.

**Numbering moves no verdict**, only levels, so it needs no row here; it is tested beside the rule.

### 9.1 On the benches

**opendataloader-bench, the harness's own evaluator** (`score.py`): MHS **0.3779 → 0.4703** over
the 107 documents whose ground truth holds a heading, median 0.1630 → 0.4859, non-zero on 58 → 74;
NID 0.8793 and TEDS 0.1728 unchanged. **19 documents rose and one fell, so §7.5's bar 2 — "MHS
rises, and falls on no document" — is breached on one document**: `01030000000121`, 0.9527 →
0.5742, where a bold `Restriction Enzyme Digest Prep (switch to the 1- 20-μL micropipette):` set
in a block of its own is read as a heading its ground truth does not hold. A colon ends a label in
ParseBench's ground truth (`Population:` is a title there), so no clause refusing a colon was built
for one document. Whether that one fall is accepted is the owner's call, as §7.5 reserves.

**ParseBench** (`docs/measurements/parsebench/`), at `4913f25` and with `type-size-v3`:
semantic formatting **0.3487 → 0.3502** — title hierarchy 0.3123 → 0.3173, the `is_title` checks
unchanged at 0.3638 — and visual grounding 0.2200 → 0.2224, where a bold heading now carries the
label `Section-header`. Content faithfulness 0.6622 → 0.6619. Most hierarchy edges still failing
on depth join two bold lines neither of which stands apart: 70 of 88, many on pages where the
leading-gap cut declined and nothing can stand apart. A heading set on two lines is still two
lines, which is what most of the remaining `is_title` misses are; that is the paragraph slice's,
not this rule's.

## 10. `type-size-v4` — the numbered bold line (2026-10-04)

On opendataloader-bench, after `-v3`, 34 of the ground truth's 193 headings were bold lines `-v3`
did not read, most of them numbered — `3.1. Status of Business Operations`, `7.1. Free Vortex` —
and set a leading over their text, so no leading-gap block holds them alone; 16 more sit just under
the 1.20× size cut and 31 run across two lines.

**Judging every bold line by the spacing was measured first, and refused.** With "stands apart"
read off the layout-unit rule's geometry — the nearest measured lines above and below do not
continue it — right headings rose 389 → 833 but false ones 256 → 884, and both bounds broke:
`nist-sp-800-171r3` 5 false headings → 203 (5.02%, and over its 180 declared), every `DISCUSSION`
and `REFERENCES` label its producer tagged `/P`; `nist-sp-800-218` 10 → 18. Readers call those
labels headings, as they do `-218`'s title, but the bound is the bound.

**What ships adds only the numbered bold line**: one opening with a section number (`3.1.`,
`IV.`) stands apart with room above it alone — more than half its height of space, or no overlap
across. Every other bold line still needs a leading-gap block of its own. Raw readings in
[`falsepos-v4.json`](falsepos-v4.json).

| document | declared | right v3 → **v4** | false v3 → **v4** | FP rate **v4** | recall v3 → **v4** |
| --- | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 | 9 | 1 → **1** | 2 → **2** | 1.67% | 11.11% → **11.11%** |
| irs-fw9 | 28 | 26 → **26** | 0 → **0** | 0.00% | 92.86% → **92.86%** |
| nist-sp-800-218 | 7 | 0 → **2** | 10 → **10** | 0.54% | 0.00% → **28.57%** |
| nist-sp-800-207 | 57 | 6 → **7** | 12 → **12** | 0.64% | 10.53% → **12.28%** |
| nist-sp-800-171r3 | 180 | 164 → **168** | 5 → **5** | 0.12% | 91.11% → **93.33%** |
| nist-sp-800-37r2 | 957 | 16 → **35** | 31 → **32** | 0.40% | 1.67% → **3.66%** |
| nist-sp-800-161r1 | 454 | 71 → **92** | 87 → **88** | 0.69% | 15.64% → **20.26%** |
| nist-sp-800-53Ar5 | 61 | 25 → **40** | 43 → **43** | 0.05% | 40.98% → **65.57%** |
| nist-sp-800-53r5 | 389 | 39 → **52** | 23 → **23** | 0.11% | 10.03% → **13.37%** |
| cfpb-home-loan-toolkit | 83 | 19 → **19** | 42 → **42** | 4.61% | 22.89% → **22.89%** |
| irs-form-1040-2025 | 24 | 22 → **22** | 1 → **1** | 0.43% | 91.67% → **91.67%** |

**Both bounds stand where they stood**: the rate band over the nine is 0.00%..4.61%, worst
`cfpb-home-loan-toolkit`, and the count bound is breached only by `nist-sp-800-218`'s title, 10
against 7, as the owner accepted for `-v2`. Right headings **389 → 464**, false ones **256 → 258**.

**The instrument's join changed with it.** `whitespace-tracks-v2` runs only where no structure is
declared, so a tree-stripped copy may now infer a table its tagged original does not, and read the
table's runs as one atom in reading order. `falsepos.py` therefore pairs the two extracts by where
each run sits — page, origin, text, the n-th of any repeats — instead of by node order, and still
refuses a pair it cannot complete. Line counts move by a fraction of a percent where a stripped
copy's table moved a run's band; §9's `-v3` readings were taken under the old join.

**opendataloader-bench**: MHS 0.4712 → **0.5398**, non-zero on 74 → 84 of 107, 12 documents rose and
one fell — `01030000000037`, 0.6698 → 0.6050, where the new `3.1.` heading is right and a figure
caption the size clause reads as a heading shares its hierarchy. NID 0.8849 → 0.8851, TEDS unchanged.
§7.5's bar 2 is breached on that document, as on `01030000000121` under `-v3`; both are the owner's
call.

**ParseBench**: overall unchanged at 33.11. Visual grounding 0.4030 → 0.4034 and title hierarchy
0.3173 → 0.3174, but content faithfulness 0.6627 → 0.6623 over six pages, each one where a section
number and its title share a baseline (`2.3 Foreign currency translation`) and become `##` lines
the content scorer marks down from the bold lines they were. The rule's gain is on the corpora whose
ground truth names headings, not on this one.

