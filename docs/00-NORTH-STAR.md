# 00 — North star

**Status:** bootstrap authority · **Applies to:** all of `ethos-parser`

---

## 1. What this is

ethos-parser is a document parser that emits **evidence**: a versioned, fingerprinted record where
every node points back into the source bytes, every capability is declared on the wire, and
everything the parser could not do is stated rather than guessed.

It does not decide whether a document is good, whether a claim is true, or whether a citation holds.
Those belong to a separate product. Its output is built to be checked by something else — including
by someone who does not trust it.

The engine answers *"what does this document contain, and exactly where?"* A separate verifier
answers *"did this AI claim actually come from this document?"* Together they answer the question
that matters. Apart, each is honest about what it does not know.

## 2. Decisions already made

These are settled. Do not reopen them in a PR, an ADR, or a design discussion. Reversing one takes a
decision from the owner, recorded here first.

Row 18 was decided on 2026-08-30 and v1 is closed on it; the row records the decision and the
evidence behind it. Row 19, the same day, gives geometric block structure a home and is the first
row that *widens* what the engine emits rather than settling what it already did. Row 20 is the
first that refuses a measured performance win on honesty grounds, and it names the price.

| # | Decision |
| --- | --- |
| 1 | **The product.** ethos-parser is DocuShell's open parser and evidence emitter. Citation verification is a separate verifier product. |
| 2 | **Build order.** Freeze the verify contract first, then build the parser against it — never against the verifier's current crate layout, and never by inventing verification rules once the parser "feels done". |
| 3 | **Trust ladder.** The engine owns L0–L2 (registered, extracted, locatable). The verifier owns L3 (grounded). Never emit a field meaning "this document is good". |
| 4 | **What it emits.** `DocumentRepresentation v0`, with `ethos.grounding.v1` as the adapter. A native locator is required on every node; geometry is optional; typed absence beats invention. |
| 5 | **Parsing stays optional, forever.** Bring-your-own parsers remain first class. This engine must never become the only path to verification. |
| 6 | **Honesty over bake-offs.** Versioned contract, declared coordinates, integer quanta, capability declarations, fail-closed behaviour, no public confidence gate, explicit derivation classes. |
| 7 | **v0 is the happy path only:** classify → extract → ground → grounding-check. No OCR, tables, Markdown, office formats, MCP, SDKs or verification. |
| 8 | **Classify** uses reason codes on two independent axes (does it need OCR, is the layout hard), a boolean derived from those reasons, no confidence float, and three exit codes. |
| 9 | **Open-source stance.** Ideas are borrowed, code is not: tables and tags from OpenDataLoader, the office IR and error taxonomy from Anydoc, rectangle and encoding handling from pdf-inspector, classify and forms ideas from LiteParse. None is a dependency for the grounded PDF core. |
| 10 | **The v1 table bar, amended 2026-08-19: the chase is parked.** The original bar was a published 0.489 score. That number is somebody else's, on their corpus — same unit, different exam. It gates no slice and is not a shipping precondition. What still binds: **fabrication stays 0**, the method stays in `table-gate-v1.md`. The chase resumes only if this repository acquires a labelled set it owns and chooses to resume it.<br><br>**Superseded on v1's status by row 18** (2026-08-30), which closed v1 on a capability statement and a band. This row said *"v1 is not complete"* and is left standing as the record of the amendment that parked the chase — but **row 18 is where v1's status is stated**, and a second copy of it here is what let five documents drift. |
| 11 | **OCR** is out until v4: a deterministic ONNX lane under its own profile, with any confidence value kept diagnostic and never used as a filter. Tesseract is never the default. |
| 12 | **Optional agents** may assist later, emitting `Proposed` only. They never overwrite `Extracted`, and never share a processor identity with the evidence path. |
| 13 | **Converting office files to PDF is forbidden.** It invents pagination. |
| 14 | **No AGPL.** PDFium only if caller-provided under an explicit ADR. The core is clean-room `lopdf` plus vendored specification data. |
| 15 | **Roadmap order, amended 2026-08-21:** after v2 comes **v2.2 (accessibility) → v3 (assist) → v4 (OCR)**. OCR moved last and was renumbered rather than resequenced, because `parser_version` sits inside `profile_sha256` and a later build carrying a lower number defeats the only job that field has. v2.1 is now a gap and nothing ever shipped under it. |
| 16 | **What v2's gate means, settled 2026-08-21: *ground* means *bind*.** A DOCX quote and an XLSX cell each resolve to an address the file itself states. It does not mean emitting the paginated `ethos.grounding.v1`, which a DOCX cannot satisfy — and the gate's own second clause, *no synthesised pages*, forbids what the literal reading would require. Widening the schema stayed blocked on an Ethos-side revision rather than refused; that revision landed as 1.1.0, and 0.39.0 took it. Office representations now project the page-less shape, with no synthesised page anywhere. |
| 17 | **"Embedded assets", settled 2026-08-21: counting them satisfies v2.** Every reader declares how many entries it passed over that hold a picture, a clip or an embedded object. *Reading* one is explicitly not v2 — an office image has no page and no coordinate system, so a node for it is a contract change rather than a reader change. |
| 18 | **DECIDED 2026-08-30. v1 closes on a capability statement and a band, not on a macro.** What v1 publishes is four clauses: the engine **reads the tables a document declares** in its structure tree, **detects ruled tables** where the producer drew the rules, **emits nothing** where neither holds, and **fabricates nothing**.<br><br>**The numbers that go with them**, on twelve tagged, public, redistributable documents — 2,068 pages, 172 tagged tables, 15,755 tagged cell slots. `tagged-tables-v1` recovers **157 of the 172** gold tables and takes combined cell-slot recall to **502‰**. The geometric detectors alone score **70‰** macro cell-slot F1, and the band is published with it and never without it: **0‰–590‰, median 0‰, ten of the twelve exactly zero**, two documents supplying the whole average. Micro recall for the geometric rules alone is **4‰**. Detection precision after v2-S20 is **941‰**, with cross-check disagreements at 0. **Fabrication is 0 on all twelve.**<br><br>**Why a band and not an average.** A single macro over mostly zeros reads as *"the engine gets 7% of table cells right"* when the truth is *"most of two documents, nothing on ten"*. The band says the real shape — bimodal — which is the finding twelve documents established that four could not.<br><br>**What stays parked.** The geometric chase. v2-S20 spent the one concrete lead this row offered and it moved precision without moving the macro; reopening means a genuinely new geometric idea, not more effort on the current one. The 0.489 comparator is somebody else's score on their own corpus and is not chased. **What stays binding:** fabrication stays 0, the method stays in `table-gate-v1.md`, and table numbers are quoted from there or not at all. |
| 19 | **DECIDED 2026-08-30. Geometric block structure is the first half of v2.2, and v2.2 is renamed *layout and accessibility*.** The reading-order cut already divides a page into column bands and then discards the division, keeping only the permutation — so an untagged document projects as one flat run of paragraphs, and the engine's own measurement of the page never reaches the wire. D4 emits it: **one optional `region` per node, 1-based, absent where the cut made no division.**<br><br>**Why v2.2 and not a new row.** You cannot write a tag for an untagged document without first deciding where its blocks are. Auto-tagging is the second half of a job whose first half was never scoped, which is why v2.2's gate — *a tag this engine writes is one it can read back and ground against* — had no route to it. **No version number is invented**, which is what [`02-ROADMAP.md`](02-ROADMAP.md) asks for, and v2 being met is what puts its successor in sight.<br><br>**The line this row does not cross.** A region says *where text sits*; it never says what text is. Roles keep coming from the structure tree or from nowhere, and a region is never a heading, a paragraph, a section or a column name — that is **P14**, and it stays refused. Layout is `Computed` from whitespace this engine measured; structure stays `Extracted` from what the document declared.<br><br>**What it costs, accepted with the decision:** the rule id moves to `gutter-columns-v2`, so `profile_sha256` moves and every golden regenerates. Contract §2 requires it — *anything that can change a byte of output belongs in the profile, or it is a bug*.<br><br>**What is refused alongside it.** D1, declared document splits, on measurement rather than principle: across all 45 PDF fixtures `/Collection`, `/EmbeddedFiles`, `/PageLabels`, `/Part` and `/DocumentFragment` occur **0** times, and all eight gate documents declare exactly one top-level element. [`17-D1-SCOPE.md`](17-D1-SCOPE.md) carries the measurement and both reopening conditions. |
| 20 | **DECIDED 2026-08-30. §3's *a constant today is a discriminator tomorrow* binds generally, not only to `coordinate_system`.** A per-node value the contract requires the artifact to **state** is spelled out on the wire, even where it is constant across every node of every document measured so far. Omitting it and restoring a default at read time is refused.<br><br>**What this costs, measured before deciding:** four fields are constant-valued and together are **18.30%** of a `nist-sp-800-218` artifact and **11.62%** of `irs-fw9` — `scalar_code_mismatch` (5.96%), `derivation` (5.15%), `kind` (3.70%), `synthesized` (3.49%). Run time is linear in emitted bytes, so that is throughput too. The bytes are refused anyway.<br><br>**Why, on `derivation`, which is the case that decides the rest.** `Node` is `deny_unknown_fields` and `derivation` carries no `serde(default)`, so a missing key is a **hard parse error** today. Adding `default` beside `skip_serializing_if` would build, verbatim, the field [`01-CONTRACT.md`](01-CONTRACT.md) §6 names as somebody else's bug — *discriminated only by an omittable nullable field* — and would default it to the **highest-trust** class. Any intermediary stripping a key it does not recognise would then launder a `Recognized` or `Proposed` node into `Extracted`, the artifact would stay well-formed, and `DerivationClass::may_be_overwritten_by` returns **false for every pair** whose left side is `Extracted` — so the laundered node is not merely mislabelled, it is **uncorrectable**.<br><br>**What is NOT refused by this row.** Encodings that keep the value *inside the artifact* and resolvable from it alone — the value is still stated, by reference rather than by repetition. That is a different question from omission and it is answered on its own evidence, per encoding.<br><br>**What would reopen it:** a contract-level rule, on the wire rather than in a doc comment or a draft schema, defining how a consumer derives the omitted value — including its units. `scalar_code_mismatch` is the live example: it is derivable as `text.chars().count() != char_codes.len()`, that definition exists only in a Rust doc comment and an unvalidated draft, and a JavaScript consumer computing `text.length` counts UTF-16 code units and silently disagrees on any astral character. |

