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

//! The tagged-structure tree, read rather than guessed at (v1-S3).
//!
//! # Consume, never synthesise
//!
//! A `/StructTreeRoot` is **evidence the author left**. Everything in this module reads it and
//! nothing infers it: no heading is deduced from a font size, no table from a `"Table 3:"`
//! prefix. That inference is the parity checklist's P14, and a role path invented from typography
//! is indistinguishable on the wire from one the author wrote — which makes it worse than no role
//! path at all.
//!
//! So a document with no tree gets **no** role paths, and says so
//! (`untagged-structure-tree-absent`). Two absences are named rather than conflated:
//!
//! | What happened | What it is not |
//! | --- | --- |
//! | A run carries no `/MCID` | evidence the document is untagged |
//! | The tree cites an mcid no run carried | evidence the page has no text there |
//!
//! Both are counted. Neither is filled in.
//!
//! # `mcid` is the join key, not the address
//!
//! v0 already copied `/MCID` off `BDC` and could say nothing about what it meant. This walk
//! supplies the other half: the tree names a `(page, mcid)` pair and the role path that reaches
//! it, and a run is bound only when the tree cites **its** page and **its** id. The join is
//! equality on both. Nothing is fuzzy, nothing is nearest-match, and a run whose id the tree never
//! mentions keeps its bare `PdfMcid` and is counted as a gap.
//!
//! # Fail closed on a tree that does not terminate
//!
//! `/K` is a graph in practice, not a tree: a malformed or hostile file can point a child back at
//! an ancestor. Walking that forever is a denial of service, and walking it "far enough" would
//! silently truncate structure. Both are refused — [`MAX_DEPTH`] and an on-path visited set turn a
//! cycle into a named [`EngineError`], not a hang and not a partial answer.

use std::collections::{BTreeMap, BTreeSet};

use ethos_parser_core::{EngineError, PdfTaggedLocator};
use lopdf::{Dictionary, Object, ObjectId};

// The rule id lives in `ethos_parser_core::STRUCT_TREE_RULE_V1` and is NOT restated here, for the same
// reason the table rule ids are not: two spellings of one rule id is exactly the drift a versioned
// id exists to prevent.

/// How deep `/K` nesting may go before this is refused as malformed.
///
/// Sixty-four. Real documents nest a handful deep — `Document/Sect/Table/TR/TD/P` is six — so a
/// limit an order of magnitude above that costs nothing on a well-formed file and bounds a hostile
/// one. It is a **refusal**, not a truncation: a tree cut off at depth 64 would produce role paths
/// that look complete and are not.
pub const MAX_DEPTH: usize = 64;

/// A `(page object, marked-content id)` pair — the join key.
///
/// The page is part of the key, not context. Two pages number their marked content from zero
/// independently, so an mcid alone addresses nothing, and a join on the id alone would bind page
/// 2's paragraph to page 1's text on any document longer than one page.
type Key = (ObjectId, i64);

/// The `mcid` recorded for an object the tree cites by `/OBJR` rather than by marked content.
///
/// `-1`, and it is a real statement rather than a sentinel dressed as data: an `/OBJR` names the
/// object directly and there IS no marked-content id involved, so any non-negative value here
/// would be an id this engine minted. Negative marked-content ids are not legal PDF, so the value
/// cannot collide with one the document wrote.
const OBJECT_CITED_NOT_MARKED: i64 = -1;

/// What one walk of a document's structure tree produced.
#[derive(Debug, Default)]
pub struct StructureTree {
    /// The bindings, keyed by page and mcid.
    bindings: BTreeMap<Key, PdfTaggedLocator>,
    /// Bindings for whole objects the tree cites by `/OBJR` — annotations and widgets (v1-S4).
    ///
    /// v1-S3 walked `/OBJR` and bound nothing, because there was no node to bind it to yet.
    /// There is now, and the tree citing a widget is the author saying where that field sits in
    /// the document's structure. Keyed by object id: an `/OBJR` names the object directly, so
    /// unlike marked content there is no page-plus-index pair to join on.
    object_bindings: BTreeMap<ObjectId, PdfTaggedLocator>,
    /// Tables the tree describes, in document order.
    pub tables: Vec<TaggedTable>,
    /// Content items whose page could not be determined, so they bind nothing.
    ///
    /// `/Pg` is inheritable and optional, and a content item under no `/Pg` at all names an mcid
    /// on an unknown page. Guessing "probably page 1" would bind text to the wrong place on any
    /// document where it mattered.
    pub items_without_page: usize,
    /// How many structure elements were reached.
    pub elements: usize,
}

