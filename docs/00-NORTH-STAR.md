# 00 — North star

**Status:** bootstrap authority · **Date:** 2026-08-12 · **Applies to:** all of `ethos-engine`

---

## 1. What this is

ethos-engine is an open, high-performance document parser that emits **evidence**: a versioned,
fingerprinted representation in which every node carries a locator back into the source bytes, every
declared capability is declared on the wire, and everything the parser could not do is stated
instead of guessed. It does not decide whether a document is good, whether a claim is true, or
whether a citation holds. Those are a separate product's job. Its output is designed to be checked
by something else — including by someone who does not trust it.

The engine answers **"what does this document contain, and exactly where?"** A separate verifier
answers **"did this AI claim actually come from this document?"** Together they answer the question
that matters. Apart, each is honest about what it does not know.

## 2. Forced product decisions

These are settled. Do not reopen them in a PR, an ADR, or a design discussion. Reversing one takes a
decision from the owner, recorded here first.

| # | Decision |
| --- | --- |
| 1 | **Product:** ethos-engine = DocuShell's open high-performance parser / evidence emitter. Citation verification (L3) is a separate verifier product ("Ethos" / Ethos-next). Together they answer: did this AI claim actually come from this document? |
| 2 | **Build order:** Freeze the verify contract (artifact + rules) first. Implement ethos-engine against that contract — not against today's Ethos crate layout. A faster/better verifier may be rebuilt later to consume the same contract. Do not invent verification rules ad hoc after the parser "feels done." |
| 3 | **Trust ladder:** Engine owns L0–L2 (registered / extracted / locatable). Verifier owns L3 (grounded). Never emit a single field that means "this document is good." |
| 4 | **Canonical emit:** `DocumentRepresentation v0` (DocuShell). Adapter: `ethos.grounding.v1`. `NativeLocator` required; geometry/`RenderedLocator` optional "for inspection"; typed absence over invention. |
| 5 | **Parsing optional forever:** BYO parsers remain first-class via a grounding-intake shape. Engine must not become the only path to verify. |
| 6 | **Honesty over bake-offs:** versioned contract, declared coordinates, integer quanta, capability declarations, fail-closed, no public confidence gate, derivation classes (`Extracted` / `Computed` / `Recognized` / `Proposed`). |
| 7 | **v0 happy path only:** classify → extract → ground → grounding-check. No OCR, no tables, no Markdown-as-evidence, no office, no MCP, no SDKs, no citation verification inside the engine. |
| 8 | **Classify:** LiteParse-shaped reason codes on two orthogonal axes (OCR-need vs layout-hard); boolean derived from reasons; no confidence float; three exit codes (simple / needs-attention / could-not-read). |
| 9 | **OSS stance:** ODL = tables/tags/XY-Cut (later); Anydoc = office IR + error taxonomy + mutation/fuzz (later); pdf-inspector = reference-only (rects/encoding/single-load/mcid ideas); LiteParse = classify/OCR-contract/forms/vectors/screenshots/`trailing_space_generated` ideas — not a dependency for grounded PDF. |
| 10 | **v1 table gate (document only):** the bar was ODL-local ~0.489 deterministic, never hybrid ~0.9×. **Amended by the owner, 2026-08-19: the chase is parked.** 64‰ is this engine on four tagged PDFs this repository owns; 0.489 is a published ODL-local score on *their* corpus — same unit, different exam. 0.489 is **not** a shipping precondition for v2 and gates no slice. The number stays on the record, the method stays in `table-gate-v1.md`, **fabrication 0 still binds**, and **v1 is not complete**. The chase resumes if and when this repository has a labelled set it owns and chooses to resume it. |
| 11 | **OCR (document only):** none in v0; PP-OCR ONNX deterministic lane + LiteParse-style HTTP contract at v2.1; confidence diagnostic only, never filter; Tesseract never default; VLM/`Proposed` at v3. |
| 12 | **Optional agents:** allowed later as assist emitting `Proposed` only; never overwrite `Extracted`; never same processor identity for draft + evidence (Workbench rule 7). Out of v0. |
| 13 | **LibreOffice→PDF office bridge:** forbidden (invents pagination). |
| 14 | **No AGPL.** PDFium caller-provided or explicitly ADR'd later; v0 prefers clean-room `lopdf` + vendored CMap data (not wrapping pdf-inspector). |

## 3. Trust ladder — who owns what

DocuShell's L0–L6 ladder (`docushell-repo/docs/WORKBENCH_ARCHITECTURE.md` II.1) places this engine
precisely.

| Level | Meaning | Owner | Notes |
| --- | --- | --- | --- |
| **L0 — Source registered** | Exact source bytes, identity, provenance recorded | **engine** | `source.sha256` + media type on every artifact |
| **L1 — Extracted** | A versioned processor/profile produced a representation with **declared capabilities** and an extraction-assurance state | **engine** | Capability declarations are the *achievement condition*, not polish. An artifact without them is not L1 |
| **L2 — Locatable** | Required evidence resolves to an inspectable native or approved rendered locator | **engine** | `NativeLocator` required on every node. Geometry optional |
| **L3 — Grounded** | Every required literal/source/freshness check passed for a submitted claim under a recorded profile | **verifier — never the engine** | The engine has no claim input and no verdict output |
| L4 — Semantically supported | — | elsewhere | Not this estate's problem in v0 |
| L5 — Policy approved | — | elsewhere | — |
| L6 — Executed | — | elsewhere | — |

