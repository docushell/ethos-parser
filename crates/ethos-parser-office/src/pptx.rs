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

//! `ppt/presentation.xml` and its slides → text runs, at the addresses the package states.
//!
//! # A slide is a part, not a page
//!
//! This is the format where the no-synthesised-pages law is easiest to get wrong in the *other*
//! direction. A DOCX has no page at all and an XLSX's page is a print artefact — but a slide is a
//! real, discrete, addressable thing the package contains, and it is tempting to call it page 3
//! of 55.
//!
//! It is not a page, it is a **part** — the same answer a worksheet got, for the same reason.
//! `p:sldSz` states a slide's size in EMUs, and turning that into a `PageRecord` would put a
//! width and a height on the wire that this engine did not measure anything against; a slide's
//! *position* in `p:sldIdLst` is display order, and an artifact carrying it as `PageRecord.index`
//! would be a citation shaped exactly like a PDF page number. `pages` stays empty and each slide
//! is one part id, so no consumer can mistake one for the other.
//!
//! # `ppt/presentation.xml` names no parts either
//!
//! The third format, the same trap: `<p:sldId id="2147310021" r:id="rId2"/>` carries an id and a
//! relationship id and **no path**. Only `ppt/_rels/presentation.xml.rels` says which part an
//! `r:id` means, and it is resolved rather than guessed — see `opc.rs`, which holds that rule for
//! this reader and `xlsx.rs`. Measured in a real 55-slide deck: `rId13` binds `slides/slide12.xml`, so
//! even the relationship *numbering* does not track the part numbering.
//!
//! # What a slide's text is addressed by
//!
//! `p:sld > p:cSld > p:spTree > p:sp > p:txBody > a:p > a:r > a:t`. The shape carries an id at
//! `p:nvSpPr > p:cNvPr`, and the locator deliberately does **not** address by it — see
//! [`ethos_parser_core::PptxLocator`]: measured across 18 real decks, that id is unique only most of
//! the time, and an address with two answers is the defect this crate spent v2-S3 removing.
//!
//! **Shapes nest.** `p:grpSp` groups appeared on essentially every slide of every real deck this
//! reader was checked against, so a reader that only looked at top-level shapes would silently
//! return a fraction of it. Depth is not tracked for its own sake: any `p:sp`, at any nesting, is
//! a shape and is counted as one.
//!
//! **And every `<p:sp>` the part contains is counted, including ones in a branch this reader does
//! not read.** The locator promises a position in the part's own document order, so a consumer
//! checking it counts elements in the file — not elements this reader chose to keep. Counting
//! only what is read would leave every address after an `<mc:AlternateContent>` one short.
//!
//! # `mc:AlternateContent`: one phrase, one node
//!
//! MCE lets a deck state the same content several ways for consumers of different capability —
//! one or more `<mc:Choice Requires="…">` and an optional `<mc:Fallback>`. A reader that walks
//! all of them emits one displayed phrase at **two or three citable addresses**, which is the
//! mirror of a silent drop and just as wrong. Exactly one branch is read, deterministically: the
//! first `Choice`. The rest are passed over and **counted when they held text** — a branch
//! wrapping only a transition is not an erasure and must not bury a real one in noise.
//!
//! # What is counted rather than read, and why a slide number is not evidence
//!
//! A `<p:graphicFrame>` holds a table, a chart or SmartArt; an `<a:fld>` holds a field. Both are
//! displayed and neither is read, so both are counted (**A14**). The field is the interesting
//! one: an `<a:fld type="slidenum">` contains a **cached** slide number its authoring tool wrote
//! at save time, which goes stale the moment the deck is reordered by a tool that does not
//! recompute it. Reading it would put a number in the evidence that is not on the screen and is
//! shaped exactly like the page index this version exists to refuse. Counting it says the text
//! is there without claiming to know what it says.

use ethos_parser_core::EngineError;
use quick_xml::events::{BytesStart, Event};

use crate::opc::{resolve_target, Relationship};
use crate::xml::{
    attribute_value, cdata_text, check_closed, decode, local_name, new_reader, parse_error,
    resolve_reference,
};

/// The part every presentation keeps its slide list in.
pub const PRESENTATION_PART: &str = "ppt/presentation.xml";

/// The part that binds each `<p:sldId>`'s `r:id` to a package part.
pub const PRESENTATION_RELS_PART: &str = "ppt/_rels/presentation.xml.rels";

/// The folder `ppt/_rels/presentation.xml.rels` resolves its targets against.
const PRESENTATION_FOLDER: &str = "ppt";

/// The relationship type a `<p:sldId>` has when it names a slide.
///
/// The same list also binds the slide master, the notes master, the handout master, the theme and
/// the presentation properties, none of which is a slide.
const SLIDE_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

/// Parts that carry text and that v2-S4 does not read, matched by prefix.
///
/// Masters and layouts are included deliberately. Most of their text is prompt text a viewer
/// never sees — *"Click to edit Master title style"* — but a footer or a standing caption authored
/// on a master **does** appear on every slide, and this reader cannot tell those apart without
/// resolving placeholder inheritance. Counting them over-declares rather than under-declares,
/// which is the direction **A14** asks for: the number tells a caller where to look, and the
/// alternative is a phrase that is on every slide being absent from the record with nothing said.
const UNREAD_TEXT_PART_PREFIXES: [&str; 8] = [
    "ppt/notesSlides/",
    "ppt/notesMasters/",
    "ppt/handoutMasters/",
    "ppt/slideMasters/",
    "ppt/slideLayouts/",
    "ppt/comments",
    "ppt/charts/",
    "ppt/diagrams/",
];

/// One `<a:r>` of a slide, with the addresses the part states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideRun {
    /// 1-based position of the `<p:sp>` within this part, in document order — or, for a run in a
    /// table cell, of the `<p:graphicFrame>` among the part's graphic frames (decision #135).
    pub shape: u32,
    /// The `id` on the shape's `<p:cNvPr>` — a label the file states, not the address.
    pub shape_id: u32,
    /// The `name` on the shape's `<p:cNvPr>`, e.g. `Title 1`.
    pub shape_name: String,
    /// 1-based position of the `<a:p>` within this shape's `<p:txBody>`, or the cell's.
    pub paragraph: u32,
    /// 1-based position of the `<a:r>` within that paragraph.
    pub run: u32,
    /// For a run in a table cell: 1-based `<a:tr>` within the `<a:tbl>`.
    pub row: Option<u32>,
    /// For a run in a table cell: 1-based `<a:tc>` within the `<a:tr>`, its grid column.
    pub column: Option<u32>,
    /// The text, concatenated from this run's `<a:t>` elements, verbatim.
    pub text: String,
}

