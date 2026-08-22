# 02 — Roadmap

**Status:** one page, deliberately. **Five versions are specified for implementation** — v0, v1,
v1.1, v1.2 and v2 — each with a scope document and a milestones document; the table below names
which. This line said *"Only v0 is specified for implementation"* from the bootstrap commit until
v2-S13.5, when it had been wrong for four scope documents and disagreed with this file's own
navigation table. **v2.2, v3 and v4 remain one line each**, which is what "one page, deliberately"
protects.
**Detail:** `03-V0-SCOPE.md` (what v0 is) and `05-MILESTONES.md` (how v0 gets built) remain the
worked pair; **§"Where the work is specified"** below maps the other four.

---

## Versions

Nine versions, one line each. **No version below v1 gets expanded until v0 ships**, and no new
version numbers get invented — every delta from the research folds into a row that already exists.

| Ver | Theme | Contents | Gate |
| --- | --- | --- | --- |
| **v0** | Honest PDF core | classify (reason codes on two orthogonal axes, counts, no confidence, three exit codes) · position-aware text runs · measured ink box or typed absence with declared semantics · `synthesized` flags · single-column order · format detection · error taxonomy · c14n/quanta/ids · capability declarations · `ethos.grounding.v1` · `grounding-check` · CLI + lib · fuzz + mutation tests | Validator agrees byte-identically with `ethos grounding check` on all 15 fixtures |
| **v0.1** | Verify + robustness — **shipped as 0.2.0** | shell out to the Ethos CLI as a declared capability · encoding-issue detection · xref repair-or-refuse | An ungrounded claim exits 1 with a report; no silent skip. **Met**: `engine verify --fail-on-ungrounded`, bytes relayed verbatim, verifier pinned in the profile |
| **v1** | **The DocuShell replacement gate** | tables (ruled + unruled) with locator cross-check · vector path data driving ruled detection · full element vocabulary incl. Header/Footer/Caption · multi-column with a stable rule · tagged-PDF consumption + `mcid` + structure tree · forms and annotations as typed, distinguishable nodes · DPI screenshots · security findings (hidden / off-page) · images · annotated PDF | Fabrication rate **0** · cross-check diagnostics emitted · an honest table number on the four-PDF set, measured at **64‰**. The **> 0.489** chase is **parked** — see below. **v1 is not complete** |
| **v1.1** | Safe Markdown | Markdown + **Anchor Map** · HTML · hyphenation / dot-leaders / drop-caps as export-only cosmetics | A Markdown-quoted citation verifies end-to-end; coverage completeness asserted |
| **v1.2** | Adoption | **MCP server** (first adapter) · Python + Node SDKs · LangChain tool · optional `liteparse → ethos.grounding.v1` adapter | Locators survive every adapter round-trip |
| **v2** | Anydoc-class formats | DOCX → XLSX → PPTX → ODT → ODS → ODP → RTF → EPUB → **CSV (S10)** · shared IR + one serializer · embedded assets **counted** (decision #17) | A DOCX quote and an XLSX cell both **bind** — each resolves to an address the file itself states; **no synthesised pages** (decision #16) |
| **v2.2** | Accessibility | auto-tag → Tagged PDF | Tagged output round-trips: a tag this engine writes is one it can read back and ground against. **Reordered ahead of assist and OCR by the owner, 2026-08-21** — see decision #15. The old condition — *"only on a named accessibility requirement, never on the critical path"* — is **withdrawn**; this is the next sequential row after v2 |
| **v3** | Assist | propose-only VLM · dual-read → review · hybrid enrichments (formula, chart) as `Recognized` / `Proposed` | Byte-diff: assist on/off ⇒ identical grounded artifacts |
| **v4** | OCR lane | PP-OCRv5/v6 ONNX in-process · LiteParse-compatible HTTP OCR contract · own profile · per-page routing · never overwrites `Extracted` · confidence never filtered on | OCR fingerprint provably incomparable with born-digital |

### The two gates worth memorising

**v1 tables: 0.489 is incomparable here, so do not chase it — and do not chase hybrid 0.9× either.**
**64‰** is this engine on **four tagged PDFs this repository owns**. **0.489** is a published
ODL-local table score on **their** corpus — the one figure ODL and pdf-inspector report
bit-identically, which is why it was picked. Same unit, different exam. The chase is **parked**
(`00-NORTH-STAR.md` #10) until this repository has a labelled set it owns and chooses to resume.
Hybrid ~0.9× was never the target and still is not: it is a non-deterministic mode, and a
determinism contract cannot sit under it. **Fabrication 0 still binds. v1 is not complete**, and
parking the chase is not a pass — `table-gate-v1.md` keeps the method and the miss.

**v0: byte-identical agreement with the Ethos CLI.** Not "close," not "equivalent modulo
formatting." Byte-identical on `structure`, `source_binding`, `representation_sha256`, `counts`,
across all 15 fixtures, as a CI job rather than a claim.

---

## Folded deltas

Research passes 3 and 4 produced changes that fold into the rows above. Recorded here so nobody
proposes a "v0.5" for them. The **Source** column names research row ids; the archive they came from
is off-tree (`reference/README.md`) and `06-STEAL-REFUSE.md` is the living record of the decisions
those rows became.

| Ver | Folded in | Source |
| --- | --- | --- |
| **v0** | Reason-code classifier on two orthogonal axes (OCR-need ∪ layout-hard) · boolean derived from reasons · **three distinct exit codes** (simple / needs-attention / could-not-read) · `synthesized` flags on characters the reader invented · `char_codes` with the ligature caveat · per-page counts, 1-indexed | memo §18.9, checklist L1–L7, L16 |
| **v0.1** | unchanged | — |
| **v1** | Forms/AcroForm and annotations as typed, **distinguishable** nodes (annotation text is never page text) · vector path data driving ruled-table detection · DPI screenshots · structure-tree extraction | memo §18.9, checklist L10–L14 |
| **v1.1** | unchanged | — |
| **v1.2** | unchanged | — |
| **v2** | Anydoc-native office parsers, **explicitly not** a LibreOffice→PDF bridge | memo §18.10 Q4 |
| **v2.2** | unchanged | — |
| **v3** | unchanged | — |
| **v4** | The LiteParse HTTP OCR contract (`POST /ocr`, multipart `file`+`language`, `{results:[{text, bbox, polygon?}]}`) **alongside** the in-process PP-OCR lane, with `confidence` dropped from the contract acted on | memo §18.8, checklist L9 |

---

## Not scheduled

Not "later." Not on the roadmap at all. Adding one requires a decision-log entry that reverses a
forced decision in `00-NORTH-STAR.md` §2.

| Item | Why not |
| --- | --- |
| PDF/UA export, accessibility studio | A different product for a different buyer. Tag *consumption* (v1) improves grounding; tag *generation* (v2.2) does not — which is why v2.2 writes tags and stops there rather than growing into a studio |
| Chart descriptions **as evidence** | A description is `Proposed`. It can exist and can never be cited |
| `--sanitize` or any mode that rewrites the evidence | The artifact is the record |
| A public confidence float, at any version | Workbench rule 9. See `01-CONTRACT.md` §9 |
| A JVM runtime dependency | ODL's cost of entry, and the reason its capabilities cannot be borrowed wholesale |
| LibreOffice → PDF office bridge | **It invents pagination.** Pagination does not exist in a DOCX and must not be synthesised |
| Wrapping pdf-inspector, LiteParse, or any competitor as the grounded PDF core | Reference-only. §`06-STEAL-REFUSE.md` |
| A second verification implementation | `07-VERIFY-BOUNDARY.md`. Linking the same verifier is not a second integration; reimplementing its semantics is |
| Any AGPL dependency | Forced decision #14 |
| RAGFlow-class retrieval platforms in the trust core | Rejected on structure, not licence — DeepDoc cannot emit `TableCellPosition` with row/col spans, so the cross-check cannot run. Seven written reversal triggers exist (memo §16.8); absent all seven, no |
| Rankings, "#1", "fastest", or any competitor bake-off table | Every headline number in this landscape is publisher-owned, and one is provably 34 points off depending on invocation flags. **0.489 is one of them** — never published as this engine's score |

---

## Where the work is specified

| Question | Document |
| --- | --- |
| What is v0, exactly? | `03-V0-SCOPE.md` |
| How does v0 get built, in what order? | `05-MILESTONES.md` — **start at M0** |
| What shape must every artifact have? | `01-CONTRACT.md` |
| Can I borrow feature X from parser Y? | `06-STEAL-REFUSE.md` |
| Where does verification live? | `07-VERIFY-BOUNDARY.md` |
| What is v1, and did its gate clear? | `08-V1-SCOPE.md` / `09-V1-MILESTONES.md` — **measured at 64‰, a miss; the 0.489 chase is parked** |
| What is v1.1 (Safe Markdown)? | `10-V11-SCOPE.md` / `11-V11-MILESTONES.md` — complete |
| What is v1.2 (adoption)? | `12-V12-SCOPE.md` / `13-V12-MILESTONES.md` — complete |
| What is v2 (office formats)? | `14-V2-SCOPE.md` / `15-V2-MILESTONES.md` — **S0 through S16 done, at 0.34.1**: the mutation lane at 0.32.0, guards at 0.32.1, the roadmap reorder at 0.32.2, prose at 0.32.3, the owner's two gate decisions at 0.32.4, the two unrun sweeps at 0.32.5, the CRC-32 answer at 0.33.0, the guards those sweeps named at 0.33.1, the `neither detector` cluster at 0.34.0 and the no-behaviour-change extractor itself at 0.34.1. S10 is CSV as an argued refusal, not a reader. **v2's format row is closed and the gate is MET** (decisions #16, #17); the last question — `zip.rs`'s CRC-32 — was answered at **S14, as 0.33.0**, measured before it shipped |

Each row above is a **scope** document plus a **milestones** document, on the pattern `03`/`05` set
for v0. A version gets that pair before it gets code — v2 had both while having none, and now has
`engine-office` reading eight formats under them. Nothing past
v2 has an implementation document, and v2.2, v3 and v4 should not get one until v2's gate is in
sight.
