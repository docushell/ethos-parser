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

//! **Tables from whitespace tracks** — decision #38, and `docs/31-TABLE-TRACKS-SCOPE.md` §2, which
//! is the rule this file is.
//!
//! `crate::unruled` asks every cell's *origin* to sit on a shared column line, so a right-aligned
//! or centred column of numbers opens a line per width and the candidate fails its gutter floor.
//! This rule reads what such a table does share: **whitespace across its rows**. A line splits into
//! cells wherever its ink leaves a gap wider than one rendered em — except after a currency sign set
//! alone, which is the sign of the amount after it; a line of two or more cells opens a table whose
//! **tracks** are those cells' extents; and the lines below join it while their cells sit on the
//! tracks — by centre, by left edge or by right edge — each track growing to hold the cell of
//! every full row that joins it.
//!
//! # What it reads
//!
//! **Upright runs only**: a run whose measured box starts at its origin, with its baseline across
//! the box. A document number set up the margin has its box across the page from its origin, and a
//! glyph of it on a row's baseline would otherwise be a column (`docs/31-TABLE-TRACKS-SCOPE.md`
//! §5). Never a run another rule's table holds, and never on a document that declares author
//! structure — the caller's gate, because a tagged document says what is a table.
//!
//! **And inside a page's columns** (`-v4`, [`detect_in_columns`]): where the reading-order rule's
//! first vertical cut divides the page and one of the columns is prose, the rule runs again on
//! each column alone, so a table set in one column of a two-column page is read without the other
//! column's lines between its rows.
//!
//! **A superscript is part of its line.** A line set smaller than the line below it, its baseline
//! above that line's by less than half that line's em, is a footnote mark or an ordinal's letters
//! raised on that line, and joins it — on a baseline of its own it would be a line of one short
//! cell between two rows, and end the table there.
//!
//! # What makes it a table and not prose
//!
//! Each clause measured before it was written (`docs/31-TABLE-TRACKS-SCOPE.md` §3, §6):
//!
//! 1. **Enough rows for what they claim.** Two rows stand only with no empty cell, apart from the
//!    lines around them, under a first row that reads as a header — a chart's axis labels over its
//!    legend are a pair with gaps, the top of three columns of prose runs on below. Two columns need
//!    four rows, and are no table where the first column is nothing but list labels or the second
//!    nothing but rising page numbers: a list or a table of contents set in two columns.
//! 2. **A steady row pitch**: the gaps between rows vary by at most half their mean.
//! 3. **The content stream wrote it row by row** — `crate::unruled`'s rule 5, at row grain: every
//!    run of row *r* is emitted before any run of row *r* + 1. Two columns of prose are written
//!    down each column, and that is what tells them from a two-column table of the same shape.
//!    Detection runs before reading order for exactly this reason. **The clause is waived for a
//!    grid of three rows and three columns or more with a column of numbers and no column of
//!    running text** — a third or more of its cells opening lower-case, as the lines of a
//!    paragraph do. A rate table exported cell by cell is written down its columns or in no order
//!    at all; so is a page set in columns — a directory, a schedule, a newspaper in a script with
//!    no letter case — and the column of numbers is what a table of values has and those do not.
//!    **Nor is it waived where the columns divide into two groups each written row by row**: a
//!    table beside a column of text is two flows side by side, whatever the text's letters say.
//!
//! # Fabrication is impossible here, not merely avoided
//!
//! A cell's text is its runs' text concatenated in content order. No run is in two cells, and no
//! run is moved: a line whose cells do not fit the tracks — or reach across one into its
//! neighbour's — ends the table instead.

use ethos_parser_core::{EngineError, IdAllocator, IdKind};

use crate::tables::{cross_check, DetectedCell, DetectedTable, QuantRect};

/// Baselines this close, in centipoints, are one line: the leading-gap cut's own tolerance.
pub const LINE_TOLERANCE: i64 = crate::blocks::LINE_TOLERANCE;

/// How far a cell may stand from its track and still be on it, in centipoints: 6pt, at its centre,
/// its left edge or its right edge.
pub const TRACK_TOLERANCE: i64 = 600;

/// How far a run's measured box may start from its origin, in centipoints, and the run still be
/// upright: rounding, and nothing more.
pub const UPRIGHT_TOLERANCE: i64 = 5;

/// The fewest cells a line may open a table with.
pub const MIN_COLUMNS: usize = 2;

/// The fewest rows a table may have — and a table of this many rows has no empty cell, since two
/// rows are thin evidence of a column and only a complete pair stands as one.
pub const MIN_ROWS: usize = 2;

/// The fewest rows a table of two columns may have. Two columns beside each other over a few lines
/// are as often prose fragments, a short list or a near miss as a table: the engine's gold negative
/// `unruled-near-miss` is three rows of two columns, and it stays no table.
pub const MIN_ROWS_OF_TWO_COLUMNS: usize = 4;

/// The fewest rows a table the content stream did not write row by row may have.
pub const MIN_ROWS_UNORDERED: usize = 3;

/// The fewest columns a table the content stream did not write row by row may have: two columns
/// written down the page are the very shape of two columns of prose.
pub const MIN_COLUMNS_UNORDERED: usize = 3;

/// A column is running text when at least one in this many of its cells opens with a lower-case
/// letter, as a paragraph's continuation lines do. Measured, not chosen: on opendataloader-bench
/// every column of prose this rule took for a table without the row-order clause opened two in
/// five or more of its lines lower-case, and a fifth instead of a third cost ParseBench tables
/// (`docs/31-TABLE-TRACKS-SCOPE.md` §7).
pub const RUNNING_TEXT_ONE_IN: usize = 3;

/// A line of prose, for the column test of [`detect_in_columns`]: this many characters or more. A
/// line of running text in a page's column holds forty to sixty.
pub const PROSE_LINE_CHARS: usize = 30;

/// The signs a cell may hold alone and so join the amount after it.
const CURRENCY_SIGNS: &str = "$¢£¥€₹₩₽";

/// The marks a number is written with besides its digits and a currency sign: separators,
/// percent, a negative's brackets or sign, ranges, ratios, times, a note's asterisk.
const NUMBER_MARKS: &str = ".,%()+-–—/:*'";

