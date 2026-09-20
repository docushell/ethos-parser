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

//! `content.xml` → cells, at the positions the part states (v2-S6).
//!
//! # The container was free and the vocabulary was not, again
//!
//! v2-S5 measured what an OpenDocument *text* document cost and found that only the ZIP container
//! transferred from OOXML. This slice is the other half of that finding. An `.ods` shares the whole
//! of an `.odt`'s **container** — the stored-first `mimetype`, the manifest, the fixed
//! `content.xml` name — and shares its **text engine**, because a cell's characters are `<text:p>`
//! characters read under exactly ODF's whitespace rule and exactly [`crate::odt`]'s allowlist. What
//! does not transfer is everything above the paragraph: `<table:table-cell>` is a different atom
//! with a different address, which is why this is a slice rather than a flag on `odt.rs`.
//!
//! **The allowlist and the namespace resolution are `odt.rs`'s own**, imported rather than
//! restated. Two copies of a rule whose entire content is *which twelve element names are the
//! sentence* is the shape that drifts, and the drift is silent: a rule that gains
//! `<text:page-count>` in one reader and not the other puts a word processor's page arithmetic into
//! one artifact and not the other, with nothing failing anywhere.
//!
//! # The address this format does not write
//!
//! SpreadsheetML writes `<c r="B12">` and [`ethos_parser_core::XlsxLocator`] reads it, and says why
//! counting would be wrong there: a sheet's rows are sparse, so a counter would give a cell a
//! different address than the file gives it.
//!
//! **OpenDocument writes no address at all** — no row number, no column letter, nowhere. A cell's
//! position is where it sits among its siblings, and a run of identical cells is compressed into
//! one element carrying `table:number-columns-repeated="n"`. So the question here is not "read or
//! count", it is *what does this file state position with*, and the answer is document order plus
//! those repeat counts. Honouring them is reading — the compressed run is the file saying "and n
//! more of these". Ignoring them would put every cell after the first compressed gap at the wrong
//! column, which is a gap every real producer writes on every row.
//!
//! The one component ODF does write down is `table:name`, and it is carried verbatim.
//!
//! # What is not read, and is counted (**A14**)
//!
//! - `<table:shapes>` — a table's page-anchored drawings. Their text is in the document and has no
//!   cell to belong to, so it is declared rather than dropped.
//! - The four regions `odt.rs` passes over, unchanged: a note body, a comment, a tracked-changes
//!   record, and a second `<draw:text-box>` inside one `<draw:frame>`.
//! - A block that held characters while no cell was open. Not reachable in a conforming
//!   spreadsheet — counted anyway, because "not reachable" is a claim about producers and v2-S4
//!   measured that claim to be false in a neighbouring format.
//!
//! # What this reader was and was not measured against
//!
//! **No corpus of real `.ods` files was available, and no ODF producer was either.** The same
//! statement v2-S5 had to make, repeated rather than quietly inherited: every rule here is read off
//! the OpenDocument specification and pinned against packages this repository authors byte by byte.
//!
//! The one thing v2-S5 explicitly owed this slice — whether its frame-alternative rule survives a
//! document that actually nests two renditions, which v2-S5 could not test because it had no such
//! file — **is measured here**, on authored ODS XML. ODF puts `<draw:frame>` in the `<text:p>`
//! content model for every document type and a spreadsheet cell's content is `<text:p>`, so the
//! construct is expressible in a conforming `.ods` and the rule is exercised rather than assumed.

use ethos_parser_core::{EngineError, OdfValueType};
use quick_xml::events::{BytesStart, Event};
use quick_xml::NsReader;

use crate::odt::{
    self, classify, namespace_of, push_source, push_stated, spaces, Element, OpenBlock, Skip,
    MAX_BLOCK_NESTING, NS_OFFICE,
};
use crate::xml::{
    cdata_text, check_closed, decode, local_name, new_ns_reader, parse_error_at, resolve_reference,
    resolved_attribute,
};

/// The OpenDocument **table** namespace, where a spreadsheet's structure lives.
///
/// Resolved rather than suffix-matched, for [`crate::odt`]'s reason and one this format sharpens:
/// `table` and `name` are local names other vocabularies use, and every element and attribute below
/// feeds an **address**.
const NS_TABLE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:table:1.0";

/// The media type an OpenDocument **spreadsheet** declares.
pub const ODS_MEDIA_TYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";

/// A ceiling on a repeat this reader will **materialise**, per axis.
///
/// Only ever reached by a repeat carrying text. An empty repeated cell costs one addition, so the
/// `table:number-columns-repeated="16384"` every producer writes for a row's trailing blanks is
/// free — which is why the cap is worth stating carefully rather than setting low. A text-bearing
/// repeat past this is a named refusal, on `zip.rs`'s rule that a bomb is "a named refusal rather
/// than an out-of-memory kill".
const MAX_MATERIALISED_REPEAT: u32 = 4096;

/// A ceiling on the cells one `content.xml` may yield.
///
/// The amplification a spreadsheet has and a text document does not: one text-bearing cell with
/// `table:number-columns-repeated="4096"` inside a row with `table:number-rows-repeated="4096"` is
/// under a hundred source bytes and sixteen million nodes.
const MAX_CELLS: usize = 1_000_000;

/// What one element of a spreadsheet's `content.xml` means to this reader.
///
/// Everything that is not spreadsheet structure falls through to [`crate::odt::classify`] — the
/// same allowlist, the same namespaces, the same names — so there is exactly one answer in this
/// crate to *"are these characters the sentence?"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Structure {
    /// `table:table` — a sheet, and the only thing here carrying a name this reader addresses by.
    Table,
    /// `table:table-row`.
    Row,
    /// `table:table-cell`.
    Cell,
    /// `table:covered-table-cell` — the placeholder a merge leaves behind.
    ///
    /// It **occupies its columns** and displays nothing, so it advances the cursor and yields no
    /// node. Treating it as absent would shift every cell to its right.
    CoveredCell,
    /// `table:shapes` — a table's page-anchored drawings. Text with no cell to belong to.
    Shapes,
    /// Not spreadsheet structure: whatever `odt.rs` says it is.
    Odf(Element),
}

fn structure(namespace: Option<&[u8]>, local: &[u8]) -> Structure {
    match (namespace, local) {
        (Some(NS_TABLE), b"table") => Structure::Table,
        (Some(NS_TABLE), b"table-row") => Structure::Row,
        (Some(NS_TABLE), b"table-cell") => Structure::Cell,
        (Some(NS_TABLE), b"covered-table-cell") => Structure::CoveredCell,
        (Some(NS_TABLE), b"shapes") => Structure::Shapes,
        _ => Structure::Odf(classify(namespace, local)),
    }
}

/// One cell that carries text, with the address the part states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The document's own name for the table, from `table:name`, entities resolved.
    pub table: String,
    /// 1-based position of the row within its table, counting `table:number-rows-repeated`.
    pub row: u32,
    /// 1-based position of the cell within its row, counting `table:number-columns-repeated`.
    pub column: u32,
    /// The cell's displayed text: its blocks under ODF's whitespace rule, joined by a line feed
    /// because a second paragraph in a cell is a second displayed line.
    pub text: String,
    /// What `office:value-type` declares the stored value to be.
    pub value_type: OdfValueType,
    /// Whether the cell carries a `table:formula`.
    ///
    /// **Not the formula's source.** This engine has no evaluator and puts none in the record; the
    /// flag says the text is a display form the producer cached rather than one a person typed.
    pub formula: bool,
}

/// What one spreadsheet `content.xml` yielded, plus what it passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheets {
    /// The cells that carry text, in the part's own document order.
    pub cells: Vec<Cell>,
    /// Regions of the part that held body text and were not read — [`crate::odt`]'s four, plus
    /// `<table:shapes>`.
    pub regions_not_read: u32,
    /// Blocks that contained a foreign subtree holding characters: an image's title, an embedded
    /// object's base64, a field's cached page number.
    pub foreign_text_not_read: u32,
    /// Blocks that held characters while no cell was open.
    pub text_outside_a_cell: u32,
}

/// Whether these bytes are an OpenDocument **spreadsheet** package, read from the bytes.
///
/// The three questions [`crate::odt::is_odt`] asks, against a different declared type — and exact
/// rather than prefixed, so `…opendocument.spreadsheet-template` (an `.ots`) is not claimed.
pub fn is_ods(bytes: &[u8]) -> bool {
    odt::declared_media_type(bytes).is_some_and(|declared| declared == ODS_MEDIA_TYPE)
}

/// A table being read: its name and the row its cursor has reached.
struct OpenTable {
    name: String,
    /// The row index the next `<table:table-row>` occupies. 1-based.
    row_cursor: u32,
    from_depth: i32,
}

