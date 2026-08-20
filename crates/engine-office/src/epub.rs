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

//! OCF → `container.xml` → the package document → the spine's XHTML (v2-S9).
//!
//! # The format that could have had a page, and does not
//!
//! `docs/14-V2-SCOPE.md` §3's law has always been *"no page this engine did not read from the
//! file"* rather than "no page ever", and every format before this one failed the reading half by
//! construction. A DOCX has no page until a renderer picks one. A slide is a **part**. An ODT's
//! `<text:soft-page-break/>` is a word processor's arithmetic. A `<draw:page>` is structure. RTF's
//! `\page` is a producer's mark.
//!
//! **An EPUB is the first one where the file may genuinely name pages.** An EPUB 3 navigation
//! document can carry a `page-list` mapping locations in the publication to the page numbers of a
//! **print edition**, and an EPUB 2 NCX can carry page targets. Those are page identifiers, written
//! down, readable — so refusing them needed an argument rather than a rule.
//!
//! The argument is that a [`engine_core::PageRecord`] is a page **with a width and a height**, and
//! `check_structure` refuses a measured box on a page-less node precisely because a rectangle
//! nobody can check is the fabrication that law exists to prevent. A publisher's label about
//! somebody else's paper has no geometry at all: nothing could ever be validated against it, and a
//! consumer receiving it as a `PageRecord` would resolve a citation against a rendering this engine
//! never saw. So `pages` stays `[]`, the label is not copied onto a page record under another name,
//! and a navigation document is passed over and **counted** like any other region.
//!
//! # Reading order is the spine, and that is the one decision this reader makes
//!
//! An EPUB is a ZIP of documents, and the shortcut it invites is the one v2-S3 walked into for a
//! workbook: take the XHTML entries in central-directory order, or sorted by name. Both are wrong
//! in **ordinary** files. Reading order lives in the package document's `<spine>`, as
//! `<itemref idref="…">` resolved through the `<manifest>`, and nothing requires a producer's file
//! names to sort that way. `opc.rs` states the rule for OOXML's `r:id`; this is that rule in
//! EPUB's spelling, and the fixture proves it by storing its chapters in the opposite order.
//!
//! Manifest `href`s are resolved **relative to the package document's own directory**, because
//! that is what the specification says they are: `href="chap01.xhtml"` inside `OEBPS/content.opf`
//! is the archive entry `OEBPS/chap01.xhtml`. A reader that took the `href` verbatim would miss
//! every publication that keeps its content in a subdirectory, which is nearly all of them.
//!
//! # The XHTML rule: HTML's own default display, inverted the way `odt.rs` inverts its own
//!
//! `odt.rs` names the handful of inline elements whose characters are the sentence and treats
//! everything else as foreign. XHTML's element set is a **closed vocabulary with a defined default
//! rendering**, so this reader can do something stronger than taste: an element in the XHTML
//! namespace is a **block** when HTML's default style sheet gives it `display: block` (or
//! `list-item`, or a table display), and otherwise it is **inline** — which is HTML's own default
//! for an element nobody has heard of, including a custom element a producer invented.
//!
//! Anything **outside** the XHTML namespace is foreign and is counted, never spliced. That is what
//! keeps an inline `<svg><title>` and a MathML `<annotation>` out of the sentence — the same two
//! constructs v2-S6 and v2-S7 had to name in ODF, arriving here through a namespace rule rather
//! than through a list.
//!
//! A short list of XHTML elements is neither: `script`, `style`, `head`, `template`, `nav` and
//! ruby annotations are **regions**, skipped and counted. `<nav>` is there for the reason above —
//! a `page-list` is exactly the construct §3 forbids minting pages from, and a table of contents
//! duplicates headings the spine documents already carry.
//!
//! # Whitespace is shared, and the three places it is not are written down
//!
//! XHTML's `white-space: normal` collapses a run of spaces, tabs, carriage returns and line feeds
//! to one space and drops one at either end of a block. Those four characters and that rule are
//! what [`crate::odt`] already implements for ODF, so its block engine is **imported** rather than
//! restated — one place where a whitespace defect can live, for the reason the shared allowlist
//! exists.
//!
//! It is not a perfect identity, and saying "the rules are the same" would be the kind of claim
//! this engine refuses to make about anything it has not checked. Three divergences, each stated
//! rather than approximated:
//!
//! - **`<pre>`.** `white-space: pre` is the document saying those spaces are content, so its
//!   characters take the path `odt.rs` uses for a stated `<text:s>` and survive verbatim. Handled.
//! - **A form feed.** HTML counts `U+000C` as ASCII whitespace and ODF does not, so one inside a
//!   block is passed through instead of collapsed. Left alone rather than added to the shared
//!   rule, because widening that rule would change what three shipped ODF readers do with a
//!   character none of this slice's measurements were about.
//! - **A line break between two CJK characters.** CSS removes it rather than turning it into a
//!   space; this reader turns it into a space. That is a real difference for a book set in
//!   Chinese, Japanese or Korean, and it is **not** approximated here: the correct rule needs the
//!   computed `white-space` value and the scripts on both sides, and this reader reads no style
//!   sheet. Recorded in `docs/15-V2-MILESTONES.md` S9 as the widest gap this slice knowingly
//!   leaves.
//!
//! **No style sheet is read at all**, which is the general case those three are instances of. A
//! book may set `white-space`, hide a block with `display: none`, reorder blocks, or insert text
//! through `::before`. None of that is applied, so the text here is what the document *states*
//! rather than what a reading system would show.
//!
//! # What this reader was and was not measured against
//!
//! **No corpus of real `.epub` files was available** — the fifth consecutive slice that has to say
//! so, repeated rather than quietly inherited. Every rule here is read off the OCF, EPUB Packages
//! and XHTML specifications and pinned against publications this repository authors byte by byte.

use engine_core::EngineError;
use quick_xml::events::{BytesStart, Event};
use quick_xml::NsReader;

use crate::odt::{
    self, namespace_of, push_source, push_stated, OpenBlock, Skip, MAX_BLOCK_NESTING,
};
use crate::xml::{
    cdata_text, check_closed, decode, local_name, new_ns_reader, parse_error_at, resolve_reference,
    unprefixed_attribute,
};

/// The media type an EPUB publication declares.
pub const EPUB_MEDIA_TYPE: &str = "application/epub+zip";

/// The entry OCF requires every publication to name its package document in.
pub const CONTAINER_PART: &str = "META-INF/container.xml";

/// The entry OCF puts encryption declarations in, when a publication has any.
pub const ENCRYPTION_PART: &str = "META-INF/encryption.xml";

/// The media type a package document declares itself as, in `container.xml`.
const PACKAGE_MEDIA_TYPE: &str = "application/oebps-package+xml";

/// The media type a spine document this reader can read declares itself as.
const XHTML_MEDIA_TYPE: &str = "application/xhtml+xml";

/// The OCF container namespace, which EPUB borrowed from OpenDocument and never renamed.
const NS_CONTAINER: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:container";

/// The package document's namespace.
const NS_OPF: &[u8] = b"http://www.idpf.org/2007/opf";

/// XHTML's namespace, which is what makes an inline `<svg><title>` foreign rather than a heading.
const NS_XHTML: &[u8] = b"http://www.w3.org/1999/xhtml";

/// XML Encryption's namespace, used by `META-INF/encryption.xml`.
const NS_XMLENC: &[u8] = b"http://www.w3.org/2001/04/xmlenc#";

/// A ceiling on the spine items this reader will open.
///
/// Not a judgement about long books: a package document is a list of *references*, so a spine that
/// names one manifest item ten thousand times would open one archive entry ten thousand times. The
/// duplicate refusal below already forbids that particular shape; this bounds the general one, on
/// `zip.rs`'s rule that a bomb is "a named refusal rather than an out-of-memory kill".
const MAX_SPINE_ITEMS: usize = 4096;

/// A ceiling on the manifest items this reader will hold.
///
/// Separate from the spine's, and larger, because a manifest legitimately declares far more than
/// the spine references — every image, font and style sheet in a publication. It is bounded because
/// a package document is bounded only by `zip.rs`'s inflate ceiling, which admits millions of
/// `<item>` elements, and [`MAX_SPINE_ITEMS`] does not reach the list the spine resolves against.
const MAX_MANIFEST_ITEMS: usize = 65_536;

