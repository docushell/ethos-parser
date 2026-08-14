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

//! Image XObjects: where one was painted, and which bytes it is (v1-S6).
//!
//! Part of [`engine_core::OBSERVATION_RULE_V1`].
//!
//! # What this module refuses to do
//!
//! **It does not decode.** Not one filter is run. The digest covers the stream exactly as the
//! file stores it, because those are the bytes that are in the document — decoding first would
//! make the fingerprint depend on this engine's inflate implementation, and two readers
//! disagreeing about what one file contains is the property a fingerprint exists to deny.
//!
//! **It does not describe.** There is no field for a caption, an alt text, or a characterization,
//! and adding one would need a slice of its own with a very different argument: text read out of
//! pixels is OCR, which v1 does not do, and a model's account of a picture is not evidence
//! (checklist O20).
//!
//! **It does not measure a box from pixel dimensions.** `/Width` and `/Height` are counts of
//! samples; the area a placement covers is the page's own matrix applied to the unit square. They
//! are different quantities in different units, and conflating them is the pdf-inspector defect
//! (`height` and the type size being one variable) in another costume.

use std::collections::BTreeMap;

use engine_core::{ImageAttributes, ImageMediaType, PaintedRect, QRect, Sha256Hex};

/// The page's `/XObject` resources, by resource name.
///
/// Names only — the subtype is resolved later, against the document. An empty map means the page
/// declares no XObjects, which is a different answer from not having looked.
pub fn page_xobjects(
    doc: &lopdf::Document,
    page_dict: &lopdf::Dictionary,
) -> BTreeMap<String, lopdf::ObjectId> {
    let mut out = BTreeMap::new();
    let Some(resources) = crate::fonts::resolve_dict(doc, page_dict.get(b"Resources").ok()) else {
        return out;
    };
    let Some(xobjects) = crate::fonts::resolve_dict(doc, resources.get(b"XObject").ok()) else {
        return out;
    };
    for (name, value) in xobjects.iter() {
        // Only an indirect reference gives an object number, and an object number is half of
        // what an image node's address IS. A directly-embedded stream has no id to cite, so it
        // is left unresolved and counted rather than given a fabricated one.
        if let lopdf::Object::Reference(id) = value {
            out.insert(String::from_utf8_lossy(name).into_owned(), *id);
        }
    }
    out
}

/// Whether an XObject is an image, and its facts, or `None` when it is not one.
///
/// A `/Form` returns `None` — this profile does not descend into form XObjects, and
/// `form-xobject-text-not-descended` is where that has been declared since M4. Anything with no
/// readable `/Subtype` also returns `None`: guessing that an unlabelled stream is a picture would
/// put a node on the wire for something the document did not call an image.
pub fn image_attributes(doc: &lopdf::Document, id: lopdf::ObjectId) -> Option<ImageAttributes> {
    let stream = doc.get_object(id).ok()?.as_stream().ok()?;
    let subtype = stream.dict.get(b"Subtype").ok()?.as_name().ok()?;
    if subtype != b"Image" {
        return None;
    }

    // The bytes AS STORED. `stream.content` is what the file holds; nothing here calls
    // `decompressed_content`, and that is the whole point of the digest.
    //
    // `of_bytes` rather than parsing a formatted string: hashing cannot fail, and the fallible
    // spelling turned every image node into a silent `None` the first time this ran.
    let bytes = stream.content.as_slice();
    let stream_sha256 = Sha256Hex::of_bytes(bytes);

    let filters = filter_names(doc, &stream.dict);
    let media_type = media_type_for(&filters);

    Some(ImageAttributes {
        stream_sha256,
        stream_bytes: bytes.len() as u64,
        filters,
        media_type,
        pixel_width: positive_u32(doc, &stream.dict, b"Width"),
        pixel_height: positive_u32(doc, &stream.dict, b"Height"),
        image_mask: matches!(
            crate::fonts::resolve_object(doc, stream.dict.get(b"ImageMask").ok()),
            Some(lopdf::Object::Boolean(true))
        ),
    })
}

