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

//! **Figure regions: paths a page paints, read as one drawing** — decision #46, under
//! `ethos_parser_core::FIGURE_RULE_V1` (`docs/34-FIGURE-REGIONS-SCOPE.md`).
//!
//! 1. **Every painted path's box**, on the page and in the forms it draws
//!    ([`crate::content::Interpreter::painted`]), less any whose box covers [`BACKGROUND`] of the
//!    page or more — that is the page's background — and any holding a line of prose, as clause 3
//!    reads one: a panel text is set on, which joins no drawing set on it beside the text.
//! 2. **Clustered by touch**: two boxes are one cluster where the gap between them is at most
//!    [`GAP`] across and at most [`GAP`] down.
//! 3. **Kept** where the cluster holds [`MIN_PATHS`] paths or more, covers between
//!    [`MIN_AREA_PER_TEN_THOUSAND`] ten-thousandths and [`BACKGROUND`] of the page, has less than
//!    half its area inside any one table found on the page, and holds no line of prose — no
//!    baseline inside it carrying more than [`MAX_LINE_CHARS`] characters other than whitespace.
//!
//! Its box is the union of its paths' boxes, within the page. **Where, never what**: a region
//! claims no run and names no caption; the runs inside it stay in the text and in reading order.
//! The constants were measured on ParseBench's visual-grounding pages
//! (`docs/34-FIGURE-REGIONS-SCOPE.md` §6).

use crate::tables::QuantRect;

/// Two painted paths are one cluster where the gap between their boxes is at most this across and
/// at most this down: 3 points, in centipoints.
pub const GAP: i64 = 300;

/// A cluster of fewer painted paths is a rule or a panel, not a figure.
pub const MIN_PATHS: usize = 3;

/// A cluster covering less of the page than this many ten-thousandths is a speck: half a
/// thousandth, where a page's hairline rule drawn in pieces covers less and an icon set beside a
/// line of text more (`docs/34-FIGURE-REGIONS-SCOPE.md` §6.2).
pub const MIN_AREA_PER_TEN_THOUSAND: i128 = 5;

/// A path or a cluster covering this many tenths of the page or more is its background.
pub const BACKGROUND: i128 = 8;

/// The longest line a figure holds, in characters other than whitespace: a figure's labels are
/// short, and a shaded panel behind text is a text box.
pub const MAX_LINE_CHARS: usize = 60;

/// Baselines in one band this many centipoints tall read as one line for the prose clause.
const LINE_BAND: i64 = 150;

/// One figure region on a page, as the extract stage carries it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DetectedFigure {
    /// The union of its paths' boxes, within the page, in page space.
    pub rect: QuantRect,
}

/// The figure regions of one page, top to bottom and then left to right.
///
/// `painted` is every painted path's box in page space; `page` is the page's own box; `tables`
/// are the tables found on it; `runs` are its text runs, each its origin and its text.
pub(crate) fn detect(
    painted: &[QuantRect],
    page: QuantRect,
    tables: &[QuantRect],
    runs: &[(i64, i64, &str)],
) -> Vec<DetectedFigure> {
    let area = |r: &QuantRect| i128::from(r.x1 - r.x0) * i128::from(r.y1 - r.y0);
    let page_area = area(&page);
    let boxes: Vec<QuantRect> = painted
        .iter()
        .copied()
        .filter(|b| area(b) * 10 < page_area * BACKGROUND && !holds_prose(b, runs))
        .collect();

    // Swept in order of left edge, so the inner loop stops at the first box too far right.
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&i| (boxes[i].x0, i));
    let mut parent: Vec<usize> = (0..boxes.len()).collect();
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if boxes[j].x0 - boxes[i].x1 > GAP {
                break;
            }
            if boxes[j].y0 - boxes[i].y1 <= GAP && boxes[i].y0 - boxes[j].y1 <= GAP {
                let (a, b) = (
                    crate::tables::root(&mut parent, i),
                    crate::tables::root(&mut parent, j),
                );
                parent[a] = b;
            }
        }
    }

    let mut clusters: std::collections::BTreeMap<usize, (QuantRect, usize)> =
        std::collections::BTreeMap::new();
    for (i, b) in boxes.iter().enumerate() {
        let r = crate::tables::root(&mut parent, i);
        let c = clusters.entry(r).or_insert((*b, 0));
        c.0 = QuantRect {
            x0: c.0.x0.min(b.x0),
            y0: c.0.y0.min(b.y0),
            x1: c.0.x1.max(b.x1),
            y1: c.0.y1.max(b.y1),
        };
        c.1 += 1;
    }

    let mut out: Vec<DetectedFigure> = clusters
        .into_values()
        .filter(|(_, paths)| *paths >= MIN_PATHS)
        .filter_map(|(b, _)| {
            // Within the page: ink painted off it is not on it.
            let b = QuantRect {
                x0: b.x0.max(page.x0),
                y0: b.y0.max(page.y0),
                x1: b.x1.min(page.x1),
                y1: b.y1.min(page.y1),
            };
            (b.x0 < b.x1 && b.y0 < b.y1).then_some(b)
        })
        .filter(|b| {
            let a = area(b);
            a * 10_000 >= page_area * MIN_AREA_PER_TEN_THOUSAND && a * 10 < page_area * BACKGROUND
        })
        .filter(|b| {
            !tables.iter().any(|t| {
                let x = (b.x1.min(t.x1) - b.x0.max(t.x0)).max(0);
                let y = (b.y1.min(t.y1) - b.y0.max(t.y0)).max(0);
                2 * i128::from(x) * i128::from(y) >= area(b)
            })
        })
        .filter(|b| !holds_prose(b, runs))
        .map(|rect| DetectedFigure { rect })
        .collect();
    out.sort_by_key(|f| (f.rect.y0, f.rect.x0, f.rect.y1, f.rect.x1));
    out
}

