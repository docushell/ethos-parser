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
//! **Since decision #38, one bit of position, and only for the bold clause.** `type-size-v3` adds
//! a clause reading a bold line as a heading where it stands apart from its neighbours, which is
//! what tells a bold heading from bold prose (`docs/measurements/headings/README.md` §7.1). The
//! caller measures that from the leading-gap cut and hands it in as [`Line`]'s `isolated`; this
//! file still names neither field, and the size clause still reads type alone.
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
    /// The font that drew the run declares itself bold (decision #38) — `type-size-v3` reads it.
    pub bold: bool,
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
    /// Characters per size bin drawn in a bold face — [`EmTally::body_is_bold`]'s numerator.
    bold: BTreeMap<i64, u64>,
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
            if run.bold {
                *self.bold.entry(bin(em)).or_insert(0) += run.chars;
            }
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
        for (em, chars) in &other.bold {
            *self.bold.entry(*em).or_insert(0) += chars;
        }
    }

    /// Whether most characters set at `body_em` are bold: a deck or a form whose prose is bold,
    /// where weight means nothing and `type-size-v3`'s bold clause withdraws rather than reading
    /// every short line as a heading. The guard `docs/measurements/headings/README.md` §7 measured
    /// beside the font-weight clause, kept. Since `type-size-v6`, [`Levels`] asks it of the size it
    /// reads bold lines against: [`Self::regular_text_em`] where the body is bold display type.
    pub(crate) fn body_is_bold(&self, body_em: i64) -> bool {
        let all = self.chars.get(&body_em).copied().unwrap_or(0);
        let bold = self.bold.get(&body_em).copied().unwrap_or(0);
        2 * bold > all
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
        let largest_common = self
            .chars
            .keys()
            .rev()
            .find(|&&em| self.is_common(em))
            .copied();
        Some(largest_common.map_or(mode, |common| common.max(mode)))
    }

    /// The text size where the body is bold display type (`type-size-v6`): the largest common size
    /// whose characters are not mostly bold, or `None` where every common size is. A bold
    /// paragraph opening a page — an annual report's statement of purpose, set larger than the
    /// text — can be the largest common size, and is then [`Self::body_em`]; the text the labels
    /// head is set in this one.
    pub(crate) fn regular_text_em(&self) -> Option<i64> {
        self.chars
            .keys()
            .rev()
            .find(|&&em| self.is_common(em) && !self.body_is_bold(em))
            .copied()
    }

    /// Whether `em` is a common size: at least 1/[`BODY_SHARE_DEN`] of the body characters, on at
    /// least [`BODY_MIN_LINES`] lines.
    fn is_common(&self, em: i64) -> bool {
        let total: u64 = self.chars.values().sum();
        self.chars.get(&em).copied().unwrap_or(0) * BODY_SHARE_DEN >= total
            && self.lines.get(&em).copied().unwrap_or(0) >= BODY_MIN_LINES
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
    /// Every run with text is drawn in a bold face (`type-size-v3`).
    all_bold: bool,
    /// The line's characters that are not whitespace, and how many of them are letters.
    chars: u64,
    letters: u64,
    /// The line starts with a lower-case letter or ends with a full stop: a sentence, or the
    /// start or end of one, and not a label.
    sentence: bool,
    /// The one fact of position `type-size-v3` reads, measured by its caller (decision #38).
    isolated: bool,
    /// How deep the line's own section number goes: 2 for `2.1 Methods`, and 1 for a line with
    /// no number or a number of one part. [`section_depth`].
    depth: u8,
    /// The line's text, lower-cased with its whitespace folded, hashed: two lines that read the same
    /// share it, which is how the label tier tells a running label from a heading
    /// ([`LABEL_RECURS`]).
    key: u64,
}

