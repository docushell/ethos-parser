# Ethos parser expansion — landscape research memo

> **Location:** `ethos-engine/docs/reference/`. Research archive.
> **Bootstrap authority for implementation:** `docs/00`–`docs/07` (this tree).
> **Memo reading order inside this archive:** §16 → §17 → §18 (evidence §§3–5 still valid).

> ## ⚠ Pass-4 update — LiteParse + four-way parity + OCR (2026-08-12)
>
> ### Reading-order authority after this pass
>
> - **Bootstrap authority: §16 + §17 + §18**, plus `ethos-engine-parity-checklist.md` (the full
>   capability-by-capability extract)
> - **Evidence: §3–§5, §16.3 and §18.3 measurements** — all still authoritative
> - **`ethos-docushell-parser-plan.md`: architecture depth only**, superseded where it conflicts
>
> **§18** adds LiteParse — the competitor Ethos's own PRD §2.1 names as *"the closest overlap for
> Ethos' Release 1 parser-core surface"* — built from source and measured. Three headline results:
> LiteParse is a **more honest codebase than pdf-inspector** and its `is-complex` **reason codes
> beat a confidence float outright** (take it); its OCR path **silently filters text at two
> confidence thresholds** and its office path **converts through LibreOffice to PDF, inventing
> pagination** (refuse both); and the 34-point disagreement between ODL's and pdf-inspector's
> published LiteParse scores is fully explained by **invocation flags, not evaluator bias** — the
> strongest available argument for Ethos's benchmark-pinning discipline.
>
> §17 remains the ODL/Anydoc/pdf-inspector parity authority (including the triple-sourced
> **ODL-local 0.489 table** bar). §16 remains the authority for the greenfield decision, the
> Workbench rules, the verify boundary and the OSS integration map.
>
> ---
>
> ## ⚠ Pass-1 direction superseded — 2026-08-12
>
> **The product direction in the body of this memo is superseded.** It concluded "measure first,
> defer the parser," and recommended a separate `ethos-lab` side project with agent assist deferred
> entirely. A DocuShell parser+verifier north star was then set by the decider, giving:
>
> ### → `ethos-docushell-parser-plan.md`
>
> …which §16 in turn amends on several points (greenfield engine, `DocumentRepresentation v0` as
> the emit target, Markdown deferred out of v0).
>
> **What remains valid here:** every teardown, measurement and licence finding. §3 (repo
> teardowns), §4 (capability matrix), §5 (grounding-fitness scores, adapter cost, the verified ODL
> adapter defect, the divergence spike), §9.2 (licence table) and §15 (method and its limits) are
> the research record and are unchanged.
>
> **What is superseded:** §1 (this section), §6 (side-project architecture), §7 (the separate
> "DocIR" proposal), §11 (workspace plan), §12 (roadmap), §13.1 (positioning) and §14
> (recommendations). See §1b immediately below for exactly what changed and why.
>
> Neither this file nor the plan is Ethos repository documentation. Neither is an ADR, a claims
> authority, or a release-prep record, and neither belongs in the Ethos `docs/` tree unless a
> decider promotes it through the normal lane.

**Date:** 2026-08-12 · **Original scope:** should Ethos build an optional advanced multi-format
parser, and if so, how · **Deliverable:** this report. No code was written beyond time-boxed spikes.

---

## 1. Executive verdict — SUPERSEDED

*Retained for the record. The reasoning below was sound given its premise; the premise changed.*

The original verdict was: **build the measurement first and the parser second**, because
`docs/proof-statement-v1.md` §7 had cut corroboration (*"Revisit only with a measured divergence
number, not a threat model"*) and cut multi-format pending a named DocuShell workflow — so the
highest-value first deliverable was a divergence report, not a converter.

The evidence behind it stands and is worth restating, because the new direction rests on the same
findings:

- Ethos's three strongest claims are all blocked by not reading the source bytes: evidence tiers
  cap at `ElementScoped` on foreign input, the in-toto `subject[1]` source binding is deliberately
  never emitted because *"Ethos never opened the PDF,"* and representation fidelity is explicitly
  unproven.
- Verified by running it: real OpenDataLoader output through `ethos verify` grounds all three
  claims while declaring **five of six capabilities limited**, byte-identically across runs.
- Nothing in the landscape closes that gap — pdf-inspector's `TextItem.height` is literally the
  same variable as `font_size`, OpenDataLoader's own schema emits **no page dimensions at all**,
  anydoc's IR has no page and no bbox by design, and Docling's VLM path stamps `[0,0,0,0]` with a
  `# FIXME` admitting it is fake. The subtler danger: Docling's dots/chandra/deepseek utilities
  parse bboxes **out of model-generated text**, producing plausible non-zero boxes that pass every
  gate Ethos has.
- The one real spike: two completely independent PDF stacks agreed on horizontal origin to
  **0.001 pt** and disagreed on height by **6.174 pt** — one reports baselines, the other ink boxes
  — which validates Ethos's decision to fingerprint character origins while excluding bbox
  dimensions, and shows a naive box-IoU dual-read would be pure noise.
- **A verified silent defect in shipped Ethos code:** the ODL adapter addresses table cells by
  array position and hardcodes spans to 1, while ODL emits authoritative addresses and *omits
  span-covered slots*, so a `table_cell` citation on any merged table resolves to a different,
  real, plausible cell and is reported `grounded`. **Still open, still worth fixing first.**

---

## 1b. What changed from the previous memo conclusion

| Topic | Previous memo | Current direction (`ethos-docushell-parser-plan.md`) |
| --- | --- | --- |
| **Primary goal** | Produce a divergence measurement; defer the parser | **Build the parser+verifier as the main goal**; measure in parallel, not instead |
| **Where it lives** | A separate `ethos-lab` repo, quarantined, mostly never upstreamed | **In Ethos**, feature-gated. Only the OCR weights, the assist lane and the bench results stay outside |
| **The IR** | A new parallel "DocIR" in the lab, deliberately never in core | **Extend the existing canonical model** — `Canvas`, typed `Geometry`, `DerivationClass`, and a `payload.structure` reference tree. A second IR inside one repo would be duplication |
| **Markdown** | An export; explicitly "do not compete on quality" | **First-class output with a quality bar**, plus the **Markdown Anchor Map** that makes RAG-over-Markdown verifiable. This is the piece no competitor ships |
| **Agent/VLM assist** | Deferred entirely; "the dual-read lane is the assist lane" | **In scope, last and quarantined** (Phase E), proposal-only, dual-read, byte-diff gated. A VLM may flag a hard page; its text can never become evidence |
| **Dual-read** | Framed as corroboration, blocked by §7 | Framed as **review-routing for the assist lane** — never fingerprint-critical, never a verification claim. The §7 cut applies to corroboration-as-a-feature and is not being overridden |
| **PDFium** | Keep; treat removal as premature | **Keep, and build a second backend under its own profile.** "Optional" is per capability: text and locators can leave PDFium, **crops cannot** |
| **Multi-format** | Blocked pending a named trigger | **The trigger fired.** DocuShell full-reliance is the "named workflow" form §7 asked for; proceed through the normal `contract-change` lane |
| **Performance** | One axis of a benchmark design | **A first-class gate on both sides.** G1 throughput is specified in three documents and implemented in zero lines — that is the first thing to build |
| **Headline risk** | Moat dilution | **Table quality.** `ethos-tables` is whitespace-only, one table per page, spans hardcoded to 1 — a regression against the parser being replaced |
| **What justifies it** | Three ladders Ethos is stuck low on | **FR-8**, the only open DocuShell friction entry, plus a documented three-parse pipeline that one native parse collapses |

**What did not change:** verification is the moat; parsing stays optional; `GroundingSource` and
the foreign-parser path stay first-class and must keep passing their tests; fail closed; no AGPL;
no fabricated geometry; no public superlatives without gated evidence.

---

## 2. Ethos current-state baseline

Everything below is **observed fact** from the working tree at `main` (0.6.0, with the
proof-statement-v1 changes uncommitted), unless labelled otherwise.

### 2.1 The boundary that defines the product

```
your parser output ──▶ GroundingSource ──▶ ethos-verify ──▶ verification report
```

`crates/ethos-core/src/grounding.rs` is deliberately the narrowest module in the tree. Its
doc comment states the rule: *"This module depends on `serde` only — no canonical model, no
backend types, no PDFium, nothing Ethos-parser-internal."* The parser is already a plugin.
That is not aspiration; it is the type system.

The trait surface is nine methods, five of which have safe defaults that return "I don't have
that" rather than a guess:

| Method | Default | Meaning of the default |
| --- | --- | --- |
| `parser()` | required | identity of producer + adapter |
| `capabilities()` | required | drives explicit downgrades |
| `fingerprint()` | required | `None` is legal |
| `pages()` / `elements()` | required | canonical order |
| `structural_provenance()` | `None` | "callers must not infer structure" |
| `spans()` / `tables()` | empty | verification downgrades accordingly |
| `crop_ref()` | `None` | no L2 evidence |

### 2.2 Locator precedence and the evidence tier ladder

`Citation` (`crates/ethos-core/src/verify_types.rs:168`) carries six optional locator fields:
`page`, `element_id`, `span_id`, `table_id`, `cell`, `bbox`. `has_locator()` is the only
"any of these" check; a citation with none of them fails closed with
`CheckReason::MissingLocator`.

The in-flight change adds `EvidenceTier` (`verify_types.rs:406`), which is the single most
important type for this memo:

```rust
pub enum EvidenceTier {
    ExactSpan,      // sub-element precision
    TableCell,      // bound by (table, row, col)
    ElementScoped,  // whole element
    PageScoped,     // page only, no element resolved
}
```

`docs/CLAIMS.md` §1 binds it: *"How precisely each claim was bound — set where the target
resolves, so it cannot drift from locator precedence."*

**This ladder measures locator precision and nothing else.** It has no axis for *how the
text came to exist*. That gap is the central design problem for OCR and agent assist, and
§8 answers it.

### 2.3 Determinism is a typed contract, not a policy

`docs/determinism-contract.md` is normative and unusually strict:

- **c14n v1**: UTF-8, no whitespace, keys sorted by code point, **integers only** — *"Floats
  do not exist in canonical Ethos."* Any non-integer number anywhere in a canonical value is
  a c14n error.
- **Quantize at extraction** (`crates/ethos-core/src/geom.rs:42`): `q = round_half_away_from_zero(pts × 100)`,
  centipoints, top-left origin, enforced *by type* — `QRect`/`QPoint` are i64, and raw `f64`
  tuples cannot cross the backend boundary. The single permitted float operation in the whole
  canonical path is that one scale-and-round.
- **ids-v1** (`crates/ethos-core/src/ids.rs`): `p%04d`, `e%06d`, `s%06d`, `t%04d` — *"deterministic
  functions of canonical order — never random, never time-based."*
- **Exclusion table**: `bbox`/`bboxes` are deliberately **excluded** from `payload_sha256`,
  because *"PDFium reports platform-sensitive rectangle dimensions for otherwise identical
  text."* Spans are anchored instead by `origin_locator` (`origin-run-locator-v1`, from
  `FPDFText_GetCharOrigin`), which **is** fingerprint-critical.

That last point is easy to miss and it matters enormously for anything new: **Ethos already
concluded that precise rectangle dimensions are too platform-sensitive to fingerprint, and
moved identity onto character origins.** Any new reader must answer the same question, and
the answer is already precedent.

### 2.4 What Ethos refuses to claim

`docs/CLAIMS.md` §2 is the most useful page in the repo for this exercise, because it names
the exact gap a parser would close:

> **Not proven:** That the representation faithfully reflects the document. *Ethos verifies a
> claim against the representation, not the representation against the source. A parser error
> that both drafts and verifies consistently is invisible.*
>
> **What would close it:** Reviewer inspection of a rendered crop; independently derived
> parsers compared against each other, **analysed and deliberately cut** in
> `docs/proof-statement-v1.md` §7 — *revisiting it needs a measured divergence rate on real
> documents, not a threat model.*

And the last row: *"Any speed, footprint, or parser-quality property — No benchmark has been
run whose numbers we would defend."*

### 2.5 The two standing rulings this memo must respect

`docs/proof-statement-v1.md` §7 already ruled on both things the brief asks for. Reporting
them faithfully matters more than re-deriving them.

**On corroboration (= the dual-read model), CUT:**

> Running two independently derived parsers and reporting their disagreement is the only
> deterministic answer to "who checks the parser?", and it is cut anyway. No external user has
> asked for it, **two parsers sharing an upstream share failure modes**, it doubles parse cost,
> and nobody has measured the divergence rate on real documents — a rate near zero makes it not
> worth building and a rate that is high makes it noise reviewers learn to ignore. […]
> **Revisit only with a measured divergence number, not a threat model.**

**On multi-format, CUT with a named trigger:**

> **Trigger to revisit:** a named DocuShell workflow requiring DOCX or XLSX verification, a
> design partner asking, or a real corpus where non-PDF is a meaningful share. Not before.

Two options were deliberately kept open at near-zero cost, both already landed: a test locking
the geometry-free text path, and `Option<[i64; 4]>` for `bbox` in the trait.

**Consequence for this memo:** the side project is not competing with those rulings — it is the
instrument that produces the evidence they demand. Its first deliverable is a number, not a
parser.

### 2.6 The five gates that make PDF the only supported format

`docs/bring-your-own-parser.md` §"Geometry is required" enumerates them, and they are all in
one layer — the artifact schema and its validator, **not** the verification algorithm:

| # | Gate | Where |
| --- | --- | --- |
| 1 | `media_type` is `const "application/pdf"` | `schemas/ethos-grounding-source.schema.json` |
| 2 | the same media-type check | `crates/ethos-core/src/grounding_json.rs` |
| 3 | `coordinate_system` pins `unit: centipoint`, `origin: top-left` | schema |
| 4 | `bbox` required on element, span, table, cell | schema |
| 5 | positive-area and in-page-bounds enforcement | `grounding_json.rs` |

I verified gates 1, 3 and 4 directly in `schemas/ethos-grounding-source.schema.json`: `media_type`
is `{"const": "application/pdf"}`, `coordinate_system` is `{"unit": {"const": "centipoint"},
"origin": {"const": "top-left"}}`, and `bbox` is in the `required` list of `element`, `span`,
`cell`, and `table`. The `bbox` def additionally forces `x1 ≥ 1` and `y1 ≥ 1` via `prefixItems`,
so a `[0,0,0,0]` sentinel cannot pass the schema alone.

The verifier itself binds text with no geometry at all. **Multi-format is a schema problem, not
an algorithm problem.** That is a much cheaper starting position than it looks.

### 2.7 A wire/trait asymmetry worth knowing before you extend either

The Rust `Capabilities` struct carries six fields (`spans`, `char_offsets`, `tables`,
`fingerprint`, `coordinate_origin`, `crop_support`). The wire `capabilities` object in
`ethos-grounding-source.schema.json` carries **three** (`spans`, `char_offsets`, `tables`),
because the other three are implied by the pinned `coordinate_system`, by `source.sha256`, and
by the absence of crops on that path.

Any new format breaks that implication — a DOCX source has no `coordinate_system` to pin — so
extending the wire schema means the three implied capabilities must become explicit. That is a
`contract-change` PR, not an incidental edit.

### 2.8 Empirical baseline: what the closest competitor actually yields today

This is the most valuable evidence in the repo, and it is already committed:
`fixtures/foreign/opendataloader/real/`.

I ran it (release binary, `ethos 0.6.0`):

```bash
ethos verify fixtures/foreign/opendataloader/real/opendataloader-output.json \
  --grounding opendataloader-json \
  --citations fixtures/foreign/opendataloader/real/citations.json
```

Result, twice, byte-identical (`sha256 445fada4…` both runs — **determinism confirmed
empirically, not just claimed**):

```json
"capability_limits": [
  "missing_fingerprint", "missing_spans", "missing_char_offsets",
  "missing_tables", "unknown_coordinate_origin"
],
"warnings": ["capability_limited"],
"grounding": { "parser": { "name": "opendataloader-pdf", "version": "unknown" } }
```

**Five of six capabilities limited.** All three checks ground, but every one lands at
`evidence_tier: "element_scoped"` — the second-weakest rung. The fixture README states the
cause plainly:

> The OpenDataLoader JSON shape does not include parser version or page dimensions, so the Ethos
> adapter reports parser version as `unknown`, derives page extents from observed bounding boxes,
> keeps coordinate origin as `unknown`, and does not declare table capability for real ODL output yet.

Page geometry had to be measured **outside** the parser entirely — `wp0-page-metadata.json`
records `pdfinfo source.pdf` → `595 × 841 pt, origin bottom-left`.

*(Note on method: the committed goldens contain `attestation` and `evidence_tier`, which the
built release binary does not emit — the binary predates the uncommitted proof-statement-v1
changes. The live output is otherwise identical to the golden.)*

### 2.9 The strategic gap, stated precisely

Three ladders, and Ethos is stuck low on all three whenever it does not read the bytes itself:

1. **Locator precision** — `PageScoped → ElementScoped → TableCell → ExactSpan`. Foreign
   parsers land at `ElementScoped`. Spans need char offsets nobody else emits.
2. **Source binding** — `proof-statement-v1.md` §1.4 rules that `subject[1]` (the source PDF)
   is emitted **only when Ethos read the bytes itself**, and today `ethos verify` never emits
   it. On the Grounding JSON path, *"Ethos never opened the PDF."*
3. **Representation trust** — unmeasured, per §2.4/§2.5.

**A first-party reader is the only thing that moves all three at once.** That, and not format
coverage, is the argument for building one.

---

## 3. Repo teardowns

All four repos were cloned and read at source level. Claims below are marked **OBSERVED** (read in
source or produced by a command), **INFERRED**, or **RECOMMENDATION**.

---

### 3.1 firecrawl/pdf-inspector — real origins wearing a fake box

**MIT** (`LICENSE`, "Copyright (c) 2026 Firecrawl"). **~83k LOC** of Rust (61k excluding two
generated tables, `adobe_korea1.rs` and `glyph_names.rs`). Backend is **`lopdf` 0.42 +
`ttf-parser` 0.25** (`Cargo.toml:49`) — **no PDFium, no ML, no network**; a crate-tree census
found zero GPL/MPL/CDDL/EPL. OBSERVED.

**One licence obligation to note before vendoring:** `external/bcmaps/` ships **168 binary
`.bcmap` files** (pdf.js-format CMaps) under an Adobe BSD-3-Clause-style notice requiring
reproduction of the copyright notice *in binary form*, and `Cargo.toml:14-21` includes them in the
published package. Vendoring pdf-inspector therefore carries a NOTICE obligation.

**Version note:** anydoc pins `pdf-inspector 0.1.8`; upstream is at **1.14.1**. Any teardown of
the two together is reading two different codebases.

It is a content-stream interpreter with its own font stack: `extractor/content_stream.rs`,
`extractor/fonts.rs`, `base14.rs`, `tounicode.rs`, `glyph_names.rs`, `adobe_korea1.rs`,
`structure_tree.rs` (PDF tagged-structure tree), `tables/`, `detector.rs`.

**What I ran** (see §5.5 for the numbers): `detect-pdf` returned `TEXT-BASED, confidence 100%,
OCR recommended NO` in 3 ms. `pdf2md` was byte-identical over three runs.
`pdf2md --items-json` emitted 7 positioned `TextItem`s.

**The coordinate story, precisely — this is the crux, and it was adversarially re-verified.**
The text-matrix implementation is genuinely competent: `combined = Tm × CTM`, with the `q`/`Q`
stack, `cm` composition, `Td`/`TD`/`T*`/`Tm`/`TL`, text rise, per-glyph advance from real font
widths, Type3 `FontMatrix` scaling, and Form XObject recursion with inherited CTM. So:

- **`x` and `width` are content-stream-real** — CTM-transformed origin and text-space advance.
  They matched an entirely independent JVM stack to **0.001 pt** (§5.5). OBSERVED.
- **`y` is the baseline, not a box edge** — `content_stream.rs:519` takes `combined[5]`, the
  text-matrix translation. Bottom-left origin. Documented only in a Rust doc-comment
  (`types.rs:103`) that no JSON, Python, or napi consumer ever sees.
- **`height` is synthesized from font size** — `content_stream.rs:548` assigns
  `height: rendered_size`, the *same variable* as `font_size`. Verified empirically: `h == fs`
  exactly on every item, in both the verifier's run (18.000/18.000, 12.000/12.000) and mine
  (32.02/32.02). `types.rs:105` admits it: *"Height (approximated from font size)."* No
  ascender, no descender, no glyph ink extent.
- **Items are per text-show operation (Tj/TJ run), not per glyph** — 66 items for a 2-page
  document; one item is the whole run `"Plain paragraph with "`. Sub-run addressing requires
  re-splitting by advance width. My own run produced 7 items for a heading plus six body lines.
- `width` is `0.0` when the widths table is missing (`content_stream.rs:543-545`), while
  pdf-inspector's *internal* `effective_width` substitutes `chars × font_size × 0.5`
  (`text_utils.rs:349-355`). **Incidence is negligible** — 4 items in ~63,000 across 26 fixtures
  (0.006%), concentrated in one broken-CID file that `detect-pdf` already routes to OCR. A real
  code path, a footnote-level risk.
- **`/Rotate` is never read** (grep: zero hits for the page key). OBSERVED.
- **`/CropBox` *is* read** — `extractor/mod.rs:1170-1206` `get_page_box()` reads `/CropBox`
  first, falls back to `/MediaBox`, and walks `/Parent` for inheritance. But it is used only for
  clipping off-page items, **not** for the coordinate origin or the top-left flip; that path uses
  `get_page_height` (`lib.rs:3308-3330`), which is MediaBox-only and discards the lower-left
  origin. The sharper finding is that the codebase holds **two incompatible page-box notions and
  the geometry path uses the wrong one.**

So: **half the box is real and half is a guess** — which is precisely the "REAL vs SYNTHESIZED"
distinction Ethos exists to police, appearing inside the tool that looked most promising.

**Scanned routing** is the genuinely reusable idea: `detect_pdf_type_with_config` with
`PdfType`, `ScanStrategy`, and a `DetectionConfig` carrying explicit thresholds. Note the
`confidence` value is a **lookup table**, not a calibrated probability (`detector.rs:310-334`).

Also worth knowing: `is_bold`/`is_italic` are **not** purely font-descriptor facts either —
`content_stream.rs:557-558` computes `is_bold_font(base_font) || desc_bold`, i.e. a name-sniffing
heuristic OR'd with the descriptor flag.

**Determinism:** deterministic for fixed bytes and a fixed binary (I verified 3×). But the
reading-order *rule* is data-dependent and cliff-shaped — `layout.rs:2420` chooses stream-order
vs y-sort from a `chaos_ratio > 0.4` vote. Stable function, not a stable contract.

**Verdict: WRAP for routing, then FORK for geometry.** The routing signal is worth ~200-350 LOC
of adapter. An honest grounding adapter is ~1,400-2,200 LOC and reuses essentially one thing —
the operator state machine's `(x, y)`. Everything geometric layered above it gets discarded.

---

### 3.2 firecrawl/anydoc — an excellent flow IR that throws away every locator

**MIT** (Sideguide Technologies Inc.). Pure Rust, **zero native dependencies**: `calamine`,
`cfb`, `csv`, `quick-xml`, `zip`, `encoding_rs`, `flate2`, and **`pdf-inspector` 0.1.8** as its
PDF backend. OBSERVED.

The IR (`src/model/`) is a clean flow-document tree: `Document{blocks, notes, assets}`;
`Block::{Heading{level,anchor,content}, Paragraph, List, Table, BlockQuote, CodeBlock, Rule}`;
`Table` with `CellSlot::{Origin(Cell), Covered{origin_row, origin_col}}` and an
exactly-once span invariant enforced by `GridBuilder` and asserted in tests. It is a genuinely
good model — better than Ethos's own on merged-cell representation.

**It has zero positional fields. No page, no bbox, no offsets** — verified by exhaustive grep.
OBSERVED.

**The finding that matters most:** PDF **bypasses the IR entirely**. `src/formats/pdf.rs:15`
calls `pdf_inspector::process_pdf_mem(bytes)` and keeps only `result.markdown` — discarding a
backend that already computed per-run `x/y/width/height/page/mcid`. It also discards
`LayoutComplexity{pages_with_tables, pages_with_columns}`. So an Ethos consumer receives a PDF's
text with **no signal that the page had a table or columns at all** — unverifiable *and unflagged
as unverifiable*.

