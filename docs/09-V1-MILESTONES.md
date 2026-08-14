# 09 — v1 slices

**Status:** implementation authority for v1 · **Scope document:** `08-V1-SCOPE.md`
**This is the code-review map for v1.** Every v1 PR belongs to exactly one slice.

**Slices, not milestones.** The M-chain ended at M7 with v0 frozen. These are `v1-S0` … `v1-S7`,
and numbering them `M8+` would imply v0's acceptance list continued into them. It did not.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v1 scope + this document | — | **done** |
| **S1** | Vector paths · ruled tables · `CellSlot` · locator cross-check | S0 | **done** |
| **S2** | Unruled tables: alignment / whitespace dual-mode | S1 | **done** |
| **S3** | Tagged-PDF consumption; `mcid` put to use | S1 | **done** |
| **S4** | Forms and annotations as typed nodes | S1 | **done** |
| **S5** | Multi-column reading order, versioned rule | S2 | not started |
| **S6** | Images, DPI screenshots, hidden / off-page findings | S3 | not started |
| **S7** | Labelled-set harness; the > 0.489 gate | S1–S6 | not started |

---

## S0 — v1 scope and slice map

- **Goal:** v1 exists as an ordered list of bounded changes rather than one roadmap line, **before**
  any of it is implemented.

- **In:** `08-V1-SCOPE.md` (what v1 is, what it is not, why 0.489 is S7's problem, the ruled/unruled
  split); this document.

- **Out:** Any code.

- **Acceptance tests:**
  - [x] Both documents exist and name S2–S7 as not started
  - [x] The 0.489 posture is written down where a later slice will read it, not only in a prompt

- **Why first:** so a detector cannot quietly acquire S2's alignment clustering while nobody has
  written down that it is a separate slice.

---

## S1 — Vector paths, ruled tables, `CellSlot`, cross-check

- **Goal:** A table a document actually *drew* is reconstructed from its ruling lines, every cell is
  cross-checked two independent ways, and nothing is invented.

- **In:** Axis-aligned path capture (`re`, axis-aligned `m`/`l`/`h`, painting operators);
  `TableCellPosition { row, column, rowspan, colspan, table_id }`; `CellSlot` occupancy; a
  ruled-grid detector over captured rectangles; the geometric ↔ structural locator cross-check as a
  typed record on the artifact; `capabilities.tables` flipped to `true`; the grounding projection
  filling `Table`/`Cell`; two engine-owned CC0 fixtures.

  **`NodeKind::{Table, TableCell}` were considered and deliberately not added.** Tables ride as
  their own `tables` array on the payload instead. A cell's text is a concatenation of runs that
  are *already* nodes, so emitting cells as nodes too would put the same text in two places — two
  places for it to drift, and two answers to "what does this document say here". The array also
  matches the wire shape the projection fills, so the mapping is a move rather than a translation.
  If a later slice needs cells in the node tree, that is a deliberate addition with its own
  reasoning, not an oversight corrected.

- **Out:** **The unruled half.** No alignment clustering, no whitespace analysis, no XY-Cut. No
  tagged structure, no forms, no multi-column, no images. Bézier curves are not tessellated into
  ruling lines. No accuracy measurement.

- **Derivation classes:** path segments and show-text runs stay **Extracted**. Grid assignment,
  row/column indices and cell-text concatenation are **Computed**, under a rule id pinned in the
  profile. Nothing overwrites Extracted.

- **Acceptance tests:**
  - [x] A ruled fixture emits `capabilities.tables == true` and a non-empty `tables` array with
        `CellSlot`-complete cells, parent ids, zero-based indices, and span 1 meaning not merged
  - [x] **Fabrication 0**: a cell enclosing no text is emitted empty, never filled from nearby.
        A test asserts every cell's text is the concatenation of runs assigned to it and never a
        novel string
  - [x] Cross-check `ok` on the ruled golden; **`mismatch`** on a hostile fixture, with the
        diagnostic present, no silent repair and no panic
  - [x] Looked-but-none: a text-only page carries `tables: []` — key present, not absent, and not a
        fabricated 1×1 table around the page
  - [x] `capabilities.tables: true` has a named proof test; every remaining `false` still declares a
        limitation
  - [x] Grounding: `capabilities.tables == tables.is_some()`; occupancy rules in `check.rs`
        unchanged and still enforced; the oracle partition is still 12 / 3
  - [x] Double-run byte identity on `extract` and `ground` for the ruled golden
  - [x] Flipping `table_detection` moves `profile_sha256`
  - [x] `two-columns` reading order unchanged — still `single-column-v1` plus its limitation
  - [x] `public_api.rs` green; path-capture types are not public unless listed

