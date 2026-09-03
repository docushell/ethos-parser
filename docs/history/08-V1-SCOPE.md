# 08 — v1 scope

**Implementation authority for v1.** Build order is in
[`09-V1-MILESTONES.md`](09-V1-MILESTONES.md).

---

## 1. What v1 is

**The DocuShell replacement gate.** v0 proved the engine can read a document honestly and say what it
could not do. v1 is the version at which DocuShell could stop calling something else.

The gate has three conditions. Two still bind as pass/fail; the third is parked.

| Condition | Status | Meaning |
| --- | --- | --- |
| **Fabrication rate 0** | **binds** | No cell text the document does not contain. Not "low" — zero |
| **Cross-check diagnostics emitted** | **binds** | Geometric and structural readings of the same cell are compared, and disagreement goes on the artifact |
| Table-cell accuracy above 0.489 | **parked** | 70‰ is this engine on twelve tagged PDFs this repository owns, and the band is what to quote: 0‰..590‰, median 0‰, ten of twelve at zero. 0.489 is a published score on **their** corpus. Same unit, different exam |

The first two are properties of the design, testable from the first slice, and they are not
negotiable. The third was always the last slice's problem, and it is now nobody's until the owner
says otherwise.

**Parking is not passing**, the number stays measured, and §3 still says how not to chase it.

**Closed 2026-08-30 by decision #18.** This section read *"v1 is not complete"* for the eleven days
between the amendment above and that decision, and the sentence is recorded here rather than removed
because it was true when written. What changed is not the number — 70‰ still stands, with its band —
but what v1 closes *on*: a capability statement plus a band, rather than a macro. The chase stays
parked, and parking is what the decision records rather than what it leaves open.

## 2. What v1 is not

**v1 is not "everything left over."** It is a bounded list, and this section is the answer to *while
we're in here, could we also…*

| Out of v1 | Lands at | Why not now |
| --- | --- | --- |
| OCR, in any form | v4 | Recognition is a different profile and a different trust ladder |
| Office formats | v2 | Read natively, never through a PDF conversion |
| Assist, VLM, agents | v3 | `Proposed` only, and the processor-identity rule still holds |
| MCP server and the SDKs | v1.2 | The locator-handle discipline must settle before the first tool exists |
| Markdown and HTML as evidence | v1.1 | Ships with the anchor map or not at all |
| A second PDF backend | undecided | One backend, one quirk set, one declared limitation set |
| Verification of any kind | never here | v0.1 added a way to *invoke* a verifier and nothing else |

## 3. About 0.489, and how not to chase it

**0.489 is somebody else's local score.** It is the only table number two publishers report
bit-identically, which is exactly why it was picked as the bar — a reproducible target rather than a
good one.

**And it is not comparable to what this repository measures.** 70‰ is this engine on twelve tagged
PDFs this repository owns; 0.489 is a published score on their corpus. Same unit, different exam. The
chase is parked until this repository has a labelled set it owns *and chooses to resume*.

**The hybrid ~0.9 figure is a different product.** That path is not deterministic, and a determinism
contract cannot sit under it. It was never a target: adopting a non-reproducible method to beat a
non-reproducible number would forfeit the property this engine exists to have.

Three rules bind every slice:

1. **No slice is measured against 0.489 at all**, and no slice may cite parking as progress.
2. **No published comparison.** No bake-off table, no ranking, no "better than" in any README. A
   number is stated with its corpus, its configuration and its harness, or it is not stated.
3. **Fabrication 0 is never traded against accuracy.** A cell filled with plausible neighbouring text
   scores better and is worse. If the two ever appear to conflict, fabrication wins and the accuracy
   number is reported lower. Parking the chase does not soften this one — it is still the condition
   that decides whether a detector may ship.

## 4. The slices

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and the milestones | done |
| **S1** | Vector paths, ruled tables from rectangles, the cell model, the locator cross-check | done |
| **S2** | Unruled tables from alignment and whitespace | done |
| **S3** | Tagged-PDF structure trees; the captured marked-content ids put to use | done |
| **S4** | Forms and annotations as typed, distinguishable nodes | done |
| **S5** | Multi-column reading order under a stable versioned rule | done |
| **S6** | Images, hidden and off-page findings, the annotated overlay | done |
| **S7a** | The labelled set and the harness — measuring only | done |
| **S7b** | Detector calibration. Six repairs measured and rejected, one shipped | done |
| **S8** | The parked stroke-ruled rule, defect-fixed and shipped as a third rule | done |
| **S7** | The accuracy gate, and declaring v1 done | **measured and MISSED — open** |

**S7 is open, the number is written down, and the chase for it is parked.** The gate metric exists,
is documented in [`table-gate-v1.md`](../table-gate-v1.md), runs in CI, and reruns to the same value.
v2-S19's band shows the macro was never the right summary: 0‰..590‰, median 0‰, ten of twelve
documents at zero.

