# 14 — v2 scope: office formats

**Status:** scope authority for v2 · **Slice detail:** `15-V2-MILESTONES.md`
**This is the code-review map for v2.** Every v2 PR belongs to exactly one slice.

**v2 reads eight formats, and v2's format row is closed.** S0 through S13.5 are done: this
document, the grounding decision in §5, `engine-office` — the fifth crate — DOCX, XLSX, PPTX, ODT,
ODS, ODP, RTF and EPUB, **S10 (CSV) as an argued refusal rather than a reader**, embedded assets
counted at S11, both A11 lanes closed at S12 and S13, the guard and prose repairs at S13.1 and
S13.3, the owner's two gate decisions at S13.4, and the two unrun sweeps at S13.5. (This said
*"S0–S10"* until v2-S13.3, three slices after S11 shipped, and *"S0 through S13.3"* until v2-S13.5,
one slice after S13.4 shipped. The same sentence has now drifted twice, and `CAPABILITY.md`'s row
was right both times — a status line in a scope document and a status row in a capability document
are two copies of one fact, and the copy nobody is looking at is the one that goes stale. That is
the same shape as `07-VERIFY-BOUNDARY.md`'s short decision table, which v2-S13.5 also
repaired.) Eight formats read, one argued refusal, and every split was against a measurement.

**Both of the reasons this section carried are settled by the owner (2026-08-21), and neither was
a format.** `00-NORTH-STAR.md` decisions **#16** and **#17** record them, and v2-S13.4 in
`15-V2-MILESTONES.md` argues them. **One question remains and it is not a gate condition** — whether
`zip.rs` should verify the CRC-32 it currently ignores, raised at v2-S13 and answerable in this
repository by a slice that owns it, with a measurement.

The two as they stood, kept because the reasoning is the record:

1. **The gate sentence's verb.** §2 below and `00-NORTH-STAR.md` both say a DOCX quote and an XLSX
   cell both **ground**. Grounding a DOCX is refused — decided at v2-S1 as option (b), pinned by a
   test, listed in `CAPABILITY.md` under **Cannot** — and every slice since has quietly re-read
   *ground* as **bind**. `15-V2-MILESTONES.md`'s S1 open-questions table — the **gate wording**
   row — flagged it as unsettled at S1, and no decision-log entry settled it until **#16**, which
   reads *ground* as **bind**: the gate's own second half, *"no synthesised pages"*, forbids what
   the literal reading requires, so read literally the sentence contradicts itself.
2. **"Embedded assets" — counted at v2-S11, and the rest is the owner's.** §2 restates it as v2's
   content and `02-ROADMAP.md`'s row names it. Until v2-S11 it was neither delivered nor descoped,
   and not merely unread but **uncounted**: `docx.rs`'s `UNREAD_TEXT_PART_PREFIXES` did not match
   `word/media/`, so a DOCX with forty embedded images declared **zero** unread parts for them,
   while EPUB's `unread_entries` counts every unread entry including media. Reported at S10 and
   **fixed at S11**, under a second code rather than a widened first one — `office-parts-not-read`'s
   message says its parts *carry text*, and a PNG does not. What S11 does **not** settle is whether
   the roadmap's word meant *counted* or *read*: no office asset is read, decoded or emitted as a
   node, and `engine-pdf`'s `ImageRecord` has no office counterpart. **Settled by #17: counted
   satisfies v2.** Reading an office asset is a contract change rather than a reader change — an
   office image has no page and no coordinate system — and it gets its own row rather than sitting
   implied inside v2's.

§3's law has now been tested against all seven shapes a "page" can take, and the last four are the
ones that cost something to refuse. A DOCX has none until a renderer invents one. A spreadsheet's
is a print artefact. A slide is a real, discrete, countable thing the package contains — and is a
**part**, so `pages` is empty and `PptxLocator` carries no slide number.

