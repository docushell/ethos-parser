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

//! A drafting model's blocks in Markdown and HTML (decision #147): which pages its reading
//! replaces, which of its tables and charts are added and where, and how each is labelled.
//!
//! # The rule — the pin's `assist-lightonocr-v1`
//!
//! Decision #123's assist lane (`docs/measurements/parsebench/ethos_bench.py`), with three changes:
//! blocks go at their reading position, an exact duplicate of an engine table is left out, and
//! every insertion is labelled. Each page the record's `assist` key holds is one of two:
//!
//! - **Swapped**, where the engine read under 30% of the model's words: `model > 0`,
//!   `10 * shared < 3 * model` and `2 * model >= engine`, counted in decision #123's words (the
//!   distinct runs of letters, digits and `_`, two characters or more, lower-cased, tags removed).
//!   `model` counts the words of the blocks a swap would write — not a reply's cut last block, not
//!   furniture `drop_furniture` leaves out, not a table's text outside its cells, not a line the
//!   guard leaves out — so no page is
//!   swapped for words it would not show, nor to nothing; `engine` the page's share of the
//!   engine-only Markdown, a table's repeated header kept (decision #151). The engine's runs and tables on the page are left out, counted under
//!   [`REPLACED_BY_PROPOSED_PAGE`], and where its first node stood the model's blocks follow, in
//!   reply order.
//! - **Added to**: each `table` and `chart` block, with the `title` or `caption` block just before
//!   it unless every word of that is the engine's on the page, goes after the last run of the page,
//!   in reading order, that overlaps it across and starts above its foot, at the next block
//!   boundary; before the page's first node where no run does. A table whose cells, spans and rows
//!   equal those of an engine table it overlaps on half the smaller box or more is left out
//!   ([`PROPOSED_TABLE_DUPLICATE`]); otherwise both stand, the engine's first. Every other block
//!   stays in the record, counted ([`PROPOSED_BLOCK_NOT_PROJECTED`]).
//!
//! **The guard decides what is shown** (decision #146): a `contradicted` or `unsupported` line is
//! left out and counted, a table cell emptied rather than removed; `disputed`, unchecked and bound
//! lines are kept; a chart's `estimate` is kept and its label says so; a reply's cut last block is
//! left out ([`PROPOSED_BLOCK_TRUNCATED`]). Under `drop_furniture` a swapped page's `header`,
//! `footer`, `page_number`, `header_image` and `footer_image` blocks go too.
//!
//! **Model text is attacker-controlled, and contained.** Each group — a swapped page, or a table or
//! chart with its caption — opens with a label naming what it is and the model, and closes with
//! `End of proposed text.`; the model's blocks sit between the two, one quote deeper, so no model
//! text stands where either does. In Markdown the group is one block quote and the model's blocks
//! a quote inside it: every line of theirs, blank lines included, opens with `> >`, and the
//! model's text reaches them only through [`md_text`]: its character references decoded, U+0000
//! left out, then `&` and `<` written as references, so no HTML block, comment, tag or autolink
//! forms; every `[` and `]` after a `\`, so no link, image or link reference definition forms; a
//! run of backticks or tildes where a fence could open after a `\`; a `\r` a line break, so no
//! line stands outside the quote; code inside a fence longer than any run of backticks in it as
//! written, which reads nothing. In HTML the group is one `<blockquote>` and the model's blocks a
//! `<blockquote>` inside it, their text escaped by [`html_text`], U+0000 left out, a `&#0;` too;
//! the model's name is escaped as its text is, in both. A table is read into cells and written by this engine: a GFM
//! grid in Markdown, its spans flattened and counted, a `<table>` keeping them in HTML; one with no
//! cell, or spans laying it out on more than [`TABLE_SLOTS`] slots, is left out
//! ([`PROPOSED_TABLE_UNPARSED`]). In any text, tags other than `<sub>`, `<sup>`
//! and `<br>` are removed and those three written by this engine. A block's bytes are `proposed`
//! segments of the anchor map naming its `m` id, escaped as written: readable, never citable.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound;

use serde::{Deserialize, Serialize};

use crate::assist::{AssistBlock, AssistMark, AssistModel, AssistOutcome, AssistRecord};
use crate::markdown::{Emit, StructuralErasure};
use crate::{DocumentRepresentation, EngineError, NodeId, NodeKind, QRect};

/// Characters of the engine's runs on a page the model's reading replaced: the census's bucket.
pub const REPLACED_BY_PROPOSED_PAGE: &str = "replaced-by-proposed-page-v1";
/// A model table whose grid equals an engine table's it overlaps: left out.
pub const PROPOSED_TABLE_DUPLICATE: &str = "proposed-table-duplicate-v1";
/// A model line holding a value the page contradicts (the guard's `contradicted`): left out.
pub const PROPOSED_VALUE_CONTRADICTED: &str = "proposed-value-contradicted-v1";
/// A model line of words the page does not bear out (the guard's `unsupported`): left out.
pub const PROPOSED_LINE_UNSUPPORTED: &str = "proposed-line-unsupported-v1";
/// The last block of a reply the server cut at its cap: left out.
pub const PROPOSED_BLOCK_TRUNCATED: &str = "proposed-block-truncated-v1";
/// A model `table` block holding no cell this engine reads, or spans laying it out on more than
/// 100,000 slots: left out.
pub const PROPOSED_TABLE_UNPARSED: &str = "proposed-table-unparsed-v1";
/// A model block the rule does not project — off a swapped page, any but a table, a chart and the
/// caption before one; or empty — kept in the record.
pub const PROPOSED_BLOCK_NOT_PROJECTED: &str = "proposed-block-not-projected-v1";

/// What a projection holds of a drafting model's blocks (decision #147): present exactly where
/// the record holds an `assist` key, so an engine-only artifact's bytes do not move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedCensus {
    /// The rule that placed the blocks, the profile's `assist.rule`.
    pub rule: String,
    /// The model whose text the blocks are, named in every label.
    pub model: AssistModel,
    /// One line per page the record holds a reply for, in page order.
    pub pages: Vec<ProposedPage>,
}

/// One page's line of a [`ProposedCensus`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedPage {
    /// The page.
    pub page: NodeId,
    /// How many distinct words the blocks a swap would write hold.
    pub model_words: usize,
    /// How many the engine-only Markdown holds of this page, a table's header row the next page
    /// repeats kept (decision #151).
    pub engine_words: usize,
    /// How many are in both.
    pub shared_words: usize,
    /// Whether the model's reading replaced the engine's here.
    pub swapped: bool,
    /// How many of the model's blocks this artifact writes.
    pub blocks: usize,
    /// What of the reply this artifact leaves out, by code, sorted: blocks and lines; and, in
    /// Markdown, `gfm-span-slots-unrepresentable-v1` for the slots a model table's spans covered.
    pub left_out: Vec<StructuralErasure>,
}

/// Add `n` to `code` on `page`'s line.
fn count(page: &mut ProposedPage, code: &str, n: usize) {
    if n == 0 {
        return;
    }
    match page
        .left_out
        .binary_search_by(|e| e.code.as_str().cmp(code))
    {
        Ok(at) => page.left_out[at].count += n,
        Err(at) => page.left_out.insert(
            at,
            StructuralErasure {
                code: code.into(),
                count: n,
            },
        ),
    }
}

/// A stretch of a block's text as a projection writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Piece {
    /// The model's text, its character references decoded: written only by [`md_text`] and
    /// [`html_text`], which escape it.
    Text(String),
    /// `<sub>`, `</sub>`, `<sup>`, `</sup>` or `<br>`, written by this engine.
    Tag(&'static str),
}

/// One cell of a model table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Cell {
    header: bool,
    rowspan: usize,
    colspan: usize,
    text: Vec<Piece>,
    /// The guard's mark on the cell's line, if any.
    mark: Option<AssistMark>,
}

/// A block as a projection writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Shown<'a> {
    /// A `title`: its heading level, 1 unless its text opens with `#`s.
    Heading(&'a str, usize, Vec<Piece>),
    /// Text.
    Paragraph(&'a str, Vec<Piece>),
    /// A `formula`, its outer `$$` or `$` taken off.
    Formula(&'a str, Vec<Piece>),
    /// `code`, the model's own fence, if it wrote one, taken off.
    Code(&'a str, Vec<Piece>),
    /// A `table`, or a `chart` holding one.
    Table(&'a str, Vec<Vec<Cell>>),
}

/// What a group of blocks is, for its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum What {
    /// A swapped page, by its number, `true` where a chart's value on it is an estimate.
    Page(u32, bool),
    /// A table.
    Table,
    /// A chart, `true` where a value is an estimate.
    Chart(bool),
}

impl What {
    fn label(self) -> String {
        match self {
            What::Page(n, false) => format!("page {n} as read"),
            What::Page(n, true) => format!("page {n} as read, chart values estimated"),
            What::Table => "table".into(),
            What::Chart(false) => "chart".into(),
            What::Chart(true) => "chart values estimated".into(),
        }
    }
}

/// Blocks written together under one label.
#[derive(Debug)]
pub(crate) struct Group<'a> {
    /// Its page's line in the census.
    census: usize,
    what: What,
    blocks: Vec<Shown<'a>>,
}

/// Where a record's proposed blocks go: computed once from the record, read by both projections.
pub(crate) struct Plan<'a> {
    record: &'a AssistRecord,
    /// Every node of a swapped page: the projections leave them out.
    pub(crate) replaced: BTreeSet<&'a str>,
    page_index: BTreeMap<&'a str, u32>,
    /// Groups written before a page's first node, by page number.
    start: BTreeMap<u32, Vec<Group<'a>>>,
    /// Groups written at the first block boundary after a node, by node index.
    after: BTreeMap<usize, Vec<Group<'a>>>,
    census: Vec<ProposedPage>,
}

/// Labels a `drop_furniture` projection leaves out of a swapped page.
const FURNITURE: [&str; 5] = [
    "header",
    "footer",
    "page_number",
    "header_image",
    "footer_image",
];

impl<'a> Plan<'a> {
    /// The plan for `repr`, or `None` where it holds no `assist` key.
    ///
    /// # Errors
    ///
    /// As [`crate::to_markdown`], whose engine-only projection the swap test reads.
    pub(crate) fn of(
        repr: &'a DocumentRepresentation,
        options: crate::ProjectionOptions,
    ) -> Result<Option<Self>, EngineError> {
        let p = repr.payload();
        let Some(record) = &p.assist else {
            return Ok(None);
        };
        let engine = engine_words(repr)?;
        // The engine's tables as the projection writes them, with their boxes, by page: a
        // repeated header left out unless kept (decision #151, review).
        let mut erasures = BTreeMap::new();
        let tables = crate::markdown::plan_tables(
            repr,
            &mut erasures,
            &BTreeSet::new(),
            !options.keep_repeated_header,
        );
        let pitch = crate::markdown::pitch_reference(&p.nodes);
        let stretches = crate::markdown::read_stretches(&p.nodes, &pitch);
        let mut grids: BTreeMap<&str, Vec<(QRect, Grid)>> = BTreeMap::new();
        for (table, plan) in p.tables.iter().zip(&tables) {
            if let (true, Some(rect)) = (plan.projected, table.geometry.measured()) {
                grids
                    .entry(table.page.as_str())
                    .or_default()
                    .push((rect, engine_grid(plan, &pitch, &stretches)));
            }
        }
        // The measured runs of each page, in reading order, a block is placed after.
        let mut runs: BTreeMap<&str, Vec<(usize, QRect)>> = BTreeMap::new();
        for (i, node) in p.nodes.iter().enumerate() {
            if let (NodeKind::TextRun, Some(rect)) =
                (node.kind, repr.geometry_at(i).and_then(|g| g.measured()))
            {
                runs.entry(node.parent.as_str())
                    .or_default()
                    .push((i, rect));
            }
        }
        let mut plan = Plan {
            record,
            replaced: BTreeSet::new(),
            page_index: p.pages.iter().map(|r| (r.id.as_str(), r.index)).collect(),
            start: BTreeMap::new(),
            after: BTreeMap::new(),
            census: Vec::with_capacity(record.pages.len()),
        };
        let none = BTreeSet::new();
        for page in &record.pages {
            // The seal holds every page declared.
            let Some(at) = p.pages.iter().find(|r| r.id == page.page) else {
                continue;
            };
            let frame = crate::assist::frame_of(at);
            // A reply the server cut ends inside its last block (guard rule 4).
            let cut = matches!(page.outcome, AssistOutcome::Truncated | AssistOutcome::Loop)
                .then(|| page.blocks.len().checked_sub(1))
                .flatten();
            // The blocks a swap would write: not the cut one, nor furniture left out on request.
            let kept = |k: usize, b: &AssistBlock| {
                Some(k) != cut
                    && !(options.drop_furniture
                        && b.label.as_deref().is_some_and(|l| FURNITURE.contains(&l)))
            };
            let texts: Vec<String> = (0..)
                .zip(&page.blocks)
                .filter(|(k, b)| kept(*k, b) && matches!(render(b), Rendered::Shown(..)))
                .map(|(_, b)| written(b))
                .collect();
            let model = crate::assist::words(&crate::assist::untagged(&texts.join("\n\n")));
            let ours = engine.get(page.page.as_str()).unwrap_or(&none);
            let shared = model.intersection(ours).count();
            // No word written is no swap, `10 * 0 < 3 * 0` failing: never a swap to nothing.
            let swapped = 10 * shared < 3 * model.len() && 2 * model.len() >= ours.len();
            let census = plan.census.len();
            let mut line = ProposedPage {
                page: page.page.clone(),
                model_words: model.len(),
                engine_words: ours.len(),
                shared_words: shared,
                swapped,
                blocks: 0,
                left_out: Vec::new(),
            };
            count(
                &mut line,
                PROPOSED_BLOCK_TRUNCATED,
                usize::from(cut.is_some()),
            );
            let mut taken = usize::from(cut.is_some());
            if swapped {
                let (mut blocks, mut estimates) = (Vec::new(), false);
                for (k, b) in (0..).zip(&page.blocks) {
                    if !kept(k, b) {
                        continue;
                    }
                    if let Some(s) = shown(b, &mut line, &mut taken) {
                        estimates |= estimated(b);
                        blocks.push(s);
                    }
                }
                plan.start.entry(at.index).or_default().push(Group {
                    census,
                    what: What::Page(at.index, estimates),
                    blocks,
                });
                plan.replaced.extend(
                    p.nodes
                        .iter()
                        .filter(|n| n.parent == page.page)
                        .map(|n| n.id.as_str()),
                );
            } else {
                let on_page = runs.get(page.page.as_str()).map_or(&[][..], Vec::as_slice);
                let engine_grids = grids.get(page.page.as_str()).map_or(&[][..], Vec::as_slice);
                for (k, b) in page.blocks.iter().enumerate() {
                    let what = match b.label.as_deref() {
                        Some("table") => What::Table,
                        Some("chart") => What::Chart(estimated(b)),
                        _ => continue,
                    };
                    if Some(k) == cut {
                        continue;
                    }
                    let model_box = b
                        .grid
                        .map(|g| crate::assist::placed(crate::assist::upright(g, at.turn), frame));
                    let duplicate = model_box.is_some_and(|m| {
                        let model = model_grid(&b.text);
                        engine_grids
                            .iter()
                            .any(|(rect, grid)| *grid == model && overlaps(m, *rect))
                    });
                    if duplicate {
                        count(&mut line, PROPOSED_TABLE_DUPLICATE, 1);
                        taken += 1;
                        continue;
                    }
                    let Some(block) = shown(b, &mut line, &mut taken) else {
                        continue;
                    };
                    let mut blocks = Vec::new();
                    if let Some(c) = k.checked_sub(1) {
                        let before = &page.blocks[c];
                        let caption = matches!(before.label.as_deref(), Some("title" | "caption"))
                            && !crate::assist::words(&crate::assist::untagged(&written(before)))
                                .is_subset(ours);
                        if caption {
                            blocks.extend(shown(before, &mut line, &mut taken));
                        }
                    }
                    blocks.push(block);
                    let group = Group {
                        census,
                        what,
                        blocks,
                    };
                    // After the last run, in reading order, that overlaps the block across and
                    // starts above its foot; before the page's first node where none does.
                    let anchor = model_box.and_then(|[x0, _, x1, y1]| {
                        on_page
                            .iter()
                            .filter(|(_, r)| {
                                let scaled = |v: i64| 1000 * i128::from(v);
                                scaled(r.x0()) < x1 && scaled(r.x1()) > x0 && scaled(r.y0()) < y1
                            })
                            .map(|(i, _)| *i)
                            .max()
                    });
                    match anchor {
                        Some(i) => plan.after.entry(i).or_default().push(group),
                        None => plan.start.entry(at.index).or_default().push(group),
                    }
                }
            }
            let left = page.blocks.len().saturating_sub(taken);
            count(&mut line, PROPOSED_BLOCK_NOT_PROJECTED, left);
            plan.census.push(line);
        }
        Ok(Some(plan))
    }

    /// A cursor over the plan, for one projection's pass.
    pub(crate) fn cursor(&self) -> Cursor<'_, 'a> {
        Cursor {
            plan: self,
            node: 0,
            page: 0,
            pending: Vec::new(),
            census: self.census.clone(),
        }
    }
}

/// `b` as written, its left-out lines counted on `line`, and `taken` counting it where it is
/// written or counted under a code of its own; `None` where nothing of it is written.
fn shown<'a>(b: &'a AssistBlock, line: &mut ProposedPage, taken: &mut usize) -> Option<Shown<'a>> {
    match render(b) {
        Rendered::Shown(s, contradicted, unsupported) => {
            count(line, PROPOSED_VALUE_CONTRADICTED, contradicted);
            count(line, PROPOSED_LINE_UNSUPPORTED, unsupported);
            *taken += 1;
            Some(s)
        }
        Rendered::Unparsed => {
            count(line, PROPOSED_TABLE_UNPARSED, 1);
            *taken += 1;
            None
        }
        Rendered::Blank => None,
    }
}

