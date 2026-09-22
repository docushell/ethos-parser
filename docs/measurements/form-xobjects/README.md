# Form XObject descent — measured, and the answer splits

**Measured 2026-09-22.** Task G3 in `.plans/PRIORITIES.md`, proposed by the PageIndex review
(`.plans/PAGEINDEX-REVIEW.md` §3.3) as *"the largest pure-recall item the review found"*.

**It is not that, and it is not nothing.** The measurement separates two populations that the
proposal had folded into one incidence figure:

| | |
| --- | --- |
| **The broad case — refuted.** A document that reads fine and happens to draw a form | On the two corpora this repository can re-measure, every undescended form together holds **1 464 bytes across 208 documents** — about a third of a page. Not a recall item |
| **The narrow case — real, and it is the one `extract.rs`:1043 names.** A page whose whole content is one form | **14 OmniDocBench documents (1.4%) emit an entirely empty artifact** and are **not** scans. For those the loss is total. How much text is behind them is **unmeasurable here**: that corpus cannot be re-fetched |

**The slice stays unbuilt, on narrower grounds than "it is not worth it".** The recall argument the
review made is refuted. The blank-artifact case survives, is already *declared* rather than silent
(§5), and would justify the work the moment a corpus of that shape can be pinned and redistributed
(§6). Nothing here refuses descent on principle — §1 is explicit that nothing does.

**Nothing was built.** No engine change, no wire change, no rule id, no `profile_sha256` move.

---

## 1. What was asked

`extract.rs`:334 counts `undescended_xobjects`; `lim::form_xobjects_not_descended` declares the count
per document, and the profile declares the policy as `form-xobject-text-not-descended`. Nothing in
[`06-STEAL-REFUSE.md`](../../06-STEAL-REFUSE.md) or [`00-NORTH-STAR.md`](../../00-NORTH-STAR.md)
refuses descending into a `/Form`; **the limitation exists because the work was not done, not because
it was decided against**. So the question was open, and the only thing missing was a number.

## 2. What ran

Two instruments, deliberately independent, so the population is cross-checked rather than asserted.

| instrument | what it reads | why |
| --- | --- | --- |
| [`declared_census.py`](declared_census.py) | this engine's own artifact — the document-scoped `form-xobjects-not-descended` limitation | what the engine *says* it skipped |
| [`formscan.rs`](formscan.rs) | the PDF itself, opening every `/Form` XObject and decoding its content stream | what those forms actually *contain* |

`formscan.rs` is **not a workspace member and not a cargo target** — it is the source of a throwaway
binary, kept here because a measurement whose method cannot be re-run is not reproducible
([`17-D1-SCOPE.md`](../../17-D1-SCOPE.md) §8). It pins `lopdf = { version = "0.44.0",
default-features = false }`, the engine's own declaration, so it reads a form stream exactly as the
descent would. To re-run it, drop it into a scratch crate with that one dependency.

Binary: `target/release/ethos-parser`, **ethos-parser 0.60.0**. Corpora: the 8 gate fixtures and the
200 `opendataloader-bench` documents, plus the committed OmniDocBench census (§4).

```bash
python3 docs/measurements/form-xobjects/declared_census.py target/release/ethos-parser \
    fixtures/gate ~/ethos-external-benchmarks/opendataloader-bench/pdfs
formscan fixtures/gate/*.pdf ~/ethos-external-benchmarks/opendataloader-bench/pdfs/*.pdf
```

## 3. The number

**208 documents, 0 failures.**

| | |
| --- | ---: |
| documents declaring `form-xobjects-not-descended` | **47** (22.6%) |
| undescended `Do` calls the engine counted | **70** |
| `/Form` XObjects `formscan` found independently | **70** |
| ...of which show any text at all | **44** |
| total text-showing operations inside them | **51** |
| **total bytes those operations show** | **1 464** |

Per document: **median 1** undescended form, max 9. 38 of the 47 have exactly one.

**The two instruments agree on 70 forms in 47 documents by different routes** — one reading the
engine's artifact, one reading the PDF with `lopdf`. That agreement is the reason the number is
quoted at all.

**Byte count, not operation count.** A single `Tj` can draw a paragraph, so 51 operations does not
bound text volume and could not decide this on its own. The bytes were measured for that reason, and
they are what settles it: 1 464 bytes is about a third of a page of prose **across the whole
corpus**, against `nist-sp-800-53Ar5` alone emitting 1 803 517 text nodes.

**Only one of the eight gate documents declares the code**, `nist-sp-800-171r3` with one form, on a
120-page document that already emits 172 054 text nodes.

