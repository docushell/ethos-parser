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

//! `content.xml` → paragraphs, at the positions the part states (v2-S5).
//!
//! # The first v2 format that is not OOXML
//!
//! Three things are shared with the three readers before this one and nothing else is: the ZIP
//! container (`zip.rs`), the XML plumbing and the entity rule (`xml.rs`), and the discipline. The
//! vocabulary is different — `text:p`, not `w:p` — the part naming is different, and the
//! `r:id`-to-part indirection `opc.rs` exists for is **absent**: the OpenDocument package
//! specification fixes the content part's name as `content.xml`, so there is no relationship to
//! resolve and no shortcut to be caught taking.
//!
//! What replaces it is the **manifest**. `META-INF/manifest.xml` is the package's own list of what
//! it contains, and this reader consults it rather than reaching straight for a part name: a
//! package that does not declare `content.xml` is refused, and one that declares it **encrypted**
//! is refused by name rather than being handed to a parser that would report the ciphertext as
//! malformed XML.
//!
//! # The page this format actually contains, and why it is still not a page
//!
//! An ODT is the sharpest test the no-synthesised-pages law has had. A DOCX has no page at all, a
//! workbook's is a printer's, and a slide is a part — but `content.xml` contains
//! `<text:soft-page-break/>`, an element whose entire meaning is *"the producing application's
//! layout broke the page here"*, and `styles.xml` contains a `style:master-page` with an
//! `fo:page-width`. Between them they would furnish a page index and a page size with no
//! arithmetic at all.
//!
//! They are **not read**, and the reason is `docs/06-STEAL-REFUSE.md` L30's own: *"It invents
//! pagination."* A soft page break is a record of somebody else's rendering — it moves when the
//! font stack, the paper size or the producing application changes, and a document saved by two
//! editors carries two different sets. Reading it would put a page number in the evidence that is
//! a measurement of a word processor, which is exactly the citation `docs/history/14-V2-SCOPE.md` §3
//! exists to refuse. `pages` is `[]`, and [`ethos_parser_core::OdtLocator`] has no room for one.
//!
//! # What a paragraph's text is, and what it is not
//!
//! The atom is the `<text:p>` or `<text:h>` block; see [`ethos_parser_core::OdtLocator`] for why it is
//! that rather than `<text:span>`.
//!
//! **A block's text is an allowlist, not a descendant walk with exceptions**, and that inversion
//! is the correction this reader's own review forced. Character data reaches a block only when
//! every element between the block and the text is [`Element::InlineText`] — a span, a hyperlink,
//! a ruby *base* — because those are the same sentence. Everything else is [`Element::Foreign`]:
//! its characters are not the block's, and they are **counted** rather than spliced or dropped.
//!
//! The first version named four regions to skip and appended everything else, which is a
//! descendant walk with an exception list. ODF puts a great deal of non-displayed character data
//! inside a `<text:p>`, and a text-anchored `<draw:frame>` is a **child of the paragraph it is
//! anchored in** — the ordinary shape, not an edge case. So an image's `<svg:title>` and
//! `<svg:desc>`, an embedded object's base64 in `<office:binary-data>`, a `<text:number>`'s
//! generated heading label and every field's cached value — `<text:page-number>`,
//! `<text:page-count>`, `<text:chapter>` — all landed in the sentence. The last of those is the
//! sharpest: this reader refuses `<text:soft-page-break/>` by name and would then have put the
//! producer's page arithmetic into `Node.text` anyway.
//!
//! Nesting is the other half, and it is load-bearing rather than tidy: ODF puts a footnote's body,
//! a comment's body and a text box's contents *inside* the block they are anchored to, as their own
//! `<text:p>` elements, so a reader that concatenated every descendant would splice a footnote into
//! the middle of the sentence that cites it. The block stack is what keeps them apart — and the
//! block a region is anchored to must **not** be closed by the region's own `</text:p>`, which is
//! how the first fix for that splice lost the rest of the sentence instead.
//!
//! Three elements become characters, because that is what the file says they are and XML would
//! otherwise collapse them: `<text:s text:c="n">` is **n** spaces — the count the file states,
//! never a count inferred from where anything sits on a page — `<text:tab/>` is a tab, and
//! `<text:line-break/>` is a line feed. Dropping them would silently join `Name` and `Value` into
//! `NameValue`, which is wrong text at a right address.
//!
//! # ODF's own whitespace rule is applied, and that is reading rather than rendering
//!
//! The three elements above exist because OpenDocument **defines** what the character data around
//! them means: a tab, a line feed or a carriage return in the source counts as a space, a run of
//! spaces counts as one, and the spaces at the two ends of a block are not part of it. That rule
//! is in the format, not in a layout engine — it is why a producer that wants three spaces has to
//! write `<text:s text:c="3">` instead of typing three — so following it is the same kind of act
//! as resolving a shared string by index in a workbook.
//!
//! Not following it would make the text depend on how the file was **serialized**: a producer that
//! indented inside a paragraph would hand back a sentence with a newline in the middle of it, and
//! two files stating the same document would state two different phrases. That is the failure
//! v2-S4 found in an *address* — one that turned on whether a paragraph was written `<a:p/>` or
//! `<a:p></a:p>` — arriving one slice later in the text. The characters the three elements state
//! are exempt, because their whole purpose is to survive it.
//!
//! # What is passed over, and counted (**A14**)
//!
//! Four regions of `content.xml` hold text this slice does not read, and each is a **declared**
//! erasure rather than a silent skip:
//!
//! - `<text:note>` — a footnote or endnote body. Not read, for the reason `word/footnotes.xml` is
//!   not read in a DOCX: it is a second stream of text, and this slice reads the body.
//! - `<office:annotation>` — a comment. Not the document's text, and reading it would put a
//!   reviewer's remark in the record as though the document said it.
//! - `<text:tracked-changes>` — the record of what a revision **deleted**. Reading it would put
//!   text the document no longer states into the evidence, which is the second-authority problem
//!   `docs/history/14-V2-SCOPE.md` §7 refuses in another form.
//! - A second or later `<draw:text-box>` inside one `<draw:frame>`. ODF frames hold *alternative*
//!   renditions of one object, of which a consumer uses the first it can process; reading all of
//!   them would emit one displayed phrase at two citable addresses, which is the mirror of a
//!   silent drop and is the same defect v2-S4 found in `<mc:AlternateContent>`.
//!
//! **And the paragraph counter advances through every one of them.** The locator promises a
//! position in the part's own document order, so a consumer checking it counts `<text:p>` and
//! `<text:h>` elements in `content.xml` — not the ones this reader chose to keep. Counting only
//! what is read would leave every address after a footnote one short, which is a locator that is
//! confidently wrong and is the defect v2-S4's review caught three times.
//!
//! # What this reader was and was not measured against
//!
//! Stated plainly, because v2-S3 and v2-S4 both changed a design after measuring real files and
//! this slice could not. **No corpus of real `.odt` files was available on the machine this slice
//! was written on, and no ODF producer was either** — so unlike the 18 decks behind `pptx.rs`,
//! every rule here is read off the OpenDocument specification and pinned against fixtures this
//! repository authored. Where that leaves a judgement call, it is made toward **over-declaring**:
//! a region this reader is unsure about is counted as unread rather than concatenated into a
//! paragraph, so the failure mode is a phrase declared missing rather than a phrase invented.

use ethos_parser_core::EngineError;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;

use crate::xml::{
    attribute_value, cdata_text, check_closed, decode, local_name, new_ns_reader, new_reader,
    parse_error, parse_error_at, resolve_reference, resolved_attribute,
};

/// The three OpenDocument namespaces this reader resolves element names in.
///
/// # Why this reader resolves namespaces when the three OOXML readers do not
///
/// `xml.rs`'s [`local_name`] matches by suffix and states the tradeoff for doing so: *"the failure
/// mode is a refusal to find content rather than wrong content."* That argument is sound for the
/// OOXML readers, where a suffix match only ever selects **content**. It does not survive here,
/// because in this reader the same match feeds an **address** and a **skip decision**:
///
/// - `ordinal` advances for any element whose local name is `p` or `h`. ODF §3.17 permits foreign
///   elements in mixed content, so a conforming `<xhtml:p>` would shift every later paragraph's
///   number — a locator that is confidently wrong, which `docs/01-CONTRACT.md` §5.2 calls strictly
///   worse than an absent one.
/// - the comment-skip triggers on local name `annotation`. MathML's `<annotation>` — the
///   `<semantics>` child carrying a formula's source form, legal inline in a `<draw:object>` — has
///   the same local name, so an inline formula would be declared to the caller as an unread
///   *reviewer's remark*, naming a gap the document does not have.
///
/// Both are conforming-document cases rather than malformed ones, so the tradeoff is re-argued
/// here rather than inherited: element names are resolved, and a name in an unexpected namespace
/// is [`Element::Foreign`].
pub(crate) const NS_TEXT: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:text:1.0";
pub(crate) const NS_OFFICE: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:office:1.0";
pub(crate) const NS_DRAW: &[u8] = b"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0";