/// Whether `b` holds a line of prose: a band of baselines inside it carrying more than
/// [`MAX_LINE_CHARS`] characters other than whitespace.
fn holds_prose(b: &QuantRect, runs: &[(i64, i64, &str)]) -> bool {
    let mut per_line: std::collections::BTreeMap<i64, usize> = std::collections::BTreeMap::new();
    for &(x, y, text) in runs {
        if x >= b.x0 && x <= b.x1 && y >= b.y0 && y <= b.y1 {
            *per_line.entry(y.div_euclid(LINE_BAND)).or_insert(0) +=
                text.chars().filter(|c| !c.is_whitespace()).count();
        }
    }
    per_line.values().any(|&chars| chars > MAX_LINE_CHARS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A box from `x0, y0` to `x1, y1` points.
    fn pt(x0: i64, y0: i64, x1: i64, y1: i64) -> QuantRect {
        QuantRect {
            x0: x0 * 100,
            y0: y0 * 100,
            x1: x1 * 100,
            y1: y1 * 100,
        }
    }

    /// A US Letter page.
    fn page() -> QuantRect {
        pt(0, 0, 612, 792)
    }

    /// Three bars of a chart, 2 points apart, over a 100 x 60 point area.
    fn chart() -> Vec<QuantRect> {
        vec![
            pt(100, 100, 130, 160),
            pt(132, 120, 162, 160),
            pt(164, 140, 200, 160),
        ]
    }

    #[test]
    fn three_touching_paths_are_one_region_boxed_by_their_union() {
        assert_eq!(
            detect(&chart(), page(), &[], &[]),
            [DetectedFigure {
                rect: pt(100, 100, 200, 160)
            }]
        );
    }

    /// The gap is where it is stated: 3 points joins and 3.01 does not, across and down.
    #[test]
    fn three_points_apart_joins_and_more_does_not() {
        // The first box reaches the others only across its 3-point gap, and the third only down.
        let joined = vec![
            pt(100, 100, 130, 160),
            pt(133, 100, 160, 160),
            pt(133, 163, 160, 170),
        ];
        assert_eq!(detect(&joined, page(), &[], &[]).len(), 1);
        let apart = vec![
            pt(100, 100, 130, 160),
            QuantRect {
                x0: 13301,
                ..pt(0, 100, 160, 160)
            },
            pt(100, 163, 130, 170),
        ];
        let found = detect(&apart, page(), &[], &[]);
        assert!(
            found.is_empty(),
            "a cluster of two and one of one: {found:?}"
        );
        let down = vec![
            pt(100, 100, 130, 160),
            pt(100, 163, 130, 170),
            pt(100, 173, 130, 180),
        ];
        assert_eq!(detect(&down, page(), &[], &[]).len(), 1);
        let below = vec![
            pt(100, 100, 130, 160),
            pt(100, 163, 130, 170),
            QuantRect {
                y0: 17301,
                ..pt(100, 0, 130, 180)
            },
        ];
        assert!(detect(&below, page(), &[], &[]).is_empty());
    }

    #[test]
    fn two_paths_are_a_rule_or_a_panel_and_three_a_figure() {
        assert!(detect(&chart()[..2], page(), &[], &[]).is_empty());
        assert_eq!(detect(&chart(), page(), &[], &[]).len(), 1);
    }

    #[test]
    fn a_speck_is_not_a_figure_and_half_a_thousandth_of_the_page_is() {
        // 612 x 792 points: half a thousandth is 242.4 square points.
        let small = vec![
            pt(100, 100, 107, 110),
            pt(108, 100, 115, 110),
            pt(116, 100, 122, 110),
        ];
        assert!(
            detect(&small, page(), &[], &[]).is_empty(),
            "22 x 10 points is under half a thousandth"
        );
        let enough = vec![
            pt(100, 100, 108, 110),
            pt(109, 100, 117, 110),
            pt(118, 100, 125, 110),
        ];
        assert_eq!(
            detect(&enough, page(), &[], &[]).len(),
            1,
            "25 x 10 points is over it"
        );
    }

    /// A path covering most of the page is its background, and does not join what it holds; a
    /// cluster covering most of the page is not a figure either.
    #[test]
    fn the_page_background_is_neither_a_path_nor_a_figure() {
        let mut painted = chart();
        painted.push(pt(0, 0, 612, 792));
        assert_eq!(
            detect(&painted, page(), &[], &[]),
            [DetectedFigure {
                rect: pt(100, 100, 200, 160)
            }]
        );
        // 590 x 690 points: 84% of the page.
        let large = vec![
            pt(10, 10, 300, 700),
            pt(302, 10, 600, 400),
            pt(302, 402, 600, 700),
        ];
        assert!(detect(&large, page(), &[], &[]).is_empty());
    }

    #[test]
    fn a_cluster_half_inside_a_table_is_the_table_s() {
        let table = [pt(100, 100, 150, 160)];
        assert!(
            detect(&chart(), page(), &table, &[]).is_empty(),
            "exactly half inside"
        );
        let less = [pt(100, 100, 149, 160)];
        assert_eq!(
            detect(&chart(), page(), &less, &[]).len(),
            1,
            "less than half inside"
        );
    }

    /// **A panel holding prose joins no drawing**: two charts set on one shaded panel are two
    /// figures where the panel holds a line of prose beside them, and one with the panel where it
    /// holds only labels.
    #[test]
    fn a_panel_holding_prose_joins_no_drawing() {
        let second: Vec<QuantRect> = chart()
            .iter()
            .map(|b| QuantRect {
                x0: b.x0 + 20000,
                x1: b.x1 + 20000,
                ..*b
            })
            .collect();
        let painted = [chart(), second, vec![pt(90, 90, 410, 300)]].concat();
        let (prose, label) = ("x".repeat(61), "x".repeat(60));
        let found = |text: &str| detect(&painted, page(), &[], &[(10000, 25000, text)]);
        assert_eq!(
            found(&prose),
            [
                DetectedFigure {
                    rect: pt(100, 100, 200, 160)
                },
                DetectedFigure {
                    rect: pt(300, 100, 400, 160)
                }
            ]
        );
        assert_eq!(
            found(&label),
            [DetectedFigure {
                rect: pt(90, 90, 410, 300)
            }]
        );
    }

    /// A line of more than sixty characters inside the box is prose, and its box a text panel.
    #[test]
    fn a_line_of_prose_inside_makes_it_a_text_box() {
        let (thirty, thirty_one) = ("a".repeat(30), "a".repeat(31));
        let spaced = format!(" {thirty}   ");
        let long = "x".repeat(80);
        let (thirty, thirty_one, spaced, long) = (
            thirty.as_str(),
            thirty_one.as_str(),
            spaced.as_str(),
            long.as_str(),
        );
        let kept = |runs: &[(i64, i64, &str)]| detect(&chart(), page(), &[], runs).len();
        assert_eq!(
            kept(&[(10500, 13000, thirty), (14000, 13000, thirty)]),
            1,
            "sixty"
        );
        assert_eq!(
            kept(&[(10500, 13000, spaced), (14000, 13000, spaced)]),
            1,
            "spaces"
        );
        assert_eq!(
            kept(&[(10500, 13000, thirty), (14000, 13000, thirty_one)]),
            0,
            "sixty-one"
        );
        assert_eq!(
            kept(&[(10500, 13000, thirty_one), (14000, 13300, thirty_one)]),
            1,
            "two lines of thirty-one"
        );
        assert_eq!(kept(&[(10500, 17000, long)]), 1, "a line below the box");
    }

    #[test]
    fn a_region_is_cut_to_the_page_and_one_off_it_is_none() {
        let edge = vec![
            pt(580, 100, 640, 160),
            pt(560, 100, 578, 160),
            pt(540, 100, 558, 160),
        ];
        assert_eq!(
            detect(&edge, page(), &[], &[]),
            [DetectedFigure {
                rect: pt(540, 100, 612, 160)
            }]
        );
        let off = vec![
            pt(700, 100, 760, 160),
            pt(762, 100, 780, 160),
            pt(782, 100, 800, 160),
        ];
        assert!(detect(&off, page(), &[], &[]).is_empty());
    }

    /// A horizontal rule is a path with no height, and still a path: an axis drawn as three
    /// lines meeting at a corner is a cluster.
    #[test]
    fn lines_with_no_area_are_paths() {
        let axes = vec![
            pt(100, 100, 100, 200),
            pt(100, 200, 250, 200),
            pt(250, 100, 250, 200),
        ];
        assert_eq!(
            detect(&axes, page(), &[], &[]),
            [DetectedFigure {
                rect: pt(100, 100, 250, 200)
            }]
        );
    }

    #[test]
    fn regions_come_top_to_bottom_then_left_to_right() {
        let mut painted = chart();
        painted.extend(chart().iter().map(|r| QuantRect {
            y0: r.y0 + 30000,
            y1: r.y1 + 30000,
            ..*r
        }));
        painted.extend(chart().iter().map(|r| QuantRect {
            x0: r.x0 + 25000,
            x1: r.x1 + 25000,
            ..*r
        }));
        let found: Vec<(i64, i64)> = detect(&painted, page(), &[], &[])
            .iter()
            .map(|f| (f.rect.x0 / 100, f.rect.y0 / 100))
            .collect();
        assert_eq!(found, [(100, 100), (350, 100), (100, 400)]);
    }
}
