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

//! `gutter-columns-v1`: reading order from whitespace in page space (v1-S5).
//!
//! [`ethos_parser_core::READING_ORDER_RULE_V1`] is this module. The id is on the profile and on every
//! artifact, and **every constant below is part of it** — changing one is a new id, not a tuning
//! pass, for the same reason the table detectors' tolerances are.
//!
//! # The rule this is deliberately not
//!
//! pdf-inspector decides multi-column on `min_lines < 15`. Fourteen lines per column come out
//! row-interleaved and fifteen come out column-major, so **adding one line to a document
//! reorders the whole page** (`docs/history/03-V0-SCOPE.md` §3.2). That is a cliff, and a cliff cannot
//! sit under a determinism contract: two documents differing by a sentence are not two documents
//! that should disagree about what reading order means.
//!
//! Nothing here counts lines, counts runs, counts characters, or compares any tally to a
//! threshold. The only question asked is geometric — *is there a vertical band of this page that
//! no text crosses* — and the answer does not move when a line is added on either side of it.
//! A `fixtures/engine/two-column-14-lines` / `-15-lines` pair exists to hold that: the same order
//! comes out of both, and a port of the line-count rule fails on exactly that pair.
//!
//! # The rule
//!
//! 1. **Atoms.** Each accepted table on the page is ONE atom holding every run whose origin falls
//!    inside its box, in content-stream order. Every other run is an atom of its own. A cut
//!    never runs through an atom, so a table cannot be shredded into fake columns of cell
//!    fragments — see [`Atom`].
//! 2. **Vertical cut.** Project every atom onto the x axis and sweep left to right. A gap wider
//!    than [`COLUMN_GUTTER_MIN`] between the running right edge and the next atom's left edge
//!    splits the page into column bands, read left to right.
//! 3. **The guard**, which is what stops an indent from reading as a column: adjacent bands must
//!    overlap vertically over at least [`COLUMN_OVERLAP_NUMERATOR`]/[`COLUMN_OVERLAP_DENOMINATOR`]
//!    of the block's own height. Columns run *beside* each other; a heading above an indented
//!    list does not. See [`bands_sit_side_by_side`].
//! 4. **Horizontal cut,** inside a band only: the same sweep on the y axis, splitting a column
//!    into blocks read top to bottom, and each block may split into columns again.
//! 5. **No gutter, no reordering.** A page whose first vertical cut fails comes out in
//!    content-stream order, unchanged, byte for byte. This is the single-column case and it is
//!    the overwhelmingly common one; see [`arrange`] for why the fallback is identity rather than
//!    a y-then-x sort.
//!
//! # Why not sort by y, then x
//!
//! Because it reorders documents that were already right. Every indented paragraph, every
//! superscript, every run whose baseline is a quantum off its neighbour's would move. A global
//! sort is not a reading-order rule; it is a claim that the content stream carries no information
//! about order, which is false for the single-column pages that are most of every corpus. This
//! rule reorders only where it has geometric evidence of columns and otherwise does nothing.
//!
//! # Why not the structure tree
//!
//! Emitting nodes in `/K` order is a *different* reading-order rule reading *different* evidence,
//! and it would need its own id. `crate::structure` walks the tree for addresses and contains no
//! sort; a guard test in that module keeps it that way. Structure-order reading is a named
//! leftover, not something this rule quietly half-does.
//!
//! # Integers only
//!
//! Every coordinate here is an `i64` in centipoints, the same quantum the locators are in
//! (`docs/01-CONTRACT.md` §4). No float enters the decision, so the cut a page gets is the cut it
//! gets on every machine, and the ordering is a pure function of the quantized origins.

use crate::tables::QuantRect;

// -------------------------------------------------------------------------------------------
// The constants. All of them are `gutter-columns-v1`.
// -------------------------------------------------------------------------------------------

/// The narrowest vertical whitespace band that counts as a column gutter, in centipoints.
///
/// 1 200 — twelve points. Sized against what it must **reject**: an inter-word space is roughly a
/// quarter to a third of the type size, so at 12pt it is 3–4pt and at 24pt still under 8pt. A
/// twelve-point floor sits above all of those and far below any real column gutter, which runs to
/// tens of points.
///
/// # Deliberately a separate constant from `crate::unruled::COLUMN_GUTTER_MIN`
///
/// The two hold the same number today and are reasoned from the same fact about type. They are
/// still two constants, because they belong to two rules with two ids: this one is part of
/// [`ethos_parser_core::READING_ORDER_RULE_V1`] and that one is part of
/// [`ethos_parser_core::TABLE_DETECTION_UNRULED_V1`]. Sharing the binding would mean a change made for
/// table detection silently reordered every multi-column document in the corpus, under an id that
/// did not move. `the_two_gutter_floors_are_independent` asserts they are separately editable.
pub const COLUMN_GUTTER_MIN: i64 = 1_200;

