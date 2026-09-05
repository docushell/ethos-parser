# 09 — v1 slices

**Implementation authority for v1.** Scope is in [`08-V1-SCOPE.md`](08-V1-SCOPE.md), and every v1 PR
belongs to exactly one slice.

**Slices, not milestones.** The M-chain ended at M7 with v0 frozen. Numbering these `M8+` would imply
v0's acceptance list continued into them. It did not.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | The scope document and this one | — | done |
| **S1** | Vector paths, ruled tables, the cell model, the locator cross-check | S0 | done |
| **S2** | Unruled tables from alignment | S1 | done |
| **S3** | Tagged-PDF structure trees | S1 | done |
| **S4** | Forms and annotations as typed nodes | S1 | done |
| **S5** | Multi-column reading order, versioned rule | S2 | done |
| **S6** | Images, hidden and off-page findings, the overlay | S3 | done |
| **S6.1** | Code width from the font's declared kind — a text-loss repair | S6 | done |
| **S6.2** | A run that draws no ink has no ink box — a fabricated-geometry repair | S6 | done |
| **S7a** | The labelled set and the harness — measurement only | S1–S6 | done |
| **S7b** | Detector calibration. Seven investigations: six rejected, one shipped | S7a | done |
| **S8** | The parked stroke-ruled rule, defect-fixed and shipped | S7b | done |
| **S7** | The accuracy gate, assessed | S7a, S7b, S8 | **measured and MISSED — open** |

---

## S0 — Scope and slice map

**Goal:** v1 exists as an ordered list of bounded changes rather than one roadmap line, **before** any
of it is implemented.

**Why first:** so a detector cannot quietly acquire S2's alignment clustering while nobody has written
down that it is a separate slice.

## S1 — Vector paths, ruled tables, the cell model, the cross-check

**Goal:** a table a document actually *drew* is reconstructed from its ruling lines, every cell is
cross-checked two independent ways, and nothing is invented.

**In:** axis-aligned path capture; the cell-position and occupancy model; a ruled-grid detector over
captured rectangles; the geometric-versus-structural cross-check as a typed record; `tables` flipped
to `true`; two engine-owned fixtures.

**Out:** the unruled half entirely. No alignment clustering, no whitespace analysis, no tagged
structure, no forms, no multi-column, no images. Bézier curves are not tessellated into ruling lines.

**Table and cell node kinds were considered and deliberately not added.** Tables ride as their own
array on the payload. A cell's text is a concatenation of runs that are *already* nodes, so emitting
cells as nodes too would put the same text in two places — two places to drift, and two answers to
"what does this document say here".

**Derivation:** path segments and text runs stay `Extracted`. Grid assignment, indices and cell-text
concatenation are `Computed`, under a rule id pinned in the profile.

**Two findings, both measured rather than assumed:**

1. The conformance fixture named `table-regular-grid` has **no path operators at all**, and no fixture
   in that corpus draws a single rule. It was an S2 fixture wearing an S1 name, so S1 authored its own
   ruled golden plus a hostile fixture for the cross-check.
2. A first version of the detector **fabricated a 662-cell table on a tax form**, with a cell spanning
   75 rows by 45 columns, and the verifier rejected the artifact. **Building one lattice from every
   rectangle on a page turns any document that *contains* boxes into a grid.** The fix is the
   coherence precondition — every lattice face must be covered by a rectangle the document painted —
   plus a cap on lattice size. Overlaps are deliberately *not* excluded by it: an overlap is a real
   disagreement and belongs in the cross-check where it is reported, not in a precondition where it
   would be silently dropped.

**Fabrication 0 is asserted rather than hoped:** every cell's text is the concatenation of runs
assigned to it and never a novel string, and a cell enclosing no text is emitted empty.

**Looked-but-none is a real answer.** A text-only page carries an empty `tables` array — key present,
not absent, and not a fabricated one-cell table around the page.

## S2 — Unruled tables

**Goal:** a table the document implies by alignment rather than by ruling.

