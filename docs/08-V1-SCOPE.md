# 08 — v1 scope

**Status:** implementation authority for v1 · **Sources:** `02-ROADMAP.md` v1 row, `06-STEAL-REFUSE.md`
**Build order:** `09-V1-MILESTONES.md` · **Predecessor:** `03-V0-SCOPE.md` (v0, frozen at M7)

---

## 1. What v1 is

**The DocuShell replacement gate.** v0 proved the engine can read a document honestly and say what
it could not do. v1 is the version at which DocuShell could stop calling something else.

`02-ROADMAP.md` states the gate as three conditions, and all three must hold together:

| Gate condition | Meaning |
| --- | --- |
| Table-cell accuracy **> 0.489** on a labelled set | Better than the deterministic baseline two publishers report bit-identically |
| **Fabrication rate 0** | No cell text the document does not contain. Not "low" — zero |
| **Cross-check diagnostics emitted** | Geometric and structural derivations of the same cell are compared, and disagreement is on the artifact |

The second and third are properties of the design and are testable from the first slice. The first
needs a labelled set that does not exist yet, and is therefore **the last slice's problem**, not
every slice's.

## 2. What v1 is not

**v1 is not "everything left over."** It is a bounded list, and this section is the answer to
"while we're in here, could we also…".

| Out of v1 | Lands at | Why not now |
| --- | --- | --- |
| OCR, in any form | v2.1 | Unchanged from v0. Recognition is a different profile and a different trust ladder |
| Office formats | v2 | Anydoc-native, never a LibreOffice→PDF bridge |
| Assist / VLM / agents | v3 | `Proposed` only, and rule 7 still holds |
| MCP server, Python / Node SDKs | v1.2 | The locator-handle discipline must settle before the first tool exists |
| Markdown / HTML as evidence | v1.1 | Ships with the Anchor Map or not at all |
| A second PDF backend | undecided | One backend, one quirk set, one declared limitation set |
| Verification of any kind | never here | `07-VERIFY-BOUNDARY.md`. v0.1 added a way to *invoke* a verifier and nothing else |

## 3. About 0.489, and how not to chase it

**0.489 is ODL-local.** It is the deterministic configuration's table-cell accuracy — the only
number two publishers report bit-identically, which is exactly why it is the bar. It is a
reproducible target rather than a good one.

**ODL-hybrid scores roughly 0.9×, and that is a different product.** The hybrid path is not
deterministic, and a determinism contract cannot sit under it. Beating a non-reproducible number
by adopting a non-reproducible method would forfeit the property this engine exists to have.

Three rules follow, and they bind every slice:

1. **No slice before S7 is measured against 0.489.** A detector tuned against a number nobody has
   computed yet is tuned against its author's intuition.
2. **No published comparison.** No bake-off table, no ranking, no "better than" in any README —
   the v0 posture (`03-V0-SCOPE.md` §6) is unchanged. When S7 produces a number it is stated with
   its corpus, its configuration, and its harness, or it is not stated.
3. **Fabrication 0 is not traded against accuracy.** A cell filled with plausible neighbouring text
   scores better and is worse. If the two ever appear to conflict, fabrication wins and the
   accuracy number is reported lower.

## 4. In — the v1 row, as slices

`02-ROADMAP.md` lists v1's contents as one line. It is not implementable as one change, so it is
ordered here. `09-V1-MILESTONES.md` is the detail; this is the map.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and `09-V1-MILESTONES.md` | **done** |
| **S1** | Vector paths · ruled tables from rectangles · `TableCellPosition` / `CellSlot` · locator cross-check | **done** |
| **S2** | Unruled tables: alignment / whitespace dual-mode | not started |
| **S3** | Tagged-PDF consumption; the already-captured `mcid` put to use | not started |
| **S4** | Forms and annotations as typed, distinguishable nodes | not started |
| **S5** | Multi-column reading order, with a stable versioned rule | not started |
| **S6** | Images, DPI screenshots, hidden / off-page findings, annotated PDF | not started |
| **S7** | Labelled-set harness; the > 0.489 gate; v1 declared done | not started |

**Slices are `v1-S*`, not M-numbers.** The milestone chain ended at M7 with v0. Numbering later
work `M8` would imply v0's acceptance list continued into it, and it did not.

## 5. The ruled/unruled split, and why S1 stops where it does

`06-STEAL-REFUSE.md` P12 describes a dual-mode detector: ruled tables from ruling lines, unruled
from alignment. S1 implements **the ruled half only**.

The split is not arbitrary sequencing. A ruling line is *evidence in the document* — the author
drew it. An alignment cluster is *an inference about the document* — the author drew nothing, and
the detector decides. Those two deserve different derivation classes, different tolerances and
different tests, and building the second before the first has shipped is how the first inherits the
second's guesswork.

**A measurement worth recording, because it shaped S1.** The Ethos conformance fixture named
`synthetic/table-regular-grid` contains **no path operators at all** — its 3×2 grid is laid out by
text position alone (six `Tm`/`Tj` pairs, zero `re`). Nothing in the conformance corpus draws a
rule. So the ruled detector cannot be demonstrated on that corpus, and S1 authors an engine-owned
CC0 fixture that does draw one. `table-regular-grid` is an **S2** fixture wearing an S1 name, and
until S2 it correctly reports "looked, found none".

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
| `structural_locators` | false | true | S3 |
| `multi_column_reading_order` | false | true | S5 |
| `char_offsets` | false | true | whichever slice makes elements coarser than spans |

**A capability flips when the proof exists, not when the code lands.** `tables: true` means *this
profile looked for tables*, which is why an empty array is a real answer and an absent key is a
different one. It does not mean every table is found — S1's narrower limitation says so in as many
words, and S2 removes it.

---

## PR review checklist

- [ ] The change belongs to a numbered slice in §4, or it is out of scope
- [ ] No item from §2 has crept in
- [ ] No accuracy number is claimed that a committed harness does not produce (§3)
- [ ] Fabrication stayed 0; nothing was traded for a better-looking result
- [ ] A new capability ⇒ a proof test ⇒ a new profile hash
- [ ] A new limitation is declared in the artifact, not only in a doc comment
