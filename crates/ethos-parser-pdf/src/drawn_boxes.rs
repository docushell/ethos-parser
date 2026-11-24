// Copyright 2026 docuShell
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

//! **Drawn boxes: a form's empty boxes, beside the run they are drawn by** — decision #168, under
//! `ethos_parser_core::DRAWN_BOX_RULE_V1`.
//!
//! 1. **A square outline**: a painted path of rectangles and axis-aligned lines
//!    ([`crate::content::Interpreter::painted`], a form's paths kept) [`MIN_SIDE`] to [`MAX_SIDE`]
//!    a side, its longer side at most a third longer than its shorter, not filled in a colour —
//!    a solid square is a bullet or a legend's swatch — and seen: a white fill alone, or a white or
//!    dashed outline, draws none. One path's fill and stroke, and an outline drawn twice, are one
//!    box.
//! 2. **Alone**: no other such square abuts it on its row or its column — a comb's cells, a grid's.
//! 3. **Empty**: no run's glyphs are set in it, and no other path a reader sees — a coloured fill
//!    and a placed image among them — crosses its inner area, a sixth of its side in from each edge
//!    (a path more than twice its size, a cell or a panel it stands in, is none). A tick or a cross makes it no box this rule reads: a glyph is
//!    a run, written where it lies, and a drawn mark is a path this rule does not read.
//! 4. **Beside a run**: of the baselines from [`LINE_ABOVE`] above the box's top to [`LINE_BELOW`]
//!    below its bottom, of runs within [`REACH`] of it across, the line's is the one nearest the
//!    box's bottom, and those runs within [`SAME_LINE`] of it are the line's. A run's extent is
//!    its ink's: its advance short of its leading and trailing whitespace, estimated by their share
//!    of its characters. Where one of them is drawn across the box, the box sits
//!    inside its text and binds nothing; otherwise the run whose near end is nearest it, within
//!    [`REACH`] to either side, takes it — before that run where it lies to the box's right, after
//!    it where it lies to its left.
//!
//! A page outlining more than [`ethos_parser_core::DRAWN_BOX_CANDIDATES_MAX`] squares reads none,
//! and a page whose search would pass [`STEPS_PER_PATH`] steps a path and [`STEPS_BASE`] more reads
//! none: each search reads only what lies near its box, so the rule's work is linear in what the
//! page draws. Extraction drops a box half or more of which a check box widget covers where the
//! widget states it on, or a state this engine cannot tell: the widget, not the outline under it,
//! says what the field holds. The constants are those a 2026-11-18 census of ParseBench-2's forms
//! measured on drawn forms, with an out-of-tree instrument.

use crate::tables::QuantRect;
use ethos_parser_core::BoxSide;

/// A box's shorter side is at least four points, in centipoints.
pub const MIN_SIDE: i64 = 400;
/// And its longer side at most sixteen.
pub const MAX_SIDE: i64 = 1600;
/// Two outlines whose edges lie within this of each other are one box: a fill and its stroke, or
/// a box drawn twice. 1.2 points.
pub const SAME: i64 = 120;
/// Two boxes on one row (or column) whose edges lie this close abut: 1.5 points.
pub const ABUT: i64 = 150;
/// A run whose glyphs' centre lies within this of a box is set inside it: a tick overshoots its box.
/// Three points.
pub const OVERSHOOT: i64 = 300;
/// A run is on a box's line where its baseline lies at most this above the box's top…
pub const LINE_ABOVE: i64 = 200;
/// …and at most this below its bottom: five points.
pub const LINE_BELOW: i64 = 500;
/// The farthest a box reaches to the run beside it: forty points.
pub const REACH: i64 = 4000;
/// Runs whose baselines lie within this of each other are on one line: 1.5 points.
pub const SAME_LINE: i64 = 150;
/// A run drawn across a box by more than this — its advance overlapping the box's width — sets
/// the box inside its text, and the box binds nothing: a point.
pub const ACROSS: i64 = 100;
/// The steps the search may take for each painted path, and [`STEPS_BASE`] more.
pub const STEPS_PER_PATH: usize = 64;
/// See [`STEPS_PER_PATH`].
pub const STEPS_BASE: usize = 100_000;

