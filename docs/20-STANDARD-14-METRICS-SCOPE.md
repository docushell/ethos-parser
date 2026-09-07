# 20 — Standard-14 metrics: measured, and the decision the owner took

**This document is a measurement, not the decision itself.** It follows
[`17-D1-SCOPE.md`](17-D1-SCOPE.md), [`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md) and
[`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md): a question was asked, an
instrument was built, and what it found is written down before anything is built on it. Unlike 17
and 18 it does not end in a refusal, and unlike 19 it does not end in a probe list. It ends in a
**recommendation and a licence the owner had to accept or decline**, because the cost is not
engineering effort — it is a vendored dataset under a licence no tool here can check.

**Accepted 2026-09-07 as decision #22** of [`00-NORTH-STAR.md`](00-NORTH-STAR.md). That row is
where the decision lives; this document is the argument behind it, and §6 is what the row had to
carry. **Shipped at 0.51.0**: every item in §6 landed, and the recovery was re-measured on the
shipped code — 12 937 → 23 885 ink boxes measured of 25 123, and `font-widths-absent` counts
66 → 0, over the 37 corpus documents that declare a Core-14 face with no `/Widths` and no
`/FontDescriptor`. §3's 83.4% was the spike's prediction over a differently-found 36-document set;
the shipped figure is in [`../CHANGELOG.md`](../CHANGELOG.md) under 0.51.0 and is the one to quote.
The ASCII limit this left behind was measured in turn and **refused** —
[`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](21-STANDARD-14-ASCII-COVERAGE-SCOPE.md).

**It is not an ADR, and this repository has none.** There is no `docs/adr/` and never has been;
the two ADR numbers cited in this tree — ADR-0004 in
[`check_dco.py`](../.github/scripts/check_dco.py) and ADR-0009 in
[`03-V0-SCOPE.md`](history/03-V0-SCOPE.md) — resolve to the sibling Ethos repository, not to
anything here. The `docs/adr/` convention comes from an `AGENTS.md` that is **untracked** — local
tooling scaffolding rather than a file this repository carries — so there is no divergence to
reconcile. What this repository uses is a numbered scope document plus a numbered row in
[`00-NORTH-STAR.md`](00-NORTH-STAR.md), and this follows that. The one place an ADR is genuinely
mandatory here is [`deny.toml`](../deny.toml), for exceptions to the dependency allowlist — and
§6.1 is why this decision does not reach it.

---

## 1. The question

A PDF may declare one of the **standard 14 fonts** — Helvetica, Times, Courier, Symbol,
ZapfDingbats and their styles — with no `/Widths` array and no `/FontDescriptor` at all:

```
/Type /Font /Subtype /Type1 /BaseFont /Helvetica
```

PDF 32000-1 §9.6.2.2 permits this **because a conforming reader is expected to hold the built-in
AFM metrics**. This profile does not vendor them, so those runs report `advance: null`, take the
typed-absent path in [`Font::ink_box`](../crates/ethos-parser-pdf/src/fonts.rs), carry no measured
ink box, and are therefore **omitted from `ethos.grounding.v1` entirely**. They have text and they
cannot be quoted.

## 2. What was measured

A throwaway spike over the 36 documents of the OmniDocBench `v1_0` corpus that declare
`font-widths-absent`, modelling `ink_box`'s actual order — a vertical envelope **first**, then a
width greater than zero — against Adobe's Core-14 AFMs. See
[`measurements/omnidocbench/`](measurements/omnidocbench/README.md) for the corpus and its terms.

## 3. What it found

**10 749 of 12 885 ungroundable nodes recover — 83.4%.** Corpus-wide that is **21.1% of every
ungroundable node** (10 749 of 50 983), against an upper bound of 23.8%, so roughly nine tenths of
the theoretical gain is real.

| recoverable, by face | nodes | vertical source |
| --- | ---: | --- |
| Times-Roman | 9 854 | `Ascender`/`Descender` |
| Times-Italic | 396 | `Ascender`/`Descender` |
| Helvetica-Oblique, Helvetica, Helvetica-Bold | 347 | `Ascender`/`Descender` |
| Times-Bold | 98 | `Ascender`/`Descender` |
| Courier | 50 | `Ascender`/`Descender` |
| **Symbol** | **4** | `FontBBox` |

**Three findings qualify that number, and each is the kind this repository publishes rather than
smooths.**

**3.1 — The Symbol/ZapfDingbats problem is four nodes.** `Symbol.afm` and `ZapfDingbats.afm`
declare no `Ascender` or `Descender` at all, only `FontBBox`, so they need a documented fallback.
On this corpus that fallback carries **4 nodes** and ZapfDingbats carries **zero**. It is a
footnote, not a design phase — and it is stated here because the reverse would have been assumed.

**3.2 — What does not recover, should not.** 1 867 nodes (14.5%) sit on `Arial`,
`TimesNewRomanPSMT`, `TimesNewRoman`, `NotoSans-Regular` and `FZCCHJW--GB1-0`. Those are **not
Core-14 names**. Supplying Helvetica's metrics for Arial is a metric *substitution*, which is
exactly the guess [`../vendor/README.md`](../vendor/README.md) refuses, and the AFMs correctly do
not reach them. A further 269 are codes absent from the charmetrics.

**3.3 — It is concentrated, and the headline reads broader than the truth.** 26 of the 36
documents benefit, but the **top five hold 55%** of the recovery and all five are Elsevier-style
scientific papers. Median per document is 281 nodes. The honest sentence is *"unembedded-Times
scientific PDFs become citable"*, not *"21% of the corpus improves"*.

**Method caveat.** The spike models `ink_box` as vertical-then-width. It does not verify each
resulting rectangle is non-degenerate and on-page, so 10 749 is a tight upper bound within the
recoverable set rather than a guarantee.

## 4. The licence, which is the actual cost

Adobe's Core-14 AFMs are **APAFML** — permissive, and **not OSI-approved** (SPDX `APAFML`,
`isOsiApproved: false`). The operative sentence, from Adobe's own `MustRead.html`:

> may be used, copied, and distributed for any purpose and without charge, with or without
> modification, provided that all copyright notices are retained; that the AFM files are not
> distributed without this file; that all modifications to this file or any of the AFM files are
> prominently noted in the modified file(s)

Three obligations bite: `MustRead.html` must physically travel with the AFMs under that filename —
a `NOTICE` mention is not enough; per-file copyright lines must survive; and a generated `.rs` or
JSON table is a **modified file** and needs a prominent in-file note.

**The Apache-2.0 alternative is rejected, on evidence rather than taste.** Mozilla pdf.js ships
standard-font metrics under Apache-2.0, a licence already reviewed here. It also carries **no
`FontBBox`**, so Symbol and ZapfDingbats would have no vertical source at all; its Symbol and
ZapfDingbats ascent/descent are `NaN`; and it carries two verified transcription defects in
`xHeight`. Taking it means resting on Mozilla's Apache-2.0 assertion over numbers that are provably
Adobe's — while the ASF, looking at the same numbers, **explicitly carved them out of Apache-2.0**
at `LICENSE.txt` line 241. Two serious projects reached opposite conclusions; adopting the more
convenient one silently is not this repository's posture. Adobe's own files also carry per-glyph
`B llx lly urx ury` boxes, which are ink boxes and strictly better input than a font-level envelope.

**A governance point that is not a tooling problem.** `cargo deny` inspects the dependency graph
only. Vendored data files are never scanned by it, so AFMs would pass CI green while carrying an
undeclared licence. Green would not mean reviewed — the same blind spot `deny.toml` already
records for `vendor/cmaps/`.

## 5. The written refusal this would reverse, and why it is wrong here

[`../vendor/README.md`](../vendor/README.md) carries a standing refusal:

> The advance is genuinely unknown from the document alone. Reporting it as unknown is correct;
> guessing it is exactly the failure this project exists to refuse.

**That reasoning does not hold for this case, and the engine already contains the counter-argument
one function away.** [`fonts.rs`](../crates/ethos-parser-pdf/src/fonts.rs) says of a composite
font's `/DW`:

> §9.7.4.3: `/DW` defaults to 1000 when the document omits it. Reading a normative default is
> reading the document, not guessing at it.

A document naming `/BaseFont /Helvetica` with no `/Widths` is doing exactly that. §9.6.2.2 makes
the metrics **known**, and merely not present. This is a normative default, not a substitution —
and the line between them is the one §3.2 holds: reading Helvetica's own metrics for Helvetica is
reading the document; supplying them for Arial is not.

The refusal also describes the gap as **widths only**. That is narrower than the limitation: these
fonts declare no descriptor, so ascent/descent is missing too, and `ink_box` needs it **first**.
The repository's own description of what it refuses is smaller than what it refuses.

## 6. What the decision row says

Taken 2026-09-07 as row **#22** of [`00-NORTH-STAR.md`](00-NORTH-STAR.md), after #21. What it had
to carry, and does:

1. **No `APAFML` entry in `deny.toml` — stated, not left silent.** The allowlist governs crate
   licences in the resolved dependency graph. AFMs are data and never enter it, exactly as
   `deny.toml` already records for `vendor/cmaps/`, which it calls data rather than a crate: *"It
   does not appear in the dependency graph; its NOTICE obligation is handled in ./NOTICE."* An
   entry `cargo deny` can never match is exactly the "just in case" entry that file's own header
   forbids. The row has to say so, because a reader who sees a non-OSI-approved licence arrive with
   no allowlist change will otherwise read it as a review that was skipped. It was not skipped — it
   is §4's blind spot, and items 2 and 3 are what stands in for the absent tooling.
2. **`vendor/afm/`** holding the pristine AFMs **and `MustRead.html` under that exact filename**,
   with a pinned sha256 and the provenance URL, on the pattern `vendor/` already uses.
3. **A `NOTICE` paragraph**, and an automatic modification note in any generated table.
4. **The correction to [`../vendor/README.md`](../vendor/README.md)**, whose current sentence
   would otherwise contradict the shipped behaviour.
5. **`cmap_data_version`'s sibling** — a profile field naming the metrics source, so artifacts from
   before and after are correctly non-comparable. Contract §2: anything that can change a byte of
   output belongs in the profile.

## 7. What would make this unnecessary

A corpus whose documents embed their fonts. The gap exists only for **unembedded** standard-14
declarations, which are a convention of older scientific typesetting rather than a property of PDF
— §3.3's concentration is that convention showing up. If the deployment population embeds its
fonts, this recovers nothing and should not be taken.