**What the last three slices actually established.** S7b ran seven investigations: the six aimed at
the *alignment* rule were all measured and rejected, and one shipped — stopping a background panel
from witnessing the grid its own decoration implies, which took detection precision from 900‰ to
1000‰ and the gate from 43‰ to 61‰ without losing a true positive. S8 then shipped a third rule for
grids drawn as ruling lines rather than filled boxes, moving one document from 246‰ to 259‰ and the
gate from 61‰ to 64‰, with the tax form held at zero tables and fabrication still 0.

**v1.1 and later started anyway, and that is not a contradiction.** Nothing in them closes S7 — they
add outputs and formats and touch no detector. The gate still reads what it reads on its own corpus,
and no later slice may be cited as evidence that it does not.

**Slices are numbered `v1-S*`, not `M*`.** The milestone chain ended at M7 with v0, and numbering
later work `M8` would imply v0's acceptance list continued into it. It did not.

## 5. The ruled/unruled split, and why S1 stopped where it did

S1 implemented **the ruled half only**; S2 added the other half as a separate rule under a separate
id, which is the point of the split rather than an accident of sequencing.

**A ruling line is evidence in the document — the author drew it. An alignment cluster is an
inference about the document — the author drew nothing and the detector decided.** Those deserve
different derivation classes, different tolerances and different tests, and building the second
before the first has shipped is how the first inherits the second's guesswork.

**A measurement that shaped S1.** The conformance fixture named `table-regular-grid` contains **no
path operators at all** — its 3×2 grid is laid out by text position alone. Nothing in that corpus
draws a rule, so a ruled detector cannot be demonstrated on it, and S1 authored its own fixture that
does draw one. That fixture was an S2 case wearing an S1 name; until S2 it correctly reported "looked
and found none", and it is now the unruled golden.

**Both halves name themselves.** Every table carries the id of the rule that produced it, and the
profile lists all the ids rather than one string, because *the document drew this grid* and *a
detector inferred this grid* are the two claims the split exists to keep apart. Where two rules could
describe one region, the ruled one wins and the other is dropped: **the author's evidence outranks
our inference, and two grids are never averaged into a third that neither rule found.**

## 6. Standing rules carried forward from v0

Repeated because v1 is where the pressure to bend them arrives.

1. **No public confidence field, ever.** A cross-check status is a typed vocabulary, never a score.
2. **No box derived from a font size** — typed absence instead.
3. **No silent drop and no silent repair.** A grid that does not tile is a diagnostic with a check
   version, not a nudged coordinate.
4. **No invented coordinate, identifier, fingerprint or page number.** A cell with no text is empty,
   never filled from nearby.
5. **Fail closed, and distinguishably.**
6. **Byte identity is a test.**
7. **The Ethos tree is read-only.**
8. **No verification.** A table does not acquire a verdict.

## 7. Capabilities are v1's contract surface

v0 established that a `true` capability needs a proof test and a `false` one needs a declared
limitation. v1 flips capabilities, so that rule is where its honesty lives.

| Capability | v0 | v1 | Flipped by |
| --- | --- | --- | --- |
| `tables` | false | **true** | S1, widened by S2 |
| `structural_locators` | false | **true** | S3 |
| `multi_column_reading_order` | false | **true** | S5 |
| `images` | — | **true** | S6 |
| `form_fields`, `annotations` | — | **true** | S4 |
| `page_screenshots` | — | false | **Not shipped** — no renderer this project may depend on |
| `char_offsets` | false | true | Whichever slice makes elements coarser than spans |

**A capability flips when the proof exists, not when the code lands.** `tables: true` means *this
profile looked for tables*, which is why an empty array is a real answer and an absent key is a
different one. It does not mean every table is found.

**When a gap closes, its limitation is deleted rather than reworded.** S1 declared
`unruled-tables-not-detected`; S2 made that sentence false and removed it. S2's narrower replacement
met the same end at S8. **A limitation that outlives the gap it describes is worse than none, because
a reader acts on it.** What is left is narrower again: an edge the document never drew is not
supplied to complete a grid, so a table ruled underneath each of its cells comes back one row short
rather than finished with a coordinate nobody wrote.

S5 made the same move for reading order, with one difference worth knowing: the old limitation code
is **kept in the vocabulary**, because a profile may still turn the capability off and for that
profile the sentence is still true. What replaced it on the default profile says what the rule
actually does — it reads whitespace and nothing else, so column structure carried only by a tag tree
is not seen, and a run whose font supplies no advance gets a declared minimum extent rather than a
measured one.

**Reading order from the structure tree is a named leftover, not a gap this slice half-filled.**
Emitting nodes in tag order is a different rule over different evidence and would need its own id.
The code contains no such sort, and a guard test says so.

---

## PR review checklist

- [ ] The change belongs to a numbered slice in §4, or it is out of scope
- [ ] Nothing from §2 has crept in
- [ ] No accuracy number is claimed that a committed harness does not produce (§3)
- [ ] Fabrication stayed 0; nothing was traded for a better-looking result
- [ ] A new capability has a proof test, and a new profile hash
- [ ] A new limitation is declared in the artifact, not only in a doc comment
