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

//! `content.xml` → the blocks a draw page displays, and the page that stays out (v2-S7).
//!
//! # The page that was free
//!
//! Every page-less format before this one had to *argue* that its page was somebody else's. A
//! DOCX has no page until a renderer picks one. A workbook's page is a printer's. A PPTX slide is
//! a **part**, and `p:sldSz` is a size rather than a page. An ODT contains a
//! `<text:soft-page-break/>` a word processor wrote at save time.
//!
//! **A presentation needs no argument at all, and that is why it is the sharpest case.**
//! `<draw:page>` elements are discrete, listed, ordered, and named; a `<style:master-page>` states
//! `fo:page-width` and `fo:page-height` beside them. A [`ethos_parser_core::PageRecord`] could be minted
//! here with **no arithmetic**, off two facts the file writes down — the first time that has been
//! true in this engine's history.
//!
//! It is refused, and the reason is `docs/06-STEAL-REFUSE.md` L30's own three words rather than a
//! paraphrase: *"It invents pagination."* **This said four from v2-S7 until v2-S14.1**, and it was
//! wrong when written rather than rotted — `git log -S` shows the quoted phrase never changed, and
//! the miscount was copied to five other files before anybody counted. L30 refuses the LibreOffice
//! bridge because a page it
//! produced is a rendering rather than a fact about the document, and a `<draw:page>` fails the
//! same test from the other direction — it is a **part of the presentation's structure**, and the
//! number a consumer would read off it is a page index this engine never verified. `pages` stays
//! `[]`, [`ethos_parser_core::OdpLocator`] carries a position named for the element, and
//! `deny_unknown_fields` is what stops a `page` arriving later as a fifth field.
//!
//! # One allowlist, not two — for the third reader
//!
//! [`crate::odt`]'s `Element`, `classify`, namespace resolution and block-text engine are
//! **imported**, exactly as [`crate::ods`] imports them and for the reason recorded there: the
//! list's whole content is *which inline names are the sentence*, and two copies of that drift
//! silently. This reader adds only what sits above the block — `draw:page`, the two shapes it
//! reads, and `presentation:notes` — and everything else falls through to one answer.
//!
//! # The frame rule, measured a third time, and it is ODT's answer for a new reason
//!
//! v2-S5 read first-rendition-wins off the specification. v2-S6 measured it on a spreadsheet and
//! found that **the atom decides the outcome**: an ODT's atom is the paragraph, so a frame's first
//! rendition becomes nodes; an ODS's atom is the **cell**, a frame floats over the sheet, its
//! words belong to no cell, and merging them into the anchoring cell was a mis-attribution.
//!
//! A presentation is a drawing, so the question is neither of those. **There is nothing for a
//! frame to float over**: the frame *is* the shape, and the shape is what this reader addresses.
//! So the first rendition becomes nodes — ODT's outcome — and the second is a declared erasure,
//! and the reason is not ODT's. See `docs/history/15-V2-MILESTONES.md` S7 for the three-format table.
//!
//! # What is read, and what is counted (**A14**)
//!
//! Read: the blocks of a `<draw:frame>` and a `<draw:custom-shape>` on a draw page, through the
//! shared allowlist. Counted, never dropped:
//!
//! - `<presentation:notes>` — the speaker's stream. Reading it as slide text would be **A14
//!   inverted**: a silent *extra* rather than a silent drop, putting a phrase in the record that
//!   nobody watching the presentation sees.
//! - A second `<draw:text-box>` in one `<draw:frame>`, and [`crate::odt`]'s three regions — a
//!   note body, a comment, a tracked-changes record.
//! - Text inside a drawing element this slice does not name as a shape: a `<draw:rect>`, a
//!   `<draw:connector>`. It has no address here, so it is declared.
//! - The foreign subtrees the shared allowlist already refuses — an image's `<svg:title>`, an
//!   object's base64, a cached `<text:page-count>` — **including when they sit in a shape with no
//!   block open**, which is the ordinary shape of a presentation and the silent drop v2-S6 found
//!   in a neighbouring format.
//!
//! # What this reader was and was not measured against
//!
//! **No corpus of real `.odp` files was available, and no ODF producer was either** — the third
//! consecutive slice that has to say so, repeated rather than quietly inherited. Every rule here
//! is read off the OpenDocument specification and pinned against packages this repository authors
//! byte by byte. The consequence is written into the design rather than left implicit: `draw:name`
//! could not be measured unique, so it is a label on the attributes and never the address.

use ethos_parser_core::{EngineError, OdfBlockKind};
use quick_xml::events::{BytesStart, Event};
use quick_xml::NsReader;

use crate::odt::{
    self, classify, namespace_of, push_source, push_stated, spaces, Element, OpenBlock, Skip,
    MAX_BLOCK_NESTING, NS_DRAW,
};
use crate::xml::{
    cdata_text, check_closed, decode, local_name, new_ns_reader, parse_error_at, resolve_reference,
    resolved_attribute,
};

/// The OpenDocument **presentation** namespace, where a deck's own vocabulary lives.
///
/// Resolved rather than suffix-matched, for [`crate::odt`]'s reason: `notes` is a local name other
/// vocabularies use, and mistaking one for a speaker's notes would silently erase a region of the
/// document.
const NS_PRESENTATION: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:presentation:1.0";

