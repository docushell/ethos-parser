# 01 — The contract

**Status:** bootstrap authority · frozen before implementation (decision #2)
**Scope:** what an ethos-parser artifact must contain, in any version, for any format

---

## 1. Why this comes first

Decision #2 freezes the verify contract before the engine is built. The reason is not process
hygiene. A parser built first and a contract written afterwards produces a contract shaped like that
parser's accidents — its rounding, its coordinate origin, its idea of what a "line" is. The verifier
then inherits them, and a rule that exists because of one library's quirk becomes a permanent
semantic.

So this document defines the artifact and [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) defines the code
that produces it. **If the two disagree, this one is right and the code is a bug.**

What "frozen" means: the *rules* below are frozen. Exact JSON field names are held against the
DocuShell companion spec, and where one is still unsettled this document says so rather than
inventing a name.

---

## 2. Artifact identity

Every artifact — classification, representation, grounding — carries the same four fields at the top
level, before any payload.

| Field | Rule |
| --- | --- |
| `artifact_type` | Names the shape exactly, e.g. `ethos.grounding.v1`. A reader that does not recognise it **fails closed** (§8) |
| `schema_version` | The version of *this shape*. Moves when the shape moves, independently of the parser |
| `parser_version` | The engine build that produced it. One shape can be emitted by many builds |
| `profile_sha256` | Hash of the pinned profile — every knob that can change output. **This is the identity that matters** |

**Profile-as-identity is the load-bearing idea.** Two artifacts are comparable if and only if their
`profile_sha256` matches. That single hash is what makes an OCR'd document non-comparable with a
born-digital parse *by contract* rather than by convention, what makes a backend swap visible, and
what turns "we changed the sample count" into a fingerprint event instead of a silent drift.

The profile must include at least: engine build identity, backend identity and version, the classify
sample count, the quantum (100 per point), the coordinate origin, the enabled capability set, and —
once they exist — the OCR engine identity, model hash and execution envelope. **Anything that can
change a byte of output belongs in the profile, or it is a bug.**

Two further identities are required, and they are not the same thing:

- **Source identity** — `sha256` of the exact original bytes. *What was read.*
- **Representation identity** — `sha256` of the canonical record produced under the pinned profile.
  *What was produced.* The verifier calls this `representation_sha256`.

---

## 3. Coordinate system — always declared on the wire

An artifact you cannot interpret without also having the source file is not evidence. Every artifact
carrying geometry declares:

```json
"coordinate_system": { "unit": "centipoint", "origin": "top-left" }
```

- **Never omit it**, even when the value is the only one the engine supports. A constant today is a
  discriminator tomorrow.
- **Never let the format imply it.** PDF's native origin is bottom-left and the artifact's is
  top-left. The transform is the engine's job, and the declaration is how a reader knows it happened.
- **A rotated page declares its rotation** (`0 | 90 | 180 | 270`), and geometry is expressed after
  rotation is applied.
- **A second coordinate system needs a new enum value, never a new default** — and that is a
  `schema_version` change.

## 4. Canonical JSON and integer quanta

The engine adopts the verifier's canonical-JSON rules wholesale. It is a solved problem; re-solving
it is pure risk.

| Rule | Detail |
| --- | --- |
| **Encoding** | UTF-8, no whitespace between tokens |
| **Key order** | Sorted by Unicode code point, explicitly at write time — never by map iteration order, because a `preserve_order` feature unification anywhere in the dependency graph would otherwise break every fingerprint silently |
| **Escaping** | Minimal: `"`, `\`, and U+0000–U+001F only. **No Unicode normalization** — extracted text is preserved exactly as extracted |
| **Numbers** | **Integers only.** Base-10, no leading zeros, no `+`, no exponent, \|n\| ≤ 2^53−1. Any non-integer anywhere in a canonical value is a hard error |
| **Arrays** | Order is semantic. Element order *is* reading order |
| **Idempotence** | `c14n(parse(c14n(v))) == c14n(v)`, property-tested |

**Floats do not exist in canonical output.** Geometry is quantized to integer centipoints, rounding
half away from zero, with `NaN`, `±Inf`, overflow and a zero quantum all errors rather than
saturating values.

**On `-0`:** the rule is about *output*, and it falls out of integers being the only representation.
It is not an input-normalization rule — the JSON text `-0` parses as a float and is rejected as a
non-integer rather than folded to `0`. Folding would mean silently accepting a float, which is the
one thing this layer exists to refuse.

**Volatile data is excluded.** Timings, memory, host and source paths live under a `diagnostics`
object that is outside the fingerprint and off by default. Nothing inside the payload may be
runtime-dependent.

**Geometry is excluded from the fingerprint**, deliberately — worth understanding before someone
"fixes" it. Text is anchored by stable origin locators, not by boxes, and two independent PDF stacks
agree on character origin to 0.001 pt while disagreeing on height by 6.174 pt. Fingerprinting the
boxes would make artifact identity hostage to the least reliable number in it.

---

## 5. Locators — origins are identity, boxes are inspection

### 5.1 The three locator kinds

| Locator | Required? | What it is |
| --- | --- | --- |
| **`NativeLocator`** | **Always, on every node** | The format's own address. For PDF: page, character origin, advance width, in integer centipoints |
| `StructuralLocator` | Where the node kind defines one | Structure-tree address: role path, `mcid`, table row and column |
| `RenderedLocator` / geometry | Optional | A box, for humans and crops; §5.3 says which kind |

`NativeLocator` is a discriminated union. Adding a format adds a variant plus an adapter profile,
fixtures and inspection behaviour. It does not change the source, run, artifact or verification
models.

**A rendered page or box is never substituted for the native address**, unless an approved format
profile declares that rendering authoritative. No such profile exists.

### 5.2 Typed absence

A missing box is a **type**, never a sentinel and never a substitute. Seven variants say *why* the
geometry is not there:

| Variant | Meaning |
| --- | --- |
| `NotReportedByReader` | The reader could not measure it. **Only this one counts toward the capability limitation** |
| `NotApplicableToKind` | This kind of node never has geometry |
| `NoInkToMeasure` | The node draws nothing, so there is nothing to measure |
| `CapabilityNotEnabled` | The profile has it switched off |
| `NotReportedByStructureTree` | The node came from the tag tree, which names no coordinate (v2-S24) |
| `MeasuredOffPage` | Measured, and the **document** draws it outside its own page, so no page-relative rectangle exists (D4-S5) |
| `NotAxisAligned` | Measured, and the run's baseline runs along neither axis, so no `[x0, y0, x1, y1]` equals its turned rectangle and a bounding box would claim area the text does not cover (0.58.0) |

Hard rules:

- **Never `height = font_size`.** A font-size-derived box is closer to invented than measured, and
  inventing a coordinate is forbidden. (One surveyed parser's `height` field is literally the same
  variable as its font size.)
- **Never a zero box, a null island, or a page-sized box** as a stand-in.
- **The consequence is stated, not hidden.** Without geometry on some nodes, crops and highlight
  rendering cannot be driven by this engine for those nodes.

**One open divergence from the DocuShell companion spec.** It models geometry as a plain optional
field and never names a variant for *why* it is absent. Typed absence carries strictly more
information and all four variants serialize down to "no geometry" on their wire cleanly, so this
engine keeps the richer type without inventing a competing field name. Pending review; blocks
nothing.

### 5.3 Declared box semantics

When a box *is* emitted, the artifact says **what kind of box it is**.

The failure to avoid: a surveyed parser emits loose em boxes — ascent-to-descent, not ink — sold as
"precise positioning", with nothing in the output saying which they are. For a line reading `acme`,
the box is as tall as if it contained `Ãj`. That is right for line grouping and wrong for a citation
highlight, and a consumer cannot tell which it got.

**The box this engine emits for a text run is not glyph ink.** Along the baseline it is the pen:
from the run's origin to where the pen stands after the run's last code, as a vector carried through
the text matrix, the CTM and the page's `/Rotate`. Each code — for a composite font, as the declared
interim `composite-font-codes-from-tounicode` divides the string — advances the pen once, by its own
width from `/Widths`, a composite font's `/W` and `/DW`, or the vendored AFM of a standard-14 face
the document names, with the text state applied as PDF 32000-1 §9.4.4 composes it. So a code that
decodes to several characters, such as a ligature, widens the box by one glyph, not by its letters.
Across the baseline it is the font's ascent and descent — from the embedded font program, the
descriptor's `/Ascent` and `/Descent` or its `/FontBBox`, or that AFM — scaled to the rendered em,
on the side the glyph tops point. Every glyph of a font gets the same two numbers, so **this is the
ascent-to-descent construction the paragraph above describes**, over the document's own advance.

A rectangle is emitted only where that baseline runs along a page axis, in either direction; a
baseline along neither gets `NotAxisAligned`, because no `[x0, y0, x1, y1]` equals the turned
rectangle. A run holding a code of no known width, a font with no ascent and descent — including a
Type 3 font whose `/FontMatrix` leaves its glyph space vertically other than the 1000-unit default,
where nothing in the font says which units its descriptor used — a run drawing only whitespace, and
a box the document draws off its own page each get a typed absence instead (§5.2). Where the reader
departs from this construction, that is a defect; [`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md)
§9 records the ones found.

