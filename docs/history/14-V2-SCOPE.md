# 14 — v2 scope: office formats

**Scope authority for v2.** Slice detail is in [`15-V2-MILESTONES.md`](15-V2-MILESTONES.md), and
every v2 PR belongs to exactly one slice.

**v2 reads eight formats — DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF and EPUB — with CSV an argued
refusal rather than a reader. The format row is closed and the gate is met.** Both questions that
stood between this repository and the gate were settled by the owner as decisions #16 and #17, and
the last open technical question — whether the ZIP reader should verify the checksum it ignored —
was answered by measurement and shipped.

**v1 is not done.** Its table number is measured and missed. v2 is the next roadmap row and it was
scoped because the owner asked for it, **not** because that gate cleared. Nothing here closes v1.

**Superseded by decision #18 (2026-08-30):** v1 closed on four capability clauses and a published
band — the engine reads the tables a document declares, detects ruled tables where the producer
drew the rules, emits nothing where neither holds, and fabricates nothing — not on the macro. The
paragraph above was true when written and did not survive the decision; it is kept as written. See
[`00-NORTH-STAR.md`](../00-NORTH-STAR.md) row 18, which is where v1's status is stated.

---

## 1. The one sentence

**A DOCX quote binds to something the document actually contains, or it does not bind at all.**

Everything below is machinery for that sentence. Its shadow is the sentence v2 exists to refuse: *a
DOCX quote binds to page 3.*

## 2. Why this version exists — and the one thing that would ruin it

The gate is stated in the north star and the roadmap in the same words:

> A DOCX quote and an XLSX cell both ground; **no synthesised pages**

The second clause is not a footnote — it is the whole hazard, and [`06-STEAL-REFUSE.md`](../06-STEAL-REFUSE.md)
already refuses the shortest path to it (row **L30**): converting office files to PDF **invents
pagination**, and that is a refusal rather than a fallback.

**A DOCX has no page.** It has a paragraph order, a run order and a style. Where a page falls is a
decision a *renderer* makes from a font stack, a printer profile and a paper size, and it changes
when any of those change. So a page on a Word citation is not a measurement of the document — it is a
measurement of the machine that printed it, and **it will look exactly like a locator, because it is
shaped like one.** That is the same failure v1.2 spent a whole version preventing for MCP, arriving
through a converter instead of a model.

**On the gate's verb.** Read literally, "ground" contradicts the same sentence's second half —
grounding requires pages, and *no synthesised pages* is right there. Decision #16 settles it: **ground
means bind.** Each quote resolves to an address the file itself states. The literal reading was later
made possible from the other side, when the verifier's schema gained a page-less shape and this engine
took it — but the verb did not change; the wire caught up to it.

## 3. The no-synthesised-pages law

**A v2 locator addresses the document's own structure. It never addresses a rendering of it.**

This is not new policy — the contract wrote it down in v0, before a second format existed: a rendered
page or box is never substituted for the native source address unless an approved format profile
declares that rendering authoritative, and no such profile exists.

v2's job is to add locator variants and **not** to create such a profile through a side door. Three
obligations, and a format satisfies all three or it does not ship:

| | Obligation | What it forbids |
| --- | --- | --- |
| **Structural** | The locator names things the file itself contains — a paragraph, a run, a cell, a sheet | A page index, a box, or a line number derived from layout — **including one the file contains**, because the obligation is about what the number *is*, not where it was found |
| **Absent, not invented** | A format with no geometry says so in the type system | A zero box, a page-sized box, or a page record sized to A4 because A4 is common |
| **No renderer in the graph** | Nothing converts, prints, paginates or lays out to obtain a locator | LibreOffice, a headless browser, a PDF round trip, a layout engine, "A4 at 72 DPI" |

### What actually checks it

- **Geometry.** A page-less node's geometry is `NotApplicableToKind` — absence that is *correct*
  rather than a gap — and it deliberately does not count toward the limitation total. It is not
  `NotReportedByReader`, which means the reader tried and could not.
- **Pages.** An **empty `pages` array is the spelling of "this document has no pages"**, and a
  non-empty one on a page-less document is refused by name as invented pagination.