/// The text of `b` a projection writes, as the swap test and the caption test read it: a table's
/// cells, not what lies outside them; any other block whole; neither a line the guard leaves out
/// (review). The model's text as written, so it is read as the engine's Markdown is.
fn written(b: &AssistBlock) -> String {
    let dropped = dropped(b);
    if is_table(b) {
        let cells: Vec<&str> = numbered(&b.text)
            .filter(|(line, ..)| !dropped.contains(line))
            .map(|(.., part)| part)
            .collect();
        cells.join("\n")
    } else if dropped.is_empty() {
        b.text.clone()
    } else {
        kept_lines(b, &dropped)
    }
}

/// The lines of `b` the guard leaves out: `contradicted` and `unsupported`.
fn dropped(b: &AssistBlock) -> BTreeSet<u32> {
    b.guard
        .iter()
        .flat_map(|g| &g.flagged)
        .filter(|f| matches!(f.mark, AssistMark::Contradicted | AssistMark::Unsupported))
        .map(|f| f.line)
        .collect()
}

/// A table block's cells, `(line, row, tag, text)`, each line numbered as the guard numbers it:
/// text outside every cell counts its own lines, and is no cell.
fn numbered(text: &str) -> impl Iterator<Item = (u32, u32, &str, &str)> {
    let mut line = 0u32;
    crate::assist::cells_of(text)
        .into_iter()
        .filter_map(move |(row, tag, part)| {
            let Some(row) = row else {
                line += u32::try_from(crate::assist::text_lines(part).len()).unwrap_or(u32::MAX);
                return None;
            };
            line += 1;
            Some((line, row, tag, part))
        })
}

/// `b`'s lines as the guard read them, references decoded, but those in `dropped`.
fn kept_lines(b: &AssistBlock, dropped: &BTreeSet<u32>) -> String {
    let kept: Vec<String> = crate::assist::block_lines(b.label.as_deref(), &b.text)
        .into_iter()
        .zip(1u32..)
        .filter(|(_, line)| !dropped.contains(line))
        .map(|((_, text), _)| text)
        .collect();
    kept.join("\n")
}

/// Whether `b` is a chart a value of which is an estimate.
fn estimated(b: &AssistBlock) -> bool {
    b.label.as_deref() == Some("chart")
        && b.guard
            .as_ref()
            .is_some_and(|g| g.flagged.iter().any(|f| f.mark == AssistMark::Estimate))
}

/// Whether `b` is written as a table: a `table` or `chart` holding one.
fn is_table(b: &AssistBlock) -> bool {
    matches!(b.label.as_deref(), Some("table" | "chart"))
        && b.text.to_ascii_lowercase().contains("<table")
}

/// Whether a model box (as [`crate::assist::placed`] gives it) and an engine table's overlap on half the smaller
/// box or more.
fn overlaps(model: [i128; 4], table: QRect) -> bool {
    let t = [table.x0(), table.y0(), table.x1(), table.y1()].map(|v| 1000 * i128::from(v));
    let area = |b: [i128; 4]| (b[2] - b[0]).max(0) * (b[3] - b[1]).max(0);
    let inter = [
        model[0].max(t[0]),
        model[1].max(t[1]),
        model[2].min(t[2]),
        model[3].min(t[3]),
    ];
    2 * area(inter) >= area(model).min(area(t)) && area(inter) > 0
}

/// A table's grid as compared for de-duplication: each row's cells, in order, as text with its
/// whitespace collapsed, and their row and column spans.
type Grid = Vec<Vec<(String, usize, usize)>>;

/// An engine table's grid, its cells' runs joined as the projections join them.
fn engine_grid(
    plan: &crate::markdown::TablePlan<'_>,
    pitch: &crate::markdown::PitchReference,
    stretches: &crate::markdown::Stretches<'_>,
) -> Grid {
    (0..plan.rows)
        .map(|row| {
            (0..plan.columns)
                .filter_map(|column| {
                    let index = row * plan.columns + column;
                    let (rowspan, colspan) = match plan.roles[index] {
                        crate::markdown::SlotRole::Covered => return None,
                        crate::markdown::SlotRole::Origin { rowspan, colspan } => {
                            (rowspan, colspan)
                        }
                        crate::markdown::SlotRole::Empty => (1, 1),
                    };
                    let mut text = String::new();
                    let mut prev: Option<&crate::Node> = None;
                    for node in &plan.slots[index] {
                        let t = crate::markdown::normalize(node.reading_text());
                        let abuts = prev.is_some_and(|prev| {
                            crate::markdown::cell_runs_abut(prev, node, pitch, stretches)
                        });
                        prev = Some(node);
                        if !t.is_empty() {
                            if !text.is_empty() && !abuts {
                                text.push(' ');
                            }
                            text.push_str(&t);
                        }
                    }
                    Some((text, rowspan, colspan))
                })
                .collect()
        })
        .collect()
}

/// A model table's grid as written, for de-duplication: its cells' text with tags removed,
/// references decoded and whitespace collapsed.
fn model_grid(html: &str) -> Grid {
    let mut grid: Grid = Vec::new();
    let mut last = None;
    for (row, tag, part) in crate::assist::cells_of(html) {
        let Some(row) = row else {
            continue;
        };
        if last != Some(row) {
            grid.push(Vec::new());
            last = Some(row);
        }
        let text = crate::assist::collapsed(&decoded(&crate::assist::untagged(part)));
        if let Some(cells) = grid.last_mut() {
            cells.push((text, span(tag, "rowspan"), span(tag, "colspan")));
        }
    }
    grid
}

/// A cell tag's `rowspan` or `colspan`, 1 where absent or unreadable, at most 1,000.
fn span(tag: &str, name: &str) -> usize {
    let lower = tag.to_ascii_lowercase();
    lower
        .find(name)
        .and_then(|at| {
            let rest = lower[at + name.len()..].trim_start().strip_prefix('=')?;
            let rest = rest.trim_start().trim_start_matches(['"', '\'']);
            let digits = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..digits.min(4)].parse::<usize>().ok()
        })
        .unwrap_or(1)
        .clamp(1, 1000)
}

/// The model's text with its character references decoded, `&dollar;` among them, as an HTML
/// reader shows it.
fn decoded(text: &str) -> String {
    crate::assist::decoded(&text.replace("&dollar;", "$"))
}

/// The page's words in the engine-only Markdown, by page id: its `source` segments of that page's
/// runs and the syntax after them, read as decision #123 read the Markdown. A table's header row
/// the next page repeats is kept there, as #123's Markdown kept it: the engine read it, whether or
/// not a projection writes it (decision #151, review).
fn engine_words(
    repr: &DocumentRepresentation,
) -> Result<BTreeMap<&str, BTreeSet<String>>, EngineError> {
    let p = repr.payload();
    let artifact = crate::markdown::markdown_of(
        repr,
        &p.identity.parser_version,
        &p.identity.profile_sha256,
        "",
        crate::ProjectionOptions {
            keep_repeated_header: true,
            ..crate::ProjectionOptions::default()
        },
        None,
    )?;
    let page_of: BTreeMap<&str, &str> = p
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n.parent.as_str()))
        .collect();
    let mut texts: BTreeMap<&str, String> = BTreeMap::new();
    let mut current: Option<&str> = None;
    for s in &artifact.anchor_map.segments {
        let bytes = &artifact.markdown[s.start..s.end];
        if s.kind == crate::SegmentKind::Source {
            let page = s.node_ids.first().and_then(|id| page_of.get(id.as_str()));
            if page.copied() != current {
                current = page.copied();
                if let Some(page) = current {
                    texts.entry(page).or_default().push('\n');
                }
            }
        }
        if let Some(page) = current {
            texts.entry(page).or_default().push_str(bytes);
        }
    }
    Ok(texts
        .into_iter()
        .map(|(page, text)| (page, crate::assist::words(&crate::assist::untagged(&text))))
        .collect())
}

/// What a block renders to.
enum Rendered<'a> {
    /// Written, with how many of its lines the guard's marks left out: contradicted, unsupported.
    Shown(Shown<'a>, usize, usize),
    /// A `table` holding no cell.
    Unparsed,
    /// Nothing to write.
    Blank,
}

