# Pinned schema snapshots

**These are copies. This repository does not own them and never edits them.**

| File | Origin | `sha256` of the snapshot |
| --- | --- | --- |
| `ethos-grounding-source.schema.json` | `../ethos/schemas/ethos-grounding-source.schema.json` (`$id` `urn:ethos:schema:grounding-source:1`) | `8d41c1e08f49ec0ca4878ac0ec3ccf3a79f7b9ffa31aa26ad0a60a6f27b319de` |
| `ethos-grounding-validation-report.schema.json` | `../ethos/schemas/ethos-grounding-validation-report.schema.json` (`$id` `urn:ethos:schema:grounding-validation-report:1`) | `eed7e2f3575a6d57f25a7b29bbdc6597255e4d9018c8f06b50546f03d45b6f11` |

## Why a snapshot rather than a path

The Ethos tree is a **test-time** dependency, not a runtime one (`docs/04-ARCHITECTURE.md` §4). A
test that read the schema straight out of `../ethos` would pass or fail depending on whether a
sibling checkout happened to be present, and would silently change meaning when that checkout moved
— which is the same class of problem as a build-time download. The snapshot is committed so the
conformance test has one fixed thing to check against, and so a reader can see exactly which shape
this engine was built to emit.

## Drift detection

`crates/engine-grounding/tests/schema_subset.rs` compares the snapshot against
`../ethos/schemas/…` **when that tree is present**, and fails with both digests when they differ.
When the tree is absent the comparison is skipped and the snapshot is still used — the engine's own
conformance testing does not depend on a sibling checkout, and only the drift check does.

This is a deliberate asymmetry, so it is worth stating plainly: **a missing Ethos tree weakens the
drift check but never weakens conformance.** That is the opposite of the M0 fixture rule, where a
missing corpus is a hard failure rather than a skip — and the difference is that a fixture is
*input the test needs* while this is *a second opinion about a file we already have*. If the
snapshot were the thing that went missing, the conformance test would fail outright.

Updating the snapshot is a deliberate act: copy the file, update the digest above, and say in the
commit what changed upstream and what it means for the projection.

## What the report snapshot is for, and what it is not

`ethos-grounding-validation-report.schema.json` describes the shape `engine grounding-check`
emits. It is pinned for the same reason as the source schema: so the shape this engine writes has
one fixed description a reader can check against.

**It is not what the checker validates against.** `engine-grounding`'s `check` module mirrors
Ethos's *parser* — `ethos-core/src/grounding_json.rs` — rule for rule and in its order, because
the schema is necessary and not sufficient. Id uniqueness, reference resolution, page ordering,
boxes inside their page, capability/array agreement, character offsets that index their element's
text, table cell occupancy: none of that is expressible in JSON Schema, and a checker that
validated only the schema would call artifacts valid that the oracle calls invalid.

## One thing worth knowing about the built oracle

The `ethos` binary in the sibling tree's `target/release/` is **older than its own source**. It
prints the validation report bare; the committed source (and the ref CI pins) wraps it in an
in-toto Statement, with the report at `predicate`. The oracle harness reads both shapes and
refuses anything else rather than guessing, so a rebuild of Ethos changes nothing here — but if
you are comparing output by hand, that is why two runs can look different.