**What each format knows and throws away** — the locator-fork answer:

| Format | Knows | Discards | Recovery cost |
| --- | --- | --- | --- |
| **xlsx/xls** | `range.start()` = absolute A1 origin of the used range (`formats/sheet/mod.rs:47`); sheet name; merged regions as absolute `Dimensions` | emits relative grid indices only; sheet name only when >1 sheet | **~10 LOC.** True A1 = `(start.0+r, start.1+c)`. The cheapest locator win in this entire memo |
| **pptx** | 1-based slide index, already computed (`pptx/mod.rs:99-103`) | slide index survives only as a conditional anchor; shape `a:xfrm/a:off/a:ext` EMU offsets **never parsed** (grep: zero hits) | slide id ~5 LOC; **EMU bbox ~200-400 LOC** in XML the frontend already walks. 12700 EMU = 1 pt exactly — integer, no floats |
| **docx/odf** | page breaks | flattened to `LineBreak` (`docx/content.rs:501`); ODF soft page breaks dropped (`odf/text.rs:398`) | correct — pagination is not in the file |
| **rtf/doc** | twip cell-edge geometry (`shared/grid.rs`) | consumed and discarded | moderate |

Also: the **Markdown table is not the IR table** — rendering erases spans, consumes the header
row, and truncates trailing empty rows/columns (`render/markdown/table.rs:32-65`). Anyone
addressing a cell by its Markdown position is addressing a different grid.

**Determinism** is good on the office half — no threads, no float comparisons, deterministic sorts
throughout, 58 committed `insta` snapshots, and a fuzz corpus. It is *not* a crate-level property:
the PDF half inherits pdf-inspector's data-dependent reading-order heuristic. Score the halves
separately.

**Verdict: STEAL THE SHAPE, do not depend on it.** The `Block`/`Inline`/`CellSlot` model is the
right flow IR and it is MIT. But anydoc's purpose is discarding exactly what Ethos needs.

---

### 3.3 opendataloader-project/opendataloader-pdf — the closest thing to a citation parser, and it cannot identify itself

**Apache-2.0** (Hancom, Inc.; MPL-2.0 before 2.0). Engine is **veraPDF (MPL-2.0, "chosen from
dual-licensed options", `THIRD_PARTY/THIRD_PARTY_LICENSES.md:4`) + Apache PDFBox 3.0.4**, on the
JVM. Third-party tally: 211 MIT, 40 Apache-2.0, 14 ISC, 13 MPL-2.0, 12 BSD-3, 7 BSD-2, 6 EPL-2.0,
3 CDDL-1.1, 1 EPL-1.0, 1 EDL-1.0. **No GPL, no AGPL.** OBSERVED.

**Two licence details that a casual reading gets wrong.** First, MPL-2.0 is not the only weak
copyleft present — **CDDL-1.1** appears via Jakarta Activation and JAXB. Second, and more
important: `java/opendataloader-pdf-cli/pom.xml:66-92` runs the **Maven Shade plugin with an
`*:*` artifact filter**, so veraPDF classes are *repackaged into the CLI jar*. "An unmodified
separate program" is therefore the wrong description of the shipped artifact. Distributing it is
still permissible — MPL-2.0 allows a Larger Work provided the covered files' source stays
available — but if `ethos-lab` ever redistributes that jar rather than invoking a user-installed
one, this needs a deliberate ruling rather than an assumption. Note also that the Python and Node
wrappers **copy a jar already in the repo**; they do not fetch one at build time.

**The architectural insight:** ODL's structure model is **accessibility-derived**, not
layout-ML-derived. It wraps veraPDF's `wcag-algorithms` and `wcag-validation`, and its output
carries `pdfua_tag: "H1"`. Reading order is per-page XY-Cut (`XYCutPlusPlusSorter.java`). This is
a PDF/UA tagger repurposed as a chunker — which explains both its strengths (real semantic roles,
best-in-class table model) and its weaknesses (quality tracks the document's tag tree).

**Its own `schema.json` is the indictment:**

- `bounding box` is documented as **`[left, bottom, right, top]`** — bottom-left, floats.
- `baseElement` requires only `{type, page number, bounding box}`. **`id` is optional.**
- **There are no page dimensions anywhere in the schema.** Only `number of pages`.
- No parser version, no schema version, no source hash.

So the artifact cannot be converted to a top-left frame without the source PDF, and cannot say
which ODL produced it. Ethos's fixture had to measure page geometry externally with `pdfinfo`
(`wp0-page-metadata.json`) and pin the generator artifact's SHA-256 in a sidecar manifest.

**The table model is the best of the four** (`TableCellSerializer.java:39-42`): authoritative
1-indexed `row number`, `column number`, `row span`, `column span`, per-cell `bounding box`, and
`is header`. Multi-page tables link via `previous table id`/`next table id`.

**Traps found in the Java, and they are serious:**

- `id` is an iteration counter assigned at `DocumentProcessor.java:405-409` — **before**
  `sortContents` at `:197`. So on a multi-column page, **array order is reading order and `id`
  order is content-stream order, and they disagree.**
- `--threads > 1` gives "output may vary slightly on some PDFs" — **upstream's own words**
  (`CLIOptions.java:223-224`). Ethos's fixture correctly pins `--threads 1`.
- Default filters **delete** content: `filterOutOfPage`, `filterTinyText`, `filterHiddenOCG` all
  default true (`FilterConfig.java:30-32`).
- `hidden text` is effectively unreachable: the only call site passes `isFilterHiddenText = true`,
  and that branch **deletes** low-contrast text rather than flagging it
  (`HiddenTextProcessor.java:56-62`).
- `JsonWriter.java:105-107` swallows exceptions after partial streaming and exits 0 — **a
  truncated output file is indistinguishable from a short document.**

**Verdict: WRAP (subprocess) and keep the existing adapter.** It is the best available foreign
grounding source and it still cannot self-declare geometry, version, or completeness.

---

### 3.4 docling-project/docling — the richest locator model, and the most dangerous

**MIT** (`docling-slim` 2.119.0, IBM). Python. The locator model comes from `docling-core`
(`docling-core>=2.91.0`): `ProvenanceItem(page_no, bbox, charspan)` with a `CoordOrigin` enum
(`TOPLEFT`/`BOTTOMLEFT`) declared per `BoundingBox`. **Structurally this is the closest thing to
what Ethos wants that exists in the open-source world.** Page identity scores 5/5 — 1-based and
absolute to the source document even under `--page-range`.

And then:

**Trap 1 — the VLM path fabricates geometry, with a confession in the source.**
`docling/pipeline/vlm_pipeline.py:735-748` overwrites the provenance of *every* item:

```python
item.prov = [ProvenanceItem(
    page_no=pg_idx + 1,
    bbox=BoundingBox(t=0.0, b=0.0, l=0.0, r=0.0),  # FIXME: would be nice not to have to "fake" it
    charspan=[0, 0],
)]
```

Ethos's positive-area gate catches this. OBSERVED, and it is a direct vindication of that gate.

**Trap 2 — and this one Ethos's gates do NOT catch.** `dots_utils.py:165-172`,
`chandra_utils.py:294-301`, and `deepseekocr_utils.py:333-340` parse bounding-box coordinates
**out of the model's generated text** and scale them into `BoundingBox`. These are
model-hallucinated boxes that are non-zero, in-bounds, and plausible. They pass every structural
check Ethos has. This is materially more dangerous than Trap 1 and it is the single most
important finding about Docling.

**Credit where due, from the adversarial pass.** On the **standard** PDF pipeline the emitted
coordinates are *not* neural-network output: `layout_postprocessor.py:645-669`
`_adjust_cluster_bboxes` sets `cluster.bbox = cells_bbox`, the exact min/max union of the
**measured** docling-parse text-cell rectangles (tables take `union(model_bbox, cells_bbox)`), and
that is what reaches `prov`. The ML decides *which cells group together* and what label they get;
the numbers are measured. So Docling's standard-path coordinate fidelity is genuinely good, and
the real risk there is **grouping and labelling, not coordinate accuracy**. Likewise, repeat-run
determinism *is* tested (`tests/test_backend_pdfium.py:51-69` loads the same page ten times), and
parallelism is not a default nondeterminism source (`settings.py:30-33` sets batch sizes and
concurrency to 1).

**Trap 3 — `charspan` is informationally empty even on the good path.** It is `(0, len(text))` for
essentially every real text element (`readingorder_model.py:99,281,377,482,526,543`). Only
`readingorder_model.py:598-601` computes a real offset. A naive adapter would declare
`char_offsets: true` and emit whole-element spans as sub-element evidence.

**Trap 4 — unit collision across formats with nothing to distinguish them.** On the PDF path
`bbox` is in points. On the XLSX path (`msexcel_backend.py:668-679`) `bbox` holds **grid
indices**. Same field, same type, same document schema. An adapter must gate on
`origin.mimetype` and hope.

**Trap 5 — no field records which *pipeline* produced the document.** `document.py:383-391`
does record a `DoclingVersion`, so the claim "the artifact says nothing about its producer" is too
strong. But standard, VLM, dots, chandra and deepseek outputs remain indistinguishable by schema,
and no model revision is recorded. Detecting a VLM document requires heuristics (all-zero bboxes,
`charspan == [0,0]` with non-empty text) — heuristic detection of a provenance fact that should
have been declared.

**Determinism:** reading order and clustering *are* ML outputs. Model weights are pulled at
`revision="main"` (`stage_model_specs.py:998`) — **not pinned** — and the default OCR engine
varies by host OS (`auto_ocr_model.py:44-77`). There is no pinned non-ML path for PDF. A latent
page-height bug is worth knowing about too: `docling_parse_backend.py:243-252` returns pypdfium2
dimensions with a live `# TODO: Take width and height from docling-parse.`

**Verdict: IGNORE as a dependency; INTEROP as a target; COMPETE on honesty.** Write a
`DoclingDocument → DocIR` adapter that *refuses* placeholder provenance, and treat Docling as a
benchmark row. Its code licence is clean; its determinism is not, and its geometry is not
self-describing.

---

### 3.5 OCR shortlist

Ranked against four hard constraints: licence clean on **both code and weights**, fully local,
deterministic under a declarable envelope, and emits real boxes.

| Engine | Code | **Default weights** | Runtime | Boxes | Determinism | Weights size |
| --- | --- | --- | --- | --- | --- | --- |
| **PP-OCRv5/v6 (ONNX)** | Apache-2.0 | **Apache-2.0** | ONNX Runtime / `ort` (Rust) | 4-pt polygon, **line-level**, abs. px | good, pinned envelope | ~22 MB mobile |
| **RapidOCR** | Apache-2.0 | Apache-2.0 | ONNXRuntime/OpenVINO/MNN | same | same | same, **SHA-256 pinned in `default_models.yaml`** |
| **ocrs** | Apache-2.0 OR MIT | **CC-BY-SA-4.0** (share-alike) | **pure Rust** (rten) | **line + word + char** | best structural story | **~12 MB** |
| **Tesseract 5.5** | Apache-2.0 | Apache-2.0 | C++ | word/line/block, hOCR | **documented irreproducibility** | 4–15 MB/lang |
| **docTR** | Apache-2.0 | Apache-2.0 | **PyTorch only** | word/line/block | Torch CPU ok | GB-scale |
| **EasyOCR** | Apache-2.0 | **no licence statement anywhere** | PyTorch | word quads | Torch CPU ok | GB-scale |
| **Surya 2** | Apache-2.0 | **RAIL-M, free only under $5M revenue** | vLLM/llama.cpp | line + block | **VLM decode — no** | ~1.3 GB |
| **Nemotron-OCR-v2** | Apache-2.0 | **NVIDIA Open Model Licence** (not OSI) | **Linux + CUDA only** | word/sentence/para | n/a | 84M params |
| **Apple Vision / ocrmac** | wrapper MIT | **closed, unpinnable** | macOS only | line/word, normalized, **bottom-left** | changes with OS updates | n/a |
| **OCRmyPDF** | MPL-2.0 | orchestrator | Python + **Ghostscript (AGPL)** | hOCR | inherits | n/a |

**On determinism, honestly.** ONNX Runtime CPU inference is numerically identical run-to-run on
the same binary and machine; weights are fixed and neither detection nor CTC recognition samples.
**Bit-identical output across machines is not available** — float addition is not associative, ONNX
does not pin operation ordering, and runtimes select kernels by CPU feature level. Text is robust
(a 1e-6 logit wobble almost never flips a CTC argmax); **geometry is the fragile part**, because
DB-style detection thresholds a probability map and a sub-ULP difference near the threshold can
move a polygon vertex by a pixel.

Mitigations, all cheap: quantize geometry to integer raster pixels at the IR boundary; drop or
bucket confidence scores (a float confidence in canonical output is a byte-diff landmine for zero
verification value); pin threads to 1 and forbid GPU; pin recognition batch size.

The defensible claim is **not** "reproducible" but *"deterministic within a declared execution
envelope"* — `{engine build id, model SHA-256, runtime version, CPU feature level, threads = 1,
raster DPI}`, all recorded in the report.

**Tesseract is disqualified outright**, and not on accuracy: the same version on the same machine
gives different results between Windows and WSL, and the same Docker image differs across host
OSes, with no root cause identified — plus long-standing upstream reports of varying results
across calls and OpenMP thread-count sensitivity.

**Pick: PP-OCRv5/v6 mobile ONNX via `oar-ocr`/`ort`** — the only candidate clean on both code and
weights, ~22 MB, in-process Rust, no Python, no subprocess. Take RapidOCR's SHA-256 pinning
manifest as the model for your own even if you never run its Python.

**Fallback: `ocrs`** — pure Rust, 12 MB, and the only engine that emits word *and* character
boxes natively, which is what a locator-first IR actually wants. Costs: accuracy, Latin-only,
"early preview" per its own README, and a ruling on CC-BY-SA weights.

**Would not use: Surya** — the highest-accuracy option in the shortlist and still the clearest no,
which is precisely the point. Runner-up no: **EasyOCR**, for shipping weights nobody can
licence-name.

---

## 4. Competitive capability matrix

| | **Ethos native** | **pdf-inspector** | **anydoc** | **OpenDataLoader** | **Docling (std)** | **Docling (VLM)** |
| --- | --- | --- | --- | --- | --- | --- |
| Licence (code) | Apache-2.0 | MIT | MIT | Apache-2.0 | MIT | MIT |
| Licence (engine/weights) | PDFium BSD-3, caller-provided | lopdf MIT | pdf-inspector MIT | veraPDF **MPL-2.0** + PDFBox | **per-model, unpinned** | per-model, unpinned |
| Language / runtime | Rust | Rust | Rust | **JVM** | Python | Python + GPU |
| Native deps | PDFium (dynamic, caller-provided) | **none** | **none** | JRE + bundled jar | torch/onnx | torch/vLLM |
| Input formats | PDF | PDF | **10+** (docx, pptx, xlsx, odf, rtf, epub, csv, doc, ppt, pdf) | PDF | 10+ | 10+ |
| Output: Markdown | yes | yes | **yes (its product)** | yes | yes | yes |
| Output: JSON w/ bboxes | **yes (`ethos.grounding.v1`)** | `--items-json` | **no** | **yes** | yes | yes (fake) |
| Output: HTML | no | no | no | **yes** | yes | yes |
| **Page identity** | `p%04d`, 1-based | `page` u32 (base **inconsistent** across API) | **none** | 1-based, no page id | **1-based, absolute** | 1-based |
| **Page dimensions emitted** | **yes** | no | n/a | **NO** | yes | yes |
| **Coordinate origin** | **declared top-left, pinned** | bottom-left, undeclared | n/a | bottom-left, **prose only** | **declared per-bbox** | declared, values fake |
| Units | **integer centipoints** | f32 points | n/a | float points | float points | float points |
| Rotation handled | **yes, normalized** | **no** (`/Rotate` unread) | n/a | **no** | partial (TODO in source) | n/a |
| **Span / sub-element** | **yes (`s%06d` + origin_locator)** | no | no | no | weak | no |
| **Char offsets** | **yes** | no | recoverable post-hoc | no | `(0, len(text))` — empty | `[0,0]` — wrong |
| **Table cell addressing** | row/col, **spans always 1** | via ML TSR model | **grid + real spans (IR)** | **row/col + real spans + per-cell bbox** | **offsets + spans + bbox** | HTML tokens |
| Multi-page tables | **no** (single page only) | no | no | **yes** (`previous/next table id`) | no | no |
| Tables per page | **1 max** | multiple | n/a | multiple | multiple | multiple |
| Ruling-line tables | **no** (whitespace only) | yes (`detect_lines.rs`) | n/a | yes (veraPDF borders) | yes (TableFormer) | n/a |
| Scanned detection | `image_only_page` warning | **yes, configurable** | no | `hidden text` (unreachable) | yes | n/a |
| OCR | no | no | no | no | **yes, pluggable** | inherent |
| **Determinism** | **contractual, CI-proven** | per-binary; cliff-shaped rules | good (office); PDF inherits | **`--threads>1` varies (upstream)** | **ML, weights unpinned** | **no** |
| **Fingerprint in output** | **yes (`source.sha256`)** | no | no | **no** | `binary_hash` (64-bit trunc) | same |
| Output format version | **yes (`schema_version`)** | **no** | **no** | **no** | yes | yes |
| **Declares its own limits** | **yes (capabilities)** | no | no | no | no | no |
| Fails closed | **yes** | partial (silent tree truncation) | partial | **no** (partial write exits 0) | no | no |

**The column that matters is the last row group.** Every competitor emits content; none emits a
declaration of what it could not do. That is the entire product boundary, in one row.

**Two rows deserve a footnote after adversarial re-verification.** Docling's *standard* PDF
pipeline emits **measured** rectangles (the union of docling-parse text-cell rects), not model
output — so its coordinate fidelity is real and the risk sits in grouping/labelling. Its
VLM-family paths emit fabricated geometry. Both ship under one schema with no field distinguishing
them, which is worse than either alone. And pdf-inspector's `x`/`width` are content-stream-exact
while its `y`/`height` are a baseline and a font size — so "has bboxes" is true and misleading in
the same breath.

**Two honest scores against Ethos.** ODL's table model is better than Ethos's native one — real
spans, per-cell boxes, multi-page linking, multiple tables per page, ruling-line support. Ethos's
`ethos-tables` is whitespace-only, caps at one table per page, hardcodes `row_span`/`col_span` to
1, and never emits table anchor elements. And Docling's page identity and per-bbox origin
declaration are both cleaner than any foreign path Ethos consumes today. Those are real gaps,
worth saying out loud, and they are also the gaps a first-party reader would close.

---

## 5. Grounding fitness scores and adapter cost

Scale: **0** absent · **1** present but unusable/dishonest · **2** needs heavy inference ·
**3** usable with a real adapter · **4** clean mapping, minor loss · **5** drop-in equivalent to
Ethos native.

| Axis | pdf-inspector | anydoc | OpenDataLoader | Docling (std PDF) |
| --- | :-: | :-: | :-: | :-: |
| Page identity | 4 | **0** | 3 | **5** |
| Element identity | 2 | 1 | 2 | 1 |
| Span identity | **0** | **0** | **0** | 1 |
| Bbox quality | **1**‡ | **0**† | 2 | **4**§ |
| Table cell addressing | 2 | **4** (IR) / 1 (MD) | **4** | **4** |
| Char offsets | **0** | 1 | **0** | 2 |
| Reading-order stability | 3 (per-binary) | **4** office / 3 PDF | 2 | 1 |
| Fingerprintability | 2 | 2 | **1** | 4 |
| **Adapter cost** | 2 | 1 (PDF) / 3 (office) | 3 | 2 |

† anydoc scores **0 rather than 1** on bbox because it never claims to have one. Honest absence
outscores a dishonest presence.

‡ pdf-inspector scores 1 despite `x`/`width` being content-stream-exact, because `y` is a baseline
and `height` is the font size. **Half the box is real; a half-real box that looks whole is worse
than no box**, which is what the scale's "present but dishonest" rung is for.

§ Raised from an initial 3 after adversarial re-verification: on the standard PDF pipeline the
emitted rectangles are the **measured** union of docling-parse text-cell rects, not model output.
The VLM/dots/chandra/deepseek paths score **0** and are a separate row in spirit.

**Reading the table.** No column is strong. The best page identity (Docling, 5) sits beside the
worst reading-order stability (Docling, 1) because the order *is* an ML output. The best table
model (ODL/Docling/anydoc, 4) sits beside near-zero span and char-offset support everywhere. **Not
one of the four can address a sub-element span** — the tier Ethos calls `ExactSpan` and the one
that makes a quote citation precise. That is the gap.

### 5.1 Adapter cost, per system

| Target | Estimate | Where the cost is |
| --- | --- | --- |
| **pdf-inspector, routing only** | **~200–350 LOC** | Consume `detect_pdf_type_with_config` → is this page groundable at all. Zero geometry crosses the boundary. Cheap, high value. **Recommended shape** |
| **pdf-inspector, honest grounding** | **~1,400–2,200 LOC** | ~250 re-deriving true glyph boxes via `ttf-parser` (height is unusable); ~200 page-space normalizer for `/Rotate`, `/CropBox`, MediaBox origin — **none of which pdf-inspector reads**; ~350 own line/element assembly with ids; ~300 spans + char offsets (requires reimplementing the conditional-space join in `types.rs:290-328`); ~200 f32→centipoint with explicit tie-break; ~250 tagged-PDF identity via `(page, mcid)` with an explicit "untagged ⇒ refuse" branch |
| **anydoc office → structural subset** | **~400–600 LOC** | element ids from a canonical block path ~150; char offsets by re-derivation ~120; table cells ~80 (near-direct copy); **absolute A1 ~10 LOC**; serde derives ~50. Buys element + span + cell grounding with `page: null, bbox: null` |
| **anydoc office → with page and bbox** | **not an adapter — a layout engine.** Reject | — |
| **OpenDataLoader (existing, working)** | **1,781 LOC** (≈1,001 production, 780 tests) | see §5.2 |
| **OpenDataLoader, to make it correct** | **+250–400 impl, +400 test** | real span-aware cell mapping ~120; explicit bottom-left→top-left flip gated on caller-supplied page height, refusing when absent ~80; version/profile pinning surfaced as a capability reason ~40; a *specified* container-flattening policy ~100 |
| **Docling → DocIR** | **~500–800 LOC, mostly rejection logic** | happy-path mapping is ~120 LOC. The rest: format gating (~80) because `bbox` means points for PDF and **grid indices for XLSX**; pipeline gating (~60) to reject VLM output *heuristically*, because no field declares the pipeline; per-bbox origin normalization (~100) with property tests |

### 5.2 The empirical anchor: what a foreign adapter actually costs

Ethos's shipped ODL adapter is the best available evidence, and the number is not the code — it is
the *diagnostics*. **1,781 lines. The CLI integration is 8 lines** (`crates/ethos-cli/src/grounding.rs:103-110`).
Everything else is the cost of refusing to guess: **85 distinct deterministic diagnostic strings
across 49 `err(...)` sites**, covering recognition, geometry, identity, page references, text
aliasing, child containers, tables, and fingerprints.

It has exactly **three** fallbacks, each explicitly reasoned: `kind` defaults to `"unknown"` when
`type` is blank; real-tree id collisions get a synthetic id (because real ODL emits duplicate
`id: 2` for an image and a paragraph, and refusing would drop a groundable paragraph over an
upstream defect); and real table spans are hardcoded to 1. Everything else that cannot be mapped
exactly is an error.

And what it produces on real input, verified by running it (§2.8): **five of six capabilities
limited**, all evidence at `element_scoped`.

**The adapter performs no origin conversion at all.** `y0 = H - top` appears nowhere. It cannot:
on the real path it derives page height *from* the boxes, so a flip would be circular. Ethos knows
ODL's convention — ADR-0016 records *"bounding boxes are PDF-point coordinates with a bottom-left
origin"* — and still declares `Unknown`, because knowing a convention in a design doc is not the
same as proving it for the bytes in hand.

That downgrade is enforced, not annotated: `ethos-verify/src/lib.rs:1533-1536` makes a
`page`+`bbox` citation against an unknown-origin source `CapabilityBlocked`, and `:1806` gates off
the adjacent-element quote join with the comment *"no coordinates, no join."*

**The real fork in adapter cost:** an adapter that consumes only parser output cannot produce
trustworthy geometry. Either accept a permanent `coordinate_origin: unknown` downgrade, or build a
mapper that *also reads the source PDF* for MediaBox. Ethos already ships both roads.

### 5.3 A verified defect this review turned up

**The ODL adapter addresses real-ODL table cells by array position and hardcodes spans to 1, while
ODL emits authoritative addresses and omits span-covered slots.** Both halves verified against
primary source:

- `adapters/grounding/opendataloader-json/src/lib.rs:546-580` — `parse_real_table_cell` takes
  `row`/`col` as parameters from array position and sets `row_span: 1, col_span: 1` (`:575-576`).
  A repo-wide grep for `"row number"`, `"column number"`, `"row span"`, `"column span"` returns
  **zero matches**.
- `TableCellSerializer.java:39-42` emits `row number`, `column number`, `row span`, `column span`
  (and `is header`).
- `TableRowSerializer.java:41-46` writes a cell **only when**
  `cell.getColNumber() == columnNumber && cell.getRowNumber() == row.getRowNumber()` — so
  span-covered slots are skipped and **the `cells` array is sparse**.

Consequence: on any table with a merged cell, array index ≠ logical column. Ethos's `(row, col)` is
shifted by the number of preceding suppressed slots, and every span reads 1. A `table_cell`
citation resolves to a *different, real, plausible* cell and is reported **`grounded`** — a silent
wrong answer, not an error.

Currently latent only because the pinned real fixture has no tables (hence `missing_tables`), and
`tables_capable` is `!tables.is_empty()`. The first real ODL document with a table activates it.
Filed as a separate task.

### 5.4 Where naive trust would create false confidence — the short list

The fields most likely to survive a code review unchallenged:

1. **`docling` dots/chandra/deepseek bboxes** — parsed out of *model-generated text*, then scaled.
   Non-zero, in-bounds, plausible. **Passes every gate Ethos has.** The most dangerous single item
   in this memo. Note this is specific to the VLM-family paths: the standard pipeline's rectangles
   are measured, which makes the two indistinguishable-by-schema outputs *differ in kind* — the
   worst possible combination.
2. **`pdf-inspector.TextItem.height`** — literally the same variable as `font_size`, next to an
   `x` and `width` that are content-stream-exact. Build `[x, y, x+width, y+height]` and you get a
   box that passes every structural check and is systematically wrong at both edges — excluding
   descenders and over-extending above cap height — on every citation, silently. **The accuracy of
   the horizontal half is what makes the vertical half dangerous:** a spot-check of a highlight
   overlay looks right.
3. **ODL `bounding box`** — bottom-left, satisfies every ordering and area check, points at the
   vertically mirrored location. Saved today only by a capability flag, and capability flags are
   easy to ignore downstream.
4. **ODL page geometry derived from the content hull** — makes the in-page containment check a
   tautology, and makes the extremal elements *exactly page-sized* by construction.
5. **`docling.charspan`** — present, well-typed, and `(0, len(text))`. Declaring
   `char_offsets: true` from it would emit whole-element spans as sub-element evidence.
6. **ODL `id`** — an iteration counter assigned *before* sorting, so `id` order and array order
   disagree on multi-column pages; optional in ODL's own schema; not stable across versions.
7. **`docling` XLSX `bbox`** — grid indices in the same field that holds points on the PDF path.
8. **ODL `hidden text`** — the default configuration *deletes* low-contrast text rather than
   flagging it. Absence of the flag is not evidence of absence of a hidden layer; it is evidence
   that nobody looked, or that the evidence was removed.

### 5.5 The divergence spike — first measurement

Two independent stacks, same PDF (`fixtures/foreign/opendataloader/real/source.pdf`), same glyph
run:

| Quantity | ODL (veraPDF/PDFBox, JVM) | pdf-inspector (lopdf, Rust) | Divergence |
| --- | --- | --- | --- |
| Heading text | `"Lorem Ipsum"` | `"Lorem  Ipsum"` | **identical** after whitespace normalization |
| Body text | 445 chars | 445 chars | **identical** |
| Left edge | 200.891 pt | 200.89 pt | **0.001 pt = 0.1 centipoint** |
| Width | 193.261 pt | 193.26 pt | **0.001 pt** |
| Height | 38.194 pt | 32.02 pt | **6.174 pt** |
| Segmentation | 2 block elements | 7 line items | structural |

The vertical gap is **semantics, not error**: pdf-inspector reports baseline + nominal font size;
ODL reports the glyph ink box. Back out the metrics — ascender **0.952 em**, descender **0.241 em**
— and they are textbook correct for a 32 pt face.

**Three conclusions.** (1) A naive box-IoU dual-read reports permanent disagreement on every
element at a magnitude that swamps any real defect. Compare **origins and text**, never boxes.
(2) Ethos's determinism contract already excludes precise bbox dimensions from `payload_sha256`
and anchors spans on character origins — it fingerprints the quantity that agrees to a tenth of a
centipoint and excludes the one that disagrees by 6 pt. That was a judgement call; it is now a
measurement. (3) Segmentation divergence (7 vs 2) is granularity, not disagreement, so the
comparison must be granularity-independent.

**Limits, stated plainly: N = 1**, on a single-page, single-column, table-free lorem PDF. This
demonstrates the method and quantifies one real semantic gap. It is **not** the divergence rate.

---

## 6. Proposed side-project architecture
> **⚠ Superseded on direction (see the banner at the top and §1b).** This assumed a quarantined `ethos-lab` side project whose first product was a measurement harness. The current authority is `ethos-docushell-parser-plan.md` §3.


The project is called **`ethos-lab`** here. Its core product for the first 60 days is **not a
parser** — it is `concord`, the instrument that produces the divergence number
`docs/proof-statement-v1.md` §7 demands. The parser is what that instrument is built out of.

### 6.1 Why this framing and not "build a better parser"

Ethos has three standing rulings that a naive parser project would collide with: corroboration
cut, multi-format cut, "new parsers" out of scope. Each has a written re-entry condition. A side
project that produces those conditions as artifacts is additive to Ethos governance. One that
ignores them is a fork.

So the ordering is inverted from the obvious one. Measure first, parse second, integrate third.

### 6.2 Pipeline

```
                          ┌──────────────────────────────────────────┐
  input bytes ──────────▶ │ FORMAT ROUTER  (sniff, never trust ext.) │  pinned
                          └────────────────┬─────────────────────────┘
                                           │ MediaKind + CanvasKind
        ┌──────────────────────────────────┼──────────────────────────────┐
        ▼                                  ▼                              ▼
 ┌──────────────┐                 ┌─────────────────┐            ┌─────────────────┐
 │ PAGED READERS│                 │ FLOW READERS    │            │ GRID READERS    │
 │ pdf/pptx/img │                 │ docx/odt/rtf/   │            │ xlsx/csv/ods    │
 │              │                 │ epub/md/html    │            │                 │
 │ R1 pdfium    │  pinned         │ quick-xml       │  pinned    │ calamine        │  pinned
 │ R2 lopdf     │  pinned(alt)    │ (anydoc-derived)│            │                 │
 └──────┬───────┘                 └────────┬────────┘            └────────┬────────┘
        │                                  │                              │
        │   ┌───────────────┐              │                              │
        ├──▶│ SCAN TRIAGE   │ pinned       │                              │
        │   │ (text-layer   │              │                              │
        │   │  coverage)    │              │                              │
        │   └───┬───────────┘              │                              │
        │       │ scanned / hard page      │                              │
        │       ▼                          │                              │
        │   ┌───────────────┐              │                              │
        │   │ OCR LANE      │ pinned under │                              │
        │   │ (separate     │ its OWN      │                              │
        │   │  profile)     │ profile      │                              │
        │   └───┬───────────┘              │                              │
        ▼       ▼                          ▼                              ▼
   ╔═══════════════════════════════════════════════════════════════════════════╗
   ║                       DocIR  — canonical, locator-first                   ║
   ║   Canvas{Paged|Flow|Grid} · Element · Span · Table · Locator · Provenance  ║
   ║   every node carries DerivationClass + producing reader id                 ║
   ╚═══════════════════════╤═══════════════════════════════════════════════════╝
                           │ (DocIR is complete and valid at this point)
        ┌──────────────────┼───────────────────────────────┐
        ▼                  ▼                               ▼
  ┌───────────┐    ┌───────────────┐              ┌──────────────────┐
  │ CONCORD   │    │ ASSIST LANE   │  diagnostic  │ ADAPTERS         │
  │ compares  │    │ agent/VLM     │  ONLY        │                  │
  │ 2 DocIRs  │    │ proposals     │              │ → ethos.grounding.v1
  │ diagnostic│    └───────┬───────┘              │ → Markdown (chunking)
  └─────┬─────┘            │                      │ → HTML (secondary)
        │                  │                      │ → overlay PNG/SVG (debug)
        └──────────┬───────┘                      └────────┬─────────┘
                   ▼                                       ▼
        ethos.concord.v0 (side artifact)          ethos verify (unchanged)
```

### 6.3 Stage contract table

`pinned` = deterministic, byte-reproducible, may feed a fingerprint.
`assist` = may propose, never mutates pinned bytes.
`diagnostic` = observational output only, never an input to a verdict.

| Stage | In | Out | Class | Fingerprint-critical | Fails by |
| --- | --- | --- | --- | --- | --- |
| Format router | bytes | `MediaKind`, `CanvasKind` | pinned | yes (enters config hash) | refusing unknown magic; never trusting file extension |
| Paged reader R1 (pdfium) | PDF bytes | DocIR(Paged) | pinned | yes | `pdfium_unavailable` — fail closed, as today |
| Paged reader R2 (lopdf) | PDF bytes | DocIR(Paged) | pinned (alt profile) | yes, under its own profile id | unsupported filter/encryption → explicit limitation |
| Flow reader | OOXML/ODF/EPUB/RTF | DocIR(Flow) | pinned | yes | malformed XML → error, never partial silent parse |
| Grid reader | XLSX/CSV/ODS | DocIR(Grid) | pinned | yes | ambiguous encoding → error (CSV) |
| Scan triage | DocIR(Paged) + page objects | `TextLayerCoverage` per page | pinned | yes | never guesses; emits coverage ratio + a pinned threshold decision |
| OCR lane | page raster | DocIR nodes, `Derivation::Recognized` | pinned **under `ethos-ocr-v*` profile** | yes, in that profile's namespace only | model hash mismatch → refuse to run |
| Concord | 2× DocIR | `ethos.concord.v0` | diagnostic | **no** | reports incomparability rather than forcing alignment |
| Assist lane | DocIR + page raster | `Proposal[]` | assist | **no** | any malformed proposal is dropped with a diagnostic |
| Grounding adapter | DocIR | `ethos.grounding.v1` | pinned | yes | refuses to emit geometry it does not have |
| Markdown adapter | DocIR | `.md` | pinned | n/a (derived artifact) | — |
| Overlay adapter | DocIR + raster | PNG/SVG | diagnostic | no | — |

### 6.4 The two rules that keep this from eroding the moat

**Rule 1 — DocIR is complete before any assist runs.** The assist lane takes a *finished* DocIR
as input and emits proposals as a separate document. There is no code path where a model's output
is an argument to the pinned reader. This is enforceable structurally: the assist crate depends on
`docir`, and `docir` does not depend on the assist crate.

**Rule 2 — OCR is a different profile, not a different flag.** This is the single most important
design decision in the memo and it needs no new machinery, because Ethos already built it.

`docs/determinism-contract.md` §6 defines the document fingerprint as
`sha256(c14n({config_sha256, payload_sha256, profile_id, profile_sha256, schema_version,
source_fingerprint}))`, and states: *"Two parses are comparable iff their fingerprints are
equal."*

So: give the OCR lane its own profile artifact (`ethos-ocr-v1.json`, pinning the model file
SHA-256, the raster DPI, the thread count, and the ONNX runtime build). An OCR-derived document
then has a different `profile_id` and `profile_sha256`, therefore a different `fingerprint`,
therefore **is not comparable to a born-digital parse of the same PDF, by the existing
contract.** A citation pinned to a born-digital fingerprint goes `stale` against an OCR parse
automatically — no new check, no new warning code, no new reviewer concept.

That is the answer to "OCR must not silently pretend to be born-digital certainty." It is
already in the contract. Use it.

### 6.5 Process and language boundaries

| Component | Language | Boundary | Why |
| --- | --- | --- | --- |
| `docir`, readers, adapters, concord | Rust | in-process | Ethos is Rust; the whole candidate dependency set (lopdf, calamine, quick-xml, zip, encoding_rs) is pure Rust and permissive; determinism control needs explicit float discipline and no GC |
| ODL reader | Rust client | **subprocess** over the ODL CLI jar | JVM never enters the Ethos process. Invoke a user-installed jar rather than redistributing one — the shipped CLI jar shades veraPDF into itself (§9.2) |
| OCR worker | Rust (`ort`) preferred, Python fallback | in-process (`ort`) or **subprocess** (Python) | see §9; the Python fallback exists so the lane is not blocked on the `ort` spike |
| Assist lane | Rust client | **subprocess / network, out-of-tree** | model calls must never be linkable into anything Ethos ships; `deny.toml` bans network-capable crates in the base tree |

**Non-negotiable packaging rule:** nothing in `ethos-lab` is ever a `[dependencies]` entry of an
Ethos core crate. The only two interop surfaces are (a) an `ethos.grounding.v1` JSON file on
disk, and (b) an optional subprocess invocation. That keeps `deny.toml` and the 30 MB G2
footprint gate untouched no matter how heavy the lab gets.

---

## 7. Canonical locator-first IR ("DocIR")
> **⚠ Superseded on direction (see the banner at the top and §1b).** The `Canvas` / typed-`Geometry` / `DerivationClass` design below is **retained and adopted** — but it now extends Ethos's existing canonical model rather than living in a separate lab IR, and it gains a `payload.structure` reference tree. The current authority is `ethos-docushell-parser-plan.md` §4.


### 7.1 The one idea that makes this work: two orthogonal axes

Ethos today has **one** honesty axis, `EvidenceTier`, and it measures locator precision:

```
PageScoped  <  ElementScoped  <  TableCell  <  ExactSpan
```

OCR and agent assist cannot be placed on that axis. An OCR'd heading can be bound at
`ExactSpan` precision and still be a guess about what the pixels say. Adding a fifth rung
("OcrScoped") would be a category error — it would make a *precise* OCR locator sort as *less
precise* than an imprecise extracted one, and reviewers would learn to misread it.

So DocIR carries a **second, orthogonal axis: `DerivationClass`** — how the value came to exist.

```rust
/// How a value came to exist. Orthogonal to EvidenceTier (how precisely it is addressed).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DerivationClass {
    /// Read from the source byte stream: content-stream operators, XML text nodes, cell values.
    Extracted = 0,
    /// A deterministic, pinned function of Extracted values: reading order, block grouping,
    /// table grid inference. No model, no pixels.
    Computed = 1,
    /// Produced by a model from pixels. OCR.
    Recognized = 2,
    /// Produced by an agent/VLM as a suggestion. Never load-bearing, never fingerprinted.
    Proposed = 3,
}
```

Two rules give it teeth:

1. **Monotone composition.** A node's class is the `max` of its own class and every node it was
   derived from. A `Computed` reading order over `Recognized` text is `Recognized`. You cannot
   launder a guess by post-processing it.
2. **Fingerprint gate.** `Proposed` never enters any fingerprint. `Recognized` enters only its
   own profile's fingerprint namespace (§6.4).

A verification report then answers two independent questions — *how precisely was it bound* and
*how was the evidence obtained* — and neither can be confused for the other.

### 7.2 Canvas: the honest answer to "what about formats with no pages"

Do not synthesise pages. `docs/bring-your-own-parser.md` is already explicit that for DOCX
*"Pagination does not exist in the file and must not be synthesised."* DocIR makes that a type:

```rust
pub enum Canvas {
    /// Has a real 2-D canvas with declared geometry: PDF page, PPTX slide, scanned image.
    Paged {
        index: u32,               // 1-based, original document order
        width: i64, height: i64,  // quanta
        rotation: u16,            // 0/90/180/270, normalized
        coord: CoordSystem,       // unit + origin, DECLARED not assumed
    },
    /// Reflowable: DOCX, ODT, RTF, EPUB, HTML, Markdown. Ordinal only.
    Flow { ordinal: u32 },        // section/chapter/part index; NOT a page
    /// Tabular: XLSX sheet, ODS sheet, CSV.
    Grid { sheet_index: u32, sheet_name: String },
}
```

`CanvasKind` (the discriminant alone) is what capability declarations key off. This is what lets
one verifier serve all three families without either lying about geometry or refusing non-PDF.

### 7.3 Locator families

```rust
pub enum Locator {
    /// Works on every canvas. The backbone.
    Structural { element: ElementId, span: Option<SpanId>, chars: Option<Range<u32>> },
    /// Paged and Grid. Ethos's existing table model, unchanged.
    Tabular { table: TableId, row: u32, col: u32 },
    /// Grid only. Native spreadsheet addressing — a real locator, not a synthesized one.
    Cellular { sheet: u32, row: u32, col: u32 },   // renders as `Sheet1!R12C3`
    /// Paged only, and ONLY when the canvas declares a coordinate system.
    Geometric { canvas: u32, bbox: QRect },
}
```

**Resolution precedence** (matches Ethos's existing order, and the reason `Geometric` is last is
that a rectangle is the weakest identity — a box can contain the wrong thing after any reflow):

```
Cellular ≻ Tabular ≻ Structural(span+chars) ≻ Structural(span) ≻ Structural(element) ≻ Geometric
```

Mapping onto Ethos's shipped `EvidenceTier`: `Cellular`/`Tabular` → `TableCell`;
`Structural` with span → `ExactSpan`; `Structural` element-only → `ElementScoped`; `Geometric`
with no resolved element → `PageScoped`. **No new tier is required.** That is deliberate — the
side project must not force a `contract-change` PR on the verifier to be useful.

### 7.4 Geometry: absence is typed, never a sentinel

The rule from `docs/bring-your-own-parser.md` — *"a zero-area box is rejected outright, so a
`[0,0,0,0]` sentinel will not get you through"* — becomes unrepresentable rather than merely
rejected:

```rust
pub enum Geometry {
    Present { bbox: QRect, derivation: DerivationClass },
    /// Typed absence. The reason is what surfaces as a capability limit.
    Absent(NoGeometry),
}