/// A block as the projections write it (see the module documentation).
fn render(b: &AssistBlock) -> Rendered<'_> {
    let id = b.id.as_str();
    let marks: BTreeMap<u32, AssistMark> = b
        .guard
        .iter()
        .flat_map(|g| &g.flagged)
        .map(|f| (f.line, f.mark))
        .collect();
    let dropped = dropped(b);
    let tally = |mark: AssistMark| marks.values().filter(|m| **m == mark).count();
    let (contradicted, unsupported) = (
        tally(AssistMark::Contradicted),
        tally(AssistMark::Unsupported),
    );
    let label = b.label.as_deref();
    if is_table(b) {
        // A cell is one of the guard's lines; text outside every cell counts its own lines and is
        // not written: decision #123's replies hold none outside a closed table.
        let mut rows: Vec<Vec<Cell>> = Vec::new();
        let mut last = None;
        for (line, row, tag, part) in numbered(&b.text) {
            if last != Some(row) {
                rows.push(Vec::new());
                last = Some(row);
            }
            let mark = marks.get(&line).copied();
            let cell = Cell {
                header: tag.to_ascii_lowercase().starts_with("<th"),
                rowspan: span(tag, "rowspan"),
                colspan: span(tag, "colspan"),
                text: if dropped.contains(&line) {
                    Vec::new()
                } else {
                    pieces(part, true)
                },
                mark,
            };
            if let Some(cells) = rows.last_mut() {
                cells.push(cell);
            }
        }
        rows.retain(|r| !r.is_empty());
        return if rows.is_empty() || !fits(&rows) {
            Rendered::Unparsed
        } else {
            Rendered::Shown(Shown::Table(id, rows), contradicted, unsupported)
        };
    }
    if label == Some("table") {
        return Rendered::Unparsed;
    }
    // Lines the guard left out: the rest, as it read them, references decoded.
    let plain = (!dropped.is_empty()).then(|| vec![Piece::Text(kept_lines(b, &dropped))]);
    // Code and a formula are not read for tags: they are written whole, references decoded.
    let mut text = match (label, plain) {
        (_, Some(plain)) => plain,
        (Some("code"), None) => vec![Piece::Text(decoded(unfenced(&b.text)))],
        (Some("formula"), None) => vec![Piece::Text(decoded(&formula(&b.text)))],
        (_, None) => pieces(&b.text, false),
    };
    // A title keeps the model's own heading level, as ParseBench's `_render_block` does; one
    // with none is a level-1 heading.
    let mut level = 1;
    if let (Some("title"), Some(Piece::Text(first))) = (label, text.first_mut()) {
        let rest = first.trim_start_matches([' ', '\t']);
        let marks = rest.len() - rest.trim_start_matches('#').len();
        let after = &rest[marks..];
        if (1..=6).contains(&marks) && after.starts_with([' ', '\t']) {
            level = marks;
            *first = after.trim_start_matches([' ', '\t']).to_string();
        }
    }
    // A space between two tags stays: `x<sup>2</sup> <sup>3</sup>` (review).
    text.retain(|p| !matches!(p, Piece::Text(t) if t.is_empty()));
    if text
        .iter()
        .all(|p| !matches!(p, Piece::Text(t) if !t.trim().is_empty()))
    {
        return Rendered::Blank;
    }
    let shown = match label {
        Some("code") => Shown::Code(id, text),
        Some("formula") => Shown::Formula(id, text),
        Some("title") => Shown::Heading(id, level, text),
        _ => Shown::Paragraph(id, text),
    };
    Rendered::Shown(shown, contradicted, unsupported)
}

/// ParseBench's `_strip_formula_delimiters`: one outer `$$` or `$` pair taken off where nothing
/// else inside is a `$`.
fn formula(text: &str) -> String {
    let stripped = text.trim();
    for delim in ["$$", "$"] {
        if stripped.chars().count() > 2 * delim.len()
            && stripped.starts_with(delim)
            && stripped.ends_with(delim)
        {
            let interior = &stripped[delim.len()..stripped.len() - delim.len()];
            if !interior.contains('$') && !interior.trim().is_empty() {
                return interior.trim().to_string();
            }
        }
    }
    text.to_string()
}

/// Code as the model wrote it, its own fence taken off where it wrote one: an opening line of
/// backticks and what follows them, and a closing one. This engine fences it.
fn unfenced(text: &str) -> &str {
    let text = text.trim_end();
    if !text.trim_start().starts_with("```") {
        return text;
    }
    let body = text.split_once('\n').map_or("", |(_, body)| body);
    body.strip_suffix("```").map_or(body, str::trim_end)
}

/// Inline tags a model writes that are removed with nothing in their place.
const INLINE: [&str; 13] = [
    "a", "b", "code", "del", "em", "font", "i", "ins", "mark", "s", "small", "span", "strong",
];

/// The model's text in pieces: `<sub>`, `<sup>` and `<br>` as this engine writes them, balanced;
/// an inline tag removed, any other tag left as a space; the text between, its references
/// decoded. A tag is `<`, a name starting with a letter and ended by a space, `/` or `>`, then the
/// next `>`. U+0000 is left out. In a cell, whitespace is collapsed. What a removed tag leaves —
/// `<im<b>g …>` is `<img …>` — is text, escaped where it is written.
fn pieces(text: &str, cell: bool) -> Vec<Piece> {
    let text = text.replace('\0', "");
    let mut out = Vec::new();
    let mut run = String::new();
    let mut open: Option<&'static str> = None;
    let flush = |run: &mut String, out: &mut Vec<Piece>| {
        if !run.is_empty() {
            out.push(Piece::Text(decoded(&std::mem::take(run))));
        }
    };
    let mut rest = text.as_str();
    while let Some(at) = rest.find('<') {
        run.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let closing = after.starts_with('/');
        let named = &after[usize::from(closing)..];
        let len = named
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(named.len());
        let tag = named.starts_with(|c: char| c.is_ascii_alphabetic())
            && named[len..].starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>');
        let Some(end) = after.find('>').filter(|_| tag) else {
            run.push('<');
            rest = after;
            continue;
        };
        match named[..len].to_ascii_lowercase().as_str() {
            name @ ("sub" | "sup") => {
                let (which, close) = if name == "sub" {
                    ("<sub>", "</sub>")
                } else {
                    ("<sup>", "</sup>")
                };
                if !closing {
                    flush(&mut run, &mut out);
                    if let Some(open) = open.take() {
                        out.push(Piece::Tag(open));
                    }
                    out.push(Piece::Tag(which));
                    open = Some(close);
                } else if open == Some(close) {
                    flush(&mut run, &mut out);
                    out.push(Piece::Tag(close));
                    open = None;
                }
            }
            "br" => {
                flush(&mut run, &mut out);
                out.push(Piece::Tag("<br>"));
            }
            name if INLINE.contains(&name) => {}
            _ => run.push(' '),
        }
        rest = &after[end + 1..];
    }
    run.push_str(rest);
    flush(&mut run, &mut out);
    if let Some(close) = open {
        out.push(Piece::Tag(close));
    }
    if cell {
        for p in &mut out {
            if let Piece::Text(t) = p {
                let mut squeezed = String::with_capacity(t.len());
                for c in t.chars() {
                    if !c.is_whitespace() {
                        squeezed.push(c);
                    } else if !squeezed.ends_with(' ') {
                        squeezed.push(' ');
                    }
                }
                *t = squeezed;
            }
        }
    }
    if let Some(Piece::Text(t)) = out.first_mut() {
        *t = t.trim_start().to_string();
    }
    if let Some(Piece::Text(t)) = out.last_mut() {
        *t = t.trim_end().to_string();
    }
    out.retain(|p| !matches!(p, Piece::Text(t) if t.is_empty()));
    out
}

/// A plan's groups, met in a projection's order: [`Cursor::at`] at the top of each node, the
/// pending groups written at each block boundary, and the census kept as they are.
pub(crate) struct Cursor<'p, 'a> {
    plan: &'p Plan<'a>,
    /// The next node whose groups are not yet pending.
    node: usize,
    /// The last page whose opening groups are pending.
    page: u32,
    pending: Vec<&'p Group<'a>>,
    census: Vec<ProposedPage>,
}

impl<'p, 'a> Cursor<'p, 'a> {
    /// At the top of node `i` of `page`: the groups after the nodes before it, and before every
    /// page up to its own, are pending.
    pub(crate) fn at(&mut self, i: usize, page: &NodeId) {
        let index = self
            .plan
            .page_index
            .get(page.as_str())
            .copied()
            .unwrap_or(self.page);
        self.reach(i, index);
    }

    /// The end of the nodes: every group not yet written is pending.
    pub(crate) fn end(&mut self) {
        self.reach(usize::MAX, u32::MAX);
    }

    fn reach(&mut self, node: usize, page: u32) {
        if node > self.node {
            self.pending
                .extend(self.plan.after.range(self.node..node).flat_map(|(_, g)| g));
            self.node = node;
        }
        if page > self.page {
            let pages = (Bound::Excluded(self.page), Bound::Included(page));
            self.pending
                .extend(self.plan.start.range(pages).flat_map(|(_, g)| g));
            self.page = page;
        }
    }

    /// The groups to write at this block boundary, in order.
    pub(crate) fn take(&mut self) -> Vec<&'p Group<'a>> {
        std::mem::take(&mut self.pending)
    }

    /// The census, once every group is written.
    pub(crate) fn finish(self) -> ProposedCensus {
        ProposedCensus {
            rule: crate::assist::ASSIST_RULE_V1.into(),
            model: self.plan.record.model.clone(),
            pages: self.census,
        }
    }

    /// The label's model: its id and version, whitespace collapsed so the label stays one line.
    fn model(&self) -> (String, String) {
        let m = &self.plan.record.model;
        (
            crate::assist::collapsed(&m.id),
            crate::assist::collapsed(&m.version),
        )
    }

    /// Write `group` as Markdown, after the separator the caller wrote: one block quote, its label
    /// first and its end line last, and between them the model's blocks one quote deeper, every
    /// line opening with `> >`, so no model text stands where those two lines stand.
    pub(crate) fn markdown(&mut self, e: &mut Emit, group: &Group<'_>) {
        let (model, version) = self.model();
        e.syntax(&format!(
            "> **Proposed:** {} by {}@{}; model text, not read from the document.",
            group.what.label(),
            md_escaped(&model, In::Name),
            md_escaped(&version, In::Name)
        ));
        let line = &mut self.census[group.census];
        e.syntax("\n>");
        for (k, block) in group.blocks.iter().enumerate() {
            e.syntax(if k == 0 { INNER } else { "\n> >\n> > " });
            match block {
                Shown::Heading(id, level, text) => {
                    e.syntax(&"#".repeat(*level));
                    e.syntax(" ");
                    md_pieces(e, text, id, In::Text);
                }
                Shown::Paragraph(id, text) => md_pieces(e, text, id, In::Text),
                Shown::Formula(id, text) => {
                    e.syntax(&format!("$${INNER}"));
                    md_pieces(e, text, id, In::Text);
                    e.syntax(&format!("{INNER}$$"));
                }
                Shown::Code(id, text) => {
                    let fence = fence(text);
                    e.syntax(&format!("{fence}{INNER}"));
                    md_pieces(e, text, id, In::Code);
                    e.syntax(&format!("{INNER}{fence}"));
                }
                Shown::Table(id, rows) => {
                    let (grid, columns, flattened) = layout(rows);
                    count(
                        line,
                        crate::markdown::GFM_SPAN_SLOTS_UNREPRESENTABLE,
                        flattened,
                    );
                    for (r, slots) in grid.iter().enumerate() {
                        if r > 0 {
                            e.syntax(INNER);
                        }
                        e.syntax("|");
                        for column in 0..columns {
                            e.syntax(" ");
                            if let Some(Some(Some(cell))) = slots.get(column) {
                                md_pieces(e, &cell.text, id, In::Cell);
                            }
                            e.syntax(" |");
                        }
                        if r == 0 {
                            e.syntax(&format!("{INNER}|"));
                            for _ in 0..columns {
                                e.syntax(" --- |");
                            }
                        }
                    }
                }
            }
            line.blocks += 1;
        }
        e.syntax(&format!("\n>\n> **{END_TEXT}**"));
    }

    /// Write `group` as HTML: one `<blockquote>` naming the model, its label a `<p>` first and its
    /// end line a `<p>` last, and between them the model's blocks in a `<blockquote>` of their own,
    /// so a browser indents them, and them further, with no style sheet.
    pub(crate) fn html(&mut self, e: &mut Emit, group: &Group<'_>) {
        let (model, version) = self.model();
        let attr = |s: &str| html_escaped(s).replace('"', "&quot;");
        e.syntax(&format!(
            "<blockquote class=\"ethos-proposed\" data-derivation=\"proposed\" \
             data-model=\"{}\" data-model-version=\"{}\">\n<p><strong>Proposed:</strong> {} by \
             {}@{}; model text, not read from the document.</p>\n<blockquote>\n",
            attr(&model),
            attr(&version),
            group.what.label(),
            html_escaped(&model),
            html_escaped(&version),
        ));
        let line = &mut self.census[group.census];
        for block in &group.blocks {
            let (open, close) = match block {
                Shown::Heading(_, level, _) => (format!("<h{level}>"), format!("</h{level}>\n")),
                Shown::Paragraph(..) => ("<p>".into(), "</p>\n".into()),
                Shown::Formula(..) => ("<p>$$".into(), "$$</p>\n".into()),
                Shown::Code(..) => ("<pre><code>".into(), "</code></pre>\n".into()),
                Shown::Table(..) => ("<table>\n".into(), "</table>\n".into()),
            };
            e.syntax(&open);
            match block {
                Shown::Heading(id, _, text)
                | Shown::Paragraph(id, text)
                | Shown::Formula(id, text)
                | Shown::Code(id, text) => html_pieces(e, text, id),
                Shown::Table(id, rows) => {
                    for row in rows {
                        e.syntax("<tr>\n");
                        for cell in row {
                            let tag = if cell.header { "th" } else { "td" };
                            let mut open = format!("<{tag}");
                            if cell.rowspan != 1 {
                                open.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
                            }
                            if cell.colspan != 1 {
                                open.push_str(&format!(" colspan=\"{}\"", cell.colspan));
                            }
                            match cell.mark {
                                Some(AssistMark::Estimate) => {
                                    open.push_str(" data-estimate=\"true\"");
                                }
                                Some(AssistMark::Contradicted) => {
                                    open.push_str(" data-dropped=\"contradicted\"");
                                }
                                Some(AssistMark::Unsupported) => {
                                    open.push_str(" data-dropped=\"unsupported\"");
                                }
                                _ => {}
                            }
                            open.push('>');
                            e.syntax(&open);
                            html_pieces(e, &cell.text, id);
                            e.syntax(&format!("</{tag}>\n"));
                        }
                        e.syntax("</tr>\n");
                    }
                }
            }
            e.syntax(&close);
            line.blocks += 1;
        }
        e.syntax(&format!(
            "</blockquote>\n<p><strong>{END_TEXT}</strong></p>\n</blockquote>\n"
        ));
    }
}