impl Line {
    /// Reduce one line: `runs` is every run of it, `text` their text in order, and `isolated` its
    /// caller's measurement: the leading-gap cut put the line in a block of its own, so whitespace
    /// wider than its band's leading stands above it and below it.
    pub(crate) fn of(runs: &[Typed], text: &str, isolated: bool) -> Line {
        let text = text.trim();
        Line {
            min_em: runs.iter().filter_map(|run| run.em).min(),
            has_text: runs.iter().any(|run| !run.blank),
            excluded: runs.iter().any(|run| run.artifact || run.table_owned),
            all_bold: runs.iter().filter(|run| !run.blank).all(|run| run.bold),
            chars: text.chars().filter(|c| !c.is_whitespace()).count() as u64,
            letters: text.chars().filter(|c| c.is_alphabetic()).count() as u64,
            sentence: text.starts_with(char::is_lowercase) || text.ends_with('.'),
            isolated,
            depth: section_depth(text),
            key: {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                text.split_whitespace()
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
                    .hash(&mut h);
                h.finish()
            },
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

/// The depth of the section number opening `text`, or `None` where it opens with none: the parts
/// of a leading `2.1.3` or `2.1.` — digits joined by full stops — or 1 for a roman numeral closed
/// by a full stop, `IV.` or `iv.`, each followed by whitespace. The author's own numbering, read as
/// the depth it states: `type-size-v3` ranks two headings set in one size by it, so `2 Foundations`
/// stands above `2.1 Databases`, and `type-size-v4` lets a numbered bold line stand with space above
/// it alone. Digits, roman numerals and full stops only, so it reads no language.
pub(crate) fn section_number(text: &str) -> Option<u8> {
    let (number, _) = text.trim_start().split_once(char::is_whitespace)?;
    let roman = |n: &str| {
        !n.is_empty()
            && (n.chars().all(|c| "IVXLCDM".contains(c))
                || n.chars().all(|c| "ivxlcdm".contains(c)))
    };
    if number.strip_suffix('.').is_some_and(roman) {
        return Some(1);
    }
    let number = number.strip_suffix('.').unwrap_or(number);
    let parts = number.split('.');
    if number.is_empty()
        || !parts
            .clone()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some(u8::try_from(parts.count()).unwrap_or(u8::MAX))
}

/// [`section_number`]'s depth, and 1 where the text opens with no number.
fn section_depth(text: &str) -> u8 {
    section_number(text).unwrap_or(1)
}

/// Most characters a bold line may hold and still be read as a heading (`type-size-v3`): a
/// section heading is a label, and a longer bold line is a bold sentence. LiteParse's figure.
pub(crate) const BOLD_HEADING_MAX_CHARS: u64 = 80;

/// The deepest level a heading is given. Markdown and HTML both stop at six.
pub(crate) const MAX_LEVEL: u8 = 6;

/// The level of a label (`type-size-v5`): below every level a heading is ranked at, and past six,
/// so the Markdown and HTML projections write it as the bold line it is rather than with `#`.
pub(crate) const LABEL_LEVEL: u8 = MAX_LEVEL + 1;

/// A label holds at most this many characters other than whitespace: a label names what follows,
/// and a longer bold line at the head of its text is a lead-in sentence.
pub(crate) const LABEL_MAX_CHARS: u64 = 60;

/// A text this many lines of a document share is no label: a running label, a repeated column
/// name, a form's field caption — each the same words again, where a heading names one place.
pub(crate) const LABEL_RECURS: u32 = 3;

/// `type-size-v3`'s verdict: a heading's level, from the sizes the document's headings are set in
/// and the depth their own numbering states (decision #38, which gives decision #29's one level a
/// rank and its size clause a sibling).
///
/// **The size clause is `type-size-v2`'s, unchanged** — [`Line::is_heading`] — so every line `-v2`
/// read as a heading `-v3` reads as one, and only its level is new.
///
/// **The bold clause is new**, and it is the clause `docs/measurements/headings/README.md` §7
/// built and refused on 2026-09-20 with the one thing that measurement said it lacked: a signal
/// telling a bold heading from bold prose *within* a document. Bold prose runs on in its
/// paragraph; a bold heading stands apart from it. So a line clears this clause when it is not
/// already a heading by size and
///
/// 1. every run with text is bold, and the document's body is not ([`EmTally::body_is_bold`]);
/// 2. it is set at least at the body em, so bold small print is not a heading — clauses 1 and 2
///    both reading the text's em instead where the body is bold display type (`-v6`, below);
/// 3. its caller measured it as standing apart — `isolated`, the one fact of position this rule
///    reads, and the reason decision #38 amends decision #29's rider: a leading-gap block of its
///    own, or, for a line opening with a section number, room above it (`type-size-v4`);
/// 4. it holds 2 to [`BOLD_HEADING_MAX_CHARS`] characters other than whitespace, at least half of
///    them letters, so a row of bold numbers is not a heading;
/// 5. it is not shaped as a sentence: it neither starts with a lower-case letter nor ends with a
///    full stop.
///
/// **The rank** orders every heading of the document by the size it is set in, largest first,
/// with a bold heading below every size; then, within one size, by the depth its section number
/// states ([`section_depth`]). Each distinct place is a level, from 1 down to [`MAX_LEVEL`], and
/// anything deeper shares the last.
///
/// **The label tier is `type-size-v5`'s** (decision #47): a line the bold clause would read but for
/// clause 3 — a bold line at the head of its text that does not stand apart, `Contact person:` or
/// `Loan terms` above the lines it names — of at most [`LABEL_MAX_CHARS`] characters and whose
/// text no [`LABEL_RECURS`] lines of the document share, is a label, at [`LABEL_LEVEL`], below
/// every ranked level. The projections write it as a bold line, never with `#`. Measured against
/// the author's tags it is mostly the section labels producers tag `/P`, and the owner shipped it
/// with `docs/28-HEADINGS-SCOPE.md` §7.5 amended to count it apart.
///
/// **A bold body set as display type keeps its text's labels** (`type-size-v6`, decision #50).
/// Where the body em is a size set mostly bold — a bold statement opening a page, larger than the
/// text and running on ten lines or more — the bold clause and the label tier read against the
/// text instead: [`EmTally::regular_text_em`], the largest common size not set mostly bold. Clause
/// 1's guard asks the same size, so the clause withdraws only where no common size is set in a
/// regular weight. The size clause and the ranks still read the body em, so no line is a heading
/// by its size that `-v5` did not read as one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Levels {
    body_em: i64,
    /// The em the bold clause and its guard read against: the body em, or under `-v6` the
    /// regular text's where the body is set mostly bold.
    bold_em: i64,
    body_is_bold: bool,
    /// Every heading's place in the document, highest first: its binned size — `i64::MIN` for a
    /// bold heading — and its depth.
    places: Vec<(i64, u8)>,
    /// The text keys [`LABEL_RECURS`] or more of the document's lines share.
    recurring: std::collections::HashSet<u64>,
}

impl Levels {
    /// The ranks for one document: every candidate `line` of it, and its `tally`. `None` where the
    /// document has no measurable body text, which is an answer — the rule then fires nowhere.
    pub(crate) fn new<'a>(
        lines: impl IntoIterator<Item = &'a Line>,
        tally: &EmTally,
    ) -> Option<Levels> {
        let body_em = tally.body_em()?;
        let bold_em = if tally.body_is_bold(body_em) {
            tally.regular_text_em().unwrap_or(body_em)
        } else {
            body_em
        };
        let lines: Vec<&Line> = lines.into_iter().collect();
        let mut counts: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
        for line in &lines {
            *counts.entry(line.key).or_insert(0) += 1;
        }
        let mut levels = Levels {
            body_em,
            bold_em,
            body_is_bold: tally.body_is_bold(bold_em),
            places: Vec::new(),
            recurring: counts
                .into_iter()
                .filter(|(_, n)| *n >= LABEL_RECURS)
                .map(|(k, _)| k)
                .collect(),
        };
        let mut places: Vec<(i64, u8)> = lines
            .iter()
            .filter_map(|line| levels.place(**line))
            .collect();
        places.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        places.dedup();
        levels.places = places;
        Some(levels)
    }

    /// The body em the ranks were read against, in centipoints.
    pub(crate) fn body_em(&self) -> i64 {
        self.body_em
    }

    /// `line`'s heading level, or `None` where no clause reads it as a heading: a ranked level, or
    /// [`LABEL_LEVEL`] for a label (`type-size-v5`) — a line the bold clause would read but for
    /// standing apart, of at most [`LABEL_MAX_CHARS`] characters, whose text no
    /// [`LABEL_RECURS`] lines of the document share. A line standing apart is ranked first, so a
    /// label is always one that does not.
    pub(crate) fn of(&self, line: Line) -> Option<u8> {
        if let Some(place) = self.place(line) {
            let n = self.places.iter().position(|&p| p == place)?;
            return Some(u8::try_from(n + 1).map_or(MAX_LEVEL, |level| level.min(MAX_LEVEL)));
        }
        let label =
            self.bold(line) && line.chars <= LABEL_MAX_CHARS && !self.recurring.contains(&line.key);
        label.then_some(LABEL_LEVEL)
    }

    /// The bold clause's conditions other than standing apart: clauses 1, 2, 4 and 5.
    fn bold(&self, line: Line) -> bool {
        !self.body_is_bold
            && line.is_candidate()
            && line.all_bold
            && line.min_em.is_some_and(|em| bin(em) >= self.bold_em)
            && (2..=BOLD_HEADING_MAX_CHARS).contains(&line.chars)
            && 2 * line.letters >= line.chars
            && !line.sentence
    }

    /// Where `line` ranks, or `None` where it is no heading.
    fn place(&self, line: Line) -> Option<(i64, u8)> {
        if line.is_heading(self.body_em) {
            return Some((bin(line.min_em?), line.depth));
        }
        (self.bold(line) && line.isolated).then_some((i64::MIN, line.depth))
    }
}

/// [`Line::of`] then [`Line::is_heading`], for a line whose runs are at hand.
#[cfg(test)]
fn is_heading(line: &[Typed], body_em: i64) -> bool {
    Line::of(line, "", false).is_heading(body_em)
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
            bold: false,
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
    // The ranks and the bold clause (`type-size-v3`)
    // ---------------------------------------------------------------------------------------

    fn bold(em: i64, text: &str) -> Typed {
        Typed {
            bold: true,
            ..run(Some(em), text)
        }
    }

    /// One line of `runs`, reading `text`.
    fn line(runs: &[Typed], text: &str, isolated: bool) -> Line {
        Line::of(runs, text, isolated)
    }

    /// A document set in plain 10pt type, with `display` lines set large, ranked.
    fn levels_over(display: &[Line]) -> Levels {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 50, 1000, 60);
        Levels::new(display, &tally).expect("the body is measurable")
    }