pub enum NoGeometry {
    /// The canvas has no 2-D extent at all (Flow, Grid). Correct and permanent.
    NoCanvas,
    /// Canvas is Paged but this reader did not report a box for this node.
    NotReportedByReader,
    /// Reader reported a box, but the canvas declares no coordinate system, so it is
    /// uninterpretable. (This is exactly today's real-ODL case.)
    UndeclaredCoordinateSystem,
    /// Deliberately dropped by profile policy.
    SuppressedByProfile,
}
```

There is no constructor that yields a zero-area or page-sized `Present`. `QRect::new` already
rejects `x0>x1`/`y0>y1` (`crates/ethos-core/src/geom.rs:86`); DocIR adds positive-area and
in-canvas checks at construction.

**One extra field that the ODL evidence proves is necessary.** §5 shows the current adapter
derives page extents from the hull of observed boxes, which makes the in-page containment check
vacuous. DocIR therefore records where the canvas extent came from:

```rust
pub enum CanvasExtentSource { DeclaredBySource, MeasuredFromSourceBytes, DerivedFromContentHull }
```

`DerivedFromContentHull` must disable containment checking and raise a capability limit, because
containment against a hull is a tautology. That is a real bug class the current design cannot
express.

### 7.5 Provenance, per node

```rust
pub struct Provenance {
    pub reader: ReaderId,              // "pdfium" | "lopdf" | "ooxml" | "calamine" | "ocr" | ...
    pub reader_version: String,
    pub profile_sha256: String,        // binds backend build + fonts + thresholds
    pub derivation: DerivationClass,
    pub agreement: Option<AgreementRef>, // concord record, when a second reader ran
}
```

Per-node, not per-document, because a single DocIR legitimately mixes classes: born-digital text
on pages 1–4 (`Extracted`) and an OCR'd scan on page 5 (`Recognized`). A document-level flag
would force the whole document down to the weakest node, which is exactly the over-conservatism
that makes honest tools unusable.

### 7.6 Stable IDs

Reuse `ids-v1` verbatim (`crates/ethos-core/src/ids.rs`) — `p%04d`, `e%06d`, `s%06d`, `t%04d`,
all functions of canonical order. Extended per canvas family:

| Canvas | Element order rule | Native locator string |
| --- | --- | --- |
| Paged | reading order, canvas ascending (as today) | `p0003/e000042` |
| Flow | **document order** — the XML sequence in `word/document.xml`. No layout engine, no font metrics, fully deterministic | `f0001/e000042` |
| Grid | sheet ascending, then row-major | `Sheet1!R12C3` |

For Flow, document order is the entire reading-order algorithm, which is why DOCX is genuinely
the cheapest format to add and why `bring-your-own-parser.md` sequences it first. That guidance
is correct and this design does not second-guess it.

### 7.7 Mapping Markdown-first tools in without inventing geometry

This is the question the brief flags, and the answer is a hard boundary.

A Markdown-first tool (Anydoc; Docling's Markdown export; any `MarkItDown`-class converter)
yields, at best, **document order + block roles + table grids**. In DocIR terms it produces:

- `Canvas::Flow` — *even when the input was a PDF*.
- `Geometry::Absent(NoCanvas)` on every node when the tool discarded position;
  `Absent(NotReportedByReader)` when it had position and dropped it.
- `DerivationClass::Computed` for structure, `Extracted` for text.
- Locators: `Structural` only. Never `Geometric`.

**The trap, stated plainly:** a PDF run through a Markdown-first converter must *not* be
represented as `Canvas::Paged`. The moment you give it a page number, a consumer will reasonably
expect a page-anchored citation, and the tool cannot support one. Marking a PDF-sourced flow
document as `Flow` looks like information loss; it is actually the only honest encoding, and it
is what stops a citation UI from drawing a highlight box it has no coordinates for.

Where a converter *does* retain a page number but no box (Anydoc's PDF path via pdf-inspector
could easily retain one — §4 shows pdf-inspector emits `page` on every item), the honest encoding
is `Canvas::Paged` with real `index` but `Geometry::Absent(NotReportedByReader)` and no declared
`CoordSystem`. That yields `PageScoped` evidence and a capability limit. Correct, and strictly
better than nothing.

### 7.8 Wire format and the `contract-change` it implies

DocIR serializes as `ethos.docir.v0` under the existing c14n rules (integers only, sorted keys,
no floats). It is a *lab* artifact — versioned `v0`, explicitly unstable, and not something third
parties are invited to depend on yet.

Adapting DocIR → `ethos.grounding.v1` is lossless **only for Paged canvases with declared
coordinates**. For Flow and Grid, `ethos.grounding.v1` cannot carry it today, because of gates 1
and 4 (§2.6): `media_type` is `const "application/pdf"` and `bbox` is required. Closing that gap
is a `contract-change` PR against three things:

1. `media_type`: `const` → an enum of supported media types.
2. `bbox`: required → required **only when** `coordinate_system` is present.
3. `capabilities`: add the three fields currently *implied* (`fingerprint`, `coordinate_origin`,
   `crop_support`) because the implication breaks the moment a non-PDF canvas exists (§2.7).

That is the whole schema cost of multi-format. It is small, it is well understood, and — per
§2.5 — it should not be spent until the documented trigger fires.

---

## 8. Dual-read, OCR, and multi-format design

### 8.1 The dual-read protocol

Ethos cut corroboration for five stated reasons (§2.5). Three of them are answered by the
measurement below; two are answered by design.

| Objection from `proof-statement-v1.md` §7 | Answer |
| --- | --- |
| "two parsers sharing an upstream share failure modes" | **Answered by choice of reader.** pdf-inspector uses `lopdf` + its own `ttf-parser`/ToUnicode/base14 stack ([Cargo.toml:49](Cargo.toml:49)). It shares *no* code with PDFium. ODL uses veraPDF+PDFBox on the JVM. Three genuinely disjoint implementations exist. |
| "nobody has measured the divergence rate" | **This is the deliverable.** §10 specifies the harness; §5.5 reports the first datum. |
| "it doubles parse cost" | True, and correct: it is opt-in and diagnostic, never on the verification hot path. |
| "no external user has asked for it" | Still true. Not this memo's job to pretend otherwise. |
| "a rate near zero makes it not worth building" | **The most likely outcome, and that is a finding worth having.** See §8.3. |

#### What the secondary reader and the assist lane may propose

| Proposal | Permitted? | Rationale |
| --- | --- | --- |
| Reading-order permutation of existing elements | yes | order is `Computed`, not evidence |
| Element role / heading level / header-footer / caption | yes | a label, not the content |
| Table grid: assigning existing cells to (row, col) | yes | structure over existing text |
| Merge / split of existing elements | yes | segmentation is `Computed` |
| "This page is hard / scanned / failed" | yes | routing signal |
| **The text of an element** | **no** | text is the evidence; a model authoring evidence destroys the product |
| **The bbox of an element** | **no** | same |
| **A new element no deterministic reader saw** | **no** | except the OCR carve-out below |

**The OCR carve-out, stated precisely.** On a canvas where the deterministic reader found *no
text layer at all*, OCR may author new nodes at `DerivationClass::Recognized`. It may **never**
overwrite, correct, or delete an `Extracted` node. The scan-triage stage decides this, and its
threshold is pinned in the profile. That single rule is what stops OCR from "improving" a
born-digital page.

#### What agreement means — and the thing my spike proves you must not compare

I ran both readers on the same PDF (`fixtures/foreign/opendataloader/real/source.pdf`) and
compared the same glyph run (§5.5 has the full numbers):

| Quantity | ODL (veraPDF/PDFBox, JVM) | pdf-inspector (lopdf, Rust) | Divergence |
| --- | --- | --- | --- |
| Text (whitespace-normalized) | 445 chars | 445 chars | **identical** |
| Left edge | 200.891 pt | 200.89 pt | **0.001 pt = 0.1 centipoint** |
| Width | 193.261 pt | 193.26 pt | **0.001 pt** |
| Height | 38.194 pt | 32.02 pt | **6.174 pt** |
| Segmentation | 2 block elements | 7 line items | structural |

The 6.17 pt vertical gap is **not an error in either reader**. pdf-inspector reports the text
*baseline* and the *nominal font size*; ODL reports the *glyph ink box*. Back out the implied
metrics and you get ascender 0.952 em, descender 0.241 em — textbook font metrics, exactly right.
The two readers mean different things by "height."

Three consequences, and they are the core of the protocol:

1. **Compare origins and text, never boxes.** A naive box-IoU dual-read on this document reports
   permanent disagreement on every single element, at a magnitude that would swamp any real
   defect. It would be pure noise — precisely the failure mode §7 of the proof statement
   predicted.
2. **Ethos's existing geometry policy is independently validated.** `determinism-contract.md` §4
   already excludes precise bbox dimensions from `payload_sha256` and anchors spans on
   `origin_locator` (character origins from `FPDFText_GetCharOrigin`). My measurement says: the
   quantity Ethos chose to fingerprint is the one two independent stacks agree on to a tenth of a
   centipoint; the quantity Ethos chose to exclude is the one they disagree on by 6 pt. That
   decision was right, and now there is a number behind it.
3. **Segmentation divergence is the real signal.** 7 line items vs 2 blocks is not disagreement
   about the document; it is disagreement about granularity. The comparison must therefore run at
   a granularity-independent level.

#### The comparison algorithm

Do not attempt element-to-element matching across readers — it presumes the segmentation
agreement you are trying to measure. Instead, per canvas:

1. **Canonical text projection.** Concatenate each reader's text in its own reading order, apply
   the *same* normalization `ethos-verify` uses for quote matching. Compare.
   - `text_divergence` = normalized Levenshtein distance. **This is the headline number.**
2. **Origin multiset.** For each reader, emit `(quantized_x, quantized_baseline_y, first_char)`
   per text run. Compare as multisets.
   - `origin_divergence` = symmetric difference / union. Robust to segmentation.
3. **Order agreement.** Match runs by origin, then Kendall tau on the two orderings.
   - `order_divergence` = `1 - tau`.
4. **Segmentation divergence** = reported, never scored as error: `|blocks_A|` vs `|blocks_B|`,
   plus the boundary-refinement relation (is one a refinement of the other?).

Height, area, and IoU are **deliberately not compared.** Record that as a documented exclusion
with the 6.17 pt measurement as its justification, exactly as the determinism contract records
its own exclusion table.

#### Agreement, disagreement, and what happens to the bytes

- **Agree** (all divergences under pinned thresholds): `ethos.concord.v0` records agreement.
  The pinned DocIR is unchanged.
- **Disagree**: `ethos.concord.v0` records the divergence, located to canvas and origin. The
  pinned DocIR is **still unchanged**. Optionally the verification report gains a
  `review_recommended` advisory.
- **Incomparable** (one reader failed, or coordinate systems are undeclared): reported as
  incomparable. Never coerced into an agreement or a disagreement.

**Can dual-read ever affect `payload_sha256` or verification-critical bytes? No. Categorically,
and by construction:** concord consumes two finished DocIRs and emits a third artifact. It has no
write path into either. `AgreementRef` on a DocIR node is a *reference into the concord artifact*,
populated only in the diagnostic copy, and excluded from the stable payload projection by the same
mechanism `diagnostics` already uses (`determinism-contract.md` §3).

**Audit fields a developer needs to trust the boundary:** on every concord record — both reader
ids and versions, both `profile_sha256`, the source `sha256`, the divergence metric name and its
pinned threshold, and the exclusion list (what was deliberately not compared, and why).

### 8.2 OCR design

Covered structurally in §6.4: **OCR gets its own profile, so OCR output is fingerprint-
incomparable with born-digital output by the existing contract.** Beyond that:

- **Capability declaration.** An OCR-bearing DocIR declares `char_offsets: false` unless the
  engine emits real per-character boxes. PP-OCR emits **line-level** 4-point polygons, so the
  honest capability set is `spans: true` (lines are spans), `char_offsets: false`. Claiming
  char offsets from a line box is fabrication.
- **Geometry conversion.** OCR boxes are in *raster pixels* at a chosen DPI. Converting to
  centipoints requires the raster DPI, which must be pinned in the profile —
  `centipoints = pixels × 7200 / dpi`. At 300 DPI that is exactly 24 centipoints per pixel, an
  integer, so the conversion is exact and float-free. **Pin DPI to a value that makes
  `7200/dpi` an integer** (72, 100, 144, 150, 200, 240, 300, 400, 600, 720). This is a free win
  and it should be a hard constraint in the profile schema.
- **Rotation/skew.** If the OCR lane deskews, the geometry no longer refers to the source page.
  Either do not deskew, or record the inverse transform and apply it. Undocumented deskew is a
  silent geometry lie.
- **The `hidden text` signal.** ODL's schema carries a `hidden text` boolean described as
  "(e.g., OCR layer)". Scanned PDFs frequently already carry an invisible OCR text layer. The
  scan-triage stage must distinguish "no text" from "text present but invisible" — treating an
  existing OCR layer as `Extracted` would grant a prior OCR pass born-digital status. Classify a
  render-mode-3 text layer as `Recognized`, not `Extracted`. **This is a real trap and Ethos's
  reserved `hidden_text_detected` warning code already anticipates it.**

### 8.3 Multi-format design

Follow the DOCX → XLSX → PPTX sequencing already ruled in `bring-your-own-parser.md`. The
analysis there is correct and I found nothing to revise. Adding to it:

- **DOCX** is the cheapest and the most informative, because it exercises the geometry-free path
  end to end. `Canvas::Flow`, document order, `Geometry::Absent(NoCanvas)`. The verifier already
  supports this (`AnchorLevel::Text`, tests `geometry_free_*`); only the artifact schema blocks it.
- **XLSX** adds `Locator::Cellular`. Note the warning in `bring-your-own-parser.md` about column
  geometry — the same nominal 8.43-character column measures 4800 centipoints under Calibri 11
  and 5400 under Verdana 11. **Do not compute column geometry.** `R1C1` is a real locator and it
  is better than a fake box. `GroundingCell` already carries `row`/`col`/`row_span`/`col_span`.
- **PPTX** last, because gate 5 (positive-area, in-page bounds) rejects legitimately off-canvas
  shapes with negative coordinates. That is a real design question — is an off-slide shape a
  parse error or a security finding? — and it should be answered before, not during,
  implementation. My view: **security finding.** An off-canvas shape is exactly the hidden-content
  pattern `hallucination-threat-model.md` cares about, and Ethos already reserves
  `off_page_text_detected` for it.
- **EPUB / RTF / HTML / Markdown** are all `Canvas::Flow` and cost almost nothing once DOCX
  works — they differ only in the reader that produces `Vec<Block>`. Anydoc's IR (§4) is
  precisely this shape already.
- **CSV** is `Canvas::Grid` with one sheet. The only hard problem is encoding detection, which
  must fail closed rather than guess (`encoding_rs` will happily guess).

---

## 9. Language, packaging, and licence plan

### 9.1 Language: Rust for everything that touches evidence

**Decision: Rust core, no hybrid in v1. A Python OCR worker is the fallback, not the plan.**

The reasoning is not "Ethos is Rust, therefore Rust." It is that the entire candidate dependency
set for a locator-first multi-format reader is *already* pure Rust and permissive:

| Capability | Crate | Licence | Native deps |
| --- | --- | --- | --- |
| PDF object/content-stream parsing | `lopdf` 0.42 | MIT | none |
| Font/CID/ToUnicode | `ttf-parser` | MIT/Apache-2.0 | none |
| XLSX / ODS / legacy XLS | `calamine` | MIT | none |
| OOXML / ODF / EPUB XML | `quick-xml` | MIT | none |
| Container | `zip` (deflate only) | MIT | none |
| Legacy `.doc`/`.ppt` (CFB) | `cfb` | MIT | none |
| Encoding | `encoding_rs` | Apache-2.0/MIT | none |
| CSV | `csv` | Unlicense/MIT | none |
| OCR (preferred) | `oar-ocr` + `ort` | Apache-2.0 | ONNX Runtime (bundled or system) |

That table is the whole argument. Anydoc already demonstrates the stack works end to end
(§4), and pdf-inspector demonstrates the PDF half produces real coordinates (§5.5).

The alternative — Python for the parser — would import the exact problems the memo exists to
avoid: float-heavy numeric stacks, dependency graphs with unpinnable model weights, GC-driven
iteration-order surprises, and a footprint that makes the G2 30 MB gate unreachable forever.

**The one real cost, and it is a live one:** `lopdf` 0.42 uses let-chains and therefore requires
**Rust ≥ 1.88**. Ethos pins `channel = "1.87.0"` (`rust-toolchain.toml:5`) and
`rust-version = "1.87"` (`Cargo.toml:20`). I hit this directly — the build fails with
`error: rustc 1.87.0 is not supported` and then `E0658` inside `lopdf`. Adopting the
pdf-inspector/anydoc stack forces an Ethos MSRV bump, or pinning an older `lopdf`. That is a
small, concrete, decidable item — but it must be decided, not discovered.

**Where a process boundary is mandatory:**

| Component | Boundary | Why |
| --- | --- | --- |
| OpenDataLoader | subprocess (CLI jar) | JVM never in-process. Note the CLI jar **shades veraPDF classes into itself** (`pom.xml:66-92`), so "unmodified separate program" is inaccurate — prefer invoking a user-installed jar over redistributing one, and get a ruling before ever bundling it |
| Agent / VLM assist | subprocess or out-of-tree service | `deny.toml` bans network-capable crates in the base tree; the assist lane must not be linkable into anything Ethos ships |
| Python OCR (fallback only) | subprocess, JSON over stdio | keeps torch/onnxruntime-python out of the Rust dependency graph entirely |

### 9.2 Licence findings that constrain the plan

Observed, from the clones:

| Project | Code licence | Engine / weights | Verdict for Ethos |
| --- | --- | --- | --- |
| **pdf-inspector** | **MIT** (`LICENSE`, "Copyright (c) 2026 Firecrawl") | `lopdf` MIT, `ttf-parser` MIT/Apache-2.0. No native deps, no models | **Clean.** Safe to vendor, fork, or depend on |
| **anydoc** | **MIT** ("Copyright (c) 2026 Sideguide Technologies Inc.") | all-Rust permissive set above | **Clean** |
| **opendataloader-pdf** | **Apache-2.0** (Hancom, Inc.; was MPL-2.0 before 2.0) | **veraPDF MPL-2.0** — *"chosen from dual-licensed options"* (`THIRD_PARTY/THIRD_PARTY_LICENSES.md:4`) + **Apache PDFBox 3.0.4**; also **CDDL-1.1** via Jakarta Activation/JAXB. Tally: 211 MIT, 40 Apache-2.0, 14 ISC, 13 MPL-2.0, 12 BSD-3, 7 BSD-2, 6 EPL-2.0, 3 CDDL-1.1, 1 EPL-1.0, 1 EDL-1.0. **No GPL, no AGPL** | **Clean to invoke, JVM, and the CLI jar shades veraPDF into itself.** Subprocess against a user-installed jar; a ruling required before redistributing one |
| **pdf-inspector `external/bcmaps`** | MIT crate | **168 Adobe-derived binary CMaps**, BSD-3-Clause-style notice requiring binary-form reproduction, shipped in the published crate | Usable, but carries a **NOTICE obligation** if vendored |
| **docling** | **MIT** (`docling-slim` 2.119.0) | code MIT; **model weights are a separate question per model** and the default PDF pipeline is ML-in-the-loop | **Code clean; weights need a per-model ruling.** See §5 |
| **PyMuPDF** | AGPL | — | Already excluded by `landscape-log.md`: *"License-excluded as dependency (AGPL); benchmark row only"* |
| **Ghostscript** (via OCRmyPDF) | AGPL | — | Never vendor. Document as a user-run step only |
| **Surya / Chandra / dots.ocr** | code Apache-2.0 | **RAIL-M variants with revenue thresholds** | Excluded — use-restricted weights are not open licences |
| **EasyOCR** | Apache-2.0 | **gen2 recognition weights carry no licence statement anywhere** | Excluded on provenance grounds alone |
| **PP-OCRv5/v6** | Apache-2.0 | **weights Apache-2.0** on the HF model cards | **The pick.** Only candidate clean on both axes |

### 9.3 The licence gate to encode in CI

`deny.toml` already exists in Ethos and bans copyleft and network-capable crates in the base
tree. `ethos-lab` needs a *second*, stricter gate that Ethos core does not need — because the lab
handles model weights, which `cargo-deny` cannot see:

1. **`cargo-deny` in the lab**, allowing MPL-2.0 only for out-of-process components, denying
   GPL/AGPL everywhere.
2. **A weights manifest** — every model file listed with source URL, SHA-256, licence SPDX (or
   `UNLICENSED`, which is a build failure), and size. Modelled on RapidOCR's
   `default_models.yaml`, which already does SHA-256 pinning.
3. **A ruling on share-alike weights.** ocrs weights are CC-BY-SA-4.0. Redistributing them
   unmodified needs attribution and notice only; fine-tuning creates a share-alike obligation.
   Decide once, write it in an ADR, and let the gate enforce it.
4. **A hard rule:** no artefact under `ethos-lab/` may ever appear in an Ethos core
   `[dependencies]` block. Enforce with a CI grep, not a convention.

---

## 10. Benchmark harness design

### 10.1 The governing constraint

`docs/CLAIMS.md` §2, last row: *"Any speed, footprint, or parser-quality property — **No benchmark
has been run whose numbers we would defend.**"* And `landscape-log.md`: *"no figure, no claim,"*
with a suite-provenance policy (OmniDocBench neutral; ParseBench publisher-owned; *"Ethos
fixtures: never presented as neutral"*).

So the harness is designed to produce **defensible numbers on named corpora**, and every axis is
reported separately. There is no composite score. A single "parser quality" number is exactly the
vanity metric this project exists to distrust.

### 10.2 The metric nobody else publishes, and the reason to build this

**Geometry fabrication rate.** Of all emitted boxes, what fraction are not real?

```
fabrication_rate = |{b : zero_area(b) ∨ page_sized(b) ∨ ¬contains_own_ink(b)}| / |all boxes|
```

- `zero_area`: `x0==x1 ∨ y0==y1`.
- `page_sized`: box area ≥ 95% of canvas area while its text is < 50% of canvas text.
- `¬contains_own_ink`: render the element's text region and check the box actually bounds the ink,
  within a declared tolerance.

I already have a non-trivial reading for one competitor without building anything:
`docling/pipeline/vlm_pipeline.py:741-748` sets **every** item on the VLM path to
`BoundingBox(t=0,b=0,l=0,r=0)` with `charspan=[0,0]`, carrying the upstream comment
`# FIXME: would be nice not to have to "fake" it`. On that pipeline the fabrication rate is
**1.0**, by the authors' own admission. `dots_utils.py:172` and `chandra_utils.py:301` emit
`charspan=[0,0]` likewise.

