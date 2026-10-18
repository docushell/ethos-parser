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
//! under `ethos_parser_core::LAYOUT_UNIT_RULE_V5`.
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
//! **And a line set smaller inside the line before it is that line's** (`line-units-v4`,
//! [`is_script`]): a subscript, a superscript or a footnote mark sits on a baseline of its own —
//! the `k` of `Qₖ`, the `2` of `CO₂`, a raised `1` — and was a line of its own, inside its unit's
//! box rather than below it, so a unit of its own, and the line around it went on as several. A
//! line set under [`SCRIPT_HEIGHT_TWENTIETHS`] twentieths of the line before it in reading order,
//! its baseline within half that line's height of that line's, lying inside that line's extent past
//! its start, is that line's. A smaller line stacked under a figure, opening where the figure
//! opens, is a line of its own, as are two lines of one size.
//!
//! **And a line is read where the reading order reads it, its words where the page set them
//! apart** (`line-units-v5`, [`lines_read`], [`spaced`]). On a page the reading order did not
//! divide, a line was every run on its baseline across the page, read where its first run was: a
//! paragraph whose baselines a byline or a caption in the next column shares was read where that
//! column was, and fell into several units, each matching nothing. Each line is now cut into its
//! pieces first, and a line as the units read it is the pieces of one line that follow each other
//! in reading order. And the text the list-marker clause and the first word are read from has a
//! space wherever two runs stand [`SPACE_GAP_TENTHS`] tenths of the line's height apart: a page that
//! sets its words by moving its pen draws no space between them, so a bullet set apart that way
//! opened no list item, and a line's first word was the whole line.
//!
//! **Where, never what.** A unit says these lines read as one piece of text; it is not the
//! author's paragraph, and nothing reads a role from it — decision #19's rule for `region` and
//! `block` holds here too. A table's runs and an `/Artifact`'s get no unit, nor does a line of
//! whitespace. The constants were measured, not chosen: on ParseBench's visual-grounding pages
//! half a line height scored above a third, four fifths and six fifths, and the list-marker and
//! heading clauses each added to it (`docs/measurements/parsebench/README.md`).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

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

/// A line set under this many twentieths of the line before it is that line's script, where it
/// also sits within and beside it (`line-units-v4`): a script is set at about two thirds of its
/// line, and two lines of one size set in different faces differ by less than three twentieths.
pub const SCRIPT_HEIGHT_TWENTIETHS: i64 = 17;

/// Two runs of a line whose boxes stand this many tenths of the line's height apart, or more, have a
/// space between them as the rule reads the line's text (`line-units-v5`): about a tenth of an em,
/// well under the narrowest word space, a quarter of one, and over the kerning between two pieces
/// of one word.
pub const SPACE_GAP_TENTHS: i64 = 1;

/// The most words a heading holds (`type-size-v7`, decision #77): a unit of heading lines past it,
/// counted as [`spaced`] reads its lines, is a lede set in display type, and its lines are read as
/// text. A news deck or a pull quote of fewer words reads as a title, and was measured so: at twenty
/// words, four formatting pages fell for decks of 21 to 28.
pub const HEADING_MAX_WORDS: usize = 30;

/// A line's box and baseline as the script join reads them: `x0, y0, x1, y1`, top-left system, and
/// its first run's baseline, centipoints.
pub(crate) type LineBox = (i64, i64, i64, i64, i64);

/// Whether the line boxed `line` is a script of the line boxed `base` before it in reading order
/// (`line-units-v4`): set clearly smaller, its baseline within half the base line's height of the
/// base line's, and lying inside the base line's extent past its start.
pub(crate) fn is_script(base: LineBox, line: LineBox) -> bool {
    let (base_height, height) = (base.3 - base.1, line.3 - line.1);
    20 * height < SCRIPT_HEIGHT_TWENTIETHS * base_height
        && 2 * (line.4 - base.4).abs() < base_height
        && line.0 > base.0
        && line.2 <= base.2
}