A detected table's or cell's box is the rectangle its detection rule measured from the page's ink.
The table record names that rule, and the profile's `table_detection` names the rules in force.

**Open: the text-run box is not declared as what it is.** The tree calls it an *ink box*, and the
representation's `assurance.capabilities.measured_ink_boxes` says one was produced; that name is not
a declaration of kind, and the box is not ink. `ethos.grounding.v1` has no field for a box's kind
(§11). §6 places boxes from font metrics under a versioned rule, and the profile names none for this
one. Pending decision.

### 5.4 The geometric/structural cross-check

Where a node has both a geometric and a structural address, **derive them independently and test them
against each other**: overlapping cell regions, a cell outside its parent table, a grid that does not
tile the table area.

*A single locator can only be trusted or not; a pair can be tested.*

The result is a **typed diagnostic carrying a check version**, never a silent repair. Nodes carry
both address kinds wherever both exist, even in versions where nothing cross-checks them yet.

---

## 6. Derivation classes

Every node declares how it came to exist. This is the axis that lets OCR and model assist exist later
without laundering into born-digital certainty.

| Class | Meaning | May author | Notes |
| --- | --- | --- | --- |
| **`Extracted`** | Read from the source's own encoding | Text, origins, font identity, `mcid` | The only class v0 produces |
| **`Computed`** | Derived deterministically from `Extracted` values by a versioned rule | Reading order, line grouping, boxes from font metrics (§5.3) | The rule's version is part of the profile |
| **`Recognized`** | Produced by a recognition engine over pixels | OCR text and geometry | v4, own profile. **May author only on canvases where the deterministic reader found no text layer at all** |
| **`Proposed`** | Suggested by a model | Nothing citable, ever | v3. Never evidence |

