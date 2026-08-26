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
| **S5** | Multi-column reading order, versioned rule | S2 | **done** |
| **S6** | Images, DPI screenshots, hidden / off-page findings | S3 | **done** |
| **S6.1** | Code width from the font's declared kind — a text-loss repair | S6 | **done** |
| **S6.2** | A run that draws no ink has no ink box — a fabricated-geometry repair | S6 | **done** |
| **S7a** | The labelled set and the harness — measurement only | S1–S6 | **done** |
| **S7b** | Detector calibration against S7a. Seven investigations: six measured and rejected, one shipped (`ruled-rects-v2`) | S7a | **done** |
| **S8** | The parked stroke-ruled rule, defect-fixed and shipped as a third rule | S7b | **done** |
| **S7** | The > 0.489 gate, assessed with its method stated | S7a, S7b, S8 | **measured and MISSED: 70‰ over twelve documents (v2-S19; 64‰ over four) — the chase is parked, the slice is not closed, and decision #18 is written and undecided** |

---

## S0 — v1 scope and slice map

- **Goal:** v1 exists as an ordered list of bounded changes rather than one roadmap line, **before**
  any of it is implemented.

- **In:** `08-V1-SCOPE.md` (what v1 is, what it is not, why 0.489 is S7's problem — since parked,
  see `08-V1-SCOPE.md` §3 — and the ruled/unruled split); this document.

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
        *(true as S1 shipped it; S5 is the slice that deliberately changed both)*
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

- **The rule, as S1 shipped it** (`ruled-rects-v1`, superseded by `-v2` at S7b): lattice from
  clustered rectangle edges within 1.5pt; at least two faces; every face covered; cells are the
  rectangles mapped onto the lattice, with a rectangle covering every face treated as the outer
  border — a rectangle that was *also* accepted as coverage evidence, which is the inconsistency
  `-v2` closed; text assigned by run **origin**,
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

  > **That slice was S8, and this paragraph called it correctly.** The rule that shipped is built
  > on exactly the closed-face coherence named here — every interior column line stroked across
  > the band — and the 1040 is held at 0 tables by a second precondition about widget rectangles.
  > `stroke-ruled-tables-not-detected` is retired; `undrawn-table-edges-not-supplied` replaces it.

- **The blanket limitation is gone, not reworded.** `unruled-tables-not-detected` said alignment is
  never inspected. That stopped being true, and a limitation that outlives the gap it describes is
  worse than none, because a reader acts on it. What replaced it is narrower on both sides: the
  profile-scoped `stroke-ruled-tables-not-detected` (itself retired the same way at S8, replaced by
  `undrawn-table-edges-not-supplied`), and a **conditional** document-scoped
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

     **S5 answered half of that and left the other half named.** It replaced the rule — with a
     *geometric* one, `gutter-columns-v1`, which reads whitespace and never the tag tree — so
     `two-columns` now reads column-major. The guard test stays green because it was never about
     whether reading order would change; it was about whether *this module* would be the thing
     that changed it. Structure-order reading is still a separate rule with no id and no fixture,
     and it is not scheduled.
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
  - [x] `synthetic/two-columns` reads in column-major order
  - [x] A one-line edit to a fixture does not change its reading order
  - [x] The rule id is in the profile and moves `profile_sha256`

- **Depends on:** S2.

### What S5 settled

1. **A new id, `gutter-columns-v1`, not a bump of `single-column-v1`.** The old string still has a
   true meaning — content-stream order — and a profile that turns the capability off still uses
   it. Bumping in place is the one move that makes two artifacts look comparable while their
   orders disagree. `READING_ORDER_RULE_V0` stays exported and stays spelled the same.

2. **The evidence is whitespace, and only whitespace.** A vertical band no run's horizontal extent
   crosses, wider than a named floor, cuts the page into columns read left to right; inside a
   column the same sweep runs horizontally. Nothing counts lines, runs or characters. The rule
   is named for what it measures, the way `ruled-rects-v1` and `unruled-align-v1` are, rather than
   for the XY-Cut family the recursion belongs to.

3. **The guard is vertical overlap, not a width.** Adjacent bands must share at least half the
   height of the shorter one, and share it strictly. That is what separates two columns from a
   heading above an indented list, which an x-axis sweep alone reads identically. Its cost is
   stated rather than hidden: a two-column page whose first column holds a single line is not
   reordered, because one baseline has no height and that picture is also what a deep indent
   looks like.

4. **Separate constants from S2, even where the number is equal.** The reading-order gutter floor
   and `unruled::COLUMN_GUTTER_MIN` both hold 1 200 and are reasoned from the same fact about
   type. They are two bindings under two rule ids, and a test asserts the reading-order module
   never reads the detector's. A tuning pass on table detection must not silently reorder every
   multi-column document in the corpus.

5. **One order.** The run array **is** the reading order; `ordinal` is its index and the span ids
   are laid over it, so `s1` is the first run a human should read rather than the first the stream
   drew. There is no parallel reading-order index — O4's defect was id order ≠ array order, and
   adding a second sequence would have reproduced it under a new name. `DetectedCell::run_indices`
   are remapped through the permutation, which is the one failure here no artifact would show.

6. **Tables are atoms.** A run inside an accepted `TableRecord.bbox` belongs to one indivisible
   object holding content-stream order, so a cut cannot shred a grid into fake columns of cell
   fragments, and a cell's text still concatenates from the runs the cell names. The table is
   placed among the page's blocks by its own box.

7. **A cut never reorders inside a group it did not cut.** The sweep sorts to *find* the cut and
   the groups are put back into content-stream order before recursing. Skipping that step was a
   real defect during this slice: it leaked the sort, so an uncut block came back ordered by
   baseline — a y-then-x sort of the page reached sideways, which on a real two-column booklet
   turned pages that were already column-major into line-by-line row-major reading. Measured on
   `cfpb-home-loan-toolkit`, fixed, and pinned by
   `a_block_the_rule_declines_to_cut_comes_back_in_stream_order`.