This metric is the wedge. It is cheap to compute, it is unarguable, and it measures the exact
property Ethos's schema already enforces and everyone else's does not.

### 10.3 The nine axes

| # | Axis | Metric | Gold needed |
| --- | --- | --- | --- |
| 1 | **Citation grounding usefulness** | **tier-weighted grounding rate**: run gold (claim, locator) pairs through the real `ethos verify` and report the distribution over `EvidenceTier`, not just pass/fail. A parser that grounds everything at `PageScoped` scores far below one that grounds 80% at `ExactSpan` | gold citations |
| 2 | **Geometry honesty** | fabrication rate (§10.2); plus **origin agreement** vs a second reader in quantized centipoints | rendered pages |
| 3 | **Reading order** | Kendall tau vs gold order; plus **paragraph-integrity rate** (fraction of gold paragraphs emitted contiguously) | gold order |
| 4 | **Markdown usefulness for RAG** | **chunk-boundary purity**: fraction of chunks that neither split a gold sentence nor merge two gold sections. Plus header/footer contamination rate. Deliberately *not* an LLM-judged score | gold section map |
| 5 | **Multi-format coverage** | format × capability matrix; every cell is `supported` / `capability-limited` / `refused` / `wrong` — where **`wrong` is the only failing value.** A refusal is a pass | — |
| 6 | **OCR usefulness** | CER/WER vs gold text; box IoU vs gold boxes; and % of OCR'd pages where a gold quote grounds | scanned gold |
| 7 | **Determinism** | double-run byte identity (same host); **cross-platform** byte identity (macOS arm64 / Linux x64 / Windows x64, mirroring `determinism-contract.md` §11); and cross-CPU-feature identity for the OCR lane | — |
| 8 | **Latency / throughput / memory** | p50/p95 wall-clock per page, peak RSS, cold vs warm | — |
| 9 | **Fail-closed correctness** | **silent-success rate — target exactly 0.** On an adversarial corpus where a capability is genuinely absent, does the tool report a limitation or quietly return something plausible? | adversarial corpus |

Axis 9 is the one that would embarrass most tools, and it is the one Ethos should lead with,
because it is the only axis where "refuses to answer" is the winning behaviour.

### 10.4 Corpus taxonomy and minimum sizes

Ethos already has `fixtures/synthetic/` (simple-text, two-columns, table-regular-grid,
rotation-90, ligature-fi-embedded-font, hyphenated-line-break, list-items, heading-export,
two-lines) and `fixtures/failure/` (password-protected, corrupt-header-valid, invalid-header,
image-only-or-blank-page, memory-limit-simulated). That is a good *conformance* corpus and a poor
*measurement* corpus — the documents are too easy and too few.

Minimum viable measurement corpus: **240 documents**, licence-clean, redistributable.

| Bucket | Min docs | Why |
| --- | --- | --- |
| Born-digital single-column | 30 | baseline; divergence here should be ~0 |
| Multi-column academic | 30 | reading order is the whole game |
| Ruled tables | 25 | table grid inference |
| Unruled / whitespace tables | 25 | where table detection actually fails |
| Financial statements | 20 | dense numerics; value claims |
| Forms (AcroForm + flattened) | 15 | field/label association |
| Scanned, clean 300dpi | 25 | OCR baseline |
| Scanned, degraded / skewed | 15 | OCR failure modes |
| Rotated / mixed orientation | 10 | rotation normalization |
| Footnote-heavy | 10 | order + stitching risk |
| Right-to-left / CJK | 15 | reading order + font/cmap |
| Hostile (hidden text, off-page, tiny fonts) | 10 | security overlap |
| Non-PDF: DOCX / XLSX / PPTX / EPUB / CSV | 10 | geometry-free path |
| **Adversarial fail-closed** | 20 | axis 9 |

Gold annotation is the expensive part. Do **not** annotate all 240. Annotate **60** fully (gold
citations, gold order, gold section map) — stratified across buckets — and use the other 180 for
determinism, fabrication rate, latency, and cross-reader divergence, all of which need **no gold
at all**. That asymmetry is the reason the divergence metric is such good value: it is the only
quality signal available at corpus scale for free.

### 10.5 The comparison matrix

Rows: Ethos native (PDFium), pdf-inspector (lopdf), OpenDataLoader, Anydoc, Docling (standard
pipeline), Docling (VLM pipeline). Columns: the nine axes.

Two rules, both from existing Ethos policy:

- **Label suite provenance.** Ethos fixtures are never presented as neutral. OmniDocBench is
  neutral; anything publisher-owned is labelled.
- **Every comparator runs pinned** — exact version, exact flags, recorded. The ODL fixture
  manifest already models this well: parser, version, source sha256, output sha256,
  **generator artifact sha256**, and the exact command line.

### 10.6 Pass/fail gates for "integration-ready"

A lab reader may be proposed for optional Ethos integration only when **all** hold:

| Gate | Threshold |
| --- | --- |
| G-A determinism | 3× same-host byte-identical **and** byte-identical across the three CI platforms, on 100% of the corpus |
| G-B fabrication | fabrication rate **exactly 0** on the whole corpus. Not "low" |
| G-C fail-closed | silent-success rate **exactly 0** on the adversarial corpus |
| G-D grounding | tier-weighted grounding rate ≥ Ethos native on the born-digital buckets, with **no regression at `ExactSpan`** |
| G-E capability honesty | every declared capability is exercised by a test that fails if the declaration is wrong |
| G-F licence | `cargo-deny` clean; weights manifest complete with SHA-256 and named licences |
| G-G footprint | added installed size does not breach the G2 30 MB gate for the default feature set |

G-B and G-C are absolute rather than statistical on purpose. They are correctness properties, not
quality properties, and a project whose thesis is honesty cannot ship a 2% dishonesty rate.

### 10.7 What not to claim

Directly aligned with `CLAIMS.md`:

- Not "the most accurate parser." Not "faster than X" without the pinned command, corpus, and
  hardware.
- Not "verified by Ethos" for anything OCR- or assist-derived without the derivation class stated
  alongside.
- Not "supports N formats" when support means capability-limited text-only binding — say
  *"binds text claims; no geometry"*.
- Not a divergence rate as a *quality* claim. A low divergence rate means two readers agree; it
  does not mean either is right. Both can share a misconception about the same malformed PDF.
- Nothing at all until the gates in §10.6 are green and the corpus is named.

---

## 11. Workspace plan on this machine
> **⚠ Superseded on direction (see the banner at the top and §1b).** The separate-repo recommendation is reversed: the engine lives in Ethos, feature-gated. Only OCR weights, the assist lane and bench results stay outside. The current authority is `ethos-docushell-parser-plan.md` §9.


### 11.1 Repo strategy — separate repo, sibling directory

**Recommendation: a new git repo at `~/Desktop/Stuff/repo/ethos-lab/`, sibling to `ethos/`, not
inside it and not a worktree.**

| Option | Verdict |
| --- | --- |
| `experiments/` inside the Ethos repo | **No.** Ethos's `Cargo.toml` workspace, `deny.toml`, and CI would see the lab's dependencies. A `contract-change` discipline that exists to make change expensive should not be perturbed by a spike. The 30 MB footprint gate and the no-network-crates rule are workspace-level facts |
| Nested git worktree | **No.** Worktrees share history. This work will be thrown away and rewritten several times; that churn does not belong in the history of a project whose selling point is a frozen contract |
| Fork of Ethos | **No.** Invites drift and an eventual painful merge |
| **Separate repo, sibling dir** | **Yes.** Clean licence quarantine, independent CI, free to be messy. Interop is two files and a subprocess |

The lab reads Ethos as a *pinned published dependency* (`ethos-doc-core = "0.6"` with the
`grounding` feature) and as a CLI binary on `$PATH`. Never a path dependency — a path dependency
would let lab changes silently affect Ethos behaviour, which is the exact coupling to avoid.

### 11.2 Directory layout

