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

//! **A document's items, each with its box and a label the record states** (decision #137).
//!
//! One flat list for a consumer that wants the page's pieces — text, section headers, running heads
//! and folios, tables, pictures and form fields — without reading the representation's nodes,
//! tables and figures itself. It began as the view the ParseBench adapter built in Python (`_layout` in
//! `docs/measurements/parsebench/ethos_bench.py`, decisions #38, #41, #46, #62, #97), and departs
//! from it where the adapter is wrong for a consumer; each departure is named below.
//! **It reads no rule of its own about what a piece of text is**:
//!
//! - A **text unit** is a block of [`crate::markdown::geometric_blocks`] — the element
//!   `ethos.grounding.v1` cites — and consecutive blocks whose runs share a page and a
//!   [`crate::TextRunAttributes::layout_unit`] are one unit, boxed by their union and joined by one
//!   space. A unit's box and label, and the units a picture's text is read from, come from the
//!   runs with a measured box — the spans `ground` cites — and on a source with no pages from every
//!   run; a unit's text is its blocks' text, as `ground`'s element states it, so a space the page
//!   drew as a run of its own, with no ink to measure, stays between its words. **A DOCX paragraph's
//!   runs are one unit** (decision #130), its text as both projections write it
//!   ([`crate::markdown::projected_text`], [`crate::markdown::paragraph_seam`]): a whitespace run
//!   joins its paragraph and never stands alone (the adapter, reading `ground`, had one element
//!   per run).
//! - Its **label** is the first the record states: a run's page `furniture` (`page-header`,
//!   `page-footer`), a Word header's or footer's included; then a level a DOCX, ODF or EPUB file
//!   declares, as both projections read it ([`crate::markdown::heading_level`]), a
//!   `section-header`; then a run's tagged role path after the document's `/RoleMap`, as Markdown
//!   reads it, innermost role first — `H`, `H1`–`H6` are a `section-header`, and `Title`, `LI`,
//!   `LBody`, `Lbl` and `Caption` settle it as `text`, since this view has no title, list or
//!   caption label (`docs/06-STEAL-REFUSE.md` P14); then a run's `inferred_heading` (decision #29)
//!   at a level 1–6, a `section-header`. Otherwise `text`. **The label tier, level 7, is `text`**,
//!   as both projections write it (decision #47); the adapter called it a heading. A tagged
//!   `Table` role labels nothing: the table is its record.
//! - A **table** is every table record, with its cells and their nodes; its runs are not text
//!   units again. Its `bbox` is the record's measured box, absent where the record has none — a
//!   tagged table's grid, an office table. The adapter dropped a table with no box and kept its
//!   runs as one element each: 4,163 of the `cfpb-home-loan-toolkit` gate document's 5,260.
//!   **A table the next page goes on with is one element** (decision #142, both projections'
//!   rule): that page's rows follow its own, each of their cells carrying its page and its own
//!   box, and the grid names the record they come from under `continued`. Its `bbox` is its first
//!   page's.
//! - A **picture** is a painted image placement or a figure region with a measured box, its text
//!   that of each unit holding a run the record names inside it (decision #97).
//! - A **form field** is each form field node (decision #163): its fully qualified name, its label,
//!   its type, its value as the document wrote it, and a check box's or radio button's state, all
//!   copied from the record (decision #162). **No box**: the rectangle a field declares is where a
//!   viewer draws it, not ink this engine measured, and a `bbox` here is always a measured one.
//!
//! **Nothing is dropped silently.** Every node is in exactly one place: a text unit's or a table's
//! `node_ids`, a picture's own image node, a form field's own node, or [`Elements::omitted`] under
//! its reason.
//!
//! Elements are in page order, and on a page in the record's reading order: each sits at the first
//! node it covers, and one covering none comes last on its page. A picture's `node_ids` and text
//! repeat runs that are elements of their own, so elements may overlap there.
//!
//! No verdict, no score: every label is one a record field states, every box one the record
//! measured or the union of such boxes, and every node id a copy.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::markdown::{
    docx_paragraph, geometric_blocks, paragraph_seam, projected_text, sheet_name,
    text_run_attributes,
};
use crate::representation::DocumentRepresentation;
use crate::{
    ArtifactIdentity, CheckState, CoordinateSystem, EngineError, FieldValue, Furniture,
    NativeLocator, NodeAttributes, NodeId, NodeKind, QRect, Sha256Hex, StructuralLocator,
};

/// The artifact type this emits.
pub const ELEMENTS_ARTIFACT_TYPE: &str = "ethos.parser.elements.v0";

/// Its schema version.
pub const ELEMENTS_SCHEMA_VERSION: &str = "0.1.0";

/// The rule that groups, labels and orders the elements, carried on the artifact.
///
/// **Not on the profile**, unlike `markdown_rule` and `locate_rule`: a profile field moves every
/// artifact's `profile_sha256`, and adding this view changes no other output. A change to how this
/// projection groups, labels or orders is a `-v2` here; a change to a reader rule it reads — the
/// layout units, the furniture, the headings, the figures — moves the record's `profile_sha256`.
pub const ELEMENTS_RULE_V1: &str = "elements-units-v1";

