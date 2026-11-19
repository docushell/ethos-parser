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

//! `word/document.xml` → runs, in the part's own document order.
//!
//! # What this reads, and why it is so little
//!
//! `<w:p>` and `<w:r>` and `<w:t>`. That was the whole reader, and `docs/history/15-V2-MILESTONES.md` S2
//! is explicit that it should be: the gate is *a quote resolves to a node*, and one paragraph of
//! runs proves that. Fields, drawings, comments, endnotes, track-changes and embedded workbooks are
//! all real OOXML and all absent — **and counted**, because a reader that silently returned the
//! body would let a caller conclude a phrase is absent from a document that contains it
//! (Anydoc's **A14**, declared erasure).
//!
//! **Decision #136 reads what a paragraph and a table declare**: a paragraph's style, outline level
//! and numbering ([`ParagraphProperties`], resolved through `word/styles.xml` by [`Styles`]), a
//! `<w:tbl>`'s rows, cells and merges ([`Table`]), and the header and footer parts a section
//! references ([`MainPart::references`]). [`read_part`] reads a header, a footer or the footnotes
//! part as it reads the body. Never a number Word renders from `word/numbering.xml`, and never a
//! style id read as a level without the style that defines it.
//!
//! # Why XML is a dependency when ZIP was not
//!
//! `zip.rs` explains the container. XML is the opposite call: entities, namespaces, CDATA and
//! encoding declarations are exactly where a hand-rolled reader silently gets *text* wrong, and
//! text is the evidence this engine exists to carry. `quick-xml` is MIT, and its only dependency
//! is `memchr`, which this graph already has.
//!
//! # No layout, anywhere
//!
//! Nothing here computes a position, and nothing could: `docs/history/14-V2-SCOPE.md` §3 forbids a
//! locator that addresses a rendering, and a paragraph does not know what page it falls on until
//! something lays it out. The ordinals below are positions in the XML, which is a fact the file
//! states about itself.

use ethos_parser_core::EngineError;
use quick_xml::events::Event;
use quick_xml::Reader;

/// The part every word-processing package keeps its body in.
pub const MAIN_PART: &str = "word/document.xml";

/// Parts that carry text, matched by prefix: counted where they were not read.
///
/// Prefix rather than exact name because a package numbers them — `word/header1.xml`,
/// `word/header2.xml`. Counting them is the point; naming each one is not. Since decision #136 a
/// header, a footer or the footnotes part the document references is read, and the caller leaves
/// what it read out of the names it counts.
const UNREAD_TEXT_PART_PREFIXES: [&str; 5] = [
    "word/header",
    "word/footer",
    "word/footnotes",
    "word/endnotes",
    "word/comments",
];

/// One `<w:r>`, with the addresses OOXML states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// 1-based position of the containing `<w:p>` in the part's document order.
    pub paragraph: u32,
    /// 1-based position of this `<w:r>` within that paragraph.
    pub run: u32,
    /// The text, concatenated from this run's `<w:t>` elements, verbatim, with the characters its
    /// `<w:tab/>`, `<w:br/>`, `<w:cr/>` and `<w:noBreakHyphen/>` state, in place.
    pub text: String,
    /// Whether any `<w:t>` in this run carried `xml:space="preserve"`.
    pub space_preserved: bool,
    /// What the run's paragraph's own `<w:pPr>` states (decision #136).
    pub paragraph_properties: ParagraphProperties,
    /// The table cell open where its `<w:r>` opened, as (table slot, row, cell), kept on the run
    /// rather than as an index into the runs read, which an anchoring run inserted before a text
    /// box's runs would shift (decision #136, review).
    pub(crate) cell: Option<(usize, usize, usize)>,
}

/// What a `<w:p>`'s own `<w:pPr>` states that this reader reads (decision #136), verbatim.
///
/// Not inside a `<w:pPrChange>`, which holds the properties a tracked change replaced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParagraphProperties {
    /// `<w:pStyle w:val>`: a styleId, an author-chosen token, so a level only through [`Styles`].
    pub style: Option<String>,
    /// `<w:outlineLvl w:val>`, zero-based as OOXML states it; nine is body text.
    pub outline_level: Option<u32>,
    /// `<w:numPr><w:numId w:val>`; `0` removes a numbering the style gives.
    pub num_id: Option<String>,
    /// `<w:numPr><w:ilvl w:val>`, zero-based.
    pub list_level: Option<u32>,
}

/// One `<w:tbl>` as the file states it (decision #136, `docx-tbl-v1`): its grid and its cells.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    /// One per `<w:tr>`.
    pub rows: u32,
    /// The widest row's grid columns: its `<w:gridBefore>` and its cells' `<w:gridSpan>`, summed.
    pub columns: u32,
    /// Row-major: one per `<w:tc>` but a `<w:vMerge>` continuing the cell above.
    pub cells: Vec<Cell>,
}

/// One cell of a [`Table`]: where it sits, how far it reaches, and its runs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cell {
    /// Zero-based row.
    pub row: u32,
    /// Zero-based grid column.
    pub column: u32,
    /// Rows covered: one, and one more for each `<w:vMerge>` continuing it.
    pub rowspan: u32,
    /// Grid columns covered: its `<w:gridSpan>`, one where it states none.
    pub colspan: u32,
    /// The runs of its paragraphs, as indices into [`MainPart::runs`]; a nested table's are that
    /// table's.
    pub runs: Vec<usize>,
}

/// How many `<w:basedOn>` links a style lookup follows: a chain longer than any Word writes, and
/// the bound that ends a cycle.
const MAX_STYLE_CHAIN: usize = 32;

/// Parts that hold an **embedded asset** and that no slice reads, matched by prefix.
///
/// `word/media/` is where a word-processing package stores a picture and `word/embeddings/` is
/// where it stores an embedded object — a workbook dropped into a report, most often.
///
/// **Prefix, and the package's own layout, is the whole of the identification** (Anydoc's **A4**).
/// No byte is sniffed and no extension is read: OOXML *states* where a package puts these, the
/// same way it states that a header is `word/header1.xml`. An `.png` sitting somewhere else is
/// not counted here, and a media part with no extension at all still is.
const EMBEDDED_PART_PREFIXES: [&str; 2] = ["word/media/", "word/embeddings/"];

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

/// What one `word/document.xml` holds: its runs, and what was passed over.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MainPart {
    /// The runs, in the part's own document order: each where its `<w:r>` opens, so a run anchoring
    /// a text box comes before the box's runs, holding its text from either side of the box.
    pub runs: Vec<Run>,
    /// `<mc:AlternateContent>` branches that held text and were passed over — a second or later
    /// `<mc:Choice>`, or an `<mc:Fallback>`.
    ///
    /// Counted **only when the branch actually held a `<w:t>`**, as in `pptx.rs`: a branch
    /// wrapping only a drawing's geometry is not an erasure and must not inflate the count.
    pub alternatives_not_read: u32,
    /// `<w:moveFrom>` regions that held text and were passed over: the old place of text a tracked
    /// move put elsewhere, which Word shows only with changes marked and which accepting them
    /// deletes. The text is read where the `<w:moveTo>` puts it, once.
    pub moves_not_read: u32,
    /// Its `<w:tbl>` elements, in the order they open (decision #136). A table with no cell is left
    /// out, and the document's slot budget is the caller's (`OFFICE_TABLE_MAX_SLOTS`).
    pub tables: Vec<Table>,
    /// Each `<w:headerReference>` and `<w:footerReference>`, in document order (decision #136).
    pub references: Vec<Reference>,
}

/// One `<w:headerReference>` or `<w:footerReference>` of a section (decision #136).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reference {
    /// `true` for a `<w:footerReference>`.
    pub footer: bool,
    /// Its `w:type`, verbatim: `default`, `first` or `even`; `default` where it states none.
    pub kind: String,
    /// Its `r:id`.
    pub id: String,
    /// Whether its section states `<w:titlePg/>`, without which Word shows no `first` part.
    pub title_page: bool,
}

/// Whether a `word/settings.xml` states `<w:evenAndOddHeaders/>`, without which Word shows no
/// `even` header or footer (decision #136, review).
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse.
pub fn even_and_odd_headers(part: &[u8], part_name: &str) -> Result<bool, EngineError> {
    let mut reader = crate::xml::new_reader(part, part_name)?;
    loop {
        match reader
            .read_event()
            .map_err(|e| crate::xml::parse_error(&reader, part_name, &e))?
        {
            Event::Eof => return Ok(false),
            Event::Start(e) | Event::Empty(e)
                if local_name(e.name().as_ref()) == b"evenAndOddHeaders" =>
            {
                return Ok(stated_on(attribute(&e, b"val", part_name)?.as_deref()));
            }
            _ => {}
        }
    }
}

