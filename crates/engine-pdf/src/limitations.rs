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

//! The limitations a PDF profile declares (`docs/01-CONTRACT.md` §7).
//!
//! `engine-core` derives one limitation per `false` capability, which covers everything a
//! caller can read off the capability set. This module owns the rest: the gaps that exist
//! because of **this backend, this vendored data, and this format** — none of which
//! `engine-core` is allowed to know about (`docs/04-ARCHITECTURE.md` §1).
//!
//! # These replace M2's `not_detected` and M3's `not_decoded`
//!
//! Both were stand-ins, and both said so in their own doc comments. Two ad-hoc lists beside a
//! capability block is two vocabularies a consumer has to learn and reconcile, and the second
//! one is always the one nobody reads. There is now one list, one shape, and one place to look:
//! `assurance.limitations`.
//!
//! # A limitation code is a promise
//!
//! Callers match on these strings, so they are kebab-case, stable, and renamed only as a schema
//! change. The prose attached to each is stable enough to assert on for the same reason: a
//! declaration nobody can match against is barely better than no declaration.

use engine_core::Limitation;

use crate::thresholds::NOT_DETECTED;

/// Classification samples a bounded number of pages and stops.
pub const CLASSIFY_SAMPLE_BOUND: &str = "classify-sample-bound";

/// The backend refuses xref entries that are not exactly 20 bytes.
pub const BACKEND_XREF_STRICT_20_BYTE: &str = "backend-xref-strict-20-byte";

/// The Adobe predefined CJK CMaps are not vendored, so a document naming one is refused.
pub const PREDEFINED_CMAPS_NOT_VENDORED: &str = "predefined-cmaps-not-vendored";

/// Text drawn inside a form XObject is not descended into.
pub const FORM_XOBJECT_TEXT_NOT_DESCENDED: &str = "form-xobject-text-not-descended";

/// A font supplied no usable widths, so advances are absent rather than guessed.
pub const FONT_WIDTHS_ABSENT: &str = "font-widths-absent";

/// The code declaring that a reason in the classification vocabulary is never emitted.
///
/// Derived from the reason's own wire spelling rather than hand-written, so the declaration and
/// the vocabulary cannot drift into disagreeing about what a reason is called.
pub fn undetected_reason_code(reason: &str) -> String {
    format!("{reason}-reason-not-detected")
}

/// Limitations true of **any** run under a PDF profile, whatever the document holds.
///
/// Always declared, on both artifacts. A backend that refuses one document in twenty-six is not
/// a property of the twenty-five it accepted, and a reader of a successful artifact still needs
/// to know which documents this engine would have turned away.
pub fn backend_limitations() -> Vec<Limitation> {
    vec![
        Limitation::profile(
            BACKEND_XREF_STRICT_20_BYTE,
            "This backend requires the exactly-20-byte cross-reference entries PDF 32000-1 \
             §7.5.4 mandates and refuses 19-byte ones (`0000000015 00000 n\\n`, missing the \
             trailing space). Roughly one valid document in twenty-six on the Ethos fixture \
             corpus is affected — `synthetic/table-regular-grid` is the known case. Such a \
             document exits 2 with a named malformed-structure error and produces no artifact \
             at all; it is never repaired, and never partially read. PDFium repairs it, so a \
             document this engine refuses may well be readable elsewhere. Repair-or-refuse is a \
             v0.1 decision, not a v0 improvisation.",
        ),
        Limitation::profile(
            PREDEFINED_CMAPS_NOT_VENDORED,
            "The Adobe predefined CJK CMaps are not vendored. A document whose font names one \
             is refused with a named error rather than decoded approximately, because a \
             substituted character is a character the document does not contain. If those files \
             land, `cmap_data_version` changes and artifacts from before and after become \
             correctly non-comparable.",
        ),
    ]
}

/// Limitations specific to classification.
///
/// Absorbs M2's `not_detected` list. The honesty it encoded is unchanged and now travels in the
/// same shape as everything else: a vocabulary that lists `garbled` while no detector can ever
/// produce it invites a caller to read an empty reason list as evidence the document is not
/// garbled — absence of a reason mistaken for evidence of its absence.
pub fn classify_limitations(sample_pages: u32) -> Vec<Limitation> {
    let mut out = backend_limitations();

    out.push(Limitation::profile(
        CLASSIFY_SAMPLE_BOUND,
        format!(
            "Classification samples at most {sample_pages} page(s) and stops; cost must not \
             scale with total page count. Pages past the bound are reported `not_attempted` — \
             they were never observed, which is not the same as observed and empty. A caller \
             needing every page must raise the sample count, which changes `profile_sha256`."
        ),
    ));

    for (reason, why) in NOT_DETECTED {
        out.push(Limitation::profile(
            &undetected_reason_code(reason),
            format!(
                "The `{reason}` reason code is in this profile's vocabulary and is never \
                 emitted by it. {why} An empty reason list is therefore not evidence that \
                 `{reason}` does not apply to this document."
            ),
        ));
    }

    out
}