8. **The horizontal cut takes only its widest gap.** Cutting at every gap at once slices a
   two-column region into one block per line, and emitting those top to bottom is row-major
   reading arrived at from the other direction. Taking the widest gap peels off whatever full-width
   thing was hiding the gutter and hands each half back to the vertical cut. Ties cut together, so
   evenly-set body text separates in one step rather than one recursion per line.

9. **S2 × S5: `two-columns` is still not a table.** Column-major is `Left top, Left bottom, Right
   top, Right bottom`; row-major is `Left top, Right top, Left bottom, Right bottom`. They are
   different sequences, the emission is still not row-major, and the alignment rule still refuses
   — with the refusal still declared. A test asserts the row-major sequence is *not* what comes
   out, so "fixing" two-columns by turning it into a 2×2 grid fails the build.

10. **Classify is untouched.** The sorter looks at origins and nothing else; it is not gated on a
    classification, and no `multi-column` layout reason was added. That code stays in
    `thresholds::NOT_DETECTED` — a reading-order rule is not a page-complexity detector, and
    routing extract policy through classify is what S2 refused.

### Leftover, named rather than half-done

**Structure-order reading.** A document whose column structure exists only in its tag tree is not
reordered: the geometric rule finds no gutter and leaves it in content-stream order. Emitting nodes
in `/K` order is a *different* rule over *different* evidence and needs its own id and its own
fixture. `structure.rs` still contains no sort and its guard test still says so — the comment there
naming S5 as the slice that would revisit it is answered by this paragraph, not by the code. Not
scheduled; it is not part of S6 or S7.

---

## S6 — Images, screenshots, hidden and off-page findings

- **Goal:** The rest of the v1 row's observational surface.

- **In:** Images as elements; DPI screenshots; hidden-text and off-page security findings; the
  annotated-PDF output.

- **Out:** Any of it becoming a verdict. A hidden-text finding is an observation.

- **Acceptance tests:**
  - [ ] Screenshots are deterministic at a pinned DPI, and the DPI is a profile field
        — **half met, and the other half is a named leftover.** `raster_dpi` IS a profile field
        and carries `{"mode":"not_emitted"}`; no screenshot is produced, because no renderer
        exists that this project is allowed to depend on. See the leftovers below
  - [x] A hidden-text fixture produces a finding, not a filtered text layer

- **Depends on:** S3.

### What S6 settled

1. **Findings are observations, and the run stays.** `invisible-render-mode` and `off-page` are
   flags on a `TextRun` that is still in the artifact, in reading order, with its text and origin
   intact — plus a document-scoped count so a consumer reading only the assurance block learns
   they exist. Nothing is filtered, nothing is scored. OpenDataLoader deletes low-contrast text
   and returns a page that looks clean; that is the defect O21 names and the one this shape
   exists to refuse.

2. **`Tr` was already tracked and read by nothing.** Measured before writing anything: invisible
   text was never dropped — the state was set at `content.rs` and no code path ever compared it
   to 3. So the defect was **silent mixing**, not data loss, and S6 is additive rather than a
   repair. Half of O21's exit criterion ("the run stays in the representation") was already true;
   what was missing is that anyone could tell.

3. **An image node is a placement and a digest, never a picture and never a caption.** Page,
   object number, the rectangle the `Do` painted into, and a sha256 over the stream **as stored**.
   Encoded rather than decoded, so the fingerprint cannot depend on this engine's inflate; a
   digest rather than a payload, because an artifact is a record about a document and not a second
   copy of it. No description, alt text or characterization — that is OCR or a model's opinion,
   and O20 says neither is evidence.

4. **The painted rect is the matrix, not the pixel count.** A PDF image is defined on the unit
   square and the CTM decides where it lands, so the area is a real measurement of the document's
   own matrix. `/Width` and `/Height` are kept, named `pixel_width`/`pixel_height`, in different
   units and a different field — a 4000×3000 photograph scaled into a 2cm thumbnail is 2cm of
   page, and conflating the two is the pdf-inspector defect in another costume.

5. **A third kind of box, kept apart from the other two.** `GeometryPresence::Measured` means ink
   measured from font metrics. `AnnotationRect` means a rectangle the author declared. `PaintedRect`
   means the page's own matrix applied to the unit square. Three provenances, three types; a
   rotated placement is `NotAxisAligned` rather than a bounding box, because a bounding box claims
   page area the picture does not cover.

6. **`Do` is interpreted for `/Image` and still not descended for `/Form`.** The operator stays
   fail-closed; a `Do` whose *name* does not resolve is counted and declared rather than refused,
   because rejecting the document would turn files that read today into failures. Inline images
   (`BI`/`ID`/`EI`) are counted too — they have no object number and no separate stream, so they
   cannot be nodes, and an uncounted skip would let "no image nodes" read as "no images".

7. **The overlay marks and never edits.** A deterministic lopdf copy with `/Square` annotations
   over tables, image placements and flagged runs, plus a per-page note that counts what has **no**
   rectangle to draw — O10's exit criterion is that absence is visible, not just presence. The
   source document's own annotations are kept and its bytes are untouched. This is not
   `--sanitize`, and a source-scan test bans the operations that would make it one.

