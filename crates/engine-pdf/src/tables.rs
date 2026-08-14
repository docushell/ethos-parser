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

//! Ruled-table detection, and the locator cross-check (v1-S1).
//!
//! # Ruled only, and why that is a slice boundary rather than a shortcut
//!
//! A ruling line is **evidence in the document**: the author drew it. An alignment cluster is an
//! **inference about the document**: the author drew nothing and a detector decided. Those deserve
//! different derivation classes and different tests, so `docs/09-V1-MILESTONES.md` puts them in
//! different slices. Nothing here looks at text alignment. A table with no rules is not found, and
//! the artifact says the detector looked.
//!
//! # The rule, in full
//!
//! Pinned as `engine_core::TABLE_DETECTION_V1` in the profile, so changing any part of it moves
//! `profile_sha256` and makes artifacts from before and after correctly non-comparable.
//!
//! 1. **Lattice from edges.** Every captured rectangle contributes its two x edges and two y
//!    edges. Edges within [`LATTICE_TOLERANCE`] of each other are one lattice line. This is what
//!    lets a grid drawn as thin *stroked line* rectangles and a grid drawn as filled *cell*
//!    rectangles produce the same lattice.
//! 2. **Faces are candidate cells.** `n` x-lines and `m` y-lines make `(n-1) × (m-1)` faces.
//!    **Fewer than two faces is not a grid** — a lone rectangle is an underline or a border, and
//!    calling it a 1×1 table would find one on most pages in existence.
//! 3. **Every face must be covered by a rectangle.** This is the coherence precondition, and it
//!    is what separates a grid from a page that merely contains rectangles. Without it, the
//!    scattered field boxes on a tax form produce one enormous lattice whose faces are mostly
//!    empty — measured, not hypothesised: `irs-form-1040-2025` yielded a 662-cell "table" with a
//!    cell spanning 75 rows by 45 columns. That is a fabricated table, and fabrication is the one
//!    thing this slice may not do.
//! 4. **Cells are the rectangles**, mapped onto the lattice. A rectangle spanning several faces
//!    is a merged cell; one covering *every* face is the table's outer border, not a cell.
//! 5. **Text is assigned by origin**, never by ink-box intersection. A run's native locator is
//!    exact; its ink box is measured or absent, and intersecting an absent box would be inventing
//!    one. A run whose origin falls in no cell stays where it is — it is still an Extracted node.
//!
//! # Fabrication is impossible here, not merely avoided
//!
//! A cell's text is the concatenation of runs **already extracted** from the page whose origins
//! fall inside it. There is no re-decode, no OCR, no nearest-neighbour. A cell enclosing no run is
//! emitted with empty text, because that is what the document put there.
//!
//! # Derivation
//!
//! The rectangles and the runs are `Extracted`. The lattice, the row/column indices, the spans and
//! the text concatenation are `Computed` — `docs/01-CONTRACT.md` §6, and nothing here overwrites
//! an Extracted value.

use engine_core::{
    quantize, CellSlot, CheckStatus, DerivationClass, EngineError, GeometricFault, IdAllocator,
    IdKind, LocatorCheck, NodeId, QRect, SlotCover, TableCellPosition, LOCATOR_CHECK_V1,
    QUANTUM_PER_POINT,
};

// The rule id lives in `engine_core::TABLE_DETECTION_V1` and is NOT restated here. Two spellings
// of one rule id is exactly the drift a versioned id exists to prevent, and a test asserting the
// two match would only catch it after somebody had already written the second one.

/// How close two edges must be, in integer centipoints, to be one lattice line.
///
/// 150 centipoints — one and a half points. Wide enough to fold the two edges of a 1pt stroked
/// ruling line into a single lattice line, which is the common way a grid is drawn; narrow enough
/// that two genuinely distinct columns are never merged, since no table places columns 1.5pt
/// apart. It is part of `TABLE_DETECTION_V1`, so changing it is a rule-version event and moves
/// the profile hash — the same discipline `crate::thresholds` is under.
pub const LATTICE_TOLERANCE: i64 = 150;

/// A rectangle in **page space**, quantized — the coordinate system the artifact declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QuantRect {
    /// Left edge.
    pub x0: i64,
    /// Top edge (top-left origin, so smaller y is higher on the page).
    pub y0: i64,
    /// Right edge.
    pub x1: i64,
    /// Bottom edge.
    pub y1: i64,
}

