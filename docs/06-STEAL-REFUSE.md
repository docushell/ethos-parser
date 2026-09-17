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
| P14 | Style and role inferred from font names or text prefixes | Presentation is not structure |
| O26 | "#1", "fastest", or any bake-off claim | Every headline in this landscape is publisher-owned, and one is provably 34 points off depending on invocation flags |
| O28 | A JVM runtime dependency | One project's cost of entry, and why its capabilities cannot be borrowed wholesale |
| L31 | An unpinned build-time renderer download | Vendor fork, by tag, no checksum, so the build is network-dependent by default |
| L19 | Sourcing character origins from one competitor's design | **Structural:** the character-origin call has zero call sites there. The fingerprint-critical primitive is not bound at all, so that design cannot be the grounded PDF core |
| — | Wrapping any competitor as the grounded PDF core | Reference only — see below |
| — | A `liteparse` → `ethos.grounding.v1` adapter | **Measured and refused at v1.2-S5** — see below |

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

## TAKE — build it as the source does

| # | Capability | From | Target |
| --- | --- | --- | --- |
| **L1** | Reason codes a caller can write policy against | LiteParse | v0 |
| **L2** | Two independent axes — OCR-need and layout-hard, neither implying the other | LiteParse | v0 |
| L3 | A boolean derived from the reason list | LiteParse | v0 |
| **L4** | No confidence in the classify output | LiteParse | v0 |
| L5 | Per-page detail, 1-indexed | LiteParse | v0 |
| **L6** | A flag for a synthesized trailing space — the best honesty field in the four projects | LiteParse | v0 |
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

None of the four sources has these. They are not borrowed and they are not optional.

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