/// A row being read, held until its end so a `table:number-rows-repeated` can be materialised.
struct OpenRow {
    /// The order this row **opened** in, which is document order.
    ///
    /// Rows are materialised when they *close*, so a table nested inside a cell — legal ODF, since
    /// a `<draw:text-box>` may hold a `<table:table>` — finishes before the row containing it and
    /// would otherwise be emitted first. `odt.rs` sorts its paragraphs by ordinal for exactly this
    /// reason, and `ODS_READING_ORDER_RULE_V1` promises document order, so it has to be true.
    seq: u32,
    first: u32,
    repeat: u32,
    /// The column index the next cell occupies. 1-based.
    column_cursor: u32,
    done: Vec<DoneCell>,
    from_depth: i32,
}

/// A cell being read.
struct OpenCell {
    column: u32,
    repeat: u32,
    value_type: OdfValueType,
    formula: bool,
    /// The cell's blocks, each with the sequence number it **opened** at.
    ///
    /// **Not a string joined as blocks close.** ODF nests blocks — a `<draw:frame>` anchored in a
    /// cell's paragraph holds paragraphs of its own — and an inner block closes before the outer
    /// one that contains it, so appending on close puts the frame's line ahead of the cell's own.
    /// `odt.rs` sorts its paragraphs by ordinal for exactly this reason; a cell has no ordinal on
    /// the wire, so the order is kept here and spent at the join.
    blocks: Vec<(u32, String)>,
    /// Whether character data reached this cell without being inside one of its blocks.
    ///
    /// ODF puts a cell's displayed text in `<text:p>` children, so anything arriving outside one is
    /// inside something else — a foreign element, an unrecognised wrapper — and is not the cell's
    /// value. `<xhtml:table><xhtml:p>` directly inside a cell is the conforming case, and without
    /// this flag its characters were neither read nor counted, which is a silent drop.
    foreign_text: bool,
    from_depth: i32,
}

/// A finished cell, waiting for its row to say how many times the row itself occurs.
struct DoneCell {
    column: u32,
    repeat: u32,
    value_type: OdfValueType,
    formula: bool,
    text: String,
}

