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

//! Derivation classes and typed geometric absence (`docs/01-CONTRACT.md` §5.2, §6).

use serde::{Deserialize, Serialize};

use crate::geom::QRect;

/// How a node came to exist.
///
/// This is the axis that lets OCR (v4) and assist (v3) be added later without laundering
/// their output into born-digital certainty. v0 produces `Extracted` and `Computed` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivationClass {
    /// Read from the source's own encoding. Text, origins, font identity, `mcid`.
    Extracted,
    /// Derived deterministically from `Extracted` values by a versioned rule. Reading order,
    /// line grouping, ink boxes computed from font metrics.
    Computed,
    /// Produced by a recognition engine over pixels. **v4.** Runs under its own profile and
    /// may author nodes only where the deterministic reader found no text layer at all.
    Recognized,
    /// Suggested by a model. **v3.** Never citable, never evidence, never overwrites anything.
    Proposed,
}

impl DerivationClass {
    /// Whether a node of this class may be the source of a verified quote.
    ///
    /// `Proposed` is never citable — a chart description or formula guess can exist in the tree
    /// and can never ground a claim.
    pub fn is_citable(self) -> bool {
        !matches!(self, Self::Proposed)
    }

    /// Whether v0 may produce this class.
    ///
    /// Exists so a future lane cannot quietly start emitting `Recognized` under the base
    /// profile: the check is cheap and the failure it prevents is silent.
    pub fn is_producible_in_v0(self) -> bool {
        matches!(self, Self::Extracted | Self::Computed)
    }

    /// Whether a node of class `self` may be overwritten by a node of class `other`.
    ///
    /// `docs/01-CONTRACT.md` §6 states two rules, and **both** are enforced here:
    ///
    /// 1. **Nothing overwrites `Extracted`.** This is what stops the LiteParse failure where OCR
    ///    is merged into the native text stream and distinguishable only by an omittable nullable
    ///    field (checklist L24).
    /// 2. **`Proposed` never overwrites anything.** A model suggestion may sit beside evidence and
    ///    may never replace it — including replacing another suggestion, since the first one may
    ///    already have been shown to a reviewer.
    ///
    /// Only checking rule 1 would let `Proposed` overwrite `Computed`, `Recognized`, or another
    /// `Proposed` — which is how a model's output quietly becomes the record.
    ///
    /// What this function deliberately does **not** encode: the v4 constraint that `Recognized`
    /// may author only on canvases where the deterministic reader found no text layer at all.
    /// That is a property of *where* a node is placed, not of which classes may replace which, and
    /// inventing a class-pair rule for it here would be a rule the contract does not state.
    pub fn may_be_overwritten_by(self, other: Self) -> bool {
        !matches!(self, Self::Extracted) && !matches!(other, Self::Proposed)
    }
}

