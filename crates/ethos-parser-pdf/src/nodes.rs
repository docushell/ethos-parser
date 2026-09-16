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

//! Extracted nodes and their locators (`docs/01-CONTRACT.md` §5).

use ethos_parser_core::{DerivationClass, GeometryPresence, NodeId};
use serde::{Deserialize, Serialize};

/// The PDF variant of `NativeLocator`.
///
/// **Required on every run.** The native locator — page plus character origin plus advance — is
/// the primitive two independent PDF stacks agree on to 0.001 pt, while disagreeing on height by
/// 6.174 pt. That asymmetry is why origins are identity and boxes are inspection
/// (`docs/01-CONTRACT.md` §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfLocator {
    /// 1-based page number.
    pub page: u32,
    /// Baseline origin x, in integer centipoints, top-left system.
    pub origin_x: i64,
    /// Baseline origin y, in integer centipoints, top-left system.
    pub origin_y: i64,
    /// Advance width in integer centipoints, or absent when the document carries no widths.
    ///
    /// `None` is **not** "zero". It means some code in the run has no width from any source this
    /// profile reads — `/Widths`, a composite font's `/W` and `/DW`, or the vendored AFM of a
    /// standard-14 face the document names (decision #22) — so the advance is unknown, and unknown
    /// is what it says. The origin is unaffected: it comes from the content stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance: Option<i64>,
}

/// Where a character came from, when it did not come from the document.
///
/// LiteParse's `trailing_space_generated` is the best honesty field in the four surveyed projects
/// (parity checklist L6): *"whether the trailing source space was synthesized by PDFium rather
/// than represented by a real space glyph."* This generalises it — a reader that inserts a
/// character has authored content, and unflagged authored content is indistinguishable from
/// evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SynthesisReason {
    /// A word gap implied by a `TJ` positioning adjustment, with no space glyph in the document.
    TjGap,
}

/// A character this reader inserted, flagged **where it was created**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynthesizedChar {
    /// Index into the run's `text`, counted in `char`s.
    pub char_index: u32,
    /// Why it was inserted.
    pub reason: SynthesisReason,
}

/// One position-aware text run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRun {
    /// Stable within this representation and this profile.
    pub id: NodeId,
    /// The decoded text.
    pub text: String,
    /// The character codes that produced it, in order.
    ///
    /// **Not 1:1 with `text`.** A ligature is one code and several scalars: the `fi` glyph is
    /// code `0x03` and characters `f` and `i`. [`Self::scalar_code_mismatch`] declares when the two
    /// counts differ rather than reconciling them, because reconciling means dropping one or the
    /// other.
    pub char_codes: Vec<u32>,
    /// True when `text.chars().count() != char_codes.len()` — Unicode scalar values, not UTF-16
    /// code units or bytes.
    ///
    /// Declared, not silently normalised. LiteParse notes the same caveat in a doc comment; this
    /// puts it on the wire where a consumer can act on it.
    ///
    /// **A comparison of two counts, not a mapping.** A character in [`Self::synthesized`] has no
    /// code and sets it too, so `true` does not by itself mean a code decoded to several
    /// characters.
    pub scalar_code_mismatch: bool,
    /// Characters this reader inserted. Empty for a run taken verbatim from the document.
    pub synthesized: Vec<SynthesizedChar>,
    /// Font resource name.
    pub font_id: String,
    /// Font size in integer centipoints.
    ///
    /// **Never used as a box height.** It is here because it is a real property of the run;
    /// pdf-inspector's defect was making `height` the same variable (parity checklist P5).
    pub font_size: i64,
    /// The required native locator.
    pub locator: PdfLocator,
    /// Measured ink box, or a typed reason there is none.
    pub geometry: GeometryPresence,
    /// Which region of its page the reading-order cut placed this run in (D4-S2).
    ///
    /// Set after detection and **before** `reorder_page`, which moves whole runs, so the value
    /// travels with its run and needs no index fixing of its own. `None` on a page the cut did
    /// not divide.
    pub region: Option<u32>,
    /// Which block of its page the leading-gap cut placed this run in.
    ///
    /// Set beside [`Self::region`] and travelling the same way. `None` on a page the rule
    /// declined — which, unlike `region`, is common even on a single-column page, because a page
    /// of uniform body text has no gap wide enough to open a second block.
    pub block: Option<u32>,
    /// Marked-content id, when the page declares one for this run.
    ///
    /// `None` means the document did not supply one. Never invented — Workbench rule 3.
    ///
    /// **Extracted**: the raw `BDC` operand, kept as the join key it is. [`Self::structural`] is
    /// the *result* of joining it against the document's structure tree, which is a different
    /// derivation and so a different field — a reader can check one against the other.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcid: Option<i64>,
    /// The structural address this run resolved to (v1-S3).
    ///
    /// Absent when the page marked nothing here. Present as `pdf_tagged` when the document's
    /// structure tree cites this run's `(page, mcid)`, as `pdf_mcid` when the content stream gave
    /// an id no structure element claims, and as `pdf_artifact` when the page marked this as
    /// furniture rather than content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural: Option<ethos_parser_core::StructuralLocator>,
    /// How this run came to exist. `Extracted` for text read from the content stream.
    pub derivation: DerivationClass,
    /// What was observed about this run beyond its text (v1-S6).
    ///
    /// **The run is here either way.** A finding is added to a node, never a reason to drop one:
    /// OpenDataLoader deletes low-contrast text before returning a page, so its caller cannot
    /// tell a clean document from a scrubbed one, and that is the defect checklist O21 names.
    ///
    /// Sorted and deduplicated, so two runs observed the same way cannot differ by the order the
    /// observations happened to be made in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<ethos_parser_core::TextFinding>,
}