/// A table as the **structure tree** describes it, derived from tags alone.
///
/// **No geometry reaches this type.** Rows, columns and spans come from `/TR`, `/TD`, `/TH`,
/// `/RowSpan` and `/ColSpan`, which is what makes it an independent second opinion about a table
/// rather than a restatement of the detector's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedTable {
    /// The page the table's cells sit on, when its content items name one.
    pub page: Option<ObjectId>,
    /// Rows, from `/TR` count.
    pub rows: u32,
    /// Columns, from the widest row once spans are counted.
    pub columns: u32,
    /// One entry per `/TD` or `/TH`, in tree order.
    pub cells: Vec<TaggedCell>,
}

/// One `/TD` or `/TH`, as the tree describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedCell {
    /// Zero-based row, from the cell's position among `/TR` elements.
    pub row: u32,
    /// Zero-based column, from the cell's position within its row after earlier spans.
    pub column: u32,
    /// `/RowSpan`, defaulting to 1. **Never 0** — a cell occupying nothing cannot exist.
    pub rowspan: u32,
    /// `/ColSpan`, defaulting to 1.
    pub colspan: u32,
    /// The marked-content ids the tree binds **under this cell**, in tree order (v1-S7b).
    ///
    /// # Still no geometry, and that is the whole point
    ///
    /// These are `/MCID` integers the tree itself cites beneath this `/TD` or `/TH` — the
    /// author's own statement of which marked content belongs to this cell. Resolving them to
    /// text is a join against runs the page drew, on `(page, mcid)`, exactly the key v1-S3
    /// already binds locators with. No coordinate is consulted at any step, so a cell's text
    /// stays as independent of the geometric detector as its row and column are.
    ///
    /// Empty means the tree describes a cell it cites no marked content for. That is a real
    /// answer — an empty cell, or one whose content the producer did not mark — and it is
    /// distinct from a cell whose mcids resolve to no run.
    pub mcids: Vec<i64>,
    /// The page the tree names for this cell's content, when its bindings name one.
    ///
    /// Per cell rather than per table because a table can straddle a page break, and joining a
    /// cell's mcid against the wrong page's marked content is how a two-page table's text gets
    /// scrambled.
    pub page: Option<ObjectId>,
}

impl StructureTree {
    /// The role path the tree gives a run, if it cites that run's page and id.
    pub fn locator_for(&self, page: ObjectId, mcid: i64) -> Option<&PdfTaggedLocator> {
        self.bindings.get(&(page, mcid))
    }

    /// The role path the tree gives a whole object it cites by `/OBJR` (v1-S4).
    pub fn locator_for_object(&self, object: ObjectId) -> Option<&PdfTaggedLocator> {
        self.object_bindings.get(&object)
    }

    /// Every key the tree cites, so a caller can count the ones no run claimed.
    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        self.bindings.keys()
    }
}

