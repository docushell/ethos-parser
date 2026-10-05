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

//! **Layout units: lines joined by their own spacing** — decision #38's paragraph-like blocks,
//! under `ethos_parser_core::LAYOUT_UNIT_RULE_V3`.
//!
//! A page's lines — the runs sharing one band, one `/Artifact` state and one baseline, the heading
//! rule's own unit — are read in reading order, and a line joins the unit above it when
//!
//! 1. its box sits no more than **half its own height** below the unit's box, or half above it
//!    where type is set tight;
//! 2. it overlaps the unit across at least **half the narrower width**, so a line of the next
//!    column is not a continuation;
//! 3. it is **the same kind**: body text, or an inferred heading of the same level, so a heading
//!    set on two lines is one unit and never shares one with the text below it;
//! 4. it does **not open with a list marker** — a bullet, or a number or a letter closed by `.` or
//!    `)` — since a list item starts a piece of its own.
//!
//! Then each line is read as its **pieces** — its runs, cut wherever the gap from one run's box to
//! the next is wider than the line's height — and a unit is **cut before a piece when the piece
//! before it ends short**: more than [`SHORT_LINE_HEIGHTS`] of the piece's own height left of the
//! unit's right edge. That is the last line of a paragraph set with no space after it, a list item
//! set without a marker, and a label set apart from its value on one line. The unit's right edge
//! is its widest piece's, so the cut is read once every line is in.
//!
//! **And a line its author broke early ends its unit** (`line-units-v3`): a unit of body text is
//! cut before a line whose first word, with the space before it, would have fit
//! [`HARD_BREAK_FIT`] times over between where the line above ends and the unit's right edge. The
//! measure did not break that line, so its author did — the end of a paragraph, an entry of a
//! contents list, an item of a list set without markers. A heading unit is not cut.
//!
//! **Where, never what.** A unit says these lines read as one piece of text; it is not the
//! author's paragraph, and nothing reads a role from it — decision #19's rule for `region` and
//! `block` holds here too. A table's runs and an `/Artifact`'s get no unit, nor does a line of
//! whitespace. The constants were measured, not chosen: on ParseBench's visual-grounding pages
//! half a line height scored above a third, four fifths and six fifths, and the list-marker and
//! heading clauses each added to it (`docs/measurements/parsebench/README.md`).

use std::collections::{BTreeMap, BTreeSet};

use crate::nodes::PageExtract;

/// A box: `x0, y0, x1, y1`, top-left system, centipoints.
type Rect = (i64, i64, i64, i64);

/// A piece ends short, and its unit is cut after it, when it stops more than this many of the next
/// piece's heights left of the unit's right edge.
pub const SHORT_LINE_HEIGHTS: i64 = 10;

/// One piece of a line as the cut reads it: its box, and the unit [`units`] gave its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    /// The union of its inked runs' measured boxes, or `None` where none was measured.
    pub rect: Option<Rect>,
    /// Its line's unit.
    pub unit: u32,
    /// The width of a space and its first word, read at the piece's own width per character, or
    /// 0 where it has no measured box or no text.
    pub first_word: i64,
    /// Its line is an inferred heading.
    pub heading: bool,
}

/// A line's first word must fit this many times over in the room the line above leaves before its
/// unit's right edge for the break between them to be the author's (`line-units-v3`).
pub const HARD_BREAK_FIT: i64 = 2;

/// One line as the rule reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitLine {
    /// The union of its inked runs' measured boxes — `x0, y0, x1, y1`, top-left system — or
    /// `None` where none was measured.
    pub rect: Option<Rect>,
    /// `None` for body text, else the level of the inferred heading it is.
    pub heading: Option<u8>,
    /// Its text opens with a list marker.
    pub marker: bool,
}

