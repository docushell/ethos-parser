# 08 — v1 scope

**Status:** implementation authority for v1 · **Sources:** `02-ROADMAP.md` v1 row, `06-STEAL-REFUSE.md`
**Build order:** `09-V1-MILESTONES.md` · **Predecessor:** `03-V0-SCOPE.md` (v0, frozen at M7)

---

## 1. What v1 is

**The DocuShell replacement gate.** v0 proved the engine can read a document honestly and say what
it could not do. v1 is the version at which DocuShell could stop calling something else.

`02-ROADMAP.md` stated the gate as three conditions. Two still bind as pass/fail; the third is
**parked**.

| Gate condition | Status | Meaning |
| --- | --- | --- |
| **Fabrication rate 0** | **binds** | No cell text the document does not contain. Not "low" — zero |
| **Cross-check diagnostics emitted** | **binds** | Geometric and structural derivations of the same cell are compared, and disagreement is on the artifact |
| Table-cell accuracy **> 0.489** on a labelled set | **parked as pass/fail** (owner, 2026-08-19) | **64‰** is this engine on **four tagged PDFs this repository owns**. **0.489** is a published ODL-local table score on **their** corpus. Same unit, different exam. The chase is **parked** until this repository has a labelled set it owns and chooses to resume |

The first two are properties of the design, testable from the first slice, and they are not
negotiable. The third was always the last slice's problem; it is now nobody's until the owner says
otherwise. **Parking is not a pass. v1 is not complete**, the number stays measured at 64‰, and
§3 below still says how not to chase it.

## 2. What v1 is not

**v1 is not "everything left over."** It is a bounded list, and this section is the answer to
"while we're in here, could we also…".

| Out of v1 | Lands at | Why not now |
| --- | --- | --- |
| OCR, in any form | v4 | Unchanged from v0. Recognition is a different profile and a different trust ladder |
| Office formats | v2 | Anydoc-native, never a LibreOffice→PDF bridge |
| Assist / VLM / agents | v3 | `Proposed` only, and rule 7 still holds |
| MCP server, Python / Node SDKs | v1.2 | The locator-handle discipline must settle before the first tool exists |
| Markdown / HTML as evidence | v1.1 | Ships with the Anchor Map or not at all |
| A second PDF backend | undecided | One backend, one quirk set, one declared limitation set |
| Verification of any kind | never here | `07-VERIFY-BOUNDARY.md`. v0.1 added a way to *invoke* a verifier and nothing else |

## 3. About 0.489, and how not to chase it — **parked, 2026-08-19**

**0.489 is ODL-local.** It is the deterministic configuration's table-cell accuracy — the only
number two publishers report bit-identically, which is exactly why it was picked as the bar. It was
a reproducible target rather than a good one.

**And it is not comparable to what this repository measures.** **64‰** is this engine on **four
tagged PDFs this repository owns**. **0.489** is a published ODL-local table score on **their**
corpus. Same unit, different exam. The chase is **parked** until this repository has a labelled set
it owns and chooses to resume. **Fabrication 0 still binds. v1 is not complete.**

**ODL-hybrid scores roughly 0.9×, and that is a different product.** The hybrid path is not
deterministic, and a determinism contract cannot sit under it. That one is not parked — it was never
a target, and adopting a non-reproducible method to beat a non-reproducible number would forfeit the
property this engine exists to have.

Three rules follow, and they bind every slice:

1. **No slice is tuned against 0.489.** Originally this read "no slice before S7", on the reasoning
   that a detector tuned against a number nobody has computed yet is tuned against its author's
   intuition. S7 has since computed it and missed, and v1.1, v1.2 and v2 shipped anyway — so the
   implication that S7 had to be *cleared* before other work moved is already historical. What
   replaces it is stronger: with the chase parked, **no slice is measured against 0.489 at all**,
   and no slice may cite parking as progress.
2. **No published comparison.** No bake-off table, no ranking, no "better than" in any README —
   the v0 posture (`03-V0-SCOPE.md` §6) is unchanged. The number S7 produced is stated with its
   corpus, its configuration, and its harness, or it is not stated.
