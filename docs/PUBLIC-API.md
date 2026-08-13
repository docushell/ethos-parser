# PUBLIC-API — the frozen v0 surface

**Status:** frozen at M7 (v0.1.0) · **Enforced by:** `crates/engine-cli/tests/public_api.rs`
**Rule:** if this document and the crate roots disagree, that test fails. Neither one is allowed to
move alone.

---

## What "frozen" means here

Every item below is **intended to be supported**: it is what a caller may depend on, and changing
it is a deliberate, changelogged act. Everything not listed is internal — not "undocumented but
usable", not "fine if you need it". Internal items may be renamed, narrowed, or deleted in any
release without a note.

The freeze is **narrowing only**. M7 removed items from the surface; it added `engine-core`'s
`diagnostics` module and nothing else. A later milestone may add, but v0.x will not silently take
away.

Three things are worth saying plainly, because each is a promise that costs something to keep:

1. **`engine-cli` exports nothing.** It is a binary. There is no `engine_cli::` anything, and the
   CLI's argument shapes are not a library contract.
2. **The parsing machinery is private.** The operator table, the content-stream interpreter, font
   and CMap resolution, encoding tables, text state and calibration thresholds were `pub` through
   M6 and are `pub(crate)` from M7. They carry `f64` fields, borrow-scoped handles and constants
   whose values are implementation decisions; publishing them would have frozen the inside of the
   parser along with its contract.
3. **Narrowing found dead code, which is the point.** With the modules public the compiler could
   not see that `Font::base_font`, `SimpleEncoding::base` and three accessors had no readers.
   Those are gone; the test-only ones are behind `cfg(test)`. See the M7 CHANGELOG entry.

---

## `engine-core` — the contract, in types

The largest surface, deliberately: this crate *is* `docs/01-CONTRACT.md`, so every artifact-shaped
type a caller reads or writes belongs here. All modules are public; the table names the modules and
the items re-exported at the crate root.

| Module | Supported items |
| --- | --- |
| `c14n` | `c14n_bytes`, `sha256_hex`, `sha256_hex_bytes`, `C14nError` |
| `geom` | `quantize`, `QRect`, `QRectError`, `QuantizeError`, `MAX_SAFE_INT`, `QUANTUM_PER_POINT` |
| `identity` | `ArtifactIdentity`, `ArtifactBinding`, `Sha256Hex`, `CoordinateSystem`, `CoordinateOrigin`, `CoordinateUnit` |
| `profile` | `Profile`, `profile_sha256`, `BackendIdentity`, `Capabilities`, `PageBudget`, `XrefRepair`, `VerifierPin`, `CMAP_DATA_VERSION`, `READING_ORDER_RULE_V0` |
| `derivation` | `DerivationClass`, `GeometryPresence`, `GeometryAbsence` |
| `assurance` | `Assurance`, `Limitation`, `LimitationScope`, `PageState`, `PageStateEntry`, `CoverageSummary`, `ProcessingGaps`, `ProcessingTerminalState`, `RefusalCode`, `PageBindingResult`, `page_binding_status`, `codes` |
| `representation` | `DocumentRepresentation`, `RepresentationPayload`, `Node`, `NodeKind`, `NodeGeometry`, `PageRecord`, `NativeLocator`, `PdfLocator`, `StructuralLocator`, `SourceIdentity`, `ProcessingRun`, `ProcessorIdentity`, `SynthesizedAt`, `TextRunAttributes`, `REPRESENTATION_ARTIFACT_TYPE`, `REPRESENTATION_SCHEMA_VERSION` |
| `ids` | `NodeId`, `IdAllocator`, `IdKind`, `sort_ids` |
| `error` | `EngineError` — the six-variant taxonomy |
| `diagnostics` | `Diagnostics`, `DiagnosticsRun`, `HostInfo`, `Stage`, `DIAGNOSTICS_VERSION` — **new at M7** |
| `verifier` | `VerifierBinary`, `relay`, `RelayRequest`, `Relayed`, `GROUNDING_ADAPTER`, `RELAY_OK`, `RELAY_REFUSED`, `RELAY_UNAVAILABLE` — **new at v0.1** |

Plus `CRATE_NAME`, which exists so the M0 link harness can assert the workspace builds.

**Internal, do not use:** nothing. Every module in this crate is contract.

**The `verifier` module is v0.1, and what it does *not* export is the point.** There is no report
type, no claim, no check, no result — the engine spawns a verifier and forwards its bytes without
reading them, so there is nothing to model. `VerifierBinary::resolve` locates and pins one,
`relay` runs it, and `Relayed` carries the child's stdout, stderr and a mapped exit code. The
three `RELAY_*` constants keep "the verifier refused" (1) apart from "the run did not happen" (2),
for the same reason the classify exit codes never collapse.

**Two further additions at v0.1**, both on `Profile` and both hash-bearing:

- `XrefRepair` — whether the one bounded cross-reference repair runs. It decides *which documents
  produce an artifact at all*, which makes it the strongest output-affecting knob in the set.
- `VerifierPin` — which verifier a run was bound to. `NotPinned` is the default and is a real
  statement rather than a missing field: the engine does not verify, and a run that consulted no
  verifier says so. `engine verify` pins the binary it spawned by version and digest.

Both are adjacently tagged (`{"mode": …}`), matching `PageBudget`. That is not cosmetic: serde
does not honour `deny_unknown_fields` on an internally tagged enum, so an internally tagged
variant would accept an unknown key, drop it, and re-hash to a digest different from the one it
arrived with.

**One caveat that is not about visibility.** `diagnostics` is public but is *not* an artifact type.
It has no `artifact_type`, no `schema_version`, no `profile_sha256`, and it is not canonicalized.
Do not persist it as a record or feed it to a fingerprint — `docs/01-CONTRACT.md` §4, and
`crates/engine-cli/tests/diagnostics.rs` asserts the separation.

