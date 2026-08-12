# ethos-engine — combined OSS parity checklist

> **Location:** `ethos-engine/docs/reference/`. Research archive.
> **Bootstrap authority for implementation:** `docs/00`–`docs/07` (this tree).
> **Memo reading order inside this archive:** §16 → §17 → §18 (evidence §§3–5 still valid).

**Date:** 2026-08-12 · **Status:** bootstrap authority, jointly with memo §16 and §18 · **Sources:**
OpenDataLoader PDF, Anydoc, pdf-inspector, **LiteParse**

> This file is the full extract of memo **§17** and **§18** (LiteParse, §3b below). It lives **outside** the Ethos git tree. It is not
> `docs/` truth, not an ADR, not a claims or release authority.
>
> **Reading-order authority:** §16 + §17 + §18 (and this file) are the bootstrap authority ·
> `ethos-parser-expansion-memo.md` §3–§5, §16.3 and §18.3 are the evidence ·
> `ethos-docushell-parser-plan.md` is architecture depth only, superseded where it conflicts.

---

## 0. How to read this

**Decision vocabulary**

| Decision | Meaning |
| --- | --- |
| **TAKE** | Build it, essentially as the source does it. The idea and the execution are both sound |
| **IMPROVE** | Build it, but the source's execution is dishonest, lossy, or cliff-shaped. Take the goal, change the mechanism |
| **REFUSE** | Do not build it, or do not build it this way. Usually because it breaks a moat rule or a Workbench rule |
| **DEFER** | Right idea, wrong time. Named version, no work before it |

**Evidence rules.** README claims are marked as claims. Where a README conflicts with a measurement
in memo §16.3 or a source reading in §3/§5, **the memo wins and the row says so**. `[F]` = observed
fact, `[I]` = inference, `[R]` = recommendation.

### 0.1 Benchmark provenance — read before believing any number below

All four projects publish headline numbers, and **all are publisher-owned**:

| Publisher | Claim | Provenance problem |
| --- | --- | --- |
| ODL | "#1 in benchmarks (0.907 overall)" | Its own corpus (`opendataloader-bench`), and the 0.907 is the **hybrid AI-backed mode**. Its deterministic free mode scores **0.831 overall / 0.489 table** |
| pdf-inspector | "0.875 overall, best table and reading order" | Competitor's corpus, results on a **fork branch**, using **pdf-inspector 0.2.6 vs OpenDataLoader 2.2.1** — both far behind today's 1.14.1 / 2.5.x |
| Anydoc | "81 score, 14/14 formats, 4.4 ms median" | **LLM-as-judge** (Claude Sonnet 5, 482 verdicts) against LibreOffice-rendered ground truth, on a corpus the README says is **"not redistributable and is not in the repo"** |
| LiteParse | scored by *others*, not itself | **The clearest case of all — see below** |

**The one number publishers independently agree on** `[F]`: **OpenDataLoader's deterministic local
mode scores 0.489 on tables.** ODL's own table reports it; pdf-inspector's table reports the
identical figure. That agreement is worth more than either headline.

