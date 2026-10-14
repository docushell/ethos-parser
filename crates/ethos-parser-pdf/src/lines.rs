//! A line chart's printed values, read back as its table — `point-labels-v1` (decision #66,
//! `docs/32-CHART-LABELS-SCOPE.md` §9).
//!
//! A line chart that prints a number at each of its points states a table, as a bar chart that
//! prints its numbers does (`crate::charts`), and this rule reads it the same way: a number is
//! paired with the one point it is set at, a point with the one label under it at the foot of the
//! plot, and a series with the legend swatch of its colour. A value is never read off a point's
//! height: a point that carries no number leaves its cell empty, and a line that prints none yields
//! no table.

use ethos_parser_core::{EngineError, IdAllocator};

use crate::charts::{chart_table, is_year, labels, number, title, Label, Value};
use crate::tables::{DetectedTable, QuantRect};
use crate::text_state::Fill;
use crate::tracks::TrackRun;

/// A line the page stroked, its points in the top-left system, centipoints.
#[derive(Debug, Clone)]
pub(crate) struct Stroke {
    pub points: Vec<(i64, i64)>,
    pub colour: Fill,
}

/// A small mark the page filled — a point of a line, or a legend's swatch — top-left, centipoints.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Mark {
    pub rect: QuantRect,
    pub colour: Fill,
}

/// A point of a series: its centre, and the half-width of what marks it.
type Point = (i64, i64, i64);

/// A series that reads: its index, its rows — each a category label and a value label — and the
/// labels of the category row under it.
type Reading = (usize, Vec<(usize, usize)>, Vec<usize>);

/// The marks of one run: one colour, one width and one height, in half points.
type MarkKey = (Fill, i64, i64);

/// One series: its points left to right — each a centre and the half-width of what marks it — and
/// its colour.
#[derive(Debug)]
struct Series {
    points: Vec<Point>,
    colour: Fill,
}

/// How far a point must stand right of the one before it, and how far one point of a line must
/// stand above or below another, for the line to be a series: half a point.
const STEP: i64 = 50;

/// Two points are one where they stand within a point and a half of each other each way: a line
/// drawn through its marks.
const SAME_POINT: i64 = 150;

/// A series keeps its numbers only where six points in ten carry one.
const LABELLED_IN_TEN: usize = 6;

/// A number nearly as near another series' point is neither's: within one and a half times its
/// distance to its nearest, and a third of a point.
const AMBIGUOUS: (i64, i64, i64) = (3, 2, 30);

/// How many label lines below a plot its category row may stand.
const CATEGORY_LINES: usize = 40;

/// Past this many points on one page the rule reads nothing there: a map's or a drawing's lines,
/// never a chart that prints a number at each point.
const MAX_POINTS: usize = 200_000;