**And an ODT states its own page breaks.** `content.xml` contains `<text:soft-page-break/>` and
`styles.xml` contains an `fo:page-width`, so a `PageRecord` would need almost no arithmetic. It is
still refused, for L30's own reason: a soft page break records where the *producing application's*
layout fell, and it moves when the font stack, the paper size or the producer changes. The reader
sees the element, contributes no character and no address from it, and `pages` stays empty.

**An ODP removes the last of the arithmetic, and is the sharpest refusal in this version.** A
presentation lists `<draw:page>` elements — discrete, ordered, named, and counted out loud by
anybody describing a deck — and a `<style:master-page>` states `fo:page-width` beside them. A
`PageRecord` needed **nothing computed at all**, which had never been true before. It is refused on
L30's four words rather than a paraphrase — *"It invents pagination"* — because a draw page is a
part of the presentation's **structure** and not a page this engine measured. `pages` is `[]`, and
`OdpLocator` carries a `draw_page` **position** named for the element rather than for what it
resembles.

**And an RTF says the word out loud.** `\page` is a page break and `\paperw` a paper width, in the
plainest language any of these formats use. Both are the producing application's print arithmetic,
so `pages` is `[]` and `RtfLocator` has one field with no room for a second. RTF is also the first
format here with **no container**, which §3's second obligation decided rather than the first: a
constant part name would have let the page-less invariant run unchanged and would have been a
string the document does not contain, so the invariant grew a fourth rule instead.

