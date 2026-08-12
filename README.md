# ethos-engine

An open, high-performance document parser that emits **evidence**: a versioned, fingerprinted
representation in which every node carries a locator back into the source bytes, every capability is
declared on the wire, and everything the parser could not do is stated rather than guessed.

It answers **"what does this document contain, and exactly where?"** It does not answer whether a
claim is true — that is a separate verifier's job. Together they answer the question that matters:
*did this AI claim actually come from this document?*

## Status

**M0 complete. No parsing yet.** The workspace, the pinned toolchain, the dependency policy, the
fixture manifest and the oracle harness exist. The four crates are skeletons: `engine` exits 2 with
a named reason, and the oracle test fails on purpose with a diagnostic naming everything still
missing. The artifact contract was frozen before any parser was written, so it is shaped by what a
verifier needs rather than by a parser's accidents.

- **Start here:** [`docs/README.md`](docs/README.md)
- **Next milestone:** **M1** — contract types, c14n, integer quanta
  ([`docs/05-MILESTONES.md`](docs/05-MILESTONES.md))

## Building

```bash
cargo test --workspace --locked -- --skip oracle_agrees_on_simple_text
```

That exclusion is deliberate and is the M0 gate. `oracle_agrees_on_simple_text` fails until M6, with
a diagnostic naming each missing milestone. Do not `#[ignore]` or delete it to get a green run.

The oracle harness needs the Ethos repo for its fixture corpus and CLI. It resolves
`../ethos/fixtures` and `../ethos/target/release/ethos` by default; override with `ETHOS_FIXTURES`
and `ETHOS_BIN`. Absence is a failure, never a skip.

## What it will do first (v0)

`classify → extract → ground → grounding-check`, on born-digital PDF. Classification emits counts and
named reason codes on two orthogonal axes — never a confidence score. Extraction emits position-aware
text runs with required native locators and measured ink boxes, or typed absence where the metrics do
not exist. No tables, no OCR, no Markdown, no office formats, no verification.

v0 is complete when its validator agrees byte-identically with the Ethos CLI across all 15
conformance fixtures.

## What it will never do

Emit a confidence float or any single field meaning "this document is good." Delete hidden text,
headers, footers, or low-confidence content without a record. Invent a coordinate, identifier,
fingerprint, or pagination. Verify a citation. Publish a competitor bake-off table.

## Licence

Apache-2.0, matching Ethos. `NOTICE` is reserved for the vendored Adobe CMap data (BSD-3-Clause). No
AGPL dependencies, enforced by `deny.toml`.