- **The locator.** A new union variant per format, with unknown fields denied so one cannot arrive
  quietly. Each format's test refuses the field somebody would actually reach for in *that* format —
  `soft_page_break` for ODT, `slide_number` for a deck, where a person really does say "it's on slide
  12".
- **Geometry again, stronger than a convention.** The structure check **refuses a measured box on a
  page-less node**. A box is validated against the page containing it, so a node with no page has
  nothing to validate against, and **a rectangle nobody can check is the fabrication this law exists
  to refuse.** The office reader could not emit one even if a later edit tried.
- **The dependency graph.** `deny.toml` is the standing proof for the third obligation: a renderer is
  a dependency, and one that arrives shows up there before it shows up in a review.

### The seven shapes a "page" can take, and why the last four cost something

- **A DOCX** has none until a renderer invents one.
- **A spreadsheet's** is a print artefact — column widths, print areas, page breaks all describe a
  printing rather than the file.
- **A slide** is a real, discrete, countable thing the package contains. It is still a **part**: the
  locator carries no slide number, because a size nothing measured and a position in a display-order
  list are not a page.
- **An ODT states its own page breaks.** `content.xml` contains a soft page break element and the
  styles contain a page width, so a page record would need almost no arithmetic. Refused anyway: a
  soft page break records where the *producing application's* layout fell, and it moves when the font
  stack, paper size or producer changes.
- **An ODP removes the last of the arithmetic** and is the sharpest refusal here. A presentation lists
  draw pages — discrete, ordered, named, counted out loud by anybody describing a deck — with a master
  page's width beside them. A page record needed **nothing computed at all**, which had never been
  true before. Refused on L30's own three words: *it invents pagination*. A draw page is part of the
  presentation's **structure**, not a page this engine measured.
- **An RTF says the word out loud** — a page break and a paper width in the plainest language any of
  these formats use. Both are the producing application's print arithmetic. RTF is also the first
  format with **no container at all**, which the second obligation decided rather than the first: a
  constant part name would have let the invariant run unchanged and would have been a string the
  document does not contain, so the invariant grew a rule instead.
- **An EPUB names the pages of a print edition** in its navigation document — real identifiers the
  file writes down, which is why this is where the law had to be *argued* rather than applied.
  Refused because a page record is a page with a **width and a height**, and a publisher's label about
  somebody else's paper has no geometry for anything to be validated against.

## 4. One record, one serializer

- A new format projects into **the representation this engine already emits** — same artifact type,
  same canonical JSON, same fingerprint discipline.
- The existing paths then consume it unchanged: `ground`, `markdown`, `html`, `mcp`, both SDKs and
  the LangChain tools. **None of them is taught a second format.**
- **There is no second canonical JSON.** A per-format artifact type would mean a per-format
  fingerprint, a per-format verifier path and a per-format bug — the outcome one serializer exists to
  prevent.

**The precondition held, and it pointed at a different crate than expected.** The projection never
learned what a page *is*: it reads no locator, derives no geometry, and addresses pages by id. The
page assumption lived in `ethos-parser-core`, where the structure check refused a node whose parent
was not a declared page — so a page-less document could not become a representation at all. That is
permitted (a contract invariant is not format machinery), and it is the sentence v2 had to revisit.

### Where the office crate lives

The architecture named `ethos-parser-office` and refused to create it early: **a fifth crate before a
second format is speculative structure.** A second format is what makes it stop being speculative,
and a scope document is not.

**One new dependency arrived with it: an XML reader.** ZIP is read in-crate over the compression
library the graph already carried, because a ZIP crate drags a dozen transitives — including a
*compressor* — to save a hundred lines of central-directory reading. XML is the opposite call and is
deliberately not hand-rolled: entities, namespaces, CDATA and encodings are exactly where a
hand-rolled reader silently gets **text** wrong, and text is the evidence.

**Nothing has arrived since.** Seven more formats added seven readers and **no dependency** — no ZIP
crate, no spreadsheet library, no ODF library, no EPUB crate, no HTML5 parser, no LibreOffice. ODT is
the sharpest case, because the shortest path to one is a converter; EPUB is the second sharpest, being
a container of XHTML that a general-purpose HTML5 parser would have read for the cost of a new
dependency and a second definition of what counts as text.

## 5. The grounding question, decided at S1

**`ethos.grounding.v1` was a PDF schema, and a DOCX could not enter it.** Three independent walls:

| # | The schema says | A DOCX has |
| --- | --- | --- |
| 1 | The media type is a constant, `application/pdf` | A different media type — it cannot even be *named* as the source |
| 2 | Every element requires a page and a box | Neither |
| 3 | Every page requires integer width, height and rotation | No page geometry to put there |

**Decided at S1: the schema stays PDF-only in this repository.** No page-less shape, no optional
page, no forked schema here. The alternative — revising the artifact so it can name a page-less
source — is a change to the **verifier's** contract, and an engine-only revision would produce
artifacts the verifier does not speak while both still called themselves `ethos.grounding.v1`. So it
was recorded as **blocked on a revision owned elsewhere, not refused.**

**That revision later landed**, and 0.39.0 took it: office representations now project the page-less
shape, with no synthesised page anywhere. The decision above is what kept the interim honest —
adding an optional page to this repository's own copy would have been the same lying artifact the
LiteParse adapter was refused for, wearing a different field name.

**Neither reading ever permitted inventing a page**, which is the third option and the only one
forbidden outright.

## 6. What v2 is

| | |
| --- | --- |
| **Formats, not features** | New readers behind the existing contract, not new contracts |
| **One record** | A format projects into the representation; every downstream path is unchanged |
| **Structural locators** | The file's own addresses, never a rendering's |
| **Clean-room** | Anydoc's *ideas*, never its code, and never a JVM |
| **Bounded by the dependency policy** | No renderer, no network, no AGPL. A format that needs one does not ship in this version |

## 7. What v2 is not

- **Not a PDF change.** No detector moves, no rule id moves, and no table number moves because an
  office format arrived.
- **Not v1 closing.** The chase is parked, which is not a pass, and no v2 slice may be cited against
  v1's own documents.
- **Not a conversion pipeline.** L30 is a refusal, not a fallback. No LibreOffice, no headless
  browser, no PDF round trip — not in the default build and not behind a flag.
- **Not a wrap of Anydoc.** The error taxonomy, content-based format detection, mutation-plus-fuzz
  testing, the cell model and declared erasure are *ideas*. Wrapping the implementation, cloning the
  repo as a coding source, or taking the JVM are all refused.
- **Not 14 formats.** Breadth is the horizon this version aims at, not a checklist it implements. The
  gate names two formats. **Each additional format was split out only once the cost of the next one
  had been measured rather than guessed** — a third OOXML format was cheap; the first non-OOXML one
  was not; ODP inherited the whole ODF container while RTF inherited nothing at all; EPUB inherited
  the ZIP reader and the XML plumbing and almost nothing else. That discipline is what stopped any
  slice from quietly acquiring a second format while its reader was open.
- **Not CSV.** Its cost is not a reader but a **detector**, because comma-separated text cannot be
  told from prose without one — and S10 measured that the detector cannot be bought at any price this
  contract can pay. The parse would fabricate nothing, but the record would claim a media type nobody
  measured, and there is no room in it to say a type was asserted rather than read.
- **Not auto-tagging, assist, or OCR.** Those have their own rows and their own gates.
- **Not permission to reopen v1.2.** The LiteParse refusal is settled and pinned by a test.

## 8. Identity

A new format is a new **value**, not a new mechanism. The profile is where that shows:

- **A format gets its own adapter profile**, so an artifact from a DOCX and an artifact from a PDF are
  **provably non-comparable** — the same way the profile hash already separates an OCR'd page from a
  born-digital one.
- **No office capability flag on the PDF profile.** A capability describes what a profile can read out
  of a document it actually reads; a flag about another format on every PDF artifact is a flag no
  consumer can act on. v1.2 refused adapter capabilities on the same reasoning.
- **The version moves per slice**, and the profile hash moves with it at minimum.

## 9. Standing rules, carried forward

Repeated because a second format is where they get bent:

1. **No public confidence field** — including on an office node or in a summary string.
2. **No box, and no role, derived from a font size** — and in v2, **no page derived from anything**.
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count.
4. **No invented coordinate, identifier, fingerprint or page number** — pagination is now the leading
   case rather than the footnote.
5. **A gap is never presented as a success** — a quote that cannot bind fails closed.
6. **Byte identity across runs, and across formats** — two runs over one DOCX produce one artifact.
