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

//! Fonts: decoding, widths, and measured metrics.
//!
//! # Three questions per glyph, three independent answers
//!
//! 1. **What character is it?** `ToUnicode` if the document ships one, otherwise the simple
//!    encoding ([`crate::encoding`]). Refused if neither can answer.
//! 2. **How far does it advance?** `/Widths` from the document. **Refused if absent** — see
//!    [`Font::advance_glyph_space`].
//! 3. **What box does its ink occupy?** Measured from the embedded font program or the
//!    `FontDescriptor`, else [`engine_core::GeometryAbsence::NotReportedByReader`].
//!
//! Keeping them separate matters because they fail separately. A font can have perfect widths and
//! no metrics, and conflating the two is how `height = font_size` gets written.

use std::collections::BTreeMap;

use engine_core::{
    quantize, EngineError, GeometryAbsence, GeometryPresence, QRect, QUANTUM_PER_POINT,
};

use crate::cmap::ToUnicode;
use crate::encoding::{BaseEncoding, SimpleEncoding};

/// Glyph-space units per em in a simple font. PDF 32000-1 §9.2.4.
const GLYPH_SPACE_UNITS: f64 = 1000.0;

/// How a font's advance widths are known, or why they are not.
#[derive(Debug, Clone, PartialEq)]
pub enum WidthSource {
    /// `/Widths` from the font dictionary, indexed from `/FirstChar`.
    Widths {
        /// First code the array covers.
        first_char: i64,
        /// Widths in glyph space.
        widths: Vec<f64>,
        /// `/FontMatrix` scale for Type 3 fonts; `None` means the 1/1000 default.
        type3_scale_x: Option<f64>,
    },
    /// No width information in the document.
    ///
    /// The standard-14 fonts may legally omit `/Widths`, expecting the reader to supply built-in
    /// AFM metrics. **This profile does not vendor those tables**, so the advance is unknown and
    /// is reported as unknown rather than guessed. Origins remain exact — they come from the
    /// content stream, not from the font.
    Absent {
        /// Why, as the `detail` of the document-scoped `font-widths-absent` limitation.
        ///
        /// This said *"for the artifact's `not_decoded` list"* until v2-S13.3. That list was M3's
        /// and M4 absorbed it into `assurance.limitations`; the string still travels, under a
        /// name that still exists.
        reason: String,
    },
}

/// How a glyph's characters are determined.
#[derive(Debug, Clone)]
pub enum Decoder {
    /// The document's own `ToUnicode` CMap. Authoritative.
    ToUnicode(ToUnicode),
    /// A simple encoding plus `/Differences`.
    Simple(SimpleEncoding),
}

/// What kind of font a resource declares itself to be (v1-S6.1).
///
/// **This decides how a string is split into character codes, and nothing else does.** PDF
/// 32000-1 §9.6 gives simple fonts single-byte codes — always, unconditionally, whatever else the
/// font ships. §9.7.5 gives a composite font's code width to the CMap named by its `/Encoding`.
///
/// Through v1-S6 the width came from whichever *decoder* the font happened to get, so a simple
/// font that shipped a `/ToUnicode` with a two-byte codespace had its codes read in pairs. That
/// destroyed text on every real document in the corpus and on none of the fixtures — see
/// `docs/09-V1-MILESTONES.md` S6.1. The kind is read from the document's own `/Subtype`, which was
/// already parsed and simply never reached the decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontKind {
    /// `Type1`, `TrueType`, `Type3`, `MMType1` — one byte per code, always.
    Simple,
    /// `Type0` — a composite font, whose `/Encoding` CMap decides the width.
    Composite,
}

impl FontKind {
    /// Classify a `/Subtype` name.
    ///
    /// **Anything unrecognised is [`Self::Simple`]**, including a font dictionary that declares no
    /// `/Subtype` at all. Single-byte is the conservative reading: it is what every simple font
    /// uses, it is what this reader did before `/ToUnicode` support existed, and treating an
    /// unlabelled font as composite would split its codes in pairs — which is the exact failure
    /// this type exists to end. A font that really is composite says so; §9.7.1 requires it.
    fn from_subtype(subtype: &str) -> Self {
        match subtype {
            "Type0" => Self::Composite,
            _ => Self::Simple,
        }
    }
}