    /// **The largest size is level 1**, the next level 2, and one size is one level wherever it
    /// stands; body type is no level at all.
    #[test]
    fn heading_sizes_rank_into_levels_largest_first() {
        let title = line(&[run(Some(2400), "Annual report")], "Annual report", false);
        let chapter = line(&[run(Some(1800), "Results")], "Results", false);
        let section = line(&[run(Some(1200), "Revenue")], "Revenue", false);
        let levels = levels_over(&[section, chapter, title]);
        assert_eq!(levels.of(title), Some(1));
        assert_eq!(levels.of(chapter), Some(2));
        assert_eq!(levels.of(section), Some(3));
        let again = line(&[run(Some(1801), "Outlook")], "Outlook", false);
        assert_eq!(levels.of(again), Some(2), "1801 bins with 1800");
        let body = line(&[run(Some(1000), "Body text")], "Body text", false);
        assert_eq!(levels.of(body), None);
    }

    /// Past six sizes every smaller one shares level 6, which is as deep as Markdown goes.
    #[test]
    fn a_seventh_size_shares_the_sixth_level() {
        let lines: Vec<Line> = (0..7)
            .map(|i| line(&[run(Some(3000 - 200 * i), "Heading")], "Heading", false))
            .collect();
        let levels = levels_over(&lines);
        let got: Vec<Option<u8>> = lines.iter().map(|l| levels.of(*l)).collect();
        assert_eq!(got, [1, 2, 3, 4, 5, 6, 6].map(Some));
    }