/// An OOXML on/off property: on where `w:val` is absent, and unless it states `0`, `false` or
/// `off`.
fn stated_on(value: Option<&str>) -> bool {
    !matches!(value, Some("0" | "false" | "off"))
}

/// The paragraph styles of a `word/styles.xml` (decision #136): each one's name, the style it is
/// based on, and what its own `<w:pPr>` declares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Styles {
    by_id: std::collections::BTreeMap<String, Style>,
    default: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Style {
    name: Option<String>,
    based_on: Option<String>,
    declared: ParagraphProperties,
}

impl Styles {
    /// The styles a paragraph's properties reach: its `<w:pStyle>`, or the default paragraph
    /// style where it names none this part defines, then each `<w:basedOn>`.
    fn chain<'a>(&'a self, p: &'a ParagraphProperties) -> impl Iterator<Item = &'a Style> + 'a {
        let first = p
            .style
            .as_deref()
            .filter(|id| self.by_id.contains_key(*id))
            .or(self.default.as_deref());
        std::iter::successors(first.and_then(|id| self.by_id.get(id)), |s| {
            s.based_on.as_deref().and_then(|id| self.by_id.get(id))
        })
        .take(MAX_STYLE_CHAIN)
    }

    /// The heading level, one-based, a paragraph declares: its own `<w:outlineLvl>`, or the first
    /// style in its chain that states one or carries a built-in name `heading N`. `None` where none
    /// does, and where the level stated is body text.
    pub fn heading_level(&self, p: &ParagraphProperties) -> Option<u32> {
        let level = |l: u32| (l <= 8).then_some(l + 1);
        if let Some(l) = p.outline_level {
            return level(l);
        }
        for style in self.chain(p) {
            if let Some(l) = style.declared.outline_level {
                return level(l);
            }
            if let Some(n) = style.name.as_deref().and_then(builtin_heading) {
                return Some(n);
            }
        }
        None
    }

    /// The list level a paragraph declares: its numbering's `<w:ilvl>`, zero where none is stated,
    /// each fact from the paragraph or else the nearest style stating it. `None` where no
    /// numbering is named, where `<w:numId w:val="0"/>` removes it, and past OOXML's nine levels.
    pub fn list_level(&self, p: &ParagraphProperties) -> Option<u32> {
        let mut num_id = p.num_id.as_deref();
        let mut level = p.list_level;
        for style in self.chain(p) {
            num_id = num_id.or(style.declared.num_id.as_deref());
            level = level.or(style.declared.list_level);
        }
        let level = level.unwrap_or(0);
        (num_id.is_some_and(|id| id != "0") && level <= 8).then_some(level)
    }
}

/// `heading 1` … `heading 9`, ECMA-376's built-in names, to a level; any case, as Word reads them.
fn builtin_heading(name: &str) -> Option<u32> {
    let n: u32 = name
        .to_ascii_lowercase()
        .strip_prefix("heading ")?
        .parse()
        .ok()?;
    (1..=9).contains(&n).then_some(n)
}

/// Read a `word/styles.xml`'s paragraph styles (decision #136).
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, as for a body part.
pub fn read_styles(part: &[u8], part_name: &str) -> Result<Styles, EngineError> {
    let mut reader = crate::xml::new_reader(part, part_name)?;
    let mut styles = Styles::default();
    let mut open: Option<(String, Style)> = None;
    let mut depth: i32 = 0;
    let mut in_change: u32 = 0;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| crate::xml::parse_error(&reader, part_name, &e))?;
        let (start, is_start) = match &event {
            Event::Eof => break,
            Event::Start(start) => {
                depth += 1;
                (start, true)
            }
            Event::Empty(start) => (start, false),
            Event::End(end) => {
                depth -= 1;
                match local_name(end.name().as_ref()) {
                    b"style" => {
                        if let Some((id, style)) = open.take() {
                            styles.by_id.entry(id).or_insert(style);
                        }
                    }
                    name if name.ends_with(b"Change") => in_change = in_change.saturating_sub(1),
                    _ => {}
                }
                continue;
            }
            _ => continue,
        };
        let qualified = start.name();
        let name = local_name(qualified.as_ref());
        if name == b"style" && is_start {
            let kind = attribute(start, b"type", part_name)?;
            if kind.as_deref().is_none_or(|k| k == "paragraph") {
                let id = attribute(start, b"styleId", part_name)?.unwrap_or_default();
                if attribute(start, b"default", part_name)?.is_some_and(|d| d == "1" || d == "true")
                {
                    styles.default.get_or_insert_with(|| id.clone());
                }
                open = Some((id, Style::default()));
            }
            continue;
        }
        if is_start && name.ends_with(b"Change") {
            in_change += 1;
            continue;
        }
        if let (Some((_, style)), 0) = (open.as_mut(), in_change) {
            match name {
                b"name" => style.name = attribute(start, b"val", part_name)?,
                b"basedOn" => style.based_on = attribute(start, b"val", part_name)?,
                _ => paragraph_property(name, start, &mut style.declared, part_name)?,
            }
        }
    }
    crate::xml::check_closed(depth, part_name)?;
    Ok(styles)
}

/// Read the runs of a `word/document.xml`.
///
/// **One branch of an `<mc:AlternateContent>`, deterministically the first `<mc:Choice>`.** That
/// element holds one or more `<mc:Choice Requires="…">` and an optional `<mc:Fallback>`, all
/// expressing *the same content* for consumers of different capability — a text box, typically,
/// written once as a `<w:drawing>` and again as VML. Every `<w:t>` in the part was collected
/// until 2026-09-18, whatever its ancestry, so one phrase reached the artifact at two citable
/// addresses: duplicated evidence, the mirror of a silent drop, and the same defect `pptx.rs`
/// was written to avoid. The branches passed over are counted in
/// [`MainPart::alternatives_not_read`] and declared.
///
/// **The address counters advance through a skipped branch**, for the reason they do in
/// `pptx.rs`: a `DocxLocator`'s `paragraph` and `run` are positions in the part's own document
/// order, and a consumer checking one counts elements in the file, not the elements this reader
/// kept. Counting only what is read would make every address after an `<mc:AlternateContent>`
/// one short — a locator that is confidently wrong.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse. Malformed XML is refused rather than
/// read as far as it goes: a truncated body would be a shorter document that still looked whole,
/// which is the failure `docs/01-CONTRACT.md` §8 refuses for an unknown content-stream operator.
pub fn read_runs(part: &[u8]) -> Result<MainPart, EngineError> {
    read_part(part, MAIN_PART)
}