/// What one element means to this reader.
///
/// # An allowlist, not an exclusion list — and that inversion is the point
///
/// The first version of this reader named four regions to skip and appended the character data of
/// **everything else** to the innermost open block. That is a descendant walk with an exception
/// list, and ODF puts a great deal of non-displayed character data inside a `<text:p>`: an image's
/// `<svg:title>` and `<svg:desc>` (written by LibreOffice whenever the user fills them in), an
/// embedded object's base64 in `<office:binary-data>`, a `<text:number>`'s generated heading label,
/// and every field's cached value — `<text:page-number>`, `<text:page-count>`, `<text:chapter>`,
/// `<text:date>`. A text-anchored `<draw:frame>` is a **child of the paragraph it is anchored in**,
/// which is the ordinary ODF shape rather than an edge case, so all of it landed in the sentence.
///
/// So the rule is inverted: character data reaches a block only when **every** element between the
/// block and the text is [`Self::InlineText`]. Anything else is [`Self::Foreign`] — its characters
/// are not the block's, and they are **counted** (**A14**) rather than spliced or dropped. That is
/// what the module header always claimed ("the character data at *its own level*") and what
/// [`crate::xml`]'s three OOXML readers get for free by gating on `<w:t>`, `<t>` and `<a:t>`.
///
/// **Three siblings, not seven, and the distinction is the point.** This reader has seven siblings
/// now — `docx`, `xlsx`, `pptx`, `ods`, `odp`, `rtf` and `epub` — and they do not divide evenly.
/// The three OOXML readers match element names by suffix; `ods`, `odp` and `epub` resolve
/// namespaces exactly as this one does, because ODF and XHTML pose the same problem; and `rtf`
/// parses no XML at all. The heading said "its three siblings" when ODT was the fourth format and
/// three was the whole set, and it kept saying it through four more formats until v2-S13.3.
/// ODF has no such element, so the gate has to be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Element {
    /// `text:p` / `text:h` — a block of text, and the unit [`ethos_parser_core::OdtLocator`] addresses.
    Block { heading: bool },
    /// `text:s` — the number of spaces the file states.
    Space,
    /// `text:tab` — a tab.
    Tab,
    /// `text:line-break` — a line feed.
    LineBreak,
    /// `text:soft-page-break` — the producing application's own layout, read and discarded.
    SoftPageBreak,
    /// A region whose text is not the document's body: a note, a comment, a tracked change.
    Region,
    /// `draw:frame` — a container whose children are alternative renditions of one object.
    Frame,
    /// `draw:text-box` — one such rendition.
    TextBox,
    /// An inline element whose characters **are** the enclosing block's text.
    InlineText,
    /// Everything else. Its characters are not the block's text.
    Foreign,
}

/// Classify a resolved element name.
///
/// The `InlineText` arm is deliberately short and deliberately closed. A wrapper this list does not
/// name has its characters **declared** rather than spliced, which is the over-declaring direction
/// this slice commits to everywhere else: the failure mode is a phrase reported missing, never a
/// phrase the document does not contain.
pub(crate) fn classify(namespace: Option<&[u8]>, local: &[u8]) -> Element {
    match (namespace, local) {
        (Some(NS_TEXT), b"p") => Element::Block { heading: false },
        (Some(NS_TEXT), b"h") => Element::Block { heading: true },
        (Some(NS_TEXT), b"s") => Element::Space,
        (Some(NS_TEXT), b"tab") => Element::Tab,
        (Some(NS_TEXT), b"line-break") => Element::LineBreak,
        (Some(NS_TEXT), b"soft-page-break") => Element::SoftPageBreak,
        (Some(NS_TEXT), b"note" | b"tracked-changes") => Element::Region,
        (Some(NS_OFFICE), b"annotation") => Element::Region,
        (Some(NS_DRAW), b"frame") => Element::Frame,
        (Some(NS_DRAW), b"text-box") => Element::TextBox,
        // Inline formatting and linking: their characters are the sentence's own.
        //
        // `ruby` and `ruby-base` are here and `ruby-text` is **not**, which is the ODF twin of the
        // `<rPh>` exclusion v2-S3 made on both of its string paths: furigana attached to a word is
        // a pronunciation guide, not part of the word. `ruby` is the transparent wrapper holding
        // the two, so it has to be transparent here as well or the base is suppressed with the
        // guide — which is what the test below caught.
        (Some(NS_TEXT), b"span" | b"a" | b"ruby" | b"ruby-base" | b"meta" | b"meta-field") => {
            Element::InlineText
        }
        _ => Element::Foreign,
    }
}

/// The namespace a resolved event reports, as bytes.
pub(crate) fn namespace_of<'a>(resolved: &'a ResolveResult<'a>) -> Option<&'a [u8]> {
    match resolved {
        ResolveResult::Bound(ns) => Some(ns.as_ref()),
        _ => None,
    }
}

/// The entry every ODF package states its document type in, first and uncompressed.
pub const MIMETYPE_ENTRY: &str = "mimetype";

/// The package's own list of what it contains.
pub const MANIFEST_PART: &str = "META-INF/manifest.xml";

/// The part an OpenDocument package keeps its document content in.
///
/// Fixed by the OpenDocument package specification rather than named by a relationship, which is
/// the one way this format is *simpler* than OOXML. It is still checked against the manifest
/// before it is read.
pub const CONTENT_PART: &str = "content.xml";

/// The media type an OpenDocument **text** document declares.
pub const ODT_MEDIA_TYPE: &str = "application/vnd.oasis.opendocument.text";

/// Entries that are packaging rather than content, and are therefore not an unread erasure.
const PACKAGING_ENTRIES: [&str; 3] = [MIMETYPE_ENTRY, MANIFEST_PART, CONTENT_PART];

/// A ceiling on how deeply `text:p` / `text:h` may nest before this reader refuses the part.
///
/// Real nesting is a handful — a paragraph anchoring a frame holding a text box holding a
/// paragraph — and `zip.rs` bounds the part at 256 MiB, which is 29 million `<text:p>` opens. That
/// is a stack of `OpenBlock`s, not a parse depth, so it is bounded here rather than left to grow.
pub(crate) const MAX_BLOCK_NESTING: usize = 256;

/// A ceiling on the characters one `content.xml` may expand to, across every block.
///
/// **On the total, because that is where the amplification is.** `zip.rs` caps the *source* at 256
/// MiB and `<text:s text:c="4096"/>` turns 23 of those bytes into 4096 — a 178× ratio with no
/// aggregate bound, so a conforming part inside the ZIP cap could expand past any host's memory.
/// `zip.rs` states the discipline this keeps: a bomb is "a named refusal rather than an
/// out-of-memory kill".
pub(crate) const MAX_TEXT_BYTES: usize = 64 * 1024 * 1024;

/// A ceiling on one `<text:s text:c="…">`, refused by name rather than allocated.
const MAX_SPACES_PER_ELEMENT: usize = 4096;

/// One `<text:p>` or `<text:h>`, with the address the part states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    /// 1-based position of the block in the part's own document order.
    pub ordinal: u32,
    /// Whether the block was a `<text:h>` rather than a `<text:p>`.
    pub heading: bool,
    /// The `text:outline-level` the `<text:h>` stated, or `None` where it stated none.
    ///
    /// Always `None` on a `<text:p>`, which ODF gives the attribute no meaning on, and `None`
    /// rather than `1` on a `<text:h>` that omits it — see [`outline_level`] for why the missing
    /// case is not defaulted.
    pub outline_level: Option<u32>,
    /// The text at this block's own level, under ODF's own whitespace rule, with `<text:s>`,
    /// `<text:tab>` and `<text:line-break>` resolved to the characters they state — and
    /// **without** the text of any block nested inside it.
    pub text: String,
}

