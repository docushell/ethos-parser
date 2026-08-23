// Copyright 2026 The ethos-engine maintainers
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

//! The table-accuracy harness (v1-S7a) and the v1 cell gate (v1-S7b).
//!
//! **This module measures. It changes nothing.** No tolerance, no detector, no rule id is touched
//! from here — the whole point of building the instrument before the calibration is that a later
//! slice's change to `unruled-align-v1` can be shown to be an improvement rather than a preference.
//! v1-S1 fabricated a 662-cell table on `irs-form-1024-2025` from a detector that looked right by
//! inspection, and inspection is what this exists to replace.
//!
//! # It earned that keep immediately
//!
//! v1-S7b was scoped to move `unruled::COLUMN_GUTTER_MIN`, on S7a's reading that the rule was
//! refusing 10.6 pt gutters against a 12 pt floor. Run through this harness, the change scores
//! **identically** — and so does disabling the floor outright, and so does disabling the row floor
//! with it. The constant was left alone and the slice reported the falsification instead. By
//! inspection it would have shipped, with a `-v2` rule id and a moved profile hash, and nothing
//! would have been better.
//!
//! # The gate
//!
//! S7a stored `{page, rows, columns, cells}`, so it could measure page-level agreement and nothing
//! finer. S7b put the cells in, with the text the structure tree binds to each one, and the gate is
//! macro-averaged cell-slot F1 over the real documents — four until v2-S19, twelve since. The
//! published method — corpus, formula, join, whitespace rule, and why the number is **not**
//! comparable to the 0.489 it is named after — is `docs/table-gate-v1.md`. Read the number off
//! `the_corpus_is_measured_and_the_numbers_are_reported`, not off any sentence here: this comment
//! said `64‰` for two slices after that stopped being true, which is the failure mode a number
//! written in prose has and a measured one does not.
//!
//! # `cross_check_disagreements` is structurally 0 since v2-S20, and that is not the check passing
//!
//! It counts tables whose `LocatorCheck` did not come back `ok`. Since `ruled-rects-v3` a grid
//! whose **structural** half disagrees is refused rather than emitted, and the other two rules
//! build a cell per face so their check cannot fail at all — so no emitted table can carry a
//! structural mismatch and this column reads 0 by construction. The number that moves instead is
//! `detected`: `nist-sp-800-218` went from nine tables to none, and the nine refusals are on the
//! artifact as `ruled-table-candidate-refused` with their pages and fault counts. Reading this
//! column as *"the detector agrees with itself"* would be reading a gap as a success.
//!
//! **The chase for 0.489 is parked** (`docs/00-NORTH-STAR.md` #10, 2026-08-19): the gate is this
//! engine on tagged PDFs this repository owns, 0.489 is a published score on somebody else's corpus,
//! and they are the same unit on a different exam. Nothing below changes — the measurement, the
//! comparator constant and the printed verdict all stay — because parking a chase is not passing
//! it, and the number is more useful written down than argued about.
//!
//! **61‰ was S7b's number**, under `ruled-rects-v2` alone. v1-S8 shipped `stroke-ruled-v1` and
//! moved it to 64‰ — the rise is `cfpb-home-loan-toolkit` alone, 246‰ to 259‰, while
//! `irs-form-1040-2025` contributes 0‰ on both sides. v2-S19 then measured twelve documents at
//! 70‰, and v2-S20 removed 11 295 false-positive cell slots without moving it at all. This
//! sentence lagged v1-S8 by two versions, which is the failure mode a number written in prose has
//! and a measured one does not:
//! `the_cell_gate_is_measured_and_its_verdict_is_recorded` asserts the VERDICT against
//! `GATE_PERMILLE` and prints the figure, so the gate cannot silently clear — but nothing makes a
//! doc comment keep up. Read the number off that test's output, not off this line.
//!
//! # Where the labels come from, and why they are not this engine's
//!
//! From the **document's own tagged structure tree**. A `/Table` element is the producer's
//! declaration that a table is there and what shape it has; `crate::structure` reads it, and a
//! guard test in that module keeps every geometric type out of it. So the ground truth is the
//! author's, the thing measured is the geometric detector, and the two derivations are independent
//! **by construction** rather than by anyone remembering to keep them apart.
//!
//! That independence is the whole reason this is measurable without hand-labelling. It is also the
//! reason the labels must never be regenerated from what the detector found: recall measured
//! against your own output is 1.0 by construction and means nothing.
//!
//! # What a tagged table is NOT
//!
//! Verified truth. It is a **claim by the producer**, and producers tag tables for layout as well
//! as for data — a two-column page furniture grid can carry `/Table` tags and nobody would want it
//! extracted as one. That inflates the denominator and makes recall read worse than it is. Every
//! label therefore records its provenance rather than presenting itself as fact, and narrowing the
//! set by sampling is a later slice's work, not something to do quietly here.

use std::collections::BTreeMap;

use engine_core::{EngineError, Profile};

use crate::document::Document;

/// Where a label came from. One value today, and it is on the wire so it cannot be forgotten.
pub const LABEL_PROVENANCE: &str = "pdf-struct-tree";

/// One document's ground truth: the tables its own structure tree declares.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelledDocument {
    /// Fixture id, as `fixtures/manifest.json` names it.
    pub id: String,
    /// Where these labels came from. Always [`LABEL_PROVENANCE`]; present so a reader never has to
    /// assume, and so a second source later is additive rather than ambiguous.
    pub provenance: String,
    /// One entry per `/Table` the tree declares, sorted by page then shape.
    pub tables: Vec<LabelledTable>,
}