**v1 is not done.** Its table number is measured and honest: macro cell-slot F1 is **64‰** on the
four tagged PDFs this repository owns, and fabrication is **0** (`table-gate-v1.md`,
`09-V1-MILESTONES.md` S7). The **> 0.489 chase is parked** — 0.489 is a published ODL-local score on
*their* corpus, same unit and a different exam, and it is not a precondition for anything in v2
(`00-NORTH-STAR.md` #10). Parking it does not close v1. **v1.1 is complete** at 0.14.1 and **v1.2 is
complete** at 0.19.0. v2 is the next row of `02-ROADMAP.md` and it is scoped because the owner asked
for it, **not** because the gate cleared. Nothing in this document closes v1, and no slice here may
be cited as evidence that it did.

---

## 1. The one sentence

**A DOCX quote binds to something the document actually contains, or it does not bind at all.**

Everything below is machinery for that sentence. Its shadow is the sentence v2 exists to refuse: *a
DOCX quote binds to page 3.*

## 2. Why this version exists — and the one thing that would ruin it

`02-ROADMAP.md`'s v2 row is **Anydoc-class formats**: DOCX → XLSX → PPTX → ODF/RTF/EPUB/CSV, through
a shared IR and one serializer, with embedded assets. Its gate is stated in `00-NORTH-STAR.md` and
the roadmap in the same words:

> A DOCX quote and an XLSX cell both ground; **no synthesised pages**

The second clause is not a footnote. It is the whole hazard, and `06-STEAL-REFUSE.md` already
refuses the shortest path to it — checklist **L30**, verbatim:

> **LibreOffice → PDF office bridge** — **REFUSE**. **It invents pagination.** Pagination does not
> exist in a DOCX and must not be synthesised. This is a refusal, not a fallback.

A DOCX has no page. It has a paragraph order, a run order and a style. Where a page appears is a
decision a *renderer* makes from a font stack, a printer profile and a paper size, and it changes
when any of those change. So a `page` on a Word citation is not a measurement of the document; it is
a measurement of the machine that printed it — and it will look exactly like a locator, because it
is shaped like one. That is the same shape of failure v1.2 §3 spent a whole version preventing for
MCP, arriving through a converter instead of a model.

## 3. The no-synthesised-pages law

**A v2 locator addresses the document's own structure. It never addresses a rendering of it.**

This is not new policy. `01-CONTRACT.md` §5.1 wrote it down in v0, before any second format existed:

> `NativeLocator` is a **discriminated union**. Adding a format adds a variant — `PdfLocator`,
> `DocxLocator`, `XlsxLocator`, `PptxLocator`, `ImageLocator`, `EmailLocator` — plus an adapter
> profile, fixtures, and inspection behaviour. It does not change the source, run, artifact,
> candidate, or verification models.
>
> **A rendered page/bbox is never substituted for the native source address**, unless an approved
> format profile defines that rendering as authoritative. No such profile exists in v0.

v2's job is to add variants and **not** to create such a profile through a side door. Three
obligations, and a format satisfies all three or it does not ship:

| | obligation | what it forbids |
| --- | --- | --- |
| **Structural** | the native locator names things the file itself contains — a paragraph, a run, a cell, a sheet | a page index, a bbox, or a line number derived from layout — **including one the file contains**, because v2-S5 measured that a file can state a page break and the obligation is about what the number *is*, not where it was found |
| **Absent, not invented** | a format with no geometry says so in the type system | a zero box, a page-sized box, or `pages: [{width: 61200, height: 79200}]` because A4 |
| **No renderer in the graph** | nothing converts, prints, paginates or lays out to obtain a locator | LibreOffice, a headless browser, a PDF round-trip, a layout engine, "A4 at 72 DPI" |

### This is checkable, and here is what checks it

The types already have the right spellings, which is why S0 can state the law without inventing
machinery for it:

- **Geometry.** `GeometryAbsence::NotApplicableToKind` — *"The node kind has no geometry by
  definition, so absence is correct rather than a gap"* — and `is_declarable_limitation` returns
  **false** for it, so a page-less format does not inflate the limitation count with a gap that is
  not one. A DOCX run is that variant. It is not `NotReportedByReader`, which means the reader tried
  and could not.
- **Pages.** `RepresentationPayload.pages` is a `Vec<PageRecord>`, so **an empty `pages` array is
  the spelling of "this document has no pages"** — and a non-empty one on a page-less document is
  refused by name as *invented pagination*. v2-S1 measured that the invariant did not yet allow
  that spelling; **v2-S2 made it allow it**, by splitting `check_structure` on the locator family
  rather than by loosening the page rule. A paginated address still needs its declared page.
- **The locator.** A new union variant, per §5.1 — `DocxLocator { part, paragraph, run }` as of
  v2-S2, `OdtLocator { part, paragraph }` as of v2-S5, and
  `OdpLocator { part, draw_page, shape, paragraph }` as of v2-S7, whose first component is a
  **position among `<draw:page>` elements**, named for the element rather than for the page a
  consumer would otherwise read it as. A locator that contained a page number
  would be the same violation as a `bbox`, wearing a field name, and `deny_unknown_fields` is what
  stops one arriving quietly. v2-S5's test refuses `page`, `bbox`, `x` **and `soft_page_break`** by
  name, because for that format the last one is the field somebody would actually reach for; v2-S7's
  adds **`slide_number`**, for the same reason in a format where a person really does say
  *"it's on slide 12"*.
- **Geometry, again, and this one is stronger than a convention.** `check_structure` **refuses a
  measured box on a page-less node**: a box is validated against the page containing it, so a node
  with no page has nothing to validate against, and a rectangle nobody can check is the fabrication
  this law exists to refuse. The office reader could not emit one even if a later edit tried.
- **The dependency graph.** `deny.toml` is the standing proof for the third obligation: a renderer
  is a dependency, and one that arrives shows up there before it shows up in a review.

S1 turned the third obligation into a test (`page_less_source.rs` reads the lock file for a
renderer) and **S2 turned the rest into tests in `engine-core`**, where the invariant lives. The
sentences existed **before** the first parser — the pattern `12-V12-SCOPE.md` §3 set for the handle
law, for the same reason, and S2 is what they were written for.

## 4. One IR, one serializer

Anydoc's **A2** (`06-STEAL-REFUSE.md`, target v2): shared IR → one serializer. Concretely:

- A new format projects into **the representation this engine already emits** — same
  `artifact_type`, same `schema_version` line, same c14n, same fingerprint discipline.
- The existing paths then consume it unchanged: `engine ground`, `engine markdown`, `engine html`,
  `engine mcp`, both SDKs and the LangChain tools. **None of them is taught a second format.**
- **There is no second canonical JSON.** A per-format artifact type would mean a per-format
  fingerprint, a per-format verifier path and a per-format bug, which is the outcome one serializer
  exists to prevent.

`04-ARCHITECTURE.md` §6 already books this in and names its precondition:

> **Second format (v2)** | A new `NativeLocator` variant + adapter profile + fixtures + inspection
> behaviour | None — **provided `engine-grounding` never learned about pages**

**Verified at v2-S1: the precondition holds, and it points at the wrong crate.**
`engine-grounding` never learned what a page *is* — it reads no locator
(`engine_grounding_has_no_pdf_concept` fails if it so much as mentions `NativeLocator`), derives no
geometry, and addresses pages by id. `project()`'s own check that `node.parent` names a declared
page is a **re-assertion of an invariant `seal` already guarantees**, not independent knowledge:
`project()` can never be handed a page-less representation, because one cannot be constructed.

The assumption that every node has a page parent lives in **`engine-core`**. That is permitted by
§1's M5 line — a contract invariant is not format machinery, and nothing there can parse anything —
but it is the sentence v2 has to revisit, and it is the finding §5 records. Pinned by
`crates/engine-grounding/tests/page_less_source.rs`.

### Where the office crate lives, and why it did not exist until v2-S2

`04-ARCHITECTURE.md` already named it and already refused to create it early:

> **A fifth crate before a second format is speculative structure.** Revisit only when office or OCR
> needs a real home — **`engine-office`**, `engine-ocr` — and not before.

So: the name is `engine-office`, its rules are the existing table's (`engine-core` must never learn
what a DOCX is, exactly as it never learned what a PDF is), and S0 did not create it — **v2-S2 did**,
because a second format is what makes it stop being speculative and a scope document is not.

