# DRAFT schemas

**None of these is a shipped contract.** They live under `docs/` on purpose: there is no
production `schemas/` path in this repository, and there will not be one until the
`DocumentRepresentation v0` field list is confirmed with DocuShell (`docs/05-MILESTONES.md` M1
"Out").

Every file carries `DRAFT` in its `$comment` and points back at `docs/01-CONTRACT.md`, which is the
authority. Where these and the contract disagree, the contract is right.

| File | Covers | Contract |
| --- | --- | --- |
| [`artifact-identity.draft.json`](artifact-identity.draft.json) | `artifact_type`, `schema_version`, `parser_version`, `profile_sha256` | §2 |
| [`coordinate-system.draft.json`](coordinate-system.draft.json) | The declaration carried by every artifact bearing geometry | §3 |
| [`profile.draft.json`](profile.draft.json) | Every output-affecting knob, with the real v0 profile as its example | §2, `04-ARCHITECTURE.md` §3 |
| [`derivation-class.draft.json`](derivation-class.draft.json) | `extracted` / `computed` / `recognized` / `proposed` | §6 |
| [`geometry.draft.json`](geometry.draft.json) | `QRect` as `[x0, y0, x1, y1]`, and typed absence | §5.2, §5.3 |
| [`error-taxonomy.draft.json`](error-taxonomy.draft.json) | The six routable error kinds | §8 |
| [`classification.draft.json`](classification.draft.json) | The M2 classification artifact: counts, the two reason axes, the derived boolean | §2, §8, §9 |
| [`extract.draft.json`](extract.draft.json) | The M3 extract artifact: text runs, `PdfLocator`, synthesized flags, the ligature caveat | §3, §4, §5, §6 |
| [`limitation.draft.json`](limitation.draft.json) | A named gap and how wide it reaches: profile, document, or one page | §7 |
| [`coverage.draft.json`](coverage.draft.json) | The M4 assurance envelope: per-page state, the coverage reconciliation, the terminal state | §7 |

## `not_detected` and `not_decoded` are gone

Both were declared stand-ins for capability machinery and both said so in their own `$comment`.
As of M4 there is **one** vocabulary: `assurance.limitations`, described by
`limitation.draft.json`. What M2 and M3 declared is still declared — the reasons travel verbatim
into limitation details — but a consumer no longer has to learn two shapes and work out which one
to trust. The migration is asserted rather than asserted-to: a test fails if either field
reappears on the wire, and another checks that every `NOT_DETECTED` entry became a limitation
carrying M2's own words.

## What is deliberately not here

**A `DocumentRepresentation v0` skeleton.** The companion spec settles the *node* shape (stable id,
kind, parent, ordinal, text/value, attributes, required `NativeLocator`, optional
`StructuralLocator`, optional `RenderedLocator`), and as of M3 the engine has real nodes —
`extract.draft.json` describes them. As of M4 the assurance half of the envelope exists too:
per-page state, the coverage summary, and the terminal state are in `coverage.draft.json`. What is
still missing is the **identity** half — the processing run and the representation fingerprint —
which is M5 work, together with the `ethos.grounding.v1` projection.

## Relationship to the Rust

The schemas were written from `crates/engine-core/src/`, not the other way round, and
`profile.draft.json`'s example is the **real** canonical profile — the same bytes and digest pinned
in `profile.rs`'s tests. They are documentation of a shape that already compiles, not a spec waiting
for an implementation.

They are **not** validated against the code by a test. Adding a JSON Schema validator dependency to
prove a DRAFT matches would be more machinery than a draft warrants; the pinned profile vector in
`profile.rs` is what actually catches drift in the one place drift matters.

**One exception, added at M4.** The claim two paragraphs up — that `profile.draft.json`'s example
is the *real* profile — turned out not to be true: the example still carried
`"unbound-until-m3"` placeholders two milestones after the backend landed. A claim that specific
either holds or should not be made, so
`contract_invariants.rs::the_profile_schema_example_is_the_real_profile` now compares the two by
`serde_json` equality. That needs no validator, and it is the only schema assertion in the tree.
