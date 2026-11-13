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
//! cell between two rows, and end the table there. **And a mark set against the text before it may
//! stand higher** (`-v16`, decision #119, [`join_superscripts`]): by less than three fifths of the
//! em, where each of its runs starts within a quarter em of where another run of either line ends.
//! A filing raises the `1` on `Comp sales¹` a little over half an em, and as a line of its own it
//! sat off every track and ended the table above its row; a chart's labels set just above another
//! line's start where no text ends, and stay lines of their own.
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
//! **A figure under a figure opens a row** (`-v16`, decision #117): a line of two cells or more
//! that sets a number on a track where the row above holds one is a row however close it sits,
//! whatever cells it leaves empty. A stub that names its group once — a cause of loss over its
//! years, a sleeve length over its clearances — left every later row of the group without a first
//! cell, and each was read as the first row's wrapped line: `2012 2013` in one cell. A year is no
//! figure here, since a statement sets its next block's year over it, and a number alone on its
//! line is a margin's line number as often as a row.
//!
//! **A header set on several lines above the first row is one row** (`-v5`, [`header_band`]): the
//! lines just above an accepted table, each within a row and a half's pitch of the line below it,
//! whose every cell sits on a track clear of its neighbours, holds fewer than
//! [`PROSE_LINE_CHARS`] characters and repeats no value its column holds below, join as the table's
//! first row — one cell per track, its lines top to bottom — where they name more than half the
//! tracks, or since `-v16` fewer over the header's last line (below). A header's cells are set
//! centred on a row of their own, stacked unevenly, and no line of them has a cell for every
//! column, so no line of them opens or joins the table on its own.
//!
//! **A header set off its columns is still theirs** (`-v7`, [`header_tracks`]): a header line of
//! two cells or more whose cells do not each sit on a track — a column name centred over figures
//! set flush right, a year over its column of numbers — gives each cell the track whose centre is
//! nearest its own, where they come out in order, one to a track. And the band's first line may
//! stand two rows' pitch above the table, where a rule and its padding set a header off its first
//! row — two and a half since `-v15` — and the lines above it still climb a row and a half at most. A header the table cannot be built
//! with — a cell of it, two column names closer than an em, covering a neighbour's column — is
//! none: the rows stand as they would without it.
//!
//! **A header's last line read as the first row is the header's** (`-v13`,
//! [`header_last_line`]): a header set on two lines whose lower line has a cell on every column
//! opens the table itself — `Shares` and `Exercise Price` under `Number of` and `Weighted Average`
//! — and the band above takes only the upper. Where the band names more than half the tracks on
//! its own, a first row whose cells on the columns of numbers hold no digit, run shorter than a
//! line of prose and repeat no value of their columns — and that says not one word in every
//! column — joins the header row, each column's lines top to bottom. **And a band naming fewer is
//! that header's upper lines** (`-v16`, decision #118): where a line of the band holds two cells or
//! more and the first row reads so, the band joins it however few tracks it names — `0 - 12 Month`
//! over `Count`, `March 31,` over `2025`, the other columns' names the first row's alone. A band of
//! one cell to a line is a title's or a caption's, and heads nothing.
//!
//! **A header set row by row is one row** (`-v15`, decision #96, [`header_block`]): a header whose
//! every line names every column opens the table at its first line, and each line stood as a row
//! — `Year Ending` over `December 31,`, its columns' names wrapped. The first two to
//! [`HEADER_MAX_LINES`] rows, or the header a band made and the rows under it, that each hold a
//! cell on every column repeating no value its column holds below, read as a header's line as the
//! `-v13` clause reads one, and sit closer to each other than the first row below sits to them, are
//! one header row, each column's lines top to bottom. Set at the rows' own pitch they stay rows:
//! a table's first rows of words. **And a header's line may name years** ([`heads`]): two cells or
//! more of it each a year and nothing else, over columns of numbers that hold none, are a
//! header's — `2024 | 2023` under `Fiscal` — where a year set beside words is still a value.
//!
//! **A header cell set over several columns heads them all** (`-v16`, decision #112,
//! [`group_row`]): over a header the band made, or a first row that reads as a header's line, a cell
//! of the band's lines, or of the lines just above the header, centred over the header ink below it
//! on two tracks or more is a group head — `Non-Tobacco` over `Female | Male`, `Inception to Date`
//! over three columns of figures' names — and the table gains a first row of group heads, each a
//! cell spanning the tracks it heads and every other track an empty cell, the header row under it.
//! The band had stacked such a cell on its nearest track, `Female Non-Tobacco` beside `Male`, or a
//! line above stopped the band and stood outside the table. A head takes the run of tracks whose ink
//! lies nearer its middle than any other cell's of its line, the widest such run centred on it; a
//! cell centred on one column's ink or set flush with it is that column's line, a cell alone on its
//! line and centred on the whole table is its title, and the stub's column is never a group's. A
//! line above the header gives a head of every cell or none, and the climb stops at the first that
//! gives none; a head over the very columns of a head below is that head's line above, and one over
//! part of them is none, so the group row is one level. A group row the table cannot be built with
//! is none.
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
//! a statement sets its `$` flush left in columns too narrow to set it more than an em off. **And a
//! sign set alone after a figure opens the next cell** (`-v16`, decision #116, [`line`]): in a line
//! that sets its signs before its figures — its first sign or figure a sign — a sign set alone
//! within an em after a figure, a percentage or a closing parenthesis is the next amount's. A
//! statement set in close columns sets the next column's `$` within an em of the last figure, and
//! the figure took it: `(692) $` was one cell, and `1,105` came without its sign. **And so does a
//! run that opens with a sign** (`-v16`, decision #119): `$134,803` set as one run within an em
//! after `95,685` is the next amount, and the two were one cell.
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
//! statement's title. Alone on its line such a cell still ends the band; since `-v16` a header
//! above it may head the table all the same, and the cell is a row under it.
//!
//! **A header over a section's label heads the table** (`-v16`, decision #118, [`labels_above`]):
//! where the band names half the tracks or fewer, the table holds a column of numbers and its first
//! row reads as no header's line, the lines of one cell just above that first row — a section's
//! label heading the first track, at any length, or a units note heading none, shorter than a line
//! of prose — each within a row and a half's pitch of the line below it, the first within two and a
//! half, stand between the rows and a band above them that names more than half the tracks and
//! holds no number but a year. That band heads the table; the labels are one row under it, every
//! other cell of it empty, and a units note heads none and stays out of the table. A statement sets
//! `Revenues` and `(In millions)` between its columns' names and its first row of figures: the band
//! ended at the note, or took the label alone, and a label as long as a line of prose ended it at
//! once.
//!
//! **A line of years below rows of data heads the next table** (`-v16`, decision #115,
//! [`heads_next_table`]): two cells or more, each a year, set a row and a half's pitch or more below
//! rows of which one past the first holds a digit, end the table, and the rows under them are read
//! as a table headed by them. A statement stacks one table per period, each headed by its years, and
//! the second's heads joined the first as a sparse row, its rows with them.
//!
//! # What makes it a table and not prose
//!
//! Each clause measured before it was written (`docs/31-TABLE-TRACKS-SCOPE.md` §3, §6):
//!
//! 1. **Enough rows for what they claim.** Two rows stand only with no empty cell, apart from the
//!    lines around them, under a first row that reads as a header — a chart's axis labels over its
//!    legend are a pair with gaps, the top of three columns of prose runs on below. Two columns need
//!    four rows, and are no table where the first column is nothing but list labels or the second
//!    nothing but rising page numbers: a list or a table of contents set in two columns — though
//!    small numbers beside a column of numbers key a table of values (`-v16`, decision #119), ages
//!    `65` to `99` beside their premiums, where a list's items are words. And no
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
//!    refused with the flows. **And where each flow holds a column of numbers, each is read on its
//!    own** (`-v14`, [`two_tables`]): the page from the last table's end down, cut at the gutter
//!    between them. A schedule set in two panels, or a two-column page's tables level with each
//!    other, is two flows each written row by row, and neither is text; a contents list set in two
//!    panels draws its page numbers with dot leaders, and those are no column of values. **And two
//!    tables drawn side by side are each read on their own, though they stood as one** (`-v16`,
//!    decision #115, [`drawn_panels`]): where the page's horizontal ink — its stroked rules, and the
//!    top and bottom of each rectangle it paints in a colour of its own — ends in the gap between
//!    two tracks and starts in it again, each side's closing that side's last row and standing an
//!    em or more above it, and none crosses the gap, the table is cut there and each side read down
//!    to its last line, with no row order of its own asked of it inside the rows the two stood as.
//!    A filing boxes two schedules side by side, written row by row across both, and they stood as
//!    one table of both their columns. **And on a page of recognized text the clause is no evidence**
//!    (decision #125): its stream order is the recognizer's, row by row down the whole page across
//!    any columns, so three columns of a scanned newsletter passed it as a table of their lines.
//!    There a grid stands only on the waiver's clauses — three rows and three columns or more, no
//!    column of running text — and no column of numbers is asked, since with no order to read a
//!    list of names and marks is a table as much as a table of values is.
//!
//! # Fabrication is impossible here, not merely avoided
//!
//! A cell's text is its runs' text concatenated in content order. No run is in two cells, and no
//! run is moved: a line whose cells do not fit the tracks — or reach across one into its
//! neighbour's — ends the table instead.

use ethos_parser_core::{EngineError, IdAllocator, IdKind};

use crate::stroke_ruled::Rule;
use crate::tables::{cross_check, DetectedCell, DetectedTable, QuantRect, LATTICE_TOLERANCE};

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

/// The most lines a header set row by row is read as (`-v15`, decision #96): a column name set on
/// three lines, `Issue` / `Age` / `(Male or Female)`, and one more.
pub const HEADER_MAX_LINES: usize = 4;

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
    /// Another rule's table already holds this run's origin — or, for the chart rules, the page
    /// clips it away (decision #105).
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
#[derive(Debug, Clone)]
struct Row {
    y: i64,
    em: i64,
    cells: Vec<Cell>,
}

/// A cell of a header's group row (`-v16`): the first and last track it heads, and its runs.
#[derive(Debug)]
struct Group {
    first: usize,
    last: usize,
    cell: Cell,
}

