# 04 — Architecture

**Status:** bootstrap authority · **Baseline:** memo §16.12 · **Scope:** v0 only
**Rule:** if this document and `01-CONTRACT.md` disagree, the contract is right and this is a bug.

---

## 1. Workspace layout

```
ethos-engine/
├── Cargo.toml              # workspace, MSRV 1.88, resolver 2
├── rust-toolchain.toml     # channel = "1.88.0"
├── deny.toml               # permissive-only licences, no network crates
├── NOTICE                  # reserved for the vendored Adobe CMaps
├── crates/
│   ├── engine-core/        # representation types, c14n, quanta, ids, capabilities, profile
│   ├── engine-pdf/         # lopdf + vendored CMaps: classify, text runs, font metrics
│   ├── engine-grounding/   # representation → ethos.grounding.v1 + the validator
│   └── engine-cli/
│       ├── src/            # classify | extract | ground | grounding-check
│       └── tests/oracle.rs # byte-identical agreement with `ethos grounding check`
├── vendor/cmaps/           # 168 Adobe .bcmap + NOTICE
├── fixtures/               # manifest referencing the Ethos conformance corpus (read-only)
└── docs/                   # this tree
```

**The oracle test lives with the CLI, not at the workspace root.** Memo §16.12 sketches
`tests/oracle.rs` at the root, but cargo only builds integration tests for *packages*, and this is a
virtual workspace — a root `tests/` directory would be silently ignored, which is the worst possible
failure for a harness whose job is to fail loudly. `crates/engine-cli/tests/` is also where it
belongs on the merits: the oracle drives the CLI and compares against another CLI. Ethos does the
same (`crates/ethos-cli/tests/verify.rs`).

**Four crates, not six — DECIDED (2026-08-12).** Memo §16.12 says "six crates, deliberately small"
and then lists four; the tree it lists is right and the prose is a slip. `core` owns everything the
contract defines, `pdf` owns everything one format needs, `grounding` owns the projection and its
validator, `cli` owns argument parsing and exit codes.

**A fifth crate before a second format is speculative structure.** Revisit only when office or OCR
needs a real home — `engine-office`, `engine-ocr` — and not before. "This module is getting large" is
not a reason to add a crate.

**v2-S2 is that revisit, and `engine-office` exists.** DOCX is the second format, so the crate stops
being speculative and starts being the only place OOXML may live. `engine-ocr` is still hypothetical
and still refused on the same rule.

**Crate boundaries as rules, not preferences:**

| Crate | May depend on | Must never contain |
| --- | --- | --- |
| `engine-core` | nothing in this workspace | Any PDF concept. No `lopdf`, no operator, no page-tree type. **And no OOXML concept**: no zip, no XML reader, no part name it parses (v2-S2) |
| `engine-pdf` | `engine-core` | Any grounding or verification concept. **Any office concept** — a DOCX reader in here is what the fifth crate exists to prevent |
| `engine-office` | `engine-core` | Any PDF concept, any grounding concept. It reads OOXML packages — documents (v2-S2), workbooks (v2-S3) and presentations (v2-S4) — **and OpenDocument text (v2-S5), which shares the ZIP reader and the XML plumbing and nothing else** — and emits the shared representation |
| `engine-grounding` | `engine-core` | Any **format** concept — it projects the *representation*, never a document |
| `engine-cli` | all four | Any logic. It parses arguments, calls the library, maps errors to exit codes |

The `engine-grounding` row is what keeps the second format cheap, and v2-S2 is where that got
tested rather than asserted: **`mcp.rs`, both SDKs, the LangChain tools and `engine-grounding` were
all unchanged** by the arrival of DOCX. What did change is `engine-core`'s page-parent invariant,
which is the cost §6 did not predict — see the note there.

**"Any PDF concept" means machinery, not vocabulary — clarified at M5**, because the rule as
written forbids something the contract requires. `01-CONTRACT.md` §5.1 defines `NativeLocator` as
a **discriminated union with a `PdfLocator` variant**, and that union is part of the artifact
contract, which `engine-core` owns. This document's own header says the contract wins where the
two disagree, so the line is:

| In `engine-core` | Verdict |
| --- | --- |
| `lopdf`, a content-stream operator, a page tree, a font program, an xref table | **Forbidden.** This is machinery: it makes the crate know how to read one format |
| A contract-defined locator variant carrying integers (`PdfLocator { page, origin_x, … }`), a format name as a string (`BackendIdentity { name: "lopdf" }`) | **Permitted.** This is data and a discriminant. Nothing here can parse anything |

`engine-grounding` is held to the **stronger** rule, and it is a rule rather than a hope: the
projection addresses pages by node id, so it never reads a locator at all, and
`engine_grounding_has_no_pdf_concept` fails if the crate so much as mentions `NativeLocator`.
That is what actually keeps the second format a variant — hiding the type in `engine-pdf` would
have kept the letter of the old wording while leaving the projection free to match on it.

## 2. CLI surface — v0

Four subcommands. **The CLI is a thin shell over the library** so the two cannot diverge; every
subcommand is a library call plus argument parsing plus an exit-code mapping.

| Command | Input | Output | Exit codes |
| --- | --- | --- | --- |
| `engine classify <pdf>` | PDF | Classification artifact: per-page counts (1-indexed), reason codes on two axes, derived boolean | 0 simple · 1 needs-attention · **2 could-not-read** |
| `engine extract <pdf>` | PDF | `DocumentRepresentation v0` | 0 ok · 2 could-not-read |
| `engine ground <representation>` | representation JSON | `ethos.grounding.v1` artifact | 0 ok · 2 refused |
| `engine grounding-check <grounding.json> [--source-artifact <pdf>]` | grounding JSON (+ optional source) | `ethos.grounding_validation.v1` report | 0 valid · 1 invalid · 2 could-not-read |

**`--source-artifact` mirrors the Ethos CLI deliberately.** The oracle test runs
`ethos grounding check <file> --source-artifact <pdf>` and compares; matching the flag name keeps the
harness readable and the comparison obvious.

**Default output is byte-identical across runs.** Volatile diagnostics (timings, memory, host,
paths) are opt-in behind `--diagnostics` and excluded from the fingerprint — so a default invocation
produces identical *files*, not merely identical payloads.

### 2.1 Single document load

`classify` and `extract` in one invocation **parse the document once and share it**. This is a
correctness rule before it is a performance one: two loads can disagree, and a classifier that saw a
different object graph from the extractor is a silent divergence with no diagnostic. Borrowed from
pdf-inspector, which gets this right (checklist P11).

Practically: the library exposes an opened-document handle; `classify` and `extract` take it by
reference; only `engine-cli` decides when to open. Nothing below the CLI opens a file.

## 3. Profile as identity

The **profile** is the pinned set of every knob that can change output. Its `sha256` goes in every
artifact (`01-CONTRACT.md` §2), and it is the mechanism that gives OCR isolation, backend isolation,
and comparability for free — with no machinery beyond one hash.

The profile must include, at minimum:

- engine build identity and `parser_version`
- backend identity and version (`lopdf x.y.z`)
- classify sample count `N`
- quantum (100 per point) and coordinate origin
- the enabled capability set
- the reading-order rule version
- vendored CMap data version
- later: OCR engine identity, model `sha256`, execution envelope (runtime, CPU feature level,
  threads, DPI)

**The test that keeps this honest:** changing any profile field must change `profile_sha256`, and a
test asserts it field by field. A knob that does not move the hash is a silent-drift bug waiting to
happen. IDs are stable only within a representation produced by the same pinned profile — a profile
change creates a new representation plus a mapping, never a pretence that node IDs are globally
stable.

## 4. Fixtures and the oracle

**Fixtures are read-only, and they live in the Ethos tree.** `ethos-engine/fixtures/` holds a
manifest that references them by path and `sha256`; it does not copy them and it never modifies them.
Copying invites drift; a hash-pinned manifest makes a fixture change a visible event in this repo.

