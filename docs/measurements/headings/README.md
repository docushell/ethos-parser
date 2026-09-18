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
