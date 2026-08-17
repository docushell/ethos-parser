# Changelog

All notable changes to ethos-engine. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Entries through M7 are grouped by **milestone** (`docs/05-MILESTONES.md`) rather than by version
number, because a milestone was the unit of work that had acceptance criteria. M7 ends that: v0 is
frozen at **0.1.0** and later entries are versions.

## [Unreleased] — v2 reads a third format, as 0.23.0

### v2-S4 — PPTX into the representation, and the format that tested the page law

**A slide's text binds, and a slide is a part.** `engine extract` reads a `.pptx` into the same
`ethos.engine.representation.v0` the other three formats produce.

#### The temptation this slice exists to have refused

DOCX has no page until a renderer decides where one falls; a spreadsheet's page is a printer's.
**A slide is neither** — it is discrete, addressable, listed in the package, and people count them
out loud. This is the first v2 format where inventing a page would not have felt like inventing
one, which is exactly why both routes to it are refused by name: `p:sldSz` is a size the authoring
tool wrote and this engine measured nothing against, and a position in `<p:sldIdLst>` is display
order that changes when a deck is reordered. `pages` is `[]`, each slide is one part id, and
`PptxLocator` carries **no slide number at all**. A test asserts the locator's exact field set, so
one cannot arrive later as a fifth field.

#### Two measurements changed the design before it shipped

- **Shape ids are not unique.** `<p:cNvPr id="7">` is a number the file wrote, so the locator
  addressed by it first. Across 18 real decks — 329 slides, 3,335 shapes — the id is present every
  time and unique only most of the time: 12 slides from an Open XML SDK generator reuse one, and
  PowerPoint opens them. Addressing by it would have given one address two answers on real files;
  refusing those files would have rejected decks that open everywhere else. The address is a
  **position**; the id is carried on the attributes, where a non-unique label is what it is.
- **`<mc:AlternateContent>` states one phrase several ways.** It appeared on 162 of 329 slides,
  carrying one or more `<mc:Choice>` and an optional `<mc:Fallback>` for consumers of different
  capability. A descendant walk emits that phrase at two or three addresses — the mirror of a
  silent drop. Exactly one branch is read: the first `Choice`. The rest are counted when they held
  text.

#### What is read, and what is counted

Of 5,297 `<a:t>` elements measured: 4,670 in top-level shapes and 160 in nested `<p:grpSp>`
groups are **read** (91.2%), while 287 in `<p:graphicFrame>` tables and charts and 180 in
`<a:fld>` fields are **counted** (**A14**), along with notes, masters, layouts, comments and
diagram parts. A reader that stopped at top-level shapes would return 88.2% of a deck and say
nothing about the rest; groups appeared on essentially every slide, so that is the ordinary case
rather than an edge one.

The field is the interesting refusal: an `<a:fld type="slidenum">` holds a **cached** slide number
written at save time that goes stale the moment the deck is reordered. Reading it would put a
number in the evidence that is not on the screen and is shaped exactly like the page index this
version refuses.

#### The rule that moved rather than being copied a third time

`xl/workbook.xml` and `ppt/presentation.xml` both list things and name no parts; both invite the
`sheet{n}` / `slide{n}` shortcut; both break under it. That rule now lives in **`opc.rs`**, and
`xml.rs` grew from the entity rule to the whole XML-reader plumbing — because the alternative was
a third copy, and three of the nine defects v2-S3's review found were two copies of one rule
disagreeing. Nothing in the moved code changed, and the DOCX and XLSX suites passed unaltered
across the move.

`resolve_target` gained `.`/`..` normalisation: 2,318 of the measured relationship targets climb a
directory, and a literal join would refuse packages that open everywhere else. A target that
climbs out of the package resolves to a name no central directory contains.

One shipped message changed with the move: the truncation refusal said *"a shorter **sheet** that
still looked whole"* because only the workbook reader used it, and now says *"a shorter **part**"*.
No test pinned the word, and the DOCX reader has its own copy of that check, so this affects
XLSX and PPTX refusal text only. Recorded because a message a caller reads is output.

#### Found by reviewing the slice before it shipped

Three defects, none of which lost text — each made a locator disagree with what a consumer
counting elements in the file would find, which is the "right address, wrong thing" class v2-S3's
review named.

- **A `<p:sp>` inside a skipped `<mc:AlternateContent>` branch did not advance the shape count**,
  so every shape after it was numbered one short and a verifier following the documented rule
  landed on the skipped shape. The counters now advance **through** skipped branches, because
  `shape` promises a position in the part's own document order — not among the shapes this reader
  chose to keep.
- **A self-closing `<a:p/>` was invisible to the paragraph count.** `<a:p/>` and `<a:p></a:p>` are
  the same infoset, so the address turned on how the deck was serialized — and python-pptx and
  Apache POI write the first for a blank line.
- **Every `<mc:Choice>` was read rather than the first**, so one displayed phrase became two or
  three citable nodes. The code's own comment claimed to prevent exactly this and guarded only the
  `Fallback` half of it.

Each fix is mutation-checked: deleting it fails a test.

#### Changed

- Workspace and both SDKs to **0.23.0**. The PDF default profile hash moves to
  `sha256:8dfca0e41d51668c6d540ec45bf83dd66503dbc2dcb1fa7c188cceebe255d4bd` on `parser_version`
  **alone**, as at S2 and S3. All four profile hashes are now mutually distinct.
- `engine_office::read` counts the main parts a package lists instead of asking three ordered
  questions, so a package claiming two formats is a named refusal rather than whichever `if` ran
  first. `is_pptx` joins `is_docx` and `is_xlsx`; none consults a file name (**A4**).
- **`docs/15-V2-MILESTONES.md`'s remaining-formats row split**, which is what S2 and S3 were
  measured for. S4 is PPTX and is done; **S5** parks ODF, RTF, EPUB and CSV, which share no
  container with each other and would have been scheduled on a guess.
- `coordinate_system` is untouched: v2-S3 decided it stays inert on page-less profiles, and a
  third such profile did not reopen it.

#### Not done

v2's gate names a DOCX quote and an XLSX cell, and both bind. PPTX is coverage beyond the gate,
and S5 is not required for it. v1 is still **missed at 64‰**.

## [Unreleased] — v2 reads a second format, as 0.22.0

### v2-S3 — XLSX into the representation, and the `coordinate_system` question closed

**A cell binds.** `engine extract` reads an `.xlsx` into the same
`ethos.engine.representation.v0` a PDF and a DOCX produce — and nothing on that path is a page, a
column width or a print range.

#### The locator, and the two spellings the file uses

`XlsxLocator { part, sheet, row, column }`, `deny_unknown_fields`, no page and no box.

**Both `part` and `sheet`**, because they answer different questions: `part` is the package's name
for the worksheet and is what the part-id ↔ part-name bijection is checked against; `sheet` is the
workbook's name for it, which is what a person citing a cell writes down and which the part name
does not contain.

**`row: u32` and `column: String`, and the asymmetry is the file's.** `<c r="B12">` writes the
column as letters and the row as digits. Reading `12` as a number is reading. Turning `B` into `2`
is arithmetic on a bijective base-26 numeral — a value the workbook never contains — so the letters
are carried through. The two concatenate back to the `r` attribute exactly.

#### `xl/_rels/workbook.xml.rels` is read, not guessed around

`xl/workbook.xml` contains **no part names**. A `<sheet>` carries a name, a `sheetId` and an
`r:id`, and only the relationship part says which package part that `r:id` means. The shortcut —
`xl/worksheets/sheet{n}.xml` in `<sheets>` order — is wrong in ordinary files: reordering sheets in
Excel leaves part names alone, deleting one leaves a gap, and part names are author-chosen. Each of
those attaches the **wrong sheet name to the right cells**, which is worse than no address at all.
The fixture's second sheet is `sheet3.xml` behind `rId7` so the shortcut cannot pass silently.

#### The first artifact with more than one part — and `engine-core` did not change

A workbook is one part per sheet. v2-S2's page-less rules already allowed that and nobody had
exercised it: the part-id ↔ part-name check is a **bijection**, not a cardinality-of-one rule, and
`check_structure` counts ordinals **per parent**, so each sheet gets its own contiguous 1-based
sequence. S3 added no invariant; it is the first slice to use the shape S2 built.

#### A cell reuses `NodeKind::TextRun` and gets its own attributes

The standing rule is *do not add a kind for a fact an existing node already carries*. `XlsxLocator`
already says sheet, row and column, which is cell-ness spelled out, and the locator union is
externally tagged — so a second kind would restate the address and make every consumer handle two
names for one concept. `NodeAttributes::OfficeCell` **is** new, because that is where the homeless
facts are: `value_type` (what `t` declares the stored value to be) and `text_source` (stored value,
cached formula result, or formula source because nothing was cached).

**`<f>` is not a second authority.** No evaluator, no `computed_value`. A cell's text is what the
workbook stored, and `text_source` is what stops `SUM(B2:B2)` being read as a number the sheet
displayed. **`capabilities.tables` stays false** on a format made of grids: `tables` means this
engine's `TableRecord` IR, and this slice emits cells.

#### The `coordinate_system` decision — measured, and left inert

v2-S2 left `coordinate_system` declaring `centipoint`/`top-left` on a page-less profile and asked
S3 to settle it with a second format's evidence. **Nothing acts on the value for a page-less
artifact**: `engine_grounding::project` hard-codes the pair rather than copying it and refuses a
non-`application/pdf` source long before reaching it; the only branch on the value in the workspace
is inside the `ethos.grounding.v1` validator, downstream of that refusal; and both SDKs re-hash the
field as opaque bytes without parsing it.

So **no mode enum**, because one would have moved every PDF artifact's `profile_sha256` to respell
a value nothing reads. Both page-less profiles keep the inert declaration, and the property that
makes it honest is pinned instead — `measured_ink_boxes: false` and **zero** measured geometry rows
on a real workbook artifact. Recorded in `Profile::docx_v0`'s doc comment and the profile-hash
ledger's twenty-sixth entry.

#### Declared erasure, and every failure closed

Charts, drawings, comments, threaded comments and pivot caches are counted and declared under
`office-parts-not-read` — and so is a listed sheet that is **not a worksheet**, since returning a
chart sheet as an empty worksheet would be a silent drop (**A14**).

Named refusals, each of them: a `<c>` with no `r` attribute (an implied address is one this engine
would have *counted*), an `r` that is not an A1 reference, a cell whose row disagrees with its
`<row>`, a `t` outside `ST_CellType`, a shared-string index that does not exist or is not a number,
a `<sheet>` whose `r:id` matches no relationship, a workbook that lists no sheets, and a package
that claims to be **both** a document and a workbook.

#### Fixed

Six of these were found by reviewing this slice against the project's own laws before shipping it,
and every one of them is the "silent" failure mode those laws exist to name. Each has a test that
fails without the fix.

- **A self-closing `<si/>` took no slot in the shared string table**, so every index after it
  resolved to the *next* string — the right cell address carrying another cell's text, sealed,
  byte-identical and with no error. The worst kind of defect this engine can produce, and it
  arrived through the one door the code's own comment about that hazard did not cover: `<si/>` is
  an `Event::Empty`, and only `Start`/`End` were matched. Any XML round-trip writes `<si/>` for
  `<si></si>`, so no unusual writer was needed.
- **`<rPh>` furigana was concatenated into inline-string cells**, producing `漢字かんじ` for a cell
  showing `漢字` — characters the reader authored. The shared-string path already stripped it; the
  inline path is the same content type and did not.
- **A second `<v>` in one cell appended instead of being refused**, so `<v>1</v><v>2</v>` became
  the value `12` — and under `t="s"` resolved shared string **12**, an entry the workbook never
  pointed at.
- **A cell whose `t` named a child it did not have was silently emptied.** `<c t="s"><is><t>Total
  revenue</t></is></c>` returned "no node", indistinguishable from an empty cell, so a caller
  could conclude a phrase was absent from a workbook containing it. Now a named refusal — the file
  contradicts itself about where the value is, and this reader does not choose a half.
- **`split_reference` repaired malformed addresses.** `u32::from_str` accepts a leading `+` and
  leading zeros, so `r="A+1"` emitted the locator `A1` — a *different cell*, and a direct
  falsification of `XlsxLocator`'s documented promise that its halves concatenate back to `r`.
- **Errors reading the shared string table were swallowed** by an `Err(_) => Vec::new()`, so a
  part that exceeded the size cap or would not inflate re-surfaced later as "the table has 0
  entries" blaming a *worksheet* — the wrong part and the wrong cause. Only an **absent** table is
  now the empty table. The table's part is also resolved through the workbook's relationships,
  since its name is author-chosen exactly as a worksheet's is.
- **Two cells at one address** are refused: an artifact carrying both would leave a citation to
  that address with two answers.
- **A cell carrying both a `<v>` and an `<is>`** is refused. Those are two answers to where its
  value is, and reading whichever `t` named would have discarded the other in silence. Found while
  fixing the mismatch above.
- **A package with no text at all could not seal.** The `geometry-absent-not-groundable`
  limitation was pushed unconditionally, and `check_geometry_matches_its_declaration` requires it
  exactly when at least one node has no measurable box — so a zero-node artifact was refused with a
  message about nothing. Pathological for a `.docx`, ordinary for a workbook with an empty sheet.
  Both readers now push it only when there are nodes.
- **`NodeAttributes`' documented cross-check did not exist.** Since v0 the type has said the tag
  duplicating `Node.kind` is a *checked* redundancy; nothing checked it, so a record reading
  `"kind":"annotation"` with `office_cell` attributes sealed cleanly and read as two different
  things depending on which field a consumer trusted. `check_structure` now enforces it on both
  construction paths. Found while adding the third variant that relies on the claim.
- **`engine-office` was outside the public-API freeze.** It shipped at v2-S2 without a row in
  `docs/PUBLIC-API.md` or the gate's `FROZEN` table, which by that document's own rule made
  `engine_office::read` — the entry point for two of the three formats `engine extract` accepts —
  "internal", renameable without a note. Added, with its own section.

#### Measured in passing, recorded rather than acted on

- **`quick-xml` 0.41 delivers a numeric character reference as a `GeneralRef` event**, named
  `#66`. The five-entity rule therefore refuses `&#66;` — ordinary XML needing no DTD. It is a
  *named refusal of a valid document*, not a silent drop, so it fails safe; widening it would
  change what a shipped DOCX artifact contains, which nothing in this slice measured a need for.
- **`Event::CData` was unmatched in the DOCX reader**, which is a silent drop. The workbook reader
  matches it; the DOCX reader's arms are left unchanged for the same scope reason.

#### Changed

- Workspace and both SDKs to **0.22.0**. The PDF default profile hash moves to
  `sha256:26c10d2a73e41c357c82589b0acfabffd8b33f9f93e3c666452b8a437743b6fa` on `parser_version`
  **alone** — every other byte of that profile is identical, as at S2.
- `engine_office::read` is now a router. `is_xlsx` joins `is_docx`; both refuse to decide by
  extension (**A4**).
- The five-entity rule and the local-name helper moved to `engine-office/src/xml.rs`, unchanged and
  message-identical, so the two readers cannot drift on what counts as text.

### v2-S2 — DOCX into the representation, and the page-parent invariant S1 found

**The first office reader, and the IR change that had to come first.** `engine extract` reads a
`.docx` into the same `ethos.engine.representation.v0` a PDF produces — and **nothing on that path
is a page**.

#### The invariant, split rather than loosened

v2-S1 measured that `DocumentRepresentation::seal` refused *every* node whose parent was not a
declared page, so a page-less document could not enter the IR at all. S2 fixed that by splitting
`check_structure` on the **locator family**, which is the one thing a node cannot fake — it reaches
the page-less rules only by carrying an address with no page in it.

| | paginated address | page-less address |
| --- | --- | --- |
| parent | a declared `PageRecord` — **unchanged, message included** | a `Part` id (`d1`) |
| `pages` | as before | must be **empty**, or the seal refuses "invented pagination" |
| geometry | a measured box is checked against its page | a measured box is **refused**: no page to check it against |
| integrity | the parent is looked up in a declared list | part id ↔ part name is a **bijection** |

The last row is the interesting one. A PDF gets its integrity from a declared-page lookup; a DOCX
has no such list, and adding a payload field for one would be machinery for a format that describes
itself already. Every page-less node names its part in its own locator instead, and the seal checks
that one part id means one part name **in both directions**.

The measured-box row is what makes "no geometry" a property of the artifact rather than a habit of
the reader: `engine-office` could not emit a rectangle if a later edit tried.

#### The locator, and why the attributes are a new variant

`DocxLocator { part, paragraph, run }` — a package part name and two 1-based document-order
positions, all of them things the file states about itself. **No page, no bbox, no `x`/`y`**, and
`deny_unknown_fields` so one cannot be added quietly.

`NodeAttributes::OfficeRun` is new rather than `TextRun` with the PDF fields blanked: a `<w:r>` has
no char codes, no font resource name and no font size this reader read, and `font_size: 0` would be
three claims the document never made. Its one field is `space_preserved` — whether
`xml:space="preserve"` was set, which decides whether a run's spaces are the document's or the
parser's. `NodeKind::TextRun` is **reused**: a `<w:r>` and a show-text run are the same thing
addressed differently, and the locator is what says which.

`Profile::docx_v0` is a **separate profile with its own hash**, so a DOCX artifact and a PDF
artifact are provably non-comparable. Every capability it declares false is one v2-S2 genuinely does
not read, and `measured_ink_boxes: false` is the load-bearing one.

#### The fifth crate, and exactly one new dependency

`engine-office`, where `04-ARCHITECTURE.md` always said an office reader would live —
*"revisit only when office or OCR needs a real home"*. DOCX is that revisit.

**`quick-xml` is the only new crate in the lock.** ZIP is read in `engine-office/src/zip.rs` over
`flate2`, which the graph already carried via `lopdf`: the `zip` crate drags twelve transitives
including `zopfli`, a *compressor*, to save ~120 lines of central-directory reading. XML is the
opposite call and is **not** hand-rolled, because entities, namespaces, CDATA and encodings are
exactly where a hand-rolled reader silently gets *text* wrong — and text is the evidence. Both
halves are v1.2-S1's reasoning about an MCP framework, applied twice with opposite answers.

**Measured, not feared:** the first version of the reader dropped `&amp;` silently, because
`quick-xml` 0.41 delivers an entity as its own event and the reader only handled `Text`. The fixture
carries an ampersand because of it, and the reader now resolves the five XML predefined entities and
**refuses every other name** rather than letting one become an empty string in the evidence.

#### What is read, what is declared unread, and how failures close

`<w:p>` / `<w:r>` / `<w:t>` in `word/document.xml`, in the part's own order. Headers, footers,
footnotes, endnotes and comments are **counted and declared** as `office-parts-not-read` — Anydoc's
**A14** applied to a package, because a reader that silently returned the body would let a caller
conclude a phrase is absent from a document that contains it.

Detection is **content-based** (**A4**): a ZIP signature plus `word/document.xml` in the central
directory, so `report.bin` reads and a `.docx` full of something else does not. A truncated archive,
a Zip64 record, an unimplemented compression method, a size that disagrees with the directory, XML
that will not parse, and a part ending with elements still open are each a **named** refusal.

#### What did not change, which is the one-IR claim being paid

**`ethos.grounding.v1` is untouched** and `engine ground` on a DOCX artifact is a named refusal
naming `application/pdf` and pointing at the law — v2-S1's decision (b), now live rather than
hypothetical. `project()`'s PDF path is unchanged; relaxing its page requirement "for office" would
have changed PDF behaviour for a format that did not exist here yet.

**`mcp.rs` is untouched, and `node_get` resolves a DOCX run anyway.** So do both SDKs and the
LangChain tools, because there is one artifact type and one serializer. That is `14-V2-SCOPE.md`
§4's whole claim, and it cost nothing to keep.

### Identity, after v2-S2

Workspace **0.21.0**, profile hash
`sha256:1a7844f5a291cdd1430ecead0523980283a6c3e7bbf3f6f431ce5a056c5d92f1` — moved on
`parser_version` **alone**. Every other byte of the PDF profile is identical, because the second
format got a profile of its own rather than a capability on this one. `XrefRepair` gained a
`not-run-for-this-format` state for that profile to use; the PDF profile still says
`pad-19-to-20-v1`, so the new variant is invisible here. **A second format is a new value, not a
change to what the first profile claims.**

**v1 is still not done** — the S7 table-cell gate is measured and missed at 64‰ — and nothing in
this entry closes it.

### v2-S0/S1 — office formats scoped, and the grounding contract decided

### v2-S1 — `ethos.grounding.v1` stays PDF-only, and the page assumption is not where S0 thought

**A contract decision with no reader.** S0 posed the question and S1 answers it: **(b)**, grounding
stays PDF-only. No page-less source shape, no optional `page`, no forked schema. Revising the
artifact is **(a)**, a change to the **verifier's** contract — `engine-cli/tests/oracle.rs` agrees
with the pinned Ethos CLI on this exact schema, so an engine-only revision would produce artifacts
the verifier does not speak while both still called themselves `ethos.grounding.v1`. (a) is recorded
as **blocked on an Ethos-side revision**, owned rather than refused. Adding an optional `page` to
the engine's copy would be the lying artifact v1.2-S5 refused for loose boxes, wearing a different
field name.

**And the measurement resized S2.** S0 wrote that deciding (b) would leave v2's gate reachable *"at
the representation level"*. **That is false.** `DocumentRepresentation::seal` refuses a node whose
parent is not a declared page — in `check_structure`, which runs on **both** construction paths, so
it cannot be reached around by deserializing instead of sealing:

> node `s1` names parent page `p1`, which is not a declared page

**A page-less document cannot become a representation at all**, let alone a grounding artifact. So
§3's claim that the empty `pages` vector spells *"this document has no pages"* is true of the **type**
and false of the **invariant**: today the empty vector is only legal for a document with no nodes
either. Reaching v2's gate is upstream of grounding, in the IR's own page-parent invariant — a v2
design decision about the representation rather than a schema question. `15` hands it to S2, which
now knows it **before** writing a reader against an invariant that would have rejected its output.

**`04-ARCHITECTURE.md` §6's precondition — *"provided `engine-grounding` never learned about
pages"* — holds, and points at the wrong crate.** This crate never learned what a page *is*: it
reads no locator, derives no geometry, and addresses pages by id. `project()`'s check that
`node.parent` names a declared page is a **re-assertion of an invariant `seal` already guarantees**,
not independent knowledge — it can never be handed a page-less representation, because one cannot be
constructed. The assumption lives in `engine-core`, which the M5 line permits.

Four guards in `crates/engine-grounding/tests/page_less_source.rs`: the three schema walls; the seal
refusal with its named message; **the same document with its page declared, which seals and
projects**, so the refusal test fails for the page and for nothing else; and a `Cargo.lock` scan for
LibreOffice, soffice, headless Chrome, wkhtmltopdf, WeasyPrint, chromiumoxide and printpdf, because
**L30 lapses as a transitive dependency before it lapses as a design decision**.

`project()` is untouched — relaxing the PDF path's page requirement "for office" would change PDF
behaviour to accommodate a format that does not exist here yet. No reader, no `zip`, no
`quick-xml`, no `engine-office`, no schema field.

### Identity, after v2-S1

Workspace **0.20.0**, profile hash
`sha256:30820a15ee750530f5232e622ee40cbfad3e8cb158d41011104cc25f5fd06c15`. Nothing but the version
moved — the seventh time. The version moves because tests landed and because a slice that measures
still moves it, on the S7b and v1.2-S5 precedent.

### v2-S0 — office formats, scoped and not started

Two documents and nothing else: [`docs/14-V2-SCOPE.md`](docs/14-V2-SCOPE.md) and
[`docs/15-V2-MILESTONES.md`](docs/15-V2-MILESTONES.md), on the pattern `10`/`11` and `12`/`13` set.
**No code, no crate, no fixture, no schema change, and the workspace stays at 0.19.0** — the profile
hash does not move, because nothing about this build parses differently.

