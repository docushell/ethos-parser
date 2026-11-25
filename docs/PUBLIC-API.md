# PUBLIC-API — the frozen v0 surface

**Status:** frozen at M7 (v0.1.0) · **Enforced by:** `crates/ethos-parser-cli/tests/public_api.rs`
**Rule:** if this document and the crate roots disagree, that test fails. Neither one is allowed to
move alone.

---

## What "frozen" means here

Every item below is **intended to be supported**: it is what a caller may depend on, and changing
it is a deliberate, changelogged act. Everything not listed is internal — not "undocumented but
usable", not "fine if you need it". Internal items may be renamed, narrowed, or deleted in any
release without a note.

The freeze is **narrowing only**. M7 removed items from the surface; it added `ethos-parser-core`'s
`diagnostics` module and nothing else. A later milestone may add, but v0.x will not silently take
away.

Three things are worth saying plainly, because each is a promise that costs something to keep:

1. **`ethos-parser-cli` exports nothing.** It is a binary. There is no `ethos_parser_cli::` anything, and the
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

## `ethos-parser-core` — the contract, in types

The largest surface, deliberately: this crate *is* `docs/01-CONTRACT.md`, so every artifact-shaped
type a caller reads or writes belongs here. All modules are public; the table names the modules and
the items re-exported at the crate root.

