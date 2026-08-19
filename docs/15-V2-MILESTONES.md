# 15 — v2 slices

**Status:** implementation authority for v2 · **Scope document:** `14-V2-SCOPE.md`
**This is the code-review map for v2.** Every v2 PR belongs to exactly one slice.

**v2 reads four formats.** S0–S5 are **done**; **S6 and S7 have not started**. `engine-office` is
the fifth crate, DOCX is the format that stopped it being speculative, XLSX is the one that made the
page-less invariant carry more than one part, PPTX is the one that tested whether a part this
engine *can* count would become a page, and ODT is the one whose file **contains an actual page
break** and still declares none. None of them did.

**The remaining-formats row split three times, each time against a measurement.** S0 wrote it as one
line on purpose — *"this row splits into real slices when S2 and S3 are done and that cost is
measured"*. S4 measured the first half: a third OOXML format is one reader, one profile and one
fixture pair, because the container, the XML rules and the `r:id`-to-part rule are shared. **S5
measured the second half** by taking the cheapest non-OOXML format and finding that only the
container transferred — a new vocabulary, a new atom, a new detection question and a new class of
nested-block defect did not. So ODT became its own slice.

**The third split is this document's own, and it is on paper before the reader exists.** S5's
finding says it: an `.ods` is not an `.odt` with different tags. `<table:table-cell>` is a different
reader with a different atom and a different locator — the distance between `xlsx.rs` and `docx.rs`,
not the distance between two OOXML packages. Writing the split *after* the ODS reader lands would
mean writing it while somebody is already inside the file, which is exactly when "while we're here,
ODP is nearly free" gets said. So **S6 is ODS alone** and **S7 is the one row that is left** — ODP,
RTF, EPUB, CSV — and S7 stays one row until the cost of the next format in it is measured, on the
rule S0 set and S4 and S5 both honoured.

**The v2 gate is still DOCX + XLSX, and both still bind.** ODT is coverage beyond it. v2 is not
complete: neither S6 nor S7 has started, and no slice here closes v1.

