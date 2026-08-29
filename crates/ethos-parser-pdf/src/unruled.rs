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

//! Unruled-table detection: the grid a document implies by **alignment** (v1-S2).
//!
//! # A different rule, not a wider one
//!
//! `crate::tables` reconstructs a grid the author *drew*. This one infers a grid from where the
//! author *put text*. They are pinned under different ids — `ruled-rects-v3` and
//! [`ethos_parser_core::TABLE_DETECTION_UNRULED_V1`] — and every table on the wire names the one that
//! produced it, because "the document drew this grid" and "we decided this was a grid" are
//! claims of very different strength and a consumer is entitled to tell them apart.
//!
//! # The rule, in full
//!
//! 1. **Column and row lines from origins.** Every run contributes its x-origin and its y-origin.
//!    Origins within [`ALIGN_TOLERANCE`] of each other are one line — the same folding
//!    `crate::tables` does to rectangle edges, on the same quantum, for the same reason.
//!
//!    **Folded by tolerance, not grown by gaps.** Growing groups until a gap appears was tried
//!    first and is wrong, because single-linkage grouping *chains*: origins at 90pt, 96pt, 102pt
//!    and 108pt each sit inside the gutter of the next, so they collapse into one "column" 18pt
//!    wide that nothing in the document aligns to. Measured on two lines of word-split prose,
//!    that chaining turned six words into a 2 × 3 table — the exact fabrication this slice may
//!    not commit, arrived at from the other direction. A tolerance fold cannot chain past
//!    [`ALIGN_TOLERANCE`], so a column line means what it says: text really is aligned there.
//!
//! 2. **A gutter floor between lines.** Adjacent column lines must be at least
//!    [`COLUMN_GUTTER_MIN`] apart and adjacent row lines at least [`ROW_GUTTER_MIN`]. Two lines
//!    closer than that are not two columns; they are one column's worth of text that failed to
//!    line up, or the word spacing inside a paragraph. Refused, not merged — merging them would
//!    be deciding which of two disagreeing alignments the author meant.
//!
//! 3. **At least 2 × 2.** One row of aligned text is a line; one column of it is a list. Neither
//!    is a table, and emitting a 1 × N "table" would find one on any page with a tab stop.
//!
//!    Falling short here is **not** a refusal and is not declared as one: no candidate existed to
//!    refuse. Every ordinary page of prose falls short here, and a limitation that rides on every
//!    document carries no information.
//!
//! 4. **The coherence precondition: every face holds a run.** This is the alignment analogue of
//!    what `crate::tables` requires of ruled grids — there, every lattice face must be covered by
//!    a rectangle the document painted; here, every face must contain a run the document placed.
//!    In both cases the lattice has to be *explained by evidence*, face by face, and in both
//!    cases the failure mode it prevents is the same one:
//!
//!    > Building one lattice from every rectangle on `irs-form-1040-2025` produced a 662-cell
//!    > table with a cell spanning 75 rows by 45 columns (v1-S1, measured). Building one lattice
//!    > from every *origin* on the same document is the identical mistake with a different input,
//!    > and this precondition is what refuses it: the form's labels and values imply a lattice of
//!    > tens of thousands of faces, of which barely any hold a run.
//!
//!    Measured on the same document: page 1 implies 23 276 faces from 1 146 runs and page 2
//!    10 848 faces from 830. Both are refused, and both were refused by the cap as well — the two
//!    guards are deliberately redundant.
//!
//! 5. **The document must have written it across the rows.** A page of newspaper columns and a
//!    table are the *same arrangement of origins* — `synthetic/two-columns` is four runs in a
//!    flawless 2 × 2, and no amount of geometry separates it from a two-row table. What separates
//!    them is something the document itself carries: **the order the author emitted the text
//!    in.** A table is written row by row; columns are written column by column. Measured on the
//!    corpus: `table-regular-grid` emits `Name, Score, Alpha, 10, Beta, 12` — across. Its
//!    same-shaped neighbour `two-columns` emits `Right top, Right bottom, Left top, Left bottom`
//!    — down.
//!
//!    So a candidate whose runs do not arrive in row-major face order is refused. This is
//!    deliberately author evidence rather than another threshold: every other way of telling
//!    these two apart is a number tuned until the fixtures fall on the right side of it, and a
//!    tuned number would not survive the next document. The cost is declared — a table whose
//!    author emitted its cells out of order is missed rather than guessed at, which is the trade
//!    this slice makes everywhere.
//!
//!    Reading order itself is untouched **by this module**. This *reads* the emission order as
//!    evidence; it never reorders anything. v1-S5 added a reading-order rule elsewhere
//!    (`crate::reading_order`), and it runs AFTER detection precisely so this precondition still
//!    sees the order the content stream produced — a rule that reordered first would be feeding
//!    the detector its own output.
//!
//! 6. **A lattice-size cap.** Past [`MAX_FACES`] this is refused outright rather than emitted
//!    truncated. A truncated table is a table with cells missing and no way to say so.
//!
//! # What this rule deliberately cannot do
//!
//! **One run per cell, in practice.** Two runs land in the same cell only when their origins fold
//! into the same column line *and* the same row line — that is, when the document showed them at
//! very nearly the same point. A cell whose text was `Tj`-split into two runs a few points apart
//! opens a second column instead, and the resulting lattice then fails the gutter floor or
//! coherence and is refused. That is a real recall cost, and it is the price of rule 1: the
//! alternative that finds those cells is the chaining that fabricates tables out of prose.
//!
//! **No merged cells.** A span in a ruled table is evidence — a rectangle really does cross two
//! faces. Alignment offers no such evidence: a cell that "spans two columns" and a cell whose
//! text is simply wide are the same picture. So every unruled cell has `rowspan == colspan == 1`,
//! and a grid that would need a merge fails coherence (the slot the merge would swallow holds no
//! run) and is refused rather than guessed at. Inventing merges to soak up empty slots is the
//! shipped ODL adapter's defect run backwards, and it is not done here.
//!
//! **No empty cells.** Coherence requires a run in every face, so an unruled table has no empty
//! cell to fill in. That is stricter than the ruled rule, which does emit an empty cell when the
//! document painted a box around nothing — a painted box is evidence that a cell exists, and
//! nothing plays that part for alignment.
//!
//! **No reordering.** The lattice groups origins; it never touches the page's Extracted node
//! list. v1-S5's `gutter-columns-v1` does reorder that list, and it does so after this module has
//! run and answered — a table's runs then travel as one atom, keeping the order its cell text was
//! concatenated in.
//!
//! # Fabrication is impossible here, not merely avoided
//!
//! A cell's text is the concatenation of runs **already extracted** whose origins fall in it, in
//! reading order. No re-decode, no nearest-neighbour, no borrowing from a neighbour cell.

