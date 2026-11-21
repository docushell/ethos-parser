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

//! **Text a page draws over itself after its own** (decision #157): a form filler's typed values,
//! read where they sit rather than where the content stream put them.
//!
//! A tool that fills a form in a PDF appends its values to the page's content stream, each in a
//! font of its own. Reading order keeps content-stream order inside a block of text
//! ([`crate::reading_order`] step 5 and its leaves), so every value came out after the captions
//! of its block, or at the foot of the page, and a reader could not tell which value answered
//! which caption.
//!
//! # Which text is an overlay
//!
//! The **longest last stretch** of the page's runs, in content-stream order, whose fonts no run
//! before it uses — text a second hand drew last, in its own fonts. It is the overlay where, among
//! its runs that show any text, at least [`MIN_RUNS`] lie inside
//! the vertical span of the page's own baselines, give or take a row ([`ROW`]), and they are at
//! least half of them, none holds more than [`MAX_WORDS`] words, and they bring a font of their
//! own for at least one run in [`FONT_SHARE`]. A filler's values sit among the captions, are short,
//! and are each drawn in a font of its own; a footer, a column, a chart's labels or a paragraph
//! drawn last shares a font or two among many runs. Measured on all 4,317 pages of ParseBench-2:
//! without the font share the stretch qualified on 265 pages that are no form (a fifth of the
//! chart pages), with it on none, and on 315 of the 320 form pages it qualified on.
//!
//! # Where an overlay's run is read
//!
//! A run in an accepted table stays where its cell puts it, and so does a run **drawn over the
//! page's own text** — its box, seven tenths of its type size above its baseline ([`ASCENT`]),
//! covers half the narrower's width and half the shorter's height of a run of the page's own that
//! holds a letter or a
//! digit and is neither one glyph (a box or a bullet) nor ends in a leader (`___`, `...`) — which is
//! a watermark, a stamp or a footnote mark, not a value. Every other run is read:
//!
//! 1. **On its row**, where a run of the page's own has its baseline within [`ROW`] of the run's:
//!    right after the nearest such run that starts left of it — ending before it, or reaching
//!    under it as a caption with its leader does (`Name: ____`, one run) or a box glyph a mark is
//!    drawn in — up to [`ROW_REACH`] away, where that nearest run is a caption: it ends in a
//!    colon, a question mark or a leader, is one glyph other than a bullet (a box, a currency
//!    sign, a form's `×` or `=`; a bullet only for a mark), or ends a phrase of at most
//!    [`CAPTION_WORDS`] words (a leader no word) that ends no sentence, the phrase its line's runs
//!    set less than their size apart — not a line of prose or a bullet, which a pull quote or a
//!    figure's labels drawn last sit beside (second and third reviews). Where it is none, the run
//!    stays where it was. A leader drawn as a run of its own, or a long
//!    caption running on under a value, anchors nothing it runs under. A mark — one glyph that is
//!    no letter or digit, a check, a dingbat — is read instead beside the nearest such run on
//!    either side within [`CHECK_REACH`]: before one to its right, the option it ticks, after one
//!    to its left; a one-letter or one-digit value reaches as any value does. A run whose row
//!    holds the page's text but no caption stays where it was.
//! 2. **Under a line**, where its row holds none: after the last run of the nearest line of the
//!    page's own above it, no more than [`LINE_REACH`] above and overlapping it across — a value
//!    set under its caption.
//!
//! An anchor is never in a table, nor is the end of the line it is followed to, and on a page the
//! column cut divided it is in the run's own
//! region. A run set more than [`SIZE_RATIO`] times larger than its anchor — a watermark's
//! letter, drawn where the page's text is split around it — stays where it was, and so does a run
//! the page marks `/Artifact`, a watermark turned on the page, which no box measured. Where as many of the
//! overlay's runs are drawn over the page's own text as are read beside it, the overlay is a
//! watermark or a stamp and nothing moves. Runs read beside one run are read row by row, left to
//! right. A page with no overlay, or an overlay of more than [`MAX_OVERLAY`] runs, is untouched,
//! byte for byte.
//!
//! **Known limits**: a sidebar drawn after the page's text, in fonts of its own, short-lined and
//! closer than the column cut's gutter, is read as values beside the lines it sits by; a stamp or
//! a chart's labels set in the gaps between lines, not over them, are read among those lines; a
//! value set over its caption (a signature line) is read with the line above it; a page that draws
//! its own words in fragments, each in a font of its own, can look like a filler's; and
//! a form whose first run (a title) has a font no later run uses is not found, since the longest
//! stretch then starts with its captions; a filler that draws every value in one font is not
//! found either; a value set under its caption, on a row another caption ends on, is read after
//! that one; and a wrapped value's second line is read after the line above it, another field's.

use std::collections::BTreeMap;

use crate::tables::QuantRect;