/// A block being read, plus the state ODF's whitespace rule and the allowlist need.
pub(crate) struct OpenBlock {
    pub(crate) ordinal: u32,
    pub(crate) heading: bool,
    /// What `<text:h text:outline-level="…">` stated, read once when the block opened.
    ///
    /// Read at open rather than carried from the element later, because the attribute is on the
    /// start tag and nothing after it restates the level.
    pub(crate) outline_level: Option<u32>,
    pub(crate) text: String,
    /// Whether collapsible whitespace has been seen since the last character was appended.
    ///
    /// **Deferred rather than appended**, which is what makes the rule's two halves one flag: a
    /// space is only written once a character follows it, so a run of them collapses to one and a
    /// run at the end of the block is never written at all.
    pub(crate) pending_space: bool,
    /// How many [`Element::Foreign`] elements are open between this block and the reader's cursor.
    ///
    /// Non-zero means the characters arriving now are inside something whose text is not this
    /// block's — an image's description, an object's base64, a field's cached value. They are
    /// counted rather than appended. See [`Element`].
    pub(crate) foreign_depth: u32,
    /// Whether any foreign subtree of this block held characters, so the count is per block
    /// rather than per element and a frame with a title *and* a description declares once.
    pub(crate) foreign_text: bool,
}

/// A region being passed over: where it began, and what it turned out to hold.
pub(crate) struct Skip {
    pub(crate) from_depth: i32,
    /// Whether this region held body characters — see [`Content::regions_not_read`].
    pub(crate) held_text: bool,
    /// How many `text:p` / `text:h` blocks are open inside this region.
    ///
    /// The reason the count is not simply "did any character appear": **every** ODF note carries a
    /// `<text:note-citation>` with its mark, and every comment a real producer writes carries
    /// `<dc:creator>` and `<dc:date>`. Those are metadata, not erased body text, so counting them
    /// would make the guard below always true and declare an erasure for a comment that has no
    /// body at all. A region's erasure is its **blocks**.
    pub(crate) block_depth: u32,
}

/// What one `content.xml` yielded, plus what it passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// The blocks that carry text, in the part's own document order.
    pub paragraphs: Vec<Paragraph>,
    /// Regions of the part that held body text and were not read: a footnote or endnote body, a
    /// comment, a tracked-changes record, or a second rendition of one framed object.
    ///
    /// Counted **once per region, and only when the region held a block with characters in it** —
    /// so a comment carrying nothing but its author's name is not an erasure, and a footnote
    /// containing a comment is two rather than one.
    pub regions_not_read: u32,
    /// Blocks that contained an [`Element::Foreign`] subtree holding characters.
    ///
    /// Counted per block rather than per element: an image with both a title and a description is
    /// one place where text this reader does not treat as the paragraph's own was passed over, and
    /// a caller reconciling the artifact against the document needs the number of *places*.
    pub foreign_text_not_read: u32,
}

/// Whether these bytes are an OpenDocument **text** package, **read from the bytes**.
///
/// Three questions, all answered by the file and none by its name (**A4**): does it open like a
/// ZIP, is its **first physical** entry a **stored** `mimetype`, and is that entry's content
/// **exactly** [`ODT_MEDIA_TYPE`]?
///
/// Exactly, not "containing": `application/vnd.oasis.opendocument.text-template` — an `.ott` —
/// contains this string as a prefix, and is a different document kind. The looser rule would claim
/// it and read it under a profile that does not describe it.
///
/// The first-and-stored requirement is the package specification's, not this reader's invention:
/// ODF requires it so a consumer can identify a document from its leading bytes. Checking it is
/// what keeps a spreadsheet (`…opendocument.spreadsheet`) and a presentation
/// (`…opendocument.presentation`) from being claimed here — those are different vocabularies with
/// different readers, and returning an empty text document for one would be a gap presented as a
/// success.
pub fn is_odt(bytes: &[u8]) -> bool {
    declared_media_type(bytes).is_some_and(|declared| declared == ODT_MEDIA_TYPE)
}

/// The media type this package's `mimetype` entry declares, if it has a conforming one.
///
/// `None` when the bytes are not a ZIP, when the first entry is not a stored `mimetype`, or when
/// its contents are not UTF-8. Deliberately **not** an error: this is the detection question, and
/// a package that fails it is simply not an ODF package for [`crate::read`]'s purposes.
pub fn declared_media_type(bytes: &[u8]) -> Option<String> {
    if !crate::zip::looks_like_zip(bytes) {
        return None;
    }
    match crate::zip::first_entry(bytes) {
        Ok(Some((name, stored))) if name == MIMETYPE_ENTRY && stored => {
            let raw = crate::zip::read_entry_for_detection(bytes, MIMETYPE_ENTRY).ok()?;
            // Trimmed, because the entry is a bare media type with no XML around it and a writer
            // that appended a newline still declared the same type. Nothing else is repaired.
            Some(std::str::from_utf8(&raw).ok()?.trim().to_string())
        }
        _ => None,
    }
}

/// One `<manifest:file-entry>` of `META-INF/manifest.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    /// `manifest:full-path`, the package's own name for the entry.
    pub full_path: String,
    /// Whether the entry carries `<manifest:encryption-data>`.
    pub encrypted: bool,
}

/// Read `META-INF/manifest.xml` into its file entries.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, if the part is truncated, or if a
/// `<manifest:file-entry>` carries no `manifest:full-path` — an entry that names nothing binds
/// nothing, and skipping it would let an encrypted `content.xml` pass as an absent declaration.
pub fn read_manifest(part: &[u8]) -> Result<Vec<ManifestEntry>, EngineError> {
    let mut reader = new_reader(part, MANIFEST_PART)?;
    let mut entries: Vec<ManifestEntry> = Vec::new();
    let mut depth: i32 = 0;
    // `Some` while inside a `<manifest:file-entry>` that may still gain an encryption child.
    let mut open: Option<ManifestEntry> = None;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, MANIFEST_PART, &e)),
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                depth += 1;
                match local_name(start.name().as_ref()) {
                    b"file-entry" => open = Some(file_entry(&start)?),
                    b"encryption-data" => {
                        if let Some(entry) = open.as_mut() {
                            entry.encrypted = true;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(start)) => match local_name(start.name().as_ref()) {
                // The form every writer emits for an unencrypted entry, so this is the arm that
                // fires in an ordinary package.
                b"file-entry" => entries.push(file_entry(&start)?),
                b"encryption-data" => {
                    if let Some(entry) = open.as_mut() {
                        entry.encrypted = true;
                    }
                }
                _ => {}
            },
            Ok(Event::End(end)) => {
                depth -= 1;
                if local_name(end.name().as_ref()) == b"file-entry" {
                    if let Some(entry) = open.take() {
                        entries.push(entry);
                    }
                }
            }
            Ok(_) => {}
        }
    }

    check_closed(depth, MANIFEST_PART)?;
    Ok(entries)
}

fn file_entry(start: &BytesStart<'_>) -> Result<ManifestEntry, EngineError> {
    let mut full_path = None;
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: MANIFEST_PART.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        if local_name(attribute.key.as_ref()) == b"full-path" {
            full_path = Some(attribute_value(&attribute, MANIFEST_PART)?);
        }
    }
    match full_path {
        Some(full_path) => Ok(ManifestEntry {
            full_path,
            encrypted: false,
        }),
        None => Err(EngineError::Malformed {
            what: MANIFEST_PART.into(),
            detail: "a `<manifest:file-entry>` carries no `manifest:full-path`, so it declares \
                     nothing about any part. Skipping it would let an encrypted `content.xml` \
                     look like a package that simply did not mention one."
                .into(),
        }),
    }
}

/// Check the manifest actually declares a readable `content.xml`, and say why if it does not.
///
/// # Errors
///
/// [`EngineError::MissingPart`] if no entry names [`CONTENT_PART`], and
/// [`EngineError::Unsupported`] if the entry declares it encrypted. Both fail closed: an
/// encrypted part handed to the XML reader would come back as "will not parse", which names the
/// wrong cause, and a package that declares no content part is not one this reader can speak for.
pub fn check_content_declared(entries: &[ManifestEntry]) -> Result<(), EngineError> {
    // **Two declarations of one part is a refusal, not a first-wins.** `find` stops at the first
    // match, and `read_manifest` preserves document order — so an unencrypted declaration written
    // ahead of an encrypted one would sail past the check below and hand ciphertext to the XML
    // reader, which is precisely what that check exists to prevent. A manifest that says two
    // different things about `content.xml` is one this reader cannot speak for.
    if entries
        .iter()
        .filter(|e| e.full_path == CONTENT_PART)
        .count()
        > 1
    {
        return Err(EngineError::Malformed {
            what: MANIFEST_PART.into(),
            detail: format!(
                "`{MANIFEST_PART}` declares `{CONTENT_PART}` more than once. Two declarations of \
                 one part may disagree — about encryption, about media type — and choosing the \
                 first would be this reader picking which of the package's own claims to believe."
            ),
        });
    }
    let Some(entry) = entries.iter().find(|e| e.full_path == CONTENT_PART) else {
        return Err(EngineError::MissingPart {
            part: format!(
                "`{CONTENT_PART}` — `{MANIFEST_PART}` is the package's own list of what it \
                 contains and it does not declare one, so there is no content this reader may \
                 claim to have read"
            ),
        });
    };
    if entry.encrypted {
        return Err(EngineError::Unsupported {
            what: "encrypted OpenDocument package".into(),
            detail: format!(
                "`{MANIFEST_PART}` declares `{CONTENT_PART}` encrypted. This reader holds no \
                 password and decrypts nothing; refused by name rather than handed to the XML \
                 reader, which would report the ciphertext as malformed XML and name the wrong \
                 cause."
            ),
        });
    }
    Ok(())
}

