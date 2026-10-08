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
//! **A cell ends at its last inked character** (`-v5`): a run's measured box spans its trailing
//! whitespace, and a space drawn wide after a value would otherwise carry the cell across the gap
//! to the next column — the caller hands each run's extent to that point.
//!
//! **A row with cells missing is a row at the table's pitch** (`-v6`): a line with a cell on the
//! first track and another beside it opens a row however close it sits, unless its first cell
//! opens lower-case — a label's second line continues the row above. Only a line with no first
//! cell, or one closer than a row and a half's pitch with a single cell, is a wrap.
//!
//! **A header set on several lines above the first row is one row** (`-v5`, [`header_band`]): the
//! lines just above an accepted table, each within a row and a half's pitch of the line below it,
//! whose every cell sits on a track clear of its neighbours, holds fewer than
//! [`PROSE_LINE_CHARS`] characters and repeats no value its column holds below, join as the table's
//! first row — one cell per track, its lines top to bottom — where they name more than half the
//! tracks. A header's cells are set centred on a row of their own, stacked unevenly, and no line
//! of them has a cell for every column, so no line of them opens or joins the table on its own.
//!
//! **A header set off its columns is still theirs** (`-v7`, [`header_tracks`]): a header line of
//! two cells or more whose cells do not each sit on a track — a column name centred over figures
//! set flush right, a year over its column of numbers — gives each cell the track whose centre is
//! nearest its own, where they come out in order, one to a track. And the band's first line may
//! stand two rows' pitch above the table, where a rule and its padding set a header off its first
//! row; the lines above it still climb a row and a half at most. A header the table cannot be built
//! with — a cell of it, two column names closer than an em, covering a neighbour's column — is
//! none: the rows stand as they would without it.
//!
//! **Two numbers set closer than an em are two cells** (`-v8`, [`split_number_pair`]): a line the
//! table would end at, a cell of which holds two numbers — cut at the widest gap between its runs,
//! each half on a track of its own, left before right — joins as the row it is with them split. A
//! financial statement sets one year's amount a space from the next year's where its columns are
//! narrow, and the cut of a line at an em read the two as one cell reaching across two tracks, which
//! ended the table there or kept it from opening. A line that joins as it is stays as it is.
//!
//! **A line wider than the table re-tracks it** (`-v9`, [`retrack`]): a line with more cells than
//! the table has tracks, set within a row and a half's pitch of the row above it, gives the table
//! its cells as tracks, where the table holds [`RETRACK_MIN_ROWS`] rows or more, each with a digit,
//! and every cell of every row sits on one of them, a track of its own, clear of its neighbours. A
//! table opened on a row whose long cell reached across an empty column, or on rows that leave a
//! column empty, ended at its first full row. A header holds no data rows, so it is no table's
//! rows to re-track: it stays the header band's.
//!
//! **A row its wrapped cell made tall reaches past the pitch** (`-v9`): a full row more than three
//! and a half pitches below the row above it joins where it sits within them of that row's last
//! wrapped line, the row's wraps fell on some of its tracks and not all, the line is set within a
//! tenth of the row's size, and the table has [`REACH_MIN_COLUMNS`] tracks or more. A catalogue
//! whose descriptions run on over several lines ended at its first tall row. And a table its reach
//! cannot keep stands without it, as it was before the reach.
//!
//! **A cell of several numbers is as many cells** (`-v10`, [`split_number_list`]): a line the table
//! would end at, a cell of which holds numbers a word space apart or more — each on a track of its
//! own, left to right — joins with them split there; a sign set against its digits stays with them.
//! `-v8` split two at the widest gap; a table set in narrow columns runs three or four together, and
//! the table ended above its first such row.
//!
//! **Dot leaders end a cell** (`-v11`, [`ends_with_leaders`]): a run ending with three dots or more,
//! set solid or spaced, ends its cell however close the next run sets, unless that run is leaders
//! and nothing else — a leader drawn as several runs continues its label. A statement draws its
//! leaders from a label up to the first figure, within an em of it, and the label took the figure:
//! the row came a cell short, and the table did not take it. **And a sign set alone joins the figure
//! after it** in a cell of several numbers, as [`join_currency_signs`] joins it to the cell after it:
//! a statement sets its `$` flush left in columns too narrow to set it more than an em off.
//!
//! **A row of data above a table is its row** (`-v11`, [`rows_above`]): once a table stands, each
//! line above its first row, within a row and a half's pitch of the line below it, that fits its
//! tracks — as it is or with its numbers split — has two cells or more, names itself on the first
//! track and holds a number in half its cells or more joins it at the top, and the header band
//! climbs from there. A table opened at its first row whose every cell sat on a track; a row above
//! it that a leader or an empty column kept from opening it was then read as a header, and ended
//! the band. A header line leaves the first track empty, and stays the band's. **And the stub's
//! heading may run long**: a header line of two cells or more whose first cell heads the first
//! track ends no band for that cell's length — `(in millions of dollars, except ratios)`, a
//! statement's title.
//!
//! # What makes it a table and not prose
//!
//! Each clause measured before it was written (`docs/31-TABLE-TRACKS-SCOPE.md` §3, §6):
//!
//! 1. **Enough rows for what they claim.** Two rows stand only with no empty cell, apart from the
//!    lines around them, under a first row that reads as a header — a chart's axis labels over its
//!    legend are a pair with gaps, the top of three columns of prose runs on below. Two columns need
//!    four rows, and are no table where the first column is nothing but list labels or the second
//!    nothing but rising page numbers: a list or a table of contents set in two columns. And no
//!    grid whose first column is nothing but bullets is a table (`-v6`): a list's items beside the
//!    lines of whatever another flow set level with them.
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
//!    table beside a column of text is two flows side by side, whatever the text's letters say —
//!    **unless the first column alone is the one group and it is the table's labels** (`-v12`,
//!    [`label_column`]): a cell in every row, each shorter than a line of prose. A spreadsheet's
//!    export writes its labels as one block before its values row by row, and the table was
//!    refused with the flows.
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

/// The fewest rows, each holding a digit, a table must have before a line wider than it re-tracks
/// it (`-v9`): a header set on two lines — `2024 | 2023` over `€'000 | €'000` — holds digits too, and
/// is the header band's.
pub const RETRACK_MIN_ROWS: usize = 3;

/// The fewest tracks a table must have for a row its wrapped cell made tall to reach past the pitch
/// (`-v9`): a list of terms beside their definitions is two columns, wraps its definitions the same
/// way, and is no table.
pub const REACH_MIN_COLUMNS: usize = 3;

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