/// One table, as the document's producer declared it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelledTable {
    /// 1-based page, where the tree's content items name one. `None` means the tree describes a
    /// table whose cells cite no page — real, and not something to guess a page for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Rows, from the `/TR` count.
    pub rows: u32,
    /// Columns, from the widest row once spans are counted.
    pub columns: u32,
    /// `/TD` and `/TH` elements the tree carries for it.
    pub cells: u32,
    /// The cells themselves, with their occupancy and their text (v1-S7b).
    ///
    /// v1-S7a stored only the count, which is why it could measure page-level agreement and
    /// nothing finer. The gate is a cell score, so the cells have to be here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_list: Vec<LabelledCell>,
}

/// One `/TD` or `/TH`, with the text the tree binds to it (v1-S7b).
///
/// # The text is the author's, not the detector's
///
/// It is the concatenation of the runs whose `(page, mcid)` the tree cites **beneath this cell**,
/// in the order the page drew them. The join key is the tree's own `/MCID` — never a coordinate,
/// never a box intersection — so a labelled cell's text is derived by a path that shares no input
/// with the geometric detector it is used to score. That independence is the reason this number
/// means anything; a gold text bound by "whatever runs fall inside the detector's cell box" would
/// score the detector against itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelledCell {
    /// Zero-based row.
    pub row: u32,
    /// Zero-based column.
    pub column: u32,
    /// `/RowSpan`, never 0.
    pub rowspan: u32,
    /// `/ColSpan`, never 0.
    pub colspan: u32,
    /// The cell's text, already through [`normalize`].
    ///
    /// Empty when the tree cites no marked content for the cell, or cites mcids no run claimed.
    /// Kept rather than dropped: an empty gold cell still occupies its slots, and a detector that
    /// invents text there is wrong in a way a dropped cell could not record.
    pub text: String,
}

/// What one document scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Score {
    /// Tables the structure tree declares.
    pub declared: u32,
    /// Tables the geometric detectors emitted.
    pub detected: u32,
    /// Detected tables sitting on a page where the tree also declares one.
    ///
    /// **Page-level agreement, deliberately, and it is the weakest useful join.** Matching by grid
    /// shape would fail whenever a detector and a producer disagree about a spanned header, and
    /// matching by box is impossible because the tagged half has no geometry — that is what makes
    /// it independent. Page-level over-credits the detector, and saying so is better than a
    /// stricter number nobody can reproduce.
    pub matched: u32,
    /// Emitted cells whose text is **not** a concatenation of runs the page drew.
    ///
    /// The fabrication count, and the only number here that has a required value: zero.
    pub fabricated_cells: u32,
    /// Emitted cells in total, so the fabrication rate has a denominator.
    pub emitted_cells: u32,
    /// Tables whose locator cross-check did not come back `ok`.
    pub cross_check_disagreements: u32,
    /// **Cell slots the detector got exactly right** (v1-S7b).
    ///
    /// A gold slot and a predicted slot at the same `(row, column)` of joined tables, whose text
    /// agrees after [`normalize`]. This is the numerator of the gate.
    pub cell_tp: u32,
    /// Predicted slots with no agreeing gold slot: wrong text, or a slot the gold does not have.
    pub cell_fp: u32,
    /// Gold slots with no agreeing predicted slot: a missed cell, or one whose text came out wrong.
    ///
    /// Every slot of an unjoined gold table is an FN, which is what makes a document with no
    /// detections score 0 rather than being quietly absent from the average.
    pub cell_fn: u32,
}

impl Score {
    fn add(&mut self, o: Score) {
        self.declared += o.declared;
        self.detected += o.detected;
        self.matched += o.matched;
        self.fabricated_cells += o.fabricated_cells;
        self.emitted_cells += o.emitted_cells;
        self.cross_check_disagreements += o.cross_check_disagreements;
        self.cell_tp += o.cell_tp;
        self.cell_fp += o.cell_fp;
        self.cell_fn += o.cell_fn;
    }

    /// **The gate number for one document**, in per-mille, or `None` when it declares no table.
    ///
    /// `F1 = 2·TP / (2·TP + FP + FN)`, over `CellSlot`s. Integer per-mille for the same reason
    /// recall is: two machines must agree on every digit.
    ///
    /// A document that declares tables and detects none scores `0`, not `None` — the absence of
    /// output is the result, and letting it drop out of the average is the "quietly deleting NIST"
    /// failure this harness is built to make impossible.
    pub fn cell_f1_permille(self) -> Option<u32> {
        if self.declared == 0 {
            return None;
        }
        let denom = u64::from(self.cell_tp) * 2 + u64::from(self.cell_fp) + u64::from(self.cell_fn);
        Some(if denom == 0 {
            0
        } else {
            (u64::from(self.cell_tp) * 2 * 1000 / denom) as u32
        })
    }

    /// Matched over declared, in **per-mille**, or `None` when nothing is declared.
    ///
    /// Integer per-mille rather than a float, for the reason every other number in this project is
    /// an integer: a ratio printed as `0.17241379310344829` is a number whose last fifteen digits
    /// are about IEEE-754 rather than about tables, and two machines can disagree about them.
    pub fn recall_permille(self) -> Option<u32> {
        (self.declared > 0)
            .then(|| (u64::from(self.matched) * 1000 / u64::from(self.declared)) as u32)
    }

    /// Matched over detected, in per-mille, or `None` when nothing was detected.
    pub fn precision_permille(self) -> Option<u32> {
        (self.detected > 0)
            .then(|| (u64::from(self.matched) * 1000 / u64::from(self.detected)) as u32)
    }
}