/// A document's items, each with its box and label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Elements {
    /// `artifact_type`, `schema_version`, `parser_version` (the build that projected this) and
    /// `profile_sha256` — **the record's own**, the profile it was read under, which names the
    /// reader rules the grouping and labels follow.
    #[serde(flatten, deserialize_with = "pinned_identity")]
    pub identity: ArtifactIdentity,
    /// Digest of the original source bytes, carried through from the representation.
    pub source_sha256: Sha256Hex,
    /// Digest of the representation these elements were built from. It does not cover geometry,
    /// which sits outside the representation's fingerprint (`docs/01-CONTRACT.md` §4).
    pub representation_sha256: Sha256Hex,
    /// Which rule built them: [`ELEMENTS_RULE_V1`].
    pub elements_rule: String,
    /// The representation's own coordinate system, which every box here is in.
    pub coordinate_system: CoordinateSystem,
    /// The pages, as the representation declares them. Empty for a source with no pages.
    pub pages: Vec<ElementPage>,
    /// The elements, in page order and on a page in reading order.
    pub elements: Vec<Element>,
    /// What no element carries, by reason, in [`OmissionReason`] order. Empty when nothing is.
    pub omitted: Vec<Omitted>,
}

/// One page, as the representation declares it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementPage {
    /// The page's 1-based position in the document — physical order, not a printed page label;
    /// an element's `page`.
    pub index: u32,
    /// Width in integer centipoints, after rotation.
    pub width: i64,
    /// Height in integer centipoints, after rotation.
    pub height: i64,
    /// Quarter turns clockwise beyond the declared rotation in which the page was read, its text
    /// drawn turned (decision #155): the frame width, height and every `bbox` are in. Absent where
    /// 0; see `PageRecord::to_declared`.
    #[serde(default, skip_serializing_if = "is_no_turn")]
    pub turn: i64,
    /// The box a viewer shows, where narrower than the page (decision #62).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<QRect>,
}

fn is_no_turn(turn: &i64) -> bool {
    *turn == 0
}

/// What an element is, as the record states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ElementLabel {
    /// Text the record states nothing more about.
    Text,
    /// A tagged heading, or a run the reader inferred a heading from its type, at level 1–6.
    SectionHeader,
    /// A running head: the reader's `furniture: header`.
    PageHeader,
    /// A folio or running foot: the reader's `furniture: footer`.
    PageFooter,
    /// A table record.
    Table,
    /// A painted image or a figure region.
    Picture,
    /// An interactive form field (decision #163).
    FormField,
}

/// One item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Element {
    /// `e1`, `e2`, … in this artifact's order.
    pub id: String,
    /// What it is.
    pub label: ElementLabel,
    /// Its page's `index`. Absent for a source with no pages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// `[x0, y0, x1, y1]`, in [`Elements::coordinate_system`]. Absent where the record has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<QRect>,
    /// Its text: a unit's blocks joined by one space (a DOCX paragraph's as the projections write
    /// it), a table's cells' texts, a picture's units', a form field's node's text (its value as
    /// text: a radio group's widgets share it, and only `form_field.state` says which is on).
    pub text: String,
    /// The representation nodes it covers, copied: a unit's runs; a table's cells' nodes; a
    /// painted image's node, then the runs the record names inside it; a form field's node.
    pub node_ids: Vec<NodeId>,
    /// A `section-header`'s level, 1–6, as tagged or inferred. Absent on every other label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// A table record's grid. Present on every `table` element, and only there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<ElementTable>,
    /// A form field's facts. Present on every `form-field` element, and only there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form_field: Option<ElementFormField>,
}

/// A form field's facts, copied from its node (decisions #162, #163).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementFormField {
    /// The fully qualified field name. Absent where the document names none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The name the form gives the field for a person to read. Absent where the field states none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The field's type as the document spells it. Absent where no ancestor declares one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_type: Option<String>,
    /// The field's value, in the shape the document wrote it.
    pub value: FieldValue,
    /// A check box's or radio button's state as its widget states it, or why it cannot be told.
    /// Present on those two kinds of field only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<CheckState>,
}

/// A table record's grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementTable {
    /// The table record's id.
    pub id: NodeId,
    /// A worksheet's name, as its cells' locators state it (decision #135). Absent on any other
    /// table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Rows in the grid.
    pub rows: u32,
    /// Columns in the grid.
    pub columns: u32,
    /// Its cells, in record order.
    pub cells: Vec<ElementCell>,
    /// The table records on the following pages this table goes on in (decision #142), in page
    /// order: their rows follow its own, their cells carry their page. Absent where it goes on in
    /// none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub continued: Vec<NodeId>,
}

/// One cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementCell {
    /// Zero-indexed row.
    pub row: u32,
    /// Zero-indexed column.
    pub column: u32,
    /// Rows it spans; 1 means not merged.
    pub rowspan: u32,
    /// Columns it spans; 1 means not merged.
    pub colspan: u32,
    /// Its page's `index`, where that is not its element's: a cell of a table going on from the
    /// page before (decision #142). Absent on every other cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Its measured box. Absent where the record has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<QRect>,
    /// Its text, as the record holds it.
    pub text: String,
    /// The nodes the record places in it.
    pub node_ids: Vec<NodeId>,
}

/// Why no element carries something the record holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OmissionReason {
    /// Text runs on a page none of whose block has a measured box, as `ground` omits them.
    NoMeasuredBox,
    /// A DOCX paragraph holding nothing but whitespace.
    WhitespaceOnly,
    /// An annotation's text: markup over the page, not the page's own.
    Annotation,
    /// An image whose placement is not an axis-aligned rectangle the record states.
    ImageNotPainted,
    /// A figure region with no measured box. A region is no node: `count` counts regions and
    /// `node_ids` is empty.
    FigureNotMeasured,
}

