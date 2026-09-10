// Copyright 2026 The ethos-parser maintainers
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

//! `DocumentRepresentation v0` — the canonical evidence record (`docs/01-CONTRACT.md` §2, §5).
//!
//! **The representation is the record; the grounding artifact is a projection of it.** Never
//! treat a grounding round-trip as proof the representation is intact — the projection is
//! deliberately lossy, and `ethos-parser-grounding` has a test enumerating exactly what it drops.
//!
//! # Why the locator union lives in this crate
//!
//! `docs/04-ARCHITECTURE.md` §1 says `ethos-parser-core` contains no PDF concept, and its own header
//! says that where it and `01-CONTRACT.md` disagree, **the contract is right and the
//! architecture doc is the bug**. §5.1 of the contract is explicit: *"`NativeLocator` is a
//! discriminated union. Adding a format adds a variant — `PdfLocator`, `DocxLocator`, … v0 ships
//! `PdfLocator` and nothing else."* The union is part of the artifact contract, so it lives with
//! the contract.
//!
//! The line that actually matters is **machinery, not vocabulary**: no `lopdf`, no
//! content-stream operator, no page tree, no font program. [`PdfLocator`] is four integers and a
//! discriminant. Nothing in this crate can parse a PDF, and nothing here knows one exists beyond
//! the name of an addressing scheme — the same line `BackendIdentity { name: "lopdf" }` already
//! sits on.
//!
//! **`ethos-parser-grounding` never reads a locator at all.** The projection addresses pages by node
//! id, so it never matches on a format variant; a test asserts the crate does not so much as
//! mention `NativeLocator`. That is what keeps the second format from becoming a rewrite, and it
//! is a stronger guarantee than hiding the type would have given.
//!
//! # The fingerprint, and why it is a subtree rather than a filter
//!
//! The `representation_c14n_sha256` field — [`DocumentRepresentation::fingerprint`] — is
//! `"sha256:" + hex(sha256(c14n(doc["representation"])))` — the digest of a **literal subtree of
//! the emitted document**. A reader recomputes it with no domain knowledge: canonicalize that
//! one member, hash it.
//!
//! The alternative — hash the whole document minus a named set of keys — makes canonicalization
//! a second rule two implementations can drift on, and a drifted rule produces a mismatch that
//! looks exactly like tampering. Keeping the hashed bytes literal means a reader who gets it
//! wrong gets it *obviously* wrong.

use serde::{Deserialize, Serialize};

use crate::assurance::Assurance;
use crate::c14n::sha256_hex_bytes;
use crate::derivation::{DerivationClass, GeometryPresence};
use crate::error::EngineError;
use crate::geom::QRect;
use crate::identity::{ArtifactIdentity, CoordinateSystem, Sha256Hex};
use crate::ids::NodeId;

/// Artifact type for the canonical record. **DRAFT** — see `docs/draft-schemas/`.
pub const REPRESENTATION_ARTIFACT_TYPE: &str = "ethos.parser.representation.v0";

/// Shape version of the representation artifact. **DRAFT**.
///
/// `0.5.0` at v1.1-S2: [`crate::TableCellRecord`] gained `node_ids`. A record written under
/// `0.4.0` carries a cell's text with no link back to the runs it is a concatenation of, and this
/// build refuses it rather than defaulting the field to empty — an empty `node_ids` means "this
/// cell encloses no run", which is a different statement from "this file predates the link".
///
/// `0.6.0` at v2-S24: [`crate::TableRecord`] and [`crate::TableCellRecord`] replaced their required
/// `bbox: QRect` with `geometry: `[`crate::GeometryPresence`], so a **tagged** table can report its
/// box as absent rather than carry an invented one. A record written under `0.5.0` has a `bbox`
/// key this shape does not, and vice versa, so the two are genuinely non-comparable and the version
/// says so rather than a reader silently coercing one into the other.
pub const REPRESENTATION_SCHEMA_VERSION: &str = "0.6.0";

/// What was read: the media type and the digest of the exact source bytes.
///
/// **Source identity, not representation identity.** `docs/01-CONTRACT.md` §2 requires both and
/// they are not the same thing: this is what went in, and
/// [`DocumentRepresentation::fingerprint`] is what came out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    /// The source's media type. **`application/pdf` at v0, and one of nine since v2-S2.**
    ///
    /// This read *"Always `application/pdf` at v0"* until v2-S13.5, which was true of v0 and had
    /// stopped describing the field: `ethos-parser-office` writes eight more into it — the OOXML three,
    /// the ODF three, `application/rtf` and `application/epub+zip`. A caller must not treat this
    /// as a constant.
    ///
    /// **Not to be confused with `ethos_parser_grounding::Source::media_type`**, which IS pinned to
    /// `application/pdf` by the `ethos.grounding.v1` schema and is correctly documented as such.
    /// Decision **#16** turns on that distinction: a DOCX cannot be *grounded* because the
    /// verifier's field is a const, and it can still *bind* because this one is not.
    pub media_type: String,
    /// Digest of the exact source bytes.
    pub sha256: Sha256Hex,
}

/// Who produced this representation, and under what pinned configuration.
///
/// The companion names a `ProcessingRun`/`StageRun` carrying processor, adapter, build and
/// profile identities. v0 has one processor and one stage, so this is the honest subset: naming
/// a `stages` array with a single hardcoded entry would be shape without information.
///
/// **No run id and no timestamp.** Either would make two runs over identical bytes under an
/// identical profile produce different artifacts, and byte identity across runs is a test here
/// (`docs/history/05-MILESTONES.md`, standing rule 6). A run's identity *is* its inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingRun {
    /// The processor that read the source.
    pub processor: ProcessorIdentity,
    /// Version id of the reading-order rule that ordered the nodes.
    ///
    /// On the artifact so a reader need not fetch the profile to know which rule produced this
    /// order. `gutter-columns-v1` since v1-S5: nodes are ordered by whitespace in page space, so
    /// a two-column document reads column-major and a page with no gutter is not reordered at
    /// all. `single-column-v1` — v0's value, and still a legal one — means content-stream order
    /// with nothing reordered, and a profile naming it also declares the multi-column limitation.
    ///
    /// **Two artifacts under the two ids can list the same runs in a different sequence**, which
    /// is why the string is here and not only in the profile.
    pub reading_order_rule: String,
}

/// Processor name, build, and the backend it read through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessorIdentity {
    /// Engine name.
    pub name: String,
    /// Engine build.
    pub version: String,
    /// Backend name and version, e.g. `lopdf 0.44.0`.
    pub backend: String,
}

/// The format-native address of a node. **Required on every node** (`01-CONTRACT.md` §5.1).
///
/// A discriminated union: adding a format adds a variant, plus an adapter profile, fixtures and
/// inspection behaviour. It does not change the source, run, artifact or verification models.
///
/// Externally tagged so a new variant is additive on the wire and an unrecognised one fails
/// closed rather than being best-effort parsed (§8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum NativeLocator {
    /// A glyph run's address: page, baseline origin, advance.
    Pdf(PdfLocator),
    /// A PDF **object's** address: page, object number, and the rectangle it declares (v1-S4).
    ///
    /// An annotation and a form field are not glyph runs. They have no baseline, no advance, and
    /// no character origin, so [`PdfLocator`] is the wrong shape for them — and filling it with a
    /// plausible origin would put a coordinate on the wire that the document does not contain.
    /// `docs/01-CONTRACT.md` §5.1 makes the locator a union precisely so a new *kind* of address
    /// is a new variant rather than a lie in an old one.
    PdfObject(PdfObjectLocator),
    /// A painted image's address: page, XObject number, and the rectangle the `Do` filled (v1-S6).
    ///
    /// **A third variant rather than a reuse of [`Self::PdfObject`], because the rectangle means
    /// something else.** An annotation's `/Rect` is a number the author wrote into a dictionary.
    /// An image's rectangle is *computed* — the current transformation matrix applied to the unit
    /// square the image is defined on — so filing it under [`AnnotationRect::Declared`] would
    /// claim the document stated something it never stated. `docs/01-CONTRACT.md` §5.1 makes the
    /// locator a union so a new *kind* of address is a new variant rather than a lie in an old one,
    /// and "declared by the author" versus "derived from the page's own matrix" is exactly that
    /// kind of difference.
    PdfImage(PdfImageLocator),
    /// A run's address inside an OOXML word-processing document: part, paragraph, run (v2-S2).
    ///
    /// **Every field is something the file contains.** `word/document.xml` is a part name from
    /// the package; the paragraph and run ordinals are the positions of the `<w:p>` and `<w:r>`
    /// elements in that part's own document order. Nothing here is laid out, and nothing here
    /// could be: `docs/history/14-V2-SCOPE.md` §3 forbids a locator that addresses a *rendering*, and a
    /// DOCX has no page until a renderer decides where one falls.
    ///
    /// There is no page, no box, no `x`/`y`, and no room to add one — the struct denies unknown
    /// fields, and `01-CONTRACT.md` §5.1's rule is that a new *kind* of address is a new variant
    /// rather than a lie in an old one.
    Docx(DocxLocator),
    /// A cell's address inside an OOXML workbook: part, sheet, row, column (v2-S3).
    ///
    /// **Every field is something the file contains**, and the two halves of the address are
    /// spelled the two different ways the file spells them — see [`XlsxLocator`].
    ///
    /// There is no page, no bbox, no column width and no print range. A spreadsheet's "page" is
    /// something a printer decides, not something the workbook states, so it is exactly the
    /// rendering `docs/history/14-V2-SCOPE.md` §3 forbids a locator from addressing.
    Xlsx(XlsxLocator),
    /// A text run's address inside an OOXML presentation: part, shape, paragraph, run (v2-S4).
    ///
    /// **The format where calling a part a page is most tempting, and still wrong.** A slide is a
    /// discrete addressable thing the package contains — unlike a DOCX page, which does not exist
    /// until a renderer decides where one falls, and unlike a spreadsheet's page, which a printer
    /// decides. That makes it a **part**, exactly as a worksheet is, and not a `PageRecord`: a
    /// slide's position in `p:sldIdLst` is display order, and putting it on the wire as a page
    /// index would be a citation shaped like a PDF page number.
    Pptx(PptxLocator),
    /// A paragraph's address inside an OpenDocument **text** document: part, paragraph (v2-S5).
    ///
    /// **The first v2 format that is not OOXML**, and the first whose file *does* contain
    /// something spelled as a page: `<text:soft-page-break/>` marks where the producing
    /// application's own layout broke the page, and `style:master-page` states a paper size. Both
    /// are a rendering the producer performed — which is exactly what `docs/history/14-V2-SCOPE.md` §3
    /// forbids a locator from addressing — so neither reaches this variant and neither becomes a
    /// `PageRecord`.
    Odt(OdtLocator),
    /// A cell's address inside an OpenDocument **spreadsheet** part (v2-S6).
    ///
    /// **The first v2 format that states no address at all.** A workbook writes `<c r="B12">` and
    /// [`XlsxLocator`] reads it; OpenDocument writes neither a row number nor a column letter, and
    /// states a cell's position by where it sits among its siblings — compressed by
    /// `table:number-columns-repeated`, which is the file saying *"and n more of these"*. Counting
    /// that is reading the position the file states, in the same sense [`OdtLocator::paragraph`]
    /// counts blocks. Inventing a letter for it would be worse: `B` is a spreadsheet
    /// application's convention, not a string this document contains.
    Ods(OdsLocator),
    /// A block's address inside an OpenDocument **presentation** part (v2-S7).
    ///
    /// **The format where a `PageRecord` would need no arithmetic at all**, and still does not get
    /// one. A `<draw:page>` is discrete, listed, named, and counted out loud — *"it's on slide
    /// 12"* — and a master page states paper in an `fo:page-width` beside it. Between them a
    /// [`PageRecord`] could be minted without this engine measuring anything, which is exactly why
    /// it is refused: a draw page is **a part of the presentation's structure**, and putting its
    /// position on the wire as a page would hand a consumer a page index this engine never
    /// verified. `docs/06-STEAL-REFUSE.md` L30 states the reason in three words — *"It invents
    /// pagination"* — so [`OdpLocator::draw_page`] is a position among elements the file lists,
    /// spelled as the element ODF actually writes.
    Odp(OdpLocator),
    /// A paragraph's address inside a Rich Text Format stream (v2-S8).
    ///
    /// **The first v2 format with no container at all.** Every format before this one is a
    /// package: the address names a part, and `check_structure` checks that one part id means one
    /// part name. An `.rtf` is a single brace-group byte stream with no parts, no manifest and no
    /// name for itself — so this variant carries **one** field, and the invariant grew a third
    /// case rather than this locator growing an invented part name. `docs/history/14-V2-SCOPE.md` §3's
    /// second obligation is *"absent, not invented"*, and a constant standing in for a part the
    /// format does not have is the small version of the page-sized box it forbids.
    Rtf(RtfLocator),
    /// A block's address inside an EPUB spine document (v2-S9).
    ///
    /// **The format that might genuinely have had pages, and does not.** §3's law was never "no
    /// page ever" — it is *no page this engine did not read from the file* — and this is the first
    /// v2 format that could have satisfied the reading half. An EPUB 3 navigation document may
    /// carry a `page-list` mapping locations to the page numbers of a print edition, and an EPUB 2
    /// NCX may carry page targets. Neither is a page **this engine measured**: they are labels a
    /// publisher wrote about somebody else's paper, with no width, no height and nothing to
    /// validate a box against. So `pages` stays `[]` here too, and the label is not copied onto a
    /// [`PageRecord`] under a different name.
    ///
    /// The address is the spine item's package path plus a position in that document. See
    /// [`EpubLocator`] for why the path comes from the **spine** and not from the ZIP.
    Epub(EpubLocator),
}

impl NativeLocator {
    /// Whether this address is inside a document the format itself paginates.
    ///
    /// **The one question `check_structure` asks before deciding what a node's parent may be.**
    /// A paginated node is parented by a declared [`PageRecord`]; a page-less one is parented by
    /// a part. Splitting on the locator rather than on a flag means a PDF node cannot reach the
    /// page-less rules by setting a boolean — the only way in is to carry an address that has no
    /// page in it, which is a thing the reader either read or did not.
    pub fn is_paginated(&self) -> bool {
        match self {
            Self::Pdf(_) | Self::PdfObject(_) | Self::PdfImage(_) => true,
            // **Eight page-less variants, and each has its own reason.** The arm below carried
            // running ordinals — "the sharpest of the four", then "the sharpest of the six" a
            // dozen lines under it — which were true at v2-S5 and v2-S7 and had been wrong ever
            // since, in opposite directions at the same time. They are gone rather than
            // renumbered: an ordinal that has to be revised every time a format lands is a
            // maintenance obligation nothing enforces, and the ordering the comments actually
            // want is "each case is sharper than the last", which does not need a denominator.
            //
            // A workbook has print layout, not pages. Where a page break falls depends on the
            // printer, the paper and a "fit to page" setting, none of which is in the file —
            // which is why `docs/06-STEAL-REFUSE.md` L30 is a refusal rather than a fallback.
            // A workbook has print layout, not pages; a presentation has slides, which are
            // parts. `p:sldSz` states a slide's size, but a size is not a page and this engine
            // measured nothing against it.
            //
            // And an OpenDocument text document is the sharpest case so far, because its
            // `content.xml` literally contains `<text:soft-page-break/>`: a position the
            // *producing application* computed from its own font stack and paper size and wrote
            // down. It is a record of somebody else's rendering, which is the thing L30 refuses
            // rather than a fact about the document — so it is not read, and there is no page
            // here either.
            // An OpenDocument spreadsheet is the workbook case again, in ODF's spelling: a
            // `<style:page-layout>` states paper and a `<text:soft-page-break/>` may appear in a
            // cell, and both are the producing application's print arithmetic rather than a page
            // this engine measured.
            //
            // And an OpenDocument presentation is sharper still, because it needs no
            // arithmetic at all: `<draw:page>` elements are discrete, listed and ordered, and a
            // master page states `fo:page-width` beside them. A `PageRecord` was available for
            // free and is still refused — a draw page is a part of the presentation's structure,
            // not a page this engine measured, and L30's objection to the LibreOffice bridge is
            // that pagination handed over by somebody else's renderer is invented here.
            //
            // And an RTF mentions a page in the plainest words any of them use: `\page` is a
            // page break and `\paperw` is a paper width, both written by the producer. Neither is
            // a page this engine measured, which is the same sentence L30 makes about the
            // LibreOffice bridge — a page handed over by somebody else's layout is invented here.
            Self::Docx(_)
            | Self::Xlsx(_)
            | Self::Pptx(_)
            | Self::Odt(_)
            | Self::Ods(_)
            | Self::Odp(_)
            //
            // And an EPUB is the one that could have read a page from the file and still does not.
            // A `page-list` in its navigation document names the pages of a PRINT edition, and an
            // NCX page target does the same in EPUB 2's spelling. Both are a publisher's label
            // about paper this engine never saw — no width, no height, nothing a box could be
            // validated against — so they are not read as pagination and `pages` stays empty.
            | Self::Rtf(_)
            | Self::Epub(_) => false,
        }
    }

    /// The part this address lives in, for a page-less format. `None` when paginated.
    pub fn part(&self) -> Option<&str> {
        match self {
            Self::Pdf(_) | Self::PdfObject(_) | Self::PdfImage(_) => None,
            Self::Docx(d) => Some(d.part.as_str()),
            Self::Xlsx(x) => Some(x.part.as_str()),
            Self::Pptx(p) => Some(p.part.as_str()),
            Self::Odt(o) => Some(o.part.as_str()),
            Self::Ods(o) => Some(o.part.as_str()),
            Self::Odp(o) => Some(o.part.as_str()),
            // **A page-less address that names no part**, which is a third answer rather than a
            // missing one — see [`Self::names_a_part`], which is the question `check_structure`
            // actually asks.
            Self::Rtf(_) => None,
            Self::Epub(e) => Some(e.part.as_str()),
        }
    }

    /// Whether this address names a part, for a page-less format that has one (v2-S8).
    ///
    /// Until this slice `part()` returning `None` meant *"paginated"*, because every page-less
    /// format so far was a package. RTF is neither: it is page-less **and** part-less, so the
    /// overload had to be separated before `check_structure` could tell the two apart. A locator
    /// that answers false here is checked on the one integrity claim it can make — every node
    /// shares one container id — rather than on the part-id bijection, which needs a part name to
    /// be a bijection between.
    pub fn names_a_part(&self) -> bool {
        match self {
            Self::Pdf(_) | Self::PdfObject(_) | Self::PdfImage(_) | Self::Rtf(_) => false,
            Self::Docx(_)
            | Self::Xlsx(_)
            | Self::Pptx(_)
            | Self::Odt(_)
            | Self::Ods(_)
            | Self::Odp(_)
            // An EPUB is a package with **many** parts — one per spine document — so it takes the
            // bijection rather than v2-S8's one-container rule. v2-S9 is the first format to use
            // both halves of the shape v2-S8 split apart.
            | Self::Epub(_) => true,
        }
    }
}