**The rule as shipped** (`unruled-align-v1`): column and row lines by folding run origins within a
tolerance; gutter floors between lines; at least 2×2; **every lattice face must contain a run
origin**; the runs must arrive in row-major face order; a face cap. Cells are always span 1, cell
boxes are the lattice faces so the grid tiles, and the table's box is the origins' extent plus a
declared padding — never a font size. Text is assigned by **origin**, exactly as the ruled rule does.

**Five decisions worth keeping, four of them measured:**

1. **A separate rule id, not a version bump.** *The document drew this grid* and *a detector inferred
   it* are claims of very different strength, and one id could not tell them apart. Every table
   carries which rule found it, because a document can hold both kinds.
2. **The detection setting became a structure.** A single string could not distinguish "looked for
   unruled tables and found none" from "never looked". A plain struct denying unknown fields rather
   than an internally-tagged enum, because tagged representations silently drop unknown keys and
   would re-hash to a different digest than they arrived with.
3. **Coherence is the alignment analogue of S1's coverage precondition.** S1 requires every face to be
   covered by a painted rectangle; S2 requires every face to contain a placed run. Both demand the
   lattice be explained by evidence face by face. Measured on the tax form: its text implies **23,276
   faces on page 1** from 1,146 runs and 10,848 on page 2 from 830. It still yields **0 tables**, and
   now says it looked and refused.
4. **Fold, do not grow.** Growing groups until a gutter appears *chains* — origins each within a
   gutter of the next collapse into one "column" nothing aligns to. Measured: two lines of word-split
   prose came out as a 2×3 table. Folding within a tolerance cannot chain past it. The cost is stated:
   a cell whose text was split a few points wide opens a column instead and is missed. **That is the
   declared price of not fabricating.**
5. **Emission order is evidence, not a threshold.** The two-column fixture is four runs in a flawless
   2×2 — geometrically identical to a two-row table. What separates them is in the file: a table is
   written across the rows, columns are written down. So a candidate whose runs do not arrive in
   row-major face order is refused. **Every alternative discriminator is a number tuned until the
   fixtures fall the right side of it.**

**Thin ruling lines became a declared leftover rather than a widening.** Measured first: the tax form
carries **520 axis-aligned stroked segments** alongside the 396 rectangles that produced S1's
fabrication. Admitting 520 more edges to that lattice is the same experiment with more input, so a
stroked-line rule needs its own closed-face coherence and its own measurement pass. That became S8,
and this paragraph called it correctly.

**The blanket limitation was deleted, not reworded.** What replaced it is narrower on both sides: a
profile-scoped code for the gap that remains, and a **conditional** document-scoped one that appears
only where a candidate was actually built and refused. A page below 2×2 never had a candidate and
declares nothing — otherwise the near-miss disclosure would ride on every document in existence and
carry no information.

## S3 — Tagged-PDF structure trees

**Goal:** use the structure tree the document already carries, and the marked-content ids v0 already
captures but does not act on.

**Out:** inventing structure where a document is untagged. **An absent id is still not evidence the
document is untagged.**

**The rule as shipped** (`struct-tree-v1`): read the structure root; recurse over arrays, references,
elements, marked-content references and bare ids; inherit the page down the tree and let a reference's
own page win; apply the role map where supplied; **bind a run only on exact (page, id) equality** —
nothing fuzzy. Cycles and nesting past 64 levels are refused by name.

**Four locator states, because they are four different facts:**

| State | What happened |
| --- | --- |
| `pdf_tagged` | The tree cites this (page, id) — the author placed this text here |
| `pdf_mcid` | The stream gave an id and **no structure element claims it** |
| `pdf_artifact` | The page marked this as furniture, deliberately outside the tree |
| absent | The page marked nothing here |

Collapsing any two loses something real. **Artifact runs stay in the node list, flagged** — a reader
that deletes running heads has silently edited the document, and the edit is undetectable downstream.

**The tagged-versus-geometric comparison is diagnostic only, under its own new id.** It is not the
existing cross-check widened: that one compares a table's own indices against its own boxes, and one
id meaning both would leave a reader unable to tell which pair of derivations disagreed. Where the
tree describes a table and no detector found one, that is said explicitly and **no table is emitted**.

**Three things this slice deliberately does not do:**

1. **No reordering.** The walk produces a lookup; the node list stays in content-stream order.
   Emitting nodes in tag order is a reading-order rule and belongs elsewhere, and a guard test asserts
   this module contains no sort.
