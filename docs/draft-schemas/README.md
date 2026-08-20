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
| [`document-representation.draft.json`](document-representation.draft.json) | **The M5 canonical record**: identity, source, processing run, pages, ordered typed nodes, the fingerprint, and the geometry sidecar | §2, §4, §5, §7 |
| [`markdown.draft.json`](markdown.draft.json) | **The v1.1 projection**: the Markdown string, the Anchor Map that inverts it, the character census, and the structural erasures GFM causes | §12, `10-V11-SCOPE.md` §4 |
| [`html.draft.json`](html.draft.json) | **The v1.1-S4 second projection**: the HTML string, the same Anchor Map, the same census — and the merges GFM had to flatten, carried as `rowspan`/`colspan` | §12, `10-V11-SCOPE.md` §4 |

## `not_detected` and `not_decoded` are gone

Both were declared stand-ins for capability machinery and both said so in their own `$comment`.
As of M4 there is **one** vocabulary: `assurance.limitations`, described by
`limitation.draft.json`. What M2 and M3 declared is still declared — the reasons travel verbatim
into limitation details — but a consumer no longer has to learn two shapes and work out which one
to trust. The migration is asserted rather than asserted-to: a test fails if either field
reappears on the wire, and another checks that every `NOT_DETECTED` entry became a limitation
carrying M2's own words.

## The grounding schema is not here, and that is deliberate

`ethos.grounding.v1` is **owned by Ethos**, not by this repo, so it is not redrafted here. A
byte-for-byte snapshot lives at
[`crates/engine-grounding/schemas/`](../../crates/engine-grounding/schemas/) with its origin path
and digest recorded, and it is the file the conformance tests actually validate against. Writing a
DRAFT of someone else's shipped schema would create a second, drifting description of a contract we
do not control.

## What is deliberately not here

~~**A `DocumentRepresentation v0` skeleton.**~~ **Landed at M5** —
`document-representation.draft.json`. The companion spec settles the node shape (stable id, kind,
parent, ordinal, text/value, attributes, required `NativeLocator`, optional `StructuralLocator`,
optional `RenderedLocator`); M3 produced real nodes, M4 added the assurance envelope, and M5 added
the identity half — the processing run and the representation fingerprint — plus the projection.

**Two divergences from the companion remain open**, both engine-local and both additive:

- **Typed `GeometryAbsence`.** The companion models geometry as an optional field and never names
  a variant for *why* it is absent. Typed absence carries strictly more information and projects
  down to an omitted field cleanly. Pending DocuShell review.
- **Pages are records, not nodes.** The companion's tree is "ordered typed nodes"; a page in this
  representation is a `PageRecord` instead, because a node carries a required `NativeLocator` whose
  PDF variant is a *character* origin — which a page does not have, and inventing one is
  forbidden. Projects to `pages[]` exactly as the companion's node tree would.

## Relationship to the Rust

The schemas were written from `crates/engine-core/src/`, not the other way round, and
`profile.draft.json`'s example is the **real** canonical profile — the same bytes and digest pinned
in `profile.rs`'s tests. They are documentation of a shape that already compiles, not a spec waiting
for an implementation.

They are **not** validated against the code by a test. Adding a JSON Schema validator dependency to
prove a DRAFT matches would be more machinery than a draft warrants; the pinned profile vector in
`profile.rs` is what actually catches drift in the one place drift matters.

**Four exceptions. The first three were added because the claim above had already stopped being true; the fourth was added in the slice that added its schema, which is the rule those three taught.**
None needs a validator; each is `serde_json` equality on the one field a reader would act on.

| test in `contract_invariants.rs` | what it pins | why it exists |
| --- | --- | --- |
| `the_profile_schema_example_is_the_real_profile` (M4) | the whole example object | it still carried `"unbound-until-m3"` placeholders two milestones after the backend landed |
| `the_representation_schema_pins_the_version_the_code_emits` (v1-S3) | `schema_version`, `artifact_type` | the schema said `0.1.0` while the code had said `0.2.0` since v1-S1 — the profile had a guard and this did not, which is why only the profile stayed honest |
| `the_markdown_schema_pins_the_version_and_rule_the_code_emits` (v1.1-S3) | `schema_version`, `artifact_type`, and the `markdown_rule` example | `markdown.draft.json` arrived at v1.1-S1 with no guard, and `markdown_rule` moved at S2 and again at S3 — the one field on that artifact whose whole job is to say which projection produced it |
| `the_html_schema_pins_the_version_and_rule_the_code_emits` (v1.1-S4) | `schema_version`, `artifact_type`, and the `html_rule` example — plus that the two projections do not share a rule id | added WITH the schema rather than after it drifted, which is the whole point of writing the pattern down |

The pattern is the point: a schema without a guard drifts, and the drift is invisible to exactly
the reader the file is for. A new draft schema gets one in the slice that adds it.

## The worked examples are regenerated, not maintained — one rule for both projections

**A worked example in this directory is the real artifact a named fixture produces at the release
the example names.** Not a sketch of one, and never a hand-edited digest. Both projection schemas
carry one, both describe the same document, and both are reproduced by three commands:

```bash
engine extract  "$ETHOS_FIXTURES/synthetic/simple-text/document.pdf" > repr.json
engine markdown repr.json    # the whole `examples[0]` of markdown.draft.json
engine html     repr.json    # the whole `examples[0]` of html.draft.json
```

`ETHOS_FIXTURES` defaults to the `conformance` root `fixtures/manifest.json` declares. The fixture
is `synthetic/simple-text`, the M0 oracle fixture, chosen because it is the smallest document that
still exercises a `source` segment, a `syntax` segment and a census that balances.

**The rule exists because half of it was already being followed and the half that was not went
unnoticed for sixteen releases** (v2-S10.2). `html.draft.json` had its `parser_version` and
`profile_sha256` dragged forward release by release while its `representation_sha256` stayed where
it was, so its three identity fields described three different builds; `markdown.draft.json` was
left whole at `0.13.0`. The two files therefore published **two different digests for the same
representation of the same source**, and at most one of them could ever have been right. Both are
now regenerated together, and a release that moves `parser_version` moves all three fields in both
files or the examples stop being artifacts.

**Nothing enforces this.** The four guards in the table above pin `schema_version`, `artifact_type`
and the two rule ids — the fields a consumer branches on — and no guard covers an identity block.
That is a known gap, recorded in `docs/15-V2-MILESTONES.md` under v2-S10.2, and it is exactly the
shape the table above says a draft schema should have had from the slice that added it.
