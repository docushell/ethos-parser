# 07 — The verify boundary

**Status:** bootstrap authority · **Read before touching anything that looks like verification.**

The single most likely way this project fails is not a bad parser. It is a second verification
authority appearing by accident — a helpful `is_grounded` field, a re-derived `evidence_tier`, a
"quick check" that becomes the thing people trust. This document exists to make that failure
visible before it is written.

---

## 1. Forced product decisions

Recorded here verbatim, identically to `00-NORTH-STAR.md` §2. These are settled.

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
| 11 | **OCR (document only):** none in v0; PP-OCR ONNX deterministic lane + LiteParse-style HTTP contract at v4; confidence diagnostic only, never filter; Tesseract never default; VLM/`Proposed` at v3. |
| 12 | **Optional agents:** allowed later as assist emitting `Proposed` only; never overwrite `Extracted`; never same processor identity for draft + evidence (Workbench rule 7). Out of v0. |
| 13 | **LibreOffice→PDF office bridge:** forbidden (invents pagination). |
| 14 | **No AGPL.** PDFium caller-provided or explicitly ADR'd later; v0 prefers clean-room `lopdf` + vendored CMap data (not wrapping pdf-inspector). |

---

## 2. Who owns what

| | **ethos-engine** | **the verifier (Ethos / Ethos-next)** |
| --- | --- | --- |
| **Question answered** | What does this document contain, and exactly where? | Did this claim come from this document? |
| **Trust levels** | L0 registered · L1 extracted · L2 locatable | **L3 grounded** |
| **Input** | Source bytes | A typed claim + a grounding artifact + a verification profile |
| **Output** | A representation and its projections | A verification report |
| **Never emits** | A verdict, a score, `grounded`, `evidence_tier`, or any field meaning "this document is good" | A parse. It does not read PDFs |
| **Determinism** | Byte-identical output under a pinned profile | Same result given the same evidence, claim, adapter version, and profile |

The engine has **no claim input**. That is the cleanest way to state the boundary: a component that
never receives a claim cannot decide whether one holds. If a design starts wanting a claim parameter,
the boundary is being crossed.

---

## 3. Why ethos-engine does not replace the Ethos repo

Four reasons, in descending order of how often they get forgotten.

**1. They answer different questions.** The engine produces evidence. The verifier judges claims
against evidence. Merging them means the thing that produced the evidence also rules on it — which is
the same structural error as Workbench rule 7, one level up.

**2. Exactly one Ethos integration is permitted.** `WORKBENCH_ARCHITECTURE.md` I.3 rule 3: the
Workbench never invokes Ethos directly, which guarantees exactly one integration exists. II.4
restates it as a must-not-break: two independent Ethos integrations are never permitted. An engine
that verifies would be the second.

**3. The contract was frozen first precisely so the verifier can be rebuilt.** Decision #2. If
Ethos-next is written for speed or a different architecture, the engine does not change — because the
engine was never built against the verifier's internals, only against the artifact contract. Merging
them destroys that property.

**4. BYO parsers stay first-class.** Decision #5. If the engine were the verifier, then the engine
would be the only path to verification, and the `GroundingSource` contract would decay into an
internal type. See §6.

---

## 4. Staged path

### Stage 0 — v0: the engine does not verify

**Shipped and frozen at M7.** The happy path terminates at a **validated** grounding artifact, not
a verified one:

```
classify → extract → ground → grounding-check
```

`grounding-check` validates **structure and source binding only** — never the verifier — and it has
a deterministic external oracle:

```bash
ethos grounding check <file> --source-artifact <pdf>
```

**Shipped at M6, and the criterion is now a passing test rather than a plan.** Of the 15
Ethos-owned fixtures, **12 reach a grounding artifact and both checkers agree** on `structure`,
`source_binding`, `representation_sha256` and `counts`; the other **3 cannot be read by this
backend at all** — a corrupt xref, an invalid header, an encrypted file — and a separate test
asserts each still exits 2 with no artifact, so the three are excluded *visibly* rather than
quietly. The harness fails if the two lists do not partition the corpus exactly.

**The count moved at v0.1, from 11/4.** The 19-byte-xref document that used to sit in the refused
list is now read: `docs/01-CONTRACT.md` §8.1 decided repair-or-refuse in favour of one bounded,
declared repair. Both lists come from a live walk of the corpus, so this number is measured on
every run rather than asserted here.

One correction to the wording this section used to carry: `grounding-check` is **not** "a
reimplementation of the JSON Schema validator only". The schema is necessary and not sufficient —
Ethos's parser enforces id uniqueness, reference resolution, page ordering, boxes inside their
page, capability/array agreement and character-offset validity, none of which JSON Schema can
express. The engine mirrors those rules too. That is still a long way short of verification, and
the line has not moved: no claim, no verdict, no `grounded`, no evidence tier, and a grep test in
the oracle harness enforces it.

**No verification _semantics_ exist in the tree.** The wording matters and changed at v0.1, so it
is worth being exact about what is and is not in here:

| In the tree | Not in the tree, and asserted so |
| --- | --- |
| A spawn shim — `engine_core::verifier` — that runs a verifier and forwards its bytes | Any type for a report, a claim, a check, an evidence tier, or a result |
| A `verify` subcommand and a `Profile::verifier` pin | Any code that reads, re-derives, summarizes or second-guesses what the verifier said |

Through v0 the sentence here read *"no verification code exists in the tree"*, and that was true
until Stage 1 shipped. It would be false now, and leaving it would be the kind of doc that
survives by nobody checking it. What has not changed is the rule underneath: **the engine has no
opinion about whether a document supports a claim**, and it cannot acquire one by accident,
because there is nothing in it that could hold such an opinion.

**Asserted in CI as of M7**, by the job `v0-no-verify` — `ci/forbidden-tokens.sh verification`, a
grep over `crates/*/src` for `evidence_tier`, `is_grounded`, `all_evidence_grounded`, `verdict`,
`verify_claim` and `claim_verified`. It is a job rather than a review item, `docs/03-V0-SCOPE.md`
§5 names it, and **it still passes with the shim in the tree** — which is the check that the shim
really is only a shim.

Bare `grounded` is deliberately **not** in that list. `GroundedBox` is real, supported API — the
type whose only constructor takes a measurement state, so a box cannot be built from a boolean —
and banning the substring would forbid the one type whose job is refusing to invent geometry. The
ban is on the verification *concept*, and a token list that fired on the mechanism enforcing an
adjacent rule would get itself disabled.

### Stage 1 — v0.1: shell out to the Ethos CLI

**Shipped.** Verification arrives as a **declared capability**, not as an implementation:

```bash
engine verify grounding.json --citations claims.json [--fail-on-ungrounded] [--config F] [--out F]
```

| Rule | How it is held |
| --- | --- |
| **Absence is a named error** | No verifier ⇒ exit 2, **no report on stdout**, a named `missing_part`. Never a skip, never a stub, never a default-pass. `ETHOS_BIN` is authoritative: set and unresolvable is a hard error, not a fallback |
| **Pin the verifier's identity** | `Profile::verifier` carries `ethos --version` **and** the sha256 of the binary's bytes. Both move `profile_sha256`, so a rebuild of the same version is as visible as a version change — a version-only pin would not have been |
| **Relay report bytes verbatim** | `engine verify` stdout is **byte-identical** to `ethos verify` with the same arguments, asserted on the grounded and the ungrounded path. `engine_core::verifier` has no type for a report, a claim, a check or a result: there is nothing to re-derive because nothing is read |
| **The gate is the product** | `--fail-on-ungrounded` exits 1 *and* writes the report. Without it the report is still written and the verifier's own exit status is forwarded — an ungrounded claim is never a silent skip either way |
| **1 and 2 never collapse** | "the verifier refused" and "the run did not happen" are different answers, kept apart for the same reason the classify exit codes are |
| **Spawn cost is acknowledged, not optimised** | ~19–22 ms measured floor. At the 20,000 docs/day design target (≈14/minute) it is irrelevant. No daemon, no socket, no cache |

The adapter id is **declared, not sniffed**: the engine passes `--grounding ethos-grounding-json`,
measured against the pinned binary rather than remembered. `ethos verify` can infer the type, but
declaring it turns a wrong input into a usage error instead of a silent fall back to native-document
loading — and a usage refusal relays as exit 2 with no report, which is the honest answer.

This matches the established estate pattern: the parse API already spawns the ODL Java CLI and the
Ethos Rust CLI once per invocation behind a byte-size admission gate.

**What Stage 1 did not do**, and the grep gate proves it: no report parsing, no re-derivation of
`all_evidence_grounded` or `capability_limits` or an evidence tier, no claim type anywhere in
`engine-pdf` or `engine-grounding`. The engine gained a way to *run* a verifier, not an opinion.

*(Still unmeasured, cheap spike: per-invocation `ethos verify` cost.)*

### Stage 2 — later: link `ethos-verify` as a crate

For OSS adopters with no Ethos binary. Two conditions, both hard:

- **Gated on byte-identical conformance** against Ethos's committed goldens
- **Off by default in any DocuShell build**, so "exactly one integration" survives

### Never — a second verify implementation

**Linking the same verifier is not a second integration. Reimplementing its semantics is.** The line
is not about process boundaries; it is about who owns the meaning of `grounded`.

---

## 5. What Ethos-next may own in-process later

A rewritten verifier may reasonably absorb what today costs a process spawn — the report generation,
the evidence binding, the profile resolution — and consume the same frozen contract in-process. That
is a verifier-side decision and it changes nothing here, which is the entire point of freezing the
contract first.

What it may **not** absorb: parsing. A verifier that parses is verifying against its own extract,
which is §8's second anti-pattern.

---

## 6. BYO parsers, forever

**The engine must not become the only path to verify.** This is a moat rule, not a courtesy.

- The `GroundingSource` contract stays **parser-neutral at the verification boundary**. Any parser
  that can produce a conforming artifact is a first-class citizen
- **The ODL adapter staying green is the continuous proof.** If a foreign adapter breaks and nobody
  notices, the path is already decorative