/// The `/Filter` chain, outermost first, as the document spells it.
///
/// A single name and an array of names are both legal (32000-1 §7.3.8.2) and both come back as a
/// list, so a consumer never has to handle two shapes.
fn filter_names(doc: &lopdf::Document, dict: &lopdf::Dictionary) -> Vec<String> {
    let name = |o: &lopdf::Object| -> Option<String> {
        o.as_name()
            .ok()
            .map(|n| String::from_utf8_lossy(n).into_owned())
    };
    match crate::fonts::resolve_object(doc, dict.get(b"Filter").ok()) {
        Some(lopdf::Object::Name(n)) => vec![String::from_utf8_lossy(&n).into_owned()],
        Some(lopdf::Object::Array(a)) => a
            .iter()
            .filter_map(|o| match crate::fonts::resolve_object(doc, Some(o)) {
                Some(ref r) => name(r),
                None => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether the stored bytes are a file in their own right.
///
/// Decided from the **outermost** filter, which is the last one applied and therefore the format
/// the bytes are actually in. Only two filters produce a standalone file; everything else is
/// PDF-specific sample data whose meaning needs `/ColorSpace` and `/BitsPerComponent` from the
/// same dictionary, and calling that `image/png` because it happens to be deflated would send a
/// consumer to open something as a format it is not.
///
/// Nothing is sniffed from the payload. Guessing a type from leading bytes is a decoder's job,
/// and a wrong guess here is worse than no answer.
fn media_type_for(filters: &[String]) -> ImageMediaType {
    match filters.last().map(String::as_str) {
        Some("DCTDecode") => ImageMediaType::Standalone("image/jpeg".into()),
        Some("JPXDecode") => ImageMediaType::Standalone("image/jp2".into()),
        _ => ImageMediaType::PdfEncodedSamples,
    }
}

/// A positive `/Width` or `/Height`, or `None`.
///
/// Absent rather than zero when the key is missing or unreadable: zero is a legal-looking value
/// that reads as a degenerate image, and an unread field must not be able to look like a read one.
fn positive_u32(doc: &lopdf::Document, dict: &lopdf::Dictionary, key: &[u8]) -> Option<u32> {
    match crate::fonts::resolve_object(doc, dict.get(key).ok()) {
        Some(lopdf::Object::Integer(v)) if v > 0 => u32::try_from(v).ok(),
        _ => None,
    }
}

/// The rectangle a placement covered, in the artifact's coordinate system.
///
/// `corners` are the transformed unit square in user space, already mapped into display space by
/// the caller. Three answers, and the two that are not a rectangle are typed rather than dropped:
/// an image whose placement this profile cannot express is still an image that was drawn.
pub fn painted_rect(corners: [(f64, f64); 4]) -> PaintedRect {
    if corners
        .iter()
        .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return PaintedRect::Malformed;
    }
    // Axis-aligned means the transformed unit square still has its sides parallel to the axes:
    // two distinct x values and two distinct y values across the four corners. A rotation or a
    // skew produces more, and its bounding box would claim area the picture does not cover.
    let q = |v: f64| engine_core::quantize(v, engine_core::QUANTUM_PER_POINT).ok();
    let Some(qs) = corners
        .iter()
        .map(|&(x, y)| Some((q(x)?, q(y)?)))
        .collect::<Option<Vec<_>>>()
    else {
        return PaintedRect::Malformed;
    };
    let xs: std::collections::BTreeSet<i64> = qs.iter().map(|&(x, _)| x).collect();
    let ys: std::collections::BTreeSet<i64> = qs.iter().map(|&(_, y)| y).collect();
    if xs.len() != 2 || ys.len() != 2 {
        return PaintedRect::NotAxisAligned;
    }
    let (x0, x1) = (*xs.first().expect("two"), *xs.last().expect("two"));
    let (y0, y1) = (*ys.first().expect("two"), *ys.last().expect("two"));
    match QRect::new(x0, y0, x1, y1) {
        Ok(r) => PaintedRect::Painted(r),
        Err(_) => PaintedRect::Malformed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_axis_aligned_placement_is_a_rectangle() {
        let c = [(10.0, 20.0), (110.0, 20.0), (110.0, 70.0), (10.0, 70.0)];
        match painted_rect(c) {
            PaintedRect::Painted(r) => {
                assert_eq!((r.x0(), r.y0(), r.x1(), r.y1()), (1000, 2000, 11000, 7000));
            }
            other => panic!("expected a rectangle, got {other:?}"),
        }
    }

    /// A rotated image covers a parallelogram, and its bounding box is bigger than the picture.
    #[test]
    fn a_rotated_placement_is_typed_absence_rather_than_a_bounding_box() {
        // The unit square under a 45° rotation.
        let s = std::f64::consts::FRAC_1_SQRT_2 * 100.0;
        let c = [(0.0, 0.0), (s, s), (0.0, 2.0 * s), (-s, s)];
        assert_eq!(painted_rect(c), PaintedRect::NotAxisAligned);
    }

    #[test]
    fn a_degenerate_placement_is_not_a_rectangle() {
        // A matrix that collapses the square to a line paints no area.
        let c = [(10.0, 20.0), (110.0, 20.0), (110.0, 20.0), (10.0, 20.0)];
        assert_eq!(painted_rect(c), PaintedRect::NotAxisAligned);
    }

    #[test]
    fn non_finite_corners_are_malformed_not_a_guess() {
        let c = [
            (f64::NAN, 0.0),
            (1.0, 0.0),
            (1.0, 1.0),
            (f64::INFINITY, 1.0),
        ];
        assert_eq!(painted_rect(c), PaintedRect::Malformed);
    }

    /// Only the two filters whose payload really is a file get a media type.
    #[test]
    fn a_media_type_is_claimed_only_where_the_bytes_are_a_file() {
        assert_eq!(
            media_type_for(&["DCTDecode".into()]),
            ImageMediaType::Standalone("image/jpeg".into())
        );
        assert_eq!(
            media_type_for(&["JPXDecode".into()]),
            ImageMediaType::Standalone("image/jp2".into())
        );
        for samples in [
            vec![],
            vec!["FlateDecode".to_string()],
            vec!["LZWDecode".to_string()],
            vec!["CCITTFaxDecode".to_string()],
            vec!["JBIG2Decode".to_string()],
            vec!["RunLengthDecode".to_string()],
        ] {
            assert_eq!(
                media_type_for(&samples),
                ImageMediaType::PdfEncodedSamples,
                "{samples:?} does not produce a standalone file"
            );
        }
        // The OUTERMOST filter decides: a JPEG wrapped in ASCII85 is not a JPEG on disk.
        assert_eq!(
            media_type_for(&["DCTDecode".into(), "ASCII85Decode".into()]),
            ImageMediaType::PdfEncodedSamples
        );
    }

    /// The digest covers stored bytes, so it must not change when a decoder would.
    #[test]
    fn the_module_never_decodes() {
        let src = include_str!("images.rs");
        let end = src.find("#[cfg(test)]").unwrap_or(src.len());
        let code: String = src[..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            code.contains("stream.content"),
            "the scan did not reach the byte source, so its claims below prove nothing"
        );
        for banned in ["decompressed_content", "decode_image", "decompress"] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the image module. The digest covers the stream AS STORED; \
                 decoding first would make the fingerprint depend on this engine's decoder, and \
                 two readers disagreeing about what one file contains is what a fingerprint \
                 exists to deny"
            );
        }
        // O20: no description, ever.
        for banned in ["alt_text", "caption", "description", "ocr"] {
            assert!(
                !code.contains(banned),
                "`{banned}` reached the image module. An image node says where a picture was and \
                 which bytes it is; what it SHOWS is OCR or a model's opinion, and neither is \
                 evidence"
            );
        }
    }
}
