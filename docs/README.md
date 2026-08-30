# Documentation map

Everything the project has decided, and where it is written down. Start with the root
[`README.md`](../README.md) if you just want to know what the tool does.

**Version 0.41.0.** PDF and eight office formats read. Markdown, HTML, MCP and both SDKs ship. The
one open gap is table accuracy — measured at roughly 7% cell-level F1, which is a miss, with the
method and the caveats in [`table-gate-v1.md`](table-gate-v1.md).

## New here? Read in this order

1. [`00-NORTH-STAR.md`](00-NORTH-STAR.md) — what the product is, and the decisions already made.
2. [`01-CONTRACT.md`](01-CONTRACT.md) — the shape of every artifact. Frozen before any code was
   written, on purpose, so it is shaped by what a verifier needs rather than by what a parser
   happened to produce.
3. [`03-V0-SCOPE.md`](03-V0-SCOPE.md) — what was in and out of the first release.
4. [`CAPABILITY.md`](CAPABILITY.md) — what this build can and cannot do today.

Then, when you need them:

- [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) before deciding where code goes.
- [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) before touching anything verification-shaped.
- [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) before borrowing an idea from another parser.

## The documents

| Doc | What it settles |
| --- | --- |
| [`00-NORTH-STAR.md`](00-NORTH-STAR.md) | The product, the forced decisions, who owns which layer of trust, and the anti-goals |
| [`01-CONTRACT.md`](01-CONTRACT.md) | Artifact identity, coordinates, canonical JSON, locators, typed absence, capabilities, fail-closed |
| [`02-ROADMAP.md`](02-ROADMAP.md) | v0 through v3, one line each, plus what is deliberately not scheduled |
| [`03-V0-SCOPE.md`](03-V0-SCOPE.md) | v0's scope, exit codes, fixtures and exit criteria. Parsed by a test — edit with care |
| [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) | Crate layout and boundaries, the single-load rule, profile-as-identity, dependency posture |
| [`05-MILESTONES.md`](05-MILESTONES.md) | M0–M7 with goals and acceptance tests. The v0 code-review map |
| [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) | Take / improve / refuse / defer, per idea, for every parser looked at |
| [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) | Where the engine stops and the verifier starts, and the anti-patterns that blur it |
| [`08-V1-SCOPE.md`](08-V1-SCOPE.md) | What v1 is, the ruled/unruled table split, and why the accuracy chase is parked |
| [`09-V1-MILESTONES.md`](09-V1-MILESTONES.md) | S0–S8. The v1 code-review map |
| [`10-V11-SCOPE.md`](10-V11-SCOPE.md) | v1.1 (Safe Markdown) and the four laws of the anchor map |
| [`11-V11-MILESTONES.md`](11-V11-MILESTONES.md) | v1.1 S0–S4, all done |
| [`12-V12-SCOPE.md`](12-V12-SCOPE.md) | v1.2 (adapters) and the handle law every adapter follows |
| [`13-V12-MILESTONES.md`](13-V12-MILESTONES.md) | v1.2 S0–S5, all done. S5 ended in a refusal |
| [`14-V2-SCOPE.md`](14-V2-SCOPE.md) | v2 (office formats) and the no-invented-pages law |
| [`15-V2-MILESTONES.md`](15-V2-MILESTONES.md) | v2 S0–S24. Eight formats read, CSV argued and refused |
| [`table-gate-v1.md`](table-gate-v1.md) | How table accuracy is measured, and the result. Quote table numbers from here or not at all |
| [`CAPABILITY.md`](CAPABILITY.md) | Can and cannot, as two tables. The answer to "what does this actually do?" |
| [`PUBLIC-API.md`](PUBLIC-API.md) | The frozen export list per crate. Checked by a test |
| [`draft-schemas/`](draft-schemas/) | Draft JSON Schemas for every artifact shape. Documentation, not a shipped contract |
| [`reference/`](reference/) | A stub explaining where the research archive went |
| [`attic/`](attic/) | Work built, measured, and deliberately not shipped — with the numbers and the reason |

## If you are a coding agent

Run the gate first, so you know the baseline you inherited:

```bash
cargo test --workspace --locked
```

Do not add `--skip` to get a green build. `oracle_agrees_on_simple_text` is the only test proving
this engine and the Ethos verifier read a document the same way; filtering it out deletes the
guarantee rather than fixing it.

A few boundaries worth knowing before you write anything:

- **`ethos-parser-core` knows nothing about file formats.** PDF work lives in `ethos-parser-pdf`,
  office work in `ethos-parser-office`, and both depend on core rather than the other way round. A
  test scans core's sources and fails on a PDF import, a stray float, or the word `confidence`.
- **Do not widen the public API** without editing [`PUBLIC-API.md`](PUBLIC-API.md) in the same
  commit — a test compares the two.
- **v0 is frozen**, and v0.1 through v2 shipped on top of it. Do not re-author the contract types,
  the classifier, the extractor, the representation or the checker.
- **The Ethos and DocuShell repos are read-only** from here. Never edit them from this project.

## Rules that get broken under deadline pressure

1. No public confidence field, ever.
2. No box derived from a font size — say the geometry is absent instead.
3. No silent drop. Nothing disappears without a typed diagnostic.
4. No invented coordinate, identifier, fingerprint, or page number.
5. Fail closed, and distinguishably — three exit codes, named errors.
6. Byte identity is a test, not an aspiration.
7. The Ethos tree is read-only.
8. No verification code in this repository.

## Known gaps

- **Two open questions for DocuShell.** Whether a geometry-absent span should be representable in a
  future `ethos.grounding.v1` revision, and whether typed `GeometryAbsence` should replace the
  companion spec's plain optional field. Both are additive and neither blocks anything.
- **Some Adobe data is not vendored** — the predefined CJK CMaps, the Core-14 AFM widths, and the
  full Adobe Glyph List. Each gap is declared on the wire rather than papered over. See
  [`vendor/README.md`](../vendor/README.md).
- **`skrifa` is pinned to 0.39**, because 0.44 needs Rust 1.89 and this workspace pins 1.88.

Grep for `TODO(` to find the rest.