**The collapse this engine must never perform:** emitting one field that means "this document is
good." Not a score, not a grade, not a boolean named `ok`. Every honest signal it emits is either a
count, a named reason, a typed absence, or a declared limitation.

## 4. Relationship to the Ethos repo

Three things are true at once, and confusing them is the most likely way this project goes wrong.

**Today's Ethos repo (`~/Desktop/Stuff/repo/ethos/`) is the verifier and the oracle.** ethos-engine
reuses its *wire contracts* (`ethos-grounding-source.schema.json` as an emit target,
`ethos-grounding-validation-report.schema.json` as a validation oracle), its *fixtures* as a
conformance corpus, and its *designs* — c14n v1, integer centipoint quanta, profile-as-identity, the
capability/fail-closed vocabulary. It reuses **none** of its crate tree.

**ethos-engine is a greenfield sibling, not a fork and not a wrapper.** It is a separate workspace
with its own MSRV (1.88, which `lopdf` 0.42 requires and Ethos's 1.87 pin cannot give). It is not a
rewrite of Ethos, does not replace Ethos, and does not verify.

**A future "Ethos-next" verifier may be rebuilt against the same frozen contract.** That is the
point of freezing the contract first (decision #2). If the verifier is rewritten for speed or
architecture, the engine does not change, because the engine was never built against the verifier's
internals. See `07-VERIFY-BOUNDARY.md`.

Never edit the Ethos repo from this project. Read it for contracts, fixtures, and oracle behaviour.

## 5. What "done" means

### The whole engine, v0 → v3

| Ver | Done when | One-line intent |
| --- | --- | --- |
| **v0** | Validator agrees byte-identically with `ethos grounding check` across all 15 fixtures | Honest PDF core: classify, position-aware runs, locators, capabilities, grounding emit |
| **v0.1** | An ungrounded claim exits 1 with a report; no silent skip | Verify by shelling out to the Ethos CLI, as a declared capability |
| **v1** | Fabrication rate **0** and an honest table number on the four-PDF set it owns — measured at **64‰**. The **> 0.489** chase is **parked** (decision #10). **Not complete** | The DocuShell replacement gate — tables, full element vocabulary, tagged PDF, forms, vectors |
| **v1.1** | A Markdown-quoted citation verifies end-to-end | Safe Markdown, only with the Anchor Map |
| **v1.2** | Locators survive every adapter round-trip | Adoption: MCP server, Python + Node SDKs, LangChain tool |
| **v2** | A DOCX quote and an XLSX cell both ground; no synthesised pages | Anydoc-class office formats through one shared IR |
| **v2.1** | An OCR'd document's fingerprint is provably incomparable with a born-digital parse | OCR lane under its own profile |
| **v2.2** | Only on a named accessibility requirement | Auto-tagging — a parallel lane, never on the critical path |
| **v3** | Byte-diff: assist on/off ⇒ identical grounded artifacts | Propose-only VLM assist |

Detail lives in `02-ROADMAP.md`. **Only v0 is specified for implementation** (`03-V0-SCOPE.md`,
`05-MILESTONES.md`). Everything past v0.1 is one line and stays one line until v0 ships.

### v0, specifically

`classify → extract → ground → grounding-check` runs on the Ethos conformance corpus, twice, and
produces byte-identical output both times; the `grounding-check` validator agrees with
`ethos grounding check` on `structure`, `source_binding`, `representation_sha256` and `counts` for
every fixture; every artifact declares its capabilities, its coordinate system, and its profile
hash; and no artifact anywhere contains a confidence float. Full criteria in `03-V0-SCOPE.md` §5.

## 6. Reading order

For a coding agent starting fresh:

1. **`00-NORTH-STAR.md`** (this file) — what the product is and what is already decided
2. **`01-CONTRACT.md`** — the artifact shape. Frozen first, by decision #2. Everything else serves it
3. **`03-V0-SCOPE.md`** — what is in and out of the first release
4. **`05-MILESTONES.md`** — the ordered work, with acceptance tests. **Start at M0**
5. **`04-ARCHITECTURE.md`** — crate layout, CLI surface, fixtures, dependency posture
6. **`07-VERIFY-BOUNDARY.md`** — read before touching anything that looks like verification
7. **`06-STEAL-REFUSE.md`** — read before proposing a feature borrowed from another parser
8. **`02-ROADMAP.md`** — only to check that a v1+ idea has a home and does not belong in v0

What 0.26.0 can and cannot do, on one page: `CAPABILITY.md`. The research archive that produced
these documents is **off-tree** and is not a second roadmap (`reference/README.md`);
`06-STEAL-REFUSE.md` is the living steal / refuse record. Architecture depth for the older
Ethos-in-DocuShell framing: `~/Desktop/Stuff/repo/ethos-docushell-parser-plan.md` — **external, not
in this tree, superseded wherever it conflicts with the documents above.**

## 7. Anti-goals

Written down so they can be pointed at in review.

- A public confidence float, score, grade, or any single field summarising document quality
- A benchmark claim of "fastest," "#1," or "best" — every competitor headline number in this
  landscape is publisher-owned and at least one is provably 34 points off depending on invocation
  flags. **0.489 included:** it is theirs, on their corpus, and this repository neither publishes it
  as its own nor now chases it (decision #10)
- Silent deletion: hidden text, headers, footers, small text, or low-confidence OCR removed without
  a record
- Any coordinate, identifier, fingerprint, or pagination the source did not contain
- A second citation-verification implementation, in any form, ever
- Becoming the only path to verification — BYO parsers stay first-class (decision #5)