/// Each line's unit, 1-based in reading order. A line with no measured box is a unit of its own.
pub(crate) fn units(lines: &[UnitLine]) -> Vec<u32> {
    let mut out = Vec::with_capacity(lines.len());
    let mut open: Option<(Rect, Option<u8>)> = None;
    let mut n = 0u32;
    for line in lines {
        match (open, line.rect) {
            (Some((unit, kind)), Some(b))
                if kind == line.heading && !line.marker && continues(unit, b) =>
            {
                let joined = (
                    unit.0.min(b.0),
                    unit.1.min(b.1),
                    unit.2.max(b.2),
                    unit.3.max(b.3),
                );
                open = Some((joined, kind));
            }
            _ => {
                n = n.saturating_add(1);
                open = line.rect.map(|b| (b, line.heading));
            }
        }
        out.push(n);
    }
    out
}

/// Each piece's unit once a piece ending short closes its unit: `pieces` in reading order. The
/// pieces of one unit that follow each other in reading order are one stretch — a unit whose lines
/// cross a page's columns is a stretch per column — and a stretch is cut before a piece when the
/// piece before it ends more than [`SHORT_LINE_HEIGHTS`] of the piece's height left of the
/// stretch's widest piece.
pub(crate) fn cut_after_short_pieces(pieces: &[Piece]) -> Vec<u32> {
    let mut out = vec![0; pieces.len()];
    let mut n = 0u32;
    let mut start = 0;
    while start < pieces.len() {
        let unit = pieces[start].unit;
        let end = (start..pieces.len())
            .find(|&k| pieces[k].unit != unit)
            .unwrap_or(pieces.len());
        let edge = pieces[start..end]
            .iter()
            .filter_map(|p| p.rect)
            .map(|r| r.2)
            .max();
        for k in start..end {
            let short = k > start
                && match (pieces[k - 1].rect, pieces[k].rect, edge) {
                    (Some(before), Some(b), Some(edge)) => {
                        before.2 < edge - SHORT_LINE_HEIGHTS * (b.3 - b.1)
                    }
                    _ => false,
                };
            if k == start || short {
                n = n.saturating_add(1);
            }
            out[k] = n;
        }
        start = end;
    }
    out
}

/// Each piece's unit once a line its author broke early closes its unit (`line-units-v3`):
/// `pieces` in reading order and `units` their units under [`cut_after_short_pieces`]. The pieces
/// of one unit are cut before a piece of body text set below the middle of the piece before it
/// when its first word would have fit [`HARD_BREAK_FIT`] times over between where that piece ends
/// and the unit's widest piece's right edge.
pub(crate) fn cut_at_hard_breaks(pieces: &[Piece], units: &[u32]) -> Vec<u32> {
    let mut out = vec![0; pieces.len()];
    let mut n = 0u32;
    let mut start = 0;
    while start < pieces.len() {
        let unit = units[start];
        let end = (start..pieces.len())
            .find(|&k| units[k] != unit)
            .unwrap_or(pieces.len());
        let edge = pieces[start..end]
            .iter()
            .filter_map(|p| p.rect)
            .map(|r| r.2)
            .max();
        for k in start..end {
            let hard = k > start
                && !pieces[k].heading
                && pieces[k].first_word > 0
                && match (pieces[k - 1].rect, pieces[k].rect, edge) {
                    (Some(before), Some(b), Some(edge)) => {
                        2 * b.1 >= before.1 + before.3
                            && before.2 + HARD_BREAK_FIT * pieces[k].first_word <= edge
                    }
                    _ => false,
                };
            if k == start || hard {
                n = n.saturating_add(1);
            }
            out[k] = n;
        }
        start = end;
    }
    out
}

/// The width of a space and the first word of `text`, read at `width` per character of its trimmed
/// text: the estimate [`cut_at_hard_breaks`] reads, 0 for a piece with no text.
fn first_word(text: &str, width: i64) -> i64 {
    let text = text.trim();
    let chars = i64::try_from(text.chars().count()).unwrap_or(0);
    let word = i64::try_from(text.chars().take_while(|c| !c.is_whitespace()).count()).unwrap_or(0);
    if chars == 0 {
        return 0;
    }
    (1 + word) * width / chars
}