/// Every table `point-labels-v1` finds on one page, in the order of its series.
///
/// `bars` are the bars of every chart `bar-labels-v3` found on the page: a number set on a bar is
/// the bar's.
///
/// # Errors
///
/// An id the allocator cannot issue.
pub(crate) fn detect(
    page: u32,
    runs: &[TrackRun<'_>],
    strokes: &[Stroke],
    marks: &[Mark],
    bars: &[QuantRect],
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    let labels = labels(runs);
    let (series, swatches) = series(strokes, marks);
    if series.iter().map(|s| s.points.len()).sum::<usize>() > MAX_POINTS {
        return Ok(Vec::new());
    }
    let free: Vec<usize> = (0..labels.len())
        .filter(|&k| !bars.iter().any(|b| covers(*b, &labels[k])))
        .collect();
    // A year names a time, as a category does, and is never a point's value: a timeline's dates
    // set at its marks, or a year axis set along a line, read so would be a table of dates.
    let numbers: Vec<usize> = free
        .iter()
        .copied()
        .filter(|&k| number(&labels[k].text).is_some() && !is_year(&labels[k].text))
        .collect();
    let cells = assign(&series, &labels, &numbers);

    // Each series that reads: its points' numbers, and the row of labels under them.
    let mut read: Vec<Reading> = Vec::new();
    for (si, s) in series.iter().enumerate() {
        let mine: Vec<(usize, usize)> = cells
            .iter()
            .filter(|&&(cs, _, _)| cs == si)
            .map(|&(_, p, k)| (p, k))
            .collect();
        let n = s.points.len();
        if mine.len() < 2 || 10 * mine.len() < LABELLED_IN_TEN * n {
            continue;
        }
        let values: Option<Vec<(usize, Value)>> = mine
            .iter()
            .map(|&(p, k)| number(&labels[k].text).map(|v| (p, v)))
            .collect();
        let Some(values) = values else { continue };
        // The numbers agree with the drawing in order: a point clearly higher on the page carries a
        // larger number.
        let agrees = values.iter().all(|&(p, v)| {
            let h = labels[mine.iter().find(|m| m.0 == p).map_or(0, |m| m.1)].h;
            values
                .iter()
                .all(|&(q, w)| !(2 * s.points[p].1 < 2 * s.points[q].1 - h && v <= w))
        });
        if !agrees {
            continue;
        }
        let pitch = pitch(&s.points);
        let low = series
            .iter()
            .filter(|o| {
                s.points
                    .iter()
                    .all(|p| o.points.iter().any(|q| 5 * (q.0 - p.0).abs() <= pitch))
            })
            .flat_map(|o| o.points.iter().map(|p| p.1 + p.2))
            .max()
            .unwrap_or(0);
        let taken: Vec<usize> = mine.iter().map(|m| m.1).collect();
        let pool: Vec<usize> = free
            .iter()
            .copied()
            .filter(|k| !taken.contains(k))
            .collect();
        let Some(categories) = categories(&s.points, &labels, &pool, pitch, low) else {
            continue;
        };
        let rows: Vec<(usize, usize)> = mine
            .iter()
            .filter_map(|&(p, k)| categories[p].map(|c| (c, k)))
            .collect();
        let mut texts: Vec<&str> = rows.iter().map(|&(c, _)| labels[c].text.as_str()).collect();
        texts.sort_unstable();
        texts.dedup();
        if rows.len() < 2 || texts.len() != rows.len() {
            continue;
        }
        let row_of: Vec<usize> = categories.iter().flatten().copied().collect();
        read.push((si, rows, row_of));
    }

    // Series reading one row of categories are one table.
    let mut groups: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
    for (i, (_, _, row)) in read.iter().enumerate() {
        let mut key = row.clone();
        key.sort_unstable();
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(i),
            None => groups.push((key, vec![i])),
        }
    }
    let mut tables = Vec::new();
    for (_, members) in groups {
        let mut used: Vec<usize> = Vec::new();
        let mut categories: Vec<(i64, usize)> = Vec::new();
        for &m in &members {
            let (si, rows, _) = &read[m];
            for &(c, k) in rows {
                used.push(k);
                if !categories.iter().any(|&(_, o)| o == c) {
                    let x = series[*si]
                        .points
                        .iter()
                        .map(|p| p.0)
                        .min_by_key(|x| (labels[c].cx() - x).abs())
                        .unwrap_or(0);
                    categories.push((x, c));
                    used.push(c);
                }
            }
        }
        categories.sort_unstable();
        let points = members.iter().flat_map(|&m| &series[read[m].0].points);
        let plot = points.fold(None::<QuantRect>, |a, p| {
            let r = QuantRect {
                x0: p.0,
                y0: p.1,
                x1: p.0,
                y1: p.1,
            };
            Some(a.map_or(r, |a| QuantRect {
                x0: a.x0.min(r.x0),
                y0: a.y0.min(r.y0),
                x1: a.x1.max(r.x1),
                y1: a.y1.max(r.y1),
            }))
        });
        let Some(plot) = plot else { continue };
        let mut headers: Vec<Vec<usize>> = Vec::new();
        for &m in &members {
            let name = legend(series[read[m].0].colour, &labels, &swatches, plot, &used)
                .filter(|block| !headers.iter().any(|h| h.iter().any(|k| block.contains(k))))
                .unwrap_or_default();
            headers.push(name);
        }
        // A lone series with no legend takes the title over it, measured from its highest number as
        // a bar chart's is.
        if let [only] = &mut headers[..] {
            if only.is_empty() {
                let top = members
                    .iter()
                    .flat_map(|&m| read[m].1.iter().map(|r| labels[r.1].y0))
                    .fold(plot.y0, i64::min);
                only.extend(title(&labels, QuantRect { y0: top, ..plot }, &used));
            }
        }
        let rows: Vec<(Vec<usize>, Vec<Option<usize>>)> = categories
            .iter()
            .map(|&(_, c)| {
                let values = members
                    .iter()
                    .map(|&m| read[m].1.iter().find(|r| r.0 == c).map(|r| r.1))
                    .collect();
                (vec![c], values)
            })
            .collect();
        tables.push(chart_table(
            page,
            &headers,
            &rows,
            &labels,
            runs,
            alloc,
            ethos_parser_core::TABLE_DETECTION_LINES_V1,
        )?);
    }
    Ok(tables)
}