/// The shortest horizontal whitespace band that separates two blocks within a column.
///
/// 600 — six points. Lower than the column floor on purpose, and for the same reason
/// `crate::unruled::ROW_GUTTER_MIN` is: consecutive lines of body text are a few points apart, so
/// a six-point floor separates paragraphs and headings from the lines inside them.
///
/// This one only ever fires **inside** an accepted column band. A page that never split
/// vertically is never cut horizontally either, so this constant cannot reorder a single-column
/// page.
pub const BLOCK_GUTTER_MIN: i64 = 600;

/// The horizontal extent given to a run whose font supplies no advance, in centipoints.
///
/// 300 — three points, a **fixed quantum**, and specifically *not* a function of font size. A run
/// with no advance has an unknown width: the standard-14 fonts may omit `/Widths` and expect
/// built-in AFM metrics this profile does not vendor, and most of the fixture corpus is in
/// exactly that state. The origin is still exact — it comes from the content stream — so what is
/// missing is the right-hand edge and only the right-hand edge.
///
/// # This is a floor, and a floor makes cuts MORE likely, not fewer
///
/// Stated plainly because it is the honest reading. A narrow extent leaves more whitespace for
/// the sweep to find, so a rule resting on this number alone would split pages that are not
/// columns. It does not rest on it: [`bands_sit_side_by_side`] is what carries the decision, and
/// it looks at vertical overlap, which the missing advance does not touch. The alternative —
/// estimating a width from the type size — is the pdf-inspector defect this project exists to
/// refuse (`docs/01-CONTRACT.md` §5), and it is not done here at any threshold.
///
/// Separate from `crate::unruled::ORIGIN_BOX_PADDING`, which happens to hold the same number for
/// a related reason (an origin is a point and a rule needs an interval), under a different rule id.
pub const UNKNOWN_ADVANCE_EXTENT: i64 = 300;

/// Numerator of the vertical-overlap fraction two bands must share to be columns.
///
/// See [`COLUMN_OVERLAP_DENOMINATOR`].
pub const COLUMN_OVERLAP_NUMERATOR: i64 = 1;

/// Denominator of the vertical-overlap fraction: adjacent bands must share **half the height of
/// the shorter of the two** before a vertical gutter between them counts as a column gutter.
///
/// # What this rejects
///
/// A heading at the top of a page and an indented list below it are separated by a wide vertical
/// band of whitespace, and a sweep on the x axis alone would call that a column gutter and read
/// the heading as column one. They are not columns, and the difference is not subtle: columns run
/// *beside* each other and share a vertical range, while a heading and the text under it share
/// none at all. Overlap must also be **strictly positive**, which is what disposes of the
/// zero-height cases — a heading whose single baseline just touches the block below it, a leader
/// line, a running header — where a fraction of nothing would otherwise be satisfied by nothing.
///
/// # Why the shorter band, and not the whole block
///
/// Because bands are not the same height and the question is whether the smaller one sits
/// *inside* the larger one's vertical range. Measuring against the block's full height would make
/// the test unsatisfiable for any short band beside a tall one — three lines of text beside a
/// forty-line column, or a paragraph beside a table whose box is taller than the origins next to
/// it — which are all the plainly-side-by-side pictures this rule exists to order.
///
/// # What it costs
///
/// A genuine two-column page whose first column holds a **single line** is not reordered: one
/// baseline has zero height, the overlap cannot be positive, and the page keeps content-stream
/// order. That is the conservative answer to a genuinely ambiguous picture — one short line beside
/// a long column is also exactly what a heading with a deep indent looks like — and the rule
/// declines rather than reordering on a coin flip.
pub const COLUMN_OVERLAP_DENOMINATOR: i64 = 2;

/// How many times the cut may alternate between axes before the rule stops and keeps what it has.
///
/// Thirty-two — a bound rather than trust in the recursion, because the depth is a property of
/// the *document*. A page contrived to hold that many distinct gap widths, each hiding the next
/// column, would otherwise decide how much stack this process uses, and running out of it is not
/// a failure mode an engine under a determinism contract is allowed to have.
///
/// Reaching it is not a truncation and drops nothing: the block below the bound comes out in
/// content-stream order, which is the same answer the rule gives any region it finds no gutter
/// in. Real layouts alternate two or three times; the ties-cut-together rule in
/// [`horizontal_cut`] is what keeps that true, since it is distinct gap *widths* and not lines
/// that consume depth.
pub const MAX_CUT_DEPTH: usize = 32;