/// One `<a:tbl>` of a slide: the grid it states (decision #135).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideTable {
    /// 1-based position of its `<p:graphicFrame>` among the part's graphic frames.
    pub frame: u32,
    /// How many `<a:tr>` it holds, less any in an `<mc:AlternateContent>` branch passed over.
    pub rows: u32,
    /// The most `<a:tc>` any of its rows holds, less any in a branch passed over.
    pub columns: u32,
    /// Every `<a:tc>` inside an `<a:tr>`, in document order, but those in a branch passed over:
    /// the file states the same row twice there, and only the branch read is the table's (review).
    pub cells: Vec<SlideTableCell>,
}

/// One `<a:tc>`, with the span its `gridSpan` and `rowSpan` state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlideTableCell {
    /// 1-based `<a:tr>`, as a locator counts it: every one the file holds.
    pub row: u32,
    /// 1-based `<a:tc>` within it, counted so.
    pub column: u32,
    /// Its 1-based row and column in the table's grid: the `<a:tr>` and `<a:tc>` read, one in an
    /// `<mc:AlternateContent>` branch passed over not counted (review).
    pub grid: (u32, u32),
    /// `rowSpan`, 1 when absent.
    pub rowspan: u32,
    /// `gridSpan`, 1 when absent.
    pub colspan: u32,
    /// `hMerge` or `vMerge`: the file says a merge from another cell covers this one.
    pub covered: bool,
}

/// What one slide part yielded, plus what it passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideContent {
    /// The runs, in the part's own document order.
    pub runs: Vec<SlideRun>,
    /// The `<a:tbl>` grids, in document order (decision #135). Their runs are in [`Self::runs`].
    pub tables: Vec<SlideTable>,
    /// Shapes whose text this slice does not read: a `<p:graphicFrame>` holding no table (a chart,
    /// SmartArt, an embedded object) and `<a:fld>` (slide numbers, dates). Counted rather than
    /// dropped in silence (**A14**).
    pub shapes_not_read: u32,
    /// `<mc:AlternateContent>` branches that held text and were passed over — a second or later
    /// `<mc:Choice>`, or an `<mc:Fallback>`.
    ///
    /// Counted **only when the branch actually held an `<a:t>`**: most of them wrap transitions
    /// and timing, and declaring those would bury a real erasure under noise.
    pub alternatives_not_read: u32,
}

/// Parts that hold an **embedded asset** and that no slice reads, matched by prefix.
///
/// A presentation carries more of these than any other OOXML format, and `ppt/media/` is where
/// every one of them lands — the picture behind a slide, the audio on a transition, the video a
/// deck plays. `ppt/embeddings/` holds an embedded workbook or document.
///
/// **Prefix, and the package's own layout, is the whole of the identification** (Anydoc's **A4**).
/// No byte is sniffed and no extension is read: OOXML *states* where a package puts these, the
/// same way it states that a slide is `ppt/slides/slide1.xml`. An `.png` sitting somewhere else is
/// not counted here, and a media part with no extension at all still is.
const EMBEDDED_PART_PREFIXES: [&str; 2] = ["ppt/media/", "ppt/embeddings/"];

/// How many parts of this package hold an embedded asset that this engine does not read.
///
/// Separate from [`unread_text_parts`] because the two answer different questions and their
/// limitation messages make different claims — see
/// `ethos_parser_core::assurance::codes::OFFICE_EMBEDDED_PARTS_NOT_READ`. Nothing here reads, decodes or
/// hashes a byte of the asset: the count is the claim.
pub fn unread_embedded_parts(entry_names: &[String]) -> u32 {
    let matched = entry_names
        .iter()
        .filter(|name| {
            EMBEDDED_PART_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .count();
    crate::declared_len(matched)
}

/// How many parts in this package carry text that this slice does not read.
pub fn unread_text_parts(entry_names: &[String]) -> u32 {
    let matched = entry_names
        .iter()
        .filter(|name| {
            UNREAD_TEXT_PART_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .count();
    crate::declared_len(matched)
}

/// Read the `<p:sldIdLst>` of a `ppt/presentation.xml` into relationship ids, in its own order.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, if a `<p:sldId>` carries no `r:id` — an
/// unbound slide has no part, and choosing one by position is the guess `opc.rs` refuses — or if
/// the presentation lists no slides at all.
pub fn read_slide_refs(part: &[u8]) -> Result<Vec<String>, EngineError> {
    let mut reader = new_reader(part, PRESENTATION_PART)?;
    let mut refs = Vec::new();
    let mut depth: i32 = 0;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, PRESENTATION_PART, &e)),
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                depth += 1;
                if local_name(start.name().as_ref()) == b"sldId" {
                    refs.push(slide_rel_id(&start)?);
                }
            }
            // The form every writer emits, so this is the arm that fires.
            Ok(Event::Empty(start)) => {
                if local_name(start.name().as_ref()) == b"sldId" {
                    refs.push(slide_rel_id(&start)?);
                }
            }
            Ok(Event::End(_)) => depth -= 1,
            Ok(_) => {}
        }
    }

    check_closed(depth, PRESENTATION_PART)?;
    if refs.is_empty() {
        return Err(EngineError::Malformed {
            what: PRESENTATION_PART.into(),
            detail: "this presentation lists no slides. An artifact with no nodes would let a \
                     caller conclude a phrase is absent from a deck nobody read."
                .into(),
        });
    }
    Ok(refs)
}

fn slide_rel_id(start: &BytesStart<'_>) -> Result<String, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: PRESENTATION_PART.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        // `r:id` has local name `id`; `<p:sldId>`'s own `id` attribute is unprefixed and would
        // collide, so the prefix is what tells them apart here — the one place in this crate
        // where dropping it would matter.
        if attribute.key.as_ref() == b"r:id" {
            return attribute_value(&attribute, PRESENTATION_PART);
        }
    }
    Err(EngineError::Malformed {
        what: PRESENTATION_PART.into(),
        detail: "a `<p:sldId>` carries no `r:id`, so the part it names cannot be resolved. \
                 Picking `ppt/slides/slide{n}.xml` by position would attach this slide's place in \
                 the deck to another slide's text."
            .into(),
    })
}