/// Walk a document's structure tree, if it has one.
///
/// Returns `Ok(None)` when the catalog declares no `/StructTreeRoot` — the honest answer for an
/// untagged document, and distinct from an error.
///
/// # Errors
///
/// [`EngineError::Malformed`] if `/K` cycles, nests past [`MAX_DEPTH`], or names an object that
/// cannot be resolved. Fail closed: a structure tree this reader cannot follow to the end is not
/// a structure tree it may report half of.
pub fn read(doc: &lopdf::Document) -> Result<Option<StructureTree>, EngineError> {
    let Ok(catalog) = doc.catalog() else {
        // No catalog is a malformed document, but it is not *this* module's finding — extraction
        // has already succeeded by the time this runs, so the honest answer is "no tree here".
        return Ok(None);
    };
    let Ok(root_ref) = catalog.get(b"StructTreeRoot") else {
        return Ok(None);
    };
    let root = resolve_dict(doc, root_ref).map_err(|e| malformed("/StructTreeRoot", &e))?;

    let role_map = read_role_map(doc, root);

    let mut tree = StructureTree::default();
    let mut walker = Walker {
        doc,
        role_map: &role_map,
        tree: &mut tree,
        on_path: BTreeSet::new(),
        tables: Vec::new(),
        cells: Vec::new(),
    };

    let kids = root.get(b"K").ok();
    if let Some(k) = kids {
        walker.walk(k, &mut Vec::new(), None, 0)?;
    }

    Ok(Some(tree))
}

/// `/RoleMap`: the document's own statement of what its custom types mean.
///
/// Read as data. An unmapped custom type is emitted as itself and never guessed at — deciding
/// that `/Foo` means `/P` because it looks like a paragraph is the same inference this module
/// exists to avoid (`docs/09-V1-MILESTONES.md` S3, decision 8).
fn read_role_map(doc: &lopdf::Document, root: &Dictionary) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(obj) = root.get(b"RoleMap") else {
        return out;
    };
    let Ok(dict) = resolve_dict(doc, obj) else {
        return out;
    };
    for (k, v) in dict.iter() {
        if let Object::Name(n) = v {
            out.insert(name_to_string(k), name_to_string(n));
        }
    }
    out
}

struct Walker<'a> {
    doc: &'a lopdf::Document,
    role_map: &'a BTreeMap<String, String>,
    tree: &'a mut StructureTree,
    /// Object ids on the current path. A child that points back at one of them is a cycle.
    on_path: BTreeSet<ObjectId>,
    /// Tables currently open, innermost last. A table nested in a cell is a different table.
    tables: Vec<TableCtx>,
    /// Cells currently open, innermost last, as `(table index, cell index)` (v1-S7b).
    ///
    /// A stack rather than a single slot because a `/TD` may contain a nested `/Table`, and that
    /// inner table's cells must claim their own marked content rather than the outer cell's.
    cells: Vec<(usize, usize)>,
}

/// A `/Table` being walked.
///
/// Row and column indices come from **position in the tree** — the nth `/TR`, the next free
/// column in it — exactly as a reader of the tags would count them. Never from a coordinate.
struct TableCtx {
    /// Which entry in `StructureTree::tables` this fills.
    index: usize,
    /// How many `/TR` elements have been entered.
    rows_seen: u32,
    /// Slots already claimed, so a cell that spans down pushes later rows' cells rightward.
    occupied: BTreeSet<(u32, u32)>,
}