v2's gate is *a DOCX quote and an XLSX cell both ground; **no synthesised pages***, and the second
clause is the whole hazard. A DOCX has no page: where one appears is a decision a renderer makes
from a font stack and a paper size, so a `page` on a Word citation measures the machine that printed
it rather than the document — and it looks exactly like a locator, because it is shaped like one.
`06-STEAL-REFUSE.md` **L30** already refuses the shortest path to that (*"LibreOffice → PDF … It
invents pagination … This is a refusal, not a fallback"*), and `01-CONTRACT.md` §5.1 already says a
rendered page/bbox is never substituted for a native source address.

So S0's contribution is making that **checkable** rather than quotable. The law names the four
places a violation would show: the empty `pages` vector as the spelling of *"this document has no
pages"*, `GeometryAbsence::NotApplicableToKind` as the spelling of *"no geometry, and that is not a
gap"*, a new `NativeLocator` variant (§5.1 already names `DocxLocator` and `XlsxLocator`), and
`deny.toml`, where a renderer would appear as a dependency before it appeared in a review.

**One question is posed and deliberately not answered.** `ethos.grounding.v1` is a PDF schema —
`source.media_type` is `{"const": "application/pdf"}`, `element` requires `page` and `bbox`, `page`
requires integer `width`/`height`/`rotation` — so the word *"ground"* in v2's own gate is undefined
for a page-less format. Either the schema revises, as a deliberate change to the **verifier's**
contract with a rationale and tests, or grounding stays PDF-only and the gate is met at the
representation level. **Inventing a page to satisfy the schema is the third option and the only one
forbidden outright.** `15` makes that decision S1, ahead of any reader whose output would depend on
it — the shape v1.2-S0 used for the handle law.

`engine-office` is **named** as where an office crate would live (`04-ARCHITECTURE.md` already
named it) and **not created**: a fifth crate before a second format is speculative structure. A1's
14-format horizon is parked as a single row until DOCX and XLSX have shipped and the cost of the
third is measured rather than guessed. No LibreOffice, no Anydoc dependency, no JVM, no AGPL.

**v1 is still not done** — the S7 table-cell gate is measured and missed at 64‰ — and nothing in
this entry closes it.

### v1.2 — complete

**v1.2 is adoption, and it adds no parse feature at all.** `engine mcp` serves the stages that
already exist over MCP on stdio: newline-delimited JSON-RPC on a pipe, three tools, no new crate
and no new dependency. **v1 is still not done** — the S7 gate is measured and missed at 64‰ — and
nothing here closes it. **v1.1 stays complete** and untouched. Not tagged.

### S0 — the law, written before the host that could break it

`docs/12-V12-SCOPE.md` and `docs/13-V12-MILESTONES.md`, on the pattern `10`/`11` set for v1.1.
S2–S5 — Python SDK, Node SDK, LangChain tool, optional liteparse adapter — are named and **not
started**.

The scope document exists mostly to hold one paragraph. The parser memo §16.7 puts MCP first
without hesitation, and then says why it could be the worst choice instead of the best:

> MCP tools are model-controlled — the model chooses the arguments. If any tool accepts a locator
> as a free-text argument that the engine then trusts, the model has become the citation authority
> in a single step.

A model that types `{"page": 3, "bbox": [10, 10, 90, 40]}` into a tool this engine believes has
*become* the thing the repository exists to prevent, and the output looks exactly like a citation
because it is shaped like one. §16.7's mitigation is structural rather than advisory — **the engine
mints every locator, returns it as an opaque handle, and re-validates it on the way back in** — and
`12-V12-SCOPE.md` §3 turns that into three obligations plus a corollary that can be tested:
**no tool argument may name document geometry.**

**A forged handle fails closed** — an error, never an empty result. An empty result tells a model
its guess was merely unlucky; an error tells it the guess was not admissible.

### S1 — the adapter, and where each half of it lives

| tool | argument | returns |
| --- | --- | --- |
| `extract` | `path` | `DocumentRepresentation v0`, byte-identical to `engine extract` |
| `ground` | `representation` | `ethos.grounding.v1` |
| `node_get` | `representation`, `node_id` | the node record from **that** artifact |

`node_get` is the version gate in its smallest form. The representation comes back inline or as a
path, is `verify_fingerprint()`-checked before anything reads it — so a model that edited the JSON
on the way through is refused rather than answered — and the id is looked up among that artifact's
own nodes. An id nothing minted is a tool error naming what was refused.

**`markdown`, `html` and `verify` are deliberately absent.** The first two would be a few lines
each and neither proves anything this slice claims; a tool that exists because it was cheap is a
surface to keep honest forever. `verify` relays the pinned Ethos CLI, and wrapping a relay in a
second protocol is a second place for a verdict to be re-derived — which is exactly what
`07-VERIFY-BOUNDARY.md` must not bend for a host's convenience.

**Locators live in `structuredContent`; `content` carries counts.** That split is §16.7's, and it
is the difference between a locator a pipeline binds and a locator a model edits. A test asserts no
coordinate reaches the model-facing string.

### No framework, and no widening of the ban

`deny.toml` bans `tokio`, `hyper`, `reqwest`, `ureq`, `rustls` and the rest of the reachable
network surface. The MCP crates available pull an async runtime, which would spend that ban to save
a few dozen lines of `match`, so this is a loop over stdin with the `serde_json` the workspace
already had. `Cargo.lock` gains nothing; `cargo deny check` is the standing proof.

stdio is a pipe rather than a socket, and newline-delimited JSON-RPC is MCP's own stdio transport
rather than a dialect invented here.

### Identity, after S1

Workspace **0.15.0**, profile hash
`sha256:c5f06d323e5fe0028e779edd622e9c6a35aa7f3eae73c02188c35bb6be286c64`. **Nothing but the
version moved** — the second time that has happened, after v1-S7b (0.9.0). No field, no rule id, no
capability: a transport is not a parse capability, and `capabilities.mcp` would put a flag on every
artifact that no consumer could act on. Two artifacts either side of this hash say exactly the same
thing about the same document, and the hash moves because `parser_version` is in it.

### S2 — the Python SDK, and the decision that makes divergence impossible

`packages/python/` — **`extract`, `ground`, `node_get`, and nothing else.** It wraps the `engine`
binary with `subprocess` and parses stdout with the stdlib `json` module, so **byte-identity with
the CLI is a tautology rather than a promise**: there is no second serialization anywhere in the
package for an artifact to change in. A test re-canonicalizes what `extract` returned and compares
it against the CLI's stdout byte for byte.

**PyO3 was refused.** It would reach the library by a second path, which is a second thing that can
disagree with the first — plus a wheel matrix, a fifth build surface, and a route by which a Rust
dependency could arrive on the Python side of the fence. The cost of the shape chosen instead is
one process spawn per call, which nobody has measured a need to avoid. **Runtime dependencies are
empty**, asserted from `pyproject.toml` and again by walking every `import` in the package against
`sys.stdlib_module_names`.

There is no MCP client here either: MCP is a process and this is a library. They are two callers of
one binary, not layers.

| function | shells out to | returns |
| --- | --- | --- |
| `extract(pdf_path)` | `engine extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `engine ground <path>` | `ethos.grounding.v1` |
| `node_get(representation, node_id)` | **nothing** | the node record from **that** artifact |

**`ground` takes a representation, not a quote and not a page.** `engine ground` takes neither — it
projects the record into `ethos.grounding.v1` — and a locator-shaped argument would be §3's
corollary violation in its purest form.

**`node_get` has no subcommand behind it**, and this slice did not add an `engine node-get` for
symmetry. MCP already carries the tool. So its checks are ported in the order `mcp.rs` runs them:
the value is a representation; its payload is re-canonicalized and re-hashed and must equal
`representation_c14n_sha256`, **before any lookup happens**; the id is looked up among that
artifact's own nodes, and a miss **raises**.

That is why `_c14n.py` exists — c14n v1 in Python, running `engine-core/src/c14n.rs`'s own parity
vectors. A fingerprint that were merely *nearly* the engine's would be worse than none: it would
accept an artifact the engine refuses, or refuse one the engine minted, and either way a caller
would be told something false about a document. The load-bearing proof is not the five vectors but
the whole artifact, which agrees on real input.

**The handle law, in Python:** a minted id returns the node the artifact carries; `s-forged` raises
`NodeNotFound` naming what was refused, never `None` and never `{}`; an edited payload raises
`FingerprintMismatch` naming both digests. **No public signature contains** `page`, `bbox`, `x`,
`y`, `width`, `height`, `row` or `column` — read off `inspect.signature`, against the same banned
list `mcp.rs` uses.

`markdown()`, `html()` and `verify()` are **absent**. The first two would be a few lines each and
prove nothing this slice claims. `verify` relays the pinned Ethos CLI, and a Python function of
that name would look like this package had an opinion about whether a claim is supported —
`07-VERIFY-BOUNDARY.md` is exactly what an SDK's convenience must not bend.

**Not published.** It is not on PyPI and this slice does not put it there.

### Identity, after S2

Workspace **0.16.0**, profile hash
`sha256:1b7a4208734b9ed52f1c0b2b725322bc854ffac229c019c01226203c67c5a81a`. **Again nothing but the
version moved** — the third time, after v1-S7b (0.9.0) and S1 above. An SDK is an adopter rather
than a parse capability, so there is no `capabilities.python` for the reason there is no
`capabilities.mcp`: a flag on every artifact that no consumer can act on. No Rust changed but the
version string.

### S3 — the Node SDK, which is the Python one in a second language

`packages/node/` — **`extract`, `ground`, `nodeGet`, and nothing else.** Same shape as S2 and for
the same reasons: spawn the `engine` binary, parse stdout, hand back the bytes the CLI printed.
JavaScript and `node:test`, no TypeScript, no bundler, no test framework, no runtime dependency.

**It is not a second design.** S2 is the contract, and if Node disagreed with Python about a
signature, an error name, or what `ground` accepts, Node would be the one that is wrong. The
differences are exactly the two the languages force — `nodeGet` rather than `node_get`, and classes
that throw rather than raise — and nothing else. Both SDKs and `package.json` are pinned to the
workspace version by a test, because two adapters at different versions over one binary is exactly
the disagreement a version string exists to make legible.

**No native addon.** napi and neon were refused for the reason S2 refused PyO3: a second path to
the library is a second thing that can disagree with the first, and this one would add a prebuild
matrix across platforms and ABI versions to buy it.

`ground` takes the artifact `extract` returned — object or path — and when handed an object it
writes **c14n bytes** to a temp file rather than `JSON.stringify` output, because a second
serialization is the one thing this package exists not to have. It does not repeat the fingerprint
check in JavaScript: the engine runs it, and the SDK surfaces the engine's own stderr.

**c14n ported a second time**, running `engine-core/src/c14n.rs`'s own parity vectors, with two
JavaScript-specific hazards handled rather than hoped: `Array.prototype.sort` compares UTF-16 code
units and disagrees with Rust's `String: Ord` above the BMP, so keys are sorted by **code point**
explicitly (a test asserts the default sort would have got that pair wrong); and `Buffer.from`
encodes a lone surrogate as U+FFFD, which is a silent repair of evidence, so the port throws.

**One divergence is real and is written down rather than papered over.** JavaScript has a single
number type, so `JSON.parse("1.0")` yields the same value as `JSON.parse("1")` and nothing can
separate them — Rust and Python reject float-*shaped* text and this port cannot. What all three
reject identically is a value that is genuinely not a whole number: `1.5` throws at every depth,
which is the property c14n needs. The unreachable half is unreachable in practice, because the
engine never prints `1.0`. A test pins the divergence so it stays a named property.

**The handle law, in Node:** a minted id returns the node object the artifact carries; `s-forged`
throws `NodeNotFound`; an edited payload throws `FingerprintMismatch` before any lookup. No
exported parameter is named `page`, `bbox`, `x`, `y`, `width`, `height`, `row` or `column` — read
out of `Function.prototype.toString`, because `Function.prototype.length` says nothing about names.

`markdown()`, `html()` and `verify()` are absent, on S2's reasoning exactly. **Not published:**
`private: true`, which is mechanical rather than a promise.

### Identity, after S3

Workspace **0.17.0**, profile hash
`sha256:83cd55301d2423d54033e449b2bcdbd07b5a5c926c441dc456cb93edc7788774`. **Again nothing but the
version moved** — the fourth time, after v1-S7b (0.9.0) and S1 and S2 above. A language binding is
an adopter rather than a parse capability, so there is no `capabilities.node` for the reason there
is no `capabilities.python` and no `capabilities.mcp`. The only edit to the Python SDK is its
`__version__`, which the workspace bump requires: leaving it at 0.16.0 would be the lie its own
test exists to catch.

### S4 — LangChain tools, and the split is the whole slice

`ethos_engine.langchain` and `ethos-engine/langchain` — **`extract`, `ground`, `node_get` in both
languages**, each calling the SDK that already exists. Nothing spawns `engine` a third way and
nothing here speaks MCP; if a tool returned bytes an SDK would not, the tool would be wrong.

**`12-V12-SCOPE.md` §3's second corollary, implemented a second time.** Locators live in the
artifact, never in the prose a model reads and edits. MCP says that with `structuredContent` versus
`content`; LangChain says it with a tool's `artifact` versus its `content`, and
`response_format="content_and_artifact"` is what makes both sides real. Without it the artifact is
stringified into `content`, and a box in `content` is a locator a model can edit and then cite —
the failure this whole version is arranged to prevent. Both optional pins are bounded on **both**
sides for that one reason.

| tool | `content` | `artifact` |
| --- | --- | --- |
| `extract` | `{n} page(s), {m} node(s). Locators are in the artifact.` | `DocumentRepresentation v0` |
| `ground` | `{n} element(s) with a measured box; {m} omitted for having none.` | `ethos.grounding.v1` |
| `node_get` | ``1 node, kind `{kind}`.`` | the node record |

**MCP is the oracle for those strings, not this slice's opinion.** Both suites compare `content`
byte-for-byte against what `engine mcp` emits, on one fixture where nothing is omitted and one
where everything is. That stops a second adapter inventing a richer sentence than the first, and it
is the proof that `ground`'s omitted count — computed out here as **nodes minus elements** — equals
the engine's own `omission.nodes_omitted`. Counting geometry rows instead would re-encode
`GeometryPresence::is_groundable` in two more languages, and a count derived from a different
question than the one being asked goes wrong the first time a second absence variant appears.

`node_get`'s summary names the **kind** and never the id, spelled the way the artifact spells it
(`text_run`) rather than the way `engine mcp` prints Rust's `Debug` (`TextRun`): reshaping it would
be the adapter inventing a name for a thing it did not read.

**One wire shape.** The three argument schemas are plain JSON Schema, verbatim from what
`engine mcp` advertises, and a test in each language asserts them against `tools/list`. `node_id`
keeps MCP's spelling in both languages even though the Node SDK's parameter is `nodeId` — the tool
argument is the wire, and one wire has one name. JSON Schema rather than pydantic models or zod, so
the adapters compare object to object and the Node package still needs nothing but
`@langchain/core`.

**The install stays empty.** `langchain-core` is an optional Python extra and `@langchain/core` an
optional Node peer, both reached on a subpath, so `import ethos_engine` and `import "ethos-engine"`
still pull nothing — asserted off the **default entry point's** import graph. Importing the subpath
without the dependency is a named failure carrying the install command; in JS that is a dynamic
import inside a try/catch at module load, so it fails where Python's `ImportError` fails rather
than as Node's generic module-not-found.

**No LangGraph adapter**, in either language: §16.7 refused one, because a bindable tool is already
what `bind_tools` and a `ToolNode` take. **No trust state** anywhere — no `grounded`, no
`verified`, no evidence tier, no score, and no `verify` tool. A forged id, an edited payload or an
unreadable document **raises**, which is MCP's `isError: true` in this framework's currency.

### A false green this slice found and closed

Both SDK suites preferred `target/release/engine` and took the first file that existed. On a tree
with a stale release build that was a **`0.11.0`** binary, and every S2 and S3 assertion passed
against it — the byte-identity checks compare the SDK against the CLI using the same binary, so
they are self-consistent whichever one it is. They proved what they claim, about the wrong engine.

Both locators now read `engine --version` and refuse a binary that is not this workspace's: an
explicit `ETHOS_ENGINE` pin errors rather than being silently overridden, and the search skips a
build from another version and names what it found.

### Identity, after S4

Workspace **0.18.0**, profile hash
`sha256:2bf3e74e6a3b972669cbc03afda66c4949b870d9695a7739dfc255a859485fee`. **Again nothing but the
version moved** — the fifth time, after v1-S7b (0.9.0) and S1, S2 and S3 above. A framework binding
is an adopter rather than a parse capability, so there is no `capabilities.langchain` for the reason
there is no `capabilities.node`.

### S5 — the liteparse adapter, measured and REFUSED

S5 asked for a `liteparse → ethos.grounding.v1` mapper **"if it is worth it"**. It was run down
rather than assumed either way, and **no adapter ships**: no mapper, no subcommand, no foreign
parser, no fixture. Two walls, and both are properties of `ethos.grounding.v1` itself rather than of
any document, so no adapter could clear them by being careful.

**Wall 1 — the producer cannot name itself.** Measured from `output/json.rs:46-65` and recorded in
the parity checklist §8: LiteParse *"emits `page, width, height, text, text_items` and nothing
else"* — checklist **L20**, the missing versioned output contract this repository exists to attack.
The grounding schema **requires** `producer: {name, version}`, both non-empty. A caller-supplied
version is not a way out: `additionalProperties: false` runs that schema's whole length, so there is
nowhere to record that an identity was *asserted* rather than *measured*, and a claimed version
would be indistinguishable from one the engine read. `VerifierBinary::identify` sets the precedent
in the opposite direction — a verifier is pinned by version **and** binary digest.

**Wall 2 — the boxes are loose and the schema cannot say so.** Checklist **L18**: their bbox is a
union of `FPDFText_GetLooseCharBox`, em boxes ascent-to-descent, **not ink**. `01-CONTRACT.md` §5.3
requires an emitted box to declare what kind it is, and says loose boxes would be *"declared
separately"*. There is no such field and no room for one. Loose boxes here would be L18 — *"sold as
precise positioning, with nothing in the output saying which it is"* — reproduced inside this
repository's own `artifact_type`, which is worse than shipping nothing, because it would look like
evidence.

**The hazard everyone predicted was not the one that bit.** The memo said an adapter *"would declare
`coordinate_origin: unknown` unless it also reads the source PDF"*. That dissolves: LiteParse's
space is top-left, 72 DPI, `CropBox`→`MediaBox`, and this engine's visible box is `/CropBox` clipped
to the media box or the media box where none is declared — same box, same origin, and 72 DPI means
points × 100 is centipoints exactly. Floats are survivable too: a value that will not land on an
integer centipoint is an omission with a count. **The refusal is about provenance and box semantics,
not geometry**, and recording which hazards were false is what makes it re-openable on evidence.

**No refusing subcommand, deliberately.** One that can only exit 2 is a permanent surface that does
nothing — S1's argument for refusing cheap MCP tools — and refusing *per document* would need a
parser for a JSON shape this tree has no sample of, so it would reject real LiteParse output as
malformed when the truth is that this engine guessed their schema.

Instead the refusal is executable the way this repository makes rules executable:
`crates/engine-grounding/tests/liteparse_refusal.rs` asserts `producer` requires a non-empty name
and version, that no property anywhere in the schema declares box semantics, and that
`additionalProperties: false` leaves no room to add one. Relax any of those and the test fails, and
S5 is reopened **deliberately**.

### Identity, after S5

Workspace **0.19.0**, profile hash
`sha256:3ad382d0bfb4cbc53cdd74f8dae44e4807c5764452e34517ca384503fdc96f5f`. Nothing but the version
moved — the sixth time — and this one shipped no feature at all. It bumps on the precedent of
v1-S7b (0.9.0), where a slice measured a dead end and moved the version anyway: two builds that
disagree about what this repository decided must not both call themselves 0.18.0.

### Unchanged

Every artifact and every rule: `markdown-blocks-v2`, `html-blocks-v2`, the three table rules, the
representation (0.5.0). The table gate (**still missed at 64‰**), the oracle (12/3),
`irs-form-1040-2025` at 0 tables, fabrication 0, and all four v1.1 goldens. `project()` and
`engine ground` are untouched — `engine-grounding/src` has no diff in this slice. Four crates, no
new Rust dependency, no HTTP, no SSE, no socket, no Tokio, no TLS. No PyO3, no maturin, no napi, no
neon, no node-gyp, no native extension, and **no default-install dependency in either SDK**. No
LangGraph, no LlamaIndex, no Haystack, no liteparse, no PDFium. Not on PyPI, not on npm. No tag.

**v1.2 is complete.** **v1 is not** — the S7 table-cell gate is measured and missed at 64‰, and
nothing in this entry closes it.

---

## v1.1-S4, HTML under the same four laws, as 0.14.1

**A second projection, and it is not the first one with angle brackets.** `ethos.html.v1` carries
an HTML string, the same **Anchor Map** that inverts every source byte of it back to representation
nodes, and the same character census — under its own rule id, `html-blocks-v2`. `markdown_rule`
does **not** move: the Markdown a document produces is byte-for-byte what 0.13.0 emitted. **v1 is
still not done** — the S7 gate is measured and missed at 64‰ — and nothing here closes it. Not
tagged. **v1.1 is complete.**

### Why this earns a slice rather than a stylesheet

**GFM cannot say `rowspan`, and HTML can.** That is the whole case, and without it a second
artifact would be a second thing to keep in sync for no gain.

v1.1-S2 had to expand every merged cell into the slots it covered and count what that cost, because
a table reading `| North | merged span |  |` has quietly become a table with an extra empty cell.
On `markdown-table-cells` the two artifacts now say different things, and the difference is
asserted rather than described:

| | `ethos.markdown.v1` | `ethos.html.v1` |
| --- | --- | --- |
| the merged cell | expanded — text in the origin, covered slot empty | `<td colspan="2">merged span</td>`, covered slot emits **nothing** |
| `gfm-span-slots-unrepresentable-v1` | **1** | **0** — the grid held the merge |
| `gfm-row-zero-separator-v1` | **1** — the delimiter row asserts a header the document never declared | **0** — every cell is a `<td>` |

**The two dropped codes are dropped because HTML does not commit them — not because they are
named `gfm-*`.** Three of the six describe faults in the *record* rather than limits of GFM
(`gfm-cell-not-placed-v1`, `gfm-cell-run-claimed-twice-v1`, `gfm-table-not-projected-v1`), and HTML
meets those identically, so it keeps them. `gfm-span-slots-unrepresentable-v1` is **recomputed**
rather than dropped: a merge costs HTML nothing *unless the grid cannot hold it*. On
`ruled-table-overlap`, whose row 1 declares both a `colspan: 2` cell and an ordinary cell in the
next slot, the span is clamped so the row stays two cells wide — and the merge that was lost is
declared, count 1. Only `gfm-row-zero-separator-v1` is unconditionally absent, because HTML asserts
no header.

**No `<th>` appears anywhere in the projection.** Neither detector reads `/TH` and the
representation carries no header declaration, so a header row would be this exporter deciding what
the document meant. GFM left S2 no choice, which is why it owed a code for it and this does not.

### The one erasure HTML still declares, and the name it has to keep

`gfm-list-item-run-joins-v1`, on **both** artifacts. Two sibling `/LI`s have identical role paths,
so nothing in the representation distinguishes "the rest of this item" from "the next item". Every
projection has to guess; HTML guesses the way Markdown does, because two artifacts of one document
that disagreed about how many list items it has would both be wrong to cite.

Its `gfm-` prefix is **historical rather than descriptive** — the erasure belongs to the tagged
tree, and it carries the name of the slice that first met it. Renaming it would change what
`ethos.markdown.v1` says under a `markdown_rule` this slice deliberately does not move, and a rule
id that stayed put while its output changed is the one dishonesty a version id exists to prevent.

### Reused rather than rewritten, and the one thing that could not be

Everything that decides *what* to emit is called, not copied: `normalize`, `heading_level`,
`list_role`, `dropped_code`, `hyphen_tail` and `plan_tables`. A second copy of the hyphen predicate
would be a second rule free to drift, and only one of them would have the corpus test that caught
`nonescr`.

The census is closed by **one shared function**, so the two artifacts *cannot* disagree about what
a document contains — and `the_two_projections_agree_about_every_character` asserts exactly that,
field by field and bucket by bucket, on every fixture in the sweep.

What could not be shared is lists. A Markdown item is a line and needs no state; an HTML one is a
`<li>` inside a `<ul>` that must be opened, nested inside its parent's still-open `<li>`, and
closed. That is why `html.rs` has a walk of its own rather than a vocabulary handed to the other.

`TablePlan` grew `roles` for this slice — `Origin { rowspan, colspan }` / `Covered` / `Empty`.
GFM never needed it, because expanding every merge makes a covered slot and an empty one identical.
It is computed in the walk `plan_tables` already did, and `placed` is deliberately left alone so
S2's `gfm-cell-not-placed-v1` counts do not move.

### Entities, and why the whole one is `source`

`&`, `<` and `>` come out as `&amp;`, `&lt;`, `&gt;`. S2 splits the GFM pipe escape — backslash
`syntax`, pipe `source` — because the exporter added a byte *beside* a real one. An entity is
different: it **replaces** the character, and `&lt;` contains no `<` to label. So the entity is
emitted whole as `source`, and the census is told it stands for **one** character rather than four.

That last part is load-bearing. Law 4 counts characters of node text, not emitted bytes; letting an
entity inflate the count would push `emitted` past what the representation holds and underflow the
whitespace residue. Inverting a `source` segment on this artifact therefore means HTML-unescaping
it — a total, lossless transform, said out loud in the module, the schema and the milestone.

### A fragment, deliberately

No `<html>`, `<head>`, `<body>` or doctype, and **no indentation**. Those are bytes no document
drew; the map would tile them honestly as `syntax`, but they would sit inside quotes a consumer is
likely to lift. Embedding a fragment is one concatenation; unwrapping a document is a parse.

### Identity

`html-blocks-v2`, `capabilities.html`, workspace **0.14.1**, profile hash
`sha256:c5f06d323e5fe0028e779edd622e9c6a35aa7f3eae73c02188c35bb6be286c64`. Two fields arrived, so
the hash moves for a reason a reader can name. A profile JSON with no `html_rule` is **refused**,
not defaulted — the posture `markdown_rule` and `table_detection.stroke_ruled` each took.

**The rule id is `-v2` although `-v1` was never released.** `html-blocks-v1` existed for one commit
and got two tables wrong: a span colliding with another cell's origin widened a row past the
declared grid, and a list whose tree skipped a depth came out unbalanced. Nothing was published
under it — never tagged, never pushed — so no consumer holds such an artifact. The id and the
version move anyway, because two builds in this repository's own history producing different bytes
under one id is precisely the state a rule id exists to make impossible. A patch bump, on the
precedent of the S6 repair slices (0.8.1, 0.8.2).

`capabilities.html` is `true` and names `an_html_quote_verifies_end_to_end` as its proof, which the
capability scan can see. `html.draft.json` ships **with** its guard,
`the_html_schema_pins_the_version_and_rule_the_code_emits` — the rule v1.1-S3 wrote down after
finding a schema that had drifted for want of one.

### Unchanged

`markdown_rule` and every byte of Markdown it produces. The representation (0.5.0), `extract`, the
three detection rules, the table gate (**still missed at 64‰**), the oracle (12/3),
`irs-form-1040-2025` at 0 tables, fabrication 0. Four crates, no new dependency, no HTML parser, no
fixture added. **`engine-pdf` imports neither projection** and its `src/` has zero mentions of
Markdown; its three `html` mentions are pre-existing magic-byte sniffing (a file that *is*
HTML) and one analogy in a doc comment. Its `tests/` names the new capability, which is where
the capability table lives. No CSS, no JS, no MCP, no tag. v1.2 has not started.

---

## v1.1-S3, the hyphenation join, as 0.13.0

**A word the page broke across a line reads as one word — in the export, and nowhere else.**
`markdown_rule` moves from `markdown-blocks-v1` to **`markdown-blocks-v2`**, and
`hyphenated-line-break` projects `hyphenated` where it used to project `hyphen-\n\nated`.
Everything else about a document projects exactly as it did. **v1 is still not done** — the S7
gate is measured and missed at 64‰ — and nothing here closes it. Not tagged.

### The scope was narrowed, and the row split

S3 was written as "HTML **and/or** export-only cosmetics". Those are two products: HTML is a second
artifact owing the same four laws (checklist **O9**), a hyphenation join is a few lines in
`to_markdown`. Shipping both under one label would have made "S3 is done" unreadable, so **HTML is
now S4, not started**, and this entry is exactly what shipped.

**Dot-leaders and drop-caps are not implemented and have no fixture.** P16 stays `[I]`. A `....`
collapse invented with nothing to measure it against is the speculative work the standing rules
refuse.

### The export joins; the evidence record does not

`hyphenated_line_breaks_are_not_rejoined_and_that_is_the_policy` is **untouched and green**:
`extract` still emits `hyphen-` and `ated` as two `Extracted` runs with the hyphen verbatim. That
is a policy, not an omission — telling a soft break-hyphen from a real compound one ("well-known"
split across lines) needs a dictionary, and a rule that guesses is the cliff-shaped heuristic this
project refuses everywhere. `docs/10-V11-SCOPE.md` §5 puts a cosmetic in the export or nowhere, and
checklist **P15** says the same thing from the other side: the export repairs, `element.text` does
not.

**So there is a quote that reads perfectly and does not ground: `hyphenated`.** No element of
`ethos.grounding.v1` contains it. That is the correct answer for a word the page drew in two
pieces, not a defect in the verifier — and it is stated rather than left to be discovered:

| where | what it says |
| --- | --- |
| the map | **one `source` segment** over the joined letters, naming **both** runs — so the two citable strings are recoverable from the artifact |
| `coverage.dropped` | `hyphenation-rejoin-dropped-v1` — `chars` is the hyphens removed, `nodes` the runs that lost one |
| the census | still balances, with the hyphen on the dropped side |

One segment rather than two adjacent ones, deliberately: the hyphen that marked where the halves
met is gone, so there is no offset at which the first run stops being the answer and the second
starts. Splitting would put that boundary somewhere and claim a precision the join threw away.

A **character** bucket rather than a `structural_erasure`, and that is the whole test for which
census a disclosure belongs in: a hyphen *is* a character of node text and it really is not in the
Markdown. The GFM erasures are counted separately precisely because their characters are all still
there and a `tables-flattened` character bucket would read `0`.

### The rule, in one sentence

Adjacent text runs, same page, **on different baselines**, the first's **emitted** text ending in
an ASCII `-` with a letter in front of it, the second starting with a letter. Not across a cell, a
list item, a heading, or the page-furniture boundary; not `foo -`, where the dash is its own word
rather than half of one; and the *immediately* next node, so an annotation or a form field between
the halves stops the join rather than being reached past.

**Pairwise, once.** A word broken twice joins its first pair and leaves the second hyphen where it
is — a stated bound, not an oversight: the census still balances and the segment still names
exactly the runs it came from.

### The line test was found by measuring, and it is the whole rule

Specified as "adjacent runs, same page", the rule joins any run ending in `-` to the run after it
— **including two fragments of one line**. Run against the benchmark corpus, that version fired
exactly once, and it fired wrongly: `cfpb-home-loan-toolkit` page 24 draws `non-escrowed` as a
string of tiny runs at a single baseline — `non-`, `escr`, `o`, `w` … — and the join produced
**`nonescr`**. A compound hyphen the author wrote, deleted; a word that is not a word; and the only
place on the whole corpus the feature did anything at all.

| corpus document | joins without the baseline clause | with it |
| --- | --- | --- |
| `cfpb-home-loan-toolkit` | 1 — `non-` + `escr` → `nonescr` | **0** |
| `irs-form-1040-2025` | 0 | 0 |
| `nist-sp-800-63b` | 0 | 0 |
| `synthetic/hyphenated-line-break` | 1 — `hyphen-` + `ated` | **1** |

So the rule requires the two runs to sit on **different `origin_y` baselines**. A hyphen inside a
line is a hyphen the author wrote; only a hyphen at a line's end is a candidate for having been put
there by the break, which is the premise of P15 and of the fixture's own name. The test is
deliberately conservative — a run with no glyph-run locator, or a page whose lines do not separate
in `origin_y`, simply does not join — because a missed join reads as the two words the page drew
and a wrong join invents one, and this project's fabrication count is 0.

Pinned by unit tests on the exact `non-` / `escr` shape, on a dangling `foo -`, and on a word
broken twice.

### A running head is the one block boundary no other clause could see

`heading_level` and `list_role` both bail unless the locator is `PdfTagged`, so a `pdf_artifact`
run — a running head, a footer, a folio — passed every other tail guard. A page whose last body
line ends in a soft hyphen and whose footer is the next node in reading order projected
`Rates may be recalcu-` + `Confidential draft` as **`recalcuConfidential`**: a word on no page,
welded out of two streams the document itself declared separate (PDF 32000 §14.8.2.2, and
`engine-pdf`'s own binding rule — *artifact wins*).

It also broke a promise `to_markdown` rule 1 makes out loud. Artifacts are kept in the projection
precisely so **a consumer that wants them gone drops them itself, knowing it did** — checklist
O21/O22 — and the per-run `source` segment is the only handle a consumer has for that. One segment
spanning body text and a running head takes the handle away.

So the two halves must be **both furniture or both not**. Equality rather than exclusion: a
two-line running head hyphenates like any paragraph, and that case still joins. What may not happen
is a join across the boundary — pinned in both directions, with an assertion that no `source`
segment spans it.

### The golden, and the third fixture it needed

S1's golden refused a quote spanning a blank line; S2's refused one carrying table chrome. Both are
*punctuation* a careful reader might squint at. S3 produces something harder — a joined word that
is ordinary English in the middle of an ordinary sentence, with nothing about it to squint at:

```text
     page:  The rate may be recalcu-
            lated at closing
 markdown:  The rate may be recalculated at closing
```

A model handed that Markdown would cite it without hesitation, and the page never drew it. So
`the_joined_word_does_not_ground_and_both_halves_do` runs the four real binaries — `extract`,
`ground`, `markdown`, `verify` — with the pinned Ethos CLI deciding:

| quote | verdict |
| --- | --- |
| `The rate may be recalcu-` — the first half, hyphen and all | **grounded** |
| `lated at closing` — the second half | **grounded** |
| `recalculated` — the word only the export contains | **`text_mismatch`** |

**The reason is pinned, not just the verdict.** `element_not_found` would mean the citation pointed
at nothing and the test would pass without the cosmetic having been examined at all; the joined
word is cited against an element that *does* exist, so the verifier finds it and judges the text.
`recalculated` is never asserted grounded — not a gap, but exactly what `docs/10-V11-SCOPE.md` §5
buys.

### Fixtures

- **`markdown-hyphen-break`** — two runs 30 points apart under a font declaring real ink metrics,
  the first ending `recalcu-`, so **both halves reach `ethos.grounding.v1`** as elements a verifier
  can actually find.

  It had to be authored for the reason `markdown-two-blocks` and `markdown-table-cells` each had
  to be. `synthetic/hyphenated-line-break` already carries the shape — `hyphen-` then `ated` — but
  its font declares no ink metrics, so both runs take the typed-absent path, the `elements` array
  comes out empty, and a golden against it would watch the verifier find nothing and refuse every
  quote. That proves nothing about the join.

  In `fixtures/manifest.json`, so the mutation suite covers it without anyone remembering to add
  it. Its one surviving mutant is `junk-after-eof`, the same class as every other engine fixture,
  triaged rather than pinned on sight: re-extracted, it gives back the same two runs with the same
  **measured** ink boxes, which is precisely what its golden depends on.

### Identity

`markdown-blocks-v2`, workspace **0.13.0**, profile hash
`sha256:4712002b4ada5138c0d49a6a8336a612f2daf470f5591f8720e994ec96f77c8c`. No field arrived and no
shape moved — the rule id changed *value*, because a document with a hyphenated line break comes
out differently under the two rules, and two artifacts either side must not compare equal. 0.13.0
rather than 0.12.1 for the same reason 0.12.0 was not a patch.

### Unchanged

The representation (0.5.0) and every artifact shape. The three detection rules, the table gate
(**still missed at 64‰**), `extract`, the oracle (12/3), `irs-form-1040-2025` at 0 tables,
fabrication 0. Regenerating `fixtures/engine/` leaves every pre-existing document byte-identical.
Four crates, no new CLI, no new artifact type. `engine-pdf` still has no Markdown in it. No HTML,
no dot-leaders, no drop-caps, no MCP, no tag. v1.1-S4 has not started.

---

## v1.1-S2, GFM tables and lists, as 0.12.0

**The grid ships, and the flattening is counted.** `markdown_rule` moves from
`markdown-linear-v1` to **`markdown-blocks-v1`**: a table on the representation becomes a GFM
table, a run the structure tree places in an `/L` becomes a list item, and everything else
projects exactly as it did. **v1 is still not done** — the S7 gate is measured and missed at 64‰ —
and nothing here closes it. Not tagged.

### What S1 declared, and why that sentence is now deleted

S1 emitted a table's cell runs as consecutive paragraphs and declared the lost grid as
`markdown-table-structure-not-projected`. The grid is here now, so that sentence is false — and a
**false limitation is worse than a missing one, because a reader acts on it**: this one would send
them to `tables` on the representation for a grid the Markdown now has. The code is **deleted**,
the way v1-S2 and v1-S8 retired their own disclosures when they shipped the rules that made them
untrue, and a test asserts it cannot come back.

What replaces it is narrower and true of every GFM table there will ever be:
**`markdown-table-spans-flattened`**. GFM has no `rowspan`, no `colspan`, and no headerless table.

### The erasures have numbers, which is what A14 asks for

`coverage.structural_erasures` — a **second** census beside the character one, `{code, count}`,
sorted, non-zero only. Separate deliberately: a merged cell's text is emitted in full, so nothing
is dropped and a `tables-flattened` *character* bucket would read `0`, which is A14's own example
of a disclosure that discloses nothing. What a merge costs is **slots**.

| code | counts |
| --- | --- |
| `gfm-span-slots-unrepresentable-v1` | slots a merge covered that GFM cannot say it covered — `rowspan × colspan - 1` |
| `gfm-row-zero-separator-v1` | once **per table**: the delimiter row makes row 0 a header, and neither detector reads `/TH` |
| `gfm-cell-run-claimed-twice-v1` | a run two cells both claimed, kept by the first so one node is not counted twice |
| `gfm-cell-not-placed-v1` | a cell outside the declared grid or on a taken slot — its runs still project, as paragraphs |
| `gfm-table-not-projected-v1` | a table with zero rows or columns |
| `gfm-list-item-run-joins-v1` | a body run appended to an already-open list item |

Once per table rather than once per cell in row 0, pinned by a test: the erasure is **one claim**,
*this table has a header*, made once about one table. Counting its cells would make a wide table
look like a worse lie than a narrow one when both told exactly one.

### The representation had to carry a link it already knew

`TableCellRecord` arrived at S1 holding a cell's text and **no** way back to the runs it is a
concatenation of. The detector has always known — `DetectedCell::run_indices` — and the conversion
dropped it. A consumer holding only the string can get back two ways and both are wrong: re-run
the geometry, which is a second copy of the detector's rule that can drift from the first
(checklist A12), or match the text, which is a guess the moment two cells hold the same word.

So `TableCellRecord` gains **`node_ids`** and the representation goes to **0.5.0**. This is a fact
the detector computed carried across a boundary, not a second detection — nothing re-reads a box —
and `DocumentRepresentation::seal` now refuses a record whose cell names a run it does not declare.
**The law forced the field:** `AnchorMap` refuses a `source` segment that names no node, so without
this a GFM cell could not have been emitted as source at all.

### Decisions worth naming

- **A cell's runs are emitted once.** The table goes where its first run goes in reading order, and
  those runs are then not also paragraphs. The characters *move*; they are not duplicated and not
  dropped. A document with no table and no list comes out **byte-for-byte as it did at S1** —
  asserted on `simple-text` and `markdown-two-blocks` as literals, map geometry included.
- **The escape is syntax; the character it escapes is source.** A cell holding `A|B` is written
  `A\|B`. Emitting that as one source segment would claim the document drew a backslash it never
  drew. `\` is escaped too, or a cell whose text is literally `\|` would split at a character the
  document merely printed.
- **Trailing empties stay.** Truncating an empty last row makes a prettier table and a different
  document, and A14 names that erasure specifically.
- **The list marker is always `- `.** The representation carries no `/ListNumbering`, and picking
  an ordered marker without one would be this exporter deciding the document meant a numbered list.
  Where the document drew its own number it drew it as an `/Lbl`, which is source text — so the
  item reads `- 1. First item`. Doubling the marker is ugly; deleting the document's own characters
  to fix it is the erasure.
- **Lists from the tree or not at all.** No bullet glyph, no hanging indent, no font name — the
  refusal L29 makes about font-size headings, one structure level up. An `/Lbl` under `/TOCI` is
  not a list item, and that is pinned.

### The cell-quote golden

S1's golden proved the claim for a paragraph, where the invented bytes are a blank line. A table is
where it gets harder, because a GFM row is *mostly* invented — pipes, spaces, dashes and an escape
wrapped around text the page really drew. `a_quote_from_a_gfm_cell_verifies_end_to_end` runs the
same four binaries against the **pinned Ethos CLI**: a quote copied out of the merged cell's origin
**grounds**, and `North | merged span` comes back **`text_mismatch`**. The reason is pinned, not
just the verdict — `element_not_found` would mean the citation pointed at nothing and the test
would pass without the Anchor Map having demonstrated anything.

### Fixtures

- **`markdown-table-cells`** — a stroked 3×3 whose font declares real ink metrics, so its **cell**
  runs reach `ethos.grounding.v1`. `ruled-table-grid` already has a merge and an empty cell but
  declares no metrics, so a golden against it would watch the verifier find nothing and refuse both
  halves. Carries the merge, a pipe inside a cell string, and a wholly empty last row.
- **`tagged-list-items`** — an `/L` / `/LI` / `/Lbl` / `/LBody` tree with a nested `/L` and one item
  whose body is two marked runs. **Neither corpus tags a list anywhere**, so this is the only
  document that reaches the branch.

Both are in `fixtures/manifest.json`, so the mutation suite covers them without anyone remembering
to add them.

### Unchanged

The three detection rules, the table gate (**still missed at 64‰**), `extract`, the oracle (12/3),
`irs-form-1040-2025` at 0 tables, fabrication 0. Four crates. No HTML, no hyphenation joining, no
MCP, no tag. v1.1-S3 had not started at this point.

---

## v1.1-S0/S1, Safe Markdown, as 0.11.0

**Markdown ships, and only ever with the map that inverts it.** `ethos.markdown.v1` carries a
Markdown string, the **Anchor Map** that binds every source byte of it back to representation
nodes, and a coverage census of every character that did not make it. **v1 is still not done** —
the S7 gate is measured and missed at 64‰ — and v1.1 began because the owner asked for the next
roadmap row, not because the gate cleared. Not tagged.

### Why this took until v1.1, and what changed

`docs/01-CONTRACT.md` §12 has said since v0 that this contract *does not define a Markdown
projection*, on Workbench rule 8: **retrieval operates on the evidence record itself, and any
projection between what is ranked and what is cited is where a locator dies silently.** A pipeline
chunks Markdown, embeds it, ranks it, hands the winner to a model; the model quotes it; the
citation then has to bind back to a run or a cell, and the Markdown threw that away. Nobody
notices, because the quote is real text and the answer looks right. The parity checklist records
the fallback plainly — **O8: rule 8 prefers no projection at all.**

v1.1 does not lift the objection. It pays it. The map is not a companion file somebody can forget:
it is a **field of the same artifact**, and `MarkdownArtifact` will not construct without one that
tiles.

### The four laws, and where each is enforced

1. **Never one without the other.** `markdown` and `anchor_map` are fields of one hashed artifact.
   There is no `--md-only`, and `no_cli_path_emits_markdown_without_its_map` reads the subcommand's
   declared flags — not its prose — to keep it that way.
2. **The map total-tiles the bytes.** `AnchorMap::new` refuses a gap, an overlap, an out-of-order
   pair, an empty segment, an offset past the end, or an offset that splits a UTF-8 character. The
   same check re-runs on parse, so a hand-edited file cannot smuggle a hole past the type. **A map
   with a hole is worse than no map**, because the hole is exactly where an unquotable byte hides.
3. **Two segment kinds, and only two.** `source` names the node ids its bytes came from; `syntax`
   is markup the exporter invented and names nothing. No third value — a "probably source" would
   be the confidence field §9 forbids.
4. **Coverage is a census.** `emitted + dropped == in_representation`, in characters, with every
   dropped character in a **named** bucket carrying a count. Checklist A14.

### The golden, which is the whole point

`a_markdown_quote_verifies_end_to_end` runs the real binaries — `extract`, `ground`, `markdown`,
`verify` — and asserts both directions against the **pinned Ethos CLI**:

| quote | taken from | verifier says |
| --- | --- | --- |
| `First block` | a `source` segment | **grounded** |
| `block\n\nSecond` | spans a `syntax` segment | **mismatch**, reason `text_mismatch` |

The second row is the one that means something. `block\n\nSecond` is **real text in the Markdown
that the page never drew** — from the `.md` alone nothing distinguishes it from a sentence the
document contains. The test pins the *reason* as well as the verdict: `element_not_found` would
mean the citation pointed at nothing and the assertion would pass without the map having
demonstrated anything.

Nothing is re-derived from the report (`docs/07-VERIFY-BOUNDARY.md`); the test reads
`all_evidence_grounded`, which the verifier wrote.

### What projects, and what is a named bucket

Only `text_run` nodes. The rest are dropped **by kind**, because each is a different fact:

| kind | bucket | why |
| --- | --- | --- |
| `form_field` | `form-field-values-not-projected-v1` | a field's `/V` lives in the AcroForm tree and no content stream draws it. v1-S4 exists to keep it distinguishable from page text; inlining it would undo exactly that, silently |
| `annotation` | `annotation-text-not-projected-v1` | a reviewer's note is markup *over* the document |
| `image` | `image-nodes-carry-no-text-v1` | a placement, not a picture — almost always `0`, present so the census is exhaustive by construction |
| — | `whitespace-collapsed-v1` | see below |

**Page artifacts are projected, not dropped.** A running head is a `text_run` carrying
`pdf_artifact`; O21/O22 is explicit that a reader deleting running heads has silently edited the
document. The flag stays in the representation and a consumer that wants them gone drops them
itself, knowing it did.

**Collapsed whitespace is a bucket, not a rounding.** A `source` segment's bytes are the
*normalization* of `node.text` — NFC, trim, collapse internal whitespace to one `U+0020`, the same
three clauses `table-gate-v1.md` publishes. Measuring `in_representation` over that same
normalization would have balanced the census by moving the denominator: a run drawn as
`Hello   world` would report 11 characters in a representation holding 13, and the two missing ones
would be invisible. So the denominator is the **raw** text and the difference gets its own named
class.

**Tables are a declared erasure, not a bucket.** A cell's text is a concatenation of runs that are
already nodes, so projecting the runs loses no character and a `tables-not-projected` count would
read `0` and disclose nothing. What is lost is the *grid*, and that gets a limitation —
`markdown-table-structure-not-projected` — rather than a number that looks like a disclosure while
the thing actually erased has none. v1.1-S2 is where a GFM table earns the right to flatten spans.

### Headings come from the tree, never from a font

`# ` through `###### ` when the node's `pdf_tagged` role path ends in `H` or `H1`…`H6`, after the
document's own `/RoleMap`. **No font size is consulted** — checklist L29 is REFUSE.

Measured: **no fixture in either corpus carries a heading role**, so on everything committed today
this branch never fires and every document projects as paragraphs. It is proved by a unit test over
a hand-built representation rather than by a PDF nobody has, which is the honest way to test a path
the corpus cannot reach.

### Added — `engine markdown`

`engine markdown <representation.json>`, the same input `engine ground` takes. One input kind,
documented, rather than a subcommand that silently means two things. The fingerprint is checked
before anything is projected: a representation that does not hash to its declared digest is refused
with exit 2, because projecting it would launder the disagreement into a fresh-looking artifact
whose map named node ids nobody can now confirm. Exit 0 / 2 only. Double-run byte-identical.

### Changed — the profile grows two fields, and the hash moves

`markdown_rule: "markdown-linear-v1"` and `capabilities.markdown: true`. A profile JSON predating
v1.1-S1 is **refused rather than defaulted** — the posture `table_detection.stroke_ruled` took at
v1-S8: a field defaulted in is a claim the run never made. `parser_version` 0.10.0 → 0.11.0.
Profile hash `sha256:08c4207d…` → `sha256:881474f7…`.

`capabilities.markdown: false` obliges `markdown-not-projected`, which is written as a **position
rather than a gap**: O8 records that rule 8 prefers no projection at all, so a build that emits none
has taken an option the checklist keeps open.

### Added — one engine-owned CC0 fixture, and why it had to exist

`markdown-two-blocks`: two runs in a font declaring real ink metrics, so **both** reach
`ethos.grounding.v1` as elements a verifier can find. Every pre-existing fixture with measurable
ink has exactly one run — no join to span — and the two-run fixtures declare no metrics, so their
`elements` array is empty and the golden would have passed vacuously against a verifier that found
nothing either way. Mutation suite 51 → 52 fixtures; one new `EXPECTED_SURVIVORS` entry,
`junk-after-eof`, the same benign class as every other engine fixture.

### Added — `docs/10-V11-SCOPE.md` and `docs/11-V11-MILESTONES.md`

v1.1-S0. The same shape v1 has in 08/09: what the version is, what it is not, the four laws, the
slice map. **S2 (tables and lists) and S3 (HTML, export-only cosmetics) are named and not
started.** Rule 8 is stated where a later slice will read it, with O8's fallback quoted rather than
paraphrased away.

**652 tests pass**, up from 619. Oracle still 12 / 3.

### What is unchanged, and checked

The table gate still reads **64‰**; `irs-form-1040-2025` still yields **0** tables; fabrication
still **0**; oracle still **12 / 3**. No detector was retuned — `ruled-rects-v2`,
`stroke-ruled-v1` and `unruled-align-v1` keep their ids and their numbers. `engine-pdf` did not
learn Markdown and `engine-grounding` did not grow a Markdown schema; the projection lives in
`engine-core`, which is where a projection of the representation belongs. No fifth crate.

Still open and still listed: page rasters, low-contrast detection, structure-order, undrawn table
edges, and **S7's miss at 64‰**.

---

## [Unreleased] — v1-S8, stroke-ruled tables, as 0.10.0

**A third table-detection rule ships.** `stroke-ruled-v1` reads the grid a document draws as
**ruling lines** — the commonest way a blank form is drawn, and ink this engine threw away from
v1-S1 to v1-S7b. `cfpb-home-loan-toolkit` cell-F1 **246‰ → 259‰**, macro **61‰ → 64‰**. The
0.489 gate is still MISSED and **v1 is still not done.** Not tagged.

### The rule S7b parked, and the one defect that was stopping it

v1-S7b built this rule, measured it in full and refused to ship it: it **regressed the document it
was built for** (246‰ → 210‰) and — worse — it **refused `cfpb-home-loan-toolkit` page 13**, the
8 × 4 loan worksheet the whole lead was named for, while emitting five bands on Closing Disclosure
pages the structure tree tags nothing on. It went to `docs/attic/stroke-ruled-v1/` as a patch.

Both failures were the **same defect**, and neither was in the band preconditions: `extract`
filtered every non-horizontal segment away before the rule saw it. "Where are the columns" was
therefore answered entirely by *where horizontal rules happen to end* — at once too strict (page 13
has one baseline ruling three cells instead of four, because that row's first cell is blank) and
too loose (two unrelated rules ending at the same x imply a boundary nobody drew).

**Step 5 moves off the faces and onto the lines.** Every column line *interior* to a band must be
stroked as vertical ink running the band's full height, collinear segments joined end to end first.
The band's outer edges are exempt — an interior line separates two cells, an outer edge is only
where the ink stops — which is the same trade step 4 already makes on the other axis. Page 13
strokes its three interior column rules at x = 210, 326 and 442 and neither outer one.

This is the precondition `stroke-ruled-tables-not-detected` itself named back at v1-S2: *"every
face bounded by four edges rather than covered by one rectangle."*

| `cfpb-home-loan-toolkit` | parked `-v1` | shipped |
| --- | --- | --- |
| p13, the 8 × 4 worksheet | **refused** | **7 × 4 emitted** |
| p22, both bands (gold tags nothing there) | emitted | **refused** |
| p23, the 3 × 3 against a tagged 5 × 3 | emitted | **refused** |
| p24, the second band (gold tags nothing there) | emitted | **refused** |
| p24 9 × 4, p25 4 × 2 and 8 × 6 | emitted | still emitted — see below |
| p6, p7 | 5 × 2, 7 × 2 | unchanged |
| p11, p16 | dropped | dropped — `ruled-rects-v2` owns those regions |
| cell-F1 | 210‰ | **259‰** |

### `irs-form-1040-2025` stays at 0 tables, on a principle rather than a special case

Under the new step 5 alone the tax form yields **five** tables: its entry boxes really are a
stroked grid with their column rules drawn. That document has been the canary for v1-S1's 662-cell
fabrication since S1, and the parked rule's four tables there were most of why its macro "doubled".

What settles it is not the geometry but **whose rectangle it is**:

| | a face | a widget `/Rect` in it |
| --- | --- | --- |
| `irs-form-1040-2025` p1 | 93.3 … 251.6 × 309 … 321 | **145.0 … 251.2 × 309 … 321** |
| `cfpb-home-loan-toolkit` p13 | 210 … 326 × 334.6 … 388.6 | 231.1 … 321.8 × 349.9 … 376.3 |

On the 1040 the face **is** the field's box, edge for edge — the ink that drew it is the widget's
frame. On page 13, *also* a fillable worksheet carrying 25 widgets, the field sits inset inside a
larger printed cell at less than half its height, and there the printed cell is a cell with a
widget in it. So a face whose four edges are a form field's four edges refuses the band; one is
enough, because a page cannot half-be a form, and it fails closed.

`forms::widget_rects` reads `/Rect` and `/Subtype` only — not through the forms walk, which is
gated on a capability and reads text. A table rule whose answer changed with an unrelated
capability would make one profile's tables silently differ from another's.

**No new constant is introduced by either step, and neither names a document, a page or a count.**

### Before and after, from the same harness

Engine 0.9.0 → 0.10.0, profile `sha256:5593cb1f…07d2d5c` → `sha256:08c4207d…5143c65`:

| | 0.9.0 (S7b) | 0.10.0 (S8) |
| --- | --- | --- |
| tables detected / matched | 8 / 8 | 14 / 13 |
| page-level precision | 1000‰ | 928‰ |
| cells emitted | 36 | 180 |
| **fabricated** | **0** | **0** |
| cross-check disagreements | **0** | **0** |
| `cfpb-home-loan-toolkit` | 24 TP / 12 FP → **246‰** | 44 TP / 136 FP → **259‰** |
| `irs-form-1040-2025` | **0 tables** → 0‰ | **0 tables** → 0‰ |
| `nist-sp-800-63b`, `nist-sp-800-53r5` | 0‰ | 0‰ |
| **MACRO cell-F1** | **61‰** | **64‰** |

**The macro rise is CFPB's alone**, which is the point: the parked rule reached 125‰ by turning the
canary document into four tables, and that is the number this slice declined to take.

### Page 13 comes back a 7 × 4 against a tagged 8 × 4

Eight baselines bound **seven** rows. The header row — `LOAN OFFER 1 / 2 / 3` — has no top edge
drawn anywhere on the page, and step 4 does not supply one; an 8 × 4 would mean writing a
coordinate no operator in the file produced. The metric charges that twice, because every detected
row is then compared against the gold row above it.

The table itself is unmistakable: `Lender name` / `Loan amount` / `Interest rate` /
`Fixed · Adjustable` / `Monthly principal and interest` / `Monthly mortgage insurance` /
`Total Loan Costs`, with `$` and `%` down the three offer columns.

### Changed — `stroke-ruled-tables-not-detected` is retired

It said a grid drawn as bare ruling lines is not detected. That is now false, so the code is
**gone rather than reworded** — the same move v1-S2 made when it retired `unruled-tables-not-detected`,
and for the same reason: a limitation that survives the gap it describes is worse than none,
because a reader acts on it.

**`undrawn-table-edges-not-supplied`** replaces it, and states the consequence a consumer will
actually meet: a table ruled underneath each of its cells is emitted **one row short**, rows
numbered from zero, rather than completed with an edge nobody drew. Two narrower misses sit under
it — rows come from horizontal rules, so a grid ruled only down its columns implies none; and
curves are still never flattened into ruling lines.

### Changed — `profile.table_detection` grows a third field, and the hash moves

`table_detection.stroke_ruled: "stroke-ruled-v1"`, `deny_unknown_fields` intact. A pre-S8 profile
JSON is now **refused** rather than defaulted, so no artifact can claim a rule its run never
executed. `parser_version` 0.9.0 → 0.10.0. Both changes are in the new hash, and two artifacts
either side of it say different things about the same document — page 13's worksheet is a table on
one side and absent on the other, which is exactly what a profile identity is for.

### Added — the refusal wiring the parked patch never had

`stroke-ruled-table-candidate-refused`, document-scoped, grouped by precondition like the ruled
one. The attic's own revival notes called its absence the slice's one real incompleteness: refusals
returned an empty vector and the rule declined bands **in silence**. Standing rule 3. Clippy is
green at `-D warnings` with no `#[allow(dead_code)]`, because the refusal is now used rather than
suppressed.

### Added — three engine-owned CC0 fixtures

The first fixtures anywhere whose grid is drawn as two-point `m`/`l` pairs.

- **`stroke-ruled-worksheet`** — page 13's shape: one baseline ruling three cells not four, three
  interior column rules stroked and neither outer one. Five baselines bound four rows, so it also
  pins the undrawn top edge.
- **`stroke-ruled-columns-not-drawn`** — byte for byte the same horizontal ink, no vertical ink.
  Refused, with the declaration. The Closing Disclosure shape.
- **`stroke-ruled-field-boxes`** — a stroked 2 × 2 whose faces are four widget `/Rect`s. Refused.
  The 1040 principle without the tax form.

The mutation suite grows with them: 48 → 51 fixtures, and three new entries in
`EXPECTED_SURVIVORS`, all `junk-after-eof`. Triaged before pinning rather than pasted in — each
was re-extracted with garbage appended past `%%EOF` and yields exactly the tables it does
unmutated, which is the same benign class every other engine fixture is in.

### Arbitration is now three-way

`ruled-rects-v2` → `stroke-ruled-v1` → `unruled-align-v1`. The first two are both ink the author
put down, so neither outranks the other and a shared region simply goes to whichever got there
first; the alignment rule is an inference *about* the document rather than evidence *in* it and
loses to either. The alignment rule now sees only runs neither of the other two has claimed. Grids
are never merged or averaged.

**619 tests pass**, up from 609. Oracle still 12 / 3.

### What it still gets wrong, reported rather than tuned away

**136 false-positive cell slots against 12**, overwhelmingly three bands: page 24's 9 × 4 and page
25's 4 × 2 and 8 × 6. Their interior column lines *are* stroked across the band, so by every
reading of the ink they are grids the author drew — the structure tree simply tags nothing there.
Every discriminator that would remove them is a threshold fitted to these four documents, which
`docs/08-V1-SCOPE.md` §3 forbids, so the cost is recorded in the precision figure instead.

**Two truncated cells on page 13** — `onthly principal and interest`, `Total Loan Costs (ection D…)`.
The producer split those strings and the leading `M` and `S` are drawn left of the band's outer
column line, so their runs fall outside the table. Cell assignment is by run origin for all three
rules, and widening the band to catch them would invent the outer edge step 4 refuses to invent.
Not fabrication — `fabricated_cells` is 0 and that table's cross-check is `ok` — but a measured
loss, and it is why page 13 matches 18 slots rather than more.

---

## [Unreleased] — v1-S7b, the gate measured at 61‰, as 0.9.0

**The gate is now a cell number, and it is missed: macro cell-F1 is 61‰ against a 489‰ floor.**
Seven investigations ran. **Six were measured and rejected; one shipped** — and the one that worked is in
the *ruled* rule, which nobody was looking at. **v1 is not done.** Not tagged.

### `ruled-rects-v2`: a rectangle cannot be both the border and the evidence

`Lattice::build` required every face of the lattice to be covered by **some** painted rectangle.
A page-background panel answers yes for every face at once. Two hundred lines away in the same
file, `detect_ruled` discarded that same panel rather than emitting it as a cell, with the comment
*"the table's own border"*. One rectangle cannot be both the only evidence a face exists and not a
cell, and the coherence precondition was the half that was wrong.

`cfpb-home-loan-toolkit` pages 22 and 23 each paint a 351 × 454 pt panel — twice — behind scattered
highlight bars:

| | page 22 | page 23 |
| --- | --- | --- |
| tagged | **nothing** | 5 × 3 |
| emitted under `-v1` | **17 × 13 holding 12 cells** | 23 × 8 holding 29 |
| false-positive cell slots | 36 | 43 |

79 of the 91 false positives the gate charged against the ruled rule, and both of the corpus's
cross-check disagreements, came from those two pages.

| | `-v1` | `-v2` |
| --- | --- | --- |
| tables detected | 10 | 8 |
| matched | 9 | 8 |
| **precision** | 900‰ | **1000‰** |
| cells emitted | 77 | 36 |
| fabricated | 0 | 0 |
| cross-check disagreements | 2 | **0** |
| cfpb TP / FP | 24 / 91 | **24** / **12** |
| cfpb cell-F1 | 175‰ | **246‰** |
| **MACRO cell-F1** | 43‰ | **61‰** |

**No true positive is lost and no other detection changes.** Exactly the two junk tables disappear.

### The ruled rule can now say when it refuses, and it refuses a lot

It never could. Every precondition failure in `Lattice::build` returned `None`, `detect_ruled`
collapsed them into an empty vector, and a page whose rectangles implied a grid their own ink did
not draw read exactly like a page that painted nothing.

**And the path was not cold — measured, after an adversarial review challenged the first draft of
this paragraph.** The ruled rule refuses **556 of the four documents' 602 pages**, 481 of
`nist-sp-800-53r5`'s 492 among them, and had done so in silence since v1-S1. What went unnoticed
was narrower: the panel case above produced a *table* instead of a refusal, and finding that is
what made the surrounding silence visible.

`ruled-table-candidate-refused` is the companion to the alignment rule's existing declaration, with
the same discipline: it names the precondition that failed and never grades how close the
rectangles came. Two variants are earned today — a face no rectangle drew, and a lattice past the
cell ceiling.

**Grouped by precondition, so the reasoning is stated once.** At 481 refused pages, repeating a
five-line explanation per page would put roughly a quarter of a megabyte of identical prose inside
a hashed artifact. Every page is still named with its own numbers; only the prose is de-duplicated,
so nothing is truncated. The alignment rule's existing declaration has the same shape and does
repeat itself — a pre-existing defect in that pattern, left alone here rather than fixed under
cover of this change.

### The fixture

`background-panel-not-a-grid`, engine-owned and CC0, 1 175 bytes: a 200 × 160 pt panel painted
**twice** with three 40 × 10 bars on it. Under `-v1` it yields a 7 × 7 table with 3 cells, 46
`Unowned` slots and a `DoesNotTile` fault; under `-v2`, no table and a declared refusal. Verified
to fail without the fix at both the unit and the artifact level.

Two details are deliberate. The panel is painted twice because the real page does, and because it
forecloses a wrong re-fix — "skip the largest rectangle" would pass a single-panel fixture. And it
is the **first engine fixture whose geometry is filled (`f`) rather than stroked (`S`)**: every
other ruled fixture strokes, so the fill path into the lattice had no fixture behind it, which is
part of why this survived six slices.

---

### The six investigations that did not ship

S7b set out to recalibrate the *alignment* rule and measured that every prescribed change does
nothing or worse. Recorded in full below, because the reason they failed is the finding. In the
order they were run: the gutter-constant calibration, band segmentation, mcid merging, mcid merging
with bands, the pages 16/17 and 8/13 investigations, and finally `stroke-ruled-v1` — built as a
complete pass and parked.

**They are written in the order they were investigated, not the order a reader would want.** The
`stroke-ruled-v1` verdict sits above the section that discovered the lead it acts on, because that
is when each was written. Left as-is rather than reordered: the sequence is the record.

### The calibration, before and after

There is no "after". The change S7b was scoped to make was tested and reverted, because the
harness says it changes nothing:

| Experiment | declared | detected | matched | recall | fabricated |
| --- | --- | --- | --- | --- | --- |
| as S7a left it (`COLUMN_GUTTER_MIN` = 1 200) | 57 | 10 | 9 | 157‰ | 0 |
| Column floor disabled (= 151) | 57 | 10 | 9 | **157‰** | 0 |
| Column **and** row floors disabled | 57 | 10 | 9 | **157‰** | 0 |

151 centipoints is below any reachable value: `fold()` guarantees adjacent lines differ by more
than `ALIGN_TOLERANCE` (150), so a floor of 151 disables the check outright. **Every number is
identical.** Moving the constant would have bumped `unruled-align-v1` to `-v2` and moved
`profile_sha256` to buy exactly no behaviour change, so the rule kept its `-v1` id — which is the
honest label for a rule that did not change.

### Why 10.6 pt looked like a cliff and was not

S7a reported that on 490 of `nist-sp-800-53r5`'s 492 pages the refused gutter was 1 062 centipoints
against a 1 200 floor, and read that as a structural gutter wrongly rejected. `gutter_fault` uses
`.find()` — that is the **first** sub-floor gap in sorted order, not the smallest. Measured
distribution of the **minimum** gap across the 600 refused pages:

| min gap | pages |
| --- | --- |
| 151 cp (1.51 pt) | 542 |
| 152–168 cp | 56 |
| 185 cp | 1 |

Every refused page has a sub-2pt adjacent-column gap, and carries 250–280 sub-floor gaps among
200–280 column lines. No floor above 186 cp changes anything; no floor at or below 151 cp exists.

### The actual defect, named and not fixed

**The candidate handed to the alignment rule is the whole page.** `tables::detect` passes every
leftover run to `unruled::detect` as one lattice, and `detect_ruled` builds one lattice from every
rect on the page. With both gutters disabled, all 600 pages reach the face check with lattices like
96 × 231 = 22 176 faces from 1 784 runs — refused by `MAX_FACES` and by coherence. Three
independent preconditions all correctly say *this page is prose*. The gutter floor is not wrong; it
is **unreachable**.

### The other declared leftover, falsified for NIST and 1040

`stroke-ruled-tables-not-detected` was the suspected reason NIST's ruled tables are missed. Census
of axis-aligned two-point stroked segments (all currently rejected; **zero diagonals** anywhere):

| Document | segments | distinct | what they are |
| --- | --- | --- | --- |
| `cfpb-home-loan-toolkit` | 1 380 | 1 375 | real table rulings |
| `irs-form-1040-2025` | 521 | 520 | the form grid |
| `nist-sp-800-63b` | 77 | **2** | 76 copies of one margin rule |
| `nist-sp-800-53r5` | 498 | **9** | 490 copies of one margin rule |

**Neither NIST document draws any table rulings at all.** Its 490 segments are one vertical sidebar
rule at x=39.3 repeated once per page. A stroke-ruled rule would add zero tables there, and on
`irs-form-1040-2025` it would re-open the surface that produced v1-S1's 662-cell fabrication. It
stays a declared limitation rather than a half-enabled rule.

### Segmentation was built, measured and reverted

The obvious repair for the whole-page candidate is to cut the page into bands first.
`unruled-align-v2` was implemented — row lines, then cells cut wherever ≥ 12 pt of whitespace
separates the end of one run from the start of the next, then bands of consecutive rows agreeing on
their cell starts — and measured in two variants.

**A, columns still folded from raw origins:** 11 detected against 10, cell-F1 unchanged at 43‰, and
a new false table on `irs-form-1040-2025`. 358 of 363 bands died on the gutter floor, because a cell
reading `Digital Identity Guidelines` is three word-level runs three points apart.

**B, the band's cell starts become its column lines:**

| Document | detected | TP | FP | cell-F1 |
| --- | --- | --- | --- | --- |
| `cfpb-home-loan-toolkit` | 13 | 28 | 103 | 193‰ |
| `irs-form-1040-2025` | **6** | **0** | **37** | 0‰ |
| `nist-sp-800-63b` | 8 | 2 | 52 | 6‰ |
| `nist-sp-800-53r5` | 114 | 42 | 1 076 | 10‰ |
| **MACRO** | **141** | **72** | **1 268** | **52‰** |

141 tables against 10, 1 302 emitted cells against 77, **+9‰** of gate score, right about 5% of the
time. Reverted — and not for missing 489‰:

- **`unruled-near-miss` becomes a table.** A gold negative turning positive is a fabrication.
- **`irs-form-1040-2025` goes 0 → 6 tables**, 0 correct cells, 37 wrong. That is v1-S1's 662-cell
  failure at a smaller scale, on the document that exists to catch it.
- **Three `unruled` unit tests fail**, including `a_face_without_text_refuses_the_whole_lattice` and
  `the_gutter_floor_is_exact_to_the_quantum`. Making cell starts the column lines routes the gutter
  floor around itself, so the variant removes a fabrication guard rather than merely scoring badly.

`fabricated_cells` stayed **0** throughout, which is worth stating precisely because it is not a
defence: the cells were real text in invented grids. That is the failure the fabrication count
cannot see and the gold negatives can, which is why this slice added them.

The measured obstacle is not a tolerance. These producers emit text word by word, so a cell is *n*
runs, and no per-page geometric rule recovers the author's cell boundaries from that without
inventing them. A rule that merged runs into cells on **evidence** — the structure tree's `/MCID`
grouping, already read — would be a different rule under a different id. Not a calibration, and not
this slice.

### MCID grouping built, measured and reverted

The direction the segmentation post-mortem pointed at: merge runs into units by their
marked-content id before any geometry, so a cell reading `Digital Identity Guidelines` is one unit
rather than three word-level runs. `TextRun.mcid` comes verbatim off `BDC`, so it is the producer's
chunking and not the structure tree. Three variants:

| Variant | tables | emitted cells | fabricated | cell-F1 |
| --- | --- | --- | --- | --- |
| baseline | 10 | 77 | 0 | 43‰ |
| merge alone | 10 | 77 | 0 | **43‰** — every number identical |
| merge across baselines + bands | 121 | 1 194 | **14** | 46‰ |
| merge within one baseline + bands | 104 | 1 086 | 0 | 45‰ |

**Merge alone changes nothing**, and all 217 library tests pass. It compresses 8.4× (median 2 302
runs to 278 units per page) and the lattice is still 74 × 157 = 11 470 faces against a 4 096 cap.
The candidate is the whole page; merging changes what is in it, not how big it is.

**The 14 fabricated cells are the first non-zero fabrication count in this slice**, and the cause is
worth recording: `extract::reorder_page` remaps `DetectedCell::run_indices` after
`gutter-columns-v1` permutes the page, and that remap preserves a cell's text only because a
table's runs stay contiguous. A unit spanning two baselines can be split by the reordering, and the
remapped indices then concatenate to a different string than the detector built. Restricting a unit
to one baseline fixed it, which confirms the diagnosis.

The baseline-scoped variant then failed exactly where segmentation failed before: `unruled-near-miss`
becomes a table, and a near-miss that v1 declared as `FacesWithoutText` now **silently produces
nothing**, because the band filter removes the incoherent candidate before the coherence check sees
it. A lost disclosure is standing rule 3, not a bad score.

**Attribution.** All three gold negatives carry **zero mcids**, so `units()` is a provable no-op on
them: the near-miss break belongs to the bands, not the merge. It also means the gold negatives are
structurally blind to mcid merging and cannot certify it — the only control that exercises it is
`irs-form-1040-2025`, whose 1 976 runs all carry mcids.

**And the gate would have needed republishing first.** An adversarial panel established that
**5 998 of 7 704 gold cells (778‰) cite exactly one mcid**, so a detector merging by mcid would
reproduce those cell texts byte-identically by construction. The grid half stays fully independent,
and handing the detector perfect text *and* a perfect grid still ceilings at macro 446‰ — below the
floor — which is a real argument that the shared signal cannot clear the gate on its own. It is
recorded in `docs/table-gate-v1.md` rather than settled, because no mcid-reading rule shipped.

The same review found a defect in the committed metric: gold joins a cell's mcid texts with a
space, the detector concatenates runs with nothing, and on **210 of 7 704 cells (27‰)** that
changes the gold text. It biases the gate **down**, so it is conservative. Documented, not
corrected — fixing it would move the labelled set and the published number in the commit that
reports them.

### `stroke-ruled-v1` built, measured in full, and not shipped

The lead above, taken as a complete slice. Two-point axis-aligned segments captured as their own
evidence — a `PathSegment` beside `PathRect`, because a box says *this cell is here* and a line says
*this edge is here* — then rule-rows by baseline, a row must tile end to end, a band is consecutive
rows whose endpoints are all column lines of the first, rows are the regions BETWEEN rules so no
edge is invented, coherence requires every face's own bottom edge drawn, 2 x 2 minimum and the
shared face cap.

|                          | ruled-rects-v2      | + stroke-ruled-v1       |
| ---                      | ---                 | ---                     |
| detected / matched       | 8 / 8               | 21 / 15                 |
| precision                | 1000‰               | 714‰                    |
| cells emitted            | 36                  | 272                     |
| fabricated               | 0                   | 0                       |
| cross-check disagreements| 0                   | 0                       |
| cfpb-home-loan-toolkit   | 24 TP / 12 FP 246‰  | 41 TP / 189 FP **210‰** |
| irs-form-1040-2025       | 0 TP 0‰             | 12 TP / 30 FP **292‰**  |
| MACRO                    | **61‰**             | **125‰**                |

**The lead was real**: 17 cfpb cells and 12 1040 cells no rule had ever found, NIST and all three gold negatives at zero.
**Correction to that first claim as written:** page 13's worksheet is *refused* by the rule's own
coherence step — one of its eight baselines rules three cells instead of four — so the table the
slice existed to find is not among the nine it emits. The exact hit is page 7, 7 x 2. 8 x 4 was the
shape of the INK, measured before the emission path existed.

**Not shipped**, for three reasons in this order:

1. It **regresses the document it was built for**, 246‰ to 210‰ — 17 more right cells bought with
   177 more wrong ones. Of the false positives, 156 are text appearing in NO gold cell at all: not
   an index artifact but genuine over-detection, from page furniture and the Closing Disclosure
   pages.
2. **Four canary tests fail.** irs-form-1040-2025 yields 4 tables where every slice since v1-S1 has
   held it at 0. That guard exists because of the 662-cell fabrication and its message says
   flipping it "is a deliberate decision needing its own evidence". **The decision was taken and it
   is to keep 1040 at zero tables.**
3. **The macro gain comes entirely from the canary document.** 1040 going 0‰ to 292‰ is what
   doubles the average while cfpb, where the rule genuinely helps, gets worse. A metric that
   rewards flipping the guard should not be what decides to flip it.

Fabrication stayed 0 and the cross-check clean throughout, so this is over-detection rather than
invention — a real distinction and not a defence, since 156 cells of real text in regions nobody
tags as tables is exactly what the gold negatives exist to catch.

The band preconditions are sound and reproduce; the problem is entirely in which regions become
bands, and every remaining tightening was a threshold fitted to these four documents. The lead does
not go away: 103 cfpb cells, 65% of that document's gold, are still behind ink the engine discards.

The rule is **parked, not discarded**: `docs/attic/stroke-ruled-v1/` holds it as a patch that
applies to this commit, with its measurement, its reason, and the two questions a future attempt
should settle before writing any code — the 1040 one, and the undrawn first row.
**No source file moved. S8 remains unstarted.**

### The second tables on pages 8 and 13, and the lead they uncovered

Page 8's is layout: a 2 × 3 `YOUR CHOICE Check one:` checkbox block, two of six cells empty, drawn
with scattered underlines and no grid. Tagged as a table, and not one anyone would extract.

**Page 13's is real, and the document draws it.** An 8 × 4 loan-comparison worksheet, stroked as
32 horizontal rules in a perfect grid — 8 baselines, 4 segments each at identical column
boundaries (x = 54, 210, 326, 442, 558), with one baseline carrying three segments to match the
gold's blank cell. Exactly the tagged shape, in ink, discarded because a two-point stroked segment
is not a rectangle.

**That makes `stroke-ruled-tables-not-detected` the largest remaining lead on this corpus, and an
earlier entry's framing of it as "falsified" was too broad.** Falsified for NIST (490 copies of one
margin rule) and for `irs-form-1040-2025` (the fabrication canary) — never assessed for
`cfpb-home-loan-toolkit`. Checked now: **every one of the nine missed tagged tables sits on a page
that strokes segments**, 103 cells in total, **65% of that document's gold**.

| | cfpb cell-F1 | macro |
| --- | --- | --- |
| today | 246‰ | 61‰ |
| if page 13 alone were found | 493‰ | 123‰ | *(the built rule refuses page 13 — see above)*
| if all 103 were found | 852‰ | 213‰ |

Still short of the 489‰ macro floor — NIST and 1040 stay at zero and carry half the average between
them — but it is the only measured lead left that moves the number. **It was then attempted** — see
the `stroke-ruled-v1` section above, which is the entry for the complete measured pass this
paragraph was asking for. The projections in the table are the ones that pass did not reach.

### Pages 16 and 17 investigated: no defect, and a property of the metric

The last remaining ruled lead. **Neither page is a detector defect**, and they are not the same
shape — an earlier note in `docs/table-gate-v1.md` had both wrong. Page 17 paints two full-height
column panels (165 × 214 and 339 × 214 pt) with its five text rows unpainted inside them; page 16
paints a single **61 pt** two-cell band behind one row of a seven-row table.

The ruled rule reconstructs exactly what was painted, a 1 × 2 grid, and the artifact already
reports the disagreement in full — `RowCountDiffers { tagged: 7, detected: 1 }` plus every one of
the 12 slots the tags have and the geometry does not. That is `tagged-vs-geometric-v1` doing what
v1-S1 built it for.

**The gate cannot see any of that.** It charges the two pages 4 false positives and 24 false
negatives, and three of the four "false positives" are text the detector extracted exactly right —
p16 rows matching gold row 5, p17 matching gold row 2. A partial detection numbers its own rows
from zero, so a correct row 5 is compared against gold row 0 and charged twice. Crediting a match
at any row offset would take cfpb from 246‰ to roughly 287‰; it is **not** done, because a join
loosened until the detector scores better is the failure `08-V1-SCOPE.md` §3 exists to prevent.
Recorded as a property of the metric, not a fault of the engine.

### The finding that reframed all five, and produced the sixth

**Every table the gate scores is a ruled one. The alignment rule emits zero tables on the entire
gate corpus** — before and after every change tried against it. The whole number is the *ruled*
detector on the rulings CFPB actually paints.

So five repairs were spent tuning a rule that contributes nothing to the number judging them. That
was not unreasonable — the alignment rule is the only candidate for the three documents that draw
no rulings — but the gate never exercised the code being changed. **Asking the other question is
what produced the one repair that worked**: the ruled rule was getting 24 of CFPB's 159 cells right
while emitting 77, and the excess was two junk tables from a single self-contradiction. See the top
of this entry.

Two facts also bound what any alignment repair could have achieved. The gold declares **no spans at
all** — all 7 704 cells are 1 × 1 — and coherence forbids an empty cell, so the unruled family can
never emit any of the 1 234 empty gold cells and a full-lattice emission charges every miss to both
FP and FN. Grant a perfect grid and free text wherever one marked-content unit supplies it, and F1
collapses to TP/G: 72/159, 11/40, 328/568, 3352/6937, **macro 446‰ — below the 489‰ floor.**

### The gate, which is the part S7a left open

S7a stored `{page, rows, columns, cells}` and could measure page agreement and nothing finer. The
cells are now in the labelled set, with their text, and the gate is a cell score.

**Cell text comes from the tree, never from the detector.** `TaggedCell` gains the `/MCID`s the
tree cites beneath it and the page they are on; those join against runs by `(page, mcid)` — v1-S3's
own key. No coordinate is read, so the gold stays independent of the geometry it scores. The
labelled set grows from 57 shape records to **7 704 cells, 6 470 of which carry text**, and is
frozen by the same re-derive-and-compare guard.

```
cell-slot accuracy (the gate metric), exact text after the whitespace rule:
  document                           TP       FP       FN    cell-F1
  cfpb-home-loan-toolkit.pdf         24       12      135      246‰
  irs-form-1040-2025.pdf              0        0       40        0‰
  nist-sp-800-63b.pdf                 0        0      568        0‰
  nist-sp-800-53r5.pdf                0        0     6937        0‰
  MACRO cell-F1 over the 4 documents that declare a table: 61‰
  gate is > 489‰: MISS
```

Macro-averaged F1 over `CellSlot`s, integer per-mille, exact text after NFC + trim + whitespace
collapse, joined by page then greedily by shape. The full method — corpus, formula, join, text
rule, and why this number is **not** comparable to the published 0.489 — is
`docs/table-gate-v1.md`, added in this commit alongside the first cell number.

A fourth finding, about the measurement rather than the detector: several NIST tagged tables are
**multi-page** (one is 278 rows, another 245), and a page-granular join cannot match those to a
per-page detection even in principle. Recorded rather than fixed by dropping them — deleting the
documents that score badly is the failure this harness exists to prevent.

### Kept honest

- **Fabrication is still 0** on all four documents, measured on 36 emitted cells.
- **The gold negatives still have none.** `synthetic/two-columns`, `synthetic/simple-text` and
  `unruled-near-miss` yield 0 geometric tables, asserted for the first time in this harness.
  `two-columns` is the one that matters: four runs in a flawless 2 × 2 whose only distinguishing
  evidence is that the author wrote it down the columns.
- **`reading_order::COLUMN_GUTTER_MIN` was not touched.** It holds 1 200 for its own reasons and
  moving it to help tables is forbidden.
- No bake-off table. No README comparison row. The leftovers are still leftovers: page rasters,
  low-contrast, structure-order, and stroke-ruled tables.

### Identity

`profile_sha256` moves from `sha256:2e07326e…089ae042` to
**`sha256:5593cb1fa7d5e9bb252e9f57643eb0f2d8062902bd7096d0eaca1e26007d2d5c`**, carrying both the
version and `table_detection.ruled` moving to `ruled-rects-v2`. Two artifacts either side disagree
about whether `cfpb-home-loan-toolkit` pages 22 and 23 hold a table, which is exactly the
disagreement a rule id exists to make legible.

`TABLE_DETECTION_V1` is **kept**, exactly as spelled, because artifacts produced before this name
it — the same reason `READING_ORDER_RULE_V0` survived v1-S5 — and `TABLE_DETECTION_V2` joins it on
the public surface.

`unicode-normalization` is added as a **dev**-dependency for the metric's NFC clause;
`cfpb-home-loan-toolkit` draws curly quotes, so NFC is load-bearing rather than decorative. The
harness is `cfg(test)`, so nothing enters the shipped library's graph.

**609 tests pass**, up from 601. Oracle still 12 / 3.

### v1 status

**S7b is done. S7 is not.** The gate is measured and missed at 61‰. `docs/09-V1-MILESTONES.md` S7
stays open with the number written down, and the README claims nothing it has not measured.

---

## v1-S7a, the labelled set and the harness

**A measuring slice. It changes no detector and improves no number.** It builds the instrument,
points it at the corpus, and reports what it sees. What it sees is bad. **Not tagged.**

### The measurement

```
document                     declared detected  matched     recall  precision
cfpb-home-loan-toolkit.pdf         17       10        9       529‰      900‰
irs-form-1040-2025.pdf              1        0        0         0‰         -
nist-sp-800-63b.pdf                13        0        0         0‰         -
nist-sp-800-53r5.pdf               26        0        0         0‰         -
TOTAL                              57       10        9       157‰      900‰
cells emitted 77, fabricated 0, cross-check disagreements 2
```

Table-cell accuracy is not "below 0.489". On three of the four real documents **no cell is produced
at all**, so a cell score has no denominator. Recall against the documents' own declarations is
**157‰**.

### Where the labels come from, and why they are not this engine's

From each document's **own tagged structure tree**. A `/Table` element is the producer's declaration
that a table is there and what shape it has; v1-S3 already reads them, and a guard test keeps every
geometric type out of that module. So the ground truth is the author's, the thing measured is the
geometric detector, and the two derivations are independent **by construction** rather than by
anyone remembering to keep them apart. That is what S3 built, and this is the slice that needed it.

No hand-labelling, no judgement of mine, and no network.

**A tagged `/Table` is a claim by the producer, not verified truth.** Producers tag tables for
layout as well as for data, so some of the 57 are almost certainly not tables anyone would want
extracted — which inflates the denominator and makes recall read worse than it is. Every label
records its provenance as `pdf-struct-tree` rather than presenting itself as fact. Narrowing the set
by sampling is a later slice's work, not something to do quietly.

**What the labels must never become is labels derived from what the detector found.** Recall measured
against your own output is 1.0 by construction. A guard scans the labelling function for that shape
— scoped to `label` alone, after an earlier draft fired on `page.tables` inside `score`, which is
the detector's output legitimately being measured.

### Committed and frozen

`fixtures/labelled/table-truth.json`: 57 tables, 7 704 cells, across the four real documents.
Derived once and frozen; `the_committed_labels_still_match_the_documents` re-derives and compares,
so a change in what the tree-walk thinks these documents declare is a reviewed edit rather than a
number that moved underneath the measurement. Regeneration is an `#[ignore]`d test, run
deliberately.

### Fabrication is 0, measured rather than asserted

The v1 exit criterion that is not a threshold. Every emitted cell's text is a concatenation of runs
the page actually drew — **77 of 77** across the real corpus. Until now that claim was checked only
on the four hand-built grids the fixtures provide.

### Why the gate is not assessed here

0.489 is a table-cell score from a third-party published corpus this repository does not have. A
number computed on a different corpus with a different evaluator is not comparable to it, and
`06-STEAL-REFUSE.md` records exactly what happens when people pretend otherwise: two publishers
scored the same tool at 0.000 and 0.693 on tables, differing only by invocation flags. The gate is
assessed at S7, with its method stated, or it is not stated. No bake-off table appears anywhere in
this repository.

### What S7b will act on, and why it waits

**Every** table-candidate refusal across all four documents is `unruled::COLUMN_GUTTER_MIN`, and on
490 of `nist-sp-800-53r5`'s 492 pages the refused gutter is **1 062 centipoints — 10.6 pt — against
a 12 pt floor**. That floor was sized to reject word spacing (*"at 12pt type a space is 3–4pt"*);
10.6 pt is a structural gutter by the rule's own reasoning.

It is not changed here on purpose. Loosening a tolerance to admit more tables is the shortest path
to fabricating them — v1-S1 produced a 662-cell table on `irs-form-1040-2025` from a detector that
looked right by inspection. With this harness the trade is visible: precision and fabrication are
measured on the same run as recall.

### Version unchanged, deliberately

**No `parser_version` bump and no profile move.** The profile is every knob that can change output,
and this slice changes none: no artifact byte differs. Bumping would produce two profile hashes
whose artifacts are byte-identical, which weakens what the hash means. A measuring instrument is not
a knob.

**601 tests pass**, up from 595.

---

## [Unreleased] — v1-S6.2, as 0.8.2

**A repair.** The engine was putting rectangles on the wire around content that draws nothing — and
because some of those rectangles left the page, **two of the three real benchmark documents could
not be read at all**. **Not tagged.**

### The failure

`nist-sp-800-53r5` and `nist-sp-800-63b` produced no artifact, exiting 2 on
`DocumentRepresentation::seal`'s `check_box_within_page`. On 53r5 that was **491 of its 492 pages**.

### What the boxes were, measured across every offender

| Document | out-of-page boxes | whitespace-only | with visible text |
| --- | --- | --- | --- |
| `nist-sp-800-53r5` | 3 450 | **3 450** | **0** |
| `nist-sp-800-63b` | 2 | **2** | **0** |

Every one is a run of spaces at a **one-point** font size, carried past the right edge by its
advance — a producer idiom for trailing whitespace. **No run with visible text is out of place
anywhere.** The coordinate transform was never wrong. The seal was refusing two documents over
rectangles drawn around nothing.

### Root cause

`Font::ink_box` builds its rectangle from the font's ascent/descent envelope stretched over the
run's **advance** — not from glyph outlines, as `FontInk`'s own doc comment always said. For a run
of spaces that is a box around nothing, labelled `Measured`.

### The variant the contract had already promised

`GeometryPresence`'s documentation explains it is deliberately not an `Option` because `None` would
collapse *"we could not measure"*, ***"there is nothing to measure"*** and *"we were not asked to
measure"* into one answer. There was a spelling for the first and the third. There was **none for
the second** — so a run of spaces got a rectangle and was called measured.

`GeometryAbsence::NoInkToMeasure` fills it. Whitespace-only runs get it, and so do zero-advance
runs, which are the same fact. It does **not** count toward the ink-measurement limitation: that one
means the reader could not measure, and here the reader could measure perfectly well and there was
nothing there.

### The count was conflating the same two things one layer up

`geometry-absent-not-groundable` reported their sum under a sentence that read as though the reader
had failed every time — on 63b, *"11 663 nodes have no measurable ink box"* when **242** were
failures and **11 421** were non-events. It now reports both, separately, and says which one is a
limitation of this reader.

### Not changed, deliberately

`check_box_within_page` **stays a hard refusal.** It is a working bug detector and it earned its
keep in this very session — it is what caught v1-S6's crop-box regression, where page dimensions
came from one box while coordinates came from another. Softening it into a limitation would have let
that ship silently. After this repair, a *visible* run outside its page means the transform really
is wrong, and that is worth failing loudly over.

### Blast radius

**150 425** nodes across the four real documents lose a box that was never ink. **Zero conformance
fixtures change** — not one contains a whitespace run that claims a box, which is why nothing caught
this. Grounding projections shrink by exactly those nodes, which is the point: a citation anchored
to a rectangle around three spaces was never evidence.

### The fixture the corpus lacked

`whitespace-past-the-page-edge`: 28 spaces at 12pt whose advance runs from x=200 to x=368 on a 300pt
page, in a font declaring real ink metrics, beside a visible run that must keep its box. On 0.8.1 it
exits 2 with *"node `s1` has a measured box [20000, 3538, 36800, 4648] outside its page [0, 0,
30000, 14400]"* — the same failure as the NIST documents, in 1 462 bytes.

