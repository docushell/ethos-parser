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
//! under `ethos_parser_core::LAYOUT_UNIT_RULE_V1`.
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
    for (line, unit) in lines.iter().zip(units(&read)) {
        for &i in line {
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
}
