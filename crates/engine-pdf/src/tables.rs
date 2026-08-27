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
//! Pinned as `engine_core::TABLE_DETECTION_V3` in the profile, so changing any part of it moves
//! `profile_sha256` and makes artifacts from before and after correctly non-comparable.
//!
//! 1. **Lattice from edges.** Every captured rectangle contributes its two x edges and two y
//!    edges. Edges within [`LATTICE_TOLERANCE`] of each other are one lattice line. This is what
//!    lets a grid drawn as thin *stroked line* rectangles and a grid drawn as filled *cell*
//!    rectangles produce the same lattice.
//! 2. **Faces are candidate cells.** `n` x-lines and `m` y-lines make `(n-1) × (m-1)` faces.
//!    **Fewer than two faces is not a grid** — a lone rectangle is an underline or a border, and
//!    calling it a 1×1 table would find one on most pages in existence.
//! 3. **Every face must be covered by a rectangle that is not the border.** This is the coherence
//!    precondition, and it is what separates a grid from a page that merely contains rectangles.
//!    The exclusion in rule 4 applies here too, and v1-S7b is the slice that noticed it had to: a
//!    rectangle spanning the whole lattice is not emitted as a cell, so it cannot be the evidence
//!    that the cells exist either. Without it, the
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

// The rule id lives in `engine_core::TABLE_DETECTION_V3` and is NOT restated here. Two spellings
// of one rule id is exactly the drift a versioned id exists to prevent, and a test asserting the
// two match would only catch it after somebody had already written the second one.

/// How close two edges must be, in integer centipoints, to be one lattice line.
///
/// 150 centipoints — one and a half points. Wide enough to fold the two edges of a 1pt stroked
/// ruling line into a single lattice line, which is the common way a grid is drawn; narrow enough
/// that two genuinely distinct columns are never merged, since no table places columns 1.5pt
/// apart. It is part of `TABLE_DETECTION_V3`, so changing it is a rule-version event and moves
/// the profile hash — the same discipline `crate::thresholds` is under.
pub const LATTICE_TOLERANCE: i64 = 150;

/// The most faces any reconstructed grid may have, shared by every detection rule.
///
/// `n` rectangles give up to `2n` lines per axis and so up to `4n²` faces. On a page that merely
/// contains rectangles that product explodes, and every one of those faces would be a cell. The
/// cap is a refusal, not a truncation: past it, this is not a table and none is emitted.
///
/// One constant rather than three (v1-S8): the ruled, unruled and stroke-ruled rules answer
/// different questions about different evidence, but "how big may a grid get before it is not a
/// grid" is the same question for all of them, and three copies is three chances to drift.
pub const MAX_FACES: usize = 4096;

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
    /// A well-formed [`QRect`], or the detection is refused rather than emitted with geometry
    /// the contract cannot express.
    pub fn as_qrect_checked(self) -> Result<QRect, EngineError> {
        QRect::new(self.x0, self.y0, self.x1, self.y1).map_err(|e| EngineError::Malformed {
            what: "table geometry".into(),
            detail: e.to_string(),
        })
    }

    fn area(self) -> i128 {
        i128::from(self.x1 - self.x0) * i128::from(self.y1 - self.y0)
    }

    /// Whether two rectangles overlap in their **interiors**. Shared edges do not count.
    pub(crate) fn overlaps(self, other: Self) -> bool {
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
    /// Retained under `cfg(test)` as the executable SPEC of the coherence scan:
    /// production decides face coverage through the difference grid in
    /// `Lattice::build`, and the equivalence test in this file re-derives every
    /// verdict through this predicate to prove the two agree. Deleting it would
    /// turn the grid's "exact rewrite" claim back into a sentence.
    #[cfg(test)]
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
    /// The tagged-versus-geometric check, when the document's tree describes a table on this
    /// page (v1-S3). Filled in by the extractor, which is where both derivations meet.
    pub tagged_check: Option<engine_core::TaggedGridCheck>,
    /// Which rule produced this table (v1-S2).
    ///
    /// Exactly one of `engine_core::TABLE_DETECTION_V3`, `engine_core::TABLE_DETECTION_UNRULED_V1`
    /// or `engine_core::TABLE_DETECTION_STROKE_V1`. Set from those constants at the **three**
    /// places a table is built — `tables.rs`'s ruled arm, `unruled.rs` and `stroke_ruled.rs` —
    /// never spelled out here: a rule id written twice is a rule id that can drift, which is the
    /// whole reason it is a pinned constant.
    ///
    /// This said *"one of two"* at *"the two places"* from v1-S2 until v2-S13.5, and stopped being
    /// true at **v1-S8**, which added the third id and the third build site in one commit.
    pub rule: String,
}