### Leftover, named

**What `Measured` actually means.** The box is a font-envelope approximation for *every* run, not
just whitespace — a capital `T` and a lowercase `o` get identical heights. Saying so properly needs
glyph outlines and touches `01-CONTRACT.md`, the meaning of `bbox` in `ethos.grounding.v1`, and S1's
locator cross-check. Not scheduled. What this slice fixed is the case that is not an approximation
but a fiction.

**Unverified, and stated rather than implied:** that a space glyph never draws ink in these fonts is
reasoned from the semantics of whitespace and from the box being an advance rectangle rather than an
outline. Confirming it needs the glyph outlines this slice does not read.

### Identity

`profile_sha256` moves from `sha256:50d846c9…21dc967` to
**`sha256:2e07326e31e5bf6eedc2ecfb2a7ec4249516ea9c07e770e803a0d852089ae042`** — the version alone.
No field changed and no capability moved, but which nodes have geometry did, and the hash has to
carry that.

**595 tests pass**, up from 592. Oracle still 12 / 3; the conformance corpus's geometry is unchanged,
asserted explicitly. 47 fixtures, up from 46.

---

## [Unreleased] — v1-S6.1, as 0.8.1

**A repair.** A simple font's character codes were being read two bytes at a time, so text was lost
on every real document in the corpus — and the artifact blamed the document for it. **Not tagged.**

