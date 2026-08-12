# 01 — The contract

**Status:** bootstrap authority · frozen before implementation (north-star decision #2)
**Scope:** what an ethos-engine artifact must contain, for any version, in any format

---

## 1. Why this document comes first

Forced decision #2: **freeze the verify contract before implementing the engine.** The reason is not
process hygiene. It is that a parser built first and a contract written afterwards produces a
contract shaped like that parser's accidents — its rounding, its coordinate origin, its idea of what
a "line" is. Then the verifier inherits them, and a rule that exists because of a 2026 lopdf quirk
becomes a permanent semantic.

So: this document defines the artifact. `04-ARCHITECTURE.md` defines the code that produces it. If
the two ever disagree, this one is right and the code is a bug.

**What "frozen" means here:** the *rules* below are frozen. The exact JSON field names of
`DocumentRepresentation v0` are a **DRAFT** target held against a spec nobody has implemented yet
(north-star §4, risk #1 in `03-V0-SCOPE.md`). Where a field name is uncertain this document says
`TODO(re-read DocumentRepresentation v0 field list)` rather than inventing one.

**M1 re-read status.** The companion was re-read against every marker. What it settles is now used
verbatim; what it does not is stated as a divergence rather than guessed at.

| Settled by the companion — now used | Where |
| --- | --- |
| `NativeLocator` (required), `StructuralLocator` (where the kind defines one), `RenderedLocator`/geometry (optional, "for inspection") | §5.1 |
| `TableCellPosition(row, column, rowspan, colspan, parent table node ID)`, **zero-indexed**, span of 1 = not merged | §5.4, v1 |
| `ProcessingRun` / `StageRun` carrying processor/adapter/build/profile identities | §7 |
| Node fields: stable id, kind, parent, ordinal, text/value, attributes | M5 |
| Source identity vs representation identity as two distinct hashes | §2, `ArtifactBinding` |
| "capability-limited" as the outcome name for a claim binding to unprocessed content | §7 |
| Locator-consistency result recorded as a **typed diagnostic with a check version**, never silently repaired | §5.4 |
| Absence expressed as an absent field, never an implied `1.0` | §9.1 |

| Still open — engine-local, pending DocuShell review | Where |
| --- | --- |
| A name for *why* geometry is absent. The companion models geometry as an optional field and never names an absence variant | §5.2 |

Nothing here blocks M2–M4. The divergence is additive and projects down cleanly.

**Source of truth for the target shape:**
`docushell-repo/docs/FUTURE_DOCUMENT_AI_TRUST_INFRASTRUCTURE_ARCHITECTURE.md`, "Minimum canonical
document representation." Read-only. Never edited from this repo.

---

## 2. Artifact identity

Every artifact the engine emits — classification, representation, grounding — carries the same four
identity fields, at the top level, before any payload.

| Field | Type | Rule |
| --- | --- | --- |
| `artifact_type` | string const | Names the shape exactly. E.g. `ethos.grounding.v1`. A reader that does not recognise it **fails closed** (§8) |
| `schema_version` | string const | Semantic version of *this artifact shape*. Changes when the shape changes, independent of the parser |
| `parser_version` | string | The engine build that produced it. Distinct from `schema_version`: the same shape can be emitted by many builds |
| `profile_sha256` | `sha256:<64 hex>` | Hash of the pinned configuration profile — every knob that can change output. **This is the identity that matters** |

**Profile-as-identity is the load-bearing idea.** Two artifacts are comparable if and only if their
`profile_sha256` matches. This is what makes an OCR'd document non-comparable with a born-digital
parse by contract rather than by convention (§6), what makes a backend swap visible, and what makes
"we changed the sample count" a fingerprint event instead of a silent drift. It costs one hash and
buys every isolation property the roadmap needs.

The profile must include, at minimum: engine build identity, backend identity and version, the
classify sample count `N`, the quantum (`100` per point), the coordinate origin, the enabled
capability set, and — when they exist — the OCR engine identity, model hash, and execution envelope.
Anything that can change a byte of output belongs in the profile or is a bug.

Two further identities, both required, and they are not the same thing (DocuShell spec, "For any
format, retain two different identities"):

- **Source identity** — `sha256` of the exact original bytes. What was read.
- **Representation identity** — `sha256` of the canonical evidence produced under the pinned
  profile. What was produced. Ethos's grounding validator names this `representation_sha256`.

---

## 3. Coordinate system — declared on the wire, always

An artifact that cannot be interpreted without its source file is not evidence. LiteParse documents
its coordinate space (viewport, top-left, 72 DPI, CropBox→MediaBox) **only in Rust doc comments and
TS JSDoc** — a consumer reading the raw JSON has to infer it (checklist L21). That is the failure to
avoid.

Every artifact carrying geometry declares:

```json
"coordinate_system": { "unit": "centipoint", "origin": "top-left" }
```

Both values are consts in `ethos.grounding.v1` today. Rules:

- **Never omit it**, even when the value is the only one the engine supports. A const today is a
  discriminator tomorrow.
- **Never let it be implied by the format.** PDF's native origin is bottom-left; the artifact's is
  top-left; the transform is the engine's job and the declaration is how a reader knows it happened.
- **A rotated page declares its rotation** (`0 | 90 | 180 | 270`) per page, and geometry is expressed
  in the declared system after rotation is applied. Fixture: `synthetic/rotation-90`.
- **A second coordinate system needs a new enum value, not a new default.** Adding one is a
  `schema_version` change.

## 4. Canonicalization and integer quanta

The engine adopts Ethos's c14n v1 wholesale (`ethos/docs/determinism-contract.md` §2, implemented in
`ethos-core/src/c14n.rs`). It is a solved problem; re-solving it is pure risk.

| Rule | Detail |
| --- | --- |
| **Encoding** | UTF-8, no whitespace between tokens |
| **Key order** | Sorted by Unicode code point, explicitly at write time — never relying on map iteration order (a `serde_json/preserve_order` feature unification anywhere in the graph would otherwise break every fingerprint silently) |
| **Escaping** | Minimal: `"`, `\`, and U+0000–U+001F only. No `\uXXXX` for non-ASCII. **No Unicode normalization** — extracted text is preserved exactly as extracted |
| **Numbers** | **Integers only.** Base-10, no leading zeros, no `+`, no exponent, \|n\| ≤ 2^53−1. Any non-integer number anywhere in a canonical value is a hard error |

**On `-0`.** The rule is about *output*: canonical output never contains `-0`, which falls out of
integers being the only representation — `i64` has no negative zero, and `quantize(-0.0)` returns
`0`. It is **not** an input-normalization rule. JSON text `-0` parses as the float `-0.0`, and c14n
rejects it as a non-integer rather than folding it to `0`. Ethos behaves identically. Folding would
mean silently accepting a float, which is the one thing this layer exists to refuse.
| **Arrays** | Order is semantic. Element order *is* reading order |
| **Idempotence** | `c14n(parse(c14n(v))) == c14n(v)`, property-tested |

**Floats do not exist in canonical output.** Geometry is quantized to integer centipoints —
`quantize(pts, 100)`, round-half-away-from-zero, with `NaN`, `±Inf`, overflow and a zero quantum as
errors rather than saturating values.

**One measured divergence from Ethos, in the engine's favour.** Ethos computes the rounding as
`(x + 0.5).floor()`, which double-rounds above `2^52` — an exact integer product returns one quantum
too large, and `MAX_SAFE_INT` (which §4 declares canonical) is refused outright. The engine uses
`f64::round`, IEEE `roundToIntegralTiesToAway`: the same rule, computed exactly. An exhaustive
knife-edge sweep of `[0, 2·10^6)` finds **one** disagreement, `0.49999999999999994`, where the
engine returns `0` — correct, since the value is below one half. No decimal literal reaches it, and
across all ten million `0.001`-step literals in `[0, 10000)` points the two agree everywhere. So the
divergence lives only where Ethos is arithmetically wrong, and page geometry cannot get there. LiteParse emits `f32` throughout with a lossy round-trip and no fixed
precision (checklist L22); that is the shape to refuse.

**Canonical exclusions.** Volatile data — timings, memory, host, source paths — lives under a
`diagnostics` object that is excluded from the fingerprint and off by default. Nothing inside the
payload may be runtime-dependent. Ethos property-tests this by asserting no diagnostics-class field
name appears in payload; do the same.

**Geometry is excluded from the fingerprint** — deliberately, and it is worth understanding why
before someone "fixes" it. Rectangle dimensions are preserved for display and inspection but are not
fingerprint-critical, because text is anchored by **stable origin locators**, not by boxes. This is
the same asymmetry §5 describes: two independent PDF stacks agree on character origin to 0.001 pt
and disagree on height by 6.174 pt. Fingerprinting the boxes would make the artifact identity
hostage to the least reliable number in it.

---

## 5. Locators — origins are identity, boxes are inspection

`DocumentRepresentation v0` settles this and the measurements agree with it: **`NativeLocator` is
required; `RenderedLocator`/geometry is optional and "for inspection."**

### 5.1 The three locator kinds

| Locator | Required? | What it is | v0 |
| --- | --- | --- | --- |
| **`NativeLocator`** | **Always, on every node** | The format-native address. For PDF: page + character origin (x, baseline y) + advance width, in integer centipoints | **yes** |
| `StructuralLocator` | Where the node kind defines one | Structure-tree address: tagged-PDF role path, `mcid`, table row/col | v0 emits `mcid` only; full structural addressing is v1 |
| `RenderedLocator` / geometry | Optional | An ink box, for humans and crops | v0: measured from font metrics, or typed absence |

`NativeLocator` is a **discriminated union**. Adding a format adds a variant — `PdfLocator`,
`DocxLocator`, `XlsxLocator`, `PptxLocator`, `ImageLocator`, `EmailLocator` — plus an adapter
profile, fixtures, and inspection behaviour. It does not change the source, run, artifact, candidate,
or verification models. v0 ships `PdfLocator` and nothing else.

**A rendered page/bbox is never substituted for the native source address**, unless an approved
format profile defines that rendering as authoritative. No such profile exists in v0.

### 5.2 Typed absence

A missing box is a **type**, never a sentinel and never a substitute.

- Where font metrics are unavailable, emit `GeometryPresence::Absent(NotReportedByReader)` **and**
  declare the capability limit (§7). Two sibling variants exist — `NotApplicableToKind` and
  `CapabilityNotEnabled` — and only `NotReportedByReader` counts toward the limitation, because a
  node kind that never has geometry is not a gap in what the engine could do.

  **`TODO(re-read DocumentRepresentation v0 field list)` — re-read, still open, and now precise.**
  The companion settles the locator names (`NativeLocator` required, `StructuralLocator` where the
  kind defines one, `RenderedLocator`/geometry optional "for inspection") and it settles that
  absence is expressed as an **absent field** — *"A processor that reports no uncertainty produces
  an absent field, never an implied `1.0`."* What it does **not** do is name a variant for *why*
  geometry is absent; its model is "optional field, omitted".

  So this is a real divergence, not a missing lookup. Typed absence carries strictly more
  information than an omitted field, and it projects down to one cleanly (all three variants
  serialize to "no geometry" on the DocuShell wire). v0 keeps the richer type and does not invent a
  competing *field name*. Pending DocuShell review — tracked in `docs/README.md`.
- **Never `height = font_size`.** pdf-inspector's `TextItem.height` is literally the same variable as
  `font_size` (checklist P5). A font-size-derived box is closer to invented than measured, and
  Workbench rule 3 forbids inventing a coordinate.
- **Never a zero box, a null island, or a page-sized box** as a stand-in.
- The consequence is stated, not hidden: without geometry on some nodes, **crops and highlight
  rendering cannot be driven by ethos-engine v0** for those nodes. PDFium/Ethos keeps the crop lane.

### 5.3 Declared box semantics

When a box *is* emitted, the artifact says **what kind of box it is**. LiteParse's bbox is a union of
`FPDFText_GetLooseCharBox` — em boxes, ascent-to-descent, not ink — sold as "precise positioning,"
with nothing in the output saying which it is (checklist L18). For a line of `acme` the box is as
tall as if it contained `Ãj`. Right for line grouping, wrong for a citation highlight.

v0 emits **measured ink boxes only** — ascent/descent from the embedded font program via
`ttf-parser`, falling back to the FontDescriptor's `/Ascent`, `/Descent`, `/FontBBox` — and declares
them as such. If a future version emits loose boxes, it declares those separately.

### 5.4 The cross-check (v1, designed now)

Where a node has both a geometric and a structural address, **derive them independently and test them
against each other**: overlapping cell regions, a cell outside its parent table, a grid that does not
tile the table area. *"A single locator can only be trusted or not; a pair can be tested."*

Result is a **typed diagnostic with a check version**, never a silent repair. This is v1 work, but
the contract must not foreclose it: nodes carry both address kinds where both exist, from v0 onward,
even while nothing cross-checks them yet.

---

## 6. Derivation classes

Every node declares how it came to exist. This is the axis that lets OCR and assist exist later
without laundering into born-digital certainty.

| Class | Meaning | May author | Notes |
| --- | --- | --- | --- |
| **`Extracted`** | Read from the source's own encoding | Text, origins, font identity, `mcid` | The only class v0 produces |
| **`Computed`** | Derived deterministically from `Extracted` values by a versioned rule | Reading order, line grouping, ink boxes from font metrics | The rule's version is part of the profile |
| **`Recognized`** | Produced by a recognition engine over pixels | OCR text and geometry | v2.1. Own profile. **May author nodes only on canvases where the deterministic reader found no text layer at all** |
| **`Proposed`** | Suggested by a model | Nothing citable, ever | v3. Never evidence. Never overwrites another class |

Three hard rules:

1. **`Recognized` never overwrites `Extracted`.** Not merged, not preferred, not reconciled.
   LiteParse merges OCR into the native text stream discriminated only by an omittable nullable
   field (checklist L24); that is the bug.
2. **`Proposed` is never citable.** A chart description or a formula guess can exist in the tree and
   can never be the source of a verified quote.
3. **Different classes mean different profiles.** An OCR run has a different `profile_sha256`, so its
   output is non-comparable with a born-digital parse *by contract*, with no new machinery.

---

## 7. Capabilities and limitations — the L1 gate

L1's achievement condition names capability declarations explicitly: *"a versioned processor/profile
produced a representation with declared capabilities and an extraction-assurance state."* **An
artifact without them has not reached L1.** They are not polish and they are not documentation.

Every artifact declares:

- **Capabilities** — what this profile can do. `ethos.grounding.v1` requires `spans`, `char_offsets`,
  `tables` as booleans. The representation's set is richer.
- **Limitations** — what it could not do *on this document*, named. v0 must declare **multi-column
  reading order** explicitly, because v0 ships single-column order and a two-column document will be
  read in the wrong order (§`03-V0-SCOPE.md`).
- **Per-page processing state** and a **coverage summary** reconciling authorized / processed /
  failed / unsupported / quarantined pages.

**Partial processing is a first-class outcome with its own terminal state.** A representation where
some pages failed may still support claims binding to pages that succeeded — but the gap must be
visible to every consumer. A verification over a partially processed document must never render as a
clean verification of the whole document. And a claim binding to a failed, unsupported, or
quarantined page returns an explicit capability-limited or indeterminate result: **absence of
extractable content is never evidence of absence in the source** (Workbench rule 4).

---

## 8. Fail closed

The engine fails closed and says why. It never fails open, and it never fails silently.

| Situation | Behaviour |
| --- | --- |
| **Unrecognised content-stream operator** | Hard error naming the operator. pdf-inspector omits the `"` show-text operator from its match: the text vanishes and surrounding runs merge with corrupt geometry, output still well-formed, undetectable downstream (checklist P6). Enumerate the operator set explicitly; anything outside it stops the parse |
| **Unknown `artifact_type` or `schema_version`** | Refuse to read. Never best-effort parse an unrecognised shape |
| **Missing capability for a requested operation** | Explicit capability-limited result. Never a stub, never a default, never a skip |
| **A number that will not quantize** (`NaN`, `±Inf`, overflow) | Error. Never saturate, never clamp |
| **Malformed xref / broken trailer** | Refuse. `lopdf` rejects 19-byte xref entries where PDF 32000-1 §7.5.4 requires exactly 20 — roughly **1 valid document in 26** on Ethos's own fixture corpus, against a backend that repairs it. Refusing is correct; **the rate is a declared limitation**, and repair-or-refuse is a v0.1 decision, not a v0 improvisation |
| **Encrypted / password-protected source** | Distinct exit code. Never the same signal as "this document is complex" |

**Three outcomes get three exit codes** (§`03-V0-SCOPE.md` for the mapping). LiteParse's
`is-complex` predicate returns 1 for password-protected, invalid-header, corrupt-header **and**
missing-file, identically to "complex" — a predicate that cannot distinguish *"hard"* from *"I could
not open this"* (checklist L16). Fail closed with an *indistinguishable* signal is still a defect.

---

## 9. No public confidence field

Workbench **rule 9**: *"Confidence is not a gate. Routing on a threshold presents an uncalibrated
number as a safety control."*

**No artifact this engine emits contains a public confidence float, score, grade, or any single field
summarising quality.** Not in classify, not in the representation, not in the grounding artifact.
Grep for it in review.

The measured case, produced by the exact feature under consideration — pdf-inspector on
`fixtures/synthetic/simple-text`:

```
Type: TEXT-BASED (extractable text)
Confidence: 50%
Pages with text: 0            ← its own evidence disagrees with its verdict
OCR recommended: NO
```

Two routing rules a reasonable engineer would write, both wrong on this corpus: `confidence ≥ 0.7 →
trust text extraction` sends a plainly text-based PDF to OCR; `type == TEXT_BASED → skip OCR` skips
OCR on a document the detector itself says has no text. LiteParse, the more honest codebase,
satisfies rule 9 by construction — no confidence field anywhere in its complexity output — and that
is the shape to copy.

**What replaces it:** counts and named reasons. `pages_with_text` / `pages_sampled`, per page,
1-indexed. Reason codes on two orthogonal axes. A boolean derived from the reason list, never the
other way round. The caller owns the policy; the engine owns the observation.

### 9.1 The one place uncertainty is permitted, and why v0 has none

`DocumentRepresentation v0` allows an optional node confidence plus optional character-ranged
low-confidence spans, because a *recognition* processor genuinely has uncertainty and hiding it at
node granularity makes a reviewer re-read a whole paragraph to find four doubtful characters. Its own
rules: uncertainty is **processor-owned, never verifier-owned**; it must never appear in a
verification report, alter a deterministic result state, or blend into a combined score; scores are
ordinal unless a profile proves otherwise; and **absence is not confidence** — a processor reporting
no uncertainty emits an absent field, never an implied `1.0`.

**ethos-engine v0 has no uncertainty to report.** Every node is `Extracted` by a deterministic
reader; there is no recognition step. So the field is **absent**, which the spec explicitly permits,
and §9's prohibition stands unqualified for v0 through v2. When the OCR lane lands at v2.1, it may
populate span-level uncertainty as a **diagnostic** — accepted if a server sends it, recorded, and
**never filtered on**. LiteParse drops text below 0.3 and again below 0.1, silently, in two different
places (checklist L23). That is the bug not to inherit.

---

## 10. Synthesized-character honesty

Some characters in a parser's output were never in the document. A reader that inserts a space
between two runs because they looked like separate words has authored content. If that is not
flagged, a verifier will one day match a quote against a space the source does not contain.

LiteParse's `trailing_space_generated` — *"whether the trailing source space was synthesized by
PDFium rather than represented by a real space glyph"* — is the single best honesty field in the four
projects surveyed (checklist L6). Adopt the idea, generalise the name.

| Rule | Detail |
| --- | --- |
| **Every synthesized character is flagged at emission** | Not reconstructed later, not inferred from spacing. Flagged where it is created |
| **Glyph codes travel with the text** | `char_codes` alongside the string, with the ligature caveat declared: ligature expansion yields more scalars than codes, so the arrays are not 1:1 and the artifact says so (checklist L7). Fixture: `synthetic/ligature-fi-embedded-font` |
| **A repair is a recorded event or it is a fabrication** | LiteParse "repairs orphaned widgets in memory" and always flattens widgets — an undeclared document mutation (checklist L13). Any normalization the engine performs is declared in the profile and visible in the artifact |
| **Hyphenation rejoin, if performed, is `Computed` and reversible** | The source bytes are recoverable from the artifact. Fixture: `synthetic/hyphenated-line-break` |

---

## 11. Grounding adapter — `ethos.grounding.v1`

The engine's canonical emit is `DocumentRepresentation v0`. The **grounding adapter** projects it
into `ethos.grounding.v1`, the shape today's Ethos verifier consumes. This mapping is what makes the
v0 oracle test possible (`05-MILESTONES.md` M6).

**Target shape** — read from `ethos/schemas/ethos-grounding-source.schema.json`, `$id`
`urn:ethos:schema:grounding-source:1`. `additionalProperties: false` throughout, so the adapter emits
exactly these fields and nothing else.

| Field | Required | Shape | Engine source |
| --- | --- | --- | --- |
| `artifact_type` | ✓ | const `ethos.grounding.v1` | literal |
| `schema_version` | ✓ | const `1.0.0` | literal |
| `source` | ✓ | `{media_type: "application/pdf", sha256: "sha256:<64hex>"}` | source identity (§2) |
| `producer` | ✓ | `{name, version}` | engine name + `parser_version` |
| `capabilities` | ✓ | `{spans: bool, char_offsets: bool, tables: bool}` | capability set (§7). **v0: `tables: false`** |
| `coordinate_system` | ✓ | `{unit: "centipoint", origin: "top-left"}` | §3 |
| `pages` | ✓ | `[{id, index≥1, width, height, rotation ∈ {0,90,180,270}}]` | per page, integer centipoints, **1-indexed** |
| `elements` | ✓ | `[{id, page, bbox, kind, text?}]` | typed nodes; `kind` matches `^[a-z0-9][a-z0-9_-]*$` |
| `spans` | optional | `[{id, page, bbox, text, element?, char_start?, char_end?}]` | text runs |
| `tables` | optional | `[{id, page, bbox, cells:[{row, col, row_span, col_span, bbox, text}]}]` | **not emitted in v0** |

**`bbox` is `[x0, y0, x1, y1]`** — left, top, right, bottom — in integer centipoints, matching
Ethos's `QRect`. Not `[x, y, w, h]`. The schema enforces `x1 ≥ 1` and `y1 ≥ 1`.

**Zero-area boxes: the engine is stricter than the oracle, deliberately.** Ethos's `QRect::new`
(`ethos-core/src/geom.rs:87`) rejects only `x0 > x1 || y0 > y1`, so a **degenerate `x0 == x1` box is
accepted** on the grounding path; the non-positive-area rejection at `crop_element.rs:284` is on the
*crop* path and does not run here. The grounding schema does not exclude it either. So a zero-area
box is not a shared error — it is something ethos-engine refuses to *emit* while Ethos would accept
it. State it that way round, and never as "matching Ethos's fail-closed behaviour."

Implemented at M1: `engine_core::QRect::new` requires `x1 > x0 && y1 > y0`, and `serde`
deserialization goes through the same constructor so a degenerate rectangle cannot enter through the
wire either.

This asymmetry is safe for the M6 oracle test because that test compares `structure`,
`source_binding`, `representation_sha256` and `counts` — not per-box validity — and because being
stricter means the engine never produces an artifact Ethos would reject. **A future stricter
*emission* rule must be checked against this direction before it lands:** the engine may refuse to
emit what the oracle tolerates, never the reverse.

**Three things the adapter must confront honestly, and none of them is a bug in the adapter:**

1. **`ethos.grounding.v1` has no confidence field anywhere.** Confirmed against the schema. Good —
   §9 is satisfied by the target shape itself.
2. **`bbox` is required on every element and span**, but §5.2 says a node with no measurable font
   metrics gets typed absence. These conflict. **DECIDED (2026-08-12): omit from grounding, count and
   declare.** Honesty and schema validity both survive; fabricating a box would sacrifice the first
   and violating the schema would sacrifice the second.

   | Layer | Behaviour |
   | --- | --- |
   | `DocumentRepresentation v0` | **Node stays.** Geometry is typed absence; `NativeLocator` still present |
   | `ethos.grounding.v1` | **Node omitted.** Declared limitation + count. Never `[0,0,0,0]`, never `height = font_size` |
   | Future | Ethos owners: optional `bbox` where a native address exists. Open, and it does not block v0 |

   Two constraints this decision imposes, both enforced at M5:
   **omission is only ever for missing measurable geometry** — never because a classifier disliked a
   page, never as a quality filter; and the omit-plus-count path needs **its own fixture with absent
   metrics**, not just `simple-text`.

   M1 built the type that makes this enforceable: `GeometryPresence::is_groundable()` takes a
   measurement state, not a boolean or a reason code, so the omission path is unreachable from a
   quality judgement by construction rather than by review.

   `TODO(confirm with Ethos owners whether a geometry-absent span should be representable in a future
   grounding schema revision. Open; does not block M0–M4.)`
3. **The grounding shape is lossy relative to `DocumentRepresentation v0`.** It carries no derivation
   class, no `mcid`, no structural locator, no synthesized flags, no per-page coverage state. That is
   expected — it is a verifier's input, not the canonical record. **The representation is the record;
   the grounding artifact is a projection.** Never treat a grounding round-trip as proof the
   representation is intact.

**Validation oracle** — `ethos/schemas/ethos-grounding-validation-report.schema.json`, artifact type
`ethos.grounding_validation.v1`:

| Field | Values |
| --- | --- |
| `structure` | `valid` \| `invalid` |
| `source_binding` | `matched` \| `mismatched` \| `not_checked` |
| `representation_sha256` | `sha256:<64hex>` |
| `counts` | `{pages, elements, spans, tables}` |

M6's exit criterion is agreement with `ethos grounding check <file> --source-artifact <pdf>` on
exactly these four, across all 15 fixtures.

---

## 12. What this contract deliberately does not do

- **It does not verify.** No claim input, no verdict output, no `grounded` field, no `evidence_tier`.
  The engine's happy path terminates at a *validated* artifact, not a *verified* one
  (`07-VERIFY-BOUNDARY.md`).
- **It does not describe a customer output schema, an extraction candidate, a resolved business
  value, a semantic assessment, a policy decision, or a reviewer decision.** Those are separate
  versioned resources that *reference* representation nodes. Keeping them out is what lets
  re-extraction and re-verification happen without reparsing, and what stops a model transcript from
  becoming the document record.
- **It does not define a Markdown projection.** Workbench rule 8: retrieval operates on the evidence
  record itself; any projection between what is ranked and what is cited is where a locator dies
  silently. Markdown ships at v1.1 *with* the Anchor Map or not at all.
- **It does not promise global identifiers.** IDs are stable only within a representation created by
  the same pinned profile. A parser upgrade creates a new representation plus a mapping/diff — it
  does not pretend node IDs are permanently global.
- **It does not claim performance.** No "fastest," no "#1," no inherited latency figure. See
  `03-V0-SCOPE.md` §6.

---

## PR review checklist

- [ ] Every emitted artifact carries `artifact_type`, `schema_version`, `parser_version`,
      `profile_sha256`
- [ ] `coordinate_system` present on every artifact carrying geometry, never implied
- [ ] No float appears anywhere in canonical output; c14n rejects non-integers as a hard error
- [ ] Object keys sorted explicitly at write time, not by map iteration order
- [ ] Every node has a `NativeLocator`
- [ ] No box is derived from a font size; absent metrics produce typed absence plus a declared limit
- [ ] Every node declares a derivation class; nothing but `Extracted` is produced in v0
- [ ] Capabilities and limitations present; per-page state and coverage summary present
- [ ] Unknown operator / unknown artifact type / unquantizable number all fail closed with a named
      error
- [ ] `grep -ri confidence` over emitted artifacts and their types returns nothing public
- [ ] Synthesized characters are flagged at the point of creation
- [ ] Grounding adapter emits exactly the schema's fields (`additionalProperties: false`) with
      `bbox` as `[x0, y0, x1, y1]`
- [ ] No verification concept has leaked in: no claim, no verdict, no `grounded`, no `evidence_tier`