/// Read a spreadsheet `content.xml` into the cells that carry text.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, if the part ends with elements still open,
/// if a `<table:table>` carries no `table:name`, if two tables carry the same one, or if a repeat
/// attribute is not a positive count. [`EngineError::ResourceLimit`] if blocks nest past
/// [`crate::odt::MAX_BLOCK_NESTING`], if a text-bearing repeat exceeds [`MAX_MATERIALISED_REPEAT`],
/// or if the part yields more than [`MAX_CELLS`] cells.
pub fn read_content(part: &[u8]) -> Result<Sheets, EngineError> {
    let mut reader = new_ns_reader(part, odt::CONTENT_PART)?;
    let mut cells: Vec<(u32, Cell)> = Vec::new();
    let mut regions_not_read = 0u32;
    let mut foreign_text_not_read = 0u32;
    let mut text_outside_a_cell = 0u32;
    let mut named: Vec<String> = Vec::new();

    // Three stacks rather than three slots. ODF lets a cell hold a text box that holds a table, so
    // the shapes genuinely nest — and a slot would let an inner `</table:table>` close the outer
    // one, which is the class of defect v2-S5's own review found twice in its block handling.
    let mut tables: Vec<OpenTable> = Vec::new();
    let mut rows: Vec<OpenRow> = Vec::new();
    let mut open_cells: Vec<OpenCell> = Vec::new();

    let mut open: Vec<OpenBlock> = Vec::new();
    let mut skips: Vec<Skip> = Vec::new();
    // Parallel to `skips`: whether this region counts **bare** character data, or only characters
    // inside one of its own blocks.
    //
    // `odt.rs`'s regions want the stricter rule and say why — every note carries a
    // `<text:note-citation>` and every comment a `<dc:creator>`, so counting bare characters would
    // declare an erasure for a comment with no body. A **drawing** has no such metadata: an
    // image's `<svg:title>` is content, sits in no `<text:p>`, and under the strict rule was
    // neither read nor counted. That is the silent drop this whole allowlist exists to prevent, so
    // drawing regions count bare characters and note-shaped regions do not.
    let mut skip_counts_bare: Vec<bool> = Vec::new();
    let mut frame_taken: Vec<bool> = Vec::new();
    let mut depth: i32 = 0;
    let mut text_bytes: usize = 0;
    let mut opened: u32 = 0;
    let mut rows_opened: u32 = 0;

    loop {
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => {
                return Err(parse_error_at(
                    reader.buffer_position(),
                    odt::CONTENT_PART,
                    &e,
                ))
            }
        };
        let namespace = namespace_of(&resolved).map(|ns| ns.to_vec());
        let namespace = namespace.as_deref();

        match event {
            Event::Eof => break,

            Event::Start(start) => {
                depth += 1;
                let qualified = start.name();
                let element = structure(namespace, local_name(qualified.as_ref()));

                let entering_region = match element {
                    // Text with no cell to belong to, passed over on a footnote's terms.
                    Structure::Shapes | Structure::Odf(Element::Region) => true,
                    // **And a frame anchored in a cell is the same object with a different
                    // anchor.** A `<draw:frame>` floats over the sheet; its words are the shape's,
                    // not the cell's value, and this reader's atom is the cell — so there is no
                    // address for them. Merging them into the cell they are anchored in was this
                    // reader's first behaviour and it is a **mis-attribution**: the artifact would
                    // put a phrase at `Sheet1 row 1 column 2` that a person reading the document
                    // does not find there. Declared instead, which is the same answer
                    // `<table:shapes>` already gets one line above.
                    //
                    // A `<table:table>` nested inside a frame is covered by the same decision. It
                    // is an alternative rendition of the framed object, and minting it as a
                    // first-class sheet address gave a citation a table no spreadsheet consumer
                    // can resolve.
                    Structure::Odf(Element::Frame) => true,
                    // The frame's one slot, claimed inside the frame's own passed-over region so
                    // a second rendition is a **second** declared erasure rather than part of the
                    // first. That difference is what makes the rule measurable at all.
                    Structure::Odf(Element::TextBox) => match frame_taken.last_mut() {
                        Some(taken) if !*taken => {
                            *taken = true;
                            false
                        }
                        _ => true,
                    },
                    _ => false,
                };
                if entering_region {
                    skips.push(Skip {
                        from_depth: depth,
                        held_text: false,
                        block_depth: 0,
                    });
                    skip_counts_bare.push(!matches!(element, Structure::Odf(Element::Region)));
                }

                // Pushed before the skip test below can suppress it: a frame's slot is per frame,
                // and frames nest.
                if element == Structure::Odf(Element::Frame) {
                    frame_taken.push(false);
                }

                match element {
                    Structure::Table if skips.is_empty() => {
                        let name = table_name(&reader, &start)?;
                        if named.iter().any(|seen| seen == &name) {
                            return Err(duplicate_table(&name));
                        }
                        named.push(name.clone());
                        tables.push(OpenTable {
                            name,
                            row_cursor: 1,
                            from_depth: depth,
                        });
                    }
                    Structure::Row if skips.is_empty() => {
                        let repeat = repeat_count(&reader, &start, b"number-rows-repeated")?;
                        let first = tables.last().map_or(1, |t| t.row_cursor);
                        if let Some(table) = tables.last_mut() {
                            table.row_cursor = advance(table.row_cursor, repeat, "row")?;
                        }
                        rows_opened += 1;
                        rows.push(OpenRow {
                            seq: rows_opened,
                            first,
                            repeat,
                            column_cursor: 1,
                            done: Vec::new(),
                            from_depth: depth,
                        });
                    }
                    Structure::Cell | Structure::CoveredCell if skips.is_empty() => {
                        let repeat = repeat_count(&reader, &start, b"number-columns-repeated")?;
                        let column = rows.last().map_or(1, |r| r.column_cursor);
                        if let Some(row) = rows.last_mut() {
                            row.column_cursor = advance(row.column_cursor, repeat, "column")?;
                        }
                        open_cells.push(OpenCell {
                            column,
                            repeat,
                            // A covered cell displays nothing and declares nothing.
                            value_type: if element == Structure::CoveredCell {
                                OdfValueType::Void
                            } else {
                                value_type(&reader, &start)?
                            },
                            formula: element == Structure::Cell
                                && attribute(&reader, &start, NS_TABLE, b"formula")?.is_some(),
                            blocks: Vec::new(),
                            foreign_text: false,
                            from_depth: depth,
                        });
                    }
                    Structure::Odf(Element::Block { heading }) => {
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth += 1;
                        } else {
                            if open.len() >= MAX_BLOCK_NESTING {
                                return Err(EngineError::ResourceLimit {
                                    limit: format!("nested blocks in `{}`", odt::CONTENT_PART),
                                    configured: MAX_BLOCK_NESTING.to_string(),
                                });
                            }
                            opened += 1;
                            open.push(OpenBlock {
                                // The order the block **opened** in, which is document order. A
                                // cell's address does not use it — the address is the cell's
                                // position — but the join below does, and the shared engine
                                // already carries the field.
                                ordinal: opened,
                                heading,
                                // Never read here. A spreadsheet's block is a cell's text, and
                                // `OfficeOdfCellAttributes` carries no block kind — `heading`
                                // itself is discarded when the block closes, so a level beside
                                // it would be a qualifier outliving the thing it qualifies.
                                outline_level: None,
                                text: String::new(),
                                pending_space: false,
                                foreign_depth: 0,
                                foreign_text: false,
                            });
                        }
                    }
                    Structure::Odf(Element::Space) => {
                        let stated = spaces(&start, !skips.is_empty())?;
                        push_stated(&mut open, &stated, &mut skips, &mut text_bytes)?;
                    }
                    Structure::Odf(Element::Tab) => {
                        push_stated(&mut open, "\t", &mut skips, &mut text_bytes)?
                    }
                    Structure::Odf(Element::LineBreak) => {
                        push_stated(&mut open, "\n", &mut skips, &mut text_bytes)?
                    }
                    Structure::Odf(Element::Foreign) if skips.is_empty() => {
                        if let Some(block) = open.last_mut() {
                            block.foreign_depth += 1;
                        }
                    }
                    _ => {}
                }
            }

            Event::Empty(start) => {
                let qualified = start.name();
                let element = structure(namespace, local_name(qualified.as_ref()));
                match element {
                    // **The serialization must not move an address.** `<table:table-cell/>` is what
                    // every producer writes for an empty cell, and it occupies its columns exactly
                    // as `<table:table-cell></table:table-cell>` does — the defect v2-S4 found in a
                    // slide's `<a:p/>`, arriving here as a column number instead of an ordinal.
                    Structure::Cell | Structure::CoveredCell if skips.is_empty() => {
                        let repeat = repeat_count(&reader, &start, b"number-columns-repeated")?;
                        if let Some(row) = rows.last_mut() {
                            row.column_cursor = advance(row.column_cursor, repeat, "column")?;
                        }
                    }
                    // And neither may `<table:table-row/>`, which is a whole empty row.
                    Structure::Row if skips.is_empty() => {
                        let repeat = repeat_count(&reader, &start, b"number-rows-repeated")?;
                        if let Some(table) = tables.last_mut() {
                            table.row_cursor = advance(table.row_cursor, repeat, "row")?;
                        }
                    }
                    // **A self-closing table is still a table**, and must meet the same two
                    // conditions: it names itself, and no sibling shares that name. Checking it
                    // only on the `Start` path would make the document's acceptance turn on how it
                    // was serialized — the defect v2-S4 found in `<a:p/>`, in a third place.
                    Structure::Table if skips.is_empty() => {
                        let name = table_name(&reader, &start)?;
                        if named.iter().any(|seen| seen == &name) {
                            return Err(duplicate_table(&name));
                        }
                        named.push(name);
                    }
                    Structure::Odf(Element::TextBox) => {
                        if let Some(taken) = frame_taken.last_mut() {
                            *taken = true;
                        }
                    }
                    Structure::Odf(Element::Space) => {
                        let stated = spaces(&start, !skips.is_empty())?;
                        push_stated(&mut open, &stated, &mut skips, &mut text_bytes)?;
                    }
                    Structure::Odf(Element::Tab) => {
                        push_stated(&mut open, "\t", &mut skips, &mut text_bytes)?
                    }
                    Structure::Odf(Element::LineBreak) => {
                        push_stated(&mut open, "\n", &mut skips, &mut text_bytes)?
                    }
                    // Read, recognised, refused — the one line `odt.rs` spends on it. A
                    // spreadsheet's print break is even more plainly a printer's than a word
                    // processor's page break, and neither is a page this engine measured.
                    Structure::Odf(Element::SoftPageBreak) => {}
                    _ => {}
                }
            }

            Event::End(end) => {
                depth -= 1;
                let qualified = end.name();
                let element = structure(namespace, local_name(qualified.as_ref()));

                let mut closed_region = false;
                if let Some(skip) = skips.last() {
                    if depth + 1 == skip.from_depth {
                        let skip = skips.pop().expect("checked above");
                        skip_counts_bare.pop();
                        if skip.held_text {
                            regions_not_read = crate::declare(regions_not_read, 1);
                        }
                        closed_region = true;
                    }
                }

                match element {
                    Structure::Table
                        if tables.last().is_some_and(|t| t.from_depth == depth + 1) =>
                    {
                        tables.pop();
                    }
                    Structure::Row if rows.last().is_some_and(|r| r.from_depth == depth + 1) => {
                        let row = rows.pop().expect("checked above");
                        match tables.last() {
                            Some(table) => materialise(&row, &table.name, &mut cells)?,
                            // **A row outside every `<table:table>` has no name to be addressed
                            // by.** Emitting its cells under `""` would give several of them one
                            // uncitable address — and this reader already refuses a *table* that
                            // carries no `table:name` for that exact reason, so accepting the same
                            // gap here would be two answers to one question.
                            None => {
                                text_outside_a_cell = crate::declare(
                                    text_outside_a_cell,
                                    crate::declared_len(row.done.len()),
                                );
                            }
                        }
                    }
                    Structure::Cell | Structure::CoveredCell
                        if open_cells.last().is_some_and(|c| c.from_depth == depth + 1) =>
                    {
                        let done = open_cells.pop().expect("checked above");
                        if done.foreign_text {
                            foreign_text_not_read = crate::declare(foreign_text_not_read, 1);
                        }
                        let mut blocks = done.blocks;
                        blocks.sort_by_key(|(order, _)| *order);
                        let text = blocks
                            .into_iter()
                            .map(|(_, text)| text)
                            .collect::<Vec<_>>()
                            .join("\n");
                        if !text.is_empty() {
                            match rows.last_mut() {
                                // **A covered cell displays nothing, so it is never a node.** The
                                // merge that covers it draws the spanning cell's text over it; text
                                // still written in the covered element is hidden by the document
                                // itself, and emitting it would put a phrase in the record at an
                                // address no reader of the document can see.
                                _ if element == Structure::CoveredCell => {
                                    text_outside_a_cell = crate::declare(text_outside_a_cell, 1);
                                }
                                Some(row) => row.done.push(DoneCell {
                                    column: done.column,
                                    repeat: done.repeat,
                                    value_type: done.value_type,
                                    formula: done.formula,
                                    text,
                                }),
                                // A cell outside every `<table:table-row>` has no row index, so it
                                // has no address. Counted rather than dropped, for the reason the
                                // block path beside it is: a phrase this reader did not place is
                                // still a phrase it did not put in the record.
                                None => {
                                    text_outside_a_cell = crate::declare(text_outside_a_cell, 1);
                                }
                            }
                        }
                    }
                    Structure::Odf(Element::Frame) => {
                        frame_taken.pop();
                    }
                    Structure::Odf(Element::Block { .. }) => {
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth = skip.block_depth.saturating_sub(1);
                        } else if let Some(block) = open.pop() {
                            if block.foreign_text {
                                foreign_text_not_read = crate::declare(foreign_text_not_read, 1);
                            }
                            if !block.text.is_empty() {
                                match open_cells.last_mut() {
                                    Some(cell) => cell.blocks.push((block.ordinal, block.text)),
                                    None => {
                                        text_outside_a_cell =
                                            crate::declare(text_outside_a_cell, 1);
                                    }
                                }
                            }
                        }
                    }
                    Structure::Odf(Element::Foreign) if skips.is_empty() && !closed_region => {
                        if let Some(block) = open.last_mut() {
                            block.foreign_depth = block.foreign_depth.saturating_sub(1);
                        }
                    }
                    _ => {}
                }
            }

            Event::Text(text) => {
                let decoded = decode(&text, odt::CONTENT_PART)?;
                if !outside_every_block(
                    &open,
                    &mut skips,
                    &skip_counts_bare,
                    &mut open_cells,
                    decoded.as_ref(),
                ) {
                    push_source(&mut open, decoded.as_ref(), &mut skips, &mut text_bytes)?;
                }
            }
            // Matched, not ignored: an unhandled `CData` arm is a silent drop.
            Event::CData(cdata) => {
                let decoded = cdata_text(&cdata, odt::CONTENT_PART)?;
                if !outside_every_block(
                    &open,
                    &mut skips,
                    &skip_counts_bare,
                    &mut open_cells,
                    decoded.as_ref(),
                ) {
                    push_source(&mut open, decoded.as_ref(), &mut skips, &mut text_bytes)?;
                }
            }
            Event::GeneralRef(entity) => {
                let resolved = resolve_reference(entity.as_ref(), odt::CONTENT_PART)?;
                let resolved = resolved.as_ref();
                if !outside_every_block(
                    &open,
                    &mut skips,
                    &skip_counts_bare,
                    &mut open_cells,
                    resolved,
                ) {
                    push_source(&mut open, resolved, &mut skips, &mut text_bytes)?;
                }
            }
            _ => {}
        }
    }

    check_closed(depth, odt::CONTENT_PART)?;
    // Stable, so the row-major expansion of a repeat keeps its order inside one row.
    cells.sort_by_key(|(seq, _)| *seq);
    let cells = cells.into_iter().map(|(_, cell)| cell).collect();

    Ok(Sheets {
        cells,
        regions_not_read,
        foreign_text_not_read,
        text_outside_a_cell,
    })
}

