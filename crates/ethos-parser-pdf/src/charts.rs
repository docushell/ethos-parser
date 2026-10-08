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

//! **Tables from a bar chart's printed labels** — decision #42, and
//! `docs/32-CHART-LABELS-SCOPE.md` §2, which is the rule this file is.
//!
//! A bar chart that prints its numbers states a table: each bar's category set at its base, its
//! value printed on it or at its end, its series named in the legend beside a swatch of its
//! colour. This rule reads that table back, and nothing else: **a value is never read off a bar's
//! length**, so a bar that carries no number leaves its cell empty.
//!
//! # What it reads
//!
//! **Bars**: rectangles the page filled, in a colour this reader knows and not white, a point or
//! more each way. **Columns**: bars of one width set end to end, each a colour the one before it
//! is not — a stack, its segments listed from the base out. **A chart**: two or more columns on one base, of one width, side by side;
//! its categories are the columns, one each — or, where the columns' colours repeat with a period
//! of two or more and the gaps inside a period are narrower than between them, each period. Its
//! series are the stack's colours, the period's positions, or the one colour it has.
//!
//! **Labels**: the upright runs no other rule's table holds, joined along a baseline wherever less
//! than half a line height separates them.
//!
//! # What makes a cell
//!
//! - **A value** is the one number whose centre lies on its bar's segment — or, for a bar of one
//!   segment, the one just beyond its end. A number two cells could read fills neither.
//! - **A category** is the one label nearest the base under its column or group (vertical) or
//!   beside it (horizontal), with the lines a long label wraps onto. Never a number but a year,
//!   never a label that starts with a value, never a line of prose, never one set on a bar.
//! - **A series** is named by the one label beside a legend swatch of its colour, near this chart
//!   and nearer it than any other bars of that colour; a chart of one colour without one is named
//!   by the title just above it, where that title is a line of its own.
//!
//! # What makes it a table and not a coincidence
//!
//! 1. **The numbers agree with the bars in order**: a bar a tenth longer never carries a smaller
//!    number, and one a third longer carries a larger one. A chart is rarely drawn exactly to
//!    scale, so the order is what is asked of it, not a fitted line.
//! 2. **Labels are systematic**: a series keeps its numbers only where six bars in ten of it carry
//!    one. A line's labels drawn over two bars of seven are not those bars' values.
//! 3. **Nothing is read twice**: no label is two categories, no two categories read alike, and no
//!    run is in two tables.
//!
//! # Fabrication is impossible here, not merely avoided
//!
//! A cell holds exactly its label's runs, its text their text concatenated; a cell with nothing
//! printed is not emitted. The table's cells are its labels where the page set them — not boxes
//! tiling a grid — so the lattice cross-check is `NotApplicable`, with that reason.

use ethos_parser_core::{CheckStatus, EngineError, IdAllocator, IdKind, LocatorCheck};

use crate::tables::{DetectedCell, DetectedTable, QuantRect};
use crate::text_state::Fill;
use crate::tracks::TrackRun;

/// Two edges this close, in centipoints, are one edge: half a point.
pub const SAME_EDGE: i64 = 50;

/// Bars whose bases lie this close, in centipoints, stand on one base.
pub const SAME_BASE: i64 = 60;

/// The smallest bar, each way, in centipoints: one point.
pub const MIN_BAR: i64 = 100;

/// The fewest columns a chart may have, and the fewest rows its table may have.
pub const MIN_BARS: usize = 2;

/// How many bars in ten of a series must carry a number for the series to keep any.
pub const LABELLED_IN_TEN: usize = 6;

/// One filled rectangle, in page space, and the colour it was filled with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bar {
    /// The rectangle, top-left system, centipoints.
    pub rect: QuantRect,
    /// Its fill.
    pub fill: Fill,
}

/// Runs joined along a baseline: what a chart prints as one label.
#[derive(Debug)]
struct Label {
    /// Its inked runs left to right, then the spaces drawn between them.
    runs: Vec<usize>,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    /// The line's height: the median height of its runs' boxes.
    h: i64,
    /// The baseline.
    base: i64,
    /// What it reads as: the runs' text, a space between two that stand apart with none drawn.
    text: String,
}

impl Label {
    fn cx(&self) -> i64 {
        (self.x0 + self.x1).div_euclid(2)
    }

    fn cy(&self) -> i64 {
        (self.y0 + self.y1).div_euclid(2)
    }

