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

//! The office reader: OOXML word-processing documents into the representation (v2-S2).
//!
//! # The fifth crate, and why it exists now and not before
//!
//! `docs/04-ARCHITECTURE.md`: *"A fifth crate before a second format is speculative structure.
//! Revisit only when office or OCR needs a real home — `engine-office`, `engine-ocr` — and not
//! before."* DOCX is that second format, so this is that home. `engine-core` never learns what a
//! `.docx` is, exactly as it never learned what a PDF is; the boundary table binds this crate the
//! same way it binds `engine-pdf`.
//!
//! # The one sentence
//!
//! **A DOCX has no page, and nothing here invents one.** `docs/14-V2-SCOPE.md` §3, made
//! mechanical in three places:
//!
//! - `payload.pages` is **empty**. There is no A4 record, no "72 DPI" default, and no renderer in
//!   this crate's dependency graph to produce one from.
//! - Every node's parent is a **part** id, not a page id. `check_structure` refuses the other
//!   spelling for a page-less address.
//! - Every node's geometry is `NotApplicableToKind`. `check_structure` refuses a *measured* box
//!   on a page-less node outright, so the rule is not a convention this crate follows — it is one
//!   the artifact cannot break.
//!
//! # Detection is content-based
//!
//! Anydoc's **A4**. [`is_docx`] reads the bytes: a ZIP local-file-header signature, and
//! `word/document.xml` in the package's own central directory. A `.docx` that is not OOXML is
//! refused, and an OOXML document named `report.bin` is read — because an extension is a claim
//! anybody can make and a magic number is one only the file can.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod docx;
pub mod zip;

use engine_core::{
    ArtifactIdentity, Assurance, DerivationClass, DocumentRepresentation, DocxLocator, EngineError,
    GeometryAbsence, GeometryPresence, IdAllocator, IdKind, Limitation, NativeLocator, Node,
    NodeAttributes, NodeGeometry, NodeKind, OfficeRunAttributes, ProcessingRun, ProcessorIdentity,
    Profile, RepresentationPayload, Sha256Hex, SourceIdentity, REPRESENTATION_ARTIFACT_TYPE,
    REPRESENTATION_SCHEMA_VERSION,
};

/// The crate name, matching the sibling crates' own marker.
pub const CRATE_NAME: &str = "engine-office";

/// The media type an OOXML word-processing document declares.
pub const DOCX_MEDIA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// Whether these bytes are an OOXML word-processing document, **read from the bytes**.
///
/// Two questions, both answered by the file: does it open like a ZIP, and does its central
/// directory list `word/document.xml`? A workbook is a ZIP too and does not list that part, which
/// is what keeps this from claiming an `.xlsx`.
///
/// Never the extension (**A4**). A renamed document still reads; a `.docx` full of something else
/// does not.
pub fn is_docx(bytes: &[u8]) -> bool {
    if !zip::looks_like_zip(bytes) {
        return false;
    }
    match zip::entry_names(bytes) {
        Ok(names) => names.iter().any(|n| n == docx::MAIN_PART),
        Err(_) => false,
    }
}

/// Read an OOXML word-processing document into a sealed representation.
///
/// # Errors
///
/// Every failure is named and nothing is partial: a package with no `word/document.xml`, a part
/// that will not inflate, XML that will not parse, or a payload the seal refuses. `01-CONTRACT.md`
/// §8 — a document read as far as it went is a shorter document that still looks whole.
pub fn read(bytes: &[u8]) -> Result<DocumentRepresentation, EngineError> {
    if !zip::looks_like_zip(bytes) {
        return Err(EngineError::Malformed {
            what: "office document".into(),
            detail: "these bytes do not open like a ZIP container, so there is no OOXML package \
                     here to read"
                .into(),
        });
    }
    let names = zip::entry_names(bytes)?;
    if !names.iter().any(|n| n == docx::MAIN_PART) {
        return Err(EngineError::MissingPart {
            part: format!(
                "`{}` — this package is a ZIP but not a word-processing document",
                docx::MAIN_PART
            ),
        });
    }

    let part = zip::read_entry(bytes, docx::MAIN_PART)?;
    let runs = docx::read_runs(&part)?;

    let profile = Profile::docx_v0();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "docx profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256.clone());

    // **One part, one part id.** Every node names `word/document.xml` in its own locator, and
    // `check_structure` checks the two agree in both directions.
    let part_id = alloc.next(IdKind::Part)?;

    let mut nodes = Vec::with_capacity(runs.len());
    let mut geometry = Vec::with_capacity(runs.len());
    for (index, run) in runs.iter().enumerate() {
        let id = alloc.next(IdKind::Span)?;
        geometry.push(NodeGeometry {
            node: id.clone(),
            // **Not `NotReportedByReader`.** That one means the reader tried to measure and could
            // not, and counts toward a declared capability gap. A DOCX run has no ink box by
            // construction — there is nothing to measure until something lays the document out,
            // and laying it out is the invented pagination §3 refuses.
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        });
        nodes.push(Node {
            id,
            kind: NodeKind::TextRun,
            parent: part_id.clone(),
            // Ordinals are contiguous within the part, in the part's document order — which is
            // the reading order OOXML itself states.
            ordinal: index as u32 + 1,
            text: run.text.clone(),
            native_locator: NativeLocator::Docx(DocxLocator {
                part: docx::MAIN_PART.to_string(),
                paragraph: run.paragraph,
                run: run.run,
            }),
            // The style tree is not read, so no structural address is claimed. Absent rather than
            // invented, as everywhere else.
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeRun(OfficeRunAttributes {
                space_preserved: run.space_preserved,
            }),
        });
    }

    let mut limitations = vec![Limitation::document(
        engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
        "this format carries no geometry: a word-processing document has no page and no ink box \
         until something lays it out, and this engine does not",
    )];
    let unread = docx::unread_text_parts(&names);
    if unread > 0 {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::OFFICE_PARTS_NOT_READ,
            format!(
                "{unread} part(s) of this package carry text and were not read — headers, \
                 footers, footnotes, endnotes or comments. v2-S2 reads `{}` only, and a phrase \
                 absent from this artifact may still be present in the document",
                docx::MAIN_PART
            ),
        ));
    }

    let payload = RepresentationPayload {
        identity: ArtifactIdentity {
            artifact_type: REPRESENTATION_ARTIFACT_TYPE.into(),
            schema_version: REPRESENTATION_SCHEMA_VERSION.into(),
            parser_version: profile.parser_version.clone(),
            profile_sha256,
        },
        source: SourceIdentity {
            media_type: DOCX_MEDIA_TYPE.into(),
            sha256: Sha256Hex::of_bytes(bytes),
        },
        processing_run: ProcessingRun {
            processor: ProcessorIdentity {
                name: "ethos-engine".into(),
                version: profile.parser_version.clone(),
                backend: format!("{} {}", profile.backend.name, profile.backend.version),
            },
            reading_order_rule: profile.reading_order_rule.clone(),
        },
        coordinate_system: profile.coordinate_system,
        // **The empty vector is the whole point.** `docs/14-V2-SCOPE.md` §3: a page-less format
        // carries no pages, and a record put here so a node could name it would be the invented
        // pagination this version exists to refuse.
        pages: Vec::new(),
        nodes,
        tables: Vec::new(),
        // No pages, so nothing was authorized and nothing has a page state. The coverage summary
        // reconciles zero against zero, which is what a document with no pages honestly is.
        assurance: Assurance::new(profile.capabilities, 0, Vec::new(), limitations)?,
    };

    DocumentRepresentation::seal(payload, geometry)
}