/// A run's address inside an OOXML word-processing part (v2-S2).
///
/// Ordinals are **1-based**, in the part's own document order, matching every other 1-based
/// ordinal in this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocxLocator {
    /// The package part this run was read from, e.g. `word/document.xml`.
    ///
    /// The package's own name for it, verbatim — not a path on disk and not a guess.
    pub part: String,
    /// 1-based position of the `<w:p>` this run belongs to, in the part's document order.
    pub paragraph: u32,
    /// 1-based position of the `<w:r>` within that paragraph.
    pub run: u32,
}

/// A cell's address inside an OOXML worksheet part (v2-S3).
///
/// # Four fields, and each one is a string the package wrote
///
/// `docs/history/14-V2-SCOPE.md` §3 names "a cell, a sheet" as things a v2 locator may address, and this
/// is that address. Nothing here is laid out and nothing here could be: a workbook has no page
/// until a printer decides where one falls, and a column's *width* is a rendering instruction
/// rather than part of a cell's name.
///
/// # Why both `part` and `sheet`, when one address usually needs one name
///
/// They answer different questions and neither substitutes for the other.
///
/// `part` is the **package's** name for the worksheet — `xl/worksheets/sheet1.xml` — and it is
/// what `check_structure`'s part-id ↔ part-name bijection is checked against, exactly as
/// [`DocxLocator::part`] is. `sheet` is the **workbook's** name for it — what `<sheet name="…">`
/// says, which is what a person citing a cell writes down and what the part name does not
/// contain. The two are bound by `xl/_rels/workbook.xml.rels`, which the reader resolves rather
/// than guesses: part names are author-chosen, and a workbook that has had a sheet deleted has
/// `sheet1.xml` and `sheet3.xml` with no `sheet2.xml`, so position is not the mapping.
///
/// # Why the row is a number and the column is a string
///
/// Because that is how the file writes them. `<c r="B12">` states the column as the letters `B`
/// and the row as the digits `12`; `<row r="12">` states the same row the same way. Reading `12`
/// as a number is reading — the attribute's own type is an integer. Turning `B` into `2` is
/// **arithmetic on a bijective base-26 numeral**, which is a computation the file never performed
/// and a value it never contains. The asymmetry is the file's, not this reader's, and carrying it
/// through is what keeps the locator a quotation rather than a derivation.
///
/// Concatenating [`Self::column`] and [`Self::row`] reproduces the `r` attribute exactly, so
/// nothing is lost by splitting it and there is no second place for the address to live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XlsxLocator {
    /// The package part this cell was read from, e.g. `xl/worksheets/sheet1.xml`.
    ///
    /// The package's own name for it, resolved through `xl/_rels/workbook.xml.rels` — not a path
    /// on disk, and not `sheet{n}.xml` guessed from the sheet's position in `<sheets>`.
    pub part: String,
    /// The workbook's own name for the sheet, from `<sheet name="…">` in `xl/workbook.xml`.
    ///
    /// Verbatim, with XML entities resolved — a `&amp;` dropped from a sheet name would be a
    /// *wrong address* rather than merely wrong text, which is the worse of the two failures.
    pub sheet: String,
    /// The row as the file numbers it: the digits of `<c r="B12">`, 1-based.
    ///
    /// **Read, not counted.** A workbook's rows are sparse — a sheet may jump from row 2 to row
    /// 12 with nothing in between — so a reader that incremented a counter would give the same
    /// cell a different address than the file gives it.
    pub row: u32,
    /// The column as the file letters it: the letters of `<c r="B12">`, e.g. `B`, `AA`.
    ///
    /// A `String` and not an index, for the reason on the type's own documentation: `2` is a
    /// number the workbook does not contain.
    pub column: String,
}

/// A text run's address inside an OOXML slide part (v2-S4).
///
/// # No slide number, and that is the whole design
///
/// A presentation is the first v2 format whose parts a person naturally counts — *"slide 12"* —
/// and this locator deliberately does not carry that number. `p:sldIdLst` states display **order**,
/// which changes when a deck is reordered and which a consumer would read as a page index; the
/// part name is what the package uses to identify a slide, and it does not move. A caller that
/// genuinely needs deck position can read `ppt/presentation.xml`, where it is a fact about the
/// presentation rather than a claim baked into every citation.
///
/// Nor is there a `p:sldSz`-derived width and height. A slide states a size in EMUs, but a size
/// is not a page, nothing here measured anything against it, and a `PageRecord` minted from it is
/// the invented pagination `docs/history/14-V2-SCOPE.md` §3 exists to refuse.
///
/// # Why the shape is a position and its id is not the address
///
/// `<p:cNvPr id="7" name="Title 1"/>` looks like the right thing to address a shape by — it is a
/// number the file wrote, and this locator was written that way first. **It was measured and
/// changed.** Across 18 real decks (329 slides, 3,335 shapes) the id is present every time and
/// **unique only most of the time**: 12 slides produced by an Open XML SDK generator reuse one
/// id, and PowerPoint opens them without complaint. An address that is not unique gives a
/// citation two answers, and refusing those files instead would reject decks that open everywhere
/// else — so the id is carried on [`OfficeSlideRunAttributes`], where being a non-unique label is
/// exactly what it is, and the address is a **position**.
///
/// All three components are therefore 1-based positions in the part's own document order: the
/// shape among the part's shapes, the paragraph within that shape's `<p:txBody>`, the run within
/// that paragraph. That is the spelling [`DocxLocator`] uses, for the reason it uses it — the file
/// states that order, and nothing was laid out to obtain it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PptxLocator {
    /// The package part this run was read from, e.g. `ppt/slides/slide1.xml`.
    ///
    /// Resolved through `ppt/_rels/presentation.xml.rels` — not `slide{n}.xml` guessed from the
    /// slide's position in `<p:sldIdLst>`.
    pub part: String,
    /// 1-based position of the `<p:sp>` within this part, in its own document order.
    ///
    /// **Shapes nest.** A `<p:sp>` inside a `<p:grpSp>` group is a shape and is counted as one,
    /// because it is one — groups appeared on essentially every slide of every real deck this was
    /// checked against, and a reader that skipped them would return a fraction of the deck.
    pub shape: u32,
    /// 1-based position of the `<a:p>` within that shape's `<p:txBody>`.
    pub paragraph: u32,
    /// 1-based position of the `<a:r>` within that paragraph.
    pub run: u32,
}

/// A paragraph's address inside an OpenDocument text part (v2-S5).
///
/// # Two fields, and the second one is the whole address
///
/// `part` is the package's name for the part the paragraph was read from — `content.xml`, which
/// the OpenDocument package specification fixes rather than leaving to a relationship. `paragraph`
/// is the 1-based position of the `<text:p>` or `<text:h>` element in that part's own document
/// order, which is the same spelling [`DocxLocator::paragraph`] uses and is obtained the same way:
/// by counting elements the file contains, never by laying anything out.
///
/// # Why the paragraph is the atom, and `<text:span>` is not
///
/// ODF splits a paragraph into `<text:span>` elements wherever formatting changes, and a paragraph
/// may contain **no span at all** — `<text:p>Plain text</text:p>` is ordinary. Addressing by span
/// would therefore leave the commonest case with no address to give, or force a span number the
/// file does not contain; and where spans do exist, their boundaries fall wherever a word was
/// bolded, so *"The **important** part."* would become three nodes and the sentence a reader
/// quotes would bind to none of them. The paragraph is the unit ODF itself treats as the block of
/// text, so it is the unit a citation lands on.
///
/// # No page, and this format is where that costs something to say
///
/// `content.xml` contains `<text:soft-page-break/>` — the position at which the *producing
/// application* broke a page, written from its own font metrics — and `styles.xml` contains a
/// `style:master-page` with an `fo:page-width`. Both look like the page a citation wants and
/// neither is one this engine measured, so neither is here and neither becomes a
/// [`PageRecord`]. `deny_unknown_fields` is what stops one arriving later as a third field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OdtLocator {
    /// The package part this paragraph was read from: `content.xml`.
    ///
    /// The package's own name for it, verbatim. Unlike OOXML's `r:id` indirection this name is
    /// fixed by the OpenDocument package specification, and this reader still checks that
    /// `META-INF/manifest.xml` declares it rather than reading a part the package does not list.
    pub part: String,
    /// 1-based position of the `<text:p>` or `<text:h>` in the part's own document order.
    ///
    /// **Counted over both elements together**, because both are blocks of text and a consumer
    /// citing "the third paragraph" is not distinguishing them — which of the two it was is on
    /// [`OfficeParagraphAttributes::block`] instead.
    ///
    /// **The count advances through regions this reader does not read**: a footnote body, a
    /// comment, a tracked-changes record. The number promises a position in the file, so it has to
    /// be the position a consumer counting elements in `content.xml` would find, not a position in
    /// the subset this slice kept.
    pub paragraph: u32,
}

/// A cell's address inside an OpenDocument **spreadsheet** part (v2-S6).
///
/// # The address the file does not write
///
/// [`XlsxLocator`] carries a row *read* and a column *lettered* because SpreadsheetML writes both
/// into one attribute — `<c r="B12">` — and its own documentation says so: counting would give a
/// sparse sheet's cell a different address than the file gives it.
///
/// **OpenDocument writes neither.** There is no `r`, no row number and no column letter anywhere
/// in a `.ods`; a cell's position is where it sits among its siblings, and a run of identical
/// cells is compressed into one element carrying `table:number-columns-repeated="n"`. So the
/// question is not "read or count" — it is *what the file states position with*, and the answer is
/// document order plus those repeat counts. Honouring them is reading. Ignoring them would put a
/// cell at the wrong column the moment a producer compressed a gap, which every real producer does
/// on every row.
///
/// Inventing `B` for column 2 would be the other failure and the worse one: the letters are a
/// spreadsheet application's convention for displaying an index, and this document contains no
/// such string. `docs/01-CONTRACT.md` §5.2 — an address the file does not contain is not an
/// address.
///
/// # No page, and no print layout either
///
/// A `.ods` states paper size in a `<style:page-layout>` and may carry `<text:soft-page-break/>`
/// inside a cell's own paragraph. Both are the producing application's print arithmetic — the
/// thing `docs/history/14-V2-SCOPE.md` §3 forbids addressing — so neither is here, `pages` is `[]`, and
/// `deny_unknown_fields` is what stops a print page arriving later as a fifth field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OdsLocator {
    /// The package part this cell was read from: `content.xml`.
    ///
    /// Fixed by the OpenDocument package specification, and still checked against
    /// `META-INF/manifest.xml` rather than reached for directly.
    pub part: String,
    /// The document's own name for the table, from `<table:table table:name="…">`.
    ///
    /// Verbatim, with XML entities resolved — the same rule [`XlsxLocator::sheet`] states, for the
    /// same reason: a `&amp;` dropped from a table name is a *wrong address*, not merely wrong
    /// text. **A package whose tables do not have distinct names is refused**, because an address
    /// that resolves two ways gives a citation two answers (v2-S4's finding, in ODF's spelling).
    pub table: String,
    /// 1-based position of the row within its table, counting `table:number-rows-repeated`.
    ///
    /// A position, not a number the file wrote — see the type's own documentation for why ODF
    /// leaves no third option. Rows nested inside `<table:table-header-rows>` and
    /// `<table:table-row-group>` count where they sit, because those elements group rows without
    /// moving them.
    pub row: u32,
    /// 1-based position of the cell within its row, counting `table:number-columns-repeated`.
    ///
    /// **A number and not letters**, because the file contains no letters. `table:covered-table-cell`
    /// — the placeholder a merge leaves behind — advances this the same as a cell does, since it
    /// occupies the position whether or not it displays.
    pub column: u32,
}

/// A block's address inside an OpenDocument **presentation** part (v2-S7).
///
/// # The page that was free, and is still not here
///
/// v2-S4 argued that a PowerPoint slide looks like a page and is a **part**, so [`PptxLocator`]
/// carries no slide number. A `<draw:page>` is one step past that: it is discrete, it is listed in
/// document order, it carries a `draw:name`, and a person counts them out loud. Put a master
/// page's `fo:page-width` beside it and a [`PageRecord`] needs **no arithmetic at all** — the
/// first time in this engine's history that has been true.
///
/// It is refused anyway, and the reason is what `docs/06-STEAL-REFUSE.md` L30 says about the
/// LibreOffice bridge rather than a paraphrase of it: *"It invents pagination."* A draw page is a
/// part of the presentation's **structure**. It is not a page this engine measured, and a number
/// on the wire under the name `page` is one a consumer would resolve against a rendering. So
/// `pages` stays `[]`, [`NativeLocator::is_paginated`] stays false for this variant, and the field
/// below is named for the element ODF writes rather than for the thing it resembles.
///
/// # Why the address is a position four times over
///
/// Every field is a count of elements the part lists, in the part's own document order — the
/// spelling [`OdtLocator::paragraph`] and [`OdsLocator::row`] use, obtained the same way.
///
/// **The names the file writes are on the attributes instead, and that is v2-S4's finding
/// applied.** `<draw:page draw:name="…">` and `<draw:frame draw:name="…">` are both *optional* in
/// OpenDocument, and no corpus of real `.odp` files was available to measure whether producers
/// write them uniquely. S4 measured the OOXML counterpart — `<p:cNvPr id>` is present every time
/// and unique only most of the time — and moved it off the address for that reason. An unmeasured
/// name gets the same treatment rather than the benefit of the doubt: it is a label on
/// [`OfficeOdfShapeAttributes`], where being a label is exactly what it is.
///
/// `deny_unknown_fields` is what stops a `page`, a `slide_number` or a `bbox` arriving later as a
/// fifth field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OdpLocator {
    /// The package part this block was read from: `content.xml`.
    ///
    /// Fixed by the OpenDocument package specification — a presentation keeps every draw page in
    /// **one** part, unlike OOXML's `ppt/slides/slide{n}.xml` — and still checked against
    /// `META-INF/manifest.xml` rather than reached for directly.
    pub part: String,
    /// 1-based position of the `<draw:page>` in the part's own document order.
    ///
    /// **Named for the element, not for what it resembles.** This is the count of `<draw:page>`
    /// elements the part lists before this one, plus one. It is not a page number, nothing was
    /// laid out to obtain it, and no [`PageRecord`] is minted from it — see the type's own
    /// documentation for why that refusal costs something in this format and is made anyway.
    pub draw_page: u32,
    /// 1-based position of the shape within that draw page, in document order.
    ///
    /// **The shapes counted are the ones this slice names**: `<draw:frame>` and
    /// `<draw:custom-shape>`, which are what a presentation writes for a text-bearing shape. A
    /// drawing element outside that set — a `<draw:rect>`, a `<draw:connector>` — does not move
    /// this count and does not become a node; text inside one is **declared** rather than dropped.
    /// Naming the set is what makes the position reproducible: a consumer counting those two
    /// elements in the draw page arrives at the same number.
    ///
    /// **Shapes nest**, and a nested one is counted where it sits. A `<draw:frame>` inside a
    /// `<draw:g>` group is a shape, for v2-S4's measured reason — groups appeared on essentially
    /// every slide of every real deck, and a reader that skipped them would return a fraction of
    /// the presentation.
    pub shape: u32,
    /// 1-based position of the `<text:p>` or `<text:h>` within that shape.
    ///
    /// Counted over both elements together, for [`OdtLocator::paragraph`]'s reason, and **the
    /// count advances through what this reader does not read** — a second `<draw:text-box>`
    /// rendition of the same shape, a speaker-notes body — because the number promises a position
    /// in the file rather than a position in the subset this slice kept.
    ///
    /// The paragraph is the atom, not the `<text:span>` and not the whole draw page. A page as one
    /// node would leave a quotation of a title bound to the entire slide; a span would split *"The
    /// **important** part."* into three nodes and bind the sentence to none of them — the argument
    /// [`OdtLocator`] makes at length, unchanged by the change of vocabulary.
    pub paragraph: u32,
}

/// A paragraph's address inside a Rich Text Format stream (v2-S8).
///
/// # One field, because the format has one stream
///
/// Every v2 format before this one is a **package**, and every one of their locators opens with a
/// part name — `word/document.xml`, `content.xml`, `xl/worksheets/sheet1.xml`. An `.rtf` is a
/// single sequence of brace groups. It has no parts, no manifest, no relationship table and no
/// name for itself anywhere in its own bytes.
///
/// So there is no `part` here, and filling one in was the tempting move: a constant would have let
/// `check_structure`'s part-id bijection run unchanged. It would also have been a string the
/// document does not contain, which is `docs/history/14-V2-SCOPE.md` §3's *"absent, not invented"* in a
/// smaller place than the page-sized box that obligation is usually about. The invariant grew a
/// third case instead — see [`NativeLocator::names_a_part`].
///
/// # The paragraph is a position, and here is what to count
///
/// 1-based, in the order the byte stream delimits paragraphs. A paragraph ends at `\par`, at
/// `\sect`, at `\cell` or `\row` inside a table, or at the end of the document — see
/// [`RtfParagraphBreak`], which records which of those it was rather than making a consumer guess.
///
/// **The count advances through destinations this reader does not read.** A `\par` inside a
/// `{\footer …}` or a `{\footnote …}` moves it, exactly as [`OdtLocator::paragraph`] advances
/// through a footnote body, and for the same reason: the number promises a position in the file,
/// so it has to be the position a consumer counting paragraph breaks in the bytes would find,
/// not a position in the subset this slice kept.
///
/// # No page, and RTF says the word out loud
///
/// `\page` is a page break, `\paperw` and `\paperh` are a paper size, and `\sect` carries a
/// whole section's page setup. All of them are the producing application's print arithmetic —
/// plainer than an ODT's `<text:soft-page-break/>` and no more measured by this engine — so none
/// of them reaches this variant and none becomes a [`PageRecord`]. `deny_unknown_fields` is what
/// stops one arriving later as a second field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RtfLocator {
    /// 1-based position of the paragraph in the stream's own order.
    pub paragraph: u32,
}

