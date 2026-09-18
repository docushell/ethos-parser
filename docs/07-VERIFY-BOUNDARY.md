# 07 — The verify boundary

**Read this before touching anything that looks like verification.**

The likeliest way this project fails is not a bad parser. It is a **second verification authority
appearing by accident** — a helpful `is_grounded` field, a re-derived evidence tier, a "quick check"
that becomes the thing people trust. This document exists to make that failure visible before it is
written.

---

## 1. The decisions behind this boundary

They live in [`00-NORTH-STAR.md`](00-NORTH-STAR.md) §2 and are deliberately **not** copied here. A
second copy of that table went stale once already — it held 14 of the rows while claiming to hold
them all — which is what a second copy of a table does.

## 2. Who owns what

| | **ethos-parser** | **the verifier** |
| --- | --- | --- |
| **Question answered** | What does this document contain, and exactly where? | Did this claim come from this document? |
| **Trust levels** | L0 registered · L1 extracted · L2 locatable | **L3 grounded** |
| **Input** | Source bytes | A typed claim, a grounding artifact, and a verification profile |
| **Output** | A representation and its projections | A verification report |
| **Never emits** | A verdict, a score, `grounded`, an evidence tier, or any field meaning "this document is good" | A parse. It does not read PDFs |
| **Determinism** | Byte-identical output under a pinned profile | The same result given the same evidence, claim, adapter version and profile |

**The engine has no claim input.** That is the cleanest way to state the boundary: a component that
never receives a claim cannot decide whether one holds. If a design starts wanting a claim parameter,
the boundary is being crossed.

## 3. Why this engine does not replace the verifier

Four reasons, in descending order of how often they get forgotten.

1. **They answer different questions.** The engine produces evidence; the verifier judges claims
   against it. Merging them means the thing that produced the evidence also rules on it.
2. **Exactly one verifier integration is permitted.** The Workbench never invokes the verifier
   directly, which is what guarantees exactly one integration exists. An engine that verified would
   be the second.
3. **The contract was frozen first precisely so the verifier can be rebuilt.** If a future verifier
   is written for speed or a different architecture, this engine does not change — because it was
   never built against the verifier's internals, only against the artifact contract. Merging them
   destroys that property.
4. **Bring-your-own parsers stay first class.** If the engine were the verifier, it would be the only
   path to verification, and the grounding-intake contract would decay into an internal type.

## 4. The staged path

### Stage 0 — the engine does not verify

The happy path ends at a **validated** grounding artifact, not a verified one:

```
classify → extract → ground → grounding-check
```

`grounding-check` validates **structure and source binding only**, and it has a deterministic
external oracle to agree with. Of the 15 conformance fixtures, 12 reach a grounding artifact and both
checkers agree; the other 3 cannot be read by this backend at all, and a separate test asserts each
still exits 2 with no artifact — so the three are excluded *visibly* rather than quietly. The harness
fails if the two lists do not partition the corpus exactly.

**A precision worth keeping:** `grounding-check` is not "the JSON Schema validator only". The schema
is necessary and not sufficient — id uniqueness, reference resolution, page ordering, boxes inside
their page, capability agreement and character-offset validity are none of them expressible in JSON
Schema, and the engine mirrors those rules too. That is still a long way short of verification, and
the line has not moved: **no claim, no verdict, no `grounded`, no evidence tier.**

**What is and is not in the tree**, stated exactly, because the wording changed once when Stage 1
shipped:

| In the tree | Not in the tree, and asserted so |
| --- | --- |
| A spawn shim that runs a verifier and forwards its bytes | Any type for a report, a claim, a check, an evidence tier, or a result |
| A `verify` subcommand and a verifier pin in the profile | Any code that reads, re-derives, summarizes or second-guesses what the verifier said |
| A `locate` subcommand, MCP tool and SDK function that answer **where a string lies** in a representation (v2.3, decision #30) | Any field, summary phrase or exit code on them that answers *whether* — and a match rule copied from the verifier's or tuned to agree with it |

**The third row is the closest this engine has come to the line, and it is worth saying why it does
not cross it.** `locate` takes a representation and a string. A string is not a claim, and this
document's own §2 is what the design is arranged around rather than what it argues past: there is no
claim parameter, so there is nothing to decide. What makes it checkable rather than merely stated is
that a string occurring nowhere is exit 0 with an empty list — the same artifact, the same code —
so no caller can read an answer out of the failure channel. `docs/26-LOCATE-SCOPE.md` §4.2 tabulates
the five places the verifier resolves a quote differently and §7 names every verdict-shaped
convenience that is refused, each with the grep or test that holds it.

The rule underneath has not changed: **the engine has no opinion about whether a document supports a
claim, and it cannot acquire one by accident, because there is nothing in it that could hold such an
opinion.**

**Enforced in CI** by `ci/forbidden-tokens.sh verification`, a grep over `crates/*/src` for
`evidence_tier`, `is_grounded`, `all_evidence_grounded`, `verdict`, `verify_claim` and
`claim_verified`. It is a job rather than a review item, and **it still passes with the shim in the
tree** — which is the check that the shim really is only a shim.

Bare `grounded` is deliberately **not** on that list. `GroundedBox` is real, supported API — the type
whose only constructor takes a measurement state, so a box cannot be built from a boolean — and
banning the substring would forbid the one type whose job is refusing to invent geometry. The ban is
on the verification *concept*, and a token list that fired on the mechanism enforcing an adjacent
rule would get itself disabled.

### Stage 1 — shell out to the verifier CLI

Verification arrives as a **declared capability**, not an implementation:

```bash
ethos-parser verify grounding.json --citations claims.json [--fail-on-ungrounded]
```

| Rule | How it is held |
| --- | --- |
| **Absence is a named error** | No verifier means exit 2 with **no report on stdout**. Never a skip, never a stub, never a default-pass. `ETHOS_BIN` is authoritative: set and unresolvable is a hard error, not a fallback |
| **The verifier's identity is pinned** | The profile carries its version **and** the sha256 of its bytes. Both move `profile_sha256`, so a rebuild of the same version is as visible as a version change — a version-only pin would not have been |
| **Report bytes are relayed verbatim** | Stdout is byte-identical to running the verifier directly, asserted on both the grounded and ungrounded paths. There is no type for a report, so there is nothing to re-derive |
| **The gate is the product** | `--fail-on-ungrounded` exits 1 *and* writes the report. Without it the report is still written and the verifier's own status is forwarded — an ungrounded claim is never a silent skip either way |
| **1 and 2 never collapse** | "The verifier refused" and "the run did not happen" are different answers |
| **Spawn cost is acknowledged, not optimised** | A measured floor of about 20 ms. At the design throughput that is irrelevant. No daemon, no socket, no cache |

**The adapter id is declared, not sniffed.** The verifier can infer the input type, but declaring it
turns a wrong input into a usage error instead of a silent fall back to native-document loading — and
a usage refusal relays as exit 2 with no report, which is the honest answer.

**What Stage 1 did not do**, and the grep gate proves it: no report parsing, no re-derivation of any
verifier field, no claim type anywhere. **The engine gained a way to *run* a verifier, not an
opinion.**

### Stage 2 — later: link the verifier as a crate

For adopters with no verifier binary. Two hard conditions: **gated on byte-identical conformance**
against the verifier's own goldens, and **off by default** in any first-party build, so "exactly one
integration" survives.

### Never — a second verify implementation

**Linking the same verifier is not a second integration. Reimplementing its semantics is.** The line
is not about process boundaries; it is about who owns the meaning of *grounded*.

## 5. What a future verifier may own in-process

A rewritten verifier may reasonably absorb what today costs a process spawn — report generation,
evidence binding, profile resolution — and consume the same frozen contract in-process. That is a
verifier-side decision and it changes nothing here, which is the entire point of freezing the
contract first.

What it may **not** absorb is **parsing**. A verifier that parses is verifying against its own
extract, which is §8's second anti-pattern.

## 6. Bring-your-own parsers, forever

**The engine must not become the only path to verification.** This is a rule, not a courtesy.

- The grounding-intake contract stays **parser-neutral at the verification boundary**. Any parser
  that can produce a conforming artifact is a first-class citizen.
- **A foreign adapter staying green is the continuous proof.** If one breaks and nobody notices, the
  path is already decorative.
- A capability the engine has and the contract cannot express is a **contract gap to close**, not a
  reason to require the engine.
- Three capability levels stay distinct, and "we support format X" is never said without naming
  which:

| Level | Meaning |
| --- | --- |
| **Native parsing** | The engine accepts source bytes and creates the canonical representation itself |
| **Adapter-based verification** | A third-party parser exposes source-bound evidence, and the verifier verifies against that |
| **First-class supported format** | Approved processor, native-locator profile, capability declarations, conformance fixtures, security review, inspection path, operating limits, support |

An architectural possibility, a local prototype, or generic parser output is **not** a
supported-format claim.

## 7. Optional lanes — OCR, agents, VLM

All of these are out of scope today. The boundary is written now because the type system that makes
them safe is being built now, and because a lane added later without these rules contaminates
everything upstream of it.

| Lane | Derivation | Profile | May author | Never |
| --- | --- | --- | --- | --- |
| Deterministic reader | `Extracted` | base | Text, origins, font identity, marked-content ids | — |
| Deterministic rules | `Computed` | base, rule version pinned | Reading order, line grouping, ink boxes | Overwrite `Extracted` |
| **OCR (v4)** | `Recognized` | **its own** | Nodes **only on canvases where the deterministic reader found no text layer at all** | Overwrite `Extracted`. Merge into the native stream. Filter on confidence |
| **Assist (v3)** | **`Proposed`** | its own | Suggestions | Be citable. Be evidence. Overwrite anything |

**Four rules that make this safe:**

1. **Fingerprint isolation is free.** A different `profile_sha256` makes an OCR'd document
   non-comparable with a born-digital parse by contract, with no new machinery.
2. **Confidence is a diagnostic, never a filter.** Accept it if a server sends it, record it, never
   act on it.
3. **Reason codes trigger OCR, not scores.** The trigger is a named observation plus per-page counts,
   not a threshold crossing.
4. **Processor identity is enforced mechanically.** Every lane declares one in its processing run, and
   a run whose drafting model and representation processor share an identity is **rejected
   mechanically**, not caught in review.

**Why that last rule matters more as models get better:** if extraction ever moves to a model that
reads page pixels directly, extraction and parsing become the same step — and verification against
the parser output becomes verification against the model's own work. **The failure looks like
success.**

## 8. Anti-patterns

Named so they can be pointed at in review.

1. **The parser becomes the product.** Symptoms: the engine grows a "quality" field; a demo shows the
   engine alone answering whether a document is trustworthy; the verifier becomes an optional
   post-processing step. The engine is infrastructure; the pair is the product. *A single field
   summarising document quality appearing in a diff is this anti-pattern in its first form.*
2. **Verifying against the model's own extract.** Symptoms: the same processor identity drafts the
   claim and produces the representation; a model reads the pixels and also answers the question. The
   mechanical guard is the processor-identity check above.
3. **Silent OCR merge.** Symptoms: OCR text in the same array as extracted text, discriminated by a
   nullable field; low-confidence text dropped before anyone sees it. The guard is the derivation
   class plus profile isolation, both non-optional.
4. **Re-deriving verifier outputs.** Symptoms: the engine reads a report and computes its own summary
   field "for convenience". **This is how a second authority is born by accident** — nobody decides
   to build one. Relay bytes verbatim or do not relay.
5. **Verification-shaped helpers.** Symptoms: `is_grounded()`, `looks_verified`, `quick_check()`, a
   `verify` feature flag, a module named `verification` with a `TODO`. Each is individually harmless
   and collectively the whole failure.
6. **Silent fallback when the verifier is absent.** Symptoms: a skipped test, a stub report, a
   default pass. Absence is a **named error**. A raw extraction may be delivered as L1/L2 with
   grounding marked not-run or failed where policy permits — **never released as grounded**.

---

## PR review checklist

- [ ] No claim parameter has appeared in any engine API
- [ ] No `grounded`, evidence tier, `verified`, or quality-summary field anywhere
- [ ] `grounding-check` still validates structure and binding **only**
- [ ] Nothing is re-derived from a verifier report; bytes are relayed verbatim or not at all
- [ ] Verifier absence produces a named error, never a skip or a stub
- [ ] A new lane declares a derivation class **and** its own profile
- [ ] `Recognized` authors only where no text layer exists; `Proposed` is never citable
- [ ] Processor identity is declared per lane, and same-identity draft-plus-evidence is rejected
      mechanically
- [ ] A foreign parser's path still works
- [ ] The Ethos tree is unmodified
