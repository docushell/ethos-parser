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

//! The office reader: OOXML documents, workbooks and presentations, and OpenDocument text.
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
//! **Neither format has a page, and nothing here invents one.** `docs/14-V2-SCOPE.md` §3, made
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
//! A workbook is the sharper case of the same rule: it has column widths, row heights, print
//! areas and page breaks, and every one of them describes a *printing* rather than the file's own
//! structure. A cell is addressed as `xl/workbook.xml` and its worksheet address it — sheet, row,
//! column — and by nothing else.
//!
//! # One artifact per package, and more than one part
//!
//! A DOCX is one part; a workbook is **one part per sheet**. v2-S2's page-less invariant already
//! allowed that — part id ↔ part name is a bijection rather than a cardinality-of-one rule, and
//! ordinals count per parent — so v2-S3 added nothing to `engine-core` and is simply the first
//! artifact to use the shape with more than one part in it.
//!
//! # Not every container here is OOXML
//!
//! **v2-S5 adds OpenDocument text**, which shares this crate's ZIP reader and its XML plumbing and
//! nothing else. Its vocabulary is different (`text:p`, not `w:p`), its content part is named by
//! the specification rather than by a relationship — so `opc.rs`'s whole reason for existing does
//! not apply — and it is the only one of the four that **declares its own type**, in an
//! uncompressed `mimetype` entry the package specification requires to come first.
//!
//! It is also the format where refusing a page costs the most to say, because an ODT is the only
//! one whose file contains an actual page break: `<text:soft-page-break/>` records where the
//! producing application's layout fell. It is a measurement of that producer, not of the document,
//! so it is read and discarded rather than turned into a `PageRecord` — see `odt.rs`.
//!
//! # Detection is content-based
//!
//! Anydoc's **A4**. [`is_docx`], [`is_xlsx`] and [`is_pptx`] read the bytes: a ZIP
//! local-file-header signature, and `word/document.xml`, `xl/workbook.xml` or
//! `ppt/presentation.xml` in the package's own central directory. [`is_odt`] asks the ODF question
//! instead — a first, stored `mimetype` entry declaring [`ODT_MEDIA_TYPE`] — which is what keeps
//! an `.ods` and an `.odp` from being claimed by a reader that speaks neither. A `.docx` that is
//! not OOXML is refused, and an OOXML document named `report.bin` is read, because an extension is
//! a claim anybody can make and a magic number is one only the file can. [`read`] is the router,
//! so a package claiming to be **more than one** is a named refusal rather than whichever check
//! happens to run first.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod docx;
pub mod odp;
pub mod ods;
pub mod odt;
mod opc;
pub mod pptx;
pub mod xlsx;
mod xml;
pub mod zip;

use engine_core::{
    ArtifactIdentity, Assurance, DerivationClass, DocumentRepresentation, DocxLocator, EngineError,
    GeometryAbsence, GeometryPresence, IdAllocator, IdKind, Limitation, NativeLocator, Node,
    NodeAttributes, NodeGeometry, NodeKind, OdfBlockKind, OdfCellTextSource, OdpLocator,
    OdsLocator, OdtLocator, OfficeCellAttributes, OfficeOdfCellAttributes,
    OfficeOdfShapeAttributes, OfficeParagraphAttributes, OfficeRunAttributes,
    OfficeSlideRunAttributes, PptxLocator, ProcessingRun, ProcessorIdentity, Profile,
    RepresentationPayload, Sha256Hex, SourceIdentity, XlsxLocator, REPRESENTATION_ARTIFACT_TYPE,
    REPRESENTATION_SCHEMA_VERSION,
};

/// The crate name, matching the sibling crates' own marker.
pub const CRATE_NAME: &str = "engine-office";

/// The media type an OOXML word-processing document declares.
pub const DOCX_MEDIA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

/// The media type an OOXML workbook declares.
pub const XLSX_MEDIA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// The media type an OOXML presentation declares.
pub const PPTX_MEDIA_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation";

/// The media type an OpenDocument text document declares (v2-S5).
///
/// **The only one of the four the package states about itself.** An OOXML package's type is
/// inferred from which main part its central directory lists; an ODF package writes it into an
/// uncompressed `mimetype` entry that the specification requires to come first, so this is read
/// rather than deduced. See [`odt::declared_media_type`].
pub const ODT_MEDIA_TYPE: &str = odt::ODT_MEDIA_TYPE;

/// How [`read`]'s router names the evidence that a package is an OpenDocument text document.
///
/// The other three entries in that list are part names, because that is what identifies an OOXML
/// package. This one is a sentence, because an ODF package identifies itself and the honest thing
/// to put in an "this package claims to be N kinds of document" message is the claim it made.
const ODT_CLAIM: &str = "mimetype = application/vnd.oasis.opendocument.text";

/// The media type an OpenDocument spreadsheet declares (v2-S6).
pub const ODS_MEDIA_TYPE: &str = ods::ODS_MEDIA_TYPE;

/// How [`read`]'s router names the evidence that a package is an OpenDocument spreadsheet.
const ODS_CLAIM: &str = "mimetype = application/vnd.oasis.opendocument.spreadsheet";

/// The media type an OpenDocument presentation declares (v2-S7).
pub const ODP_MEDIA_TYPE: &str = odp::ODP_MEDIA_TYPE;

/// How [`read`]'s router names the evidence that a package is an OpenDocument presentation.
const ODP_CLAIM: &str = "mimetype = application/vnd.oasis.opendocument.presentation";

/// The prefix every OpenDocument media type shares.
///
/// What separates an ODF package from the other users of the same container rule — an `.epub`
/// declares `application/epub+zip` in the same first, stored `mimetype` entry.
const ODF_MEDIA_TYPE_PREFIX: &str = "application/vnd.oasis.opendocument.";

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