/// Limitations specific to extraction.
///
/// Absorbs M3's `not_decoded` list. `font-widths-absent` is document-scoped and appended by the
/// extractor when it actually occurs; the rest are true of every run.
pub fn extract_limitations() -> Vec<Limitation> {
    let mut out = backend_limitations();

    out.push(Limitation::profile(
        FORM_XOBJECT_TEXT_NOT_DESCENDED,
        "Text drawn inside a form XObject (via `Do`) is not descended into. The operator is \
         acknowledged rather than skipped silently, but a document relying on form XObjects for \
         its text will under-report runs. Declared here so a short run list is read as a \
         declared gap rather than as a sparse page.",
    ));

    out
}

/// The document-scoped limitation for a font that supplied no usable widths.
pub fn font_widths_absent(detail: &str) -> Limitation {
    Limitation::document(
        FONT_WIDTHS_ABSENT,
        format!(
            "A font on this document supplies no usable width information, so affected runs \
             report an ABSENT advance rather than a guessed one — and no ink box is measured \
             for them, since a box with no width would have to be invented. The origin is \
             unaffected: it comes from the content stream. Detail: {detail}"
        ),
    )
}

/// The document-scoped limitation for a run stopped by the configured page budget.
pub fn resource_limit_pages(budget: u32, page_count: u32) -> Limitation {
    Limitation::document(
        engine_core::codes::RESOURCE_LIMIT_PAGES,
        format!(
            "A configured page budget of {budget} stopped this run before the document's \
             {page_count} pages were exhausted. Pages past the budget are `quarantined`: they \
             exist, they were never read, and nothing may be concluded about their contents. \
             The artifact's terminal state is `partial` for exactly this reason and must not be \
             presented as a complete reading of the source."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_core::LimitationScope;

    /// Every code this module emits, in one place, so a rename is a visible event.
    const PDF_CODES: [&str; 7] = [
        CLASSIFY_SAMPLE_BOUND,
        BACKEND_XREF_STRICT_20_BYTE,
        PREDEFINED_CMAPS_NOT_VENDORED,
        FORM_XOBJECT_TEXT_NOT_DESCENDED,
        FONT_WIDTHS_ABSENT,
        "garbled-reason-not-detected",
        "multi-column-reason-not-detected",
    ];

    #[test]
    fn every_code_is_stable_kebab_case() {
        for code in PDF_CODES {
            assert!(
                !code.is_empty()
                    && code
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "`{code}` is not a stable kebab-case code callers can match on"
            );
        }
    }

    /// The derived undetected-reason codes are pinned, because they are wire spellings.
    #[test]
    fn the_undetected_reason_codes_are_pinned() {
        assert_eq!(
            undetected_reason_code("garbled"),
            "garbled-reason-not-detected"
        );
        assert_eq!(
            undetected_reason_code("multi-column"),
            "multi-column-reason-not-detected"
        );
    }

    /// `not_detected` absorbed, entry for entry — nothing M2 declared was dropped in the move.
    #[test]
    fn every_not_detected_entry_became_a_limitation() {
        let declared = classify_limitations(8);
        for (reason, why) in NOT_DETECTED {
            let code = undetected_reason_code(reason);
            let l = declared
                .iter()
                .find(|l| l.code == code)
                .unwrap_or_else(|| panic!("`{reason}` lost its declaration in the M4 migration"));
            assert!(
                l.detail.contains(why),
                "the reason M2 gave for `{reason}` must survive verbatim, not be paraphrased"
            );
            assert_eq!(l.scope, LimitationScope::Profile);
        }
    }

    #[test]
    fn every_limitation_carries_a_real_reason() {
        let mut all = classify_limitations(8);
        all.extend(extract_limitations());
        all.push(font_widths_absent("no /Widths array"));
        all.push(resource_limit_pages(1, 2));
        for l in &all {
            assert!(
                l.detail.len() > 60,
                "`{}` needs a real reason, not a placeholder: {}",
                l.code,
                l.detail
            );
        }
    }

    /// The xref limitation is declared even on a document that opened cleanly.
    #[test]
    fn the_backend_refusal_is_declared_on_every_artifact() {
        for set in [classify_limitations(8), extract_limitations()] {
            let l = set
                .iter()
                .find(|l| l.code == BACKEND_XREF_STRICT_20_BYTE)
                .expect("the backend's refusal rate is a property of every run under it");
            assert_eq!(l.scope, LimitationScope::Profile);
            assert!(
                l.detail.contains("exits 2"),
                "a caller must be told the failure mode, not just that one exists"
            );
        }
    }

    #[test]
    fn the_sample_bound_limitation_names_the_configured_count() {
        let l = classify_limitations(3)
            .into_iter()
            .find(|l| l.code == CLASSIFY_SAMPLE_BOUND)
            .expect("declared");
        assert!(l.detail.contains("at most 3 page(s)"), "{}", l.detail);
    }
}