use ethos_parser_core::{EngineError, IdAllocator, IdKind};

use crate::tables::{cross_check, DetectedCell, DetectedTable, QuantRect, RunOrigin};

/// How close two origins must be, in centipoints, to be one lattice line.
///
/// 150 — one and a half points, deliberately the same as `crate::tables::LATTICE_TOLERANCE`. Both
/// answer the same question ("are these two coordinates the same coordinate?") about the same
/// quantized page space, and giving them different answers would mean a document's ruled and
/// unruled grids disagreed about what "aligned" means.
///
/// Part of [`ethos_parser_core::TABLE_DETECTION_UNRULED_V1`]: changing it is a rule-version event and
/// moves `profile_sha256`.
pub const ALIGN_TOLERANCE: i64 = 150;

/// The smallest distance between two adjacent **column** lines, in centipoints.
///
/// 1 200 — twelve points. Sized against what it must reject rather than what it must accept: an
/// inter-word space in body text is roughly a quarter to a third of the type size, so at 12pt it
/// is 3–4pt and at 24pt still under 8pt. A twelve-point floor sits above all of those and far
/// below any real column gutter, which runs to tens of points. Two lines closer than this are not
/// two columns.
///
/// Part of [`ethos_parser_core::TABLE_DETECTION_UNRULED_V1`].
pub const COLUMN_GUTTER_MIN: i64 = 1_200;

/// The smallest distance between two adjacent **row** lines, in centipoints.
///
/// 600 — six points. Lower than the column floor on purpose: consecutive table rows are one line
/// apart, and line spacing is routinely tighter than a column gutter. What it excludes is runs
/// meant to sit on one baseline that missed it by more than [`ALIGN_TOLERANCE`] — those are a
/// disagreement about where the line is, not two rows.
///
/// Part of [`ethos_parser_core::TABLE_DETECTION_UNRULED_V1`].
pub const ROW_GUTTER_MIN: i64 = 600;

/// How far the table's box extends beyond the extreme run origins, in centipoints.
///
/// 300 — three points, a **fixed quantum**, and specifically *not* a function of font size. A box
/// derived from a font size is the defect `docs/01-CONTRACT.md` §5 and the parity checklist's P5
/// both name: pdf-inspector makes `height` the same variable as the type size, and the result is
/// a box that is confidently wrong. This padding claims nothing about ink. It exists because an
/// origin is a point and a table needs an area, and it is declared here so a reader knows exactly
/// how much of the box is inferred.
///
/// Part of [`ethos_parser_core::TABLE_DETECTION_UNRULED_V1`].
pub const ORIGIN_BOX_PADDING: i64 = 300;

