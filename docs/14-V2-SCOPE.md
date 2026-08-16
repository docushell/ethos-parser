# 14 — v2 scope: office formats

**Status:** scope authority for v2 · **Slice detail:** `15-V2-MILESTONES.md`
**This is the code-review map for v2.** Every v2 PR belongs to exactly one slice.

**v2 is scoped, not started.** No parser for any office format exists in this tree, and S0 is these
two documents.

**v1 is not done.** Its gate — table-cell accuracy above 0.489 — is measured and **missed at 64‰**
(`table-gate-v1.md`, `09-V1-MILESTONES.md` S7). **v1.1 is complete** at 0.14.1 and **v1.2 is
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
| **Structural** | the native locator names things the file itself contains — a paragraph, a run, a cell, a sheet | a page index, a bbox, or a line number derived from layout |
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
- **Pages.** `RepresentationPayload.pages` is a `Vec<PageRecord>`. A format without pages carries
  the empty vector. **An empty `pages` array is the spelling of "this document has no pages"**, and
  a non-empty one on a DOCX is the defect this law exists to catch.
- **The locator.** A new union variant, per §5.1. A `DocxLocator` that contains a page number is the
  same violation as a `bbox`, wearing a field name.
- **The dependency graph.** `deny.toml` is the standing proof for the third obligation: a renderer
  is a dependency, and one that arrives shows up there before it shows up in a review.

S1+ turn these into tests. S0's job is that the sentences exist **before** the first parser that
could break them — the pattern `12-V12-SCOPE.md` §3 set for the handle law, for the same reason.

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

That precondition is **not yet verified** and S1 must verify it rather than assume it. Today
`engine-grounding` passes `PageRecord`s through from the representation into `ethos.grounding.v1`,
which is a weaker position than "never learned about pages" implies. Whether that is pass-through
or knowledge is the first question §5 asks.

### Where an office crate would live, and why it does not exist yet

`04-ARCHITECTURE.md` already named it and already refused to create it early:

> **A fifth crate before a second format is speculative structure.** Revisit only when office or OCR
> needs a real home — **`engine-office`**, `engine-ocr` — and not before.

So: the name is `engine-office`, its rules are the existing table's (`engine-core` must never learn
what a DOCX is, exactly as it never learned what a PDF is), and **S0 does not create it.** A second
format is what makes it stop being speculative; a scope document is not.

## 5. The open contract question, posed and not answered

**`ethos.grounding.v1` is a PDF schema today, and a DOCX cannot enter it.** Three independent walls,
read off `crates/engine-grounding/schemas/ethos-grounding-source.schema.json`:

| # | the schema says | a DOCX has |
| --- | --- | --- |
| 1 | `source.media_type` is `{"const": "application/pdf"}` | a different media type — it cannot even be *named* as the source |
| 2 | `element.required` is `["id", "page", "bbox", "kind"]` | no page and no box |
| 3 | `page.required` is `["id", "index", "width", "height", "rotation"]`, all integers ≥ 1 | no page geometry to put there |

The v2 gate says *"a DOCX quote and an XLSX cell both **ground**"*. Those three walls mean the word
`ground` in that sentence is **undefined for a page-less format**, and it has exactly two honest
readings:

- **(a) The schema revises.** `ethos.grounding.v1` gains a page-less source shape — a decision with a
  written rationale, a version, and tests, made in the open. It is a change to the **verifier's**
  contract, so it is not an adapter's business and not a thing to do quietly inside a parser slice.
- **(b) Grounding stays PDF-only**, and v2's gate is met at the **representation** level: a DOCX
  quote resolves to a node through the same fingerprint-checked handle path `node_get` already uses,
  and `ethos.grounding.v1` continues to mean *"a box on a page in a PDF"*.

**S0 does not choose.** It records that the choice exists, that it is load-bearing for the gate's own
wording, and that **neither reading permits inventing a page to satisfy the schema** — which is the
third option and the only one that is forbidden outright. `15-V2-MILESTONES.md` S1 is where the
decision is made, before any reader exists whose output would depend on it.

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
- **Not v1 closing.** Macro cell-slot F1 is **64‰** against a 489‰ floor. `08-V1-SCOPE.md`,
  `09-V1-MILESTONES.md` and `table-gate-v1.md` still govern, and no v2 slice may be cited against
  them.
- **Not a conversion pipeline.** L30 is a refusal, not a fallback. No LibreOffice, no headless
  browser, no PDF round-trip, in the default build or behind a flag.
- **Not a wrap of Anydoc.** `06-STEAL-REFUSE.md`'s A-rows are ideas — the error taxonomy,
  content-based format detection, mutation-plus-fuzz testing, the `CellSlot` model, A14's declared
  erasure. Wrapping the implementation, cloning the repo as a coding source, or taking the JVM
  (**O28**) are all refused, and the reason is the one that refused wrapping a competitor as the
  grounded PDF core.
- **Not 14 formats.** **A1** — Anydoc's coverage breadth — is the horizon this version aims at, not
  a checklist this version implements. The gate names two formats. The rest are parked in `15` as
  one row until two have shipped and the cost of the third is measured rather than guessed.
- **Not OCR, auto-tagging, or assist.** v2.1, v2.2 and v3 have their own rows and their own gates.
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