/// Whether a bar covers a fifth of a label: a number set on a bar is the bar's.
fn covers(bar: QuantRect, l: &Label) -> bool {
    let w = l.x1.min(bar.x1) - l.x0.max(bar.x0);
    let h = l.y1.min(bar.y1) - l.y0.max(bar.y0);
    let area = i128::from(l.x1 - l.x0) * i128::from(l.y1 - l.y0);
    w > 0 && h > 0 && 5 * i128::from(w) * i128::from(h) > area.max(1)
}

/// The median step between a series' points across the plot.
fn pitch(points: &[Point]) -> i64 {
    let mut steps: Vec<i64> = points.windows(2).map(|w| w[1].0 - w[0].0).collect();
    steps.sort_unstable();
    steps.get(steps.len() / 2).copied().unwrap_or(1).max(1)
}

/// The series on a page, and the swatches a legend may name them by.
///
/// A series is a stroked line of three points or more, each standing right of the one before it,
/// not all at one height, or three marks or more of one colour and one size, each standing right of
/// the one before. A line drawn over a series already read — through a run of marks, or the same
/// line drawn twice — is that series; a run of marks all on a longer line is the line. A swatch is any small mark, or a short level
/// stroke.
fn series(strokes: &[Stroke], marks: &[Mark]) -> (Vec<Series>, Vec<Mark>) {
    let mut swatches: Vec<Mark> = marks.to_vec();
    let mut lines: Vec<&Stroke> = Vec::new();
    for s in strokes {
        if let [a, b] = s.points[..] {
            let len = (b.0 - a.0).abs();
            if (a.1 - b.1).abs() < 30 && (300..=4000).contains(&len) {
                swatches.push(Mark {
                    rect: QuantRect {
                        x0: a.0.min(b.0),
                        y0: a.1 - 50,
                        x1: a.0.max(b.0),
                        y1: a.1 + 50,
                    },
                    colour: s.colour,
                });
            }
        }
        let rising = s.points.windows(2).all(|w| w[1].0 - w[0].0 > STEP);
        let level = s.points.windows(2).all(|w| (w[1].1 - w[0].1).abs() <= STEP);
        if s.points.len() >= 3 && rising && !level {
            lines.push(s);
        }
    }
    let mut runs: Vec<(MarkKey, Vec<Point>)> = Vec::new();
    for m in marks {
        let (w, h) = (m.rect.x1 - m.rect.x0, m.rect.y1 - m.rect.y0);
        let key = (m.colour, (w + 25).div_euclid(50), (h + 25).div_euclid(50));
        let point = (
            (m.rect.x0 + m.rect.x1).div_euclid(2),
            (m.rect.y0 + m.rect.y1).div_euclid(2),
            w.max(h).div_euclid(2),
        );
        match runs.iter_mut().find(|r| r.0 == key) {
            Some(r) => r.1.push(point),
            None => runs.push((key, vec![point])),
        }
    }
    let mut out: Vec<Series> = Vec::new();
    for (key, mut points) in runs {
        points.sort_unstable();
        points.dedup();
        if key.0.count != 0
            && points.len() >= 3
            && points.windows(2).all(|w| w[1].0 - w[0].0 > STEP)
        {
            out.push(Series {
                points,
                colour: key.0,
            });
        }
    }
    for line in lines {
        let points: Vec<Point> = line.points.iter().map(|&(x, y)| (x, y, 0)).collect();
        if out
            .iter()
            .any(|o| 10 * on(&points, &o.points) >= 8 * points.len())
        {
            continue; // drawn over a series already read: its marks, or itself drawn twice
        }
        out.retain(|o| {
            !(on(&o.points, &points) == o.points.len() && o.points.len() < points.len())
        });
        out.push(Series {
            points,
            colour: line.colour,
        });
    }
    (out, swatches)
}