/// How many entries of this package hold content that this slice does not read (**A14**).
///
/// Everything in the central directory except the three entries this reader consumes and the
/// directory entries a writer may record. That deliberately over-counts — `settings.xml` holds a
/// cursor position nobody would cite — and over-counting is the direction A14 asks for: the number
/// tells a caller there is more in the package, and the alternative is a list of prefixes that
/// silently stops matching the first time a producer names something new.
pub fn unread_entries(entry_names: &[String]) -> u32 {
    let matched = entry_names
        .iter()
        .filter(|name| !name.ends_with('/') && !PACKAGING_ENTRIES.contains(&name.as_str()))
        .count();
    crate::declared_len(matched)
}

/// Read a `content.xml` into the blocks that carry text.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse or the part ends with elements still open,
/// and [`EngineError::ResourceLimit`] if the part's blocks nest past [`MAX_BLOCK_NESTING`] or its
/// text expands past [`MAX_TEXT_BYTES`].
pub fn read_content(part: &[u8]) -> Result<Content, EngineError> {
    let mut reader = new_ns_reader(part, CONTENT_PART)?;
    let mut paragraphs: Vec<Paragraph> = Vec::new();
    let mut regions_not_read = 0u32;
    let mut foreign_text_not_read = 0u32;

    // The blocks currently open, innermost last. A stack rather than a single slot because ODF
    // nests them: a text box's paragraphs, a table cell's paragraphs and a list item's paragraphs
    // all sit inside whatever block anchors them. Text goes to the innermost one, which is what
    // keeps a footnote's sentence out of the middle of the sentence citing it.
    let mut open: Vec<OpenBlock> = Vec::new();
    // 1-based, advanced for **every** block the part contains — including ones in a region this
    // reader passes over, because the address is a position in the file.
    let mut ordinal: u32 = 0;
    let mut depth: i32 = 0;
    // Characters appended to all blocks so far, so a part inside the ZIP reader's size cap cannot
    // expand past this one. `<text:s text:c="4096"/>` is 23 source bytes producing 4096 output
    // ones, so the part cap alone bounds nothing useful.
    let mut text_bytes: usize = 0;

    // The regions currently being passed over, innermost last. **A stack, because these regions
    // genuinely nest**: ODF puts a comment inside a footnote body, and a `<text:deletion>` holds
    // whole text bodies that may contain either. A single slot registered only the outermost, so N
    // nested regions declared one erasure — a count that under-declares, which is the direction
    // A14 exists to prevent.
    let mut skips: Vec<Skip> = Vec::new();
    // One flag per open `<draw:frame>`: whether a `<draw:text-box>` has already been read from it.
    // A frame's children are alternative renditions of one object, so the first this reader can
    // process is the one it uses and the rest are duplicates of a phrase already in the record.
    //
    // A stack rather than a flag, and pushed and popped **unconditionally** — frames nest, and the
    // two operations have to be guarded identically or a frame inside a passed-over region pops an
    // enclosing frame's flag.
    let mut frame_taken: Vec<bool> = Vec::new();

    loop {
        let (resolved, event) = match reader.read_resolved_event() {
            Ok(pair) => pair,
            Err(e) => return Err(parse_error_at(reader.buffer_position(), CONTENT_PART, &e)),
        };
        let namespace = namespace_of(&resolved).map(|ns| ns.to_vec());
        let namespace = namespace.as_deref();

        match event {
            Event::Eof => break,

            Event::Start(start) => {
                depth += 1;
                let qualified = start.name();
                let element = classify(namespace, local_name(qualified.as_ref()));

                // A region this slice does not read. Entered even inside another one, so each is
                // counted on its own terms.
                let entering_region = match element {
                    Element::Region => true,
                    // A second rendition in this frame, or a text box outside any frame. The
                    // first is a duplicate of a phrase already in the record; the second cannot
                    // happen in a conforming package. Both are passed over rather than guessed at.
                    Element::TextBox => match frame_taken.last_mut() {
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
                }

                match element {
                    Element::Frame => frame_taken.push(false),
                    // **The counter advances whether or not this subtree is read.** A consumer
                    // checking the locator counts `<text:p>` and `<text:h>` elements in the file,
                    // so an address after a footnote must not be one short.
                    Element::Block { heading } => {
                        ordinal += 1;
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth += 1;
                        } else {
                            if open.len() >= MAX_BLOCK_NESTING {
                                return Err(EngineError::ResourceLimit {
                                    limit: format!("nested blocks in `{CONTENT_PART}`"),
                                    configured: MAX_BLOCK_NESTING.to_string(),
                                });
                            }
                            // **Only a `<text:h>` states one.** Reading the attribute on a
                            // `<text:p>` would record a level for a block that declared none,
                            // and would refuse a whole package over a stray value ODF gives no
                            // meaning where it sits — `spaces`'s rule for a count in a branch
                            // nobody reads, applied to a block that is not a heading.
                            let level = if heading {
                                outline_level(&reader, &start)?
                            } else {
                                None
                            };
                            open.push(OpenBlock {
                                ordinal,
                                heading,
                                outline_level: level,
                                text: String::new(),
                                pending_space: false,
                                foreign_depth: 0,
                                foreign_text: false,
                            });
                        }
                    }
                    Element::Space => {
                        let spaces = spaces(&start, !skips.is_empty())?;
                        push_stated(&mut open, &spaces, &mut skips, &mut text_bytes)?;
                    }
                    Element::Tab => push_stated(&mut open, "\t", &mut skips, &mut text_bytes)?,
                    Element::LineBreak => {
                        push_stated(&mut open, "\n", &mut skips, &mut text_bytes)?
                    }
                    // Not the block's text, and not dropped in silence either.
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
                let element = classify(namespace, local_name(qualified.as_ref()));
                match element {
                    // A self-closing element starts no subtree, and is still an element the part
                    // contains: `<text:p/>` is what a writer emits for a blank line, so the
                    // address must not turn on how the document was serialized.
                    Element::Block { .. } => {
                        ordinal += 1;
                    }
                    // **And neither must the frame's one slot.** Claimed here as well as on the
                    // `Start` path, or `<draw:text-box/>` and `<draw:text-box></draw:text-box>`
                    // — the same document, two serializations — would disagree about which
                    // rendition was read and what was declared.
                    Element::TextBox => {
                        if let Some(taken) = frame_taken.last_mut() {
                            *taken = true;
                        }
                    }
                    Element::Space => {
                        let spaces = spaces(&start, !skips.is_empty())?;
                        push_stated(&mut open, &spaces, &mut skips, &mut text_bytes)?;
                    }
                    Element::Tab => push_stated(&mut open, "\t", &mut skips, &mut text_bytes)?,
                    Element::LineBreak => {
                        push_stated(&mut open, "\n", &mut skips, &mut text_bytes)?
                    }
                    // Read and refused in one line, which is the whole of this format's argument
                    // with `docs/history/14-V2-SCOPE.md` §3: the producing application's page break is
                    // seen, recognised, and not turned into a `PageRecord`. It contributes no
                    // character either — it is a layout mark, not text the document states.
                    Element::SoftPageBreak => {}
                    _ => {}
                }
            }

            Event::End(end) => {
                depth -= 1;
                let qualified = end.name();
                let element = classify(namespace, local_name(qualified.as_ref()));

                // A passed-over region ends where it began. Its body text, if it had any, is
                // declared — once, for this region alone.
                let mut closed_region = false;
                if let Some(skip) = skips.last() {
                    if depth + 1 == skip.from_depth {
                        let skip = skips.pop().expect("checked above");
                        if skip.held_text {
                            regions_not_read = crate::declare(regions_not_read, 1);
                        }
                        closed_region = true;
                    }
                }

                match element {
                    Element::Frame => {
                        frame_taken.pop();
                    }
                    Element::Block { .. } => {
                        if let Some(skip) = skips.last_mut() {
                            skip.block_depth = skip.block_depth.saturating_sub(1);
                        } else if let Some(block) = open.pop() {
                            if block.foreign_text {
                                foreign_text_not_read = crate::declare(foreign_text_not_read, 1);
                            }
                            // A block with no characters — a spacer paragraph, an empty heading —
                            // is not a node, for the reason a `<w:r>` with no `<w:t>` is not:
                            // nothing was erased, because there was never a character there. A
                            // block of nothing but collapsible whitespace lands here too, because
                            // ODF's own rule says that block displays nothing.
                            if !block.text.is_empty() {
                                paragraphs.push(Paragraph {
                                    ordinal: block.ordinal,
                                    heading: block.heading,
                                    outline_level: block.outline_level,
                                    text: block.text,
                                });
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
                let decoded = decode(&text, CONTENT_PART)?;
                push_source(&mut open, decoded.as_ref(), &mut skips, &mut text_bytes)?;
            }
            // Matched, not ignored: an unhandled `CData` arm is a silent drop.
            Event::CData(cdata) => {
                let decoded = cdata_text(&cdata, CONTENT_PART)?;
                push_source(&mut open, decoded.as_ref(), &mut skips, &mut text_bytes)?;
            }
            Event::GeneralRef(entity) => {
                let resolved = resolve_reference(entity.as_ref(), CONTENT_PART)?;
                let resolved = resolved.as_ref();
                push_source(&mut open, resolved, &mut skips, &mut text_bytes)?;
            }
            _ => {}
        }
    }

    check_closed(depth, CONTENT_PART)?;
    // Emitted in the part's own document order, which nesting alone does not give: an outer block
    // closes after the inner ones it contains, so the pop order is not the file's order.
    paragraphs.sort_by_key(|p| p.ordinal);
    Ok(Content {
        paragraphs,
        regions_not_read,
        foreign_text_not_read,
    })
}

/// Append **character data** to the innermost open block, under ODF's whitespace rule.
///
/// A tab, a line feed, a carriage return or a space in the source is one collapsible space; a run
/// of them is still one; and one at either end of the block is not part of it. See the module
/// header for why that rule is the format's rather than a renderer's.
///
/// Three destinations, and which one the characters reach is the whole of [`Element`]'s argument:
/// a passed-over region records that it held body text, a [`Element::Foreign`] subtree records
/// that its block passed some over, and anything else is the block's own text.
pub(crate) fn push_source(
    open: &mut [OpenBlock],
    text: &str,
    skips: &mut [Skip],
    text_bytes: &mut usize,
) -> Result<(), EngineError> {
    let has_characters = text.chars().any(|c| !is_collapsible(c));
    if let Some(skip) = skips.last_mut() {
        // Whitespace alone is not text a region held, and neither is a note's citation mark or a
        // comment's author: only characters inside one of the region's own blocks are an erasure.
        if has_characters && skip.block_depth > 0 {
            skip.held_text = true;
        }
        return Ok(());
    }
    let Some(block) = open.last_mut() else {
        return Ok(());
    };
    if block.foreign_depth > 0 {
        if has_characters {
            block.foreign_text = true;
        }
        return Ok(());
    }
    for character in text.chars() {
        if is_collapsible(character) {
            // Deferred, never appended: this is what makes a run collapse to one and a run at the
            // end of the block disappear, without a second pass over the string.
            block.pending_space = !block.text.is_empty();
        } else {
            if block.pending_space {
                push_char(block, ' ', text_bytes)?;
                block.pending_space = false;
            }
            push_char(block, character, text_bytes)?;
        }
    }
    Ok(())
}

/// Append characters the file states **as an element**: `<text:s>`, `<text:tab>`,
/// `<text:line-break>`.
///
/// Exempt from the collapsing rule, because surviving it is the entire reason ODF spells them this
/// way. A collapsible space already seen is written out first, so `a <text:s text:c="2"/>b` is the
/// three spaces the document displays rather than two.
pub(crate) fn push_stated(
    open: &mut [OpenBlock],
    text: &str,
    skips: &mut [Skip],
    text_bytes: &mut usize,
) -> Result<(), EngineError> {
    if text.is_empty() {
        return Ok(());
    }
    if let Some(skip) = skips.last_mut() {
        // Symmetric with `push_source`: a stated character inside a region's block is that
        // region's body text, and one outside every block — a stray tab between two elements —
        // is not. Without the `block_depth` test the two paths would disagree about whether the
        // same passed-over subtree held anything.
        if skip.block_depth > 0 {
            skip.held_text = true;
        }
        return Ok(());
    }
    let Some(block) = open.last_mut() else {
        return Ok(());
    };
    if block.foreign_depth > 0 {
        block.foreign_text = true;
        return Ok(());
    }
    if block.pending_space {
        push_char(block, ' ', text_bytes)?;
        block.pending_space = false;
    }
    for character in text.chars() {
        push_char(block, character, text_bytes)?;
    }
    Ok(())
}

/// Append one character, against the part's total text budget.
///
/// The budget is on the **total**, not on any one element, because that is where the amplification
/// lives: `zip.rs` caps a part at 256 MiB of source, and `<text:s text:c="4096"/>` turns 23 of
/// those bytes into 4096. `zip.rs`'s own rule is that a bomb is "a named refusal rather than an
/// out-of-memory kill", and a per-element cap does not deliver it.
fn push_char(
    block: &mut OpenBlock,
    character: char,
    text_bytes: &mut usize,
) -> Result<(), EngineError> {
    let len = character.len_utf8();
    if *text_bytes + len > MAX_TEXT_BYTES {
        return Err(EngineError::ResourceLimit {
            limit: format!("text bytes expanded from `{CONTENT_PART}`"),
            configured: MAX_TEXT_BYTES.to_string(),
        });
    }
    *text_bytes += len;
    block.text.push(character);
    Ok(())
}

/// The four characters ODF treats as one collapsible space wherever they appear in text content.
fn is_collapsible(character: char) -> bool {
    matches!(character, ' ' | '\t' | '\n' | '\r')
}

/// The number of spaces a `<text:s>` states, as `text:c` — which defaults to one.
///
/// **Read, never inferred.** `text:c` is a count the file wrote down; the alternative would be
/// deriving spaces from where words sit on a rendered line, which is the layout this reader does
/// not perform. A `text:c` that is not a number is a named refusal rather than a repaired one: a
/// repaired count produces text the document does not contain.
///
/// Three things about the parse are the schema's rather than this reader's taste:
///
/// - **`text:c` is trimmed**, because its XSD type is `positiveInteger`, whose `whiteSpace` facet
///   is `collapse`. `" 3"` is a valid lexical form of 3 and every ODF consumer reads it as three
///   spaces; refusing it would refuse a conforming document.
/// - **Zero is refused**, for the same reason a non-numeric count is. `positiveInteger` excludes
///   it, and `" ".repeat(0)` is the empty string — which would silently join `Name` and `Value`
///   into `NameValue`, the exact failure this module's header says these elements exist to prevent.
/// - **`xmlns:c` is not `text:c`.** [`local_name`] matches by suffix, so a namespace declaration
///   would be read as the count and refuse the document — or, with a numeric URI, silently state
///   the wrong number of spaces. Namespace bindings are skipped before the match.
///
/// `skipping` short-circuits to one space, and it is not laziness: inside a passed-over region the
/// only question is whether the region held characters at all, and **a malformed count in a branch
/// nobody reads must not refuse the document** — v2-S4's rule for a shape id in a skipped
/// `<mc:Choice>`, applied to the one attribute this reader parses.
pub(crate) fn spaces(start: &BytesStart<'_>, skipping: bool) -> Result<String, EngineError> {
    if skipping {
        return Ok(" ".into());
    }
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: CONTENT_PART.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        // A namespace declaration is not an attribute of the element in this sense, and
        // `xmlns:c`'s local name is `c`.
        if attribute.key.as_ref().starts_with(b"xmlns") {
            continue;
        }
        if local_name(attribute.key.as_ref()) == b"c" {
            let raw = attribute_value(&attribute, CONTENT_PART)?;
            let count: usize = raw
                .trim()
                .parse()
                .ok()
                .filter(|count| *count > 0)
                .ok_or_else(|| EngineError::Malformed {
                    what: CONTENT_PART.into(),
                    detail: format!(
                        "`<text:s text:c=\"{raw}\">` does not state a number of spaces. \
                             `text:c` is a positive integer; refused rather than repaired, \
                             because a repaired count puts characters in the text that the \
                             document did not state — or, for a zero, silently removes the \
                             separator this element exists to carry."
                    ),
                })?;
            // A count past any real run of spaces is a resource question, not a text question.
            // 4096 is far past a real document and far short of a memory problem; the part's
            // running total in `push_char` is what bounds the aggregate.
            if count > MAX_SPACES_PER_ELEMENT {
                return Err(EngineError::ResourceLimit {
                    limit: format!("spaces in one `<text:s>` of `{CONTENT_PART}`"),
                    configured: MAX_SPACES_PER_ELEMENT.to_string(),
                });
            }
            return Ok(" ".repeat(count));
        }
    }
    Ok(" ".into())
}

/// `text:outline-level` on a `<text:h>`, or `None` where the element states none.
///
/// **The level the file states, and nothing where it states nothing.** ODF makes the attribute
/// optional, and a `<text:h>` that omits it takes its level from the outline style in
/// `styles.xml` — a part [`unread_entries`] declares this reader did not open. Defaulting to `1`
/// here would put a number on the wire that came out of a part the artifact says was not read,
/// which is the half-claim `OfficeParagraphAttributes` refuses; `None` says the element stated no
/// level, which is what happened.
///
/// `u32` rather than `u8`, because ODF types the attribute as a positive integer and names no
/// ceiling: a conforming `text:outline-level="300"` is a level, and this reader carries it as
/// written. What a projection does with a level past `h6` is the projection's question — a reader
/// that clamped it here would be repairing a document that is not broken.
///
/// Zero and a value that will not parse are refused by name, for [`spaces`]'s reason and
/// `ods::repeat_count`'s: the attribute exists to state a heading's depth, so a value this reader
/// cannot read is a depth of unknown size, and guessing one would claim a structure the document
/// did not state.
pub(crate) fn outline_level(
    reader: &quick_xml::NsReader<&[u8]>,
    start: &BytesStart<'_>,
) -> Result<Option<u32>, EngineError> {
    let Some(raw) = resolved_attribute(reader, start, NS_TEXT, b"outline-level", CONTENT_PART)?
    else {
        return Ok(None);
    };
    match raw.trim().parse::<u32>() {
        Ok(0) | Err(_) => Err(EngineError::Malformed {
            what: CONTENT_PART.into(),
            detail: format!(
                "`<text:h text:outline-level=\"{raw}\">` does not state a heading level. \
                 `text:outline-level` is a positive integer; refused rather than repaired, \
                 because a repaired level claims a place in the document's outline that the \
                 document did not state — and a zero states a heading at no depth at all."
            ),
        }),
        Ok(n) => Ok(Some(n)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real OpenDocument namespace URIs, because the reader now resolves them.
    ///
    /// A test that bound `text:` to `"t"` would exercise nothing this reader does: every element
    /// would classify as [`Element::Foreign`] and every assertion below would be about the
    /// fallback path rather than the real one.
    const OPEN: &str = concat!(
        r#"<?xml version="1.0"?><office:document-content"#,
        r#" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0""#,
        r#" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0""#,
        r#" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0""#,
        r#" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0""#,
        r#" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0""#,
        r#" xmlns:dc="http://purl.org/dc/elements/1.1/""#,
        r#"><office:body><office:text>"#
    );
    const CLOSE: &str = "</office:text></office:body></office:document-content>";

    fn body(inner: &str) -> String {
        format!("{OPEN}{inner}{CLOSE}")
    }

    fn read(inner: &str) -> Content {
        read_content(body(inner).as_bytes()).expect("well-formed")
    }

    #[test]
    fn blocks_carry_the_positions_the_part_states() {
        let content = read(
            "<text:h text:outline-level=\"1\">Evidence</text:h>\
             <text:p>A quote binds to a paragraph</text:p>\
             <text:p/>\
             <text:p>and never to a page.</text:p>",
        );
        let addressed: Vec<(u32, bool, &str)> = content
            .paragraphs
            .iter()
            .map(|p| (p.ordinal, p.heading, p.text.as_str()))
            .collect();
        assert_eq!(
            addressed,
            vec![
                (1, true, "Evidence"),
                (2, false, "A quote binds to a paragraph"),
                // 3 is the self-closing blank line: no text, so no node, but the count saw it.
                (4, false, "and never to a page."),
            ]
        );
    }

    #[test]
    fn a_span_is_part_of_its_paragraph_rather_than_a_second_address() {
        let content = read(
            "<text:p>The <text:span text:style-name=\"T1\">important</text:span> part.</text:p>",
        );
        assert_eq!(content.paragraphs.len(), 1, "one sentence, one node");
        assert_eq!(content.paragraphs[0].text, "The important part.");
    }

    /// The defect a descendant walk would have: a footnote's body spliced into the sentence.
    #[test]
    fn a_nested_block_does_not_leak_into_the_block_that_anchors_it() {
        let content = read(
            "<text:p>Cited here<text:note><text:note-citation>1</text:note-citation>\
             <text:note-body><text:p>A source nobody read.</text:p></text:note-body>\
             </text:note> and continued.</text:p>\
             <text:p>After.</text:p>",
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["Cited here and continued.", "After."]);
        assert_eq!(
            content.regions_not_read, 1,
            "the note held text, so it is declared rather than dropped in silence"
        );
        // The note's own `<text:p>` is block 2, so the block after it is block 3 — the position a
        // consumer counting elements in the file finds.
        assert_eq!(content.paragraphs[1].ordinal, 3);
    }

    #[test]
    fn a_table_cell_is_read_and_is_a_block_like_any_other() {
        let content = read(
            "<table:table><table:table-row><table:table-cell>\
             <text:p>In a cell.</text:p></table:table-cell></table:table-row></table:table>",
        );
        assert_eq!(content.paragraphs.len(), 1);
        assert_eq!(content.paragraphs[0].text, "In a cell.");
    }

    #[test]
    fn a_comment_and_a_tracked_deletion_are_passed_over_and_counted() {
        let content = read(
            "<text:tracked-changes><text:changed-region><text:deletion>\
             <text:p>Struck out.</text:p></text:deletion></text:changed-region></text:tracked-changes>\
             <text:p>Current text<office:annotation><text:p>A remark.</text:p></office:annotation>.</text:p>",
        );
        assert_eq!(
            content.paragraphs.len(),
            1,
            "neither the deletion nor the comment is the document's own text"
        );
        assert_eq!(content.paragraphs[0].text, "Current text.");
        assert_eq!(content.regions_not_read, 2);
    }

    /// A frame states alternative renditions of one object, and only the first is read.
    #[test]
    fn a_second_rendition_of_one_frame_is_not_a_second_address() {
        let content = read(
            "<text:p><draw:frame>\
             <draw:text-box><text:p>The caption.</text:p></draw:text-box>\
             <draw:text-box><text:p>The caption.</text:p></draw:text-box>\
             </draw:frame></text:p>",
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["The caption."],
            "one displayed phrase is one citable node"
        );
        assert_eq!(content.regions_not_read, 1);
    }

    #[test]
    fn spaces_tabs_and_breaks_are_the_characters_the_file_states() {
        let content = read(
            "<text:p>a<text:s text:c=\"3\"/>b</text:p>\
             <text:p>Name<text:tab/>Value</text:p>\
             <text:p>one<text:line-break/>two</text:p>\
             <text:p>x<text:s/>y</text:p>",
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["a   b", "Name\tValue", "one\ntwo", "x y"]);
    }

    /// ODF's own whitespace rule, and the reason the three elements above exist.
    #[test]
    fn source_whitespace_collapses_the_way_the_format_says_it_does() {
        let content = read(
            "<text:p>\n   A sentence\n   the producer indented.\n  </text:p>\
             <text:p>tabs\tand\nnewlines\rare spaces</text:p>\
             <text:p>   </text:p>",
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "A sentence the producer indented.",
                "tabs and newlines are spaces"
            ],
            "the text a quote binds to must not depend on how the file was serialized"
        );
        assert_eq!(
            content.paragraphs[1].ordinal, 2,
            "and the block of nothing but whitespace still held its position"
        );
    }

    /// The exemption, in both directions: a stated character survives, and a collapsible space
    /// before one is written out rather than swallowed.
    #[test]
    fn a_stated_character_is_not_collapsed_and_does_not_swallow_the_space_before_it() {
        let content = read(
            "<text:p>a <text:s text:c=\"2\"/>b</text:p>\
             <text:p><text:s text:c=\"2\"/>indented</text:p>\
             <text:p>a\n<text:tab/>b</text:p>",
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["a   b", "  indented", "a \tb"]);
    }

    #[test]
    fn a_space_count_that_is_not_a_number_is_refused_rather_than_repaired() {
        assert!(
            read_content(body("<text:p>a<text:s text:c=\"lots\"/>b</text:p>").as_bytes()).is_err()
        );
    }

    /// **The allowlist, over the shape a real producer writes.** A text-anchored `<draw:frame>` is
    /// a child of the paragraph it is anchored in, and LibreOffice writes `<svg:title>` and
    /// `<svg:desc>` whenever an image has a Title or Description. Under a descendant walk both
    /// land in the sentence.
    #[test]
    fn a_frames_title_and_description_are_not_the_anchoring_sentence() {
        let content = read(
            "<text:p>See Figure 1<draw:frame draw:name=\"F1\" text:anchor-type=\"as-char\">\
               <draw:image xlink:href=\"Pictures/1.png\"/>\
               <svg:title>Revenue by quarter</svg:title>\
               <svg:desc>Bar chart, four bars</svg:desc>\
             </draw:frame> for detail.</text:p>",
        );
        assert_eq!(
            content.paragraphs[0].text, "See Figure 1 for detail.",
            "the image's title and alt text are not characters the document displays here"
        );
        assert_eq!(
            content.foreign_text_not_read, 1,
            "and they are declared rather than dropped in silence"
        );
    }

    /// The same rule over an embedded object's base64 — the case that splices a kilobyte of
    /// `iVBORw0KGgo…` into the middle of a sentence.
    #[test]
    fn an_embedded_objects_base64_is_not_the_anchoring_sentence() {
        let content = read(
            "<text:p>Figure 1.<draw:frame><draw:image>\
               <office:binary-data>iVBORw0KGgoAAAANSUhEUg</office:binary-data>\
             </draw:image></draw:frame></text:p>",
        );
        assert_eq!(content.paragraphs[0].text, "Figure 1.");
        assert_eq!(content.foreign_text_not_read, 1);
    }

    /// **A field's cached value is the producer's arithmetic**, and this is the format where that
    /// matters most: the reader refuses `<text:soft-page-break/>` by name, so it must not put the
    /// same producer's page number into the text through a different element.
    #[test]
    fn a_fields_cached_page_number_does_not_reach_the_text() {
        let content = read(
            "<text:p>Continued on page <text:page-number text:select-page=\"next\">8</text:page-number> \
             of <text:page-count>12</text:page-count>.</text:p>",
        );
        assert_eq!(
            content.paragraphs[0].text, "Continued on page of .",
            "the words are the document's; the numbers are the word processor's"
        );
        assert_eq!(content.foreign_text_not_read, 1);
    }

    /// **`<text:number>` is a generated label**, present only for consumers that do not number —
    /// the ODF twin of the cached `<a:fld type="slidenum">` v2-S4 refused.
    #[test]
    fn a_generated_heading_number_is_not_part_of_the_heading() {
        let content = read(
            "<text:h text:outline-level=\"2\"><text:number>2.1</text:number>Scope of this report</text:h>",
        );
        assert_eq!(
            content.paragraphs[0].text, "Scope of this report",
            "not `2.1Scope of this report`, which no consumer displays and no quote binds to"
        );
        assert!(content.paragraphs[0].heading);
        assert_eq!(content.foreign_text_not_read, 1);
    }

    /// **The level the element states, and `None` where it states none.**
    ///
    /// A `<text:h>` without the attribute takes its level from an outline style in `styles.xml`,
    /// which `unread_entries` declares unread — so the honest answer is that this element stated
    /// no level, not that it stated level one.
    #[test]
    fn a_headings_outline_level_is_read_and_an_absent_one_is_not_defaulted() {
        let content = read(
            "<text:h text:outline-level=\"2\">Scope</text:h>\
             <text:h>Unlevelled</text:h>",
        );
        assert_eq!(content.paragraphs[0].outline_level, Some(2));
        assert_eq!(
            content.paragraphs[1].outline_level, None,
            "`None` rather than `Some(1)`: the level would have come from a part this reader \
             declares it did not open"
        );
        assert!(
            content.paragraphs[1].heading,
            "still a heading, with no level stated"
        );
    }

    /// **ODF gives the attribute no meaning on a `<text:p>`, so it is not read there.**
    ///
    /// Recording it would put a level on a block that declared none, and refusing on it would
    /// fail a package over an attribute that means nothing where it sits.
    #[test]
    fn a_paragraphs_outline_level_is_not_read() {
        let content = read("<text:p text:outline-level=\"1\">Not a heading</text:p>");
        assert!(!content.paragraphs[0].heading);
        assert_eq!(content.paragraphs[0].outline_level, None);
    }

    /// **A level past `h6` is carried as written.** ODF types the attribute as a positive integer
    /// and names no ceiling; clamping here would repair a document that is not broken, and what a
    /// projection emits for a level it has no element for is the projection's question.
    #[test]
    fn an_outline_level_past_the_six_html_headings_is_carried_rather_than_clamped() {
        let content = read("<text:h text:outline-level=\"300\">Deep</text:h>");
        assert_eq!(content.paragraphs[0].outline_level, Some(300));
    }

    /// The `text:c` refusal, for the attribute that states a heading's depth.
    #[test]
    fn an_outline_level_that_is_not_a_positive_integer_is_refused_rather_than_repaired() {
        for raw in ["0", "deep", "-1", ""] {
            let xml = body(&format!("<text:h text:outline-level=\"{raw}\">H</text:h>"));
            assert!(
                read_content(xml.as_bytes()).is_err(),
                "`text:outline-level=\"{raw}\"` states no depth, and a guessed one claims a \
                 place in the outline the document did not"
            );
        }
    }

    /// …and the same value inside a region nobody reads does **not** refuse the document, which is
    /// why the read sits in the branch that opens a block rather than in `classify`.
    #[test]
    fn a_malformed_outline_level_in_a_passed_over_region_does_not_refuse_the_document() {
        let content = read(
            "<text:p>read<text:note><text:note-body>\
             <text:h text:outline-level=\"deep\">skipped</text:h>\
             </text:note-body></text:note></text:p>",
        );
        assert_eq!(content.paragraphs.len(), 1);
        assert_eq!(content.paragraphs[0].text, "read");
        assert_eq!(content.regions_not_read, 1);
    }

    /// **Ruby: the base is the word, the ruby text is the pronunciation guide.** The ODF twin of
    /// the `<rPh>` furigana exclusion v2-S3 made on both of its string paths.
    #[test]
    fn ruby_annotation_text_is_not_concatenated_into_the_word() {
        let content = read(
            "<text:p>The <text:ruby><text:ruby-base>漢字</text:ruby-base>\
             <text:ruby-text>かんじ</text:ruby-text></text:ruby> in question.</text:p>",
        );
        assert_eq!(
            content.paragraphs[0].text, "The 漢字 in question.",
            "the base survives and the guide does not"
        );
        assert_eq!(content.foreign_text_not_read, 1);
    }

    /// **A foreign element does not move an address.** ODF §3.17 permits foreign elements in mixed
    /// content, so namespace blindness here would be a wrong locator rather than missing content —
    /// which is why this reader resolves names where the three OOXML readers match by suffix.
    #[test]
    fn a_foreign_element_with_a_block_name_neither_counts_nor_emits() {
        let content = read(
            "<text:p>One</text:p>\
             <xhtml:p xmlns:xhtml=\"http://www.w3.org/1999/xhtml\">foreign</xhtml:p>\
             <text:p>Two</text:p>",
        );
        let addressed: Vec<(u32, &str)> = content
            .paragraphs
            .iter()
            .map(|p| (p.ordinal, p.text.as_str()))
            .collect();
        assert_eq!(
            addressed,
            vec![(1, "One"), (2, "Two")],
            "a consumer counting `text:p` in the file finds Two at 2, and so does the locator"
        );
    }

    /// **MathML's `<annotation>` is not a comment.** Same local name, different namespace — and
    /// under suffix matching an inline formula was declared to the caller as an unread reviewer's
    /// remark, naming a gap the document does not have.
    #[test]
    fn a_mathml_annotation_is_not_declared_as_a_comment() {
        let content = read(
            "<text:p>Where <draw:frame><draw:object>\
               <math:math xmlns:math=\"http://www.w3.org/1998/Math/MathML\"><semantics>\
                 <mrow><mi>x</mi></mrow>\
                 <annotation encoding=\"StarMath 5.0\">x sup 2</annotation>\
               </semantics></math:math>\
             </draw:object></draw:frame> holds.</text:p>",
        );
        assert_eq!(content.paragraphs[0].text, "Where holds.");
        assert_eq!(
            content.regions_not_read, 0,
            "no footnote, comment or tracked change is in this document, so none is declared"
        );
        assert_eq!(
            content.foreign_text_not_read, 1,
            "the formula is declared as what it is: text inside the block this reader did not read"
        );
    }

    /// **Nested regions count separately.** ODF puts a comment inside a footnote body, and a
    /// single-slot skip declared the two as one erasure.
    #[test]
    fn a_comment_inside_a_footnote_is_two_regions_not_one() {
        let content = read(
            "<text:p>Cited<text:note><text:note-body>\
               <text:p>A source nobody read.\
                 <office:annotation><text:p>A reviewer's remark.</text:p></office:annotation>\
               </text:p>\
             </text:note-body></text:note>.</text:p>",
        );
        assert_eq!(content.paragraphs.len(), 1);
        assert_eq!(content.paragraphs[0].text, "Cited.");
        assert_eq!(
            content.regions_not_read, 2,
            "a footnote body and a comment are two passages a caller would find missing"
        );
    }

    /// **A comment with no body is not an erasure.** Every ODF note carries a citation mark and
    /// every real comment carries its author, so counting "did any character appear" would make the
    /// guard always true and declare an erasure for a comment that erased nothing.
    #[test]
    fn a_region_carrying_only_metadata_declares_nothing() {
        let content = read(
            "<text:p>Reviewed<office:annotation>\
               <dc:creator>A reviewer</dc:creator><dc:date>2026-01-01T00:00:00</dc:date>\
             </office:annotation>.</text:p>\
             <text:p>Cited<text:note><text:note-citation>1</text:note-citation>\
             <text:note-body></text:note-body></text:note>.</text:p>",
        );
        assert_eq!(
            content.regions_not_read, 0,
            "neither region holds a block with characters in it, so neither is a missing passage"
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, vec!["Reviewed.", "Cited."]);
    }

    /// CDATA is character data, and an unmatched arm is a silent drop. Its two sibling readers each
    /// pin theirs; this one did not, while the acceptance box claimed it did.
    #[test]
    fn cdata_is_read_rather_than_silently_dropped() {
        let content = read("<text:p>Total<![CDATA[ & due]]></text:p>");
        assert_eq!(
            content.paragraphs[0].text, "Total & due",
            "deleting the CData arm would make this `Total`"
        );
    }

    /// `text:c` is an XSD `positiveInteger`: whitespace collapses, and zero is not one.
    #[test]
    fn a_space_count_follows_its_own_schema_type() {
        assert_eq!(
            read("<text:p>a<text:s text:c=\" 3\"/>b</text:p>").paragraphs[0].text,
            "a   b",
            "a lexical form every ODF consumer reads as three must not refuse the document"
        );
        assert!(
            read_content(body("<text:p>Name<text:s text:c=\"0\"/>Value</text:p>").as_bytes())
                .is_err(),
            "zero is out of schema, and repairing it to nothing is the NameValue join this \
             element exists to prevent"
        );
    }

    /// A namespace declaration is not the count attribute, even though its local name is `c`.
    #[test]
    fn a_namespace_binding_is_not_the_space_count() {
        let content = read("<text:p>a<text:s xmlns:c=\"urn:x\" text:c=\"3\"/>b</text:p>");
        assert_eq!(content.paragraphs[0].text, "a   b");
    }

    /// …and the same count inside a region nobody reads does **not** refuse the document — v2-S4's
    /// rule for a malformed shape id in a skipped branch.
    #[test]
    fn a_malformed_space_count_in_a_passed_over_region_does_not_refuse_the_document() {
        let content = read(
            "<text:p>read<text:note><text:note-body>\
             <text:p>a<text:s text:c=\"lots\"/>b</text:p></text:note-body></text:note></text:p>",
        );
        assert_eq!(content.paragraphs.len(), 1);
        assert_eq!(content.paragraphs[0].text, "read");
        assert_eq!(content.regions_not_read, 1);
    }

    /// **The frame stack stays balanced across a passed-over region.** A `<draw:frame>` inside a
    /// footnote is never read, but its `</draw:frame>` still pops — so if the push were guarded and
    /// the pop were not, it would pop the *enclosing* frame's flag, and that frame's first text box
    /// would then read as a second rendition and be declared unread.
    #[test]
    fn a_frame_inside_a_passed_over_region_does_not_unbalance_the_enclosing_one() {
        // The inner frame is written with **explicit Start and End tags**. Written `<draw:frame/>`
        // it is an `Event::Empty`, which touches neither the push nor the pop — so the push/pop
        // asymmetry this test is named for was never created and the mutation it claims to catch
        // sailed through. A self-closing frame is balanced by construction; only a nested open
        // frame can unbalance the stack.
        let content = read(
            "<text:p><draw:frame>\
               <text:note><text:note-body><text:p>note<draw:frame></draw:frame></text:p></text:note-body></text:note>\
               <draw:text-box><text:p>The caption.</text:p></draw:text-box>\
             </draw:frame></text:p>",
        );
        let texts: Vec<&str> = content.paragraphs.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(
            texts,
            vec!["The caption."],
            "the enclosing frame's first text box is still its first"
        );
        assert_eq!(content.regions_not_read, 1, "the note, and nothing else");
    }

    /// The producing application's own page break is seen and refused, not counted or emitted.
    ///
    /// Both places a writer puts one: between two blocks, and inside the block it splits.
    #[test]
    fn a_soft_page_break_contributes_nothing_and_is_not_an_address() {
        let content = read(
            "<text:p>before</text:p><text:soft-page-break/>\
             <text:p>split<text:soft-page-break/>across</text:p>",
        );
        let addressed: Vec<(u32, &str)> = content
            .paragraphs
            .iter()
            .map(|p| (p.ordinal, p.text.as_str()))
            .collect();
        assert_eq!(
            addressed,
            vec![(1, "before"), (2, "splitacross")],
            "the break is not a block and not a character: it moves no address, adds nothing to \
             the text, and becomes no page"
        );
    }

    #[test]
    fn entities_are_decoded_rather_than_carried_as_source() {
        let content = read("<text:p>Rows &amp; columns &lt; cells</text:p>");
        assert_eq!(content.paragraphs[0].text, "Rows & columns < cells");
    }

    #[test]
    fn malformed_and_truncated_content_is_refused() {
        assert!(read_content(b"<office:text><text:p>truncated").is_err());
        assert!(read_content(b"not xml at all <<<").is_err());
    }

    #[test]
    fn the_manifest_is_read_and_a_missing_content_entry_is_a_refusal() {
        let entries = read_manifest(
            br#"<manifest:manifest xmlns:manifest="m">
                  <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/>
                  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
                </manifest:manifest>"#,
        )
        .expect("well-formed");
        assert_eq!(entries.len(), 2);
        assert!(check_content_declared(&entries).is_ok());

        let without: Vec<ManifestEntry> = entries
            .into_iter()
            .filter(|e| e.full_path != CONTENT_PART)
            .collect();
        assert!(check_content_declared(&without).is_err());
    }

    #[test]
    fn an_encrypted_content_part_is_refused_by_name() {
        let entries = read_manifest(
            br#"<manifest:manifest xmlns:manifest="m">
                  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml">
                    <manifest:encryption-data manifest:checksum="x">
                      <manifest:algorithm manifest:algorithm-name="Blowfish CFB"/>
                    </manifest:encryption-data>
                  </manifest:file-entry>
                </manifest:manifest>"#,
        )
        .expect("well-formed");
        let error = check_content_declared(&entries).expect_err("no password, no reading");
        assert!(
            error.to_string().contains("encrypted"),
            "the refusal names its reason: {error}"
        );
    }

    #[test]
    fn unread_entries_are_everything_but_the_three_this_reader_consumes() {
        let names: Vec<String> = [
            "mimetype",
            "META-INF/manifest.xml",
            "content.xml",
            "styles.xml",
            "meta.xml",
            "settings.xml",
            "Pictures/",
            "Pictures/10000201.png",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            unread_entries(&names),
            4,
            "styles, meta, settings and one picture — the folder entry is not a file"
        );
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// The shape v2-S9's review reproduced in EPUB at 85×, repaired here. Both of this reader's
    /// counters are bounded only by the part's event count.
    #[test]
    fn an_odt_erasure_count_saturates_rather_than_wrapping() {
        let content = read(
            "<text:p>Cited here<text:note><text:note-citation>1</text:note-citation>\
             <text:note-body><text:p>A source nobody read.</text:p></text:note-body>\
             </text:note> and continued.</text:p>",
        );
        assert_eq!(content.regions_not_read, 1, "the note held text");

        let mut folded = u32::MAX - 1;
        for _ in 0..3 {
            folded = crate::declare(folded, content.regions_not_read);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );
    }
}