/// A line's pieces, left to right: `runs` is each run's origin and, where it is inked and measured,
/// its box; a new piece opens wherever the gap from one box to the next is wider than `height`,
/// and a run with no box joins the piece it follows. Each piece is its box and its runs' indices.
fn pieces_of(runs: &[(i64, Option<Rect>)], height: i64) -> Vec<(Option<Rect>, Vec<usize>)> {
    let mut order: Vec<usize> = (0..runs.len()).collect();
    order.sort_unstable_by_key(|&i| (runs[i].0, i));
    let mut out: Vec<(Option<Rect>, Vec<usize>)> = Vec::new();
    for i in order {
        let b = runs[i].1;
        match (out.last_mut(), b) {
            (Some((Some(open), _)), Some(b)) if b.0 - open.2 > height => {
                out.push((Some(b), vec![i]));
            }
            (Some((open, members)), _) => {
                if let Some(b) = b {
                    *open = Some(open.map_or(b, |o| {
                        (o.0.min(b.0), o.1.min(b.1), o.2.max(b.2), o.3.max(b.3))
                    }));
                }
                members.push(i);
            }
            (None, _) => out.push((b, vec![i])),
        }
    }
    out
}

/// Whether a line boxed `b` continues the unit boxed `unit`: clauses 1 and 2, in integers.
pub(crate) fn continues(unit: Rect, b: Rect) -> bool {
    let height = b.3 - b.1;
    let gap = b.1 - unit.3;
    let overlap = unit.2.min(b.2) - unit.0.max(b.0);
    let narrower = (unit.2 - unit.0).min(b.2 - b.0);
    height > 0 && 2 * gap <= height && 2 * gap >= -height && 2 * overlap >= narrower
}

/// Whether `text` opens with a list marker: a bullet, or a number of up to three digits or a
/// single letter, perhaps opened by `(` and closed by `.` or `)`, and then whitespace.
pub(crate) fn opens_with_marker(text: &str) -> bool {
    let text = text.trim_start();
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if "•·▪◦‣–—-*".contains(first) {
        return chars.next().is_some_and(char::is_whitespace);
    }
    let text = text.strip_prefix('(').unwrap_or(text);
    let label: String = text
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    let numbered = (1..=3).contains(&label.len()) && label.bytes().all(|b| b.is_ascii_digit());
    let lettered = label.len() == 1 && label.bytes().all(|b| b.is_ascii_alphabetic());
    let mut rest = text[label.len()..].chars();
    (numbered || lettered)
        && matches!(rest.next(), Some('.' | ')'))
        && rest.next().is_some_and(char::is_whitespace)
}

