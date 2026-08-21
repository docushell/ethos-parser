# 06 — Steal / refuse

**Status:** the living steal / refuse extract. **This file is the authority** — the ~70-row research
checklist it was extracted from is off-tree (`reference/README.md`), so nothing below defers to it.
Row ids (`O5`, `A14`, `L30`, `P6`, …) stay citable as the names of decisions recorded here.

**Read this before proposing a feature borrowed from another parser.** It exists to prevent a
specific failure: a good-faith PR that imports a competitor's capability along with the bug that
makes it dishonest.

---

## The four-way steal formula

From the research pass that produced this file, quoted verbatim so the wording survives its source:

> ethos-engine should take **the table model, tagged-PDF consumption, and XY-Cut reading order** from
> **OpenDataLoader**; **the shared multi-format IR, the `CellSlot` merged-cell model, the six-variant
> error taxonomy, content-based format detection, and mutation-plus-fuzz testing** from **Anydoc**;
> **rectangle-based table detection, encoding-issue detection, single-document-load, and the `mcid`
> bridge** from **pdf-inspector**; **the reason-code classifier with two orthogonal axes, the open
> HTTP OCR spec, forms/annotations/vector extraction, DPI screenshots, and
> `trailing_space_generated`** from **LiteParse** — and then add the things none of them has: **a
> versioned output contract with a fingerprint, declared coordinate systems, integer quanta, typed
> absence, derivation classes, an intra-representation geometric↔structural locator cross-check, and
> citation verification.**

The last clause is the product. The first four are how it gets built without re-deriving twenty years
of PDF handling.

---

## Decision vocabulary

| Decision | Meaning |
| --- | --- |
| **TAKE** | Build it essentially as the source does. Idea and execution both sound |
| **IMPROVE** | Build it, change the mechanism. The goal is right; the source's execution is dishonest, lossy, or cliff-shaped |
| **REFUSE** | Do not build it, or not this way. It breaks a moat rule or a Workbench rule |
| **DEFER** | Right idea, wrong time. Named version, no work before it |

---

## The decisions that prevent bad PRs

Not the full table. These are the ones a reasonable engineer would get wrong.

### REFUSE — a PR doing this is rejected on principle, not on quality

| # | Capability | Decision | Why |
| --- | --- | --- | --- |
| P3 | Public confidence float | **REFUSE** | Workbench rule 9. And it is self-contradicting in practice: TEXT-BASED at 50% with zero text pages |
| L23 | Confidence used as a filter | **REFUSE** | LiteParse drops text below 0.3, then again below 0.1, silently, in two places. Rules 9 **and** 4 |
| L18 | Loose char boxes sold as precise positioning | **REFUSE as-is** | Em boxes ascent-to-descent, not ink — a line of `acme` boxed as tall as `Ãj`, with nothing in the output saying which it is. Right for line grouping, wrong for a citation highlight |
| L22 | Floats on the wire | **REFUSE** | Lossy round-trip, no fixed precision. Floats do not exist in canonical output |
| L21 | Undeclared coordinate space | **REFUSE** | Documented only in Rust doc comments and TS JSDoc. An artifact a reader cannot interpret without the source is not evidence |
| L20 | Shipping without a versioned output contract | **REFUSE** | This is the moat. No competitor has started it |
| L30 | **LibreOffice → PDF office bridge** | **REFUSE** | **It invents pagination.** Pagination does not exist in a DOCX and must not be synthesised. This is a refusal, not a fallback |
| L24/L25 | OCR merged into the native stream; OCR on by default | **REFUSE** | Discriminated only by an omittable nullable field. OCR is opt-in, profile-isolated, and authors only where no text layer exists |
| O21/O22/L27/L28 | Hidden text, headers, footers, small text dropped by default | **REFUSE** | ODL's README says *filters*; the Java **deletes**. Take the threat model, invert the mechanism: **report, never delete** |
| O23 | `--sanitize` rewriting the evidence | **REFUSE** | The artifact is the record |
| L13 | Undeclared AcroForm "repair" and always-on widget flattening | **REFUSE** | A repair that is not recorded is a fabrication. Rule 3 |
| P14 | Style and role inferred from font names or text prefixes | **REFUSE** | Presentation is not structure |
| O26 | "#1" / "fastest" / bake-off claims | **REFUSE** | Every headline in this landscape is publisher-owned, and one is provably 34 points off depending on invocation flags |
| O28 | JVM runtime dependency | **REFUSE** | ODL's cost of entry, and why its capabilities cannot be borrowed wholesale |
| — | **Wrapping pdf-inspector (or any competitor) as the grounded PDF core** | **REFUSE** | Reference-only. See below |
| L31 | Build-time PDFium download, unpinned | **REFUSE** | Vendor fork, by tag, no checksum, `vendor/` absent so the build is network-dependent by default |
| L19 | Sourcing character origins from LiteParse's design | **REFUSE — structural** | `FPDFText_GetCharOrigin` has **zero call sites**. The fingerprint-critical primitive is not bound at all, so that design cannot be the grounded PDF core |
| — | **A `liteparse` → `ethos.grounding.v1` adapter** | **REFUSE — measured at v1.2-S5** | Two walls in the *artifact type*, not in any document. Their JSON emits `page, width, height, text, text_items` and nothing else (L20), so it cannot name its own producer — and this schema requires `producer: {name, version}` with no way to mark a field asserted rather than measured. Their boxes are loose em boxes (L18) and §`01-CONTRACT.md` 5.3 requires an emitted box to declare its kind; this schema has nowhere to. Coordinates were **not** the blocker the memo predicted — see below. Pinned by `engine-grounding/tests/liteparse_refusal.rs` |