    fn rect(&self) -> QuantRect {
        QuantRect {
            x0: self.x0,
            y0: self.y0,
            x1: self.x1,
            y1: self.y1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    /// Bars rising from a base below them.
    Vertical,
    /// Bars running right from a base at their left.
    Horizontal,
}

/// Bars of one width set end to end: a stack, its segments from the base out.
#[derive(Debug, Clone)]
struct Column {
    segs: Vec<usize>,
    /// The span across the bars' direction.
    lo: i64,
    hi: i64,
    /// Where it stands, and where its last segment ends.
    base: i64,
    end: i64,
}

impl Column {
    fn thickness(&self) -> i64 {
        self.hi - self.lo
    }
}

/// Columns on one base: its categories, each one column or one period of columns.
#[derive(Debug)]
struct Chart {
    axis: Axis,
    groups: Vec<Vec<Column>>,
    stacked: bool,
    /// Where the groups were read from their gaps (`-v2`): the series' colours, and each group's
    /// columns' series. `None` where a column's place in its group is its series.
    slots: Option<Slots>,
    /// Whether its columns are each a colour none of the others is (`-v2`): one series whose
    /// colours key its categories, so no title is read as the series' name.
    by_category: bool,
}

/// The series of a chart whose groups were read from their gaps (`-v2`).
#[derive(Debug)]
struct Slots {
    /// The series' colours, in the order their bars stand in a group.
    fills: Vec<Fill>,
    /// For each group, each column's series.
    of: Vec<Vec<usize>>,
}

/// What a chart's table holds before it is built: labels by index.
#[derive(Debug)]
struct Found {
    headers: Vec<Option<usize>>,
    rows: Vec<(Vec<usize>, Vec<Option<usize>>)>,
}

/// Every table `bar-labels-v2` finds on one page, top to bottom.
///
/// `page_area` is the page's area in square centipoints: a rectangle a quarter of the page or more
/// is a background, never a bar.
///
/// # Errors
///
/// An id the allocator cannot issue.
pub(crate) fn detect(
    page: u32,
    runs: &[TrackRun<'_>],
    bars: &[Bar],
    page_area: i128,
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    let labels = labels(runs);
    let ink = ink(bars, page_area);
    let charts: Vec<Chart> = [Axis::Vertical, Axis::Horizontal]
        .into_iter()
        .flat_map(|axis| charts(&ink, axis))
        .collect();
    let boxes: Vec<(QuantRect, Fill)> = charts
        .iter()
        .flat_map(|c| {
            let b = bars_box(c, &ink);
            fills(c, &ink).into_iter().map(move |f| (b, f))
        })
        .collect();
    let mut found: Vec<Found> = charts
        .iter()
        .filter_map(|c| table(c, &labels, &ink, &boxes))
        .collect();
    // No run is in two tables: where two read the same labels, the one with more cells stands.
    found.sort_by_key(|f| std::cmp::Reverse(cells_of(f).len()));
    let mut taken: Vec<usize> = Vec::new();
    let mut kept: Vec<Found> = Vec::new();
    for f in found {
        let mine = cells_of(&f);
        if mine.iter().any(|l| taken.contains(l)) {
            continue;
        }
        taken.extend(mine);
        kept.push(f);
    }
    let mut tables = kept
        .iter()
        .map(|f| build(page, f, &labels, runs, alloc))
        .collect::<Result<Vec<_>, _>>()?;
    tables.sort_by_key(|t| (t.rect.y0, t.rect.x0));
    Ok(tables)
}

/// The labels a table reads: headers, categories, values.
fn cells_of(f: &Found) -> Vec<usize> {
    let mut out: Vec<usize> = f.headers.iter().flatten().copied().collect();
    for (category, values) in &f.rows {
        out.extend(category);
        out.extend(values.iter().flatten());
    }
    out
}

/// The page's labels: upright runs no other rule's table holds, by line, joined wherever less than
/// half the line's height separates one run's ink from the next.
fn labels(runs: &[TrackRun<'_>]) -> Vec<Label> {
    let mut order: Vec<usize> = (0..runs.len())
        .filter(|&i| {
            !runs[i].claimed && !runs[i].text.trim().is_empty() && crate::tracks::upright(&runs[i])
        })
        .collect();
    order.sort_by_key(|&i| (runs[i].y, runs[i].x, i));
    let height = |i: usize| runs[i].rect.map_or(0, |b| b.y1 - b.y0);
    let mut lines: Vec<(i64, i64, Vec<usize>)> = Vec::new();
    for i in order {
        let h = height(i);
        match lines.last_mut() {
            Some((y, lh, members)) if 10 * (runs[i].y - *y).abs() <= 3 * h.max(*lh) => {
                members.push(i);
                *lh = (*lh).max(h);
            }
            _ => lines.push((runs[i].y, h, vec![i])),
        }
    }
    let mut out = Vec::new();
    for (base, _, mut members) in lines {
        members.sort_by_key(|&i| (runs[i].rect.map_or(0, |b| b.x0), i));
        let mut hs: Vec<i64> = members.iter().map(|&i| height(i)).collect();
        hs.sort_unstable();
        let h = hs[hs.len() / 2];
        let mut current: Option<Label> = None;
        for i in members {
            let Some(b) = runs[i].rect else { continue };
            match current.as_mut() {
                Some(l) if 2 * (b.x0 - l.x1) <= h => {
                    let prev = *l.runs.last().expect("a label holds a run");
                    if 100 * (b.x0 - l.x1) > 12 * h
                        && !runs[prev].text.ends_with(char::is_whitespace)
                        && !runs[i].text.starts_with(char::is_whitespace)
                    {
                        l.text.push(' ');
                    }
                    l.text.push_str(runs[i].text);
                    l.runs.push(i);
                    l.x1 = l.x1.max(b.x1);
                    l.y0 = l.y0.min(b.y0);
                    l.y1 = l.y1.max(b.y1);
                }
                _ => {
                    out.extend(current.take());
                    current = Some(Label {
                        runs: vec![i],
                        x0: b.x0,
                        y0: b.y0,
                        x1: b.x1,
                        y1: b.y1,
                        h,
                        base,
                        text: runs[i].text.to_string(),
                    });
                }
            }
        }
        out.extend(current);
    }
    // A space the page drew as a run of its own, inside a label on its line, is that label's: the
    // tracks rule's cells hold theirs the same way, and the cell's text is then the words it draws.
    for i in (0..runs.len())
        .filter(|&i| !runs[i].claimed && runs[i].text.trim().is_empty() && !runs[i].text.is_empty())
    {
        if let Some(l) = out.iter_mut().find(|l| {
            10 * (runs[i].y - l.base).abs() <= 3 * l.h && l.x0 < runs[i].x && runs[i].x < l.x1
        }) {
            l.runs.push(i);
        }
    }
    for l in &mut out {
        l.text = l.text.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    out
}

/// The rectangles that can be bars: filled in a known colour that is not white, a point or more
/// each way, under a quarter of the page — each drawn twice counted once.
fn ink(bars: &[Bar], page_area: i128) -> Vec<Bar> {
    let mut cand: Vec<Bar> = bars
        .iter()
        .copied()
        .filter(|b| {
            let r = b.rect;
            b.fill.count != 0
                && !b.fill.is_white()
                && r.x1 - r.x0 >= MIN_BAR
                && r.y1 - r.y0 >= MIN_BAR
                && 4 * i128::from(r.x1 - r.x0) * i128::from(r.y1 - r.y0) < page_area
        })
        .collect();
    cand.sort_by_key(|b| (b.fill, b.rect.x0, b.rect.y0, b.rect.x1, b.rect.y1));
    let mut out: Vec<Bar> = Vec::new();
    for b in cand {
        let twice = out
            .iter()
            .rev()
            .take_while(|k| k.fill == b.fill && b.rect.x0 - k.rect.x0 <= SAME_EDGE)
            .any(|k| {
                (k.rect.y0 - b.rect.y0).abs() <= SAME_EDGE
                    && (k.rect.x1 - b.rect.x1).abs() <= SAME_EDGE
                    && (k.rect.y1 - b.rect.y1).abs() <= SAME_EDGE
            });
        if !twice {
            out.push(b);
        }
    }
    out
}

/// The span of a bar across `axis`, and its extent along it.
fn span(b: &Bar, axis: Axis) -> ((i64, i64), (i64, i64)) {
    let r = b.rect;
    match axis {
        Axis::Vertical => ((r.x0, r.x1), (r.y0, r.y1)),
        Axis::Horizontal => ((r.y0, r.y1), (r.x0, r.x1)),
    }
}

/// Stacks: bars sharing one span across `axis`, set end to end along it.
fn columns(ink: &[Bar], axis: Axis) -> Vec<Column> {
    let mut order: Vec<usize> = (0..ink.len()).collect();
    order.sort_by_key(|&i| (span(&ink[i], axis), i));
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in order {
        let (lo, hi) = span(&ink[i], axis).0;
        let home = groups
            .iter_mut()
            .rev()
            .take_while(|g| span(&ink[g[0]], axis).0 .0 >= lo - SAME_EDGE)
            .find(|g| {
                let (glo, ghi) = span(&ink[g[0]], axis).0;
                (glo - lo).abs() <= SAME_EDGE && (ghi - hi).abs() <= SAME_EDGE
            });
        match home {
            Some(g) => g.push(i),
            None => groups.push(vec![i]),
        }
    }
    let mut out = Vec::new();
    for mut g in groups {
        g.sort_by_key(|&i| (span(&ink[i], axis).1, i));
        let mut chains: Vec<Vec<usize>> = Vec::new();
        for i in g {
            match chains.last_mut() {
                // A stack's segments are its series: two touching segments of one colour are a
                // row of shaded cells or a bar drawn in pieces, never one stack.
                Some(c)
                    if (span(&ink[i], axis).1 .0
                        - span(&ink[*c.last().expect("a chain holds a bar")], axis)
                            .1
                             .1)
                        .abs()
                        <= SAME_EDGE
                        && !ink[i]
                            .fill
                            .same(ink[*c.last().expect("a chain holds a bar")].fill) =>
                {
                    c.push(i)
                }
                _ => chains.push(vec![i]),
            }
        }
        for c in chains {
            let (lo, hi) = span(&ink[c[0]], axis).0;
            let first = span(&ink[c[0]], axis).1;
            let last = span(&ink[*c.last().expect("a chain holds a bar")], axis).1;
            out.push(match axis {
                Axis::Vertical => Column {
                    base: last.1,
                    end: first.0,
                    segs: c.into_iter().rev().collect(),
                    lo,
                    hi,
                },
                Axis::Horizontal => Column {
                    base: first.0,
                    end: last.1,
                    segs: c,
                    lo,
                    hi,
                },
            });
        }
    }
    out
}

/// The charts along `axis`: columns on one base, of one width, side by side, grouped into their
/// categories.
fn charts(ink: &[Bar], axis: Axis) -> Vec<Chart> {
    let mut cols = columns(ink, axis);
    cols.sort_by_key(|c| (c.base, c.lo));
    let mut bases: Vec<Vec<Column>> = Vec::new();
    for c in cols {
        match bases.last_mut() {
            Some(b) if (c.base - b[0].base).abs() <= SAME_BASE => b.push(c),
            _ => bases.push(vec![c]),
        }
    }
    let mut out = Vec::new();
    for mut b in bases {
        b.sort_by_key(|c| (c.lo, c.hi));
        let mut runs: Vec<Vec<Column>> = Vec::new();
        for c in b {
            let split = runs.last().and_then(|r| r.last()).is_none_or(|a| {
                let gap = c.lo - a.hi;
                let (ta, tc) = (a.thickness(), c.thickness());
                gap < -SAME_EDGE || gap > 4 * ta.max(tc) || 100 * (ta - tc).abs() > 15 * ta.max(tc)
            });
            if split {
                runs.push(vec![c]);
            } else {
                runs.last_mut().expect("a run to join").push(c);
            }
        }
        for run in runs.into_iter().filter(|r| r.len() >= MIN_BARS) {
            let lengths: Vec<i64> = run.iter().map(|c| (c.end - c.base).abs()).collect();
            let (min, max) = (
                lengths.iter().min().copied().unwrap_or(0),
                lengths.iter().max().copied().unwrap_or(0),
            );
            let stacked = run.iter().any(|c| c.segs.len() > 1);
            if !stacked && 100 * (max - min) < max {
                continue; // every bar one length and one segment: shading, not data
            }
            let colours: Vec<Fill> = run.iter().map(|c| ink[c.segs[0]].fill).collect();
            // `-v2`: three columns or more, each a colour none of the others is, are one series
            // coloured by category.
            let each_its_own = !stacked
                && run.len() >= 3
                && colours
                    .iter()
                    .enumerate()
                    .all(|(i, f)| colours[..i].iter().all(|g| !g.same(*f)));
            if stacked || colours.iter().all(|&f| f.same(colours[0])) || each_its_own {
                out.push(Chart {
                    axis,
                    groups: run.into_iter().map(|c| vec![c]).collect(),
                    stacked,
                    slots: None,
                    by_category: each_its_own,
                });
                continue;
            }
            // Bars side by side per category: the shortest period whose colours repeat exactly.
            let before = out.len();
            for n in 2..=run.len() / 2 {
                let mut distinct = colours[..n].to_vec();
                distinct.sort_unstable();
                distinct.dedup();
                if run.len() % n != 0
                    || distinct.len() != n
                    || !(0..run.len()).all(|i| colours[i].same(colours[i % n]))
                {
                    continue;
                }
                let groups: Vec<Vec<Column>> = run.chunks(n).map(<[Column]>::to_vec).collect();
                let inner = groups
                    .iter()
                    .flat_map(|g| g.windows(2).map(|w| w[1].lo - w[0].hi))
                    .max()
                    .unwrap_or(0);
                let outer = groups
                    .windows(2)
                    .map(|w| w[1][0].lo - w[0][n - 1].hi)
                    .min()
                    .unwrap_or(0);
                if outer > inner {
                    out.push(Chart {
                        axis,
                        groups,
                        stacked: false,
                        slots: None,
                        by_category: false,
                    });
                }
                break;
            }
            // `-v2`: no period — a bar left out of a group, or a group set in colours of its own —
            // and the groups are read from the gaps between them.
            if out.len() == before {
                out.extend(gap_groups(run, &colours, axis));
            }
        }
    }
    out
}

/// The groups of a run of columns read from the gaps between them (`-v2`), where no period of
/// colours holds: a gap more than twice the widest of the narrow ones — within half the narrowest
/// again, and half a point — ends a group. Two groups or more, one of two columns or more. A
/// series is a colour two groups or more hold, its place the mean of its first places there; a
/// column of another colour takes the series of its place, only in a group holding one column per
/// series. No group holds two columns of one series.
fn gap_groups(run: Vec<Column>, colours: &[Fill], axis: Axis) -> Option<Chart> {
    let gaps: Vec<i64> = run.windows(2).map(|w| w[1].lo - w[0].hi).collect();
    let narrowest = *gaps.iter().min()?;
    let inside = narrowest.max(0) * 3 / 2 + SAME_EDGE;
    let widest_inside = gaps
        .iter()
        .copied()
        .filter(|&g| g <= inside)
        .max()
        .unwrap_or(narrowest);
    let between = gaps.iter().copied().filter(|&g| g > inside).min()?;
    if between <= 2 * widest_inside.max(SAME_EDGE) {
        return None;
    }
    let mut groups: Vec<Vec<usize>> = vec![vec![0]];
    for (i, &g) in gaps.iter().enumerate() {
        if g > inside {
            groups.push(Vec::new());
        }
        groups.last_mut().expect("a group").push(i + 1);
    }
    if groups.len() < MIN_BARS || groups.iter().all(|g| g.len() < 2) {
        return None;
    }
    // Each colour: the groups holding it, and the sum of its first places there.
    let mut seen: Vec<(Fill, usize, usize)> = Vec::new();
    for g in &groups {
        for (place, &k) in g.iter().enumerate() {
            if g[..place].iter().any(|&j| colours[j].same(colours[k])) {
                continue; // a group counts once
            }
            match seen.iter_mut().find(|c| c.0.same(colours[k])) {
                Some(c) => {
                    c.1 += 1;
                    c.2 += place;
                }
                None => seen.push((colours[k], 1, place)),
            }
        }
    }
    let mut series: Vec<(Fill, usize, usize)> = seen.into_iter().filter(|c| c.1 >= 2).collect();
    if series.len() < 2 {
        return None;
    }
    // By mean place, compared without dividing: a/b against c/d as a·d against c·b.
    series.sort_by(|a, b| (a.2 * b.1).cmp(&(b.2 * a.1)));
    let fills: Vec<Fill> = series.iter().map(|s| s.0).collect();
    let mut of: Vec<Vec<usize>> = Vec::with_capacity(groups.len());
    for g in &groups {
        let mut slot = Vec::with_capacity(g.len());
        for (place, &k) in g.iter().enumerate() {
            match fills.iter().position(|f| f.same(colours[k])) {
                Some(s) => slot.push(s),
                None if g.len() == fills.len() => slot.push(place),
                None => return None,
            }
        }
        if slot.iter().enumerate().any(|(i, s)| slot[..i].contains(s)) {
            return None;
        }
        of.push(slot);
    }
    let mut run: Vec<Option<Column>> = run.into_iter().map(Some).collect();
    Some(Chart {
        axis,
        groups: groups
            .iter()
            .map(|g| {
                g.iter()
                    .map(|&k| run[k].take().expect("each column once"))
                    .collect()
            })
            .collect(),
        stacked: false,
        slots: Some(Slots { fills, of }),
        by_category: false,
    })
}

/// A chart's series: the stack's colours from the base out, the period's positions, or its one
/// colour.
fn series(chart: &Chart, ink: &[Bar]) -> Vec<Fill> {
    if let Some(slots) = &chart.slots {
        return slots.fills.clone();
    }
    if chart.stacked {
        let mut out: Vec<Fill> = Vec::new();
        for c in chart.groups.iter().flatten() {
            for &s in &c.segs {
                if !out.iter().any(|f| f.same(ink[s].fill)) {
                    out.push(ink[s].fill);
                }
            }
        }
        out
    } else {
        chart.groups[0]
            .iter()
            .map(|c| ink[c.segs[0]].fill)
            .collect()
    }
}

fn fills(chart: &Chart, ink: &[Bar]) -> Vec<Fill> {
    series(chart, ink)
}

/// The box of a chart's bars.
fn bars_box(chart: &Chart, ink: &[Bar]) -> QuantRect {
    let mut b: Option<QuantRect> = None;
    for &s in chart.groups.iter().flatten().flat_map(|c| &c.segs) {
        let r = ink[s].rect;
        b = Some(match b {
            Some(a) => QuantRect {
                x0: a.x0.min(r.x0),
                y0: a.y0.min(r.y0),
                x1: a.x1.max(r.x1),
                y1: a.y1.max(r.y1),
            },
            None => r,
        });
    }
    b.expect("a chart has bars")
}

/// The table a chart prints, or `None` where it does not print one.
fn table(
    chart: &Chart,
    labels: &[Label],
    ink: &[Bar],
    boxes: &[(QuantRect, Fill)],
) -> Option<Found> {
    let axis = chart.axis;
    let groups = &chart.groups;
    let series = series(chart, ink);
    let gaps: Vec<i64> = groups
        .windows(2)
        .map(|w| w[1][0].lo - w[0][w[0].len() - 1].hi)
        .collect();
    let mut sorted = gaps.clone();
    sorted.sort_unstable();
    let gap = sorted
        .get(sorted.len() / 2)
        .copied()
        .unwrap_or_else(|| groups[0][0].thickness());

    // Values: the one number on each segment, or just beyond a single bar's end.
    let mut cells: Vec<((usize, usize), usize, i64)> = Vec::new();
    for (gi, g) in groups.iter().enumerate() {
        for (ci, c) in g.iter().enumerate() {
            for &s in &c.segs {
                let r = ink[s].rect;
                let col = if chart.stacked {
                    series
                        .iter()
                        .position(|f| f.same(ink[s].fill))
                        .expect("every colour is a series")
                } else if let Some(slots) = &chart.slots {
                    slots.of[gi][ci]
                } else {
                    ci
                };
                let on = |k: &Label| match axis {
                    Axis::Vertical => {
                        r.x0 - k.h / 4 <= k.cx()
                            && k.cx() <= r.x1 + k.h / 4
                            && r.y0 - 3 * k.h / 10 <= k.cy()
                            && k.cy() <= r.y1 + 3 * k.h / 10
                    }
                    Axis::Horizontal => {
                        r.y0 - k.h / 4 <= k.cy()
                            && k.cy() <= r.y1 + k.h / 4
                            && r.x0 - 3 * k.h / 10 <= k.cx()
                            && k.cx() <= r.x1 + 3 * k.h / 10
                    }
                };
                let beyond = |k: &Label| match axis {
                    Axis::Vertical => {
                        c.lo - k.h / 4 <= k.cx()
                            && k.cx() <= c.hi + k.h / 4
                            && k.y1 <= c.end + 3 * k.h / 10
                            && 2 * (c.end - k.y1) <= 3 * k.h
                    }
                    Axis::Horizontal => {
                        c.lo - k.h / 4 <= k.cy()
                            && k.cy() <= c.hi + k.h / 4
                            && k.x0 >= c.end - 3 * k.h / 10
                            && k.x0 - c.end <= 3 * k.h
                    }
                };
                let picks: Vec<usize> = (0..labels.len())
                    .filter(|&k| {
                        number(&labels[k].text).is_some()
                            && (on(&labels[k]) || (c.segs.len() == 1 && beyond(&labels[k])))
                    })
                    .collect();
                if let [k] = picks[..] {
                    let length = match axis {
                        Axis::Vertical => r.y1 - r.y0,
                        Axis::Horizontal => r.x1 - r.x0,
                    };
                    cells.push(((gi, col), k, length));
                }
            }
        }
    }
    // A number two cells could read fills neither.
    let mut readers: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for &(_, k, _) in &cells {
        *readers.entry(k).or_default() += 1;
    }
    cells.retain(|&(_, k, _)| readers[&k] == 1);
    // A series keeps its numbers only where most of its bars carry one.
    for (col, &colour) in series.iter().enumerate() {
        let bars_in = groups
            .iter()
            .enumerate()
            .map(|(gi, g)| {
                if chart.stacked {
                    g.iter()
                        .flat_map(|c| &c.segs)
                        .filter(|&&s| ink[s].fill.same(colour))
                        .count()
                } else if let Some(slots) = &chart.slots {
                    usize::from(slots.of[gi].contains(&col))
                } else {
                    usize::from(col < g.len())
                }
            })
            .sum::<usize>();
        let carried = cells.iter().filter(|c| c.0 .1 == col).count();
        if 10 * carried < LABELLED_IN_TEN * bars_in {
            cells.retain(|c| c.0 .1 != col);
        }
    }

    // Categories, and the rows they open.
    let mut rows: Vec<(Vec<usize>, Vec<Option<usize>>)> = Vec::new();
    let mut points: Vec<(i64, Value)> = Vec::new();
    let mut used: Vec<usize> = Vec::new();
    for (gi, g) in groups.iter().enumerate() {
        let (lo, hi, base) = (g[0].lo, g[g.len() - 1].hi, g[0].base);
        let category = match axis {
            Axis::Vertical => {
                let below: Vec<usize> = (0..labels.len())
                    .filter(|&k| {
                        let l = &labels[k];
                        4 * (lo - l.cx()) <= gap
                            && 4 * (l.cx() - hi) <= gap
                            && l.y0 >= base - 3 * l.h / 10
                            && l.y0 - base <= 8 * l.h
                    })
                    .collect();
                below_label(labels, below, (lo - gap / 2, hi + gap / 2))
                    .filter(|lab| 2 * (labels[lab[0]].y0 - base) <= 5 * labels[lab[0]].h)
            }
            Axis::Horizontal => {
                let left: Vec<usize> = (0..labels.len())
                    .filter(|&k| {
                        let l = &labels[k];
                        4 * (lo - l.cy()) <= gap
                            && 4 * (l.cy() - hi) <= gap
                            && 10 * (l.x1 - base) <= 3 * l.h
                            && base - l.x1 <= 15 * l.h
                    })
                    .collect();
                beside_label(labels, left)
            }
        };
        let Some(category) = category else { continue };
        let text = joined(labels, &category);
        if (number(&text).is_some() && !is_year(&text))
            || starts_with_value(&text)
            || prose(&text)
            || category.iter().any(|&k| on_ink(&labels[k], ink, chart))
        {
            continue;
        }
        let values: Vec<Option<usize>> = (0..series.len())
            .map(|col| cells.iter().find(|c| c.0 == (gi, col)).map(|c| c.1))
            .collect();
        if values.iter().all(Option::is_none) {
            continue;
        }
        if category.iter().any(|k| used.contains(k)) {
            return None; // one label read for two categories
        }
        used.extend(&category);
        for c in cells.iter().filter(|c| c.0 .0 == gi) {
            if let Some(v) = number(&labels[c.1].text) {
                points.push((c.2, v));
            }
        }
        rows.push((category, values));
    }
    if rows.len() < MIN_BARS {
        return None;
    }
    let mut texts: Vec<String> = rows.iter().map(|(c, _)| joined(labels, c)).collect();
    texts.sort_unstable();
    texts.dedup();
    if texts.len() != rows.len() || !in_order(&points) {
        return None;
    }
    used.extend(rows.iter().flat_map(|(_, v)| v.iter().flatten()));

    // Headers: the legend's names for the series' colours; a chart of one colour without one takes
    // the title above it.
    let bars = bars_box(chart, ink);
    // `-v2`: a legend's distance is from the chart and the labels it read, so a legend set beyond
    // the categories is as near as one set beside the bars.
    let near = used.iter().fold(bars, |a, &k| {
        let r = labels[k].rect();
        QuantRect {
            x0: a.x0.min(r.x0),
            y0: a.y0.min(r.y0),
            x1: a.x1.max(r.x1),
            y1: a.y1.max(r.y1),
        }
    });
    let mut headers: Vec<Option<usize>> = series
        .iter()
        .map(|&f| legend(f, chart, labels, ink, (bars, near), boxes, &used))
        .collect();
    // A chart coloured by category takes no title: its colours key its categories, and the line
    // above it is as often the section's heading (`-v2`).
    if series.len() == 1 && headers[0].is_none() && !chart.by_category {
        let top = rows
            .iter()
            .flat_map(|(_, v)| v.iter().flatten())
            .map(|&k| labels[k].y0)
            .fold(bars.y0, i64::min);
        headers[0] = title(labels, QuantRect { y0: top, ..bars }, &used);
    }
    // A series with no number printed in it is no column.
    let keep: Vec<usize> = (0..series.len())
        .filter(|&c| rows.iter().any(|(_, v)| v[c].is_some()))
        .collect();
    Some(Found {
        headers: keep.iter().map(|&c| headers[c]).collect(),
        rows: rows
            .into_iter()
            .map(|(c, v)| (c, keep.iter().map(|&k| v[k]).collect()))
            .collect(),
    })
}

/// The label under a vertical chart's column: its nearest line, holding one label, and the lines
/// below it the label wraps onto — under a line apart, inside the column's slot, and only where the
/// first line fills most of the slot or a line ends in a hyphen.
fn below_label(labels: &[Label], mut cand: Vec<usize>, slot: (i64, i64)) -> Option<Vec<usize>> {
    cand.sort_by_key(|&k| (labels[k].y0, labels[k].x0));
    let mut lines: Vec<Vec<usize>> = Vec::new();
    for k in cand {
        match lines.last_mut() {
            Some(line) if 2 * (labels[k].y0 - labels[line[0]].y0) <= labels[k].h => line.push(k),
            _ => lines.push(vec![k]),
        }
    }
    let mut out: Vec<usize> = Vec::new();
    for line in lines {
        let [k] = line[..] else {
            if out.is_empty() {
                return None;
            }
            break;
        };
        if let Some(&first) = out.first() {
            let (l, prev) = (&labels[k], &labels[*out.last().expect("a line")]);
            let wrapped = out.iter().any(|&o| labels[o].text.ends_with(['-', '–']))
                || 10 * (labels[first].x1 - labels[first].x0) >= 6 * (slot.1 - slot.0);
            if 5 * (l.y0 - prev.y1) > 4 * l.h || !wrapped || l.x0 < slot.0 || l.x1 > slot.1 {
                break;
            }
        }
        out.push(k);
    }
    (!out.is_empty()).then_some(out)
}

/// The label beside a horizontal chart's bar: the labels whose right edge lies nearest the base,
/// lines stacked under a line apart.
fn beside_label(labels: &[Label], cand: Vec<usize>) -> Option<Vec<usize>> {
    let near = cand.iter().map(|&k| labels[k].x1).max()?;
    let mut first: Vec<usize> = cand
        .into_iter()
        .filter(|&k| 2 * (near - labels[k].x1) <= labels[k].h)
        .collect();
    first.sort_by_key(|&k| labels[k].y0);
    if first
        .windows(2)
        .any(|w| 5 * (labels[w[1]].y0 - labels[w[0]].y1) > 4 * labels[w[1]].h)
    {
        return None;
    }
    Some(first)
}

/// Whether a label is a line of prose: it opens lower-case and runs to a prose line's length, as a
/// paragraph's continuation lines do and a chart's categories do not.
fn prose(text: &str) -> bool {
    text.starts_with(char::is_lowercase) && text.chars().count() >= crate::tracks::PROSE_LINE_CHARS
}

/// Labels' text, in order, one space between.
fn joined(labels: &[Label], ks: &[usize]) -> String {
    ks.iter()
        .map(|&k| labels[k].text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a label is set on a bar of any chart: a fifth of its area or more.
fn on_ink(l: &Label, ink: &[Bar], _chart: &Chart) -> bool {
    let area = i128::from((l.x1 - l.x0).max(1)) * i128::from((l.y1 - l.y0).max(1));
    ink.iter().any(|b| {
        let r = b.rect;
        let (w, h) = (
            l.x1.min(r.x1) - l.x0.max(r.x0),
            l.y1.min(r.y1) - l.y0.max(r.y0),
        );
        w > 0 && h > 0 && 5 * i128::from(w) * i128::from(h) > area
    })
}

/// The legend's name for `fill`: the one label beside a small swatch of that colour, near this
/// chart and nearer it than any other bars of the colour, with nothing but the chart's own labels
/// between a legend above it and its bars; the nearest such name, unless another name stands
/// almost as near.
#[allow(clippy::too_many_arguments)]
fn legend(
    fill: Fill,
    chart: &Chart,
    labels: &[Label],
    ink: &[Bar],
    (bars, near): (QuantRect, QuantRect),
    boxes: &[(QuantRect, Fill)],
    used: &[usize],
) -> Option<usize> {
    let mine: Vec<usize> = chart
        .groups
        .iter()
        .flatten()
        .flat_map(|c| c.segs.clone())
        .collect();
    let small = |b: &Bar| b.rect.x1 - b.rect.x0 <= 2500 && b.rect.y1 - b.rect.y0 <= 2500;
    let dist = |r: QuantRect, x: i64, y: i64| -> i64 {
        let dx = (r.x0 - x).max(0).max(x - r.x1);
        let dy = (r.y0 - y).max(0).max(y - r.y1);
        ((dx as f64).hypot(dy as f64)) as i64
    };
    let mut found: Vec<(i64, usize)> = Vec::new();
    for (si, s) in ink.iter().enumerate() {
        if !s.fill.same(fill) || !small(s) || mine.contains(&si) {
            continue;
        }
        let (sx, sy) = (
            (s.rect.x0 + s.rect.x1).div_euclid(2),
            (s.rect.y0 + s.rect.y1).div_euclid(2),
        );
        let names: Vec<usize> = (0..labels.len())
            .filter(|&k| {
                let l = &labels[k];
                0 <= l.x0 - s.rect.x1
                    && l.x0 - s.rect.x1 <= 2 * l.h
                    && 5 * (l.cy() - sy).abs() <= 3 * l.h
            })
            .collect();
        let [k] = names[..] else { continue };
        let l = &labels[k];
        if number(&l.text).is_some() && !is_year(&l.text) {
            continue;
        }
        // The name runs into the next entry.
        if ink.iter().any(|o| {
            o != s
                && small(o)
                && 5 * ((o.rect.y0 + o.rect.y1).div_euclid(2) - l.cy()).abs() <= 3 * l.h
                && l.x0 < (o.rect.x0 + o.rect.x1).div_euclid(2)
                && (o.rect.x0 + o.rect.x1).div_euclid(2) < l.x1
        }) {
            continue;
        }
        let d = dist(near, sx, sy);
        if d > (bars.x1 - bars.x0).max(bars.y1 - bars.y0) {
            continue; // too far to be this chart's
        }
        if boxes
            .iter()
            .any(|&(b, f)| f.same(fill) && b != bars && dist(b, sx, sy) < d)
        {
            continue; // nearer another chart of the colour
        }
        let swatch = i128::from(s.rect.x1 - s.rect.x0) * i128::from(s.rect.y1 - s.rect.y0);
        if ink.iter().enumerate().any(|(oi, o)| {
            o.fill.same(fill)
                && !mine.contains(&oi)
                && i128::from(o.rect.x1 - o.rect.x0) * i128::from(o.rect.y1 - o.rect.y0)
                    > 4 * swatch
                && dist(o.rect, sx, sy) < d
        }) {
            continue; // bars of its colour stand nearer: another chart's legend
        }
        let lines = wrapped_name(labels, k, ink);
        if sy < bars.y0
            && labels.iter().enumerate().any(|(o, x)| {
                !used.contains(&o)
                    && !lines.contains(&o)
                    && number(&x.text).is_none()
                    && s.rect.y1 < x.cy()
                    && x.cy() < bars.y0
                    && 5 * (x.cy() - sy).abs() > 3 * x.h
                    && x.x1 > bars.x0
                    && x.x0 < bars.x1
            })
        {
            continue; // text stands between the legend and this chart
        }
        found.push((d, k));
    }
    found.sort_unstable();
    let &(d, k) = found.first()?;
    let name = joined(labels, &wrapped_name(labels, k, ink));
    if found
        .iter()
        .any(|&(od, ok)| joined(labels, &wrapped_name(labels, ok, ink)) != name && od < 2 * d + 500)
    {
        return None;
    }
    Some(k)
}

/// A legend name and the lines it wraps onto: the same left edge, under a line apart, no swatch
/// before them.
fn wrapped_name(labels: &[Label], first: usize, ink: &[Bar]) -> Vec<usize> {
    let mut out = vec![first];
    loop {
        let prev = &labels[*out.last().expect("a name")];
        let next: Vec<usize> = (0..labels.len())
            .filter(|&k| {
                let l = &labels[k];
                2 * (l.x0 - labels[first].x0).abs() <= l.h
                    && -3 * l.h <= 10 * (l.y0 - prev.y1)
                    && 5 * (l.y0 - prev.y1) <= 3 * l.h
                    && l.y0 > prev.y0
                    && !ink.iter().any(|b| {
                        5 * ((b.rect.y0 + b.rect.y1).div_euclid(2) - l.cy()).abs() <= 3 * l.h
                            && 0 <= l.x0 - b.rect.x1
                            && l.x0 - b.rect.x1 <= 2 * l.h
                    })
            })
            .collect();
        match next[..] {
            [k] if !out.contains(&k) => out.push(k),
            _ => return out,
        }
    }
}

/// The title of a chart of one colour: the block of lines nearest above it, within four lines,
/// each the only label on its line over the chart and spanning its centre, ending in no full stop
/// and holding no number but a year.
fn title(labels: &[Label], chart: QuantRect, used: &[usize]) -> Option<usize> {
    let cx = (chart.x0 + chart.x1).div_euclid(2);
    let above: Vec<usize> = (0..labels.len())
        .filter(|&k| {
            let l = &labels[k];
            !used.contains(&k)
                && l.y1 <= chart.y0
                && chart.y0 - l.y1 <= 4 * l.h
                && l.x0 <= cx
                && cx <= l.x1
        })
        .collect();
    let &first = above
        .iter()
        .max_by_key(|&&k| (labels[k].y1, std::cmp::Reverse(k)))?;
    let alone = labels
        .iter()
        .filter(|o| {
            10 * (o.base - labels[first].base).abs() <= 3 * labels[first].h
                && o.x1 > chart.x0
                && o.x0 < chart.x1
        })
        .count()
        == 1;
    let text = &labels[first].text;
    if !alone || text.ends_with('.') || !only_years(text) {
        return None;
    }
    // Only a one-line title names a column: a cell holds one label.
    if above.iter().any(|&k| {
        k != first
            && 0 <= labels[first].y0 - labels[k].y1
            && 5 * (labels[first].y0 - labels[k].y1) <= 3 * labels[k].h
    }) {
        return None;
    }
    Some(first)
}

/// Whether every number in `text` is a year.
fn only_years(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .all(is_year)
}

/// A number as charts print one, exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Value {
    /// The digits, signed, with the decimals' digits after them.
    mantissa: i128,
    /// How many of them are decimals.
    scale: u32,
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let scale = self.scale.max(other.scale);
        let a = self.mantissa * 10i128.pow(scale - self.scale);
        let b = other.mantissa * 10i128.pow(scale - other.scale);
        a.cmp(&b)
    }
}

/// `text` as a number: an optional sign or bracket, an optional currency sign, digits — grouped in
/// threes by commas, or not at all — an optional decimal part, an optional unit (`%`, `x`, `k`,
/// `m`, `bn`, `pp`) and an optional closing bracket. Anything else is not a number.
fn number(text: &str) -> Option<Value> {
    let s: Vec<char> = text.chars().collect();
    let mut i = 0;
    let lead = s.first().copied().filter(|c| "(-−–+".contains(*c));
    if lead.is_some() {
        i += 1;
    }
    if s.get(i).is_some_and(|c| "$€£¥".contains(*c)) {
        i += 1;
    }
    let digits = |i: usize| s[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    let first = digits(i);
    if first == 0 || first > 18 {
        return None;
    }
    let mut whole: String = s[i..i + first].iter().collect();
    i += first;
    if s.get(i) == Some(&',') {
        if first > 3 {
            return None;
        }
        while s.get(i) == Some(&',') {
            if digits(i + 1) != 3 || whole.len() > 15 {
                return None;
            }
            whole.extend(&s[i + 1..i + 4]);
            i += 4;
        }
    }
    let mut decimals = String::new();
    if s.get(i) == Some(&'.') && digits(i + 1) > 0 {
        let n = digits(i + 1).min(6);
        if digits(i + 1) > 6 {
            return None;
        }
        decimals = s[i + 1..i + 1 + n].iter().collect();
        i += 1 + n;
    }
    if s.get(i).is_some_and(|c| c.is_whitespace()) {
        i += 1;
    }
    let rest: String = s[i..].iter().collect::<String>().to_lowercase();
    let unit = ["%", "x", "k", "m", "bn", "pp"]
        .into_iter()
        .find(|u| rest.starts_with(u))
        .map_or(0, |u| u.chars().count());
    i += unit;
    let closed = s.get(i) == Some(&')');
    if closed {
        i += 1;
    }
    if i != s.len() {
        return None;
    }
    let mantissa: i128 = format!("{whole}{decimals}").parse().ok()?;
    let negative = matches!(lead, Some('-' | '−' | '–')) || (lead == Some('(') && closed);
    Some(Value {
        mantissa: if negative { -mantissa } else { mantissa },
        scale: decimals.len() as u32,
    })
}

/// A year: four digits, 19xx or 20xx.
fn is_year(text: &str) -> bool {
    text.len() == 4
        && text.chars().all(|c| c.is_ascii_digit())
        && (text.starts_with("19") || text.starts_with("20"))
}

/// Whether a label opens with a value: an optional bracket and currency sign, then digits, a
/// point, digits.
fn starts_with_value(text: &str) -> bool {
    let t = text.strip_prefix('(').unwrap_or(text);
    let t = t.strip_prefix(['$', '€', '£', '¥']).unwrap_or(t);
    let d = t.chars().take_while(char::is_ascii_digit).count();
    d > 0 && t[d..].starts_with('.') && t[d + 1..].starts_with(|c: char| c.is_ascii_digit())
}

/// Whether the numbers agree with their bars in order: no bar a tenth longer than another carries
/// a smaller number, and none a third longer carries one not larger. Positive numbers only, and two
/// of them at least.
fn in_order(points: &[(i64, Value)]) -> bool {
    let zero = Value {
        mantissa: 0,
        scale: 0,
    };
    let pts: Vec<&(i64, Value)> = points.iter().filter(|(l, v)| *l > 0 && *v > zero).collect();
    if pts.len() < 2 {
        return false;
    }
    pts.iter().all(|&&(l1, v1)| {
        pts.iter()
            .all(|&&(l2, v2)| !(10 * l1 < 9 * l2 && v1 > v2 || 4 * l1 < 3 * l2 && v1 >= v2))
    })
}

/// The table from what a chart printed: a header row of its series' names, then a row per
/// category — a cell for each label, none where nothing was printed.
fn build(
    page: u32,
    f: &Found,
    labels: &[Label],
    runs: &[TrackRun<'_>],
    alloc: &mut IdAllocator,
) -> Result<DetectedTable, EngineError> {
    let id = alloc.next(IdKind::Table)?;
    let mut cells = Vec::new();
    let mut place = |row: usize, column: usize, ks: &[usize]| {
        let mut run_indices: Vec<usize> = ks
            .iter()
            .flat_map(|&k| labels[k].runs.iter().copied())
            .collect();
        run_indices.sort_unstable();
        let mut rect = labels[ks[0]].rect();
        for &k in &ks[1..] {
            let r = labels[k].rect();
            rect = QuantRect {
                x0: rect.x0.min(r.x0),
                y0: rect.y0.min(r.y0),
                x1: rect.x1.max(r.x1),
                y1: rect.y1.max(r.y1),
            };
        }
        let text: String = run_indices.iter().map(|&i| runs[i].text).collect();
        cells.push(DetectedCell {
            position: ethos_parser_core::TableCellPosition {
                row: row as u32,
                column: column as u32,
                rowspan: 1,
                colspan: 1,
                table_id: id.clone(),
            },
            rect,
            run_indices,
            text,
        });
    };
    for (c, h) in f.headers.iter().enumerate() {
        if let Some(k) = h {
            place(0, c + 1, &[*k]);
        }
    }
    for (r, (category, values)) in f.rows.iter().enumerate() {
        place(r + 1, 0, category);
        for (c, v) in values.iter().enumerate() {
            if let Some(k) = v {
                place(r + 1, c + 1, &[*k]);
            }
        }
    }
    let rect = cells
        .iter()
        .map(|c| c.rect)
        .reduce(|a, r| QuantRect {
            x0: a.x0.min(r.x0),
            y0: a.y0.min(r.y0),
            x1: a.x1.max(r.x1),
            y1: a.y1.max(r.y1),
        })
        .expect("a table has cells");
    Ok(DetectedTable {
        id,
        page,
        rect,
        rows: f.rows.len() as u32 + 1,
        columns: f.headers.len() as u32 + 1,
        cells,
        check: not_a_lattice(),
        tagged_check: None,
        rule: ethos_parser_core::TABLE_DETECTION_CHARTS_V2.to_string(),
    })
}

/// The cross-check a chart's table carries: `NotApplicable`, always.
///
/// The geometric half of the check asks the cells' boxes to tile the table's; a chart's cells are
/// its labels where the page set them — a category under its bar, a value over it, a series'
/// name in the legend — and never tile anything.
fn not_a_lattice() -> LocatorCheck {
    LocatorCheck {
        check_id: ethos_parser_core::LOCATOR_CHECK_V1.to_string(),
        check_version: "1".to_string(),
        outcome: CheckStatus::NotApplicable {
            reason: "a chart's cells are its labels where the page set them, not boxes tiling a \
                     grid, so the geometric-vs-structural cross-check does not apply"
                .to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_parser_core::Profile;

    fn alloc() -> IdAllocator {
        IdAllocator::new(Profile::default().profile_sha256().expect("hashes"))
    }

    /// A 10pt label at `(x, y)` points — `y` its baseline — `w` points wide.
    fn run(x: i64, y: i64, w: i64, text: &'static str) -> TrackRun<'static> {
        let (x, y, w) = (x * 100, y * 100, w * 100);
        TrackRun {
            x,
            y,
            em: Some(1000),
            rect: Some(QuantRect {
                x0: x,
                y0: y - 800,
                x1: x + w,
                y1: y + 200,
            }),
            text,
            claimed: false,
        }
    }

    fn rgb(r: i32, g: i32, b: i32) -> Fill {
        Fill {
            components: [r, g, b, 0],
            count: 3,
        }
    }

    /// A bar from `(x0, y0)` to `(x1, y1)` points.
    fn bar(x0: i64, y0: i64, x1: i64, y1: i64, fill: Fill) -> Bar {
        Bar {
            rect: QuantRect {
                x0: x0 * 100,
                y0: y0 * 100,
                x1: x1 * 100,
                y1: y1 * 100,
            },
            fill,
        }
    }

    const PAGE: i128 = 61_200 * 79_200;

    fn found(runs: &[TrackRun<'_>], bars: &[Bar]) -> Vec<DetectedTable> {
        detect(1, runs, bars, PAGE, &mut alloc()).expect("detects")
    }

    /// The grid as text: one row per line, cells by `|`, an empty slot empty.
    fn grid(t: &DetectedTable) -> Vec<String> {
        (0..t.rows)
            .map(|r| {
                (0..t.columns)
                    .map(|c| {
                        t.cells
                            .iter()
                            .find(|x| x.position.row == r && x.position.column == c)
                            .map_or("", |x| x.text.as_str())
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect()
    }

    /// Four bars on a base at y=300, their values over them, years under them, a title above.
    fn four_years(
        values: [&'static str; 4],
        heights: [i64; 4],
    ) -> (Vec<TrackRun<'static>>, Vec<Bar>) {
        let blue = rgb(0, 300, 800);
        let years = ["2020", "2021", "2022", "2023"];
        let mut runs = vec![run(150, 182, 30, "Solar")];
        let mut bars = Vec::new();
        for k in 0..4 {
            let x = 100 + 40 * k as i64;
            bars.push(bar(x, 300 - heights[k], x + 30, 300, blue));
            runs.push(run(x + 5, 300 - heights[k] - 3, 20, values[k]));
            runs.push(run(x + 3, 312, 24, years[k]));
        }
        (runs, bars)
    }

    #[test]
    fn a_bar_chart_prints_its_table() {
        let (runs, bars) = four_years(["10", "20", "30", "40"], [25, 50, 75, 100]);
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(
            grid(&t[0]),
            vec!["|Solar", "2020|10", "2021|20", "2022|30", "2023|40"]
        );
        assert_eq!(t[0].rule, ethos_parser_core::TABLE_DETECTION_CHARTS_V2);
        assert!(matches!(
            t[0].check.outcome,
            CheckStatus::NotApplicable { .. }
        ));
        // Every cell holds exactly its runs' text: nothing placed, nothing joined that was not drawn.
        for cell in &t[0].cells {
            let joined: String = cell.run_indices.iter().map(|&i| runs[i].text).collect();
            assert_eq!(cell.text, joined);
        }
    }

    #[test]
    fn numbers_against_the_bars_order_are_no_table() {
        // The tallest bar carries the smallest number: whatever these are, they are not its value.
        let (runs, bars) = four_years(["40", "20", "30", "10"], [25, 50, 75, 100]);
        assert!(found(&runs, &bars).is_empty());
    }

    #[test]
    fn bars_mostly_without_numbers_keep_none() {
        // Two numbers over seven bars — a line's labels drawn across them — are not their values.
        let blue = rgb(0, 300, 800);
        let mut runs = Vec::new();
        let mut bars = Vec::new();
        for k in 0..7 {
            let x = 100 + 40 * k;
            bars.push(bar(x, 300 - 20 - 10 * k, x + 30, 300, blue));
            runs.push(run(
                x + 3,
                312,
                24,
                ["a1", "b2", "c3", "d4", "e5", "f6", "g7"][k as usize],
            ));
        }
        runs.push(run(105, 270, 20, "35%"));
        runs.push(run(345, 210, 20, "52%"));
        assert!(found(&runs, &bars).is_empty());
    }

    #[test]
    fn a_number_two_bars_could_read_is_neither_bars() {
        // A wide label over two bars of nearly one height, its centre over both: the order of
        // their lengths cannot tell which it is, so it is neither's.
        let blue = rgb(0, 300, 800);
        let bars = vec![
            bar(100, 205, 120, 300, blue),
            bar(124, 200, 144, 300, blue),
            bar(300, 105, 320, 300, blue),
            bar(324, 100, 344, 300, blue),
        ];
        let runs = vec![
            run(112, 197, 20, "7"),
            run(312, 97, 20, "9"),
            run(105, 312, 6, "A"),
            run(130, 312, 6, "B"),
            run(305, 312, 6, "C"),
            run(330, 312, 6, "D"),
        ];
        assert!(found(&runs, &bars).is_empty());
    }
    #[test]
    fn bars_of_one_length_are_shading() {
        // Rows shaded one length, a number in each: a table's banding, not a chart's bars.
        let grey = rgb(900, 900, 900);
        let bars: Vec<Bar> = (0..4)
            .map(|k| bar(100, 100 + 20 * k, 400, 115 + 20 * k, grey))
            .collect();
        let mut runs: Vec<TrackRun<'static>> = (0..4)
            .map(|k| {
                run(
                    60,
                    112 + 20 * k as i64,
                    30,
                    ["Row", "Rows", "Rowx", "Rowy"][k],
                )
            })
            .collect();
        runs.extend((0..4).map(|k| run(150, 112 + 20 * k as i64, 14, ["12", "34", "56", "78"][k])));
        assert!(found(&runs, &bars).is_empty());
    }

    #[test]
    fn side_by_side_bars_are_named_by_their_legend() {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let mut bars = Vec::new();
        let mut runs = Vec::new();
        for (k, cat) in ["North", "South", "East"].into_iter().enumerate() {
            let x = 100 + 80 * k as i64;
            let (ha, hb) = (30 + 20 * k as i64, 40 + 20 * k as i64);
            bars.push(bar(x, 300 - ha, x + 30, 300, a));
            bars.push(bar(x + 30, 300 - hb, x + 60, 300, b));
            runs.push(run(x + 5, 300 - ha - 3, 18, ["11", "15", "19"][k]));
            runs.push(run(x + 35, 300 - hb - 3, 18, ["14", "18", "22"][k]));
            runs.push(run(x + 15, 312, 30, cat));
        }
        bars.push(bar(100, 330, 106, 336, a));
        runs.push(run(110, 336, 30, "2023"));
        bars.push(bar(200, 330, 206, 336, b));
        runs.push(run(210, 336, 30, "2024"));
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(
            grid(&t[0]),
            vec!["|2023|2024", "North|11|14", "South|15|18", "East|19|22"]
        );
    }

    #[test]
    fn a_legend_nearer_other_bars_of_its_colour_names_nothing_here() {
        // The swatch sits under a block of the same blue that is no chart of its own: it is that
        // block's legend, not this chart's.
        let (mut runs, mut bars) = four_years(["10", "20", "30", "40"], [25, 50, 75, 100]);
        runs.remove(0); // no title
        let blue = rgb(0, 300, 800);
        bars.push(bar(265, 280, 285, 320, blue));
        bars.push(bar(270, 330, 276, 336, blue));
        runs.push(run(280, 336, 40, "Imports"));
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(grid(&t[0])[0], "|", "no name for this chart's colour");
    }

    /// North, South and East, two series side by side in each — values over the bars, the
    /// category under the group, a legend for `a` and `b` below — with each group's colours given,
    /// `None` for a bar left out, `inside` points between a group's bars and `step` between groups.
    fn pairs(
        colours: [[Option<Fill>; 2]; 3],
        inside: i64,
        step: i64,
    ) -> (Vec<TrackRun<'static>>, Vec<Bar>) {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let values = [["11", "14"], ["15", "18"], ["19", "22"]];
        let mut bars = Vec::new();
        let mut runs = Vec::new();
        for (k, cat) in ["North", "South", "East"].into_iter().enumerate() {
            let x = 100 + step * k as i64;
            for (s, colour) in colours[k].iter().enumerate() {
                let Some(colour) = colour else { continue };
                let h = 30 + 20 * k as i64 + 10 * s as i64;
                let x0 = x + s as i64 * (30 + inside);
                bars.push(bar(x0, 300 - h, x0 + 30, 300, *colour));
                runs.push(run(x0 + 5, 300 - h - 3, 18, values[k][s]));
            }
            runs.push(run(x + 15 + inside / 2, 312, 30, cat));
        }
        bars.push(bar(100, 330, 106, 336, a));
        runs.push(run(110, 336, 30, "2023"));
        bars.push(bar(200, 330, 206, 336, b));
        runs.push(run(210, 336, 30, "2024"));
        (runs, bars)
    }

    /// **Grouped bars with a bar left out are read from their gaps** (`-v2`): South prints no
    /// 2024 bar, so the colours repeat in no period; the gaps still group the bars, and each bar's
    /// colour names its series.
    #[test]
    fn grouped_bars_with_a_bar_left_out_are_read_from_their_gaps() {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let (runs, bars) = pairs(
            [[Some(a), Some(b)], [Some(a), None], [Some(a), Some(b)]],
            0,
            80,
        );
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(
            grid(&t[0]),
            vec!["|2023|2024", "North|11|14", "South|15|", "East|19|22"]
        );
    }

    /// **A group set in colours of its own takes the series of its places** (`-v2`): South's
    /// bars are highlighted in two other colours; a group holding one bar per series reads them by
    /// where they stand.
    #[test]
    fn a_group_in_colours_of_its_own_takes_the_series_of_its_places() {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let (c, d) = (rgb(100, 800, 100), rgb(800, 100, 100));
        let (runs, bars) = pairs(
            [[Some(a), Some(b)], [Some(c), Some(d)], [Some(a), Some(b)]],
            0,
            80,
        );
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(
            grid(&t[0]),
            vec!["|2023|2024", "North|11|14", "South|15|18", "East|19|22"]
        );
        // Highlighted in one colour, twice: a group counts once, and its bars take their places.
        let (runs, bars) = pairs(
            [[Some(a), Some(b)], [Some(c), Some(c)], [Some(a), Some(b)]],
            0,
            80,
        );
        assert_eq!(grid(&found(&runs, &bars)[0])[2], "South|15|18");
    }

    /// **Groups read from their gaps refuse what they cannot place** (`-v2`): a group holding two
    /// bars of one series is no chart, nor are groups whose gaps do not stand clearly apart — ten
    /// points inside a group and eighteen between, where thirty between reads.
    #[test]
    fn groups_from_gaps_refuse_what_they_cannot_place() {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let (c, d) = (rgb(100, 800, 100), rgb(800, 100, 100));
        let (runs, bars) = pairs(
            [[Some(a), Some(b)], [Some(a), Some(a)], [Some(a), Some(b)]],
            0,
            80,
        );
        assert!(
            found(&runs, &bars).is_empty(),
            "two bars of one series in a group"
        );
        let colours = [[Some(a), Some(b)], [Some(c), Some(d)], [Some(a), Some(b)]];
        let (runs, bars) = pairs(colours, 10, 88);
        assert!(found(&runs, &bars).is_empty(), "gaps of 10 and 18 points");
        let (runs, bars) = pairs(colours, 10, 100);
        assert_eq!(found(&runs, &bars).len(), 1, "gaps of 10 and 30 points");
    }

    /// **Bars each a colour of its own are one series** (`-v2`): a chart coloured by category,
    /// whose colours key its categories, so the line above names no series. Two bars of two colours
    /// say no such thing.
    #[test]
    fn bars_each_a_colour_of_their_own_are_one_series() {
        let (runs, mut bars) = four_years(["10", "20", "30", "40"], [25, 50, 75, 100]);
        for (k, b) in bars.iter_mut().enumerate() {
            b.fill = rgb(100 * k as i32, 300, 800);
        }
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(
            grid(&t[0]),
            vec!["|", "2020|10", "2021|20", "2022|30", "2023|40"]
        );
        assert!(found(&runs[..5], &bars[..2]).is_empty(), "two bars");
    }

    /// **A legend set beyond the categories names its series** (`-v2`): measured from the chart
    /// and the labels it read, a legend left of long category names is as near as one beside the
    /// bars; measured from the bars alone, it stood too far.
    #[test]
    fn a_legend_beyond_the_categories_names_its_series() {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let mut bars = Vec::new();
        let mut runs = Vec::new();
        let cats = [
            "North America region",
            "South America region",
            "Europe and Africa",
        ];
        for (k, cat) in cats.into_iter().enumerate() {
            let y = 100 + 40 * k as i64;
            let (la, lb) = (40 + 20 * k as i64, 50 + 20 * k as i64);
            bars.push(bar(300, y, 300 + la, y + 10, a));
            bars.push(bar(300, y + 10, 300 + lb, y + 20, b));
            runs.push(run(300 + la + 3, y + 8, 18, ["40", "60", "80"][k]));
            runs.push(run(300 + lb + 3, y + 18, 18, ["50", "70", "90"][k]));
            runs.push(run(100, y + 13, 170, cat));
        }
        bars.push(bar(40, 230, 46, 236, a));
        runs.push(run(50, 236, 30, "2023"));
        bars.push(bar(40, 245, 46, 251, b));
        runs.push(run(50, 251, 30, "2024"));
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(
            grid(&t[0]),
            vec![
                "|2023|2024",
                "North America region|40|50",
                "South America region|60|70",
                "Europe and Africa|80|90"
            ]
        );
    }

    #[test]
    fn a_stacked_bar_is_a_row_of_its_segments() {
        let (a, b) = (rgb(0, 300, 800), rgb(900, 500, 100));
        let mut bars = Vec::new();
        let mut runs = Vec::new();
        for (k, cat) in ["Men", "Women"].into_iter().enumerate() {
            let y = 100 + 30 * k as i64;
            let split = 220 + 60 * k as i64; // drawn to scale: 40 and 60 of 100
            bars.push(bar(100, y, split, y + 20, a));
            bars.push(bar(split, y, 400, y + 20, b));
            runs.push(run(140, y + 14, 16, ["40", "60"][k]));
            runs.push(run(300, y + 14, 16, ["60", "40"][k]));
            runs.push(run(50, y + 14, 40, cat));
        }
        bars.push(bar(100, 170, 106, 176, a));
        runs.push(run(110, 176, 30, "Yes"));
        bars.push(bar(200, 170, 206, 176, b));
        runs.push(run(210, 176, 30, "No"));
        let t = found(&runs, &bars);
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(grid(&t[0]), vec!["|Yes|No", "Men|40|60", "Women|60|40"]);
    }

    #[test]
    fn a_table_s_shaded_cells_are_no_stack() {
        // Each value cell of each row shaded with a rectangle of its own, one grey, touching:
        // rows of a table, not stacked bars.
        let grey = rgb(800, 800, 800);
        let mut bars = Vec::new();
        let mut runs = Vec::new();
        for (k, (a, b)) in [("25.9%", "25.5%"), ("16.8%", "17.0%")]
            .into_iter()
            .enumerate()
        {
            let y = 100 + 36 * k as i64;
            bars.push(bar(430, y, 485, y + 18, grey));
            bars.push(bar(485, y, 540, y + 18, grey));
            runs.push(run(380, y + 13, 40, ["Women", "URM"][k]));
            runs.push(run(440, y + 13, 30, a));
            runs.push(run(495, y + 13, 30, b));
        }
        assert!(found(&runs, &bars).is_empty());
    }

    #[test]
    fn a_line_of_prose_is_no_category() {
        // Bars to the right of a paragraph, its lines level with them: those lines are not the
        // bars' categories.
        let blue = rgb(0, 300, 800);
        let mut bars = Vec::new();
        let mut runs = Vec::new();
        for k in 0..3 {
            let y = 100 + 30 * k as i64;
            bars.push(bar(300, y, 360 + 40 * k as i64, y + 18, blue));
            runs.push(run(
                365 + 40 * k as i64,
                y + 13,
                20,
                ["12", "19", "26"][k as usize],
            ));
            runs.push(run(
                40,
                y + 13,
                255,
                [
                    "and the people we hire every year bring",
                    "skills that make our teams better at what",
                    "they do, which is why we invest in them",
                ][k as usize],
            ));
        }
        assert!(found(&runs, &bars).is_empty());
    }

    #[test]
    fn a_rotated_label_is_not_read() {
        // A run whose box does not start at its origin is set on its side.
        let (mut runs, bars) = four_years(["10", "20", "30", "40"], [25, 50, 75, 100]);
        for r in runs.iter_mut().filter(|r| r.text == "2021") {
            r.rect = r.rect.map(|b| QuantRect {
                x0: b.x0 - 300,
                ..b
            });
        }
        let t = found(&runs, &bars);
        assert_eq!(grid(&t[0]), vec!["|Solar", "2020|10", "2022|30", "2023|40"]);
    }

    #[test]
    fn a_space_drawn_as_a_run_stays_in_its_label() {
        let (mut runs, bars) = four_years(["10", "20", "30", "40"], [25, 50, 75, 100]);
        // `Solar` becomes `Solar PV`, its space a run of its own with no ink to box.
        runs.push(TrackRun {
            x: 18_100,
            y: 18_200,
            em: Some(1000),
            rect: None,
            text: " ",
            claimed: false,
        });
        runs.push(run(184, 182, 14, "PV"));
        let t = found(&runs, &bars);
        assert_eq!(grid(&t[0])[0], "|Solar PV");
    }

    #[test]
    fn numbers_are_read_as_charts_print_them() {
        let v = |s| number(s).map(|v| (v.mantissa, v.scale));
        assert_eq!(v("1,069"), Some((1069, 0)));
        assert_eq!(v("$2.6bn"), Some((26, 1)));
        assert_eq!(v("(9.9%)"), Some((-99, 1)));
        assert_eq!(v("-3"), Some((-3, 0)));
        for no in [
            "0.36670.3264",
            "25 2",
            "12,34",
            "2019/20",
            "Q1",
            "1.2.3",
            "",
        ] {
            assert_eq!(number(no), None, "{no}");
        }
        assert!(is_year("2024") && !is_year("2124") && !is_year("204"));
        assert!(starts_with_value("0.6644 CP") && !starts_with_value("Less than $1.00bn"));
        assert!(only_years("Revenue (2015-2024)") && !only_years("Figure 1.7 Population"));
        assert!(number("10").unwrap() < number("10.5").unwrap());
    }
}