### What was wrong

`Font::split_codes` took the code width from **whichever decoder the font got**:

```rust
let width = match &self.decoder {
    Decoder::ToUnicode(t) => t.code_bytes().max(1),   // the /ToUnicode codespace
    Decoder::Simple(_)    => 1,
};
```

`/ToUnicode` wins whenever a document ships one, so a **simple** font declaring a `<0000><FFFF>`
codespace had its single-byte codes fused in pairs. PDF 32000-1 §9.6 is unambiguous: a simple
font's codes are always one byte, and `/ToUnicode` maps codes to Unicode — it has no say in how a
string is split. The doc comment one line above already said *"One byte per code for simple
fonts"*. The code did not do it, because `/Subtype` was parsed and then never reached the decision.

### What it cost, measured

Font instances split wrongly: **2 426** in `nist-sp-800-53r5`, **303** in `nist-sp-800-63b`, **98**
in `cfpb-home-loan-toolkit`, **0** in `irs-form-1040-2025`, and **0 across all nine conformance
synthetics** — every one of which is a Type1 with no `/ToUnicode` and therefore took the correct
path. That distribution is the whole story: the shape that breaks appears in no owned fixture and
in every real document.

On `cfpb-home-loan-toolkit`, **8 417 of 28 783 text runs were missing** from the artifact. What
survived was visibly damaged — `"You’rtartinoooortgag"` where the page reads *"You're starting to
look for a mortgage"* — while runs in the same document's CID fonts were perfect on the same page.
The fault tracked the font's kind, not the document.