impl TextRun {
    /// Whether the scalar count and code count actually disagree.
    ///
    /// Recomputed rather than trusted, so the stored flag can be checked against the data.
    pub fn compute_scalar_code_mismatch(&self) -> bool {
        self.text.chars().count() != self.char_codes.len()
    }
}

/// Per-page extraction result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageExtract {
    /// 1-based page number.
    pub index: u32,
    /// Page width in integer centipoints, after rotation.
    pub width: i64,
    /// Page height in integer centipoints, after rotation.
    pub height: i64,
    /// Page rotation as declared: 0, 90, 180 or 270.
    pub rotation: i64,
    /// Runs in reading order.
    pub runs: Vec<TextRun>,
    /// Ruled tables detected on this page (v1-S1).
    ///
    /// **Empty means the detector looked and found none** — never "did not look". The capability
    /// says which of those two a reader is seeing, and `ethos.grounding.v1` draws the same
    /// distinction with an absent key versus an empty array.
    ///
    /// **Geometric only.** A table the document tagged but no detector matched is in
    /// [`Self::tagged_tables`] instead, kept apart so `accuracy`'s geometric gate scores the
    /// detectors against the tree without the tagged tables — which come from the tree — scoring it
    /// against itself.
    pub tables: Vec<crate::tables::DetectedTable>,
    /// Tables the document's structure tree declares that no geometric detector matched (v2-S24).
    ///
    /// A separate list from [`Self::tables`] because it is a different kind of statement: these are
    /// `Extracted` from the document's own `/Table` tags and carry no geometry, where a detected
    /// table is `Computed` from ink and carries a measured box. Empty for an untagged document, for
    /// a tagged one whose every table a detector matched, and for a page the tree describes no
    /// table on. Omitted from the wire when empty, so a document with no tagged tables is
    /// byte-identical to one produced before this field existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tagged_tables: Vec<crate::tables::TaggedTableRecord>,
    /// Form fields and annotations this page carries (v1-S4).
    ///
    /// **Empty means the walk looked and found none.** The capabilities say which of those a
    /// reader is seeing, exactly as they do for `tables`.
    ///
    /// Separate from `runs` because these are not runs: their text comes from dictionaries that
    /// no content-stream operator mentions. Merging them into the run list is the one thing this
    /// slice exists to prevent.
    pub objects: Vec<PageObjectRecord>,
    /// Images this page painted with `Do` (v1-S6).
    ///
    /// **Empty means the walk looked and found none**, as for `tables` and `objects`. One entry
    /// per placement, in the order the page painted them — the same XObject drawn five times is
    /// five entries sharing one object number, which is the fact a consumer needs to tell that
    /// from five different pictures.
    ///
    /// Separate from `runs` for the reason `objects` is: an image is not text, it has no
    /// baseline, and nothing about it belongs in a reading order over glyphs.
    pub images: Vec<ImageRecord>,
}

/// One image placement, as extraction records it (v1-S6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRecord {
    /// Stable id, allocated like any other node's.
    pub id: NodeId,
    /// Page, XObject number, and the rectangle this placement painted into.
    pub locator: ethos_parser_core::PdfImageLocator,
    /// What the stream dictionary declares, and the digest of its bytes.
    pub attributes: ethos_parser_core::ImageAttributes,
}