/// How many of `a`'s points stand on one of `b`'s.
fn on(a: &[Point], b: &[Point]) -> usize {
    a.iter()
        .filter(|p| {
            b.iter()
                .any(|q| (p.0 - q.0).abs() <= SAME_POINT && (p.1 - q.1).abs() <= SAME_POINT)
        })
        .count()
}

/// How far label `l` stands from point `p` of a series `pitch` apart, in tenths of a centipoint, or
/// `None` where it is not set at the point: centred over or under it within a line and a half, or
/// beside it on its height within a line and a fifth.
fn reach(l: &Label, p: Point, pitch: i64) -> Option<i64> {
    let (x, y, r) = p;
    let dx = (l.cx() - x).abs();
    let half = (l.x1 - l.x0).div_euclid(2);
    if 10 * dx <= 10 * half + 3 * l.h && 20 * dx <= 9 * pitch {
        let over = y - r - l.y1;
        if l.cy() < y && 10 * over >= -3 * l.h && 2 * over <= 3 * l.h {
            return Some(10 * over.max(0) + dx);
        }
        let under = l.y0 - (y + r);
        if l.cy() > y && 10 * under >= -3 * l.h && 2 * under <= 3 * l.h {
            return Some(10 * under.max(0) + dx);
        }
    }
    if 10 * (l.cy() - y).abs() <= 6 * l.h {
        let right = l.x0 - (x + r);
        if right >= 0 && 10 * right <= 12 * l.h {
            return Some(10 * right);
        }
        let left = (x - r) - l.x1;
        if left >= 0 && 10 * left <= 12 * l.h {
            return Some(10 * left);
        }
    }
    None
}

/// Each number to the one point it is set at, over every series of the page: its nearest, unless
/// another series' point stands nearly as near; each point keeps its nearest number. As
/// `(series, point, label)`.
fn assign(series: &[Series], labels: &[Label], numbers: &[usize]) -> Vec<(usize, usize, usize)> {
    let pitches: Vec<i64> = series.iter().map(|s| pitch(&s.points)).collect();
    let mut best: Vec<((usize, usize), (i64, usize))> = Vec::new();
    for &k in numbers {
        let mut reached: Vec<(i64, usize, usize)> = Vec::new();
        for (si, s) in series.iter().enumerate() {
            for (pi, &p) in s.points.iter().enumerate() {
                if let Some(d) = reach(&labels[k], p, pitches[si]) {
                    reached.push((d, si, pi));
                }
            }
        }
        reached.sort_unstable();
        let Some(&(d, si, pi)) = reached.first() else {
            continue;
        };
        let (num, den, slack) = AMBIGUOUS;
        if reached[1..]
            .iter()
            .any(|o| o.1 != si && den * o.0 <= num * d + den * 10 * slack)
        {
            continue;
        }
        match best.iter_mut().find(|b| b.0 == (si, pi)) {
            Some(b) if d < b.1 .0 => b.1 = (d, k),
            Some(_) => {}
            None => best.push(((si, pi), (d, k))),
        }
    }
    best.into_iter()
        .map(|((si, pi), (_, k))| (si, pi, k))
        .collect()
}

