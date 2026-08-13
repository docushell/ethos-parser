# Changelog

All notable changes to ethos-engine. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

Entries through M7 are grouped by **milestone** (`docs/05-MILESTONES.md`) rather than by version
number, because a milestone was the unit of work that had acceptance criteria. M7 ends that: v0 is
frozen at **0.1.0** and later entries are versions.

## [0.1.0] — v0, frozen

**Not tagged.** The freeze is in-tree; creating the tag and any release is a separate, deliberate
act. Everything below is committed and green under `cargo test --workspace --locked`.

### M7 — CLI + library freeze + v0 exit criteria as CI jobs

**No new capability.** M7 is the milestone that closes v0 rather than extending it: it makes the
exit criteria checkable, makes the public surface a decision, and adds the two test layers §5 asks
for. One behaviour change landed, and it is a hardening the mutation work found — see below.

#### Version: 0.0.0 → 0.1.0, and the profile hash moves

`parser_version` is a `Profile` field, so a version bump **is** a profile change:

```
profile_sha256  f34be632…6faf1e   ->   d2ebf3ef…6d21fc
```

That is the design working, not a regression. Artifacts produced before and after are correctly
non-comparable, because the profile that produced them really did change. Two pins moved with it:
`the_default_profile_is_pinned` and `docs/draft-schemas/profile.draft.json`'s example. Nothing else
in the suite depended on the value — the oracle comparison is over `structure`, `source_binding`,
`representation_sha256` and `counts`, none of which carry the profile.

#### Added — `--diagnostics`

Opt-in, global across all four subcommands, **off by default**.

- **stdout is untouched.** The artifact is byte-identical with the flag and without, and
  `the_flag_adds_one_stderr_line_and_changes_no_stdout_byte` asserts exactly that per subcommand.
  A default run writes *nothing* to stderr at all.
- **One JSON object on stderr**, carrying stage, engine version, wall-clock microseconds, input
  path and size, build target, and resident bytes where the platform reports one cheaply
  (Linux only; `None` elsewhere, which is a typed absence rather than a zero).
- **Not an artifact, deliberately.** No `artifact_type`, no `schema_version`, no `profile_sha256`,
  and not canonicalized — it uses plain `serde_json`, not `c14n`. An object that looked like an
  artifact would eventually be consumed like one, and then a timing would be inside somebody's
  hash.
- **New:** `engine_core::diagnostics` — `Diagnostics`, `DiagnosticsRun`, `HostInfo`, `Stage`,
  `DIAGNOSTICS_VERSION`. The library assembles the observation; the CLI only picks the stream,
  which is the whole of what a thin shell may do.
- `no_diagnostics_field_name_appears_in_any_artifact` walks every key of all four artifacts at
  every depth and asserts none is diagnostics-class. Checking that two runs match would have
  passed for an artifact carrying a `host` field on a machine where the host never changes.

#### Changed — the public API is now a list

`docs/PUBLIC-API.md` is new and enumerates every supported export per crate.
`crates/engine-cli/tests/public_api.rs` fails if a crate root and that document disagree in either
direction.

**`engine-pdf`'s parsing machinery is `pub(crate)`:** `ops`, `content`, `cmap`, `encoding`,
`fonts`, `metrics`, `text_state`, `thresholds`, `nodes`, `magic`, `classify`, `document`,
`extract`, `represent`, `reasons`. Everything a caller needs is re-exported at the crate root by
name. `exit` and `limitations` stay public — the latter because its constants are wire vocabulary
a consumer matches on after reading an artifact.

**Narrowing found dead code, which is the argument for narrowing.** With the modules public the
compiler could not see that these had no readers:

| Item | Disposition |
| --- | --- |
| `Font::subtype`, `Font::base_font` | **Removed.** Parsed and stored since M3, never read. `/BaseFont` is no longer read at all |
| `SimpleEncoding::base` | **Removed.** No callers anywhere |
| `ToUnicode::len` / `is_empty`, `Matrix::apply`, `Operator::token` / `ALL` | `#[cfg(test)]` — used only by the tests that prove the tables round-trip |

`tests/extraction.rs::an_unknown_operator_produces_no_artifact` was rewritten to go through the
public API — a real document with one operator token overwritten in place — instead of driving the
interpreter directly. That was the only thing keeping `ops` and `content` public, and the
replacement is a stronger test: it asserts the property a caller depends on.

#### Fixed — a fail-closed path that was the call site's guarantee, not the type's

`Interpreter::run` returned `Err` on an unrecognised operator and left everything shown before it
sitting in `self.shown`. **No artifact was ever wrong** — `extract` propagates with `?` and drops
the interpreter — but the guarantee lived at the call site, and the test meant to cover it passed
for the wrong reason: it ran a stream that had shown nothing yet, so it would have held even if
partial output were kept. `run` now clears `shown` and `undecodable` on the error path, and the
test shows real text first, with a control asserting the unmutated stream does produce output.

#### Added — fixture mutation over the whole manifest

`crates/engine-pdf/tests/robustness.rs`. All **23** manifest fixtures × **6** deterministic
mutations = 130 mutants; the 8 pairs that cannot be built are pinned with reasons rather than
skipped. Mutations: empty, truncate-to-16, a byte flipped in the last tenth, `%PDF` overwritten,
junk after `%%EOF`, and a same-length operator substitution injecting a token outside Table A.1.

Every mutant must either be refused with one of the six taxonomy codes, or read — and a mutant
that reads must bind to **its own** digest. The third outcome, an artifact that reads as though
the original had been parsed, is what the suite exists to forbid; nothing downstream could detect
it. No mutant panics.

**28 survivors, pinned and triaged into two classes.** `junk-after-eof` on everything that opens
(a reader reaches the trailer via `startxref`, so appended bytes are outside every declared
offset), and `flip-tail-byte` on four documents where `lopdf` recovers by scanning for the catalog
instead of trusting a damaged trailer reference — inspected, not assumed: on `synthetic/two-lines`
the flipped byte is the `t` of `/Root`. The same mutation refuses on eleven other fixtures.