## 3. Trust ladder — who owns what

| Level | Meaning | Owner |
| --- | --- | --- |
| **L0 — Source registered** | The exact source bytes, identity and provenance are recorded | **engine** |
| **L1 — Extracted** | A versioned processor produced a representation with declared capabilities and an assurance state | **engine** |
| **L2 — Locatable** | Every piece of required evidence resolves to an inspectable locator | **engine** |
| **L3 — Grounded** | Every required check passed for a submitted claim under a recorded profile | **verifier — never the engine** |
| L4–L6 | Semantic support, policy approval, execution | elsewhere |

Capability declarations are the *achievement condition* for L1, not polish: an artifact without them
is not L1. A native locator is required on every node; geometry is optional.

**The collapse this engine must never perform** is emitting one field that means "this document is
good" — not a score, not a grade, not a boolean named `ok`. Every honest signal it emits is a count,
a named reason, a typed absence, or a declared limitation.

## 4. Relationship to the Ethos repo

Three things are true at once, and confusing them is the likeliest way this project goes wrong.

**The Ethos repo is the verifier and the oracle.** This engine reuses its wire contracts as an emit
target and a validation oracle, its fixtures as a conformance corpus, and its designs — canonical
JSON, integer centipoint quanta, profile-as-identity, the capability and fail-closed vocabulary. It
reuses **none** of its crate tree.