    /// **A bold line standing apart is a heading**, the level below the smallest heading size —
    /// and level 1 in a document that sets nothing large.
    #[test]
    fn a_bold_line_that_stands_apart_is_the_level_below_the_sizes() {
        let label = line(&[bold(1000, "Methods")], "Methods", true);
        assert_eq!(levels_over(&[label]).of(label), Some(1));
        let title = line(&[run(Some(2400), "Annual report")], "Annual report", false);
        assert_eq!(levels_over(&[title, label]).of(label), Some(2));
    }

    /// **A bold line that does not stand apart is a label** (`type-size-v5`), below every ranked
    /// level — and bold prose, a lead-in longer than a label, is no heading of either kind.
    #[test]
    fn a_bold_line_that_runs_on_is_a_label_and_bold_prose_is_none() {
        let label = line(&[bold(1000, "Methods")], "Methods", false);
        assert_eq!(levels_over(&[label]).of(label), Some(LABEL_LEVEL));
        let title = line(&[run(Some(2400), "Annual report")], "Annual report", false);
        assert_eq!(levels_over(&[title, label]).of(label), Some(7));
        // Sixty characters other than whitespace, and sixty-one.
        let sixty = "Abcdefghij ".repeat(6);
        let sixty_one = format!("{sixty}k");
        for (text, want) in [(sixty.as_str(), Some(7)), (sixty_one.as_str(), None)] {
            let l = line(&[bold(1000, text)], text, false);
            assert_eq!(levels_over(&[l]).of(l), want, "{} characters", l.chars);
        }
    }