impl Walker<'_> {
    /// Walk one `/K` value: an array, a reference, an element, an `/MCR`, or a bare mcid integer.
    fn walk(
        &mut self,
        k: &Object,
        role_path: &mut Vec<String>,
        page: Option<ObjectId>,
        depth: usize,
    ) -> Result<(), EngineError> {
        if depth > MAX_DEPTH {
            return Err(EngineError::Malformed {
                what: "structure tree".into(),
                detail: format!(
                    "`/K` nests past {MAX_DEPTH} levels. Refused rather than truncated: a tree \
                     cut off partway produces role paths that look complete and are not"
                ),
            });
        }

        match k {
            // A bare integer is a marked-content id on the current `/Pg`.
            Object::Integer(mcid) => {
                self.bind(*mcid, page, role_path, None);
                Ok(())
            }
            Object::Array(items) => {
                for item in items {
                    self.walk(item, role_path, page, depth + 1)?;
                }
                Ok(())
            }
            Object::Reference(id) => {
                if !self.on_path.insert(*id) {
                    return Err(EngineError::Malformed {
                        what: "structure tree".into(),
                        detail: format!(
                            "`/K` cycles: object {} {} R is its own ancestor. Refused rather \
                             than walked, because following it does not terminate and stopping \
                             partway would report a structure the document does not have",
                            id.0, id.1
                        ),
                    });
                }
                let obj = self.doc.get_object(*id).map_err(|e| {
                    malformed(
                        "structure element",
                        &format!("object {} {} R does not resolve: {e}", id.0, id.1),
                    )
                })?;
                let r = self.walk(obj, role_path, page, depth + 1);
                self.on_path.remove(id);
                r
            }
            Object::Dictionary(d) => self.walk_dict(d, role_path, page, depth),
            // A `/K` of any other shape says nothing this reader can act on. Skipped rather than
            // refused: it is not a content item, so nothing binds and nothing is lost.
            _ => Ok(()),
        }
    }

    /// A dictionary under `/K`: a marked-content reference, an object reference, or an element.
    fn walk_dict(
        &mut self,
        d: &Dictionary,
        role_path: &mut Vec<String>,
        page: Option<ObjectId>,
        depth: usize,
    ) -> Result<(), EngineError> {
        match d.get(b"Type").ok().and_then(as_name) {
            // `/MCR` — a marked-content reference. Its own `/Pg` wins over the inherited one.
            Some(t) if t == "MCR" => {
                let pg = page_of(d).or(page);
                if let Some(mcid) = d.get(b"MCID").ok().and_then(|o| o.as_i64().ok()) {
                    self.bind(mcid, pg, role_path, None);
                }
                return Ok(());
            }
            // `/OBJR` — an object reference to an annotation, widget or XObject. v1-S3 walked
            // past these: nothing existed to bind them to. v1-S4 emits annotations and form
            // fields as nodes, so the citation is recorded and those nodes can carry the role
            // path the tree gives them.
            //
            // Still no text is read from here. An `/OBJR` says *where in the structure* an
            // object sits, not what it contains.
            Some(t) if t == "OBJR" => {
                if let Ok(Object::Reference(oid)) = d.get(b"Obj") {
                    if !role_path.is_empty() {
                        let standard: Vec<String> =
                            role_path.iter().map(|r| self.standard_role(r)).collect();
                        let mapped = standard != *role_path;
                        self.tree.object_bindings.insert(
                            *oid,
                            PdfTaggedLocator {
                                // An `/OBJR` is not marked content and has no id of its own.
                                // `-1` would be a number nobody wrote, so this reuses the
                                // sentinel-free option: the tree cites the object, not an mcid.
                                mcid: OBJECT_CITED_NOT_MARKED,
                                role_path: role_path.to_vec(),
                                standard_role_path: mapped.then_some(standard),
                                element_id: None,
                            },
                        );
                    }
                }
                return Ok(());
            }
            _ => {}
        }

        // Otherwise this is a structure element: it has an `/S`, or it is not one.
        let Some(raw_role) = d.get(b"S").ok().and_then(as_name) else {
            return Ok(());
        };
        self.tree.elements += 1;

        // `/Pg` is inheritable: an element's page applies to its descendants until one of them
        // names its own. Implemented as inheritance rather than as "the page we happen to be on",
        // because guessing would bind text to a page the document never named.
        let pg = page_of(d).or(page);
        let element_id = d
            .get(b"ID")
            .ok()
            .and_then(as_text)
            .filter(|s| !s.is_empty());

        let standard = self.standard_role(&raw_role);
        role_path.push(raw_role);

        // `/Table`, `/TR`, `/TD` and `/TH` are read AFTER `/RoleMap`, because a document that
        // maps its own `/MyRow` onto `/TR` has told us that is a row. The raw name still rides on
        // the role path, so the mapping is applied without laundering what the file says.
        let opened_table = standard == "Table";
        if opened_table {
            self.tree.tables.push(TaggedTable {
                page: pg,
                rows: 0,
                columns: 0,
                cells: Vec::new(),
            });
            self.tables.push(TableCtx {
                index: self.tree.tables.len() - 1,
                rows_seen: 0,
                occupied: BTreeSet::new(),
            });
        } else if standard == "TR" {
            if let Some(ctx) = self.tables.last_mut() {
                ctx.rows_seen += 1;
            }
        }
        // Opened only when `note_cell` actually recorded one: a `/TD` outside any `/TR`, or one
        // outside any table, records nothing, and pushing a slot for it would attach the next
        // marked content to whatever cell happened to be last.
        let opened_cell = if standard == "TD" || standard == "TH" {
            self.note_cell(d)
        } else {
            None
        };
        if let Some(slot) = opened_cell {
            self.cells.push(slot);
        }

        let result = (|| {
            if let Ok(k) = d.get(b"K") {
                self.walk_element_kids(k, role_path, pg, depth, element_id.as_deref())?;
            }
            Ok(())
        })();

        if opened_cell.is_some() {
            self.cells.pop();
        }
        if opened_table {
            if let Some(ctx) = self.tables.pop() {
                self.finish_table(ctx.index);
            }
        }
        role_path.pop();
        result
    }

    /// Record a `/TD` or `/TH` against the innermost open table.
    ///
    /// The column is the first slot in this row that nothing already claims, so a cell spanning
    /// down from an earlier row pushes this one rightward — the same rule a reader applies to an
    /// HTML table, and the only one that makes the resulting occupancy exact.
    ///
    /// Returns `(table index, cell index)` when a cell was recorded, so the caller can bind the
    /// marked content that arrives beneath it. `None` when nothing was recorded, which must not
    /// become a slot — see the call site.
    fn note_cell(&mut self, d: &Dictionary) -> Option<(usize, usize)> {
        let (rowspan, colspan) = spans_of(d);
        let ctx = self.tables.last_mut()?;
        if ctx.rows_seen == 0 {
            // A cell outside any `/TR`. The document's tree is not shaped like a table here, and
            // placing it at row 0 would invent a row the file does not have.
            return None;
        }
        let row = ctx.rows_seen - 1;
        let mut column = 0u32;
        while ctx.occupied.contains(&(row, column)) {
            column += 1;
        }
        for r in row..row.saturating_add(rowspan) {
            for c in column..column.saturating_add(colspan) {
                ctx.occupied.insert((r, c));
            }
        }
        let index = ctx.index;
        let cell = TaggedCell {
            row,
            column,
            rowspan,
            colspan,
            mcids: Vec::new(),
            page: None,
        };
        let t = self.tree.tables.get_mut(index)?;
        t.cells.push(cell);
        Some((index, t.cells.len() - 1))
    }

    /// An element's kids, with the element's `/ID` available to whatever binds directly under it.
    fn walk_element_kids(
        &mut self,
        k: &Object,
        role_path: &mut Vec<String>,
        page: Option<ObjectId>,
        depth: usize,
        element_id: Option<&str>,
    ) -> Result<(), EngineError> {
        // A bare mcid directly under this element is the common case, and it is the only place
        // the element's own `/ID` addresses the content — so it is threaded here rather than
        // carried down the whole recursion, where it would attach to a grandchild's content.
        match k {
            Object::Integer(mcid) => {
                self.bind(*mcid, page, role_path, element_id);
                Ok(())
            }
            Object::Array(items) => {
                for item in items {
                    self.walk_element_kids(item, role_path, page, depth + 1, element_id)?;
                }
                Ok(())
            }
            Object::Dictionary(d)
                if d.get(b"Type").ok().and_then(as_name).as_deref() == Some("MCR") =>
            {
                let pg = page_of(d).or(page);
                if let Some(mcid) = d.get(b"MCID").ok().and_then(|o| o.as_i64().ok()) {
                    self.bind(mcid, pg, role_path, element_id);
                }
                Ok(())
            }
            other => self.walk(other, role_path, page, depth + 1),
        }
    }

    /// Record a binding, or count the item if its page is unknown.
    fn bind(
        &mut self,
        mcid: i64,
        page: Option<ObjectId>,
        role_path: &[String],
        element_id: Option<&str>,
    ) {
        let Some(page) = page else {
            // No `/Pg` anywhere up the chain. The mcid addresses an unknown page, and a guess
            // would be a binding to the wrong text.
            self.tree.items_without_page += 1;
            return;
        };
        if role_path.is_empty() {
            return;
        }

        // A `/Table` element commonly carries no `/Pg` of its own — the page is named further
        // down, on the cells or their content references, which is where the content actually
        // is. So an open table learns its page from the first content item that binds under it,
        // rather than being left page-less and uncomparable.
        for ctx in &self.tables {
            if let Some(t) = self.tree.tables.get_mut(ctx.index) {
                if t.page.is_none() {
                    t.page = Some(page);
                }
            }
        }

        // v1-S7b. The innermost open `/TD` or `/TH` claims this marked content. Recorded here
        // rather than by re-walking later, because "under this cell" is a fact about the tree's
        // shape that only the walk knows — and it is the author's own statement of which content
        // is in which cell, which is what makes cell text independent of the detector.
        if let Some(&(ti, ci)) = self.cells.last() {
            if let Some(c) = self
                .tree
                .tables
                .get_mut(ti)
                .and_then(|t| t.cells.get_mut(ci))
            {
                c.mcids.push(mcid);
                if c.page.is_none() {
                    c.page = Some(page);
                }
            }
        }

        let standard: Vec<String> = role_path.iter().map(|r| self.standard_role(r)).collect();
        let mapped = standard != role_path;

        // Record cell membership for the innermost enclosing table, so a tagged table's cells can
        // be reached by the same key a run is.
        self.tree.bindings.insert(
            (page, mcid),
            PdfTaggedLocator {
                mcid,
                role_path: role_path.to_vec(),
                standard_role_path: mapped.then_some(standard),
                element_id: element_id.map(str::to_owned),
            },
        );
    }

    /// A role after `/RoleMap`, or unchanged when the document maps nothing for it.
    fn standard_role(&self, raw: &str) -> String {
        self.role_map
            .get(raw)
            .cloned()
            .unwrap_or_else(|| raw.into())
    }

    /// Fill in a table's grid once its subtree has been walked.
    ///
    /// Rows and columns are the extent the cells actually reach, spans included — **no box is
    /// consulted**, which is what makes this an independent opinion about the same table rather
    /// than a restatement of the detector's.
    fn finish_table(&mut self, at: usize) {
        let Some(t) = self.tree.tables.get_mut(at) else {
            return;
        };
        t.rows = t.cells.iter().map(|c| c.row + c.rowspan).max().unwrap_or(0);
        t.columns = t
            .cells
            .iter()
            .map(|c| c.column + c.colspan)
            .max()
            .unwrap_or(0);
    }
}