/// Every table `whitespace-tracks-v10` finds on one page, in reading-down order. `rules` is the
/// page's horizontal ink (`-v16`): its stroked rules, and each painted rectangle's top and bottom.
/// `recognized` says the page's runs are text a recognizer read (decision #125): their stream order
/// is the recognizer's, row by row down the whole page across any columns, so it is no evidence that
/// a grid was written as a table, and such a grid stands only as one written in no order does.
///
/// # Errors
///
/// An id the allocator cannot issue.
pub(crate) fn detect(
    page: u32,
    runs: &[TrackRun<'_>],
    rules: &[Rule],
    recognized: bool,
    alloc: &mut IdAllocator,
) -> Result<Vec<DetectedTable>, EngineError> {
    detect_within(page, runs, rules, None, recognized, alloc)
}

/// [`detect`], where `ordered` names the baseline of the first row of a table that stood (`-v16`,
/// decision #115): one side of two tables drawn side by side, which stood as one, needs no row order
/// of its own from that row down — the table they stood as had it, or the grid's waiver, over every
/// line the side is read to.
fn detect_within(
    page: u32,
    runs: &[TrackRun<'_>],
    rules: &[Rule],
    ordered: Option<i64>,
    recognized: bool,
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
                let within = ordered.is_some_and(|y0| rows[0].y >= y0 - LINE_TOLERANCE);
                (rows.len() > MIN_ROWS || pair_plausible(&lines, i, end, rows, runs))
                    && accepted(rows, runs, within, recognized)
            };
            let mut stand = stands(&rows, end);
            // `-v9`: a table its reach past the pitch cannot keep stands without it.
            if let (false, Some((kept, at))) = (stand, reach) {
                rows.truncate(kept);
                end = at;
                stand = stands(&rows, end);
            }
            // `-v14`: two flows side by side, each holding a column of numbers, are each read on
            // their own — the page from the last table's end down, cut at the gutter between them,
            // since one side's table may run on below where the other's ended this one. `-v16`
            // (decision #115): and two tables set side by side, each drawn its own rules, that
            // stood as one, are read so too — down to this one's last line, as far as the rules
            // say, each side's rows inside those it stood with a table's rows already.
            let apart = if stand {
                drawn_panels(&rows, lines[end - 1].y, rules)
                    .map(|k| (k, lines[end - 1].y, Some(rows[0].y)))
            } else {
                two_tables(&rows, runs).map(|k| (k, i64::MAX, ordered))
            };
            if let Some((k, bottom, side_ordered)) = apart {
                let tracks = tracks_of(&rows);
                let cut = (tracks[k - 1].1 + tracks[k].0).div_euclid(2);
                let top = lines[floor].y;
                for left in [true, false] {
                    let side: Vec<TrackRun<'_>> = runs
                        .iter()
                        .map(|r| TrackRun {
                            claimed: r.claimed
                                || r.y < top - LINE_TOLERANCE
                                || r.y > bottom.saturating_add(LINE_TOLERANCE)
                                || (r.x < cut) != left,
                            ..*r
                        })
                        .collect();
                    tables.extend(detect_within(
                        page,
                        &side,
                        rules,
                        side_ordered,
                        recognized,
                        alloc,
                    )?);
                }
                i = end;
                floor = end;
                continue;
            }
            if stand {
                let mut first = rows_above(&lines, floor, i, &mut rows, runs);
                let tracks = tracks_of(&rows);
                let mut top = header_band(&lines, floor, first, &tracks, &rows, runs);
                let last = header_last_line(&rows, runs);
                // `-v16` (decision #118): a header over the one-cell lines just above the first
                // row — a section's label on the stub's track, a units note on none — heads a table
                // of numbers whose band names half the tracks or fewer and whose first row reads as
                // no header's line, where that header holds no number but a year. The labels are a
                // row under it; a units note heads no track, so `header_row` puts it in no cell.
                if !last
                    && 2 * tracks_named(&lines[top..first], &tracks) <= tracks.len()
                    && !numeric_columns(tracks.len(), &rows, runs).is_empty()
                {
                    let over = labels_above(&lines, floor, first, &tracks, runs);
                    let above = header_band(&lines, floor, over, &tracks, &rows, runs);
                    let band = &lines[above..over];
                    if over < first
                        && 2 * tracks_named(band, &tracks) > tracks.len()
                        && band.iter().flat_map(|l| &l.cells).all(|c| {
                            let text = cell_text(c, runs);
                            !is_number(text.trim()) || is_year(text.trim())
                        })
                    {
                        let labels = header_row(&lines[over..first], &tracks);
                        if labels.cells.iter().any(|c| !c.runs.is_empty()) {
                            rows.insert(0, labels);
                        }
                        (top, first) = (above, over);
                    }
                }
                let mut headed = false;
                let mut joined: Option<Row> = None;
                if top < first {
                    let mut header = header_row(&lines[top..first], &tracks);
                    let named = header.cells.iter().filter(|c| !c.runs.is_empty()).count();
                    // `-v16` (decision #118): a band with a line of two cells or more over a first
                    // row that reads as the header's last line is that header's upper lines, however
                    // few tracks it names; a band of one cell to a line is a title or a caption.
                    let wide = lines[top..first].iter().any(|l| l.cells.len() >= 2);
                    if 2 * named > tracks.len() || (wide && last) {
                        // `-v13`: the header's last line, read as the first row, is the header's.
                        if last {
                            let row = rows.remove(0);
                            stack(&mut header, &row);
                            joined = Some(row);
                        }
                        rows.insert(0, header);
                        headed = true;
                    }
                }
                // `-v15` (decision #96): a header whose lines each read as a row is one row.
                let block = header_block(&rows, runs);
                let stacked: Vec<Row> = rows.drain(1..block).collect();
                for row in &stacked {
                    stack(&mut rows[0], row);
                }
                // `-v16` (decision #112): a header cell set over several columns heads them all,
                // over a header the band made or a first row that reads as one.
                let saved = rows[0].clone();
                let groups = if headed || heads(&rows[0], &rows[1..], runs) {
                    let band = if headed { top..first } else { first..first };
                    group_row(&lines, floor, band, &mut rows[0], &tracks, runs)
                } else {
                    Vec::new()
                };
                let mut table = build(page, &rows, &groups, runs, alloc)?;
                // A group row the table cannot be built with is none: the header stands as it was.
                if table.is_none() && !groups.is_empty() {
                    rows[0] = saved;
                    table = build(page, &rows, &[], runs, alloc)?;
                }
                // A header the table cannot be built with is none (`-v7`): the rows stand alone.
                if table.is_none() && headed {
                    rows.splice(0..1, stacked);
                    if let Some(row) = joined.take() {
                        rows.insert(0, row);
                    }
                    table = build(page, &rows, &[], runs, alloc)?;
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
/// first line within two and a half rows' pitch of the table (`-v15`; two since `-v7`) — whose
/// cells [`header_tracks`] gives a
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
        // Pitches of 1.2 ems, in integers, as in `grow`: 1.5 is 9/5, and 2.5 is 15/5 (`-v15`; `-v7`
        // read the first line's at 2, 12/5).
        let em = line.em.max(below.em);
        let reach = if top == start { 15 } else { 9 };
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

/// How many of `tracks` the lines of `band` name, as [`header_row`] stacks them; none for no lines.
fn tracks_named(band: &[Line], tracks: &[(i64, i64)]) -> usize {
    if band.is_empty() {
        return 0;
    }
    header_row(band, tracks)
        .cells
        .iter()
        .filter(|c| !c.runs.is_empty())
        .count()
}

/// The first of the lines of one cell just above a table's first row (`-v16`, decision #118):
/// climbing from `start` to no lower than `floor`, each within a row and a half's pitch of the line
/// below it — the first within two and a half — a section's label, heading the first track as
/// [`header_tracks`] reads it, at any length, or a units note, heading none and shorter than a line
/// of prose. `start` where the line above it is neither.
fn labels_above(
    lines: &[Line],
    floor: usize,
    start: usize,
    tracks: &[(i64, i64)],
    runs: &[TrackRun<'_>],
) -> usize {
    let mut k = start;
    while k > floor {
        let (line, below) = (&lines[k - 1], &lines[k]);
        // Pitches of 1.2 ems, in integers, as in `header_band`.
        let em = line.em.max(below.em);
        let reach = if k == start { 15 } else { 9 };
        let [cell] = &line.cells[..] else { break };
        let mapping = header_tracks(line, tracks);
        let label = mapping.as_deref() == Some(&[0]);
        let note =
            mapping.is_none() && cell_text(cell, runs).trim().chars().count() < PROSE_LINE_CHARS;
        if 5 * (below.y - line.y) > reach * em || !(label || note) {
            break;
        }
        k -= 1;
    }
    k
}

/// Whether a table's first row is its header's last line (`-v13`): a column whose rows below hold
/// a digit in half of them or more; on every such column a cell with no digit in it, shorter than
/// a line of prose and repeating no value of its column; and not one word said in three cells or
/// more past the first, which is a row of placeholders — `NA` across every column — and no
/// header. A first row opens its tracks, so it holds a cell on each; and a table under a band holds
/// three rows or more, since two stand only clear of every line within four ems above them.
fn header_last_line(rows: &[Row], runs: &[TrackRun<'_>]) -> bool {
    heads(&rows[0], &rows[1..], runs)
}

/// The columns past the first whose rows in `below` hold a digit in half of them or more: a
/// table's columns of numbers.
fn numeric_columns(columns: usize, below: &[Row], runs: &[TrackRun<'_>]) -> Vec<usize> {
    (1..columns)
        .filter(|&k| {
            let digits = below
                .iter()
                .filter(|r| {
                    cell_text(&r.cells[k], runs)
                        .chars()
                        .any(|c| c.is_ascii_digit())
                })
                .count();
            2 * digits >= below.len()
        })
        .collect()
}

/// Whether `row` reads as a header's line over the rows `below` it, as [`header_last_line`] reads
/// a first row — and since `-v15` (decision #96) a line naming two years or more, each a cell of
/// its own over a column of numbers that holds no year, is a header's: `2024 | 2023` under
/// `Fiscal`, where a single year beside words is a value.
fn heads(row: &Row, below: &[Row], runs: &[TrackRun<'_>]) -> bool {
    let text = |r: &Row, k: usize| cell_text(&r.cells[k], runs).trim().to_string();
    let columns = row.cells.len();
    let numeric = numeric_columns(columns, below, runs);
    let said: Vec<String> = (1..columns)
        .map(|k| text(row, k))
        .filter(|t| !t.is_empty())
        .collect();
    let years = numeric.iter().filter(|&&k| is_year(&text(row, k))).count();
    !numeric.is_empty()
        && (said.len() < 3 || said.iter().any(|t| *t != said[0]))
        && numeric.iter().all(|&k| {
            let head = text(row, k);
            let named = !head.chars().any(|c| c.is_ascii_digit())
                || (years >= 2 && is_year(&head) && !below.iter().any(|r| is_year(&text(r, k))));
            named
                && head.chars().count() < PROSE_LINE_CHARS
                && below.iter().all(|r| text(r, k) != head)
        })
}

/// Whether `text` is a year and nothing more: four digits, 1900 to 2099.
fn is_year(text: &str) -> bool {
    text.len() == 4
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text.starts_with("19") || text.starts_with("20"))
}

/// How many of a table's first rows are one header set on several lines (`-v15`, decision #96):
/// the most, from two to [`HEADER_MAX_LINES`] with a row below them, that each hold a cell on every
/// column repeating no value its column holds below them — a log's `COMMENT` set down its first
/// column is no header's — and read as a header's line over the rows below them all, set closer to
/// each other than the first row below is set to them: each line measured from where the one above
/// it ends, a wrapped line from its last. One where no such lines open the table.
fn header_block(rows: &[Row], runs: &[TrackRun<'_>]) -> usize {
    let text = |r: &Row, k: usize| cell_text(&r.cells[k], runs).trim().to_string();
    let bottom = |r: &Row| {
        r.cells
            .iter()
            .flat_map(|c| c.runs.iter())
            .map(|&i| runs[i].y)
            .max()
            .unwrap_or(r.y)
    };
    (2..=rows.len().saturating_sub(1).min(HEADER_MAX_LINES))
        .rev()
        .find(|&n| {
            let below = &rows[n..];
            let leading = (1..n).map(|k| rows[k].y - bottom(&rows[k - 1])).max();
            rows[..n].iter().all(|row| {
                (0..row.cells.len()).all(|k| {
                    let head = text(row, k);
                    !head.is_empty() && below.iter().all(|r| text(r, k) != head)
                }) && heads(row, below, runs)
            }) && leading.is_some_and(|leading| leading < rows[n].y - bottom(&rows[n - 1]))
        })
        .unwrap_or(1)
}

/// `row`'s runs set under `header`'s, column by column, each cell widened to hold both.
fn stack(header: &mut Row, row: &Row) {
    for (cell, c) in header.cells.iter_mut().zip(&row.cells) {
        if c.runs.is_empty() {
            continue;
        }
        if cell.runs.is_empty() {
            (cell.x0, cell.x1) = (c.x0, c.x1);
        } else {
            (cell.x0, cell.x1) = (cell.x0.min(c.x0), cell.x1.max(c.x1));
        }
        cell.runs.extend(&c.runs);
    }
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

/// The group row over a table's header row `header` (`-v16`, decision #112), left to right: the
/// group heads of its `band`'s lines, bottom to top, each taken out of the header cell its band
/// stacked it in, and of the lines just above the header's top line — the band's, or the first
/// row's where it has none — each within a row and a half's pitch of the line below it, the first
/// over a first row within two and a half (`-v15`). A line above gives every cell of it a head or
/// none, and the climb ends at the first that gives none. A head over the very tracks a head below
/// it heads is that head's line above; [`line_groups`] says the rest. No line below `floor` is read.
fn group_row(
    lines: &[Line],
    floor: usize,
    band: std::ops::Range<usize>,
    header: &mut Row,
    tracks: &[(i64, i64)],
    runs: &[TrackRun<'_>],
) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let take = |line: &Line, header: &Row, groups: &mut Vec<Group>, whole: bool| -> bool {
        let Some(found) = line_groups(line, header, groups, tracks, runs, whole) else {
            return false;
        };
        for (first, last, cell) in found {
            match groups
                .iter_mut()
                .find(|g| (g.first, g.last) == (first, last))
            {
                Some(group) => {
                    group.cell.x0 = group.cell.x0.min(cell.x0);
                    group.cell.x1 = group.cell.x1.max(cell.x1);
                    group.cell.runs.extend(&cell.runs);
                }
                None => groups.push(Group { first, last, cell }),
            }
        }
        true
    };
    for k in band.clone().rev() {
        take(&lines[k], header, &mut groups, false);
    }
    let mut top = band.start;
    while top > floor {
        let (line, below) = (&lines[top - 1], &lines[top]);
        // Pitches of 1.2 ems, in integers, as in `header_band`.
        let em = line.em.max(below.em);
        let reach = if band.is_empty() && top == band.start {
            15
        } else {
            9
        };
        if line.cells.is_empty()
            || 5 * (below.y - line.y) > reach * em
            || !take(line, header, &mut groups, true)
        {
            break;
        }
        top -= 1;
    }
    // The heads' runs leave the header cells the band stacked them in.
    let taken: std::collections::BTreeSet<usize> = groups
        .iter()
        .flat_map(|g| g.cell.runs.iter().copied())
        .collect();
    for (k, cell) in header.cells.iter_mut().enumerate() {
        if !cell.runs.iter().any(|i| taken.contains(i)) {
            continue;
        }
        cell.runs.retain(|i| !taken.contains(i));
        (cell.x0, cell.x1) = cell
            .runs
            .iter()
            .filter(|&&i| !runs[i].text.trim().is_empty())
            .map(|&i| (runs[i].x, runs[i].rect.map_or(runs[i].x, |b| b.x1)))
            .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
            .unwrap_or((tracks[k].0, tracks[k].0));
    }
    groups.sort_by_key(|g| g.first);
    groups
}

/// The group heads one header `line` holds over the ink of `header` below it (`-v16`), each with
/// the first and last track it heads: the cells [`group_of`] reads as heading two tracks or more,
/// where no cell of the line runs as long as a line of prose; but never one heading part of what one
/// of `groups` heads. With `whole`, a line above the header, every cell must head two tracks or more
/// and none part of another's, or the line gives none: `None`.
fn line_groups(
    line: &Line,
    header: &Row,
    groups: &[Group],
    tracks: &[(i64, i64)],
    runs: &[TrackRun<'_>],
    whole: bool,
) -> Option<Vec<(usize, usize, Cell)>> {
    let taken: std::collections::BTreeSet<usize> = groups
        .iter()
        .flat_map(|g| g.cell.runs.iter().copied())
        .collect();
    let under: Vec<Option<(i64, i64)>> = header
        .cells
        .iter()
        .map(|c| {
            c.runs
                .iter()
                .copied()
                .filter(|i| {
                    !taken.contains(i)
                        && !runs[*i].text.trim().is_empty()
                        && runs[*i].y > line.y + LINE_TOLERANCE
                })
                .map(|i| (runs[i].x, runs[i].rect.map_or(runs[i].x, |b| b.x1)))
                .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
        })
        .collect();
    // The stub's heading names its own column and parts no other cell's.
    let rivals: Vec<bool> = line
        .cells
        .iter()
        .map(|c| track_of(c, tracks) != Some(0) || !clear_of_neighbours(c, 0, tracks))
        .collect();
    let spans: Vec<Option<(usize, usize)>> = (0..line.cells.len())
        .map(|n| group_of(line, n, &rivals, &under, tracks))
        .collect();
    let short = line
        .cells
        .iter()
        .all(|c| cell_text(c, runs).trim().chars().count() < PROSE_LINE_CHARS);
    if whole && (!short || spans.iter().any(Option::is_none)) {
        return None;
    }
    let mut found = Vec::new();
    for (cell, span) in line.cells.iter().zip(&spans) {
        let Some((first, last)) = *span else {
            continue;
        };
        let part = groups
            .iter()
            .any(|g| g.first <= last && first <= g.last && (g.first, g.last) != (first, last));
        if part && whole {
            return None;
        }
        if short && !part {
            found.push((first, last, cell.clone()));
        }
    }
    Some(found)
}

/// The first and last track the `n`th cell of `line` heads as a group head (`-v16`), where it is
/// one: of the tracks past the first whose header ink below (`under`) has its middle nearer this
/// cell's middle than any other cell's of the line — the stub's heading aside (`rivals`) — the
/// widest run of two or more, from one such track to another with only tracks of no ink between,
/// whose ink together holds the cell within [`TRACK_TOLERANCE`] of either end and has its middle
/// within the cell, to a sixteenth of the run's width or a track's tolerance; of two as wide, the
/// one whose middle is nearer the cell's. **None** for a cell on a track clear of its neighbours
/// that is centred over that track's ink below, set flush with it left or right, or set over a
/// track with none — that column's own line — nor for a cell alone on its line and centred on the
/// whole table, within a track's tolerance: its title. The stub's column, the first track, is
/// never a group's.
fn group_of(
    line: &Line,
    n: usize,
    rivals: &[bool],
    under: &[Option<(i64, i64)>],
    tracks: &[(i64, i64)],
) -> Option<(usize, usize)> {
    let cell = &line.cells[n];
    let centre2 = cell.x0 + cell.x1;
    // A column's own line: on a track clear of its neighbours, centred over that track's ink below
    // or flush with it, or over a track with none.
    if let Some(k) = track_of(cell, tracks).filter(|&k| clear_of_neighbours(cell, k, tracks)) {
        if under[k].is_none_or(|(u0, u1)| {
            (2 * u0 <= centre2 && centre2 <= 2 * u1)
                || (cell.x0 - u0).abs() <= TRACK_TOLERANCE
                || (cell.x1 - u1).abs() <= TRACK_TOLERANCE
        }) {
            return None;
        }
    }
    // The table's title: alone on its line, centred on the whole table, the stub's column with it.
    let (w0, w1) = under.iter().flatten().fold(
        (tracks[0].0, tracks[tracks.len() - 1].1),
        |(w0, w1), &(u0, u1)| (w0.min(u0), w1.max(u1)),
    );
    if line.cells.len() == 1 && (w0 + w1 - centre2).abs() <= 2 * TRACK_TOLERANCE {
        return None;
    }
    let ours = |k: usize| {
        under[k].is_some_and(|(x0, x1)| {
            let d = (x0 + x1 - centre2).abs();
            line.cells
                .iter()
                .zip(rivals)
                .enumerate()
                .all(|(m, (c, &rival))| m == n || !rival || (x0 + x1 - c.x0 - c.x1).abs() > d)
        })
    };
    let mut best: Option<(usize, i64, usize, usize)> = None;
    for a in (1..under.len()).filter(|&a| ours(a)) {
        let mut ink: Option<(i64, i64)> = None;
        for (b, track) in under.iter().enumerate().skip(a) {
            let Some((x0, x1)) = *track else {
                continue;
            };
            if !ours(b) {
                break;
            }
            let (u0, u1) = ink.map_or((x0, x1), |(u0, u1)| (u0.min(x0), u1.max(x1)));
            ink = Some((u0, u1));
            let holds = u0 - TRACK_TOLERANCE <= cell.x0 && cell.x1 <= u1 + TRACK_TOLERANCE;
            // The tolerance, not the cell's extent alone: a run reaching one track too far keeps
            // its middle inside a wide cell.
            let off = (u0 + u1 - centre2).abs();
            let centred = 2 * cell.x0 <= u0 + u1
                && u0 + u1 <= 2 * cell.x1
                && 8 * off <= (u1 - u0).max(16 * TRACK_TOLERANCE);
            if holds && centred && best.is_none_or(|(w, o, _, _)| (b - a, -off) > (w, -o)) {
                best = Some((b - a, off, a, b));
            }
        }
    }
    best.and_then(|(w, _, a, b)| (w > 0).then_some((a, b)))
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
    rules: &[Rule],
    recognized: bool,
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
        tables.extend(detect(page, &own, rules, recognized, alloc)?);
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
/// than half that line's em — or, since `-v16` (decision #119), by less than three fifths where it
/// is set against the text before it: each of its runs starts within a quarter em of where another
/// run of either line ends. The line below keeps its own baseline, em and first run.
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
        // `-v16` (decision #119): a mark raised more than half an em is one only where it is set
        // against the text before it — a filing's `Comp sales¹` — and a chart's labels set just
        // above another line's are set against none.
        let set_against = || {
            let others = inked(below);
            small.iter().all(|&i| {
                others.iter().chain(&small).any(|&j| {
                    j != i
                        && runs[j]
                            .rect
                            .is_some_and(|b| 4 * (runs[i].x - b.x1).abs() <= em)
                })
            })
        };
        !small.is_empty()
            && small.iter().all(|&i| runs[i].em.is_some_and(|e| e < em))
            && raised > 0
            && (2 * raised < em || (5 * raised < 3 * em && set_against()))
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
/// the next run's origin is wider than the line's em, after a run ending in dot leaders unless the
/// next is leaders and nothing else (`-v11`), and, in a line that sets its signs before its
/// figures, at a sign set alone after a figure (`-v16`, decision #116) or a run that opens with one
/// (decision #119). A whitespace run belongs to the cell whose extent holds its origin, and opens
/// none.
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
    // `-v16` (decision #116): a line that sets a currency sign before its first figure sets each
    // sign before its figure, so a sign set alone within an em after a figure is the next column's,
    // and opens its cell. `-v16` (decision #119): and so does a run that opens with a sign —
    // `$134,803` within an em after `95,685`.
    let alone_sign = |text: &str| {
        let mut chars = text.trim().chars();
        matches!((chars.next(), chars.next()), (Some(c), None) if CURRENCY_SIGNS.contains(c))
    };
    let prefixed = inked
        .iter()
        .map(|&i| runs[i].text.trim())
        .find(|t| alone_sign(t) || is_number(t))
        .is_some_and(|t| t.starts_with(|c: char| CURRENCY_SIGNS.contains(c)));
    let mut cells: Vec<Cell> = Vec::new();
    let mut after_leader = false;
    let mut after_figure = false;
    for &i in &inked {
        let run = &runs[i];
        // Upright, so measured: `lines` let no other inked run through.
        let end = run.rect.map_or(run.x, |b| b.x1);
        let opens = prefixed
            && after_figure
            && run
                .text
                .trim_start()
                .starts_with(|c: char| CURRENCY_SIGNS.contains(c));
        match cells.last_mut() {
            Some(cell)
                if run.x - cell.x1 <= em && (!after_leader || leaders_only(run.text)) && !opens =>
            {
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
        after_figure = run
            .text
            .trim_end()
            .ends_with(|c: char| c.is_ascii_digit() || c == '%' || c == ')');
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
        // `-v16` (decision #115): a line of years below rows of data, set off from them by a row
        // and a half's pitch or more, heads a table of its own.
        if 5 * gap >= 9 * em && heads_next_table(line, &rows, runs) {
            break;
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
        // `-v16` (decision #117): a figure does not wrap — a line that sets a number, no year, on a
        // track where the row above holds one opens a row at any gap, whatever cells it leaves
        // empty: a stub names its group once. A year heads a statement's next block as often as it
        // is a value, and a number alone on its line is a margin's line number as often as a row.
        let figure = |c: &Cell| is_number(cell_text(c, runs).trim());
        let under = line.cells.iter().zip(&mapping).any(|(c, &k)| {
            figure(c) && !is_year(cell_text(c, runs).trim()) && figure(&above.cells[k])
        });
        if line.cells.len() >= 2 && (5 * gap >= 9 * em || opens_row || under) {
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

/// Whether `line` heads the next table (`-v16`, decision #115): two cells or more, each a year, under
/// rows of which one past the first holds a digit. A statement stacks its years' tables one under
/// another, each headed by its years, and the second's heads joined the first as a sparse row.
fn heads_next_table(line: &Line, rows: &[Row], runs: &[TrackRun<'_>]) -> bool {
    let digits = |c: &Cell| cell_text(c, runs).chars().any(|ch| ch.is_ascii_digit());
    line.cells.len() >= 2
        && rows[1..].iter().any(|r| r.cells.iter().any(digits))
        && line
            .cells
            .iter()
            .all(|c| is_year(cell_text(c, runs).trim()))
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

/// Whether the rows are a table: enough of them, a steady pitch, and written row by row — or
/// `ordered` (`-v16`), one side of rows that stood as a table already.
fn accepted(rows: &[Row], runs: &[TrackRun<'_>], ordered: bool, recognized: bool) -> bool {
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
        // `-v16` (decision #119): small numbers beside a column of numbers key a table of values —
        // ages beside their premiums — and are no list's labels; a list's items are words.
        let values = column(1).iter().all(|t| is_number(t.trim()));
        if (labels(&column(0)) && !values) || is_contents(&column(1)) {
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
    // On a page of recognized text the order is the recognizer's (decision #125), and says nothing.
    if ordered || (!recognized && written_row_by_row(rows, 0..columns)) {
        return true;
    }
    // A grid of three rows and three columns or more with a column of numbers and no column of
    // running text needs no row order: the clause above tells prose from a table, and a table of
    // values exported cell by cell is written down its columns. **Unless its columns divide into
    // two groups each written row by row**: that is two flows side by side — a table beside a
    // column of text — whatever the text's letters say.
    rows.len() >= MIN_ROWS_UNORDERED
        && columns >= MIN_COLUMNS_UNORDERED
        // On a page of recognized text no column of numbers is asked (decision #125): with no order to
        // read, a list of names and marks is a table, and prose is still refused as running text.
        && (recognized || (0..columns).any(|k| numbers(&column(k))))
        && (0..columns).all(|k| !running_text(&column(k)))
        && (recognized
            || !(1..columns).any(|k| {
                written_row_by_row(rows, 0..k)
                    && written_row_by_row(rows, k..columns)
                    && !(k == 1 && label_column(rows, runs))
            }))
}

/// The column the second of two flows opens at (`-v14`), where a candidate written neither row by
/// row nor in one flow divides into two groups of two columns or more, each written row by row and
/// holding a column of numbers none of which ends in dot leaders, with no column of running text:
/// two tables set side by side. Numbers drawn with leaders are a contents list's page numbers.
fn two_tables(rows: &[Row], runs: &[TrackRun<'_>]) -> Option<usize> {
    let columns = rows[0].cells.len();
    let column = |k: usize| -> Vec<String> {
        rows.iter()
            .map(|r| cell_text(&r.cells[k], runs))
            .filter(|t| !t.trim().is_empty())
            .collect()
    };
    if written_row_by_row(rows, 0..columns)
        || rows.len() < MIN_ROWS_UNORDERED
        || (0..columns).any(|k| running_text(&column(k)))
    {
        return None;
    }
    let values = |c: usize| {
        let texts = column(c);
        numbers(&texts) && !texts.iter().any(|t| ends_with_leaders(t))
    };
    (MIN_COLUMNS..=columns.saturating_sub(MIN_COLUMNS)).find(|&k| {
        written_row_by_row(rows, 0..k)
            && written_row_by_row(rows, k..columns)
            && (0..k).any(values)
            && (k..columns).any(values)
    })
}

/// The column a second table opens at (`-v16`, decision #115), where two tables stand side by side
/// and the page draws each its own rules: from four ems above the first row down to `last`, the
/// table's last line, horizontal ink ends in the gap between two tracks on the left and starts in it
/// on the right, each over the two tracks beside the gap, closing its side's last row and standing an
/// em or more above it too — a box's top and bottom, or a header bar and a closing rule — and none
/// crosses the gap from an em above the first row down. A rule drawn cell by cell is one rule; a
/// column group's heading drawn in a box of its own over rows it does not close is no table of its
/// own, and a bar over both tables' titles joins neither.
fn drawn_panels(rows: &[Row], last: i64, rules: &[Rule]) -> Option<usize> {
    let tracks = tracks_of(rows);
    let n = tracks.len();
    let em = rows[0].em;
    let (top, bottom) = (rows[0].y - 4 * em, last + em);
    let mut ink: Vec<(i64, i64, i64)> = rules
        .iter()
        .filter(|r| r.y >= top && r.y <= bottom)
        .map(|r| (r.y, r.x0, r.x1))
        .collect();
    ink.sort_unstable();
    let mut spans: Vec<(i64, i64, i64)> = Vec::with_capacity(ink.len());
    for (y, x0, x1) in ink {
        match spans.last_mut() {
            Some(last)
                if (y - last.0).abs() <= LATTICE_TOLERANCE
                    && x0 >= last.1 - LATTICE_TOLERANCE
                    && x0 <= last.2 + LATTICE_TOLERANCE =>
            {
                last.2 = last.2.max(x1);
            }
            _ => spans.push((y, x0, x1)),
        }
    }
    // Whether ink at `ys` closes the last row holding a cell in `cols`, and stands an em or more
    // above it too.
    let brackets = |ys: Vec<i64>, cols: std::ops::Range<usize>| {
        rows.iter()
            .filter(|r| r.cells[cols.clone()].iter().any(|c| !c.runs.is_empty()))
            .map(|r| r.y)
            .next_back()
            .is_some_and(|last| ys.iter().any(|&y| y >= last) && ys.iter().any(|&y| y <= last - em))
    };
    (MIN_COLUMNS..=n.saturating_sub(MIN_COLUMNS)).find(|&k| {
        let (gap0, gap1) = (tracks[k - 1].1, tracks[k].0);
        let left: Vec<i64> = spans
            .iter()
            .filter(|&&(_, x0, x1)| {
                x0 <= tracks[k - 2].0 + TRACK_TOLERANCE
                    && x1 >= gap0 - TRACK_TOLERANCE
                    && x1 <= gap1
            })
            .map(|s| s.0)
            .collect();
        let right: Vec<i64> = spans
            .iter()
            .filter(|&&(_, x0, x1)| {
                x0 >= gap0
                    && x0 <= gap1 + TRACK_TOLERANCE
                    && x1 >= tracks[k + 1].1 - TRACK_TOLERANCE
            })
            .map(|s| s.0)
            .collect();
        gap0 < gap1
            && !spans
                .iter()
                .any(|&(y, x0, x1)| y >= rows[0].y - em && x0 < gap0 && x1 > gap1)
            && brackets(left, 0..k)
            && brackets(right, k..n)
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

/// The table the rows make, under a group row of `groups` where there are any (`-v16`), or `None`
/// where a run of it has no measured box to place it by.
fn build(
    page: u32,
    rows: &[Row],
    groups: &[Group],
    runs: &[TrackRun<'_>],
    alloc: &mut IdAllocator,
) -> Result<Option<DetectedTable>, EngineError> {
    let columns = rows[0].cells.len();
    // `-v16`: the group row, first, where there is one.
    let lead = usize::from(!groups.is_empty());
    let count = rows.len() + lead;
    // Each column's and each row's extent: the union of its inked runs' boxes. A group head's
    // runs set its row's extent and no column's.
    let mut xs: Vec<Option<(i64, i64)>> = vec![None; columns];
    let mut ys: Vec<Option<(i64, i64)>> = vec![None; count];
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
                widen(&mut ys[r + lead], b.y0, b.y1);
            }
        }
    }
    for group in groups {
        for &i in group
            .cell
            .runs
            .iter()
            .filter(|&&i| !runs[i].text.trim().is_empty())
        {
            let Some(b) = runs[i].rect else {
                return Ok(None);
            };
            widen(&mut ys[0], b.y0, b.y1);
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
        y1: ys[count - 1].1,
    };
    if table_rect.as_qrect_checked().is_err() {
        return Ok(None);
    }
    let id = alloc.next(IdKind::Table)?;
    let mut cells = Vec::with_capacity(count * columns);
    let mut push = |r: usize, c: usize, span: usize, of: &[usize]| {
        let mut run_indices = of.to_vec();
        run_indices.sort_unstable();
        let text: String = run_indices.iter().map(|&i| runs[i].text).collect();
        cells.push(DetectedCell {
            position: ethos_parser_core::TableCellPosition {
                row: r as u32,
                column: c as u32,
                rowspan: 1,
                colspan: span as u32,
                table_id: id.clone(),
            },
            rect: QuantRect {
                x0: xs[c].0,
                y0: ys[r].0,
                x1: xs[c + span - 1].1,
                y1: ys[r].1,
            },
            run_indices,
            text,
        });
    };
    // `-v16`: a group head spans the tracks it heads; every other track is an empty cell.
    let mut c = 0;
    while lead == 1 && c < columns {
        let (span, of) = groups
            .iter()
            .find(|g| g.first == c)
            .map_or((1, &[][..]), |g| (g.last - g.first + 1, &g.cell.runs[..]));
        push(0, c, span, of);
        c += span;
    }
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.cells.iter().enumerate() {
            push(r + lead, c, 1, &cell.runs);
        }
    }
    let check = cross_check(table_rect, count as u32, columns as u32, &cells);
    Ok(Some(DetectedTable {
        id,
        page,
        rect: table_rect,
        rows: count as u32,
        columns: columns as u32,
        cells,
        check,
        tagged_check: None,
        rule: ethos_parser_core::TABLE_DETECTION_TRACKS_V16.to_string(),
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
        detect(1, runs, &[], false, &mut alloc()).expect("detects")
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

    /// **A header's last line read as the first row is the header's** (`-v13`): `Units` and
    /// `Price`, a row's pitch above the first row of figures, open the table themselves, and the
    /// band takes only `Number of` and `Average` above them; they join the header row, each
    /// column's lines top to bottom. Two columns may end on one word.
    #[test]
    fn a_header_s_last_line_read_as_the_first_row_is_the_header_s() {
        let lines = |a: &'static str, b: &'static str| {
            under(vec![
                run(235, 172, 35, "Number of "),
                run(335, 172, 35, "Average "),
                run(100, 184, 40, "Region"),
                run(245, 184, 25, a),
                run(345, 184, 25, b),
            ])
        };
        let found = lines("Units", "Price");
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(
            texts(&found[0])[0],
            ["Region", "Number of Units", "Average Price"]
        );
        assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0"]);
        assert_eq!(
            texts(&lines("Factor", "Factor")[0])[0],
            ["Region", "Number of Factor", "Average Factor"],
            "two columns ending on one word"
        );
        // A header that cannot be built with its last line leaves that line the first row again.
        let found = under(vec![
            run(105, 172, 25, "Line"),
            run(190, 172, 240, "Units and price"),
            run(100, 184, 40, "Region"),
            run(245, 184, 25, "Units"),
            run(345, 184, 25, "Price"),
        ]);
        assert_eq!(texts(&found[0])[0], ["Region", "Units", "Price"]);
        assert_eq!(found[0].rows, 5);
    }

    /// A four-column ledger with `first` as its first row a row's pitch above its figures, under a
    /// band naming its three columns of numbers — or only the last, a caption's worth. South's
    /// change is `None`.
    fn ledger(band: bool, first: [&'static str; 4]) -> Vec<DetectedTable> {
        let band = if band {
            vec![
                run(235, 172, 35, "Current "),
                run(315, 172, 35, "Prior "),
                run(395, 172, 35, "Change "),
            ]
        } else {
            vec![run(395, 172, 35, "Change ")]
        };
        ledger_under(band, first)
    }

    /// [`ledger`]'s four columns, `first` the first row, under the lines of `band`.
    fn ledger_under(band: Vec<TrackRun<'static>>, first: [&'static str; 4]) -> Vec<DetectedTable> {
        let mut runs = band;
        let x = [100, 240, 320, 400];
        let width = |k: usize| if k == 0 { 40 } else { 30 };
        for (k, text) in first.into_iter().enumerate() {
            runs.push(run(x[k], 184, width(k), text));
        }
        let rows = [
            ["North", "7.5", "112.0", "1.5"],
            ["South", "18.25", "9.1", "None"],
            ["East", "4.0", "65.5", "2.25"],
            ["West", "12.0", "70.25", "0.5"],
        ];
        for (j, row) in rows.into_iter().enumerate() {
            for (k, text) in row.into_iter().enumerate() {
                runs.push(run(x[k], 200 + 16 * j as i64, width(k), text));
            }
        }
        tables(&runs)
    }

    /// **A first row is the header's only where it reads as one** (`-v13`): a row of placeholders,
    /// a cell repeating a value of its column, a digit, or a cell as long as a line of prose on a
    /// column of numbers stays a row; so does any first row under a band that names
    /// a column of four — a caption, whose first row is the header itself — and the first row of a
    /// table of words, which has no column of numbers to tell a header by.
    #[test]
    fn a_first_row_is_the_header_s_only_where_it_reads_as_one() {
        let found = ledger(true, ["Region", "Units", "Price", "Delta"]);
        assert_eq!(
            texts(&found[0])[0],
            ["Region", "Current Units", "Prior Price", "Change Delta"]
        );
        for (first, why) in [
            (["Region", "NA", "NA", "NA"], "placeholders"),
            (
                ["Region", "Units", "Price", "None"],
                "a value of its column",
            ),
            (["Region", "Units", "2024", "Delta"], "a digit"),
            (
                [
                    "Region",
                    "Units sold over the whole quarter",
                    "Price",
                    "Delta",
                ],
                "a line of prose",
            ),
        ] {
            let found = ledger(true, first);
            assert_eq!(
                texts(&found[0])[0],
                ["", "Current ", "Prior ", "Change "],
                "{why}"
            );
            assert_eq!(texts(&found[0])[1], first, "{why}");
        }
        let first = ["Region", "Units", "Price", "Delta"];
        assert_eq!(texts(&ledger(false, first)[0])[0], first, "a caption");
        let mut words = vec![
            run(235, 172, 35, "Current "),
            run(335, 172, 35, "Planned "),
            run(100, 184, 40, "Region"),
            run(240, 184, 30, "Status"),
            run(340, 184, 30, "Phase"),
        ];
        for (j, row) in [
            ["North", "Open", "One"],
            ["South", "Shut", "Two"],
            ["East", "Held", "Six"],
            ["West", "Done", "Ten"],
        ]
        .into_iter()
        .enumerate()
        {
            let y = 200 + 16 * j as i64;
            words.extend([
                run(100, y, 40, row[0]),
                run(240, y, 30, row[1]),
                run(340, y, 30, row[2]),
            ]);
        }
        assert_eq!(
            texts(&tables(&words)[0])[1],
            ["Region", "Status", "Phase"],
            "a table of words"
        );
    }

    /// **A band naming half the tracks or fewer is its last line's upper lines** (`-v16`, decision
    /// #118): `Current` and `Prior`, one line of two cells over two columns of four, join the first
    /// row that reads as the header's last line, each column's lines top to bottom. Over a first row
    /// that reads as no header's line they head nothing; nor do lines of one cell each — a title on
    /// the stub's track and a stamp over the last column, two tracks between them.
    #[test]
    fn a_band_naming_few_tracks_is_its_last_line_s_upper_lines() {
        let upper = || vec![run(235, 172, 35, "Current "), run(315, 172, 35, "Prior ")];
        let found = ledger_under(upper(), ["Region", "Units", "Price", "Delta"]);
        assert_eq!((found[0].rows, found[0].columns), (5, 4));
        assert_eq!(
            texts(&found[0])[0],
            ["Region", "Current Units", "Prior Price", "Delta"]
        );
        assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0", "1.5"]);
        let first = ["Region", "Units", "2024", "Delta"];
        assert_eq!(
            texts(&ledger_under(upper(), first)[0])[0],
            first,
            "a first row that is no header's"
        );
        let first = ["Region", "Units", "Price", "Delta"];
        let found = ledger_under(
            vec![run(100, 164, 40, "Rates"), run(395, 172, 35, "Proposed")],
            first,
        );
        assert_eq!(texts(&found[0])[0], first, "one cell to a line");
    }

    /// **A header's last line may name years** (`-v15`, decision #96): `2024` and `2023` — or
    /// `1999` and `1998` — each a cell of its own over a column of figures, under `Current` and
    /// `Prior`, are the header's. A year over a column of years, a number that is no year beside
    /// the years, five digits and a year's letters leave the line a row.
    #[test]
    fn a_header_s_last_line_may_name_years() {
        for years in [["2024", "2023"], ["1999", "1998"]] {
            let found = ledger(true, ["Region", years[0], years[1], "Delta"]);
            assert_eq!(
                texts(&found[0])[0],
                [
                    "Region".to_string(),
                    format!("Current {}", years[0]),
                    format!("Prior {}", years[1]),
                    "Change Delta".to_string()
                ]
            );
            assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0", "1.5"]);
        }
        for (first, why) in [
            (["Region", "2024", "24", "Delta"], "a number no year"),
            (["Region", "2024", "20235", "Delta"], "five digits"),
            (["Region", "2024", "20x4", "Delta"], "a year's letters"),
            (
                ["Region", "2024", "2023", "18"],
                "a number beside the years",
            ),
        ] {
            let found = ledger(true, first);
            assert_eq!(
                texts(&found[0])[0],
                ["", "Current ", "Prior ", "Change "],
                "{why}"
            );
            assert_eq!(texts(&found[0])[1], first, "{why}");
        }
        let mut runs = vec![
            run(235, 172, 35, "Current "),
            run(315, 172, 35, "Prior "),
            run(100, 184, 40, "Region"),
            run(240, 184, 30, "2024"),
            run(320, 184, 30, "2023"),
        ];
        for (j, row) in [
            ["North", "1998", "112.0"],
            ["South", "2003", "9.1"],
            ["East", "2010", "65.5"],
            ["West", "2015", "70.25"],
        ]
        .into_iter()
        .enumerate()
        {
            let y = 200 + 16 * j as i64;
            runs.extend([
                run(100, y, 40, row[0]),
                run(240, y, 30, row[1]),
                run(320, y, 30, row[2]),
            ]);
        }
        assert_eq!(
            texts(&tables(&runs)[0])[1],
            ["Region", "2024", "2023"],
            "a year over a column of years"
        );
    }

    /// `lines` — each a baseline in points and its three cells, set on the columns of [`values`],
    /// an empty one left out — above [`values`]' four rows.
    fn headed_by(lines: &[(i64, [&'static str; 3])]) -> Vec<DetectedTable> {
        let mut runs = Vec::new();
        for &(y, cells) in lines {
            for (x, text) in [100, 240, 335].into_iter().zip(cells) {
                if !text.is_empty() {
                    runs.push(run(x, y, 30, text));
                }
            }
        }
        under(runs)
    }

    /// **A header whose lines each read as a row is one row** (`-v15`, decision #96): `Year`,
    /// `Number of` and `Average` over `Ending`, `Units` and `Price` — each line a cell on every
    /// column, set closer to the next than the first row of figures is to them — are one header,
    /// each column's lines top to bottom; so are three lines and four. Set at the rows' own pitch
    /// they are two rows, as they are where the lower line holds a digit, leaves a column of numbers
    /// empty or repeats a value its column holds, and five lines are no header.
    #[test]
    fn a_header_whose_lines_each_read_as_a_row_is_one_row() {
        let found = headed_by(&[
            (176, ["Year ", "Number of ", "Average "]),
            (186, ["Ending", "Units", "Price"]),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(
            texts(&found[0])[0],
            ["Year Ending", "Number of Units", "Average Price"]
        );
        assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0"]);
        let found = headed_by(&[
            (172, ["Year ", "Number ", "Average "]),
            (180, ["Ending ", "of ", "Unit "]),
            (188, ["Date", "Units", "Price"]),
        ]);
        assert_eq!(found[0].rows, 5);
        assert_eq!(
            texts(&found[0])[0],
            ["Year Ending Date", "Number of Units", "Average Unit Price"]
        );
        let found = headed_by(&[
            (168, ["Year ", "Number ", "Average "]),
            (175, ["Ending ", "of ", "Unit "]),
            (182, ["Close ", "Sold ", "Sale "]),
            (189, ["Date", "Units", "Price"]),
        ]);
        assert_eq!(found[0].rows, 5, "four lines");
        assert_eq!(
            texts(&found[0])[0],
            [
                "Year Ending Close Date",
                "Number of Sold Units",
                "Average Unit Sale Price"
            ]
        );
        let five = headed_by(&[
            (164, ["Year ", "Number ", "Average "]),
            (171, ["Ending ", "of ", "Unit "]),
            (178, ["Close ", "Sold ", "Sale "]),
            (185, ["Date ", "Units ", "Price "]),
            (192, ["Here", "Now", "Then"]),
        ]);
        assert_eq!(five[0].rows, 9, "five lines");
        for (upper, lower, why) in [
            (168, ["Ending", "Units", "Price"], "set at the rows' pitch"),
            (176, ["Ending", "Units", "Price (1)"], "a digit"),
            (
                176,
                ["Ending", "Units", ""],
                "a column of numbers left empty",
            ),
            (176, ["North", "Units", "Price"], "a value its column holds"),
        ] {
            let found = headed_by(&[
                (upper, ["Year ", "Number of ", "Average "]),
                (upper + if upper == 168 { 16 } else { 10 }, lower),
            ]);
            assert_eq!(found[0].rows, 6, "{why}");
            assert_eq!(
                texts(&found[0])[0],
                ["Year ", "Number of ", "Average "],
                "{why}"
            );
            assert_eq!(texts(&found[0])[1], lower, "{why}");
        }
    }

    /// **A header is the most lines set closer than the rows below, each measured where the one
    /// above it ends** (`-v15`, decision #96): lines 8 and then 10 points apart over a row 12 below
    /// are one header of three; 8 and then 12 apart over a row 10 below, a header of two over a row
    /// of words. A line whose wrap ends 12 points over the first row of figures is set closer to
    /// them than to the line 14 above it, and stays a row. And two lines over a single row of
    /// figures are its header.
    #[test]
    fn a_header_is_its_closest_lines_measured_where_each_ends() {
        let three = headed_by(&[
            (170, ["Year ", "Number ", "Average "]),
            (178, ["Ending ", "of ", "Unit "]),
            (188, ["Date", "Units", "Price"]),
        ]);
        assert_eq!(three[0].rows, 5);
        assert_eq!(
            texts(&three[0])[0],
            ["Year Ending Date", "Number of Units", "Average Unit Price"]
        );
        let two = headed_by(&[
            (170, ["Year ", "Number of ", "Average "]),
            (178, ["Ending", "Units", "Price"]),
            (190, ["Total", "Count", "Rate"]),
        ]);
        assert_eq!(two[0].rows, 6);
        assert_eq!(
            texts(&two[0])[0],
            ["Year Ending", "Number of Units", "Average Price"]
        );
        assert_eq!(texts(&two[0])[1], ["Total", "Count", "Rate"]);
        let wrapped = under(vec![
            run(100, 166, 30, "Year "),
            run(240, 166, 30, "Number "),
            run(335, 166, 30, "Average "),
            run(100, 180, 30, "Ending"),
            run(240, 180, 30, "of Units "),
            run(335, 180, 30, "Unit "),
            run(240, 188, 30, "Sold"),
            run(335, 188, 30, "Price"),
        ]);
        assert_eq!(wrapped[0].rows, 6, "a wrapped line");
        assert_eq!(
            texts(&wrapped[0])[1],
            ["Ending", "of Units Sold", "Unit Price"]
        );
        let one = tables(&[
            run(100, 176, 30, "Year "),
            run(240, 176, 30, "Number of "),
            run(335, 176, 30, "Average "),
            run(100, 186, 30, "Ending"),
            run(240, 186, 30, "Units"),
            run(335, 186, 30, "Price"),
            run(100, 200, 40, "North"),
            run(250, 200, 20, "7.5"),
            run(330, 200, 40, "112.0"),
        ]);
        assert_eq!(one[0].rows, 2, "a single row of figures");
        assert_eq!(
            texts(&one[0])[0],
            ["Year Ending", "Number of Units", "Average Price"]
        );
    }

    /// **A header's lines go on below its band** (`-v15`, decision #96): `in Millions`, `Sold`
    /// and `Paid`, set under the header the band and its last line made, closer to it than the
    /// first row of figures is, are its third line. And where that header cannot be built with —
    /// `Units and price` covering a neighbour's column — its lines stand as rows again, every one.
    #[test]
    fn a_header_s_lines_go_on_below_its_band() {
        let found = under(vec![
            run(235, 172, 35, "Number of "),
            run(335, 172, 35, "Average "),
            run(100, 182, 40, "Region "),
            run(245, 182, 25, "Units "),
            run(345, 182, 25, "Price "),
            run(100, 190, 40, "in Millions"),
            run(245, 190, 25, "Sold"),
            run(345, 190, 25, "Paid"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(
            texts(&found[0])[0],
            [
                "Region in Millions",
                "Number of Units Sold",
                "Average Price Paid"
            ]
        );
        let found = under(vec![
            run(105, 172, 25, "Line"),
            run(190, 172, 240, "Units and price"),
            run(100, 182, 40, "Region "),
            run(245, 182, 25, "Units "),
            run(345, 182, 25, "Price "),
            run(100, 190, 40, "in Millions"),
            run(245, 190, 25, "Sold"),
            run(345, 190, 25, "Paid"),
        ]);
        assert_eq!(found[0].rows, 6);
        assert_eq!(texts(&found[0])[0], ["Region ", "Units ", "Price "]);
        assert_eq!(texts(&found[0])[1], ["in Millions", "Sold", "Paid"]);
    }

    /// **The band is a header's and nothing else's**: more than two and a half rows' pitch above the
    /// rows (`-v15`) it is apart from them; a line naming one column of three is a title, not a
    /// header; a cell of
    /// thirty characters is a caption; a cell repeating a value its column holds is a row the
    /// table did not take. Each leaves the table as it was.
    #[test]
    fn a_band_apart_titled_captioned_or_repeating_is_no_header() {
        for (band, why) in [
            (
                vec![run(105, 169, 25, "Line"), run(240, 169, 25, "Type")],
                "thirty-one points above the first row",
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

    /// **A header's first line may stand two and a half rows' pitch above the table** (`-v15`): 30
    /// points over 10-point rows, where a rule and its padding set a header further off its first
    /// row than `-v7`'s two; a point further and it heads nothing.
    #[test]
    fn a_header_two_and_a_half_rows_pitch_above_is_the_table_s() {
        let found = under(vec![run(105, 170, 25, "Line"), run(240, 170, 25, "Type")]);
        assert_eq!(texts(&found[0])[0], ["Line", "Type", ""]);
        let found = under(vec![run(105, 169, 25, "Line"), run(240, 169, 25, "Type")]);
        assert_eq!(
            texts(&found[0])[0],
            ["North", "7.5", "112.0"],
            "31 points above"
        );
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

    /// Four rows of a region's figures in four columns, the first at 200 points, under `head` —
    /// the lines of a header set above them, in content order before the rows.
    fn channels(head: Vec<TrackRun<'static>>) -> Vec<DetectedTable> {
        let mut runs = head;
        for (j, row) in [
            ["North", "7.5", "11.0", "8.1", "9.4"],
            ["South", "18.2", "9.1", "6.5", "7.7"],
            ["East", "4.0", "65.5", "5.2", "3.3"],
            ["West", "12.0", "70.2", "4.8", "2.9"],
        ]
        .into_iter()
        .enumerate()
        {
            let y = 200 + 16 * j as i64;
            runs.push(run(100, y, 40, row[0]));
            for (k, text) in row[1..].iter().enumerate() {
                runs.push(run([200, 250, 330, 380][k], y, 25, text));
            }
        }
        tables(&runs)
    }

    /// The header row of [`channels`], a row's pitch above its figures.
    fn names(y: i64) -> Vec<TrackRun<'static>> {
        vec![
            run(100, y, 40, "Region"),
            run(200, y, 25, "Units"),
            run(250, y, 25, "Price"),
            run(330, y, 25, "Units"),
            run(380, y, 25, "Price"),
        ]
    }

    /// Each cell of a table's group row, its text and the columns it spans.
    fn groups(table: &DetectedTable) -> Vec<(String, u32)> {
        table
            .cells
            .iter()
            .filter(|c| c.position.row == 0)
            .map(|c| (c.text.clone(), c.position.colspan))
            .collect()
    }

    /// **A header cell set over several columns heads them all** (`-v16`, decision #112): `Retail`
    /// centred over the first pair of figures' names and `Online` over the second, a line two ems
    /// above the header row the table opened at — as far off it as a band's first line may stand
    /// off a table — are a group row over it, each a cell spanning the two columns it heads, the
    /// stub's column an empty cell, where they had been left out of the table, its header only
    /// `Units` and `Price`.
    #[test]
    fn a_header_cell_set_over_several_columns_heads_them_all() {
        let mut head = vec![run(222, 164, 31, "Retail"), run(352, 164, 31, "Online")];
        head.extend(names(184));
        let found = channels(head);
        assert_eq!((found[0].rows, found[0].columns), (6, 5));
        assert_eq!(
            groups(&found[0]),
            [
                (String::new(), 1),
                ("Retail".to_string(), 2),
                ("Online".to_string(), 2)
            ]
        );
        assert_eq!(
            texts(&found[0])[1],
            ["Region", "Units", "Price", "Units", "Price"]
        );
        assert_eq!(texts(&found[0])[2], ["North", "7.5", "11.0", "8.1", "9.4"]);
        // The spans cover every slot once.
        if let ethos_parser_core::CheckStatus::Mismatch { structural, .. } = &found[0].check.outcome
        {
            assert!(structural.is_empty(), "{structural:?}");
        }
    }

    /// **And a group head the band stacked on one column is taken off it**: `Retail` and
    /// `Online` over `Units` and `Price`, the band's two lines above the first row of figures, made
    /// the header `Retail Units` / `Price` / `Online Units` / `Price`; now the group row heads the
    /// pairs and the header row names each column once. Two lines heading the same columns are one
    /// group head, its lines top to bottom.
    #[test]
    fn a_group_head_in_the_band_heads_its_columns_and_leaves_theirs() {
        let mut head = vec![run(222, 172, 31, "Retail "), run(352, 172, 31, "Online ")];
        head.extend(names(184).into_iter().skip(1));
        let found = channels(head);
        assert_eq!((found[0].rows, found[0].columns), (6, 5));
        assert_eq!(
            groups(&found[0]),
            [
                (String::new(), 1),
                ("Retail ".to_string(), 2),
                ("Online ".to_string(), 2)
            ]
        );
        assert_eq!(
            texts(&found[0])[1],
            ["", "Units", "Price", "Units", "Price"]
        );
        // A group head set on two lines, the upper over the very columns of the lower.
        let mut head = vec![
            run(222, 160, 31, "Store "),
            run(352, 160, 31, "Web "),
            run(222, 172, 31, "Retail"),
            run(352, 172, 31, "Online"),
        ];
        head.extend(names(184));
        let found = channels(head);
        assert_eq!(
            groups(&found[0]),
            [
                (String::new(), 1),
                ("Store Retail".to_string(), 2),
                ("Web Online".to_string(), 2)
            ]
        );
    }

    /// **Narrow heads each take the columns nearest them**: `P` to `S`, each centred between the
    /// two columns it heads, are four group heads of two columns each — a column nearer another
    /// head's middle than its own is none of its.
    #[test]
    fn narrow_group_heads_each_take_the_columns_nearest_them() {
        let mut runs = Vec::new();
        for (k, letter) in ["P", "Q", "R", "S"].into_iter().enumerate() {
            runs.push(run(235 + 100 * k as i64, 172, 5, letter));
        }
        for k in 0..8 {
            runs.push(run(200 + 50 * k, 184, 25, ["Cash", "Card"][k as usize % 2]));
        }
        for (j, label) in ["North", "South", "East", "West"].into_iter().enumerate() {
            let y = 200 + 16 * j as i64;
            runs.push(run(100, y, 40, label));
            for k in 0..8 {
                runs.push(run(
                    200 + 50 * k,
                    y,
                    25,
                    ["7.5", "11.0"][(j + k as usize) % 2],
                ));
            }
        }
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (6, 9));
        assert_eq!(
            groups(&found[0]),
            [
                (String::new(), 1),
                ("P".to_string(), 2),
                ("Q".to_string(), 2),
                ("R".to_string(), 2),
                ("S".to_string(), 2)
            ]
        );
    }

    /// Six columns of fares by zone, `Zone` and `head` on one line of the band over the columns'
    /// names `Mon` to `Sat`, which leave the stub's column to `Zone`.
    fn fares(head: TrackRun<'static>) -> Vec<DetectedTable> {
        let mut runs = vec![run(100, 172, 20, "Zone"), head];
        for (k, name) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
            .into_iter()
            .enumerate()
        {
            runs.push(run(200 + 50 * k as i64, 184, 20, name));
        }
        for (j, zone) in ["Z1", "Z2", "Z3", "Z4"].into_iter().enumerate() {
            let y = 200 + 16 * j as i64;
            runs.push(run(100, y, 40, zone));
            for k in 0..6 {
                runs.push(run(
                    200 + 50 * k,
                    y,
                    20,
                    ["4.25", "5.75"][(j + k as usize) % 2],
                ));
            }
        }
        tables(&runs)
    }

    /// **A group head takes the widest run of columns centred on it**: `Opening hours`, centred
    /// over all six columns of fares, heads all six — not the four or two at their middle, whose
    /// middles are its own as well — though `Zone`, the stub's heading on its line, sits nearer the
    /// first; and `Late opening`, centred over the last five, heads those five, the run of
    /// six keeping its middle inside the wide cell but a sixteenth of its width off the cell's.
    #[test]
    fn a_group_head_takes_the_widest_run_of_columns_centred_on_it() {
        let found = fares(run(300, 172, 70, "Opening hours"));
        assert_eq!((found[0].rows, found[0].columns), (6, 7));
        assert_eq!(
            groups(&found[0]),
            [(String::new(), 1), ("Opening hours".to_string(), 6)]
        );
        assert_eq!(
            texts(&found[0])[1],
            ["Zone", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
        );
        let found = fares(run(315, 172, 90, "Late opening"));
        assert_eq!(
            groups(&found[0]),
            [
                (String::new(), 1),
                (String::new(), 1),
                ("Late opening".to_string(), 5)
            ]
        );
    }

    /// **A cell over one column, over the stub, or over the whole table heads no group**: a cell
    /// centred over one column's name is that column's line, though the run of three columns
    /// around it is centred on it too; a cell reaching over the stub is no group's, the stub's
    /// column being never one; a cell as long as a line of prose is none, and neither is the rest
    /// of its line; a title alone on its line, centred over the whole table, is its title; a head
    /// over part of what a head below it heads is none, and the climb ends there; and a cell over
    /// two columns with no header under it heads nothing.
    #[test]
    fn a_cell_over_one_column_the_stub_or_the_table_heads_no_group() {
        let found = fares(run(247, 172, 26, "Peak "));
        assert_eq!(found[0].rows, 5, "over one column");
        assert_eq!(
            texts(&found[0])[0],
            ["Zone", "Mon", "Peak Tue", "Wed", "Thu", "Fri", "Sat"]
        );
        for (line, why) in [
            (
                vec![run(120, 172, 100, "Region and units")],
                "over the stub",
            ),
            (
                vec![
                    run(222, 164, 31, "Retail outlets and wholesalers"),
                    run(352, 164, 31, "Online"),
                ],
                "a line of prose",
            ),
        ] {
            let mut head = line;
            head.extend(names(184));
            let found = channels(head);
            assert_eq!(found[0].rows, 5, "{why}");
            assert_eq!(
                texts(&found[0])[0],
                ["Region", "Units", "Price", "Units", "Price"],
                "{why}"
            );
        }
        // A title alone on its line, centred over a table whose stub is narrow, is centred too over
        // the run of its columns of figures that leaves the last out; it heads none of them.
        let mut runs = vec![run(185, 172, 150, "Quarterly summary")];
        runs.push(run(100, 184, 20, "Week"));
        for k in 0..6 {
            runs.push(run(
                150 + 50 * k,
                184,
                20,
                ["Ash", "Elm", "Oak", "Fir", "Yew", "All"][k as usize],
            ));
        }
        for j in 0..4 {
            let y = 200 + 16 * j;
            runs.push(run(100, y, 20, ["W1", "W2", "W3", "W4"][j as usize]));
            for k in 0..6 {
                runs.push(run(
                    150 + 50 * k,
                    y,
                    20,
                    ["1.5", "-2.0"][(j + k) as usize % 2],
                ));
            }
        }
        let found = tables(&runs);
        assert_eq!(found[0].rows, 5, "a title");
        assert_eq!(texts(&found[0])[0][0], "Week", "a title");
        // `All channels` over all four columns of figures, above `Retail` and `Online`.
        let mut head = vec![
            run(260, 160, 60, "All channels"),
            run(222, 172, 31, "Retail"),
            run(352, 172, 31, "Online"),
        ];
        head.extend(names(184));
        let found = channels(head);
        assert_eq!(
            groups(&found[0]),
            [
                (String::new(), 1),
                ("Retail".to_string(), 2),
                ("Online".to_string(), 2)
            ],
            "a head over part of another's"
        );
        assert_eq!(
            found[0].rect.y0, 16_400,
            "the table starts at the group row, its line left out"
        );
        // `Retail` and `Online` straight over the figures, no header row between.
        let found = channels(vec![
            run(222, 184, 31, "Retail"),
            run(352, 184, 31, "Online"),
        ]);
        assert_eq!(texts(&found[0])[0], ["North", "7.5", "11.0", "8.1", "9.4"]);
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
    /// the first column of a header line that names the others. Alone on its line under that header
    /// the long cell ended the band, and the table stood without one; since decision #118 (`-v16`) a
    /// line of one cell on the stub's track is a section's label at any length, the header above it
    /// heads the table, and the label is a row under it.
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
        let found = under(vec![
            run(205, 170, 30, "Units"),
            run(300, 170, 30, "Price"),
            run(100, 186, 45, "(in millions of dollars, except ratios)"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (6, 3));
        assert_eq!(texts(&found[0])[0], ["", "Units", "Price"]);
        assert_eq!(
            texts(&found[0])[1],
            ["(in millions of dollars, except ratios)", "", ""]
        );
        assert_eq!(texts(&found[0])[2], ["North", "7.5", "112.0"]);
    }

    /// **A header over a section's label heads the table** (`-v16`, decision #118): `2024` and
    /// `2023` over `(in millions)`, a units note on no track, over `Revenues`, a label on the stub's
    /// track — the band took `Revenues` alone, one column of three, and ended at the note. The years
    /// head the table, `Revenues` is a row under them, and the note is in no cell. A label as long
    /// as a line of prose may stand two and a half rows' pitch above the rows.
    #[test]
    fn a_header_over_a_section_s_label_heads_the_table() {
        let found = under(vec![
            run(245, 170, 25, "2024"),
            run(345, 170, 25, "2023"),
            run(280, 180, 40, "(in millions)"),
            run(100, 190, 40, "Revenues"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (6, 3));
        assert_eq!(texts(&found[0])[0], ["", "2024", "2023"]);
        assert_eq!(texts(&found[0])[1], ["Revenues", "", ""]);
        assert_eq!(texts(&found[0])[2], ["North", "7.5", "112.0"]);
        assert!(
            found[0].cells.iter().all(|c| !c.text.contains("millions")),
            "the note heads none"
        );
        let found = under(vec![
            run(245, 166, 25, "Units"),
            run(345, 166, 25, "Price"),
            run(100, 176, 40, "Senior secured debt by subsidiary"),
        ]);
        assert_eq!((found[0].rows, found[0].columns), (6, 3), "a long label");
        assert_eq!(texts(&found[0])[0], ["", "Units", "Price"]);
        assert_eq!(
            texts(&found[0])[1],
            ["Senior secured debt by subsidiary", "", ""]
        );
    }

    /// **…and nothing else heads it so** (`-v16`, decision #118): a band over the note holding a
    /// number that is no year is another table's rows; a caption as long as a line of prose on no
    /// track is no units note; a note further than a row and a half's pitch above a label ends the
    /// climb; a table of words has no column of numbers — a form's field line over its instruction
    /// is no header; a band naming most columns on its own heads the table as `-v5` reads it, the
    /// label its line; and a first row that reads as the header's last line keeps its place. Each
    /// leaves the table as it was.
    #[test]
    fn a_header_over_a_label_holds_no_number_and_stands_over_figures() {
        for (band, why) in [
            (
                vec![
                    run(245, 170, 25, "100"),
                    run(345, 170, 25, "200"),
                    run(280, 186, 40, "(in millions)"),
                ],
                "a number no year",
            ),
            (
                vec![
                    run(245, 170, 25, "Units"),
                    run(345, 170, 25, "Price"),
                    run(280, 186, 40, "Figures restated for the merger"),
                ],
                "a caption",
            ),
            (
                vec![
                    run(245, 158, 25, "2024"),
                    run(345, 158, 25, "2023"),
                    run(280, 168, 40, "(in millions)"),
                    run(100, 190, 40, "Revenues"),
                ],
                "a note too far above the label",
            ),
        ] {
            let found = under(band);
            assert_eq!((found[0].rows, found[0].columns), (4, 3), "{why}");
            assert_eq!(texts(&found[0])[0], ["North", "7.5", "112.0"], "{why}");
        }
        let mut form = vec![
            run(100, 178, 40, "Library:"),
            run(240, 178, 30, "OVLLIB"),
            run(100, 188, 40, "Type choices, then press Enter."),
        ];
        for (j, row) in [
            ["Queue", "*JOB", "Name"],
            ["Library", "*LIBL", "Name"],
            ["Copies", "1", "1-255"],
            ["Fidelity", "*TEXT", "*ALL"],
        ]
        .into_iter()
        .enumerate()
        {
            let y = 200 + 16 * j as i64;
            form.extend([
                run(100, y, 40, row[0]),
                run(240, y, 30, row[1]),
                run(340, y, 30, row[2]),
            ]);
        }
        assert_eq!(
            texts(&tables(&form)[0])[0],
            ["Queue", "*JOB", "Name"],
            "a form"
        );
        let found = under(vec![
            run(245, 180, 25, "Units"),
            run(345, 180, 25, "Price"),
            run(100, 190, 40, "Revenues"),
        ]);
        assert_eq!(
            texts(&found[0])[0],
            ["Revenues", "Units", "Price"],
            "a band naming most columns"
        );
        let first = ["Region", "Units", "Price", "Delta"];
        let found = ledger_under(
            vec![
                run(235, 164, 35, "Current "),
                run(315, 164, 35, "Prior "),
                run(395, 164, 35, "Change "),
                run(100, 174, 40, "Revenues of the regional segments"),
            ],
            first,
        );
        assert_eq!(
            found[0].rows, 5,
            "a first row read as the header's last line"
        );
        assert_eq!(texts(&found[0])[0], first);
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
        assert_eq!(found[0].rule, ethos_parser_core::TABLE_DETECTION_TRACKS_V16);
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
        let found =
            detect_in_columns(1, &runs, &[left, right], &[], false, &mut alloc()).expect("detects");
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
        let found = detect_in_columns(1, &runs, &[labels, values], &[], false, &mut alloc())
            .expect("detects");
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

    /// Two panels of `columns` side by side, each a header and four rows, the left panel written
    /// whole before the right: two flows, each written row by row.
    fn panels(left: [&'static str; 5], right: [&'static str; 5]) -> Vec<TrackRun<'static>> {
        let mut runs = Vec::new();
        for (x, column) in [(100, left), (300, right)] {
            for (k, text) in column.into_iter().enumerate() {
                let y = 100 + 16 * k as i64;
                let figure = ["Rate", "$195.70", "$201.40", "$208.15", "$214.90"][k];
                runs.extend([run(x, y, 40, text), run(x + 70, y, 35, figure)]);
            }
        }
        runs
    }

    /// **Two flows of numbers side by side are two tables** (`-v14`): a schedule set in two panels,
    /// written one panel and then the other, is read as each panel's table — and still as none
    /// where one panel's first column is running text or a column of words beside no numbers.
    #[test]
    fn two_flows_of_numbers_side_by_side_are_two_tables() {
        const AGES: [&str; 5] = ["Age", "65", "66", "67", "68"];
        let found = tables(&panels(AGES, ["Age", "75", "76", "77", "78"]));
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].rows, found[0].columns), (5, 2));
        assert_eq!(texts(&found[1])[1], ["75", "$195.70"]);

        let words = |x: i64| -> Vec<TrackRun<'static>> {
            ["Name", "Avery", "Blake", "Casey", "Drew"]
                .into_iter()
                .zip(["City", "Leeds", "Hull", "York", "Bath"])
                .enumerate()
                .flat_map(|(k, (name, city))| {
                    let y = 100 + 16 * k as i64;
                    [run(x, y, 40, name), run(x + 70, y, 40, city)]
                })
                .collect()
        };
        let mut left_words = words(100);
        left_words.extend(panels(AGES, AGES).into_iter().filter(|r| r.x >= 30000));
        assert!(
            tables(&left_words).is_empty(),
            "words beside no numbers, left"
        );
        let mut right_words: Vec<TrackRun<'static>> = panels(AGES, AGES)
            .into_iter()
            .filter(|r| r.x < 30000)
            .collect();
        right_words.extend(words(300));
        assert!(
            tables(&right_words).is_empty(),
            "words beside no numbers, right"
        );

        let prose = panels(
            ["Age", "rates rise", "with age", "and term", "of cover"],
            AGES,
        );
        assert!(tables(&prose).is_empty(), "running text");
    }

    /// **A contents list set in two panels is no table** (`-v14`): its page numbers, drawn with dot
    /// leaders, are no column of values, and the two panels stay two flows.
    #[test]
    fn a_contents_list_in_two_panels_is_no_table() {
        let mut runs = Vec::new();
        for (x, entries) in [
            (
                100,
                [
                    ("07....", "About us"),
                    ("24....", "Footprint"),
                    ("26....", "Message"),
                    ("30....", "Leadership"),
                    ("34....", "Strategy"),
                ],
            ),
            (
                300,
                [
                    ("72....", "Board"),
                    ("74....", "Engagement"),
                    ("78....", "Investors"),
                    ("82....", "Consumers"),
                    ("88....", "Community"),
                ],
            ),
        ] {
            for (k, (page, title)) in entries.into_iter().enumerate() {
                let y = 100 + 16 * k as i64;
                runs.extend([run(x, y, 30, page), run(x + 70, y, 50, title)]);
            }
        }
        assert!(tables(&runs).is_empty());
    }

    /// **One side's table runs on below where the other's ended** (`-v14`): a note under the left
    /// panel ends the candidate across both, and the right panel's rows below it are still its
    /// table's — each side is read from the last table's end down.
    #[test]
    fn a_side_s_table_runs_on_below_the_other_s() {
        let side = |x: i64, ages: &[&'static str]| -> Vec<TrackRun<'static>> {
            ages.iter()
                .zip([
                    "Rate", "$195.70", "$201.40", "$208.15", "$214.90", "$221.65", "$228.40",
                ])
                .enumerate()
                .flat_map(|(k, (&age, rate))| {
                    let y = 100 + 16 * k as i64;
                    [run(x, y, 40, age), run(x + 70, y, 35, rate)]
                })
                .collect()
        };
        let mut runs = side(100, &["Age", "65", "66", "67", "68"]);
        runs.push(run(100, 180, 130, "Source: rate filing"));
        runs.extend(side(300, &["Age", "75", "76", "77", "78", "79", "80"]));
        let found = tables(&runs);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].rows, found[1].rows), (5, 7));
    }

    /// **Two flows are read apart only where each could be a table** (`-v14`): two rows of two
    /// panels are too few to tell two tables from two flows of anything; a list's bullets beside a
    /// table set level with it stay the list's, the candidate written row by row; and a table beside
    /// a single column of numbers leaves that column no table of its own to be.
    #[test]
    fn two_flows_are_read_apart_only_where_each_could_be_a_table() {
        let mut runs = Vec::new();
        for (x, plan) in [(100, "Basic"), (300, "Plus")] {
            runs.extend([
                run(x, 100, 40, "Plan"),
                run(x + 70, 100, 25, "2023"),
                run(x + 130, 100, 25, "2024"),
            ]);
            runs.extend([
                run(x, 116, 40, plan),
                run(x + 70, 116, 20, "7.5"),
                run(x + 130, 116, 20, "8.0"),
            ]);
        }
        assert!(tables(&runs).is_empty(), "two rows");

        let rows = [
            ("1,200", "North", "7.5"),
            ("950", "South", "18.25"),
            ("2,400", "East", "4.0"),
            ("610", "West", "12.0"),
            ("1,075", "Central", "3.5"),
        ];
        let bullets: Vec<TrackRun<'static>> = rows
            .iter()
            .enumerate()
            .flat_map(|(k, &(count, region, figure))| {
                let y = 100 + 16 * k as i64;
                [
                    run(100, y, 5, "•"),
                    run(130, y, 30, count),
                    run(250, y, 40, region),
                    run(320, y, 25, figure),
                ]
            })
            .collect();
        assert!(tables(&bullets).is_empty(), "a list's bullets");

        let mut beside: Vec<TrackRun<'static>> = rows
            .iter()
            .enumerate()
            .flat_map(|(k, &(_, region, figure))| {
                let y = 100 + 16 * k as i64;
                [run(100, y, 40, region), run(170, y, 25, figure)]
            })
            .collect();
        beside.extend(
            ["12", "15", "18", "21", "24"]
                .into_iter()
                .enumerate()
                .map(|(k, n)| run(300, 100 + 16 * k as i64, 15, n)),
        );
        assert!(tables(&beside).is_empty(), "a single column of numbers");
    }

    /// **Each side is read from the last table's end** (`-v14`): a panel's own first line, set a
    /// little off the other panel's baseline, opens no table across the page, and is still the
    /// first row of its panel's — the right panel's, set closer above `Age` and `Rate` than its
    /// rows are to each other, as its header's first line (`-v15`, decision #96).
    #[test]
    fn each_side_is_read_from_the_last_table_s_end() {
        let side = |x: i64, top: i64| -> Vec<TrackRun<'static>> {
            let mut runs = vec![run(x, top, 40, "Plan"), run(x + 70, top, 35, "Monthly")];
            runs.extend(
                ["Age", "65", "66", "67", "68"]
                    .into_iter()
                    .zip(["Rate", "$195.70", "$201.40", "$208.15", "$214.90"])
                    .enumerate()
                    .flat_map(|(k, (age, rate))| {
                        let y = 100 + 16 * k as i64;
                        [run(x, y, 40, age), run(x + 70, y, 35, rate)]
                    }),
            );
            runs
        };
        let mut runs = side(100, 84);
        runs.extend(side(300, 86));
        let found = tables(&runs);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].rows, found[1].rows), (6, 5));
        assert_eq!(texts(&found[0])[0], ["Plan", "Monthly"]);
        assert_eq!(texts(&found[1])[0], ["PlanAge", "MonthlyRate"]);
    }

    /// Two schedules side by side, `Age` and `Rate` over four rows each, written row by row across
    /// both — or, `by_column`, down each column in turn.
    fn schedules(by_column: bool) -> Vec<TrackRun<'static>> {
        let ages = [
            ["Age", "65", "66", "67", "68"],
            ["Age", "75", "76", "77", "78"],
        ];
        let rates = ["Rate", "$195.70", "$201.40", "$208.15", "$214.90"];
        let cell = |side: usize, column: usize, k: usize| {
            let (x, y) = (
                100 + 200 * side as i64 + 70 * column as i64,
                100 + 16 * k as i64,
            );
            if column == 0 {
                run(x, y, 40, ages[side][k])
            } else {
                run(x, y, 35, rates[k])
            }
        };
        let mut runs = Vec::new();
        if by_column {
            for side in 0..2 {
                for column in 0..2 {
                    runs.extend((0..5).map(|k| cell(side, column, k)));
                }
            }
        } else {
            for k in 0..5 {
                for side in 0..2 {
                    runs.extend((0..2).map(|column| cell(side, column, k)));
                }
            }
        }
        runs
    }

    /// A rule `y` points down the page, from `x0` to `x1` points.
    fn rule(y: i64, x0: i64, x1: i64) -> Rule {
        Rule {
            y: y * 100,
            x0: x0 * 100,
            x1: x1 * 100,
        }
    }

    /// A box round each of [`schedules`]: its top over `Age`, its bottom under the last row — the
    /// left's ending in the gutter, the right's starting in it.
    fn boxes() -> Vec<Rule> {
        vec![
            rule(88, 95, 215),
            rule(168, 95, 215),
            rule(88, 290, 415),
            rule(168, 290, 415),
        ]
    }

    fn drawn(runs: &[TrackRun<'_>], rules: &[Rule]) -> Vec<DetectedTable> {
        detect(1, runs, rules, false, &mut alloc()).expect("detects")
    }

    /// **Two tables drawn side by side are two tables, though they stood as one** (`-v16`, decision
    /// #115): two schedules written row by row across both stand as one table of four columns; each
    /// drawn in a box of its own, or under a rule of its own with a rule under its last row, each is
    /// read on its own.
    #[test]
    fn two_tables_drawn_side_by_side_are_two_tables() {
        let runs = schedules(false);
        let one = drawn(&runs, &[]);
        assert_eq!(one.len(), 1, "the premise: undrawn, one table");
        assert_eq!((one[0].rows, one[0].columns), (5, 4));
        let two = drawn(&runs, &boxes());
        assert_eq!(two.len(), 2);
        assert_eq!((two[0].rows, two[0].columns), (5, 2));
        assert_eq!(texts(&two[0])[1], ["65", "$195.70"]);
        assert_eq!(texts(&two[1])[1], ["75", "$195.70"]);
        let bars = [
            rule(104, 95, 215),
            rule(168, 95, 215),
            rule(104, 290, 415),
            rule(168, 290, 415),
        ];
        assert_eq!(
            drawn(&runs, &bars).len(),
            2,
            "a header rule and a closing rule"
        );
        let pieces: Vec<Rule> = boxes()
            .iter()
            .flat_map(|r| {
                let middle = (r.x0 + r.x1) / 2;
                [Rule { x1: middle, ..*r }, Rule { x0: middle, ..*r }]
            })
            .collect();
        assert_eq!(drawn(&runs, &pieces).len(), 2, "rules drawn cell by cell");
    }

    /// **Ink across the gutter keeps them one** (`-v16`): a rule under both headers at once is one
    /// table's, whatever boxes stand beside it; a bar over both titles, more than an em above the
    /// first row, joins neither.
    #[test]
    fn ink_across_the_gutter_keeps_two_boxes_one_table() {
        let runs = schedules(false);
        let mut across = boxes();
        across.push(rule(104, 95, 415));
        assert_eq!(drawn(&runs, &across).len(), 1, "a rule under both headers");
        let mut titled = boxes();
        titled.push(rule(80, 95, 415));
        assert_eq!(drawn(&runs, &titled).len(), 2, "a bar over both titles");
        // Nor is it either side's top: a side whose box has none under it is closed alone.
        for top in [0, 2] {
            let mut ink = titled.clone();
            ink.remove(top);
            assert_eq!(drawn(&runs, &ink).len(), 1, "a bar over both, no top");
        }
    }

    /// **A side's ink must close its rows and stand above them** (`-v16`): a box round each header
    /// alone is a column group's heading drawn in a box of its own, a rule under each last row alone
    /// draws no side's top, and one side boxed beside a header box is no two tables either.
    #[test]
    fn a_side_s_ink_must_close_its_rows_and_stand_above_them() {
        let runs = schedules(false);
        let heads = [
            rule(88, 95, 215),
            rule(104, 95, 215),
            rule(88, 290, 415),
            rule(104, 290, 415),
        ];
        assert_eq!(drawn(&runs, &heads).len(), 1, "boxes round the headers");
        let under = [rule(168, 95, 215), rule(168, 290, 415)];
        assert_eq!(drawn(&runs, &under).len(), 1, "rules under the last rows");
        for half in [
            [boxes()[0], boxes()[1], heads[2], heads[3]],
            [heads[0], heads[1], boxes()[2], boxes()[3]],
        ] {
            assert_eq!(drawn(&runs, &half).len(), 1, "one side boxed");
        }
        // And a side's ink spans both its columns, from the first to the gutter or from the gutter
        // to the last: a box round one column is that column's.
        for (side, x0, x1, why) in [
            (0, 95, 145, "the left's labels"),
            (0, 165, 215, "the left's rates"),
            (2, 290, 345, "the right's labels"),
            (2, 365, 415, "the right's rates"),
        ] {
            let mut ink = boxes();
            ink.splice(side..side + 2, [rule(88, x0, x1), rule(168, x0, x1)]);
            assert_eq!(drawn(&runs, &ink).len(), 1, "a box round {why} alone");
        }
    }

    /// **A side needs no order of its own inside the rows the two stood as** (`-v16`): two boxed
    /// schedules written down each column in turn stand as one grid of numbers, by the grid's
    /// waiver; each side, of two columns, has none, and is read as its table all the same.
    #[test]
    fn a_side_needs_no_row_order_inside_the_rows_it_stood_with() {
        let runs = schedules(true);
        assert_eq!(drawn(&runs, &[]).len(), 1, "the premise: one grid");
        let two = drawn(&runs, &boxes());
        assert_eq!(two.len(), 2);
        assert_eq!((two[0].rows, two[1].rows), (5, 5));
    }

    /// **Outside them, a side is asked its order as ever** (`-v16`): a line over the left schedule,
    /// written after both, opens no table with its rows; and a row under it, below where the two
    /// ended, is no row of its table. A row the right box holds under the rest, which the two read
    /// as a wrap of their last row, is inside them: the right schedule's last row.
    #[test]
    fn a_side_is_read_inside_the_lines_the_two_stood_on() {
        let mut runs = schedules(true);
        runs.extend([run(100, 64, 40, "Plan"), run(170, 64, 35, "Basic")]);
        let two = drawn(&runs, &boxes());
        assert_eq!(two.len(), 2);
        assert_eq!(texts(&two[0])[0], ["Age", "Rate"]);

        let mut runs = schedules(false);
        runs.extend([
            run(100, 180, 40, "69"),
            run(170, 180, 35, "$221.65"),
            run(330, 180, 50, "see note"),
        ]);
        let two = drawn(&runs, &boxes());
        assert_eq!(two.len(), 2);
        assert_eq!((two[0].rows, two[1].rows), (5, 5));

        let mut runs = schedules(true);
        runs.extend([run(300, 180, 40, "79"), run(370, 180, 35, "$221.65")]);
        let mut ink = boxes();
        ink[3] = rule(184, 290, 415);
        let two = drawn(&runs, &ink);
        assert_eq!(two.len(), 2, "the right schedule's row under the rest");
        assert_eq!((two[0].rows, two[1].rows), (5, 6));
        assert_eq!(texts(&two[1])[5], ["79", "$221.65"]);
    }

    /// Two tables stacked: three rows of a label and two figures under `2024 | 2023`, then
    /// `second`'s cells `gap` points below the last row, over three rows more.
    fn stacked(
        gap: i64,
        figures: [&'static str; 2],
        second: &[&'static str],
    ) -> Vec<TrackRun<'static>> {
        let mut runs = vec![run(250, 100, 25, "2024"), run(330, 100, 25, "2023")];
        for (k, label) in ["Sales", "Costs", "Margin"].into_iter().enumerate() {
            let y = 116 + 16 * k as i64;
            runs.extend([
                run(100, y, 40, label),
                run(250, y, 25, figures[0]),
                run(330, y, 25, figures[1]),
            ]);
        }
        let top = 148 + gap;
        runs.extend(
            second
                .iter()
                .zip([250, 330])
                .map(|(&text, x)| run(x, top, 25, text)),
        );
        for (k, label) in ["Sales", "Costs", "Margin"].into_iter().enumerate() {
            let y = top + 16 + 16 * k as i64;
            runs.extend([
                run(100, y, 40, label),
                run(250, y, 25, "8.1"),
                run(330, y, 25, "7.7"),
            ]);
        }
        runs
    }

    /// **A line of years below rows of data heads the next table** (`-v16`, decision #115): set a
    /// row and a half's pitch or more below them, `2022 | 2021` ends the table and heads the rows
    /// under it — and not at the rows' own pitch, nor a line of a year and a word, nor one year
    /// alone, nor under rows that hold no figure, the years over them being no figures of a row.
    #[test]
    fn a_line_of_years_below_rows_of_data_heads_the_next_table() {
        let figures = ["7.5", "6.9"];
        let two = tables(&stacked(24, figures, &["2022", "2021"]));
        assert_eq!(two.len(), 2);
        assert_eq!((two[0].rows, two[1].rows), (4, 4));
        assert_eq!(texts(&two[1])[0], ["", "2022", "2021"]);
        for (gap, figures, second, why) in [
            (16, figures, &["2022", "2021"][..], "at the rows' pitch"),
            (24, figures, &["2022", "Total"][..], "a year and a word"),
            (24, figures, &["2022"][..], "one year"),
            (
                24,
                ["high", "low"],
                &["2022", "2021"][..],
                "rows of no figure",
            ),
        ] {
            assert_eq!(tables(&stacked(gap, figures, second)).len(), 1, "{why}");
        }
        // A label over the labels makes the years' line the table's first row: its years are the
        // header's, no row's figures.
        let mut headed = stacked(24, ["high", "low"], &["2022", "2021"]);
        headed.insert(0, run(100, 100, 40, "Region"));
        assert_eq!(tables(&headed).len(), 1, "years in the first row");
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

    /// **A sign set alone after a figure is the next amount's** (`-v16`, decision #116): a line
    /// that sets its first sign before its figure sets each sign so, and a `$` within an em after
    /// `(692)` or `3,210`, in columns set close, opens the next column's cell — the figure took it,
    /// and the next amount came without its sign.
    #[test]
    fn a_sign_set_alone_after_a_figure_opens_the_next_amount() {
        let runs = vec![
            run(100, 100, 50, "(in millions)"),
            run(255, 100, 25, "2025"),
            run(305, 100, 25, "2024"),
            run(100, 116, 40, "Net loss"),
            run(200, 116, 5, "$"),
            run(250, 116, 30, "(692)"),
            run(285, 116, 5, "$"),
            run(305, 116, 25, "1,105"),
            run(100, 132, 40, "Revenue"),
            run(200, 132, 5, "$"),
            run(255, 132, 25, "3,210"),
            run(285, 132, 5, "$"),
            run(305, 132, 25, "2,950"),
            run(100, 148, 40, "Costs"),
            run(255, 148, 25, "1,840"),
            run(305, 148, 25, "1,704"),
        ];
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[1], ["Net loss", "$(692)", "$1,105"]);
        assert_eq!(texts(&found[0])[2], ["Revenue", "$3,210", "$2,950"]);
    }

    /// **And after a percentage; and only where the signs come first** (`-v16`, decision #116): a
    /// `$` set after `10 %` opens the next amount's cell too; a line that sets its signs after its
    /// figures — `692 €` — keeps each with its figure, and a `$` after a word, `US $`, is one sign
    /// with it.
    #[test]
    fn a_sign_opens_a_cell_only_after_a_figure_in_a_line_that_sets_signs_first() {
        let shares = vec![
            run(100, 100, 40, "Segment"),
            run(220, 100, 25, "2025"),
            run(262, 100, 15, "Share"),
            run(305, 100, 25, "2024"),
            run(100, 116, 40, "Sales"),
            run(190, 116, 5, "$"),
            run(220, 116, 25, "31,370"),
            run(262, 116, 10, "10"),
            run(273, 116, 5, "%"),
            run(283, 116, 5, "$"),
            run(305, 116, 25, "29,000"),
            run(100, 132, 40, "Costs"),
            run(190, 132, 5, "$"),
            run(220, 132, 25, "21,000"),
            run(267, 132, 5, "9"),
            run(273, 132, 5, "%"),
            run(283, 132, 5, "$"),
            run(305, 132, 25, "19,800"),
            run(100, 148, 40, "Margin"),
            run(220, 148, 25, "10,370"),
            run(262, 148, 10, "33"),
            run(273, 148, 5, "%"),
            run(310, 148, 20, "9,200"),
        ];
        let found = tables(&shares);
        assert_eq!((found[0].rows, found[0].columns), (4, 4));
        assert_eq!(texts(&found[0])[1], ["Sales", "$31,370", "10%", "$29,000"]);

        let after = |y: i64, label: &'static str, a: &'static str, b: &'static str| {
            [
                run(100, y, 40, label),
                run(250, y, 20, a),
                run(272, y, 5, "€"),
                run(305, y, 20, b),
                run(327, y, 5, "€"),
            ]
        };
        let mut euros = vec![
            run(100, 100, 40, "Posten"),
            run(250, 100, 25, "2025"),
            run(305, 100, 25, "2024"),
        ];
        euros.extend(after(116, "Umsatz", "692", "1.105"));
        euros.extend(after(132, "Kosten", "410", "388"));
        euros.extend(after(148, "Saldo", "282", "717"));
        let found = tables(&euros);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[1], ["Umsatz", "692€", "1.105€"]);

        let dollars = |y: i64, label: &'static str, a: &'static str, b: &'static str| {
            [
                run(100, y, 40, label),
                run(245, y, 10, "US"),
                run(257, y, 5, "$"),
                run(265, y, 20, a),
                run(310, y, 10, "US"),
                run(322, y, 5, "$"),
                run(330, y, 20, b),
            ]
        };
        let mut us = vec![
            run(100, 100, 40, "Item"),
            run(255, 100, 25, "2025"),
            run(325, 100, 25, "2024"),
        ];
        us.extend(dollars(116, "Sales", "4,210", "3,980"));
        us.extend(dollars(132, "Costs", "2,870", "2,640"));
        us.extend(dollars(148, "Net", "1,340", "1,340"));
        let found = tables(&us);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[1], ["Sales", "US$4,210", "US$3,980"]);
    }

    /// **A run that opens with a sign after a figure is the next amount's** (`-v16`, decision #119):
    /// a statement sets `$134,803` as one run within an em after `95,685`, in columns set close, and
    /// the two amounts were one cell, the table a column short. A figure drawn as two runs, `30,6`
    /// and `74`, is still one amount: only a sign opens one.
    #[test]
    fn a_run_that_opens_with_a_sign_after_a_figure_opens_the_next_amount() {
        let row = |y: i64, label: &'static str, a: &'static str, b: &'static str| {
            [
                run(100, y, 50, label),
                run(200, y, 5, "$"),
                run(209, y, 25, a),
                run(240, y, 35, b),
            ]
        };
        let mut runs: Vec<TrackRun<'static>> = row(100, "Fixed rate", "95,685", "$134,803").into();
        runs.extend(row(116, "Variable", "183,125", "$71,693"));
        runs.extend([
            run(100, 132, 50, "Floating"),
            run(200, 132, 5, "$"),
            run(209, 132, 15, "30,6"),
            run(224, 132, 10, "74"),
            run(240, 132, 35, "$22,000"),
        ]);
        runs.extend(row(148, "Total", "309,484", "$228,496"));
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[0], ["Fixed rate", "$95,685", "$134,803"]);
        assert_eq!(texts(&found[0])[2], ["Floating", "$30,674", "$22,000"]);
    }

    /// **A run that opens with a sign after a word stays with it** (`-v16`, decision #119): `US`
    /// then `$4,210` within an em is one amount, as `US $` is — only a figure ends one.
    #[test]
    fn a_run_that_opens_with_a_sign_after_a_word_stays_with_it() {
        let row = |y: i64, label: &'static str, a: &'static str, b: &'static str| {
            [
                run(100, y, 40, label),
                run(245, y, 10, "US"),
                run(257, y, 25, a),
                run(310, y, 10, "US"),
                run(322, y, 25, b),
            ]
        };
        let mut runs = vec![
            run(100, 100, 40, "Item"),
            run(255, 100, 25, "2025"),
            run(325, 100, 25, "2024"),
        ];
        runs.extend(row(116, "Sales", "$4,210", "$3,980"));
        runs.extend(row(132, "Costs", "$2,870", "$2,640"));
        runs.extend(row(148, "Net", "$1,340", "$1,340"));
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[1], ["Sales", "US$4,210", "US$3,980"]);
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

    /// A 6.5pt mark at `x` points, `w` points wide, its baseline `raised` centipoints above `y`
    /// points: a footnote mark's size, or a chart label's, beside [`run`]'s 10pt.
    fn mark(x: i64, y: i64, raised: i64, w: i64, text: &'static str) -> TrackRun<'static> {
        let (x, y, w) = (x * 100, y * 100 - raised, w * 100);
        TrackRun {
            x,
            y,
            em: Some(650),
            rect: Some(QuantRect {
                x0: x,
                y0: y - 520,
                x1: x + w,
                y1: y + 130,
            }),
            text,
            claimed: false,
        }
    }

    /// Three columns of a filing's segment table, `raised` centipoints the height of the `1` set
    /// against `Comp sales (ex. fuel)`.
    fn segments(raised: i64) -> Vec<TrackRun<'static>> {
        vec![
            run(100, 100, 40, "Segment"),
            run(250, 100, 30, "2024"),
            run(330, 100, 30, "2023"),
            run(100, 116, 40, "Net sales"),
            run(250, 116, 30, "$21.9"),
            run(330, 116, 30, "$21.4"),
            run(100, 132, 90, "Comp sales (ex. fuel)"),
            mark(190, 132, raised, 3, "1"),
            run(255, 132, 25, "3.1%"),
            run(330, 132, 30, "12.2%"),
            run(100, 148, 50, "Transactions"),
            run(255, 148, 25, "3.6%"),
            run(335, 148, 25, "6.7%"),
        ]
    }

    /// **A mark set against its label joins it, raised over half an em** (`-v16`, decision #119): a
    /// filing raises the `1` on `Comp sales¹` a little over half an em. On a line of its own it sat
    /// off every track, past the labels above it, and ended the table above its row — and the rows
    /// either side of it were each too few to stand.
    #[test]
    fn a_mark_set_against_its_label_joins_it_though_raised_over_half_an_em() {
        let found = tables(&segments(580));
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(
            texts(&found[0])[2],
            ["Comp sales (ex. fuel)1", "3.1%", "12.2%"]
        );
    }

    /// **A mark raised three fifths of an em or more is no superscript** (`-v16`, decision #119):
    /// the band set against the text ends there, and `North` keeps no mark set that high over it.
    #[test]
    fn a_mark_raised_three_fifths_of_an_em_is_no_superscript() {
        let mut runs = numbers();
        runs.insert(4, mark(140, 116, 620, 3, "1"));
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 3));
        assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0"]);
    }

    /// **A chart's labels raised against no text stay lines of their own** (`-v16`, decision #119):
    /// a value label set smaller, over half an em above a bar's name and value, starts where no run
    /// ends — eight points past the value, and narrower than a quarter em itself. Joined, each line
    /// came out a row and the four a table over the chart, which took the chart's own runs.
    #[test]
    fn a_chart_s_labels_raised_against_no_text_stay_lines_of_their_own() {
        let bars = [
            ("Indonesia", "39"),
            ("Philippines", "37"),
            ("Malaysia", "36"),
            ("South Korea", "30"),
        ];
        let chart: Vec<TrackRun<'static>> = bars
            .iter()
            .enumerate()
            .flat_map(|(k, &(name, value))| {
                let y = 100 + 16 * k as i64;
                [
                    run(100, y, 45, name),
                    run(170, y, 10, value),
                    mark(188, y, 550, 2, "5"),
                ]
            })
            .collect();
        assert!(tables(&chart).is_empty());
    }

    /// **And a line set against text in part is no superscript** (`-v16`, decision #119): each of
    /// its runs must start where text ends, and a label far along the line starts where none does.
    #[test]
    fn a_line_set_against_text_only_in_part_is_no_superscript() {
        let bars = [
            ("Indonesia", "39"),
            ("Philippines", "37"),
            ("Malaysia", "36"),
            ("South Korea", "30"),
        ];
        let chart: Vec<TrackRun<'static>> = bars
            .iter()
            .enumerate()
            .flat_map(|(k, &(name, value))| {
                let y = 100 + 16 * k as i64;
                [
                    run(100, y, 45, name),
                    run(170, y, 10, value),
                    mark(180, y, 550, 2, "5"),
                    mark(260, y, 550, 2, "8"),
                ]
            })
            .collect();
        assert!(tables(&chart).is_empty());
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

    /// Two columns: list labels `1` to `4` beside `items`.
    fn numbered(items: [&'static str; 4]) -> Vec<TrackRun<'static>> {
        ["1", "2", "3", "4"]
            .iter()
            .zip(items)
            .enumerate()
            .flat_map(|(k, (&label, item))| {
                let y = 100 + 16 * k as i64;
                [run(100, y, 8, label), run(130, y, 120, item)]
            })
            .collect()
    }

    /// **Small numbers beside a column of numbers key a table** (`-v16`, decision #119): ages beside
    /// their premiums are a rate table's rows, where a list's labels stand beside words.
    #[test]
    fn small_numbers_beside_a_column_of_numbers_key_a_table() {
        let runs: Vec<TrackRun<'static>> = [
            ("65", "$195.70"),
            ("66", "$201.40"),
            ("67", "$210.05"),
            ("68", "$217.40"),
        ]
        .iter()
        .enumerate()
        .flat_map(|(k, &(age, premium))| {
            let y = 100 + 16 * k as i64;
            [run(100, y, 10, age), run(160, y, 35, premium)]
        })
        .collect();
        let found = tables(&runs);
        assert_eq!((found[0].rows, found[0].columns), (4, 2));
        assert_eq!(texts(&found[0])[0], ["65", "$195.70"]);
        assert_eq!(
            tables(&numbered(["2,500", "1,000", "750", "600"])).len(),
            1,
            "numbers beside numbers"
        );
    }

    /// **A numbered list of words is a list, though an item is a figure** (`-v16`, decision #119):
    /// only a column of numbers, every item one, makes the labels a table's keys.
    #[test]
    fn a_numbered_list_of_words_is_a_list_though_an_item_is_a_figure() {
        assert!(tables(&numbered([
            "Read the notice",
            "1,000",
            "Sign the form",
            "Mail it back"
        ]))
        .is_empty());
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

    /// `-v16` (decision #117): **a figure under a figure opens a row** — a line with no first cell,
    /// half a row's pitch under North, that sets a number where North holds one is a row of its
    /// own, its first cell empty and its word beside the number its own. Only years, a number
    /// alone on its line, or a number under a word, and it is still North's wrapped line.
    #[test]
    fn a_figure_under_a_figure_opens_a_row() {
        let with = |north: &'static str, extra: &[TrackRun<'static>]| {
            let mut runs = numbers();
            runs[4] = run(250, 116, 20, north);
            for (k, r) in extra.iter().enumerate() {
                runs.insert(6 + k, *r);
            }
            tables(&runs)
        };
        let found = with(
            "7.5",
            &[run(250, 124, 20, "8.0"), run(330, 124, 40, "est.")],
        );
        assert_eq!((found[0].rows, found[0].columns), (5, 3));
        assert_eq!(texts(&found[0])[1], ["North", "7.5", "112.0"]);
        assert_eq!(texts(&found[0])[2], ["", "8.0", "est."]);
        for (north, extra) in [
            (
                "7.5",
                &[run(250, 124, 30, "2025"), run(340, 124, 30, "2026")][..],
            ),
            ("7.5", &[run(250, 124, 20, "8.0")]),
            (
                "nil",
                &[run(250, 124, 20, "8.0"), run(330, 124, 40, "note")],
            ),
        ] {
            let found = with(north, extra);
            assert_eq!((found[0].rows, found[0].columns), (4, 3), "{extra:?}");
        }
    }

    /// A statement stacks its periods' blocks, the next one's year set alone over it and its header
    /// below that: the year is no figure under the block above, which ends where the next block's
    /// header is set (Apple's 10-K p50, three segment tables read as one before the year was none).
    #[test]
    fn a_year_set_alone_over_the_next_block_opens_no_row() {
        let mut runs = numbers();
        runs.push(run(245, 175, 30, "2025"));
        runs.extend([run(240, 195, 30, "Q1"), run(340, 195, 30, "Q2")]);
        for (k, (label, a, b)) in [
            ("North", "1.0", "2.0"),
            ("South", "3.0", "4.0"),
            ("East", "5.0", "6.0"),
            ("West", "7.0", "8.0"),
        ]
        .into_iter()
        .enumerate()
        {
            let y = 211 + 16 * k as i64;
            runs.extend([
                run(100, y, 40, label),
                run(250, y, 20, a),
                run(350, y, 20, b),
            ]);
        }
        let shapes: Vec<(u32, u32)> = tables(&runs).iter().map(|t| (t.rows, t.columns)).collect();
        assert_eq!(shapes, [(4, 3), (5, 3)]);
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

    /// Three columns of prose set on one baseline grid, written row by row across them — the order
    /// a recognizer reads a scanned newsletter in, line by line down the whole page.
    fn newsletter() -> Vec<TrackRun<'static>> {
        let lines = [
            [
                "There are other advantages as",
                "you can build that into your",
                "for not staying on top of it",
            ],
            [
                "well, says Marcus R. Chatterton,",
                "requests. So the collection",
                "and what the rule now asks",
            ],
            [
                "chair of the litigation section",
                "becomes regularized, he said",
                "of every party to a dispute",
            ],
            [
                "of the bar and its committee on",
                "and it saves a trial a full",
                "who must authenticate the",
            ],
        ];
        lines
            .iter()
            .enumerate()
            .flat_map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .map(move |(c, &text)| run(60 + 170 * c as i64, 100 + 12 * r as i64, 150, text))
            })
            .collect()
    }

    /// **On a page of recognized text, row order is no evidence of a table** (decision #125): the
    /// three columns a drawn page writes row by row stand as a table, as they always have, and the
    /// same columns read by a recognizer, whose order is row by row whatever the page holds, do not.
    #[test]
    fn a_recognized_page_does_not_read_its_columns_of_prose_as_a_table() {
        let runs = newsletter();
        assert_eq!(
            tables(&runs).len(),
            1,
            "a drawn page's row order still says table"
        );
        assert!(detect(1, &runs, &[], true, &mut alloc())
            .expect("detects")
            .is_empty());
    }

    /// **And a recognized grid with no running text stands without a column of numbers**
    /// (decision #125): a scanned list of hospitals, their counties and the services each marks.
    #[test]
    fn a_recognized_list_of_names_and_marks_is_a_table() {
        let rows = [
            ["Alameda Hospital", "Alameda", "•", "•"],
            ["Alta Bates Medical", "Alameda", "•", ""],
            ["Eden Medical Center", "Alameda", "", "•"],
            ["Enloe Medical Center", "Butte", "•", "•"],
        ];
        let runs: Vec<TrackRun<'static>> = rows
            .iter()
            .enumerate()
            .flat_map(|(r, row)| {
                row.iter()
                    .enumerate()
                    .filter(|(_, text)| !text.is_empty())
                    .map(move |(c, &text)| {
                        run(
                            60 + 120 * c as i64,
                            100 + 14 * r as i64,
                            90.min(8 * text.len() as i64),
                            text,
                        )
                    })
            })
            .collect();
        let found = detect(1, &runs, &[], true, &mut alloc()).expect("detects");
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].rows, found[0].columns), (4, 4));
    }
}