/// A table the document's structure tree declares, emitted when no geometric detector matched it
/// on its page (v2-S24).
///
/// # Why this is a different type from [`DetectedTable`], not a flag on it
///
/// A `DetectedTable` is a grid this engine **inferred from ink** — `Computed`, with a box a
/// detector measured. This is a grid the document **declared in its tags** — `Extracted`, with no
/// box at all, because a structure tree states structure and never a coordinate. The two are
/// different kinds of statement, and folding them into one type would mean either giving a tagged
/// table an `Option` box (the sentinel this project refuses) or giving a detected table a
/// geometry it does not need to justify. Keeping them apart also keeps the geometric gate honest:
/// `accuracy` scores [`DetectedTable`]s against the tree, and a tagged table scored against the
/// tree it came from would be measuring the tree against itself.
///
/// The geometry is **typed-absent**: [`engine_core::GeometryPresence::Absent`] with
/// [`engine_core::GeometryAbsence::NotReportedByStructureTree`]. No rectangle is invented.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaggedTableRecord {
    /// Stable id.
    pub id: NodeId,
    /// 1-based page.
    pub page: u32,
    /// Rows, from the tree's `/TR` extent (spans counted).
    pub rows: u32,
    /// Columns, from the tree's widest row once spans are counted.
    pub columns: u32,
    /// Cells, in tree order.
    pub cells: Vec<TaggedCellRecord>,
    /// Which rule produced this table: always [`engine_core::TABLE_DETECTION_TAGGED_V1`]. Set from
    /// the constant at the one build site, never spelled here, for the reason the detected ids are
    /// not: a rule id written twice is one that can drift.
    pub rule: String,
    /// The locator cross-check. Always `NotApplicable`, and it must be: [`LOCATOR_CHECK_V1`]
    /// compares a geometric derivation against a structural one, and a tagged table has no geometry
    /// to put on the geometric side. Reporting `Ok` would be the check passing a comparison it
    /// never ran — the risk v2-S24 names explicitly.
    pub check: LocatorCheck,
    /// Typed-absent geometry: [`engine_core::GeometryAbsence::NotReportedByStructureTree`]. The box
    /// is not invented, and the reason it is missing is a property of the source rather than a gap
    /// in this reader.
    pub geometry: engine_core::GeometryPresence,
}

/// One `/TD` or `/TH` of a tagged table (v2-S24).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaggedCellRecord {
    /// Where it sits and how far it reaches, from the tree's position and `/RowSpan`/`/ColSpan`.
    pub position: TableCellPosition,
    /// Indices into the page's run list whose `(page, mcid)` the tree binds beneath this cell, in
    /// reading order. The join key is the tree's own `/MCID` — never a coordinate — so the cell's
    /// text is as independent of any geometric detector as its row and column are.
    pub run_indices: Vec<usize>,
    /// The concatenation of those runs' text. **Never a novel string**: the fabrication-0 invariant
    /// holds here exactly as it does for a detected cell — a cell binding no run carries the empty
    /// string, because that is what the document put there.
    pub text: String,
    /// Typed-absent geometry, as for the table.
    pub geometry: engine_core::GeometryPresence,
}

/// The typed-absent geometry every tagged table and tagged cell carries (v2-S24).
///
/// A single constant so the reason is stated in one place: a table read from the structure tree has
/// no box because the tree names none, and inventing one is exactly the fabrication this rule may
/// not commit.
pub const TAGGED_TABLE_GEOMETRY: engine_core::GeometryPresence =
    engine_core::GeometryPresence::Absent(engine_core::GeometryAbsence::NotReportedByStructureTree);