/// One font resource, resolved.
#[derive(Debug, Clone)]
pub struct Font {
    /// Resource name, e.g. `F1`.
    pub id: String,
    /// What the document declares this font to be. Decides the code width (v1-S6.1).
    pub kind: FontKind,
    /// How to turn codes into characters.
    pub decoder: Decoder,
    /// How to turn codes into advances.
    pub widths: WidthSource,
    /// Measured ink extent for this font, in glyph space, or a typed absence.
    ///
    /// Font-level rather than per-glyph: a per-glyph ink box needs the glyph outline, which is
    /// M-later work. This is the font's ascent/descent envelope, and it is **measured** — from
    /// the embedded program or the descriptor — or absent.
    pub ink: FontInk,
}

/// A font's vertical ink extent, measured or typed-absent.
#[derive(Debug, Clone, PartialEq)]
pub enum FontInk {
    /// Ascent and descent in glyph-space units, and where they came from.
    Measured {
        /// Highest ink above the baseline.
        ascent: f64,
        /// Lowest ink below the baseline. Negative.
        descent: f64,
        /// `embedded-font-program` or `font-descriptor`.
        source: &'static str,
    },
    /// No metrics available.
    ///
    /// **Never substituted with the font size.** pdf-inspector's `TextItem.height` is literally
    /// the same variable as `font_size` (parity checklist P5) — a font-size-derived box is closer
    /// to invented than measured, and inventing a coordinate is forbidden.
    Absent(GeometryAbsence),
}

impl Font {
    /// Split a string operand into character codes.
    ///
    /// **The width comes from what the font DECLARES ITSELF TO BE, not from how it happens to be
    /// decoded** (v1-S6.1). A simple font is one byte per code, always — `/ToUnicode` maps codes
    /// to Unicode and has no say in how a string is divided (PDF 32000-1 §9.6).
    ///
    /// Through v1-S6 this read the `/ToUnicode` codespace whenever one existed, so a simple font
    /// shipping `<0000><FFFF>` had its codes fused in pairs: 2 827 font instances across the three
    /// real corpus documents, 8 417 runs dropped from one of them, and zero fixtures affected
    /// because every conformance font is a Type1 with no `/ToUnicode`.
    ///
    /// The composite branch is a **declared interim**. Nothing here parses `/Encoding` CMaps, so a
    /// Type0 font's width still comes from its `/ToUnicode` codespace — right for Identity-H, which
    /// is what real documents overwhelmingly use, and unverified otherwise.
    /// `composite-font-codes-from-tounicode` says so on any artifact where it applies, rather than
    /// leaving it to be discovered the way this defect was.
    pub fn split_codes(&self, bytes: &[u8]) -> Vec<u32> {
        let width = match self.kind {
            FontKind::Simple => 1,
            FontKind::Composite => match &self.decoder {
                Decoder::ToUnicode(t) => t.code_bytes().max(1),
                Decoder::Simple(_) => 1,
            },
        };
        if width == 1 {
            return bytes.iter().map(|b| u32::from(*b)).collect();
        }
        bytes
            .chunks(width)
            .map(|c| c.iter().fold(0u32, |acc, b| (acc << 8) | u32::from(*b)))
            .collect()
    }

    /// Decode one character code to its characters.
    ///
    /// # Errors
    ///
    /// [`EngineError::Unsupported`] when neither the `ToUnicode` CMap nor the encoding can map
    /// the code. Refused rather than replaced: a substitution character in the evidence is a
    /// character the document does not contain.
    pub fn decode_code(&self, code: u32) -> Result<String, EngineError> {
        match &self.decoder {
            Decoder::ToUnicode(t) => {
                t.get(code)
                    .map(str::to_string)
                    .ok_or_else(|| EngineError::Unsupported {
                        what: "character code".into(),
                        detail: format!(
                            "font /{} has a ToUnicode CMap with no entry for code {code:#04x}",
                            self.id
                        ),
                    })
            }
            Decoder::Simple(e) => {
                let byte = u8::try_from(code).map_err(|_| EngineError::Malformed {
                    what: "character code".into(),
                    detail: format!("code {code:#x} exceeds one byte in a simple font"),
                })?;
                e.decode(byte).map(str::to_string)
            }
        }
    }