/// Account for character data that reached a cell without being inside one of its blocks.
///
/// Returns whether the characters were handled here, in which case the block engine is not asked.
///
/// **The silent drop this closes.** ODF permits foreign elements in mixed content, so
/// `<xhtml:table><xhtml:p>…</xhtml:p></xhtml:table>` sits legally inside a `<table:table-cell>`.
/// Those characters are not the cell's value — a cell's displayed text is its `<text:p>` children —
/// but with no block open the block engine has nowhere to put them and returns without recording
/// anything. They were neither read nor counted, which is the one failure mode this reader's whole
/// allowlist exists to prevent.
fn outside_every_block(
    open: &[OpenBlock],
    skips: &mut [Skip],
    counts_bare: &[bool],
    cells: &mut [OpenCell],
    text: &str,
) -> bool {
    if let Some(skip) = skips.last_mut() {
        // **Only a drawing region takes the bare rule.** A note-shaped one falls through to the
        // shared engine, which counts characters inside the region's own blocks and ignores the
        // `<dc:creator>` and `<text:note-citation>` every real producer writes beside them —
        // `odt.rs` states why, and changing it here would declare an erasure for a comment that
        // has no body.
        if !counts_bare.last().copied().unwrap_or(false) {
            return false;
        }
        // A drawing region's characters are content wherever they sit: an image's `<svg:title>`
        // is in no `<text:p>` at all, so under the block-only rule it was neither read nor counted.
        if text.chars().any(|c| !matches!(c, ' ' | '\t' | '\n' | '\r')) {
            skip.held_text = true;
        }
        return true;
    }
    if !open.is_empty() {
        return false;
    }
    let Some(cell) = cells.last_mut() else {
        return false;
    };
    // The whitespace between two elements is serialization, not erased content — the same four
    // characters ODF's own rule collapses.
    if text.chars().any(|c| !matches!(c, ' ' | '\t' | '\n' | '\r')) {
        cell.foreign_text = true;
    }
    true
}

/// Expand one row's finished cells across the rows and columns the file says they occupy.
///
/// The repeats are the file's own statement that a run of identical cells exists, so materialising
/// them is reading it. Both are capped, and only when they carry text — an empty repeat is a
/// cursor addition, which is what keeps a trailing `table:number-columns-repeated="16384"` free.
fn materialise(
    row: &OpenRow,
    table: &str,
    cells: &mut Vec<(u32, Cell)>,
) -> Result<(), EngineError> {
    if row.done.is_empty() {
        return Ok(());
    }
    if row.repeat > MAX_MATERIALISED_REPEAT {
        return Err(too_many(
            "rows one text-bearing `<table:table-row>` may repeat",
        ));
    }
    for offset in 0..row.repeat {
        for cell in &row.done {
            if cell.repeat > MAX_MATERIALISED_REPEAT {
                return Err(too_many(
                    "columns one text-bearing `<table:table-cell>` may repeat",
                ));
            }
            for step in 0..cell.repeat {
                // **The aggregate binds here, not once per XML event.** One `</table:table-row>`
                // expands rows x columns in a single call, so a cap consulted after the event loop
                // is consulted after the allocation it exists to prevent — 594 source bytes reached
                // 1.83 GB resident before it fired. `odt.rs` states the rule this now follows, that
                // "a per-element cap does not deliver it", and spends its budget in the innermost
                // push for exactly the same reason.
                if cells.len() >= MAX_CELLS {
                    return Err(EngineError::ResourceLimit {
                        limit: format!("cells in `{}`", odt::CONTENT_PART),
                        configured: MAX_CELLS.to_string(),
                    });
                }
                cells.push((
                    row.seq,
                    Cell {
                        table: table.to_string(),
                        row: advance(row.first, offset, "row")?,
                        column: advance(cell.column, step, "column")?,
                        text: cell.text.clone(),
                        value_type: cell.value_type,
                        formula: cell.formula,
                    },
                ));
            }
        }
    }
    Ok(())
}

/// Move a 1-based cursor forward, refusing rather than pinning at the ceiling.
///
/// **A saturated cursor is the failure this reader exists to avoid.** `saturating_add` would give
/// every later cell the same `u32::MAX`, so distinct cells would share one address — a locator that
/// is confidently wrong, which `docs/01-CONTRACT.md` §5.2 calls strictly worse than an absent one.
/// A document whose own repeat counts run past what an index can hold is one this reader cannot
/// address, and saying so is the honest answer.
fn advance(cursor: u32, by: u32, axis: &str) -> Result<u32, EngineError> {
    cursor
        .checked_add(by)
        .ok_or_else(|| EngineError::Malformed {
            what: odt::CONTENT_PART.into(),
            detail: format!(
            "this document's `table:number-{axis}s-repeated` counts run past what a {axis} index \
             can hold. Clamping would give distinct cells one address, so the document is refused \
             rather than addressed wrongly."
        ),
        })
}

fn too_many(limit: &str) -> EngineError {
    EngineError::ResourceLimit {
        limit: limit.into(),
        configured: MAX_MATERIALISED_REPEAT.to_string(),
    }
}

/// `table:name`, resolved and required.
fn table_name(reader: &NsReader<&[u8]>, start: &BytesStart<'_>) -> Result<String, EngineError> {
    match attribute(reader, start, NS_TABLE, b"name")? {
        Some(name) => Ok(name),
        None => Err(EngineError::Malformed {
            what: odt::CONTENT_PART.into(),
            detail: "a `<table:table>` carries no `table:name`. The name is the only component of \
                     a cell's address this format writes down, so a table without one cannot be \
                     cited — and putting a position there would be this reader inventing the half \
                     the file does state."
                .into(),
        }),
    }
}

fn duplicate_table(name: &str) -> EngineError {
    EngineError::Malformed {
        what: odt::CONTENT_PART.into(),
        detail: format!(
            "this document declares more than one `<table:table>` named `{name}`. The name is the \
             address, so two of them give one citation two answers — the defect v2-S4 measured in \
             a slide's shape id, arriving here in the one address component ODF writes down."
        ),
    }
}

/// `office:value-type`, mapped to ODF's own list, or a named refusal.
fn value_type(
    reader: &NsReader<&[u8]>,
    start: &BytesStart<'_>,
) -> Result<OdfValueType, EngineError> {
    let Some(raw) = attribute(reader, start, NS_OFFICE, b"value-type")? else {
        return Ok(OdfValueType::Void);
    };
    match raw.as_str() {
        "float" => Ok(OdfValueType::Float),
        "percentage" => Ok(OdfValueType::Percentage),
        "currency" => Ok(OdfValueType::Currency),
        "date" => Ok(OdfValueType::Date),
        "time" => Ok(OdfValueType::Time),
        "boolean" => Ok(OdfValueType::Boolean),
        "string" => Ok(OdfValueType::String),
        other => Err(EngineError::Unsupported {
            what: "office:value-type".into(),
            detail: format!(
                "`{other}` is not one of OpenDocument's declared value types. Refused by name \
                 rather than recorded as a string, because a type this build does not recognise is \
                 one whose stored value it cannot describe."
            ),
        }),
    }
}

/// One attribute, matched on its **resolved namespace** and local name.
///
/// Not a suffix match. `name`, `formula` and `value-type` all feed either an address or a declared
/// fact, and `docs/history/15-V2-MILESTONES.md` S5 states the rule this follows: a suffix match is
/// acceptable where it can only select content, and not where it selects an address.
///
/// The matcher itself moved to [`crate::xml`] at v2-S7, unchanged, so the presentation reader asks
/// `draw:name` the identical question rather than restating it.
fn attribute(
    reader: &NsReader<&[u8]>,
    start: &BytesStart<'_>,
    namespace: &[u8],
    want: &[u8],
) -> Result<Option<String>, EngineError> {
    resolved_attribute(reader, start, namespace, want, odt::CONTENT_PART)
}

