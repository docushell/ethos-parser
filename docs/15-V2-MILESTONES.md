# 15 — v2 slices

**Status:** implementation authority for v2 · **Scope document:** `14-V2-SCOPE.md`
**This is the code-review map for v2.** Every v2 PR belongs to exactly one slice.

**v2 reads its first format.** S0–S2 are **done**; **S3–S4 are not started**. `engine-office` is the
fifth crate and DOCX is the format that stopped it being speculative.

**v1 is not done.** S7's gate is measured and **missed at 64‰** against a 489‰ floor
(`09-V1-MILESTONES.md` S7, `table-gate-v1.md`). **v1.1 is complete** at 0.14.1 and **v1.2 is
complete** at 0.19.0. v2 began because the owner asked for the next roadmap row, and nothing in it
closes v1.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v2 scope + this document | — | **done** |
| **S1** | The grounding contract for a page-less source | S0 | **done — (b), and one finding** |
| **S2** | DOCX → representation — **and the page-parent invariant S1 found** | S1 | **done** |
| **S3** | XLSX → representation: sheets and cells | S2 | **not started** |
| **S4** | The remaining office formats — PPTX, ODF, RTF, EPUB, CSV | S3 | **not started** |

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

## S3 — XLSX → representation — **not started**

- **Goal:** the second half of the v2 gate — an XLSX **cell** binds.

- **The standing constraint:** the same three obligations, and one that is specific to spreadsheets:
  a cell's address is `(sheet, row, column)` **as the file states it**, and a rendered column width
  or a print range is not part of it. A spreadsheet's "page" is a print artefact and is exactly the
  thing §3 forbids.

- **What S2 handed this slice:** `Profile::docx_v0` carries `coordinate_system:
  {centipoint, top-left}` because the field is required and there is no page-less spelling of it.
  Nothing under that profile ever emits a coordinate — `measured_ink_boxes` is false and every
  geometry row is absent — so the declaration is inert rather than wrong, but it is a field saying
  something it cannot mean. Giving it an honest spelling is a mode enum on a field every existing
  artifact carries, which moves every PDF hash; **S3 is where that is worth deciding**, because
  XLSX gives the question a second format's worth of evidence rather than one.

- **Out:** formulas as anything but text, charts, pivot caches, and every format S4 parks.

---

## S4 — the remaining office formats — **not started**

- **Goal:** PPTX, ODF, RTF, EPUB and CSV, on the terms the first two established.

- **Deliberately one row.** **A1** — Anydoc's 14-format coverage — is v2's horizon, not its
  checklist. Scheduling five formats before two have shipped would be a waterfall built on a guess
  about what the second one costs. This row splits into real slices when S2 and S3 are done and that
  cost is **measured**.

- **The one thing already known about this row:** EPUB may genuinely have pages and CSV genuinely
  has none, so §3's law is not "no page ever" but "no page this engine did not read from the file."
  Whichever formats have a native pagination declare it; the rest carry the empty vector.

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
