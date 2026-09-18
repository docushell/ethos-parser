# 27 — `locate` slices

**Implementation authority for North Star decision #30.** Scope is in
[`26-LOCATE-SCOPE.md`](26-LOCATE-SCOPE.md), and every `locate` PR belongs to exactly one slice
below. The document stands under the assumption that the scope's §12 table holds; if the owner
overrules one of its settled calls, the affected slice is re-cut here before code moves.

**Written 2026-09-18 against local `main` at `fceaf98`, workspace 0.58.0 unreleased.** Nothing here
reopens v2.2, and nothing here changes what any reader reads out of any document.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and the scope beside it | done |
| **S1** | The core query, the profile field, and the cap measured | **done 2026-09-18** |
| **S2** | The three surfaces in one slice: CLI, MCP, both SDKs | **done 2026-09-18** |
| **S3** | The measurements that are not the cap's | not started |

**S1 lands before S2 on purpose**, for the reason
[`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) gives for its own order: the
offsets, the block boundary and the concatenation invariant are the things that can be wrong in a
way no adapter test would catch, and an adapter tested against a query that was never tested alone
is an adapter whose test can pass because both halves share a mistake.

---

## S0 — the documents

**Done.** [`26-LOCATE-SCOPE.md`](26-LOCATE-SCOPE.md) and this file, committed before any code, which
[`02-ROADMAP.md`](02-ROADMAP.md) requires of every version and which the block cut is the one
recorded exception to.

**Acceptance.** Both documents in the tree; `ci/doc-version.sh` and both forbidden-token greps exit
0; `OPEN-WORK.md` item 6.7 points at them.

## S1 — the core query, the profile field, and the cap measured

**New file:** `crates/ethos-parser-core/src/locate.rs`, and its `pub mod locate;` line in
`lib.rs`. `ethos-parser-core`, because the query has no PDF and no office concept in it (scope
§10).

**What it holds:** `locate(&DocumentRepresentation, &str) -> Result<Locations, EngineError>`;
`Locations`, `Occurrence`, `OccurrencePart`, `OccurrencesWithheld`; `LOCATE_RULE_V1`,
`LOCATE_MAX_QUOTE_BYTES`, `LOCATE_MAX_OCCURRENCES`, `LOCATIONS_ARTIFACT_TYPE`,
`LOCATIONS_SCHEMA_VERSION`. It calls `markdown::geometric_blocks` rather than restating the cut.

**What moves with it:**

- `crates/ethos-parser-core/src/profile.rs` — the `locate_rule` field, its rustdoc, and the
  `the_default_profile_is_pinned` re-pin, taken from the test's own output.
- `docs/draft-schemas/profile.draft.json` — the new field.
- `docs/PUBLIC-API.md` — the `ethos-parser-core` section gains the types and the constants.
- `crates/ethos-parser-grounding/tests/` — the equality test that
  `LOCATE_MAX_QUOTE_BYTES == check::limits::MAX_STRING_BYTES` and
  `LOCATE_MAX_OCCURRENCES == check::limits::MAX_ELEMENTS`. It lives in the grounding crate because
  that crate can see both, and core cannot see it.

**Acceptance.** Scope §9's T1 to T13 and T18 as library tests, T19's greps, the equality test above,
and **the cap confirmed or lowered on a measured number**: the size of a locations artifact at
1,000,000 occurrences, recorded in the commit message and in `docs/measurements/locate/`. No
surface ships on an unmeasured cap, which is why the measurement is here and not in S3.

**The invariant this slice exists to protect:** the parts' own slices, concatenated in order, equal
the quote exactly (T12). It fails the moment offsets are written against a block's text instead of
each node's.

**Amended 2026-09-18, on landing.** Done, with three deviations from the plan above and one
finding:

- **The cap is confirmed at 1,000,000**, not lowered: 133,778,286 bytes (127.6 MiB) at the ceiling,
  503 MiB of peak RSS and 2.09 s to produce, in
  [`measurements/locate/README.md`](measurements/locate/README.md) with the reasons.
- **The measurement's instrument is an `#[ignore]`d test**, not a script in
  `docs/measurements/locate/`, because the artifact it measures has no subcommand until S2 and a
  Python instrument would have had nothing to drive. `accuracy.rs`'s generator is the precedent.
- **The equality test landed as a unit test in `ethos-parser-grounding/src/check.rs`**, not in that
  crate's `tests/`, because `check::limits` is `pub(crate)`: widening a module so an integration
  test could read it would be a change to the crate for a test's convenience.
- **The measurement found a quadratic cost defect** in the first implementation — a block's member
  scalar counts were re-counted per match, so the ceiling's own worst case (one block of a million
  scalars holding a million matches) did not finish. The fix is a per-block prefix of member starts
  with `partition_point`. This is the argument for the rule that produced it: a cap defended by
  argument alone would have shipped on a code path that could not reach it.
- **The tests split by what they can prove.** The units, the refusals and the folding cases are
  unit tests over hand-built representations in `locate.rs` (T7, T10, T11 and the rule's own
  negatives); T1 to T6, T8, T9, T12, T13 and T18 are in `crates/ethos-parser-cli/tests/locate.rs`,
  which is where both readers are visible. T12 and T13 are asserted on **every** case rather than
  once, because an invariant checked on one document is an invariant nobody checked on the others.

## S2 — the three surfaces, in one slice