/// [`read_runs`] for any part that holds paragraphs — the body, a header, a footer, the footnotes
/// (decision #136) — named `part_name` in what it refuses.
///
/// # Errors
///
/// As [`read_runs`].
pub fn read_part(part: &[u8], part_name: &str) -> Result<MainPart, EngineError> {
    let text = std::str::from_utf8(part).map_err(|e| EngineError::Malformed {
        what: part_name.into(),
        detail: format!("the part is not UTF-8: {e}"),
    })?;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);

    let mut runs: Vec<Run> = Vec::new();
    // The `<w:p>` elements counted so far, and the open ones, innermost last, each with the runs
    // counted in it. A text box's paragraphs sit inside a run of the paragraph anchoring the box,
    // so a run after the box is counted in its own paragraph again, where one counter gave it the
    // box's (review of decision #130). A run outside every paragraph goes on in the last one.
    let mut paragraphs: u32 = 0;
    let mut open_paragraphs: Vec<(u32, u32)> = Vec::new();
    let mut last_paragraph: (u32, u32) = (0, 0);
    // Decision #136: what each open paragraph's `<w:pPr>` states, beside `open_paragraphs`; the
    // tables, references and tracked-change depth `Structure` keeps.
    let mut open_properties: Vec<ParagraphProperties> = Vec::new();
    let mut structure = Structure::default();
    // `Some` while inside a `<w:r>`: the run being built, and the depth of its `<w:r>`, so only
    // an element directly in it states a character. A text box's run opens inside the run
    // anchoring the box, which waits in `enclosing`, its text and its depth kept, and goes on
    // after the box (review of decision #130). Each run is put at `run_slot`, where its `<w:r>`
    // opened among the runs read: the anchoring run before the box's.
    let mut open_run: Option<Run> = None;
    let mut run_at: i32 = 0;
    let mut run_slot: usize = 0;
    let mut enclosing: Vec<(Run, i32, bool, usize)> = Vec::new();
    // `true` while inside a `<w:t>`, so text outside one is not collected.
    let mut in_text = false;
    // Whether the open run's `<w:t>` elements held any text. Since decision #130 the characters a
    // run states as an element make it a node too, unless the run is inside a `<w:del>`, whose
    // depth `deleted` is: see `push_stated`.
    let mut run_has_text = false;
    let mut deleted: u32 = 0;
    // Element depth, checked at EOF. `quick-xml` reports a *mismatched* close tag on its own but
    // reaching the end of input with elements still open is not an error to it — and a body that
    // stopped halfway is a shorter document that still looks whole, which is exactly what
    // `01-CONTRACT.md` §8 refuses.
    let mut depth: i32 = 0;
    // One flag per open `<mc:AlternateContent>`, the depth of the subtree being passed over, and
    // whether that subtree held any `<w:t>`.
    let mut alt_taken: Vec<bool> = Vec::new();
    let mut skip_from: Option<i32> = None;
    let mut skipped_text = false;
    let mut alternatives_not_read: u32 = 0;
    // Whether the subtree being passed over is a `<w:moveFrom>` rather than an alternative.
    let mut skipping_move = false;
    let mut moves_not_read: u32 = 0;
    // Whether it is a separator or a continuation notice (decision #136, review), counted nowhere.
    let mut skipping_note = false;

    loop {
        match reader.read_event() {
            Err(e) => {
                return Err(EngineError::Malformed {
                    what: part_name.into(),
                    detail: format!(
                        "XML will not parse at byte {}: {e}",
                        reader.buffer_position()
                    ),
                })
            }
            Ok(Event::Eof) => break,

            // A self-closing `<w:p/>` or `<w:r/>` is still an element in the file, and the
            // ordinals count elements in the file. Written `<w:p></w:p>` the same paragraph
            // counted, so without this a locator depended on how the XML was serialized.
            Ok(Event::Empty(empty)) => match local_name(empty.name().as_ref()) {
                b"p" => {
                    paragraphs += 1;
                    last_paragraph = (paragraphs, 0);
                }
                b"r" => {
                    next_run(&mut open_paragraphs, &mut last_paragraph);
                }
                // Taken, as `<mc:Choice></mc:Choice>` is, so a later branch is passed over.
                b"Choice" if skip_from.is_none() => {
                    if let Some(taken) = alt_taken.last_mut() {
                        *taken = true;
                    }
                }
                // Decision #136: and what it states about a paragraph, a table or a section.
                name if skip_from.is_none() => {
                    if depth == run_at {
                        push_stated(name, open_run.as_mut());
                    }
                    structure.open(name, false);
                    structure.element(name, &empty, open_properties.last_mut(), part_name)?;
                }
                _ => {}
            },

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
                            skipping_move = false;
                        }
                    },
                    b"Fallback" if skip_from.is_none() => {
                        skip_from = Some(depth);
                        skipped_text = false;
                        skipping_move = false;
                    }
                    // A tracked move's old place, ECMA-376's move source: passed over as an
                    // alternative is, and counted apart, so the moved text is read once, where
                    // `<w:moveTo>` puts it (review 2026-09-26 N18).
                    b"moveFrom" if skip_from.is_none() => {
                        skip_from = Some(depth);
                        skipped_text = false;
                        skipping_move = true;
                    }
                    // Decision #136 (review): a note Word sets as a separator line, or as the
                    // notice that a note runs on, is page layout and no note, so it is passed over
                    // by its `w:type` and counted nowhere — a page-break notice exists only where
                    // something is paginated, which nothing here is.
                    b"footnote" | b"endnote"
                        if skip_from.is_none()
                            && matches!(
                                attribute(&start, b"type", part_name)?.as_deref(),
                                Some("separator" | "continuationSeparator" | "continuationNotice")
                            ) =>
                    {
                        skip_from = Some(depth);
                        skipped_text = false;
                        skipping_move = false;
                        skipping_note = true;
                    }
                    _ => {}
                }

                let skipping = skip_from.is_some();
                match name {
                    b"del" => deleted += 1,
                    b"p" => {
                        paragraphs += 1;
                        open_paragraphs.push((paragraphs, 0));
                        open_properties.push(ParagraphProperties::default());
                    }
                    b"r" => {
                        let (paragraph, run) = next_run(&mut open_paragraphs, &mut last_paragraph);
                        if !skipping {
                            if let Some(anchor) = open_run.take() {
                                enclosing.push((anchor, run_at, run_has_text, run_slot));
                            }
                            run_has_text = false;
                            run_at = depth;
                            run_slot = runs.len();
                            open_run = Some(Run {
                                paragraph,
                                run,
                                text: String::new(),
                                space_preserved: false,
                                paragraph_properties: open_properties
                                    .last()
                                    .cloned()
                                    .unwrap_or_default(),
                                cell: structure.cell_at(),
                            });
                        }
                    }
                    b"t" => {
                        if skipping {
                            skipped_text = true;
                        } else {
                            in_text = true;
                            if let Some(run) = open_run.as_mut() {
                                if has_preserve(&start, part_name)? {
                                    run.space_preserved = true;
                                }
                            }
                        }
                    }
                    name if !skipping => {
                        if depth == run_at + 1 {
                            push_stated(name, open_run.as_mut());
                        }
                        structure.open(name, true);
                        structure.element(name, &start, open_properties.last_mut(), part_name)?;
                    }
                    _ => {}
                }
            }

            Ok(Event::End(end)) => {
                let closing = depth;
                depth -= 1;
                let qualified = end.name();
                let name = local_name(qualified.as_ref());
                if skip_from == Some(closing) {
                    skip_from = None;
                    let note = std::mem::take(&mut skipping_note);
                    if skipped_text && !note {
                        if skipping_move {
                            moves_not_read = crate::declare(moves_not_read, 1);
                        } else {
                            alternatives_not_read = crate::declare(alternatives_not_read, 1);
                        }
                    }
                    skipped_text = false;
                }
                if name == b"AlternateContent" {
                    alt_taken.pop();
                }
                match name {
                    b"t" => in_text = false,
                    b"del" => deleted = deleted.saturating_sub(1),
                    b"p" => {
                        if let Some(closed) = open_paragraphs.pop() {
                            last_paragraph = closed;
                        }
                        open_properties.pop();
                    }
                    // Only the open run's own `</w:r>`: a run passed over inside it, in a branch
                    // not read, closes no run.
                    b"r" if closing == run_at => {
                        if let Some(run) = open_run.take() {
                            // A run with no character at all — a drawing anchor, a footnote mark —
                            // carries no text to cite. It is not an erasure either: there was never
                            // a character in it. Skipped without a count, deliberately. A run that
                            // states only a tab or a break is a node (decision #130).
                            if run_has_text || (deleted == 0 && !run.text.is_empty()) {
                                runs.insert(run_slot, run);
                            }
                        }
                        if let Some((anchor, at, has_text, slot)) = enclosing.pop() {
                            (open_run, run_at, run_has_text, run_slot) =
                                (Some(anchor), at, has_text, slot);
                        }
                    }
                    name if skip_from.is_none() => structure.close(name),
                    _ => {}
                }
            }

            Ok(Event::Text(text)) if in_text => {
                if let Some(run) = open_run.as_mut() {
                    let decoded = crate::xml::decode(&text, part_name)?;
                    run.text.push_str(decoded.as_ref());
                    run_has_text |= !decoded.is_empty();
                }
            }
            // Matched, not ignored, as `pptx.rs` matches it: an unhandled `CData` arm is a silent
            // drop, and this reader had one — `<w:t>kept <![CDATA[and this]]></w:t>` read `kept `.
            Ok(Event::CData(cdata)) if in_text => {
                if let Some(run) = open_run.as_mut() {
                    let decoded = crate::xml::cdata_text(&cdata, part_name)?;
                    run.text.push_str(decoded.as_ref());
                    run_has_text |= !decoded.is_empty();
                }
            }
            // **`quick-xml` 0.41 delivers an entity as its own event**, so a reader that only
            // handled `Text` would drop `&amp;` silently and hand back `a  b` for `a &amp; b` —
            // measured, not feared: that is what the first version of this file did, and the
            // fixture's ampersand is in the corpus because of it.
            //
            // The rule itself moved to `xml.rs` at v2-S3, unchanged and message-identical,
            // because the workbook reader needs the same one and two copies of "what counts as
            // text" is two places for the answer to drift.
            Ok(Event::GeneralRef(entity)) if in_text => {
                let resolved = crate::xml::resolve_reference(entity.as_ref(), part_name)?;
                let resolved = resolved.as_ref();
                if let Some(run) = open_run.as_mut() {
                    run.text.push_str(resolved);
                    run_has_text |= !resolved.is_empty();
                }
            }
            Ok(_) => {}
        }
    }

    if depth != 0 {
        return Err(EngineError::Malformed {
            what: part_name.into(),
            detail: format!(
                "the part ends with {depth} element(s) still open; it is truncated, and a body \
                 read as far as it went would be a shorter document that still looked whole"
            ),
        });
    }
    // Each run into the cell it opened in, by its place among the runs read, now that every run
    // stands where it goes.
    let mut tables = structure.tables;
    for (index, run) in runs.iter().enumerate() {
        let Some((slot, row, cell)) = run.cell else {
            continue;
        };
        if let Some(cell) = tables
            .get_mut(slot)
            .and_then(Option::as_mut)
            .and_then(|rows| rows.get_mut(row))
            .and_then(|row| row.cells.get_mut(cell))
        {
            cell.runs.push(index);
        }
    }
    Ok(MainPart {
        runs,
        alternatives_not_read,
        moves_not_read,
        tables: tables.into_iter().flatten().filter_map(grid).collect(),
        references: structure.references,
    })
}