/// One run, as the rule reads it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrackRun<'a> {
    /// Baseline origin x, top-left system, centipoints.
    pub x: i64,
    /// Baseline origin y, top-left system, centipoints.
    pub y: i64,
    /// Rendered em, or `None` where the text rendering matrix had no vertical scale.
    pub em: Option<i64>,
    /// The run's measured box: its extent along the line, and how the rule knows it is upright.
    pub rect: Option<QuantRect>,
    /// The run's text.
    pub text: &'a str,
    /// Another rule's table already holds this run's origin.
    pub claimed: bool,
}

/// A cell of one line, or of one row: an x extent and the runs in it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    x0: i64,
    x1: i64,
    runs: Vec<usize>,
}

/// One line: its first run's baseline, its em, and its cells — empty where any run of it cannot
/// be measured, which makes it no table's line.
#[derive(Debug)]
struct Line {
    y: i64,
    em: i64,
    cells: Vec<Cell>,
}

/// One row: the line that opened it, and its cells — wider than that line where a wrap joined.
#[derive(Debug)]
struct Row {
    y: i64,
    em: i64,
    cells: Vec<Cell>,
}

/// Every table `whitespace-tracks-v4` finds on one page, in reading-down order.
///
/// # Errors
///
/// An id the allocator cannot issue.
pub(crate) fn detect(
    page: u32,
    runs: &[TrackRun<'_>],
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    let lines = lines(runs);
    let mut tables = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some((rows, end)) = grow(&lines, i) {
            let pair_stands = rows.len() > MIN_ROWS || pair_plausible(&lines, i, end, &rows, runs);
            if pair_stands && accepted(&rows, runs) {
                if let Some(table) = build(page, &rows, runs, alloc)? {
                    tables.push(table);
                    i = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    Ok(tables)
}

/// The tables [`detect`] finds inside each of a page's columns — `columns` as
/// `crate::reading_order::columns` gives them, by run index — **where one of the columns is
/// prose**: three lines or more, at least half of them [`PROSE_LINE_CHARS`] characters or more.
///
/// Two columns of prose with a table in one of them interleave line by line across the gutter,
/// so the page-wide pass reads the table's rows between the other column's lines, and the
/// row-order clause rightly refuses what it makes of them; inside its own column the table is
/// rows again. A page that is one wide table cut at the gutter between its labels and its values
/// has no prose column, and stays the page-wide pass's to find or refuse.
///
/// # Errors
///
/// An id the allocator cannot issue.
pub(crate) fn detect_in_columns(
    page: u32,
    runs: &[TrackRun<'_>],
    columns: &[Vec<usize>],
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    if !columns.iter().any(|column| prose(runs, column)) {
        return Ok(Vec::new());
    }
    let mut tables = Vec::new();
    for column in columns {
        let inside: std::collections::BTreeSet<usize> = column.iter().copied().collect();
        let own: Vec<TrackRun<'_>> = runs
            .iter()
            .enumerate()
            .map(|(i, run)| TrackRun {
                claimed: run.claimed || !inside.contains(&i),
                ..*run
            })
            .collect();
        tables.extend(detect(page, &own, alloc)?);
    }
    Ok(tables)
}

/// Whether the runs of `column` are prose: three lines or more, at least half of them
/// [`PROSE_LINE_CHARS`] characters or more.
fn prose(runs: &[TrackRun<'_>], column: &[usize]) -> bool {
    let mut inked: Vec<usize> = column
        .iter()
        .copied()
        .filter(|&i| !runs[i].text.trim().is_empty())
        .collect();
    inked.sort_by_key(|&i| (runs[i].y, runs[i].x, i));
    let mut lines: Vec<usize> = Vec::new();
    let mut first = i64::MIN;
    for i in inked {
        if lines.is_empty() || (runs[i].y - first).abs() > LINE_TOLERANCE {
            lines.push(0);
            first = runs[i].y;
        }
        if let Some(chars) = lines.last_mut() {
            *chars += runs[i].text.trim().chars().count();
        }
    }
    lines.len() >= 3 && 2 * lines.iter().filter(|&&n| n >= PROSE_LINE_CHARS).count() >= lines.len()
}

/// Whether `run` is set upright, left to right: its measured box starts at its origin, runs to the
/// right of it, and its baseline crosses the box — ascent above, descent below. A run drawn up a
/// margin — a document number set vertically beside the text — has its box across the page from its
/// origin and its baseline at the box's edge, and a glyph of it on a row's baseline is no column.
/// Whitespace has no box, and passes.
///
/// **The box, not the pen advance**, because a document may state a zero advance for every glyph
/// and draw the glyphs at their font program's widths — the box is measured from what is drawn.
fn upright(run: &TrackRun<'_>) -> bool {
    if run.text.trim().is_empty() {
        return true;
    }
    run.rect.is_some_and(|b| {
        (b.x0 - run.x).abs() <= UPRIGHT_TOLERANCE && b.x1 > b.x0 && b.y0 < run.y && run.y <= b.y1
    })
}

/// The page's lines, top to bottom, from the upright runs no other rule holds.
fn lines(runs: &[TrackRun<'_>]) -> Vec<Line> {
    let mut order: Vec<usize> = (0..runs.len())
        .filter(|&i| !runs[i].claimed && upright(&runs[i]))
        .collect();
    order.sort_by_key(|&i| (runs[i].y, runs[i].x, i));
    let mut grouped: Vec<Vec<usize>> = Vec::new();
    for i in order {
        match grouped.last_mut() {
            Some(line) if (runs[i].y - runs[line[0]].y).abs() <= LINE_TOLERANCE => line.push(i),
            _ => grouped.push(vec![i]),
        }
    }
    join_superscripts(runs, grouped)
        .into_iter()
        .map(|members| line(runs, &members))
        .collect()
}

/// The page's lines with each superscript joined to the line it is raised on: a line every inked
/// run of which is set smaller than the line below it, its baseline above that line's by less
/// than half that line's em. The line below keeps its own baseline, em and first run.
fn join_superscripts(runs: &[TrackRun<'_>], grouped: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    let inked = |members: &[usize]| -> Vec<usize> {
        members
            .iter()
            .copied()
            .filter(|&i| !runs[i].text.trim().is_empty())
            .collect()
    };
    let raised_on = |line: &[usize], below: &[usize]| -> bool {
        let mut ems: Vec<i64> = inked(below).iter().filter_map(|&i| runs[i].em).collect();
        ems.sort_unstable();
        let Some(&em) = ems.get(ems.len() / 2) else {
            return false;
        };
        let small = inked(line);
        let raised = runs[below[0]].y - runs[line[0]].y;
        !small.is_empty()
            && small.iter().all(|&i| runs[i].em.is_some_and(|e| e < em))
            && raised > 0
            && 2 * raised < em
    };
    let mut out: Vec<Vec<usize>> = Vec::with_capacity(grouped.len());
    let mut lines = grouped.into_iter().peekable();
    while let Some(line) = lines.next() {
        match lines.peek_mut() {
            Some(below) if raised_on(&line, below) => below.extend(line),
            _ => out.push(line),
        }
    }
    out
}

/// One line's cells: its inked runs left to right, a new cell wherever the gap from one run's box to
/// the next run's origin is wider than the line's em. A whitespace run belongs to the cell whose
/// extent holds its origin, and opens none.
fn line(runs: &[TrackRun<'_>], members: &[usize]) -> Line {
    let y = runs[members[0]].y;
    let mut inked: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&i| !runs[i].text.trim().is_empty())
        .collect();
    inked.sort_by_key(|&i| (runs[i].x, i));
    let mut ems: Vec<i64> = Vec::with_capacity(inked.len());
    for &i in &inked {
        match runs[i].em {
            Some(em) if em > 0 => ems.push(em),
            _ => {
                return Line {
                    y,
                    em: 0,
                    cells: Vec::new(),
                }
            }
        }
    }
    ems.sort_unstable();
    let Some(&em) = ems.get(ems.len() / 2) else {
        return Line {
            y,
            em: 0,
            cells: Vec::new(),
        };
    };
    let mut cells: Vec<Cell> = Vec::new();
    for &i in &inked {
        let run = &runs[i];
        // Upright, so measured: `lines` let no other inked run through.
        let end = run.rect.map_or(run.x, |b| b.x1);
        match cells.last_mut() {
            Some(cell) if run.x - cell.x1 <= em => {
                cell.x1 = cell.x1.max(end);
                cell.runs.push(i);
            }
            _ => cells.push(Cell {
                x0: run.x,
                x1: end,
                runs: vec![i],
            }),
        }
    }
    let mut cells = join_currency_signs(runs, cells);
    for &i in members.iter().filter(|&&i| runs[i].text.trim().is_empty()) {
        let x = runs[i].x;
        if let Some(cell) = cells.iter_mut().find(|c| c.x0 <= x && x <= c.x1) {
            cell.runs.push(i);
        }
    }
    Line { y, em, cells }
}

/// A line's cells with each currency sign set alone joined to the cell after it: a financial
/// statement sets its `$` flush left in the column and the amount flush right, further apart than
/// an em, and the two are one value. A sign with no cell after it stays a cell of its own.
fn join_currency_signs(runs: &[TrackRun<'_>], cells: Vec<Cell>) -> Vec<Cell> {
    let mut out: Vec<Cell> = Vec::with_capacity(cells.len());
    let mut sign: Option<Cell> = None;
    for cell in cells {
        let cell = match sign.take() {
            Some(s) => Cell {
                x0: s.x0,
                x1: s.x1.max(cell.x1),
                runs: s.runs.into_iter().chain(cell.runs).collect(),
            },
            None => cell,
        };
        let text = cell_text(&cell, runs);
        let mut chars = text.trim().chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if CURRENCY_SIGNS.contains(c) => sign = Some(cell),
            _ => out.push(cell),
        }
    }
    out.extend(sign);
    out
}

/// Whether `cell` sits on `track`: its centre inside it, or its left or right edge on the track's,
/// each within [`TRACK_TOLERANCE`].
fn on_track(cell: &Cell, (start, end): (i64, i64)) -> bool {
    let centre2 = cell.x0 + cell.x1;
    (2 * (start - TRACK_TOLERANCE) <= centre2 && centre2 <= 2 * (end + TRACK_TOLERANCE))
        || (cell.x0 - start).abs() <= TRACK_TOLERANCE
        || (cell.x1 - end).abs() <= TRACK_TOLERANCE
}

/// Whether `cell`, on track `k`, stays clear of the tracks either side of it: it neither starts
/// before the one to its left ends nor ends after the one to its right starts. A line of prose
/// under a table is one wide cell whose centre may land on any track, and it is not a cell.
fn clear_of_neighbours(cell: &Cell, k: usize, tracks: &[(i64, i64)]) -> bool {
    let left = k.checked_sub(1).and_then(|l| tracks.get(l));
    left.is_none_or(|&(_, end)| cell.x0 > end)
        && tracks.get(k + 1).is_none_or(|&(start, _)| cell.x1 < start)
}

/// The track `cell` sits on: the nearest whose extent holds its centre, else the nearest left edge,
/// else the nearest right edge, each within [`TRACK_TOLERANCE`].
fn track_of(cell: &Cell, tracks: &[(i64, i64)]) -> Option<usize> {
    let centre2 = cell.x0 + cell.x1;
    let by_centre = tracks
        .iter()
        .enumerate()
        .filter(|(_, &(s, e))| {
            2 * (s - TRACK_TOLERANCE) <= centre2 && centre2 <= 2 * (e + TRACK_TOLERANCE)
        })
        .min_by_key(|(i, &(s, e))| ((centre2 - (s + e)).abs(), *i))
        .map(|(i, _)| i);
    by_centre
        .or_else(|| {
            tracks
                .iter()
                .enumerate()
                .filter(|(_, &(s, _))| (cell.x0 - s).abs() <= TRACK_TOLERANCE)
                .min_by_key(|(i, &(s, _))| ((cell.x0 - s).abs(), *i))
                .map(|(i, _)| i)
        })
        .or_else(|| {
            tracks
                .iter()
                .enumerate()
                .filter(|(_, &(_, e))| (cell.x1 - e).abs() <= TRACK_TOLERANCE)
                .min_by_key(|(i, &(_, e))| ((cell.x1 - e).abs(), *i))
                .map(|(i, _)| i)
        })
}

/// The rows a table opened at line `start` grows to, and the line after its last; `None` where the
/// line opens no table.
fn grow(lines: &[Line], start: usize) -> Option<(Vec<Row>, usize)> {
    let first = &lines[start];
    if first.cells.len() < MIN_COLUMNS {
        return None;
    }
    let mut tracks: Vec<(i64, i64)> = first.cells.iter().map(|c| (c.x0, c.x1)).collect();
    let mut rows = vec![Row {
        y: first.y,
        em: first.em,
        cells: first.cells.clone(),
    }];
    let mut j = start + 1;
    while let Some(line) = lines.get(j) {
        let Some(above) = rows.last() else { break };
        if line.cells.is_empty() {
            break;
        }
        // Pitches of 1.2 ems, in integers: 3.5 pitches is 21/5 em, 1.5 is 9/5.
        let em = above.em.max(line.em);
        let gap = line.y - above.y;
        if 5 * gap > 21 * em {
            break;
        }
        if line.cells.len() == tracks.len() {
            let off = line
                .cells
                .iter()
                .zip(&tracks)
                .filter(|(c, &t)| !on_track(c, t))
                .count();
            let clear = line
                .cells
                .iter()
                .enumerate()
                .all(|(k, c)| clear_of_neighbours(c, k, &tracks));
            if off > 1 || !clear {
                break;
            }
            for (track, cell) in tracks.iter_mut().zip(&line.cells) {
                *track = (track.0.min(cell.x0), track.1.max(cell.x1));
            }
            rows.push(Row {
                y: line.y,
                em: line.em,
                cells: line.cells.clone(),
            });
            j += 1;
            continue;
        }
        if line.cells.len() > tracks.len() {
            break;
        }
        let Some(mapping) = line
            .cells
            .iter()
            .map(|c| track_of(c, &tracks))
            .collect::<Option<Vec<usize>>>()
        else {
            break;
        };
        let mut distinct = mapping.clone();
        distinct.sort_unstable();
        distinct.dedup();
        let clear = line
            .cells
            .iter()
            .zip(&mapping)
            .all(|(c, &k)| clear_of_neighbours(c, k, &tracks));
        if distinct.len() != mapping.len() || !clear {
            break;
        }
        if line.cells.len() >= 2 && 5 * gap >= 9 * em {
            // A sparse row: the tracks it leaves empty are empty cells.
            let mut cells: Vec<Cell> = tracks
                .iter()
                .map(|&(x0, _)| Cell {
                    x0,
                    x1: x0,
                    runs: Vec::new(),
                })
                .collect();
            for (cell, &m) in line.cells.iter().zip(&mapping) {
                cells[m] = cell.clone();
            }
            rows.push(Row {
                y: line.y,
                em: line.em,
                cells,
            });
        } else if let Some(row) = rows.last_mut() {
            // A wrap: the line's runs join the row above, cell by cell.
            for (cell, &m) in line.cells.iter().zip(&mapping) {
                row.cells[m].runs.extend(&cell.runs);
            }
        }
        j += 1;
    }
    Some((rows, j))
}

/// Whether the rows are a table: enough of them, a steady pitch, and written row by row.
fn accepted(rows: &[Row], runs: &[TrackRun<'_>]) -> bool {
    let columns = rows[0].cells.len();
    if rows.len() < MIN_ROWS
        || (columns == 2 && rows.len() < MIN_ROWS_OF_TWO_COLUMNS)
        || (rows.len() == MIN_ROWS
            && rows
                .iter()
                .any(|r| r.cells.iter().any(|c| c.runs.is_empty())))
    {
        return false;
    }
    // Two columns, the first nothing but list labels or the second nothing but rising page
    // numbers, are a list or a table of contents set in two columns, not a table. A list's labels
    // stand on the left; a right-hand column of small numbers is a column of values.
    let column = |k: usize| -> Vec<String> {
        rows.iter()
            .map(|r| cell_text(&r.cells[k], runs))
            .filter(|t| !t.trim().is_empty())
            .collect()
    };
    if columns == 2 {
        let labels = |texts: &[String]| !texts.is_empty() && texts.iter().all(|t| is_label(t));
        if labels(&column(0)) || is_contents(&column(1)) {
            return false;
        }
    }
    // The pitch's spread at most half its mean, in integers: 4·n·Σg² ≤ 5·(Σg)².
    let gaps: Vec<i128> = rows
        .windows(2)
        .map(|w| i128::from(w[1].y - w[0].y))
        .collect();
    let n = gaps.len() as i128;
    let sum: i128 = gaps.iter().sum();
    let squares: i128 = gaps.iter().map(|g| g * g).sum();
    if sum <= 0 || 4 * n * squares > 5 * sum * sum {
        return false;
    }
    // Every run of a row emitted before any run of the next: the rows, read in content order,
    // never go back up.
    if written_row_by_row(rows, 0..columns) {
        return true;
    }
    // A grid of three rows and three columns or more with a column of numbers and no column of
    // running text needs no row order: the clause above tells prose from a table, and a table of
    // values exported cell by cell is written down its columns. **Unless its columns divide into
    // two groups each written row by row**: that is two flows side by side — a table beside a
    // column of text — whatever the text's letters say.
    rows.len() >= MIN_ROWS_UNORDERED
        && columns >= MIN_COLUMNS_UNORDERED
        && (0..columns).any(|k| numbers(&column(k)))
        && (0..columns).all(|k| !running_text(&column(k)))
        && !(1..columns)
            .any(|k| written_row_by_row(rows, 0..k) && written_row_by_row(rows, k..columns))
}

/// Whether the content stream wrote the cells of `columns` row by row: every run of a row in them
/// emitted before any run of the next.
fn written_row_by_row(rows: &[Row], columns: std::ops::Range<usize>) -> bool {
    let mut by_run: Vec<(usize, usize)> = rows
        .iter()
        .enumerate()
        .flat_map(|(r, row)| {
            row.cells[columns.clone()]
                .iter()
                .flat_map(move |c| c.runs.iter().map(move |&i| (i, r)))
        })
        .collect();
    by_run.sort_unstable();
    by_run.windows(2).all(|w| w[0].1 <= w[1].1)
}

/// Whether a table of two rows stands as one — LiteParse's two-row test, adapted: the pair stands
/// apart, with no line within a row's reach above or below it, and every cell of its first row
/// opens with a letter or digit that is no lower-case letter and ends with no comma or semicolon. The
/// top of three columns of prose — headings over the first lines of their paragraphs — fails the
/// first, since the paragraphs run on below; a line of names over its affiliation numbers fails the
/// second, its cells opening with the commas between the names.
fn pair_plausible(
    lines: &[Line],
    start: usize,
    end: usize,
    rows: &[Row],
    runs: &[TrackRun<'_>],
) -> bool {
    let within_reach = |above: &Line, below: &Line| {
        let em = above.em.max(below.em);
        5 * (below.y - above.y) <= 21 * em
    };
    let clear_above = start
        .checked_sub(1)
        .and_then(|k| lines.get(k))
        .is_none_or(|above| !within_reach(above, &lines[start]));
    let clear_below = lines
        .get(end)
        .is_none_or(|below| !within_reach(&lines[end - 1], below));
    let heads = rows[0].cells.iter().all(|c| {
        let text = cell_text(c, runs);
        let text = text.trim();
        text.chars()
            .next()
            .is_some_and(|first| first.is_alphanumeric() && !first.is_lowercase())
            && !text.ends_with([',', ';'])
    });
    clear_above && clear_below && heads
}

/// Whether a column's non-empty cells, top to bottom, are a column of numbers: at least half of
/// them hold a digit and nothing but digits, spaces, [`NUMBER_MARKS`] and [`CURRENCY_SIGNS`].
fn numbers(texts: &[String]) -> bool {
    let number = |t: &&String| {
        t.chars().any(|c| c.is_ascii_digit())
            && t.chars().all(|c| {
                c.is_ascii_digit()
                    || c.is_whitespace()
                    || NUMBER_MARKS.contains(c)
                    || CURRENCY_SIGNS.contains(c)
            })
    };
    !texts.is_empty() && 2 * texts.iter().filter(number).count() >= texts.len()
}

/// Whether a column's non-empty cells, top to bottom, are running text: at least one in
/// [`RUNNING_TEXT_ONE_IN`] of them opens with a lower-case letter.
fn running_text(texts: &[String]) -> bool {
    let lower = texts
        .iter()
        .filter(|t| {
            t.trim_start()
                .chars()
                .next()
                .is_some_and(char::is_lowercase)
        })
        .count();
    !texts.is_empty() && RUNNING_TEXT_ONE_IN * lower >= texts.len()
}

/// A cell's text: its runs' text in content order.
fn cell_text(cell: &Cell, runs: &[TrackRun<'_>]) -> String {
    let mut order = cell.runs.clone();
    order.sort_unstable();
    order.iter().map(|&i| runs[i].text).collect()
}

/// Whether `text` is a list label and nothing more: any single character — a bullet, a letter,
/// the mark an icon font draws, which may map to any letter at all — a number of up to three
/// digits, or a single letter perhaps opened by `(` and closed by `.` or `)`. A column of these is
/// a list's labels, or a column of note numbers.
fn is_label(text: &str) -> bool {
    let text = text.trim();
    let mut chars = text.chars();
    if let (Some(_), None) = (chars.next(), chars.next()) {
        return true;
    }
    let text = text.strip_prefix('(').unwrap_or(text);
    let text = text
        .strip_suffix('.')
        .or_else(|| text.strip_suffix(')'))
        .unwrap_or(text);
    ((1..=3).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_digit()))
        || (text.chars().count() == 1 && text.chars().all(|c| c.is_ascii_alphabetic()))
}

/// Whether `texts`, a column's cells top to bottom, are a table of contents' page numbers: three
/// or more, each a number of up to four digits or a roman numeral, the numbers never falling.
fn is_contents(texts: &[String]) -> bool {
    let mut last = 0u32;
    texts.len() >= 3
        && texts.iter().all(|t| {
            let t = t.trim();
            if !t.is_empty() && t.len() <= 4 && t.bytes().all(|b| b.is_ascii_digit()) {
                let page = t.parse::<u32>().unwrap_or(0);
                let rising = page >= last;
                last = page;
                rising
            } else {
                !t.is_empty() && t.chars().all(|c| "ivxlcdmIVXLCDM".contains(c))
            }
        })
}

/// The table the rows make, or `None` where a run of it has no measured box to place it by.
fn build(
    page: u32,
    rows: &[Row],
    runs: &[TrackRun<'_>],
    alloc: &mut IdAllocator,
) -> Result<Option<DetectedTable>, EngineError> {
    let columns = rows[0].cells.len();
    // Each column's and each row's extent: the union of its inked runs' boxes.
    let mut xs: Vec<Option<(i64, i64)>> = vec![None; columns];
    let mut ys: Vec<Option<(i64, i64)>> = vec![None; rows.len()];
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.cells.iter().enumerate() {
            for &i in cell
                .runs
                .iter()
                .filter(|&&i| !runs[i].text.trim().is_empty())
            {
                let Some(b) = runs[i].rect else {
                    return Ok(None);
                };
                widen(&mut xs[c], b.x0, b.x1);
                widen(&mut ys[r], b.y0, b.y1);
            }
        }
    }
    let (Some(mut xs), Some(mut ys)) = (
        xs.into_iter().collect::<Option<Vec<_>>>(),
        ys.into_iter().collect::<Option<Vec<_>>>(),
    ) else {
        return Ok(None);
    };
    // Neighbours that overlap meet halfway, so no two cells share area.
    for bands in [&mut xs, &mut ys] {
        for k in 1..bands.len() {
            if bands[k - 1].1 > bands[k].0 {
                let mid = (bands[k - 1].1 + bands[k].0).div_euclid(2);
                bands[k - 1].1 = mid;
                bands[k].0 = mid;
            }
        }
        if bands.iter().any(|&(a, b)| a >= b) {
            return Ok(None);
        }
    }
    let table_rect = QuantRect {
        x0: xs[0].0,
        y0: ys[0].0,
        x1: xs[columns - 1].1,
        y1: ys[rows.len() - 1].1,
    };
    if table_rect.as_qrect_checked().is_err() {
        return Ok(None);
    }
    let id = alloc.next(IdKind::Table)?;
    let mut cells = Vec::with_capacity(rows.len() * columns);
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.cells.iter().enumerate() {
            let mut run_indices = cell.runs.clone();
            run_indices.sort_unstable();
            let text: String = run_indices.iter().map(|&i| runs[i].text).collect();
            cells.push(DetectedCell {
                position: ethos_parser_core::TableCellPosition {
                    row: r as u32,
                    column: c as u32,
                    rowspan: 1,
                    colspan: 1,
                    table_id: id.clone(),
                },
                rect: QuantRect {
                    x0: xs[c].0,
                    y0: ys[r].0,
                    x1: xs[c].1,
                    y1: ys[r].1,
                },
                run_indices,
                text,
            });
        }
    }
    let check = cross_check(table_rect, rows.len() as u32, columns as u32, &cells);
    Ok(Some(DetectedTable {
        id,
        page,
        rect: table_rect,
        rows: rows.len() as u32,
        columns: columns as u32,
        cells,
        check,
        tagged_check: None,
        rule: ethos_parser_core::TABLE_DETECTION_TRACKS_V4.to_string(),
    }))
}

/// Grow `band` to hold `[lo, hi]`.
fn widen(band: &mut Option<(i64, i64)>, lo: i64, hi: i64) {
    *band = Some(match *band {
        Some((a, b)) => (a.min(lo), b.max(hi)),
        None => (lo, hi),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_parser_core::Profile;

    fn alloc() -> IdAllocator {
        IdAllocator::new(Profile::default().profile_sha256().expect("hashes"))
    }

    /// A 10pt run at `(x, y)` points, `w` points wide, boxed as the font envelope would box it.
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

    fn tables(runs: &[TrackRun<'_>]) -> Vec<DetectedTable> {
        detect(1, runs, &mut alloc()).expect("detects")
    }

    fn texts(table: &DetectedTable) -> Vec<Vec<String>> {
        (0..table.rows)
            .map(|r| {
                table
                    .cells
                    .iter()
                    .filter(|c| c.position.row == r)
                    .map(|c| c.text.clone())
                    .collect()
            })
            .collect()
    }

    /// A label column and two right-aligned number columns, written row by row: the numbers' left
    /// origins all differ, so no origin lines up, and the whitespace between columns is what says
    /// table.
    fn numbers() -> Vec<TrackRun<'static>> {
        vec![
            run(100, 100, 40, "Region"),
            run(240, 100, 30, "2023"),
            run(340, 100, 30, "2024"),
            run(100, 116, 40, "North"),
            run(250, 116, 20, "7.5"),
            run(330, 116, 40, "112.0"),
            run(100, 132, 40, "South"),
            run(235, 132, 35, "18.25"),
            run(350, 132, 20, "9.1"),
            run(100, 148, 40, "East"),
            run(245, 148, 25, "4.0"),
            run(340, 148, 30, "65.5"),
        ]
    }

    #[test]
    fn right_aligned_columns_are_a_table() {
        let found = tables(&numbers());
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[2], ["South", "18.25", "9.1"]);
        assert_eq!(found[0].rule, ethos_parser_core::TABLE_DETECTION_TRACKS_V4);
    }

    /// **Written down each column, a grid with a column of numbers is still a table** — a rate
    /// table exported cell by cell is — **and columns of text are not**: three columns of a
    /// paragraph's lines, a page of names set in columns, or two columns of anything, written down
    /// the page are the shape the row-order clause exists to refuse.
    #[test]
    fn columns_written_down_the_page_are_a_table_only_of_values() {
        let column_major = |mut runs: Vec<TrackRun<'static>>| {
            // Every run of the first column first, then the second, then the third.
            runs.sort_by_key(|r| (r.x / 7000, r.y));
            runs
        };
        let found = tables(&column_major(numbers()));
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[2], ["South", "18.25", "9.1"]);

        const PROSE: [[&str; 3]; 4] = [
            ["The survey was", "Respondents came", "It is important"],
            ["made available in", "from the member", "to note that the"],
            ["four languages and", "states, and the", "sample is not"],
            ["used one platform", "majority of them", "representative"],
        ];
        let prose: Vec<TrackRun<'static>> = PROSE
            .iter()
            .enumerate()
            .flat_map(|(k, line)| {
                let y = 100 + 16 * k as i64;
                [
                    run(100, y, 100, line[0]),
                    run(230, y, 100, line[1]),
                    run(360, y, 100, line[2]),
                ]
            })
            .collect();
        assert_eq!(
            tables(&prose).len(),
            1,
            "the premise: written row by row, the same lines are a grid"
        );
        assert!(tables(&column_major(prose)).is_empty());
        assert!(tables(&column_major(glossary(4))).is_empty());

        // A page set in columns with no letter case to give its prose away — here a directory of
        // names, roles and cities — has no column of numbers, and written down each column it is
        // no table either.
        const DIRECTORY: [[&str; 3]; 4] = [
            ["Douglas Smith", "Chris Qualizza", "Kim Henricks"],
            ["Regional Manager", "Sales Director", "Field Engineer"],
            ["Silverdale", "Jackson", "Elmore"],
            ["Washington", "Wisconsin", "Ohio"],
        ];
        let directory: Vec<TrackRun<'static>> = DIRECTORY
            .iter()
            .enumerate()
            .flat_map(|(k, line)| {
                let y = 100 + 16 * k as i64;
                [
                    run(100, y, 90, line[0]),
                    run(230, y, 90, line[1]),
                    run(360, y, 90, line[2]),
                ]
            })
            .collect();
        assert_eq!(
            tables(&directory).len(),
            1,
            "the premise: written row by row, the directory is a grid"
        );
        assert!(tables(&column_major(directory)).is_empty());
    }

    /// A column of prose on the left, a table on the right whose rows fall between the prose's
    /// lines, and the content stream writing the prose first: in the order the rows are given.
    fn prose_beside_a_table() -> (Vec<TrackRun<'static>>, Vec<usize>, Vec<usize>) {
        let mut runs = Vec::new();
        for k in 0..8 {
            runs.push(run(
                40,
                100 + 12 * k,
                180,
                "the paragraph runs on down this column",
            ));
        }
        let table = [
            ["Region", "2023", "2024"],
            ["North", "7.5", "112.0"],
            ["South", "18.25", "9.1"],
            ["East", "4.0", "65.5"],
        ];
        for (k, row) in table.iter().enumerate() {
            let y = 106 + 16 * k as i64;
            runs.push(run(300, y, 40, row[0]));
            runs.push(run(400, y, 30, row[1]));
            runs.push(run(480, y, 30, row[2]));
        }
        let left: Vec<usize> = (0..8).collect();
        let right: Vec<usize> = (8..runs.len()).collect();
        (runs, left, right)
    }

    /// **A table in one column of a two-column page is found inside its column** (`-v4`): read
    /// across the page its rows fall between the prose's lines and nothing stands.
    #[test]
    fn a_table_beside_a_column_of_prose_is_found_in_its_column() {
        let (runs, left, right) = prose_beside_a_table();
        assert!(
            tables(&runs).is_empty(),
            "the premise: read across, no table"
        );
        let found = detect_in_columns(1, &runs, &[left, right], &mut alloc()).expect("detects");
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[2], ["South", "18.25", "9.1"]);
    }

    /// **No prose column, no split**: a table cut at the gutter between its labels and its values
    /// is not read as two columns' worth of tables.
    #[test]
    fn columns_without_prose_are_not_read_one_by_one() {
        let (runs, _, right) = prose_beside_a_table();
        let labels: Vec<usize> = right
            .iter()
            .copied()
            .filter(|&i| runs[i].x == 30000)
            .collect();
        let values: Vec<usize> = right
            .iter()
            .copied()
            .filter(|&i| runs[i].x != 30000)
            .collect();
        let found = detect_in_columns(1, &runs, &[labels, values], &mut alloc()).expect("detects");
        assert!(found.is_empty());
    }

    /// **A table beside a column of text is two flows**, though the text gives itself away by no
    /// lower-case line: written as the table's rows and then the column top to bottom, it is no
    /// table, where the same cells written row by row are one of four columns.
    #[test]
    fn a_table_beside_a_column_of_text_is_two_flows() {
        const NOTES: [&str; 4] = [
            "Totals exclude",
            "Figures in",
            "Source: survey",
            "Revised May",
        ];
        let mut runs = numbers();
        runs.extend(
            NOTES
                .iter()
                .enumerate()
                .map(|(k, &text)| run(420, 100 + 16 * k as i64, 80, text)),
        );
        assert!(tables(&runs).is_empty());
        let mut by_row = runs.clone();
        by_row.sort_by_key(|r| (r.y, r.x));
        assert_eq!(tables(&by_row)[0].columns, 4, "the premise");
    }

    /// **A currency sign set alone is its amount's sign**: a `$` flush left in the column and the
    /// amount flush right are one cell, and the track grows to hold them both — so the next row's
    /// amount, with no sign, still sits on the column its header is centred over.
    #[test]
    fn a_currency_sign_joins_its_amount_and_a_track_grows_to_hold_them() {
        let runs = vec![
            run(100, 100, 50, "(in millions)"),
            run(235, 100, 25, "2025"),
            run(343, 100, 25, "2024"),
            run(100, 116, 80, "Normal costs"),
            run(200, 116, 5, "$"),
            run(265, 116, 25, "5,061"),
            run(310, 116, 5, "$"),
            run(375, 116, 25, "4,896"),
            run(100, 132, 80, "Amortization"),
            run(265, 132, 25, "3,122"),
            run(375, 132, 25, "3,245"),
        ];
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (3, 3));
        assert_eq!(texts(&found[0])[1], ["Normal costs", "$5,061", "$4,896"]);
        assert_eq!(texts(&found[0])[2], ["Amortization", "3,122", "3,245"]);
    }

    /// **A footnote mark raised on a row is part of that row** — set smaller and above its
    /// baseline, it is no line of its own, which would end the table or join the row above.
    #[test]
    fn a_superscript_joins_the_line_it_is_raised_on() {
        let mut runs = numbers();
        let mut mark = run(140, 113, 3, "1");
        mark.em = Some(650);
        mark.rect = Some(QuantRect {
            x0: 14000,
            y0: 10800,
            x1: 14300,
            y1: 11400,
        });
        runs.insert(4, mark);
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["Region", "2023", "2024"]);
        assert_eq!(texts(&found[0])[1], ["North1", "7.5", "112.0"]);
    }

    /// **Two rows stand only complete**: a header and one full row are a table, and the same pair
    /// with a cell missing is not — two rows are thin evidence of a column.
    #[test]
    fn two_rows_are_a_table_only_with_no_empty_cell() {
        let found = tables(&numbers()[..6]);
        assert_eq!((found[0].rows, found[0].columns), (2, 3));
        let mut gapped = numbers()[..6].to_vec();
        gapped.remove(4);
        assert!(tables(&gapped).is_empty());
    }

    /// **A pair with prose running on below it is the top of a page's columns**, not a table.
    #[test]
    fn a_pair_with_prose_running_on_below_is_no_table() {
        let mut runs = numbers()[..6].to_vec();
        runs.push(run(
            100,
            132,
            300,
            "and the paragraph runs on across the page",
        ));
        assert!(tables(&runs).is_empty());
        let mut apart = numbers()[..6].to_vec();
        apart.push(run(100, 200, 300, "a paragraph well below the pair"));
        assert_eq!(
            tables(&apart).len(),
            1,
            "the same pair with room below stands"
        );
    }

    /// **A pair whose first row is no header is no table**: a cell opening lower-case or with the
    /// comma between two names, as a line of authors over its affiliation numbers does.
    #[test]
    fn a_pair_whose_first_row_is_no_header_is_no_table() {
        for head in [", Jan Koschorreck", "region"] {
            let mut runs = numbers()[..6].to_vec();
            runs[0].text = head;
            assert!(tables(&runs).is_empty(), "{head:?}");
        }
    }

    /// Two columns of `n` rows: a term and its definition.
    fn glossary(n: usize) -> Vec<TrackRun<'static>> {
        const TERMS: [(&str, &str); 4] = [
            ("AED", "Advanced Electronic Data"),
            ("AFC", "Audit and Finance Committee"),
            ("ASC", "Accounting Standards Codification"),
            ("ASU", "Accounting Standards Update"),
        ];
        TERMS[..n]
            .iter()
            .enumerate()
            .flat_map(|(k, &(term, definition))| {
                let y = 100 + 16 * k as i64;
                [run(100, y, 30, term), run(200, y, 160, definition)]
            })
            .collect()
    }

    /// **Two columns need four rows**: a glossary of four terms is a table, of three is not.
    #[test]
    fn two_columns_need_four_rows() {
        assert!(tables(&glossary(3)).is_empty());
        let found = tables(&glossary(4));
        assert_eq!((found[0].rows, found[0].columns), (4, 2));
    }

    /// **A list set in two columns is a list**: a column of bullets or numbers beside its items —
    /// and any single character is a bullet, since an icon font's may map to any letter.
    #[test]
    fn a_column_of_list_labels_is_a_list_not_a_table() {
        for label in ["•", "1.", "(a)", "12", "Ȟ"] {
            let runs: Vec<TrackRun<'static>> = (0..4)
                .flat_map(|k| {
                    let y = 100 + 16 * k;
                    [
                        run(100, y, 8, label),
                        run(130, y, 200, "an item of the list"),
                    ]
                })
                .collect();
            assert!(tables(&runs).is_empty(), "{label:?}");
        }
    }

    /// **A table of contents is not a table**: entries beside page numbers that never fall. The
    /// same column with a number falling is a table's column of values.
    #[test]
    fn rising_page_numbers_are_contents_and_falling_ones_are_values() {
        let contents = |pages: [&'static str; 4]| -> Vec<TrackRun<'static>> {
            pages
                .iter()
                .enumerate()
                .flat_map(|(k, &page)| {
                    let y = 100 + 16 * k as i64;
                    [run(100, y, 160, "Chapter title"), run(400, y, 15, page)]
                })
                .collect()
        };
        assert!(tables(&contents(["3", "7", "7", "12"])).is_empty());
        assert!(tables(&contents(["iii", "v", "1", "9"])).is_empty());
        assert_eq!(tables(&contents(["30", "7", "12", "4"])).len(), 1);
    }

    /// A cell's second line joins it, and the row count does not grow.
    #[test]
    fn a_wrapped_cell_joins_the_row_above() {
        let mut runs = numbers();
        runs.insert(6, run(100, 124, 40, "coast"));
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[1], ["Northcoast", "7.5", "112.0"]);
    }

    /// A row a clear gap below with a cell missing is a row with an empty cell.
    #[test]
    fn a_sparse_row_leaves_its_missing_cell_empty() {
        let mut runs = numbers();
        runs.push(run(100, 168, 40, "West"));
        runs.push(run(340, 168, 30, "12.0"));
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[4], ["West", "", "12.0"]);
        let empty = &found[0].cells[13];
        assert!(empty.run_indices.is_empty() && empty.text.is_empty());
    }

    /// **A line of prose under a table is not a cell**: its one wide cell may centre on any track,
    /// and taking it as a wrap would put a sentence inside a cell.
    #[test]
    fn a_line_of_prose_under_the_table_is_not_absorbed() {
        let mut runs = numbers();
        runs.push(run(100, 160, 260, "Source: the regional statistics office"));
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        let held: Vec<usize> = found[0]
            .cells
            .iter()
            .flat_map(|c| c.run_indices.iter().copied())
            .collect();
        assert!(
            !held.contains(&12),
            "the source line stays out of the table"
        );
    }

    /// **A glyph drawn up the margin is no column.** Two columns, and on each row's baseline a
    /// character of a document number set vertically beside the text: its box runs across the
    /// page, its pen along it, and it would otherwise be a third column.
    #[test]
    fn a_margin_set_vertically_opens_no_column() {
        let mut runs = Vec::new();
        for (k, y) in [100, 116, 132, 148].into_iter().enumerate() {
            let mut glyph = run(20, y, 3, ["N", "I", "S", "T"][k]);
            glyph.rect = Some(QuantRect {
                x0: 1700,
                y0: y * 100,
                x1: 2600,
                y1: y * 100 + 300,
            });
            runs.push(glyph);
            runs.push(run(100, y, 40, "Label"));
            runs.push(run(240, y, 30, "Value"));
        }
        assert_eq!(tables(&runs)[0].columns, 2);
        runs.iter_mut().filter(|r| r.x == 2000).for_each(|r| {
            r.rect = Some(QuantRect {
                x0: r.x,
                y0: r.y - 800,
                x1: r.x + 300,
                y1: r.y + 200,
            })
        });
        assert_eq!(
            tables(&runs)[0].columns,
            3,
            "the premise: set upright, the same glyphs are a column"
        );
    }

    /// Runs another rule's table holds are not read again, and a line with a run the rule cannot
    /// measure is no table's line.
    #[test]
    fn claimed_and_unmeasured_runs_open_nothing() {
        let mut claimed = numbers();
        claimed.iter_mut().for_each(|r| r.claimed = true);
        assert!(tables(&claimed).is_empty());
        let mut unboxed = numbers();
        unboxed.iter_mut().for_each(|r| r.rect = None);
        assert!(tables(&unboxed).is_empty());
        let mut unmeasured = numbers();
        unmeasured[4].em = None;
        unmeasured[7].em = None;
        unmeasured[10].em = None;
        assert!(tables(&unmeasured).is_empty());
    }

    /// **Fabrication is impossible**: every cell's text is exactly its runs' text, in content
    /// order, and no run is in two cells.
    #[test]
    fn a_cells_text_is_its_runs_and_no_run_is_in_two() {
        let runs = numbers();
        let found = tables(&runs);
        let mut seen = std::collections::BTreeSet::new();
        for cell in &found[0].cells {
            let joined: String = cell.run_indices.iter().map(|&i| runs[i].text).collect();
            assert_eq!(cell.text, joined);
            for &i in &cell.run_indices {
                assert!(seen.insert(i), "run {i} is in two cells");
            }
        }
    }
}