8. **Screenshots are not shipped, by decision.** Rendering a page needs a PDF renderer; PDFium is
   admitted only caller-provided under an explicit ADR (`00-NORTH-STAR.md` #14), no AGPL renderer
   clears `deny.toml`'s allowlist, and shelling out to `pdftoppm` would put an unpinned binary
   between the document and the artifact. `raster_dpi` records the not-emitted state on the
   profile anyway, so a renderer arriving later is a `profile_sha256` event rather than a silent
   change of meaning.

9. **A coordinate repair, found while building the off-page rule.** The page transform used the
   box's width and height and **discarded its origin**, so every coordinate on a page whose
   `/MediaBox` does not start at `(0, 0)` was shifted. Not one document in either corpus has such
   a box — measured across all 67 PDFs available here — which is why it survived six slices. It
   had to be fixed before an off-page finding could be honest: a bounds test against a frame the
   content is offset from reports ordinary text as off-page, and a **fabricated** security finding
   is worse than none. `/CropBox` is now read as the visible box, and `/Rotate` inheritance and its
   indirect-reference form are handled for the same reason.

10. **Classify is unchanged, and answers a different question.** It counts image XObjects a page's
    `/Resources` **declare**; extraction emits a node per `Do` that **paints** one. The
    `image-declared-not-drawn` fixture pins both answers at once: `embedded-images` from classify,
    zero image nodes from extract. Neither is wrong.

### Leftovers, named

- **Page rasters** (`page-raster-not-emitted`). See settled point 8. Not scheduled; it needs an
  ADR that this repository has no convention for and a renderer it has no dependency for.
- **Low-contrast text** (`low-contrast-not-detected`). Not possible without new machinery, which
  decision 6 of the slice made the condition: the twelve colour operators are recognised and
  discarded, `/ExtGState` is never resolved so alpha is unreachable, and the graphics state has no
  colour slot. Doing it needs a colour-space model plus a contrast **threshold** — the same shape
  as the vowel-frequency test this project already refuses for `garbled`.
- **Images inside form XObjects.** Not seen, because this profile does not descend into them;
  `form-xobject-text-not-descended` covers the text half and the image half is the same gap.
- **Structure-order reading**, still. Unchanged from v1-S5 and still not scheduled.

---

## S6.1 — Code width comes from the font, not from its decoder

**A repair slice, numbered off S6 rather than given a number of its own**, because it fixes a
defect S6's audit surfaced rather than adding capability. It is a slice at all — instead of a
commit — because it changes decoded text on real documents, which moves `profile_sha256` and makes
every artifact produced before it correctly non-comparable with one produced after.

- **Goal:** A simple font's string is split into single-byte codes, always. And the artifact stops
  attributing the reader's loss to the document.

- **The defect.** `Font::split_codes` took the code width from **whichever decoder the font got**:

  ```rust
  let width = match &self.decoder {
      Decoder::ToUnicode(t) => t.code_bytes().max(1),   // the /ToUnicode codespace
      Decoder::Simple(_)    => 1,
  };
  ```

  `/ToUnicode` wins whenever a document ships one, so a **simple** font declaring a `<0000><FFFF>`
  codespace had its single-byte codes read two at a time. PDF 32000-1 §9.6 is unambiguous: a simple
  font's codes are always one byte, and `/ToUnicode` maps codes to Unicode — it has no say in how a
  string is split. The doc comment one line above already said *"One byte per code for simple
  fonts"*; the code did not do it, because `/Subtype` was parsed and then never reached the
  decision.

- **What it cost, measured rather than estimated.** Font instances whose codes were split wrongly:

  | Document | Simple fonts mis-split | Type0 |
  | --- | --- | --- |
  | `nist-sp-800-53r5` | **2 426** | 41 |
  | `nist-sp-800-63b` | **303** | 28 |
  | `cfpb-home-loan-toolkit` | **98** | 73 |
  | `irs-form-1040-2025` | 0 | 0 |
  | all 9 conformance synthetics | **0** | 0 |

  On `cfpb-home-loan-toolkit`, **8 417 text runs were omitted** from the artifact. What survived was
  visibly damaged: `"You’rtartinoooortgag"` where the page reads *"You're starting to look for a
  mortgage"*. Runs in the document's CID fonts were perfect on the same page, which is the signature
  — the fault tracked the font's kind, not the document.

- **Why six slices passed green over it.** Every fixture in the conformance corpus is a Type1 font
  with **no** `/ToUnicode`, so all nine take the `Decoder::Simple` path and split correctly. The
  shape that breaks appears in zero owned fixtures and in every real document. S6.1 authors the
  fixture that was missing.

- **The dishonesty, which is the worse half.** The lost runs were declared — as
  `broken-font-encoding`, *"A font on this document has an incomplete or damaged encoding."* That is
  a **false statement about a conformant document**: the fonts were fine and the reader was wrong.
  A declaration that misattributes is worse than no declaration, because a reader acts on it — and
  this one would have sent someone to fix a document that had nothing wrong with it.

- **In:** `FontKind` read from `/Subtype` and carried on `Font`; code width decided by the font's
  declared kind; a versioned rule id on the profile; `broken-font-encoding` reworded to describe
  what happened without asserting a cause it cannot establish; the missing fixture.

- **Out:** **Type0 done properly.** Composite fonts still take their width from the `/ToUnicode`
  codespace, because nothing here parses `/Encoding` CMaps at all. That is *correct for Identity-H*,
  which is overwhelmingly what real documents use, and **unverified for anything else** — so it is
  declared as `composite-font-codes-from-tounicode` rather than left to be discovered. Doing it
  properly means parsing CMaps including mixed-width codespaces, which is a slice of its own and
  touches 142 font instances that are not currently broken. Also out: OCR, glyph outlines, the
  predefined CJK CMaps that are already a declared limitation.

- **Acceptance tests:**
  - [x] A simple font with a two-byte `/ToUnicode` codespace decodes its text — the fixture the
        corpus lacks, which fails on the previous build
  - [x] `cfpb-home-loan-toolkit` drops **zero** runs to `broken-font-encoding`
  - [x] Every conformance golden is byte-identical apart from the profile hash: they never took the
        broken path, so a repair that moved them would be a different bug
  - [x] `broken-font-encoding` no longer asserts the document is damaged
  - [x] A Type0 document declares the composite-font interim
  - [x] Oracle still 12 / 3; `two-columns` still column-major; the 1040 still 0 tables

- **Why before S7.** S7's gate is table-cell accuracy measured on real documents. Measuring it
  against a text layer missing 8 417 runs measures the wrong thing — and it would be wrong in the
  flattering direction, since a garbled cell fails to match rather than fabricating a match.

- **Depends on:** S6.

---

## S6.2 — A run that draws no ink has no ink box

**The second repair slice off S6**, and the second one found by asking what the first one's audit
turned up rather than by a test failing.

- **Goal:** Stop putting a rectangle on the wire around content that draws nothing — and let two
  real documents produce an artifact again.

- **The failure.** `nist-sp-800-53r5` and `nist-sp-800-63b` produced **no artifact at all**, exiting
  2 on `DocumentRepresentation::seal`'s `check_box_within_page`. On 53r5 that was **491 of its 492
  pages**. Two of the three real benchmark documents were unreadable, and had been since the check
  was written.

- **What the boxes actually were**, measured across every offender rather than sampled:

  | Document | out-of-page boxes | whitespace-only | with visible text |
  | --- | --- | --- | --- |
  | `nist-sp-800-53r5` | 3 450 | **3 450** | **0** |
  | `nist-sp-800-63b` | 2 | **2** | **0** |

  Every one is a run of spaces at a **one-point** font size, placed past the right edge of the page
  — a producer idiom for trailing whitespace. **No run with visible text is out of place anywhere.**
  The coordinate transform was never wrong; the seal was refusing two documents over rectangles
  drawn around nothing.

- **The root cause.** `Font::ink_box` builds the box from the font's ascent/descent envelope
  stretched over the run's **advance**:

  ```rust
  let top_pt    = baseline_y_pt - (ascent  / GLYPH_SPACE_UNITS) * font_size_pt;
  let bottom_pt = baseline_y_pt - (descent / GLYPH_SPACE_UNITS) * font_size_pt;
  // x spans origin .. origin + advance
  ```

  That is not per-glyph ink, and `FontInk`'s own doc comment always said so — *"Font-level rather
  than per-glyph: a per-glyph ink box needs the glyph outline, which is M-later work."* For a run of
  spaces it is a rectangle around nothing, labelled `Measured`.

- **The contract had already named the gap and left it unfilled.** `GeometryPresence`'s doc explains
  it is deliberately not an `Option` because `None` would collapse *"we could not measure"*,
  ***"there is nothing to measure"***, and *"we were not asked to measure"* into one answer. There
  was a variant for the first (`NotReportedByReader`) and the third (`CapabilityNotEnabled`), and
  **none for the second**. S6.2 adds it.

- **In:** `GeometryAbsence::NoInkToMeasure`; whitespace-only runs and zero-advance runs get it
  instead of a fabricated box; it does **not** count toward the ink-measurement limitation, because
  the reader *could* measure — there was nothing there, which is a different fact from a font that
  supplies no metrics. Plus the fixture the corpus lacks.

- **Out — and deliberately.** `check_box_within_page` **stays a hard refusal**. It is a working bug
  detector and it earned its keep in this very session: it is what caught v1-S6's crop-box
  regression, where page dimensions came from one box while coordinates came from another. Softening
  it into a limitation would have let that ship silently. After this slice a *visible* run outside
  its page means the transform really is wrong, and that is worth failing loudly over.

- **Blast radius**, and the pattern by now familiar: **150 425** nodes across the four real documents
  lose a meaningless box; **zero** conformance fixtures change, because not one of them contains a
  whitespace run that claims a box. Grounding projections shrink by exactly those nodes, which is
  the point — a citation anchored to a rectangle around three spaces was never evidence.

- **Acceptance tests:**
  - [x] Both NIST documents produce an artifact
  - [x] A whitespace run reports `no_ink_to_measure`, not a box and not `not_reported_by_reader`
  - [x] The absence does **not** inflate `geometry-absent-not-groundable`
  - [x] Every conformance golden's geometry is unchanged
  - [x] A run with visible text still gets its box, and still refuses the document if it lands
        outside the page

- **Leftover, named:** **what `Measured` actually means.** The box is a font-envelope approximation
  for *every* run, not just whitespace — a capital `T` and a lowercase `o` get identical box heights.
  Saying so properly needs glyph outlines, and it touches `01-CONTRACT.md`, the meaning of `bbox` in
  `ethos.grounding.v1`, and S1's locator cross-check. That is a slice of its own and it is not
  scheduled. What S6.2 fixes is the case that is not an approximation but a fiction: a box around
  nothing.

  **Unverified, and stated rather than implied:** that a space glyph never draws ink in these fonts
  is reasoned from the semantics of whitespace and from the box being an advance rectangle rather
  than an outline. Confirming it needs the glyph outlines this slice does not read.

- **Depends on:** S6.

---

## S7a — The labelled set, and the harness

**S7 split in two, and this is the measuring half.** The original slice asked for a labelled set, a
harness, and a verdict against 0.489 in one step. Measuring first and changing the detector second
is the order this repository already argues for everywhere else — and it matters more than usual
here, because the alternative is tuning a tolerance until a number looks better, which is exactly
how v1-S1's 662-cell fabrication on `irs-form-1040-2025` got written.

**S7a changes no detector and improves no number.** It builds the instrument, points it at the
corpus, and reports what it sees. The number it reports is bad.

- **Goal:** A committed labelled set, a harness that reruns to the same number, and an honest
  measurement of where table detection actually stands.

- **What the measurement found, before any of it was built.** Across the four real documents:

  | Document | pages | tables found | tagged tables MISSED |
  | --- | --- | --- | --- |
  | `nist-sp-800-53r5` | 492 | **0** | 26 |
  | `nist-sp-800-63b` | 80 | **0** | 13 |
  | `irs-form-1040-2025` | 2 | **0** | 1 |
  | `cfpb-home-loan-toolkit` | 28 | 10 | 8 |

  Table-cell accuracy today is not below 0.489. It is **not measurable as a cell score on three of
  the four documents at all**, because no cell is produced.

  And the cause is one constant. **Every** table-candidate refusal across all four documents is
  `unruled::COLUMN_GUTTER_MIN`, and on 490 of `nist-sp-800-53r5`'s 492 pages the refused gutter is
  **1 062 centipoints — 10.6 pt — against a 12 pt floor**. That floor was sized to reject word
  spacing (*"at 12pt type a space is 3–4pt"*); 10.6 pt is a structural gutter by the rule's own
  reasoning. Acting on that is **S7b's**, deliberately, and only once this harness can prove the
  change is an improvement rather than a preference.

  > **S7b ran that proof, and this paragraph is wrong.** 1 062 cp is the *first* sub-floor gap
  > `gutter_fault`'s `.find()` returns, not the smallest; the smallest is 151 cp on 542 of the 600
  > refused pages. Disabling the floor entirely leaves the corpus **scoring identically**. The
  > paragraph is left standing rather than edited, because what it records is that a plausible
  > reading of a refusal detail survived review and was killed by a measurement — which is the
  > entire reason S7a was built before S7b. See the S7b section below.

- **Where the labels come from, and why they are not mine.** Every one of the four documents carries
  a **tagged structure tree**, and its `/Table` elements are the document author's own declaration
  of where a table is and what shape it has. v1-S3 already reads them — `TaggedTable` carries page,
  rows, columns and per-cell spans, and a guard test keeps geometry out of that module entirely.

  So the ground truth is the author's, the thing measured is the geometric detector, and the two
  derivations are independent **by construction** rather than by anyone's care. That independence is
  what S3 built and it is why this harness needs no hand-labelling and no judgement of mine.

- **The caveat, stated rather than implied.** A tagged `/Table` is a **claim by the document's
  producer**, not verified truth. Producers use table tags for layout as well as for tabular data,
  so some of the 57 are almost certainly not tables anyone would want extracted. That inflates the
  denominator and makes recall read worse than it is. The labelled set therefore records the
  provenance of every label as `pdf-struct-tree` rather than presenting it as fact, and a later
  slice may narrow it by sampling. **What it must never become is labels derived from what the
  detector found** — that measures nothing, because recall against your own output is 1.0 by
  construction.

- **In:** a committed labelled set with declared provenance; a harness computing recall, precision,
  cell accuracy where both grids exist, fabrication rate, and the cross-check disagreement summary;
  determinism asserted by rerunning; the numbers printed so a CI log records them.

- **Out:** any detector change, any tolerance change, any verdict against 0.489. Publishing a
  comparison of any kind — `06-STEAL-REFUSE.md` is explicit that 0.489 is a floor to beat and never
  a claim to publish, and no bake-off table appears in this repository.

- **Acceptance tests:**
  - [x] The labelled set is committed, and every label names where it came from
  - [x] The harness reruns to the same number
  - [x] Fabrication rate is **0**, measured rather than asserted — every emitted cell's text is a
        concatenation of runs the page actually drew
  - [x] Cross-check disagreements are counted across the set
  - [x] No bake-off table anywhere in the repository

- **Why the gate is not assessed here.** 0.489 is a table-cell score from a third-party published
  corpus this repository does not have. A number computed on a different corpus with a different
  evaluator is not comparable to it, and `06-STEAL-REFUSE.md` records exactly what happens when
  people pretend otherwise: two publishers scored the same tool at 0.000 and 0.693 on tables,
  differing only by invocation flags. The gate is assessed at S7, with its method stated, or it is
  not stated.

- **Depends on:** S1–S6.

---

## S7b — Detector calibration, measured

- **Goal:** Make the detector find the tables the documents say are there, with every change proved
  by S7a's harness rather than by inspection.

- **Status: done.** The prescribed alignment calibration was a no-op and was not taken. A different
  defect, in the **ruled** rule, was found and shipped as `ruled-rects-v2`.

- **The evidenced item, and its falsification.** S7a read `unruled::COLUMN_GUTTER_MIN` as refusing
  490 of 492 pages of `nist-sp-800-53r5` at 10.6 pt against a 12 pt floor, and inferred a wrong
  cliff. Measured:

  - `gutter_fault` uses `.find()`, so 1 062 cp is the **first** sub-floor gap in sorted order, not
    the smallest. The **minimum** gap is 151 cp on 542 of the 600 refused pages, and 185 cp at
    worst. Refused pages carry 250–280 sub-floor gaps among 200–280 column lines.
  - Setting the floor to 151 disables the check outright (`fold` guarantees gaps > `ALIGN_TOLERANCE`
    = 150). The corpus scores **identically**: 57 / 10 / 9 / 157‰ / 0 fabricated.
  - Disabling the row floor as well: **still identical.**

  So the constant was not moved. Bumping `unruled-align-v1` to `-v2` would have moved
  `profile_sha256` to buy no behaviour change, and a rule-version event that versions nothing is
  worse than none.

- **What the cliff actually is.** The candidate handed to the alignment rule is **the whole page** —
  `tables::detect` passes every leftover run as one lattice, and `detect_ruled` builds one lattice
  from every rect. With both gutters off, all 600 pages reach the face check with lattices like
  96 × 231 = 22 176 faces from 1 784 runs, refused by `MAX_FACES` and coherence. The floor is not
  wrong; it is unreachable.

- **Segmentation was then built and measured, and it is the third dead end.** `unruled-align-v2` —
  row lines, cells cut by ≥ 12 pt of whitespace between one run's end and the next run's start,
  bands of consecutive rows agreeing on their cell starts — in two variants:

  | Variant | detected | cell-F1 | what broke |
  | --- | --- | --- | --- |
  | baseline | 10 | 43‰ | — |
  | A: columns still folded from origins | 11 | 43‰ | a false table on 1040; 358 of 363 bands died on the gutter floor |
  | B: cell starts become the columns | **141** | **52‰** | see below |

  Variant B emits 1 302 cells against 77 and gets 72 right. It was reverted because
  `unruled-near-miss` becomes a table, `irs-form-1040-2025` goes from 0 to 6 tables with 0 correct
  cells and 37 wrong, and three `unruled` unit tests fail — including the coherence and
  gutter-quantum ones, because making cell starts the column lines routes the gutter floor around
  itself. It removes a fabrication guard, which is forbidden outright, rather than merely scoring
  badly. `fabricated_cells` stayed 0: real text in invented grids, which is the failure that count
  cannot see and the gold negatives can.

  The obstacle is not a tolerance. These producers emit text word by word, so a cell is *n* runs,
  and no per-page geometric rule recovers the author's cell boundaries without inventing them.

- **MCID grouping was then built and measured, and it is the fourth and fifth dead end.** Merge
  runs into units by their `BDC` marked-content id — the producer's own chunking, not the tree —
  before any geometry:

  | Variant | tables | emitted | fabricated | cell-F1 |
  | --- | --- | --- | --- | --- |
  | baseline | 10 | 77 | 0 | 43‰ |
  | merge alone | 10 | 77 | 0 | **43‰**, every number identical, 217/217 tests green |
  | merge across baselines + bands | 121 | 1 194 | **14** | 46‰ |
  | merge within one baseline + bands | 104 | 1 086 | 0 | 45‰ |

  Merging compresses 8.4× and still leaves a 74 × 157 = 11 470-face lattice against a 4 096 cap:
  the candidate is the whole page, and merging changes what is in it rather than how big it is.
  The 14 fabricated cells came from `extract::reorder_page` — its `run_indices` remap holds only
  while a table's runs stay contiguous, and a unit spanning two baselines can be split by
  `gutter-columns-v1`. Baseline-scoped merging fixed that and then failed as the bands did:
  `unruled-near-miss` becomes a table, and a candidate v1 refused as `FacesWithoutText` now
  produces **no declaration at all**, because the band filter removes it before coherence sees it.

  All three gold negatives carry **zero mcids**, so the merge is a provable no-op on them — the
  near-miss break is the bands', and the negatives cannot certify the merge. Separately, an
  adversarial panel established that **778‰ of gold cells cite exactly one mcid**, so an
  mcid-reading detector would reproduce their text by construction; the gate would need a
  published ceiling and a null control before its number could be quoted. See
  `docs/table-gate-v1.md`.

- **What all five repairs missed, and the sixth that worked.** Every table the gate scores is a
  ruled one — **the alignment rule emits zero on the entire corpus**, before and after every change
  tried against it. Five slices went into a rule the gate never exercised.

  Asking the other question found a real defect. `Lattice::build` required every face to be covered
  by *some* painted rectangle; a background panel answers yes for all of them at once, while
  `detect_ruled` separately discarded that panel as "the table's own border". One rectangle cannot
  be both the only evidence a face exists and not a cell. `cfpb-home-loan-toolkit` pages 22 and 23
  paint a 351 × 454 pt panel behind highlight bars and emitted a 17 × 13 table holding 12 cells —
  on a page whose tree declares no table — and a 23 × 8 against a tagged 5 × 3, together supplying
  79 of 91 false positives and both cross-check disagreements.

  `ruled-rects-v2` excludes a lattice-spanning rectangle from being a coherence witness:

  | | `-v1` | `-v2` |
  | --- | --- | --- |
  | detected / matched | 10 / 9 | 8 / 8 |
  | precision | 900‰ | **1000‰** |
  | cross-check disagreements | 2 | **0** |
  | cfpb TP / FP | 24 / 91 | 24 / **12** |
  | **MACRO cell-F1** | 43‰ | **61‰** |

  No true positive lost, no other detection changed. Pinned by the engine-owned fixture
  `background-panel-not-a-grid`, and the ruled rule gained `ruled-table-candidate-refused` — it had
  no way to declare a refusal at all before, which went unnoticed because the precondition almost
  never fired.

- **`stroke-ruled-v1` built, measured in full, and not shipped *at this slice*.** (**S8 shipped
  it** — one precondition changed, and both failures recorded below went away. See S8.) The lead
  below, taken as a complete slice: segments captured as their own evidence, rule-rows by baseline, a row must tile
  end to end, a band is consecutive rows agreeing on the columns, rows are the regions BETWEEN
  rules so no edge is invented, coherence per face, 2 × 2 minimum.

  | | `ruled-rects-v2` | `+ stroke-ruled-v1` |
  | --- | --- | --- |
  | detected / matched | 8 / 8 | 21 / 15 |
  | precision | 1000‰ | 714‰ |
  | cells emitted | 36 | 272 |
  | fabricated | 0 | 0 |
  | cfpb | 24 TP / 12 FP → 246‰ | 41 TP / 189 FP → **210‰** |
  | 1040 | 0 TP → 0‰ | 12 TP / 30 FP → **292‰** |
  | **MACRO** | **61‰** | **125‰** |

  Real capability — 17 cfpb and 12 1040 cells nothing had found, an exact 7 × 2 hit on page 7, NIST
  and every gold negative at zero. **Page 13 is refused** by the rule's own coherence step (one
  baseline rules three cells, not four), so the table the slice existed to find is not among the
  nine it emits. Not shipped because it **regresses cfpb** (156 of its false
  positives are text in no gold cell at all), because **four 1040 canary tests fail** and the
  decision taken was to keep that document at zero tables, and because the macro gain comes
  entirely from the canary document rather than from the improvement. See `docs/table-gate-v1.md`.