/// Whether these bytes are an OOXML workbook, **read from the bytes**.
///
/// The same two questions [`is_docx`] asks, over the part only a workbook has. A `.docx` is a ZIP
/// too and does not list `xl/workbook.xml`, which is what keeps the two apart without either one
/// consulting a file name (**A4**).
///
/// Note that `xl/workbook.bin` — a macro-enabled binary workbook, `.xlsb` — is deliberately not
/// this: it is a different format with a different reader, and claiming it here would produce an
/// artifact from a part this crate cannot parse.
pub fn is_xlsx(bytes: &[u8]) -> bool {
    if !zip::looks_like_zip(bytes) {
        return false;
    }
    match zip::entry_names(bytes) {
        Ok(names) => names.iter().any(|n| n == xlsx::WORKBOOK_PART),
        Err(_) => false,
    }
}

/// Whether these bytes are an OOXML presentation, **read from the bytes**.
///
/// The same two questions [`is_docx`] asks, over the part only a presentation has. Never the
/// extension (**A4**): a deck named `deck.bin` reads, and a `.pptx` full of something else does
/// not. A legacy `.ppt` is an OLE compound file rather than a ZIP and is refused at the first
/// question, which is correct — it is a different format with a different reader.
pub fn is_pptx(bytes: &[u8]) -> bool {
    if !zip::looks_like_zip(bytes) {
        return false;
    }
    match zip::entry_names(bytes) {
        Ok(names) => names.iter().any(|n| n == pptx::PRESENTATION_PART),
        Err(_) => false,
    }
}

/// Whether these bytes are an OpenDocument **text** package, **read from the bytes** (v2-S5).
///
/// A different question from the three above, because ODF is a different container convention: an
/// OOXML package is identified by which main part it lists, and an ODF package **states its own
/// type** in a `mimetype` entry the specification requires to be first and uncompressed. So this
/// asks that entry, which is why an `.ods` and an `.odp` are not claimed here — they are the same
/// container with a different vocabulary and a different reader, and returning an empty text
/// document for one would be a gap presented as a success.
///
/// Never the extension (**A4**), in both directions: a document named `report.bin` reads, and an
/// `.odt` full of something else does not.
pub fn is_odt(bytes: &[u8]) -> bool {
    odt::is_odt(bytes)
}

/// Whether these bytes are an OpenDocument **spreadsheet**, **read from the bytes** (v2-S6).
///
/// The same question [`is_odt`] asks against a different declared type. Exact rather than
/// prefixed, so an `.ots` template is not claimed.
pub fn is_ods(bytes: &[u8]) -> bool {
    ods::is_ods(bytes)
}

/// Whether these bytes are an OpenDocument **presentation**, **read from the bytes** (v2-S7).
///
/// The same question [`is_odt`] asks against a different declared type. Exact rather than
/// prefixed, so an `.otp` template is not claimed — and neither is an `.odg`, whose `content.xml`
/// is the **same** `<draw:page>` vocabulary this reader knows. That last one is the reason the
/// match has to be exact rather than "does it look like a drawing": a graphics document would
/// parse and produce nodes, so a prefix match here would return a plausible artifact for a format
/// nobody decided to support.
pub fn is_odp(bytes: &[u8]) -> bool {
    odp::is_odp(bytes)
}

/// Whether these bytes are **any** OpenDocument package — one this engine reads or one it does not.
///
/// # Why this is a third question rather than an `||` of the other two
///
/// A caller dispatching on format needs to know *"is the office reader the one to ask"* before it
/// knows *"is this a kind the office reader implements"*, and only an ODF package can answer the
/// first about itself: it writes its type into a first, uncompressed `mimetype` entry.
///
/// Without this, an `.odp` answers `false` to every predicate here and falls through to the PDF
/// reader, which refuses it for having no `%PDF-` header. That is fail-closed and it names the
/// wrong cause — `docs/15-V2-MILESTONES.md` S5 recorded it as the one defect S5 deferred to S6, and
/// this is the half of the fix that lives outside [`read`].
///
/// # The declared type is checked, because the container rule is not ODF's alone
///
/// "First entry, stored, named `mimetype`" is the **OCF** rule, and EPUB uses it too — an `.epub`
/// declares `application/epub+zip` in exactly that place. Answering `true` on the entry's mere
/// presence would route an EPUB here to be told *"this is an OpenDocument package"*, which is
/// false, and would re-create for a format S7 parks precisely the wrong-cause refusal this slice
/// exists to close. So the declared type must be in OpenDocument's own namespace.
pub fn is_opendocument(bytes: &[u8]) -> bool {
    odt::declared_media_type(bytes)
        .is_some_and(|declared| declared.starts_with(ODF_MEDIA_TYPE_PREFIX))
}

