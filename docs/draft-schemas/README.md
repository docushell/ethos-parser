# Draft schemas

**None of these is a shipped contract.** [`../01-CONTRACT.md`](../01-CONTRACT.md) is the authority;
where these disagree with it, the contract is right. Every file says `DRAFT` in its `$comment`.

They were written *from* the Rust in `crates/ethos-parser-core/src/`, not the other way round — they
document a shape that already compiles, rather than specifying one to be built.

| File | Covers |
| --- | --- |
| [`artifact-identity.draft.json`](artifact-identity.draft.json) | `artifact_type`, `schema_version`, `parser_version`, `profile_sha256` |
| [`coordinate-system.draft.json`](coordinate-system.draft.json) | The declaration every artifact with geometry carries |
| [`profile.draft.json`](profile.draft.json) | Every knob that affects output. Its example is the real v0 profile |
| [`derivation-class.draft.json`](derivation-class.draft.json) | `extracted` / `computed` / `recognized` / `proposed` |
| [`geometry.draft.json`](geometry.draft.json) | `QRect` as `[x0, y0, x1, y1]`, and typed absence |
| [`error-taxonomy.draft.json`](error-taxonomy.draft.json) | The six routable error kinds |
| [`classification.draft.json`](classification.draft.json) | The `classify` artifact: counts, the two reason axes, the derived boolean |
| [`extract.draft.json`](extract.draft.json) | Text runs, locators, synthesized flags, the ligature caveat |
| [`limitation.draft.json`](limitation.draft.json) | A named gap and how far it reaches: profile, document, or one page |
| [`coverage.draft.json`](coverage.draft.json) | Per-page state, the coverage reconciliation, the terminal state |
| [`document-representation.draft.json`](document-representation.draft.json) | The canonical record: identity, source, processing run, pages, ordered nodes, fingerprint, geometry sidecar |
| [`markdown.draft.json`](markdown.draft.json) | `ethos.markdown.v1` — the string, the anchor map, the census, and what GFM erases |
| [`html.draft.json`](html.draft.json) | `ethos.html.v1` — the same, with merges kept as `rowspan`/`colspan` |

The grounding schema is **not** drafted here. `ethos.grounding.v1` is owned by Ethos, and writing a
draft of someone else's shipped schema would create a second, drifting description of a contract we
do not control. A byte-for-byte snapshot lives in
[`crates/ethos-parser-grounding/schemas/`](../../crates/ethos-parser-grounding/schemas/).

## Every draft needs a guard

These files are not validated against the code wholesale — adding a JSON Schema validator to prove a
*draft* matches would be more machinery than a draft warrants. Instead each has a small test pinning
the one or two fields a reader would actually branch on:

| Test in `contract_invariants.rs` | Pins |
| --- | --- |
| `the_profile_schema_example_is_the_real_profile` | The whole example object |
| `the_representation_schema_pins_the_version_the_code_emits` | `schema_version`, `artifact_type` |
| `the_markdown_schema_pins_the_version_and_rule_the_code_emits` | `schema_version`, `artifact_type`, `markdown_rule` |
| `the_html_schema_pins_the_version_and_rule_the_code_emits` | The same, plus that the two projections do not share a rule id |

The first three were added *after* the file they guard had already drifted. The fourth was added in
the same slice as its schema, which is the lesson those three taught: **a schema without a guard
drifts, and the drift is invisible to exactly the reader the file is for.** A new draft gets one in
the commit that adds it.

## Worked examples are regenerated, not maintained

A worked example here is the **real artifact** a named fixture produces at the release it names —
never a sketch, and never a hand-edited digest. Both projection schemas carry one, both describe the
same document, and three commands reproduce them:

```bash
ethos-parser extract  "$ETHOS_FIXTURES/synthetic/simple-text/document.pdf" > repr.json
ethos-parser markdown repr.json    # the whole examples[0] of markdown.draft.json
ethos-parser html     repr.json    # the whole examples[0] of html.draft.json
```

The fixture is `synthetic/simple-text` — the smallest document that still exercises a `source`
segment, a `syntax` segment, and a census that balances.

This rule exists because half of it was already being followed and the other half went unnoticed for
sixteen releases. The two files ended up publishing **different digests for the same representation
of the same document**, and at most one could have been right. Regenerate them together, or the
examples stop being artifacts. **Nothing enforces this** — the guards above cover version and rule
ids, not identity blocks. That gap is known and recorded.

## Two open divergences from the DocuShell companion spec

Both are engine-local, both additive, and neither blocks anything:

- **Typed `GeometryAbsence`.** The companion models geometry as a plain optional field and never
  says *why* it is absent. Typed absence carries strictly more information and projects down to an
  omitted field cleanly.
- **Pages are records, not nodes.** Every node carries a required locator whose PDF form is a
  *character* origin — which a page does not have, and inventing one is forbidden. A page is a
  `PageRecord` instead, and projects to `pages[]` exactly as a node tree would.