/// Every table `whitespace-tracks-v10` finds on one page, in reading-down order.
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
    // Lines before this one are an earlier table's, and no header of a later one.
    let mut floor = 0;
    while i < lines.len() {
        if let Some((mut rows, mut end, reach)) = grow(&lines, i, runs) {
            let stands = |rows: &[Row], end: usize| {
                (rows.len() > MIN_ROWS || pair_plausible(&lines, i, end, rows, runs))
                    && accepted(rows, runs)
            };
            let mut stand = stands(&rows, end);
            // `-v9`: a table its reach past the pitch cannot keep stands without it.
            if let (false, Some((kept, at))) = (stand, reach) {
                rows.truncate(kept);
                end = at;
                stand = stands(&rows, end);
            }
            if stand {
                let first = rows_above(&lines, floor, i, &mut rows, runs);
                let tracks = tracks_of(&rows);
                let top = header_band(&lines, floor, first, &tracks, &rows, runs);
                let mut headed = false;
                if top < first {
                    let header = header_row(&lines[top..first], &tracks);
                    let named = header.cells.iter().filter(|c| !c.runs.is_empty()).count();
                    if 2 * named > tracks.len() {
                        rows.insert(0, header);
                        headed = true;
                    }
                }
                let mut table = build(page, &rows, runs, alloc)?;
                // A header the table cannot be built with is none (`-v7`): the rows stand alone.
                if table.is_none() && headed {
                    rows.remove(0);
                    table = build(page, &rows, runs, alloc)?;
                }
                if let Some(table) = table {
                    tables.push(table);
                    i = end;
                    floor = end;
                    continue;
                }
            }
        }
        i += 1;
    }
    Ok(tables)
}