#### The adapter, measured (v1.2-S5)

This file and the parity checklist both listed a `liteparse → ethos.grounding.v1` adapter as a
reasonable v1.2+ convenience — *"~300–500 LOC"*, and *"would declare `coordinate_origin: unknown`
unless it also reads the source PDF"*. v1.2-S5 implemented that as far as the evidence allows and
the answer is **no adapter**.

**The predicted blocker dissolved.** LiteParse's space is top-left, 72 DPI, `CropBox`→`MediaBox`
(memo §18.2 #3), and this engine's visible box is `/CropBox` clipped to the media box, or the media
box where none is declared. Same box, same origin; 72 DPI is one point per unit, so points × 100 is
centipoints exactly. An adapter that read the PDF for page geometry — which is reading a document
this repository already knows how to measure, not laundering foreign text — could have declared
`top-left` honestly. Floats are not a blocker either: a value that will not land on an integer
centipoint is an omission with a count, the same honesty `project()` already uses.

**Two different walls stopped it, and both are in `ethos.grounding.v1` itself:**

1. **Provenance.** Their output *"emits `page, width, height, text, text_items` and nothing else"*
   (checklist §8, measured from `output/json.rs:46-65`) — checklist **L20**, the missing versioned
   contract this repository exists to attack. It cannot say what produced it. The grounding schema
   requires `producer: {name, version}`, both non-empty, and `additionalProperties: false` runs its
   whole length, so there is nowhere to record that an identity was *asserted by a caller* rather
   than *measured*. `VerifierBinary::identify` sets this repository's precedent in the opposite
   direction: a verifier is pinned by version **and** binary digest, because an identity that can
   be claimed is one that can disagree with what it describes.
2. **Box semantics.** Their bbox is a union of `FPDFText_GetLooseCharBox` — em boxes,
   ascent-to-descent, not ink (**L18**). `01-CONTRACT.md` §5.3: *"When a box is emitted, the
   artifact says what kind of box it is … If a future version emits loose boxes, it declares those
   separately."* This schema has no such field and no room for one. Loose boxes here would be L18 —
   *"sold as precise positioning, with nothing in the output saying which it is"* — reproduced
   inside this repository's own `artifact_type`, which is worse than not shipping, because the
   result would look like evidence.

Neither wall is per-document, so no adapter could clear them by being careful. The refusal is
pinned to those schema facts by `crates/engine-grounding/tests/liteparse_refusal.rs`: relax the
`producer` requirement or add a box-semantics field and the test fails, and this decision gets taken
again on purpose rather than lapsing.

**What would change the answer:** LiteParse emitting a self-identifying, versioned artifact, and
either side gaining a way to declare box semantics on the wire. Neither is this repository's to do
for them, and widening `ethos.grounding.v1` is a change to the **verifier's** contract, not an
adapter's business.

---

### IMPROVE — take the goal, change the mechanism

| # | Capability | Target | Why the source's version is wrong |
| --- | --- | --- | --- |
| P1 | Bounded classification | **v0** | `Sample(8)` is defeated by a Phase-3 full rescan: `Pages(1)` costs the same as `Full`. Ours must actually stop |
| P2 | Counts instead of verdicts | **v0** | A verdict plus a score is two wrong answers. Counts plus reasons is one honest one |
| **P6** | **Fail closed on unknown operators** | **v0** | The `"` show-text operator is absent from the match: its text vanishes, surrounding runs merge with corrupt geometry, output still looks well-formed. **Undetectable downstream** |
| P5 | Honest geometry | **v0** | `TextItem.height` is literally the same variable as `font_size`; `y` is an unlabelled baseline |
| L8 | Typed derivation instead of a nullable confidence | **v0** | "None for native PDF text" is the right idea carried by the wrong field |
| **L16** | **Three exit codes, not two** | **v0** | Password-protected, invalid-header, corrupt-header and missing-file all exit 1, identically to "complex" |
| P7 | Stable reading order | v1 | Multi-column flips on `min_lines < 15`. Fourteen lines interleave, fifteen go column-major. A one-line edit reorders the page |
| O1 | Determinism as a **contract**, not a mode name | v0 | A mode can be turned off. A contract is checked |
| O2 | Self-describing JSON | v0 | Declared coordinate system, real page geometry, versioned shape |
| O5 | The table **detector** (the model is fine) | v1 | ODL's row/col + spans model is the best in open source. Its deterministic detection scores 0.489 on their corpus — a comparator, not this engine's floor |
| O8 | Markdown **only** with the Anchor Map | v1.1 | Rule 8 prefers no projection at all |
| P20 | xref repair-or-refuse | v0.1 | `lopdf` refuses 19-byte entries (spec requires 20); PDFium repairs. Refusing is correct; the rate is a **declared limitation** |
| A14 | Declared erasure | v2 | If something is removed, the artifact says so and says how much. **Per kind, since v2-S11**: an embedded asset is counted under its own code rather than folded into the text-part count, because one number cannot honestly answer *how much* for two kinds of erasure |

### TAKE — build it as the source does

| # | Capability | From | Target |
| --- | --- | --- | --- |
| **L1** | Reason codes a caller can write policy against | LiteParse | **v0** |
| **L2** | Two orthogonal axes — OCR-need vs layout-hard; neither implies the other | LiteParse | **v0** |
| L3 | Boolean derived from the reason list | LiteParse | v0 |
| **L4** | No confidence in the classify output — satisfied by construction | LiteParse | v0 |
| L5 | Per-page detail, 1-indexed | LiteParse | v0 |
| **L6** | `trailing_space_generated` — the best honesty field in the four projects | LiteParse | **v0** |
| L7 | `char_codes` + the ligature caveat stated rather than papered over | LiteParse | v0 |
| P11 | Single document load shared by classify and extract | pdf-inspector | v0 |
| P9 | Vendored CMap tables | pdf-inspector | v0 |
| P19 | The `mcid` bridge from text run to tagged structure | pdf-inspector | v0 (capture) / v1 (use) |
| A5 | The six-variant error taxonomy | Anydoc | v0 |
| A4 | Content-based format detection | Anydoc | v0 |
| **A11** | Mutation testing every fixture + `cargo-fuzz` per format | Anydoc | v0 — **both lanes now cover both corpora. Fuzz: PDF (v0-M7) and all eight office formats (v2-S12), through three targets. Mutation: the 55 PDF manifest fixtures (v0-M7) and the 16 office packages (v2-S13), through two harnesses.** See the note below |
| P10 | Encoding-issue detection | pdf-inspector | v0.1 |
| O13 | Tagged-PDF consumption | ODL | v1 |
| O4 | XY-Cut reading order | ODL | v1 |
| O7 | Multi-page table linking | ODL | v1 |
| A3 | `CellSlot` merged-cell model | Anydoc | v1 |
| P12 | Dual-mode table detection (rectangles + alignment) | pdf-inspector | v1 |
| L11 | Vector path data for ruled-line detection | LiteParse | v1 |
| L12/L13 | Forms and annotations as typed, **distinguishable** nodes | LiteParse | v1 |
| L14 | DPI screenshots | LiteParse | v1 |
| A2 | Shared IR → one serializer | Anydoc | v2 |

**A11 has two halves and they are at different places.** Stated here because the row above cannot
hold it and a half-discharged obligation read as a whole one is how a gap survives.

| Half | PDF | Office |
| --- | --- | --- |
| **`cargo-fuzz`** | **covered** since v0-M7 — `open_and_classify` and `open_and_extract` | **covered at v2-S12** — `office_read`, on `engine_office::read`, the one entry point all eight formats share, seeded from `fixtures/office/` |
| **Mutation testing every fixture** | **covered** — every fixture in `fixtures/manifest.json` damaged six ways, survivors pinned and triaged (v0-M7) | **covered at v2-S13** — every package in `fixtures/office/` damaged **twelve** ways, 148 mutants, survivors pinned and triaged in five classes, run by CI job `v0-office-mutation`. A **second harness** in `crates/engine-office/tests/robustness.rs` rather than a second root in the manifest: that file's `all_fixtures()` feeds every entry of every root to `Document::open_bytes`, so office entries would have been refused as non-PDF while every assertion still passed |

**The mutation half is not the same six kinds, and that is the result rather than a shortcut.** A
PDF is a byte stream; an office document is mostly a container. Five of the PDF kinds carry over
with their mechanics rewritten around the ZIP, one — `unknown-operator` — reduces to nothing for a
package and is dropped rather than faked, and RTF keeps it under its own name because its stream is
plaintext. Four kinds are new because a container has hazards a byte stream does not: a truncated
central directory, a byte flipped inside a compressed entry, a forged second end-of-central-directory
record, and an overwritten `mimetype` body. Every pair a kind cannot apply to is **pinned and
counted**, so a mutation that quietly stops applying is a red test rather than lost coverage.

*"`cargo-fuzz` **per format**"* is discharged by one target rather than eight, and that is a
measured choice rather than a shortcut: `read` is the single entry point every format shares, so a
corpus holding one valid package of each shape drives all eight through it. Eight harnesses would
divide one corpus eight ways and explore each branch on a fraction of the budget. A format measured
unreachable from that target is the argument for splitting one out; the module tree is not.
| A1 | 14-format coverage | Anydoc | v2 |
| **L9** | The open HTTP OCR contract, `confidence` dropped from what we act on | LiteParse | v2.1 |

### DEFER — right idea, named version, no work before it

`O6` complex-table hybrid (v3) · `O9` HTML (v1.1) · `O14` auto-tagging (v2.2, parallel lane) ·
`O19` formula LaTeX (v3) · `A9`/`P18` WASM (v2+) · `A10` agent skill (v1.2) · `P8` RTL (v1) ·
`L15` content bounds / metadata / XFA (v1+)

---

## Ethos-native — not parity items, and the reason the engine exists

None of the four sources has these. They are not borrowed and they are not optional.

| # | Capability | Ver |
| --- | --- | --- |
| E1 | Citation verification — **the separate verifier's job**, never the engine's | v0.1 (by shell-out) |
| E2 | Capability declarations + fail-closed | v0 |
| E3 | Profile-as-identity (`profile_sha256` in the fingerprint) | v0 |
| E4 | c14n v1 + integer quanta | v0 |
| E5 | `DocumentRepresentation v0` + `ethos.grounding.v1` | v0 |
| E6 | Intra-representation geometric ↔ structural cross-check | v1 |
| E7 | Derivation honesty (`Extracted`/`Computed`/`Recognized`/`Proposed`) | v0 |
| E8 | BYO parser path, forever | v0 |

---

## Why pdf-inspector is reference-only

This gets re-litigated, so the answer is written down.

**The parts worth wrapping are exactly the parts that would have to be un-wrapped.**
`TextItem.height` is the same variable as `font_size`. `y` is a baseline the JSON never labels. The
classifier's confidence contradicts its own counts. The `"` operator is missing from the content
stream match, and `Tz` appears nowhere in the source tree, so widths are wrong on any document using
it. Depending on the crate means inheriting types whose fields mean something other than their names,
in a project whose entire product is not doing that.

Its MIT licence gives us the code to read regardless. Upstream moved 0.1.8 → 1.14.x in months.
Reference-only is both the cheaper path and the safer one.

**The split that follows:** depend on `lopdf` for the object/xref layer (commodity); **vendor** the
encoding tables (data, no churn, regenerating is pure cost); **clean-room** the content-stream
interpreter, classification, and layout.

**And on LiteParse, which is the more honest codebase of the two:** if anything were ever wrapped it
would not be pdf-inspector. But LiteParse cannot be the grounded PDF core either — it does not bind
`FPDFText_GetCharOrigin` at all, emits floats, declares no coordinate space, and carries no schema
version. A `liteparse → ethos.grounding.v1` adapter is a reasonable **v1.2+** item for adopters who
already run it, and it would have to declare `coordinate_origin: unknown` unless the adapter also
reads the source PDF.

---

## One number, and how to use it — **as of 2026-08-19, mostly not at all**

**ODL-local scores 0.489 on tables.** It is the only figure two independent publishers report
bit-identically on the same corpus. ODL's README leads with 0.907 — that is its **hybrid AI-backed**
mode, a different and non-deterministic product.

**The chase is parked.** **64‰** is this engine on **four tagged PDFs this repository owns**;
**0.489** is that published ODL-local score on **their** corpus. Same unit, different exam. 0.489 is
no longer the floor the next slice must beat (`00-NORTH-STAR.md` #10), and it resumes only if this
repository has a labelled set it owns and the owner chooses to resume. **Fabrication 0 still binds.
v1 is not complete.**

What has not changed: 0.489 is **never a claim to publish**. And read every other number in
this landscape as configuration-dependent: ODL and pdf-inspector scored LiteParse on the same corpus
with the same evaluator and got **0.576 vs 0.873 overall, 0.000 vs 0.693 on tables**. The corpus and
evaluator are provably not the variable — ODL's own row and markitdown's row are bit-identical across
both tables. The cause was invocation flags: ODL ran LiteParse in its default `text` output format
and with OCR on. A competitor benchmarked a rival by running the first command in its README and
published a number 34 points low.

That is the strongest available argument for pinning the exact artifact **and invocation** — and the
reason ethos-engine publishes none of these tables.