- **The second tables on pages 8 and 13, and the lead they uncovered.** Page 8's is layout — a
  2 × 3 checkbox block with scattered underlines. **Page 13's is a real 8 × 4 worksheet the page
  strokes as 32 horizontal rules in a perfect grid** (8 baselines × 4 segments at x = 54, 210, 326,
  442, 558), discarded because a two-point segment is not a rectangle.

  So `stroke-ruled-tables-not-detected` was falsified for NIST and 1040 only, never assessed for
  cfpb — where **all nine missed tagged tables sit on pages that stroke segments, 103 cells, 65% of
  that document's gold**. Finding them would give cfpb 852‰ and macro 213‰ (page 13 alone: 493‰ and
  123‰). Still under the floor, and still the only lead left that moves the number. **It was then
  attempted as a complete pass** — see the bullet above — and not shipped.

- **Pages 16 and 17 investigated: no defect.** The last ruled lead. Page 17 paints two full-height
  column panels (165 × 214, 339 × 214 pt) with unpainted text rows inside; page 16 paints a single
  61 pt two-cell band behind one row of seven. The rule reconstructs what was painted and
  `tagged-vs-geometric-v1` reports the disagreement slot by slot —
  `RowCountDiffers { tagged: 7, detected: 1 }` plus 12 `SlotOnlyInTagged`. The gate charges 4 FP
  and 24 FN and cannot see the cross-check; three of those four FP are text extracted exactly
  right, at a row index a partial detection has no way to know. Crediting row-offset matches would
  give cfpb ~287‰ and is refused as gate-chasing. See `docs/table-gate-v1.md`.