    /// Advance for one code, in **text space units per unit font size**.
    ///
    /// Multiply by font size to get text-space advance. Returns `None` when the document carries
    /// no width information — see [`WidthSource::Absent`].
    pub fn advance_glyph_space(&self, code: u32) -> Option<f64> {
        match &self.widths {
            WidthSource::Absent { .. } => None,
            WidthSource::Widths {
                first_char,
                widths,
                type3_scale_x,
            } => {
                let idx = i64::from(code) - first_char;
                let raw = *widths.get(usize::try_from(idx).ok()?)?;
                Some(match type3_scale_x {
                    // Type 3 widths are in glyph space, mapped to text space by /FontMatrix.
                    Some(sx) => raw * sx,
                    None => raw / GLYPH_SPACE_UNITS,
                })
            }
        }
    }

    /// The measured ink box for a run, in text space, given a font size and baseline origin.
    ///
    /// # Errors
    ///
    /// Never. Absence is a value, not an error: a run without metrics is still evidence, and its
    /// origin is still exact.
    pub fn ink_box(
        &self,
        origin_x_pt: f64,
        baseline_y_pt: f64,
        width_pt: f64,
        font_size_pt: f64,
    ) -> GeometryPresence {
        let FontInk::Measured {
            ascent, descent, ..
        } = &self.ink
        else {
            let FontInk::Absent(why) = &self.ink else {
                unreachable!("FontInk has two variants")
            };
            return GeometryPresence::Absent(*why);
        };

        if width_pt <= 0.0 {
            // A zero-width run covers no area, so there is no box — and since v1-S6.2 it says
            // which kind of nothing that is. It was `NotReportedByReader`, which claims the reader
            // failed; the reader did not fail, the run has no extent.
            return GeometryPresence::Absent(GeometryAbsence::NoInkToMeasure);
        }

        // Ascent/descent are glyph-space units per em; scale by the font size.
        let top_pt = baseline_y_pt - (ascent / GLYPH_SPACE_UNITS) * font_size_pt;
        let bottom_pt = baseline_y_pt - (descent / GLYPH_SPACE_UNITS) * font_size_pt;

        let (Ok(x0), Ok(y0), Ok(x1), Ok(y1)) = (
            quantize(origin_x_pt, QUANTUM_PER_POINT),
            quantize(top_pt, QUANTUM_PER_POINT),
            quantize(origin_x_pt + width_pt, QUANTUM_PER_POINT),
            quantize(bottom_pt, QUANTUM_PER_POINT),
        ) else {
            return GeometryPresence::Absent(GeometryAbsence::NotReportedByReader);
        };

        match QRect::new(x0, y0, x1, y1) {
            Ok(r) => GeometryPresence::Measured(r),
            // A degenerate rectangle is refused rather than nudged into validity.
            Err(_) => GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        }
    }
}

/// Resolve every font in a page's resource dictionary.
///
/// # Errors
///
/// [`EngineError`] if a font dictionary is present but unreadable. A page with no `/Font`
/// resource yields an empty map rather than an error — that is a page without text, not a broken
/// page.
pub fn load_page_fonts(
    doc: &lopdf::Document,
    page_dict: &lopdf::Dictionary,
) -> Result<BTreeMap<String, Font>, EngineError> {
    let mut out = BTreeMap::new();

    let Some(resources) = resolve_dict(doc, page_dict.get(b"Resources").ok()) else {
        return Ok(out);
    };
    let Some(fonts) = resolve_dict(doc, resources.get(b"Font").ok()) else {
        return Ok(out);
    };

    for (name, value) in fonts.iter() {
        let id = String::from_utf8_lossy(name).to_string();
        let Some(fd) = resolve_dict(doc, Some(value)) else {
            return Err(EngineError::Malformed {
                what: "font resource".into(),
                detail: format!("/{id} does not resolve to a dictionary"),
            });
        };
        out.insert(id.clone(), load_font(doc, &id, &fd)?);
    }

    Ok(out)
}

fn load_font(doc: &lopdf::Document, id: &str, fd: &lopdf::Dictionary) -> Result<Font, EngineError> {
    let subtype = fd
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| String::from_utf8_lossy(n).to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // `/BaseFont` is deliberately not read. It was parsed and stored through M6 and nothing ever
    // looked at it — the decoder comes from `/ToUnicode` or the encoding, the advance from
    // `/Widths`, and the ink box from the embedded program or the descriptor. A font name would
    // only be useful for substituting metrics this profile refuses to substitute, so carrying it
    // was state with no reader. Removed at M7 with the rest of the API freeze.

    // 1. Decoder — ToUnicode wins when the document ships one.
    let decoder = if let Some(stream) = resolve_stream(doc, fd.get(b"ToUnicode").ok()) {
        let bytes = stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone());
        Decoder::ToUnicode(ToUnicode::parse(&bytes)?)
    } else {
        Decoder::Simple(load_simple_encoding(doc, fd)?)
    };

    // 2. Widths.
    let widths = load_widths(doc, fd, &subtype, id);

    // 3. Ink metrics.
    let ink = crate::metrics::resolve_font_ink(doc, fd);

    Ok(Font {
        id: id.to_string(),
        kind: FontKind::from_subtype(&subtype),
        decoder,
        widths,
        ink,
    })
}