const _: () = assert!(
    BLOCK_GUTTER_MIN < COLUMN_GUTTER_MIN,
    "a column gutter must be the wider of the two: if a block gap were wider than a column gap, \
     every paragraph break would out-rank the gutter beside it"
);
const _: () = assert!(
    UNKNOWN_ADVANCE_EXTENT < COLUMN_GUTTER_MIN,
    "the extent given to a run of unknown width must be smaller than a gutter, or two runs on one \
     baseline could be fused across a real column boundary by the fallback alone"
);
const _: () = assert!(
    COLUMN_OVERLAP_NUMERATOR > 0 && COLUMN_OVERLAP_NUMERATOR <= COLUMN_OVERLAP_DENOMINATOR,
    "the overlap fraction is a fraction: zero would drop the guard that keeps an indent from \
     reading as a column, and more than one is unsatisfiable"
);

// -------------------------------------------------------------------------------------------
// Input
// -------------------------------------------------------------------------------------------

/// One run's geometry, as this rule needs it.
///
/// Origin and advance, and nothing else. **No font size**, because no extent here is derived from
/// one; no text, because the rule does not read words; no ink box, because most runs have none
/// and a rule that needed one would work on a fraction of the corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunGeometry {
    /// Baseline origin x in centipoints, top-left system.
    pub x: i64,
    /// Baseline origin y in centipoints, top-left system.
    pub y: i64,
    /// Advance width in centipoints, or `None` when the document carries no width for it.
    pub advance: Option<i64>,
}

/// An indivisible unit of page content.
///
/// **A cut never runs through one.** The whole of decision 6 lives in this type: a table's runs
/// are one atom, so the column sweep sees a table as a single wide object and cannot split it
/// into a left column of first-column cells and a right column of second-column cells. A table's
/// internal order is content-stream order, unchanged, which is the order its cell text was
/// concatenated in — so a cell's text still matches the runs the cell names.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Atom {
    /// Left edge of the atom's horizontal extent.
    x0: i64,
    /// Right edge.
    x1: i64,
    /// Top edge of its vertical extent.
    y0: i64,
    /// Bottom edge.
    y1: i64,
    /// The runs it holds, by index into the page's stream-order run list, in that order.
    members: Vec<usize>,
    /// The stream index of its first member. The tiebreak that makes every sort here total.
    rank: usize,
}

// -------------------------------------------------------------------------------------------
// Entry point
// -------------------------------------------------------------------------------------------

/// The reading order of one page, as a permutation of its stream-order run indices.
///
/// `runs` is the page's runs in content-stream order. `tables` is the boxes of the tables the
/// detectors accepted on this page — the atoms of decision 6.
///
/// The return value is always a permutation of `0..runs.len()`: every run appears exactly once,
/// so no run can be dropped or duplicated by reordering. `the_result_is_always_a_permutation`
/// asserts it over generated pages, because "the sorter lost a run" is the one failure here that
/// would be silent in every artifact that did not also carry a count.
///
/// **The identity permutation is a real answer**, not a fallback state: it is what a single-column
/// page means under this rule.
pub fn order(runs: &[RunGeometry], tables: &[QuantRect]) -> Vec<usize> {
    if runs.len() < 2 {
        return (0..runs.len()).collect();
    }

    let atoms = atomize(runs, tables);
    let arranged = arrange(&atoms);

    let mut out = Vec::with_capacity(runs.len());
    for a in arranged {
        out.extend_from_slice(&atoms[a].members);
    }
    debug_assert_eq!(out.len(), runs.len(), "reordering is a permutation");
    out
}

