# Pinned schema snapshots

**These are copies. This repository does not own them and never edits them.**

| File | Origin | `sha256` |
| --- | --- | --- |
| `ethos-grounding-source.schema.json` | `../ethos/schemas/ethos-grounding-source.schema.json` | `410f1ce12d8f48bcf8d3d5f6b38baa12ff0d7ce02fc3562d74c902ad7fd13894` |
| `ethos-grounding-validation-report.schema.json` | `../ethos/schemas/ethos-grounding-validation-report.schema.json` | `eed7e2f3575a6d57f25a7b29bbdc6597255e4d9018c8f06b50546f03d45b6f11` |

## Why a snapshot instead of a path

The Ethos tree is a test-time dependency, not a runtime one. A test that read the schema straight
out of `../ethos` would pass or fail depending on whether a sibling checkout happened to be there,
and would silently change meaning when that checkout moved. The snapshot gives the conformance test
one fixed thing to check against, and lets a reader see exactly which shape this engine was built to
emit.

**A missing Ethos tree weakens the drift check but never weakens conformance.** The drift check
compares the snapshot against `../ethos/schemas/…` when that tree is present and reports both
digests when they differ; when it is absent, the comparison is skipped and the snapshot is still
used. That is the opposite of the fixture rule, where a missing corpus is a hard failure — the
difference is that a fixture is *input a test needs*, while this is *a second opinion about a file
we already have*. If the snapshot itself went missing, the conformance test would fail outright.

Updating a snapshot is a deliberate act: copy the file, update the digest above, and say in the
commit what changed upstream and what it means for the projection.

## What the report schema is not

`ethos-grounding-validation-report.schema.json` describes the shape `ethos-parser grounding-check`
emits. **It is not what the checker validates against.** The checker mirrors Ethos's own parser rule
for rule and in its order, because the schema is necessary but not sufficient: id uniqueness,
reference resolution, page ordering, boxes inside their page, character offsets that actually index
their element's text, table cell occupancy — none of that is expressible in JSON Schema. A checker
that validated only the schema would call artifacts valid that the oracle calls invalid.

## One thing worth knowing about the built oracle

The `ethos` binary in the sibling tree's `target/release/` is older than its own source. It prints
the validation report bare; the committed source wraps it in an in-toto Statement with the report at
`predicate`. The oracle harness reads both shapes and refuses anything else rather than guessing — so
a rebuild of Ethos changes nothing here, but if you are comparing output by hand, that is why two
runs can look different.
