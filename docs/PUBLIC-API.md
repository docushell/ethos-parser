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
| `c14n` | `c14n_bytes`, `canonical_bytes_of`, `sha256_hex`, `sha256_hex_bytes`, `C14nError` |
| `geom` | `quantize`, `QRect`, `QRectError`, `QuantizeError`, `MAX_SAFE_INT`, `QUANTUM_PER_POINT` |
| `identity` | `ArtifactIdentity`, `ArtifactBinding`, `Sha256Hex`, `CoordinateSystem`, `CoordinateOrigin`, `CoordinateUnit` |
| `html` | `to_html`, `HtmlArtifact`, `HTML_ARTIFACT_TYPE`, `HTML_SCHEMA_VERSION`, `HTML_RULE_BLOCKS_V2` |
| `markdown` | `to_markdown`, `AnchorMap`, `Segment`, `SegmentKind`, `Coverage`, `DroppedBucket`, `StructuralErasure`, `MarkdownArtifact`, `MARKDOWN_ARTIFACT_TYPE`, `MARKDOWN_SCHEMA_VERSION`, `MARKDOWN_RULE_BLOCKS_V2`, `GFM_SPAN_SLOTS_UNREPRESENTABLE`, `GFM_ROW_ZERO_SEPARATOR`, `GFM_CELL_RUN_CLAIMED_TWICE`, `GFM_CELL_NOT_PLACED`, `GFM_TABLE_NOT_PROJECTED`, `GFM_LIST_ITEM_RUN_JOINS` |
| `profile` | `Profile`, `profile_sha256`, `BackendIdentity`, `Capabilities`, `PageBudget`, `XrefRepair`, `VerifierPin`, `TableDetection`, `CMAP_DATA_VERSION`, `READING_ORDER_RULE_V0`, `READING_ORDER_RULE_V1`, `OBSERVATION_RULE_V1`, `TEXT_CODE_RULE_V1`, `RasterDpi`, `NOT_RUN`, `DOCX_READING_ORDER_RULE_V1`, `DOCX_TEXT_CODE_RULE_V2`, `XLSX_READING_ORDER_RULE_V1`, `XLSX_TEXT_CODE_RULE_V2`, `PPTX_READING_ORDER_RULE_V1`, `PPTX_TEXT_CODE_RULE_V2`, `ODT_READING_ORDER_RULE_V1`, `ODT_TEXT_CODE_RULE_V2`, `ODS_READING_ORDER_RULE_V1`, `ODS_TEXT_CODE_RULE_V2`, `ODP_READING_ORDER_RULE_V1`, `ODP_TEXT_CODE_RULE_V2`, `RTF_READING_ORDER_RULE_V1`, `RTF_TEXT_CODE_RULE_V1`, `EPUB_READING_ORDER_RULE_V1`, `EPUB_TEXT_CODE_RULE_V1`, `TABLE_DETECTION_V1`, `TABLE_DETECTION_V2`, `TABLE_DETECTION_V3`, `TABLE_DETECTION_UNRULED_V1`, `TABLE_DETECTION_STROKE_V1`, `TABLE_DETECTION_TAGGED_V1`, `STRUCT_TREE_RULE_V1`, `FORM_ANNOTATION_RULE_V1` |
| `derivation` | `DerivationClass`, `GeometryPresence`, `GeometryAbsence` |
| `assurance` | `Assurance`, `Limitation`, `LimitationScope`, `PageState`, `PageStateEntry`, `CoverageSummary`, `ProcessingGaps`, `ProcessingTerminalState`, `RefusalCode`, `PageBindingResult`, `page_binding_status`, `codes` |
| `representation` | `DocumentRepresentation`, `RepresentationPayload`, `Node`, `NodeKind`, `NodeGeometry`, `PageRecord`, `NativeLocator`, `PdfLocator`, `DocxLocator`, `XlsxLocator`, `PptxLocator`, `OdtLocator`, `OdsLocator`, `OdpLocator`, `RtfLocator`, `EpubLocator`, `PdfObjectLocator`, `StructuralLocator`, `PdfTaggedLocator`, `PdfArtifactLocator`, `AnnotationRect`, `NodeAttributes`, `OfficeRunAttributes`, `OfficeCellAttributes`, `OfficeSlideRunAttributes`, `OfficeParagraphAttributes`, `OdfBlockKind`, `OfficeOdfCellAttributes`, `OfficeOdfShapeAttributes`, `RtfParagraphAttributes`, `RtfParagraphBreak`, `EpubBlockAttributes`, `OdfValueType`, `OdfCellTextSource`, `CellValueType`, `CellTextSource`, `FormFieldAttributes`, `AnnotationAttributes`, `FieldValue`, `SourceIdentity`, `ProcessingRun`, `ProcessorIdentity`, `SynthesizedAt`, `TextRunAttributes`, `TextFinding`, `PdfImageLocator`, `PaintedRect`, `ImageAttributes`, `ImageMediaType`, `REPRESENTATION_ARTIFACT_TYPE`, `REPRESENTATION_SCHEMA_VERSION` |
| `ids` | `NodeId`, `IdAllocator`, `IdKind`, `sort_ids` |
| `error` | `EngineError` — the six-variant taxonomy |
| `diagnostics` | `Diagnostics`, `DiagnosticsRun`, `HostInfo`, `Stage`, `DIAGNOSTICS_VERSION` — **new at M7** |
| `verifier` | `VerifierBinary`, `relay`, `RelayRequest`, `Relayed`, `GROUNDING_ADAPTER`, `RELAY_OK`, `RELAY_REFUSED`, `RELAY_UNAVAILABLE` — **new at v0.1** |
| `tables` | `CellSlot`, `TableCellPosition`, `SlotCover`, `SlotFault`, `TableRecord`, `TableCellRecord`, `LocatorCheck`, `CheckStatus`, `GeometricFault`, `LOCATOR_CHECK_V1` — **new at v1-S1** · `TaggedGridCheck`, `TaggedGridStatus`, `TaggedGridFault`, `TAGGED_GRID_CHECK_V1` — **new at v1-S3** |

