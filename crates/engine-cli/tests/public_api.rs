// Copyright 2026 The ethos-engine maintainers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The public API freeze, as a test (`docs/05-MILESTONES.md` M7).
//!
//! `docs/PUBLIC-API.md` claims to list every supported export. A document that claims that and is
//! not checked becomes wrong on the first PR that adds a `pub use` — quietly, and in the direction
//! that matters, because the new export is supported from the moment someone imports it.
//!
//! So the list is pinned here and cross-checked against both the crate roots and the document.
//! Three ways to fail, and each names a different mistake:
//!
//! | Failure | Meaning |
//! | --- | --- |
//! | An export not in [`FROZEN`] | The surface grew. Intended? Then pin it and changelog it |
//! | A [`FROZEN`] entry not exported | The surface shrank. That is a breaking change |
//! | A [`FROZEN`] entry missing from `PUBLIC-API.md` | The document stopped describing the code |
//!
//! # Why it reads source text
//!
//! Because the alternative is `rustdoc --output-format json`, which is nightly-only, and this
//! workspace pins a stable toolchain everywhere on purpose (`rust-toolchain.toml`). Scanning the
//! crate root for `pub` items is the same technique the contract guards in
//! `engine-core/tests/contract_invariants.rs` already use, and it is exact for the thing being
//! scanned: **a crate root**, where every export is a single declaration and nothing is generated.
//!
//! # What it does not cover, stated rather than implied
//!
//! **Methods defined outside `lib.rs`.** `Document::open_bytes` gaining a sibling would not fail
//! this test; `engine_pdf` gaining a `pub mod` or a `pub use` would. That is the boundary this is
//! built to hold — a *module or item* becoming reachable — because that is the change that
//! happens by accident. A new method on an already-public type is a deliberate act on a type
//! whose docs say it is supported, and `docs/PUBLIC-API.md` describes those by name.
//!
//! Items that happen to live in a crate root — `engine-pdf`'s inline `exit` module, and
//! `engine-grounding`'s `GroundedBox`/`OmissionReport` methods — are covered, and are pinned
//! below. The asymmetry is a property of where the code sits, not a rule.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

/// Every name a crate root exports.
///
/// Handles the four shapes that appear in these files: `pub mod x;`, an inline `pub mod x { … }`,
/// a braced `pub use path::{a, b};` possibly spanning lines, and a bare `pub use path::Item;`.
/// Comment lines are dropped first so prose naming an item is not mistaken for an export.
fn crate_exports(crate_name: &str) -> BTreeSet<String> {
    let path = repo_root().join(format!("crates/{crate_name}/src/lib.rs"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()));

    let code: String = raw
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut names = BTreeSet::new();

    for line in code.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("pub mod ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.insert(name);
            }
        }
        for kw in [
            "pub const ",
            "pub fn ",
            "pub struct ",
            "pub enum ",
            "pub type ",
        ] {
            if let Some(rest) = t.strip_prefix(kw) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    names.insert(name);
                }
            }
        }
    }

    // `pub use` statements, brace-aware and newline-tolerant.
    let mut rest = code.as_str();
    while let Some(at) = rest.find("pub use ") {
        rest = &rest[at + "pub use ".len()..];
        let end = rest.find(';').expect("a `pub use` statement ends in `;`");
        let stmt = &rest[..end];
        rest = &rest[end + 1..];

        if let (Some(open), Some(close)) = (stmt.find('{'), stmt.rfind('}')) {
            for item in stmt[open + 1..close].split(',') {
                let item = item.trim();
                if !item.is_empty() {
                    names.insert(item.to_string());
                }
            }
        } else if let Some(last) = stmt.rsplit("::").next() {
            let last = last.trim();
            if !last.is_empty() {
                names.insert(last.to_string());
            }
        }
    }

    assert!(
        names.len() > 5,
        "the export scan found only {} name(s) in {crate_name}; the scanner is broken, not the \
         crate",
        names.len()
    );
    names
}

// -------------------------------------------------------------------------------------------
// The frozen surface
// -------------------------------------------------------------------------------------------