impl QuantRect {
    fn as_qrect(self) -> Result<QRect, EngineError> {
        QRect::new(self.x0, self.y0, self.x1, self.y1).map_err(|e| EngineError::Malformed {
            what: "table geometry".into(),
            detail: e.to_string(),
        })
    }

    fn area(self) -> i128 {
        i128::from(self.x1 - self.x0) * i128::from(self.y1 - self.y0)
    }

    /// Whether two rectangles overlap in their **interiors**. Shared edges do not count.
    fn overlaps(self, other: Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    fn contains(self, other: Self) -> bool {
        self.x0 <= other.x0 && self.y0 <= other.y0 && self.x1 >= other.x1 && self.y1 >= other.y1
    }

    /// Containment allowing [`LATTICE_TOLERANCE`] of slack on every side.
    ///
    /// Used for the coverage precondition, and tolerant for the same reason the lattice is: a
    /// lattice line is a **cluster representative**, not any one rectangle's exact edge. Asking
    /// for exact containment against it would fail on the very documents clustering exists to
    /// handle — a grid drawn with 1pt rules, where each line's edges differ by the rule's width.
    fn covers_within_tolerance(self, other: Self) -> bool {
        self.x0 - LATTICE_TOLERANCE <= other.x0
            && self.y0 - LATTICE_TOLERANCE <= other.y0
            && self.x1 + LATTICE_TOLERANCE >= other.x1
            && self.y1 + LATTICE_TOLERANCE >= other.y1
    }
}

/// A detected cell, before it becomes nodes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DetectedCell {
    /// Where it sits and how far it reaches.
    pub position: TableCellPosition,
    /// Its box in page space.
    pub rect: QuantRect,
    /// Indices into the page's run list, in reading order, whose origins fall inside.
    pub run_indices: Vec<usize>,
    /// The concatenation of those runs' text. **Never a novel string.**
    pub text: String,
}

/// A detected table.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DetectedTable {
    /// Stable id.
    pub id: NodeId,
    /// 1-based page.
    pub page: u32,
    /// The table's bounding box in page space.
    pub rect: QuantRect,
    /// Rows in the lattice.
    pub rows: u32,
    /// Columns in the lattice.
    pub columns: u32,
    /// Cells, row-major.
    pub cells: Vec<DetectedCell>,
    /// The cross-check result for this table.
    pub check: LocatorCheck,
}

// -------------------------------------------------------------------------------------------
// Detection
// -------------------------------------------------------------------------------------------

/// A text run's origin and text, as the detector needs it.
pub struct RunOrigin<'a> {
    /// Origin x in page space, quantized.
    pub x: i64,
    /// Origin y in page space, quantized.
    pub y: i64,
    /// The run's text, already Extracted.
    pub text: &'a str,
}