**v1 is not done.** Its table number is measured and honest: macro cell-slot F1 is **64‰** on the
four tagged PDFs this repository owns, fabrication is **0**, and the **> 0.489 chase is parked** —
0.489 is a published ODL-local score on *their* corpus, same unit and a different exam
(`00-NORTH-STAR.md` #10, `09-V1-MILESTONES.md` S7, `table-gate-v1.md`). Parking is not a pass.
**v1-S7 is `09`'s slice and has nothing to do with this document's S7.**

**v1.1 is complete** at 0.14.1 and **v1.2 is complete** at 0.19.0. v2 began because the owner asked
for the next roadmap row, and nothing in it closes v1.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v2 scope + this document | — | **done** |
| **S1** | The grounding contract for a page-less source | S0 | **done — (b), and one finding** |
| **S2** | DOCX → representation — **and the page-parent invariant S1 found** | S1 | **done** |
| **S3** | XLSX → representation: sheets and cells | S2 | **done — and the `coordinate_system` decision** |
| **S4** | PPTX → representation: slides and shapes | S3 | **done — and a slide is a part** |
| **S5** | ODT → representation: paragraphs, and the page break in the file | S4 | **done — and the break is still not a page** |
| **S6** | ODS → representation: a spreadsheet the OpenDocument way | S5 | **not started** |
| **S7** | The remaining office formats — ODP, RTF, EPUB, CSV | S6 | **not started** |

**The order is deliberate.** S1 is a decision with no parser, ahead of the reader whose output
depends on it — the shape v1.2-S0 used for the handle law, and for the same reason: *so the first
implementation cannot quietly acquire a `page` "for convenience" while nobody has written down that
it may not.*

---

## S0 — v2 scope and slice map

- **Goal:** v2 exists as an ordered list of bounded changes **before** any office parser does, and
  the no-synthesised-pages law is written down **before** the first format that can violate it.

- **In:** `14-V2-SCOPE.md` (what v2 is, what it is not, the no-synthesised-pages law with its
  checkable spellings, the one IR posture, the open grounding question); this document; index rows
  in `docs/README.md` and `02-ROADMAP.md`; a CHANGELOG note.

- **Out:** any code. Any version bump. Any crate. Any fixture. Any schema change.

- **Acceptance tests:**
  - [x] Both documents exist and name S1–S4 as **not started**
  - [x] LibreOffice → PDF is refused in writing, with **L30**'s own reason quoted rather than
        paraphrased: *"It invents pagination. Pagination does not exist in a DOCX and must not be
        synthesised. This is a refusal, not a fallback."*
  - [x] The no-synthesised-pages law is **checkable**: it names the empty `pages` vector,
        `GeometryAbsence::NotApplicableToKind`, the `NativeLocator` union and `deny.toml` as the
        four places a violation would show, rather than asking a reviewer to remember
  - [x] One IR and one serializer is stated, with **no second canonical JSON per format**, and
        `04-ARCHITECTURE.md` §6's precondition (*"provided `engine-grounding` never learned about
        pages"*) is marked **unverified** rather than assumed
  - [x] `engine-office` is **named** as where an office crate would live, and not created
  - [x] The grounding-schema-versus-DOCX question is **posed with its three walls quoted from the
        schema**, given two honest readings, and left undecided
  - [x] v1 is still described as **missed at 64‰**; v1.2 is complete; S5 is refused
  - [x] Workspace unchanged at **0.19.0**; profile hash unchanged; no Rust, Python, Node, schema or
        fixture diff

- **Why first:** because the cheapest way to make a DOCX quote "ground" is to print it to PDF and
  read the page number off the result, and that produces a citation that looks correct, validates
  against the current schema, and is a measurement of a printer.

---

## S1 — the grounding contract for a page-less source

- **Status: done — the answer is (b).** `0.20.0`. No reader, no crate, no schema field.
  `ethos.grounding.v1` **stays PDF-only**, and the slice's measurement resized S2.

- **Goal:** decide what *"a DOCX quote grounds"* means, **before** a reader exists whose output
  depends on the answer.

### The decision, and why the other two were not available

| | reading | verdict |
| --- | --- | --- |
| **(a)** | revise `ethos.grounding.v1` for a page-less source | **blocked on Ethos.** Not refused — owned elsewhere |
| **(b)** | grounding stays PDF-only | **chosen** |
| **(c)** | not yet | not needed; the question was decidable |

**(a) is not this repository's to make.** `07-VERIFY-BOUNDARY.md` puts the artifact's contract on
the verifier's side, and `engine-cli/tests/oracle.rs` agrees with the pinned Ethos CLI on this exact
schema. An engine-only revision would produce artifacts the verifier does not speak while both still
called themselves `ethos.grounding.v1` — a fork of the contract M6 exists to keep identical. Adding
an optional `page` to the engine's copy would be the lying artifact v1.2-S5 refused for loose boxes,
wearing a different field name.

So **(b)**, and the tradeoff it buys, stated in one table:

| | under (b) |
| --- | --- |
| **gate wording** | *"a DOCX quote … grounds"* does **not** mean `ethos.grounding.v1`. The gate sentence needs re-reading, or the gate needs (a) |
| **schema** | untouched. Three walls intact, PDF-shaped, pinned by a test |
| **verifier ownership** | unchanged. The engine does not fork what it does not own |
| **S2 unblocked?** | **no — and that is the finding below** |

### The finding: the page assumption is not where S0 thought

S0 wrote that under (b) the gate would be met *"at the representation level"*. **That is false.**
`DocumentRepresentation::seal` refuses a node whose parent is not a declared page, in
`check_structure`, which runs on **both** construction paths:

> node `s1` names parent page `p1`, which is not a declared page

**A page-less document cannot become a representation at all.** So reaching the gate is upstream of
grounding, in the IR's own page-parent invariant — a v2 design decision about the representation,
not a schema question. S2 carries it, and now knows it before writing a reader against an invariant
that would have rejected its output.

### `04-ARCHITECTURE.md` §6's precondition, verified

> None — **provided `engine-grounding` never learned about pages**

**It holds, and it points at the wrong crate.** `engine-grounding` never learned what a page *is*:
it reads no locator (`engine_grounding_has_no_pdf_concept` fails if it mentions `NativeLocator`),
derives no geometry, and addresses pages by id. `project()`'s check that `node.parent` names a
declared page is a **re-assertion of an invariant `seal` already guarantees** — it can never be
handed a page-less representation, because one cannot be constructed. The assumption lives in
`engine-core`, which the M5 line permits (a contract invariant is not format machinery) and which
v2 has to revisit anyway.

- **In:** `crates/engine-grounding/tests/page_less_source.rs` (four guards); the decision in
  `14-V2-SCOPE.md` §5 and the verified precondition in §4; the §6 note in `04-ARCHITECTURE.md`;
  `0.20.0` and the moved profile hash; both SDK version pins; CHANGELOG; `docs/README.md`.

- **Out:** any reader, any format, any `zip` or `quick-xml`, `engine-office`. Any change to
  `ethos.grounding.v1`. Any `project()` change — the PDF path is untouched, and relaxing its page
  requirement "for office" would change PDF behaviour to accommodate a format that does not exist
  here yet.

- **Acceptance tests:**
  - [x] The three walls are pinned: `source.media_type` is `{"const": "application/pdf"}`, `element`
        requires `page` and `bbox`, `page` requires integer `index`/`width`/`height`/`rotation` with
        `width` bounded below by 1 — so a zero-width page is not a way to spell "no pages"
  - [x] A node whose parent is not a declared page **fails to seal**, with a message that names the
        reason
  - [x] The **same document with its page declared** seals *and* projects, so the test above fails
        for the page and for nothing else
  - [x] No renderer in `Cargo.lock` — LibreOffice, soffice, headless Chrome, wkhtmltopdf,
        WeasyPrint, chromiumoxide, printpdf — because **L30 lapses as a transitive dependency**
        before it lapses as a design decision
  - [x] `project()` unchanged; PDF goldens and the oracle (12 / 3) unchanged in meaning
  - [x] No office parser, no fifth crate, no schema field added
  - [x] Workspace **0.20.0**, both SDKs **0.20.0**, profile hash
        `sha256:30820a15ee750530f5232e622ee40cbfad3e8cb158d41011104cc25f5fd06c15`
  - [x] Table gate still **64‰**; `irs-form-1040-2025` still 0 tables; fabrication still 0
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt;
        Python and Node suites green after the version pin

- **Depends on:** S0.

---

## S2 — DOCX → representation

- **Status: done.** `0.21.0`. `engine-office` exists, `engine extract` reads a `.docx`, and
  `pages` is `[]` on the artifact it produces.

- **Goal:** a DOCX projects into the representation this engine already emits, and a quote from it
  resolves to a node.

- **The standing constraint:** §3's three obligations. The locator is a new `NativeLocator` variant
  addressing the document's own structure; geometry is `NotApplicableToKind`; `pages` is empty; no
  renderer enters the dependency graph.

- **What S1 handed this slice:** `pages` being empty is **not currently legal for a document with
  nodes** — `check_structure` requires every node's parent to be a declared page. So S2's first
  question is not "how do I read a DOCX" but **"what is a node's parent in a document with no
  pages?"** A structural parent (a body, a section, a paragraph) is the obvious answer and it is a
  change to `engine-core`'s invariant, which is a v2 design decision with review cost — not
  something to discover halfway through a reader. Whatever it becomes, `Node.parent`'s doc comment
  (*"The page this node was drawn on"*) stops being true and has to move with it.

### The invariant, changed the way S1 said it would have to be

`check_structure` now splits on the **locator family**, which is the one thing a node cannot fake:
it reaches the page-less rules only by carrying an address with no page in it.

| | paginated address | page-less address |
| --- | --- | --- |
| parent | a declared `PageRecord` — **unchanged, message included** | a `Part` id (`d1`), new at this slice |
| `pages` | as before | must be **empty**, or the seal refuses "invented pagination" |
| geometry | a measured box is checked against its page | a measured box is **refused**: there is no page to check it against |
| integrity | the parent is looked up in a declared list | part id ↔ part name is a **bijection**, so nothing needs a second list |

The last row is the part worth arguing about. A PDF gets its integrity from a declared-page lookup;
a DOCX has no such list, and inventing one would be a payload field for a format that describes
itself already. Instead every page-less node names its part in its own locator, and the seal checks
that one part id means one part name **in both directions**. That buys the same property without a
list to keep in sync.

The measured-box row is what makes "no geometry" a fact about the artifact rather than a habit of
the reader: `engine-office` could not emit a rectangle even if a later edit tried to.

### The locator, and the attributes

`DocxLocator { part, paragraph, run }` — a package part name and two 1-based document-order
positions. **No page, no bbox, no `x`/`y`**, and `deny_unknown_fields` so one cannot be added
quietly. If a field would have to be computed by laying the document out, it does not belong here.

`NodeAttributes::OfficeRun` is a **new variant rather than `TextRun` with the PDF fields blanked**:
a `<w:r>` has no char codes, no font resource name and no font size this reader read, and
`font_size: 0` would be three claims the document never made. Its one field is `space_preserved`,
the DOCX counterpart to LiteParse's `trailing_space_generated` — whether `xml:space="preserve"` was
set decides whether a run's spaces are the document's or the parser's.

`NodeKind::TextRun` is **reused**, not duplicated. A `<w:r>` and a show-text run are the same thing
addressed differently, and the locator is what says which.

### The crate, and the one new dependency

`engine-office`, exactly where `04-ARCHITECTURE.md` said an office reader would live. `engine-core`
learns no OOXML; `engine-grounding` still mentions no locator.

**One new crate in the lock: `quick-xml`.** ZIP is read in `engine-office/src/zip.rs` over `flate2`,
which the graph already carried via `lopdf` — the `zip` crate drags twelve transitives including
`zopfli`, a *compressor*, to save ~120 lines of central-directory reading. XML is the opposite call
and is **not** hand-rolled: entities, namespaces, CDATA and encodings are exactly where a
hand-rolled reader silently gets *text* wrong, and text is the evidence. That asymmetry is the
whole dependency argument, and v1.2-S1's refusal of an MCP framework is the precedent for both
halves.

**Measured, not feared:** the first version of the reader dropped `&amp;` silently, because
`quick-xml` 0.41 delivers an entity as its own event. The fixture carries an ampersand because of
it, and the reader now resolves the five XML predefined entities and **refuses every other name**
rather than letting one become an empty string in the evidence.

### What is read, and what is declared unread

`<w:p>` / `<w:r>` / `<w:t>` in `word/document.xml`, in the part's own order. Styles, numbering,
fields, drawings, comments, track-changes and embedded workbooks are **not** read.

Headers, footers, footnotes, endnotes and comments are counted and declared —
`office-parts-not-read`, with the count — because a reader that silently returned the body would
let a caller conclude a phrase is absent from a document that contains it. That is Anydoc's **A14**
applied to a package, and `fixtures/office/unread-parts` is the fixture that proves it lands.

### Detection, and every failure closed

**A4: the bytes decide.** A ZIP local-header signature plus `word/document.xml` in the central
directory — so `report.bin` reads and a `.docx` full of something else does not. A truncated
archive, a Zip64 record, an unimplemented compression method, a size that disagrees with the
directory, XML that will not parse, and a part that ends with elements still open are each a
**named** refusal.

- **In:** `crates/engine-office/` (reader, ZIP, tests); the locator-aware invariant, `IdKind::Part`,
  `DocxLocator`, `OfficeRunAttributes`, `Profile::docx_v0` and `XrefRepair::NotRun` in
  `engine-core`; `project()`'s named refusal in `engine-grounding`; content dispatch in
  `engine extract`; `fixtures/office/` and its generator; `0.21.0`, the moved profile hash and both
  SDK pins; `14`/`15`; `04-ARCHITECTURE.md`; CHANGELOG; README.

- **Out:** XLSX, PPTX, ODF, RTF, EPUB, CSV. Styles, numbering, fields, drawings, comments,
  track-changes, embedded assets. Any change to `ethos.grounding.v1`. Any `project()` change for the
  PDF path. Markdown or HTML for a DOCX. New MCP tools, new SDK functions, a LangChain path. A
  cargo-fuzz campaign — `A11`'s mutation lane for this format waits for a second one.

- **Acceptance tests:**
  - [x] A page-less document **seals** with `pages: []`; a paginated node in that same document
        **still refuses**, with the message S1 pinned
  - [x] A page-less document that declares a page is refused as **"invented pagination"**; a
        page-less node with a measured box is refused for having **no page to contain it**; a
        page-less node parented by a page id is refused
  - [x] Part id ↔ part name is a bijection, checked both ways
  - [x] `DocxLocator` carries no geometry; `deny_unknown_fields` on it and on `OfficeRunAttributes`
  - [x] `engine-office` exists and `engine-core` parses no ZIP and no OOXML
  - [x] The fixture extracts; a known phrase is on a node with a `DocxLocator`; `node_get` over
        **unmodified MCP** resolves it and a forged id fails closed
  - [x] Detection is content-based both ways: a renamed `.docx` reads, a `.docx` that is not one is
        a named failure with empty stdout
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the law
  - [x] `pages` is `[]`, every geometry row is `NotApplicableToKind`, and the profile hash differs
        from the PDF default
  - [x] Unread text parts are counted and declared; the clean fixture declares none
  - [x] Two runs over one document produce identical bytes
  - [x] `Cargo.lock` still has no LibreOffice, soffice, headless Chrome, wkhtmltopdf, WeasyPrint,
        chromiumoxide or printpdf; `cargo deny check` passes
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables; the
        markdown and html goldens still green
  - [x] Workspace **0.21.0**, both SDKs **0.21.0**, profile hash
        `sha256:1a7844f5a291cdd1430ecead0523980283a6c3e7bbf3f6f431ce5a056c5d92f1`

- **Depends on:** S1.

---

## S3 — XLSX → representation

- **Status: done.** `0.22.0`. `engine extract` reads a `.xlsx`, a cell binds, and the artifact
  carries **two parts** — the first this engine has ever produced.

- **Goal:** the second half of the v2 gate — an XLSX **cell** binds.

- **The standing constraint:** the same three obligations, and one that is specific to spreadsheets:
  a cell's address is `(sheet, row, column)` **as the file states it**, and a rendered column width
  or a print range is not part of it. A spreadsheet's "page" is a print artefact and is exactly the
  thing §3 forbids.

### The locator, and the two spellings the file uses

`XlsxLocator { part, sheet, row, column }` — `deny_unknown_fields`, **no page, no bbox, no column
width, no print area**. Two things about it are worth arguing.

**Both `part` and `sheet`, because they answer different questions.** `part` is the package's name
for the worksheet (`xl/worksheets/sheet1.xml`) and is what `check_structure`'s part-id ↔ part-name
bijection checks. `sheet` is the workbook's name for it (`<sheet name="Ledger">`) and is what a
person citing a cell writes down. The part name does not contain the sheet name and never will.

**`row: u32` and `column: String`, and the asymmetry is the file's.** `<c r="B12">` states the
column as the letters `B` and the row as the digits `12`. Reading `12` as a number is reading — the
attribute's own type is an integer. Turning `B` into `2` is arithmetic on a bijective base-26
numeral, which is a computation the file never performed and a value it never contains.
Concatenating the two reproduces the `r` attribute exactly, so splitting it loses nothing and
creates no second place for the address to live.

### The part the shortcut would have skipped

**`xl/workbook.xml` contains no part names at all.** A `<sheet>` carries `name`, `sheetId` and
`r:id`, and only `xl/_rels/workbook.xml.rels` says which part an `r:id` means. So the reader reads
four parts, not three.

The shortcut — assume `xl/worksheets/sheet{n}.xml` in `<sheets>` order — is wrong in *ordinary*
files. Reordering sheets in Excel reorders the `<sheet>` elements and leaves the part names alone,
so the first sheet is routinely `sheet3.xml`; deleting a sheet leaves a gap; and part names are
author-chosen. Every one of those failures attaches the **wrong sheet name to the right cells** —
a locator that is confidently wrong, which is strictly worse than one that is absent. The fixture's
second sheet is `sheet3.xml` behind `rId7` for exactly this reason, so the shortcut cannot pass.

### The invariant that did not have to change

**S3 added nothing to `engine-core`'s page-less rules.** A workbook is one part per sheet, and
v2-S2's shape already allows it: the part-id ↔ part-name check is a **bijection**, not a
cardinality-of-one rule, so two ids naming two parts violates neither direction; and
`check_structure` counts ordinals **per parent**, so each sheet carries its own contiguous 1-based
sequence. S2 built the shape with one part and S3 is the first artifact to use it with more than
one — which is what `two_parts_with_two_names_seal_and_keep_separate_ordinals` now pins.

### `NodeKind::TextRun` is reused, and here is the tradeoff

v1-S1 refused a `TableCell` kind for PDF because a cell's text was already in runs. For a workbook
that argument does not apply — the cell **is** the atom and no run exists — so the question is the
standing rule's other half: *does "this text is a cell" name a fact no existing node carries?*

It does not. `XlsxLocator` says sheet, row and column, which is cell-ness spelled out in the one
place a consumer must already look; the locator union is externally tagged, so telling a cell from
a glyph run is one match arm. A second kind would restate the locator — exactly why v1-S3 refused
`Paragraph` when the role path already said `P` — and would make every consumer handle two names
for one concept. So the kind is reused and **`NodeAttributes::OfficeCell` is a new variant**,
because *that* is where the facts with no home live: `value_type` (what `t` says the stored value
is) and `text_source` (whether the text is a stored value, a cached formula result, or a formula's
source because nothing was cached). A `<c>` has no `xml:space` to record and a `<w:r>` has no value
type, so `OfficeRun` with fields blanked would have been three claims the file never made.

`capabilities.tables` stays **false** on a format made of grids, and that is not modesty: `tables`
means *this run emitted `TableRecord`s*, and this slice emits cells. A consumer reading `true`
would go looking for a table IR that is not there.

### `<f>` is not a second authority

No evaluator, and no `computed_value` field. A cell's text is the value the workbook **stored** —
its cached `<v>`, or the formula source as written when nothing was cached — and `text_source` says
which, so `SUM(B2:B2)` can never be read as a number the sheet displayed.

### `coordinate_system` — measured, and deliberately left inert

**The decision S2 handed this slice, and the answer is: no mode enum.**

S2 left `docx_v0` carrying `centipoint`/`top-left` because the field is required and has no
page-less spelling, and asked S3 to decide it with two formats' worth of evidence. The evidence is
that **nothing acts on the value for a page-less artifact**:

| where | what it does |
| --- | --- |
| `engine_grounding::project` | **hard-codes** `centipoint`/`top-left` rather than copying the representation's, and refuses a non-`application/pdf` source 179 lines earlier |
| `engine-grounding/src/check.rs` | the only branch on the value anywhere — inside the `ethos.grounding.v1` validator, downstream of that same refusal |
| both SDKs | zero occurrences; the field is re-hashed as opaque bytes and never parsed |
| `library_surface.rs` | asserts the pair for *geometry-bearing* artifacts, all of them PDF |

So a mode enum would have moved **every PDF artifact's profile hash** to respell a value nothing
reads. `xlsx_v0` carries the same inert declaration, and the property that makes it honest rather
than merely quiet is pinned instead: both page-less profiles declare `measured_ink_boxes: false`,
and `the_page_less_profiles_declare_the_same_inert_coordinate_system` asserts **zero measured
geometry rows** on a real workbook artifact. Written into `Profile::docx_v0`'s own doc comment and
the profile-hash ledger's twenty-sixth entry, so the next slice finds the decision where it will
look for it.

### What is read, and what is declared unread

`xl/workbook.xml`'s sheet list, `xl/_rels/workbook.xml.rels`, each worksheet's `<sheetData>`, and
`xl/sharedStrings.xml`. Styles, number formats, charts, pivot caches, drawings, comments,
conditional formatting, merged-cell geometry and VBA are **not** read.

Charts, chart sheets, drawings, comments, threaded comments and pivot caches are counted and
declared — `office-parts-not-read`, with the count — and so is a listed sheet that is **not a
worksheet**: a chart sheet or a dialog sheet has no `<sheetData>`, and returning it as an empty
worksheet would be a silent drop. That is **A14** applied to a workbook.

The two halves are proven separately, because one fixture cannot do both.
`fixtures/office/workbook-unread-parts` covers the unread *parts* half. The listed-sheet half
needs a package with a **dialog sheet and no chart, drawing or comment part** — a real chart sheet
drags `xl/charts/` and `xl/drawings/` along, so the parts count would fire anyway and the branch
would stay unproven — so `a_listed_sheet_that_is_not_a_worksheet_is_declared_with_a_count` authors
that package inside the test. Deleting `|| non_worksheets > 0` from the reader fails it; before
that test existed, deleting it failed nothing.

### Detection, and every failure closed

**A4: the bytes decide.** A ZIP local-file-header signature plus `xl/workbook.xml` in the central
directory. A package listing **both** main parts is a **named refusal** rather than a race between
two `if`s — `engine_office::read` decides on the central directory, so the answer does not depend on
the order of the dispatcher's lines. `xl/workbook.bin` (`.xlsb`) is deliberately not claimed.

Each of these is a named refusal: a `<c>` with no `r` attribute (an implied address is one this
engine would have *counted*), an `r` that is not an A1 reference, a cell whose row disagrees with
its `<row r="…">`, a `t` outside `ST_CellType`, a shared-string index that does not exist or is not
a number, a `<sheet>` whose `r:id` matches no relationship, a part that will not inflate, XML that
will not parse, and a part that ends with elements still open.

### Two things measured in passing, and what was done about each

**`quick-xml` 0.41 delivers a numeric character reference as a `GeneralRef` event too**, named
`#66`. So the five-entity rule refuses `&#66;` — ordinary XML that needs no DTD. It is a **named
refusal of a valid document**, not a silent drop, so it fails in the safe direction; recorded in
`engine-office/src/xml.rs` and left alone, because widening it would change what a shipped DOCX
artifact contains and nothing in this slice measured a need for that.

**`Event::CData` was unmatched**, and an unmatched CDATA arm is a *silent drop* — the one failure
the v2 standing rules name first. The workbook reader matches it. The DOCX reader's arms are
unchanged, and the observation is recorded here rather than acted on for the same scope reason.

**One repair shipped:** the `geometry-absent-not-groundable` limitation was pushed
unconditionally, and `check_geometry_matches_its_declaration` requires it to be present exactly
when at least one node has no measurable box — so a package with **no text at all** could not seal.
Pathological for a `.docx`, ordinary for a workbook with an empty sheet. Both readers now push it
only when there are nodes.

### What reviewing the slice against §3 found before it shipped

The reader was written, then read back against the standing rules rather than against its own
intent. That found **eight** defects, and fixing them surfaced a ninth. The pattern in them is
worth keeping: every one was a *silent* failure, and three were in code whose own comment
described the hazard it had.

| | defect | why it was silent |
| --- | --- | --- |
| 1 | a self-closing `<si/>` took no slot in the shared string table | `Empty` is its own event; only `Start`/`End` were matched, so every later index resolved to the **next** string — a right address carrying another cell's text |
| 2 | `<rPh>` furigana concatenated into inline-string cells | the shared-string path stripped it; `<is>` is the same content type and did not |
| 3 | a second `<v>` appended instead of being refused | `<v>1</v><v>2</v>` became `12`, and under `t="s"` resolved shared string **12** |
| 4 | a `t` naming a child the cell did not have returned "no node" | indistinguishable from an empty cell, so present characters vanished |
| 5 | `split_reference` repaired `A+1` and `A01` into `A1` | `u32::from_str` accepts a sign and leading zeros, so the locator spelled a **different cell** |
| 6 | two cells at one address both became nodes | a citation to that address would have had two answers |
| 7 | errors reading the string table were swallowed | a part over the size cap re-surfaced as "the table has 0 entries", blaming a worksheet |
| 8 | `NodeAttributes`' documented kind cross-check did not exist | prose since v0; a record could read as two different things depending on which field was trusted |
| 9 | a cell carrying **both** a `<v>` and an `<is>` | found while fixing 4: whichever `t` named would be read and the other dropped without a word |

Numbers 1, 2 and 5 are the ones that matter most, because each produced a **sealed,
byte-identical, error-free artifact with the wrong text at the right address** — the failure this
module's own header calls strictly worse than no address at all. Number 5 also falsified
`XlsxLocator`'s documented promise that its two halves concatenate back to the `r` attribute,
which is now true rather than intended.

Two more were found in this repository's claims rather than its code: `engine-office` had been
**outside the public-API freeze** since S2 — making `read` and `is_docx` "internal" by
`PUBLIC-API.md`'s own rule — and the non-worksheet declaration branch was ticked as covered while
no test reached it. Both are closed, the second mutation-checked: deleting `|| non_worksheets > 0`
now fails a test, and before it did not fail anything.

- **In:** `crates/engine-office/{xlsx.rs, xml.rs}` and the router in `lib.rs`; `XlsxLocator`,
  `NodeAttributes::OfficeCell`, `OfficeCellAttributes`, `CellValueType`, `CellTextSource`,
  `Profile::xlsx_v0`, `XLSX_READING_ORDER_RULE_V1` and `XLSX_TEXT_CODE_RULE_V1` in `engine-core`;
  content dispatch in `engine extract`; `fixtures/office/workbook-cells` and
  `workbook-unread-parts` with their generator; `0.22.0`, the moved profile hash and both SDK
  pins; `PUBLIC-API.md` and its gate — **which `engine-office` had been outside since S2, so
  `read` and `is_docx` were "internal" by that document's own rule until this slice put the crate
  in the frozen table**; `14`/`15`; `04-ARCHITECTURE.md`; CHANGELOG; README.

- **Out:** PPTX, ODF, RTF, EPUB, CSV. Formula evaluation, styles, number formats, charts, pivot
  caches, drawings, comments, conditional formatting, merged-cell geometry, VBA. Any change to
  `ethos.grounding.v1`. Any `project()` change. Markdown or HTML for a workbook. New MCP tools,
  new SDK functions. A mode enum on `coordinate_system` — decided above, with the evidence.

- **Acceptance tests:**
  - [x] A known cell's text is on a node with an `XlsxLocator`; `pages` is `[]`; `node_get` over
        **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator carries no page, no bbox, no column width and no print area, and
        `deny_unknown_fields` refuses each of those by name
  - [x] Sheet ↔ part is resolved through `xl/_rels/workbook.xml.rels`, proven by a fixture whose
        second sheet is `sheet3.xml` behind `rId7`; a dangling `r:id` is a named refusal
  - [x] Rows are read, not counted: a sheet numbering rows 1, 2, 12 has no cell at row 3
  - [x] Two sheets seal as **two parts**, with the bijection holding in both directions and
        ordinals restarting at 1 per part — with **no change to `engine-core`'s invariant**
  - [x] A shared-string index that does not exist **fails closed**; a self-closing `<si/>` still
        holds its position, so no later index is repointed; `&amp;` survives in a cell's text
        *and* in a sheet name; rich-text `<si>` runs concatenate; `<rPh>` furigana does not — on
        **both** the shared-string and the inline-string path
  - [x] A cell with no `t` is a number, which is SpreadsheetML's declared default
  - [x] A formula cell carries its cached `<v>` and is labelled `cached_formula_result`; one with
        no cached value carries its source and is labelled `formula_source`; nothing is evaluated
  - [x] Detection is content-based both ways: a renamed workbook reads, a `.xlsx` that is not one
        is a named failure with empty stdout, a DOCX is never claimed as a workbook, and a package
        that is **both** is refused by name
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the
        law — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; `xlsx_v0`'s hash differs from `docx_v0`'s and
        from the PDF default; `capabilities.tables` is false
  - [x] Unread parts and non-worksheet listed sheets are counted and declared — **each held by
        its own test**, the second by a package authored inside the test rather than by a fixture
        that does not contain the case; the clean fixture declares none
  - [x] Two runs over one workbook produce identical bytes
  - [x] The `coordinate_system` decision is written down with its evidence, both page-less
        profiles keep the inert declaration, and **no PDF hash moved for it** — the hash moved on
        `parser_version` alone, as at S2
  - [x] `Cargo.lock` still has no LibreOffice, soffice, headless Chrome, wkhtmltopdf, WeasyPrint,
        chromiumoxide or printpdf, and no `zip`, `zopfli`, `calamine` or `umya-spreadsheet`;
        `cargo deny check` passes
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0; the markdown and html goldens still green
  - [x] Workspace **0.22.0**, both SDKs **0.22.0**, profile hash
        `sha256:26c10d2a73e41c357c82589b0acfabffd8b33f9f93e3c666452b8a437743b6fa`

- **Depends on:** S2.

---

## S4 — PPTX → representation

- **Status: done.** `0.23.0`. `engine extract` reads a `.pptx`, a slide's text binds, and **a
  slide is a part rather than a page** — which is the whole of this slice's argument.

- **Goal:** the next format on the roadmap line, on the terms the first two established.

### The temptation this format exists to test

DOCX has no page at all: one does not exist until a renderer decides where it falls. XLSX's page
is a print artefact: a printer decides it. **A slide is neither.** It is discrete, addressable,
listed in the package, and a person counts them out loud — *"it's on slide 12"*. This is the first
v2 format where inventing a page would not even feel like inventing one.

It is still a **part**, and the two things that would have made it a page are both refused by name:

| the tempting field | what it actually is |
| --- | --- |
| `p:sldSz` → `PageRecord {width, height}` | a size the authoring tool wrote, which this engine measured nothing against |
| position in `<p:sldIdLst>` → `PageRecord.index` | display **order**, which changes when a deck is reordered, and which a consumer would read as a page number |

So `pages` is `[]`, each slide is one part id, and `PptxLocator` carries **no slide number at
all** — a caller that wants deck position reads `ppt/presentation.xml`, where it is a fact about
the presentation rather than a claim baked into every citation. `a_deck_declares_no_pages_and_
carries_no_page_number` asserts the locator's exact field set, so a slide index cannot arrive
later as a fifth field.

### The locator, and the field that measurement moved

`PptxLocator { part, shape, paragraph, run }` — `deny_unknown_fields`, no page, no bbox, no
`x`/`y`.

**`shape` was going to be the shape's own id, and measurement changed it.** `<p:cNvPr id="7"
name="Title 1"/>` is a number the file wrote, which is exactly the kind of thing this engine
prefers over one it counted. Across 18 real decks — 329 slides, 3,335 shapes — the id is present
every time and **unique only most of the time**: 12 slides from an Open XML SDK generator reuse
one, and PowerPoint opens them without complaint. Addressing by it would have given one address
two answers on real files, and refusing those files would have rejected decks that open
everywhere else. So the id moved to the attributes, where a non-unique label is exactly what it
is, and all three components are 1-based positions in the part's own document order.

That is the same correction v2-S3 made twice, arriving before the code shipped rather than after.

### What a reader that only saw top-level shapes would miss

Measured, not assumed. Of 5,297 `<a:t>` elements across those decks:

| where | count | this reader |
| --- | --- | --- |
| `p:sp > p:txBody` | 4,670 | **read** |
| `p:grpSp > … > p:sp` (nested groups) | 160 | **read** — groups are shapes |
| `p:graphicFrame` (tables, charts, SmartArt) | 287 | **counted** (A14) |
| `a:fld` (slide numbers, dates) | 180 | **counted** (A14) |

A reader that saw only **top-level** shapes would get 4,670 of 5,297 — 88.2% — and say nothing
about the rest; descending into groups brings this one to 91.2%, and the remaining 8.8% is
counted rather than dropped. Groups appeared on essentially every slide of every deck, so handling
them is not an edge case; and the field is the interesting refusal, because an
`<a:fld type="slidenum">` holds a **cached** slide number written at save time that goes stale the
moment the deck is reordered. Reading it would put a number in the evidence
that is not on the screen and is shaped exactly like the page index this version refuses.

### `mc:AlternateContent`: one phrase, one node — and the counters still see the rest

`mc:AlternateContent` appeared on 162 of 329 slides. It carries one or more `mc:Choice` and an
optional `mc:Fallback`, **all stating the same content** for consumers of different capability. A
descendant walk emits that phrase at two or three addresses — the mirror image of a silent drop.
Exactly one branch is read: the first `Choice`, deterministically. The rest are counted when they
held text, and a branch wrapping only a transition is not counted at all.

**The counters still advance through the skipped branches**, and that is the half worth stating.
`shape` promises a position in the part's own document order, so a consumer checking it counts
`<p:sp>` elements in the file. Counting only what was read would leave every address after an
`AlternateContent` one short — a locator that is confidently wrong, which is exactly the failure
this slice's own review caught before it shipped. The same holds one level down for `<a:p>` and
`<a:r>`, and for the self-closing `<a:p/>` that python-pptx and Apache POI write for a blank
line: the address must not turn on how a deck was serialized.

### The rule that moved to a module of its own

**Three formats now share one answer to "what part does this `r:id` mean?"** `xl/workbook.xml` and
`ppt/presentation.xml` both list things and name no parts; both invite the `sheet{n}` / `slide{n}`
shortcut; both break under it. `opc.rs` holds that rule now, along with the target resolution, and
`xml.rs` grew from the entity rule to the whole XML-reader plumbing — because the alternative was
a third copy, and **three of the nine defects v2-S3's review found were two copies of one rule
disagreeing**. Nothing in the moved code changed; the DOCX and XLSX suites passed unaltered across
the move, which is what made it a move rather than a rewrite.

`resolve_target` did gain one thing: `.` and `..` normalisation. 2,318 of the measured relationship
targets climb a directory, and a reader that joined them literally would refuse packages that open
everywhere else. A target that climbs out of the package resolves to a name no central directory
contains, which is a named refusal rather than a path this reader goes looking for.

### Detection, and every failure closed

**A4: the bytes decide.** `ppt/presentation.xml` in the central directory. The router now counts
the main parts a package lists rather than asking three ordered questions, so a package claiming
two formats is a **named refusal** rather than whichever `if` ran first. A legacy `.ppt` is an OLE
compound file, not a ZIP, and is refused at the first question — correctly, since it is a
different format with a different reader.

Named refusals: a `<p:sldId>` with no `r:id`; an `r:id` matching no relationship; two entries
resolving to one part; a presentation listing no slides; a `<p:sp>` with text and no `<p:cNvPr
id>`; a non-numeric id; a truncated part.

- **In:** `crates/engine-office/{pptx.rs, opc.rs}` and the widened `xml.rs`; the three-way router
  in `lib.rs`; `PptxLocator`, `NodeAttributes::OfficeSlideRun`, `OfficeSlideRunAttributes`,
  `Profile::pptx_v0`, `PPTX_READING_ORDER_RULE_V1` and `PPTX_TEXT_CODE_RULE_V1` in `engine-core`;
  content dispatch in `engine extract`; `fixtures/office/deck-slides` and `deck-unread-parts`;
  `0.23.0`, the moved profile hash and both SDK pins; `PUBLIC-API.md` and its gate; `14`/`15`;
  CHANGELOG; README.

- **Out:** ODF, RTF, EPUB, CSV — parked in S5. Speaker notes, masters and layouts as evidence;
  SmartArt and chart text; animations, transitions, embedded workbooks, theme fonts, shape
  positions. Any change to `ethos.grounding.v1`. Any `project()` change. Markdown or HTML for a
  deck. A `coordinate_system` mode enum — v2-S3 closed that and this slice did not reopen it.

- **Acceptance tests:**
  - [x] A known slide phrase is on a node with a `PptxLocator`; `pages` is `[]`; `node_get` over
        **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator's field set is exactly `{part, shape, paragraph, run}` — **no slide number**,
        no page, no box — and `deny_unknown_fields` refuses each of those by name
  - [x] Slide parts are resolved through `ppt/_rels/presentation.xml.rels`, proven by a fixture
        whose second slide is `slide7.xml` behind `rId4`; a dangling `r:id` is a named refusal
  - [x] Two shapes sharing one `<p:cNvPr id>` — a file PowerPoint opens — still get distinct
        addresses, and the id survives as a label
  - [x] A shape inside a `<p:grpSp>` is read and addressed like any other
  - [x] Two slides seal as **two parts**, ordinals restarting at 1 per part, with **no change to
        `engine-core`'s invariant**
  - [x] `&amp;` survives; CDATA is matched rather than dropped; only the **first** `<mc:Choice>`
        of an `<mc:AlternateContent>` is read, and the branches passed over are counted **when
        they held text**
  - [x] The document-order counters advance **through** a skipped branch, so a shape after an
        `<mc:AlternateContent>` keeps the number the file gives it; a self-closing `<a:p/>` counts
        as a paragraph, so the address does not turn on how the deck was serialized. Both
        mutation-checked
  - [x] Detection is content-based: a renamed deck reads, a `.pptx` that is not one fails with
        empty stdout, a DOCX and an XLSX are never claimed as presentations, and a package that
        is two formats is refused by name
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the
        law — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; all four profile hashes are mutually
        distinct; `capabilities.tables` is false
  - [x] Unread parts **and** unread shapes are counted and declared, each held by its own test —
        and the shape branch is **mutation-checked**: deleting it fails a test, and the first
        version of that assertion did not, because `v2-S4 reads slide shape text only` contains
        both a `2` and the word `shape`
  - [x] Two runs over one deck produce identical bytes
  - [x] `Cargo.lock` gains **no new dependency** — no `zip`, `zopfli`, `calamine` or renderer
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0
  - [x] Workspace **0.23.0**, both SDKs **0.23.0**, profile hash
        `sha256:8dfca0e41d51668c6d540ec45bf83dd66503dbc2dcb1fa7c188cceebe255d4bd`

### What reviewing the slice against §3 found before it shipped

Three defects, all of the same class v2-S3's review named: **a right address pointing at the wrong
thing.** None of them lost text; each made a locator disagree with what a consumer counting
elements in the file would find.

| | defect | why it was wrong |
| --- | --- | --- |
| 1 | a `<p:sp>` in a skipped `<mc:AlternateContent>` branch did not advance the shape count | every shape after it was numbered one short, so a verifier following the documented rule landed on the skipped shape |
| 2 | a self-closing `<a:p/>` was invisible to the paragraph count | `<a:p/>` and `<a:p></a:p>` are the same infoset; the address turned on the serialization, and python-pptx and Apache POI write the first |
| 3 | every `<mc:Choice>` was read, not just the first | one displayed phrase became two or three citable nodes — the duplication this reader's own comment claimed to prevent, enforced against `Fallback` only |

Defect 3 is the sharpest: the code said *"reading both would emit the same text twice at two
addresses"* and then guarded one of the two ways that happens. All three are fixed and each is
mutation-checked — deleting the fix fails a test.

- **Depends on:** S3.

---

## S5 — ODT → representation

- **Status: done.** `0.24.0`. `engine extract` reads a `.odt`, a paragraph binds, and **the one v2
  format whose file contains a page break still declares no pages** — which is this slice's whole
  argument.

- **Goal:** OpenDocument **text**, on the terms the first three established, and the
  remaining-formats row split so the next one is scheduled against a measurement rather than a
  guess.

### Why this slice is ODT and not "ODF"

S4 measured that a *fourth OOXML* format would be cheap, and said so. ODT is the first case where
that finding does not transfer: it is a ZIP, and that is the end of the overlap. The vocabulary is
`text:p` rather than `w:p`; the `r:id`-to-part indirection **`opc.rs` exists for does not apply**,
because the OpenDocument package specification fixes the content part's name; and the package
declares its own type in a `mimetype` entry rather than being identified by which main part it
lists.

**An `.ods` would have been a second slice wearing this one's name.** A spreadsheet's `content.xml`
is `<table:table-cell>`, which is a different reader with a different atom and a different locator
— the distance between `xlsx.rs` and `docx.rs`, not the distance between two OOXML packages. So S5
is ODT alone. That finding is also what split the row again: **S6 is ODS alone**, and **S7 carries
what is left** — ODP, RTF, EPUB, CSV.

### The format that tested the law the hardest, and still did not move it

S4's argument was that a slide *looks* like a page and is a part. ODT is one step past that: its
`content.xml` **contains an actual page break**.

| what the file states | what it is |
| --- | --- |
| `<text:soft-page-break/>` | the position at which the *producing application's* layout broke the page, computed from its font stack and paper size and written down at save time |
| `styles.xml`'s `fo:page-width` / `fo:page-height` | the paper the author chose, which this engine measured nothing against |

Between them a `PageRecord` would have needed **no arithmetic at all** — an index from counting
breaks, a width and a height from the master page. That is exactly why it is refused, and the
reason is L30's own, quoted rather than paraphrased: *"It invents pagination."* A soft page break
moves when the font stack, the paper size or the producing application changes, so a citation
carrying it would be a measurement of a word processor. It is **read and discarded**: the reader
matches the element, contributes no character and no address, and `pages` stays `[]`. The clean
fixture contains one on purpose, so the test that `pages` is empty is not vacuous.

### The locator, and why the atom is the paragraph

`OdtLocator { part, paragraph }` — `deny_unknown_fields`, no page, no bbox, no `x`/`y`, and
`a_text_document_declares_no_pages_and_carries_no_page_number` refuses `page`, `bbox`, `x` and
`soft_page_break` by name.

**`<text:span>` was the obvious analogue of `<w:r>` and it is the wrong atom**, for two reasons the
format supplies:

1. **A paragraph may contain no span at all.** `<text:p>Plain text</text:p>` is ordinary ODF. A
   span-based address would leave the commonest case with no address, or force a span number the
   file does not contain — which is the invented identifier §9's fourth standing rule forbids.
2. **Where spans do exist, their boundaries are wherever a word was bolded.** *"The **important**
   part."* would become three nodes, and the sentence a reader quotes would bind to none of them.
   DOCX has the same property and S2 accepted it because a `<w:r>` is the only address a DOCX
   offers; ODF offers the block, and the block is what ODF itself treats as the unit of text.

So the address is the block, and `NodeAttributes::OfficeParagraph` carries the one fact the element
states that the locator does not: whether it was a `<text:p>` or a `<text:h>`. A fifth attributes
variant rather than `OfficeRun` with a borrowed field, for the reason there was a fourth: **ODF does
not use `xml:space`**, so `space_preserved` would be a claim about a mechanism this format does not
have. Its whitespace mechanism is `<text:s text:c="n">`, which this reader reads as the count the
file states.

### Two numbers on one node, and they are deliberately different

`Node.ordinal` counts nodes in the artifact, contiguously, as it does for every other format.
`OdtLocator.paragraph` counts `<text:p>` and `<text:h>` elements **in the file** — including the
ones inside a footnote, a comment or a tracked-changes record that this slice does not read, and
including a self-closing `<text:p/>` blank line.

The unread fixture makes the difference visible: its three nodes carry ordinals 1, 2, 3 and
paragraphs 1, 3, 5. A reader that numbered only what it kept would call them 1, 2, 3 in both
columns, and **every citation after a footnote would name the wrong paragraph** — the confidently
wrong locator S4's review caught three times, arriving here through a different door. Both
directions are mutation-checked.

### ODF's own whitespace rule is applied, and that is reading

The three elements that become characters — `<text:s text:c="n">`, `<text:tab/>`,
`<text:line-break/>` — exist because OpenDocument **defines** what the character data around them
means: a tab, line feed or carriage return counts as a space, a run of spaces counts as one, and
the spaces at a block's two ends are not part of it. That rule is in the format, not in a layout
engine; it is *why* a producer wanting three spaces must write `<text:s text:c="3">` rather than
typing three.

Not applying it would make the text depend on how the file was **serialized** — a producer that
indented inside a paragraph would hand back a sentence with a newline in the middle of it. That is
S4's defect 2 (*"the address turned on the serialization"*) arriving one slice later in the *text*,
which is worse. The characters the three elements state are exempt, because surviving the rule is
their whole purpose.

### Detection, and the reason it asks a different question

**A4: the bytes decide**, and for ODF the bytes say so out loud. `is_odt` asks whether the
**first** central-directory entry is a **stored** `mimetype` containing
`application/vnd.oasis.opendocument.text` — first and stored because the package specification
requires it, so that a consumer can identify a document from its leading bytes.

That is what keeps an `.ods` and an `.odp` out. Both have a `content.xml`; reading a spreadsheet's
with this vocabulary would return a document with **no text and no error**, which is a gap
presented as a success (§9, rule 5). A package carrying an ODF `mimetype` *and* `word/document.xml`
is a named refusal, because the router counts evidence rather than asking four ordered questions.

### One thing measured in passing, recorded rather than acted on

**Through the CLI, an `.ods` is refused with a message about PDF.** `is_odt` correctly declines it,
none of the other three claim it either, so it falls through to the PDF reader and gets *"expected a
PDF header (%PDF-) at byte 0"*. It fails closed with empty stdout and exit 2, which is what §3 and
§9's fifth rule require — but the message names the wrong cause, which is the same complaint this
slice makes about handing an encrypted `content.xml` to the XML reader.

It is **not fixed here**, on v2-S3's precedent for `&#66;` and the unmatched `CData` arm: a named
refusal for the ODF family means deciding what this engine says about a spreadsheet it will read in
S6, and inventing that sentence now would put a claim about ODS in a slice that does not read one.
**Recorded as S6's, and S6 owns fixing it** — where the answer is one line rather than a guess.

### The manifest, which replaces the rule `opc.rs` holds for OOXML

`content.xml`'s name is fixed, so there is no relationship to resolve — but the package's own list
of what it contains is still consulted before anything is inflated. `META-INF/manifest.xml` is read,
and two failures are named rather than discovered later:

- the manifest **does not declare** `content.xml` — a package that does not list its own content is
  not one this reader can speak for
- the manifest declares it **encrypted** — refused by name, because handing ciphertext to the XML
  reader would come back as *"will not parse"* and name the wrong cause

### What is read, and what is declared unread

Read: `content.xml`'s `<text:p>` and `<text:h>` blocks, wherever they sit — a table cell's, a text
box's and a list item's are blocks like any other. Not read, and **counted** (**A14**):

| | why it is not read |
| --- | --- |
| every package entry but the three consumed | `styles.xml` (where a header or footer lives), `meta.xml`, `settings.xml`, `Pictures/`, embedded objects. Counted by subtraction rather than by a prefix list, so a producer naming something new cannot slip past |
| `<text:note>` | a footnote or endnote body — a second stream of text, as `word/footnotes.xml` is in a DOCX |
| `<office:annotation>` | a comment; reading it would put a reviewer's remark in the record as the document's own |
| `<text:tracked-changes>` | what a revision **deleted**; reading it would put text the document no longer states into the evidence |
| a second `<draw:text-box>` in one `<draw:frame>`, **or any text box with no enclosing frame** | ODF frames hold *alternative renditions* of one object and a consumer uses the first it can process — reading both emits one displayed phrase at two citable addresses, which is S4's `<mc:AlternateContent>` finding in ODF's spelling. A frameless text box cannot occur in a conforming package, so it is passed over rather than guessed at — stated here because the reader's behaviour is wider than "a second one" and the prose used to claim otherwise |
| any element inside a block that is not an allowlisted inline one | an image's `<svg:title>`/`<svg:desc>`, an embedded object's `<office:binary-data>`, a generated `<text:number>`, a field's cached value, `<text:ruby-text>` furigana. Counted in their own bucket, because they are characters *inside* a block rather than a region of the part |

### What an adversarial review found *after* the author's own, and the rule it changed

The self-review below found four nesting defects. A second, adversarial pass over the shipped slice
found the **architecture** wrong, and that is the finding worth keeping.

`read_content` was a descendant walk with a four-item exception list: every `Event::Text` reached
the innermost open block unless it sat inside a note, a comment, a tracked change or a second
`<draw:text-box>`. ODF puts a great deal of non-displayed character data inside a `<text:p>`, and a
text-anchored `<draw:frame>` is a **child of the paragraph it is anchored in** — the ordinary shape,
not an edge case. So all of this landed in the sentence:

| what | where it comes from |
| --- | --- |
| `<svg:title>`, `<svg:desc>` | an image's title and alt text; LibreOffice writes them whenever the user fills them in |
| `<office:binary-data>` | an embedded object's base64, a spec-legal alternative to `xlink:href` — a kilobyte of `iVBORw0KGgo…` spliced mid-sentence |
| `<text:number>` | a heading's **generated** label, present only for consumers that do not number: `2.1Scope of this report` |
| `<text:page-number>`, `<text:page-count>`, `<text:chapter>` | a field's **cached** value |
| `<text:ruby-text>` | furigana — a pronunciation guide, not part of the word |

The fourth row is the one that indicts the slice rather than the code: this reader refuses
`<text:soft-page-break/>` **by name** and then put the same producer's page arithmetic into
`Node.text` anyway. `ODT_TEXT_CODE_RULE_V1`'s own doc comment already said *"a field's cached
rendering, a list's number and a footnote's mark are all produced by a layout this reader does not
perform"* — a sentence that was false about the code it names. That is v2-S3's pattern exactly:
three of its nine defects were in code whose own comment described the hazard it had.

**So the rule is inverted.** Character data reaches a block only when every element between them is
an allowlisted inline one — a span, a hyperlink, a ruby *base*. Everything else is foreign: its
characters are counted (**A14**, a new `foreign_text_not_read` bucket) rather than spliced. Two of
the five rows above were already fixed once, in another format, by another slice: `<rPh>` furigana
on both of v2-S3's string paths, and `<a:fld type="slidenum">` in v2-S4. **The rule existed; the new
reader did not apply it** — which is the shape of nearly every defect these reviews find.

Three further defects came from the same review and are fixed with it:

| | defect | why it was wrong |
| --- | --- | --- |
| 1 | element names matched by **suffix**, so a conforming `<xhtml:p>` advanced the block counter | ODF §3.17 permits foreign elements in mixed content, so this shifted every later address — and `xml.rs`'s "a refusal to find content rather than wrong content" trade was written for readers where the match selects *content*, not an *address*. This reader now resolves namespaces |
| 2 | MathML's `<annotation>` shares a local name with `<office:annotation>` | an inline formula was declared to the caller as an unread *reviewer's remark*, naming a gap the document does not have |
| 3 | `skip_from` was one slot, on the claim these regions "do not meaningfully nest" | they do — ODF puts a comment inside a footnote body — so two erased passages were declared as one, under-declaring, which is the direction A14 exists to prevent |

And `<text:s text:c>` is now read as the `positiveInteger` its schema says it is: `" 3"` is three
(whiteSpace `collapse`), `0` is refused rather than repaired to nothing — which was the `NameValue`
join this module's header says these elements exist to prevent — and `xmlns:c` is no longer mistaken
for `text:c`.

### What the same review found in the tests, which is the more uncomfortable half

Seven assertions did not test what they were named for, and the acceptance boxes below ticked
several of them off. The worst was the guard this slice leaned on hardest:

> `a_text_document_declares_no_pages_and_carries_no_page_number` proved the fixture "really
> contains" a `<text:soft-page-break/>` by grepping **`make_fixtures.py`** — where the literal also
> appears inside a `#` comment. Deleting the real element from the fixture left the test green.

It also asserted a property of a Python source file rather than of the bytes under test. It now
inflates the fixture's own `content.xml` and looks there, and deleting the element fails it.

The others, each now fixed and mutation-checked: the frame-stack test wrote its nested frame
self-closing, so the push/pop asymmetry it was named for was never created; `Event::CData` had no
ODT test while the box ticked "CDATA is matched rather than dropped"; the `stored` half of ODF
detection had none either, and the test's own comment conceded it (`build_zip` stores everything —
there is now a builder that deflates); `OdfBlockKind::Paragraph` was asserted nowhere, so the
mapping could have been a constant `Heading`; and `paragraphs.sort_by_key` was a no-op in every
test, because no test had a kept block nested inside a kept block.

