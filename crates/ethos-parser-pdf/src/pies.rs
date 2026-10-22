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

//! A pie's printed values, read back as its table — `slice-labels-v1` (decision #90,
//! `docs/32-CHART-LABELS-SCOPE.md` §10).
//!
//! A pie that prints a number at each slice states a table, as a bar chart that prints its numbers
//! does (`crate::charts`): each slice's number set on it or beside it, the slice named in the
//! legend beside a swatch of its colour or by the lines set over its number, each pie by the line
//! over it or the label in its hole. This rule reads that table back. A value is never read off a
//! slice's size: a pie that prints no number at a slice yields no table.

use std::f64::consts::{PI, TAU};

use ethos_parser_core::{EngineError, IdAllocator};

use crate::charts::{chart_table, is_year, labels, number, Label, Value};
use crate::lines::{legend, Mark};
use crate::tables::{DetectedTable, QuantRect};
use crate::text_state::Fill;
use crate::tracks::TrackRun;

/// A slice a page filled, top-left, centipoints: the centre its straight sides meet at, the points
/// its arc reaches in order, and its colour.
#[derive(Debug, Clone)]
pub(crate) struct Slice {
    pub centre: (i64, i64),
    pub rim: Vec<(i64, i64)>,
    pub fill: Fill,
}

/// A slice's arc: where it starts and how far it turns the way angles grow, in radians, and its
/// colour.
#[derive(Debug, Clone, Copy)]
struct Arc {
    from: f64,
    turn: f64,
    fill: Fill,
}

impl Arc {
    /// How far `angle` lies outside this arc, in radians: nought on it.
    fn off(&self, angle: f64) -> f64 {
        let d = (angle - self.from).rem_euclid(TAU);
        if d <= self.turn {
            0.0
        } else {
            (d - self.turn).min(TAU - d)
        }
    }

    fn mid(&self) -> f64 {
        (self.from + self.turn / 2.0).rem_euclid(TAU)
    }

    /// Whether this arc lies within `other`.
    fn within(&self, other: &Arc) -> bool {
        let d = (self.from - other.from + SAME_ANGLE).rem_euclid(TAU) - SAME_ANGLE;
        d >= -SAME_ANGLE && d + self.turn <= other.turn + SAME_ANGLE
    }
}

/// A pie: its centre and radius, centipoints, and its slices in the order drawn.
#[derive(Debug)]
struct Pie {
    centre: (i64, i64),
    r: i64,
    arcs: Vec<Arc>,
}

impl Pie {
    fn rect(&self) -> QuantRect {
        QuantRect {
            x0: self.centre.0 - self.r,
            y0: self.centre.1 - self.r,
            x1: self.centre.0 + self.r,
            y1: self.centre.1 + self.r,
        }
    }

    /// How far a label stands outside this pie's rim, in centipoints: nought on it or inside.
    fn gap(&self, l: &Label) -> i64 {
        let dx = (l.x0 - self.centre.0).max(0).max(self.centre.0 - l.x1);
        let dy = (l.y0 - self.centre.1).max(0).max(self.centre.1 - l.y1);
        ((dx as f64).hypot(dy as f64) as i64 - self.r).max(0)
    }

    /// The angle a label's centre stands at about this pie's.
    fn angle(&self, l: &Label) -> f64 {
        ((l.cy() - self.centre.1) as f64)
            .atan2((l.cx() - self.centre.0) as f64)
            .rem_euclid(TAU)
    }

    /// Whether a label is set over this pie's centre: the pie's name in its hole, never a slice's.
    fn holds(&self, l: &Label) -> bool {
        l.x0 <= self.centre.0
            && self.centre.0 <= l.x1
            && l.y0 <= self.centre.1
            && self.centre.1 <= l.y1
    }
}

/// A row of a pie's table: the labels naming it, and each pie's number in it.
type Row = (Vec<usize>, Vec<Option<usize>>);

/// Slices whose centres stand this close each way, in centipoints, and whose radii differ by no
/// more, are one pie's: a point.
const SAME_CENTRE: i64 = 100;

/// Two angles this close are one, in radians: a degree.
const SAME_ANGLE: f64 = TAU / 360.0;

/// The furthest a number may stand from the slice it is matched to, in radians: a quarter turn.
const MAX_OFF: f64 = TAU / 4.0;