/// One run, as this rule reads it, in content-stream order: its baseline origin, advance and size
/// in centipoints, top-left system — the size its measured box's height, or its type size where
/// none was measured — the font resource it was drawn in, its text, and whether the page marks it
/// `/Artifact`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Drawn<'a> {
    pub x: i64,
    pub y: i64,
    pub advance: Option<i64>,
    pub size: i64,
    pub font: &'a str,
    pub text: &'a str,
    pub artifact: bool,
}

impl Drawn<'_> {
    fn right(&self) -> i64 {
        self.x + self.advance.unwrap_or(0).max(0)
    }

    fn words(&self) -> usize {
        self.text.split_whitespace().count()
    }

    fn glyphs(&self) -> usize {
        self.text.trim().chars().count()
    }

    /// How far this run's glyphs rise above its baseline: [`ASCENT`] of its type size.
    fn rise(&self) -> i64 {
        self.size.max(0) * ASCENT.0 / ASCENT.1
    }

    /// Whether this run's box — its advance across, its glyphs' rise above the baseline — covers
    /// `other`'s: overlapping it across at least half the narrower's width, and up and down at
    /// least half the shorter's rise. A value typed under a caption's foot grazes it; a watermark
    /// or a stamp set over a line covers it.
    fn covers(&self, other: &Drawn<'_>) -> bool {
        let across = self.right().min(other.right()) - self.x.max(other.x);
        let up = self.y.min(other.y) - (self.y - self.rise()).max(other.y - other.rise());
        let narrower = (self.right() - self.x).min(other.right() - other.x);
        let shorter = self.rise().min(other.rise());
        narrower > 0 && shorter > 0 && 2 * across >= narrower && 2 * up >= shorter
    }

    /// Whether this run ends in a leader: `___`, `...` or `…`.
    fn ends_in_leader(&self) -> bool {
        let t = self.text.trim();
        t.ends_with("___") || t.ends_with("...") || t.ends_with('…')
    }

    /// Its words that hold a letter or a digit: a leader, `____`, is none.
    fn lettered_words(&self) -> usize {
        self.text
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .count()
    }

    /// Whether this run is a list's bullet: one glyph a line of a list opens with.
    fn is_bullet(&self) -> bool {
        matches!(
            self.text.trim(),
            "•" | "◦"
                | "▪"
                | "▫"
                | "‣"
                | "⁃"
                | "∙"
                | "●"
                | "○"
                | "■"
                | "-"
                | "–"
                | "—"
                | "*"
        )
    }

    /// Whether this run holds a letter or a digit.
    fn has_words(&self) -> bool {
        self.text.chars().any(char::is_alphanumeric)
    }

    /// Whether this run, of the page's own, is text a run drawn over it hides: a letter or a digit,
    /// more than one glyph, and no leader at its end.
    fn is_words(&self) -> bool {
        self.glyphs() > 1 && self.has_words() && !self.ends_in_leader()
    }
}

/// How far apart two baselines may be and the runs still share a row: 5 pt.
const ROW: i64 = 500;
/// How far a left neighbour's right edge may reach past a run's left edge and still end before
/// it: 2 pt.
const TOUCH: i64 = 200;
/// How far left of a run its caption on the row may end: 250 pt.
const ROW_REACH: i64 = 25_000;
/// How far above a run the line it is set under may be: 40 pt.
const LINE_REACH: i64 = 4_000;
/// How far left of a run the line above may end and still overlap it: 30 pt.
const LINE_SLACK: i64 = 3_000;
/// How far a one-glyph run may be from the option it ticks: 60 pt.
const CHECK_REACH: i64 = 6_000;
/// How far a run's glyphs rise above its baseline, as a share of its type size: seven tenths, a
/// capital's height in most faces.
const ASCENT: (i64, i64) = (7, 10);
/// How many of an overlay's runs must lie inside the page's own text.
const MIN_RUNS: usize = 3;
/// The most words one run of an overlay may hold.
const MAX_WORDS: usize = 8;
/// An overlay brings a font of its own for at least one run in this many: a filler draws its
/// values each in a font of its own.
const FONT_SHARE: usize = 3;
/// How many times larger than its anchor a run may be set and still be a value: a form's value is
/// set up to twice its caption's tiny type, a watermark's letters many times the body's.
const SIZE_RATIO: i64 = 3;
/// The most words a caption's phrase may hold where it ends in no colon, question mark or leader:
/// more is a line of prose, which a pull quote or a figure's labels drawn last sit beside (review).
const CAPTION_WORDS: usize = 5;
/// The most runs an overlay may hold: more is no form a person fills, and each run asks the page's
/// own on its row (review: a crafted page of 32,000 and 32,000 took 27 s).
const MAX_OVERLAY: usize = 1_000;