**One triage finding is worth recording.** Widening the operator match to be whitespace-delimited
(half the corpus writes `(text) Tj\n`, which a space-delimited search missed entirely) made the
mutation appear to apply to the two NIST benchmarks. It was not applying: the hit in
`nist-sp-800-63b` is at offset 301887, inside a Flate stream, surrounded by binary. Overwriting it
corrupts compressed data and fails on *decompression* — the test would have gone green while
proving nothing about operator handling. Compressed documents are now excluded from that mutation.

#### Added — `cargo-fuzz` on the PDF entry point

`fuzz/`, **excluded from the workspace** so `libfuzzer-sys` never enters the graph `cargo deny`
inspects or `cargo build --workspace` compiles. Two targets, because libFuzzer's coverage feedback
is per-target and one binary that sometimes classifies and sometimes extracts explores both worse
than either alone:

- `open_and_classify` — `Document::open_bytes` → `classify` → `to_canonical_bytes`
- `open_and_extract` — the deep path through the interpreter, CMaps, font metrics and quantization

An `EngineError` is a pass; a panic is a release blocker. CI budget is 60s per target with
`-timeout=10` for hang prevention and `-max_len=65536`, on nightly **installed only in that job** —
the workspace MSRV stays 1.88. Four degenerate seeds are committed in `fuzz/seeds/`;
`fuzz/seed-corpus.sh` adds the five engine-owned fixtures at run time. The Ethos conformance corpus
is not seeded from: those fixtures are read-only and referenced by hash, and a fuzz corpus is a
copy.

#### Added — `docs/03-V0-SCOPE.md` §5 as fifteen named CI jobs

A matrix in `.github/workflows/ci.yml`, one entry per criterion, each named after the criterion so
a reviewer sees which line is green. `check` still runs the whole suite as the umbrella gate and is
deliberately not the proof — one tick cannot tell you the classification bound still holds.

`v0-classify-bound` is its own job so a timing wobble is legible as that gate rather than as an
unexplained suite failure, and runs `--exact --test-threads=1`. The load-bearing assertion remains
the instrumented counter (`pages_content_scanned == 8` on 492 pages), which cannot flake.

Two criteria are greps, so they are greps: `ci/forbidden-tokens.sh` runs `v0-no-confidence` and
`v0-no-verify` over `crates/*/src`, needing no toolchain. It strips `//` comments (the repo argues
these rules at length in prose) and the `mod tests` block (two of them list the banned tokens *as
data*). **The first version skipped from `mod tests` to end of file** on the reasoning that test
modules come last; they do, and it was still wrong — a probe appended after one sailed through
clean. It now resumes at the closing brace.

`crates/engine-cli/tests/v0_exit_criteria.rs` closes the loop: every §5 line is ticked and names a
job, every named job exists, every matrix entry is claimed by a criterion, **no `--skip` appears
anywhere in CI**, and **no job's filter matches zero tests**.

The last two are the ways this scheme could go hollow while staying green. A `--skip` is the
cheapest way to turn a red criterion green and it deletes the criterion in the process. A filter
naming a renamed test is quieter still: `cargo test -- a_renamed_test` selects nothing, libtest
prints `ok. 0 passed`, and the job passes having checked nothing at all — with every box still
ticked and every job still present. Every filter token in the workflow is now required to occur
inside some test function's name.

#### Added — library-only thin-shell proofs

`crates/engine-cli/tests/library_surface.rs`. The existing CLI tests assert the binary's stdout
equals the library's bytes, which on its own is circular: it proves the two agree, not that the
library alone can produce the artifact. If a subcommand grew a step the CLI performed itself, both
sides would include it and both tests would pass. These do not spawn the binary, and
`no_test_in_this_file_spawns_the_binary` asserts that about the source.

Also new there: `every_geometry_bearing_artifact_declares_its_coordinate_system`, which asserts the
§5 line in both directions — the representation and grounding artifacts declare a frame, and the
classification, which carries no geometry, does not.

#### Documentation

- **`docs/PUBLIC-API.md`** — new. Three crate tables, what is internal and why, and the CLI↔library
  thin-shell mapping with the test names on both sides.
- **`README.md`** — rewritten. It said "M3 complete" three milestones later. Now states v0 frozen,
  the four subcommands, the §5 job map, and an explicit performance posture: the only quantitative
  claim is the bound-test counter, and no bake-off table appears anywhere.
- **`docs/03-V0-SCOPE.md` §5** — all fifteen boxes ticked, each naming its job, plus a §5.1 table
  of what each job actually runs.
- **`docs/README.md`**, **`docs/05-MILESTONES.md`** — M7 marked done; next work is v0.1, a roadmap
  item rather than an M-number.
- **`docs/07-VERIFY-BOUNDARY.md`** — re-read, unchanged. It still describes reality:
  `grounding-check` validates structure and binding, nothing verifies a claim, and the boundary is
  now a CI job rather than a review item.

### M6 — `grounding-check` validator + oracle agreement + double-run identity

**The bare `cargo test --workspace --locked` is green for the first time since the repo existed.**
`oracle_agrees_on_simple_text` was written in the first commit to fail, with a diagnostic naming
what was missing; it failed for six milestones and now runs the comparison it always described.

**Added — `engine-grounding::check`**

- `grounding_check(grounding_json, source_pdf_bytes) -> ValidationReport`, plus `ValidationReport`,
  `Structure`, `SourceBinding`, `Counts`, `ReportError`. Emits `ethos.grounding_validation.v1`.
- **Structure and binding only.** No claim, no verdict, no `grounded`, no evidence tier, no quote
  matching, and nothing re-derived from a verifier's report. A grep test over the check path
  enforces it — with the unit-test module cut out first, because a test that asserts those tokens
  are absent has to name them, and a scan that cannot tell a rule from its enforcement fires on
  itself. (It did, once.)