```
~/Desktop/Stuff/repo/ethos-lab/
├── Cargo.toml                      # own workspace; MSRV 1.88 (see §9.1)
├── LICENSE                         # Apache-2.0, matching Ethos
├── THIRD_PARTY/                    # licence tally + notices, modelled on ODL's
├── deny.toml                       # stricter than Ethos core's
├── README.md                       # states loudly: EXPERIMENTAL, not an Ethos deliverable
├── crates/
│   ├── docir/                      # §7. The IR + c14n + ids. Zero heavy deps
│   ├── docir-json/                 # ethos.docir.v0 wire format + validator
│   ├── reader-lopdf/               # PDF via pdf-inspector/lopdf  → DocIR(Paged)
│   ├── reader-flow/                # DOCX/ODT/RTF/EPUB via quick-xml → DocIR(Flow)
│   ├── reader-grid/                # XLSX/ODS/CSV via calamine     → DocIR(Grid)
│   ├── reader-odl/                 # SUBPROCESS client for the ODL jar → DocIR(Paged)
│   ├── reader-ocr/                 # oar-ocr/ort lane              → DocIR(Recognized)
│   ├── concord/                    # ★ the divergence harness — the M0 product
│   ├── docir-to-grounding/         # DocIR → ethos.grounding.v1
│   ├── docir-to-markdown/          # DocIR → Markdown (chunking)
│   ├── docir-overlay/              # DocIR + raster → debug bbox overlay (diagnostic)
│   └── lab-cli/                    # `lab read|concord|adapt|bench`
├── assist/                         # OUT OF THE CARGO WORKSPACE on purpose
│   └── proposer/                   # agent/VLM client; network lives only here
├── models/
│   └── manifest.toml               # every weight: URL, SHA-256, licence SPDX, size
├── corpus/
│   ├── manifest.json               # sha256 + provenance + licence per document
│   └── gold/                       # gold citations, order, section maps
├── bench/                          # harness + result goldens
└── docs/
    ├── ADR-L000-scope-and-non-goals.md
    ├── ADR-L001-derivation-class.md
    └── divergence-report-*.md      # ★ the artifacts that feed the Ethos decision
```

Two structural points worth defending:

- **`assist/` is outside the Cargo workspace.** Not a feature flag, not an optional dependency —
  outside. It is the only place network code exists, and `cargo build` at the workspace root can
  never pull it in. This is a stronger guarantee than a lint.
- **`corpus/` stores a manifest, not blobs.** Documents are fetched and SHA-256-verified. Keeps
  the repo small and makes licence provenance auditable per document, which matters if any
  numbers are ever published.

### 11.3 Interop contract with Ethos

Exactly three surfaces, all versioned, none of them a Rust link-time dependency:

1. **`ethos.grounding.v1` JSON** — the lab writes it; `ethos grounding check` and `ethos verify
   --grounding` consume it. This already works today and needs no Ethos change for PDF.
2. **`ethos.docir.v0` JSON** — lab-internal, `v0`, explicitly unstable. Ethos never reads it.
3. **`ethos.concord.v0` JSON** — the divergence artifact. Consumed by humans and by the decision
   process, not by the verifier.

Round-trip test, runnable from day one and the lab's primary integration gate:

```bash
lab read --format auto doc.pdf --out doc.docir.json
lab adapt grounding doc.docir.json --out doc.grounding.json
ethos grounding check doc.grounding.json --source-artifact doc.pdf
ethos verify doc.grounding.json --citations gold.json --fail-on-ungrounded
```

If that pipeline is green, the lab is producing honest output by Ethos's own definition — because
`ethos grounding check` is the validator that enforces all five gates.

### 11.4 What must never leak into Ethos core

| Never | Why |
| --- | --- |
| Any `ethos-lab` crate in an Ethos `[dependencies]` | CI grep, not a convention |
| Model weights, of any licence | Footprint gate, plus weights are not code and `cargo-deny` cannot see them |
| The JVM, or a bundled ODL jar | Subprocess only |
| `ort` / ONNX Runtime / torch | OCR is a separate lane with a separate profile |
| Any network-capable crate | `deny.toml` already bans it; the assist lane lives outside the workspace |
| MPL-2.0 sources compiled into an Ethos binary | veraPDF is invoked as a separate process, never linked — and never redistributed without a ruling on the shaded jar |

---

## 12. Milestone roadmap
> **⚠ Superseded on direction (see the banner at the top and §1b).** Replaced by a phased roadmap that builds the parser as the main goal. The current authority is `ethos-docushell-parser-plan.md` §10.


### 12.1 What to prototype first, for maximum learning per week

**M0 is `concord`, and it is not a parser.** Two binaries already exist on this machine — the
Ethos CLI and a `pdf-inspector` build I produced in ~5 minutes — and a third (ODL) is a
`pip install` away. The entire M0 deliverable is: run all three over a corpus, project each into
canonical text and quantized origins, and report divergence.