/// One block of an XHTML document, with the position that document states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// 1-based position among the document's block elements, counting the ones inside regions this
    /// reader does not read.
    pub ordinal: u32,
    /// The XHTML element's local name, verbatim.
    pub element: String,
    /// The text at this block's own level, under XHTML's whitespace rule, and **without** the text
    /// of any block nested inside it.
    pub text: String,
}

/// One spine document, with the address the package states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpineDocument {
    /// The archive entry this document was read from, resolved through the manifest.
    pub part: String,
    /// Whether the spine lists it as part of the linear reading order.
    pub linear: bool,
    /// The blocks that carry text, in the document's own order.
    pub blocks: Vec<Block>,
}

/// What one publication yielded, plus what it passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    /// The archive entry the package document was read from, as `container.xml` names it.
    pub package_part: String,
    /// The spine's documents, in the spine's own order.
    pub documents: Vec<SpineDocument>,
    /// Every archive entry this reader consumed, so the rest can be counted.
    pub read_entries: Vec<String>,
    /// Regions of a spine document that held text and were not read: a script, a style sheet, a
    /// navigation document's lists, a `<head>`, a template, a ruby annotation.
    pub regions_not_read: u32,
    /// Blocks that contained a subtree outside the XHTML namespace holding characters: an inline
    /// SVG's title, a MathML annotation.
    pub foreign_text_not_read: u32,
    /// Character data that reached no block, so there is no address this reader could cite it at.
    pub text_outside_a_block: u32,
    /// Spine items whose declared media type this slice does not read.
    pub spine_items_not_read: u32,
    /// Rootfiles after the first, which are additional **renditions** of the same publication.
    pub extra_renditions: u32,
}

/// Whether these bytes are an EPUB publication, **read from the bytes** (**A4**).
///
/// The OCF question [`crate::odt::is_odt`] asks, against a different declared type: a first,
/// **stored** entry named `mimetype` whose content is exactly [`EPUB_MEDIA_TYPE`].
///
/// **The container rule is shared and the family is not.** OCF is where OpenDocument's packaging
/// came from, so an `.epub` and an `.odt` are told apart by *what they declare*, never by the shape
/// of the declaration. `crate::is_opendocument` answers on the declared type for exactly this
/// reason (v2-S6), and an EPUB is never told it is OpenDocument.
///
/// Exact rather than prefixed. Never the extension, in both directions: a publication named
/// `book.bin` reads, and an `.epub` full of something else does not.
pub fn is_epub(bytes: &[u8]) -> bool {
    odt::declared_media_type(bytes).is_some_and(|declared| declared == EPUB_MEDIA_TYPE)
}

/// Read a publication: the container chain, then the spine's documents.
///
/// # Errors
///
/// [`EngineError::MissingPart`] if `container.xml`, the package document or a spine document is
/// absent. [`EngineError::Malformed`] if any of them will not parse, if `container.xml` names no
/// package document, if the spine is empty, if an `<itemref>` names no manifest item, if the spine
/// lists one item twice, or if an `href` leaves the container. [`EngineError::Unsupported`] if a
/// spine document is encrypted. [`EngineError::ResourceLimit`] if the spine exceeds
/// [`MAX_SPINE_ITEMS`] or the text exceeds [`crate::odt::MAX_TEXT_BYTES`].
pub fn read(bytes: &[u8]) -> Result<Publication, EngineError> {
    let container =
        crate::zip::read_entry(bytes, CONTAINER_PART).map_err(|_| EngineError::MissingPart {
            part: format!(
                "`{CONTAINER_PART}` — an OCF container names its package document only here, and \
                 a publication without one is one this reader cannot say it has read"
            ),
        })?;
    let rootfiles = read_container(&container)?;
    let Some(package_part) = rootfiles.first().cloned() else {
        return Err(EngineError::Malformed {
            what: CONTAINER_PART.into(),
            detail: "this container declares no package document of media type \
                     `application/oebps-package+xml`. That element is where a publication states \
                     which entry holds its manifest and spine, so without one there is no reading \
                     order this reader could stand behind."
                .into(),
        });
    };

    let package =
        crate::zip::read_entry(bytes, &package_part).map_err(|_| EngineError::MissingPart {
            part: format!(
                "`{package_part}` — `{CONTAINER_PART}` names this entry as the package document \
                 and the archive does not contain it"
            ),
        })?;
    let spine = read_package(&package, &package_part)?;

    // **Encryption is checked against the spine, not against its own presence.** Nearly every real
    // publication that carries `META-INF/encryption.xml` carries it for font obfuscation, and
    // refusing those would refuse books whose text is in the clear. What must never happen is
    // handing ciphertext to the XML reader, which would report malformed XML and name the wrong
    // cause — the defect v2-S6 fixed for `%PDF-` in a different shape.
    let encrypted = match crate::zip::read_entry(bytes, ENCRYPTION_PART) {
        Ok(part) => read_encryption(&part)?,
        Err(_) => Default::default(),
    };

    let mut read_entries = vec![
        odt::MIMETYPE_ENTRY.to_string(),
        CONTAINER_PART.to_string(),
        package_part.clone(),
    ];
    let mut documents = Vec::new();
    let mut regions_not_read = 0u32;
    let mut foreign_text_not_read = 0u32;
    let mut text_outside_a_block = 0u32;
    let mut spine_items_not_read = 0u32;
    // One budget across the whole publication, because that is where the expansion is: `zip.rs`
    // bounds each entry it inflates, and nothing bounds the number of entries but the spine.
    let mut text_bytes = 0usize;

    for item in &spine {
        if !item.readable {
            spine_items_not_read = spine_items_not_read.saturating_add(1);
            continue;
        }
        if encrypted.contains(&item.part) {
            return Err(EngineError::Unsupported {
                what: "encrypted EPUB spine document".into(),
                detail: format!(
                    "`{ENCRYPTION_PART}` declares `{}` encrypted. This reader holds no key and \
                     decrypts nothing; refused by name rather than handed to the XML reader, \
                     which would report the ciphertext as malformed XML and name the wrong cause.",
                    item.part
                ),
            });
        }
        let document =
            crate::zip::read_entry(bytes, &item.part).map_err(|_| EngineError::MissingPart {
                part: format!(
                    "`{}` — the spine lists it and the archive does not contain it, so a document \
                     this publication states it has could not be read",
                    item.part
                ),
            })?;
        let read = read_document(&document, &item.part, &mut text_bytes)?;
        // **Saturating, because a wrapped erasure count is a silent drop presented as a success.**
        // Each of these is bounded only by its document's event count, and a spine may hold
        // thousands of documents — so a plain `+=` can pass `u32::MAX` and come back small, which
        // is an artifact reporting that it erased almost nothing while it erased four billion
        // things. A14's whole content is that the number is honest. Saturating over-declares at
        // the ceiling, which is the direction this reader takes everywhere else.
        regions_not_read = regions_not_read.saturating_add(read.regions_not_read);
        foreign_text_not_read = foreign_text_not_read.saturating_add(read.foreign_text_not_read);
        text_outside_a_block = text_outside_a_block.saturating_add(read.text_outside_a_block);
        read_entries.push(item.part.clone());
        documents.push(SpineDocument {
            part: item.part.clone(),
            linear: item.linear,
            blocks: read.blocks,
        });
    }

    Ok(Publication {
        package_part,
        documents,
        read_entries,
        regions_not_read,
        foreign_text_not_read,
        text_outside_a_block,
        spine_items_not_read,
        extra_renditions: u32::try_from(rootfiles.len().saturating_sub(1)).unwrap_or(u32::MAX),
    })
}

/// How many archive entries hold content this slice did not read (**A14**).
///
/// Everything except the entries the reader consumed and the directory entries a writer may
/// record. That deliberately over-counts — a `.ncx` whose whole content is the table of contents
/// the navigation document already carries is counted too — and over-counting is the direction A14
/// asks for: the number tells a caller there is more in the package.
pub fn unread_entries(entry_names: &[String], read: &[String]) -> u32 {
    entry_names
        .iter()
        .filter(|name| !name.ends_with('/') && !read.iter().any(|seen| seen == *name))
        .count() as u32
}

// -------------------------------------------------------------------------------------------
// The container chain
// -------------------------------------------------------------------------------------------