/// `table:number-columns-repeated` / `table:number-rows-repeated`, which default to one.
fn repeat_count(
    reader: &NsReader<&[u8]>,
    start: &BytesStart<'_>,
    want: &[u8],
) -> Result<u32, EngineError> {
    let Some(raw) = attribute(reader, start, NS_TABLE, want)? else {
        return Ok(1);
    };
    match raw.trim().parse::<u32>() {
        Ok(0) | Err(_) => Err(EngineError::Malformed {
            what: odt::CONTENT_PART.into(),
            detail: format!(
                "`table:{}` is `{raw}`, which is not a positive count. That attribute is how this \
                 format states a run of cells exists, so a value this reader cannot read is a run \
                 of unknown length — and guessing one would move every address after it.",
                String::from_utf8_lossy(want)
            ),
        }),
        Ok(n) => Ok(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real OpenDocument namespace URIs, because this reader resolves them.
    ///
    /// A test that bound `table:` to something else would exercise nothing: every element would
    /// fall through to [`crate::odt::classify`] as [`Element::Foreign`] and every assertion below
    /// would be about the fallback path rather than the real one.
    const OPEN: &str = concat!(
        r#"<?xml version="1.0"?><office:document-content"#,
        r#" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0""#,
        r#" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0""#,
        r#" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0""#,
        r#" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0""#,
        r#" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0""#,
        r#" xmlns:xhtml="http://www.w3.org/1999/xhtml""#,
        r#" xmlns:math="http://www.w3.org/1998/Math/MathML""#,
        r#" xmlns:dc="http://purl.org/dc/elements/1.1/""#,
        r#"><office:body><office:spreadsheet>"#
    );
    const CLOSE: &str = "</office:spreadsheet></office:body></office:document-content>";

    fn body(inner: &str) -> String {
        format!("{OPEN}{inner}{CLOSE}")
    }

    /// One table named `S`, wrapping the rows given.
    fn read(rows: &str) -> Sheets {
        read_raw(&format!(
            r#"<table:table table:name="S">{rows}</table:table>"#
        ))
    }

    fn read_raw(inner: &str) -> Sheets {
        read_content(body(inner).as_bytes()).expect("the part reads")
    }

    fn err(inner: &str) -> EngineError {
        read_content(body(inner).as_bytes()).expect_err("the part is refused")
    }

    fn at(sheets: &Sheets, row: u32, column: u32) -> Option<&Cell> {
        sheets
            .cells
            .iter()
            .find(|c| c.row == row && c.column == column)
    }

    // ---------------------------------------------------------------------------------------
    // The address, which is the whole reason this is a slice
    // ---------------------------------------------------------------------------------------

    /// A cell binds at the position the file states, and the table name is the file's own.
    #[test]
    fn a_cell_binds_at_the_position_the_file_states() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell office:value-type="string"><text:p>Alpha</text:p></table:table-cell>
                 <table:table-cell office:value-type="float" office:value="42"><text:p>42</text:p></table:table-cell>
               </table:table-row>
               <table:table-row>
                 <table:table-cell office:value-type="string"><text:p>Beta</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 3);
        assert_eq!(at(&sheets, 1, 1).map(|c| c.text.as_str()), Some("Alpha"));
        assert_eq!(at(&sheets, 1, 2).map(|c| c.text.as_str()), Some("42"));
        assert_eq!(at(&sheets, 2, 1).map(|c| c.text.as_str()), Some("Beta"));
        assert_eq!(at(&sheets, 1, 1).unwrap().table, "S");
        assert_eq!(at(&sheets, 1, 2).unwrap().value_type, OdfValueType::Float);
    }

    /// **The repeat attributes are part of the address, not an optimisation.**
    ///
    /// The load-bearing test of this reader. An empty run of three columns costs one addition and
    /// puts the next cell at column 5 — a reader that ignored the attribute would say column 3, and
    /// every real producer writes such a run on every row.
    #[test]
    fn an_empty_repeated_cell_advances_the_column_it_says_it_does() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell office:value-type="string"><text:p>A</text:p></table:table-cell>
                 <table:table-cell table:number-columns-repeated="3"/>
                 <table:table-cell office:value-type="string"><text:p>E</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 2);
        assert_eq!(at(&sheets, 1, 1).map(|c| c.text.as_str()), Some("A"));
        assert_eq!(
            at(&sheets, 1, 5).map(|c| c.text.as_str()),
            Some("E"),
            "three repeated columns sit at 2, 3 and 4, so the next cell is 5"
        );
    }

    /// The same, spelled the other way. `<table:table-cell/>` and its long form are one document.
    #[test]
    fn the_serialization_of_an_empty_cell_does_not_move_an_address() {
        let short = read(
            r#"<table:table-row><table:table-cell/>
                 <table:table-cell office:value-type="string"><text:p>B</text:p></table:table-cell>
               </table:table-row>"#,
        );
        let long = read(
            r#"<table:table-row><table:table-cell></table:table-cell>
                 <table:table-cell office:value-type="string"><text:p>B</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(short.cells, long.cells);
        assert_eq!(short.cells[0].column, 2);
    }

    /// A covered cell occupies its columns and displays nothing.
    #[test]
    fn a_covered_cell_occupies_its_column_and_yields_no_node() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell table:number-columns-spanned="2" office:value-type="string"><text:p>Wide</text:p></table:table-cell>
                 <table:covered-table-cell/>
                 <table:table-cell office:value-type="string"><text:p>After</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 2);
        assert_eq!(at(&sheets, 1, 1).map(|c| c.text.as_str()), Some("Wide"));
        assert_eq!(at(&sheets, 1, 3).map(|c| c.text.as_str()), Some("After"));
    }

    /// A repeated **row** is the rows the file says it is, and the cursor lands past all of them.
    #[test]
    fn a_repeated_row_is_the_rows_the_file_states() {
        let sheets = read(
            r#"<table:table-row table:number-rows-repeated="3">
                 <table:table-cell office:value-type="string"><text:p>Same</text:p></table:table-cell>
               </table:table-row>
               <table:table-row>
                 <table:table-cell office:value-type="string"><text:p>Next</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 4);
        for row in 1..=3 {
            assert_eq!(at(&sheets, row, 1).map(|c| c.text.as_str()), Some("Same"));
        }
        assert_eq!(at(&sheets, 4, 1).map(|c| c.text.as_str()), Some("Next"));
    }

    /// An empty repeated row costs nothing and still moves the cursor.
    #[test]
    fn an_empty_repeated_row_advances_without_materialising() {
        let sheets = read(
            r#"<table:table-row table:number-rows-repeated="1048576"/>
               <table:table-row>
                 <table:table-cell office:value-type="string"><text:p>Deep</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 1);
        assert_eq!(sheets.cells[0].row, 1_048_577);
    }

    /// A repeat carrying text past the cap is a named refusal, not an allocation.
    #[test]
    fn a_text_bearing_repeat_past_the_cap_is_refused_by_name() {
        let e = err(&format!(
            r#"<table:table table:name="S"><table:table-row>
                 <table:table-cell table:number-columns-repeated="{}" office:value-type="string"><text:p>Bomb</text:p></table:table-cell>
               </table:table-row></table:table>"#,
            MAX_MATERIALISED_REPEAT + 1
        ));
        assert!(
            matches!(&e, EngineError::ResourceLimit { limit, .. } if limit.contains("columns")),
            "expected a named column-repeat limit, got {e}"
        );
    }

    /// A repeat this reader cannot read is a refusal, because guessing would move every address.
    #[test]
    fn a_repeat_that_is_not_a_positive_count_is_refused() {
        for bad in ["0", "-1", "many", ""] {
            let e = err(&format!(
                r#"<table:table table:name="S"><table:table-row>
                     <table:table-cell table:number-columns-repeated="{bad}"/>
                   </table:table-row></table:table>"#
            ));
            assert!(
                matches!(&e, EngineError::Malformed { detail, .. } if detail.contains("positive count")),
                "`{bad}` should be refused, got {e}"
            );
        }
    }

    // ---------------------------------------------------------------------------------------
    // The name, which is the one component ODF writes down
    // ---------------------------------------------------------------------------------------

    /// The table name is the file's own, entities resolved — a dropped `&` is a wrong address.
    #[test]
    fn the_table_name_is_the_files_own_with_entities_resolved() {
        let sheets = read_raw(
            r#"<table:table table:name="Rows &amp; Columns"><table:table-row>
                 <table:table-cell office:value-type="string"><text:p>x</text:p></table:table-cell>
               </table:table-row></table:table>"#,
        );
        assert_eq!(sheets.cells[0].table, "Rows & Columns");
    }

    /// Two tables of one name give one citation two answers, so the document is refused.
    #[test]
    fn two_tables_of_one_name_are_refused() {
        let e = err(
            r#"<table:table table:name="S"><table:table-row><table:table-cell/></table:table-row></table:table>
               <table:table table:name="S"><table:table-row><table:table-cell/></table:table-row></table:table>"#,
        );
        assert!(
            matches!(&e, EngineError::Malformed { detail, .. } if detail.contains("more than one")),
            "got {e}"
        );
    }

    /// A table with no name cannot be cited, so it is refused rather than numbered.
    #[test]
    fn a_table_with_no_name_is_refused_rather_than_numbered() {
        let e = err(
            r#"<table:table><table:table-row><table:table-cell/></table:table-row></table:table>"#,
        );
        assert!(
            matches!(&e, EngineError::Malformed { detail, .. } if detail.contains("table:name")),
            "got {e}"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The text, which is `odt.rs`'s engine reading the same blocks
    // ---------------------------------------------------------------------------------------

    /// Two paragraphs in a cell are two displayed lines, joined by the line feed the file states.
    #[test]
    fn two_paragraphs_in_a_cell_are_two_lines_of_one_node() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>First</text:p><text:p>Second</text:p>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 1, "one cell is one node");
        assert_eq!(sheets.cells[0].text, "First\nSecond");
    }

    /// ODF's stated characters survive into a cell, exactly as they do into a paragraph.
    #[test]
    fn stated_characters_reach_a_cell_unchanged() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell office:value-type="string"><text:p>a<text:s text:c="3"/>b</text:p></table:table-cell>
                 <table:table-cell office:value-type="string"><text:p>Name<text:tab/>Value</text:p></table:table-cell>
                 <table:table-cell office:value-type="string"><text:p>Rows &amp; columns</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(at(&sheets, 1, 1).unwrap().text, "a   b");
        assert_eq!(at(&sheets, 1, 2).unwrap().text, "Name\tValue");
        assert_eq!(at(&sheets, 1, 3).unwrap().text, "Rows & columns");
    }

    /// A cell whose paragraph is only collapsible whitespace displays nothing and is no node.
    #[test]
    fn a_cell_that_displays_nothing_is_no_node() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell office:value-type="string"><text:p>   </text:p></table:table-cell>
                 <table:table-cell/>
                 <table:table-cell office:value-type="string"><text:p>Real</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 1);
        assert_eq!(sheets.cells[0].column, 3);
    }

    /// A formula cell's text is the producer's cached display form, and says so.
    #[test]
    fn a_formula_cells_text_is_labelled_as_cached() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell table:formula="of:=SUM([.A1:.A2])" office:value-type="float" office:value="3"><text:p>3</text:p></table:table-cell>
                 <table:table-cell office:value-type="float" office:value="3"><text:p>3</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert!(
            at(&sheets, 1, 1).unwrap().formula,
            "the cell carries a formula"
        );
        assert!(!at(&sheets, 1, 2).unwrap().formula, "and this one does not");
        assert_eq!(
            at(&sheets, 1, 1).unwrap().text,
            "3",
            "the cached display form, never the formula source"
        );
    }

    /// Every declared value type is read, and one this build does not know is refused by name.
    #[test]
    fn every_declared_value_type_is_read_and_an_unknown_one_is_refused() {
        for (declared, expected) in [
            ("float", OdfValueType::Float),
            ("percentage", OdfValueType::Percentage),
            ("currency", OdfValueType::Currency),
            ("date", OdfValueType::Date),
            ("time", OdfValueType::Time),
            ("boolean", OdfValueType::Boolean),
            ("string", OdfValueType::String),
        ] {
            let sheets = read(&format!(
                r#"<table:table-row><table:table-cell office:value-type="{declared}"><text:p>v</text:p></table:table-cell></table:table-row>"#
            ));
            assert_eq!(sheets.cells[0].value_type, expected, "`{declared}`");
        }
        let sheets = read(
            r#"<table:table-row><table:table-cell><text:p>v</text:p></table:table-cell></table:table-row>"#,
        );
        assert_eq!(
            sheets.cells[0].value_type,
            OdfValueType::Void,
            "a cell declaring no type declares no typed value"
        );
        let e = err(r#"<table:table table:name="S"><table:table-row>
                 <table:table-cell office:value-type="quaternion"><text:p>v</text:p></table:table-cell>
               </table:table-row></table:table>"#);
        assert!(matches!(e, EngineError::Unsupported { .. }), "got {e}");
    }

    // ---------------------------------------------------------------------------------------
    // Namespaces — a suffix match here would steal an address or a skip
    // ---------------------------------------------------------------------------------------

    /// A foreign `table` is not a table, and a foreign `p` is not a block.
    ///
    /// ODF permits foreign elements in mixed content, so both are conforming documents rather than
    /// malformed ones. A suffix match would open a second table — refusing the document for a
    /// duplicate name it does not have — and would put an XHTML paragraph's text in a cell.
    #[test]
    fn a_foreign_table_is_not_a_table_and_a_foreign_p_is_not_a_block() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Kept</text:p>
                 <xhtml:table><xhtml:p>Stolen</xhtml:p></xhtml:table>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 1);
        assert_eq!(
            sheets.cells[0].text, "Kept",
            "the XHTML paragraph's text is not the cell's"
        );
        assert_eq!(
            sheets.foreign_text_not_read, 1,
            "and it is declared rather than dropped"
        );
    }

    /// MathML's `<annotation>` is not `<office:annotation>`.
    ///
    /// A suffix match would declare an inline formula as an unread reviewer's remark — naming a gap
    /// the document does not have, and dropping the cell's own text with it.
    #[test]
    fn mathml_annotation_is_not_a_comment() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Kept<math:annotation>x+1</math:annotation></text:p>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "Kept");
        assert_eq!(
            sheets.regions_not_read, 0,
            "a formula's source form is not a comment"
        );
        assert_eq!(sheets.foreign_text_not_read, 1);
    }

    // ---------------------------------------------------------------------------------------
    // The allowlist — `odt.rs`'s, exercised through a cell
    // ---------------------------------------------------------------------------------------

    /// Non-displayed character data never reaches a cell, and is counted instead.
    ///
    /// Every one of these landed in `Node.text` in the ODT reader's first version. The last two are
    /// the sharpest: this engine refuses `<text:soft-page-break/>` by name and would otherwise have
    /// put the same producer's page arithmetic into a cell anyway.
    #[test]
    fn non_displayed_character_data_never_reaches_a_cell() {
        for (name, xml, leak) in [
            (
                "embedded object data",
                r#"<draw:object-ole><office:binary-data>QUJD</office:binary-data></draw:object-ole>"#,
                "QUJD",
            ),
            (
                "generated number",
                r#"<text:number>2.1</text:number>"#,
                "2.1",
            ),
            (
                "cached page count",
                r#"<text:page-count>17</text:page-count>"#,
                "17",
            ),
            (
                "cached page number",
                r#"<text:page-number>4</text:page-number>"#,
                "4",
            ),
            (
                "ruby text",
                r#"<text:ruby><text:ruby-base>base</text:ruby-base><text:ruby-text>guide</text:ruby-text></text:ruby>"#,
                "guide",
            ),
        ] {
            let sheets = read(&format!(
                r#"<table:table-row><table:table-cell office:value-type="string">
                     <text:p>Value{xml}</text:p>
                   </table:table-cell></table:table-row>"#
            ));
            let text = &sheets.cells[0].text;
            assert!(
                !text.contains(leak),
                "{name}: `{leak}` reached the cell as `{text}`"
            );
            assert!(
                text.starts_with("Value"),
                "{name}: the cell's own text was lost — `{text}`"
            );
            assert_eq!(
                sheets.foreign_text_not_read, 1,
                "{name}: it must be declared, not dropped"
            );
        }
    }

    /// A ruby base **is** the word, and only its guide is passed over.
    #[test]
    fn a_ruby_base_is_the_word_and_only_the_guide_is_passed_over() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p><text:ruby><text:ruby-base>word</text:ruby-base><text:ruby-text>guide</text:ruby-text></text:ruby></text:p>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "word");
    }

    // ---------------------------------------------------------------------------------------
    // The frame-alternative rule, measured — what v2-S5 owed this slice
    // ---------------------------------------------------------------------------------------

    /// **One frame, two renditions — and the second is declared separately from the first.**
    ///
    /// v2-S5 wrote first-rendition-wins off the specification and said plainly that it had never
    /// been measured against a document that nests two renditions, because it had none. This is
    /// that document, authored here: ODF puts `<draw:frame>` in the `<text:p>` content model for
    /// every document type, so a spreadsheet cell can hold one.
    ///
    /// **What the rule looks like in a spreadsheet is not what it looks like in a text document,
    /// and that is the finding.** In an ODT the first rendition's paragraphs become nodes, because
    /// the atom is the paragraph. Here the atom is the **cell**, and a frame floats over the sheet
    /// — its words belong to no cell, so there is no address at which "one displayed phrase becomes
    /// one node" could be true. Merging them into the anchoring cell was this reader's first
    /// behaviour and it was a mis-attribution.
    ///
    /// So what is measurable in ODS is the half that survives the atom change: **the first
    /// rendition and the second are accounted separately**, so two renditions declare two erasures
    /// where one declares one. That is the rule being exercised, and the cell's own text is
    /// untouched by either.
    #[test]
    fn a_second_rendition_is_declared_separately_from_the_first() {
        let one = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>CELL<draw:frame>
                   <draw:text-box><text:p>FIRST</text:p></draw:text-box>
                 </draw:frame></text:p>
               </table:table-cell></table:table-row>"#,
        );
        let two = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>CELL<draw:frame>
                   <draw:text-box><text:p>FIRST</text:p></draw:text-box>
                   <draw:text-box><text:p>SECOND</text:p></draw:text-box>
                 </draw:frame></text:p>
               </table:table-cell></table:table-row>"#,
        );

        for (label, sheets) in [("one rendition", &one), ("two renditions", &two)] {
            assert_eq!(sheets.cells.len(), 1, "{label}: one cell is one node");
            assert_eq!(
                sheets.cells[0].text, "CELL",
                "{label}: a framed shape's words are not the cell's value"
            );
        }
        assert_eq!(one.regions_not_read, 1, "the frame is declared");
        assert_eq!(
            two.regions_not_read, 2,
            "and a SECOND rendition is declared separately — the rule, measured"
        );
    }

    /// The same claim on the self-closing path, so the rule does not turn on serialization.
    #[test]
    fn the_frame_rule_holds_for_a_self_closing_first_rendition() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Cell<draw:frame>
                   <draw:text-box/>
                   <draw:text-box><text:p>SECOND</text:p></draw:text-box>
                 </draw:frame></text:p>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "Cell");
        assert_eq!(
            sheets.regions_not_read, 1,
            "an empty first rendition still claims the frame's one slot — so the frame itself \
             declares nothing, and the one erasure is the second rendition"
        );
    }

    /// **An image's title and description never reach a cell**, and are declared with the frame.
    #[test]
    fn an_images_title_and_description_are_declared_with_their_frame() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Value<draw:frame><draw:image>
                   <svg:title>IMAGE-TITLE</svg:title><svg:desc>IMAGE-DESC</svg:desc>
                 </draw:image></draw:frame></text:p>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "Value");
        assert_eq!(sheets.regions_not_read, 1);
    }

    // ---------------------------------------------------------------------------------------
    // What is passed over, and counted
    // ---------------------------------------------------------------------------------------

    /// A table's page-anchored drawings hold text with no cell to belong to.
    #[test]
    fn page_anchored_shapes_are_declared_rather_than_dropped() {
        let sheets = read(
            r#"<table:shapes><draw:frame><draw:text-box><text:p>Floating</text:p></draw:text-box></draw:frame></table:shapes>
               <table:table-row>
                 <table:table-cell office:value-type="string"><text:p>Cell</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(sheets.cells.len(), 1);
        assert_eq!(sheets.cells[0].text, "Cell");
        assert_eq!(sheets.regions_not_read, 1);
        assert_eq!(
            sheets.text_outside_a_cell, 0,
            "a declared region is not also an orphan"
        );
    }

    /// A comment's body is not the sheet's text.
    #[test]
    fn a_comment_body_is_not_the_cells_text() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Kept</text:p>
                 <office:annotation><dc:creator>Reviewer</dc:creator><text:p>Check this</text:p></office:annotation>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "Kept");
        assert_eq!(sheets.regions_not_read, 1);
    }

    /// A block holding text with no cell open has no address, and is counted rather than dropped.
    #[test]
    fn text_with_no_cell_to_belong_to_is_counted() {
        let sheets = read_raw(r#"<text:p>Orphan</text:p>"#);
        assert!(sheets.cells.is_empty());
        assert_eq!(sheets.text_outside_a_cell, 1);
    }

    /// A print break inside a cell is read, recognised, and contributes nothing.
    #[test]
    fn a_print_break_inside_a_cell_contributes_no_character() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Split<text:soft-page-break/> and rejoined</text:p>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "Split and rejoined");
        assert_eq!(sheets.foreign_text_not_read, 0);
    }

    /// **ODF groups rows without moving them**, so a grouped row is where a counter finds it.
    ///
    /// `<table:table-header-rows>` and `<table:table-row-group>` wrap rows for repeat-on-print and
    /// outlining. Both are ordinary in a real spreadsheet, and neither changes a row's position — a
    /// reader that reset or skipped inside them would put every later row at the wrong index.
    #[test]
    fn rows_inside_odf_grouping_wrappers_keep_their_positions() {
        let sheets = read(
            r#"<table:table-header-rows>
                 <table:table-row><table:table-cell office:value-type="string"><text:p>HEADER</text:p></table:table-cell></table:table-row>
               </table:table-header-rows>
               <table:table-row-group>
                 <table:table-row><table:table-cell office:value-type="string"><text:p>GROUPED</text:p></table:table-cell></table:table-row>
               </table:table-row-group>
               <table:table-row>
                 <table:table-cell office:value-type="string"><text:p>PLAIN</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(at(&sheets, 1, 1).map(|c| c.text.as_str()), Some("HEADER"));
        assert_eq!(at(&sheets, 2, 1).map(|c| c.text.as_str()), Some("GROUPED"));
        assert_eq!(at(&sheets, 3, 1).map(|c| c.text.as_str()), Some("PLAIN"));
    }

    /// **A table nested inside a frame is not minted as a sheet.**
    ///
    /// Legal ODF: a `<draw:text-box>` may hold a `<table:table>`. It is an alternative rendition of
    /// the framed object, and minting it gave a citation a first-class sheet address that no
    /// spreadsheet consumer can resolve — `Inner` is not a sheet of this document. It is declared
    /// with its frame instead, and the table containing it is untouched: the outer cell keeps its
    /// own text, and the outer row cursor does not advance because an inner row did.
    #[test]
    fn a_table_nested_inside_a_frame_is_declared_rather_than_minted() {
        let sheets = read_raw(
            r#"<table:table table:name="Outer">
                 <table:table-row><table:table-cell office:value-type="string">
                   <text:p>OUTER-A<draw:frame><draw:text-box>
                     <table:table table:name="Inner">
                       <table:table-row><table:table-cell office:value-type="string"><text:p>INNER</text:p></table:table-cell></table:table-row>
                     </table:table>
                   </draw:text-box></draw:frame></text:p>
                 </table:table-cell></table:table-row>
                 <table:table-row><table:table-cell office:value-type="string"><text:p>OUTER-B</text:p></table:table-cell></table:table-row>
               </table:table>"#,
        );
        let seen: Vec<(&str, u32, u32, &str)> = sheets
            .cells
            .iter()
            .map(|c| (c.table.as_str(), c.row, c.column, c.text.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![("Outer", 1, 1, "OUTER-A"), ("Outer", 2, 1, "OUTER-B")],
            "no `Inner` address is minted, and the outer table is undisturbed"
        );
        assert_eq!(sheets.regions_not_read, 1, "and the frame is declared");
    }

    // ---------------------------------------------------------------------------------------
    // Bombs, overflow, and the addresses that must not exist
    // ---------------------------------------------------------------------------------------

    /// **The million-cell cap binds during the expansion, not after the event.**
    ///
    /// One `</table:table-row>` expands rows x columns in a single call, so a cap consulted once
    /// per XML event is consulted after the allocation it exists to prevent: this document is 4096
    /// rows of 4096 columns from a few hundred source bytes, and it reached 1.83 GB resident before
    /// the check fired. `odt.rs` states the rule — "a per-element cap does not deliver it".
    ///
    /// The per-axis caps are deliberately **not** what refuses this: both are exactly at their
    /// limit, so only the aggregate can stop it.
    #[test]
    fn a_repeat_product_within_both_axis_caps_is_still_refused_by_the_aggregate() {
        let cap = MAX_MATERIALISED_REPEAT;
        let e = err(&format!(
            r#"<table:table table:name="S">
                 <table:table-row table:number-rows-repeated="{cap}">
                   <table:table-cell table:number-columns-repeated="{cap}" office:value-type="string"><text:p>x</text:p></table:table-cell>
                 </table:table-row>
               </table:table>"#
        ));
        assert!(
            matches!(&e, EngineError::ResourceLimit { limit, configured }
                if limit.contains("cells") && configured == &MAX_CELLS.to_string()),
            "expected the aggregate cell cap, got {e}"
        );
    }

    /// A cursor that would run past what an index holds is refused, never clamped.
    ///
    /// Clamping would give distinct cells one address — a locator that is confidently wrong, which
    /// the contract calls strictly worse than an absent one.
    #[test]
    fn a_cursor_past_the_ceiling_is_refused_rather_than_pinned() {
        for (attribute, axis) in [
            ("number-columns-repeated", "column"),
            ("number-rows-repeated", "row"),
        ] {
            let element = if axis == "column" {
                format!(r#"<table:table-cell table:{attribute}="4294967295"/>"#)
            } else {
                format!(r#"<table:table-row table:{attribute}="4294967295"/>"#)
            };
            let e = err(&format!(
                r#"<table:table table:name="S"><table:table-row>{element}{element}</table:table-row></table:table>"#
            ));
            assert!(
                matches!(&e, EngineError::Malformed { detail, .. } if detail.contains("past what")),
                "{axis}: expected a named refusal, got {e}"
            );
        }
    }

    /// **A cell outside every row has no address, so its text is declared rather than dropped.**
    ///
    /// The module header commits to this: "not reachable in a conforming spreadsheet — counted
    /// anyway, because 'not reachable' is a claim about producers". It was a silent drop.
    #[test]
    fn a_cell_outside_every_row_is_counted_rather_than_dropped() {
        let sheets = read(
            r#"<table:table-cell office:value-type="string"><text:p>Ghost</text:p></table:table-cell>"#,
        );
        assert!(
            sheets.cells.is_empty(),
            "there is no row index to cite it at"
        );
        assert_eq!(sheets.text_outside_a_cell, 1, "and it is declared");
    }

    /// **A row outside every table has no name, so its cells have no address either.**
    ///
    /// This reader refuses a `<table:table>` that carries no `table:name` for exactly that reason,
    /// so emitting these under `""` would be two answers to one question — and several of them
    /// would collide on one uncitable address.
    #[test]
    fn a_row_outside_every_table_is_counted_rather_than_addressed_as_nothing() {
        let sheets = read_raw(
            r#"<table:table-row>
                 <table:table-cell office:value-type="string"><text:p>A</text:p></table:table-cell>
                 <table:table-cell office:value-type="string"><text:p>B</text:p></table:table-cell>
               </table:table-row>
               <table:table-row>
                 <table:table-cell office:value-type="string"><text:p>C</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert!(sheets.cells.is_empty(), "no table name, so no address");
        assert_eq!(sheets.text_outside_a_cell, 3, "and all three are declared");
    }

    /// **A covered cell displays nothing, so it is never a node** — even carrying content.
    ///
    /// The merge that covers it draws the spanning cell's text over it. Emitting text still written
    /// in the covered element would put a phrase in the record at an address no reader of the
    /// document can see.
    #[test]
    fn a_covered_cell_carrying_content_is_still_no_node() {
        let sheets = read(
            r#"<table:table-row>
                 <table:table-cell table:number-columns-spanned="2" office:value-type="string"><text:p>Wide</text:p></table:table-cell>
                 <table:covered-table-cell office:value-type="string"><text:p>HIDDEN</text:p></table:covered-table-cell>
                 <table:table-cell office:value-type="string"><text:p>After</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert!(
            !sheets.cells.iter().any(|c| c.text.contains("HIDDEN")),
            "the document hides it, so the artifact does not show it: {:?}",
            sheets.cells
        );
        assert_eq!(sheets.text_outside_a_cell, 1, "and it is declared");
        assert_eq!(at(&sheets, 1, 3).map(|c| c.text.as_str()), Some("After"));
    }

    /// A self-closing table meets the same two conditions a long-form one does.
    ///
    /// Checking them only on the `Start` path made the document's acceptance turn on how it was
    /// serialized — the defect v2-S4 found in `<a:p/>`, in a third place.
    #[test]
    fn a_self_closing_table_is_held_to_the_same_two_conditions() {
        let unnamed = err(r#"<table:table/>"#);
        assert!(
            matches!(&unnamed, EngineError::Malformed { detail, .. } if detail.contains("table:name")),
            "got {unnamed}"
        );
        let duplicate = err(
            r#"<table:table table:name="S"><table:table-row><table:table-cell/></table:table-row></table:table>
               <table:table table:name="S"/>"#,
        );
        assert!(
            matches!(&duplicate, EngineError::Malformed { detail, .. } if detail.contains("more than one")),
            "got {duplicate}"
        );
    }

    /// Skip regions nest, and each is counted on its own terms.
    ///
    /// A single slot would register only the outermost, so N nested regions would declare one —
    /// a count that under-declares, which is the direction A14 exists to prevent.
    #[test]
    fn nested_skip_regions_are_counted_separately() {
        let sheets = read(
            r#"<table:table-row><table:table-cell office:value-type="string">
                 <text:p>Kept</text:p>
                 <office:annotation><text:p>OUTER<text:note><text:p>INNER</text:p></text:note></text:p></office:annotation>
               </table:table-cell></table:table-row>"#,
        );
        assert_eq!(sheets.cells[0].text, "Kept");
        assert_eq!(
            sheets.regions_not_read, 2,
            "a note inside a comment is two erasures, not one"
        );
    }

    /// **Attributes are matched on their resolved namespace**, not on a suffix.
    ///
    /// `name`, `formula` and `value-type` each feed an address or a declared fact, so a foreign
    /// attribute that happens to share a local name must not be read as ODF's.
    #[test]
    fn a_foreign_attribute_sharing_a_local_name_is_not_the_odf_one() {
        let sheets = read_raw(
            r#"<table:table table:name="Real" xhtml:name="Fake">
                 <table:table-row>
                   <table:table-cell office:value-type="string" xhtml:formula="=A1" xhtml:number-columns-repeated="7"><text:p>x</text:p></table:table-cell>
                   <table:table-cell office:value-type="string"><text:p>y</text:p></table:table-cell>
                 </table:table-row>
               </table:table>"#,
        );
        assert_eq!(
            sheets.cells[0].table, "Real",
            "the ODF name, not the XHTML one"
        );
        assert!(
            !sheets.cells[0].formula,
            "a foreign `formula` attribute does not label the text cached"
        );
        assert_eq!(
            sheets.cells[1].column, 2,
            "and a foreign repeat count does not move an address"
        );
    }

    /// **`is_ods` is exact, and `is_opendocument` is not the container rule.**
    ///
    /// Both need a real package to say anything. `…spreadsheet-template` shares this type's prefix
    /// and is a different document kind; an `.epub` uses the same first-and-stored `mimetype`
    /// container rule and is not OpenDocument at all — answering `true` for it would route an EPUB
    /// here to be told "this is an OpenDocument package", which is false.
    #[test]
    fn detection_is_exact_and_the_family_question_is_not_the_container_rule() {
        for (declared, ods, odf) in [
            ("application/vnd.oasis.opendocument.spreadsheet", true, true),
            (
                "application/vnd.oasis.opendocument.spreadsheet-template",
                false,
                true,
            ),
            (
                "application/vnd.oasis.opendocument.presentation",
                false,
                true,
            ),
            ("application/epub+zip", false, false),
        ] {
            let package = package_declaring(declared);
            assert_eq!(is_ods(&package), ods, "is_ods(`{declared}`)");
            assert_eq!(
                crate::is_opendocument(&package),
                odf,
                "is_opendocument(`{declared}`)"
            );
        }
        assert!(!is_ods(b"not a zip"));
        assert!(!crate::is_opendocument(b"not a zip"));
    }

    /// A minimal ODF-shaped package: `mimetype` first and **stored**, as the container requires.
    fn package_declaring(media_type: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, body) in [("mimetype", media_type), ("content.xml", "<x/>")] {
            let offset = out.len() as u32;
            let data = body.as_bytes();
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);

            directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            directory.extend_from_slice(&[10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name.as_bytes());
        }
        let directory_offset = out.len() as u32;
        let directory_size = directory.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&directory_size.to_le_bytes());
        out.extend_from_slice(&directory_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// Two reads of one part produce the same cells.
    #[test]
    fn two_reads_of_one_part_agree() {
        let part = body(
            r#"<table:table table:name="S"><table:table-row>
                 <table:table-cell office:value-type="string"><text:p>x</text:p></table:table-cell>
               </table:table-row></table:table>"#,
        );
        assert_eq!(
            read_content(part.as_bytes()).unwrap(),
            read_content(part.as_bytes()).unwrap()
        );
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// This reader is the one that folded a *length* rather than a one, so it pins both halves of
    /// the repair: the saturating fold, and the `usize` count that reaches it without an `as u32`
    /// cast. A cast is the worse of the two, because it wraps in debug as well as release.
    #[test]
    fn an_ods_erasure_count_saturates_rather_than_wrapping() {
        let sheets = read_raw(
            r#"<table:table-row>
                 <table:table-cell office:value-type="string"><text:p>A</text:p></table:table-cell>
                 <table:table-cell office:value-type="string"><text:p>B</text:p></table:table-cell>
               </table:table-row>"#,
        );
        assert_eq!(
            sheets.text_outside_a_cell, 2,
            "no table name, so no address"
        );

        let mut folded = u32::MAX - 1;
        for _ in 0..3 {
            folded = crate::declare(folded, sheets.text_outside_a_cell);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );

        // The row-length path, where `as u32` would have reported zero for a row of exactly
        // `u32::MAX + 1` cells rather than the ceiling.
        let past = usize::try_from(u32::MAX).expect("64-bit") + 1;
        assert_eq!(crate::declared_len(past), u32::MAX);
    }
}