/// A block's address inside an EPUB spine document (v2-S9).
///
/// # The part comes from the spine, and that is the whole of the reading-order decision
///
/// An EPUB is a ZIP, and the shortcut it invites is the one v2-S3 walked into for a workbook:
/// take the XHTML entries in the order the central directory lists them, or sorted by name. Both
/// are wrong in **ordinary** files. A publication's reading order is stated in the package
/// document's `<spine>`, as a list of `<itemref idref="…">` resolved through the `<manifest>` —
/// and nothing requires a producer's file names to sort that way, or the archive to store them in
/// it. Every such shortcut attaches the **right content to the wrong position**, which
/// `docs/01-CONTRACT.md` §5.2 calls strictly worse than an absent address. `opc.rs` states the
/// same rule for `r:id`, and this is that rule in EPUB's spelling.
///
/// [`Self::part`] is therefore the spine item's package path, resolved from the manifest `href`
/// **relative to the package document's own directory** — an `href="chap01.xhtml"` in
/// `OEBPS/content.opf` is the entry `OEBPS/chap01.xhtml`, and a reader that took the `href`
/// verbatim would miss every EPUB that keeps its content in a subdirectory, which is nearly all
/// of them.
///
/// # The block is a position, and `<span>` is not the atom
///
/// [`Self::block`] is 1-based within its spine document, counting the XHTML flow elements this
/// reader names as blocks — the argument [`OdtLocator::paragraph`] makes about `<text:span>`,
/// unchanged by the change of vocabulary: a `<span>` boundary falls wherever a word was styled,
/// so *"The **important** part."* would become three nodes and the sentence a reader quotes would
/// bind to none of them.
///
/// **The count advances through blocks this reader does not read.** A navigation document's `<ol>`
/// and `<li>` take positions here even though their text never reaches a node, because the number
/// promises a position in the document rather than a position in the subset this slice kept. A
/// `<script>`, a `<style>` and the `<nav>` element itself take **no** position — none of them is a
/// block — so a gap in this sequence is a passed-over *block*, not a passed-over region.
///
/// # No page, and this is the format that could have had one
///
/// See [`NativeLocator::Epub`]. `deny_unknown_fields` is what stops a `page` arriving later as a
/// third field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpubLocator {
    /// The package path of the spine document this block was read from, e.g. `OEBPS/chap01.xhtml`.
    ///
    /// The archive's own name for the entry, resolved from the manifest rather than guessed — not
    /// a path on disk, not the `href` verbatim, and not a name taken from the ZIP's ordering.
    pub part: String,
    /// 1-based position of the block within that spine document, in its own document order.
    pub block: u32,
}

/// A PDF node's native address: page plus character origin plus advance.
///
/// The origin is the fingerprint-critical primitive. Two independent PDF stacks agree on
/// character origin to 0.001 pt while disagreeing on box height by 6.174 pt — which is the whole
/// reason origins are identity and boxes are inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfLocator {
    /// 1-based page number, as the document numbers its own pages.
    pub page: u32,
    /// Baseline origin x, integer centipoints, in the declared coordinate system.
    pub origin_x: i64,
    /// Baseline origin y, integer centipoints, in the declared coordinate system.
    pub origin_y: i64,
    /// Advance width in centipoints, or **absent** when the document carries no widths.
    ///
    /// `None` is not zero. Absent means the reader does not know, and says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance: Option<i64>,
}

/// A PDF object's address: which page carries it, which object it is, and the box it declares.
///
/// The object number is the document's own, so a consumer can find the dictionary this node was
/// read from without a mapping table — the same reason a text run carries its origin rather than
/// an index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfObjectLocator {
    /// 1-based page number, as the document numbers its own pages.
    pub page: u32,
    /// The object number this node was read from.
    pub object: u32,
    /// Its generation number. Usually 0; carried because an object id is both halves.
    pub generation: u32,
    /// The rectangle the object declares, or a typed reason there is none.
    pub rect: AnnotationRect,
}

/// The `/Rect` an annotation or widget declares, or why there is none.
///
/// **Deliberately not [`crate::derivation::GeometryPresence`].** That type means *measured ink*,
/// and this rectangle is nothing of the kind: it is a number the author wrote into the dictionary,
/// saying where the annotation sits. Reporting it as `Measured` would claim a provenance it does
/// not have, and mixing declared rectangles into the same field as measured ones — with nothing on
/// the wire to tell them apart — is the flattening this project refuses everywhere else.
///
/// **Deliberately not `Option<QRect>`** either, for the reason `GeometryPresence` is not: `None`
/// would collapse "the dictionary has no `/Rect`" and "it has one this profile cannot express"
/// into one answer, and they are different facts about the document. Neither becomes a
/// page-sized box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "value")]
#[non_exhaustive]
pub enum AnnotationRect {
    /// The rectangle the document declared, quantized into the artifact's coordinate system.
    Declared(QRect),
    /// The dictionary carries no `/Rect`.
    NotDeclared,
    /// It carries one that does not describe a rectangle this profile can express.
    ///
    /// Non-numeric operands, the wrong arity, or coordinates outside the quantizable range. The
    /// annotation is still emitted — refusing the node over its box would delete content because
    /// its geometry was bad, which is the wrong trade.
    Malformed,
}

impl AnnotationRect {
    /// The rectangle, if one was declared.
    pub fn declared(self) -> Option<QRect> {
        match self {
            Self::Declared(r) => Some(r),
            _ => None,
        }
    }
}

/// A painted image's address (v1-S6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfImageLocator {
    /// 1-based page number, as the document numbers its own pages.
    pub page: u32,
    /// The object number of the image XObject that was drawn.
    ///
    /// **The same object may appear on many pages and many times on one page.** Each `Do` is its
    /// own node with its own rectangle, and they share this number — which is the fact a consumer
    /// needs to know that one picture was placed five times rather than five pictures placed once.
    pub object: u32,
    /// Its generation number. Usually 0; carried because an object id is both halves.
    pub generation: u32,
    /// The area on the page this placement covered.
    pub rect: PaintedRect,
}

/// The area a `Do` painted an image into, or a typed reason there is no rectangle (v1-S6).
///
/// # Not [`crate::derivation::GeometryPresence`], and not [`AnnotationRect`]
///
/// Three kinds of box now exist in this record and the whole point is that they stay apart.
/// `GeometryPresence::Measured` means **ink measured from font metrics**. [`AnnotationRect`]
/// means **a rectangle the author declared** in a dictionary. This means **the page's own
/// transformation matrix, applied to the unit square** every PDF image is defined on
/// (32000-1 §8.9.5.2) — computed by this engine from evidence the content stream supplies, which
/// is a third provenance and gets a third type.
///
/// **It is never the bitmap's pixel dimensions.** A 4000×3000 photograph scaled into a 2cm
/// thumbnail is 2cm of page, and reporting its pixel count as a box would be the pdf-inspector
/// defect (`height` and the type size being one variable) wearing different clothes. The pixel
/// dimensions are kept, separately and clearly labelled, on [`ImageAttributes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "value")]
#[non_exhaustive]
pub enum PaintedRect {
    /// The matrix mapped the unit square onto this axis-aligned rectangle.
    Painted(QRect),
    /// The matrix rotates or skews, so the placement is not an axis-aligned rectangle.
    ///
    /// A rotated image really does cover a parallelogram, and its bounding box is a **larger**
    /// area than the image occupies — reporting one would claim page area the picture does not
    /// cover. The node is still emitted: refusing an element because its geometry is oblique
    /// would delete evidence over a box, which is the wrong trade in both directions.
    ///
    /// This is deliberately *not* what the ruled-table detector does with an oblique rectangle —
    /// that one drops the rectangle, because a rule line it cannot place is not evidence of a
    /// grid. An image it cannot place is still an image that was drawn.
    NotAxisAligned,
    /// The matrix produced coordinates this profile cannot express.
    ///
    /// Non-finite, or outside the quantizable integer range. As above, the node survives its box.
    Malformed,
}

impl PaintedRect {
    /// The rectangle, if the placement was axis-aligned and expressible.
    pub fn painted(self) -> Option<QRect> {
        match self {
            Self::Painted(r) => Some(r),
            _ => None,
        }
    }
}

/// A structural address, where the node kind defines one.
///
/// # Three answers, and they are not the same answer (v1-S3)
///
/// v0 through v1-S2 emitted marked-content ids only: the `BDC` operand, captured verbatim, with
/// no idea what it referred to. v1-S3 reads the document's `/StructTreeRoot`, so a node can now
/// carry the address the **author** gave it. The variants are the three distinct things that can
/// be true, and collapsing any two of them would lose information a consumer needs:
///
/// | Variant | What happened |
/// | --- | --- |
/// | [`Self::PdfTagged`] | the tree cites this `(page, mcid)`, so the author placed this text here |
/// | [`Self::PdfMcid`] | the content stream gave an id and **the tree did not cite it** |
/// | [`Self::PdfArtifact`] | the content stream marked this as page furniture, outside the tree |
/// | absent | the content stream marked nothing here at all |
///
/// A `PdfMcid` on a tagged document is a real gap and is counted as one; it is not the same as a
/// document with no tree, and neither is the same as text the author deliberately excluded from
/// its structure. Nothing here is inferred: no role is guessed from a font size or a text prefix,
/// which is the defect the parity checklist records as P14.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum StructuralLocator {
    /// A PDF marked-content id, captured verbatim from `BDC`, **which no structure element
    /// claims**.
    ///
    /// Either the document has no structure tree, or it has one and this id is not in it. The
    /// assurance block's limitations say which; this variant alone does not, because a node
    /// cannot see the document.
    PdfMcid(i64),
    /// The address the document's own structure tree gives this content.
    PdfTagged(PdfTaggedLocator),
    /// Content the page marked as an **artifact**: running heads, folios, rules, decoration.
    ///
    /// Page furniture the author deliberately kept out of the structure tree (PDF 32000-1
    /// §14.8.2.2). It is flagged, **never dropped** — the parity checklist's O21/O22 are about
    /// exactly this: a reader that deletes running heads has silently edited the document, and a
    /// consumer cannot tell the difference between text that was not there and text that was
    /// removed on its behalf.
    PdfArtifact(PdfArtifactLocator),
}

/// A node's address in the document's tagged-structure tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfTaggedLocator {
    /// The marked-content id the tree cited to reach this content.
    pub mcid: i64,
    /// Structure types from the root down, **exactly as the document wrote them** in `/S`.
    ///
    /// Never laundered. A document using a custom type gets its custom name here, so a consumer
    /// reading this path sees what the file says rather than what this engine made of it.
    pub role_path: Vec<String>,
    /// The same path after the document's own `/RoleMap` is applied.
    ///
    /// **Present only when the map actually changed something**, so its presence is the signal
    /// that a custom type was in play. A `/RoleMap` is the document telling us what its custom
    /// types mean; applying it is reading the file, and guessing without one would not be
    /// (`docs/history/09-V1-MILESTONES.md` S3, decision 8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_role_path: Option<Vec<String>>,
    /// The innermost element's `/ID`, when it declares one.
    ///
    /// Absent means the document supplied none. Never minted here — an identifier this engine
    /// invented would not address anything in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
}

/// A node the page marked as an artifact rather than as content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfArtifactLocator {
    /// The marked-content id, when the artifact sequence carried one.
    ///
    /// Usually absent: an artifact is by definition not a structure content item, so it has no
    /// reason to be numbered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcid: Option<i64>,
}

/// What a node is.
///
/// # Why this stayed at one variant for four slices, and grew at the fifth
///
/// Adding a kind is easy and usually wrong. v1-S1 refused `TableCell` because a cell's text is
/// already a run, so a cell node would put one string in two places to drift. v1-S3 refused
/// `Paragraph` and `Heading` because the structure tree's role path already says `P` — a kind
/// would restate it. The standing rule that came out of both: **do not add a kind for a fact some
/// existing node already carries.**
///
/// v1-S4 is the case that rule was waiting for. A form field's value and an annotation's comment
/// are **not** in any content stream: nothing draws them, no run holds them, and without a node
/// of their own they are simply absent from the record. They are a different fact, so they get
/// different kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeKind {
    /// One run of text, as the source format delimits runs.
    ///
    /// **Not a line and not a paragraph.** For a PDF that is exactly what one show-text operator
    /// drew; for an OOXML document it is one `<w:r>`. Neither is grouped, because grouping would
    /// be a claim about layout that no code here makes — and for a page-less format there is no
    /// layout to make it from (v2-S2).
    TextRun,
    /// An interactive form field's value, read from its dictionary (v1-S4).
    ///
    /// **Never a text run.** A field's `/V` lives in the AcroForm tree, not in the page's content
    /// stream, and copying it into a run would put text on the page that the page does not draw.
    /// The reverse is also true and matters: when a form has been *flattened* at save time, its
    /// values really were painted onto the page, and those glyphs are runs — this kind is about
    /// where this engine reads a value from, not about denying that flattened ink is ink.
    FormField,
    /// An annotation's own text, read from its dictionary (v1-S4).
    ///
    /// A comment, a sticky note, a free-text callout. Markup a reader added *over* the document
    /// rather than content the document draws, which is exactly why it is distinguishable: a
    /// consumer that cannot tell a reviewer's note from the page's own words cannot cite either
    /// one safely.
    Annotation,
    /// An image the page painted with `Do` (v1-S6).
    ///
    /// **A placement, not a picture.** The node says *an image XObject was drawn here, and these
    /// bytes are it* — page, object number, the rectangle the `Do` painted into, and a digest of
    /// the stream as stored. Nothing is decoded, nothing is recognised, and the node carries no
    /// description, caption or alt text: text derived from pixels is OCR, which is out of v1
    /// entirely, and a model's account of a picture is not evidence (checklist O20).
    ///
    /// Passing the rule "do not add a kind for a fact some existing node already carries": no run
    /// holds an image, no table cell does, and the classifier's `embedded-images` reason counts
    /// *declared resources* on a page without saying where any of them was drawn or which bytes
    /// it was. Without a kind of its own an image is simply absent from the record.
    Image,
}

impl NodeKind {
    /// The wire spelling, which is also the grounding artifact's `kind`.
    ///
    /// Constrained to `^[a-z0-9][a-z0-9_-]*$` by `ethos.grounding.v1`; a test pins that the
    /// spelling still matches after any rename.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextRun => "text_run",
            Self::FormField => "form_field",
            Self::Annotation => "annotation",
            Self::Image => "image",
        }
    }
}

/// Facts about a node that only its kind has.
///
/// # Why a union rather than one struct with optional fields
///
/// A text run has character codes and a font size; a form field has a field type and a value; an
/// annotation has a subtype and flags. Putting all of them on one struct would mean every node
/// carrying mostly-null fields, and `deny_unknown_fields` would police the shape while saying
/// nothing about which fields are *meaningful* for a given kind.
///
/// Externally tagged, matching [`NativeLocator`] and [`StructuralLocator`], so a new kind is
/// additive on the wire and an unrecognised one fails closed.
///
/// The tag duplicates [`Node::kind`], which is a drift risk — so it is a **checked** one:
/// [`Self::kind`] returns the kind these attributes belong to, and a contract test asserts every
/// node agrees with its own attributes. Redundancy that is tested is a cross-check; redundancy
/// that is not is two places for the truth to live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum NodeAttributes {
    /// A glyph run's facts.
    TextRun(TextRunAttributes),
    /// A form field's facts (v1-S4).
    FormField(FormFieldAttributes),
    /// An annotation's facts (v1-S4).
    Annotation(AnnotationAttributes),
    /// A painted image's facts (v1-S6).
    Image(ImageAttributes),
    /// An office run's facts (v2-S2).
    ///
    /// **A variant rather than [`Self::TextRun`] with the PDF fields blanked.** A `<w:r>` has no
    /// character codes, no font resource name and no font size this reader read — filling
    /// `font_size: 0` and `font_id: ""` would put three claims on the wire that the document
    /// never made, which is the fabrication `01-CONTRACT.md` §5.2 forbids. The union discipline
    /// §5.1 sets for locators is the same one here: a new *kind* of fact is a new variant.
    OfficeRun(OfficeRunAttributes),
    /// A spreadsheet cell's facts (v2-S3).
    ///
    /// **A third variant rather than [`Self::OfficeRun`], because a `<c>` is not a `<w:r>`.** A
    /// cell has no `xml:space="preserve"` to record and a run has no value type and no formula,
    /// so either struct filled with the other's fields would be claims the file never made — the
    /// same argument that made `OfficeRun` a variant rather than a blanked [`Self::TextRun`].
    OfficeCell(OfficeCellAttributes),
    /// A slide run's facts (v2-S4).
    ///
    /// **A fourth variant rather than [`Self::OfficeRun`] with a borrowed field.** DrawingML does
    /// not use `xml:space` — checked against three real decks, zero occurrences — so
    /// `space_preserved: false` would be a claim about a mechanism the format does not have,
    /// which is the fabrication that made `OfficeRun` a variant in the first place.
    OfficeSlideRun(OfficeSlideRunAttributes),
    /// An OpenDocument paragraph's facts (v2-S5).
    ///
    /// **A fifth variant, and the argument has not changed.** ODF does not use
    /// `xml:space="preserve"` — whitespace is carried by `<text:s text:c="…">` instead, which this
    /// reader reads as the count the file states — so [`Self::OfficeRun`]'s one field would be a
    /// claim about a mechanism this format does not have, exactly as it would have been for a
    /// slide run.
    OfficeParagraph(OfficeParagraphAttributes),
    /// An OpenDocument spreadsheet cell's facts (v2-S6).
    ///
    /// **A sixth variant, and the first one that is a second *cell*.** The obvious move is
    /// [`Self::OfficeCell`], and it does not survive contact with the two formats' vocabularies:
    /// its [`CellValueType`] is ECMA-376's list, in which `SharedString` names a workbook-only
    /// table and `percentage` and `currency` do not exist at all. Reusing it would leave one
    /// variant permanently unreachable and two ODF types unsayable — a shape that either blanks
    /// fields or invents a mapping, and `docs/history/14-V2-SCOPE.md` §8 refuses both.
    OfficeOdfCell(OfficeOdfCellAttributes),
    /// An OpenDocument presentation shape's facts (v2-S7).
    ///
    /// **A seventh variant, and the first that carries two names.** [`Self::OfficeParagraph`] has
    /// the block kind and nothing else, which is all an ODT paragraph states about itself; a
    /// presentation's block sits inside a shape on a draw page, and both of those carry an
    /// optional `draw:name` the file writes down. Reusing `OfficeParagraph` would drop them, and
    /// they are the two strings a person reading a citation recognises — the same argument that
    /// put `shape_name` on [`Self::OfficeSlideRun`] rather than leaving it in the package.
    OfficeOdfShape(OfficeOdfShapeAttributes),
    /// A Rich Text Format paragraph's facts (v2-S8).
    ///
    /// **An eighth variant for one field, and the field is the reason.** RTF ends a paragraph with
    /// four different control words, and which one it was is not cosmetic: `\cell` and `\row`
    /// say the text sat in a table, which is a fact about the document that no other field here
    /// carries and that this slice deliberately does **not** turn into a
    /// [`crate::tables::TableRecord`]. [`Self::OfficeParagraph`] carries ODF's block kind, which
    /// RTF has no counterpart for; reusing it would mean answering "was this a `<text:h>`" about a
    /// format with no such distinction.
    RtfParagraph(RtfParagraphAttributes),
    /// An EPUB block's facts (v2-S9).
    ///
    /// **A ninth variant, and neither of its two fields fits an existing one.**
    /// [`Self::OfficeParagraph`] carries ODF's two-block distinction, which XHTML does not have —
    /// it has twenty-odd flow elements, and answering "was this a `<text:h>`" about one of them
    /// would be a mapping this engine would then have to defend. [`Self::RtfParagraph`] carries
    /// the control word that ended a paragraph, which an XHTML document does not state at all.
    EpubBlock(EpubBlockAttributes),
}