- **The other declared leftover, also falsified.** `stroke-ruled-tables-not-detected` was the
  suspected reason NIST's *ruled* tables are missed. Census of axis-aligned two-point stroked
  segments, all currently rejected, zero diagonals anywhere in the corpus:

  | Document | segments | distinct | what they are |
  | --- | --- | --- | --- |
  | `cfpb-home-loan-toolkit` | 1 380 | 1 375 | real table rulings |
  | `irs-form-1040-2025` | 521 | 520 | the form grid |
  | `nist-sp-800-63b` | 77 | **2** | 76 copies of one margin rule |
  | `nist-sp-800-53r5` | 498 | **9** | 490 copies of one margin rule |

  **Neither NIST document draws table rulings at all.** A stroked-line rule adds nothing there, and
  on `irs-form-1040-2025` it re-opens v1-S1's 662-cell fabrication surface. It stays a limitation.

  > **S8 update: the first half held and the second did not.** Both NIST numbers are exactly as
  > measured here and both documents still score 0‰ under `stroke-ruled-v1`. But the 1040's
  > fabrication surface turned out to be closable on a principle — a face whose four edges are a
  > form field's four edges is that field's box — and `cfpb-home-loan-toolkit`, which this census
  > shows drawing 1 375 distinct real table rulings, was never assessed here at all. That column
  > was the lead. The limitation is now retired; see S8.