/// `engine-core` — every module is contract, so every module is public.
///
/// The `tables` block is v1-S1: the table occupancy model (`CellSlot`, `TableCellPosition`,
/// `SlotCover`) and the artifact records that carry it. `SlotCover` is the **structural** half of
/// the locator cross-check and sees no geometry at all — that separation is what makes the check
/// two independent derivations rather than one restated twice.
///
/// The `verifier` block is v0.1: the engine spawns a verifier and relays its bytes, and that has
/// to be reachable as a library call because `engine-cli` exports nothing. Note what is NOT here
/// — no report type, no claim, no check, no result. There is nothing to export because the engine
/// never reads what it forwards.
const CORE: &[&str] = &[
    "ArtifactBinding",
    "ArtifactIdentity",
    "FORM_ANNOTATION_RULE_V1",
    "AnchorMap",
    "AnnotationAttributes",
    "AnnotationRect",
    "Assurance",
    "BackendIdentity",
    "C14nError",
    "CMAP_DATA_VERSION",
    "CRATE_NAME",
    "Capabilities",
    "CellSlot",
    "CheckStatus",
    "CoordinateOrigin",
    "CoordinateSystem",
    "Coverage",
    "CoordinateUnit",
    "CoverageSummary",
    "DIAGNOSTICS_VERSION",
    "DerivationClass",
    "DroppedBucket",
    "Diagnostics",
    "DiagnosticsRun",
    "DocumentRepresentation",
    "EngineError",
    "GROUNDING_ADAPTER",
    "FieldValue",
    "FormFieldAttributes",
    "GeometricFault",
    "GeometryAbsence",
    "GeometryPresence",
    "HostInfo",
    "IdAllocator",
    "IdKind",
    "ImageAttributes",
    "ImageMediaType",
    "LOCATOR_CHECK_V1",
    "Limitation",
    "LimitationScope",
    "LocatorCheck",
    "MAX_SAFE_INT",
    "NativeLocator",
    "Node",
    "NodeAttributes",
    "NodeGeometry",
    "NodeId",
    "NodeKind",
    "PageBindingResult",
    "PageBudget",
    "PageRecord",
    "PageState",
    "PageStateEntry",
    "PaintedRect",
    "PdfArtifactLocator",
    "PdfImageLocator",
    // v2-S2. The second format's address, its facts, and the rule ids a profile carries when a
    // rule does not run for that format.
    "DocxLocator",
    "OfficeRunAttributes",
    "NOT_RUN",
    "DOCX_READING_ORDER_RULE_V1",
    "DOCX_TEXT_CODE_RULE_V1",
    // v2-S3. The third format's address and its facts. A cell is not a run: it has a value type
    // and it may have a formula, and neither is a thing a `<w:r>` has.
    "XlsxLocator",
    "OfficeCellAttributes",
    "CellValueType",
    "CellTextSource",
    "XLSX_READING_ORDER_RULE_V1",
    "XLSX_TEXT_CODE_RULE_V1",
    // v2-S4. The third format's address and its facts. A slide run is neither a cell nor a
    // `<w:r>`: it has a shape, and DrawingML has no `xml:space` to record.
    "PptxLocator",
    "OfficeSlideRunAttributes",
    "PPTX_READING_ORDER_RULE_V1",
    "PPTX_TEXT_CODE_RULE_V1",
    // v2-S5. The fifth format's address and its facts, and the first that is not OOXML. The
    // address is a BLOCK rather than a run, because ODF paragraphs often carry no inline element
    // to address; the fact is which of ODF's two blocks it was, because ODF has no `xml:space` to
    // record and its whitespace mechanism is a count the file states instead.
    "OdtLocator",
    "OfficeParagraphAttributes",
    "OdfBlockKind",
    "ODT_READING_ORDER_RULE_V1",
    "ODT_TEXT_CODE_RULE_V1",
    // v2-S6. The sixth format's address and its facts. The address is the first in this contract
    // that the source file does not write down at all — ODF states a cell's position by where it
    // sits among its siblings, compressed by `table:number-columns-repeated` — and the facts are a
    // second cell type rather than `OfficeCellAttributes` reused, because `CellValueType` is
    // ECMA-376's list and has no `percentage` or `currency` in it.
    "OdsLocator",
    "OfficeOdfCellAttributes",
    "OdfValueType",
    "OdfCellTextSource",
    "ODS_READING_ORDER_RULE_V1",
    "ODS_TEXT_CODE_RULE_V1",
    "PdfLocator",
    "PdfObjectLocator",
    "PdfTaggedLocator",
    "ProcessingGaps",
    "ProcessingRun",
    "ProcessingTerminalState",
    "ProcessorIdentity",
    "Profile",
    "QRect",
    "QRectError",
    "QUANTUM_PER_POINT",
    "QuantizeError",
    "RasterDpi",
    "OBSERVATION_RULE_V1",
    "READING_ORDER_RULE_V0",
    "READING_ORDER_RULE_V1",
    "RELAY_OK",
    "RELAY_REFUSED",
    "RELAY_UNAVAILABLE",
    "REPRESENTATION_ARTIFACT_TYPE",
    "REPRESENTATION_SCHEMA_VERSION",
    "RefusalCode",
    "RelayRequest",
    "Relayed",
    "RepresentationPayload",
    "Sha256Hex",
    "SlotCover",
    "SlotFault",
    "STRUCT_TREE_RULE_V1",
    "SourceIdentity",
    "Stage",
    "StructuralLocator",
    "SynthesizedAt",
    "GFM_CELL_NOT_PLACED",
    "GFM_CELL_RUN_CLAIMED_TWICE",
    "GFM_LIST_ITEM_RUN_JOINS",
    "GFM_ROW_ZERO_SEPARATOR",
    "GFM_SPAN_SLOTS_UNREPRESENTABLE",
    "GFM_TABLE_NOT_PROJECTED",
    "HTML_ARTIFACT_TYPE",
    "HTML_RULE_BLOCKS_V2",
    "HTML_SCHEMA_VERSION",
    "HtmlArtifact",
    "MARKDOWN_ARTIFACT_TYPE",
    "MARKDOWN_RULE_BLOCKS_V2",
    "MARKDOWN_SCHEMA_VERSION",
    "TABLE_DETECTION_STROKE_V1",
    "TABLE_DETECTION_UNRULED_V1",
    "TABLE_DETECTION_V1",
    "TABLE_DETECTION_V2",
    "TEXT_CODE_RULE_V1",
    "TableCellPosition",
    "TableCellRecord",
    "TableDetection",
    "TableRecord",
    "TextFinding",
    "TAGGED_GRID_CHECK_V1",
    "TaggedGridCheck",
    "TaggedGridFault",
    "TaggedGridStatus",
    "TextRunAttributes",
    "VerifierBinary",
    "VerifierPin",
    "XrefRepair",
    "assurance",
    "c14n",
    "c14n_bytes",
    "codes",
    "derivation",
    "diagnostics",
    "error",
    "geom",
    "identity",
    "ids",
    "page_binding_status",
    "profile",
    "profile_sha256",
    "quantize",
    "html",
    "markdown",
    "relay",
    "representation",
    "sha256_hex",
    "sha256_hex_bytes",
    "Segment",
    "SegmentKind",
    "StructuralErasure",
    "MarkdownArtifact",
    "sort_ids",
    "to_html",
    "to_markdown",
    "tables",
    "verifier",
];

