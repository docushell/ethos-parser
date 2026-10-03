# 30 — Form XObject scope: text a page draws through a form is text the page draws

**Status: accepted and built.** Written 2026-10-03 against `main` at `0325cd3`, workspace
0.64.0-dev.1. The owner took §9's four answers as proposed the same day ("start and implement as
per your recommendations"), recorded as North Star decision #37, and S0–S3 were built on it. §13
holds the results. Where the build departs from the text below, the paragraph says **Amended** and
why.

Source: [`measurements/form-xobjects/`](measurements/form-xobjects/README.md), which measured the
case on 2026-09-22, found it split, and named in its §6 what would reopen it. **Every number below
was measured by this session** with
[`measurements/form-xobjects/parsebench_forms.py`](measurements/form-xobjects/parsebench_forms.py)
over ParseBench (`run-llama/ParseBench` at `afb36bd`, dataset `llamaindex/ParseBench`, both
Apache-2.0).

---

## 1. The one sentence

**A `Do` naming a `/Form` runs a content stream the page draws**, so the text that stream shows is
page text — read with the form's own fonts under the form's own matrix, addressed the way every
other run is, and counted where a form cannot be read.

## 2. Why this is reopened

`measurements/form-xobjects/README.md` §6 set three conditions, any one sufficient. Two are now met.

- **A redistributable corpus of the blank-artifact shape.** ParseBench is Apache-2.0 and pinned by
  commit and dataset revision. It holds the shape §4 there could count and not measure.
- **Documents of that shape, obtained and pinned.** Eight in ParseBench's text set (§3), pinned by
  the dataset's revision. **Amended:** none is committed here; §11's tests build the shape instead.

The third, a named caller, is not claimed.

That measurement also said the broad case was refuted: 1,464 bytes behind forms across 208
documents. §3 does not repeat that result on this corpus, and the difference is the corpus: charts
and slide exports draw their labels through forms, and the 208 held few of either.

## 3. The measurement

The script walks every page's `/Resources /XObject`, descends nested forms once each, and counts
text-showing operators (`Tj`, `TJ`, `'`, `"`) in each form's decoded stream. It counts operators,
not characters: characters need the decoder this scope proposes.

| | Text set | Whole corpus |
| --- | ---: | ---: |
| Documents scanned | 506 | all categories, 2,078 pages |
| Documents with text inside a form | 12 | 533 |
| Of those, pages whose own stream shows no text | 8 | 9 |
| Forms holding text | 45 | 1,903 |
| Text-showing operators inside forms | 1,346 | 33,596 |
| Deepest nesting | 3 | 3 |
| Forms drawn again after their first `Do` | 0 | 1,138 |
| Forms with a `/Matrix` that is not the identity | 0 | 123 |
| Forms holding marked content | 0 | 225 |
| Forms holding an `/MCID` | 0 | 8 |
| Forms with `/StructParents` | 0 | 1 |
| Forms with no `/Resources` of their own | 0 | 1 |
| Tagged documents among them | 1 | 1 |

Three readings, and one caveat.

- **The blank-artifact shape is real and measurable here.** Eight text documents show no text in
  their pages' own streams, so their artifacts carry no text run. Seven score 0.00 on ParseBench's content-faithfulness rules where PyMuPDF's text
  scores 0.92 to 0.97 (`text_misc__marks` is the eighth, at 0.20).
- **The broad case is not refuted on this corpus.** 533 documents lose some text.
- **The hard parts are rare.** Every form but one carries its own `/Resources`. Marked content
  inside a form is 225 of 4,573 forms, an `/MCID` 8, `/StructParents` 1.
- **Caveat: the operator counts are not characters**, and a form drawn twice is counted once.

What it is worth on the one score available: read as well as PyMuPDF reads them, the seven empty
documents are 1.3 points of content faithfulness on the 506 (0.6478 today). The partial ones were
not estimated.

## 4. What is read