/// What [`read_part`] keeps beyond runs (decision #136): the tables open and read, the header and
/// footer references, and how deep it is in a tracked change's old properties.
#[derive(Default)]
struct Structure {
    /// One slot per `<w:tbl>`, in the order they open, its rows filled in when it closes.
    tables: Vec<Option<Vec<RawRow>>>,
    open_tables: Vec<OpenTable>,
    /// Inside a `<w:pPrChange>`, `<w:tcPrChange>` or any `…Change`: the properties a tracked change
    /// replaced, which are not the paragraph's or the cell's.
    in_change: u32,
    references: Vec<Reference>,
    /// Inside a `<w:sectPr>`: where its references start in `references`, and its `<w:titlePg>`.
    section: Option<(usize, bool)>,
}

/// A `<w:tbl>` being read: its slot in [`Structure::tables`], its rows, and whether its last cell
/// is still open.
struct OpenTable {
    slot: usize,
    rows: Vec<RawRow>,
    in_cell: bool,
}

#[derive(Default)]
struct RawRow {
    before: u32,
    cells: Vec<RawCell>,
}

#[derive(Default)]
struct RawCell {
    span: u32,
    /// `<w:vMerge>`: `Some(true)` restarts a merge, `Some(false)` continues the cell above.
    merge: Option<bool>,
    runs: Vec<usize>,
}

impl Structure {
    /// A table, a row or a cell opens — or, written self-closing, opens and closes.
    fn open(&mut self, name: &[u8], long: bool) {
        if self.in_change > 0 {
            if long && name.ends_with(b"Change") {
                self.in_change += 1;
            }
            return;
        }
        match name {
            b"tbl" if long => {
                self.open_tables.push(OpenTable {
                    slot: self.tables.len(),
                    rows: Vec::new(),
                    in_cell: false,
                });
                self.tables.push(None);
            }
            b"tr" => {
                if let Some(t) = self.open_tables.last_mut() {
                    t.rows.push(RawRow::default());
                }
            }
            b"tc" => {
                if let Some(t) = self.open_tables.last_mut() {
                    if let Some(row) = t.rows.last_mut() {
                        row.cells.push(RawCell::default());
                        t.in_cell = long;
                    }
                }
            }
            b"sectPr" if long => self.section = Some((self.references.len(), false)),
            name if long && name.ends_with(b"Change") => self.in_change += 1,
            _ => {}
        }
    }

    /// A table or a cell closes, or a tracked change's old properties end.
    fn close(&mut self, name: &[u8]) {
        if name.ends_with(b"Change") {
            self.in_change = self.in_change.saturating_sub(1);
            return;
        }
        if self.in_change > 0 {
            return;
        }
        match name {
            b"sectPr" => {
                if let Some((from, title_page)) = self.section.take() {
                    for r in &mut self.references[from..] {
                        r.title_page = title_page;
                    }
                }
            }
            b"tc" => {
                if let Some(t) = self.open_tables.last_mut() {
                    t.in_cell = false;
                }
            }
            b"tbl" => {
                if let Some(t) = self.open_tables.pop() {
                    self.tables[t.slot] = Some(t.rows);
                }
            }
            _ => {}
        }
    }

    /// A property this reader reads: a paragraph's style, outline level or numbering, a cell's span
    /// or merge, a row's skipped grid columns, a section's header or footer reference.
    fn element(
        &mut self,
        name: &[u8],
        start: &quick_xml::events::BytesStart<'_>,
        paragraph: Option<&mut ParagraphProperties>,
        part_name: &str,
    ) -> Result<(), EngineError> {
        if self.in_change > 0 {
            return Ok(());
        }
        match name {
            b"gridSpan" | b"vMerge" | b"gridBefore" => {
                let value = attribute(start, b"val", part_name)?;
                let Some(row) = self.open_tables.last_mut().and_then(|t| t.rows.last_mut()) else {
                    return Ok(());
                };
                match name {
                    b"gridBefore" => row.before = number(value.as_deref()).unwrap_or(0),
                    _ => {
                        if let Some(cell) = row.cells.last_mut() {
                            if name == b"gridSpan" {
                                cell.span = number(value.as_deref()).unwrap_or(1);
                            } else {
                                cell.merge = Some(value.as_deref() == Some("restart"));
                            }
                        }
                    }
                }
            }
            b"headerReference" | b"footerReference" => {
                if let Some(id) = attribute(start, b"id", part_name)? {
                    self.references.push(Reference {
                        footer: name == b"footerReference",
                        kind: attribute(start, b"type", part_name)?
                            .unwrap_or_else(|| "default".into()),
                        id,
                        title_page: false,
                    });
                }
            }
            b"titlePg" => {
                if let Some((_, title_page)) = self.section.as_mut() {
                    *title_page = stated_on(attribute(start, b"val", part_name)?.as_deref());
                }
            }
            _ => {
                if let Some(paragraph) = paragraph {
                    paragraph_property(name, start, paragraph, part_name)?;
                }
            }
        }
        Ok(())
    }

    /// The open cell of the innermost table, if one is open: (table slot, row, cell).
    fn cell_at(&self) -> Option<(usize, usize, usize)> {
        let t = self.open_tables.last().filter(|t| t.in_cell)?;
        let row = t.rows.len().checked_sub(1)?;
        let cell = t.rows[row].cells.len().checked_sub(1)?;
        Some((t.slot, row, cell))
    }
}

/// A table's grid from its rows: each row's cells laid from its `<w:gridBefore>` on, each
/// `<w:gridSpan>` wide, and a `<w:vMerge>` continuing the cell above extending that cell down a row.
/// `None` for a table with no cell. Nothing here allocates per slot, so a `<w:gridSpan>` of four
/// billion costs nothing; the caller's slot budget refuses such a grid, counted.
fn grid(rows: Vec<RawRow>) -> Option<Table> {
    let mut cells: Vec<Cell> = Vec::new();
    let mut columns: u32 = 0;
    // The cell reaching down to the row above at each column, so a continued merge finds its cell
    // in one lookup rather than a walk back over every cell (review: 8,000 rows took 11 s).
    let mut above: std::collections::BTreeMap<u32, usize> = Default::default();
    for (r, row) in rows.iter().enumerate() {
        let r = u32::try_from(r).ok()?;
        let mut column = row.before;
        let mut reaching = std::collections::BTreeMap::new();
        for raw in &row.cells {
            let span = raw.span.max(1);
            let at = match above.get(&column).filter(|_| raw.merge == Some(false)) {
                Some(&i) => {
                    cells[i].rowspan += 1;
                    cells[i].runs.extend(&raw.runs);
                    i
                }
                None => {
                    cells.push(Cell {
                        row: r,
                        column,
                        rowspan: 1,
                        colspan: span,
                        runs: raw.runs.clone(),
                    });
                    cells.len() - 1
                }
            };
            reaching.insert(column, at);
            column = column.saturating_add(span);
        }
        above = reaching;
        columns = columns.max(column);
    }
    let rows = u32::try_from(rows.len()).ok()?;
    (!cells.is_empty()).then_some(Table {
        rows,
        columns,
        cells,
    })
}