impl NodeAttributes {
    /// The node kind these attributes describe.
    pub fn kind(&self) -> NodeKind {
        match self {
            Self::TextRun(_) => NodeKind::TextRun,
            Self::FormField(_) => NodeKind::FormField,
            Self::Annotation(_) => NodeKind::Annotation,
            Self::Image(_) => NodeKind::Image,
            // Deliberately the same kind as a PDF run. A `<w:r>` and a show-text run are the
            // same *thing* — one run of text — differing in how they are addressed, and the
            // locator is what says which. A second kind would make every consumer handle two
            // names for one concept at no gain.
            Self::OfficeRun(_) => NodeKind::TextRun,
            // **And so is a cell**, for the standing rule rather than in spite of it. v1-S1
            // refused a `TableCell` kind because a PDF cell's text was already in runs; here the
            // cell *is* the atom, so the question is only whether "this text is a cell" is a fact
            // no existing node carries — and it is not. [`XlsxLocator`] says sheet, row and
            // column, which is cell-ness spelled out; a kind would restate the locator, which is
            // exactly why v1-S3 refused `Paragraph` when the role path already said `P`.
            Self::OfficeCell(_) => NodeKind::TextRun,
            // And so is a slide run, for the standing rule: `PptxLocator` already says which
            // shape and which paragraph, so a `Slide` kind would restate the address.
            Self::OfficeSlideRun(_) => NodeKind::TextRun,
            // And so is an ODF paragraph. v1-S3 refused a `Paragraph` kind when the role path
            // already said `P`, and the same answer holds here for a stronger reason: whether
            // this block was a `<text:p>` or a `<text:h>` is a *fact about the element*, which
            // belongs on the attributes, and `OdtLocator` already says it is a paragraph-shaped
            // address. A kind would restate one and misplace the other.
            Self::OfficeParagraph(_) => NodeKind::TextRun,
            // And so is an ODF cell, for the reason `OfficeCell` is: `OdsLocator` says table, row
            // and column, which is cell-ness already spelled out in the address.
            Self::OfficeOdfCell(_) => NodeKind::TextRun,
            // And so is a presentation's block. A `Slide` kind is the one this format would tempt
            // somebody into, and it would be the restatement v1-S3 refused twice over: the address
            // already says which draw page and which shape, and whether the block was a `<text:p>`
            // or a `<text:h>` is on the attributes where a fact about an element belongs.
            Self::OfficeOdfShape(_) => NodeKind::TextRun,
            // And so is an RTF paragraph, for the reason the ODF one is: the locator already says
            // this is a paragraph-shaped address, and a `Paragraph` kind would restate it.
            Self::RtfParagraph(_) => NodeKind::TextRun,
            // And so is an EPUB block. A `Heading` kind is the one this format would tempt
            // somebody into, and `element` already carries the file's own word for it — v1-S3
            // refused a `Paragraph` kind when the role path already said `P`, for that reason.
            Self::EpubBlock(_) => NodeKind::TextRun,
        }
    }
}

/// What a painted image XObject declares about itself (v1-S6).
///
/// **Everything here is read from the stream dictionary or computed over its bytes.** Nothing is
/// decoded, nothing is recognised, and there is no field for a description — a caption produced by
/// a model is not evidence (checklist O20), and text read out of pixels is OCR, which v1 does not
/// do at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageAttributes {
    /// `sha256:` over the stream's bytes **exactly as the file stores them**, still encoded.
    ///
    /// # Why the encoded bytes, and why a digest rather than the bytes
    ///
    /// Encoded, because those are the bytes that are actually in the document. Decoding first
    /// would make the digest depend on this engine's decoder — two readers with different
    /// inflate implementations would disagree about what the same file contains, which is the
    /// property a fingerprint exists to deny.
    ///
    /// A digest rather than a payload, because an artifact is a record *about* a document, not a
    /// second copy of it. Inlining megabytes of JPEG would make the canonical JSON unreadable, and
    /// would put the picture in two places to drift. This identifies the bytes and lets a consumer
    /// go and get them; it does not replace them.
    pub stream_sha256: Sha256Hex,
    /// Length in bytes of the stream this digest covers.
    ///
    /// Beside the digest so a reader can tell a zero-length stream from an absent one without
    /// fetching anything: `sha256:e3b0c442…` is the digest of nothing, and a consumer should not
    /// have to know that constant to notice.
    pub stream_bytes: u64,
    /// The `/Filter` chain, outermost first, exactly as the document spells it.
    ///
    /// Empty means the stream declares no filter — raw samples. Kept verbatim rather than
    /// normalised, because the filter names are what say whether [`Self::stream_sha256`] covers a
    /// file some other tool can open.
    pub filters: Vec<String>,
    /// Whether the stored bytes are a standalone image file, and of what type.
    pub media_type: ImageMediaType,
    /// `/Width` in samples, as the dictionary declares it.
    ///
    /// **Not a box, and in different units from every rectangle in this record.** Named
    /// `pixel_width` rather than `width` for exactly that reason: the area this image covers on
    /// the page is [`PdfImageLocator::rect`], and the two are unrelated numbers. Absent when the
    /// dictionary omits it or gives a value this profile cannot read — never defaulted to zero,
    /// which would read as a degenerate image rather than an unread field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_width: Option<u32>,
    /// `/Height` in samples. See [`Self::pixel_width`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_height: Option<u32>,
    /// `/ImageMask true`: the stream is a 1-bit stencil, painting the current colour through a
    /// mask rather than carrying image data of its own.
    ///
    /// Reported because a stencil mask is a different thing from a picture, and a consumer
    /// counting "images on this page" should be able to tell them apart.
    pub image_mask: bool,
}

/// Whether an image stream's stored bytes are a file in their own right (v1-S6).
///
/// # The distinction, and why it is not an `Option<String>`
///
/// A `/DCTDecode` stream **is** a JPEG: write those bytes to disk and an image viewer opens them.
/// A `/FlateDecode` stream is not a file at all — it is PDF-specific sample data whose meaning
/// depends on `/ColorSpace`, `/BitsPerComponent` and `/Decode` in the same dictionary. Calling the
/// second one `image/png` because it is compressed, or leaving both as `None`, would tell a
/// consumer the same thing about two genuinely different situations.
///
/// [`Self::stream_sha256`](ImageAttributes::stream_sha256) covers the stored bytes either way. This
/// says whether those bytes travel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "value")]
#[non_exhaustive]
pub enum ImageMediaType {
    /// The stored bytes are a standalone file of this media type.
    ///
    /// Only where the filter chain says so outright: `/DCTDecode` is `image/jpeg` and
    /// `/JPXDecode` is `image/jp2`. Nothing is sniffed from the payload — guessing a type from
    /// leading bytes is a decoder's job, and a wrong guess here would send a consumer to open
    /// something as a format it is not.
    Standalone(String),
    /// The bytes are PDF-encoded samples, not a file in any image format.
    ///
    /// The honest answer for `/FlateDecode`, `/LZWDecode`, `/RunLengthDecode`, `/CCITTFaxDecode`,
    /// `/JBIG2Decode` and unfiltered streams alike. Their filters are still named in
    /// [`ImageAttributes::filters`]; what this says is that saving the bytes to a `.png` would
    /// produce a file nothing can open.
    PdfEncodedSamples,
}

/// Something observed about a text run that a consumer must not learn about by accident (v1-S6).
///
/// # An observation. Never a filter, and never a verdict
///
/// This is checklist O21, and the rule is one sentence: **the run stays.** OpenDataLoader deletes
/// low-contrast text before returning a page, so the page it returns looks clean and the reader has
/// no way to know anything was removed. Removing evidence because it looks suspicious is how a
/// document that hides an instruction and a document that contains nothing become the same
/// artifact.
///
/// So a finding is data attached to a node that is still there, with its text, its origin and its
/// locator intact. It does not say the document is malicious, does not score it, and does not
/// change what any other stage does — `docs/07-VERIFY-BOUNDARY.md` applies here as everywhere:
/// this engine reports, it does not judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum TextFinding {
    /// The run was drawn in text rendering mode 3 or 7 — painted with no fill and no stroke.
    ///
    /// `Tr 3` is how a scanner's OCR layer is laid under a page image, which is ordinary and
    /// useful. It is also how text is hidden from a human reader while staying perfectly legible
    /// to anything that reads the text layer. **This profile does not decide which one it is
    /// looking at**; it reports that the mode was set, and leaves the two indistinguishable cases
    /// distinguishable by context a consumer has and this engine does not.
    InvisibleRenderMode,
    /// The run's origin lies outside the page's visible box.
    ///
    /// Measured against `/CropBox` where the page declares one and `/MediaBox` otherwise — the
    /// box a viewer actually shows — after `/Rotate`. Content outside it is in the file and is
    /// not on the page.
    OffPage,
}

impl TextFinding {
    /// The code this finding is counted under in `assurance.limitations`.
    ///
    /// One string, used in both places. The per-node flag and the document-level count are two
    /// views of one observation, and giving them two spellings is how they drift into looking
    /// like two different findings.
    pub fn as_code(self) -> &'static str {
        match self {
            Self::InvisibleRenderMode => crate::codes::INVISIBLE_RENDER_MODE_TEXT,
            Self::OffPage => crate::codes::OFF_PAGE_TEXT,
        }
    }

    /// Every finding this profile can report.
    pub const ALL: [Self; 2] = [Self::InvisibleRenderMode, Self::OffPage];
}

/// What an interactive form field declares about itself (v1-S4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormFieldAttributes {
    /// The **fully-qualified** field name: the `/T` of every ancestor, root first, joined by `.`.
    ///
    /// PDF's own convention (32000-1 §12.7.3.2), and the name a form's data is keyed by. Absent
    /// when no `/T` appears anywhere up the chain — some widgets genuinely have no name, and
    /// minting one would produce a key that addresses nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
    /// `/FT` as the document spells it — `Tx`, `Btn`, `Ch`, `Sig` — inherited from the nearest
    /// ancestor that declares one.
    ///
    /// Absent when no ancestor declares it. Not guessed from the value's shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
    /// The field's value, `/V`, in the shape the document wrote it.
    pub value: FieldValue,
    /// `/Ff` bits this profile recognises, named, in ascending bit order.
    ///
    /// The flags a consumer needs to read a value correctly: whether it is a password, a
    /// multi-select, a radio group. Unrecognised bits are **not** dropped — see
    /// [`Self::unrecognized_flag_bits`].
    pub flags: Vec<String>,
    /// The raw `/Ff` bits this profile has no name for.
    ///
    /// Kept rather than discarded: a flag nobody named is still something the document said, and
    /// silently dropping it would make an unread field indistinguishable from an unset one.
    ///
    /// `default` is load-bearing beside `skip_serializing_if`, not tidiness: without it the
    /// artifact serializes with the key omitted and then **fails to deserialize its own output**.
    /// A `Vec` is not an `Option`, which serde treats as optional on its own — the oracle caught
    /// this on the first real form it ran.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unrecognized_flag_bits: Vec<u32>,
}

/// A form field's `/V`, in the shape the document wrote it.
///
/// **Not normalised into a string.** A checkbox's `/Off` and a text field's `"Off"` are different
/// things, and a consumer that has to guess which one it is holding has been handed a worse
/// answer than the file contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
#[non_exhaustive]
pub enum FieldValue {
    /// A text string, decoded the same way page text is.
    Text(String),
    /// A name object — a checkbox or radio state such as `Yes` or `Off`.
    ///
    /// Carried as the token the file contains, **never converted to a boolean**: `/Off` and
    /// `/Yes` are the common spellings but not the only legal ones, and `true`/`false` would be
    /// this engine's reading rather than the document's text.
    Name(String),
    /// A choice field's selection, one entry per selected option.
    Choice(Vec<String>),
    /// An integer value.
    Integer(i64),
    /// The field declares no `/V`.
    ///
    /// A blank form, which is a real and common state. **Never an empty string**: "the author
    /// left this unfilled" and "the author filled this in with nothing" are different facts.
    Absent,
    /// `/V` is present in a shape this profile does not express.
    ///
    /// Declared rather than dropped, so an unread value is distinguishable from an unset one.
    Unsupported,
}

impl FieldValue {
    /// The value as text, for the node's `text` field. Empty for every non-textual state.
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) | Self::Name(s) => s.clone(),
            Self::Choice(v) => v.join("\n"),
            Self::Integer(i) => i.to_string(),
            Self::Absent | Self::Unsupported => String::new(),
        }
    }
}

/// What an annotation declares about itself (v1-S4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationAttributes {
    /// `/Subtype` as the document spells it: `Text`, `FreeText`, `Highlight`, `Link`, `Widget`, …
    ///
    /// Verbatim, including a subtype this profile has no special handling for. An unknown subtype
    /// is a node with a name on it, not a node that vanished.
    pub subtype: String,
    /// `/NM`, the annotation's own name, when it declares one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `/T`, conventionally the author of a markup annotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `/F` bits this profile recognises, named, in ascending bit order.
    ///
    /// **Data, not a filter.** `hidden` and `no_view` are reported and the node stays — deleting
    /// content because the document asked a viewer not to show it is an undeclared edit, and the
    /// parity checklist's O21 is exactly that: report, do not drop.
    pub flags: Vec<String>,
    /// The raw `/F` bits this profile has no name for. Kept for the same reason as `/Ff`'s, and
    /// `default` for the same reason: an omitted key must still deserialize.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unrecognized_flag_bits: Vec<u32>,
}

/// Format-specific facts about a node that do not fit the common fields.
///
/// Kept as a typed struct rather than an open map: an open map is a place for a future field to
/// arrive unreviewed, and `deny_unknown_fields` cannot police one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRunAttributes {
    /// The character codes that produced the text, in order.
    ///
    /// **Not 1:1 with `text`.** A ligature is one code and several scalars.
    pub char_codes: Vec<u32>,
    /// True when `text.chars().count() != char_codes.len()`. Declared, not reconciled.
    pub scalar_code_mismatch: bool,
    /// Characters this reader inserted, by index into `text`. Empty for verbatim text.
    pub synthesized: Vec<SynthesizedAt>,
    /// Font resource name, as the document names it.
    pub font_id: String,
    /// Font size in integer centipoints. **Never used as a box height.**
    pub font_size: i64,
    /// Which region of its page the reading-order cut placed this run in, 1-based, in reading
    /// order (D4-S2).
    ///
    /// # Where, never what
    ///
    /// A region is a position in a partition the rule measured from whitespace. It is **not** a
    /// paragraph, a heading, a section or a column name, and nothing downstream may read a role
    /// from it — that is `docs/06-STEAL-REFUSE.md` P14, and roles come from the document's own
    /// structure tree ([`super::PdfTaggedLocator::role_path`]) or from nowhere.
    ///
    /// # Absent means the cut made no division
    ///
    /// Not "single column" — those coincide on almost every real page and are not the same claim.
    /// Absent covers four states: no gutter met the rule, fewer than two runs, the capability is
    /// off, or the format has no cut. A node cannot tell them apart and does not need to: the
    /// profile's `reading_order_rule` already names which rule ran, and unlike a missing ink box
    /// none of the four can differ between two runs of one artifact. That is why this is a bare
    /// `Option` where [`crate::GeometryPresence`] is a typed absence —
    /// `docs/16-D4-SCOPE.md` §4 argues it against that precedent.
    ///
    /// Absent on the overwhelmingly common single-column page, where `skip_serializing_if` makes
    /// it cost nothing: the engine's run time is linear in the bytes it emits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<u32>,
    /// Which block of its page the leading-gap cut placed this run in, 1-based, in reading order.
    ///
    /// # Where, never what — the same rule [`Self::region`] is under
    ///
    /// A block is a stretch of text the rule found separated from its neighbours by vertical
    /// whitespace wider than the band's own leading. It is **not** a paragraph, a heading, a list
    /// item or a section, and nothing downstream may read a role from it. That is
    /// `docs/06-STEAL-REFUSE.md` **P14**, and it is the line
    /// [`docs/19-BLOCK-SUBDIVISION-SCOPE.md`] §6 draws in as many words when it permits this
    /// field at all: *"a **Computed block subdivision**… an unnamed index, absent where the rule
    /// declined."* Roles come from the document's own structure tree
    /// ([`super::PdfTaggedLocator::role_path`]) or from nowhere.
    ///
    /// **A block boundary is not a paragraph boundary, and the measurement says so.** On the one
    /// gate document that can carry a real paragraph label, the rule finds **63.7% of them at
    /// 100% precision** — it never fires mid-paragraph across 719 chances, and it misses better
    /// than a third of the breaks. §9.2 says why the ceiling is there: 35.1% of real paragraph
    /// breaks carry no extra leading at all, so no gap rule can see them. A consumer that treats
    /// a block as a paragraph is treating a 64%-recall index as a structure.
    ///
    /// # Absent means the rule declined, and that is the common case
    ///
    /// Unlike `region`, absence here is ordinary rather than a corner: a page of uniform body text
    /// has no gap wide enough to open a second block, and it is *supposed* to have none. Absent
    /// covers that, a band whose modal gap is not a leading at all (a table-column interleave
    /// pitch, which §5 measured on 9% of bands), a band whose gaps are a scatter with no mode, and
    /// a format with no cut. A node cannot tell them apart and does not need to — the profile's
    /// `reading_order_rule` names which rule ran, and none of the four can differ between two runs
    /// of one artifact. A bare `Option` for the reason `region` is one: `docs/16-D4-SCOPE.md` §4.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block: Option<u32>,
    /// What was observed about this run that a reader would not see in its text (v1-S6).
    ///
    /// Empty for an ordinary run, and empty is the common case. **A run carrying a finding is
    /// still a run**: same text, same origin, same locator, same position in reading order. The
    /// findings are additive information, never a reason to remove anything — see [`TextFinding`].
    ///
    /// Sorted and deduplicated, so two artifacts describing the same run cannot differ by the
    /// order two observations happened to be made in.
    ///
    /// `default` beside `skip_serializing_if` is load-bearing, not tidiness: without it the
    /// artifact serializes with the key omitted and then fails to deserialize its own output. A
    /// `Vec` is not an `Option`, which serde treats as optional on its own — the same trap v1-S4's
    /// `unrecognized_flag_bits` fell into, recorded there and avoided here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<TextFinding>,
}

/// An office run's facts: what OOXML states about a `<w:r>` and nothing else (v2-S2).
///
/// Deliberately one field. Everything else a run could carry — a style id, bold, a language — is
/// in `word/styles.xml` or a `<w:rPr>` this slice does not read, and a field that is always
/// `false` because nobody populated it is worse than no field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeRunAttributes {
    /// Whether the `<w:t>` carried `xml:space="preserve"`.
    ///
    /// **The DOCX counterpart to `trailing_space_generated`**, which `06-STEAL-REFUSE.md` calls
    /// the best honesty field in the four surveyed projects. Without `xml:space="preserve"` an
    /// XML consumer may collapse leading and trailing whitespace, so whether it was set decides
    /// whether this run's spaces are the document's or the parser's. Recorded rather than
    /// resolved: this reader takes the text verbatim either way and says which case it was.
    pub space_preserved: bool,
}