Decision #30: *"It ships as a CLI subcommand, an MCP tool and SDK functions in one slice."*

**The CLI.** `ethos-parser locate <representation> --quote-file <FILE>`, the eleventh subcommand.
Exit 0 answered, 2 could not read or refused, **and no exit 1 ever** (scope §6.1). The quote is read
verbatim through the bounded read — no trim, no BOM removal — and non-UTF-8 bytes are a named
refusal.

**The MCP tool.** A fourth tool beside `extract`, `ground` and `node_get`, taking `representation`
and `quote` and nothing else, `additionalProperties: false`. The two tool-count assertions
(`mcp.rs:833`, `mcp.rs:876`) move from three to four, which is how a fourth tool is meant to arrive
— through the line that says so. The model-facing summary is counts and nothing else, and the phrase
"not found" is refused in it.

**The SDKs.** `locate()` in Python and Node, both shelling out to the subcommand and neither porting
the match rule (scope §6.3). The quote reaches the child through a file, never argv.

**Every document that states a subcommand count moves with this slice**, each listed in scope §10:
`main.rs`'s header, `verifier.rs:41`, `diagnostics.rs:132`, `PUBLIC-API.md` (the count, the thin-shell
sentence and the mapping table), `04-ARCHITECTURE.md`, `docs/README.md`, `docs/CAPABILITY.md`, the
roadmap's v2.3 row, `OPEN-WORK.md` item 6.7, and a `locations.draft.json` beside the other draft
schemas.

**Acceptance.** Scope §9's T14 to T17 and T20, the CLI-equals-library test, and
`mcp_stdio.rs`'s `every_reply_in_a_session_is_the_reply_a_fresh_server_gives` covering a `locate`
call. `ci/gate.sh` and `ci.yml` do not change: the gate stays at nine steps, so
`the_local_gate_runs_what_ci_runs` needs nothing.

**Amended 2026-09-18, on landing.** Done. Four things the plan above did not anticipate, each a
decision rather than an accident:

- **The quote file's read is bounded by the quote's own ceiling**, not by `read_source`'s 2 GiB.
  Scope §6.1 said the source ceiling would apply first and the 16,384-byte limit would then refuse
  by name; it reads one byte past the quote ceiling instead, so the by-name refusal still fires
  with the number and a caller who points `--quote-file` at a document pays a refusal rather than a
  two-gigabyte allocation. `read_source`'s ceiling is a ceiling on a *source*, and a quote is not a
  source.
- **Both SDK suites were falsely green, and the guard that was supposed to prevent it said it
  could not be.** Each locator prefers `target/release/ethos-parser` and checked only
  `--version`, which cannot tell two builds of one *unreleased* version apart: a release build
  from 00:45 reported the same `0.58.0`, had no `locate` subcommand and advertised three MCP
  tools, and the parity tests below passed against it. Both locators now probe the subcommands
  the suites actually drive, reject a binary that cannot answer them, and name staleness apart
  from a version mismatch — and both doc comments, which claimed the false green was impossible,
  now say plainly that it was not and quote the measurement. **The gate is where this would have
  mattered**: step 6 builds debug and step 9 runs the SDK suites, which preferred the stale
  release build.
- **Both SDK suites' tool-list parity tests had to be loosened, deliberately.** Each asserted set
  equality between the tools `ethos-parser mcp` advertises and the LangChain adapters'
  `TOOL_SCHEMAS`, so a fourth MCP tool failed them. A LangChain tool for `locate` is refused in
  this slice (scope §6.3), so the assertion now requires every LangChain schema to equal the
  server's **and** the set difference to be exactly `{locate}` — the exclusion is named, so a fifth
  tool still fails. A bare subset check would have let the next tool arrive silently.
- **`locations.draft.json` carries its own fixture**, `untagged-shredded-line` with the quote
  `arrow`, rather than `simple-text` like the two projection schemas. `simple-text` cannot show the
  one thing an occurrence is for: three parts from three runs of one block.
- **Two corrections to the scope's own wire illustration**, found by printing the artifact: a
  measured box is `[x0, y0, x1, y1]` and not an object of named edges, and `occurrences_withheld`
  is omitted rather than null. Both are recorded at §3.2.

**And `CHANGELOG.md` does not move here**, though scope §10 listed it. Feature commits in this
repository do not touch the changelog; the release commit that cuts the version writes the entry,
which is where the 0.59.0 draft belongs.

## S3 — the measurements that are not the cap's

Occurrence counts for a realistic quote over `fixtures/gate`, the band with the worst document
named, and the wall time of a `locate` call on the largest of them. Recorded in
`docs/measurements/locate/` with the commands, as every measurement here is.

**Acceptance.** The figures in the tree, each re-derivable from the committed instrument.

---

## What is not a slice

- **Any change to the match rule.** `locate-scalar-exact-v1` is the rule; a normalised or folded
  rule is a `-v2` with its own scope, and never a flag on `-v1` (scope §4.1, §7).
- **A LangChain tool.** Refused for this slice and reopening on a named host (scope §6.3).
- **Searching a table cell's concatenated text.** The next `-v2` candidate, recorded in scope §12.
- **A committed non-BMP or combining-mark fixture.** T10 and T11 are hand-built representation
  tests; the standing item is in `OPEN-WORK.md` §5.
- **Anything verdict-shaped.** Scope §7 lists what is refused, by name, and the greps are the proof.
