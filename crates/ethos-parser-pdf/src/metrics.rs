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

//! Font metrics: measured, or typed-absent (`docs/01-CONTRACT.md` §5.2).
//!
//! # The one rule
//!
//! **A box is measured or it does not exist.** There is no fallback, no estimate, and above all
//! no `height = font_size`. pdf-inspector's `TextItem.height` is literally the same variable as
//! `font_size` (parity checklist P5); a font-size-derived box is closer to invented than
//! measured, and inventing a coordinate is forbidden outright.
//!
//! # Order of preference, and why
//!
//! 1. **The embedded font program**, read with `skrifa`. This is the font the document actually
//!    carries, so its ascent and descent are that font's own — design metrics for the whole font,
//!    not the ink of any one glyph.
//! 2. **`FontDescriptor` `/Ascent` and `/Descent`**, then `/FontBBox`. Author-declared rather
//!    than measured from outlines, but still *stated by the document* rather than by us. A Type 3
//!    font's envelope from either step is then gated on its `/FontMatrix` in `load_font`.
//! 3. **Nothing.** [`GeometryAbsence::NotReportedByReader`], declared and counted. For a simple
//!    font naming a standard-14 face, `load_font` step 3b then fills that absence from the
//!    vendored AFM of the face the document named (decision #22); nothing else refills it.
//!
//! Each step is narrower than the last, and the last answer here is honest rather than helpful.

use ethos_parser_core::GeometryAbsence;

use crate::fonts::{resolve_array, resolve_dict, resolve_stream, FontInk};

/// Resolve a font's ink extent.
///
/// Never fails: absence is a value. A run without metrics is still evidence, and its origin is
/// still exact.
pub fn resolve_font_ink(doc: &lopdf::Document, font_dict: &lopdf::Dictionary) -> FontInk {
    // Type 0 fonts hold their descriptor on the descendant.
    let descriptor_holder = resolve_array(doc, font_dict.get(b"DescendantFonts").ok())
        .and_then(|arr| arr.first().and_then(|o| resolve_dict(doc, Some(o))))
        .unwrap_or_else(|| font_dict.clone());

    let descriptor = resolve_dict(doc, descriptor_holder.get(b"FontDescriptor").ok());

    if let Some(desc) = &descriptor {
        if let Some(ink) = from_embedded_program(doc, desc) {
            return ink;
        }
        if let Some(ink) = from_descriptor(doc, desc) {
            return ink;
        }
    }

    FontInk::Absent(GeometryAbsence::NotReportedByReader)
}

/// Read ascent and descent from the embedded font program.
fn from_embedded_program(doc: &lopdf::Document, descriptor: &lopdf::Dictionary) -> Option<FontInk> {
    // /FontFile2 is TrueType; /FontFile3 may hold OpenType. /FontFile is Type1, which `skrifa`
    // does not read — that case falls through to the descriptor.
    let stream = resolve_stream(doc, descriptor.get(b"FontFile2").ok())
        .or_else(|| resolve_stream(doc, descriptor.get(b"FontFile3").ok()))?;

    let bytes = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());

    // `skrifa`, not `ttf-parser`: RUSTSEC-2026-0192 records that ttf-parser's author has
    // declared it unmaintained with no safe upgrade, and names skrifa (Google Fonts' fontations
    // project) as the maintained successor. Swapping beats ignoring the advisory.
    let face = skrifa::FontRef::new(&bytes).ok()?;
    let m = skrifa::MetadataProvider::metrics(
        &face,
        skrifa::instance::Size::unscaled(),
        skrifa::instance::LocationRef::default(),
    );

    let upem = f64::from(m.units_per_em);
    if upem <= 0.0 {
        return None;
    }

    // Normalise to the 1000-unit glyph space the rest of the pipeline uses, so a 2048-upem
    // TrueType face and a 1000-upem CFF face are directly comparable.
    let scale = 1000.0 / upem;
    let ascent = f64::from(m.ascent) * scale;
    let descent = f64::from(m.descent) * scale;

    if ascent <= 0.0 || descent >= 0.0 {
        // Implausible metrics are refused rather than passed through: an "ink box" with the
        // baseline outside it is worse than no box.
        return None;
    }

    Some(FontInk::Measured {
        ascent,
        descent,
        source: "embedded-font-program",
    })
}