/// Every table `slice-labels-v1` finds on one page, a chart's pies left to right.
///
/// `marks` are the small marks the page filled: a legend's swatches among them.
///
/// # Errors
///
/// An id the allocator cannot issue.
pub(crate) fn detect(
    page: u32,
    runs: &[TrackRun<'_>],
    slices: &[Slice],
    marks: &[Mark],
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    let pies = pies(slices);
    if pies.is_empty() {
        return Ok(Vec::new());
    }
    let labels = labels(runs);
    // A year names a time, as a pie's title does, and is never a slice's value.
    let numbers: Vec<usize> = (0..labels.len())
        .filter(|&k| number(&labels[k].text).is_some() && !is_year(&labels[k].text))
        .collect();
    // Each number to the pie whose rim it stands nearest, within the pie's radius of it.
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); pies.len()];
    for &k in &numbers {
        let l = &labels[k];
        if pies.iter().any(|p| p.holds(l)) {
            continue;
        }
        let Some(p) = (0..pies.len()).min_by_key(|&p| (pies[p].gap(l), p)) else {
            continue;
        };
        if pies[p].gap(l) <= pies[p].r {
            near[p].push(k);
        }
    }
    // Each pie that reads, and the number for each of its slices.
    let mut read: Vec<(usize, Vec<usize>)> = Vec::new();
    for (p, pie) in pies.iter().enumerate() {
        let mut nums = near[p].clone();
        nums.sort_by_key(|&k| (pie.gap(&labels[k]), k));
        if nums.len() < pie.arcs.len() {
            continue;
        }
        nums.truncate(pie.arcs.len());
        // Its numbers are of one kind — shares, or amounts — as a line's are (decision #89).
        let share = |k: usize| labels[k].text.ends_with('%');
        if nums.iter().any(|&k| share(k) != share(nums[0])) {
            continue;
        }
        if let Some(values) = matched(pie, &labels, &nums) {
            read.push((p, values));
        }
    }
    // Pies read side by side — one height, a colour in common — are one chart.
    let mut order: Vec<usize> = (0..read.len()).collect();
    order.sort_by_key(|&i| (pies[read[i].0].centre.0, i));
    let mut charts: Vec<Vec<usize>> = Vec::new();
    for i in order {
        let p = &pies[read[i].0];
        let beside = |q: &Pie| {
            2 * (p.centre.1 - q.centre.1).abs() <= p.r.min(q.r)
                && p.arcs
                    .iter()
                    .any(|a| q.arcs.iter().any(|b| a.fill.same(b.fill)))
        };
        match charts
            .iter_mut()
            .find(|c| c.last().is_some_and(|&j| beside(&pies[read[j].0])))
        {
            Some(c) => c.push(i),
            None => charts.push(vec![i]),
        }
    }
    let mut taken: Vec<usize> = Vec::new();
    let mut tables = Vec::new();
    for chart in charts {
        let members: Vec<&(usize, Vec<usize>)> = chart.iter().map(|&i| &read[i]).collect();
        let mut used = taken.clone();
        used.extend(members.iter().flat_map(|m| m.1.iter().copied()));
        let plot = members
            .iter()
            .map(|m| pies[m.0].rect())
            .reduce(|a, r| QuantRect {
                x0: a.x0.min(r.x0),
                y0: a.y0.min(r.y0),
                x1: a.x1.max(r.x1),
                y1: a.y1.max(r.y1),
            })
            .expect("a chart has a pie");
        let Some(rows) = by_legend(&pies, &members, &labels, marks, plot, &used)
            .or_else(|| by_name_over(&members, &labels, &used))
        else {
            continue;
        };
        used.extend(rows.iter().flat_map(|r| r.0.iter().copied()));
        let mut headers: Vec<Vec<usize>> = Vec::new();
        for m in &members {
            let pie = &pies[m.0];
            let mine = m.1.iter().chain(
                rows.iter()
                    .filter(|r| r.1.iter().flatten().any(|v| m.1.contains(v)))
                    .flat_map(|r| r.0.iter()),
            );
            let top = mine
                .map(|&k| labels[k].y0)
                .fold(pie.centre.1 - pie.r, i64::min);
            let others: Vec<QuantRect> = members
                .iter()
                .filter(|o| o.0 != m.0)
                .map(|o| pies[o.0].rect())
                .collect();
            let name = title(pie, top, &others, &labels, &used);
            used.extend(&name);
            headers.push(name);
        }
        taken = used;
        tables.push(chart_table(
            page,
            &headers,
            &rows,
            &[],
            &labels,
            runs,
            alloc,
            ethos_parser_core::TABLE_DETECTION_PIES_V1,
        )?);
    }
    Ok(tables)
}

/// A slice's arc, from the points its rim reaches in order.
fn arc(s: &Slice) -> Option<Arc> {
    let at = |p: (i64, i64)| ((p.1 - s.centre.1) as f64).atan2((p.0 - s.centre.0) as f64);
    let turn: f64 = s
        .rim
        .windows(2)
        .map(|w| (at(w[1]) - at(w[0]) + PI).rem_euclid(TAU) - PI)
        .sum();
    let first = at(*s.rim.first()?);
    let (from, turn) = if turn < 0.0 {
        (first + turn, -turn)
    } else {
        (first, turn)
    };
    (turn > 0.0).then_some(Arc {
        from: from.rem_euclid(TAU),
        turn,
        fill: s.fill,
    })
}