/// A slide run's facts: what PresentationML states about the shape holding it (v2-S4).
///
/// Everything else a run
/// could carry — a language, bold, a theme colour — is in an `<a:rPr>` or a slide layout this
/// slice does not read, and a field that is always empty because nobody populated it is worse
/// than no field.
///
/// The shape's **placeholder type** (`<p:ph type="title"/>`) is the one further fact worth having
/// and is deliberately absent: resolving what a placeholder means needs the slide layout and the
/// master, which this slice does not read, so a `placeholder` field would be either incomplete or
/// a second reader's worth of work. Named here so the next slice finds the decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeSlideRunAttributes {
    /// The `id` on the shape's `<p:cNvPr>`, as the slide states it.
    ///
    /// **A label, not an address**, and the distinction is measured rather than cautious: real
    /// decks exist in which two shapes on one slide carry the same id, so [`PptxLocator::shape`]
    /// is a position instead. Carried anyway because it is a fact the file states and the one a
    /// consumer needs to match this run against the same shape in the original package.
    pub shape_id: u32,
    /// The `name` on the shape's `<p:cNvPr>`, e.g. `Title 1`, verbatim.
    ///
    /// What the authoring tool called the shape, which is what a person reading a citation
    /// recognises. It may be empty: the attribute is required by the schema, its content is not.
    pub shape_name: String,
}

/// An OpenDocument paragraph's facts: what `content.xml` states about the block (v2-S5).
///
/// One field, and it is the one thing the element says about itself that [`OdtLocator`] does not
/// carry. Everything else a paragraph could carry — `text:style-name`, a list level, a language —
/// resolves through `styles.xml`, which this slice does not read, and a field nobody populated is
/// worse than no field.
///
/// **`text:outline-level` is deliberately absent.** A `<text:h>` states one, and it is a real fact
/// the file contains — but reading it would put a document *outline* on the wire while
/// `structural_locators` is false and no structure was read, which is a half of a claim rather
/// than a small one. Named here so the next slice finds the decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeParagraphAttributes {
    /// Which of ODF's two text blocks this node was read from.
    pub block: OdfBlockKind,
}

/// Whether an OpenDocument block was a paragraph or a heading (v2-S5).
///
/// The two elements ODF uses for a block of text, and the distinction is the file's own: a
/// `<text:h>` is a heading and a `<text:p>` is not, stated by the element name rather than
/// inferred from a font size — which `docs/history/14-V2-SCOPE.md` §9's second standing rule forbids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OdfBlockKind {
    /// `<text:p>`.
    Paragraph,
    /// `<text:h>`.
    Heading,
}

/// A spreadsheet cell's facts: what SpreadsheetML states about a `<c>` and nothing else (v2-S3).
///
/// Two fields, and neither is derivable from the other. The value type is what the cell's `t`
/// attribute says the stored value *is*; the text source is where the node's `text` came *from*.
/// A numeric cell computed by a formula is `Number` + [`CellTextSource::CachedFormulaResult`],
/// and a text cell that stores its own string is `InlineString` + [`CellTextSource::StoredValue`]
/// — the pair is not two spellings of one fact.
///
/// **Nothing here is a rendering.** A cell's displayed form comes from a number format in
/// `xl/styles.xml` — `42` shown as `$42.00` or as `1970-02-11` — and this reader does not read
/// styles, does not apply them, and does not carry a formatted string the file did not store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeCellAttributes {
    /// What the cell's `t` attribute says its stored value is.
    pub value_type: CellValueType,
    /// Where this node's `text` came from.
    pub text_source: CellTextSource,
}

/// What a `<c>`'s `t` attribute declares its stored value to be (v2-S3).
///
/// ECMA-376 `ST_CellType`. Every variant is one of the attribute's own legal values, and a `t`
/// this build does not recognise is a **named refusal** rather than a guess — the posture the
/// entity reader takes for the same reason, in `ethos-parser-office`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CellValueType {
    /// `t="n"`, **or no `t` at all** — SpreadsheetML's own default.
    ///
    /// The absent case is not this engine choosing: ECMA-376 defines the attribute's default as
    /// `n`, so a cell with no `t` states "number" by stating nothing. Reading a format's declared
    /// default is reading; assuming *text* — which is what a reader that defaulted to string
    /// would do — would put a quoted numeral in the evidence for a cell that holds a number.
    Number,
    /// `t="s"` — `<v>` is a 0-based index into `xl/sharedStrings.xml`.
    SharedString,
    /// `t="inlineStr"` — the text is in the cell's own `<is>`, and `<v>` is unused.
    InlineString,
    /// `t="str"` — `<v>` is a formula's cached **string** result, stored verbatim.
    FormulaString,
    /// `t="b"` — `<v>` is `0` or `1`, as stored. Not translated to `true`/`false`.
    Boolean,
    /// `t="e"` — `<v>` is an error literal, e.g. `#DIV/0!`, as stored.
    Error,
    /// `t="d"` — `<v>` is an ISO 8601 date **as text**, as stored.
    ///
    /// Distinct from [`Self::Number`] on purpose: a date stored as a serial number under a date
    /// number format is a `Number` here, because that is what the file stores. Converting one to
    /// the other needs the workbook's 1900/1904 epoch and a calendar, and a calendar conversion
    /// is a computation, not a reading.
    Date,
}

/// Where a cell node's `text` came from (v2-S3).
///
/// # Why this exists as its own field
///
/// A `<c>` may carry a formula. `<f>` is **not a second authority**: this engine has no evaluator
/// and will not acquire one, so the text is whatever the file stored. But "the number the
/// workbook last cached" and "the formula's source, because nothing was cached" are different
/// facts about the same cell, and a consumer that could not tell them apart would read
/// `SUM(B2:B2)` as if the sheet displayed that string. Labelling which one it is costs one field;
/// leaving it unlabelled costs the artifact its honesty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CellTextSource {
    /// The value the cell stores: its `<v>`, or the `<is>` of an inline string. No formula.
    StoredValue,
    /// The cell carries a `<f>` **and** a cached `<v>`. The text is that cached value, as stored.
    ///
    /// Nothing was evaluated. This says the workbook's own last-computed value is what is here.
    CachedFormulaResult,
    /// The cell carries a `<f>` and **no** cached `<v>`. The text is the formula source, as
    /// stored — `SUM(B2:B2)`, which is not a value the sheet ever displayed.
    FormulaSource,
}

/// An OpenDocument spreadsheet cell's facts: what ODF states about a `<table:table-cell>` (v2-S6).
///
/// # A second cell type, rather than [`OfficeCellAttributes`] with ODF poured into it
///
/// [`CellValueType`] is ECMA-376's `ST_CellType` — its variants are `n`, `s`, `inlineStr`, `str`,
/// `b`, `e`, `d`, and `SharedString` names a table that exists only in a workbook. ODF's
/// `office:value-type` is a different, overlapping list with `percentage` and `currency` in it and
/// no shared-string index anywhere. Mapping one onto the other would need a translation table this
/// engine would then have to defend, and reusing the type with `SharedString` unreachable and
/// `text_source` half-meaningless is the "blank those fields" shape `docs/history/14-V2-SCOPE.md` §8
/// refuses: a new format is a new **value**, and a value it cannot express needs its own type.
///
/// **Nothing here is a rendering.** ODF stores a cell's typed value in an attribute
/// (`office:value`, `office:date-value`, …) and its *displayed* form as `<text:p>` children. This
/// reader takes the displayed form, because that is the text a person quotes, and states which
/// attribute-declared type sat behind it. It applies no number format and computes no date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeOdfCellAttributes {
    /// What the cell's `office:value-type` attribute says its stored value is.
    pub value_type: OdfValueType,
    /// Where this node's `text` came from.
    pub text_source: OdfCellTextSource,
}

/// What a `<table:table-cell>`'s `office:value-type` declares its stored value to be (v2-S6).
///
/// OpenDocument's own list. A value this build does not recognise is a **named refusal** rather
/// than a guess, which is the posture [`CellValueType`] takes for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OdfValueType {
    /// `office:value-type="float"` — ODF's only numeric type. There is no separate integer.
    Float,
    /// `office:value-type="percentage"` — a float whose stored value is the fraction, not the
    /// percent. `office:value="0.25"` displays as `25%`, and `25%` is the displayed text.
    Percentage,
    /// `office:value-type="currency"` — a float with an `office:currency` code beside it.
    Currency,
    /// `office:value-type="date"` — `office:date-value`, an ISO 8601 date **as text**.
    Date,
    /// `office:value-type="time"` — `office:time-value`, an ISO 8601 duration **as text**.
    Time,
    /// `office:value-type="boolean"` — `office:boolean-value`, `true` or `false`.
    Boolean,
    /// `office:value-type="string"` — the value is the cell's own text.
    String,
    /// **No `office:value-type` at all.**
    ///
    /// Not this engine choosing a default: ODF defines the attribute as optional and a cell
    /// without one declares no typed value. Every real producer writes it on a cell that holds
    /// something, so this is the empty cell, the covered cell, and the cell whose only content is
    /// a shape. Distinct from [`Self::String`], which is a cell that states it holds text.
    Void,
}

/// Where an OpenDocument cell node's `text` came from (v2-S6).
///
/// Two variants where [`CellTextSource`] has three, and the missing one is the point. A
/// SpreadsheetML formula cell may cache nothing, leaving a reader with only the formula source to
/// report — ODF's `<text:p>` **is** the display form, so a cell with a `table:formula` and no
/// paragraph displays nothing and becomes no node at all. This reader therefore never puts a
/// formula's source text in `text`, and has no variant that would let it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OdfCellTextSource {
    /// The cell carries no `table:formula`. The text is the paragraphs the file stores.
    StoredText,
    /// The cell carries a `table:formula`. The text is the display form the producer **last
    /// cached**, as stored.
    ///
    /// Nothing was evaluated, and this engine has no evaluator. The label exists so a consumer
    /// can tell a typed-in `42` from a `42` that is one recalculation away from being something
    /// else.
    CachedFormulaText,
}

/// A presentation block's facts: the two names the file writes, and which block it was (v2-S7).
///
/// # Names here because they could not be measured unique
///
/// [`OdpLocator`] addresses by position. These are the strings the document itself puts on the
/// draw page and the shape, and they are on the attributes for v2-S4's measured reason rather than
/// as a matter of taste: an address that is not unique gives a citation two answers, and **no
/// corpus of real `.odp` files was available to measure whether producers write `draw:name`
/// uniquely** — or write it at all, since OpenDocument makes it optional on both elements. An
/// unmeasured identifier is a label, which is where `<p:cNvPr id>` ended up when S4 measured it
/// across 18 real decks and found it unique only most of the time.
///
/// # `presentation:class` is deliberately absent
///
/// `<draw:frame presentation:class="title">` states a placeholder role **on the frame**, so unlike
/// PresentationML's `<p:ph>` it needs no layout and no master to read — which makes it the one
/// further fact that could honestly be carried here. It is left out because this slice reads no
/// styles and mints no structural locator, and a role field on a reader that claims
/// `structural_locators: false` is half of a claim rather than a small one. Named here so the next
/// slice finds the decision, exactly as v2-S4 named the placeholder type it did not resolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfficeOdfShapeAttributes {
    /// The `draw:name` on the `<draw:page>`, verbatim with entities resolved, or **absent**.
    ///
    /// `None` is not the empty string: OpenDocument makes the attribute optional, so a page
    /// without one has stated no name and this says so rather than inventing `""` to stand in for
    /// it. What a producer writes here is what a person recognises — `Slide 1`, `page1` — which is
    /// why it is carried at all when the address does not need it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draw_page_name: Option<String>,
    /// The `draw:name` on the shape, verbatim with entities resolved, or **absent**.
    ///
    /// The counterpart to [`OfficeSlideRunAttributes::shape_name`], with the same optionality
    /// rule as the field above: absent means the file states none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_name: Option<String>,
    /// Which of ODF's two text blocks this node was read from.
    ///
    /// The same [`OdfBlockKind`] v2-S5 introduced, reused rather than re-spelled: `<text:h>` and
    /// `<text:p>` mean in a presentation exactly what they mean in a text document, and a second
    /// enum for one of them is the drift the shared allowlist exists to prevent.
    pub block: OdfBlockKind,
}

/// A Rich Text Format paragraph's facts: how the stream ended it (v2-S8).
///
/// One field, and it is the one thing the bytes say about this paragraph that [`RtfLocator`] does
/// not carry. Everything else an RTF paragraph could state — a style index, an indent, a
/// justification — resolves through `\stylesheet`, which this slice does not read, and a field
/// nobody populated is worse than no field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RtfParagraphAttributes {
    /// Which control word ended this paragraph.
    pub terminator: RtfParagraphBreak,
}

/// An EPUB block's facts: what the document and its spine state about it (v2-S9).
///
/// Two fields, and both are read rather than inferred. Everything else a block could carry — a
/// class, an `epub:type`, a computed heading level — resolves through a style sheet this slice
/// does not read or through a vocabulary it would have to map, and a field nobody populated is
/// worse than no field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpubBlockAttributes {
    /// The XHTML element's local name, verbatim: `p`, `h1`, `li`, `td`, `pre`, …
    ///
    /// **The file's own word for it, not a role this reader inferred.** A closed enum would need a
    /// mapping from twenty-odd flow elements onto a smaller vocabulary, and every such mapping is
    /// a claim this engine would then have to defend — `docs/history/14-V2-SCOPE.md` §9's second standing
    /// rule forbids deriving a role, and naming the element is the honest alternative to deriving
    /// one. Carried as a `String` for the reason [`TextRunAttributes::font_id`] is: it is a name
    /// the document states.
    ///
    /// Always an XHTML element. A `<title>` inside an inline `<svg>` is a different vocabulary and
    /// never reaches a node — see [`crate::NativeLocator::Epub`]'s reader.
    pub element: String,
    /// Whether the spine lists this document as part of the linear reading order.
    ///
    /// `<itemref linear="no">` marks auxiliary content — pop-up footnote targets, colophons, an
    /// appendix a reading system reaches only from a link. It **is** read, because it is a
    /// publication document the spine lists and skipping it would drop text the book contains;
    /// carrying the flag is what keeps that from being a silent *extra*, which is the failure
    /// v2-S7 named A14 inverted. A consumer that wants the main flow filters on this.
    pub linear: bool,
}

/// The control word that ended an RTF paragraph (v2-S8).
///
/// **A declared fact, not a table.** `\cell` and `\row` say the text sat inside a table, and
/// recording that is the honest half of a decision this slice makes explicitly: a
/// [`crate::tables::TableRecord`] is the PDF detector's finding about a grid it inferred from ink,
/// so an RTF table does not become one and `capabilities.tables` stays false. A consumer that
/// needs to know a phrase was in a cell reads it here, from a control word the file wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RtfParagraphBreak {
    /// `\par` — the ordinary paragraph mark.
    Paragraph,
    /// `\sect` — a section break, which ends the paragraph before it as well.
    Section,
    /// `\cell` — the end of a table cell.
    Cell,
    /// `\row` — the end of a table row.
    Row,
    /// The stream ended while this paragraph still held text.
    ///
    /// Not a defect and not a repair: an RTF writer is not required to put `\par` after the last
    /// paragraph, so this is the ordinary shape of a final paragraph rather than a truncation.
    /// Truncation is a **closed** brace this reader never saw, and that is a named refusal.
    EndOfStream,
}

/// A character the reader authored rather than read, flagged where it was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynthesizedAt {
    /// Index into the node's `text`, counted in `char`s.
    pub char_index: u32,
    /// Why it was inserted, in the format's own vocabulary.
    pub reason: String,
}

/// One page of the source, as a record rather than a node.
///
/// **Pages are deliberately not nodes.** A node carries a required `NativeLocator`, and the PDF
/// variant of that is a *character* origin — a page has none, so making a page a node would
/// force the emitter to write an origin it does not have. `{"origin_x":0,"origin_y":0}` for a
/// page is indistinguishable on the wire from a run drawn in the corner, and inventing a
/// coordinate is forbidden outright (`01-CONTRACT.md` §5.2, Workbench rule 3). Keeping pages in
/// their own list costs one array and removes the temptation entirely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRecord {
    /// Stable id within this representation, e.g. `p1`.
    pub id: NodeId,
    /// **1-based page number as the document numbers it** — not this record's position in the
    /// array. The two differ the moment a page is quarantined or fails, and a citation renders
    /// against the document's number, not against our array index.
    pub index: u32,
    /// Page width in integer centipoints, after rotation.
    pub width: i64,
    /// Page height in integer centipoints, after rotation.
    pub height: i64,
    /// Page rotation as declared: 0, 90, 180 or 270.
    pub rotation: i64,
}

/// One ordered, typed node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// Stable within this representation **and this profile**, never globally.
    ///
    /// A profile upgrade creates a new representation plus a mapping, rather than pretending
    /// node ids are permanently global (companion, and `ids` module).
    pub id: NodeId,
    /// What this node is.
    pub kind: NodeKind,
    /// The container this node belongs to: a **page** where the format has pages, and a **part**
    /// where it does not (v2-S2).
    ///
    /// It was *"the page this node was drawn on"* until v2 added a format that has none. Which of
    /// the two it is is decided by [`Self::native_locator`] and checked in `check_structure`, so
    /// the two cannot disagree: a PDF node's parent is a declared [`PageRecord`], and a page-less
    /// node's parent is a [`crate::IdKind::Part`] id that every node in that part shares.
    /// `docs/history/14-V2-SCOPE.md` §3 is why this is not simply "a page, and DOCX gets a fake one".
    pub parent: NodeId,
    /// Position among the nodes sharing this parent, **1-based**, in reading order.
    pub ordinal: u32,
    /// The decoded text.
    pub text: String,
    /// The required format-native address.
    pub native_locator: NativeLocator,
    /// A structural address where the document supplied one. Never invented.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural_locator: Option<StructuralLocator>,
    /// How this node came to exist. `Extracted` for text read from the content stream.
    pub derivation: DerivationClass,
    /// Facts only this node's kind has. Its tag must agree with [`Self::kind`].
    pub attributes: NodeAttributes,
}

