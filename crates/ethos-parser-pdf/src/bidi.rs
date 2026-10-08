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

//! **A right-to-left line read right to left** — `ethos_parser_core::RIGHT_TO_LEFT_RULE_V1`
//! (decision #54).
//!
//! A producer whose layout engine resolved bidi draws Hebrew and Arabic glyphs where they sit,
//! left to right, so a run's codes — and the text read off them — run against the reading: the
//! `שלום` a reader sees is drawn `ם`, `ו`, `ל`, `ש`. This rule reads such a line as its reader
//! does.
//!
//! A **line** is a stretch of runs consecutive in reading order, in one region, whose baselines lie
//! within [`crate::blocks::LINE_TOLERANCE`] of the first's: one baseline, with the marks set a
//! little above or below it. A line whose right-to-left letters (Unicode `Bidi_Class` R or AL)
//! outnumber its left-to-right ones (L) is read:
//!
//! 1. its **glyphs** are taken where the page draws them, left to right — its runs by where they
//!    start, and each run's glyphs the way its pen advanced. A glyph is one code's characters,
//!    kept whole: a lam-alef ligature's two letters stay in the order its map gives them, and a
//!    space this reader inserted after a run is a glyph of its own;
//! 2. the Unicode Bidirectional Algorithm (UAX #9, through `unicode-bidi`) resolves each glyph's
//!    level in a right-to-left paragraph, and its reordering, applied to the glyphs as drawn,
//!    gives the order they are read in: right to left, with a number or a Latin word inside read
//!    left to right.
//!
//! Each run of the line takes its place in reading order where its first glyph is read, and its
//! glyphs as read are its `reading` — the run's own characters, moved, none added and none
//! dropped, with a space this reader inserted flagged where it is read. **The run's `text`,
//! `char_codes` and `synthesized` keep the order the page drew**: they are `Extracted` and
//! untouched, and the reading is `Computed` beside them.
//!
//! **Refused, line by line**, leaving its runs as drawn for `right-to-left-not-reordered` to
//! count: a line where a run's glyphs would not be read together — a number begun in one run
//! and finished in a run beside one of its letters — and a line holding a run of several
//! right-to-left glyphs whose pen states no direction (no advance, or none along the line).

use ethos_parser_core::Reading;
use unicode_bidi::{bidi_class, BidiClass, Level, ParagraphBidiInfo};

/// One run as the rule reads it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seen<'a> {
    /// The run's text, in the order its codes were drawn.
    pub text: &'a str,
    /// How many characters of `text` each code decoded to, in the order drawn. Characters past
    /// their sum are spaces this reader inserted, each a glyph of its own.
    pub code_chars: &'a [usize],
    /// The origin, top-left centipoints.
    pub x: i64,
    /// The baseline, top-left centipoints.
    pub y: i64,
    /// The pen's travel over the run along the line, or `None` where its font gives no widths.
    pub advance: Option<i64>,
    /// The reading-order region the run lies in.
    pub region: Option<u32>,
}

