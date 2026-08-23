# 15 — v2 slices

**Status:** implementation authority for v2 · **Scope document:** `14-V2-SCOPE.md`
**This is the code-review map for v2.** Every v2 PR belongs to exactly one slice.

**v2 reads eight formats, and v2's format row is closed.** S0 through S15 are **done** — eight formats
read and **S10 (CSV) an argued refusal rather than a reader**. `engine-office` is
the fifth crate, DOCX is the format that stopped it being speculative, XLSX is the one that made the
page-less invariant carry more than one part, PPTX is the one that tested whether a part this
engine *can* count would become a page, ODT is the one whose file **contains an actual page
break**, ODP is the one that would have handed over a `PageRecord` for **free** — discrete `<draw:page>`
elements and a master page's `fo:page-width`, no arithmetic anywhere — RTF is the one that writes
`\page` outright and has no container to hang an address on, and **EPUB is the one whose file may
genuinely name pages**: a navigation document's `page-list` gives the page numbers of a print
edition. None of them became a page, and the last one needed an argument rather than a rule.

**The remaining-formats row split six times, each time against a measurement.** S0 wrote it as one
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
ODP is nearly free" gets said. So **S6 was ODS alone** and **S7 was the one row that was left** —
ODP, RTF, EPUB, CSV — and it stayed one row until the cost of the next format in it was measured, on
the rule S0 set and S4 and S5 both honoured.

**The fourth split is S7's, and it is the same rule applied to the same row.** S7 implements ODP
alone and leaves **RTF, EPUB and CSV** as **S8**. The measurement it had was S6's: ODF's container
transfers and everything above it does not, and ODP is inside the family while the other three are
not — RTF is not XML at all, EPUB is a ZIP of XHTML that shares OCF's container rule and is **not**
OpenDocument, and CSV has no container. Scheduling all four while somebody was already inside an
ODF reader is exactly the "while we're here" that S0 refused.

**The fifth split is S8's, and it is the measurement the row had been waiting for.** S8 implements
RTF alone and leaves **EPUB and CSV** as **S9**. What it measured is that RTF inherited *nothing*:
not the container, not the XML reader, not the allowlist, not even the shape of the locator — an
`.rtf` has no parts, so `check_structure`'s page-less invariant grew a fourth rule for it. A format
that shares no machinery with any of the seven before it is not a line item on somebody else's
slice.

**The sixth split is S9's, and it leaves the row one format long.** S9 implements EPUB alone and
leaves **CSV** as **S10**. EPUB inherited the ZIP reader and the XML plumbing and almost nothing
else: a new container chain (`container.xml` → a package document), a reading order the file
*states* rather than implies, a new element vocabulary, and a character-reference rule the shared
one could not supply. CSV inherits none of that — it has no container and, worse, **no detector**,
which is the whole of what S10 has to argue.

**The v2 gate is DOCX + XLSX, and both bind — which is now what the gate says.** ODT, ODS, ODP,
RTF and EPUB are coverage beyond it. The owner settled both of the questions this paragraph used to
carry on 2026-08-21, as `00-NORTH-STAR.md` decisions **#16** (*ground* means **bind**) and **#17**
(embedded assets **counted** satisfies v2); v2-S13.4 argues both. **The last question is answered:**
`zip.rs` verifies the CRC-32 it used to ignore, shipped at **v2-S14 (0.33.0)** on a measured
false-refusal rate of **zero**. No slice here closes v1.

**v2-S13.5 ran the two sweeps v2-S13.3 recorded as unrun**, closing this file's only unmet
acceptance box. It found that `07-VERIFY-BOUNDARY.md` had been claiming to hold this repository's
forced decisions *verbatim, identically* while holding fourteen of seventeen — missing exactly #15,
#16 and #17. Two clusters are named and deferred there rather than half-repaired, and two comments
were found naming tests that have never existed. **v2-S14.1 wrote those two guards** (0.33.1),
repaired the `four words` cluster at every live site, and re-checked S13.5's findings against the
code — where one of them, *"zero doc sites"* for `not_detected`, did not survive. **v2-S15 moved
the `neither detector` cluster** (0.34.0), all fifteen sites together because two are emitted wire
strings, which is why it is a minor and not a patch: an artifact this build emits differs from one
0.33.1 emitted for the same bytes.

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
| **S6** | ODS → representation: a spreadsheet the OpenDocument way | S5 | **done — and the address the file never writes** |
| **S7** | ODP → representation: draw pages, shapes and blocks | S6 | **done — and the page that was free** |
| **S8** | RTF → representation: a stream with no container | S7 | **done — and the address with no part** |
| **S9** | EPUB → representation: the spine, and the page a publisher named | S8 | **done — and §3's law argued rather than applied** |
| **S9.1** | The erasure counters that could wrap — a **repair**, not a format | S9 | **done — and a site list is not a search** |
| **S10** | CSV — **an argued refusal, not a reader** | S9.1 | **done — and the format nothing detects** |
| **S10.1** | The seven claims v2-S9.1 and v2-S10 left false — a **doc repair**, no version | S10 | **done — and it left itself out of this table** |
| **S10.2** | The docs that stopped describing the code — a **doc repair** at 0.29.1 | S10.1 | **done — and a site list is still not a search** |
| **S11** | **Embedded assets, counted** — the last undelivered v2 content obligation | S10.2 | **done — and a second bucket rather than a wider one** |
| **S12** | **The office readers get fuzzed** — `A11`'s deferred lane, six slices past its condition | S11 | **done — and the campaign found nothing, which is a result and not a pass** |

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

## S6 — ODS → representation

- **Status: done.** `0.25.0`. `engine extract` reads a `.ods`, a cell binds at the position the file
  states, and **an ODF sibling this engine does not read now says so about itself** — which is the
  one defect S5 deferred here.

- **Goal:** OpenDocument **spreadsheet**, on the terms the first four established. One format. Not
  "the ODF family", not "the rest of the office formats".

### The address this format does not write, which is why it was a slice

S5 measured that only ODF's *container* transferred from OOXML. This slice is the other half: the
container was free again — the stored-first `mimetype`, the manifest, the fixed `content.xml` name,
all reused unchanged — and the structure above it was not.

| what the format writes | what this reader does |
| --- | --- |
| `table:name` on `<table:table>` | carries it verbatim, entities resolved. **The only component of a cell's address ODF writes down** |
| nothing at all for a row or a column | counts position in document order |
| `table:number-columns-repeated="n"` on a cell | **honours it** — the file saying *"and n more of these"* is a statement of position, so reading it is reading |
| `table:covered-table-cell` | advances the cursor and yields no node: it occupies its columns and displays nothing |

`XlsxLocator` documents why counting would be *wrong* for a workbook — sheets are sparse, so a
counter would give a cell a different address than `<c r="B12">` gives it. That argument does not
transfer, because ODF writes no `r`. **The question here was never "read or count"**; it was *what
does this file state position with*, and the answer is document order plus the repeats. A reader
that ignored them would put every cell after the first compressed gap at the wrong column — and
every real producer writes such a gap on every row.

**The column is a number, and that is deliberate.** `XlsxLocator::column` is a `String` because the
workbook writes `B` down. A `.ods` contains no letters anywhere, so a letter here would be a
spreadsheet application's display convention rather than a string the document holds.

### One allowlist, not two

`odt.rs`'s `Element`, `classify`, namespace resolution and block-text engine are **imported**, not
restated. The list's whole content is *which inline names are the sentence*, and two copies of that
drift silently: a rule that gains `<text:page-count>` in one reader and not the other puts a word
processor's page arithmetic into one artifact and not the other, with nothing failing anywhere. The
ODS reader adds only what is above the paragraph — `table:table`, `table:table-row`,
`table:table-cell`, `table:covered-table-cell`, `table:shapes` — and everything else falls through.

`ods.rs` closed several gaps the shared engine did not have, each of the same shape — **text that
was neither read nor counted**, which is the one failure the allowlist exists to prevent. Character
data reaching a **cell** while no block is open (`<xhtml:table><xhtml:p>` is legal there); a cell
outside every row; a row outside every table; a covered cell carrying content the merge hides; and a
drawing's `<svg:title>`, which sits in no `<text:p>` at all and so escaped the block-only rule that
`odt.rs` applies to notes and comments for a reason of its own. All are counted now.

### The frame-alternative rule, measured — and it looks different here

S5 wrote first-rendition-wins off the specification and recorded plainly that **it had never been
measured against a document that nests two renditions**, because no such file was available. It is
measured here. ODF puts `<draw:frame>` in the `<text:p>` content model for **every** document type,
so a conforming `.ods` can hold a frame with two `<draw:text-box>` children — no half ODP reader was
needed and none was written.

**The result is not the one the slice was scoped to expect, and that is the finding.** In an ODT the
atom is the paragraph, so a frame's first rendition becomes nodes. Here the atom is the **cell**, and
a frame floats over the sheet: its words belong to no cell, so there is no address at which *"one
displayed phrase becomes one node"* could be true. The first implementation merged them into the
anchoring cell, and that was a **mis-attribution** — the artifact put a phrase at `Sheet1 row 1
column 2` that a person reading the document does not find there. A `<draw:frame>` inside a cell is
therefore a declared region, the same answer `<table:shapes>` already got.

What survives the atom change, and is what this slice actually measured:

| construct | result |
| --- | --- |
| frame with **one** rendition | cell keeps its own text · **1** declared erasure |
| frame with **two** renditions | cell keeps its own text · **2** declared erasures |
| first rendition self-closing | the empty first still claims the frame's slot · **1** erasure |
| `<table:table>` nested in a frame | **not minted as a sheet** — an alternative rendition is not a document's table |

So the rule is exercised: the first rendition and the second are accounted separately. **The ODT
rule is unchanged**, and the half of it that does not transfer is written down here rather than
asserted as though it had passed.

### The `%PDF-` message, fixed

S5 recorded it: an `.ods` fell past the office branch to the PDF reader and was refused with
*"expected a PDF header (%PDF-) at byte 0"* — fail-closed, and naming the wrong cause.

The fix is in two places and neither is a special case. `engine_office::is_opendocument` asks the
**family** question — *is the office reader the one to ask* — which only an ODF package can answer
about itself, and the CLI dispatches on that instead of on `is_odt`. Then `read` refuses a declared
ODF type it does not implement, by name, before the router runs. An `.odp` now names itself.

### What could not be measured, recorded

**No corpus of real `.ods` files was available and no ODF producer was either** — the same statement
S5 had to make, repeated rather than quietly inherited. Every rule is read off the OpenDocument
specification and pinned against packages this repository authors byte by byte. Two fixtures:
`sheet-cells` consumes every entry it contains and declares **no** erasure, and `sheet-unread-parts`
declares all three kinds.

- **In:** `crates/engine-office/{ods.rs, lib.rs}` — `read_content`, `Cell`, `Sheets`, `is_ods`,
  `is_opendocument`, `read_ods` and the unimplemented-ODF refusal; `odt.rs`'s text engine widened to
  `pub(crate)` with **no logic change**; `OdsLocator`, `NativeLocator::Ods`,
  `NodeAttributes::OfficeOdfCell`, `OfficeOdfCellAttributes`, `OdfValueType`, `OdfCellTextSource`,
  `Profile::ods_v0`, `ODS_READING_ORDER_RULE_V1` and `ODS_TEXT_CODE_RULE_V1` in `engine-core`; the
  ODF family question in `engine extract`; `fixtures/office/sheet-cells` and `sheet-unread-parts`
  with their generator; `0.25.0`, the moved profile hash and both SDK pins; `PUBLIC-API.md` and its
  gate; `14`/`15`; `CAPABILITY.md`; CHANGELOG; README.

- **Out:** ODP, RTF, EPUB, CSV — **S7**. Styles as evidence, `office:value` as a second text source,
  number formats, embedded objects, charts, named expressions, data pilots. Any change to
  `ethos.grounding.v1`. Markdown or HTML for an ODS. New MCP tools, new SDK functions. A
  `coordinate_system` mode enum — v2-S3 closed that and this slice did not reopen it. Any new
  dependency: no `zip`, no `calamine`, no ODF crate, no LibreOffice. Any PDF detector change, and
  any move on the parked 0.489 chase.

- **Acceptance tests:**
  - [x] A known cell is on a node with an `OdsLocator`; `pages` is `[]`; `tables` is `[]`;
        `node_get` over **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator's field set is exactly `{part, table, row, column}`, and `deny_unknown_fields`
        refuses `page`, `bbox`, `x`, `print_page` and `soft_page_break` **by name**
  - [x] **`Total` is at column 5**, because three repeated columns sit between it and the first
        cell — asserted after inflating the fixture's own `content.xml` to show the attribute is
        really there. `<table:table-cell/>` and its long form agree, so serialization moves no
        address
  - [x] A repeated **row** is the rows the file states; an empty repeat of 1 048 576 costs one
        addition; a text-bearing repeat past the cap is a **named refusal**, and a repeat that is
        not a positive count is refused rather than guessed
  - [x] **The aggregate cell cap binds during the expansion**, not once per XML event — a document
        at both per-axis limits is refused by the million-cell cap rather than allocating 1.83 GB
        first. A cursor that would run past what an index holds is **refused**, never clamped: two
        cells sharing one address is the confidently-wrong locator the contract forbids
  - [x] Text with no address is **declared, never dropped** — a cell outside every row, a row
        outside every table, and a covered cell that carries content the merge hides
  - [x] A covered cell occupies its columns and yields no node
  - [x] The table name keeps its `&amp;` — a dropped entity there is a **wrong address**, not
        merely wrong text — and two tables of one name are refused
  - [x] Every `office:value-type` ODF declares is read, a cell declaring none is `Void`, and an
        unknown one is a named refusal. A `table:formula` labels the text **cached**, and no formula
        source reaches any node
  - [x] Two paragraphs in one cell are two lines of **one** node, joined in document order — a
        nested block closes first, and the join is by open order rather than close order
  - [x] The allowlist holds through a cell: an image's title and description, an embedded object's
        base64, a generated `<text:number>`, a **cached `<text:page-count>` and `<text:page-number>`**
        and ruby guide text are each **absent** from `Node.text` and **present** in the A14 count —
        asserted by inflating the fixture ZIP, never by grepping the generator
  - [x] Namespaces are resolved: a foreign `<xhtml:table>` is not a table, a foreign `<xhtml:p>` is
        not a block, and MathML's `<annotation>` is not `<office:annotation>`
  - [x] **The frame-alternative rule is measured** on authored ODS XML: two renditions declare two
        erasures where one declares one, and the anchoring cell keeps its own text. Mutation-checked
        — removing the second rendition from the **bytes** lowers the declared count. What does not
        transfer from ODT is recorded above rather than asserted
  - [x] `<table:shapes>` text and text with no cell to belong to are counted, not dropped
  - [x] A `<text:soft-page-break/>` inside a cell contributes no character, and `pages` stays `[]`
  - [x] Detection is content-based and **exact against real packages**: `…spreadsheet` is claimed,
        `…spreadsheet-template` is not, an `.odt` is not, and a renamed spreadsheet still reads.
        `is_opendocument` is the ODF **namespace**, not the container rule — an `.epub` uses the same
        first-and-stored `mimetype` entry and must not be told it is OpenDocument
  - [x] An unimplemented ODF type (`.odp`, `.odg`, `.odf`) is refused naming **OpenDocument** and
        the declared type, and **not** naming `%PDF-` — *the `.odp` row moved to S7, which reads
        it; `.odg` and `.odf` still take this path*
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the law
        — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; all **six** profile hashes are mutually
        distinct; `capabilities.tables` is false
  - [x] Two runs over one document produce identical bytes, for both fixtures
  - [x] `Cargo.lock` gains **no new dependency** — no `zip`, no `calamine`, no ODF crate
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0; the 0.489 chase still parked
  - [x] Workspace **0.25.0**, both SDKs **0.25.0**, profile hash
        `sha256:bdd835b47ac40b05928375961f832878e4134171ab6f2bdf436c6f531dfb89aa`

- **Depends on:** S5.

---

## S7 — ODP → representation

- **Status: done.** `0.26.0`. `engine extract` reads an `.odp`, a block binds on the draw page and
  shape the file lists, and **the one format that could have handed this engine a page for free
  still declares none**.

- **Goal:** OpenDocument **presentation**, on the terms the first five established. One format.
  Not "the ODF family", not "the rest of the office formats".

### The page that was free, and is still refused

Every page-less format before this one had to *argue* that its page belonged to somebody else, and
each argument had a step in it. A DOCX has no page until a renderer picks one. A workbook's page
depends on the printer, the paper and a "fit to page" setting. A PPTX slide is a **part**, and
`p:sldSz` is a size rather than a page. An ODT contains a `<text:soft-page-break/>` — but it is a
position a word processor computed from its own font stack.

**A presentation needs no argument at all, and that is what makes it the sharpest case in v2.**

| the tempting field | what it actually is |
| --- | --- |
| `<draw:page>` → `PageRecord.index` | display **order** in `content.xml`, which a consumer would read as a page number |
| a master's `fo:page-width` / `fo:page-height` | paper the authoring tool wrote; this engine measured nothing against it |
| cached `<text:page-number>` / `<text:page-count>` | producer arithmetic; the shared allowlist already refuses these as text |

`<draw:page>` elements are discrete, listed, ordered and named, and a `<style:master-page>` states
paper beside them. A `PageRecord` needed **no arithmetic** — the first time that has been true in
this engine's history. It is refused on `docs/06-STEAL-REFUSE.md` L30's own four words rather than
a paraphrase of them: *"It invents pagination."* L30 refuses the LibreOffice bridge because a page
it produced is a rendering rather than a fact about the document, and a `<draw:page>` fails the same
test from the other side — it is **a part of the presentation's structure**, and the number a
consumer would read off it is a page index this engine never verified.

So `pages` is `[]`, `is_paginated()` is false, and the address carries a `draw_page` **position**
named for the element rather than for the thing it resembles.

### The address, and the two names that are not in it

`OdpLocator { part, draw_page, shape, paragraph }` — four positions in the part's own document
order, `deny_unknown_fields`, and a test that refuses `page`, `bbox`, `x`, `slide_number` and
`soft_page_break` **by name**.

| component | what the file states | what this reader does |
| --- | --- | --- |
| `part` | `content.xml`, fixed by the package specification. **One part for the whole deck**, unlike `ppt/slides/slide{n}.xml` | carries it verbatim, and still checks the manifest declares it |
| `draw_page` | nothing numeric — the pages are simply listed | counts `<draw:page>` elements in document order |
| `shape` | nothing numeric | counts the shapes **this slice names** — `<draw:frame>` and `<draw:custom-shape>` — within the draw page |
| `paragraph` | nothing numeric | counts `<text:p>` / `<text:h>` within the shape, advancing through what it does not read |

**This is where `PptxLocator`'s argument stops transferring.** S4 kept a slide's identity in the
*part name*, because `p:sldIdLst` states display order and a part name does not move. An `.odp` has
**one** part for every draw page, so there is no part name to lean on — which is why this locator
carries a component `PptxLocator` deliberately does not, and why that component had to be a
position rather than a number the file wrote.

**Both `draw:name`s are labels, not the address, and that is v2-S4's finding applied rather than
quoted.** `<draw:page draw:name>` and `<draw:frame draw:name>` are *optional* in OpenDocument, and
**no corpus of real `.odp` files was available to measure whether producers write them uniquely**.
S4 measured the OOXML counterpart across 18 real decks — `<p:cNvPr id>` is present every time and
unique only most of the time — and moved it onto the attributes. An unmeasured identifier gets the
same treatment rather than the benefit of the doubt, so both names are on
`OfficeOdfShapeAttributes`, where being a label is exactly what they are.

**The shape set is named, and the consequence is stated rather than hidden.** `<draw:frame>` and
`<draw:custom-shape>` are what a presentation writes for a text-bearing shape; a `<draw:rect>` or a
`<draw:connector>` carrying text does **not** move the shape count and does **not** become a node —
its text is declared instead. Naming the set is what makes the position reproducible: a consumer
counting those two elements arrives at the same number this reader did. Widening the set later
would move every address, which is what `parser_version` and the profile hash exist to make visible.

### The frame-alternative rule, measured a third time — and the atom decides again

S5 wrote first-rendition-wins off the specification and could not test it. S6 tested it on a
spreadsheet and found **the atom decides the outcome**. S7 is the third data point, and it is worth
one table:

| slice | the atom | a `<draw:frame>` with two `<draw:text-box>` children | why |
| --- | --- | --- | --- |
| **S5 — ODT** | the paragraph | first rendition **becomes nodes**; second is **1** erasure | the frame is anchored *in* a paragraph, and paragraphs are what this reader addresses |
| **S6 — ODS** | the **cell** | **neither** becomes a node; **2** erasures, and the anchoring cell keeps its own text | a frame floats over the sheet: its words belong to no cell, so there is no address at which "one displayed phrase becomes one node" could be true |
| **S7 — ODP** | the block **inside the shape** | first rendition **becomes nodes**; second is **1** erasure | a presentation *is* a drawing, so there is nothing to float over — the frame **is** the shape this reader addresses |

**The outcome matches ODT and the reason matches neither.** In an ODT a frame is a floating object
inside a stream of text; in an ODP it is the text. Recording that distinction is the point of the
table: a later reader that copies "frames are never nodes" from S6, or "frames are always nodes"
from S5, will be right for the wrong reason in one of the three and wrong in the other.

Measured on authored ODP XML and mutation-checked on the fixture's **bytes**: deleting the second
`<draw:text-box>` from the package lowers the declared region count, and so does deleting the
speaker notes. A **self-closing** first rendition still claims the slot — the construct S5 and S6
both got wrong when their fixtures never nested one.

### Speaker notes are A14 inverted, so they are counted rather than spliced

`<presentation:notes>` holds a whole second page of shapes. Reading it as slide text would not be a
silent **drop** — it would be a silent **extra**, putting a phrase in the record that nobody
watching the presentation sees, which is worse because a consumer cannot tell it apart from
evidence. It is a declared region with a count, and the A14 message names it first.

Masters, layouts and handouts live in `styles.xml`, which is not read at all: they are unread
package entries, and the fixture puts `MASTER-PAGE-TEXT` in one so the test that no master text
reaches a node is not vacuous. That is v2-S4's *"no placeholder inheritance is resolved"* in ODF's
spelling, and it means **a slide whose title lives only on its master reads as having none** —
stated here rather than discovered by a caller.

### One allowlist, still not two — and the silent drop this format added

`odt.rs`'s `Element`, `classify`, namespace resolution and block-text engine are **imported** for
the third reader, on S6's argument unchanged. `odp.rs` adds only what sits above the block:
`draw:page`, the two shapes, and `presentation:notes`.

Building it that way surfaced one gap neither earlier ODF reader had, and it is the **ordinary**
shape of a presentation rather than an edge case: `<draw:frame><draw:image><svg:title>` contains no
`<text:p>` **anywhere in it**, so the shared engine's per-block foreign counter never sees it and
the characters were neither read nor counted. In an ODT and an ODS that subtree always sits inside
a paragraph or a cell; in a presentation the shape is the thing and a block is only one of the
things inside it. It is counted at the **shape** now. Text loose on a draw page, and text in a
drawing element this slice does not name, are counted for the same reason.

The namespace-resolved attribute matcher moved from `ods.rs` to `xml.rs` rather than being restated
— the same argument as the allowlist, in a smaller place.

### What could not be measured, recorded

**No corpus of real `.odp` files was available and no ODF producer was either** — the third
consecutive slice that has to say so, repeated rather than quietly inherited. Every rule is read off
the OpenDocument specification and pinned against packages this repository authors byte by byte.
The consequence is written into the design rather than left implicit: `draw:name` could not be
measured unique, so it is never the address.

Two fixtures: `presentation-pages` consumes every entry it contains and declares **no** erasure, and
`presentation-unread-parts` declares all three kinds.

- **In:** `crates/engine-office/{odp.rs, lib.rs}` — `read_content`, `TextBlock`, `Presentation`,
  `is_odp`, `read_odp`, `ODP_MEDIA_TYPE` and the widened unimplemented-ODF refusal; `ods.rs`'s
  attribute matcher moved to `xml.rs` with **no logic change**; `OdpLocator`,
  `NativeLocator::Odp`, `NodeAttributes::OfficeOdfShape`, `OfficeOdfShapeAttributes`,
  `Profile::odp_v0`, `ODP_READING_ORDER_RULE_V1` and `ODP_TEXT_CODE_RULE_V1` in `engine-core`;
  `fixtures/office/presentation-pages` and `presentation-unread-parts` with their generator;
  `0.26.0`, the moved profile hash and both SDK pins; `PUBLIC-API.md` and its gate; `14`/`15`;
  `CAPABILITY.md`; CHANGELOG; README.

- **Out:** RTF, EPUB, CSV — **S8**. Animations, transitions, `presentation:class` as a role, master
  and layout text as body, notes as evidence, embedded spreadsheets and charts as evidence, theme
  fonts, shape **geometry** as a locator, `.odg`. Any change to `ethos.grounding.v1`. Markdown or
  HTML for an ODP. New MCP tools, new SDK functions. A `coordinate_system` mode enum — v2-S3 closed
  that and this slice did not reopen it. Any new dependency: no `zip`, no ODF crate, no
  LibreOffice. Any PDF detector change, and any move on the parked 0.489 chase.

- **Acceptance tests:**
  - [x] A known title is on a node with an `OdpLocator`; `pages` is `[]`; `tables` is `[]`;
        `node_get` over **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator's field set is exactly `{part, draw_page, shape, paragraph}`, and
        `deny_unknown_fields` refuses `page`, `bbox`, `x`, `slide_number` and `soft_page_break`
        **by name**
  - [x] **`<draw:page>` did not become a `PageRecord`**, and the test is not vacuous: the fixture's
        own `content.xml` is inflated and asserted to list **two** of them, and the second
        package's `styles.xml` to carry a `<style:master-page>` with `fo:page-width`
  - [x] The serialization moves no address: `<draw:frame/>`, `<text:p/>` and `<draw:text-box/>`
        each hold their position, asserted against the fixture's own bytes
  - [x] Both `draw:name`s are on the attributes and never in the locator, and `&amp;` survives in
        visible text **and** in both of them
  - [x] Document-order counters advance through skipped regions and self-closing empties, and a
        nested shape's blocks emit in the part's order rather than in close order
  - [x] **The frame-alternative rule is measured** on authored ODP XML: the first rendition becomes
        nodes and the second is one declared erasure, an empty first still claims the slot, and the
        result is recorded against ODT **and** ODS in one table. Mutation-checked — removing the
        second rendition from the **bytes** lowers the declared count, and so does removing the
        speaker notes
  - [x] Speaker notes, a comment, an unnamed drawing shape and a master page's text are each
        **absent** from `Node.text` and **present** in a count
  - [x] The allowlist holds through a shape: an image's title and description, an embedded object's
        base64, a generated `<text:number>`, a cached `<text:page-count>` and `<text:page-number>`
        and ruby guide text are each absent from `Node.text` and present in the A14 count — asserted
        by inflating the fixture ZIP, never by grepping the generator. **Including the case with no
        block open at all**, which is this format's own silent drop
  - [x] Namespaces are resolved: a foreign `<xhtml:p>` is not a block, and MathML's `<annotation>`
        is not `<office:annotation>`
  - [x] A `<text:soft-page-break/>` inside a shape's paragraph contributes no character, and
        `pages` stays `[]`
  - [x] Detection is content-based and **exact**: `…presentation` is claimed,
        `…presentation-template` is not, an `.odt` and an `.ods` are not, an `.epub` is not
        OpenDocument at all, and a renamed presentation still reads
  - [x] **An `.odg` is refused naming OpenDocument and the declared type**, and not naming `%PDF-`
        — the sharpest case in the family, because its `content.xml` really is `<draw:page>`
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the law
        — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; all **seven** profile hashes are mutually
        distinct; `capabilities.tables` is false
  - [x] Two runs over one document produce identical bytes, for both fixtures
  - [x] `Cargo.lock` gains **no new dependency** — no `zip`, no ODF crate, no renderer
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0; the 0.489 chase still parked
  - [x] Workspace **0.26.0**, both SDKs **0.26.0**, and the PDF profile hash moved on
        `parser_version` **alone** to `sha256:ba367599bc4649f6ca71c45f71dd7b71c22a26f8a4997cb0e39544afdde02773`

- **Depends on:** S6.

---

## S8 — RTF → representation

- **Status: done.** `0.27.0`. `engine extract` reads an `.rtf`, a paragraph binds at the position
  the stream states, and **the first v2 format with no container at all** got there without
  inventing one.

- **Goal:** Rich Text Format, on the terms the first six established. One format. Not "the rest of
  the office formats".

### Not a package, and that is what cost something