/// Group the page's runs into atoms: one per table, one per loose run.
fn atomize(runs: &[RunGeometry], tables: &[QuantRect]) -> Vec<Atom> {
    // Which table, if any, claims each run. The detectors already guarantee accepted tables do
    // not overlap, so `position` is well defined; taking the first match anyway keeps this a
    // total function rather than one that relies on an invariant enforced elsewhere.
    let claim: Vec<Option<usize>> = runs
        .iter()
        .map(|r| {
            tables
                .iter()
                .position(|t| r.x >= t.x0 && r.x < t.x1 && r.y >= t.y0 && r.y < t.y1)
        })
        .collect();

    let mut atoms: Vec<Atom> = Vec::with_capacity(runs.len());
    // Where each table's atom landed, so the second run of a table joins the first rather than
    // starting another one. `None` until that table's first run is seen — a table whose box
    // encloses no run produces no atom at all, because an atom with no members would take a
    // position in the order without any text to put there.
    let mut table_atom: Vec<Option<usize>> = vec![None; tables.len()];

    for (i, run) in runs.iter().enumerate() {
        match claim[i] {
            Some(t) => match table_atom[t] {
                Some(a) => atoms[a].members.push(i),
                None => {
                    let r = tables[t];
                    table_atom[t] = Some(atoms.len());
                    atoms.push(Atom {
                        // The table's own box, which the document's rectangles or its text
                        // alignment produced. Not the union of the origins inside it: the box is
                        // the geometry the detector accepted, and the sweep should see the table
                        // the same way the detector did.
                        x0: r.x0,
                        x1: r.x1,
                        y0: r.y0,
                        y1: r.y1,
                        members: vec![i],
                        rank: i,
                    });
                }
            },
            None => {
                let (x0, x1) = extent(run);
                atoms.push(Atom {
                    x0,
                    x1,
                    // A baseline origin is a point. It gets no height here, because the only
                    // number available to give it one is the font size, and a box derived from a
                    // font size is the defect this engine refuses.
                    y0: run.y,
                    y1: run.y,
                    members: vec![i],
                    rank: i,
                });
            }
        }
    }
    atoms
}

/// One run's horizontal extent: measured where the document supplies a width, floored where it
/// does not.
///
/// A negative advance is normalised rather than dropped — it is what a right-to-left or
/// negatively scaled run looks like, and an interval with `x1 < x0` would silently break every
/// sweep downstream.
fn extent(run: &RunGeometry) -> (i64, i64) {
    let advance = run.advance.unwrap_or(UNKNOWN_ADVANCE_EXTENT);
    let far = run.x.saturating_add(advance);
    (run.x.min(far), run.x.max(far))
}

// -------------------------------------------------------------------------------------------
// The recursion
// -------------------------------------------------------------------------------------------

/// Order a block by cutting it into columns, or leave it exactly as it is.
///
/// **The `else` branch is the important one.** When no vertical gutter meets the rule, this
/// returns the identity — content-stream order, untouched. That is decision 10 and it is what
/// makes the change safe for the corpus: a single-column page is byte-identical before and after
/// this slice apart from the profile hash and the rule id.
fn arrange(atoms: &[Atom]) -> Vec<usize> {
    let index: Vec<usize> = (0..atoms.len()).collect();
    arrange_columns(atoms, &index, 0)
}

fn arrange_columns(atoms: &[Atom], block: &[usize], depth: usize) -> Vec<usize> {
    if block.len() < 2 || depth >= MAX_CUT_DEPTH {
        return block.to_vec();
    }
    match vertical_cut(atoms, block) {
        Some(bands) => bands
            .iter()
            .flat_map(|b| arrange_blocks(atoms, b, depth + 1))
            .collect(),
        None => block.to_vec(),
    }
}

/// Order one column band by cutting it into blocks top to bottom.
///
/// Reached only from inside an accepted vertical cut, which is why a horizontal gutter cannot
/// reorder a page that never split into columns.
fn arrange_blocks(atoms: &[Atom], band: &[usize], depth: usize) -> Vec<usize> {
    if band.len() < 2 || depth >= MAX_CUT_DEPTH {
        return band.to_vec();
    }
    match horizontal_cut(atoms, band) {
        // **Each block is offered back to the vertical cut**, and that is the whole reason the
        // horizontal cut takes only its widest gap. A full-width heading over two columns has no
        // gutter running past it, so the columns are invisible until the heading is peeled off;
        // peeling it off is this step, and the columns appear on the way back into
        // `arrange_columns`. A horizontal cut that took every gap at once would peel the page
        // apart line by line instead, and lines of a two-column page emitted top to bottom are
        // **row-major** — reading across the gutter, which is the interleaving this slice exists
        // to end.
        Some(blocks) => blocks
            .iter()
            .flat_map(|b| arrange_columns(atoms, b, depth + 1))
            .collect(),
        None => band.to_vec(),
    }
}

/// Split a block into column bands, left to right, or refuse.
///
/// Refuses when there is no gap wide enough, and — the guard that carries the rule — when the
/// bands a gap would produce do not sit beside each other vertically.
fn vertical_cut(atoms: &[Atom], block: &[usize]) -> Option<Vec<Vec<usize>>> {
    let mut order: Vec<usize> = block.to_vec();
    // A total order, so the sweep is a pure function of the geometry: left edge, then right edge,
    // then stream position. Two atoms with identical boxes still have distinct ranks.
    order.sort_by_key(|&a| (atoms[a].x0, atoms[a].x1, atoms[a].rank));

    let mut bands: Vec<Vec<usize>> = vec![vec![order[0]]];
    let mut edge = atoms[order[0]].x1;
    for &a in &order[1..] {
        if atoms[a].x0.saturating_sub(edge) >= COLUMN_GUTTER_MIN {
            bands.push(Vec::new());
        }
        bands.last_mut().expect("seeded above").push(a);
        edge = edge.max(atoms[a].x1);
    }

    if bands.len() < 2 {
        return None;
    }
    if !bands_sit_side_by_side(atoms, block, &bands) {
        return None;
    }
    Some(restore_stream_order(atoms, bands))
}