/// The reading order with each right-to-left line read right to left, and every run such a line
/// holds as read.
///
/// `order` is the page's reading order over the positions of `runs`; the order returned holds the
/// same positions, and the readings are indexed by position. A page holding no right-to-left
/// letter comes back as it went in.
pub(crate) fn read(runs: &[Seen<'_>], order: &[usize]) -> (Vec<usize>, Vec<Option<Reading>>) {
    let mut read_order = order.to_vec();
    let mut readings = vec![None; runs.len()];
    let holds_letter = |i: usize| runs[i].text.chars().any(is_right_to_left);
    if !(0..runs.len()).any(holds_letter) {
        return (read_order, readings);
    }
    let mut start = 0;
    while start < order.len() {
        let first = &runs[order[start]];
        let end = order[start..]
            .iter()
            .position(|&i| {
                runs[i].region != first.region
                    || (runs[i].y - first.y).abs() > crate::blocks::LINE_TOLERANCE
            })
            .map_or(order.len(), |n| start + n);
        let members = &order[start..end];
        if members.iter().any(|&i| holds_letter(i)) {
            if let Some(line) = line(runs, members) {
                for (k, (i, reading)) in line.into_iter().enumerate() {
                    read_order[start + k] = i;
                    readings[i] = Some(reading);
                }
            }
        }
        start = end;
    }
    (read_order, readings)
}

/// Whether `c` is a right-to-left letter: `Bidi_Class` R or AL.
fn is_right_to_left(c: char) -> bool {
    matches!(bidi_class(c), BidiClass::R | BidiClass::AL)
}

/// One line's runs in the order they are read, each as read; `None` where the line is not read
/// right to left or is refused.
fn line(runs: &[Seen<'_>], members: &[usize]) -> Option<Vec<(usize, Reading)>> {
    // The glyphs left to right, each with the member it belongs to.
    let mut by_x: Vec<usize> = (0..members.len()).collect();
    by_x.sort_by_key(|&j| {
        let run = &runs[members[j]];
        run.x + run.advance.unwrap_or(0).min(0)
    });
    let mut glyphs: Vec<(usize, Glyph<'_>)> = Vec::new();
    for &j in &by_x {
        let run = &runs[members[j]];
        let mut drawn = glyphs_of(run.text, run.code_chars)?;
        let letters = drawn
            .iter()
            .filter(|g| g.text.chars().any(is_right_to_left))
            .count();
        match run.advance {
            Some(a) if a > 0 => {}
            Some(a) if a < 0 => drawn.reverse(),
            _ if letters > 1 => return None,
            _ => {}
        }
        glyphs.extend(drawn.into_iter().map(|g| (j, g)));
    }

    let (mut rtl, mut ltr) = (0usize, 0usize);
    for c in glyphs.iter().flat_map(|(_, g)| g.text.chars()) {
        match bidi_class(c) {
            BidiClass::R | BidiClass::AL => rtl += 1,
            BidiClass::L => ltr += 1,
            _ => {}
        }
    }
    if rtl <= ltr {
        return None;
    }

    // One character standing for each glyph, so the algorithm moves glyphs and never splits one.
    let stand_ins: String = glyphs.iter().map(|(_, g)| stand_in(g.text)).collect();
    let info = ParagraphBidiInfo::new(&stand_ins, Some(Level::rtl()));
    let levels = info.reordered_levels_per_char(0..stand_ins.len());
    let reading = ParagraphBidiInfo::reorder_visual(&levels);

    // Where each member's glyphs are read: a run whose glyphs are not read together refuses the
    // line, since no order of whole runs reads it.
    let mut span: Vec<Option<(usize, usize)>> = vec![None; members.len()];
    for (k, &g) in reading.iter().enumerate() {
        let j = glyphs[g].0;
        span[j] = match span[j] {
            None => Some((k, 1)),
            Some((first, count)) if first + count == k => Some((first, count + 1)),
            Some(_) => return None,
        };
    }
    let mut spans: Vec<(usize, usize, usize)> = Vec::with_capacity(members.len());
    for (j, s) in span.into_iter().enumerate() {
        let (first, count) = s?;
        spans.push((first, count, j));
    }
    spans.sort_unstable();
    Some(
        spans
            .into_iter()
            .map(|(first, count, j)| {
                let mut read = Reading {
                    text: String::new(),
                    synthesized: Vec::new(),
                };
                let mut chars = 0u32;
                for &g in &reading[first..first + count] {
                    let glyph = &glyphs[g].1;
                    if glyph.inserted {
                        read.synthesized.push(chars);
                    }
                    read.text.push_str(glyph.text);
                    chars = chars.saturating_add(
                        u32::try_from(glyph.text.chars().count()).unwrap_or(u32::MAX),
                    );
                }
                (members[j], read)
            })
            .collect(),
    )
}

/// One glyph: one code's characters, or one character this reader inserted.
#[derive(Debug, Clone, Copy)]
struct Glyph<'a> {
    text: &'a str,
    inserted: bool,
}

/// `text` cut into its glyphs, in the order drawn: each code's characters, then each character
/// past them alone, as one this reader inserted. `None` where the counts claim more characters
/// than `text` holds.
fn glyphs_of<'a>(text: &'a str, code_chars: &[usize]) -> Option<Vec<Glyph<'a>>> {
    let mut out = Vec::with_capacity(code_chars.len());
    let mut rest = text;
    for &n in code_chars.iter().filter(|&&n| n > 0) {
        let end = rest.char_indices().nth(n).map_or(rest.len(), |(i, _)| i);
        if rest[..end].chars().count() < n {
            return None;
        }
        out.push(Glyph {
            text: &rest[..end],
            inserted: false,
        });
        rest = &rest[end..];
    }
    out.extend(rest.char_indices().map(|(i, c)| Glyph {
        text: &rest[i..i + c.len_utf8()],
        inserted: true,
    }));
    Some(out)
}

