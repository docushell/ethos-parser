# 04 — Architecture

**Rule:** if this document and [`01-CONTRACT.md`](01-CONTRACT.md) disagree, the contract is right and
this is a bug.

---

## 1. Workspace layout

```
ethos-parser/
├── Cargo.toml              # workspace, MSRV 1.88
├── deny.toml               # permissive licences only, no network crates
├── crates/
│   ├── ethos-parser-core/        # representation types, canonical JSON, quanta, ids, capabilities, profile
│   ├── ethos-parser-pdf/         # lopdf: classify, text runs, font metrics, encoding tables
│   ├── ethos-parser-office/      # eight office formats
│   ├── ethos-parser-grounding/   # representation → ethos.grounding.v1, plus the validator
│   └── ethos-parser-cli/         # nine subcommands; tests/oracle.rs lives here
├── vendor/README.md        # what is carried, and what deliberately is not
├── fixtures/               # a manifest referencing four corpus roots
└── docs/
```

**A crate exists when a boundary needs enforcing** — not when a module gets large. That is why
`ethos-parser-office` was added rather than a DOCX module going into `ethos-parser-pdf`, and why
`engine-ocr` does not exist yet.

**The oracle test lives with the CLI, not at the workspace root.** Cargo only builds integration
tests for packages, so a root `tests/` directory in a virtual workspace would be silently ignored —
the worst possible failure for a harness whose job is to fail loudly. It also belongs there on the
merits: the oracle drives one CLI and compares against another.

### Crate boundaries, as rules rather than preferences

| Crate | May depend on | Must never contain |
| --- | --- | --- |
| `ethos-parser-core` | nothing in this workspace | Any PDF concept — no `lopdf`, no operator, no page-tree type. And no office concept: no zip, no XML reader, no part name it parses |
| `ethos-parser-pdf` | `ethos-parser-core` | Any grounding, verification or office concept |
| `ethos-parser-office` | `ethos-parser-core` | Any PDF or grounding concept. It reads eight formats into the shared representation; the seven package formats share the ZIP reader and the XML plumbing and nothing else |
| `ethos-parser-grounding` | `ethos-parser-core` | Any **format** concept at all — it projects the *representation*, never a document |
| `ethos-parser-cli` | all four | Any logic. It parses arguments, calls the library, and maps errors to exit codes |

**"Any PDF concept" means machinery, not vocabulary.** The contract defines `NativeLocator` as a
discriminated union with a `PdfLocator` variant, and `ethos-parser-core` owns the contract, so the
line is:

| In `ethos-parser-core` | Verdict |
| --- | --- |
| `lopdf`, a content-stream operator, a page tree, a font program, an xref table | **Forbidden** — machinery, which teaches the crate to read one format |
| A contract-defined locator variant carrying integers, or a format name as a string | **Permitted** — data and a discriminant. Nothing here can parse anything |

`ethos-parser-grounding` is held to the **stronger** rule, and it is enforced rather than hoped for:
the projection addresses pages by node id, so it never reads a locator, and a test fails if the crate
so much as mentions `NativeLocator`. That is what actually keeps adding a format cheap — hiding the
type in `ethos-parser-pdf` would have kept the letter of the rule while leaving the projection free
to match on it.

**This held up when it was tested.** When DOCX arrived, `ethos-parser-grounding`, the MCP server and
both SDKs were untouched. What did move was a `ethos-parser-core` invariant — see §6.

## 2. CLI surface

**This section is v0's record, not today's surface.** v0 froze four subcommands and they are left at
four deliberately; rewriting them would erase what v0 committed to. The five that came later were
each argued in the scope document of the version that added them: `verify` at v0.1, `overlay` at
v1-S6, `markdown` at v1.1-S1, `html` at v1.1-S4, `mcp` at v1.2-S1. `enum Command` in
`crates/ethos-parser-cli/src/main.rs` is the list that cannot go stale.

**The CLI is a thin shell over the library**, so the two cannot diverge. Every subcommand is a
library call plus argument parsing plus an exit-code mapping — and that rule binds all nine, not just
the four below.