/// Whether the last stretch from `start` qualifies as an overlay.
fn qualifies(runs: &[Drawn<'_>], start: usize) -> bool {
    let own: Vec<i64> = runs[..start]
        .iter()
        .filter(|r| r.words() > 0)
        .map(|r| r.y)
        .collect();
    let (Some(&top), Some(&bottom)) = (own.iter().min(), own.iter().max()) else {
        return false;
    };
    let shown: Vec<&Drawn<'_>> = runs[start..].iter().filter(|r| r.words() > 0).collect();
    let inside = shown
        .iter()
        .filter(|r| (top - ROW..=bottom + ROW).contains(&r.y))
        .count();
    let short = shown.iter().all(|r| r.words() <= MAX_WORDS);
    let fonts: std::collections::BTreeSet<&str> = shown.iter().map(|r| r.font).collect();
    short
        && inside >= MIN_RUNS
        && 2 * inside >= shown.len()
        && FONT_SHARE * fonts.len() >= shown.len()
}

/// Where the overlay starts, if the page has one: the first run of the longest last stretch in
/// fonts no earlier run uses, where it qualifies.
fn overlay(runs: &[Drawn<'_>]) -> Option<usize> {
    let mut first: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, r) in runs.iter().enumerate() {
        first.entry(r.font).or_insert(i);
    }
    let (mut start, mut earliest) = (runs.len(), runs.len());
    for i in (1..runs.len()).rev() {
        earliest = earliest.min(first[runs[i].font]);
        if earliest >= i {
            start = i;
        }
    }
    (start < runs.len() && qualifies(runs, start)).then_some(start)
}

/// Where an overlay's run is read: before or after a run of the page's own.
#[derive(Clone, Copy)]
enum Anchor {
    Before(usize),
    After(usize),
}

/// `order` with the page's overlay, if it has one, read where it sits; `None` where nothing moves.
/// `runs` are in content-stream order, `regions` the column cut's per run (empty where it did not
/// divide the page), and `tables` the accepted tables' boxes.
pub(crate) fn place(
    runs: &[Drawn<'_>],
    tables: &[QuantRect],
    regions: &[Option<u32>],
    order: &[usize],
) -> Option<Vec<usize>> {
    let start = overlay(runs)?;
    if runs.len() - start > MAX_OVERLAY {
        return None;
    }
    let in_table: Vec<bool> = runs
        .iter()
        .map(|r| {
            tables
                .iter()
                .any(|t| r.x >= t.x0 && r.x < t.x1 && r.y >= t.y0 && r.y < t.y1)
        })
        .collect();
    let region = |i: usize| regions.get(i).copied().flatten();
    let mut position = vec![0; runs.len()];
    for (p, &i) in order.iter().enumerate() {
        position[i] = p;
    }
    // The page's own runs that may anchor one, by baseline, so a run asks only those near it.
    let mut own: Vec<usize> = (0..start)
        .filter(|&j| !in_table[j] && runs[j].words() > 0)
        .collect();
    own.sort_by_key(|&j| (runs[j].y, j));
    let near = |lo: i64, hi: i64| {
        let a = own.partition_point(|&j| runs[j].y < lo);
        let b = own.partition_point(|&j| runs[j].y <= hi);
        &own[a..b]
    };
    // Each own run's phrase on its line, read once for the page (second review: asked for each
    // value it was quadratic): the line's runs set less than their size apart, their words with a
    // letter or a digit, whether one ends in a leader, and whether the last ends a sentence.
    let mut phrase: Vec<(usize, bool, bool)> = vec![(0, false, false); runs.len()];
    let mut i = 0;
    while i < own.len() {
        let y = runs[own[i]].y;
        let mut k = i;
        while k < own.len() && runs[own[k]].y - y <= ROW {
            k += 1;
        }
        let mut line = own[i..k].to_vec();
        line.sort_by_key(|&j| (runs[j].x, j));
        let mut a = 0;
        while a < line.len() {
            let mut b = a;
            while b + 1 < line.len()
                && runs[line[b]].right() + runs[line[b]].size >= runs[line[b + 1]].x
            {
                b += 1;
            }
            let part = &line[a..=b];
            let words = part.iter().map(|&j| runs[j].lettered_words()).sum();
            let leader = part.iter().any(|&j| runs[j].ends_in_leader());
            let sentence = runs[line[b]].text.trim_end().ends_with(['.', '!']);
            for &j in part {
                phrase[j] = (words, leader, sentence);
            }
            a = b + 1;
        }
        i = k;
    }
    // The last run, in reading order, of the line of the page's own that `j` is on.
    let line_end = |j: usize| {
        let mut p = position[j];
        while let Some(&n) = order.get(p + 1) {
            // Review: a table continuing the line is the table's, and the value is not read in it.
            if n >= start || in_table[n] || (runs[n].y - runs[j].y).abs() > ROW {
                break;
            }
            p += 1;
        }
        order[p]
    };
    let mut anchors: Vec<(usize, Anchor)> = Vec::new();
    let mut over = 0usize;
    for o in start..runs.len() {
        let r = &runs[o];
        // A run the page marks `/Artifact` — a watermark, a running head — is no filler's value.
        if r.words() == 0 || in_table[o] || r.artifact {
            continue;
        }
        let same = |j: &&usize| region(**j) == region(o);
        // Drawn over the page's own text: a watermark, a stamp, a mark, not a value.
        let reach = r.rise() + ROW;
        if near(r.y - reach, r.y + reach)
            .iter()
            .any(|&j| runs[j].is_words() && r.covers(&runs[j]))
        {
            over += 1;
            continue;
        }
        let row: Vec<usize> = near(r.y - ROW, r.y + ROW)
            .iter()
            .filter(same)
            .copied()
            .collect();
        let anchor = if !row.is_empty() {
            let lone = r.glyphs() == 1;
            // A mark: one glyph that is no letter or digit. A one-letter or one-digit value — an
            // initial, a count — is read as far from its caption as any value (review).
            let limit = if lone && !r.has_words() {
                CHECK_REACH
            } else {
                ROW_REACH
            };
            row.iter()
                .filter_map(|&j| {
                    let b = &runs[j];
                    // A caption ending in its leader, or a box the mark is drawn in.
                    let caption = (b.has_words() && b.ends_in_leader()) || b.glyphs() == 1;
                    // A caption at all: a colon, a question mark or a leader at its end, a box a
                    // mark is drawn in, or a short phrase that ends no sentence — never a line of
                    // prose, nor a bullet a line of prose follows (reviews: a pull quote's lines
                    // and a figure's labels were read into the paragraph beside them).
                    let end = b.text.trim_end();
                    let (words, leader, sentence) = phrase[j];
                    // One glyph — a box, a currency sign, a form's `×` or `=` — is one, but a
                    // bullet only for a mark (third review: bullets anchored a pull quote; the
                    // `£` before an amount must still anchor it).
                    let captions = (b.glyphs() == 1 && (lone || !b.is_bullet()))
                        || leader
                        || end.ends_with(':')
                        || end.ends_with('?')
                        || ((1..=CAPTION_WORDS).contains(&words) && !sentence);
                    if b.x <= r.x + TOUCH && (b.right() <= r.x + TOUCH || caption) {
                        // Ending before it, or a caption reaching under it with its leader.
                        Some(((r.x - b.right()).max(0), j, Anchor::After(j), captions))
                    } else if lone && b.x >= r.right() - TOUCH / 2 {
                        Some(((b.x - r.right()).max(0), j, Anchor::Before(j), true))
                    } else {
                        None
                    }
                })
                .filter(|(d, ..)| *d <= limit)
                .min_by_key(|(d, j, ..)| (*d, *j))
                // The nearest only: where it is no caption, the run stays where it was, never
                // read after a caption farther off (second review: a value went to the field
                // before its own).
                .and_then(|(_, _, a, ok)| ok.then_some(a))
        } else {
            let across = |j: &&usize| {
                let b = &runs[**j];
                b.x <= r.right() && b.right() >= r.x - LINE_SLACK
            };
            near(r.y - LINE_REACH, r.y - 1)
                .iter()
                .filter(same)
                .filter(across)
                .min_by_key(|&&j| (r.y - runs[j].y, (runs[j].x - r.x).abs(), j))
                .map(|&j| Anchor::After(line_end(j)))
        };
        // Set more than [`SIZE_RATIO`] times larger than the text it would be read beside: a
        // watermark's letters, not a value.
        let Some(a) = anchor else { continue };
        let (Anchor::Before(j) | Anchor::After(j)) = a;
        if r.size > SIZE_RATIO * runs[j].size {
            continue;
        }
        anchors.push((o, a));
    }
    // More of it drawn over the page's own words than read beside them: a watermark or a stamp,
    // not a filler's values.
    if anchors.is_empty() || over >= anchors.len() {
        return None;
    }
    let mut before: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut after: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(o, a) in &anchors {
        match a {
            Anchor::Before(j) => before.entry(j).or_default().push(o),
            Anchor::After(j) => after.entry(j).or_default().push(o),
        }
    }
    let by_rows = |v: &mut Vec<usize>| *v = rows_then_left_to_right(runs, v);
    before.values_mut().for_each(by_rows);
    after.values_mut().for_each(by_rows);
    let moved: std::collections::BTreeSet<usize> = anchors.iter().map(|(o, _)| *o).collect();
    let mut out = Vec::with_capacity(order.len());
    for &i in order {
        if moved.contains(&i) {
            continue;
        }
        out.extend(before.get(&i).into_iter().flatten());
        out.push(i);
        out.extend(after.get(&i).into_iter().flatten());
    }
    debug_assert_eq!(out.len(), order.len(), "placing is a permutation");
    Some(out)
}

/// `runs` at `picked`, read row by row — a row the runs whose baselines are within [`ROW`] of its
/// first's — and left to right in each.
fn rows_then_left_to_right(runs: &[Drawn<'_>], picked: &[usize]) -> Vec<usize> {
    let mut by_y = picked.to_vec();
    by_y.sort_by_key(|&o| (runs[o].y, runs[o].x, o));
    let mut out = Vec::with_capacity(by_y.len());
    let mut row: Vec<usize> = Vec::new();
    for o in by_y {
        if row.first().is_some_and(|&f| runs[o].y - runs[f].y > ROW) {
            row.sort_by_key(|&k| (runs[k].x, k));
            out.append(&mut row);
        }
        row.push(o);
    }
    row.sort_by_key(|&k| (runs[k].x, k));
    out.append(&mut row);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run at `(x, y)` points, `w` points wide, set at 10 points.
    fn run<'a>(x: i64, y: i64, w: i64, font: &'a str, text: &'a str) -> Drawn<'a> {
        Drawn {
            x: x * 100,
            y: y * 100,
            advance: Some(w * 100),
            size: 1_000,
            font,
            text,
            artifact: false,
        }
    }

    /// A form of three captions on three rows, then a filler's three values drawn last.
    fn form<'a>() -> Vec<Drawn<'a>> {
        vec![
            run(50, 100, 40, "F1", "Name:"),
            run(50, 120, 40, "F1", "City:"),
            run(50, 140, 40, "F1", "Phone:"),
            run(100, 100, 60, "F9", "Ada Lovelace"),
            run(100, 120, 50, "F10", "London"),
            run(100, 141, 50, "F11", "555 0100"),
        ]
    }

    fn placed(runs: &[Drawn<'_>]) -> Option<Vec<usize>> {
        let order: Vec<usize> = (0..runs.len()).collect();
        place(runs, &[], &[], &order)
    }

    #[test]
    fn a_fillers_values_are_read_after_their_captions() {
        assert_eq!(placed(&form()), Some(vec![0, 3, 1, 4, 2, 5]));
        // Four values: the longest stretch is read, not the last three alone.
        let mut runs = form();
        runs.insert(3, run(50, 160, 40, "F1", "Email:"));
        runs.push(run(100, 160, 80, "F12", "ada@example.org"));
        assert_eq!(placed(&runs), Some(vec![0, 4, 1, 5, 2, 6, 3, 7]));
    }

    #[test]
    fn a_caption_whose_leader_runs_under_its_value_is_its_caption() {
        // Review: one run `Name: ________` reaching under the value paired it with the row above.
        let mut runs = form();
        runs[0] = run(50, 100, 150, "F1", "Name: __________");
        runs[1] = run(50, 120, 150, "F1", "City: ..........");
        assert_eq!(placed(&runs), Some(vec![0, 3, 1, 4, 2, 5]));
    }

    #[test]
    fn a_value_under_its_caption_is_read_after_that_line() {
        let mut runs = form();
        // The city set under its caption, the caption's line two runs long.
        runs.insert(2, run(95, 120, 20, "F1", "(town)"));
        runs[5] = run(55, 128, 50, "F10", "London");
        assert_eq!(placed(&runs), Some(vec![0, 4, 1, 2, 5, 3, 6]));
    }

    #[test]
    fn a_caption_reaching_under_a_value_without_a_leader_is_no_anchor() {
        // A long caption whose run goes on under the value, a little above it: no anchor, so the
        // value stays where it was rather than being read inside the caption's line.
        let mut runs = form();
        runs[0] = run(50, 100, 200, "F1", "Name of the applicant in full");
        runs[3] = run(100, 104, 60, "F9", "Ada Lovelace");
        assert_eq!(placed(&runs), Some(vec![0, 1, 4, 2, 5, 3]));
    }

    #[test]
    fn a_leader_drawn_on_its_own_is_no_caption() {
        // The dots drawn before their caption, the value typed over them: after the caption.
        let mut runs = vec![
            run(160, 100, 100, "F1", "................"),
            run(50, 100, 100, "F1", "Duration of stay"),
        ];
        runs.extend(form()[1..3].iter().copied());
        runs.extend([
            run(180, 100, 40, "F9", "1 years"),
            run(100, 120, 50, "F10", "London"),
            run(100, 141, 50, "F11", "555 0100"),
        ]);
        assert_eq!(placed(&runs), Some(vec![0, 1, 4, 2, 5, 3, 6]));
    }

    #[test]
    fn a_one_glyph_mark_is_read_before_the_option_it_ticks() {
        // Review: ZapfDingbats' check reads as `4`, a typed one as `X`.
        for mark in ["✓", "4", "X"] {
            let runs = vec![
                run(60, 100, 30, "F1", "Yes"),
                run(110, 100, 30, "F1", "No"),
                run(50, 120, 60, "F1", "Signed"),
                run(103, 100, 6, "F9", mark),
                run(120, 120, 40, "F10", "A. Lovelace"),
                run(150, 100, 40, "F11", "today"),
            ];
            assert_eq!(placed(&runs), Some(vec![0, 3, 1, 5, 2, 4]), "{mark}");
        }
        // A mark before a row's first option, its question too far to its left to be read by.
        let runs = vec![
            run(50, 100, 30, "F1", "Smoker?"),
            run(160, 100, 20, "F1", "Yes"),
            run(200, 100, 20, "F1", "No"),
            run(50, 120, 60, "F1", "Signed"),
            run(150, 100, 6, "F9", "X"),
            run(120, 120, 40, "F10", "A. Lovelace"),
            run(230, 100, 40, "F11", "today"),
        ];
        assert_eq!(placed(&runs), Some(vec![0, 4, 1, 2, 6, 3, 5]));
        // A mark inside a box glyph goes after the box, with a width or none: the box is one glyph,
        // not words the mark is drawn over.
        for advance in [None, Some(400)] {
            let runs = vec![
                run(50, 100, 6, "F1", "o"),
                run(60, 100, 30, "F1", "Yes"),
                run(50, 120, 60, "F1", "Signed"),
                Drawn {
                    advance,
                    ..run(51, 100, 0, "F9", "✔")
                },
                run(120, 120, 40, "F10", "A. Lovelace"),
                run(150, 100, 40, "F11", "today"),
            ];
            assert_eq!(placed(&runs), Some(vec![0, 3, 1, 5, 2, 4]), "{advance:?}");
        }
    }

    #[test]
    fn text_drawn_over_the_pages_own_stays_where_it_was() {
        // A watermark across the body, and a stamp over a line: drawn over words, not by them.
        let runs = vec![
            run(50, 100, 300, "F1", "The board met on Tuesday and agreed"),
            run(50, 112, 300, "F1", "to a revised plan for the next year"),
            run(50, 124, 300, "F1", "together with an estimate of the cost"),
            run(120, 112, 90, "W1", "CONFIDENTIAL"),
            run(150, 124, 60, "W2", "RECEIVED"),
            run(200, 100, 60, "W3", "19 NOV"),
        ];
        assert_eq!(placed(&runs), None);
    }

    #[test]
    fn a_stamp_mostly_over_the_text_moves_nothing_and_a_large_run_stays() {
        // Two of three runs drawn over words, one in the margin by a line: a stamp, untouched.
        let body = |runs: &mut Vec<Drawn<'_>>| {
            runs.extend([
                run(50, 100, 300, "F1", "The board met on Tuesday and agreed"),
                run(50, 112, 300, "F1", "to a revised plan for the next year"),
                run(50, 124, 300, "F1", "together with an estimate of the cost"),
            ]);
        };
        let mut runs = Vec::new();
        body(&mut runs);
        runs.extend([
            run(120, 112, 90, "W1", "CONFIDENTIAL"),
            run(150, 124, 60, "W2", "RECEIVED"),
            run(400, 100, 60, "W3", "19 NOV"),
        ]);
        assert_eq!(placed(&runs), None);
        // A watermark's letter four times the body's size, set by a line, stays where it was.
        let mut runs = form();
        runs[4] = Drawn {
            size: 4_000,
            ..run(100, 120, 30, "W9", "C")
        };
        assert_eq!(placed(&runs), Some(vec![0, 3, 1, 2, 5, 4]));
        // So does one the page marks `/Artifact`, at the body's size: no box measured its turn.
        let mut runs = form();
        runs[4] = Drawn {
            artifact: true,
            ..run(100, 120, 30, "W9", "C")
        };
        assert_eq!(placed(&runs), Some(vec![0, 3, 1, 2, 5, 4]));
        // A value set twice the size of its caption, under its foot, is read: it grazes it.
        let mut runs = form();
        runs[4] = Drawn {
            size: 2_000,
            ..run(55, 134, 50, "F10", "London")
        };
        assert_eq!(placed(&runs), Some(vec![0, 3, 1, 4, 2, 5]));
    }

    #[test]
    fn a_form_whose_title_has_a_font_of_its_own_is_not_found() {
        // A known limit: the longest stretch in new fonts begins at the title and holds the
        // captions, and does not qualify. Trying shorter ones found overlays on text pages too.
        let mut runs = vec![run(50, 80, 100, "T1", "Claim form")];
        runs.extend(form());
        assert_eq!(placed(&runs), None);
    }

    #[test]
    fn two_runs_of_one_value_are_read_left_to_right_whatever_their_baselines() {
        // Review: a 0.3 pt rise put the second word first.
        let mut runs = form();
        runs[3] = run(100, 100, 25, "F9", "Ada");
        runs.push(Drawn {
            y: 9_970,
            ..run(130, 100, 40, "F12", "Lovelace")
        });
        assert_eq!(placed(&runs), Some(vec![0, 3, 6, 1, 4, 2, 5]));
    }

    #[test]
    fn text_drawn_last_in_one_font_is_no_overlay() {
        // A column, a chart's labels: many short runs drawn last in one font of their own.
        let mut runs = form();
        runs.insert(3, run(50, 160, 40, "F1", "Email:"));
        runs.push(run(100, 160, 80, "F9", "ada@example.org"));
        for r in &mut runs[4..] {
            r.font = "F9";
        }
        assert_eq!(placed(&runs), None);
        // A third of them in fonts of their own is a filler's.
        runs[7].font = "F10";
        assert!(placed(&runs).is_some());
    }

    #[test]
    fn a_page_with_no_overlay_is_untouched() {
        // Fewer than three values.
        let mut runs = form();
        runs.truncate(5);
        assert_eq!(placed(&runs), None);
        // A value longer than eight words.
        let mut runs = form();
        runs[5].text = "one two three four five six seven eight nine";
        assert_eq!(placed(&runs), None);
        // Drawn last, but in a font the page's own text uses.
        let mut runs = form();
        for r in &mut runs[3..] {
            r.font = "F1";
        }
        assert_eq!(placed(&runs), None);
        // Mostly below the page's own text: a footer, not a filler.
        let mut runs = form();
        runs[4].y = 70_000;
        runs[5].y = 71_000;
        assert_eq!(placed(&runs), None);
    }

    #[test]
    fn a_value_in_a_table_or_another_region_stays_where_it_was() {
        let runs = form();
        let order: Vec<usize> = (0..runs.len()).collect();
        // The city's value inside an accepted table: its cell places it.
        let table = QuantRect {
            x0: 9_000,
            y0: 11_000,
            x1: 20_000,
            y1: 13_000,
        };
        assert_eq!(
            place(&runs, &[table], &[], &order),
            Some(vec![0, 3, 1, 2, 5, 4])
        );
        // The city's value in another column than its caption.
        let regions = [Some(1), Some(1), Some(1), Some(1), Some(2), Some(1)];
        assert_eq!(
            place(&runs, &[], &regions, &order),
            Some(vec![0, 3, 1, 2, 5, 4])
        );
    }

    /// Review: text drawn last beside lines of prose — a pull quote, a figure's labels — is no
    /// filler's values, though in fonts of its own and on the prose's rows; prose drawn a word to
    /// a run is prose too.
    #[test]
    fn text_drawn_last_beside_prose_stays_where_it_was() {
        let prose = |y: i64| {
            run(
                50,
                y,
                200,
                "F1",
                "that nobody expected to be close or quick",
            )
        };
        let mut runs: Vec<Drawn<'_>> = (0..6).map(|k| prose(100 + 12 * k)).collect();
        runs.extend([
            run(300, 112, 120, "Q1", "this close."),
            run(300, 124, 120, "Q2", "A committee member"),
            run(300, 136, 120, "Q1", "said so"),
        ]);
        assert_eq!(placed(&runs), None);
        // The same prose, a word to a run, an em apart.
        let words = ["that", "nobody", "expected", "to", "be", "close"];
        let mut runs: Vec<Drawn<'_>> = Vec::new();
        for k in 0..6 {
            for (w, word) in words.iter().enumerate() {
                runs.push(run(50 + 40 * w as i64, 100 + 12 * k, 33, "F1", word));
            }
        }
        runs.extend([
            run(300, 112, 120, "Q1", "this close."),
            run(300, 124, 120, "Q2", "A committee member"),
            run(300, 136, 120, "Q1", "said so"),
        ]);
        assert_eq!(placed(&runs), None);
    }

    /// Review: a caption of more words than a phrase holds is still one where it ends in a colon or
    /// a question mark.
    #[test]
    fn a_long_caption_ending_in_a_colon_or_a_question_mark_is_a_caption() {
        let runs = vec![
            run(
                50,
                100,
                150,
                "F1",
                "Please state your full legal name here:",
            ),
            run(
                50,
                120,
                150,
                "F1",
                "Are you employed by this company today?",
            ),
            run(50, 140, 40, "F1", "Phone:"),
            run(210, 100, 60, "F9", "Ada Lovelace"),
            run(210, 120, 30, "F10", "Yes"),
            run(100, 140, 50, "F11", "555 0100"),
        ];
        assert_eq!(placed(&runs), Some(vec![0, 3, 1, 4, 2, 5]));
    }

    /// Review: a one-digit or one-letter value is a value, read as far from its caption as any;
    /// only a mark keeps the check's short reach.
    #[test]
    fn a_one_character_value_is_read_after_its_caption() {
        let mut runs = form();
        runs[3] = run(180, 100, 6, "F9", "3");
        assert_eq!(placed(&runs), Some(vec![0, 3, 1, 4, 2, 5]));
    }

    /// Review: a value under a caption whose line a table continues is read after the caption,
    /// not inside the table.
    #[test]
    fn a_line_is_not_followed_into_a_table() {
        let runs = vec![
            run(50, 100, 50, "F1", "Charges:"),
            run(120, 100, 40, "F1", "Item"),
            run(120, 112, 40, "F1", "Rent"),
            run(50, 140, 50, "F1", "Name:"),
            run(50, 160, 50, "F1", "City:"),
            run(55, 114, 40, "F9", "Paid"),
            run(100, 140, 60, "F10", "Ada Lovelace"),
            run(100, 160, 50, "F11", "London"),
        ];
        let table = QuantRect {
            x0: 11_500,
            y0: 9_000,
            x1: 20_000,
            y1: 12_000,
        };
        let order: Vec<usize> = (0..runs.len()).collect();
        assert_eq!(
            place(&runs, &[table], &[], &order),
            Some(vec![0, 5, 1, 2, 3, 6, 4, 7])
        );
    }

    /// Review: an overlay of more than [`MAX_OVERLAY`] runs is no form a person fills.
    #[test]
    fn a_huge_overlay_moves_nothing() {
        let fonts: Vec<String> = (0..=MAX_OVERLAY).map(|k| format!("V{k}")).collect();
        let mut runs = form()[..3].to_vec();
        runs.extend(fonts.iter().map(|f| run(100, 100, 10, f, "x")));
        assert_eq!(placed(&runs), None);
    }

    /// Third review: only the nearest run on the row is read; where it is no caption — a long
    /// phrase ending in no colon — the value stays where it was, never read after a caption
    /// farther off, another field's.
    #[test]
    fn a_value_beside_no_caption_stays_where_it_was() {
        let runs = vec![
            run(50, 100, 30, "F1", "Code:"),
            run(
                90,
                100,
                200,
                "F1",
                "Name of employer or business you work for",
            ),
            run(50, 120, 40, "F1", "City:"),
            run(50, 140, 40, "F1", "Phone:"),
            run(300, 100, 60, "F9", "Analytical Engines"),
            run(100, 120, 50, "F10", "London"),
            run(100, 141, 50, "F11", "555 0100"),
        ];
        assert_eq!(placed(&runs), Some(vec![0, 1, 2, 5, 3, 6, 4]));
    }

    /// Third review: a short line that ends a sentence — a paragraph's last — is prose, and a
    /// pull quote beside it stays; bullets a pull quote's lines sit beside are no captions either.
    #[test]
    fn a_sentences_end_or_a_bullet_is_no_caption() {
        let prose = "the committee met again on tuesday to weigh";
        let mut runs = vec![
            run(50, 100, 200, "F1", prose),
            run(50, 112, 60, "F1", "end of story."),
            run(50, 124, 200, "F1", prose),
        ];
        runs.extend([
            run(300, 100, 100, "Q1", "Nobody expected"),
            run(300, 112, 100, "Q2", "the vote to be"),
            run(300, 124, 100, "Q1", "this close."),
        ]);
        assert_eq!(placed(&runs), None);
        let runs = vec![
            run(50, 100, 4, "F1", "•"),
            run(50, 112, 4, "F1", "•"),
            run(50, 124, 4, "F1", "•"),
            run(58, 100, 100, "Q1", "Nobody expected"),
            run(58, 112, 100, "Q2", "the vote to be"),
            run(58, 124, 100, "Q1", "this close."),
        ];
        assert_eq!(placed(&runs), None);
    }

    /// Third review: a leader drawn as a run of its own is no word of its caption's phrase, and a
    /// phrase ending in one is a caption however long.
    #[test]
    fn a_caption_ending_in_a_leader_run_is_one_however_long() {
        let runs = vec![
            run(50, 100, 150, "F1", "Name of your current employer here"),
            run(203, 100, 60, "F1", "______"),
            run(50, 120, 40, "F1", "City:"),
            run(50, 140, 40, "F1", "Phone:"),
            run(270, 100, 60, "F9", "Analytical"),
            run(100, 120, 50, "F10", "London"),
            run(100, 141, 50, "F11", "555 0100"),
        ];
        assert_eq!(placed(&runs), Some(vec![0, 1, 4, 2, 5, 3, 6]));
    }

    /// Third review's measurement: one glyph that is no bullet — a currency sign, a form's `×` —
    /// is a caption the amount after it is read after; ParseBench-2's forms lost their amounts.
    #[test]
    fn a_currency_sign_is_the_caption_of_the_amount_after_it() {
        let runs = vec![
            run(50, 100, 120, "F1", "85 Box 77 minus box 84"),
            run(300, 100, 6, "F1", "£"),
            run(50, 120, 40, "F1", "City:"),
            run(50, 140, 40, "F1", "Phone:"),
            run(310, 100, 40, "F9", "7,126"),
            run(100, 120, 50, "F10", "London"),
            run(100, 141, 50, "F11", "555 0100"),
        ];
        assert_eq!(placed(&runs), Some(vec![0, 1, 4, 2, 5, 3, 6]));
    }
}