/// `lines` in reading order with each script joined to the line before it (`line-units-v4`):
/// `boxes` holds each line's own box, and a script is read against the box of the line it joins
/// as that line was drawn, never as the scripts already joined to it widen it.
pub(crate) fn join_scripts(lines: Vec<Vec<usize>>, boxes: &[Option<LineBox>]) -> Vec<Vec<usize>> {
    let mut out: Vec<(Vec<usize>, Option<LineBox>)> = Vec::with_capacity(lines.len());
    for (line, &own) in lines.into_iter().zip(boxes) {
        let script = out
            .last()
            .and_then(|(_, base)| *base)
            .zip(own)
            .is_some_and(|(base, own)| is_script(base, own));
        match out.last_mut() {
            Some((base, _)) if script => base.extend(line),
            _ => out.push((line, own)),
        }
    }
    out.into_iter().map(|(line, _)| line).collect()
}

/// The lines the rule reads (`line-units-v5`): `of` holds, for each piece in reading order, the
/// line it was cut from, and each line read is the range of the pieces of one line that follow each
/// other there. A baseline the reading order reads in two places — a paragraph's line and a
/// byline's beside it, in a column the reading order did not divide from it — is two lines, each
/// read where it is read.
pub(crate) fn lines_read(of: &[usize]) -> Vec<Range<usize>> {
    let mut start = 0;
    of.chunk_by(|a, b| a == b)
        .map(|line| {
            start += line.len();
            start - line.len()..start
        })
        .collect()
}

