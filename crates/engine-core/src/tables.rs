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
    /// Its box, in the artifact's declared coordinate system — or a typed reason there is none.
    ///
    /// `Measured` for a cell a detector bounded from ink; `Absent` for a cell of a **tagged**
    /// table, whose grid came from `/TR`/`/TD` and whose box the structure tree never stated
    /// ([`crate::GeometryAbsence::NotReportedByStructureTree`], v2-S24). A tagged cell is never
    /// given an invented box — that is the whole reason this is a [`crate::GeometryPresence`] and
    /// not a bare rectangle.
    pub geometry: crate::derivation::GeometryPresence,
    /// The concatenation of extracted runs, joined by [`Self::node_ids`].
    ///
    /// For a geometric cell those are the runs whose origins fall inside its box; for a **tagged**
    /// cell they are the runs the structure tree binds under it by `(page, mcid)` (v2-S24). Either
    /// way it is real runs concatenated, never re-decoded and never placed.
    ///
    /// **Never a novel string.** A cell enclosing no run carries an empty one, because that is
    /// what the document put there — `docs/09-V1-MILESTONES.md` S1's fabrication-0 criterion.
    pub text: String,
    /// The nodes [`Self::text`] is the concatenation of, in reading order (v1.1-S2).
    ///
    /// # Why the link is carried rather than recoverable
    ///
    /// The detector already knows it: `DetectedCell::run_indices` addresses the page's run list,
    /// and every one of those runs becomes a node under its own id. Until v1.1-S2 the conversion
    /// threw that away and a cell arrived carrying only a **string**.
    ///
    /// A consumer holding a string can get back to the nodes two ways, and both are wrong. It can
    /// re-run the geometry — "runs whose origins fall inside this box" — which is a second copy of
    /// the detector's rule that can drift from the first, and `docs/06-STEAL-REFUSE.md` A12 is
    /// about exactly that. Or it can match the text, which is a guess the moment two cells hold
    /// the same word.
    ///
    /// The Markdown projection makes this concrete rather than theoretical: a `source` segment
    /// must name the nodes its bytes came from ([`crate::markdown::AnchorMap`] law 3), so a GFM
    /// cell **cannot be emitted as source at all** without this field. Carrying a fact the
    /// detector computed is not detection.
    ///
    /// Empty exactly when [`Self::text`] is empty: a cell enclosing no run names no node.
    pub node_ids: Vec<NodeId>,
}