/// The locator cross-check a tagged table carries: `NotApplicable`, always (v2-S24).
///
/// Built here rather than inline so the reason travels with it and cannot drift into a stray `Ok`.
/// The geometric-vs-structural check compares two derivations of one table; a tagged table supplies
/// only the structural one, so the comparison cannot run.
pub fn tagged_not_applicable_check() -> LocatorCheck {
    LocatorCheck {
        check_id: LOCATOR_CHECK_V1.to_string(),
        check_version: "1".to_string(),
        outcome: CheckStatus::NotApplicable {
            reason:
                "a tagged table carries no geometry, so the geometric-vs-structural cross-check \
                     has nothing to compare against the structural derivation"
                    .to_string(),
        },
    }
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

/// Detect every table on one page: ruled, then stroke-ruled, then unruled (v1-S2, widened v1-S8).
///
/// # Arbitration, and the order the three rules run in
///
/// The rules answer different questions and can all answer for the same region. When they do,
/// exactly one table is kept, and which one is settled by **what kind of statement the evidence
/// is** rather than by which grid is larger or scores better:
///
/// 1. **`ruled-rects-v3`** — the author *filled a box* for each cell.
/// 2. **`stroke-ruled-v1`** — the author *drew the lines* of the grid.
/// 3. **`unruled-align-v1`** — the author drew nothing and this engine inferred a grid from where
///    the text sits.
///
/// The first two are both ink the author put down, so neither claim is weaker than the other;
/// what settles a region they share is simply that the ruled rule got there first, and a filled
/// cell box is the more complete statement of the two (it bounds a cell on four sides by itself).
/// The third is an inference about the document rather than evidence in it, and it loses to
/// either — preferring ours would be preferring our reading to the author's statement.
///
/// The grids are never averaged or merged. Two derivations of one region that disagree are a real
/// disagreement; splitting the difference would produce a grid no rule found, with no rule id that
/// honestly describes it.
///
/// The kept table's `rule` field is the whole diagnostic: a reader who sees `ruled-rects-v3` knows
/// the author painted it, `stroke-ruled-v1` that they ruled it, and nothing was lost that a
/// second, weaker derivation of the same cells would have added.
///
/// Unruled detection runs only on runs whose origins fall **outside** every accepted ruled and
/// stroke-ruled table, so a table's own text can never also seed an alignment lattice.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a rectangle will not quantize into a well-formed box.
pub fn detect(
    page: u32,
    rects: &[QuantRect],
    rules: &[crate::stroke_ruled::Rule],
    uprights: &[crate::stroke_ruled::Upright],
    runs: &[RunOrigin<'_>],
    fields: &[QuantRect],
    alloc: &mut IdAllocator,
) -> Result<Detected, EngineError> {
    let (mut tables, ruled_refusal) = detect_ruled(page, rects, runs, alloc)?;

    // v1-S8. The rule on the ruling LINES the page stroked, on regions the ruled rule did not
    // already claim.
    let claimed: Vec<QuantRect> = tables.iter().map(|t| t.rect).collect();
    let stroked =
        crate::stroke_ruled::detect(page, rules, uprights, runs, fields, &claimed, alloc)?;
    let stroke_refusal = stroked.refusal;
    tables.extend(stroked.tables);

    // Only runs no ruled or stroke-ruled table already claims. A run inside an accepted table is
    // spoken for; letting it also vote on an alignment lattice would let one piece of text produce
    // two tables that both claim it.
    let leftover: Vec<usize> = (0..runs.len())
        .filter(|i| {
            let r = &runs[*i];
            !tables
                .iter()
                .any(|t| r.x >= t.rect.x0 && r.x < t.rect.x1 && r.y >= t.rect.y0 && r.y < t.rect.y1)
        })
        .collect();

    let unruled = crate::unruled::detect(page, runs, &leftover, alloc)?;
    let refusal = unruled.refusal;
    if let Some(t) = unruled.table {
        // Belt and braces on the arbitration. The leftover filter already excludes runs inside a
        // ruled table, but a lattice inferred from runs *around* one could still box it in, and
        // two tables claiming one region is the thing this must not emit.
        if !tables.iter().any(|r| r.rect.overlaps(t.rect)) {
            tables.push(t);
        }
    }

    Ok(Detected {
        tables,
        refusal,
        ruled_refusal,
        stroke_refusal,
    })
}

/// Why the ruled rule refused a candidate lattice (v1-S7b).
///
/// **A disclosure, never a score**, and the direct companion to [`crate::unruled::Refusal`]. Each
/// variant names a precondition that failed; none grades how close the rectangles came, because a
/// "nearly a table" number is a confidence field wearing a different hat
/// (`docs/01-CONTRACT.md` §9).
///
/// # Why this arrived six slices after the rule
///
/// **Not because the path was cold.** The ruled rule refuses 556 of the four real documents' 602
/// pages, and did so in silence from v1-S1 until this type existed. What went unnoticed was
/// narrower: under `ruled-rects-v1` a rectangle enclosing the whole lattice satisfied coverage for
/// every face at once, so a page painting decoration on a background panel produced a *table*
/// rather than a refusal — `cfpb-home-loan-toolkit` page 22 emitted a 17 × 13 grid holding 12
/// cells. Fixing that is what made the surrounding silence visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuledRefusal {
    /// A face of the lattice was covered by no rectangle the document painted.
    ///
    /// The scattered-boxes case, and since `-v2` the panel case: the rectangles imply a grid whose
    /// cells they do not draw.
    FaceWithoutRectangle {
        /// Faces the lattice implied.
        faces: usize,
        /// Rectangles the page painted in the region.
        rects: usize,
    },
    /// The lattice exceeded [`Lattice::MAX_FACES`].
    LatticeTooLarge {
        /// Faces implied.
        faces: usize,
    },
    /// The grid's own cross-check rejected it **structurally** (v2-S20).
    ///
    /// The rule reconstructed a lattice, assigned every rectangle to it, and then
    /// [`cross_check`]'s structural half found the result contradicts itself: a slot two
    /// rectangles both claim, or a cell reaching past the grid it declares. Counts, never a
    /// score — how many faults there were, not how nearly the grid held together.
    ///
    /// **Only the structural half refuses, and the difference is measured rather than tidy.**
    /// The structural half compares indices and spans, so it is arithmetic on integers a rule
    /// assigned and no tolerance enters it: a slot owned twice is a contradiction in the rule's
    /// own bookkeeping. The geometric half compares **exact boxes** against a lattice built with
    /// [`LATTICE_TOLERANCE`], so it reports the very slop that tolerance exists to absorb — a
    /// grid drawn as 1 pt stroked rules, whose edges are a centipoint apart, disagrees with
    /// itself geometrically while being a perfectly good grid.
    /// `tests::near_edges_fold_into_one_lattice_line` is that case, and it emits.
    ///
    /// So the geometric count is carried here for disclosure and is not what declined the
    /// candidate. A table whose geometry alone disagrees is still emitted, still carrying its
    /// `Mismatch` — which is what keeps [`engine_core::CheckStatus::Mismatch`] a reachable state
    /// on the wire rather than one this slice quietly retired.
    CrossCheckRejected {
        /// Structural faults — the indices and spans disagreeing among themselves. **This is the
        /// count that refused the candidate**, and it is never zero here.
        structural: usize,
        /// Geometric faults — the boxes disagreeing among themselves. Reported, not decisive.
        geometric: usize,
    },
}

impl RuledRefusal {
    /// Which precondition failed, as a stable short name.
    ///
    /// The limitation groups by this and prints [`Self::explanation`] **once** per kind rather
    /// than once per page. On `nist-sp-800-53r5` the ruled rule refuses 481 of 492 pages, and
    /// repeating a five-line explanation 481 times would put a quarter of a megabyte of identical
    /// prose inside a hashed artifact. Every page is still named, with its own numbers — this is
    /// a change of layout, not a truncation.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::FaceWithoutRectangle { .. } => "a cell the ink does not draw",
            Self::LatticeTooLarge { .. } => "past the cell ceiling",
            Self::CrossCheckRejected { .. } => "a grid that contradicts itself",
        }
    }

    /// The reasoning behind a kind of refusal, stated once.
    pub fn explanation(&self) -> String {
        match self {
            Self::FaceWithoutRectangle { .. } => String::from(
                "The rectangles implied a grid whose cells they do not all draw. A ruled grid must \
                 be explained by the ink face by face, or it is a lattice this engine inferred \
                 rather than one the document drew. A rectangle merely ENCLOSING the grid does not \
                 count: it is the table's own border, it is not emitted as a cell, and it is not \
                 evidence that the cells exist either",
            ),
            Self::LatticeTooLarge { .. } => format!(
                "The rectangles implied more than the {}-cell ceiling for a reconstructed grid. \
                 Refused rather than truncated: a truncated table is a table with cells missing \
                 and no way to say which",
                Lattice::MAX_FACES
            ),
            Self::CrossCheckRejected { .. } => String::from(
                "The grid was reconstructed and then rejected by its own cross-check \
                 (`geometric-vs-structural-v1`): the row and column indices this rule assigned \
                 and the boxes it assigned them from do not describe the same grid. Two \
                 rectangles claiming one slot is the usual case, and it means the ink itself is \
                 not consistent about where a cell is. Refused rather than emitted with the \
                 disagreement noted beside it, because a table on the wire is projected as a \
                 table — the Markdown and HTML projections draw every grid the artifact carries \
                 and read no check — so a contradiction recorded in a field nothing consults \
                 would reach a reader as a grid and reach nobody as a warning. The disagreement \
                 is not discarded: it is this entry, with the page and the fault counts",
            ),
        }
    }

    /// This page's own numbers, without the reasoning.
    pub fn detail(&self) -> String {
        match self {
            Self::FaceWithoutRectangle { faces, rects } => {
                format!("{rects} rectangles implied {faces} cells")
            }
            Self::LatticeTooLarge { faces } => format!("{faces} cells implied"),
            Self::CrossCheckRejected {
                structural,
                geometric,
            } => format!("{structural} structural and {geometric} geometric fault(s)"),
        }
    }
}

