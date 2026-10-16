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

//! Stroke-ruled table detection: the grid a document draws as **lines** (v1-S8).
//!
//! # The third rule, and why it is a third rule
//!
//! `crate::tables` reconstructs a grid from rectangles the author *filled* — each cell is a box.
//! `crate::unruled` infers one from where the author *put text*. Neither sees the commonest way a
//! blank form is drawn: a **rule under each cell** and nothing else.
//!
//! `crate::content::flush_subpath` needs four or five points spanning two distinct x and two
//! distinct y before it will call a subpath a rectangle, so a two-point `m`/`l` stroked with `S`
//! produces nothing at all. That is the `stroke-ruled-tables-not-detected` limitation this engine
//! declared from v1-S1 until this slice retracted it, and v1-S7b measured what it cost: **every
//! one of the nine tagged tables `cfpb-home-loan-toolkit` missed sat on a page that strokes
//! segments — 103 cells, 65% of that document's gold.** Its page 13 draws an 8 × 4 loan worksheet
//! as 32 horizontal rules and nothing else.
//!
//! # The rule, in full
//!
//! 1. **Rule-rows.** Horizontal segments sharing a baseline within
//!    [`crate::tables::LATTICE_TOLERANCE`] are one rule-row, in x order.
//!
//! 2. **A rule-row must TILE.** Its segments must run end to end, each starting where the last
//!    finished. Two underlines with a gap between them are two underlines: only a contiguous run
//!    of rules is a row of cells. **A row drawn as one stroke tiles too** (`stroke-ruled-v2`,
//!    decision #72): a table ruled across its rows in single strokes and divided by vertical ones
//!    draws its cells so, and such a row divides only where vertical ink does (step 3). A lone
//!    underline still implies no grid — with nothing dividing it, it bounds one column, under
//!    step 7's floor — and step 8 asks after every band a single stroke rules.
//!
//! 3. **A band is consecutive rule-rows that agree about the columns.** The first tiling row sets
//!    the column lines; a later row joins the band only if **every one of its endpoints is already
//!    one of them**. It may rule fewer cells than the band has columns — a row with a blank cell
//!    does — but it may not introduce a boundary the band does not have. Measured: requiring exact
//!    equality instead split `cfpb-home-loan-toolkit` page 13's worksheet into a 4 × 4 and a 3 × 4
//!    at the one row whose first cell is blank. Since `stroke-ruled-v2` the band's columns are also
//!    the vertical strokes run across the whole of it, strictly between its ends.
//!
//! 4. **Rows are the regions BETWEEN rules, so nothing is invented.** *n* baselines bound *n − 1*
//!    rows. A form ruled under each cell does not draw the top edge of its first row, and this
//!    rule will not supply one: the cost is that such a table loses its first row, and the
//!    alternative is a coordinate no operator in the file produced. `docs/01-CONTRACT.md` §5 and
//!    the standing rule against invented coordinates both point the same way here.
//!
//! 5. **Coherence: every INTERIOR column line must be stroked across the whole band.** The
//!    author's own vertical ink has to agree about where the columns are — see below, because
//!    this is the precondition v1-S8 exists to get right.
//!
//! 6. **A face a form field's own rectangle reproduces is that field's box, not a cell**, and one
//!    of them refuses the band. See below.
//!
//! 7. **At least 2 × 2**, and a cap at [`crate::tables::MAX_FACES`] — the same two guards, for
//!    the same reasons, as both other rules.
//!
//! 8. **A band one of whose rows is a single stroke stands only where** (`stroke-ruled-v2`) it
//!    holds no form field's widget — a form's boxes are drawn so; no ruled row of it holds rows
//!    the page did not rule ([`crate::tables::rows_not_ruled`], the ruled rule's own clause); and
//!    no line of text set on two or more of its columns, three runs to a column at most, stands
//!    within a row's height above or below it — a table whose rules stop before its rows do, which
//!    the whitespace rule reads whole. Each is a declared refusal.
//!
//! # Step 5, which is the whole difference from the rule that was parked
//!
//! **The id is still `-v1`, and deliberately.** A rule version exists so that two artifacts either
//! side of it are correctly non-comparable — and the parked rule never shipped. It sat in
//! `docs/attic/stroke-ruled-v1/` as a patch, was never named by a profile and never produced a
//! table on any wire, so there is no artifact anywhere claiming `stroke-ruled-v1` that means
//! anything other than what this module does. Burning a `-v2` for a rule nobody ever ran would
//! make the id count builds rather than behaviours.
//!
//! The parked rule asked that **every face's own bottom edge be drawn**. Measured at S7b, that
//! precondition was both too strict and too loose, in the same slice:
//!
//! - **Too strict.** It refused `cfpb-home-loan-toolkit` page 13 — the worksheet the whole lead
//!   was named for — because one of its eight baselines rules three cells instead of four. That
//!   row's first cell is *blank*, and an author who leaves a blank cell unruled has not stopped
//!   drawing a grid. A face without its own rule under it is principle (b) of this slice.
//! - **Too loose.** It accepted six bands on `cfpb-home-loan-toolkit` pages 22, 23, 24 and 25 —
//!   Closing Disclosure pages that stroke hundreds of segments — because rules that merely happen
//!   to *end* at a common x imply a column boundary under that reading, and nothing checked
//!   whether the author had drawn one.
//!
//! Both are the same defect: **the parked rule never read the page's vertical ink at all.**
//! `extract` filtered non-horizontal segments away before the rule ever saw one.
//!
//! So the coherence precondition moves off the faces and onto the **lines**: every column line
//! *interior* to the band must be stroked as vertical ink running the band's full height,
//! collinear segments joined end to end first. This is the precondition
//! the retired `stroke-ruled-tables-not-detected` limitation itself named back at v1-S2 — *"every
//! face bounded by four edges rather than covered by one rectangle"* — and the version that
//! shipped is that idea with exactly one concession stated out loud:
//!
//! **The band's OUTER edges are exempt.** An interior line separates two cells, so an author who
//! did not draw one did not divide there; an outer edge is only where the ink stops. Requiring
//! the outer border too would refuse page 13, whose author stroked the three interior column
//! rules at x = 210, 326 and 442 and neither of the outer ones. Step 4 already makes the same
//! trade on the other axis, and for the same reason: a border nobody drew is not evidence, but
//! its absence is not evidence against the grid either.
//!
//! Measured, on the same corpus and in the same run: page 13 emits, pages 22 and 23 and the
//! second band on 24 are refused, and no document outside `cfpb-home-loan-toolkit` changes at all.
//!
//! # Step 6, and why a tax form stays silent
//!
//! `irs-form-1040-2025` yields **0 geometric tables** in every slice since v1-S1, and it is the
//! canary for that slice's 662-cell fabrication. Under step 5 alone it yields five, because its
//! entry boxes really are a stroked grid with their column rules drawn.
//!
//! What settles it is not the geometry but **whose rectangle it is**. On the 1040 a face and a
//! widget's `/Rect` are the *same box* — measured, `93.3 … 251.6 × 309 … 321` against a widget at
//! `145.0 … 251.2 × 309 … 321`, edge for edge. The ink that drew it is the field's own frame. On
//! `cfpb-home-loan-toolkit` page 13, which is *also* a fillable worksheet carrying 25 widgets, the
//! widget sits **inside** a larger printed cell — face `210 … 326 × 334.6 … 388.6` holding a
//! widget at `231.1 … 321.8 × 349.9 … 376.3`, inset on all four sides and less than half the
//! face's height. There the printed cell is a cell and the widget is content in it.
//!
//! So: **a face whose four edges are a form field's four edges is that field's box**, within the
//! same [`crate::tables::LATTICE_TOLERANCE`] every other edge comparison in this rule uses, and
//! one such face refuses the band. One is enough because a page cannot half-be a form, and this
//! is the direction that fails *closed* — v1-S4 already gives a widget a node of its own kind, and
//! a table rule emitting an annotation's rectangle as a cell would be claiming geometry that is
//! not page ink.
//!
//! No new constant is introduced by either step, and neither names a document, a page or a count.
//!
//! # Fabrication is impossible here, as everywhere
//!
//! A cell's text is the concatenation of runs **already extracted** whose origins fall inside it,
//! in reading order. No re-decode, no nearest-neighbour, no borrowing from a neighbour cell. A
//! face enclosing no run is emitted with empty text, because that is what the document put there
//! — and a blank worksheet is mostly empty faces on purpose.