**Twelve mutations now fail a test** across the repair. The pattern in the seven is one this
repository has named before and should expect again: *an assertion whose subject can be empty, or
whose evidence is a file other than the one under test, is not an assertion.*

### What reviewing the slice against §3 found before it shipped

Four defects, and every one of them is about **nesting** — which is the thing ODF does that no OOXML
format in v2 does. ODF puts a footnote's body, a comment's body and a text box's contents *inside*
the block they are anchored to, as their own `<text:p>` elements.

| | defect | why it was wrong |
| --- | --- | --- |
| 1 | a descendant walk concatenated a footnote into the sentence citing it | `Cited hereA source nobody read. and continued.` — a sealed, error-free, byte-identical artifact stating a phrase the document does not contain |
| 2 | fixing 1 made the footnote's `</text:p>` close the **anchoring** block | the rest of the sentence after the footnote was silently lost: `Cited here` with ` and continued.` gone |
| 3 | the block counter advanced only for blocks that were read | every address after a footnote was one short, so a citation landed on the wrong paragraph |
| 4 | `<draw:frame>` was pushed under the skip guard and popped without one | a frame inside a footnote popped an **enclosing** frame's flag, and the enclosing frame's first text box then read as a second rendition and was declared unread |

Defects 1 and 2 are the pair worth keeping: **the fix for a splice introduced a truncation**, in the
same five lines, and only a test that asserted the whole sentence caught the second. Defect 4 came
out of reading the reader back against itself rather than from a failing test — no fixture nests a
frame inside a passed-over region — which is the review S3 and S4 both did and the reason it is done
here too. All four are fixed and mutation-checked.

