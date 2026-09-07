# 21 — Widening standard-14 coverage past ASCII: measured, and refused

**This document is a measurement that ends in a refusal.** It follows
[`17-D1-SCOPE.md`](17-D1-SCOPE.md) and [`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md), which end
the same way, rather than [`20-STANDARD-14-METRICS-SCOPE.md`](20-STANDARD-14-METRICS-SCOPE.md),
which ended in a recommendation the owner accepted as decision #22.

Decision #22 shipped at 0.51.0 and named its own limit: a width is found by asking the font's own
decoder what a code means, so coverage stops where this profile's encoding tables stop. Both #22's
row and the 0.51.0 CHANGELOG entry say widening it *"needs the Annex D glyph-name column, which is
its own measurement rather than a guess bolted on"*. This is that measurement.

---

## 1. The question

After 0.51.0, 1 238 of 25 123 geometry entries are still typed-absent across the 37 OmniDocBench
documents that declare a Core-14 face with no `/Widths` and no `/FontDescriptor`. **How many of
those would a glyph-name table recover?**

The table in question is PDF 32000-1 Annex D's `WinAnsiEncoding` column — code to glyph *name*,
roughly 96 entries above ASCII, or 224 for the whole range. The repository already vendors Annex
D's code-to-*text* column, so this is the same table's other half, admitted on the same grounds:
a published specification, transcribed.

## 2. What was measured

Every typed-absent geometry entry in those 37 documents, classified by resolving each node's
`font_id` back to the `/BaseFont` its document declares — because the artifact carries the resource
name and not the face, and the first cut of this measurement was wrong for exactly that reason. It
counted absences per *document* rather than per *font*, which put 564 nodes in the "a glyph-name
table could reach this" column that a glyph-name table cannot reach at all.

## 3. What it found

| Class | Nodes | Share |
| --- | ---: | ---: |
| The font is **not** one of the standard 14 | 1 110 | 89.7% |
| Not a text node | 77 | 6.2% |
| Core-14, every code ASCII | 44 | 3.6% |
| **Core-14 and a code `WinAnsiEncoding` defines — what the table would reach** | **7** | **0.6%** |

**Seven nodes.** They are codes `0x93` and `0x94`, four of each — `quotedblleft` and
`quotedblright`. That is **0.03% of the 25 123 geometry entries** in these documents, and 0.6% of
the residual the widening was proposed to address.

**3.1 — The 1 110 are the decision working, not a shortfall.** They sit on faces the documents do
not name as standard-14. Supplying Helvetica's metrics for them is the metric *substitution*
decision #22 refuses, and no encoding table changes that. The largest single contributor is an
embedded subset font, `DXGYRQ+TTFF5AB400t00`, which carries its own `/Widths` and whose absence is
a missing **ink envelope** rather than a missing width — a different gap, reached by a different
fix, and not by this one.

**3.2 — The 44 are correct, and checking them was worth it.** They looked like a defect in what
0.51.0 shipped. They are not. Most are `Courier` code 32, whose AFM entry is
`C 32 ; WX 600 ; N space ; B 0 0 0 0` — a **space draws no ink**, so there is no non-degenerate box
to emit and declining to emit one is right. §3's method caveat in `20` predicted this class
directly: the spike *"does not verify each resulting rectangle is non-degenerate"*. The remainder is
a document declaring `WinAnsiEncoding` over `Symbol`, whose built-in encoding maps code 70 to `Phi`
and not to `F`; the two disagree, and reporting no advance is the honest reading of a font
dictionary contradicting itself.

## 4. The refusal

**Not built.** Between 96 and 224 entries of hand-transcribed specification data, to recover seven
nodes, is the trade this repository already refuses in two other places:

- `deny.toml`'s header — *"An entry added 'just in case' is a licence nobody reviewed"* — is the
  same argument about the same kind of table.
- [`20`](20-STANDARD-14-METRICS-SCOPE.md) §4 rejected pdf.js's Apache-2.0 metrics partly because
  they carry **two verified `xHeight` transcription defects**. A 224-entry table typed by hand
  carries that identical risk, and it would be carried for a 0.03% return.

Decision #20's shape applies in reverse: that row refused a *measured performance win* on honesty
grounds and named the price. This one refuses a *measured correctness win* on transcription-risk
grounds, and the price is seven nodes.

## 5. What would reopen it

- **A corpus where the number is not seven.** The 0.03% is a property of this corpus — English
  scientific PDFs in unembedded Times and Helvetica. A population using Latin-1 accented text in
  standard-14 faces would move it, and the instrument under
  [`measurements/omnidocbench/`](measurements/omnidocbench/README.md) re-measures it.
- **A derived table rather than a transcribed one.** The objection is transcription risk, not the
  table. A generator that emits the mapping from a source that can be checked against the
  `WinAnsiEncoding` text column already vendored — every entry cross-validated, no hand-typing —
  removes the whole argument in §4. It would still need the seven-node return to be worth the
  machinery.
- **A different consumer.** These names would also widen `/Differences` decoding, which drops runs
  today when it meets a name the 61-entry glyph table does not carry. That is a **text** gap rather
  than a geometry one, it was not measured here, and it should not be folded into this decision
  without its own number.