/// The most faces an unruled lattice may have.
///
/// Same value and the same reasoning as the ruled cap: past it, this is not a table, and the
/// answer is a refusal rather than a truncation.
pub const MAX_FACES: usize = 4096;

/// Why the alignment rule refused a candidate lattice.
///
/// **A disclosure, never a score.** Each variant names a precondition that failed. None of them
/// grades how close the candidate came, because a "nearly a table" number is a confidence field
/// wearing a different hat (`docs/01-CONTRACT.md` §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The lattice exceeded [`MAX_FACES`].
    LatticeTooLarge {
        /// Faces implied.
        faces: usize,
    },
    /// Faces of the lattice held no run, so the grid was not explained by the text.
    ///
    /// The 1040 case, and the near-miss case: columns that align on some rows and not others
    /// leave holes, and a hole is a refusal rather than an invented empty cell.
    FacesWithoutText {
        /// How many faces held no run origin.
        empty: usize,
        /// How many faces the lattice had.
        faces: usize,
    },
    /// Two adjacent lines sat closer together than the gutter floor.
    ///
    /// Word spacing inside a paragraph, or a column that lines up on some rows and not others.
    /// Refused rather than merged: merging two disagreeing alignments means picking one, and
    /// nothing in the document says which.
    GutterBelowFloor {
        /// `true` for a column gutter, `false` for a row one.
        columns: bool,
        /// The offending distance, in centipoints.
        gap: i64,
        /// The floor it fell short of.
        floor: i64,
    },
    /// The document emitted the text down the columns rather than across the rows.
    ///
    /// See [`detect`]'s step 4. A page of newspaper columns and a table are the same picture
    /// once you have thrown away the order the author wrote them in, so the order is not thrown
    /// away.
    EmissionOrderNotRowMajor {
        /// The face the run at [`Self::EmissionOrderNotRowMajor::at`] belongs to, as a row-major
        /// ordinal.
        ordinal: usize,
        /// The largest ordinal already seen when it arrived.
        after: usize,
        /// Its index into the page's run list.
        at: usize,
    },
}

impl Refusal {
    /// The wire detail for the limitation this refusal rides on.
    pub fn detail(&self) -> String {
        match self {
            Self::LatticeTooLarge { faces } => format!(
                "the text implied {faces} cells, past the {MAX_FACES}-cell ceiling for an \
                 inferred grid. Refused rather than truncated: a truncated table is a table with \
                 cells missing and no way to say which"
            ),
            Self::FacesWithoutText { empty, faces } => format!(
                "the text implied a grid of {faces} cells but only filled {}: {empty} would have \
                 been empty. An inferred grid must be explained by the text face by face, or it \
                 is a lattice this engine drew rather than one the document implied. Filling \
                 those cells in is the fabrication this precondition exists to refuse",
                faces - empty
            ),
            Self::GutterBelowFloor {
                columns,
                gap,
                floor,
            } => format!(
                "two adjacent {} lines sat {gap} centipoints apart, under the {floor}-centipoint \
                 floor for an inferred grid. That distance is word spacing or a near miss, not a \
                 gutter. The two were refused rather than merged: merging them would mean \
                 choosing between two alignments the document does not choose between",
                if *columns { "column" } else { "row" }
            ),
            Self::EmissionOrderNotRowMajor { ordinal, after, at } => format!(
                "the arrangement fits a grid, but the document wrote it DOWN the columns rather \
                 than ACROSS the rows: run {at} belongs to cell {ordinal} in row-major order and \
                 arrived after cell {after}. Text laid out in columns and text laid out in a \
                 table are the same picture once the author's own ordering is discarded, so it \
                 is not discarded. Multi-column layout is a reading-order question and is \
                 declared as its own limitation"
            ),
        }
    }
}

/// What the alignment rule produced for one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The table, if a candidate satisfied every precondition.
    pub table: Option<DetectedTable>,
    /// Why a candidate was refused, if one was built and refused.
    pub refusal: Option<Refusal>,
}