/// A paragraph property this reader reads into `paragraph`: `<w:pStyle>`, `<w:outlineLvl>`,
/// `<w:numId>`, `<w:ilvl>`. Any other element states none of them.
fn paragraph_property(
    name: &[u8],
    start: &quick_xml::events::BytesStart<'_>,
    paragraph: &mut ParagraphProperties,
    part_name: &str,
) -> Result<(), EngineError> {
    match name {
        b"pStyle" => paragraph.style = attribute(start, b"val", part_name)?,
        b"outlineLvl" => {
            paragraph.outline_level = number(attribute(start, b"val", part_name)?.as_deref())
        }
        b"numId" => paragraph.num_id = attribute(start, b"val", part_name)?,
        b"ilvl" => paragraph.list_level = number(attribute(start, b"val", part_name)?.as_deref()),
        _ => {}
    }
    Ok(())
}

/// A decimal attribute value; `None` for anything else, which states no number.
fn number(value: Option<&str>) -> Option<u32> {
    value?.trim().parse().ok()
}

/// The value of the attribute whose local name is `want`, unescaped.
fn attribute(
    start: &quick_xml::events::BytesStart<'_>,
    want: &[u8],
    part_name: &str,
) -> Result<Option<String>, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        if local_name(attribute.key.as_ref()) == want {
            return Ok(Some(crate::xml::attribute_value(&attribute, part_name)?));
        }
    }
    Ok(None)
}

use crate::xml::local_name;

/// Count one more `<w:r>` in the paragraph it belongs to — the innermost open one, or the last
/// one where none is open — and return its address.
fn next_run(open: &mut [(u32, u32)], last: &mut (u32, u32)) -> (u32, u32) {
    let at = open.last_mut().unwrap_or(last);
    at.1 += 1;
    *at
}

/// Append the character a run-content element states, if it states one (ECMA-376 §17.3.3).
///
/// `<w:tab/>` is a tab, `<w:br/>` and `<w:cr/>` a line feed, `<w:noBreakHyphen/>` U+2011 — the
/// characters `odt.rs` reads from `<text:tab/>` and `<text:line-break/>` and `rtf.rs` from `\tab`,
/// `\line` and `\_`. Without them the text on either side is welded into one word. Since the
/// review of decision #130, `<w:ptab/>`, the positional tab Word sets a right-aligned date with, is
/// a tab too, and **only an element directly in the `<w:r>` states one**: a text box's
/// `<w:tabs><w:tab/></w:tabs>`, inside a run of the paragraph anchoring it, defines a tab stop and
/// holds no character.
///
/// **They make a run a node on their own since decision #130**, so a label's tab or a line break
/// Word writes as a run of its own reaches the paragraph — **but never inside `<w:del>`.** A run
/// there keeps its text in `<w:delText>`, which is not read, but states its tabs, breaks and
/// hyphens with these same elements, so counting it a node would put a deleted hyphen in the
/// evidence.
fn push_stated(name: &[u8], run: Option<&mut Run>) {
    let stated = match name {
        b"tab" | b"ptab" => '\t',
        b"br" | b"cr" => '\n',
        b"noBreakHyphen" => '\u{2011}',
        _ => return,
    };
    if let Some(run) = run {
        run.text.push(stated);
    }
}