One thing worth naming that is **not** a defect: `Node.ordinal` and `OdtLocator.paragraph`
disagreeing looks like one on first read, and is the design. The artifact carries both, and the
table above is why.

### What this slice could not do, stated rather than skipped

**No corpus of real `.odt` files was available, and no ODF producer was either.** S3 and S4 both
changed a design after measuring real files — shape ids are not unique, `<mc:AlternateContent>` is
on half the slides — and that method was not available here. Every rule in `odt.rs` is read off the
OpenDocument specification and pinned against fixtures this repository authored.

Where that left a judgement call it is made toward **over-declaring**: a region this reader is
unsure about is counted as unread rather than concatenated into a paragraph, so the failure mode is
a phrase declared missing rather than a phrase invented. The frame-alternative rule is the clearest
case — it is spec-derived and fixture-tested, not measured against files in the wild, and **it is
still unmeasured in the wild today**. **S6 measures it** on authored ODS XML over the same
container, which is the nearest honest thing available; measuring it against real ODF producers
needs a corpus this repository does not have.

- **In:** `crates/engine-office/{odt.rs, lib.rs, zip.rs}` — `zip::first_entry`, the four-way router
  and `read_odt`; `OdtLocator`, `NativeLocator::Odt`, `NodeAttributes::OfficeParagraph`,
  `OfficeParagraphAttributes`, `OdfBlockKind`, `Profile::odt_v0`, `ODT_READING_ORDER_RULE_V1` and
  `ODT_TEXT_CODE_RULE_V1` in `engine-core`; content dispatch in `engine extract`;
  `fixtures/office/text-paragraphs` and `text-unread-parts` with their generator; `0.24.0`, the
  moved profile hash and both SDK pins; `PUBLIC-API.md` and its gate; `14`/`15`; CHANGELOG; README.

