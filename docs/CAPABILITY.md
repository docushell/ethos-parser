# What this engine can and cannot do

**These tables describe 0.41.0.** When the workspace version moves, this page moves with it or it
is wrong.

This is the honest inventory — the answer to *"what does ethos-parser actually do today?"* It is not
the roadmap and not a sales pitch. If something is not in the **Can** table, do not promise it.

---

## Can

Everything below runs locally, with no network and no renderer.

| Area | What works | Where |
| --- | --- | --- |
| **Classify** | Sorts a PDF into named reason codes on two axes — does it need OCR, is the layout hard — with per-page counts and three distinct exit codes. No confidence score anywhere | `classify` |
| **Extract (PDF)** | Text runs from a born-digital PDF, each with a locator back into the source bytes, and an ink box only where font metrics actually measured one | `extract` |
| **Extract (office)** | DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF and EPUB into the same record, each addressed the way its own format addresses itself: `{part, paragraph, run}` for Word, `{part, sheet, row, column}` for a workbook, `{part, shape, paragraph, run}` for a slide, and so on | `extract` |
| **Office pages** | `pages` is always `[]` for office formats. No spreadsheet print range, slide, soft page break or RTF `\page` is ever turned into a page record | — |
| **Grounding** | Emits `ethos.grounding.v1` in both shapes: the paginated 1.0.0 shape for PDFs, and the page-less 1.1.0 shape for office documents, where each element carries its native locator instead of geometry. A DOCX quote extracted here, grounded here, and checked by the sibling verifier comes back `grounded` | `ground` |
| **Grounding check** | A validator that agrees byte-for-byte with `ethos grounding check`. Measured against 15 Ethos-owned fixtures; 12 reach an artifact and are compared, and the rest fail closed rather than being skipped | `grounding-check` |
| **Markdown / HTML** | Both artifacts carry an anchor map — every byte is either `source` or `syntax` — plus a census of what did not make it. Both are projected from the representation, so office documents project too | `markdown`, `html` |
| **Verify** | Shells out to the pinned Ethos CLI and relays its bytes verbatim. The engine does not verify anything itself | `verify` |
| **Overlay** | An annotated copy of a PDF showing what was detected, including a note counting what has **no** box to draw | `overlay` |
| **Adapters** | MCP over stdio, a Python SDK, a Node SDK, and LangChain tools over both. Every locator is minted by the engine, handed back opaque, and re-validated on the way in. No adapter takes a geometry argument | `mcp`, `packages/` |
| **Format detection** | Decided by reading bytes at offset 0, never by file name or extension. Bytes that state no known format are refused by naming what was looked for, and nothing else. A `.csv`, a letter and a log line all get byte-identical stderr | `extract` |
| **Tables** | Row/column model with spans and cell occupancy, four named detection rules (`ruled-rects-v3`, `unruled-align-v1`, `stroke-ruled-v1`, `tagged-tables-v1`), and a cross-check between what the geometry says and what the document's own tags say. Where the two disagree structurally, the rule refuses the grid rather than emitting it | `extract` |
| **Fabrication** | **0** across the twelve-document labelled set. Measured every run, not asserted | `table-gate-v1.md` |
| **Table accuracy** | Stated as a capability, not an average: the engine reads the tables a document **declares** (157 of 172 gold tables; combined cell-slot recall **502‰**) and **detects** ruled tables where the producer drew them (geometric macro F1 **70‰**, band 0‰–590‰, median 0‰ — ten of twelve score zero). It emits nothing where neither holds | `table-gate-v1.md` |
| **Fuzzing** | Three `cargo-fuzz` targets covering the PDF entry points and the office router. One campaign ran 3.8M executions under AddressSanitizer and found nothing. Mutation testing damages every PDF fixture six ways and every office package twelve | `fuzz/` |
| **Layout regions** | Every text run says which region of its page the reading-order cut placed it in, 1-based in reading order, under `gutter-columns-v2`. **A region is a column band**: runs stacked within one column share a region however many paragraphs separate them, and the field is absent wherever the cut made no division — which is most pages. It is `Computed` geometry and never a role: no heading, paragraph or section is read from it | `extract` |
| **Determinism** | Two runs over one document produce identical bytes, in every format | CI |

---

## Cannot