- **The standing hazard, honoured.** Loosening a tolerance to admit more tables is the shortest path
  to fabricating them, so the gold negatives are now asserted: `synthetic/two-columns`,
  `synthetic/simple-text` and `unruled-near-miss` still yield **0** geometric tables.

- **Acceptance tests:**
  - [x] Before/after on the four-document corpus, from the same harness, in the CHANGELOG
  - [x] Fabrication still **0** after calibration, `irs-form-1040-2025` included
  - [x] `two-columns` is still not a table, and neither are the other gold negatives
  - [x] `reading_order::COLUMN_GUTTER_MIN` untouched
  - [x] No bake-off table anywhere in the repository

- **Depends on:** S7a.

---

## S8 — Stroke-ruled tables: the parked rule, defect-fixed and shipped

- **Goal:** Take `stroke-ruled-v1` out of the attic, fix the two defects that stopped it shipping
  at S7b, and land it **only** if it does not regress the document it was built for.

- **Status: done, and it shipped.** `cfpb-home-loan-toolkit` cell-F1 **246‰ → 259‰**, macro
  **61‰ → 64‰**, `irs-form-1040-2025` held at **0 geometric tables**, fabrication **0**,
  cross-check disagreements **0**, all three gold negatives still silent. Shipped as **0.10.0**;
  profile `sha256:08c4207d…5143c65`.

