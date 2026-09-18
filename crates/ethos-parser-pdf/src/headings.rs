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

//! **Headings inferred from the type the page draws** — decision #29, and
//! `docs/28-HEADINGS-SCOPE.md` §3, which is the rule this file is.
//!
//! On a document that declares no structure, a line is an inferred heading when every run of it
//! with a measurable rendered em is drawn at least **1.20×** the document's body em. The verdict
//! reaches the wire as `TextRunAttributes::inferred_heading`, `Computed` and never the author's,
//! with `headings-inferred-from-type` declared beside it. The profile's `heading_inference_rule`
//! names the rule, and a profile naming anything else runs nothing here.
//!
//! # What it reads, and what it may not
//!
//! **The rendered em**: the vertical scale of the text rendering matrix, `Tfs` composed with the
//! text matrix and the CTM (§9.4.4). **Not the `Tf` operand**, which a page may set to 1 while it
//! draws its type in the matrix — 87 of the 200 bench documents carry one `font_size` on every
//! run, so the operand says nothing on 43.5% of the corpus (§3.1). **Not the ink box's height**,
//! which is the font's ascent-to-descent envelope rather than its type, and costs a 29.37%
//! false-positive rate on `nist-sp-800-218` (§3.1, §7.3).
//!
//! **It reads type, never position.** Decision #29's rider says so, and the reading-order cut's
//! two fields — the column band and the leading-gap index — are exactly the position a rule
//! reaching for "a heading is a short line above a gap" would read. So this file's code may not
//! name either, and `the_rule_reads_no_region_and_no_block` scans for both words. The caller
//! forms the lines — the runs sharing one page, band, `/Artifact` state and baseline, which is
//! `markdown.rs`'s `LineKey` — and hands this module each line's runs by their type alone.
//!
//! # The reference is the document's own — its largest common size (`type-size-v2`)
//!
//! A 14pt heading is display type in a 10pt document and body type in a 17pt one, so the cut is a
//! multiple of a number measured on the document under test (§3.2), per document and not per page.
//!
//! **The body is the largest size the document sets a substantial share of its text in** — a size
//! holding at least 1/[`BODY_SHARE_DEN`] of the body characters and running on at least
//! [`BODY_MIN_LINES`] lines — and the most common size only where no size is that. `type-size-v1`
//! took the most common size by characters, and the false-positive measurement showed what that
//! costs (`docs/measurements/headings/README.md`): on `nist-sp-800-218` 63% of the characters are
//! 9pt small type and the prose is 12pt, on `nist-sp-800-53Ar5` 75% are 8.5pt assessment text and
//! the prose is 11pt, so the mode made ordinary prose clear the cut on every line — 2,979 false
//! headings against 68 declared. **Dense small type is common; it is not the body.**
//!
//! **Both conditions, because each alone fails a different document.** A share of characters alone
//! makes a short page's title the body — one 24pt line over five short ones is 9.5% of that page's
//! characters. A share of lines alone does the same on a six-line page. A size running on ten or
//! more lines is text a reader reads, and no title or heading block in the eleven measured
//! documents does.

use std::collections::BTreeMap;

/// Numerator of the heading cut: a line's type clears it at [`HEADING_CUT_NUM`]/[`HEADING_CUT_DEN`]
/// of the body em.
///
/// **1.20×, and not a measured optimum, because there is none to take** (§3.4): the proxy's band
/// moves by 0.1 points between 1.15× and 1.25×, and the one document that decides the bound is
/// insensitive to the threshold because its errors come from elsewhere. The round number inside
/// the interval the evidence does not distinguish; §7.5's false-positive bound, not this
/// constant, is what the rule ships on. Written as integers because contract §4 admits no float.
pub(crate) const HEADING_CUT_NUM: i64 = 6;
/// Denominator of the heading cut. 6/5 is 1.20.
pub(crate) const HEADING_CUT_DEN: i64 = 5;