**No document in either corpus loses its page to a form.** Zero of the 47 emit zero text nodes.

## 4. OmniDocBench — the narrow case is real, and it is the half that survives

> **Corrected 2026-09-22, same day, before this document had been acted on.** The first version of
> this section said the census *"predates the document-scoped counter"* and could only offer a
> shape-based upper bound of *"at most 14, consistent with, not confirmed"*. **Both statements were
> wrong.** `FORM_XOBJECTS_NOT_DESCENDED` was already present at `a18b2b0`, the build the census ran
> on, and the census carries it. The cause was a substring grep: `form-xobject` matches the
> profile-scoped `form-xobject-text-not-descended` **and** the document-scoped
> `form-xobjects-not-descended`, so the two were counted as one and the document-scoped code looked
> absent. The numbers below are read by exact code name and they make the case **stronger**, not
> weaker.

`extract.rs`:1043 motivates the counter with *"a page whose entire content is `q /Xf1 Do Q` — the
shape a page-slicing tool produces, and 4 of 104 sampled OmniDocBench documents"*. That shape is the
real prize: the form holds the whole page, not a caption.

The OmniDocBench v1_0 census — 981 documents, re-run 2026-09-18 on `a18b2b0`, recorded in
[`measurements/omnidocbench/`](../omnidocbench/README.md) — answers it directly:

| | |
| --- | ---: |
| documents declaring `form-xobjects-not-descended` | **256** (26.1%) |
| ...emitting zero text bytes | **26** |
| ...**and placing zero images either** | **15** (1.53% of the corpus) |
| of those 15, classified `no-text` **without** `scanned` | **14** |

**Those 14 are not scans.** A scanned page classifies `scanned` / `embedded-images`; these carry
`['no-text']` alone, place no image, emit no text and produce **an entirely empty artifact**. They
declare an undescended form on the same artifact. That is the `q /Xf1 Do Q` case, **attributable
rather than merely shape-matched**, and at 1.53% it corroborates the order of magnitude of the
`4 of 104` figure rather than contradicting it. A further **47 of the 256 emit under 100 bytes**.

**What cannot be measured here:** how much text is behind those 14 forms. The corpus is not on this
machine and cannot be re-fetched — OmniDocBench has shipped page images rather than PDFs since
2025-09-25, under a research-use-only licence. So the *incidence* of total loss is measured; its
*volume* is not.

The census file itself is `full_census.json` at the repository root and is **git-excluded**, so it is
not redistributable with the repository; the figures above are quoted here because the file is not.

## 5. Why the declaration already does the job

The descent was wanted for two things. The second is already shipped.

1. **Recover the text.** Measured above at 1 464 bytes over 208 documents. Not worth an L/XL slice.
2. **Stop a form-wrapped page reading as a blank one.** This is what `extract.rs`:1043 is actually
   about, and the **document-scoped counter shipped at v2.2-S2 closes it**: such a page carries
   `form-xobjects-not-descended` with a count, and the detail says in as many words that *"a short
   run list on this document is a declared gap, not a sparse page."* §4's 14 documents are exactly
   that population, and they carry the code — so a reader of one of those artifacts **is told**, and
   the empty output cannot be mistaken for a blank page.

That is the argument for leaving the slice unbuilt: **the recall half is 1 464 bytes, and the
honesty half is already shipped for the population where the loss is total.** What is *not* claimed
is that those 14 documents lose nothing — they lose everything, the artifact says so, and nobody has
measured what.

## 6. What would reopen it

The trigger is sharper than "a corpus where the bytes are large", because §4 already identifies the
population that matters.

- **A redistributable corpus of the blank-artifact shape** — born-digital pages whose whole content
  is one form. §4 measures the incidence at 1.4% but cannot measure the volume, and that is the one
  number the decision turns on. `17-D1-SCOPE.md` §8's rule applies: it must be a corpus this
  repository can pin and redistribute, or the measurement is not reproducible.
- **Any one of those 14 documents obtained and pinned here.** A single fixture whose whole page sits
  behind a form would make the case concrete and give the slice a regression test on day one.
- **A named caller** holding documents of that shape — which converts the 1.4% from a corpus
  statistic into a requirement.

## 7. What this does not say

It does not say descending is wrong, refused, or dishonest — §1 is explicit that nothing refuses it.
It does not say no document anywhere loses text to a form; §4 says the opposite, with a bound it
cannot tighten. It says only that **on the evidence this repository can re-measure today, the work is
not justified**, and it records the number so the case is not re-argued from scratch.