- **Two findings, both measured rather than assumed:**

  1. `synthetic/table-regular-grid` has **no path operators at all** — its grid is text position
     alone, and no fixture in the Ethos conformance corpus draws a single rule. It is an S2 fixture
     with an S1 name, so S1 authors `fixtures/engine/ruled-table-grid` (3×3, one merged cell, one
     empty cell) as the golden, plus `ruled-table-overlap` as the cross-check hostile.
     `table-regular-grid` correctly reports `tables: []` plus `unruled-tables-not-detected`.
  2. A first version of the detector **fabricated a 662-cell table on `irs-form-1040-2025`**, with
     a cell spanning 75 rows by 45 columns, and Ethos rejected the artifact. Building one lattice
     from every rectangle on a page turns any document that *contains* boxes into a grid. The fix
     is the coherence precondition — **every lattice face must be covered by a rectangle the
     document painted** — plus a cap on lattice size. Overlaps are deliberately not excluded by it:
     an overlap is a real disagreement and belongs in the cross-check, where it is reported, not in
     a precondition, where it would be silently dropped.

- **The rule, as shipped** (`ruled-rects-v1`): lattice from clustered rectangle edges within
  1.5pt; at least two faces; every face covered; cells are the rectangles mapped onto the lattice,
  with a rectangle covering every face treated as the outer border; text assigned by run **origin**,
  never by ink-box intersection. A grid drawn as thin ruling lines rather than cell rectangles does
  not satisfy the coverage precondition and is not detected — that is honest scope, declared by
  `unruled-tables-not-detected`, and it widens at S2.

- **Depends on:** S0.

---

## S2 — Unruled tables

- **Goal:** A table the document implies by alignment rather than by ruling.

- **In:** The second half of `06-STEAL-REFUSE.md` P12: column/row inference from text alignment,
  under its own versioned rule id and its own derivation class. `synthetic/table-regular-grid`
  becomes a real golden. The S1 "ruled detector found nothing" limitation narrows or goes.

- **Out:** Everything S1 excluded that S2 does not name.

- **Acceptance tests:**
  - [x] `table-regular-grid` emits its 3×2 grid with correct text per cell
  - [x] The alignment rule has a version id in the profile, distinct from the ruled one, and a
        document detected both ways records which rule fired
  - [x] A near-miss fixture (columns that nearly align) does **not** produce a table, and says why
  - [x] Fabrication still 0; cross-check still runs on unruled tables

- **The rule, as shipped** (`unruled-align-v1`): column and row lines by folding run origins
  within 150 centipoints; a gutter floor of 1 200 centipoints between column lines and 600 between
  row lines; at least 2 × 2; **every lattice face must contain a run origin**; the runs must arrive
  in row-major face order; a 4 096-face cap. Cells are always span 1, cell boxes are the lattice
  faces so the grid tiles, and the table's box is the origins' extent plus a declared 300-centipoint
  padding — never a font size. Text is assigned by **origin**, exactly as the ruled rule does.

