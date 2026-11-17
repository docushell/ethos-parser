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

//! **Strikeout and underline from the lines a page draws through and under its runs** — part of
//! `ethos_parser_core::OBSERVATION_RULE_V5` (decision #154).
//!
//! A PDF has no strikeout or underline: a word processor draws each as a thin line, stroked or
//! painted as a thin rectangle, over or under the glyphs. A line marks a run where it is:
//!
//! 1. **thin and level** — a stroked horizontal segment, or a painted rectangle no taller than a
//!    fifth of the run's em (a highlight behind a word is a rectangle, and not thin);
//! 2. **in the run's band** — through it, between [`STRIKE_LOW`] and [`STRIKE_HIGH`] of an em above
//!    its baseline, for a strikeout; under it, from its baseline to [`UNDER_DEPTH`] of an em below,
//!    for an underline;
//! 3. **across the run** — over three quarters of the run's width;
//! 4. **under the text it marks along its length** — no stretch of it longer than an em, at its
//!    ends or between two runs, marks no run, so a table's rule under a row, a running head's rule
//!    under its title and its folio, marks nothing;
//! 5. **no box's edge** — no vertical line, stroked or painted, ends at either of its ends, so the
//!    bottom of a box around a field's value is no underline.
//!
//! A space between two runs one line marks carries the mark, so a struck phrase is one span. A
//! superscript or subscript is never struck: set off its line's baseline, its line's underline
//! crosses its middle.
//! **What it reads**: positions, ems and the page's own ink, nothing else. A mark is `Computed`;
//! the text is the document's, and the projections only wrap it in `~~` and `<u>`. A line through
//! part of a run marks none of it: a run is the grain, and no run is split.

use crate::nodes::TextRun;
use crate::stroke_ruled::{Rule, Upright};
use crate::tables::QuantRect;

/// A strikeout's band starts this many hundredths of an em above the baseline.
pub const STRIKE_LOW: i64 = 15;
/// And ends this many hundredths of an em above it.
pub const STRIKE_HIGH: i64 = 50;
/// An underline's band runs from the baseline down this many hundredths of an em.
pub const UNDER_DEPTH: i64 = 30;
/// A painted rectangle is a line where its height is at most this many hundredths of an em.
pub const MAX_THICKNESS: i64 = 20;
/// A line's end meets a vertical line within this many quanta: one point.
const END_TOLERANCE: i64 = 100;

/// A thin level line the page drew: its centre `y`, its ends, and its height where a painted
/// rectangle drew it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stroke {
    pub y: i64,
    pub x0: i64,
    pub x1: i64,
    pub height: Option<i64>,
}

/// A vertical line the page drew: a stroked segment, or a painted rectangle taller than wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Post {
    pub x: i64,
    pub y0: i64,
    pub y1: i64,
}

/// The page's lines a mark can be read from, and the posts that make one a box's edge.
#[derive(Debug, Default)]
pub(crate) struct Ink {
    pub strokes: Vec<Stroke>,
    pub posts: Vec<Post>,
}

impl Ink {
    /// The page's stroked level `rules` and vertical `uprights`, and the `rects` it painted in a
    /// colour of its own (`white` are those a fill painted white, which draw no line). A rule
    /// inside one of the `wide` bars a line stroked a point wide or more draws (decision #105) is
    /// as tall as its bar: a thick stroke painting a box is no thin line.
    pub(crate) fn of(
        rules: &[Rule],
        uprights: &[Upright],
        rects: &[QuantRect],
        white: &[QuantRect],
        wide: &[QuantRect],
    ) -> Self {
        let mut ink = Ink::default();
        ink.strokes.extend(rules.iter().map(|r| {
            let (x0, x1) = (r.x0.min(r.x1), r.x0.max(r.x1));
            let height = wide
                .iter()
                .find(|b| b.y0 <= r.y && r.y <= b.y1 && b.x0 <= x0 && x1 <= b.x1)
                .map(|b| b.y1 - b.y0);
            Stroke {
                y: r.y,
                x0,
                x1,
                height,
            }
        }));
        ink.posts.extend(uprights.iter().map(|u| Post {
            x: u.x,
            y0: u.y0.min(u.y1),
            y1: u.y0.max(u.y1),
        }));
        for r in rects.iter().filter(|r| !white.contains(r)) {
            let (w, h) = (r.x1 - r.x0, r.y1 - r.y0);
            if w > h {
                ink.strokes.push(Stroke {
                    y: (r.y0 + r.y1).div_euclid(2),
                    x0: r.x0,
                    x1: r.x1,
                    height: Some(h),
                });
            } else if h > w {
                ink.posts.push(Post {
                    x: (r.x0 + r.x1).div_euclid(2),
                    y0: r.y0,
                    y1: r.y1,
                });
            }
        }
        ink
    }
}