/// Detect ruled tables on one page.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a rectangle will not quantize into a well-formed box.
pub fn detect(
    page: u32,
    rects: &[QuantRect],
    runs: &[RunOrigin<'_>],
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    let Some(lattice) = Lattice::build(rects) else {
        return Ok(Vec::new());
    };

    let table_rect = lattice.bounds();
    let id = alloc.next(IdKind::Table)?;

    // Step 3: rectangles spanning several faces are merged cells and claim them. A rectangle
    // covering every face is the outer border, not a cell.
    let face_count = (lattice.rows() * lattice.columns()) as usize;
    let mut claimed: std::collections::BTreeMap<CellSlot, usize> =
        std::collections::BTreeMap::new();
    let mut cells: Vec<(TableCellPosition, QuantRect)> = Vec::new();

    for r in rects {
        let Some(span) = lattice.span_of(*r) else {
            continue;
        };
        let covered = (span.rowspan * span.colspan) as usize;
        if covered == face_count && face_count > 1 {
            continue; // the table's own border
        }
        let position = TableCellPosition {
            row: span.row,
            column: span.column,
            rowspan: span.rowspan,
            colspan: span.colspan,
            table_id: id.clone(),
        };
        for slot in position.slots() {
            *claimed.entry(slot).or_insert(0) += 1;
        }
        cells.push((position, *r));
    }

    // No gap-filling. Cells are the rectangles the document drew, and nothing else — the
    // coherence precondition already guarantees every face is covered by one, so an "unclaimed"
    // face cannot exist here. Inventing a cell for one would be inventing a cell.
    let _ = &claimed;

    cells.sort_by_key(|(p, _)| (p.row, p.column));

    // Step 5: assign runs by ORIGIN. Never by ink box — an absent box would have to be invented.
    let detected: Vec<DetectedCell> = cells
        .into_iter()
        .map(|(position, rect)| {
            let mut run_indices = Vec::new();
            let mut text = String::new();
            for (i, run) in runs.iter().enumerate() {
                if run.x >= rect.x0 && run.x < rect.x1 && run.y >= rect.y0 && run.y < rect.y1 {
                    run_indices.push(i);
                    text.push_str(run.text);
                }
            }
            DetectedCell {
                position,
                rect,
                run_indices,
                text,
            }
        })
        .collect();

    let check = cross_check(table_rect, lattice.rows(), lattice.columns(), &detected);

    // A well-formed box for the table, or the whole detection is refused rather than emitted with
    // geometry the contract cannot express.
    table_rect.as_qrect()?;

    Ok(vec![DetectedTable {
        id,
        page,
        rect: table_rect,
        rows: lattice.rows(),
        columns: lattice.columns(),
        cells: detected,
        check,
    }])
}

// -------------------------------------------------------------------------------------------
// The cross-check (E6)
// -------------------------------------------------------------------------------------------

/// Compare a structural derivation against a geometric one.
///
/// **The two halves share no input.** The structural half sees `(row, column, rowspan, colspan)`
/// and nothing else; the geometric half sees boxes and nothing else. That is what makes this a
/// check rather than a restatement — a detector bug that put a merged cell's span and its box out
/// of step shows up here, and could not show up in a check whose halves came from one source.
///
/// Nothing is repaired. A disagreement is recorded and the Extracted runs are untouched.
pub fn cross_check(
    table: QuantRect,
    rows: u32,
    columns: u32,
    cells: &[DetectedCell],
) -> LocatorCheck {
    let outcome = if cells.is_empty() {
        CheckStatus::NotApplicable {
            reason: "the table has no cells to check".into(),
        }
    } else {
        // Structural: indices and spans only.
        let positions: Vec<TableCellPosition> = cells.iter().map(|c| c.position.clone()).collect();
        let structural = SlotCover::derive(rows, columns, &positions).faults();

        // Geometric: boxes only.
        let mut geometric = Vec::new();
        for c in cells {
            if !table.contains(c.rect) {
                geometric.push(GeometricFault::CellOutsideTable {
                    slot: CellSlot::new(c.position.row, c.position.column),
                });
            }
        }
        for (i, a) in cells.iter().enumerate() {
            for b in cells.iter().skip(i + 1) {
                if a.rect.overlaps(b.rect) {
                    geometric.push(GeometricFault::CellsOverlap {
                        a: CellSlot::new(a.position.row, a.position.column),
                        b: CellSlot::new(b.position.row, b.position.column),
                    });
                }
            }
        }
        let cell_area: i128 = cells.iter().map(|c| c.rect.area()).sum();
        if cell_area != table.area() {
            geometric.push(GeometricFault::DoesNotTile {
                cells: cell_area.try_into().unwrap_or(i64::MAX),
                table: table.area().try_into().unwrap_or(i64::MAX),
            });
        }
        geometric.sort();
        geometric.dedup();

        if structural.is_empty() && geometric.is_empty() {
            CheckStatus::Ok
        } else {
            CheckStatus::Mismatch {
                structural,
                geometric,
            }
        }
    };

    LocatorCheck {
        check_id: LOCATOR_CHECK_V1.to_string(),
        check_version: "1".to_string(),
        outcome,
    }
}

// -------------------------------------------------------------------------------------------
// The lattice
// -------------------------------------------------------------------------------------------

/// Where a rectangle sits on the lattice.
struct Span {
    row: u32,
    column: u32,
    rowspan: u32,
    colspan: u32,
}

/// Lattice lines derived from the captured rectangles' edges.
struct Lattice {
    xs: Vec<i64>,
    ys: Vec<i64>,
}

impl Lattice {
    /// The most faces a ruled grid may have under this rule.
    ///
    /// `n` rectangles give up to `2n` lines per axis and so up to `4n²` faces. On a page that
    /// merely contains rectangles that product explodes, and every one of those faces would be a
    /// cell. The cap is a refusal, not a truncation: past it, this is not a ruled table and no
    /// table is emitted.
    const MAX_FACES: usize = 4096;

    fn build(rects: &[QuantRect]) -> Option<Self> {
        if rects.is_empty() {
            return None;
        }
        let xs = cluster(rects.iter().flat_map(|r| [r.x0, r.x1]));
        let ys = cluster(rects.iter().flat_map(|r| [r.y0, r.y1]));
        // Two lines in each axis bound one face — but **one face is not a table**. A lone
        // rectangle is an underline, a highlight, a text-box border; calling it a 1x1 table would
        // find one on most pages in existence, and `docs/09-V1-MILESTONES.md` S1 names "a
        // fabricated 1x1 table around the page" as a thing this must not do. A grid needs at
        // least two faces.
        if xs.len() < 2 || ys.len() < 2 {
            return None;
        }
        let faces = (xs.len() - 1) * (ys.len() - 1);
        if !(2..=Self::MAX_FACES).contains(&faces) {
            return None;
        }

        let lattice = Self { xs, ys };

        // **The coherence precondition.** Every face has to be covered by a rectangle the
        // document actually painted. A genuine ruled grid drawn as cell rectangles satisfies this
        // exactly; a page with scattered boxes does not, and would otherwise become a table with
        // hundreds of invented cells.
        //
        // Overlaps are deliberately NOT excluded here — a rectangle claiming a face another
        // rectangle already owns is a real disagreement, and it belongs in the cross-check where
        // it is reported rather than in a precondition where it would be silently dropped.
        for row in 0..lattice.rows() {
            for column in 0..lattice.columns() {
                let face = lattice.face(row, column);
                if !rects.iter().any(|r| r.covers_within_tolerance(face)) {
                    return None;
                }
            }
        }

        Some(lattice)
    }

    fn rows(&self) -> u32 {
        (self.ys.len() - 1) as u32
    }

    fn columns(&self) -> u32 {
        (self.xs.len() - 1) as u32
    }

    fn bounds(&self) -> QuantRect {
        QuantRect {
            x0: self.xs[0],
            y0: self.ys[0],
            x1: self.xs[self.xs.len() - 1],
            y1: self.ys[self.ys.len() - 1],
        }
    }

    fn face(&self, row: u32, column: u32) -> QuantRect {
        QuantRect {
            x0: self.xs[column as usize],
            y0: self.ys[row as usize],
            x1: self.xs[column as usize + 1],
            y1: self.ys[row as usize + 1],
        }
    }

    /// Which faces a rectangle covers, if its edges land on lattice lines.
    fn span_of(&self, r: QuantRect) -> Option<Span> {
        let c0 = index_of(&self.xs, r.x0)?;
        let c1 = index_of(&self.xs, r.x1)?;
        let r0 = index_of(&self.ys, r.y0)?;
        let r1 = index_of(&self.ys, r.y1)?;
        if c1 <= c0 || r1 <= r0 {
            return None; // a zero-width or zero-height rectangle is a rule, not a cell
        }
        Some(Span {
            row: r0 as u32,
            column: c0 as u32,
            rowspan: (r1 - r0) as u32,
            colspan: (c1 - c0) as u32,
        })
    }
}

/// Sorted distinct values, folding anything within [`LATTICE_TOLERANCE`] together.
fn cluster(values: impl Iterator<Item = i64>) -> Vec<i64> {
    let mut all: Vec<i64> = values.collect();
    all.sort_unstable();
    let mut out: Vec<i64> = Vec::new();
    for v in all {
        match out.last() {
            Some(&last) if v - last <= LATTICE_TOLERANCE => {}
            _ => out.push(v),
        }
    }
    out
}

fn index_of(lines: &[i64], v: i64) -> Option<usize> {
    lines
        .iter()
        .position(|l| (l - v).abs() <= LATTICE_TOLERANCE)
}

/// Quantize a user-space rectangle into page space.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a coordinate will not quantize.
pub fn quantize_rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Result<QuantRect, EngineError> {
    let q = |v: f64| {
        quantize(v, QUANTUM_PER_POINT).map_err(|e| EngineError::Malformed {
            what: "table geometry".into(),
            detail: e.to_string(),
        })
    };
    let (a, b) = (q(x0)?, q(x1)?);
    let (c, d) = (q(y0)?, q(y1)?);
    Ok(QuantRect {
        x0: a.min(b),
        y0: c.min(d),
        x1: a.max(b),
        y1: c.max(d),
    })
}

/// The derivation class every computed table field carries.
///
/// Named here so the detector cannot quietly claim its output was Extracted.
pub const TABLE_DERIVATION: DerivationClass = DerivationClass::Computed;

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::{Profile, SlotFault};

    fn alloc() -> IdAllocator {
        IdAllocator::new(Profile::default().profile_sha256().expect("hashes"))
    }

    /// A rectangle given in **points**, converted to the centipoints the detector works in.
    ///
    /// The tests originally passed raw centipoints and read like points, so a "100-wide" column
    /// was one point across — inside `LATTICE_TOLERANCE`, and the whole grid folded to a single
    /// face. Taking points and scaling here keeps the test data at document scale.
    fn r(x0: i64, y0: i64, x1: i64, y1: i64) -> QuantRect {
        let q = i64::from(QUANTUM_PER_POINT);
        QuantRect {
            x0: x0 * q,
            y0: y0 * q,
            x1: x1 * q,
            y1: y1 * q,
        }
    }

    /// A point coordinate in centipoints, for tests that need to place a run inside a cell.
    fn pt(v: i64) -> i64 {
        v * i64::from(QUANTUM_PER_POINT)
    }

    /// A 2×2 grid drawn as four cell rectangles.
    fn grid_2x2() -> Vec<QuantRect> {
        vec![
            r(0, 0, 100, 100),
            r(100, 0, 200, 100),
            r(0, 100, 100, 200),
            r(100, 100, 200, 200),
        ]
    }

    #[test]
    fn no_rectangles_means_no_table_rather_than_an_empty_one() {
        let t = detect(1, &[], &[], &mut alloc()).unwrap();
        assert!(t.is_empty(), "a page with no rules has no ruled table");
    }

    #[test]
    fn a_single_rectangle_is_not_a_grid() {
        // One box is an underline, a highlight, a border — not a table. Calling it a 1×1 table
        // would find one on most pages in existence.
        let t = detect(1, &[r(0, 0, 100, 100)], &[], &mut alloc()).unwrap();
        assert!(t.is_empty(), "got {t:?}");
    }

    #[test]
    fn a_plain_grid_is_detected_and_cross_checks_ok() {
        let t = detect(1, &grid_2x2(), &[], &mut alloc()).unwrap();
        assert_eq!(t.len(), 1);
        let table = &t[0];
        assert_eq!((table.rows, table.columns), (2, 2));
        assert_eq!(table.cells.len(), 4);
        assert!(table.cells.iter().all(|c| !c.position.is_merged()));
        assert_eq!(table.check.outcome, CheckStatus::Ok, "{:?}", table.check);
    }

    #[test]
    fn a_merged_cell_claims_every_slot_it_covers() {
        // Top row is one rectangle spanning both columns.
        let rects = vec![
            r(0, 0, 200, 100),
            r(0, 100, 100, 200),
            r(100, 100, 200, 200),
        ];
        let t = detect(1, &rects, &[], &mut alloc()).unwrap();
        let table = &t[0];
        assert_eq!(table.cells.len(), 3, "the merge is one cell, not two");

        let merged = table.cells.iter().find(|c| c.position.is_merged()).unwrap();
        assert_eq!(merged.position.colspan, 2);
        assert_eq!(merged.position.rowspan, 1);
        assert_eq!(merged.position.slots().len(), 2);
        assert_eq!(table.check.outcome, CheckStatus::Ok, "{:?}", table.check);
    }

    #[test]
    fn an_uncovered_face_means_no_table_rather_than_an_invented_cell() {
        // Three rectangles on a 2x2 lattice: the fourth face has no rectangle at all. Filling it
        // in would be inventing a cell, so there is no table here.
        //
        // **This is the precondition that keeps a form from becoming a table.** Measured on
        // `irs-form-1040-2025`: without it, its scattered field boxes produced a 662-cell grid
        // with a cell spanning 75 rows by 45 columns, and Ethos rejected the artifact outright.
        let rects = vec![r(0, 0, 100, 100), r(100, 0, 200, 100), r(0, 100, 100, 200)];
        let t = detect(1, &rects, &[], &mut alloc()).unwrap();
        assert!(t.is_empty(), "got {t:?}");
    }

    #[test]
    fn scattered_rectangles_do_not_become_a_grid() {
        // The shape a form has: boxes that share no lattice and cover almost none of the faces
        // their edges imply.
        let rects = vec![
            r(0, 0, 20, 10),
            r(200, 100, 220, 110),
            r(400, 300, 420, 310),
            r(50, 500, 70, 510),
        ];
        let t = detect(1, &rects, &[], &mut alloc()).unwrap();
        assert!(
            t.is_empty(),
            "a page with boxes on it is not a table: {t:?}"
        );
    }

    #[test]
    fn an_outer_border_is_not_mistaken_for_a_cell() {
        let mut rects = grid_2x2();
        rects.push(r(0, 0, 200, 200)); // the table's own border
        let t = detect(1, &rects, &[], &mut alloc()).unwrap();
        let table = &t[0];
        assert_eq!(table.cells.len(), 4, "the border is not a fifth cell");
        assert_eq!(table.check.outcome, CheckStatus::Ok, "{:?}", table.check);
    }

    #[test]
    fn overlapping_rectangles_are_reported_and_never_repaired() {
        // The hostile case. Two rectangles claim the same face; the check must say so, and the
        // geometry must come out untouched.
        let rects = vec![
            r(0, 0, 200, 100),
            r(100, 0, 200, 100),
            r(0, 100, 100, 200),
            r(100, 100, 200, 200),
        ];
        let t = detect(1, &rects, &[], &mut alloc()).unwrap();
        let table = &t[0];

        match &table.check.outcome {
            CheckStatus::Mismatch {
                structural,
                geometric,
            } => {
                assert!(
                    structural
                        .iter()
                        .any(|f| matches!(f, SlotFault::OwnedMoreThanOnce { .. })),
                    "the structural half must see the double claim: {structural:?}"
                );
                assert!(
                    !geometric.is_empty(),
                    "and the geometric half must see it independently: {geometric:?}"
                );
            }
            other => panic!("expected a mismatch, got {other:?}"),
        }
    }

    #[test]
    fn text_is_assigned_by_origin_and_never_invented() {
        let runs = vec![
            RunOrigin {
                x: pt(10),
                y: pt(10),
                text: "TL",
            },
            RunOrigin {
                x: pt(110),
                y: pt(10),
                text: "TR",
            },
            // Nothing in the bottom row: those cells must come out EMPTY.
        ];
        let t = detect(1, &grid_2x2(), &runs, &mut alloc()).unwrap();
        let table = &t[0];

        let cell = |row, col| {
            table
                .cells
                .iter()
                .find(|c| c.position.row == row && c.position.column == col)
                .unwrap()
        };
        assert_eq!(cell(0, 0).text, "TL");
        assert_eq!(cell(0, 1).text, "TR");
        assert_eq!(
            cell(1, 0).text,
            "",
            "an empty cell is empty, never borrowed"
        );
        assert_eq!(cell(1, 1).text, "");
    }

    #[test]
    fn every_cell_text_is_a_concatenation_of_assigned_runs() {
        // Fabrication 0, as a property rather than an example: a cell's text is exactly the runs
        // it was assigned, in order, and nothing else.
        let runs = vec![
            RunOrigin {
                x: pt(10),
                y: pt(10),
                text: "a",
            },
            RunOrigin {
                x: pt(20),
                y: pt(20),
                text: "b",
            },
        ];
        let t = detect(1, &grid_2x2(), &runs, &mut alloc()).unwrap();
        for c in &t[0].cells {
            let expected: String = c.run_indices.iter().map(|i| runs[*i].text).collect();
            assert_eq!(
                c.text, expected,
                "cell text must be its runs and only its runs"
            );
        }
    }

    #[test]
    fn near_edges_fold_into_one_lattice_line() {
        // A grid drawn as 1pt stroked rules gives two edges per line, 100 centipoints apart.
        // They are one line, or every rule becomes a one-centipoint-wide column.
        let mut rects = grid_2x2();
        // Nudge one rectangle's left edge OUTWARD by a single centipoint — well inside the
        // tolerance a 1pt stroked rule needs, so it must be the SAME lattice line, not a new
        // column. Outward rather than inward so the face stays covered: an inward nudge would be
        // testing the coverage precondition instead of the clustering.
        rects[3].x0 -= 1;
        let t = detect(1, &rects, &[], &mut alloc()).unwrap();
        assert_eq!(
            (t[0].rows, t[0].columns),
            (2, 2),
            "a one-centipoint nudge must not open a column"
        );
    }

    #[test]
    fn a_table_is_computed_never_extracted() {
        // The derivation class is the contract's answer to "did the document say this?". A table
        // is an inference over Extracted geometry, and saying otherwise would be a lie about
        // provenance (docs/01-CONTRACT.md §6).
        assert_eq!(TABLE_DERIVATION, DerivationClass::Computed);
    }

    #[test]
    fn the_check_record_carries_its_own_version() {
        let t = detect(1, &grid_2x2(), &[], &mut alloc()).unwrap();
        assert_eq!(t[0].check.check_id, LOCATOR_CHECK_V1);
        assert!(!t[0].check.check_version.is_empty());
    }
}
