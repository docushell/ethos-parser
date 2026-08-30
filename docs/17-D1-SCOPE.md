# 17 — D1 scope: declared document splits — measured and refused

**Scope authority for D1.** There are no milestones, because there is no work. This document
records what was proposed, what was measured, and why the measurement ends it.

**D1 has no roadmap row and does not get one.** [`02-ROADMAP.md`](02-ROADMAP.md) says no new version
numbers get invented; this document is the argument that none is needed.

---

## 1. The one sentence

**A PDF that is a packet of many documents should say so with the file's own words — and across
every document this repository owns, the file has no such words.**

## 2. What was proposed

A packet is real. A filing bundle, a closing binder, a submission pack: one PDF holding twelve
documents. A citation into one of those today reads *page 47 of `packet.pdf`* when the honest
address is *page 3 of the invoice inside `packet.pdf`*, and that is a field-precision gap rather
than a cosmetic one.

The proposal, following the shape that produced this engine's best table result: read the splits
the document **declares**, emit nothing where none are declared, never infer a boundary. That is
`tagged-tables-v1`'s posture applied to a second question.

## 3. The candidate signals, and what each actually declares

The proposal is only as good as the existence of a signal that genuinely states a boundary. Five
were considered. **Four declare something else, and the fifth declares nothing here.**

| Signal | What the specification says it is | Boundary? |
| --- | --- | --- |
| `/Outlines` | A navigation tree — bookmarks a reader may jump to | **No.** A three-hundred-page book with chapter bookmarks is one document. Treating an outline entry as a boundary is inference, which is the one thing this proposal forbade itself |
| `/PageLabels` | Page **numbering** ranges — roman front matter, then arabic | **No.** It declares what a page is *called*. A numbering restart is evidence of nothing but numbering |
| `/Collection` | A PDF Portfolio: the file is explicitly a container of documents | **Yes** — and see §4 |
| `/Names` → `/EmbeddedFiles` | Attachments | **No, and the wrong shape.** An attachment is a separate file carried alongside, not a division of this document's pages. "Split" is the wrong word for it |
| `/Part`, `/DocumentFragment`, repeated top-level `/Document` | Standard structure elements naming a large-scale division | **Yes** — and see §4 |

Two of the five are honest boundary declarations. The question is then purely empirical: do they
occur?

## 4. The measurement

Over **every one of the 45 PDF fixtures in this repository**, counting raw object occurrences:

| Signal | Occurrences |
| --- | --- |
| `/Collection` | **0** |
| `/EmbeddedFiles` | **0** |
| `/PageLabels` | **0** |
| `/Part` | **0** |
| `/DocumentFragment` | **0** |
| `/Outlines` | 8, in six documents — and §3 says what an outline is not |

And in the structure tree, read through the engine's own walk rather than by grepping bytes, on the
eight gate documents — 2,068 pages, every tagged run's `role_path` inspected:

| Document | Tagged runs | Distinct top-level roles |
| --- | --- | --- |
| `irs-f1040sd-2025` | 1,098 | 1 — `Document` |
| `irs-fw9` | 1,097 | 1 — `Document` |
| `nist-sp-800-161r1` | 529,859 | 1 — `Document` |
| `nist-sp-800-171r3` | 165,381 | 1 — `Document` |
| `nist-sp-800-207` | 86,098 | 1 — `Sect` |
| `nist-sp-800-218` | 59,682 | 1 — `Sect` |
| `nist-sp-800-37r2` | 384,835 | 1 — `Sect` |
| `nist-sp-800-53Ar5` | 1,649,453 | 1 — `Document` |

**Every document declares exactly one top-level element.** Not one declares two. There is no packet
in this corpus, and there is no declared division inside any document in it.

## 5. Why that ends it

**A detector with no positive case is not a detector, it is an assertion.** Built as specified, D1
would emit nothing on all 45 fixtures and pass every test by doing so. Its correctness would rest
entirely on code review, because no document available to this repository could distinguish a
working implementation from one that returns the empty set unconditionally.

This repository has a name for shipping in that state. [`CAPABILITY.md`](CAPABILITY.md) closes with
it: *when something moves from Cannot to Can, it moves with a measurement, not with a sentence.*
D1 has no measurement to move with.

The comparison to the parked table chase is worth making because D1 is **worse**, and the
difference is instructive rather than rhetorical. Decision #18's geometric detectors score 0‰ on ten
of twelve documents — but they score 590‰ on one and 70‰ macro overall, so the band has a shape and
the rule has demonstrated it can fire. D1's band would be a single point at zero, with no document
anywhere in the repository able to move it.

## 6. What D1 is not

- **Not "later".** A deferral names a version, and there is no version this belongs to.
- **Not a refusal on principle.** Every other refusal in
  [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) is structural: floats on the wire are lossy whatever
  the corpus, and converting office files to PDF invents pagination on every document ever written.
  **D1 is refused on evidence, and evidence can change.** §8 says exactly what would change it.
- **Not a claim that packets do not exist.** They plainly do. The claim is narrower and entirely
  about this repository: nothing here can measure one, so nothing here can honestly ship a reader
  for one.
- **Not a reason to read `/Outlines` "while we are here".** An outline is a navigation aid. Putting
  one on the wire under a name suggesting a document boundary would be **P14 wearing a different
  hat** — structure inferred from presentation, indistinguishable on the wire from structure the
  author declared.

## 7. What this cost, and what it bought

The investigation is the deliverable. It cost two measurements and no production code, and it
bought a written answer to a question that would otherwise be re-proposed every time somebody meets
a filing bundle. [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) exists for exactly this failure — *a
good-faith PR that imports a competitor's capability along with the bug that makes it dishonest* —
and D1's row belongs in it beside the `liteparse` adapter, which was also built as far as the
evidence allowed and then refused.

## 8. What would reopen it

Both halves, not either:

1. **A corpus.** Public, redistributable documents this repository owns, containing at least one
   genuine `/Collection` portfolio or a structure tree declaring two or more top-level `/Document`
   or `/Part` elements. Without a positive case there is nothing to test against.
2. **The owner choosing to resume**, the same second half decision #18 left outstanding for the
   geometric table chase.

**Neither half is met, and the first is not met by finding such a file on the internet** — it has to
be one this repository can redistribute and pin, or the measurement is not reproducible.

## 9. Standing rules this decision keeps

1. **A gap is never presented as a success.** An empty split list on every document is a gap.
2. **No invented identifier, coordinate or boundary** — and a boundary inferred from a bookmark is
   invented no matter how reasonable the inference looks.
3. **Refusals are recorded with their reopening conditions**, so the decision is taken again on
   purpose rather than lapsing.