fn load_simple_encoding(
    doc: &lopdf::Document,
    fd: &lopdf::Dictionary,
) -> Result<SimpleEncoding, EngineError> {
    let mut base = BaseEncoding::Builtin;
    let mut differences = BTreeMap::new();

    match fd.get(b"Encoding").ok() {
        Some(lopdf::Object::Name(n)) => {
            base = BaseEncoding::from_name(n).ok_or_else(|| EngineError::Unsupported {
                what: "encoding".into(),
                detail: format!(
                    "/Encoding /{} is not a simple encoding this profile carries. Predefined \
                     CMaps (the Adobe CJK set) are not vendored; a document needing one is \
                     refused rather than decoded approximately.",
                    String::from_utf8_lossy(n)
                ),
            })?;
        }
        Some(obj) => {
            if let Some(ed) = resolve_dict(doc, Some(obj)) {
                if let Ok(lopdf::Object::Name(n)) = ed.get(b"BaseEncoding") {
                    base = BaseEncoding::from_name(n).unwrap_or(BaseEncoding::Builtin);
                }
                if let Ok(lopdf::Object::Array(items)) = ed.get(b"Differences") {
                    let mut code: i64 = 0;
                    for item in items {
                        match item {
                            lopdf::Object::Integer(i) => code = *i,
                            lopdf::Object::Real(r) => code = *r as i64,
                            lopdf::Object::Name(n) => {
                                if let Ok(c) = u8::try_from(code) {
                                    differences.insert(c, String::from_utf8_lossy(n).to_string());
                                }
                                code += 1;
                            }
                            _ => {
                                return Err(EngineError::Malformed {
                                    what: "/Differences array".into(),
                                    detail: "entries must be integers or names".into(),
                                })
                            }
                        }
                    }
                }
            }
        }
        None => {}
    }

    Ok(SimpleEncoding::new(base, differences))
}

fn load_widths(
    doc: &lopdf::Document,
    fd: &lopdf::Dictionary,
    subtype: &str,
    id: &str,
) -> WidthSource {
    let first_char = fd.get(b"FirstChar").ok().and_then(|o| o.as_i64().ok());
    let widths = fd
        .get(b"Widths")
        .ok()
        .and_then(|o| resolve_array(doc, Some(o)))
        .map(|arr| {
            arr.iter()
                .map(|o| match o {
                    lopdf::Object::Integer(i) => *i as f64,
                    lopdf::Object::Real(r) => f64::from(*r),
                    _ => 0.0,
                })
                .collect::<Vec<f64>>()
        });

    match (first_char, widths) {
        (Some(fc), Some(w)) if !w.is_empty() => {
            // Type 3 widths are in glyph space and need /FontMatrix to reach text space.
            let type3_scale_x = if subtype == "Type3" {
                resolve_array(doc, fd.get(b"FontMatrix").ok()).and_then(|m| match m.first() {
                    Some(lopdf::Object::Real(r)) => Some(f64::from(*r)),
                    Some(lopdf::Object::Integer(i)) => Some(*i as f64),
                    _ => None,
                })
            } else {
                None
            };
            WidthSource::Widths {
                first_char: fc,
                widths: w,
                type3_scale_x,
            }
        }
        _ => WidthSource::Absent {
            reason: format!(
                "font /{id} ({subtype}{}) carries no /Widths array. The standard-14 built-in AFM \
                 metrics are not vendored by this profile, so the advance is unknown rather than \
                 assumed. Origins are unaffected — they come from the content stream.",
                fd.get(b"BaseFont")
                    .ok()
                    .and_then(|o| o.as_name().ok())
                    .map(|n| format!(", {}", String::from_utf8_lossy(n)))
                    .unwrap_or_default()
            ),
        },
    }
}

// --- object helpers ---------------------------------------------------------------------