/// A size is **common** when it holds at least 1/`BODY_SHARE_DEN` of the body characters: 1/20.
///
/// **Inside the interval the evidence does not distinguish, and at its conservative end.** Over the
/// eleven documents whose authors declare headings, every size above the body holds at most 4.2%
/// of the characters, and the smallest prose size that must count as body holds 10.4%
/// (`nist-sp-800-53Ar5`'s 11pt). Any share in (4.2%, 10.4%] picks the same reference on all eleven.
/// A lower share lets more sizes qualify and raises the reference, so the rule finds fewer headings;
/// a higher one falls back to the most common size more often, which is how `type-size-v1`
/// fabricated them. Refusing is the direction to err in, so the share sits low.
pub(crate) const BODY_SHARE_DEN: u64 = 20;

/// …and runs on at least this many lines.
///
/// Ten. A title, a heading block or a single display line never does in the measured documents —
/// the most lines any size above the body runs on there is eight — and the prose sizes that must
/// qualify run on 491 lines (`nist-sp-800-218`) to 3,032 (`nist-sp-800-53r5`). A short page whose
/// body runs on fewer lines than this has no common size, and its reference is the most common
/// size, which on a short page is its body.
pub(crate) const BODY_MIN_LINES: u64 = 10;

/// Rendered ems are binned to this, in centipoints, before the mode is taken.
///
/// 10 — a tenth of a point, the bin the leading cut uses for its gaps and for the same reason:
/// without it a document's body type at 1000, 999 and 1001 centipoints is three modes. Declared
/// here rather than imported, because the module holding the other one is named after the field
/// this file's guard forbids it to name.
pub(crate) const EM_BIN: i64 = 10;

/// One run, as the rule reads it: its type, and the facts that exclude its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Typed {
    /// The rendered em in centipoints, or `None` where the text rendering matrix had no vertical
    /// scale to measure.
    pub em: Option<i64>,
    /// Unicode scalars in the run's text — the weight the body em is measured by.
    pub chars: u64,
    /// The run's text is whitespace only.
    pub blank: bool,
    /// The page marked this run `/Artifact` (§14.8.2.2): furniture, by the author's own word.
    pub artifact: bool,
    /// A table the detector accepted owns this run.
    pub table_owned: bool,
}

/// The document's rendered ems, binned: the characters set at each size, and the lines whose
/// dominant size each is — from which the body em is read.
///
/// Built a page at a time and merged, because pages are extracted in parallel and folded in
/// order. Merging is addition, so the fold order cannot change the answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EmTally {
    /// Characters per size bin, over non-blank, non-artifact runs.
    chars: BTreeMap<i64, u64>,
    /// Lines per size bin, each line counted once at the size most of its characters are set in.
    lines: BTreeMap<i64, u64>,
}

impl EmTally {
    /// Count one run toward the body em, **if it is body text at all**: a whitespace run carries
    /// no type a reader sees, and an `/Artifact` is furniture the author set apart from the text.
    pub(crate) fn add(&mut self, run: &Typed) {
        if run.blank || run.artifact {
            return;
        }
        if let Some(em) = run.em {
            *self.chars.entry(bin(em)).or_insert(0) += run.chars;
        }
    }

    /// Count one line at the size most of its body characters are set in. A line with no
    /// measurable body text counts nowhere, and ties go to the smaller size.
    pub(crate) fn add_line(&mut self, runs: &[Typed]) {
        let mut per: BTreeMap<i64, u64> = BTreeMap::new();
        for run in runs.iter().filter(|run| !run.blank && !run.artifact) {
            if let Some(em) = run.em {
                *per.entry(bin(em)).or_insert(0) += run.chars;
            }
        }
        if let Some(best) = per.values().copied().max() {
            if let Some((em, _)) = per.iter().find(|(_, chars)| **chars == best) {
                *self.lines.entry(*em).or_insert(0) += 1;
            }
        }
    }