3. **Fabrication 0 is not traded against accuracy.** A cell filled with plausible neighbouring text
   scores better and is worse. If the two ever appear to conflict, fabrication wins and the
   accuracy number is reported lower. Parking the chase does not soften this one — it is the
   condition that still decides whether a detector may ship.

## 4. In — the v1 row, as slices

`02-ROADMAP.md` lists v1's contents as one line. It is not implementable as one change, so it is
ordered here. `09-V1-MILESTONES.md` is the detail; this is the map.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and `09-V1-MILESTONES.md` | **done** |
| **S1** | Vector paths · ruled tables from rectangles · `TableCellPosition` / `CellSlot` · locator cross-check | **done** |
| **S2** | Unruled tables: alignment / whitespace dual-mode | **done** |
| **S3** | Tagged-PDF consumption; the already-captured `mcid` put to use | **done** |
| **S4** | Forms and annotations as typed, distinguishable nodes | **done** |
| **S5** | Multi-column reading order, with a stable versioned rule | **done** |
| **S6** | Images, DPI screenshots, hidden / off-page findings, annotated PDF | **done** |
| **S7a** | The labelled set and the harness, measuring only | **done** |
| **S7b** | Detector calibration, measured. Six repairs rejected, one shipped (`ruled-rects-v2`) | **done** |
| **S8** | The parked stroke-ruled rule, defect-fixed and shipped as a third rule | **done** |
| **S7** | The > 0.489 gate; v1 declared done | **measured and MISSED: 64‰ — chase parked, slice still open** |