/// Read ascent and descent from the `FontDescriptor`, falling back to `/FontBBox`.
fn from_descriptor(doc: &lopdf::Document, descriptor: &lopdf::Dictionary) -> Option<FontInk> {
    let num = |key: &[u8]| -> Option<f64> {
        match descriptor.get(key).ok()? {
            lopdf::Object::Integer(i) => Some(*i as f64),
            lopdf::Object::Real(r) => Some(f64::from(*r)),
            _ => None,
        }
    };

    if let (Some(a), Some(d)) = (num(b"Ascent"), num(b"Descent")) {
        if a > 0.0 && d < 0.0 {
            return Some(FontInk::Measured {
                ascent: a,
                descent: d,
                source: "font-descriptor",
            });
        }
    }

    // /FontBBox is [llx lly urx ury]; ury is the ink top, lly the ink bottom.
    let bbox = resolve_array(doc, descriptor.get(b"FontBBox").ok())?;
    if bbox.len() != 4 {
        return None;
    }
    let val = |i: usize| -> Option<f64> {
        match &bbox[i] {
            lopdf::Object::Integer(v) => Some(*v as f64),
            lopdf::Object::Real(v) => Some(f64::from(*v)),
            _ => None,
        }
    };
    let (lly, ury) = (val(1)?, val(3)?);
    if ury > 0.0 && lly < 0.0 {
        return Some(FontInk::Measured {
            ascent: ury,
            descent: lly,
            source: "font-descriptor",
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{Dictionary, Object};

    fn doc() -> lopdf::Document {
        lopdf::Document::new()
    }

    #[test]
    fn a_font_with_no_descriptor_yields_typed_absence() {
        // The standard-14 case: /Type /Font /Subtype /Type1 /BaseFont /Helvetica and nothing else.
        let mut fd = Dictionary::new();
        fd.set("Subtype", Object::Name(b"Type1".to_vec()));
        fd.set("BaseFont", Object::Name(b"Helvetica".to_vec()));

        assert_eq!(
            resolve_font_ink(&doc(), &fd),
            FontInk::Absent(GeometryAbsence::NotReportedByReader),
            "no metrics means no box — not a font-size-shaped one"
        );
    }

    #[test]
    fn ascent_and_descent_are_read_from_the_descriptor() {
        let mut desc = Dictionary::new();
        desc.set("Ascent", Object::Integer(718));
        desc.set("Descent", Object::Integer(-207));
        let mut fd = Dictionary::new();
        fd.set("FontDescriptor", Object::Dictionary(desc));

        assert_eq!(
            resolve_font_ink(&doc(), &fd),
            FontInk::Measured {
                ascent: 718.0,
                descent: -207.0,
                source: "font-descriptor",
            }
        );
    }

    #[test]
    fn font_bbox_is_the_fallback_when_ascent_is_missing() {
        let mut desc = Dictionary::new();
        desc.set(
            "FontBBox",
            Object::Array(vec![
                Object::Integer(-166),
                Object::Integer(-225),
                Object::Integer(1000),
                Object::Integer(931),
            ]),
        );
        let mut fd = Dictionary::new();
        fd.set("FontDescriptor", Object::Dictionary(desc));

        assert_eq!(
            resolve_font_ink(&doc(), &fd),
            FontInk::Measured {
                ascent: 931.0,
                descent: -225.0,
                source: "font-descriptor",
            }
        );
    }

    #[test]
    fn implausible_descriptor_metrics_are_refused() {
        // A descriptor claiming the baseline sits outside the ink is not usable, and passing it
        // through would produce a box that renders wrong while looking well formed.
        for (a, d) in [(0, 0), (-100, -200), (718, 207)] {
            let mut desc = Dictionary::new();
            desc.set("Ascent", Object::Integer(a));
            desc.set("Descent", Object::Integer(d));
            let mut fd = Dictionary::new();
            fd.set("FontDescriptor", Object::Dictionary(desc));
            assert_eq!(
                resolve_font_ink(&doc(), &fd),
                FontInk::Absent(GeometryAbsence::NotReportedByReader),
                "ascent {a}, descent {d} should be refused"
            );
        }
    }

    #[test]
    fn a_malformed_font_bbox_yields_absence() {
        let mut desc = Dictionary::new();
        desc.set("FontBBox", Object::Array(vec![Object::Integer(0)]));
        let mut fd = Dictionary::new();
        fd.set("FontDescriptor", Object::Dictionary(desc));
        assert_eq!(
            resolve_font_ink(&doc(), &fd),
            FontInk::Absent(GeometryAbsence::NotReportedByReader)
        );
    }

    #[test]
    fn an_unparseable_font_program_falls_through_to_the_descriptor() {
        let mut desc = Dictionary::new();
        desc.set("Ascent", Object::Integer(750));
        desc.set("Descent", Object::Integer(-250));
        desc.set(
            "FontFile2",
            Object::Stream(lopdf::Stream::new(
                Dictionary::new(),
                b"not a font".to_vec(),
            )),
        );
        let mut fd = Dictionary::new();
        fd.set("FontDescriptor", Object::Dictionary(desc));

        assert_eq!(
            resolve_font_ink(&doc(), &fd),
            FontInk::Measured {
                ascent: 750.0,
                descent: -250.0,
                source: "font-descriptor",
            },
            "a broken embedded program must not lose the descriptor's declared metrics"
        );
    }
}