/// What one page's detection produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    /// The tables, ruled first.
    pub tables: Vec<DetectedTable>,
    /// Why the alignment rule refused a candidate here, if it built one and refused it.
    ///
    /// `None` means it never got as far as a candidate — there was nothing lattice-shaped to
    /// refuse. This is a *disclosure*, not a score: it names the precondition that failed and
    /// never grades how close the candidate came (`docs/01-CONTRACT.md` §9).
    pub refusal: Option<crate::unruled::Refusal>,
    /// Why the **ruled** rule refused a candidate here, if it built one and refused it (v1-S7b).
    ///
    /// Separate from `refusal` rather than folded into it: the two rules read different evidence,
    /// and one page can perfectly well have its rectangles refused and its alignment refused for
    /// unrelated reasons. Collapsing them would make a reader guess which rule spoke.
    pub ruled_refusal: Option<RuledRefusal>,
    /// Why the **stroke-ruled** rule refused a candidate band here, if it built one (v1-S8).
    ///
    /// A third field for the third rule, for the same reason the second one is separate: one page
    /// can have its rectangles refused, its ruling lines refused and its alignment refused, each
    /// on its own evidence, and a reader must never have to guess which rule spoke.
    pub stroke_refusal: Option<crate::stroke_ruled::Refusal>,
}