| Module | Supported items |
| --- | --- |
| `assist` | `attach`, `AssistRecord`, `AssistModel`, `AssistPage`, `AssistOutcome`, `AssistBlock`, `ASSIST_ANSWERS_VERSION`, `ASSIST_RULE_V1`, `ASSIST_RENDERER`, `ASSIST_REQUEST_V1`, the budget `ASSIST_ANSWERS_MAX_BYTES` and `ASSIST_MAX_BLOCKS_PER_PAGE`, and the ten refusal names `ASSIST_ANSWERS_UNREADABLE`, `ASSIST_ANSWERS_OTHER_SOURCE`, `ASSIST_ANSWERS_OTHER_RENDERER`, `ASSIST_ANSWERS_OTHER_REQUEST`, `ASSIST_ANSWERS_OTHER_PAGE`, `ASSIST_ANSWERS_OTHER_MODEL`, `ASSIST_ANSWERS_MODEL_IS_PATH`, `ASSIST_ANSWERS_MODEL_IS_PROCESSOR`, `ASSIST_ANSWERS_OVER_BUDGET`, `ASSIST_BASE`. A drafting model's replies file, checked against the record and read into `Proposed` blocks carried on `RepresentationPayload::assist` rather than as `Node`s: model text has no native locator, so it is uncitable by construction. `attach` reseals the record under `Profile::with_assist`. `AssistGuard`, `AssistFlag`, `AssistMark` (`Contradicted`, `Unsupported`, `Estimate`, `Disputed`) and `ASSIST_GUARD_V1`: what the engine's runs inside each block's box say of its lines, carried on `AssistBlock::guard`; `AssistPin`'s `guard` names that rule. `AssistOutcome`'s `Truncated` and `Loop`: a reply the server cut at its token cap, whose cut last block is dropped, and one whose cut last block ends in a loop, dropped too. `assist_trigger`: the pages of a sealed record a model is asked to read by default, where a scan, under 20 words, a figure, a table, a picture over a quarter of the visible page or U+FFFD is on the page. The whole module is added in 0.66.0 |
| `c14n` | `c14n_bytes`, `canonical_bytes_of`, `sha256_hex`, `sha256_hex_bytes`, `C14nError` |
| `geom` | `quantize`, `QRect`, `QRectError`, `QuantizeError`, `MAX_SAFE_INT`, `QUANTUM_PER_POINT` |
| `identity` | `ArtifactIdentity`, `ArtifactBinding`, `Sha256Hex`, `CoordinateSystem`, `CoordinateOrigin`, `CoordinateUnit`. `ArtifactIdentity::deserialize_pinned` is how an artifact's own `Deserialize` refuses a type or version it does not read (§8) |
| `html` | `to_html`, `to_html_with` (added in 0.66.0: `to_html` with `ProjectionOptions`), `HtmlArtifact`, `HTML_ARTIFACT_TYPE`, `HTML_SCHEMA_VERSION`, `HTML_RULE_BLOCKS_V14` |
| `markdown` | `to_markdown`, `to_markdown_with` and `ProjectionOptions` (added in 0.66.0: `to_markdown` and `to_html` with what the caller asks to leave out or write differently, each counted; every option is off by default, and with all off the output is the plain call's, byte for byte. `drop_furniture` leaves out repeated running heads and folios, `drop_flagged` text the reader flagged as not seen, `infer_lists` writes list items read from markers on a document that declares no structure, `keep_repeated_header` keeps as a row a table's header row the next page repeats, which the joined table leaves out by default, and `unstyled` writes no style marks), `AnchorMap`, `Segment`, `SegmentKind`, `Coverage`, `DroppedBucket`, `StructuralErasure`, `MarkdownArtifact`, `MARKDOWN_ARTIFACT_TYPE`, `MARKDOWN_SCHEMA_VERSION`, `MARKDOWN_RULE_BLOCKS_V14`, and the census codes for text the projection joined or could not represent: `GFM_SPAN_SLOTS_UNREPRESENTABLE`, `GFM_ROW_ZERO_SEPARATOR`, `GFM_CELL_RUN_CLAIMED_TWICE`, `GFM_CELL_NOT_PLACED`, `GFM_TABLE_NOT_PROJECTED`, `GFM_LIST_ITEM_RUN_JOINS`, `MCID_RUN_JOINS`, `BASELINE_RUN_JOINS_ABUTTED`, `BASELINE_RUN_JOINS_SPACED`, `LAYOUT_UNIT_LINE_JOINS`, `HEADING_NUMBER_JOINS`, `LAYOUT_UNIT_GAP_JOINS`, `BLOCK_ELEMENT_RUN_JOINS`, `HEADING_LEVEL_UNRESOLVED`, `HEADING_LEVEL_UNREPRESENTABLE` |
| `locate` | `locate`, `Locations`, `Occurrence`, `OccurrencePart`, `OccurrencesWithheld`, `Searched`, `LOCATIONS_ARTIFACT_TYPE`, `LOCATIONS_SCHEMA_VERSION`, `LOCATE_RULE_V1`, `LOCATE_MAX_QUOTE_BYTES`, `LOCATE_MAX_OCCURRENCES`. Where a string lies in a representation, and nothing else: no verdict, no score, and a string occurring nowhere is an empty answer rather than an error. The two ceilings are `ethos.grounding.v1`'s own, re-declared because this crate may not depend on the grounding crate and asserted equal by a test that can see both |
| `elements` | `to_elements`, `Elements`, `Element`, `ElementLabel`, `ElementPage`, `ElementTable` (its `detection_rule` names the rule that read the table; `headers_unbound` marks a crafted grid whose header lists would pass the view's budget), `ElementCell` (with its cell record's `id`, its `header` flag, and on a data cell the header cells over it, `column_headers` and `row_headers`, by id; added in 0.66.0), `ElementFormField` (a form field's name, label, type, value and state, under `ElementLabel::FormField`), `Omitted`, `OmissionReason`, `ELEMENTS_ARTIFACT_TYPE`, `ELEMENTS_SCHEMA_VERSION`, `ELEMENTS_RULE_V1`, all added in 0.66.0. A document's items as one flat list, each with its page, box, text, the node ids it covers and a label the record states (`text`, `section-header`, `page-header`, `page-footer`, `table`, `picture`, `form-field`, `caption`, `list-item`), read from the record with no layout rule of its own. The rule id is on the artifact, not the profile, so this view moves no other artifact's `profile_sha256` |
| `profile` | `Profile`, `profile_sha256`, `BackendIdentity`, `Capabilities`, `PageBudget`, `XrefRepair`, `VerifierPin`, `RecognizerPin`, `RecognizerIdentity` (added in 0.66.0: which recognizer an OCR profile's runs come from, `Profile::ocr_v0` pinning it), `AssistPin` (added in 0.66.0: which drafting model an assisted record's blocks come from, `Profile::with_assist` pinning it; absent from every other profile), `TableDetection` (its optional `office`, added in 0.66.0, names the table rule an office format ran), `RasterDpi`, `CMAP_DATA_VERSION` (the character-decoding data the profile carries), `NOT_RUN` (what a rule field holds where that rule does not run for the profile's format). **Rule ids**, each naming one version of one rule: a changed rule takes a new id and an old id keeps its meaning, so a record always says which rule read it. PDF reading order: `READING_ORDER_RULE_V0`, `READING_ORDER_RULE_V1`, `READING_ORDER_RULE_V2`, `READING_ORDER_RULE_V3`, `READING_ORDER_RULE_V4`, `READING_ORDER_RULE_V5` (the last added in 0.66.0). What a page's content is observed to hold, images and text findings: `OBSERVATION_RULE_V3`, `OBSERVATION_RULE_V4`, `OBSERVATION_RULE_V5` (added in 0.66.0: three new findings, and `invisible-render-mode` narrowed to mode-7 text with nothing painted through its clip). A font's codes read as text: `TEXT_CODE_RULE_V2`, `TEXT_CODE_RULE_V3`, `TEXT_CODE_RULE_V4`, `TEXT_CODE_RULE_V5` (added in 0.66.0: a TrueType glyph its program names `Euro` reads as `€`, not the `¤` its `/ToUnicode` declares). Tables from painted rectangles: `TABLE_DETECTION_V1`, `TABLE_DETECTION_V2`, `TABLE_DETECTION_V3`, `TABLE_DETECTION_V4`, `TABLE_DETECTION_V5`, `TABLE_DETECTION_V6`, `TABLE_DETECTION_V7`, `TABLE_DETECTION_V8`, `TABLE_DETECTION_V9`, `TABLE_DETECTION_V10`, `TABLE_DETECTION_V11`, `TABLE_DETECTION_V12`, `TABLE_DETECTION_V13`, `TABLE_DETECTION_V14` (the last added in 0.66.0); from text alignment: `TABLE_DETECTION_UNRULED_V1`; from stroked ruling lines: `TABLE_DETECTION_STROKE_V1`, `TABLE_DETECTION_STROKE_V2`; from the tables a structure tree declares: `TABLE_DETECTION_TAGGED_V1`, `TABLE_DETECTION_TAGGED_V2` (added in 0.66.0: a tagged `/TH` cell is a header cell); from the whitespace a table leaves across its rows: `TABLE_DETECTION_TRACKS_V3`, `TABLE_DETECTION_TRACKS_V4`, `TABLE_DETECTION_TRACKS_V5`, `TABLE_DETECTION_TRACKS_V6`, `TABLE_DETECTION_TRACKS_V7`, `TABLE_DETECTION_TRACKS_V8`, `TABLE_DETECTION_TRACKS_V9`, `TABLE_DETECTION_TRACKS_V10`, `TABLE_DETECTION_TRACKS_V11`, `TABLE_DETECTION_TRACKS_V12`, `TABLE_DETECTION_TRACKS_V13`, `TABLE_DETECTION_TRACKS_V14`, `TABLE_DETECTION_TRACKS_V15`, `TABLE_DETECTION_TRACKS_V16` (the last added in 0.66.0); a bar chart's printed labels read back as its table: `TABLE_DETECTION_CHARTS_V1`, `TABLE_DETECTION_CHARTS_V2`, `TABLE_DETECTION_CHARTS_V3`; a line chart's: `TABLE_DETECTION_LINES_V1`; a pie's: `TABLE_DETECTION_PIES_V1`. The structure tree read and bound to runs: `STRUCT_TREE_RULE_V1`, `STRUCT_TREE_RULE_V2`, `STRUCT_TREE_RULE_V3`. Headings inferred from type: `HEADING_INFERENCE_RULE_V4`, `HEADING_INFERENCE_RULE_V5`, `HEADING_INFERENCE_RULE_V6`, `HEADING_INFERENCE_RULE_V7`, `HEADING_INFERENCE_RULE_V8` (the last added in 0.66.0). Lines joined into layout units by their own spacing: `LAYOUT_UNIT_RULE_V1`, `LAYOUT_UNIT_RULE_V2`, `LAYOUT_UNIT_RULE_V3`, `LAYOUT_UNIT_RULE_V4`, `LAYOUT_UNIT_RULE_V5`, `LAYOUT_UNIT_RULE_V6`, `LAYOUT_UNIT_RULE_V7` (the last added in 0.66.0). Running heads and folios: `FURNITURE_RULE_V1`. Figure regions from the paths a page paints: `FIGURE_RULE_V1`. A form's empty boxes from the squares a page outlines: `DRAWN_BOX_RULE_V1` and `DRAWN_BOX_CANDIDATES_MAX` (both added in 0.66.0, decision #168: the most squares a page may outline for the rule to read any). Right-to-left lines read in logical order: `RIGHT_TO_LEFT_RULE_V1`. Form fields and annotations: `FORM_ANNOTATION_RULE_V2`, `FORM_ANNOTATION_RULE_V3` (added in 0.66.0: a check box's or radio button's state and a field's label, as the widget states them). The other formats' reading order and text rules: `DOCX_READING_ORDER_RULE_V3` and `DOCX_TEXT_CODE_RULE_V4` (both added in 0.66.0), `XLSX_READING_ORDER_RULE_V1`, `XLSX_TEXT_CODE_RULE_V3`, `XLSX_TEXT_CODE_RULE_V4` (added in 0.66.0: a number cell shows the value its number format displays), `PPTX_READING_ORDER_RULE_V1`, `PPTX_TEXT_CODE_RULE_V3`, `ODT_READING_ORDER_RULE_V1`, `ODT_TEXT_CODE_RULE_V2`, `ODS_READING_ORDER_RULE_V1`, `ODS_TEXT_CODE_RULE_V2`, `ODP_READING_ORDER_RULE_V1`, `ODP_TEXT_CODE_RULE_V2`, `RTF_READING_ORDER_RULE_V1`, `RTF_TEXT_CODE_RULE_V2`, `EPUB_READING_ORDER_RULE_V1`, `EPUB_TEXT_CODE_RULE_V2`. Office tables, all added in 0.66.0: `TABLE_DETECTION_XLSX_SHEETS_V1` (a worksheet's tables), `TABLE_DETECTION_PPTX_TBL_V1` (a slide's tables), `TABLE_DETECTION_DOCX_V1` (a DOCX's `<w:tbl>` tables, the DOCX profile's `TableDetection::office`), and `OFFICE_TABLE_MAX_SLOTS` (the most cell slots one office document's tables may span between them) |
| `proposed` | `ProposedCensus`, `ProposedPage` and the codes `REPLACED_BY_PROPOSED_PAGE`, `PROPOSED_TABLE_DUPLICATE`, `PROPOSED_VALUE_CONTRADICTED`, `PROPOSED_LINE_UNSUPPORTED`, `PROPOSED_BLOCK_TRUNCATED`, `PROPOSED_TABLE_UNPARSED`, `PROPOSED_BLOCK_NOT_PROJECTED`, added in 0.66.0: where a record holds an `assist` key, `to_markdown_with` and `to_html_with` write the model's page in place of a page the engine read under 30% of its words, add its tables and charts at their reading position elsewhere, each labelled `Proposed` with the model's name and closed by an end line, the model's blocks one quote deeper and escaped so none of it leaves that quote nor stands for those two lines, and say what they wrote and left out in `MarkdownArtifact::proposed` and `HtmlArtifact::proposed`; the model's bytes are `SegmentKind::Proposed` segments, which `is_invertible` never accepts. Breaking for a caller matching `SegmentKind` exhaustively, or building either artifact by its fields |
| `derivation` | `DerivationClass`, `GeometryPresence`, `GeometryAbsence` |
| `assurance` | `Assurance`, `Limitation`, `LimitationScope`, `PageState`, `PageStateEntry`, `CoverageSummary`, `ProcessingGaps`, `ProcessingTerminalState`, `RefusalCode`, `PageBindingResult`, `page_binding_status`, `codes` |
| `representation` | `DocumentRepresentation`, `RepresentationPayload`, `Node`, `NodeKind`, `NodeGeometry`, `PageRecord` (`turn` and `to_declared`, added in 0.66.0: the turn a page is read in beyond its rotation, and a box on it mapped back to the declared frame; `drawn_boxes`, added in 0.66.0, the empty boxes a page draws, which a struct literal must name), `DrawnBox` and `BoxSide` (added in 0.66.0, decision #168: an empty box a page draws, beside the run of its line it is drawn by, and on which side), `NativeLocator`, `PdfLocator`, `RecognizedLocator` (added in 0.66.0: a recognized run's place on its page and its pixels in the image read; `NativeLocator::run_locator` reads a run's place whether drawn or recognized), `DocxLocator`, `XlsxLocator`, `PptxLocator`, `OdtLocator`, `OdsLocator`, `OdpLocator`, `RtfLocator`, `EpubLocator`, `PdfObjectLocator`, `PdfObjectNumber`, `StructuralLocator`, `PdfTaggedLocator`, `PdfArtifactLocator`, `AnnotationRect`, `NodeAttributes`, `OfficeRunAttributes` (`heading_level`, `list_level` and `furniture` added in 0.66.0; it implements `Default`), `OfficeCellAttributes`, `OfficeSlideRunAttributes` (`line_break_before`, added in 0.66.0: a slide cell's run after an `<a:br>`), `OfficeParagraphAttributes`, `OdfBlockKind`, `OfficeOdfCellAttributes`, `OfficeOdfShapeAttributes`, `RtfParagraphAttributes`, `RtfParagraphBreak`, `EpubBlockAttributes`, `OdfValueType`, `OdfCellTextSource`, `CellValueType`, `CellTextSource`, `FormFieldAttributes` (`label` and `state` added in 0.66.0; code that builds one with a struct literal must name them), `CheckState` and `CheckStateAbsence` (added in 0.66.0: a check box's or radio button's state as its widget states it, or why it cannot be told), `AnnotationAttributes`, `FieldValue`, `SourceIdentity`, `ProcessingRun`, `ProcessorIdentity`, `SynthesizedAt`, `TextRunAttributes` (`read_beside`, added in 0.66.0, is the run a form filler's value was read beside; a struct literal must name it), `Script` (a run set above or below its line: superscript or subscript), `Furniture` (the edge of its page a run's line stands apart at: header or footer), `Reading` (a run's text as read, beside the order the page drew; `Node::reading_text` returns it where a run carries one and the text elsewhere), `TextFinding` (`TinyText`, `HiddenLayer` and `FillMatchesBackground` added in 0.66.0), `PdfImageLocator`, `PaintedRect`, `ImageAttributes`, `ImageMediaType`, `REPRESENTATION_ARTIFACT_TYPE`, `REPRESENTATION_SCHEMA_VERSION` |
| `ids` | `NodeId`, `IdAllocator`, `IdKind`, `sort_ids` |
| `error` | `EngineError` — the six-variant taxonomy |
| `diagnostics` | `Diagnostics`, `DiagnosticsRun`, `HostInfo`, `Stage`, `DIAGNOSTICS_VERSION` |
| `verifier` | `VerifierBinary`, `relay`, `RelayRequest`, `Relayed`, `GROUNDING_ADAPTER`, `RELAY_OK`, `RELAY_REFUSED`, `RELAY_UNAVAILABLE` |
| `figures` | `FigureRecord` (`34-FIGURE-REGIONS-SCOPE.md`). One region the figure rule infers from the paths a page paints, carried on `RepresentationPayload::figures` rather than as a `Node`: a region claims no text, and the runs inside it are nodes already. A box and the rule that drew it, `Computed` |
| `outlines` | `OutlineRecord` (`29-OUTLINES-SCOPE.md`). One entry of the hierarchy a PDF catalog declares, carried on `RepresentationPayload::outlines` rather than as a `Node`: a bookmark title is text no content stream painted, so it has no native locator and North Star #4 requires one on every node. The consequence is deliberate — **an outline title is not quotable**, because `locate` and grounding both read `nodes` |
| `tables` | `CellSlot`, `TableCellPosition`, `SlotCover`, `SlotFault`, `TableRecord`, `TableCellRecord` (its `header` flag, added in 0.66.0: a tagged `/TH`, a DOCX table's leading `<w:tblHeader/>` rows, or a row a geometric rule read as its table's header; a struct literal must name it), `LocatorCheck`, `CheckStatus`, `GeometricFault`, `LOCATOR_CHECK_V1`: a table's cells and the slots they cover, and the check of its indices against its boxes · `TaggedGridCheck`, `TaggedGridStatus`, `TaggedGridFault`, `TAGGED_GRID_CHECK_V1`: the check of a detected grid against the grid the structure tree declares |

Plus `CRATE_NAME`, which exists so a link test can assert the workspace builds.

**Internal, do not use:** nothing as a module. Every module in this crate is contract; three
`#[doc(hidden)]` functions inside them are public for the PDF reader alone and no supported API:
`markdown::opens_with_marker` and `markdown::lone_marker` (its layout units' list markers) and
`assist::is_digit` (its section numbers' digits, decision #174).

**The `tables` module.** `TableCellPosition` carries `(row, column, rowspan, colspan,
table_id)` with zero-based indices and span 1 meaning *not merged*, and `CellSlot` enumerates
every slot a merged cell occupies — addressing cells by array index with an implied span of 1 is
the defect the Ethos verifier's opendataloader adapter shipped with, and its real cost is that no
cross-check can run afterwards.

`SlotCover` is the **structural** half of the locator cross-check and is derived from indices and
spans alone; `GeometricFault` is the geometric half and is derived from boxes alone. A test asserts
no geometry reaches `SlotCover`, because a check whose two halves shared an input would agree with
itself. `LocatorCheck` rides on the artifact rather than in `--diagnostics`: a mismatch changes
whether a cell is trustworthy, which is a statement the artifact makes, not an observation about
the run.

**`FormField` and `Annotation` are node kinds; `TableCell` and `Paragraph` are not.** The rule is
*do not add a kind for a fact an existing node already carries*: a cell's text is already a run,
and a paragraph's role path already says `P`. A field's `/V` and an annotation's `/Contents` are in
**dictionaries**, drawn by nothing, and without a node of their own they would be absent from the
record.

`NodeAttributes` is a union, because a text run's character codes and a form field's value do not
belong on one struct. Its tag duplicates `Node::kind`, which is a *checked*
redundancy: `NodeAttributes::kind()` returns the kind, and a contract test asserts every node
agrees with its own attributes.

`NativeLocator` has a `PdfObject` variant for these. An annotation has no baseline, no advance and no character
origin, so `PdfLocator` is the wrong shape — and filling it with a plausible origin would put a
coordinate on the wire the document does not contain. Its `AnnotationRect` is deliberately not
`GeometryPresence`: that type means *a box this engine measured* — for a text run, the pen's
advance over the font's envelope, not glyph ink (contract §5.3) — and a `/Rect` is a number the
author wrote.

**The tagged-versus-geometric check is a *second* check rather than a wider first one.** `LOCATOR_CHECK_V1` compares a table's own indices against its own boxes;
`TAGGED_GRID_CHECK_V1` compares the grid the document's **structure tree** declares against the
grid a detector found. Overloading one id with both would leave a reader unable to tell which pair
of derivations disagreed. `TaggedGridCheck` rides on `TableRecord` as an **absent key** where the
tree describes no table on that page — absent is not the same as agreement.

`StructuralLocator` has two variants for a tagged PDF. `PdfTagged` carries the role path the
document's own tree gives a node, with `standard_role_path` present only when `/RoleMap` actually
remapped something; `PdfArtifact` marks content the page called furniture rather than body text.
Those runs are **kept in `nodes`** — a reader that deletes running heads has silently edited the
document. `PdfMcid` means an id the content stream supplied that **no structure element claims**.

`PdfTaggedLocator` carries `derivation` on every tagged locator, with no default: `extracted`
where the innermost citing element is the author's, `computed` where it carries this engine's own
owner attribute (`/O /EthosParser`, read under `/A` and through `/ClassMap`), so a tag this engine
wrote and read back is never reported as the author's. `STRUCT_TREE_RULE_V2` names the reading; a
representation read under an earlier structure-tree rule and one read under this one each refuse
the other (`docs/23-AUTO-TAGGING-SCOPE.md` §8).

**What the `verifier` module does *not* export is the point.** There is no report
type, no claim, no check, no result — the engine spawns a verifier and forwards its bytes without
reading them, so there is nothing to model. `VerifierBinary::resolve` locates and pins one,
`relay` runs it, and `Relayed` carries the child's stdout, stderr and a mapped exit code. The
three `RELAY_*` constants keep "the verifier refused" (1) apart from "the run did not happen" (2),
for the same reason the classify exit codes never collapse.

**Two more items on `Profile`**, both part of its hash:

- `XrefRepair` — whether the one bounded cross-reference repair runs. It decides *which documents
  produce an artifact at all*, which makes it the strongest output-affecting knob in the set.
- `VerifierPin` — which verifier a run was bound to. `NotPinned` is the default and is a real
  statement rather than a missing field: the engine does not verify, and a run that consulted no
  verifier says so. `ethos-parser verify` pins the binary it spawned by version and digest.

Both are adjacently tagged (`{"mode": …}`), matching `PageBudget`. That is not cosmetic: serde
does not honour `deny_unknown_fields` on an internally tagged enum, so an internally tagged
variant would accept an unknown key, drop it, and re-hash to a digest different from the one it
arrived with.

**One caveat that is not about visibility.** `diagnostics` is public but is *not* an artifact type.
It has no `artifact_type`, no `schema_version`, no `profile_sha256`, and it is not canonicalized.
Do not persist it as a record or feed it to a fingerprint — `docs/01-CONTRACT.md` §4, and
`crates/ethos-parser-cli/tests/diagnostics.rs` asserts the separation.

## `ethos-parser-pdf` — the PDF reader

| Kind | Supported items |
| --- | --- |
| Handle | `Document` — `open`, `open_bytes`, `source_sha256`, `byte_len`, `page_count` |
| Stages | `classify`, `extract`, `extract_recognized` and `extract_image` (added in 0.66.0: what `--ocr` runs, a scanned page read by a recognizer and a PNG or JPEG file read as one page, which `image_media_type` recognizes by its signature; `docs/35-OCR-SCOPE.md`), `to_representation` (and `into_representation`, which takes the extract so its runs move into the record rather than being copied), `build_overlay`, `write_tags` — the last writes a copy of an untagged PDF carrying this engine's own `/Document`/`/Div` structure tree, marked computed on every element, fills absence only, and reads its output back before returning it (`docs/23-AUTO-TAGGING-SCOPE.md` §3) |
| Stage artifacts | `Classification`, `PageClassification`, `SourceRef`, `ExtractArtifact`, `PageExtract`, `TextRun`, `SynthesizedChar`, `SynthesisReason`, `PdfLocator`, `ImageRecord`, `RecognizedSource` (added in 0.66.0) |
| Recognizer protocol | `Recognize` — the trait a recognizer implements (the CLI's over `ethos-parser-ocr serve`, a test's as a function) · `RecognitionRequest`, `ImageSpec`, `PdfImageSpec`, `Colour`, `Base`, `FlateParms`, `Recognition`, `RecognizedLine`, `RecognizedWord` — the protocol's request and answer (added in 0.66.0) |
| Reason vocabulary | `OcrNeedReason`, `LayoutComplexityReason` |
| Format detection | `check_pdf_magic` · `aims_at_the_pdf_reader` |
| Modules | `exit` (`SIMPLE`, `NEEDS_ATTENTION`, `COULD_NOT_READ`, `exit_code`) · `limitations` (limitation-code constants and builders) |
| Constants | `CLASSIFICATION_ARTIFACT_TYPE`, `CLASSIFICATION_SCHEMA_VERSION`, `EXTRACT_ARTIFACT_TYPE`, `EXTRACT_SCHEMA_VERSION`, `OVERLAY_ARTIFACT_TYPE`, `TAGS_ARTIFACT_TYPE` (`ethos.parser.tags.v0`, stamped on the tagged document's catalog; it moves when the written shape moves), `PROCESSOR_NAME`, `CRATE_NAME` |

**Internal, do not use:** `ops`, `content`, `cmap`, `encoding`, `fonts`, `metrics`, `text_state`,
`thresholds`, `nodes`, `magic`, `classify`, `document`, `extract`, `represent`, `reasons`,
`images`, `overlay`, `reading_order`, `tagging` as
*modules*. The items named above are re-exported at the crate root and that is the address to use;
the module paths are not.

`limitations` stays public on purpose: its constants are **wire vocabulary**. A consumer matching
on the `code` field of a limitation it read out of an artifact needs the same strings the engine
wrote, and retyping them by hand is how a consumer silently stops matching.

## `ethos-parser-grounding` — the projection and its validator

| Kind | Supported items |
| --- | --- |
| Projection | `project`, `Projection`, `OmissionReport` (with `is_lossy`), `SpansWithheld`, `ElementsOmitted`, `TablesWithheld`, `to_canonical_bytes` |
| Artifact | `GroundingSource`, `Source`, `Producer`, `GroundingCapabilities`, `GroundingCoordinateSystem`, `Page`, `Element`, `Span`, `Table`, `Cell` |
| Emittable geometry | `GroundedBox`, with `from_presence` and `to_array` — and `from_presence` is its **only** constructor, taking a `GeometryPresence` |
| Validator (`check`) | `grounding_check`, `grounding_check_reading_source`, `ValidationReport`, `Structure`, `SourceBinding`, `Counts`, `ReportError` |
| Constants | `GROUNDING_ARTIFACT_TYPE`, `SOURCE_MEDIA_TYPE`, `GROUNDING_SCHEMA_VERSION`, `GROUNDING_SCHEMA_VERSION_PAGE_LESS`, `PAGE_LESS_MEDIA_TYPES`, `GEOMETRY_ABSENT_OMITTED`, `SPANS_WITHHELD_OVER_LIMIT`, `ELEMENTS_OMITTED_OVER_LIMIT`, `TABLES_WITHHELD_OVER_LIMIT`, `VALIDATION_ARTIFACT_TYPE`, `VALIDATION_SCHEMA_VERSION`, `CRATE_NAME` |

**Internal, do not use:** `grounded_box` as a module. It is private already, and that privacy is
load-bearing rather than tidy — `project()` lives outside it and therefore cannot construct a
`GroundedBox` from anything but a measurement state. See the crate docs.

## `ethos-parser-office` — the OOXML readers, the three OpenDocument readers, RTF and EPUB

`ethos_parser_office::read` is the entry point for every non-PDF format `ethos-parser extract`
accepts. Like every item in this document, it is supported because it is listed here: by the rule
at the top, an item left out is internal, renameable without a note.

| Kind | Supported items |
| --- | --- |
| Entry point | `read` — dispatches on the package's own evidence, and refuses one claiming to be more than one format |
| Detection | `is_docx`, `is_xlsx`, `is_pptx`, `is_odt`, `is_ods`, `is_odp`, `is_rtf`, `is_epub` — **content, never extension** · `is_opendocument` — the ODF **family**, implemented or not |
| Media types | `DOCX_MEDIA_TYPE`, `XLSX_MEDIA_TYPE`, `PPTX_MEDIA_TYPE`, `ODT_MEDIA_TYPE`, `ODS_MEDIA_TYPE`, `ODP_MEDIA_TYPE`, `RTF_MEDIA_TYPE`, `EPUB_MEDIA_TYPE` |
| Readers | `docx` (runs of `word/document.xml`; also, added in 0.66.0, `read_part` for a header, footer or footnotes part, `read_styles` and `Styles` for `word/styles.xml`, `ParagraphProperties` on each `Run`, `Table` and `Cell` with `MainPart`'s `tables`, and `Reference` with `MainPart`'s `references` and `even_and_odd_headers` for `word/settings.xml`), `xlsx` (sheets, shared strings, cells), `pptx` (slides, shapes, runs), `odt` (the manifest, and `content.xml`'s paragraphs), `ods` (`content.xml`'s tables, rows and cells), `odp` (`content.xml`'s draw pages, shapes and blocks), `rtf` (a brace-group control-word stream's paragraphs), `epub` (the OCF container chain, a package document's spine, and its XHTML blocks) |
| Container | `zip` — `looks_like_zip`, `entry_names`, `read_entry`, `first_entry` over the existing `flate2` |
| Marker | `CRATE_NAME` |

**`is_odt` asks a different question from the other three, and that is the format's doing.** An
OOXML package is identified by which main part its central directory lists; an ODF package
**declares its own type**, in a `mimetype` entry the package specification requires to be first and
stored. `zip::first_entry` is public for exactly that: "first, and uncompressed" is part of what
an ODF package *is*, so a reader needs to be able to ask.

**`is_opendocument` is a third kind of question.** `is_odt` and `is_ods` each ask *"is this a kind
we read"*; this one asks *"is the office reader the one to ask"*, which only an ODF package can
answer about itself. The CLI dispatches on it, so an ODF package of a type this crate does not read
reaches the office reader rather than falling through to the PDF reader, which would refuse it for
having no `%PDF-` header: fail-closed, but naming the wrong cause. `read` then refuses a declared
ODF type it does not implement, by name.

**`is_epub` asks the OCF question against a different declared type**, and that is the whole
reason `is_opendocument` was written the way it was: an `.epub` writes `application/epub+zip` into
the same first, stored `mimetype` entry an ODF package uses, so answering the family question on
that entry's *presence* would tell an EPUB it is OpenDocument. It does not, and a test says so.

**`is_rtf` asks no container question at all.** Every predicate above it opens a package;
an `.rtf` is a byte stream that begins `{\rtf`, and that is the whole of detection. A bare `{` is
not enough, and an OLE compound file — a legacy `.doc`, beginning `D0 CF 11 E0` — is a different
format with a different reader.

**`is_odp` is exact rather than prefixed, and that matters more than it does for `is_ods`.** An
`.ots` template would be a nuisance; an `.odg` drawing is a hazard, because its `content.xml`
really *is* the `<draw:page>` vocabulary this reader knows. A prefix match would produce a
**plausible artifact for a format nobody decided to support** — which is worse than a refusal that
names the wrong cause, since it does not look like a failure at all.

**Internal, do not use:** `xml`, which holds the entity rule, the local-name helper, the
namespace-resolved attribute matcher and the XML-reader plumbing the **seven** XML-backed readers
share — the three ODF readers and `epub` included, which between them reach every item in it except
the one `attribute_value` calls internally. **Seven and not eight:** `rtf` is the one reader that
reaches none of it, because a `{\rtf` byte stream carries no XML for an XML reader to parse.
And `opc`, which holds the one rule that turns an
`r:id` into a part name. Private on purpose: it is one rule about *what counts as text*, and two callers is
already the reason it exists — a third caller reaching it from outside the crate would make it a
contract before anyone decided it should be one.

## `ethos-parser-cli` — the binary

**Exports nothing.** `ethos-parser` is a `[[bin]]`; there is no library target and no supported
`ethos_parser_cli::` path. The CLI's **twelve** subcommands, their flags and their exit codes are
a *product* contract, not a Rust one: `classify`, `extract`, `ground`, `grounding-check`,
`verify`, `overlay`, `markdown`, `html`, `mcp`, `tag`, `locate` and `elements`, each described by
`ethos-parser <command> --help`. `docs/04-ARCHITECTURE.md` §2 describes the first four and says
so, and carries `tag`'s row as the exception to its output rule; `enum Command` in
`crates/ethos-parser-cli/src/main.rs` is the list that cannot go stale.

---

## The thin-shell mapping

`docs/history/03-V0-SCOPE.md` §1 item 19 and `docs/04-ARCHITECTURE.md` §2 both require that the CLI be a
thin shell — that **every subcommand behaviour be reachable through the library**. This table is
that mapping, and the right-hand column is what a caller writes instead of spawning a process.

**It maps seven of the twelve, and that gap is the finding rather than the table's shape.** The rule
binds every subcommand; the rows here cover v0's four, `tag`, `locate` and `elements`, so `verify`, `overlay`,
`markdown`, `html` and `mcp` have their thin-shell mapping stated nowhere. Each *is* thin — `ethos-parser verify`
relays through `ethos_parser_core::verifier`, `markdown` and `html` through `ethos_parser_core`'s projections,
`overlay` through `ethos_parser_pdf`, and `mcp` is the shell around all of them — but "is" and "is
written down where a caller can check it" are different claims, and this document exists to make
the second one. Filling in the five rows needs the library-only and CLI-equals-library proofs named
per row, the way the seven below name theirs, and that is a slice rather than a sentence.

| Subcommand | Library call | Library-only proof | CLI-equals-library proof |
| --- | --- | --- | --- |
| `ethos-parser classify <pdf>` | `Document::open_bytes` on the bounded read → `ethos_parser_pdf::classify` → `Classification::to_canonical_bytes` | `library_surface.rs::classify_is_reachable_and_canonical_from_the_library` | `classify_cli.rs::the_cli_output_matches_the_library` |
| `ethos-parser extract <pdf>` | `Document::open` → `ethos_parser_pdf::extract` → `ethos_parser_pdf::to_representation` → `DocumentRepresentation::write_canonical_to`, which streams the bytes `to_canonical_bytes` returns; with `--assist-answers FILE`, the record and FILE's bytes → `ethos_parser_core::attach` first (decision #145), and with `--assist SERVER` before that, the pages `ethos_parser_core::assist_trigger` or `--assist-pages` names → `ethos-parser-assist ask`, which writes FILE (decisions #149 and #150) | `library_surface.rs::extract_and_represent_are_reachable_and_canonical_from_the_library`, `library_surface.rs::write_canonical_to_writes_the_bytes_to_canonical_bytes_returns` | `grounding.rs::the_cli_path_matches_the_library` |
| `ethos-parser ground <repr>` | `serde_json::from_slice::<DocumentRepresentation>` → `verify_fingerprint` → `ethos_parser_grounding::project` → `ethos_parser_grounding::to_canonical_bytes` | `library_surface.rs::project_is_reachable_and_canonical_from_the_library` | `grounding.rs::the_cli_path_matches_the_library` |
| `ethos-parser grounding-check <json> [--source-artifact <pdf>]` | `ethos_parser_grounding::grounding_check`, or `grounding_check_reading_source` with a source → `ValidationReport::to_canonical_bytes` / `exit_code` | `library_surface.rs::grounding_check_is_reachable_and_canonical_from_the_library` | `oracle.rs::oracle_agrees_on_all_ethos_owned_fixtures` |
| `ethos-parser tag <pdf>` | `Document::open_bytes` on the bounded read → `ethos_parser_pdf::write_tags`, whose bytes are the output; exit 0, or 2 on any error | `tagging_write.rs::the_writer_emits_the_readers_fixture_shape` (in `crates/ethos-parser-pdf/tests`) | `tag_cli.rs::the_cli_bytes_equal_the_librarys` |
| `ethos-parser locate <repr> --quote-file <file>` | `serde_json::from_slice::<DocumentRepresentation>` → `verify_fingerprint` → `ethos_parser_core::locate` → `Locations::to_canonical_bytes`; exit 0 answered, 2 refused, **never 1** | `library_surface.rs::locate_is_reachable_and_canonical_from_the_library` | `locate_cli.rs::the_cli_prints_the_bytes_the_library_produces` |
| `ethos-parser elements <repr>` or `elements --source <file>` | `serde_json::from_slice::<DocumentRepresentation>` → `verify_fingerprint` (or the source read as `extract` reads it) → `ethos_parser_core::to_elements` → `Elements::to_canonical_bytes`; exit 0 projected, 2 refused | `elements.rs` unit tests (in `crates/ethos-parser-core/src`) | `elements_cli.rs::the_source_path_the_two_step_path_and_a_repeat_print_the_same_bytes` |

`--diagnostics` is the one flag with no library equivalent to call, because it *is* the shell's
job: `ethos_parser_core::diagnostics::DiagnosticsRun` assembles the observation and the CLI chooses the
stream. A library caller constructs it directly.

The exit-code mapping is library-owned too — `ethos_parser_pdf::exit::exit_code` for `classify` and
`ValidationReport::exit_code` for `grounding-check` — so a caller embedding the engine routes
outcomes the same way the binary does, rather than re-deriving the rule.

---

## Changing this surface

1. Change the crate root.
2. Change this document.
3. `cargo test -p ethos-parser-cli --test public_api` goes green, or one of the two was forgotten.
4. Say why in `CHANGELOG.md`. An export is a promise; adding one is cheap and removing one is not.
