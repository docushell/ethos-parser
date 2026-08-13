# ethos-engine

An open, high-performance document parser that emits **evidence**: a versioned, fingerprinted
representation in which every node carries a locator back into the source bytes, every capability is
declared on the wire, and everything the parser could not do is stated rather than guessed.

It answers **"what does this document contain, and exactly where?"** It does not answer whether a
claim is true — that is a separate verifier's job. Together they answer the question that matters:
*did this AI claim actually come from this document?*

## Status

**M3 complete. Classification and extraction both work.** The artifact contract is Rust
(`engine-core`), frozen *before* any parser was written so it is shaped by what a verifier needs
rather than by a parser's accidents.

```bash
engine classify document.pdf   # counts and reason codes; exit 0 / 1 / 2
engine extract  document.pdf   # position-aware text runs; exit 0 / 2
```

Extraction interprets content streams against an **exhaustive** operator table — an unrecognised
operator stops the parse instead of being skipped — puts a native locator on every run, and
reports an ink box only when it was measured.

`engine-grounding` projects the record into `ethos.grounding.v1` and validates one, and the oracle
test compares its verdict against the Ethos CLI's on every fixture that reaches an artifact.

- **Start here:** [`docs/README.md`](docs/README.md)
- **Next milestone:** **M7** — API freeze, fuzz and mutation layers, v0 exit criteria as CI jobs
  ([`docs/05-MILESTONES.md`](docs/05-MILESTONES.md))

## Building

```bash
cargo test --workspace --locked
```

No exclusion: `oracle_agrees_on_simple_text` failed by design from M0 through M5 and
runs the real comparison as of M6. Adding `--skip` to get a green build would delete the only
thing proving this engine and the verifier read an artifact the same way.

The oracle harness needs the Ethos repo for its fixture corpus and CLI. It resolves
`../ethos/fixtures` and `../ethos/target/release/ethos` by default; override with `ETHOS_FIXTURES`
and `ETHOS_BIN`. Absence is a failure, never a skip.

## What it will do first (v0)

`classify → extract → ground → grounding-check`, on born-digital PDF. Classification emits counts and
named reason codes on two orthogonal axes — never a confidence score. Extraction emits position-aware
text runs with required native locators and measured ink boxes, or typed absence where the metrics do
not exist. No tables, no OCR, no Markdown, no office formats, no verification.

v0 is complete when every line of `docs/03-V0-SCOPE.md` §5 is a green CI job. The validator half of
that is already there: it agrees with the Ethos CLI on `structure`, `source_binding`,
`representation_sha256` and `counts` for all 11 conformance fixtures that reach a grounding
artifact, and the other 4 are asserted to fail closed rather than quietly skipped.

## What it will never do

Emit a confidence float or any single field meaning "this document is good." Delete hidden text,
headers, footers, or low-confidence content without a record. Invent a coordinate, identifier,
fingerprint, or pagination. Verify a citation. Publish a competitor bake-off table.

## Licence

Apache-2.0, matching Ethos. `NOTICE` is reserved for the vendored Adobe CMap data (BSD-3-Clause). No
AGPL dependencies, enforced by `deny.toml`.