2. **No new node kinds.** A role path on the existing run carries the answer, so paragraph and heading
   variants would put the same fact in two places.
3. **No using tags to fix a detector.** The tax form still yields 0 tables and the near miss is still
   a near miss. **A tagged grid does not rescue an untagged one**, and an alignment lattice is not
   nudged into existence because a tree mentions a table elsewhere.

**Absence is named on both sides and invented on neither**, through four conditional limitations, each
present only where it is true: no tree at all; marked content the tree does not claim; the tree citing
content no run carried; and a property list this profile does not resolve — because **an unread id is
not an absent one**.

## S4 — Forms and annotations

**Goal:** widgets and annotations as typed, distinguishable nodes.

**Out: annotation text is never page text.** A surveyed parser flattens widgets into the text layer,
which is an undeclared document mutation, and the whole point of this slice is not doing that.

**Measured before writing anything:** extraction reads page content streams and nothing else, so
annotation and widget strings were **never** reaching a text run. The defect did not exist here and
this slice is purely additive. Worth recording, because the opposite finding would have made S4 a
repair rather than a feature.

**Walked from the page, not from the form**, and that is not a detail. A field dictionary names no
page; its *widget* does, by sitting in that page's annotation list. Walking from the page gives every
node a real parent, produces one node per widget rather than a field plus a clone, and makes an orphan
detectable as a field no page walk reached — three things that would otherwise need separate
machinery.

**Two new node kinds, and the rule that permitted them.** S1 refused a cell kind because a cell's text
is already a run; S3 refused a paragraph kind because the role path already says so. The standing rule
from both is *do not add a kind for a fact an existing node already carries* — and **a field's value
and an annotation's comment are carried by nothing**, because no content stream draws them. Without a
kind of their own they are simply absent from the record.

**A new locator variant, not a fabricated origin.** An annotation has no baseline, no advance and no
character origin, so it carries page, object number and the declared rectangle. That rectangle is
deliberately a different type from measured geometry, because **measured means ink and a declared rect
is a number the author wrote.** A missing or unusable one is typed-absent, never a page-sized box.

**Flag bits this profile has no name for are kept as raw bit positions**, because a flag nobody named
is still something the document said.

**What is declared rather than repaired or dropped:** an unresolvable parent chain, with the source
bytes provably untouched; a dynamic-form packet, with static siblings still read; and nodes the
grounding schema cannot express, counted **separately** from geometry-absent nodes because "no ink
could be measured" and "not text at all" are different facts. **A hidden annotation is flagged and
kept** — honouring a rendering instruction by deleting content is an undeclared edit.

**The tax form measured:** 199 widgets, **0 tables still**. The widgets cannot become an alignment
lattice because that rule clusters *run* origins and a field is not a run — structural exclusion
rather than a threshold that happens to reject them. Its 126 text fields report absent, not empty
string: **a blank form is not a form filled in with nothing.**

## S5 — Multi-column reading order

**Goal:** replace the declared limitation with a rule.

**Out: a cliff-shaped heuristic.** A surveyed parser flips its behaviour at 15 lines, so a one-line
edit reorders a whole page. Not that rule, and not a variant of it.

**What S5 settled:**

1. **A new id, not a bump of the old one.** The old string still has a true meaning — content-stream
   order — and a profile that turns the capability off still uses it. **Bumping in place is the one
   move that makes two artifacts look comparable while their orders disagree.**
2. **The evidence is whitespace, and only whitespace.** A vertical band no run's horizontal extent
   crosses, wider than a named floor, cuts the page into columns; inside a column the same sweep runs
   horizontally. Nothing counts lines, runs or characters. The rule is named for what it measures.
3. **The guard is vertical overlap, not a width.** Adjacent bands must share at least half the height
   of the shorter one. That is what separates two columns from a heading above an indented list, which
   an x-axis sweep alone reads identically. Its cost is stated: a two-column page whose first column
   holds a single line is not reordered, because one baseline has no height and that picture is also
   what a deep indent looks like.
