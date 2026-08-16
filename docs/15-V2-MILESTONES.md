# 15 — v2 slices

**Status:** implementation authority for v2 · **Scope document:** `14-V2-SCOPE.md`
**This is the code-review map for v2.** Every v2 PR belongs to exactly one slice.

**v2 is scoped, not started.** S0 is these two documents; **S1–S4 are not started** and no office
parser exists in this tree.

**v1 is not done.** S7's gate is measured and **missed at 64‰** against a 489‰ floor
(`09-V1-MILESTONES.md` S7, `table-gate-v1.md`). **v1.1 is complete** at 0.14.1 and **v1.2 is
complete** at 0.19.0. v2 began because the owner asked for the next roadmap row, and nothing in it
closes v1.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v2 scope + this document | — | **done** |
| **S1** | The grounding contract for a page-less source: decide, or say what would decide it | S0 | **not started** |
| **S2** | DOCX → representation: the reader, the locator variant, the profile | S1 | **not started** |
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

## S1 — the grounding contract for a page-less source — **not started**

- **Goal:** decide what *"a DOCX quote grounds"* means, **before** a reader exists whose output
  depends on the answer.

- **The standing constraint:** `14-V2-SCOPE.md` §5. `ethos.grounding.v1` has three walls against a
  page-less source — `source.media_type` is `{"const": "application/pdf"}`, `element` requires
  `page` and `bbox`, and `page` requires integer `width`/`height`/`rotation`. Neither honest reading
  permits inventing a page to get past them.

- **The two readings**, either of which is a legitimate outcome:
  - **(a) revise the schema** — a page-less source shape, with a written rationale, a version, and
    tests, taken as a deliberate change to the **verifier's** contract;
  - **(b) grounding stays PDF-only** — v2's gate is met at the representation level through the
    fingerprint-checked handle path `node_get` already uses, and `ethos.grounding.v1` keeps meaning
    *"a box on a page in a PDF"*.

- **A third outcome is legitimate and S5 is the precedent:** *"not yet, and here is what S2 would
  have to produce for this to be decidable."* A slice that measures a question and refuses to answer
  it early is done, provided the refusal is written down with its evidence and pinned by a test.

- **Also in scope:** verifying `04-ARCHITECTURE.md` §6's precondition — whether `engine-grounding`
  merely passes `PageRecord`s through or has actually learned about pages — because the answer
  decides whether a second format is a variant or a rewrite.

- **Out:** any reader. Any format. Any change to `ethos.grounding.v1` made *without* the decision
  this slice exists to record.

---

## S2 — DOCX → representation — **not started**

- **Goal:** a DOCX projects into the representation this engine already emits, and a quote from it
  resolves to a node.

- **The standing constraint:** §3's three obligations. The locator is a new `NativeLocator` variant
  addressing the document's own structure; geometry is `NotApplicableToKind`; `pages` is empty; no
  renderer enters the dependency graph.

- **Expected shape**, to be confirmed rather than assumed: a `DocxLocator` naming the part, the
  paragraph and the run — the addresses OOXML itself contains. **If a field on it would have to be
  computed by laying the document out, it does not belong on it.**

- **Also here:** the adapter profile (so a DOCX artifact is provably non-comparable with a PDF one),
  fixtures authored in this tree, mutation coverage on the pattern **A11**, and the format-detection
  question — content-based, per **A4**, never by extension.

- **Out:** XLSX. PPTX. Embedded assets beyond what the gate needs. A fifth crate, unless this slice
  is what proves `engine-pdf` cannot stay PDF-only — in which case it is `engine-office` and the
  boundary table in `04-ARCHITECTURE.md` binds it.

---

## S3 — XLSX → representation — **not started**

- **Goal:** the second half of the v2 gate — an XLSX **cell** binds.

- **The standing constraint:** the same three obligations, and one that is specific to spreadsheets:
  a cell's address is `(sheet, row, column)` **as the file states it**, and a rendered column width
  or a print range is not part of it. A spreadsheet's "page" is a print artefact and is exactly the
  thing §3 forbids.

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