- **The one change that made it shippable.** Both of the parked rule's failures were the same
  defect — **`extract` discarded the page's vertical segments before the rule ever saw them**, so
  "where are the columns" was decided entirely by where horizontal rules happen to end. Step 5
  therefore moves off the faces and onto the lines: *every column line interior to the band must be
  stroked as vertical ink across the band's full height*, outer edges exempt. That is the
  precondition `stroke-ruled-tables-not-detected` itself named at S2, and it fixes both symptoms at
  once:

  | | parked `-v1` | shipped |
  | --- | --- | --- |
  | `cfpb` p13, the 8 × 4 worksheet the lead was named for | refused | **7 × 4 emitted** |
  | `cfpb` p22 / p23 / p24-second, Closing Disclosure furniture | 5 bands emitted | **refused** |
  | `cfpb` cell-F1 | 210‰ | **259‰** |

- **And the 1040 stays silent on a principle, not a special case.** Under the new step 5 alone it
  yields five tables: its entry boxes are a stroked grid with their column rules drawn. The second
  precondition is **whose rectangle it is** — a face whose four edges are a form field's four edges
  is that field's box. Measured, a 1040 face is `93.3 … 251.6 × 309 … 321` against a widget at
  `145.0 … 251.2 × 309 … 321`, edge for edge; page 13 — *also* a fillable worksheet with 25 widgets
  — insets its fields well inside larger printed cells. One such face refuses the band. No new
  constant; `LATTICE_TOLERANCE` throughout.

- **It comes back one row short, and says so.** Eight baselines bound seven rows: the worksheet's
  header row has no top edge drawn and step 4 does not supply one. Rather than absorb that, the
  profile limitation `stroke-ruled-tables-not-detected` is **retired and replaced** by
  `undrawn-table-edges-not-supplied`, which states the offset a consumer will meet. Third code to
  hold that position — `unruled-tables-not-detected` (S1) → `stroke-ruled-…` (S2) → this (S8) —
  each removed rather than reworded.

  **This was a decision, not an oversight.** S8's own terms asked for a shape match to the tagged
  8 × 4, and that is unreachable while the standing rule against invented coordinates holds: the
  top edge is not in the file. The two were put side by side and **the decision was taken, and it
  is to ship the 7 × 4 and declare the offset.** A later slice that wants the 8 × 4 is proposing to
  write a coordinate no operator produced, and owes its own evidence for that.

- **What it still gets wrong.** 136 false-positive cell slots against 12, overwhelmingly page 24's
  9 × 4 and page 25's 4 × 2 and 8 × 6. Those bands' interior column lines *are* stroked, so they
  are grids by every reading of the ink; the structure tree simply tags nothing there.

  **Also a decision, and the same shape as the one above.** Removing them needs a threshold fitted
  to these four documents, which §3 forbids, and the honest reading is denominator inflation
  running the other way: a producer who does not tag a table is not evidence that no table is
  there. **The decision was taken, and it is to keep those bands and report the cost** — page
  precision 1000‰ → 928‰, carried in the number rather than tuned out of it. Full account in
  `docs/table-gate-v1.md`.