4. **Separate constants from the table rule, even where the number is equal.** Both floors hold the
   same value and are reasoned from the same fact about type, but they are two bindings under two rule
   ids, and a test asserts the reading-order module never reads the detector's. **A tuning pass on
   table detection must not silently reorder every multi-column document in the corpus.**
5. **One order.** The run array *is* the reading order and the span ids are laid over it, so the first
   run is the first a human should read rather than the first the stream drew. There is no parallel
   reading-order index — adding a second sequence reproduces the defect it was meant to fix.
6. **Tables are atoms.** A run inside an accepted table box belongs to one indivisible object, so a
   cut cannot shred a grid into fake columns of cell fragments.
7. **A cut never reorders inside a group it did not cut.** The sweep sorts to *find* the cut, and the
   groups are put back into stream order before recursing. Skipping that was a real defect during this
   slice: it leaked the sort, so an uncut block came back ordered by baseline — which on a real
   two-column booklet turned pages that were already column-major into line-by-line row-major reading.
   Measured, fixed, and pinned by a test.
8. **The horizontal cut takes only its widest gap.** Cutting at every gap at once slices a two-column
   region into one block per line, and emitting those top to bottom is row-major reading arrived at
   from the other direction.
9. **The two-column fixture is still not a table.** Column-major and row-major are different
   sequences, the emission is still not row-major, and the alignment rule still refuses. A test
   asserts the row-major sequence is *not* what comes out, so "fixing" the fixture by turning it into
   a 2×2 grid fails the build.
10. **Classify is untouched.** The sorter looks at origins and nothing else. **A reading-order rule is
    not a page-complexity detector**, and routing extract policy through classify is what S2 refused.

**Leftover, named rather than half-done: structure-order reading.** A document whose column structure
exists only in its tag tree is not reordered. Emitting nodes in tag order is a *different* rule over
*different* evidence and needs its own id and fixture. Not scheduled.

## S6 — Images, findings, and the overlay

**Goal:** the rest of the v1 row's observational surface.

**Out:** any of it becoming a verdict. A hidden-text finding is an observation.

**What S6 settled:**

1. **Findings are observations, and the run stays.** Invisible-render-mode and off-page are flags on a
   run that is still in the artifact, in reading order, with its text and origin intact, plus a
   document-scoped count. Nothing is filtered, nothing is scored. A competitor deletes low-contrast
   text and returns a page that looks clean; **that is the defect this shape exists to refuse.**
2. **The render mode was already tracked and read by nothing.** Measured before writing anything:
   invisible text was never dropped — the state was set and no code path ever compared it. So the
   defect was **silent mixing, not data loss**, and half the exit criterion was already true. What was
   missing is that anyone could tell.
3. **An image node is a placement and a digest, never a picture and never a caption.** Page, object
   number, the rectangle it was painted into, and a hash over the stream **as stored** — encoded
   rather than decoded, so the fingerprint cannot depend on this engine's decompressor. A digest
   rather than a payload, because **an artifact is a record about a document, not a second copy of
   it.** No description or alt text: that is OCR or a model's opinion, and neither is evidence.
4. **The painted rect is the matrix, not the pixel count.** A PDF image is defined on the unit square
   and the transform decides where it lands, so the area is a real measurement of the document's own
   matrix. Pixel dimensions are kept under their own names, in different units and a different field —
   **a 4000×3000 photograph scaled into a 2 cm thumbnail is 2 cm of page.**
5. **A third kind of box, kept apart from the other two.** Measured ink, a rectangle the author
   declared, and the page's own matrix applied to the unit square are three provenances and three
   types. A rotated placement is typed as not-axis-aligned rather than given a bounding box, because a
   bounding box claims page area the picture does not cover.
6. **Image operators are interpreted, form XObjects are still not descended.** A reference whose name
   does not resolve is counted and declared rather than refused, because rejecting the document would
   turn files that read today into failures. Inline images are counted too — they have no object
   number and cannot be nodes, and **an uncounted skip would let "no image nodes" read as "no
   images".**
7. **The overlay marks and never edits.** Annotations over tables, image placements and flagged runs,
   plus a per-page note counting what has **no** rectangle to draw — the criterion is that absence is
   visible, not just presence. The document's own annotations are kept and its bytes untouched, and a
   source-scan test bans the operations that would make this an editor.