**One new crate arrived in the lock with it: `quick-xml`.** ZIP is read inside `engine-office` over
`flate2`, which the graph already carried, because the `zip` crate drags twelve transitives —
including a *compressor* — to save a hundred lines of central-directory reading. XML is the opposite
call and is not hand-rolled: entities, namespaces, CDATA and encodings are where a hand-rolled
reader silently gets **text** wrong, and text is the evidence. Both halves are v1.2-S1's reasoning
about an MCP framework, applied twice with opposite answers.

**And nothing has arrived since.** XLSX, PPTX, ODT, ODS, ODP, RTF and EPUB each added a reader and
**no dependency**: no `zip`, no `zopfli`, no `calamine`, no ODF library, no EPUB crate, no HTML5
parser and no LibreOffice. v2-S5 is the sharpest case, because the shortest path to an ODT is a
converter and `06-STEAL-REFUSE.md` L30 is what refuses it — and v2-S9 is the second sharpest, an
EPUB being a container of XHTML that a general-purpose HTML5 parser would have read for the cost of
a new dependency and a second definition of what counts as text.

## 5. The contract question, decided at S1

**`ethos.grounding.v1` is a PDF schema today, and a DOCX cannot enter it.** Three independent walls,
read off `crates/engine-grounding/schemas/ethos-grounding-source.schema.json`:

| # | the schema says | a DOCX has |
| --- | --- | --- |
| 1 | `source.media_type` is `{"const": "application/pdf"}` | a different media type — it cannot even be *named* as the source |
| 2 | `element.required` is `["id", "page", "bbox", "kind"]` | no page and no box |
| 3 | `page.required` is `["id", "index", "width", "height", "rotation"]`, all integers ≥ 1 | no page geometry to put there |