/// The labels naming a series' points: the first line of labels below the plot's lowest point —
/// of the forty below it — holding one label under eight points in ten, no label twice and at most
/// half of them numbers but years. Indexed by point.
fn categories(
    points: &[Point],
    labels: &[Label],
    pool: &[usize],
    pitch: i64,
    low: i64,
) -> Option<Vec<Option<usize>>> {
    let mut below: Vec<usize> = pool
        .iter()
        .copied()
        .filter(|&k| labels[k].y0 >= low)
        .collect();
    below.sort_by_key(|&k| (labels[k].y0, labels[k].x0, k));
    let mut lines: Vec<Vec<usize>> = Vec::new();
    for k in below {
        match lines.last_mut() {
            Some(line) if 2 * (labels[k].y0 - labels[line[0]].y0) <= labels[k].h => line.push(k),
            _ => lines.push(vec![k]),
        }
    }
    for line in lines.iter().take(CATEGORY_LINES) {
        let got: Vec<Option<usize>> = points
            .iter()
            .map(|p| {
                let under: Vec<usize> = line
                    .iter()
                    .copied()
                    .filter(|&k| 5 * (labels[k].cx() - p.0).abs() <= 2 * pitch)
                    .collect();
                if let [k] = under[..] {
                    Some(k)
                } else {
                    None
                }
            })
            .collect();
        let found: Vec<usize> = got.iter().flatten().copied().collect();
        let mut distinct = found.clone();
        distinct.sort_unstable();
        distinct.dedup();
        let numeric = found
            .iter()
            .filter(|&&k| number(&labels[k].text).is_some() && !is_year(&labels[k].text))
            .count();
        if 10 * found.len() >= 8 * points.len()
            && 2 * numeric <= found.len()
            && distinct.len() == found.len()
        {
            return Some(got);
        }
    }
    None
}