8. **Screenshots are not shipped, by decision.** Rendering needs a renderer; none clears the
   dependency policy, and shelling out to an external converter would put an unpinned binary between
   the document and the artifact. The setting records the not-emitted state anyway, so a renderer
   arriving later is a profile-hash event rather than a silent change of meaning.
9. **A coordinate repair, found while building the off-page rule.** The page transform used the box's
   width and height and **discarded its origin**, so every coordinate on a page whose media box does
   not start at the origin was shifted. Not one document in either corpus has such a box — measured
   across all 67 PDFs available — which is why it survived six slices. It had to be fixed before an
   off-page finding could be honest: **a fabricated security finding is worse than none.**
10. **Classify is unchanged and answers a different question.** It counts images a page's resources
    *declare*; extraction emits a node per image a page *paints*. A fixture pins both answers at once,
    and neither is wrong.

**Leftovers, named:** page rasters; low-contrast text, which needs a colour model plus a **threshold**
— the same shape as the detector this project already refuses elsewhere; images inside form XObjects;
and structure-order reading.

## S6.1 — Code width comes from the font, not from its decoder

**A repair slice, numbered off S6** because it fixes a defect S6's audit surfaced rather than adding
capability. It is a slice rather than a commit because it changes decoded text on real documents,
which moves the profile hash and makes every earlier artifact correctly non-comparable.

**The defect.** The code width came from *whichever decoder the font got*, so a **simple** font
declaring a two-byte codespace in its Unicode map had its single-byte codes read two at a time. The
specification is unambiguous: a simple font's codes are always one byte, and the Unicode map has no
say in how a string is split. **The doc comment one line above already said so; the code did not do
it**, because the font subtype was parsed and then never reached the decision.

**What it cost, measured rather than estimated:** 2,426 mis-split simple fonts in one NIST document,
303 in another, 98 in a third. On one real document **8,417 text runs were omitted** from the artifact,
and what survived was visibly damaged — `"You'rtartinoooortgag"` where the page reads *"You're
starting to look for a mortgage"*. Runs in the same page's composite fonts were perfect, which is the
signature: **the fault tracked the font's kind, not the document.**

**Why six slices passed green over it.** Every conformance fixture uses a font with no Unicode map, so
all nine take the correct path. **The shape that breaks appears in zero owned fixtures and in every
real document.** S6.1 authored the fixture that was missing.

**The dishonesty, which is the worse half.** The lost runs were declared — as a *broken font encoding*
on the document. That is a **false statement about a conformant document**: the fonts were fine and
the reader was wrong. **A declaration that misattributes is worse than no declaration, because a
reader acts on it** — this one would have sent someone to fix a document that had nothing wrong with
it. The code was reworded to describe what happened without asserting a cause it cannot establish.

**Out: composite fonts done properly.** They still take their width from the Unicode codespace,
because nothing here parses encoding CMaps. That is *correct* for the encoding real documents
overwhelmingly use and **unverified for anything else**, so it is declared rather than left to be
discovered. Doing it properly means parsing mixed-width codespaces and touches 142 font instances that
are not currently broken.

**Why before S7:** measuring table-cell accuracy against a text layer missing 8,417 runs measures the
wrong thing — and it would be wrong in the *flattering* direction, since a garbled cell fails to match
rather than fabricating a match.

## S6.2 — A run that draws no ink has no ink box

The second repair off S6, and the second found by asking what the first one's audit turned up.

**The failure.** Two real NIST documents produced **no artifact at all**, exiting 2 on the seal's
box-within-page check — on 491 of one document's 492 pages. **Two of the three real benchmark
documents were unreadable, and had been since the check was written.**

**What the boxes actually were**, measured across every offender rather than sampled: 3,450 and 2
out-of-page boxes respectively, and **every single one is whitespace** — a run of spaces at a
one-point font size placed past the right edge, a producer idiom for trailing whitespace. **No run
with visible text is out of place anywhere.** The coordinate transform was never wrong; the seal was
refusing two documents over rectangles drawn around nothing.

**The root cause.** The ink box is built from the font's ascent/descent envelope stretched over the
run's advance. That is not per-glyph ink, and the type's own doc comment always said so. **For a run
of spaces it is a rectangle around nothing, labelled as measured.**