/// What no element carries, for one reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Omitted {
    /// Why.
    pub reason: OmissionReason,
    /// How many: nodes, or figure regions for [`OmissionReason::FigureNotMeasured`].
    pub count: u32,
    /// The nodes, in reading order. They stay in the representation.
    pub node_ids: Vec<NodeId>,
}

impl Elements {
    /// The canonical bytes this artifact serializes to.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        crate::c14n::canonical_bytes_of(self).map_err(|e| malformed(e.to_string()))
    }
}

/// [`Elements::identity`], read only as this module's type at this module's version.
fn pinned_identity<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ArtifactIdentity, D::Error> {
    ArtifactIdentity::deserialize_pinned(d, ELEMENTS_ARTIFACT_TYPE, &[ELEMENTS_SCHEMA_VERSION])
}

fn malformed(detail: String) -> EngineError {
    EngineError::Malformed {
        what: "elements artifact".into(),
        detail,
    }
}

/// One text unit while it is built.
struct Unit {
    /// `(page, layout_unit)` where a run carries a unit; a block carrying none merges with nothing.
    key: Option<(Option<u32>, u32)>,
    page: Option<u32>,
    bbox: Option<QRect>,
    text: String,
    /// Every run of its blocks: what its text is made of, and its `node_ids`.
    members: Vec<usize>,
    /// The runs a unit, a label and a picture are read from: those with a measured box — the spans
    /// `ground` cites — or every run on a source with no pages, where none has one.
    runs: Vec<usize>,
    /// A DOCX paragraph (decision #130): its last run holding text, which the next joins onto.
    paragraph: Option<Option<usize>>,
}

impl Unit {
    /// Decision #130: run `i` of this DOCX paragraph, its text joined as both projections join it.
    fn push_paragraph_run(&mut self, nodes: &[crate::Node], i: usize) {
        self.members.push(i);
        self.runs.push(i);
        if nodes[i].reading_text().trim().is_empty() {
            return;
        }
        if let Some(Some(prev)) = self.paragraph {
            self.text
                .push_str(paragraph_seam(nodes, i, &nodes[prev]).unwrap_or(" "));
        }
        self.text.push_str(&projected_text(nodes, i));
        self.paragraph = Some(Some(i));
    }
}

/// The smallest box holding both: the union of measured boxes, measured rather than inferred.
fn union(a: Option<QRect>, b: Option<QRect>) -> Result<Option<QRect>, EngineError> {
    match (a, b) {
        (Some(a), Some(b)) => QRect::new(
            a.x0().min(b.x0()),
            a.y0().min(b.y0()),
            a.x1().max(b.x1()),
            a.y1().max(b.y1()),
        )
        .map(Some)
        .map_err(|e| malformed(e.to_string())),
        (a, b) => Ok(a.or(b)),
    }
}

/// A unit's label and a `section-header`'s level, from what its runs' records state.
///
/// Page furniture first (decision #41), a Word header's or footer's included (decision #136): a
/// running head the heading rule also read stays the page's header. Then, run by run, a level a
/// page-less source declares — a DOCX paragraph's, an ODF heading's, an EPUB `<h1>`–`<h6>` — as
/// both projections read it ([`crate::markdown::heading_level`]); the tagged role path after the
/// document's `/RoleMap`, innermost first; and the run's inferred heading at a level the
/// projections write as one.
fn label_of(nodes: &[crate::Node], members: &[usize]) -> (ElementLabel, Option<u8>) {
    for &i in members {
        let furniture = match &nodes[i].attributes {
            NodeAttributes::OfficeRun(a) => a.furniture,
            _ => text_run_attributes(&nodes[i]).and_then(|a| a.furniture),
        };
        match furniture {
            Some(Furniture::Header) => return (ElementLabel::PageHeader, None),
            Some(Furniture::Footer) => return (ElementLabel::PageFooter, None),
            None => {}
        }
    }
    for &i in members {
        let node = &nodes[i];
        if text_run_attributes(node).is_none() {
            if let Some(level) = crate::markdown::heading_level(node) {
                return (ElementLabel::SectionHeader, Some(level));
            }
        }
        if let Some(StructuralLocator::PdfTagged(t)) = &node.structural_locator {
            let path = t.standard_role_path.as_ref().unwrap_or(&t.role_path);
            for role in path.iter().rev() {
                let heading = match role.as_str() {
                    "H" | "H1" => Some(1),
                    "H2" => Some(2),
                    "H3" => Some(3),
                    "H4" => Some(4),
                    "H5" => Some(5),
                    "H6" => Some(6),
                    "Title" | "LI" | "LBody" | "Lbl" | "Caption" => {
                        return (ElementLabel::Text, None)
                    }
                    _ => None,
                };
                if heading.is_some() {
                    return (ElementLabel::SectionHeader, heading);
                }
            }
        }
        // Decision #47: level 7 is the label tier, a bold line both projections write without `#`.
        if let Some(level) = text_run_attributes(node)
            .filter(|a| a.inferred_heading)
            .map(|a| a.inferred_heading_level.unwrap_or(1).max(1))
            .filter(|&level| level <= 6)
        {
            return (ElementLabel::SectionHeader, Some(level));
        }
    }
    (ElementLabel::Text, None)
}