### The dishonest half, which is worse

Those 8 417 runs **were** declared — as `broken-font-encoding`: *"A font on this document has an
incomplete or damaged encoding."* That was a **false statement about a conformant file**. The fonts
were fine; the reader was fusing codes that were never in the document, and then reporting the
document as damaged.

A declaration that misattributes is worse than no declaration, because a reader acts on it — someone
would have gone to fix a document with nothing wrong with it. The code now reports only what can be
established: a code arrived and this profile had no character for it. Whether the cause is a damaged
font, an encoding this profile does not vendor, or a defect in this reader is **not decided there**.

### Fixed

`FontKind`, read from the document's own `/Subtype` and carried on `Font`. A simple font is one byte
per code, always. Anything unrecognised — including a font declaring no `/Subtype` — is simple,
which is the conservative reading and what this reader did before `/ToUnicode` support existed.

`declared-font-codes-v1` is on the profile, because this decides **what the text says**: two
artifacts either side of it disagree about a document's content, which is exactly the disagreement a
rule id exists to make legible.

### Declared rather than left to be found

Composite (`/Type0`) fonts still take their width from the `/ToUnicode` codespace, because nothing
here parses `/Encoding` CMaps. That agrees with `Identity-H` — what real documents overwhelmingly
use — and is unverified for anything else, so it is now declared as
`composite-font-codes-from-tounicode` wherever it applies. Doing Type0 properly means parsing CMaps
including mixed-width codespaces; that is a slice of its own, touching 142 font instances that are
not currently broken.

### The fixture the corpus never had

`simple-font-two-byte-tounicode`: a TrueType carrying a `/ToUnicode` whose codespace declares two
bytes. It reads `"Hi there"`; under the old rule the codes fuse, none is in the map, and the run is
dropped entirely. It fails on 0.8.0 and passes here.

Authoring it required an explicit `struct_tree` flag in the fixture generator, replacing a heuristic
that spliced `/StructTreeRoot 6 0 R` into any fixture with an extra object. That heuristic had
mis-fired **twice already** — an image XObject at v1-S6, and now a `/ToUnicode` CMap — and it turned
out to have been mis-firing all along: **`annotation-contents` has been carrying a `/StructTreeRoot`
pointing at its own annotation dictionary**. That fixture is re-pinned, now declares no structure
tree, and reports `untagged-structure-tree-absent` as it always should have.

### Unchanged, and tested to be

Every conformance golden decodes exactly as before — `simple-text`, `two-lines`, `two-columns`,
`hyphenated-line-break`, `ligature-fi-embedded-font` all asserted explicitly. They never took the
broken path, so a repair that moved them would have been fixing something else.

### Known, and not fixed here

`nist-sp-800-53r5` and `nist-sp-800-63b` still exit 2 on `check_box_within_page` — a measured ink
box outside its page. **Verified pre-existing**: both fail identically at 0.8.0 and at every earlier
version. Two of the three real benchmark documents therefore produce no artifact at all, which needs
its own slice and blocks nothing here.

### Identity

`profile_sha256` moves from `sha256:3de478c9…d53d5ace` to
**`sha256:50d846c99379099c40a3fee91cccdee09bc909d5e30139e82413f6e9021dc967`** — the version and the
new `text_code_rule`.

**592 tests pass**, up from 587. Oracle partition still 12 / 3; `two-columns` still column-major;
the 1040 still 0 tables and its widgets still never runs. 46 fixtures, up from 45.

---

## [Unreleased] — v1-S6, as 0.8.0

The sixth slice of v1 (`docs/09-V1-MILESTONES.md`): **the rest of v1's observational surface** —
images as located, fingerprinted nodes; hidden and off-page text reported as findings; an annotated
overlay. **Not tagged.** v1 is six slices of seven — S7 is not started.

**Page screenshots are NOT in this release**, and that is a decision rather than an omission. See
below.

### Measured before anything was written

Three questions, answered against the code rather than assumed:

1. **Is `Tr = 3` invisible text dropped today?** **No — emitted, and unflagged.** `Tr` was parsed
   into the text state from v0 onward and read by *nothing*: a workspace grep for `render_mode`
   returned the declaration, the default and the assignment, and no comparison against 3 anywhere.
   So this slice is **additive, not a repair**: the text was never lost, and half of checklist
   O21's exit criterion ("the run stays in the representation") was already satisfied. The defect
   was **silent mixing** — an OCR layer, a hidden instruction and visible prose all arrived at a
   consumer identical.
2. **Does `Do` fail closed on an image XObject?** **No — a deliberate no-op**, with the operand
   never read, so a `Do` with a missing name and a `Do` drawing a photograph were the same event.
3. **What does `failure/image-only-or-blank-page` classify as?** `no-text` — and it turns out
   **the fixture contains no image at all**. All 431 bytes of it are an empty content stream and an
   empty `/Resources`; it is the blank half of "image-only or blank page". `no-text` is the correct
   answer, and a test now pins it, because this is exactly the slice where somebody reads the name,
   expects an image, and "fixes" the classifier into fabricating one.

Also measured: **no fixture in either owned corpus contains an image XObject** — 22 engine-owned,
15 conformance, all zero — so every fixture this slice needed had to be authored.

### Added — `page-observations-v1`

One rule id over one pass, because images and findings read the same graphics state: the current
transformation matrix places an image, and the text rendering mode and the visible page box decide
what a run is flagged with.

**Images.** `Do` is now interpreted for `/Subtype /Image`. Each placement becomes a
`NodeKind::Image` carrying page, object number, the rectangle the `Do` painted into, a sha256 over
the stream **as stored**, its `/Filter` chain, its declared pixel dimensions and whether it is a
stencil mask.

- **The rect is the matrix, not the pixel count.** A PDF image is defined on the unit square and
  the CTM decides where it lands, so the area is a measurement of the document's own matrix. The
  golden fixture is 2×2 samples painted into 120×60 points precisely so the two numbers cannot be
  confused; `/Width` and `/Height` ride separately as `pixel_width`/`pixel_height`.
- **The digest covers encoded bytes.** Decoding first would make the fingerprint depend on this
  engine's inflate implementation, and two readers disagreeing about what one file contains is what
  a fingerprint exists to deny. A digest rather than a payload, because an artifact is a record
  *about* a document, not a second copy of it.
- **A media type only where the bytes really are a file.** `/DCTDecode` is `image/jpeg` and
  `/JPXDecode` is `image/jp2`; everything else — Flate, LZW, CCITT, JBIG2, unfiltered — is
  `pdf_encoded_samples`, because saving those bytes to a `.png` produces a file nothing can open.
  Nothing is sniffed from the payload.
- **No description, caption or alt text, ever under this profile.** Reading pixels is OCR, which v1
  does not do; a model's account of a picture is not evidence (checklist O20). A source-scan test
  bans the field names.

**Findings.** `TextFinding::{InvisibleRenderMode, OffPage}` on the run, plus a document-scoped count
in `assurance.limitations`. **The run stays** — same text, same origin, same place in reading order.
OpenDataLoader deletes low-contrast text and returns a page that looks clean, so its caller cannot
tell a scrubbed document from an innocent one; that is the defect O21 names.

`invisible-render-mode` covers `Tr 3` and `Tr 7`. Modes 4–6 paint and are deliberately not flagged.
The engine does **not** decide what it is looking at: the same mode carries a scanner's OCR layer,
which is ordinary, and a prompt hidden behind an image, which is not, and nothing in the content
stream tells them apart.

### Added — the annotated overlay, and `engine overlay`

A deterministic lopdf copy of the document with `/Square` annotations over table boxes, image
placements and flagged runs, plus a per-page `/Text` note. **The note is the part that satisfies
O10**, whose exit criterion is that the overlay distinguishes present geometry from typed absence:
it counts the nodes on that page with *no* rectangle to draw — a run whose font supplies no ink
metrics, an image placed by a non-axis-aligned matrix — so a partly-read page cannot look fully
read. Its own rectangle is the page corner and says so, because there is no honest place to anchor
a marker for content whose position is what is unknown.

**It annotates and never edits.** No content stream is touched, no text is removed, and the
document's own annotations are kept. This is not `--sanitize`, and a source-scan test bans the
operations that would make it one. Byte identity holds per fresh build — lopdf's writer *mutates*
the document it saves, so the overlay clones per call and never saves a cached document twice.

Zero new dependencies: `lopdf` already writes, its objects live in a `BTreeMap`, it generates no
`/ID`, and its only clock is behind a feature this build does not enable.

### Fixed — the page-box origin was discarded

`to_top_left` used the box's **width and height** and threw away its origin, so a page whose
`/MediaBox` is `[0 20 612 812]` — legal, and not unusual — had every coordinate in the artifact
shifted by 20 points, with the top of the page landing at `y = -20`.

**Not one document in either corpus declares a box whose origin is other than `(0, 0)`**, measured
across all 67 PDFs available to this repository, which is why it survived six slices. On such a page
the subtraction is the identity, so **no existing golden moves**.

It had to be fixed before an off-page finding could exist at all: a bounds test against a frame the
content is systematically offset from reports ordinary text at the top of a page as off-page, and a
**fabricated** security finding is worse than no finding. `off-page-and-offset-box` is the fixture
the corpus lacked. `/Rotate` inheritance and its indirect-reference and real-number forms are
handled for the same reason — a wrongly-unrotated page puts every coordinate somewhere else.

`/CropBox` is now read as the **visible** box, clipped to the media box. Off-page is measured
against it, because measuring against `/MediaBox` on a page that crops would report ordinary trimmed
content as off-page. All three benchmark documents declare a `/CropBox`; all three declare one equal
to their media box, so this changes nothing on the current corpus and everything on a document that
actually crops.

### Fixed — a two-frame page, caught by probing this slice's own change

v1-S6 briefly took a page's declared width and height from the **`/CropBox`** while every
coordinate stayed in the **`/MediaBox`** frame. Two frames on one page — and
`DocumentRepresentation::seal` refuses an artifact whose measured box falls outside its declared
page, so **a document that crops stopped producing an artifact at all**, exiting 2 with *"the
measurement or the coordinate transform is wrong"*. It was right.

Nothing caught it: no document in either corpus crops (all three benchmark PDFs declare a
`/CropBox` equal to their `/MediaBox`), and every other engine fixture supplies no ink metrics, so
no measured box existed anywhere that could fall outside a page. Found by building a probe page by
hand — `/MediaBox [0 0 300 200]`, `/CropBox [50 50 250 150]`, real font metrics, text in the
cropped-away margin — which is now the `crop-box-smaller-than-media` fixture.

Page dimensions come from the media box, which is the frame coordinates are expressed in. The crop
box is still read and is still what an off-page finding is measured against; that is a different
question with its own answer on the run.

### Fixed — a silent skip, caught by its own symptom

`Sha256Hex::parse(sha256_hex_bytes(…))` returns `Err` for every input: the first produces bare hex,
the second requires the `sha256:` prefix. Written with `.ok()?` at a `let … else { continue }`, it
made **every image node vanish** with no error anywhere — an empty array that looked exactly like an
honest "found none". `Sha256Hex::of_bytes` is infallible by construction, because hashing cannot
fail and the fallible spelling was never describing a real possibility.

### Not shipped, and why

