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

//! The table-accuracy harness (v1-S7a).
//!
//! **This module measures. It changes nothing.** No tolerance, no detector, no rule id is touched
//! from here — the whole point of building the instrument before the calibration is that a later
//! slice's change to `unruled-align-v1` can be shown to be an improvement rather than a preference.
//! v1-S1 fabricated a 662-cell table on `irs-form-1024-2025` from a detector that looked right by
//! inspection, and inspection is what this exists to replace.
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
}

impl Score {
    fn add(&mut self, o: Score) {
        self.declared += o.declared;
        self.detected += o.detected;
        self.matched += o.matched;
        self.fabricated_cells += o.fabricated_cells;
        self.emitted_cells += o.emitted_cells;
        self.cross_check_disagreements += o.cross_check_disagreements;
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

    let mut tables: Vec<LabelledTable> = crate::structure::read(doc.inner())?
        .map(|tree| {
            tree.tables
                .iter()
                .map(|t| LabelledTable {
                    page: t.page.and_then(|p| page_of.get(&p).copied()),
                    rows: t.rows,
                    columns: t.columns,
                    cells: t.cells.len() as u32,
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
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::bench_fixture;

    /// The four real documents, which are the whole point: the owned fixtures are purpose-built to
    /// exercise one detector behaviour each, and scoring against them would measure how well this
    /// engine reproduces its own test cases.
    const CORPUS: [&str; 4] = [
        "cfpb-home-loan-toolkit.pdf",
        "irs-form-1040-2025.pdf",
        "nist-sp-800-63b.pdf",
        "nist-sp-800-53r5.pdf",
    ];

    fn measure() -> (Vec<(String, Score)>, Score) {
        let profile = Profile::default();
        let mut rows = Vec::new();
        let mut total = Score::default();
        for name in CORPUS {
            let bytes = bench_fixture(name);
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            let labels = label(&doc, name).expect("labels");
            let s = score(&doc, &labels, &profile).expect("scores");
            total.add(s);
            rows.push((name.to_string(), s));
        }
        (rows, total)
    }

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
            .map(|name| {
                let bytes = bench_fixture(name);
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
            .map(|name| {
                let bytes = bench_fixture(name);
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
            "  {:<28} {:>8} {:>8} {:>8} {:>10} {:>10}",
            "document", "declared", "detected", "matched", "recall", "precision"
        );
        for (name, s) in &rows {
            println!(
                "  {:<28} {:>8} {:>8} {:>8} {:>9}‰ {:>9}",
                name,
                s.declared,
                s.detected,
                s.matched,
                s.recall_permille().map_or("-".into(), |v| v.to_string()),
                s.precision_permille()
                    .map_or("-".to_string(), |v| format!("{v}‰"))
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
            "  cells emitted {}, fabricated {}, cross-check disagreements {}\n",
            total.emitted_cells, total.fabricated_cells, total.cross_check_disagreements
        );

        // The set is non-empty, or the assertions below pass vacuously.
        assert!(total.declared > 0, "the labelled set has content");
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
        for banned in [
            "tables::detect",
            "unruled::detect",
            "page.tables",
            "extract(",
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