/// The elements of `repr`, under [`ELEMENTS_RULE_V1`], stamped with the record's own profile.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a node, table or figure names a page the representation does not
/// declare, or a node has no geometry row.
pub fn to_elements(
    repr: &DocumentRepresentation,
    parser_version: &str,
) -> Result<Elements, EngineError> {
    let payload = repr.payload();
    let nodes = &payload.nodes;
    let paginated = !payload.pages.is_empty();
    let pages: BTreeMap<&str, u32> = payload
        .pages
        .iter()
        .map(|p| (p.id.as_str(), p.index))
        .collect();
    // A source with no pages has no page to give: its elements carry none.
    let page_of = |id: &NodeId| -> Result<Option<u32>, EngineError> {
        if !paginated {
            return Ok(None);
        }
        pages
            .get(id.as_str())
            .copied()
            .map(Some)
            .ok_or_else(|| malformed(format!("`{}` is not a declared page", id.as_str())))
    };
    let index: BTreeMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut omitted: BTreeMap<OmissionReason, Vec<NodeId>> = BTreeMap::new();
    let mut omit = |reason, ids: &[usize]| {
        omitted
            .entry(reason)
            .or_default()
            .extend(ids.iter().map(|&i| nodes[i].id.clone()));
    };

    // Every table record is an element, and its runs are not units again. Every table's runs are
    // their own blocks, as `ground` cuts them.
    let tabled: BTreeSet<&str> = payload
        .tables
        .iter()
        .flat_map(|t| t.cells.iter().flat_map(|c| c.node_ids.iter()))
        .map(NodeId::as_str)
        .collect();

    let mut units: Vec<Unit> = Vec::new();
    for block in geometric_blocks(nodes, &tabled) {
        let first = &nodes[block.members[0]];
        match first.kind {
            NodeKind::TextRun => {}
            // Decision #163: an element each, below.
            NodeKind::FormField => continue,
            NodeKind::Annotation => {
                omit(OmissionReason::Annotation, &block.members);
                continue;
            }
            // A picture, or counted with the pictures below.
            NodeKind::Image => continue,
        }
        if block
            .members
            .iter()
            .all(|&i| tabled.contains(nodes[i].id.as_str()))
        {
            continue;
        }
        // Decision #130: the next run of one DOCX paragraph continues its unit.
        if let Some(last) = units.last_mut().filter(|u| u.paragraph.is_some()) {
            let prev = &nodes[*last.members.last().expect("a unit holds a run")];
            if paragraph_seam(nodes, block.members[0], prev).is_some() {
                for &i in &block.members {
                    last.push_paragraph_run(nodes, i);
                }
                continue;
            }
        }
        let mut bbox = None;
        let mut runs = Vec::new();
        for &i in &block.members {
            let measured = repr
                .geometry_at(i)
                .ok_or_else(|| malformed(format!("node {i} has no geometry row")))?
                .measured();
            if measured.is_some() || !paginated {
                runs.push(i);
            }
            bbox = union(bbox, measured)?;
        }
        // On a page, a block with no measured box is no element, as `ground` omits it.
        if runs.is_empty() {
            omit(OmissionReason::NoMeasuredBox, &block.members);
            continue;
        }
        let page = page_of(&first.parent)?;
        if docx_paragraph(first).is_some() {
            let mut unit = Unit {
                key: None,
                page,
                bbox,
                text: String::new(),
                members: Vec::new(),
                runs: Vec::new(),
                paragraph: Some(None),
            };
            for &i in &block.members {
                unit.push_paragraph_run(nodes, i);
            }
            units.push(unit);
            continue;
        }
        let unit = runs
            .iter()
            .find_map(|&i| text_run_attributes(&nodes[i]).and_then(|a| a.layout_unit));
        match (unit, units.last_mut()) {
            (Some(u), Some(last)) if last.key == Some((page, u)) => {
                last.bbox = union(last.bbox, bbox)?;
                last.text.push(' ');
                last.text.push_str(&block.text);
                last.members.extend(&block.members);
                last.runs.extend(runs);
            }
            _ => units.push(Unit {
                key: unit.map(|u| (page, u)),
                page,
                bbox,
                text: block.text,
                members: block.members,
                runs,
                paragraph: None,
            }),
        }
    }
    // A DOCX paragraph of whitespace alone has no text to stand as.
    units.retain(|u| {
        let blank = u.paragraph == Some(None);
        if blank {
            omit(OmissionReason::WhitespaceOnly, &u.members);
        }
        !blank
    });

    // A picture's text: that of each unit holding a run the record names inside it.
    let text_inside = |named: &[NodeId]| -> String {
        let named: BTreeSet<&str> = named.iter().map(NodeId::as_str).collect();
        units
            .iter()
            .filter(|u| u.runs.iter().any(|&i| named.contains(nodes[i].id.as_str())))
            .map(|u| u.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    };

    let mut out: Vec<Element> = Vec::new();
    for u in &units {
        let (label, level) = label_of(nodes, &u.runs);
        out.push(element(
            label,
            u.page,
            u.bbox,
            u.text.clone(),
            u.members.iter().map(|&i| nodes[i].id.clone()).collect(),
            level,
            None,
        ));
    }
    // Decision #142: a table the next page goes on with is one element with that page's rows
    // under its own, each cell there carrying its page.
    let links = crate::markdown::continued_tables(repr);
    let mut tables: Vec<Option<Element>> = Vec::with_capacity(payload.tables.len());
    let mut opens: Vec<usize> = (0..payload.tables.len()).collect();
    for (k, t) in payload.tables.iter().enumerate() {
        let cell_nodes = t.cells.iter().flat_map(|c| c.node_ids.iter());
        let text = t
            .cells
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let page = page_of(&t.page)?;
        if let Some((prev, _)) = links[k] {
            opens[k] = opens[prev];
            if let Some(Some(head)) = tables.get_mut(opens[k]) {
                let grid = head
                    .table
                    .as_mut()
                    .expect("a table element carries its grid");
                let below = grid.rows;
                grid.rows += t.rows;
                grid.continued.push(t.id.clone());
                grid.cells.extend(t.cells.iter().map(|c| ElementCell {
                    row: c.position.row + below,
                    page,
                    ..cell_of(c)
                }));
                head.text.push(' ');
                head.text.push_str(&text);
                head.node_ids.extend(cell_nodes.cloned());
                tables.push(None);
                continue;
            }
        }
        tables.push(Some(element(
            ElementLabel::Table,
            page,
            t.geometry.measured(),
            text,
            cell_nodes.clone().cloned().collect(),
            None,
            Some(ElementTable {
                id: t.id.clone(),
                name: cell_nodes
                    .clone()
                    .find_map(|id| sheet_name(&nodes[*index.get(id.as_str())?]))
                    .map(str::to_string),
                rows: t.rows,
                columns: t.columns,
                cells: t.cells.iter().map(cell_of).collect(),
                continued: Vec::new(),
            }),
        )));
    }
    out.extend(tables.into_iter().flatten());
    for (i, node) in nodes.iter().enumerate() {
        if node.kind != NodeKind::Image {
            continue;
        }
        let painted = match (&node.native_locator, &node.attributes) {
            (NativeLocator::PdfImage(at), NodeAttributes::Image(image)) => {
                at.rect.painted().map(|rect| (rect, image))
            }
            _ => None,
        };
        let Some((rect, image)) = painted else {
            omit(OmissionReason::ImageNotPainted, &[i]);
            continue;
        };
        out.push(element(
            ElementLabel::Picture,
            page_of(&node.parent)?,
            Some(rect),
            text_inside(&image.runs_inside),
            std::iter::once(node.id.clone())
                .chain(image.runs_inside.iter().cloned())
                .collect(),
            None,
            None,
        ));
    }
    // Decision #163: each form field, at its node, with no box: a declared rectangle is not one
    // this engine measured.
    for node in nodes {
        let NodeAttributes::FormField(f) = &node.attributes else {
            continue;
        };
        let mut e = element(
            ElementLabel::FormField,
            page_of(&node.parent)?,
            None,
            node.text.clone(),
            vec![node.id.clone()],
            None,
            None,
        );
        e.form_field = Some(ElementFormField {
            name: f.field_name.clone(),
            label: f.label.clone(),
            field_type: f.field_type.clone(),
            value: f.value.clone(),
            state: f.state.clone(),
        });
        out.push(e);
    }
    let mut figures_unmeasured = 0usize;
    for f in &payload.figures {
        let Some(rect) = f.geometry.measured() else {
            figures_unmeasured += 1;
            continue;
        };
        out.push(element(
            ElementLabel::Picture,
            page_of(&f.page)?,
            Some(rect),
            text_inside(&f.runs_inside),
            f.runs_inside.clone(),
            None,
            None,
        ));
    }

    // Page order, and on a page the record's reading order: each element at the first node it
    // covers, one covering none last. Stable, so a tie keeps the order built above.
    out.sort_by_key(|e| {
        let first = e
            .node_ids
            .iter()
            .filter_map(|id| index.get(id.as_str()).copied())
            .min();
        (e.page, first.unwrap_or(usize::MAX))
    });
    for (n, e) in out.iter_mut().enumerate() {
        e.id = format!("e{}", n + 1);
    }

    let count = |n: usize| u32::try_from(n).map_err(|_| malformed(format!("{n} omitted")));
    let mut omissions = Vec::new();
    for (reason, node_ids) in omitted {
        omissions.push(Omitted {
            reason,
            count: count(node_ids.len())?,
            node_ids,
        });
    }
    if figures_unmeasured > 0 {
        omissions.push(Omitted {
            reason: OmissionReason::FigureNotMeasured,
            count: count(figures_unmeasured)?,
            node_ids: Vec::new(),
        });
    }

    Ok(Elements {
        identity: ArtifactIdentity {
            artifact_type: ELEMENTS_ARTIFACT_TYPE.to_string(),
            schema_version: ELEMENTS_SCHEMA_VERSION.to_string(),
            parser_version: parser_version.to_string(),
            profile_sha256: payload.identity.profile_sha256.clone(),
        },
        source_sha256: payload.source.sha256.clone(),
        representation_sha256: repr.fingerprint().clone(),
        elements_rule: ELEMENTS_RULE_V1.to_string(),
        coordinate_system: payload.coordinate_system,
        pages: payload
            .pages
            .iter()
            .map(|p| ElementPage {
                index: p.index,
                width: p.width,
                height: p.height,
                turn: p.turn,
                visible: p.visible,
            })
            .collect(),
        elements: out,
        omitted: omissions,
    })
}

/// A table record's cell, as an element's grid carries it.
fn cell_of(c: &crate::TableCellRecord) -> ElementCell {
    ElementCell {
        row: c.position.row,
        column: c.position.column,
        rowspan: c.position.rowspan,
        colspan: c.position.colspan,
        page: None,
        bbox: c.geometry.measured(),
        text: c.text.clone(),
        node_ids: c.node_ids.clone(),
    }
}

/// An element before its id is given.
fn element(
    label: ElementLabel,
    page: Option<u32>,
    bbox: Option<QRect>,
    text: String,
    node_ids: Vec<NodeId>,
    level: Option<u8>,
    table: Option<ElementTable>,
) -> Element {
    Element {
        id: String::new(),
        label,
        page,
        bbox,
        text,
        node_ids,
        level,
        table,
        form_field: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::tests::{
        cell, payload, repr_of_lines, repr_of_paths, repr_with_table, text_node,
        with_inferred_headings, with_units,
    };
    use crate::{GeometryAbsence, GeometryPresence, IdAllocator, IdKind, NodeGeometry, Profile};

    fn artifact(repr: &DocumentRepresentation) -> Elements {
        to_elements(repr, &Profile::default().parser_version).unwrap()
    }

    fn elements(repr: &DocumentRepresentation) -> Vec<Element> {
        artifact(repr).elements
    }

    fn summary(repr: &DocumentRepresentation) -> Vec<(ElementLabel, String, Option<u8>)> {
        elements(repr)
            .into_iter()
            .map(|e| (e.label, e.text, e.level))
            .collect()
    }

    /// **A form field's facts reach its element as the record holds them** (decision #163): name,
    /// label, type, value and state, the node its own, and no box.
    #[test]
    fn a_form_field_element_copies_its_nodes_facts() {
        let repr = repr_of_lines(&["Agree to terms"]);
        let mut payload = repr.payload().clone();
        let mut geometry = repr.geometry().to_vec();
        let page = payload.pages[0].id.clone();
        let facts = crate::FormFieldAttributes {
            field_name: Some("terms.agree".into()),
            field_type: Some("Btn".into()),
            label: Some("I agree".into()),
            value: crate::FieldValue::Name("Yes".into()),
            state: Some(CheckState::Checked("Yes".into())),
            flags: Vec::new(),
            unrecognized_flag_bits: Vec::new(),
        };
        let id = NodeId::from_parts(IdKind::FormField, 1);
        payload.nodes.push(crate::Node {
            id: id.clone(),
            kind: NodeKind::FormField,
            parent: page,
            ordinal: payload.nodes.len() as u32 + 1,
            text: String::new(),
            native_locator: NativeLocator::PdfObject(crate::PdfObjectLocator {
                page: 1,
                object: 9,
                generation: 0,
                rect: crate::AnnotationRect::NotDeclared,
            }),
            structural_locator: None,
            derivation: crate::DerivationClass::Extracted,
            attributes: NodeAttributes::FormField(facts.clone()),
        });
        geometry.push(NodeGeometry {
            node: id.clone(),
            presence: GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        });
        payload
            .assurance
            .limitations
            .push(crate::assurance::Limitation::document(
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
                "test: a field has no ink box",
            ));
        let repr = DocumentRepresentation::seal(payload, geometry).unwrap();
        let out = artifact(&repr);
        let field = out.elements.last().unwrap();
        assert_eq!(field.label, ElementLabel::FormField);
        assert_eq!(field.node_ids, [id]);
        assert_eq!(field.bbox, None);
        assert_eq!(
            field.form_field,
            Some(ElementFormField {
                name: facts.field_name,
                label: facts.label,
                field_type: facts.field_type,
                value: facts.value,
                state: facts.state,
            })
        );
        assert!(out.omitted.is_empty());
    }

    /// `repr` with each run's furniture and inferred level set, one entry per node in order.
    fn with_runs(
        repr: DocumentRepresentation,
        runs: &[(Option<Furniture>, Option<u8>)],
    ) -> DocumentRepresentation {
        let mut payload = repr.payload().clone();
        for (node, (furniture, level)) in payload.nodes.iter_mut().zip(runs) {
            if let NodeAttributes::TextRun(a) = &mut node.attributes {
                a.furniture = *furniture;
                a.inferred_heading_level = *level;
            }
        }
        DocumentRepresentation::seal(payload, repr.geometry().to_vec()).unwrap()
    }

    /// `repr` with the nodes at `indices` given no box, as a run the reader could not measure.
    fn without_boxes(repr: DocumentRepresentation, indices: &[usize]) -> DocumentRepresentation {
        let mut geometry = repr.geometry().to_vec();
        for &i in indices {
            geometry[i].presence = GeometryPresence::Absent(GeometryAbsence::NotReportedByReader);
        }
        let mut payload = repr.payload().clone();
        payload
            .assurance
            .limitations
            .push(crate::assurance::Limitation::document(
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
                "test: runs the reader could not measure",
            ));
        DocumentRepresentation::seal(payload, geometry).unwrap()
    }

    #[test]
    fn consecutive_blocks_of_one_layout_unit_are_one_element() {
        let repr = with_units(
            repr_of_lines(&["One", "two", "Three", "Four", "Five"]),
            &[Some(1), Some(1), Some(2), None, None],
        );
        let got = elements(&repr);
        let texts: Vec<&str> = got.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, ["One two", "Three", "Four", "Five"]);
        let ids = &repr.payload().nodes;
        assert_eq!(got[0].node_ids, [ids[0].id.clone(), ids[1].id.clone()]);
        assert_eq!(
            got.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["e1", "e2", "e3", "e4"]
        );
        assert!(got.iter().all(|e| e.page == Some(1) && e.bbox.is_some()));
    }

    #[test]
    fn furniture_comes_before_a_heading_and_a_heading_carries_its_level() {
        let repr = with_inferred_headings(
            with_units(
                repr_of_lines(&["Running head", "Title", "Section", "Body", "7"]),
                &[Some(1), Some(2), Some(3), Some(4), Some(5)],
            ),
            &[0, 1, 2],
        );
        let repr = with_runs(
            repr,
            &[
                (Some(Furniture::Header), None),
                (None, None),
                (None, Some(3)),
                (None, None),
                (Some(Furniture::Footer), None),
            ],
        );
        assert_eq!(
            summary(&repr),
            [
                (ElementLabel::PageHeader, "Running head".into(), None),
                (ElementLabel::SectionHeader, "Title".into(), Some(1)),
                (ElementLabel::SectionHeader, "Section".into(), Some(3)),
                (ElementLabel::Text, "Body".into(), None),
                (ElementLabel::PageFooter, "7".into(), None),
            ]
        );
    }

    /// The label tier — level 7, a bold line at the head of its text (decision #47) — is text, as
    /// both projections write it; a level the projections write with `#` is a section header.
    #[test]
    fn the_label_tier_is_text() {
        let repr = with_runs(
            with_inferred_headings(repr_of_lines(&["Loan terms", "Rates"]), &[0, 1]),
            &[(None, Some(7)), (None, Some(6))],
        );
        assert_eq!(
            summary(&repr),
            [
                (ElementLabel::Text, "Loan terms".into(), None),
                (ElementLabel::SectionHeader, "Rates".into(), Some(6)),
            ]
        );
    }

    #[test]
    fn a_role_path_is_read_innermost_first_and_before_an_inferred_heading() {
        let repr = with_inferred_headings(
            repr_of_paths(&[
                ("Intro", Some(&["Document", "Sect", "H2"])),
                ("Cell", Some(&["Document", "Table", "TR", "TD", "P"])),
                ("Item", Some(&["Document", "L", "LI", "LBody"])),
                ("Heading in a list", Some(&["Document", "H1", "LI"])),
                ("Bare", Some(&["Document", "P"])),
                ("Prose", Some(&["Document", "P"])),
            ]),
            &[2, 4],
        );
        assert_eq!(
            summary(&repr),
            [
                (ElementLabel::SectionHeader, "Intro".into(), Some(2)),
                (ElementLabel::Text, "Cell".into(), None),
                (ElementLabel::Text, "Item".into(), None),
                (ElementLabel::Text, "Heading in a list".into(), None),
                (ElementLabel::SectionHeader, "Bare".into(), Some(1)),
                (ElementLabel::Text, "Prose".into(), None),
            ]
        );
    }

    /// The role path after the document's `/RoleMap`, as Markdown reads it: a custom type mapped
    /// to `H2` is a section header, and a standard `H1` mapped to `P` is not.
    #[test]
    fn the_role_path_is_read_after_the_documents_role_map() {
        let repr = repr_of_paths(&[
            ("Mapped", Some(&["Document", "Heading"])),
            ("Unmapped", Some(&["Document", "H1"])),
        ]);
        let mut payload = repr.payload().clone();
        for (node, mapped) in payload
            .nodes
            .iter_mut()
            .zip([["Document", "H2"], ["Document", "P"]])
        {
            if let Some(StructuralLocator::PdfTagged(t)) = &mut node.structural_locator {
                std::sync::Arc::make_mut(t).standard_role_path =
                    Some(mapped.map(String::from).to_vec());
            }
        }
        let repr = DocumentRepresentation::seal(payload, repr.geometry().to_vec()).unwrap();
        assert_eq!(
            summary(&repr),
            [
                (ElementLabel::SectionHeader, "Mapped".into(), Some(2)),
                (ElementLabel::Text, "Unmapped".into(), None),
            ]
        );
    }

    #[test]
    fn a_measured_table_is_one_element_and_its_runs_are_not_text_again() {
        let repr = repr_with_table(
            &["Before", "Name", "Age", "Ann", "30", "After"],
            2,
            2,
            &[
                cell(0, 0, &[1]),
                cell(0, 1, &[2]),
                cell(1, 0, &[3]),
                cell(1, 1, &[4]),
            ],
        );
        let got = elements(&repr);
        // In reading order: the table sits where its first cell's run is read.
        assert_eq!(
            got.iter().map(|e| e.label).collect::<Vec<_>>(),
            [ElementLabel::Text, ElementLabel::Table, ElementLabel::Text]
        );
        let table = &got[1];
        assert_eq!(table.text, "Name Age Ann 30");
        let nodes = &repr.payload().nodes;
        assert_eq!(
            table.node_ids,
            nodes[1..5].iter().map(|n| n.id.clone()).collect::<Vec<_>>()
        );
        let grid = table
            .table
            .as_ref()
            .expect("a table element carries its grid");
        assert_eq!((grid.rows, grid.columns, grid.cells.len()), (2, 2, 4));
        assert_eq!(grid.cells[3].node_ids, [nodes[4].id.clone()]);
        assert!(table.bbox.is_some());
        assert_eq!(got[2].text, "After");
    }

    /// **A table the next page goes on with is one element** (decision #142): that page's rows
    /// follow its own, each of those cells carries its page and box, and the grid names the record
    /// it goes on in. Every node is still in one place.
    #[test]
    fn a_table_the_next_page_goes_on_with_is_one_element() {
        use crate::markdown::tests::{paged_tables, staff, FIRST, SECOND};
        let repr = paged_tables(&staff(&[]), &[FIRST, SECOND], crate::TABLE_DETECTION_V14);
        let got = elements(&repr);
        let tables: Vec<&Element> = got
            .iter()
            .filter(|e| e.label == ElementLabel::Table)
            .collect();
        assert_eq!(tables.len(), 1);
        let (table, record) = (tables[0], &repr.payload().tables);
        assert_eq!(table.page, Some(1));
        assert_eq!(table.text, "Name Dept Ann Audit Name Dept Bob Tax Cy Legal");
        let grid = table
            .table
            .as_ref()
            .expect("a table element carries its grid");
        assert_eq!((grid.rows, grid.columns, grid.cells.len()), (5, 2, 10));
        assert_eq!(grid.id, record[0].id);
        assert_eq!(grid.continued, [record[1].id.clone()]);
        assert_eq!(grid.cells[6].row, 3);
        assert_eq!(grid.cells[6].text, "Bob");
        assert_eq!(grid.cells[6].page, Some(2));
        assert_eq!(grid.cells[6].bbox, record[1].cells[2].geometry.measured());
        assert_eq!(grid.cells[3].page, None);
        let nodes = &repr.payload().nodes;
        assert_eq!(table.node_ids.len(), 10);
        assert!(table.node_ids.contains(&nodes[12].id));
        // A third page goes on with the same element.
        let lines = staff(&[
            ("Page 3", 3, Some(crate::Furniture::Footer)),
            ("Name", 3, None),
            ("Dept", 3, None),
            ("Dee", 3, None),
            ("Ops", 3, None),
        ]);
        let third = crate::markdown::tests::Grid {
            page: 3,
            cells: &[14, 15, 16, 17],
            ..FIRST
        };
        let three = elements(&paged_tables(
            &lines,
            &[FIRST, SECOND, third],
            crate::TABLE_DETECTION_V14,
        ));
        let grids: Vec<&ElementTable> = three.iter().filter_map(|e| e.table.as_ref()).collect();
        assert_eq!(grids.len(), 1);
        assert_eq!((grids[0].rows, grids[0].continued.len()), (7, 2));
        assert_eq!(
            grids[0].cells.last().map(|c| (c.row, c.page)),
            Some((6, Some(3)))
        );
        // The furniture between the pieces comes after the table, as read.
        let labels: Vec<ElementLabel> = got.iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            [
                ElementLabel::Text,
                ElementLabel::Table,
                ElementLabel::PageFooter,
                ElementLabel::PageFooter
            ]
        );
    }

    /// A table whose record has no box — a tagged table's grid — is one element with its cells and
    /// no `bbox`, not one element per run.
    #[test]
    fn a_table_with_no_box_is_one_element_without_a_bbox() {
        let repr = repr_with_table(
            &["Name", "Age", "After"],
            1,
            2,
            &[cell(0, 0, &[0]), cell(0, 1, &[1])],
        );
        let mut payload = repr.payload().clone();
        let absent = GeometryPresence::Absent(GeometryAbsence::NotReportedByStructureTree);
        payload.tables[0].geometry = absent;
        for c in &mut payload.tables[0].cells {
            c.geometry = absent;
        }
        let repr = DocumentRepresentation::seal(payload, repr.geometry().to_vec()).unwrap();
        let got = elements(&repr);
        assert_eq!(
            got.iter()
                .map(|e| (e.label, e.bbox.is_some()))
                .collect::<Vec<_>>(),
            [(ElementLabel::Table, false), (ElementLabel::Text, true)]
        );
        let grid = got[0].table.as_ref().unwrap();
        assert!(grid
            .cells
            .iter()
            .all(|c| c.bbox.is_none() && c.node_ids.len() == 1));
        assert_eq!(got[0].page, Some(1));
    }

    /// Every node is in exactly one place: a text unit's or a table's `node_ids`, a picture's own
    /// image node, or `omitted` under its reason.
    #[test]
    fn a_run_with_no_box_is_omitted_and_counted() {
        let repr = without_boxes(repr_of_lines(&["Kept", "Lost", "Also lost"]), &[1, 2]);
        let a = artifact(&repr);
        assert_eq!(a.elements.len(), 1);
        let nodes = &repr.payload().nodes;
        assert_eq!(
            a.omitted,
            [Omitted {
                reason: OmissionReason::NoMeasuredBox,
                count: 2,
                node_ids: vec![nodes[1].id.clone(), nodes[2].id.clone()],
            }]
        );
        assert!(artifact(&repr_of_lines(&["All kept"])).omitted.is_empty());
    }

    /// Page order first: a figure that names no run comes last on its page, never after the next
    /// page's elements.
    #[test]
    fn a_figure_naming_no_run_ends_its_own_page() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = |alloc: &mut IdAllocator, index| crate::PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index,
            width: 30000,
            height: 14400,
            rotation: 0,
            turn: 0,
            visible: None,
        };
        let (p1, p2) = (page(&mut alloc, 1), page(&mut alloc, 2));
        let one = text_node(&mut alloc, &p1.id, 1, "One", None);
        let mut two = text_node(&mut alloc, &p2.id, 1, "Two", None);
        if let NativeLocator::Pdf(l) = &mut two.native_locator {
            l.page = 2;
        }
        let measured = GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap());
        let geometry = [&one, &two]
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: measured,
            })
            .collect();
        let mut payload = payload(vec![one, two], vec![p1.clone(), p2]);
        payload.assurance.capabilities.figures = true;
        payload.figures.push(crate::FigureRecord {
            page: p1.id.clone(),
            geometry: measured,
            derivation: crate::DerivationClass::Computed,
            detection_rule: crate::FIGURE_RULE_V1.into(),
            runs_inside: Vec::new(),
        });
        let repr = DocumentRepresentation::seal(payload, geometry).unwrap();
        assert_eq!(
            elements(&repr)
                .iter()
                .map(|e| (e.page, e.label))
                .collect::<Vec<_>>(),
            [
                (Some(1), ElementLabel::Text),
                (Some(1), ElementLabel::Picture),
                (Some(2), ElementLabel::Text),
            ]
        );
    }
}