/// The pies a page drew: slices about one centre, of one radius, turning a full circle between
/// them, two or more — a slice drawn again, or within a larger one of its colour, counted once.
fn pies(slices: &[Slice]) -> Vec<Pie> {
    let mut pies: Vec<Pie> = Vec::new();
    for s in slices {
        let (Some(arc), Some(&p0)) = (arc(s), s.rim.first()) else {
            continue;
        };
        let r = ((p0.0 - s.centre.0) as f64)
            .hypot((p0.1 - s.centre.1) as f64)
            .round() as i64;
        match pies.iter_mut().find(|p| {
            (p.centre.0 - s.centre.0).abs() <= SAME_CENTRE
                && (p.centre.1 - s.centre.1).abs() <= SAME_CENTRE
                && (p.r - r).abs() <= SAME_CENTRE
        }) {
            Some(p) => p.arcs.push(arc),
            None => pies.push(Pie {
                centre: s.centre,
                r,
                arcs: vec![arc],
            }),
        }
    }
    for p in &mut pies {
        let arcs = p.arcs.clone();
        p.arcs = arcs
            .iter()
            .enumerate()
            .filter(|&(i, a)| {
                !arcs.iter().enumerate().any(|(j, b)| {
                    j != i
                        && b.fill.same(a.fill)
                        && a.within(b)
                        && (b.turn > a.turn + SAME_ANGLE || j < i)
                })
            })
            .map(|(_, a)| *a)
            .collect();
    }
    pies.retain(|p| {
        p.arcs.len() >= 2 && (p.arcs.iter().map(|a| a.turn).sum::<f64>() - TAU).abs() <= SAME_ANGLE
    });
    pies
}

/// Each slice's number: the numbers taken round the centre in order, matched to the slices in
/// theirs — numbers set beside a pie on leader lines keep its slices' order — at the one turn of
/// the one against the other that sets them nearest, each within a quarter turn of its slice. A
/// slice a tenth larger than another never carries a smaller number, and one a third larger
/// carries a larger one.
fn matched(pie: &Pie, labels: &[Label], nums: &[usize]) -> Option<Vec<usize>> {
    let n = pie.arcs.len();
    let mut arcs: Vec<usize> = (0..n).collect();
    arcs.sort_by(|&a, &b| pie.arcs[a].mid().total_cmp(&pie.arcs[b].mid()));
    let mut nums = nums.to_vec();
    nums.sort_by(|&a, &b| pie.angle(&labels[a]).total_cmp(&pie.angle(&labels[b])));
    let cost = |s: usize| {
        (0..n)
            .map(|i| pie.arcs[arcs[i]].off(pie.angle(&labels[nums[(i + s) % n]])))
            .fold((0.0, 0.0_f64), |(t, m), o| (t + o, m.max(o)))
    };
    let costs: Vec<(f64, f64)> = (0..n).map(cost).collect();
    let best = (0..n).min_by(|&a, &b| costs[a].0.total_cmp(&costs[b].0))?;
    if costs[best].1 > MAX_OFF
        || (0..n).any(|s| s != best && costs[s].0 <= costs[best].0 + SAME_ANGLE)
    {
        return None;
    }
    let mut values = vec![0; n];
    for i in 0..n {
        values[arcs[i]] = nums[(i + best) % n];
    }
    let sizes: Vec<Value> = values
        .iter()
        .map(|&k| number(&labels[k].text).map(Value::abs))
        .collect::<Option<_>>()?;
    for a in 0..n {
        for b in 0..n {
            let (ta, tb) = (pie.arcs[a].turn, pie.arcs[b].turn);
            if (10.0 * ta > 11.0 * tb && sizes[a] < sizes[b])
                || (3.0 * ta > 4.0 * tb && sizes[a] <= sizes[b])
            {
                return None;
            }
        }
    }
    Some(values)
}