/// The whitespace rule, applied to **both** sides before any comparison.
///
/// Trim, then collapse every internal run of Unicode whitespace to a single `U+0020`. Stated here
/// and in `docs/table-gate-v1.md` because a text-equality score is only reproducible if the
/// normalization is part of the published method.
///
/// # Exact after this, and nothing looser
///
/// No case folding, no punctuation stripping, no edit distance, no judge. A cell that reads
/// `1,024` where the page drew `1.024` is **wrong**, and a metric that scored it 0.9 would be
/// grading this engine on how close it came rather than on whether it was right. The one
/// concession is whitespace, because a run split across two `Tj`s with a space between them and
/// one drawn as a single string are the same text by any reading, and the difference is an
/// artifact of how the producer chunked the stream.
///
/// # NFC first, and it is load-bearing
///
/// `cfpb-home-loan-toolkit` draws curly quotes and em dashes, so the strings compared here are not
/// ASCII and NFC cannot be waved away as the identity. Two producers can spell the same character
/// composed or decomposed, and a gate that scored those as different cells would be measuring the
/// producer's encoder rather than this engine's detector. `unicode-normalization` is a
/// **dev**-dependency for exactly this: the harness is `cfg(test)`, so nothing enters the shipped
/// library's graph.
pub fn normalize(s: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let nfc: String = s.nfc().collect();
    nfc.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Read one document's labels from its own structure tree.
///
/// # Errors
///
/// Propagates whatever opening or walking the document raises. A document with no tree yields an
/// empty label list, which is an answer rather than a failure — and a different answer from a
/// document whose tree declares no table.
pub fn label(doc: &Document, id: &str) -> Result<LabelledDocument, EngineError> {
    let page_of: BTreeMap<lopdf::ObjectId, u32> =
        doc.pages().iter().map(|&(n, oid)| (oid, n)).collect();

    // The text the pages drew, keyed the way the tree cites it. Built by interpreting content
    // streams — the same layer the detector's runs come from, and deliberately NOT the detector:
    // no rectangle, no lattice and no table is consulted, and `tables::detect` is never called.
    // The join is the tree's own `/MCID`, so which text lands in which cell is the author's
    // statement rather than a box intersection.
    let text_of = marked_text(doc)?;

    let mut tables: Vec<LabelledTable> = crate::structure::read(doc.inner())?
        .map(|tree| {
            tree.tables
                .iter()
                .map(|t| {
                    let mut cell_list: Vec<LabelledCell> = t
                        .cells
                        .iter()
                        .map(|c| LabelledCell {
                            row: c.row,
                            column: c.column,
                            rowspan: c.rowspan,
                            colspan: c.colspan,
                            text: normalize(
                                &c.mcids
                                    .iter()
                                    .filter_map(|m| {
                                        c.page
                                            .and_then(|p| text_of.get(&(p, *m)))
                                            .map(String::as_str)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            ),
                        })
                        .collect();
                    cell_list.sort();
                    LabelledTable {
                        page: t.page.and_then(|p| page_of.get(&p).copied()),
                        rows: t.rows,
                        columns: t.columns,
                        cells: t.cells.len() as u32,
                        cell_list,
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    // Sorted so the committed file is a function of the document rather than of walk order.
    tables.sort();

    Ok(LabelledDocument {
        id: id.to_string(),
        provenance: LABEL_PROVENANCE.to_string(),
        tables,
    })
}

/// Every `(page object, mcid)` the document's pages drew text under, and that text.
///
/// Runs the content interpreter per page and nothing else. Text is concatenated in the order the
/// page drew it, which is the only order the document states.
fn marked_text(doc: &Document) -> Result<BTreeMap<(lopdf::ObjectId, i64), String>, EngineError> {
    let mut out: BTreeMap<(lopdf::ObjectId, i64), String> = BTreeMap::new();
    for &(_, page_id) in doc.pages() {
        let Ok(page_dict) = doc.inner().get_dictionary(page_id) else {
            continue;
        };
        let Ok(decoded) = doc.inner().get_and_decode_page_content(page_id) else {
            continue;
        };
        let fonts = crate::fonts::load_page_fonts(doc.inner(), page_dict)?;
        let xobjects = crate::images::page_xobjects(doc.inner(), page_dict);
        let mut interp = crate::content::Interpreter::new(&fonts).with_xobjects(xobjects);
        interp.run(&decoded.operations)?;
        for shown in &interp.shown {
            // An artifact is page furniture the author excluded from the content, and a header
            // rule's text is not a table cell's.
            if shown.artifact || shown.text.is_empty() {
                continue;
            }
            if let Some(mcid) = shown.mcid {
                out.entry((page_id, mcid))
                    .or_default()
                    .push_str(&shown.text);
            }
        }
    }
    Ok(out)
}

/// Score one document against its labels.
///
/// # Errors
///
/// Propagates extraction failures. A document that cannot be read has no score, and inventing one
/// would be worse than the gap.
pub fn score(
    doc: &Document,
    labels: &LabelledDocument,
    profile: &Profile,
) -> Result<Score, EngineError> {
    let extract = crate::extract::extract(doc, profile)?;

    let declared_pages: std::collections::BTreeSet<u32> =
        labels.tables.iter().filter_map(|t| t.page).collect();

    let mut s = Score {
        declared: labels.tables.len() as u32,
        ..Score::default()
    };

    for page in &extract.pages {
        // Every run's text on this page, for the fabrication check. A cell's text must be a
        // concatenation of runs the page actually drew — that is the S1 invariant, and this is the
        // place it is MEASURED across a corpus rather than asserted on a fixture.
        for table in &page.tables {
            s.detected += 1;
            if declared_pages.contains(&page.index) {
                s.matched += 1;
            }
            if !matches!(table.check.outcome, engine_core::CheckStatus::Ok) {
                s.cross_check_disagreements += 1;
            }
            for cell in &table.cells {
                s.emitted_cells += 1;
                let from_runs: String = cell
                    .run_indices
                    .iter()
                    .filter_map(|i| page.runs.get(*i))
                    .map(|r| r.text.as_str())
                    .collect();
                if from_runs != cell.text {
                    s.fabricated_cells += 1;
                }
            }
        }
    }

    let (tp, fp, fn_) = cell_slots(&extract, labels);
    s.cell_tp = tp;
    s.cell_fp = fp;
    s.cell_fn = fn_;
    Ok(s)
}

/// A table's cells expanded to `(slot, text)`, with merged cells owning every slot they cover.
///
/// Expansion is what makes the two sides comparable at all: the tagged half declares spans and the
/// unruled half never does, so comparing cell *lists* would score a correct 2-column merge as two
/// misses. `CellSlot` is `engine_core`'s own occupancy model, used rather than re-derived.
fn expand<'a>(
    cells: impl Iterator<Item = (u32, u32, u32, u32, &'a str)>,
) -> BTreeMap<engine_core::CellSlot, String> {
    let mut out = BTreeMap::new();
    for (row, column, rowspan, colspan, text) in cells {
        // `CellSlot` is `engine_core`'s occupancy type, but the slots are enumerated here rather
        // than through `TableCellPosition::slots` — that needs a parent `NodeId`, and minting one
        // for a comparison that never reaches the wire would put an identifier nobody allocated
        // into the harness. The rule is the same one §5.4 settles: a merged cell owns every slot
        // it covers.
        for r in row..row.saturating_add(rowspan.max(1)) {
            for c in column..column.saturating_add(colspan.max(1)) {
                out.insert(engine_core::CellSlot::new(r, c), normalize(text));
            }
        }
    }
    out
}

/// True positives, false positives and false negatives over cell slots.
///
/// # The join, stated because a cell score is meaningless without it
///
/// A labelled table has **no geometry** — that is what makes it independent — so it cannot be
/// matched to a detected table by box overlap. The join is therefore:
///
/// 1. **Page.** A detected table can only join a gold table the tree places on the same page.
/// 2. **Shape, greedily.** Among the gold tables still unclaimed on that page, the one whose
///    `rows × columns` is closest to the detected table's, ties broken by the lower gold index so
///    the result does not depend on iteration order.
///
/// Everything unjoined is counted, never dropped: an unjoined gold table contributes all its slots
/// as false negatives, and an unjoined detected table all of its as false positives. A join that
/// silently discarded either side would be a score for the tables that happened to line up.
fn cell_slots(
    extract: &crate::extract::ExtractArtifact,
    labels: &LabelledDocument,
) -> (u32, u32, u32) {
    let (mut tp, mut fp, mut fn_) = (0u32, 0u32, 0u32);

    // Gold tables by page, with an index so "claimed" is recordable.
    let mut gold_by_page: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, t) in labels.tables.iter().enumerate() {
        if let Some(p) = t.page {
            gold_by_page.entry(p).or_default().push(i);
        }
    }
    let mut claimed: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();

    for page in &extract.pages {
        for table in &page.tables {
            let pred = expand(table.cells.iter().map(|c| {
                (
                    c.position.row,
                    c.position.column,
                    c.position.rowspan,
                    c.position.colspan,
                    c.text.as_str(),
                )
            }));

            // Step 2 of the join.
            let pick = gold_by_page
                .get(&page.index)
                .into_iter()
                .flatten()
                .filter(|i| !claimed.contains(*i))
                .min_by_key(|i| {
                    let g = &labels.tables[**i];
                    let d = (i64::from(g.rows) * i64::from(g.columns)
                        - i64::from(table.rows) * i64::from(table.columns))
                    .abs();
                    (d, **i)
                })
                .copied();

            let Some(gi) = pick else {
                // No gold table on this page to join. Every predicted slot is a false positive.
                fp += pred.len() as u32;
                continue;
            };
            claimed.insert(gi);
            let gold = expand(
                labels.tables[gi]
                    .cell_list
                    .iter()
                    .map(|c| (c.row, c.column, c.rowspan, c.colspan, c.text.as_str())),
            );

            for (slot, text) in &pred {
                match gold.get(slot) {
                    Some(g) if g == text => tp += 1,
                    // Both a wrong-text slot and a slot the gold does not have. Charged to both
                    // sides when the gold has one, because the detector both produced a wrong cell
                    // and failed to produce the right one.
                    Some(_) => fp += 1,
                    None => fp += 1,
                }
            }
            fn_ += gold
                .keys()
                .filter(|s| pred.get(*s).is_none_or(|p| gold[*s] != *p))
                .count() as u32;
        }
    }

    // Gold tables nothing joined: every slot missed. This is the term that keeps a document with
    // zero detections in the average at 0 instead of out of it.
    for (i, t) in labels.tables.iter().enumerate() {
        if !claimed.contains(&i) {
            fn_ += expand(
                t.cell_list
                    .iter()
                    .map(|c| (c.row, c.column, c.rowspan, c.colspan, c.text.as_str())),
            )
            .len() as u32;
        }
    }
    (tp, fp, fn_)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{bench_fixture, gate_fixture};

    /// The real documents, which are the whole point: the owned fixtures are purpose-built to
    /// exercise one detector behaviour each, and scoring against them would measure how well this
    /// engine reproduces its own test cases.
    ///
    /// **`(root, file)`, because the corpus spans two roots and did not before v2-S19.** The
    /// original four live in the Ethos `benchmark` tree, which this repository does not own and
    /// cannot write to; the documents S19 added are committed here under `gate`. That split is the
    /// reason the set sat at four for twelve slices — not labelling effort, which is zero, but
    /// nowhere to put a fifth.
    ///
    /// Admission is a written rule rather than a judgement: public, redistributable, stable at a
    /// URL, tagged, and carrying at least one `/Table`. See `docs/table-gate-v1.md`
    /// §"What qualifies a document for this corpus", which also records why the tagged personal
    /// documents on the developer machine are refused on privacy AND on representativeness.
    const CORPUS: [(&str, &str); CORPUS_LEN] = [
        ("benchmark", "cfpb-home-loan-toolkit.pdf"),
        ("benchmark", "irs-form-1040-2025.pdf"),
        ("benchmark", "nist-sp-800-63b.pdf"),
        ("benchmark", "nist-sp-800-53r5.pdf"),
        ("gate", "irs-f1040sd-2025.pdf"),
        ("gate", "irs-fw9.pdf"),
        ("gate", "nist-sp-800-161r1.pdf"),
        ("gate", "nist-sp-800-171r3.pdf"),
        ("gate", "nist-sp-800-207.pdf"),
        ("gate", "nist-sp-800-218.pdf"),
        ("gate", "nist-sp-800-37r2.pdf"),
        ("gate", "nist-sp-800-53Ar5.pdf"),
    ];

    const CORPUS_LEN: usize = 12;

    /// Read one corpus document, from whichever root declares it.
    fn corpus_bytes(root: &str, file: &str) -> Vec<u8> {
        match root {
            "benchmark" => bench_fixture(file),
            "gate" => gate_fixture(file),
            other => panic!("the gate corpus has no root `{other}`"),
        }
    }

    /// **Every document the gate scores is hash-pinned in `fixtures/manifest.json`.** No
    /// exceptions, and the absence of an exception is the point.
    ///
    /// # What this used to assert, and why it changed
    ///
    /// Until v2-S19 this test was called
    /// `the_gate_corpus_is_pinned_except_the_one_document_that_is_not`, and it pinned a **gap**:
    /// `cfpb-home-loan-toolkit.pdf` had no manifest entry at all, so the document carrying the
    /// largest single share of the 64‰ gate number was pinned by nothing and the corpus could
    /// have changed underneath the score with every test still green. The four manifest entries
    /// whose notes name it are engine-owned fixtures *derived* from it — different files.
    ///
    /// v2-S13.1 pinned that gap rather than closing it, because closing it moves the mutation
    /// harness: `fixtures/manifest.json`'s `counts` drive
    /// `crates/engine-pdf/tests/robustness.rs`, so a fourth `benchmark` entry moves the corpus off
    /// its pinned fifty-five fixtures and its pinned mutant total. It said the day the gap closed,
    /// this test would fail and bring whoever closed it back to that paragraph.
    ///
    /// **That is exactly what happened.** S19 had to grow the corpus, growing it meant touching
    /// the manifest anyway, and the mutation numbers moved in the same commit — so the gap closed
    /// as a side effect of the slice that could pay for it.
    ///
    /// The assertion is now a universal rather than a two-list comparison, because a two-list
    /// comparison of pinned-versus-not has nothing left to say once the second list is empty, and
    /// keeping it would invite someone to add a document to the wrong side of it.
    #[test]
    fn every_gate_document_is_hash_pinned() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("manifest dir has two ancestors")
            .to_path_buf();
        let m: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("fixtures/manifest.json")).expect("manifest readable"),
        )
        .expect("manifest is valid JSON");

        let fixtures = m["fixtures"].as_array().expect("fixtures");
        assert!(
            fixtures.len() >= 50,
            "the manifest lists {} fixture(s); a short read makes every check below vacuous",
            fixtures.len()
        );

        let mut unpinned = Vec::new();
        for (declared_root, doc) in CORPUS {
            let pinned = fixtures.iter().any(|f| {
                f["root"].as_str() == Some(declared_root)
                    && f["path"].as_str().is_some_and(|p| p == doc)
                    && f["sha256"]
                        .as_str()
                        .is_some_and(|h| h.starts_with("sha256:"))
            });
            if !pinned {
                unpinned.push(format!("{declared_root}/{doc}"));
            }
        }

        assert!(
            unpinned.is_empty(),
            "{} gate document(s) are scored but not hash-pinned: {unpinned:?}\n\
             A document the gate publishes a number about must be pinned by digest, or the corpus \
             can change underneath the score with every test still green. Add the manifest entry \
             — and remember its `counts` drive the mutation harness, so \
             `crates/engine-pdf/tests/robustness.rs`'s pinned totals move with it.",
            unpinned.len()
        );

        // The corpus is big enough to tell "the detector is weak" from "these few are hard".
        // Four could not, which is the finding that produced v2-S19.
        assert!(
            CORPUS.len() >= 12,
            "the gate corpus is {} document(s). Twelve is the floor v2-S19 set, and the reason is \
             stated rather than round: four documents cannot distinguish a weak detector from a \
             hard sample, and the claim that reframed five slices of work is exactly the kind a \
             small corpus produces spuriously.",
            CORPUS.len()
        );
    }

    fn measure() -> (Vec<(String, Score)>, Score) {
        let profile = Profile::default();
        let mut rows = Vec::new();
        let mut total = Score::default();
        for (root, name) in CORPUS {
            let bytes = corpus_bytes(root, name);
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            let labels = label(&doc, name).expect("labels");
            let s = score(&doc, &labels, &profile).expect("scores");
            total.add(s);
            rows.push((name.to_string(), s));
        }
        (rows, total)
    }

    /// **The gate number: macro cell-F1 in per-mille**, averaged over the documents that declare
    /// at least one table.
    ///
    /// Macro rather than micro, and that choice changes the answer here. `nist-sp-800-53r5`
    /// carries 26 of the corpus's 57 tagged tables and thousands of its cells; a micro average
    /// would let `cfpb-home-loan-toolkit`'s cells be drowned by it, or vice versa. Macro gives
    /// each document one vote, so a rule that works on one producer's output and fails on three
    /// cannot read as three-quarters right.
    ///
    /// A document declaring nothing is excluded from the average — there is no cell score to
    /// average — but a document declaring tables and detecting none scores **0** and stays in.
    fn macro_cell_f1_permille(rows: &[(String, Score)]) -> Option<u32> {
        let scored: Vec<u32> = rows
            .iter()
            .filter_map(|(_, s)| s.cell_f1_permille())
            .collect();
        (!scored.is_empty()).then(|| {
            (scored.iter().map(|v| u64::from(*v)).sum::<u64>() / scored.len() as u64) as u32
        })
    }

    /// The published ODL-local comparator, in per-mille — **not** a floor this repository is
    /// still chasing. See `docs/table-gate-v1.md` for why a number computed on this corpus is not
    /// comparable to the 0.489 it is named after, and `docs/00-NORTH-STAR.md` #10 for the owner's
    /// 2026-08-19 decision to park the chase.
    ///
    /// It stays at 489 on purpose. The verdict test below prints the measured figure against it,
    /// so a sudden jump in either direction is visible in the report rather than inferred from a
    /// diff. Parking the chase changed what the comparison *means*, not whether it is printed.
    const GATE_PERMILLE: u32 = 489;

    /// The committed labelled set, as it sits on disk.
    fn committed() -> Vec<LabelledDocument> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("repo root")
            .join("fixtures/labelled/table-truth.json");
        serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| {
            panic!(
                "the labelled set is missing at {}: {e}\nIt is committed data, not something \
                 regenerated on demand — see `the_committed_labels_still_match_the_documents`.",
                path.display()
            )
        }))
        .expect("the labelled set is valid JSON")
    }

    /// Re-derive the committed labelled set. **Run deliberately, never in CI.**
    ///
    /// `cargo test -p engine-pdf --lib -- --ignored regenerate_the_labelled_set`
    ///
    /// Ignored rather than automatic, and a test rather than a feature flag, for two reasons. A
    /// generator that runs on every build makes the ground truth follow the code under test, which
    /// is the one thing this harness must not do. And exposing it through a cargo feature would put
    /// `accuracy` on the crate's public surface — measurement machinery is not contract, and
    /// `public_api.rs` was right to fail when it briefly was.
    #[test]
    #[ignore]
    fn regenerate_the_labelled_set() {
        let profile = Profile::default();
        let fresh: Vec<LabelledDocument> = CORPUS
            .iter()
            .map(|(root, name)| {
                let bytes = corpus_bytes(root, name);
                let doc = Document::open_bytes(&bytes, &profile).expect("opens");
                label(&doc, name).expect("labels")
            })
            .collect();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("repo root")
            .join("fixtures/labelled/table-truth.json");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&fresh).expect("serializes") + "\n",
        )
        .expect("writes");
        println!("wrote {}", path.display());
    }

    /// **The labels are frozen, and this is what freezing them buys.**
    ///
    /// They were derived once from each document's own structure tree and written to disk. This
    /// re-derives them and compares. A difference means the tree-walk changed what it thinks these
    /// documents declare — which may be a fix or may be a regression, but either way it must be a
    /// reviewed edit to a committed file rather than a number that quietly moved underneath the
    /// measurement.
    #[test]
    fn the_committed_labels_still_match_the_documents() {
        let profile = Profile::default();
        let fresh: Vec<LabelledDocument> = CORPUS
            .iter()
            .map(|(root, name)| {
                let bytes = corpus_bytes(root, name);
                let doc = Document::open_bytes(&bytes, &profile).expect("opens");
                label(&doc, name).expect("labels")
            })
            .collect();
        assert_eq!(
            fresh,
            committed(),
            "the structure tree now describes different tables than the committed labelled set \
             records. Re-derive it deliberately and review the diff; do not let the ground truth \
             follow the code under test."
        );
    }

    /// Every label says where it came from, so no reader has to assume it is verified truth.
    #[test]
    fn every_label_declares_its_provenance() {
        for d in committed() {
            assert_eq!(
                d.provenance, LABEL_PROVENANCE,
                "{}'s labels must name their source: a tagged `/Table` is the PRODUCER's claim, \
                 not a verified fact, and a set that hid that would read as stronger than it is",
                d.id
            );
        }
    }

    /// **The measurement, printed so a CI log records it rather than only that nothing failed.**
    ///
    /// Run it with `--nocapture`. The numbers here are bad on purpose: v1-S7a builds the
    /// instrument and reports what it sees, and v1-S7b is where the detector changes.
    #[test]
    fn the_corpus_is_measured_and_the_numbers_are_reported() {
        let (rows, total) = measure();

        println!("\ntable accuracy, labels from each document's own structure tree:");
        println!(
            "  {:<28} {:>8} {:>8} {:>8} {:>10} {:>10} {:>7} {:>7}",
            "document", "declared", "detected", "matched", "recall", "precision", "fabr", "xcheck"
        );
        for (name, s) in &rows {
            println!(
                "  {:<28} {:>8} {:>8} {:>8} {:>9}‰ {:>9} {:>7} {:>7}",
                name,
                s.declared,
                s.detected,
                s.matched,
                s.recall_permille().map_or("-".into(), |v| v.to_string()),
                s.precision_permille()
                    .map_or("-".to_string(), |v| format!("{v}‰")),
                s.fabricated_cells,
                s.cross_check_disagreements
            );
        }
        println!(
            "  {:<28} {:>8} {:>8} {:>8} {:>9}‰ {:>9}",
            "TOTAL",
            total.declared,
            total.detected,
            total.matched,
            total
                .recall_permille()
                .map_or("-".into(), |v| v.to_string()),
            total
                .precision_permille()
                .map_or("-".to_string(), |v| format!("{v}‰"))
        );
        println!(
            "  cells emitted {}, fabricated {}, cross-check disagreements {}",
            total.emitted_cells, total.fabricated_cells, total.cross_check_disagreements
        );

        // The gate half. Printed under its own heading so nobody can read a page-level recall as
        // the cell score: 140‰ of pages agreeing is not 140‰ of cells right, and `08-V1-SCOPE.md`
        // §3 is explicit that the gate is a cell number.
        println!("\ncell-slot accuracy (the gate metric), exact text after the whitespace rule:");
        println!(
            "  {:<28} {:>8} {:>8} {:>8} {:>10}",
            "document", "TP", "FP", "FN", "cell-F1"
        );
        for (name, s) in &rows {
            println!(
                "  {:<28} {:>8} {:>8} {:>8} {:>9}",
                name,
                s.cell_tp,
                s.cell_fp,
                s.cell_fn,
                s.cell_f1_permille()
                    .map_or("-".to_string(), |v| format!("{v}‰"))
            );
        }
        let macro_f1 = macro_cell_f1_permille(&rows);
        println!(
            "  MACRO cell-F1 over the {} documents that declare a table: {}",
            rows.iter().filter(|(_, s)| s.declared > 0).count(),
            macro_f1.map_or("-".to_string(), |v| format!("{v}‰"))
        );

        // **The band, printed rather than left to a reader's impression.** v2-S19 exists because
        // four documents cannot tell a weak detector from a hard sample, and a macro average is
        // exactly the statistic that hides which it is: an average of mostly zeros is stable for a
        // reason that has nothing to do with the detector being stable. So the spread, the median
        // and the count of documents scoring nothing are printed beside it, and the sentence a
        // record is allowed to write about "holding" has to survive all four numbers.
        let mut scored: Vec<u32> = rows
            .iter()
            .filter_map(|(_, s)| s.cell_f1_permille())
            .collect();
        scored.sort_unstable();
        if let (Some(&lo), Some(&hi)) = (scored.first(), scored.last()) {
            let median = scored[scored.len() / 2];
            let zeros = scored.iter().filter(|v| **v == 0).count();
            println!(
                "  band across documents: {lo}‰ .. {hi}‰, median {median}‰, {zeros} of {} score 0‰",
                scored.len()
            );
        }
        println!(
            "  gate is > {GATE_PERMILLE}‰: {}\n",
            match macro_f1 {
                Some(v) if v > GATE_PERMILLE => "PASS",
                Some(_) => "MISS",
                None => "not assessable",
            }
        );

        // The set is non-empty, or the assertions below pass vacuously.
        assert!(total.declared > 0, "the labelled set has content");
    }

    /// **The gate, asserted rather than only printed** — with the honest consequence when it is
    /// missed.
    ///
    /// The number is computed and reported either way. What this test refuses to do is let the
    /// gate be silently absent: it fails if the metric cannot be computed at all, and it records
    /// the standing miss as an explicit expectation so that *clearing* the gate also breaks the
    /// build and forces the milestone documents to be updated in the same commit.
    ///
    /// v1-S7b measures **0‰**. That is not the gate being unassessed — it is the gate being
    /// assessed and missed, and the difference is the whole point of S7a having been built first.
    #[test]
    fn the_cell_gate_is_measured_and_its_verdict_is_recorded() {
        let (rows, _) = measure();
        let f1 = macro_cell_f1_permille(&rows)
            .expect("the corpus declares tables, so the gate is computable");
        assert!(
            f1 <= GATE_PERMILLE,
            "macro cell-F1 is {f1}‰, which CLEARS the {GATE_PERMILLE}‰ gate. That is the good \
             outcome and it must not pass silently: update `docs/table-gate-v1.md`, tick S7 in \
             `docs/09-V1-MILESTONES.md` and `docs/08-V1-SCOPE.md` §4, and invert this assertion \
             to a floor."
        );
    }

    /// **The gold negatives: pages with no geometric table must still produce none.**
    ///
    /// A cell score can always be raised by loosening the detector, and the corpus above cannot
    /// see the cost of that — a document that declares no table contributes no false positive
    /// anywhere in the macro average. These three do. `synthetic/two-columns` is the one that
    /// matters most: it is four runs in a flawless 2 × 2 whose only distinguishing evidence is
    /// that the author wrote it down the columns, so it is exactly the fixture a loosened
    /// alignment rule turns into a table first.
    ///
    /// Owned negatives, deliberately kept out of the gate average: scoring this engine against
    /// grids it built itself measures how well it reproduces its own test cases. They are a
    /// safety rail on the calibration, not a number.
    #[test]
    fn the_gold_negatives_still_have_no_geometric_table() {
        let profile = Profile::default();
        // Two corpora: `two-columns` and `simple-text` are conformance fixtures, `near-miss` is
        // engine-owned. Named per fixture rather than guessed, because `test_support` fails loudly
        // on a missing corpus and a skip here would be a negative that quietly stopped running.
        for (corpus, rel) in [
            ("conformance", "synthetic/two-columns/document.pdf"),
            ("conformance", "synthetic/simple-text/document.pdf"),
            ("engine", "unruled-near-miss/document.pdf"),
        ] {
            let bytes = match corpus {
                "engine" => crate::test_support::engine_fixture(rel),
                _ => crate::test_support::conformance_fixture(rel),
            };
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            let ex = crate::extract::extract(&doc, &profile).expect("extracts");
            let found: usize = ex.pages.iter().map(|p| p.tables.len()).sum();
            assert_eq!(
                found, 0,
                "{rel} now yields {found} geometric table(s). It is a gold negative: the page \
                 draws no grid and implies none that this engine may claim. A calibration that \
                 buys recall here has bought a fabrication."
            );
        }
    }

    /// **The published whitespace rule is the rule that ran**, on the exact shapes it claims.
    ///
    /// `docs/table-gate-v1.md` states the normalization as part of the method, so a reader can
    /// recompute the number. This pins each clause of it: NFC composes, the ends are trimmed,
    /// internal whitespace collapses to one space, and nothing else is touched — a case change or
    /// a punctuation difference still makes two cells different, because the gate is exact text
    /// and a fuzzy match would be grading the detector on how close it came.
    #[test]
    fn the_published_whitespace_rule_is_the_one_that_runs() {
        // NFC: `e` + U+0301 composes to `é`, so two producers spelling one character differently
        // do not score as two different cells.
        assert_eq!(normalize("e\u{0301}"), "é");
        assert_eq!(normalize("\u{00e9}"), "é");
        // Trim, and collapse every internal whitespace run — including the tab and newline a
        // producer's stream chunking can leave between two `Tj`s of one cell.
        assert_eq!(normalize("  Total \t due\n\n now  "), "Total due now");
        // And nothing looser than that.
        assert_ne!(normalize("Total"), normalize("total"));
        assert_ne!(normalize("1,024"), normalize("1.024"));
    }

    /// The corpus really does carry the non-ASCII text that makes NFC load-bearing.
    ///
    /// Without this, a later cleanup could drop the `unicode-normalization` dev-dependency,
    /// observe every test still green, and quietly narrow the published method.
    #[test]
    fn the_corpus_carries_non_ascii_text_so_nfc_is_not_decorative() {
        let profile = Profile::default();
        let bytes = bench_fixture("cfpb-home-loan-toolkit.pdf");
        let doc = Document::open_bytes(&bytes, &profile).expect("opens");
        assert!(
            label(&doc, "cfpb-home-loan-toolkit.pdf")
                .expect("labels")
                .tables
                .iter()
                .any(|t| t.cell_list.iter().any(|c| !c.text.is_ascii())),
            "no gold cell carries non-ASCII text any more. If that is a real corpus change the \
             NFC clause may be dropped — but it must be dropped deliberately, not by a test that \
             stopped covering it"
        );
    }

    /// **Fabrication is 0, measured across the corpus rather than asserted on a fixture.**
    ///
    /// The v1 exit criterion that is not a threshold: every emitted cell's text is a concatenation
    /// of runs the page actually drew. A cell that invented a character would fail here, and this
    /// is the first place that claim is checked over real documents instead of over the four
    /// hand-built grids the fixtures provide.
    #[test]
    fn no_emitted_cell_contains_text_the_page_did_not_draw() {
        let (_, total) = measure();
        assert_eq!(
            total.fabricated_cells, 0,
            "{} of {} emitted cells carry text that is not a concatenation of the runs they \
             name — fabrication is the one number in this harness with a required value",
            total.fabricated_cells, total.emitted_cells
        );
    }

    /// **The harness reruns to the same number**, which is the acceptance criterion that makes
    /// every other number in it worth reading.
    #[test]
    fn the_measurement_is_stable_across_runs() {
        let (first_rows, first) = measure();
        let (second_rows, second) = measure();
        assert_eq!(first, second, "the totals must not move between runs");
        assert_eq!(first_rows, second_rows, "nor must any document's row");
    }

    /// The labels are the document's, and the harness must not be able to quietly make them its own.
    ///
    /// Recall measured against the detector's own output is 1.0 by construction. This scans the
    /// module for the shape that mistake would take.
    #[test]
    fn the_labels_never_come_from_the_detector() {
        // **Scoped to `label` alone, and that scoping is the point.** An earlier draft scanned the
        // whole module and fired on `page.tables` inside `score` — which is the detector's output
        // being *measured*, exactly what this harness is for. A guard that cannot tell the
        // labelling half from the scoring half is the word-matching instrument `thresholds.rs`
        // already warns about, and it would have been silenced rather than fixed.
        let src = include_str!("accuracy.rs");
        let start = src
            .find("pub fn label(")
            .expect("the labelling function is named `label`");
        let end = src[start..]
            .find("\npub fn score(")
            .map(|i| start + i)
            .expect("`score` follows `label`");
        let code: String = src[start..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("crate::structure::read"),
            "the scan did not reach the label source, so its claim below proves nothing"
        );
        // v1-S7b bound cell TEXT into the labels, which is the change that could most easily have
        // made them the detector's. The text join runs in `marked_text`, which sits between
        // `label` and `score` and so is inside the scanned region — asserted rather than assumed,
        // because a later reorder that moved it out would silently narrow this guard to nothing.
        assert!(
            code.contains("fn marked_text(") && code.contains("interp.shown"),
            "the scan no longer covers the cell-text binding. Move `marked_text` back between \
             `label` and `score`, or widen the scan: an unscanned binding is an unguarded one"
        );
        for banned in [
            "tables::detect",
            "unruled::detect",
            "page.tables",
            "extract(",
            // The geometric shapes a text binding would reach for if it stopped using the tree's
            // own `/MCID` and started asking which runs fall inside a box.
            "interp.rects",
            "DetectedTable",
            "origin_x",
            "QRect",
        ] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the labelling half of this harness. Labels come from the \
                 document's structure tree; deriving them from what a detector found makes recall \
                 1.0 by construction and measures nothing"
            );
        }
    }
}