use ethos_parser_core::{EngineError, IdAllocator, IdKind};

use crate::tables::{cross_check, DetectedCell, DetectedTable, QuantRect, RunOrigin};

/// A horizontal ruling line in page space, quantized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// The baseline it sits on.
    pub y: i64,
    /// Left end.
    pub x0: i64,
    /// Right end.
    pub x1: i64,
}

/// A **vertical** ruling line in page space, quantized (v1-S8).
///
/// Read as corroboration in step 5. A vertical segment never defines a row, and opens a column
/// only where it runs across a whole band strictly between its ends (`stroke-ruled-v2`, step 3);
/// otherwise a column line this rule emits is an endpoint of a horizontal rule that tiled, and the
/// verticals only say whether the author agreed it was a boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upright {
    /// The column line it sits on.
    pub x: i64,
    /// Top end, in page space, so the smaller number is higher on the page.
    pub y0: i64,
    /// Bottom end.
    pub y1: i64,
}

/// Why the stroke-ruled rule refused a candidate band.
///
/// **A disclosure, never a score**, exactly as for the other two rules. Each variant names a
/// precondition that failed; none grades how close the band came, because a "nearly a table"
/// number is a confidence field wearing a different hat (`docs/01-CONTRACT.md` §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A column line interior to the band is not stroked across it.
    ///
    /// The over-detection case, and the one `stroke-ruled-v1` could not see: rules that merely
    /// end at a common x imply a boundary nobody drew.
    ColumnLineNotStroked {
        /// Column lines interior to the band.
        interior: usize,
        /// How many of them the page strokes across the band's full height.
        stroked: usize,
    },
    /// A face of the band is a form field's own rectangle.
    ///
    /// The widget case: the box was drawn as the field's frame, so it is an annotation's geometry
    /// rather than a grid the page ruled.
    FaceIsAFormFieldBox {
        /// Faces the band implied.
        faces: usize,
    },
    /// The band exceeded the shared face ceiling.
    BandTooLarge {
        /// Faces implied.
        faces: usize,
    },
    /// `stroke-ruled-v2`: a band ruled by single strokes holds a form field's widget.
    FormFieldInBand {
        /// Faces the band implied.
        faces: usize,
    },
    /// `stroke-ruled-v2`: a ruled row of a band ruled by single strokes holds rows the page did
    /// not rule.
    RowsNotRuled {
        /// Faces the band implied.
        faces: usize,
    },
    /// `stroke-ruled-v2`: the rows of a band ruled by single strokes run on past its rules.
    RowsRunOn {
        /// Faces the band implied.
        faces: usize,
    },
}