**The contract had already named the gap and left it unfilled.** Typed absence exists precisely so
that *could not measure*, *nothing to measure* and *not asked to measure* are three answers rather
than one. There was a variant for the first and third and **none for the second**. S6.2 adds it, and
it deliberately does not count toward the ink-measurement limitation — the reader *could* measure;
there was nothing there, which is a different fact from a font that supplies no metrics.

**The hard refusal stays.** It is a working bug detector and it earned its keep in this very session:
it is what caught S6's crop-box regression, where page dimensions came from one box while coordinates
came from another. Softening it into a limitation would have let that ship silently. **After this
slice, a *visible* run outside its page means the transform really is wrong, and that is worth failing
loudly over.**

**Blast radius:** 150,425 nodes across the four real documents lose a meaningless box; **zero**
conformance fixtures change. Grounding projections shrink by exactly those nodes, which is the point —
**a citation anchored to a rectangle around three spaces was never evidence.**

**Leftover, named: what "measured" actually means.** The box is a font-envelope approximation for
*every* run, not just whitespace — a capital T and a lowercase o get identical heights. Saying so
properly needs glyph outlines and touches the contract, the meaning of a box in the grounding schema,
and the locator cross-check. Not scheduled. What S6.2 fixes is the case that is not an approximation
but a fiction: **a box around nothing.**

## S7a — The labelled set and the harness

**S7 split in two, and this is the measuring half.** Measuring first and changing the detector second
matters more than usual here, because the alternative is tuning a tolerance until a number looks
better — which is exactly how S1's 662-cell fabrication got written.

**S7a changes no detector and improves no number.** It builds the instrument, points it at the corpus,
and reports what it sees. **The number it reports is bad.**

**What the measurement found:** across the four real documents, three produce **zero tables** and the
fourth produces ten against eight tagged. Table-cell accuracy is not below the comparator — **it is
not measurable as a cell score on three of the four documents at all**, because no cell is produced.

**Where the labels come from, and why they are not anyone's judgement.** Every document carries a
tagged structure tree, and its table elements are the author's own declaration of where a table is and
what shape it has. The engine already reads them, in a module a guard test keeps geometry out of
entirely. **So the ground truth is the author's, the thing measured is the geometric detector, and the
two are independent by construction rather than by anyone's care.**

**The caveat, stated rather than implied.** A tagged table is a **claim by the document's producer**,
not verified truth. Producers use table tags for layout as well as for data, so some labels are almost
certainly not tables anyone would want extracted. That inflates the denominator and makes recall read
worse than it is, so every label records its provenance rather than being presented as fact. **What it
must never become is labels derived from what the detector found** — that measures nothing, because
recall against your own output is 1.0 by construction.

**Out:** any detector change, any tolerance change, any verdict against the comparator, and publishing
a comparison of any kind.

## S7b — Detector calibration, measured

**Goal:** make the detector find the tables the documents say are there, with every change proved by
the harness rather than by inspection.

**The prescribed calibration was a no-op and was not taken.** A different defect, in the **ruled**
rule, was found and shipped.

**The evidenced item, and its falsification.** S7a read a gutter floor as refusing 490 of 492 pages at
10.6 pt against a 12 pt floor and inferred a cliff. Measured: that value is the *first* sub-floor gap
in sorted order, not the smallest — the minimum is 1.51 pt on 542 of the 600 refused pages. Setting
the floor there disables the check outright, and **the corpus scores identically**. Disabling the row
floor as well: still identical. So the constant was not moved; a rule-version event that versions
nothing is worse than none.

**What the cliff actually is.** The candidate handed to the alignment rule is **the whole page**. With
both gutters off, all 600 pages reach the face check with lattices like 96 × 231 = 22,176 faces from
1,784 runs, refused by the cap and by coherence. **The floor is not wrong; it is unreachable.**