    /// **A text three lines of the document share is no label**: a running label or a repeated
    /// caption names no place, where a heading names one. Two lines sharing it are still labels.
    #[test]
    fn a_text_three_lines_share_is_no_label() {
        let once = line(&[bold(1000, "Notes")], "Notes", false);
        let again = line(&[bold(1000, "notes ")], "notes ", false);
        assert_eq!(levels_over(&[once, again]).of(once), Some(LABEL_LEVEL));
        let levels = levels_over(&[once, again, once]);
        assert_eq!(levels.of(once), None, "lower case and spacing read alike");
        assert_eq!(levels.of(again), None);
        let other = line(&[bold(1000, "Methods")], "Methods", false);
        assert_eq!(levels.of(other), Some(LABEL_LEVEL));
    }

    /// A bold line shaped as a sentence, too long to be a label, or mostly digits is not one.
    #[test]
    fn a_bold_sentence_a_long_line_and_a_row_of_numbers_are_not_headings() {
        for text in [
            "The results are shown below.",
            "and the remaining costs",
            "47.5 14.2 93.1",
            "X",
            &"Long bold label ".repeat(7),
        ] {
            let l = line(&[bold(1000, text)], text, true);
            assert_eq!(levels_over(&[l]).of(l), None, "{text:?}");
        }
        let colon = line(
            &[bold(1000, "Reference frameworks:")],
            "Reference frameworks:",
            true,
        );
        assert_eq!(
            levels_over(&[colon]).of(colon),
            Some(1),
            "a colon ends a label, not a sentence"
        );
    }

    /// Bold small print — a footnote's label, a figure's credit — is under the body em.
    #[test]
    fn bold_small_print_is_not_a_heading() {
        let credit = line(&[bold(800, "Source")], "Source", true);
        assert_eq!(levels_over(&[credit]).of(credit), None);
    }

    /// A line mixing bold and regular runs is a bold lead-in, not a bold line.
    #[test]
    fn a_line_mixing_bold_and_regular_runs_is_not_bold() {
        let lead = line(
            &[bold(1000, "Note:"), run(Some(1000), " see below")],
            "Note: see below",
            true,
        );
        assert_eq!(levels_over(&[lead]).of(lead), None);
        let spaced = [bold(1000, "Key"), run(Some(1000), " "), bold(1000, "terms")];
        let spaced = line(&spaced, "Key terms", true);
        assert_eq!(
            levels_over(&[spaced]).of(spaced),
            Some(1),
            "a plain space between bold words is no regular text"
        );
    }

    /// **Where the body is bold, the bold clause withdraws**: weight means nothing there.
    #[test]
    fn where_the_body_is_bold_the_bold_clause_withdraws() {
        let mut tally = EmTally::default();
        for _ in 0..50 {
            let body = [bold(1000, &"x".repeat(60))];
            tally.add(&body[0]);
            tally.add_line(&body);
        }
        let label = line(&[bold(1000, "Methods")], "Methods", true);
        let levels = Levels::new(&[label], &tally).expect("the body is measurable");
        assert_eq!(levels.of(label), None);
    }

