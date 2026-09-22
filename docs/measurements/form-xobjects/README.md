# Form XObject descent — measured, and not worth building

**Measured 2026-09-22.** Task G3 in `.plans/PRIORITIES.md`, proposed by the PageIndex review
(`.plans/PAGEINDEX-REVIEW.md` §3.3) as *"the largest pure-recall item the review found"*.

**It is not.** On the two corpora this repository can re-measure, the entire text behind every
undescended form is **1 464 bytes across 208 documents**. The slice is refused on measurement, the
way v1.2-S5's liteparse adapter and C1-S5's font-weight clause were.

**Nothing was built.** No engine change, no wire change, no rule id, no `profile_sha256` move. The
existing declarations stand exactly as they are, and §5 explains why they already do the job the
descent was meant to do.

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

## 4. OmniDocBench — where the case would have been, and why it stays unproven

`extract.rs`:1043 motivates the counter with *"a page whose entire content is `q /Xf1 Do Q` — the
shape a page-slicing tool produces, and 4 of 104 sampled OmniDocBench documents"*. That shape is the
real prize: there the form holds the whole page, not a caption.

The committed census [`full_census.json`](../../../full_census.json) covers all **981** OmniDocBench
v1_0 documents, and it **cannot answer this directly**: it predates the document-scoped counter
(v2.2-S2), so its only form code is the profile-scoped one, which rides all 962 readable artifacts
and says nothing about any of them. What it can answer is the shape:

- **225 of 981 emit zero text bytes**, all with `pages_with_text: 0`.
- **209 of those classify as `scanned` / `embedded-images`** — pages with no text layer, which is the
  OCR case (L24/L25, opt-in and profile-isolated), not this one.
- **16 emit zero text AND place zero images.** A page with no text and no image drawn is the shape an
  undescended form leaves behind. Two of the sixteen carry `vector-text` with `dense-graphics` and
  `table-likely` — text drawn as paths, which descending into a form would not recover either.

So **at most 14 of 981 (1.4%)** are consistent with the `q /Xf1 Do Q` case. **Consistent with, not
confirmed**: a scanned page and a form-wrapped page are indistinguishable in this census, and the
corpus can no longer be fetched to check — OmniDocBench has shipped page images rather than PDFs
since 2025-09-25, under a research-use-only licence. The `4 of 104` figure is neither reproduced nor
contradicted here, and this document does not claim to have done either.

## 5. Why the declaration already does the job

The descent was wanted for two things. The second is already shipped.

1. **Recover the text.** Measured above at 1 464 bytes over 208 documents. Not worth an L/XL slice.
2. **Stop a form-wrapped page reading as a blank one.** This is what `extract.rs`:1043 is actually
   about, and the **document-scoped counter shipped at v2.2-S2 closes it**: such a page now carries
   `form-xobjects-not-descended` with a count, and the detail says in as many words that *"a short
   run list on this document is a declared gap, not a sparse page."* The census of §4 shows the
   problem as it looked **before** that counter existed. A reader of a current artifact is told.

That is the whole argument for refusing the slice: the honesty half is done, and the recall half is
1 464 bytes.

## 6. What would reopen it

- **A corpus where the bytes are large.** The bar is this measurement's own shape: forms measured on a
  corpus this repository can pin and redistribute, showing text volume that matters against the
  documents' own totals. A born-digital corpus of page-sliced PDFs is the obvious candidate and this
  repository owns none.
- **A named caller** holding documents of that shape.
- **The `4 of 104` claim made checkable** — a re-fetchable corpus, or those documents pinned here.

## 7. What this does not say

It does not say descending is wrong, refused, or dishonest — §1 is explicit that nothing refuses it.
It does not say no document anywhere loses text to a form; §4 says the opposite, with a bound it
cannot tighten. It says only that **on the evidence this repository can re-measure today, the work is
not justified**, and it records the number so the case is not re-argued from scratch.