/// Put each group's members back into content-stream order.
///
/// **The sweep sorts to find the cut, not to produce the answer.** Without this the sorted order
/// leaks: a group the recursion then declines to cut is returned exactly as the sweep left it, so
/// a block with no columns in it comes back sorted by y — a global y-then-x sort of the page,
/// arrived at by accident, and the precise thing `docs/history/09-V1-MILESTONES.md` S5 decision 10 forbids.
/// It is worth being concrete about what that cost: on a real two-column booklet it turned pages
/// that were already column-major in the content stream into line-by-line row-major reading, which
/// is *worse* than doing nothing at all.
///
/// So a cut decides two things and only two: how the atoms are **grouped**, and what order the
/// **groups** go in. What order the atoms inside a group go in is not the cut's business, and the
/// answer is the one the rule gives everywhere it has no evidence — the order the document drew
/// them. `rank` is that order; for a table atom it is where the table's first run appeared.
fn restore_stream_order(atoms: &[Atom], mut groups: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    for g in &mut groups {
        g.sort_by_key(|&a| atoms[a].rank);
    }
    groups
}

/// Whether the bands a vertical gutter would produce are columns rather than stacked content.
///
/// Columns run beside each other: the shorter of two adjacent bands sits inside the taller one's
/// vertical range. A heading and the indented list beneath it are also separated by wide
/// whitespace on the x axis and are not columns, and this is the difference between the two
/// pictures — the heading shares none of the list's vertical range.
///
/// Every adjacent pair must pass, not the outermost pair and not an average — a three-band split
/// whose middle band is a caption stacked below the others is not three columns, and an average
/// would let it through.
fn bands_sit_side_by_side(atoms: &[Atom], block: &[usize], bands: &[Vec<usize>]) -> bool {
    let top = block.iter().map(|&a| atoms[a].y0).min();
    let bottom = block.iter().map(|&a| atoms[a].y1).max();
    let (Some(top), Some(bottom)) = (top, bottom) else {
        return false;
    };
    // Everything on one baseline. There is no vertical structure to read at all, so a gap on that
    // line is a word space, a tab stop, or a leader — never a column gutter. Refusing here is what
    // keeps a contents line of "Title ......... 3" from splitting a page in two.
    if bottom.saturating_sub(top) <= 0 {
        return false;
    }

    let span = |b: &Vec<usize>| -> (i64, i64) {
        let t = b.iter().map(|&a| atoms[a].y0).min().unwrap_or(top);
        let d = b.iter().map(|&a| atoms[a].y1).max().unwrap_or(bottom);
        (t, d)
    };

    bands.windows(2).all(|pair| {
        let (lt, lb) = span(&pair[0]);
        let (rt, rb) = span(&pair[1]);
        let overlap = lb.min(rb).saturating_sub(lt.max(rt));
        // Strictly positive, before any fraction is considered. Two bands that merely touch — a
        // heading whose baseline is the first line of the block beside it — overlap by zero, and
        // zero satisfies a fraction of zero. This is the arm that rejects them.
        if overlap <= 0 {
            return false;
        }
        let shorter = (lb.saturating_sub(lt)).min(rb.saturating_sub(rt));
        // Integer comparison of `overlap / shorter >= NUM / DEN`, cross-multiplied so no division
        // and no float enters the decision.
        overlap.saturating_mul(COLUMN_OVERLAP_DENOMINATOR)
            >= shorter.saturating_mul(COLUMN_OVERLAP_NUMERATOR)
    })
}

