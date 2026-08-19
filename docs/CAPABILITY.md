# What this engine can and cannot do

**Version in this file, not in its name:** these two tables describe **0.26.0**. When the workspace
version moves, this page moves with it or it is wrong.

This is the honest inventory, for the question *"what does ethos-engine actually do today?"* It is
not the north star (`00-NORTH-STAR.md`), not the roadmap (`02-ROADMAP.md`), and not a claim.
Anything not in the **Can** table is not something to promise.

---

## Can

Everything below runs locally, in this tree, with no network and no renderer.

| Area | What works | Where |
| --- | --- | --- |
| **Classify** | A PDF is classified into reason codes on two orthogonal axes (OCR-need, layout-hard), with per-page counts and three distinct exit codes. No confidence float anywhere | `engine classify` |
| **Extract (PDF)** | A born-digital PDF's text runs, each with a `NativeLocator` back into the source bytes, and an ink box only where font metrics measured one | `engine extract` |
| **Grounding emit** | `ethos.grounding.v1` — **for PDF only**. `source.media_type` is a `const` of `application/pdf` in the schema, and the projector refuses anything else by name | `engine ground` |
| **Grounding check** | A validator that agrees byte-identically with `ethos grounding check` on `structure`, `source_binding`, `representation_sha256` and `counts`. The corpus is 15 Ethos-owned fixtures; **12 reach an artifact and are compared** (`ORACLE_AGREED_COUNT`), and the rest fail closed rather than being skipped | `engine grounding-check` |
| **Markdown / HTML** | `ethos.markdown.v1` and `ethos.html.v1`, **each carrying its Anchor Map** — every byte is `source` or `syntax`, with a coverage census for what did not make it. Projected from **the representation**, so an office representation projects too — measured: `engine markdown` on a DOCX artifact exits 0 with a full map. It was designed and tested against PDF; the office path is unexercised, not refused | `engine markdown`, `engine html` |
| **Verify** | Shells out to the Ethos CLI as a *declared capability*, relaying its bytes verbatim, with the verifier pinned in the profile. The engine does not verify anything itself | `engine verify` |
| **Overlay** | An annotated PDF showing what was detected — including what has **no** box | `engine overlay` |
| **Adapters** | MCP over stdio (`extract`, `ground`, `node_get`), a Python SDK, a Node SDK, and LangChain tools over both. The engine **mints** every locator, hands it back **opaque**, and **re-validates** it on the way in. No adapter takes a geometry keyword argument | `engine mcp`, `packages/python`, `packages/node` |
| **Extract (office)** | **DOCX, XLSX, PPTX, ODT, ODS, ODP** into the same representation, with **structural** locators, every field a thing the file contains — `{part, paragraph, run}`, `{part, sheet, row, column}`, `{part, shape, paragraph, run}`, `{part, paragraph}`, `{part, table, row, column}`, `{part, draw_page, shape, paragraph}`. Each is `deny_unknown_fields`. The ODS row and column are **positions**, because OpenDocument writes no address at all; so is the ODP draw page, which is named for the `<draw:page>` element rather than for the page it resembles — see `15-V2-MILESTONES.md` S6 and S7 | `engine extract` |
| **Office pages** | **`pages: []`, always.** No office format synthesises a page — not a spreadsheet's print range, not a slide, not an ODT's or an ODS's `<text:soft-page-break/>`, and **not a `<draw:page>` with a master page's `fo:page-width` beside it**, which is the one case where a `PageRecord` needed no arithmetic at all and was refused anyway (row **L30**) | — |
| **Office names** | Where a file writes a name a person recognises — a sheet, a table, a draw page, a shape — it is carried **verbatim with entities resolved**. A name that could not be measured unique is on the **attributes**, never in the address: `<p:cNvPr id>` was measured across 18 real decks and is not, and ODF's optional `draw:name` was never measurable at all | `engine extract` |
| **Office grounding** | `engine ground` on an office artifact is a **named refusal** naming `application/pdf` and the law. It fails closed with empty stdout and exit 2 | `engine ground` |
| **Tables** | The row/column + span **model**, `CellSlot` occupancy, three named detection rules (`ruled-rects-v2`, `unruled-align-v1`, `stroke-ruled-v1`), and the geometric↔structural **cross-check**, whose disagreements go on the artifact | `engine extract` |
| **Fabrication** | **0** on the four-PDF labelled set — measured every run, not asserted | `table-gate-v1.md` |
| **Table accuracy** | Macro cell-slot **F1 = 64‰** on those four documents, by a method stated completely enough to recompute | `table-gate-v1.md` |
| **Determinism** | Two runs over one document produce identical bytes, across every format | CI |

---