/// `/RowSpan` and `/ColSpan`, defaulting to 1.
///
/// PDF 32000-1 §14.8.5.7 puts them in an attribute dictionary under `/A` with `/O /Table`; some
/// producers write them straight onto the element. Both are read, `/A` first, because a producer
/// that writes both means the attribute dictionary.
///
/// **A declared span of 0 is corrected to 1 and not honoured.** Zero is not "unmerged" — it is a
/// cell occupying nothing, which cannot be true of a cell that exists, and propagating it would
/// make the occupancy check fail for a reason that is this reader's fault rather than the
/// document's.
fn spans_of(d: &Dictionary) -> (u32, u32) {
    let from = |dict: &Dictionary, key: &[u8]| -> Option<u32> {
        dict.get(key)
            .ok()
            .and_then(|o| o.as_i64().ok())
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v >= 1)
    };

    let mut row = None;
    let mut col = None;
    if let Ok(a) = d.get(b"A") {
        for dict in attribute_dicts(a) {
            row = row.or_else(|| from(dict, b"RowSpan"));
            col = col.or_else(|| from(dict, b"ColSpan"));
        }
    }
    (
        row.or_else(|| from(d, b"RowSpan")).unwrap_or(1),
        col.or_else(|| from(d, b"ColSpan")).unwrap_or(1),
    )
}

