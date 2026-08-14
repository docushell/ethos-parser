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

//! Table structure, and the occupancy model that makes it checkable (v1-S1).
//!
//! # `CellSlot`, and the defect it exists to avoid
//!
//! A table is not an array of rows of cells. A merged cell occupies **every** `(row, column)`
//! slot it covers, and addressing cells by array index with an implied span of 1 loses that —
//! the shipped Ethos ODL adapter does exactly this (memo §16), and the consequence is not a
//! cosmetic one: with spans discarded, *no* cross-check can run, because there is nothing left to
//! check the geometry against.
//!
//! So the model here is the one `docs/01-CONTRACT.md` §5.4 settles:
//!
//! | Rule | Consequence |
//! | --- | --- |
//! | Indices are **zero-based** | `(0, 0)` is the top-left slot |
//! | `rowspan`/`colspan` of **1 means not merged** | Never 0, never absent-meaning-1 |
//! | Every cell names its **parent table** | A cell is addressable without its container |
//! | A merged cell **owns every slot it covers** | [`CellSlot`] enumerates them |
//!
//! # Why occupancy is a type and not a convention
//!
//! [`SlotCover`] is the structural half of the E6 locator cross-check. It is derived from indices
//! and spans **alone** — it never sees a bounding box — so when it is compared against a
//! geometric derivation of the same table the two are genuinely independent. A check whose two
//! halves shared an input would agree with itself.

use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::ids::IdKind;
use crate::ids::NodeId;

/// One `(row, column)` slot in a table's grid.
///
/// A merged cell covers several. Ordering is row-major so a sorted list reads the way the table
/// does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellSlot {
    /// Zero-based row.
    pub row: u32,
    /// Zero-based column.
    pub column: u32,
}

impl CellSlot {
    /// A slot.
    pub fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}

/// Where a cell sits in its table, and how far it reaches.
///
/// The companion document settles this shape exactly — `TableCellPosition(row, column, rowspan,
/// colspan, parent table node ID)` — and it is used verbatim rather than re-derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableCellPosition {
    /// Zero-based row of the cell's top-left slot.
    pub row: u32,
    /// Zero-based column of the cell's top-left slot.
    pub column: u32,
    /// Rows covered. **1 means not merged**, never 0.
    pub rowspan: u32,
    /// Columns covered. **1 means not merged**, never 0.
    pub colspan: u32,
    /// The table this cell belongs to. Required: a cell without its parent is unaddressable.
    pub table_id: NodeId,
}

impl TableCellPosition {
    /// Every slot this cell occupies, row-major.
    ///
    /// The whole point of the type. A `2×2` merged cell at `(1, 1)` occupies four slots, and any
    /// code that treats it as occupying one is the defect this module names.
    pub fn slots(&self) -> Vec<CellSlot> {
        let mut out = Vec::with_capacity((self.rowspan * self.colspan) as usize);
        for r in self.row..self.row.saturating_add(self.rowspan) {
            for c in self.column..self.column.saturating_add(self.colspan) {
                out.push(CellSlot::new(r, c));
            }
        }
        out
    }

    /// Whether this cell is merged in either direction.
    pub fn is_merged(&self) -> bool {
        self.rowspan > 1 || self.colspan > 1
    }

    /// Whether the spans are structurally sane.
    ///
    /// A zero span is not "unmerged" — it is a cell occupying nothing, which cannot be true of a
    /// cell that exists.
    pub fn is_well_formed(&self) -> bool {
        self.rowspan >= 1 && self.colspan >= 1
    }
}

/// The structural derivation of a table's occupancy: which cell owns which slot.
///
/// Built from [`TableCellPosition`]s alone. **No geometry reaches this type**, which is what
/// makes it an independent half of the cross-check rather than a restatement of the detector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotCover {
    rows: u32,
    columns: u32,
    /// Slot → how many cells claim it. Anything but exactly 1 is a defect.
    owners: std::collections::BTreeMap<CellSlot, u32>,
}

impl SlotCover {
    /// Derive the cover of a declared grid from its cells.
    pub fn derive(rows: u32, columns: u32, cells: &[TableCellPosition]) -> Self {
        let mut owners: std::collections::BTreeMap<CellSlot, u32> =
            std::collections::BTreeMap::new();
        for cell in cells {
            for slot in cell.slots() {
                *owners.entry(slot).or_insert(0) += 1;
            }
        }
        Self {
            rows,
            columns,
            owners,
        }
    }