    /// Add another page's tally to this one.
    pub(crate) fn merge(&mut self, other: &EmTally) {
        for (em, chars) in &other.chars {
            *self.chars.entry(*em).or_insert(0) += chars;
        }
        for (em, lines) in &other.lines {
            *self.lines.entry(*em).or_insert(0) += lines;
        }
    }

    /// The body em, in centipoints: **the larger of the most common size and the largest common
    /// size** — common meaning at least 1/[`BODY_SHARE_DEN`] of the body characters on at least
    /// [`BODY_MIN_LINES`] lines. `None` on a document with no measurable body text, which is an
    /// answer: the rule then fires nowhere.
    ///
    /// **Never below the most common size, so the repair can only remove headings.** The most
    /// common size is what `type-size-v1` used; taking the larger of the two means `-v2`'s cut is
    /// never lower than `-v1`'s on any document, so `-v2` fires on a subset of the lines `-v1` fired
    /// on — a repair that cannot fabricate a heading `-v1` did not. Without the `max`, a document
    /// whose most common size ran on fewer than ten long lines while a smaller size was common
    /// would get a *lower* reference and more headings.
    ///
    /// **Ties for the most common size go to the smaller**, so the mode of a tie is stable and
    /// does not depend on iteration order; and the smaller of two equally common sizes is the
    /// more conservative reference for a rule that fires on type *larger* than it.
    pub(crate) fn body_em(&self) -> Option<i64> {
        let best = self.chars.values().copied().max()?;
        let mode = self
            .chars
            .iter()
            .find(|(_, chars)| **chars == best)
            .map(|(em, _)| *em)?;
        let total: u64 = self.chars.values().sum();
        let largest_common = self
            .chars
            .iter()
            .rev()
            .find(|(em, chars)| {
                **chars * BODY_SHARE_DEN >= total
                    && self.lines.get(em).copied().unwrap_or(0) >= BODY_MIN_LINES
            })
            .map(|(em, _)| *em);
        Some(largest_common.map_or(mode, |common| common.max(mode)))
    }
}

/// Round half up to [`EM_BIN`], deterministically and in integers.
fn bin(em: i64) -> i64 {
    (em + EM_BIN / 2) / EM_BIN * EM_BIN
}

/// One line, reduced to exactly what the verdict needs.
///
/// **Reduced, because the verdict waits for the whole document**: the body em is a mode over every
/// page, and pages are extracted in parallel, so each page reduces its lines as it goes and the
/// fold decides them once the reference exists. Holding every run's type until then would cost
/// the document's run count in memory to answer a question three facts per line answer exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Line {
    /// The smallest measurable em on the line. Every measurable run clears the cut exactly when
    /// this one does, so it is clause 1 and clause 2 in one value: `None` is a line with nothing
    /// measurable.
    min_em: Option<i64>,
    /// Some run of it is not whitespace (clause 3).
    has_text: bool,
    /// An `/Artifact`, or a table's (clauses 4 and 5).
    excluded: bool,
}

impl Line {
    /// Reduce one line's runs. `runs` is every run of one line.
    pub(crate) fn of(runs: &[Typed]) -> Line {
        Line {
            min_em: runs.iter().filter_map(|run| run.em).min(),
            has_text: runs.iter().any(|run| !run.blank),
            excluded: runs.iter().any(|run| run.artifact || run.table_owned),
        }
    }

    /// Whether this line is an inferred heading, against the document's `body_em`.
    ///
    /// The clauses are §3.4's, in its order:
    ///
    /// 1. every run with a measurable em satisfies `5 · em ≥ 6 · body_em`;
    /// 2. at least one run has a measurable em;
    /// 3. the line's text is not whitespace only;
    /// 4. the page did not mark the line an `/Artifact`;
    /// 5. no run of it is a table's.
    ///
    /// The sixth — the document declares no structure — is the caller's, because it is a fact
    /// about the document and this sees one line.
    ///
    /// **The whole line, never its tallest run** (§3.3): a drop cap or a single display glyph set
    /// in body text makes a line's maximum large and leaves its minimum at body size, and a heading
    /// is a line whose type is large throughout. That is why the reduction keeps the minimum.
    pub(crate) fn is_heading(self, body_em: i64) -> bool {
        match self.min_em {
            Some(em) if self.is_candidate() => HEADING_CUT_DEN * em >= HEADING_CUT_NUM * body_em,
            _ => false,
        }
    }