impl Refusal {
    /// Which precondition failed, as a stable short name.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ColumnLineNotStroked { .. } => "a column line the page never drew",
            Self::FaceIsAFormFieldBox { .. } => "a cell that is a form field's own box",
            Self::BandTooLarge { .. } => "past the cell ceiling",
            Self::FormFieldInBand { .. } => "a form's boxes ruled by single strokes",
            Self::RowsNotRuled { .. } => "rows the page did not rule",
            Self::RowsRunOn { .. } => "rows running on past the rules",
        }
    }

    /// The reasoning behind a kind of refusal, stated once.
    pub fn explanation(&self) -> String {
        match self {
            Self::ColumnLineNotStroked { .. } => String::from(
                "Consecutive ruling lines ended at common x positions, which is what a row of \
                 cells looks like — but at least one of the column boundaries they imply is not \
                 drawn anywhere on the page. A column line INTERIOR to a grid separates two \
                 cells, so an author who never stroked one never divided there, and a grid built \
                 on it would be this engine reading a boundary into where two unrelated rules \
                 happen to stop. The band's outer edges are not asked for: an outer edge is only \
                 where the ink ends",
            ),
            Self::FaceIsAFormFieldBox { .. } => String::from(
                "A cell of the grid has the same four edges as a form field's own rectangle, so \
                 the box was drawn as that field's frame rather than as a rule dividing a table. \
                 A widget is already a node of its own kind here (v1-S4's forms slice), and \
                 emitting its rectangle a second time as a table cell would claim geometry that \
                 is not page ink. A printed cell that merely CONTAINS a smaller field box is \
                 unaffected — that is a real cell with a widget sitting in it",
            ),
            Self::BandTooLarge { .. } => format!(
                "the ruling lines implied more than the {}-cell ceiling for a reconstructed grid. \
                 Refused rather than truncated: a truncated table is a table with cells missing \
                 and no way to say which",
                crate::tables::MAX_FACES
            ),
            Self::FormFieldInBand { .. } => String::from(
                "Rules each drawn as one stroke, divided by vertical strokes, made a grid of boxes \
                 that holds a form field's widget. A form's boxes are drawn so, and its fields are \
                 already nodes of their own kind here; read as a table, the boxes would be cells \
                 the page drew to be filled in, not a table it printed",
            ),
            Self::RowsNotRuled { .. } => String::from(
                "A row the single-stroke rules bound holds, in every one of its cells, three or \
                 more lines on baselines its neighbours share: those are rows the page did not \
                 rule, and the grid would read them as one",
            ),
            Self::RowsRunOn { .. } => String::from(
                "A line of text set on two or more of the grid's columns, sparse as a table's row \
                 is, stands within a row's height above or below the single-stroke rules that bound \
                 it: the table runs on past where its rules stop, and the grid would read only the \
                 ruled part of it",
            ),
        }
    }

    /// This band's own numbers, without the reasoning.
    pub fn detail(&self) -> String {
        match self {
            Self::ColumnLineNotStroked { interior, stroked } => {
                format!("{stroked} of {interior} interior column lines are stroked")
            }
            Self::FaceIsAFormFieldBox { faces } => {
                format!("a form field's box is one of {faces} cells")
            }
            Self::BandTooLarge { faces } => format!("{faces} cells implied"),
            Self::FormFieldInBand { faces } => {
                format!("a form field's widget lies among {faces} cells")
            }
            Self::RowsNotRuled { faces } => format!("a row of {faces} cells holds unruled rows"),
            Self::RowsRunOn { faces } => format!("{faces} cells, and a row of text past them"),
        }
    }
}

/// What the stroke-ruled rule produced for one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The tables whose bands satisfied every precondition.
    pub tables: Vec<DetectedTable>,
    /// The first refusal across the page's bands, if one was built and refused.
    pub refusal: Option<Refusal>,
}