- **A form's content stream, where a `Do` draws it**, in the order the page draws it: the form's
  runs take their place between the page runs before and after the `Do`.
- **Under the form's own `/Resources`.** Fonts and nested XObjects resolve there. A form with no
  `/Resources` resolves in the page's, which PDF 32000-1 §7.8.3 permits and marks obsolete.
- **Under `/Matrix`.** The form's matrix is concatenated to the CTM for the duration of the form,
  inside a saved graphics state, so a run's origin is in the page's declared coordinate system
  like any other.
- **Each time it is drawn.** A form drawn twice shows its text twice, at two places. That is what
  the page draws; 1,138 such draws exist in the corpus.
- **Nested forms**, to a fixed depth (§7).
- **Images inside a form**, since the same pass places them and `image-payload-not-embedded`
  already says they are not seen there. They become image nodes as a page's own do.

`/BBox` clips. It is **not applied**, as no clip path is applied to a page's own text. **Amended:**
a run drawn outside a form's box is not counted either, for the same reason — nothing counts text a
page clips away, so counting it for forms alone would say more about forms than about clipping.

**Not its rectangles and lines.** **Amended** on measurement (§8): they are not offered to the table
rules. Its text runs and images are the page's.

## 5. How, in this code

`content::Interpreter` holds no `Document`, by design: it gets fonts and XObject names, and
extraction sorts `/Image` from `/Form` (`content.rs`, `draw_xobject`). That stays.

- **Extraction resolves a form the first time the page draws it** (`form_xobjects::PageForms`):
  its decoded operations, its fonts (through the existing font cache), its matrix and its own
  XObject names, cached by object id for the rest of the page. **Amended** from "before the
  interpreter runs, once per document": a form a page lists and never draws then costs nothing and
  refuses nothing, and producers that share one resource dictionary across every page would
  otherwise decode every form once per page.
- **The interpreter gains one case in `draw_xobject`**: a name that resolves to a form saves the
  graphics state, concatenates the matrix, swaps the font and XObject tables, runs the operations
  through the same `dispatch`, and restores.
- **Nothing downstream changes shape.** A run from a form is a `ShownText` like any other. Its
  `PdfLocator` is `{page, origin_x, origin_y, advance}`, which names no stream, so it needs no new
  field.
- **Everything the page's operators read is restored after a form**: the text state, the open
  marked-content sequences, the path being built, whether text was drawn since the pen was placed —
  and which run a `TJ` gap is written onto. **Amended** after bar 2 failed on two ParseBench pages:
  a form whose glyphs this profile could not name lent six gap spaces to the page's last run, so each
  stream now writes gaps only onto its own runs.

Everything the page's own stream is held to applies to a form's: the decoded-bytes ceiling, the
operation ceiling, the tokeniser refusals, the font refusals.

## 6. Where it goes on the wire, and what moves

- **No schema change.** No new node kind, locator field or artifact type.
- **`form-xobjects-not-descended` keeps its code and changes its count**: it counts the forms this
  reader did not enter (§7), where today it counts every form drawn. On most documents it
  disappears.
- **`form-xobject-text-not-descended`**, the profile-scoped twin, is reworded to say what is still
  not entered.
- **A rule id moves, because a document's text moves.** `page-observations-v2` names "which `Do`
  calls become nodes — `/Subtype /Image` only, never `/Form`" as part of itself, so it is the one
  that goes to `-v3`. `profile_sha256` moves with it.
- **A run drawn inside a form names its font by the resource path that reaches it**, `Xf1/F1`:
  `/F1` in the page's resources is a different entry, and may be a different font.
- **One new document-scoped code**, `form-xobject-mcids-not-bound`, counts §7's `/MCID`s.
- **`classify` counts what a page draws through a form.** **Amended:** this scope named only
  `extract`, and left alone `classify` would call a page whose whole content is one form `no-text`
  while `extract` read its text. Text operators count each time a form is drawn; a form's images
  count once, as a page's resources do.