- **Five decisions worth keeping, four of them measured:**

  1. **A separate rule id, not a bump.** `ruled-rects-v1` means *the document drew this grid*;
     `unruled-align-v1` means *a detector inferred it*. Those are claims of very different
     strength, and one id could not tell them apart. `TableRecord.detection_rule` carries the
     answer per table, because a document can hold both kinds and `derivation` is `Computed` for
     both.

  2. **`table_detection` became a structure.** A single string could not distinguish "looked for
     unruled tables and found none" from "never looked". A plain struct with `deny_unknown_fields`
     rather than an internally-tagged enum, for the reason v0.1 measured: internally-tagged
     representations silently drop unknown keys and would re-hash to a different digest than they
     arrived with.

  3. **Coherence is the alignment analogue of S1's coverage precondition.** S1 requires every face
     to be covered by a painted rectangle; S2 requires every face to contain a placed run. Both
     demand the lattice be explained by evidence face by face, and both exist to refuse the same
     thing. Measured on `irs-form-1040-2025`: its text implies **23 276** faces on page 1 from
     1 146 runs and **10 848** on page 2 from 830. It still yields **0 tables**, and now says it
     looked and refused.

  4. **Fold, do not grow.** Growing groups until a gutter appears *chains* — origins each within a
     gutter of the next collapse into one "column" nothing aligns to. Measured: two lines of
     word-split prose came out as a 2 × 3 table. Folding within a tolerance cannot chain past it.
     The cost is that a cell whose text was `Tj`-split a few points wide opens a column instead,
     and is missed; that is the declared price of not fabricating.

  5. **Emission order is evidence, not a threshold.** `synthetic/two-columns` is four runs in a
     flawless 2 × 2 — geometrically identical to a two-row table. What separates them is in the
     file: a table is written across the rows, columns are written down. So a candidate whose runs
     do not arrive in row-major face order is refused. Every alternative discriminator is a number
     tuned until the fixtures fall the right side of it. Reading order is untouched; this reads
     the order, it does not change it.

- **Decision 8 resolved as (b): thin ruling lines are a declared leftover.** A grid stroked as bare
  line segments still fails the ruled rule's coverage precondition, and
  `stroke-ruled-tables-not-detected` names that. Measured before deciding:
  `irs-form-1040-2025` carries **520 axis-aligned stroked segments** alongside the 396 rectangles
  that produced S1's 662-cell fabrication. Admitting 520 more edges to that lattice is the same
  experiment with more input, so a stroked-line rule needs its own closed-face coherence — every
  face bounded by four edges — and its own measurement pass against that form. That is a slice of
  work, not a widening of this one.

- **The blanket limitation is gone, not reworded.** `unruled-tables-not-detected` said alignment is
  never inspected. That stopped being true, and a limitation that outlives the gap it describes is
  worse than none, because a reader acts on it. What replaced it is narrower on both sides: the
  profile-scoped `stroke-ruled-tables-not-detected`, and a **conditional** document-scoped
  `unruled-table-candidate-refused` that appears only where a candidate was actually built and
  refused. A page below 2 × 2 never had a candidate and declares nothing — otherwise the near-miss
  disclosure would ride on every document in existence and carry no information.

- **Depends on:** S1.

---

## S3 — Tagged-PDF consumption

- **Goal:** Use the structure tree the document already carries, and the `mcid` v0 already captures
  but does not act on.

- **In:** Structure-tree traversal, role mapping, `StructuralLocator` widened past `mcid`,
  `capabilities.structural_locators` flipped to `true` with its proof test.

- **Out:** Inventing structure where a document is untagged. An absent `mcid` is still not evidence
  the document is untagged.

- **Acceptance tests:**
  - [x] A tagged fixture yields role paths; an untagged one yields the declared absence, unchanged
  - [x] `structural_locators: true` has a proof test and its limitation is removed
  - [x] Tagged and geometric derivations of the same table are cross-checked against each other

- **The rule, as shipped** (`struct-tree-v1`): read the catalog's `/StructTreeRoot`; recurse `/K`
  over arrays, references, structure elements, `/MCR` marked-content references and bare mcid
  integers; inherit `/Pg` down the tree and let an `/MCR`'s own `/Pg` win over it; apply `/RoleMap`
  where the document supplies one; bind a run **only** on exact `(page object, mcid)` equality.
  Cycles and nesting past 64 levels are refused by name. Recognised for the table check, after
  `/RoleMap`: `Table`, `TR`, `TD`, `TH`, with `/RowSpan` and `/ColSpan` read from `/A` attribute
  dictionaries or from the element, defaulting to 1 and never 0.

- **Four locator states, because they are four different facts.** `StructuralLocator` gained two
  variants rather than one:

  | State | What happened |
  | --- | --- |
  | `pdf_tagged` | the tree cites this `(page, mcid)` — the author placed this text here |
  | `pdf_mcid` | the stream gave an id and **no structure element claims it** |
  | `pdf_artifact` | the page marked this as furniture, deliberately outside the tree |
  | absent | the page marked nothing here |

  Collapsing any two loses something real. `pdf_artifact` runs stay in `nodes`, flagged — a reader
  that deletes running heads has silently edited the document (checklist O21/O22), and the edit is
  undetectable downstream.

