# 06 — Steal / refuse

**This file is the authority.** The research checklist it was extracted from is off-tree, so nothing
below defers to it. Row ids (`O5`, `A14`, `L30`, `P6`, …) stay citable as the names of the decisions
recorded here.

**Read this before proposing a feature borrowed from another parser.** It exists to prevent one
specific failure: a good-faith PR that imports a competitor's capability along with the bug that
makes it dishonest.

---

## The formula

Take **the table model, tagged-PDF consumption and XY-cut reading order** from OpenDataLoader. Take
**the shared multi-format record, the merged-cell model, the error taxonomy, content-based format
detection, and mutation-plus-fuzz testing** from Anydoc. Take **rectangle-based table detection,
encoding-issue detection, the one-document-load rule and the marked-content bridge** from
pdf-inspector. Take **the reason-code classifier on two axes, the open HTTP OCR contract, forms and
vector extraction, and the synthesized-space flag** from LiteParse.

Then add what none of them has: **a versioned output contract with a fingerprint, declared coordinate
systems, integer quanta, typed absence, derivation classes, a geometric-versus-structural
cross-check, and citation verification.**

That last sentence is the product. The first four are how it gets built without re-deriving twenty
years of PDF handling.

Take from **PageIndex** almost nothing — and that is the finding, not an omission. See below.

## The fifth source is a consumer, not a peer