- **Out:** ODS — S6; ODP, RTF, EPUB, CSV — S7. Styles as evidence, tracked changes as a second
  authority, embedded objects, `text:outline-level`, headers and footers beyond the A14 count. Any
  change to `ethos.grounding.v1`. Any `project()` change. Markdown or HTML for an ODT. New MCP
  tools, new SDK functions. A `coordinate_system` mode enum — v2-S3 closed that and this slice did
  not reopen it. Any new dependency: no `zip`, no ODF crate, no LibreOffice.

- **Acceptance tests:**
  - [x] A known phrase is on a node with an `OdtLocator`; `pages` is `[]`; `node_get` over
        **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator's field set is exactly `{part, paragraph}`, and `deny_unknown_fields` refuses
        `page`, `bbox`, `x` and `soft_page_break` **by name**
  - [x] The fixture **contains** a `<text:soft-page-break/>` — asserted by inflating the fixture's
        own `content.xml`, **not** by grepping the generator, which is how the first version of this
        guard passed on a `#` comment — so the empty `pages` vector is a refusal rather than an
        absence. Mutation-checked by deleting the element from the fixture
  - [x] The block count advances **through** a footnote and a comment, so the blocks after them are
        3 and 5 rather than 2 and 3; a self-closing `<text:p/>` counts, so the address does not turn
        on how the document was serialized. Both mutation-checked
  - [x] A sentence split by a `<text:span>` is **one** node, so the sentence a reader quotes binds
  - [x] `&amp;` survives; **CDATA is matched rather than dropped, held by its own test**;
        `<text:s text:c="3">` is the three spaces the file states, `" 3"` is also three because the
        attribute's schema type collapses whitespace, and a non-numeric *or zero* count is a named
        refusal; ODF's whitespace rule is applied and the three stated characters are exempt — each
        mutation-checked
  - [x] A nested block does not leak into the block that anchors it, in both directions: the
        footnote's text is not in the sentence, and the sentence after the footnote is not lost
  - [x] Only the first `<draw:text-box>` of a `<draw:frame>` is read — claimed on the self-closing
        path too, so the artifact does not turn on whether the producer wrote `<draw:text-box/>` —
        and a passed-over rendition is counted **when it held a block with characters in it**, so a
        comment carrying only its author's name is not declared an erasure. Mutation-checked
  - [x] Character data reaches a block only through an **allowlisted inline element**: an image's
        title and description, an embedded object's base64, a generated `<text:number>`, a field's
        cached value and `<text:ruby-text>` furigana are each counted rather than spliced into the
        sentence — five tests, each mutation-checked
  - [x] Element names are **namespace-resolved**, so a foreign `<xhtml:p>` moves no address and
        MathML's `<annotation>` is not declared as a comment — mutation-checked
  - [x] Nested passed-over regions count separately: a comment inside a footnote is two erasures
  - [x] Both block kinds reach the artifact, and document order survives nesting — the sort was a
        no-op in every earlier test
  - [x] Detection is content-based: a renamed document reads, an `.odt` that is not one fails with
        empty stdout, an `.ods`/`.odp`/`.odg` is **never** claimed, and a package that is two
        formats is refused by name
  - [x] The `mimetype` entry is checked **first and stored**, both halves held by their own test —
        the ordering one over a reordered package, the compression one over a package whose first
        entry is deflated, which needed a ZIP builder that can deflate because the old one stored
        everything and said so
  - [x] "First" means the **archive's leading bytes**, not its index: a conforming package whose
        central directory is written in name order still reads, where reading directory order
        refused a genuine `.odt` and sent it to the PDF reader
  - [x] A package that lists one entry twice is refused, and so is a manifest that declares
        `content.xml` twice — which otherwise let an unencrypted declaration hide an encrypted one
  - [x] The manifest is consulted: an undeclared `content.xml` and an **encrypted** one are each a
        named refusal, and so is a package with no manifest — mutation-checked
  - [x] Unread package entries **and** unread regions are counted and declared, each held by its own
        test, the second by a package authored inside the test; the clean fixture declares none
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the law
        — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; all **five** profile hashes are mutually
        distinct; `capabilities.tables` is false
  - [x] Two runs over one document produce identical bytes
  - [x] `Cargo.lock` gains **no new dependency** — no `zip`, `zopfli`, ODF crate or renderer
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0
  - [x] Workspace **0.24.0**, both SDKs **0.24.0**, profile hash
        `sha256:dc89ca65af172b8d9537d96b0579dcf2c3fdf7fe8e0200bf85282fc9d1b6cd67`