Plus `CRATE_NAME`, which exists so the M0 link harness can assert the workspace builds.

**Internal, do not use:** nothing. Every module in this crate is contract.

**The `tables` module is v1-S1.** `TableCellPosition` carries `(row, column, rowspan, colspan,
table_id)` with zero-based indices and span 1 meaning *not merged*, and `CellSlot` enumerates
every slot a merged cell occupies — addressing cells by array index with an implied span of 1 is
the shipped Ethos ODL-adapter defect, and its real cost is that no cross-check can run afterwards.

`SlotCover` is the **structural** half of the locator cross-check and is derived from indices and
spans alone; `GeometricFault` is the geometric half and is derived from boxes alone. A test asserts
no geometry reaches `SlotCover`, because a check whose two halves shared an input would agree with
itself. `LocatorCheck` rides on the artifact rather than in `--diagnostics`: a mismatch changes
whether a cell is trustworthy, which is a statement the artifact makes, not an observation about
the run.

**Two node kinds arrived at v1-S4, and the rule they broke was deliberate.** v1-S1 refused
`TableCell` because a cell's text is already a run; v1-S3 refused `Paragraph` because the role path
already says `P`. The standing rule from both — *do not add a kind for a fact an existing node
already carries* — is exactly why `FormField` and `Annotation` qualify: a field's `/V` and an
annotation's `/Contents` are in **dictionaries**, drawn by nothing, and without a node of their own
they are simply absent from the record.

`NodeAttributes` became a union at the same time, because a text run's character codes and a form
field's value do not belong on one struct. Its tag duplicates `Node::kind`, which is a *checked*
redundancy: `NodeAttributes::kind()` returns the kind, and a contract test asserts every node
agrees with its own attributes.

`NativeLocator` gained `PdfObject`. An annotation has no baseline, no advance and no character
origin, so `PdfLocator` is the wrong shape — and filling it with a plausible origin would put a
coordinate on the wire the document does not contain. Its `AnnotationRect` is deliberately not
`GeometryPresence`: that type means *measured ink*, and a `/Rect` is a number the author wrote.

**The tagged-versus-geometric check is v1-S3, and it is a *second* check rather than a wider
first one.** `LOCATOR_CHECK_V1` compares a table's own indices against its own boxes;
`TAGGED_GRID_CHECK_V1` compares the grid the document's **structure tree** declares against the
grid a detector found. Overloading one id with both would leave a reader unable to tell which pair
of derivations disagreed. `TaggedGridCheck` rides on `TableRecord` as an **absent key** where the
tree describes no table on that page — absent is not the same as agreement.