/// `engine-pdf` — narrowed at M7. The parsing machinery is `pub(crate)`; only `exit` and
/// `limitations` remain public modules, and `limitations` because its constants are wire
/// vocabulary a consumer matches on.
///
/// `SIMPLE`/`NEEDS_ATTENTION`/`COULD_NOT_READ`/`exit_code` are `exit`'s own members. The scanner
/// sees them because `exit` is an inline `pub mod` in `lib.rs`, and listing them is right: they
/// are as public as the module holding them, and the exit-code mapping is exactly the kind of
/// thing an embedding caller needs so it routes outcomes the way the binary does.
const PDF: &[&str] = &[
    "CLASSIFICATION_ARTIFACT_TYPE",
    "CLASSIFICATION_SCHEMA_VERSION",
    "COULD_NOT_READ",
    "CRATE_NAME",
    "Classification",
    "Document",
    "NEEDS_ATTENTION",
    "SIMPLE",
    "EXTRACT_ARTIFACT_TYPE",
    "EXTRACT_SCHEMA_VERSION",
    "ExtractArtifact",
    "LayoutComplexityReason",
    "OcrNeedReason",
    "OVERLAY_ARTIFACT_TYPE",
    "PROCESSOR_NAME",
    "build_overlay",
    "PageClassification",
    "ImageRecord",
    "PageExtract",
    "PdfLocator",
    "SourceRef",
    "SynthesisReason",
    "SynthesizedChar",
    "TextRun",
    "check_pdf_magic",
    "classify",
    "exit",
    "exit_code",
    "extract",
    "limitations",
    "to_representation",
];