- **Depends on:** S4.

---

## S6 — ODS → representation — **not started**

- **Goal:** OpenDocument **spreadsheet**, on the terms the first four established. One format. Not
  "the ODF family", not "the rest of the office formats".

- **Why it is one format, written down before the reader exists.** S5 measured what the first
  non-OOXML format cost and found that only the container transferred. An `.ods` inherits that
  container and nothing above it: `<table:table-cell>` is a different reader from `<text:p>`, with a
  different atom, a different locator and a different question about what a sheet is. The scope is
  fixed here, ahead of the code, for the reason S0 and S1 both used — *so the first implementation
  cannot quietly acquire a second format "while we're in here" while nobody has written down that
  it may not.* **ODP is S7's**, and being one `zip::first_entry` call away does not move it.

- **What S5 hands over, and S6 owes back.**

  | Handoff | What it is | What S6 owes |
  | --- | --- | --- |
  | **An `.ods` is refused with a message about PDF** | `is_odt` correctly declines it, nothing else claims it, so it falls through to the PDF reader and gets *"expected a PDF header (%PDF-) at byte 0"*. It fails closed — empty stdout, exit 2 — but names the wrong cause | S6 fixes it, because S6 is where the ODF family's named refusal can be written without inventing a claim about a format nothing reads. It is not a "while we're here": it is the one thing S5 deferred **to** this slice |
  | **The frame-alternative rule is unmeasured in the wild** | `odt.rs` reads only the first `<draw:text-box>` of a `<draw:frame>`. Spec-derived, fixture-tested, never seen against a real ODF producer | S6 measures it on **authored ODS XML** over the same container. That is the nearest honest thing available; a real-producer corpus does not exist here, and S6 says so rather than implying it measured one |