pub(crate) fn resolve_dict<'a>(
    doc: &'a lopdf::Document,
    obj: Option<&'a lopdf::Object>,
) -> Option<lopdf::Dictionary> {
    match obj? {
        lopdf::Object::Dictionary(d) => Some(d.clone()),
        lopdf::Object::Reference(r) => doc.get_object(*r).ok()?.as_dict().ok().cloned(),
        _ => None,
    }
}

pub(crate) fn resolve_array<'a>(
    doc: &'a lopdf::Document,
    obj: Option<&'a lopdf::Object>,
) -> Option<Vec<lopdf::Object>> {
    match obj? {
        lopdf::Object::Array(a) => Some(a.clone()),
        lopdf::Object::Reference(r) => doc.get_object(*r).ok()?.as_array().ok().cloned(),
        _ => None,
    }
}

/// Follow one indirect reference, if the object is one (v1-S6).
///
/// A page attribute may be written either way — `/Rotate 90` and `/Rotate 90 0 R` mean the same
/// thing — and a reader that handles only the direct form silently reads the indirect one as
/// absent. That is how a rotated page becomes an unrotated one, and every coordinate on it wrong.
pub(crate) fn resolve_object<'a>(
    doc: &'a lopdf::Document,
    obj: Option<&'a lopdf::Object>,
) -> Option<lopdf::Object> {
    match obj? {
        lopdf::Object::Reference(r) => doc.get_object(*r).ok().cloned(),
        other => Some(other.clone()),
    }
}