/// Geometry for one node, carried **outside** the fingerprinted payload.
///
/// `docs/01-CONTRACT.md` §4 excludes geometry from the fingerprint deliberately: rectangles are
/// preserved for display and inspection but are not fingerprint-critical, because text is
/// anchored by stable origin locators rather than by boxes. Fingerprinting the boxes would make
/// artifact identity hostage to the least reliable number in it.
///
/// Implemented by **keeping the type out of the payload** rather than by filtering it out at
/// hashing time. A filter is a rule someone can quietly change; a type that is not there cannot
/// be hashed by accident.
///
/// # The trust boundary, stated plainly
///
/// Because these rows are outside the digest, **a verified representation attests to the text,
/// the reading order and the origins — not to the rectangles.** That matters more than it first
/// appears, because the grounding projection drops the locators and keeps only the boxes: the one
/// thing the fingerprint does not cover is the only spatial claim that survives into
/// `ethos.grounding.v1`. §4's reasoning still holds for the *record* (boxes are the least reliable
/// number in it, and identity must not be hostage to them); this note exists so nobody reads a
/// verified fingerprint as covering a bbox it never touched.
///
/// What *is* bound across the seam is the **count**: a representation whose sidecar disagrees with
/// its own `geometry-absent-not-groundable` declaration is refused at construction and at parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeGeometry {
    /// The node this row describes. Rows are in the same order as [`RepresentationPayload::nodes`].
    pub node: NodeId,
    /// A measured ink box, or a typed reason there is none.
    pub presence: GeometryPresence,
}

/// Everything the fingerprint covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationPayload {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    pub identity: ArtifactIdentity,
    /// What was read.
    pub source: SourceIdentity,
    /// Who read it, under what pinned configuration.
    pub processing_run: ProcessingRun,
    /// Declared on every artifact carrying geometry, never implied by the format (§3).
    pub coordinate_system: CoordinateSystem,
    /// Pages of the source, in page order.
    pub pages: Vec<PageRecord>,
    /// Ordered typed nodes, in reading order, grouped by page.
    pub nodes: Vec<Node>,
    /// Ruled tables, in page order (v1-S1).
    ///
    /// **An empty array means the detector looked and found none**, never that it did not look —
    /// `capabilities.tables` says which. Carried as its own array rather than as `nodes`: a table
    /// cell's text is a concatenation of runs that are *already* nodes, and emitting it twice
    /// would create a second place for the same text to live and a second place for it to drift.
    #[serde(default)]
    pub tables: Vec<crate::tables::TableRecord>,
    /// M4's L1 gate, carried forward by value: capabilities, limitations, per-page state,
    /// coverage, terminal state.
    pub assurance: Assurance,
}

impl RepresentationPayload {
    /// The canonical bytes this payload hashes over.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the payload will not canonicalize.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        crate::c14n::canonical_bytes_of(self).map_err(|e| EngineError::Malformed {
            what: "representation payload".into(),
            detail: e.to_string(),
        })
    }

    /// `sha256:<hex>` over [`Self::canonical_bytes`].
    ///
    /// # Errors
    ///
    /// Propagates [`Self::canonical_bytes`].
    pub fn fingerprint(&self) -> Result<Sha256Hex, EngineError> {
        let bytes = self.canonical_bytes()?;
        Ok(Sha256Hex::from_hex(&sha256_hex_bytes(&bytes))
            .expect("sha256 hex is always 64 lowercase hex digits"))
    }
}

/// The emitted canonical record.
///
/// Constructed only through [`Self::seal`], which computes the fingerprint and validates every
/// geometry box against its page. There is no constructor that *accepts* a fingerprint, for the
/// same reason [`Assurance`] derives its terminal state instead of accepting one: a value that
/// can be asserted independently of what it describes is a value that can disagree with it.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "RepresentationWire")]
pub struct DocumentRepresentation {
    artifact_type: String,
    schema_version: String,
    representation: RepresentationPayload,
    representation_c14n_sha256: Sha256Hex,
    geometry: Vec<NodeGeometry>,
    /// The payload's canonical bytes, kept from the pass `seal` hashed — so the
    /// emit path can splice them into the envelope instead of walking the whole
    /// payload a second time. Derived, not identity: absent on a parsed artifact,
    /// never on the wire, and excluded from equality below, because two artifacts
    /// that differ only in whether this cache is warm are the same artifact.
    #[serde(skip)]
    payload_c14n: Option<Vec<u8>>,
}

/// Equality is the five wire fields and nothing else — `payload_c14n` is a
/// derived cache, and deriving `PartialEq` over it would make a minted artifact
/// unequal to its own parsed round-trip.
impl PartialEq for DocumentRepresentation {
    fn eq(&self, other: &Self) -> bool {
        self.artifact_type == other.artifact_type
            && self.schema_version == other.schema_version
            && self.representation == other.representation
            && self.representation_c14n_sha256 == other.representation_c14n_sha256
            && self.geometry == other.geometry
    }
}

impl Eq for DocumentRepresentation {}

/// The wire form, so `deny_unknown_fields` and the structural checks both apply on parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepresentationWire {
    artifact_type: String,
    schema_version: String,
    representation: RepresentationPayload,
    representation_c14n_sha256: Sha256Hex,
    geometry: Vec<NodeGeometry>,
}

/// Serialized by hand rather than through `#[serde(into = "RepresentationWire")]`:
/// serde's `into` attribute serializes via `self.clone().into()`, which deep-cloned
/// the entire artifact — every run's text and codes — once per serialization, purely
/// to re-emit the same five fields under the same names. The wire struct still owns
/// parsing, where `deny_unknown_fields` and the structural checks live, and a test
/// below pins that this emits exactly what the wire form emits.
impl Serialize for DocumentRepresentation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        let mut s = serializer.serialize_struct("RepresentationWire", 5)?;
        s.serialize_field("artifact_type", &self.artifact_type)?;
        s.serialize_field("schema_version", &self.schema_version)?;
        s.serialize_field("representation", &self.representation)?;
        s.serialize_field(
            "representation_c14n_sha256",
            &self.representation_c14n_sha256,
        )?;
        s.serialize_field("geometry", &self.geometry)?;
        s.end()
    }
}

impl TryFrom<RepresentationWire> for DocumentRepresentation {
    type Error = EngineError;

    /// Structural well-formedness, checked at **parse** time.
    ///
    /// The fingerprint is deliberately *not* checked here — see
    /// [`DocumentRepresentation::verify_fingerprint`] for why a validator that cannot load the
    /// artifact it is judging cannot describe it.
    fn try_from(w: RepresentationWire) -> Result<Self, Self::Error> {
        let d = Self {
            artifact_type: w.artifact_type,
            schema_version: w.schema_version,
            representation: w.representation,
            representation_c14n_sha256: w.representation_c14n_sha256,
            geometry: w.geometry,
            payload_c14n: None,
        };
        d.check_structure()?;
        Ok(d)
    }
}

impl DocumentRepresentation {
    /// Seal a payload: validate it, attach its geometry, compute the fingerprint.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the geometry sidecar does not describe exactly the nodes in
    /// order, if a node's parent is not a declared page, if ordinals are not 1-based and
    /// contiguous per page, or if a **measured box falls outside its page**.
    pub fn seal(
        payload: RepresentationPayload,
        geometry: Vec<NodeGeometry>,
    ) -> Result<Self, EngineError> {
        // One canonical pass serves both the fingerprint and, kept as a cache, the
        // eventual print — the payload was being walked twice per artifact.
        let payload_bytes = payload.canonical_bytes()?;
        let fingerprint = Sha256Hex::from_hex(&sha256_hex_bytes(&payload_bytes))
            .expect("sha256 hex is always 64 lowercase hex digits");
        let d = Self {
            artifact_type: REPRESENTATION_ARTIFACT_TYPE.to_string(),
            schema_version: REPRESENTATION_SCHEMA_VERSION.to_string(),
            representation: payload,
            representation_c14n_sha256: fingerprint,
            geometry,
            payload_c14n: Some(payload_bytes),
        };
        d.check_structure()?;
        Ok(d)
    }

    /// Whether an id was minted as a [`crate::IdKind::Part`].
    ///
    /// Read off the prefix, which is what the id *is* on the wire. Prefixes are one character and
    /// distinct, so this cannot confuse a part with a page.
    fn is_part_id(id: &NodeId) -> bool {
        id.as_str().starts_with(crate::IdKind::Part.prefix())
    }

    /// The shape a page-less document has to have, checked once before the node walk (v2-S2).
    ///
    /// Four rules, and each one closes a way of faking a page. This said **three** from v2-S2
    /// until v2-S13.5; rule 4 arrived with RTF at v2-S8 and was numbered into the list without
    /// the count above it moving:
    ///
    /// 1. **`pages` is empty.** `docs/history/14-V2-SCOPE.md` §3: a non-empty `pages` on a document whose
    ///    nodes have no page is the defect the law exists to catch — an A4 record minted so the
    ///    old invariant would pass.
    /// 2. **No mixing.** A document is paginated or it is not. One of each would be a record
    ///    claiming a DOCX run and a PDF glyph share a coordinate space.
    /// 3. **Part id and part name agree, both ways.** Nothing declares parts in a separate list,
    ///    and nothing needs to: every page-less node names its part in its own locator, so the
    ///    artifact is self-describing as long as one part id means one part name and vice versa.
    ///    That is the integrity the declared-page lookup buys for a PDF, bought here without
    ///    inventing a list to look things up in.
    /// 4. **A part-less format's nodes all share one container id** (v2-S8). RTF is page-less
    ///    *and* part-less — one brace-group byte stream with no parts to name — so rule 3 has no
    ///    part name to be a bijection between. The claim it can still make is the same one in the
    ///    only shape the format allows: one document, one container, one id. **Mixing the two
    ///    shapes is refused**, because a document whose nodes disagree about whether they live in
    ///    a named part is a record describing two documents.
    fn check_page_less_shape(p: &RepresentationPayload) -> Result<(), EngineError> {
        if !p.pages.is_empty() {
            return Err(Self::malformed(format!(
                "this document's nodes have page-less addresses, but it declares {} page \
                 record(s). A page-less format has no pages: `pages` is empty, and a record put \
                 there so a node could name it would be invented pagination.",
                p.pages.len()
            )));
        }

        let mut part_of_id: std::collections::BTreeMap<&str, &str> = Default::default();
        let mut id_of_part: std::collections::BTreeMap<&str, &str> = Default::default();
        // The container every part-less node shares, once one has been seen (v2-S8).
        let mut only_container: Option<&str> = None;

        for node in &p.nodes {
            // **The part-less shape, checked on the one claim it can make.** Asked through
            // `names_a_part` rather than through `part()` being `None`, because until v2-S8 that
            // answer meant "paginated" and RTF made the overload wrong: an `.rtf` is page-less and
            // has no part either.
            if !node.native_locator.names_a_part() {
                let id = node.parent.as_str();
                match only_container {
                    Some(previous) if previous != id => {
                        return Err(Self::malformed(format!(
                            "node `{}` is parented by `{id}` where another part-less node is \
                             parented by `{previous}`. This format has one container, so its \
                             nodes have one container id — the integrity a part name buys for the \
                             package formats, in the only shape a stream can state it.",
                            node.id
                        )));
                    }
                    Some(_) => {}
                    None => only_container = Some(id),
                }
                continue;
            }

            let Some(part) = node.native_locator.part() else {
                return Err(Self::malformed(format!(
                    "node `{}` has a paginated address in a document whose other nodes have \
                     none. A representation describes one document, and one document is either \
                     paginated or it is not.",
                    node.id
                )));
            };
            let id = node.parent.as_str();

            if let Some(previous) = part_of_id.insert(id, part) {
                if previous != part {
                    return Err(Self::malformed(format!(
                        "part `{id}` names `{previous}` on one node and `{part}` on another; a \
                         part id means exactly one part"
                    )));
                }
            }
            if let Some(previous) = id_of_part.insert(part, id) {
                if previous != id {
                    return Err(Self::malformed(format!(
                        "part `{part}` is `{previous}` on one node and `{id}` on another; one \
                         part has exactly one id"
                    )));
                }
            }
        }

        // **And the two shapes are not mixed.** A document whose nodes disagree about whether they
        // live in a named part is a record describing two documents — the same objection rule 2
        // makes about mixing a paginated address with a page-less one.
        if only_container.is_some() && !part_of_id.is_empty() {
            return Err(Self::malformed(
                "this document mixes nodes that name a part with nodes that cannot. A \
                 representation describes one document, and one document is either a package with \
                 parts or a stream without them."
                    .into(),
            ));
        }
        Ok(())
    }

    fn malformed(detail: String) -> EngineError {
        EngineError::Malformed {
            what: "document representation".into(),
            detail,
        }
    }

    /// Every structural invariant, in one place, run on both construction paths.
    fn check_structure(&self) -> Result<(), EngineError> {
        let p = &self.representation;

        if self.artifact_type != REPRESENTATION_ARTIFACT_TYPE
            || self.schema_version != REPRESENTATION_SCHEMA_VERSION
        {
            return Err(Self::malformed(format!(
                "unrecognised artifact `{}` version `{}`; this build reads only \
                 `{REPRESENTATION_ARTIFACT_TYPE}` `{REPRESENTATION_SCHEMA_VERSION}`. Refusing \
                 rather than best-effort parsing an unknown shape.",
                self.artifact_type, self.schema_version
            )));
        }
        // The top-level tags exist so a reader can fail closed BEFORE parsing a payload. They
        // are a mirror of the payload's own identity, and a mirror that can disagree is worse
        // than no mirror.
        if p.identity.artifact_type != self.artifact_type
            || p.identity.schema_version != self.schema_version
        {
            return Err(Self::malformed(
                "the top-level artifact type/version disagree with the payload's identity".into(),
            ));
        }

        if self.geometry.len() != p.nodes.len() {
            return Err(Self::malformed(format!(
                "{} geometry rows for {} nodes; every node needs exactly one row, because a \
                 missing row would make absence ambiguous with omission",
                self.geometry.len(),
                p.nodes.len()
            )));
        }

        let mut pages_by_id = std::collections::BTreeMap::new();
        for page in &p.pages {
            if pages_by_id.insert(page.id.as_str(), page).is_some() {
                return Err(Self::malformed(format!("duplicate page id `{}`", page.id)));
            }
        }

        // **v2-S2: which family of document this is, decided by the nodes' own addresses.**
        // Not a flag and not a media type — a node reaches the page-less rules only by carrying
        // an address with no page in it, which is a thing the reader either read or did not.
        if p.nodes.iter().any(|n| !n.native_locator.is_paginated()) {
            Self::check_page_less_shape(p)?;
        }

        let mut seen_nodes = std::collections::BTreeSet::new();
        let mut ordinal_by_parent: std::collections::BTreeMap<&str, u32> = Default::default();

        for (i, node) in p.nodes.iter().enumerate() {
            if !seen_nodes.insert(node.id.as_str()) {
                return Err(Self::malformed(format!("duplicate node id `{}`", node.id)));
            }
            if self.geometry[i].node != node.id {
                return Err(Self::malformed(format!(
                    "geometry row {i} describes `{}` but node {i} is `{}`; the sidecar is \
                     index-aligned with the node list",
                    self.geometry[i].node, node.id
                )));
            }

            // A paginated node is parented by a declared page; a page-less one by a part. The
            // PDF branch is **unchanged**, message included: `docs/history/14-V2-SCOPE.md` §3 adds a
            // second family rather than loosening the first.
            let page = if node.native_locator.is_paginated() {
                let Some(page) = pages_by_id.get(node.parent.as_str()) else {
                    return Err(Self::malformed(format!(
                        "node `{}` names parent page `{}`, which is not a declared page",
                        node.id, node.parent
                    )));
                };
                Some(*page)
            } else {
                if !Self::is_part_id(&node.parent) {
                    return Err(Self::malformed(format!(
                        "node `{}` has a page-less address but its parent `{}` is not a part id \
                         (`{}`-prefixed). A page-less node is parented by the part it was read \
                         from, never by a page this engine invented for it.",
                        node.id,
                        node.parent,
                        crate::IdKind::Part.prefix()
                    )));
                }
                None
            };

            let next = ordinal_by_parent.entry(node.parent.as_str()).or_insert(0);
            *next += 1;
            if node.ordinal != *next {
                return Err(Self::malformed(format!(
                    "node `{}` has ordinal {} where {} was expected; ordinals are 1-based and \
                     contiguous within their container — a page where the format has pages, a \
                     part where it does not — in reading order",
                    node.id, node.ordinal, *next
                )));
            }

            match (self.geometry[i].presence, page) {
                (GeometryPresence::Measured(r), Some(page)) => {
                    Self::check_box_within_page(&node.id, r, page)?;
                }
                // **The no-invented-geometry law, mechanised.** A box is validated against the
                // page that contains it, so a node with no page has nothing to validate against
                // — and a rectangle nobody can check is exactly the fabrication
                // `docs/history/14-V2-SCOPE.md` §3 refuses. Absence is the only honest presence here.
                (GeometryPresence::Measured(_), None) => {
                    return Err(Self::malformed(format!(
                        "node `{}` has a page-less address and a measured box. There is no page \
                         to contain it, so the rectangle cannot be checked and must not be \
                         emitted: a page-less node's geometry is typed absence.",
                        node.id
                    )));
                }
                _ => {}
            }
        }

        self.check_geometry_matches_its_declaration()?;
        self.check_cell_runs_are_declared_nodes(&seen_nodes)?;
        self.check_attributes_agree_with_kind()?;

        Ok(())
    }

    /// Every node's `kind` agrees with the kind its own attributes belong to.
    ///
    /// [`NodeAttributes`]'s documentation has said since v0 that this redundancy is a **checked**
    /// one — *"redundancy that is tested is a cross-check; redundancy that is not is two places
    /// for the truth to live"* — and until v2-S3 nothing checked it. Both construction paths ran
    /// straight past a node reading `"kind":"annotation"` with `office_cell` attributes, so a
    /// consumer dispatching on `kind` and one dispatching on the attributes tag would read the
    /// same node as two different things.
    ///
    /// Found while adding the third variant that relies on the claim.
    fn check_attributes_agree_with_kind(&self) -> Result<(), EngineError> {
        for node in &self.representation.nodes {
            let claimed = node.attributes.kind();
            if node.kind != claimed {
                return Err(Self::malformed(format!(
                    "node `{}` is a `{}` but carries `{}` attributes. The two are a cross-check, \
                     not a convenience: a record whose kind and attributes disagree reads as two \
                     different things depending on which field a consumer trusts.",
                    node.id,
                    node.kind.as_str(),
                    claimed.as_str()
                )));
            }
        }
        Ok(())
    }