| Command | Input | Output | Exit codes |
| --- | --- | --- | --- |
| `classify <pdf>` | PDF | Per-page counts, reason codes on two axes, a derived boolean | 0 simple · 1 needs attention · 2 could not read |
| `extract <pdf>` | PDF | `DocumentRepresentation v0` | 0 ok · 2 could not read |
| `ground <representation>` | representation JSON | `ethos.grounding.v1` | 0 ok · 2 refused |
| `grounding-check <grounding.json> [--source-artifact <pdf>]` | grounding JSON | A validation report | 0 valid · 1 invalid · 2 could not read |

`--source-artifact` mirrors the Ethos CLI deliberately: the oracle runs both with the same flag, which
keeps the comparison obvious.

**Default output is byte-identical across runs.** Volatile diagnostics are opt-in behind
`--diagnostics` and excluded from the fingerprint, so a default invocation produces identical
*files*, not merely identical payloads.

### 2.1 One document load

`classify` and `extract` in one invocation **parse the document once and share it**. This is a
correctness rule before it is a performance one: two loads can disagree, and a classifier that saw a
different object graph from the extractor is a silent divergence with no diagnostic.

In practice the library exposes an opened-document handle, `classify` and `extract` take it by
reference, and **only the CLI decides when to open a file.** Nothing below the CLI opens one.

## 3. Profile as identity

The profile is the pinned set of every knob that can change output. Its `sha256` goes in every
artifact, and it is what gives OCR isolation, backend isolation and comparability for free — with no
machinery beyond one hash.

It must include at least: engine build identity and `parser_version`; backend identity and version;
the classify sample count; the quantum and coordinate origin; the enabled capability set; the
reading-order rule version; the vendored data version; and later, the OCR engine identity, model
hash and execution envelope.

**The test that keeps this honest:** changing any profile field must change `profile_sha256`, and a
test asserts that field by field. A knob that does not move the hash is a silent-drift bug waiting to
happen.

Ids are stable only within a representation produced by the same pinned profile. A profile change
creates a new representation plus a mapping — never a pretence that node ids are globally stable.

## 4. Fixtures and the oracle

**Fixtures are referenced, not copied.** `fixtures/manifest.json` names each one by path and
`sha256`. Copying invites drift; a hash-pinned manifest makes a fixture change a visible event.

**Four roots, each independently overridable** — conformance, benchmark, engine-owned and the table
gate corpus. Separate roots keep benchmark and engine-owned documents from inflating the 15 the
oracle criterion counts. See [`fixtures/README.md`](../fixtures/README.md).

**Engine-owned fixtures are enumerated, not open.** Where the Ethos corpus has no fixture for a
behaviour this engine must test, the engine authors one under CC0 and marks it in the manifest with a
note saying which behaviour upstream could not cover. `counts.engine_owned` in the manifest is where
the number lives, so no prose can drift from it. Adding another needs the same justification — the
upstream corpus genuinely cannot cover it — not merely convenience.

**The oracle is `ethos grounding check`.** The engine's own `grounding-check` reimplements the
validator only, never the verifier, and it has a deterministic external oracle to agree with:

```bash
ethos grounding check <file> --source-artifact <pdf>
```

`crates/ethos-parser-cli/tests/oracle.rs` runs both across all 15 fixtures and asserts byte-identical
agreement on structure, source binding, representation hash and counts. **The Ethos binary is a
test-time dependency, and its absence fails the test loudly — it never degrades to a skip.**

**Test layers, all of them cheap:**

| Layer | What it catches |
| --- | --- |
| Golden artifacts per fixture | Any change to emitted bytes, intended or not |
| Double-run byte identity | Nondeterminism: map ordering, timestamps, addresses, float drift |
| Oracle agreement | Divergence from the verifier's reading of the same artifact |
| `cargo-fuzz` on the entry points | Panics, hangs and fail-open paths on malformed input |
| Mutation testing over fixtures | Assertions that pass for the wrong reason |
| Profile-field sensitivity | A knob that does not move `profile_sha256` |
| `cargo deny` | Licence and dependency-posture regressions |