Four hard rules:

1. **Nothing overwrites `Extracted`.** Not merged, not preferred, not reconciled. A surveyed parser
   merges OCR into the native text stream, discriminated only by an omittable nullable field. That is
   the bug.
2. **`Proposed` overwrites nothing** — not `Computed`, not `Recognized`, not another `Proposed`. A
   suggestion may sit beside evidence and may never replace it, including replacing an earlier
   suggestion a reviewer may already have seen. This is numbered separately because the first
   implementation enforced rule 1 and missed this one entirely.
3. **`Proposed` is never citable.** A chart description or a formula guess can exist in the tree and
   can never be the source of a verified quote.
4. **Different classes mean different profiles.** An OCR run has a different `profile_sha256`, so its
   output is non-comparable with a born-digital parse by contract, with no new machinery.

Rules 1 and 2 are the whole of the overwrite rule. The v4 constraint in the `Recognized` row governs
*where* a node may be placed, not which classes may replace which, so it is deliberately not encoded
as a class-pair rule.

---

## 7. Capabilities and limitations — the L1 gate

L1's achievement condition names capability declarations explicitly. **An artifact without them has
not reached L1.** They are not polish and not documentation.

Every artifact declares:

- **Capabilities** — what this profile can do.
- **Limitations** — what it could not do *on this document*, each one named.
- **Per-page processing state**, and a **coverage summary** reconciling authorized, processed,
  failed, unsupported, quarantined and not-attempted pages.