/// `/A` is one attribute dictionary or an array of them, optionally interleaved with revision
/// numbers. Both shapes are read; anything else contributes nothing.
fn attribute_dicts(a: &Object) -> Vec<&Dictionary> {
    match a {
        Object::Dictionary(d) => vec![d],
        Object::Array(items) => items
            .iter()
            .filter_map(|o| match o {
                Object::Dictionary(d) => Some(d),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl TaggedTable {
    /// Compare this tagged grid against a grid some detector found on the same page.
    ///
    /// # The halves share no input
    ///
    /// `self` comes from `/S`, `/TR`, `/TD`, `/RowSpan` and `/ColSpan` — **no box was read to
    /// build it**. The `detected` dimensions and positions come from painted rectangles or text
    /// origins — **no structure type was read to build those**. So agreement here is two
    /// independent readings of one table arriving at the same grid, and a check whose halves came
    /// from one source would agree with itself.
    ///
    /// Nothing is repaired. When they disagree the faults are named and the detector's grid is
    /// kept unchanged: preferring the tree's answer would be this engine choosing between two
    /// disagreeing sources with nothing on the wire to record that it chose.
    pub fn check_against(
        &self,
        detected_rows: u32,
        detected_columns: u32,
        detected: &[ethos_parser_core::TableCellPosition],
    ) -> ethos_parser_core::TaggedGridCheck {
        use ethos_parser_core::{CellSlot, TaggedGridFault, TaggedGridStatus};

        let mut faults = Vec::new();
        if self.rows != detected_rows {
            faults.push(TaggedGridFault::RowCountDiffers {
                tagged: self.rows,
                detected: detected_rows,
            });
        }
        if self.columns != detected_columns {
            faults.push(TaggedGridFault::ColumnCountDiffers {
                tagged: self.columns,
                detected: detected_columns,
            });
        }

        let tagged_slots: BTreeSet<CellSlot> = self
            .cells
            .iter()
            .flat_map(|c| {
                (c.row..c.row.saturating_add(c.rowspan)).flat_map(move |r| {
                    (c.column..c.column.saturating_add(c.colspan))
                        .map(move |col| CellSlot::new(r, col))
                })
            })
            .collect();
        let detected_slots: BTreeSet<CellSlot> = detected.iter().flat_map(|p| p.slots()).collect();

        for slot in tagged_slots.difference(&detected_slots) {
            faults.push(TaggedGridFault::SlotOnlyInTagged(*slot));
        }
        for slot in detected_slots.difference(&tagged_slots) {
            faults.push(TaggedGridFault::SlotOnlyInDetected(*slot));
        }
        faults.sort();
        faults.dedup();

        ethos_parser_core::TaggedGridCheck {
            check_id: ethos_parser_core::TAGGED_GRID_CHECK_V1.to_string(),
            check_version: "1".to_string(),
            outcome: if faults.is_empty() {
                TaggedGridStatus::Ok
            } else {
                TaggedGridStatus::Mismatch { faults }
            },
        }
    }
}

/// `/Pg`, when the dictionary names one.
fn page_of(d: &Dictionary) -> Option<ObjectId> {
    match d.get(b"Pg").ok()? {
        Object::Reference(id) => Some(*id),
        _ => None,
    }
}

fn as_name(o: &Object) -> Option<String> {
    match o {
        Object::Name(n) => Some(name_to_string(n)),
        _ => None,
    }
}

fn as_text(o: &Object) -> Option<String> {
    match o {
        Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
        Object::Name(n) => Some(name_to_string(n)),
        _ => None,
    }
}

/// A PDF name as text.
///
/// Lossy on purpose: a name is a byte string and this is a label, not evidence. A run's *text* is
/// never decoded this way — that path refuses rather than substitutes.
fn name_to_string(n: &[u8]) -> String {
    String::from_utf8_lossy(n).into_owned()
}

fn resolve_dict<'a>(doc: &'a lopdf::Document, o: &'a Object) -> Result<&'a Dictionary, String> {
    let (_, resolved) = doc.dereference(o).map_err(|e| e.to_string())?;
    resolved.as_dict().map_err(|e| e.to_string())
}

fn malformed(what: &str, detail: &str) -> EngineError {
    EngineError::Malformed {
        what: what.into(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    /// The source of this module, minus its tests.
    ///
    /// Split on the test module by name. Scanning "everything before the first `#[cfg(test)]`"
    /// is the spelling that made v1-S1's independence guard vacuous — that attribute also lands
    /// on `use` statements — and scanning the whole file would trip over this assertion's own
    /// text.
    fn module_code() -> String {
        let src = include_str!("structure.rs");
        let code = src
            .split("\n#[cfg(test)]\nmod tests {")
            .next()
            .expect("split always yields a first part");
        assert!(
            code.contains("pub fn read") && code.contains("fn check_against"),
            "the guard did not reach the walk and the comparison; it would pass regardless"
        );
        code.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_tagged_half_of_the_check_sees_no_geometry() {
        // The independence the tagged-versus-geometric check depends on. The tree walk derives
        // rows, columns and spans from `/S`, `/TR`, `/TD`, `/RowSpan` and `/ColSpan` — if a box
        // ever reaches it, the two halves stop being two halves and the check agrees with itself.
        //
        // Same shape as the v1-S2 fix to `SlotCover`'s guard: scan the region by name, and assert
        // the scan reached it.
        let code = module_code();
        for geometric in [
            "QRect",
            "QuantRect",
            "bbox",
            "GeometryPresence",
            "origin_x",
            "origin_y",
            "quantize",
        ] {
            assert!(
                !code.contains(geometric),
                "`{geometric}` reached the tagged half of the check. Its whole value is being an \
                 independent second opinion about a table, and a derivation that reads boxes is \
                 not independent of one built from boxes"
            );
        }
    }

    /// **A tagged cell claims the marked content the tree puts under it** (v1-S7b).
    ///
    /// This is what makes the table gate's gold text the author's rather than the detector's: the
    /// join key is the tree's own `/MCID`, so which text belongs to which cell is read off the
    /// tags and never off a box. Without it the labelled set can record a cell's shape and not
    /// what it says, which is exactly where v1-S7a had to stop.
    #[test]
    fn a_tagged_cell_claims_the_marked_content_beneath_it() {
        let bytes = crate::test_support::engine_fixture("tagged-table-agrees/document.pdf");
        let doc = lopdf::Document::load_mem(&bytes).expect("loads");
        let tree = super::read(&doc).expect("walks").expect("is tagged");
        let table = tree.tables.first().expect("the fixture declares a table");

        assert!(
            table.cells.iter().any(|c| !c.mcids.is_empty()),
            "no cell claimed any marked content, so a labelled cell could carry a shape and no \
             text — which is the hole v1-S7b exists to close"
        );
        // Every claimed id is one the tree really binds on that cell's page. A cell that claimed
        // an id nothing binds would resolve to empty text and read as a detector miss.
        for cell in &table.cells {
            for mcid in &cell.mcids {
                let page = cell
                    .page
                    .expect("a cell that claims content names its page");
                assert!(
                    tree.locator_for(page, *mcid).is_some(),
                    "cell ({}, {}) claims mcid {mcid} on a page the tree binds nothing for",
                    cell.row,
                    cell.column
                );
            }
        }
        // No id is claimed twice. The walk pushes a cell before its kids and pops after, so two
        // cells sharing content would mean the stack leaked — and the same text would be scored
        // as gold for two different slots.
        let mut seen = std::collections::BTreeSet::new();
        for cell in &table.cells {
            for mcid in &cell.mcids {
                assert!(
                    seen.insert((cell.page, *mcid)),
                    "mcid {mcid} is claimed by more than one cell; the open-cell stack leaked"
                );
            }
        }
    }

    #[test]
    fn the_walk_never_reorders_anything() {
        // v1-S3 attaches addresses. Reading the tree and then emitting nodes in `/K` order is a
        // READING-ORDER rule, and that is v1-S5's slice — a tagged two-column document must not
        // start reading column-major because this module walked its structure first.
        let code = module_code();
        for reordering in ["sort_by", "sort_unstable_by", "reverse()", "reading_order"] {
            assert!(
                !code.contains(reordering),
                "`{reordering}` in the structure walk. This module produces a lookup table keyed \
                 by (page, mcid); the node list is built elsewhere and stays in content-stream \
                 order until v1-S5 replaces the rule deliberately"
            );
        }
    }
}