**Added — `engine grounding-check [--source-artifact <pdf>]`**

The last unimplemented subcommand. `no_subcommand_claims_to_be_unimplemented` replaces the test
that used to assert the opposite — inverted rather than deleted, because the property still
matters in the other direction.

**`representation_sha256` is the hash of the grounding file's raw bytes**

Measured from `../ethos/crates/ethos-core/src/grounding_json.rs`, where `parse_grounding_json`
does `hash.update(bytes)` on the slice it was handed, and confirmed by running the binary: for our
own artifact it returned exactly `sha256sum g.json`. It is **not**
`DocumentRepresentation::representation_c14n_sha256`, which digests a representation's payload
subtree — a different input for a different purpose. The two sharing most of a name is precisely
why M5 renamed ours, and a test pins the distinction: appending a newline to an artifact changes
this digest while leaving `counts` identical, because it follows the bytes and not the meaning.

**Why the checker mirrors Ethos's parser and not just the pinned schema**

The schema is necessary and not sufficient. Id uniqueness, reference resolution, page ordering,
boxes inside their page, capability/array agreement, character offsets that index their element's
text, table cell occupancy — none of it is expressible in JSON Schema, and a checker validating
only the schema would call artifacts valid that the oracle calls invalid. The rules are
transcribed **in Ethos's order**, because the order decides which code an artifact with two faults
reports, and the harness compares the code and the path, not just the verdict.

That fidelity work found six real gaps, all fixed rather than tolerated. The first surfaced on the
checker's own first run: `source.sha256` was typed as a self-validating `Sha256Hex`, so a
malformed digest was rejected during deserialization and reported at path `/`, where Ethos reports
`invalid_field` at `/source` — the same verdict at the wrong place, which is half an agreement.
The wire type is now a plain string and the strictness lives in the checker at Ethos's path;
emission is unaffected, because `project()` still builds it from a validated digest.

The other five were found by **hunting for divergence** rather than by testing the inputs that
came to mind, and every one of them agreed on `structure` while disagreeing on the reason:

| input | engine said | Ethos says |
| --- | --- | --- |
| a float where an integer belongs | `invalid_field` `/` | `invalid_json` `/` |
| an integer past `2^53-1` | `invalid_invariant` `/pages/0` | `limit_exceeded` `/` |
| an oversized multibyte string | `limit_exceeded` `/elements/0/text` | `limit_exceeded` `/` |
| an unknown field inside a page | `unknown_field` `/` | `unknown_field` `/pages/0/zzz` |
| `null` where a string belongs | `invalid_field` `/` | `invalid_json` `/` |

The cause was structural: `deny_unknown_fields` and typed deserialization reach the right verdict
by the wrong route, and can only ever say `/`. Ethos refuses these **before** any field is typed,
in a strict value pass, and then walks the value tree to report an unknown field's exact path. The
checker now does both, in that order, and a `adversarial_inputs_agree_on_verdict_code_and_path`
test pins nine such inputs against the live oracle so they cannot come back.

**One ordering difference survives and is documented rather than chased**: Ethos's deserializer is
streaming, so an artifact broken in *two* different ways reports whichever fault appears first in
the bytes, while the engine's walk reports whichever rule comes first in its own order. Both call
such an input invalid with an error present, and all four compared fields are identical — only the
code can differ, and only for an artifact that is already broken twice.

**Oracle matrix**

| | |
| --- | --- |
| Ethos-owned fixtures compared and agreed | **11 / 15** |
| Fixtures this backend cannot read at all | **4** — `table-regular-grid` (19-byte xref), `corrupt-header-valid`, `invalid-header`, `password-protected` |

The four are excluded **visibly**: the harness requires the agreed and refused lists to partition
the corpus exactly, prints each refusal with its reason, and a separate test asserts each still
exits 2 with no artifact on stdout. A fixture that started quietly emitting an empty artifact
would move between the lists and be caught.

**Exit codes: finer than Ethos, never contradictory**

| Outcome | engine | Ethos |
| --- | --- | --- |
| valid + `matched` / `not_checked` | 0 | 0 |
| valid + `mismatched` | **1** | 2 |
| invalid structure | **1** | 2 |
| could not read the input | 2 | 2 |

Both agree on zero versus non-zero, which is what a shell predicate reads. Where they differ the
engine keeps its own taxonomy (`03-V0-SCOPE.md` §3.1): 1 is "I read it and the answer is no", 2 is
"I could not read it". Collapsing those is the LiteParse defect this project exists to refuse. The
oracle criterion is agreement on the **report**, and the report distinguishes them either way.

**The built oracle is older than its own source, and the harness had to handle it**

The `ethos` binary in the sibling tree prints the validation report bare. The committed source —
and the ref CI pins, `ETHOS_ORACLE_REF` — wraps it in an in-toto Statement with the report at
`predicate`. So the local run and the CI run see different shapes. `extract_report` reads either
and **refuses anything else rather than guessing** which field holds the verdict; hunting for the
first object with a `structure` key would be exactly the best-effort parsing of an unrecognised
shape §8 forbids. A unit test feeds it both envelopes, because the local run alone would never
exercise the wrapped path.

**Double-run byte identity, on files**

`classify → extract → ground → grounding-check`, twice, into separate directories, comparing bytes
on disk across every openable conformance fixture. Not parsed equality — a value comparison passes
while the files differ by key order or a trailing newline, and the contract is about artifacts a
consumer stores.

**CI, and the docs that still taught people to bypass it**

`--skip oracle_agrees_on_simple_text` is deleted from the workflow, and so is the informational
`continue-on-error` step that used to report the oracle's expected failure. The bare command is the
gate now.