/// Pair every listed slide with the part it names, through the relationship part.
///
/// Returns the slide parts in the presentation's own order, plus the count of listed entries whose
/// relationship is **not** a slide — a declared erasure rather than a silent skip.
///
/// # Errors
///
/// [`EngineError::Malformed`] if an `r:id` matches no relationship, or if two entries resolve to
/// the same part: one part is one slide, and two part ids naming one part is the bijection
/// `check_structure` would refuse later with a message about ids rather than about the deck.
pub fn resolve_slides(
    rel_ids: &[String],
    rels: &[Relationship],
) -> Result<(Vec<String>, u32), EngineError> {
    let mut parts: Vec<String> = Vec::with_capacity(rel_ids.len());
    let mut other_kinds = 0u32;

    for rel_id in rel_ids {
        let Some(rel) = rels.iter().find(|r| &r.id == rel_id) else {
            return Err(EngineError::Malformed {
                what: PRESENTATION_RELS_PART.into(),
                detail: format!(
                    "slide entry `{rel_id}` names a relationship `{PRESENTATION_RELS_PART}` does \
                     not declare. The part a slide lives in is only stated there, so an \
                     unresolved `r:id` is a refusal."
                ),
            });
        };
        if rel.external || rel.kind != SLIDE_REL_TYPE {
            other_kinds = crate::declare(other_kinds, 1);
            continue;
        }
        let part = resolve_target(&rel.target, PRESENTATION_FOLDER);
        if parts.contains(&part) {
            return Err(EngineError::Malformed {
                what: PRESENTATION_RELS_PART.into(),
                detail: format!(
                    "two slide entries resolve to `{part}`. One part is one slide; reading it \
                     twice would put the same text at two addresses."
                ),
            });
        }
        parts.push(part);
    }

    Ok((parts, other_kinds))
}