/// A line's text as the rule reads it (`line-units-v5`): `runs` holds each run's text and, where it
/// is inked and measured, its box's left and right edges, in the order they are read, and a space
/// is read wherever a box opens [`SPACE_GAP_TENTHS`] tenths of `height` or more past where the box
/// before it ended — a word gap the page set by moving its pen, and drew no space for.
pub(crate) fn spaced(runs: &[(&str, Option<(i64, i64)>)], height: i64) -> String {
    let mut text = String::new();
    let mut end: Option<i64> = None;
    for &(run, ink) in runs {
        if let (Some(end), Some((x0, _))) = (end, ink) {
            if height > 0 && 10 * (x0 - end) >= SPACE_GAP_TENTHS * height {
                text.push(' ');
            }
        }
        text.push_str(run);
        if let Some((_, x1)) = ink {
            end = Some(x1);
        }
    }
    text
}

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
    // `-v4`: each script joins the line it is set in.
    let boxes: Vec<Option<LineBox>> = lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|&i| &page.runs[i])
                .filter(|run| !run.text.trim().is_empty())
                .filter_map(|run| run.geometry.measured())
                .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
                .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
                .map(|r| (r.0, r.1, r.2, r.3, page.runs[line[0]].locator.origin_y))
        })
        .collect();
    let lines = join_scripts(lines, &boxes);
    let ink = |i: usize| {
        let run = &page.runs[i];
        (!run.text.trim().is_empty())
            .then(|| run.geometry.measured())
            .flatten()
            .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
    };
    let join = |a: Rect, b: Rect| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3));
    // `-v5`: each line is cut into its pieces before any unit is read, each piece placed by its
    // first run, and a line as the units read it is the pieces of one line that follow each other
    // in reading order.
    let mut cut: Vec<(usize, Option<Rect>, Vec<usize>)> = Vec::new();
    for (l, line) in lines.iter().enumerate() {
        let height = line
            .iter()
            .filter_map(|&i| ink(i))
            .reduce(join)
            .map_or(0, |r| r.3 - r.1);
        let placed: Vec<(i64, Option<Rect>)> = line
            .iter()
            .map(|&i| (page.runs[i].locator.origin_x, ink(i)))
            .collect();
        for (rect, at) in pieces_of(&placed, height) {
            cut.push((l, rect, at.iter().map(|&k| line[k]).collect()));
        }
    }
    cut.sort_unstable_by_key(|(_, _, runs)| runs.iter().copied().min());
    let read = lines_read(&cut.iter().map(|(l, _, _)| *l).collect::<Vec<_>>());
    let text = |runs: &[usize], height: i64| {
        let runs: Vec<(&str, Option<(i64, i64)>)> = runs
            .iter()
            .map(|&i| (page.runs[i].text.as_str(), ink(i).map(|b| (b.0, b.2))))
            .collect();
        spaced(&runs, height)
    };
    let unit_lines: Vec<UnitLine> = read
        .iter()
        .map(|range| {
            let members: BTreeSet<usize> = cut[range.clone()]
                .iter()
                .flat_map(|(_, _, runs)| runs.iter().copied())
                .collect();
            let line: Vec<usize> = lines[cut[range.start].0]
                .iter()
                .copied()
                .filter(|i| members.contains(i))
                .collect();
            let rect = line.iter().filter_map(|&i| ink(i)).reduce(join);
            let heading = line
                .iter()
                .map(|&i| &page.runs[i])
                .find(|run| !run.text.trim().is_empty())
                .filter(|run| run.inferred_heading)
                .map(|run| run.inferred_heading_level.unwrap_or(1));
            UnitLine {
                rect,
                heading,
                marker: opens_with_marker(&text(&line, rect.map_or(0, |r| r.3 - r.1))),
            }
        })
        .collect();
    let mut pieces = Vec::with_capacity(cut.len());
    let mut words = Vec::with_capacity(cut.len());
    for ((range, unit), line) in read.iter().zip(units(&unit_lines)).zip(&unit_lines) {
        let height = line.rect.map_or(0, |r| r.3 - r.1);
        for (_, rect, runs) in &cut[range.clone()] {
            let read = text(runs, height);
            pieces.push(Piece {
                rect: *rect,
                unit,
                first_word: rect.map_or(0, |r| first_word(&read, r.2 - r.0)),
                heading: line.heading.is_some(),
            });
            words.push(read.split_whitespace().count());
        }
    }
    let short = cut_after_short_pieces(&pieces);
    let assigned = cut_at_hard_breaks(&pieces, &short);
    // `type-size-v7` (decision #77): a heading unit past [`HEADING_MAX_WORDS`] is a lede, read as
    // text. A heading line never shares a unit with body text, so a unit is one or the other.
    let mut tally: BTreeMap<u32, (usize, bool)> = BTreeMap::new();
    for ((unit, piece), n) in assigned.iter().zip(&pieces).zip(&words) {
        let (count, heading) = tally.entry(*unit).or_insert((0, true));
        *count += n;
        *heading &= piece.heading;
    }
    for ((_, _, runs), unit) in cut.iter().zip(assigned) {
        let lede = tally
            .get(&unit)
            .is_some_and(|&(count, heading)| heading && count > HEADING_MAX_WORDS);
        for &i in runs {
            page.runs[i].layout_unit = Some(unit);
            if lede {
                page.runs[i].inferred_heading = false;
                page.runs[i].inferred_heading_level = None;
            }
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

    /// A line from `x0` to `x1` points, `height` points tall above its baseline at `baseline`.
    fn script_box(x0: i64, x1: i64, baseline: i64, height: i64) -> LineBox {
        let descent = height / 5;
        (
            x0 * 100,
            (baseline - height + descent) * 100,
            x1 * 100,
            (baseline + descent) * 100,
            baseline * 100,
        )
    }

    /// **A script is part of the line it is set in** (`line-units-v4`): smaller, its baseline
    /// within half the line's height, inside the line past its start — lowered or raised.
    #[test]
    fn a_line_set_smaller_inside_its_line_is_that_line_s() {
        let base = script_box(100, 500, 110, 10);
        assert!(is_script(base, script_box(200, 206, 113, 7)), "a subscript");
        assert!(
            is_script(base, script_box(300, 330, 106, 7)),
            "a superscript"
        );
        assert!(
            !is_script(base, script_box(300, 330, 105, 7)),
            "raised half the line or more"
        );
        assert!(
            !is_script(base, script_box(300, 330, 106, 9)),
            "not clearly smaller"
        );
        assert!(
            !is_script(base, script_box(100, 150, 113, 7)),
            "opening where the line opens: stacked under it, not set in it"
        );
        assert!(
            !is_script(base, script_box(480, 520, 113, 7)),
            "running past its end"
        );
    }

    /// **A script is read against its line as drawn**: two scripts of one line both join it, and a
    /// line that would be a script only of the taller box the pair makes of it stays a line, as does
    /// one running past the line's own end.
    #[test]
    fn scripts_are_read_against_their_line_as_drawn() {
        let lines = vec![vec![0], vec![1], vec![2], vec![3], vec![4]];
        let boxes = [
            Some(script_box(100, 300, 110, 10)),
            Some(script_box(200, 206, 113, 7)),
            Some(script_box(250, 290, 106, 7)),
            Some(script_box(150, 200, 112, 9)),
            Some(script_box(290, 310, 113, 7)),
        ];
        assert_eq!(
            join_scripts(lines, &boxes),
            [vec![0, 1, 2], vec![3], vec![4]],
            "nine points is no script of ten, though of the fourteen the scripts reach"
        );
        assert_eq!(
            join_scripts(
                vec![vec![0], vec![1]],
                &[None, Some(script_box(200, 206, 113, 7))]
            ),
            [vec![0], vec![1]],
            "an unmeasured line holds no script"
        );
    }

    /// **A line is read where the reading order reads it** (`line-units-v5`): the pieces of one line
    /// that follow each other in reading order are one line, and a line read in two places is two.
    #[test]
    fn a_line_read_in_two_places_is_two_lines() {
        // A paragraph's last two baselines (lines 0 and 1) are a byline's too, beside it, and the
        // byline is read first: each piece is a line of its own, read where it is read.
        assert_eq!(lines_read(&[0, 1, 2, 0, 1]), [0..1, 1..2, 2..3, 3..4, 4..5]);
        // A line's pieces read together stay one line.
        assert_eq!(lines_read(&[0, 0, 1, 1, 1, 0]), [0..2, 2..5, 5..6]);
        assert_eq!(lines_read(&[]), Vec::<Range<usize>>::new());
        // So the byline is a unit and the paragraph another.
        let lines = [
            body(112, 400, 480),
            body(124, 400, 480),
            body(100, 72, 300),
            body(112, 72, 300),
            body(124, 72, 300),
        ];
        assert_eq!(units(&lines), [1, 1, 2, 2, 2]);
    }

    /// **A word gap the page set by moving its pen reads as a space** (`line-units-v5`): from a
    /// tenth of the line's height, and not under it, so a bullet set apart by its position opens a
    /// list item and a line's first word is a word, while two kerned pieces of a word stay one.
    #[test]
    fn a_gap_a_tenth_of_the_line_high_reads_as_a_space() {
        // A line 1,000 high: a bullet 300 short of its item, then `ite` and `m` kerned 20 apart.
        let item = [
            ("•", Some((0, 400))),
            ("First", Some((700, 2_000))),
            ("ite", Some((2_300, 2_900))),
            ("m", Some((2_920, 3_200))),
        ];
        assert_eq!(spaced(&item, 1_000), "• First item");
        assert!(opens_with_marker(&spaced(&item, 1_000)));
        assert!(!opens_with_marker("•Firstitem"), "as written, no marker");
        // The boundary: a gap of a tenth is a space, a centipoint less is none.
        assert_eq!(
            spaced(&[("a", Some((0, 100))), ("b", Some((200, 300)))], 1_000),
            "a b"
        );
        assert_eq!(
            spaced(&[("a", Some((0, 100))), ("b", Some((199, 300)))], 1_000),
            "ab"
        );
        // A run with no box reads as written and moves nothing; a line with no height reads none.
        let drawn = [("a", Some((0, 100))), (" ", None), ("b", Some((150, 300)))];
        assert_eq!(spaced(&drawn, 1_000), "a b");
        assert_eq!(
            spaced(&[("a", Some((0, 100))), ("b", Some((900, 1_000)))], 0),
            "ab"
        );
        // `Cash flow` over 5,200: its first word and space are 2,888, where read as written,
        // `Cashflow`, its first word was the whole line and a character more.
        let entry = [("Cash", Some((0, 2_800))), ("flow", Some((3_100, 5_200)))];
        assert_eq!(first_word(&spaced(&entry, 1_000), 5_200), 2_888);
        assert_eq!(first_word("Cashflow", 5_200), 5_850);
    }
}