**Partial processing is a first-class outcome with its own terminal state.** A representation where
some pages failed may still support claims binding to pages that succeeded — but the gap must be
visible to every consumer, and a verification over a partly processed document must never render as
a clean verification of the whole document. A claim binding to a failed, unsupported or quarantined
page returns an explicit capability-limited result: **absence of extractable content is never
evidence of absence in the source.**

### 7.1 Wire spellings

Callers match on these, so they are named here rather than left to the implementation.

| Field | Shape |
| --- | --- |
| `assurance` | The envelope, on every classification and representation |
| `assurance.capabilities` | The producing profile's capability set |
| `assurance.limitations[]` | `{code, detail, scope}`, where `scope` is `{kind: profile\|document\|page, value?}` |
| `assurance.coverage` | `pages_authorized` plus five disposition buckets |
| `assurance.page_states[]` | `{index, state}` for **every** authorized page, 1-based |
| `assurance.terminal_state` | `complete` \| `partial` \| `refused` |

Four rules, each with a test rather than a convention behind it:

1. **No capability `true` without a named proof test, and no capability `false` without a declared
   limitation.** Both are exhaustiveness-gated, so adding a capability without covering it fails to
   compile.
2. **Every authorized page appears in `page_states`**, including ones deliberately never looked at.
   Listing only the exceptions would make "absent from the list" imply "processed" — a sentinel by
   omission.
3. **A page state's reason is a limitation code, not prose.** The prose lives once, in the matching
   limitation, so "every gap names a declared limitation" is checkable.
4. **The terminal state is derived from the page states, never asserted alongside them.** An artifact
   cannot claim `complete` while carrying a page nobody read.

`refused` is modelled but unreachable from a v0 artifact: a hard failure exits 2 with a named error
and no body, because a body would be a representation of a document nobody read. The variant exists
so a caller has a name for that outcome and does not reach for `partial`, which means something
materially different — **refused is "I read nothing", partial is "I read some of it, and here is
exactly which"**.

`not_attempted` is load-bearing. Bounded classification samples N pages and stops, so a 492-page
document at N=8 has 484 pages nobody observed. Folding them into `processed` would claim observations
nobody made; folding them into `failed` would claim failures that never happened.

---

## 8. Fail closed

The engine fails closed and says why. It never fails open, and it never fails silently.

| Situation | Behaviour |
| --- | --- |
| **Unrecognised content-stream operator** | Hard error naming the operator. The operator set is enumerated explicitly; anything outside it stops the parse |
| **Unknown `artifact_type` or `schema_version`** | Refuse to read. Never best-effort parse an unrecognised shape |
| **Missing capability for a requested operation** | An explicit capability-limited result. Never a stub, never a default, never a skip |
| **A number that will not quantize** | Error. Never saturate, never clamp |
| **Malformed cross-reference table or trailer** | Refuse, except the one bounded class in §8.1 |
| **Encrypted or password-protected source** | A distinct exit code. Never the same signal as "this document is complex" |

The measured case for the operator rule: a surveyed parser omits the `"` show-text operator from its
match. The text vanishes, surrounding runs merge with corrupt geometry, and the output is still
well-formed — undetectable downstream.

**Three outcomes get three exit codes.** Another surveyed parser returns the same code for
password-protected, invalid header, corrupt header **and** missing file as it does for "complex" — a
predicate that cannot tell *hard* from *I could not open this*. **Failing closed with an
indistinguishable signal is still a defect.**

### 8.1 The one repair

**Decision: repair, bounded and declared.** This is the only repair the engine performs.