/// A chart's rows named by its legend: one for each colour, its name beside a swatch of it, its
/// cells each pie's number for the slice of that colour. A pie with two slices of one colour has
/// none.
fn by_legend(
    pies: &[Pie],
    members: &[&(usize, Vec<usize>)],
    labels: &[Label],
    marks: &[Mark],
    plot: QuantRect,
    used: &[usize],
) -> Option<Vec<Row>> {
    let mut rows: Vec<(Fill, Vec<usize>, Vec<Option<usize>>)> = Vec::new();
    for (c, m) in members.iter().enumerate() {
        let pie = &pies[m.0];
        for (a, arc) in pie.arcs.iter().enumerate() {
            if pie.arcs.iter().filter(|o| o.fill.same(arc.fill)).count() != 1 {
                return None;
            }
            match rows.iter_mut().find(|r| r.0.same(arc.fill)) {
                Some(r) => r.2[c] = Some(m.1[a]),
                None => {
                    let name = legend(arc.fill, labels, marks, plot, used)?;
                    let mut cells = vec![None; members.len()];
                    cells[c] = Some(m.1[a]);
                    rows.push((arc.fill, name, cells));
                }
            }
        }
    }
    distinct(rows.into_iter().map(|(_, n, c)| (n, c)).collect())
}