/// Why a measurement is absent — a **type**, never a sentinel value.
///
/// `docs/01-CONTRACT.md` §5.2 forbids `height = font_size`, zero boxes, null islands, and
/// page-sized stand-ins. A font-size-derived box is closer to invented than measured, and
/// Workbench rule 3 forbids inventing a coordinate.
///
/// TODO(re-read DocumentRepresentation v0 field list): the DocuShell companion models geometry
/// as an *optional* field and states only that "a processor that reports no uncertainty
/// produces an absent field, never an implied 1.0" (companion, "Uncertainty granularity"). It
/// never names a variant for *why* geometry is absent. These spellings are therefore
/// engine-local: they carry strictly more information than the companion's shape, and they
/// project to a plain absent field on the DocuShell wire. Pending DocuShell review — see
/// `docs/README.md` open TODOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryAbsence {
    /// The reader has no metrics for this glyph's font, so no ink box can be measured.
    ///
    /// The common case: a font program with neither usable ascent/descent nor a `/FontBBox`.
    NotReportedByReader,
    /// The node kind has no geometry by definition, so absence is correct rather than a gap.
    NotApplicableToKind,
    /// The node draws no ink, so there is no ink box to measure (v1-S6.2).
    ///
    /// **The variant this type's own documentation promised and did not have.** The paragraph
    /// below explains that `Option<QRect>` was refused because it would collapse *"we could not
    /// measure"*, *"there is nothing to measure"* and *"we were not asked to measure"* into one
    /// answer — and until v1-S6.2 the middle case had no spelling, so a run of spaces was given a
    /// rectangle and called `Measured`.
    ///
    /// A whitespace-only run is the case in practice. Its box was never ink: `Font::ink_box`
    /// stretches the *font's* ascent/descent envelope over the run's **advance**, so for a space
    /// it produced a rectangle around nothing. On `nist-sp-800-53r5` that fabrication put 3 450
    /// boxes past the edge of the page and the seal refused the document — 491 of its 492 pages
    /// unreadable, over content that draws nothing.
    ///
    /// **It is not [`Self::NotReportedByReader`], and the difference is the whole point.** That one
    /// means the reader could not measure and counts toward a declared capability limitation. This
    /// one means the reader could measure perfectly well and there was nothing there. Counting it
    /// would inflate the ink-measurement limitation by 150 425 nodes on the four real corpus
    /// documents while saying nothing true about this reader.
    NoInkToMeasure,
    /// This profile does not declare the capability that would produce the measurement.
    CapabilityNotEnabled,
    /// The node was reconstructed from the document's structure tree, which states structure and
    /// never a coordinate — so there is no box to report, and none is invented (v2-S24).
    ///
    /// The case in practice is a **tagged table**: its grid comes from `/Table`, `/TR`, `/TD`,
    /// `/RowSpan` and `/ColSpan`, and not one of those is a rectangle. The engine emits the table
    /// because the document declares it, and reports the geometry as absent because the tree the
    /// table came from names none.
    ///
    /// **It is deliberately NOT [`Self::NotReportedByReader`], and the difference is the same one
    /// v1-S6.2 drew for [`Self::NoInkToMeasure`].** `NotReportedByReader` means the reader tried to
    /// measure ink and the font supplied no metrics — a real gap in what this engine can do, which
    /// [`GeometryPresence::is_declarable_limitation`] counts. Here the reader did not try and could
    /// not: the thing it read from is a structure tree, and a tag has no geometry to measure.
    /// Counting it as a reader limitation would inflate that count with nodes whose absent geometry
    /// is a property of the source, not a shortfall of the reader — exactly the conflation that
    /// motivated splitting `NoInkToMeasure` out. So this variant is **not** a declarable
    /// limitation, and it is not groundable either: a table with no box cannot enter a
    /// `ethos.grounding.v1` projection any more than a run with no ink box can.
    NotReportedByStructureTree,
}

/// Geometry that was measured, or a typed reason it was not.
///
/// Deliberately **not** `Option<QRect>`. `None` would collapse three different situations —
/// "we could not measure", "there is nothing to measure", and "we were not asked to measure" —
/// into one, and the first is a capability limitation that must be declared and counted while
/// the second is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "state",
    content = "value",
    deny_unknown_fields
)]
pub enum GeometryPresence {
    /// A measured ink box, from the embedded font program or the font descriptor.
    Measured(QRect),
    /// No box, and the reason why.
    Absent(GeometryAbsence),
}

impl GeometryPresence {
    /// The rectangle, if one was measured.
    pub fn measured(self) -> Option<QRect> {
        match self {
            Self::Measured(r) => Some(r),
            Self::Absent(_) => None,
        }
    }

    /// Whether this absence should be counted toward a declared capability limitation.
    ///
    /// Only [`GeometryAbsence::NotReportedByReader`] counts. A node kind that never has
    /// geometry is not a gap in what the engine could do, and reporting it as one would inflate
    /// the limitation count into meaninglessness.
    pub fn is_declarable_limitation(self) -> bool {
        matches!(self, Self::Absent(GeometryAbsence::NotReportedByReader))
    }