/// The media type an OpenDocument **presentation** declares.
pub const ODP_MEDIA_TYPE: &str = "application/vnd.oasis.opendocument.presentation";

/// What one element of a presentation's `content.xml` means to this reader.
///
/// Everything that is not presentation structure falls through to [`crate::odt::classify`] — the
/// same allowlist, the same namespaces, the same names — so there is exactly one answer in this
/// crate to *"are these characters the sentence?"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Structure {
    /// `draw:page` — one page of the presentation's own structure.
    ///
    /// **Not a [`ethos_parser_core::PageRecord`].** See the module header: this is the element that
    /// makes the refusal cost something, and it is refused anyway.
    Page,
    /// `draw:frame` — a shape, and the one whose children are alternative renditions.
    Frame,
    /// `draw:custom-shape` — a shape that holds its blocks directly, with no rendition wrapper.
    CustomShape,
    /// `presentation:notes` — the speaker's stream, which is not what the audience reads.
    Notes,
    /// Not presentation structure: whatever `odt.rs` says it is.
    Odf(Element),
}

fn structure(namespace: Option<&[u8]>, local: &[u8]) -> Structure {
    match (namespace, local) {
        (Some(NS_DRAW), b"page") => Structure::Page,
        (Some(NS_DRAW), b"frame") => Structure::Frame,
        (Some(NS_DRAW), b"custom-shape") => Structure::CustomShape,
        (Some(NS_PRESENTATION), b"notes") => Structure::Notes,
        _ => Structure::Odf(classify(namespace, local)),
    }
}

/// One block that carries text, with the address the part states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBlock {
    /// 1-based position of the `<draw:page>` this block sits on, in the part's document order.
    pub draw_page: u32,
    /// The page's own `draw:name`, entities resolved, or `None` when it states none.
    pub draw_page_name: Option<String>,
    /// 1-based position of the shape within that draw page.
    pub shape: u32,
    /// The shape's own `draw:name`, entities resolved, or `None` when it states none.
    pub shape_name: Option<String>,
    /// 1-based position of the block within that shape.
    pub paragraph: u32,
    /// Whether the block was a `<text:h>` rather than a `<text:p>`.
    pub heading: bool,
    /// The block's displayed text, under ODF's whitespace rule.
    pub text: String,
}

/// What one presentation `content.xml` yielded, plus what it passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Presentation {
    /// The blocks that carry text, in the part's own document order.
    pub blocks: Vec<TextBlock>,
    /// Regions of the part that held text and were not read: a speaker-notes body, a second
    /// framed rendition, a drawing shape this slice does not name, and [`crate::odt`]'s three.
    pub regions_not_read: u32,
    /// Blocks and shapes that contained a foreign subtree holding characters: an image's title, an
    /// embedded object's base64, a field's cached page number.
    pub foreign_text_not_read: u32,
    /// Text that reached no shape, so there is no address this reader could cite it at.
    pub text_outside_a_shape: u32,
}

/// Whether these bytes are an OpenDocument **presentation** package, read from the bytes.
///
/// The three questions [`crate::odt::is_odt`] asks, against a different declared type — and exact
/// rather than prefixed, so `…opendocument.presentation-template` (an `.otp`) is not claimed, and
/// neither is `…opendocument.graphics`, whose body is the same `<draw:page>` vocabulary under a
/// different declaration.
pub fn is_odp(bytes: &[u8]) -> bool {
    odt::declared_media_type(bytes).is_some_and(|declared| declared == ODP_MEDIA_TYPE)
}

/// A draw page, as an address component a block can carry away.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PageRef {
    index: u32,
    name: Option<String>,
}

/// A `<draw:page>` being read: which one it is, and how many shapes it has opened.
struct OpenPage {
    page: PageRef,
    /// The index the next shape occupies. 1-based.
    shape_cursor: u32,
    from_depth: i32,
}

/// A shape being read.
struct OpenShape {
    /// The draw page it sits on, or `None` when it sits outside every one.
    ///
    /// A shape with no page has no address to give its blocks — the same gap `ods.rs` closes for a
    /// cell outside every row. Its text is declared rather than filed under a page index this
    /// reader would have had to invent.
    page: Option<PageRef>,
    index: u32,
    name: Option<String>,
    /// The index the next block occupies. 1-based.
    paragraph_cursor: u32,
    /// Whether a `<draw:text-box>` has already been read from this shape.
    ///
    /// A frame's children are alternative renditions of one object, so the first this reader can
    /// process is the one it uses and the rest are duplicates of a phrase already in the record.
    /// Carried per shape rather than in a separate stack, so the push and the pop are the same two
    /// lines that manage the shape itself — v2-S5's rule, which asks that the two be guarded
    /// identically or a nested frame pops an enclosing frame's flag.
    text_box_taken: bool,
    /// Whether a foreign subtree of this shape held characters while no block was open.
    ///
    /// **The ordinary shape of a presentation, not an edge case.** `<draw:frame><draw:image>
    /// <svg:title>` has no `<text:p>` anywhere in it, so the shared engine's per-block foreign
    /// counter never sees it: the characters were neither read nor counted, which is the failure
    /// the whole allowlist exists to prevent and the one v2-S6 found in a neighbouring format.
    foreign_text: bool,
    from_depth: i32,
}