- **Page screenshots** (`page-raster-not-emitted`, `page_screenshots: false`). Rendering a page
  means a PDF renderer — glyph rasterization, shadings, blend modes, image filters. PDFium is
  admitted only caller-provided under an explicit ADR (`00-NORTH-STAR.md` #14) and this repository
  has no ADR convention to write one in; no AGPL renderer clears `deny.toml`'s allowlist; and
  shelling out to `pdftoppm` would put an unpinned binary between the document and the artifact.
  A half-built rasterizer would be worse than the gap.

  **`raster_dpi` is on the profile anyway**, carrying `{"mode":"not_emitted"}` — a declared state
  rather than an absent field, so a renderer arriving later moves `profile_sha256` instead of
  silently changing what an artifact means.
- **Low-contrast text** (`low-contrast-not-detected`). Not possible without new machinery: the
  twelve colour operators are recognised and discarded, `/ExtGState` is never resolved so alpha is
  unreachable, and the graphics state carries no colour slot. Doing it needs a colour-space model
  plus a contrast **threshold** — the same shape as the vowel-frequency test this project already
  refuses for `garbled`, and a wrong one would flag ordinary light-grey body text as hidden.
- **Inline images** (`inline-images-not-emitted`) and **unresolved `Do` names**
  (`xobject-name-unresolved`). Both counted and declared rather than skipped. An inline image has no
  object number and no separate stream, so it cannot be a node; an unresolved name is a bounded
  malformation, and refusing the document over it would turn files that read today into failures.

### Changed

- `capabilities.images` false → **true**, with a both-halves proof: a page that paints an image
  yields a node, a page that only declares one yields none. `capabilities.page_screenshots` arrives
  as an explicit `false`.
- `non-text-nodes-not-projected` reworded: it said "form fields or annotations", which became false
  the moment an image node existed. `ethos.grounding.v1` carries one kind of box and it means
  measured ink; an image's painted rectangle is a **third** provenance, and flattening it in is what
  v1-S4 refused for declared rectangles.
- Classify is **untouched**. It counts image XObjects a page's `/Resources` declare; extraction
  emits a node per `Do` that paints one. `image-declared-not-drawn` pins both answers at once —
  `embedded-images` from classify, zero image nodes from extract — and neither is wrong.
- `engine-pdf` gained an `overlay` and an `images` module, both `pub(crate)`; the CLI stays a thin
  shell. New exports: `build_overlay`, `OVERLAY_ARTIFACT_TYPE`, `ImageRecord`, and in `engine-core`
  `TextFinding`, `PdfImageLocator`, `PaintedRect`, `ImageAttributes`, `ImageMediaType`, `RasterDpi`,
  `OBSERVATION_RULE_V1`.

### Identity

`profile_sha256` moves from `sha256:1131244222…0de113` to
**`sha256:3de478c92c536b7ed10999be655515ce70bf37f2b5aec5031614145ad53d5ace`** — the version, the new
`observation_rule` and `raster_dpi` fields, and two capability flips.

**587 tests pass**, up from 573. Oracle partition still 12 / 3; `two-columns` still column-major and
still not a table; the 1040 still yields 0 tables and its widgets are still never runs; `fmt`,
`clippy -D warnings`, `cargo deny check` and both grep gates are clean. The fixture manifest declares
45 fixtures, up from 40.

---

## [Unreleased] — v1-S5, as 0.7.0

The fifth slice of v1 (`docs/09-V1-MILESTONES.md`): **reading order becomes a rule instead of a
declared limitation.** `synthetic/two-columns` reads column-major. v1 is five slices of seven — S6
and S7 are not started.

**This is the first change that reorders evidence rather than adding it.** Two artifacts either
side of it can list the same runs, with the same text and the same origins, in a different
sequence — and a consumer that concatenated them would get two different documents. That is why
the rule id is a profile field, why it took a new name rather than a version bump, and why
`profile_sha256` moves.

### Added — `gutter-columns-v1`

Runs are ordered by **whitespace in page space**. A vertical band no run's horizontal extent
crosses, at least 12pt wide, cuts a page into column bands read left to right; inside a band the
same sweep runs horizontally to order blocks top to bottom, and each block may split into columns
again. Every coordinate is an `i64` in centipoints; no float enters the decision.

**Nothing counts lines.** pdf-inspector flips multi-column on `min_lines < 15`, so fourteen lines
come out row-interleaved and fifteen come out column-major and a one-line edit reorders a whole
page (`docs/03-V0-SCOPE.md` §3.2). Two engine-owned CC0 fixtures sit on either side of exactly
that boundary — `two-column-14-lines` and `two-column-15-lines`, the same page differing by one
line in the left column — and read identically. A port of the line-count rule fails on that pair
and nowhere else in the corpus.

**A new id, not a bump of `single-column-v1`.** That string still means what it always meant —
content-stream order, nothing reordered — and a profile that turns the capability off still uses
it. `READING_ORDER_RULE_V0` stays exported and stays spelled the same; `READING_ORDER_RULE_V1`
joins it on the frozen surface.

### Added — the guard that decides the rule

Adjacent bands must overlap **vertically** over at least half the height of the shorter one, and
the overlap must be strictly positive. Columns run beside each other; a heading above an indented
list does not, and an x-axis sweep alone reads the two identically. The cost is stated rather than
hidden: a two-column page whose first column holds a single line is **not** reordered, because one
baseline has no height and that picture is also what a deep indent looks like.

A run whose font supplies no advance gets a declared minimum extent of 3pt — a floor, never a
measurement, and never derived from a font size. The floor makes cuts *more* likely, not fewer,
which is said plainly in the source: the overlap guard is what carries the decision, not the width.

### Changed — one order, and only one

The run array **is** the reading order. `ordinal` is its index and the span ids are laid over it,
so `s1` is the first run a human should read rather than the first the content stream drew. There
is no parallel reading-order index: O4's defect was id order ≠ array order, and a second sequence
would have reproduced it under a new name. `DetectedCell::run_indices` are remapped through the
permutation — the one failure here that no artifact would otherwise show.

Because the span counter is independent of the table and annotation counters, re-laying the same
contiguous ids over the permuted list is *identical* to having allocated them after the reorder,
without moving ids that have nothing to do with reading order.

### Changed — `synthetic/two-columns`'s golden, reversed in the open

Through v1-S4 that fixture asserted `Right top, Right bottom, Left top, Left bottom` and its test
called the result *"visibly wrong reading order, and honest about it."* It now asserts
`Left top, Left bottom, Right top, Right bottom`. The honesty is replaced by a rule, and the
replacement is announced: a different `reading_order_rule`, a different `profile_sha256`, and a
test that derives the expected sequence from the **origins** rather than from the strings "Left"
and "Right", so a fixture whose labels stopped matching its geometry could not quietly pass.

`two-columns` is still **not** a table. Column-major is `Left top, Left bottom, Right top, Right
bottom`; row-major would be `Left top, Right top, Left bottom, Right bottom`. Those are different
sequences, the emission is still not row-major, `unruled-align-v1` still refuses, and the refusal
is still declared. A test asserts the row-major sequence is *not* what comes out, so "fixing"
two-columns by turning it into a 2×2 grid fails the build.

### Changed — the limitation retires the way S2's did

`multi-column-reading-order` — *a multi-column document is read in the WRONG ORDER* — is **gone**
from the default profile rather than reworded, because a stale limitation is acted on. It is kept
in the vocabulary for a profile that turns the capability off, where the sentence is still true;
that is the S3 lesson, where `structural-locators-not-claimed` survived its own slice for the same
reason.

What replaced it is narrower and partners a **true** capability, as
`stroke-ruled-tables-not-detected` does for tables: `reading-order-geometric-only` says the order
is built from whitespace and nothing else, so column structure carried only by a tag tree is not
seen, and a run with no advance is judged against a floor.

### Two defects found by measuring, not by reasoning

Both were caught by running the rule over `cfpb-home-loan-toolkit`, a real two-column booklet, and
comparing column-majorness against stream order page by page.

1. **The sweep's sort leaked into the answer.** Groups were returned in the order the sweep had
   sorted them into, so a block the recursion then declined to cut came back ordered by baseline —
   a global y-then-x sort of the page, reached sideways, and precisely what S5 decision 10 forbids.
   On that booklet it turned pages that were *already* column-major in the content stream into
   line-by-line row-major reading, which is worse than doing nothing. A cut now decides two things
   only: how atoms are grouped, and what order the groups go in. What order the atoms inside a
   group go in is content-stream order.
2. **The horizontal cut was too eager.** Cutting at every gap at once slices a two-column region
   into one block per line, and emitting those top to bottom is row-major reading arrived at from
   the other direction. It now takes only its widest gap — which peels off whatever full-width
   heading was hiding the gutter and hands each half back to the vertical cut — with ties cut
   together, so evenly-set body text separates in one step rather than one recursion per line.

After both: on that document 7 pages became more column-major, 18 were untouched, and 1 shifted by
a single block boundary where a table's runs are now gathered together. `irs-form-1040-2025` is
**not reordered on either page** — a dense form has no page-height gutter, and the rule does
nothing where it has no evidence.

### Unchanged, and tested to be

- **Tables are atoms.** A run inside an accepted table box belongs to one indivisible object
  holding content-stream order, so a cut cannot shred a grid into fake columns of cell fragments.
  S1's and S2's goldens are unchanged, every cell's text still concatenates from the runs the cell
  names, and fabrication is still 0.
- **Single-column pages are byte-identical** apart from the version and the profile hash.
  `two-lines`, `simple-text`, `list-items`, `heading-export` and `hyphenated-line-break` all come
  out in exactly the order they came out at v0 — the rule finds no gutter and returns the identity.
- **Classify is untouched.** The sorter reads origins; it is not gated on a classification, and no
  `multi-column` layout reason was added. That entry stays in `thresholds::NOT_DETECTED` with its
  reasoning rewritten: reading a two-column page correctly is a different claim from reporting that
  a page is two-column, and routing extract policy through classify is what S2 refused.
- **`/Annots` order is untouched.** Form fields and annotations stay in the order the author wrote
  the array, after all of a page's text, exactly as S4 left them. Interleaving widgets into the
  text order by `/Rect` would mix two sources.
- **`structure.rs` still contains no sort**, and its guard test still says so. `/K` order is a
  different rule over different evidence.
- **The 1040** still yields 0 tables and its 199-widget neighbourhood; form and annotation strings
  are still never `TextRun`s.

### Leftover, named

**Structure-order reading.** A document whose column structure exists only in its tag tree is not
reordered. That would be a different rule with its own id and its own fixture, and it is not
scheduled — not S6, not S7.

### Identity

`profile_sha256` moves from `sha256:95bd8b68…8bd87` to
**`sha256:1131244222e618442352e40da58cb6231b11f4b7140db7421671a258700de113`** — the version bump,
`reading_order_rule` moving to `gutter-columns-v1`, and
`capabilities.multi_column_reading_order` flipping false → true. Artifacts from before and after
are correctly non-comparable, and for this slice that is load-bearing rather than bookkeeping.

**573 tests pass**, up from 554. Oracle partition still 12 / 3; double-run byte identity holds;
`fmt`, `clippy -D warnings`, `cargo deny check` and both grep gates are clean. The fixture manifest
declares 40 fixtures, up from 38.

---

## [Unreleased] — v1-S4, as 0.6.0

The fourth slice of v1 (`docs/09-V1-MILESTONES.md`): a form field's value and an annotation's
comment become **nodes of their own kind**, and neither is ever page text. **Not tagged.** v1 is
four slices of seven — S5 through S7 are not started.

### Measured before anything was written

Extraction reads `get_page_content(page)` and nothing else, so annotation and widget strings were
**never** reaching `TextRun`. The LiteParse defect this slice guards against (checklist L13) did
not exist here, which makes S4 purely additive rather than a repair. Recorded because the opposite
finding would have changed what the slice was.

### Added — `form-annotations-v1`

Each page's annotation list, read in the order the document wrote it. A widget resolves **up** its
parent chain — bounded at 16, cycles declared — gathering the inheritable name, type, value and
flags; everything else becomes an annotation carrying its subtype, `/NM`, `/T` and flags.

**Walked from the page, not from the form**, and that is not an implementation detail. A field
dictionary names no page; its widget does, by sitting in that page's annotation list. Walking that
way gives every node a real page parent, produces one node per widget rather than a field plus a
clone, and makes an orphan detectable as a field no page walk reached — three properties that
would otherwise each need their own machinery.

Flag bits this profile has no name for are **kept** as raw bit positions. A flag nobody named is
still something the document said, and dropping it would make an unread flag indistinguishable
from an unset one.

### Added — two node kinds, and the rule that permitted them

v1-S1 refused `TableCell` because a cell's text is already a run. v1-S3 refused `Paragraph`
because the role path already says `P`. The standing rule from both is *do not add a kind for a
fact an existing node already carries* — and `FormField` and `Annotation` are exactly the case it
was waiting for: no content stream draws them, no run holds them, and without kinds of their own
they are simply absent from the record.

`NodeAttributes` became an externally-tagged union at the same time, because a run's character
codes and a field's value do not belong on one struct. Its tag duplicates `Node::kind`, which is a
**checked** redundancy: `NodeAttributes::kind()` returns the kind and a test asserts every node
agrees with its own attributes. Redundancy that is tested is a cross-check; redundancy that is not
is two places for the truth to live.

### Added — `NativeLocator::PdfObject`, because a fake origin is worse than a new variant

An annotation has no baseline, no advance and no character origin. Filling `PdfLocator` with a
plausible origin would put a coordinate on the wire that the document does not contain, so the
union gained a variant carrying page, object number and the declared rectangle — which is what
`docs/01-CONTRACT.md` §5.1 makes it a union for.

`AnnotationRect` is deliberately **not** `GeometryPresence`: that type means *measured ink*, and a
`/Rect` is a number the author wrote. Mixing declared rectangles into the same field as measured
ones, with nothing on the wire to tell them apart, is the flattening this project refuses
everywhere else. A missing or unusable rect is typed-absent — never a page-sized box.

### Added — capabilities `form_fields` and `annotations`

Two flags rather than one, because they are separately provable and separately absent: a document
can carry comments and no form, or a form and no comments, and one flag would be true on the
strength of either. Each has a proof covering **both halves** — found where they exist, absent
where they do not, with the flag true either way.

`/OBJR` now binds (decision 7). v1-S3 walked object references and bound nothing because no node
existed to bind them to; a field or annotation the structure tree cites now carries the role path
the tree gives it. No role is invented where the tree is silent.

### Declared rather than repaired, dropped, or guessed

- **`form-field-parent-unresolved`** — a widget naming a parent the file does not contain.
  LiteParse repairs orphaned widgets in memory and always flattens; this emits the widget with
  what it declares about *itself*, says the link was broken, and a test asserts the source bytes
  are byte-identical afterwards. The visible consequence — a field name shorter than the form
  intends — is the honest one.
- **`xfa-forms-not-extracted`** — a dynamic-form packet, detected and never parsed (checklist
  L15). Static fields beside it still read, which is why this is a limitation and not a refusal.
- **`non-text-nodes-not-projected`** — nodes `ethos.grounding.v1` has nowhere to put, counted
  **separately** from `geometry-absent-not-groundable`. Those two answer different questions: "no
  ink box could be measured" is a gap in what was read, "not text at all" is a gap in what the
  target schema can express. One number for both would make it impossible to tell a document whose
  fonts carry no metrics from one that simply has a form on it.
- **A hidden annotation stays a node.** `/F` bit 2 asks a viewer not to draw it; honouring that by
  deleting content would be an undeclared edit with nothing to say it happened (checklist O21).
- **A blank field reports `Absent`, not `""`.** "Left unfilled" and "filled in with nothing" are
  different facts about a form.
- **A checkbox value stays a name.** `/Off` and `/Yes` are not converted to booleans: those are
  the common spellings, not the only legal ones, and `true`/`false` would be this engine's reading
  rather than the document's text.

### `irs-form-1040-2025`, measured

199 widgets, 126 `Tx` + 73 `Btn`, **0 tables still**. Its fields cannot become an alignment
lattice because `unruled-align-v1` clusters *run* origins and a field is not a run — structural
exclusion rather than a threshold that happens to reject them, which is the stronger guarantee and
is now pinned by a test. Its 126 text fields report `Absent`; its 73 buttons carry `/Off`. It also
declares an XFA packet.

### Changed — `profile_sha256` moves, and both schemas bump

New profile field `form_annotation_rule`, plus the two capabilities. The default profile hash is
now `sha256:95bd8b68e99b684e476c7d47e8dd8b00da2acdaf0f3e5d57cd631d5f46d8bd87`.
`REPRESENTATION_SCHEMA_VERSION` 0.3.0 → 0.4.0 and `EXTRACT_SCHEMA_VERSION` 0.2.0 → 0.3.0: `Node`
and `PageExtract` both gained fields on `deny_unknown_fields` types, so these are breaking reads.

### Two defects caught by existing guards, both worth naming

- **The architecture boundary test rejected `acroform` in `engine-core`.** It was right:
  `docs/04-ARCHITECTURE.md` §1 says that crate learns no format concept. The rule id and profile
  field are now `form-annotations-v1` / `form_annotation_rule` — named for what the rule *does*,
  with the format-specific walk in `engine-pdf`. Same split `table_detection` already used: a
  generic field holding `"ruled-rects-v1"`. The capability limitation prose was reworded off
  `/AcroForm` and `/Annots` for the same reason.
- **The oracle caught an artifact that could not read its own output.**
  `unrecognized_flag_bits` had `skip_serializing_if` without `default`, so the key was omitted on
  write and required on read. A `Vec` is not an `Option`, which serde treats as optional on its
  own. Found on the first real form the oracle ran.

### Fixtures

Four engine-owned CC0 additions (`engine_owned` 16 → 20). Every one's value or comment is a string
**no `Tj` on its page draws**, which is the whole test: a reader that copied dictionary text into
the text layer would be visibly caught.

`form-field-value` (a field beside a printed label), `annotation-contents` (a comment plus a
hidden annotation), `form-orphan-widget`, `form-xfa-stub`. `make_fixtures.py` gained
`catalog_extra` and `page_extra` so a fixture can splice its own `/AcroForm` and `/Annots`.

### API

`engine_core::{NodeAttributes, FormFieldAttributes, AnnotationAttributes, FieldValue,
PdfObjectLocator, AnnotationRect, FORM_ANNOTATION_RULE_V1}` added to the frozen surface and to
`docs/PUBLIC-API.md`. `IdKind` gained `FormField` (`f`) and `Annotation` (`a`). The walk stays
`pub(crate)`.

**554 tests pass**, up from 540. Oracle partition still 12 / 3; S1's, S2's and S3's goldens
unchanged; `two-columns` still reads `single-column-v1` in the same order.

---

## [Unreleased] — v1-S3, as 0.5.0

The third slice of v1 (`docs/09-V1-MILESTONES.md`): the document's **tagged-structure tree**, read
and bound to text, so a node can carry the role path its author gave it. **Not tagged.** v1 is
three slices of seven — S4 through S7 are not started.

### Added — `struct-tree-v1`, reading `/StructTreeRoot`

v0 copied `/MCID` off `BDC` and could say nothing about what it meant: an id resolved against
nothing. This walks the catalog's structure tree — `/K` over arrays, references, structure
elements, `/MCR` marked-content references and bare integers, with `/Pg` inherited down the tree
and an `/MCR`'s own `/Pg` winning over it — and supplies the other half of the join.

**The join is exact equality on `(page object, mcid)`.** Nothing fuzzy, nothing nearest-match: an
mcid means one thing on one page, and a looser join would file text under a heading that does not
claim it. A test swaps in an id the tree never cites and asserts it binds to neither neighbour.

`/RoleMap` is read as data. A document that maps its own `/Para` onto `/P` has told us what it
means, so the mapping is applied — and an unmapped custom type is emitted as itself. Deciding
`/Odd` "must mean" `/P` because it sits where a paragraph would is the inference this refuses.

**Fail closed on a tree that does not terminate.** `/K` cycles and nesting past 64 levels are
refused by name with no artifact. Walking a cycle does not terminate; stopping partway would
report a structure the document does not have.

### Added — four structural-locator states, because they are four different facts

`StructuralLocator` gained two variants beside `PdfMcid`:

| State | What happened |
| --- | --- |
| `pdf_tagged` | the tree cites this `(page, mcid)` — the author placed this text here |
| `pdf_mcid` | the stream gave an id and **no structure element claims it** |
| `pdf_artifact` | the page marked this as furniture, deliberately outside the tree |
| absent | the page marked nothing here |

Collapsing any two loses something real. `PdfMcid` now means something narrower and more useful
than it did: an id that resolved against nothing, counted as the gap it is.

**`pdf_artifact` runs stay in `nodes`.** A reader that deletes running heads and folios has
silently edited the document (parity checklist O21/O22), and the edit is undetectable downstream.
The content stream's tag is now kept alongside its id — v0 discarded it, which left `/Artifact`
indistinguishable from `/P` and the only options "call furniture body text" or "delete it".

`PdfTaggedLocator.role_path` is the raw `/S` names, **never laundered**, with
`standard_role_path` present only when `/RoleMap` actually remapped something — so its presence is
the signal that a custom type was in play.

### Added — `tagged-vs-geometric-v1`, a second check rather than a wider first one

Where the tree describes a `/Table` and a detector found a table on the same page, the two grids
are compared and the result rides on `TableRecord.tagged_check`.

**A new check id, deliberately.** `geometric-vs-structural-v1` compares a table's own indices
against its own boxes; this compares the document's tags against a detector's grid. One id meaning
both would leave a reader unable to tell which pair of derivations disagreed.

The halves share no input — the tree walk reads no box, and neither detector reads a `/S` — and a
guard test asserts it by name, in the shape v1-S2 fixed `SlotCover`'s guard into. Disagreements
are typed (`RowCountDiffers`, `ColumnCountDiffers`, `SlotOnlyInTagged`, `SlotOnlyInDetected`),
never scored, and **nothing is repaired**: the hostile fixture's detector still reports its 2×2
while the tree's claim of a third row sits beside it.

Decision 7 resolved as **diagnostic-only**. No third `tagged-struct-v1` detector was added: a
tagged table's cells are already addressable through the role paths on its runs, so a table whose
cells this engine positioned would add reach that nothing lacked. Where the tree describes a table
no detector found, `tagged-table-without-geometric-table` says so and no table is emitted.

### Changed — `capabilities.structural_locators` is true, and `profile_sha256` moves

v0 through v1-S2 left it false and said exactly why: an `mcid` with the tree unread is not an
address. The tree is read now, so the claim this flag makes — *this profile looks* — is true.

Its proof test covers **both halves**: a tagged document whose runs come back with the roles its
own tree gives them, and an untagged one that gains nothing. A test that only checked the first
could be satisfied by an engine that invents roles; one that only checked the second, by an engine
that never looks.

New profile field `struct_tree_rule`. The default profile hash is now
`sha256:8cc7607fc8e0203e8e64192fbbb2ac7689bf46d1c6a1c8d4d02be53c4de2ed22`, moved by the version
bump, the new rule id and the capability flip together. `REPRESENTATION_SCHEMA_VERSION` 0.2.0 →
0.3.0 and `EXTRACT_SCHEMA_VERSION` 0.1.0 → 0.2.0: both shapes gained fields and both types are
`deny_unknown_fields`, so these are breaking reads rather than additive ones.

`structural-locators-not-claimed` is **not** removed — it is the partner of a `false` capability
and still fires for a profile that turns the feature off. What replaced it for the default profile
is four *conditional* document-scoped codes, each present only where it is true:
`untagged-structure-tree-absent`, `structure-mcid-unbound`, `structure-item-without-content`, and
`mcid-property-list-by-name` (a `BDC` whose property list indirects through `/Properties`, which
this profile does not resolve — an **unread** id is not an **absent** one).

### Non-regressions

S3 attaches addresses; it changes no earlier slice's answer, and a test asserts all of it at once:

- **No reordering.** The walk produces a lookup keyed by `(page, mcid)`; the node list stays in
  content-stream order. `synthetic/two-columns` still reads `single-column-v1` in the same order.
  Emitting nodes in `/K` order is a reading-order rule and belongs to S5 — a guard test asserts the
  module contains no sort.
- **No new `NodeKind`s.** A role path on the existing `TextRun` carries the answer; `Paragraph`
  and `TableCell` variants would put one fact in two places, which is the reasoning S1 used when it
  refused to emit cells as nodes.
- **No using tags to fix a detector.** `irs-form-1040-2025` still yields 0 tables and
  `unruled-near-miss` is still a near miss. S2's golden is still a 3×2 `unruled-align-v1` table and
  `ruled-table-grid` is still ruled.
- Oracle partition still 12 / 3.

### Fixtures

Five engine-owned CC0 additions (`engine_owned` 11 → 16):
`tagged-structure-roles` (one run in each of the four locator states, including the artifact and
the unmarked one), `tagged-rolemap`, `tagged-table-agrees`, `tagged-table-disagrees` (the tree
claims a third row whose content the page never wrote), and `tagged-cycle`. `make_fixtures.py`
grew an `extra_objects` parameter that numbers structure objects from 6 and adds
`/StructTreeRoot 6 0 R` to the catalog; it refuses to combine that with a `/FontDescriptor`, which
also claims object 6.

### API

`engine_core::{PdfTaggedLocator, PdfArtifactLocator, TaggedGridCheck, TaggedGridStatus,
TaggedGridFault, STRUCT_TREE_RULE_V1, TAGGED_GRID_CHECK_V1}` added to the frozen surface and to
`docs/PUBLIC-API.md`. The tree walk stays `pub(crate)`.

`ethos.grounding.v1` is unchanged — it is `additionalProperties: false` and byte-pinned against
Ethos's own schema, so role paths stay on the representation and never enter the projection.

---

## [Unreleased] — v1-S2, as 0.4.0

The second slice of v1 (`docs/09-V1-MILESTONES.md`): tables a document implies by **alignment**
rather than by painted rectangles, under their own rule id, plus a per-table record of which rule
fired. **Not tagged.** v1 is two slices of seven — S3 through S7 are not started, and the > 0.489
gate is S7's.

### Added — `unruled-align-v1`, a second detection rule

A grid inferred from where a document placed text. Pinned as
`engine_core::TABLE_DETECTION_UNRULED_V1`, and **a separate id rather than a bump of
`ruled-rects-v1`**: one means *the author drew this grid* and the other means *a detector decided
this was one*, and an artifact that could not tell them apart would be flattening the stronger
claim into the weaker.

The rule, in full — origins folded into lines within 150 centipoints; a gutter floor of 1 200
centipoints between column lines and 600 between row lines; at least 2 × 2; **every lattice face
must contain a run origin**; the runs must arrive in row-major face order; a 4 096-face cap. Cells
are always span 1 and never empty, cell boxes are the lattice faces so the grid tiles, and the
table's box is the origins' extent plus a declared 300-centipoint padding — never a font size.

`synthetic/table-regular-grid` is now the golden it always was: six `Tm`/`Tj` pairs, zero path
operators, and a 3 × 2 table with `Name`/`Score`/`Alpha`/`10`/`Beta`/`12` in the right cells,
cross-check `ok`.

### Added — every table says which rule found it

`TableRecord.detection_rule` — `ruled-rects-v1` or `unruled-align-v1`, **per table**, because one
document can hold both kinds and `derivation` is `Computed` for both. A new engine-owned fixture
`both-table-rules` carries one grid of each and the artifact records both ids.

Ruled wins where both rules could describe one region (`ruled-wins-shared-region`): unruled
detection runs only on runs no accepted ruled table already claims, and an overlapping unruled
table is dropped rather than emitted alongside. The two grids are never averaged — that would
produce a grid neither rule found, under a rule id describing neither.

### Changed — `profile.table_detection` is a structure, and `profile_sha256` moves

Was `"ruled-rects-v1"`; is now `{"ruled": "ruled-rects-v1", "unruled": "unruled-align-v1"}`. With
two rules running, one string had to mean two things: a reader could not tell "looked for unruled
tables and found none" from "never looked", which is the same empty-array-versus-absent-key
distinction the contract draws everywhere else.

A plain struct with `deny_unknown_fields`, **not** an internally-tagged enum — v0.1 measured that
internally-tagged representations buffer through a map and drop keys they do not recognise, so a
profile carrying a third rule id would deserialize with it discarded and re-hash to a different
digest than it arrived with.

The default profile hash is now
`sha256:b24fc93984ef60916037c59553474e42eaf9339922e7c22af19cdc9856f11f82`, moved by the version
bump and the new shape together. Artifacts from before and after are correctly non-comparable:
one looked for ruled grids only, the other also inferred grids from alignment.

### Removed — `unruled-tables-not-detected`

Deleted rather than reworded. It said a table laid out by alignment alone is not found, which
stopped being true the moment this slice shipped, and a limitation that outlives the gap it
describes is worse than none because a reader acts on it.

Two narrower declarations replace it:

- **`stroke-ruled-tables-not-detected`** (profile-scoped). A grid stroked as bare ruling lines
  still fails the ruled rule's coverage precondition. Genuinely true of every document this build
  reads — see the thin-lines note below.
- **`unruled-table-candidate-refused`** (document-scoped, **conditional**). Present only where the
  alignment rule built a candidate and refused it, naming the page and the precondition that
  failed. A page below 2 × 2 never had a candidate and declares nothing: a near-miss disclosure
  riding on every document in existence would carry no information, which is exactly what was
  wrong with the code it replaced.

### Non-regressions, measured

- **`irs-form-1040-2025` still yields 0 tables.** Alignment repeats S1's 662-cell mistake with a
  different input if unguarded: this form's text implies **23 276** lattice faces on page 1 from
  1 146 runs and **10 848** on page 2 from 830. The coherence precondition and the cap both refuse
  it, and the artifact now says it looked and refused. A test pins no 75 × 45 cell and no lattice.
- **The S1 ruled goldens are byte-identical** to their v1-S1 output apart from the new
  `detection_rule` field — verified by building `5662f24` in a worktree and diffing, not assumed.
- **`synthetic/two-columns` still reads `single-column-v1`** with its multi-column limitation, and
  now emits no table. It is four runs in a flawless 2 × 2 and geometrically indistinguishable from
  a two-row table; what distinguishes it is in the file, and the rule reads it (below).
- **Oracle partition still 12 / 3**, including the now-populated `table-regular-grid`.
- Cross-check independence, fabrication 0, and double-run byte identity all still hold, with the
  fabrication property widened to cover unruled tables.

### Two defects found while building this, both by tests

- **Growing column groups until a gutter appears chains.** Origins each inside the next one's
  gutter collapse into a single "column" nothing aligns to; measured, two lines of word-split
  prose came out as a 2 × 3 table. `unruled-align-v1` folds within a tolerance instead, which
  cannot chain past it. The cost is declared: a cell whose text was `Tj`-split a few points wide
  opens a column and the lattice is refused, so that table is missed rather than fabricated.
- **v1-S1's cross-check independence guard was vacuous.** It scanned "every line before the first
  `#[cfg(test)]`", and that attribute sits on a `use` at the top of `tables.rs` — so it read one
  line (`use serde::…`) and could not fail whatever leaked into `SlotCover`. It now scans the
  structural types by name and asserts it reached them. No defect was hiding behind it; the guard
  simply was not guarding.

### Thin ruling lines: declared, not attempted (decision 8b)

`irs-form-1040-2025` carries **520 axis-aligned stroked segments** alongside the 396 rectangles
that produced S1's 662-cell fabrication. Admitting 520 more edges to that lattice is the same
experiment with more input, so a stroked-line rule needs its own closed-face coherence — every
face bounded by four edges — and its own measurement pass against that form. That is a slice of
work rather than a widening of this one, and until it happens the gap is named by
`stroke-ruled-tables-not-detected`.

### Fixtures

Three engine-owned CC0 additions (`fixtures/engine/`, `engine_owned` 8 → 11):

- **`unruled-near-miss`** — three rows, two columns, and one value five points off the column the
  other two share. Past the fold tolerance and under the gutter floor, so: no table, plus a named
  refusal. Rounding 205 back to 200 would move a coordinate a reader had no way to know was moved.
- **`both-table-rules`** — a painted grid and an aligned-text grid on one page.
- **`ruled-wins-shared-region`** — a painted grid whose text is also a clean alignment grid.

### API

`engine_core::{TableDetection, TABLE_DETECTION_UNRULED_V1}` added to the frozen surface and to
`docs/PUBLIC-API.md`. The alignment detector itself stays `pub(crate)`: its tolerances and
`Refusal` vocabulary move whenever the rule version does, and a caller pinned to them would be
pinned to a version of the rule rather than to the contract.

`ethos.grounding.v1` is unchanged — it is `additionalProperties: false` and byte-pinned against
Ethos's own schema, so the projection stays a move of cells, boxes and text. The rule id lives on
the representation only.

---

## [Unreleased] — v1-S1, as 0.3.0

The first slice of v1 (`docs/09-V1-MILESTONES.md`): vector-path capture, ruled tables from those
paths, `CellSlot` occupancy, and the geometric-versus-structural locator cross-check. **Not
tagged.** v1 is one slice of seven — S2 through S7 are listed as not started, and the > 0.489 gate
is S7's, not this one's.

### Added — v1 has an implementation map (S0)

`docs/08-V1-SCOPE.md` and `docs/09-V1-MILESTONES.md`, written **before** the detector so the
unruled half could not drift into this slice by accident. They record what v1 is, what it is not,
why 0.489 is measured once at S7 rather than chased at every slice, and the ruled/unruled split:
a ruling line is evidence the author drew, an alignment cluster is an inference a detector made,
and those deserve different derivation classes and different tests.

### Added — vector path capture

`re` and axis-aligned `m`/`l`/`h` are interpreted rather than acknowledged-and-skipped. Only
**painted** subpaths are captured: a path ended with `n`, or used as a clip, drew no ink and is not
a ruling line — otherwise every document that clips to its margins would contain a table.

Three things are deliberately refused rather than approximated, each with a test:

- **Bézier curves are never flattened.** Tessellating one into straight edges would manufacture
  ruling lines for a table the document drew with curves.
- **A diagonal never becomes a rectangle.** The bounding box of a triangle is three edges nobody
  drew.
- **A rotated or skewed CTM drops the rectangle.** Its image is a parallelogram, and boxing it
  would invent four edges.

### Added — ruled tables, `CellSlot`, and the cross-check

`engine_core::tables` carries the occupancy model: `TableCellPosition { row, column, rowspan,
colspan, table_id }`, zero-based, span 1 meaning *not merged*, and `CellSlot` enumerating every
slot a merged cell owns. Addressing cells by array index with an implied span of 1 is the shipped
Ethos ODL-adapter defect (memo §16), and its real cost is not cosmetic: with spans discarded there
is nothing left for a cross-check to check.

**The cross-check's two halves share no input.** `SlotCover` derives occupancy from indices and
spans alone and never sees a box; the geometric half derives containment, overlap and tiling from
boxes alone and never sees an index. A test asserts no geometric identifier reaches `SlotCover`,
because a check whose halves came from one source would agree with itself. The result rides on the
**artifact**, not in `--diagnostics`: a mismatch changes whether a cell is trustworthy, which is a
statement the artifact makes rather than an observation about the run. Status is a typed vocabulary
— `ok` / `mismatch` / `not_applicable` — never a score.

### The two findings that shaped the slice

**`synthetic/table-regular-grid` has no path operators at all.** Its 3×2 grid is laid out by text
position — six `Tm`/`Tj` pairs, zero `re` — and no fixture in the Ethos conformance corpus draws a
single rule. So the ruled detector cannot be demonstrated on that corpus, and v1-S1 authors
`fixtures/engine/ruled-table-grid` (3×3, one merged cell, one deliberately empty cell) as the
golden. The fixture with "table" in its name is an **S2** fixture, and until S2 it correctly
reports `tables: []` plus a limitation. Measured, not assumed — decision #4 asked for exactly that.

**A first version of the detector fabricated a 662-cell table on a tax form.** Building one lattice
from every rectangle on a page means a document that merely *contains* boxes becomes a grid:
`irs-form-1040-2025` produced two tables, one with a cell spanning 75 rows by 45 columns, and Ethos
rejected the artifact outright. The fix is a **coherence precondition** — every lattice face must
be covered by a rectangle the document painted — plus a cap on lattice size. The form now yields
zero tables and validates again; the golden is unchanged. Overlaps are deliberately *not* excluded
by that precondition: an overlap is a real disagreement and belongs in the cross-check, where it is
reported, rather than in a precondition, where it would be silently dropped.

### Changed — `capabilities.tables` is true, and what that claims is narrow

`tables: true` means **this profile looked**. It does not mean every table is found.
`ethos.grounding.v1` already encodes that distinction — absent key means did not look, empty array
means looked and found none — and the projection now fills the array rather than refusing the
capability.

The `tables-not-extracted` limitation is gone as the false-capability partner, replaced by
`unruled-tables-not-detected`: **the only limitation in the set that partners a `true` capability
rather than a `false` one.** Without it an empty array reads as "this page has no table", when what
it means is "no *ruled* table was found here".

### Changed — schema and version

`REPRESENTATION_SCHEMA_VERSION` 0.1.0 → **0.2.0**: the payload grew a `tables` array. No
`ethos.grounding.v2` was invented — Ethos's v1 already has `tables`, and the projection fills it.

Workspace 0.2.0 → **0.3.0**, and `profile_sha256` moves `8357e5ba…7f2497` →
`fad389ae…ef9a14` for three reasons: the version, the new `table_detection` rule id, and
`capabilities.tables` flipping. That last one is not a knob but a **claim** — artifacts before it
did not look for tables and artifacts after it did, which is exactly what a profile hash exists to
make visible.

### Oracle

`ethos grounding check` accepts the tables payload and agrees with the engine's own checker
byte-for-byte on it — `counts.tables: 1`, `structure: valid`, `source_binding: matched`, identical
`representation_sha256`. The partition is still 12 readable / 3 refused.

## [Unreleased] — v0.1, as 0.2.0

The roadmap row after v0 (`docs/02-ROADMAP.md`): citation verification as a declared capability,
encoding-issue detection, and the xref repair-or-refuse decision. **Not tagged.** v0's exit
criteria are untouched — `docs/03-V0-SCOPE.md` §5 is still fifteen criteria and fourteen jobs, and
a test asserts that number does not move for v0.1 work.

### Version: 0.1.0 → 0.2.0, and the profile hash moves again

```
profile_sha256  d2ebf3ef…6d21fc   ->   8357e5ba…7f2497
```

Three causes at once, and the middle one is the largest identity change since M1:

| Cause | Why it is identity |
| --- | --- |
| `parser_version` 0.1.0 → 0.2.0 | A `Profile` field, as always |
| **`xref_repair`** | Decides *which documents produce an artifact at all* |
| **`verifier`** | Which verifier a run was bound to |

Both new fields are adjacently tagged (`{"mode": …}`), matching `PageBudget` — and that is
load-bearing rather than cosmetic. serde does **not** honour `deny_unknown_fields` on an
*internally* tagged enum, so `{"mode":"refuse","future_knob":true}` would have parsed, dropped the
knob, and re-hashed to a digest different from the one it arrived with. The nested-field test
caught it during the change.

The version number now leads the roadmap label by one minor: v0 shipped as 0.1.0, and this row —
"v0.1" — ships as 0.2.0. Two profile fields and a fifth subcommand are more than a patch.

### Added — `engine verify`: invoking a verifier, still not verifying

```bash
engine verify grounding.json --citations claims.json [--fail-on-ungrounded] [--config F] [--out F]
```

**The report is the verifier's bytes.** `engine verify` stdout is byte-identical to running
`ethos verify` with the same arguments, asserted on both the grounded and the ungrounded path.
Not "the same fields" — the same bytes, because the engine forwards them and forms no opinion.

`engine_core::verifier` is the whole of it, and **what it does not contain is the design**: no
type for a report, a claim, a check, an evidence tier or a result. There is nothing to re-derive
because there is nothing that reads. `ci/forbidden-tokens.sh verification` still exits 0 with the
shim in the tree, which is the check that the shim really is only a shim.

| Path | Behaviour |
| --- | --- |
| Grounded claim | Exit 0, report relayed verbatim |
| Ungrounded + `--fail-on-ungrounded` | **Exit 1**, report still written — the product gate |
| Ungrounded, no flag | The verifier's own exit status forwarded, report still written. Never a silent skip |
| No verifier | **Exit 2, nothing on stdout**, a named `missing_part`. Never a skip, never a stub, never a default-pass |
| Verifier usage refusal | Exit 2, no report, the verifier's own stderr forwarded |

1 and 2 never collapse, for the reason the classify codes never collapse: a caller must be able to
tell *the check failed* from *the check did not run*.

**`ETHOS_BIN` is authoritative, not a hint** — set and unresolvable is a hard error rather than a
fallback to some other binary. An operator who pinned a verifier and silently got a different one
is in the worst position available: they believe they know which one answered. Resolution order is
the oracle harness's, so one binary answers for both.

**The pin is version *and* digest.** Two builds of the same version can differ, so a version-only
pin would not make a rebuild fingerprint-visible. A test asserts that changing either moves
`profile_sha256`.

**The adapter is declared, not sniffed.** The engine passes `--grounding ethos-grounding-json`,
measured against the pinned binary rather than remembered — it accepts three adapter ids and only
that one loads an `ethos.grounding.v1` artifact. `ethos verify` *can* infer the type, and
declaring it anyway is what turns a wrong input into a usage error instead of a silent fall back
to native-document loading.

One measurement worth recording, because it cost an hour: a citations envelope's
`document_fingerprint` must be the **sha256 of the grounding file's raw bytes** — the same digest
M6 established as `representation_sha256`. Naming the PDF's own digest instead returns
`stale_fingerprint`, and omitting the envelope entirely returns `missing_citation_fingerprint`.
Neither is an error; both are reports saying the claim did not ground, which is exactly the kind
of confident-but-wrong green a test could have been written around.

### Added — encoding-issue detection (parity checklist P10)

Through v0, one unmappable glyph anywhere refused the **whole document**. Fail-closed, but far
more than the evidence required: a page with a single bad code yielded nothing at all.

v0.1 drops the affected run and keeps the page, declaring `broken-font-encoding` with a count.
The run is dropped **whole** — two alternatives were rejected explicitly:

- emitting `U+FFFD` puts a character in the evidence the document does not contain;
- omitting just the bad code splices the surrounding glyphs into a word the document never wrote,
  which is undetectable downstream and worse than losing the run.

A document that shows text and decodes **none** of it is still refused outright, with a named
error. That is the line: an artifact carrying zero runs would be indistinguishable from a
genuinely blank page.

New fixture `fixtures/engine/broken-font-encoding` (CC0, engine-owned, sixth of its kind). Its
`/Differences` remap codes 200–202 to glyph names no vendored table resolves — and in
WinAnsiEncoding those three codes are E-grave, E-acute and E-circumflex, so a reader that ignored
`/Differences`, or fell back to the base encoding on a glyph-name miss, would emit `ÈÉÊ` inside a
perfectly well-formed artifact. The test asserts those characters are absent, that `Readable`
survives, and that the limitation is present with its count.

A correction found on the way: the code comment at the drop site claimed *"the run continues so
the rest of the page is still extractable"*. It did not — the line under it was `return Err(e)`.
The code was the truth and the comment was aspiration.

### Added — the xref decision, written down (parity checklist P20)

**Decided: repair, bounded and declared.** `docs/01-CONTRACT.md` §8.1 is the decision;
`engine_pdf::xref` is its implementation; `Profile::xref_repair` is the knob, and
`{"mode":"refuse"}` restores v0's behaviour exactly.

v0 refused 19-byte cross-reference entries where PDF 32000-1 §7.5.4 requires 20 — about 1 valid
document in 26 on the Ethos corpus, against a backend (PDFium) that repairs it — and deferred the
call. The repair pads each entry and re-parses. Nothing else is touched.

**Why it is safe is a stronger claim than "it works", and the preconditions are the argument.**
Padding grows the file, and a cross-reference entry *is* a byte offset — moving a byte an offset
points at would turn a refusal into the one outcome this project refuses outright: a document that
parses into the wrong objects and produces a well-formed artifact that is silently wrong. So the
repair runs only when nothing an offset points at can move:

1. exactly one `xref` keyword table;
2. no `/Prev` in the trailer, so no incremental-update chain reaches into moved bytes;
3. `startxref` names that table's own start;
4. **every** entry is the 19-byte class — a mixed-stride table is worse repaired than refused;
5. every in-use offset precedes the table.

Each has a unit test that constructs the hostile document and asserts refusal. The repair is also
a fallback — the document is parsed as written first — and magic and encryption are answered
before it, so neither is ever repaired. General PDFium-style recovery was considered and rejected:
it makes "the engine read it" stop implying "the document said it", and the whole artifact
contract rests on that implication.

**The consequence, in the open.** `synthetic/table-regular-grid` now reads, yielding its real
content (`Name`, `Score`, `Alpha`, `10`, `Beta`, `12`). The oracle partition moved from **11
compared / 4 refused to 12 / 3**, and every place that quoted those numbers moved in the same
change: `01-CONTRACT.md` §11, `03-V0-SCOPE.md` §4, `07-VERIFY-BOUNDARY.md` Stage 0,
`fixtures/README.md`, and the `ORACLE_AGREED_COUNT` constant. Ten tests asserted the old
behaviour and were rewritten rather than deleted — including the mutation survivor set, which
gained `table-regular-grid/junk-after-eof` for the same reason every other openable document has
it. It is still a *table* document read as single-column text in stream order: the tables
limitation is unchanged, and v0.1 is not v1.

### Changed — two guards that were passing for the wrong reason

- **`ci/forbidden-tokens.sh`** checked for `/* */` block comments in the **raw** file, before
  stripping `//` comments — so a doc comment containing `crates/*/src`, prose about the scan's own
  scope, failed the scan. It now checks the stripped text. A `/*` inside a `//` comment is not a
  block comment, and a guard that cannot tell the difference is one somebody eventually disables.
  Both probes still fire: a real block comment and a real token each fail it.
- **`every_matrix_job_is_claimed_by_a_criterion`** scanned every `- id:` in the workflow, so
  v0.1's new matrix looked like unclaimed §5 jobs. It is now scoped structurally to the
  `v0-exit-criteria` block, and `the_v01_gates_exist` keeps the v0.1 matrix from emptying out
  quietly in exchange.

### CI

Three new named jobs in their own `v01-gates` matrix — `v01-verify-relay`, `v01-encoding`,
`v01-xref-decision` — deliberately separate from `v0-exit-criteria` so v0's map does not move.
The oracle is still required everywhere it was, and no `--skip` appears anywhere.

## [0.1.0] — v0, frozen

**Not tagged.** The freeze is in-tree; creating the tag and any release is a separate, deliberate
act. Everything below is committed and green under `cargo test --workspace --locked`.

### M7 — CLI + library freeze + v0 exit criteria as CI jobs

**No new capability.** M7 is the milestone that closes v0 rather than extending it: it makes the
exit criteria checkable, makes the public surface a decision, and adds the two test layers §5 asks
for. One behaviour change landed, and it is a hardening the mutation work found — see below.

#### Version: 0.0.0 → 0.1.0, and the profile hash moves

`parser_version` is a `Profile` field, so a version bump **is** a profile change:

```
profile_sha256  f34be632…6faf1e   ->   d2ebf3ef…6d21fc
```

That is the design working, not a regression. Artifacts produced before and after are correctly
non-comparable, because the profile that produced them really did change. Two pins moved with it:
`the_default_profile_is_pinned` and `docs/draft-schemas/profile.draft.json`'s example. Nothing else
in the suite depended on the value — the oracle comparison is over `structure`, `source_binding`,
`representation_sha256` and `counts`, none of which carry the profile.

#### Added — `--diagnostics`

Opt-in, global across all four subcommands, **off by default**.

- **stdout is untouched.** The artifact is byte-identical with the flag and without, and
  `the_flag_adds_one_stderr_line_and_changes_no_stdout_byte` asserts exactly that per subcommand.
  A default run writes *nothing* to stderr at all.
- **One JSON object on stderr**, carrying stage, engine version, wall-clock microseconds, input
  path and size, build target, and resident bytes where the platform reports one cheaply
  (Linux only; `None` elsewhere, which is a typed absence rather than a zero).
- **Not an artifact, deliberately.** No `artifact_type`, no `schema_version`, no `profile_sha256`,
  and not canonicalized — it uses plain `serde_json`, not `c14n`. An object that looked like an
  artifact would eventually be consumed like one, and then a timing would be inside somebody's
  hash.
- **New:** `engine_core::diagnostics` — `Diagnostics`, `DiagnosticsRun`, `HostInfo`, `Stage`,
  `DIAGNOSTICS_VERSION`. The library assembles the observation; the CLI only picks the stream,
  which is the whole of what a thin shell may do.
- `no_diagnostics_field_name_appears_in_any_artifact` walks every key of all four artifacts at
  every depth and asserts none is diagnostics-class. Checking that two runs match would have
  passed for an artifact carrying a `host` field on a machine where the host never changes.

#### Changed — the public API is now a list

`docs/PUBLIC-API.md` is new and enumerates every supported export per crate.
`crates/engine-cli/tests/public_api.rs` fails if a crate root and that document disagree in either
direction.

**`engine-pdf`'s parsing machinery is `pub(crate)`:** `ops`, `content`, `cmap`, `encoding`,
`fonts`, `metrics`, `text_state`, `thresholds`, `nodes`, `magic`, `classify`, `document`,
`extract`, `represent`, `reasons`. Everything a caller needs is re-exported at the crate root by
name. `exit` and `limitations` stay public — the latter because its constants are wire vocabulary
a consumer matches on after reading an artifact.

**Narrowing found dead code, which is the argument for narrowing.** With the modules public the
compiler could not see that these had no readers:

| Item | Disposition |
| --- | --- |
| `Font::subtype`, `Font::base_font` | **Removed.** Parsed and stored since M3, never read. `/BaseFont` is no longer read at all |
| `SimpleEncoding::base` | **Removed.** No callers anywhere |
| `ToUnicode::len` / `is_empty`, `Matrix::apply`, `Operator::token` / `ALL` | `#[cfg(test)]` — used only by the tests that prove the tables round-trip |

`tests/extraction.rs::an_unknown_operator_produces_no_artifact` was rewritten to go through the
public API — a real document with one operator token overwritten in place — instead of driving the
interpreter directly. That was the only thing keeping `ops` and `content` public, and the
replacement is a stronger test: it asserts the property a caller depends on.

#### Fixed — a fail-closed path that was the call site's guarantee, not the type's

`Interpreter::run` returned `Err` on an unrecognised operator and left everything shown before it
sitting in `self.shown`. **No artifact was ever wrong** — `extract` propagates with `?` and drops
the interpreter — but the guarantee lived at the call site, and the test meant to cover it passed
for the wrong reason: it ran a stream that had shown nothing yet, so it would have held even if
partial output were kept. `run` now clears `shown` and `undecodable` on the error path, and the
test shows real text first, with a control asserting the unmutated stream does produce output.

#### Added — fixture mutation over the whole manifest

`crates/engine-pdf/tests/robustness.rs`. All **23** manifest fixtures × **6** deterministic
mutations = 130 mutants; the 8 pairs that cannot be built are pinned with reasons rather than
skipped. Mutations: empty, truncate-to-16, a byte flipped in the last tenth, `%PDF` overwritten,
junk after `%%EOF`, and a same-length operator substitution injecting a token outside Table A.1.

Every mutant must either be refused with one of the six taxonomy codes, or read — and a mutant
that reads must bind to **its own** digest. The third outcome, an artifact that reads as though
the original had been parsed, is what the suite exists to forbid; nothing downstream could detect
it. No mutant panics.

**28 survivors, pinned and triaged into two classes.** `junk-after-eof` on everything that opens
(a reader reaches the trailer via `startxref`, so appended bytes are outside every declared
offset), and `flip-tail-byte` on four documents where `lopdf` recovers by scanning for the catalog
instead of trusting a damaged trailer reference — inspected, not assumed: on `synthetic/two-lines`
the flipped byte is the `t` of `/Root`. The same mutation refuses on eleven other fixtures.

**One triage finding is worth recording.** Widening the operator match to be whitespace-delimited
(half the corpus writes `(text) Tj\n`, which a space-delimited search missed entirely) made the
mutation appear to apply to the two NIST benchmarks. It was not applying: the hit in
`nist-sp-800-63b` is at offset 301887, inside a Flate stream, surrounded by binary. Overwriting it
corrupts compressed data and fails on *decompression* — the test would have gone green while
proving nothing about operator handling. Compressed documents are now excluded from that mutation.

#### Added — `cargo-fuzz` on the PDF entry point

`fuzz/`, **excluded from the workspace** so `libfuzzer-sys` never enters the graph `cargo deny`
inspects or `cargo build --workspace` compiles. Two targets, because libFuzzer's coverage feedback
is per-target and one binary that sometimes classifies and sometimes extracts explores both worse
than either alone:

- `open_and_classify` — `Document::open_bytes` → `classify` → `to_canonical_bytes`
- `open_and_extract` — the deep path through the interpreter, CMaps, font metrics and quantization

An `EngineError` is a pass; a panic is a release blocker. CI budget is 60s per target with
`-timeout=10` for hang prevention and `-max_len=65536`, on nightly **installed only in that job** —
the workspace MSRV stays 1.88. Four degenerate seeds are committed in `fuzz/seeds/`;
`fuzz/seed-corpus.sh` adds the five engine-owned fixtures at run time. The Ethos conformance corpus
is not seeded from: those fixtures are read-only and referenced by hash, and a fuzz corpus is a
copy.

#### Added — `docs/03-V0-SCOPE.md` §5 as fifteen named CI jobs

A matrix in `.github/workflows/ci.yml`, one entry per criterion, each named after the criterion so
a reviewer sees which line is green. `check` still runs the whole suite as the umbrella gate and is
deliberately not the proof — one tick cannot tell you the classification bound still holds.

`v0-classify-bound` is its own job so a timing wobble is legible as that gate rather than as an
unexplained suite failure, and runs `--exact --test-threads=1`. The load-bearing assertion remains
the instrumented counter (`pages_content_scanned == 8` on 492 pages), which cannot flake.

Two criteria are greps, so they are greps: `ci/forbidden-tokens.sh` runs `v0-no-confidence` and
`v0-no-verify` over `crates/*/src`, needing no toolchain. It strips `//` comments (the repo argues
these rules at length in prose) and the `mod tests` block (two of them list the banned tokens *as
data*). **The first version skipped from `mod tests` to end of file** on the reasoning that test
modules come last; they do, and it was still wrong — a probe appended after one sailed through
clean. It now resumes at the closing brace.

`crates/engine-cli/tests/v0_exit_criteria.rs` closes the loop: every §5 line is ticked and names a
job, every named job exists, every matrix entry is claimed by a criterion, **no `--skip` appears
anywhere in CI**, and **no job's filter matches zero tests**.

The last two are the ways this scheme could go hollow while staying green. A `--skip` is the
cheapest way to turn a red criterion green and it deletes the criterion in the process. A filter
naming a renamed test is quieter still: `cargo test -- a_renamed_test` selects nothing, libtest
prints `ok. 0 passed`, and the job passes having checked nothing at all — with every box still
ticked and every job still present. Every filter token in the workflow is now required to occur
inside some test function's name.

#### Added — library-only thin-shell proofs

`crates/engine-cli/tests/library_surface.rs`. The existing CLI tests assert the binary's stdout
equals the library's bytes, which on its own is circular: it proves the two agree, not that the
library alone can produce the artifact. If a subcommand grew a step the CLI performed itself, both
sides would include it and both tests would pass. These do not spawn the binary, and
`no_test_in_this_file_spawns_the_binary` asserts that about the source.

Also new there: `every_geometry_bearing_artifact_declares_its_coordinate_system`, which asserts the
§5 line in both directions — the representation and grounding artifacts declare a frame, and the
classification, which carries no geometry, does not.

#### Documentation

- **`docs/PUBLIC-API.md`** — new. Three crate tables, what is internal and why, and the CLI↔library
  thin-shell mapping with the test names on both sides.
- **`README.md`** — rewritten. It said "M3 complete" three milestones later. Now states v0 frozen,
  the four subcommands, the §5 job map, and an explicit performance posture: the only quantitative
  claim is the bound-test counter, and no bake-off table appears anywhere.
- **`docs/03-V0-SCOPE.md` §5** — all fifteen boxes ticked, each naming its job, plus a §5.1 table
  of what each job actually runs.
- **`docs/README.md`**, **`docs/05-MILESTONES.md`** — M7 marked done; next work is v0.1, a roadmap
  item rather than an M-number.
- **`docs/07-VERIFY-BOUNDARY.md`** — re-read, unchanged. It still describes reality:
  `grounding-check` validates structure and binding, nothing verifies a claim, and the boundary is
  now a CI job rather than a review item.

### M6 — `grounding-check` validator + oracle agreement + double-run identity

**The bare `cargo test --workspace --locked` is green for the first time since the repo existed.**
`oracle_agrees_on_simple_text` was written in the first commit to fail, with a diagnostic naming
what was missing; it failed for six milestones and now runs the comparison it always described.

**Added — `engine-grounding::check`**

- `grounding_check(grounding_json, source_pdf_bytes) -> ValidationReport`, plus `ValidationReport`,
  `Structure`, `SourceBinding`, `Counts`, `ReportError`. Emits `ethos.grounding_validation.v1`.
- **Structure and binding only.** No claim, no verdict, no `grounded`, no evidence tier, no quote
  matching, and nothing re-derived from a verifier's report. A grep test over the check path
  enforces it — with the unit-test module cut out first, because a test that asserts those tokens
  are absent has to name them, and a scan that cannot tell a rule from its enforcement fires on
  itself. (It did, once.)

**Added — `engine grounding-check [--source-artifact <pdf>]`**

The last unimplemented subcommand. `no_subcommand_claims_to_be_unimplemented` replaces the test
that used to assert the opposite — inverted rather than deleted, because the property still
matters in the other direction.

**`representation_sha256` is the hash of the grounding file's raw bytes**

Measured from `../ethos/crates/ethos-core/src/grounding_json.rs`, where `parse_grounding_json`
does `hash.update(bytes)` on the slice it was handed, and confirmed by running the binary: for our
own artifact it returned exactly `sha256sum g.json`. It is **not**
`DocumentRepresentation::representation_c14n_sha256`, which digests a representation's payload
subtree — a different input for a different purpose. The two sharing most of a name is precisely
why M5 renamed ours, and a test pins the distinction: appending a newline to an artifact changes
this digest while leaving `counts` identical, because it follows the bytes and not the meaning.

**Why the checker mirrors Ethos's parser and not just the pinned schema**

The schema is necessary and not sufficient. Id uniqueness, reference resolution, page ordering,
boxes inside their page, capability/array agreement, character offsets that index their element's
text, table cell occupancy — none of it is expressible in JSON Schema, and a checker validating
only the schema would call artifacts valid that the oracle calls invalid. The rules are
transcribed **in Ethos's order**, because the order decides which code an artifact with two faults
reports, and the harness compares the code and the path, not just the verdict.

That fidelity work found six real gaps, all fixed rather than tolerated. The first surfaced on the
checker's own first run: `source.sha256` was typed as a self-validating `Sha256Hex`, so a
malformed digest was rejected during deserialization and reported at path `/`, where Ethos reports
`invalid_field` at `/source` — the same verdict at the wrong place, which is half an agreement.
The wire type is now a plain string and the strictness lives in the checker at Ethos's path;
emission is unaffected, because `project()` still builds it from a validated digest.

The other five were found by **hunting for divergence** rather than by testing the inputs that
came to mind, and every one of them agreed on `structure` while disagreeing on the reason:

| input | engine said | Ethos says |
| --- | --- | --- |
| a float where an integer belongs | `invalid_field` `/` | `invalid_json` `/` |
| an integer past `2^53-1` | `invalid_invariant` `/pages/0` | `limit_exceeded` `/` |
| an oversized multibyte string | `limit_exceeded` `/elements/0/text` | `limit_exceeded` `/` |
| an unknown field inside a page | `unknown_field` `/` | `unknown_field` `/pages/0/zzz` |
| `null` where a string belongs | `invalid_field` `/` | `invalid_json` `/` |

The cause was structural: `deny_unknown_fields` and typed deserialization reach the right verdict
by the wrong route, and can only ever say `/`. Ethos refuses these **before** any field is typed,
in a strict value pass, and then walks the value tree to report an unknown field's exact path. The
checker now does both, in that order, and a `adversarial_inputs_agree_on_verdict_code_and_path`
test pins nine such inputs against the live oracle so they cannot come back.

**One ordering difference survives and is documented rather than chased**: Ethos's deserializer is
streaming, so an artifact broken in *two* different ways reports whichever fault appears first in
the bytes, while the engine's walk reports whichever rule comes first in its own order. Both call
such an input invalid with an error present, and all four compared fields are identical — only the
code can differ, and only for an artifact that is already broken twice.

**Oracle matrix**

| | |
| --- | --- |
| Ethos-owned fixtures compared and agreed | **11 / 15** |
| Fixtures this backend cannot read at all | **4** — `table-regular-grid` (19-byte xref), `corrupt-header-valid`, `invalid-header`, `password-protected` |

The four are excluded **visibly**: the harness requires the agreed and refused lists to partition
the corpus exactly, prints each refusal with its reason, and a separate test asserts each still
exits 2 with no artifact on stdout. A fixture that started quietly emitting an empty artifact
would move between the lists and be caught.

**Exit codes: finer than Ethos, never contradictory**

| Outcome | engine | Ethos |
| --- | --- | --- |
| valid + `matched` / `not_checked` | 0 | 0 |
| valid + `mismatched` | **1** | 2 |
| invalid structure | **1** | 2 |
| could not read the input | 2 | 2 |

Both agree on zero versus non-zero, which is what a shell predicate reads. Where they differ the
engine keeps its own taxonomy (`03-V0-SCOPE.md` §3.1): 1 is "I read it and the answer is no", 2 is
"I could not read it". Collapsing those is the LiteParse defect this project exists to refuse. The
oracle criterion is agreement on the **report**, and the report distinguishes them either way.

**The built oracle is older than its own source, and the harness had to handle it**

The `ethos` binary in the sibling tree prints the validation report bare. The committed source —
and the ref CI pins, `ETHOS_ORACLE_REF` — wraps it in an in-toto Statement with the report at
`predicate`. So the local run and the CI run see different shapes. `extract_report` reads either
and **refuses anything else rather than guessing** which field holds the verdict; hunting for the
first object with a `structure` key would be exactly the best-effort parsing of an unrecognised
shape §8 forbids. A unit test feeds it both envelopes, because the local run alone would never
exercise the wrapped path.

**Double-run byte identity, on files**

`classify → extract → ground → grounding-check`, twice, into separate directories, comparing bytes
on disk across every openable conformance fixture. Not parsed equality — a value comparison passes
while the files differ by key order or a trailing newline, and the contract is about artifacts a
consumer stores.

**CI, and the docs that still taught people to bypass it**

`--skip oracle_agrees_on_simple_text` is deleted from the workflow, and so is the informational
`continue-on-error` step that used to report the oracle's expected failure. The bare command is the
gate now.

The audit caught the embarrassing half of that: the **root `README.md`** — the first thing a new
reader runs — still printed the `--skip` command and still said "next milestone: M4". The CI
comment added by this change says re-adding a skip "would delete the only thing that proves this
engine and the verifier read an artifact the same way", while the front door taught exactly that.
Corrected, along with `01-CONTRACT.md` §11 and `05-MILESTONES.md` M6, which both still claimed
agreement "across all 15 fixtures", and M6's "In" line, which still specified **JSON Schema
validation only** — a decision this milestone deliberately reversed and never recorded.

`ORACLE_AGREED_COUNT = 11` is now pinned beside `ETHOS_OWNED_FIXTURE_COUNT = 15` and asserted, for
the reason the 15 already was: the number is quoted in three docs, and without the assertion a
regression that refused six more documents would move them quietly to the refused list and leave
the suite green.

**What the corpus agreement can and cannot show — stated plainly, because the number reads stronger
than it is**

All 15 Ethos-owned fixtures are standard-14 Helvetica with no font descriptor, so every node's
geometry is typed-absent and **every grounding artifact the corpus produces is `1 page / 0 elements
/ 0 spans`**. Eleven agreements over eleven copies of that shape exercise the identity, source,
coordinate-system, capability and page rules — and never reach the element loop, the span loop, the
offsets rule or the table rules, which is most of what the checker does. An adversarial audit found
this and it was the right catch. `the_oracle_agrees_on_an_artifact_with_real_elements_and_spans`
now compares a benchmark document that projects 1975 elements and 1975 spans, and breaks a rule
*inside* the element loop to prove both checkers locate it identically. It is a benchmark document
rather than one of the 15, so it does not touch the oracle count.

**Six more fidelity gaps, found by audit, all fixed**

The differential corpus above was written by the same person who wrote the checker, which is a weak
form of evidence. An independent audit drove ~119 crafted artifacts through both binaries and found
six divergences the corpus missed:

| input | engine said | Ethos says |
| --- | --- | --- |
| `kind` longer than 256 bytes | `invalid` | **`valid`** |
| a table cell's `text` past the byte limit | **`valid`** | `invalid` |
| `rotation: -90` | `invalid_invariant` `/pages/0/rotation` | `invalid_field` `/` |
| a *value* containing the text "duplicate field" | `duplicate_key` | `invalid_field` |
| an invalid artifact plus a non-PDF source | no report at all | a full report |

Two of those are the serious kind. **The cell-text hole let the engine say `valid` where the
verifier says `invalid`** — the one direction `01-CONTRACT.md` §11 forbids, and the exact failure a
consumer would hit by shipping an artifact this engine had blessed. **The `kind` limit was invented
here**: 256 comes from the JSON Schema, Ethos's parser has no length bound on `kind` at all, and
for a *checker* the oracle wins — being stricter than the verifier does not make the engine safer,
it makes the two disagree. The `rotation` field is now `u16`, matching Ethos's own type, so the
refusal happens at the same stage. Error classification no longer substring-matches text a
*document* can control.

The last one was a comment asserting the opposite of the code: `check.rs` claimed the PDF magic
check ran "before the artifact parses… the same order Ethos uses". Ethos parses first and reads the
source second, and the difference was visible — an invalid artifact with an unusable source got a
report from the oracle and nothing from the engine. Order corrected, comment corrected.

**Fixed — a harness bug that looked like a product bug**

Two oracle tests walk the same corpus and Cargo runs them in parallel threads of one process, so
scratch directories named from the pid and fixture id collided: one test deleted the working
directory of the other mid-run, and it surfaced as "the engine printed no report". An atomic
counter makes the isolation real rather than probable. Worth recording because the symptom
pointed squarely at the wrong component.

### M5 — `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter

The canonical evidence record, and the projection a verifier consumes. `engine extract` now emits
the record; `engine ground` projects it.

**Added — `engine-core`**

- `representation` — `DocumentRepresentation`, `RepresentationPayload`, `PageRecord`, `Node`,
  `NativeLocator` (the contract's discriminated union, `Pdf` variant only), `StructuralLocator`,
  `NodeKind`, `TextRunAttributes`, `NodeGeometry`, `ProcessingRun`, `SourceIdentity`.
- **The fingerprint is the digest of a literal subtree**, `representation_c14n_sha256` =
  `"sha256:" + hex(sha256(c14n(doc["representation"])))`. A reader recomputes it with no domain
  knowledge. The alternative — hash the document minus a named key set — makes canonicalization a
  second rule two implementations can drift on, and a drifted rule produces a mismatch that looks
  exactly like tampering.
- **Named `representation_c14n_sha256`, not `representation_sha256`**, because Ethos already uses
  that name for a hash of a grounding **file's raw bytes**. Two different things under one name is
  how a consumer concludes tampering where there is only a naming collision.
- **Geometry sits outside the fingerprint** (§4), implemented by keeping the type out of the
  payload rather than by filtering at hash time — a filter is a rule someone can quietly change; a
  type that is not there cannot be hashed by accident. Two mirrored tests: moving a box does not
  move the digest, and moving a text origin does.
- **A measured box outside its page is a hard error.** Reachable, not theoretical: an ink box is
  `baseline_y − ascent × size` in a top-left system, so a baseline within one ascent of the page
  top yields a negative `y0` that `QRect::new` accepts. Clamping fabricates; omitting would have to
  travel the geometry-omission path, which takes a typed absence by construction. So it is refused.
- **Pages are records, not nodes.** A node carries a required `NativeLocator` whose PDF variant is
  a *character* origin. A page has none, and `{"origin_x":0,"origin_y":0}` for a page is
  indistinguishable on the wire from a run drawn in the corner.

**Added — `engine-pdf`**

- `represent::to_representation` — `ExtractArtifact` → record. `extract()` keeps its signature and
  its artifact, so M3's thirty-odd behavioural tests still assert on what the parser produces;
  `every_extracted_run_appears_exactly_once` is the bridge that stops the two drifting.

**Added — `engine-grounding`** (was an M0 skeleton)

- `project()` → `Projection { source, omission }`, and `to_canonical_bytes()`.
- **The omission rule is a type, not a convention.** `GroundedBox` has a private field and one
  constructor, `from_presence(GeometryPresence)`. There is no `GroundedBox::new(x0, y0, x1, y1)`,
  so "omit because the page looked bad" would require *adding* a constructor. An audit showed the
  first version of that guard — a source scan — could be walked past by writing the new
  constructor in the obvious shape, so `GroundedBox` now lives in its own module with the field
  private to it: `project()` cannot construct one either, and the guarantee is the compiler's
  rather than a grep's.
- The crate depends on `engine-core` alone and **never reads a locator at all**: the projection
  addresses pages by node id. A test fails if it so much as mentions `NativeLocator`, which is a
  stronger guarantee than hiding the type in `engine-pdf` would have given.

**Added — schema conformance without a new dependency**

- A **JSON Schema subset validator driven by the pinned schema file itself**, so it cannot drift
  from the rules it enforces. It fails loudly on any keyword it does not implement — a validator
  that silently ignores a keyword reports success over rules it never checked — and
  `the_validator_rejects_each_deliberate_break` runs 17 artifacts broken one rule at a time.
  Without that corpus, every positive conformance test would be satisfied by a validator that
  returns `Ok(())`.
- `crates/engine-grounding/schemas/` holds a byte-for-byte snapshot of Ethos's schema with its
  origin and digest recorded, plus a drift check against `../ethos` when that tree is present.

**Measured, and it is the most important thing in this milestone**

**Across the entire Ethos conformance corpus, zero runs have measurable geometry.** Every fixture
is standard-14 Helvetica with no `/FontDescriptor`, so every grounding artifact projected from the
corpus is `elements: []`, `spans: []`, with the whole run count omitted. The engine reads the text
correctly and the record holds it with native locators intact — none of it can cross into a schema
that requires a `bbox`. The geometry-omission path is therefore the **normal** path, not an edge
case, and `TODO(confirm with Ethos owners)` about an optional `bbox` now has a number behind it
rather than a hypothesis.

**Aligned with Ethos's runtime validator, measured from its source rather than guessed**

Beyond the JSON Schema, `ethos-core/src/grounding_json.rs` enforces rules the schema cannot
express, and emitting something it would reject would be a landmine for M6:
`capabilities.spans` ⟺ the `spans` array is present; `capabilities.tables` ⟺ `tables` is present
(so `tables: false` means the key is **absent**, not an empty array); offsets present ⟺
`char_offsets`; boxes must lie inside their page; page indices ascend from 1. All are asserted on
emitted artifacts.

**`capabilities.char_offsets` stays `false`, and M5 is where that was settled**

The milestone allowed flipping it once a hierarchy existed. The hierarchy now exists and the answer
is still no: v0 does no line grouping, so an element and a span are the *same object* and an offset
would always be `0..len` — advertising sub-element addressing the engine cannot do. Ethos's own
validator ties the capability to the fields, so claiming it would oblige every span to carry them.
It flips at v1, with grouping. Five doc sites that promised "flips at M5" were corrected.

**Fixed — an overclaim caught before it shipped**

The first draft of `fixtures/README.md` said the new `absent-font-metrics` fixture was the only one
pairing a known advance with an absent box. Measured, that is false: `/Widths` has always been
supplied to every engine fixture, so three M3 fixtures already produce seven such runs. The
fixture's real and narrower contribution is the `from_descriptor → None` route — a descriptor that
**resolves and answers nothing** — which nothing else reaches, and which is the shape most likely to
tempt a `height = font_size` fallback.

**Clarified — `docs/04-ARCHITECTURE.md` §1**

"No PDF concept in `engine-core`" as written forbids something `01-CONTRACT.md` §5.1 requires: the
`NativeLocator` union with a `PdfLocator` variant. The architecture doc's own header says the
contract wins, so §1 now states the line as **machinery, not vocabulary** — no `lopdf`, no
operator, no page tree, no font program; a contract-defined locator variant carrying integers is
data. `engine-grounding` is held to the stronger rule and a test enforces it.

**Fixed after an adversarial audit, and the findings are worth recording**

A multi-agent audit ran the real `ethos grounding check` against every emitted artifact — **13/13
`structure: valid`, `source_binding: matched`**, including a 2-page real form projecting 1975
elements. The emitter was right. The *tests* were not, and six of them passed while the behaviour
they were named for was mutated away:

- **The geometry sidecar was unauthenticated in a way that mattered.** It sits outside the digest
  by §4, but the projection drops locators and keeps boxes — so the one thing the fingerprint did
  not cover was the only spatial claim reaching `ethos.grounding.v1`. Flipping a row
  `Measured` → `Absent` made a node vanish with no declaration; `Absent` → `Measured` emitted a
  fabricated box while the record still declared the node unmeasurable. Both passed
  `verify_fingerprint`. **`check_structure` now binds the sidecar to the payload's own
  `geometry-absent-not-groundable` declaration**, closing both, and the trust boundary is stated
  where a reader will meet it: *a verified representation attests to the text, the order and the
  origins — not to the rectangles.*
- **`GroundedBox`'s "the type system says it" claim was false.** The private field stopped other
  crates while `project()`, in the same module, could build a box from anything; the grep meant to
  cover that gap was walked past by writing `fn from_raw(x0, y0, x1, y1) -> Self`. The type now
  lives in its own module, and the bypass **fails to compile**: `tuple struct constructor
  GroundedBox is private`.
- **`omission_selects_rather_than_empties` did not test selection.** It compared two all-or-nothing
  documents, so an emitter that dropped every box as soon as any node lacked one passed the whole
  suite — the exact §11 hole. It now runs on a mixed-geometry document and asserts the emitted span
  ids are *exactly* the nodes with measured geometry.
- **Nothing checked that an emitted bbox was the measured one.** A fabricated `[x0, y0, x0+1,
  y0+1]` satisfied shape and containment and passed. Now compared against the node's own rectangle,
  over 100+ boxes.
- **`lossiness_is_asserted_not_assumed` was vacuous** — pointed at a fixture whose artifact had no
  elements, so the dropped field names could not have appeared however the projection behaved.
  Retargeted, with a non-empty guard.
- **Three boundary guards read `src/lib.rs` alone**, so a second file in the crate was invisible to
  all of them. Now recursive.
- **A limitation shipping inside every artifact still said the hierarchy "lands at M5"** — it had
  landed. Corrected to the real reason.

Each of the first four fixes was re-verified by re-applying the audit's mutation and watching the
named test fail.

**Not done, deliberately**

- **No `grounding-check`, no oracle agreement.** That is M6. This milestone produces artifacts; it
  does not validate them as a product feature. The subset validator is test-layer only, and a test
  asserts it never becomes a runtime dependency.
- No tables, no char offsets, no multi-column reordering, no fabricated geometry.

### M4 — Capabilities, typed absence, explicit multi-column limitation (the L1 gate)

L1's achievement condition names capability declarations explicitly, so an artifact without them
has not reached "extracted" regardless of how good its text is. Every emitted artifact now
declares what the profile can do, what it could not do, what happened to each page, and how the
run ended.

**Added — `engine-core`**

- `assurance` — `Limitation` (stable kebab code + detail + `Profile | Document | Page(n)` scope),
  `PageState`, `PageStateEntry`, `CoverageSummary`, `ProcessingGaps`, `ProcessingTerminalState`,
  `RefusalCode`, and the `Assurance` envelope both artifacts embed.
- `Assurance::new` **derives** the coverage summary and the terminal state from the page states it
  is given. An artifact that claims `Complete` while carrying an unread page is not a bug this
  type can have — which is the only way to guarantee `docs/01-CONTRACT.md` §7's rule that a
  verification over a partially processed document never renders as a clean verification of the
  whole document.
- `CoverageSummary` reconciles: `pages_authorized == processed + failed + unsupported +
  quarantined + not_attempted`. Six buckets, not five: **`not_attempted` is the one a four-bucket
  summary would have had to lie about.** Bounded classification samples `N` pages and stops, so
  folding the rest into `processed` would claim observations nobody made and folding them into
  `failed` would claim failures that never happened.
- `page_binding_status` — **capability-limited beats negative** (Workbench rule 4). A query bound
  to a failed, quarantined, or unattempted page returns `CapabilityLimited { limitation_code }`,
  never a boolean "not present". The signature is the enforcement: there is no way to express
  "missing", so absence of extractable content cannot become evidence of absence in the source.
  `NotInDocument` is a separate answer, because "we skipped it" and "there is no such page" are
  different facts.
- `Capabilities::declared_limitations` — the mirror of the proof rule: **no capability may be
  `false` without a declared limitation**, derived by exhaustive destructuring so adding one
  without a code is a compile error.
- `PageBudget` on `Profile` — `Unlimited` or `AtMost(n)`, a **declared state rather than an
  optional field**. An `Option` missing from incoming JSON deserializes to `None` and silently
  re-hashes as though it had been there; a declared enum cannot.

**Added — `engine-pdf`**

- `limitations` — the format-specific codes `engine-core` is not allowed to know about:
  `backend-xref-strict-20-byte`, `predefined-cmaps-not-vendored`,
  `form-xobject-text-not-descended`, `font-widths-absent`, `classify-sample-bound`, and the
  derived `*-reason-not-detected` pair.
- The **xref refusal is declared on artifacts for documents it did not refuse.**
  `synthetic/table-regular-grid` exits 2 with no body, so the only place a caller can learn this
  backend turns away roughly one document in twenty-six is an artifact for one it accepted.

**Changed — the wire, deliberately**

- **`not_detected` (M2) and `not_decoded` (M3) are gone.** Both were declared stand-ins. Their
  content is now `assurance.limitations`, carrying M2's and M3's reasons verbatim — a test fails
  if any `NOT_DETECTED` entry loses its declaration in the move, and another fails if either field
  reappears. Two vocabularies on one artifact means a consumer has to work out which to trust, and
  the answer is never written down.
- **`capabilities.char_offsets`: `true` → `false`.** v0 emits runs and no element/span hierarchy,
  so there is nothing for an offset to index into. M4 asked for the proof and there was none.
  Flips at M5 with `DocumentRepresentation v0`, with a test.
- **`profile_sha256` moved** to
  `sha256:f34be6328f858e09c241cb51c7b0dbb0fe065ecbf5f00e9942cd6bbcdd6faf1e` — the honest
  `char_offsets` value plus the new `page_budget` knob. Artifacts from before and after are
  correctly non-comparable, because the profile that produced them really did change.
- `Classification::pages_sampled` is now `min(page_count, classify_sample_pages, page_budget)`, so
  it keeps meaning "the pages this run intended to read" and stays equal to
  `pages_content_scanned`.

**`failure/memory-limit-simulated`, stated plainly**

Its bytes are **identical to `synthetic/simple-text`** (`sha256:f2f6ab91…`, asserted in the test
rather than taken on trust). The fixture name means *a limit simulated by configuration*, not
*this PDF is huge*, so the test sets `page_budget` explicitly — and on a one-page document the
only budget that bites is zero. The behaviour is a **declared limitation plus a coverage gap**,
not a hard refusal: one authorized page, zero processed, one quarantined, terminal state
`partial`, and no page tree at all. The same bytes under the default profile read cleanly, which
is the proof the gap came from the knob.

**Fixed — two guards that were not guarding**

- The profile sensitivity test's `capabilities.char_offsets` mutation wrote back the value the
  field already held once `V0` flipped, so it proved nothing while passing. Every mutation now
  asserts it actually changed the profile before asking whether the hash moved.
- `no_pdf_type_or_import_in_engine_core` banned the **prefix** `struct Page`, which read the
  format-agnostic `PageStateEntry` as a PDF page-tree type — forbidding a type the contract
  requires. Banned type names are now matched as whole identifiers, with a test proving the
  exact-match form still catches `struct Page {`.
- `profile.draft.json`'s example still carried `"unbound-until-m3"` placeholders two milestones
  after the backend landed, while its README told readers the example *is* the real profile. The
  example is corrected and `the_profile_schema_example_is_the_real_profile` now holds it there.

**Not done, deliberately**

- **No CLI flag for the page budget.** It is a profile knob, so the binary cannot emit a partial
  artifact at v0 and the question of which exit code one deserves does not arise. Exit-code
  meanings are unchanged.
- **`pages_failed` and `pages_unsupported` are always zero in v0 artifacts.** Extraction fails
  closed on a hard error and emits nothing at all, so no page reaches those states through the
  public path. The buckets are declared with honest zeros and the query helper handles them, but
  nothing pretends they are exercised.
- No repair, no invented pagination, no capability claimed without a proof test.

### M3 — Extract: text runs, native locators, fail-closed operators, synthesized flags

Position-aware text runs whose origins are trustworthy, whose boxes are measured or typed-absent,
and whose parse stops rather than silently dropping content.

**Added — `engine-pdf`**

- `ops` — all **73** operators of PDF 32000-1 Table A.1 as an enum. Dispatch is an exhaustive
  match with **no wildcard arm**: adding a variant without handling it is a compile error, and a
  token outside the table is a hard error naming it. Operators that do not move a glyph are
  *named* no-ops, so "we do not interpret `rg`" is a decision in the source rather than an
  accident of a `_ =>`.
- `content` — the interpreter, including all four show-text operators. **`"` and `'` are
  implemented**; `"`'s absence from pdf-inspector's match is the disqualifying defect, where text
  vanishes and adjacent runs merge with corrupt geometry while the output stays well-formed.
- `text_state` — CTM, text and line matrices, `Tc`/`Tw`/`Tz`/`TL`/`Ts`. **`Tz` multiplies the
  whole advance expression**, spacing terms included; pdf-inspector implements it nowhere.
- `fonts`, `metrics` — three independent questions per glyph (what character, how far, what box),
  because they fail independently. Conflating the last two is how `height = font_size` gets
  written.
- `cmap` — `ToUnicode` parsing (`bfchar`, `bfrange`, codespace ranges), the source of the ligature
  caveat: `<03>` → `<00660069>` is one code and two characters.
- `encoding` — `WinAnsiEncoding` in full, `StandardEncoding`'s ASCII range including the two codes
  where it is *not* ASCII (`0x27` is `quoteright`, `0x60` is `quoteleft`), and `/Differences`.
- `nodes` — `TextRun` with `PdfLocator`, `SynthesizedChar`, `scalar_code_mismatch`.
- `engine extract <pdf>` — canonical JSON, exit 0 or 2. **No exit 1**: "needs attention" is a
  classify concept, and overloading it would make a caller's `&&` chain mean two different things
  depending on which subcommand ran.

**Honesty, and where it shows**

- **Ink boxes are measured or absent.** The whole conformance corpus is standard-14 Helvetica with
  no `FontDescriptor`, so every run reports `NotReportedByReader` — the typed-absence path, proved
  on real documents rather than a mock. One engine-owned fixture carries a descriptor so the
  measured path is proved too.
- **Absent advance is absent, not zero.** A standard-14 font may omit `/Widths` and expect built-in
  AFM metrics, which this profile does not vendor. The advance is omitted from the wire; the origin
  is unaffected, because it comes from the content stream.
- **Hyphenation is not rejoined**, and that is a stated policy with a golden. Distinguishing a soft
  break-hyphen from a real compound hyphen needs a dictionary, and a rule that guesses is the
  cliff-shaped heuristic refused everywhere else. Silent rejoining is forbidden outright.
- **Reading order is stream order.** `two-columns` comes out right-column-first — visibly wrong,
  and asserted that way, because that is what `single-column-v1` means.

**Changed**

- **`skrifa` replaces `ttf-parser`**, which the milestone named. RUSTSEC-2026-0192 records that
  ttf-parser's author declared it unmaintained with no safe upgrade, and names skrifa as the
  maintained successor. Pinned to 0.39 because 0.44's tree needs Rust 1.89 and this workspace pins
  1.88.
- `Profile.cmap_data_version`: `absent-until-m3` → `annex-d-encodings-1`, naming what is actually
  carried. The pinned profile digest moved accordingly.
- The float-ban guard now matches **whole tokens**. It was matching substrings, and a sha256
  digest containing `…cf32c2e…` tripped it — a guard that cries wolf on a hash is a guard someone
  eventually disables.

**Deviation from the milestone text, stated plainly**

M3 called for vendoring ~168 Adobe `.bcmap` CMaps. They are **not** vendored. Nothing in the corpus
exercises them, they could not be obtained and verified in this pass, and committing binary data no
test touches would be worse than declaring the gap. A document naming a predefined CMap is
**refused** with a named error. The same applies to the Core-14 AFM widths and the full Adobe Glyph
List. All three are declared in `not_decoded` and explained in `vendor/README.md`.

**Not done**

- M4 capability blocks and coverage summaries; M5 `DocumentRepresentation v0` and the grounding
  projection; M6 oracle agreement.

### M2 — Classify: reason codes, two axes, three exit codes, bounded sampling

`engine-pdf` opens a PDF once and reports what it observed. No verdict, no confidence, no quality
field — the engine owns the observation, the caller owns the routing policy.

**Added**

- `engine_pdf::Document` — magic-checked, opened once, shared by every stage. M3's `extract` takes
  the same handle rather than reopening: two loads can disagree, and a classifier that saw a
  different object graph from the extractor is a silent divergence with no diagnostic.
- `engine_pdf::classify` — per-page counts, two orthogonal reason axes, a derived boolean, and
  `pages_content_scanned` as an observable bound.
- `OcrNeedReason` / `LayoutComplexityReason` — **separate types**, so `table-likely` cannot be
  constructed as an OCR-need reason. A dense financial table in crisp born-digital text needs no
  OCR at all; a single "complex" list would send it to an engine that could only make it worse.
- `engine classify <pdf>` — canonical JSON on stdout, exit 0 / 1 / 2. `extract`, `ground` and
  `grounding-check` exit 2 naming the milestone that owns them, rather than printing usage and
  exiting 0 as though the work happened.
- `docs/draft-schemas/classification.draft.json`.

**Measured, and it changed two acceptance lines**

- **`irs-form-1040-2025` is exit 1, not exit 0.** It fires `table-likely` and `dense-graphics`.
  Suppressing a true layout reason to make a doc line come out right is the tuning this project
  refuses, so the fixture choice changed: `synthetic/simple-text` is the exit-0 case, and
  `irs-form-1040` became the axis-independence case, which it serves better.
- **The 20%-total-time bound was unachievable, and the reason was informative.** The counter proved
  the page walk was already bounded — 492 pages, 8 scanned — while `lopdf` parses the whole object
  graph eagerly, so total cost is `O(parse) + O(N × per-page)`. Timing the phases separately shows
  what the claim actually is: **246× the pages for 1.7× the classify time.**

**Honesty about what is not detected**

`garbled` and `multi-column` are in the vocabulary and are **never emitted**, because no sound
detector exists for either. The artifact declares both in `not_detected` with the reason — silence
would let a caller read an empty list as evidence a document is not garbled.

`sparse-text` requires short text **alongside imagery**. LiteParse uses `text_length < 20`
unconditionally and compounds it with a vowel-frequency garble heuristic that strips items from the
tally first, so an acronym-dense page can be reported as `no-text` when its text extracted
perfectly. Neither mechanism exists here, and a test asserts `nist-sp-800-53r5` — a control
catalogue full of `AC-2`/`SC-7` — is not reported as textless.

**Changed**

- `Profile.backend.version` is now the resolved `lopdf` version, `0.44.0`, replacing
  `unbound-until-m3`. The pinned profile digest moved accordingly — the mechanism working as
  designed.
- `deny.toml` gains `BSD-3-Clause` (`lopdf` → `encoding_rs`; also the licence the vendored Adobe
  CMaps will need at M3), plus `Unlicense`, `Zlib` and `0BSD` from the same subtree. Each added in
  the PR that introduced the crate needing it.

**Not done**

- Text extraction: content-stream interpretation, CMaps, ink boxes, `NativeLocator` emit (M3). The
  classifier walks content streams to **count** operators; it decodes no text and interprets
  nothing.

### M1 — Contract types, c14n / quanta, `schema_version` on the wire

`docs/01-CONTRACT.md` is now Rust. The artifact shape stops being negotiable and starts being a
compile error.

**Added — `engine-core`**

- `c14n` — c14n v1, a clean-room implementation of the contract Ethos implements. UTF-8, no
  whitespace, keys sorted **explicitly at write time**, minimal escaping, integers only,
  idempotent. Parity vectors lifted from Ethos's committed tests prove byte compatibility without
  taking a Cargo dependency on Ethos.
- `geom` — `quantize` (round-half-away-from-zero; `NaN`/`±∞`/overflow are errors, never clamps)
  and `QRect` as `[x0, y0, x1, y1]` with construction-time validation.
- `identity` — `ArtifactIdentity`, `ArtifactBinding`, `Sha256Hex`, `CoordinateSystem`.
- `profile` — `Profile`, `Capabilities`, `BackendIdentity`, and `profile_sha256()`.
- `derivation` — `DerivationClass` and `GeometryPresence` / `GeometryAbsence`.
- `ids` — `IdAllocator`, `NodeId`, `IdKind`, `sort_ids`.
- `error` — the six-variant `EngineError` taxonomy.

**Added — docs**

- `docs/draft-schemas/` — six DRAFT JSON Schemas plus an index. Under `docs/` deliberately; there
  is still no production `schemas/` path.
- This changelog.

**Decisions worth knowing**

- **`quantize` uses `f64::round`, not Ethos's `(x + 0.5).floor()`.** That idiom double-rounds: once
  `|x| ≥ 2^52` the sum is unrepresentable and rounds *before* `floor` runs, so an exact integer
  product returns one quantum too large — and `MAX_SAFE_INT`, which the contract declares canonical,
  is refused outright. `f64::round` is IEEE `roundToIntegralTiesToAway`: same rule, computed
  exactly. Measured divergence from Ethos across an exhaustive knife-edge sweep of `[0, 2·10^6)`:
  **exactly one value**, `0.49999999999999994`, where this returns `0` (correct — it is below one
  half) and Ethos returns `1`. Unreachable from decimal text, and across all ten million
  `0.001`-step literals in `[0, 10000)` points the two agree everywhere.
- **`QRect` rejects zero-area rectangles**, which is *stricter* than Ethos, whose `QRect::new`
  rejects only `x0 > x1`. Stricter-on-emission is the only safe direction: the engine may refuse to
  emit what the oracle tolerates, never the reverse.
- **`quantize` rejects `quantum_per_point == 0`**, which would otherwise map every coordinate on
  the page to the origin and return `Ok(0)` while doing it.
- **Typed absence, not `Option<QRect>`.** `None` would collapse "could not measure", "nothing to
  measure", and "not asked to measure" into one value, and only the first is a declarable capability
  limitation.
- **`Sha256Hex` rejects uppercase hex** rather than folding it, so one digest has exactly one
  spelling and a comparison never fails for formatting reasons.
- **The default profile is pinned by test**, bytes and digest. A profile change is now a deliberate
  act — including a crate version bump, since `parser_version` is part of identity by design.
- **`Unicode-3.0` added to `deny.toml`'s allowlist.** M1 is the PR that introduced the crate needing
  it: `serde`'s `derive` feature reaches `unicode-ident`, whose licence is
  `(MIT OR Apache-2.0) AND Unicode-3.0` — the `AND` makes it non-optional. Added on introduction
  rather than in advance, which is what the prior comment asked for.

**Fixed after adversarial review**

- **Unknown-field denial now covers every nested object, not the ones someone remembered.**
  `deny_unknown_fields` is not recursive, and the first pass stopped one level short: a profile
  carrying `coordinate_system.future_knob` parsed cleanly and **re-hashed to the unmodified default
  digest** — measured — so an artifact would claim comparability with a profile it does not match.
  Now on `CoordinateSystem`, plus `ArtifactIdentity`, `ArtifactBinding` and `GeometryPresence`,
  matching the `additionalProperties: false` their draft schemas already declared. The nested test
  derives its field list from the serialized value rather than a hardcoded array, so a nested
  object added later is covered automatically.
- **`DerivationClass::may_be_overwritten_by` implemented only half of §6.** It ignored its second
  argument entirely, so it protected `Extracted` while letting `Proposed` overwrite `Computed`,
  `Recognized`, and other `Proposed` nodes — the path by which a model's suggestion quietly becomes
  the record. Both rules are now enforced and pinned by a full 4×4 matrix, plus a test asserting
  the result actually depends on the overwriter.
- `Profile` now denies unknown fields. Without it a profile from a newer engine deserialized with
  its unknown knob silently dropped, then **re-hashed to a different digest than it arrived with** —
  an artifact claiming comparability it does not have.
- The profile sensitivity gate now destructures to the leaf. A field added to `Capabilities` or
  `BackendIdentity` is as output-affecting as one on `Profile`; a gate stopping at
  `capabilities: _` waved it straight through.
- The float ban was scoped to `quantize`'s body rather than the whole of `geom.rs`. The file-level
  skip was a hole big enough for `QRect` — which lives there, derives `Serialize`, and would have
  passed with an `f64` field. Verified by injecting one and watching the guard fire.
- `sort_ids` put a malformed id **first** while its doc claimed last, because `Option`'s natural
  order sorts `None` first. Now sorts last, with a test.

**Not done, deliberately**

- No PDF parsing, classification, or extraction (M2, M3).
- No grounding projection (M5).
- `oracle_agrees_on_simple_text` still fails with its M1–M6 diagnostic, as designed until M6.

### M0 — Repo skeleton, toolchain, deny policy, failing oracle harness

- Four-crate workspace with enforced boundaries; toolchain pinned to 1.88.0.
- `deny.toml`: allowlist-only licences (no AGPL), no network crates, pinned registry.
- `fixtures/manifest.json` — hash-pinned references into the Ethos corpus across two roots.
- Oracle harness with negative-path tests; `oracle_agrees_on_simple_text` fails by design.
- CI: fmt, clippy, build, test, deny, plus a job that *proves* the AGPL gate rejects.
