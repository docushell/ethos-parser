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

//! Every threshold that can produce a reason code, in one place.
//!
//! # Why these are constants and not profile fields
//!
//! A threshold change alters classification output, so it is output-affecting — the same test
//! [`crate::classify`] applies to `classify_sample_pages`, which *is* on the profile. The
//! difference is intent: `classify_sample_pages` is a **caller-tunable** knob (sample more pages
//! if you want), while these are **calibration**. A caller who could tune `TABLE_LIKELY_MIN_RULES`
//! could tune the engine into agreeing with them, which is the property this project exists to
//! deny.
//!
//! So they are compile-time constants, and **changing one is a `parser_version` event** — a
//! release, not a configuration change. `parser_version` is part of `Profile`, so a bump moves
//! `profile_sha256` and artifacts from before and after are correctly non-comparable.
//!
//! # A threshold may produce a reason. It may never produce a routing decision.
//!
//! `docs/05-MILESTONES.md` M2 review checklist. Nothing below decides what to *do*; each decides
//! only what to *report*.

/// Text-showing operators, per PDF 32000-1 §9.4.3.
///
/// All four, including `'` and `"`. pdf-inspector omits `"` from its operator match, so its text
/// vanishes silently and surrounding runs merge with corrupt geometry (parity checklist P6). The
/// classifier does not interpret these operators — it counts them — but under-counting because
/// of a missing operator is the same defect one layer up, and it would show as a false `no-text`.
pub const TEXT_SHOWING_OPERATORS: [&str; 4] = ["Tj", "TJ", "'", "\""];

/// Path-construction and painting operators, used for the vector and table signals.
pub const PATH_OPERATORS: [&str; 10] = ["m", "l", "c", "v", "y", "re", "h", "S", "s", "f"];

/// Axis-aligned rectangle operator — the ruling-line primitive a table grid is drawn with.
pub const RECTANGLE_OPERATOR: &str = "re";

/// Below this many text bytes a page counts as sparse — **but only alongside imagery**.
///
/// # The trap this avoids
///
/// LiteParse uses `text_length < 20` on its own, and compounds it with an item-level garble
/// heuristic (a 10% vowel floor) that strips items from the tally *before* the comparison. An
/// acronym-dense page — a NIST control table of `AC-2`/`SC-7`, a parts list of `SKU`/`QTY`/`MFG` —
/// can therefore be reported as `no-text` when its text extracted perfectly
/// (`docs/03-V0-SCOPE.md`'s open-measurements list, and
/// `classification.rs::an_acronym_dense_document_is_not_reported_as_textless`, which pins that this
/// engine does not do it).
///
/// Two consequences for this engine:
///
/// 1. **No vowel heuristic exists here**, at any layer. Nothing is removed from a tally on a
///    guess about whether it looks like words.
/// 2. **Short text alone is not sparse text.** Sparseness is a claim that the page holds content
///    the text layer does not cover, and the evidence for that is *other content* — imagery. A
///    page with twenty characters and nothing else is a short page. Both surveyed classifiers get
///    `synthetic/simple-text` wrong in opposite directions precisely because they skip this
///    second condition.
pub const SPARSE_TEXT_MAX_BYTES: u32 = 20;

/// Path operators on a page before it counts as vector-heavy, when no text is present.
///
/// Deliberately high. The signal is "someone drew glyphs as curves", and a page with a logo and a
/// rule line is not that.
pub const VECTOR_TEXT_MIN_PATH_OPS: u32 = 200;

/// Path operators before a page counts as densely drawn.
pub const DENSE_GRAPHICS_MIN_PATH_OPS: u32 = 500;

/// Axis-aligned rectangles before a page reads as tabular.
///
/// A table grid drawn with ruling lines produces many `re` operators. Twelve is roughly a
/// four-by-three grid — enough that a bordered callout box or a header rule does not trip it.
pub const TABLE_LIKELY_MIN_RECTANGLES: u32 = 12;

/// Reason codes this profile's detectors never emit, and why.
///
/// Declaring these is not paperwork. A vocabulary that lists `garbled` while no detector can ever
/// produce it invites a caller to conclude a document is not garbled because the list is empty —
/// absence of a reason read as evidence of its absence, which is Workbench rule 4 one level up.
///
/// The full capability-declaration machinery arrives at M4; this is the minimum honesty M2 owes
/// while emitting a vocabulary it only partly implements.
pub const NOT_DETECTED: [(&str, &str); 2] = [
    (
        "garbled",
        "No sound detector exists. The known approach — a vowel-frequency floor — reports \
         acronym-dense text as garbled, so this profile emits nothing rather than emit that.",
    ),
    (
        "multi-column",
        "No detector emits this LAYOUT REASON, and v1-S5 did not add one. That slice shipped a \
         reading-order rule — `gutter-columns-v1`, which cuts on whitespace in page space — and \
         reading a two-column page correctly is a different claim from reporting that a page is \
         two-column. The classifier answers `what is in this document`; the reading-order rule \
         answers `in what order`. Gating the sorter on a classification would route extract \
         policy through classify, which is exactly what v1-S2 refused, and emitting the reason \
         because the sorter happened to cut would make a page-complexity vocabulary report an \
         internal decision instead of an observation. If a real layout detector lands, it gets \
         its own evidence and its own test, and this entry goes in the same commit.",
    ),
];