/// One run as the rule reads it, in content order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seen {
    /// Inked, with a measured box and a rendered em: the box's `x0` and `x1` and the baseline `y`,
    /// all top-left quanta.
    Placed { x0: i64, x1: i64, y: i64, em: i64 },
    /// Whitespace: it carries a mark both runs either side of it on its line carry.
    Space,
    /// Inked, with no box or no em to read: never marked.
    Unread,
}

/// What a run is marked: struck through, underlined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Marks {
    pub strike: bool,
    pub under: bool,
}

/// Each run's marks, in `seen`'s order.
pub(crate) fn marks(seen: &[Seen], ink: &Ink) -> Vec<Marks> {
    let mut out = vec![Marks::default(); seen.len()];
    for stroke in ink.strokes.iter().filter(|s| !boxed(s, &ink.posts)) {
        for strike in [true, false] {
            // The runs it lies across, in the band it marks.
            let across: Vec<(usize, i64, i64, i64)> = seen
                .iter()
                .enumerate()
                .filter_map(|(i, run)| match *run {
                    Seen::Placed { x0, x1, y, em }
                        if in_band(stroke, y, em, strike)
                            && stroke.height.is_none_or(|h| 100 * h <= MAX_THICKNESS * em)
                            && 4 * (stroke.x1.min(x1) - stroke.x0.max(x0)) >= 3 * (x1 - x0) =>
                    {
                        Some((i, x0, x1, em))
                    }
                    _ => None,
                })
                .collect();
            let Some(em) = across.iter().map(|a| a.3).max() else {
                continue;
            };
            // Under the text it marks along its length: no stretch of it longer than an em, at
            // its ends or between two runs, marks no run.
            let mut spans: Vec<(i64, i64)> = across.iter().map(|a| (a.1, a.2)).collect();
            spans.sort_unstable();
            let mut reach = stroke.x0;
            let mut gap = false;
            for (x0, x1) in spans {
                gap |= x0 - reach > em;
                reach = reach.max(x1);
            }
            if gap || stroke.x1 - reach > em {
                continue;
            }
            for &(i, ..) in &across {
                if strike {
                    out[i].strike = true;
                } else {
                    out[i].under = true;
                }
            }
        }
    }
    carry(seen, &mut out);
    out
}

/// Whether `stroke` is in the strikeout band (`strike`) or the underline band of a run on the
/// baseline `y` at `em`. Top-left quanta: above the baseline is a smaller `y`.
fn in_band(stroke: &Stroke, y: i64, em: i64, strike: bool) -> bool {
    if strike {
        y * 100 - STRIKE_HIGH * em <= stroke.y * 100 && stroke.y * 100 <= y * 100 - STRIKE_LOW * em
    } else {
        y <= stroke.y && stroke.y * 100 <= y * 100 + UNDER_DEPTH * em
    }
}

/// Whether a post ends at either end of `stroke`: a box's edge.
fn boxed(stroke: &Stroke, posts: &[Post]) -> bool {
    posts.iter().any(|p| {
        ((p.x - stroke.x0).abs() <= END_TOLERANCE || (p.x - stroke.x1).abs() <= END_TOLERANCE)
            && p.y0 - END_TOLERANCE <= stroke.y
            && stroke.y <= p.y1 + END_TOLERANCE
    })
}