    /// Every node a table cell names is a declared node of this record (v1.1-S2).
    ///
    /// **The link is only worth carrying if it resolves.** A cell naming an id no node carries is
    /// a citation to nothing, and it fails in the least visible way available: the Markdown
    /// projection would emit that cell's bytes as a `source` segment addressing a node the
    /// consumer cannot look up, which is precisely the silently-dead locator
    /// `docs/history/10-V11-SCOPE.md` §2 exists to prevent.
    ///
    /// Checked here rather than in the projection because it is a property of the **record**, and
    /// a record that cannot satisfy it should never be sealed.
    fn check_cell_runs_are_declared_nodes(
        &self,
        seen_nodes: &std::collections::BTreeSet<&str>,
    ) -> Result<(), EngineError> {
        for table in &self.representation.tables {
            for cell in &table.cells {
                for id in &cell.node_ids {
                    if !seen_nodes.contains(id.as_str()) {
                        return Err(Self::malformed(format!(
                            "cell `{}` of table `{}` names run `{id}`, which is not a declared \
                             node. A cell whose text cannot be traced to evidence is worse than a \
                             cell with no text.",
                            cell.id, table.id
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// The sidecar and the payload's own declaration must agree about what is groundable.
    ///
    /// **This is the seam the fingerprint cannot cover.** Geometry sits outside the digest by
    /// §4, deliberately — but the payload declares, under
    /// `geometry-absent-not-groundable`, how many nodes cannot be projected. Without this check
    /// the two can be made to contradict each other while `verify_fingerprint` still passes:
    ///
    /// - flip a row `Measured` → `Absent` and the node silently vanishes from any projection
    ///   while the record declares no gap at all;
    /// - flip one `Absent` → `Measured` and a **fabricated box** is emitted while the record
    ///   still says that node has no measurable ink.
    ///
    /// Binding the two costs nothing and closes both. What it deliberately does **not** do is
    /// authenticate box *values* — §4's rationale is that boxes are the least reliable number in
    /// the artifact and identity must not be hostage to them. So the honest statement of the
    /// trust boundary, which `NodeGeometry` and the CLI both repeat, is: a verified
    /// representation attests to the text, the order and the origins. It does not attest to the
    /// rectangles.
    fn check_geometry_matches_its_declaration(&self) -> Result<(), EngineError> {
        let not_groundable = self.nodes_not_groundable();
        let declared = self
            .representation
            .assurance
            .limitations
            .iter()
            .any(|l| l.code == crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE);

        if (not_groundable > 0) != declared {
            return Err(Self::malformed(format!(
                "{not_groundable} node(s) have no measurable ink box, but the payload {} declare \
                 `{}`. The geometry sidecar sits outside the fingerprint by design, so this \
                 agreement is what stops a record and its own projection contradicting each \
                 other.",
                if declared { "does" } else { "does not" },
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            )));
        }
        Ok(())
    }

    /// A measured box must lie inside its page. **Hard error, never a clamp.**
    ///
    /// Reachable in practice, not theoretical: an ink box is `baseline_y - ascent × size` in a
    /// top-left system, so a run whose baseline sits within one ascent of the page top produces
    /// a negative `y0`, and `QRect::new` accepts it — it validates ordering and magnitude, not
    /// page containment. The grounding schema requires `x0, y0 >= 0`, and Ethos additionally
    /// requires `x1 <= page.width && y1 <= page.height`, so such a box is un-emittable.
    ///
    /// Three responses were available and two are forbidden. **Clamping fabricates** a
    /// coordinate the document does not contain (Workbench rule 3). **Omitting** it would have
    /// to travel the geometry-omission path, and that path takes a typed absence by construction
    /// — an out-of-page *measured* box is not an absence, and routing it through would be
    /// exactly the "omit for a non-geometry reason" hole §11 forbids. So the box is refused, and
    /// loudly: by the time one reaches here the measurement or the coordinate transform is wrong,
    /// and that is worth finding rather than hiding behind a plausible rectangle.
    ///
    /// **"By the time one reaches here" is doing real work in that sentence, and it did not used
    /// to be there (D4-S5).** This check twice refused documents whose transform was correct, and
    /// each time the answer was upstream rather than a weaker invariant. v1-S6.2: a run of spaces
    /// was given a rectangle around nothing, and 491 of `nist-sp-800-53r5`'s 492 pages were
    /// unreadable — answered by [`crate::GeometryAbsence::NoInkToMeasure`]. D4-S5: a document
    /// *itself* drew ink off its own page — `01030000000029.pdf` sets `Tm` at x = −435.1181 pt
    /// against a media box starting at 0 — and six of two hundred DP-Bench documents produced no
    /// artifact at all; answered by [`crate::GeometryAbsence::MeasuredOffPage`], which the PDF
    /// reader now decides while the page is still in scope.
    ///
    /// **The invariant here is deliberately unchanged by both.** Its job is to catch an engine
    /// that computed a coordinate it cannot justify, and a reachable case that is *not* that is a
    /// reason to teach the producer a new spelling — never to widen the seal, which would spend
    /// the one check standing between a transform bug and a plausible-looking artifact.
    fn check_box_within_page(
        node: &NodeId,
        r: QRect,
        page: &PageRecord,
    ) -> Result<(), EngineError> {
        if r.x0() < 0 || r.y0() < 0 || r.x1() > page.width || r.y1() > page.height {
            return Err(Self::malformed(format!(
                "node `{node}` has a measured box [{}, {}, {}, {}] outside its page \
                 [0, 0, {}, {}]. A box is not clamped to fit and not silently dropped: this \
                 means the measurement or the coordinate transform is wrong.",
                r.x0(),
                r.y0(),
                r.x1(),
                r.y1(),
                page.width,
                page.height
            )));
        }
        Ok(())
    }

    /// The payload the fingerprint covers.
    pub fn payload(&self) -> &RepresentationPayload {
        &self.representation
    }

    /// The declared fingerprint.
    pub fn fingerprint(&self) -> &Sha256Hex {
        &self.representation_c14n_sha256
    }

    /// Geometry rows, index-aligned with `payload().nodes`.
    pub fn geometry(&self) -> &[NodeGeometry] {
        &self.geometry
    }

    /// Geometry for the node at `index` in `payload().nodes`.
    pub fn geometry_at(&self, index: usize) -> Option<GeometryPresence> {
        self.geometry.get(index).map(|g| g.presence)
    }

    /// How many nodes cannot be projected into a grounding artifact.
    ///
    /// Keyed to [`GeometryPresence::is_groundable`] — the **same predicate the omission uses** —
    /// rather than to one absence variant. A count derived from a different question than the
    /// one the adapter asks is a count that goes wrong the first time a second absence variant
    /// occurs.
    pub fn nodes_not_groundable(&self) -> u32 {
        self.geometry
            .iter()
            .filter(|g| !g.presence.is_groundable())
            .count() as u32
    }

    /// Recompute the fingerprint from the payload as it stands.
    ///
    /// # Errors
    ///
    /// Propagates canonicalization failure.
    pub fn recompute_fingerprint(&self) -> Result<Sha256Hex, EngineError> {
        self.representation.fingerprint()
    }

    /// Check the declared fingerprint against the payload.
    ///
    /// Deliberately **not** run by `Deserialize`. `grounding-check` (M6) must be able to load an
    /// artifact and then *report* on it — Ethos's own report distinguishes `structure` from
    /// `source_binding` — and a validator that refuses to parse what it is judging can only ever
    /// say "invalid JSON".
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] naming **both** digests, so the failure is diagnosable rather
    /// than merely detected.
    pub fn verify_fingerprint(&self) -> Result<(), EngineError> {
        let actual = self.recompute_fingerprint()?;
        if actual != self.representation_c14n_sha256 {
            return Err(Self::malformed(format!(
                "declared representation_c14n_sha256 is {} but the payload hashes to {}",
                self.representation_c14n_sha256, actual
            )));
        }
        Ok(())
    }

    /// Canonical bytes of the whole artifact.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        let malformed = |e: crate::c14n::C14nError| EngineError::Malformed {
            what: "representation".into(),
            detail: e.to_string(),
        };
        // A minted artifact carries the payload bytes `seal` already canonicalized
        // and hashed; splicing them into the envelope skips the second whole-payload
        // walk. A parsed artifact has no cache and takes the full pass, and a test
        // pins the two routes byte-equal.
        if let Some(payload_bytes) = &self.payload_c14n {
            // The four small members are canonicalized into owned buffers; the payload — 82% of
            // the artifact on a gate document — is BORROWED from the cache rather than cloned
            // into the argument. An emit path that exists to avoid walking the payload twice was
            // copying it instead, which is a full-artifact allocation at peak for nothing.
            let artifact_type =
                crate::c14n::canonical_bytes_of(&self.artifact_type).map_err(malformed)?;
            let schema_version =
                crate::c14n::canonical_bytes_of(&self.schema_version).map_err(malformed)?;
            let digest = crate::c14n::canonical_bytes_of(&self.representation_c14n_sha256)
                .map_err(malformed)?;
            let geometry = crate::c14n::canonical_bytes_of(&self.geometry).map_err(malformed)?;
            return crate::c14n::canonical_object(vec![
                ("artifact_type", artifact_type.as_slice()),
                ("schema_version", schema_version.as_slice()),
                ("representation", payload_bytes.as_slice()),
                ("representation_c14n_sha256", digest.as_slice()),
                ("geometry", geometry.as_slice()),
            ])
            .map_err(malformed);
        }
        crate::c14n::canonical_bytes_of(self).map_err(malformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assurance::{PageState, PageStateEntry};
    use crate::c14n::c14n_bytes;
    use crate::derivation::GeometryAbsence;
    use crate::ids::{IdAllocator, IdKind};
    use crate::profile::{Capabilities, Profile};

    fn payload(nodes: Vec<Node>, pages: Vec<PageRecord>) -> RepresentationPayload {
        let profile = Profile::default();
        // Authorize every page up to the highest one present, and mark the ones this
        // representation does not carry as never attempted — the shape a budgeted run really
        // produces, rather than one where the page list and the coverage disagree.
        let authorized = pages.iter().map(|p| p.index).max().unwrap_or(0);
        let states: Vec<PageStateEntry> = (1..=authorized)
            .map(|index| PageStateEntry {
                index,
                state: if pages.iter().any(|p| p.index == index) {
                    PageState::Processed
                } else {
                    PageState::NotAttempted(crate::assurance::codes::RESOURCE_LIMIT_PAGES.into())
                },
            })
            .collect();
        RepresentationPayload {
            identity: ArtifactIdentity {
                artifact_type: REPRESENTATION_ARTIFACT_TYPE.into(),
                schema_version: REPRESENTATION_SCHEMA_VERSION.into(),
                parser_version: profile.parser_version.clone(),
                profile_sha256: profile.profile_sha256().unwrap(),
            },
            source: SourceIdentity {
                media_type: "application/pdf".into(),
                sha256: Sha256Hex::from_hex(&sha256_hex_bytes(b"x")).unwrap(),
            },
            processing_run: ProcessingRun {
                processor: ProcessorIdentity {
                    name: "ethos-parser".into(),
                    version: profile.parser_version.clone(),
                    backend: "lopdf 0.44.0".into(),
                },
                reading_order_rule: profile.reading_order_rule.clone(),
            },
            coordinate_system: CoordinateSystem::V0,
            pages,
            nodes,
            tables: Vec::new(),
            assurance: Assurance::new(
                Capabilities::V0,
                authorized,
                states,
                vec![crate::assurance::Limitation::document(
                    crate::assurance::codes::RESOURCE_LIMIT_PAGES,
                    "test fixture: pages this representation does not carry",
                )],
            )
            .unwrap(),
        }
    }

    fn page(alloc: &mut IdAllocator, index: u32) -> PageRecord {
        PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index,
            width: 30000,
            height: 14400,
            rotation: 0,
        }
    }

    fn node(alloc: &mut IdAllocator, parent: &NodeId, ordinal: u32, page: u32) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: parent.clone(),
            ordinal,
            text: "hello".into(),
            native_locator: NativeLocator::Pdf(PdfLocator {
                page,
                origin_x: 7200,
                origin_y: 7200,
                advance: Some(1000),
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::TextRun(TextRunAttributes {
                char_codes: vec![0x68, 0x65, 0x6c, 0x6c, 0x6f],
                scalar_code_mismatch: false,
                synthesized: Vec::new(),
                findings: Vec::new(),
                font_id: "F1".into(),
                font_size: 2400,
                region: None,
                block: None,
            }),
        }
    }

    fn geom(node: &Node, presence: GeometryPresence) -> NodeGeometry {
        NodeGeometry {
            node: node.id.clone(),
            presence,
        }
    }

    /// Seal, declaring the geometry omission when the sidecar actually has one.
    ///
    /// The declaration is not optional garnish: `check_structure` binds the sidecar to it, so a
    /// helper that skipped it would make every absent-geometry test fail for the wrong reason.
    fn seal(
        nodes: Vec<Node>,
        pages: Vec<PageRecord>,
        geometry: Vec<NodeGeometry>,
    ) -> Result<DocumentRepresentation, EngineError> {
        let mut p = payload(nodes, pages);
        if geometry.iter().any(|g| !g.presence.is_groundable()) {
            let mut limitations = p.assurance.limitations.clone();
            limitations.push(crate::assurance::Limitation::document(
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
                "test fixture: nodes with no measurable ink box",
            ));
            p.assurance = Assurance::new(
                p.assurance.capabilities,
                p.assurance.coverage.pages_authorized,
                p.assurance.page_states.clone(),
                limitations,
            )
            .unwrap();
        }
        DocumentRepresentation::seal(p, geometry)
    }

    /// The spliced emit route and the full serialization are one byte stream —
    /// and a parsed artifact, whose cache is cold, equals its minted original.
    #[test]
    fn the_payload_cache_splice_is_the_full_serialization_by_another_route() {
        let minted = simple();
        let spliced = minted.to_canonical_bytes().expect("cache route");
        let full = crate::c14n::canonical_bytes_of(&minted).expect("full route");
        assert_eq!(spliced, full, "the splice must not be able to differ");

        let parsed: DocumentRepresentation = serde_json::from_slice(&spliced).expect("round-trips");
        assert_eq!(
            parsed, minted,
            "equality is the wire fields; a cold cache must not break it"
        );
        assert_eq!(
            parsed.to_canonical_bytes().expect("cold-cache route"),
            spliced,
            "a parsed artifact re-emits the same bytes through the full pass"
        );
    }

    fn simple() -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
        );
        seal(vec![n], vec![p], vec![g]).unwrap()
    }

    // ---------------------------------------------------------------------------------------
    // v2-S2: the page-less family
    // ---------------------------------------------------------------------------------------

    /// A page-less node, parented by a part, in a document that declares no pages.
    fn docx_node(alloc: &mut IdAllocator, part: &NodeId, ordinal: u32, paragraph: u32) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: part.clone(),
            ordinal,
            text: "hello".into(),
            native_locator: NativeLocator::Docx(DocxLocator {
                part: "word/document.xml".into(),
                paragraph,
                run: 1,
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeRun(OfficeRunAttributes {
                space_preserved: false,
            }),
        }
    }

    /// **The invariant v2-S2 exists to change.** A page-less document seals with no pages at all.
    ///
    /// Before this slice `check_structure` required every node's parent to be a declared page, so
    /// this representation was unconstructible and a DOCX could not enter the IR. The empty
    /// `pages` vector is now what `14-V2-SCOPE.md` §3 always said it was — the spelling of *"this
    /// document has no pages"* — rather than a shape only a document with no nodes could have.
    #[test]
    fn a_page_less_document_seals_with_no_pages_at_all() {
        let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
        let part = alloc.next(IdKind::Part).unwrap();
        let n1 = docx_node(&mut alloc, &part, 1, 1);
        let n2 = docx_node(&mut alloc, &part, 2, 2);
        let geometry = vec![
            geom(
                &n1,
                GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            ),
            geom(
                &n2,
                GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            ),
        ];
        let sealed = seal(vec![n1, n2], Vec::new(), geometry).expect("a page-less document seals");
        assert!(sealed.payload().pages.is_empty());
        assert_eq!(sealed.payload().nodes.len(), 2);
    }

    /// **A PDF node in that same document still refuses.** The second family did not open a hole.
    #[test]
    fn a_paginated_node_in_a_page_less_document_still_refuses() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let part = alloc.next(IdKind::Part).unwrap();
        let n = node(&mut alloc, &part, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        );
        let err = seal(vec![n], Vec::new(), vec![g]).unwrap_err();
        assert!(
            err.to_string().contains("is not a declared page"),
            "a paginated address still needs its page: {err}"
        );
    }

    /// **The fake A4 record, refused by name.** This is the defect §3 exists to catch.
    ///
    /// The shortest route to making a DOCX pass the old invariant was to mint one page record and
    /// point every node at it. That produces a document claiming a page nothing measured, so it is
    /// refused whether or not the geometry would have fitted inside it.
    #[test]
    fn a_page_less_document_that_declares_a_page_is_refused() {
        let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
        let a4 = page(&mut alloc, 1);
        let part = alloc.next(IdKind::Part).unwrap();
        let n = docx_node(&mut alloc, &part, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        );
        let err = seal(vec![n], vec![a4], vec![g]).unwrap_err();
        assert!(
            err.to_string().contains("invented pagination"),
            "the refusal must name what it refuses: {err}"
        );
    }

    /// **A page-less node may not carry a measured box.**
    ///
    /// A box is validated against the page containing it, so a node with no page has nothing to
    /// validate against — and a rectangle nobody can check is the fabrication §3 refuses. This is
    /// the rule that makes "no geometry" a property of the artifact rather than a convention the
    /// office reader happens to follow.
    #[test]
    fn a_page_less_node_may_not_carry_a_measured_box() {
        let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
        let part = alloc.next(IdKind::Part).unwrap();
        let n = docx_node(&mut alloc, &part, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
        );
        let err = seal(vec![n], Vec::new(), vec![g]).unwrap_err();
        assert!(
            err.to_string().contains("no page to contain it"),
            "the refusal must say why a box cannot be checked here: {err}"
        );
    }

    /// **A page-less node parented by something that is not a part is refused.**
    #[test]
    fn a_page_less_node_needs_a_part_for_a_parent() {
        let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
        // A page id, in a document with no pages: the shape a reader would produce if it kept
        // minting page ids out of habit.
        let not_a_part = alloc.next(IdKind::Page).unwrap();
        let n = docx_node(&mut alloc, &not_a_part, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        );
        let err = seal(vec![n], Vec::new(), vec![g]).unwrap_err();
        assert!(err.to_string().contains("is not a part id"), "{err}");
    }

    /// **One part id means one part name, both ways.**
    ///
    /// Nothing declares parts in a separate list, so this bijection is what buys the integrity a
    /// declared-page lookup buys for a PDF — without inventing a list to look things up in.
    #[test]
    fn a_part_id_and_a_part_name_agree_in_both_directions() {
        let mut alloc = IdAllocator::new(Profile::docx_v0().profile_sha256().unwrap());
        let part = alloc.next(IdKind::Part).unwrap();
        let n1 = docx_node(&mut alloc, &part, 1, 1);
        let mut n2 = docx_node(&mut alloc, &part, 2, 2);
        // Same part id, a different part name.
        n2.native_locator = NativeLocator::Docx(DocxLocator {
            part: "word/header1.xml".into(),
            paragraph: 1,
            run: 1,
        });
        let geometry = vec![
            geom(
                &n1,
                GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            ),
            geom(
                &n2,
                GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            ),
        ];
        let err = seal(vec![n1, n2], Vec::new(), geometry).unwrap_err();
        assert!(err.to_string().contains("means exactly one part"), "{err}");
    }

    /// **Two parts is legal, and this is the case that proves the rule above is a bijection.**
    ///
    /// A workbook is the first format that produces one part per sheet, and nothing in
    /// `check_page_less_shape` had to change for it: two ids mapping to two names violates
    /// neither direction, and `check_structure` counts ordinals **per parent**, so each part
    /// carries its own contiguous 1-based sequence.
    #[test]
    fn two_parts_with_two_names_seal_and_keep_separate_ordinals() {
        let mut alloc = IdAllocator::new(Profile::xlsx_v0().profile_sha256().unwrap());
        let first = alloc.next(IdKind::Part).unwrap();
        let second = alloc.next(IdKind::Part).unwrap();

        let mut nodes = Vec::new();
        for (part, sheet, name, ordinals) in [
            (&first, "Ledger", "xl/worksheets/sheet1.xml", 1..=2u32),
            (&second, "Notes", "xl/worksheets/sheet3.xml", 1..=1u32),
        ] {
            for ordinal in ordinals {
                nodes.push(Node {
                    id: alloc.next(IdKind::Span).unwrap(),
                    kind: NodeKind::TextRun,
                    parent: part.clone(),
                    ordinal,
                    text: format!("{sheet} {ordinal}"),
                    native_locator: NativeLocator::Xlsx(XlsxLocator {
                        part: name.into(),
                        sheet: sheet.into(),
                        row: ordinal,
                        column: "A".into(),
                    }),
                    structural_locator: None,
                    derivation: DerivationClass::Extracted,
                    attributes: NodeAttributes::OfficeCell(OfficeCellAttributes {
                        value_type: CellValueType::Number,
                        text_source: CellTextSource::StoredValue,
                    }),
                });
            }
        }
        let geometry: Vec<_> = nodes
            .iter()
            .map(|n| {
                geom(
                    n,
                    GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
                )
            })
            .collect();

        seal(nodes, Vec::new(), geometry).expect("many parts is what a workbook is");
    }

    /// **The cross-check `NodeAttributes`' own docs promised, made real at v2-S3.**
    ///
    /// A node whose `kind` and attributes disagree is refused on both construction paths. Before
    /// this, the claim was prose only, and the third variant to rely on it is what surfaced that.
    #[test]
    fn a_node_whose_kind_disagrees_with_its_attributes_is_refused() {
        let mut alloc = IdAllocator::new(Profile::xlsx_v0().profile_sha256().unwrap());
        let part = alloc.next(IdKind::Part).unwrap();
        let mut node = Node {
            id: alloc.next(IdKind::Span).unwrap(),
            // Cell facts under an annotation's kind: two readings of one node.
            kind: NodeKind::Annotation,
            parent: part,
            ordinal: 1,
            text: "a cell".into(),
            native_locator: NativeLocator::Xlsx(XlsxLocator {
                part: "xl/worksheets/sheet1.xml".into(),
                sheet: "Ledger".into(),
                row: 1,
                column: "A".into(),
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeCell(OfficeCellAttributes {
                value_type: CellValueType::Number,
                text_source: CellTextSource::StoredValue,
            }),
        };
        let geometry = vec![geom(
            &node,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        )];
        let err = seal(vec![node.clone()], Vec::new(), geometry.clone()).unwrap_err();
        assert!(err.to_string().contains("cross-check"), "{err}");

        // And the same node with the kind its attributes name seals, so the test above fails for
        // the disagreement and for nothing else.
        node.kind = NodeKind::TextRun;
        seal(vec![node], Vec::new(), geometry).expect("agreement is all that was missing");
    }

    /// A cell's address carries no page and no box, and none can be added quietly.
    #[test]
    fn an_xlsx_locator_denies_a_page_and_a_box() {
        let honest =
            r#"{"part":"xl/worksheets/sheet1.xml","sheet":"Ledger","row":12,"column":"B"}"#;
        serde_json::from_str::<XlsxLocator>(honest).expect("the four fields the file states");

        for smuggled in [
            r#"{"part":"p","sheet":"s","row":1,"column":"A","page":1}"#,
            r#"{"part":"p","sheet":"s","row":1,"column":"A","bbox":[0,0,1,1]}"#,
            r#"{"part":"p","sheet":"s","row":1,"column":"A","column_width":2048}"#,
            r#"{"part":"p","sheet":"s","row":1,"column":"A","print_area":"A1:B2"}"#,
        ] {
            assert!(
                serde_json::from_str::<XlsxLocator>(smuggled).is_err(),
                "`deny_unknown_fields` is what stops a rendering arriving as a field: {smuggled}"
            );
        }
    }

    /// A slide run's address carries no page, no box and **no slide number**.
    #[test]
    fn a_pptx_locator_denies_a_page_a_box_and_a_slide_index() {
        let honest = r#"{"part":"ppt/slides/slide1.xml","shape":1,"paragraph":2,"run":1}"#;
        serde_json::from_str::<PptxLocator>(honest).expect("the four fields the file states");

        for smuggled in [
            r#"{"part":"p","shape":1,"paragraph":1,"run":1,"page":3}"#,
            r#"{"part":"p","shape":1,"paragraph":1,"run":1,"slide":3}"#,
            r#"{"part":"p","shape":1,"paragraph":1,"run":1,"slide_index":3}"#,
            r#"{"part":"p","shape":1,"paragraph":1,"run":1,"bbox":[0,0,1,1]}"#,
            r#"{"part":"p","shape":1,"paragraph":1,"run":1,"x":914400,"y":914400}"#,
        ] {
            assert!(
                serde_json::from_str::<PptxLocator>(smuggled).is_err(),
                "a slide is a part, not a page, and `deny_unknown_fields` is what holds it: \
                 {smuggled}"
            );
        }
    }

    /// **Every** profile is mutually distinct, so no two formats' artifacts compare equal.
    ///
    /// The array used to hold four — `pdf`, `docx`, `xlsx`, `pptx` — while nine profiles existed,
    /// and the name still said *every*. The nine-way property was covered, by
    /// `the_epub_profile_is_its_own_and_all_nine_are_distinct` in
    /// `crates/ethos-parser-office/tests/epub_representation.rs`, so nothing went unverified; what went
    /// wrong is subtler and is the trap v2-S9's review named. A test whose name overclaims makes
    /// a reader stop looking: the next person to add a tenth profile reads *every* here, sees a
    /// green tick, and never learns that the array is a list someone has to remember to grow.
    ///
    /// Every constructor is `pub` and lives in this crate, so there was never a reason for the
    /// short list beyond the order they were written in.
    #[test]
    fn every_profile_is_distinct_from_every_other() {
        let profiles = [
            ("pdf", Profile::default()),
            ("docx", Profile::docx_v0()),
            ("xlsx", Profile::xlsx_v0()),
            ("pptx", Profile::pptx_v0()),
            ("odt", Profile::odt_v0()),
            ("ods", Profile::ods_v0()),
            ("odp", Profile::odp_v0()),
            ("rtf", Profile::rtf_v0()),
            ("epub", Profile::epub_v0()),
        ];

        // The count, asserted rather than assumed. A tenth profile that nobody added here would
        // otherwise leave this test still passing and still claiming *every*.
        assert_eq!(
            profiles.len(),
            9,
            "nine profiles ship: the default PDF one and eight `*_v0()` constructors. If a \
             format was added, it belongs in this array — the name of this test is a promise."
        );
        for (i, (left_name, left)) in profiles.iter().enumerate() {
            for (right_name, right) in profiles.iter().skip(i + 1) {
                assert_ne!(
                    left.profile_sha256().unwrap(),
                    right.profile_sha256().unwrap(),
                    "{left_name} and {right_name} must be provably non-comparable"
                );
            }
        }
    }

    /// A workbook artifact is comparable with neither a PDF one nor a DOCX one.
    #[test]
    fn the_xlsx_profile_is_neither_the_pdf_nor_the_docx_profile() {
        let xlsx = Profile::xlsx_v0().profile_sha256().unwrap();
        assert_ne!(xlsx, Profile::default().profile_sha256().unwrap());
        assert_ne!(xlsx, Profile::docx_v0().profile_sha256().unwrap());
    }

    /// A DOCX profile and the PDF default are **provably non-comparable** (`14-V2-SCOPE.md` §8).
    #[test]
    fn the_docx_profile_is_not_the_pdf_profile() {
        assert_ne!(
            Profile::docx_v0().profile_sha256().unwrap(),
            Profile::default().profile_sha256().unwrap(),
            "two formats sharing a profile hash would make their artifacts falsely comparable"
        );
        let docx = Profile::docx_v0();
        assert!(
            !docx.capabilities.measured_ink_boxes,
            "a DOCX has no ink box"
        );
        assert!(!docx.capabilities.tables, "v2-S2 reads no tables");
        assert!(
            docx.capabilities.spans,
            "a run is a span, and that is the claim this reader makes"
        );
    }

    #[test]
    fn manual_serialize_emits_the_wire_shape_and_the_streaming_route_agrees() {
        // The hand-written Serialize replaced `#[serde(into = "RepresentationWire")]`
        // to stop cloning the whole artifact per serialization; this pins that it
        // still emits exactly the wire form — the parse round-trip re-serializes to
        // the same bytes — and that the streaming canonical route agrees with the
        // Value route on a real artifact.
        let d = simple();
        let bytes = d.to_canonical_bytes().unwrap();
        assert_eq!(
            bytes,
            c14n_bytes(&serde_json::to_value(&d).unwrap()).unwrap()
        );
        let reparsed: DocumentRepresentation = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reparsed.to_canonical_bytes().unwrap(), bytes);
    }

    #[test]
    fn the_fingerprint_is_the_digest_of_the_representation_subtree() {
        let d = simple();
        let v = serde_json::to_value(&d).unwrap();

        // The reader algorithm, written out exactly as an external implementer would do it:
        // canonicalize the `representation` member, hash it, prefix with `sha256:`.
        let subtree = c14n_bytes(&v["representation"]).unwrap();
        let expected = format!("sha256:{}", sha256_hex_bytes(&subtree));

        assert_eq!(v["representation_c14n_sha256"].as_str().unwrap(), expected);
        assert_eq!(d.fingerprint().to_string(), expected);
        d.verify_fingerprint().unwrap();
    }

    #[test]
    fn geometry_is_outside_the_fingerprint() {
        let mut a = simple();
        let before = a.fingerprint().clone();

        // Mutate the one measured box. The fingerprint must not move — `01-CONTRACT.md` §4
        // excludes geometry deliberately, because boxes are the least reliable number in the
        // artifact and identity must not be hostage to them.
        let measured = a
            .geometry
            .iter()
            .filter(|g| matches!(g.presence, GeometryPresence::Measured(_)))
            .count();
        assert!(
            measured > 0,
            "this test is vacuous without a measured box to mutate"
        );
        a.geometry[0].presence = GeometryPresence::Measured(QRect::new(1, 1, 99, 99).unwrap());

        assert_eq!(
            a.recompute_fingerprint().unwrap(),
            before,
            "moving a box must not move the fingerprint"
        );
    }

    #[test]
    fn changing_a_text_origin_does_move_the_fingerprint() {
        // The mirror of the test above, and the reason it is safe: origins ARE identity, so the
        // exclusion must be narrow. Without this, "geometry is excluded" could silently become
        // "locators are excluded" and nobody would notice.
        let mut d = simple();
        let before = d.fingerprint().clone();
        d.representation.nodes[0].native_locator = NativeLocator::Pdf(PdfLocator {
            page: 1,
            origin_x: 7201,
            origin_y: 7200,
            advance: Some(1000),
        });
        assert_ne!(d.recompute_fingerprint().unwrap(), before);
    }

    #[test]
    fn a_tampered_payload_is_detected_by_verify_but_still_parses() {
        let mut d = simple();
        d.representation.nodes[0].text = "goodbye".into();
        let err = d.verify_fingerprint().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(d.fingerprint().as_str()),
            "the failure must name both digests: {msg}"
        );
    }

    #[test]
    fn a_measured_box_outside_its_page_is_refused() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);

        // Negative y0: the reachable case — a baseline within one ascent of the page top.
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, -10, 100, 100).unwrap()),
        );
        let err = DocumentRepresentation::seal(payload(vec![n.clone()], vec![p.clone()]), vec![g])
            .unwrap_err();
        assert!(err.to_string().contains("outside its page"), "{err}");

        // Overrunning the right margin.
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, 0, 30001, 100).unwrap()),
        );
        assert!(seal(vec![n], vec![p], vec![g]).is_err());
    }

    #[test]
    fn typed_absence_is_never_refused_for_being_outside_a_page() {
        // Absence has no coordinates to be outside anything. Guards against a future
        // containment check that reaches for a box that is not there.
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        let d = seal(vec![n], vec![p], vec![g]).unwrap();
        assert_eq!(d.nodes_not_groundable(), 1);
    }

    /// Both flip attacks across the fingerprint seam.
    ///
    /// The sidecar is outside the digest by §4, so these edits do not move it. Without the
    /// declaration cross-check, each produces a record that is fingerprint-valid and flatly
    /// contradicts its own projection.
    #[test]
    fn a_sidecar_that_contradicts_its_declaration_is_refused() {
        // Measured -> Absent: the node would vanish from any projection while the record
        // declares no gap at all.
        let mut d = simple();
        assert!(d.verify_fingerprint().is_ok());
        d.geometry[0].presence = GeometryPresence::Absent(GeometryAbsence::NotReportedByReader);
        assert!(
            d.verify_fingerprint().is_ok(),
            "the premise: geometry is outside the digest, so this edit is invisible to it"
        );
        assert!(
            d.check_structure().is_err(),
            "a node that became ungroundable with no declaration must be refused"
        );

        // Absent -> Measured: a FABRICATED box would be emitted while the record still says the
        // node has no measurable ink.
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let pg = page(&mut alloc, 1);
        let n = node(&mut alloc, &pg.id, 1, 1);
        let mut d = seal(
            vec![n.clone()],
            vec![pg],
            vec![geom(
                &n,
                GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
            )],
        )
        .unwrap();
        d.geometry[0].presence = GeometryPresence::Measured(QRect::new(1, 1, 10, 10).unwrap());
        assert!(d.verify_fingerprint().is_ok());
        assert!(
            d.check_structure().is_err(),
            "a fabricated box beside a standing omission declaration must be refused"
        );

        // And the refusal survives a round trip through the wire, not just in memory.
        let v = serde_json::to_value(&d).unwrap();
        assert!(serde_json::from_value::<DocumentRepresentation>(v).is_err());
    }

    #[test]
    fn a_misaligned_geometry_sidecar_is_refused() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);

        // Too few rows.
        assert!(DocumentRepresentation::seal(
            payload(vec![n.clone()], vec![p.clone()]),
            Vec::new()
        )
        .is_err());

        // Right count, wrong node.
        let wrong = NodeGeometry {
            node: NodeId::from_parts(IdKind::Span, 99),
            presence: GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        };
        assert!(seal(vec![n], vec![p], vec![wrong]).is_err());
    }

    #[test]
    fn a_node_whose_parent_is_not_a_page_is_refused() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let mut n = node(&mut alloc, &p.id, 1, 1);
        n.parent = NodeId::from_parts(IdKind::Page, 77);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        let err = seal(vec![n], vec![p], vec![g]).unwrap_err();
        assert!(err.to_string().contains("not a declared page"), "{err}");
    }

    #[test]
    fn ordinals_must_be_one_based_and_contiguous_within_a_page() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let mut n = node(&mut alloc, &p.id, 1, 1);
        n.ordinal = 2; // skips 1
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        assert!(seal(vec![n], vec![p], vec![g]).is_err());
    }

    #[test]
    fn a_page_index_is_the_documents_number_not_the_array_position() {
        // The distinction that matters the moment a page is quarantined: the second processed
        // page of a document may be page 7, and a citation renders against 7.
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let mut p = page(&mut alloc, 7);
        p.index = 7;
        let n = node(&mut alloc, &p.id, 1, 7);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        let d = seal(vec![n], vec![p], vec![g]).unwrap();
        assert_eq!(d.payload().pages[0].index, 7);
        match &d.payload().nodes[0].native_locator {
            NativeLocator::Pdf(l) => assert_eq!(l.page, 7),
            other => panic!("a text run's address is a glyph locator, not {other:?}"),
        }
    }

    #[test]
    fn an_unknown_artifact_type_is_refused_rather_than_best_effort_parsed() {
        let d = simple();
        let mut v = serde_json::to_value(&d).unwrap();
        v["artifact_type"] = serde_json::Value::String("ethos.parser.something.v9".into());
        let parsed: Result<DocumentRepresentation, _> = serde_json::from_value(v);
        assert!(
            parsed.is_err(),
            "§8: never best-effort parse an unknown shape"
        );
    }

    #[test]
    fn the_top_level_tags_cannot_disagree_with_the_payload() {
        let d = simple();
        let mut v = serde_json::to_value(&d).unwrap();
        v["representation"]["identity"]["schema_version"] =
            serde_json::Value::String("9.9.9".into());
        let parsed: Result<DocumentRepresentation, _> = serde_json::from_value(v);
        assert!(
            parsed.is_err(),
            "a mirror that can disagree is worse than no mirror"
        );
    }

    #[test]
    fn the_artifact_round_trips_through_c14n() {
        let d = simple();
        let bytes = d.to_canonical_bytes().unwrap();
        let back: DocumentRepresentation = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, d);
        assert_eq!(back.fingerprint(), d.fingerprint());
    }

    #[test]
    fn an_unknown_field_anywhere_fails_closed() {
        let d = simple();
        for pointer in [
            "".to_string(),
            "/representation".into(),
            "/representation/source".into(),
            "/representation/processing_run".into(),
            "/representation/processing_run/processor".into(),
        ] {
            let mut v = serde_json::to_value(&d).unwrap();
            let target = if pointer.is_empty() {
                &mut v
            } else {
                v.pointer_mut(&pointer).expect("pointer resolves")
            };
            target
                .as_object_mut()
                .expect("object")
                .insert("future_knob".into(), serde_json::Value::from(1));
            let parsed: Result<DocumentRepresentation, _> = serde_json::from_value(v);
            assert!(
                parsed.is_err(),
                "unknown field at `{pointer}` must be refused"
            );
        }
    }

    #[test]
    fn the_node_kind_wire_spelling_matches_the_grounding_id_pattern() {
        // `ethos.grounding.v1` constrains `kind` to ^[a-z0-9][a-z0-9_-]*$. The representation's
        // spelling becomes the grounding artifact's `kind` verbatim, so it has to satisfy that
        // here rather than be rewritten at the boundary.
        let k = NodeKind::TextRun.as_str();
        assert_eq!(k, "text_run");
        let mut chars = k.chars();
        let first = chars.next().unwrap();
        assert!(first.is_ascii_lowercase() || first.is_ascii_digit());
        assert!(chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'));
    }

    #[test]
    fn no_float_reaches_the_wire() {
        simple().to_canonical_bytes().expect("integers only");
    }
}
