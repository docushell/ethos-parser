# ethos-engine

An open, high-performance document parser that emits **evidence**: a versioned, fingerprinted
representation in which every node carries a locator back into the source bytes, every capability is
declared on the wire, and everything the parser could not do is stated rather than guessed.

It answers **"what does this document contain, and exactly where?"** It does not answer whether a
claim is true — that is a separate verifier's job. Together they answer the question that matters:
*did this AI claim actually come from this document?*

## Status

**v0 is complete and frozen — v0.1.0.** Every line of `docs/03-V0-SCOPE.md` §5 is a named CI job,
and the public API is a deliberate list rather than whatever happened to be `pub`
([`docs/PUBLIC-API.md`](docs/PUBLIC-API.md)).

Four subcommands, one library, one document load:

```bash
engine classify        document.pdf                      # counts and reason codes  · 0 / 1 / 2
engine extract         document.pdf                      # DocumentRepresentation v0 · 0 / 2
engine ground          representation.json               # ethos.grounding.v1        · 0 / 2
engine grounding-check grounding.json --source-artifact document.pdf   # validation  · 0 / 1 / 2
```

Add `--diagnostics` to any of them for timing, host and input details **on stderr**. stdout is the
artifact and does not change: two runs over the same bytes produce identical files, with the flag
and without.

The artifact contract is Rust (`engine-core`), frozen *before* any parser was written so it is
shaped by what a verifier needs rather than by a parser's accidents. Extraction interprets content
streams against an **exhaustive** operator table — an unrecognised operator stops the parse instead
of being skipped — puts a native locator on every run, and reports an ink box only when it was
measured. `engine-grounding` projects the record into `ethos.grounding.v1` and validates one; the
oracle test compares its answer against the Ethos CLI's on every fixture that reaches an artifact.

- **Start here:** [`docs/README.md`](docs/README.md)
- **The frozen surface:** [`docs/PUBLIC-API.md`](docs/PUBLIC-API.md)
- **What is next:** v0.1 — `ethos verify` as a declared capability, and the xref repair-or-refuse
  decision ([`docs/02-ROADMAP.md`](docs/02-ROADMAP.md)). Not started.

## Building

```bash
cargo test --workspace --locked
```

No exclusion, and no `--skip` anywhere in CI — a test asserts that
(`v0_exit_criteria.rs::no_ci_job_skips_a_test`). `oracle_agrees_on_simple_text` failed by design
from M0 through M5 and runs the real comparison as of M6; filtering it out to get a green build
would delete the only thing proving this engine and the verifier read an artifact the same way.

The oracle harness needs the Ethos repo for its fixture corpus and CLI. It resolves
`../ethos/fixtures` and `../ethos/target/release/ethos` by default; override with `ETHOS_FIXTURES`,
`ETHOS_BENCH_CORPUS` and `ETHOS_BIN`. Absence is a failure, never a skip.

## The v0 exit criteria, as CI

`docs/03-V0-SCOPE.md` §5 is fifteen lines, and each one names the job that proves it — so a
reviewer sees *which* criterion is green, not just that nothing failed:

| | |
| --- | --- |
| `v0-happy-path` | the full path over all 15 conformance fixtures |
| `v0-double-run` | byte-identical **files** across two runs, not merely payloads |
| `v0-oracle` | agreement with `ethos grounding check`, fixture by fixture |
| `v0-artifact-identity` · `v0-coordinates` | identity envelope, and a declared frame wherever there is geometry |
| `v0-no-confidence` · `v0-no-verify` | greps over `crates/*/src`; both are jobs, not review items |
| `v0-c14n` · `v0-locators` · `v0-l1-gate` | integers only, a locator on every node, capabilities and limitations on the wire |
| `v0-exit-codes` · `v0-fail-closed` | three distinguishable codes; unknown operator, magic and quantum each refused by name |
| `v0-classify-bound` | 492 pages at N=8 still scans 8 |
| `v0-fixture-mutation` · `v0-fuzz-smoke` | every fixture damaged six ways; `cargo-fuzz` on the PDF entry point |
| `deny-policy-is-enforced` | `cargo deny`, plus a probe that proves the AGPL rule actually fires |

`crates/engine-cli/tests/v0_exit_criteria.rs` checks that mapping in both directions, so a renamed
job or an unticked box is a red test rather than a stale document.

## Performance posture

**Bounded, and measured only against itself.** The claim v0 makes is that classification cost does
not scale with page count, and the proof is an instrumented counter rather than a stopwatch: on a
492-page document at N=8, `pages_content_scanned` is 8.

No competitor comparison, ranking, or bake-off table appears anywhere in this repository, and no
performance figure is published that a committed harness does not produce. See
`docs/03-V0-SCOPE.md` §6 for why — the widely-copied figure in this space traces back to a doc
comment with no benchmark behind it.

## What it will never do

Emit a confidence float or any single field meaning "this document is good." Delete hidden text,
headers, footers, or low-confidence content without a record. Invent a coordinate, identifier,
fingerprint, or pagination. Verify a citation. Publish a competitor bake-off table.

## Licence

Apache-2.0, matching Ethos. `NOTICE` is reserved for the vendored Adobe CMap data (BSD-3-Clause). No
AGPL dependencies, enforced by `deny.toml` and proved by the `deny-policy-is-enforced` job.
