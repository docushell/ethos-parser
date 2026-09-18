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
//! # The reference is the document's own
//!
//! A 14pt heading is display type in a 10pt document and body type in a 17pt one, so the cut is a
//! multiple of a number measured on the document under test: the **char-weighted mode** of the
//! rendered em over every non-blank, non-artifact run, binned to a tenth of a point (§3.2). By
//! characters and not lines, because a title page or a column of captions is many lines and few
//! characters. Per document and not per page, because a page holding only a part title would take
//! its own heading as its body and find none.

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

/// The document's rendered ems, char-weighted and binned, from which the body em is read.
///
/// Built a page at a time and merged, because pages are extracted in parallel and folded in
/// order; the mode of the merged tally is the mode of the document. Merging is addition, so the
/// fold order cannot change the answer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct EmTally(BTreeMap<i64, u64>);

impl EmTally {
    /// Count one run toward the body em, **if it is body text at all**: a whitespace run carries
    /// no type a reader sees, and an `/Artifact` is furniture the author set apart from the text.
    pub(crate) fn add(&mut self, run: &Typed) {
        if run.blank || run.artifact {
            return;
        }
        if let Some(em) = run.em {
            *self.0.entry(bin(em)).or_insert(0) += run.chars;
        }
    }

    /// Add another page's tally to this one.
    pub(crate) fn merge(&mut self, other: &EmTally) {
        for (em, chars) in &other.0 {
            *self.0.entry(*em).or_insert(0) += chars;
        }
    }

    /// The body em: the modal bin, in centipoints, or `None` on a document with no measurable
    /// body text — which is an answer, and the rule then fires nowhere.
    ///
    /// **Ties go to the smallest bin**, so the mode of a tie is stable and does not depend on
    /// iteration order; and the smaller of two equally common sizes is the more conservative
    /// reference for a rule that fires on type *larger* than it.
    pub(crate) fn body_em(&self) -> Option<i64> {
        let best = self.0.values().copied().max()?;
        self.0
            .iter()
            .find(|(_, chars)| **chars == best)
            .map(|(em, _)| *em)
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

    /// **By characters, not by lines.** Twelve short title lines at 24pt and one paragraph at
    /// 10pt: counted by line the title would be the body; counted by character the prose is.
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