/// One table, as it appears on the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableRecord {
    /// Stable id.
    pub id: NodeId,
    /// The page this table is on.
    pub page: NodeId,
    /// Its bounding box, in the artifact's declared coordinate system — or a typed reason there is
    /// none.
    ///
    /// `Measured` for a table a detector bounded from ink; `Absent` for a **tagged** table, which
    /// the document declared in its structure tree and which carries no coordinate to bound it with
    /// ([`crate::GeometryAbsence::NotReportedByStructureTree`], v2-S24). No box is invented for the
    /// tagged case, which is why this is a [`crate::GeometryPresence`] rather than a bare rectangle.
    pub geometry: crate::derivation::GeometryPresence,
    /// Rows in the lattice.
    pub rows: u32,
    /// Columns in the lattice.
    pub columns: u32,
    /// Cells, row-major.
    pub cells: Vec<TableCellRecord>,
    /// The derivation class of the table structure.
    ///
    /// `Computed` for a geometric table: the ruling lines and the text are Extracted, and the grid,
    /// the indices, the spans and the concatenation are an inference over them
    /// (`docs/01-CONTRACT.md` §6). **`Extracted` for a tagged table** (v2-S24): its grid is not an
    /// inference at all but the document's own `/Table`/`/TR`/`/TD` structure read off the tree, so
    /// the stronger class is the honest one — and it is exactly this field, paired with
    /// [`Self::detection_rule`], that lets a consumer tell "the engine inferred this grid from ink"
    /// from "the document declared this grid and the engine read it". `Extracted` rides with
    /// `tagged-tables-v1`; `Computed` with the three geometric ids.
    pub derivation: crate::derivation::DerivationClass,
    /// **Which rule found this table** — `ruled-rects-v3`, `unruled-align-v1`, `stroke-ruled-v1`
    /// or `tagged-tables-v1` (v1-S2, S7b, S8, v2-S24).
    ///
    /// This named **two** from v1-S7b until v2-S13.5, then three, and a fourth arrived at v2-S24.
    /// `stroke-ruled-v1` shipped at v1-S8 and is written into this field by `engine-pdf`'s third
    /// build site; `tagged-tables-v1` is written by a fourth. A consumer matching on the listed
    /// values must carry all four.
    ///
    /// Per table, not per document, because one document can carry all four kinds and the
    /// difference matters to a consumer:
    ///
    /// | Value | What the document did | What the engine did | `derivation` |
    /// | --- | --- | --- | --- |
    /// | `ruled-rects-v3` | painted the grid | read it | `Computed` |
    /// | `unruled-align-v1` | placed text in columns | inferred it | `Computed` |
    /// | `stroke-ruled-v1` | stroked the ruling lines | read the lines and bounded the cells | `Computed` |
    /// | `tagged-tables-v1` | tagged the grid in its structure tree | read the tags | `Extracted` |
    ///
    /// `derivation` is `Computed` for the first three — all inferences over Extracted ink — and
    /// `Extracted` for the fourth, which is the document's own statement rather than an inference.
    /// The profile cannot carry this distinction: it says which rules *ran*, not which one produced
    /// any given table. Only a per-table field can answer "did the author draw this grid, tag it,
    /// or did we decide it was one".
    ///
    /// A plain string, matching the rule-id-as-data convention `Profile::table_detection` uses,
    /// and matching the ids pinned there.
    pub detection_rule: String,
    /// The locator cross-check for this table.
    ///
    /// **On the artifact, not in `--diagnostics`.** A mismatch changes whether a cell is
    /// trustworthy, which makes it part of what the artifact says rather than an observation
    /// about the run that produced it.
    pub locator_check: LocatorCheck,
    /// The tagged-versus-geometric check, when the document's structure tree describes a table on
    /// this page (v1-S3).
    ///
    /// **Absent means the tree said nothing about a table here** — either the document is
    /// untagged, or its tree describes no `/Table` on this page. That is not the same as the two
    /// derivations agreeing, so it is an absent key rather than an `Ok`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagged_check: Option<TaggedGridCheck>,
}

/// The identity of the locator cross-check, carried on every check record.
pub const LOCATOR_CHECK_V1: &str = "geometric-vs-structural-v1";

/// The identity of the tagged-versus-geometric check (v1-S3).
///
/// **A different check, not a wider one.** [`LOCATOR_CHECK_V1`] compares a table's *own* indices
/// against its *own* boxes; this compares the grid the **document's structure tree** declares
/// against the grid a detector found. Overloading one id with both would make a status
/// uninterpretable — a reader could not tell which pair of derivations disagreed.
pub const TAGGED_GRID_CHECK_V1: &str = "tagged-vs-geometric-v1";

/// One way the document's tags and a detector's grid can disagree.
///
/// **Typed, and never a score.** These are compared and counted; "how badly do they disagree" is
/// not a question this answers, because the answer would be the confidence field
/// `docs/01-CONTRACT.md` §9 forbids.
///
/// Nothing here is repaired. When the tree says three rows and the detector found two, the
/// artifact says so and keeps the detector's two — because picking the tree's answer would be
/// this engine deciding which of two disagreeing sources to believe, with nothing on the wire to
/// record that it did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "fault")]
pub enum TaggedGridFault {
    /// The tree and the detector count rows differently.
    RowCountDiffers {
        /// Rows the structure tree declares.
        tagged: u32,
        /// Rows the detector found.
        detected: u32,
    },
    /// The tree and the detector count columns differently.
    ColumnCountDiffers {
        /// Columns the structure tree declares.
        tagged: u32,
        /// Columns the detector found.
        detected: u32,
    },
    /// A slot the tree's cells cover and the detector's do not.
    SlotOnlyInTagged(CellSlot),
    /// A slot the detector's cells cover and the tree's do not.
    SlotOnlyInDetected(CellSlot),
}