## `engine-pdf` — the PDF reader

| Kind | Supported items |
| --- | --- |
| Handle | `Document` — `open`, `open_bytes`, `source_sha256`, `byte_len`, `page_count` |
| Stages | `classify`, `extract`, `to_representation` |
| Stage artifacts | `Classification`, `PageClassification`, `SourceRef`, `ExtractArtifact`, `PageExtract`, `TextRun`, `SynthesizedChar`, `SynthesisReason`, `PdfLocator` |
| Reason vocabulary | `OcrNeedReason`, `LayoutComplexityReason` |
| Format detection | `check_pdf_magic` |
| Modules | `exit` (`SIMPLE`, `NEEDS_ATTENTION`, `COULD_NOT_READ`, `exit_code`) · `limitations` (limitation-code constants and builders) |
| Constants | `CLASSIFICATION_ARTIFACT_TYPE`, `CLASSIFICATION_SCHEMA_VERSION`, `EXTRACT_ARTIFACT_TYPE`, `EXTRACT_SCHEMA_VERSION`, `PROCESSOR_NAME`, `CRATE_NAME` |

**Internal, do not use:** `ops`, `content`, `cmap`, `encoding`, `fonts`, `metrics`, `text_state`,
`thresholds`, `nodes`, `magic`, `classify`, `document`, `extract`, `represent`, `reasons` as
*modules*. The items named above are re-exported at the crate root and that is the address to use;
the module paths are not.

`limitations` stays public on purpose: its constants are **wire vocabulary**. A consumer matching
on the `code` field of a limitation it read out of an artifact needs the same strings the engine
wrote, and retyping them by hand is how a consumer silently stops matching.

## `engine-grounding` — the projection and its validator

| Kind | Supported items |
| --- | --- |
| Projection | `project`, `Projection`, `OmissionReport` (with `is_lossy`), `to_canonical_bytes` |
| Artifact | `GroundingSource`, `Source`, `Producer`, `GroundingCapabilities`, `GroundingCoordinateSystem`, `Page`, `Element`, `Span`, `Table`, `Cell` |
| Emittable geometry | `GroundedBox`, with `from_presence` and `to_array` — and `from_presence` is its **only** constructor, taking a `GeometryPresence` |
| Validator (`check`) | `grounding_check`, `ValidationReport`, `Structure`, `SourceBinding`, `Counts`, `ReportError` |
| Constants | `GROUNDING_ARTIFACT_TYPE`, `GROUNDING_SCHEMA_VERSION`, `GEOMETRY_ABSENT_OMITTED`, `VALIDATION_ARTIFACT_TYPE`, `VALIDATION_SCHEMA_VERSION`, `CRATE_NAME` |

**Internal, do not use:** `grounded_box` as a module. It is private already, and that privacy is
load-bearing rather than tidy — `project()` lives outside it and therefore cannot construct a
`GroundedBox` from anything but a measurement state. See the crate docs.

## `engine-cli` — the binary

**Exports nothing.** `engine` is a `[[bin]]`; there is no library target and no supported
`engine_cli::` path. The CLI's four subcommands, their flags and their exit codes are a *product*
contract described in `docs/04-ARCHITECTURE.md` §2, not a Rust one.

---

## The thin-shell mapping

`docs/03-V0-SCOPE.md` §1 item 19 and `docs/04-ARCHITECTURE.md` §2 both require that the CLI be a
thin shell — that **every subcommand behaviour be reachable through the library**. This table is
that mapping, and the right-hand column is what a caller writes instead of spawning a process.

| Subcommand | Library call | Library-only proof | CLI-equals-library proof |
| --- | --- | --- | --- |
| `engine classify <pdf>` | `Document::open` → `engine_pdf::classify` → `Classification::to_canonical_bytes` | `library_surface.rs::classify_is_reachable_and_canonical_from_the_library` | `classify_cli.rs::the_cli_output_matches_the_library` |
| `engine extract <pdf>` | `Document::open` → `engine_pdf::extract` → `engine_pdf::to_representation` → `DocumentRepresentation::to_canonical_bytes` | `library_surface.rs::extract_and_represent_are_reachable_and_canonical_from_the_library` | `grounding.rs::the_cli_path_matches_the_library` |
| `engine ground <repr>` | `serde_json::from_slice::<DocumentRepresentation>` → `verify_fingerprint` → `engine_grounding::project` → `engine_grounding::to_canonical_bytes` | `library_surface.rs::project_is_reachable_and_canonical_from_the_library` | `grounding.rs::the_cli_path_matches_the_library` |
| `engine grounding-check <json> [--source-artifact <pdf>]` | `engine_grounding::grounding_check` → `ValidationReport::to_canonical_bytes` / `exit_code` | `library_surface.rs::grounding_check_is_reachable_and_canonical_from_the_library` | `oracle.rs::oracle_agrees_on_all_ethos_owned_fixtures` |

`--diagnostics` is the one flag with no library equivalent to call, because it *is* the shell's
job: `engine_core::diagnostics::DiagnosticsRun` assembles the observation and the CLI chooses the
stream. A library caller constructs it directly.

The exit-code mapping is library-owned too — `engine_pdf::exit::exit_code` for `classify` and
`ValidationReport::exit_code` for `grounding-check` — so a caller embedding the engine routes
outcomes the same way the binary does, rather than re-deriving the rule.

---

## Changing this surface

1. Change the crate root.
2. Change this document.
3. `cargo test -p engine-cli --test public_api` goes green, or one of the two was forgotten.
4. Say why in `CHANGELOG.md`. An export is a promise; adding one is cheap and removing one is not.