/// One run, as the rule reads it, in content-stream order.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Run {
    /// Where its ink starts along its line: its origin, past the advance its leading whitespace
    /// takes, estimated by its share of the run's characters.
    pub x: i64,
    /// Its baseline.
    pub y: i64,
    /// Where its ink ends: its advance's end, short of its trailing whitespace, estimated so.
    pub end: i64,
    /// Its glyphs' centre: across, its ink's extent's; down, its measured box's, or its baseline
    /// where it has none.
    pub centre: (i64, i64),
    /// Whether it holds text other than whitespace.
    pub words: bool,
}

/// One painted path, as the rule reads it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Path {
    /// Its box.
    pub rect: QuantRect,
    /// Whether it draws more than rectangles and axis-aligned lines.
    pub shaped: bool,
    /// Whether a fill painted it in a colour other than white: a square so filled is a bullet or a
    /// swatch, and inside a box it is a mark.
    pub coloured: bool,
    /// Whether no reader sees it: a white fill alone, or a white or dashed outline. A square drawn
    /// only so is no box.
    pub unseen: bool,
}

/// The empty boxes the page draws, each with the content-stream position of the run it is drawn
/// beside and the side; empty where the page draws none, or more than the rule reads.
pub(crate) fn detect(paths: &[Path], runs: &[Run]) -> Vec<(QuantRect, usize, BoxSide)> {
    let mut left = STEPS_PER_PATH
        .saturating_mul(paths.len().saturating_add(runs.len()))
        .saturating_add(STEPS_BASE);
    let mut charge = |n: usize| match left.checked_sub(n) {
        Some(rest) => {
            left = rest;
            true
        }
        None => false,
    };
    let square = |r: QuantRect| {
        let (w, h) = (r.x1 - r.x0, r.y1 - r.y0);
        w.min(h) >= MIN_SIDE && w.max(h) <= MAX_SIDE && 3 * w.max(h) <= 4 * w.min(h)
    };
    let same = |a: QuantRect, b: QuantRect| {
        (a.x0 - b.x0).abs() <= SAME
            && (a.y0 - b.y0).abs() <= SAME
            && (a.x1 - b.x1).abs() <= SAME
            && (a.y1 - b.y1).abs() <= SAME
    };
    // Every path by its left edge, for the searches below: those whose left edge lies in a span.
    let mut by_x: Vec<Path> = paths.to_vec();
    by_x.sort_by_key(|p| p.rect.x0);
    let window = |lo: i64, hi: i64| {
        let from = by_x.partition_point(|p| p.rect.x0 < lo);
        let to = by_x.partition_point(|p| p.rect.x0 < hi);
        &by_x[from..to.max(from)]
    };
    // 1. Square outlines a reader sees, one box for each: an outline drawn more often seen than
    //    unseen — a white fill under a stroke is seen, a white fill alone is not — and none filled
    //    in a colour.
    let mut outlines: Vec<QuantRect> = paths
        .iter()
        .filter(|p| !p.shaped && !p.coloured && !p.unseen && square(p.rect))
        .map(|p| p.rect)
        .collect();
    if outlines.len() > ethos_parser_core::DRAWN_BOX_CANDIDATES_MAX {
        return Vec::new();
    }
    outlines.sort_by_key(|r| (r.y0, r.x0, r.y1, r.x1));
    // Kept in the outlines' order, by top edge, so a box searches only those near its top.
    let mut boxes: Vec<QuantRect> = Vec::new();
    for r in outlines {
        let near = window(r.x0 - SAME, r.x0 + SAME + 1);
        let from = boxes.partition_point(|b| b.y0 < r.y0 - SAME);
        if !charge(near.len() + boxes.len() - from + 1) {
            return Vec::new();
        }
        if boxes[from..].iter().any(|&b| same(b, r)) {
            continue;
        }
        let (mut seen, mut unseen, mut solid) = (0usize, 0usize, false);
        for p in near.iter().filter(|p| same(p.rect, r)) {
            if p.coloured {
                solid = true;
            } else if p.unseen {
                unseen += 1;
            } else if !p.shaped {
                seen += 1;
            }
        }
        // A square filled in a colour is no box, and is ruled out here so that it makes no box
        // beside it a comb's cell: a form's grey number badge set against its box.
        if seen > unseen && !solid {
            boxes.push(r);
        }
    }
    // 2. Alone on its row and its column.
    let abuts = |a: QuantRect, b: QuantRect| {
        let row = (a.y0 - b.y0).abs() <= ABUT
            && (a.y1 - b.y1).abs() <= ABUT
            && ((a.x1 - b.x0).abs() <= ABUT || (b.x1 - a.x0).abs() <= ABUT);
        let column = (a.x0 - b.x0).abs() <= ABUT
            && (a.x1 - b.x1).abs() <= ABUT
            && ((a.y1 - b.y0).abs() <= ABUT || (b.y1 - a.y0).abs() <= ABUT);
        row || column
    };
    // A neighbour on its row or column has its top within a side and a gap of the box's own.
    let mut alone: Vec<QuantRect> = Vec::new();
    for (i, &a) in boxes.iter().enumerate() {
        let from = boxes.partition_point(|b| b.y0 < a.y0 - MAX_SIDE - ABUT);
        let to = boxes.partition_point(|b| b.y0 <= a.y1 + ABUT);
        if !charge(to.saturating_sub(from) + 1) {
            return Vec::new();
        }
        if !(from..to).any(|j| j != i && abuts(a, boxes[j])) {
            alone.push(a);
        }
    }
    if alone.is_empty() {
        return Vec::new();
    }
    // 3. Empty: no other path a reader sees crosses the box's inner area — a sixth of its side in
    //    from each edge, so a tick that overshoots its box still marks it, and no path more than
    //    twice its size, a cell or a panel it stands in, does — and no run's glyphs are set in it.
    let mut centres: Vec<(i64, i64)> = runs.iter().filter(|r| r.words).map(|r| r.centre).collect();
    centres.sort_unstable();
    let empty = |b: QuantRect, charge: &mut dyn FnMut(usize) -> bool| -> Option<bool> {
        let (w, h) = (b.x1 - b.x0, b.y1 - b.y0);
        let inner = QuantRect {
            x0: b.x0 + w / 6,
            y0: b.y0 + h / 6,
            x1: b.x1 - w / 6,
            y1: b.y1 - h / 6,
        };
        let near = window(b.x0 - 2 * w, inner.x1);
        if !charge(near.len() + 1) {
            return None;
        }
        let marked = near.iter().any(|p| {
            let r = p.rect;
            !p.unseen
                && r.x0 < inner.x1
                && r.x1 > inner.x0
                && r.y0 < inner.y1
                && r.y1 > inner.y0
                && r.x1 - r.x0 <= 2 * w
                && r.y1 - r.y0 <= 2 * h
                && (p.shaped || p.coloured || !same(r, b))
        });
        let lo = centres.partition_point(|c| c.0 < b.x0 - OVERSHOOT);
        let hi = centres.partition_point(|c| c.0 <= b.x1 + OVERSHOOT);
        if !charge(hi - lo + 1) {
            return None;
        }
        let run_inside = centres[lo..hi]
            .iter()
            .any(|c| c.1 >= b.y0 - OVERSHOOT && c.1 <= b.y1 + OVERSHOOT);
        Some(!marked && !run_inside)
    };
    // 4. Beside the nearest run of its line — the line whose baseline lies nearest the box's
    //    bottom — and none where a run of that line is drawn across it.
    let mut by_y: Vec<usize> = (0..runs.len()).filter(|&i| runs[i].words).collect();
    by_y.sort_by_key(|&i| (runs[i].y, i));
    let mut found: Vec<(QuantRect, usize, BoxSide)> = Vec::new();
    for b in alone {
        match empty(b, &mut charge) {
            None => return Vec::new(),
            Some(false) => continue,
            Some(true) => {}
        }
        let lo = by_y.partition_point(|&i| runs[i].y < b.y0 - LINE_ABOVE);
        let hi = by_y.partition_point(|&i| runs[i].y <= b.y1 + LINE_BELOW);
        if !charge(2 * (hi - lo) + 1) {
            return Vec::new();
        }
        // Only runs within reach say where its line is: another column's does not.
        let within = |i: usize| runs[i].end >= b.x0 - REACH && runs[i].x <= b.x1 + REACH;
        let Some(baseline) = by_y[lo..hi]
            .iter()
            .filter(|&&i| within(i))
            .map(|&i| runs[i].y)
            .min_by_key(|&y| ((y - b.y1).abs(), y))
        else {
            continue;
        };
        let line = || {
            by_y[lo..hi]
                .iter()
                .copied()
                .filter(move |&i| within(i) && (runs[i].y - baseline).abs() <= SAME_LINE)
        };
        if line().any(|i| runs[i].end.min(b.x1) - runs[i].x.max(b.x0) > ACROSS) {
            continue;
        }
        let mut best: Option<(i64, BoxSide, usize)> = None;
        for i in line() {
            let r = runs[i];
            let near = if r.x >= b.x1 - ACROSS {
                Some(((r.x - b.x1).max(0), BoxSide::Before))
            } else if r.end <= b.x0 + ACROSS {
                Some(((b.x0 - r.end).max(0), BoxSide::After))
            } else {
                None
            };
            // The nearest, every run of the line being within reach; at one distance, the run the
            // box is drawn before.
            if let Some((d, side)) = near {
                let rank = |d: i64, side: BoxSide| (d, side == BoxSide::After);
                if best.is_none_or(|(bd, bs, _)| rank(d, side) < rank(bd, bs)) {
                    best = Some((d, side, i));
                }
            }
        }
        if let Some((_, side, i)) = best {
            // One box a side to a run: the nearer, where two would be drawn by one.
            match found.iter_mut().find(|f| f.1 == i && f.2 == side) {
                Some(f) => {
                    let gap = |r: QuantRect| match side {
                        BoxSide::Before => runs[i].x - r.x1,
                        BoxSide::After => r.x0 - runs[i].end,
                    };
                    if gap(b) < gap(f.0) {
                        f.0 = b;
                    }
                }
                None => found.push((b, i, side)),
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> QuantRect {
        QuantRect { x0, y0, x1, y1 }
    }

    fn outline(x0: i64, y0: i64, side: i64) -> Path {
        Path {
            rect: rect(x0, y0, x0 + side, y0 + side),
            shaped: false,
            coloured: false,
            unseen: false,
        }
    }

    /// A run of words with its origin at `x` on baseline `y`, `width` long.
    fn run(x: i64, y: i64, width: i64) -> Run {
        Run {
            x,
            y,
            end: x + width,
            centre: (x + width / 2, y - 300),
            words: true,
        }
    }

    /// **A box drawn before its option is written before it, and one drawn after its caption
    /// after it** (decision #168): `☐ Yes` beside `Yes`, and `Signature ☐` where nothing follows.
    #[test]
    fn a_box_is_read_beside_the_nearest_run_of_its_line() {
        let paths = [outline(10_000, 20_000, 900), outline(40_000, 20_000, 900)];
        let runs = [run(11_200, 20_800, 2_000), run(30_000, 20_800, 8_000)];
        let found = detect(&paths, &runs);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].1, found[0].2), (0, BoxSide::Before));
        assert_eq!((found[1].1, found[1].2), (1, BoxSide::After));
    }

    /// A square filled in a colour is a bullet or a swatch; one with a tick set inside it, or a
    /// path drawn inside it, is marked; a comb's cells abut; a rectangle is no square; and a box
    /// with no run within reach on its line binds nothing (decision #168).
    #[test]
    fn only_an_empty_square_alone_on_its_line_is_a_box() {
        let label = [run(11_200, 20_800, 2_000)];
        let filled = [
            outline(10_000, 20_000, 900),
            Path {
                coloured: true,
                ..outline(10_000, 20_000, 900)
            },
        ];
        assert!(detect(&filled, &label).is_empty(), "a solid square");
        let mut ticked = label.to_vec();
        // A tick set off its label's baseline, its glyphs inside the box.
        ticked.push(run(10_100, 20_500, 600));
        assert!(
            detect(&[outline(10_000, 20_000, 900)], &ticked).is_empty(),
            "a tick inside"
        );
        let crossed = [
            outline(10_000, 20_000, 900),
            Path {
                rect: rect(10_100, 20_100, 10_800, 20_800),
                shaped: true,
                coloured: false,
                unseen: false,
            },
        ];
        assert!(detect(&crossed, &label).is_empty(), "a cross drawn inside");
        let comb = [outline(10_000, 20_000, 900), outline(10_900, 20_000, 900)];
        assert!(detect(&comb, &label).is_empty(), "a comb's cells");
        let wide = [Path {
            rect: rect(10_000, 20_000, 11_500, 20_900),
            ..outline(0, 0, 0)
        }];
        assert!(
            detect(&wide, &[run(11_700, 20_800, 2_000)]).is_empty(),
            "no square"
        );
        let far = [run(16_000, 20_800, 2_000)];
        assert!(
            detect(&[outline(10_000, 20_000, 900)], &far).is_empty(),
            "nothing within reach"
        );
        let other_line = [run(11_200, 23_000, 2_000)];
        assert!(
            detect(&[outline(10_000, 20_000, 900)], &other_line).is_empty(),
            "a run on another line"
        );
        assert_eq!(
            detect(&[outline(10_000, 20_000, 900)], &label).len(),
            1,
            "and the box itself"
        );
    }

    /// Its size, its column, its line and its side (decision #168): a square under four points or
    /// over sixteen is none; boxes stacked edge to edge are a grid's; a run above the box's line
    /// is not on it; and the nearer run takes the box, on whichever side.
    #[test]
    fn a_box_is_a_mid_sized_square_and_binds_the_nearer_run() {
        let label = [run(11_200, 20_800, 2_000)];
        assert!(
            detect(&[outline(10_000, 20_000, 300)], &label).is_empty(),
            "tiny"
        );
        assert!(
            detect(
                &[outline(10_000, 19_000, 2_000)],
                &[run(12_200, 20_800, 2_000)]
            )
            .is_empty(),
            "huge"
        );
        let column = [outline(10_000, 20_000, 900), outline(10_000, 20_900, 900)];
        assert!(detect(&column, &label).is_empty(), "a grid's column");
        let above = [run(11_200, 19_700, 2_000)];
        assert!(
            detect(&[outline(10_000, 20_000, 900)], &above).is_empty(),
            "a run above its line"
        );
        // A caption ending 3 points left of it, an option 20 points right: the caption's.
        let both = [run(4_000, 20_800, 5_700), run(12_900, 20_800, 2_000)];
        let found = detect(&[outline(10_000, 20_000, 900)], &both);
        assert_eq!((found[0].1, found[0].2), (0, BoxSide::After));
    }

    /// A box a reader sees (decision #168): a white fill alone draws none, a white fill under a
    /// stroke draws one, and a tick that overshoots its box still marks it.
    #[test]
    fn a_box_is_seen_and_a_mark_may_overshoot_it() {
        let label = [run(11_200, 20_800, 2_000)];
        let white = Path {
            unseen: true,
            ..outline(10_000, 20_000, 900)
        };
        assert!(
            detect(&[outline(10_000, 20_000, 900), white], &label).is_empty(),
            "a white fill alone"
        );
        let stroked = [
            outline(10_000, 20_000, 900),
            white,
            outline(10_000, 20_000, 900),
        ];
        assert_eq!(
            detect(&stroked, &label).len(),
            1,
            "a white fill under a stroke"
        );
        let overshoot = [
            outline(10_000, 20_000, 900),
            Path {
                rect: rect(9_700, 19_600, 10_800, 20_700),
                shaped: true,
                ..outline(0, 0, 0)
            },
        ];
        assert!(detect(&overshoot, &label).is_empty(), "a tick past its box");
    }

    /// The line is the box's own, and a run across it sets it in text (decision #168, review): a
    /// run on the line above stands nearer it than the run it is drawn before, and loses; and a
    /// run drawn across the box — `Nationality:        E` with a box in its gap — takes none.
    #[test]
    fn a_box_binds_on_its_own_line_and_never_inside_a_run() {
        let b = [outline(10_000, 20_000, 900)];
        let lines = [run(9_000, 19_850, 900), run(12_500, 20_850, 2_000)];
        let found = detect(&b, &lines);
        assert_eq!((found[0].1, found[0].2), (1, BoxSide::Before), "{found:?}");
        let across = [run(6_000, 20_800, 5_000), run(11_300, 20_800, 2_000)];
        assert!(detect(&b, &across).is_empty(), "inside a run");
    }

    /// Another column's run sets no line (decision #168, second review): a run 170 points away whose
    /// baseline lies at the box's bottom leaves the box with the option it stands before.
    #[test]
    fn another_column_sets_no_line() {
        let b = [outline(10_000, 20_000, 900)];
        let runs = [run(11_200, 20_650, 2_000), run(27_000, 20_900, 2_000)];
        let found = detect(&b, &runs);
        assert_eq!((found[0].1, found[0].2), (0, BoxSide::Before), "{found:?}");
    }

    /// Separated boxes are read up to the cap however many (decision #168, second review): the
    /// searches are near each box, so 480 of them pass the budget.
    #[test]
    fn many_separated_boxes_are_all_read() {
        let paths: Vec<Path> = (0..480)
            .map(|k| outline(1_000 + (k % 20) * 4_000, 1_000 + (k / 20) * 2_000, 900))
            .collect();
        let runs: Vec<Run> = (0..480)
            .map(|k| run(2_100 + (k % 20) * 4_000, 1_800 + (k / 20) * 2_000, 500))
            .collect();
        assert_eq!(detect(&paths, &runs).len(), 480);
    }

    /// A grey badge set against a box makes it no comb's cell (decision #168, third review): the
    /// badge is filled in a colour, no box itself, and the box beside it is read.
    #[test]
    fn a_filled_badge_against_a_box_leaves_it_alone() {
        let badge = outline(8_900, 20_000, 900);
        let paths = [
            badge,
            Path {
                coloured: true,
                ..badge
            },
            outline(9_800, 20_000, 900),
        ];
        let found = detect(&paths, &[run(11_000, 20_800, 2_000)]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0.x0, 9_800);
    }

    /// A page outlining more squares than the rule reads reads none (decision #168).
    #[test]
    fn a_page_of_squares_reads_none() {
        let mut paths: Vec<Path> = (0..=ethos_parser_core::DRAWN_BOX_CANDIDATES_MAX as i64)
            .map(|k| outline(1_000 + (k % 20) * 2_000, 1_000 + (k / 20) * 2_000, 900))
            .collect();
        // Enough other ink that the step budget alone would read them.
        paths.extend((0..20_000).map(|k| Path {
            rect: rect(50_000 + k, 50_000, 50_001 + k, 50_100),
            ..outline(0, 0, 0)
        }));
        let runs = [run(2_100, 1_800, 500)];
        assert!(detect(&paths, &runs).is_empty());
        assert_eq!(detect(&paths[..1], &runs).len(), 1);
    }
}