**Segmentation, built and measured — the third dead end.** Cutting the page into candidate bands
emitted 141 tables against 10 and got 72 of 1,302 cells right. Reverted, because a gold negative
becomes a table, the tax form goes from 0 to 6 tables with 0 correct cells, and making cell starts the
column lines **routes the gutter floor around itself**. It removes a fabrication guard, which is
forbidden outright, rather than merely scoring badly. The fabricated-cell count stayed 0: **real text
in invented grids, which is the failure that count cannot see and the gold negatives can.**

**Marked-content grouping, built and measured — the fourth and fifth dead ends.** Merging runs into
units by the producer's own chunking, before any geometry: **on its own it changes every number by
nothing** and all tests stay green. Combined with bands it repeats the same failure, and one variant
fabricated 14 cells because a unit spanning two baselines can be split by the reading-order rule.

Separately, an adversarial check established that **778‰ of gold cells cite exactly one marked-content
id**, so a detector reading those ids would reproduce their text by construction. The gate would need
a published ceiling and a null control before such a number could be quoted.

**What all five repairs missed, and the sixth that worked.** Every table the gate scores is a **ruled**
one — **the alignment rule emits zero on the entire corpus**, before and after every change tried
against it. **Five slices went into a rule the gate never exercised.**

Asking the other question found a real defect. The ruled rule required every face to be covered by
*some* painted rectangle — and a page-background panel answers yes for all of them at once, while the
same rule separately discarded that panel as "the table's own border". **One rectangle cannot be both
the only evidence a face exists and not a cell.** Two pages painting a panel behind highlight bars
emitted a 17 × 13 table holding 12 cells, on a page whose own tree declares no table.

`ruled-rects-v2` excludes a lattice-spanning rectangle from being a coherence witness:

| | `-v1` | `-v2` |
| --- | --- | --- |
| detected / matched | 10 / 9 | 8 / 8 |
| precision | 900‰ | **1000‰** |
| cross-check disagreements | 2 | **0** |
| **macro cell-F1** | 43‰ | **61‰** |

**No true positive lost, no other detection changed.** The ruled rule also gained a way to declare a
refusal at all, which it had never had — unnoticed because the precondition almost never fired.

**A stroke-ruled rule was built, measured in full, and not shipped at this slice.** It reached macro
125‰ — but it regressed the very document it was built for, refused the page it existed to find, and
made the tax form yield four tables where every slice since S1 had held it at zero. **The macro gain
came entirely from the canary document rather than from the improvement.** Numbers kept; see S8.

**The census that made the case.** Axis-aligned two-point stroked segments across the corpus: 1,380
in one real booklet (1,375 distinct — real table rulings), 521 in the tax form (the form grid), and 77
and 498 in the two NIST documents of which only **2 and 9 are distinct** — the rest are copies of one
margin rule. **Neither NIST document draws table rulings at all.**

**The standing hazard, honoured.** Loosening a tolerance to admit more tables is the shortest path to
fabricating them, so three gold negatives are now asserted to yield zero tables.

## S8 — The stroke-ruled rule, defect-fixed and shipped

**Goal:** take the parked rule out of the attic, fix the two defects that stopped it shipping, and land
it **only** if it does not regress the document it was built for.

**Shipped.** One document 246‰ → 259‰, macro 61‰ → 64‰, the tax form held at **0** geometric tables,
fabrication **0**, cross-check disagreements **0**, all three gold negatives still silent.

**The one change that made it shippable.** Both of the parked rule's failures were the same defect —
**`extract` discarded the page's vertical segments before the rule ever saw them**, so "where are the
columns" was decided entirely by where horizontal rules happen to end. The coherence step therefore
moved off the faces and onto the lines: *every column line interior to a band must be stroked across
the band's full height*, outer edges exempt. That fixes both symptoms at once — the worksheet the lead
was named for is now emitted, and the furniture that was being emitted is now refused.

**And the tax form stays silent on a principle, not a special case.** Under the new rule alone it
yields five tables, because its entry boxes are a stroked grid. The second precondition is **whose
rectangle it is** — a face whose four edges are a form field's four edges is that field's box.
Measured, a tax-form face and its widget are the same box edge for edge, while the worksheet page —
also fillable, with 25 widgets — insets its fields well inside larger printed cells. **One such face
refuses the band. No new constant.**