/// Parallel to the open-block stack: the address a block will carry, fixed when it **opened**.
///
/// Not computed when the block closes. ODF nests blocks — a shape inside a text box inside a
/// shape — and an inner block closes before the outer one that contains it, so the innermost open
/// shape at close time is not the one the block belongs to.
#[derive(Debug, Clone)]
struct BlockAddress {
    page: PageRef,
    shape: u32,
    shape_name: Option<String>,
    paragraph: u32,
}

/// Read a presentation `content.xml` into the blocks that carry text.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, if the part ends with elements still
/// open, or if a shape or block index runs past what a `u32` holds. [`EngineError::ResourceLimit`]
/// if blocks nest past [`crate::odt::MAX_BLOCK_NESTING`] or the part's text expands past
/// [`crate::odt::MAX_TEXT_BYTES`].
pub fn read_content(part: &[u8]) -> Result<Presentation, EngineError> {
    let mut reader = new_ns_reader(part, odt::CONTENT_PART)?;
    let mut blocks: Vec<(u32, TextBlock)> = Vec::new();
    let mut regions_not_read = 0u32;
    let mut foreign_text_not_read = 0u32;
    let mut text_outside_a_shape = 0u32;

    // Stacks rather than slots, for `ods.rs`'s reason: ODF lets a shape hold a text box that holds
    // a shape, so these genuinely nest — and a slot would let an inner `</draw:frame>` close the
    // outer one, which is the class of defect v2-S5's own review found twice.
    let mut pages: Vec<OpenPage> = Vec::new();
    let mut shapes: Vec<OpenShape> = Vec::new();

    let mut open: Vec<OpenBlock> = Vec::new();
    // Pushed and popped in lockstep with `open`. See [`BlockAddress`].
    let mut addresses: Vec<Option<BlockAddress>> = Vec::new();
    let mut skips: Vec<Skip> = Vec::new();
    // Parallel to `skips`: whether this region counts **bare** character data, or only characters
    // inside one of its own blocks.
    //
    // `odt.rs`'s three regions want the stricter rule and say why: every note carries a
    // `<text:note-citation>` and every comment a `<dc:creator>`, so counting bare characters would
    // declare an erasure for a comment with no body. Everything this reader adds is a **drawing** —
    // a notes page, an alternative rendition, an unread shape — and a drawing's `<svg:title>` sits
    // in no block at all, so those count bare characters. v2-S6 measured that distinction; it is
    // carried here rather than re-derived.
    let mut skip_counts_bare: Vec<bool> = Vec::new();

    let mut depth: i32 = 0;
    let mut text_bytes: usize = 0;
    // Document order across the whole part, used to sort blocks that close out of order.
    let mut opened: u32 = 0;
    // 1-based, advanced for **every** `<draw:page>` the part contains.
    let mut pages_opened: u32 = 0;

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
                    // **The speaker's stream is a second stream.** Splicing it into the slide's
                    // text would be A14 inverted — a silent extra rather than a silent drop — so
                    // it is counted and named instead. `docs/history/15-V2-MILESTONES.md` S7 records that
                    // whether notes are evidence is a question this slice does not answer.
                    Structure::Notes | Structure::Odf(Element::Region) => true,
                    // The shape's one rendition slot. A second `<draw:text-box>` is a **second**
                    // declared erasure rather than part of the first, which is what makes the rule
                    // measurable at all.
                    Structure::Odf(Element::TextBox) => match shapes.last_mut() {
                        Some(shape) if !shape.text_box_taken => {
                            shape.text_box_taken = true;
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

                match element {
                    Structure::Page => {
                        pages_opened = advance(pages_opened, "draw page")?;
                        pages.push(OpenPage {
                            page: PageRef {
                                index: pages_opened,
                                name: draw_name(&reader, &start)?,
                            },
                            shape_cursor: 1,
                            from_depth: depth,
                        });
                    }
                    // **Pushed whether or not this subtree is read.** A frame inside a
                    // `<presentation:notes>` is a shape the draw page contains, so a consumer
                    // counting shapes finds it — and pushing unconditionally is also what keeps
                    // the rendition slot's push and pop symmetric, which v2-S5 asks for by name.
                    Structure::Frame | Structure::CustomShape => {
                        let index = pages.last().map_or(0, |p| p.shape_cursor);
                        let page = pages.last().map(|p| p.page.clone());
                        if let Some(open_page) = pages.last_mut() {
                            open_page.shape_cursor = advance(open_page.shape_cursor, "shape")?;
                        }
                        shapes.push(OpenShape {
                            page,
                            index,
                            name: draw_name(&reader, &start)?,
                            paragraph_cursor: 1,
                            text_box_taken: false,
                            foreign_text: false,
                            from_depth: depth,
                        });
                    }
                    Structure::Odf(Element::Block { heading }) => {
                        opened = advance(opened, "block")?;
                        // **The shape's counter advances through what this reader does not read**,
                        // for `OdtLocator::paragraph`'s reason: the number promises a position in
                        // the file, so an address after a passed-over rendition must not be one
                        // short.
                        let paragraph = match shapes.last_mut() {
                            Some(shape) => {
                                let at = shape.paragraph_cursor;
                                shape.paragraph_cursor = advance(shape.paragraph_cursor, "block")?;
                                Some(at)
                            }
                            None => None,
                        };
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth += 1;
                        } else {
                            if open.len() >= MAX_BLOCK_NESTING {
                                return Err(EngineError::ResourceLimit {
                                    limit: format!("nested blocks in `{}`", odt::CONTENT_PART),
                                    configured: MAX_BLOCK_NESTING.to_string(),
                                });
                            }
                            let address = match (shapes.last(), paragraph) {
                                (Some(shape), Some(paragraph)) => {
                                    shape.page.clone().map(|page| BlockAddress {
                                        page,
                                        shape: shape.index,
                                        shape_name: shape.name.clone(),
                                        paragraph,
                                    })
                                }
                                _ => None,
                            };
                            open.push(OpenBlock {
                                ordinal: opened,
                                heading,
                                text: String::new(),
                                pending_space: false,
                                foreign_depth: 0,
                                foreign_text: false,
                            });
                            addresses.push(address);
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
                    // **The serialization must not move an address.** `<draw:page/>` is a page the
                    // part lists, and `<draw:frame/>` a shape it contains — the defect v2-S4 found
                    // in a slide's `<a:p/>` and v2-S6 found in `<table:table-cell/>`, arriving
                    // here as a draw-page index and a shape index.
                    Structure::Page => pages_opened = advance(pages_opened, "draw page")?,
                    Structure::Frame | Structure::CustomShape => {
                        if let Some(open_page) = pages.last_mut() {
                            open_page.shape_cursor = advance(open_page.shape_cursor, "shape")?;
                        }
                    }
                    Structure::Odf(Element::Block { .. }) => {
                        opened = advance(opened, "block")?;
                        if let Some(shape) = shapes.last_mut() {
                            shape.paragraph_cursor = advance(shape.paragraph_cursor, "block")?;
                        }
                    }
                    // And neither may the rendition slot: `<draw:text-box/>` and its long form are
                    // one document, and they have to agree about which rendition was read.
                    Structure::Odf(Element::TextBox) => {
                        if let Some(shape) = shapes.last_mut() {
                            shape.text_box_taken = true;
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
                    // presentation's own `<draw:page>` is already refused above; a break inside a
                    // shape's paragraph is a second spelling of the same claim, and it contributes
                    // no character either.
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
                    Structure::Page if pages.last().is_some_and(|p| p.from_depth == depth + 1) => {
                        pages.pop();
                    }
                    Structure::Frame | Structure::CustomShape
                        if shapes.last().is_some_and(|s| s.from_depth == depth + 1) =>
                    {
                        let shape = shapes.pop().expect("checked above");
                        if shape.foreign_text {
                            foreign_text_not_read = crate::declare(foreign_text_not_read, 1);
                        }
                    }
                    Structure::Odf(Element::Block { .. }) => {
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth = skip.block_depth.saturating_sub(1);
                        } else if let Some(block) = open.pop() {
                            let address = addresses.pop().flatten();
                            if block.foreign_text {
                                foreign_text_not_read = crate::declare(foreign_text_not_read, 1);
                            }
                            if !block.text.is_empty() {
                                match address {
                                    Some(at) => blocks.push((
                                        block.ordinal,
                                        TextBlock {
                                            draw_page: at.page.index,
                                            draw_page_name: at.page.name,
                                            shape: at.shape,
                                            shape_name: at.shape_name,
                                            paragraph: at.paragraph,
                                            heading: block.heading,
                                            text: block.text,
                                        },
                                    )),
                                    // A block inside a drawing element this slice does not name as
                                    // a shape, or outside every draw page. It has no address, so
                                    // it is declared rather than filed under an index this reader
                                    // would have had to invent.
                                    None => {
                                        text_outside_a_shape =
                                            crate::declare(text_outside_a_shape, 1);
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
                    &mut shapes,
                    &pages,
                    &mut text_outside_a_shape,
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
                    &mut shapes,
                    &pages,
                    &mut text_outside_a_shape,
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
                    &mut shapes,
                    &pages,
                    &mut text_outside_a_shape,
                    resolved,
                ) {
                    push_source(&mut open, resolved, &mut skips, &mut text_bytes)?;
                }
            }
            _ => {}
        }
    }

    check_closed(depth, odt::CONTENT_PART)?;
    // Document order, which nesting alone does not give: an outer block closes after the inner
    // ones it contains, so the pop order is not the file's order.
    blocks.sort_by_key(|(ordinal, _)| *ordinal);
    let blocks = blocks.into_iter().map(|(_, block)| block).collect();

    Ok(Presentation {
        blocks,
        regions_not_read,
        foreign_text_not_read,
        text_outside_a_shape,
    })
}

/// Account for character data that reached no open block.
///
/// Returns whether the characters were handled here, in which case the block engine is not asked.
///
/// **Three silent drops, and the middle one is the ordinary case.** A drawing region's
/// `<svg:title>` sits in no `<text:p>`; a shape's image, embedded object or cached field sits in
/// no `<text:p>` **and** in no region, because in a presentation the shape is the thing and a
/// block is only one of the things inside it; and text loose on a draw page belongs to no shape at
/// all. Under the block-only rule the shared engine applies, all three were neither read nor
/// counted.
fn outside_every_block(
    open: &[OpenBlock],
    skips: &mut [Skip],
    counts_bare: &[bool],
    shapes: &mut [OpenShape],
    pages: &[OpenPage],
    outside: &mut u32,
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
        if has_characters(text) {
            skip.held_text = true;
        }
        return true;
    }
    if !open.is_empty() {
        return false;
    }
    if let Some(shape) = shapes.last_mut() {
        if has_characters(text) {
            shape.foreign_text = true;
        }
        return true;
    }
    // Loose on a draw page: not a conforming construct, and counted rather than assumed away.
    // v2-S4 measured that "no producer writes that" is a claim about producers rather than about
    // documents, so the branch exists.
    if !pages.is_empty() {
        if has_characters(text) {
            *outside = outside.saturating_add(1);
            return true;
        }
        return true;
    }
    false
}

/// Whether these characters are anything but the four ODF collapses.
///
/// The whitespace between two elements is serialization, not erased content.
fn has_characters(text: &str) -> bool {
    text.chars().any(|c| !matches!(c, ' ' | '\t' | '\n' | '\r'))
}

/// Move a 1-based cursor forward, refusing rather than pinning at the ceiling.
///
/// **A saturated cursor is a wrong address, not a large one.** `saturating_add` would give every
/// later shape the same `u32::MAX`, so distinct shapes would share one address — the locator
/// `docs/01-CONTRACT.md` §5.2 calls strictly worse than an absent one. `ods.rs` states the rule
/// this follows, and it applies here for the same reason even though no real presentation could
/// reach it.
fn advance(cursor: u32, what: &str) -> Result<u32, EngineError> {
    cursor.checked_add(1).ok_or_else(|| EngineError::Malformed {
        what: odt::CONTENT_PART.into(),
        detail: format!(
            "this document contains more {what}s than a {what} index can hold. Clamping would \
             give distinct elements one address, so the document is refused rather than addressed \
             wrongly."
        ),
    })
}

/// `draw:name`, resolved, and **optional because OpenDocument makes it optional**.
///
/// `None` is not the empty string. A page or a shape that states no name has stated none, and
/// `""` would be this reader inventing a value to stand in for an absent one — which is the
/// typed-absence discipline `docs/history/14-V2-SCOPE.md` §3 applies to geometry, in a smaller place.
fn draw_name(
    reader: &NsReader<&[u8]>,
    start: &BytesStart<'_>,
) -> Result<Option<String>, EngineError> {
    resolved_attribute(reader, start, NS_DRAW, b"name", odt::CONTENT_PART)
}

/// The block kind, in the contract's own spelling.
pub(crate) fn block_kind(heading: bool) -> OdfBlockKind {
    if heading {
        OdfBlockKind::Heading
    } else {
        OdfBlockKind::Paragraph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real OpenDocument namespace URIs, because this reader resolves them.
    ///
    /// A test that bound `draw:` to something else would exercise nothing: every element would
    /// fall through to [`crate::odt::classify`] and every assertion below would be about the
    /// fallback path rather than the real one.
    const OPEN: &str = concat!(
        r#"<?xml version="1.0"?><office:document-content"#,
        r#" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0""#,
        r#" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0""#,
        r#" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0""#,
        r#" xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0""#,
        r#" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0""#,
        r#" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0""#,
        r#" xmlns:xhtml="http://www.w3.org/1999/xhtml""#,
        r#" xmlns:math="http://www.w3.org/1998/Math/MathML""#,
        r#" xmlns:dc="http://purl.org/dc/elements/1.1/""#,
        r#"><office:body><office:presentation>"#
    );
    const CLOSE: &str = "</office:presentation></office:body></office:document-content>";

    fn body(inner: &str) -> String {
        format!("{OPEN}{inner}{CLOSE}")
    }

    fn read_raw(inner: &str) -> Presentation {
        read_content(body(inner).as_bytes()).expect("the part reads")
    }

    /// One draw page named `p1`, wrapping the shapes given.
    fn read(shapes: &str) -> Presentation {
        read_raw(&format!(
            r#"<draw:page draw:name="p1">{shapes}</draw:page>"#
        ))
    }

    /// A `<draw:frame>` holding one text box holding the blocks given.
    fn framed(name: &str, blocks: &str) -> String {
        format!(
            r#"<draw:frame draw:name="{name}"><draw:text-box>{blocks}</draw:text-box></draw:frame>"#
        )
    }

    fn at(deck: &Presentation, page: u32, shape: u32, paragraph: u32) -> Option<&TextBlock> {
        deck.blocks
            .iter()
            .find(|b| (b.draw_page, b.shape, b.paragraph) == (page, shape, paragraph))
    }

    // ---------------------------------------------------------------------------------------
    // The address
    // ---------------------------------------------------------------------------------------

    /// A block binds at the position the file states, on the page and shape the file lists.
    #[test]
    fn a_block_binds_at_the_position_the_file_states() {
        let deck = read_raw(&format!(
            r#"<draw:page draw:name="Cover">{}{}</draw:page>
               <draw:page draw:name="Detail">{}</draw:page>"#,
            framed("Title 1", "<text:h>The title</text:h>"),
            framed("Body 1", "<text:p>First</text:p><text:p>Second</text:p>"),
            framed("Title 2", "<text:p>Second page</text:p>"),
        ));

        assert_eq!(deck.blocks.len(), 4);
        let title = at(&deck, 1, 1, 1).expect("page one, shape one, block one");
        assert_eq!(title.text, "The title");
        assert!(title.heading, "a `<text:h>` is a heading");
        assert_eq!(title.draw_page_name.as_deref(), Some("Cover"));
        assert_eq!(title.shape_name.as_deref(), Some("Title 1"));

        assert_eq!(at(&deck, 1, 2, 1).map(|b| b.text.as_str()), Some("First"));
        assert_eq!(at(&deck, 1, 2, 2).map(|b| b.text.as_str()), Some("Second"));
        let second = at(&deck, 2, 1, 1).expect("the second page restarts both counters");
        assert_eq!(second.text, "Second page");
        assert_eq!(second.draw_page_name.as_deref(), Some("Detail"));
    }

    /// **A `<draw:page>` is not a page, and the reader has no way to say it is.**
    ///
    /// The type system carries this one — there is no page field to fill — so what is asserted
    /// here is the half a test can reach: the draw-page index is a **position**, and it counts
    /// elements rather than anything a renderer would produce.
    #[test]
    fn the_draw_page_index_counts_elements_and_nothing_else() {
        let deck = read_raw(&format!(
            r#"<draw:page/><draw:page draw:name="Second">{}</draw:page>"#,
            framed("T", "<text:p>On the second page</text:p>"),
        ));
        assert_eq!(
            deck.blocks[0].draw_page, 2,
            "the self-closing page is a page the part lists, so it moves the index"
        );
    }

    /// A page with no `draw:name` states none, and `None` is not `""`.
    #[test]
    fn an_unnamed_page_and_shape_state_no_name_rather_than_an_empty_one() {
        let deck = read_raw(
            r#"<draw:page><draw:frame><draw:text-box><text:p>Anonymous</text:p></draw:text-box></draw:frame></draw:page>"#,
        );
        assert_eq!(deck.blocks.len(), 1);
        assert_eq!(deck.blocks[0].draw_page_name, None);
        assert_eq!(deck.blocks[0].shape_name, None);
    }

    /// An entity in a name survives, because a name is what a person recognises.
    #[test]
    fn an_entity_in_a_name_is_resolved() {
        let deck = read(&framed("Rows &amp; Columns", "<text:p>Body</text:p>"));
        assert_eq!(deck.blocks[0].shape_name.as_deref(), Some("Rows & Columns"));
    }

    /// **The serialization does not move an address**, for shapes as for pages and blocks.
    #[test]
    fn the_serialization_of_an_empty_shape_does_not_move_an_address() {
        let short = read(&format!(
            "<draw:frame/>{}",
            framed("T", "<text:p>Second shape</text:p>")
        ));
        let long = read(&format!(
            "<draw:frame></draw:frame>{}",
            framed("T", "<text:p>Second shape</text:p>")
        ));
        assert_eq!(short.blocks, long.blocks);
        assert_eq!(short.blocks[0].shape, 2);
    }

    /// And neither does an empty block: `<text:p/>` is what a producer writes for a blank line.
    #[test]
    fn the_serialization_of_an_empty_block_does_not_move_an_address() {
        let deck = read(&framed("T", "<text:p/><text:p>Second block</text:p>"));
        assert_eq!(deck.blocks.len(), 1);
        assert_eq!(deck.blocks[0].paragraph, 2);
    }

    /// A shape nested in a group is a shape, and it is counted where it sits (v2-S4's finding).
    #[test]
    fn a_shape_inside_a_group_is_a_shape() {
        let deck = read(&format!(
            "<draw:g>{}</draw:g>{}",
            framed("Grouped", "<text:p>Inside a group</text:p>"),
            framed("Loose", "<text:p>Outside it</text:p>"),
        ));
        assert_eq!(deck.blocks.len(), 2);
        assert_eq!(
            at(&deck, 1, 1, 1).map(|b| b.text.as_str()),
            Some("Inside a group")
        );
        assert_eq!(
            at(&deck, 1, 2, 1).map(|b| b.text.as_str()),
            Some("Outside it")
        );
    }

    /// A `<draw:custom-shape>` holds its blocks directly, with no text box between.
    #[test]
    fn a_custom_shape_holds_its_blocks_directly() {
        let deck = read(
            r#"<draw:custom-shape draw:name="Arrow"><text:p>Drawn text</text:p></draw:custom-shape>"#,
        );
        assert_eq!(deck.blocks.len(), 1);
        assert_eq!(deck.blocks[0].text, "Drawn text");
        assert_eq!(deck.blocks[0].shape_name.as_deref(), Some("Arrow"));
    }

    /// Blocks emitted in **document order**, which the pop order is not.
    #[test]
    fn nested_shapes_emit_in_document_order() {
        let deck = read(&format!(
            r#"<draw:frame draw:name="Outer"><draw:text-box>
                 <text:p>Outer text{}</text:p>
               </draw:text-box></draw:frame>"#,
            framed("Inner", "<text:p>Inner text</text:p>"),
        ));
        assert_eq!(
            deck.blocks
                .iter()
                .map(|b| b.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Outer text", "Inner text"],
            "the outer block opened first, and closes last"
        );
        assert_eq!(deck.blocks[0].shape, 1);
        assert_eq!(
            deck.blocks[1].shape, 2,
            "the nested frame is the page's second shape"
        );
    }

    // ---------------------------------------------------------------------------------------
    // The frame rule, and everything that is counted rather than read
    // ---------------------------------------------------------------------------------------

    /// **The first rendition is the shape's text, and the second is a declared erasure.**
    ///
    /// v2-S6 measured that a spreadsheet's cell atom makes both renditions erasures. A
    /// presentation's atom is the block **inside** the shape, so the ODT outcome returns — and the
    /// count is what makes the two renditions separable at all.
    #[test]
    fn the_first_rendition_is_the_shape_and_the_second_is_declared() {
        let one = read(&framed("T", "<text:p>Displayed</text:p>"));
        assert_eq!(one.blocks.len(), 1);
        assert_eq!(one.blocks[0].text, "Displayed");
        assert_eq!(one.regions_not_read, 0, "one rendition declares nothing");

        let two = read(
            r#"<draw:frame draw:name="T">
                 <draw:text-box><text:p>Displayed</text:p></draw:text-box>
                 <draw:text-box><text:p>ALTERNATIVE</text:p></draw:text-box>
               </draw:frame>"#,
        );
        assert_eq!(two.blocks.len(), 1);
        assert_eq!(two.blocks[0].text, "Displayed");
        assert_eq!(
            two.regions_not_read, 1,
            "the second rendition is one erasure"
        );
        assert!(!two.blocks.iter().any(|b| b.text.contains("ALTERNATIVE")));
    }

    /// A self-closing first rendition still claims the slot, so the second is still the second.
    #[test]
    fn a_self_closing_first_rendition_claims_the_slot() {
        let deck = read(
            r#"<draw:frame draw:name="T">
                 <draw:text-box/>
                 <draw:text-box><text:p>ALTERNATIVE</text:p></draw:text-box>
               </draw:frame>"#,
        );
        assert!(deck.blocks.is_empty(), "the read rendition was empty");
        assert_eq!(
            deck.regions_not_read, 1,
            "and the second is passed over rather than promoted into the empty first's place"
        );
    }

    /// **The passed-over rendition still moves the shape's block counter.**
    #[test]
    fn a_passed_over_rendition_advances_the_block_counter() {
        let deck = read(
            r#"<draw:frame draw:name="T">
                 <draw:text-box><text:p>First</text:p></draw:text-box>
                 <draw:text-box><text:p>ALTERNATIVE</text:p></draw:text-box>
               </draw:frame>
               <draw:frame draw:name="U"><draw:text-box><text:p>Other</text:p></draw:text-box></draw:frame>"#,
        );
        assert_eq!(at(&deck, 1, 1, 1).map(|b| b.text.as_str()), Some("First"));
        assert_eq!(at(&deck, 1, 2, 1).map(|b| b.text.as_str()), Some("Other"));
    }

    /// **Speaker notes are a second stream, counted and not spliced.**
    #[test]
    fn speaker_notes_are_declared_rather_than_read_as_slide_text() {
        let deck = read(&format!(
            "{}<presentation:notes>{}</presentation:notes>",
            framed("T", "<text:p>On the slide</text:p>"),
            framed("N", "<text:p>SPOKEN-ALOUD</text:p>"),
        ));
        assert_eq!(deck.blocks.len(), 1);
        assert_eq!(deck.blocks[0].text, "On the slide");
        assert_eq!(deck.regions_not_read, 1);
        assert!(!deck.blocks.iter().any(|b| b.text.contains("SPOKEN-ALOUD")));
    }

    /// A drawing element this slice does not name is text with no address — declared, not dropped.
    #[test]
    fn text_in_an_unnamed_drawing_shape_is_declared() {
        let deck = read(r#"<draw:rect draw:name="R"><text:p>DRAWN-TEXT</text:p></draw:rect>"#);
        assert!(deck.blocks.is_empty());
        assert_eq!(deck.text_outside_a_shape, 1);
    }

    /// **A shape's foreign subtree holds characters with no block open, and is counted.**
    ///
    /// The ordinary shape of a presentation: `<draw:frame><draw:image><svg:title>` contains no
    /// `<text:p>` at all, so the shared engine's per-block counter never sees it.
    #[test]
    fn a_shapes_image_title_is_counted_rather_than_dropped() {
        let deck = read(
            r#"<draw:frame draw:name="Picture"><draw:image><svg:title>IMAGE-TITLE</svg:title></draw:image></draw:frame>"#,
        );
        assert!(deck.blocks.is_empty());
        assert_eq!(deck.foreign_text_not_read, 1);
    }

    /// The shared allowlist holds through a shape: none of it reaches a block's text.
    #[test]
    fn the_shared_allowlist_holds_through_a_shape() {
        let deck = read(&framed(
            "T",
            r#"<text:p>Fields:<text:page-count>17</text:page-count><text:page-number>4</text:page-number></text:p>
               <text:p><text:ruby><text:ruby-base>kanji</text:ruby-base><text:ruby-text>GUIDE</text:ruby-text></text:ruby></text:p>
               <text:p><text:number>2.1</text:number>Label</text:p>"#,
        ));
        let all: Vec<&str> = deck.blocks.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(all, vec!["Fields:", "kanji", "Label"]);
        assert_eq!(
            deck.foreign_text_not_read, 3,
            "one per block that passed something over"
        );
    }

    /// Namespaces are resolved: a foreign `<xhtml:p>` is not an ODF block.
    #[test]
    fn a_foreign_paragraph_is_not_a_block() {
        let deck = read(&framed("T", "<xhtml:p>NOT-A-BLOCK</xhtml:p>"));
        assert!(deck.blocks.is_empty());
        assert_eq!(
            deck.foreign_text_not_read, 1,
            "its characters reached the shape and were counted there"
        );
    }

    /// MathML's `<annotation>` is not `<office:annotation>`, so it does not erase a region.
    #[test]
    fn mathml_annotation_is_not_an_office_annotation() {
        let deck = read(&framed(
            "T",
            "<text:p>Formula:<math:annotation>SOURCE</math:annotation></text:p>",
        ));
        assert_eq!(deck.blocks.len(), 1);
        assert_eq!(deck.blocks[0].text, "Formula:");
        assert_eq!(deck.regions_not_read, 0, "a region was never entered");
        assert_eq!(deck.foreign_text_not_read, 1);
    }

    /// The stated characters are the file's, and the collapse rule is ODF's.
    #[test]
    fn the_text_is_what_the_file_states() {
        let deck = read(&framed(
            "T",
            r#"<text:p>Rows &amp; columns<text:tab/>tabbed.</text:p>
               <text:p>Three:<text:s text:c="3"/>stated.</text:p>
               <text:p>Broken<text:soft-page-break/> and rejoined.</text:p>"#,
        ));
        let all: Vec<&str> = deck.blocks.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(
            all,
            vec![
                "Rows & columns\ttabbed.",
                "Three:   stated.",
                "Broken and rejoined."
            ]
        );
    }

    /// A block outside every draw page has no address, and is declared.
    #[test]
    fn a_block_outside_every_draw_page_is_declared() {
        let deck = read_raw(&framed("Loose", "<text:p>NO-PAGE</text:p>"));
        assert!(deck.blocks.is_empty());
        assert_eq!(deck.text_outside_a_shape, 1);
    }

    /// A comment inside a shape is `odt.rs`'s region, counted on the stricter rule.
    #[test]
    fn a_comment_in_a_shape_is_declared_once() {
        let deck = read(&framed(
            "T",
            "<text:p>Reviewed<office:annotation><dc:creator>A reviewer</dc:creator><text:p>REMARK</text:p></office:annotation> and unchanged.</text:p>",
        ));
        assert_eq!(deck.blocks.len(), 1);
        assert_eq!(deck.blocks[0].text, "Reviewed and unchanged.");
        assert_eq!(deck.regions_not_read, 1);
    }

    /// **Mismatched nesting is refused, and this reader depends on that.**
    ///
    /// A block's address is fixed when it OPENS and spent when it closes, held in a stack parallel
    /// to the open-block stack. The two can only fall out of step if a `</text:p>` arrives inside a
    /// region its `<text:p>` was outside — which is not well-formed XML. Pinned here rather than
    /// assumed, because the failure it would produce is a node carrying another block's address.
    #[test]
    fn mismatched_nesting_is_refused() {
        let crossed = format!(
            "{OPEN}<draw:page><draw:frame><draw:text-box><text:p>a</draw:text-box></text:p>\
             </draw:frame></draw:page>{CLOSE}"
        );
        assert!(
            read_content(crossed.as_bytes()).is_err(),
            "crossed tags must not parse, or the parallel address stack could desync"
        );
    }

    /// A truncated part is refused rather than read as far as it went.
    #[test]
    fn a_truncated_part_is_refused() {
        let malformed = format!("{OPEN}<draw:page><draw:frame><draw:text-box><text:p>Open forever");
        assert!(read_content(malformed.as_bytes()).is_err());
    }

    /// Detection is exact: a template and a drawing are not presentations.
    #[test]
    fn the_declared_type_is_matched_exactly() {
        for near in [
            "application/vnd.oasis.opendocument.presentation-template",
            "application/vnd.oasis.opendocument.graphics",
            "application/vnd.oasis.opendocument.text",
            "application/epub+zip",
        ] {
            assert_ne!(near, ODP_MEDIA_TYPE);
        }
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// The shape v2-S9's review reproduced in EPUB at 85×, repaired here.
    #[test]
    fn an_odp_erasure_count_saturates_rather_than_wrapping() {
        let deck = read(&framed(
            "T",
            r#"<text:p>Fields:<text:page-count>17</text:page-count></text:p>
               <text:p><text:ruby><text:ruby-base>kanji</text:ruby-base><text:ruby-text>GUIDE</text:ruby-text></text:ruby></text:p>"#,
        ));
        assert_eq!(
            deck.foreign_text_not_read, 2,
            "one per block that passed something over"
        );

        let mut folded = u32::MAX - 1;
        for _ in 0..3 {
            folded = crate::declare(folded, deck.foreign_text_not_read);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );
    }
}