v0 refused cross-reference entries of 19 bytes where the spec requires exactly 20 — roughly one valid
document in 26 on the conformance corpus, against a backend that repairs it.

| | |
| --- | --- |
| **The class** | Every entry in the table is 19 bytes, the specified trailing space missing |
| **The repair** | Pad each entry to 20 bytes, then parse normally. Nothing else is altered |
| **The knob** | `Profile::xref_repair`, defaulting to `pad-19-to-20-v1`; setting `refuse` restores v0 exactly |
| **The declaration** | Every artifact from a repaired open carries `xref-entry-padded`, document-scoped, with the entry count |

**Why a repair is admissible at all.** §10 says a repair is a recorded event or it is a fabrication.
This one is recorded three ways: in the profile hash, so a repairing build's artifacts are
non-comparable with a refusing build's; in a per-document limitation; and in that limitation's
detail, which states what was changed. Nothing is silent.

**Why it is safe — a stronger claim than "it works".** Padding grows the file, so bytes move, and a
cross-reference entry *is* a byte offset. Moving a byte that an offset points at would turn a refusal
into the one outcome this project refuses outright: a document that parses into the wrong objects and
produces a well-formed artifact that is silently wrong. So the repair runs only when nothing an
offset points at can move:

1. exactly one cross-reference table;
2. the trailer declares no `/Prev`, so no incremental-update chain reaches into moved bytes;
3. `startxref` names that table's own start offset;
4. **every** entry matches the 19-byte class — a mixed-stride table is worse repaired than refused;
5. every in-use offset precedes the table, so only its own tail, the trailer, `startxref` and `%%EOF`
   move, none of which is addressed by offset.

Any precondition failing reports the original parse error unchanged. The repair is a fallback — the
document is parsed as written first, so a well-formed file never reaches it — and encryption and
magic-number failures are answered before it.

**What was rejected: general recovery.** A reader that repairs whatever it can is useful, and it is
not this. It makes "the engine read it" stop implying "the document said it", and the whole artifact
contract rests on that implication. One named class with published preconditions can be argued with;
a recovery heuristic cannot.

---

## 9. No public confidence field

**No artifact this engine emits contains a public confidence float, score, grade, or any single field
summarising quality.** Not in classify, not in the representation, not in the grounding artifact.
A CI grep enforces it.

The measured case, produced by the exact feature under consideration — a surveyed parser on a plain
text-based PDF:

```
Type: TEXT-BASED (extractable text)
Confidence: 50%
Pages with text: 0            ← its own evidence disagrees with its verdict
OCR recommended: NO
```

Two routing rules a reasonable engineer would write, both wrong on this document: *confidence ≥ 0.7
means trust text extraction* sends a plainly text-based PDF to OCR, and *type is TEXT_BASED so skip
OCR* skips OCR on a document the detector itself says has no text.

**What replaces it:** counts and named reasons. Pages with text over pages sampled, per page. Reason
codes on two independent axes. A boolean derived from the reason list, never the other way round.
**The caller owns the policy; the engine owns the observation.**

### 9.1 The one place uncertainty is permitted

The companion spec allows an optional node confidence plus character-ranged low-confidence spans,
because a *recognition* processor genuinely has uncertainty, and hiding it at node granularity makes
a reviewer re-read a paragraph to find four doubtful characters. Its rules: uncertainty is
processor-owned and never verifier-owned; it must never appear in a verification report, alter a
deterministic result, or blend into a combined score; and **absence is not confidence** — a processor
reporting no uncertainty emits an absent field, never an implied `1.0`.

**This engine has no uncertainty to report.** Every node is `Extracted` by a deterministic reader,
with no recognition step, so the field is absent and §9's prohibition stands unqualified. When the
OCR lane lands at v4 it may populate span-level uncertainty as a **diagnostic** — recorded, and never
filtered on. The bug not to inherit: a surveyed parser drops text below 0.3 and again below 0.1,
silently, in two different places.

---

## 10. Synthesized-character honesty