- **Decision 7 resolved as diagnostic-only.** A `/Table` in the tree is compared against a table a
  detector found on the same page and the result rides on `TableRecord.tagged_check` under a
  **new** id, `tagged-vs-geometric-v1`. It is not `geometric-vs-structural-v1` widened: that one
  compares a table's own indices against its own boxes, and one id meaning both would leave a
  reader unable to tell which pair of derivations disagreed. No third `tagged-struct-v1` rule was
  needed — a tagged table's cells are already addressable through the role paths on its runs, so
  emitting a table whose cells this engine positioned would add reach nothing lacked. Where the
  tree describes a table and no detector found one, `tagged-table-without-geometric-table` says so
  and **no table is emitted**.

- **Three things this slice deliberately does not do:**

  1. **No reordering.** The walk produces a lookup keyed by `(page, mcid)`; the node list stays in
     content-stream order. Emitting nodes in `/K` order is a reading-order rule and belongs to S5,
     and a guard test asserts this module contains no sort. `synthetic/two-columns` still reads
     `single-column-v1`, in the same order as before.
  2. **No new `NodeKind`s.** A role path on the existing `TextRun` carries the answer, so
     `Paragraph`/`Heading`/`TableCell` variants would put the same fact in two places — the same
     reasoning S1 used when it refused to emit cells as nodes.
  3. **No using tags to fix a detector.** The 1040 still yields 0 tables and the unruled near miss
     is still a near miss. A tagged grid does not rescue an untagged one, and an alignment lattice
     is not nudged into existence because a tree mentions a table elsewhere.

- **Absence is named on both sides, and never invented on either.** Four conditional
  document-scoped limitations, each present only where it is true:
  `untagged-structure-tree-absent` (no tree at all), `structure-mcid-unbound` (marked content the
  tree does not claim), `structure-item-without-content` (the tree cites content no run carried),
  and `mcid-property-list-by-name` (a `BDC` whose property list indirects through `/Properties`,
  which this profile does not resolve — an *unread* id is not an *absent* one).

- **Depends on:** S1.

---

## S4 — Forms and annotations

- **Goal:** Widgets and annotations as typed, distinguishable nodes.

- **In:** Annotation and form-field nodes, each addressable and each distinguishable from page text.

- **Out:** **Annotation text is never page text.** LiteParse flattens widgets into the text layer,
  which is an undeclared document mutation (checklist L13); the whole point of this slice is not
  doing that.

- **Acceptance tests:**
  - [x] A form fixture's field values are nodes of their own kind, absent from page text
  - [x] A test asserts no annotation string appears in any text run

- **Measured before writing anything.** Extraction reads `get_page_content(page)` and nothing
  else, so annotation and widget strings were **never** reaching `TextRun` — the LiteParse defect
  did not exist here and this slice is purely additive. Worth recording because the opposite
  finding would have made S4 a repair rather than a feature.

- **The rule, as shipped** (`form-annotations-v1`): each page's annotation list is read in the
  order the document wrote it; a widget resolves **up** its parent chain (bounded at 16, cycles
  declared) gathering the inheritable name, type, value and flags; everything else becomes an
  annotation carrying its subtype, `/NM`, `/T` and flags. Flag bits this profile has no name for
  are **kept** as raw bit positions, because a flag nobody named is still something the document
  said.

- **Walked from the page, not from the form**, and that is not a detail. A field dictionary names
  no page; its *widget* does, by sitting in that page's annotation list. Walking from the page
  gives every node a real parent, produces one node per widget rather than a field plus a clone,
  and makes an orphan detectable as a field no page walk reached — three things that would
  otherwise need separate machinery.

- **Two node kinds, and the rule that permitted them.** S1 refused `TableCell` because a cell's
  text is already a run; S3 refused `Paragraph` because the role path already says `P`. The
  standing rule from both is *do not add a kind for a fact an existing node already carries* —
  and a field's value and an annotation's comment are carried by nothing, because no content
  stream draws them. Without a kind of their own they are simply absent from the record.