/// One annotation or form field, as extraction records it (v1-S4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageObjectRecord {
    /// Stable id, allocated like any other node's.
    pub id: NodeId,
    /// The object number it was read from, and the rectangle it declares.
    pub locator: ethos_parser_core::PdfObjectLocator,
    /// Its text: a field's value, or an annotation's `/Contents`. Empty where it carries none.
    pub text: String,
    /// The kind-specific facts, which also decide the node kind.
    pub attributes: ethos_parser_core::NodeAttributes,
    /// A role path, when the structure tree cites this object (v1-S4, decision 7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural: Option<ethos_parser_core::StructuralLocator>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethos_parser_core::{c14n_bytes, GeometryAbsence, IdAllocator, IdKind, Profile, QRect};

    fn run(text: &str, codes: Vec<u32>) -> TextRun {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        TextRun {
            id: alloc.next(IdKind::Span).unwrap(),
            region: None,
            block: None,
            text: text.to_string(),
            char_codes: codes,
            scalar_code_mismatch: false,
            synthesized: Vec::new(),
            font_id: "F1".into(),
            font_size: 2400,
            locator: PdfLocator {
                page: 1,
                origin_x: 7200,
                origin_y: 7200,
                advance: Some(1000),
            },
            geometry: GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
            mcid: None,
            findings: Vec::new(),
            structural: None,
            derivation: DerivationClass::Extracted,
        }
    }

    #[test]
    fn a_ligature_run_reports_a_mismatch() {
        // "fi" is one code and two characters.
        let r = run("fi", vec![0x03]);
        assert!(
            r.compute_scalar_code_mismatch(),
            "two scalars from one code must be declared, not reconciled"
        );
    }

    #[test]
    fn a_plain_run_reports_no_mismatch() {
        let r = run("abc", vec![0x61, 0x62, 0x63]);
        assert!(!r.compute_scalar_code_mismatch());
    }

    #[test]
    fn an_absent_advance_is_omitted_rather_than_zero() {
        let mut r = run("a", vec![0x61]);
        r.locator.advance = None;
        let v = serde_json::to_value(&r.locator).unwrap();
        let s = String::from_utf8(c14n_bytes(&v).unwrap()).unwrap();
        assert!(
            !s.contains("advance"),
            "an unknown advance is omitted, never serialized as 0: {s}"
        );
        assert!(s.contains("origin_x"), "the origin is still exact: {s}");
    }

    #[test]
    fn a_known_advance_is_present() {
        let r = run("a", vec![0x61]);
        let v = serde_json::to_value(&r.locator).unwrap();
        let s = String::from_utf8(c14n_bytes(&v).unwrap()).unwrap();
        assert!(s.contains("\"advance\":1000"));
    }

    #[test]
    fn synthesized_characters_survive_canonicalization() {
        let mut r = run("a b", vec![0x61, 0x62]);
        r.synthesized.push(SynthesizedChar {
            char_index: 1,
            reason: SynthesisReason::TjGap,
        });
        let v = serde_json::to_value(&r).unwrap();
        let bytes = c14n_bytes(&v).unwrap();
        let s = String::from_utf8(bytes.clone()).unwrap();
        assert!(s.contains("\"char_index\":1"));
        assert!(s.contains("\"reason\":\"tj-gap\""));

        let back: TextRun = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, r, "the flag must survive a round trip");
    }

    #[test]
    fn a_run_round_trips_through_c14n() {
        let mut r = run("hello", vec![0x68, 0x65, 0x6c, 0x6c, 0x6f]);
        r.geometry = GeometryPresence::Measured(QRect::new(10, 20, 30, 40).unwrap());
        r.mcid = Some(7);
        let bytes = c14n_bytes(&serde_json::to_value(&r).unwrap()).unwrap();
        let back: TextRun = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn an_absent_mcid_is_omitted_never_invented() {
        let r = run("a", vec![0x61]);
        let s = String::from_utf8(c14n_bytes(&serde_json::to_value(&r).unwrap()).unwrap()).unwrap();
        assert!(
            !s.contains("mcid"),
            "absent means absent, not a default id: {s}"
        );
    }

    #[test]
    fn no_float_reaches_the_wire() {
        let r = run("a", vec![0x61]);
        // c14n rejects any non-integer number, so success is the assertion.
        c14n_bytes(&serde_json::to_value(&r).unwrap()).expect("no floats in a run");
    }
}