/// A model table's slots, row by row: each the cell starting there, `Some(None)` where a span
/// covers it, `None` where nothing is; with the grid's width and how many slots spans covered.
type Slots<'c> = Vec<Vec<Option<Option<&'c Cell>>>>;

/// The most slots a model table's grid may hold. Past it the table is not written (review): its
/// spans could ask gigabytes of [`layout`] and of Markdown. ParseBench's largest holds 3,326.
const TABLE_SLOTS: usize = 100_000;

/// Whether `rows` lay out on at most [`TABLE_SLOTS`] slots. No row of the grid is wider than the
/// columns its own cells and the spans from above reaching it ask for together, so the rows times
/// the widest of those bound it, with nothing allocated.
fn fits(rows: &[Vec<Cell>]) -> bool {
    let mut ends = vec![0usize; rows.len() + 1];
    let (mut width, mut widest) = (0usize, 0usize);
    for (r, row) in rows.iter().enumerate() {
        width -= ends[r];
        for cell in row {
            width += cell.colspan;
            ends[(r + cell.rowspan).min(rows.len())] += cell.colspan;
        }
        widest = widest.max(width);
    }
    rows.len().saturating_mul(widest) <= TABLE_SLOTS
}

/// Lay a model table's cells out on a grid, as an HTML table lays out: each row's cells from the
/// left, past slots a span from above already holds; a row span reaching past the last row stops
/// there.
fn layout(rows: &[Vec<Cell>]) -> (Slots<'_>, usize, usize) {
    let mut grid: Slots<'_> = vec![Vec::new(); rows.len()];
    let mut flattened = 0;
    for (r, row) in rows.iter().enumerate() {
        let mut c = 0;
        for cell in row {
            while grid[r].get(c).is_some_and(Option::is_some) {
                c += 1;
            }
            let rowspan = cell.rowspan.min(rows.len() - r);
            for (dr, slots) in grid[r..r + rowspan].iter_mut().enumerate() {
                if slots.len() < c + cell.colspan {
                    slots.resize(c + cell.colspan, None);
                }
                for (dc, slot) in slots[c..c + cell.colspan].iter_mut().enumerate() {
                    *slot = Some((dr == 0 && dc == 0).then_some(cell));
                }
            }
            flattened += rowspan * cell.colspan - 1;
            c += cell.colspan;
        }
    }
    let columns = grid.iter().map(Vec::len).max().unwrap_or(0);
    (grid, columns, flattened)
}

/// The line closing every group, in bold, at its label's depth.
const END_TEXT: &str = "End of proposed text.";

/// A line break inside a group's model text in Markdown: the inner quote's prefix.
const INNER: &str = "\n> > ";

/// Where a drafting model's text goes in Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum In {
    /// A paragraph, a heading or a formula.
    Text,
    /// The model's id or version in a label: as text, and no emphasis, code span or strikethrough
    /// either, so a name carries no markup into the label's line.
    Name,
    /// A GFM table cell.
    Cell,
    /// Inside a fence, which reads nothing.
    Code,
}

/// Write a block's pieces as Markdown: text by [`md_text`], tags as syntax.
fn md_pieces(e: &mut Emit, pieces: &[Piece], id: &str, at: In) {
    for piece in pieces {
        match piece {
            Piece::Tag(tag) => e.syntax(tag),
            Piece::Text(text) => md_text(e, text, id, at),
        }
    }
}

/// **The one way a drafting model's text reaches Markdown**, as `proposed` bytes inside its
/// group's inner quote: each line after the first opens with `> > ` (`> >` where blank), a `\r`
/// or `\r\n` breaking a line as CommonMark reads it, so no line of it stands outside that quote;
/// each line escaped by [`md_escaped`].
fn md_text(e: &mut Emit, text: &str, id: &str, at: In) {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let last = text.matches('\n').count();
    for (k, line) in text.split('\n').enumerate() {
        // A blank line inside the text is `> >` alone; its last line opens with `> > ` even when
        // empty, so a tag written after it stays at the model's depth (review).
        if k > 0 {
            e.syntax(if line.is_empty() && k < last {
                "\n> >"
            } else {
                INNER
            });
        }
        e.proposed(&md_escaped(line, at), id);
    }
}

/// One line of a drafting model's text as Markdown writes it, U+0000 left out first, so what is
/// escaped is what is written. Outside code: `&` and `<` as `&amp;` and `&lt;`, so no HTML block,
/// comment, tag, autolink or reference forms; every `[` and `]` after a `\`, and a `\` before
/// one doubled, so no link, image or link reference definition forms; the first of three or more
/// backticks or tildes where a fence could open — the line's start, or after a space, a tab or a
/// `>` — after a `\`, so no fence opens; in a cell, `|` and `\` after a `\`, as the engine's
/// cells write them; in a name, `*`, `_`, `` ` `` and `~` after a `\`. In code nothing else: a
/// fence reads none of it.
fn md_escaped(line: &str, at: In) -> String {
    let line: Vec<char> = line.chars().filter(|&c| c != '\0').collect();
    // Which characters are written after a `\`, from the end, so a `\` before one is doubled and
    // cannot undo it.
    let mut slashed = vec![false; line.len()];
    for k in (0..line.len()).rev() {
        let c = line[k];
        let opens = k == 0 || matches!(line[k - 1], ' ' | '\t' | '>');
        slashed[k] = match c {
            _ if at == In::Code => false,
            '[' | ']' => true,
            '\\' => at == In::Cell || slashed.get(k + 1).copied().unwrap_or(false),
            '`' | '~' => {
                at == In::Name
                    || opens
                        && line
                            .get(k..k + 3)
                            .is_some_and(|run| run.iter().all(|&x| x == c))
            }
            '*' | '_' => at == In::Name,
            '|' => at == In::Cell,
            _ => false,
        };
    }
    let mut out = String::with_capacity(line.len());
    for (&c, &slash) in line.iter().zip(&slashed) {
        match c {
            '&' if at != In::Code => out.push_str("&amp;"),
            '<' if at != In::Code => out.push_str("&lt;"),
            _ => {
                if slash {
                    out.push('\\');
                }
                out.push(c);
            }
        }
    }
    out
}

/// A fence for `code`: one backtick more than the longest run of them in it as [`md_escaped`]
/// writes it, three at least, so no line of it closes the fence.
fn fence(code: &[Piece]) -> String {
    let longest = code
        .iter()
        .filter_map(|p| match p {
            Piece::Text(t) => md_escaped(t, In::Code)
                .split(|c| c != '`')
                .map(str::len)
                .max(),
            Piece::Tag(_) => None,
        })
        .max()
        .unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}

/// Write a block's pieces as HTML: text by [`html_text`], tags as syntax.
fn html_pieces(e: &mut Emit, pieces: &[Piece], id: &str) {
    for piece in pieces {
        match piece {
            Piece::Tag(tag) => e.syntax(tag),
            Piece::Text(text) => html_text(e, text, id),
        }
    }
}

/// **The one way a drafting model's text reaches HTML**, as `proposed` bytes: [`html_escaped`].
fn html_text(e: &mut Emit, text: &str, id: &str) {
    e.proposed(&html_escaped(text), id);
}