    /// What is wrong with this cover, if anything.
    ///
    /// Returns every fault rather than the first: a table with two overlaps and a hole should say
    /// so once, not three times over three runs.
    pub fn faults(&self) -> Vec<SlotFault> {
        let mut out = Vec::new();

        for row in 0..self.rows {
            for column in 0..self.columns {
                let slot = CellSlot::new(row, column);
                match self.owners.get(&slot).copied().unwrap_or(0) {
                    1 => {}
                    0 => out.push(SlotFault::Unowned(slot)),
                    n => out.push(SlotFault::OwnedMoreThanOnce { slot, owners: n }),
                }
            }
        }

        // A cell reaching past the grid it declares. Caught separately because it is a different
        // mistake from an overlap: the grid dimensions and the spans disagree.
        for slot in self.owners.keys() {
            if slot.row >= self.rows || slot.column >= self.columns {
                out.push(SlotFault::OutsideGrid(*slot));
            }
        }

        out.sort();
        out.dedup();
        out
    }

    /// Whether every declared slot is owned exactly once and nothing reaches outside the grid.
    pub fn is_exact(&self) -> bool {
        self.faults().is_empty()
    }
}

/// One way a slot cover can be wrong.
///
/// Typed rather than a message, because these are compared and counted. **No severity, no score**
/// — a cover is exact or it is not (`docs/01-CONTRACT.md` §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "fault")]
pub enum SlotFault {
    /// A slot inside the declared grid that no cell claims — a hole.
    Unowned(CellSlot),
    /// A slot claimed by more than one cell — an overlap.
    OwnedMoreThanOnce {
        /// The contested slot.
        slot: CellSlot,
        /// How many cells claimed it.
        owners: u32,
    },
    /// A cell reaching past the grid it declares.
    OutsideGrid(CellSlot),
}

/// The outcome of comparing two independent derivations of the same table.
///
/// **A typed vocabulary, never a score.** `docs/01-CONTRACT.md` §9 forbids a confidence field, and
/// "how trustworthy is this table" is exactly the question a float here would appear to answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum CheckStatus {
    /// Both derivations agree.
    Ok,
    /// They disagree. The faults are named; nothing was repaired to make them agree.
    Mismatch {
        /// What the structural derivation found wrong.
        structural: Vec<SlotFault>,
        /// What the geometric derivation found wrong.
        geometric: Vec<GeometricFault>,
    },
    /// The check could not run — there was nothing to compare.
    NotApplicable {
        /// Why.
        reason: String,
    },
}

/// A way the geometric derivation can disagree with itself.
///
/// Derived from **boxes alone**, never from indices, which is what makes this half independent of
/// [`SlotFault`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "fault")]
pub enum GeometricFault {
    /// A cell box that is not inside its parent table's box.
    CellOutsideTable {
        /// Which slot's cell.
        slot: CellSlot,
    },
    /// Two cell boxes whose interiors overlap.
    CellsOverlap {
        /// One slot.
        a: CellSlot,
        /// The other.
        b: CellSlot,
    },
    /// The cells' areas do not sum to the table's area — the grid does not tile.
    DoesNotTile {
        /// Sum of the cell areas, in square centipoints.
        cells: i64,
        /// The table's area.
        table: i64,
    },
}

/// The cross-check record that rides on the artifact.
///
/// **On the artifact, not on `--diagnostics`.** A locator mismatch changes whether a cell is
/// trustworthy, so it is part of what the artifact says rather than an observation about the run
/// that produced it (`docs/01-CONTRACT.md` §4 draws that line).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocatorCheck {
    /// Which check this is.
    pub check_id: String,
    /// Its version, so a result recorded under an older rule is not mistaken for this one.
    pub check_version: String,
    /// What it found.
    pub outcome: CheckStatus,
}

/// One cell, as it appears on the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableCellRecord {
    /// Stable id.
    pub id: NodeId,
    /// Where it sits and how far it reaches.
    pub position: TableCellPosition,
    /// Its box, in the artifact's declared coordinate system.
    pub bbox: crate::geom::QRect,
    /// The concatenation of the extracted runs whose origins fall inside it.
    ///
    /// **Never a novel string.** A cell enclosing no run carries an empty one, because that is
    /// what the document put there — `docs/09-V1-MILESTONES.md` S1's fabrication-0 criterion.
    pub text: String,
}