/// A series' name: beside the nearest swatch of its colour, the block of lines starting just right
/// of it — the line that starts there and the lines of its left edge above and below it, each read
/// up to the next swatch on its row — unless another name stands almost as near.
fn legend(
    colour: Fill,
    labels: &[Label],
    swatches: &[Mark],
    plot: QuantRect,
    used: &[usize],
) -> Option<Vec<usize>> {
    if colour.count == 0 {
        return None;
    }
    let mut found: Vec<(i64, Vec<usize>, String)> = Vec::new();
    for s in swatches.iter().filter(|s| s.colour == colour) {
        let cy = (s.rect.y0 + s.rect.y1).div_euclid(2);
        let first: Vec<usize> = (0..labels.len())
            .filter(|&k| {
                let l = &labels[k];
                !used.contains(&k)
                    && l.x0 >= s.rect.x1
                    && l.x0 - s.rect.x1 <= 2 * l.h
                    && 5 * (l.cy() - cy).abs() <= 8 * l.h
            })
            .collect();
        let Some(&lead) = first.iter().min_by_key(|&&k| (labels[k].x0, k)) else {
            continue;
        };
        let x0 = labels[lead].x0;
        let stop = swatches
            .iter()
            .filter(|o| {
                o.rect.x0 > s.rect.x1
                    && ((o.rect.y0 + o.rect.y1).div_euclid(2) - cy).abs() <= 2 * labels[lead].h
            })
            .map(|o| o.rect.x0)
            .min()
            .unwrap_or(i64::MAX);
        let mut starts: Vec<usize> = (0..labels.len())
            .filter(|&k| {
                let l = &labels[k];
                2 * (l.x0 - x0).abs() <= l.h && 5 * (l.cy() - cy).abs() <= 11 * l.h
            })
            .collect();
        starts.sort_by_key(|&k| (labels[k].y0, k));
        let mut block: Vec<usize> = Vec::new();
        let mut lines: Vec<String> = Vec::new();
        for k in starts {
            let l = &labels[k];
            let mut row: Vec<usize> = (0..labels.len())
                .filter(|&o| {
                    let m = &labels[o];
                    !used.contains(&o)
                        && 10 * (m.base - l.base).abs() <= 3 * l.h
                        && 2 * m.x0 >= 2 * x0 - l.h
                        && m.x1 <= stop
                })
                .collect();
            row.sort_by_key(|&o| (labels[o].x0, o));
            let text = row
                .iter()
                .map(|&o| labels[o].text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            if !text.is_empty() && !lines.contains(&text) {
                lines.push(text);
                for o in row {
                    if !block.contains(&o) {
                        block.push(o);
                    }
                }
            }
        }
        let text = lines.join(" ");
        if block.is_empty() || (number(&text).is_some() && !is_year(&text)) {
            continue;
        }
        let (sx, sy) = ((s.rect.x0 + s.rect.x1).div_euclid(2), cy);
        let dx = (plot.x0 - sx).max(0).max(sx - plot.x1);
        let dy = (plot.y0 - sy).max(0).max(sy - plot.y1);
        let d = (dx as f64).hypot(dy as f64) as i64;
        if d > (plot.x1 - plot.x0).max(plot.y1 - plot.y0) {
            continue;
        }
        found.push((d, block, text));
    }
    found.sort_by(|a, b| (a.0, &a.2).cmp(&(b.0, &b.2)));
    let (d, block, name) = found.first()?;
    if found
        .iter()
        .any(|(od, _, other)| other != name && *od < 2 * d + 500)
    {
        return None;
    }
    Some(block.clone())
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

    /// A line through `(x, y)` points.
    fn line(points: &[(i64, i64)], colour: Fill) -> Stroke {
        Stroke {
            points: points.iter().map(|&(x, y)| (x * 100, y * 100)).collect(),
            colour,
        }
    }

    /// Text of the cell at `(row, column)`.
    fn cell(t: &DetectedTable, row: u32, column: u32) -> Option<String> {
        t.cells
            .iter()
            .find(|c| c.position.row == row && c.position.column == column)
            .map(|c| c.text.clone())
    }

    /// One line of five points rising left to right, a number over each, a year under each, and
    /// a title over the chart.
    fn one_line() -> (Vec<TrackRun<'static>>, Vec<Stroke>) {
        let points = [(100, 300), (150, 280), (200, 260), (250, 240), (300, 220)];
        let mut runs = vec![run(160, 195, 80, "Wages growth")];
        for (i, &(x, y)) in points.iter().enumerate() {
            runs.push(run(
                x - 8,
                y - 4,
                16,
                ["4.1", "4.5", "4.9", "5.3", "5.7"][i],
            ));
            runs.push(run(
                x - 10,
                340,
                20,
                ["2020", "2021", "2022", "2023", "2024"][i],
            ));
        }
        (runs, vec![line(&points, rgb(0, 0, 900))])
    }

    /// **A line printing a number at each point is a table**: a row per category under the plot,
    /// its numbers in one column named by the title over the chart.
    #[test]
    fn a_line_printing_its_numbers_is_a_table() {
        let (runs, strokes) = one_line();
        let t = detect(1, &runs, &strokes, &[], &[], &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1);
        let t = &t[0];
        assert_eq!(t.rule, ethos_parser_core::TABLE_DETECTION_LINES_V1);
        assert_eq!((t.rows, t.columns), (6, 2));
        assert_eq!(cell(t, 0, 1).as_deref(), Some("Wages growth"));
        assert_eq!(cell(t, 1, 0).as_deref(), Some("2020"));
        assert_eq!(cell(t, 1, 1).as_deref(), Some("4.1"));
        assert_eq!(cell(t, 5, 1).as_deref(), Some("5.7"));
    }

    /// **The numbers must agree with the drawing**: a point higher on the page never carries a
    /// smaller number, and six points in ten must carry one.
    #[test]
    fn numbers_that_disagree_with_the_drawing_or_leave_most_points_bare_are_no_table() {
        let (mut runs, strokes) = one_line();
        // The highest point's number made the smallest.
        let k = runs.iter().position(|r| r.text == "5.7").expect("there");
        runs[k].text = "1.0";
        assert!(detect(1, &runs, &strokes, &[], &[], &mut alloc())
            .expect("reads")
            .is_empty());
        // Three of five points bare.
        let (runs, strokes) = one_line();
        let runs: Vec<TrackRun<'static>> = runs
            .into_iter()
            .filter(|r| !["4.5", "4.9", "5.3"].contains(&r.text))
            .collect();
        assert!(detect(1, &runs, &strokes, &[], &[], &mut alloc())
            .expect("reads")
            .is_empty());
    }

    /// **Two lines sharing one row of categories are one table**, each named by the label beside a
    /// legend stroke of its colour; a number standing nearly as near the other line's point is
    /// neither's.
    #[test]
    fn two_lines_with_a_legend_are_one_table() {
        let (blue, red) = (rgb(0, 0, 900), rgb(900, 0, 0));
        let upper = [(100, 200), (150, 195), (200, 190), (250, 185), (300, 180)];
        let lower = [(100, 300), (150, 290), (200, 280), (250, 270), (300, 260)];
        let mut runs = Vec::new();
        for i in 0..5 {
            let (x, y) = upper[i];
            runs.push(run(
                x - 8,
                y - 4,
                16,
                ["8.0", "8.2", "8.4", "8.6", "8.8"][i],
            ));
            let (x, y) = lower[i];
            runs.push(run(
                x - 8,
                y + 14,
                16,
                ["2.0", "2.4", "2.8", "3.2", "3.6"][i],
            ));
            runs.push(run(
                x - 10,
                340,
                20,
                ["2020", "2021", "2022", "2023", "2024"][i],
            ));
        }
        runs.push(run(125, 384, 40, "Imports"));
        runs.push(run(225, 384, 40, "Exports"));
        let strokes = vec![
            line(&upper, blue),
            line(&lower, red),
            line(&[(100, 380), (120, 380)], blue),
            line(&[(200, 380), (220, 380)], red),
        ];
        let t = detect(1, &runs, &strokes, &[], &[], &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        let t = &t[0];
        assert_eq!((t.rows, t.columns), (6, 3));
        assert_eq!(cell(t, 0, 1).as_deref(), Some("Imports"));
        assert_eq!(cell(t, 0, 2).as_deref(), Some("Exports"));
        assert_eq!(cell(t, 3, 1).as_deref(), Some("8.4"));
        assert_eq!(cell(t, 3, 2).as_deref(), Some("2.8"));
        // A number set between the two lines' third points, nearly as near each, is neither's.
        let mut between = runs.clone();
        let k = between.iter().position(|r| r.text == "2.8").expect("there");
        between[k] = run(192, 243, 16, "2.8");
        let t = detect(1, &between, &strokes, &[], &[], &mut alloc()).expect("reads");
        assert!(t.iter().all(|t| cell(t, 3, 2).is_none()), "{t:?}");
    }

    /// **A line drawn through its marks is one series**, and a run of marks reads as a line does.
    #[test]
    fn a_line_through_its_marks_is_one_series() {
        let (runs, strokes) = one_line();
        let marks: Vec<Mark> = strokes[0]
            .points
            .iter()
            .map(|&(x, y)| Mark {
                rect: QuantRect {
                    x0: x - 200,
                    y0: y - 200,
                    x1: x + 200,
                    y1: y + 200,
                },
                colour: rgb(0, 0, 900),
            })
            .collect();
        let (s, _) = series(&strokes, &marks);
        assert_eq!(s.len(), 1);
        let t = detect(1, &runs, &[], &marks, &[], &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1);
        assert_eq!(cell(&t[0], 3, 1).as_deref(), Some("4.9"));
    }

    /// **A year set at a point is no value**: a timeline's dates at its marks name times, as
    /// categories do, and read as values they would be a table of dates.
    #[test]
    fn a_year_at_a_point_is_no_value() {
        let (runs, strokes) = one_line();
        let years = ["2007", "2010", "2013", "2016", "2019"];
        let names = ["One", "Two", "Three", "Four", "Five"];
        let runs: Vec<TrackRun<'static>> = runs
            .into_iter()
            .map(|mut r| {
                if let Some(i) = ["4.1", "4.5", "4.9", "5.3", "5.7"]
                    .iter()
                    .position(|t| *t == r.text)
                {
                    r.text = years[i];
                } else if let Some(i) = ["2020", "2021", "2022", "2023", "2024"]
                    .iter()
                    .position(|t| *t == r.text)
                {
                    r.text = names[i];
                }
                r
            })
            .collect();
        assert!(detect(1, &runs, &strokes, &[], &[], &mut alloc())
            .expect("reads")
            .is_empty());
    }

    /// **A series rises left to right and is not level**: a line that turns back on itself — a
    /// shape's outline — or one at a single height — a rule — is no series.
    #[test]
    fn a_line_that_turns_back_or_runs_level_is_no_series() {
        let blue = rgb(0, 0, 900);
        let back = line(&[(100, 300), (200, 250), (150, 200)], blue);
        let level = line(&[(100, 300), (200, 300), (300, 300)], blue);
        let rising = line(&[(100, 300), (200, 250), (300, 200)], blue);
        let (s, _) = series(&[back, level, rising.clone()], &[]);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].points.len(), rising.points.len());
    }

    /// **A number set beside its point on its height is the point's**, as one over or under it
    /// is.
    #[test]
    fn a_number_beside_its_point_is_read() {
        let points = [(100, 300), (150, 280), (200, 260), (250, 240), (300, 220)];
        let mut runs = vec![run(160, 195, 80, "Wages growth")];
        for (i, &(x, y)) in points.iter().enumerate() {
            runs.push(run(
                x + 4,
                y + 3,
                16,
                ["4.1", "4.5", "4.9", "5.3", "5.7"][i],
            ));
            runs.push(run(
                x - 10,
                340,
                20,
                ["2020", "2021", "2022", "2023", "2024"][i],
            ));
        }
        let t = detect(
            1,
            &runs,
            &[line(&points, rgb(0, 0, 900))],
            &[],
            &[],
            &mut alloc(),
        )
        .expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 2, 1).as_deref(), Some("4.5"));
    }

    /// **A line drawn twice is read once**, so its numbers are not torn between two copies of it;
    /// and **a row of numbers under the plot is no category row**: the row of labels the
    /// categories are read from holds at most half numbers but years.
    #[test]
    fn a_line_drawn_twice_is_read_once_and_numbers_name_no_category() {
        let (mut runs, strokes) = one_line();
        for (i, x) in [100, 150, 200, 250, 300].into_iter().enumerate() {
            runs.push(run(x - 8, 320, 16, ["10", "20", "30", "40", "50"][i]));
        }
        let twice = [strokes.clone(), strokes].concat();
        let t = detect(1, &runs, &twice, &[], &[], &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 1, 0).as_deref(), Some("2020"));
        assert_eq!(cell(&t[0], 1, 1).as_deref(), Some("4.1"));
    }

    /// **A number as near two lines' points is neither's**: set midway between them it stands
    /// nearly as near each, and a series that kept it would be guessing.
    #[test]
    fn a_number_as_near_two_lines_is_neither_s() {
        let upper = [(100, 195), (150, 192), (200, 190), (250, 187), (300, 185)];
        let lower = [(100, 215), (150, 212), (200, 210), (250, 207), (300, 205)];
        let mut runs = Vec::new();
        for i in 0..5 {
            runs.push(run(
                upper[i].0 - 10,
                340,
                20,
                ["2020", "2021", "2022", "2023", "2024"][i],
            ));
            if i == 2 {
                continue;
            }
            let (x, y) = upper[i];
            runs.push(run(
                x - 8,
                y - 4,
                16,
                ["8.0", "8.2", "8.4", "8.6", "8.8"][i],
            ));
            let (x, y) = lower[i];
            runs.push(run(
                x - 8,
                y + 12,
                16,
                ["2.0", "2.4", "2.8", "3.2", "3.6"][i],
            ));
        }
        // Midway between the third points: five points under the upper, five over the lower.
        runs.push(run(192, 203, 16, "5.5"));
        let strokes = vec![line(&upper, rgb(0, 0, 900)), line(&lower, rgb(900, 0, 0))];
        let t = detect(1, &runs, &strokes, &[], &[], &mut alloc()).expect("reads");
        assert_eq!(t.len(), 1, "{t:?}");
        assert_eq!(cell(&t[0], 1, 1).as_deref(), Some("8.0"));
        // The middle category carries no number in either series, so it is no row.
        assert_eq!(t[0].rows, 5, "{t:?}");
        assert!(t[0].cells.iter().all(|c| c.text != "5.5"), "{t:?}");
    }

    /// **A number set on a bar is the bar's**, never a line's.
    #[test]
    fn a_number_on_a_bar_is_no_line_value() {
        let (runs, strokes) = one_line();
        let bars: Vec<QuantRect> = [100, 150, 200, 250, 300]
            .iter()
            .map(|&x| QuantRect {
                x0: (x - 12) * 100,
                y0: 200 * 100,
                x1: (x + 12) * 100,
                y1: 330 * 100,
            })
            .collect();
        assert!(detect(1, &runs, &strokes, &[], &bars, &mut alloc())
            .expect("reads")
            .is_empty());
    }
}