**ethos-parser is a greenfield sibling, not a fork and not a wrapper.** Separate workspace, its own
MSRV. It does not replace Ethos and does not verify.

**A future verifier may be rebuilt against the same frozen contract.** That is the whole point of
freezing the contract first: if the verifier is rewritten, this engine does not change, because it
was never built against the verifier's internals.

Never edit the Ethos repo from this project. Read it for contracts, fixtures and oracle behaviour.

## 5. What "done" means

| Ver | Done when | Intent |
| --- | --- | --- |
| **v0** | The validator agrees byte-identically with `ethos grounding check` across all 15 fixtures | An honest PDF core |
| **v0.1** | An ungrounded claim exits 1 with a report, and nothing is ever silently skipped | Verification by shelling out, as a declared capability |
| **v1** | Fabrication is 0 and the table result is published as a capability plus a band on the set this repo owns. **Closed** on decision #18 | The DocuShell replacement gate |
| **v1.1** | A Markdown-quoted citation verifies end to end | Safe Markdown, only with the anchor map |
| **v1.2** | Locators survive every adapter round trip | MCP server, Python and Node SDKs, LangChain tools |
| **v2** | A DOCX quote and an XLSX cell both bind to an address the file states, with no synthesised pages | Office formats through one shared record |
| **v2.2** | Both: a region is emitted wherever the cut divided a page and nowhere else, **and** a tag this engine writes is one it can read back and ground against | Layout and accessibility — geometric block structure, then auto-tagging (decision #19) |
| **v3** | Assist on and assist off produce identical grounded artifacts | Propose-only VLM assist |
| **v4** | An OCR'd document's fingerprint is provably incomparable with a born-digital parse | OCR under its own profile |

Five versions have full scope and milestone documents: v0, v1, v1.1, v1.2 and v2, and **v2.2's first
half has a scope document** — [`16-D4-SCOPE.md`](16-D4-SCOPE.md), by decision #19. Its second half,
v3 and v4 are still one line each, deliberately.

## 6. Reading order

1. **`00-NORTH-STAR.md`** (this file) — the product, and what is already decided.
2. **`01-CONTRACT.md`** — the artifact shape. Frozen first, by decision #2.
3. **`03-V0-SCOPE.md`** — what was in and out of the first release.
4. **`05-MILESTONES.md`** — the ordered work, with acceptance tests.
5. **`04-ARCHITECTURE.md`** — crate layout, CLI surface, fixtures, dependencies.
6. **`07-VERIFY-BOUNDARY.md`** — before touching anything verification-shaped.
7. **`06-STEAL-REFUSE.md`** — before borrowing a feature from another parser.
8. **`02-ROADMAP.md`** — to check that a later idea has a home.

[`CAPABILITY.md`](CAPABILITY.md) says what this build can and cannot do today.

## 7. Anti-goals

Written down so they can be pointed at in review.

- A public confidence float, score, grade, or any field summarising document quality.
- A "fastest" or "best" benchmark claim. Every competitor headline in this space is publisher-owned,
  and at least one is provably 34 points off depending on invocation flags — 0.489 included.
- Silent deletion: hidden text, headers, footers or small text removed without a record.
- Any coordinate, identifier, fingerprint or page number the source did not contain.
- A second citation-verification implementation, in any form, ever.
- Becoming the only path to verification.