/// Set `layout_unit` on every run of `page` the rule gives a unit to.
pub(crate) fn assign(page: &mut PageExtract) {
    let owned: BTreeSet<usize> = page
        .tables
        .iter()
        .flat_map(|t| t.cells.iter())
        .flat_map(|c| c.run_indices.iter().copied())
        .collect();
    let mut by_line: BTreeMap<(Option<u32>, i64), Vec<usize>> = BTreeMap::new();
    for (i, run) in page.runs.iter().enumerate() {
        let artifact = matches!(
            run.structural,
            Some(ethos_parser_core::StructuralLocator::PdfArtifact(_))
        );
        if !owned.contains(&i) && !artifact {
            by_line
                .entry((run.region, run.locator.origin_y))
                .or_default()
                .push(i);
        }
    }
    // Reading order: `runs` is in its final order, so a line's first run places it. A line of
    // whitespace alone is no unit's.
    let mut lines: Vec<Vec<usize>> = by_line
        .into_values()
        .filter(|line| line.iter().any(|&i| !page.runs[i].text.trim().is_empty()))
        .collect();
    lines.sort_unstable_by_key(|line| line[0]);
    let read: Vec<UnitLine> = lines
        .iter()
        .map(|line| {
            let inked: Vec<&crate::nodes::TextRun> = line
                .iter()
                .map(|&i| &page.runs[i])
                .filter(|run| !run.text.trim().is_empty())
                .collect();
            let rect = inked
                .iter()
                .filter_map(|run| run.geometry.measured())
                .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
                .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)));
            let heading = inked
                .first()
                .filter(|run| run.inferred_heading)
                .map(|run| run.inferred_heading_level.unwrap_or(1));
            let text: String = line.iter().map(|&i| page.runs[i].text.as_str()).collect();
            UnitLine {
                rect,
                heading,
                marker: opens_with_marker(&text),
            }
        })
        .collect();
    // Each piece with the runs it holds, ordered by its first run: reading order.
    let mut placed_pieces: Vec<(usize, Piece, Vec<usize>)> = Vec::new();
    for ((line, unit), read) in lines.iter().zip(units(&read)).zip(&read) {
        let height = read.rect.map_or(0, |r| r.3 - r.1);
        let placed: Vec<(i64, Option<Rect>)> = line
            .iter()
            .map(|&i| {
                let run = &page.runs[i];
                let b = (!run.text.trim().is_empty())
                    .then(|| run.geometry.measured())
                    .flatten()
                    .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()));
                (run.locator.origin_x, b)
            })
            .collect();
        for (rect, at) in pieces_of(&placed, height) {
            let runs: Vec<usize> = at.iter().map(|&k| line[k]).collect();
            let first = runs.iter().copied().min().unwrap_or(usize::MAX);
            let text: String = runs.iter().map(|&i| page.runs[i].text.as_str()).collect();
            let piece = Piece {
                rect,
                unit,
                first_word: rect.map_or(0, |r| first_word(&text, r.2 - r.0)),
                heading: read.heading.is_some(),
            };
            placed_pieces.push((first, piece, runs));
        }
    }
    placed_pieces.sort_unstable_by_key(|(first, _, _)| *first);
    let pieces: Vec<Piece> = placed_pieces.iter().map(|(_, piece, _)| *piece).collect();
    let members: Vec<&Vec<usize>> = placed_pieces.iter().map(|(_, _, runs)| runs).collect();
    let short = cut_after_short_pieces(&pieces);
    for (runs, unit) in members.into_iter().zip(cut_at_hard_breaks(&pieces, &short)) {
        for &i in runs {
            page.runs[i].layout_unit = Some(unit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body line 10 points tall at `top`, from `x0` to `x1` points.
    fn body(top: i64, x0: i64, x1: i64) -> UnitLine {
        UnitLine {
            rect: Some((x0 * 100, top * 100, x1 * 100, (top + 10) * 100)),
            heading: None,
            marker: false,
        }
    }

    /// **Lines a leading apart are one unit; a paragraph's worth of space opens the next.** Lines
    /// 12 points apart leave 2 points between 10-point boxes, under half a line; 18 points apart
    /// leave 8, over it.
    #[test]
    fn lines_a_leading_apart_are_one_unit_and_a_gap_opens_the_next() {
        let lines = [
            body(100, 72, 500),
            body(112, 72, 500),
            body(124, 72, 300),
            body(142, 72, 500),
        ];
        assert_eq!(units(&lines), [1, 1, 1, 2]);
    }

    /// The boundary is where it is stated: a gap of exactly half the line's height joins.
    #[test]
    fn half_a_line_joins_and_more_does_not() {
        assert_eq!(units(&[body(100, 72, 500), body(115, 72, 500)]), [1, 1]);
        assert_eq!(units(&[body(100, 72, 500), body(116, 72, 500)]), [1, 2]);
    }

    /// **The next column is not a continuation**, however close its first line sits.
    #[test]
    fn a_line_beside_the_unit_is_not_part_of_it() {
        assert_eq!(units(&[body(100, 72, 290), body(112, 310, 530)]), [1, 2]);
    }

    /// **A heading never shares a unit with body text**, and a heading set on two lines is one.
    #[test]
    fn headings_and_body_text_keep_apart_and_a_two_line_heading_is_one() {
        let heading = |top| UnitLine {
            heading: Some(2),
            ..body(top, 72, 400)
        };
        let deeper = UnitLine {
            heading: Some(3),
            ..body(124, 72, 400)
        };
        let lines = [
            heading(100),
            heading(112),
            deeper,
            body(136, 72, 500),
            body(148, 72, 500),
        ];
        assert_eq!(units(&lines), [1, 1, 2, 3, 3]);
    }

    /// **A list item starts a unit**, set however tight under the one above.
    #[test]
    fn a_list_marker_opens_a_unit() {
        let item = UnitLine {
            marker: true,
            ..body(112, 72, 500)
        };
        assert_eq!(
            units(&[body(100, 72, 500), item, body(124, 72, 500)]),
            [1, 2, 2]
        );
    }

    /// One piece, from `x0` to `x1` points at `top`, 10 points tall, in unit `unit`.
    fn piece(top: i64, x0: i64, x1: i64, unit: u32) -> Piece {
        Piece {
            rect: Some((x0 * 100, top * 100, x1 * 100, (top + 10) * 100)),
            unit,
            first_word: 0,
            heading: false,
        }
    }

    /// [`piece`], with a first word `word` points wide, its space included.
    fn worded(top: i64, x0: i64, x1: i64, word: i64, unit: u32) -> Piece {
        Piece {
            first_word: word * 100,
            ..piece(top, x0, x1, unit)
        }
    }

    /// **A piece ending short closes its unit**: ten of the next piece's heights left of the unit's
    /// widest piece is no cut, a point more is — the last line of a paragraph set with no space
    /// after it, a list item set without a marker, a label set apart from its value.
    #[test]
    fn a_piece_ending_short_closes_its_unit() {
        // The widest piece ends at 500 points, and each is 10 points tall.
        for (end, want) in [(400, [1, 1, 1]), (399, [1, 2, 2])] {
            let pieces = [
                piece(100, 72, end, 1),
                piece(112, 72, 500, 1),
                piece(124, 72, 500, 1),
            ];
            assert_eq!(
                cut_after_short_pieces(&pieces),
                want,
                "a first piece ending at {end}"
            );
        }
        // A short last piece closes nothing after it, and two units stay two.
        let pieces = [
            piece(100, 72, 500, 1),
            piece(112, 72, 200, 1),
            piece(142, 72, 500, 2),
            piece(154, 72, 100, 2),
        ];
        assert_eq!(cut_after_short_pieces(&pieces), [1, 1, 2, 2]);
        // A label and its value on one line: the label ends short of the value's right edge.
        let pieces = [piece(100, 72, 120, 1), piece(100, 400, 500, 1)];
        assert_eq!(cut_after_short_pieces(&pieces), [1, 2]);
    }

    /// **A unit whose lines cross a page's columns is a stretch per column**: the second column's
    /// pieces, read after the first column's, do not make the first column's lines end short.
    #[test]
    fn a_unit_across_columns_is_cut_per_stretch() {
        let pieces = [
            piece(100, 72, 300, 1),
            piece(112, 72, 300, 1),
            piece(124, 72, 300, 2),
            piece(100, 400, 700, 1),
            piece(112, 400, 700, 1),
        ];
        assert_eq!(cut_after_short_pieces(&pieces), [1, 1, 2, 3, 3]);
    }

    /// **A line is cut into pieces where its ink stands apart**: a gap wider than the line's height
    /// opens a piece, a gap of exactly it does not, and a space joins the piece it follows.
    #[test]
    fn a_line_is_cut_into_pieces_where_its_ink_stands_apart() {
        let inked = |x0: i64, x1: i64| (x0 * 100, Some((x0 * 100, 1000, x1 * 100, 2000)));
        let space = |x: i64| (x * 100, None);
        // 10-point line height: 10 points apart is one piece, 11 is two.
        let runs = [inked(72, 100), space(100), inked(110, 150), inked(161, 200)];
        let pieces = pieces_of(&runs, 1000);
        let members: Vec<Vec<usize>> = pieces.iter().map(|(_, m)| m.clone()).collect();
        assert_eq!(members, [vec![0, 1, 2], vec![3]]);
        assert_eq!(pieces[0].0, Some((7200, 1000, 15000, 2000)));
    }

    /// A line with no measured box is a unit of its own and joins nothing after it.
    #[test]
    fn an_unmeasured_line_stands_alone() {
        let unmeasured = UnitLine {
            rect: None,
            ..body(112, 72, 500)
        };
        assert_eq!(
            units(&[body(100, 72, 500), unmeasured, body(124, 72, 500)]),
            [1, 2, 3]
        );
    }

    #[test]
    fn list_markers_are_bullets_numbers_and_letters_closed_and_spaced() {
        for marker in [
            "• Item",
            "- Item",
            "1. Item",
            "12) Item",
            "(a) Item",
            "b. Item",
            "  3.\tItem",
        ] {
            assert!(opens_with_marker(marker), "{marker:?}");
        }
        for prose in [
            "1999 was a year",
            "U.S. policy",
            "3.5 million",
            "Item",
            "a.m. meeting",
            "-5 degrees",
            "",
        ] {
            assert!(!opens_with_marker(prose), "{prose:?}");
        }
    }

    /// **A line its author broke early closes its unit** (`line-units-v3`): where its first word
    /// would have fit twice over in the room the line above leaves before the unit's right edge.
    /// The boundary is where it is stated.
    #[test]
    fn a_line_broken_early_closes_its_unit() {
        // The unit's right edge is 500 points and the first line ends at 400: 100 points of room.
        for (word, want) in [(50, [1, 2, 2]), (51, [1, 1, 1])] {
            let pieces = [
                piece(100, 72, 400, 1),
                worded(112, 72, 500, word, 1),
                worded(124, 72, 500, 30, 1),
            ];
            assert_eq!(
                cut_at_hard_breaks(&pieces, &[1, 1, 1]),
                want,
                "a first word of {word} points"
            );
        }
    }

    /// A contents list set without markers: each entry ends short of the widest, and the next
    /// entry's first word fits after it, so each is a unit of its own.
    #[test]
    fn a_list_set_without_markers_is_a_unit_per_entry() {
        let pieces = [
            worded(100, 72, 250, 20, 1),
            worded(112, 72, 260, 20, 1),
            worded(124, 72, 300, 20, 1),
        ];
        assert_eq!(cut_at_hard_breaks(&pieces, &[1, 1, 1]), [1, 2, 3]);
    }

    /// A heading set on two lines stays one, however short its first line.
    #[test]
    fn a_heading_is_not_cut() {
        let mut pieces = [worded(100, 72, 200, 20, 1), worded(112, 72, 400, 20, 1)];
        for p in &mut pieces {
            p.heading = true;
        }
        assert_eq!(cut_at_hard_breaks(&pieces, &[1, 1]), [1, 1]);
    }

    /// Pieces on one line are the short-piece cut's to divide, not this one's: the second piece
    /// does not sit below the middle of the first.
    #[test]
    fn pieces_on_one_line_are_not_cut_here() {
        let pieces = [worded(100, 72, 120, 20, 1), worded(104, 400, 500, 20, 1)];
        assert_eq!(cut_at_hard_breaks(&pieces, &[1, 1]), [1, 1]);
        let below = [worded(100, 72, 120, 20, 1), worded(105, 400, 500, 20, 1)];
        assert_eq!(cut_at_hard_breaks(&below, &[1, 1]), [1, 2]);
    }

    /// Units stay apart, and a piece with no measured first word cuts nothing.
    #[test]
    fn units_stay_apart_and_an_unmeasured_word_cuts_nothing() {
        let pieces = [
            worded(100, 72, 300, 20, 1),
            worded(112, 72, 500, 0, 1),
            worded(142, 72, 300, 20, 2),
            worded(154, 72, 500, 20, 2),
        ];
        assert_eq!(cut_at_hard_breaks(&pieces, &[1, 1, 2, 2]), [1, 1, 2, 3]);
    }

    /// The first word's width is read at the piece's own width per character, its space included.
    #[test]
    fn the_first_word_is_read_at_the_piece_s_own_pitch() {
        // Eleven characters over 1,100 centipoints: "Alpha" and its space are 600.
        assert_eq!(first_word("Alpha Gamma", 1_100), 600);
        assert_eq!(first_word("  Alpha Gamma  ", 1_100), 600);
        assert_eq!(first_word("", 1_100), 0);
    }
}