    /// **A bold body set as display type keeps its text's labels** (`type-size-v6`): twelve lines
    /// of a bold statement at 12pt over fifty of plain 10pt text make 12pt the body, set mostly
    /// bold. Read against the text, a bold 10pt line at the head of its text is still a label and
    /// one standing apart a heading; a plain 12pt line is still no heading by its size.
    #[test]
    fn a_bold_body_set_as_display_type_keeps_its_text_s_labels() {
        let mut tally = EmTally::default();
        lines_of(&mut tally, 50, 1000, 60);
        for _ in 0..12 {
            let statement = [bold(1200, &"x".repeat(60))];
            tally.add(&statement[0]);
            tally.add_line(&statement);
        }
        assert_eq!(
            tally.body_em(),
            Some(1200),
            "the statement is the largest common size"
        );
        let label = line(&[bold(1000, "Reporting Period")], "Reporting Period", false);
        let heading = line(&[bold(1000, "Restatements")], "Restatements", true);
        let display = line(&[run(Some(1200), "Our purpose")], "Our purpose", false);
        let levels =
            Levels::new(&[label, heading, display], &tally).expect("the body is measurable");
        assert_eq!(levels.of(label), Some(LABEL_LEVEL));
        assert_eq!(levels.of(heading), Some(1));
        assert_eq!(
            levels.of(display),
            None,
            "the size clause still reads the body em"
        );
    }

    /// **…and the clause still withdraws where no common size is set in a regular weight**: a bold
    /// deck over five lines of plain small print — a footer, too few lines to be text.
    #[test]
    fn a_bold_deck_over_a_plain_footer_still_withdraws() {
        let mut tally = EmTally::default();
        for _ in 0..50 {
            let body = [bold(1000, &"x".repeat(60))];
            tally.add(&body[0]);
            tally.add_line(&body);
        }
        lines_of(&mut tally, 5, 700, 60);
        let label = line(&[bold(1000, "Methods")], "Methods", true);
        let levels = Levels::new(&[label], &tally).expect("the body is measurable");
        assert_eq!(levels.of(label), None);
    }

    /// **The depth a section number states**: its parts, digits joined by full stops and followed
    /// by whitespace. Anything else is depth 1, an unnumbered heading's.
    #[test]
    fn a_section_number_states_its_depth() {
        for (text, depth) in [
            ("2 Foundations", 1),
            ("2.1 Databases", 2),
            ("2.1.3. Scope", 3),
            ("Methods", 1),
            ("2.1", 1),
            ("A.1 Annex", 1),
            (".1 Stray", 1),
            ("1..2 Broken", 1),
            ("3.5 million readers", 2),
        ] {
            assert_eq!(section_depth(text), depth, "{text:?}");
        }
    }

    /// **A section number is digits joined by full stops, or a roman numeral closed by one**, then
    /// whitespace; anything else is no number, and a number is never read inside a word.
    #[test]
    fn a_section_number_is_digits_or_a_closed_roman_numeral() {
        for (text, depth) in [
            ("3.1. Status", Some(2)),
            ("IV. Results", Some(1)),
            ("iv. results", Some(1)),
            ("7 Theory", Some(1)),
        ] {
            assert_eq!(section_number(text), depth, "{text:?}");
        }
        for text in ["Introduction", "I am", "IV Results", "Mix. word", "3.5", ""] {
            assert_eq!(section_number(text), None, "{text:?}");
        }
    }

    /// **Within one size, the numbering ranks**: `2` above `2.1` above `2.1.1`, all below a
    /// larger size and all above a bold heading.
    #[test]
    fn numbering_ranks_headings_set_in_one_size() {
        let title = line(&[run(Some(2400), "Report")], "Report", false);
        let two = line(&[run(Some(1400), "2 Foundations")], "2 Foundations", false);
        let two_one = line(&[run(Some(1400), "2.1 Databases")], "2.1 Databases", false);
        let deeper = line(&[run(Some(1400), "2.1.1 Keys")], "2.1.1 Keys", false);
        let methods = line(&[run(Some(1400), "Methods")], "Methods", false);
        let label = line(&[bold(1000, "Notes")], "Notes", true);
        let levels = levels_over(&[deeper, two_one, two, title, methods, label]);
        assert_eq!(levels.of(title), Some(1));
        assert_eq!(levels.of(two), Some(2));
        assert_eq!(
            levels.of(methods),
            Some(2),
            "unnumbered is depth 1, as `2` is"
        );
        assert_eq!(levels.of(two_one), Some(3));
        assert_eq!(levels.of(deeper), Some(4));
        assert_eq!(levels.of(label), Some(5));
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
    /// source, the way `reading_order.rs` asserts it reads no font size. Decision #38 lets the
    /// bold clause read one bit its caller measured, `isolated`; the fields stay out of this file.
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
