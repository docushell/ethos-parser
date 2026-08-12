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
| [`classification.draft.json`](classification.draft.json) | The M2 classification artifact: counts, the two reason axes, the derived boolean, `not_detected` | §2, §8, §9 |
| [`extract.draft.json`](extract.draft.json) | The M3 extract artifact: text runs, `PdfLocator`, synthesized flags, the ligature caveat, `not_decoded` | §3, §4, §5, §6 |

## What is deliberately not here

**A `DocumentRepresentation v0` skeleton.** The companion spec settles the *node* shape (stable id,
kind, parent, ordinal, text/value, attributes, required `NativeLocator`, optional
`StructuralLocator`, optional `RenderedLocator`), and as of M3 the engine has real nodes —
`extract.draft.json` describes them. What is still missing is the **envelope**: the processing run,
the representation fingerprint, per-page state, and the coverage summary. Those are M4 and M5 work,
and writing the envelope before the capability machinery exists would be an invented shape that
later work would have to argue with.

## Relationship to the Rust

The schemas were written from `crates/engine-core/src/`, not the other way round, and
`profile.draft.json`'s example is the **real** canonical profile — the same bytes and digest pinned
in `profile.rs`'s tests. They are documentation of a shape that already compiles, not a spec waiting
for an implementation.

They are **not** validated against the code by a test. Adding a JSON Schema validator dependency to
prove a DRAFT matches would be more machinery than a draft warrants; the pinned profile vector in
`profile.rs` is what actually catches drift in the one place drift matters.