- **The one thing already known about the shape:** a spreadsheet's page is a printer's, which v2-S3
  settled for XLSX and §3's law settles again here. Whatever an `.ods` says about print ranges or
  paper, `pages` stays `[]` and the locator carries no page.

- **Not required for v2's gate.** The gate names a DOCX quote and an XLSX cell, and both bind. This
  slice, and S5 before it, is coverage beyond it. **Nothing here closes v1**, whose table chase is
  parked rather than passed.

- **Depends on:** S5.

---

## S7 — the remaining office formats — **not started**

- **Goal:** ODP, RTF, EPUB and CSV, on the terms the first five established.

- **Still deliberately one row, and S5 and S6 are why it is a shorter one.** **A1** — Anydoc's
  14-format coverage — is v2's horizon, not its checklist. S4 measured that a fourth *OOXML* format
  would be cheap. **S5 measured what the first non-OOXML format actually cost**, and the answer is
  that ODF's container was free and everything above it was not: a new vocabulary, a new atom, a new
  detection question and a new class of nested-block defect. That is why ODS left this row and
  became S6. ODP inherits the container work and none of the rest; RTF is not XML at all; EPUB is a
  ZIP of XHTML; CSV has no container. Scheduling them as one slice each before any of them has been
  looked at would still be the waterfall S0 refused — and this row splits again the same way the
  last two did, **against a measurement**, not against an estimate.