/// Read `META-INF/container.xml` into the package documents it names, in its own order.
///
/// The **first** is the publication's default rendition, which is what the specification calls it;
/// any others are additional renditions of the same work and are counted rather than read.
///
/// Names are resolved against [`NS_CONTAINER`] rather than suffix-matched. `rootfile` and
/// `full-path` are names other vocabularies use, and both feed an **address** — the rule
/// `docs/15-V2-MILESTONES.md` S5 states, and the one `ods.rs` re-argues for `table:name`.
fn read_container(part: &[u8]) -> Result<Vec<String>, EngineError> {
    let mut reader = new_ns_reader(part, CONTAINER_PART)?;
    let mut rootfiles: Vec<String> = Vec::new();
    let mut depth: i32 = 0;

    loop {
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => return Err(parse_error_at(reader.buffer_position(), CONTAINER_PART, &e)),
        };
        let namespace = namespace_of(&resolved).map(|ns| ns.to_vec());
        let namespace = namespace.as_deref();

        // A `<rootfile/>` is what every writer emits, and `<rootfile></rootfile>` is the same
        // declaration — so both forms take one path. Serialization does not move an address.
        let start = match &event {
            Event::Start(start) => {
                depth += 1;
                Some(start)
            }
            Event::Empty(start) => Some(start),
            Event::End(_) => {
                depth -= 1;
                None
            }
            Event::Eof => break,
            _ => None,
        };

        if let Some(start) = start {
            let qualified = start.name();
            if namespace == Some(NS_CONTAINER) && local_name(qualified.as_ref()) == b"rootfile" {
                let media = unprefixed_attribute(&reader, start, b"media-type", CONTAINER_PART)?;
                // Only a package document. A container may point at other things, and reading one
                // of them as a manifest would produce a publication with no spine and no error.
                if media.as_deref() == Some(PACKAGE_MEDIA_TYPE) {
                    let path = unprefixed_attribute(&reader, start, b"full-path", CONTAINER_PART)?
                        .ok_or_else(|| EngineError::Malformed {
                            what: CONTAINER_PART.into(),
                            detail: "a `<rootfile>` carries no `full-path`. That attribute is the \
                                     only place a publication names its package document, so a \
                                     rootfile without one points nowhere — and choosing an entry \
                                     for it would be this reader inventing the publication's own \
                                     structure."
                                .into(),
                        })?;
                    rootfiles.push(normalize(&path, "", CONTAINER_PART)?);
                }
            }
        }
    }

    check_closed(depth, CONTAINER_PART)?;
    Ok(rootfiles)
}

/// One spine entry, resolved to the archive entry it names.
#[derive(Debug)]
struct SpineItem {
    part: String,
    linear: bool,
    /// Whether the manifest declares a media type this slice reads.
    readable: bool,
}

/// Read a package document into its spine, resolved through its manifest.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, if a manifest `<item>` carries no `id` or
/// no `href`, if two items share an `id`, if the spine is empty, if an `<itemref>` names an `id`
/// the manifest does not declare, or if the spine lists one item twice.
/// [`EngineError::ResourceLimit`] if the spine exceeds [`MAX_SPINE_ITEMS`].
fn read_package(part: &[u8], part_name: &str) -> Result<Vec<SpineItem>, EngineError> {
    // The directory the package document sits in, which every `href` is relative to.
    let base = part_name.rsplit_once('/').map_or("", |(dir, _)| dir);

    let mut reader = new_ns_reader(part, part_name)?;
    // Manifest order, so a duplicate `id` is caught rather than overwritten.
    let mut items: Vec<(String, String, bool)> = Vec::new();
    // **A set beside the list, because the duplicate check must not be a linear scan.** The list's
    // length is the package document's, which `zip.rs` bounds at 256 MiB — millions of items — and
    // scanning it once per item is quadratic. The spine's cap does not reach the list the spine
    // resolves *against*, so this one is bounded separately below.
    let mut seen_ids: std::collections::BTreeSet<String> = Default::default();
    let mut spine: Vec<(String, bool)> = Vec::new();
    let mut saw_spine = false;
    let mut depth: i32 = 0;

    loop {
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => return Err(parse_error_at(reader.buffer_position(), part_name, &e)),
        };
        let namespace = namespace_of(&resolved).map(|ns| ns.to_vec());
        let namespace = namespace.as_deref();

        let start = match &event {
            Event::Start(start) => {
                depth += 1;
                Some(start)
            }
            Event::Empty(start) => Some(start),
            Event::End(_) => {
                depth -= 1;
                None
            }
            Event::Eof => break,
            _ => None,
        };

        let Some(start) = start else { continue };
        if namespace != Some(NS_OPF) {
            continue;
        }
        let qualified = start.name();
        match local_name(qualified.as_ref()) {
            b"item" => {
                let id = required(
                    &reader,
                    start,
                    b"id",
                    part_name,
                    "a manifest `<item>`",
                    "id",
                )?;
                let href = required(
                    &reader,
                    start,
                    b"href",
                    part_name,
                    "a manifest `<item>`",
                    "href",
                )?;
                if items.len() >= MAX_MANIFEST_ITEMS {
                    return Err(EngineError::ResourceLimit {
                        limit: format!("manifest items in `{part_name}`"),
                        configured: MAX_MANIFEST_ITEMS.to_string(),
                    });
                }
                if !seen_ids.insert(id.clone()) {
                    return Err(EngineError::Malformed {
                        what: part_name.into(),
                        detail: format!(
                            "this manifest declares more than one `<item>` with `id=\"{id}\"`. The \
                             spine addresses items by that id, so two of them give one `<itemref>` \
                             two answers — the defect v2-S4 measured in a slide's shape id, \
                             arriving here in the identifier a publication's reading order is \
                             built on."
                        ),
                    });
                }
                let media = unprefixed_attribute(&reader, start, b"media-type", part_name)?;
                let readable = media.as_deref() == Some(XHTML_MEDIA_TYPE);
                items.push((id, normalize(&href, base, part_name)?, readable));
            }
            b"spine" => saw_spine = true,
            b"itemref" => {
                let idref = required(
                    &reader,
                    start,
                    b"idref",
                    part_name,
                    "a spine `<itemref>`",
                    "idref",
                )?;
                // `linear` defaults to `yes`, and anything that is not exactly `no` is the
                // default — read rather than guessed, because the attribute is enumerated.
                let linear = unprefixed_attribute(&reader, start, b"linear", part_name)?
                    .is_none_or(|stated| stated != "no");
                if spine.len() >= MAX_SPINE_ITEMS {
                    return Err(EngineError::ResourceLimit {
                        limit: format!("spine items in `{part_name}`"),
                        configured: MAX_SPINE_ITEMS.to_string(),
                    });
                }
                spine.push((idref, linear));
            }
            _ => {}
        }
    }

    check_closed(depth, part_name)?;

    if !saw_spine || spine.is_empty() {
        return Err(EngineError::Malformed {
            what: part_name.into(),
            detail: "this package document declares no spine items. The spine is where a \
                     publication states its reading order, so without one there is no order this \
                     reader could follow — and taking the manifest's items in archive order would \
                     be a guess that attaches the right content to the wrong position."
                .into(),
        });
    }

    let mut resolved_spine: Vec<SpineItem> = Vec::new();
    for (idref, linear) in spine {
        let Some((_, part, readable)) = items.iter().find(|(id, _, _)| id == &idref) else {
            return Err(EngineError::Malformed {
                what: part_name.into(),
                detail: format!(
                    "the spine lists `<itemref idref=\"{idref}\">` and the manifest declares no \
                     item with that id. Only the manifest says which archive entry an id means, so \
                     an unresolvable reference is a document this reader cannot address — and \
                     guessing an entry for it is the confidently-wrong locator \
                     `docs/01-CONTRACT.md` §5.2 forbids."
                ),
            });
        };
        // **One part name, one part id.** `check_structure` proves that bijection over the whole
        // artifact, so a spine that reached the same entry twice would be refused there with a
        // message about part ids. Refused here instead, where the cause is visible.
        if resolved_spine.iter().any(|item| &item.part == part) {
            return Err(EngineError::Malformed {
                what: part_name.into(),
                detail: format!(
                    "the spine reaches `{part}` more than once. A part name is half of this \
                     artifact's address, so one entry appearing twice would give two positions one \
                     name and leave a citation with two answers."
                ),
            });
        }
        resolved_spine.push(SpineItem {
            part: part.clone(),
            linear,
            readable: *readable,
        });
    }
    Ok(resolved_spine)
}