Some characters in a parser's output were never in the document. A reader that inserts a space
between two runs because they looked like separate words has authored content. Unflagged, a verifier
will one day match a quote against a space the source does not contain.

| Rule | Detail |
| --- | --- |
| **Every synthesized character is flagged at emission** | Where it is created — not reconstructed later, not inferred from spacing |
| **Glyph codes travel with the text** | `char_codes` alongside the string, with the ligature caveat declared: ligature expansion yields more scalars than codes, so the arrays are not 1:1. `scalar_code_mismatch` is true when the count of Unicode scalar values and the count of codes differ — which a synthesized character also causes — so it compares counts; it is not a mapping |
| **A repair is a recorded event or it is a fabrication** | Any normalization the engine performs is declared in the profile and visible in the artifact. A surveyed parser silently repairs orphaned widgets in memory and always flattens widgets — an undeclared document mutation |
| **Hyphenation rejoin, if performed, is `Computed` and reversible** | The source bytes stay recoverable from the artifact |

---

## 11. The grounding adapter — `ethos.grounding.v1`

The canonical emit is `DocumentRepresentation v0`. The grounding adapter projects it into
`ethos.grounding.v1`, the shape the verifier consumes. That mapping is what makes the oracle test
possible.

The target schema is `additionalProperties: false` throughout, so the adapter emits exactly these
fields and nothing else:

| Field | Required | Shape |
| --- | --- | --- |
| `artifact_type` | ✓ | const `ethos.grounding.v1` |
| `schema_version` | ✓ | `1.0.0` paginated, `1.1.0` page-less |
| `source` | ✓ | `{media_type, sha256}` |
| `producer` | ✓ | `{name, version}` |
| `capabilities` | ✓ | `{spans, char_offsets, tables}` |
| `coordinate_system` | ✓ | `{unit: "centipoint", origin: "top-left"}` |
| `pages` | ✓ | `[{id, index≥1, width, height, rotation}]`, **1-indexed** — empty for page-less documents |
| `elements` | ✓ | `[{id, page, bbox, kind, text?}]` |
| `spans` | optional | `[{id, page, bbox, text, element?, char_start?, char_end?}]` |
| `tables` | optional | `[{id, page, bbox, cells:[…]}]` |

**`bbox` is `[x0, y0, x1, y1]`** — left, top, right, bottom — in integer centipoints. Not
`[x, y, w, h]`.

**`char_start` and `char_end` count Unicode scalars** into the owning element's text — not UTF-8
bytes, not UTF-16 code units (a JavaScript `text.length`), and not character codes. `char_start` is
inclusive and `char_end` exclusive, so `element.text` sliced by scalars over `char_start..char_end`
is exactly `span.text`, which is the rule the consuming validator applies. Every member of the
element counts, a run with no box included, because the element's text is every member's
concatenated; a space the reader synthesized is a character of its run's text and counts as one
(§10; PDF 32000-1 §9.4.3 — a `TJ` number shows no glyph, so a code index would be wrong). Both are
present on every span exactly when `capabilities.char_offsets` is true, and an artifact claims that
capability only while it carries spans.

**On zero-area boxes, the engine is deliberately stricter than the oracle.** The verifier's rectangle
constructor rejects only inverted boxes, so a degenerate `x0 == x1` box is accepted on the grounding
path, and the schema does not exclude it either. So this is not a shared error — it is something this
engine refuses to *emit* while the verifier would accept it. State it that way round, never as
"matching the verifier's fail-closed behaviour". **A future stricter emission rule must be checked
against this direction before it lands: the engine may refuse to emit what the oracle tolerates,
never the reverse.**

**Three things the adapter confronts honestly, none of them a bug in the adapter:**

1. **The target shape has no confidence field anywhere.** §9 is satisfied by the schema itself.