/// The first of the lines above a table's first row that are rows of it (`-v11`), each put at the
/// top of `rows`: climbing from `start` to no lower than `floor`, each line within a row and a
/// half's pitch of the line below it that fits the table's tracks — as it is, or with its numbers
/// split — has two cells or more, names itself on the first track, and holds a number in half its
/// cells or more. A header line leaves the first track empty and stays the header band's.
fn rows_above(
    lines: &[Line],
    floor: usize,
    start: usize,
    rows: &mut Vec<Row>,
    runs: &[TrackRun<'_>],
) -> usize {
    let tracks = tracks_of(rows);
    let mut first = start;
    while first > floor {
        let (line, below) = (&lines[first - 1], &lines[first]);
        // Pitches of 1.2 ems, in integers, as in `grow`: 1.5 is 9/5.
        let em = line.em.max(below.em);
        if line.cells.is_empty() || 5 * (below.y - line.y) > 9 * em {
            break;
        }
        let split;
        let line = if fits(line, &tracks) {
            line
        } else {
            split = split_numbers(line, &tracks, runs);
            if !fits(&split, &tracks) {
                break;
            }
            &split
        };
        let numbers = line
            .cells
            .iter()
            .filter(|c| is_number(cell_text(c, runs).trim()))
            .count();
        let Some(mapping) = line
            .cells
            .iter()
            .map(|c| track_of(c, &tracks))
            .collect::<Option<Vec<usize>>>()
        else {
            break;
        };
        if line.cells.len() < 2 || mapping.first() != Some(&0) || 2 * numbers < line.cells.len() {
            break;
        }
        let cells = if line.cells.len() == tracks.len() {
            line.cells.clone()
        } else {
            placed(&line.cells, &mapping, &tracks)
        };
        rows.insert(
            0,
            Row {
                y: line.y,
                em: line.em,
                cells,
            },
        );
        first -= 1;
    }
    first
}

/// A sparse line's cells on `tracks`: each at the track `mapping` gives it, every other track an
/// empty cell at its start.
fn placed(cells: &[Cell], mapping: &[usize], tracks: &[(i64, i64)]) -> Vec<Cell> {
    let mut row: Vec<Cell> = tracks
        .iter()
        .map(|&(x0, _)| Cell {
            x0,
            x1: x0,
            runs: Vec::new(),
        })
        .collect();
    for (cell, &m) in cells.iter().zip(mapping) {
        row[m] = cell.clone();
    }
    row
}

/// Each column's extent over `rows`: the union of its cells that hold runs.
fn tracks_of(rows: &[Row]) -> Vec<(i64, i64)> {
    (0..rows[0].cells.len())
        .map(|k| {
            rows.iter()
                .map(|r| &r.cells[k])
                .filter(|c| !c.runs.is_empty())
                .map(|c| (c.x0, c.x1))
                .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
                .unwrap_or((rows[0].cells[k].x0, rows[0].cells[k].x1))
        })
        .collect()
}

/// The first of the lines above a table's first row that are its header: climbing from `start`
/// to no lower than `floor`, each line within a row and a half's pitch of the line below it — the
/// first line within two rows' pitch of the table (`-v7`) — whose cells [`header_tracks`] gives a
/// track each, no two on one track, each holding fewer than [`PROSE_LINE_CHARS`] characters — a
/// caption or a sentence ends the band, though since `-v11` the stub's heading, on the first track of
/// a line of two cells or more, may run long — and repeating no value its column holds in `rows`,
/// as a row the table did not take would.
fn header_band(
    lines: &[Line],
    floor: usize,
    start: usize,
    tracks: &[(i64, i64)],
    rows: &[Row],
    runs: &[TrackRun<'_>],
) -> usize {
    let values: Vec<std::collections::BTreeSet<String>> = (0..tracks.len())
        .map(|k| {
            rows.iter()
                .map(|r| cell_text(&r.cells[k], runs).trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .collect();
    let mut top = start;
    while top > floor {
        let (line, below) = (&lines[top - 1], &lines[top]);
        // Pitches of 1.2 ems, in integers, as in `grow`: 1.5 is 9/5, and 2 is 12/5.
        let em = line.em.max(below.em);
        let reach = if top == start { 12 } else { 9 };
        if line.cells.is_empty() || 5 * (below.y - line.y) > reach * em {
            break;
        }
        let Some(mapping) = header_tracks(line, tracks) else {
            break;
        };
        let mut distinct = mapping.clone();
        distinct.sort_unstable();
        distinct.dedup();
        // `-v11`: the stub's heading, on the first track of a line of two cells or more, may run
        // long — `(in millions of dollars, except ratios)`, a statement's title.
        let stub = line.cells.len() >= 2 && mapping.first() == Some(&0);
        let header_like = line
            .cells
            .iter()
            .zip(&mapping)
            .enumerate()
            .all(|(n, (c, &k))| {
                let text = cell_text(c, runs);
                let text = text.trim();
                (text.chars().count() < PROSE_LINE_CHARS || (stub && n == 0))
                    && !values[k].contains(text)
            });
        if distinct.len() != mapping.len() || !header_like {
            break;
        }
        top -= 1;
    }
    top
}

/// The track each of `line`'s cells heads: the one it sits on, clear of its neighbours, where every
/// cell sits on one; else, for a line of two cells or more (`-v7`), the track whose centre is
/// nearest each cell's, where those come out strictly left to right. A single cell off every
/// track — a caption, a units note — heads none.
fn header_tracks(line: &Line, tracks: &[(i64, i64)]) -> Option<Vec<usize>> {
    let on: Option<Vec<usize>> = line.cells.iter().map(|c| track_of(c, tracks)).collect();
    if let Some(on) = on {
        if line
            .cells
            .iter()
            .zip(&on)
            .all(|(c, &k)| clear_of_neighbours(c, k, tracks))
        {
            return Some(on);
        }
    }
    if line.cells.len() < 2 {
        return None;
    }
    let nearest: Vec<usize> = line
        .cells
        .iter()
        .map(|c| {
            let centre2 = c.x0 + c.x1;
            (0..tracks.len())
                .min_by_key(|&k| ((tracks[k].0 + tracks[k].1 - centre2).abs(), k))
                .unwrap_or(0)
        })
        .collect();
    nearest.windows(2).all(|w| w[0] < w[1]).then_some(nearest)
}

/// A table's header row from its header `band`: each track's cells from every line of the band,
/// top to bottom; a track no line names is an empty cell.
fn header_row(band: &[Line], tracks: &[(i64, i64)]) -> Row {
    let mut cells: Vec<Cell> = tracks
        .iter()
        .map(|&(x0, _)| Cell {
            x0,
            x1: x0,
            runs: Vec::new(),
        })
        .collect();
    for line in band {
        let mapping = header_tracks(line, tracks).unwrap_or_default();
        for (c, &k) in line.cells.iter().zip(&mapping) {
            let cell = &mut cells[k];
            if cell.runs.is_empty() {
                (cell.x0, cell.x1) = (c.x0, c.x1);
            } else {
                (cell.x0, cell.x1) = (cell.x0.min(c.x0), cell.x1.max(c.x1));
            }
            cell.runs.extend(&c.runs);
        }
    }
    Row {
        y: band[0].y,
        em: band[0].em,
        cells,
    }
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
pub(crate) fn upright(run: &TrackRun<'_>) -> bool {
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
/// the next run's origin is wider than the line's em, and after a run ending in dot leaders unless
/// the next is leaders and nothing else (`-v11`). A whitespace run belongs to the cell whose extent
/// holds its origin, and opens none.
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
    let mut after_leader = false;
    for &i in &inked {
        let run = &runs[i];
        // Upright, so measured: `lines` let no other inked run through.
        let end = run.rect.map_or(run.x, |b| b.x1);
        match cells.last_mut() {
            Some(cell) if run.x - cell.x1 <= em && (!after_leader || leaders_only(run.text)) => {
                cell.x1 = cell.x1.max(end);
                cell.runs.push(i);
            }
            _ => cells.push(Cell {
                x0: run.x,
                x1: end,
                runs: vec![i],
            }),
        }
        after_leader = ends_with_leaders(run.text);
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

/// Whether `text` ends with dot leaders (`-v11`): three dots or more, set solid or spaced, or an
/// ellipsis — what a statement draws from a label to its first figure.
fn ends_with_leaders(text: &str) -> bool {
    text.trim_end()
        .chars()
        .rev()
        .take_while(|&c| c == '.' || c == ' ' || c == '\u{2026}')
        .filter(|&c| c != ' ')
        .count()
        >= 3
}

/// Whether `text` is dot leaders and nothing else (`-v11`): a leader drawn as several runs.
fn leaders_only(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text
            .chars()
            .all(|c| c == '.' || c == ' ' || c == '\u{2026}')
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

/// The rows a table opened at line `start` grows to, the line after its last, and where its first
/// reach past the pitch began (`-v9`) — the rows before it, and its line; `None` where the line
/// opens no table.
fn grow(lines: &[Line], start: usize, runs: &[TrackRun<'_>]) -> Option<Grown> {
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
    // `-v9`: the baseline of the last line taken, row or wrap; the tracks the current row's wrapped
    // lines fell on; and where the first reach past the pitch began.
    let mut last = first.y;
    let mut wrapped: Vec<usize> = Vec::new();
    let mut reach: Option<(usize, usize)> = None;
    while let Some(line) = lines.get(j) {
        let Some(above) = rows.last() else { break };
        if line.cells.is_empty() {
            break;
        }
        // Pitches of 1.2 ems, in integers: 3.5 pitches is 21/5 em, 1.5 is 9/5.
        let em = above.em.max(line.em);
        let gap = line.y - above.y;
        if 5 * gap > 21 * em {
            // `-v9`: a row its wrapped cell made tall reaches as far below its last wrapped line,
            // for a full row of its size; a tenth of the size is the bin a row's own type keeps.
            let reaches = tracks.len() >= REACH_MIN_COLUMNS
                && !wrapped.is_empty()
                && wrapped.len() < tracks.len()
                && 5 * (line.y - last) <= 21 * em
                && 10 * (line.em - above.em).abs() <= above.em
                && line.cells.len() == tracks.len()
                && fits(line, &tracks);
            if !reaches {
                break;
            }
            reach.get_or_insert((rows.len(), j));
        }
        // `-v8`: a line the table would end at joins with its cells of two numbers split, where
        // that makes it a row.
        let split;
        let line = if fits(line, &tracks) {
            line
        } else {
            split = split_numbers(line, &tracks, runs);
            if fits(&split, &tracks) {
                &split
            } else {
                line
            }
        };
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
            last = line.y;
            wrapped.clear();
            j += 1;
            continue;
        }
        if line.cells.len() > tracks.len() {
            // `-v9`: a line wider than the table, set at the row pitch under rows of data, re-tracks
            // it where every row sits on the line's cells.
            let wider: Vec<(i64, i64)> = line.cells.iter().map(|c| (c.x0, c.x1)).collect();
            let data = rows.len() >= RETRACK_MIN_ROWS
                && rows.iter().all(|r| {
                    r.cells
                        .iter()
                        .any(|c| cell_text(c, runs).chars().any(|ch| ch.is_ascii_digit()))
                });
            let Some(retracked) = (data && 5 * gap <= 9 * em)
                .then(|| retrack(&rows, &wider))
                .flatten()
            else {
                break;
            };
            rows = retracked;
            tracks = wider;
            rows.push(Row {
                y: line.y,
                em: line.em,
                cells: line.cells.clone(),
            });
            last = line.y;
            wrapped.clear();
            j += 1;
            continue;
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
        // `-v6`: a line with a cell on the first track and another beside it opens a row at any
        // gap, unless its first cell opens lower-case — a label's second line continues it.
        let opens_row = mapping.len() >= 2
            && mapping.first() == Some(&0)
            && !cell_text(&line.cells[0], runs)
                .trim_start()
                .chars()
                .next()
                .is_some_and(char::is_lowercase);
        if line.cells.len() >= 2 && (5 * gap >= 9 * em || opens_row) {
            // A sparse row: the tracks it leaves empty are empty cells.
            rows.push(Row {
                y: line.y,
                em: line.em,
                cells: placed(&line.cells, &mapping, &tracks),
            });
            wrapped.clear();
        } else if let Some(row) = rows.last_mut() {
            // A wrap: the line's runs join the row above, cell by cell.
            for (cell, &m) in line.cells.iter().zip(&mapping) {
                row.cells[m].runs.extend(&cell.runs);
                if !wrapped.contains(&m) {
                    wrapped.push(m);
                }
            }
        }
        last = line.y;
        j += 1;
    }
    Some((rows, j, reach))
}

/// What [`grow`] returns: the rows, the line after the last, and where the first reach past the
/// pitch began — the rows before it, and its line.
type Grown = (Vec<Row>, usize, Option<(usize, usize)>);

/// `rows` on `tracks`, a wider line's cells (`-v9`): each cell holding runs on a track of its own,
/// clear of its neighbours; `None` where a row's cells do not sit so.
fn retrack(rows: &[Row], tracks: &[(i64, i64)]) -> Option<Vec<Row>> {
    let mut out = Vec::with_capacity(rows.len() + 1);
    for row in rows {
        let mut cells: Vec<Cell> = tracks
            .iter()
            .map(|&(x0, _)| Cell {
                x0,
                x1: x0,
                runs: Vec::new(),
            })
            .collect();
        for cell in row.cells.iter().filter(|c| !c.runs.is_empty()) {
            let k = track_of(cell, tracks)?;
            if !cells[k].runs.is_empty() || !clear_of_neighbours(cell, k, tracks) {
                return None;
            }
            cells[k] = cell.clone();
        }
        out.push(Row {
            y: row.y,
            em: row.em,
            cells,
        });
    }
    Some(out)
}

/// Whether `line` joins rows on `tracks` as [`grow`] reads them: a full line, one cell at most off
/// its track, or a sparse one, each cell on a track of its own — every cell clear of its
/// neighbours.
fn fits(line: &Line, tracks: &[(i64, i64)]) -> bool {
    if line.cells.is_empty() || line.cells.len() > tracks.len() {
        return false;
    }
    if line.cells.len() == tracks.len() {
        let off = line
            .cells
            .iter()
            .zip(tracks)
            .filter(|(c, &t)| !on_track(c, t))
            .count();
        return off <= 1
            && line
                .cells
                .iter()
                .enumerate()
                .all(|(k, c)| clear_of_neighbours(c, k, tracks));
    }
    let Some(mapping) = line
        .cells
        .iter()
        .map(|c| track_of(c, tracks))
        .collect::<Option<Vec<usize>>>()
    else {
        return false;
    };
    let mut distinct = mapping.clone();
    distinct.sort_unstable();
    distinct.dedup();
    distinct.len() == mapping.len()
        && line
            .cells
            .iter()
            .zip(&mapping)
            .all(|(c, &k)| clear_of_neighbours(c, k, tracks))
}

/// `line` with each cell that [`split_number_list`] reads as numbers on tracks of their own (`-v10`),
/// or failing that [`split_number_pair`] as two (`-v8`), split into them.
fn split_numbers(line: &Line, tracks: &[(i64, i64)], runs: &[TrackRun<'_>]) -> Line {
    let mut cells = Vec::with_capacity(line.cells.len() + 1);
    for cell in &line.cells {
        if let Some(numbers) = split_number_list(cell, line.em, tracks, runs) {
            cells.extend(numbers);
            continue;
        }
        match split_number_pair(cell, tracks, runs) {
            Some((left, right)) => {
                cells.push(left);
                cells.push(right);
            }
            None => cells.push(cell.clone()),
        }
    }
    Line {
        y: line.y,
        em: line.em,
        cells,
    }
}

/// `cell` cut at every gap between its inked runs wider than a fifth of `em` — a word space — where
/// that makes two pieces or more, each a number on a track, the tracks left to right (`-v10`); `None`
/// where it does not. A sign set against its digits stays with them, and since `-v11` a sign set
/// alone joins the piece after it, as [`join_currency_signs`] joins it to the cell after it.
fn split_number_list(
    cell: &Cell,
    em: i64,
    tracks: &[(i64, i64)],
    runs: &[TrackRun<'_>],
) -> Option<Vec<Cell>> {
    let mut order = cell.runs.clone();
    order.sort_by_key(|&i| (runs[i].x, i));
    let end = |i: usize| runs[i].rect.map_or(runs[i].x, |b| b.x1);
    let mut pieces: Vec<Cell> = Vec::new();
    // The furthest any inked run so far reaches: a gap is measured from it.
    let mut reach: Option<i64> = None;
    for &i in &order {
        let inked = !runs[i].text.trim().is_empty();
        match pieces.last_mut() {
            Some(piece) if !inked || reach.is_some_and(|r| 5 * (runs[i].x - r) <= em) => {
                if inked {
                    piece.x1 = piece.x1.max(end(i));
                }
                piece.runs.push(i);
            }
            _ if inked => pieces.push(Cell {
                x0: runs[i].x,
                x1: end(i),
                runs: vec![i],
            }),
            _ => {}
        }
        if inked {
            reach = Some(reach.map_or(end(i), |r| r.max(end(i))));
        }
    }
    let pieces = join_currency_signs(runs, pieces);
    if pieces.len() < 2 {
        return None;
    }
    let mut previous: Option<usize> = None;
    for piece in &pieces {
        let k = track_of(piece, tracks)?;
        if previous.is_some_and(|p| p >= k) || !is_number(&cell_text(piece, runs)) {
            return None;
        }
        previous = Some(k);
    }
    Some(pieces)
}

/// `cell` as two cells, cut at the widest gap between its inked runs, where each half is a number on
/// a track, the left half's track before the right's; `None` where it is not that. A half in the
/// gutter between two tracks is no track's value, though a full line may hold one cell off its
/// track.
fn split_number_pair(
    cell: &Cell,
    tracks: &[(i64, i64)],
    runs: &[TrackRun<'_>],
) -> Option<(Cell, Cell)> {
    let mut order = cell.runs.clone();
    order.sort_by_key(|&i| (runs[i].x, i));
    let inked = |i: &usize| !runs[*i].text.trim().is_empty();
    let end = |i: usize| runs[i].rect.map_or(runs[i].x, |b| b.x1);
    let ink: Vec<usize> = order.iter().copied().filter(inked).collect();
    let (gap, at) = ink
        .windows(2)
        .map(|w| (runs[w[1]].x - end(w[0]), runs[w[1]].x))
        .max()?;
    if gap <= 0 {
        return None;
    }
    let half = |left: bool| -> Option<Cell> {
        let members: Vec<usize> = order
            .iter()
            .copied()
            .filter(|&i| (runs[i].x < at) == left)
            .collect();
        let x0 = members
            .iter()
            .filter(|i| inked(i))
            .map(|&i| runs[i].x)
            .min()?;
        let x1 = members.iter().filter(|i| inked(i)).map(|&i| end(i)).max()?;
        Some(Cell {
            x0,
            x1,
            runs: members,
        })
    };
    let (left, right) = (half(true)?, half(false)?);
    let ordered = track_of(&left, tracks)? < track_of(&right, tracks)?;
    (ordered && is_number(&cell_text(&left, runs)) && is_number(&cell_text(&right, runs)))
        .then_some((left, right))
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
    // A first column of nothing but bullets is a list, whatever stands beside it (`-v6`): its items,
    // and the lines of whatever the next flow set level with them.
    let bullet = |t: &String| {
        let mut chars = t.trim().chars();
        matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_alphanumeric())
    };
    let first = column(0);
    if !first.is_empty() && first.iter().all(bullet) {
        return false;
    }
    if columns == 2 {
        let labels = |texts: &[String]| !texts.is_empty() && texts.iter().all(|t| is_label(t));
        if labels(&column(0)) || is_contents(&column(1)) {
            return false;
        }
    }
    // `-v11`: titles drawn with dot leaders, then a column of numbers, are a table of contents at
    // any width — the leaders now end the title's cell, so its page number is a column of its own.
    // A statement drawn with leaders sets several columns of figures after its labels, and a
    // schedule leads its years or ranges, not titles, to its figures.
    let titles = column(columns - 2);
    if !titles.is_empty()
        && titles
            .iter()
            .all(|t| ends_with_leaders(t) && t.chars().any(char::is_alphabetic))
        && column(columns - 1).iter().all(|t| is_number(t.trim()))
    {
        return false;
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
        && !(1..columns).any(|k| {
            written_row_by_row(rows, 0..k)
                && written_row_by_row(rows, k..columns)
                && !(k == 1 && label_column(rows, runs))
        })
}

/// Whether the first column is the table's labels (`-v12`): a cell in every row, each shorter than
/// a line of prose. A spreadsheet's export writes its labels as one block before its values row by
/// row, the order of a column of text beside a table; a column of text runs its lines long, and a
/// label column names every row.
fn label_column(rows: &[Row], runs: &[TrackRun<'_>]) -> bool {
    rows.iter().all(|r| {
        let text = cell_text(&r.cells[0], runs);
        let text = text.trim();
        !text.is_empty() && text.chars().count() < PROSE_LINE_CHARS
    })
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
    !texts.is_empty() && 2 * texts.iter().filter(|t| is_number(t)).count() >= texts.len()
}

/// Whether `text` is a number: a digit, and nothing but digits, spaces, [`NUMBER_MARKS`] and
/// [`CURRENCY_SIGNS`].
fn is_number(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit())
        && text.chars().all(|c| {
            c.is_ascii_digit()
                || c.is_whitespace()
                || NUMBER_MARKS.contains(c)
                || CURRENCY_SIGNS.contains(c)
        })
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
        rule: ethos_parser_core::TABLE_DETECTION_TRACKS_V12.to_string(),
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

    /// Four rows of values, labels and two numbers, the first at 200 points: no header of their own.
    fn values() -> Vec<TrackRun<'static>> {
        vec![
            run(100, 200, 40, "North"),
            run(250, 200, 20, "7.5"),
            run(330, 200, 40, "112.0"),
            run(100, 216, 40, "South"),
            run(235, 216, 35, "18.25"),
            run(350, 216, 20, "9.1"),
            run(100, 232, 40, "East"),
            run(245, 232, 25, "4.0"),
            run(340, 232, 30, "65.5"),
            run(100, 248, 40, "West"),
            run(240, 248, 30, "12.0"),
            run(335, 248, 35, "70.25"),
        ]
    }

    /// `band` set above [`values`]: the table's rows, below the band in content order too.
    fn under(band: Vec<TrackRun<'static>>) -> Vec<DetectedTable> {
        tables(&band.into_iter().chain(values()).collect::<Vec<_>>())
    }

    /// **A header set on several lines above the rows is one row**: `Line` and `Type` on one
    /// baseline, `Most Recent` above and `Year` below it over the third column — no line of it a
    /// cell for every column — read as the table's first row, each column's lines top to bottom.
    #[test]
    fn a_header_set_on_several_lines_above_the_rows_is_one_row() {
        let found = under(vec![
            run(325, 170, 50, "Most Recent "),
            run(105, 178, 25, "Line"),
            run(240, 178, 25, "Type"),
            run(340, 186, 20, "Year"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[0], ["Line", "Type", "Most Recent Year"]);
        assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0"]);
    }

    /// **The band is a header's and nothing else's**: more than a row and a half above the rows it
    /// is apart from them; a line naming one column of three is a title, not a header; a cell of
    /// thirty characters is a caption; a cell repeating a value its column holds is a row the
    /// table did not take. Each leaves the table as it was.
    #[test]
    fn a_band_apart_titled_captioned_or_repeating_is_no_header() {
        for (band, why) in [
            (
                vec![run(105, 170, 25, "Line"), run(240, 170, 25, "Type")],
                "thirty points above the first row",
            ),
            (vec![run(340, 186, 20, "Year")], "one column of three named"),
            (
                vec![run(105, 186, 30, "Regional rainfall by quarter, mm")],
                "a caption",
            ),
            (
                vec![
                    run(105, 186, 25, "Region"),
                    run(240, 186, 30, "Rainfall by quarter, in millimetres"),
                ],
                "a long cell past the stub",
            ),
            (
                vec![run(100, 186, 40, "North"), run(240, 186, 25, "Type")],
                "a value of the first column",
            ),
        ] {
            let found = under(band);
            assert_eq!((found[0].rows, found[0].columns), (4, 3), "{why}");
            assert_eq!(texts(&found[0])[0], ["North", "7.5", "112.0"], "{why}");
        }
    }

    /// **A header set off its columns is still theirs** (`-v7`): `Units` and `Price`, set left of
    /// their columns of figures, on no track of their own, head the columns whose centres are
    /// nearest theirs; two cells nearest one column head nothing.
    #[test]
    fn a_header_set_off_its_columns_is_still_theirs() {
        let found = under(vec![
            run(100, 186, 30, "Region"),
            run(205, 186, 30, "Units"),
            run(300, 186, 30, "Price"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[0], ["Region", "Units", "Price"]);
        let found = under(vec![run(205, 186, 20, "Units"), run(228, 186, 20, "Count")]);
        assert_eq!(
            (found[0].rows, found[0].columns),
            (4, 3),
            "both nearest the second column"
        );
    }

    /// **A header's first line may stand two rows' pitch above the table** (`-v7`), where a rule and
    /// its padding set it off: 20 points over 10-point rows. A line above it still joins only within
    /// a row and a half's pitch, and a single cell on no track — a units note — heads nothing.
    #[test]
    fn a_header_two_rows_pitch_above_is_the_table_s_and_no_further() {
        let found = under(vec![run(105, 180, 25, "Line"), run(240, 180, 25, "Type")]);
        assert_eq!(texts(&found[0])[0], ["Line", "Type", ""]);
        let found = under(vec![
            run(105, 160, 25, "Unit"),
            run(105, 180, 25, "Line"),
            run(240, 180, 25, "Type"),
        ]);
        assert_eq!(
            texts(&found[0])[0],
            ["Line", "Type", ""],
            "a second line 20 points above"
        );
        let found = under(vec![
            run(160, 172, 40, "(in millions)"),
            run(105, 186, 25, "Line"),
            run(240, 186, 25, "Type"),
        ]);
        assert_eq!(texts(&found[0])[0], ["Line", "Type", ""]);
    }

    /// **A header the table cannot be built with is none** (`-v7`): `Units and price`, nearest the
    /// third column, covers the second, so no cell of the second could be told from it; the table
    /// is its four rows, every one of them, as with no header at all.
    #[test]
    fn a_header_covering_a_neighbour_s_column_leaves_the_rows_whole() {
        let found = under(vec![
            run(105, 186, 25, "Line"),
            run(190, 186, 240, "Units and price"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["North", "7.5", "112.0"]);
    }

    /// Four rows of amounts whose `South` row sets its two amounts 5 points apart — closer than an
    /// em, so one cell reaching from the second column into the third — or two words the same way.
    fn amounts(left: &'static str, right: &'static str) -> Vec<TrackRun<'static>> {
        vec![
            run(100, 100, 40, "Region"),
            run(240, 100, 30, "2023"),
            run(340, 100, 30, "2024"),
            run(100, 116, 40, "North"),
            run(250, 116, 20, "7.5"),
            run(330, 116, 40, "112.0"),
            run(100, 132, 40, "South"),
            run(228, 132, 42, left),
            run(275, 132, 95, right),
            run(100, 148, 40, "East"),
            run(245, 148, 25, "4.0"),
            run(340, 148, 30, "65.5"),
        ]
    }

    /// **Two numbers set closer than an em are two cells** (`-v8`): `South`'s amounts, one cell
    /// reaching across two columns, are each their column's, and the table runs on through them.
    #[test]
    fn two_numbers_closer_than_an_em_are_two_cells() {
        let found = tables(&amounts("1,420,142", "1,381,247"));
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[2], ["South", "1,420,142", "1,381,247"]);
    }

    /// **…and two words are not**: a phrase reaching across two columns is no pair of values, and
    /// the line still ends the rows above it, too few to stand.
    #[test]
    fn two_words_closer_than_an_em_are_not_split() {
        assert!(tables(&amounts("Oat", "flakes")).is_empty());
    }

    /// Three rows of data, a label and a figure each, the middle column empty, 16 points apart.
    fn data_rows() -> Vec<TrackRun<'static>> {
        vec![
            run(100, 100, 40, "North"),
            run(330, 100, 40, "112.0"),
            run(100, 116, 40, "South"),
            run(350, 116, 20, "9.1"),
            run(100, 132, 40, "East"),
            run(340, 132, 30, "65.5"),
        ]
    }

    /// `West`'s line at `y`, filling the middle column.
    fn wider_line(y: i64) -> Vec<TrackRun<'static>> {
        vec![
            run(100, y, 40, "West"),
            run(240, y, 30, "12.0"),
            run(335, y, 35, "70.25"),
        ]
    }

    /// **A line wider than the table re-tracks it** (`-v9`): three rows of data leave the middle
    /// column empty and `West`'s line fills it; the table runs on in three columns, the rows above
    /// on the tracks they sit on.
    #[test]
    fn a_line_wider_than_the_table_re_tracks_it() {
        let found = tables(&[data_rows(), wider_line(148)].concat());
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["North", "", "112.0"]);
        assert_eq!(texts(&found[0])[3], ["West", "12.0", "70.25"]);
    }

    /// **…not under two rows**, too few to be the table's — a header set on two lines holds digits
    /// too; **nor under rows that hold no digit**, a header's words; **nor across a gap**, `West`
    /// two rows' pitch below opening something new. No table stands in any of them.
    #[test]
    fn a_wider_line_re_tracks_only_rows_of_data_at_the_pitch() {
        let two = [&data_rows()[..4], &wider_line(132)[..]].concat();
        assert!(tables(&two).is_empty(), "two rows");
        let words: Vec<TrackRun<'static>> = data_rows()
            .into_iter()
            .map(|r| {
                if r.x > 20_000 {
                    TrackRun { text: "High", ..r }
                } else {
                    r
                }
            })
            .chain(wider_line(148))
            .collect();
        assert!(tables(&words).is_empty(), "no digit");
        assert!(
            tables(&[data_rows(), wider_line(156)].concat()).is_empty(),
            "two rows' pitch below"
        );
    }

    /// One catalogue row at `y`: a code, a name and a description whose `wraps` run on under it, 12
    /// points apart, in the third column.
    fn tall(y: i64, code: &'static str, wraps: &[&'static str]) -> Vec<TrackRun<'static>> {
        let mut out = vec![
            run(100, y, 20, code),
            run(200, y, 30, "Oats"),
            run(300, y, 80, "Rolled grain"),
        ];
        for (k, wrap) in (1..).zip(wraps) {
            out.push(run(300, y + 12 * k, 60, wrap));
        }
        out
    }

    /// Four catalogue rows 46 points apart — past three and a half pitches of a row's first line,
    /// 22 points under its last.
    fn catalogue() -> Vec<TrackRun<'static>> {
        [(100, "1.1"), (146, "1.2"), (192, "1.3"), (238, "1.4")]
            .into_iter()
            .flat_map(|(y, code)| tall(y, code, &["and dried", "at speed"]))
            .collect()
    }

    /// **A row its wrapped cell made tall reaches past the pitch** (`-v9`): every row of the
    /// catalogue is the table's, which ended at its first row.
    #[test]
    fn a_row_its_wrapped_cell_made_tall_reaches_past_the_pitch() {
        let found = tables(&catalogue());
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
    }

    /// **…not in two columns** — terms beside their definitions; **nor where the wraps fill every
    /// column**, a block of figures and their captions; **nor to a row set at another size**.
    #[test]
    fn a_tall_row_reaches_only_in_three_columns_wrapped_in_part_at_its_size() {
        let two: Vec<TrackRun<'static>> =
            catalogue().into_iter().filter(|r| r.x != 20_000).collect();
        assert!(tables(&two).is_empty(), "two columns");
        let every: Vec<TrackRun<'static>> = [100, 146, 192, 238]
            .into_iter()
            .flat_map(|y| {
                vec![
                    run(100, y, 20, "1.1"),
                    run(200, y, 30, "Oats"),
                    run(300, y, 80, "Rolled grain"),
                    run(100, y + 12, 20, "and"),
                    run(200, y + 12, 30, "more"),
                    run(300, y + 24, 60, "at speed"),
                ]
            })
            .collect();
        assert!(tables(&every).is_empty(), "every column wrapped");
        let larger: Vec<TrackRun<'static>> = [(100, "1.1"), (146, "1.2"), (192, "1.3")]
            .into_iter()
            .flat_map(|(y, code)| tall(y, code, &["and dried", "at speed"]))
            .chain(tall(254, "1.4", &[]).into_iter().map(|r| TrackRun {
                em: Some(1400),
                ..r
            }))
            .collect();
        assert_eq!(tables(&larger)[0].rows, 3, "a row set at 14 points");
    }

    /// **…nor beyond the reach of the row's last wrapped line** — rows 62 points apart, one wrapped
    /// line each, 50 points under it; **nor for a row missing a cell**, whose name is not there.
    #[test]
    fn a_tall_row_reaches_a_full_row_within_reach_of_its_last_line() {
        let far: Vec<TrackRun<'static>> = [(100, "1.1"), (162, "1.2"), (224, "1.3"), (286, "1.4")]
            .into_iter()
            .flat_map(|(y, code)| tall(y, code, &["and dried"]))
            .collect();
        assert!(
            tables(&far).is_empty(),
            "past the last wrapped line's reach"
        );
        let sparse: Vec<TrackRun<'static>> = catalogue()
            .into_iter()
            .filter(|r| r.y == 10_000 || r.x != 20_000)
            .collect();
        assert!(tables(&sparse).is_empty(), "a row missing its name");
    }

    /// **…and a table its reach cannot keep stands without it**: five rows at a 12-point pitch, the
    /// last made tall, and a sixth 46 points under it, which the pitch refuses — the five stand.
    #[test]
    fn a_table_its_reach_cannot_keep_stands_without_it() {
        let runs: Vec<TrackRun<'static>> = [(100, "1.1"), (112, "1.2"), (124, "1.3"), (136, "1.4")]
            .into_iter()
            .flat_map(|(y, code)| tall(y, code, &[]))
            .chain(tall(148, "1.5", &["and dried", "at speed"]))
            .chain(tall(194, "1.6", &[]))
            .collect();
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
    }

    /// Three rows of a label and three figures, `South`'s figures 6 points apart — closer than an
    /// em, so one cell across three columns — or three words the same way.
    fn figures(a: &'static str, b: &'static str, c: &'static str) -> Vec<TrackRun<'static>> {
        vec![
            run(100, 100, 40, "North"),
            run(250, 100, 20, "1.0"),
            run(310, 100, 20, "2.0"),
            run(370, 100, 20, "3.0"),
            run(100, 116, 40, "South"),
            run(252, 116, 18, a),
            run(276, 116, 54, b),
            run(336, 116, 54, c),
            run(100, 132, 40, "East"),
            run(255, 132, 15, "4.0"),
            run(315, 132, 15, "5.0"),
            run(375, 132, 15, "6.0"),
        ]
    }

    /// **A cell of several numbers is as many cells** (`-v10`): `South`'s three figures are each
    /// their column's, and the table runs on through them; three words are not split.
    #[test]
    fn a_cell_of_several_numbers_is_as_many_cells() {
        let found = tables(&figures("11.5", "22.5", "33.5"));
        assert_eq!((found[0].rows, found[0].columns), (3, 4));
        assert_eq!(texts(&found[0])[1], ["South", "11.5", "22.5", "33.5"]);
        assert!(tables(&figures("alpha", "beta", "gamma")).is_empty());
    }

    /// Four rows whose labels draw dot leaders up to their first figure, five points short of it:
    /// `South`'s leader is drawn as two runs.
    fn leaders() -> Vec<TrackRun<'static>> {
        vec![
            run(100, 100, 30, "North"),
            run(132, 100, 128, "................"),
            run(265, 100, 20, "7.5"),
            run(330, 100, 15, "9.1"),
            run(100, 116, 30, "South"),
            run(132, 116, 60, "........"),
            run(194, 116, 66, "........"),
            run(265, 116, 20, "12.0"),
            run(330, 116, 15, "4.4"),
            run(100, 132, 30, "East"),
            run(132, 132, 128, "................"),
            run(265, 132, 20, "3.0"),
            run(330, 132, 15, "6.5"),
            run(100, 148, 12, "St."),
            run(114, 148, 18, "Louis"),
            run(134, 148, 126, "................"),
            run(265, 148, 20, "8.0"),
            run(330, 148, 15, "2.2"),
        ]
    }

    /// **Dot leaders end a cell** (`-v11`): each label keeps its leaders and the figure they lead to
    /// is its column's, though it sets within an em of them; a leader drawn as two runs is still its
    /// label's, and an abbreviation's one point ends no cell.
    #[test]
    fn dot_leaders_end_a_cell() {
        let found = tables(&leaders());
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["North................", "7.5", "9.1"]);
        assert_eq!(
            texts(&found[0])[1],
            ["South................", "12.0", "4.4"]
        );
        assert_eq!(
            texts(&found[0])[3],
            ["St.Louis................", "8.0", "2.2"]
        );
    }

    /// **A sign set alone joins the figure after it** (`-v11`): `South`'s three amounts, each `$`
    /// flush left in its column, run together within an em into one cell, and split into their
    /// columns with each sign kept with its figure.
    #[test]
    fn a_sign_set_alone_joins_the_figure_after_it() {
        let runs = vec![
            run(100, 100, 40, "North"),
            run(265, 100, 20, "1.0"),
            run(312, 100, 20, "2.0"),
            run(359, 100, 20, "3.0"),
            run(100, 116, 40, "South"),
            run(250, 116, 5, "$"),
            run(260, 116, 25, "11.5"),
            run(293, 116, 5, "$"),
            run(307, 116, 25, "22.5"),
            run(340, 116, 5, "$"),
            run(354, 116, 25, "33.5"),
            run(100, 132, 40, "East"),
            run(265, 132, 20, "4.0"),
            run(312, 132, 20, "5.0"),
            run(359, 132, 20, "6.0"),
        ];
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (3, 4));
        assert_eq!(texts(&found[0])[1], ["South", "$11.5", "$22.5", "$33.5"]);
    }

    /// **Titles drawn with dot leaders, then their page numbers, are a table of contents** (`-v11`),
    /// at any width: the leaders end each title's cell, so the page numbers stand as a column of their
    /// own, and neither three columns — numbered sections — nor two whose numbers carry a revision
    /// mark beside them are a table.
    #[test]
    fn titles_drawn_with_leaders_then_page_numbers_are_contents() {
        let mut sections = Vec::new();
        let mut revised = Vec::new();
        for (n, (number, title, page)) in [
            ("1", "Introduction", "1"),
            ("2", "Zero Trust Basics", "4"),
            ("3", "Components", "9"),
            ("4", "Deployment", "17"),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 100 + 16 * i64::try_from(n).expect("small");
            sections.extend([
                run(100, y, 10, number),
                run(130, y, 60, title),
                run(192, y, 200, "................................"),
                run(400, y, 10, page),
            ]);
            revised.extend([
                run(130, y, 60, title),
                run(192, y, 200, "................................"),
                run(400, y, 30, ["103101", "105103", "106104", "107105"][n]),
            ]);
        }
        assert!(tables(&sections).is_empty(), "numbered sections");
        assert!(tables(&revised).is_empty(), "revised page numbers");
        // Titles led to words, not numbers, are a table of two columns.
        let terms: Vec<TrackRun<'static>> = revised
            .iter()
            .enumerate()
            .map(|(k, r)| {
                if k % 3 == 2 {
                    run(400, r.y / 100, 60, "Alpha Beta")
                } else {
                    *r
                }
            })
            .collect();
        assert_eq!(tables(&terms).len(), 1, "a glossary drawn with leaders");
        // Years led to their figures are a schedule, not titles.
        let years: Vec<TrackRun<'static>> = revised
            .iter()
            .enumerate()
            .map(|(k, r)| {
                if k % 3 == 0 {
                    run(130, r.y / 100, 60, ["2022", "2023", "2024", "2025"][k / 3])
                } else {
                    *r
                }
            })
            .collect();
        assert_eq!(tables(&years).len(), 1, "a schedule drawn with leaders");
    }

    /// **A row of data above a table is its row** (`-v11`): `Total`, repeating `North`'s figure and
    /// leaving the third column empty, is no header and could open no table, and it is the table's
    /// first row — within a row and a half's pitch of it, and no further.
    #[test]
    fn a_row_of_data_above_a_table_is_its_row() {
        let total = |y: i64| vec![run(100, y, 40, "Total"), run(250, y, 20, "7.5")];
        let found = under(total(184));
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[0], ["Total", "7.5", ""]);
        let found = under(total(170));
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["North", "7.5", "112.0"]);
    }

    /// **…and a header line leaves the first track empty and stays the band's**: `2023` and `2024`
    /// under `Fiscal` are numbers, and they are the header's second line, not a row.
    #[test]
    fn a_header_line_of_numbers_stays_the_band_s() {
        let found = under(vec![
            run(245, 168, 25, "Fiscal"),
            run(340, 168, 30, "Fiscal"),
            run(250, 184, 20, "2023"),
            run(340, 184, 30, "2024"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[0], ["", "Fiscal2023", "Fiscal2024"]);
    }

    /// **The stub's heading may run long** (`-v11`): `(in millions of dollars, except ratios)` heads
    /// the first column of a header line that names the others.
    #[test]
    fn the_stub_s_heading_may_run_long() {
        let found = under(vec![
            run(100, 186, 45, "(in millions of dollars, except ratios)"),
            run(205, 186, 30, "Units"),
            run(300, 186, 30, "Price"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(
            texts(&found[0])[0],
            ["(in millions of dollars, except ratios)", "Units", "Price"]
        );
        // Alone on its line, a long cell is a caption or a units note, and the band ends there.
        let found = under(vec![
            run(205, 170, 30, "Units"),
            run(300, 170, 30, "Price"),
            run(100, 186, 45, "(in millions of dollars, except ratios)"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["North", "7.5", "112.0"]);
    }

    /// **…nor a number in the gutter**: `5`, set between the second column and the third and on
    /// neither, is no column's value, though a full line may hold one cell off its track.
    #[test]
    fn a_number_in_the_gutter_is_no_column_s() {
        let mut runs = amounts("1,420,142", "1,381,247");
        runs[7] = run(228, 132, 57, "1,420,142");
        runs[8] = run(290, 132, 45, "5");
        assert!(tables(&runs).is_empty());
    }

    /// **A row with cells missing is a row at the table's pitch** (`-v6`): an invoice's item with no
    /// quantity, 16 points under the row above like every other row, stays its own row; the second
    /// line of a label, opening lower-case, still continues the row above.
    #[test]
    fn a_sparse_row_at_the_pitch_is_a_row_and_a_label_s_second_line_a_wrap() {
        let items = |third: [TrackRun<'static>; 2]| -> Vec<TrackRun<'static>> {
            let mut runs = vec![
                run(100, 100, 10, "1"),
                run(160, 100, 60, "Widget A"),
                run(300, 100, 15, "15"),
                run(400, 100, 30, "$60.00"),
                run(100, 116, 10, "2"),
                run(160, 116, 60, "Widget B"),
                run(300, 116, 15, "12"),
                run(400, 116, 30, "$48.00"),
            ];
            runs.extend(third);
            runs.extend([
                run(100, 148, 10, "4"),
                run(160, 148, 60, "Widget D"),
                run(300, 148, 15, "8"),
                run(400, 148, 30, "$32.00"),
            ]);
            runs
        };
        let found = tables(&items([
            run(100, 132, 10, "3"),
            run(400, 132, 30, "$45.00"),
        ]));
        assert_eq!((found[0].rows, found[0].columns), (4, 4));
        assert_eq!(texts(&found[0])[2], ["3", "", "", "$45.00"]);
        let found = tables(&items([
            run(100, 132, 40, "continued"),
            run(400, 132, 30, "net"),
        ]));
        assert_eq!((found[0].rows, found[0].columns), (3, 4));
        assert_eq!(
            texts(&found[0])[1],
            ["2continued", "Widget B", "12", "$48.00net"]
        );
    }

    /// **A first column of bullets is a list, whatever stands beside it** (`-v6`): three columns —
    /// bullets, items, and the lines of a paragraph set level with them — are no table.
    #[test]
    fn a_first_column_of_bullets_is_a_list_at_any_width() {
        let runs: Vec<TrackRun<'static>> = (0..4)
            .flat_map(|k| {
                let y = 100 + 16 * k;
                [
                    run(100, y, 8, "•"),
                    run(130, y, 120, "an item of the list"),
                    run(320, y, 200, "a line of the paragraph beside it"),
                ]
            })
            .collect();
        assert!(tables(&runs).is_empty());
    }

    #[test]
    fn right_aligned_columns_are_a_table() {
        let found = tables(&numbers());
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[2], ["South", "18.25", "9.1"]);
        assert_eq!(found[0].rule, ethos_parser_core::TABLE_DETECTION_TRACKS_V12);
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

    /// **A label column written as one block is the table's** (`-v12`): a spreadsheet's export writes
    /// `North` to `West` first and then each row's figures, the order of a column of text beside a
    /// table, and the grid stands; a column of notes written so, each line past thirty characters
    /// and none opening lower-case, or a label column missing a row's label, is still two flows.
    #[test]
    fn a_label_column_written_as_one_block_is_the_table_s() {
        let block = |labels: [&'static str; 5]| -> Vec<TrackRun<'static>> {
            let mut runs: Vec<TrackRun<'static>> = labels
                .iter()
                .enumerate()
                .filter(|(_, text)| !text.is_empty())
                .map(|(k, &text)| run(100, 100 + 20 * k as i64, 40, text))
                .collect();
            for (k, (a, b)) in [
                ("7.5", "112.0"),
                ("18.25", "9.1"),
                ("4.0", "65.5"),
                ("12.0", "70.25"),
                ("3.5", "8.0"),
            ]
            .into_iter()
            .enumerate()
            {
                let y = 100 + 20 * k as i64;
                runs.extend([run(250, y, 20, a), run(340, y, 30, b)]);
            }
            runs
        };
        let found = tables(&block(["North", "South", "East", "West", "Central"]));
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[1], ["South", "18.25", "9.1"]);
        assert!(
            tables(&block([
                "Sales rose in every region but one",
                "Prices held through the third quarter",
                "Costs fell as the new plant opened",
                "Margins widened in the second half",
                "Outlook for next year remains firm",
            ]))
            .is_empty(),
            "a column of notes"
        );
        assert!(
            tables(&block(["North", "South", "East", "", "Central"])).is_empty(),
            "a row with no label"
        );
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
