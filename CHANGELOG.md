# Changelog

All notable changes to ethos-engine. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

This project is pre-release and has no published versions. Entries are grouped by **milestone**
(`docs/05-MILESTONES.md`) rather than by version number, because a milestone is the unit of work
that has acceptance criteria.

## [Unreleased]

### M1 — Contract types, c14n / quanta, `schema_version` on the wire

`docs/01-CONTRACT.md` is now Rust. The artifact shape stops being negotiable and starts being a
compile error.

**Added — `engine-core`**

- `c14n` — c14n v1, a clean-room implementation of the contract Ethos implements. UTF-8, no
  whitespace, keys sorted **explicitly at write time**, minimal escaping, integers only,
  idempotent. Parity vectors lifted from Ethos's committed tests prove byte compatibility without
  taking a Cargo dependency on Ethos.
- `geom` — `quantize` (round-half-away-from-zero; `NaN`/`±∞`/overflow are errors, never clamps)
  and `QRect` as `[x0, y0, x1, y1]` with construction-time validation.
- `identity` — `ArtifactIdentity`, `ArtifactBinding`, `Sha256Hex`, `CoordinateSystem`.
- `profile` — `Profile`, `Capabilities`, `BackendIdentity`, and `profile_sha256()`.
- `derivation` — `DerivationClass` and `GeometryPresence` / `GeometryAbsence`.
- `ids` — `IdAllocator`, `NodeId`, `IdKind`, `sort_ids`.
- `error` — the six-variant `EngineError` taxonomy.

**Added — docs**

- `docs/draft-schemas/` — six DRAFT JSON Schemas plus an index. Under `docs/` deliberately; there
  is still no production `schemas/` path.
- This changelog.

**Decisions worth knowing**

- **`QRect` rejects zero-area rectangles**, which is *stricter* than Ethos, whose `QRect::new`
  rejects only `x0 > x1`. Stricter-on-emission is the only safe direction: the engine may refuse to
  emit what the oracle tolerates, never the reverse.
- **Typed absence, not `Option<QRect>`.** `None` would collapse "could not measure", "nothing to
  measure", and "not asked to measure" into one value, and only the first is a declarable capability
  limitation.
- **`Sha256Hex` rejects uppercase hex** rather than folding it, so one digest has exactly one
  spelling and a comparison never fails for formatting reasons.
- **The default profile is pinned by test**, bytes and digest. A profile change is now a deliberate
  act — including a crate version bump, since `parser_version` is part of identity by design.
- **`Unicode-3.0` added to `deny.toml`'s allowlist.** M1 is the PR that introduced the crate needing
  it: `serde`'s `derive` feature reaches `unicode-ident`, whose licence is
  `(MIT OR Apache-2.0) AND Unicode-3.0` — the `AND` makes it non-optional. Added on introduction
  rather than in advance, which is what the prior comment asked for.

**Not done, deliberately**

- No PDF parsing, classification, or extraction (M2, M3).
- No grounding projection (M5).
- `oracle_agrees_on_simple_text` still fails with its M1–M6 diagnostic, as designed until M6.

### M0 — Repo skeleton, toolchain, deny policy, failing oracle harness

- Four-crate workspace with enforced boundaries; toolchain pinned to 1.88.0.
- `deny.toml`: allowlist-only licences (no AGPL), no network crates, pinned registry.
- `fixtures/manifest.json` — hash-pinned references into the Ethos corpus across two roots.
- Oracle harness with negative-path tests; `oracle_agrees_on_simple_text` fails by design.
- CI: fmt, clippy, build, test, deny, plus a job that *proves* the AGPL gate rejects.