Every reader in `engine-office` before this one opens by asking a **container** a question: does
the central directory list `word/document.xml`, does the first stored entry declare an OpenDocument
type, which part does this `r:id` resolve to. An `.rtf` has none of that. It is one sequence of
bytes — `{`, `}`, control words beginning with `\`, and everything else is text — with no manifest,
no parts, and no name the document has for itself.

So `RtfLocator` carries **one** field, and the tempting move was the other one: a constant part
name would have let `check_structure`'s page-less shape run unchanged, because that check proves
one part id means one part name. It would also have put a string in every citation that the
document does not contain, which is `14-V2-SCOPE.md` §3's *"absent, not invented"* in a smaller
place than the page-sized box that obligation is usually about.

**The invariant grew a fourth rule instead.** A locator now answers `names_a_part`, and one that
answers false is checked on the only integrity claim its format can make — **one document, one
container, one id** — while mixing the two shapes in one artifact is refused by name. The overload
that had to be separated first is worth recording: until this slice, `part()` returning `None`
meant *"paginated"*, because every page-less format so far was a package. RTF is neither.

### The page, said out loud

| the tempting field | what it actually is |
| --- | --- |
| `\page` → a `PageRecord` boundary | where the **producing application** broke a page |
| `\paperw` / `\paperh` | paper the authoring tool wrote; this engine measured nothing against it |
| a `{\field{\*\fldinst PAGE }}`'s cached result | producer arithmetic, and the field is skipped whole |

An ODT hides its page inside `<text:soft-page-break/>` and an ODP inside a `<draw:page>` element.
RTF writes `\page`. It is matched, contributes no character, and produces no `PageRecord`;
`pages` is `[]` and the locator has one field with no room for a second. L30 refuses invented
pagination whether inventing it costs a renderer or costs nothing, and this is the fourth format in
a row where the file says the word and the artifact does not.

### The atom, and what a consumer counts

A paragraph. `\par` ends one, and so do `\sect`, `\cell` and `\row` — recorded on
`RtfParagraphAttributes::terminator` rather than flattened, because **which** of them it was is a
fact the stream states and `\cell` is how a consumer learns the text sat in a table.

**Spans are not the atom, for `OdtLocator`'s reason.** `{\b important}` is formatting, and a
paragraph may contain no formatting group at all — addressing by run would leave the commonest case
with no address to give.

**The count advances through destinations this reader does not read.** A `\par` inside a
`{\footer …}` moves it, so the fixture's addressed paragraphs are `1, 4, 5, 6, 7, 8` with the
header's and the footer's own paragraphs occupying 2 and 3. That is `OdtLocator::paragraph`'s rule
in RTF's spelling, and the reason is the same: the number promises a position in the file, so it
has to be the position a consumer counting paragraph breaks in the bytes would find, not a position
in the subset this slice kept.

### Skip unless transparent — the inverted allowlist, outside XML

`odt.rs` inverted its rule because ODF puts a great deal of non-displayed character data inside a
`<text:p>`. RTF has the same hazard in a different shape: a group's text belongs to the body only
if the group is **formatting**, and a group whose first control word names a *destination* holds
something else entirely.

There is no way to enumerate every destination a producer might write, so the rule is inverted the
same way: a group is transparent only when its first control word is in a **deliberately short and
closed list**, and everything else is skipped and counted. `{\*\…}` is skipped without consulting
the list at all, because `\*` is the format's own marker for *"a destination a reader may ignore"*
and honouring it is reading the file.

The direction is the one every slice since v2-S5 has committed to, and it matters most here:
over-skipping reports a phrase missing, which the artifact declares; under-skipping puts a
`{\footer …}`'s words in the body, which is **A14 inverted** — a silent *extra* a consumer cannot
tell from evidence.

**One defect worth recording**, because it made the whole rule silently off. The first version
marked the **new** group decided when a `{` opened, so every group was already decided by the time
its first control word arrived and every destination read as transparent. The unit tests this list
exists for caught it, and the fixed code marks the *enclosing* group instead.

### Characters, and the code page this reader does not have

Read: plain 7-bit characters, `\uN` as the scalar it names — surrogate pairs combined, because a
producer writes an astral scalar as two of them — and a small closed set of special-character
control words (`\tab`, `\emdash`, `\lquote`, …) that each stand for exactly one character.
`\ucN` says how many fallback characters follow a `\u`, and reading them as well would put the same
character in the record twice.

**`\'hh` above 0x7F is declared, never guessed.** That byte's meaning depends on `\ansicpg1252`,
`\ansicpg932` or another declaration, and this reader carries no table for one. Emitting a Latin-1
character would be mojibake presented as a success, which is worse than the gap. Below 0x80 the
byte is the same character in every ANSI code page, so reading it is reading rather than choosing —
the fixture writes `\'26` and gets `&`, and writes `\'e9` and gets a counted erasure.

### Detection, and the `%PDF-` message fixed for the shape rather than one more format

`{\rtf` is the whole of it. A bare `{` is not enough, an OLE compound file — a legacy `.doc`,
beginning `D0 CF 11 E0` — is a different format with a different reader, and a renamed `.rtf` reads.

**The router's last line changed too, and it is not about RTF.** Before this slice an `.epub` was a
ZIP that no office predicate claimed, so it fell to the PDF reader and was refused for having no
`%PDF-` header — fail-closed, wrong cause, the third time that defect has appeared. A ZIP is
definitively not a PDF, so the CLI now sends **any** ZIP to the office router, whose own refusal
names what the package is and is not. v2-S6's pin holds untouched: `is_opendocument` still answers
on the declared **type**, so an `.epub` is never told it *is* OpenDocument.

**A CSV still takes the true-unknown-bytes path**, and that is recorded rather than fixed. Comma-
separated text cannot be told from prose without a reader, and guessing would claim every comma
file. It is refused for having no `%PDF-` header, which is honest for bytes nothing recognises.

### No new dependency

A hand-rolled control-word walker in `engine-office`, in character with `zip.rs`'s argument for
hand-rolling the ZIP reader: an RTF crate would need a licence check, a determinism argument and a
transitive-dependency review to save a few hundred lines that this repository can state completely.
No RTF crate, no `zip` crate, no LibreOffice, no shelling out.

### What could not be measured, recorded

**No corpus of real `.rtf` files was available** — the fourth consecutive slice that has to say so,
repeated rather than quietly inherited. Every rule is read off the RTF specification and pinned
against streams this repository authors byte by byte. Two consequences are stated rather than
hidden: the transparent list is short, so a formatting group it does not name has its text
declared; and a control word this reader does not name contributes no character, which is right for
the overwhelming majority of them and would be wrong for a special character the list misses.

Two fixtures: `rich-text-paragraphs` carries only what the reader consumes and declares **no**
erasure, and `rich-text-unread-destinations` declares both kinds.

- **In:** `crates/engine-office/{rtf.rs, lib.rs}` — `read`, `Paragraph`, `Document`, `is_rtf`,
  `read_rtf`, `RTF_MEDIA_TYPE` and the router's RTF branch; `RtfLocator`, `NativeLocator::Rtf`,
  `NativeLocator::names_a_part`, `NodeAttributes::RtfParagraph`, `RtfParagraphAttributes`,
  `RtfParagraphBreak`, the fourth rule in `check_page_less_shape`, `Profile::rtf_v0`,
  `RTF_READING_ORDER_RULE_V1` and `RTF_TEXT_CODE_RULE_V1` in `engine-core`; the CLI's RTF and
  container branches; `fixtures/office/rich-text-paragraphs` and `rich-text-unread-destinations`
  with their generator; `0.27.0`, the moved profile hash and both SDK pins; `PUBLIC-API.md` and its
  gate; `14`/`15`; `CAPABILITY.md`; CHANGELOG; README.

- **Out:** EPUB and CSV — **S9**. RTF tables as `TableRecord`s, OLE and embedded objects as
  evidence, `\pict` decoding, field evaluation, style resolution, code-page tables. `.odg`, and any
  widening of ODP's shape set — v2-S7 recorded that widening moves every ODP address. Any change to
  `ethos.grounding.v1`. Markdown or HTML for an RTF. New MCP tools, new SDK functions. A
  `coordinate_system` mode enum. Any new dependency. Any PDF detector change, and any move on the
  parked 0.489 chase.

- **Acceptance tests:**
  - [x] A known phrase is on a node with an `RtfLocator`; `pages` is `[]`; `tables` is `[]`;
        `node_get` over **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator's field set is exactly `{paragraph}`, and `deny_unknown_fields` refuses `page`,
        `bbox`, `x`, `part` and `sect` **by name**
  - [x] The address **names no part** and does not invent one; every node shares one container id,
        and mixing part-named with part-less locators in one artifact is refused
  - [x] **`\page` did not become a `PageRecord`**, and the test is not vacuous: the fixture's own
        bytes are asserted to carry `\page` and `\paperw`, and the break contributes no character
  - [x] A4: `{\rtf` decides. A bare `{`, another first control word, an OLE compound file, a ZIP
        and a PDF are each **not** claimed, and a renamed `.rtf` reads
  - [x] Every destination class is **counted and absent from the body** — a font table, a colour
        table, a style sheet, document information, an ignorable `{\*\…}`, a header, a footer, a
        footnote, a field's instruction **and** its cached result, and a `\pict`'s `\bin` data —
        asserted against the fixture's own bytes, never by grepping the generator
  - [x] Mutation-checked: removing the footer from the **bytes** lowers the declared count by
        exactly one, and nothing is promoted into the body by its removal
  - [x] The paragraph counter advances through a destination this reader does not read: the
        fixture's addressed paragraphs are `1, 4, 5, 6, 7, 8`
  - [x] `\uN` is the scalar it names, a surrogate pair is one scalar, `\ucN` is honoured, `\'26` is
        `&`, and **`\'e9` is a counted erasure rather than a Latin-1 guess**
  - [x] `\cell` and `\row` are recorded as terminators and **not** as a `TableRecord`
  - [x] A truncated stream, an extra `}`, a bare trailing `\`, a `\bin` past the end and an
        unsupported `\rtfN` are each a **named refusal**
  - [x] An `.epub` is still **not** OpenDocument, and an unread ZIP is refused naming the container
        rather than a missing `%PDF-` header; an `.odg` still names OpenDocument and its type
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the law
        — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; all **eight** profile hashes are mutually
        distinct; `capabilities.tables` is false
  - [x] Two runs over one document produce identical bytes, for both fixtures
  - [x] `Cargo.lock` gains **no new dependency** — no RTF crate, no `zip` crate, no renderer
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0; the 0.489 chase still parked
  - [x] Workspace **0.27.0**, both SDKs **0.27.0**, and the PDF profile hash moved on
        `parser_version` **alone** to
        `sha256:a9416ce96a7471d8a9cd951eda179978d737f1bfb1670faa151ad75ea734160d`

- **Depends on:** S7.

---

## S9 — EPUB → representation

- **Status: done.** `0.28.0`. `engine extract` reads an `.epub`, a block binds in the spine
  document the package names, and **the first format whose file may genuinely name pages still
  declares none**.

- **Goal:** EPUB, on the terms the first eight established. One format. Not "the rest of the office
  formats", and not CSV.

### The page, and the first time §3's law had to be argued rather than applied

§3 has always read *"no page **this engine did not read from the file**"* rather than "no page
ever", and every format before this one failed the reading half by construction:

| slice | what the file said | why it was not a page |
| --- | --- | --- |
| S2 DOCX | nothing | a page does not exist until a renderer picks one |
| S3 XLSX | a print range | the printer, the paper and a "fit to page" setting decide it |
| S4 PPTX | a slide, and `p:sldSz` | a slide is a **part**; a size is not a page |
| S5 ODT | `<text:soft-page-break/>` | a word processor's arithmetic, written at save time |
| S7 ODP | `<draw:page>` + a master's `fo:page-width` | structure, and paper the authoring tool wrote |
| S8 RTF | `\page`, `\paperw` | a producer's mark |

**An EPUB 3 navigation document may carry a `page-list`**, and an EPUB 2 NCX may carry page
targets. Those are page numbers of a **print edition** — identifiers, written down, readable, and
nothing about them is a rendering this engine performed. The earlier arguments do not reach them.

The one that does is the shape of the thing they would become. A `PageRecord` is a page **with a
width and a height**, and `check_structure` refuses a *measured box* on a page-less node precisely
because a rectangle nobody can check is the fabrication that law exists to prevent. A publisher's
label about somebody else's paper has no geometry at all: nothing could ever be validated against
it, and a consumer receiving it as a page record would resolve a citation against a rendering this
engine never saw. So the label is not copied onto `pages` under another name, `<nav>` is a counted
region like any other, and the fixture carries `PRINT-PAGE-17` in its own bytes so the empty vector
is a refusal rather than an absence.

### Reading order is the spine, and the archive is the trap

`EpubLocator { part, block }`, `deny_unknown_fields`, and a test that refuses `page`, `bbox`, `x`,
`page_list_label` and `spine_index` **by name**.

An EPUB is a ZIP of documents, and taking the XHTML entries in central-directory order — or sorted
by name — is v2-S3's `sheet{n}.xml` defect in a new container. Reading order lives in the package
document's `<spine>`, as `<itemref idref="…">` resolved through the `<manifest>`; `opc.rs` states
the rule for OOXML's `r:id`, and this is that rule in EPUB's spelling.

**The fixture proves it rather than asserting it.** `book-spine` stores `OEBPS/aa-second.xhtml`
*before* `OEBPS/zz-first.xhtml` and lists them the other way round in its spine, so archive order
and name order both disagree with the answer — and both orderings are read out of the package's own
bytes before the assertion runs.

Two more rules the container states and this reader follows:

- **`href` is relative to the package document's directory.** `href="chap01.xhtml"` inside
  `OEBPS/content.opf` is the entry `OEBPS/chap01.xhtml`. A reader that took the `href` verbatim
  would miss every publication that keeps its content in a subdirectory, which is nearly all of
  them.
- **References are percent-decoded per segment**, because they are IRI references and a book with a
  space in a file name writes `chap%2001.xhtml`. Per segment, so a `%2F` cannot invent a boundary
  the reference did not have. An absolute, remote or `..`-escaping reference is refused rather than
  clamped.

### `names_a_part` is true, and this is the first slice to use both halves of v2-S8's split

An EPUB is a package with **many** parts, so it takes the bijection `read_xlsx` has used since
v2-S3: one part id per spine document, ordinals contiguous within each. v2-S8 split
`check_page_less_shape` so a format with no parts could be checked on the one claim it *can* make;
v2-S9 is the first artifact to use the other half of that split, and it needed nothing new in
`engine-core`.

`node.ordinal` and `EpubLocator::block` are deliberately different numbers, and a reviewer should
not "fix" the divergence: the ordinal is contiguous within the part, and the block address advances
through blocks that mint no node — a `<script>`, a navigation list, an empty `<p>`.

### HTML's own default display, which is stronger than a list

`odt.rs` names the handful of inline elements whose characters are the sentence and calls
everything else foreign, because ODF has no default rendering to appeal to. XHTML does: its element set is a
closed vocabulary with a defined default style sheet. So the rule here is read off the
specification rather than chosen:

- an element in the XHTML namespace is a **block** when HTML gives it `display: block`, `list-item`
  or a table display;
- an XHTML element this reader has never heard of is **inline**, which is HTML's own answer for a
  custom element a producer invented;
- anything **outside** the XHTML namespace is **foreign** and is counted, never spliced.

That last line is what keeps an inline `<svg><title>` and a MathML `<annotation>` out of the
sentence — the two constructs v2-S6 and v2-S7 each had to name by hand in ODF, arriving here
through a namespace rule instead.

`script`, `style`, `template`, `noscript`, `nav` and ruby annotations are **regions**: skipped and
counted. `<head>` is a region too, on `odt.rs`'s **strict** rule — it counts only characters inside
its own blocks, because every XHTML document carries a `<title>` and counting it would declare an
erasure on every document that has had nothing removed. That is the argument `odt.rs` makes about a
note's `<text:note-citation>`, and it is why the clean fixture declares nothing.

**`linear="no"` is read and labelled.** A spine item marked non-linear is auxiliary — a pop-up
footnote target, a colophon — and it is still a document the spine lists. Dropping it would lose
text the book contains; reading it unlabelled would be the silent *extra* v2-S7 named A14 inverted.
So it is read and `EpubBlockAttributes::linear` says which it was.

### The whitespace engine is shared, and the three places it is not are written down

XHTML's `white-space: normal` collapses a run of spaces, tabs, carriage returns and line feeds to
one space and drops one at either end of a block. Those four characters and that rule are what
`odt.rs` already implements, so its block engine is **imported** rather than restated. Saying "the
rules are the same" would be a claim this engine has not checked, so the three divergences are
stated instead:

| divergence | what this reader does |
| --- | --- |
| `<pre>` (`white-space: pre`) | **handled** — its characters take the path `odt.rs` uses for a stated `<text:s>` and survive verbatim |
| a form feed, `U+000C` | HTML collapses it and ODF does not, so it is **passed through**. Left alone rather than added to the shared rule, which would change what three shipped ODF readers do with a character no measurement here was about |
| a line break between two CJK characters | CSS removes it; this reader makes it a **space**. **The widest gap this slice knowingly leaves**, and it is not approximated: the correct rule needs a computed `white-space` value and the scripts on both sides, and this reader reads no style sheet |

**No style sheet is read at all**, which is the general case those three are instances of. A book
may set `white-space`, hide a block with `display: none`, reorder blocks, or generate text through
`::before`; none of it is applied, so the text here is what the document *states* rather than what a
reading system would show.

### Numeric character references, and the six hashes that decided how

v2-S3 measured that `quick-xml` delivers `&#233;` as a general reference named `#233`, so the
shared entity rule refuses it — *"a named refusal of a valid document"*, recorded then and left
alone because no measurement asked for more. **XHTML asks.** A content document is hand-authored
XML with no DTD, so its authors reach for `&#160;` and `&#8217;` constantly, and a character
reference needs no DTD to resolve.

It is **not** folded into the shared function, and the reason is the profile rather than taste. Six
shipped readers name a `text_code_rule`, and a rule id has to move when the behaviour it names
moves. Widening `resolve_entity` would change what a DOCX reader does with a document it currently
refuses — six profile hash moves for a slice that measured one format. So `xml.rs` gained
`resolve_reference`, the EPUB reader uses it, the other six still refuse, and **unifying them is a
decision with six hash moves attached that this slice records rather than makes**.

**Two gaps recorded rather than closed:**

1. **Attributes.** `unescape_attribute` still routes through the shared rule, so
   `<item href="a&#32;b.xhtml"/>` is a named refusal of a legal package document. Rare; recorded
   here rather than left undecided.
2. **HTML named entities.** `&nbsp;` is an HTML name, not an XML one, and an XML parser without the
   DTD cannot resolve it. Refusing it is what the specification says to do, and this reader does.

### Encryption, checked against the spine rather than against its own presence

Nearly every real publication carrying `META-INF/encryption.xml` carries it to **obfuscate a font**,
and its text is in the clear. Refusing all of them would refuse readable books; ignoring the file
would hand ciphertext to the XML reader, which reports malformed XML and names the wrong cause —
the defect v2-S6 fixed for `%PDF-` in a different shape.

So the declaration is read, and a spine document named in it is a **named refusal**; a font is
counted as an unread entry and the book reads. Both halves are tested.

### Detection, and the pin this slice had to leave alone

`is_epub` asks the OCF question `is_odt` asks, against a different declared type: a first,
**stored** `mimetype` entry whose content is exactly `application/epub+zip`. Exact rather than
prefixed, never the extension.

**The container rule is shared and the family is not.** v2-S6 wrote `is_opendocument` to answer on
the declared **type** rather than on that entry's presence, precisely so this row's EPUB would not
arrive to be told it is OpenDocument. That pin held through S7 and S8 and holds here — the test
that says so moved from `rtf_cli.rs` to `epub_cli.rs`, where the format it protects now lives.

**The CLI's dispatch did not change.** v2-S8 made the router's last line the *container* question,
so every ZIP already reached the office router; an EPUB simply resolves there now instead of being
refused. That is tested rather than assumed.

### What could not be measured, recorded

**No corpus of real `.epub` files was available** — the fifth consecutive slice that has to say so,
repeated rather than quietly inherited. Every rule is read off the OCF, EPUB Packages and XHTML
specifications and pinned against publications this repository authors byte by byte.

**An adversarial review ran before this slice shipped, and six of its findings were real.** Two are
worth recording in full because of what they say about the reader's own discipline:

1. **A panic rather than a wrong answer.** The first percent-decoder indexed a segment as a `&str`
   by byte offset, so a `%` followed by a multi-byte scalar sliced inside it and crashed. A reader
   whose whole contract is a named refusal must not have an input that takes the process down.
2. **A wrapped erasure count, reproduced end to end.** `read` folded each document's A14 counts into
   publication-wide `u32`s with a plain `+=`. A 31 MB publication of 82 documents exited **0** and
   declared 51,032,704 passed-over runs against a true 4,346,000,000 — the sum minus 2³², an 85×
   **under**-declaration presented as a complete read. That is the exact failure A14 exists to
   prevent, arriving through an accumulator rather than through a reading rule; the counters
   saturate now. **The same shape exists in the already-shipped PPTX reader** and is filed rather
   than fixed here, because changing a shipped format's behaviour is not this slice's to make.

The others: two unbounded lists with quadratic scans (the manifest, and `encryption.xml`), an
uncapped region stack beside a capped block stack, a silent drop the `strict` rule opened for a
`<title>` outside the head, and a signed character reference (`&#+66;`) that resolved because Rust's
integer parsers accept a leading `+`.

**And one test was vacuous.** `the_declared_type_is_matched_exactly` compared two compile-time
literals and never called `is_epub`. Replacing it with one that builds a package per near-miss
immediately caught a real prefix-versus-exact defect the original could not have seen — which is the
argument for the no-vacuous-tests rule stated better than any prose could.

Two fixtures: `book-spine` consumes every entry it contains and declares **no** erasure, and
`book-unread-parts` declares every kind.

- **In:** `crates/engine-office/{epub.rs, lib.rs}` — `read`, `Block`, `SpineDocument`,
  `Publication`, `is_epub`, `unread_entries`, `read_epub`, `EPUB_MEDIA_TYPE` and the router's EPUB
  claim; `xml.rs`'s `resolve_reference` and `unprefixed_attribute`; `EpubLocator`,
  `NativeLocator::Epub`, `NodeAttributes::EpubBlock`, `EpubBlockAttributes`, `Profile::epub_v0`,
  `EPUB_READING_ORDER_RULE_V1` and `EPUB_TEXT_CODE_RULE_V1` in `engine-core`;
  `fixtures/office/book-spine` and `book-unread-parts` with their generator; `0.28.0`, the moved
  profile hash and both SDK pins; `PUBLIC-API.md` and its gate; `14`/`15`; `CAPABILITY.md`;
  CHANGELOG; README. The `engine-office` crate header, stale since v2-S6, now names every format.

- **Out:** CSV — **S10**. CSS as evidence, JavaScript, SMIL and media overlays, SVG content
  documents, EPUB dictionaries, `alt` text and other attributes as text, `epub:type` as a
  structural vocabulary, `TableRecord`s from `<table>`. The `.odg` refusal, ODP's shape set and
  RTF's `\'hh` rule — all untouched. Any change to `ethos.grounding.v1`. Markdown or HTML for an
  EPUB. New MCP tools, new SDK functions. A `coordinate_system` mode enum. Any new dependency: no
  `zip` crate, no EPUB crate, no HTML5 parser. Any PDF detector change, and any move on the parked
  0.489 chase.

- **Acceptance tests:**
  - [x] A known phrase is on a node with an `EpubLocator`; `pages` is `[]`; `tables` is `[]`;
        `node_get` over **unmodified MCP** resolves the minted id and `s-forged` fails closed
  - [x] The locator's field set is exactly `{part, block}`, and `deny_unknown_fields` refuses
        `page`, `bbox`, `x`, `page_list_label` and `spine_index` **by name**
  - [x] **Reading order is the spine**, proven against the package's own bytes: the archive stores
        the chapters in one order, the spine states another, and the nodes arrive in the spine's
  - [x] **A `page-list` did not become `PageRecord`s**, and the test is not vacuous: the fixture's
        navigation document is inflated and asserted to name print pages, which are then asserted
        absent from every node
  - [x] `names_a_part` is true; one part id per spine document; the part-id ↔ part-name bijection
        holds both ways; `ordinal` is contiguous within each part while `block` — which restarts
        per part too — skips the positions of blocks that minted no node
  - [x] A4: exact `application/epub+zip` in a first, **stored** `mimetype`; a reordered entry is
        not an OCF container; a renamed publication reads
  - [x] **`is_opendocument` is still false for an EPUB**, and an `.odg` still names OpenDocument
        and its declared type
  - [x] The allowlist holds: a script, a style sheet, a `<template>`, a ruby annotation, an inline
        SVG's `<title>` and a MathML `<annotation>` are each absent from `Node.text` **and present
        in a count** — asserted by inflating the package's own entries, never by grepping the
        generator. A `<head>`'s `<title>` is absent from `Node.text` and **counted nowhere**, which
        is the `strict` rule's whole purpose and is asserted separately
  - [x] Namespaces are resolved: an XHTML `<p>` is a block, a `<title>` inside an inline `<svg>` is
        not the document's title region, and a package document in another vocabulary states
        nothing
  - [x] `<pre>` keeps the whitespace the document wrote; everything else collapses XHTML's way;
        `<br/>` is a line feed; a numeric character reference resolves and a reference naming no
        scalar is refused
  - [x] `linear="no"` is **read and labelled**, and the flag is not simply false everywhere
  - [x] Mutation-checked: removing a `<script>` from the **bytes** lowers the declared region count
        by exactly one, and nothing is promoted into the body by its removal
  - [x] An encrypted **spine document** is a named refusal; an obfuscated font alone is not
  - [x] A missing `container.xml`, a package with no spine, an `<itemref>` naming no manifest item,
        two manifest items of one id, a spine reaching one entry twice, and a duplicated archive
        entry are each a **named refusal**
  - [x] A reference that is absolute, remote or escapes the container root is refused; a `%` before
        a multi-byte character does not panic
  - [x] `engine ground` on the artifact is a **named refusal** naming `application/pdf` and the law
        — with **no change to `engine-grounding`**
  - [x] Every geometry row is `NotApplicableToKind`; all **nine** profile hashes are mutually
        distinct; `capabilities.tables` is false
  - [x] Two runs over one publication produce identical bytes, for both fixtures
  - [x] `Cargo.lock` gains **no new dependency** — no `zip` crate, no EPUB crate, no HTML5 parser
  - [x] **A CSV still does not extract**, and this slice did not sniff commas to change that
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication still 0; the 0.489 chase still parked
  - [x] Workspace **0.28.0**, both SDKs **0.28.0**, and the PDF profile hash moved on
        `parser_version` **alone**

- **Depends on:** S8.

---

## S10 — CSV — **done, as an argued refusal.** No reader

- **Goal, as reached:** the last format in v2's row, and the one this engine **does not read**.
  S10 ships the **refusal**, argues it, and pins it in a test a later slice must delete on purpose.

- **The guess this section carried before the slice is overturned.** It read: *"Whoever
  implements CSV owns that message, and owes an argument for whatever distinguishes a CSV from a
  text file that happens to contain commas — **probably one the caller supplies rather than one
  the bytes do**."* The caller-supplied detector is refused, and the reason is this repository's
  own precedent rather than a new principle.

### The parse is not the problem. One field is

A CSV parse of arbitrary text **fabricates nothing**. Every field's `text` would be bytes
genuinely present in the stream; every record ordinal a true line count, not a rendering; and
`pages: []` simply **true**, because there is no geometry to invent and no page to invent. None of
the failure modes this repository was built against is present: not **L30**'s invented pagination,
not v1-S1's 662 cells the page never drew, not v1.2-S5's loose boxes sold as ink.

**Exactly one thing would be false, and it is one field.**
`SourceIdentity.media_type` (`crates/engine-core/src/representation.rs:83`) would say `text/csv`
about a file nobody measured to be one. That is an **invented identifier**, which standing rule 4
forbids — and `SourceIdentity` is `deny_unknown_fields` with **two** fields and **no room to say
"asserted"** (`representation.rs:79-86`).

So a caller-supplied `--format csv` does not break **A4**'s *rule*. It breaks A4's **guarantee**,
and the artifact has nowhere to say so. That is not a new argument. It is
`docs/13-V12-MILESTONES.md:550-555` word for word, written about a caller-supplied *version* at
v1.2-S5, where it was decisive enough to refuse an entire integration:

> an identity that can be asserted is an identity that can disagree with what it describes

It also settles the follow-on question. *If the caller asserts CSV and the bytes are not CSV, what
happens?* **The engine cannot tell.** That is definitionally what "no detector" means: there is no
check that could fire, so a wrong assertion always produces a **successful** artifact, and
fail-closed is not reachable from inside that design.

**The naive reason is the wrong one and is recorded here so it is not re-derived.** A one-column
parse of prose is not illegal because it fabricates content — it does not fabricate content. The
illegal thing is the media-type claim. Getting this backwards sends the next reader at the parse
instead of at `SourceIdentity`, and builds the wrong slice.

### What the slice actually is: the last wrong-cause refusal, fixed for the shape

Measured at `0.28.1`, before the change:

```
$ engine extract rows.csv
engine: unsupported media type: expected a PDF header (%PDF-) at byte 0, found "name," [unsupported]
exit 2
```

The exit code was right and the **cause was wrong**, in the exact way v2-S8 described when it fixed
the ZIP shape. The sentence is in the router's own comment, at
`crates/engine-cli/src/main.rs` (search for *"the third time"*; line numbers in that file moved
when this slice added the branch below it):

> This is the third time the same defect has been fixed for a different format, and it is fixed
> here for the shape rather than for one more member of it.

A `.csv` is not a broken PDF. It is bytes that state **no format at all**, and PDF was merely the
fallthrough that happened to catch them. S8 fixed the *container* shape; **S10 owns the last one**:
bytes carrying no signature, no container and no declaration. A prose `.txt` got the same message
with different quoted bytes, which is how it is known to be a shape rather than a format.

**The fix is a fallthrough refusal, not a CSV detector.**
`crates/engine-cli/src/main.rs`'s router is a six-term `||` of `is_*` predicates, and anything
answering false fell to the PDF reader. **No seventh term was added.** What was added is the branch
that was missing: bytes that are neither office-shaped nor `%PDF-`-headed are refused by naming
what was **looked for**, without being handed to a reader that was never asked for.

`check_pdf_magic` (`crates/engine-pdf/src/magic.rs`) is **exactly as it was** — its message is
correct for a caller who explicitly chose the PDF reader. `MAX_HEADER_OFFSET` is still `0` and its
`debug_assert` is still there: **no signature is scanned for at any offset**.

The one new export is `engine_pdf::aims_at_the_pdf_reader`, and it is not a detector. It decides
nothing about what bytes *are*; it answers one routing question — *is a message about a PDF header
the honest cause for these bytes* — and it is true in exactly two cases: the bytes start with the
header, or they are a proper prefix of it. Frozen in `PUBLIC-API.md` under Format detection.

### The two edges, tested rather than assumed

1. **A truncated PDF keeps the PDF-specific message, down to the zero-byte file.** Bytes that are
   a proper prefix of `%PDF-` **did** aim at this reader — a PDF cut short in transit is a PDF
   whose length is the story — so *"file is 3 bytes, shorter than the 5-byte PDF header"* is its
   honest cause. Displacing it would be the same wrong-cause defect pointing the other way. A
   four-byte `%PDX` is **not** a prefix and takes the no-format branch.

2. **`engine classify` did not move with `engine extract`, and the divergence is named here.**
   `classify` never reaches the office router at all — it opens the file with the PDF reader
   directly — so a `.csv` handed to it is still refused for having no `%PDF-` header. That was left
   deliberately: `classify` **is** the PDF classifier, and a caller who ran it named the PDF reader
   by naming the subcommand, which is the same argument that keeps the truncated-PDF message where
   it is. The divergence is pinned by
   `classify_still_answers_as_the_pdf_classifier_and_the_divergence_is_named` so it cannot change in
   either direction without a note. **MCP's `extract` has the same shape and is likewise unmoved.**

### The argument, mechanized

The refusal's honesty is not a sentence. It is **one test**:

> A `.csv` and a **letter containing a shopping list** receive **byte-identical stderr**.

That can only pass if nothing sniffed, and it is the assertion no detector-shaped implementation
can satisfy. It is written from both directions in
`crates/engine-cli/tests/no_format_cli.rs`: prose with no commas, prose **with** commas, a log line
with a **uniform comma count** — the one property a naive CSV detector is usually built on — and
the `.csv` itself. Each drives the real binary over a real file's own bytes.

The refusal names what was **looked for** — a ZIP local file header, an RTF brace group, a PDF
header — and names **no format it did not measure**. `%PDF-` is absent from its text on purpose.

**The two existing pins were strengthened rather than deleted.**
`crates/engine-cli/tests/epub_cli.rs` and `crates/engine-cli/tests/rtf_cli.rs` both asserted only
exit 2 and empty stdout and said nothing about stderr — so both would have stayed green straight
through the message change they existed to notice. That is the trap v2-S9's review named as the
vacuous test, and both now assert the message text.

### The reopening preconditions, so a later slice inherits a decision rather than a mood

A refusal without a stated reopening condition is a punt. There are exactly two, and **neither is
met at HEAD, neither is a day's work, and neither was started**:

1. **`SourceIdentity` can record asserted-vs-measured.** That is a `REPRESENTATION_SCHEMA_VERSION`
   move — currently `"0.5.0"` at `crates/engine-core/src/representation.rs:72` — and therefore a
   **contract slice with its own scope doc**, not a format slice.

2. **Or a measured predicate with a measured false-positive rate**: a real corpus of prose and
   real-world CSV in `fixtures/`, with the rate stated the way `docs/table-gate-v1.md` states 64‰.
   A predicate whose error rate nobody measured is exactly the "detector that guessed" this slice
   refused to write.

### Two questions S10 escalates and does not answer

Both are the owner's, in the shape decision #10 already shows. Neither is decided here.

1. **The v2 gate's verb.** `docs/00-NORTH-STAR.md:93` and `docs/02-ROADMAP.md:20` both say a DOCX
   quote and an XLSX cell both **ground**. Grounding a DOCX is **refused** — decided at v2-S1 as
   option (b), pinned by a test, listed in `CAPABILITY.md` under **Cannot**. Every slice since has
   quietly re-read *ground* as **bind** — this document's own status preamble says *"the v2 gate is
   still DOCX + XLSX, and both still **bind**"*, and the CHANGELOG's v2-S9 entry says it again —
   while **S1's open-questions table, the `gate wording` row**, flagged the question as unsettled
   **at S1**: *"the gate sentence needs re-reading, or the gate needs (a)."*
   It was never settled by a decision-log entry. Amending it is the owner's.

2. **"Embedded assets" is a real, undelivered v2 obligation.** It is in the roadmap row
   (`02-ROADMAP.md:20`) and restated as v2's content (`14-V2-SCOPE.md` §2, the "Anydoc-class
   formats" sentence). A repo-wide grep finds
   **three hits total** and nothing else: no A-row in `06-STEAL-REFUSE.md`, no `CAPABILITY.md` row
   in either table, no scope section, no acceptance test. (That count was taken *before* this
   slice; S10 adds the `CAPABILITY.md` row naming it, so a grep run after this slice finds four.) It was neither delivered nor descoped —
   it fell out of the conversation after S2. And it is not merely unread but **uncounted**:
   `crates/engine-office/src/docx.rs:51-57` limits `UNREAD_TEXT_PART_PREFIXES` to
   header/footer/footnotes/endnotes/comments, so `word/media/image1.png` lands in **no A14 bucket
   at all** — a DOCX with forty embedded images declares zero unread parts for them, while EPUB's
   `unread_entries` (`epub.rs:348-353`) counts every unread entry **including media**. **Reported
   here, not fixed here, and not descoped in passing.**

### One observation the next slice inherits

**No office reader has ever been fuzzed.** `fuzz/` targets only `engine-core` and `engine-pdf`.
v2-S9's first adversarial finding was a percent-decoder **panic** that survived to review precisely
because office code is unfuzzed. Not S10's job — S10 added no reader — and it is written down here
so the next slice does not have to rediscover it.

### What S10 may and may not say about v2

**S10 closes v2's format row. It does not close v2.**

> **v2's format row is closed: eight formats read, one argued refusal, and every split was against
> a measurement.**

`CAPABILITY.md`'s `**"v2 is complete"** | not yet` row **stays**, with its reason now naming the two
escalated items above rather than *"S10 has not started"*.

- **Acceptance — all met:**
  - [x] A `.csv` and a **letter containing a shopping list** produce **byte-identical stderr**,
        asserted in a test that reads each fixture's own bytes
  - [x] Prose with no commas, prose with commas, and a uniform-comma log line are each refused in
        those same words
  - [x] The refusal names what was looked for and **does not** mention `%PDF-`
  - [x] A **truncated PDF**, including the 0-byte case, keeps the PDF-specific message; `%PDX`
        does not
  - [x] `epub_cli.rs` and `rtf_cli.rs` now assert **stderr text**, and were strengthened rather
        than deleted
  - [x] Exit stays **2**, `code()` stays `unsupported`, stdout stays empty
  - [x] No `is_csv`, no `text/csv`, no `.extension()`, `.file_name()` or `.file_stem()` in any
        shipping source — pinned by a source walker, not by a grep in a commit message
  - [x] `Cargo.lock` gains **no** crate. No `csv` crate: its defaults violate three standing rules
        at once — lenient line-ending normalisation, flexible field counts, and header inference
  - [x] Still **nine** profiles, still mutually distinct; no `Profile::csv_v0`; the PDF hash moved
        on `parser_version` **alone**; `capabilities.tables` false
  - [x] Workspace **0.29.0** across every literal site **the release touches**; both SDK suites
        run by hand and pass. `fuzz/Cargo.lock` is **not** among them and still names an older
        version — it sits outside the workspace lock and no `--locked` job reads it
  - [x] `CAPABILITY.md` body **0.29.0**; the CSV row is a refusal with named reopening
        preconditions; ODP and RTF **Cannot** rows intact
  - [x] Oracle still 12 / 3; table gate still **64‰**; `GATE_PERMILLE` still 489;
        `irs-form-1040-2025` still 0 tables; fabrication still 0; the 0.489 chase still parked

- **Depends on:** S9.

---

## S10.1 — the seven claims v2-S9.1 and v2-S10 left false — **done**, no version

Doc-only, no version bump, commit `99c801d`. It repaired seven statements those two slices
introduced or should have fixed: this document's status line still said *"S10 has not started"*,
the slice table had no row for S9.1 at all, S10's "embedded assets" grep count was false the moment
it was written, S10's own version tick claimed every literal site while `fuzz/Cargo.lock` still
named `0.27.0`, `erasure_counters.rs` cited a file that did not exist at the release it cited, and
`xlsx.rs` had been repaired without the saturation test its slice claimed for every repaired
reader.

**It recorded itself nowhere.** The rows above are added by S10.2, because S10.1's own finding —
*"a shipped slice existed in the CHANGELOG and in git and nowhere in the milestones document that
is supposed to be the v2 code-review map"* — was true of S10.1 the moment it landed. It fixed the
pattern for S9.1 and reproduced it for itself, in git and in nothing else.

## S10.2 — the docs that stopped describing the code — **done**, as 0.29.1

**No behaviour change, no profile field, no reader.** A patch release on the precedent v2-S9.1 set
at `0.28.1`: `parser_version` moves and the nine profile hashes move with it, because a build is a
build.

The slice was handed **ten** measured statements false at `99c801d`. It fixed sixteen sites, and
the arithmetic is the point rather than the score: six were found by searching, and **one of the
ten was itself wrong**.

### The eleventh, and the method — because a site list is not a search

That is S9.1's handoff, restated here because this slice was given exactly the shape that produced
it: a numbered list. The target set was derived from the code first and the list checked against
it second.

| The sweep | What it is | What it found |
| --- | --- | --- |
| Every `file:NN` citation in `docs/` and `README.md`, re-resolved against the file it names | a mechanical resolve, not a read | **three rotted citations**, all the v2-S10 defect class |
| Every number-word standing beside *readers*, *formats*, *profiles*, *crates*, *counters* | counted against the tree | **four** stale counts, one of them the correction below |
| Every *"does not exist"*, *"has not started"*, *"is unstarted"* | checked against what shipped | **two** — the office-crate heading, and *"S8 is unstarted"* about a slice that is done |
| Every shipped commit, checked for a row in this document | `git log` against the slice table | **S10.1 is in git and in nothing else** |

**One of the ten was wrong, and the correction is the finding.** `PUBLIC-API.md` said the `xml`
module is shared by *"all **six** readers"* and the brief said the truth is eight. It is **seven**.
Eight is the format count, not the reader-of-XML count: `rtf` imports nothing from that module,
because a `{\rtf` byte stream carries no XML. The note now says seven *and says why not eight*, so
the next reader to count formats and reach for that number is stopped by the sentence itself.

### The two decisions, not typos

**1. The draft-schema worked examples — regenerated, under one rule for both files.**

The brief offered "bring it current" or "freeze, and explain why its sibling is not". The
measurement chose: both examples describe the **same source document** (`synthetic/simple-text`,
`sha256:f2f6ab91…`), both carry a `representation_sha256`, and **the two digests disagreed with
each other**. At most one could ever have been right; neither was. `html.draft.json` had had two of
its three identity fields dragged forward release by release and the third left behind, so it
described three different builds at once; `markdown.draft.json` was whole but sixteen releases
stale.

- **Freeze loses** because a frozen specimen must name the release that produced it, and neither
  file could say which one that was — html's fields disagreed and markdown's `0.13.0` was accurate
  only by neglect. A freeze rule would have had to invent the provenance it was meant to preserve.
- **Regenerate wins** because both artifacts are reproducible in three commands from a fixture the
  manifest already declares, and because it is the only option under which the word *example*
  stays true. The rule and the commands are in `docs/draft-schemas/README.md`, where a reader of
  either file will find them.

No digest was hand-edited. Both `examples[0]` objects are now **byte-equal to what the CLI emits**,
checked by comparing the parsed objects rather than by reading them — and every non-identity field
in both was already exact, which is why this was a three-field repair and not a rewrite.

**Known and owed, not fixed here:** nothing guards those identity blocks. The four guards in that
README pin `schema_version`, `artifact_type` and the two rule ids — the fields a consumer branches
on — and none covers `parser_version`, `profile_sha256` or `representation_sha256`. Adding one is a
source change and this release has none. It is the shape that README's own table says a draft
schema should have had from the slice that added it.

**2. `ETHOS_FIXTURES` — the three harnesses now honour it, and this is a behaviour change.**

`markdown_cli.rs`, `html_cli.rs` and `mcp_stdio.rs` hardcoded `repo_root().join("../ethos/fixtures")`
and asserted the file exists. `fixtures/manifest.json` declares that root **and** an
`ETHOS_FIXTURES` override, which `classify_cli.rs`, `diagnostics.rs`, `grounding.rs` and
`library_surface.rs` have always read.

The brief called this a behaviour choice and said to do it or record it. **Do it**, because the
measurement moved it out of the cosmetic category: `.github/workflows/ci.yml` checks the verifier
out at `ethos-oracle/` and points `ETHOS_FIXTURES` there, and `<workspace>/../ethos/fixtures` is
not a path that exists on a runner. Those three files were green only where the verifier happens to
be a **sibling checkout of this one**. Shown both ways rather than argued: with the corpus
relocated and the override set, all 35 tests pass; with the override pointed at a path that does
not exist, the repaired harness fails and the old one passed regardless — an override that could
not be observed to work is not an override.

**What this does not claim.** This repository has **no git remote**, so that workflow has never
run. The finding is what the workflow *says* versus where those three files *look*, which is
checkable from the tree; whether CI was ever red is not, and is not asserted.

### What S10.2 may and may not say

**It repairs the record. It does not advance v2.** No reader, no profile field, no schema version,
no fixture, no CI job, and no format. Both of v2's open items are untouched and both are still the
owner's: the gate sentence's verb, and the undelivered *embedded assets* obligation.

- **Acceptance — all met:**
  - [x] Each of the ten is fixed, and the one that was misstated is corrected **with its
        correction argued** rather than silently adjusted
  - [x] A search was run rather than a list worked through; the method is the table above and it
        found **six more sites** plus a slice missing from this document
  - [x] No behaviour change but one, named above and argued: three test harnesses resolve a
        corpus root through the manifest instead of a hardcoded path
  - [x] Every `file:NN` citation this slice would have written is a **section or symbol name**
        instead, and the three it repaired were replaced with named anchors — the v2-S10 defect
        class, which had shipped twice and was still live in two documents
  - [x] Workspace **0.29.1** at every literal site including `fuzz/Cargo.lock`, which v2-S10.1
        explicitly left; both SDK suites run by hand and pass
  - [x] Nine profile hashes move on `parser_version` **alone** and stay mutually distinct
  - [x] Oracle still 12 / 3; table gate still **64‰**; `GATE_PERMILLE` still 489;
        `irs-form-1040-2025` still 0 tables; fabrication still 0; the 0.489 chase still parked
  - [x] No git tag

- **Depends on:** S10.1.

---

## S11 — embedded assets, counted — **done**, as 0.30.0

**The measurement, first.** `02-ROADMAP.md`'s v2 row names v2's content as *"shared IR + one
serializer · embedded assets"* and `14-V2-SCOPE.md` §2 restates it. v2-S2 listed embedded assets
**Out** for that slice and no later slice picked them up, so they were neither delivered nor
descoped — they fell out of the conversation after S2 and stayed out for nine slices.

**That alone would be an omission. What made it a defect is A14.** *If something is removed, the
artifact says so and says how much.* Five readers of eight had counted their media since the slice
that added each of them:

| Reader | Counted media before this slice? | How |
| --- | --- | --- |
| ODT / ODS / ODP | **yes** | `odt::unread_entries` counts every non-packaging entry, and the limitation prose already said *"pictures or an embedded object"* |
| EPUB | **yes** | `epub::unread_entries` counts every entry not read |
| RTF | **yes** | `\pict` is a destination, counted in `destinations_not_read` |
| **DOCX / XLSX / PPTX** | **NO** | `unread_text_parts` is prefix-matched to header/footer/footnotes/endnotes/comments, charts/drawings/comments/pivotCache, and notes/masters/layouts/charts/diagrams. `word/media/image1.png` matched **nothing** |

So a DOCX with forty embedded images declared **zero** parts not read for them. Not under-counted —
**uncounted**, in no bucket at all.

### It was also unexercised, so step one was a fixture and a failing test

**None of the three OOXML fixtures contained a single media entry.** The blindness could not have
been noticed by anything in the suite, which is the more interesting half of the finding: a gap that
no fixture reaches is a gap no amount of test-running reports.

So the fixtures gained media **before** any reader changed — `make_fixtures.py` authors them, and
the counts are deliberately three different numbers (DOCX **2**, XLSX **1**, PPTX **3**) so a reader
returning another reader's count is caught by the number alone. `crates/engine-office/tests/embedded_assets.rs`
then asserted the declared count against the un-fixed readers and **four of its six tests failed**,
each on the same fact: the count did not move. The two that passed are the two asserting the *text*
bucket was unchanged, which is the baseline the fix had to preserve.

**A correction to the brief's own measurement.** It named `fixtures/office/sheet-unread-parts` as
the XLSX fixture. That fixture is an **`.ods`**; the XLSX one is `workbook-unread-parts`. Media was
added to the three OOXML fixtures, which are `unread-parts`, `workbook-unread-parts` and
`deck-unread-parts`.

### The decision: a second bucket, and why the other two lose

`OFFICE_PARTS_NOT_READ` was the only office bucket, and its OOXML message reads *"N part(s) of this
package **carry text** and were not read — headers, footers, footnotes, endnotes or comments"*.

**(a) A second bucket — CHOSEN.** A new code, `office-embedded-parts-not-read`, counted per OOXML
reader beside the text count. Costs one constant, one function per reader, three limitation sites
and a doc row.

- It is **the only option under which both messages stay true of every file they fire on**. That is
  not a tidiness argument: a caller reads *"parts carry text and were not read"* and goes looking
  for words. Told that about a PNG, they look for words that are not there and cannot be.
- It answers **A14's *how much*, per kind**. Forty images and forty unread headers are different
  facts with different remedies, and one number cannot say both. A caller who can tell them apart
  knows whether re-reading the document could ever surface the missing thing.
- It matches what ODF already tells a caller **in prose** — *"pictures or an embedded object"* —
  so the crate now says the same thing everywhere, in a machine-readable place in three readers and
  in prose in the other five.

**(b) Rescope the existing bucket** to "parts not read", drop *"carry text"*, count both kinds in
one number. Cheaper by one constant. **It loses on the same ground v2-S9.1 was held to**: a
counter's *meaning* changing in place, under a name that did not move. Every artifact ever produced
carries `office-parts-not-read` meaning *text parts*; after (b) the same code on the same package
would mean something else, with nothing on the wire to say which. And it **costs a caller the
ability to tell forty images from forty unread headers** — permanently, because the two are
summed and cannot be separated afterwards.

**(c) Descope embedded assets from v2**, with a `CAPABILITY.md` **Cannot** row. **It loses because
the argument it needs cannot be made.** (c) requires arguing that a media part is not an erasure —
and ODF, EPUB and RTF all count theirs, so the repository would be asserting that the same fact
about the same package is an erasure in five readers and not in three. It would also have required
striking *"embedded assets"* from `02-ROADMAP.md`'s v2 row in the same commit, which is an
owner-facing change. Not taken, and the roadmap row is untouched.

### What this slice is not, stated rather than implied

- **No asset byte is read, decoded, hashed or emitted.** `ImageRecord` stays in
  `engine-pdf/src/nodes.rs`; `engine-core` learns nothing about images.
- **No node for a media part.** A `word/media/image1.png` has no text, no address a citation could
  land on and no geometry. A node for one would be a node nobody can cite.
- **`pages` stays `[]`.** Nothing here touches §3.
- **No new detection.** A media part is identified by **where the package puts it**, which the
  package itself states — never by sniffing its bytes and never by its extension (**A4**). An
  extensionless entry under `word/media/` is counted; a `.png` somewhere else is not.
- **`xl/drawings/` stays in the text bucket.** A drawing part is XML that positions a picture and
  carries its title and description; the picture is `xl/media/`. Two erasures, two places, and the
  package is the one that separates them.

### The half this does not discharge, and it is the owner's

The roadmap says *"embedded assets"* and this slice makes them **counted**. It does not make them
**read**. Whether that row was ever asking for more than a count is not this repository's to decide,
and the shape of the question is concrete rather than philosophical: `engine-pdf` emits an
`ImageRecord` for a PDF image, and no office reader emits anything comparable. Restated at S12 with
the gate-verb question, in the shape decision #10 already shows.

- **Acceptance — all met:**
  - [x] The three OOXML fixtures gained media parts, authored by `make_fixtures.py`, and a test
        **asserted the pre-fix count and watched it fail to move** before any reader changed
  - [x] Both limitation messages are true of every file that triggers them; a test asserts no
        message says *"carry text"* about an embedded asset
  - [x] ODF, EPUB and RTF are unchanged — verified by building the previous commit in an isolated
        worktree and comparing **whole artifacts** at the same `parser_version`: all ten of their
        fixtures byte-identical, which is stronger than the counts
  - [x] One test per OOXML reader that a package with media declares it, and one that a package
        without media declares **nothing** — the code absent rather than present reading zero
  - [x] The three fixtures with **no** media — `simple-paragraphs`, `workbook-cells`,
        `deck-slides` — are byte-identical, because the media fixtures derive their content-types
        part rather than widening the shared one
  - [x] Workspace **0.30.0**; all nine profile hashes move on `parser_version` alone and stay
        mutually distinct; both SDK suites run by hand and pass
  - [x] Oracle still 12 / 3; table gate still **64‰**; `GATE_PERMILLE` still 489; fabrication 0
  - [x] No git tag

- **Depends on:** S10.2.

---

## S12 — the office readers get fuzzed — **done**, as 0.31.0

**The obligation and how long it stood open.** `06-STEAL-REFUSE.md`'s **A11** — *"Mutation testing
every fixture + `cargo-fuzz` **per format**"*, from Anydoc, due at **v0**. v2-S2 deferred the office
half in one clause, inside the same **Out:** bullet that deferred embedded assets: *"a cargo-fuzz
campaign — `A11`'s mutation lane for this format waits for a second one"*. **The condition was met
at v2-S3 and there are now eight.** `fuzz/Cargo.toml` depended on `engine-core` and `engine-pdf`
only; no office byte had ever been fuzzed.

**Not hypothetical.** v2-S9's first adversarial finding was a **panic** in the percent-decoder,
reachable from any `href` in a crafted package document, and the review record says it survived to
review *precisely because office code is unfuzzed*. A reader whose whole contract is a named
refusal must not have an input that takes the process down.

### One target, and the evidence that one is enough

`office_read` drives `engine_office::read(&bytes)` — the single entry point all eight formats share
and the one `engine extract` calls. The brief asked whether the router target plus a seeded corpus
already reaches the eight readers, and to split **only if it can be shown it does not**. It was
measured rather than assumed: every entry of the grown corpus was driven through the CLI and the
resulting `source.media_type` counted.

| Reader reached from the one target | successful artifacts |
| --- | --- |
| RTF | 267 |
| ODP | 18 |
| PPTX | 9 |
| EPUB | 8 |
| XLSX | 8 |
| ODT | 6 |
| ODS | 5 |
| DOCX | 4 |
| *(refused, fail-closed — also under test)* | *1220* |

**All eight, so no split.** Eight harnesses would divide one corpus eight ways and explore each
branch on a fraction of the budget, which is libFuzzer's coverage feedback working against itself.
A format later measured **unreachable** from this target is the argument for splitting one out; the
shape of the module tree is not.

### The corpus is the fixtures, and the seeding is scripted

`fuzz/seed-corpus.sh office_read` copies the **sixteen** packages in `fixtures/office/` — one valid
package of every shape this engine reads, already committed, already engine-owned. That is the seed
set A11 asks for and it was in the tree the whole time. Seeding matters more here than for the PDF
targets: random bytes almost never open like a ZIP, so an unseeded office campaign would spend its
entire budget being refused at the first predicate.

`corpus/` stays generated state, as `fuzz/.gitignore` already says; `seeds/` is the committed part,
and for this target the seeds are the fixtures.

### The oracle

**No panic. The type system supplies the rest.** `read` returns
`Result<DocumentRepresentation, EngineError>`, so *"every failure is a named `EngineError`"* is not
something the target can check — it is what the signature makes true. A `Malformed`, `Unsupported`,
`MissingPart` or `ResourceLimit` is a **pass**, and almost everything a fuzzer produces should be
one. A failure is an unwrap, an index panic, an arithmetic overflow, an allocation the resource
limits should have refused, or a hang. The build carries `-Cdebug-assertions` and AddressSanitizer,
so an overflow that would wrap silently in release aborts here.

### The campaign, and what it found

**Two runs, both to completion, on the same growing corpus.**

**The `exec/s` row is a rate under stated conditions, not a property of the engine, and this table
did not say so until v2-S12.1 added this paragraph.** Both runs were measured while
`cargo test --workspace` and other builds shared the machine. S12.1 re-measured on an idle host and
got **887 exec/s** on a 652-entry corpus — within one percent of run 1, and well under run 2 — so
load is not the dominant term; corpus size and composition are. The execution counts, the coverage
figures and the zero-crash result are unaffected: those are facts about what ran. See S12.1 for the
full comparison and for the one number this correction moves.

| | run 1 | run 2 | total |
| --- | --- | --- | --- |
| **executions** | 805,456 | 3,002,735 | **3,808,191** |
| **wall clock** | 901 s | 2,401 s | **3,391 s ≈ 57 min** |
| **exec/s** *(machine shared)* | 893 | 1,250 | 1,123 average |
| **edge coverage** | 8,148 | **8,985** | — |
| **features** | 22,101 | **26,074** | — |
| **corpus** | 1,429 | **2,544** entries / 3.7 MB | grown from 16 |
| **crashes / timeouts / OOM** | **0** | **0** | **0** |

**It found nothing, and that is a result rather than a pass.**

What ~3.8 million executions against a corpus that reaches all eight readers **does** support:
`zip.rs`'s hand-rolled central-directory reader, `MAX_INFLATED_BYTES`, the percent-decoder and
reference resolution in `epub.rs`, `MAX_BLOCK_NESTING`, RTF group depth, `MAX_TEXT_BYTES`, the
`quick-xml` depth arithmetic, and the eight shipping `expect("checked above")` claims — each a
claim that a `last()` guard makes a `pop()` safe — all survived without a panic, an
out-of-bounds, an overflow or a hang, under ASan and debug assertions.

What it **does not** support, said plainly rather than left to inference:

- **Coverage is not proof.** 8,985 edges is what this corpus reached in 57 minutes, not the
  reachable set. A campaign is a lower bound on what is broken, never an upper one.
- **It is a snapshot, not a gate.** Nothing re-runs it. A reader added tomorrow is unfuzzed until
  someone runs this again, exactly as the office readers were for nine slices.
- **The mutation half of A11 is still open**, and this slice does not touch it. No office fixture
  has been mutated. The office fixtures are not in `fixtures/manifest.json`, so the v0-M7 mutation
  harness does not reach them and would need a second corpus root. **Named as open in
  `06-STEAL-REFUSE.md`, beside the half that is now closed.**

### CI: no job added, and the budget stated either way

**No fuzz target is added to CI's required jobs by this slice.** The measured budget, so the
decision is the owner's rather than mine: the fuzz crate builds in **~50 s** from cold on this
machine, and the campaign sustains **~1,100 exec/s**. A 60-second smoke run per push therefore
costs roughly **two minutes** of job time and buys ~66,000 executions — under 2% of what this slice
ran, and against a corpus CI would rebuild from the sixteen seeds every time, since `corpus/` is
not committed. A useful campaign wants a **persisted** corpus and a schedule, not a per-push job;
that is an infrastructure decision with a storage question attached, and it is not this slice's to
make.

> **Corrected at v2-S12.1: the executions figure above is roughly double the real one.** The
> paragraph applies the *warm-corpus* rate to a *cold* corpus while saying in the same breath that
> CI rebuilds from the sixteen seeds every time — and those two halves disagree. Measured under
> exactly CI's conditions — fresh corpus from the seeds, 60 seconds, idle machine — the figure is
> **35,048 executions at 574 exec/s**, not ~66,000. The old number stays here because a published
> measurement that changes has to show its own history. **The decision is unchanged and the
> correction strengthens it**: 35,000 executions per push is a weaker case for a per-push campaign
> than 66,000 was.
>
> **v2-S12.1 did add a `cargo fuzz build office_read` step**, and that is not a reversal of this
> paragraph. The budget argument above is about *running* the target. Nothing in the repository
> was *building* it — `cargo build --workspace` excludes `fuzz/` — so the target could have
> stopped compiling with every job still green.

### Restated for the owner, unchanged and unsettled

**The v2 gate's verb.** `00-NORTH-STAR.md`'s gate table and `02-ROADMAP.md`'s v2 row both say a
DOCX quote and an XLSX cell both **ground**. Grounding a DOCX is **refused** — decided at v2-S1 as
option (b), pinned by a test, listed in `CAPABILITY.md` under **Cannot**. Every slice since has read
*ground* as **bind**. S1's open-questions table flagged it as unsettled at S1, in the **gate
wording** row, and no decision-log entry ever settled it. The two readings and what each costs:

| Reading | What it means | What it costs |
| --- | --- | --- |
| **"ground" means `ethos.grounding.v1`** | v2's gate is **not met** and cannot be met without option (a) — an Ethos-side schema revision, owned elsewhere | v2 stays open on a dependency this repository does not control. The eight readers are complete and the gate is not |
| **"ground" means "binds to an address the file states"** | v2's gate **is met**, and has been since v2-S3 | The gate sentence in two documents is reworded to say *bind*, and the word *ground* stops meaning two things in one repository |

**Not settled here.** Amending either document is the owner's.

- **Acceptance — all met:**
  - [x] `fuzz/` builds with `engine-office` and one office target
  - [x] Corpus seeded from `fixtures/office/` by `seed-corpus.sh`, scripted rather than copied
  - [x] A campaign was **actually run** — 3,808,191 executions over 3,391 s, stated here and in
        the CHANGELOG
  - [x] **No crash was found**, so there is no crash to fix, no regression test to add and no
        crashing input to commit. Said with the numbers rather than as a pass
  - [x] `fuzz/Cargo.lock` updated and committed
  - [x] **No fuzz job added to CI**, with the time budget stated above either way
  - [x] `A11`'s row says what is covered and what is not, and the **mutation half is named open**
  - [x] Workspace **0.31.0**; nine profile hashes move on `parser_version` alone
  - [x] No git tag

- **Depends on:** S11.

---

## S12.1 — the guards that were never there — **done**, as 0.31.1

**A patch release on the precedent v2-S9.1 and v2-S10.2 set.** No behaviour changed, no profile
field moved, and the nine hashes moved anyway on `parser_version` alone, for the reason those two
slices give: a build is a build, and the version is the only field that can tell a reader which one
produced the artifact in their hand.

### The defect, and it was v2-S12's own

`fuzz/fuzz_targets/office_read.rs` existed and **nothing compiled it.**

Three things had to be true at once, and they were:

- `Cargo.toml` excludes `fuzz/` from the workspace on purpose — `libfuzzer-sys` is a nightly-only
  sanitizer shim and has no business in the dependency graph of a shipped library — so
  `cargo build --workspace` never touches a fuzz target.
- CI's `v0-fuzz-smoke` built `open_and_classify` and `open_and_extract`, by name.
- `crates/engine-cli/tests/v0_exit_criteria.rs`'s `the_fuzz_target_and_seed_corpus_are_present`
  iterated the same two names, hardcoded, and asserted each contained `Document::open_bytes`.

So `office_read` could have stopped compiling against the engine API and **every job would have
stayed green.** That is the same shape as `fuzz/Cargo.lock` sitting at `0.27.0` for two slices: a
thing outside every gate, drifting quietly, found by a human reading rather than by a test.

### The fix is the build, not the run

**v2-S12's budget decision stands and this slice does not reopen it.** No fuzz *run* step was
added. What was added is `cargo fuzz build office_read`, one line inside the step that already
builds the other two, on a job that already installs nightly and `cargo-fuzz`. It buys the only
thing that was missing: proof the target compiles.

The guard changed shape rather than gaining an entry. It now reads the target list **from
`fuzz/fuzz_targets/`**, because a list written by hand is precisely what let the third target land
unguarded, and for each target it asserts three things:

| Assertion | What it catches |
| --- | --- |
| the file uses `fuzz_target!` | a target that is not a libFuzzer harness |
| the file drives **its own** entry point | `office_read` drives `engine_office::read`; the PDF targets drive `Document::open_bytes`. Asserting one entry point across all three would either fail or push a lie into the target to make it pass |
| **some CI job names `cargo fuzz build <target>`** | the actual defect. Nothing else in the repository compiles a fuzz target |

The count is pinned at three, so a fourth target is a decision someone has to make here rather than
one that happens by itself.

**Verified by breaking it.** With the new line removed from `ci.yml`, the test fails and names the
target, the cause and the repair; with it restored, it passes. A guard that has never been observed
to fail is a guard nobody has checked.

`docs/03-V0-SCOPE.md` §5.1's `v0-fuzz-smoke` row said *"`cargo fuzz build` on both targets"*. It
says three now, and says which one is built without being run and why.

### The measured statements, and the method — because a site list is not a search

The brief named four. The method was the one v2-S10.2 recorded: **derive the target set from the
code, then check the handed list against it.** Seven read-only sweeps ran over
`crates/*/src` comments, over `docs/` and the top-level markdown, and over every `#[test]` in the
workspace; each candidate was then re-checked by an independent pass whose default answer was
*refuted*, which threw three of the fifty-five out. **Fifty-two survived.** The four in the brief
were all real. One of them was understated, and four more sites in
`crates/engine-pdf/tests/robustness.rs` were found outside the sweeps by reading it as v2-S13's
contract. **Twenty-four are repaired here. Twenty-eight are not, and are named below.**

**What this slice repaired:**

| Site | Was | Is |
| --- | --- | --- |
| `engine-office/src/lib.rs`, `read`'s dispatch comment | *"With **four** formats a chain of `if`s…"* | seven, and the neighbouring sentence now says entries **four through seven** share the `mimetype` kind rather than only the fourth |
| `engine-office/src/lib.rs`, `read`'s RTF pre-check | *"The **six** formats below are packages"* | seven |
| `engine-office/src/lib.rs`, `read`'s ODF media-type guard | *"this engine reads **two** of the family"* | three — the `if` three lines below it already named ODT, ODS **and** ODP |
| `engine-office/src/lib.rs`, `is_opendocument` | *"a **third** question rather than an `||` of the other two"*, and *"an `.odp` answers `false` to every predicate here"* | the `||` has grown to three and `is_odp` now answers `true`; the argument is restated around the ODF formats this engine does **not** implement, where it is still exactly right |
| `engine-office/src/lib.rs`, `ODT_MEDIA_TYPE` and `ODT_CLAIM` | *"the only one of the **four**"*, *"the other **three** entries"* | four of eight media types are self-declared; ODS, ODP and EPUB do the same |
| `engine-office/src/lib.rs`, `read_pptx` | *"three formats now instead of one"* | three at v2-S4, seven by v2-S12 |
| `engine-core/src/lib.rs`, crate docs | *"# **Three** rules this crate enforces in the type system"*, over a list of four | four. Wrong since M4 |
| `engine-core/src/representation.rs`, `every_profile_is_distinct_from_every_other` | the name said *every*; the body checked **four** of nine | nine, with the count asserted. See below |
| `engine-cli/tests/v0_exit_criteria.rs`, `no_job_filter_selects_zero_tests` | *"Every test function name in the workspace"*, over a hardcoded list of **four** crates | read from `Cargo.toml`'s `members`. See below |
| `engine-pdf/tests/robustness.rs`, `EXPECTED_SURVIVORS` triage | *"`flip-tail-byte` on **four** documents"*, *"the other **eleven**"* | nine and forty-six. Measured, not estimated |
| `engine-pdf/tests/robustness.rs`, `run_mutant` | *"the **twenty** small fixtures"* | fifty-two |
| `engine-pdf/tests/robustness.rs`, the injection floor | `checked >= 12`, arguing *"the floor sits just below"* forty-four | `>= 40`. See below |
| `docs/03-V0-SCOPE.md` §4 | *"**one** engine-authored CC0 fixture… the only fixture this repo owns"* | thirty-seven |
| `docs/03-V0-SCOPE.md` §5.1, `v0-artifact-identity` | three test names | four — `artifact_identity_round_trips_through_c14n` was missing, and the job runs it |
| `fixtures/README.md` | *"the 15 conformance entries are `ethos`; **33** are"* | thirty-seven, which is what the manifest's own `counts` says |
| `.github/workflows/ci.yml`, the mutation job comment | *"**23** fixtures"* | the number is gone. The harness derives it and prints it; a number in a comment is a number nothing checks, and this one said 23 while the corpus reached 55 underneath it |
| `fuzz/seed-corpus.sh` | *"the **five** CC0 fixtures this repo owns"* | thirty-seven |

**Two of those are guards rather than prose, and they were failing at their own job.**

`no_job_filter_selects_zero_tests` exists to catch the quietest failure this scheme has: a CI filter
naming a renamed test, so the job prints `ok. 0 passed` and goes green having checked nothing. It
scanned four crates and `engine-office` joined the workspace at v2-S1, so for twelve slices the
scanner could not see a fifth of the tree. The consequence is a false *negative* — a job filtering
on an office test would have been reported as matching nothing, because the scanner was blind
rather than because the test was gone. It reads `Cargo.toml`'s `members` now, and asserts it found
at least five, because a scanner that finds nothing passes.

`an_injected_unknown_operator_stops_the_parse` carried the floor `checked >= 12` and a sentence
explaining that *"the floor sits just below"* the real number, so a fixture becoming unreadable
would be caught rather than quietly shrinking coverage. Fourteen fixtures took the injection at M7.
**Forty-four take it now.** The corpus tripled underneath a floor that did not move, so three
quarters of it could have stopped extracting with the test still green. A floor far below the real
number has stopped being a floor.

`every_profile_is_distinct_from_every_other` is the third, and the interesting one, because
**nothing was unverified**: the nine-way property is proven by
`the_epub_profile_is_its_own_and_all_nine_are_distinct` in
`crates/engine-office/tests/epub_representation.rs`. The damage a name that overclaims does is to
the next reader — someone adding a tenth profile reads *every*, sees green, and never learns the
array is a list a human has to remember to grow. Every constructor is `pub` and lives in
`engine-core`, so the short list was never anything but the order they were written in.

### What this slice did **not** repair, named rather than left to be rediscovered

**Twenty-eight confirmed false statements remain.** They are not deferred because they are
acceptable; they are deferred because repairing this many is a slice, and this repository already
has the precedent for that being its own slice twice over — S10.1 (*"the seven claims v2-S9.1 and
v2-S10 left false"*) and S10.2 (*"the docs that stopped describing the code"*). Bundling thirty-five
prose repairs into a patch release whose job is a CI step would make the diff unreadable and the
argument unreviewable.

The line drawn: **this slice repaired every confirmed false statement in the files it had to open
anyway**, plus `crates/engine-pdf/tests/robustness.rs`, because v2-S13 builds a second harness from
that file's argument and a wrong count in a contract propagates into the thing built from it.

What is left, by file, so the next slice inherits a search result rather than a mood:

| File | Confirmed | Shape |
| --- | --- | --- |
| `docs/04-ARCHITECTURE.md` | 5 | *"Two roots"* (three), *"Four subcommands"* (nine), the vendored-CMap file count, *"exactly one"* engine fixture (37), the `engine-office` row naming four formats (eight) |
| `crates/engine-core/src/profile.rs` | 3 | *"Its six siblings"*, an exhaustiveness-gate comment, and `the_profile_names_every_table_rule_and_any_one_moves_the_hash` |
| `crates/engine-core/src/representation.rs` | 2 | the `NativeLocator::Rtf` ordinal, and *"the sharpest of the four"* against *"the sharpest of the six"* a few lines down |
| `crates/engine-pdf/tests/extraction.rs` | 2 | two source scans that assert an empty offender list with no floor on what they read |
| `crates/engine-office/src/{odt,opc,xml}.rs` | 3 | *"its three siblings"* (seven), *"One rule, three formats"* (two), and a `text_code_rule` count |
| `crates/engine-core/src/{lib,verifier,assurance}.rs` | 3 | the module table's row count, *"the other four subcommands"* (eight), and an assertion message |
| `crates/engine-pdf/src/{fonts,limitations}.rs` | 2 | a corpus size and a code-array length |
| `crates/engine-cli/{src/mcp.rs,tests/oracle.rs}` | 2 | a tool-count loop with no floor, and *"the four-crate wiring"* |
| `crates/engine-core/tests/contract_invariants.rs` | 1 | *"Every public type canonicalizes"* over a hand-listed sample |
| `crates/engine-office/tests/erasure_counters.rs` | 1 | `PDF_COUNTERS` and the *"every count"* claim above it |
| `crates/engine-pdf/tests/capabilities.rs` | 1 | *"Every source line of the workspace's integration tests"* |
| `docs/PUBLIC-API.md`, `NOTICE`, `docs/table-gate-v1.md` | 3 | *"four subcommands"*, a `not_decoded` list that M4 removed, and a manifest-as-single-source claim |

> **Status at v2-S13.3: all twenty-eight are closed.** The twelve guards were repaired at S13.1 and
> the sixteen prose statements at S13.3, which also found five more `not_decoded` sites and fifteen
> further mismatches the list never named.
>
> **Status at v2-S13.1: twelve of the twenty-eight are closed.** The twelve that were **guards**
> rather than prose are repaired in S13.1 below, because a test that overclaims is a hole and not a
> typo — and the `docs/table-gate-v1.md` entry turned out to be a latent gate hole rather than the
> prose this table filed it as. **Sixteen remain, all prose**, and they are S13.2's. This note
> exists because a deferred list that stays stale after being acted on is the defect it was written
> to prevent.

**One of those is more than prose and should be read first.** `no_job_filter_selects_zero_tests`
only inspects `run:` lines whose command begins `cargo test`. The `v1s1-gates` and
`v1s7-table-gate` matrices quote their commands — `run: "cargo test …"` — so the command begins
with a double quote and the whole line is skipped. Every filter token in those two matrices
(`content::tests`, `tables::`, `ruled_grid`, `accuracy::` and the rest) is unchecked by the guard
that exists to check exactly that. Repairing it is not a one-word fix: the scanner matches bare
`fn` names, and `content::tests` is a module path, so making it see those lines would fail on
tokens that are correct. That is a real piece of design and it is not a patch release's.

### The campaign rate, restated with its conditions — and one number corrected

The brief offered two honest options: state the method beside the recorded numbers, or re-run both
campaigns clean and replace the table. **Option (a), and the third measurement is why.**

S12's table records 893 exec/s, 1,250 exec/s and a 1,123 average, taken while `cargo test
--workspace` and other builds shared the machine. The record did not say so, and this slice adds
that clause. What it does **not** do is call the recorded rate a floor, because two idle
re-measurements on the same host say otherwise:

| Run | Corpus at start | Wall | Executions | exec/s | Edges | Crashes |
| --- | --- | --- | --- | --- | --- | --- |
| S12 run 1 — *machine shared* | 16 seeds | 901 s | 805,456 | 893 | 8,148 | 0 |
| S12 run 2 — *machine shared* | 1,429 | 2,401 s | 3,002,735 | 1,250 | 8,985 | 0 |
| **S12.1 A — idle, warm corpus** | 652 | 121 s | 107,328 | **887** | 7,662 | **0** |
| **S12.1 B — idle, cold, as CI would run it** | 16 seeds | 61 s | 35,048 | **574** | 7,126 | **0** |

An idle machine on a mid-sized corpus sustains 887 — within one percent of run 1, and well under
run 2. **Load is not the dominant term; the corpus is.** The rate is a property of a particular run
against a particular corpus, not of the engine, and stating it without those conditions is what made
it read like a constant. The execution counts and the zero-crash result are untouched by any of
this — those are facts about what ran.

**The correction.** S12's CI-budget paragraph extrapolated from ~1,100 exec/s to *"~66,000
executions"* for a 60-second smoke run, while saying in the same breath that CI would rebuild the
corpus from the sixteen seeds every time. Those two halves disagree: the warm rate does not apply
to a cold corpus. Measured under exactly CI's conditions — fresh corpus from the sixteen seeds, 60
seconds, idle machine — the real figure is **35,048 executions at 574 exec/s**, roughly half what
was published.

**The decision is unchanged, and moves further in the same direction.** 35,000 executions per push
is a weaker case for a per-push office campaign than 66,000 was, not a stronger one. A useful
campaign still wants a persisted corpus and a schedule, which is still an infrastructure decision
with a storage question attached, and still not this slice's to make. The old number stays visible
here and in the CHANGELOG rather than being overwritten, because a published measurement that
changes has to show its own history.

**The build cost, also re-measured:** `cargo fuzz build office_read` after touching the target
recompiles in **11 s** on this host, against the ~50 s S12 records for a cold build of the whole
fuzz crate. Either figure is small next to a job that already installs nightly and `cargo-fuzz`,
which is what makes the build worth adding where the run is not.

### Restated for the owner, unchanged and unsettled

Both questions below are the **owner's**, both were escalated at S11 and S12, and neither is
settled here. They are restated because a slice that touches this file restates them.

**1. The v2 gate's verb.** `00-NORTH-STAR.md`'s gate table and `02-ROADMAP.md`'s v2 row both say a
DOCX quote and an XLSX cell both **ground**. Grounding a DOCX is **refused** — decided at v2-S1 as
option (b), pinned by a test, listed in `CAPABILITY.md` under **Cannot**. Every slice since has read
*ground* as **bind**.

| Reading | What it means | What it costs |
| --- | --- | --- |
| **"ground" means `ethos.grounding.v1`** | v2's gate is **not met** and cannot be met without option (a) — an Ethos-side schema revision, owned elsewhere | v2 stays open on a dependency this repository does not control. The eight readers are complete and the gate is not |
| **"ground" means "binds to an address the file states"** | v2's gate **is met**, and has been since v2-S3 | The gate sentence in two documents is reworded to say *bind*, and the word *ground* stops meaning two things in one repository |

**2. Embedded assets: counted, or read?** v2-S11 made every reader **count** what it does not read,
which closed the **A14** violation. No office asset is **read**. `engine-pdf` emits an `ImageRecord`
for a PDF image; no office reader emits anything comparable. Whether `02-ROADMAP.md`'s v2 row was
asking for more than a count is not this repository's to decide.

- **Acceptance — all met:**
  - [x] `office_read` is built by `v0-fuzz-smoke` and covered by
        `the_fuzz_target_and_seed_corpus_are_present`, which asserts `engine_office::read` for it
        rather than the PDF entry point
  - [x] The target list is read from the directory; the guard asserts, per target, that a CI job
        names it. **Verified by removing the CI line and watching the test go red**
  - [x] **No fuzz run step added.** v2-S12's budget decision is untouched
  - [x] The four named statements repaired; a fifth searched for and **fifty-two confirmed** by
        the sweeps plus four more found by hand in the mutation harness. **Twenty-four repaired
        here; twenty-eight named above** with the reason they are not
  - [x] The campaign rate carries its conditions, the CI extrapolation is corrected from ~66,000 to
        a measured 35,048, and the old numbers stay visible
  - [x] **No behaviour change.** The diff is docs, comments, CI YAML, three test files, one crate
        doc heading and the version literals
  - [x] Workspace **0.31.1**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both SDK suites run by hand and pass
  - [x] Oracle still 12 / 3; `ETHOS_OWNED_FIXTURE_COUNT` still 15; table gate still **64‰**;
        `GATE_PERMILLE` still 489
  - [x] No git tag

- **Depends on:** S12.

---

## S13 — A11's other half — **done**, as 0.32.0

**The obligation, and it was due at v0.** `06-STEAL-REFUSE.md`'s **A11** — *"Mutation testing every
fixture + `cargo-fuzz` per format"*, from Anydoc. v2-S12 closed the fuzz half for office and wrote
the mutation half into A11's own row as **OPEN**: *"No office fixture has been mutated."* Sixteen
packages, eight formats, never damaged and never asked what they would do about it. This closes it.

`crates/engine-office/tests/robustness.rs` mutates every package in `fixtures/office/` **twelve**
ways — **148 mutants across sixteen fixtures, and not one of them panicked.**

### The decision: a second harness, and why a second manifest root loses

**Option (a), a second harness enumerating `fixtures/office/` directly, is what shipped.** The
expected answer, and the brief was right that it was — but not for the reason the brief offered.
The cost it named was *"a second implementation of the damage kinds"*, and that cost turned out to
be nearly zero, because **the damage kinds could not have been shared anyway**. Only five of the
PDF harness's six mean anything to a container, one means nothing at all, and four new ones exist
only because a ZIP has hazards a byte stream does not. Sharing code across those two sets would
have been sharing a name, not a mechanism.

**Option (b), an `office` root in `fixtures/manifest.json` with `robustness.rs` taught to skip
non-PDF roots, loses on something sharper than duplication.** `all_fixtures()` in the PDF harness
walks **every entry of every root** and hands each to `Document::open_bytes`. Sixteen ZIP and RTF
files added to that array are refused as `malformed` for having no `%PDF-` header — and **every
assertion in that file still passes**: nothing panics, no artifact binds to the wrong digest, and
an emptied survivor set matches zero survivors. The result is a suite that reports coverage of
sixteen office packages while proving nothing about any of them. *A green suite that mutated the
wrong corpus is worse than no suite*, because the reported coverage is now false and nobody is
looking.

It also edits a v0 harness and a v0-frozen manifest, and the manifest's own tripwire —
`all_fixtures()` asserts the array length equals `conformance_ethos_owned + benchmark +
engine_owned` — has no office term, so the sum would have silently stopped covering the corpus it
was written to guard.

**The oracle hazard, checked rather than assumed.** The brief was right to flag it and right to
demand proof. `ETHOS_OWNED_FIXTURE_COUNT` (**15**) and `ORACLE_AGREED_COUNT` (**12**) both live in
`crates/engine-cli/tests/oracle.rs`, and every gate that uses them selects with
`f["owner"] == "ethos"` — **never by root**. Owner and root correlate perfectly today, which is
exactly why someone could add a root, believe the count is root-scoped, and be wrong in a way that
surfaces later. Option (a) touches no manifest, so the risk is nil by construction;
`adding_this_harness_did_not_touch_the_fixture_manifest_or_the_oracle_count` asserts it anyway —
three roots, fifteen `ethos`-owned entries, and zero manifest paths that are not `.pdf`.

### What a mutant means for a package, written down

The brief asked what each damage kind should do to a container rather than a byte stream. The
answers, measured:

| Kind | Applies to | What happens |
| --- | --- | --- |
| `empty` | all 16 | Refused at `read`'s first guard: neither `{\rtf` nor something that opens like a ZIP |
| `truncate-16` | all 16 | Sixteen bytes is a *partial local header*, so `looks_like_zip` still answers **true** and the refusal has to come from `find_eocd`. Sharper than the PDF version, which dies at detection |
| `central-directory-truncated` | 14 packages | The offset comes from the archive's own EOCD, so the cut lands exactly where the directory begins on every package. Refused: *"no end-of-central-directory record"* |
| `flip-tail-byte` | all 16 | Lands inside the central directory on every package. **Survives on 7, refuses on 9, and which one is decided entirely by the field it hits** — see below |
| `first-deflated-part-byte-flipped` | 14 packages | The compressed-entry case, aimed at whatever part is physically first |
| `main-part-byte-flipped` | 14 packages | The compressed-entry case aimed at a part the reader must read. **This is where the finding is** |
| `header-overwritten` | all 16 | A4 in office spelling: clobbering `PK\x03\x04` or `{\rtf` leaves nothing to recognise |
| `junk-after-eof` | all 16 | **Survives on all sixteen, by two different mechanisms** |
| `second-eocd-appended` | 14 packages | No PDF analogue. `find_eocd` takes the **last** `PK\x05\x06` and never validates the comment-length field, so a forged trailing record relocates the whole directory read. Refused as `missing_part` — a package listing zero entries lists no main part |
| `mimetype-body-overwritten` | 8 OCF packages | The only mutant that reaches an ODF/EPUB package's self-declaration, which is the evidence `is_opendocument` and `is_epub` read |
| `rtf-version-bumped` | 2 RTF | `\rtf1` → `\rtf9`. The only route to `unsupported_version`: overwriting the magic instead kills `is_rtf` first and the router refuses before the gate is reached |
| `rtf-control-word-mangled` | 2 RTF | `\par` → `\pzr`. **The only mutant in the set that produces a wrong-but-plausible artifact rather than a refusal** |

**`unknown-operator` is absent, and that is a result rather than an omission.** The PDF kind
substitutes a same-length token into a plaintext content stream so `/Length` stays honest. Every
XML part in every package here is deflated; the only stored entry anywhere is `mimetype`. A
same-length substitution into compressed bytes cannot reach an XML reader — it fails on inflation
or on the length check, which `main-part-byte-flipped` already covers under a name that describes
what actually happens. Doing it honestly means inflate, substitute, re-deflate, and rewrite the CRC
and both size fields, which is **authoring a fixture rather than damaging one**. RTF keeps the kind
under its own name because its stream is plaintext and the substitution is trivial.

**Over-declared rather than silently skipped.** Every pair a kind cannot apply to is pinned in
`EXPECTED_INAPPLICABLE` — 44 of them — and asserted **exactly**, not as a floor. The five container
kinds are inapplicable to the two RTF streams and the two RTF kinds to the fourteen packages;
`mimetype-body-overwritten` is inapplicable to the six OOXML packages, because an OOXML package
does not declare its own type. That is the honest form of the brief's observation that the six PDF
kinds might reduce for RTF: **they reduce to seven, and RTF gains two of its own in exchange.**

### The finding, and it is the reason to have built this

**`zip.rs` verifies a part's declared length and never its CRC-32.**

`main-part-byte-flipped` flips one byte inside the compressed data of `word/document.xml`,
`xl/workbook.xml`, `ppt/presentation.xml`, `content.xml` or `META-INF/container.xml`. Ten of the
fourteen packages refuse, which is the expected outcome. **Four do not**, and the chain is worth
stating in full because every link is load-bearing:

1. The flipped byte leaves a deflate stream that `miniz_oxide` still inflates — zlib refuses the
   same bytes outright with *"invalid distance too far back"*; the permissiveness belongs to the
   backend, not to the format.
2. It inflates to **exactly** the declared uncompressed size, substituting a NUL where the invalid
   back-reference was. `zip.rs`'s only integrity check is `out.len() != uncompressed_size`, so it
   passes.
3. The central directory carries a CRC-32 for that entry. **Nothing reads it.**
4. The corrupted XML reaches the reader. The damage lands in a namespace URI, and the OOXML readers
   match namespaces by suffix, so it parses.
5. The extracted text comes out **byte-identical to the original's**.

The artifact is distinguishable from one built from the undamaged package by exactly one field:
`source.sha256`, which binds to the mutant. **That is the designed safety property working, with
nothing behind it.**

**This slice does not change a reader over it, and the reason is not squeamishness.** The contract
the harness asserts holds: the mutant was read, and the artifact bound to the bytes it actually
read, so nothing downstream can mistake it for the original. What is uncomfortable is narrower —
a *corrupted* part was read as though intact, with no declared erasure, in an engine whose thesis
is that a gap is never presented as a success.

**Escalated rather than settled**, because it is a reader change with a cost and a measurement
attached:

| Option | What it costs |
| --- | --- |
| **Verify CRC-32 in `zip::read_entry`** | A checksum pass over every inflated part on every read. `flate2` already exposes `Crc`, so no new dependency. It would move four of this harness's pinned survivors into refusals, and would refuse some real-world archives whose writers got the CRC wrong — which is a compatibility question this repository has no corpus to answer |
| **Leave it, and rely on `source.sha256`** | What ships today. A consumer comparing digests always sees a different document, which it is. A consumer *not* comparing digests sees a plausible artifact for a corrupted file |

**Not settled here.** It is a change to a hand-rolled v0-era reader that ~3.8 million fuzz
executions have already hammered, and it deserves its own slice with its own measurement rather
than a rider on a test slice.

### Survivors are pinned in five classes, each explained

36 survivors, and none of the classes is the PDF harness's — which is the strongest evidence that
the second harness was the right call:

1. **`junk-after-eof`, all sixteen, two mechanisms under one name.** On packages, `find_eocd` scans
   backward and every directory offset is absolute, so appended bytes sit outside everything the
   archive declares. On the two RTF streams it is not that at all: **RTF has no end-of-file
   marker**, `rtf::read` runs to `stream.len()`, and the appended bytes become document text.
2. **`rtf-control-word-mangled`, both RTF streams.** Swallowed by the reader's `other =>` arm, so
   two paragraphs merge and every later ordinal shifts. Correct — RTF readers are required to skip
   words they do not know — and pinned with its own test that the node count actually falls.
3. **`flip-tail-byte`, seven.** Survives exactly when the byte lands on a field nothing reads:
   external attributes, a CRC nobody verifies, a modification date, the uncompressed size of
   `meta.xml`, or the comment-length of the **last** directory entry. Refuses on a name (non-UTF-8,
   so `entry_names` refuses by name), a local-header offset, a name length, or the size of a part
   that *is* read.
4. **`first-deflated-part-byte-flipped`, seven.** Six are one fact: for OOXML the first deflated
   entry is `[Content_Types].xml`, and **no reader in this crate reads it** — its only appearance
   in `crates/engine-office/src` is inside a `docx.rs` unit test's list of names.
5. **`main-part-byte-flipped`, four.** The finding above.

### Two tests that exist because the fixtures corrected an assumption

**The harness was wrong twice and the corpus said so, which is the right way round.**

`appending_junk_to_an_rtf_stream_becomes_document_text` first asserted *"exactly one more node"* on
both RTF fixtures. True of `rich-text-paragraphs`, which ends `\row }` so the appended bytes open a
new paragraph. **False of `rich-text-unread-destinations`**, which ends `are both read.}` with no
trailing break, so the appended bytes are absorbed into the final paragraph: the node count holds
and the last node's text grows. The test now asserts the property that is actually true — the junk
becomes document text, by one shape or the other — and names both.

`an_unrecognised_rtf_control_word_is_swallowed_and_merges_two_paragraphs` was worse: it searched for
`\par` as a substring and matched the **`\pard`** that opens every paragraph in both fixtures.
Mangling `\pard` changes nothing a reader can see, because it only resets properties that were
already default — so the mutation applied, counted, survived, and proved nothing. RTF delimits a
control word by the first non-letter, so `find_control_word` now requires that delimiter. **This is
the same trap `crates/engine-pdf/tests/robustness.rs` records for `find_operator`**, where a
space-delimited search for ` Tj ` silently missed every fixture writing `(text) Tj\n`. Two harnesses,
two corpora, the same mistake — which suggests it is a property of mutation harnesses rather than
of either format.

### The corpus is the manifest, because office has no manifest

`all_fixtures()` reads `fixtures/office/` rather than a list in the file — the lesson of v2-S11's
three OOXML fixtures with no media part and of v2-S12.1's uncompiled fuzz target. The skip list is
`fuzz/seed-corpus.sh`'s, deliberately: that script already enumerates this corpus to seed
`office_read`, and two enumerators of one directory that disagree is how a fuzz corpus and a
mutation corpus drift apart without anyone noticing. `__pycache__` is the one that matters and it
is not hypothetical — it is gitignored, present locally, absent from a fresh CI checkout, and a
walk that did not skip it would mutate a different number of files in the two places.

There is no manifest `counts` field to cross-check against, so the harness cross-checks the
directory against itself: **sixteen fixtures, two of each of the eight formats**, asserted. A format
down to one fixture is a format whose second shape stopped being tested.

### CI

`v0-office-mutation`, a **top-level job rather than a fifteenth matrix entry**, and that is forced
rather than chosen: `the_v01_gates_exist` asserts the `v0-exit-criteria` matrix is exactly fourteen
entries, and v0 is frozen. §5's mutation criterion now names three jobs instead of two, which keeps
the criteria count at fifteen and makes `every_named_job_exists_in_the_workflow` assert this block
exists. All nine guards in `v0_exit_criteria.rs` were run against the new job rather than reasoned
about.

No corpus checkout: `fixtures/office/` is engine-owned and committed here, so this is the cheapest
job in the workflow.

### Restated for the owner, unchanged and unsettled

Both questions below are the **owner's**, both were escalated at S11 and S12, and neither is
settled here. The CRC-32 question above is **new and separate** — it belongs to this repository and
is a reader change, not a gate wording.

**1. The v2 gate's verb.** `00-NORTH-STAR.md`'s gate table and `02-ROADMAP.md`'s v2 row both say a
DOCX quote and an XLSX cell both **ground**. Grounding a DOCX is **refused** — decided at v2-S1 as
option (b), pinned by a test, listed in `CAPABILITY.md` under **Cannot**. Every slice since has read
*ground* as **bind**.

| Reading | What it means | What it costs |
| --- | --- | --- |
| **"ground" means `ethos.grounding.v1`** | v2's gate is **not met** and cannot be met without option (a) — an Ethos-side schema revision, owned elsewhere | v2 stays open on a dependency this repository does not control. The eight readers are complete and the gate is not |
| **"ground" means "binds to an address the file states"** | v2's gate **is met**, and has been since v2-S3 | The gate sentence in two documents is reworded to say *bind*, and the word *ground* stops meaning two things in one repository |

**2. Embedded assets: counted, or read?** v2-S11 made every reader **count** what it does not read,
which closed the **A14** violation. No office asset is **read**. `engine-pdf` emits an `ImageRecord`
for a PDF image; no office reader emits anything comparable. Whether `02-ROADMAP.md`'s v2 row was
asking for more than a count is not this repository's to decide.

- **Acceptance — all met:**
  - [x] All **16** fixtures in `fixtures/office/` are mutated, and the list is derived from the
        directory rather than hardcoded — with the count and the two-per-format shape asserted,
        since office has no manifest `counts` to check against
  - [x] Both permitted outcomes asserted, including that a **read** mutant binds to the **mutant's**
        `source.sha256` — with a floor, so a corpus where nothing parses cannot make that test
        vacuous
  - [x] **36 survivors pinned**, in five classes, each explained by mechanism rather than by name
  - [x] **No panic**, on any of 148 mutants
  - [x] `ETHOS_OWNED_FIXTURE_COUNT` still **15**; oracle still **12 / 3**; asserted by a test in the
        new harness, not assumed
  - [x] CI job `v0-office-mutation` exists, is named by `docs/03-V0-SCOPE.md` §5 and §5.1, and all
        nine `v0_exit_criteria.rs` guards pass against it
  - [x] **A11's row updated again** — the mutation half is now closed for office, and the note says
        which kinds transferred and which did not
  - [x] The option chosen is argued above and the other is refuted with the mechanism that kills it
  - [x] Workspace **0.32.0**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both SDK suites run by hand and pass
  - [x] No git tag

- **Depends on:** S12.1.

---

## S13.1 — the guards that check nothing — **done**, as 0.32.1

**A patch release, on the precedent v2-S9.1, v2-S10.2 and v2-S12.1 set.** No behaviour changed, no
profile field moved, and the nine hashes moved anyway on `parser_version` alone, for the reason
those slices give: a build is a build.

**The no-behaviour claim is proven rather than asserted, and it is stronger than the usual one.**
`parser_version` is `env!("CARGO_PKG_VERSION")`, so the version literal lives in `Cargo.toml` and
**not one non-comment line in any `crates/*/src` file moved** — every src edit is a comment or sits
inside `#[cfg(test)]`. Every non-comment line outside `mod tests { … }` was extracted from all
thirty `crates/*/src/**.rs` files at `HEAD` and at the working tree and diffed — **19,249 lines,
identical** — using the same scope `ci/forbidden-tokens.sh` uses. The extractor was checked by
injecting a `pub const` into `profile.rs` and watching the diff report it, because a proof that
cannot fail proves nothing.

### The defect, and it was the same shape as S12.1's

`crates/engine-cli/tests/v0_exit_criteria.rs`'s `no_job_filter_selects_zero_tests` exists to catch
the quietest failure this scheme has, and **it could not see five of the workflow's twenty-two
`cargo test` commands.** The parser required `cmd.starts_with("cargo test")`; the `v1s1-gates` and
`v1s7-table-gate` matrices quote their commands, so the command began with `"` and the line was
discarded before a token was read. **Forty-five of sixty filter tokens were checked. Fifteen were
not.**

The quoting is not incidental. Three of those five commands filter on a **module path** —
`content::tests`, `tables::`, `accuracy::` — and the scan collected bare `fn` names, which a module
path can never occur inside. The matrix whose style quotes its `run:` is the matrix whose filters
the scan could not have matched anyway. Both halves of the defect arrived in the same place.

### One of the fifteen was dead, and had been for twenty-two commits

`no_ruling_lines` matched no test in the workspace. It was **correct when written**: `5662f24`
(v1-S1) added the CI line and `fn a_page_with_no_ruling_lines_reports_an_empty_table_list` in the
same commit. `174e27f` (v1-S2) renamed that test to
`a_page_that_implies_no_grid_reports_an_empty_table_list` and did not touch `ci.yml` — the commit's
thirty-file stat list does not include it. libtest silently ignores a filter matching nothing.

**Renamed to `implies_no_grid`, not exempted.** Measured: the five tokens selected 26 distinct
tests and the first four selected the same 26, so the dead one selected nothing; the rename takes
the job to **27**, restoring the "looked, found none" negative behind that entry's own `gate:`
label of *"fabrication 0"*. The property was never unproven — `ci.yml:122`'s unfiltered
`cargo test --workspace` runs it every build. What was false is the named gate's claim to check it.

**One correction to the brief that ordered this slice:** the token sits in `v1s1-gates`, matrix id
`v1s1-ruled-tables`. The job actually named `v1s7-table-gate` runs `accuracy::` and carries no
`ruling` token at all.

### The repair is a rule, not a number

Stripping the quotes is one line, and one line is what leaves the next variant to be found by a
human. The guard now asserts **every non-comment line mentioning `cargo test` was parsed as a
command** — which does not depend on predicting the shape, whether that is single quotes, a leading
`env FOO=bar`, or a YAML block scalar.

And the haystack is no longer bare `fn` names. `workspace_test_paths` reconstructs the string
libtest itself filters on: module path joined to function name, for every `#[test]` in the
workspace. **1,213 names, asserted equal to the count of `#[test]` attributes in the tree**, with a
floor beneath the equality because an equality holds trivially at zero. This is strictly *tighter*
as well as wider — a token naming a private helper used to read as live while selecting nothing.

**Verified by breaking it, twice.** With `no_ruling_lines` restored the guard names it and fails;
with the quote-stripping removed it names all five commands and fails.

**A module-path rule, not an exemption list.** The alternative was a second exemption array beside
`NAME_READING_EXEMPTIONS`, and that file's own doctrine — a stale exemption is a hole nobody is
watching — argues against adding one to paper over a parser that could not read module paths. It
did not need one: modelling what libtest matches makes all three module-path filters resolve
against modules that exist.

### The twelve, all confirmed, and five more found

Every guard among S12.1's twenty-eight deferred statements was re-derived from the code and
adversarially re-verified by an independent pass whose default answer was *refuted*. **None was
refuted.** Repaired here:

| Site | Was | Is |
| --- | --- | --- |
| `engine-core/src/profile.rs`, `every_profile_field_is_hash_sensitive` | the comment claimed the destructuring made a silently uncovered knob impossible; the pattern had **34 leaves and 24 mutations covering 23** | eight more mutated and **demonstrated** to move the digest; three named as unmutatable (one legal value each); count pinned at 32 |
| `engine-core/src/profile.rs`, `the_profile_names_every_table_rule_and_any_one_moves_the_hash` | *any one* over three rules, **two** moved | `stroke_ruled` moved too, all three pairwise digests distinct |
| `engine-core/src/assurance.rs`, `every_false_capability_declares_a_limitation` | **eleven** of twelve capability codes named | twelve. The omission was `html`, added at v1.1-S4 |
| `engine-pdf/src/limitations.rs`, `PDF_CODES` | seven entries covering **five** of the module's seven `pub const` spellings | nine, and the list is cross-checked against spellings read back out of the source |
| `engine-office/tests/erasure_counters.rs` | `READERS` a nine-name array against twelve files; **`PDF_COUNTERS` checked by nothing** | the directory is the list; the eight PDF accumulators are derived and compared |
| `engine-cli/tests/oracle.rs`, `every_workspace_crate_links` | doc said *four-crate*, workspace has five, body asserted **three** | four, with the member count read from `Cargo.toml` and `engine-cli` named as this test's own binary crate |
| `engine-cli/src/mcp.rs` | two per-tool properties over a list with **no floor** | three tools and four arguments asserted |
| `engine-pdf/tests/capabilities.rs`, `test_sources` | *"every integration test in the workspace"* over a hardcoded two crates; floor `> 1000` bytes against a real 660,013 | five crates from `Cargo.toml`'s `members` — 24 files to **40** — with a crate-count equality and real floors |
| `engine-pdf/tests/extraction.rs`, the two source scans | offenders accumulated, emptiness asserted, **no floor whatever** | 28 files / 16,201 lines and 4,267 lines asserted; the six named extract modules asserted to still resolve |
| `engine-pdf/tests/extraction.rs`, `the_conformance_corpus_keeps_every_box_it_had` | name said the corpus, body listed five | renamed `five_conformance_documents_keep_every_box_they_had`; the five are not widenable, and each is asserted to have produced runs so `all()` cannot hold vacuously |
| `engine-core/tests/contract_invariants.rs`, `public_type_samples` | four tests said *every public type* over **17 values / 15 types** against 188 frozen exports | the number is stated, the universal claim attributed to `floats_appear_only_inside_quantize`, and a new test pins the sample and asserts every sampled type exists |
| `docs/table-gate-v1.md` §Corpus | *"All four … are hash-pinned in `fixtures/manifest.json`"* | **three of four** — see below |

**#12 was a latent gate hole, not prose, so it stayed in this slice.**
`cfpb-home-loan-toolkit.pdf` has **no manifest entry at all**; the four entries whose notes name it
are engine-owned fixtures *derived* from it. The `benchmark` root holds three where the table names
four, so the document carrying the largest single share of the 64‰ gate number is pinned by
nothing, and the corpus could change underneath the score with every test green.

**Pinned rather than closed**, by `the_gate_corpus_is_pinned_except_the_one_document_that_is_not`,
which asserts exactly which three are pinned and which one is not. Adding the fourth entry moves
`fixtures/manifest.json`'s `counts`, which drive `crates/engine-pdf/tests/robustness.rs` off its
pinned **55 fixtures and 318 mutants** and move `EXPECTED_SURVIVORS` with them. That is a corpus
decision with a measurement attached, and it is not a patch release's. The day it is made, that
test fails and brings whoever makes it back to the paragraph.

### A thirteenth was searched for, and five were found

**The method, so that "five" is a result rather than a mood.** Three independent read-only sweeps,
over **324, 284 and 170** candidates: one enumerating every `#[test]` whose name makes a universal
claim, one for the accumulate-offenders-and-assert-empty shape, one for hardcoded arrays that could
be derived. Each candidate was then re-checked by a pass whose default answer was *refuted*. All
five are repaired here rather than named for a later slice, because naming them is what S12.1 had
to do and this slice exists because that was expensive.

- **`engine-pdf/src/thresholds.rs`, `the_garbled_reason_is_never_constructed`** — the same floorless
  shape as the two in `extraction.rs`, plus a two-name exemption list nothing asserted still
  resolved.
- **`engine-core/tests/contract_invariants.rs`** — five contract invariants all funnel through one
  walk of `engine-core/src` and **not one recorded how much it read.** Thirty-seven banned needles
  return an empty hit list on a scan that read nothing exactly as on a scan that read everything.
  The file guards its *matchers* three ways and guarded its *corpus* not at all. The floor sits in
  the shared helper, so a sixth invariant inherits it.
- **`engine-pdf/tests/robustness.rs`, `a_surviving_mutant_never_claims_to_be_the_original`** — the
  digest comparison runs only for a mutant that parses, and nothing counted them. Sixty survive and
  `EXPECTED_SURVIVORS` pins which, so the count is now asserted equal to it. **The office harness
  written from this file at S13 already carries this floor**; it was never back-ported. Two
  harnesses, one guard — the same asymmetry S13 recorded for `\par`/`\pard`, arriving from the
  other direction.
- **`engine-pdf/tests/extraction.rs`, `every_run_carries_a_native_locator`** — a CI-named gate
  (`v0-locators`) that skips a fixture twice, silently, and floors on `total > 10` where `total`
  counts **runs**. One small document produces more than ten, so fifty-four of the fifty-five could
  have stopped extracting with the gate green. Now floors on fixtures that extracted: **44 of 55**.
- **`engine-cli/tests/public_api.rs`, `FROZEN`** — the per-crate diff is derived; the *set of
  crates* was a four-name array. At `ac148cf` (v2-S2, 0.21.0) it had three entries while
  `engine-office/src/lib.rs` already exported six items: **that crate's public surface was unfrozen
  and undocumented for a whole release and no test failed.** The set is now derived from
  `Cargo.toml`'s members filtered to those with a `src/lib.rs`.

### Restated for the owner, unchanged and unsettled

**Three** questions stand between this repository and *"v2 is complete"*. All three are the
**owner's**, and none is settled here. They are restated because a slice that touches this file
restates them.

**1. The v2 gate's verb.** `00-NORTH-STAR.md`'s gate table and `02-ROADMAP.md`'s v2 row both say a
DOCX quote and an XLSX cell both **ground**. Grounding a DOCX is **refused** — decided at v2-S1 as
option (b), pinned by a test, listed in `CAPABILITY.md` under **Cannot**. Every slice since has read
*ground* as **bind**.

| Reading | What it means | What it costs |
| --- | --- | --- |
| **"ground" means `ethos.grounding.v1`** | v2's gate is **not met** and cannot be met without option (a) — an Ethos-side schema revision, owned elsewhere | v2 stays open on a dependency this repository does not control. The eight readers are complete and the gate is not |
| **"ground" means "binds to an address the file states"** | v2's gate **is met**, and has been since v2-S3 | The gate sentence in two documents is reworded to say *bind*, and the word *ground* stops meaning two things in one repository |

**2. Embedded assets: counted, or read?** v2-S11 made every reader **count** what it does not read,
which closed the **A14** violation. No office asset is **read**. `engine-pdf` emits an `ImageRecord`
for a PDF image; no office reader emits anything comparable. Whether `02-ROADMAP.md`'s v2 row was
asking for more than a count is not this repository's to decide.

**3. Should `zip.rs` verify CRC-32?** Raised at S13 and unchanged here.
`crates/engine-office/src/zip.rs` checks a part's **declared length** and never its **CRC-32**. On
four of fourteen packages, a byte flipped inside the main part's compressed data leaves a stream
`miniz_oxide` still inflates — zlib refuses the same bytes — to exactly the declared length, with a
NUL where the invalid back-reference was. The corruption lands in a namespace URI the OOXML readers
match by suffix, so the extracted text comes out **byte-identical to the original's**; the artifact
differs in exactly one field, `source.sha256`. Unlike the other two, this one is **this
repository's** to answer rather than DocuShell's — but only by a slice that owns it, with a
measurement, and never as a rider on a test or docs slice. **This slice is a docs-and-tests slice
and does not answer it.**

- **Acceptance — all met:**
  - [x] `no_job_filter_selects_zero_tests` inspects **all twenty-two** `cargo test` commands,
        quoted or not; the token count is floored at 55 against a real 60, one command below the
        total; and every line mentioning `cargo test` must parse, so the hole cannot reopen in an
        unpredicted shape
  - [x] `no_ruling_lines` **resolved by rename**, with the orphaning commit identified. Not
        exempted
  - [x] Module-path filters handled by a **rule** — reconstructing the libtest name — argued
        against the `NAME_READING_EXEMPTIONS` precedent rather than around it
  - [x] The twelve verified against the code and repaired; **#12 confirmed a latent gate hole**
        and pinned rather than papered over
  - [x] A thirteenth searched for by three sweeps over 324/284/170 candidates; **five found and
        all five repaired**
  - [x] **No behaviour change**, proven mechanically over 19,249 shipping lines, with the proof
        itself checked by injection
  - [x] Workspace **0.32.1**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both SDK suites run by hand and pass
  - [x] Oracle still 12 / 3; `ETHOS_OWNED_FIXTURE_COUNT` still 15; table gate still **64‰**;
        `GATE_PERMILLE` still 489
  - [x] No git tag

- **Depends on:** S13.

---

## S13.2 — the roadmap reordered — **done**, as 0.32.2

**An owner decision, recorded rather than argued.** This slice changes no code, no reader and no
behaviour. It moves rows in a table and renumbers one of them, and it needs a version because three
of the twenty-three references it moves are rustdoc in `crates/engine-core/src/derivation.rs`.

### What changed

The ladder after v2 was **v2.1 (OCR) → v2.2 (accessibility) → v3 (assist)**. It is now
**v2.2 (accessibility) → v3 (assist) → v4 (OCR)**.

### Why OCR was renumbered rather than only resequenced

The owner asked for the order **v2, v2.2, v3, v2.1**. Taken literally that ships a build numbered
`2.1` after one numbered `3.0`, and this repository cannot afford that. `parser_version` is inside
`profile_sha256`, and `01-CONTRACT.md` calls profile-as-identity *"the load-bearing idea"*. A version
number whose ordering does not match build order stops being able to answer *which build produced
this artifact* — which is the entire reason the field exists.

So the sequence is exactly what was asked for and the label moved with it: **OCR is v4.**

**`v2.1` is now a gap in the ladder, and that is fine.** Nothing ever shipped under it — the
CHANGELOG has zero references — so no artifact carries it and no reader has to be told anything. A
gap is monotonic; an out-of-order number is not.

### Accessibility's gate had to be replaced, not just moved

`v2.2` read: *"Only on a named accessibility requirement. Never on the critical path."* That is a
**conditional parallel lane**, and a conditional parallel lane cannot also be the mandatory next
row. The two are incompatible, and moving the row without touching the gate would have left the
table contradicting itself.

The owner withdrew the condition. The gate is replaced with one that can actually be met and
actually be failed: **tagged output round-trips — a tag this engine writes is one it can read back
and ground against.** That is the accessibility analogue of every other gate in the table: a
property with a test behind it, not a business precondition.

### What this slice deliberately did not do

**It did not plan v4.** `02-ROADMAP.md`'s own policy stands: a version gets a scope document and a
milestones document before it gets code, and nothing past v2 gets that pair until v2's gate is in
sight. Moving OCR later does not make it planned, and this slice writes no `16-` document.

**It did not pay v4's two blockers**, which are unchanged and now recorded in decision #15:

| Blocker | State |
| --- | --- |
| `deny.toml` denies `reqwest`, `hyper` and `hyper-util` by name, and its header says the OCR lane *"will need a reviewed exception with its own ADR; it does not get one in advance"* | The ADR does not exist |
| ONNX runtime would be the largest runtime dependency in a tree that has no `zip` crate, no EPUB crate and no HTML5 parser | No exception written |

Moving OCR to last makes both **less** urgent, not resolved.

### Restated for the owner, unchanged and unsettled

All three questions stand. **1.** The v2 gate's verb — `ground` meaning `ethos.grounding.v1`, or
meaning *binds to an address the file states*. **2.** Embedded assets: counted, or read. **3.** New
at v2-S13 — whether `zip.rs` should verify the CRC-32 it currently ignores, since a corrupted part
that still inflates to its declared length is read as though intact. Full statements and costs at
the end of S13.

`docs/CAPABILITY.md` said *"two owner questions"* and now says three; v2-S13 added the third and did
not update that row.

- **Acceptance — all met:**
  - [x] Ladder order is v2 → v2.2 → v3 → v4 in `02-ROADMAP.md` and in `00-NORTH-STAR.md`'s gate
        table, and the two tables agree
  - [x] Every `v2.1` reference renumbered — twenty-three across eleven documents, `deny.toml` and
        three rustdoc comments. **Zero left in the tree**, and the CHANGELOG untouched because it
        is a historical record
  - [x] Accessibility's gate replaced with a testable one, and the withdrawal recorded rather than
        silently dropped
  - [x] **Decision #15** records the reorder, the renumbering argument and the two unpaid blockers
  - [x] **No behaviour change.** No non-comment line moved in any shipping `src` file
  - [x] Workspace **0.32.2**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both SDK suites run by hand and pass
  - [x] No git tag

- **Depends on:** S13.1.

---

## S13.3 — the statements that stopped being true — **done**, as 0.32.3

**A patch release, on the precedent v2-S9.1, v2-S10.2, v2-S12.1 and v2-S13.1 set.** No behaviour
changed. Every `crates/*/src` edit is a comment or sits inside `#[cfg(test)]`.

**This is the prose half of S12.1's fifty-two.** S12.1 confirmed fifty-two false statements,
repaired twenty-four and named twenty-eight. S13.1 took the twelve that were guards, because a test
that overclaims is a hole rather than a typo. These are the sixteen that are prose — plus what a
derived sweep found on top of them, because a handed list is not a search.

### The `vendor/` cluster, and it had to be one decision

`vendor/` holds **one** tracked file, `vendor/README.md`. Where the encoding data actually lives had
to be settled before any sentence about it could be reworded, and `vendor/README.md` had the answer
the whole time: the tables are `const fn` builders inside `crates/engine-pdf/src/encoding.rs`, baked
into `.rodata`, *"because a separate file would be the same bytes with a parser in front."*
`encoding.rs` itself claimed they were *"written out as data under `vendor/encodings/`"*, and the
doc on `WIN_ANSI` said the same in fewer words. `docs/04-ARCHITECTURE.md`'s tree drew
`vendor/cmaps/ # 168 Adobe .bcmap + NOTICE` for files that never landed.

**`not_decoded` was five sites, not the one the brief named.** M4 absorbed that list into
`assurance.limitations`; `NOTICE`, `vendor/README.md`, `encoding.rs`, `fonts.rs` and
`profile.rs`'s `CMAP_DATA_VERSION` all still named it.

**Two errors this slice introduced and an adversarial pass caught**, recorded because a repair that
introduces a new false statement is worse than the one it fixed:

1. A first draft said the CMap limitation rides on *"every artifact this build produces"*. It does
   not — `engine-office` builds its limitation lists fresh and never calls `backend_limitations()`.
   Every **PDF** artifact, classify and extract alike.
2. A first draft of `vendor/README.md` said two of its three gaps fail closed. **One does.** A
   predefined CMap is refused; a Core-14 font with no `/Widths` is read and reports `advance: null`;
   an unrecognised `/Differences` name **drops that run** and is counted into
   `broken-font-encoding`. That last one changed at **v0.1** and this file's own table row had said
   *"is refused"* ever since.

### One of the sixteen was misstated in the brief

`NativeLocator::Rtf`'s *"the invariant grew a third case"* is **true** and is left verbatim. It
counts the parenting invariant's three cases — paginated, page-less naming a part, page-less naming
none — and `part()`'s Rtf arm and `names_a_part`'s doc say the same thing in the same words. RTF is
the *seventh* page-less variant, but the sentence makes no ordinal claim about variants. This is the
third consecutive slice whose handed list contained one site that did not survive checking, which is
why the list is checked against the code rather than the other way round.

### The sweep, and what it found beyond the brief

**1,789 candidate lines examined; 15 confirmed mismatches.** The method was the one v2-S10.2
recorded and v2-S13.1 repeated: derive the target set from the code, then check the handed list
against it. Two of the three planned sweeps did not run — the session hit its usage limit — so the
sweep coverage here is **one of three**, and that is stated rather than rounded off.

What it found that the brief did not name: `CAPABILITY.md` claiming *"no office fixture has been
mutated"* one slice after v2-S13 mutated all sixteen, directly contradicting `06-STEAL-REFUSE.md`'s
A11 row; `docs/README.md`'s *"fourteen decisions"* in two places against `00-NORTH-STAR.md`'s
fifteen, and *"sixteen named jobs"* against §5's seventeen; `06-STEAL-REFUSE.md`'s *"four kinds are
new"* against the harness's five — the kind it collapsed away being the byte-flip that produced
v2-S13's CRC-32 finding; `README.md`'s *"S4 (HTML) is not started"* when it shipped at 0.14.0 and
*"S1 through S7b"* when S8 is done; and `Capabilities::V0`'s doc describing `structural_locators` as
narrowed to `false` three lines above a literal that has read `true` since v1-S3.

### P9 — a finding for the owner, not a repair

`docs/06-STEAL-REFUSE.md`'s TAKE table carries `| P9 | Vendored CMap tables | pdf-inspector | v0 |`.
The column is **Target**, so the row is not a claim of completion — but v0 is frozen and complete,
and the CMaps were **deliberately not carried**, argued at length in `vendor/README.md`. **Nothing
anywhere records that deferral**: no note on the row, nothing in `05-MILESTONES.md` M3,
`03-V0-SCOPE.md` or `CAPABILITY.md`. **A11**'s row in the same table carries a long annotation, so
the table can hold that kind of note and P9's does not.

Whether a v0-target obligation consciously not met leaves v0's gate met is the **owner's** call, not
this slice's. Named, and stopped.

- **Acceptance — all met:**
  - [x] The sixteen repaired or explicitly declined, each verified against the code — **one
        declined**, `NativeLocator::Rtf`, because it is true
  - [x] The `vendor/` cluster resolved as **one** decision, with the encoding data's real location
        established first
  - [x] An independent site set derived and the difference reported in both directions: **five**
        `not_decoded` sites where the brief named one, and **fifteen** sweep findings on top
  - [x] **No behaviour change**; every `src` edit a comment or inside `#[cfg(test)]`
  - [x] S12.1's deferred list updated to say what is closed
  - [x] Workspace **0.32.3**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both projection examples regenerated and **verified** against real artifacts
  - [x] No git tag

- **Not met, and stated rather than rounded off:**
  - [ ] **Two of three planned sweeps did not run**, on a usage limit. A `crates/*/src` comment
        sweep and a cross-reference-rot sweep are unrun, and neither is a formality: the one sweep
        that did run found fifteen.

    > **Closed at v2-S13.5 (0.32.5).** Both ran. 12,805 candidate comment lines and 1,372
    > backticked identifiers examined; the prior held. The finding that justified the slice was
    > `07-VERIFY-BOUNDARY.md` claiming to hold the forced decisions *verbatim, identically* while
    > holding fourteen of seventeen — missing the three the owner had just settled.

- **Depends on:** S13.1.

---

## S13.4 — the gate's verb, and what "embedded assets" meant — **done**, as 0.32.4

**The owner settled two of the three standing questions on 2026-08-21.** This slice records the
decisions and rewords the sentences that carried them. No behaviour changed; no code changed at
all beyond the version literal's home in `Cargo.toml`.

Both are now `00-NORTH-STAR.md` forced decisions — **#16** and **#17** — beside **#15**, the
roadmap reorder the owner amended the same day. A question answered in a chat and not written into
the decision table is a question that gets re-escalated by the next slice, which is exactly what
happened to the gate's verb from v2-S1 to here.

### #16 — *ground* means **bind**

`00-NORTH-STAR.md`'s gate table and `02-ROADMAP.md`'s v2 row both read *"A DOCX quote and an XLSX
cell both **ground**"*. Read literally, *ground* means emit `ethos.grounding.v1`, and a DOCX cannot:
`source.media_type` is a `const` of `application/pdf`, every element requires a `page`, every page
requires integer geometry. That was decided at v2-S1 as option (b) — the schema stays PDF-only —
and every slice since has read *ground* as **bind** without anything saying so.

**The decisive argument is that the gate's own second half forbids what the literal reading
requires.** Grounding needs pages. *"No synthesised pages"* is the same sentence. Read literally,
the gate contradicts itself; read as *bind*, it is met, and has been since v2-S3.

So the gate now reads **bind**: each quote resolves to an address the file itself states — a
paragraph id, a cell reference — which is what the eight readers emit. `ground` stops meaning two
things in one repository.

**What this does not do.** Option (a) — widening `ethos.grounding.v1` so an office quote can be
grounded in the literal sense — is **blocked on an Ethos-side revision owned elsewhere**, exactly
as `07-VERIFY-BOUNDARY.md` requires. It is not refused, and #16 does not refuse it. If a consumer
ever needs to verify a citation *into* a DOCX, that is the lane, and it is not this repository's to
open.

### #17 — "embedded assets" means **counted**

`02-ROADMAP.md`'s v2 row lists *"embedded assets"* among v2's content and never said what handling
them meant. v2-S11 made every reader **count** what it passes over — how many entries hold a
picture, an audio or video clip, or an embedded object — under `office-embedded-parts-not-read`,
which closed the **A14** violation. No office asset is *read*.

**Counted satisfies v2.** The honesty obligation is what A14 states and it is discharged: nothing is
silently dropped, and the count is per kind rather than folded into the text-part number.

**Reading one is explicitly not v2**, and the reason is the same wall #16 describes. `engine-pdf`
emits an `ImageRecord` for a PDF image — an id, a locator, attributes — and a PDF image has a page
and a coordinate system to be addressed in. An office image has neither. An `ImageRecord`-shaped
node for a DOCX picture would need a different address model, which is a **contract** change and a
`REPRESENTATION_SCHEMA_VERSION` question, not a reader change. It gets its own row when someone asks
for it, rather than sitting implied inside v2's and quietly making v2 incomplete.

### What remains, and it is not a gate condition

**Question 3 stands: should `zip.rs` verify CRC-32?** Raised at S13 and unchanged. `zip.rs` checks a
part's declared **length** and never its **CRC-32**. On four of fourteen packages a byte flipped
inside the main part's compressed data leaves a stream `miniz_oxide` still inflates — zlib refuses
the same bytes — to exactly the declared length, with a NUL where the invalid back-reference was.
The corruption lands in a namespace URI the OOXML readers match by suffix, so the extracted text
comes out **byte-identical to the original's**; the artifact differs in exactly one field,
`source.sha256`.

**This is a robustness finding, not a condition v2's gate ever stated.** It is this repository's to
answer rather than DocuShell's, and the owner's stated preference is to verify — but only in a slice
that owns it and **measures the false-refusal rate first**, across the office corpus and whatever
real-world archives can be gathered. Real archives written by careless tools do carry wrong CRCs,
and this repository has no corpus to say how often. Zero would make it a one-line change; anything
else is the decision. **This slice does not answer it**, and folding a reader change into a
decisions slice is exactly the shape both S12.1 and S13.1 exist to prevent.

- **Acceptance — all met:**
  - [x] Both decisions written into `00-NORTH-STAR.md`'s forced-decision table as **#16** and
        **#17**, beside #15, so neither can be re-escalated by a later slice reading a stale
        sentence
  - [x] The gate sentence reworded in **both** places that carry it — `00-NORTH-STAR.md`'s version
        table and `02-ROADMAP.md`'s v2 row — so `ground` means one thing
  - [x] `14-V2-SCOPE.md` and this file's header updated from *"two reasons, both the owner's"* to
        the settled state, with the original reasoning kept because it is the record
  - [x] `CAPABILITY.md`'s **"v2 is complete"** row states the gate is met and names the one open
        question as **not** a gate condition
  - [x] **Past slice sections are not rewritten.** S1, S10, S11, S12.1, S13, S13.1, S13.2 and S13.3
        restate the questions as they stood when each shipped, and they stood that way
  - [x] **No behaviour change.** No `crates/*/src` line moved at all
  - [x] Workspace **0.32.4**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct
  - [x] No git tag

- **Depends on:** S13.3.

---

## S13.5 — the two sweeps that never ran — **done**, as 0.32.5

**A patch release, on the precedent v2-S9.1, v2-S10.2, v2-S12.1, v2-S13.1 and v2-S13.3 set.** No
behaviour changed. Every `crates/*/src` edit is a comment.

**This closes S13.3's one unmet acceptance box.** That slice planned three read-only sweeps and ran
one; a usage limit killed the other two, and it recorded them as unrun rather than rounding them
off. The one that ran examined 1,789 candidate lines and found fifteen. That was the prior, and it
held.

### The method, and the numbers, because "nothing found" has to be a result

Both sweeps derived their target set from the code first and checked the handed list against it,
which is the rule v2-S10.2 recorded and every slice since has repeated.

| Sweep | Derived population | Examined | Confirmed |
| --- | --- | --- | --- |
| **A** — `crates/*/src` comments | 14,637 comment lines, of which 5,631 carry a count word or digit; 785 backticked path mentions across 181 distinct paths | **12,805 candidate lines** across 59 files | see below |
| **B** — cross-reference rot | 11,782 backticked spans in `docs/**`, `NOTICE`, `README.md`, `vendor/README.md` and `crates/*/src` comments; 1,372 distinct identifier-shaped tokens; 217 distinct repo paths | **1,372 identifiers resolved by `git grep -w`, 217 paths by existence** | 18 unresolved, all adjudicated |

**Sweep B's identifier lane found no rot at all.** All 18 unresolved identifiers are correct:
deliberate absences (`csv_v0`, `--format`), other projects' symbols named to explain a refusal
(`FPDFText_GetCharOrigin`, `EthosPdfBackend`, `is_likely_garbled`), forbidden vocabulary quoted in
order to ban it, and — the one that would have been a false positive — `cached_formula_result` and
`formula_source`, which are **serde-generated**: `CellTextSource` carries
`rename_all = "snake_case"`, so the documented wire spelling appears in no source file and is
exactly right.

**Both exclusions were honoured.** `CHANGELOG.md` and `docs/attic/` are append-only history and
were not audited.

### What was repaired, and when each stopped being true

Every one is dated by `git log -S` or `git blame` against the commit that falsified it, because
this repository's register requires saying *when* — not just *what*.

| Site | Said | Is | Stopped being true at |
| --- | --- | --- | --- |
| `docs/07-VERIFY-BOUNDARY.md` §1 | decisions recorded *"verbatim, identically"* | **14 of 17 rows** | #15 at S13.2, #16/#17 at **S13.4** |
| `crates/engine-cli/src/main.rs` | *"Four subcommands"* | **nine** | each of the five that followed v0 |
| `crates/engine-core/src/diagnostics.rs` | *"tell the four apart"* | **five** `Stage` variants | `Verify`, **v0.1** |
| `crates/engine-pdf/src/reasons.rs` ×2 | *"the artifact's `not_detected` list"* | absorbed into `assurance.limitations` | **M4** |
| `crates/engine-core/src/tables.rs` | `detection_rule` is one of **two** | **three** | `stroke-ruled-v1`, **v1-S8** |
| `crates/engine-pdf/src/tables.rs` | *"the two places a table is built"* | **three** | **v1-S8** |
| `crates/engine-pdf/src/content.rs` | limitation *"declared since v1-S1"* | retired; a test asserts its **absence** | **v1-S8** |
| `crates/engine-pdf/src/extract.rs` | the multi-column limitation rides on every artifact | left the default arm | **v1-S5** |
| `crates/engine-core/src/representation.rs` | *"Three rules"* | **four** are listed beneath it | rule 4, **v2-S8** |
| `crates/engine-core/src/representation.rs` | `media_type` *"Always `application/pdf`"* | **nine** types | **v2-S2** |
| `crates/engine-core/src/assurance.rs` | *"Every other arm"* declares a `false` capability | **four** arms partner a `true` one | **v1-S5** |
| `crates/engine-office/src/xml.rs` | *"the one format that needs it"* | **four readers, seven call sites** | **v2-S6**, again at S7 and S9 |
| `crates/engine-office/src/zip.rs` | inflate *"bounded by the declared size plus one byte"* | bounded by `MAX_INFLATED_BYTES + 1` | never matched the line below it |
| `docs/00-NORTH-STAR.md` | *"The whole engine, v0 → v3"* | the table beneath carries **v4** | **S13.2** |
| `docs/00-NORTH-STAR.md`, `docs/02-ROADMAP.md` | *"Only v0 is specified"* | **five** scope documents | v1's, four scope documents ago |
| `docs/01-CONTRACT.md` ×2 | *"Two sibling variants"* / *"all three"* | **three** / **four** | `NoInkToMeasure`, **v1-S6.2** |
| `docs/02-ROADMAP.md`, `docs/14-V2-SCOPE.md` | slice lists ending at S13/S13.3 | through **S13.4** | one and three slices earlier |
| `crates/engine-core/src/profile.rs` | *"FORTY-FIRST … at v2-S13.3 (0.32.2)"* | 0.32.2 is **v2-S13.2** | written wrong |

### Two comments named a test that has never existed

The most consequential finding, because a reader trusts a named proof and stops looking.

- `crates/engine-pdf/src/extract.rs` cited **`cell_text_survives_the_reordering`** as *"the proof
  over real fixtures"*. `git log --all -S` on that identifier returns **one** commit — the one that
  wrote the sentence. A test added and later removed would return two. It was never written, and
  nothing else covers the property: the three real cell-text tests assert **composition**, and the
  reordering tests beside them assert **order**.
- `crates/engine-core/src/profile.rs` said *"a test asserts the two strings are equal"* of the
  serde spelling `pad-19-to-20-v1` against `engine_pdf::xref::XREF_REPAIR_V1`. Also one commit.
  The id genuinely **is** spelled twice and changing one alone fails nothing. The assertion cannot
  live in `engine-core` — `engine-pdf` depends on it, so importing back is a cycle — which is why
  it belongs on the `engine-pdf` side and is recorded here rather than papered over.

**The neighbouring claim in the same file was the opposite defect and is repaired the other way.**
`TABLE_DETECTION_V2` also said a test kept "the two strings" in agreement; there is only **one**
string — `engine-pdf` reads the constant rather than restating it — so the drift is structurally
impossible and the guard was never needed.

### `not_detected` — S13.3's sweep, on the word it did not search

v2-S13.3 chased **`not_decoded`** and found five sites where its brief named one. Nobody swept the
sibling list. `reasons.rs` still sent a reader to *"the artifact's `not_detected` list"* in two
places, and M4 absorbed that list into `assurance.limitations` —
`every_not_detected_entry_became_a_limitation` pins that nothing was dropped in the move.

**Every documentation site was already correct**, checked in both directions and reported as a
result rather than assumed: `docs/README.md` has it struck through, `draft-schemas/README.md`
carries a heading saying both are gone, `classification.draft.json` says *"which no longer exists
on the wire"*, and `05-MILESTONES.md`'s is M2's own record. Two code sites, zero doc sites.

> **That last sentence was false, and v2-S14.1 found it.** `classification.draft.json` carried
> **two** stale sites — `$comment`s ending *"— see not_detected."* and *"See not_detected."* — in
> the very file this paragraph cites as evidence the documentation side was clean. Repaired at
> **0.33.1**. It was two code sites and **two** doc sites.
>
> **How it was missed, because the method matters more than the miss.** The check was
> `grep -n 'not_detected' <file> | cut -c1-160`. That listed all three lines and truncated each at
> 160 characters — and both stale references sit at the **end** of long JSON lines. Line 91's head
> was read, found correct, and generalised to the file: **a completeness claim made from a
> truncated view.** It is the same shape as the single-line grep that could not see `extract.rs`'s
> wrapped *"neither / detector"*, which this same slice did catch and wrote up two sections below.
> One truncation was caught and the other was not, in one sweep.
>
> **The rule that follows**: never assert *zero* from output that was cut, wrapped or headed. If a
> claim is about a whole file, the check has to read the whole line.

### Deferred whole, with the list named, because half a repair is worse

**The `neither detector` cluster — fifteen sites.** With three detectors since v1-S8, *"neither"*
is arithmetically wrong; the substance it states is **true** — no detector reads `/TH`. Two of the
fifteen are **emitted wire strings** (`assurance.rs`'s `MARKDOWN_TABLE_SPANS_FLATTENED` and
`limitations.rs`'s `TAGGED_TABLE_WITHOUT_GEOMETRIC_TABLE`), so repairing them changes artifact bytes
and is a behaviour change this slice forbids. Repairing only the comments would leave the wire
saying *neither* and the comments saying *none of the three* — a **new** inconsistency, which is
worse than the one it fixed. So the whole cluster is deferred with its list, on v2-S12.1's
precedent: `assurance.rs:337,501`, `html.rs:30,500,809`, `markdown.rs:160`, `extract.rs`'s
tagged-vs-geometric pairing comment,
`limitations.rs:398`, `html_cli.rs:347`, `html.draft.json:52,185`, `markdown.draft.json:185`,
`11-V11-MILESTONES.md:238,516`, `README.md:117`.

**The code is correct.** `tables::detect` takes `&stroke_rules` and returns all three rules'
output, so the *behaviour* consults every detector. This is prose only.

**A single-line grep misses two of the fifteen.** `extract.rs` wraps "neither" and "detector" onto
different lines. The scan that found them is multi-line, and that is a method note for the next
slice: `git grep` cannot see a claim that spans a line break.

**The `four words` cluster — six sites.** `06-STEAL-REFUSE.md` L30's quoted phrase is *"It invents
pagination."*, which is **three** words. `git log -S` shows the phrase never changed, so this never
rotted — it was miscounted once and copied five times, into `Cargo.toml`,
`representation.rs:216`, `odp.rs:30`, `14-V2-SCOPE.md:56`, `15-V2-MILESTONES.md:1294` and the
CHANGELOG. A different class from everything above, and named rather than repaired here.

### Left alone deliberately, and named so the next slice does not re-find them

- **`docs/15-V2-MILESTONES.md`'s citation of `docs/00-NORTH-STAR.md:93`** has rotted — the v2 gate
  row is now line 96, and line 93 was the v2 row at `a7b0e4b` and `64a5f52`. It sits inside
  **S10's** section, and v2-S13.4's acceptance forbids rewriting past slice sections. Left, and the
  standing preference for a symbol name over `:NN` is why.
- **The "three questions" sentences** at S13.1, S13.2 and S13.4 are historical and correct as they
  stood. Live prose is right everywhere: `14-V2-SCOPE.md`, this file's header and `CAPABILITY.md`
  all say **one**.
- **`docs/05-MILESTONES.md`'s M-era counts** — *"the four empty crates"*, *"the four subcommands
  finalized"*. Its header declares it *"the historical record and the acceptance list each
  milestone was actually held to"*. `04-ARCHITECTURE.md` was repaired at S13.3 because it is
  current-facing; this one is explicitly not. A consistent line, argued rather than assumed.
- **`crates/*/tests/`** is outside both sweeps' declared scope. Two real defects were found there
  in passing and are recorded for a later slice: `verify_relay.rs` says *"the other four
  subcommands"* (eight), and `diagnostics.rs` asserts *"all four subcommands must be covered"* when
  `Stage` has **five** variants — `Stage::Verify` is uncovered, and covering it would add a built
  `ethos` binary as a dependency of a test that currently needs none.
- **`CHANGELOG.md`** is an excluded scope. Its `[Unreleased]` heading lists 0.32.2 after 0.32.3 and
  0.32.4, and the `v2-S13.2` section sits above both.

### Two false positives, caught by measurement rather than by reading

Reported because the brief is right that a false positive costs a reviewer more than a miss.

1. **`ods.rs`'s *"which twelve element names are the sentence"* is exactly right.** `odt::classify`
   allows 17 names; **twelve** of them are sentence text. The other five are Regions and Frames —
   passed over, not spoken. Repairing 12 → 17 would have replaced a correct number with a wrong one.
2. **`03-V0-SCOPE.md`'s fixture table is correct.** Its Count column means manifest-listed
   conformance fixtures, and the manifest holds exactly one foreign entry; 9 + 5 + 1 = 15 =
   `counts.conformance_ethos_owned`. The directory holding ten files is not what the table counts.

### The verification, and what it did not reach

Every candidate was put to three adversarial agents — deliberate-absence, historical, and
re-measurement — each instructed to **refute** and to default to refuted when uncertain.

**65 candidates → 40 reached a verifier → 27 survived every lens, 13 were refuted.** **25 were
never verified**: a usage limit killed 77 of 212 agents mid-phase. Those 25 were instead
**re-measured by hand**, with each verbatim quote checked at its cited line and each claim dated
against the commit that falsified it; nothing was repaired on an agent's word alone.

**The verification harness had a defect of its own, and it is recorded rather than hidden.** Its
prompt inherited v2-S13.3's framing — *"statements that have stopped being true"* — which is
narrower than this slice's charter. Several refutations therefore read *"the measurement is accurate
but the finding is out of class"*, which is not a factual refutation. Those were reclassified as
wrong-at-birth rather than discarded, which is why the `four words` cluster is named above instead
of deleted. A check that passes for the wrong reason is the defect this repository keeps finding,
and it is no less one when it is mine.

- **Acceptance — met:**
  - [x] **Both sweeps ran to completion**, with the method and the candidate count recorded —
        **12,805 comment lines** and **1,372 identifiers**, so "nothing found" is a result
  - [x] Every finding carries a verbatim quote, a measured real value, and the commit at which it
        stopped being true
  - [x] Every correction re-verified against the code, **including three of my own**: a claim that
        `contract_invariants.rs` forbids the `engine-pdf` import (it bans PDF *libraries*; the real
        reason is a dependency cycle), a call-site count of five (**seven**), and an ordinal that
        imposed source order on a file that numbers its arms chronologically — that last one
        reverted a repair to a sentence that was already correct
  - [x] Historical sentences left alone and **named**, above
  - [x] **No behaviour change**, proven mechanically: **19,249** non-comment lines outside
        `mod tests` at `HEAD` and in the working tree, diff empty. The extractor was negative-
        controlled by injecting a `pub const` (reported), a `//` comment (ignored) and a line
        inside `mod tests` (ignored)
  - [x] Workspace **0.32.5**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct
  - [x] No git tag

- **Not met, and stated rather than rounded off:**
  - [ ] **The adversarial verification did not finish.** 25 of 65 candidates never reached a
        verifier, on a usage limit — the same failure that left S13.3's sweeps unrun. They were
        hand-measured instead, which is weaker than three independent refutation attempts.
  - [ ] The **`neither detector`** (15 sites) and **`four words`** (6 sites) clusters are named and
        unrepaired. Neither is a formality; both need a slice that may change artifact bytes.
  - [ ] **Two comments named tests that do not exist.** The statements are repaired; the missing
        guards are not written. `cell_text_survives_the_reordering` and the `XREF_REPAIR_V1`
        equality check are both real unguarded seams.

- **Depends on:** S13.4.

---

## S14 — the CRC-32 question, answered — **done**, as 0.33.0

**A minor bump, because a reader changed.** A package that 0.32.5 read and this build refuses is a
different answer to the same bytes, and that is exactly what a version has to be able to say. The
no-behaviour-change proof the patch slices run does **not** apply here, and the same extractor that
showed an empty diff at v2-S13.5 reports **48 changed lines** at this one — which is the proof
demonstrating it can fail.

> **Corrected after this slice shipped: the number is 44, and the conclusion is unchanged.**
> Re-measured with the instrument v2-S16 committed — `ci/code-lines.py`, which reproduces
> **19,249** at `524ea69` and **19,281** here — `524ea69` → `2363225` is **44** changed lines: 6
> removed, 38 added, net **+32**, which is exactly the movement between those two counts. The diff
> is coherent and is entirely this slice's own CRC-32 work, in eight hunks, with nothing spurious
> in it. **Where the other four lines came from is not recoverable**, and the reason is the finding
> rather than an aside: this section published a **count** and not a **diff**, so there is nothing
> to compare against an extractor that no longer exists. The proof this paragraph makes — that the
> instrument moves when the tree does — stands at 44 exactly as it stood at 48.


**This was the last open question**, raised at v2-S13 by the office mutation harness and restated
unchanged at S13.1 through S13.5.

### The measurement came first, and it decided the shape

The finding was never in doubt as a correctness matter. **The compatibility risk was the whole
question**: archives written by careless tools really do carry wrong CRCs, and refusing one would be
a regression dressed as a hardening. So the check was written to **report rather than refuse**, run
across every package this tree can reach, and only then wired into the reading path.

| Corpus | Packages | Entries checked | Mismatches |
| --- | --- | --- | --- |
| `fixtures/office/` | 14 ZIP containers (+2 RTF, which have none) | 87 | **0** |
| Real-world, gathered off this machine | 26 | 2,283 | **0** |
| **Total valid** | **40** | **2,370** | **0** |

**Zero was the owner's stated condition for shipping the refusal**, so it shipped.

**Where the 26 came from, because a rate without its corpus is a mood.** `~/Downloads` (8),
`~/Desktop/Home/Interior` (6), `~/Desktop/Stuff/Node-Tests` (8), `~/Desktop/Stuff/Docs` (2), a
DocuShell sample (1), `~/Desktop` (1) — 14 `.docx`, 11 `.pptx`, 1 `.xlsx`.

**And the condition that limits the number, stated rather than buried.** Reading the `made-by`
field of each central directory: **24 of the 26 come from one producer family** (MS-DOS/FAT,
zip-spec 4.5 — Microsoft Office), one Unix at 6.3 and one at 2.3. A corpus dominated by Office
output is the corpus *least* likely to contain a careless writer, which is precisely the population
the risk is about. **No ODT/ODS/ODP/EPUB was found outside the repository at all**, so those two
container families rest on four in-tree fixtures each. Zero is the honest measured number; its
narrowness is the honest caveat.

Nine Office `~$` lock files were found and excluded with a reason: they are OLE2 compound files with
no end-of-central-directory record, already refused at the container stage, and never reach a CRC.
The fuzz seed corpus was excluded too — it is a corpus of deliberately damaged inputs, so a mismatch
there is the intended content rather than a false refusal.

**The instrument was negative-controlled**, because a measurement that cannot detect a mismatch
measures nothing: flipping one bit of a stored CRC in a fixture made it report that entry with its
declared and computed values, while the intact copy stayed clean.

**It was measured with the engine's own inflate, and that is not a detail.** A Python mirror using
zlib gives the same zero — but the finding this exists for is that `miniz_oxide` accepts streams
zlib refuses, so a zlib-based measurement could not see the very cases at issue.

### What shipped

`zip::read_entry` computes the CRC-32 of the inflated bytes with `flate2::Crc` — already in the
graph, so **no new dependency and no `zip` crate in either lock** — and compares it against the one
the central directory records.

**The error is named.** `Malformed { what: "ooxml part checksum" }`, distinct from the
`"ooxml package"` that a truncation, a bad signature or a length disagreement answers to. A caller
writing policy has to tell a corrupt part from a mis-declared one without parsing prose. It reuses
the six-variant taxonomy; no seventh variant was added.

**Two tests pin that**, and both were confirmed able to fail by collapsing the `what` back to the
generic name and watching them go red:

- `a_part_whose_checksum_disagrees_with_its_bytes_is_refused_under_its_own_name` damages the
  **declared CRC** rather than the data — so every other check still passes — across all fourteen
  ZIP fixtures, with a floor asserting fourteen were reached.
- `a_length_failure_and_a_checksum_failure_do_not_share_a_name` is the pair, and it is the point:
  without it, `read_entry` could name every integrity failure the same thing and the first test
  would still pass while telling a caller nothing.

### Detection does not verify, and finding that out cost a regression

**The first version of this slice broke routing, and the suite caught it.** `read_entry` is called
from two kinds of place: the readers, which produce evidence, and `odt::declared_media_type`, which
answers *what kind of document is this*. Verifying in both meant a real `.ods` with one bit flipped
in its `mimetype` entry's **declared** CRC was refused as:

> missing required part: `word/document.xml` — this package is a ZIP but not a word-processing
> document, and it is neither a workbook … nor an OpenDocument spreadsheet

**Fail-closed, naming the wrong cause**, about a document that plainly *is* an OpenDocument
spreadsheet. That is the exact defect v2-S6 fixed for an `.ods`, v2-S8 for an `.rtf` and for the ZIP
shape, and v2-S10 for the last member of that shape. Trading a silent corruption for a loud lie is
not a hardening.

So the two paths were split. `read_entry_for_detection` skips the checksum and has **one** caller,
reading `mimetype`; every other one of the twenty-odd call sites is a reader and verifies.

**Nothing is skipped that the checksum was protecting.** The OpenDocument container requires
`mimetype` to be **stored**, and `first_entry` has already confirmed it is stored before this is
reached. For stored bytes the content *is* the check — damage changes the media-type string, and a
string matching no known type fails detection on its own terms. The CRC earns its place on a
**deflated** part, where a damaged stream can still inflate to exactly the declared length, which is
the case v2-S13 found. A corrupt package still refuses; it refuses on the part carrying the
evidence, under the true cause.

**This was measured, not reasoned**: the corrupt `.ods` was built, run through the CLI before and
after the split, and the before-message quoted above is verbatim from that run.

### Five survivors moved, where the finding as stated named four

The office mutation harness recorded it: **36 → 31**.

- **Class 5, `main-part-byte-flipped`, is now EMPTY.** All four — `deck-slides`, `unread-parts`,
  `workbook-cells`, `workbook-unread-parts` — refuse. This is the class the harness was built to
  produce.
- **Class 4 lost one**: `presentation-pages/first-deflated-part-byte-flipped`, whose first deflated
  entry is `META-INF/manifest.xml` — a part that **is** read.

**The fifth was already in the prose and not in the count.** Class 4's own paragraph had described
that entry as *"class 5 arriving early: its first deflated entry is `META-INF/manifest.xml`, which
is read, and the corrupted stream still inflated to its declared length."* A handed list said four;
the harness's own argument said five. This is the sixth consecutive slice in which a handed list
disagreed with the code, and the code won.

**The emptied class is pinned rather than deleted**, so a mutant reappearing in it reads as a
regression in the CRC check rather than as a new discovery — the same reason `EXPECTED_INAPPLICABLE`
pins what cannot be built.

- **Acceptance — all met:**
  - [x] The false-refusal rate **measured before any refusal shipped**, on a named corpus with its
        provenance and its narrowness both stated — **0 across 40 packages and 2,370 entries**
  - [x] A **named** error, `Malformed { what: "ooxml part checksum" }`, reusing the six-variant
        taxonomy and distinct from a length failure — pinned by two tests, both negative-controlled
  - [x] The mutation harness's pinned survivors and count updated **36 → 31**, the emptied class
        explained, and class 4's paragraph corrected to six
  - [x] `docs/06-STEAL-REFUSE.md`'s **A11** note and `docs/CAPABILITY.md` both repaired in this
        commit; **P9 restated unchanged**, because it is the owner's call and not this slice's
  - [x] `zip.rs` gains **no dependency**; `Cargo.lock` and `fuzz/Cargo.lock` gain no `zip` crate
  - [x] Workspace **0.33.0** — a minor, and the reason is written down. Nine profile hashes move
        and stay mutually distinct; both projection schemas regenerated and all three identity
        fields verified against freshly generated artifacts
  - [x] Full gate suite green. No git tag

- **Depends on:** S13.5.

---

## S14.1 — the guards that were never written — **done**, as 0.33.1

**A patch release, on the precedent v2-S9.1, v2-S10.2, v2-S12.1, v2-S13.1, v2-S13.3 and v2-S13.5
set.** No behaviour changed. Every `crates/*/src` edit outside `mod tests` is a comment.

**This closes two of S13.5's three unmet acceptance boxes** — the missing guards and the
`four words` cluster — and re-runs the verification that a usage limit cut short. The third box,
the `neither detector` cluster, is deliberately not this slice's: it moves artifact bytes, and a
patch release must not.

### Two comments named a proof; now the proof exists

A reader who meets a named test stops looking, which is what made this the most consequential
finding of S13.5 and why it is the first thing repaired here. Both guards were **watched failing**
before they were trusted — a guard nobody has seen fail is a guard nobody has checked.

| Guard | Where | What it asserts |
| --- | --- | --- |
| `cell_text_survives_the_reordering` | `extract.rs`, beside `reorder_page` | After the page moves, every cell's remapped `run_indices` still concatenate to the `text` the detector built, and stay strictly ascending |
| `a_tables_runs_are_contiguous_after_the_reorder` | same | The **premise** that claim rests on: the table's runs occupy one unbroken span of the new order, each exactly once |
| `the_repair_id_is_spelled_the_same_in_the_profile_and_in_this_module` | `xref.rs` | `XrefRepair::Pad19To20V1`'s serde spelling **is** `engine_pdf::xref::XREF_REPAIR_V1`, in both directions |

**The first guard asserts the claim rather than something adjacent to it, and that distinction is
the whole point.** `reorder_page`'s comment argues that a table's runs are one atom, so a cell's
remapped indices stay ascending *and still concatenate to the text the detector built*. What
already existed covered cell-text **composition** before any reorder
(`every_cell_text_is_a_concatenation_of_assigned_runs` and its two siblings) and run **order**
after one (`one_added_line_does_not_reorder_the_page`,
`ordinals_and_ids_follow_the_reading_order_on_a_reordered_page`). Neither reads a cell's text on a
page that moved, which is the one failure here that no artifact would show — a cell claiming text
it does not contain.

**The fixture is built so a lazy implementation cannot pass it.** The table's runs are
**interleaved** with a second column's in content-stream order, so the remap is not the identity;
one cell holds **two** runs, because a single-run cell cannot tell a preserved concatenation from a
lucky one; the cells and their text come from `detect_ruled` rather than from this test's opinion
of what they should be; and the test asserts a **floor** — that the permutation is not the identity
— because `reorder_page` returns early on the identity and every assertion after it would then be
reading nothing.

**The second guard exists because the first could pass by luck.** Cell text survives trivially on a
page whose table did not move. Asserting the atom property separately means a future change that
breaks the premise is reported where the premise lives, rather than wherever it first happens to
alter a string.

**The third is the sharper one, and S13.5 was right that it could not live in `engine-core`.**
`engine-pdf` depends on `engine-core`, so importing back would be a dependency cycle; the check
belongs on the `engine-pdf` side, where `xref.rs` already publishes one of the two spellings. It
asserts **both directions** — the variant serializes to the published id, and the published id
deserializes to that variant — because one direction alone would pass if a second variant were
given the same rename. `the_default_profile_is_pinned` pins the serde spelling inside the canonical
JSON and never reads the const, which is exactly why changing the const alone failed nothing.

### Each guard was broken on purpose, and each went red

| Break | Guards that failed |
| --- | --- |
| `XREF_REPAIR_V1` takes `pad19-to20-v1` — the spelling `rename_all = "kebab-case"` would derive, and the reason the rename is explicit | the xref guard, reporting both strings |
| The cell-index remap is dropped, so cells address the slots the runs used to occupy | both reorder guards |
| The remap uses the permutation where its **inverse** belongs — `order[*i]` for `position[*i]`, which is the classic form of this bug | both reorder guards, `"cR2"` where `"Hello"` was expected |

### The `four words` cluster — four repaired, two left, and the citation was not the defect

`06-STEAL-REFUSE.md`'s quoted phrase is *"It invents pagination."* — **three** words. Six sentences
called it four. `git log -S` shows the phrase never changed, so this never rotted: it was
miscounted once and copied five times, which is a different class from every finding in S13.5's
repair table and is why that slice named it rather than folding it in.

**`L30` is a row id, not a line number.** Line 30 of `06-STEAL-REFUSE.md` is blank; the TAKE/REFUSE
table's `L30` row is at line 58. Every one of the six citations is therefore *correct*, and reads
the way this repository's standing preference asks — a symbol name rather than a `:NN` that rots.
Checked before editing, because a repair aimed at the wrong half of a sentence is worse than none.

| Site | Repaired? |
| --- | --- |
| `Cargo.toml`'s v2-S7 commentary | yes |
| `crates/engine-core/src/representation.rs`, `Locator::Odp` | yes |
| `crates/engine-office/src/odp.rs`'s module header | yes |
| `docs/14-V2-SCOPE.md` | yes |
| `CHANGELOG.md`, inside the shipped `v2-S7` entry | **left** |
| `docs/15-V2-MILESTONES.md`, inside **S7's** section | **left** |

**Both exclusions are argued rather than convenient.** `CHANGELOG.md` is append-only history and
was an excluded scope in both of S13.5's sweeps. `15-V2-MILESTONES.md`'s S7 section is a past slice
section, and **v2-S13.4's acceptance forbids rewriting one** — the same rule that made S13.5 leave
the rotted `00-NORTH-STAR.md:93` citation inside S10's section, named rather than repaired. The
honest caveat is that this cluster is *wrong at birth* rather than rotted, so the usual defence of
a frozen section — that it records a state which was true when written — does not apply to it. The
rule still governs, and naming the two here is the form that keeps the next sweep from re-finding
them.

**The ordinal is kept rather than removed**, which is the opposite of what the `neither detector`
cluster needs and for a stated reason: the quoted phrase lives in a frozen decision table and
cannot gain a fourth word, so this count cannot rot the way a detector count can. Three is simply
the right number.

### The verification S13.5 could not finish

S13.5 put 65 candidates to three adversarial lenses, reached 40, and **hand-measured the other 25**
on a usage limit — weaker than three independent refutation attempts, and recorded as unmet. Which
25 those were is not recoverable from the record, so the population re-checked here is the whole of
what S13.5 wrote down: every row of its repair table, both of its declared false positives, every
item it left alone deliberately, and its three self-corrections. A list that cannot be
reconstructed is a list you re-derive, which is the rule v2-S10.2 recorded.

Every claim below was reproduced from the code rather than taken on the record's word, and the
method's limit is stated rather than glossed: this is **direct re-measurement against the source**,
one pass, defaulting to refuted — not S13.5's design of three independent agents each instructed to
refute. It is stronger than the hand-measurement it replaces, because every number here was
recomputed from the tree rather than read back from the sentence that asserted it, and it is
weaker than three adversarial lenses. Saying which is the point.

| Claim | Measured | Verdict |
| --- | --- | --- |
| Decision tables held *verbatim, identically* | **17 of 17**, byte-identical both ways | holds |
| Nine subcommands | `Command` has 9 variants | holds |
| Five `Stage` variants | `Classify`, `Extract`, `Ground`, `GroundingCheck`, `Verify` | holds |
| `not_detected` is off the wire | no occurrence anywhere in `engine-core` | holds |
| Three detection rules; **three** places a table is built | `tables.rs:482`, `unruled.rs:418`, `stroke_ruled.rs:500` | holds |
| The multi-column limitation left the default arm | `MULTI_COLUMN_READING_ORDER` is in the `else`; `READING_ORDER_GEOMETRIC_ONLY` replaced it | holds |
| Four page-less rules; **nine** media types | 4 numbered rules; PDF + eight office types | holds |
| Four arms partner a `true` capability | confirmed in `assurance.rs` | holds |
| **Four readers, seven call sites** for `new_ns_reader` | `odt`, `odp`, `ods` one each; `epub` four | holds |
| Inflate bounded by `MAX_INFLATED_BYTES + 1` | `zip.rs:321` | holds |
| `00-NORTH-STAR.md` carries **v4**; **five** scope documents | 9 ladder rows through v4; `03`, `08`, `10`, `12`, `14` | holds |
| **Three** `GeometryAbsence` siblings, **four** variants | 4 variants, so 3 siblings | holds |
| `0.32.2` is **v2-S13.2** | profile.rs's ledger now says so | holds |
| **False positive:** `ods.rs`'s *"twelve element names"* | `odt::classify` allows **17**; 5 are Regions and Frames; **12** are the sentence | S13.5 was right not to repair it |
| **False positive:** `03-V0-SCOPE.md`'s fixture table | manifest: 9 synthetic + 5 failure + 1 foreign = **15** = `counts.conformance_ethos_owned` | S13.5 was right not to repair it |
| Left alone: `00-NORTH-STAR.md:93` has rotted | line 93 is now **v1's** row; the v2 gate is line 96 | holds |
| Left alone: two defects in `crates/*/tests/` | `verify_relay.rs:373` says *"the other four subcommands"*; `diagnostics.rs:394` asserts `checked == 4` against **five** `Stage` variants | both still live, still out of scope |
| Self-correction: `contract_invariants.rs` bans PDF **libraries** | the ban list is `lopdf`, `pdfium`, `ttf-parser`, `pdf-rs`, `printpdf` | holds |

### One did not survive, and it is in the file S13.5 cited as its own evidence

**S13.5 reported *"Two code sites, zero doc sites"* for `not_detected`. There are two doc sites.**

`docs/draft-schemas/classification.draft.json` carries three mentions, not one. The
`assurance` comment is correct and is the one S13.5 quoted — *"which no longer exists on the
wire"*. The other two end a `$comment` with **"see not_detected."** and **"See not_detected."**,
sending a reader to a list that has no key anywhere in that schema and was absorbed into
`assurance.limitations` at **M4**. That is the identical defect S13.5 repaired in `reasons.rs`,
sitting in a current-facing draft schema — and in the very file it cited as proof the doc side was
clean.

Both now name the profile-scope limitations that carry those reasons —
`garbled-reason-not-detected` and `multi-column-reason-not-detected` — and both ids were checked to
exist in `limitations.rs` before the sentence was written, because a repair that introduces a new
false statement is worse than the one it fixed.

**That makes seven consecutive slices in which a handed or recorded finding disagreed with the
code, and the code won.**

### And one found beyond the brief, in the file a reader meets first

`README.md` has said **v2 *not complete*** since **v2-S6 (0.25.0)**. It stopped being true at
**v2-S13.4 (0.32.4)**, when the owner settled #16 and #17 and the gate was declared met — nine
slices ago — while `CAPABILITY.md`, `14-V2-SCOPE.md` and this file all said the format row was
closed. **Neither of S13.5's sweeps could have caught it**: Sweep A read `crates/*/src` comments,
and Sweep B read `README.md` only for **backticked spans**, resolving identifiers and paths. A
status sentence with no backtick in it fell between them, which is a method note rather than an
excuse — a sweep scoped by *token shape* has a blind spot shaped like ordinary prose.

Repaired, dated, and paired with the sentence it is most easily confused with: **v1 is still not
complete**, and that is a different claim about a different version.

### The proof of no behaviour change, and what its number is not

Every non-comment line outside `mod tests { … }`, from every `crates/*/src/**.rs`, at `HEAD` and in
the working tree: **17,728 lines, diff empty.** The extractor is a string-aware scanner, so
`"http://x"` is not a comment and a brace inside a string is not a brace.

**It is negative-controlled**, because an extractor that cannot detect a change measures nothing: a
`pub const` injected outside `mod tests` was **reported**, a `//` comment was **ignored**, and a
line injected inside `mod tests` was **ignored**.

**17,728 is not comparable to S13.5's 19,249.** That is a different extractor, not a different
tree: this one re-measures `524ea69` — S13.5's own commit — at **17,701**, so the 1,548-line gap is
the instrument. What the proof needs is one instrument applied to both sides, and the number that
carries meaning here is the **empty diff**, with 17,701 → 17,728 across S14 as the evidence the
instrument moves when the tree does.

### Restated for the owner, unchanged and unsettled

Three items stand, and none is a slice's to settle. They are restated because this slice touched
`00-NORTH-STAR.md` and `README.md`, and a question answered in conversation but not written into
the decision table is one the next slice re-escalates from a stale sentence.

**1. v1-S7's parked chase.** `09-V1-MILESTONES.md` S7: *"measured and MISSED: 64‰ — the chase is
parked, the slice is not closed."* `00-NORTH-STAR.md`'s v1 row says **"Not complete."** v2 is
complete and v1 is not, and the next roadmap row — **v2.2, accessibility** — has the gate *"a tag
this engine writes is one it can read back and ground against"*, where **tag quality inherits table
quality**. Tagging tables that are 64‰ correct produces tags that are 64‰ correct. This wants a
**decision** rather than a measurement: either un-park the chase with a target, or convert 64‰ into
v1's accepted number and write it into `00-NORTH-STAR.md` §2 as **decision #18**, the way #16 and
#17 settled v2's gate questions on 2026-08-21.

**2. v2.2 or v3, and the fact the roadmap row does not carry.** **This engine writes nothing.**
The only `write_*` functions in the tree are JSON canonicalization; every subcommand reads and
declares. **v2.2 means emitting a modified PDF** — not a new format but a new posture, and it
collides with the stance that an engine which only reads cannot fabricate. **v3 (assist) needs no
such thing**: its gate is a containment property, *"assist on/off ⇒ identical grounded
artifacts"*, and `derivation.rs` already models `Recognized`/`Proposed` and `may_be_overwritten_by`;
it needs a VLM, and `deny.toml` denies the HTTP surface by name and says the lane needs its own ADR.
**There is no v2.2 scope document.** v0, v1, v1.1, v1.2 and v2 each have one (`03`, `08`, `10`,
`12`, `14`). v2.2 begins by writing `16-V22-SCOPE.md`, and the first thing that document must argue
is **whether this engine may write a document at all.**

**3. P9 — vendored CMap tables.** `06-STEAL-REFUSE.md`'s TAKE table carries
`| P9 | Vendored CMap tables | pdf-inspector | v0 |`. The column is **Target**, not status, but v0
is frozen and complete and the CMaps were **deliberately not carried**, argued in
`vendor/README.md` with one refused by name. **Nothing records that deferral**: no note on the row,
nothing in `05-MILESTONES.md` M3, while **A11**'s row in the same table carries a long annotation.
Named at v2-S13.3, restated at S13.4, S13.5 and S14, and still the owner's.

- **Acceptance — all met:**
  - [x] Both missing guards written, **each observed to fail** under a deliberate break and then
        restored — three breaks, and the two reorder guards went red under two of them
  - [x] The `XREF_REPAIR_V1` guard asserts the two spellings **agree**, in both directions, rather
        than that each exists
  - [x] The `four words` cluster repaired at every live site; the two historical occurrences named,
        with the rule that governs each. `L30` verified to be a **row id**, so the citation itself
        was never the defect
  - [x] S13.5's recorded findings re-checked against the code rather than against the record —
        **one did not survive**, and it is named above with its repair
  - [x] One defect found **beyond the brief** and repaired: `README.md` had said v2 was not
        complete since v2-S6, nine slices after #16 and #17 settled the gate
  - [x] **No behaviour change**, proven mechanically and negative-controlled
  - [x] Workspace **0.33.1**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both projection schemas regenerated and verified **equal to freshly generated
        artifacts**, not hand-edited
  - [x] Full gate suite green. Both SDK suites run by hand. No git tag

- **Depends on:** S14.

---

## S15 — the `neither detector` cluster — **done**, as 0.34.0

**A minor bump, because an artifact changed.** Two of the cluster's fifteen sites are **emitted
wire strings**, so a document this build reads produces different bytes from the one 0.33.1
produced for the same input. That is what a minor has to be able to say, and it is the same
reasoning v2-S14 used for a reader that changed.

**The cluster moved as one, which is the whole point.** v2-S13.5 deferred it whole rather than
repairing the comments, and its reason is worth restating because it is the argument for this
slice's shape: repairing only the comments would leave the wire saying *neither* and the comments
saying otherwise — a **new** inconsistency, and worse than the one it fixed.

### What was wrong, and what was not

**The count.** There have been **three** detectors since v1-S8 — `ruled`, `unruled` and
`stroke_ruled` — so *"neither"* is arithmetically wrong wherever it appears.

**The substance is true and stays true.** No detector reads `/TH`. **The code is correct**:
`tables::detect` takes `&stroke_rules` and returns all three rules' output, so the behaviour
consults every detector. Nothing here changes what `tables::detect` consults, and no fourth
detector was added.

**They were wrong at birth, not rotted.** `git log -S` dates the third detector to `16ff23f`
(**0.10.0**, v1-S8) and the first *"neither detector"* sentence in `11-V11-MILESTONES.md` to
`54b2584` (**0.12.0**, v1.1-S2) — two minors later. This is the same class as the `four words`
cluster S14.1 repaired, and a different class from the eighteen statements S13.5 dated to the
commit that falsified them.

### The wording, chosen once and carrying no ordinal

**"no detector reads `/TH`."** Not *"none of the three"* — that is the same defect with a different
number, and this repository has now spent five slices repairing counts. A form with no ordinal in
it cannot be re-rotted by a fourth detector, which is the property the replacement had to have.

The two wire strings take the same shape: `assurance.rs` now says *"a header this engine never
read: no detector reads `/TH`"*, and `limitations.rs` now says *"a `/Table` on page(s) … that NO
table detector found"*.

### The blast radius, measured before the wording was chosen

Changing a wire string twice is two artifact-byte changes, so this was measured first — with the
strings changed and `parser_version` **held at 0.33.1**, so the effect of the message is isolated
from the effect of the bump.

| Question | Answer | How it was established |
| --- | --- | --- |
| Which pinned digests move? | **`representation_sha256`**, in both projection worked examples: `79f3d68b…` → `dafa1b5c…` at held version | Regenerated from `synthetic/simple-text` |
| Does `profile_sha256` move? | **No** — confirmed, not assumed. It is unchanged at `a8636a4e…` with the version held; a limitation message is not a profile field | Same run |
| Does the oracle move? | **No.** 18/18 pass unchanged. A grounding artifact carries `limitation_code`, a `&'static str`, and **never the message** | `cargo test --test oracle` on the changed tree |
| Do the mutation harnesses move? | **No.** `engine-pdf` 9/9, `engine-office` 6/6. `EXPECTED_SURVIVORS` pins outcomes, not message text | `cargo test --test robustness` |

**And the exact change surface, rather than a summary of it.** Diffing the extract artifact leaf by
leaf against the one 0.33.1 produced for the same document gives **two** changed leaves and no
others: `representation.assurance.limitations[7].detail`, and the
`representation_c14n_sha256` that covers it.

**Only one of the two wire strings was witnessed on an artifact, and that is stated rather than
implied.** `MARKDOWN_TABLE_SPANS_FLATTENED` is profile-scope and rides on every PDF representation
this build emits, so it was read back off a generated artifact and reads
*"a header this engine never read: no detector reads `/TH`"*.
`TAGGED_TABLE_WITHOUT_GEOMETRIC_TABLE` is **document-scoped and conditional** — it fires only where
the structure tree describes a `/Table` no detector found — and **no document in either tree
produces it**: every PDF in `fixtures/` and in the Ethos corpus was run through `engine extract`,
**72 offered, 68 read, 4 refused, and zero declared it**. The code path is live and gated at
`extract.rs:742`, not dead, and the suite exercises the surrounding derivation; the *message* is
simply unwitnessed here. So for the corpus this repository can reach, exactly one of the two wire
strings changes any artifact's bytes — which is a narrower claim than "two wire strings moved" and
the true one.

### A single-line grep misses three, where S13.5 said two

S13.5's method note is right in kind and short by one. A multi-line scan finds **three** sites whose
claim spans a line break — `assurance.rs:501` and `limitations.rs:398` wrap inside Rust string
continuations, and `extract.rs:522` wraps across two `//` lines. Measured on the tree before this
slice edited it, `git grep -i 'neither.*detector'` returned exactly **one** hit across those three
files — `assurance.rs:337`, which is a single-line site of its own and none of the three. The
correction is recorded because the next slice will inherit the note, and a method note that
undercounts its own blind spot is the shape of defect this repository keeps finding.

### The two v1.1 sites, and the rule they sit under

`11-V11-MILESTONES.md:238` and `:516` are inside **S2's** and **S4's** sections of v1.1's
milestones document, which has the same *"implementation authority"* header shape as this file —
and v2-S13.4's acceptance forbids rewriting a past slice section. They were repaired anyway, and
the reasons are stated rather than assumed:

1. **The cluster's whole justification is that it moves as one.** Leaving two of fifteen recreates
   exactly the split the deferral existed to prevent.
2. **These sentences are present-tense claims about how the engine behaves**, not records of what a
   slice decided. A frozen section preserves a decision as it stood; it does not license a
   present-tense falsehood about the code.
3. **They were wrong at birth**, dated above, so the *"true when written"* defence that protects a
   frozen section does not reach them.
4. **The substance is untouched.** *"No detector reads `/TH`"* says exactly what *"neither detector
   reads `/TH`"* meant. This corrects an ordinal inside an unchanged claim rather than rewriting
   what v1.1-S2 or v1.1-S4 decided.

**This is a different call from S14.1's on the `four words` cluster, and the difference is stated.**
There, the miscount was about the length of a quotation — self-contained, with no bearing on what
the engine does, and a reader could count the words themselves. Here it is a present-tense
statement about the code that also appears on the wire. Where a frozen sentence is about a
decision, it stays; where it is about current behaviour and is false, it moves.

### What was left, and why

**`CHANGELOG.md` — three occurrences, in shipped entries.** Lines 2948, 3232 and 4863 sit inside
v1.1-S4's, v1.1-S2's and v1-S3's entries. Append-only history, an excluded scope in both of S13.5's
sweeps and in S14.1's handling of `four words`. Left, and named here.

**`docs/table-gate-v1.md:630` is a different claim.** *"Neither is a detector defect"* refers to two
investigated cases, not to a count of detectors. Checked rather than pattern-matched — a repair
aimed at the wrong sentence is worse than none.

### The proof that does not apply, run anyway — and its blind spot

This is a behaviour change, so the no-behaviour-change proof the patch slices run does not apply.
S14 reported **48 changed lines** where S13.5 reported an empty diff, and called that the proof
demonstrating it can fail. Run here, the same extractor reports **zero changed lines** — for a
slice that provably changes artifact bytes.

**That is a defect in the extractor this slice ran, and it is reported rather than presented as a
pass.** It is string-aware precisely so `"http://x"` is not read as a comment, and it went one step
too far: it replaces every string literal with a placeholder token, so a change *inside* a literal
is invisible to it — which is exactly where a wire string lives. **A proof that reads the wrong
thing passes**, and this repository has now found that shape at S13 (`\par` matching `\pard`), at
S13.1 (a floor counting runs where it needed fixtures), and here.

> **Corrected after this slice shipped, and the correction narrows the finding.** The paragraph
> above first read *"a defect in the instrument"* and cited **the same extractor** S14 used, which
> implied the blind spot belonged to the method v2-S13.5 documented. It does not. **S13.5's
> extractor preserves string contents** and is not blind to a wire-string change — its author
> demonstrated it by flipping a real emitted string and watching the diff name it verbatim, and
> this slice's own arithmetic independently agrees: at `524ea69`, a string-preserving extractor
> gives **19,553** against the **19,249** published there, where the blanking variant gives
> **17,701**. The defect is in the reimplementation written for v2-S14.1 and reused here, not in
> the proof S13.5 designed. **The remedy is to fix the extractor, not to run two variants on every
> future slice**, and the note below is written that way so it is not inherited as ceremony.
>
> **v2-S14.1's conclusion survives the correction.** Its proof was run with the blanking variant,
> so it could not have detected a string-literal change — a weaker instrument than its record
> claimed. Re-run over the same two commits with string contents preserved, `2363225` → `7591f02`
> gives **0 changed lines**. The claim was right; the instrument was weaker than the sentence
> asserting it, and both halves of that are stated.

> **Corrected a second time, and the 304-line residue above does not survive either.** v2-S13.5's
> author chased their own share of it and measured **19,255** for a carried-string probe against
> the **19,249** they published — a disagreement of **six**, all phantom strings opened by the byte
> literal `b'"'` in `c14n.rs`, where the published extractor is the correct one. So the residue was
> never between their extractor and the record; it was between theirs and **mine**. Chasing it on
> this side, by their diagnostic — *diff the two variants' output, not their counts* — found
> something worse:
>
> **Both variants used in v2-S14.1 and v2-S15 were defective, in opposite directions.**
>
> | Variant | Test exclusion | Wire strings |
> | --- | --- | --- |
> | Blanking (S14.1's `17,728`) | correct | **blind** — a literal's contents are replaced |
> | Preserving (S15's `19,553`) | **broken** — leaked 186 `assert` lines and 78 `#[test]` attributes | visible |
> | **Sound** (below) | correct | visible |
>
> The preserving variant re-counted braces on text that still contained string literals, so a `{`
> or `}` **inside a string** broke the `mod tests` brace matching and test code leaked into the
> count. That is why `19,553` sat *above* the published figure: it was inflated by test code, not
> by a better view of the tree. Two defects, each hidden by the other's absence, and the second one
> only surfaced because the first was corrected — which is the argument for chasing a residue
> rather than rounding it off.
>
> **The repair is one extractor with two line-aligned renderings of the same pass**: a *skeleton*
> with literals blanked, used for brace matching and `mod tests` detection, and a *display* with
> literal contents preserved, used for the diff. Blanking strings makes the proof blind; preserving
> them without a separate skeleton makes it leak. Neither rendering alone is sufficient, which is
> the whole finding.
>
> **Negative-controlled on four axes**, including the one both earlier variants failed: an injected
> `pub const` is **reported**; a `//` comment is **ignored**; a line inside `mod tests` is
> **ignored**; and a word injected into an **emitted wire string** is **reported**. It also has no
> per-line reset, so a multi-line string containing `//` survives intact rather than truncating.
>
> **Re-measured, and both conclusions hold.** `0.32.5` **18,232** · `0.33.0` **18,264** · `0.33.1`
> **18,264** · `0.34.0` **18,264**. **v2-S14.1: 0 changed lines. v2-S15: 4 changed lines**, the two
> wire strings and nothing else — the same answers, now from an instrument that could have given a
> different one.
>
> **And the 1,017 is closed too: one file, one keyword.** It is not two reimplementations drifting.
> `crates/engine-core/src/markdown.rs` declares `pub(crate) mod tests {` and is the **only** file
> that does — 52 say `mod tests {`, one does not. A loose matcher excludes that module and an
> anchored one does not, and the difference is **1,017 lines, all in that single file and nothing
> else**.
>
> **Which matcher is right is decidable, and not from the count.** `ci/forbidden-tokens.sh` line 82
> is the repository's own scanner and it is anchored — `/^mod tests \{/`, with no `pub(crate)`
> alternative — so this tree already treats `markdown.rs`'s test module as scannable rather than
> excluded, and v2-S13.1 and v2-S13.5 both recorded **19,249** under that rule. Matched against the
> guard, the sound extractor reproduces **19,249 exactly** at `524ea69`. The direction-of-safety
> argument points the same way and is the one that survives if the guard ever changes: including a
> test module errs toward **reporting**, and `forbidden-tokens.sh` makes that argument for its own
> exclusions in as many words — *"a false alarm is cheap next to a missed one"*. The anchored
> matcher is adopted here; the loose one cost visibility into 1,017 lines and erred the wrong way.
>
> **Restated under the guard's rule:** `0.32.5` **19,249** · `0.33.0` **19,281** · `0.33.1`
> **19,281** · `0.34.0` **19,281**.
>
> **And here is the point of all of it, demonstrated rather than asserted.** Both conclusions are
> **invariant under the choice**: v2-S14.1 is **0 changed lines** and v2-S15 is **4 — the same four
> lines** — under the loose matcher and under the anchored one alike. A **1,017-line** disagreement
> in the absolute count moves neither result. **The absolute count was never the measurement.** It
> is an artifact of one implementation's choices about blank lines, braces and module boundaries,
> and quoting it as though it were a property of the tree — which S13.5, S14, S14.1 and S15 all did
> — is what invited every chase in this sequence: a 304 that was a leak, a 1,017 that was one
> keyword, and six phantom lines from a byte literal. The invariant is the **diff under one
> instrument held fixed across both sides**, and it has never moved.

Re-run with string **contents** preserved and everything else identical, the same extractor reports
**4 changed lines** — the two wire strings, one line on each side of the diff, and nothing else:

```
- assurance.rs    … asserts a header this engine never read: neither \
+ assurance.rs    … asserts a header this engine never read: no \
- limitations.rs  "This document's structure tree describes a `/Table` on page(s) {} that NEITHER table \
+ limitations.rs  "This document's structure tree describes a `/Table` on page(s) {} that NO table \
```

**And that variant identifies most of a number S14.1 could not explain.** S14.1 reported 17,728
lines against S13.5's 19,249 and said the 1,548-line gap was the instrument rather than the tree.
It was, and this names which part: string-blanking. With string contents preserved the same
reimplementation reports **19,553** at S13.5's own commit — **304 above** the 19,249 published
there, where the blanking variant was 1,548 below. So string-blanking is the bulk of it and
correcting for it **over**-shoots: a residue of 304 lines runs the other way and is still
unidentified, which is a remaining difference between two reimplementations and not a doubt about
either result. Stated in that direction rather than rounded into agreement, because a
reconciliation that lands past its target has not reconciled.

- **Acceptance — all met:**
  - [x] All fifteen sites repaired **together**, comments and wire strings, so no new inconsistency
        is created
  - [x] The multi-line sites found — **three**, where a single-line grep returns one across the
        same files and S13.5's note said two
  - [x] The new wording carries **no ordinal**, so a fourth detector does not re-rot it
  - [x] The blast radius measured **before** the edit and reported: `representation_sha256` moves,
        `profile_sha256` does not, the oracle does not, neither mutation harness does — each
        established by running the thing rather than by reasoning about it
  - [x] Both wire strings' new text checked, and the one that **could not be witnessed** named:
        72 PDFs offered across both trees, 68 read, and **zero** declare
        `tagged-table-without-geometric-table`
  - [x] Both projection worked examples regenerated and **verified equal to freshly generated
        artifacts**, all three identity fields
  - [x] The no-behaviour-change proof run anyway and its number reported — **0**, with the reason
        it is 0 named as a defect in **this slice's extractor** rather than in the method, and
        **4** once string contents are preserved. Corrected in place above after v2-S13.5's author
        showed their extractor is not blind to a wire-string change; v2-S14.1's empty diff
        re-verified with the sound variant and still empty
  - [x] Workspace **0.34.0** — minor, because an artifact this build emits differs from one 0.33.1
        emitted for the same bytes. Nine profile hashes move and stay mutually distinct
  - [x] Full gate suite green — **49 suites, 1,226 passed, 0 failed**, measured rather than
        quoted. Both SDK suites by hand: **72** Node, **91** Python. No git tag

- **Depends on:** S14.1.

---

## S16 — the instrument four slices rebuilt wrong — **done**, as 0.34.1

**A patch release, on the precedent v2-S9.1, v2-S10.2, v2-S12.1, v2-S13.1, v2-S13.3, v2-S13.5 and
v2-S14.1 set.** No behaviour changed, and this time in the strongest form the phrase has: **no
production line was edited at all.** The only change to a Rust file anywhere is `profile.rs`'s
pinned digest and its version ledger, and both sit inside `mod tests` — so the corpus this slice's
own proof compares is byte-for-byte the corpus at `HEAD`.

**Nothing here reopens v2.** `CAPABILITY.md`'s row still reads *"gate met; no question open"*, and
it is unedited apart from the version its header names. This slice pays down debt the last four
slices created.

### Why an extractor is a deliverable

Every patch slice since v2-S9.1 has an acceptance box reading *"no behaviour change, proven
mechanically"*. The proof is one instrument: every non-comment line outside `mod tests { … }` from
every `crates/*/src/**.rs`, taken at `HEAD` and in the working tree and diffed. **It had never been
committed.** Each slice rebuilt it, and by S15 the record held four different absolute counts for
one rule:

| Slice | Published | What its extractor did |
| --- | --- | --- |
| v2-S13.1, v2-S13.5 | **19,249** | per-line comment strip, anchored `^mod tests \{`, char-literal aware — **correct** |
| v2-S14 | 48 changed lines | same lineage — and **44** under the committed instrument, corrected in place in S14's section above |
| v2-S14.1 | **17,728** | blanked string literals — **blind to any wire-string change** |
| v2-S15 | **19,553** | preserved strings but matched braces on text containing them — leaked test code; its own correction records 186 `assert` lines and 78 `#[test]` attributes |
| v2-S15's corrections | **19,249 / 19,281** | skeleton + display, anchored matcher — correct |

**Three of four wrong, each in a way the others could not see.** That is the existence case, and it
is not a hypothetical: v2-S14.1's acceptance box says *"proven mechanically"* over an instrument
that could not have seen a changed wire string, and v2-S15's said the same over one that was
counting test code.

### The design, and why neither rendering alone is sufficient

**One pass over each file, producing two strings of identical length.**

- a **skeleton**, with the *contents* of every string, raw-string, byte-string and character
  literal replaced by spaces. It is what finds `//`, what finds `mod tests {`, and what finds the
  `}` that closes it.
- a **display**, the source unchanged. It is what the corpus emits, so a change *inside* a literal
  is visible.

Blanking the literals makes the proof blind; preserving them without a separate skeleton makes it
leak. **Both halves are load-bearing and the control proves each separately** — which is the one
thing four hand-rolled versions never established about themselves.

Two details are worth naming because they are where the earlier versions actually failed. Literal
state is carried **across line boundaries**, so a multi-line string containing `//` is not
truncated at it — there is no per-line reset. And a `'` opens a character literal only if it
closes; otherwise it is a lifetime, which is what once opened six phantom string literals from the
byte literal `b'"'` in `c14n.rs`.

### The module rule is the repository's own, not a second opinion

`ci/forbidden-tokens.sh` line 82 is `/^mod tests \{/` — anchored at column 0, no `pub(crate)`
alternative — and its skip resumes at the matching `^}` rather than running to end of file. The
script matches that rule exactly and its header says so, naming
`crates/engine-core/src/markdown.rs` as the **one** file in the tree that writes
`pub(crate) mod tests {` against fifty-two that write `mod tests {`. A second definition of "test
module" in one repository is precisely the drift a shared rule exists to prevent, and the choice is
worth 1,017 lines — all of them in that single file.

The direction-of-safety argument points the same way and survives if the guard ever changes:
including a test module errs toward **reporting**, and `forbidden-tokens.sh` makes that argument
for its own exclusions in as many words — *"a false alarm is cheap next to a missed one."*

### Where it lives, and what it deliberately is not

**`ci/`, not `crates/`.** `ci/forbidden-tokens.sh` is the precedent: a scanner that reads
`crates/*/src` and is run by hand or by a named job, not a crate. `fixtures/*/make_fixtures.py` is
the precedent for Python in-tree, and Python is the right language for a literal-aware tokenizer —
an `awk` state machine over raw strings, byte literals and lifetimes is exactly the kind of thing
that has already been got wrong three times.

**It is not a new CI job.** v2-S12's budget decision stands and this slice does not open it. Adding
the script without wiring a job is the minimum that solves the problem: the next slice runs it by
hand, as every slice already does, but runs *the same one*.

### The control, and where it lives

**Seven axes, each observed failing under a deliberate break before it was trusted.** The first
four are the axes every earlier slice controlled and then threw away with the script; the last
three exist because *"neither rendering alone is sufficient"* has to be a checked claim rather than
a sentence.

| Break | Axes that went red |
| --- | --- |
| Emit the skeleton rather than the display — **v2-S14.1's defect** | the wire-string axis; the multi-line `//` axis |
| Match the module rule on the display rather than the skeleton — **v2-S15's defect** | the `mod tests`-in-a-string axis; the column-zero `}` axis; the test-exclusion axis; the multi-line `//` axis |
| Reset the literal state at every line boundary — the per-line lineage | the same four |
| Never skip `mod tests` | the test-exclusion axis; the column-zero `}` axis |
| Drop the `//` strip | the comment axis |
| Emit nothing — *a corpus that reads nothing passes* | the `pub const` axis; the wire-string axis; the `mod tests`-in-a-string axis; the multi-line `//` axis |

**The control runs on every invocation, not behind a flag**, and that is the answer to *"where does
a shell script's test live"*. The two options were a `#[test]` in `crates/engine-cli/tests/` that
shells out, and a self-test mode run by hand. **Neither was taken as offered.** A `cargo test`
home would make the control run at a *different time* from the instrument — it would tell you the
extractor was sound at some point, not that it was sound when the proof was taken — and it would
make `python3` a hard dependency of a Rust suite that has never needed one. A hand-run self-test
mode runs only when someone remembers, which is the objection the standing rule *prefer a guard
that runs to a guard that exists* is aimed at. Running the control **as part of every measurement**
answers both: the guard runs, and it runs on the run whose soundness the proof depends on.

**The cost is stated rather than hidden.** The gate suite does not report a break in this file.
The next person to take a measurement does, before they get a number.

### The precondition nobody had asserted — measured, and one of the two is false

S15's record names two preconditions of *"proven mechanically"* and fixes neither, and both were
measured here rather than assumed.

| Precondition | Measured | Verdict |
| --- | --- | --- |
| No multi-line string literal outside a test module contains `//` | **0** | true today |
| No string literal outside `mod tests` contains a brace | **527** | **false** — every `format!` placeholder is one |

**The second does not survive, and 527 is not a near miss.** It was never true; what was true is
the narrower property the *anchored* rule actually needs — that no line beginning at column zero
with `}` or `mod tests {` lies inside a literal — and that measures **0** today.

**Neither is asserted, and the reason is the same for both: this instrument does not depend on
either.** The skeleton blanks braces inside literals and the pass carries literal state across
lines, so both preconditions are **discharged by construction**. Asserting a property the proof
does not rest on would forbid legal Rust and fail on innocent code, and asserting one that is
already false 527 times over would be worse than saying nothing. Two of the seven control axes
exist to prove the independence instead — *prefer a failing test that reproduces the gap to a fix
that asserts it is gone*, applied to a precondition.

**The one precondition the instrument does have, it checks.** A `/* */` block comment in code
context is refused by name, exactly as `ci/forbidden-tokens.sh` refuses it and for the same reason:
`//`-stripping is only safe while no block comment can hide a line inside one.

### Found beyond the brief, reported and not repaired

Running the lint half of the gate turned up two things that are **red at `HEAD`** and were red
before this slice touched anything. Both are named here and left, because repairing either is an
edit this slice's own scope forbids.

| Where | What | Why it is left |
| --- | --- | --- |
| `crates/engine-office/src/zip.rs` (a doubled blank line before `fn inflate`, since v2-S14) and `crates/engine-pdf/src/extract.rs` (`fn a_table_beside_a_column(\n)`, since v2-S14.1) | `cargo fmt --all --check` fails on the pinned 1.88.0 toolchain | Both are `crates/*/src` edits, which is **out of PR 1** by its own acceptance |
| `crates/engine-cli/tests/v0_exit_criteria.rs`, `crates/engine-office/tests/erasure_counters.rs:241`, `crates/engine-office/tests/robustness.rs` | three `clippy` warnings, which CI's `-D warnings` would fail | All three are in `crates/*/tests/`, and **a sweep of that directory is out of scope** for v2-S17 as well as this slice |

**No claim is made about CI**, red or green: this repository has no remote and its workflow has
never run. The measurement is local, on the toolchain `rust-toolchain.toml` pins, and it says only
that two commands in `ci.yml` do not currently succeed on this tree.

**"Full gate suite green" continues to mean the test suite**, which is what every slice since
v2-S9.1 has meant by it and what this one measured: 49 suites, 1,226 passed, 0 failed. Saying which
is the point.

### Reproduction, which is the acceptance

| Commit | Version | Corpus | Recorded |
| --- | --- | --- | --- |
| `524ea69` | 0.32.5 | **19,249** | 19,249 at v2-S13.1 and v2-S13.5 |
| `2363225` | 0.33.0 | **19,281** | 19,281 |
| `7591f02` | 0.33.1 | **19,281** | 19,281 |
| `438223d` | 0.34.0 | **19,281** | 19,281 |

And the diffs, which are what the numbers are for: **v2-S14.1 is 0 changed lines**, and **v2-S15 is
4** — the two wire strings, one line on each side, matching the excerpt S15 recorded verbatim.
`524ea69` → `2363225` is **44** rather than the 48 that section published, corrected in place
there.

**And it reproduces the other lineage's figures too, which is what closes the 1,017.** Swapping
only the module matcher for the loose `^(pub\(crate\) )?mod tests \{` and changing nothing else
gives **18,232** at `524ea69` and **18,264** at `0.33.0`, `0.33.1` and `0.34.0` — the four figures
S15's correction recorded for that variant. The gap is **1,017 at both commits**, and diffing the
two outputs rather than their counts puts every one of those lines in
`crates/engine-core/src/markdown.rs` and in no other file. One instrument, both matchers, and the
disagreement is one file and one keyword exactly as S15 said.

**The absolute count is still not the measurement.** It is an implementation artifact, and the
script's own header says so. What this slice buys is not a better number; it is that there is now
**one** instrument to disagree with rather than four to reconcile.

- **Acceptance — all met:**
  - [x] The extractor committed as `ci/code-lines.py`, with the skeleton/display split and a
        header comment arguing why neither rendering alone is sufficient
  - [x] `mod tests` detection matches `ci/forbidden-tokens.sh`'s anchored rule, and says so in the
        file, naming `markdown.rs` as the one file that differs and the 1,017 lines at stake
  - [x] Reproduces **19,249** at `524ea69` and **19,281** at `HEAD`, on the first measurement, and
        every intermediate figure and both recorded diffs besides
  - [x] Negative-controlled on **seven** axes — the four required and three more that make the
        design's own claim checkable — **each observed to fail** under a deliberate break and then
        restored. The control lives in the tree and runs on every invocation
  - [x] Both named preconditions measured: **0** and **527**. Neither asserted, with the reason
        argued rather than assumed; the one the instrument does have is checked and refused
  - [x] **No behaviour change** — **19,281 lines at `HEAD`, diff empty**, proven with the
        committed instrument. The one `crates/*/src` edit is inside `mod tests`
  - [x] Workspace **0.34.1**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both projection schemas regenerated and verified **equal to freshly generated
        artifacts**, all three identity fields
  - [x] Full gate suite green. Both SDK suites by hand. No git tag

- **Depends on:** S15.

---

## S17 — the two guards outside `src` — **done**, as 0.34.2

**A patch release.** No behaviour changed, and **no line of `crates/*/src` changed at all**: the
only Rust edits are in `crates/engine-cli/tests/`, plus `profile.rs`'s pinned digest and version
ledger, both inside `mod tests`. Proven with **v2-S16's committed extractor** rather than a
hand-rolled one — **19,281 lines at `HEAD`, diff empty** — which is the first time that sentence
has been written about an instrument anybody else can run.

**Nothing here reopens v2.** Both sites were named at v2-S13.5, restated at S14.1, and deferred
twice because both sweeps declared `crates/*/tests/` out of scope.

### The interesting one: a guard passing while covering four fifths of its subject

`diagnostics.rs`'s `no_diagnostics_field_name_appears_in_any_artifact` ended with
`assert_eq!(checked, 4, "all four subcommands must be covered")`. It was wrong on both halves.

| The assertion said | The tree says |
| --- | --- |
| four | **`Stage` has five variants** — `Classify`, `Extract`, `Ground`, `GroundingCheck`, `Verify` |
| subcommands | **a stage is not a subcommand.** There are **nine** subcommands, and `Stage`'s own doc comment records that they map onto five stages: `markdown`, `html` and `overlay` report under the phase they project from, and `mcp` has no stage at all |

**A floor set one below its own population is the shape v2-S13.1 exists for** — *a guard that reads
the wrong thing also passes* — and this one announced the gap in its own message while passing.

### Three options, one taken, two refused by name

**Taken — derive the floor from `Stage`, and assert which stages were walked rather than how
many.** The expected list is built from the enum and filtered through an **exhaustive match**, so a
sixth `Stage` variant stops the file compiling until somebody decides which side of the line it
falls on. That is the part a hardcoded `4` could never do, and it is the whole content of *prefer a
derived floor to a hardcoded one*. Asserting the stage **names** rather than a count is a second
gain that came free: a break now says which stage went missing instead of `3 != 4`.

**Refused — cover `Stage::Verify`.** Not on cost, which was the reason S13.5 deferred it, but on
substance: **there is nothing engine-owned to walk.** `engine verify` writes the verifier's bytes
verbatim and composes nothing of its own, so a key walk over that stdout asserts a property of the
**pinned Ethos binary** and reports it as evidence about this engine. Worse, it would be a weaker
assertion than the one already made:
`verify_relay.rs::a_grounded_claim_relays_the_verifier_bytes_verbatim` asserts stdout is
**byte-identical** to `ethos verify`'s, and a byte-identical relay cannot have injected anything at
all. The property is covered; it is covered elsewhere; and it is covered more strongly.

**Refused — cover it behind the oracle's skip.** Refused twice by this repository's own words.
`oracle.rs`'s header: *"Nothing here skips. An absent oracle binary, a missing corpus, or a hash
mismatch is a failure, never a quiet pass."* `verify_relay.rs`'s: *"Absence is loud."* And standing
rule 5: *a gap is never presented as a success*. A skipped test that reads as green is the defect,
not the remedy.

**Which rule wins, and why.** *Prefer a derived floor to a hardcoded one* wins over *a gap is never
presented as a success* — but only because the second rule is not actually engaged. It forbids
**presenting a gap as a success**, and once the property is asserted where it genuinely lives there
is no gap to present, only a boundary. The repair states that boundary in the file, with the test
that carries the property named, rather than leaving it implied by a number. Had `Stage::Verify`
been genuinely unasserted anywhere, this call would have gone the other way and taken the process
dependency.

**So no new dependency was taken.** `diagnostics.rs` still spawns only `engine`, and
`oracle.rs`'s resolver did not need to be reused. It was read first, because the option that would
have used it had to be refused on its merits rather than on the inconvenience of the thing it
needed.

### The cluster moved as one, on S15's precedent

Repairing only the assertion would have left `all_four_stages` and `Case`'s doc comment asserting
the number the assertion had just stopped claiming — a **new** inconsistency inside one file, which
is exactly the reason v2-S15 moved the `neither detector` cluster whole rather than repairing its
comments. `all_four_stages` is now `covered_stages`, and `Case`'s comment says a stage is not a
subcommand and why the counts differ.

### The prose one, where the handed repair was wrong

`verify_relay.rs` said *"**Reachable through the library**, like the other four subcommands."* The
brief carried the repair as *nine subcommands, so eight others*. **That repair is a new false
statement**, and checking it rather than applying it is the finding.

1. **`verify` is not among the four `docs/PUBLIC-API.md` maps.** That table has four rows —
   `classify`, `extract`, `ground`, `grounding-check` — and the document says in as many words that
   `verify`, `overlay`, `markdown`, `html` and `mcp` *"have their thin-shell mapping stated
   nowhere"*. So the sentence could not have been naming those four either; it is wrong under the
   charitable reading as well as the plain one.
2. **Eight would assert something about `mcp` that this repository deliberately has not settled.**
   `mcp` is a server loop rather than a document pass — `Stage`'s doc comment says so, and it is
   the one subcommand with no stage — `engine-cli` publishes no library target, and
   `PUBLIC-API.md` defers its mapping in the same sentence it defers `verify`'s. A count asserted
   in a test comment would answer that by accident.

**The replacement carries no ordinal**, on the rule v2-S15 settled: it states the thin-shell
requirement, names this test as the library-only proof the missing row would cite, and cannot be
re-rotted by a tenth subcommand.

**The pattern the handoff names held again: a handed finding disagreed with the code, and the code
won.** Stated without an ordinal, on the same rule as the wording v2-S15 chose — a running count is
the one thing in this record that has never survived a slice.

### One found beyond the two sites, reported and left

`crates/engine-cli/Cargo.toml`'s package `description` reads *"ethos-engine command line: classify
| extract | ground | grounding-check"* — v0's four, of nine. It was found while re-resolving the
first site by symbol. **A slice that widens its own scope is how the last four each ended up
rebuilding an extractor**, so it is named here and left for whoever owns the next sweep.

The two lint gates v2-S16 reported red at `HEAD` are still red and still out of scope: two
`cargo fmt` sites under `crates/*/src`, and three `clippy` warnings under `crates/*/tests/` — the
directory this slice is explicitly forbidden to sweep.

### Each repaired guard watched failing

| Break | What the guard said |
| --- | --- |
| Drop the `ground` case from `covered_stages` | `["classify", "extract", "grounding-check"]` against the derived `["classify", "extract", "ground", "grounding-check"]` — it **names the missing stage**, where `assert_eq!(checked, 4)` could only have said `3 != 4` |
| Flip `Stage::Verify` to walked in the match | `verify` appears in the expected list and not in the walked one, so claiming coverage this file does not have fails immediately |
| Add a sixth `Stage` variant in `engine-core` | `error[E0004]: non-exhaustive patterns: Stage::Probe not covered`, at `diagnostics.rs` — the derived half refusing to compile, which is the thing a hardcoded number could not do |

All three were reverted, and `engine-core/src/diagnostics.rs` is byte-identical to `HEAD`.

- **Acceptance — all met:**
  - [x] Both sites re-resolved **by symbol** — `the_relay_is_reachable_from_the_library` and
        `no_diagnostics_field_name_appears_in_any_artifact` — and repaired
  - [x] The `diagnostics.rs` floor **derived** from `Stage` through an exhaustive match, asserting
        which stages were walked rather than how many; the other two options named with the reason
        each was refused
  - [x] `Stage::Verify` is **not** covered, so `oracle.rs`'s resolver was **not** needed and **no
        new process dependency** was taken. The reason is substantive, not economic, and the test
        that does carry the property is named in the file
  - [x] Each repaired guard **observed to fail** under a deliberate break — three of them — and
        restored
  - [x] **No behaviour change**, proven with **v2-S16's committed extractor**: 19,281 lines at
        `HEAD`, diff empty. No `crates/*/src` line changed at all
  - [x] Workspace **0.34.2**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; both projection schemas regenerated and verified **equal to freshly generated
        artifacts**
  - [x] One third site found and **left**, named above. Full gate suite green. Both SDK suites by
        hand. No git tag

- **Depends on:** S16.

---

## S18 — the gate that has never been green — **done**, as 0.34.3

**A patch release.** No behaviour changed. Every edit is a formatting reflow, a lint, a metadata
string, or new test-and-script material; the one production file `cargo fmt` touched
(`engine-office/src/zip.rs`) lost a duplicate blank line and nothing else. Proven with v2-S16's
committed extractor: **19,281 lines at `HEAD` and 19,281 after, diff empty.**

### The defect, and why four slices did not see it

`cargo fmt --all --check` **exited non-zero at `HEAD` and had since v2-S14.** Four sites:

| Site | Shape | Introduced |
| --- | --- | --- |
| `engine-office/src/zip.rs:302` | a doubled blank line between two items | v2-S14 |
| `engine-pdf/src/extract.rs:1295` | `fn a_table_beside_a_column(\n)` — a signature broken across lines that fits on one | v2-S14.1 |
| `engine-pdf/src/extract.rs:1306` | a `const` table's trailing comments over-indented by one column | v2-S14.1 |
| `engine-pdf/src/extract.rs:1338` | an `assert_eq!` on one line that rustfmt writes as four | v2-S14.1 |

Three of the four are inside `mod tests`, which is why `ci/code-lines.py` reports no change: it
counts non-comment lines outside `mod tests` in `crates/*/src`, and blank lines are not counted at
all. **That is the proof of no behaviour change, and it is also the reason the defect was cheap to
miss** — nothing a slice normally runs looks at those lines.

**The `check` job runs `cargo fmt --check` as its first gate**, so the first CI run this repository
ever performs would have failed on formatting. That is the worst available first signal: it teaches
whoever wires up the remote that the gate is noise rather than that the gate is right.

### The finding underneath the defect: no "green" in this repository has ever been a fact

`.github/workflows/ci.yml` **has never run.** No remote, no tag, 85 commits. Every green any record
claims was a local partial run — somebody's `cargo test`, with whichever checks they remembered —
and `fmt` was one nobody remembered for four consecutive slices that each recorded a green.

Repairing the four sites fixes today and not tomorrow. So the slice also ships **one command that
runs everything CI runs**:

```
ci/gate.sh    # both token greps, then fmt, clippy, build, test, deny
```

### The script was the easy half; the guard is the point

`ci/forbidden-tokens.sh` set the precedent — *"a script rather than inline YAML so the gate can be
run locally, exactly as CI runs it, and so the two cannot drift."* That argument scales to the whole
gate, but so does its risk, and at this size the risk is larger than the precedent's: **a
convenience script nobody verified against CI is worse than no script**, because a local green then
means something the remote does not enforce. It manufactures confidence instead of measuring it —
which is precisely the failure that produced this slice.

So `ci/gate.sh` is **not the authority.** `ci.yml` is, and
`v0_exit_criteria.rs::the_local_gate_runs_what_ci_runs` asserts **set equality in both directions**:

- **CI → script.** Every `run:` command in the `check` job is in the script, unless its step name is
  in `GATE_SKIPS`.
- **script → CI.** Every command the script runs is a command CI runs — without this the script
  quietly becomes a second source of truth.

The two grep criteria are read **out of the workflow** rather than written into the test, so
deleting those jobs fails here too. Commands are compared **character for character**: CI is the
authority, and a tidied flag ordering is a different command.

**No new parser.** `matrix_ids_of` already scoped a job structurally; that scoping is now
`job_block`, and `run_steps_of` is built on it. A `run: |` block scalar is consumed by indentation
rather than scanned, so a shell line inside one cannot be read as YAML — the same class of defect as
a test-region skip that runs to end of file.

**The exclusions are asserted complete, which is what stops the guard going hollow.** A sixth step
added to `check` is in neither the script nor `GATE_SKIPS` and fails; a `GATE_SKIPS` entry whose CI
step was renamed away fails, because a skip that excuses nothing while looking like it excuses
something is the v2-S13.1 shape again. Four are excluded and each is argued in the script's header:
the oracle build (it would write into a tree this repository does not own), the toolchain tripwire
(no second toolchain to disagree locally), `v0-fuzz-smoke` (nightly), and `deny-policy-is-enforced`
(it `git checkout --`s a file, which discards uncommitted work — and a gate is run *while working*).
Every other matrix entry is a labelled re-run of a subset of `cargo test --workspace`, which is
step 6.

**The guard was watched failing six ways** and restored each time:

| Break | What it said |
| --- | --- |
| `cargo deny check` dropped from the script | `only in ci.yml: ["cargo deny check"]` |
| a command added that CI does not run | `only in ci/gate.sh: ["cargo test --workspace --all-features"]` |
| clippy's flags reordered in the script | both spellings listed, one on each side |
| a sixth step added to `check` | `only in ci.yml: ["cargo audit"]` |
| a `check` step renamed out from under `GATE_SKIPS` | names the stale skip and says to remove it |
| the `v0-no-confidence` grep removed from CI | names the grep and says the script must lose it too |

### The three lints, and the third that was hiding behind the first two

v2-S17 reported three clippy warnings in `crates/*/tests/` and was forbidden to sweep there. Two are
in `engine-office`: a `&file` that is already a `&str`, and an `is_none()`/`return None` pair that is
`?`. **The third was invisible until those two were fixed** — `-D warnings` aborts compilation, so
`engine-cli`'s `!path.extension().is_some_and(|e| e == "rs")` never got linted while `engine-office`
failed first. It is now `is_none_or`. The count S17 gave was right; the ordering is worth recording,
because "clippy is clean" measured behind a failing crate is not a measurement.

### The third site of a list that one enum owns

`engine-cli`'s package description said `classify | extract | ground | grounding-check` — the v0
four, when there are nine. v2-S13.5 repaired this drift in `main.rs` and
`engine-core/src/verifier.rs` and missed this one.

**It now names none.** Those two sites are prose that can carry a count with its history; a one-line
metadata string cannot argue, nothing guards it, and this is its third staleness with a tenth
subcommand plausible at v2.2. `Command` in `src/main.rs` is the list that cannot go stale, so the
description says what the crate *is* — `one thin subcommand per library entry point` — rather than
what it currently contains. The alternative, writing nine names in and adding a guard to pin them,
buys a guard for a string nobody reads and leaves the duplication in place.

### Scope refused

**No remote, no tag, no push.** PR 1's job is to make those safe rather than to perform them; they
are the owner's. **No new CI job** — the workflow is unchanged, which is the point: the script is
measured against it, so changing both at once would have measured nothing. **The SDK suites stay
hand-run**, v1.2's decision, unchanged.

- **Acceptance — all met:**
  - [x] `cargo fmt --all --check` **exits 0** — the first time that is true at a commit in this
        repository's history
  - [x] `cargo clippy --workspace --all-targets -- -D warnings` exits 0; all three
        `crates/*/tests/` warnings repaired, including the one hidden behind the other two
  - [x] `engine-cli`'s package description names **none** of the nine, with the reason recorded
        beside it
  - [x] `ci/gate.sh` ships **with a guard** — `the_local_gate_runs_what_ci_runs` asserts set
        equality against `ci.yml` in both directions and asserts the exclusion list complete.
        **Observed failing six ways** and restored
  - [x] **No behaviour change**, proven with `ci/code-lines.py`: **19,281 lines at `HEAD` and
        19,281 after, diff empty.** `cargo fmt` touched one shipping file — `zip.rs` lost a
        duplicate blank line — and the extractor counts neither blank lines nor `mod tests`, so
        the empty diff is a real proof and not a blind one. Every changed line shown to be a
        reflow, a lint, or a string
  - [x] Workspace **0.34.3**; nine profile hashes move on `parser_version` alone and stay mutually
        distinct; all three draft schemas regenerated by the documented commands and verified
        **equal to freshly generated artifacts** — no digest hand-edited
  - [x] **`ci/gate.sh` run end to end on the committed tree: all 7 checks, exit 0, 1,227
        tests** — 1,226 plus the guard this slice adds. SDKs by hand: **72 Node, 91
        Python**. **No git tag**

- **Depends on:** S16 (the extractor this proves no-behaviour-change with), S17 (which found two of
  the three lints and the description site).

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