- **`CAPABILITY.md`'s "Text drawn inside a `/Form` XObject" row** moves from Cannot to Can, with
  §3's numbers and §7's remainder.

## 7. Refusals and counted absences

| Case | What happens | Why |
| --- | --- | --- |
| A form that draws itself, directly or through another | The inner `Do` is not entered and is counted | A cycle has no finite reading |
| Nesting deeper than 8 | The `Do` past the limit is not entered and is counted | The corpus's deepest is 3; a bound must exist |
| A form whose stream does not decode, or passes the decoded-bytes ceiling | The page is refused, as for the page's own stream | A form is page content; a page read without it is a read nobody made |
| A page whose forms push it past the operation ceiling | Refused under the existing ceiling | The cost is the page's whatever stream holds it |
| A font inside a form this profile cannot decode | As on a page: its runs are dropped and counted, or the document refused where the font is a predefined CMap | One rule for fonts |
| Marked content inside a form | Read for `/Artifact` and `/ActualText` counting as on a page; **an `/MCID` inside a form binds nothing** and is counted | PDF 32000-1 §14.7.4 resolves it through the form's `/StructParents`, which this reader does not open. 8 forms in the corpus |
| A `Do` naming a form inside a marked-content sequence of the page | The form's runs take the page sequence's `/MCID` | §14.7.4: the form is the sequence's content |

## 8. What it does to `tag`, `overlay`, `ground` and the table rules

- **`tag` refuses a document with a run drawn through a form.** The writer wraps a page's text
  operators in marked-content sequences inside the page's own stream; a run inside a form has no
  operator there to wrap, and wrapping the `Do` would bind every run of the form to one element.
  Refused by name, as a document that already carries a tree is. Checked against the previous
  build: it tagged six such documents among the 271 gate, engine and opendataloader-bench ones —
  the form fixture and five benchmark pages — writing a tree that reached only the page's own text.
- **`overlay` and `ground` need nothing.** Both read runs and boxes from the representation.
- **The table rules see no form ink.** **Amended** on measurement. As proposed, the rectangles and
  lines a form paints reached the table rules, and the ruled rule builds one lattice from every
  rectangle on a page: a header tab drawn by a form broke the 13 x 4 and 10 x 4 tables two
  opendataloader-bench pages draw themselves (`01030000000082`, `01030000000084`), TEDS 0.1728 ->
  0.1343, while ParseBench's table score was identical with and without form ink. So a form's
  rectangles and lines are dropped when it returns, and a table whose rules a form draws is not
  detected from them — said in `form-xobject-text-not-descended`.

## 9. What the owner decided

All four as proposed, 2026-10-03 (decision #37):

1. **§2 reopens it.**
2. **A form that does not decode refuses the page** (§7, row 3), as the page's own stream would.
3. **`tag` refuses such documents** (§8).
4. **`page-observations-v3`** is the rule id that moves.

## 10. The acceptance bar, set before the code

1. **The eight blank ParseBench documents emit runs**, and each of the seven scores within 0.10 of
   PyMuPDF's text on content faithfulness.
2. **No document that reads today loses a run.** Over the 208 gate and opendataloader-bench
   documents and ParseBench's 2,078 pages, every run in today's artifact is in the new one, with
   the same text and origin.
3. **opendataloader-bench NID does not fall** from its value at S1's base commit (0.8849 at
   `f4aef01`).
4. **Fabrication stays 0** on the twelve-document labelled table set.
5. **`form-xobjects-not-descended` counts only what §7 says is not entered**, checked on a fixture
   for each row.
6. **Two runs over one document are byte-identical**, as for every artifact.
7. **Peak memory and time on the 208 are reported**, not bounded in advance: resolving forms once
   per document is the design, and the number says whether it held.

## 11. Slices