/// `engine-grounding` — the projection, the artifact shape, and the validator.
///
/// `from_presence`, `to_array` and `is_lossy` are methods defined in this crate root rather than
/// in a submodule, so the scanner sees them. They are as public as the types carrying them, and
/// `from_presence` in particular is the one constructor `GroundedBox` has — the item most worth
/// pinning in the whole crate, since its absence is what stops a box being built from a boolean.
const GROUNDING: &[&str] = &[
    "CRATE_NAME",
    "Cell",
    "Counts",
    "Element",
    "GEOMETRY_ABSENT_OMITTED",
    "GROUNDING_ARTIFACT_TYPE",
    // v2-S2. The one media type this artifact can name as its source, so `project` can refuse a
    // page-less one by name rather than on a page lookup that would report the wrong reason.
    "SOURCE_MEDIA_TYPE",
    "GROUNDING_SCHEMA_VERSION",
    "GroundedBox",
    "GroundingCapabilities",
    "GroundingCoordinateSystem",
    "GroundingSource",
    "OmissionReport",
    "Page",
    "Producer",
    "Projection",
    "ReportError",
    "Source",
    "SourceBinding",
    "Span",
    "Structure",
    "Table",
    "VALIDATION_ARTIFACT_TYPE",
    "VALIDATION_SCHEMA_VERSION",
    "ValidationReport",
    "check",
    "from_presence",
    "grounding_check",
    "is_lossy",
    "project",
    "to_array",
    "to_canonical_bytes",
];

/// `engine-office` — the OOXML readers, and OpenDocument text. **New to the freeze at v2-S3.**
///
/// It shipped at v2-S2 outside this table, which meant `engine_office::read` — the entry point
/// for two of the three input formats `engine extract` accepts — was classified "internal" by
/// `PUBLIC-API.md`'s own rule and could have been renamed without a note. A second format is what
/// made that visible: the same omission would have left `is_xlsx` unguarded on arrival.
const OFFICE: &[&str] = &[
    "CRATE_NAME",
    "DOCX_MEDIA_TYPE",
    "XLSX_MEDIA_TYPE",
    "PPTX_MEDIA_TYPE",
    // v2-S5. `is_odt` asks a different question from its three siblings, and that is the format's
    // doing rather than an inconsistency: an OOXML package is identified by which main part it
    // lists, and an ODF package declares its own type in a first, uncompressed `mimetype` entry.
    "ODT_MEDIA_TYPE",
    // v2-S6. `is_opendocument` is a THIRD kind of question, and the CLI is why it exists: a caller
    // dispatching on format needs "is the office reader the one to ask" before "is this a kind the
    // office reader implements", and only an ODF package can answer the first about itself.
    "ODS_MEDIA_TYPE",
    "docx",
    "is_docx",
    "is_odt",
    "is_ods",
    "is_opendocument",
    "is_pptx",
    "is_xlsx",
    "ods",
    "odt",
    "pptx",
    "read",
    "xlsx",
    "zip",
];

const FROZEN: [(&str, &[&str]); 4] = [
    ("engine-core", CORE),
    ("engine-pdf", PDF),
    ("engine-grounding", GROUNDING),
    ("engine-office", OFFICE),
];

// -------------------------------------------------------------------------------------------
// The tests
// -------------------------------------------------------------------------------------------

/// **The exports are exactly the frozen list.**
#[test]
fn the_public_surface_is_the_frozen_one() {
    for (crate_name, frozen) in FROZEN {
        let actual = crate_exports(crate_name);
        let expected: BTreeSet<String> = frozen.iter().map(|s| s.to_string()).collect();

        let added: Vec<&String> = actual.difference(&expected).collect();
        let removed: Vec<&String> = expected.difference(&actual).collect();

        assert!(
            added.is_empty(),
            "`{crate_name}` exports {} item(s) the freeze does not list: {added:?}\n\n\
             An export is a promise. If it is meant to be supported, add it to FROZEN and to \
             docs/PUBLIC-API.md and say so in CHANGELOG.md. If it is machinery, make it \
             `pub(crate)` — that is what M7 did to the parser's insides.",
            added.len()
        );
        assert!(
            removed.is_empty(),
            "`{crate_name}` no longer exports {} frozen item(s): {removed:?}\n\n\
             This is a breaking change for anyone who imported them. Intended removals are \
             allowed and wanted — they just have to be deliberate, which means editing FROZEN, \
             docs/PUBLIC-API.md and CHANGELOG.md in the same commit.",
            removed.len()
        );
    }
}