    /// Whether this node may be emitted into a grounding artifact.
    ///
    /// `ethos.grounding.v1` requires `bbox` on every element and span. A node without measured
    /// geometry is **omitted and counted** rather than emitted with a fabricated box — the
    /// decision recorded in `docs/01-CONTRACT.md` §11.
    ///
    /// Note the type: this takes a measurement state, not a boolean or a reason code, so the
    /// omission path is unreachable from a quality judgement.
    pub fn is_groundable(self) -> bool {
        matches!(self, Self::Measured(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c14n::c14n_bytes;

    const ALL_CLASSES: [DerivationClass; 4] = [
        DerivationClass::Extracted,
        DerivationClass::Computed,
        DerivationClass::Recognized,
        DerivationClass::Proposed,
    ];

    #[test]
    fn nothing_overwrites_extracted() {
        for c in ALL_CLASSES {
            assert!(
                !DerivationClass::Extracted.may_be_overwritten_by(c),
                "{c:?} must not overwrite Extracted"
            );
        }
    }

    #[test]
    fn proposed_overwrites_nothing() {
        for target in ALL_CLASSES {
            assert!(
                !target.may_be_overwritten_by(DerivationClass::Proposed),
                "Proposed must not overwrite {target:?} — a model suggestion may sit beside \
                 evidence and never replace it"
            );
        }
    }

    /// The full 4×4 matrix, written out.
    ///
    /// The half-implemented version of this rule passed `nothing_overwrites_extracted` while
    /// letting `Proposed` overwrite `Computed`, `Recognized` and `Proposed`. A matrix is the only
    /// form of this test that cannot be satisfied by checking one argument and ignoring the other.
    #[test]
    fn the_overwrite_matrix_is_exhaustive_and_pinned() {
        use DerivationClass::{Computed, Extracted, Proposed, Recognized};

        // (target, overwriter, allowed)
        let expected = [
            (Extracted, Extracted, false),
            (Extracted, Computed, false),
            (Extracted, Recognized, false),
            (Extracted, Proposed, false),
            (Computed, Extracted, true),
            (Computed, Computed, true),
            (Computed, Recognized, true),
            (Computed, Proposed, false),
            (Recognized, Extracted, true),
            (Recognized, Computed, true),
            (Recognized, Recognized, true),
            (Recognized, Proposed, false),
            (Proposed, Extracted, true),
            (Proposed, Computed, true),
            (Proposed, Recognized, true),
            (Proposed, Proposed, false),
        ];

        assert_eq!(
            expected.len(),
            ALL_CLASSES.len() * ALL_CLASSES.len(),
            "the matrix must cover every ordered pair"
        );

        for (target, overwriter, allowed) in expected {
            assert_eq!(
                target.may_be_overwritten_by(overwriter),
                allowed,
                "{target:?}.may_be_overwritten_by({overwriter:?}) should be {allowed}"
            );
        }
    }

    #[test]
    fn the_overwrite_rule_reads_both_arguments() {
        // A guard against regressing to `!matches!(self, Extracted)`: holding the target fixed at
        // a non-Extracted class, the answer must still depend on the overwriter.
        let a = DerivationClass::Computed.may_be_overwritten_by(DerivationClass::Recognized);
        let b = DerivationClass::Computed.may_be_overwritten_by(DerivationClass::Proposed);
        assert_ne!(a, b, "the overwriter argument must affect the result");
    }

    #[test]
    fn proposed_is_never_citable_and_everything_else_is() {
        assert!(!DerivationClass::Proposed.is_citable());
        for c in ALL_CLASSES {
            if c != DerivationClass::Proposed {
                assert!(c.is_citable(), "{c:?} should be citable");
            }
        }
    }

    #[test]
    fn v0_produces_only_extracted_and_computed() {
        assert!(DerivationClass::Extracted.is_producible_in_v0());
        assert!(DerivationClass::Computed.is_producible_in_v0());
        assert!(!DerivationClass::Recognized.is_producible_in_v0());
        assert!(!DerivationClass::Proposed.is_producible_in_v0());
    }

    #[test]
    fn derivation_classes_serialize_as_snake_case() {
        for (c, s) in [
            (DerivationClass::Extracted, "\"extracted\""),
            (DerivationClass::Computed, "\"computed\""),
            (DerivationClass::Recognized, "\"recognized\""),
            (DerivationClass::Proposed, "\"proposed\""),
        ] {
            assert_eq!(serde_json::to_string(&c).unwrap(), s);
        }
    }

    #[test]
    fn absence_is_a_variant_not_an_option() {
        let a = GeometryPresence::Absent(GeometryAbsence::NotReportedByReader);
        let b = GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind);
        // Both have no rectangle, yet they are distinguishable — the whole point.
        assert_eq!(a.measured(), None);
        assert_eq!(b.measured(), None);
        assert_ne!(a, b);
    }

    #[test]
    fn only_unmeasurable_geometry_counts_as_a_limitation() {
        assert!(
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader)
                .is_declarable_limitation()
        );
        assert!(
            !GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind)
                .is_declarable_limitation()
        );
        assert!(
            !GeometryPresence::Absent(GeometryAbsence::CapabilityNotEnabled)
                .is_declarable_limitation()
        );
        assert!(
            !GeometryPresence::Absent(GeometryAbsence::NotReportedByStructureTree)
                .is_declarable_limitation(),
            "a tagged table's absent geometry is a property of the structure tree it came from, \
             not a gap in what this reader could measure — counting it would inflate the \
             ink-measurement limitation exactly as NoInkToMeasure would have"
        );
        let r = QRect::new(0, 0, 10, 10).unwrap();
        assert!(!GeometryPresence::Measured(r).is_declarable_limitation());
    }