/// A drafting model's text, or its name, as HTML writes it: escaped, U+0000 left out.
fn html_escaped(text: &str) -> String {
    crate::html::escape(&text.replace('\0', ""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::tests::{payload, placed_run};
    use crate::{
        GeometryPresence, HtmlArtifact, IdAllocator, IdKind, MarkdownArtifact, NativeLocator,
        NodeGeometry, PageRecord, Profile, ProjectionOptions, SegmentKind, Sha256Hex,
    };
    use serde_json::json;

    /// A run: its text and box, in centipoints, on a 612 x 792 point page.
    type Run<'a> = (&'a str, [i64; 4]);

    /// An engine table over runs of page 1: its cells `(row, column, run index)`, and its box.
    type Table<'a> = (&'a [(u32, u32, usize)], [i64; 4]);

    /// A made-up record of `pages`, each its runs in reading order, and an engine table.
    fn record(pages: &[&[Run]], table: Option<Table>) -> DocumentRepresentation {
        record_with(pages, table, &[])
    }

    /// A page's `[width, height]` after its turn, its rotation and its visible box.
    type Frame = ([i64; 2], i64, i64, Option<QRect>);

    /// [`record`], each page framed as `frames` says; a page past its end upright, 612 x 792
    /// points, uncropped.
    fn record_with(
        pages: &[&[Run]],
        table: Option<Table>,
        frames: &[Frame],
    ) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let records: Vec<PageRecord> = (1..=pages.len() as u32)
            .map(|index| {
                let ([width, height], rotation, turn, visible) = frames
                    .get(index as usize - 1)
                    .copied()
                    .unwrap_or(([61200, 79200], 0, 0, None));
                PageRecord {
                    id: alloc.next(IdKind::Page).unwrap(),
                    index,
                    width,
                    height,
                    rotation,
                    turn,
                    visible,
                    drawn_boxes: Vec::new(),
                }
            })
            .collect();
        let (mut nodes, mut geometry) = (Vec::new(), Vec::new());
        for (page, runs) in records.iter().zip(pages) {
            for (k, (text, [x0, y0, x1, y1])) in (1..).zip(*runs) {
                let advance = Some(x1 - x0);
                let mut node =
                    placed_run(&mut alloc, &page.id, k, text, None, *x0, *y1, advance, None);
                if let NativeLocator::Pdf(at) = &mut node.native_locator {
                    at.page = page.index;
                }
                let rect = QRect::new(*x0, *y0, *x1, *y1).unwrap();
                geometry.push(NodeGeometry {
                    node: node.id.clone(),
                    presence: GeometryPresence::Measured(rect),
                });
                nodes.push(node);
            }
        }
        let mut p = payload(nodes, records);
        if let Some((cells, [x0, y0, x1, y1])) = table {
            let id = alloc.next(IdKind::Table).unwrap();
            let cells: Vec<crate::TableCellRecord> = cells
                .iter()
                .map(|&(row, column, i)| crate::TableCellRecord {
                    header: false,
                    id: alloc.next(IdKind::Element).unwrap(),
                    position: crate::TableCellPosition {
                        row,
                        column,
                        rowspan: 1,
                        colspan: 1,
                        table_id: id.clone(),
                    },
                    geometry: geometry[i].presence,
                    text: p.nodes[i].text.clone(),
                    node_ids: vec![p.nodes[i].id.clone()],
                })
                .collect();
            p.tables.push(crate::TableRecord {
                id,
                page: p.pages[0].id.clone(),
                geometry: GeometryPresence::Measured(QRect::new(x0, y0, x1, y1).unwrap()),
                rows: cells.iter().map(|c| c.position.row + 1).max().unwrap_or(0),
                columns: cells
                    .iter()
                    .map(|c| c.position.column + 1)
                    .max()
                    .unwrap_or(0),
                cells,
                derivation: crate::DerivationClass::Computed,
                detection_rule: crate::TABLE_DETECTION_V3.to_string(),
                locator_check: crate::LocatorCheck {
                    check_id: crate::LOCATOR_CHECK_V1.into(),
                    check_version: "1".into(),
                    outcome: crate::CheckStatus::Ok,
                },
                tagged_check: None,
            });
        }
        DocumentRepresentation::seal(p, geometry).unwrap()
    }

    /// `repr` with made-up replies attached: `(page, finish_reason, reply)`.
    fn assisted(
        repr: DocumentRepresentation,
        replies: &[(u32, &str, &str)],
    ) -> DocumentRepresentation {
        assisted_by(repr, replies, ("example/made-up-model", "r1"))
    }

    /// [`assisted`], the replies of the model `(id, version)`.
    fn assisted_by(
        repr: DocumentRepresentation,
        replies: &[(u32, &str, &str)],
        (id, version): (&str, &str),
    ) -> DocumentRepresentation {
        let pixels = Sha256Hex::of_bytes(b"made-up pixels").to_string();
        // Each page rendered at its visible box's size, turned as its rotation declares.
        let sizes: Vec<[i64; 2]> = repr
            .payload()
            .pages
            .iter()
            .map(|p| {
                let [_, _, width, height] = crate::assist::frame_of(p);
                if p.turn % 180 == 0 {
                    [width, height]
                } else {
                    [height, width]
                }
            })
            .collect();
        let pages: Vec<serde_json::Value> = replies
            .iter()
            .map(|(page, finish, reply)| {
                json!({
                    "page": page, "points": sizes[*page as usize - 1], "dpi": 200, "width": 1700,
                    "height": 2200, "pixels_sha256": pixels, "finish_reason": finish,
                    "reply": reply
                })
            })
            .collect();
        let file = json!({
            "ethos_assist_answers": 1,
            "source_sha256": repr.payload().source.sha256.to_string(),
            "renderer": crate::ASSIST_RENDERER,
            "request": crate::ASSIST_REQUEST_V1,
            "model": {"id": id, "version": version},
            "pages": pages
        });
        let bytes = serde_json::to_vec(&file).unwrap();
        crate::assist::attach(repr, &Profile::default(), &bytes, None).unwrap()
    }

    fn markdown_with(
        repr: &DocumentRepresentation,
        options: ProjectionOptions,
    ) -> MarkdownArtifact {
        let profile = Profile::default();
        let sha = profile.profile_sha256().unwrap();
        crate::to_markdown_with(
            repr,
            &profile.parser_version,
            &sha,
            &profile.markdown_rule,
            options,
        )
        .unwrap()
    }

    fn markdown(repr: &DocumentRepresentation) -> MarkdownArtifact {
        markdown_with(repr, ProjectionOptions::default())
    }

    fn html(repr: &DocumentRepresentation) -> HtmlArtifact {
        let profile = Profile::default();
        let sha = profile.profile_sha256().unwrap();
        crate::to_html(repr, &profile.parser_version, &sha, &profile.html_rule).unwrap()
    }

    /// A page's census line, as `(swapped, blocks, left out)`.
    type Line = (bool, usize, Vec<(String, usize)>);

    /// Each page's census line.
    fn lines(census: &Option<ProposedCensus>) -> Vec<Line> {
        census
            .as_ref()
            .unwrap()
            .pages
            .iter()
            .map(|p| {
                let left = p.left_out.iter().map(|e| (e.code.clone(), e.count));
                (p.swapped, p.blocks, left.collect())
            })
            .collect()
    }

    fn left(codes: &[(&str, usize)]) -> Vec<(String, usize)> {
        codes.iter().map(|(c, n)| ((*c).to_string(), *n)).collect()
    }

    /// The label a projection writes before an insertion of `what`.
    fn label(what: &str) -> String {
        format!(
            "> **Proposed:** {what} by example/made-up-model@r1; model text, not read from the \
             document."
        )
    }

    /// A group as Markdown writes it: one block quote holding its label, the model's `blocks` one
    /// quote deeper — each written here with its later lines quoted once, as `x\n> y`, and quoted
    /// once more by this — and its end line.
    fn quoted(what: &str, blocks: &[&str]) -> String {
        let mut group = label(what) + "\n>";
        for (k, block) in blocks.iter().enumerate() {
            group.push_str(if k == 0 { "\n> > " } else { "\n> >\n> > " });
            group.push_str(&block.replace("\n>", "\n> >"));
        }
        group + "\n>\n> **End of proposed text.**"
    }

    /// Whether `markdown`'s groups hold their model text as a CommonMark reader reads them: from a
    /// label to its end line every line is `>` alone, or opens with `> >`, the model's depth, but
    /// for those two; and every code fence a model line holds closes before the inner quote does,
    /// so no line of code is read as Markdown.
    fn contained(markdown: &str) -> bool {
        let (mut inside, mut open) = (false, None::<(char, usize)>);
        for line in markdown.lines() {
            if !inside {
                inside = line.starts_with("> **Proposed:** ");
                continue;
            }
            let content = match line.strip_prefix("> >") {
                Some(rest) if rest.is_empty() || rest.starts_with(' ') => {
                    rest.get(1..).unwrap_or("")
                }
                _ if open.is_some() => return false,
                _ if line == ">" => continue,
                _ if line == "> **End of proposed text.**" => {
                    inside = false;
                    continue;
                }
                _ => return false,
            };
            let indent = content.len() - content.trim_start_matches(' ').len();
            let body = content.trim_start_matches(' ');
            let run = |c: char| body.chars().take_while(|&x| x == c).count();
            match (open, body.chars().next()) {
                (None, Some(c @ ('`' | '~'))) if indent < 4 && run(c) >= 3 => {
                    open = Some((c, run(c)))
                }
                (Some((c, n)), Some(x))
                    if x == c && indent < 4 && run(c) >= n && body.trim_end().len() == run(c) =>
                {
                    open = None;
                }
                _ => {}
            }
        }
        !inside && open.is_none()
    }

    /// A group's opening in HTML: its `<blockquote>` naming the model, and its label.
    const OPEN: &str = "<blockquote class=\"ethos-proposed\" data-derivation=\"proposed\" \
                        data-model=\"example/made-up-model\" data-model-version=\"r1\">\n";

    /// The inner quote's close, the line closing every group in HTML, and its `<blockquote>`.
    const HTML_END: &str =
        "</blockquote>\n<p><strong>End of proposed text.</strong></p>\n</blockquote>\n";

    fn html_label(what: &str) -> String {
        format!(
            "{OPEN}<p><strong>Proposed:</strong> {what} by example/made-up-model@r1; model text, \
             not read from the document.</p>\n<blockquote>\n"
        )
    }

    /// Runs one to a line, down a page from the top.
    fn lines_of<'a>(texts: &[&'a str]) -> Vec<Run<'a>> {
        (0i64..)
            .zip(texts)
            .map(|(k, t)| (*t, [7200, 7200 + 2400 * k, 40000, 8400 + 2400 * k]))
            .collect()
    }

    /// **A page the engine read under 30% of the model's words of is the model's page**, labelled
    /// with the page and the model and closed, where its first node stood, or at the end for a page
    /// of no runs; the engine's runs there are counted, the census balances, and every model byte
    /// is a `proposed` segment naming its block.
    #[test]
    fn a_low_coverage_page_is_swapped_and_labelled() {
        let one = lines_of(&["First page text here"]);
        let two = lines_of(&["zz qq"]);
        let repr = assisted(
            record(&[&one, &two, &[]], None),
            &[
                (
                    2,
                    "stop",
                    "![title](100,50,900,90) # A made-up heading\n\n\
                     ![text](100,100,900,200) Some invented words the engine never read.",
                ),
                (
                    3,
                    "stop",
                    "![text](100,100,900,200) Words on a page of no runs",
                ),
            ],
        );
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "First page text here\n\n{}\n\n{}\n",
                quoted(
                    "page 2 as read",
                    &[
                        "# A made-up heading",
                        "Some invented words the engine never read."
                    ]
                ),
                quoted("page 3 as read", &["Words on a page of no runs"])
            )
        );
        let replaced = md
            .coverage
            .dropped
            .iter()
            .find(|b| b.code == REPLACED_BY_PROPOSED_PAGE)
            .map(|b| (b.chars, b.nodes));
        assert_eq!(replaced, Some((5, 1)));
        assert!(md.coverage.balances());
        let census = md.proposed.as_ref().unwrap();
        assert_eq!(census.rule, crate::ASSIST_RULE_V1);
        assert_eq!(census.model.id, "example/made-up-model");
        let page = &census.pages[0];
        let counts = (page.model_words, page.engine_words, page.shared_words);
        assert_eq!((page.page.as_str(), counts), ("p2", (10, 2, 0)));
        assert_eq!(lines(&md.proposed), [(true, 2, vec![]), (true, 1, vec![])]);
        let proposed: Vec<(&str, Vec<&str>)> = md
            .anchor_map
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Proposed)
            .map(|s| {
                let ids = s.node_ids.iter().map(String::as_str).collect();
                (&md.markdown[s.start..s.end], ids)
            })
            .collect();
        assert_eq!(
            proposed,
            [
                ("A made-up heading", vec!["m1"]),
                ("Some invented words the engine never read.", vec!["m2"]),
                ("Words on a page of no runs", vec!["m3"]),
            ]
        );
        assert!(!md.anchor_map.is_invertible(0, md.markdown.len()));
        assert_eq!(
            html(&repr).html,
            format!(
                "<p>First page text here</p>\n{}<h1>A made-up heading</h1>\n<p>Some invented \
                 words the engine never read.</p>\n{HTML_END}{}<p>Words on a page of no runs</p>\n\
                 {HTML_END}",
                html_label("page 2 as read"),
                html_label("page 3 as read")
            )
        );
    }

    /// **The swap test, in integers**: under 30% of the model's words shared, and the model reading
    /// at least half as many words as the engine.
    #[test]
    fn the_swap_test_is_decision_123s_in_integers() {
        let words = |prefix: &str, n: usize| -> String {
            let words: Vec<String> = (1..=n).map(|i| format!("{prefix}{i}")).collect();
            words.join(" ")
        };
        let model = |shared: usize| format!("{} {}", words("s", shared), words("m", 10 - shared));
        // (engine words, model words): three of ten shared stays, two swaps; with none shared, an
        // engine of 21 words stays and one of 20 swaps.
        let pages = [
            (words("s", 3), model(3)),
            (words("s", 2), model(2)),
            (words("e", 21), model(0)),
            (words("e", 20), model(0)),
        ];
        let runs: Vec<Vec<Run>> = pages.iter().map(|(e, _)| lines_of(&[e.as_str()])).collect();
        let runs: Vec<&[Run]> = runs.iter().map(Vec::as_slice).collect();
        let replies: Vec<String> = pages
            .iter()
            .map(|(_, m)| format!("![text](100,100,900,200) {m}"))
            .collect();
        let replies: Vec<(u32, &str, &str)> = (1..)
            .zip(&replies)
            .map(|(page, r)| (page, "stop", r.as_str()))
            .collect();
        let md = markdown(&assisted(record(&runs, None), &replies));
        let swapped: Vec<(usize, usize, usize, bool)> = md
            .proposed
            .unwrap()
            .pages
            .iter()
            .map(|p| (p.model_words, p.engine_words, p.shared_words, p.swapped))
            .collect();
        assert_eq!(
            swapped,
            [
                (10, 3, 3, false),
                (10, 2, 2, true),
                (10, 21, 0, false),
                (10, 20, 0, true)
            ]
        );
    }

    /// **A table goes after the last run, in reading order, that starts above its foot and
    /// overlaps it across, at the next block boundary**, with the caption just before it; a table
    /// no run stands above goes before the page's first node; a title all of whose words are the
    /// engine's is left out, as is every other block.
    #[test]
    fn a_table_is_inserted_at_its_reading_position() {
        let runs = [
            ("Intro paragraph", [7200, 7200, 40000, 8400]),
            ("Second line", [7200, 9600, 20000, 10800]),
            ("Side note", [40000, 30000, 60000, 31200]),
            ("Below text", [7200, 60000, 40000, 61200]),
        ];
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![title](100,5,900,9) Intro paragraph\n\n\
                 ![table](100,10,900,40) <table><tr><td>Top</td><td>7</td></tr></table>\n\n\
                 ![text](100,90,900,110) Intro paragraph\n\n\
                 ![caption](100,200,900,220) Table 1: Made-up figures\n\n\
                 ![table](100,250,500,500) <table><tr><th>Name</th><th>Count</th></tr>\
                 <tr><td>Apples</td><td>12</td></tr></table>\n\n\
                 ![text](100,760,900,780) Below text",
            )],
        );
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "{}\n\nIntro paragraph\n\nSecond line\n\n{}\n\nSide note\n\nBelow text\n",
                quoted("table", &["| Top | 7 |\n> | --- | --- |"]),
                quoted(
                    "table",
                    &[
                        "Table 1: Made-up figures",
                        "| Name | Count |\n> | --- | --- |\n> | Apples | 12 |"
                    ]
                )
            )
        );
        assert_eq!(
            lines(&md.proposed),
            [(false, 3, left(&[(PROPOSED_BLOCK_NOT_PROJECTED, 3)]))]
        );
        assert_eq!(
            html(&repr).html,
            format!(
                "{}<table>\n<tr>\n<td>Top</td>\n<td>7</td>\n</tr>\n</table>\n{HTML_END}<p>Intro \
                 paragraph</p>\n<p>Second line</p>\n{}<p>Table 1: Made-up figures</p>\n<table>\n\
                 <tr>\n<th>Name</th>\n<th>Count</th>\n</tr>\n<tr>\n<td>Apples</td>\n<td>12</td>\n\
                 </tr>\n</table>\n{HTML_END}<p>Side note</p>\n<p>Below text</p>\n",
                html_label("table"),
                html_label("table")
            )
        );
    }

    /// An engine table of four rows on page 1, under a line of text.
    fn fruit() -> DocumentRepresentation {
        let row = |y: i64, a: &'static str, b: &'static str| {
            [
                (a, [7200, y, 18000, y + 1000]),
                (b, [20000, y, 30000, y + 1000]),
            ]
        };
        let mut runs: Vec<Run> = vec![("Intro words here", [7200, 7200, 40000, 8400])];
        let rows = [
            ("Name", "Count"),
            ("Apples", "12"),
            ("Pears", "30"),
            ("Plums", "45"),
        ];
        for (k, (a, b)) in (0..).zip(rows) {
            runs.extend(row(20000 + 2000 * k, a, b));
        }
        let cells: Vec<(u32, u32, usize)> =
            (0..8).map(|k| (k / 2, k % 2, k as usize + 1)).collect();
        // On the 0-1000 grid, the box (100, 250) to (500, 350).
        record(&[&runs], Some((&cells, [6120, 19800, 30600, 27720])))
    }

    /// The engine table [`fruit`] holds, as GFM.
    const FRUIT: &str =
        "| Name | Count |\n| --- | --- |\n| Apples | 12 |\n| Pears | 30 |\n| Plums | 45 |";

    /// A model table, its box given on the grid and its rows given.
    fn table_at(grid: &str, rows: &[[&str; 2]]) -> String {
        let cells: String = rows
            .iter()
            .map(|[a, b]| format!("<tr><td>{a}</td><td>{b}</td></tr>"))
            .collect();
        format!("![table]({grid}) <table>{cells}</table>")
    }

    const FRUIT_ROWS: [[&str; 2]; 4] = [
        ["Name", "Count"],
        ["Apples", "12"],
        ["Pears", "30"],
        ["Plums", "45"],
    ];

    /// **A model table whose grid equals the engine table it overlaps on half the smaller box is
    /// left out**, and counted; one differing in a cell stands after the engine's, and so does an
    /// equal one that does not overlap it, however thin; one above it goes before it.
    #[test]
    fn an_exact_duplicate_of_an_engine_table_is_dropped() {
        let pears = [FRUIT_ROWS[0], FRUIT_ROWS[1], ["Pear", "30"], FRUIT_ROWS[3]];
        let reply = [
            // Over the engine's lower half: exactly half of this box is the engine's.
            table_at("100,300,500,400", &FRUIT_ROWS),
            table_at("110,250,500,345", &pears),
            table_at("110,600,500,600", &FRUIT_ROWS),
            table_at("100,120,500,200", &[["Lone", "1"]]),
        ]
        .join("\n\n");
        let repr = assisted(fruit(), &[(1, "stop", &reply)]);
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "Intro words here\n\n{}\n\n{FRUIT}\n\n{}\n\n{}\n",
                quoted("table", &["| Lone | 1 |\n> | --- | --- |"]),
                quoted(
                    "table",
                    &[
                        "| Name | Count |\n> | --- | --- |\n> | Apples | 12 |\n> | Pear | 30 |\n> \
                       | Plums | 45 |"
                    ]
                ),
                quoted("table", &[&FRUIT.replace('\n', "\n> ")])
            )
        );
        assert_eq!(
            lines(&md.proposed),
            [(false, 3, left(&[(PROPOSED_TABLE_DUPLICATE, 1)]))]
        );
        assert!(html(&repr).html.starts_with(&format!(
            "<p>Intro words here</p>\n{}<table>\n<tr>\n<td>Lone</td>\n<td>1</td>\n</tr>\n\
             </table>\n{HTML_END}<table>\n<tr>\n<td>Name</td>",
            html_label("table")
        )));
    }

    /// **A contradicted or unsupported cell is emptied, never removed**, counted, and said in HTML.
    #[test]
    fn contradicted_and_unsupported_cells_are_emptied() {
        let rows = [
            FRUIT_ROWS[0],
            ["Apples", "13"],
            FRUIT_ROWS[2],
            FRUIT_ROWS[3],
            ["Invented four word line", ""],
        ];
        let reply = table_at("110,250,500,345", &rows);
        let repr = assisted(fruit(), &[(1, "stop", &reply)]);
        let marks: Vec<(u32, AssistMark)> = repr.payload().assist.as_ref().unwrap().pages[0].blocks
            [0]
        .guard
        .as_ref()
        .unwrap()
        .flagged
        .iter()
        .map(|f| (f.line, f.mark))
        .collect();
        assert_eq!(
            marks,
            [(4, AssistMark::Contradicted), (9, AssistMark::Unsupported)]
        );
        let md = markdown(&repr);
        assert!(md.markdown.ends_with(&format!(
            "{}\n",
            quoted(
                "table",
                &["| Name | Count |\n> | --- | --- |\n> | Apples |  |\n> | Pears | 30 |\n> | Plums \
                   | 45 |\n> |  |  |"]
            )
        )));
        let dropped = [
            (PROPOSED_LINE_UNSUPPORTED, 1),
            (PROPOSED_VALUE_CONTRADICTED, 1),
        ];
        assert_eq!(lines(&md.proposed), [(false, 1, left(&dropped))]);
        let html = html(&repr).html;
        assert!(html
            .contains("<tr>\n<td>Apples</td>\n<td data-dropped=\"contradicted\"></td>\n</tr>\n"));
        assert!(html.contains(&format!(
            "<tr>\n<td data-dropped=\"unsupported\"></td>\n<td></td>\n</tr>\n</table>\n{HTML_END}"
        )));
    }

    /// **A caption's contradicted line is left out**, the rest of it written as the guard read it:
    /// decoded, then escaped as any model text is, a decoded U+0000 left out.
    #[test]
    fn a_contradicted_caption_line_is_dropped() {
        let runs = [
            ("Revenue 2023 rose", [7200, 7200, 40000, 8400]),
            ("Total 998", [7200, 9600, 40000, 10800]),
        ];
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![caption](100,85,700,140) Revenue 2023 rose sharply &lt;!--&#0;\nTotal 999\n\n\
                 ![table](100,200,700,300) <table><tr><td>Q</td><td>9</td></tr></table>",
            )],
        );
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "Revenue 2023 rose\n\nTotal 998\n\n{}\n",
                quoted(
                    "table",
                    &[
                        "Revenue 2023 rose sharply &lt;!--",
                        "| Q | 9 |\n> | --- | --- |"
                    ]
                )
            )
        );
        assert_eq!(
            lines(&md.proposed),
            [(false, 2, left(&[(PROPOSED_VALUE_CONTRADICTED, 1)]))]
        );
        assert!(html(&repr)
            .html
            .contains("<p>Revenue 2023 rose sharply &lt;!--</p>\n<table>"));
    }

    /// **A chart's values not in the page's text are kept and labelled estimates**, in the label
    /// and, in HTML, on their cells.
    #[test]
    fn chart_estimates_are_labelled() {
        let runs = lines_of(&["Revenue by year"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![chart](100,200,900,500) <table><tr><th>Year</th><th>Revenue</th></tr>\
                 <tr><td>2023</td><td>41</td></tr></table>",
            )],
        );
        assert_eq!(
            markdown(&repr).markdown,
            format!(
                "Revenue by year\n\n{}\n",
                quoted(
                    "chart values estimated",
                    &["| Year | Revenue |\n> | --- | --- |\n> | 2023 | 41 |"]
                )
            )
        );
        assert!(html(&repr).html.ends_with(&format!(
            "{}<table>\n<tr>\n<th>Year</th>\n<th>Revenue</th>\n</tr>\n<tr>\n<td \
             data-estimate=\"true\">2023</td>\n<td data-estimate=\"true\">41</td>\n</tr>\n\
             </table>\n{HTML_END}",
            html_label("chart values estimated")
        )));
    }

    /// **A reply the server cut loses its last block**, truncated or a loop, on a page added to
    /// or swapped; its blocks before it stand.
    #[test]
    fn a_cut_last_block_is_dropped() {
        let one = lines_of(&["One Two engine words"]);
        let two = lines_of(&["zz"]);
        let looped = format!(
            "![text](100,100,900,200) Made-up words\n\n![table](100,300,900,400) <table>{}",
            "<tr><td>0</td></tr>".repeat(8)
        );
        let repr = assisted(
            record(&[&one, &two], None),
            &[
                (
                    1,
                    "length",
                    "![table](100,100,900,200) <table><tr><td>One</td><td>Two</td></tr></table>\
                     \n\n![table](100,300,900,400) <table><tr><td>Cut",
                ),
                (2, "length", &looped),
            ],
        );
        let outcomes: Vec<AssistOutcome> = repr
            .payload()
            .assist
            .as_ref()
            .unwrap()
            .pages
            .iter()
            .map(|p| p.outcome)
            .collect();
        assert_eq!(outcomes, [AssistOutcome::Truncated, AssistOutcome::Loop]);
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "One Two engine words\n\n{}\n\n{}\n",
                quoted("table", &["| One | Two |\n> | --- | --- |"]),
                quoted("page 2 as read", &["Made-up words"])
            )
        );
        let cut = left(&[(PROPOSED_BLOCK_TRUNCATED, 1)]);
        assert_eq!(
            lines(&md.proposed),
            [(false, 1, cut.clone()), (true, 1, cut)]
        );
    }

    /// **A model table is written from its cells**: in GFM its spans flattened and counted, a
    /// `|` escaped and whitespace collapsed; in HTML with its spans and header cells.
    #[test]
    fn a_model_tables_spans_are_flattened_in_gfm_and_kept_in_html() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![table](100,100,900,300) <table><tr><th colspan=\"2\">Head</th></tr>\
                 <tr><td rowspan=2>A  \n b</td><td>1|2</td></tr><tr><td>2</td></tr></table>",
            )],
        );
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &["| Head |  |\n> | --- | --- |\n> | A b | 1\\|2 |\n> |  | 2 |"]
                )
            )
        );
        let spans = left(&[(crate::markdown::GFM_SPAN_SLOTS_UNREPRESENTABLE, 2)]);
        assert_eq!(lines(&md.proposed), [(true, 1, spans)]);
        let html = html(&repr);
        assert!(html.html.contains(
            "<table>\n<tr>\n<th colspan=\"2\">Head</th>\n</tr>\n<tr>\n<td rowspan=\"2\">A \
             b</td>\n<td>1|2</td>\n</tr>\n<tr>\n<td>2</td>\n</tr>\n</table>\n"
        ));
        assert_eq!(lines(&html.proposed), [(true, 1, vec![])]);
    }

    /// **No model HTML reaches either projection**: `<sub>`, `<sup>` and `<br>` are written by
    /// this engine, balanced; an inline tag goes, any other tag leaves a space, and what is no tag
    /// stays text; a character reference is decoded, then escaped, in both.
    #[test]
    fn model_html_is_never_passed_through() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) x<sup>2</sup> and H<sub>2</sub>O<br/>next \
                 <script>alert(1)</script> <b>bold</b> &amp; &dollar;5 stray</sub> \
                 <https://example.org> <sup>open",
            )],
        );
        let md = markdown(&repr).markdown;
        assert!(md.contains(
            "> x<sup>2</sup> and H<sub>2</sub>O<br>next  alert(1)  bold &amp; $5 stray \
             &lt;https://example.org> <sup>open</sup>\n"
        ));
        let html = html(&repr).html;
        assert!(html.contains(
            "<p>x<sup>2</sup> and H<sub>2</sub>O<br>next  alert(1)  bold &amp; $5 stray \
             &lt;https://example.org&gt; <sup>open</sup></p>\n"
        ));
        assert!(!html.contains("<script") && !html.contains("<b>"));
    }

    /// **Model text cannot leave its group** (decision #147's review): on every path — a
    /// paragraph, code, a formula, a table cell; a dropped line's rewrite in
    /// [`a_contradicted_caption_line_is_dropped`] — Markdown keeps it inside the group's quote, a
    /// `\r` breaking a line there too; `&` and `<` are references, so a tag a removed tag rebuilt,
    /// a decoded `&lt;` or a comment is text; a `[` that could open a link reference definition,
    /// after a list marker too, is escaped; code sits in a fence its own backticks cannot close;
    /// a `&#0;` is left out. HTML escapes the same text.
    #[test]
    fn model_text_cannot_leave_its_group() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) a <im<b>g src=x> &lt;b&gt; &#0;b [1]: x\n- [2]: y\n\
                 <!-- c\r\r**End of proposed text.**\n\n\
                 ![code](100,300,900,400) ```js\n<b>&lt;</b>\n```\n````\n```\n\n\
                 ![formula](100,500,900,600) $$a<b &amp; [c]$$\n\n\
                 ![table](100,700,900,800) <table><tr><td>&lt;i&gt;|\\ [3]: z</td></tr></table>",
            )],
        );
        assert_eq!(
            markdown(&repr).markdown,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &[
                        "a &lt;img src=x> &lt;b> b \\[1\\]: x\n> - \\[2\\]: y\n> &lt;!-- c\n>\n> \
                         **End of proposed text.**",
                        "`````\n> <b><</b>\n> ```\n> ````\n> `````",
                        "$$\n> a&lt;b &amp; \\[c\\]\n> $$",
                        "| &lt;i>\\|\\\\ \\[3\\]: z |\n> | --- |",
                    ]
                )
            )
        );
        assert_eq!(
            html(&repr).html,
            format!(
                "{}<p>a &lt;img src=x&gt; &lt;b&gt; b [1]: x\n- [2]: y\n&lt;!-- c\r\r**End of \
                 proposed text.**</p>\n<pre><code>&lt;b&gt;&lt;&lt;/b&gt;\n```\n````</code></pre>\n\
                 <p>$$a&lt;b &amp; [c]$$</p>\n<table>\n<tr>\n<td>&lt;i&gt;|\\ [3]: z</td>\n</tr>\n\
                 </table>\n{HTML_END}",
                html_label("page 1 as read")
            )
        );
    }

    /// **The label names the model escaped**, by the function its text goes through: a `"` in its
    /// id or version cannot close an attribute, nor a `<` open a tag, nor a `[` a link; and in
    /// Markdown no `*`, `_`, `` ` `` or `~` marks anything up, a `\` before one doubled, so the
    /// name carries no markup into the label's line.
    #[test]
    fn the_models_name_is_escaped_in_the_label() {
        let runs = lines_of(&["zz"]);
        let repr = assisted_by(
            record(&[&runs], None),
            &[(1, "stop", "![text](100,100,900,200) Made-up words")],
            (
                "ex/m \"q\" <!-- [x](http://e.example/) &amp; **End of proposed text.** _",
                "r1\" onload=\"x \\*",
            ),
        );
        assert!(markdown(&repr).markdown.starts_with(
            "> **Proposed:** page 1 as read by ex/m \"q\" &lt;!-- \\[x\\](http://e.example/) \
             &amp;amp; \\*\\*End of proposed text.\\*\\* \\_@r1\" onload=\"x \\\\\\*; model text, \
             not read from the document.\n>\n> > Made-up words\n"
        ));
        assert!(html(&repr).html.starts_with(
            "<blockquote class=\"ethos-proposed\" data-derivation=\"proposed\" data-model=\"ex/m \
             &quot;q&quot; &lt;!-- [x](http://e.example/) &amp;amp; **End of proposed text.** _\" \
             data-model-version=\"r1&quot; onload=&quot;x \\*\">\n<p><strong>Proposed:</strong> \
             page 1 as read by ex/m \"q\" &lt;!-- [x](http://e.example/) &amp;amp; **End of \
             proposed text.** _@r1\" onload=\"x \\*; model text, not read from the \
             document.</p>\n<blockquote>\n"
        ));
    }

    /// **A U+0000 cannot shorten a code fence** (the second review's breakout): the fence is sized
    /// on the code as written, its NUL left out, so ``` ``\0`` ``` is a run of four and the fence
    /// five, and the model's `</blockquote>` stays code; a reference or the character alike.
    #[test]
    fn a_nul_cannot_shorten_a_code_fence() {
        let runs = lines_of(&["zz"]);
        for nul in ["&#0;", "\0"] {
            let reply = format!(
                "![code](100,300,900,400) x = 1\n``{nul}``\n</blockquote>\n\n<p>Forged</p>"
            );
            let repr = assisted(record(&[&runs], None), &[(1, "stop", &reply)]);
            let md = markdown(&repr).markdown;
            assert_eq!(
                md,
                format!(
                    "{}\n",
                    quoted(
                        "page 1 as read",
                        &["`````\n> x = 1\n> ````\n> </blockquote>\n>\n> <p>Forged</p>\n> `````"]
                    )
                ),
                "{nul:?}"
            );
            assert!(contained(&md), "{nul:?}");
        }
    }

    /// **A line of model text cannot open a fence** (the second review): outside code, a run of
    /// three backticks or tildes where a fence could open — a line's start, or after a space, a tab
    /// or a `>` — has its first character escaped, in a paragraph, a title and a formula.
    #[test]
    fn a_model_line_cannot_open_a_fence() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) Words.\n```\n- ~~~ x\n\n\
                 ![title](100,50,900,90) # Heading\n~~~\n\n\
                 ![formula](100,500,900,600) $$a\n```\nb$$",
            )],
        );
        let md = markdown(&repr).markdown;
        assert_eq!(
            md,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &[
                        "Words.\n> \\```\n> - \\~~~ x",
                        "# Heading\n> \\~~~",
                        "$$\n> a\n> \\```\n> b\n> $$"
                    ]
                )
            )
        );
        assert!(contained(&md));
    }

    /// **No model text can stand where the label and end line stand** (the second review): every
    /// model line is one quote deeper than the group's own two, so a model's `**End of proposed
    /// text.**`, a `####` title of those words or of a label's, a `>` of its own or a header cell
    /// is read inside the inner quote, never as the group's end or label; in HTML the model's
    /// blocks are in an inner `<blockquote>`, the label and end line outside it, and no model text
    /// is ever `<strong>`.
    #[test]
    fn model_text_cannot_stand_for_the_label_or_end_line() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) Words.\n\n**End of proposed text.**\n\nAfter.\n\n\
                 ![title](100,50,900,90) #### End of proposed text.\n\n\
                 ![title](100,50,900,90) #### Proposed: page 1 as read by \
                 example/made-up-model@r1; model text, not read from the document.\n\n\
                 ![text](100,100,900,200) > **End of proposed text.**\n\n\
                 ![table](100,700,900,800) <table><tr><th>End of proposed text.</th></tr></table>",
            )],
        );
        let md = markdown(&repr).markdown;
        assert_eq!(
            md,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &[
                        "Words.\n>\n> **End of proposed text.**\n>\n> After.",
                        "#### End of proposed text.",
                        "#### Proposed: page 1 as read by example/made-up-model@r1; model text, \
                         not read from the document.",
                        "> **End of proposed text.**",
                        "| End of proposed text. |\n> | --- |"
                    ]
                )
            )
        );
        assert!(contained(&md));
        let ours = |line: &str| md.lines().filter(|l| *l == line).count();
        assert_eq!(ours("> **End of proposed text.**"), 1);
        assert_eq!(md.lines().filter(|l| l.starts_with("> **")).count(), 2);
        let html = html(&repr).html;
        assert_eq!(
            html,
            format!(
                "{}<p>Words.\n\n**End of proposed text.**\n\nAfter.</p>\n<h4>End of proposed \
                 text.</h4>\n<h4>Proposed: page 1 as read by example/made-up-model@r1; model text, \
                 not read from the document.</h4>\n<p>&gt; **End of proposed text.**</p>\n\
                 <table>\n<tr>\n<th>End of proposed text.</th>\n</tr>\n</table>\n{HTML_END}",
                html_label("page 1 as read")
            )
        );
        assert_eq!(html.matches("<strong>").count(), 2);
    }

    /// **No link or image forms from model text** (the second review): every `[` and `]` is
    /// escaped, mid-line too, in text and in a cell, and a model's `\` before one doubled so it
    /// cannot undo that; an autolink's `<` is a reference.
    #[test]
    fn no_link_or_image_forms_from_model_text() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) see x[click](javascript:alert(1)) \
                 ![](http://e.example/p.png) [site][1] <http://e.example/a> \\[y\\](b)\n\n\
                 ![table](100,700,900,800) <table><tr><td>[c](http://e.example/)</td></tr></table>",
            )],
        );
        let md = markdown(&repr).markdown;
        assert_eq!(
            md,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &[
                        "see x\\[click\\](javascript:alert(1)) !\\[\\](http://e.example/p.png) \
                         \\[site\\]\\[1\\] &lt;http://e.example/a> \\\\\\[y\\\\\\](b)",
                        "| \\[c\\](http://e.example/) |\n> | --- |"
                    ]
                )
            )
        );
        assert!(contained(&md));
    }

    /// **A page is swapped only for words a swap would write** (decision #147's review): not a
    /// reply's cut last block, so a page is never swapped to nothing; not furniture
    /// `--drop-furniture` leaves out; not a chart's text outside its cells.
    #[test]
    fn the_swap_counts_only_what_it_would_write() {
        let one = lines_of(&["Engine read this real sentence on page one"]);
        let two = lines_of(&["Engine read this real sentence on page two"]);
        let three = lines_of(&["Quarter revenue table"]);
        let sixty: Vec<String> = (0..60).map(|k| format!("word{k}")).collect();
        let cut = format!("![text](100,100,900,900) {}", sixty.join(" "));
        let repr = assisted(
            record(&[&one, &two, &three], None),
            &[
                (1, "length", &cut),
                (
                    2,
                    "stop",
                    "![header](100,10,900,40) Annual report of the example holding company \
                     limited\n\n![page_number](450,950,550,990) 17\n\n\
                     ![text](100,100,900,900) Engine read",
                ),
                (
                    3,
                    "stop",
                    "![chart](100,100,900,400) <table><tr><td>Quarter</td><td>Revenue</td></tr>\
                     </table>\n\nTen invented words nobody printed here at all today",
                ),
            ],
        );
        let census = |options| {
            let md = markdown_with(&repr, options);
            let counts: Vec<(usize, usize, usize)> = md
                .proposed
                .as_ref()
                .unwrap()
                .pages
                .iter()
                .map(|p| (p.model_words, p.engine_words, p.shared_words))
                .collect();
            (lines(&md.proposed), counts)
        };
        let truncated = left(&[(PROPOSED_BLOCK_TRUNCATED, 1)]);
        assert_eq!(
            census(ProjectionOptions::default()),
            (
                vec![
                    (false, 0, truncated.clone()),
                    (true, 3, vec![]),
                    (false, 1, vec![])
                ],
                vec![(0, 8, 0), (11, 8, 2), (2, 3, 2)]
            )
        );
        let options = ProjectionOptions {
            drop_furniture: true,
            ..ProjectionOptions::default()
        };
        assert_eq!(
            census(options).0,
            [
                (false, 0, truncated),
                (false, 0, left(&[(PROPOSED_BLOCK_NOT_PROJECTED, 3)])),
                (false, 1, vec![])
            ]
        );
    }

    /// **The swap counts the words the guard leaves** (review): thirty blocks of the engine's words,
    /// each with a line or a cell of a value the page does not hold, write only the engine's
    /// words, and do not swap the page; text on page 1, tables on page 2.
    #[test]
    fn the_swap_counts_the_words_the_guard_leaves() {
        let shared = "alpha bravo charlie delta echo foxtrot golf hotel india juliet";
        let runs = lines_of(&[shared, "Total 500"]);
        let text: Vec<String> = (0..30)
            .map(|k| format!("![text](100,50,900,300) {shared}\nTotal {}", 9001 + k))
            .collect();
        let tables: Vec<String> = (0..30)
            .map(|k| {
                format!(
                    "![table](100,50,900,300) <table><tr><td>{shared}</td></tr>\
                     <tr><td>Total</td><td>{}</td></tr></table>",
                    9001 + k
                )
            })
            .collect();
        let repr = assisted(
            record(&[&runs, &runs], None),
            &[
                (1, "stop", &text.join("\n\n")),
                (2, "stop", &tables.join("\n\n")),
            ],
        );
        for page in &repr.payload().assist.as_ref().unwrap().pages {
            let contradicted = page
                .blocks
                .iter()
                .flat_map(|b| &b.guard.as_ref().unwrap().flagged)
                .filter(|f| f.mark == AssistMark::Contradicted)
                .count();
            assert_eq!(
                contradicted, 30,
                "{}: each value is contradicted",
                page.page
            );
        }
        let md = markdown(&repr);
        let counts: Vec<(usize, usize, usize, bool)> = md
            .proposed
            .as_ref()
            .unwrap()
            .pages
            .iter()
            .map(|p| (p.model_words, p.engine_words, p.shared_words, p.swapped))
            .collect();
        assert_eq!(counts, [(10, 12, 10, false), (11, 12, 11, false)]);
    }

    /// **The swap reads the model's text as it reads the engine's** (review): a `<` and a `>` in
    /// running text hide the same words on both sides, so a page read right is not swapped.
    #[test]
    fn the_swap_reads_the_model_as_it_reads_the_engine() {
        let text =
            "Results p < 0.05 alpha bravo charlie delta echo foxtrot golf hotel india juliet \
                    where n > 30";
        let runs = lines_of(&[text]);
        let reply = format!("![text](100,50,900,300) {text}");
        let md = markdown(&assisted(record(&[&runs], None), &[(1, "stop", &reply)]));
        let page = &md.proposed.as_ref().unwrap().pages[0];
        assert_eq!(
            (
                page.model_words,
                page.engine_words,
                page.shared_words,
                page.swapped
            ),
            (2, 2, 2, false)
        );
    }

    /// **A repeated header the joined table leaves out still counts as read** (decision #151,
    /// review): the swap test reads every row the engine read, so a continuation page is not
    /// given to the model because its header no longer stands in the table, kept or not.
    #[test]
    fn a_left_out_repeated_header_counts_as_read() {
        use crate::markdown::tests::{paged_tables, staff, FIRST, SECOND};
        let reply = "![text](100,50,900,300) Name Dept Bob alpha bravo charlie delta echo foxtrot \
                     golf";
        let repr = assisted(
            paged_tables(&staff(&[]), &[FIRST, SECOND], crate::TABLE_DETECTION_V14),
            &[(2, "stop", reply)],
        );
        for keep_repeated_header in [false, true] {
            let options = ProjectionOptions {
                keep_repeated_header,
                ..ProjectionOptions::default()
            };
            let md = markdown_with(&repr, options);
            let page = &md.proposed.as_ref().unwrap().pages[0];
            assert_eq!(
                (
                    page.model_words,
                    page.engine_words,
                    page.shared_words,
                    page.swapped
                ),
                (10, 7, 3, false)
            );
        }
    }

    /// **A model table is a duplicate of the joined table as the projection writes it**
    /// (decision #151, review): its repeated header left out, or kept on request.
    #[test]
    fn a_duplicate_is_judged_against_the_joined_table_as_written() {
        use crate::markdown::tests::{paged_tables, staff, Grid, FIRST, SECOND};
        let header_only = Grid {
            cells: &[7, 8],
            ..SECOND
        };
        let runs: Vec<_> = staff(&[])
            .into_iter()
            .filter(|l| !matches!(l.0, "Bob" | "Tax" | "Cy" | "Legal"))
            .collect();
        let reply = table_at("0,0,1000,1000", &[["Name", "Dept"], ["Ann", "Audit"]]);
        let repr = assisted(
            paged_tables(&runs, &[FIRST, header_only], crate::TABLE_DETECTION_V14),
            &[(1, "stop", &reply)],
        );
        let duplicate = |keep_repeated_header| {
            let options = ProjectionOptions {
                keep_repeated_header,
                ..ProjectionOptions::default()
            };
            let md = markdown_with(&repr, options);
            let counted = lines(&md.proposed)[0]
                .2
                .contains(&(PROPOSED_TABLE_DUPLICATE.to_string(), 1));
            (counted, md.markdown.contains("**Proposed:**"))
        };
        assert_eq!(duplicate(false), (true, false));
        assert_eq!(duplicate(true), (false, true));
    }

    /// **A caption is added for the words it writes** (review): one whose words not the engine's
    /// stand on a line the guard leaves out writes only the engine's, and is left out.
    #[test]
    fn a_caption_is_judged_by_what_it_writes() {
        let runs = [
            ("Revenue 2023 rose", [7200, 7200, 40000, 8400]),
            ("Total 998", [7200, 9600, 40000, 10800]),
        ];
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![caption](100,85,700,140) Revenue 2023 rose\nTotal 999\n\n\
                 ![table](100,200,700,300) <table><tr><td>Q</td><td>9</td></tr></table>",
            )],
        );
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "Revenue 2023 rose\n\nTotal 998\n\n{}\n",
                quoted("table", &["| Q | 9 |\n> | --- | --- |"])
            )
        );
        assert_eq!(
            lines(&md.proposed),
            [(false, 1, left(&[(PROPOSED_BLOCK_NOT_PROJECTED, 1)]))]
        );
    }

    /// **A line break before a tag stays inside its quote** (review): the next line opens with
    /// `> > `, as every model line does.
    #[test]
    fn a_line_break_before_a_tag_stays_inside_its_quote() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) Made-up words<sup>1</sup>\n<sup>2</sup> and<br>\n<br>end",
            )],
        );
        let md = markdown(&repr).markdown;
        assert!(contained(&md), "{md}");
        assert_eq!(
            md,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &["Made-up words<sup>1</sup>\n> <sup>2</sup> and<br>\n> <br>end"]
                )
            )
        );
    }

    /// **A swapped page's chart estimates are named in its label** (review), as a chart added to a
    /// page has them named: the Markdown table writes no cell's mark.
    #[test]
    fn a_swapped_pages_chart_estimates_are_labelled() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![chart](100,200,900,500) <table><tr><th>Year</th><th>Revenue</th></tr>\
                 <tr><td>2023</td><td>41</td></tr></table>",
            )],
        );
        let what = "page 1 as read, chart values estimated";
        assert_eq!(
            markdown(&repr).markdown,
            format!(
                "{}\n",
                quoted(
                    what,
                    &["| Year | Revenue |\n> | --- | --- |\n> | 2023 | 41 |"]
                )
            )
        );
        assert!(html(&repr).html.starts_with(&html_label(what)));
    }

    /// **A space between two inline elements stays** (review), in both projections; a block of
    /// tags and spaces only writes nothing; the swap reads a tag as a space.
    #[test]
    fn a_space_between_inline_elements_stays() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![text](100,100,900,200) Made-up words<br>here (x)<sup>2</sup> <sup>3</sup>\n\n\
                 ![text](100,300,900,400) <br> <br>",
            )],
        );
        let md = markdown(&repr);
        assert_eq!(
            md.markdown,
            format!(
                "{}\n",
                quoted(
                    "page 1 as read",
                    &["Made-up words<br>here (x)<sup>2</sup> <sup>3</sup>"]
                )
            )
        );
        assert_eq!(md.proposed.as_ref().unwrap().pages[0].model_words, 4);
        assert!(html(&repr)
            .html
            .contains("<p>Made-up words<br>here (x)<sup>2</sup> <sup>3</sup></p>"));
    }

    /// **A model table too large to lay out is not written** (review): spans that would lay it
    /// out on more than [`TABLE_SLOTS`] slots, here each row's starting past the last one's, count
    /// it unparsed.
    #[test]
    fn a_table_too_large_to_lay_out_is_not_written() {
        let runs = lines_of(&["One Two engine words"]);
        let rows = "<tr><td rowspan=1000 colspan=1000>x</td></tr>".repeat(12);
        let reply = format!("![table](100,300,900,400) <table>{rows}</table>");
        let md = markdown(&assisted(record(&[&runs], None), &[(1, "stop", &reply)]));
        assert_eq!(md.markdown, "One Two engine words\n");
        assert_eq!(
            lines(&md.proposed),
            [(false, 0, left(&[(PROPOSED_TABLE_UNPARSED, 1)]))]
        );
    }

    /// **The grid's bound is its rows times its widest row**, a span counted on every row it
    /// covers and on none after.
    #[test]
    fn the_grid_bound_is_the_rows_times_the_widest_row() {
        let cell = |rowspan, colspan| Cell {
            header: false,
            rowspan,
            colspan,
            text: Vec::new(),
            mark: None,
        };
        let wide = |n: usize| -> Vec<Vec<Cell>> { (0..n).map(|_| vec![cell(1, 1000)]).collect() };
        assert!(fits(&wide(100)));
        assert!(!fits(&wide(101)));
        let mut rows = vec![vec![cell(1, 500)]];
        rows.extend((1..200).map(|_| vec![cell(1, 1)]));
        assert!(fits(&rows), "200 rows, the widest 500");
        rows[0][0].rowspan = 2;
        assert!(!fits(&rows), "200 rows, the widest 501");
    }

    /// **A block is placed on the page's visible box, turned as the record's sizes are** (design
    /// §4.3): on a cropped page the grid spans the crop, not the media box; on a turned page it
    /// spans the turned width and height, with no rotation term. On a page read turned (decision
    /// #155) the picture shows the page as declared, so the grid turns with the page.
    #[test]
    fn a_block_is_placed_on_a_cropped_and_a_turned_page() {
        let run = |text, y: i64| (text, [21000, y, 59000, y + 1000]);
        let cropped = [run("Top run", 11000), run("Middle run", 26000)];
        let turned = [run("Second top", 5000), run("Second middle", 20000)];
        let read_turned = [run("Third top", 5000), run("Third middle", 20000)];
        let crop = QRect::new(20000, 10000, 60000, 50000).unwrap();
        let repr = assisted(
            record_with(
                &[&cropped, &turned, &read_turned],
                None,
                &[
                    ([61200, 79200], 0, 0, Some(crop)),
                    ([79200, 61200], 90, 0, None),
                    ([79200, 61200], 0, 90, None),
                ],
            ),
            &[
                (
                    1,
                    "stop",
                    "![table](100,300,900,360) <table><tr><td>Top</td><td>run</td></tr></table>",
                ),
                (
                    2,
                    "stop",
                    "![table](100,250,900,300) <table><tr><td>Second</td><td>top</td></tr>\
                     </table>",
                ),
                (
                    3,
                    "stop",
                    "![table](250,100,300,900) <table><tr><td>Third</td><td>top</td></tr>\
                     </table>",
                ),
            ],
        );
        assert_eq!(
            markdown(&repr).markdown,
            format!(
                "Top run\n\n{}\n\nMiddle run\n\nSecond top\n\n{}\n\nSecond middle\n\n\
                 Third top\n\n{}\n\nThird middle\n",
                quoted("table", &["| Top | run |\n> | --- | --- |"]),
                quoted("table", &["| Second | top |\n> | --- | --- |"]),
                quoted("table", &["| Third | top |\n> | --- | --- |"])
            )
        );
    }

    /// A title keeps its model's level, a formula loses its outer delimiters, code is fenced, an
    /// empty block is not written, and under `drop_furniture` a swapped page's header and page
    /// number go; each left out counted.
    #[test]
    fn blocks_render_by_label_and_furniture_goes_on_request() {
        let runs = lines_of(&["zz"]);
        let repr = assisted(
            record(&[&runs], None),
            &[(
                1,
                "stop",
                "![header](100,10,900,20) Made-up running head\n\n\
                 ![title](100,50,900,90) ## Second level\n\n\
                 ![formula](100,100,900,120) $$x^2 + y$$\n\n\
                 ![code](100,130,900,160) let made_up = 1;\n\n\
                 ![page_number](100,950,900,980) 7\n\n\
                 ![image](100,500,900,600)",
            )],
        );
        let body = [
            "## Second level",
            "$$\n> x^2 + y\n> $$",
            "```\n> let made_up = 1;\n> ```",
        ];
        let all = markdown(&repr);
        let every = [&["Made-up running head"], &body[..], &["7"]].concat();
        assert_eq!(
            all.markdown,
            format!("{}\n", quoted("page 1 as read", &every))
        );
        assert_eq!(
            lines(&all.proposed),
            [(true, 5, left(&[(PROPOSED_BLOCK_NOT_PROJECTED, 1)]))]
        );
        let options = ProjectionOptions {
            drop_furniture: true,
            ..ProjectionOptions::default()
        };
        let dropped = markdown_with(&repr, options);
        assert_eq!(
            dropped.markdown,
            format!("{}\n", quoted("page 1 as read", &body))
        );
        assert_eq!(
            lines(&dropped.proposed),
            [(true, 3, left(&[(PROPOSED_BLOCK_NOT_PROJECTED, 3)]))]
        );
    }

    /// **Where nothing is inserted, the Markdown and HTML are the engine-only ones byte for
    /// byte** but for the census; and an engine-only record's carry no census at all.
    #[test]
    fn a_reply_that_inserts_nothing_moves_no_byte() {
        let runs = lines_of(&["Some engine words", "More engine words"]);
        let engine = record(&[&runs], None);
        let repr = assisted(
            engine.clone(),
            &[(1, "stop", "![text](100,90,900,110) Some engine words")],
        );
        let (plain, with) = (markdown(&engine), markdown(&repr));
        assert_eq!(plain.markdown, with.markdown);
        assert_eq!(plain.anchor_map, with.anchor_map);
        assert_eq!(plain.coverage, with.coverage);
        assert_eq!(html(&engine).html, html(&repr).html);
        assert!(plain.proposed.is_none() && html(&engine).proposed.is_none());
        let text = String::from_utf8(plain.to_canonical_bytes().unwrap()).unwrap();
        assert!(!text.contains("proposed"));
        assert_eq!(
            lines(&with.proposed),
            [(false, 0, left(&[(PROPOSED_BLOCK_NOT_PROJECTED, 1)]))]
        );
        // And the same record projects to the same bytes, run after run.
        assert_eq!(markdown(&repr), with);
    }

    /// **A table the next page goes on with is not joined across a swapped page**, either way:
    /// the page the model's reading replaces takes its table with it, the other page keeps its
    /// own, and no row is written that no cell holds (decision #142's join, held off).
    #[test]
    fn a_table_is_never_joined_across_a_swapped_page() {
        use crate::markdown::tests::{paged_tables, staff, FIRST, SECOND};
        let engine = paged_tables(&staff(&[]), &[FIRST, SECOND], crate::TABLE_DETECTION_V14);
        let joined = markdown(&engine).markdown;
        assert!(joined.contains("| Ann | Audit |") && joined.contains("| Bob | Tax |"));
        let reply =
            "![text](100,100,900,200) Ten invented words nobody printed on this made-up page";
        for (page, kept, gone) in [(2, "| Ann | Audit |", "Bob"), (1, "| Bob | Tax |", "Ann")] {
            let repr = assisted(engine.clone(), &[(page, "stop", reply)]);
            let (md, html) = (markdown(&repr), html(&repr));
            assert!(
                md.coverage.balances() && html.coverage.balances(),
                "page {page}"
            );
            assert!(
                md.markdown.contains(kept) && !md.markdown.contains(gone),
                "page {page}"
            );
            assert!(!html.html.contains(gone), "page {page}");
            assert_eq!(lines(&md.proposed), [(true, 1, vec![])], "page {page}");
            let joins = md
                .coverage
                .structural_erasures
                .iter()
                .any(|e| e.code == crate::markdown::TABLES_JOINED_ACROSS_PAGES);
            assert!(!joins, "page {page}");
        }
    }

    /// A `proposed` segment names exactly one block: none, or two, is no map.
    #[test]
    fn a_proposed_segment_names_one_block() {
        let segment = |ids: &[&str]| crate::Segment {
            kind: SegmentKind::Proposed,
            start: 0,
            end: 2,
            node_ids: ids.iter().map(|s| (*s).to_string()).collect(),
        };
        assert!(crate::AnchorMap::new(vec![segment(&["m1"])], "ab").is_ok());
        assert!(crate::AnchorMap::new(vec![segment(&[])], "ab").is_err());
        assert!(crate::AnchorMap::new(vec![segment(&["m1", "m2"])], "ab").is_err());
    }
}