/// Infer an unruled table from text alignment on one page.
///
/// `considered` indexes into `runs`: the runs no ruled table already claims. Passing indices
/// rather than a filtered slice keeps `DetectedCell::run_indices` addressing the page's real run
/// list, which is what a consumer resolves them against.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the inferred geometry will not quantize into a well-formed box.
pub fn detect(
    page: u32,
    runs: &[RunOrigin<'_>],
    considered: &[usize],
    alloc: &mut IdAllocator,
) -> Result<Outcome, EngineError> {
    if considered.is_empty() {
        return Ok(Outcome {
            table: None,
            refusal: None,
        });
    }

    // Step 1. Fold, never grow: see the module header on chaining.
    let columns = fold(considered.iter().map(|i| runs[*i].x));
    let rows = fold(considered.iter().map(|i| runs[*i].y));

    // Step 3. Below 2x2 there is **no candidate at all**, so there is nothing to declare. This is
    // not a near miss; it is an ordinary page. Reporting a refusal here would put the near-miss
    // limitation on almost every document in existence, and a limitation that appears everywhere
    // carries no information — which is precisely what was wrong with the blanket
    // `unruled-tables-not-detected` this slice retired.
    if rows.len() < 2 || columns.len() < 2 {
        return Ok(Outcome {
            table: None,
            refusal: None,
        });
    }

    // Step 2. The gutter floor, on both axes.
    if let Some(r) = gutter_fault(&columns, COLUMN_GUTTER_MIN, true)
        .or_else(|| gutter_fault(&rows, ROW_GUTTER_MIN, false))
    {
        return Ok(Outcome {
            table: None,
            refusal: Some(r),
        });
    }

    // Step 6. Checked before occupancy so a pathological page cannot make us build the map first.
    let faces = rows.len() * columns.len();
    if faces > MAX_FACES {
        return Ok(Outcome {
            table: None,
            refusal: Some(Refusal::LatticeTooLarge { faces }),
        });
    }

    // Assign every considered run to its face, by ORIGIN. Never by ink box: a run's box is
    // measured or absent, and intersecting an absent one would mean inventing it.
    let mut occupants: std::collections::BTreeMap<(usize, usize), Vec<usize>> =
        std::collections::BTreeMap::new();
    for i in considered {
        let run = &runs[*i];
        let r = line_of(&rows, run.y);
        let c = line_of(&columns, run.x);
        occupants.entry((r, c)).or_default().push(*i);
    }

    // Step 3. The coherence precondition.
    if occupants.len() != faces {
        return Ok(Outcome {
            table: None,
            refusal: Some(Refusal::FacesWithoutText {
                empty: faces - occupants.len(),
                faces,
            }),
        });
    }

    // Step 4. The emission-order precondition. Last, because it is the only one that needs the
    // lattice to exist before it can name a face.
    if let Some(r) = order_fault(runs, considered, &rows, &columns) {
        return Ok(Outcome {
            table: None,
            refusal: Some(r),
        });
    }

    let lattice = Lattice::new(&rows, &columns, runs, considered);
    let table_rect = lattice.bounds();
    let id = alloc.next(IdKind::Table)?;

    let mut cells = Vec::with_capacity(faces);
    for r in 0..rows.len() {
        for c in 0..columns.len() {
            // Coherence passed, so every face has occupants; an absent entry here would be a bug
            // in this function rather than a property of the document.
            let run_indices = occupants
                .get(&(r, c))
                .cloned()
                .expect("coherence guarantees every face is occupied");
            // Reading order, so a cell's text reads the way the page does. `considered` is
            // already in run order, and pushes preserved it — sorted anyway so the concatenation
            // cannot depend on iteration order.
            let mut run_indices = run_indices;
            run_indices.sort_unstable();
            let text: String = run_indices.iter().map(|i| runs[*i].text).collect();

            cells.push(DetectedCell {
                position: ethos_parser_core::TableCellPosition {
                    row: r as u32,
                    column: c as u32,
                    // Always 1. Alignment carries no evidence of a span, and a merge inferred
                    // without evidence is an invented one — see the module header.
                    rowspan: 1,
                    colspan: 1,
                    table_id: id.clone(),
                },
                rect: lattice.face(r, c),
                run_indices,
                text,
            });
        }
    }

    let check = cross_check(table_rect, rows.len() as u32, columns.len() as u32, &cells);

    // A well-formed box, or the detection is refused rather than emitted with geometry the
    // contract cannot express.
    table_rect.as_qrect_checked()?;

    Ok(Outcome {
        table: Some(DetectedTable {
            id,
            page,
            rect: table_rect,
            rows: rows.len() as u32,
            columns: columns.len() as u32,
            cells,
            check,
            tagged_check: None,
            rule: ethos_parser_core::TABLE_DETECTION_UNRULED_V1.to_string(),
        }),
        refusal: None,
    })
}

/// Whether the document wrote this arrangement across the rows, or down the columns.
///
/// Returns the first run that arrived out of row-major order, or `None` if the whole page reads
/// across. See [`detect`]'s step 4 for why this is a precondition and not a preference.
///
/// Runs are compared by **face ordinal**, so several runs inside one cell are in order with each
/// other whatever their x — a cell whose text was split into two `Tj`s must not fail this.
fn order_fault(
    runs: &[RunOrigin<'_>],
    considered: &[usize],
    rows: &[i64],
    columns: &[i64],
) -> Option<Refusal> {
    let mut high = 0usize;
    for i in considered {
        let run = &runs[*i];
        let ordinal = line_of(rows, run.y) * columns.len() + line_of(columns, run.x);
        if ordinal < high {
            return Some(Refusal::EmissionOrderNotRowMajor {
                ordinal,
                after: high,
                at: *i,
            });
        }
        high = ordinal;
    }
    None
}

/// Sorted distinct lines, folding anything within [`ALIGN_TOLERANCE`] of the last **line**.
///
/// Comparing against the last emitted line rather than the last value is what stops a chain: a
/// ladder of origins each a point from the next cannot walk a single line across the page. This
/// is `crate::tables::cluster` applied to origins instead of edges, and the two are deliberately
/// the same shape.
///
/// A line is a coordinate the document actually contains — the first origin of its group — where
/// a mean or a midpoint would be a number nobody wrote.
fn fold(values: impl Iterator<Item = i64>) -> Vec<i64> {
    let mut all: Vec<i64> = values.collect();
    all.sort_unstable();
    let mut out: Vec<i64> = Vec::new();
    for v in all {
        match out.last() {
            Some(&last) if v - last <= ALIGN_TOLERANCE => {}
            _ => out.push(v),
        }
    }
    out
}

/// The first pair of adjacent lines closer together than `floor`.
fn gutter_fault(lines: &[i64], floor: i64, columns: bool) -> Option<Refusal> {
    lines
        .windows(2)
        .map(|w| w[1] - w[0])
        .find(|gap| *gap < floor)
        .map(|gap| Refusal::GutterBelowFloor {
            columns,
            gap,
            floor,
        })
}

/// Which group a value belongs to: the last line at or below it.
///
/// Total by construction — `lines[0]` is the smallest value in the set every lookup comes from,
/// so nothing can fall before it.
fn line_of(lines: &[i64], v: i64) -> usize {
    // Binary search with the linear rposition's exact semantics on sorted input:
    // the LAST line ≤ v, or 0 when every line is greater.
    lines.partition_point(|l| *l <= v).saturating_sub(1)
}

/// The face geometry of an inferred grid.
///
/// # The boxes have to tile, so they are faces rather than text bounds
///
/// The cross-check's geometric half requires the cells to partition the table's box exactly. A
/// per-cell box hugging its own runs would leave the gutters uncovered, every unruled table would
/// report `DoesNotTile`, and a mismatch that fires on every well-formed table says nothing about
/// any of them. So a cell's box is its **lattice face**, and the faces tile by construction.
///
/// Boundaries between adjacent lines sit at the midpoint of the gap — a coordinate derived from
/// two origins the document contains. The outer boundaries sit [`ORIGIN_BOX_PADDING`] beyond the
/// extreme origins. Nothing here is derived from a font size.
struct Lattice {
    /// Row boundaries, `rows + 1` of them.
    ys: Vec<i64>,
    /// Column boundaries, `columns + 1` of them.
    xs: Vec<i64>,
}

impl Lattice {
    fn new(rows: &[i64], columns: &[i64], runs: &[RunOrigin<'_>], considered: &[usize]) -> Self {
        // The far edges come from the largest origins, not from the last line: the last line is
        // where its group STARTS, and the group extends to whatever origin sits furthest into it.
        let max_x = considered.iter().map(|i| runs[*i].x).max().unwrap_or(0);
        let max_y = considered.iter().map(|i| runs[*i].y).max().unwrap_or(0);
        Self {
            ys: boundaries(rows, max_y),
            xs: boundaries(columns, max_x),
        }
    }

    fn bounds(&self) -> QuantRect {
        QuantRect {
            x0: self.xs[0],
            y0: self.ys[0],
            x1: self.xs[self.xs.len() - 1],
            y1: self.ys[self.ys.len() - 1],
        }
    }

    fn face(&self, row: usize, column: usize) -> QuantRect {
        QuantRect {
            x0: self.xs[column],
            y0: self.ys[row],
            x1: self.xs[column + 1],
            y1: self.ys[row + 1],
        }
    }
}

/// `lines.len() + 1` boundaries: padded outside, midpoint between.
fn boundaries(lines: &[i64], max_origin: i64) -> Vec<i64> {
    let mut out = Vec::with_capacity(lines.len() + 1);
    out.push(lines[0] - ORIGIN_BOX_PADDING);
    for (a, b) in lines.iter().zip(lines.iter().skip(1)) {
        // Floor division on a positive gap; both operands are coordinates from the document.
        out.push(a + (b - a) / 2);
    }
    out.push(max_origin + ORIGIN_BOX_PADDING);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_parser_core::{CheckStatus, Profile, QUANTUM_PER_POINT};

    fn alloc() -> IdAllocator {
        IdAllocator::new(Profile::default().profile_sha256().expect("hashes"))
    }

    fn pt(v: i64) -> i64 {
        v * i64::from(QUANTUM_PER_POINT)
    }

    fn run<'a>(x: i64, y: i64, text: &'a str) -> RunOrigin<'a> {
        RunOrigin {
            x: pt(x),
            y: pt(y),
            text,
        }
    }

    fn all(n: usize) -> Vec<usize> {
        (0..n).collect()
    }

    /// The `synthetic/table-regular-grid` geometry, in points: 3 rows x 2 columns.
    fn regular_grid() -> Vec<RunOrigin<'static>> {
        vec![
            run(72, 72, "Name"),
            run(180, 72, "Score"),
            run(72, 96, "Alpha"),
            run(180, 96, "10"),
            run(72, 120, "Beta"),
            run(180, 120, "12"),
        ]
    }

    #[test]
    fn the_regular_grid_is_a_three_by_two_table() {
        // The slice's golden, as geometry. `synthetic/table-regular-grid` has SIX `Tm`/`Tj` pairs
        // and zero path operators — it is a table drawn entirely by where the text sits, which is
        // why v1-S1 correctly found nothing in it.
        let runs = regular_grid();
        let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
        let t = out.table.expect("the grid must be found");
        assert_eq!((t.rows, t.columns), (3, 2));
        assert_eq!(t.cells.len(), 6);
        assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_UNRULED_V1);
        assert_eq!(t.check.outcome, CheckStatus::Ok, "{:?}", t.check);
        assert!(out.refusal.is_none());
    }

    #[test]
    fn every_cell_carries_the_text_the_document_put_there() {
        let runs = regular_grid();
        let t = detect(1, &runs, &all(runs.len()), &mut alloc())
            .unwrap()
            .table
            .unwrap();
        let at = |r: u32, c: u32| {
            t.cells
                .iter()
                .find(|x| x.position.row == r && x.position.column == c)
                .unwrap()
                .text
                .as_str()
        };
        assert_eq!(at(0, 0), "Name");
        assert_eq!(at(0, 1), "Score");
        assert_eq!(at(1, 0), "Alpha");
        assert_eq!(at(1, 1), "10");
        assert_eq!(at(2, 0), "Beta");
        assert_eq!(at(2, 1), "12");
    }

    #[test]
    fn no_unruled_cell_is_ever_merged() {
        // Alignment carries no evidence of a span. Every cell is 1x1 or the lattice is refused.
        let runs = regular_grid();
        let t = detect(1, &runs, &all(runs.len()), &mut alloc())
            .unwrap()
            .table
            .unwrap();
        assert!(t.cells.iter().all(|c| !c.position.is_merged()));
        assert!(t.cells.iter().all(|c| c.position.is_well_formed()));
    }

    #[test]
    fn cell_text_is_exactly_the_runs_assigned_to_it() {
        // Fabrication 0 as a property: a cell's text is its runs, in order, and nothing else.
        let runs = regular_grid();
        let t = detect(1, &runs, &all(runs.len()), &mut alloc())
            .unwrap()
            .table
            .unwrap();
        for c in &t.cells {
            let expected: String = c.run_indices.iter().map(|i| runs[*i].text).collect();
            assert_eq!(c.text, expected);
            assert!(
                !c.run_indices.is_empty(),
                "coherence means no unruled cell is ever empty"
            );
        }
    }

    #[test]
    fn two_runs_at_one_origin_are_one_cell() {
        // Runs the document showed at very nearly the same point fold into one line on both axes
        // and therefore into one cell, concatenated in reading order. This is the only way an
        // unruled cell holds more than one run — see the module header on why a `Tj`-split cell
        // a few points wide opens a column instead.
        let mut runs = regular_grid();
        runs.insert(
            1,
            RunOrigin {
                x: pt(72) + ALIGN_TOLERANCE,
                y: pt(72),
                text: "-plate",
            },
        );
        let t = detect(1, &runs, &all(runs.len()), &mut alloc())
            .unwrap()
            .table
            .unwrap();
        assert_eq!((t.rows, t.columns), (3, 2));
        let first = t
            .cells
            .iter()
            .find(|c| c.position.row == 0 && c.position.column == 0)
            .unwrap();
        assert_eq!(first.text, "Name-plate");
        assert_eq!(first.run_indices, vec![0, 1]);
    }

    #[test]
    fn a_ladder_of_origins_does_not_chain_into_one_column() {
        // The defect that made this rule fold rather than grow. Under gap-growing, origins each
        // inside the next one's gutter collapse into a single "column" nothing aligns to, and two
        // lines of word-split prose came out as a 2x3 table. Folding cannot walk past the
        // tolerance, so each of these stays its own line and the lattice is refused.
        let runs = vec![
            run(72, 72, "the"),
            run(90, 72, "quick"),
            run(114, 72, "fox"),
            run(72, 84, "jumps"),
            run(96, 84, "over"),
            run(118, 84, "it"),
        ];
        let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
        assert!(
            out.table.is_none(),
            "six words of prose are not a 2x3 table: {:?}",
            out.table
        );
    }

    #[test]
    fn a_face_without_text_refuses_the_whole_lattice() {
        // The near-miss shape and the form shape, at their smallest: one slot has nothing in it.
        // The answer is no table, NOT a table with an invented empty cell.
        let runs = vec![
            run(72, 72, "a"),
            run(180, 72, "b"),
            run(72, 96, "c"),
            // (1, 1) empty
        ];
        let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
        assert!(out.table.is_none(), "got {:?}", out.table);
        assert_eq!(
            out.refusal,
            Some(Refusal::FacesWithoutText { empty: 1, faces: 4 })
        );
    }

    #[test]
    fn a_single_row_is_a_line_and_not_a_table() {
        let runs = vec![run(72, 72, "a"), run(180, 72, "b"), run(280, 72, "c")];
        let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
        assert!(out.table.is_none());
        assert!(
            out.refusal.is_none(),
            "a line of text never was a candidate, so nothing was refused: {:?}",
            out.refusal
        );
    }

    #[test]
    fn a_single_column_is_a_list_and_not_a_table() {
        // Leader dots and bullet lists are the common shape here.
        let runs = vec![run(72, 72, "a"), run(72, 96, "b"), run(72, 120, "c")];
        let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
        assert!(out.table.is_none());
        assert!(out.refusal.is_none(), "{:?}", out.refusal);
    }

    #[test]
    fn an_ordinary_page_declares_no_refusal() {
        // The near-miss disclosure is only worth carrying if it is rare. A refusal reported for
        // every page of prose in existence would say nothing at all — the failure mode of the
        // blanket limitation this slice retired.
        for runs in [
            vec![run(72, 72, "Hello Ethos")],
            vec![run(72, 72, "one"), run(72, 96, "two")],
            vec![run(72, 72, "a"), run(180, 72, "b")],
        ] {
            let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
            assert!(out.table.is_none());
            assert!(
                out.refusal.is_none(),
                "{} run(s) -> {:?}",
                runs.len(),
                out.refusal
            );
        }
    }

    #[test]
    fn columns_under_the_gutter_floor_are_refused_rather_than_merged() {
        // Two "columns" a few points apart are word spacing or a near miss. Merging them would
        // mean choosing between two alignments the document does not choose between.
        let runs = vec![
            run(72, 72, "a"),
            run(78, 72, "b"),
            run(72, 96, "c"),
            run(78, 96, "d"),
        ];
        let out = detect(1, &runs, &all(runs.len()), &mut alloc()).unwrap();
        assert!(out.table.is_none(), "got {:?}", out.table);
        assert_eq!(
            out.refusal,
            Some(Refusal::GutterBelowFloor {
                columns: true,
                gap: 600,
                floor: COLUMN_GUTTER_MIN,
            })
        );
    }

    #[test]
    fn rows_under_the_gutter_floor_are_refused_too() {
        // Baselines that missed each other by more than the fold tolerance are a disagreement
        // about where the line is, not two rows.
        let runs = vec![
            RunOrigin {
                x: pt(72),
                y: pt(72),
                text: "a",
            },
            RunOrigin {
                x: pt(180),
                y: pt(72),
                text: "b",
            },
            RunOrigin {
                x: pt(72),
                y: pt(72) + ROW_GUTTER_MIN - 100,
                text: "c",
            },
            RunOrigin {
                x: pt(180),
                y: pt(72) + ROW_GUTTER_MIN - 100,
                text: "d",
            },
        ];
        let out = detect(1, &runs, &all(4), &mut alloc()).unwrap();
        assert!(out.table.is_none(), "got {:?}", out.table);
        assert!(matches!(
            out.refusal,
            Some(Refusal::GutterBelowFloor { columns: false, .. })
        ));
    }

    #[test]
    fn the_gutter_floor_is_exact_to_the_quantum() {
        // The threshold, pinned on both sides. It is a declared cliff: named in
        // `COLUMN_GUTTER_MIN`, part of the rule id, and moving it moves the profile hash. A
        // threshold nobody pinned is a threshold that drifts.
        let at = |dx: i64| {
            vec![
                RunOrigin {
                    x: pt(72),
                    y: pt(72),
                    text: "a",
                },
                RunOrigin {
                    x: pt(72) + dx,
                    y: pt(72),
                    text: "b",
                },
                RunOrigin {
                    x: pt(72),
                    y: pt(96),
                    text: "c",
                },
                RunOrigin {
                    x: pt(72) + dx,
                    y: pt(96),
                    text: "d",
                },
            ]
        };

        let below = at(COLUMN_GUTTER_MIN - 1);
        assert!(
            detect(1, &below, &all(4), &mut alloc())
                .unwrap()
                .table
                .is_none(),
            "one quantum under the floor is not a column"
        );

        let over = at(COLUMN_GUTTER_MIN);
        let t = detect(1, &over, &all(4), &mut alloc())
            .unwrap()
            .table
            .expect("and exactly at the floor, it is");
        assert_eq!((t.rows, t.columns), (2, 2));
    }

    #[test]
    fn the_cells_tile_their_table_exactly() {
        // The property the cross-check's geometric half tests, asserted directly so a failure
        // points at the geometry rather than at the check.
        let runs = regular_grid();
        let t = detect(1, &runs, &all(runs.len()), &mut alloc())
            .unwrap()
            .table
            .unwrap();
        let sum: i64 = t
            .cells
            .iter()
            .map(|c| (c.rect.x1 - c.rect.x0) * (c.rect.y1 - c.rect.y0))
            .sum();
        let area = (t.rect.x1 - t.rect.x0) * (t.rect.y1 - t.rect.y0);
        assert_eq!(sum, area, "inferred faces must partition the table's box");
    }

    #[test]
    fn no_box_is_derived_from_a_font_size() {
        // The standing rule (`docs/09-V1-MILESTONES.md`, rule 2). The padding is a declared
        // constant, so identical origins must give identical boxes whatever the type size — and
        // this module never receives a font size at all, which is the real guarantee.
        //
        // Split on the test module explicitly. Scanning "everything before the first
        // `#[cfg(test)]`" is the spelling that made this guard's v1-S1 sibling vacuous — that
        // attribute also appears on a `use` at the top of a file — and scanning the whole file
        // would trip over this very assertion's own text.
        let src = include_str!("unruled.rs");
        let code = src
            .split("\n#[cfg(test)]\nmod tests {")
            .next()
            .expect("split always yields a first part");
        assert!(
            code.contains("pub fn detect"),
            "the guard did not reach the detector and would pass regardless"
        );
        let code: String = code
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for banned in ["font_size", "FontSize"] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the unruled geometry; boxes come from origins and a declared \
                 padding, never from type size"
            );
        }
    }

    #[test]
    fn runs_a_ruled_table_already_claims_are_not_considered() {
        // The arbitration contract, from this side: `considered` is the filter, and a run left
        // out of it cannot vote on the lattice or land in a cell.
        let runs = regular_grid();
        let out = detect(1, &runs, &[0, 1], &mut alloc()).unwrap();
        assert!(
            out.table.is_none(),
            "two runs on one line cannot be a table: {:?}",
            out.table
        );
    }

    #[test]
    fn no_runs_at_all_is_not_even_a_refusal() {
        // Nothing to refuse. A page with no leftover text never built a candidate, and saying it
        // refused one would be reporting an event that did not happen.
        let out = detect(1, &[], &[], &mut alloc()).unwrap();
        assert!(out.table.is_none());
        assert!(out.refusal.is_none());
    }

    #[test]
    fn every_refusal_explains_itself_without_scoring_anything() {
        // Typed vocabulary, never a confidence value (`docs/01-CONTRACT.md` §9).
        for r in [
            Refusal::LatticeTooLarge { faces: 99_999 },
            Refusal::FacesWithoutText {
                empty: 12,
                faces: 20,
            },
            Refusal::GutterBelowFloor {
                columns: true,
                gap: 500,
                floor: COLUMN_GUTTER_MIN,
            },
            Refusal::EmissionOrderNotRowMajor {
                ordinal: 0,
                after: 3,
                at: 2,
            },
        ] {
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