## Cannot

Some of these are *not yet*; some are **refusals** that no version reverses. The column says which.

| Claim | Kind | Why |
| --- | --- | --- |
| **"v1 is complete"** | not yet | S7 is open. The table number is measured and missed at 64‰, and the **> 0.489 chase is parked, which is not a pass** (`00-NORTH-STAR.md` #10) |
| **"v2 is complete"** | not yet | S0–S7 are done. **S8 (RTF, EPUB, CSV) has not started** |
| **Beating 0.489, or quoting it as this engine's score** | **refusal** | 0.489 is a published ODL-local score on **their** corpus; 64‰ is this engine on **four tagged PDFs this repository owns**. Same unit, different exam. It is never published as ours, and it is no longer chased |
| **OCR, or a scan read as `Extracted`** | not yet — **v2.1**, under its own profile | Recognition is a different derivation class and a different trust ladder. An OCR fingerprint must be provably incomparable with a born-digital parse before the lane exists |
| **Wrapping LiteParse, OpenDataLoader, Anydoc or pdf-inspector as the grounded PDF core** | **refusal** | Reference-only, and their ideas are taken rather than their code (`06-STEAL-REFUSE.md`). The measured LiteParse adapter was **refused** and is pinned by a test |
| **LibreOffice → PDF for office formats** | **refusal** | **It invents pagination.** Pagination does not exist in a DOCX and must not be synthesised. Not a fallback, not behind a flag (row **L30**) |
| **Grounding a DOCX quote as `ethos.grounding.v1`** | **decided at v2-S1**, option (b): the schema stays PDF-only | Three walls — `source.media_type` is a `const` of `application/pdf`, every element requires a `page`, every page requires integer geometry. Widening the engine's copy would be a change to the **verifier's** contract, which `07-VERIFY-BOUNDARY.md` forbids making here; option (a) is **blocked on an Ethos-side revision, owned elsewhere rather than refused**. Pinned by `engine-grounding/tests/page_less_source.rs` (`14-V2-SCOPE.md` §5) |
| **RTF, EPUB, CSV** | not yet — **S8** | An unimplemented ODF sibling is refused **by name**, naming OpenDocument and the type the package declares about itself. v2-S6 closed the older defect where it was refused for having no `%PDF-` header. An `.epub` shares OCF's first-and-stored `mimetype` rule and is deliberately **not** told it is OpenDocument |
| **`.odg` drawings, and ODF templates** | not yet | Named separately from the row above because an `.odg`'s `content.xml` really *is* the `<draw:page>` vocabulary v2-S7's reader knows. Detection is **exact**, not prefixed, so a drawing is refused by name rather than producing a plausible artifact for a format nobody decided to support |
| **Text in an ODP drawing shape other than `<draw:frame>` / `<draw:custom-shape>`** | not yet | Those two are what a presentation writes for a text-bearing shape, and naming the set is what makes `OdpLocator::shape` a reproducible position. Text in a `<draw:rect>` or a `<draw:connector>` is **declared** (**A14**) rather than read — no corpus of real `.odp` files was available to measure which elements matter, so the set is read off the specification and kept narrow |
| **Speaker notes, masters and layouts as slide text** | **refusal** | Notes are a second stream and a master's text belongs to every slide, so splicing either in would be a silent **extra** rather than a silent drop — worse, because a consumer cannot tell it from evidence. Both are counted and named (**A14**), and a slide whose title lives only on its master reads as having none |
| **A public confidence float, score, grade, or `is_good`** | **refusal** | Workbench rule 9, `01-CONTRACT.md` §9, and a CI grep (`ci/forbidden-tokens.sh confidence`) that fails the build |
| **Any AGPL dependency** | **refusal** | Forced decision #14. `cargo deny` proves it, and CI proves the proof by flipping a crate to AGPL and requiring rejection |
| **A JVM anywhere** | **refusal** | OpenDataLoader's cost of entry, and the reason its capabilities cannot be borrowed wholesale |
| **A verdict about a claim, a document, or a citation** | **refusal, forever** | The engine owns L0–L2. L3 is the verifier's, and a second verification implementation is the failure this estate is arranged to prevent (`07-VERIFY-BOUNDARY.md`) |
| **Page rasters / DPI screenshots** | not yet | Needs a PDF renderer, and this build depends on no C++ stack and no AGPL code by decision. Declared as a limitation rather than silently absent |

---

## The one rule that ties both tables together

Everything the engine could not do is **stated** — as a named limitation, a typed absence, or a
counted bucket — rather than guessed at or quietly dropped. A gap is never presented as a success.
If something in the **Cannot** table ever moves to **Can**, it moves with a measurement, not with a
sentence.