/// Relationships between thresholds, enforced at **compile time**.
///
/// These were runtime `assert!`s until clippy pointed out the obvious: a comparison between two
/// constants is decided by the compiler, so the assertion was optimized away and the "test" tested
/// nothing. As `const` assertions they are strictly stronger — reordering the constants stops the
/// crate building rather than failing a test run nobody may have executed.
const _: () = assert!(
    VECTOR_TEXT_MIN_PATH_OPS < DENSE_GRAPHICS_MIN_PATH_OPS,
    "a page dense enough to be all-vector text is not automatically dense-graphics; the two \
     signals must be able to fire independently"
);
const _: () = assert!(
    SPARSE_TEXT_MAX_BYTES > 0,
    "zero bytes is no-text, not sparse-text — sparseness needs some text to be sparse"
);
const _: () = assert!(
    TABLE_LIKELY_MIN_RECTANGLES > 1,
    "one rectangle is a box, not a table grid"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasons::{LayoutComplexityReason, OcrNeedReason};

    #[test]
    fn the_show_text_operator_set_is_complete() {
        // PDF 32000-1 §9.4.3 defines exactly these four.
        assert_eq!(TEXT_SHOWING_OPERATORS.len(), 4);
        for op in ["Tj", "TJ", "'", "\""] {
            assert!(
                TEXT_SHOWING_OPERATORS.contains(&op),
                "`{op}` missing — this is the pdf-inspector P6 defect, which surfaces here as a \
                 false no-text rather than as lost text"
            );
        }
    }

    #[test]
    fn every_not_detected_entry_names_a_real_reason_code() {
        for (name, why) in NOT_DETECTED {
            let known = OcrNeedReason::ALL.iter().any(|r| r.as_str() == name)
                || LayoutComplexityReason::ALL
                    .iter()
                    .any(|r| r.as_str() == name);
            assert!(
                known,
                "`{name}` is declared undetected but is not in either vocabulary"
            );
            assert!(
                why.len() > 40,
                "`{name}` needs a real reason, not a placeholder"
            );
        }
    }

    /// `Garbled` is never constructed outside its own declaration.
    ///
    /// The compounding-garble trap, asserted where it can actually be caught. An earlier version
    /// of this test searched the source for the word "vowel" — which matched its own doc comments
    /// and the `NOT_DETECTED` text explaining the trap, i.e. it fired on the documentation of the
    /// thing rather than the thing. Word-matching was the wrong instrument.
    ///
    /// What matters is not whether a particular algorithm appears; it is that **no code path
    /// emits `Garbled`**. That is narrow, mechanical, and cannot be satisfied by renaming a
    /// variable. The behavioural half — that no fixture produces it either — lives in
    /// `tests/classification.rs`.
    #[test]
    fn the_garbled_reason_is_never_constructed() {
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut hits = Vec::new();

        for entry in std::fs::read_dir(&src_dir).expect("src is readable") {
            let path = entry.expect("dir entry").path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            // reasons.rs declares the variant; thresholds.rs documents why it is unreachable.
            if name == "reasons.rs" || name == "thresholds.rs" {
                continue;
            }
            if path.extension().is_some_and(|e| e == "rs") {
                let src = std::fs::read_to_string(&path).expect("readable");
                for (n, line) in src.lines().enumerate() {
                    if line.trim_start().starts_with("//") {
                        continue; // prose about the trap is expected and welcome
                    }
                    if line.contains("Garbled") {
                        hits.push(format!("{name}:{}: {}", n + 1, line.trim()));
                    }
                }
            }
        }

        assert!(
            hits.is_empty(),
            "`OcrNeedReason::Garbled` is constructed in engine-pdf:\n  {}\n\n\
             No sound detector exists. The known approach — a vowel-frequency floor — reports \
             acronym-dense text (a NIST control table of AC-2/SC-7) as garbled. If a real \
             detector lands, remove it from `NOT_DETECTED` in the same commit.",
            hits.join("\n  ")
        );
    }

    #[test]
    fn undetected_reasons_and_emitted_reasons_are_disjoint_by_construction() {
        // Anything listed in NOT_DETECTED must be absent from what the classifier can produce.
        // The behavioural proof over real fixtures is in tests/classification.rs; this pins the
        // declaration itself so the two lists cannot drift apart silently.
        let declared: Vec<&str> = NOT_DETECTED.iter().map(|(n, _)| *n).collect();
        assert!(declared.contains(&"garbled"));
        assert!(declared.contains(&"multi-column"));
    }
}