/// One table, as it appears on the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableRecord {
    /// Stable id.
    pub id: NodeId,
    /// The page this table is on.
    pub page: NodeId,
    /// Its bounding box.
    pub bbox: crate::geom::QRect,
    /// Rows in the lattice.
    pub rows: u32,
    /// Columns in the lattice.
    pub columns: u32,
    /// Cells, row-major.
    pub cells: Vec<TableCellRecord>,
    /// The derivation class of the table structure.
    ///
    /// Always `Computed`: the ruling lines and the text are Extracted, and the grid, the indices,
    /// the spans and the concatenation are an inference over them (`docs/01-CONTRACT.md` §6).
    pub derivation: crate::derivation::DerivationClass,
    /// The locator cross-check for this table.
    ///
    /// **On the artifact, not in `--diagnostics`.** A mismatch changes whether a cell is
    /// trustworthy, which makes it part of what the artifact says rather than an observation
    /// about the run that produced it.
    pub locator_check: LocatorCheck,
}

/// The identity of the locator cross-check, carried on every check record.
pub const LOCATOR_CHECK_V1: &str = "geometric-vs-structural-v1";

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(row: u32, column: u32, rowspan: u32, colspan: u32) -> TableCellPosition {
        TableCellPosition {
            row,
            column,
            rowspan,
            colspan,
            table_id: NodeId::from_parts(IdKind::Table, 1),
        }
    }

    #[test]
    fn an_unmerged_cell_occupies_exactly_one_slot() {
        let c = cell(2, 3, 1, 1);
        assert_eq!(c.slots(), vec![CellSlot::new(2, 3)]);
        assert!(!c.is_merged(), "span 1 means NOT merged");
    }

    #[test]
    fn a_merged_cell_occupies_every_slot_it_covers() {
        // The defect this whole module exists to prevent: addressing this cell as one slot loses
        // three, and then no cross-check can run at all.
        let c = cell(1, 1, 2, 2);
        assert!(c.is_merged());
        assert_eq!(
            c.slots(),
            vec![
                CellSlot::new(1, 1),
                CellSlot::new(1, 2),
                CellSlot::new(2, 1),
                CellSlot::new(2, 2),
            ]
        );
    }

    #[test]
    fn a_complete_grid_covers_exactly() {
        let cells = vec![cell(0, 0, 1, 1), cell(0, 1, 1, 1), cell(1, 0, 1, 2)];
        let cover = SlotCover::derive(2, 2, &cells);
        assert!(cover.is_exact(), "faults: {:?}", cover.faults());
    }

    #[test]
    fn a_hole_is_reported_rather_than_filled() {
        let cells = vec![cell(0, 0, 1, 1), cell(0, 1, 1, 1), cell(1, 0, 1, 1)];
        let cover = SlotCover::derive(2, 2, &cells);
        assert_eq!(
            cover.faults(),
            vec![SlotFault::Unowned(CellSlot::new(1, 1))]
        );
        assert!(!cover.is_exact());
    }

    #[test]
    fn an_overlap_is_reported_with_its_owner_count() {
        // Two cells claiming (0,1). Silently letting the last writer win is how a table quietly
        // loses a cell.
        let cells = vec![cell(0, 0, 1, 2), cell(0, 1, 1, 1), cell(1, 0, 1, 2)];
        let cover = SlotCover::derive(2, 2, &cells);
        assert_eq!(
            cover.faults(),
            vec![SlotFault::OwnedMoreThanOnce {
                slot: CellSlot::new(0, 1),
                owners: 2
            }]
        );
    }

    #[test]
    fn a_cell_reaching_past_the_grid_is_a_distinct_fault() {
        // Different from an overlap: here the spans and the declared dimensions disagree.
        let cells = vec![cell(0, 0, 1, 3)];
        let cover = SlotCover::derive(1, 2, &cells);
        assert!(cover
            .faults()
            .contains(&SlotFault::OutsideGrid(CellSlot::new(0, 2))));
    }

    #[test]
    fn a_zero_span_is_not_well_formed() {
        // Zero is not "unmerged". A cell occupying nothing cannot be a cell that exists.
        assert!(!cell(0, 0, 0, 1).is_well_formed());
        assert!(!cell(0, 0, 1, 0).is_well_formed());
        assert!(cell(0, 0, 1, 1).is_well_formed());
    }

    #[test]
    fn the_cover_sees_no_geometry() {
        // Guard the independence the cross-check depends on. If a bounding box ever reaches this
        // type, the two halves of E6 stop being two halves.
        let src = include_str!("tables.rs");
        let code: String = src
            .lines()
            .take_while(|l| !l.trim_start().starts_with("#[cfg(test)]"))
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for geometric in ["QRect", "bbox", "GeometryPresence", "x0", "y0"] {
            assert!(
                !code.contains(geometric),
                "`{geometric}` reached the structural half of the cross-check; the two halves \
                 must derive from different inputs or they agree with themselves"
            );
        }
    }
}