2. **`bbox` is required on every element, but §5.2 gives some nodes typed absence.** These conflict.
   **Decided: omit from grounding, count and declare.** Fabricating a box would sacrifice honesty;
   violating the schema would sacrifice validity. This way both survive.

   | Layer | Behaviour |
   | --- | --- |
   | `DocumentRepresentation v0` | **Node stays.** Geometry is typed absence; the native locator is still present |
   | `ethos.grounding.v1` | **Node omitted**, with a declared limitation and a count. Never `[0,0,0,0]`, never `height = font_size` |

   Two constraints this imposes: **omission is only ever for missing measurable geometry** — never
   because a classifier disliked a page, never as a quality filter — **or, since G2, for a string
   longer than the grounding schema admits**: an element whose text exceeds `ethos.grounding.v1`'s
   16,384 bytes (or a page-less locator its 2,048) is omitted, counted and declared under
   `elements-omitted-over-schema-limit`, because truncating it would put a quote on the wire the
   document does not contain. That is a measurement of length against a published limit, not a
   judgement of the text, and it is the only other reason. The omit-and-count path needs
   its own fixture with absent metrics. The type that makes the geometry omission enforceable takes
   a *measurement state* rather than a boolean, so that path is unreachable from a quality judgement
   by construction rather than by review. The length omission is not built that way: it is a fixed
   byte comparison against the published limit, held by `ethos-parser-grounding`'s schema-limit
   tests.

3. **The grounding shape is lossy relative to the representation.** It carries no derivation class, no
   `mcid`, no structural locator, no synthesized flags, no per-page coverage. That is expected — it is
   a verifier's input, not the canonical record. **The representation is the record; the grounding
   artifact is a projection.** Never treat a grounding round-trip as proof the representation is
   intact.

**The validation oracle** reports `structure` (valid/invalid), `source_binding`
(matched/mismatched/not_checked), `representation_sha256`, and `counts` of pages, elements, spans and
tables. Agreement on exactly those four is the oracle criterion. Measured: 12 of the 15 conformance
fixtures reach a grounding artifact and agree; the other 3 cannot be opened by this backend and are
asserted to fail closed. Both lists come from a live walk rather than a hardcoded set, so neither can
go stale quietly.

---

## 12. What this contract deliberately does not do

- **It does not verify.** No claim input, no verdict output, no `grounded` field, no evidence tier.
  The happy path ends at a *validated* artifact, not a *verified* one.
- **It does not describe a customer output schema, an extraction candidate, a resolved business
  value, a semantic assessment, or a reviewer decision.** Those are separate versioned resources that
  *reference* representation nodes. Keeping them out is what lets re-extraction and re-verification
  happen without reparsing, and what stops a model transcript from becoming the document record.
- **It does not define a Markdown projection.** Retrieval operates on the evidence record itself, and
  any projection between what is ranked and what is cited is where a locator dies silently. Markdown
  ships at v1.1 **with** the anchor map or not at all.
- **It does not promise global identifiers.** Ids are stable only within a representation created by
  the same pinned profile. A parser upgrade creates a new representation plus a mapping, rather than
  pretending node ids are permanently global.
- **It does not claim performance.** No "fastest", no "#1", no inherited latency figure.

---

## PR review checklist

- [ ] Every artifact carries `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`
- [ ] `coordinate_system` is present wherever there is geometry, never implied
- [ ] No float appears in canonical output; non-integers are a hard error
- [ ] Object keys are sorted explicitly at write time, not by map iteration order
- [ ] Every node has a `NativeLocator`
- [ ] No box is derived from a font size; absent metrics produce typed absence plus a declared limit
- [ ] Every node declares a derivation class
- [ ] Every nested object in a hashed type denies unknown fields — `deny_unknown_fields` is **not**
      recursive, and a dropped nested knob re-hashes to the unmodified digest
- [ ] Capabilities, limitations, per-page state and coverage summary are all present
- [ ] Unknown operator, unknown artifact type and unquantizable number all fail closed by name
- [ ] `grep -ri confidence` over emitted artifacts and their types returns nothing public
- [ ] Synthesized characters are flagged where they are created
- [ ] The grounding adapter emits exactly the schema's fields, with `bbox` as `[x0, y0, x1, y1]`
- [ ] No verification concept has leaked in: no claim, no verdict, no `grounded`, no evidence tier