/// A space whose runs either side on its line carry a mark carries it too.
fn carry(seen: &[Seen], out: &mut [Marks]) {
    let placed = |k: usize| match seen[k] {
        Seen::Placed { y, em, .. } => Some((y, em)),
        _ => None,
    };
    for i in 0..seen.len() {
        if seen[i] != Seen::Space {
            continue;
        }
        let before = (0..i).rev().find(|&k| seen[k] != Seen::Space);
        let after = (i + 1..seen.len()).find(|&k| seen[k] != Seen::Space);
        let (Some(b), Some(a)) = (before, after) else {
            continue;
        };
        let (Some((yb, em)), Some((ya, _))) = (placed(b), placed(a)) else {
            continue;
        };
        if 10 * (ya - yb).abs() > em {
            continue;
        }
        out[i].strike = out[b].strike && out[a].strike;
        out[i].under = out[b].under && out[a].under;
    }
}

/// Decision #154: mark the page's `runs` — in content order, `ems` beside them — struck through or
/// underlined where `ink` says.
pub(crate) fn assign(runs: &mut [TextRun], ems: &[Option<i64>], ink: &Ink) {
    if ink.strokes.is_empty() {
        return;
    }
    let seen: Vec<Seen> = runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            if run.text.trim().is_empty() {
                return Seen::Space;
            }
            match (run.geometry.measured(), ems.get(i).copied().flatten()) {
                (Some(b), Some(em)) if em > 0 && b.x1() > b.x0() => Seen::Placed {
                    x0: b.x0(),
                    x1: b.x1(),
                    y: run.locator.origin_y,
                    em,
                },
                _ => Seen::Unread,
            }
        })
        .collect();
    // A script is set off its line's baseline, so its line's underline can cross its own middle
    // (review): it is never struck, and the spaces beside it are carried again without it.
    let mut found = marks(&seen, ink);
    for (m, run) in found.iter_mut().zip(runs.iter()) {
        m.strike &= run.script.is_none();
    }
    carry(&seen, &mut found);
    for (run, m) in runs.iter_mut().zip(found) {
        run.strikeout = m.strike;
        run.underline = m.under;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run `w` points wide at `x` points on the baseline `y` points, set at a 10-point em.
    fn at(x: i64, y: i64, w: i64) -> Seen {
        Seen::Placed {
            x0: x * 100,
            x1: (x + w) * 100,
            y: y * 100,
            em: 1000,
        }
    }

    /// A stroked line at `y` points from `x0` to `x1` points.
    fn line(y: f64, x0: i64, x1: i64) -> Stroke {
        Stroke {
            y: (y * 100.0) as i64,
            x0: x0 * 100,
            x1: x1 * 100,
            height: None,
        }
    }

    fn ink(strokes: &[Stroke], posts: &[Post]) -> Ink {
        Ink {
            strokes: strokes.to_vec(),
            posts: posts.to_vec(),
        }
    }

    const STRUCK: Marks = Marks {
        strike: true,
        under: false,
    };
    const UNDER: Marks = Marks {
        strike: false,
        under: true,
    };
    const NONE: Marks = Marks {
        strike: false,
        under: false,
    };

    /// **A line through a run's middle strikes it; one just under its baseline underlines it**:
    /// three runs on a 10-point line at y = 100, the first struck 3 points up, the third
    /// underlined 1.5 points down, the second marked by neither.
    #[test]
    fn a_line_through_a_run_strikes_it_and_one_under_underlines_it() {
        let seen = [at(50, 100, 40), at(95, 100, 40), at(140, 100, 40)];
        let got = marks(
            &seen,
            &ink(&[line(97.0, 50, 90), line(101.5, 140, 180)], &[]),
        );
        assert_eq!(got, [STRUCK, NONE, UNDER]);
    }

    /// **A space between two struck runs is struck too**, so the phrase is one span; one beside
    /// a struck run and an unmarked one is not.
    #[test]
    fn a_space_between_two_struck_runs_is_struck() {
        let seen = [
            at(50, 100, 40),
            Seen::Space,
            at(93, 100, 40),
            Seen::Space,
            at(140, 100, 40),
        ];
        let got = marks(&seen, &ink(&[line(97.0, 50, 133)], &[]));
        assert_eq!(got, [STRUCK, STRUCK, STRUCK, NONE, NONE]);
        // Two struck runs on two lines: the space between them is on neither's line.
        let seen = [at(50, 100, 40), Seen::Space, at(50, 114, 40)];
        let got = marks(&seen, &ink(&[line(97.0, 50, 90), line(111.0, 50, 90)], &[]));
        assert_eq!(got, [STRUCK, NONE, STRUCK]);
    }

    /// **What is no mark**: a line over less than three quarters of the run; one reaching more
    /// than an em past the text it would mark, as a table's rule under a row does; the bottom of a
    /// box, a post at its end; a painted rectangle a highlight's height; a line above the strike
    /// band, and one below the underline band.
    #[test]
    fn lines_that_mark_nothing() {
        let run = [at(50, 100, 40)];
        let none = |strokes: &[Stroke], posts: &[Post]| marks(&run, &ink(strokes, posts));
        assert_eq!(
            none(&[line(97.0, 50, 75)], &[]),
            [NONE],
            "a third of the run"
        );
        assert_eq!(none(&[line(101.5, 20, 300)], &[]), [NONE], "a table's rule");
        let posts = [
            Post {
                x: 4800,
                y0: 9000,
                y1: 10200,
            },
            Post {
                x: 9200,
                y0: 9000,
                y1: 10200,
            },
        ];
        assert_eq!(
            none(&[line(101.5, 48, 92)], &posts),
            [NONE],
            "a box's bottom"
        );
        let highlight = Stroke {
            height: Some(1000),
            ..line(96.0, 50, 90)
        };
        assert_eq!(none(&[highlight], &[]), [NONE], "a highlight");
        assert_eq!(none(&[line(93.0, 50, 90)], &[]), [NONE], "above the band");
        assert_eq!(
            none(&[line(99.0, 50, 90)], &[]),
            [NONE],
            "under the strike band"
        );
        assert_eq!(none(&[line(104.0, 50, 90)], &[]), [NONE], "below the band");
        // The same lines, kept: a thin painted strike, and an underline just past the text.
        let thin = Stroke {
            height: Some(80),
            ..line(97.0, 50, 90)
        };
        assert_eq!(none(&[thin], &[]), [STRUCK]);
        assert_eq!(none(&[line(101.5, 45, 98)], &[]), [UNDER]);
    }

    /// **A rule under a whole line is no underline of the runs at its ends** (review): a running
    /// head's title at one end and its folio at the other, the rule between them under nothing;
    /// words a space apart along one underline are one underline.
    #[test]
    fn a_rule_whose_middle_marks_nothing_is_no_mark() {
        let seen = [at(72, 100, 60), at(500, 100, 40)];
        let got = marks(&seen, &ink(&[line(101.5, 72, 540)], &[]));
        assert_eq!(got, [NONE, NONE]);
        let words = [at(72, 100, 40), Seen::Space, at(115, 100, 40)];
        let got = marks(&words, &ink(&[line(101.5, 72, 155)], &[]));
        assert_eq!(got, [UNDER, UNDER, UNDER]);
    }

    /// **A line stroked thick is no mark** (review of ParseBench-2): a white letter on a square a
    /// stroked line 10 points wide paints is not struck through; the same line stroked thin is.
    #[test]
    fn a_line_stroked_thick_is_no_mark() {
        let rule = Rule {
            y: 9700,
            x0: 5000,
            x1: 9000,
        };
        let bar = QuantRect {
            x0: 5000,
            y0: 9200,
            x1: 9000,
            y1: 10200,
        };
        let run = [at(50, 100, 40)];
        let thick = Ink::of(&[rule], &[], &[], &[], &[bar]);
        assert_eq!(thick.strokes[0].height, Some(1000));
        assert_eq!(marks(&run, &thick), [NONE]);
        assert_eq!(marks(&run, &Ink::of(&[rule], &[], &[], &[], &[])), [STRUCK]);
    }
}