**[VectifyAI/PageIndex](https://github.com/VectifyAI/PageIndex)** (MIT) was read against this engine on
2026-09-22 at `9c4c3ff`. It is the first surveyed project that is **not a parser**. The four above
answer *what does this document contain*; PageIndex answers *which part of it should I read*, and
stacks two layers to do it: `pageindex/flash/**`, a deterministic LLM-free PDF layout-to-outline
extractor, and above it a reasoning retrieval layer with tree search, citations and MCP.

That makes it the first evidence in this survey of **what a retriever actually asks a parser for** —
which is why its rows below are mostly `REFUSE`. A consumer's conveniences are exactly the places a
producer is tempted to guess, and PageIndex guesses in eleven documented ways. The rows are worth having
anyway: until now several of this repository's rules argued only from its own documents, and a rule
with an external, code-level case study behind it is harder to talk someone out of.

**What it gets right, and this engine does not.** Its `embedded_toc.py` reads the document's own
`/Outlines` and **grades how far that declaration can be trusted** — FULL, SKELETON or IGNORE, on
measured properties — then states its residual uncertainty in this repository's own idiom: *"a
bookmark target carries no reliable on-page position, so a section runs onto the page where the next
entry starts (the boundary page is shared; slack, never truncation)."* This engine reads
`/StructTreeRoot` and has never opened `/Outlines`. That is `PI-A` below, and it is the only
capability in the fifth source worth wanting.

**A model's routing problem is a real cost of this engine's refusals**, and `PI7` names it rather than
pretending otherwise: with no outline on the wire, a model using this engine has no cheap way to
choose where to look, so it is pushed toward reading everything.

## Decision vocabulary

| Decision | Meaning |
| --- | --- |
| **TAKE** | Build it essentially as the source does. Idea and execution both sound |
| **IMPROVE** | Build it, but change the mechanism. The goal is right; the execution is dishonest, lossy, or cliff-shaped |
| **REFUSE** | Do not build it, or not this way. It breaks a rule this project exists to keep |
| **DEFER** | Right idea, wrong time. Named version, no work before it |

---

## REFUSE — a PR doing this is rejected on principle, not on quality

| # | Capability | Why |
| --- | --- | --- |
| P3 | A public confidence float | Self-contradicting in practice: one parser reports TEXT-BASED at 50% with zero text pages |
| L23 | Confidence used as a filter | One parser drops text below 0.3, then again below 0.1, silently, in two different places |
| L18 | Loose char boxes sold as precise positioning | Em boxes, ascent to descent, not ink — a line of `acme` boxed as tall as `Ãj`, with nothing in the output saying which it is |
| L22 | Floats on the wire | Lossy round trip, no fixed precision |
| L21 | An undeclared coordinate space | Documented only in source comments. An artifact a reader cannot interpret without the source file is not evidence |
| L20 | Shipping without a versioned output contract | This is the moat. No competitor has started it |
| L30 | **Converting office files to PDF** | **It invents pagination.** Pagination does not exist in a DOCX and must not be synthesised. A refusal, not a fallback |
| L24 / L25 | OCR merged into the native text stream; OCR on by default | Discriminated only by an omittable nullable field. OCR is opt-in, profile-isolated, and authors only where no text layer exists |
| O21 / O22 / L27 / L28 | Hidden text, headers, footers or small text dropped by default | One project's README says *filters* while the code **deletes**. Take the threat model, invert the mechanism: **report, never delete** |
| O23 | A mode that rewrites the evidence | The artifact is the record |
| L13 | Undeclared form "repair" and always-on widget flattening | A repair that is not recorded is a fabrication |
| P14 | Style and role inferred from font names or text prefixes | Presentation is not structure. **Narrowed 2026-09-17** by [North Star decision #29](00-NORTH-STAR.md), for headings alone: a heading may be inferred from font size or font name where a document declares no structure, as `Computed`. Every other role stays refused |
| O26 | "#1", "fastest", or any bake-off claim | Every headline in this landscape is publisher-owned, and one is provably 34 points off depending on invocation flags. **The discriminator is the comparator, not the number** (sharpened against PageIndex, 2026-09-22): it publishes cost and accuracy curves of its own system under declared config, corpus and invocation, and those are honest; the same README's FinanceBench headline compares to an unnamed, unconfigured "Vector RAG", and **that** is what this row refuses. Measuring yourself in public is not the sin. Choosing your rival's invocation is |
| O28 | A JVM runtime dependency | One project's cost of entry, and why its capabilities cannot be borrowed wholesale |
| L31 | An unpinned build-time renderer download | Vendor fork, by tag, no checksum, so the build is network-dependent by default |
| L19 | Sourcing character origins from one competitor's design | **Structural:** the character-origin call has zero call sites there. The fingerprint-critical primitive is not bound at all, so that design cannot be the grounded PDF core |
| — | Wrapping any competitor as the grounded PDF core | Reference only — see below |
| — | A `liteparse` → `ethos.grounding.v1` adapter | **Measured and refused at v1.2-S5** — see below |
| **PI1** | **A node's text substituted from a navigation label** | PageIndex overwrites an extracted heading with the `/Outlines` string when the two are fuzzy-similar (`embedded_toc.py:322`), keeps the extracted node's page index, and records nothing — so a node can claim `start_index: N` carrying a title that appears nowhere on page N. Breaks contract §6 rule 1: *"Nothing overwrites `Extracted`. Not merged, not preferred, not reconciled."* If the two disagree that is a fact to count, not a defect to repair. **Load-bearing if `PI-A` ships** |
| **PI2** | **Provenance carried in an identifier instead of a field** | The pipeline mints `0266.1` for an LLM-expanded node, calls that provenance in its own docstring (`tree_optimize.py:207-224`), then relabels it flat and drops the per-node source log (`flash/api.py:95-99`). **Not** a refusal of generated content beside measured content — North Star #12 and contract §6 rules 2–3 permit exactly that. What is refused is the mechanism: **a derivation class is a field on the node, never an id convention, because an id convention is the first thing a later pass normalises away** |
| **PI3** | **A line-number rail deleted from the text** | `strip_line_numbers` (`phases/line_numbers.py:105-162`) runs before stats or clustering, removes the first span of every line in a detected margin rail, and drops the line entirely when it empties — no count, no flag. **A fifth kind for the O21/O22/L27/L28 row**, and worse than the header case: the deleted span is the leading token of a body line, so a caller comparing against the rendered page has no cue. The hazard is real and already handled here without deleting — `docs/measurements/block-subdivision/labelled.py:22-26` calls it the *digit-line drop* |
| **PI4** | **A mined phrase list used as a classification input** | `flash/data/boilerplate_phrases.json` is 5 904 unversioned, unattributed strings compiled into a case-folded trie; a match silently disqualifies a block from being body text. **At least eleven entries are not phrases at all** — ten runs of mis-decoded halfwidth katakana, the first five entries of the file among them, plus an HTML-escaped Office conditional comment. The admissible shape, so this is not read as banning all data: a normative table from a published specification (P9) is *data*; a list of what documents in someone's corpus tended to say is a *judgement* |
| **PI5** | **A second text source reconciled by ordinal alignment** | Text from PDFium, re-decoded from its own content-stream walk, patched one over the other on a list-length equality (`unicode_apply.py:66`) with silence as the fallback. That is the opposite of E6, which refuses the output when two sources disagree and names the disagreement on the wire. **The refusal is the reconciliation mechanism, not the second source** — L14 page screenshots sits in the TAKE table at v1, and North Star #14 permits PDFium caller-provided under an explicit ADR |
| **PI6** | **Normalization, bidi reordering or character substitution before the wire** | A 1 377-entry compatibility table rewrites `ſ` to `s` and `Ĳ` to `IJ`, changing the scalar count a citation is measured in. Severs `char_start`/`char_end` and `locate-scalar-exact-v1`, and contract §4 already settles it: *"No Unicode normalization — extracted text is preserved exactly as extracted"* |
| **PI7** | **A tool argument that lets the model name the page, plus a resolver that trusts it** | `client.py:2299-2302` promotes a model's guessed page into a structured citation record with no check anywhere. This is the only external, code-level case study for why `no_tool_argument_names_a_coordinate` exists; until now that test argued from `docs/history/12-V12-SCOPE.md` §3 alone. **The row names its own cost** — see the routing paragraph above — because a refusal that omits what it costs is the dishonest version of itself |
| **PI8** | **Fabricated titles** | `api.py:102-118`: where layout yields no hierarchy, one node per page titled `Page N`; on **every** path including the bookmark ones, a node titled `Preface` covering the pages before the first entry. Neither string is text any document wrote, and in the output they are indistinguishable from the real headings beside them. `17-D1-SCOPE.md:130` already refuses the related half — *"a boundary inferred from a bookmark is invented no matter how reasonable the inference looks"* |
| **PI9** | **A parsing rule implemented a second time in another language** | `flash` is transliterated JavaScript: `numbering.py:42-70` implements ECMAScript's `ToNumber` grammar — hex, octal, binary, `Infinity` — inside a *PDF section-number parser*; `aggregates.py:23-33` is an `Array.prototype.sort` callback run through `cmp_to_key`; every class carries minifier slot names. `docs/naming-rules.md:3-6` confirms the JS implementation ships, and the only cross-implementation conformance corpus governs **filename sanitisation**. **Attaches to the WASM DEFER** (`A9`/`P18`): a WASM build compiled from the same Rust is the same implementation and is fine; a hand-written re-implementation of the same rule ids is what this refuses |
| **PI10** | **A single score fusing signals of different kinds** | `heading_score = dominant_font_size + (2 if caps_heavy) + (1 if bold)` (`model/block.py:216`) puts three signals on one axis measured in points, then compares it to `body + 0.5`, `+1`, `+1.5`, `+2`, `+5`, `1.5×body`. A body-size ALL-CAPS line is indistinguishable from a 2 pt-larger mixed-case one at every downstream test, and nothing records which signal supplied the points. **Cites an existing rule rather than creating one** — `28-HEADINGS-SCOPE.md:724` already refuses "a confidence, **a score**, a near-miss or an ordering of candidates". It refuses the **mechanism**, not the case signal, which remains an S5-shaped question needing its own instrument and its own bound |
| **PI11** | **Caption-to-figure association** | Already settled four times over — P14, North Star #29's rider 3, `images.rs`'s field-name test, and `23-AUTO-TAGGING-SCOPE.md:265` (*"No heading, list, table, caption or span is ever written"*). Recorded so the next reader of the fifth source does not reopen it. PageIndex's version would breach twice: its flash pipeline never reads an image XObject at all, so for a figure-type region it infers the figure from **the area of the region containing no text** — a figure inferred from the absence of evidence |

### The adapter that was measured and refused

Both this file and the research checklist listed a `liteparse → ethos.grounding.v1` adapter as a
reasonable convenience — a few hundred lines, and probably having to declare an unknown coordinate
origin. v1.2-S5 built it as far as the evidence allows, and the answer is **no adapter**.

**The predicted blocker dissolved.** Their coordinate space and this engine's visible box turn out to
be the same box with the same origin, and their DPI makes the unit conversion exact. Floats were not
a blocker either — a value that will not land on an integer centipoint is an omission with a count,
the same honesty the projection already uses.

**Two different walls stopped it, and both are in `ethos.grounding.v1` itself:**

1. **Provenance.** Their output emits page, width, height, text and text items and nothing else
   (**L20**), so it cannot say what produced it. The grounding schema requires a producer name and
   version, and `additionalProperties: false` runs its whole length, so there is nowhere to record
   that an identity was *asserted by a caller* rather than *measured*. This repository's own verifier
   handling sets the precedent in the opposite direction: a verifier is pinned by version **and**
   binary digest, because an identity that can be claimed is one that can disagree with what it
   describes.
2. **Box semantics.** Their boxes are loose em boxes (**L18**), and the contract requires an emitted
   box to declare its kind. This schema has no such field and no room for one. Loose boxes here would
   reproduce L18 *inside this repository's own artifact type* — worse than not shipping, because the
   result would look like evidence.

Neither wall is per-document, so no adapter could clear them by being careful. The refusal is pinned
to those schema facts by a test: relax the producer requirement or add a box-semantics field, and the
test fails so this decision gets taken again on purpose rather than lapsing.

**What would change the answer:** they emit a self-identifying, versioned artifact, and either side
gains a way to declare box semantics on the wire. Neither is this repository's to do for them, and
widening `ethos.grounding.v1` is a change to the *verifier's* contract, not an adapter's business.

### Form XObject descent — measured 2026-09-22, and the verdict split

PageIndex descends into a `/Form` XObject's content stream. This engine counts the `Do` and does not,
and **nothing here refuses it**: the limitation exists because the work was not done. So it was
measured before being proposed, and the answer was not the one expected.
Full method and figures: [`measurements/form-xobjects/`](measurements/form-xobjects/README.md).

**The broad case is refuted.** Over the 8 gate documents and the 200 `opendataloader-bench` ones, 47
declare `form-xobjects-not-descended`, holding 70 forms; 44 of those show any text, and together they
show **1 464 bytes** — about a third of a page across the whole corpus, against 1 803 517 text nodes
in one gate fixture alone. Two independent instruments agree on 70 forms in 47 documents by different
routes. **Bytes and not operations**, because one `Tj` can draw a paragraph, and that distinction is
what settled it.

**The narrow case is real and is the one the counter was written for.** Read by exact code name, the
981-document OmniDocBench census holds **256 declaring it, of which 14 emit an entirely empty artifact
and are not scans** — born-digital pages whose whole content sits behind a form, the shape
`extract.rs:1043` describes. For those the loss is total. **How much text is behind them is
unmeasured**: that corpus cannot be re-fetched.

**Unbuilt, on narrower grounds than "not worth it."** The recall argument is refuted, and for the
population where loss is total the v2.2-S2 document-scoped counter already declares the emptiness, so
it cannot be mistaken for a blank page. **What reopens it:** a redistributable corpus of that shape,
or **any one of those 14 documents pinned here as a fixture**, which would give the slice a regression
test on day one.

---

## IMPROVE — take the goal, change the mechanism

| # | Capability | Target | Why the source's version is wrong |
| --- | --- | --- | --- |
| P1 | Bounded classification | v0 | Their sample bound is defeated by a later full rescan, so sampling one page costs the same as sampling all. Ours must actually stop |
| P2 | Counts instead of verdicts | v0 | A verdict plus a score is two wrong answers. Counts plus reasons is one honest one |
| **P6** | **Fail closed on unknown operators** | v0 | The `"` show-text operator is absent from their match: its text vanishes, surrounding runs merge with corrupt geometry, and the output still looks well-formed. **Undetectable downstream** |
| P5 | Honest geometry | v0 | Their `height` is literally the same variable as the font size, and `y` is an unlabelled baseline |
| L8 | Typed derivation instead of a nullable confidence | v0 | "None for native PDF text" is the right idea carried by the wrong field |
| **L16** | **Three exit codes, not two** | v0 | Password-protected, invalid header, corrupt header and missing file all exit 1, identically to "complex" |
| P7 | Stable reading order | v1 | Their multi-column rule flips at 15 lines: fourteen interleave, fifteen go column-major. A one-line edit reorders the page |
| O1 | Determinism as a **contract**, not a mode name | v0 | A mode can be turned off. A contract is checked |
| O2 | Self-describing JSON | v0 | Declared coordinate system, real page geometry, versioned shape |
| O5 | The table **detector** — the model itself is fine | v1 | Their row/column and span model is the best in open source. Their deterministic detector's score is a comparator, not this engine's floor |
| O8 | Markdown **only** with the anchor map | v1.1 | No projection at all beats a projection without one |
| P20 | Cross-reference repair or refusal | v0.1 | See [`01-CONTRACT.md`](01-CONTRACT.md) §8.1 — one bounded, declared repair |
| A14 | Declared erasure | v2 | If something is removed, the artifact says so and says how much — **per kind**, because one number cannot honestly answer *how much* for two kinds of erasure |
| **PI-A** | **Read the document's own `/Outlines`** | **pending — D1 §5** | The idea is right and nowhere near P14: a bookmark tree is a hierarchy the **author wrote**, which puts it beside O13 and P19 and on `structure.rs`'s own rule, *"Consume, never synthesise."* A repo-wide search of `crates/` for `Outlines`, `Bookmark` and `/Dest` returns **zero**. Four things in PageIndex's version are refused separately — PI1 (title repair), PI8 (`Preface`), the silent drop of entries whose destination does not resolve, and **re-stacking the levels**, so the depth on its wire is a position in a pruned stack rather than the depth the document declared. **What must be re-argued first is D1 §5's evidence bar, not P14**: §3 there refused `/Outlines` as a *document-boundary* signal and §6 conditioned the refusal on the name. Measured for that argument: **6 of 70 fixture PDFs carry one**, holding 2 273 entries, max depth 5, zero unresolvable destinations, 2 backward-stepping entries, 97.4% of titles present in the text of their resolved page. D1's own band was a single point at zero on 45 fixtures |
| **PI-B** | **The bookmark-title-versus-page-text cross-check** | **with PI-A** | `embedded_toc.py:286-295` asks whether a bookmark's normalised title appears in the text of the page its destination resolves to — declared structure checked against where the text really is, which is **E6's shape pointed at a new pair**. PageIndex spends it as a silent insert gate (`:456`). Here it is spent as **a count**: one named code per entry whose title was not found on its resolved page, in `markdown.rs:313`'s idiom. No entry dropped, no title rewritten. The folding the check needs is its own versioned rule id, not a flag — `locate-scalar-exact-v1`'s precedent |
| **PI-C** | ~~Cross-page recurrence as a furniture guard on inferred headings~~ **REFUSED on its own measurement, 2026-09-22** ([`measurements/headings/` §8](measurements/headings/README.md)) — only N=10 withholds zero author-declared headings, and there it withholds **one line across eleven documents**; every useful threshold breaches §7.5 bar 2, N=2 removing **14** real headings on 3 documents. The split is the finding: **pure gain on 6 documents, destructive on 3**, and what separates them is **position** — which decision #29's rider forbids this rule to read. PageIndex's own detector gates on a top/bottom-20% band **first** and uses recurrence only to confirm, so **the half this engine may take is exactly the half that does not work alone.** The gap it aimed at is still open and still real | ~~unscoped~~ | `type-size-v2`'s only page-furniture exclusion is `/Artifact` marked content, a **tagged**-PDF convention — and the rule fires only where a document declares no structure, so the guard does not fire on the population it runs on. A running head at 1.20× body em fires on every page. **Take only the recurrence half, and only as a negative gate**: a line whose normalized text appears on N or more distinct pages is not an inferred heading. **Text identity across pages and nothing else** — PageIndex's `detect_header_footer` gates on a top-20%/bottom-20% band *first* (`header_footer.py:296-298`) and uses recurrence only to confirm, and a band reads position, which row 29's rider forbids and `the_rule_reads_no_region_and_no_block` enforces. Subtractive, so it can only withhold and cannot fabricate; every withholding counted. **Needs §7.5's four bars set before code** — bar 2 is binding now that the first MHS measurement has happened |

## TAKE — build it as the source does

| # | Capability | From | Target |
| --- | --- | --- | --- |
| **L1** | Reason codes a caller can write policy against | LiteParse | v0 |
| **L2** | Two independent axes — OCR-need and layout-hard, neither implying the other | LiteParse | v0 |
| L3 | A boolean derived from the reason list | LiteParse | v0 |
| **L4** | No confidence in the classify output | LiteParse | v0 |
| L5 | Per-page detail, 1-indexed | LiteParse | v0 |
| **L6** | A flag for a synthesized trailing space — the best honesty field in the four parser projects | LiteParse | v0 |
| L7 | Glyph codes alongside the text, with the ligature caveat stated rather than papered over | LiteParse | v0 |
| P11 | One document load shared by classify and extract | pdf-inspector | v0 |
| P9 | Vendored encoding tables | pdf-inspector | v0 |
| P19 | The marked-content bridge from a text run to the tagged structure | pdf-inspector | v0 capture, v1 use |
| A5 | The six-variant error taxonomy | Anydoc | v0 |
| A4 | Content-based format detection | Anydoc | v0 |
| **A11** | Mutation testing every fixture, plus fuzzing per format | Anydoc | v0 — see the note below |
| P10 | Encoding-issue detection | pdf-inspector | v0.1 |
| O13 | Tagged-PDF consumption | ODL | v1 |
| O4 | XY-cut reading order | ODL | v1 |
| O7 | Multi-page table linking | ODL | v1 |
| A3 | The merged-cell slot model | Anydoc | v1 |
| P12 | Dual-mode table detection — rectangles and alignment | pdf-inspector | v1 |
| L11 | Vector path data for ruled-line detection | LiteParse | v1 |
| L12 / L13 | Forms and annotations as typed, **distinguishable** nodes | LiteParse | v1 |
| L14 | Page screenshots | LiteParse | v1 |
| A2 | One shared record, one serializer | Anydoc | v2 |
| A1 | Broad format coverage | Anydoc | v2 |
| **L9** | The open HTTP OCR contract, with confidence dropped from what we act on | LiteParse | v4 |

### A11 has two halves, and they landed at different times

Stated separately because a half-discharged obligation read as a whole one is how a gap survives.

| Half | PDF | Office |
| --- | --- | --- |
| **Fuzzing** | Covered at v0-M7 — two targets on the PDF entry points | Covered at v2-S12 — one target on the single entry point all eight formats share, seeded from the office fixtures |
| **Mutation testing** | Covered at v0-M7 — every manifest fixture damaged six ways, survivors pinned and triaged | Covered at v2-S13 — every office package damaged **twelve** ways, 148 mutants, survivors pinned in five explained classes |

**The office mutation kinds are not the same six, and that is a result rather than a shortcut.** A
PDF is a byte stream; an office document is mostly a container. Five PDF kinds carry over with their
mechanics rewritten around the ZIP. One — unknown operator — reduces to nothing for a package and is
dropped rather than faked, though RTF keeps it because its stream is plaintext. Five kinds are new,
because a container has hazards a byte stream does not: a truncated central directory, a byte flipped
inside the first deflated entry, a byte flipped inside the main part, a forged second
end-of-central-directory record, and an overwritten `mimetype` body. The two byte-flips are separate
kinds because they damage different things — the first entry a reader meets, and the part carrying
the document's text — and it was the second that found a real defect.

**What it found, and the fix.** The ZIP reader verified a part's declared length and never its
CRC-32, so a corrupted part that still inflated to the right size was read as though intact. It now
compares the CRC the central directory carries and refuses a mismatch under its own name, distinct
from a length or signature failure so a caller can switch on the cause. It shipped only after the
false-refusal rate was **measured at zero** over 40 valid packages and 2,370 entries, because
refusing a valid archive would be a regression dressed up as a hardening. Survivors fell from 36 to
31, emptying the main-part class entirely.

**One fuzz target rather than eight** is a measured choice. `read` is the single entry point every
format shares, so a corpus holding one valid package of each shape drives all eight through it. Eight
harnesses would divide one corpus eight ways and explore each branch on a fraction of the budget. A
format measured *unreachable* from that target would be the argument for splitting one out; the
module tree is not.

## DEFER — right idea, named version, no work before it

`O6` complex-table hybrid (v3) · `O9` HTML (v1.1) · `O14` auto-tagging (v2.2) · `O19` formula LaTeX
(v3) · `A9`/`P18` WASM (v2+) · `A10` agent skill (v1.2) · `P8` right-to-left (v1) · `L15` content
bounds, metadata and XFA (v1+)

---

## Native to this engine — the reason it exists

None of the five sources has these. They are not borrowed and they are not optional.

| # | Capability | Ver |
| --- | --- | --- |
| E1 | Citation verification — **the separate verifier's job**, never the engine's | v0.1, by shell-out |
| E2 | Capability declarations and fail-closed behaviour | v0 |
| E3 | Profile-as-identity | v0 |
| E4 | Canonical JSON and integer quanta | v0 |
| E5 | `DocumentRepresentation v0` and `ethos.grounding.v1` | v0 |
| E6 | The geometric-versus-structural cross-check | v1 |
| E7 | Derivation honesty | v0 |
| E8 | The bring-your-own-parser path, forever | v0 |

---

## Why the competitors are reference-only

This gets re-litigated, so the answer is written down.

**The parts worth wrapping are exactly the parts that would have to be un-wrapped.** In one surveyed
codebase, `height` is the same variable as the font size, `y` is a baseline the JSON never labels,
the classifier's confidence contradicts its own counts, the `"` operator is missing from the content
stream match, and horizontal scaling appears nowhere in the tree, so widths are wrong on any document
using it. Depending on it means inheriting types whose fields mean something other than their names,
in a project whose entire product is not doing that.

The more honest of the two cannot be the grounded core either: it does not bind the character-origin
call at all, emits floats, declares no coordinate space, and carries no schema version.

**The split that follows:** depend on `lopdf` for the object and cross-reference layer, which is
commodity. **Vendor** the encoding tables, which are data that does not churn. **Clean-room** the
content-stream interpreter, the classification and the layout.

---

## One number, and how to use it

**The published deterministic table score of 0.489** is the only figure two independent publishers
report bit-identically on the same corpus. The 0.907 one project leads with is its hybrid AI-backed
mode — a different, non-deterministic product.

**The chase is parked.** 69‰ is this engine on twelve tagged PDFs this repository owns, re-stated at
0.58.0 where it was 70‰, and the band belongs with it: 0‰..590‰, median 0‰, ten of twelve at zero. 0.489 is their score on their corpus, in a
different unit — TEDS ([`table-gate-v1.md`](table-gate-v1.md) §2). It is no longer the floor the next slice must beat, and it resumes only if
this repository has a labelled set it owns **and** the owner chooses to resume. v2-S19 built the set,
so only the owner's half is outstanding. **Fabrication 0 still binds.** v1 itself is closed — decision
#18 (2026-08-30) closed it on a capability statement and a band rather than on this macro, and
parking the chase is what that decision records rather than what it leaves open.

**What has not changed: 0.489 is never a claim to publish.** And read every other number in this
landscape as configuration-dependent. Two publishers scored the same parser on the same corpus with
the same evaluator and got 0.576 versus 0.873 overall, and 0.000 versus 0.693 on tables. The corpus
and evaluator are provably not the variable — other rows are bit-identical across both tables. The
cause was invocation flags: one ran the other's tool in its default text output mode with OCR on. **A
competitor benchmarked a rival by running the first command in its README and published a number 34
points low.**

That is the strongest available argument for pinning the exact artifact **and** the exact
invocation — and the reason this project publishes none of these tables.