/// Read one slide part's shapes into the runs that carry text.
///
/// # What becomes a node, and what does not
///
/// A run with no `<a:t>` text — an empty placeholder, a shape that is only a rectangle — is not a
/// node, for the reason a `<w:r>` with no `<w:t>` is not: nothing was erased because there was
/// never a character there. A `<p:graphicFrame>` or an `<a:fld>` **is** counted, because those do
/// hold text this slice does not read.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, a `<p:sp>` carries no `<p:cNvPr id>`, an
/// id is not a number, or two runs in one part would share an address.
pub fn read_slide(part: &[u8], part_name: &str) -> Result<SlideContent, EngineError> {
    let mut reader = new_reader(part, part_name)?;
    let mut runs: Vec<SlideRun> = Vec::new();
    let mut shapes_not_read = 0u32;
    let mut alternatives_not_read = 0u32;

    // The shape currently open, if any: `(id, name)` once `<p:cNvPr>` has been seen.
    let mut shape: Option<(u32, String)> = None;
    let mut in_shape = false;
    // 1-based position of the shape being read. **Advanced for every `<p:sp>` the part contains**,
    // including ones in a branch this reader does not read — see the note on skipping below.
    let mut shape_ordinal: u32 = 0;
    let mut in_body = false;
    let mut paragraph: u32 = 0;
    let mut run_in_paragraph: u32 = 0;
    let mut open_run: Option<SlideRun> = None;
    let mut in_text = false;
    let mut depth: i32 = 0;
    // `(shape, row, column, paragraph, run)`, row and column 0 outside a table: each is 1-based.
    let mut seen: std::collections::BTreeSet<(u32, u32, u32, u32, u32)> = Default::default();
    // Decision #135: the graphic frame open, its position among the part's frames, the table it
    // holds, and the cell being read — `(row, column)` within it.
    let mut in_frame = false;
    let mut frame_ordinal: u32 = 0;
    let mut tables: Vec<SlideTable> = Vec::new();
    let mut table: Option<SlideTable> = None;
    let mut cell: Option<(u32, u32)> = None;
    let mut table_row: u32 = 0;
    let mut table_column: u32 = 0;
    // The same, counting only the rows and cells of branches read: where each sits in the grid.
    let mut grid: (u32, u32) = (0, 0);

    // **Markup Compatibility and Extensibility**, and the two things it can do wrong.
    //
    // `<mc:AlternateContent>` holds one or more `<mc:Choice Requires="…">` and an optional
    // `<mc:Fallback>`, all expressing *the same content* for consumers of different capability.
    // A reader that walks all of them emits one displayed phrase at two citable addresses —
    // duplicated evidence, which is the mirror of a silent drop. So exactly one branch is read:
    // the first `Choice`, deterministically.
    //
    // `alt_taken` is one flag per open `AlternateContent`; `skip_from` is the depth of the
    // subtree currently being passed over, and `skipped_text` records whether that subtree
    // actually held any `<a:t>` — a branch wrapping only transitions or timing is not an erasure
    // and must not inflate the declared count.
    let mut alt_taken: Vec<bool> = Vec::new();
    let mut skip_from: Option<i32> = None;
    let mut skipped_text = false;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, part_name, &e)),
            Ok(Event::Eof) => break,

            Ok(Event::Start(start)) => {
                depth += 1;
                let qualified = start.name();
                let name = local_name(qualified.as_ref());

                match name {
                    b"AlternateContent" => alt_taken.push(false),
                    b"Choice" if skip_from.is_none() => match alt_taken.last_mut() {
                        Some(taken) if !*taken => *taken = true,
                        _ => {
                            skip_from = Some(depth);
                            skipped_text = false;
                        }
                    },
                    b"Fallback" if skip_from.is_none() => {
                        skip_from = Some(depth);
                        skipped_text = false;
                    }
                    _ => {}
                }

                // **The counters advance whether or not this subtree is read.** A locator's
                // `shape` is documented as the position of the `<p:sp>` in the part's own
                // document order, and a consumer checking it counts elements in the file — not
                // elements this reader chose to keep. Counting only what is read would make every
                // address after an `<mc:AlternateContent>` one short, which is a locator that is
                // confidently wrong.
                match name {
                    b"sp" => {
                        shape_ordinal += 1;
                        in_shape = true;
                        shape = None;
                    }
                    // Counted in every branch, as a `<p:sp>` is, so a frame's position is the
                    // file's.
                    b"graphicFrame" => {
                        frame_ordinal += 1;
                        if skip_from.is_none() {
                            in_frame = true;
                            shape = None;
                        }
                    }
                    // Parsed only for a branch being read: a malformed id in a branch nobody
                    // reads should not refuse the deck.
                    b"cNvPr"
                        if (in_shape || in_frame) && shape.is_none() && skip_from.is_none() =>
                    {
                        shape = Some(shape_identity(&start, part_name)?);
                    }
                    // A frame that states no `<p:cNvPr id>` has no label for its runs: its table is
                    // passed over and the frame counted, as every frame was before decision #135.
                    b"tbl"
                        if in_frame
                            && shape.is_some()
                            && skip_from.is_none()
                            && table.is_none() =>
                    {
                        table = Some(SlideTable {
                            frame: frame_ordinal,
                            rows: 0,
                            columns: 0,
                            cells: Vec::new(),
                        });
                        table_row = 0;
                        grid = (0, 0);
                    }
                    b"tr" if table.is_some() => {
                        table_row += 1;
                        table_column = 0;
                        open_row(&mut table, &mut grid, skip_from.is_none());
                    }
                    // A cell outside any row has no row to name: passed over, and counted.
                    b"tc" if table.is_some() && table_row == 0 => {
                        shapes_not_read = crate::declare(shapes_not_read, 1);
                    }
                    b"tc" if table.is_some() => {
                        table_column += 1;
                        cell = Some((table_row, table_column));
                        if skip_from.is_none() {
                            let at = (table_row, table_column);
                            place_cell(&mut table, &mut grid, table_cell(&start, at, part_name)?);
                        }
                    }
                    b"txBody" if in_shape || cell.is_some() => {
                        in_body = true;
                        paragraph = 0;
                    }
                    b"p" if in_body => {
                        paragraph += 1;
                        run_in_paragraph = 0;
                    }
                    b"r" if in_body => {
                        run_in_paragraph += 1;
                        if skip_from.is_none() {
                            let (id, name) =
                                shape.clone().ok_or_else(|| missing_shape_id(part_name))?;
                            open_run = Some(SlideRun {
                                shape: if cell.is_some() {
                                    frame_ordinal
                                } else {
                                    shape_ordinal
                                },
                                shape_id: id,
                                shape_name: name,
                                paragraph,
                                run: run_in_paragraph,
                                row: cell.map(|(r, _)| r),
                                column: cell.map(|(_, c)| c),
                                text: String::new(),
                            });
                        }
                    }
                    b"t" => {
                        if skip_from.is_some() {
                            skipped_text = true;
                        } else if open_run.is_some() {
                            in_text = true;
                        }
                    }
                    // Text this slice does not read, counted rather than dropped: a field's cached
                    // rendering. A frame holding no table is counted where it closes.
                    b"fld" if skip_from.is_none() => {
                        shapes_not_read = crate::declare(shapes_not_read, 1);
                    }
                    _ => {}
                }
            }

            Ok(Event::Empty(start)) => {
                let qualified = start.name();
                let name = local_name(qualified.as_ref());
                match name {
                    // A self-closing element opens and closes at once, so it starts no subtree to
                    // skip — but it is still an element the part contains, and the counters that
                    // address by document order have to see it. `<a:p/>` is what python-pptx and
                    // Apache POI write for a blank line, so a reader that missed it would give
                    // the same deck two different addresses depending on how it was serialized.
                    b"sp" => shape_ordinal += 1,
                    b"p" if in_body => {
                        paragraph += 1;
                        run_in_paragraph = 0;
                    }
                    b"r" if in_body => run_in_paragraph += 1,
                    b"cNvPr"
                        if (in_shape || in_frame) && shape.is_none() && skip_from.is_none() =>
                    {
                        shape = Some(shape_identity(&start, part_name)?);
                    }
                    b"t" if skip_from.is_some() => skipped_text = true,
                    b"graphicFrame" => {
                        frame_ordinal += 1;
                        if skip_from.is_none() {
                            shapes_not_read = crate::declare(shapes_not_read, 1);
                        }
                    }
                    b"fld" if skip_from.is_none() => {
                        shapes_not_read = crate::declare(shapes_not_read, 1);
                    }
                    // A self-closing `<a:tc/>` still holds its grid column.
                    b"tc" if table.is_some() && table_row > 0 => {
                        table_column += 1;
                        if skip_from.is_none() {
                            let at = (table_row, table_column);
                            place_cell(&mut table, &mut grid, table_cell(&start, at, part_name)?);
                        }
                    }
                    b"tr" if table.is_some() => {
                        table_row += 1;
                        table_column = 0;
                        open_row(&mut table, &mut grid, skip_from.is_none());
                    }
                    _ => {}
                }
            }

            Ok(Event::End(end)) => {
                depth -= 1;

                // A skipped subtree ends where it began. Its text, if it had any, is declared.
                if let Some(started_at) = skip_from {
                    if depth + 1 == started_at {
                        skip_from = None;
                        if skipped_text {
                            alternatives_not_read = crate::declare(alternatives_not_read, 1);
                        }
                        skipped_text = false;
                    }
                }

                match local_name(end.name().as_ref()) {
                    b"AlternateContent" => {
                        alt_taken.pop();
                    }
                    b"sp" => {
                        in_shape = false;
                        in_body = false;
                        shape = None;
                    }
                    b"graphicFrame" if in_frame => {
                        in_frame = false;
                        in_body = false;
                        shape = None;
                        cell = None;
                        match table.take() {
                            Some(t) => tables.push(t),
                            // A chart, SmartArt or an embedded object: text this slice does not
                            // read, counted.
                            None => shapes_not_read = crate::declare(shapes_not_read, 1),
                        }
                    }
                    b"tc" => cell = None,
                    b"txBody" => in_body = false,
                    b"t" => in_text = false,
                    b"r" => {
                        if let Some(run) = open_run.take() {
                            if !run.text.is_empty() {
                                let (row, column) = (run.row.unwrap_or(0), run.column.unwrap_or(0));
                                if !seen.insert((run.shape, row, column, run.paragraph, run.run)) {
                                    return Err(EngineError::Malformed {
                                        what: part_name.to_string(),
                                        detail: format!(
                                            "this slide states shape {} paragraph {} run {} more \
                                             than once, so a citation to that address would have \
                                             two answers.",
                                            run.shape, run.paragraph, run.run
                                        ),
                                    });
                                }
                                runs.push(run);
                            }
                        }
                    }
                    _ => {}
                }
            }

            Ok(Event::Text(text)) if in_text => {
                if let Some(run) = open_run.as_mut() {
                    run.text.push_str(decode(&text, part_name)?.as_ref());
                }
            }
            // Matched, not ignored: an unhandled `CData` arm is a silent drop.
            Ok(Event::CData(cdata)) if in_text => {
                if let Some(run) = open_run.as_mut() {
                    run.text.push_str(cdata_text(&cdata, part_name)?.as_ref());
                }
            }
            Ok(Event::GeneralRef(entity)) if in_text => {
                if let Some(run) = open_run.as_mut() {
                    run.text
                        .push_str(&resolve_reference(entity.as_ref(), part_name)?);
                }
            }
            Ok(_) => {}
        }
    }

    check_closed(depth, part_name)?;
    Ok(SlideContent {
        runs,
        tables,
        shapes_not_read,
        alternatives_not_read,
    })
}

