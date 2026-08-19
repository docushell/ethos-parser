# 02 — Roadmap

**Status:** one page, deliberately. Only v0 is specified for implementation.
**Detail:** `03-V0-SCOPE.md` (what v0 is) and `05-MILESTONES.md` (how v0 gets built).

---

## Versions

Nine versions, one line each. **No version below v1 gets expanded until v0 ships**, and no new
version numbers get invented — every delta from the research folds into a row that already exists.

| Ver | Theme | Contents | Gate |
| --- | --- | --- | --- |
| **v0** | Honest PDF core | classify (reason codes on two orthogonal axes, counts, no confidence, three exit codes) · position-aware text runs · measured ink box or typed absence with declared semantics · `synthesized` flags · single-column order · format detection · error taxonomy · c14n/quanta/ids · capability declarations · `ethos.grounding.v1` · `grounding-check` · CLI + lib · fuzz + mutation tests | Validator agrees byte-identically with `ethos grounding check` on all 15 fixtures |
| **v0.1** | Verify + robustness — **shipped as 0.2.0** | shell out to the Ethos CLI as a declared capability · encoding-issue detection · xref repair-or-refuse | An ungrounded claim exits 1 with a report; no silent skip. **Met**: `engine verify --fail-on-ungrounded`, bytes relayed verbatim, verifier pinned in the profile |
| **v1** | **The DocuShell replacement gate** | tables (ruled + unruled) with locator cross-check · vector path data driving ruled detection · full element vocabulary incl. Header/Footer/Caption · multi-column with a stable rule · tagged-PDF consumption + `mcid` + structure tree · forms and annotations as typed, distinguishable nodes · DPI screenshots · security findings (hidden / off-page) · images · annotated PDF | Table-cell accuracy **> 0.489** on a labelled set · fabrication rate 0 · cross-check diagnostics emitted |
| **v1.1** | Safe Markdown | Markdown + **Anchor Map** · HTML · hyphenation / dot-leaders / drop-caps as export-only cosmetics | A Markdown-quoted citation verifies end-to-end; coverage completeness asserted |
| **v1.2** | Adoption | **MCP server** (first adapter) · Python + Node SDKs · LangChain tool · optional `liteparse → ethos.grounding.v1` adapter | Locators survive every adapter round-trip |
| **v2** | Anydoc-class formats | DOCX → XLSX → PPTX → ODF/RTF/EPUB/CSV · shared IR + one serializer · embedded assets | A DOCX quote and an XLSX cell both ground; **no synthesised pages** |
| **v2.1** | OCR lane | PP-OCRv5/v6 ONNX in-process · LiteParse-compatible HTTP OCR contract · own profile · per-page routing · never overwrites `Extracted` · confidence never filtered on | OCR fingerprint provably incomparable with born-digital |
| **v2.2** | Accessibility (**parallel lane**) | auto-tag → Tagged PDF | Only on a named accessibility requirement. Never on the critical path |
| **v3** | Assist | propose-only VLM · dual-read → review · hybrid enrichments (formula, chart) as `Recognized` / `Proposed` | Byte-diff: assist on/off ⇒ identical grounded artifacts |

### The two gates worth memorising

**v1 tables: beat 0.489, not 0.9×.** ODL's README leads with 0.907 overall; that is its **hybrid
AI-backed** mode. Its deterministic local mode scores **0.831 overall / 0.489 table**, and that
0.489 is the one figure ODL and pdf-inspector independently report **bit-identically** on the same
corpus. Beating ODL-local deterministically is a realistic v1 target. Matching ODL-hybrid is a
different, non-deterministic product, and where hybrid is better the honest move is to say so rather
than to chase it. See `reference/ethos-engine-parity-checklist.md` §0.1.

**v0: byte-identical agreement with the Ethos CLI.** Not "close," not "equivalent modulo
formatting." Byte-identical on `structure`, `source_binding`, `representation_sha256`, `counts`,
across all 15 fixtures, as a CI job rather than a claim.

---

## Folded deltas

Research passes 3 and 4 produced changes that fold into the rows above. Recorded here so nobody
proposes a "v0.5" for them.