fn has_preserve(
    start: &quick_xml::events::BytesStart<'_>,
    part_name: &str,
) -> Result<bool, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        if local_name(attribute.key.as_ref()) == b"space" && attribute.value.as_ref() == b"preserve"
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A tracked move's text is read once, where it now stands** (review 2026-09-26 N18).
    /// `<w:moveFrom>` holds the old place, which Word hides and accepting the changes deletes; read
    /// as current text, `Moved sentence.` stood twice. The passed-over region is counted.
    #[test]
    fn a_moved_sentence_is_read_where_it_now_stands() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
              <w:p><w:moveFrom w:id="1"><w:r><w:t>Moved sentence.</w:t></w:r></w:moveFrom><w:r><w:t xml:space="preserve"> Stays.</w:t></w:r></w:p>
              <w:p><w:moveTo w:id="2"><w:r><w:t>Moved sentence.</w:t></w:r></w:moveTo></w:p>
            </w:body></w:document>"#;
        let main = read_runs(xml.as_bytes()).expect("well-formed");
        let read: Vec<(u32, u32, &str)> = main
            .runs
            .iter()
            .map(|r| (r.paragraph, r.run, r.text.as_str()))
            .collect();
        assert_eq!(read, vec![(1, 2, " Stays."), (2, 1, "Moved sentence.")]);
        assert_eq!(main.moves_not_read, 1);
        assert_eq!(main.alternatives_not_read, 0);
    }

    /// **A CDATA section in `<w:t>` is text** (found building the 2026-09-26 office fixes): the
    /// reader had no arm for it, so `kept <![CDATA[and this]]>` read `kept ` and declared nothing.
    #[test]
    fn a_cdata_section_in_a_text_element_is_read() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
              <w:p><w:r><w:t>kept <![CDATA[and this]]></w:t></w:r></w:p>
              <w:p><w:r><w:t><![CDATA[alone]]></w:t></w:r></w:p>
            </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        let texts: Vec<&str> = runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, vec!["kept and this", "alone"]);
    }

    #[test]
    fn runs_carry_the_positions_the_part_states() {
        let xml = r#"<?xml version="1.0"?>
            <w:document xmlns:w="x"><w:body>
              <w:p><w:r><w:t>First</w:t></w:r><w:r><w:t xml:space="preserve"> second</w:t></w:r></w:p>
              <w:p><w:r><w:t>Third</w:t></w:r></w:p>
            </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(runs.len(), 3);
        assert_eq!(
            (runs[0].paragraph, runs[0].run, runs[0].text.as_str()),
            (1, 1, "First")
        );
        assert_eq!(
            (runs[1].paragraph, runs[1].run, runs[1].text.as_str()),
            (1, 2, " second")
        );
        assert_eq!(
            (runs[2].paragraph, runs[2].run, runs[2].text.as_str()),
            (2, 1, "Third")
        );
        assert!(!runs[0].space_preserved);
        assert!(
            runs[1].space_preserved,
            "the attribute is read, not guessed"
        );
    }

    /// **A self-closing `<w:p/>` or `<w:r/>` counts.** Only a start tag advanced the ordinals, so
    /// an empty paragraph or run written short moved every later address, and two serializations
    /// of one document cited the same text at different places.
    #[test]
    fn a_self_closing_paragraph_or_run_still_counts() {
        let long = r#"<w:document xmlns:w="x"><w:body><w:p></w:p>
            <w:p><w:r></w:r><w:r><w:t>kept</w:t></w:r></w:p></w:body></w:document>"#;
        let short = r#"<w:document xmlns:w="x"><w:body><w:p/>
            <w:p><w:r/><w:r><w:t>kept</w:t></w:r></w:p></w:body></w:document>"#;
        let addresses = |xml: &str| {
            read_runs(xml.as_bytes())
                .expect("well-formed")
                .runs
                .iter()
                .map(|r| (r.text.clone(), r.paragraph, r.run))
                .collect::<Vec<_>>()
        };
        assert_eq!(addresses(long), [("kept".to_string(), 2, 2)]);
        assert_eq!(addresses(short), addresses(long));
    }

    #[test]
    fn a_run_with_no_text_is_not_a_node() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:footnoteReference w:id="1"/></w:r><w:r><w:t>only this</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "only this");
        assert_eq!(
            runs[0].run, 2,
            "the empty run still counted for the address"
        );
    }

    /// **A run holding only a tab or a break is a node** (decision #130). Word writes a label's tab
    /// and a line break as runs of their own, and dropping them left `Company Name` and
    /// `: Acme Ltd` with nothing between them. A deleted one is still not a node.
    #[test]
    fn a_run_holding_only_a_tab_or_a_break_is_a_node() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:t>Company Name</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>: Acme Ltd</w:t></w:r></w:p>
            <w:p><w:r><w:t>one</w:t></w:r><w:r><w:br/></w:r><w:del><w:r><w:tab/></w:r></w:del><w:r><w:t>two</w:t></w:r></w:p>
            <w:p><w:del><w:r><w:delText>x</w:delText></w:r></w:del><w:r><w:tab/></w:r><w:r><w:ptab w:alignment="right"/></w:r></w:p>
        </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(
            runs.iter()
                .map(|r| (r.paragraph, r.run, r.text.as_str()))
                .collect::<Vec<_>>(),
            [
                (1, 1, "Company Name"),
                (1, 2, "\t"),
                (1, 3, ": Acme Ltd"),
                (2, 1, "one"),
                (2, 2, "\n"),
                (2, 4, "two"),
                (3, 2, "\t"),
                (3, 3, "\t")
            ],
            "a tab after a deletion is read, and a positional tab is a tab"
        );
    }

    /// **A run after a text box is its own paragraph's** (review of decision #130). A text box's
    /// `<w:p>` sits inside a run of the paragraph that anchors it, and the counter had no stack,
    /// so `After` took the box's paragraph and its run count: a locator naming the wrong `<w:p>`.
    /// Drawn as DrawingML and as VML alike.
    #[test]
    fn a_run_after_a_text_box_is_its_own_paragraphs() {
        for anchor in [
            "<w:drawing><wps:txbx><w:txbxContent><w:p><w:r><w:t>Box</w:t></w:r></w:p></w:txbxContent></wps:txbx></w:drawing>",
            "<w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>Box</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict>",
        ] {
            let xml = format!(
                r#"<w:document xmlns:w="x" xmlns:wps="s" xmlns:v="v"><w:body>
                <w:p><w:r><w:t>Before</w:t></w:r><w:r>{anchor}</w:r><w:r><w:t>After</w:t></w:r></w:p>
                <w:p><w:r><w:t>Next</w:t></w:r></w:p></w:body></w:document>"#
            );
            let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
            assert_eq!(
                runs.iter()
                    .map(|r| (r.paragraph, r.run, r.text.as_str()))
                    .collect::<Vec<_>>(),
                [
                    (1, 1, "Before"),
                    (2, 1, "Box"),
                    (1, 3, "After"),
                    (3, 1, "Next")
                ],
                "{anchor}"
            );
        }
    }

    /// **Text a run holds around the text box it anchors is kept** (review of decision #130). The
    /// box's run, opened inside the anchoring one, took its place: `Before ` was dropped, and
    /// `After`, read with no run open, was never read. The anchoring run is one node at its own
    /// address, holding what it states in order, where its `<w:r>` opens, before the box's runs.
    /// A box written twice keeps it open past the branch passed over.
    #[test]
    fn text_around_a_text_box_in_one_run_is_kept() {
        let drawing = "<w:drawing><wps:txbx><w:txbxContent><w:p><w:r><w:t>Box</w:t></w:r></w:p></w:txbxContent></wps:txbx></w:drawing>";
        let alternate = format!(
            "<mc:AlternateContent><mc:Choice Requires=\"wps\">{drawing}</mc:Choice><mc:Fallback><w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>Box</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></mc:Fallback></mc:AlternateContent>"
        );
        for (anchor, next) in [(drawing.to_string(), 3), (alternate, 4)] {
            let xml = format!(
                r#"<w:document xmlns:w="x" xmlns:wps="s" xmlns:v="v" xmlns:mc="mc"><w:body>
                <w:p><w:r><w:t xml:space="preserve">Before </w:t>{anchor}<w:br/><w:t>After</w:t></w:r><w:r><w:t>Tail</w:t></w:r></w:p>
                <w:p><w:r><w:t>Next</w:t></w:r></w:p></w:body></w:document>"#
            );
            let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
            assert_eq!(
                runs.iter()
                    .map(|r| (r.paragraph, r.run, r.text.as_str()))
                    .collect::<Vec<_>>(),
                [
                    (1, 1, "Before \nAfter"),
                    (2, 1, "Box"),
                    (1, 2, "Tail"),
                    (next, 1, "Next")
                ],
                "{anchor}"
            );
        }
    }

    /// **A tab stop is no tab.** A text box's `<w:tabs><w:tab/></w:tabs>` defines where its tabs
    /// stop, inside a run of the anchoring paragraph, and read as content it was a `\t` the file
    /// does not hold. Only an element directly in `<w:r>` states a character.
    #[test]
    fn a_tab_stop_inside_a_run_states_nothing() {
        let xml = r#"<w:document xmlns:w="x" xmlns:wps="s"><w:body>
            <w:p><w:r><w:t>Before</w:t></w:r><w:r><w:drawing><wps:txbx><w:txbxContent><w:p><w:pPr><w:tabs><w:tab w:val="left" w:pos="720"/></w:tabs></w:pPr></w:p></w:txbxContent></wps:txbx></w:drawing></w:r><w:r><w:t>After</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(
            runs.iter()
                .map(|r| (r.paragraph, r.run, r.text.as_str()))
                .collect::<Vec<_>>(),
            [(1, 1, "Before"), (1, 3, "After")]
        );
    }

    /// **A tab or a break a run states as an element is a character**, so the text on either
    /// side is not welded: `textutil`'s `Line one<br>Line two` read as `Line oneLine two`. A run
    /// holding only a break is a node since decision #130; a deleted run holding a no-break hyphen
    /// is not, and a passed-over branch states nothing.
    #[test]
    fn stated_tabs_and_breaks_separate_the_text_they_sit_between() {
        let xml = r#"<w:document xmlns:w="x" xmlns:mc="mc"><w:body>
            <w:p><w:r><w:t>Line one</w:t><w:br/><w:t>Line two</w:t></w:r></w:p>
            <w:p><w:r><w:t>Name:</w:t><w:tab></w:tab><w:t>Value</w:t></w:r><w:r><w:br/></w:r></w:p>
            <w:p><w:r><w:t>A</w:t><w:cr/><w:t>B</w:t><w:noBreakHyphen/><w:t>C</w:t></w:r>
              <w:del><w:r><w:delText>x</w:delText><w:noBreakHyphen/></w:r></w:del></w:p>
            <w:p><w:r><w:t>kept</w:t><mc:AlternateContent><mc:Choice Requires="a"></mc:Choice>
              <mc:Fallback><w:tab/><w:br></w:br></mc:Fallback></mc:AlternateContent></w:r></w:p>
        </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            [
                "Line one\nLine two",
                "Name:\tValue",
                "\n",
                "A\nB\u{2011}C",
                "kept"
            ]
        );
    }

    /// **One `<mc:AlternateContent>` is one phrase, read once** (`docs/OPEN-WORK.md` §6,
    /// confirmed and fixed 2026-09-18). A text box is written twice — a `<w:drawing>` under
    /// `<mc:Choice>` and VML under `<mc:Fallback>` — and every `<w:t>` in the part used to be
    /// collected whatever its ancestry, so the phrase reached the artifact at two citable
    /// addresses. The first `<mc:Choice>` is read and the rest are counted.
    #[test]
    fn an_alternate_content_is_read_once_and_the_rest_counted() {
        let xml = r#"<w:document xmlns:w="x" xmlns:mc="mc"><w:body>
            <w:p><w:r><w:t>before</w:t></w:r></w:p>
            <mc:AlternateContent>
              <mc:Choice Requires="wps"><w:p><w:r><w:t>in a text box</w:t></w:r></w:p></mc:Choice>
              <mc:Fallback><w:p><w:r><w:t>in a text box</w:t></w:r></w:p></mc:Fallback>
            </mc:AlternateContent>
            <w:p><w:r><w:t>after</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let out = read_runs(xml.as_bytes()).expect("well-formed");
        let seen: Vec<_> = out
            .runs
            .iter()
            .map(|r| (r.text.as_str(), r.paragraph, r.run))
            .collect();
        assert_eq!(
            seen,
            vec![("before", 1, 1), ("in a text box", 2, 1), ("after", 4, 1)],
            "the phrase once, and `after` numbered as the file numbers it — the fourth `<w:p>`"
        );
        assert_eq!(
            out.alternatives_not_read, 1,
            "the fallback held text and was passed over"
        );
    }

    /// **A branch that holds no text is not an erasure.** Most of them wrap a drawing's geometry,
    /// and declaring those would bury a real one under noise (`pptx.rs` makes the same point).
    #[test]
    fn an_alternate_branch_without_text_is_not_counted() {
        let xml = r#"<w:document xmlns:w="x" xmlns:mc="mc"><w:body>
            <mc:AlternateContent>
              <mc:Choice Requires="wps"><w:p><w:r><w:t>kept</w:t></w:r></w:p></mc:Choice>
              <mc:Fallback><w:p><w:r><w:drawing/></w:r></w:p></mc:Fallback>
            </mc:AlternateContent>
        </w:body></w:document>"#;
        let out = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(out.runs.len(), 1);
        assert_eq!(out.runs[0].text, "kept");
        assert_eq!(out.alternatives_not_read, 0);
    }

    /// **A second `<mc:Choice>` is passed over too**, and two skipped branches count two.
    #[test]
    fn only_the_first_choice_is_read() {
        let xml = r#"<w:document xmlns:w="x" xmlns:mc="mc"><w:body>
            <mc:AlternateContent>
              <mc:Choice Requires="a"><w:p><w:r><w:t>first</w:t></w:r></w:p></mc:Choice>
              <mc:Choice Requires="b"><w:p><w:r><w:t>second</w:t></w:r></w:p></mc:Choice>
              <mc:Fallback><w:p><w:r><w:t>third</w:t></w:r></w:p></mc:Fallback>
            </mc:AlternateContent>
        </w:body></w:document>"#;
        let out = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(
            out.runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            vec!["first"]
        );
        assert_eq!(out.alternatives_not_read, 2);
    }

    /// **A self-closing `<mc:Choice/>` is taken, as the same element written long is.** Only a
    /// start tag marked a branch taken, so after `<mc:Choice/>` the next Choice was read as well,
    /// and two serializations of one document said different things.
    #[test]
    fn a_self_closing_choice_is_taken_like_a_long_one() {
        let long = r#"<w:document xmlns:w="x" xmlns:mc="mc"><w:body><mc:AlternateContent>
            <mc:Choice Requires="a"></mc:Choice>
            <mc:Choice Requires="b"><w:p><w:r><w:t>second</w:t></w:r></w:p></mc:Choice>
        </mc:AlternateContent></w:body></w:document>"#;
        let short = long.replace("\"a\"></mc:Choice>", "\"a\"/>");
        let read = |xml: &str| {
            let out = read_runs(xml.as_bytes()).expect("well-formed");
            (out.runs.len(), out.alternatives_not_read)
        };
        assert_eq!(read(long), (0, 1), "the second Choice is passed over");
        assert_eq!(read(&short), read(long));
    }

    #[test]
    fn entities_are_decoded_rather_than_carried_as_source() {
        let xml = r#"<w:document xmlns:w="x"><w:body><w:p><w:r>
            <w:t>a &amp; b &lt; c</w:t></w:r></w:p></w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(
            runs[0].text, "a & b < c",
            "this is why XML is not hand-rolled"
        );
    }

    /// **A line end in a `<w:t>` is the LF an XML parser delivers**, not the file's own CRLF or CR.
    #[test]
    fn a_crlf_or_a_lone_cr_in_run_text_is_one_line_feed() {
        let xml = "<w:document xmlns:w=\"x\"><w:body><w:p><w:r>\
            <w:t xml:space=\"preserve\">a\r\nb\rc</w:t></w:r></w:p></w:body></w:document>";
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        assert_eq!(runs[0].text, "a\nb\nc");
    }

    #[test]
    fn malformed_xml_is_refused_rather_than_read_as_far_as_it_goes() {
        let xml = "<w:document><w:body><w:p><w:r><w:t>truncated";
        assert!(read_runs(xml.as_bytes()).is_err());
    }

    /// **A paragraph's style, outline level and numbering are read off its own `<w:pPr>`**
    /// (decision #136), a text box's paragraph keeping its own, and never off a `<w:pPrChange>`,
    /// which holds what a tracked change replaced.
    #[test]
    fn a_paragraphs_declared_properties_are_read_off_its_own_ppr() {
        let xml = r#"<w:document xmlns:w="x" xmlns:wps="s"><w:body>
            <w:p><w:pPr><w:pStyle w:val="Heading2"/><w:pPrChange><w:pPr><w:pStyle w:val="Old"/><w:numPr><w:ilvl w:val="3"/><w:numId w:val="9"/></w:numPr></w:pPr></w:pPrChange></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>
            <w:p><w:pPr><w:outlineLvl w:val="1"/><w:numPr><w:ilvl w:val="2"/><w:numId w:val="4"/></w:numPr></w:pPr><w:r><w:t>Item</w:t></w:r><w:r><w:drawing><wps:txbx><w:txbxContent><w:p><w:pPr><w:pStyle w:val="Box"/></w:pPr><w:r><w:t>In box</w:t></w:r></w:p></w:txbxContent></wps:txbx></w:drawing></w:r><w:r><w:t>After</w:t></w:r></w:p>
            <w:p><w:r><w:t>Plain</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed").runs;
        let props = |style: Option<&str>, outline, num: Option<&str>, level| ParagraphProperties {
            style: style.map(str::to_string),
            outline_level: outline,
            num_id: num.map(str::to_string),
            list_level: level,
        };
        let item = props(None, Some(1), Some("4"), Some(2));
        assert_eq!(
            runs.iter()
                .map(|r| (r.text.as_str(), r.paragraph_properties.clone()))
                .collect::<Vec<_>>(),
            [
                ("Title", props(Some("Heading2"), None, None, None)),
                ("Item", item.clone()),
                ("In box", props(Some("Box"), None, None, None)),
                ("After", item),
                ("Plain", ParagraphProperties::default()),
            ]
        );
    }

    /// **A level is read through the style that defines it** (decision #136): its own
    /// `<w:outlineLvl>`, its `<w:basedOn>` parent's, or a built-in name `heading N`; never the
    /// styleId alone, and never where the level stated is body text. Numbering comes from the
    /// paragraph or its style, and `numId 0` removes it.
    #[test]
    fn a_level_is_read_through_the_style_that_defines_it() {
        let styles = read_styles(
            br#"<w:styles xmlns:w="x">
              <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style>
              <w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style>
              <w:style w:type="paragraph" w:styleId="Mine"><w:name w:val="Mine"/><w:basedOn w:val="Heading1"/></w:style>
              <w:style w:type="paragraph" w:styleId="H3"><w:name w:val="Heading 3"/></w:style>
              <w:style w:type="paragraph" w:styleId="Heading4"><w:name w:val="Not a heading"/></w:style>
              <w:style w:type="paragraph" w:styleId="Body"><w:name w:val="heading 2"/><w:pPr><w:outlineLvl w:val="9"/></w:pPr></w:style>
              <w:style w:type="paragraph" w:styleId="Changed"><w:name w:val="Changed"/><w:pPr><w:pPrChange><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:pPrChange></w:pPr></w:style>
              <w:style w:type="paragraph" w:styleId="Bullet"><w:name w:val="List Bullet"/><w:pPr><w:numPr><w:numId w:val="7"/></w:numPr></w:pPr></w:style>
              <w:style w:type="paragraph" w:styleId="Loop"><w:name w:val="Loop"/><w:basedOn w:val="Loop"/></w:style>
              <w:style w:type="character" w:styleId="Heading5"><w:name w:val="heading 5"/></w:style>
            </w:styles>"#,
            "word/styles.xml",
        )
        .expect("well-formed");
        let p = |style: &str| ParagraphProperties {
            style: (!style.is_empty()).then(|| style.to_string()),
            ..ParagraphProperties::default()
        };
        let levels: Vec<Option<u32>> = [
            "Heading1", "Mine", "H3", "Heading4", "Body", "Changed", "Loop", "Heading5", "",
        ]
        .iter()
        .map(|s| styles.heading_level(&p(s)))
        .collect();
        assert_eq!(
            levels,
            [
                Some(1),
                Some(1),
                Some(3),
                None,
                None,
                None,
                None,
                None,
                None
            ]
        );
        let own = ParagraphProperties {
            outline_level: Some(2),
            ..p("Heading1")
        };
        assert_eq!(styles.heading_level(&own), Some(3), "its own level first");

        assert_eq!(
            styles.list_level(&p("Bullet")),
            Some(0),
            "the style's numbering"
        );
        let deeper = ParagraphProperties {
            list_level: Some(1),
            ..p("Bullet")
        };
        assert_eq!(styles.list_level(&deeper), Some(1));
        let removed = ParagraphProperties {
            num_id: Some("0".into()),
            ..p("Bullet")
        };
        assert_eq!(styles.list_level(&removed), None, "numId 0 removes it");
        let past = ParagraphProperties {
            num_id: Some("3".into()),
            list_level: Some(9),
            ..p("")
        };
        assert_eq!(styles.list_level(&past), None, "past OOXML's nine levels");
        assert_eq!(styles.list_level(&p("Heading1")), None);
    }

    /// **A table is the grid the file states** (decision #136, `docx-tbl-v1`): a row per `<w:tr>`,
    /// a cell per `<w:tc>` laid from `<w:gridBefore>` and `<w:gridSpan>` wide, a `<w:vMerge>`
    /// continuing the cell above extending it, and a nested table's runs that table's own.
    #[test]
    fn a_table_is_the_grid_the_file_states() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:t>Before</w:t></w:r></w:p>
            <w:tbl>
              <w:tr><w:tc><w:tcPr><w:gridSpan w:val="2"/></w:tcPr><w:p><w:r><w:t>Wide</w:t></w:r></w:p></w:tc><w:tc><w:tcPr><w:vMerge w:val="restart"/><w:tcPrChange><w:tcPr><w:gridSpan w:val="5"/></w:tcPr></w:tcPrChange></w:tcPr><w:p><w:r><w:t>Tall</w:t></w:r></w:p></w:tc></w:tr>
              <w:tr><w:trPr><w:gridBefore w:val="1"/></w:trPr><w:tc><w:p><w:r><w:t>B</w:t></w:r><w:r><w:t>2</w:t></w:r></w:p>
                <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Inner</w:t></w:r></w:p></w:tc></w:tr></w:tbl><w:p><w:r><w:t>Tail</w:t></w:r></w:p></w:tc>
                <w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc></w:tr>
            </w:tbl>
            <w:p><w:r><w:t>After</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let main = read_runs(xml.as_bytes()).expect("well-formed");
        let texts: Vec<&str> = main.runs.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(
            texts,
            ["Before", "Wide", "Tall", "B", "2", "Inner", "Tail", "After"]
        );
        let cell = |row, column, rowspan, colspan, runs: &[usize]| Cell {
            row,
            column,
            rowspan,
            colspan,
            runs: runs.to_vec(),
        };
        assert_eq!(
            main.tables,
            [
                Table {
                    rows: 2,
                    columns: 3,
                    cells: vec![
                        cell(0, 0, 1, 2, &[1]),
                        cell(0, 2, 2, 1, &[2]),
                        cell(1, 1, 1, 1, &[3, 4, 6]),
                    ],
                },
                Table {
                    rows: 1,
                    columns: 1,
                    cells: vec![cell(0, 0, 1, 1, &[5])],
                },
            ],
            "in the order they open"
        );
    }

    /// **A grid is the file's, however wide, and a table with no cell is none**: nothing here
    /// allocates per slot, and the document's slot budget is the caller's, counted there.
    #[test]
    fn a_grid_is_kept_as_stated_and_a_table_with_no_cell_is_none() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:tbl><w:tr><w:tc><w:tcPr><w:gridSpan w:val="4000000000"/></w:tcPr><w:p><w:r><w:t>x</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
            <w:tbl><w:tr></w:tr></w:tbl>
        </w:body></w:document>"#;
        let main = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(main.runs.len(), 1);
        assert_eq!(
            main.tables
                .iter()
                .map(|t| (t.rows, t.columns))
                .collect::<Vec<_>>(),
            [(1, 4_000_000_000)]
        );
    }

    /// **A section's header and footer references are kept** (decision #136), in document order,
    /// self-closing or long, with their type and whether their section states `<w:titlePg/>`, and
    /// not out of a passed-over branch.
    #[test]
    fn header_and_footer_references_are_kept_in_order() {
        let xml = r#"<w:document xmlns:w="x" xmlns:r="r" xmlns:mc="mc"><w:body>
            <w:p><w:pPr><w:sectPr><w:headerReference w:type="first" r:id="rH1"/><w:titlePg/></w:sectPr></w:pPr></w:p>
            <mc:AlternateContent><mc:Choice Requires="a"/><mc:Fallback><w:sectPr><w:headerReference r:id="rSkip"/></w:sectPr></mc:Fallback></mc:AlternateContent>
            <w:sectPr><w:headerReference w:type="default" r:id="rH2"></w:headerReference><w:footerReference r:id="rF1"/><w:titlePg w:val="0"/></w:sectPr>
        </w:body></w:document>"#;
        let main = read_runs(xml.as_bytes()).expect("well-formed");
        let reference = |footer, kind: &str, id: &str, title_page| Reference {
            footer,
            kind: kind.into(),
            id: id.into(),
            title_page,
        };
        assert_eq!(
            main.references,
            [
                reference(false, "first", "rH1", true),
                reference(false, "default", "rH2", false),
                reference(true, "default", "rF1", false),
            ]
        );
        assert!(even_and_odd_headers(
            br#"<w:settings xmlns:w="x"><w:evenAndOddHeaders/></w:settings>"#,
            "s"
        )
        .unwrap());
        assert!(!even_and_odd_headers(
            br#"<w:settings xmlns:w="x"><w:evenAndOddHeaders w:val="false"/></w:settings>"#,
            "s"
        )
        .unwrap());
        assert!(!even_and_odd_headers(br#"<w:settings xmlns:w="x"/>"#, "s").unwrap());
    }

    /// **A run holding a text box goes on in its cell, and the box's runs in theirs** (review of
    /// decision #136 after C3): the anchoring run is put before the box's, which moved every run
    /// recorded by its place, so a cell named the wrong runs.
    #[test]
    fn a_cell_names_its_own_runs_around_a_text_box() {
        let boxed = |inner: &str| {
            format!(
                "<w:drawing><wps:txbx><w:txbxContent>{inner}</w:txbxContent></wps:txbx></w:drawing>"
            )
        };
        let xml = format!(
            r#"<w:document xmlns:w="x" xmlns:wps="s"><w:body>
            <w:tbl><w:tr><w:tc><w:p><w:r><w:t xml:space="preserve">Before </w:t>{}<w:t>After</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
            <w:p><w:r><w:t xml:space="preserve">Anchor </w:t>{}<w:t>tail</w:t></w:r></w:p>
        </w:body></w:document>"#,
            boxed("<w:p><w:r><w:t>Box</w:t></w:r></w:p>"),
            boxed(
                "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Inner</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"
            ),
        );
        let main = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(
            main.runs
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>(),
            ["Before After", "Box", "Anchor tail", "Inner"]
        );
        assert_eq!(
            main.tables
                .iter()
                .map(|t| t.cells.iter().map(|c| c.runs.clone()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            [vec![vec![0, 1]], vec![vec![3]]],
            "the outer cell holds its run and its box's, the boxed table its own run"
        );
    }

    /// **A separator or a continuation notice is no note** (review of decision #136): passed over
    /// by its `w:type`, and counted nowhere, where `(continued)` read as a footnote.
    #[test]
    fn a_separator_or_a_continuation_notice_is_no_note() {
        let xml = r#"<w:footnotes xmlns:w="x">
            <w:footnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:footnote>
            <w:footnote w:type="continuationNotice" w:id="5"><w:p><w:r><w:t>(continued)</w:t></w:r></w:p></w:footnote>
            <w:footnote w:id="1"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:t>First note.</w:t></w:r></w:p></w:footnote>
            <mc:AlternateContent xmlns:mc="mc"><mc:Choice Requires="a"/><mc:Fallback><w:p><w:r><w:t>alt</w:t></w:r></w:p></mc:Fallback></mc:AlternateContent>
        </w:footnotes>"#;
        let part = read_part(xml.as_bytes(), "word/footnotes.xml").expect("well-formed");
        assert_eq!(
            part.runs
                .iter()
                .map(|r| (r.paragraph, r.text.as_str()))
                .collect::<Vec<_>>(),
            [(3, "First note.")],
            "the notice's paragraph still counts in the addresses"
        );
        assert_eq!(part.alternatives_not_read, 1, "a later branch still counts");
    }

    #[test]
    fn unread_text_parts_are_counted_by_prefix() {
        let names: Vec<String> = [
            "word/document.xml",
            "word/header1.xml",
            "word/header2.xml",
            "word/footer1.xml",
            "word/styles.xml",
            "[Content_Types].xml",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // `styles.xml` carries no text a citation could land in, so it is not an erasure.
        assert_eq!(unread_text_parts(&names), 3);
    }
}
