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

//! Fonts: decoding, widths, and measured metrics.
//!
//! # Three questions per glyph, three independent answers
//!
//! 1. **What character is it?** `ToUnicode` if the document ships one, otherwise the simple
//!    encoding ([`crate::encoding`]). Refused if neither can answer.
//! 2. **How far does it advance?** `/Widths` from the document. **Refused if absent** — see
//!    [`Font::advance_glyph_space`].
//! 3. **What box does its ink occupy?** Measured from the embedded font program or the
//!    `FontDescriptor`, else [`ethos_parser_core::GeometryAbsence::NotReportedByReader`].
//!
//! Keeping them separate matters because they fail separately. A font can have perfect widths and
//! no metrics, and conflating the two is how `height = font_size` gets written.

use std::collections::BTreeMap;
use std::sync::Arc;

use ethos_parser_core::{
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
    /// `/W` and `/DW` from a composite font's DESCENDANT CIDFont, keyed by CID (v2.2-S3).
    ///
    /// **A `Type0` font never carries `/Widths`** — PDF 32000-1 §9.7.4.3 puts a composite font's
    /// widths on the descendant, as `/W` spans with `/DW` as the default. Until v2.2-S3 this
    /// module knew only the simple shape, so every composite font fell through to
    /// [`Self::Absent`] and reported an unknown advance while the document supplied a perfectly
    /// good one. Measured on the 200-document `opendataloader-bench` corpus: **50 of 50
    /// documents and 72 of 72 composite fonts** were declared width-absent, and every one of them
    /// had `/W` or `/DW` in the file. A 100% false-positive rate.
    ///
    /// The neighbouring reader already knew: [`crate::metrics::resolve_font_ink`] opens with
    /// *"Type 0 fonts hold their descriptor on the descendant"* and walks `/DescendantFonts`
    /// correctly for the ink envelope. The rule was in the tree, one file away, and had never
    /// been applied to the second reader that needs it.
    Cid {
        /// `first_cid -> (last_cid, width)`, the spans `/W` names, in glyph space.
        ///
        /// A `BTreeMap` rather than a list because the lookup is per glyph on the innermost
        /// extraction loop: `range(..=cid).next_back()` finds the last span beginning at or
        /// before a CID in log time. Both `/W` forms land here — `c [w1 w2 …]` as one
        /// single-CID span per width, and `c_first c_last w` as one span. Both occur in the
        /// wild: 1 143 and 810 respectively on that corpus, so implementing one would have
        /// silently mis-measured the other.
        spans: BTreeMap<u32, (u32, f64)>,
        /// `/DW`, the width of every CID `/W` does not name.
        ///
        /// **1000 when the document omits it, and that is reading rather than guessing**:
        /// §9.7.4.3 states *"Default value: 1000"*, so an absent `/DW` is a declared value the
        /// same way an absent `/Encoding` on a simple font declares the built-in one. The
        /// distinction matters here because it is the difference between this variant and
        /// [`Self::Absent`]: under an Identity CMap a composite font's advance is therefore
        /// **never** unknown, which is why nothing falls through to `Absent` for width reasons
        /// once the encoding is Identity.
        default: f64,
    },
    /// Adobe's own metrics for a standard-14 face the document named (decision #22).
    ///
    /// §9.6.2.2 lets a font dictionary name one of the standard 14 with no `/Widths` **because** a
    /// conforming reader is expected to hold these metrics, so they are known and merely absent
    /// from the file. That is the same standing [`Self::Cid`] already gives an omitted `/DW`.
    ///
    /// **Only for a face the document itself names.** `Arial` is not `Helvetica` and reaches
    /// [`Self::Absent`] as before — see [`crate::afm::for_base_font`].
    Standard14 {
        /// The Core-14 face, as matched. Carried so a trace names the face rather than a pointer.
        face: &'static str,
        /// The vendored metrics for it.
        metrics: &'static crate::afm::Core14,
    },
    /// No width information in the document, and none this profile may supply.
    ///
    /// Reached by a font that is not one of the standard 14 — where supplying metrics would be a
    /// *substitution* rather than a reading — and by a standard-14 code this profile's encoding
    /// tables do not cover. The advance is reported as unknown rather than guessed. Origins remain
    /// exact: they come from the content stream, not from the font.
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
/// `docs/history/09-V1-MILESTONES.md` S6.1. The kind is read from the document's own `/Subtype`, which was
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
    /// Set when this font declares itself symbolic, supplies no `/ToUnicode` and names no base
    /// encoding — so its codes were resolved through `StandardEncoding`, which PDF 32000-1
    /// §9.6.6.2 specifies for a NONSYMBOLIC font. Carries the base font name for the
    /// document-scoped limitation's detail, the same shape `WidthSource::Absent` uses.
    pub builtin_encoding_assumed: Option<String>,
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
    pub fn decode_code(&self, code: u32) -> Result<&str, EngineError> {
        // Borrowed, not owned: this is the innermost loop of extraction — one call
        // per glyph across every document — and both decoders can lend from their
        // tables. The error message is built only on the failure path.
        match &self.decoder {
            Decoder::ToUnicode(t) => t.get(code).ok_or_else(|| EngineError::Unsupported {
                what: "character code".into(),
                detail: format!(
                    "font /{} has a ToUnicode CMap with no entry for code {code:#04x}",
                    self.id
                ),
            }),
            Decoder::Simple(e) => {
                let byte = u8::try_from(code).map_err(|_| EngineError::Malformed {
                    what: "character code".into(),
                    detail: format!("code {code:#x} exceeds one byte in a simple font"),
                })?;
                e.decode(byte)
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
            // Decision #22. Ask this font's own decoder what the code means and look that up, so
            // one arm covers every encoding the profile already reads — WinAnsi, Standard,
            // `/Differences`, `ToUnicode`. Where the document names no base encoding the AFM's
            // own `C` column IS the answer, and for Symbol and ZapfDingbats it is the only one,
            // because their codes are not StandardEncoding.
            WidthSource::Standard14 { metrics, .. } => {
                if let Decoder::Simple(enc) = &self.decoder {
                    if enc.base() == BaseEncoding::Builtin {
                        if let Some(w) = metrics.advance_for_builtin_code(code) {
                            return Some(w);
                        }
                    }
                    // The document names a glyph and the AFM holds that glyph's width, so this
                    // asks the question directly instead of round-tripping through characters.
                    // It is also the only route that reaches a code above ASCII, where
                    // `StandardEncoding`'s table stops — `21` measured that gap and refused to
                    // close it by hand; `winansi_names` closes it by derivation.
                    if let Some(w) = u8::try_from(code)
                        .ok()
                        .and_then(|b| enc.glyph_name(b))
                        .and_then(|n| metrics.advance_for_glyph(n))
                    {
                        return Some(w);
                    }
                }
                metrics.advance_for_text(self.decode_code(code).ok()?)
            }
            // v2.2-S3. The CID is the code, because `load_cid_widths` refuses to build this
            // variant under any encoding where that is not true by definition. That refusal is
            // what lets this arm be three lines instead of a CMap parser.
            WidthSource::Cid { spans, default } => {
                let width = spans
                    .range(..=code)
                    .next_back()
                    .filter(|(_, (last, _))| code <= *last)
                    .map_or(*default, |(_, (_, w))| *w);
                Some(width / GLYPH_SPACE_UNITS)
            }
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
    doc: &crate::document::Document,
    page_dict: &lopdf::Dictionary,
) -> Result<BTreeMap<String, Arc<Font>>, EngineError> {
    let mut out = BTreeMap::new();

    let Some(resources) = resolve_dict(doc.inner(), page_dict.get(b"Resources").ok()) else {
        return Ok(out);
    };
    let Some(fonts) = resolve_dict(doc.inner(), resources.get(b"Font").ok()) else {
        return Ok(out);
    };

    for (name, value) in fonts.iter() {
        let id = String::from_utf8_lossy(name).to_string();
        // Fonts are shared document-wide via inherited /Resources, and parsing one
        // inflates and reads its /ToUnicode CMap and embedded font program — so the
        // parse is cached on the document, keyed by (object id, resource name).
        // Only referenced fonts can be cached: an inline dictionary has no object
        // id, and gets parsed per page as before.
        let reference = match value {
            lopdf::Object::Reference(oid) => Some(*oid),
            _ => None,
        };
        if let Some(oid) = reference {
            if let Some(cached) = doc.cached_font(oid, &id) {
                out.insert(id, cached);
                continue;
            }
        }
        let Some(fd) = resolve_dict(doc.inner(), Some(value)) else {
            return Err(EngineError::Malformed {
                what: "font resource".into(),
                detail: format!("/{id} does not resolve to a dictionary"),
            });
        };
        let font = Arc::new(load_font(doc.inner(), &id, &fd)?);
        if let Some(oid) = reference {
            doc.cache_font(oid, &id, Arc::clone(&font));
        }
        out.insert(id, font);
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

    // `/BaseFont` was deliberately not read from M7 until decision #22, on the reasoning that a
    // font name "would only be useful for substituting metrics this profile refuses to
    // substitute". Decision #22 drew the line one step in from there: reading Helvetica's own
    // metrics for a font the document CALLS Helvetica is reading the document, and only
    // supplying them for `Arial` is the substitution that stays refused. So the name now has two
    // readers — the standard-14 lookup below, and `base_font_detail` for limitation details.

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
    let mut widths = load_widths(doc, fd, &subtype, id);

    // 3. Ink metrics.
    let mut ink = crate::metrics::resolve_font_ink(doc, fd);

    // 3b. Decision #22: a standard-14 face the document named, whose metrics §9.6.2.2 expects a
    //     reader to hold. The two fallbacks are independent because the gaps are — a font can
    //     declare a descriptor and no `/Widths`, or the reverse — and each only ever fills a
    //     typed absence, so nothing the document actually stated is overwritten.
    //
    //     Composite fonts are excluded: their advances are CID-keyed via `/W` and `/DW`, so an
    //     AFM code lookup would be measuring a different thing under the same name.
    if matches!(FontKind::from_subtype(&subtype), FontKind::Simple) {
        if let Some(metrics) = base_font_name(fd)
            .as_deref()
            .and_then(crate::afm::for_base_font)
        {
            if matches!(widths, WidthSource::Absent { .. }) {
                widths = WidthSource::Standard14 {
                    face: metrics.face,
                    metrics,
                };
            }
            if matches!(ink, FontInk::Absent(_)) {
                ink = FontInk::Measured {
                    ascent: metrics.ascent,
                    descent: metrics.descent,
                    source: metrics.vertical_source,
                };
            }
        }
    }

    // 4. Did this font get `StandardEncoding` without the specification licensing it? PDF 32000-1
    //    §9.6.6.2 names that fallback for a NONSYMBOLIC font; Table 123 puts Symbolic at bit 3
    //    and Nonsymbolic at bit 6. A font that sets the first, sets no `/ToUnicode` and names no
    //    base is being decoded through a table it never asked for — reported, never dropped.
    let builtin_encoding_assumed = match &decoder {
        Decoder::Simple(enc) if enc.base() == BaseEncoding::Builtin && is_symbolic(doc, fd) => {
            Some(base_font_detail(fd))
        }
        _ => None,
    };

    Ok(Font {
        id: id.to_string(),
        kind: FontKind::from_subtype(&subtype),
        decoder,
        widths,
        builtin_encoding_assumed,
        ink,
    })
}

/// Whether the font's descriptor sets the Symbolic flag and not the Nonsymbolic one.
///
/// PDF 32000-1 Table 123: bit position 3 (value 4) is Symbolic, bit position 6 (value 32) is
/// Nonsymbolic. A descriptor that sets both is contradicting itself and is read as nonsymbolic,
/// because that is the reading under which this profile's fallback is licensed.
fn is_symbolic(doc: &lopdf::Document, fd: &lopdf::Dictionary) -> bool {
    const SYMBOLIC: i64 = 4;
    const NONSYMBOLIC: i64 = 32;
    let Some(descriptor) = resolve_dict(doc, fd.get(b"FontDescriptor").ok()) else {
        return false;
    };
    let Ok(flags) = descriptor.get(b"Flags").and_then(|o| o.as_i64()) else {
        return false;
    };
    flags & SYMBOLIC != 0 && flags & NONSYMBOLIC == 0
}

/// `BaseFont NAME` for a limitation detail, or a fixed string when the font declares none.
/// `/BaseFont` as the document spells it, with no interpretation.
///
/// Separate from [`base_font_detail`], which builds a human sentence for a limitation's `detail`.
/// This one is read by the standard-14 lookup and must stay the raw name.
fn base_font_name(fd: &lopdf::Dictionary) -> Option<String> {
    match fd.get(b"BaseFont") {
        Ok(lopdf::Object::Name(n)) => Some(String::from_utf8_lossy(n).to_string()),
        _ => None,
    }
}

fn base_font_detail(fd: &lopdf::Dictionary) -> String {
    match fd.get(b"BaseFont") {
        Ok(lopdf::Object::Name(n)) => format!("BaseFont {}", String::from_utf8_lossy(n)),
        _ => "a font declaring no /BaseFont".to_string(),
    }
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
                // v2.2-S8. Two different absences reach here, and saying the wrong one sends a
                // reader after data that would not help. An Identity CMap needs no vendoring at
                // all: §9.7.4.2 makes it the identity, so the code IS the CID. What is missing
                // there is the step AFTER that one.
                detail: if matches!(n.as_slice(), b"Identity-H" | b"Identity-V") {
                    format!(
                        "/Encoding /{} maps character codes to CIDs by the identity (§9.7.4.2), \
                         so nothing about the CMap is missing — the code IS the CID. What is \
                         absent is the step after it: this font supplies no `/ToUnicode`, and \
                         there is no other source here for CID to Unicode. Adobe publishes such \
                         a mapping per registry and ordering, and this profile carries none of \
                         them; where the descendant's /CIDSystemInfo ordering is \
                         `Adobe-Identity-0` the CIDs are the subset font's own and no published \
                         table decodes them either — only the embedded font program or a \
                         `/ToUnicode` can. **Vendoring the predefined CJK CMaps would not change \
                         this document**, which is what the message here used to imply. A \
                         substituted character is a character the document does not contain, so \
                         the page is refused rather than decoded approximately.",
                        String::from_utf8_lossy(n)
                    )
                } else {
                    format!(
                        "/Encoding /{} is not a simple encoding this profile carries. Predefined \
                         CMaps (the Adobe CJK set) are not vendored; a document needing one is \
                         refused rather than decoded approximately.",
                        String::from_utf8_lossy(n)
                    )
                },
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

/// One number from a PDF object, integer or real.
fn number(obj: Option<&lopdf::Object>) -> Option<f64> {
    match obj? {
        lopdf::Object::Integer(i) => Some(*i as f64),
        lopdf::Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

/// A composite font's widths, from its descendant CIDFont (v2.2-S3, PDF 32000-1 §9.7.4.3).
///
/// # Why this refuses every encoding but Identity
///
/// `/W` is keyed by **CID**, and `advance_glyph_space` is given a **character code**. The map
/// between them is the CMap named by `/Encoding`, and this profile parses no CMaps — the standing
/// interim [`Font::split_codes`] already declares as `composite-font-codes-from-tounicode`. Under
/// `Identity-H` and `Identity-V` the map is the identity by definition (§9.7.4.2), so the code IS
/// the CID and no CMap is needed. Under anything else the CID is unknown, and a width looked up
/// with the wrong key is worse than no width: it is a plausible number, and this engine's whole
/// posture is that a plausible number is the one failure a consumer cannot detect.
///
/// The restriction costs nothing measurable. All **78** composite fonts on the 200-document
/// `opendataloader-bench` corpus declare `Identity-H` — which is the claim
/// [`Font::split_codes`]'s comment already made in prose (*"right for Identity-H, which is what
/// real documents overwhelmingly use"*) and that this is the first slice to put a number on.
fn load_cid_widths(doc: &lopdf::Document, fd: &lopdf::Dictionary, id: &str) -> WidthSource {
    let encoding = fd.get(b"Encoding").ok().and_then(|o| o.as_name().ok());
    if !matches!(encoding, Some(b"Identity-H") | Some(b"Identity-V")) {
        return WidthSource::Absent {
            reason: format!(
                "font /{id} (Type0{}) declares /Encoding {}, and this profile parses no CMaps, so \
                 the mapping from character code to CID is unknown. `/W` is keyed by CID, so a \
                 width read under this encoding would be a plausible number for the wrong glyph \
                 — the one failure a consumer cannot detect. The advance is therefore reported \
                 as ABSENT rather than guessed. **This is not the standard-14 case and nothing \
                 here is waiting on vendored AFM tables**; that sentence belongs to a simple \
                 font with no /Widths. Origins are unaffected — they come from the content \
                 stream.",
                base_font_suffix(fd),
                encoding.map_or_else(
                    || "no name this reader could read".to_string(),
                    |n| format!("/{}", String::from_utf8_lossy(n))
                ),
            ),
        };
    }

    let Some(descendant) = resolve_array(doc, fd.get(b"DescendantFonts").ok())
        .and_then(|arr| arr.first().and_then(|o| resolve_dict(doc, Some(o))))
    else {
        return WidthSource::Absent {
            reason: format!(
                "font /{id} (Type0{}) declares an Identity encoding but no /DescendantFonts array \
                 this reader could resolve, so there is no CIDFont to read /W or /DW from. \
                 §9.7.4.1 requires exactly one descendant. The advance is ABSENT rather than \
                 assumed, and the document is malformed rather than this profile limited.",
                base_font_suffix(fd),
            ),
        };
    };

    // §9.7.4.3: `/DW` defaults to 1000 when the document omits it. Reading a normative default is
    // reading the document, not guessing at it — the same standing as an absent `/Encoding` on a
    // simple font meaning the built-in one. So from here the advance is always known.
    let default = number(descendant.get(b"DW").ok()).unwrap_or(1000.0);

    let mut spans: BTreeMap<u32, (u32, f64)> = BTreeMap::new();
    if let Some(w) = resolve_array(doc, descendant.get(b"W").ok()) {
        let mut i = 0usize;
        while i < w.len() {
            let Some(first) = number(w.get(i)) else { break };
            let Ok(first) = u32::try_from(first as i64) else {
                break;
            };
            // Form one: `c [w1 w2 … wn]` — consecutive CIDs from c. Form two:
            // `c_first c_last w` — one width across a range. They interleave inside one array,
            // so the shape of the NEXT object decides which this is; a parser that assumed one
            // form would read the other's numbers as CIDs and silently produce garbage spans.
            match w.get(i + 1) {
                Some(lopdf::Object::Array(items)) => {
                    for (k, item) in items.iter().enumerate() {
                        if let Some(width) = number(Some(item)) {
                            let cid = first.saturating_add(k as u32);
                            spans.insert(cid, (cid, width));
                        }
                    }
                    i += 2;
                }
                Some(_) => {
                    let (Some(last), Some(width)) = (number(w.get(i + 1)), number(w.get(i + 2)))
                    else {
                        break;
                    };
                    let Ok(last) = u32::try_from(last as i64) else {
                        break;
                    };
                    if last >= first {
                        spans.insert(first, (last, width));
                    }
                    i += 3;
                }
                None => break,
            }
        }
    }

    WidthSource::Cid { spans, default }
}

/// `, BaseFontName` for a limitation detail, or nothing when the font declares none.
fn base_font_suffix(fd: &lopdf::Dictionary) -> String {
    fd.get(b"BaseFont")
        .ok()
        .and_then(|o| o.as_name().ok())
        .map(|n| format!(", {}", String::from_utf8_lossy(n)))
        .unwrap_or_default()
}

fn load_widths(
    doc: &lopdf::Document,
    fd: &lopdf::Dictionary,
    subtype: &str,
    id: &str,
) -> WidthSource {
    // v2.2-S3. **The dispatch that was missing.** A composite font's widths live somewhere else
    // entirely and are keyed by something else entirely; reading `/Widths` off a `Type0`
    // dictionary asks a question the format never answers there.
    if subtype == "Type0" {
        return load_cid_widths(doc, fd, id);
    }

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
                base_font_suffix(fd),
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
            builtin_encoding_assumed: None,
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
    // --- v2.2-S3: the composite width table -------------------------------------------------

    /// `/W` array elements, spelled the way the array reads.
    ///
    /// `/W [ 1 [500 750] 5 7 250 ]` becomes `vec![n(1), a(&[500, 750]), n(5), n(7), n(250)]`, so
    /// a reader of these tests compares them against §9.7.4.3's two forms directly.
    fn n(v: i64) -> lopdf::Object {
        lopdf::Object::Integer(v)
    }
    fn a(v: &[i64]) -> lopdf::Object {
        lopdf::Object::Array(v.iter().copied().map(n).collect())
    }

    /// A `/Type0` font and its descendant, as one `lopdf::Document`.
    ///
    /// Built here rather than as a fixture because two cases below cannot both be one:
    /// `fixtures/engine/make_fixtures.py` gives every fixture exactly one font, and `/DW` present
    /// and `/DW` absent are different fonts. The fixture proves the whole path end to end; these
    /// prove the table.
    fn widths_of(encoding: &str, dw: Option<i64>, w: Option<Vec<lopdf::Object>>) -> WidthSource {
        let mut doc = lopdf::Document::with_version("1.7");
        let mut desc = lopdf::Dictionary::new();
        desc.set("Subtype", lopdf::Object::Name(b"CIDFontType2".to_vec()));
        if let Some(dw) = dw {
            desc.set("DW", lopdf::Object::Integer(dw));
        }
        if let Some(w) = w {
            desc.set("W", lopdf::Object::Array(w));
        }
        let id = doc.add_object(desc);

        let mut font = lopdf::Dictionary::new();
        font.set("Type", lopdf::Object::Name(b"Font".to_vec()));
        font.set("Subtype", lopdf::Object::Name(b"Type0".to_vec()));
        font.set(
            "Encoding",
            lopdf::Object::Name(encoding.as_bytes().to_vec()),
        );
        font.set(
            "DescendantFonts",
            lopdf::Object::Array(vec![lopdf::Object::Reference(id)]),
        );
        load_cid_widths(&doc, &font, "F1")
    }

    /// **Both `/W` forms, and they interleave inside one array** (PDF 32000-1 §9.7.4.3).
    ///
    /// A parser that knew one form would read the other's numbers as CIDs and build silently
    /// wrong spans — no error, no refusal, just a plausible advance for the wrong glyph. Both
    /// occur in the wild: 1 143 array-form and 810 range-form entries across the 200-document
    /// `opendataloader-bench` corpus.
    #[test]
    fn both_w_forms_are_read_and_may_interleave() {
        let w = widths_of(
            "Identity-H",
            Some(900),
            Some(vec![
                n(1),
                a(&[500, 750]),
                n(5),
                n(7),
                n(250),
                n(10),
                a(&[111]),
            ]),
        );
        assert!(matches!(w, WidthSource::Cid { .. }), "got {w:?}");

        // **Through `advance_glyph_space`, not through a local copy of it.** The first draft of
        // this test walked `spans` with its own `range(..=cid).next_back()` — a reimplementation
        // of the lookup it exists to check, which a mutant making the range end EXCLUSIVE
        // survived here and died only in the integration test. A guard that reads its own
        // subject through a private copy of the subject is this repository's recurring defect,
        // and writing one inside the slice that fixes an instance of it would be a poor joke.
        let f = font_with(w, FontInk::Absent(GeometryAbsence::NotReportedByReader));
        let at = |cid: u32| f.advance_glyph_space(cid).map(|a| a * GLYPH_SPACE_UNITS);
        assert_eq!(at(1), Some(500.0), "array form, first entry");
        assert_eq!(at(2), Some(750.0), "array form, second entry");
        assert_eq!(at(3), Some(900.0), "named by neither, so /DW");
        assert_eq!(at(5), Some(250.0), "range form, first CID");
        assert_eq!(
            at(7),
            Some(250.0),
            "range form, LAST CID — §9.7.4.3's range is INCLUSIVE at both ends"
        );
        assert_eq!(
            at(8),
            Some(900.0),
            "one past the range is /DW again, not 250"
        );
        assert_eq!(
            at(10),
            Some(111.0),
            "an array form AFTER a range form, in one array"
        );
    }

    /// **§9.7.4.3's default is 1000, and reading it is reading the document.**
    ///
    /// The one case the fixture cannot express, and the one a mutant survived while `/DW` was
    /// 1000 there: at that value an implementation that ignored the default entirely still
    /// produced the right number. An absent `/DW` is a declared value the same way an absent
    /// `/Encoding` on a simple font declares the built-in one — so this is not a guess, and a
    /// composite font under an Identity CMap therefore has no width-absent case at all.
    #[test]
    fn the_spec_default_applies_when_dw_is_absent() {
        let w = widths_of("Identity-H", None, Some(vec![n(1), a(&[500])]));
        let WidthSource::Cid { spans, default } = &w else {
            panic!("expected Cid widths, got {w:?}")
        };
        assert_eq!(*default, 1000.0, "§9.7.4.3: \"Default value: 1000\"");
        assert_eq!(spans.len(), 1, "only what /W names is a span");

        // And with no `/W` at all the font is still fully described.
        let bare = widths_of("Identity-H", None, None);
        let WidthSource::Cid { spans, default } = &bare else {
            panic!("expected Cid widths, got {bare:?}")
        };
        assert!(spans.is_empty());
        assert_eq!(*default, 1000.0);
    }

    /// **Anything but Identity refuses, and says why.**
    ///
    /// `/W` is keyed by CID; the code→CID map is the `/Encoding` CMap, and this profile parses
    /// none. Returning widths anyway would produce a plausible number for the wrong glyph, which
    /// is the single failure a consumer cannot detect. The reason must also NOT be the
    /// standard-14 sentence — that was the wrong explanation printed on 50 of the 51 corpus
    /// documents that printed it.
    #[test]
    fn a_cmap_this_profile_cannot_read_refuses_rather_than_guessing() {
        for encoding in ["UniJIS-UCS2-H", "GBK-EUC-H", "90ms-RKSJ-H"] {
            let w = widths_of(encoding, Some(900), Some(vec![n(1), a(&[500])]));
            let WidthSource::Absent { reason } = &w else {
                panic!("{encoding} is not Identity and must refuse, got {w:?}")
            };
            assert!(reason.contains(encoding), "the reason names it: {reason}");
            assert!(
                reason.contains("not the standard-14 case"),
                "and disowns the AFM sentence: {reason}"
            );
        }
        // Identity-V is Identity too — vertical writing does not change the CID mapping.
        assert!(matches!(
            widths_of("Identity-V", Some(900), None),
            WidthSource::Cid { .. }
        ));
    }

    /// A `/Type0` with no descendant is malformed, and says that rather than blaming a table.
    #[test]
    fn a_composite_font_with_no_descendant_is_named_as_malformed() {
        let mut font = lopdf::Dictionary::new();
        font.set("Subtype", lopdf::Object::Name(b"Type0".to_vec()));
        font.set("Encoding", lopdf::Object::Name(b"Identity-H".to_vec()));
        let doc = lopdf::Document::with_version("1.7");
        let WidthSource::Absent { reason } = load_cid_widths(&doc, &font, "F1") else {
            panic!("no descendant, no widths")
        };
        assert!(reason.contains("/DescendantFonts"), "{reason}");
        assert!(
            reason.contains("malformed rather than this profile limited"),
            "the fault is the document's and the message says so: {reason}"
        );
    }

    /// A font dictionary with a descriptor carrying `flags`, and whatever `/Encoding` is given.
    fn symbolic_font(flags: i64, encoding: Option<lopdf::Object>) -> lopdf::Dictionary {
        let mut desc = lopdf::Dictionary::new();
        desc.set("Flags", lopdf::Object::Integer(flags));
        let mut fd = lopdf::Dictionary::new();
        fd.set("Subtype", lopdf::Object::Name(b"Type1".to_vec()));
        fd.set("BaseFont", lopdf::Object::Name(b"ABCDEF+CMEX10".to_vec()));
        fd.set("FontDescriptor", lopdf::Object::Dictionary(desc));
        if let Some(enc) = encoding {
            fd.set("Encoding", enc);
        }
        fd
    }

    /// **A symbolic font decoded through `StandardEncoding` says so** (v2.2-S6).
    ///
    /// PDF 32000-1 §9.6.6.2 gives that fallback to a NONSYMBOLIC font. Applying it to a symbolic
    /// one may produce the wrong character — CMEX10 code 90 is `integraldisplay` and arrives as
    /// `Z` — and until this the artifact reported `scalar_code_mismatch: false` and nothing else.
    #[test]
    fn a_symbolic_font_with_no_tounicode_and_no_base_declares_the_assumption() {
        let fd = symbolic_font(4, None);
        let font = load_font(&lopdf::Document::new(), "F1", &fd).expect("loads");
        assert_eq!(
            font.builtin_encoding_assumed.as_deref(),
            Some("BaseFont ABCDEF+CMEX10"),
            "the assumption must be declared, and name the font it was made about"
        );
    }

    /// **A named base encoding is the document telling us which table to use**, so nothing is
    /// assumed and nothing is declared — even though the font is flagged symbolic.
    ///
    /// Not hypothetical: 4 of the 42 OmniDocBench documents whose fonts set the symbolic bit
    /// carry `/Encoding << /BaseEncoding /WinAnsiEncoding /Differences [...] >>`, and their text
    /// decodes correctly. A rule that fired on the flag alone would have libelled them.
    #[test]
    fn a_symbolic_font_that_names_a_base_encoding_declares_nothing() {
        let mut enc = lopdf::Dictionary::new();
        enc.set(
            "BaseEncoding",
            lopdf::Object::Name(b"WinAnsiEncoding".to_vec()),
        );
        let fd = symbolic_font(4, Some(lopdf::Object::Dictionary(enc)));
        let font = load_font(&lopdf::Document::new(), "F1", &fd).expect("loads");
        assert_eq!(font.builtin_encoding_assumed, None);
    }

    /// **A nonsymbolic font is exactly the case §9.6.6.2 licenses**, so it declares nothing.
    #[test]
    fn a_nonsymbolic_font_declares_nothing() {
        let fd = symbolic_font(32, None);
        let font = load_font(&lopdf::Document::new(), "F1", &fd).expect("loads");
        assert_eq!(font.builtin_encoding_assumed, None);
    }

    /// **A descriptor claiming both flags is read as nonsymbolic**, because that is the reading
    /// under which the fallback this profile already applies is licensed.
    #[test]
    fn a_font_flagged_both_symbolic_and_nonsymbolic_declares_nothing() {
        let fd = symbolic_font(4 | 32, None);
        let font = load_font(&lopdf::Document::new(), "F1", &fd).expect("loads");
        assert_eq!(font.builtin_encoding_assumed, None);
    }

    /// **An Identity CMap is refused for a different reason than a predefined CJK one, and says
    /// so** (v2.2-S8).
    ///
    /// Both reach the same refusal, and until this slice both blamed the unvendored Adobe CJK
    /// set. That is true of `/GBK-EUC-H` and false of `/Identity-H`: §9.7.4.2 makes the identity
    /// CMap the identity, so nothing about it is missing. 8 of the 20 OmniDocBench documents that
    /// produce no artifact are the second kind, and the old message would have sent a reader
    /// after a dataset that could not have helped them.
    #[test]
    fn an_identity_cmap_does_not_blame_the_unvendored_cjk_set() {
        for name in [b"Identity-H".as_slice(), b"Identity-V".as_slice()] {
            let mut fd = lopdf::Dictionary::new();
            fd.set("Subtype", lopdf::Object::Name(b"Type0".to_vec()));
            fd.set("Encoding", lopdf::Object::Name(name.to_vec()));
            let err = load_font(&lopdf::Document::new(), "F1", &fd)
                .expect_err("no /ToUnicode and no simple encoding is still a refusal");
            let detail = format!("{err}");
            assert!(
                detail.contains("the code IS the CID"),
                "an identity CMap must be refused for its own reason: {detail}"
            );
            assert!(
                !detail.contains("are not vendored"),
                "an identity CMap needs no vendored data, so it must not blame it: {detail}"
            );
        }
    }

    /// **A genuine predefined CJK CMap still names the unvendored set**, which for it is the
    /// truthful answer.
    #[test]
    fn a_predefined_cjk_cmap_still_names_the_unvendored_set() {
        let mut fd = lopdf::Dictionary::new();
        fd.set("Subtype", lopdf::Object::Name(b"Type0".to_vec()));
        fd.set("Encoding", lopdf::Object::Name(b"GBK-EUC-H".to_vec()));
        let err = load_font(&lopdf::Document::new(), "F1", &fd).expect_err("refused");
        let detail = format!("{err}");
        assert!(detail.contains("are not vendored"), "{detail}");
        assert!(!detail.contains("the code IS the CID"), "{detail}");
    }
}