/// Read `META-INF/encryption.xml` into the archive entries it declares encrypted.
fn read_encryption(part: &[u8]) -> Result<std::collections::BTreeSet<String>, EngineError> {
    let mut reader = new_ns_reader(part, ENCRYPTION_PART)?;
    // A set, and capped, for `read_package`'s two reasons: the membership test below runs once per
    // spine item, and the list's length is an input the publication controls.
    let mut encrypted: std::collections::BTreeSet<String> = Default::default();
    let mut depth: i32 = 0;

    loop {
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => {
                return Err(parse_error_at(
                    reader.buffer_position(),
                    ENCRYPTION_PART,
                    &e,
                ))
            }
        };
        let namespace = namespace_of(&resolved).map(|ns| ns.to_vec());
        let namespace = namespace.as_deref();

        let start = match &event {
            Event::Start(start) => {
                depth += 1;
                Some(start)
            }
            Event::Empty(start) => Some(start),
            Event::End(_) => {
                depth -= 1;
                None
            }
            Event::Eof => break,
            _ => None,
        };

        if let Some(start) = start {
            let qualified = start.name();
            if namespace == Some(NS_XMLENC) && local_name(qualified.as_ref()) == b"CipherReference"
            {
                if let Some(uri) = unprefixed_attribute(&reader, start, b"URI", ENCRYPTION_PART)? {
                    if encrypted.len() >= MAX_MANIFEST_ITEMS {
                        return Err(EngineError::ResourceLimit {
                            limit: format!("encrypted resources in `{ENCRYPTION_PART}`"),
                            configured: MAX_MANIFEST_ITEMS.to_string(),
                        });
                    }
                    encrypted.insert(normalize(&uri, "", ENCRYPTION_PART)?);
                }
            }
        }
    }

    check_closed(depth, ENCRYPTION_PART)?;
    Ok(encrypted)
}

/// One attribute a construct cannot be addressed without.
fn required(
    reader: &NsReader<&[u8]>,
    start: &BytesStart<'_>,
    want: &[u8],
    part_name: &str,
    what: &str,
    attribute: &str,
) -> Result<String, EngineError> {
    unprefixed_attribute(reader, start, want, part_name)?.ok_or_else(|| EngineError::Malformed {
        what: part_name.into(),
        detail: format!(
            "{what} carries no `{attribute}`. That attribute is how this publication states \
                 its own structure, so an element without one binds nothing — and supplying a \
                 value would be this reader inventing the half the file does not state."
        ),
    })
}

/// Resolve a package-relative reference to the archive entry it names.
///
/// # Three rules, and each closes a way of reading the wrong entry
///
/// 1. **Relative to the referring document's directory**, which is what the specification says an
///    `href` is. `chap01.xhtml` in `OEBPS/content.opf` is `OEBPS/chap01.xhtml`.
/// 2. **Percent-decoded per segment**, because these are IRI references and a publication with a
///    space in a file name writes `chap%2001.xhtml`. Decoding a whole path at once would let
///    `%2F` invent a segment boundary; decoding each segment separately leaves it a literal
///    character inside the name, which simply matches no archive entry.
/// 3. **Never above the container root.** A `..` that walks out is refused rather than clamped: a
///    reference outside the package is not a package part, and resolving it to something inside
///    would be this reader choosing an entry the publication did not name.
///
/// A fragment is dropped — it addresses a position *within* a document, and the document is what
/// is being named here. An absolute or remote reference is refused for the same reason a `..`
/// escape is.
fn normalize(reference: &str, base: &str, part_name: &str) -> Result<String, EngineError> {
    let reference = reference.split('#').next().unwrap_or(reference);
    if reference.is_empty() {
        return Err(outside(reference, part_name));
    }
    if reference.starts_with('/') || reference.contains("://") {
        return Err(outside(reference, part_name));
    }

    let mut segments: Vec<String> = if base.is_empty() {
        Vec::new()
    } else {
        base.split('/').map(str::to_string).collect()
    };
    for raw in reference.split('/') {
        match raw {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(outside(reference, part_name));
                }
            }
            segment => segments.push(percent_decode(segment, part_name)?),
        }
    }
    if segments.is_empty() {
        return Err(outside(reference, part_name));
    }
    Ok(segments.join("/"))
}

fn outside(reference: &str, part_name: &str) -> EngineError {
    EngineError::Malformed {
        what: part_name.into(),
        detail: format!(
            "`{reference}` does not name an entry inside this container. A reference that is \
             absolute, remote, or walks above the package root is not a package part, and \
             resolving it to something inside would be this reader choosing an entry the \
             publication did not name."
        ),
    }
}

/// Percent-decode one path segment, refusing a sequence that is not UTF-8.
///
/// A `%` that does not introduce two hexadecimal digits is a literal `%`, which is what every IRI
/// consumer does with one — and the archive entry it then fails to match is a named missing part
/// rather than a guess.
fn percent_decode(segment: &str, part_name: &str) -> Result<String, EngineError> {
    if !segment.contains('%') {
        return Ok(segment.to_string());
    }
    let raw = segment.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(raw.len());
    let mut at = 0;
    while at < raw.len() {
        // **Bytes throughout, and never a `&str` slice.** A reference may hold any UTF-8 — a file
        // name with a euro sign in it is legal — so indexing the string by byte offset would land
        // inside a multi-byte scalar and panic. A reader whose whole contract is a named refusal
        // must not have an input that crashes it.
        if raw[at] == b'%' {
            if let Some(byte) = raw
                .get(at + 1..at + 3)
                .and_then(|hex| std::str::from_utf8(hex).ok())
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                out.push(byte);
                at += 3;
                continue;
            }
        }
        out.push(raw[at]);
        at += 1;
    }
    String::from_utf8(out).map_err(|e| EngineError::Malformed {
        what: part_name.into(),
        detail: format!(
            "`{segment}` percent-decodes to bytes that are not UTF-8: {e}. An archive entry name \
             is text, so a reference this reader cannot decode is one it cannot resolve — and \
             replacing the undecodable bytes would name a different entry."
        ),
    })
}

// -------------------------------------------------------------------------------------------
// The XHTML documents
// -------------------------------------------------------------------------------------------

/// What one element of a spine document means to this reader.
///
/// # HTML's own default display, which is stronger than a list
///
/// `odt.rs` names the inline elements whose characters are the sentence and calls everything else
/// foreign, because ODF has no default rendering to appeal to. XHTML does: its element set is a
/// closed vocabulary and HTML defines a default style sheet for it. So [`Self::Block`] is the set
/// HTML gives `display: block`, `list-item` or a table display, and **an XHTML element this list
/// does not name is [`Self::Inline`]** — which is HTML's own default for an element nobody has
/// heard of, including a custom element a producer invented. That is a rule read off the
/// specification rather than a preference, and it is why this list can be finite without the
/// failure mode a finite list usually has.
///
/// The namespace is what does the real work. Anything **outside** XHTML is [`Self::Foreign`]: an
/// inline `<svg>`'s `<title>` and a MathML `<annotation>` are the two constructs v2-S6 and v2-S7
/// each had to name by hand in ODF, and here they fall out of the namespace rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Element {
    /// A flow element whose own-level characters are a node.
    Block,
    /// `pre` — a block whose whitespace is the document's, not the serializer's.
    Preserve,
    /// A region whose characters are not body text, skipped and counted.
    ///
    /// `strict` follows `odt.rs`'s note rule: count only characters inside the region's own
    /// blocks. **Every** XHTML `<head>` carries a `<title>`, so counting bare characters there
    /// would declare an erasure on every document that has had nothing removed — which is the
    /// exact argument `odt.rs` makes about a note's `<text:note-citation>`.
    Region { strict: bool },
    /// An element whose characters **are** the enclosing block's text.
    Inline,
    /// `br` — a line feed the element states.
    Break,
    /// Outside the XHTML namespace. Its characters are not the block's text.
    Foreign,
}

/// HTML's `display: block`, `list-item` and table displays, which is what makes a node here.
const BLOCK_ELEMENTS: &[&[u8]] = &[
    b"address",
    b"article",
    b"aside",
    b"blockquote",
    b"body",
    b"caption",
    b"center",
    b"dd",
    b"details",
    b"dialog",
    b"dir",
    b"div",
    b"dl",
    b"dt",
    b"fieldset",
    b"figcaption",
    b"figure",
    b"footer",
    b"form",
    b"h1",
    b"h2",
    b"h3",
    b"h4",
    b"h5",
    b"h6",
    b"header",
    b"hgroup",
    b"hr",
    b"legend",
    b"li",
    b"main",
    b"menu",
    b"ol",
    b"optgroup",
    b"option",
    b"p",
    b"section",
    b"summary",
    b"table",
    b"tbody",
    b"td",
    b"tfoot",
    b"th",
    b"thead",
    b"tr",
    b"ul",
];

