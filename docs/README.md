# Documentation map

Everything the project has decided, and where it is written down. Start with the root
[`README.md`](../README.md) if you just want to know what the tool does.

**Version 0.66.0-dev.1.** PDF and eight office formats read. Markdown, HTML, MCP and both SDKs ship.
`tag`, the tenth subcommand, writes a structure tree over an untagged PDF from the block cut, marked
computed; it fills absence only ([`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md)).
`locate`, the eleventh, reports where a string lies in a representation — node ids, offsets and the
record's own boxes — and never whether anything is true
([`26-LOCATE-SCOPE.md`](26-LOCATE-SCOPE.md)).
Tables are stated as a capability rather than as one average, re-measured at 0.58.0: the engine
reads the tables a document declares (combined cell-slot recall 503‰), detects ruled tables where
the producer drew them (geometric macro 69‰, band 0‰–590‰), emits nothing where neither holds, and
fabricates nothing. The
method and the caveats are in [`table-gate-v1.md`](table-gate-v1.md).

## New here? Read in this order

1. [`00-NORTH-STAR.md`](00-NORTH-STAR.md) — what the product is, and the decisions already made.
2. [`01-CONTRACT.md`](01-CONTRACT.md) — the shape of every artifact. Frozen before any code was
   written, on purpose, so it is shaped by what a verifier needs rather than by what a parser
   happened to produce.
3. [`CAPABILITY.md`](CAPABILITY.md) — what this build can and cannot do today.

Then, when you need them:

- [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) before deciding where code goes.
- [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) before touching anything verification-shaped.
- [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) before borrowing an idea from another parser.

## The live documents

These describe the engine as it is now. Read them to work on it.

| Doc | What it settles |
| --- | --- |
| [`00-NORTH-STAR.md`](00-NORTH-STAR.md) | The product, the forced decisions, who owns which layer of trust, and the anti-goals |
| [`01-CONTRACT.md`](01-CONTRACT.md) | Artifact identity, coordinates, canonical JSON, locators, typed absence, capabilities, fail-closed |
| [`02-ROADMAP.md`](02-ROADMAP.md) | v0 through v4, one line each, plus what is deliberately not scheduled |
| [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) | Crate layout and boundaries, the single-load rule, profile-as-identity, dependency posture |
| [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) | Take / improve / refuse / defer, per idea, for every parser looked at |
| [`07-VERIFY-BOUNDARY.md`](07-VERIFY-BOUNDARY.md) | Where the engine stops and the verifier starts, and the anti-patterns that blur it |
| [`table-gate-v1.md`](table-gate-v1.md) | How table accuracy is measured, and the result. Quote table numbers from here or not at all |
| [`CAPABILITY.md`](CAPABILITY.md) | Can and cannot, as two tables. The answer to "what does this actually do?" |
| [`BENCHMARKS.md`](BENCHMARKS.md) | The public benchmark scores, ranked against the other parsers, with each benchmark's pin |
| [`INSTALL.md`](INSTALL.md) | What each release archive holds, platform requirements, Windows, macOS quarantine, building from source |
| [`OPEN-WORK.md`](OPEN-WORK.md) | What is pending and what each item waits on: v2.2's clause two as built, v2.3's items, owner decisions, known defects, stale docs |
| [`PUBLIC-API.md`](PUBLIC-API.md) | The frozen export list per crate. Checked by a test |
| [`RELEASING.md`](RELEASING.md) | How a version is numbered, tagged and released — written before the act, because a publish cannot be undone |
| [`draft-schemas/`](draft-schemas/) | Draft JSON Schemas for every artifact shape. Documentation, not a shipped contract |
| [`reference/`](reference/) | A stub explaining where the research archive went |
| [`attic/`](attic/) | Patches kept for the record, not for reuse. Its one entry, `stroke-ruled-v1`, was parked at v1-S7b and then revived and shipped at 0.10.0 |

## Scope documents

One per piece of work whose argument was written down before, or instead of, the code: what was
asked, what was measured, and what was built or refused. The decision itself lives in its
[`00-NORTH-STAR.md`](00-NORTH-STAR.md) row.

| Doc | What it settles |
| --- | --- |
| [`16-D4-SCOPE.md`](16-D4-SCOPE.md) | D4, the first half of v2.2: geometric block structure (decision #19) |
| [`17-D1-SCOPE.md`](17-D1-SCOPE.md) | D1, declared document splits: measured and refused |
| [`18-INTERNING-SCOPE.md`](18-INTERNING-SCOPE.md) | Role-path interning: measured and refused |
| [`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md) | Block subdivision: a measurement, never scoped; the cut shipped anyway at 0.55.0 |
| [`20-STANDARD-14-METRICS-SCOPE.md`](20-STANDARD-14-METRICS-SCOPE.md) | Standard-14 metrics from vendored Adobe AFMs: the measurement behind decision #22, shipped at 0.51.0 |
| [`21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](21-STANDARD-14-ASCII-COVERAGE-SCOPE.md) | Standard-14 coverage past ASCII: refused as a hand-transcribed table, built as a derived one at 0.52.0 |
| [`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md) | Word boxes: refused on measurement, and `char_offsets` turned on from the same evidence |
| [`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md) | Auto-tagging, v2.2's second half: a tag this engine writes, and reads back as its own |
| [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) | Auto-tagging's slices |
| [`25-KNOBS-SCOPE.md`](25-KNOBS-SCOPE.md) | Five knobs — password, page sets, `continue_on_page_error`, batch, colour — measured (decision #31) |
| [`26-LOCATE-SCOPE.md`](26-LOCATE-SCOPE.md) | `locate(representation, quote)`: the shape decision #30 left to a scope |
| [`27-LOCATE-MILESTONES.md`](27-LOCATE-MILESTONES.md) | `locate`'s slices |
| [`28-HEADINGS-SCOPE.md`](28-HEADINGS-SCOPE.md) | A heading inferred from type where the document declares no structure (decision #29) |
| [`29-OUTLINES-SCOPE.md`](29-OUTLINES-SCOPE.md) | A PDF's `/Outlines`, emitted as the author wrote it; built as `outlines-v1` |
| [`30-FORM-XOBJECTS-SCOPE.md`](30-FORM-XOBJECTS-SCOPE.md) | Text a page draws through a Form XObject is text the page draws (decision #37) |
| [`31-TABLE-TRACKS-SCOPE.md`](31-TABLE-TRACKS-SCOPE.md) | Tables from whitespace tracks (decision #38, slice D) |
| [`32-CHART-LABELS-SCOPE.md`](32-CHART-LABELS-SCOPE.md) | A chart's printed labels read back as its table: bars, lines and pies (decision #42 and after) |
| [`33-UNMAPPED-CODES-SCOPE.md`](33-UNMAPPED-CODES-SCOPE.md) | Codes a font's declared map leaves unmapped, read through what the font itself states (decision #43) |
| [`34-FIGURE-REGIONS-SCOPE.md`](34-FIGURE-REGIONS-SCOPE.md) | Figure regions, so a picture drawn with paths has an element (decision #46) |
| [`35-OCR-SCOPE.md`](35-OCR-SCOPE.md) | v3, the OCR lane: a scanned page is read, and says so (decision #120) |
| [`36-CHART-LENGTHS-SCOPE.md`](36-CHART-LENGTHS-SCOPE.md) | A bar chart's values read off its axis (decision #122): measured at no gain and parked; nothing of it is in the tree |
| [`37-ASSIST-TRANSPORT.md`](37-ASSIST-TRANSPORT.md) | The assist lane's transport, the system `curl` (decision #144): the ADR `deny.toml` requires |

## [`history/`](history/) — shipped work

Scope and milestone records for releases that are done. Nothing here describes the current
build; read it to find out **why** something is the way it is, not **what** it is.

| Doc | Release |
| --- | --- |
| [`03-V0-SCOPE.md`](history/03-V0-SCOPE.md) · [`05-MILESTONES.md`](history/05-MILESTONES.md) | v0 — M0–M7. `03` is parsed by a test; edit with care |
| [`08-V1-SCOPE.md`](history/08-V1-SCOPE.md) · [`09-V1-MILESTONES.md`](history/09-V1-MILESTONES.md) | v1 — S0–S8, the ruled/unruled table split |
| [`10-V11-SCOPE.md`](history/10-V11-SCOPE.md) · [`11-V11-MILESTONES.md`](history/11-V11-MILESTONES.md) | v1.1 — Safe Markdown and the four laws of the anchor map |
| [`12-V12-SCOPE.md`](history/12-V12-SCOPE.md) · [`13-V12-MILESTONES.md`](history/13-V12-MILESTONES.md) | v1.2 — adapters and the handle law. S5 ended in a refusal |
| [`14-V2-SCOPE.md`](history/14-V2-SCOPE.md) · [`15-V2-MILESTONES.md`](history/15-V2-MILESTONES.md) | v2 — office formats, the no-invented-pages law, CSV refused |

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
- **v0 is frozen**, and every version since shipped on top of it. Do not re-author the contract types,
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
- **Some Adobe data is not vendored** — the predefined CJK CMaps, which map character codes to
  CIDs, a gap declared on the wire rather than papered over. The Core-14 AFMs (decision #22), the
  Adobe Glyph List (decision #59) and Adobe's CID-to-Unicode mappings for its own collections
  (decision #78) are vendored. See [`vendor/README.md`](../vendor/README.md).
- **`skrifa` is pinned to 0.39**, because 0.44 needs Rust 1.89 and this workspace pins 1.88.

The one `TODO(` left in the tree, in `crates/ethos-parser-core/src/derivation.rs` and mirrored in
`draft-schemas/geometry.draft.json`, is the first gap above.