**Two roots, each independently overridable.** `conformance` (`ETHOS_FIXTURES`, default
`../ethos/fixtures`) holds the 15 the M6 criterion counts. `benchmark` (`ETHOS_BENCH_CORPUS`,
default `../ethos/benchmarks/gate-zero/corpus`) holds the large real-world PDFs M2's acceptance
names — the 492-page bounded-cost A/B document is not in `fixtures/` and never was. Separate roots
keep benchmark documents from inflating the 15.

**One exception, and it is enumerated rather than open.** Where the Ethos corpus has no fixture for a
behaviour this engine must test, the engine authors its own under CC0, stores it here, and marks it
`owner: "engine"` in the manifest. Today that is exactly one: the **absent-font-metrics** fixture that
exercises the geometry-omission path at M5. Adding a second engine-owned fixture needs the same
justification — the Ethos corpus genuinely cannot cover it — not merely convenience.

**The oracle is `ethos grounding check`.** For v0, the engine's `grounding-check` is a
reimplementation of the **JSON Schema validator only** — never the verifier — and it has a
deterministic external oracle to agree with:

```bash
ethos grounding check <file> --source-artifact <pdf>
```

`crates/engine-cli/tests/oracle.rs` runs both, across all 15 fixtures, and asserts byte-identical agreement on
`structure`, `source_binding`, `representation_sha256`, and `counts`. This is a CI job, not a claim.

**The Ethos binary is a test-time dependency, not a runtime one.** Its absence fails the oracle test
loudly; it never degrades to a skip. The engine itself does not invoke Ethos at v0 — that is v0.1,
and it arrives as a *declared capability* (`07-VERIFY-BOUNDARY.md`).

**Test layers, all of them cheap:**

| Layer | What it catches |
| --- | --- |
| Golden artifacts per fixture | Any change to emitted bytes, intended or not |
| Double-run byte identity | Nondeterminism: map ordering, timestamps, addresses, float drift |
| Oracle agreement | Divergence from the verifier's reading of the same artifact |
| `cargo-fuzz` on the PDF entry point | Panics, hangs, and fail-open paths on malformed input (from Anydoc, checklist A11) |
| Mutation testing over fixtures | Assertions that pass for the wrong reason |
| Profile-field sensitivity test | A knob that does not move `profile_sha256` |
| `cargo deny` | Licence and dependency-posture regressions |

## 5. Dependency posture

| Layer | Decision | Why |
| --- | --- | --- |
| **Object / xref layer** | **Depend on `lopdf`** | The commodity part. Requires Rust ≥ 1.88, which is why this workspace pins 1.88 and Ethos's 1.87 pin stops mattering — a concrete win of the sibling decision |
| **Encoding tables** — 168 Adobe `.bcmap` CMaps, glyph-name and Korea1 tables | **Vendor**, with the Adobe BSD-3-Clause NOTICE reproduced | They are *data*, they do not churn, and regenerating them is pure cost |
| **Content-stream interpreter, classification, layout** | **Clean-room**, using pdf-inspector as architectural reference only | See below |
| **Font metrics** | `ttf-parser` over the embedded font program | For measured ink boxes; FontDescriptor fallback; typed absence beyond that |
| **PDFium** | Not in v0. If ever, **caller-provided** and `sha256`-pinned, via an explicit ADR | Never a build-time download |
| **AGPL, anywhere** | **Forbidden.** `deny.toml` enforces | Forced decision #14 |
| **Network crates** | **Forbidden** in v0 | Nothing in the happy path reaches the network |

**Why pdf-inspector is reference-only and not a dependency**, since this decision gets re-litigated:
the parts worth wrapping are exactly the parts that would have to be un-wrapped. `TextItem.height` is
literally the same variable as `font_size`; `y` is a baseline the JSON never labels; the classifier's
confidence contradicts its own counts (TEXT-BASED at 50% with zero text pages). Depending on the
crate means inheriting types whose fields mean something other than their names, in a project whose
entire product is not doing that. Its MIT licence gives us the code to read either way, and upstream
moved 0.1.8 → 1.14.x in months. Reference-only is the cheaper path, and the `"`-operator silent drop
(checklist P6) makes it the safer one.

**What that leaves as the transferable ideas:** rectangle detection, encoding-issue detection, the
single-document-load rule, and the `mcid` bridge. Take those; write them.