/// Regions whose characters are not body text, and whether they count bare characters.
const REGION_ELEMENTS: &[(&[u8], bool)] = &[
    // Strict: a `<title>` is metadata every document has, so bare characters here are not an
    // erasure of anything.
    (b"head", true),
    (b"title", true),
    // Bare: a script's source, a style sheet's rules, a navigation document's lists, a template's
    // inert content and a ruby annotation are all characters in the document that this artifact
    // does not carry, and none of them is metadata every document has.
    (b"script", false),
    (b"style", false),
    (b"template", false),
    (b"noscript", false),
    (b"nav", false),
    (b"rt", false),
    (b"rp", false),
];

fn classify(namespace: Option<&[u8]>, local: &[u8]) -> Element {
    if namespace != Some(NS_XHTML) {
        return Element::Foreign;
    }
    if local == b"pre" {
        return Element::Preserve;
    }
    if local == b"br" {
        return Element::Break;
    }
    if let Some((_, strict)) = REGION_ELEMENTS.iter().find(|(name, _)| *name == local) {
        return Element::Region { strict: *strict };
    }
    if BLOCK_ELEMENTS.contains(&local) {
        return Element::Block;
    }
    Element::Inline
}

/// What one spine document yielded.
#[derive(Debug)]
struct DocumentRead {
    blocks: Vec<Block>,
    regions_not_read: u32,
    foreign_text_not_read: u32,
    text_outside_a_block: u32,
}

/// Parallel to the open-block stack: what the block will be called, fixed when it opened.
struct BlockMeta {
    ordinal: u32,
    element: String,
}