    /// Whether this line could be a heading against *any* body em: clauses 2 to 5, which do not
    /// depend on the reference. The fold keeps only these, so it holds one entry per display
    /// line rather than one per line of the document.
    pub(crate) fn is_candidate(self) -> bool {
        self.min_em.is_some() && self.has_text && !self.excluded
    }
}

/// [`Line::of`] then [`Line::is_heading`], for a line whose runs are at hand.
#[cfg(test)]
fn is_heading(line: &[Typed], body_em: i64) -> bool {
    Line::of(line).is_heading(body_em)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(em: Option<i64>, text: &str) -> Typed {
        Typed {
            em,
            chars: text.chars().count() as u64,
            blank: text.trim().is_empty(),
            artifact: false,
            table_owned: false,
        }
    }

    // ---------------------------------------------------------------------------------------
    // The body em
    // ---------------------------------------------------------------------------------------

    /// **Where no size is common, the fallback is the mode by characters, not by lines.** Twelve
    /// short title runs at 24pt and one paragraph at 10pt, with no lines counted: counted by line
    /// the title would be the body; counted by character the prose is.
    #[test]
    fn the_body_em_is_weighted_by_characters_not_by_lines() {
        let mut tally = EmTally::default();
        for _ in 0..12 {
            tally.add(&run(Some(2400), "Title"));
        }
        tally.add(&run(Some(1000), &"x".repeat(400)));
        assert_eq!(tally.body_em(), Some(1000));
    }

    /// Whitespace and furniture are not body text, however much of them a page draws.
    #[test]
    fn whitespace_and_artifacts_are_not_counted_toward_the_body() {
        let mut tally = EmTally::default();
        tally.add(&run(Some(1000), "body text here"));
        tally.add(&run(Some(700), &" ".repeat(500)));
        let mut furniture = run(Some(800), &"x".repeat(500));
        furniture.artifact = true;
        tally.add(&furniture);
        assert_eq!(tally.body_em(), Some(1000));
    }

    /// Binned to a tenth of a point, so 999, 1000 and 1003 are one body size, not three.
    #[test]
    fn nearby_ems_are_one_body_size() {
        let mut tally = EmTally::default();
        tally.add(&run(Some(999), "aaaa"));
        tally.add(&run(Some(1003), "bbbb"));
        tally.add(&run(Some(1200), "cccccc"));
        assert_eq!(
            tally.body_em(),
            Some(1000),
            "8 characters at ~10pt beat 6 at 12pt"
        );
    }

    /// A tie goes to the smaller bin, the conservative reference for a rule that fires above it.
    #[test]
    fn a_tie_goes_to_the_smaller_size() {
        let mut tally = EmTally::default();
        tally.add(&run(Some(1200), "abcd"));
        tally.add(&run(Some(1000), "wxyz"));
        assert_eq!(tally.body_em(), Some(1000));
    }

    /// Merging pages is addition, so the fold's order cannot move the answer.
    #[test]
    fn merging_page_tallies_is_order_independent() {
        let mut a = EmTally::default();
        a.add(&run(Some(1000), "one two three"));
        let mut b = EmTally::default();
        b.add(&run(Some(1400), "four five six seven eight"));
        let mut ab = a.clone();
        ab.merge(&b);
        let mut ba = b.clone();
        ba.merge(&a);
        assert_eq!(ab, ba);
        assert_eq!(ab.body_em(), Some(1400));
    }

    /// No measurable body text is an answer: no reference, so the rule fires nowhere.
    #[test]
    fn a_document_with_no_measurable_text_has_no_body_em() {
        let mut tally = EmTally::default();
        tally.add(&run(None, "unmeasured"));
        tally.add(&run(Some(1000), "   "));
        assert_eq!(tally.body_em(), None);
    }

    /// `n` lines, each one run of `chars` characters at `em`, counted as the reader counts them.
    fn lines_of(tally: &mut EmTally, n: u64, em: i64, chars: usize) {
        for _ in 0..n {
            let line = [run(Some(em), &"x".repeat(chars))];
            tally.add(&line[0]);
            tally.add_line(&line);
        }
    }

    /// **Dense small type is common; it is not the body** — `nist-sp-800-53Ar5`'s shape, which
    /// `type-size-v1` read as the body and so called ordinary prose a heading on every line. 75% of
    /// the characters at 8.5pt, the prose at 11pt on thousands of lines: the prose is the body.
    #[test]
    fn dense_small_type_is_common_but_the_prose_is_the_body() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 2000, 850, 70);
        lines_of(&mut tally, 250, 1100, 75);
        assert_eq!(tally.body_em(), Some(1100));
    }

    /// **The larger of two common sizes is the body** — `nist-sp-800-218`'s 9pt and 12pt.
    #[test]
    fn the_larger_of_two_common_sizes_is_the_body() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 1265, 900, 45);
        lines_of(&mut tally, 491, 1200, 72);
        assert_eq!(tally.body_em(), Some(1200));
    }

    /// **A short page's title never becomes the body.** One 24pt line over five short 12pt ones is
    /// 9.5% of the page's characters — a share alone would make it the reference and the page
    /// would have no heading. It runs on one line, so it is not common; neither is the body, on
    /// five, so the reference falls back to the most common size, which on a short page is its body.
    #[test]
    fn a_short_pages_title_never_becomes_the_body() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 5, 1200, 22);
        lines_of(&mut tally, 1, 2400, 12);
        assert_eq!(tally.body_em(), Some(1200));
    }

    /// A size above the body on fewer than ten lines is headings, whatever its share.
    #[test]
    fn a_size_on_few_lines_is_never_the_body() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 100, 1000, 60);
        lines_of(&mut tally, 9, 1400, 80);
        assert!(
            9 * 80 * BODY_SHARE_DEN as usize >= 100 * 60 + 9 * 80,
            "the test's premise: the large size holds more than a twentieth of the characters"
        );
        assert_eq!(tally.body_em(), Some(1000));
    }

    /// A size on many lines that holds less than a twentieth of the characters is not common.
    #[test]
    fn a_size_under_a_twentieth_of_the_characters_is_not_common() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 400, 1000, 70);
        lines_of(&mut tally, 40, 1300, 20);
        assert_eq!(tally.body_em(), Some(1000));
    }

    /// **The reference is never below the most common size**, so `-v2` can only remove headings
    /// `-v1` found. Here the most common size runs on four enormous lines and fails the line
    /// floor, while a smaller size is common: without the `max` the reference would drop to it
    /// and more lines would fire than under `-v1`.
    #[test]
    fn the_reference_never_falls_below_the_most_common_size() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 4, 1200, 2000);
        lines_of(&mut tally, 100, 900, 40);
        assert_eq!(tally.body_em(), Some(1200));
    }

    /// Merging keeps the lines as well as the characters, so a common size split across pages is
    /// still common.
    #[test]
    fn line_counts_merge_across_pages() {
        let mut a = EmTally::default();
        lines_of(&mut a, 100, 900, 50);
        lines_of(&mut a, 6, 1200, 100);
        let mut b = EmTally::default();
        lines_of(&mut b, 6, 1200, 100);
        assert_eq!(
            a.body_em(),
            Some(900),
            "six lines alone are not common, though they hold a tenth of the characters"
        );
        a.merge(&b);
        assert_eq!(a.body_em(), Some(1200), "twelve lines across two pages are");
    }

    // ---------------------------------------------------------------------------------------
    // The line
    // ---------------------------------------------------------------------------------------

    /// The cut is where it is stated to be: 1.20× clears it, a centipoint under does not.
    #[test]
    fn the_cut_is_six_fifths_of_the_body_and_inclusive() {
        assert!(is_heading(&[run(Some(1200), "Heading")], 1000));
        assert!(!is_heading(&[run(Some(1199), "Heading")], 1000));
        assert!(is_heading(&[run(Some(2400), "Display")], 1000));
        assert!(!is_heading(&[run(Some(1000), "Body")], 1000));
    }

    /// **The whole line, not its tallest run** (§3.3). A 4× glyph opening a body line is a drop
    /// cap, and a maximum test would make the line a heading.
    #[test]
    fn a_tall_glyph_does_not_make_a_line_a_heading() {
        let line = [run(Some(4000), "T"), run(Some(1000), "he rest of the line")];
        assert!(!is_heading(&line, 1000));
    }

    /// A heading drawn as several runs is one heading: every run clears the cut.
    #[test]
    fn a_heading_drawn_in_several_runs_is_one_line() {
        let line = [
            run(Some(2400), "Display"),
            run(Some(2400), " "),
            run(Some(2400), "line"),
        ];
        assert!(is_heading(&line, 1000));
    }

    /// A run with no measurable em neither blocks nor makes the verdict, but a line of nothing
    /// but such runs has no type to read.
    #[test]
    fn an_unmeasured_run_neither_blocks_nor_makes_a_heading() {
        assert!(is_heading(
            &[run(Some(2400), "Display"), run(None, "line")],
            1000
        ));
        assert!(!is_heading(&[run(None, "Display line")], 1000));
    }

    /// Whitespace is not a heading, whatever size it is set at.
    #[test]
    fn a_whitespace_line_is_never_a_heading() {
        assert!(!is_heading(&[run(Some(3000), "   ")], 1000));
    }

    /// **An `/Artifact` line is never an inferred heading**: the page said it is furniture, and
    /// that is the document's own statement about its own content.
    #[test]
    fn an_artifact_line_is_never_an_inferred_heading() {
        let mut header = run(Some(2400), "Running header");
        header.artifact = true;
        assert!(!is_heading(&[header], 1000));
    }

    /// **A table cell is never a heading**, however large its type: the table owns the run.
    #[test]
    fn a_table_cell_is_never_one() {
        let mut cell = run(Some(2400), "Column title");
        cell.table_owned = true;
        assert!(!is_heading(&[cell], 1000));
    }

    // ---------------------------------------------------------------------------------------
    // The guard
    // ---------------------------------------------------------------------------------------

    /// This file's code, comments and the tests excluded — the words below may be *discussed*
    /// here, which is how the header explains the guard, and never *used*.
    fn rule_code() -> String {
        let src = include_str!("headings.rs");
        let end = src.find("#[cfg(test)]").unwrap_or(src.len());
        src[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// **The rule reads type and not position**, which is decision #29's rider — asserted on the
    /// source, the way `reading_order.rs` asserts it reads no font size.
    ///
    /// The two words are the reading-order cut's fields: the column band and the leading-gap
    /// index. A rule that could name them could decide *a heading is a short line above a gap*,
    /// which is reading a role off the cut and is what P14 refuses. **What a word scan cannot
    /// prove** is the same here as there: it catches a name, not an equivalent computed some other
    /// way, and the behavioural tests above are where the rule's actual reach is pinned.
    #[test]
    fn the_rule_reads_no_region_and_no_block() {
        let code = rule_code();
        assert!(
            code.contains("pub(crate) fn is_heading(self, body_em: i64)"),
            "the scan did not reach the rule, so the absences below would prove nothing"
        );
        for banned in ["region", "block"] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the heading rule's code. This rule reads type, never position \
                 (decision #29's rider), and the caller is the one place a line is formed"
            );
        }
    }
}