The audit caught the embarrassing half of that: the **root `README.md`** — the first thing a new
reader runs — still printed the `--skip` command and still said "next milestone: M4". The CI
comment added by this change says re-adding a skip "would delete the only thing that proves this
engine and the verifier read an artifact the same way", while the front door taught exactly that.
Corrected, along with `01-CONTRACT.md` §11 and `05-MILESTONES.md` M6, which both still claimed
agreement "across all 15 fixtures", and M6's "In" line, which still specified **JSON Schema
validation only** — a decision this milestone deliberately reversed and never recorded.

`ORACLE_AGREED_COUNT = 11` is now pinned beside `ETHOS_OWNED_FIXTURE_COUNT = 15` and asserted, for
the reason the 15 already was: the number is quoted in three docs, and without the assertion a
regression that refused six more documents would move them quietly to the refused list and leave
the suite green.

**What the corpus agreement can and cannot show — stated plainly, because the number reads stronger
than it is**

All 15 Ethos-owned fixtures are standard-14 Helvetica with no font descriptor, so every node's
geometry is typed-absent and **every grounding artifact the corpus produces is `1 page / 0 elements
/ 0 spans`**. Eleven agreements over eleven copies of that shape exercise the identity, source,
coordinate-system, capability and page rules — and never reach the element loop, the span loop, the
offsets rule or the table rules, which is most of what the checker does. An adversarial audit found
this and it was the right catch. `the_oracle_agrees_on_an_artifact_with_real_elements_and_spans`
now compares a benchmark document that projects 1975 elements and 1975 spans, and breaks a rule
*inside* the element loop to prove both checkers locate it identically. It is a benchmark document
rather than one of the 15, so it does not touch the oracle count.

**Six more fidelity gaps, found by audit, all fixed**

The differential corpus above was written by the same person who wrote the checker, which is a weak
form of evidence. An independent audit drove ~119 crafted artifacts through both binaries and found
six divergences the corpus missed:

| input | engine said | Ethos says |
| --- | --- | --- |
| `kind` longer than 256 bytes | `invalid` | **`valid`** |
| a table cell's `text` past the byte limit | **`valid`** | `invalid` |
| `rotation: -90` | `invalid_invariant` `/pages/0/rotation` | `invalid_field` `/` |
| a *value* containing the text "duplicate field" | `duplicate_key` | `invalid_field` |
| an invalid artifact plus a non-PDF source | no report at all | a full report |

Two of those are the serious kind. **The cell-text hole let the engine say `valid` where the
verifier says `invalid`** — the one direction `01-CONTRACT.md` §11 forbids, and the exact failure a
consumer would hit by shipping an artifact this engine had blessed. **The `kind` limit was invented
here**: 256 comes from the JSON Schema, Ethos's parser has no length bound on `kind` at all, and
for a *checker* the oracle wins — being stricter than the verifier does not make the engine safer,
it makes the two disagree. The `rotation` field is now `u16`, matching Ethos's own type, so the
refusal happens at the same stage. Error classification no longer substring-matches text a
*document* can control.

The last one was a comment asserting the opposite of the code: `check.rs` claimed the PDF magic
check ran "before the artifact parses… the same order Ethos uses". Ethos parses first and reads the
source second, and the difference was visible — an invalid artifact with an unusable source got a
report from the oracle and nothing from the engine. Order corrected, comment corrected.

**Fixed — a harness bug that looked like a product bug**

Two oracle tests walk the same corpus and Cargo runs them in parallel threads of one process, so
scratch directories named from the pid and fixture id collided: one test deleted the working
directory of the other mid-run, and it surfaced as "the engine printed no report". An atomic
counter makes the isolation real rather than probable. Worth recording because the symptom
pointed squarely at the wrong component.

### M5 — `DocumentRepresentation v0` emit + `ethos.grounding.v1` adapter

The canonical evidence record, and the projection a verifier consumes. `engine extract` now emits
the record; `engine ground` projects it.

**Added — `engine-core`**