The reason this is the highest-value first week is unusually concrete: it is the *only* work item
that directly satisfies a documented Ethos re-entry condition (*"Revisit only with a measured
divergence number"*). Everything else in this memo is optional. This one produces a decision.

And it is cheap because §10.4's asymmetry holds: divergence needs **no gold annotations**. You
can run it over 200 documents the day you have 200 documents.

### 12.2 Milestones

| ID | Deliverable | Exit criterion |
| --- | --- | --- |
| **M0** | `concord` over 3 readers; text + origin + order divergence; first divergence report | A number, with a named corpus, in `docs/divergence-report-001.md` |
| **M1** | `docir` + c14n + ids-v1 + `DerivationClass` + `Canvas` | Property tests: c14n idempotent, no floats, ids stable under permutation of internal iteration order |
| **M2** | `reader-lopdf` → DocIR → `ethos.grounding.v1` → `ethos verify` round trip | The §11.3 pipeline green on the born-digital buckets; fabrication rate 0 |
| **M3** | `reader-flow` (DOCX), then `reader-grid` (XLSX) | Geometry-free path proven end to end; `Locator::Cellular` grounds a real spreadsheet claim. **Requires the §7.8 schema `contract-change` to reach `ethos verify`** |
| **M4** | OCR lane under its own profile | An OCR'd page grounds a quote, and its fingerprint is provably incomparable with the born-digital parse |
| **M5** | Assist lane, proposals only | A proposal changes zero pinned bytes, demonstrated by byte-diff with and without the lane enabled |
| **M6** | Upstream decision | A written recommendation per component, with gate results from §10.6 |

### 12.3 30 / 60 / 90 days

**Days 1–30 — measure.**
1. Assemble corpus v0: 60 born-digital + 20 multi-column + 20 table-heavy, licence-clean, with a
   SHA-256 manifest. *This is the long pole. Start it on day 1 and let it run in parallel.*
2. Build `concord` v0: text projection + normalized Levenshtein; quantized-origin multiset; Kendall tau.
3. Wire three readers: Ethos CLI (needs `ETHOS_PDFIUM_LIBRARY_PATH` — currently unset on this
   machine per `ethos doctor`), `pdf2md --items-json`, ODL CLI.
4. **Deliverable: `divergence-report-001.md`.** State the rate, the corpus, the exclusions
   (heights are not compared, with the 6.17 pt measurement as the reason), and a recommendation on
   whether corroboration is worth building.
5. Decide the MSRV question (§9.1) — it blocks everything Rust.

**Days 31–60 — represent.**
6. `docir` + `docir-json` with c14n and property tests.
7. `reader-lopdf`: DocIR(Paged) from pdf-inspector's `TextItem` stream. **The real work here is
   deriving an honest ink box from baseline + font metrics via `ttf-parser`, or declaring
   `Geometry::Absent(NotReportedByReader)` and shipping origins only.** My spike says the origins
   are excellent and the heights are semantically incompatible — so ship origins first.
8. `docir-to-grounding` + the §11.3 round trip green on born-digital.
9. Fabrication-rate and fail-closed harnesses (axes 2 and 9). Run them against all comparators
   and publish the matrix internally.

**Days 61–90 — extend, one axis at a time.**
10. `reader-flow` for DOCX. Exercises `Canvas::Flow` and `Geometry::Absent(NoCanvas)`.
11. Draft the §7.8 `contract-change` PR against `ethos-grounding-source.schema.json` — three
    edits, well understood. **Do not merge it until the §2.5 trigger fires.**
12. OCR spike: `oar-ocr` + PP-OCRv5 mobile, own profile, integer-exact DPI. Run named spikes A
    (batch-size sensitivity), B (CTC word-offset stability), C (cross-CPU byte-diff).
13. `divergence-report-002.md` on the full corpus including scans.
14. M6 recommendation.

**Explicitly not in 90 days:** the assist/VLM lane, PPTX, HTML/EPUB, and any public claim.

### 12.4 What should eventually upstream, and what should not

| Component | Destination | Why |
| --- | --- | --- |
| `DerivationClass` as a concept | **Upstream**, eventually | It is a verifier concept, not a parser concept. But only once OCR exists to need it — adding it earlier is speculative contract surface |
| The three-edit schema `contract-change` (§7.8) | **Upstream when triggered** | Small, understood, already scoped in `bring-your-own-parser.md` |
| `CanvasExtentSource` (§7.4) | **Upstream** | It fixes a real bug class today: the vacuous containment check on hull-derived pages (§5) |
| `docir-to-grounding` adapter | **Upstream as an adapter**, alongside `opendataloader-json` | Adapters are already a first-class directory |
| `reader-lopdf` | **Optional Ethos feature, at best** | Only if it clears every §10.6 gate. Ethos does not need a second PDF reader to be a verifier |
| `concord` | **Stay in the lab, probably forever** | Its output is a research artifact. If corroboration ever ships, it ships as a small, focused thing — not this harness |
| OCR lane | **Lab, or a separate optional package** | Weights must never be near Ethos core |
| Assist / VLM lane | **Lab forever** | Network + nondeterminism. It has no business in a tree that bans network-capable crates |
| DocIR itself | **Lab forever** | Ethos already has a canonical model and `GroundingSource`. A second IR in core would be duplication, not progress |

---

## 13. Market position, risks, and anti-goals

### 13.1 Is "open-source parser + citation verifier" a real wedge?
> **⚠ Positioning superseded (see §1b).** The market read below still holds; what changed is that Ethos is now DocuShell's parser in fact, not only its verifier. Current positioning is in `ethos-docushell-parser-plan.md` §12.


**The parser is not the wedge. The wedge is that the verifier refuses to accept a parser's word
for anything, and every competitor's output demonstrates why that matters.**

Answering the brief's five questions directly:

**1. Is it differentiated vs Docling / Anydoc / OpenDataLoader alone?** Yes, but not on parsing.
All three are converters; none verifies. More importantly, none of them can *self-declare* the
things a citation needs. Concretely, from the evidence in this memo:

- OpenDataLoader's own `schema.json` has **no page dimensions anywhere** and declares bbox as
  `[left, bottom, right, top]` in prose only, so its artifacts cannot be converted to a top-left
  system without the source PDF. Ethos's fixture had to measure page geometry with an external
  `pdfinfo` call and record it in a sidecar.
- Docling's VLM pipeline stamps `[0,0,0,0]` on every element with a `# FIXME` admitting it is
  fake.
- Anydoc's IR has no page and no bbox at all, by design.

A converter's job ends at "here is the content." A verifier's job starts at "prove where it came
from." That gap is real and nobody is standing in it.

**2. What are developers actually missing?** Not another Markdown converter — that market is
crowded and commoditised. What is missing is: *my RAG pipeline cited page 4; is that citation
real, and can I prove it to an auditor tomorrow, offline, with no key?* Ethos already answers
that and is one of very few things that does. The gap in Ethos's own surface is narrower than
"parsing": it is that **on the foreign-parser path, five of six capabilities degrade** and the
strongest artifact field (`subject[1]`, the source document) is deliberately never emitted.

**3. Where should Ethos not compete?** Conversion UX and Markdown quality (Docling, Anydoc and
MarkItDown win, and it is not close). Hosted OCR SaaS. Layout-model accuracy leaderboards — that
is an ML research race with a different cost structure. Format breadth as a headline number.
Anything requiring model weights in the default install.

**4. What must be true for this to strengthen rather than dilute Ethos?** Four things, and they
are testable: (a) the parser remains optional and foreign parsers stay first-class; (b) no lab
artefact ever becomes an Ethos core dependency; (c) the first public output is a *measurement*,
not a feature; (d) the parser earns its place by moving `EvidenceTier` up, not by adding formats.
If a proposed change does not move a citation from `ElementScoped` to `ExactSpan`, or from
"unknown origin" to "declared origin", it is conversion work and belongs elsewhere.

**5. Recommended public narrative** — in this order, because the order *is* the positioning:

> **Ethos verifies citations. It reads documents so it can verify them better.**
>
> - The verifier is the product. Parsing is optional and always will be.
> - Bring your own parser: OpenDataLoader, Docling, or your own, through one adapter trait.
> - Evidence is locator-first. Every verdict states how precisely it was bound — and what could
>   not be established.
> - The primary path is deterministic and pinned. Same bytes, same profile, same bytes out,
>   offline, no key.
> - OCR and model assist are capability-limited lanes that can never masquerade as born-digital
>   certainty — enforced by profile identity, not by a disclaimer.

Do not say "advanced parser." Say "first-party reader, so Ethos can name the bytes it read."

### 13.2 Top 10 risks, ranked by severity

| # | Risk | Severity | Why it is real | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | **Moat dilution** — the lab becomes the project and Ethos becomes a parser with a verifier attached | **Critical** | It is the natural gravity of parser work; parsing is more fun and more visibly "done" | Ship the measurement before the parser. §12.4 pre-commits what never upstreams. Rule: no lab work that does not move an evidence tier |
| 2 | **The divergence number comes back near zero and the project loses its rationale** | **High** | Genuinely likely on born-digital documents — my N=1 spike already agrees to 0.1 centipoint | This is a *success*, not a failure: it retires an open question cheaply and confirms an existing ruling. Plan to publish it either way and stop |
| 3 | **Geometry semantics divergence read as parser error** | **High** | Measured: 6.17 pt on one heading, purely from baseline+em vs ink box | Compare origins and text, never heights/IoU. Document the exclusion with the measurement |
| 4 | **OCR output leaks into born-digital confidence** | **High** | The failure would be silent and would directly falsify Ethos's core claim | Separate profile ⇒ different fingerprint ⇒ non-comparable by the existing contract (§6.4). Plus `DerivationClass` |
| 5 | **Model-weight licence contamination** | **High** | Surya/Chandra RAIL-M revenue cliffs; EasyOCR weights have *no* licence statement at all | Weights manifest with SPDX; `UNLICENSED` fails the build. PP-OCR (Apache-2.0 weights) as the pick |
| 6 | **MSRV / toolchain fracture** | Medium | Confirmed: `lopdf` 0.42 needs Rust ≥1.88; Ethos pins 1.87 | Decide in week 1. Either bump Ethos, or pin older `lopdf`, or keep the lab on its own MSRV and interop by file only |
| 7 | **Corpus licensing** | Medium | Redistributable, gold-annotated document corpora are genuinely scarce, and unpublishable numbers are worthless | Manifest-and-fetch with per-document provenance. Budget real time for this — it is the long pole |
| 8 | **Upstream schema drift** | Medium | ODL element `id` is *optional in its own schema*, real output contains duplicate ids, and ids are not stable across ODL versions | Pin generator artifact SHA-256 (the existing fixture manifest already does this). Treat foreign ids as unstable by default |
| 9 | **Scope creep into format breadth** | Medium | "Just add PPTX" is always one afternoon away, and PPTX has an unresolved gate-5 question | DOCX → XLSX → PPTX, each gated on a real requirement. Resolve off-canvas coordinates before PPTX |
| 10 | **Publishing a number that cannot be defended** | Medium | `CLAIMS.md` explicitly forbids it and it would damage the one thing Ethos actually sells | §10.6 gates; §10.7 not-to-claim list; suite-provenance labels |

### 13.3 Anti-goals — written down so they can be pointed at

1. **Not a Markdown converter.** Markdown is an *export* of DocIR, never its centre. The moment
   Markdown quality drives IR design, the locators are gone.
2. **Not a layout-model competitor.** No leaderboard chasing, no training runs.
3. **Not a second canonical model inside Ethos.** DocIR stays in the lab.
4. **No model in the pinned path.** Ever. Assist proposes; the pinned path does not listen.
5. **No invented geometry.** No page-sized boxes, no `[0,0,0,0]`, no derived-from-hull page
   extents presented as declared page geometry.
6. **No silent degradation.** A missing capability is stated. Refusal is a valid, and often the
   correct, output.
7. **No network in anything shippable.** The assist lane lives outside the Cargo workspace.
8. **No public claim before the gates are green** and the corpus is named.

---

## 14. Final recommendations — forced choices
> **⚠ Superseded on direction (see the banner at the top and §1b).** Replaced. The current authority is `ethos-docushell-parser-plan.md` §12.


### 14.1 Build / wrap / steal / ignore, per capability

| Capability | Decision | What it means concretely |
| --- | --- | --- |
| **PDF text + coordinates (secondary reader)** | **WRAP → then FORK** | Use the `pdf-inspector` **CLI** for M0 (`--items-json`), against **current upstream 1.14.x**, not the 0.1.8 anydoc pins. Fork `reader-lopdf` at M2: its `TextItem` is a *placement record*, not a locator record — `x`/`width` are exact, `y` is a baseline, `height` is the font size, and items are per-Tj-run, so ink boxes and sub-run spans are yours to build. Note `extract_text_with_positions` takes a **path, not bytes** |
| **PDF scanned-vs-text routing** | **STEAL IDEA** | `detect_pdf_type` returned `TEXT-BASED, 100% confidence, OCR recommended NO` in 3 ms. Reimplement as pinned `TextLayerCoverage` with a profile-pinned threshold — do not inherit a tunable heuristic into a determinism contract |
| **Multi-format → structured content** | **STEAL IDEA (fork the shape, not the goal)** | Anydoc's `Block`/`Inline`/`Table`+`CellSlot` model is the right flow-document IR and it is MIT. Take the shape; add `Canvas`, ids, and provenance. Do not depend on it — its whole purpose is discarding what you need |
| **Multi-format file readers** | **WRAP** | `calamine`, `quick-xml`, `zip`, `cfb`, `encoding_rs` directly. No wrapper library needed |
| **Rich structured JSON with bboxes** | **COMPETE** | This is the product. ODL is the closest and its artifacts cannot self-declare page geometry or coordinate origin. Ethos's `ethos.grounding.v1` is already stricter and more honest |
| **OpenDataLoader as a reader** | **WRAP (subprocess)** | Keep the existing adapter. Add a `reader-odl` that shells the CLI so it can be a concord participant. Never in-process |
| **Docling** | **IGNORE as a dependency; INTEROP as a target** | MIT code, but ML-in-the-loop by default and its VLM path fabricates geometry. Write a `DoclingDocument → DocIR` adapter that *refuses* placeholder provenance. Treat as a benchmark row |
| **PDF/UA + WCAG structure signal** | **STEAL IDEA** | ODL's structure comes from accessibility tagging (veraPDF `wcag-algorithms`). `pdfua_tag` and `heading level` are exactly the structural signal the threat model wants, and the current adapter drops them because `GroundingElement` has nowhere to put them. DocIR should have somewhere |
| **OCR** | **WRAP (`oar-ocr` + PP-OCRv5/v6 mobile ONNX)** | Only candidate with Apache-2.0 on *both* code and weights. Pure Rust via `ort` — no Python, no subprocess |
| **Agent / VLM assist** | **DEFER** | Not in v1. See §14.5 |
| **Markdown / HTML export** | **BUILD (trivially)** | A deterministic projection of DocIR. Do not compete on quality |
| **Table structure inference** | **BUILD, later** | Ethos has `ADR-0010-deterministic-table-candidates` and a 1,175-line `ethos-tables`. Extending it beats importing anything |
| **PyMuPDF / Ghostscript / Surya / EasyOCR** | **IGNORE** | AGPL, AGPL, revenue-capped RAIL-M, and unlicensed weights respectively |

### 14.2 Primary language

**Rust, single language, for everything that touches evidence.** Python appears only as a
fallback OCR worker behind a subprocess, and only if the `ort` route fails. Rationale in §9.1; the
short version is that the whole candidate dependency set is already pure-Rust MIT with zero native
dependencies, which is a coincidence worth exploiting. **Blocking item: the 1.87 vs 1.88 MSRV
conflict, decided in week 1.**

### 14.3 Primary PDF coordinate strategy

**Keep Ethos's native geometry policy exactly as it is. It is already right, and my spike
produced the number that shows why.**

- Quantize at extraction to integer centipoints, top-left origin, half-away-from-zero. Unchanged.
- Keep precise bbox dimensions **out** of the fingerprint; keep `origin_locator` (character
  origins) **in**. Two independent PDF stacks agreed on horizontal origin to **0.001 pt** and
  disagreed on height by **6.174 pt**. Ethos fingerprints the agreeing quantity and excludes the
  disagreeing one. That was a judgement call in the determinism contract; it is now a measurement.
- **Do not adopt ODL's schema shape.** Bottom-left floats with no page dimensions is strictly
  worse, and the memo's own fixture proves it is unconvertible without the source PDF.
- **Steal one idea from pdf-inspector: `mcid`.** Its `TextItem` carries the marked-content id,
  which links a text run to the PDF tagged-structure tree. That is the honest bridge between
  geometric text runs and accessibility-derived structure — and it is how you would ever reconcile
  a pdf-inspector run with an ODL element.
- **Add `CanvasExtentSource`** (§7.4). The current hull-derivation makes in-page containment
  vacuous on the real ODL path, which is a live correctness gap, not a hypothetical.

### 14.4 OCR pick

**#1 PP-OCRv5/v6 mobile ONNX weights via `oar-ocr`/`ort`** — Apache-2.0 code *and* weights, ~22 MB,
pure Rust, deterministic CPU inference under a pinned envelope. **#2 `ocrs`** — pure Rust, 12 MB,
the only engine emitting word *and* character boxes natively, but CC-BY-SA weights, Latin-only, and
self-described "early preview." **Rejected: Surya** (RAIL-M revenue cliff + VLM decoding),
**EasyOCR** (weights with no licence statement), **Tesseract** (documented cross-platform
irreproducibility), **Ghostscript/OCRmyPDF** (AGPL), **Apple Vision** (unpinnable).

Pin DPI so `7200/dpi` is an integer — the pixel→centipoint conversion is then exact and float-free.

### 14.5 Does agent/VLM assist belong in v1?

**No. Defer to M5 at the earliest, and treat it as optional forever.**

Three reasons, in order of weight: (1) the assist lane cannot touch the pinned path by
construction, so its only possible value is diagnostic — and the *deterministic* secondary reader
already provides a better diagnostic at a fraction of the cost and none of the nondeterminism;
(2) `deny.toml` bans network-capable crates in the base tree, so the lane is architecturally
peripheral no matter how good it is; (3) building it early would invite exactly the moat dilution
that is risk #1.

The honest framing: **the dual-read lane is the assist lane.** A second deterministic reader
disagreeing with the first is a better "this page is hard" signal than a model saying so, because
it is reproducible. Build that; revisit VLM only if measured divergence identifies a class of
pages that *neither* deterministic reader handles.

### 14.6 What upstreams and what does not

Table in §12.4. The one-line version: **concepts upstream, code mostly does not.**
`DerivationClass`, `CanvasExtentSource`, and the three-edit schema `contract-change` are verifier
concepts that belong in Ethos when triggered. DocIR, `concord`, the OCR lane, and the assist lane
stay in the lab. `reader-lopdf` upstreams only if it clears every gate in §10.6 — and Ethos does
not need a second PDF reader to be a verifier.

### 14.7 The single most important recommendation

**Spend the first 30 days producing `divergence-report-001.md` and nothing else.**

It is the only work item that satisfies a written Ethos re-entry condition. It needs no gold
annotations, no new IR, no schema change, and no OCR. Two of the three readers are already built
on this machine. And it has the rare property that **both outcomes are valuable**: a near-zero
rate retires corroboration permanently and cheaply, with evidence; a non-zero rate is the
strongest possible argument for everything else in this memo — and, unlike a threat model, it is
the argument `docs/proof-statement-v1.md` §7 explicitly said it would accept.

---

## 15. Appendix

### 15.1 Commands run, and what they established

```bash
# Ethos: environment
./target/release/ethos --version                  # ethos 0.6.0
./target/release/ethos doctor                     # ETHOS_PDFIUM_LIBRARY_PATH: unset
                                                  # => native PDF path NOT runnable here as-is

# Ethos: real OpenDataLoader round trip, run twice
./target/release/ethos verify \
  fixtures/foreign/opendataloader/real/opendataloader-output.json \
  --grounding opendataloader-json \
  --citations fixtures/foreign/opendataloader/real/citations.json \
  --out run1.json                                 # sha256 445fada4a0598e29…
# repeated                                        # sha256 445fada4a0598e29…  (byte-identical)
# vs committed golden: identical except `attestation` and `evidence_tier`,
# which the built binary predates (uncommitted proof-statement-v1 work)

# External repos
git clone --depth 50 https://github.com/firecrawl/pdf-inspector
git clone --depth 50 https://github.com/firecrawl/anydoc
git clone --depth 50 https://github.com/opendataloader-project/opendataloader-pdf
git clone --depth 50 https://github.com/docling-project/docling
# sizes: anydoc 4.4M · pdf-inspector 25M · opendataloader-pdf 68M · docling 350M

# MSRV finding
cargo build --release --bins            # error: rustc 1.87.0 is not supported (time)
cargo update time@0.3.55 --precise 0.3.41
cargo build --release --bins            # error[E0658] in lopdf (let-chains) => needs >= 1.88
rustup toolchain install 1.88.0 --profile minimal    # additive; default unchanged
cargo +1.88.0 build --release --bins    # Finished in 4m30s => detect-pdf, pdf2md, dump_ops

# The divergence spike
./detect-pdf  <ethos>/fixtures/foreign/opendataloader/real/source.pdf
#   => TEXT-BASED, Confidence 100%, OCR recommended: NO, 3ms
./pdf2md      <same>.pdf                          # markdown, 3 runs byte-identical
                                                  # sha256 87c6223a292cd1c8… ×3
./pdf2md      <same>.pdf --items-json             # 7 positioned TextItems
```

### 15.2 Key files inspected

**Ethos** (`/Users/saumild/Desktop/Stuff/repo/ethos`)

| File | What it established |
| --- | --- |
| `crates/ethos-core/src/grounding.rs` | the whole parser boundary; serde-only; safe defaults |
| `crates/ethos-core/src/geom.rs:42` | `quantize()`, the only permitted float math |
| `crates/ethos-core/src/ids.rs` | ids-v1 formats and ordinal rules |
| `crates/ethos-core/src/verify_types.rs:168,406,442` | `Citation`, `EvidenceTier`, `Attestation` |
| `crates/ethos-verify/src/lib.rs:1485-1594` | real locator precedence + `LocatorConflict` |
| `crates/ethos-core/src/model.rs:639-665` | `stable_payload_projection` strips bbox before hashing |
| `crates/ethos-tables/src/lib.rs` | whitespace-only table detection; its limits |
| `schemas/ethos-grounding-source.schema.json` | gates 1, 3, 4; bbox `x1≥1`, `y1≥1` |
| `docs/determinism-contract.md` | c14n v1, quantization, exclusion table, ids |
| `docs/CLAIMS.md` | what is and is not proven; the benchmark prohibition |
| `docs/proof-statement-v1.md` §1.3–1.4, §7 | representation-hash rule; corroboration and multi-format cuts |
| `docs/bring-your-own-parser.md` | the five gates; DOCX→XLSX→PPTX sequencing |
| `adapters/grounding/opendataloader-json/src/lib.rs` | 1,781 lines; the real cost of a foreign adapter |
| `fixtures/foreign/opendataloader/real/*` | **the single most useful artifact in the repo** |
| `docs/landscape-log.md` | watchlist, suite-provenance policy, PyMuPDF AGPL exclusion |
| `deny.toml` | licence allowlist, no-network rule |

**External**

| File | What it established |
| --- | --- |
| `pdf-inspector/Cargo.toml:49` | `lopdf` 0.42 + rayon — **not PDFium** |
| `pdf-inspector/src/types.rs:88-136` | `TextItem{x,y,width,height: f32}`, `PdfRect`, `PdfLine` |
| `pdf-inspector/src/detector.rs` | `PdfType`, `ScanStrategy`, `DetectionConfig` |
| `anydoc/Cargo.toml` | calamine, cfb, csv, quick-xml, zip, **pdf-inspector 0.1.8** |
| `anydoc/src/model/*.rs` | flow IR: no page, no bbox, `CellSlot::{Origin,Covered}` |
| `opendataloader-pdf/schema.json` | `[left, bottom, right, top]`; `id` optional; **no page dimensions** |
| `opendataloader-pdf/THIRD_PARTY/THIRD_PARTY_LICENSES.md:4,109,235-242` | veraPDF MPL-2.0 + PDFBox 3.0.4; WCAG algorithms; no GPL |
| `docling/pipeline/vlm_pipeline.py:741-748` | `BoundingBox(t=0,b=0,l=0,r=0)`, `charspan=[0,0]`, `# FIXME … "fake" it` |
| `docling/utils/dots_utils.py:172`, `chandra_utils.py:301` | further `charspan=[0,0]` sites |
| `docling/pyproject.toml:6-9,51` | `docling-slim` 2.119.0, MIT, `docling-core>=2.91` |

### 15.3 Open questions — "unknown, need spike X"

| ID | Question | Why it blocks something |
| --- | --- | --- |
| **S1** | What is the divergence rate on a real corpus (not one lorem page)? | The entire corroboration decision. §12.3 days 1–30 |
| **S2** | Can `ttf-parser` + pdf-inspector's font data yield an ink box matching PDFium's to ≤1 centipoint? | Decides whether `reader-lopdf` can ever emit `Geometry::Present`, or must ship origins only |
| **S3** | Is pdf-inspector's `f32` pipeline byte-stable across x86-64 AVX2 / AVX-512 / aarch64? | Whether a second reader can be `pinned` at all, or is diagnostic-only forever |
| **S4** | Does ODL's `--threads` change output *order*? | Whether ODL can ever be a pinned participant |
| **S5** | Are ODL element `id`s stable across versions? (Two versions are pinned side by side, suggesting this was already suspected) | Whether stored citations survive an ODL upgrade |
| **S6** | Does PP-OCR `rec_batch_num` / batch composition change recognition output? | OCR determinism envelope |
| **S7** | Are CTC-derived word offsets stable under cross-CPU float variance? | Whether OCR can ever claim word-level spans |
| **S8** | Cross-machine byte-diff of PP-OCRv5 mobile, single-threaded | What the OCR determinism claim may say out loud |
| **S9** | Is Docling's *standard* (non-VLM) pipeline's `charspan` real, or also placeholder? | Whether a Docling adapter is possible at all above `ElementScoped` |
| **S10** | Does `mcid` reliably bridge pdf-inspector text runs to ODL's PDF/UA elements? | Cross-reader element matching without geometry |
| **S11** | What is the real distribution of gate-5 violations (negative/off-canvas coords) in PPTX? | Prerequisite to PPTX support |
| **S12** | Bump Ethos MSRV to 1.88, or pin an older `lopdf`? (Confirmed: `pdf-inspector` alone does **not** build on 1.87 — reproduced twice, `E0658` in `lopdf/src/object.rs:473`) | Blocks all Rust work in the lab |
| **S13** | Does a `ttf-parser`-derived ink box from pdf-inspector's baseline+advance match PDFium's rectangle closely enough to be worth emitting at all — or should `reader-lopdf` ship origins only and declare `Geometry::Absent`? | The single design decision that determines whether a second reader can ever be `pinned` rather than diagnostic |
| **S14** | Does ODL's shipped `samples/json/*.json` match real output from the same version? (The adversarial pass found the repo sample and real 2.4.7 output on the identical PDF differ) | Whether ODL's own samples can be trusted as schema documentation |

### 15.4 Method and its limits — stated plainly

- **Observed fact** = read in source or produced by a command in §15.1.
- **Inference** = reasoned from observed fact, and labelled as such in the text.
- **Recommendation** = my judgement.

**Each external teardown was then adversarially re-verified against the source by a second
reader instructed to refute it.** That pass materially changed this report, and the corrections
are worth listing because they show which kinds of claim were fragile:

| Corrected | From | To |
| --- | --- | --- |
| pdf-inspector size | "~26k LOC" | **82,970** (61,270 excl. generated tables) |
| pdf-inspector `/CropBox` | "never read" | **read** by `get_page_box()`, but unused for the origin/flip — two incompatible page-box notions |
| pdf-inspector item granularity | "per-glyph" | **per text-show operation (Tj/TJ run)** |
| pdf-inspector `width == 0.0` | implied material | **4 items in ~63,000 (0.006%)** — a footnote |
| pdf-inspector bcmaps | 176 files | **168**, plus an Adobe NOTICE obligation |
| Docling standard-path bbox | "handed to neural networks" | **measured** union of docling-parse cell rects; ML decides grouping/labels only |
| Docling provenance | "nothing records the producer" | `DoclingVersion` **is** recorded; the *pipeline* and model revision are not |
| Docling parallelism | nondeterminism source | **not at defaults** (batch sizes and concurrency = 1) |
| ODL veraPDF | "unmodified separate program" | **shaded into the CLI jar**; CDDL-1.1 deps also present |
| anydoc reading-order stability | one crate-level score | **split**: office verified, PDF half inherits pdf-inspector |

My own spike (§5.5) was independently corroborated: the second reader also found `height == font_size`
exactly on every item and `y` at the baseline, from the construction site rather than from my
arithmetic.

Three limits worth naming:

1. **The divergence spike is N=1** on a single-page, single-column, table-free lorem PDF. It
   demonstrates the *method* and produces a real measurement of a real semantic gap. It is **not**
   the divergence rate, and nothing in this memo should be read as claiming it is.
2. **The native Ethos PDF path was never executed.** `ETHOS_PDFIUM_LIBRARY_PATH` is unset on this
   machine, so every statement about the native parser comes from source reading, not behaviour.
3. **Docling and OpenDataLoader were not run.** Their behaviour here is from source, schemas,
   committed goldens, and the pinned real-output fixture in the Ethos repo.

---

## 16. Pass 2 — ethos-engine readiness (2026-08-12)

This section is the **current authority for ethos-engine bootstrap decisions**. It reconciles the
pass-1 evidence above and the DocuShell parser+verifier plan with three clarifications from the
decider: build a fresh sibling `~/Desktop/Stuff/repo/ethos-engine/`; consider pdf-inspector-class
PDF features for v0; make ethos-engine easy for OSS adopters. It also folds in
`docushell-repo/docs/WORKBENCH_ARCHITECTURE.md`, which was not read in pass 1 and which changes
several answers.

### 16.1 What changed since pass 1 and the plan

| Topic | Pass-1 / plan position | Pass-2 position |
| --- | --- | --- |
| Where the engine lives | Extend Ethos in place (plan §3.1) | **Greenfield sibling `ethos-engine/`.** Reuse Ethos *contracts and fixtures*, not its crate tree |
| The canonical model | Design our own (`Canvas`, `payload.structure`, …) | **Emit DocuShell's `DocumentRepresentation v0`.** It is already specified in the companion document, down to `NativeLocator` / `StructuralLocator` / `TableCellPosition`. Do not invent a competing shape |
| Geometry | "typed `Geometry`, absence is a type" | Still true, and now *justified by the spec*: `DocumentRepresentation v0` makes **`NativeLocator` required and `RenderedLocator`/geometry optional, "for inspection."** Geometry was never the primary locator |
| Fidelity check | Cross-parser divergence (concord) | **Intra-representation locator cross-check** is better, cheaper and deterministic: carry geometric *and* structural addresses derived independently, then test them against each other. Cross-parser divergence drops to a background diagnostic |
| Confidence | Not discussed | **No public confidence field at all.** Workbench rule 9, and measured evidence below |
| Verify | Ambiguous | **Staged: v0 does not verify.** See §16.6 |
| Markdown + anchor map | A headline v1 deliverable | **Deferred out of v0.** Workbench rule 8 says retrieval must operate on the evidence record itself; a Markdown projection is exactly the thing rule 8 warns about, and the anchor map is the mitigation for a projection v0 does not need yet |
| Tables | Phase-B gate | Still the hardest quality item, still not in v0. The **locator cross-check** is what makes it testable when it lands |
| OCR pick | PP-OCRv5/v6 ONNX | Unchanged as a *deterministic* lane — but note DocuShell has already named **GLM-OCR (MIT, 0.9B)** as its Part II accuracy-lane candidate, "pending product and security approval". These are different lanes with different rules; see §16.9 |

### 16.2 The rules pass 1 did not have

`WORKBENCH_ARCHITECTURE.md` "Rules you must not break" binds ethos-engine. Four are new constraints
on this design:

- **Rule 8 — retrieval operates on the evidence record itself.** *"Any projection between what is
  ranked and what is cited is a place a locator dies silently… a projection that dropped
  `tableCell` made a question the document plainly answered unanswerable, and took four deploys to
  trace."* This is why Markdown-for-RAG leaves v0.
- **Rule 9 — confidence is not a gate.** *"Routing on a threshold presents an uncalibrated number
  as a safety control."*
- **Rule 7 — the drafting path and the verifying representation must be independently derived.**
  *"If extraction moves to a VLM that reads page pixels directly, the extraction and the parsing
  are the same step — and verification against the parser output is verification against the
  model's own work."*
- **Rule 3 — never invent an identifier, locator, coordinate, or fingerprint.**

And the **L0–L6 trust ladder** places ethos-engine precisely: it produces **L0 (source
registered), L1 (extracted), L2 (locatable)**. **L3 (grounded) is Ethos's and only Ethos's.**
Note that L1's achievement condition names capability declarations explicitly — *"a versioned
processor/profile produced a representation with declared capabilities and an extraction-assurance
state"* — so capability declarations are not polish, they are the L1 gate.

**The collapse ethos-engine must never perform:** emitting one field that means "this document is
good."

### 16.3 pdf-inspector for PDF v0 — measured, not assumed

I ran the built `detect-pdf` binary over all 45 PDFs in the Ethos tree (best of 5, warm cache,
macOS x86_64). **Observed fact:**

| Document | Size | Pages | Sampled | Internal detect | Process wall-clock |
| --- | --- | --- | --- | --- | --- |
| nist-sp-800-53r5 | 5.9 MB | **492** | **8** | **447 ms** | 473 ms |
| nist-sp-800-63b | 1.4 MB | 80 | 8 | 43 ms | 64 ms |
| cfpb-home-loan-toolkit | 1.5 MB | 28 | 8 | 20 ms | 41 ms |
| irs-form-1040-2025 | 215 KB | 2 | 2 | 12 ms | 31 ms |
| ODL `source.pdf` | 10 KB | 1 | 1 | **1 ms** | 21 ms |
| synthetic fixtures (×30) | ~1 KB | 1–2 | 1–2 | **0–1 ms** | 18–22 ms |

**Is "10–50 ms" realistic? No, and the reason is a bug, not a benchmark disagreement.**

1. **The "10–50 ms" figure is a doc-comment, not a measurement.** It originates at
   `src/lib.rs:390` — *"Returns the PDF type and which pages need OCR (~10-50ms)"* — and is
   propagated into five READMEs. No benchmark exists in the repo to support it.
2. **The advertised page sampling does not bound cost.** `DetectionConfig::default()` is
   `ScanStrategy::Sample(8)` and Phase 1 honours it — `pages_sampled` is 8 even on the 492-page
   file. But **Phase 3 re-scans every page** (`detector.rs:431-447`): for a `TextBased` document
   `pages_needing_ocr` is empty, so `0 < total_pages` holds and `analyze_page_content` runs on all
   492. **Proved by controlled experiment, not inference** — on the same file, `Pages(vec![1])`
   costs **439 ms**, `Sample(8)` **412 ms**, `Full` **434 ms**. Asking for one page costs the same
   as asking for all of them.
3. **So cost scales with page count**, at roughly **0.4–0.55 ms per page** on top of parse — not
   with file size, as I first inferred from the CLI numbers alone. Two files of near-identical size
   diverge 18 ms (28 pages) vs 85 ms (269 pages); a 430-page file spends 97% of its time in
   per-page work with only 6.4 ms of parse.
4. **~19–22 ms of every CLI number is process spawn** (measured: min 19.5 ms, median 22.4 ms for a
   no-op invocation of the 7 MB binary). In-process, small files cost **0.05–0.4 ms**.

**Recommendation:** do not carry the 10–50 ms claim forward in any form. ethos-engine's own
classifier should be **genuinely bounded** — sample *N* pages and stop — which is a property this
implementation advertises and does not have. State the cost as *"~0.5 ms per sampled page, plus
document parse"*, and make the sample count a pinned profile field. If ethos-engine ever shells out
to a classifier, fork/exec alone consumes the entire budget: **in-process only**.

#### The disqualifying finding: the extractor fails open

**Observed fact.** The PDF `"` show-text operator is **absent from the operator match**. Its text
vanishes silently, and the surrounding runs are merged into a single item with corrupt geometry.
`Tz` (horizontal text scaling) does not appear anywhere in the source tree, so `.width` is wrong
whenever a document uses it.

Silent text loss with no diagnostic is the exact failure class Workbench rule 4 exists to prevent —
*"absence of extractable content is never evidence of absence in the source"* — and it cannot be
detected downstream, because the output looks well-formed. **This, more than the geometry
semantics, is why the answer is reference-only rather than wrap.** Any clean-room implementation
must enumerate the operator set explicitly and **fail closed on an unrecognised operator**.

#### The reading-order cliff, with its constant

**Observed fact.** Multi-column detection flips on a single line of text: `min_lines < 15` at
`layout.rs:1793`. Fourteen lines per column produces row-interleaved order; fifteen produces
column-major. Measured on generated fixtures. A one-line edit to a document reorders the whole
page — which is why v0 ships single-column plus an explicit limitation rather than inheriting this.

#### The confidence field is not merely uncalibrated — it contradicts itself

**Observed fact.** `fixtures/synthetic/simple-text/document.pdf`:

```
Type: TEXT-BASED (extractable text)
Confidence: 50%
Pages with text: 0            ← its own evidence disagrees with its verdict
OCR recommended: NO
```

Ethos extracts this document's text without difficulty. Meanwhile `two-columns` reports
`Pages with text: 1` → `Confidence: 100%`. So `confidence` tracks `pages_with_text / pages_sampled`
with a floor, and a document with **zero** text pages still classifies TEXT-BASED at 50%.

Two routing rules a reasonable engineer would write, both wrong on this corpus:

- `confidence ≥ 0.7 → trust text extraction` sends a plainly text-based PDF to OCR.
- `type == TEXT_BASED → skip OCR` skips OCR on a document the detector itself says has no text.

This is the measured, concrete case for Workbench rule 9, produced by the exact feature under
consideration. **Do not ship a public confidence float.**

#### A measured robustness gap, root-caused

**Observed fact.** `fixtures/synthetic/table-regular-grid/document.pdf` — a valid, CC0, Ethos-
authored fixture with committed extraction/layout/table goldens — **fails to open**:
`Error: PDF parsing error: couldn't parse input: invalid file trailer`.

Root cause, confirmed by byte inspection: its cross-reference entries are **19 bytes**
(`0000000015 00000 n\n`) where PDF 32000-1 §7.5.4 requires **exactly 20** (`…n \n`, note the
trailing space). Working fixtures have the 20-byte form. PDFium repairs it; lopdf rejects it.

That is one valid document in ~26 — a **~4% open-failure rate on Ethos's own conformance corpus**,
against a backend that handles it. The class is *strict-vs-repairing xref parsing*, and real-world
PDFs are full of it. It fails closed, which is correct, but it must be a declared limitation and it
argues against a lopdf-only v0 for any production path.

#### Feature verdicts for v0

| Capability | v0? | Why |
| --- | --- | --- |
| Text-layer classification | **yes — clean-room, genuinely bounded** | ~200 LOC. The existing semantics are contradictory *and* its sampling does not bound cost; inheriting either imports a bug |
| Position-aware text runs (origin + advance) | **yes — clean-room, operator set enumerated** | The CTM/text-matrix design is the reference to learn from, but the implementation drops `"` and ignores `Tz`. Fail closed on unrecognised operators |
| Per-page OCR routing signal | **yes, as counts** | Emit `pages_with_text` / `pages_sampled` per page, not a verdict + score. Note the upstream 0-vs-1-indexed inconsistency between `PdfProcessResult` and `PdfClassification` — pick 1-based and test it |
| Ink bbox from real font metrics | **yes, with typed absence** | See §16.5 |
| `mcid` (marked-content id) | **yes** | It is the honest bridge from a text run to the tagged-structure tree, and it is nearly free |
| Reading order | **v0: single-column + explicit limitation** | Upstream picks stream-order vs y-sort from a `chaos_ratio > 0.4` vote. A cliff-shaped heuristic cannot sit under a determinism contract; multi-column ships when it has a stable rule and a fixture |
| Tables | **no** | Rebuild later with the locator cross-check as its test |
| Markdown / HTML | **no** | Rule 8; see §16.1 |
| Underline/strikeout detection | **no** | Geometric inference presented as a font attribute |
| `confidence` float | **never** | Rule 9 |
| WASM / napi bindings | **no** | Later, driven by adopter demand |

#### Dependency posture: reference-only, plus vendored data tables

**Recommendation, split by layer:**

- **Object/xref layer → depend on `lopdf`** directly. It is the commodity part.
- **Encoding tables (168 Adobe `.bcmap` CMaps, glyph-name and Korea1 tables) → vendor**, MIT, with
  the Adobe BSD-3-Clause NOTICE reproduced. These are *data*, they do not churn, and regenerating
  them is pure cost.
- **Content-stream interpreter, classification, layout → clean-room**, using pdf-inspector as
  architectural reference.

Reason: the parts worth wrapping are exactly the parts we would have to un-wrap. `TextItem.height`
is literally the same variable as `font_size`; `y` is a baseline the JSON never labels; the
classifier's confidence contradicts its own counts. Depending on the crate means inheriting a type
whose fields mean something other than their names, in a project whose entire product is not doing
that. Add that upstream moved 0.1.8 → 1.14.x in months and MIT gives us the code anyway, and
reference-only is the cheaper path.

### 16.4 Greenfield decision — confirmed, with a precise reuse boundary

**Build `~/Desktop/Stuff/repo/ethos-engine/` as a fresh sibling.** Reuse from Ethos:

| Take | Do not take |
| --- | --- |
| **Contracts**: `ethos-grounding-source.schema.json` as the emit target; `ethos-grounding-validation-report.schema.json` as the validation oracle; the verification-config schema | `ethos-document.schema.json` as a model target — `DocumentRepresentation v0` is the shape to emit |
| **Fixtures** as a conformance corpus: `fixtures/synthetic/*` (9 cases, each with committed goldens), `fixtures/failure/*` (5 fail-closed cases), `fixtures/foreign/opendataloader/real/*` | `crates/ethos-tables` — whitespace-only, one table per page, spans hardcoded to 1, no table anchor element |
| **Designs**: c14n v1; quantize/`QRect` integer centipoints; ids-v1 ordering discipline; **profile-as-identity** (the best idea in the repo); the capability/fail-closed vocabulary; the `EthosPdfBackend` 3-method seam | The 16-line Markdown renderer; `ElementType` (10 declared, 3 produced) |
| **The CLI as an oracle**: `ethos grounding check` and `ethos verify` in CI | The crate tree, wholesale |

**MSRV:** `lopdf` 0.42 requires Rust ≥ 1.88; Ethos pins 1.87. ethos-engine is a separate workspace,
so it simply pins **1.88** and the conflict disappears — one of the concrete wins of the sibling
decision.

### 16.5 Coordinates in v0 — origins are identity, boxes are inspection

`DocumentRepresentation v0` settles this: **`NativeLocator` is required; `RenderedLocator`/geometry
is optional and "for inspection."** So:

1. **The required primitive is the native locator** — page + character origin (x, baseline y) +
   advance width, quantized to integer centipoints. This is also what pass 1 measured as the
   quantity two independent PDF stacks agree on to **0.001 pt**, while disagreeing on height by
   **6.174 pt**.
2. **Ship a real ink box in v0, but only from measured font metrics** — ascent/descent from the
   embedded font program via `ttf-parser`, falling back to the FontDescriptor's `/Ascent`,
   `/Descent`, `/FontBBox`. Roughly 250–400 LOC.
3. **Where metrics are unavailable, emit typed absence** (`NotReportedByReader`) and declare the
   capability limit. Never `height = font_size`. A font-size-derived box is closer to invented than
   measured, and rule 3 forbids inventing a coordinate.

Consequence to state plainly: without geometry on some nodes, **crops and highlight rendering
cannot be driven by ethos-engine v0** for those nodes. That is why PDFium/Ethos keeps the crop lane
for now.

**Adopt the spec's own fidelity idea:** where a node has both a geometric and a structural address,
derive them **independently** and cross-check — overlapping cell regions, a cell outside its parent
table, a grid that does not tile the table area. *"A single locator can only be trusted or not; a
pair can be tested."* Record the result as a typed diagnostic with a check version, **never
silently repaired**. This is a better fidelity mechanism than cross-parser divergence, and it needs
no second parser.

### 16.6 Where verify lives — staged

**Constraint:** `WORKBENCH_ARCHITECTURE.md` I.3 rule 3 — *"The Workbench never invokes Ethos
directly… This guarantees exactly one Ethos integration exists"* — and II.4's must-not-break,
*"Two independent Ethos integrations are never permitted."*

- **Stage 0 (v0): ethos-engine does not verify.** The happy path terminates at a *validated*
  grounding artifact: `classify → extract → grounding → grounding-check`. `grounding-check` is a
  reimplementation of the **JSON Schema validator only**, never the verifier, and it has a
  deterministic external oracle: `ethos grounding check <file> --source-artifact <pdf>`.
  **Exit criterion:** across all 15 fixtures, ethos-engine's validator and Ethos agree
  byte-identically on `structure`, `source_binding`, `representation_sha256` and `counts`. A CI
  job, not a claim.
- **Stage 1 (v0.1): shell out to the Ethos CLI**, as a declared capability. Absence is a named
  error, never a skip or a stub report. Pin `sha256(ethos)` and `ethos --version` into
  ethos-engine's profile so a verifier swap is fingerprint-visible. Relay report bytes verbatim;
  never re-derive `all_evidence_grounded`, `capability_limits` or `evidence_tier` from them —
  re-deriving is how a second authority is born by accident. This matches the established estate
  pattern: the parse API already spawns the ODL Java CLI and the Ethos Rust CLI once per
  invocation behind a byte-size admission gate.
- **Stage 2 (later): link `ethos-verify` as a crate**, for OSS adopters with no Ethos binary —
  gated on byte-identical conformance against Ethos's committed goldens, and **off by default in
  any DocuShell build** so "exactly one integration" survives.
- **Never: a second verify implementation.** Linking the same verifier is not a second integration.
  Reimplementing its semantics is.

At the design target — 20,000 docs/day ≈ 14/minute — a process spawn per document is negligible.
Do not optimise it away. *(Unmeasured: per-invocation `ethos verify` cost. Cheap spike.)*

### 16.7 OSS integration map

| Host / surface | Role | Ship in ethos-engine? | Use inside DocuShell? | Conditions / triggers |
| --- | --- | --- | --- | --- |
| **CLI + JSON** | baseline contract | **yes, first** | yes, via the parse API | Always. It is the pinnable, language-agnostic surface Ethos already proved |
| **Rust lib + Python/Node SDK** | embed | **yes, with the CLI** | yes, behind the parse API | CLI is a thin shell over the library so they cannot diverge |
| **MCP server** | agent/IDE surface | **yes — first adapter after CLI+SDK** | Yes, but it belongs to **the parse-API product, not the Workbench** — decision log, `WORKBENCH_ARCHITECTURE.md:326` | Locators are **only ever returned** by the engine and passed back as opaque handles it minted and re-validates. Never accept a locator as free-text model input |
| **LangChain tool (Py + JS)** | callable tool | **yes — second** | No; the *Agent steps* slot is occupied by LangGraph.js | Locators travel in the tool `artifact`, never in `content`. Must not set trust state |
| **LangGraph / LangGraph.js** | orchestration | **no separate adapter** — the LangChain tool runs inside it unchanged | Incumbent, trigger-gated | Replacing it needs a decision-log entry, never a deferral |
| **Docling** | **ingest** source | **yes — highest-value input adapter** | already in the estate via ODL → Docling over local HTTP | Gate on the *producing pipeline*: refuse or hard-flag VLM/dots/chandra output, which fabricates geometry |
| **LlamaIndex** | retrieval host | **yes — later** | Deferred; needs **all four** trigger conditions | Every locator key must be in `excluded_llm_metadata_keys`; return type must carry page + bbox + tableCell |
| **Haystack** | retrieval host | **yes — later** | Deferred, same trigger row | `Document.score` must stay `None` |
| **Unstructured** | ingest source | yes, after Docling | no trigger today | Preserve polygons or declare the polygon→bbox reduction as a limitation |
| **RAGFlow** | full platform | **no** (library extraction only, later) | **no** — rejected, retained as benchmark reference | See §16.8 |
| **Dify** | full platform | **no** | **no** — Apache-2.0 **with additional conditions**: prohibits multi-tenant operation without written authorisation and prohibits removing console branding. Fails licence-gate limbs 1 and 3 | Reachable via MCP; that is the whole answer |
| **n8n** | workflow host | **no** | **no** — Sustainable Use Licence restricts SaaS embedding and use as the engine of a paid product | Reachable via MCP |
| **Windmill** | workflow host | **no** | **no** — AGPLv3 fails limb 2 for bundling | Only as customer-supplied network-isolated infra |
| **Langflow** | demo surface | optional, low priority | no — redundant with the authoring canvas | MIT-clean, so cheap if a demo is ever wanted |

**MCP first, and it is not close.** It is the only host whose native return type carries a locator
under an *enforced* schema (`outputSchema` + `structuredContent`); every other host on the list is
effectively `Dict[str, Any]`. One MCP server reaches every other host here — including the
licence-failing ones — without dragging their licences toward us. And the July 2026 statelessness
change suits a deterministic verifier exactly: no session, no handshake, every call self-contained.

**Its one real hazard, stated plainly:** MCP tools are model-controlled — the model chooses the
arguments. If any tool accepts a locator as a free-text argument that the engine then trusts, the
model has become the citation authority in a single step. The mitigation is structural: **the
engine mints every locator, returns it as an opaque handle, and re-validates it on the way back
in.** Get that wrong and MCP is the worst option on the list rather than the best.

### 16.8 The DocuShell soft-revisit policy for RAGFlow-class platforms

Today's default stands: **no RAGFlow-class platform in the trust core.** RAGFlow's rejection was
explicitly *not* a licence issue — it is Apache-2.0 clean. The reason is structural: DeepDoc does
not model row/column relationships, so it cannot emit `TableCellPosition` with `rowspan`/`colspan`,
which means the geometric-versus-structural cross-check **cannot run**, and table-cell claims
degrade to region-only bindings on exactly the documents that matter.

The default is not dogma. Reversing it requires **all** of the following, written as triggers:

1. **The structural blocker is gone** — the candidate demonstrably emits row/column plus
   `rowspan`/`colspan` per cell, verified against its source on a labelled set, so the
   geometric-vs-structural cross-check can actually run.
2. **The return type carries `page`, `bbox` and `tableCell` without loss.** Per the Part II trigger
   row this disqualifies a framework *whether or not the trigger has fired* — the only alternatives
   are a null locator, which fails closed and makes the framework useless, or an invented one,
   which violates rule 3 outright.
3. **All four retrieval-framework trigger conditions fire**: >2,000 docs/day for one tenant or
   >100k records in one query scope; a technique not implementable against `BoundEvidenceRecord` in
   one engineer-week; a measured, reproduced recall deficit on a labelled set; and a dedicated
   engineer.
4. **It sits behind a DocuShell interface returning `BoundEvidenceRecord`**, in a separately owned
   service that never holds the evidence contract.
5. **It never becomes a citation or verification authority.** Ethos is unaffected either way — *it
   never receives a query and never ranks*.
6. **Licence gate still passes** all three limbs at adoption time.
7. **A decision-log entry** reverses the Appendix C row.

Absent all seven, the answer is no — and the useful posture in the meantime is the one Appendix C
already names: keep it as a **benchmark reference for retrieval recall**, and take techniques
rather than platforms.

### 16.9 Corrections to earlier recommendations

| Earlier | Correction |
| --- | --- |
| Plan §3.1: extend Ethos's canonical model | Superseded — greenfield engine emitting `DocumentRepresentation v0` |
| Plan §4.4: Markdown Anchor Map as a headline deliverable | **Deferred out of v0.** Still the right design *if* a Markdown projection is ever needed; rule 8 says prefer no projection at all |
| Plan §8.3: cross-parser agreement as the fidelity mechanism | Demoted. The **intra-representation locator cross-check** is cheaper, deterministic, and needs no second parser |
| Pass-1 §14.1: "wrap pdf-inspector, then fork" | Corrected to **reference-only + vendor the data tables**. The measured confidence contradiction and the `height`/`y` semantics make the wrap step negative-value |
| Pass-1 OCR: PP-OCR is *the* pick | Still the pick for a **deterministic** OCR lane. But DocuShell has separately named **GLM-OCR (MIT, 0.9B)** as its Part II *accuracy-lane* candidate, pending approval. A VLM-class reader engages **rule 7** — if it produces the representation, it must not also draft the claims |
| Pass-1: assist lane enforced by byte-diff CI | Keep it, and **add a cheaper precondition**: give every lane a declared processor identity in `ProcessingRun`, so a run whose drafting model and representation processor are the same identity is **rejected mechanically** rather than caught in review |

### 16.10 ethos-engine v0 scope

**In:** PDF only · text-layer classification emitting **counts, not confidence** · position-aware
text runs (origin + advance + font id/size + page + `mcid`) · ink bbox from measured font metrics
with typed absence · single-column reading order with an explicit multi-column limitation ·
canonical representation with c14n + integer quanta + profile-pinned ids · capability/limitation
declarations (the L1 gate) · `ethos.grounding.v1` emission · `grounding-check` validator with the
Ethos CLI as oracle · CLI + Rust library.

**Out of v0:** tables · Markdown/HTML · OCR · assist/VLM · office formats · a second PDF backend ·
crops · MCP server · SDKs · verification.

**The happy path, and nothing else:** `classify → extract → grounding → grounding-check`.

### 16.11 Risks of starting to code now

| # | Risk | Mitigation before or during bootstrap |
| --- | --- | --- |
| 1 | **`DocumentRepresentation v0` is a target, not a shipped type.** Building against a spec nobody has implemented risks divergence from what DocuShell eventually needs | Emit it and validate against the companion document's field list; treat the first implementation as the reference and expect a review round |
| 2 | **lopdf's ~4% open-failure rate** on a corpus PDFium handles | Declare it; keep PDFium/Ethos available for the documents it rejects; measure the rate on a real corpus, not 26 fixtures |
| 3 | **Multi-column reading order has no stable rule yet** | v0 ships single-column plus an explicit limitation rather than a cliff-shaped heuristic |
| 4 | Ink-box work is the only unbounded item in v0 | Time-box it; typed absence is an acceptable v0 answer for hard fonts |
| 5 | MCP locator-handle discipline is easy to get wrong | It is an API-shape decision — settle it before the first tool exists, per II.7.1's reasoning |
| 6 | Two engines could drift into two answers | Stage 0 forbids verification entirely; the oracle test is the guard |

**Unmeasured, and worth an hour each:** per-invocation `ethos verify` cost; lopdf open-failure rate
on a real (non-fixture) corpus; ink-box agreement with PDFium on the ADR-0009 five.

### 16.12 Bootstrap steps for `repo/ethos-engine/`

Recorded for when the decider says go. **Not executed in this pass.**

```bash
cd ~/Desktop/Stuff/repo && mkdir ethos-engine && cd ethos-engine
git init && rustup toolchain install 1.88.0
printf '[toolchain]\nchannel = "1.88.0"\n' > rust-toolchain.toml
# Apache-2.0, matching Ethos; NOTICE reserved for the vendored Adobe CMaps
```

Initial workspace — six crates, deliberately small:

```
ethos-engine/
├── Cargo.toml            # workspace, MSRV 1.88, resolver 2
├── deny.toml             # copied discipline: permissive-only, no network crates
├── crates/
│   ├── engine-core/      # representation types, c14n, quanta, ids, capabilities
│   ├── engine-pdf/       # lopdf + vendored CMaps: classify + text runs + font metrics
│   ├── engine-grounding/ # representation → ethos.grounding.v1 + the validator
│   └── engine-cli/       # classify | extract | ground | grounding-check
├── vendor/cmaps/         # 168 Adobe .bcmap + NOTICE
├── fixtures/             # symlink/copy manifest of the Ethos conformance corpus
└── tests/oracle.rs       # byte-identical agreement with `ethos grounding check`
```

First commit should contain the oracle test and one fixture, failing. The second should make it
pass on `simple-text`. Nothing else belongs in week one.

### 16.13 Go / no-go

**Go**, with one caveat that is cheap to close: `DocumentRepresentation v0` is specified but not
implemented anywhere, so ethos-engine will be its first implementation. That is an opportunity as
much as a risk — but it should be an explicit, acknowledged decision rather than something
discovered in review, and the field list in the companion document should be re-read against the
first schema commit.

---

## 17. Combined OSS parity checklist (ODL + Anydoc + pdf-inspector)

> **The full checklist — ~70 rows with README claim, memo evidence, decision, target version, exit
> criterion and impact for every capability — lives in
> `ethos-engine/docs/reference/ethos-engine-parity-checklist.md`.** This section carries the verdict, the
> summary tables, the version map, the performance bar and the corrections. The two are one
> document; the split exists only because the full tables are too wide to read comfortably here.

### 17.1 Executive verdict — where ethos-engine stands tall

The three sources are individually strong and collectively leave a gap that is structural rather
than one of effort. **OpenDataLoader** has the best *table model* in open source — authoritative
row/column plus `rowspan`/`colspan`, per-cell boxes, `is header`, multi-page linking — and its own
README concedes it cannot process Word, Excel or PowerPoint at all. **Anydoc** covers fourteen
formats through one document model and one serializer, and carries no page, no bbox and no element
id anywhere in that model, by design. **pdf-inspector** has a competent text-matrix implementation
and publishes a classifier whose advertised sampling does not bound its cost, whose confidence
contradicts its own counts, and whose extractor silently drops the `"` show-text operator. The
number worth more than any headline is one both ODL and pdf-inspector independently report:
**ODL's deterministic local mode scores 0.489 on tables** — the 0.907 that leads its README is the
*hybrid AI-backed* mode. So the bar for replacing ODL deterministically is far lower than its
marketing implies, and the bar for matching its hybrid is a different, non-deterministic product.
ethos-engine wins by taking ODL's table *model*, pdf-inspector's *rectangle detection* and Anydoc's
*shared-IR discipline*, then adding the three things none of them has: a locator on every node
across every format, a geometric↔structural cross-check that makes a wrong cell *detectable*, and
citation verification. It also wins by refusing things they ship — a confidence float, silent
deletion of hidden text and footers, `--sanitize` rewriting the evidence, and any "#1" claim, given
that all three headline numbers are publisher-owned. **Standing tall is not winning a bake-off; it
is being the only one whose output can be checked.**

### 17.2 Summary

**TAKE — 22 items, build essentially as the source does.** Semantic element types · XY-Cut reading
order · multi-page table linking · annotated-PDF debugging · images with coordinates ·
**tagged-PDF consumption + `mcid` bridge** · per-page OCR routing · the Word/Excel/PPT gap ODL
declares · 14-format coverage · **shared IR → one serializer** · **`CellSlot` merged-cell model** ·
content-based format detection · **the six-variant error taxonomy** · embedded assets ·
**mutation-testing every fixture + cargo-fuzz per format** · vendored CMap tables ·
**encoding-issue detection** · single document load shared by classify and extract ·
**dual-mode table detection (rectangles + alignment)** · hyphenation rejoin.

**IMPROVE — 16 items, right goal, wrong mechanism.** Determinism as a *contract* not a mode name ·
self-describing JSON (declared coordinate system, real page geometry) · the table *detector* (the
model is fine) · Markdown **only with the Anchor Map** · **hidden text: report, never delete** ·
**header/footer: classify, never drop** · SDKs without a JVM · declared erasure ·
**bounded classification** · counts instead of verdicts · **honest geometry** ·
**fail closed on unknown operators** · stable reading order · declared heading basis ·
page numbers classified not deleted · xref repair-or-refuse.

**REFUSE — 9 items.** PDF/UA export · accessibility studio · chart descriptions *as evidence* ·
`--sanitize` · "#1"/"fastest" claims · a JVM runtime dependency · Anydoc's PDF-through-Markdown
bridge · **a public confidence float** · style and role inferred from font names or text prefixes.

**DEFER — 8 items.** Complex-table hybrid (v3) · HTML (v1.1) · **auto-tagging (v2.2+, parallel
lane)** · formula LaTeX (v3) · WASM (v2+) · agent skill (v1.2) · RTL (v1).

**Ethos-native, keep explicit:** citation verification · capability declarations and fail-closed ·
profile-as-identity · c14n and integer quanta · `DocumentRepresentation v0` + `ethos.grounding.v1` ·
the **intra-representation geometric↔structural cross-check** · derivation honesty
(`Extracted`/`Computed`/`Recognized`/`Proposed`) · the BYO-parser path, forever.

### 17.3 Benchmark provenance — read before believing any number

| Publisher | Claim | Problem |
| --- | --- | --- |
| ODL | "#1 in benchmarks (0.907 overall)" | Own corpus; the 0.907 is the **hybrid AI mode**. Deterministic free mode is **0.831 overall / 0.489 table** |
| pdf-inspector | "0.875 overall, best table + reading order" | Competitor's corpus, results on a **fork branch**, **pdf-inspector 0.2.6 vs ODL 2.2.1** — both far behind today's 1.14.1 / 2.5.x |
| Anydoc | "score 81, 14/14 formats, 4.4 ms median" | **LLM-as-judge** (482 verdicts) against LibreOffice renders, on a corpus that is **"not redistributable and is not in the repo"**; process spawn excluded |

The one figure two independent publishers agree on is ODL-local's **0.489 table** score. Ethos's
`docs/landscape-log.md` already requires publisher-owned suites to be labelled; all three are.

### 17.4 Version map

| Ver | Theme | Gate |
| --- | --- | --- |
| **v0** | Honest PDF core — classify (counts, bounded), position-aware runs, measured ink box or typed absence, single-column order, format detection, error taxonomy, capability declarations, `ethos.grounding.v1`, `grounding-check`, CLI + lib, fuzz + mutation tests | Validator agrees byte-identically with `ethos grounding check` on all 15 fixtures |
| **v0.1** | Verify (shell out) · encoding-issue detection · xref repair-or-refuse | Ungrounded exits 1 with a report; no silent skip |
| **v1** | **DocuShell replacement gate** — tables with locator cross-check, full element vocabulary, multi-column with a stable rule, tagged-PDF consumption + `mcid`, security findings, images, annotated PDF | Table-cell accuracy **> 0.489** on a labelled set; fabrication rate 0 |
| **v1.1** | Safe Markdown + **Anchor Map** · HTML · export-only cosmetics | A Markdown-quoted citation verifies end-to-end |
| **v1.2** | **MCP server** · Python + Node SDKs · LangChain tool | Locators survive every adapter round-trip |
| **v2** | Anydoc-class formats: DOCX → XLSX → PPTX → ODF/RTF/EPUB/CSV, shared IR, embedded assets | A DOCX quote and an XLSX cell both ground; no synthesised pages |
| **v2.1** | OCR lane, own profile, per-page routing | OCR fingerprint provably incomparable with born-digital |
| **v2.2** | Accessibility — auto-tag → Tagged PDF (**parallel lane, not on the critical path**) | Only on a named accessibility requirement |
| **v3** | Assist: propose-only VLM, dual-read → review, hybrid enrichments as `Recognized`/`Proposed` | Byte-diff: assist on/off ⇒ identical grounded artifacts |

### 17.5 Performance bar

| Axis | Target | Method |
| --- | --- | --- |
| Classification | ≤ **0.5 ms per sampled page** in-process, N = 8 default; **cost must not scale with total page count** | Best-of-5 in-process on the Ethos frozen corpus |
| Classification bound test | 500-page PDF at N=8 within **20%** of an 8-page PDF at similar bytes/page | The A/B that would have caught `detector.rs:431-447` |
| Born-digital extract | **≥ 134 pages/sec p50** | From Ethos's own G1 rule `≥ max(120 pps, 2× remeasured ODL)`; ODL-local is 0.015 s/page ≈ 67 pps by its own README. Harness: `run_gate_zero.py` `measure_command()` |
| Office convert | median **< 10 ms/doc** | On a **named, redistributable** corpus — the thing Anydoc's 4.4 ms cannot offer |
| Verify path | spawn acknowledged, not optimised | ~19–22 ms measured spawn floor; at 20,000 docs/day ≈ 14/min it is irrelevant |
| Public claims | none | No "fastest"/"#1"/"best" without a gated harness and a named corpus |

### 17.6 Forced decisions — reaffirmed

| # | Question | Answer |
| --- | --- | --- |
| 1 | pdf-inspector: reference-only + vendor CMaps + lopdf? | **Yes, and more firmly.** The `"`-operator silent drop (P6) is a stronger reason than the geometry semantics were |
| 2 | No public confidence field? | **Yes.** Rule 9, plus a measured self-contradiction: TEXT-BASED at 50% with zero text pages |
| 3 | Native locator required; ink box measured or typed-absent? | **Yes**, and `DocumentRepresentation v0` independently agrees: `NativeLocator` required, geometry optional "for inspection" |
| 4 | Markdown out of v0; Anchor Map when it ships? | **Yes.** Rule 8 |
| 5 | Tables as the v1 hard gate for the ODL replacement? | **Yes, with the bar corrected to 0.489** — ODL-local, not ODL-hybrid |
| 6 | Auto-tag / accessibility version? | **v2.2, parallel lane.** Tag *consumption* is v1 because it improves grounding; tag *generation* is a different product for a different buyer |
| 7 | Hybrid enrichments (formula, chart captions)? | **v3.** Formula LaTeX is `Recognized`; a chart description is `Proposed` and never citable |
| 8 | First OSS adapter after CLI + lib still MCP? | **Yes** — only host whose return type carries locators under an enforced schema |
| 9 | Greenfield `ethos-engine/`? | **Yes** |

### 17.7 Corrections this pass makes

| Where | Was | Now |
| --- | --- | --- |
| §16.11 risk #1 | "Table quality is a regression against ODL" | **Partly wrong.** ODL-*local* scores 0.489 on tables; the strong number is its hybrid AI mode. The v1 gate is "beat 0.489 deterministically", which is achievable — and where hybrid was better, say so rather than match it |
| §16.3 | pdf-inspector's classifier is slow on large files | Also **unbounded**: `Sample(8)` is defeated by a Phase-3 full rescan, so `Pages(1)` costs the same as `Full`. Ours must be genuinely bounded |
| §5 / §3.3 | ODL "filters" hidden text and headers/footers | The README says *filters*; the Java **deletes**. Take the threat model, invert the mechanism: report, never delete |
| Plan §4.4 | Markdown Anchor Map as a headline v1 deliverable | v1.1, and only alongside Markdown. Rule 8 prefers no projection at all |
| Plan §12.1 | "WRAP pdf-inspector, then fork" | Superseded by §16 and reaffirmed here: **reference-only** |
| Pass-1 §13.2 | Moat dilution was risk #1 | Still real, but the operational risk #1 is now **shipping formats faster than table quality** |

---

## 18. LiteParse deep analysis + four-way parity (pass 4)

LiteParse (run-llama, **Apache-2.0, no addendum**) is the one competitor Ethos's own PRD names as
the closest overlap. `docs/ethos-product-requirements.md` §2.1, verbatim:

> "LiteParse is the closest overlap for Ethos' Release 1 parser-core surface… **Ethos must
> therefore not position itself as merely a fast local parser with bounding boxes. That would be a
> weak and already occupied lane.**"
>
> Explicit non-moats: bounding boxes alone · Markdown export alone · hidden-text filtering alone ·
> local execution alone · *"deterministic" marketing language without a versioned contract*.

It is already pinned in `benchmarks/competitors.lock.json` at **2.0.8** as *"context (non-gating);
closest Release-1 overlap, watch per risk R5"*. Two corrections to that record are below (§18.7).

**Headline, and it is uncomfortable:** LiteParse is a **materially more honest codebase than
pdf-inspector**. There is no `height == font_size` conflation, no lookup-table confidence, no
missing content-stream operator. It has taken the "fast local parser with bounding boxes" lane and
taken it well. Its weakness is a different class — **undeclared silent deletion and one undeclared
document mutation** — and, decisively for Ethos, **no versioned output contract at all**. That is
the lane to attack, and the PRD predicted it.

### 18.1 What LiteParse is genuinely good at

| Capability | README claim | Observed in source / measured | Decision | Why | Ver | Exit criterion |
| --- | --- | --- | --- | --- | --- | --- |
| **Complexity reason codes** | "Cheaply check whether a document needs OCR" | `[F]` `ocr_merge.rs:53-81` — `ComplexityReason::{Scanned, NoText, SparseText, EmbeddedImages, Garbled, VectorText, AnnotationText}` | **TAKE** | A reason vocabulary a caller can write policy against. Strictly better than a confidence float | **v0** | Every routing decision traces to a named reason, never a score |
| **Two orthogonal verdict axes** | not claimed | `[F]` `ocr_merge.rs:278-286` — `LayoutComplexityReason::{MultiColumn, TableLikely, DenseGraphics}`, doc'd *"Orthogonal to `ComplexityReason`: none of these imply OCR"* | **TAKE** | Separating *needs OCR* from *layout is hard* is the correct decomposition and nobody else has it | **v0** | `table-likely` never triggers an OCR route |
| **Boolean derived from reasons** | — | `[F]` `ocr_merge.rs:235` — `needs_ocr = !reasons.is_empty()` | **TAKE** | The list is the truth; the boolean is a convenience | v0 | Removing the boolean loses no information |
| **No confidence in the complexity output** | — | `[F]` grepped across Rust, napi, WASM, Python, TS — none | **TAKE** | Workbench **rule 9** satisfied by construction. pdf-inspector fails this | v0 | No confidence field in any classify artifact |
| **Per-page detail, 1-indexed** | "list the pages that need OCR" | `[F]` `PageComplexityStats` (`ocr_merge.rs:97-142`), 1-based per `parser.rs:342` | **TAKE** | Per-page routing without an all-or-nothing verdict | v0 | A page-index round-trip test pins the base |
| **`trailing_space_generated`** | not claimed | `[F]` `types.rs:110-112` — *"Whether the trailing source space was synthesized by PDFium rather than represented by a real space glyph"* | **TAKE** | **The best honesty field in any of the four projects.** It marks synthesized content explicitly | **v0** | Every synthesized character is flagged at emission |
| **`char_codes` + ligature caveat** | — | `[F]` `types.rs:103-108` — glyph codes, with a note that ligature expansion yields more scalars than codes | **TAKE** | Glyph-vs-scalar mismatch handled honestly rather than papered over | v0 | Ligature fixture round-trips with the mismatch declared |
| **OCR/native discriminator** | — | `[F]` `types.rs:113-115` — `confidence: Option<f32>`, *"None for native PDF text"* | **IMPROVE** | The idea is right; a nullable float is the wrong carrier. Ours is a typed `DerivationClass` | v0 | OCR-derived text is distinguishable by type, not by field absence |
| **Open HTTP OCR API spec** | "Standard API" | `[F]` `OCR_API_SPEC.md` — `POST /ocr`, multipart `file`+`language`, `{results:[{text,bbox,confidence,polygon?}]}`; polygon *"in the glyphs' upright reading frame"* so rotation is recoverable | **TAKE** | A minimal, engine-agnostic contract. Adopt it nearly verbatim, drop the confidence gate | **v2.1** | Any conforming server works without an ethos-engine change |
| **Structure-tree extraction** | "tagged-PDF logical structure in JSON" | `[F]` real; bridges tags to text runs **partially** | **TAKE, with care** | Author-declared structure is the best signal available | v1 | Tagged roles attach to the right runs; untagged declares the limitation |
| **Vector path data** | "page-scoped vector path data" | `[F]` real, with `filled_path_bounds(3.0, 0.9)` | **TAKE** | Ruling-line geometry is exactly what a real table detector needs and Ethos has none | v1 | Ruled-table fixtures detected from path data |
| **Forms + annotations as first-class** | AcroForm widgets, page annotations | `[F]` real | **TAKE (extraction), REFUSE (silent repair)** | Annotation text must be *distinguishable* from page content | v1 | Annotation-sourced text is typed as such |
| **Screenshots / DPI rasters** | "for LLM agents" | `[F]` real, DPI-controlled | **TAKE** | Feeds crops and debug overlays | v1 | Deterministic raster at a pinned DPI |
| **`is-complex` as a shell predicate** | `lit is-complex doc.pdf --quiet && lit parse …` | `[F]` per-page JSON on stdout, verdict on stderr, non-zero exit when any page needs OCR | **IMPROVE** | Good ergonomics, broken semantics — see §18.2 | v0 | Three distinct outcomes get three distinct exit codes |
| **Bindings breadth** | Rust/Node/Python/WASM/CLI | `[F]` all real, published | **TAKE** | The adoption bar for OSS | v1.2 | Parity with Anydoc's binding discipline |

### 18.2 What not to copy

| # | Item | Evidence | Rule broken |
| --- | --- | --- | --- |
| 1 | **Competing on "fast PDFium + boxes"** | PRD §2.1 | The lane is occupied, and LiteParse occupies it well |
| 2 | **No versioned output contract** | `[F]` `output/json.rs:46-65` emits `page,width,height,text,text_items` — grep for `schema_version\|parser_version` across `output/` and `main.rs` returns **nothing** | This is Ethos's declared differentiator, and it is real |
| 3 | **Coordinate space undeclared** | `[F]` viewport, top-left, 72 DPI, CropBox→MediaBox — documented **only** in Rust doc comments and TS JSDoc. A consumer reading raw JSON must infer it | Rule 3-adjacent: an artifact that cannot be interpreted without its source |
| 4 | **Floats on the wire** | `[F]` `f32` throughout (`types.rs:65-68`), serde_json decimal, lossy round-trip, no fixed precision | Ethos c14n: *"floats do not exist in canonical Ethos"* |
| 5 | **Loose char boxes sold as "precise positioning"** | `[F]` bbox is a union of `FPDFText_GetLooseCharBox` — em boxes, ascent-to-descent, **not ink**. *"For a line of 'acme' the box is as tall as if it contained 'Ãj'."* Nothing in the output says which it is | Right for line grouping, wrong for a citation highlight — and undeclared |
| 6 | **`FPDFText_GetCharOrigin` is not bound at all** | `[F]` zero call sites | The fingerprint-critical primitive in Ethos's `origin-run-locator-v1`. **ethos-engine cannot source origins from this design** |
| 7 | **Confidence used as a filter, twice, silently** | `[F]` `ocr/tesseract.rs:131` — `if conf > 0.3 && !text.trim().is_empty()`; `ocr_merge.rs:632` — `if r.confidence <= 0.1 { continue }` | **Rules 9 and 4.** Text below a threshold is dropped with no record |
| 8 | **OCR merged into the native text stream** | `[F]` their own architecture diagram: *"OCR Merge — Native text + OCR results"*; `ocr_merge.rs` | Discriminated only by an omittable nullable field |
| 9 | **OCR on by default** | `[F]` `config.rs:209` — `ocr_enabled: cfg!(feature = "tesseract")`; README: *"OCR enabled by default"* | OCR must be opt-in and profile-isolated |
| 10 | **Tesseract as the bundled default** | `[F]` bundled; prior memo §3.5: documented cross-platform irreproducibility | Determinism |
| 11 | **`keep_headers_footers: false` by default** | `[F]` `config.rs:226` | Same silent-deletion class as ODL's filters. Classify, don't drop |
| 12 | **`preserve_small_text` opt-in** | `[F]` `main.rs:88` | Small text dropped by default |
| 13 | **Undeclared AcroForm "repair" and always-on widget flattening** | `[F]` README: *"repairs orphaned widgets in memory"* — an undeclared document mutation | Rule 3. A repair that is not recorded is a fabrication |
| 14 | **Markdown with no locator, structure from font names** | `[F]` `markdown_layout/` ~11k lines; font-NAME inference confirmed | Rule 8, and the same refusal as pdf-inspector P14 |
| 15 | **LibreOffice → PDF for office formats** | `[F]` README "Multi-Format Input Support"; external binary, user-installed | **It invents pagination.** `bring-your-own-parser.md`: *"Pagination does not exist in the file and must not be synthesised"* |
| 16 | **LlamaParse cloud upsell as the answer to hard documents** | `[F]` README top: *"For complex documents (dense tables, multi-column layouts, charts, handwritten text, or scanned PDFs), you'll get significantly better results with LlamaParse"* | Not a rule break — a strategic tell. **The vendor names its own weak spots**, and they are exactly DocuShell's documents |
| 17 | **Build-time PDFium download, unpinned** | `[F]` `crates/pdfium-sys/build.rs:85-163` — resolves env → `vendor/pdfium/` (**absent**) → auto-download from a **run-llama fork** of `pdfium-binaries`, **by tag, with no checksum verification** | Supply chain. Ethos's caller-provided policy + pinned sha256 is stronger |

### 18.3 The measurement — and it cuts both ways

Built from source (`cargo +1.88.0 build --release --bin lit -p liteparse --no-default-features`,
5m45s, 19.6 MB binary + 6.86 MB `libpdfium.dylib`). Process-spawn floor **22.2 ms**, matching the
pdf-inspector figure in §16.3. Best of 5.

| File | Pages | Verdict | OCR pages | Reasons | Layout | Exit | ms |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `synthetic/simple-text` | 1 | **COMPLEX** | 1/1 | `no-text` | — | 1 | 108 |
| `synthetic/two-columns` | 1 | **COMPLEX** | 1/1 | `sparse-text` | — | 1 | 104 |
| `synthetic/table-regular-grid` | 1 | **COMPLEX** | 1/1 | `sparse-text` | — | 1 | 106 |
| `failure/image-only-or-blank-page` | 1 | COMPLEX | 1/1 | `no-text` | — | 1 | 36 |
| `failure/password-protected` | — | **ERROR** `Pdf(PasswordRequired)` | — | — | — | **1** | 38 |
| `irs-form-1040-2025` | 2 | **SIMPLE** | 0/2 | — | `table-likely` | **0** | 146 |
| `nist-sp-800-63b` | 80 | COMPLEX | 38/80 | `embedded-images`, `sparse-text`, `vector-text` | multi-column, table-likely, dense-graphics | 1 | 821 |
| `nist-sp-800-53r5` | **492** | COMPLEX | 492/492 | `embedded-images`, `sparse-text` | multi-column, table-likely, dense-graphics | 1 | **8241** |

**Three findings.**

1. **Both classifiers fail on Ethos's synthetic corpus, in opposite directions.** pdf-inspector
   calls `simple-text` TEXT-BASED while reporting `Pages with text: 0` (under-routes). LiteParse
   calls it `no-text` and demands OCR (over-routes). Neither is wrong about the document; both
   thresholds are calibrated for real-world pages and neither is safe on a 20-character fixture.
   **The lesson is not "pick the better classifier" — it is that the classifier must emit counts
   and reasons and let the caller own the policy.**
2. **LiteParse's classifier is ~20× slower than pdf-inspector's on the 492-page document**
   (8,241 ms vs 412 ms) because it does far more — vector paths, XY-cut column counting, table
   detection, image coverage. It is not "cheap" in pdf-inspector's sense; it is *richer*. Neither
   is bounded.
3. **Exit code 1 is overloaded.** Measured: `password-protected`, `invalid-header`,
   `corrupt-header-valid` and a missing file **all exit 1**, identically to "this document is
   complex." The README's own predicate — `lit is-complex doc.pdf --quiet && lit parse doc.pdf
   --no-ocr` — cannot distinguish *"complex"* from *"I could not open this."* It fails closed, but
   with an **indistinguishable** signal. ethos-engine must give three outcomes three exit codes.

**A latent bug worth recording** `[I]`: two garble heuristics compound. Item-level
`is_likely_garbled` (`ocr_merge.rs:816-822`, 10% vowel floor) strips items from `text_length`,
which can then drive `text_length < 20` → **`no-text` on a page whose text extracted perfectly**.
Acronym-dense pages — a NIST control table of `AC-2`/`SC-7`, a parts list of `SKU`/`QTY`/`MFG` —
are the shape that triggers it. *Unknown, needs a spike: one generated fixture, ~30 min.*

### 18.4 The benchmark forensics — the most useful finding in this pass

ODL and pdf-inspector both publish results on the **same corpus** (`opendataloader-bench`) and
report **wildly different LiteParse numbers**:

| Engine | ODL overall | PI overall | ODL table | PI table | ODL heading | PI heading | ODL speed | PI speed |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| opendataloader (local) | 0.831 | 0.831 | 0.489 | 0.489 | 0.739 | 0.739 | 0.015 s/pg | 2.569 s/200 |
| markitdown | 0.589 | 0.589 | 0.273 | 0.273 | 0.000 | 0.000 | 0.114 | 16.165 s |
| **liteparse** | **0.576** | **0.873** | **0.000** | **0.693** | **0.000** | **0.811** | **1.061 s/pg** | **0.750 s/200** |

**The evaluator and corpus are not the variable** `[F]`: ODL's own row and markitdown's row are
*bit-identical* across both tables; pymupdf4llm differs by ≤0.012. Three of four shared rows
reproduce to three decimals. Only LiteParse's row moves — and by 34 points.

Two causes, both diagnosable `[I]`:

1. **ODL ran LiteParse in its default `text` output format.** The signature is exact — table
   `0.000` *and* heading `0.000` while reading order holds at 0.866. That is not a bad parser; it
   is a parser emitting no markdown structure. Markdown is **never** a default at any LiteParse
   surface: CLI `default_value = "text"` (`main.rs:50`), Rust `OutputFormat::Json`
   (`config.rs:216`), Python default `"json"`. LiteParse's *own* eval provider has to pass
   `output_format="markdown"` explicitly. A benchmark author following the README's first example
   gets plain text.
2. **ODL ran it with OCR on; pdf-inspector explicitly disabled OCR.** LiteParse's OCR is on by
   default (`config.rs:209`). Normalised, every other engine's speed agrees across the two tables
   within 1.4× — **LiteParse is off by ≈283×**.

**Why this matters more than LiteParse.** A competitor benchmarked a rival using the rival's own
default flags and published a number 34 points low — not through malice, just by running the first
command in the README. **This is the single strongest argument in the whole corpus for Ethos's
benchmark discipline**: `competitors.lock.json` pinning the exact artifact *and invocation*, and
the rule that *"competitor crashes/timeouts are recorded as data, never patched around."* Every
cross-vendor table in this landscape should be read as configuration-dependent until proven
otherwise. **Ethos should publish none of them, and should cite this discrepancy when explaining
why.**

### 18.5 Where LiteParse sits against the other three

| Need | ODL | Anydoc | pdf-inspector | LiteParse | Winner to learn from |
| --- | --- | --- | --- | --- | --- |
| Table **model** | **best** (row/col + spans + per-cell bbox + `is header`) | good (`CellSlot`) | weak | weak (0.000–0.693, config-dependent) | **ODL** |
| Table **detection** | 0.489 local | n/a | 0.814 claimed | vendor names tables as its weak spot | **pdf-inspector ideas + own detector** |
| Office native IR | **cannot** (declared) | **best** (14 formats, one IR) | n/a | LibreOffice→PDF (invents pagination) | **Anydoc** |
| Classify + routing | hybrid triage | none | confidence float ✗ | **reason codes ✓, two axes ✓** | **LiteParse** |
| Geometry honesty | bottom-left, undeclared origin | honest absence | `height`==`font_size` ✗ | loose boxes, undeclared space, floats | **none — ethos-engine wins here** |
| Character origins | no | no | per-run only | **not bound at all** | **none — Ethos native only** |
| OCR plug shape | hybrid server | none | none | **open HTTP spec ✓** | **LiteParse** |
| Screenshots / crops | annotated PDF | none | none | **DPI rasters ✓** | **LiteParse** |
| Tagged PDF + mcid | **best** (`use_struct_tree`) | n/a | `mcid` on items | structure tree ✓ | **ODL + pdf-inspector** |
| Forms / annotations | no | no | no | **only one** ✓ | **LiteParse** |
| Vector graphics | veraPDF borders | n/a | `detect_lines` | **path data exposed** ✓ | **LiteParse + pdf-inspector** |
| Safety filters | deletes ✗ | n/a | n/a | deletes ✗ | **none — ethos-engine wins here** |
| Runtime weight | **JVM** | **none** ✓ | none ✓ | PDFium sidecar + optional LibreOffice/Tesseract | **Anydoc** |
| Versioned output contract | no | no | no | **no** | **none — this is the moat** |
| Verification | no | no | no | no | **none — this is the product** |
| Adoption SDKs | Py/Node/Java + LangChain | Rust/Node/Py/WASM + skill | Py/Node/WASM | **Rust/Node/Py/WASM/CLI + agent skill** | **LiteParse / Anydoc** |

**What LiteParse uniquely contributes, in one line:** the **reason-code classifier with two
orthogonal axes**, the **open HTTP OCR contract**, **forms/annotations/vector-path extraction**,
**DPI screenshots**, and the `trailing_space_generated` honesty field — none of which the other
three have.

### 18.6 The four-way steal formula

> ethos-engine should take **the table model, tagged-PDF consumption, and XY-Cut reading order**
> from **OpenDataLoader**; **the shared multi-format IR, the `CellSlot` merged-cell model, the
> six-variant error taxonomy, content-based format detection, and mutation-plus-fuzz testing** from
> **Anydoc**; **rectangle-based table detection, encoding-issue detection, single-document-load, and
> the `mcid` bridge** from **pdf-inspector**; **the reason-code classifier with two orthogonal axes,
> the open HTTP OCR spec, forms/annotations/vector extraction, DPI screenshots, and
> `trailing_space_generated`** from **LiteParse** — and then add the things none of them has:
> **a versioned output contract with a fingerprint, declared coordinate systems, integer quanta,
> typed absence, derivation classes, an intra-representation geometric↔structural locator
> cross-check, and citation verification.**

### 18.7 Corrections to prior passes

| Where | Was | Now |
| --- | --- | --- |
| §17 four-way scope | Three sources | **Four.** LiteParse is the closest competitor and was missing from the parity work |
| §16.3 / §17 classify | pdf-inspector's confidence float is the shape to avoid | Still true — and **LiteParse shows the shape to adopt**: reason codes on two orthogonal axes, boolean derived |
| §17 "ODL-local 0.489 is the v1 bar" | Holds | **Holds**, and is now triple-sourced: ODL's own table, pdf-inspector's table, both bit-identical |
| §17 pdf-inspector "reference-only" | Holds | **Holds and strengthens.** LiteParse is the more honest codebase; if anything is ever wrapped it is not pdf-inspector |
| §17 Anydoc as the office-IR source | Holds | **Strengthened.** LiteParse's LibreOffice→PDF bridge invents pagination — the exact thing `bring-your-own-parser.md` forbids |
| `competitors.lock.json` | LiteParse pinned at **2.0.8**; platform artifacts imply no macOS x86_64 wheel | **Two corrections:** current is **2.11.1**; and `liteparse-2.0.8-cp312-cp312-macosx_10_12_x86_64.whl` **does exist** on PyPI. The lock's platform-artifact list is incomplete, not the world |
| §16.7 OCR posture | PP-OCR ONNX preferred; Tesseract excluded | Unchanged — **and** adopt LiteParse's HTTP OCR *contract* so any engine plugs in without an engine change |

### 18.8 OCR strategy for ethos-engine

| Lane | Engine | When used | Derivation / profile | Determinism claim | Licence | Ver |
| --- | --- | --- | --- | --- | --- | --- |
| **None (default)** | — | always, unless explicitly enabled | `Extracted` | full byte-identity under the pinned profile | — | **v0** |
| **Deterministic OCR** | **PP-OCRv5/v6 mobile ONNX** via `oar-ocr`/`ort` | opt-in, only on canvases with **no text layer** | `Recognized`, own profile `ethos-ocr-v1` | *"deterministic within a declared execution envelope"* — engine build, model sha256, runtime, CPU feature level, threads=1, pinned DPI | **Apache-2.0 code *and* weights** | **v2.1** |
| **Pluggable HTTP OCR** | any conforming server | adopter's choice | `Recognized`, profile records the endpoint identity | none claimed — declared as caller-supplied | adopter's | **v2.1** |
| **Tesseract** | — | never as default | `Recognized` if enabled at all | **cannot claim determinism** — documented cross-platform irreproducibility | Apache-2.0 | optional, opt-in |
| **VLM / accuracy lane** | GLM-OCR or similar | hard pages, human-reviewed | **`Proposed`** — never evidence | none | per-model | **v3** |

Answers to the specific questions:

- **Default OCR: none until v2.1.** v0 ships classification and honest refusal, not recognition.
- **The pluggable HTTP API is the right shape and should be adopted nearly verbatim** — `POST /ocr`,
  multipart `file`+`language`, `{results:[{text, bbox, polygon?}]}`. **Drop `confidence` from the
  contract we act on**: accept it if a server sends it, record it as a diagnostic, and never filter
  on it. LiteParse filters at 0.3 and again at 0.1, silently; that is the bug to not inherit.
- **Tesseract: optional, never the pinned default**, and any run that uses it declares
  "no determinism claim."
- **Scan triage feeds OCR without a confidence gate** because the trigger is a *reason code*
  (`no-text`, `scanned`) plus per-page counts, not a score crossing a threshold.
- **OCR never overwrites `Extracted` text** — it may author nodes only on canvases where the
  deterministic reader found no text layer at all.
- **Fingerprint isolation** comes free from the existing profile mechanism: a different
  `profile_sha256` makes an OCR'd document non-comparable with a born-digital parse by contract.
- **Rule 7**: if a VLM ever produces the representation, it must not also draft the claims. Give
  each lane a declared processor identity so a same-identity run can be rejected mechanically.

### 18.9 Roadmap deltas

Fold in; do not invent versions.

| Ver | Change |
| --- | --- |
| **v0** | **+ reason-code classifier on two orthogonal axes** (`ComplexityReason`-style ∪ `LayoutComplexityReason`-style), boolean derived, **three distinct exit codes** (simple / needs-attention / could-not-read). **+ `synthesized` flags** on characters PDFium invented |
| **v0.1** | unchanged (verify by shell-out, encoding detection, xref repair-or-refuse) |
| **v1** | **+ forms/AcroForm and annotations as typed, distinguishable nodes** (annotation text is never page text). **+ vector path data** to drive ruled-table detection. **+ DPI screenshots** |
| **v1.1** | unchanged (Markdown + Anchor Map) |
| **v1.2** | unchanged (MCP, SDKs, LangChain) |
| **v2** | unchanged — **Anydoc-native office parsers, explicitly not a LibreOffice→PDF bridge** |
| **v2.1** | **+ the LiteParse HTTP OCR contract** alongside the in-process PP-OCR lane |
| **v3** | unchanged |

### 18.10 Forced answers

1. **Is LiteParse a dependency, reference-only, benchmark row, or grounding adapter?**
   **Reference-only + benchmark row now; optional grounding adapter later; never a dependency.**
   It cannot be the grounded PDF core — it does not bind `FPDFText_GetCharOrigin`, emits floats,
   declares no coordinate space, and carries no schema version. A `liteparse → ethos.grounding.v1`
   adapter is a reasonable **v1.2+** item for OSS adopters who already run it, and it would declare
   `coordinate_origin: unknown` unless the adapter also reads the source PDF.
2. **Should classify UX follow `is-complex` reasons over pdf-inspector confidence?** **Yes,
   decisively** — with three exit codes instead of two, and with the reason list as the artifact
   rather than the boolean.
3. **What does LiteParse's PDFium packaging teach?** Mostly what *not* to do: an unpinned
   build-time download from a vendor fork, no checksum, `vendor/` absent so the build is
   network-dependent by default. Ethos's caller-provided + sha256-pinned posture is stronger. The
   one transferable idea is **runtime dynamic loading via `libloading`** rather than link-time
   binding, which Ethos already does.
4. **Office: Anydoc-native or LibreOffice bridge?** **Anydoc-native, and the bridge is a REFUSE,
   not a fallback.** Converting DOCX→PDF manufactures a page canvas the source does not have.
5. **OCR default + pluggable API for v2.1?** PP-OCRv5/v6 ONNX in-process as the deterministic lane;
   LiteParse's HTTP contract for everything else; no OCR at all before v2.1.
6. **Does LiteParse change the v1 gate (tables > 0.489)?** **No** — and it reinforces it. The 0.489
   figure is now triple-sourced, and LiteParse's own README names dense tables as the reason to
   leave for LlamaParse. Tables remain the v1 gate.