/// Read an office package into a sealed representation, dispatching on **what the bytes contain**.
///
/// A word-processing document and a workbook are told apart by the parts their own central
/// directory lists, never by a file name (**A4**). A package that lists both main parts is a
/// **named refusal** rather than a race between two `if`s: one representation describes one
/// document, and picking whichever check ran first would make the answer depend on the order of
/// this function's lines.
///
/// # Errors
///
/// Every failure is named and nothing is partial: a package with neither main part, one with
/// both, a part that will not inflate, XML that will not parse, or a payload the seal refuses.
/// `01-CONTRACT.md` §8 — a document read as far as it went is a shorter document that still looks
/// whole.
pub fn read(bytes: &[u8]) -> Result<DocumentRepresentation, EngineError> {
    if !zip::looks_like_zip(bytes) {
        return Err(EngineError::Malformed {
            what: "office document".into(),
            detail: "these bytes do not open like a ZIP container, so there is no office package \
                     here to read"
                .into(),
        });
    }
    let names = zip::entry_names(bytes)?;

    // **Counted, not asked in order.** With four formats a chain of `if`s would make the answer
    // depend on which line ran first for a package claiming to be two of them; collecting the
    // evidence a package actually carries makes the ambiguous case impossible to reach by
    // accident and names it instead.
    //
    // The fourth entry is a different *kind* of evidence and that is the format's doing, not a
    // shortcut: OOXML packages are told apart by which main part they list, while an ODF package
    // declares its own type in a first, uncompressed `mimetype` entry. A package carrying both
    // kinds of evidence is exactly the ambiguity this shape exists to refuse — an ODF `mimetype`
    // wrapped around `word/document.xml` is a file claiming to be two documents.
    let claimed: Vec<&str> = [
        docx::MAIN_PART,
        xlsx::WORKBOOK_PART,
        pptx::PRESENTATION_PART,
    ]
    .into_iter()
    .filter(|part| names.iter().any(|n| n == part))
    .chain(odt::is_odt(bytes).then_some(ODT_CLAIM))
    .chain(ods::is_ods(bytes).then_some(ODS_CLAIM))
    .chain(odp::is_odp(bytes).then_some(ODP_CLAIM))
    .collect();

    // **An ODF package states what it is, and this engine reads two of the family.** A declared
    // type it does not implement is refused here, by name, rather than left to fall past the
    // office branch — where the PDF reader would refuse it for having no `%PDF-` header and name a
    // cause that is not the one. Checked before the match rather than inside its `[]` arm, so a
    // package carrying an ODF `mimetype` *and* an OOXML main part cannot resolve to the OOXML
    // reader: that would be this engine choosing which of the file's own claims to believe, which
    // is the ambiguity the `claimed` list exists to refuse.
    if let Some(declared) = odt::declared_media_type(bytes) {
        if declared.starts_with(ODF_MEDIA_TYPE_PREFIX)
            && declared != ODT_MEDIA_TYPE
            && declared != ODS_MEDIA_TYPE
            && declared != ODP_MEDIA_TYPE
        {
            return Err(EngineError::Unsupported {
                what: "media type".into(),
                detail: format!(
                    "this is an OpenDocument package: its `{}` entry declares `{declared}`, and \
                     this engine reads `{ODT_MEDIA_TYPE}`, `{ODS_MEDIA_TYPE}` and \
                     `{ODP_MEDIA_TYPE}` only. The container is shared and the vocabulary is not — \
                     a formula's content is `<math:math>` and a chart's is `<chart:chart>`, so \
                     reading either with a reader built for `<text:p>`, `<table:table-cell>` or \
                     `<draw:page>` would return a document with no text and no error, which is a \
                     gap presented as a success. A drawing (`.odg`) is the sharper case and is \
                     refused for a sharper reason: its content really is `<draw:page>`, so a \
                     presentation reader would produce a plausible artifact for a format nobody \
                     decided to support.",
                    odt::MIMETYPE_ENTRY
                ),
            });
        }
    }

    match claimed[..] {
        [docx::MAIN_PART] => read_docx(bytes, &names),
        [xlsx::WORKBOOK_PART] => read_xlsx(bytes, &names),
        [pptx::PRESENTATION_PART] => read_pptx(bytes, &names),
        [ODT_CLAIM] => read_odt(bytes, &names),
        [ODS_CLAIM] => read_ods(bytes, &names),
        [ODP_CLAIM] => read_odp(bytes, &names),
        [] => Err(EngineError::MissingPart {
            // **The DOCX-shaped message is kept**, because it is the one a caller handing over a
            // renamed or corrupted `.docx` needs, and it is pinned by a test.
            part: format!(
                "`{}` — this package is a ZIP but not a word-processing document, and it is \
                 neither a workbook (`{}`), a presentation (`{}`), an OpenDocument text document \
                 (`{}` declaring `{}`), an OpenDocument spreadsheet (declaring `{}`) nor an \
                 OpenDocument presentation (declaring `{}`)",
                docx::MAIN_PART,
                xlsx::WORKBOOK_PART,
                pptx::PRESENTATION_PART,
                odt::MIMETYPE_ENTRY,
                ODT_MEDIA_TYPE,
                ODS_MEDIA_TYPE,
                ODP_MEDIA_TYPE
            ),
        }),
        _ => Err(EngineError::Malformed {
            what: "office package".into(),
            detail: format!(
                "this package lists {} main parts — {} — so it claims to be more than one kind \
                 of document at once. A representation describes one document; reading it as any \
                 of them would be this engine choosing which of the file's own claims to believe.",
                claimed.len(),
                claimed.join("`, `")
            ),
        }),
    }
}

fn read_docx(bytes: &[u8], names: &[String]) -> Result<DocumentRepresentation, EngineError> {
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

    // **Declared only when there is something to declare about.** `check_geometry_matches_its_
    // declaration` requires this code to be present exactly when at least one node has no
    // measurable box, so a package with no text at all must not carry it: nothing would be
    // reconciled against it and the seal refuses the mismatch. Found at v2-S3, where an empty
    // sheet is ordinary rather than pathological.
    let mut limitations = Vec::new();
    if !nodes.is_empty() {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            "this format carries no geometry: a word-processing document has no page and no ink \
             box until something lays it out, and this engine does not",
        ));
    }
    let unread = docx::unread_text_parts(names);
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