The v2 gate says *"a DOCX quote and an XLSX cell both **ground**"*. Those three walls mean the word
`ground` in that sentence is **undefined for a page-less format**.

### Decided at v2-S1: (b) — `ethos.grounding.v1` stays PDF-only

**No page-less source shape, no optional `page`, no forked schema in this repository.**

The alternative — **(a)**, revising the artifact so it can name a page-less source — is a change to
the **verifier's** contract, and `07-VERIFY-BOUNDARY.md` is why it cannot be made here.
`engine-cli/tests/oracle.rs` agrees with the pinned Ethos CLI on this exact schema, so an
engine-only revision would produce artifacts the verifier does not speak while both still called
themselves `ethos.grounding.v1`. **(a) is therefore blocked on an Ethos-side revision** — recorded
as owned elsewhere, not refused. Adding an optional `page` to the engine's copy would be the lying
artifact v1.2-S5 refused for loose boxes, wearing a different field name.

Pinned by `crates/engine-grounding/tests/page_less_source.rs`, which asserts all three walls. Relax
one and S1 is reopened deliberately.

### And the measurement that resized S2

S0 wrote the sentence above assuming the page assumption lived in `ethos.grounding.v1` — so that
deciding (b) would leave v2's gate reachable *"at the representation level"*. **That was wrong, and
S1 measured it.**

`DocumentRepresentation::seal` refuses a node whose parent is not a declared page. The check is in
`check_structure`, which runs on **both** construction paths, so it cannot be reached around by
deserializing instead of sealing:

> node `s1` names parent page `p1`, which is not a declared page

**A page-less document cannot become a representation at all**, let alone a grounding artifact. So
§3's statement that the empty `pages` vector spells *"this document has no pages"* is true of the
**type** and false of the **invariant**: today the empty vector is only legal for a document with no
nodes either.

That does not change the decision — (b) is still right, and for the reason given. It changes **whose
problem the gate is**: reaching *"a DOCX quote grounds"* is upstream of grounding, in the
representation's own page-parent invariant, and that is a v2 design decision about the IR rather
than a schema question. `15-V2-MILESTONES.md` S2 carries it.

**Neither reading permits inventing a page to satisfy either constraint** — that is the third option
and the only one forbidden outright.

## 6. What v2 is

| | |
| --- | --- |
| **Formats, not features** | new readers behind the existing contract, not new contracts |
| **One IR** | a format projects into the representation; every downstream path is unchanged |
| **Structural locators** | the file's own addresses, never a rendering's |
| **Clean-room** | Anydoc's ideas (`06-STEAL-REFUSE.md` A-rows), never Anydoc's code, never a JVM |
| **Bounded by `deny.toml`** | no renderer, no network, no AGPL; a format that needs one does not ship in this version |

## 7. What v2 is not

- **Not a PDF change.** No detector moves, no rule id moves, no table number moves because an office
  format arrived. `ruled-rects-v2`, `stroke-ruled-v1`, `unruled-align-v1`, `markdown-blocks-v2` and
  `html-blocks-v2` keep their ids and their numbers.
- **Not v1 closing.** Macro cell-slot F1 is **64‰**, and 489‰ is a published comparator rather than
  a live floor — the chase is parked, which is not a pass. `08-V1-SCOPE.md`, `09-V1-MILESTONES.md`
  and `table-gate-v1.md` still govern, and no v2 slice may be cited against them.
- **Not a conversion pipeline.** L30 is a refusal, not a fallback. No LibreOffice, no headless
  browser, no PDF round-trip, in the default build or behind a flag.
- **Not a wrap of Anydoc.** `06-STEAL-REFUSE.md`'s A-rows are ideas — the error taxonomy,
  content-based format detection, mutation-plus-fuzz testing, the `CellSlot` model, A14's declared
  erasure. Wrapping the implementation, cloning the repo as a coding source, or taking the JVM
  (**O28**) are all refused, and the reason is the one that refused wrapping a competitor as the
  grounded PDF core.