- `representation` — `DocumentRepresentation`, `RepresentationPayload`, `PageRecord`, `Node`,
  `NativeLocator` (the contract's discriminated union, `Pdf` variant only), `StructuralLocator`,
  `NodeKind`, `TextRunAttributes`, `NodeGeometry`, `ProcessingRun`, `SourceIdentity`.
- **The fingerprint is the digest of a literal subtree**, `representation_c14n_sha256` =
  `"sha256:" + hex(sha256(c14n(doc["representation"])))`. A reader recomputes it with no domain
  knowledge. The alternative — hash the document minus a named key set — makes canonicalization a
  second rule two implementations can drift on, and a drifted rule produces a mismatch that looks
  exactly like tampering.
- **Named `representation_c14n_sha256`, not `representation_sha256`**, because Ethos already uses
  that name for a hash of a grounding **file's raw bytes**. Two different things under one name is
  how a consumer concludes tampering where there is only a naming collision.
- **Geometry sits outside the fingerprint** (§4), implemented by keeping the type out of the
  payload rather than by filtering at hash time — a filter is a rule someone can quietly change; a
  type that is not there cannot be hashed by accident. Two mirrored tests: moving a box does not
  move the digest, and moving a text origin does.
- **A measured box outside its page is a hard error.** Reachable, not theoretical: an ink box is
  `baseline_y − ascent × size` in a top-left system, so a baseline within one ascent of the page
  top yields a negative `y0` that `QRect::new` accepts. Clamping fabricates; omitting would have to
  travel the geometry-omission path, which takes a typed absence by construction. So it is refused.
- **Pages are records, not nodes.** A node carries a required `NativeLocator` whose PDF variant is
  a *character* origin. A page has none, and `{"origin_x":0,"origin_y":0}` for a page is
  indistinguishable on the wire from a run drawn in the corner.

**Added — `engine-pdf`**

- `represent::to_representation` — `ExtractArtifact` → record. `extract()` keeps its signature and
  its artifact, so M3's thirty-odd behavioural tests still assert on what the parser produces;
  `every_extracted_run_appears_exactly_once` is the bridge that stops the two drifting.

**Added — `engine-grounding`** (was an M0 skeleton)

- `project()` → `Projection { source, omission }`, and `to_canonical_bytes()`.
- **The omission rule is a type, not a convention.** `GroundedBox` has a private field and one
  constructor, `from_presence(GeometryPresence)`. There is no `GroundedBox::new(x0, y0, x1, y1)`,
  so "omit because the page looked bad" would require *adding* a constructor. An audit showed the
  first version of that guard — a source scan — could be walked past by writing the new
  constructor in the obvious shape, so `GroundedBox` now lives in its own module with the field
  private to it: `project()` cannot construct one either, and the guarantee is the compiler's
  rather than a grep's.
- The crate depends on `engine-core` alone and **never reads a locator at all**: the projection
  addresses pages by node id. A test fails if it so much as mentions `NativeLocator`, which is a
  stronger guarantee than hiding the type in `engine-pdf` would have given.

**Added — schema conformance without a new dependency**

- A **JSON Schema subset validator driven by the pinned schema file itself**, so it cannot drift
  from the rules it enforces. It fails loudly on any keyword it does not implement — a validator
  that silently ignores a keyword reports success over rules it never checked — and
  `the_validator_rejects_each_deliberate_break` runs 17 artifacts broken one rule at a time.
  Without that corpus, every positive conformance test would be satisfied by a validator that
  returns `Ok(())`.
- `crates/engine-grounding/schemas/` holds a byte-for-byte snapshot of Ethos's schema with its
  origin and digest recorded, plus a drift check against `../ethos` when that tree is present.

**Measured, and it is the most important thing in this milestone**

**Across the entire Ethos conformance corpus, zero runs have measurable geometry.** Every fixture
is standard-14 Helvetica with no `/FontDescriptor`, so every grounding artifact projected from the
corpus is `elements: []`, `spans: []`, with the whole run count omitted. The engine reads the text
correctly and the record holds it with native locators intact — none of it can cross into a schema
that requires a `bbox`. The geometry-omission path is therefore the **normal** path, not an edge
case, and `TODO(confirm with Ethos owners)` about an optional `bbox` now has a number behind it
rather than a hypothesis.

**Aligned with Ethos's runtime validator, measured from its source rather than guessed**

Beyond the JSON Schema, `ethos-core/src/grounding_json.rs` enforces rules the schema cannot
express, and emitting something it would reject would be a landmine for M6:
`capabilities.spans` ⟺ the `spans` array is present; `capabilities.tables` ⟺ `tables` is present
(so `tables: false` means the key is **absent**, not an empty array); offsets present ⟺
`char_offsets`; boxes must lie inside their page; page indices ascend from 1. All are asserted on
emitted artifacts.

**`capabilities.char_offsets` stays `false`, and M5 is where that was settled**

The milestone allowed flipping it once a hierarchy existed. The hierarchy now exists and the answer
is still no: v0 does no line grouping, so an element and a span are the *same object* and an offset
would always be `0..len` — advertising sub-element addressing the engine cannot do. Ethos's own
validator ties the capability to the fields, so claiming it would oblige every span to carry them.
It flips at v1, with grouping. Five doc sites that promised "flips at M5" were corrected.

**Fixed — an overclaim caught before it shipped**

The first draft of `fixtures/README.md` said the new `absent-font-metrics` fixture was the only one
pairing a known advance with an absent box. Measured, that is false: `/Widths` has always been
supplied to every engine fixture, so three M3 fixtures already produce seven such runs. The
fixture's real and narrower contribution is the `from_descriptor → None` route — a descriptor that
**resolves and answers nothing** — which nothing else reaches, and which is the shape most likely to
tempt a `height = font_size` fallback.

**Clarified — `docs/04-ARCHITECTURE.md` §1**

"No PDF concept in `engine-core`" as written forbids something `01-CONTRACT.md` §5.1 requires: the
`NativeLocator` union with a `PdfLocator` variant. The architecture doc's own header says the
contract wins, so §1 now states the line as **machinery, not vocabulary** — no `lopdf`, no
operator, no page tree, no font program; a contract-defined locator variant carrying integers is
data. `engine-grounding` is held to the stronger rule and a test enforces it.

**Fixed after an adversarial audit, and the findings are worth recording**

A multi-agent audit ran the real `ethos grounding check` against every emitted artifact — **13/13
`structure: valid`, `source_binding: matched`**, including a 2-page real form projecting 1975
elements. The emitter was right. The *tests* were not, and six of them passed while the behaviour
they were named for was mutated away:

- **The geometry sidecar was unauthenticated in a way that mattered.** It sits outside the digest
  by §4, but the projection drops locators and keeps boxes — so the one thing the fingerprint did
  not cover was the only spatial claim reaching `ethos.grounding.v1`. Flipping a row
  `Measured` → `Absent` made a node vanish with no declaration; `Absent` → `Measured` emitted a
  fabricated box while the record still declared the node unmeasurable. Both passed
  `verify_fingerprint`. **`check_structure` now binds the sidecar to the payload's own
  `geometry-absent-not-groundable` declaration**, closing both, and the trust boundary is stated
  where a reader will meet it: *a verified representation attests to the text, the order and the
  origins — not to the rectangles.*
- **`GroundedBox`'s "the type system says it" claim was false.** The private field stopped other
  crates while `project()`, in the same module, could build a box from anything; the grep meant to
  cover that gap was walked past by writing `fn from_raw(x0, y0, x1, y1) -> Self`. The type now
  lives in its own module, and the bypass **fails to compile**: `tuple struct constructor
  GroundedBox is private`.
- **`omission_selects_rather_than_empties` did not test selection.** It compared two all-or-nothing
  documents, so an emitter that dropped every box as soon as any node lacked one passed the whole
  suite — the exact §11 hole. It now runs on a mixed-geometry document and asserts the emitted span
  ids are *exactly* the nodes with measured geometry.
- **Nothing checked that an emitted bbox was the measured one.** A fabricated `[x0, y0, x0+1,
  y0+1]` satisfied shape and containment and passed. Now compared against the node's own rectangle,
  over 100+ boxes.
- **`lossiness_is_asserted_not_assumed` was vacuous** — pointed at a fixture whose artifact had no
  elements, so the dropped field names could not have appeared however the projection behaved.
  Retargeted, with a non-empty guard.
- **Three boundary guards read `src/lib.rs` alone**, so a second file in the crate was invisible to
  all of them. Now recursive.
- **A limitation shipping inside every artifact still said the hierarchy "lands at M5"** — it had
  landed. Corrected to the real reason.

Each of the first four fixes was re-verified by re-applying the audit's mutation and watching the
named test fail.

**Not done, deliberately**

- **No `grounding-check`, no oracle agreement.** That is M6. This milestone produces artifacts; it
  does not validate them as a product feature. The subset validator is test-layer only, and a test
  asserts it never becomes a runtime dependency.
- No tables, no char offsets, no multi-column reordering, no fabricated geometry.

### M4 — Capabilities, typed absence, explicit multi-column limitation (the L1 gate)

L1's achievement condition names capability declarations explicitly, so an artifact without them
has not reached "extracted" regardless of how good its text is. Every emitted artifact now
declares what the profile can do, what it could not do, what happened to each page, and how the
run ended.

**Added — `engine-core`**

- `assurance` — `Limitation` (stable kebab code + detail + `Profile | Document | Page(n)` scope),
  `PageState`, `PageStateEntry`, `CoverageSummary`, `ProcessingGaps`, `ProcessingTerminalState`,
  `RefusalCode`, and the `Assurance` envelope both artifacts embed.
- `Assurance::new` **derives** the coverage summary and the terminal state from the page states it
  is given. An artifact that claims `Complete` while carrying an unread page is not a bug this
  type can have — which is the only way to guarantee `docs/01-CONTRACT.md` §7's rule that a
  verification over a partially processed document never renders as a clean verification of the
  whole document.
- `CoverageSummary` reconciles: `pages_authorized == processed + failed + unsupported +
  quarantined + not_attempted`. Six buckets, not five: **`not_attempted` is the one a four-bucket
  summary would have had to lie about.** Bounded classification samples `N` pages and stops, so
  folding the rest into `processed` would claim observations nobody made and folding them into
  `failed` would claim failures that never happened.
- `page_binding_status` — **capability-limited beats negative** (Workbench rule 4). A query bound
  to a failed, quarantined, or unattempted page returns `CapabilityLimited { limitation_code }`,
  never a boolean "not present". The signature is the enforcement: there is no way to express
  "missing", so absence of extractable content cannot become evidence of absence in the source.
  `NotInDocument` is a separate answer, because "we skipped it" and "there is no such page" are
  different facts.
- `Capabilities::declared_limitations` — the mirror of the proof rule: **no capability may be
  `false` without a declared limitation**, derived by exhaustive destructuring so adding one
  without a code is a compile error.
- `PageBudget` on `Profile` — `Unlimited` or `AtMost(n)`, a **declared state rather than an
  optional field**. An `Option` missing from incoming JSON deserializes to `None` and silently
  re-hashes as though it had been there; a declared enum cannot.

**Added — `engine-pdf`**

- `limitations` — the format-specific codes `engine-core` is not allowed to know about:
  `backend-xref-strict-20-byte`, `predefined-cmaps-not-vendored`,
  `form-xobject-text-not-descended`, `font-widths-absent`, `classify-sample-bound`, and the
  derived `*-reason-not-detected` pair.
- The **xref refusal is declared on artifacts for documents it did not refuse.**
  `synthetic/table-regular-grid` exits 2 with no body, so the only place a caller can learn this
  backend turns away roughly one document in twenty-six is an artifact for one it accepted.

**Changed — the wire, deliberately**

- **`not_detected` (M2) and `not_decoded` (M3) are gone.** Both were declared stand-ins. Their
  content is now `assurance.limitations`, carrying M2's and M3's reasons verbatim — a test fails
  if any `NOT_DETECTED` entry loses its declaration in the move, and another fails if either field
  reappears. Two vocabularies on one artifact means a consumer has to work out which to trust, and
  the answer is never written down.
- **`capabilities.char_offsets`: `true` → `false`.** v0 emits runs and no element/span hierarchy,
  so there is nothing for an offset to index into. M4 asked for the proof and there was none.
  Flips at M5 with `DocumentRepresentation v0`, with a test.
- **`profile_sha256` moved** to
  `sha256:f34be6328f858e09c241cb51c7b0dbb0fe065ecbf5f00e9942cd6bbcdd6faf1e` — the honest
  `char_offsets` value plus the new `page_budget` knob. Artifacts from before and after are
  correctly non-comparable, because the profile that produced them really did change.
- `Classification::pages_sampled` is now `min(page_count, classify_sample_pages, page_budget)`, so
  it keeps meaning "the pages this run intended to read" and stays equal to
  `pages_content_scanned`.

**`failure/memory-limit-simulated`, stated plainly**

Its bytes are **identical to `synthetic/simple-text`** (`sha256:f2f6ab91…`, asserted in the test
rather than taken on trust). The fixture name means *a limit simulated by configuration*, not
*this PDF is huge*, so the test sets `page_budget` explicitly — and on a one-page document the
only budget that bites is zero. The behaviour is a **declared limitation plus a coverage gap**,
not a hard refusal: one authorized page, zero processed, one quarantined, terminal state
`partial`, and no page tree at all. The same bytes under the default profile read cleanly, which
is the proof the gap came from the knob.

**Fixed — two guards that were not guarding**

- The profile sensitivity test's `capabilities.char_offsets` mutation wrote back the value the
  field already held once `V0` flipped, so it proved nothing while passing. Every mutation now
  asserts it actually changed the profile before asking whether the hash moved.
- `no_pdf_type_or_import_in_engine_core` banned the **prefix** `struct Page`, which read the
  format-agnostic `PageStateEntry` as a PDF page-tree type — forbidding a type the contract
  requires. Banned type names are now matched as whole identifiers, with a test proving the
  exact-match form still catches `struct Page {`.
- `profile.draft.json`'s example still carried `"unbound-until-m3"` placeholders two milestones
  after the backend landed, while its README told readers the example *is* the real profile. The
  example is corrected and `the_profile_schema_example_is_the_real_profile` now holds it there.

**Not done, deliberately**

- **No CLI flag for the page budget.** It is a profile knob, so the binary cannot emit a partial
  artifact at v0 and the question of which exit code one deserves does not arise. Exit-code
  meanings are unchanged.
- **`pages_failed` and `pages_unsupported` are always zero in v0 artifacts.** Extraction fails
  closed on a hard error and emits nothing at all, so no page reaches those states through the
  public path. The buckets are declared with honest zeros and the query helper handles them, but
  nothing pretends they are exercised.
- No repair, no invented pagination, no capability claimed without a proof test.

### M3 — Extract: text runs, native locators, fail-closed operators, synthesized flags

Position-aware text runs whose origins are trustworthy, whose boxes are measured or typed-absent,
and whose parse stops rather than silently dropping content.

**Added — `engine-pdf`**

- `ops` — all **73** operators of PDF 32000-1 Table A.1 as an enum. Dispatch is an exhaustive
  match with **no wildcard arm**: adding a variant without handling it is a compile error, and a
  token outside the table is a hard error naming it. Operators that do not move a glyph are
  *named* no-ops, so "we do not interpret `rg`" is a decision in the source rather than an
  accident of a `_ =>`.
- `content` — the interpreter, including all four show-text operators. **`"` and `'` are
  implemented**; `"`'s absence from pdf-inspector's match is the disqualifying defect, where text
  vanishes and adjacent runs merge with corrupt geometry while the output stays well-formed.
- `text_state` — CTM, text and line matrices, `Tc`/`Tw`/`Tz`/`TL`/`Ts`. **`Tz` multiplies the
  whole advance expression**, spacing terms included; pdf-inspector implements it nowhere.
- `fonts`, `metrics` — three independent questions per glyph (what character, how far, what box),
  because they fail independently. Conflating the last two is how `height = font_size` gets
  written.
- `cmap` — `ToUnicode` parsing (`bfchar`, `bfrange`, codespace ranges), the source of the ligature
  caveat: `<03>` → `<00660069>` is one code and two characters.
- `encoding` — `WinAnsiEncoding` in full, `StandardEncoding`'s ASCII range including the two codes
  where it is *not* ASCII (`0x27` is `quoteright`, `0x60` is `quoteleft`), and `/Differences`.
- `nodes` — `TextRun` with `PdfLocator`, `SynthesizedChar`, `scalar_code_mismatch`.
- `engine extract <pdf>` — canonical JSON, exit 0 or 2. **No exit 1**: "needs attention" is a
  classify concept, and overloading it would make a caller's `&&` chain mean two different things
  depending on which subcommand ran.

**Honesty, and where it shows**

- **Ink boxes are measured or absent.** The whole conformance corpus is standard-14 Helvetica with
  no `FontDescriptor`, so every run reports `NotReportedByReader` — the typed-absence path, proved
  on real documents rather than a mock. One engine-owned fixture carries a descriptor so the
  measured path is proved too.
- **Absent advance is absent, not zero.** A standard-14 font may omit `/Widths` and expect built-in
  AFM metrics, which this profile does not vendor. The advance is omitted from the wire; the origin
  is unaffected, because it comes from the content stream.
- **Hyphenation is not rejoined**, and that is a stated policy with a golden. Distinguishing a soft
  break-hyphen from a real compound hyphen needs a dictionary, and a rule that guesses is the
  cliff-shaped heuristic refused everywhere else. Silent rejoining is forbidden outright.
- **Reading order is stream order.** `two-columns` comes out right-column-first — visibly wrong,
  and asserted that way, because that is what `single-column-v1` means.

**Changed**

- **`skrifa` replaces `ttf-parser`**, which the milestone named. RUSTSEC-2026-0192 records that
  ttf-parser's author declared it unmaintained with no safe upgrade, and names skrifa as the
  maintained successor. Pinned to 0.39 because 0.44's tree needs Rust 1.89 and this workspace pins
  1.88.
- `Profile.cmap_data_version`: `absent-until-m3` → `annex-d-encodings-1`, naming what is actually
  carried. The pinned profile digest moved accordingly.
- The float-ban guard now matches **whole tokens**. It was matching substrings, and a sha256
  digest containing `…cf32c2e…` tripped it — a guard that cries wolf on a hash is a guard someone
  eventually disables.

**Deviation from the milestone text, stated plainly**

M3 called for vendoring ~168 Adobe `.bcmap` CMaps. They are **not** vendored. Nothing in the corpus
exercises them, they could not be obtained and verified in this pass, and committing binary data no
test touches would be worse than declaring the gap. A document naming a predefined CMap is
**refused** with a named error. The same applies to the Core-14 AFM widths and the full Adobe Glyph
List. All three are declared in `not_decoded` and explained in `vendor/README.md`.

**Not done**

- M4 capability blocks and coverage summaries; M5 `DocumentRepresentation v0` and the grounding
  projection; M6 oracle agreement.

### M2 — Classify: reason codes, two axes, three exit codes, bounded sampling

`engine-pdf` opens a PDF once and reports what it observed. No verdict, no confidence, no quality
field — the engine owns the observation, the caller owns the routing policy.

**Added**

- `engine_pdf::Document` — magic-checked, opened once, shared by every stage. M3's `extract` takes
  the same handle rather than reopening: two loads can disagree, and a classifier that saw a
  different object graph from the extractor is a silent divergence with no diagnostic.
- `engine_pdf::classify` — per-page counts, two orthogonal reason axes, a derived boolean, and
  `pages_content_scanned` as an observable bound.
- `OcrNeedReason` / `LayoutComplexityReason` — **separate types**, so `table-likely` cannot be
  constructed as an OCR-need reason. A dense financial table in crisp born-digital text needs no
  OCR at all; a single "complex" list would send it to an engine that could only make it worse.
- `engine classify <pdf>` — canonical JSON on stdout, exit 0 / 1 / 2. `extract`, `ground` and
  `grounding-check` exit 2 naming the milestone that owns them, rather than printing usage and
  exiting 0 as though the work happened.
- `docs/draft-schemas/classification.draft.json`.

**Measured, and it changed two acceptance lines**

- **`irs-form-1040-2025` is exit 1, not exit 0.** It fires `table-likely` and `dense-graphics`.
  Suppressing a true layout reason to make a doc line come out right is the tuning this project
  refuses, so the fixture choice changed: `synthetic/simple-text` is the exit-0 case, and
  `irs-form-1040` became the axis-independence case, which it serves better.
- **The 20%-total-time bound was unachievable, and the reason was informative.** The counter proved
  the page walk was already bounded — 492 pages, 8 scanned — while `lopdf` parses the whole object
  graph eagerly, so total cost is `O(parse) + O(N × per-page)`. Timing the phases separately shows
  what the claim actually is: **246× the pages for 1.7× the classify time.**

**Honesty about what is not detected**

`garbled` and `multi-column` are in the vocabulary and are **never emitted**, because no sound
detector exists for either. The artifact declares both in `not_detected` with the reason — silence
would let a caller read an empty list as evidence a document is not garbled.

`sparse-text` requires short text **alongside imagery**. LiteParse uses `text_length < 20`
unconditionally and compounds it with a vowel-frequency garble heuristic that strips items from the
tally first, so an acronym-dense page can be reported as `no-text` when its text extracted
perfectly. Neither mechanism exists here, and a test asserts `nist-sp-800-53r5` — a control
catalogue full of `AC-2`/`SC-7` — is not reported as textless.

**Changed**

- `Profile.backend.version` is now the resolved `lopdf` version, `0.44.0`, replacing
  `unbound-until-m3`. The pinned profile digest moved accordingly — the mechanism working as
  designed.
- `deny.toml` gains `BSD-3-Clause` (`lopdf` → `encoding_rs`; also the licence the vendored Adobe
  CMaps will need at M3), plus `Unlicense`, `Zlib` and `0BSD` from the same subtree. Each added in
  the PR that introduced the crate needing it.

**Not done**

- Text extraction: content-stream interpretation, CMaps, ink boxes, `NativeLocator` emit (M3). The
  classifier walks content streams to **count** operators; it decodes no text and interprets
  nothing.

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

- **`quantize` uses `f64::round`, not Ethos's `(x + 0.5).floor()`.** That idiom double-rounds: once
  `|x| ≥ 2^52` the sum is unrepresentable and rounds *before* `floor` runs, so an exact integer
  product returns one quantum too large — and `MAX_SAFE_INT`, which the contract declares canonical,
  is refused outright. `f64::round` is IEEE `roundToIntegralTiesToAway`: same rule, computed
  exactly. Measured divergence from Ethos across an exhaustive knife-edge sweep of `[0, 2·10^6)`:
  **exactly one value**, `0.49999999999999994`, where this returns `0` (correct — it is below one
  half) and Ethos returns `1`. Unreachable from decimal text, and across all ten million
  `0.001`-step literals in `[0, 10000)` points the two agree everywhere.
- **`QRect` rejects zero-area rectangles**, which is *stricter* than Ethos, whose `QRect::new`
  rejects only `x0 > x1`. Stricter-on-emission is the only safe direction: the engine may refuse to
  emit what the oracle tolerates, never the reverse.
- **`quantize` rejects `quantum_per_point == 0`**, which would otherwise map every coordinate on
  the page to the origin and return `Ok(0)` while doing it.
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

**Fixed after adversarial review**

- **Unknown-field denial now covers every nested object, not the ones someone remembered.**
  `deny_unknown_fields` is not recursive, and the first pass stopped one level short: a profile
  carrying `coordinate_system.future_knob` parsed cleanly and **re-hashed to the unmodified default
  digest** — measured — so an artifact would claim comparability with a profile it does not match.
  Now on `CoordinateSystem`, plus `ArtifactIdentity`, `ArtifactBinding` and `GeometryPresence`,
  matching the `additionalProperties: false` their draft schemas already declared. The nested test
  derives its field list from the serialized value rather than a hardcoded array, so a nested
  object added later is covered automatically.
- **`DerivationClass::may_be_overwritten_by` implemented only half of §6.** It ignored its second
  argument entirely, so it protected `Extracted` while letting `Proposed` overwrite `Computed`,
  `Recognized`, and other `Proposed` nodes — the path by which a model's suggestion quietly becomes
  the record. Both rules are now enforced and pinned by a full 4×4 matrix, plus a test asserting
  the result actually depends on the overwriter.
- `Profile` now denies unknown fields. Without it a profile from a newer engine deserialized with
  its unknown knob silently dropped, then **re-hashed to a different digest than it arrived with** —
  an artifact claiming comparability it does not have.
- The profile sensitivity gate now destructures to the leaf. A field added to `Capabilities` or
  `BackendIdentity` is as output-affecting as one on `Profile`; a gate stopping at
  `capabilities: _` waved it straight through.
- The float ban was scoped to `quantize`'s body rather than the whole of `geom.rs`. The file-level
  skip was a hole big enough for `QRect` — which lives there, derives `Serialize`, and would have
  passed with an `f64` field. Verified by injecting one and watching the guard fire.
- `sort_ids` put a malformed id **first** while its doc claimed last, because `Option`'s natural
  order sorts `None` first. Now sorts last, with a test.

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