/// Read one XHTML spine document into the blocks that carry text.
fn read_document(
    part: &[u8],
    part_name: &str,
    text_bytes: &mut usize,
) -> Result<DocumentRead, EngineError> {
    let mut reader = new_ns_reader(part, part_name)?;
    let mut blocks: Vec<(u32, Block)> = Vec::new();
    let mut regions_not_read = 0u32;
    let mut foreign_text_not_read = 0u32;
    let mut text_outside_a_block = 0u32;

    let mut open: Vec<OpenBlock> = Vec::new();
    // Pushed and popped in lockstep with `open`. XHTML nests blocks freely — a `<td>` inside a
    // `<table>` inside a `<div>` — and an inner block closes before the outer one that contains
    // it, so the element a block is named after has to be fixed when it opens.
    let mut meta: Vec<BlockMeta> = Vec::new();
    let mut skips: Vec<Skip> = Vec::new();
    // Parallel to `skips`: whether this region counts bare character data. See [`Element::Region`].
    let mut skip_strict: Vec<bool> = Vec::new();

    let mut depth: i32 = 0;
    // 1-based, advanced for **every** block the document contains — including ones inside a region
    // this reader passes over, because the address is a position in the document.
    let mut opened: u32 = 0;
    // How many `<pre>` are open. `white-space: pre` is the document stating that its spaces are
    // content, so those characters take the path `odt.rs` uses for a stated `<text:s>`.
    let mut preserving: u32 = 0;

    loop {
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => return Err(parse_error_at(reader.buffer_position(), part_name, &e)),
        };
        let namespace = namespace_of(&resolved).map(|ns| ns.to_vec());
        let namespace = namespace.as_deref();

        match event {
            Event::Eof => break,

            Event::Start(start) => {
                depth += 1;
                let qualified = start.name();
                let local = local_name(qualified.as_ref()).to_vec();
                let element = classify(namespace, &local);

                if let Element::Region { strict } = element {
                    // **Capped beside the block stack, not instead of it.** These regions nest —
                    // a `<nav>` inside a `<template>` — and nesting depth is bounded only by the
                    // part's length, so an unbounded stack here is the out-of-memory kill
                    // `zip.rs` says a bomb must not be. `<rt>` costs four bytes a level.
                    if skips.len() >= MAX_BLOCK_NESTING {
                        return Err(EngineError::ResourceLimit {
                            limit: format!("nested regions in `{part_name}`"),
                            configured: MAX_BLOCK_NESTING.to_string(),
                        });
                    }
                    skips.push(Skip {
                        from_depth: depth,
                        held_text: false,
                        block_depth: 0,
                    });
                    // **Strict only where a document actually puts its head.** The rule exists so
                    // that the `<title>` every XHTML document carries is not an erasure on every
                    // document — and that argument holds only in the prologue, before any block
                    // has opened. A `<title>` inside the body is not metadata, and it takes the
                    // ordinary bare rule so its characters are counted rather than lost.
                    skip_strict.push(strict && open.is_empty());
                }

                match element {
                    Element::Block | Element::Preserve => {
                        opened = advance(opened, part_name)?;
                        if element == Element::Preserve {
                            preserving += 1;
                        }
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth += 1;
                        } else {
                            if open.len() >= MAX_BLOCK_NESTING {
                                return Err(EngineError::ResourceLimit {
                                    limit: format!("nested blocks in `{part_name}`"),
                                    configured: MAX_BLOCK_NESTING.to_string(),
                                });
                            }
                            open.push(OpenBlock {
                                ordinal: opened,
                                // ODF's word for one of its two block elements. XHTML has no such
                                // distinction, so it is left false and the element's own name is
                                // carried beside it instead.
                                heading: false,
                                text: String::new(),
                                pending_space: false,
                                foreign_depth: 0,
                                foreign_text: false,
                            });
                            meta.push(BlockMeta {
                                ordinal: opened,
                                element: String::from_utf8_lossy(&local).into_owned(),
                            });
                        }
                    }
                    Element::Break => push_stated(&mut open, "\n", &mut skips, text_bytes)?,
                    Element::Foreign if skips.is_empty() => {
                        if let Some(block) = open.last_mut() {
                            block.foreign_depth += 1;
                        }
                    }
                    _ => {}
                }
            }

            Event::Empty(start) => {
                let qualified = start.name();
                let local = local_name(qualified.as_ref()).to_vec();
                match classify(namespace, &local) {
                    // **The serialization must not move an address.** `<p/>` is what a writer
                    // emits for an empty paragraph, and it occupies its position exactly as
                    // `<p></p>` does — the defect v2-S4 found in a slide's `<a:p/>`, arriving here
                    // as a block ordinal.
                    Element::Block | Element::Preserve => opened = advance(opened, part_name)?,
                    Element::Break => push_stated(&mut open, "\n", &mut skips, text_bytes)?,
                    _ => {}
                }
            }

            Event::End(end) => {
                depth -= 1;
                let qualified = end.name();
                let local = local_name(qualified.as_ref()).to_vec();
                let element = classify(namespace, &local);

                let mut closed_region = false;
                if let Some(skip) = skips.last() {
                    if depth + 1 == skip.from_depth {
                        let skip = skips.pop().expect("checked above");
                        skip_strict.pop();
                        if skip.held_text {
                            regions_not_read = regions_not_read.saturating_add(1);
                        }
                        closed_region = true;
                    }
                }

                match element {
                    Element::Block | Element::Preserve => {
                        if element == Element::Preserve {
                            preserving = preserving.saturating_sub(1);
                        }
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth = skip.block_depth.saturating_sub(1);
                        } else if let Some(block) = open.pop() {
                            let at = meta.pop();
                            if block.foreign_text {
                                foreign_text_not_read = foreign_text_not_read.saturating_add(1);
                            }
                            // A block with no characters is not a node, for the reason a `<w:r>`
                            // with no `<w:t>` is not: nothing was erased, because there was never
                            // a character there.
                            if !block.text.is_empty() {
                                if let Some(at) = at {
                                    blocks.push((
                                        at.ordinal,
                                        Block {
                                            ordinal: at.ordinal,
                                            element: at.element,
                                            text: block.text,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                    Element::Foreign if skips.is_empty() && !closed_region => {
                        if let Some(block) = open.last_mut() {
                            block.foreign_depth = block.foreign_depth.saturating_sub(1);
                        }
                    }
                    _ => {}
                }
            }

            Event::Text(text) => {
                let decoded = decode(&text, part_name)?;
                characters(
                    decoded.as_ref(),
                    &mut open,
                    &mut skips,
                    &skip_strict,
                    preserving,
                    &mut text_outside_a_block,
                    text_bytes,
                )?;
            }
            // Matched, not ignored: an unhandled `CData` arm is a silent drop.
            Event::CData(cdata) => {
                let decoded = cdata_text(&cdata, part_name)?;
                characters(
                    decoded.as_ref(),
                    &mut open,
                    &mut skips,
                    &skip_strict,
                    preserving,
                    &mut text_outside_a_block,
                    text_bytes,
                )?;
            }
            Event::GeneralRef(entity) => {
                let resolved = resolve_reference(entity.as_ref(), part_name)?;
                characters(
                    resolved.as_ref(),
                    &mut open,
                    &mut skips,
                    &skip_strict,
                    preserving,
                    &mut text_outside_a_block,
                    text_bytes,
                )?;
            }
            _ => {}
        }
    }

    check_closed(depth, part_name)?;
    // Document order, which nesting alone does not give: an outer block closes after the inner
    // ones it contains, so the pop order is not the file's order.
    blocks.sort_by_key(|(ordinal, _)| *ordinal);
    Ok(DocumentRead {
        blocks: blocks.into_iter().map(|(_, block)| block).collect(),
        regions_not_read,
        foreign_text_not_read,
        text_outside_a_block,
    })
}

/// Route character data to the block, the region's tally, or the no-address bucket.
///
/// **The third destination is the one that would otherwise be a silent drop.** `odt.rs`'s engine
/// returns without recording anything when no block is open, which is correct for ODF — every
/// `<text:p>` is a block — and wrong here: character data directly inside an `<html>` element, or
/// inside a foreign subtree before any block opens, reaches nothing. v2-S6 found this class in a
/// spreadsheet and v2-S7 found it again in a presentation; it is closed here before it can be.
#[allow(clippy::too_many_arguments)]
fn characters(
    text: &str,
    open: &mut [OpenBlock],
    skips: &mut [Skip],
    strict: &[bool],
    preserving: u32,
    outside: &mut u32,
    text_bytes: &mut usize,
) -> Result<(), EngineError> {
    if let Some(skip) = skips.last_mut() {
        // A strict region counts only characters inside its own blocks; everything else counts
        // bare characters too, because a script's source and a style sheet's rules sit in no block
        // at all. See [`Element::Region`].
        if strict.last().copied().unwrap_or(false) {
            return push_source(open, text, skips, text_bytes);
        }
        if has_characters(text) {
            skip.held_text = true;
        }
        return Ok(());
    }
    if open.is_empty() {
        if has_characters(text) {
            *outside = outside.saturating_add(1);
        }
        return Ok(());
    }
    if preserving > 0 {
        // `white-space: pre`. `push_stated` is the path `odt.rs` uses for characters the file
        // states as an element, and it is exempt from the collapsing rule for the same reason.
        return push_stated(open, text, skips, text_bytes);
    }
    push_source(open, text, skips, text_bytes)
}

/// Whether these characters are anything but the four XHTML collapses.
fn has_characters(text: &str) -> bool {
    text.chars().any(|c| !matches!(c, ' ' | '\t' | '\n' | '\r'))
}

/// Move the block counter forward, refusing rather than pinning at the ceiling.
///
/// **A saturated ordinal is a wrong address, not a large one.** `saturating_add` would give every
/// later block the same `u32::MAX`, so distinct blocks would share one address — the locator
/// `docs/01-CONTRACT.md` §5.2 calls strictly worse than an absent one.
fn advance(ordinal: u32, part_name: &str) -> Result<u32, EngineError> {
    ordinal
        .checked_add(1)
        .ok_or_else(|| EngineError::Malformed {
            what: part_name.into(),
            detail:
                "this document contains more blocks than a block index can hold. Clamping would \
                 give distinct blocks one address, so the document is refused rather than \
                 addressed wrongly."
                    .into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const XHTML_OPEN: &str = concat!(
        r#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml""#,
        r#" xmlns:svg="http://www.w3.org/2000/svg""#,
        r#" xmlns:m="http://www.w3.org/1998/Math/MathML""#,
        r#"><body>"#
    );
    const XHTML_CLOSE: &str = "</body></html>";

    fn read_body(inner: &str) -> DocumentRead {
        let mut bytes = 0usize;
        read_document(
            format!("{XHTML_OPEN}{inner}{XHTML_CLOSE}").as_bytes(),
            "chapter.xhtml",
            &mut bytes,
        )
        .expect("the document reads")
    }

    fn texts(read: &DocumentRead) -> Vec<&str> {
        read.blocks.iter().map(|b| b.text.as_str()).collect()
    }

    // ---------------------------------------------------------------------------------------
    // Blocks and the address
    // ---------------------------------------------------------------------------------------

    #[test]
    fn blocks_bind_at_the_position_the_document_states() {
        let read = read_body("<h1>Title</h1><p>First</p><p>Second</p>");
        assert_eq!(texts(&read), vec!["Title", "First", "Second"]);
        assert_eq!(
            read.blocks.iter().map(|b| b.ordinal).collect::<Vec<_>>(),
            vec![2, 3, 4],
            "`<body>` is a block too, so it takes position 1 and mints no node"
        );
        assert_eq!(
            read.blocks
                .iter()
                .map(|b| b.element.as_str())
                .collect::<Vec<_>>(),
            vec!["h1", "p", "p"],
            "the element name is the file's own word for it"
        );
    }

    /// **The counter advances through a region this reader does not read.**
    #[test]
    fn a_block_inside_a_skipped_region_still_moves_the_counter() {
        let read = read_body("<p>Before</p><nav><ol><li>TOC</li></ol></nav><p>After</p>");
        assert_eq!(texts(&read), vec!["Before", "After"]);
        assert_eq!(
            read.blocks[1].ordinal, 5,
            "`<body>` is 1 and `Before` is 2; the passed-over `<ol>` and `<li>` are blocks the \
             document contains and take 3 and 4, so `After` is 5. The `<nav>` itself is a region \
             rather than a block and takes no position."
        );
        assert_eq!(read.regions_not_read, 1);
    }

    /// **The serialization does not move an address.** `<p/>` is an empty paragraph.
    #[test]
    fn a_self_closing_block_still_holds_its_position() {
        let short = read_body("<p/><p>Second</p>");
        let long = read_body("<p></p><p>Second</p>");
        assert_eq!(short.blocks, long.blocks);
        assert_eq!(short.blocks[0].ordinal, 3);
    }

    /// Blocks emitted in **document order**, which the pop order is not.
    #[test]
    fn nested_blocks_emit_in_document_order() {
        let read = read_body("<div>Outer text<p>Inner text</p></div>");
        assert_eq!(texts(&read), vec!["Outer text", "Inner text"]);
    }

    #[test]
    fn a_table_cell_is_its_own_block_and_is_not_a_table_record() {
        let read = read_body("<table><tr><td>Left</td><td>Right</td></tr></table>");
        assert_eq!(texts(&read), vec!["Left", "Right"]);
        assert_eq!(
            read.blocks
                .iter()
                .map(|b| b.element.as_str())
                .collect::<Vec<_>>(),
            vec!["td", "td"]
        );
    }

    // ---------------------------------------------------------------------------------------
    // The XHTML rule
    // ---------------------------------------------------------------------------------------

    /// **An element HTML gives no block display is inline, which is HTML's own default.**
    #[test]
    fn an_unknown_xhtml_element_is_inline_because_html_says_so() {
        let read = read_body("<p>The <my-widget>custom</my-widget> part.</p>");
        assert_eq!(
            texts(&read),
            vec!["The custom part."],
            "HTML renders an element nobody has heard of inline, so its characters are the \
             sentence's"
        );
        assert_eq!(read.regions_not_read, 0);
        assert_eq!(read.foreign_text_not_read, 0);
    }

    /// **A different namespace is foreign, and that is what keeps an SVG title out.**
    #[test]
    fn a_foreign_namespace_is_counted_rather_than_spliced() {
        let read = read_body("<p>Figure:<svg:svg><svg:title>SVG-TITLE</svg:title></svg:svg></p>");
        assert_eq!(texts(&read), vec!["Figure:"]);
        assert_eq!(read.foreign_text_not_read, 1);
    }

    #[test]
    fn mathml_annotation_is_foreign_too() {
        let read = read_body("<p>Formula:<m:math><m:annotation>SOURCE</m:annotation></m:math></p>");
        assert_eq!(texts(&read), vec!["Formula:"]);
        assert_eq!(read.foreign_text_not_read, 1);
    }

    /// **A `<title>` inside an inline SVG is not an XHTML title**, so the namespace decides.
    #[test]
    fn a_foreign_title_is_not_the_documents_title_region() {
        let read = read_body("<p>a<svg:svg><svg:title>T</svg:title></svg:svg>b</p>");
        assert_eq!(
            texts(&read),
            vec!["ab"],
            "the foreign subtree is skipped and the block's own characters continue"
        );
    }

    #[test]
    fn a_script_and_a_style_are_regions() {
        let read =
            read_body("<p>Kept</p><script>var x = 1;</script><style>.c { color: red }</style>");
        assert_eq!(texts(&read), vec!["Kept"]);
        assert_eq!(read.regions_not_read, 2);
    }

    /// **A `<head>` counts only characters inside its own blocks**, because every document has a
    /// `<title>` and counting it would declare an erasure on every document.
    #[test]
    fn a_head_with_only_a_title_declares_no_erasure() {
        let mut bytes = 0usize;
        let read = read_document(
            br#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>T</title></head><body><p>Body</p></body></html>"#,
            "chapter.xhtml",
            &mut bytes,
        )
        .expect("reads");
        assert_eq!(texts(&read), vec!["Body"]);
        assert_eq!(read.regions_not_read, 0);
    }

    /// **The strict rule is for the document's own head, and nowhere else.**
    ///
    /// `<head>` and `<title>` count only characters inside their own blocks, so the `<title>` every
    /// XHTML document carries is not an erasure on every document. That argument holds only in the
    /// prologue. A `<title>` in the **body** is not metadata — HTML renders it as nothing — so it
    /// must not be spliced into a block, and it must not vanish uncounted either.
    #[test]
    fn a_title_in_the_body_is_counted_rather_than_lost() {
        let read = read_body("<p>Kept</p><title>STRAY-TITLE</title>");
        assert_eq!(texts(&read), vec!["Kept"], "it is not spliced into a block");
        assert_eq!(
            read.regions_not_read, 1,
            "and it is not lost either: a block is open, so this is not the document's head"
        );
    }

    #[test]
    fn ruby_guide_text_is_a_region_and_the_base_is_read() {
        let read = read_body("<p><ruby>kanji<rt>GUIDE</rt></ruby></p>");
        assert_eq!(texts(&read), vec!["kanji"]);
        assert_eq!(read.regions_not_read, 1);
    }

    /// Character data that reaches no block is **declared**, never dropped.
    #[test]
    fn text_outside_every_block_is_declared() {
        let mut bytes = 0usize;
        let read = read_document(
            br#"<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml">LOOSE-TEXT</html>"#,
            "chapter.xhtml",
            &mut bytes,
        )
        .expect("reads");
        assert!(read.blocks.is_empty());
        assert_eq!(read.text_outside_a_block, 1);
    }

    /// A document with no XHTML namespace mints no blocks, and says so rather than looking empty.
    #[test]
    fn a_document_with_no_xhtml_namespace_declares_what_it_could_not_place() {
        let mut bytes = 0usize;
        let read = read_document(
            b"<?xml version=\"1.0\"?><html><body><p>NOT-XHTML</p></body></html>",
            "chapter.xhtml",
            &mut bytes,
        )
        .expect("reads");
        assert!(
            read.blocks.is_empty(),
            "no element is in the XHTML namespace"
        );
        assert_eq!(read.text_outside_a_block, 1);
    }

    // ---------------------------------------------------------------------------------------
    // Characters
    // ---------------------------------------------------------------------------------------

    #[test]
    fn whitespace_collapses_the_way_xhtml_states() {
        let read = read_body("<p>  one   two\n\n  three  </p>");
        assert_eq!(texts(&read), vec!["one two three"]);
    }

    /// **`<pre>` is the one divergence**, because `white-space: pre` is the document's own claim.
    #[test]
    fn a_pre_block_keeps_the_whitespace_the_document_wrote() {
        let read = read_body("<pre>  two spaces\n  and a line</pre>");
        assert_eq!(texts(&read), vec!["  two spaces\n  and a line"]);
    }

    #[test]
    fn a_break_is_the_line_feed_the_element_states() {
        let read = read_body("<p>one<br/>two</p>");
        assert_eq!(texts(&read), vec!["one\ntwo"]);
    }

    /// **A numeric character reference resolves**, which the shared entity rule refuses.
    #[test]
    fn numeric_character_references_resolve() {
        let read = read_body("<p>caf&#233; &#x2014; &amp; done</p>");
        assert_eq!(texts(&read), vec!["café — & done"]);
    }

    #[test]
    fn an_html_named_entity_is_still_a_named_refusal() {
        let mut bytes = 0usize;
        let error = read_document(
            format!("{XHTML_OPEN}<p>a&nbsp;b</p>{XHTML_CLOSE}").as_bytes(),
            "chapter.xhtml",
            &mut bytes,
        )
        .expect_err("refused");
        assert!(error.to_string().contains("nbsp"), "{error}");
    }

    #[test]
    fn a_reference_that_names_no_scalar_is_refused() {
        let mut bytes = 0usize;
        // The last two are the shape rather than the value: XML writes no sign in a character
        // reference, and Rust's integer parsers accept one — so `&#+66;` would have resolved to
        // `B` and been spliced in as though the document had written it.
        for bad in ["&#xD800;", "&#1114112;", "&#+66;", "&#x+44;"] {
            assert!(
                read_document(
                    format!("{XHTML_OPEN}<p>{bad}</p>{XHTML_CLOSE}").as_bytes(),
                    "chapter.xhtml",
                    &mut bytes,
                )
                .is_err(),
                "{bad} names no Unicode scalar"
            );
        }
    }

    #[test]
    fn a_truncated_document_is_refused() {
        let mut bytes = 0usize;
        assert!(read_document(
            format!("{XHTML_OPEN}<p>open forever").as_bytes(),
            "chapter.xhtml",
            &mut bytes
        )
        .is_err());
    }

    // ---------------------------------------------------------------------------------------
    // Reference resolution
    // ---------------------------------------------------------------------------------------

    #[test]
    fn an_href_resolves_against_the_package_documents_directory() {
        assert_eq!(
            normalize("chap01.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/chap01.xhtml"
        );
        assert_eq!(
            normalize("text/chap01.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/text/chap01.xhtml"
        );
        assert_eq!(
            normalize("../shared/a.xhtml", "OEBPS/text", "opf").unwrap(),
            "OEBPS/shared/a.xhtml"
        );
        assert_eq!(normalize("a.xhtml", "", "opf").unwrap(), "a.xhtml");
        assert_eq!(
            normalize("./a.xhtml#frag", "OEBPS", "opf").unwrap(),
            "OEBPS/a.xhtml",
            "a fragment addresses a position inside a document, not the document"
        );
    }

    #[test]
    fn a_percent_escape_is_decoded_per_segment() {
        assert_eq!(
            normalize("chap%2001.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/chap 01.xhtml"
        );
        assert_eq!(
            normalize("caf%C3%A9.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/café.xhtml"
        );
        assert_eq!(
            normalize("a%2Fb.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/a/b.xhtml",
            "decoded inside the segment, so it can never invent a boundary the reference did not \
             have — and simply matches no archive entry"
        );
        assert_eq!(
            normalize("100%.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/100%.xhtml",
            "a `%` that introduces no hex pair is a literal `%`"
        );
    }

    /// **A reference may hold any UTF-8, and decoding it must not panic.**
    ///
    /// The first version indexed the segment as a `&str` by byte offset, so a `%` followed by a
    /// multi-byte scalar sliced inside it and crashed. A reader whose whole contract is a named
    /// refusal must not have an input that takes the process down with it.
    #[test]
    fn a_percent_before_a_multi_byte_character_does_not_panic() {
        assert_eq!(
            normalize("a%\u{20ac}b.xhtml", "OEBPS", "opf").unwrap(),
            "OEBPS/a%\u{20ac}b.xhtml"
        );
        assert_eq!(
            normalize("%\u{20ac}", "OEBPS", "opf").unwrap(),
            "OEBPS/%\u{20ac}"
        );
        assert_eq!(normalize("a%", "OEBPS", "opf").unwrap(), "OEBPS/a%");
        assert_eq!(normalize("a%4", "OEBPS", "opf").unwrap(), "OEBPS/a%4");
    }

    /// A sequence that decodes to bytes that are not UTF-8 is refused rather than replaced.
    #[test]
    fn an_undecodable_escape_is_refused() {
        assert!(normalize("a%FF%FEb.xhtml", "OEBPS", "opf").is_err());
    }

    #[test]
    fn a_reference_outside_the_container_is_refused() {
        for bad in [
            "../../etc/passwd",
            "/absolute.xhtml",
            "https://example.invalid/a.xhtml",
            "",
        ] {
            assert!(
                normalize(bad, "OEBPS", "opf").is_err(),
                "`{bad}` does not name an entry inside this container"
            );
        }
    }

    // ---------------------------------------------------------------------------------------
    // The container chain
    // ---------------------------------------------------------------------------------------

    fn container(inner: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles>{inner}</rootfiles></container>"#
        )
    }

    #[test]
    fn the_first_rootfile_is_the_default_rendition() {
        let read = read_container(
            container(
                r#"<rootfile full-path="a/one.opf" media-type="application/oebps-package+xml"/>
                   <rootfile full-path="b/two.opf" media-type="application/oebps-package+xml"/>"#,
            )
            .as_bytes(),
        )
        .expect("reads");
        assert_eq!(read, vec!["a/one.opf".to_string(), "b/two.opf".to_string()]);
    }

    #[test]
    fn a_rootfile_that_is_not_a_package_document_is_not_one() {
        let read = read_container(
            container(r#"<rootfile full-path="x.xml" media-type="application/something-else"/>"#)
                .as_bytes(),
        )
        .expect("reads");
        assert!(read.is_empty());
    }

    #[test]
    fn a_rootfile_with_no_full_path_is_refused() {
        assert!(read_container(
            container(r#"<rootfile media-type="application/oebps-package+xml"/>"#).as_bytes()
        )
        .is_err());
    }

    /// A `rootfile` in another vocabulary is not this container's.
    #[test]
    fn a_foreign_rootfile_is_not_a_rootfile() {
        let read = read_container(
            br#"<?xml version="1.0"?><c:container xmlns:c="urn:example:other"><c:rootfiles><c:rootfile full-path="x.opf" media-type="application/oebps-package+xml"/></c:rootfiles></c:container>"#,
        )
        .expect("reads");
        assert!(read.is_empty());
    }

    // ---------------------------------------------------------------------------------------
    // The package document
    // ---------------------------------------------------------------------------------------

    fn package(manifest: &str, spine: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><manifest>{manifest}</manifest><spine>{spine}</spine></package>"#
        )
    }

    fn read_spine(manifest: &str, spine: &str) -> Vec<SpineItem> {
        read_package(package(manifest, spine).as_bytes(), "OEBPS/content.opf")
            .expect("the package reads")
    }

    /// **The spine is the reading order, and the manifest's own order is not.**
    #[test]
    fn the_spine_states_the_order_and_the_manifest_does_not() {
        let items = read_spine(
            r#"<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/>
               <item id="b" href="b.xhtml" media-type="application/xhtml+xml"/>"#,
            r#"<itemref idref="b"/><itemref idref="a"/>"#,
        );
        assert_eq!(
            items.iter().map(|i| i.part.as_str()).collect::<Vec<_>>(),
            vec!["OEBPS/b.xhtml", "OEBPS/a.xhtml"]
        );
    }

    #[test]
    fn linear_defaults_to_yes_and_is_read_when_stated() {
        let items = read_spine(
            r#"<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/>
               <item id="b" href="b.xhtml" media-type="application/xhtml+xml"/>"#,
            r#"<itemref idref="a"/><itemref idref="b" linear="no"/>"#,
        );
        assert!(items[0].linear);
        assert!(!items[1].linear);
    }

    #[test]
    fn a_spine_item_of_another_media_type_is_not_read() {
        let items = read_spine(
            r#"<item id="a" href="a.svg" media-type="image/svg+xml"/>"#,
            r#"<itemref idref="a"/>"#,
        );
        assert!(!items[0].readable);
    }

    #[test]
    fn an_itemref_with_no_manifest_item_is_refused() {
        let error = read_package(
            package(
                r#"<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/>"#,
                r#"<itemref idref="missing"/>"#,
            )
            .as_bytes(),
            "OEBPS/content.opf",
        )
        .expect_err("refused");
        assert!(error.to_string().contains("missing"), "{error}");
    }

    #[test]
    fn two_manifest_items_of_one_id_are_refused() {
        assert!(read_package(
            package(
                r#"<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/>
                   <item id="a" href="b.xhtml" media-type="application/xhtml+xml"/>"#,
                r#"<itemref idref="a"/>"#,
            )
            .as_bytes(),
            "OEBPS/content.opf",
        )
        .is_err());
    }

    /// One entry reached twice would give two positions one part name.
    #[test]
    fn a_spine_that_reaches_one_entry_twice_is_refused() {
        assert!(read_package(
            package(
                r#"<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/>
                   <item id="b" href="a.xhtml" media-type="application/xhtml+xml"/>"#,
                r#"<itemref idref="a"/><itemref idref="b"/>"#,
            )
            .as_bytes(),
            "OEBPS/content.opf",
        )
        .is_err());
    }

    #[test]
    fn a_package_with_an_empty_spine_is_refused() {
        assert!(read_package(
            package(
                r#"<item id="a" href="a.xhtml" media-type="application/xhtml+xml"/>"#,
                "",
            )
            .as_bytes(),
            "OEBPS/content.opf",
        )
        .is_err());
    }

    #[test]
    fn a_manifest_item_with_no_id_or_no_href_is_refused() {
        for manifest in [
            r#"<item href="a.xhtml" media-type="application/xhtml+xml"/>"#,
            r#"<item id="a" media-type="application/xhtml+xml"/>"#,
        ] {
            assert!(read_package(
                package(manifest, r#"<itemref idref="a"/>"#).as_bytes(),
                "OEBPS/content.opf",
            )
            .is_err());
        }
    }

    /// A package in another vocabulary states nothing this reader may act on.
    #[test]
    fn a_foreign_package_vocabulary_is_not_a_package() {
        assert!(read_package(
            br#"<?xml version="1.0"?><package xmlns="urn:example:other"><manifest><item id="a" href="a.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="a"/></spine></package>"#,
            "OEBPS/content.opf",
        )
        .is_err());
    }

    // ---------------------------------------------------------------------------------------
    // Detection
    // ---------------------------------------------------------------------------------------

    /// **Detection is exact, and this asks `is_epub` rather than comparing two literals.**
    ///
    /// The first version of this test looped over near-miss strings asserting each was `!=` the
    /// constant — a tautology the compiler could fold, which would have passed against a reader
    /// that claimed every one of them. It builds a package for each instead.
    #[test]
    fn the_declared_type_is_matched_exactly() {
        for (declared, claimed) in [
            ("application/epub+zip", true),
            // Trimmed, because `declared_media_type` trims: a writer that appended a newline
            // declared the same type. Asserted rather than assumed — that trim is v2-S5's.
            ("application/epub+zip\n", true),
            ("application/epub", false),
            ("application/epub+zip-template", false),
            ("application/vnd.oasis.opendocument.text", false),
            ("application/zip", false),
            ("", false),
        ] {
            let package = ocf(&[("mimetype", declared)]);
            assert_eq!(is_epub(&package), claimed, "`{declared}`");
        }

        // And the entry has to be FIRST and STORED, which is what makes it an OCF container.
        assert!(
            !is_epub(&ocf(&[
                ("META-INF/container.xml", "<container/>"),
                ("mimetype", "application/epub+zip"),
            ])),
            "a `mimetype` that is not the first entry is not an OCF declaration"
        );

        assert!(!is_epub(b"not a zip"));
        assert!(!is_epub(b""));
        assert!(!is_epub(b"PK\x03\x04"), "a truncated ZIP declares nothing");
    }

    /// A ZIP whose entries are stored, in the order given — the shape OCF requires of `mimetype`.
    fn ocf(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, body) in entries {
            let offset = out.len() as u32;
            let data = body.as_bytes();
            let crc = crc32(data);
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);

            directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            directory.extend_from_slice(&[10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            directory.extend_from_slice(&crc.to_le_bytes());
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
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&directory_size.to_le_bytes());
        out.extend_from_slice(&directory_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in data {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }
}