- **Not 14 formats.** **A1** — Anydoc's coverage breadth — is the horizon this version aims at, not
  a checklist this version implements. The gate names two formats. The rest are split out of the
  remaining-formats row only when the cost of the next one has been **measured** rather than
  guessed: S4 measured that a third OOXML format was cheap, and S5 measured that the first
  non-OOXML format was not. On that finding the row split again on paper before any of it was
  written — **S6 is ODS alone**, and **S7 was the one row that was left**: ODP, RTF, EPUB, CSV. S6
  shipped ODS and did **not** acquire a second format while its reader was open, which is what
  writing the split first bought. **S7 then spent S6's measurement**: ODP is inside the ODF family
  and inherited the container, the manifest check and the whole allowlist, while RTF, EPUB and CSV
  inherit none of it — so ODP left the row and **S8 became RTF, EPUB, CSV**. S8 then measured that
  RTF inherits nothing at all — not the container, not the XML reader, not the allowlist, not the
  shape of the locator — and left with **S9 as EPUB, CSV**. S9 then measured that EPUB inherits the
  ZIP reader and the XML plumbing and almost nothing else, and left **S10 as CSV** — a format whose
  cost is not a reader but a **detector**, because comma-separated text cannot be told from prose
  without one. **S10 then measured that the detector cannot be bought at any price this contract
  can pay**, and shipped the refusal instead: the parse would fabricate nothing, but
  `SourceIdentity.media_type` would claim a type nobody measured, and `SourceIdentity` has two
  fields and no room to record that a type was asserted rather than read. That closes the row —
  eight formats read, one argued refusal, and every split from S4 to S9 made against a
  measurement.
- **Not auto-tagging, assist, or OCR.** v2.2, v3 and v4 have their own rows and their own gates.
- **Not permission to reopen v1.2.** S5's LiteParse refusal is settled: no adapter, no mapper, and
  no refusing CLI, pinned by `crates/engine-grounding/tests/liteparse_refusal.rs`. Its two walls are
  producer identity and undeclared loose boxes — **not** the coordinate hazard everyone predicted —
  and widening `ethos.grounding.v1` to accommodate LiteParse is not a thing v2 does under cover of
  §5's question.

## 8. Identity

A new format is a new **value**, not a new mechanism (`04-ARCHITECTURE.md` §6). The profile is where
that shows:

- **A format gets its own adapter profile**, as §5.1 requires — so an artifact from a DOCX and an
  artifact from a PDF are **provably non-comparable**, the same way `profile_sha256` already
  separates an OCR'd page from a born-digital one.
- **No `capabilities.docx`, no `capabilities.office`** on the *PDF* profile. `01-CONTRACT.md`'s
  capability discipline is that a capability describes what a profile can read out of a document it
  actually reads; a flag about another format on every PDF artifact is a flag no consumer can act
  on. v1.2 refused `capabilities.mcp`, `capabilities.python`, `capabilities.node`,
  `capabilities.langchain` and `capabilities.liteparse` on that reasoning, and it holds here.
- **The version moves per slice**, as every version has since v0.1, and the profile hash moves with
  `parser_version` at minimum.

## 9. Standing rules, carried forward

Unchanged from `08-V1-SCOPE.md` §6, `10-V11-SCOPE.md` §8 and `12-V12-SCOPE.md` §8, and repeated
because a second format is where they get bent:

1. **No public confidence field** — including on an office node or in a summary string
2. **No box, and no role, derived from a font size** — and in v2, **no page derived from anything**
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count
   (Anydoc's **A14**, declared erasure, is the v2 target for exactly this)
4. **No invented coordinate, identifier, fingerprint or pagination** — pagination is now the leading
   case rather than the footnote
5. **A gap is never presented as a success** — a quote that cannot bind fails closed
6. **Byte-identity across runs**, and across formats: two runs over one DOCX produce one artifact