/// The outcome of comparing the document's tags against a detected grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum TaggedGridStatus {
    /// The tree and the detector describe the same grid.
    Ok,
    /// They do not. The disagreements are named; nothing was changed to reconcile them.
    Mismatch {
        /// What differs.
        faults: Vec<TaggedGridFault>,
    },
    /// The check could not run.
    NotApplicable {
        /// Why.
        reason: String,
    },
}

/// The tagged-versus-geometric check record, when the document's tree describes this table.
///
/// # The halves share no input, which is the whole point
///
/// The tagged half reads `/S`, `/TR`, `/TD`, `/RowSpan` and `/ColSpan` and **never a box**. The
/// geometric half reads painted rectangles or text origins and **never a structure type**. So an
/// agreement is two independent readings of one table arriving at the same grid, which is worth
/// something; a check whose halves shared a source would agree with itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaggedGridCheck {
    /// Which check this is. [`TAGGED_GRID_CHECK_V1`].
    pub check_id: String,
    /// Its version.
    pub check_version: String,
    /// What it found.
    pub outcome: TaggedGridStatus,
}

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

    /// The source of one item, delimited by the declaration that follows it.
    ///
    /// Anchored on declarations rather than on line numbers so that reordering the file breaks
    /// the test loudly instead of quietly shrinking what it reads.
    fn source_between(from: &str, to: &str) -> String {
        let src = include_str!("tables.rs");
        let start = src.find(from).unwrap_or_else(|| {
            panic!("`{from}` is gone; this guard no longer reads what it names")
        });
        let rest = &src[start..];
        let end = rest
            .find(to)
            .unwrap_or_else(|| panic!("`{to}` is gone; this guard no longer reads what it names"));
        rest[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_cover_sees_no_geometry() {
        // Guard the independence the cross-check depends on. If a bounding box ever reaches the
        // structural half, the two halves of E6 stop being two halves and the check agrees with
        // itself.
        //
        // **This reads the structural types specifically, not "the file up to the tests".** The
        // v1-S1 spelling took lines `take_while(|l| !l.starts_with("#[cfg(test)]"))`, which stops
        // at the `#[cfg(test)] use crate::ids::IdKind;` import at the TOP of this file — so it
        // scanned exactly one line (`use serde::...`) and could not fail. `TableCellRecord` and
        // `TableRecord` legitimately carry `bbox`, which is why widening the old scan to the
        // whole file was never an option: the region is what matters, so the region is what is
        // named.
        let structural = format!(
            "{}\n{}",
            // CellSlot + TableCellPosition and their impls: the cover's only inputs.
            source_between("pub struct CellSlot", "/// The structural derivation"),
            // SlotCover and its impl: the derivation itself.
            source_between(
                "pub struct SlotCover",
                "/// One way a slot cover can be wrong"
            ),
        );

        // Sanity: the scan must actually reach the code, or every assertion below is vacuous.
        assert!(
            structural.contains("fn derive") && structural.contains("fn slots"),
            "the guard did not reach SlotCover::derive and TableCellPosition::slots; it is \
             reading the wrong region and would pass regardless of what leaked in"
        );

        for geometric in ["QRect", "bbox", "GeometryPresence", "x0", "y0"] {
            assert!(
                !structural.contains(geometric),
                "`{geometric}` reached the structural half of the cross-check; the two halves \
                 must derive from different inputs or they agree with themselves"
            );
        }
    }
}