`StructuralLocator` gained two variants at v1-S3. `PdfTagged` carries the role path the document's
own tree gives a node, with `standard_role_path` present only when `/RoleMap` actually remapped
something; `PdfArtifact` marks content the page called furniture rather than body text. Those runs
are **kept in `nodes`** — a reader that deletes running heads has silently edited the document
(parity checklist O21/O22). The pre-existing `PdfMcid` now means something narrower and more
useful: an id the content stream supplied that **no structure element claims**.

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
| Stages | `classify`, `extract`, `to_representation`, `build_overlay` |
| Stage artifacts | `Classification`, `PageClassification`, `SourceRef`, `ExtractArtifact`, `PageExtract`, `TextRun`, `SynthesizedChar`, `SynthesisReason`, `PdfLocator`, `ImageRecord` |
| Reason vocabulary | `OcrNeedReason`, `LayoutComplexityReason` |
| Format detection | `check_pdf_magic` · `aims_at_the_pdf_reader` |
| Modules | `exit` (`SIMPLE`, `NEEDS_ATTENTION`, `COULD_NOT_READ`, `exit_code`) · `limitations` (limitation-code constants and builders) |
| Constants | `CLASSIFICATION_ARTIFACT_TYPE`, `CLASSIFICATION_SCHEMA_VERSION`, `EXTRACT_ARTIFACT_TYPE`, `EXTRACT_SCHEMA_VERSION`, `OVERLAY_ARTIFACT_TYPE`, `PROCESSOR_NAME`, `CRATE_NAME` |

**Internal, do not use:** `ops`, `content`, `cmap`, `encoding`, `fonts`, `metrics`, `text_state`,
`thresholds`, `nodes`, `magic`, `classify`, `document`, `extract`, `represent`, `reasons`,
`images`, `overlay`, `reading_order` as
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
| Constants | `GROUNDING_ARTIFACT_TYPE`, `SOURCE_MEDIA_TYPE`, `GROUNDING_SCHEMA_VERSION`, `GEOMETRY_ABSENT_OMITTED`, `VALIDATION_ARTIFACT_TYPE`, `VALIDATION_SCHEMA_VERSION`, `CRATE_NAME` |

**Internal, do not use:** `grounded_box` as a module. It is private already, and that privacy is
load-bearing rather than tidy — `project()` lives outside it and therefore cannot construct a
`GroundedBox` from anything but a measurement state. See the crate docs.

## `engine-office` — the OOXML readers, the three OpenDocument readers, RTF and EPUB

**New to this document at v2-S3, and the omission is worth recording.** The crate shipped at
v2-S2 outside the freeze table, which by this document's own rule at the top made
`engine_office::read` — the entry point for every non-PDF format `engine extract` accepts —
"internal", renameable without a note. A second format is what surfaced it: the same gap would
have let `is_xlsx` arrive unguarded.