    #[test]
    fn only_measured_geometry_is_groundable() {
        let r = QRect::new(0, 0, 10, 10).unwrap();
        assert!(GeometryPresence::Measured(r).is_groundable());
        for a in [
            GeometryAbsence::NotReportedByReader,
            GeometryAbsence::NotApplicableToKind,
            GeometryAbsence::CapabilityNotEnabled,
            GeometryAbsence::NotReportedByStructureTree,
        ] {
            assert!(!GeometryPresence::Absent(a).is_groundable());
        }
    }

    #[test]
    fn geometry_presence_round_trips_through_c14n() {
        let cases = [
            GeometryPresence::Measured(QRect::new(1, 2, 3, 4).unwrap()),
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
            GeometryPresence::Absent(GeometryAbsence::CapabilityNotEnabled),
            GeometryPresence::Absent(GeometryAbsence::NotReportedByStructureTree),
        ];
        for c in cases {
            let v = serde_json::to_value(c).unwrap();
            let bytes = c14n_bytes(&v).unwrap();
            let back: GeometryPresence = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(back, c);
        }
    }

    #[test]
    fn geometry_presence_refuses_unknown_fields() {
        let extra =
            serde_json::json!({"state": "absent", "value": "not_reported_by_reader", "extra": 1});
        assert!(
            serde_json::from_value::<GeometryPresence>(extra).is_err(),
            "GeometryPresence must fail closed on an unknown field"
        );

        // Guard the guard: the exact shape still parses.
        let ok = serde_json::json!({"state": "absent", "value": "not_reported_by_reader"});
        assert_eq!(
            serde_json::from_value::<GeometryPresence>(ok).unwrap(),
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader)
        );
    }

    #[test]
    fn absent_geometry_carries_no_zero_box_on_the_wire() {
        let v = serde_json::to_value(GeometryPresence::Absent(
            GeometryAbsence::NotReportedByReader,
        ))
        .unwrap();
        let s = String::from_utf8(c14n_bytes(&v).unwrap()).unwrap();
        assert_eq!(s, r#"{"state":"absent","value":"not_reported_by_reader"}"#);
        assert!(!s.contains('0'), "no zero coordinates may appear: {s}");
    }
}