Some of these are *not yet*. Others are refusals no version reverses. The middle column says which.

| Claim | Kind | Why |
| --- | --- | --- |
| **"v1 is complete"** | done | Closed on decision #18: v1 states the table capability and the band rather than a single macro. Fabrication is 0, the method is pinned in `table-gate-v1.md`, and the geometric chase stays parked |
| **OCR, or a scan read as extracted text** | not yet — v4 | Recognition is a different class of derivation and needs its own trust ladder. An OCR fingerprint must be provably incomparable with a born-digital parse before that lane can exist |
| **CSV** | refusal, with named reopening conditions | A CSV parse would not fabricate content — every character would be real. Exactly one field would be false: the record would claim the file *is* a CSV when nobody measured that, and there is nowhere in the record to mark a field as asserted rather than measured. Reopens if either the record gains that distinction, or someone builds a format predicate with a measured false-positive rate |
| **Converting office files to PDF to get pages** | refusal | It invents pagination. A DOCX has no pages, and a page on a Word citation measures the printer rather than the document. Not as a fallback, not behind a flag |
| **An EPUB's `page-list` as pagination** | refusal | It names pages of a *print* edition. A page record is a page with a width and a height; a publisher's label about someone else's paper has no geometry to validate against |
| **A paragraph boundary on an untagged page** | not yet | The reading-order cut opens a region only where it finds a **vertical** gutter, so it separates columns and not paragraphs. Within a column the cut's finer leaves fall on individual lines — body text set with uniform leading ties every baseline gap — so numbering them would put a line ordinal on the wire under a block's name. Paragraph structure comes from the tag tree or not at all, and `region` is deliberately not a substitute |
| **A confidence score, grade, or `is_good` field** | refusal | Enforced by a CI grep that fails the build |
| **A verdict about a claim or a citation** | refusal, permanent | The engine owns extraction and grounding. Verification belongs to the verifier, and a second verification implementation is exactly the failure this project is arranged to prevent |
| **Any AGPL dependency, or a JVM** | refusal | `cargo deny` enforces the licence rule, and CI proves the proof by flipping a crate to AGPL and requiring rejection |
| **A benchmark table comparing this to other parsers** | refusal | Headline numbers in this space are publisher-owned. The same tool has scored 0.000 and 0.693 on tables under two publishers, differing only by invocation flags |
| **Page rasters / screenshots** | not yet | Needs a PDF renderer, and this build depends on no C++ stack and no AGPL code by decision |
| **Office embedded images, audio and video** | not yet | Counted and declared by every reader, never decoded. A media part has no text, no address a citation could bind to, and no geometry |
| **RTF tables, pictures, embedded objects and fields** | not yet | `\cell` and `\row` are recorded as a paragraph terminator, which is a fact the file states, and never as a table, because no detector ran |
| **Decoding an RTF `\'hh` byte above 0x7F** | refusal | Its meaning depends on a code page this reader does not read. Emitting a Latin-1 character instead would be mojibake dressed up as success. The byte is counted instead |
| **EPUB stylesheets, scripts, media overlays and `alt` text** | not yet | No stylesheet is read, so text hidden by `display: none` is still in the record and generated text is not. Attributes are never read as text |
| **A line break between two CJK characters in an EPUB** | not yet | CSS removes it; this reader turns it into a space. The correct rule needs a computed `white-space` value and the scripts on both sides |
| **`.odg` drawings and ODF templates** | not yet | Format detection is exact rather than prefixed, so a drawing is refused by name rather than producing a plausible artifact for a format nobody decided to support |
| **Speaker notes, slide masters and layouts as slide text** | refusal | Notes are a second stream and a master's text belongs to every slide. Splicing either in would be a silent *addition* to the evidence, which is worse than a silent drop because a consumer cannot tell it apart from the real thing. Both are counted instead |
| **Text in ODP shapes other than frames and custom shapes** | not yet | Those two are what a presentation writes for text. The set is read off the specification and kept narrow, because no corpus of real `.odp` files was available to measure which others matter |

---

## The rule underneath both tables

Everything the engine could not do is **stated** — as a named limitation, a typed absence, or a
counted bucket. A gap is never presented as a success. When something moves from **Cannot** to
**Can**, it moves with a measurement, not with a sentence.