- **A new locator variant, not a fabricated origin.** An annotation has no baseline, no advance
  and no character origin. `NativeLocator::PdfObject` carries page, object number and the
  declared `/Rect`; `AnnotationRect` is deliberately **not** `GeometryPresence`, because that
  type means measured ink and a `/Rect` is a number the author wrote. A missing or unusable rect
  is typed-absent, never a page-sized box.

- **What is declared rather than repaired or dropped:** an unresolvable parent chain
  (`form-field-parent-unresolved`, with the source bytes provably untouched); a dynamic-form
  packet (`xfa-forms-not-extracted`, static siblings still read); and nodes the grounding schema
  cannot express (`non-text-nodes-not-projected`, counted **separately** from
  `geometry-absent-not-groundable` because "no ink could be measured" and "not text at all" are
  different facts). A hidden annotation is flagged and **kept** — honouring a rendering
  instruction by deleting content is an undeclared edit (checklist O21).

- **Decision 7 shipped: `/OBJR` binds.** S3 walked object references and bound nothing because
  no node existed to bind them to. A form field or annotation the structure tree cites now
  carries the role path the tree gives it. No role is invented where the tree is silent, and
  nothing is reordered.

- **`irs-form-1040-2025` measured**: 199 widgets, 126 `Tx` + 73 `Btn`, **0 tables still**. The
  widgets cannot become an alignment lattice because `unruled-align-v1` clusters *run* origins
  and a field is not a run — structural exclusion rather than a threshold that happens to reject
  them. Its 126 text fields report `Absent`, not `""`: a blank form is not a form filled in with
  nothing. It also carries an XFA packet, now declared.

- **Depends on:** S1.

---

## S5 — Multi-column reading order

- **Goal:** Replace the declared limitation with a rule.

- **In:** A stable, versioned reading-order rule; `reading_order_rule` moves off `single-column-v1`;
  `capabilities.multi_column_reading_order` flipped with its proof test.

- **Out:** A cliff-shaped heuristic. pdf-inspector flips on `min_lines < 15`, so a one-line edit
  reorders a whole page (`03-V0-SCOPE.md` §3.2). **Not that rule, and not a variant of it.**

- **Acceptance tests:**
  - [ ] `synthetic/two-columns` reads in column-major order
  - [ ] A one-line edit to a fixture does not change its reading order
  - [ ] The rule id is in the profile and moves `profile_sha256`

- **Depends on:** S2.

---

## S6 — Images, screenshots, hidden and off-page findings

- **Goal:** The rest of the v1 row's observational surface.

- **In:** Images as elements; DPI screenshots; hidden-text and off-page security findings; the
  annotated-PDF output.

- **Out:** Any of it becoming a verdict. A hidden-text finding is an observation.

- **Acceptance tests:**
  - [ ] Screenshots are deterministic at a pinned DPI, and the DPI is a profile field
  - [ ] A hidden-text fixture produces a finding, not a filtered text layer

- **Depends on:** S3.

---

## S7 — The labelled set, and the gate

- **Goal:** The v1 gate, measured.

- **In:** A committed labelled set and a harness; table-cell accuracy computed and reported with its
  corpus and configuration; fabrication rate measured as 0; the cross-check's diagnostic output
  summarized.

- **Out:** Publishing a competitor comparison. The number is stated with its method or not stated.

- **Acceptance tests:**
  - [ ] The harness is committed and reruns to the same number
  - [ ] Table-cell accuracy **> 0.489** on the labelled set
  - [ ] Fabrication rate **0**, measured rather than asserted
  - [ ] Cross-check diagnostics emitted across the set, with disagreement counted
  - [ ] No bake-off table anywhere in the repository

- **Depends on:** S1–S6.

---

## Standing rules for every v1 slice

Repeated from `08-V1-SCOPE.md` §6 because these are the ones deadline pressure reaches for:

1. **No public confidence field.** A cross-check status is typed vocabulary, never a score
2. **No box derived from a font size**
3. **No silent drop and no silent repair** — a grid that does not tile is a diagnostic, not a nudge
4. **No invented coordinate, identifier, fingerprint or pagination** — an empty cell is empty
5. **Fail closed, and distinguishably**
6. **Byte identity is a test**
7. **The Ethos tree is read-only**
8. **No verification** — a table does not acquire a verdict