- **The one thing already known about this row:** EPUB may genuinely have pages and CSV genuinely
  has none, so §3's law is not "no page ever" but "no page this engine did not read from the
  file." Whichever formats have a native pagination declare it; the rest carry the empty vector.
  S4 and S5 are the precedent for the first half: a slide looked like a page and was a part, and an
  ODT's soft page break *is* a page break and is still somebody else's.

- **Not required for v2's gate.** The gate names a DOCX quote and an XLSX cell, and both bind.
  This row is coverage beyond it.

- **Depends on:** S6.

---

## Standing rules for every v2 slice

Carried from `08-V1-SCOPE.md` §6, `10-V11-SCOPE.md` §8, `12-V12-SCOPE.md` §8 and `14-V2-SCOPE.md`
§9:

1. **No public confidence field** — including on an office node or in a summary string
2. **No box, and no role, derived from a font size** — and **no page derived from anything**
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count
   (**A14**, declared erasure)
4. **No invented coordinate, identifier, fingerprint or pagination** — and in v2 pagination is the
   leading case, not the footnote
5. **A gap is never presented as a success** — a quote that cannot bind fails closed
6. **Byte-identity across runs**, and across formats
7. **No renderer, no converter, no JVM, no AGPL** — `deny.toml` is the standing proof