- A capability the engine has and the contract cannot express is a **contract gap to close**, not a
  reason to require the engine
- Three capability levels stay explicitly distinct, and "Ethos supports format X" is never said
  without naming which one:

| Level | Meaning |
| --- | --- |
| **Native parsing** | The engine accepts source bytes and creates the canonical representation itself |
| **Adapter-based verification** | A third-party parser exposes source-bound evidence through `GroundingSource`, and the verifier verifies against that |
| **First-class supported format** | Approved processor, native-locator profile, capability declarations, conformance fixtures, security review, UI inspection path, operating limits, customer support |

Architectural possibility, a local prototype, or generic parser output is **not** a supported-format
claim.

---

## 7. Optional lanes — OCR, agents, VLM

All of these are out of v0. The boundary is written now because the type system that makes them safe
is being built now (`01-CONTRACT.md` §6), and because a lane added later without these rules
contaminates everything upstream of it.

| Lane | Derivation | Profile | May author | Never |
| --- | --- | --- | --- | --- |
| Deterministic reader | `Extracted` | base | Text, origins, font identity, `mcid` | — |
| Deterministic rules | `Computed` | base (rule version pinned) | Reading order, line grouping, ink boxes | Overwrite `Extracted` |
| **OCR (v4)** | `Recognized` | **own** (`ethos-ocr-v1`) | Nodes **only on canvases where the deterministic reader found no text layer at all** | Overwrite `Extracted`. Merge into the native stream. Filter on confidence |
| **VLM / assist (v3)** | **`Proposed`** | own | Suggestions | Be citable. Be evidence. Overwrite anything |

**Four rules that make this safe:**

1. **Fingerprint isolation is free.** A different `profile_sha256` makes an OCR'd document
   non-comparable with a born-digital parse *by contract*, with no new machinery.
2. **Confidence is a diagnostic, never a filter.** Accept it if a server sends it, record it, never
   act on it. LiteParse drops text below 0.3 and again below 0.1, silently, in two places.
3. **Reason codes trigger OCR, not scores.** The trigger is `no-text` or `scanned` plus per-page
   counts — a named observation, not a threshold crossing.
4. **Rule 7 is enforced mechanically.** Every lane declares a **processor identity** in its processing
   run. A run whose drafting model and representation processor share an identity is **rejected
   mechanically**, not caught in review. This is the cheap precondition that makes a VLM lane safe to
   build at all — cheaper than the byte-diff CI it supplements.

**Why rule 7 matters more as models get better:** if extraction moves to a VLM that reads page pixels
directly, extraction and parsing become the same step — and verification against the parser output
becomes verification against the model's own work. The failure looks like success.

---

## 8. Anti-patterns

Named so they can be pointed at in review.

**1. The parser becomes the product.** Symptoms: the engine grows a "quality" field; a demo shows the
engine alone answering whether a document is trustworthy; the verifier becomes an optional
post-processing step. The engine is infrastructure. The pair is the product. *If a single field
summarising document quality appears in a diff, that is this anti-pattern in its first form.*

**2. Verifying against the model's own extract.** Symptoms: the same model or processor identity
drafts the claim and produces the representation; a VLM reads the pixels and also answers the
question. Workbench rule 7. The mechanical guard is the processor-identity check in §7.

**3. Silent OCR merge.** Symptoms: OCR text in the same array as extracted text, discriminated by a
nullable field; a merged stream where derivation is recoverable only by field absence; low-confidence
text dropped before anyone sees it. The guard is `DerivationClass` plus profile isolation, both
non-optional.

**4. Re-deriving verifier outputs.** Symptoms: the engine reads an Ethos report and computes its own
`all_evidence_grounded`, or recomputes `evidence_tier` "for convenience." **This is how a second
authority is born by accident** — nobody decides to build one. Relay bytes verbatim or do not relay.

**5. Verification-shaped helpers.** Symptoms: `is_grounded()`, `looks_verified`, `quick_check()`, a
`verify` feature flag, a module named `verification` with a `TODO`. Each is individually harmless and
collectively the whole failure.

**6. Silent fallback when the verifier is absent.** Symptoms: a skipped test, a stub report, a
default-pass. Absence is a **named error**. A raw extraction may be delivered as L1/L2 with grounding
marked `not_run` or `failed` where policy permits — **never released as grounded**.

---

## PR review checklist

- [ ] No claim parameter has appeared in any engine API
- [ ] No `grounded`, `evidence_tier`, `verified`, or quality-summary field anywhere
- [ ] `grounding-check` still validates structure and binding **only**
- [ ] Nothing is re-derived from an Ethos report; bytes are relayed verbatim or not at all
- [ ] Verifier absence produces a named error, never a skip or a stub
- [ ] New lane ⇒ declared derivation class **and** its own profile
- [ ] `Recognized` authors only where no text layer exists; `Proposed` is never citable
- [ ] Processor identity is declared per lane, and same-identity draft+evidence is rejected
      mechanically
- [ ] The `GroundingSource` path still works for a foreign parser — the ODL adapter is green
- [ ] The Ethos tree is unmodified
