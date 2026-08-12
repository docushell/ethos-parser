# ethos-engine

An open, high-performance document parser that emits **evidence**: a versioned, fingerprinted
representation in which every node carries a locator back into the source bytes, every capability is
declared on the wire, and everything the parser could not do is stated rather than guessed.

It answers **"what does this document contain, and exactly where?"** It does not answer whether a
claim is true — that is a separate verifier's job. Together they answer the question that matters:
*did this AI claim actually come from this document?*

## Status

**Docs bootstrap. Pre-code.** There is no Rust in this repository yet, by design. The artifact
contract is frozen before the parser is written so the contract is shaped by what a verifier needs,
not by a parser's accidents.

- **Start here:** [`docs/README.md`](docs/README.md)
- **Then:** implement **M0** from [`docs/05-MILESTONES.md`](docs/05-MILESTONES.md), and nothing else

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