/// Split a column band into blocks, top to bottom, **at its most significant gap only**, or
/// refuse.
///
/// The same sweep as [`vertical_cut`] on the other axis, and with no overlap guard: blocks within
/// a column are stacked by construction, and requiring them to overlap horizontally would refuse
/// every centred heading over left-aligned text.
///
/// # Why only the widest gap
///
/// Because a horizontal cut destroys column structure and a vertical one recovers it, so the
/// order the two run in decides what a page means. Cut at every gap at once and a two-column
/// region becomes one block per line; emit those blocks top to bottom and the reader gets a line
/// of the left column, a line of the right, a line of the left — **row-major**, the interleaved
/// order this slice replaces, arrived at from the other direction. Taking only the widest gap
/// peels off whatever full-width thing was hiding the gutter — a banner heading, a rule, a
/// caption — and hands each half straight back to [`vertical_cut`], which then finds the columns.
///
/// Ties are cut together, and that is not a detail: a column of evenly-set body text has one gap
/// value repeated down its whole length, so its lines separate in a single step instead of one
/// recursion per line. What bounds the alternation is therefore the number of *distinct* gap
/// widths in a band, which on a real page is small.
fn horizontal_cut(atoms: &[Atom], band: &[usize]) -> Option<Vec<Vec<usize>>> {
    let mut order: Vec<usize> = band.to_vec();
    order.sort_by_key(|&a| (atoms[a].y0, atoms[a].y1, atoms[a].rank));

    // Pass one: the widest gap between consecutive atoms, and whether it clears the floor.
    let mut widest = 0i64;
    let mut edge = atoms[order[0]].y1;
    for &a in &order[1..] {
        widest = widest.max(atoms[a].y0.saturating_sub(edge));
        edge = edge.max(atoms[a].y1);
    }
    if widest < BLOCK_GUTTER_MIN {
        return None;
    }

    // Pass two: cut at that width and at every gap tied with it, and nowhere else.
    let mut blocks: Vec<Vec<usize>> = vec![vec![order[0]]];
    let mut edge = atoms[order[0]].y1;
    for &a in &order[1..] {
        if atoms[a].y0.saturating_sub(edge) >= widest {
            blocks.push(Vec::new());
        }
        blocks.last_mut().expect("seeded above").push(a);
        edge = edge.max(atoms[a].y1);
    }

    (blocks.len() >= 2).then(|| restore_stream_order(atoms, blocks))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(x: i64, y: i64) -> RunGeometry {
        RunGeometry {
            x,
            y,
            advance: None,
        }
    }

    fn wide(x: i64, y: i64, advance: i64) -> RunGeometry {
        RunGeometry {
            x,
            y,
            advance: Some(advance),
        }
    }

    /// The gate fixture's geometry, written out: the content stream draws the right column first.
    fn two_columns() -> Vec<RunGeometry> {
        vec![
            run(22_000, 6_000), // Right top
            run(22_000, 8_800), // Right bottom
            run(7_200, 6_000),  // Left top
            run(7_200, 8_800),  // Left bottom
        ]
    }

    #[test]
    fn two_columns_reads_column_major() {
        assert_eq!(order(&two_columns(), &[]), vec![2, 3, 0, 1]);
    }

    /// **The anti-cliff test, at the level the rule is written.**
    ///
    /// Two columns of N lines and of N+1 lines produce the same reading order, for every N from
    /// one line to well past the fifteen a line-count rule flips on. The fixture pair asserts the
    /// same thing over real PDF bytes; this asserts it over the rule itself, where a regression
    /// would be introduced.
    #[test]
    fn adding_a_line_never_changes_the_shape_of_the_order() {
        for n in 2..40i64 {
            for extra in [0, 1] {
                let mut runs = Vec::new();
                // The stream writes the right column first, as the gate fixture does.
                for i in 0..n {
                    runs.push(run(22_000, 6_000 + i * 1_200));
                }
                for i in 0..(n + extra) {
                    runs.push(run(7_200, 6_000 + i * 1_200));
                }
                let got = order(&runs, &[]);
                let left: Vec<usize> = (n as usize..runs.len()).collect();
                let right: Vec<usize> = (0..n as usize).collect();
                let want: Vec<usize> = left.into_iter().chain(right).collect();
                assert_eq!(
                    got, want,
                    "left column then right, at {n} lines with {extra} added — a rule that \
                     flipped on a line count would disagree with itself somewhere in this range"
                );
            }
        }
    }

    #[test]
    fn a_single_column_page_is_left_exactly_alone() {
        let runs = vec![run(7_200, 6_000), run(7_200, 9_600), run(7_200, 13_200)];
        assert_eq!(order(&runs, &[]), vec![0, 1, 2]);
    }

    /// The guard, on the picture it exists for.
    ///
    /// A heading and an indented list below it are separated by more than a gutter's width on the
    /// x axis. They are not columns, and the vertical ranges are what say so.
    #[test]
    fn an_indented_block_below_a_heading_is_not_a_column() {
        let runs = vec![
            run(7_200, 6_000),  // heading
            run(20_000, 9_600), // indented, far to the right
            run(20_000, 12_000),
        ];
        assert_eq!(
            order(&runs, &[]),
            vec![0, 1, 2],
            "stacked content shares no vertical range with the block above it, so no cut fires"
        );
    }

    /// Runs on one baseline are never split, however wide the gap.
    ///
    /// A leader line — `Title ....... 3` — is the shape that would otherwise cut a page in half.
    #[test]
    fn one_baseline_is_never_two_columns() {
        let runs = vec![run(7_200, 6_000), run(50_000, 6_000)];
        assert_eq!(order(&runs, &[]), vec![0, 1]);
    }

    /// A measured advance closes a gap a floored one would have left open.
    #[test]
    fn a_known_advance_is_used_and_can_refuse_a_cut() {
        // Two lines, each a wide run at x=7200 reaching past x=20000, and a second run at 20500.
        // With the real widths there is no gutter; the runs touch.
        let runs = vec![
            wide(7_200, 6_000, 13_500),
            wide(20_500, 6_000, 3_000),
            wide(7_200, 9_600, 13_500),
            wide(20_500, 9_600, 3_000),
        ];
        assert_eq!(
            order(&runs, &[]),
            vec![0, 1, 2, 3],
            "20500 - 20700 is under the floor, so these are one column"
        );

        // Move the second run right by a gutter's width and the same four runs are two columns.
        let split = vec![
            wide(7_200, 6_000, 13_500),
            wide(22_000, 6_000, 3_000),
            wide(7_200, 9_600, 13_500),
            wide(22_000, 9_600, 3_000),
        ];
        assert_eq!(order(&split, &[]), vec![0, 2, 1, 3]);
    }

    /// **Decision 6.** A table is one object to the sweep, and its runs keep stream order.
    #[test]
    fn a_table_is_never_cut_into_columns() {
        // A 2x2 grid whose two columns are further apart than the gutter floor. Loose, it would
        // split; inside a table box it must not.
        let runs = vec![
            run(7_200, 7_200),
            run(22_000, 7_200),
            run(7_200, 9_600),
            run(22_000, 9_600),
        ];
        let table = QuantRect {
            x0: 6_000,
            y0: 6_000,
            x1: 30_000,
            y1: 11_000,
        };
        assert_eq!(
            order(&runs, &[table]),
            vec![0, 1, 2, 3],
            "the table's runs are one atom in content-stream order — row-major, as its cell text \
             was concatenated"
        );

        // Same runs, no table: now the geometry really is two columns.
        assert_eq!(order(&runs, &[]), vec![0, 2, 1, 3]);
    }

    /// A table sitting beside a column of text is placed by its own box.
    #[test]
    fn a_table_takes_its_place_among_the_blocks_by_its_box() {
        let runs = vec![
            run(22_000, 7_200), // in the table, on the right
            run(22_000, 9_600), // in the table
            run(7_200, 7_200),  // loose text, left column
            run(7_200, 9_600),
        ];
        let table = QuantRect {
            x0: 20_000,
            y0: 6_000,
            x1: 32_000,
            y1: 11_000,
        };
        assert_eq!(order(&runs, &[table]), vec![2, 3, 0, 1]);
    }

    /// A table box enclosing no run contributes nothing rather than an empty slot.
    #[test]
    fn an_empty_table_box_adds_no_atom() {
        let runs = vec![run(7_200, 6_000), run(7_200, 9_600)];
        let elsewhere = QuantRect {
            x0: 40_000,
            y0: 40_000,
            x1: 50_000,
            y1: 50_000,
        };
        assert_eq!(order(&runs, &[elsewhere]), vec![0, 1]);
    }

    /// **The sweep's sort must not leak into the answer.**
    ///
    /// A cut fires on this page — there is a marginal element far to the right, exactly like the
    /// side tab on a real booklet page — and the main band that comes out of it holds no columns
    /// of its own. Those runs must come back in the order the document drew them. An earlier
    /// draft returned them in the order the sweep had sorted them into, which is a y-then-x sort
    /// of the page reached sideways: on a real two-column document it took pages that were
    /// already column-major in the content stream and read them line by line, across the gutter.
    #[test]
    fn a_block_the_rule_declines_to_cut_comes_back_in_stream_order() {
        // Two columns' worth of text written column-major by the stream, which a wide run at the
        // top bridges — so no gutter runs the height of the page and the body cannot be cut.
        let mut runs = vec![wide(7_200, 4_000, 40_000)]; // full-width banner
        for i in 0..4 {
            runs.push(run(7_200, 8_000 + i * 1_200)); // left column, drawn first
        }
        for i in 0..4 {
            runs.push(run(24_000, 8_000 + i * 1_200)); // right column, drawn second
        }
        runs.push(run(52_000, 8_000)); // the marginal element that lets a cut fire at all
        runs.push(run(52_000, 11_600));

        assert_eq!(
            order(&runs, &[]),
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            "the body is already column-major in the stream and the rule found no gutter in it, \
             so it must come back untouched — not sorted by baseline"
        );
    }

    #[test]
    fn three_columns_read_left_to_right() {
        let runs = vec![
            run(40_000, 6_000),
            run(40_000, 9_600),
            run(7_200, 6_000),
            run(7_200, 9_600),
            run(23_000, 6_000),
            run(23_000, 9_600),
        ];
        assert_eq!(order(&runs, &[]), vec![2, 3, 4, 5, 0, 1]);
    }

    /// Nothing is dropped, nothing is duplicated, whatever the geometry.
    #[test]
    fn the_result_is_always_a_permutation() {
        // A deterministic spread of pathological pages: coincident origins, negative advances,
        // origins at the saturation edge, and everything on one baseline.
        let cases: Vec<Vec<RunGeometry>> = vec![
            vec![],
            vec![run(0, 0)],
            vec![run(0, 0), run(0, 0), run(0, 0)],
            vec![run(i64::MAX - 1, 0), run(0, 0)],
            vec![wide(7_200, 6_000, -4_000), wide(22_000, 6_000, -4_000)],
            (0..50)
                .map(|i| run((i % 7) * 5_000, (i / 7) * 1_200))
                .collect(),
            (0..50).map(|i| run(i * 1_300, 6_000)).collect(),
        ];
        for (n, case) in cases.iter().enumerate() {
            let got = order(case, &[]);
            let mut seen = got.clone();
            seen.sort_unstable();
            assert_eq!(
                seen,
                (0..case.len()).collect::<Vec<_>>(),
                "case {n}: every run appears exactly once"
            );
        }
    }

    /// Ordering the same page twice gives the same answer, and it does not depend on the order
    /// the atoms happen to be built in beyond the declared tiebreak.
    #[test]
    fn the_order_is_a_pure_function_of_the_geometry() {
        let runs = two_columns();
        let first = order(&runs, &[]);
        for _ in 0..8 {
            assert_eq!(order(&runs, &[]), first);
        }
    }

    /// This module's own code: comments stripped, and everything from the test attribute down
    /// removed.
    ///
    /// Both exclusions are the lesson `thresholds::tests::the_garbled_reason_is_never_constructed`
    /// learned the hard way, and both were needed here on the first run. The prose above argues
    /// at length about the constants these guards ban, and the guards below list the banned
    /// tokens **as data** — so a scan of the raw file fires on the documentation of the rule and
    /// on the test that enforces it, rather than on any code. The scan resumes nowhere: the test
    /// module is last, and the two assertions that this function actually reached the constants
    /// are what stop it passing vacuously.
    fn rule_code() -> String {
        let src = include_str!("reading_order.rs");
        let end = src.find("#[cfg(test)]").unwrap_or(src.len());
        src[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// **Decision 3, asserted rather than trusted.** The two rules' gutter floors are separate
    /// bindings, so a change to one cannot silently change the other.
    #[test]
    fn the_two_gutter_floors_are_independent() {
        let code = rule_code();
        assert!(
            code.contains("pub const COLUMN_GUTTER_MIN: i64 = 1_200;"),
            "the scan did not reach this rule's own floor, so its absence below would prove \
             nothing"
        );
        assert!(
            !code.contains("unruled::"),
            "reading order must not read the unruled detector's constants: the gutter floors \
             hold the same number under two different rule ids, and sharing a binding would make \
             a table tuning pass reorder every multi-column document in the corpus"
        );
    }

    /// No number here comes from a font size, and nothing here counts anything.
    #[test]
    fn the_rule_reads_no_font_size_and_counts_nothing() {
        let code = rule_code();
        assert!(
            code.contains("UNKNOWN_ADVANCE_EXTENT"),
            "the scan did not reach the fallback extent, which is the one place a font size \
             would be reached for"
        );
        for banned in ["font_size", "min_lines", "line_count"] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the reading-order rule. A box from a type size is the P5 \
                 defect, and a line count is the cliff this slice replaces"
            );
        }
        // `bands.len() >= 2` is deliberately NOT banned here, and an earlier draft of this test
        // did ban it. Counting the groups a cut produced is asking "did it cut", not comparing a
        // tally to a calibrated threshold — and word-matching cannot tell those apart, which is
        // the instrument `thresholds.rs` already says is the wrong one. The claim that no line
        // count decides anything is behavioural, and
        // `adding_a_line_never_changes_the_shape_of_the_order` is where it is actually made.
    }
}