/// Detect ruled tables on one page.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a rectangle will not quantize into a well-formed box.
pub fn detect_ruled(
    page: u32,
    rects: &[QuantRect],
    runs: &[RunOrigin<'_>],
    alloc: &mut IdAllocator,
) -> Result<(Vec<DetectedTable>, Option<RuledRefusal>), EngineError> {
    let lattice = match Lattice::build(rects) {
        Ok(l) => l,
        Err(refusal) => return Ok((Vec::new(), refusal)),
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

    // **v2-S20: the structural half of the check is acted on.** Until this slice the cross-check
    // was computed here and consumed by nothing — a grid that contradicted itself was emitted
    // with the contradiction recorded beside it. `nist-sp-800-218` is where that stopped being
    // theoretical: nine grids of up to 103 x 22 built from page furniture its rectangles fold
    // into one lattice, every one of them carrying `OwnedMoreThanOnce` faults by the hundred, and
    // together contributing 11 295 false-positive cell slots to the twelve-document gate corpus —
    // more than the rest of it produces in either direction.
    //
    // **Why the rule declines rather than declaring.** `engine_core::markdown` and
    // `engine_core::html` project every table the artifact carries and consult no check, so a
    // reader of either projection receives the grid and never the disagreement. A disclosure no
    // surface reads is a disclosure in name only, and this repository has a name for that shape.
    //
    // **Why only the structural half.** See [`RuledRefusal::CrossCheckRejected`]: the structural
    // half is arithmetic on indices this rule assigned and admits no tolerance, while the
    // geometric half compares exact boxes against a lattice built with `LATTICE_TOLERANCE` and so
    // fires on the slop that tolerance exists to absorb. Gating on both was measured first and
    // refuses `tests::near_edges_fold_into_one_lattice_line` — a 2 x 2 whose only defect is that
    // one edge sits a single centipoint out, which is what a 1 pt stroked rule looks like.
    //
    // **Nothing is deleted.** The refusal below carries the page and both fault counts into
    // `ruled-table-candidate-refused`, the channel this rule's other two refusals already use —
    // so the artifact still says a grid was found here and rejected, and why.
    //
    // **This is the only rule whose check can fail at all.** `crate::stroke_ruled` and
    // `crate::unruled` build a cell for every face of their lattice, so their cells tile the
    // table exactly, never overlap and never fall outside it; their cross-check is `Ok` by
    // construction. Gating them too would be dead code, and
    // `tests::the_other_two_rules_build_a_cell_for_every_face` is the proof rather than a comment.
    if let CheckStatus::Mismatch {
        structural,
        geometric,
    } = &check.outcome
    {
        if !structural.is_empty() {
            return Ok((
                Vec::new(),
                Some(RuledRefusal::CrossCheckRejected {
                    structural: structural.len(),
                    geometric: geometric.len(),
                }),
            ));
        }
    }

    // A well-formed box for the table, or the whole detection is refused rather than emitted with
    // geometry the contract cannot express.
    table_rect.as_qrect_checked()?;

    Ok((
        vec![DetectedTable {
            id,
            page,
            rect: table_rect,
            rows: lattice.rows(),
            columns: lattice.columns(),
            cells: detected,
            check,
            tagged_check: None,
            rule: engine_core::TABLE_DETECTION_V3.to_string(),
        }],
        None,
    ))
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
        // Same pairs, same wire order, one sweep instead of all pairs (up to 4096
        // cells made that 16.7M overlap tests per table). Indices sort by x0; a
        // candidate pair must satisfy the overlap predicate's own necessary
        // condition x0_later < x1_earlier before the full test runs; and because
        // the wire has always recorded faults in (i, j > i) cell order, the found
        // pairs are re-sorted into exactly that order before they are pushed.
        let mut by_x0: Vec<usize> = (0..cells.len()).collect();
        by_x0.sort_unstable_by_key(|&i| (cells[i].rect.x0, i));
        let mut overlapping: Vec<(usize, usize)> = Vec::new();
        for (k, &p) in by_x0.iter().enumerate() {
            for &q in &by_x0[k + 1..] {
                if cells[q].rect.x0 >= cells[p].rect.x1 {
                    break;
                }
                if cells[p].rect.overlaps(cells[q].rect) {
                    overlapping.push((p.min(q), p.max(q)));
                }
            }
        }
        overlapping.sort_unstable();
        for (i, j) in overlapping {
            geometric.push(GeometricFault::CellsOverlap {
                a: CellSlot::new(cells[i].position.row, cells[i].position.column),
                b: CellSlot::new(cells[j].position.row, cells[j].position.column),
            });
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
    /// The most faces a ruled grid may have. See [`MAX_FACES`].
    const MAX_FACES: usize = MAX_FACES;

    /// The lattice, or why there is not one (v1-S7b).
    ///
    /// `Err(None)` means no candidate ever existed — no rectangles, or too few lines to bound two
    /// faces. That is an ordinary page, not a near miss, and declaring it would put a limitation on
    /// every document in existence. `Err(Some(_))` is a real refusal with a reason, and it reaches
    /// the artifact as `ruled-table-candidate-refused`.
    fn build(rects: &[QuantRect]) -> Result<Self, Option<RuledRefusal>> {
        if rects.is_empty() {
            return Err(None);
        }
        let xs = cluster(rects.iter().flat_map(|r| [r.x0, r.x1]));
        let ys = cluster(rects.iter().flat_map(|r| [r.y0, r.y1]));
        // Two lines in each axis bound one face — but **one face is not a table**. A lone
        // rectangle is an underline, a highlight, a text-box border; calling it a 1x1 table would
        // find one on most pages in existence, and `docs/09-V1-MILESTONES.md` S1 names "a
        // fabricated 1x1 table around the page" as a thing this must not do. A grid needs at
        // least two faces.
        if xs.len() < 2 || ys.len() < 2 {
            return Err(None);
        }
        let faces = (xs.len() - 1) * (ys.len() - 1);
        if faces < 2 {
            // One face is a box, not a grid. No candidate existed.
            return Err(None);
        }
        if faces > Self::MAX_FACES {
            return Err(Some(RuledRefusal::LatticeTooLarge { faces }));
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
        //
        // **A rectangle spanning the whole lattice is not evidence** (v1-S7b). `detect_ruled`
        // already refuses to emit such a rectangle as a cell — *"the table's own border"* — and
        // the two claims cannot both stand: a rectangle that is not a cell because it merely
        // encloses the grid is equally not proof that the grid's faces were drawn. Counting it
        // was how `cfpb-home-loan-toolkit` page 22 became a 17 x 13 table with 12 cells on a page
        // whose tree declares no table at all: the page paints a 351 x 454 pt background panel,
        // the panel covers every face, and twelve scattered highlight bars supplied the edges.
        // Measured across that document, this single confusion produced 79 of the 91 false-positive
        // cell slots the gate charged against the ruled rule.
        let encloses_everything = |r: &QuantRect| {
            lattice
                .span_of(*r)
                .is_some_and(|s| (s.rowspan * s.colspan) as usize == faces)
        };
        // The same precondition, decided in one pass instead of one scan per
        // (face, rectangle) pair. `covers_within_tolerance` against a face whose
        // edges ARE lattice lines is an interval condition on the line indices —
        // xs[c] >= r.x0 − tol and xs[c+1] <= r.x1 + tol, and likewise for rows —
        // so each rectangle marks the block of faces it covers in a 2-D difference
        // grid, and `encloses_everything`, a property of the rectangle alone, runs
        // once per rectangle instead of once per pair. A face is uncovered under
        // this scan exactly when it was under the per-pair scan, and the refusal
        // carries the identical payload, so nothing on the wire can move.
        let columns = lattice.columns() as usize;
        let row_count = lattice.rows() as usize;
        let mut coverage = vec![0i64; (row_count + 1) * (columns + 1)];
        let stride = columns + 1;
        for r in rects {
            if encloses_everything(r) {
                continue;
            }
            let c_lo = lattice
                .xs
                .partition_point(|x| *x < r.x0 - LATTICE_TOLERANCE);
            let c_hi = lattice
                .xs
                .partition_point(|x| *x <= r.x1 + LATTICE_TOLERANCE);
            let r_lo = lattice
                .ys
                .partition_point(|y| *y < r.y0 - LATTICE_TOLERANCE);
            let r_hi = lattice
                .ys
                .partition_point(|y| *y <= r.y1 + LATTICE_TOLERANCE);
            // Covered faces are [r_lo, r_hi−1) × [c_lo, c_hi−1): a face needs both
            // its near line inside the rectangle's tolerance band and its far line,
            // which is the next line up.
            if c_hi < c_lo + 2 || r_hi < r_lo + 2 {
                continue;
            }
            let (c1, r1) = (c_hi - 1, r_hi - 1);
            coverage[r_lo * stride + c_lo] += 1;
            coverage[r_lo * stride + c1] -= 1;
            coverage[r1 * stride + c_lo] -= 1;
            coverage[r1 * stride + c1] += 1;
        }
        let mut running = vec![0i64; (row_count + 1) * (columns + 1)];
        for row in 0..row_count {
            for column in 0..columns {
                let above = if row > 0 {
                    running[(row - 1) * stride + column]
                } else {
                    0
                };
                let left = if column > 0 {
                    running[row * stride + column - 1]
                } else {
                    0
                };
                let diag = if row > 0 && column > 0 {
                    running[(row - 1) * stride + column - 1]
                } else {
                    0
                };
                let total = coverage[row * stride + column] + above + left - diag;
                running[row * stride + column] = total;
                if total == 0 {
                    return Err(Some(RuledRefusal::FaceWithoutRectangle {
                        faces,
                        rects: rects.len(),
                    }));
                }
            }
        }

        Ok(lattice)
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

    /// Retained under `cfg(test)` with [`QuantRect::covers_within_tolerance`], as
    /// the other half of the coherence scan's executable spec.
    #[cfg(test)]
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
    // Binary search with the linear scan's exact semantics: the FIRST line within
    // ±LATTICE_TOLERANCE of `v`. `lines` is sorted, so the first candidate at all
    // is the first line ≥ v − tolerance; it matches iff it is also ≤ v + tolerance.
    // (Clustering keeps successive lines more than one tolerance apart, but nothing
    // here relies on that — the equivalence holds for any sorted input.)
    let first = lines.partition_point(|l| *l < v - LATTICE_TOLERANCE);
    (lines.get(first).copied()? - v <= LATTICE_TOLERANCE).then_some(first)
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

    /// The difference-grid coherence scan agrees with its per-pair spec on every
    /// face, for lattices that pass and lattices that refuse — including the
    /// tolerance edges, a spanning rectangle, and the excluded whole-grid border.
    #[test]
    fn the_coverage_grid_is_the_per_pair_scan_by_another_route() {
        let tol_pt = LATTICE_TOLERANCE / i64::from(QUANTUM_PER_POINT);
        let cases: Vec<(&str, Vec<QuantRect>)> = vec![
            ("clean 2x2 grid", vec![
                r(0, 0, 100, 50), r(100, 0, 200, 50),
                r(0, 50, 100, 100), r(100, 50, 200, 100),
            ]),
            ("one face uncovered", vec![
                r(0, 0, 100, 50), r(100, 0, 200, 50),
                r(0, 50, 100, 100),
            ]),
            ("cover within tolerance", vec![
                r(0, 0, 100, 50), r(100, 0, 200, 50),
                r(0, 50, 100, 100),
                // Edges one tolerance inside the face it must cover.
                QuantRect {
                    x0: pt(100) + LATTICE_TOLERANCE,
                    y0: pt(50) + LATTICE_TOLERANCE,
                    x1: pt(200) - LATTICE_TOLERANCE,
                    y1: pt(100) - LATTICE_TOLERANCE,
                },
            ]),
            ("cover just past tolerance", vec![
                r(0, 0, 100, 50), r(100, 0, 200, 50),
                r(0, 50, 100, 100),
                r(100 + tol_pt + 1, 50, 200, 100),
            ]),
            ("spanning rectangle covers two faces", vec![
                r(0, 0, 100, 50), r(100, 0, 200, 50),
                r(0, 50, 200, 100),
            ]),
            ("whole-grid border is not evidence", vec![
                r(0, 0, 100, 50), r(100, 0, 200, 50),
                r(0, 50, 100, 100), r(100, 50, 200, 100),
                r(0, 0, 200, 100),
            ]),
        ];
        for (name, rects) in cases {
            let built = Lattice::build(&rects);
            // The spec's own answer, re-derived per (face, rectangle) pair exactly
            // as the pre-0.37.2 scan computed it.
            let spec = |lattice: &Lattice| {
                let encloses = |rect: &QuantRect| {
                    lattice.span_of(*rect).is_some_and(|s| {
                        (s.rowspan * s.colspan) as usize
                            == (lattice.rows() * lattice.columns()) as usize
                    })
                };
                (0..lattice.rows()).all(|row| {
                    (0..lattice.columns()).all(|column| {
                        let face = lattice.face(row, column);
                        rects
                            .iter()
                            .any(|rect| !encloses(rect) && rect.covers_within_tolerance(face))
                    })
                })
            };
            match built {
                Ok(lattice) => assert!(
                    spec(&lattice),
                    "{name}: the grid accepted a lattice the per-pair spec refuses"
                ),
                Err(Some(RuledRefusal::FaceWithoutRectangle { .. })) => {
                    // Rebuild the lattice geometry alone to ask the spec the same
                    // question the grid answered.
                    let xs = cluster(rects.iter().flat_map(|rect| [rect.x0, rect.x1]));
                    let ys = cluster(rects.iter().flat_map(|rect| [rect.y0, rect.y1]));
                    let lattice = Lattice { xs, ys };
                    assert!(
                        !spec(&lattice),
                        "{name}: the grid refused a lattice the per-pair spec accepts"
                    );
                }
                Err(other) => panic!("{name}: unexpected refusal {other:?}"),
            }
        }
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
        let t = detect_ruled(1, &[], &[], &mut alloc()).unwrap().0;
        assert!(t.is_empty(), "a page with no rules has no ruled table");
    }

    #[test]
    fn a_single_rectangle_is_not_a_grid() {
        // One box is an underline, a highlight, a border — not a table. Calling it a 1×1 table
        // would find one on most pages in existence.
        let t = detect_ruled(1, &[r(0, 0, 100, 100)], &[], &mut alloc())
            .unwrap()
            .0;
        assert!(t.is_empty(), "got {t:?}");
    }

    #[test]
    fn a_plain_grid_is_detected_and_cross_checks_ok() {
        let t = detect_ruled(1, &grid_2x2(), &[], &mut alloc()).unwrap().0;
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
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
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
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
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
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
        assert!(
            t.is_empty(),
            "a page with boxes on it is not a table: {t:?}"
        );
    }

    /// **A background panel is not evidence that a grid was drawn** (v1-S7b).
    ///
    /// The defect this pins, measured on `cfpb-home-loan-toolkit` page 22: the page paints a
    /// 351 × 454 pt panel behind twelve small text-highlight bars. The bars' edges cluster into a
    /// 17 × 13 lattice, the panel covers every one of its 221 faces, and coherence passed — so the
    /// page emitted a table declaring 17 rows and 13 columns while holding 12 cells, on a page
    /// whose structure tree declares no table at all.
    ///
    /// The panel was simultaneously *not a cell* (`detect_ruled` skips it as "the table's own
    /// border") and *proof that every cell exists*. Those cannot both be true, and this is the
    /// half that was wrong.
    #[test]
    fn a_background_panel_does_not_make_scattered_bars_a_grid() {
        let mut rects = vec![
            r(0, 0, 400, 500), // the panel
        ];
        // Bars at unrelated positions, exactly as a highlighted list looks. Their edges imply a
        // lattice; nothing about them tiles it.
        for (i, y) in [40, 130, 260, 380].iter().enumerate() {
            let x0 = 20 + (i as i64) * 17;
            rects.push(r(x0, *y, x0 + 150, y + 12));
        }
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
        assert!(
            t.is_empty(),
            "a panel with bars on it is not a grid. The panel covers every face, but a rectangle \
             that merely encloses the lattice is not evidence that the lattice was drawn — it is \
             the same rectangle `an_outer_border_is_not_mistaken_for_a_cell` refuses to emit. Got \
             {t:?}"
        );
    }

    /// The companion to the test above: excluding the border as a **witness** must not stop a
    /// genuine bordered grid being found, because its own cells still witness their own faces.
    #[test]
    fn a_bordered_grid_is_still_a_grid_after_the_border_stops_being_evidence() {
        let mut rects = grid_2x2();
        rects.push(r(0, 0, 200, 200));
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
        assert_eq!(
            t.len(),
            1,
            "the four cell rectangles cover all four faces on their own"
        );
        assert_eq!(t[0].cells.len(), 4);
    }

    #[test]
    fn an_outer_border_is_not_mistaken_for_a_cell() {
        let mut rects = grid_2x2();
        rects.push(r(0, 0, 200, 200)); // the table's own border
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
        let table = &t[0];
        assert_eq!(table.cells.len(), 4, "the border is not a fifth cell");
        assert_eq!(table.check.outcome, CheckStatus::Ok, "{:?}", table.check);
    }

    /// **The cross-check still sees the double claim** — asserted on the check itself, because
    /// since v2-S20 no emitted table can carry a `Mismatch` to assert it on.
    ///
    /// That is the cost of gating emission on this check, and it is why this test exists in this
    /// shape rather than being deleted with the table it used to read: the check is still the
    /// engine's only statement about whether a reconstructed grid agrees with itself, and it must
    /// still be able to make it. What changed is who acts on the answer.
    #[test]
    fn the_cross_check_still_sees_two_rectangles_claiming_one_slot() {
        let table = r(0, 0, 200, 200);
        let id = alloc().next(IdKind::Table).expect("allocates");
        let cell = |row, column, rect| DetectedCell {
            position: TableCellPosition {
                row,
                column,
                rowspan: 1,
                colspan: 1,
                table_id: id.clone(),
            },
            rect,
            run_indices: Vec::new(),
            text: String::new(),
        };
        // Row 0 column 1 is claimed twice, and the two boxes overlap. Both halves must see it,
        // independently — that independence is what makes this a check rather than a restatement.
        let cells = vec![
            cell(0, 0, r(0, 0, 100, 100)),
            cell(0, 1, r(100, 0, 200, 100)),
            cell(0, 1, r(100, 0, 200, 100)),
            cell(1, 0, r(0, 100, 100, 200)),
            cell(1, 1, r(100, 100, 200, 200)),
        ];

        match &cross_check(table, 2, 2, &cells).outcome {
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

    /// **A grid whose own cross-check rejects it is refused, and the refusal says so** (v2-S20).
    ///
    /// The hostile case, and the whole of this slice. Two rectangles claim the same face. Until
    /// v2-S20 the rule emitted the grid with the contradiction recorded beside it; the reason it
    /// no longer does is that neither projection this repository ships reads that field, so the
    /// grid reached a consumer and the contradiction did not.
    ///
    /// Nothing is repaired and nothing is silently dropped: the geometry is not nudged to make
    /// the grid tile, and the candidate leaves as a named refusal carrying its fault counts.
    #[test]
    fn a_grid_its_own_cross_check_rejects_is_refused_rather_than_emitted() {
        let rects = vec![
            r(0, 0, 200, 100),
            r(100, 0, 200, 100),
            r(0, 100, 100, 200),
            r(100, 100, 200, 200),
        ];
        let (tables, refusal) = detect_ruled(1, &rects, &[], &mut alloc()).unwrap();

        assert!(
            tables.is_empty(),
            "a grid the rule has itself found to contradict itself must not reach the artifact"
        );
        match refusal {
            Some(RuledRefusal::CrossCheckRejected {
                structural,
                geometric,
            }) => {
                assert!(
                    structural > 0 && geometric > 0,
                    "the refusal must carry what disagreed, from both halves: \
                     {structural} structural, {geometric} geometric"
                );
            }
            other => panic!("expected a cross-check refusal, got {other:?}"),
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
        let t = detect_ruled(1, &grid_2x2(), &runs, &mut alloc()).unwrap().0;
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
        let t = detect_ruled(1, &grid_2x2(), &runs, &mut alloc()).unwrap().0;
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
        let t = detect_ruled(1, &rects, &[], &mut alloc()).unwrap().0;
        // **A table is emitted** — the wire-emission is the whole point, so it is asserted
        // rather than left implicit in the `t[0]` below (v2-S23).
        assert_eq!(
            t.len(),
            1,
            "the folded grid must reach the artifact, not be refused"
        );
        assert_eq!(
            (t[0].rows, t[0].columns),
            (2, 2),
            "a one-centipoint nudge must not open a column"
        );

        // **And it still emits, carrying the disagreement** (v2-S20). The lattice folded the
        // nudged edge; the cross-check's geometric half compares exact boxes and does not, so
        // this grid disagrees with itself geometrically while being a perfectly good grid. That
        // is the case the v2-S20 gate is deliberately narrow enough to admit — gating on both
        // halves refuses it — and it is what keeps `CheckStatus::Mismatch` a state an emitted
        // table can still be in rather than one this slice retired by construction.
        //
        // **v2-S23 names the fault kind, verified rather than inherited.** After v2-S20 the
        // `ruled-table-overlap` fixture no longer carries a `Mismatch` on the wire — it is
        // refused structurally — so this synthetic 2 x 2 is the *only* thing keeping the variant
        // reachable, and the record relied on that without asserting which fault it is. Nudging
        // cell (1,1)'s left edge one centipoint into cell (1,0) makes the exact boxes overlap by
        // that centipoint, so the fault is `CellsOverlap` (and the areas no longer sum, so
        // `DoesNotTile` rides with it). Both are geometric; neither is structural. Pinning the
        // variant here means a refactor that stopped producing an overlap — and so quietly
        // retired the on-wire `Mismatch` — fails this test rather than passing it.
        match &t[0].check.outcome {
            CheckStatus::Mismatch {
                structural,
                geometric,
            } => {
                assert!(
                    structural.is_empty(),
                    "a folded edge is not a bookkeeping error: {structural:?}"
                );
                assert!(
                    geometric
                        .iter()
                        .any(|f| matches!(f, GeometricFault::CellsOverlap { .. })),
                    "the folded edge overlaps its neighbour by the centipoint, so the geometric \
                     fault that keeps `Mismatch` on the wire is `CellsOverlap`: {geometric:?}"
                );
            }
            other => panic!(
                "expected a geometric-only mismatch — the state that proves the v2-S20 gate did \
                 not retire `Mismatch` on the wire — got {other:?}"
            ),
        }
    }

    /// **The other two rules cannot fail their cross-check**, which is why v2-S20 gates only this
    /// one and gating them would be dead code.
    ///
    /// `crate::stroke_ruled` and `crate::unruled` both emit a cell for **every** face of their
    /// lattice, computed from the same lines the table's own box is computed from. Their cells
    /// therefore tile the table exactly, no two overlap, and none reaches outside it — so both
    /// halves of the check are empty by construction. The ruled rule is different in kind: its
    /// cells are the rectangles the document painted, and the document may paint two of them over
    /// one face.
    ///
    /// Run rather than read: each rule is given the smallest input that makes it fire, and the
    /// property is asserted on what it produced.
    #[test]
    fn the_other_two_rules_build_a_cell_for_every_face() {
        let q = |v: f64| (v * f64::from(QUANTUM_PER_POINT)).round() as i64;

        // A 2 x 2 ruled in lines: three baselines bound two rows, three column lines bound two
        // columns, and the one interior column line is stroked across the band.
        let rules: Vec<crate::stroke_ruled::Rule> = [100.0, 150.0, 200.0]
            .iter()
            .flat_map(|y| {
                [(0.0, 50.0), (50.0, 100.0)].map(|(x0, x1)| crate::stroke_ruled::Rule {
                    y: q(*y),
                    x0: q(x0),
                    x1: q(x1),
                })
            })
            .collect();
        let uprights = vec![crate::stroke_ruled::Upright {
            x: q(50.0),
            y0: q(100.0),
            y1: q(200.0),
        }];
        let stroked =
            crate::stroke_ruled::detect(1, &rules, &uprights, &[], &[], &[], &mut alloc())
                .expect("detects");
        let stroke_table = stroked.tables.first().expect("the band is a table");

        // A 2 x 2 implied by four runs at four aligned origins, which is what the alignment rule
        // reads and all it reads.
        let runs = [
            (10.0, 10.0, "a"),
            (110.0, 10.0, "b"),
            (10.0, 60.0, "c"),
            (110.0, 60.0, "d"),
        ]
        .map(|(x, y, text)| RunOrigin {
            x: q(x),
            y: q(y),
            text,
        });
        let leftover: Vec<usize> = (0..runs.len()).collect();
        let inferred = crate::unruled::detect(1, &runs, &leftover, &mut alloc()).expect("detects");
        let unruled_table = inferred.table.expect("the alignment rule finds this grid");

        for t in [stroke_table, &unruled_table] {
            assert_eq!(
                t.cells.len(),
                (t.rows * t.columns) as usize,
                "`{}` must build a cell for every face, or its check could fail and v2-S20's \
                 gate on the ruled rule alone would be leaving a case out",
                t.rule
            );
            assert_eq!(
                t.check.outcome,
                CheckStatus::Ok,
                "`{}` produced a cross-check disagreement. `detect_ruled` says these two cannot, \
                 and gates only itself on that basis — if that is no longer true the gate is \
                 what should change, not this assertion: {:?}",
                t.rule,
                t.check
            );
        }
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
        let t = detect_ruled(1, &grid_2x2(), &[], &mut alloc()).unwrap().0;
        assert_eq!(t[0].check.check_id, LOCATOR_CHECK_V1);
        assert!(!t[0].check.check_version.is_empty());
    }
}