/// Detect stroke-ruled tables on one page.
///
/// `claimed` are boxes an earlier rule already owns; a band overlapping one is dropped, because
/// two tables claiming one region is the thing detection must not emit. `fields` are the
/// rectangles this page's form-field widgets declare, for step 6.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a band's geometry will not quantize into a well-formed box.
pub fn detect(
    page: u32,
    rules: &[Rule],
    uprights: &[Upright],
    runs: &[RunOrigin<'_>],
    fields: &[QuantRect],
    claimed: &[QuantRect],
    alloc: &mut IdAllocator,
) -> Result<Outcome, EngineError> {
    let mut tables = Vec::new();
    let mut refusal = None;
    for mut band in bands(rules) {
        divide(&mut band, uprights);
        match emit(page, &band, uprights, runs, fields, alloc)? {
            Ok(t) => {
                if !claimed.iter().any(|c| c.overlaps(t.rect))
                    && !tables
                        .iter()
                        .any(|p: &DetectedTable| p.rect.overlaps(t.rect))
                {
                    tables.push(t);
                }
            }
            Err(r) => refusal = refusal.or(r),
        }
    }
    Ok(Outcome { tables, refusal })
}

/// One candidate band: consecutive rule-rows agreeing about the columns.
struct Band {
    /// Baselines, top to bottom in page space.
    baselines: Vec<i64>,
    /// Column lines, left to right.
    columns: Vec<i64>,
    /// `stroke-ruled-v2`: one of its rows is a single stroke, so step 8 asks after it.
    single: bool,
}

/// Group the page's rules into candidate bands — steps 1 to 3 of the rule.
fn bands(rules: &[Rule]) -> Vec<Band> {
    let tol = crate::tables::LATTICE_TOLERANCE;
    let mut sorted: Vec<Rule> = rules.to_vec();
    sorted.sort_by_key(|r| (r.y, r.x0, r.x1));

    // Step 1. Rule-rows.
    let mut rows: Vec<Vec<Rule>> = Vec::new();
    for r in sorted {
        match rows.last_mut() {
            Some(row) if (r.y - row[0].y).abs() <= tol => row.push(r),
            _ => rows.push(vec![r]),
        }
    }

    // Step 2. A rule-row tiles when its segments run end to end — and, `stroke-ruled-v2`, a row
    // drawn as one stroke tiles, to be divided by vertical ink alone ([`divide`]).
    let tiles =
        |row: &Vec<Rule>| -> bool { row.windows(2).all(|w| (w[1].x0 - w[0].x1).abs() <= tol) };
    let edges = |row: &Vec<Rule>| -> Vec<i64> {
        let mut v: Vec<i64> = row.iter().flat_map(|r| [r.x0, r.x1]).collect();
        v.sort_unstable();
        v.dedup_by(|a, b| (*a - *b).abs() <= tol);
        v
    };

    // Step 3. Consecutive rows that agree about the columns.
    let mut out = Vec::new();
    let mut i = 0;
    while i < rows.len() {
        if !tiles(&rows[i]) {
            i += 1;
            continue;
        }
        let columns = edges(&rows[i]);
        let compatible = |row: &Vec<Rule>| -> bool {
            tiles(row)
                && edges(row)
                    .iter()
                    .all(|e| columns.iter().any(|c| (c - e).abs() <= tol))
        };
        let mut j = i;
        while j + 1 < rows.len() && compatible(&rows[j + 1]) {
            j += 1;
        }
        if j > i {
            out.push(Band {
                baselines: rows[i..=j].iter().map(|r| r[0].y).collect(),
                columns,
                single: rows[i..=j].iter().any(|r| r.len() == 1),
            });
        }
        i = j + 1;
    }
    out
}

/// `stroke-ruled-v2`, step 3: the band's columns are also the vertical strokes run across the
/// whole of it, strictly between its ends — the division a row drawn as one stroke leaves to
/// vertical ink. A stroke that crosses only part of the band divides nothing here.
fn divide(band: &mut Band, uprights: &[Upright]) {
    let tol = crate::tables::LATTICE_TOLERANCE;
    let (top, bottom) = (band.baselines[0], band.baselines[band.baselines.len() - 1]);
    let (left, right) = (band.columns[0], band.columns[band.columns.len() - 1]);
    for u in uprights {
        if u.x > left + tol
            && u.x < right - tol
            && !band.columns.iter().any(|c| (c - u.x).abs() <= tol)
            && column_is_stroked(uprights, u.x, top, bottom)
        {
            band.columns.push(u.x);
        }
    }
    band.columns.sort_unstable();
}

/// `stroke-ruled-v2`, step 8: whether the rows of `t` run on past its rules — a line of text set on
/// two or more of its columns, with no more runs than three to a column, as a table's row is and a
/// paragraph's line drawn glyph by glyph is not, within one row's height above or below it.
fn runs_on(t: &DetectedTable, runs: &[RunOrigin<'_>]) -> bool {
    let tol = crate::blocks::LINE_TOLERANCE;
    let pitch = (t.rect.y1 - t.rect.y0) / i64::from(t.rows.max(1));
    let columns: Vec<(i64, i64)> = (0..t.columns)
        .filter_map(|k| {
            t.cells
                .iter()
                .find(|c| c.position.column == k)
                .map(|c| (c.rect.x0, c.rect.x1))
        })
        .collect();
    let near: Vec<&RunOrigin<'_>> = runs
        .iter()
        .filter(|o| !o.text.trim().is_empty() && o.x >= t.rect.x0 && o.x < t.rect.x1)
        .filter(|o| {
            (o.y < t.rect.y0 && t.rect.y0 - o.y <= pitch)
                || (o.y > t.rect.y1 && o.y - t.rect.y1 <= pitch)
        })
        .collect();
    near.iter().any(|a| {
        let line: Vec<&&RunOrigin<'_>> = near.iter().filter(|b| (b.y - a.y).abs() <= tol).collect();
        let set: std::collections::BTreeSet<usize> = line
            .iter()
            .filter_map(|o| columns.iter().position(|&(x0, x1)| o.x >= x0 && o.x < x1))
            .collect();
        set.len() >= 2 && line.len() <= 3 * columns.len()
    })
}

/// Whether the page strokes column line `x` continuously from `y0` down to `y1`.
///
/// Collinear segments are joined end to end first, because a grid whose verticals are drawn one
/// cell at a time — four short strokes rather than one long one — has still drawn the line. Two
/// strokes that abut within [`crate::tables::LATTICE_TOLERANCE`] are one line, the same tolerance
/// and the same reasoning as step 2's tiling test.
fn column_is_stroked(uprights: &[Upright], x: i64, y0: i64, y1: i64) -> bool {
    let tol = crate::tables::LATTICE_TOLERANCE;
    let mut runs: Vec<(i64, i64)> = uprights
        .iter()
        .filter(|u| (u.x - x).abs() <= tol)
        .map(|u| (u.y0, u.y1))
        .collect();
    if runs.is_empty() {
        return false;
    }
    runs.sort_unstable();
    let mut merged: Vec<(i64, i64)> = vec![runs[0]];
    for (a, b) in runs.into_iter().skip(1) {
        let last = merged.last_mut().expect("seeded above");
        if a <= last.1 + tol {
            last.1 = last.1.max(b);
        } else {
            merged.push((a, b));
        }
    }
    merged.iter().any(|(a, b)| *a <= y0 + tol && *b >= y1 - tol)
}

/// Steps 4 to 7, and the emission.
#[allow(clippy::type_complexity)]
fn emit(
    page: u32,
    band: &Band,
    uprights: &[Upright],
    runs: &[RunOrigin<'_>],
    fields: &[QuantRect],
    alloc: &mut IdAllocator,
) -> Result<Result<DetectedTable, Option<Refusal>>, EngineError> {
    let tol = crate::tables::LATTICE_TOLERANCE;
    // Step 4. Rows are the regions BETWEEN rules. Nothing is invented above the first.
    let rows = band.baselines.len() - 1;
    let columns = band.columns.len() - 1;
    // At least 2 x 2, which is what `crate::unruled` requires and what `crate::tables` requires
    // in faces. Two stacked rules are two rules; one row of cells is a row of cells.
    if rows < 2 || columns < 2 {
        // No candidate existed. An ordinary page of ruled underlining reaches here, and declaring
        // it would put a limitation on every document that draws a line.
        return Ok(Err(None));
    }
    let faces = rows * columns;
    if faces > crate::tables::MAX_FACES {
        return Ok(Err(Some(Refusal::BandTooLarge { faces })));
    }

    // Step 5. Every INTERIOR column line must be stroked across the band. The outer two are
    // exempt: an outer edge is where the ink stops, an interior one is a division the author
    // either drew or did not make.
    let (top, bottom) = (band.baselines[0], band.baselines[rows]);
    let interior = columns - 1;
    let stroked = band.columns[1..columns]
        .iter()
        .filter(|x| column_is_stroked(uprights, **x, top, bottom))
        .count();
    if stroked != interior {
        return Ok(Err(Some(Refusal::ColumnLineNotStroked {
            interior,
            stroked,
        })));
    }

    // Step 6. A face a form field's rectangle reproduces is that field's box.
    for r in 0..rows {
        for c in 0..columns {
            let (x0, x1) = (band.columns[c], band.columns[c + 1]);
            let (y0, y1) = (band.baselines[r], band.baselines[r + 1]);
            if fields.iter().any(|f| {
                (f.x0 - x0).abs() <= tol
                    && (f.y0 - y0).abs() <= tol
                    && (f.x1 - x1).abs() <= tol
                    && (f.y1 - y1).abs() <= tol
            }) {
                return Ok(Err(Some(Refusal::FaceIsAFormFieldBox { faces })));
            }
        }
    }

    let rect = QuantRect {
        x0: band.columns[0],
        y0: band.baselines[0],
        x1: band.columns[columns],
        y1: band.baselines[rows],
    };
    // Step 8, `stroke-ruled-v2`: a band ruled by single strokes holding a form field's widget is a
    // form's boxes.
    if band.single && fields.iter().any(|f| f.overlaps(rect)) {
        return Ok(Err(Some(Refusal::FormFieldInBand { faces })));
    }
    // The id the table takes if it stands, drawn from a copy until step 8 has spoken, so a band
    // it refuses moves no id after it.
    let id = alloc.clone().next(IdKind::Table)?;
    let mut cells = Vec::with_capacity(faces);
    for r in 0..rows {
        for c in 0..columns {
            let face = QuantRect {
                x0: band.columns[c],
                y0: band.baselines[r],
                x1: band.columns[c + 1],
                y1: band.baselines[r + 1],
            };
            // Assigned by ORIGIN, never by ink box — the same rule both other detectors follow.
            let mut run_indices: Vec<usize> = (0..runs.len())
                .filter(|i| {
                    let run = &runs[*i];
                    run.x >= face.x0 && run.x < face.x1 && run.y >= face.y0 && run.y < face.y1
                })
                .collect();
            run_indices.sort_unstable();
            let text: String = run_indices.iter().map(|i| runs[*i].text).collect();
            cells.push(DetectedCell {
                position: ethos_parser_core::TableCellPosition {
                    row: r as u32,
                    column: c as u32,
                    // Always 1. A ruling line carries no evidence of a span: a cell that "spans
                    // two columns" and a row that simply ruled one of its boundaries away are the
                    // same picture, and step 3 already refuses a row that invents a boundary.
                    rowspan: 1,
                    colspan: 1,
                    table_id: id.clone(),
                },
                rect: face,
                run_indices,
                text,
            });
        }
    }
    let check = cross_check(rect, rows as u32, columns as u32, &cells);
    rect.as_qrect_checked()?;
    let table = DetectedTable {
        id,
        page,
        rect,
        rows: rows as u32,
        columns: columns as u32,
        cells,
        check,
        tagged_check: None,
        rule: ethos_parser_core::TABLE_DETECTION_STROKE_V2.to_string(),
    };
    // Step 8, the rest: no ruled row holding rows the page did not rule, and no rows running on
    // past the rules.
    if band.single && crate::tables::rows_not_ruled(&table, runs) {
        return Ok(Err(Some(Refusal::RowsNotRuled { faces })));
    }
    if band.single && runs_on(&table, runs) {
        return Ok(Err(Some(Refusal::RowsRunOn { faces })));
    }
    alloc.next(IdKind::Table)?;
    Ok(Ok(table))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_parser_core::{Profile, QUANTUM_PER_POINT};

    fn alloc() -> IdAllocator {
        IdAllocator::new(Profile::default().profile_sha256().expect("hashes"))
    }

    fn pt(v: f64) -> i64 {
        (v * f64::from(QUANTUM_PER_POINT)).round() as i64
    }

    fn rule(y: f64, x0: f64, x1: f64) -> Rule {
        Rule {
            y: pt(y),
            x0: pt(x0),
            x1: pt(x1),
        }
    }

    fn upright(x: f64, y0: f64, y1: f64) -> Upright {
        Upright {
            x: pt(x),
            y0: pt(y0),
            y1: pt(y1),
        }
    }

    fn face(x0: f64, y0: f64, x1: f64, y1: f64) -> QuantRect {
        QuantRect {
            x0: pt(x0),
            y0: pt(y0),
            x1: pt(x1),
            y1: pt(y1),
        }
    }

    /// `cfpb-home-loan-toolkit` page 13's loan worksheet, in page space.
    ///
    /// **Transcribed from `docs/table-gate-v1.md`**, which records the eight baselines and the
    /// four column spans in user space, flipped into the top-left system this rule works in
    /// (792 pt page). Eight baselines 54 pt apart bar the last, four segments each — except the
    /// third from the top, which rules three cells because the row's first cell is blank on the
    /// printed worksheet.
    ///
    /// A fixture rather than the document, deliberately: this pins the geometry that motivated
    /// v1-S8 with no corpus present, so the case survives a machine that cannot find the
    /// benchmark PDFs.
    fn page_13() -> (Vec<Rule>, Vec<Upright>) {
        let baselines = [280.6, 334.6, 388.6, 442.6, 496.6, 550.6, 604.6, 667.6];
        let columns = [54.0, 210.0, 326.0, 442.0, 558.0];
        let mut rules = Vec::new();
        for (i, y) in baselines.iter().enumerate() {
            // The blank first cell: this baseline rules columns 1..4 and not column 0.
            let first = usize::from(i == 3);
            for c in first..4 {
                rules.push(rule(*y, columns[c], columns[c + 1]));
            }
        }
        // The three INTERIOR column rules the page strokes. The outer two it does not.
        let uprights = columns[1..4]
            .iter()
            .map(|x| upright(*x, baselines[0], baselines[7]))
            .collect();
        (rules, uprights)
    }

    /// **The table v1-S8 exists to find.** `stroke-ruled-v1` as parked refused this band outright.
    #[test]
    fn a_blank_cell_does_not_refuse_the_worksheet_it_sits_in() {
        let (rules, uprights) = page_13();
        let out = detect(13, &rules, &uprights, &[], &[], &[], &mut alloc()).expect("detects");
        assert_eq!(out.tables.len(), 1, "one band, one table");
        let t = &out.tables[0];
        // Eight baselines bound SEVEN rows: the worksheet's header row has no top edge drawn and
        // step 4 does not supply one. The tagged table is an 8 x 4 and this is the honest 7 x 4 —
        // `codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED` is where that is declared.
        assert_eq!((t.rows, t.columns), (7, 4));
        assert_eq!(t.cells.len(), 28);
        assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_STROKE_V2);
        // The blank cell is still emitted, and it is EMPTY rather than filled from a neighbour.
        assert!(t.cells.iter().all(|c| c.text.is_empty()));
    }

    /// The same worksheet with its interior column rules **not drawn** is not a grid.
    ///
    /// This is the half `stroke-ruled-v1` could not see: rules that merely end at a common x
    /// imply a boundary, and without vertical ink nothing says the author drew one. It is what
    /// refuses `cfpb-home-loan-toolkit`'s Closing Disclosure pages.
    #[test]
    fn rules_ending_at_a_common_x_are_not_a_column_the_author_drew() {
        let (rules, _) = page_13();
        let out = detect(13, &rules, &[], &[], &[], &[], &mut alloc()).expect("detects");
        assert!(out.tables.is_empty(), "no vertical ink, no columns");
        assert_eq!(
            out.refusal,
            Some(Refusal::ColumnLineNotStroked {
                interior: 3,
                stroked: 0
            }),
            "and it says so rather than declining in silence"
        );
    }

    /// A column line stroked **cell by cell** is still a column line.
    ///
    /// Four short abutting strokes and one long one are the same line, which is the same reading
    /// step 2 gives a row of rules.
    #[test]
    fn a_column_ruled_one_cell_at_a_time_still_counts() {
        let (rules, _) = page_13();
        let baselines = [280.6, 334.6, 388.6, 442.6, 496.6, 550.6, 604.6, 667.6];
        let mut uprights = Vec::new();
        for x in [210.0, 326.0, 442.0] {
            for w in baselines.windows(2) {
                uprights.push(upright(x, w[0], w[1]));
            }
        }
        let out = detect(13, &rules, &uprights, &[], &[], &[], &mut alloc()).expect("detects");
        assert_eq!(out.tables.len(), 1);
        assert_eq!((out.tables[0].rows, out.tables[0].columns), (7, 4));
    }

    /// **A tax form's entry boxes are not a table.**
    ///
    /// The `irs-form-1040-2025` principle, pinned without the form: a grid whose faces ARE the
    /// widget rectangles is a form's field boxes. Measured on that document, a face runs
    /// `93.3 … 251.6 x 309 … 321` against a widget at `145.0 … 251.2 x 309 … 321`.
    #[test]
    fn a_grid_of_form_field_boxes_is_not_a_table() {
        let (rules, uprights) = page_13();
        let columns = [54.0, 210.0, 326.0, 442.0, 558.0];
        let baselines = [280.6, 334.6, 388.6, 442.6, 496.6, 550.6, 604.6, 667.6];
        // One widget reproducing one face, within LATTICE_TOLERANCE on every side.
        let fields = vec![face(
            columns[1] + 0.5,
            baselines[0],
            columns[2],
            baselines[1] - 0.5,
        )];
        let out = detect(13, &rules, &uprights, &[], &fields, &[], &mut alloc()).expect("detects");
        assert!(
            out.tables.is_empty(),
            "the boxes are the form's, not a grid's"
        );
        assert_eq!(
            out.refusal,
            Some(Refusal::FaceIsAFormFieldBox { faces: 28 })
        );
    }

    /// A widget **inside** a printed cell leaves that cell a cell.
    ///
    /// The other half of the same principle, and the reason `cfpb-home-loan-toolkit` page 13
    /// survives it while carrying 25 widgets: there the field box is inset within a larger ruled
    /// cell, which is a real cell with something sitting in it.
    #[test]
    fn a_field_sitting_inside_a_ruled_cell_leaves_it_a_cell() {
        let (rules, uprights) = page_13();
        // The page-13 shape, measured: face 210 … 326 x 334.6 … 388.6 holding a widget at
        // 231.1 … 321.8 x 349.9 … 376.3.
        let fields = vec![face(231.1, 349.9, 321.8, 376.3)];
        let out = detect(13, &rules, &uprights, &[], &fields, &[], &mut alloc()).expect("detects");
        assert_eq!(out.tables.len(), 1);
        assert_eq!((out.tables[0].rows, out.tables[0].columns), (7, 4));
    }

    /// A region an earlier rule already claimed does not get a second grid.
    #[test]
    fn a_band_over_a_region_the_ruled_rule_took_is_dropped() {
        let (rules, uprights) = page_13();
        let claimed = vec![face(54.0, 280.6, 558.0, 667.6)];
        let out = detect(13, &rules, &uprights, &[], &[], &claimed, &mut alloc()).expect("detects");
        assert!(out.tables.is_empty(), "the ruled rule wins on overlap");
    }

    /// `stroke-ruled-v2`'s fixture: two rows ruled across in single strokes from x = 100 to 400 at
    /// y = 100, 140 and 180, the outer sides stroked, and an interior column line at x = 250 run
    /// over `interior` of the band's height; a run on each cell's baseline.
    fn single_strokes(interior: (f64, f64)) -> (Vec<Rule>, Vec<Upright>, Vec<RunOrigin<'static>>) {
        let rules = [100.0, 140.0, 180.0]
            .map(|y| rule(y, 100.0, 400.0))
            .to_vec();
        let uprights = vec![
            upright(100.0, 100.0, 180.0),
            upright(250.0, interior.0, interior.1),
            upright(400.0, 100.0, 180.0),
        ];
        let runs = [
            (110.0, 125.0, "A1"),
            (260.0, 125.0, "A2"),
            (110.0, 165.0, "B1"),
            (260.0, 165.0, "B2"),
        ]
        .map(|(x, y, text)| RunOrigin {
            x: pt(x),
            y: pt(y),
            text,
        })
        .to_vec();
        (rules, uprights, runs)
    }

    /// **A table ruled across its rows in single strokes and divided by vertical ones is a grid**
    /// (`stroke-ruled-v2`): `-v1`'s tiling step formed no row of it, and the page reported no
    /// candidate at all.
    #[test]
    fn rows_drawn_as_one_stroke_and_divided_by_vertical_ones_are_a_grid() {
        let (rules, uprights, runs) = single_strokes((100.0, 180.0));
        let out = detect(1, &rules, &uprights, &runs, &[], &[], &mut alloc()).expect("detects");
        assert_eq!(out.tables.len(), 1, "{:?}", out.refusal);
        let t = &out.tables[0];
        assert_eq!((t.rows, t.columns), (2, 2));
        let texts: Vec<&str> = t.cells.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(texts, ["A1", "A2", "B1", "B2"]);
        assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_STROKE_V2);
    }

    /// A single stroke nothing divides is no grid, and a vertical stroke crossing only part of the
    /// band divides nothing: one column each time, under step 7's floor, with nothing to declare.
    #[test]
    fn a_single_stroke_divides_only_where_vertical_ink_crosses_the_whole_band() {
        let (rules, uprights, runs) = single_strokes((100.0, 180.0));
        let outer: Vec<Upright> = vec![uprights[0], uprights[2]];
        for (uprights, why) in [
            (outer, "nothing divides it"),
            (
                single_strokes((100.0, 140.0)).1,
                "the interior stroke crosses one row of two",
            ),
        ] {
            let out = detect(1, &rules, &uprights, &runs, &[], &[], &mut alloc()).expect("detects");
            assert!(out.tables.is_empty(), "{why}: {:?}", out.tables);
            assert_eq!(out.refusal, None, "{why}");
        }
    }

    /// **Step 8.** A band ruled by single strokes stands only where no form field's widget lies in
    /// it, no ruled row of it holds rows the page did not rule, and no row of text runs on past its
    /// rules — while a paragraph's line drawn glyph by glyph below it does not count as one.
    #[test]
    fn a_band_ruled_by_single_strokes_is_asked_three_more_things() {
        let (rules, uprights, runs) = single_strokes((100.0, 180.0));
        let widget = vec![face(120.0, 110.0, 200.0, 130.0)];
        let out = detect(1, &rules, &uprights, &runs, &widget, &[], &mut alloc()).expect("detects");
        assert!(out.tables.is_empty());
        assert_eq!(out.refusal, Some(Refusal::FormFieldInBand { faces: 4 }));

        let mut unruled = runs[..2].to_vec();
        for y in [150.0, 160.0, 170.0] {
            unruled.push(RunOrigin {
                x: pt(110.0),
                y: pt(y),
                text: "x",
            });
            unruled.push(RunOrigin {
                x: pt(260.0),
                y: pt(y),
                text: "y",
            });
        }
        let out = detect(1, &rules, &uprights, &unruled, &[], &[], &mut alloc()).expect("detects");
        assert!(out.tables.is_empty());
        assert_eq!(out.refusal, Some(Refusal::RowsNotRuled { faces: 4 }));

        let mut on = runs.clone();
        on.extend([(110.0, "C1"), (260.0, "C2")].map(|(x, text)| RunOrigin {
            x: pt(x),
            y: pt(195.0),
            text,
        }));
        let out = detect(1, &rules, &uprights, &on, &[], &[], &mut alloc()).expect("detects");
        assert!(out.tables.is_empty());
        assert_eq!(out.refusal, Some(Refusal::RowsRunOn { faces: 4 }));

        let mut paragraph = runs.clone();
        paragraph.extend((0..7).map(|k| RunOrigin {
            x: pt(110.0 + 40.0 * f64::from(k)),
            y: pt(195.0),
            text: "w",
        }));
        let out =
            detect(1, &rules, &uprights, &paragraph, &[], &[], &mut alloc()).expect("detects");
        assert_eq!(
            out.tables.len(),
            1,
            "a paragraph's line is no row of the table"
        );

        // A caption under one column is no row either, and a row of text further off than a row's
        // height is not this table's.
        for (extra, why) in [
            (vec![(110.0, 195.0, "Source")], "a caption under one column"),
            (
                vec![(110.0, 230.0, "C1"), (260.0, 230.0, "C2")],
                "a row further off than one",
            ),
        ] {
            let mut near = runs.clone();
            near.extend(extra.into_iter().map(|(x, y, text)| RunOrigin {
                x: pt(x),
                y: pt(y),
                text,
            }));
            let out = detect(1, &rules, &uprights, &near, &[], &[], &mut alloc()).expect("detects");
            assert_eq!(out.tables.len(), 1, "{why}");
        }
    }

    /// Step 8 asks only after a band a single stroke rules: the worksheet, tiled cell by cell,
    /// stands with a row of text set on its columns just below it.
    #[test]
    fn a_band_its_rows_tile_is_not_asked_step_8() {
        let (rules, uprights) = page_13();
        let below = [
            (60.0, 680.0, "Total"),
            (220.0, 680.0, "12"),
            (330.0, 680.0, "34"),
        ]
        .map(|(x, y, text)| RunOrigin {
            x: pt(x),
            y: pt(y),
            text,
        });
        let out = detect(13, &rules, &uprights, &below, &[], &[], &mut alloc()).expect("detects");
        assert_eq!(out.tables.len(), 1, "{:?}", out.refusal);
    }

    /// A band step 8 refuses moves no id: the next band's table is numbered as if it were alone.
    #[test]
    fn a_band_step_8_refuses_moves_no_table_id() {
        let (mut rules, mut uprights, mut runs) = single_strokes((100.0, 180.0));
        runs.extend([(110.0, "C1"), (260.0, "C2")].map(|(x, text)| RunOrigin {
            x: pt(x),
            y: pt(195.0),
            text,
        }));
        // A second table further down, ruled to other ends, so it is a band of its own.
        rules.extend([300.0, 340.0, 380.0].map(|y| rule(y, 100.0, 500.0)));
        uprights.extend([100.0, 300.0, 500.0].map(|x| upright(x, 300.0, 380.0)));
        let out = detect(1, &rules, &uprights, &runs, &[], &[], &mut alloc()).expect("detects");
        assert_eq!(out.tables.len(), 1);
        assert_eq!(out.refusal, Some(Refusal::RowsRunOn { faces: 4 }));
        assert_eq!(out.tables[0].id, alloc().next(IdKind::Table).unwrap());
    }

    /// Every refusal explains itself, and none of them scores anything.
    #[test]
    fn every_refusal_explains_itself_without_scoring_anything() {
        for r in [
            Refusal::ColumnLineNotStroked {
                interior: 3,
                stroked: 1,
            },
            Refusal::FaceIsAFormFieldBox { faces: 20 },
            Refusal::BandTooLarge { faces: 99_999 },
            Refusal::FormFieldInBand { faces: 4 },
            Refusal::RowsNotRuled { faces: 4 },
            Refusal::RowsRunOn { faces: 4 },
        ] {
            assert!(!r.kind().is_empty());
            assert!(!r.explanation().is_empty());
            let d = r.detail();
            assert!(!d.is_empty());
            for scored in ["confidence", "score", "likely", "probably", "%"] {
                assert!(
                    !d.to_ascii_lowercase().contains(scored),
                    "`{scored}` in a refusal detail: {d}"
                );
            }
        }
    }
}