| Ver | Folded in | Source |
| --- | --- | --- |
| **v0** | Reason-code classifier on two orthogonal axes (OCR-need ∪ layout-hard) · boolean derived from reasons · **three distinct exit codes** (simple / needs-attention / could-not-read) · `synthesized` flags on characters the reader invented · `char_codes` with the ligature caveat · per-page counts, 1-indexed | memo §18.9, checklist L1–L7, L16 |
| **v0.1** | unchanged | — |
| **v1** | Forms/AcroForm and annotations as typed, **distinguishable** nodes (annotation text is never page text) · vector path data driving ruled-table detection · DPI screenshots · structure-tree extraction | memo §18.9, checklist L10–L14 |
| **v1.1** | unchanged | — |
| **v1.2** | unchanged | — |
| **v2** | Anydoc-native office parsers, **explicitly not** a LibreOffice→PDF bridge | memo §18.10 Q4 |
| **v2.1** | The LiteParse HTTP OCR contract (`POST /ocr`, multipart `file`+`language`, `{results:[{text, bbox, polygon?}]}`) **alongside** the in-process PP-OCR lane, with `confidence` dropped from the contract acted on | memo §18.8, checklist L9 |
| **v2.2** | unchanged | — |
| **v3** | unchanged | — |

---

## Not scheduled

Not "later." Not on the roadmap at all. Adding one requires a decision-log entry that reverses a
forced decision in `00-NORTH-STAR.md` §2.

| Item | Why not |
| --- | --- |
| PDF/UA export, accessibility studio | A different product for a different buyer. Tag *consumption* (v1) improves grounding; tag *generation* (v2.2, gated) does not |
| Chart descriptions **as evidence** | A description is `Proposed`. It can exist and can never be cited |
| `--sanitize` or any mode that rewrites the evidence | The artifact is the record |
| A public confidence float, at any version | Workbench rule 9. See `01-CONTRACT.md` §9 |
| A JVM runtime dependency | ODL's cost of entry, and the reason its capabilities cannot be borrowed wholesale |
| LibreOffice → PDF office bridge | **It invents pagination.** Pagination does not exist in a DOCX and must not be synthesised |
| Wrapping pdf-inspector, LiteParse, or any competitor as the grounded PDF core | Reference-only. §`06-STEAL-REFUSE.md` |
| A second verification implementation | `07-VERIFY-BOUNDARY.md`. Linking the same verifier is not a second integration; reimplementing its semantics is |
| Any AGPL dependency | Forced decision #14 |
| RAGFlow-class retrieval platforms in the trust core | Rejected on structure, not licence — DeepDoc cannot emit `TableCellPosition` with row/col spans, so the cross-check cannot run. Seven written reversal triggers exist (memo §16.8); absent all seven, no |
| Rankings, "#1", "fastest", or any competitor bake-off table | Every headline number in this landscape is publisher-owned, and one is provably 34 points off depending on invocation flags (memo §18.4) |

---

## Where the work is specified

| Question | Document |
| --- | --- |
| What is v0, exactly? | `03-V0-SCOPE.md` |
| How does v0 get built, in what order? | `05-MILESTONES.md` — **start at M0** |
| What shape must every artifact have? | `01-CONTRACT.md` |
| Can I borrow feature X from parser Y? | `06-STEAL-REFUSE.md` |
| Where does verification live? | `07-VERIFY-BOUNDARY.md` |
| What is v1, and did its gate clear? | `08-V1-SCOPE.md` / `09-V1-MILESTONES.md` — **the gate is missed at 64‰** |
| What is v1.1 (Safe Markdown)? | `10-V11-SCOPE.md` / `11-V11-MILESTONES.md` — complete |
| What is v1.2 (adoption)? | `12-V12-SCOPE.md` / `13-V12-MILESTONES.md` — complete |
| What is v2 (office formats)? | `14-V2-SCOPE.md` / `15-V2-MILESTONES.md` — **S0–S5 done at 0.24.0; S6 not started** |

Each row above is a **scope** document plus a **milestones** document, on the pattern `03`/`05` set
for v0. A version gets that pair before it gets code — v2 had both while having none, and now has
`engine-office` reading two formats under them. Nothing past
v2 has an implementation document, and v2.1, v2.2 and v3 should not get one until v2's gate is in
sight.