/// **`docs/PUBLIC-API.md` names every frozen item.**
///
/// The document is the thing a caller reads; the list above is the thing the compiler agrees
/// with. Without this, the two drift and the readable one loses.
#[test]
fn the_public_api_document_names_every_frozen_item() {
    let path = repo_root().join("docs/PUBLIC-API.md");
    let doc = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()));

    let mut missing = Vec::new();
    for (crate_name, frozen) in FROZEN {
        for item in frozen {
            // Backticked, so a word appearing in prose does not count as documentation of an
            // export. `CRATE_NAME` is described once in prose for all three crates rather than
            // repeated in each table.
            if !doc.contains(&format!("`{item}`")) {
                missing.push(format!("{crate_name}::{item}"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "docs/PUBLIC-API.md does not name {} exported item(s):\n  {}\n\n\
         Every supported export is documented, or it is not supported.",
        missing.len(),
        missing.join("\n  ")
    );
}

/// **The internal modules stay internal.**
///
/// Named individually rather than derived, because this is the list M7 narrowed and the point is
/// that a later PR reverting one is visible. `the_public_surface_is_the_frozen_one` would also
/// catch it; this says *why* it matters in the failure message.
#[test]
fn the_parsing_machinery_is_not_public() {
    let exports = crate_exports("engine-pdf");
    for internal in [
        "ops",
        "content",
        "cmap",
        "encoding",
        "fonts",
        "metrics",
        "text_state",
        "thresholds",
        "nodes",
        "magic",
        "document",
        "represent",
        "reasons",
        // v1-S2. The alignment detector's tolerances and `Refusal` vocabulary are the rule's
        // insides: they move whenever `unruled-align-v1` becomes `-v2`, and a caller pinned to
        // them would be pinned to a version of the rule rather than to the contract. What a
        // caller needs — which rule found a table — is on `TableRecord::detection_rule`.
        "unruled",
    ] {
        assert!(
            !exports.contains(internal),
            "`engine_pdf::{internal}` is public again. It carries `f64` fields, borrow-scoped \
             handles or calibration constants — implementation, not contract. Whatever a caller \
             needs from it should be re-exported at the crate root by name."
        );
    }
}

/// **`engine-cli` is a binary and exports nothing.**
///
/// Asserted against the manifest rather than by looking for a `lib.rs`, because the absence of a
/// file is weak evidence: cargo would happily build a library target the day someone adds one.
#[test]
fn the_cli_has_no_library_target() {
    let dir = repo_root().join("crates/engine-cli");
    assert!(
        !dir.join("src/lib.rs").is_file(),
        "engine-cli grew a src/lib.rs. The CLI is a thin shell over the library; a library \
         target here would be a second place for behaviour to live (docs/04-ARCHITECTURE.md §1)."
    );

    let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("engine-cli manifest");
    assert!(
        !manifest.contains("[lib]"),
        "engine-cli's manifest declares a [lib] target"
    );
    assert!(
        manifest.contains("[[bin]]"),
        "engine-cli must declare its binary target explicitly"
    );
}

/// Guard the guard: the scanner must find what is actually there, and miss what is not.
#[test]
fn the_export_scanner_reads_the_shapes_these_files_use() {
    let core = crate_exports("engine-core");
    // `pub mod x;`
    assert!(core.contains("c14n"), "plain `pub mod` not seen");
    // braced multi-line `pub use`
    assert!(core.contains("Sha256Hex"), "braced `pub use` not seen");
    // `pub const`
    assert!(core.contains("CRATE_NAME"), "`pub const` not seen");

    let pdf = crate_exports("engine-pdf");
    // inline `pub mod x { … }` — no semicolon
    assert!(pdf.contains("exit"), "inline `pub mod` not seen");
    // and the narrowing really took
    assert!(
        !pdf.contains("ops"),
        "`pub(crate) mod` counted as an export"
    );
}