/// A chart's rows named by the lines set over each number: one for each name, its cells each pie's
/// number under it.
fn by_name_over(
    members: &[&(usize, Vec<usize>)],
    labels: &[Label],
    used: &[usize],
) -> Option<Vec<Row>> {
    let mut rows: Vec<(String, Vec<usize>, Vec<Option<usize>>)> = Vec::new();
    for (c, m) in members.iter().enumerate() {
        for &v in &m.1 {
            let name = over(labels, v, used);
            if name.is_empty() {
                return None;
            }
            let text = name
                .iter()
                .map(|&k| labels[k].text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            match rows.iter_mut().find(|r| r.0 == text) {
                Some(r) if r.2[c].is_none() => r.2[c] = Some(v),
                Some(_) => return None,
                None => {
                    let mut cells = vec![None; members.len()];
                    cells[c] = Some(v);
                    rows.push((text, name, cells));
                }
            }
        }
    }
    distinct(rows.into_iter().map(|(_, n, c)| (n, c)).collect())
}

/// The rows, where no label names two of them.
fn distinct(rows: Vec<Row>) -> Option<Vec<Row>> {
    let shared = rows.iter().enumerate().any(|(i, r)| {
        rows[..i]
            .iter()
            .any(|o| o.0.iter().any(|k| r.0.contains(k)))
    });
    (!shared).then_some(rows)
}

/// A slice's name set over its number: the lines just above it, each centred on it and the nearest
/// such, none a number.
fn over(labels: &[Label], v: usize, used: &[usize]) -> Vec<usize> {
    let mut block: Vec<usize> = Vec::new();
    let mut top = v;
    loop {
        let t = &labels[top];
        let next: Vec<usize> = (0..labels.len())
            .filter(|&k| {
                let l = &labels[k];
                k != v
                    && !used.contains(&k)
                    && !block.contains(&k)
                    && l.cy() < t.cy()
                    && 5 * (t.y0 - l.y1) <= 3 * l.h
                    && 3 * (t.y0 - l.y1) >= -l.h
                    && (l.cx() - labels[v].cx()).abs() <= l.h
                    && number(&l.text).is_none()
            })
            .collect();
        // The nearest of them.
        let Some(&k) = next.iter().max_by_key(|&&k| (labels[k].y1, k)) else {
            break;
        };
        block.push(k);
        top = k;
    }
    block.reverse();
    block
}

/// A pie's name: the label in its hole, or else the line nearest over everything it holds — its
/// labels over this pie, within four lines of `top`, and over no other pie of its chart — where
/// that line is no number but a year and ends in no full stop.
fn title(
    pie: &Pie,
    top: i64,
    others: &[QuantRect],
    labels: &[Label],
    used: &[usize],
) -> Vec<usize> {
    if let Some(k) = (0..labels.len()).find(|&k| !used.contains(&k) && pie.holds(&labels[k])) {
        return vec![k];
    }
    let (x0, x1) = (pie.centre.0 - pie.r, pie.centre.0 + pie.r);
    let above: Vec<usize> = (0..labels.len())
        .filter(|&k| {
            let l = &labels[k];
            !used.contains(&k) && l.y1 <= top && top - l.y1 <= 4 * l.h && l.x0 < x1 && x0 < l.x1
        })
        .collect();
    let Some(&first) = above
        .iter()
        .max_by_key(|&&k| (labels[k].y1, std::cmp::Reverse(k)))
    else {
        return Vec::new();
    };
    let mut line: Vec<usize> = above
        .into_iter()
        .filter(|&k| 10 * (labels[k].base - labels[first].base).abs() <= 3 * labels[first].h)
        .collect();
    line.sort_by_key(|&k| (labels[k].x0, k));
    // A number set over a pie is a callout, and a line ending in a full stop is prose: neither
    // names it.
    let text = line
        .iter()
        .map(|&k| labels[k].text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if text.ends_with('.') || (number(&text).is_some() && !is_year(&text)) {
        return Vec::new();
    }
    if line.iter().any(|&k| {
        others
            .iter()
            .any(|o| labels[k].x0 < o.x1 && o.x0 < labels[k].x1)
    }) {
        return Vec::new();
    }
    line
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

    /// A slice about `c` points, of radius `r`, from angle `from` to `to` in degrees — growing the
    /// way the page's y does — its rim a point every 45 degrees or less.
    fn slice(c: (i64, i64), r: i64, from: f64, to: f64, fill: Fill) -> Slice {
        let steps = ((to - from).abs() / 45.0).ceil().max(1.0) as usize;
        let rim = (0..=steps)
            .map(|i| {
                let a = (from + (to - from) * i as f64 / steps as f64).to_radians();
                (
                    (100.0 * (c.0 as f64 + r as f64 * a.cos())).round() as i64,
                    (100.0 * (c.1 as f64 + r as f64 * a.sin())).round() as i64,
                )
            })
            .collect();
        Slice {
            centre: (c.0 * 100, c.1 * 100),
            rim,
            fill,
        }
    }

    /// An eight-point legend square with its top-left corner at `(x, y)` points.
    fn swatch(x: i64, y: i64, colour: Fill) -> Mark {
        Mark {
            rect: QuantRect {
                x0: x * 100,
                y0: y * 100,
                x1: x * 100 + 800,
                y1: y * 100 + 800,
            },
            colour,
        }
    }

    /// A legend right of a pie about (200, 200): a swatch of each colour, a line apart, each with
    /// its name.
    fn legend_of(colours: &[Fill]) -> (Vec<TrackRun<'static>>, Vec<Mark>) {
        let names = ["Q1", "Q2", "Q3", "Q4"];
        let runs = (0..colours.len())
            .map(|i| run(312, 207 + 15 * i as i64, 20, names[i]))
            .collect();
        let marks = (0..colours.len())
            .map(|i| swatch(300, 200 + 15 * i as i64, colours[i]))
            .collect();
        (runs, marks)
    }

    /// Text of the cell at `(row, column)`.
    fn cell(t: &DetectedTable, row: u32, column: u32) -> Option<String> {
        t.cells
            .iter()
            .find(|c| c.position.row == row && c.position.column == column)
            .map(|c| c.text.clone())
    }

    /// A pie of three slices about (200, 200), each with its number on it, a title over it and a
    /// legend under it.
    fn one_pie() -> (Vec<TrackRun<'static>>, Vec<Slice>, Vec<Mark>) {
        let (blue, red, green) = (rgb(0, 0, 900), rgb(900, 0, 0), rgb(0, 900, 0));
        let slices = vec![
            slice((200, 200), 50, 0.0, 180.0, blue),
            slice((200, 200), 50, 180.0, 288.0, red),
            slice((200, 200), 50, 288.0, 360.0, green),
        ];
        let runs = vec![
            run(160, 140, 80, "Market share"),
            run(192, 234, 16, "50%"),
            run(175, 180, 16, "30%"),
            run(216, 187, 16, "20%"),
            run(130, 180, 8, "5"),
            run(162, 277, 30, "Alpha"),
            run(162, 292, 30, "Beta"),
            run(162, 302, 30, "Gamma "),
            run(162, 312, 30, "group"),
        ];
        let marks = vec![
            swatch(150, 270, blue),
            swatch(150, 285, red),
            swatch(150, 300, green),
        ];
        (runs, slices, marks)
    }

    /// **A pie printing a number on each slice is a table**: a row per slice, named by the legend
    /// beside a swatch of its colour — a name of two lines whose swatch stands between them read
    /// whole — its number in one column named by the title over the pie. A number beside the pie,
    /// farther than the slices' own, is no slice's.
    #[test]
    fn a_pie_printing_its_numbers_is_a_table() {
        let (runs, slices, marks) = one_pie();
        let t = detect(1, &runs, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        let t = &t[0];
        assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_PIES_V1);
        assert_eq!((t.rows, t.columns), (4, 2));
        assert_eq!(cell(t, 0, 1).as_deref(), Some("Market share"));
        assert_eq!(cell(t, 1, 0).as_deref(), Some("Alpha"));
        assert_eq!(cell(t, 1, 1).as_deref(), Some("50%"));
        assert_eq!(cell(t, 2, 0).as_deref(), Some("Beta"));
        assert_eq!(cell(t, 2, 1).as_deref(), Some("30%"));
        assert_eq!(cell(t, 3, 0).as_deref(), Some("Gamma group"));
        assert_eq!(cell(t, 3, 1).as_deref(), Some("20%"));
    }

    /// **A pie reads only as the table it states**: its slices turn a full circle, each carries a
    /// number of one kind — no other number stands in for a slice's — and a slice a tenth larger
    /// than another never carries a smaller one.
    #[test]
    fn a_pie_short_of_a_circle_a_number_or_its_order_is_no_table() {
        let (runs, slices, marks) = one_pie();
        // A ninth of the circle undrawn, the numbers otherwise agreeing.
        let short = vec![
            slice((200, 200), 50, 0.0, 170.0, rgb(0, 0, 900)),
            slice((200, 200), 50, 170.0, 270.0, rgb(900, 0, 0)),
            slice((200, 200), 50, 270.0, 340.0, rgb(0, 900, 0)),
        ];
        assert!(detect(1, &runs, &short, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
        // A slice bare, and a number of another kind than the slices' beside it, where that
        // slice's would stand.
        let mut far: Vec<TrackRun<'static>> = runs
            .iter()
            .filter(|r| !["20%", "5"].contains(&r.text))
            .cloned()
            .collect();
        let mut stray = far.clone();
        stray.push(run(249, 165, 8, "5"));
        assert!(detect(1, &stray, &slices, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
        // A slice bare, and a number of their kind further than the radius from the rim.
        far.push(run(320, 200, 16, "20%"));
        assert!(detect(1, &far, &slices, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
        // One slice turning the whole circle is no chart.
        let whole = vec![slice((200, 200), 50, 0.0, 360.0, rgb(0, 0, 900))];
        assert!(detect(1, &runs, &whole, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
        // The half carrying less than the fifth.
        let mut swapped = runs.clone();
        let k = swapped.iter().position(|r| r.text == "50%").expect("there");
        swapped[k].text = "10%";
        assert!(detect(1, &swapped, &slices, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
    }

    /// **A line of prose, or a number, over a pie is no title**: a sentence's last line ends in a
    /// full stop, and a number set over a pie is a callout.
    #[test]
    fn prose_or_a_number_over_a_pie_is_no_title() {
        let (runs, slices, marks) = one_pie();
        for over in [run(160, 140, 80, "Shares sold."), run(192, 140, 16, "8%")] {
            let mut runs: Vec<TrackRun<'static>> = runs
                .iter()
                .filter(|r| r.text != "Market share")
                .cloned()
                .collect();
            runs.push(over);
            let t = detect(1, &runs, &slices, &marks, &mut alloc()).expect("reads");
            assert_eq!(t.len(), 1, "{t:?}");
            assert_eq!(cell(&t[0], 0, 1), None);
            assert_eq!(cell(&t[0], 1, 1).as_deref(), Some("50%"));
        }
    }

    /// **A slice for a negative number is drawn as large as its size**, and reads so.
    #[test]
    fn a_negative_number_is_as_large_as_its_size() {
        let (runs, slices, marks) = one_pie();
        let amounts: Vec<TrackRun<'static>> = runs
            .into_iter()
            .filter(|r| r.text != "5")
            .map(|mut r| {
                r.text = match r.text {
                    "50%" => "($50)",
                    "30%" => "$30",
                    "20%" => "$20",
                    t => t,
                };
                r
            })
            .collect();
        let t = detect(1, &amounts, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 1, 1).as_deref(), Some("($50)"));
    }

    /// **A name beside two swatches names neither**: a legend that cannot say which colour its
    /// line names reads no row.
    #[test]
    fn a_name_beside_two_swatches_names_neither() {
        let (blue, red) = (rgb(0, 0, 900), rgb(900, 0, 0));
        let slices = vec![
            slice((200, 200), 50, 0.0, 216.0, blue),
            slice((200, 200), 50, 216.0, 360.0, red),
        ];
        let runs = vec![
            run(192, 234, 16, "60%"),
            run(192, 175, 16, "40%"),
            run(312, 210, 20, "Both"),
        ];
        let marks = vec![swatch(300, 200, blue), swatch(300, 210, red)];
        assert!(detect(1, &runs, &slices, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
    }

    /// **A year over a slice's name is no part of it**: set just over the lines naming a number,
    /// it names the pie.
    #[test]
    fn a_year_over_a_name_is_no_part_of_it() {
        let (orange, blue) = (rgb(1000, 600, 0), rgb(0, 0, 900));
        let slices = vec![
            slice((200, 200), 50, 330.0, 570.0, orange),
            slice((200, 200), 50, 210.0, 260.0, blue),
            slice((200, 200), 50, 260.0, 330.0, orange),
        ];
        let runs = vec![
            run(259, 226, 32, "Utilities"),
            run(267, 236, 16, "67%"),
            run(118, 169, 24, "Travel"),
            run(122, 179, 16, "14%"),
            run(171, 112, 20, "2024"),
            run(161, 122, 40, "Office costs"),
            run(173, 132, 16, "19%"),
        ];
        let t = detect(1, &runs, &slices, &[], &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 0, 1).as_deref(), Some("2024"));
        assert_eq!(cell(&t[0], 3, 0).as_deref(), Some("Office costs"));
    }

    /// **Each order clause on its own**: a slice a quarter larger carrying the smaller number, and
    /// one half as large again carrying the same.
    #[test]
    fn a_larger_slice_never_carries_a_smaller_or_the_same_number() {
        let (blue, red) = (rgb(0, 0, 900), rgb(900, 0, 0));
        let pie = |split: f64| {
            vec![
                slice((200, 200), 50, 0.0, split, blue),
                slice((200, 200), 50, split, 360.0, red),
            ]
        };
        let (names, marks) = legend_of(&[blue, red]);
        let numbers = |a: &'static str, b: &'static str| {
            let mut runs = vec![run(192, 234, 16, a), run(192, 175, 16, b)];
            runs.extend(names.iter().cloned());
            runs
        };
        // 200 and 160 degrees, a quarter larger: never the smaller number.
        assert!(
            detect(1, &numbers("40", "45"), &pie(200.0), &marks, &mut alloc())
                .expect("reads")
                .is_empty()
        );
        assert_eq!(
            detect(1, &numbers("45", "40"), &pie(200.0), &marks, &mut alloc())
                .expect("reads")
                .len(),
            1
        );
        // 220 and 140 degrees, half as large again: a larger number, not the same.
        assert!(
            detect(1, &numbers("50", "50"), &pie(220.0), &marks, &mut alloc())
                .expect("reads")
                .is_empty()
        );
    }

    /// **A number is no slice's where it stands a quarter turn from it, or fits two slices as
    /// well**: four numbers bunched over one quarter of four, and two set on the line between two
    /// halves.
    #[test]
    fn a_number_far_off_or_fitting_two_ways_is_none() {
        let colours = [
            rgb(0, 0, 900),
            rgb(900, 0, 0),
            rgb(0, 900, 0),
            rgb(900, 900, 0),
        ];
        let quarters: Vec<Slice> = (0..4)
            .map(|q| {
                slice(
                    (200, 200),
                    50,
                    90.0 * q as f64,
                    90.0 * (q + 1) as f64,
                    colours[q],
                )
            })
            .collect();
        let (names, marks) = legend_of(&colours);
        let mut bunched: Vec<TrackRun<'static>> = [100.0_f64, 120.0, 140.0, 160.0]
            .iter()
            .map(|a| {
                let (x, y) = (
                    200.0 + 30.0 * a.to_radians().cos(),
                    200.0 + 30.0 * a.to_radians().sin(),
                );
                run(x as i64 - 8, y as i64 + 3, 16, "25%")
            })
            .collect();
        bunched.extend(names.iter().cloned());
        assert!(detect(1, &bunched, &quarters, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
        let halves = vec![
            slice((200, 200), 50, 0.0, 180.0, colours[0]),
            slice((200, 200), 50, 180.0, 360.0, colours[1]),
        ];
        let mut between = vec![run(252, 203, 16, "50%"), run(132, 203, 16, "50%")];
        between.extend(names.iter().cloned());
        assert!(detect(1, &between, &halves, &marks, &mut alloc())
            .expect("reads")
            .is_empty());
    }

    /// **A slice drawn again, or within a larger one of its colour, is that slice**: a pie whose
    /// producer filled one slice twice still turns one circle.
    #[test]
    fn a_slice_drawn_again_is_one_slice() {
        let (runs, mut slices, marks) = one_pie();
        slices.push(slice((200, 200), 50, 20.0, 60.0, rgb(0, 0, 900)));
        slices.push(slices[1].clone());
        let t = detect(1, &runs, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 1, 1).as_deref(), Some("50%"));
    }

    /// **Numbers set beside a pie keep its slices' order**: on leader lines, a small slice's number
    /// stands off its slice, even over the next one, and is still its own; a slice is named by the
    /// lines set over its number — each line centred on it, the nearest first where a name's lines
    /// are set closer than their height — where the legend cannot tell two slices of one colour
    /// apart. A line more than four lines over everything the pie holds is no title.
    #[test]
    fn numbers_on_leader_lines_keep_their_slices_order() {
        let (orange, blue) = (rgb(1000, 600, 0), rgb(0, 0, 900));
        let slices = vec![
            slice((200, 200), 50, 330.0, 570.0, orange),
            slice((200, 200), 50, 210.0, 260.0, blue),
            slice((200, 200), 50, 260.0, 330.0, orange),
        ];
        let runs = vec![
            run(259, 220, 32, "Rent and "),
            run(259, 228, 32, "Utilities"),
            run(267, 236, 16, "67%"),
            run(80, 169, 30, "Note"),
            run(118, 169, 24, "Travel"),
            run(122, 179, 16, "14%"),
            run(161, 122, 40, "Office costs"),
            run(173, 132, 16, "19%"),
            run(170, 52, 60, "Annual report"),
            run(162, 287, 30, "Costs"),
            run(212, 287, 30, "Travel spend"),
        ];
        let marks = vec![swatch(150, 280, orange), swatch(200, 280, blue)];
        let t = detect(1, &runs, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        let t = &t[0];
        assert_eq!((t.rows, t.columns), (4, 2));
        assert_eq!(cell(t, 0, 1), None);
        assert_eq!(cell(t, 1, 0).as_deref(), Some("Rent and Utilities"));
        assert_eq!(cell(t, 1, 1).as_deref(), Some("67%"));
        assert_eq!(cell(t, 2, 0).as_deref(), Some("Travel"));
        assert_eq!(cell(t, 2, 1).as_deref(), Some("14%"));
        assert_eq!(cell(t, 3, 0).as_deref(), Some("Office costs"));
        assert_eq!(cell(t, 3, 1).as_deref(), Some("19%"));
    }

    /// **Pies side by side sharing a legend are one table**, a column for each named by the title
    /// over it — a year, which is never a slice's value.
    #[test]
    fn pies_side_by_side_are_one_table() {
        let (blue, red) = (rgb(0, 0, 900), rgb(900, 0, 0));
        let slices = vec![
            slice((150, 200), 40, 0.0, 216.0, blue),
            slice((150, 200), 40, 216.0, 360.0, red),
            slice((300, 200), 40, 0.0, 180.0, blue),
            slice((300, 200), 40, 180.0, 360.0, red),
        ];
        let runs = vec![
            run(140, 152, 20, "2023"),
            run(290, 152, 20, "2024"),
            run(142, 229, 16, "60%"),
            run(142, 183, 16, "40%"),
            run(292, 229, 16, "50%"),
            run(292, 183, 16, "50%"),
            run(212, 267, 30, "North"),
            run(262, 267, 30, "South"),
        ];
        let marks = vec![swatch(200, 260, blue), swatch(250, 260, red)];
        let t = detect(1, &runs, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        let t = &t[0];
        assert_eq!((t.rows, t.columns), (3, 3));
        assert_eq!(cell(t, 0, 1).as_deref(), Some("2023"));
        assert_eq!(cell(t, 0, 2).as_deref(), Some("2024"));
        assert_eq!(cell(t, 1, 0).as_deref(), Some("North"));
        assert_eq!(cell(t, 1, 1).as_deref(), Some("60%"));
        assert_eq!(cell(t, 2, 2).as_deref(), Some("50%"));
        // One line over both pies names neither.
        let mut over_both: Vec<TrackRun<'static>> = runs
            .iter()
            .filter(|r| !["2023", "2024"].contains(&r.text))
            .cloned()
            .collect();
        over_both.push(run(120, 152, 200, "Share by region"));
        let t = detect(1, &over_both, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!((cell(&t[0], 0, 1), cell(&t[0], 0, 2)), (None, None));
        // Pies of no colour in common, or at two heights, are two tables.
        let (green, gold) = (rgb(0, 900, 0), rgb(900, 900, 0));
        let mut apart = slices.clone();
        apart[2].fill = green;
        apart[3].fill = gold;
        let t = detect(1, &runs, &apart, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "the second pie has no legend: {t:?}");
        assert_eq!(t[0].columns, 2);
        let mut stacked = slices.clone();
        for s in &mut stacked[2..] {
            s.centre.1 += 6000;
            for p in &mut s.rim {
                p.1 += 6000;
            }
        }
        let mut lowered = runs.clone();
        for r in lowered.iter_mut().filter(|r| r.x == 29200) {
            r.y += 6000;
            r.rect = r.rect.map(|b| QuantRect {
                y0: b.y0 + 6000,
                y1: b.y1 + 6000,
                ..b
            });
        }
        let t = detect(1, &lowered, &stacked, &marks, &mut alloc()).expect("reads");
        assert!(t.iter().all(|t| t.columns == 2), "{t:?}");
    }

    /// **The label in a pie's hole names the pie, never a slice**: a donut's total set at its
    /// centre is its title, and the slices keep their own numbers.
    #[test]
    fn the_label_in_a_pies_hole_names_it() {
        let (runs, slices, marks) = one_pie();
        let mut runs: Vec<TrackRun<'static>> = runs
            .into_iter()
            .filter(|r| r.text != "Market share")
            .collect();
        runs.push(run(190, 204, 20, "$870"));
        let t = detect(1, &runs, &slices, &marks, &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 0, 1).as_deref(), Some("$870"));
        assert_eq!(cell(&t[0], 1, 1).as_deref(), Some("50%"));
    }
}