**And the one that shows why none of them can be trusted at face value** `[F]`: ODL and
pdf-inspector both score LiteParse on the *same* corpus with the *same* evaluator and report
**0.576 vs 0.873 overall, 0.000 vs 0.693 on tables** — a 34-point gap. The corpus and evaluator are
provably not the variable: ODL's own row and markitdown's row are **bit-identical** across both
tables. The cause is **invocation flags** — ODL ran LiteParse in its default `text` output format
(so a markdown-structure evaluator scored tables and headings at exactly 0.000) and with OCR on
(LiteParse's default), while pdf-inspector explicitly disabled OCR and requested markdown. Full
forensics in memo §18.4.

**This is the strongest available argument for Ethos's benchmark discipline.** A competitor
benchmarked a rival by running the first command in its README and published a number 34 points
low. `competitors.lock.json` pins the exact artifact *and invocation* for precisely this reason.
Ethos should publish none of these tables, and should cite this discrepancy when explaining why.

**Consequence, and it corrects memo §16.11 risk #1:** the bar for replacing ODL *deterministically*
on tables is **0.489**, not 0.928. The 0.928 comes from routing complex pages to an AI backend —
which is what DocuShell's `docling-fast` hybrid actually does today. Beating ODL-local on tables is
a realistic v1 target; matching ODL-hybrid is a different, non-deterministic product.

Ethos's own `docs/landscape-log.md` already requires publisher-owned suites to be labelled. All
four of the above are.

---

## 1. OpenDataLoader PDF

Apache-2.0 (Hancom). Engine: veraPDF (MPL-2.0, shaded into the CLI jar) + Apache PDFBox, on the JVM.

| # | Capability | README claim | Memo evidence | Decision | Ver | Why ethos-engine is better | Exit criterion | DS | OSS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| O1 | Deterministic local PDF extract | "Deterministic local mode" | `[F]` §3.3: real, but `--threads > 1` "output may vary slightly on some PDFs" — upstream's own words | **IMPROVE** | v0 | Determinism is a *contract* with byte-identity tests, not a mode name. Threads never change output | 3× same-host byte-identical on the full fixture corpus | High | High |
| O2 | JSON with bboxes + semantic types | "JSON with bounding boxes for source citations" | `[F]` §3.3: schema declares `[left, bottom, right, top]`, **no page dimensions anywhere**, `id` optional | **IMPROVE** | v0 | Declared coordinate system, declared page geometry, required stable ids, typed absence. The artifact can be interpreted without the source PDF | `ethos grounding check` passes with `coordinate_origin` declared, not `unknown` | High | High |
| O3 | Semantic element types (heading/para/list/table/caption/header/footer/image) | Capability matrix: all "Yes, Free" | `[F]` §16.1: Ethos's own `ElementType` declares 10 and produces 3 | **TAKE** | v1 | Same vocabulary, but every declared type has a producer and a fixture | No declared element type lacks a producer; CI asserts it | High | High |
| O4 | Reading order (XY-Cut++) | "wrong reading order" solved; RO 0.902 local | `[F]` §3.3: per-page XY-Cut (`XYCutPlusPlusSorter.java`); `id` order ≠ array order on multi-column | **TAKE (algorithm), IMPROVE (identity)** | v1 | XY-Cut is a good deterministic algorithm. Fix the split-brain: one order, ids derived from it | Kendall tau ≥ ODL-local on the shared corpus; ids monotone in reading order | High | High |
| O5 | Table model: row/col + rowspan/colspan + per-cell bbox + `is header` | "Table extraction (simple borders) Yes/Free" | `[F]` §5.3: the **model** is best-in-class; `[F]` §0.1: the **local accuracy is 0.489** | **TAKE (model), IMPROVE (detector)** | v1 | Take the schema wholesale — it *is* `TableCellPosition`. Build a detector that beats 0.489 and cross-checks geometry against structure | Table-cell accuracy > ODL-local on a labelled set; locator cross-check emits typed diagnostics | **High** | High |
| O6 | Complex/borderless tables | "Yes — Free (Hybrid)" | `[I]`: hybrid = route to AI backend | **DEFER** | v3 | When it lands it is a `Recognized` lane with its own profile, never born-digital certainty | An AI-derived table cell is fingerprint-incomparable with a deterministic one | Med | Med |
| O7 | Multi-page table linking (`previous/next table id`) | implicit in schema | `[F]` §3.3: present in schema | **TAKE** | v1 | Ethos's model has no multi-page table concept at all today | A table spanning a page break resolves as one table | Med | Med |
| O8 | Markdown output | "structured Markdown for chunking" | `[F]` §16.1: Workbench rule 8 — a projection is where locators die | **IMPROVE** | v1.1 | Markdown **always** ships with the Anchor Map (total tiling, `syntax` vs `source` segments). Never one without the other | A Markdown-quoted citation verifies end-to-end through the map | High | High |
| O9 | HTML output | "Web display with styling" | — | **DEFER** | v1.1 | Same map discipline or not at all | — | Low | Med |
| O10 | Annotated PDF (visual debugging) | "see detected structures" | — | **TAKE** | v1 | Genuinely useful and honest — it renders what was actually detected. Ours also renders *absence* | Overlay distinguishes `Geometry::Present` from typed absence | Med | High |
| O11 | Plain text output | listed | — | **TAKE** | v0 | Trivial, and the honest fallback when Markdown is not wanted | Byte-identical across runs | Low | Med |
| O12 | Images extracted with coordinates | "Yes, Free" | — | **TAKE** | v1 | Deterministic asset extraction with a real locator | Image node carries a real bbox or typed absence | Med | Med |
| O13 | Tagged-PDF structure **consumption** (`--use-struct-tree`) | "extracts the exact layout the author intended — no guessing" | `[F]` §3.3: ODL's whole structure model is accessibility-derived (veraPDF WCAG algorithms) | **TAKE** | v1 | Author-declared structure is the highest-quality signal available and most parsers ignore it. Bridge it to text runs via `mcid` | A tagged PDF yields author roles; an untagged one declares the limitation, never fakes it | High | High |
| O14 | Auto-tag → Tagged PDF **generation** | "First open-source tool to generate Tagged PDFs end-to-end", Apache-2.0 | `[F]` README capability matrix; built with Dual Lab/veraPDF | **DEFER — parallel lane** | v2.2+ | Different product, different buyer (remediation vs evidence). Writing tags is not verifying claims | Only starts on a named accessibility requirement | Low | Med |
| O15 | PDF/UA-1 / PDF/UA-2 export | "💼 Enterprise" | `[F]` README tier table | **REFUSE** | — | Not our product. Compliance export is a separate business | — | Low | Low |
| O16 | Accessibility studio (visual editor) | "💼 Enterprise" | — | **REFUSE** | — | Not our product | — | Low | Low |
| O17 | OCR, 80+ languages, `--force-ocr`, `--ocr-lang` | "Built-in OCR in hybrid mode… 300 DPI+" | `[I]` §3.5: OCR is a separate profile with its own fingerprint namespace | **TAKE (as a lane), IMPROVE (isolation)** | v2.1 | OCR output is `Recognized` under `ethos-ocr-v*`, never overwrites an `Extracted` node, and is fingerprint-incomparable with a born-digital parse **by the existing contract** | An OCR'd page grounds a quote and its fingerprint provably differs from the born-digital parse | Med | High |
| O18 | Per-page OCR routing | implicit via hybrid | `[F]` §16.3: pdf-inspector's `pages_needing_ocr` is the better-shaped version | **TAKE** | v0 | Emit per-page **counts**, not a verdict + score. Caller writes the policy explicitly | Per-page `has_text_layer` + counts in the artifact; no confidence field | High | High |
| O19 | Formula extraction → LaTeX | "Yes — Free (Hybrid)" | `[I]`: model transcription of pixels/glyphs | **DEFER** | v3 | `Recognized` at best. A LaTeX string is a *transcription*, and must carry its derivation class | Formula node is `Recognized`, excluded from the born-digital fingerprint | Low | Med |
| O20 | Chart/picture AI descriptions (SmolVLM 256M) | "useful for RAG search and accessibility alt text" | `[I]`: definitionally model-generated prose | **REFUSE as evidence / DEFER as annotation** | v3 | A description is `Proposed` — it is the model's words, not the document's. It may never be citable | If it ships, it is never a citable node and never enters a fingerprint | Low | Med |
| O21 | AI safety: hidden-text / off-page / invisible-layer filtering | "OpenDataLoader automatically filters" | `[F]` §3.3: `HiddenTextProcessor` **deletes** low-contrast text; the only call site passes `isFilterHiddenText = true`, so `hidden text: true` is unreachable from any CLI config | **TAKE (threat model), IMPROVE (mechanism)** | v1 | **Report, never delete.** Ethos already reserves `hidden_text_detected`, `off_page_text_detected`, `low_contrast_text_detected`. Deleting the evidence is what makes the rest of the document look clean | Hidden text produces a security finding and stays in the representation, flagged | **High** | High |
| O22 | Header/footer/watermark filtering | "Yes, Free" | `[F]` §3.3: `filterOutOfPage`, `filterTinyText`, `filterHiddenOCG` all **default true** — content vanishes with no record | **TAKE (classification), IMPROVE (disposition)** | v1 | **Classify, do not drop.** Emit `Header`/`Footer` element types and let the *exporter* exclude them. The representation keeps everything | An honest citation to a footnote near the trim edge still grounds | High | High |
| O23 | `--sanitize` (emails/URLs/phones → placeholders) | "sanitize sensitive data" | `[F]` §3.3: ten regexes incl. **any bare 10–18 digit number** and **any http(s) URL**, mutating text in place, nothing marks a node as sanitised | **REFUSE** | — | It rewrites the evidence you are grounding against. Financial documents are full of 10–18 digit numbers. Redaction is a downstream product decision, never a parser default | — | Low | Low |
| O24 | Python / Node / Java SDKs | listed | `[F]` README repeats 8× "each convert() spawns a JVM process, so repeated calls are slow" | **TAKE (SDKs), IMPROVE (no JVM)** | v1.2 | Pure-Rust core means no per-call JVM spawn. Anydoc's binding discipline is the model (§2) | Python + Node SDKs with no runtime beyond the native lib | Med | **High** |
| O25 | LangChain loader (`langchain-opendataloader-pdf`) | separate published package | `[F]` §16.7 | **TAKE** | v1.2 | Ours returns locators in the tool `artifact`, never only in `content` | A LangChain-loaded doc still carries page + bbox + tableCell | Low | High |
| O26 | Benchmarks / "#1" claims | "0.907 overall, #1" | `[F]` §0.1 above: publisher-owned; local mode is 0.831 / 0.489 | **REFUSE** | — | We publish no superlative without a gated harness and a named corpus. Ethos's claims gate already bans `fastest` pending G1 | No public ranking claim exists | Low | Low |
| O27 | Word / Excel / PPT | Capability matrix: **"Process Word/Excel/PPT — No"** | `[F]` README limitation row | **TAKE (the gap)** | v2 | This is ODL's declared hole and Anydoc's strength. Covering it with locators is a genuine differentiator | DOCX/XLSX citations ground with `Structural`/`Cellular` locators | High | High |
| O28 | JVM runtime dependency | "Requires Java 11+" | `[F]` README | **REFUSE** | — | Single Rust binary, no JVM, no per-call process spawn | `ldd`/`otool` shows no JVM; single static-ish binary | High | **High** |

---

## 2. Anydoc

MIT (Sideguide Technologies). Pure Rust, no ML, no native deps. PDF delegated to pdf-inspector.

| # | Capability | README claim | Memo evidence | Decision | Ver | Why ethos-engine is better | Exit criterion | DS | OSS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A1 | 14 formats (doc/docx/docm, ppt/pptx/…, xls/xlsx/xlsb, odt/ods/odp, rtf, epub, csv, pdf) | "14/14 formats" | `[F]` §3.2 | **TAKE** | v2 | Same coverage, plus locators. ODL cannot do these at all | Format × capability matrix filled, each with `supported`/`capability-limited`/`refused` | High | **High** |
| A2 | **Shared Document IR → one GFM serializer** | "output quirks get fixed once" | `[F]` §3.2: genuinely good design | **TAKE** | v2 | Same funnel, but the IR carries locators and the serializer emits an Anchor Map | A table-escaping fix for DOCX is automatically one for RTF | Med | High |
| A3 | Merged cells — `CellSlot::{Origin, Covered}` with an enforced exactly-once invariant | "tables with merged cells and header rows" | `[F]` §3.2: better than Ethos's own model | **TAKE** | v1 | Adopt the slot model directly; it maps cleanly to `TableCellPosition(row, col, rowspan, colspan, parent)` | A merged-cell citation resolves to the origin cell, never a covered slot | **High** | High |
| A4 | Content-based format detection (PDF header, RTF open group, OLE stream names, ZIP mimetype) | "mislabeled files still convert correctly" | `[F]` README | **TAKE** | v0 | Never trust the extension. Ours additionally **fails closed** on unknown magic rather than guessing | A `.txt`-named DOCX is detected; an unknown magic is refused, not guessed | Med | High |
| A5 | Error taxonomy: `Unsupported`/`Malformed`/`Encrypted`/`ResourceLimit`/`MissingPart`/`Io` | listed | `[F]` README | **TAKE** | v0 | An excellent, small, honest taxonomy. Maps directly onto Ethos's fail-closed error-code discipline | Every failure path returns exactly one typed variant; no generic error | Med | High |
| A6 | Embedded assets on the document model, tagged with media type | "raw bytes stay available… tagged with their media type" | `[F]` §3.2 | **TAKE** | v2 | Same, plus a locator per asset | Asset node carries media type + a real locator or typed absence | Low | Med |
| A7 | Speed: pure Rust, no ML, median < 5 ms/doc | "Median conversion time is under 5ms per document" | `[F]` §0.1: LLM-judged, non-redistributable corpus, process spawn excluded | **TAKE (the target), REFUSE (the claim)** | v2 | Adopt single-digit-ms office conversion as an internal target on a **named, redistributable** corpus | Median office convert < 10 ms on a named corpus, method published | Med | High |
| A8 | Bindings: Node on libuv threadpool (never blocks the event loop), Python releases the GIL, TS types + Python stubs shipped | listed | `[F]` README | **TAKE** | v1.2 | This is exactly the right binding discipline. Copy it | Node binding does not block the event loop under load; Python releases the GIL | Low | **High** |
| A9 | WASM (browser) | listed | `[F]` README | **DEFER** | v2+ | Real adopter value, no DocuShell need. Cheap once the core is `no_std`-friendly | — | Low | Med |
| A10 | Agent Skill (`npx skills add`) | "any agent can read office documents" | `[F]` README | **DEFER** | v1.2 | MCP covers the same audience with an enforced output schema (§16.7) | — | Low | Med |
| A11 | Testing discipline: snapshot tests + `tests/robustness.rs` mutation-testing every fixture + cargo-fuzz per format | listed under Development | `[F]` README | **TAKE** | v0 | **The single most copyable thing in the repo.** Mutation-testing every fixture is exactly right for a parser whose product is honesty | Fuzz target per format; mutation test over the whole fixture corpus, in CI from week one | Med | High |
| A12 | PDF via pdf-inspector → Markdown directly | "Text-based PDFs convert locally… no OCR service required" | `[F]` §3.2: **PDF bypasses the IR entirely**; `formats/pdf.rs:15` keeps only `result.markdown`, discarding per-run `x/y/width/height/page/mcid` **and** `LayoutComplexity` | **REFUSE** | — | Our PDF path is first-class and locator-bearing. Never route grounded PDF through a Markdown-only bridge | PDF and office paths produce the same representation type | **High** | High |
| A13 | Honest absence of locators in the IR | not claimed | `[F]` §3.2: no page, no bbox, by design — scores **0** not 1 on bbox because it never pretends | **TAKE (the honesty)** | — | Same principle, typed: `Geometry::Absent(NoCanvas)` for Flow canvases | No synthesised page or box on any Flow-canvas node | High | High |
| A14 | Markdown erasure (spans flattened, header row synthesised, trailing empties truncated) | not claimed | `[F]` §3.2: the Markdown table is **not** the IR table | **IMPROVE** | v1.1 | Erasure is *declared* in the Anchor Map's coverage block, and CI asserts `emitted ∪ dropped == all` | Completeness assertion green on every fixture | Med | Med |

---

## 3. pdf-inspector

MIT (Firecrawl). ~83k LOC Rust, `lopdf` + `ttf-parser`, no PDFium, no ML, no network.

**§16.3 measurements are authoritative wherever they conflict with the README.**

| # | Capability | README claim | Memo evidence | Decision | Ver | Why ethos-engine is better | Exit criterion | DS | OSS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| P1 | Smart classification (TextBased/Scanned/ImageBased/Mixed) | "in ~10-50ms by sampling content streams" · "detects 300+ page PDFs in milliseconds" | **§16.3 wins** `[F]`: the figure is a **doc-comment at `src/lib.rs:390`**, not a benchmark. Measured **412 ms on 492 pages**. Sampling is defeated by a Phase-3 full rescan (`detector.rs:431-447`) — `Pages(1)` costs 439 ms vs `Full` 434 ms | **IMPROVE** | v0 | Ours is **genuinely bounded**: sample N, stop. Cost = parse + N × per-page scan, and does not grow with total pages | Classifying a 500-page PDF at N=8 is within 20% of an 8-page PDF at similar bytes/page | High | High |
| P2 | Four-way type taxonomy | listed | `[F]` §16.3: `simple-text` classifies **TEXT-BASED with `Pages with text: 0`** | **IMPROVE** | v0 | Emit the **observation** (`pages_with_text`, `pages_sampled`, per-page `has_text_layer`), and let the *caller* name the type. A verdict that contradicts its own counts is worse than no verdict | Artifact carries counts; the four-way label is derived and reproducible from them | High | High |
| P3 | **Confidence score (0.0–1.0)** | "Returns a confidence score (0.0-1.0)" | **§16.3 wins** `[F]`: six literals in a lookup table; `simple-text` → TEXT-BASED at 50% with zero text pages | **REFUSE** | — | Workbench **rule 9**: confidence is not a gate. You cannot threshold a number that does not exist. Counts are facts; confidence is an opinion | No `confidence` field in any public artifact | **High** | High |
| P4 | Per-page OCR routing (`pages_needing_ocr`) | "per-page OCR routing instead of all-or-nothing" | `[F]` §16.3: 0-vs-1-indexed inconsistency between `PdfProcessResult` and `PdfClassification` | **TAKE (idea), IMPROVE (contract)** | v0 | One base (1-based), tested, and per-page counts rather than a bare list | A page-index round-trip test pins the base | High | High |
| P5 | Position-aware text: font info, X/Y, per-item | "Position-aware extraction with font info, X/Y coordinates" | **§16.3 wins** `[F]`: `x`/`width` content-stream-real; **`y` is a baseline**; **`height` IS `font_size`** (same variable); items are per-Tj-run, not per-glyph | **TAKE (design), IMPROVE (honesty)** | v0 | `NativeLocator` = origin + advance, exact. Ink box from **measured font metrics** or typed absence. Never `height = font_size` | No emitted box is derived from font size; fabrication rate exactly 0 | **High** | High |
| P6 | Operator coverage | implied complete | **§16.3 wins** `[F]`: the `"` show-text operator is **absent from the match** — its text vanishes silently and surrounding runs merge with corrupt geometry. `Tz` absent from the whole tree, so `.width` is wrong under horizontal scaling | **IMPROVE — the disqualifier** | v0 | Enumerate the operator set explicitly and **fail closed on an unrecognised operator**. Workbench rule 4: absence of extracted content is never evidence of absence | A synthetic fixture using `"` and `Tz` either extracts correctly or refuses with a named limitation | **High** | High |
| P7 | Multi-column reading order | "automatic multi-column reading order" | **§16.3 wins** `[F]`: flips on one line of text — `min_lines < 15` at `layout.rs:1793`; 14 lines → row-interleaved, 15 → column-major | **IMPROVE** | v1 | A stable, documented rule with a fixture at the boundary. v0 ships single-column plus an explicit limitation rather than inheriting a cliff | A ±1-line perturbation does not reorder the page | High | High |
| P8 | RTL text support | listed | not verified | **DEFER** | v1 | Needs a fixture before it is claimed | — | Low | Med |
| P9 | CID / ToUnicode CMap decoding (Type0/Identity-H, UTF-16BE, UTF-8, Latin-1) | listed | `[F]` §16.3: ~25k LOC + 168 Adobe `.bcmap` files | **TAKE — vendor the data, clean-room the logic** | v0 | Vendoring the CMap **tables** (MIT, with the Adobe BSD-3-Clause NOTICE) skips the tedious part; the decode logic is ours and testable | A CID-font fixture round-trips to correct Unicode | High | High |
| P10 | **Encoding-issue detection** ("flags broken font encodings so callers can fall back to OCR") | listed | `[F]` README | **TAKE** | v0.1 | Genuinely good idea, and it is a *capability limitation*, which is exactly our vocabulary. Ours emits it as a declared limit, not a hint | A broken-encoding fixture emits a named limitation, not mojibake | High | High |
| P11 | **Single document load shared between detect and extract** | "avoiding redundant I/O" | `[F]` README | **TAKE** | v0 | Obviously right, and it is how classify→extract stays cheap | Classify+extract loads the document once; asserted in a test | Med | Med |
| P12 | Table detection: rectangle-based (drawing ops) + heuristic (text alignment) | "Dual-mode… financial tables, footnotes, continuation tables" | `[F]` §16: Ethos has **no ruling-line detection at all**; ODL-local scores 0.489 | **TAKE (dual-mode idea)** | v1 | Rectangle-based detection from `re` operators is the piece Ethos lacks entirely. Add the geometric↔structural cross-check nobody else has | Ruled + unruled fixtures pass; cross-check emits typed diagnostics | **High** | High |
| P13 | Markdown: headings via font-size tiers + 0.5 pt clustering | listed | `[I]`: threshold heuristic | **IMPROVE** | v1.1 | Prefer author-declared structure (`use_struct_tree`, `mcid`) and fall back to font tiers **with the fallback declared** | Heading source (`tagged` vs `inferred`) recorded per node | Med | Med |
| P14 | Markdown: bold/italic from **font-name patterns**; code blocks from **monospace font names**; captions from **"Figure"/"Table"/"Source:" prefixes** | listed | `[I]`: name/prefix inference presented as structure | **REFUSE (as structure), DEFER (as annotation)** | — | A font called `Courier` is not a code block. These are guesses; if emitted at all they are `Computed` with a declared basis | No style/role node is derived solely from a font name or a text prefix | Low | Low |
| P15 | Hyphenation rejoin across lines | listed | `[F]` Ethos already has fixture `hyphenated-line-break` | **TAKE** | v1.1 | Same, and `element.text` keeps exactly what was extracted while the *export* repairs | Fixture passes; raw text unmodified | Med | Med |
| P16 | Dot-leader collapsing, drop caps, sub/superscript | listed | `[I]` | **TAKE (export-only)** | v1.1 | Cosmetic export transforms only; they never touch the representation | Anchor Map marks them `syntax` or an invertible `emit` | Low | Low |
| P17 | **Page-number filtering** ("Page numbers filtered from output") | listed | `[I]`: same class as ODL's silent deletion (O22) | **IMPROVE** | v1.1 | Classify as `Header`/`Footer`, exclude in the *exporter*, keep in the representation | A citation to a page number still grounds | Med | Med |
| P18 | Browser WASM with embedded CMaps | listed | `[F]` README | **DEFER** | v2+ | Same as A9 | — | Low | Med |
| P19 | Structure tree reading (`structure_tree.rs`, `mcid`) | not headlined | `[F]` §16.3: `mcid` on `TextItem` is the honest bridge to the tag tree | **TAKE** | v1 | Pairs with O13. `mcid` is how a geometric run binds to an author-declared role | A tagged PDF's roles attach to the correct text runs via `mcid` | High | High |
| P20 | Robustness: strict xref | not claimed | `[F]` §16.3: `table-regular-grid/document.pdf` has **19-byte xref entries** where PDF 32000-1 §7.5.4 requires 20 — PDFium repairs, lopdf refuses. **~4% open-failure on Ethos's own valid corpus** | **IMPROVE** | v0.1 | A bounded, declared repair path for known-benign malformations (20-byte stride tolerance), or an honest `Malformed` refusal. Never a silent partial parse | The fixture either opens with a declared repair or fails with a named error | **High** | High |

---

## 3b. LiteParse

Apache-2.0 (run-llama), **no addendum**. Rust + PDFium (downloaded at build time from a run-llama
fork, unpinned). Pinned in Ethos's `competitors.lock.json` at 2.0.8; current upstream is **2.11.1**.
Ethos PRD §2.1 names it *"the closest overlap for Ethos' Release 1 parser-core surface."*
Full analysis in memo **§18**.

| # | Capability | README claim | Memo evidence | Decision | Ver | Why ethos-engine is better | Exit criterion | DS | OSS |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| L1 | **Complexity reason codes** | "Cheaply check whether a document needs OCR" | `[F]` §18.1: `ComplexityReason::{Scanned,NoText,SparseText,EmbeddedImages,Garbled,VectorText,AnnotationText}` (`ocr_merge.rs:53-81`) | **TAKE** | v0 | Same vocabulary, plus every reason maps to a declared capability limit | Every routing decision traces to a named reason | High | High |
| L2 | **Two orthogonal verdict axes** | not claimed | `[F]` `LayoutComplexityReason::{MultiColumn,TableLikely,DenseGraphics}`, doc'd *"none of these imply OCR"* | **TAKE** | v0 | Separating "needs OCR" from "layout is hard" is the right decomposition; nobody else has it | `table-likely` never triggers an OCR route | High | High |
| L3 | Boolean derived from reasons | — | `[F]` `needs_ocr = !reasons.is_empty()` (`:235`) | **TAKE** | v0 | The list is truth, the boolean a convenience | Removing the boolean loses no information | Med | Med |
| L4 | **No confidence in classify output** | — | `[F]` §18.1: grepped Rust/napi/WASM/Py/TS — none | **TAKE** | v0 | Rule 9 satisfied by construction. pdf-inspector fails this | No confidence field in any classify artifact | **High** | High |
| L5 | Per-page detail, 1-indexed | "list the pages that need OCR" | `[F]` `PageComplexityStats`, 1-based | **TAKE** | v0 | Per-page routing without all-or-nothing | Index base pinned by a round-trip test | High | High |
| L6 | **`trailing_space_generated`** | not claimed | `[F]` `types.rs:110-112` — marks spaces PDFium synthesized vs real glyphs | **TAKE** | v0 | **The best honesty field in any of the four projects** | Every synthesized character flagged at emission | High | High |
| L7 | `char_codes` + ligature caveat | — | `[F]` `types.rs:103-108` | **TAKE** | v0 | Glyph-vs-scalar mismatch declared, not papered over | Ligature fixture round-trips with the mismatch declared | Med | Med |
| L8 | OCR/native discriminator | — | `[F]` `confidence: Option<f32>`, "None for native PDF text" | **IMPROVE** | v0 | Right idea, wrong carrier — ours is a typed `DerivationClass`, not a nullable float | OCR text distinguishable by type, not field absence | **High** | High |
| L9 | **Open HTTP OCR API spec** | "Standard API" | `[F]` `OCR_API_SPEC.md`: `POST /ocr`, multipart, `{results:[{text,bbox,confidence,polygon?}]}`; polygon in the upright reading frame recovers rotation | **TAKE (contract), IMPROVE (drop the gate)** | v2.1 | Adopt nearly verbatim; accept `confidence` as a diagnostic and **never filter on it** | Any conforming server works with no engine change | Med | **High** |
| L10 | Structure-tree extraction | "tagged-PDF logical structure in JSON" | `[F]` real; bridges tags to runs partially | **TAKE** | v1 | Pairs with ODL's `use_struct_tree` and pdf-inspector's `mcid` | Tagged roles attach to the right runs | High | High |
| L11 | **Vector path data** | "page-scoped vector path data" | `[F]` `filled_path_bounds(3.0, 0.9)` | **TAKE** | v1 | Ruling-line geometry is what a real table detector needs; Ethos has none | Ruled-table fixtures detected from path data | **High** | Med |
| L12 | **Forms (AcroForm)** | "repairs orphaned widgets in memory" | `[F]` §18.2 #13: an **undeclared document mutation**; widget flattening always on | **TAKE (extraction), REFUSE (silent repair)** | v1 | Any repair is recorded as a typed diagnostic, never silent | A repaired form emits a diagnostic naming the repair | High | Med |
| L13 | **Annotations** | "page annotations in structured JSON" | `[F]` real | **TAKE (typed)** | v1 | Annotation text must be **distinguishable** from page content — mixing them is an evidence and security problem | Annotation-sourced text is typed as such | High | Med |
| L14 | Screenshots / DPI rasters | "essential for LLM agents" | `[F]` real, DPI-controlled | **TAKE** | v1 | Feeds crops and debug overlays deterministically | Deterministic raster at a pinned DPI | Med | High |
| L15 | Content bounds, metadata, XFA | listed | `[F]` real, not stubs | **DEFER** | v1+ | Useful, not v0 | — | Low | Low |
| L16 | `is-complex` as a shell predicate | `lit is-complex doc.pdf --quiet && lit parse …` | `[F]` §18.3: **exit 1 means "complex" OR "could not open"** — measured on password-protected, invalid-header, corrupt-header, missing file | **IMPROVE** | v0 | **Three outcomes, three exit codes.** Fails closed, but LiteParse's signal is indistinguishable | Encrypted ≠ complex ≠ simple at the exit-code level | High | High |
| L17 | Bindings: Rust/Node/Py/WASM/CLI + agent skill | listed | `[F]` all published | **TAKE** | v1.2 | The adoption bar | Parity with Anydoc's binding discipline | Low | **High** |
| L18 | **Spatial bbox = loose char boxes** | "Precise text positioning information" | `[F]` §18.2 #5: union of `FPDFText_GetLooseCharBox` — em boxes, ascent-to-descent, **not ink**. "For a line of 'acme' the box is as tall as if it contained 'Ãj'" | **IMPROVE** | v0 | Ink box from measured font metrics, or typed absence — and **declared either way** | No box's semantics are undeclared | **High** | High |
| L19 | **Character origins** | — | `[F]` §18.2 #6: `FPDFText_GetCharOrigin` **is not bound at all** | **REFUSE (as a source)** | — | Origins are Ethos's fingerprint-critical primitive. This design cannot supply them | Origins come from our own extraction | **High** | Med |
| L20 | **Versioned output contract** | — | `[F]` §18.2 #2: `output/json.rs:46-65` emits no `schema_version`, no `parser_version`; grep returns nothing | **REFUSE (the omission)** | v0 | This is Ethos's declared differentiator and it is real | Every artifact carries `artifact_type` + `schema_version` | **High** | High |
| L21 | **Undeclared coordinate space** | — | `[F]` viewport/top-left/72 DPI/CropBox→MediaBox, documented only in Rust doc comments and TS JSDoc | **REFUSE** | v0 | An artifact must be interpretable without its source | `coordinate_system` declared on the wire | **High** | High |
| L22 | **Floats on the wire** | — | `[F]` `f32` throughout, serde_json decimal, lossy round-trip | **REFUSE** | v0 | Integer centipoints; "floats do not exist in canonical Ethos" | No float in any canonical artifact | High | Med |
| L23 | **Confidence used as a silent filter, twice** | — | `[F]` `ocr/tesseract.rs:131` `conf > 0.3`; `ocr_merge.rs:632` `<= 0.1` | **REFUSE** | — | Rules 9 and 4 — text dropped below a threshold with no record | No text is dropped for a score; drops are declared | **High** | High |
| L24 | **OCR merged into the native stream** | architecture diagram: "OCR Merge — Native text + OCR results" | `[F]` `ocr_merge.rs` | **REFUSE** | — | `Recognized` never merges into `Extracted`; separate profile, separate fingerprint namespace | An OCR'd page's fingerprint provably differs | **High** | High |
| L25 | **OCR on by default** | "OCR enabled by default" | `[F]` `config.rs:209` | **REFUSE** | — | OCR is opt-in, and absent before v2.1 | Default build performs no recognition | High | High |
| L26 | **Tesseract bundled as default** | "zero setup, bundled" | `[F]` + prior memo §3.5: documented cross-platform irreproducibility | **REFUSE (as default)** | — | PP-OCR ONNX (Apache-2.0 code *and* weights) is the deterministic pick | No determinism claim is made for any Tesseract run | Med | Med |
| L27 | **`keep_headers_footers: false` default** | — | `[F]` `config.rs:226` | **REFUSE** | — | Classify as Header/Footer; the *exporter* excludes. The representation keeps everything | A citation to a footer still grounds | **High** | Med |
| L28 | **`preserve_small_text` opt-in** | — | `[F]` `main.rs:88` | **REFUSE** | — | Same rule: never drop by default | A citation to small print still grounds | High | Med |
| L29 | **Markdown: no locator, structure from font names** | "Structured Markdown … great for RAG" | `[F]` §18.2 #14: `markdown_layout/` ~11k lines, font-NAME inference confirmed | **REFUSE** | — | Markdown ships only with the Anchor Map; no role from a font name | A Markdown citation resolves through the map | High | High |
| L30 | **LibreOffice → PDF for office** | "automatic conversion" | `[F]` §18.2 #15: external user-installed binary; **converting DOCX→PDF invents pagination** | **REFUSE** | — | Anydoc-native parsers per format. `bring-your-own-parser.md`: "Pagination does not exist in the file and must not be synthesised" | No office format acquires a synthetic page canvas | **High** | High |
| L31 | Build-time PDFium download | — | `[F]` §18.2 #17: `pdfium-sys/build.rs:85-163` — env → absent `vendor/` → auto-download from a **run-llama fork**, **by tag, no checksum** | **REFUSE** | — | Caller-provided + sha256-pinned, per ADR-0002/0013/0015 | Air-gapped build works with no network | High | Med |
| L32 | LlamaParse cloud upsell | "you'll get significantly better results with LlamaParse" | `[F]` README top | **REFUSE (as strategy), NOTE (as a tell)** | — | The vendor names its own weak spots — dense tables, multi-column, charts, handwriting, scans — and they are exactly DocuShell's documents | — | Med | Low |

**Adapter cost estimate.** A `liteparse → ethos.grounding.v1` adapter is **~300–500 LOC** and would
declare `coordinate_origin: unknown` unless it also reads the source PDF for page geometry — the
same fork the ODL adapter hit. Reasonable as a **v1.2+** convenience for adopters already running
LiteParse; never the grounded core.

---

## 4. Ethos-native wins to keep explicit

These are not parity items. They are the reasons the engine exists, and none of the three sources has them.

| # | Capability | Nearest source | Decision | Ver | Why it is the moat |
| --- | --- | --- | --- | --- | --- |
| E1 | **Citation verification** ("did this claim come from this document?") | none | **KEEP** | v0.1 | No parser in the landscape verifies. This is the product |
| E2 | Capability declarations + fail-closed | none — all three fail open or filter silently | **KEEP** | v0 | L1's achievement condition names capability declarations. They are the gate, not polish |
| E3 | Profile-as-identity (`profile_sha256` in the fingerprint) | none | **KEEP** | v0 | Gives OCR isolation, backend isolation and comparability for free, with no new machinery |
| E4 | c14n v1 + integer quanta ("floats do not exist in canonical Ethos") | none | **KEEP** | v0 | Byte-identical output is a contract, not an aspiration |
| E5 | `DocumentRepresentation v0` + `ethos.grounding.v1` | ODL's JSON is the closest and cannot self-declare its coordinate system | **KEEP** | v0 | An artifact that can be interpreted without the source file |
| E6 | **Intra-representation geometric ↔ structural locator cross-check** | none | **KEEP** | v1 | *"A single locator can only be trusted or not; a pair can be tested."* This is the fidelity mechanism, and it is why RAGFlow was rejected |
| E7 | Derivation honesty (`Extracted`/`Computed`/`Recognized`/`Proposed`) | none | **KEEP** | v0 | The axis that lets OCR and assist exist without laundering into born-digital certainty |
| E8 | BYO parser path (`GroundingSource`) forever | none | **KEEP** | v0 | Moat rule 2. The ODL adapter staying green is the continuous proof |

---

## 5. Summary — TAKE / IMPROVE / REFUSE / DEFER (four sources)

**TAKE (36)** — build essentially as the source does
*ODL:* O3 semantic element types · O4 XY-Cut reading order · O7 multi-page tables · O10 annotated
PDF · O11 text output · O12 images with coords · O13 tagged-PDF consumption · O18 per-page routing ·
O27 the Word/Excel/PPT gap.
*Anydoc:* A1 14 formats · A2 shared IR → one serializer · A3 CellSlot merged cells · A4
content-based detection · A5 error taxonomy · A6 embedded assets · A11 mutation + fuzz testing.
*pdf-inspector:* P9 CMap tables (vendor) · P10 encoding-issue detection · P11 single load ·
P12 dual-mode tables · P15 hyphenation · P19 `mcid` bridge.
*LiteParse:* **L1 reason codes** · **L2 two orthogonal axes** · L3 derived boolean · **L4 no
confidence in classify** · L5 per-page 1-indexed · **L6 `trailing_space_generated`** · L7
`char_codes` + ligature caveat · **L9 open HTTP OCR contract** · L10 structure tree · **L11 vector
path data** · L12 forms (extraction) · L13 annotations (typed) · L14 DPI screenshots · L17 bindings.

**IMPROVE (20)** — right goal, wrong mechanism
O1 determinism as a contract · O2 self-describing JSON · O5 table detector · O8 Markdown + Anchor
Map · **O21 hidden text → report, not delete** · **O22 header/footer → classify, not drop** ·
O24 SDKs without a JVM · A14 declared erasure · P1 bounded classification · P2 counts not verdicts ·
P5 honest geometry · **P6 fail closed on unknown operators** · P7 stable reading order · P13
declared heading basis · P17 page numbers classified not deleted · P20 xref robustness ·
**L8 typed derivation instead of a nullable confidence** · **L16 three exit codes, not two** ·
**L18 declared box semantics** · L12 forms repair recorded.

**REFUSE (19)**
*ODL:* O15 PDF/UA export · O16 accessibility studio · O20 chart descriptions **as evidence** ·
O23 `--sanitize` · O26 "#1" claims · O28 JVM dependency.
*Anydoc:* A12 PDF-through-Markdown bridge.
*pdf-inspector:* P3 public confidence float · P14 style/role from font names and prefixes.
*LiteParse:* **L19 char origins unavailable** · **L20 no versioned output contract** · **L21
undeclared coordinate space** · **L22 floats on the wire** · **L23 confidence as a silent filter** ·
**L24 OCR merged into the native stream** · **L25 OCR on by default** · L26 Tesseract as default ·
**L27 headers/footers dropped by default** · L28 small text dropped by default · **L29 Markdown
without a locator** · **L30 LibreOffice→PDF office bridge** · L31 unpinned PDFium download.

**DEFER (9)**
O6 complex-table hybrid (v3) · O9 HTML (v1.1) · O14 auto-tagging (v2.2+, parallel lane) ·
O19 formula LaTeX (v3) · A9 WASM (v2+) · A10 agent skill (v1.2) · P8 RTL (v1) · P18 WASM (v2+) ·
L15 content bounds / metadata / XFA (v1+).

### 5b. OCR strategy — summary

Full table and reasoning in memo **§18.8**.

| Lane | Engine | Derivation / profile | Determinism claim | Ver |
| --- | --- | --- | --- | --- |
| **None (default)** | — | `Extracted` | full byte-identity under the pinned profile | **v0** |
| Deterministic OCR | **PP-OCRv5/v6 mobile ONNX** (Apache-2.0 code *and* weights) | `Recognized`, profile `ethos-ocr-v1` | *"deterministic within a declared execution envelope"* | **v2.1** |
| Pluggable HTTP OCR | any conforming server, **LiteParse's contract** | `Recognized`, endpoint identity in the profile | none claimed — caller-supplied | **v2.1** |
| Tesseract | — | `Recognized` if enabled | **no determinism claim** | optional, never default |
| VLM accuracy lane | GLM-OCR or similar | **`Proposed`** — never evidence | none | **v3** |

Three rules: OCR is **opt-in**; it may author nodes **only** where the deterministic reader found no
text layer; and `confidence` is accepted as a diagnostic and **never filtered on** — LiteParse drops
text at 0.3 and again at 0.1, silently, and that is the bug not to inherit.

---

## 6. Version mapping

| Ver | Theme | Contents | Gate |
| --- | --- | --- | --- |
| **v0** | Honest PDF core | classify — **reason codes on two orthogonal axes, counts, no confidence, three exit codes** · position-aware runs · measured ink box or typed absence **with declared semantics** · **`synthesized` flags** · single-column order · format detection · error taxonomy · c14n/quanta/ids · capability declarations · `ethos.grounding.v1` · `grounding-check` · CLI + lib · fuzz + mutation tests | Validator agrees byte-identically with `ethos grounding check` on all 15 fixtures |
| **v0.1** | Verify + robustness | shell out to Ethos CLI · encoding-issue detection · xref repair-or-refuse | An ungrounded claim exits 1 with a report; no silent skip |
| **v1** | **The DocuShell replacement gate** | tables (ruled + unruled) with locator cross-check · **vector path data** driving ruled detection · full element vocabulary incl. Header/Footer/Caption · multi-column with a stable rule · tagged-PDF consumption + `mcid` + structure tree · **forms and annotations as typed, distinguishable nodes** · **DPI screenshots** · security findings (hidden/off-page) · images · annotated PDF | Table-cell accuracy **> ODL-local (0.489)** on a labelled set, fabrication rate 0, cross-check diagnostics emitted |
| **v1.1** | Safe Markdown | Markdown + **Anchor Map** · HTML · hyphenation/dot-leaders/drop-caps as export-only | A Markdown-quoted citation verifies end-to-end; coverage completeness asserted |
| **v1.2** | Adoption | **MCP server** · Python + Node SDKs · LangChain tool | Locators survive every adapter round-trip |
| **v2** | Anydoc-class formats | DOCX → XLSX → PPTX → ODF/RTF/EPUB/CSV · shared IR + one serializer · embedded assets | A DOCX quote and an XLSX cell both ground; no synthesised pages |
| **v2.1** | OCR lane | PP-OCR ONNX in-process · **LiteParse-compatible HTTP OCR contract** · own profile · per-page routing · never overwrites `Extracted` · confidence never filtered on | OCR fingerprint provably incomparable with born-digital |
| **v2.2** | Accessibility (parallel lane) | auto-tag → Tagged PDF | Only on a named accessibility requirement |
| **v3** | Assist | propose-only VLM · dual-read → review · hybrid enrichments (formula, chart) as `Recognized`/`Proposed` | Byte-diff: assist on/off ⇒ identical grounded artifacts |

---

## 7. Performance bar

Targets, with methods. **None of these is publishable** until Ethos's G1 gate exists and a corpus
is named (`claims_gate.py` bans `fastest` with the reason "needs reproducible benchmark + G1 pass").

| Axis | Target | Method |
| --- | --- | --- |
| **Classification** | ≤ **0.5 ms per sampled page** in-process, default N = 8; **cost must not scale with total page count** | Best-of-5, in-process, Ethos frozen corpus. The property pdf-inspector advertises and lacks |
| **Classification bound test** | 500-page PDF at N=8 within **20%** of an 8-page PDF at similar bytes/page | Direct A/B — this is the test that would have caught `detector.rs:431-447` |
| **Born-digital extract** | **≥ 134 pages/sec p50** | Derived from Ethos's own G1 rule `≥ max(120 pps, 2× remeasured ODL)`, with ODL-local measured by its own README at 0.015 s/page ≈ 67 pps. Harness: `benchmarks/harness/run_gate_zero.py` `measure_command()` |
| **Office convert** | median **< 10 ms/doc** | On a **named, redistributable** corpus. Anydoc claims 4.4 ms median but excludes process spawn and cannot share its corpus |
| **Verify path** | spawn cost acknowledged, not optimised | ~19–22 ms measured process-spawn floor for a Rust CLI. At 20,000 docs/day ≈ 14/min it is irrelevant. **Do not micro-optimise** |
| **Memory** | declared per configuration | Ethos's documented 2 KB/element ceiling is already measured at 2.66 KB/element and recorded undecided — decide it before it blocks a release |
| **Claims** | none | No "fastest"/"#1"/"best" without a gated harness and a named corpus. All three sources' headline numbers are publisher-owned (§0.1) |

---

## 8. When ethos-engine stands tall

Not "when it wins a bake-off." When it does things the other **four** structurally cannot.

**0. It is the only one with a versioned output contract.** `[F]` None of ODL, Anydoc,
pdf-inspector or LiteParse emits a `schema_version`, a `parser_version`, or a declared coordinate
system on the wire. LiteParse — the closest competitor, and the most honest codebase of the four —
emits `page, width, height, text, text_items` and nothing else; its top-left/72-DPI/CropBox origin
is documented **only in Rust doc comments**. Ethos's PRD §2.1 lists *"deterministic marketing
language without a versioned contract"* as an explicit non-moat. The contract is the moat, and it
is the one thing no competitor has started.

1. **Honesty the others cannot match, because it is in the type system.** No box derived from a font
   size (P5). No silent operator drop (P6). No confidence to threshold on (P3). No deleted hidden
   text (O21) or dropped footer (O22). No sanitised evidence (O23). Absence is typed, and a missing
   capability is a declaration rather than a gap.
2. **Verification.** ODL, Anydoc and pdf-inspector all answer *"what does this document say?"*
   ethos-engine + Ethos answer *"did this claim come from it?"* Nothing else in the landscape does.
3. **Multi-format with locators.** ODL cannot process Word/Excel/PPT at all (its own capability
   matrix says so). Anydoc covers 14 formats and carries no page, no bbox and no element id.
   ethos-engine covers Anydoc's formats *and* keeps a locator on every node.
4. **Local PDF at least as useful as ODL-local plus pdf-inspector's and LiteParse's ideas.**
   ODL-local's table score is 0.489 by two independent publishers' agreement; pdf-inspector's
   geometry is half-synthesised; LiteParse's boxes are loose em boxes sold as "precise positioning"
   and it does not bind character origins at all. Taking ODL's *table model*, pdf-inspector's
   *rectangle detection*, LiteParse's *vector path data*, and adding the geometric↔structural
   cross-check beats all three on the axis that matters — whether a cited cell is the right cell.
5. **Hybrid and OCR without laundering.** OCR is `Recognized` under its own profile, so an OCR'd
   page is fingerprint-incomparable with a born-digital parse *by the existing contract*. ODL's
   hybrid mode has no such boundary — its AI-derived table and its deterministic table look
   identical in the output.
6. **Markdown that can be verified.** When Markdown ships it ships with the Anchor Map: total
   tiling, `syntax` bytes that are never quotable, `source` bytes that invert to canonical text.
   Anydoc's Markdown table is not Anydoc's IR table, and nothing records the difference.
7. **Adoption without a second citation authority.** CLI, library, SDKs and MCP — where the engine
   mints every locator and re-validates it on the way back in, so no host can become the authority.
   LiteParse matches the binding breadth; nothing matches the authority discipline.
7b. **Routing you can write policy against.** LiteParse's reason codes on two orthogonal axes are
   the right shape and ethos-engine adopts them — with three exit codes instead of two, so
   *"complex"*, *"simple"* and *"I could not open this"* are finally distinguishable. LiteParse's own
   README predicate cannot tell an encrypted file from a scanned one.
8. **What it deliberately does not do.** No PDF/UA export, no accessibility studio, no chart
   descriptions as evidence, no rankings. Those are other people's products, and saying so is part
   of standing tall.