/// An `<a:tr>` opening: a row of the table's grid where its branch is read (review).
fn open_row(table: &mut Option<SlideTable>, grid: &mut (u32, u32), read: bool) {
    if let (true, Some(t)) = (read, table.as_mut()) {
        *grid = (grid.0 + 1, 0);
        t.rows = grid.0;
    }
}

/// A read `<a:tc>`, at the next column of the grid's row (review).
fn place_cell(table: &mut Option<SlideTable>, grid: &mut (u32, u32), mut cell: SlideTableCell) {
    if let Some(t) = table.as_mut() {
        grid.1 += 1;
        cell.grid = *grid;
        t.columns = t.columns.max(grid.1);
        t.cells.push(cell);
    }
}

/// An `<a:tc>` with the span it states and whether it says a merge covers it (decision #135), at
/// `(row, column)` as a locator counts them.
fn table_cell(
    start: &BytesStart<'_>,
    (row, column): (u32, u32),
    part_name: &str,
) -> Result<SlideTableCell, EngineError> {
    let mut out = SlideTableCell {
        row,
        column,
        grid: (row, column),
        rowspan: 1,
        colspan: 1,
        covered: false,
    };
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        let value = attribute_value(&attribute, part_name)?;
        // A span that is not a positive number states no merge, so the cell is one slot.
        let span = || {
            value
                .trim()
                .parse::<u32>()
                .ok()
                .filter(|n| *n >= 1)
                .unwrap_or(1)
        };
        match local_name(attribute.key.as_ref()) {
            b"hMerge" | b"vMerge" if matches!(value.trim(), "1" | "true") => out.covered = true,
            b"gridSpan" => out.colspan = span(),
            b"rowSpan" => out.rowspan = span(),
            _ => {}
        }
    }
    Ok(out)
}

fn missing_shape_id(part_name: &str) -> EngineError {
    EngineError::Malformed {
        what: part_name.to_string(),
        detail: "a `<p:sp>` carries text but no `<p:cNvPr id>`. The shape id is how this reader \
                 addresses a run without counting one, and an address it counted would move every \
                 time a shape was added above it."
            .into(),
    }
}