**And on LiteParse's PDFium packaging** — build-time download from a vendor fork, by tag, with no
checksum, `vendor/` absent so the build is network-dependent by default (checklist L31). The one
transferable idea is **runtime dynamic loading via `libloading`** rather than link-time binding.
Everything else is what not to do.

## 6. How later lanes plug in without contaminating v0

The point of the type system chosen in `01-CONTRACT.md` is that v2.1 and v3 need **no new
mechanism**, only new values.

| Lane | Plugs in as | v0 impact |
| --- | --- | --- |
| **OCR (v2.1)** | A node source that authors `Recognized` nodes **only on canvases where the deterministic reader found no text layer at all**, under its own profile (`ethos-ocr-v1`) | None. `DerivationClass` already exists; `profile_sha256` already isolates. No v0 type changes |
| **HTTP OCR (v2.1)** | An implementation of the same node-source trait, behind `POST /ocr` (LiteParse's contract, `confidence` accepted as a diagnostic and never filtered on) | None |
| **Assist / VLM (v3)** | Authors `Proposed` nodes only. Never overwrites. Never citable | None |
| **Second format (v2)** | A new `NativeLocator` variant + adapter profile + fixtures + inspection behaviour | None — provided `engine-grounding` never learned about pages |
| **Second backend** | A trait seam modelled on Ethos's `EthosPdfBackend` 3-method shape, with backend identity in the profile | Design the seam in v0; implement one side |

**The second-format row, paid at v2-S2.** The row said a new format costs a `NativeLocator`
variant, an adapter profile, fixtures and inspection behaviour, and "None" downstream. That was
right about downstream — `engine-grounding`, `mcp.rs` and both SDKs are untouched — and it missed
one line item: **`engine-core`'s invariant that every node's parent is a declared page**, which S2
had to split by locator family. The row is otherwise exactly what a second format cost.

**The precondition, verified at v2-S1 — and it points at the wrong crate.**
`engine-grounding` never learned what a page *is*: it reads no locator, derives no geometry, and
addresses pages by id, which `engine_grounding_has_no_pdf_concept` enforces. But the assumption that
**every node has a page parent** lives in `engine-core`: `DocumentRepresentation::check_structure`
refuses a node whose parent is not a declared page, on both construction paths, so a page-less
document cannot become a representation at all. That is permitted by §1's M5 line — a contract
invariant is not format machinery — and it is the sentence v2 has to revisit, so "None" in the row
above is the cost to *grounding*, not the cost to the version.
`crates/engine-grounding/tests/page_less_source.rs` measures it; `14-V2-SCOPE.md` §5 records it.

**Rule 7 enforcement, designed now, implemented when assist exists:** give every lane a **declared
processor identity** in the processing run. A run whose drafting model and representation processor
share an identity is **rejected mechanically**, not caught in review. This is cheaper than the
byte-diff CI it supplements, and it is the precondition that makes a VLM lane safe to build at all.

## 7. What this architecture refuses

- **No verification code, type, or field anywhere in the tree at v0.** Not a stub, not a feature flag,
  not a `TODO`-shaped module. See `07-VERIFY-BOUNDARY.md`
- **No crate wrapping a competitor's parser as the grounded PDF core**
- **No build-time network access.** Not for PDFium, not for CMaps, not for models
- **No logic in `engine-cli`.** If a behaviour can only be exercised through the CLI, it is in the
  wrong crate and the library is incomplete
- **No PDF concept in `engine-core` or `engine-grounding`**

---

## PR review checklist

- [ ] Crate boundaries respected — no PDF type in `core` or `grounding`, no logic in `cli`
- [ ] One document load per invocation; nothing below the CLI opens a file
- [ ] New knob ⇒ added to the profile ⇒ profile-sensitivity test updated
- [ ] Fixtures referenced by hash-pinned manifest, never copied or modified
- [ ] Oracle test still green across all 15 fixtures; failure is loud, never a skip
- [ ] `cargo deny` green — no AGPL, no network crates
- [ ] New dependency justified against §5, with a licence named
- [ ] No verification concept has appeared
- [ ] Default invocation still produces byte-identical files across two runs