- **In:** `crates/engine-pdf/src/stroke_ruled.rs`; `PathSegment` capture in `content.rs`;
  `forms::widget_rects`; three-way arbitration in `tables::detect` (ruled → stroke-ruled →
  alignment, author evidence before inference); `codes::STROKE_RULED_TABLE_CANDIDATE_REFUSED` with
  its builder and accumulator; `TableDetection.stroke_ruled` with schema, digest and PUBLIC-API;
  three engine-owned CC0 fixtures.

- **Out:** Any retuning of `unruled-align-v1`. Changing the gate's join. Declaring v1 done. A tag.

- **Acceptance tests:**
  - [x] `cfpb-home-loan-toolkit` cell-F1 does not fall below 246‰ — **259‰**
  - [x] `irs-form-1040-2025` yields **0** geometric tables; all four S7b canaries green
  - [x] All three gold negatives still yield 0 tables
  - [x] `fabricated_cells` **0** and cross-check disagreements **0** on the four real documents
  - [x] Page 13 is emitted. It is a 7 × 4 rather than the tagged 8 × 4, on a decision taken with
        the reason declared (`undrawn-table-edges-not-supplied`) and fixture-pinned
  - [x] Every refusal is declared — no `#[allow(dead_code)]`, clippy green at `-D warnings`
  - [x] The default profile does not declare `stroke-ruled-tables-not-detected` while emitting
        `detection_rule: stroke-ruled-v1`
  - [x] No bake-off table anywhere in the repository

- **Depends on:** S7b.

---

## S7 — The > 0.489 gate, assessed

**Status: assessed, and MISSED. Macro cell-F1 is 70‰ over twelve documents, historically measured
against a 489‰ floor. v1 is not done.** (61‰ when first assessed at S7b; S8 shipped a third
detection rule and re-measured to 64‰ on four documents; **v2-S19 grew the corpus to twelve** and
re-measured to 70‰ with **no detector change**.)

**The band is the number to read, not the macro.** 0‰ .. 590‰, median 0‰, **ten of the twelve
score exactly 0‰** — the detector emits no table at all on them. Two documents supply 849 of the
851 averaged points, and removing one drops the macro to 23‰. So neither 64‰ nor 70‰ was ever a
property of this engine; what twelve documents establish is the **shape**, which is bimodal.
**Decision `00-NORTH-STAR.md` #18, written by v2-S19 and NOT decided, is what closes this slice.**

**The chase is parked, 2026-08-19, and parking is not a pass.** **70‰** is this engine on **twelve
tagged PDFs this repository owns** (v2-S19; 64‰ over the earlier four); **0.489** is a published ODL-local table score on **their**
corpus. Same unit, different exam. Beating it is no longer a shipping precondition for any slice
(`00-NORTH-STAR.md` #10), and it resumes only if this repository has a labelled set it owns and the
owner chooses to resume. **Fabrication 0 still binds. v1 is not complete.** The unticked box below
stays unticked: it records a miss, not a decision.

**v1.1 has since started** — Safe Markdown, scoped in [`10-V11-SCOPE.md`](10-V11-SCOPE.md) and
[`11-V11-MILESTONES.md`](11-V11-MILESTONES.md) — because the owner asked for the next roadmap row.
It adds an output and changes no detector, so **this number is unchanged by it** and this slice
stays open.

- **Goal:** The v1 gate, measured.

- **In:** A committed labelled set and a harness; table-cell accuracy computed and reported with its
  corpus and configuration; fabrication rate measured as 0; the cross-check's diagnostic output
  summarized.

- **Out:** Publishing a competitor comparison. The number is stated with its method or not stated.

- **The number, and where its method lives.** `docs/table-gate-v1.md` — corpus, formula, join rule,
  whitespace rule, engine version, profile hash, and one paragraph on why a score computed here is
  **not** comparable to the published 0.489 it is named after.

  | Document | TP | FP | FN | cell-F1 |
  | --- | --- | --- | --- | --- |
  | `cfpb-home-loan-toolkit.pdf` | 44 | 136 | 115 | 259‰ |
  | `irs-form-1040-2025.pdf` | 0 | 0 | 40 | 0‰ |
  | `nist-sp-800-63b.pdf` | 0 | 0 | 568 | 0‰ |
  | `nist-sp-800-53r5.pdf` | 0 | 0 | 6 937 | 0‰ |
  | **MACRO** | | | | **64‰** |

  Macro-averaged F1 over `CellSlot`s, integer per-mille, exact text after NFC + trim + whitespace
  collapse. **Page-level recall (228‰) is a diagnostic and is not this number** — 228‰ of pages
  agreeing is not 228‰ of cells right.

- **Acceptance tests:**
  - [x] The harness is committed and reruns to the same number
  - [ ] Table-cell accuracy **> 0.489** on the labelled set — **MISSED at 70‰ over twelve
        documents** (64‰ over the four measured before v2-S19), see above. Left unticked on
        purpose: the chase is parked, and a parked chase does not tick a box
  - [x] Fabrication rate **0**, measured rather than asserted
  - [x] Cross-check diagnostics emitted across the set, with disagreement counted — **0**, first
        reached by `ruled-rects-v2` at S7b and **held through S8**, which added a third rule and
        took emitted cells from 36 to 180. A count that stays at zero while five times the cells
        go out is the stronger reading of it
  - [x] No bake-off table anywhere in the repository

- **What is not being done to close it.** Narrowing the labelled set by dropping the documents that
  score badly; relabelling `irs-form-1040-2025` as a gold negative; reading page-level recall as a
  cell score; accepting the four grids a stroke-ruled rule finds on that form. Each would raise the
  number and none would raise the accuracy. **S8 declined the last of those explicitly**: the
  parked rule reached macro 125‰ by turning the canary into four tables, and the shipped one holds
  it at zero and reports 64‰.

- **What is left, and it is not the geometric-alignment family.** Page rasters, low-contrast
  detection and structure-order remain untouched and unmeasured. On the two NIST documents the
  gate still reads 0‰: they draw no table rulings at all — 490 and 76 copies of one margin rule —
  and several of their tagged tables are multi-page, which the page-granular join cannot match even
  in principle.

- **Depends on:** S1–S6, S7a, S7b, S8.

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