/// Read an OOXML workbook into a sealed representation (v2-S3).
///
/// # One part id per worksheet, and why that needed nothing new
///
/// A DOCX has one part and one part id. A workbook has one **per sheet**, and the page-less
/// invariant already allows it: `check_page_less_shape` checks that a part id and a part name
/// agree *in both directions*, which is a bijection rather than a cardinality-of-one rule, and
/// `check_structure` counts ordinals **per parent**, so each sheet gets its own contiguous 1-based
/// sequence. v2-S3 added no invariant to `engine-core`; it is the first artifact to use the shape
/// v2-S2 built.
fn read_xlsx(bytes: &[u8], names: &[String]) -> Result<DocumentRepresentation, EngineError> {
    let workbook = zip::read_entry(bytes, xlsx::WORKBOOK_PART)?;
    let declared = xlsx::read_sheets(&workbook)?;

    let rels_part =
        zip::read_entry(bytes, xlsx::WORKBOOK_RELS_PART).map_err(|_| EngineError::MissingPart {
            part: format!(
                "`{}` — a workbook states its sheet names in `{}` and the part each one lives in \
                 only here, so without it no cell has an address this reader can stand behind",
                xlsx::WORKBOOK_RELS_PART,
                xlsx::WORKBOOK_PART
            ),
        })?;
    let rels = xlsx::read_relationships(&rels_part, xlsx::WORKBOOK_RELS_PART)?;
    let (sheets, non_worksheets) = xlsx::resolve_sheets(&declared, &rels)?;

    // **The string table is bound by relationship too**, for the reason the sheets are: its part
    // name is author-chosen, and `xl/sharedStrings.xml` is a convention rather than a rule.
    //
    // Absent is a legal package — a workbook whose cells are all numbers or inline strings needs
    // no table — and only *absent* becomes the empty table. A part that exists and will not
    // inflate, exceeds the size cap, or will not parse is **propagated**, because swallowing it
    // would re-surface later as "the table has 0 entries" pointing at a worksheet, which names
    // the wrong part and states the wrong cause.
    let shared = match xlsx::shared_strings_part(&rels, names) {
        Some(part_name) => {
            let part = zip::read_entry(bytes, &part_name)?;
            xlsx::read_shared_strings(&part, &part_name)?
        }
        None => Vec::new(),
    };

    let profile = Profile::xlsx_v0();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "xlsx profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256.clone());

    let mut nodes = Vec::new();
    let mut geometry = Vec::new();
    for sheet in &sheets {
        let part = zip::read_entry(bytes, &sheet.part)?;
        let cells = xlsx::read_cells(&part, &sheet.part, &shared)?;

        // One sheet, one part id — minted per sheet so the bijection holds in both directions.
        let part_id = alloc.next(IdKind::Part)?;
        for (index, cell) in cells.iter().enumerate() {
            let id = alloc.next(IdKind::Span)?;
            geometry.push(NodeGeometry {
                node: id.clone(),
                // A cell has no ink box by construction, exactly as a `<w:r>` has none. Not
                // `NotReportedByReader`: nothing tried to measure and failed, there is nothing
                // to measure until something lays the sheet out, and laying it out is the
                // invented pagination `docs/14-V2-SCOPE.md` §3 refuses.
                presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            });
            nodes.push(Node {
                id,
                // The same kind a PDF run and a `<w:r>` carry. The locator is what says this one
                // is a cell — see `NodeAttributes::kind`.
                kind: NodeKind::TextRun,
                parent: part_id.clone(),
                // Contiguous **within this sheet**, in the order the part lists its cells.
                ordinal: index as u32 + 1,
                text: cell.text.clone(),
                native_locator: NativeLocator::Xlsx(XlsxLocator {
                    part: sheet.part.clone(),
                    sheet: sheet.name.clone(),
                    row: cell.row,
                    column: cell.column.clone(),
                }),
                // No style tree is read, so no structural address is claimed.
                structural_locator: None,
                derivation: DerivationClass::Extracted,
                attributes: NodeAttributes::OfficeCell(OfficeCellAttributes {
                    value_type: cell.value_type,
                    text_source: cell.text_source,
                }),
            });
        }
    }

    let mut limitations = Vec::new();
    if !nodes.is_empty() {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            "this format carries no geometry: a workbook has no page and no ink box until \
             something prints it, and where a page break falls is a fact about the printer \
             rather than about the file",
        ));
    }

    let unread = xlsx::unread_text_parts(names);
    if unread > 0 || non_worksheets > 0 {
        let mut detail = String::new();
        if unread > 0 {
            detail.push_str(&format!(
                "{unread} part(s) of this package carry text and were not read — charts, \
                 drawings, comments or pivot caches. "
            ));
        }
        if non_worksheets > 0 {
            detail.push_str(&format!(
                "{non_worksheets} sheet(s) this workbook lists are not worksheets — a chart \
                 sheet or a dialog sheet — and have no cells to address. "
            ));
        }
        detail.push_str(
            "v2-S3 reads worksheet cells only, and a phrase absent from this artifact may still \
             be present in the workbook",
        );
        limitations.push(Limitation::document(
            engine_core::assurance::codes::OFFICE_PARTS_NOT_READ,
            detail,
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
            media_type: XLSX_MEDIA_TYPE.into(),
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
        // A spreadsheet's pages are a print artefact. `docs/06-STEAL-REFUSE.md` L30 refuses the
        // printer, so there is nothing here to declare and the vector stays empty.
        pages: Vec::new(),
        nodes,
        // **Not a table, on the format made of grids.** `tables` here means this engine's table
        // IR, produced by the PDF detectors from ruled and unruled evidence. Those never ran;
        // this artifact carries cells, and `Profile::xlsx_v0` declares `tables: false` to say so.
        tables: Vec::new(),
        assurance: Assurance::new(profile.capabilities, 0, Vec::new(), limitations)?,
    };

    DocumentRepresentation::seal(payload, geometry)
}

/// Read an OpenDocument text document into a sealed representation (v2-S5).
///
/// # One part, and the manifest is consulted anyway
///
/// The OpenDocument package specification fixes the content part's name, so there is no
/// `r:id`-to-part resolution here and `opc.rs` does not apply. What replaces it is the package's
/// own manifest: `META-INF/manifest.xml` is asked whether it declares `content.xml` and whether it
/// declares it **encrypted**, before anything is inflated. Reaching straight for the part would
/// read one the package does not list, and would hand ciphertext to the XML reader — which would
/// come back as "will not parse" and name the wrong cause.
fn read_odt(bytes: &[u8], names: &[String]) -> Result<DocumentRepresentation, EngineError> {
    // **A package that names one entry twice is refused before anything is read.** ZIP permits
    // duplicate names and consumers disagree about which one wins; `zip::read_entry` takes the
    // first. So a second `content.xml` would be neither read nor counted — the artifact would state
    // the first one's text, declare no erasure, and a consumer preferring the last would see a
    // different document. `mimetype` is worse still, because it decides detection.
    let mut sorted: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    sorted.sort_unstable();
    if let Some(pair) = sorted.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(EngineError::Malformed {
            what: "opendocument package".into(),
            detail: format!(
                "this package lists `{}` more than once. Consumers disagree about which entry of \
                 a duplicated name wins, so reading either would be this engine choosing which of \
                 the file's own claims to believe — and the entry it did not read would leave the \
                 record with nothing naming it.",
                pair[0]
            ),
        });
    }

    let manifest =
        zip::read_entry(bytes, odt::MANIFEST_PART).map_err(|_| EngineError::MissingPart {
            part: format!(
                "`{}` — an OpenDocument package states what it contains only here, and a package \
                 with no manifest is one this reader cannot say it has read",
                odt::MANIFEST_PART
            ),
        })?;
    odt::check_content_declared(&odt::read_manifest(&manifest)?)?;

    let part = zip::read_entry(bytes, odt::CONTENT_PART)?;
    let content = odt::read_content(&part)?;

    let profile = Profile::odt_v0();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "odt profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256.clone());

    // **One part, one part id**, as a DOCX has. Every node names `content.xml` in its own locator
    // and `check_structure` checks the two agree in both directions.
    let part_id = alloc.next(IdKind::Part)?;

    let mut nodes = Vec::with_capacity(content.paragraphs.len());
    let mut geometry = Vec::with_capacity(content.paragraphs.len());
    for (index, block) in content.paragraphs.iter().enumerate() {
        let id = alloc.next(IdKind::Span)?;
        geometry.push(NodeGeometry {
            node: id.clone(),
            // Not `NotReportedByReader`: nothing tried to measure and failed. An ODF paragraph
            // has no ink box until something lays the document out, and the one thing in the file
            // that looks like the result of doing so — `<text:soft-page-break/>` — is a record of
            // the producing application's layout rather than a measurement of this one's.
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        });
        nodes.push(Node {
            id,
            // The same kind a PDF run, a `<w:r>`, a cell and a slide run carry. The locator is
            // what says which — see `NodeAttributes::kind`.
            kind: NodeKind::TextRun,
            parent: part_id.clone(),
            // Contiguous within the part, in the order the part lists its blocks. **Not** the
            // locator's `paragraph`: that one counts every block the file contains, including the
            // ones in regions this reader passed over, so the two are deliberately different
            // numbers and the artifact carries both.
            ordinal: index as u32 + 1,
            text: block.text.clone(),
            native_locator: NativeLocator::Odt(OdtLocator {
                part: odt::CONTENT_PART.to_string(),
                paragraph: block.ordinal,
            }),
            // `styles.xml` is not read, so no structural address is claimed.
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeParagraph(OfficeParagraphAttributes {
                block: if block.heading {
                    OdfBlockKind::Heading
                } else {
                    OdfBlockKind::Paragraph
                },
            }),
        });
    }

    let mut limitations = Vec::new();
    if !nodes.is_empty() {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            "this format carries no geometry: an OpenDocument text document has no ink box until \
             something lays it out, and the page break its producer stored is a record of that \
             producer's layout rather than a measurement of this document",
        ));
    }

    let unread = odt::unread_entries(names);
    if unread > 0 || content.regions_not_read > 0 || content.foreign_text_not_read > 0 {
        let mut detail = String::new();
        if unread > 0 {
            detail.push_str(&format!(
                "{unread} entry(ies) of this package were not read — styles, which is where a \
                 header or a footer lives, metadata, settings, pictures or an embedded object. "
            ));
        }
        if content.regions_not_read > 0 {
            detail.push_str(&format!(
                "{} region(s) of `{}` hold text this slice does not read — a footnote or endnote \
                 body, a comment, a tracked-changes record, or a second rendition of one framed \
                 object, which a consumer displays once and this reader therefore cites once. ",
                content.regions_not_read,
                odt::CONTENT_PART
            ));
        }
        if content.foreign_text_not_read > 0 {
            detail.push_str(&format!(
                "{} block(s) contain characters that are not the block's own text — an image's \
                 title or description, an embedded object's data, a field's cached value, or a \
                 generated heading number. Those are produced by a layout or a numbering pass this \
                 reader does not perform, so they are counted rather than read into the sentence \
                 they sit inside. ",
                content.foreign_text_not_read
            ));
        }
        detail.push_str(
            "v2-S5 reads the content part's own paragraphs only, and a phrase absent from this \
             artifact may still be present in the document",
        );
        limitations.push(Limitation::document(
            engine_core::assurance::codes::OFFICE_PARTS_NOT_READ,
            detail,
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
            media_type: ODT_MEDIA_TYPE.into(),
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
        // **The format that writes its own page breaks down, and the vector is still empty.**
        // `<text:soft-page-break/>` is in `content.xml` and `fo:page-width` is in `styles.xml`;
        // between them a `PageRecord` would need no arithmetic at all. It would still be a
        // measurement of the word processor that saved the file — `docs/06-STEAL-REFUSE.md` L30 —
        // so neither is read and there is nothing here to declare.
        pages: Vec::new(),
        nodes,
        tables: Vec::new(),
        assurance: Assurance::new(profile.capabilities, 0, Vec::new(), limitations)?,
    };

    DocumentRepresentation::seal(payload, geometry)
}

/// Read an OpenDocument spreadsheet into a sealed representation (v2-S6).
///
/// # One part, one part id — and a cell rather than a paragraph
///
/// The container work is v2-S5's, unchanged and shared: the duplicate-entry refusal, the manifest
/// check, the fixed `content.xml` name. What differs is above it. The atom is the
/// `<table:table-cell>`, addressed by the table's own name and the row and column the file states
/// positionally — see [`engine_core::OdsLocator`] for why ODF leaves no third option — and the
/// facts are ODF's own value type rather than SpreadsheetML's.
fn read_ods(bytes: &[u8], names: &[String]) -> Result<DocumentRepresentation, EngineError> {
    // The same refusal `read_odt` opens with, and for the same reason: `zip::read_entry` takes the
    // first entry of a duplicated name, so a second `content.xml` would be neither read nor
    // counted and a consumer preferring the last would see a different document.
    let mut sorted: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    sorted.sort_unstable();
    if let Some(pair) = sorted.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(EngineError::Malformed {
            what: "opendocument package".into(),
            detail: format!(
                "this package lists `{}` more than once. Consumers disagree about which entry of \
                 a duplicated name wins, so reading either would be this engine choosing which of \
                 the file's own claims to believe — and the entry it did not read would leave the \
                 record with nothing naming it.",
                pair[0]
            ),
        });
    }

    let manifest =
        zip::read_entry(bytes, odt::MANIFEST_PART).map_err(|_| EngineError::MissingPart {
            part: format!(
                "`{}` — an OpenDocument package states what it contains only here, and a package \
                 with no manifest is one this reader cannot say it has read",
                odt::MANIFEST_PART
            ),
        })?;
    odt::check_content_declared(&odt::read_manifest(&manifest)?)?;

    let part = zip::read_entry(bytes, odt::CONTENT_PART)?;
    let sheets = ods::read_content(&part)?;

    let profile = Profile::ods_v0();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "ods profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let part_id = alloc.next(IdKind::Part)?;

    let mut nodes = Vec::with_capacity(sheets.cells.len());
    let mut geometry = Vec::with_capacity(sheets.cells.len());
    for (index, cell) in sheets.cells.iter().enumerate() {
        let id = alloc.next(IdKind::Span)?;
        geometry.push(NodeGeometry {
            node: id.clone(),
            // Nothing tried to measure and failed. A spreadsheet cell has no ink box until a
            // printer is chosen, and choosing one is the arithmetic §3 refuses.
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        });
        nodes.push(Node {
            id,
            kind: NodeKind::TextRun,
            parent: part_id.clone(),
            // Contiguous within the part, in the order the part lists its cells. **Not** the
            // address: `OdsLocator` carries that, and the two differ the moment a producer writes
            // a row out of order or repeats one.
            ordinal: index as u32 + 1,
            text: cell.text.clone(),
            native_locator: NativeLocator::Ods(OdsLocator {
                part: odt::CONTENT_PART.to_string(),
                table: cell.table.clone(),
                row: cell.row,
                column: cell.column,
            }),
            // `styles.xml` is not read, so no structural address is claimed.
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeOdfCell(OfficeOdfCellAttributes {
                value_type: cell.value_type,
                text_source: if cell.formula {
                    OdfCellTextSource::CachedFormulaText
                } else {
                    OdfCellTextSource::StoredText
                },
            }),
        });
    }

    let mut limitations = Vec::new();
    if !nodes.is_empty() {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            "this format carries no geometry: an OpenDocument spreadsheet has no ink box, and the \
             paper its `<style:page-layout>` names is a print setting rather than a measurement of \
             this document",
        ));
    }

    let unread = odt::unread_entries(names);
    if unread > 0
        || sheets.regions_not_read > 0
        || sheets.foreign_text_not_read > 0
        || sheets.text_outside_a_cell > 0
    {
        let mut detail = String::new();
        if unread > 0 {
            detail.push_str(&format!(
                "{unread} entry(ies) of this package were not read — styles, metadata, settings, \
                 pictures or an embedded object. "
            ));
        }
        if sheets.regions_not_read > 0 {
            detail.push_str(&format!(
                "{} region(s) of `{}` hold text this slice does not read — a note body, a comment, \
                 a tracked-changes record, a table's page-anchored drawings, or a second rendition \
                 of one framed object, which a consumer displays once and this reader therefore \
                 cites once. ",
                sheets.regions_not_read,
                odt::CONTENT_PART
            ));
        }
        if sheets.foreign_text_not_read > 0 {
            detail.push_str(&format!(
                "{} block(s) contain characters that are not the block's own text — an image's \
                 title or description, an embedded object's data, or a field's cached value such \
                 as a page number. Those are produced by a layout or a numbering pass this reader \
                 does not perform, so they are counted rather than read into the cell they sit \
                 inside. ",
                sheets.foreign_text_not_read
            ));
        }
        if sheets.text_outside_a_cell > 0 {
            detail.push_str(&format!(
                "{} block(s) held text while no cell was open, so there is no address this reader \
                 could cite them at. ",
                sheets.text_outside_a_cell
            ));
        }
        detail.push_str(
            "v2-S6 reads the content part's own cells only, and a phrase absent from this artifact \
             may still be present in the document",
        );
        limitations.push(Limitation::document(
            engine_core::assurance::codes::OFFICE_PARTS_NOT_READ,
            detail,
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
            media_type: ODS_MEDIA_TYPE.into(),
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
        // A spreadsheet's page is a printer's — v2-S3's finding, in ODF's spelling. A
        // `<style:page-layout>` states paper and a `<text:soft-page-break/>` may sit inside a
        // cell's own paragraph; both are the producing application's print arithmetic, so neither
        // is read and there is nothing here to declare.
        pages: Vec::new(),
        nodes,
        // **Not the `tables` vector, and that is the decision this reader is most likely to be
        // asked about.** A `TableRecord` is the PDF detector's finding about a grid it inferred
        // from ink, carrying a rule id that says how. A spreadsheet's cells are addresses the file
        // states outright — putting them here would claim a detector ran, and `capabilities.tables`
        // is false for the same reason.
        tables: Vec::new(),
        assurance: Assurance::new(profile.capabilities, 0, Vec::new(), limitations)?,
    };

    DocumentRepresentation::seal(payload, geometry)
}

/// Read an OpenDocument presentation into a sealed representation (v2-S7).
///
/// # One part for the whole deck, and still no page
///
/// The container work is v2-S5's, unchanged and shared for the third time. What differs is what
/// sits above it, and it differs from PresentationML as much as from ODS. A `.pptx` keeps every
/// slide in **its own part**, so [`read_pptx`] mints a part id per slide and the part name is what
/// identifies one. An `.odp` keeps every `<draw:page>` in the single `content.xml`, so there is
/// one part id here and the draw page is a **position inside it** — which is why
/// [`engine_core::OdpLocator`] has a field [`engine_core::PptxLocator`] deliberately does not.
///
/// That field is a position among elements, never a [`PageRecord`]. See [`odp`] for why this is
/// the format where that refusal costs something and is made anyway.
fn read_odp(bytes: &[u8], names: &[String]) -> Result<DocumentRepresentation, EngineError> {
    // The same refusal `read_odt` and `read_ods` open with, and for the same reason:
    // `zip::read_entry` takes the first entry of a duplicated name, so a second `content.xml`
    // would be neither read nor counted and a consumer preferring the last would see a different
    // document.
    let mut sorted: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    sorted.sort_unstable();
    if let Some(pair) = sorted.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(EngineError::Malformed {
            what: "opendocument package".into(),
            detail: format!(
                "this package lists `{}` more than once. Consumers disagree about which entry of \
                 a duplicated name wins, so reading either would be this engine choosing which of \
                 the file's own claims to believe — and the entry it did not read would leave the \
                 record with nothing naming it.",
                pair[0]
            ),
        });
    }

    let manifest =
        zip::read_entry(bytes, odt::MANIFEST_PART).map_err(|_| EngineError::MissingPart {
            part: format!(
                "`{}` — an OpenDocument package states what it contains only here, and a package \
                 with no manifest is one this reader cannot say it has read",
                odt::MANIFEST_PART
            ),
        })?;
    odt::check_content_declared(&odt::read_manifest(&manifest)?)?;

    let part = zip::read_entry(bytes, odt::CONTENT_PART)?;
    let deck = odp::read_content(&part)?;

    let profile = Profile::odp_v0();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "odp profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let part_id = alloc.next(IdKind::Part)?;

    let mut nodes = Vec::with_capacity(deck.blocks.len());
    let mut geometry = Vec::with_capacity(deck.blocks.len());
    for (index, block) in deck.blocks.iter().enumerate() {
        let id = alloc.next(IdKind::Span)?;
        geometry.push(NodeGeometry {
            node: id.clone(),
            // Nothing tried to measure and failed. A shape states `svg:x` and `svg:width` on the
            // draw page's own canvas, and this reader neither reads them nor converts them: they
            // are a position in a drawing rather than an ink box, and `docs/14-V2-SCOPE.md` §3
            // refuses a rectangle nobody can check against a page.
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        });
        nodes.push(Node {
            id,
            kind: NodeKind::TextRun,
            parent: part_id.clone(),
            // Contiguous within the part, in the order the part lists its blocks. **Not** the
            // address: `OdpLocator` carries that, and the two differ the moment a shape nests
            // inside another one.
            ordinal: index as u32 + 1,
            text: block.text.clone(),
            native_locator: NativeLocator::Odp(OdpLocator {
                part: odt::CONTENT_PART.to_string(),
                draw_page: block.draw_page,
                shape: block.shape,
                paragraph: block.paragraph,
            }),
            // `styles.xml` and the master pages are not read, so no structural address is claimed
            // — and a presentation layout's outline level is exactly the half-claim v2-S5 refused.
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeOdfShape(OfficeOdfShapeAttributes {
                draw_page_name: block.draw_page_name.clone(),
                shape_name: block.shape_name.clone(),
                block: odp::block_kind(block.heading),
            }),
        });
    }

    let mut limitations = Vec::new();
    if !nodes.is_empty() {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            "this format carries no geometry: a shape's `svg:x` and `svg:width` place it on a \
             drawing canvas rather than measure its ink, and the paper a `<style:master-page>` \
             names is the producing application's rather than a measurement of this document",
        ));
    }

    let unread = odt::unread_entries(names);
    if unread > 0
        || deck.regions_not_read > 0
        || deck.foreign_text_not_read > 0
        || deck.text_outside_a_shape > 0
    {
        let mut detail = String::new();
        if unread > 0 {
            detail.push_str(&format!(
                "{unread} entry(ies) of this package were not read — styles and the master pages \
                 they carry, metadata, settings, pictures or an embedded object. "
            ));
        }
        if deck.regions_not_read > 0 {
            detail.push_str(&format!(
                "{} region(s) of `{}` hold text this slice does not read — a speaker-notes body, a \
                 second rendition of one framed object, a drawing shape this slice does not name, \
                 a note body, a comment, or a tracked-changes record. **Speaker notes are the one \
                 to check first**: they are a second stream rather than a gap, and reading them as \
                 slide text would put a phrase in the record that nobody watching the presentation \
                 sees. ",
                deck.regions_not_read,
                odt::CONTENT_PART
            ));
        }
        if deck.foreign_text_not_read > 0 {
            detail.push_str(&format!(
                "{} block(s) or shape(s) contain characters that are not displayed text — an \
                 image's title or description, an embedded object's data, or a field's cached \
                 value such as a page number. Those are produced by a layout or a numbering pass \
                 this reader does not perform, so they are counted rather than read into the shape \
                 they sit inside. ",
                deck.foreign_text_not_read
            ));
        }
        if deck.text_outside_a_shape > 0 {
            detail.push_str(&format!(
                "{} block(s) held text while no shape was open, so there is no address this reader \
                 could cite them at. ",
                deck.text_outside_a_shape
            ));
        }
        detail.push_str(
            "v2-S7 reads the content part's own draw pages only, and a phrase absent from this \
             artifact may still be present in the document",
        );
        limitations.push(Limitation::document(
            engine_core::assurance::codes::OFFICE_PARTS_NOT_READ,
            detail,
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
            media_type: ODP_MEDIA_TYPE.into(),
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
        // **The one place in this crate where a `PageRecord` was available for free**, and the
        // vector is still empty. `<draw:page>` elements are discrete, ordered and named, and a
        // `<style:master-page>` states `fo:page-width` beside them — no arithmetic at all. A draw
        // page is a part of the presentation's structure rather than a page this engine measured,
        // and `docs/06-STEAL-REFUSE.md` L30 refuses invented pagination whether inventing it costs
        // a renderer or costs nothing.
        pages: Vec::new(),
        nodes,
        // Not the `tables` vector either. A `<table:table>` on a draw page is a shape's content,
        // and a `TableRecord` is the PDF detector's finding about a grid it inferred from ink;
        // `capabilities.tables` is false because no detector ran.
        tables: Vec::new(),
        assurance: Assurance::new(profile.capabilities, 0, Vec::new(), limitations)?,
    };

    DocumentRepresentation::seal(payload, geometry)
}

/// Read an OOXML presentation into a sealed representation (v2-S4).
///
/// # One part id per slide, and no page anywhere
///
/// A slide is a part, so the shape v2-S3 built for one-part-per-sheet serves unchanged: a part id
/// per slide, ordinals contiguous within each, and the part-id ↔ part-name bijection carrying
/// three formats now instead of one. `pages` stays empty — see `pptx.rs` for why a slide's size
/// and its position in the deck are both things this engine will not turn into a `PageRecord`.
fn read_pptx(bytes: &[u8], names: &[String]) -> Result<DocumentRepresentation, EngineError> {
    let presentation = zip::read_entry(bytes, pptx::PRESENTATION_PART)?;
    let rel_ids = pptx::read_slide_refs(&presentation)?;

    let rels_part = zip::read_entry(bytes, pptx::PRESENTATION_RELS_PART).map_err(|_| {
        EngineError::MissingPart {
            part: format!(
                "`{}` — a presentation lists its slides in `{}` and the part each one lives in \
                 only here, so without it no run has an address this reader can stand behind",
                pptx::PRESENTATION_RELS_PART,
                pptx::PRESENTATION_PART
            ),
        }
    })?;
    let rels = opc::read_relationships(&rels_part, pptx::PRESENTATION_RELS_PART)?;
    let (slides, non_slides) = pptx::resolve_slides(&rel_ids, &rels)?;

    let profile = Profile::pptx_v0();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "pptx profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256.clone());

    let mut nodes = Vec::new();
    let mut geometry = Vec::new();
    let mut shapes_not_read = 0u32;
    let mut alternatives_not_read = 0u32;

    for slide in &slides {
        let part = zip::read_entry(bytes, slide)?;
        let content = pptx::read_slide(&part, slide)?;
        shapes_not_read += content.shapes_not_read;
        alternatives_not_read += content.alternatives_not_read;

        // One slide, one part id — minted per slide so the bijection holds in both directions.
        let part_id = alloc.next(IdKind::Part)?;
        for (index, run) in content.runs.iter().enumerate() {
            let id = alloc.next(IdKind::Span)?;
            geometry.push(NodeGeometry {
                node: id.clone(),
                // A slide shape has an `<a:xfrm>` offset and extent, and this is still typed
                // absence: those are numbers an authoring tool wrote about placement, not ink
                // this engine measured. `check_structure` refuses a measured box on a page-less
                // node, so the reader could not emit one even if a later edit read the xfrm.
                presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            });
            nodes.push(Node {
                id,
                kind: NodeKind::TextRun,
                parent: part_id.clone(),
                // Contiguous **within this slide**, in the order the part lists its shapes.
                ordinal: index as u32 + 1,
                text: run.text.clone(),
                native_locator: NativeLocator::Pptx(PptxLocator {
                    part: slide.clone(),
                    shape: run.shape,
                    paragraph: run.paragraph,
                    run: run.run,
                }),
                structural_locator: None,
                derivation: DerivationClass::Extracted,
                attributes: NodeAttributes::OfficeSlideRun(OfficeSlideRunAttributes {
                    shape_id: run.shape_id,
                    shape_name: run.shape_name.clone(),
                }),
            });
        }
    }

    let mut limitations = Vec::new();
    if !nodes.is_empty() {
        limitations.push(Limitation::document(
            engine_core::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            "this format carries no geometry: a slide states where an authoring tool placed a \
             shape, which is not a measurement of ink and not a page this engine read",
        ));
    }

    let unread = pptx::unread_text_parts(names);
    if unread > 0 || non_slides > 0 || shapes_not_read > 0 || alternatives_not_read > 0 {
        let mut detail = String::new();
        if unread > 0 {
            detail.push_str(&format!(
                "{unread} part(s) of this package carry text and were not read — speaker notes, \
                 masters, layouts, comments, charts or diagrams. "
            ));
        }
        if non_slides > 0 {
            detail.push_str(&format!(
                "{non_slides} entry(ies) in the slide list are not slides. "
            ));
        }
        if shapes_not_read > 0 {
            detail.push_str(&format!(
                "{shapes_not_read} shape(s) on the slides that were read hold text this slice \
                 does not read — a table or chart in a `<p:graphicFrame>`, or an `<a:fld>` whose \
                 text is a cached slide number rather than something the deck states. "
            ));
        }
        if alternatives_not_read > 0 {
            detail.push_str(&format!(
                "{alternatives_not_read} `<mc:AlternateContent>` branch(es) holding text were \
                 passed over — a deck states the same content more than once for consumers of \
                 different capability, and this reader takes the first rather than emitting one \
                 phrase at two addresses. "
            ));
        }
        detail.push_str(
            "v2-S4 reads slide shape text only, and a phrase absent from this artifact may still \
             be present in the presentation",
        );
        limitations.push(Limitation::document(
            engine_core::assurance::codes::OFFICE_PARTS_NOT_READ,
            detail,
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
            media_type: PPTX_MEDIA_TYPE.into(),
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
        // **A slide is a part, not a page.** The empty vector is what keeps a deck's slide
        // numbers from becoming page numbers on the wire.
        pages: Vec::new(),
        nodes,
        tables: Vec::new(),
        assurance: Assurance::new(profile.capabilities, 0, Vec::new(), limitations)?,
    };

    DocumentRepresentation::seal(payload, geometry)
}