## 5. Dependency posture

| Layer | Decision | Why |
| --- | --- | --- |
| **Object and xref layer** | Depend on `lopdf` | The commodity part |
| **Encoding tables** | Vendor as data, with attribution | They do not churn, and regenerating them is pure cost |
| **Content-stream interpreter, classification, layout** | **Clean-room** | See below |
| **Font metrics** | `ttf-parser` over the embedded font program | For measured ink boxes, with a descriptor fallback and typed absence beyond that |
| **PDFium** | Not present. If ever, caller-provided and hash-pinned under an explicit ADR | Never a build-time download |
| **AGPL, anywhere** | **Forbidden**, enforced by `cargo deny` | Decision #14 |
| **Network crates** | **Forbidden** | Nothing in the happy path reaches the network |

**Why another parser is reference-only rather than a dependency**, since this gets re-litigated: the
parts worth wrapping are exactly the parts that would have to be un-wrapped. In the surveyed
codebase, `height` is literally the same variable as the font size, `y` is a baseline the JSON never
labels, and the classifier's confidence contradicts its own counts. Depending on it means inheriting
types whose fields mean something other than their names, in a project whose entire product is not
doing that. Its licence lets us read the code either way.

**What that leaves as transferable ideas:** rectangle detection, encoding-issue detection, the
one-document-load rule, and the marked-content bridge. Take those; write them.

## 6. How later lanes plug in without contaminating what exists

The point of the contract's type system is that later versions need **no new mechanism**, only new
values.

| Lane | Plugs in as | Impact on what exists |
| --- | --- | --- |
| **OCR (v4)** | A node source authoring `Recognized` nodes only where the deterministic reader found no text layer, under its own profile | None. The derivation class exists and the profile hash already isolates |
| **Assist (v3)** | Authors `Proposed` nodes only. Never overwrites, never citable | None |
| **A new format (v2)** | A new locator variant, adapter profile, fixtures and inspection behaviour | None downstream — provided `ethos-parser-grounding` never learned about pages |
| **A second backend** | A trait seam with backend identity in the profile | Design the seam early; implement one side |

**What the new-format row cost when it was actually paid.** It was right about downstream: grounding,
MCP and both SDKs were untouched. It missed one line item — **`ethos-parser-core`'s invariant that
every node's parent is a declared page**, which had to be split by locator family. The page
assumption was never in grounding; it was in the contract types, where a page-less document could not
become a representation at all.

**The processor-identity rule, designed now and implemented when assist exists:** every lane declares
its processor identity in the processing run, and a run whose drafting model and representation
processor share an identity is **rejected mechanically** rather than caught in review. That is
cheaper than the byte-diff CI it supplements, and it is the precondition that makes a model-assist
lane safe to build at all.

## 7. What this architecture refuses

- **No verification code, type or field anywhere in the tree** — not a stub, not a feature flag, not
  a `TODO`-shaped module.
- **No crate wrapping a competitor's parser as the grounded PDF core.**
- **No build-time network access.** Not for a renderer, not for data, not for models.
- **No logic in `ethos-parser-cli`.** If a behaviour can only be exercised through the CLI, it is in
  the wrong crate and the library is incomplete.
- **No PDF concept in `ethos-parser-core` or `ethos-parser-grounding`.**

---

## PR review checklist

- [ ] Crate boundaries respected — no format type in `core` or `grounding`, no logic in `cli`
- [ ] One document load per invocation; nothing below the CLI opens a file
- [ ] A new knob is added to the profile, and the sensitivity test updated
- [ ] Fixtures referenced by hash-pinned manifest, never copied or modified
- [ ] Oracle test green across all 15 fixtures; failure is loud, never a skip
- [ ] `cargo deny` green — no AGPL, no network crates
- [ ] A new dependency is justified against §5, with a licence named
- [ ] No verification concept has appeared
- [ ] A default invocation still produces byte-identical files across two runs