fn shape_identity(start: &BytesStart<'_>, part_name: &str) -> Result<(u32, String), EngineError> {
    let mut id = None;
    let mut name = String::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        match local_name(attribute.key.as_ref()) {
            b"id" => {
                let raw = attribute_value(&attribute, part_name)?;
                id = Some(raw.parse::<u32>().map_err(|_| EngineError::Malformed {
                    what: part_name.to_string(),
                    detail: format!(
                        "`<p:cNvPr id=\"{raw}\">` is not a shape id. Refused rather than \
                         repaired: a repaired id addresses some shape, and nothing here knows \
                         which."
                    ),
                })?);
            }
            b"name" => name = attribute_value(&attribute, part_name)?,
            _ => {}
        }
    }
    match id {
        Some(id) => Ok((id, name)),
        None => Err(missing_shape_id(part_name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLIDE: &str = "ppt/slides/slide1.xml";

    fn slide(body: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><p:sld xmlns:p="p" xmlns:a="a"><p:cSld><p:spTree>{body}</p:spTree></p:cSld></p:sld>"#
        )
    }

    fn shape(id: u32, name: &str, paragraphs: &str) -> String {
        format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{name}"/></p:nvSpPr><p:txBody>{paragraphs}</p:txBody></p:sp>"#
        )
    }

    fn read(body: &str) -> Result<SlideContent, EngineError> {
        read_slide(slide(body).as_bytes(), SLIDE)
    }

    #[test]
    fn a_run_carries_the_shape_id_the_file_states() {
        let out = read(&shape(
            7,
            "Title 1",
            "<a:p><a:r><a:t>Evidence, not extraction.</a:t></a:r></a:p>",
        ))
        .expect("well-formed");
        assert_eq!(out.runs.len(), 1);
        assert_eq!(out.runs[0].shape, 1, "the first shape in the part");
        assert_eq!(out.runs[0].shape_id, 7, "the file's id, carried as a label");
        assert_eq!(out.runs[0].shape_name, "Title 1");
        assert_eq!((out.runs[0].paragraph, out.runs[0].run), (1, 1));
    }

    #[test]
    fn paragraphs_and_runs_are_numbered_within_their_shape() {
        let body = shape(
            2,
            "A",
            "<a:p><a:r><a:t>one</a:t></a:r><a:r><a:t>two</a:t></a:r></a:p><a:p><a:r><a:t>three</a:t></a:r></a:p>",
        ) + &shape(3, "B", "<a:p><a:r><a:t>four</a:t></a:r></a:p>");
        let out = read(&body).expect("well-formed");
        let addr: Vec<_> = out
            .runs
            .iter()
            .map(|r| (r.shape, r.paragraph, r.run, r.text.as_str()))
            .collect();
        assert_eq!(
            addr,
            vec![
                (1, 1, 1, "one"),
                (1, 1, 2, "two"),
                (1, 2, 1, "three"),
                (2, 1, 1, "four"),
            ],
            "numbering restarts inside each shape, as the file nests them"
        );
    }

    /// **Groups appear on essentially every slide of a real deck**, so a reader that only saw
    /// top-level shapes would return a fraction of it and say nothing.
    #[test]
    fn a_shape_inside_a_group_is_still_a_shape() {
        let inner = shape(
            5,
            "Grouped",
            "<a:p><a:r><a:t>inside a group</a:t></a:r></a:p>",
        );
        let body = format!(
            r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="99" name="Group 4"/></p:nvGrpSpPr>{inner}</p:grpSp>"#
        );
        let out = read(&body).expect("well-formed");
        assert_eq!(out.runs.len(), 1);
        assert_eq!(
            out.runs[0].shape, 1,
            "the group itself is not a shape; the `<p:sp>` is"
        );
        assert_eq!(
            out.runs[0].shape_id, 5,
            "and the group's own id 99 is not the run's label either"
        );
        assert_eq!(out.runs[0].text, "inside a group");
    }

    #[test]
    fn a_shape_with_no_text_is_not_a_node() {
        let body = shape(2, "Empty", "<a:p><a:endParaRPr/></a:p>")
            + &shape(3, "Full", "<a:p><a:r><a:t>only this</a:t></a:r></a:p>");
        let out = read(&body).expect("well-formed");
        assert_eq!(out.runs.len(), 1);
        assert_eq!(out.runs[0].text, "only this");
    }

    #[test]
    fn entities_and_cdata_both_survive() {
        let out = read(&shape(
            2,
            "T",
            "<a:p><a:r><a:t>a &amp; b</a:t></a:r><a:r><a:t><![CDATA[c & d]]></a:t></a:r></a:p>",
        ))
        .expect("well-formed");
        assert_eq!(out.runs[0].text, "a & b");
        assert_eq!(
            out.runs[1].text, "c & d",
            "an unmatched CData arm would have made this empty"
        );
    }

    #[test]
    fn a_shape_with_no_id_is_refused_rather_than_counted() {
        let body = r#"<p:sp><p:nvSpPr><p:cNvPr name="No id"/></p:nvSpPr><p:txBody><a:p><a:r><a:t>x</a:t></a:r></a:p></p:txBody></p:sp>"#;
        let error =
            read(body).expect_err("an address this reader counted is not one the file states");
        assert!(error.to_string().contains("no `<p:cNvPr id>`"), "{error}");
    }

    #[test]
    fn a_non_numeric_shape_id_is_refused_rather_than_repaired() {
        let body = r#"<p:sp><p:nvSpPr><p:cNvPr id="2x" name="N"/></p:nvSpPr><p:txBody><a:p><a:r><a:t>x</a:t></a:r></a:p></p:txBody></p:sp>"#;
        assert!(read(body).is_err());
    }

    /// **Two shapes with one `cNvPr id` is a real file**, produced by a real generator and opened
    /// by PowerPoint. Addressing by that id would give one address two answers; addressing by
    /// position gives each run its own, and the id survives as the label it is.
    #[test]
    fn two_shapes_sharing_an_id_still_get_distinct_addresses() {
        let body = shape(4, "A", "<a:p><a:r><a:t>first</a:t></a:r></a:p>")
            + &shape(4, "B", "<a:p><a:r><a:t>second</a:t></a:r></a:p>");
        let out = read(&body).expect("a file PowerPoint opens is a file this reads");
        assert_eq!(out.runs.len(), 2);
        assert_eq!((out.runs[0].shape, out.runs[1].shape), (1, 2));
        assert_eq!((out.runs[0].shape_id, out.runs[1].shape_id), (4, 4));
    }

    /// The uniqueness guard still exists, and this is what would trip it: a part whose paragraph
    /// or run numbering ever repeated inside one shape.
    #[test]
    fn one_address_means_one_run() {
        let out = read(&shape(
            2,
            "T",
            "<a:p><a:r><a:t>a</a:t></a:r><a:r><a:t>b</a:t></a:r></a:p>",
        ))
        .expect("well-formed");
        let addresses: std::collections::BTreeSet<_> = out
            .runs
            .iter()
            .map(|r| (r.shape, r.paragraph, r.run))
            .collect();
        assert_eq!(addresses.len(), out.runs.len());
    }

    /// **The address is a position in the part, not a position among the shapes this reader
    /// kept.** A shape after an `<mc:AlternateContent>` must still be numbered as the file numbers
    /// it — counting only what is read would make every later address one short, which is a
    /// locator that is confidently wrong.
    #[test]
    fn a_skipped_branch_still_advances_the_document_order_count() {
        let alt = format!(
            r#"<mc:AlternateContent xmlns:mc="mc"><mc:Choice Requires="a14">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>"#,
            shape(3, "B", "<a:p><a:r><a:t>B</a:t></a:r></a:p>"),
            shape(4, "Bfb", "<a:p><a:r><a:t>Bfb</a:t></a:r></a:p>"),
        );
        let body = shape(2, "A", "<a:p><a:r><a:t>A</a:t></a:r></a:p>")
            + &alt
            + &shape(5, "D", "<a:p><a:r><a:t>D</a:t></a:r></a:p>");
        let out = read(&body).expect("well-formed");

        let seen: Vec<_> = out
            .runs
            .iter()
            .map(|r| (r.text.as_str(), r.shape))
            .collect();
        assert_eq!(
            seen,
            vec![("A", 1), ("B", 2), ("D", 4)],
            "the part contains four `<p:sp>`; `D` is the fourth, and the third is the skipped one"
        );
        assert_eq!(out.alternatives_not_read, 1, "the skipped branch held text");
    }

    /// The same, one level down: a paragraph after a skipped branch keeps the file's numbering.
    #[test]
    fn a_skipped_branch_inside_a_shape_still_advances_the_paragraph_count() {
        let body = shape(
            2,
            "T",
            r#"<a:p><a:r><a:t>one</a:t></a:r></a:p><mc:AlternateContent xmlns:mc="mc"><mc:Choice Requires="a14"><a:p><a:r><a:t>two</a:t></a:r></a:p></mc:Choice><mc:Fallback><a:p><a:r><a:t>twofb</a:t></a:r></a:p></mc:Fallback></mc:AlternateContent><a:p><a:r><a:t>four</a:t></a:r></a:p>"#,
        );
        let out = read(&body).expect("well-formed");
        assert_eq!(
            out.runs
                .iter()
                .map(|r| (r.text.as_str(), r.paragraph))
                .collect::<Vec<_>>(),
            vec![("one", 1), ("two", 2), ("four", 4)]
        );
    }

    /// **A self-closing `<a:p/>` is a paragraph.** python-pptx and Apache POI write one for a
    /// blank line, and it arrives as its own event — so a reader that only counted `Start` would
    /// give the same deck two different addresses depending on how it was serialized.
    #[test]
    fn a_self_closing_paragraph_still_counts() {
        let out = read(&shape(
            2,
            "T",
            "<a:p><a:r><a:t>First</a:t></a:r></a:p><a:p/><a:p><a:r><a:t>Third</a:t></a:r></a:p>",
        ))
        .expect("well-formed");
        assert_eq!(
            out.runs
                .iter()
                .map(|r| (r.text.as_str(), r.paragraph))
                .collect::<Vec<_>>(),
            vec![("First", 1), ("Third", 3)],
            "`Third` is in the third `<a:p>`, however the blank one was written"
        );

        // The identical infoset written the long way must give the identical address.
        let long_form = read(&shape(
            2,
            "T",
            "<a:p><a:r><a:t>First</a:t></a:r></a:p><a:p></a:p><a:p><a:r><a:t>Third</a:t></a:r></a:p>",
        ))
        .expect("well-formed");
        assert_eq!(out.runs, long_form.runs, "serialization is not an address");
    }

    /// **Only the first `<mc:Choice>` is read.** MCE lets a deck express one phrase several ways
    /// for consumers of different capability; reading every branch emits it at several addresses,
    /// which is the mirror of a silent drop.
    #[test]
    fn only_the_first_choice_of_an_alternate_content_is_read() {
        let body = format!(
            r#"<mc:AlternateContent xmlns:mc="mc"><mc:Choice Requires="a14">{}</mc:Choice><mc:Choice Requires="v">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>"#,
            shape(2, "Modern", "<a:p><a:r><a:t>Q3 revenue</a:t></a:r></a:p>"),
            shape(3, "VML", "<a:p><a:r><a:t>Q3 revenue</a:t></a:r></a:p>"),
            shape(4, "Raster", "<a:p><a:r><a:t>Q3 revenue</a:t></a:r></a:p>"),
        );
        let out = read(&body).expect("well-formed");
        assert_eq!(out.runs.len(), 1, "one phrase, one node: {:?}", out.runs);
        assert_eq!(out.runs[0].shape_name, "Modern");
        assert_eq!(
            out.alternatives_not_read, 2,
            "and the two that were passed over are declared, not dropped in silence"
        );
    }

    /// A branch that wraps no text is not an erasure and must not inflate the count.
    #[test]
    fn an_alternate_content_holding_no_text_declares_nothing() {
        let body = shape(2, "T", "<a:p><a:r><a:t>read</a:t></a:r></a:p>")
            + r#"<mc:AlternateContent xmlns:mc="mc"><mc:Choice Requires="p14"><p:transition/></mc:Choice><mc:Fallback><p:transition/></mc:Fallback></mc:AlternateContent>"#;
        let out = read(&body).expect("well-formed");
        assert_eq!(out.runs.len(), 1);
        assert_eq!(
            out.alternatives_not_read, 0,
            "transitions and timing are the common case and carry no text"
        );
    }

    /// `<mc:Fallback>` duplicates the `<mc:Choice>` a modern consumer renders. Reading both would
    /// emit the same text twice at two addresses — the mirror of a silent drop.
    #[test]
    fn an_alternate_content_fallback_is_not_read_twice() {
        let body = format!(
            r#"<mc:AlternateContent xmlns:mc="mc"><mc:Choice Requires="a">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>"#,
            shape(2, "Modern", "<a:p><a:r><a:t>once</a:t></a:r></a:p>"),
            shape(3, "Legacy", "<a:p><a:r><a:t>once</a:t></a:r></a:p>"),
        );
        let out = read(&body).expect("well-formed");
        assert_eq!(out.runs.len(), 1, "the Choice only: {:?}", out.runs);
        assert_eq!(out.runs[0].shape_name, "Modern");
    }

    #[test]
    fn a_truncated_slide_is_refused_rather_than_read_as_far_as_it_goes() {
        assert!(read_slide(b"<p:sld><p:cSld><p:spTree><p:sp>", SLIDE).is_err());
    }

    #[test]
    fn text_this_slice_does_not_read_is_counted() {
        let body = shape(2, "T", "<a:p><a:r><a:t>read</a:t></a:r></a:p>")
            + r#"<p:graphicFrame><a:tbl><a:tc><a:txBody><a:p><a:r><a:t>in a table</a:t></a:r></a:p></a:txBody></a:tc></a:tbl></p:graphicFrame>"#
            + r#"<p:sp><p:nvSpPr><p:cNvPr id="9" name="Num"/></p:nvSpPr><p:txBody><a:p><a:fld id="x" type="slidenum"><a:t>3</a:t></a:fld></a:p></p:txBody></p:sp>"#;
        let out = read(&body).expect("well-formed");
        assert_eq!(out.runs.len(), 1, "only the plain shape's run is read");
        assert_eq!(out.runs[0].text, "read");
        assert_eq!(
            out.shapes_not_read, 2,
            "the table and the field are declared, not dropped"
        );
        assert!(
            !out.runs.iter().any(|r| r.text.contains("in a table")),
            "a table cell's text is not flattened into the run sequence"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The slide list, and the relationship part
    // ---------------------------------------------------------------------------------------

    #[test]
    fn the_slide_list_is_read_in_the_presentation_s_own_order() {
        let refs = read_slide_refs(
            br#"<p:presentation xmlns:r="r"><p:sldIdLst>
                  <p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId7"/>
                </p:sldIdLst></p:presentation>"#,
        )
        .expect("well-formed");
        assert_eq!(refs, vec!["rId2", "rId7"]);
    }

    /// `<p:sldId>` has both an `id` and an `r:id`, and only the prefix tells them apart.
    #[test]
    fn the_slide_s_own_id_is_not_mistaken_for_its_relationship_id() {
        let refs = read_slide_refs(
            br#"<p:presentation xmlns:r="r"><p:sldIdLst><p:sldId id="256" r:id="rId9"/></p:sldIdLst></p:presentation>"#,
        )
        .expect("well-formed");
        assert_eq!(refs, vec!["rId9"], "`256` is the slide's id, not a rel id");
    }

    #[test]
    fn a_slide_entry_with_no_relationship_id_is_refused() {
        assert!(read_slide_refs(
            br#"<p:presentation><p:sldIdLst><p:sldId id="256"/></p:sldIdLst></p:presentation>"#
        )
        .is_err());
    }

    #[test]
    fn a_presentation_that_lists_no_slides_is_refused() {
        let error = read_slide_refs(br#"<p:presentation><p:sldIdLst/></p:presentation>"#)
            .expect_err("an empty deck would look like a deck with no words");
        assert!(error.to_string().contains("lists no slides"), "{error}");
    }

    fn rel(id: &str, kind: &str, target: &str) -> Relationship {
        Relationship {
            id: id.into(),
            kind: kind.into(),
            target: target.into(),
            external: false,
        }
    }

    #[test]
    fn slides_are_bound_by_relationship_id_not_by_position() {
        // Deliberately crossed: position and `r:id` disagree, and the `r:id` decides.
        let rels = vec![
            rel("rId2", SLIDE_REL_TYPE, "slides/slide9.xml"),
            rel("rId7", SLIDE_REL_TYPE, "slides/slide1.xml"),
        ];
        let (parts, others) =
            resolve_slides(&["rId2".into(), "rId7".into()], &rels).expect("both resolve");
        assert_eq!(others, 0);
        assert_eq!(
            parts,
            vec!["ppt/slides/slide9.xml", "ppt/slides/slide1.xml"],
            "the first slide of the deck is slide9.xml, and only the rels say so"
        );
    }

    #[test]
    fn a_slide_whose_relationship_is_missing_is_a_named_refusal() {
        let error = resolve_slides(&["rId4".into()], &[]).expect_err("an unresolved `r:id`");
        assert!(error.to_string().contains("rId4"), "{error}");
    }

    #[test]
    fn a_listed_entry_that_is_not_a_slide_is_counted_rather_than_read() {
        let rels = vec![rel(
            "rId1",
            "http://example/notesSlide",
            "notesSlides/n1.xml",
        )];
        let (parts, others) = resolve_slides(&["rId1".into()], &rels).expect("resolves");
        assert!(parts.is_empty());
        assert_eq!(others, 1, "declared, not silently dropped");
    }

    #[test]
    fn two_entries_resolving_to_one_part_are_refused() {
        let rels = vec![
            rel("rId1", SLIDE_REL_TYPE, "slides/slide1.xml"),
            rel("rId2", SLIDE_REL_TYPE, "/ppt/slides/slide1.xml"),
        ];
        let error = resolve_slides(&["rId1".into(), "rId2".into()], &rels)
            .expect_err("one part is one slide");
        assert!(error.to_string().contains("two slide entries"), "{error}");
    }

    #[test]
    fn unread_text_parts_are_counted_by_prefix() {
        let names: Vec<String> = [
            "ppt/presentation.xml",
            "ppt/slides/slide1.xml",
            "ppt/notesSlides/notesSlide1.xml",
            "ppt/slideLayouts/slideLayout1.xml",
            "ppt/slideMasters/slideMaster1.xml",
            "ppt/theme/theme1.xml",
            "ppt/media/image1.png",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // A theme and an image carry no text a citation could land in.
        assert_eq!(unread_text_parts(&names), 3);
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// v2-S9's adversarial review reproduced the wrap in EPUB — a 31 MB publication exiting 0
    /// while declaring 51,032,704 passed-over runs against a true 4,346,000,000 — and the same
    /// plain `+=` was here, both on this reader's per-slide counters and on the fold in
    /// [`crate::read_pptx`] that sums them. A deck's slide count is bounded by nothing but its
    /// central directory, so the fold is the reachable half.
    #[test]
    fn a_slide_erasure_count_saturates_rather_than_wrapping() {
        let out = read("<p:graphicFrame/><p:graphicFrame/>").expect("well-formed");
        assert_eq!(out.shapes_not_read, 2, "both frames are counted");

        // Three slides' worth of this slide's count, from two below the ceiling, is four over it.
        let mut folded = u32::MAX - 2;
        for _ in 0..3 {
            folded = crate::declare(folded, out.shapes_not_read);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );
    }

    // Decision #135: a frame's `<a:tbl>`.

    fn frame(id: u32, tbl: &str) -> String {
        format!(
            r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{id}" name="Table"/></p:nvGraphicFramePr><a:graphic><a:graphicData><a:tbl>{tbl}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
        )
    }

    #[test]
    fn a_table_cell_run_is_addressed_by_frame_row_and_column() {
        let tbl = "<a:tr><a:tc><a:txBody><a:p><a:r><a:t>a</a:t></a:r></a:p></a:txBody></a:tc>\
                   <a:tc gridSpan=\"2\"><a:txBody><a:p><a:r><a:t>b</a:t></a:r></a:p></a:txBody></a:tc>\
                   <a:tc hMerge=\"1\"/></a:tr>\
                   <a:tr><a:tc/><a:tc><a:txBody><a:p/><a:p><a:r><a:t>c</a:t></a:r></a:p></a:txBody></a:tc></a:tr>";
        let body = shape(2, "T", "<a:p><a:r><a:t>t</a:t></a:r></a:p>")
            + r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="9" name="Chart"/></p:nvGraphicFramePr><a:graphic><a:graphicData><c:chart/></a:graphicData></a:graphic></p:graphicFrame>"#
            + &frame(4, tbl)
            + &shape(3, "U", "<a:p><a:r><a:t>u</a:t></a:r></a:p>");
        let out = read(&body).expect("well-formed");
        let addr: Vec<_> = out
            .runs
            .iter()
            .map(|r| {
                (
                    r.text.as_str(),
                    r.shape,
                    r.row,
                    r.column,
                    r.paragraph,
                    r.run,
                )
            })
            .collect();
        assert_eq!(
            addr,
            vec![
                ("t", 1, None, None, 1, 1),
                ("a", 2, Some(1), Some(1), 1, 1),
                ("b", 2, Some(1), Some(2), 1, 1),
                ("c", 2, Some(2), Some(2), 2, 1),
                // A shape after the frames keeps the position it had: frames are counted apart.
                ("u", 2, None, None, 1, 1),
            ]
        );
        assert_eq!(
            out.runs[2].shape_id, 4,
            "a cell's run carries its frame's id"
        );
        assert_eq!(
            out.tables,
            vec![SlideTable {
                frame: 2,
                rows: 2,
                columns: 3,
                cells: [
                    (1, 1, 1, 1, false),
                    (1, 2, 1, 2, false),
                    (1, 3, 1, 1, true),
                    (2, 1, 1, 1, false),
                    (2, 2, 1, 1, false),
                ]
                .map(|(row, column, rowspan, colspan, covered)| SlideTableCell {
                    row,
                    column,
                    grid: (row, column),
                    rowspan,
                    colspan,
                    covered
                })
                .to_vec(),
            }]
        );
        // The first frame holds no table, so it is the one shape counted as unread.
        assert_eq!(out.shapes_not_read, 1);
    }

    /// A cell before any row names no row: passed over and counted, never row 0 or a wrapped one.
    #[test]
    fn a_cell_outside_any_row_is_counted_not_placed() {
        let tbl = "<a:tc><a:txBody><a:p><a:r><a:t>orphan</a:t></a:r></a:p></a:txBody></a:tc>\
                   <a:tr><a:tc><a:txBody><a:p><a:r><a:t>x</a:t></a:r></a:p></a:txBody></a:tc></a:tr>";
        let out = read(&frame(4, tbl)).expect("well-formed");
        let texts: Vec<_> = out.runs.iter().map(|r| (r.text.as_str(), r.row)).collect();
        assert_eq!(texts, vec![("x", Some(1))]);
        assert_eq!(out.tables[0].cells.len(), 1);
        assert_eq!(out.shapes_not_read, 1);
    }
}