pub(crate) fn resolve_stream<'a>(
    doc: &'a lopdf::Document,
    obj: Option<&'a lopdf::Object>,
) -> Option<lopdf::Stream> {
    match obj? {
        lopdf::Object::Stream(s) => Some(s.clone()),
        lopdf::Object::Reference(r) => doc.get_object(*r).ok()?.as_stream().ok().cloned(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font_with(widths: WidthSource, ink: FontInk) -> Font {
        Font {
            id: "F1".into(),
            kind: FontKind::Simple,
            decoder: Decoder::Simple(SimpleEncoding::new(BaseEncoding::WinAnsi, BTreeMap::new())),
            widths,
            ink,
        }
    }

    #[test]
    fn a_font_without_widths_reports_no_advance_rather_than_guessing() {
        let f = font_with(
            WidthSource::Absent {
                reason: "no /Widths".into(),
            },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        assert_eq!(
            f.advance_glyph_space(b'A' as u32),
            None,
            "an unknown advance is None, never a plausible default"
        );
    }

    #[test]
    fn widths_are_indexed_from_first_char() {
        let f = font_with(
            WidthSource::Widths {
                first_char: 65,
                widths: vec![500.0, 600.0, 700.0],
                type3_scale_x: None,
            },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        assert_eq!(f.advance_glyph_space(65), Some(0.5));
        assert_eq!(f.advance_glyph_space(67), Some(0.7));
        assert_eq!(f.advance_glyph_space(68), None, "past the array end");
        assert_eq!(f.advance_glyph_space(64), None, "before FirstChar");
    }

    #[test]
    fn type3_widths_use_the_font_matrix_scale() {
        let f = font_with(
            WidthSource::Widths {
                first_char: 1,
                widths: vec![600.0],
                type3_scale_x: Some(0.001),
            },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        assert_eq!(f.advance_glyph_space(1), Some(0.6));
    }

    /// The rule this whole module exists to protect.
    #[test]
    fn ink_is_never_derived_from_font_size() {
        let f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        // A 24pt font with no metrics gets no box, not a 24pt-tall one.
        let g = f.ink_box(72.0, 72.0, 100.0, 24.0);
        assert_eq!(
            g,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader)
        );
        assert!(g.measured().is_none());
    }

    #[test]
    fn measured_metrics_produce_a_real_box() {
        let f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Measured {
                ascent: 718.0,
                descent: -207.0,
                source: "font-descriptor",
            },
        );
        let g = f.ink_box(72.0, 100.0, 50.0, 10.0);
        let r = g.measured().expect("a measured box");
        // Top is above the baseline (smaller y in a top-left system), bottom below.
        assert!(r.y0() < r.y1());
        assert_eq!(r.x0(), 7200);
        assert_eq!(r.x1(), 12200);
        // 718/1000 * 10pt = 7.18pt above the baseline -> 100 - 7.18 = 92.82pt -> 9282 centipoints
        assert_eq!(r.y0(), 9282);
        // -207/1000 * 10pt = -2.07 -> 100 + 2.07 = 102.07pt
        assert_eq!(r.y1(), 10207);
    }

    #[test]
    fn a_zero_width_run_gets_absence_not_a_degenerate_box() {
        let f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Measured {
                ascent: 718.0,
                descent: -207.0,
                source: "font-descriptor",
            },
        );
        assert!(f.ink_box(72.0, 100.0, 0.0, 10.0).measured().is_none());
    }

    #[test]
    fn simple_fonts_split_one_byte_per_code() {
        let f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        assert_eq!(f.split_codes(b"Hi"), vec![0x48, 0x69]);
    }

    fn two_byte_cmap() -> ToUnicode {
        ToUnicode::parse(
            b"1 begincodespacerange <0000> <ffff> endcodespacerange
              1 beginbfchar <0041> <0061> endbfchar",
        )
        .unwrap()
    }

    /// **This golden reversed at v1-S6.1, and this note is the record.**
    ///
    /// It used to be `two_byte_codespaces_split_in_pairs`, and it asserted that a font whose
    /// decoder is a two-byte `ToUnicode` splits its string in pairs — for *any* font, because the
    /// width came from the decoder. That was the defect: PDF 32000-1 §9.6 gives a simple font
    /// single-byte codes unconditionally, and `/ToUnicode` maps codes to characters without any
    /// say in how a string is divided.
    ///
    /// The test was not wrong about the code; it was wrong about the rule, and it held the wrong
    /// rule in place for six slices while **2 827 font instances** were read two bytes at a time:
    /// 2 426 in `nist-sp-800-53r5`, 303 in `nist-sp-800-63b` and 98 in `cfpb-home-loan-toolkit`,
    /// with none in `irs-form-1040-2025` and none across the conformance synthetics.
    ///
    /// The figure is v1-S6.1's own measurement and it stands. What did not was the phrase
    /// *"across the three real corpus documents"*, repaired at v2-S13.3: **four** real documents
    /// were measured, the fourth contributing zero, and the three `fixtures/manifest.json`
    /// declares as the `benchmark` corpus are a *different* three — `cfpb-home-loan-toolkit`,
    /// which supplied 98 of these, is not among them, so read against the manifest's trio the
    /// number would have been 2 729. Naming the documents costs a line and cannot drift; a
    /// count of a set nobody can identify already had.
    #[test]
    fn a_simple_font_splits_one_byte_per_code_whatever_its_tounicode_says() {
        let mut f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        f.decoder = Decoder::ToUnicode(two_byte_cmap());
        assert_eq!(f.kind, FontKind::Simple);
        assert_eq!(
            f.split_codes(&[0x00, 0x41, 0x00, 0x42]),
            vec![0x00, 0x41, 0x00, 0x42],
            "a two-byte codespace on a SIMPLE font changes nothing about how its string is split"
        );
    }

    /// The composite half, which is still the `ToUnicode` codespace — and declared as such.
    ///
    /// Not because that is right in general: §9.7.5 gives the width to the CMap named by
    /// `/Encoding`, and nothing here parses one. It agrees with `Identity-H` and is unverified
    /// otherwise, which is what `composite-font-codes-from-tounicode` exists to say out loud.
    #[test]
    fn a_composite_font_takes_its_width_from_the_cmap() {
        let mut f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        f.kind = FontKind::Composite;
        f.decoder = Decoder::ToUnicode(two_byte_cmap());
        assert_eq!(
            f.split_codes(&[0x00, 0x41, 0x00, 0x42]),
            vec![0x0041, 0x0042]
        );
    }

    /// An unlabelled font is simple, which is the conservative reading.
    #[test]
    fn a_font_with_no_recognised_subtype_is_simple() {
        for subtype in ["Type1", "TrueType", "Type3", "MMType1", "Unknown", ""] {
            assert_eq!(
                FontKind::from_subtype(subtype),
                FontKind::Simple,
                "`{subtype}` must split one byte per code"
            );
        }
        assert_eq!(FontKind::from_subtype("Type0"), FontKind::Composite);
    }

    #[test]
    fn an_unmapped_code_is_refused() {
        let f = font_with(
            WidthSource::Absent { reason: "x".into() },
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
        );
        // WinAnsi 0x81 is unassigned.
        assert!(f.decode_code(0x81).is_err());
    }
}