| Slice | What | Done when |
| --- | --- | --- |
| **S0** | Fixtures: a page whose whole content is one form; a nested form; a form with a `/Matrix`; a form drawn twice; a form that draws itself; a form with no `/Resources` | Each is in `fixtures/` with its expected runs written by hand. **Amended:** built in memory by the extraction suite's own builder, as its other content-stream tests are, beside the one file fixture (`form-xobject-text-drawn`), whose expectation flips |
| **S1** | Resolve and descend: §5, with §7's cycle and depth rows | S0's fixtures read; bar 2 holds on the gate documents |
| **S2** | Declarations: the limitation counts and wording, `page-observations-v3`, the `tag` refusal | Bars 5 and 6; the gate is green |
| **S3** | Measure: bars 1, 3, 4 and 7, and the table change of §8 | The numbers are in `measurements/form-xobjects/README.md`, and `CAPABILITY.md` moves the row |

## 12. What this scope does not decide

- **Binding an `/MCID` inside a form to the structure tree.** Counted here; a slice of its own if
  the 8 become a requirement.
- **Clipping**, by `/BBox` or by any clip path.
- **Tagging a document whose text is in a form.**
- **Type 3 fonts**, whose glyphs are content streams of their own and a different question.
- **Annotation appearance streams**, which are forms no page `Do` draws.
- **Table evidence a form paints** (§8). Reopens with a document whose table's rules are drawn
  inside a form, or with a ruled rule that does not build one lattice from every rectangle on a page.
- **Reading order where a form's text lands in a page's whitespace.** On `01030000000042` a pie
  chart's labels, drawn by a form, stop the gutter cut dividing the page and its columns interleave:
  NID 0.9319 -> 0.6223, the one large fall in §13.
- **Found and not fixed: the structure reader ignores an MCR's `/Stm`.** A tree item citing an
  `/MCID` inside a form is read as the page's id of the same number. It predates this scope, since a
  form's own ids were never read before; now they are counted and bind nothing (§7), so the two no
  longer meet.

## 13. Results, measured 2026-10-03

Base: `0325cd3`, built in a separate worktree. ParseBench as in §3; opendataloader-bench at its
`7af1d8f` with this repository's adapter.

| Bar | Result |
| --- | --- |
| 1. The eight blank documents emit runs, the seven within 0.10 of PyMuPDF | **Met.** All eight emit runs. The seven: 0.893–0.967 against PyMuPDF's 0.924–0.967, the widest gap 0.031 (`text_misc__reverRo`). `text_misc__marks` reads 0.204, as PyMuPDF does |
| 2. No document that reads today loses a run | **Met** after the gap fix in §5. Over 2,308 documents — 8 gate, 63 engine fixtures, 200 opendataloader-bench, 2,037 ParseBench PDFs — no run lost, no new refusal; 566 documents gain 82,285 runs |
| 3. opendataloader-bench NID does not fall | **Missed by 0.0002.** 0.8849 -> 0.8847: 24 documents rise, 9 fall, and the net is `01030000000042` alone (§12) |
| 4. Fabrication stays 0 on the labelled set | **Met.** 0 geometric, 0 of 15,641 tagged cells; combined recall 503‰, unchanged |
| 5. `form-xobjects-not-descended` counts only §7's cases | **Met** by test: a form drawing itself, and a tenth form nested past eight, each count 1 |
| 6. Two runs are byte-identical | **Met** on the form fixture and two ParseBench documents |
| 7. Time and memory on the 208 | 15.1–15.4 s against 15.5–15.8 s for the base, median RSS 8.1 against 8.0 MiB, peak 3,037 against 3,033 MiB (`nist-sp-800-53Ar5`, the same document) |

**What moved on the benchmarks:** ParseBench content faithfulness 0.6478 -> 0.6612, formatting
0.0929 -> 0.0970, tables unchanged; opendataloader-bench TEDS 0.1728 and MHS 0.3801, unchanged
after §8's amendment.