| Kind | Supported items |
| --- | --- |
| Entry point | `read` — dispatches on the package's own evidence, and refuses one claiming to be more than one format |
| Detection | `is_docx`, `is_xlsx`, `is_pptx`, `is_odt`, `is_ods`, `is_odp`, `is_rtf`, `is_epub` — **content, never extension** (Anydoc's **A4**) · `is_opendocument` — the ODF **family**, implemented or not |
| Media types | `DOCX_MEDIA_TYPE`, `XLSX_MEDIA_TYPE`, `PPTX_MEDIA_TYPE`, `ODT_MEDIA_TYPE`, `ODS_MEDIA_TYPE`, `ODP_MEDIA_TYPE`, `RTF_MEDIA_TYPE`, `EPUB_MEDIA_TYPE` |
| Readers | `docx` (runs of `word/document.xml`), `xlsx` (sheets, shared strings, cells), `pptx` (slides, shapes, runs), `odt` (the manifest, and `content.xml`'s paragraphs), `ods` (`content.xml`'s tables, rows and cells), `odp` (`content.xml`'s draw pages, shapes and blocks), `rtf` (a brace-group control-word stream's paragraphs), `epub` (the OCF container chain, a package document's spine, and its XHTML blocks) |
| Container | `zip` — `looks_like_zip`, `entry_names`, `read_entry`, `first_entry` over the existing `flate2` |
| Marker | `CRATE_NAME` |

**`is_odt` asks a different question from the other three, and that is the format's doing.** An
OOXML package is identified by which main part its central directory lists; an ODF package
**declares its own type**, in a `mimetype` entry the package specification requires to be first and
stored. `zip::first_entry` is public at v2-S5 for exactly that: "first, and uncompressed" is part
of what an ODF package *is*, so a reader needs to be able to ask.

**`is_opendocument` is a third kind of question, added at v2-S6.** `is_odt` and `is_ods` each ask
*"is this a kind we read"*; this one asks *"is the office reader the one to ask"*, which only an ODF
package can answer about itself. The CLI dispatches on it, and the reason is a measured defect: with
only the narrow predicates, an `.odp` answered `false` to every one of them and fell through to the
PDF reader, which refused it for having no `%PDF-` header. Fail-closed, and naming the wrong cause.
`read` then refuses a declared ODF type it does not implement, by name.

**`is_epub` asks the OCF question against a different declared type** (v2-S9), and that is the
whole reason `is_opendocument` was written the way it was: an `.epub` writes `application/epub+zip`
into the same first, stored `mimetype` entry an ODF package uses, so answering the family question
on that entry's *presence* would tell an EPUB it is OpenDocument. It does not, and the test that
says so has been in this suite since v2-S6.

**`is_rtf` asks no container question at all** (v2-S8). Every predicate above it opens a package;
an `.rtf` is a byte stream that begins `{\rtf`, and that is the whole of detection. A bare `{` is
not enough, and an OLE compound file — a legacy `.doc`, beginning `D0 CF 11 E0` — is a different
format with a different reader.

**`is_odp` is exact rather than prefixed, and that matters more than it did for `is_ods`** (v2-S7).
An `.ots` template would have been a nuisance; an `.odg` drawing is a hazard, because its
`content.xml` really *is* the `<draw:page>` vocabulary this reader knows. A prefix match would have
produced a **plausible artifact for a format nobody decided to support** — which is worse than the
`%PDF-` message v2-S6 fixed, since it does not look like a failure at all.

**Internal, do not use:** `xml`, which holds the entity rule, the local-name helper, the
namespace-resolved attribute matcher and the XML-reader plumbing the **seven** XML-backed readers
share — the three ODF readers and `epub` included, which between them reach every item in it except
the one `attribute_value` calls internally. **Seven and not eight:** `rtf` is the one reader that
reaches none of it, because a `{\rtf` byte stream carries no XML for an XML reader to parse.
And `opc`, which holds the one rule that turns an
`r:id` into a part name. Private on purpose: it is one rule about *what counts as text*, and two callers is
already the reason it exists — a third caller reaching it from outside the crate would make it a
contract before anyone decided it should be one.

## `engine-cli` — the binary

**Exports nothing.** `engine` is a `[[bin]]`; there is no library target and no supported
`engine_cli::` path. The CLI's **nine** subcommands, their flags and their exit codes are a
*product* contract, not a Rust one: `classify`, `extract`, `ground` and `grounding-check` from v0,
then `verify` (v0.1), `overlay` (v1-S6), `markdown` (v1.1-S1), `html` (v1.1-S4) and `mcp` (v1.2-S1).
`docs/04-ARCHITECTURE.md` §2 describes the v0 four and says so; `enum Command` in
`crates/engine-cli/src/main.rs` is the list that cannot go stale.

This said *"four subcommands"* and pointed at §2, which also said four — two documents agreeing
with each other and neither agreeing with the binary. Repaired at v2-S13.3.

---

## The thin-shell mapping

`docs/03-V0-SCOPE.md` §1 item 19 and `docs/04-ARCHITECTURE.md` §2 both require that the CLI be a
thin shell — that **every subcommand behaviour be reachable through the library**. This table is
that mapping, and the right-hand column is what a caller writes instead of spawning a process.

**It maps four of the nine, and that gap is the finding rather than the table's shape.** The rule
binds every subcommand; the rows here cover only v0's four, so `verify`, `overlay`, `markdown`,
`html` and `mcp` have their thin-shell mapping stated nowhere. Each *is* thin — `engine verify`
relays through `engine_core::verifier`, `markdown` and `html` through `engine_core`'s projections,
`overlay` through `engine_pdf`, and `mcp` is the shell around all of them — but "is" and "is
written down where a caller can check it" are different claims, and this document exists to make
the second one. Filling in the five rows needs the library-only and CLI-equals-library proofs named
per row, the way the four below name theirs, and that is a slice rather than a sentence.

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