/// The character whose class a glyph takes: its first strong or numeric character, else its
/// first that is not a combining mark, else its first.
fn stand_in(glyph: &str) -> char {
    let class = |c: char| bidi_class(c);
    glyph
        .chars()
        .find(|&c| {
            matches!(
                class(c),
                BidiClass::L | BidiClass::R | BidiClass::AL | BidiClass::EN | BidiClass::AN
            )
        })
        .or_else(|| glyph.chars().find(|&c| class(c) != BidiClass::NSM))
        .or_else(|| glyph.chars().next())
        .unwrap_or(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run on the line at baseline 1 000, advancing 300 a glyph, each code one character
    /// unless `code_chars` says otherwise.
    fn glyph_run<'a>(text: &'a str, code_chars: &'a [usize], x: i64) -> Seen<'a> {
        let glyphs = glyphs_of(text, code_chars).expect("counts fit").len();
        Seen {
            text,
            code_chars,
            x,
            y: 1_000,
            advance: Some(300 * i64::try_from(glyphs).expect("small")),
            region: None,
        }
    }

    fn run(text: &str, x: i64) -> Seen<'_> {
        const ONES: &[usize] = &[1; 16];
        glyph_run(text, &ONES[..text.chars().count()], x)
    }

    /// The reading order and each run's text as read.
    fn read_all(runs: &[Seen<'_>]) -> (Vec<usize>, Vec<Option<String>>) {
        let (order, readings) = read(runs, &(0..runs.len()).collect::<Vec<_>>());
        (
            order,
            readings.into_iter().map(|r| r.map(|r| r.text)).collect(),
        )
    }

    #[test]
    fn a_word_drawn_left_to_right_reads_right_to_left() {
        // שלום, drawn as its glyphs sit: ם ו ל ש.
        let runs = [run("\u{5DD}\u{5D5}\u{5DC}\u{5E9}", 1_000)];
        let (order, texts) = read_all(&runs);
        assert_eq!(order, vec![0]);
        assert_eq!(texts[0].as_deref(), Some("\u{5E9}\u{5DC}\u{5D5}\u{5DD}"));
    }

    #[test]
    fn a_glyph_of_two_letters_keeps_their_order() {
        // سلام drawn as م, the lam-alef ligature (one code mapped to ل then ا), س.
        let runs = [glyph_run("\u{645}\u{644}\u{627}\u{633}", &[1, 2, 1], 1_000)];
        let (_, texts) = read_all(&runs);
        assert_eq!(texts[0].as_deref(), Some("\u{633}\u{644}\u{627}\u{645}"));
    }

    #[test]
    fn a_space_the_reader_inserted_is_flagged_where_it_is_read() {
        // על, then the space this reader inserted for the gap after it — read first, as the gap
        // to the right of a word read right to left is.
        let runs = [glyph_run("\u{5DC}\u{5E2} ", &[1, 1], 1_000)];
        let (_, readings) = read(&runs, &[0]);
        let reading = readings[0].as_ref().expect("read");
        assert_eq!(reading.text, " \u{5E2}\u{5DC}");
        assert_eq!(reading.synthesized, vec![0]);
    }

    #[test]
    fn runs_drawn_left_to_right_are_read_right_to_left() {
        // עולם, a space, שלום — drawn left to right as the page sets them, so read from the right.
        let runs = [
            run("\u{5DD}\u{5DC}\u{5D5}\u{5E2}", 1_000),
            run(" ", 2_200),
            run("\u{5DD}\u{5D5}\u{5DC}\u{5E9}", 2_500),
        ];
        let (order, texts) = read_all(&runs);
        assert_eq!(order, vec![2, 1, 0]);
        assert_eq!(texts[2].as_deref(), Some("\u{5E9}\u{5DC}\u{5D5}\u{5DD}"));
        assert_eq!(texts[1].as_deref(), Some(" "));
        assert_eq!(texts[0].as_deref(), Some("\u{5E2}\u{5D5}\u{5DC}\u{5DD}"));
    }

    #[test]
    fn a_number_in_a_right_to_left_line_reads_left_to_right() {
        // `שלום 123`: the number sits to the left of the word, its digits in their own order.
        let runs = [
            run("12", 1_000),
            run("34", 1_600),
            run(" ", 2_200),
            run("\u{5DD}\u{5D5}\u{5DC}\u{5E9}", 2_500),
        ];
        let (order, texts) = read_all(&runs);
        assert_eq!(order, vec![3, 2, 0, 1]);
        assert_eq!(texts[0].as_deref(), Some("12"));
        assert_eq!(texts[1].as_deref(), Some("34"));
    }

    #[test]
    fn a_run_whose_pen_moved_left_reads_in_its_own_order() {
        // שלום with its codes in reading order and its pen moving left: the glyphs sit right to
        // left as the codes run, so read from the right they are read in the codes' order.
        let mut word = run("\u{5E9}\u{5DC}\u{5D5}\u{5DD}", 2_200);
        word.advance = Some(-1_200);
        let (_, texts) = read_all(&[word]);
        assert_eq!(texts[0].as_deref(), Some("\u{5E9}\u{5DC}\u{5D5}\u{5DD}"));
    }

    #[test]
    fn a_line_mostly_left_to_right_is_left_as_drawn() {
        let runs = [
            run("Hello", 1_000),
            run(" ", 2_500),
            run("\u{5DD}\u{5D5}\u{5DC}\u{5E9}", 2_800),
        ];
        let (order, texts) = read_all(&runs);
        assert_eq!(order, vec![0, 1, 2]);
        assert!(texts.iter().all(Option::is_none));
    }

    #[test]
    fn a_run_whose_glyphs_are_not_read_together_refuses_its_line() {
        // `ש 123` drawn as `12` and `3 ש`: the number's last digit is read after the letter its
        // run holds, apart from it, so no order of whole runs reads the line.
        let runs = [run("12", 1_000), run("3 \u{5E9}", 1_600)];
        let (order, texts) = read_all(&runs);
        assert_eq!(order, vec![0, 1]);
        assert!(texts.iter().all(Option::is_none));
    }

    #[test]
    fn a_run_with_no_direction_refuses_its_line() {
        let mut word = run("\u{5DD}\u{5D5}\u{5DC}\u{5E9}", 1_000);
        word.advance = None;
        let (_, texts) = read_all(&[word]);
        assert_eq!(texts[0], None);
        // One glyph needs no direction.
        let mut letter = run("\u{5E9}", 1_000);
        letter.advance = None;
        let (_, texts) = read_all(&[letter]);
        assert_eq!(texts[0].as_deref(), Some("\u{5E9}"));
    }

    #[test]
    fn two_baselines_are_two_lines() {
        let mut below = run("\u{5D1}\u{5D0}", 3_000);
        below.y += crate::blocks::LINE_TOLERANCE + 1;
        let runs = [run("\u{5D3}\u{5D2}", 1_000), below];
        let (order, texts) = read_all(&runs);
        assert_eq!(
            order,
            vec![0, 1],
            "each line read alone keeps the lines in order"
        );
        assert_eq!(texts[0].as_deref(), Some("\u{5D2}\u{5D3}"));
        assert_eq!(texts[1].as_deref(), Some("\u{5D0}\u{5D1}"));
    }

    #[test]
    fn two_regions_are_two_lines() {
        // The same baseline, consecutive in reading order, but either side of a column gutter:
        // each column's line is read alone.
        let mut right = run("\u{5D1}\u{5D0}", 9_000);
        right.region = Some(2);
        let mut left = run("\u{5D3}\u{5D2}", 1_000);
        left.region = Some(1);
        let (order, texts) = read_all(&[left, right]);
        assert_eq!(order, vec![0, 1]);
        assert_eq!(texts[0].as_deref(), Some("\u{5D2}\u{5D3}"));
        assert_eq!(texts[1].as_deref(), Some("\u{5D0}\u{5D1}"));
    }

    #[test]
    fn a_page_with_no_right_to_left_letter_is_untouched() {
        let runs = [run("b", 2_000), run("a", 1_000)];
        let (order, texts) = read_all(&runs);
        assert_eq!(order, vec![0, 1]);
        assert!(texts.iter().all(Option::is_none));
    }
}