**S7 is open, with the number written down, and the chase for the number is parked.** The gate
metric exists, is documented in `docs/table-gate-v1.md`, runs in CI and reruns to the same value;
macro cell-F1 is **64‰**, historically measured against a 489‰ comparator that this repository no
longer treats as a shipping floor (`00-NORTH-STAR.md` #10). Parking it closes nothing.

S7b ran seven investigations: the six aimed at the *alignment* rule were all measured and rejected,
and one — `ruled-rects-v2`, which stops a background panel from witnessing its own lattice —
shipped, taking precision from 900‰ to 1000‰ and the gate from 43‰ to 61‰ without losing
a true positive. **S8 then shipped a third detection rule**, `stroke-ruled-v1`, for the grid a
document draws as ruling lines: `cfpb-home-loan-toolkit` 246‰ → 259‰ and the gate 61‰ → 64‰, with
`irs-form-1040-2025` held at 0 tables and fabrication still 0.
**v1 is not done and is not claimed to be.**

**v1.1 has started anyway, and that is not a contradiction.** `02-ROADMAP.md`'s next row is Safe
Markdown, the owner asked for it, and it is scoped in [`10-V11-SCOPE.md`](10-V11-SCOPE.md) and
[`11-V11-MILESTONES.md`](11-V11-MILESTONES.md). Nothing in v1.1 closes S7: it adds an *output*
(`ethos.markdown.v1`, always paired with its Anchor Map) and touches no detector. The gate still
reads 64‰, and no v1.1 slice may be cited as evidence that it does not.

**Slices are `v1-S*`, not M-numbers.** The milestone chain ended at M7 with v0. Numbering later
work `M8` would imply v0's acceptance list continued into it, and it did not.

## 5. The ruled/unruled split, and why S1 stopped where it did

`06-STEAL-REFUSE.md` P12 describes a dual-mode detector: ruled tables from ruling lines, unruled
from alignment. S1 implemented **the ruled half only**; S2 added the other half as a *separate
rule under a separate id*, which is the whole point of the split rather than an accident of
sequencing.

The split is not arbitrary sequencing. A ruling line is *evidence in the document* — the author
drew it. An alignment cluster is *an inference about the document* — the author drew nothing, and
the detector decides. Those two deserve different derivation classes, different tolerances and
different tests, and building the second before the first has shipped is how the first inherits the
second's guesswork.

**A measurement worth recording, because it shaped S1.** The Ethos conformance fixture named
`synthetic/table-regular-grid` contains **no path operators at all** — its 3×2 grid is laid out by
text position alone (six `Tm`/`Tj` pairs, zero `re`). Nothing in the conformance corpus draws a
rule. So the ruled detector cannot be demonstrated on that corpus, and S1 authors an engine-owned
CC0 fixture that does draw one. `table-regular-grid` was an **S2** fixture wearing an S1 name, and
until S2 it correctly reported "looked, found none". **S2 shipped and it is now the unruled
golden**, emitting its 3×2 grid under `unruled-align-v1`.

**Both halves shipped, and both name themselves.** Every table carries `detection_rule`, and the
profile lists both ids rather than one string, because "the document drew this grid" and "a
detector inferred this grid" are the two claims this split exists to keep apart. Where both rules
could describe one region, the ruled one wins and the unruled one is dropped: the author's
evidence outranks our inference, and the two grids are never averaged into a third that neither
rule found.

## 6. Standing rules, carried forward from v0

Unchanged, and repeated because v1 is where the pressure to bend them arrives.

1. **No public confidence field, ever.** A cross-check status is a typed vocabulary, never a score.
2. **No box derived from a font size**; typed absence instead.
3. **No silent drop, and no silent repair.** A grid that does not tile is a diagnostic with a check
   version, not a nudged coordinate.
4. **No invented coordinate, identifier, fingerprint or pagination.** A cell with no text is empty,
   never filled from nearby.
5. **Fail closed, and distinguishably.**
6. **Byte identity is a test.**
7. **The Ethos tree is read-only.**
8. **No verification.** A table does not acquire a verdict.

## 7. Capabilities are the v1 contract surface

v0 established that a `true` capability needs a proof test and a `false` one needs a declared
limitation. v1 flips capabilities, so that rule is where its honesty lives.

| Capability | v0 | v1 target | Flipped by |
| --- | --- | --- | --- |
| `tables` | false | **true** | S1 (ruled), widened by S2 |
| `structural_locators` | false | **true** | S3 |
| `multi_column_reading_order` | false | **true** | S5 |
| `images` | — | **true** | S6 |
| `page_screenshots` | — | false | **not shipped** — no renderer this project may depend on; see `09-V1-MILESTONES.md` S6 |
| `char_offsets` | false | true | whichever slice makes elements coarser than spans |
| `form_fields` | — | **true** | S4 |
| `annotations` | — | **true** | S4 |

**A capability flips when the proof exists, not when the code lands.** `tables: true` means *this
profile looked for tables*, which is why an empty array is a real answer and an absent key is a
different one. It does not mean every table is found.

S1 declared the gap as `unruled-tables-not-detected`. S2 shipped the alignment rule, so that
sentence became false and **the code was deleted rather than reworded** — a limitation that
outlives the gap it describes is worse than none, because a reader acts on it. S2's replacement,
`stroke-ruled-tables-not-detected`, met the same end at **S8**, which shipped the stroke-ruled
rule. What "looked" now covers is all three rules, and the leftover is narrower again:
`undrawn-table-edges-not-supplied` — an edge the document never drew is not supplied to complete a
grid, so a table ruled underneath each of its cells comes back one row short rather than finished
with a coordinate nobody wrote.

S5 made the same move for reading order. v0 declared `multi-column-reading-order` — *a
multi-column document is read in the WRONG ORDER* — on every artifact. `gutter-columns-v1` made
that sentence false, so the code left the default profile rather than being softened. It is
**kept in the vocabulary**, because a profile may still turn the capability off and for that
profile the sentence is still true; this is the S3 lesson, where `structural-locators-not-claimed`
survived its own slice for the same reason. What replaced it is
`reading-order-geometric-only`: the rule reads whitespace and nothing else, so column structure
carried only by a tag tree is not seen, and a run whose font supplies no advance is given a
declared minimum extent rather than a measured one.

**Structure-order reading is a named leftover, not a gap this slice half-filled.** Emitting nodes
in `/K` order is a different rule over different evidence and would need its own id; `structure.rs`
still contains no sort, and a guard test in it still says so.

---

## PR review checklist

- [ ] The change belongs to a numbered slice in §4, or it is out of scope
- [ ] No item from §2 has crept in
- [ ] No accuracy number is claimed that a committed harness does not produce (§3)
- [ ] Fabrication stayed 0; nothing was traded for a better-looking result
- [ ] A new capability ⇒ a proof test ⇒ a new profile hash
- [ ] A new limitation is declared in the artifact, not only in a doc comment