**It comes back one row short, and says so.** Eight baselines bound seven rows: the worksheet's header
row has no top edge drawn, and the rule does not supply one. Rather than absorb that, the old
limitation was **retired and replaced** by one that states the offset a consumer will meet — the third
code to hold that position, each removed rather than reworded.

**This was a decision, not an oversight.** A shape match to the tagged 8×4 is unreachable while the
rule against invented coordinates holds: **the top edge is not in the file.** The decision was taken:
ship the 7×4 and declare the offset. A later slice that wants the 8×4 is proposing to write a
coordinate no operator produced, and owes its own evidence for that.

**What it still gets wrong**, and the same shape of decision. 136 false-positive cell slots against 12,
overwhelmingly two pages whose bands' interior column lines *are* stroked — they are grids by every
reading of the ink, and the structure tree simply tags nothing there. Removing them needs a threshold
fitted to these four documents, which the scope forbids, and **a producer who does not tag a table is
not evidence that no table is there.** The decision was to keep those bands and report the cost — page
precision 1000‰ → 928‰, carried in the number rather than tuned out of it.

## S7 — The gate, assessed

**Assessed, and MISSED. Macro cell-F1 is 70‰ over twelve documents, against a 489‰ comparator. v1 is
not done.** It read 61‰ when first assessed, 64‰ after S8 shipped a third rule, and 70‰ when the
corpus grew from four documents to twelve **with no detector change**.

**The band is the number to read, not the macro.** 0‰..590‰, median 0‰, **ten of the twelve score
exactly 0‰** — the detector emits no table at all on them. Two documents supply all 849 of the
averaged points, and removing one drops the macro to 23‰. **So neither 64‰ nor 70‰ was ever a property
of this engine**; what twelve documents establish is the shape, which is bimodal.

**The chase is parked, and parking is not a pass.** 70‰ is this engine on twelve tagged PDFs this
repository owns; 0.489 is a published score on their corpus, **in a different unit — TEDS, not
cell-slot F1** (`table-gate-v1.md` §2; this paragraph said *"same unit"* and was wrong when written).
Beating it is
no longer a shipping precondition for any slice, and it resumes only if this repository has a labelled
set it owns *and* the owner chooses to resume. **Fabrication 0 still binds.**

**Decision #18 closed this slice on 2026-08-30**, and v1 with it. This paragraph read *"v1 is not
complete"* and the line below it called #18 *"written and undecided"*; both were true when written and
neither survived the decision. v1 closes on four capability clauses and a published band — the engine
reads the tables a document declares, detects ruled tables where the producer drew the rules, emits
nothing where neither holds, and fabricates nothing — not on the macro. See
[`00-NORTH-STAR.md`](../00-NORTH-STAR.md) row 18, which is where v1's status is stated.

The method lives in [`table-gate-v1.md`](../table-gate-v1.md): corpus, formula, join rule, whitespace
rule, engine version, profile hash, and why a score computed here is not comparable to the published
number it is named after.

**Page-level recall is a diagnostic and is not this number.** Some fraction of pages agreeing is not
that fraction of cells right.

**What is not being done to close it:** narrowing the labelled set by dropping documents that score
badly; relabelling the tax form as a gold negative; reading page-level recall as a cell score; or
accepting the four grids a stroke-ruled rule finds on that form. **Each would raise the number and
none would raise the accuracy.** S8 declined the last of those explicitly — the parked rule reached
macro 125‰ by turning the canary into four tables, and the shipped one holds it at zero.

**What is left, and it is not the geometric-alignment family.** Page rasters, low-contrast detection
and structure-order remain untouched and unmeasured. On the two NIST documents the gate still reads
0‰: they draw no table rulings at all, and several of their tagged tables are multi-page, which a
page-granular join cannot match even in principle.

---

## Standing rules for every v1 slice

1. **No public confidence field.** A cross-check status is typed vocabulary, never a score.
2. **No box derived from a font size.**
3. **No silent drop and no silent repair** — a grid that does not tile is a diagnostic, not a nudge.
4. **No invented coordinate, identifier, fingerprint or page number** — an empty cell is empty.
5. **Fail closed, and distinguishably.**
6. **Byte identity is a test.**
7. **The Ethos tree is read-only.**
8. **No verification** — a table does not acquire a verdict.
