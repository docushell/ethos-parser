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

//! A scanned page, asked of a recognizer and answered as words on the page (v3,
//! `docs/35-OCR-SCOPE.md`).
//!
//! **This module decodes nothing.** It reads what the image's dictionary states — size, colour
//! space, filters and their parameters, `/Decode` — and hands the recognizer the stream exactly as
//! the file stores it; the recognizer decodes, and refuses by name what it cannot — save a filter
//! it has no decoder for, named here first by its own rule. What comes back
//! is mapped through the matrix the page painted the image with, which is the only arithmetic
//! here, and it is the same arithmetic `images.rs` applies to the unit square.
//!
//! The recognizer itself is behind [`Recognize`]: the engine never spawns a process, so this crate
//! stays testable with a recognizer that is a function, and the CLI owns the binary.

use ethos_parser_core::{
    quantize, EngineError, PdfObjectNumber, QRect, QRectError, RecognizerIdentity,
    QUANTUM_PER_POINT,
};
use serde::{Deserialize, Serialize};

use crate::fonts::{resolve_array, resolve_object, resolve_stream};

/// The recognizer `--ocr` reads a scanned page with. Implemented by the CLI over
/// `ethos-parser-ocr serve`; by a function in tests.
pub trait Recognize {
    /// The identity the recognizer reports. Extraction refuses to start unless it is the one the
    /// profile pins.
    fn identity(&self) -> &RecognizerIdentity;
    /// Read one image. `Err` is a failure of the recognizer itself (it exited, it answered out of
    /// protocol), which ends the extraction; a refusal to read an image is an answer
    /// ([`Recognition::refused`]) and is counted against its page.
    fn read(&mut self, request: &RecognitionRequest) -> Result<Recognition, EngineError>;
}

/// One request: the JSON header `ethos-parser-ocr serve` reads, and the image's bytes after it.
#[derive(Clone, Debug, Serialize)]
pub struct RecognitionRequest {
    /// Protocol version, 1.
    pub ocr_request: u32,
    /// Echoed in the answer.
    pub id: String,
    /// What the bytes are.
    pub image: ImageSpec,
    /// Stands the stored raster upright as the page shows it: mirrored left to right first when 4
    /// or more, then turned `orientation % 4` quarter turns clockwise.
    pub orientation: u8,
    /// The length of [`Self::data`].
    pub bytes: u64,
    /// The image's bytes: a PDF stream as stored, or an image file whole.
    #[serde(skip)]
    pub data: Vec<u8>,
}

/// What a request's bytes are.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImageSpec {
    /// An image file, PNG or JPEG.
    File,
    /// A PDF image XObject's stream, as its dictionary describes it.
    Pdf(PdfImageSpec),
}

/// What an image XObject's dictionary states, resolved to what the recognizer reads.
#[derive(Clone, Debug, Serialize)]
pub struct PdfImageSpec {
    /// `/Width`, in samples.
    pub width: u32,
    /// `/Height`, in samples.
    pub height: u32,
    /// `/BitsPerComponent`; 1 for an image mask.
    pub bits_per_component: u32,
    /// `None` for an image mask, which has no colour space.
    pub colour: Option<Colour>,
    /// `/Filter`, in order.
    pub filters: Vec<String>,
    /// `/DecodeParms`, one per filter; `None` where the filter has none this protocol carries.
    pub decode_parms: Vec<Option<FlateParms>>,
    /// `/Decode`, where the dictionary states one.
    pub decode: Option<Vec<f64>>,
    /// `/ImageMask`.
    pub image_mask: bool,
}

/// A colour space, resolved to the components it holds.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "space", rename_all = "snake_case")]
pub enum Colour {
    /// One component: `DeviceGray`, `CalGray`, or ICC-based with one.
    Gray,
    /// Three: `DeviceRGB`, `CalRGB`, or ICC-based with three.
    Rgb,
    /// Four: `DeviceCMYK`, or ICC-based with four.
    Cmyk,
    /// `/Indexed`: a palette over a base space.
    Indexed {
        /// The base space.
        base: Base,
        /// The highest index.
        hival: u32,
        /// The palette, `hival + 1` colours in the base's components.
        lookup: Vec<u8>,
    },
}

/// An indexed space's base.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Base {
    /// One component.
    Gray,
    /// Three.
    Rgb,
    /// Four.
    Cmyk,
}

/// A Flate or LZW stream's `/DecodeParms` (32000-1 Table 8), defaults filled in.
#[derive(Clone, Debug, Serialize)]
pub struct FlateParms {
    /// `/Predictor`, 1 when absent.
    pub predictor: u32,
    /// `/Colors`, 1 when absent.
    pub colors: u32,
    /// `/BitsPerComponent`, 8 when absent.
    pub bits_per_component: u32,
    /// `/Columns`, 1 when absent.
    pub columns: u32,
}

/// The recognizer's answer.
#[derive(Clone, Debug, Deserialize)]
pub struct Recognition {
    /// Protocol version, 1.
    pub ocr_response: u32,
    /// The request's id.
    pub id: String,
    /// The identity the recognizer reports, on every answer.
    pub identity: RecognizerIdentity,
    /// The stored raster's width, as decoded.
    pub width: u32,
    /// The stored raster's height, as decoded.
    pub height: u32,
    /// What it read, in its reading order.
    pub lines: Vec<RecognizedLine>,
    /// Why the image was not read, by name.
    pub refused: Option<String>,
}

/// One line, its quad in the stored raster's pixels.
#[derive(Clone, Debug, Deserialize)]
pub struct RecognizedLine {
    /// The line's text as read, spaces included.
    pub text: String,
    /// The text's top-left, top-right, bottom-right and bottom-left corners.
    pub quad: [[i32; 2]; 4],
    /// Its words, in order.
    pub words: Vec<RecognizedWord>,
}

/// One word, its quad in the stored raster's pixels.
#[derive(Clone, Debug, Deserialize)]
pub struct RecognizedWord {
    /// The word's characters.
    pub text: String,
    /// Its corners, as the line's are.
    pub quad: [[i32; 2]; 4],
}

/// The media type an image file's own signature states, for the two kinds `--ocr` reads whole
/// (`docs/35-OCR-SCOPE.md` §4): PNG and JPEG. Asked of the first bytes only, as every other
/// signature the engine reads is.
pub fn image_media_type(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// The pixel size an image file's own header states, width then height (decision #132): a PNG's
/// `IHDR`, which the format puts first, or a JPEG's first frame header, found by stepping over the
/// marker segments before it as libjpeg does. A header read, not a decode; `None` where neither is
/// found. A JPEG's height may be 0, set later by a DNL marker.
pub(crate) fn image_file_size(b: &[u8]) -> Option<(u32, u32)> {
    let be16 = |i: usize| {
        b.get(i..i + 2)
            .map(|x| u32::from(u16::from_be_bytes([x[0], x[1]])))
    };
    let be32 = |i: usize| {
        b.get(i..i + 4)
            .map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
    };
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return (b.get(12..16)? == b"IHDR").then_some((be32(16)?, be32(20)?));
    }
    let mut i = 2; // past the JPEG's start-of-image marker
    loop {
        // A byte that starts no marker is skipped to the next 0xFF, as libjpeg's `next_marker`
        // skips it.
        i += b.get(i..)?.iter().position(|&x| x == 0xFF)?;
        match *b.get(i + 1)? {
            0xFF => i += 1,                      // a fill byte
            0x00 | 0x01 | 0xD0..=0xD8 => i += 2, // no marker, or a marker with no segment
            0xD9 => return None,                 // the image ended before a frame
            // A frame header, `SOFn`: its height, then its width. C4, C8 and CC are not frames.
            m @ 0xC0..=0xCF if !matches!(m, 0xC4 | 0xC8 | 0xCC) => {
                return Some((be16(i + 7)?, be16(i + 5)?))
            }
            _ => i += 2 + be16(i + 2)? as usize,
        }
    }
}

/// A one-page PDF, `width` by `height` points, that draws nothing: the page an image file's words
/// are set on, one pixel to one point. Never written anywhere; the artifact's source is the file.
pub(crate) fn blank_page(width: u32, height: u32) -> Result<Vec<u8>, EngineError> {
    use lopdf::{dictionary, Object};
    let mut doc = lopdf::Document::with_version("1.7");
    let pages = doc.new_object_id();
    let page = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), i64::from(width).into(), i64::from(height).into()],
        "Resources" => dictionary! {},
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
        ),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).map_err(|e| EngineError::Io {
        detail: e.to_string(),
    })?;
    Ok(bytes)
}

/// The placement of an image file on its own page: pixel (x, y) at (x, y) points from the top
/// left, so a recognized word's place on the page is its place in the file.
pub(crate) fn whole_page(page_id: lopdf::ObjectId, width: u32, height: u32) -> Candidate {
    let (w, h) = (f64::from(width), f64::from(height));
    Candidate {
        page_id,
        // No image object: the page is the image.
        object: (0, 0),
        corners: [(0.0, h), (w, h), (w, 0.0), (0.0, 0.0)],
        visible: (0.0, 0.0, w, h),
    }
}

/// A page the trigger took: no text-showing operator, and exactly one image placed (§4).
#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub page_id: lopdf::ObjectId,
    pub object: lopdf::ObjectId,
    /// The unit square's corners (0,0), (1,0), (1,1), (0,1) on the page, in points, top-left
    /// system, `/Rotate` applied: where the image's pixels land.
    pub corners: [(f64, f64); 4],
    /// The box a viewer shows, in the same system: (x0, y0, x1, y1).
    pub visible: (f64, f64, f64, f64),
}

/// A recognized word placed on its page, quantized, ready to become a run.
#[derive(Clone, Debug)]
pub(crate) struct PageWord {
    /// The word, with the space the recognizer read after it where one follows on its line.
    pub text: String,
    pub origin_x: i64,
    pub origin_y: i64,
    /// The word box's width; `None` for a line that does not run left to right on the page,
    /// whose travel along x is not its length.
    pub advance: Option<i64>,
    /// The height of the word's line, between its quad's top and bottom edges: the size a text
    /// layer would set it at.
    pub font_size: i64,
    /// The word's left and right over its line's top and bottom; `None` where they enclose no
    /// area, as for a word read in the padding past a short line's end.
    pub rect: Option<QRect>,
    pub image: Option<PdfObjectNumber>,
    pub pixels: [[i32; 2]; 4],
}

/// The request for `candidate`'s image, or the name of what this protocol cannot carry.
pub(crate) fn request(
    doc: &lopdf::Document,
    candidate: &Candidate,
    page_number: u32,
) -> Result<RecognitionRequest, String> {
    let stream = doc
        .get_object(candidate.object)
        .and_then(lopdf::Object::as_stream)
        .map_err(|_| "image-not-loaded".to_string())?;
    let dict = &stream.dict;
    let int = |key: &[u8]| -> Option<i64> {
        resolve_object(doc, dict.get(key).ok()).and_then(|o| o.as_i64().ok())
    };
    let (width, height) = match (int(b"Width"), int(b"Height")) {
        (Some(w), Some(h))
            if w > 0 && h > 0 && w <= i64::from(u32::MAX) && h <= i64::from(u32::MAX) =>
        {
            (w as u32, h as u32)
        }
        _ => return Err("image-size-not-read".into()),
    };
    // A mask decides what of the image a viewer sees, and the protocol carries the image alone:
    // read without it, black text masked onto a black image reaches the recognizer all black, and
    // what a stencil hides could be read as text no viewer shows (PR 53's review).
    if dict.has(b"SMask") {
        return Err("soft-mask-not-read".into());
    }
    if dict.has(b"Mask") {
        return Err("mask-not-read".into());
    }
    let image_mask = resolve_object(doc, dict.get(b"ImageMask").ok())
        .and_then(|o| o.as_bool().ok())
        .unwrap_or(false);
    let bits_per_component = match int(b"BitsPerComponent") {
        Some(b) => u32::try_from(b).map_err(|_| "bits-per-component-not-read".to_string())?,
        None if image_mask => 1,
        None => 8, // a JPEG 2000 stream may omit it; that filter is refused by name below
    };
    let names = |o: Option<lopdf::Object>| -> Vec<String> {
        match o {
            Some(lopdf::Object::Name(n)) => vec![String::from_utf8_lossy(&n).into_owned()],
            Some(lopdf::Object::Array(a)) => a
                .iter()
                .filter_map(|x| x.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .collect(),
            _ => Vec::new(),
        }
    };
    let filters = names(resolve_object(doc, dict.get(b"Filter").ok()));
    // Decision #132: a JPEG 2000 stream may leave its colour space to its own header, so a filter
    // the recognizer does not decode is named before the colour space is asked for, by the
    // recognizer's own rule and name (`ocr/src/decode.rs`, `pdf`, which decodes in order): the
    // first that is neither Flate nor a final DCT. `the_filter_rule_is_the_recognizers` holds the
    // two together.
    let last = filters.len().saturating_sub(1);
    if let Some((_, f)) = filters.iter().enumerate().find(|&(i, f)| match f.as_str() {
        "FlateDecode" | "Fl" => false,
        "DCTDecode" | "DCT" => i != last,
        _ => true,
    }) {
        return Err(format!("filter-not-decoded:{f}"));
    }
    let colour = if image_mask {
        None
    } else {
        Some(colour(doc, dict.get(b"ColorSpace").ok())?)
    };
    let parms: Vec<Option<lopdf::Object>> = match resolve_object(doc, dict.get(b"DecodeParms").ok())
    {
        Some(lopdf::Object::Array(a)) => a.iter().map(|o| resolve_object(doc, Some(o))).collect(),
        Some(o) => vec![Some(o)],
        None => Vec::new(),
    };
    let decode_parms = filters
        .iter()
        .enumerate()
        .map(
            |(i, f)| match (f.as_str(), parms.get(i).cloned().flatten()) {
                (
                    "FlateDecode" | "Fl" | "LZWDecode" | "LZW",
                    Some(lopdf::Object::Dictionary(d)),
                ) => {
                    let p = |key: &[u8], default: i64| {
                        resolve_object(doc, d.get(key).ok())
                            .and_then(|o| o.as_i64().ok())
                            .unwrap_or(default)
                    };
                    Some(FlateParms {
                        predictor: p(b"Predictor", 1).clamp(0, 15) as u32,
                        colors: p(b"Colors", 1).clamp(1, 4) as u32,
                        bits_per_component: p(b"BitsPerComponent", 8).clamp(1, 16) as u32,
                        columns: p(b"Columns", 1).clamp(1, i64::from(u32::MAX)) as u32,
                    })
                }
                _ => None,
            },
        )
        .collect();
    let decode = resolve_array(doc, dict.get(b"Decode").ok()).map(|a| {
        a.iter()
            .filter_map(|o| match o {
                lopdf::Object::Integer(i) => Some(*i as f64),
                lopdf::Object::Real(r) => Some(f64::from(*r)),
                _ => None,
            })
            .collect()
    });
    let orientation =
        orientation(candidate.corners, width, height).ok_or("placement-not-axis-aligned")?;
    Ok(RecognitionRequest {
        ocr_request: 1,
        id: format!("page {page_number}"),
        image: ImageSpec::Pdf(PdfImageSpec {
            width,
            height,
            bits_per_component,
            colour,
            filters,
            decode_parms,
            decode,
            image_mask,
        }),
        orientation,
        bytes: stream.content.len() as u64,
        data: stream.content.clone(),
    })
}

/// A colour space resolved to what the recognizer reads, or the name of one it does not.
fn colour(doc: &lopdf::Document, o: Option<&lopdf::Object>) -> Result<Colour, String> {
    let space = resolve_object(doc, o).ok_or("colour-space-not-read")?;
    let named = |n: &[u8]| -> Result<Colour, String> {
        match n {
            b"DeviceGray" | b"G" | b"CalGray" => Ok(Colour::Gray),
            b"DeviceRGB" | b"RGB" | b"CalRGB" => Ok(Colour::Rgb),
            b"DeviceCMYK" | b"CMYK" => Ok(Colour::Cmyk),
            other => Err(format!(
                "colour-space-not-read:{}",
                String::from_utf8_lossy(other)
            )),
        }
    };
    match &space {
        lopdf::Object::Name(n) => named(n),
        lopdf::Object::Array(a) => {
            let family = a
                .first()
                .and_then(|f| f.as_name().ok())
                .ok_or("colour-space-not-read")?;
            match family {
                b"ICCBased" => {
                    let s =
                        resolve_stream(doc, a.get(1)).ok_or("colour-space-not-read:ICCBased")?;
                    match resolve_object(doc, s.dict.get(b"N").ok()).and_then(|n| n.as_i64().ok()) {
                        Some(1) => Ok(Colour::Gray),
                        Some(3) => Ok(Colour::Rgb),
                        Some(4) => Ok(Colour::Cmyk),
                        _ => Err("colour-space-not-read:ICCBased".into()),
                    }
                }
                b"CalGray" | b"CalRGB" | b"DeviceGray" | b"DeviceRGB" | b"DeviceCMYK" => {
                    named(family)
                }
                b"Indexed" | b"I" => {
                    let base = match colour(doc, a.get(1))? {
                        Colour::Gray => Base::Gray,
                        Colour::Rgb => Base::Rgb,
                        Colour::Cmyk => Base::Cmyk,
                        Colour::Indexed { .. } => {
                            return Err("colour-space-not-read:Indexed".into())
                        }
                    };
                    let hival = resolve_object(doc, a.get(2))
                        .and_then(|h| h.as_i64().ok())
                        .filter(|h| (0..=255).contains(h))
                        .ok_or("colour-space-not-read:Indexed")?;
                    let lookup = match resolve_object(doc, a.get(3)) {
                        Some(lopdf::Object::String(bytes, _)) => bytes,
                        // A lookup table in a stream would have to be decoded here; refused by name.
                        _ => return Err("colour-space-not-read:Indexed".into()),
                    };
                    Ok(Colour::Indexed {
                        base,
                        hival: hival as u32,
                        lookup,
                    })
                }
                other => Err(format!(
                    "colour-space-not-read:{}",
                    String::from_utf8_lossy(other)
                )),
            }
        }
        _ => Err("colour-space-not-read".into()),
    }
}

/// The orientation that stands the stored raster upright as the page shows it: the one of the
/// eight axis-aligned ones whose turn (and mirror) points the raster's right and down where the
/// page draws them. `None` when the placement skews or turns by anything but a quarter.
pub(crate) fn orientation(corners: [(f64, f64); 4], width: u32, height: u32) -> Option<u8> {
    let [c00, c10, _, c01] = corners;
    // Where one stored pixel to the right, and one down, land on the page (top-left system). The
    // stored raster's first row is the top of the unit square, v = 1.
    let right = (
        (c10.0 - c00.0) / f64::from(width),
        (c10.1 - c00.1) / f64::from(width),
    );
    let down = (
        -(c01.0 - c00.0) / f64::from(height),
        -(c01.1 - c00.1) / f64::from(height),
    );
    let sign = |v: f64, scale: f64| -> i32 {
        if v.abs() <= scale * 1e-9 {
            0
        } else if v > 0.0 {
            1
        } else {
            -1
        }
    };
    let scale = right
        .0
        .abs()
        .max(right.1.abs())
        .max(down.0.abs())
        .max(down.1.abs());
    let shown = [
        [sign(right.0, scale), sign(down.0, scale)],
        [sign(right.1, scale), sign(down.1, scale)],
    ];
    (0..8u8).find(|&o| turn_matrix(o) == shown)
}

/// How orientation `o` moves the stored raster's right and down, as columns of a matrix over
/// (x right, y down): mirrored left to right first when `o >= 4`, then turned `o % 4` quarter turns
/// clockwise — a clockwise turn sends right to down and down to left.
fn turn_matrix(o: u8) -> [[i32; 2]; 2] {
    let mut m = if o >= 4 {
        [[-1, 0], [0, 1]]
    } else {
        [[1, 0], [0, 1]]
    };
    for _ in 0..o % 4 {
        // R = [[0, -1], [1, 0]]; m = R * m.
        m = [[-m[1][0], -m[1][1]], [m[0][0], m[0][1]]];
    }
    m
}

/// One run of a recognized line: its text, its left and right on the page, the pixels it was read
/// from, and whether it advances along x.
type Piece = (String, f64, f64, [[i32; 2]; 4], bool);

/// The words of `answer`, placed on `candidate`'s page: each word's left and right through the
/// placement, over its line's top and bottom; the space the recognizer read after a word kept on
/// it; the size the line's own height between its quad's top and bottom edges, not its box's,
/// which a skewed line makes taller. A line that does not run left to right on the page — turned
/// within the image, or upside down — is one run, its words joined by spaces, with no advance
/// along x: a word of it has no left and right a line's rules could read. Words whose box's
/// centre falls outside the visible box are dropped and counted, as text no viewer shows.
/// `image` is the image XObject read.
pub(crate) fn place(
    candidate: &Candidate,
    answer: &Recognition,
    image: Option<PdfObjectNumber>,
) -> Result<(Vec<PageWord>, u32), EngineError> {
    let (w, h) = (f64::from(answer.width), f64::from(answer.height));
    let [c00, c10, _, c01] = candidate.corners;
    // Pixel corner (px, py) to the page: u = px / w, v = 1 - py / h on the unit square.
    let at = |p: [i32; 2]| -> (f64, f64) {
        let (u, v) = (f64::from(p[0]) / w, 1.0 - f64::from(p[1]) / h);
        (
            c00.0 + u * (c10.0 - c00.0) + v * (c01.0 - c00.0),
            c00.1 + u * (c10.1 - c00.1) + v * (c01.1 - c00.1),
        )
    };
    let span = |q: &[[i32; 2]; 4]| -> (f64, f64, f64, f64) {
        let pts = q.map(at);
        let xs = pts.map(|p| p.0);
        let ys = pts.map(|p| p.1);
        let lo = |v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = |v: [f64; 4]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (lo(xs), lo(ys), hi(xs), hi(ys))
    };
    let q = |pt: f64| {
        quantize(pt, QUANTUM_PER_POINT).map_err(|_| EngineError::Malformed {
            what: "recognized word".into(),
            detail: "a recognized word's place cannot be expressed in centipoints".into(),
        })
    };
    let (vx0, vy0, vx1, vy1) = candidate.visible;
    let (mut out, mut dropped) = (Vec::new(), 0u32);
    for line in &answer.lines {
        let p = line.quad.map(at);
        let (dx, dy) = (p[1].0 - p[0].0, p[1].1 - p[0].1);
        let (ux, uy) = (p[3].0 - p[0].0, p[3].1 - p[0].1);
        let height = q((ux * ux + uy * uy).sqrt())?;
        let (lx0, ly0, lx1, ly1) = span(&line.quad);
        let along_x = dx > 0.0 && dy.abs() <= dx;
        let pieces: Vec<Piece> = if along_x {
            let last = line.words.len().saturating_sub(1);
            line.words
                .iter()
                .enumerate()
                .map(|(k, word)| {
                    let (x0, _, x1, _) = span(&word.quad);
                    let text = if k < last {
                        format!("{} ", word.text)
                    } else {
                        word.text.clone()
                    };
                    (text, x0, x1, word.quad, true)
                })
                .collect()
        } else {
            let text = line
                .words
                .iter()
                .map(|wd| wd.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            vec![(text, lx0, lx1, line.quad, false)]
        };
        for (text, x0, x1, pixels, advances) in pieces {
            if text.is_empty() {
                continue;
            }
            let (cx, cy) = ((x0 + x1) / 2.0, (ly0 + ly1) / 2.0);
            if !(vx0..=vx1).contains(&cx) || !(vy0..=vy1).contains(&cy) {
                dropped = dropped.saturating_add(1);
                continue;
            }
            let (qx0, qy0, qx1, qy1) = (q(x0)?, q(ly0)?, q(x1)?, q(ly1)?);
            // A word read in the padding past a short line's end — a lone `−`, the `in Data` of a
            // narrow label — has steps that map past the line's last column at both edges, so its
            // box encloses nothing. It keeps its text and its origin, and reports no box, as a
            // drawn run without an advance reports none; one such word refused the whole document.
            let rect = match QRect::new(qx0, qy0, qx1.max(qx0), qy1.max(qy0)) {
                Ok(rect) => Some(rect),
                Err(QRectError::NonPositiveArea) => None,
                Err(QRectError::OutOfRange) => {
                    return Err(EngineError::Malformed {
                        what: "recognized word".into(),
                        detail: "a recognized word's box is not a rectangle".into(),
                    })
                }
            };
            out.push(PageWord {
                text,
                origin_x: qx0,
                origin_y: qy1,
                advance: rect.filter(|_| advances).map(|r| r.x1() - r.x0()),
                font_size: height,
                rect,
                image,
                pixels,
            });
        }
    }
    Ok((out, dropped))
}

#[cfg(test)]
mod tests {
    use super::*;

    use ethos_parser_core::{DerivationClass, NativeLocator, Profile, RecognizerPin};

    use crate::document::Document;

    /// A one-page PDF, 200 x 100 points, drawing `images` copies of a 20 x 10 gray image with the
    /// matrix `cm`, and `text` with Helvetica when given.
    fn scanned(cm: &str, images: usize, text: Option<&str>) -> Vec<u8> {
        use lopdf::{dictionary, Object, Stream};
        let mut doc = lopdf::Document::with_version("1.7");
        let pages = doc.new_object_id();
        let image = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 20, "Height" => 10,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
            },
            vec![255u8; 200],
        ));
        let font = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"
        });
        let mut ops = String::new();
        for _ in 0..images {
            ops.push_str(&format!("q {cm} cm /Im0 Do Q "));
        }
        if let Some(t) = text {
            ops.push_str(&format!("BT /F1 12 Tf 10 50 Td ({t}) Tj ET"));
        }
        let content = doc.add_object(Stream::new(dictionary! {}, ops.into_bytes()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => dictionary! {
                "XObject" => dictionary! { "Im0" => image },
                "Font" => dictionary! { "F1" => font },
            },
            "Contents" => content,
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("saves");
        bytes
    }

    /// A recognizer that is a function: it answers every request with `answer` (or refuses with
    /// `refused`) and keeps the requests.
    struct Fake {
        identity: RecognizerIdentity,
        /// The identity its answers carry, where it is not the one it states.
        answers_as: Option<RecognizerIdentity>,
        refused: Option<String>,
        requests: Vec<RecognitionRequest>,
        /// The pixel "world" ends at; 18, inside the 20-pixel image, unless a test moves it.
        reach: i32,
    }

    impl Fake {
        fn new() -> Self {
            Fake {
                identity: RecognizerIdentity::ethos_parser_ocr(),
                answers_as: None,
                refused: None,
                requests: Vec::new(),
                reach: 18,
            }
        }
    }

    impl Recognize for Fake {
        fn identity(&self) -> &RecognizerIdentity {
            &self.identity
        }
        fn read(&mut self, request: &RecognitionRequest) -> Result<Recognition, EngineError> {
            self.requests.push(request.clone());
            let word = |t: &str, x0: i32, x1: i32| RecognizedWord {
                text: t.into(),
                quad: [[x0, 2], [x1, 2], [x1, 8], [x0, 8]],
            };
            Ok(Recognition {
                ocr_response: 1,
                id: request.id.clone(),
                identity: self
                    .answers_as
                    .clone()
                    .unwrap_or_else(|| self.identity.clone()),
                width: 20,
                height: 10,
                lines: if self.refused.is_some() {
                    Vec::new()
                } else {
                    vec![RecognizedLine {
                        text: "Hello world".into(),
                        quad: [[2, 2], [18, 2], [18, 8], [2, 8]],
                        words: vec![word("Hello", 2, 9), word("world", 11, self.reach)],
                    }]
                },
                refused: self.refused.clone(),
            })
        }
    }

    fn codes(a: &crate::ExtractArtifact) -> Vec<&str> {
        a.assurance
            .limitations
            .iter()
            .map(|l| l.code.as_str())
            .collect()
    }

    /// The one-image scan with its image's dictionary edited.
    fn scanned_with(edit: impl FnOnce(&mut lopdf::Dictionary)) -> Vec<u8> {
        let mut pdf =
            lopdf::Document::load_mem(&scanned("200 0 0 100 0 0", 1, None)).expect("loads");
        let image = pdf
            .objects
            .values_mut()
            .find_map(|o| match o {
                lopdf::Object::Stream(s) if s.dict.has(b"Width") => Some(s),
                _ => None,
            })
            .expect("the page's image");
        edit(&mut image.dict);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).expect("saves");
        bytes
    }

    /// `filters` as a `/Filter` array.
    fn filter(filters: &[&str]) -> lopdf::Object {
        lopdf::Object::Array(
            filters
                .iter()
                .map(|f| lopdf::Object::Name(f.as_bytes().to_vec()))
                .collect(),
        )
    }

    /// The reason the page's `scanned-page-not-recognized` names, if it carries one.
    fn reason(a: &crate::ExtractArtifact) -> Option<String> {
        a.assurance
            .limitations
            .iter()
            .find(|l| l.code == ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED)
            .and_then(|l| l.detail.split('`').nth(1).map(str::to_string))
    }

    #[test]
    fn a_scanned_page_reads_as_recognized_runs() {
        let profile = Profile::ocr_v0();
        let doc =
            Document::open_bytes(&scanned("200 0 0 100 0 0", 1, None), &profile).expect("opens");
        let mut fake = Fake::new();
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert_eq!(fake.requests.len(), 1);
        assert_eq!(fake.requests[0].orientation, 0);
        assert_eq!(fake.requests[0].data.len(), 200, "the stream as stored");
        let runs = &artifact.pages[0].runs;
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            ["Hello ", "world"]
        );
        let r = &runs[0];
        assert_eq!(r.derivation, DerivationClass::Recognized);
        // 2 of 20 pixels across 200 points, the line's foot 8 of 10 pixels down 100 points.
        assert_eq!(
            (r.locator.origin_x, r.locator.origin_y, r.locator.advance),
            (2000, 8000, Some(7000))
        );
        assert_eq!(r.font_size, 6000);
        assert_eq!(
            r.recognized.as_ref().unwrap().pixels,
            [[2, 2], [9, 2], [9, 8], [2, 8]]
        );
        assert!(codes(&artifact).contains(&ethos_parser_core::codes::RECOGNIZED_TEXT_HAS_NO_FONT));
        // The representation addresses it where the recognizer read it, and seals.
        let rep = crate::to_representation(&artifact, &profile).expect("seals");
        let node = rep
            .payload()
            .nodes
            .iter()
            .find(|n| n.text == "Hello ")
            .expect("the run is a node");
        assert_eq!(node.derivation, DerivationClass::Recognized);
        let NativeLocator::Recognized(at) = &node.native_locator else {
            panic!("a recognized run is addressed as one");
        };
        assert_eq!(at.run.origin_x, 2000);
        assert!(at.image.is_some());
    }

    #[test]
    fn a_flipped_image_is_asked_for_upright() {
        let profile = Profile::ocr_v0();
        let doc =
            Document::open_bytes(&scanned("200 0 0 -100 0 100", 1, None), &profile).expect("opens");
        let mut fake = Fake::new();
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert_eq!(turn_matrix(fake.requests[0].orientation), [[1, 0], [0, -1]]);
        // Flipped, the stored rows 2..8 of 10 land at 80..20 points down the page: the line's
        // foot is the lower edge, 80 points, as it is upright, because the rows straddle the
        // middle; what moved is the request, which stands the raster up before it is read.
        assert_eq!(artifact.pages[0].runs[0].locator.origin_y, 8000);
    }

    #[test]
    fn a_page_that_shows_text_is_not_asked() {
        let profile = Profile::ocr_v0();
        let doc = Document::open_bytes(&scanned("200 0 0 100 0 0", 1, Some("drawn")), &profile)
            .expect("opens");
        let mut fake = Fake::new();
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert!(fake.requests.is_empty());
        assert!(artifact.pages[0]
            .runs
            .iter()
            .all(|r| r.derivation == DerivationClass::Extracted));
        assert!(!codes(&artifact).contains(&ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED));
    }

    #[test]
    fn two_images_are_refused_by_name() {
        let profile = Profile::ocr_v0();
        let doc =
            Document::open_bytes(&scanned("200 0 0 100 0 0", 2, None), &profile).expect("opens");
        let mut fake = Fake::new();
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert!(fake.requests.is_empty());
        let l = artifact
            .assurance
            .limitations
            .iter()
            .find(|l| l.code == ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED)
            .expect("counted on its page");
        assert!(l.detail.contains("several-images"), "{}", l.detail);
    }

    #[test]
    fn an_image_a_mask_shapes_is_refused_by_name() {
        // PR 53's review: the protocol carries the image alone, so a page whose image a soft mask
        // or a mask shapes is refused by name, never read as something no viewer sees.
        use lopdf::{dictionary, Object, Stream};
        let profile = Profile::ocr_v0();
        for (key, reason) in [("SMask", "soft-mask-not-read"), ("Mask", "mask-not-read")] {
            let mut pdf =
                lopdf::Document::load_mem(&scanned("200 0 0 100 0 0", 1, None)).expect("loads");
            let mask = pdf.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image", "Width" => 20, "Height" => 10,
                    "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                },
                vec![255u8; 200],
            ));
            let image = pdf
                .objects
                .iter_mut()
                .find_map(|(id, o)| match o {
                    Object::Stream(s) if *id != mask && s.dict.has(b"Width") => Some(s),
                    _ => None,
                })
                .expect("the page's image");
            image.dict.set(key, mask);
            let mut bytes = Vec::new();
            pdf.save_to(&mut bytes).expect("saves");
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            let mut fake = Fake::new();
            let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            assert!(fake.requests.is_empty(), "{key}: nothing is sent");
            let l = artifact
                .assurance
                .limitations
                .iter()
                .find(|l| l.code == ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED)
                .expect("counted on its page");
            assert!(l.detail.contains(reason), "{}", l.detail);
        }
    }

    #[test]
    fn a_recognizer_refusal_is_counted_on_its_page() {
        let profile = Profile::ocr_v0();
        let doc =
            Document::open_bytes(&scanned("200 0 0 100 0 0", 1, None), &profile).expect("opens");
        let mut fake = Fake::new();
        fake.refused = Some("filter-not-decoded:JPXDecode".into());
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert!(artifact.pages[0].runs.is_empty());
        assert!(artifact.assurance.limitations.iter().any(|l| l.code
            == ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED
            && l.detail.contains("filter-not-decoded:JPXDecode")));
    }

    #[test]
    fn a_filter_no_recognizer_decodes_is_named_before_the_colour_space() {
        // Decision #132: a JPEG 2000 stream may leave its colour space to its own header, so a
        // filter the recognizer does not decode is refused by the name the recognizer gives it —
        // the first that is neither Flate nor a final DCT — before the colour space is asked for,
        // and nothing is sent.
        let profile = Profile::ocr_v0();
        for (filters, colour, refused) in [
            (&["FlateDecode", "JPXDecode"][..], false, Some("JPXDecode")),
            (
                &["FlateDecode", "CCITTFaxDecode"],
                true,
                Some("CCITTFaxDecode"),
            ),
            (&["JBIG2Decode"], true, Some("JBIG2Decode")),
            (
                &["ASCII85Decode", "JPXDecode"],
                false,
                Some("ASCII85Decode"),
            ),
            (&["DCTDecode", "FlateDecode"], true, Some("DCTDecode")),
            (&["FlateDecode", "DCTDecode"], true, None),
        ] {
            let bytes = scanned_with(|d| {
                d.set("Filter", filter(filters));
                if !colour {
                    d.remove(b"ColorSpace");
                }
            });
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            let mut fake = Fake::new();
            let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            match refused {
                Some(f) => {
                    assert!(fake.requests.is_empty(), "{filters:?}: nothing is sent");
                    assert_eq!(
                        reason(&artifact).as_deref(),
                        Some(format!("filter-not-decoded:{f}").as_str()),
                        "{filters:?}"
                    );
                }
                None => assert_eq!(fake.requests.len(), 1, "{filters:?} is decoded"),
            }
        }
    }

    #[test]
    fn the_filter_rule_is_the_recognizers() {
        // `request` names the filters `ocr/src/decode.rs` refuses, before it is asked, so the two
        // say the same: a decoder added there is a filter admitted here, in the same change.
        let decode = include_str!("../../../ocr/src/decode.rs");
        for arm in [
            r#""FlateDecode" | "Fl" => {"#,
            r#""DCTDecode" | "DCT" if i + 1 == spec.filters.len() => return dct(spec, &bytes),"#,
            r#"other => return Err(format!("filter-not-decoded:{other}")),"#,
        ] {
            assert!(
                decode.contains(arm),
                "ocr/src/decode.rs no longer reads `{arm}`: change `request`'s filter rule with it"
            );
        }
    }

    #[test]
    fn the_profile_and_the_recognizer_must_name_each_other() {
        let bytes = scanned("200 0 0 100 0 0", 1, None);
        // The OCR profile with no recognizer, and a recognizer under a profile pinning none.
        let ocr = Profile::ocr_v0();
        let doc = Document::open_bytes(&bytes, &ocr).expect("opens");
        assert!(matches!(
            crate::extract(&doc, &ocr),
            Err(EngineError::Unsupported { .. })
        ));
        let base = Profile::default();
        let doc = Document::open_bytes(&bytes, &base).expect("opens");
        assert!(matches!(
            crate::extract_recognized(&doc, &base, &mut Fake::new()),
            Err(EngineError::Unsupported { .. })
        ));
        // A recognizer reporting another identity.
        let doc = Document::open_bytes(&bytes, &ocr).expect("opens");
        let mut other = Fake::new();
        other.identity.version = "9.9.9".into();
        assert!(matches!(
            crate::extract_recognized(&doc, &ocr, &mut other),
            Err(EngineError::Unsupported { .. })
        ));
        // Refused before anything is read: a document with no scanned page asks it nothing, and
        // is still not made under a profile naming another recognizer.
        let drawn = scanned("200 0 0 100 0 0", 1, Some("drawn"));
        let doc = Document::open_bytes(&drawn, &ocr).expect("opens");
        assert!(matches!(
            crate::extract_recognized(&doc, &ocr, &mut other),
            Err(EngineError::Unsupported { .. })
        ));
        assert!(other.requests.is_empty());
        // One that states the pinned identity and answers as another, a scan and an image file.
        let mut two_faced = Fake::new();
        two_faced.answers_as = Some(other.identity.clone());
        let doc = Document::open_bytes(&bytes, &ocr).expect("opens");
        assert!(matches!(
            crate::extract_recognized(&doc, &ocr, &mut two_faced),
            Err(EngineError::Unsupported { .. })
        ));
        assert!(matches!(
            crate::extract_image(&png(20, 10), &ocr, &mut two_faced),
            Err(EngineError::Unsupported { .. })
        ));
        // And without `--ocr` nothing is read: no runs, nothing recognized.
        let doc = Document::open_bytes(&bytes, &base).expect("opens");
        let plain = crate::extract(&doc, &base).expect("extracts");
        assert!(plain.pages[0].runs.is_empty());
        assert!(!codes(&plain).iter().any(|c| c.starts_with("recognized")));
        assert!(matches!(base.recognizer, RecognizerPin::NotPinned));
    }

    #[test]
    fn a_scan_read_without_ocr_is_counted_as_ocr_would_count_it() {
        // Decision #132: under the default profile, a page `--ocr` would ask the recognizer to
        // read is counted on its page as not requested, so a scan read without the flag is not a
        // blank page; one `--ocr` would refuse before asking is counted by that reason.
        let base = Profile::default();
        let extract = |bytes: &[u8]| {
            let doc = Document::open_bytes(bytes, &base).expect("opens");
            crate::extract(&doc, &base).expect("extracts")
        };
        let scan = extract(&scanned("200 0 0 100 0 0", 1, None));
        assert!(scan.pages[0].runs.is_empty());
        let l = scan
            .assurance
            .limitations
            .iter()
            .find(|l| l.code == ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED)
            .expect("counted on its page");
        assert_eq!(l.scope, ethos_parser_core::LimitationScope::Page(1));
        assert_eq!(reason(&scan).as_deref(), Some("ocr-not-requested"));
        for (bytes, why) in [
            (
                scanned_with(|d| d.set("Filter", filter(&["CCITTFaxDecode"]))),
                "filter-not-decoded:CCITTFaxDecode",
            ),
            (
                scanned_with(|d| d.set("SMask", lopdf::Object::Reference((1, 0)))),
                "soft-mask-not-read",
            ),
        ] {
            assert_eq!(reason(&extract(&bytes)).as_deref(), Some(why));
        }
        // Never on a page that shows text, nor on one the trigger itself does not take.
        for (images, text) in [(1, Some("drawn")), (2, None)] {
            let other = extract(&scanned("200 0 0 100 0 0", images, text));
            assert!(
                !codes(&other).contains(&ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED),
                "{images} image(s), text {text:?}"
            );
        }
    }

    #[test]
    fn an_upright_placement_needs_no_turn() {
        // A 100 x 50 image painted upright at (10, 20)-(110, 70) on the page (top-left system):
        // (0,0) of the unit square is the bottom-left corner, (0,1) the top-left.
        let corners = [(10.0, 70.0), (110.0, 70.0), (110.0, 20.0), (10.0, 20.0)];
        assert_eq!(orientation(corners, 100, 50), Some(0));
    }

    #[test]
    fn a_flipped_placement_is_stood_upright() {
        // Painted with a negative vertical scale: the stored first row lands at the page's bottom.
        let corners = [(10.0, 20.0), (110.0, 20.0), (110.0, 70.0), (10.0, 70.0)];
        let o = orientation(corners, 100, 50).unwrap();
        // Mirror then a half turn is a flip top to bottom.
        assert_eq!(turn_matrix(o), [[1, 0], [0, -1]]);
    }

    #[test]
    fn a_quarter_turned_placement_turns_back() {
        // The unit square's x axis runs up the page and its y axis to the left, so the stored
        // raster's right points up and its down points right: the page shows it a quarter turn
        // anticlockwise, which is three clockwise.
        let corners = [(110.0, 70.0), (110.0, 20.0), (60.0, 20.0), (60.0, 70.0)];
        assert_eq!(orientation(corners, 50, 50), Some(3));
        assert_eq!(turn_matrix(3), [[0, 1], [-1, 0]]);
    }

    #[test]
    fn a_skewed_placement_is_refused() {
        let corners = [(10.0, 70.0), (110.0, 60.0), (110.0, 10.0), (10.0, 20.0)];
        assert_eq!(orientation(corners, 100, 50), None);
    }

    #[test]
    fn words_land_through_the_placement_and_keep_their_spaces() {
        // A 200 x 100 pixel image painted over (0,0)-(100,50) points: half a point per pixel.
        let candidate = Candidate {
            page_id: (1, 0),
            object: (2, 0),
            corners: [(0.0, 50.0), (100.0, 50.0), (100.0, 0.0), (0.0, 0.0)],
            visible: (0.0, 0.0, 100.0, 50.0),
        };
        let word = |t: &str, x0: i32, x1: i32| RecognizedWord {
            text: t.into(),
            quad: [[x0, 10], [x1, 10], [x1, 30], [x0, 30]],
        };
        let answer = Recognition {
            ocr_response: 1,
            id: "page 1".into(),
            identity: RecognizerIdentity::ethos_parser_ocr(),
            width: 200,
            height: 100,
            lines: vec![
                RecognizedLine {
                    text: "May 2024".into(),
                    quad: [[10, 8], [90, 8], [90, 32], [10, 32]],
                    words: vec![word("May", 10, 40), word("2024", 50, 90)],
                },
                // A line set up the image's right edge, read bottom to top: its top-left corner
                // is its bottom-left on the page.
                RecognizedLine {
                    text: "Side bar".into(),
                    quad: [[150, 90], [150, 10], [170, 10], [170, 90]],
                    words: vec![
                        RecognizedWord {
                            text: "Side".into(),
                            quad: [[150, 90], [150, 10], [170, 10], [170, 90]],
                        },
                        RecognizedWord {
                            text: "bar".into(),
                            quad: [[150, 90], [150, 10], [170, 10], [170, 90]],
                        },
                    ],
                },
            ],
            refused: None,
        };
        let (words, dropped) = place(&candidate, &answer, None).unwrap();
        assert_eq!(dropped, 0);
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["May ", "2024", "Side bar"]
        );
        // x 10 px = 5 pt = 500 centipoints; the line runs 8..32 px = 4..16 pt.
        assert_eq!(
            (words[0].origin_x, words[0].origin_y, words[0].advance),
            (500, 1600, Some(1500))
        );
        assert_eq!(words[0].font_size, 1200);
        assert_eq!(words[1].origin_x, 2500);
        // The sideways line is one run, sized by its own height (20 px = 10 pt), with no advance.
        let side = &words[2];
        assert_eq!(
            (side.text.as_str(), side.advance, side.font_size),
            ("Side bar", None, 1000)
        );
        assert_eq!(words.len(), 3);
    }

    #[test]
    fn a_word_past_its_pages_edge_keeps_its_text_and_not_its_box() {
        // PR 53's review: a word whose centre is on the page and whose box runs past its edge
        // reached the seal as a measured box outside the page, and the whole document was refused.
        let profile = Profile::ocr_v0();
        let mut fake = Fake::new();
        fake.reach = 24; // "world", 11..24, runs past the 20-pixel image; its centre, 17.5, is on it
        let png = png(20, 10);
        let artifact = crate::extract_image(&png, &profile, &mut fake).expect("extracts");
        let world = &artifact.pages[0].runs[1];
        assert_eq!(world.text, "world");
        assert_eq!(
            world.geometry,
            ethos_parser_core::GeometryPresence::Absent(
                ethos_parser_core::GeometryAbsence::MeasuredOffPage
            )
        );
        crate::to_representation(&artifact, &profile).expect("seals");
    }

    #[test]
    fn a_word_whose_box_encloses_nothing_keeps_its_text_and_reports_no_box() {
        // ParseBench under --ocr: a word read in the padding past a short line's end came back with
        // no width — a lone `−` at x = 206..206 — and the whole document was refused.
        let profile = Profile::ocr_v0();
        let mut fake = Fake::new();
        fake.reach = 11; // "world", 11..11: no width; its centre, 11, is on the 20-pixel image
        let png = png(20, 10);
        let artifact = crate::extract_image(&png, &profile, &mut fake).expect("extracts");
        let world = &artifact.pages[0].runs[1];
        assert_eq!(world.text, "world");
        assert_eq!(
            world.geometry,
            ethos_parser_core::GeometryPresence::Absent(
                ethos_parser_core::GeometryAbsence::NotReportedByReader
            )
        );
        assert_eq!(world.locator.advance, None);
        crate::to_representation(&artifact, &profile).expect("seals");
    }

    #[test]
    fn an_image_files_page_holds_a_word_where_the_file_does() {
        // A word 5..25 pixels down a 100-pixel image lies 5..25 points down its page: off the
        // middle, so a page set upside down would put it 75..95 points down.
        let answer = Recognition {
            ocr_response: 1,
            id: "image".into(),
            identity: RecognizerIdentity::ethos_parser_ocr(),
            width: 200,
            height: 100,
            lines: vec![RecognizedLine {
                text: "Top".into(),
                quad: [[10, 5], [60, 5], [60, 25], [10, 25]],
                words: vec![RecognizedWord {
                    text: "Top".into(),
                    quad: [[10, 5], [60, 5], [60, 25], [10, 25]],
                }],
            }],
            refused: None,
        };
        let (words, dropped) = place(&whole_page((1, 0), 200, 100), &answer, None).unwrap();
        assert_eq!(dropped, 0);
        assert_eq!(
            (words[0].origin_x, words[0].origin_y, words[0].advance),
            (1000, 2500, Some(5000))
        );
    }

    #[test]
    fn a_word_no_viewer_shows_is_dropped_and_counted() {
        // A 20 x 10 pixel image painted over (0,0)-(200,100) points on a page whose visible box
        // ends at x = 100: "Hello" (pixels 2..9) is shown, "world" (11..18) is not.
        let candidate = Candidate {
            page_id: (1, 0),
            object: (2, 0),
            corners: [(0.0, 100.0), (200.0, 100.0), (200.0, 0.0), (0.0, 0.0)],
            visible: (0.0, 0.0, 100.0, 100.0),
        };
        let mut fake = Fake::new();
        let answer = fake
            .read(&RecognitionRequest {
                ocr_request: 1,
                id: "page 1".into(),
                image: ImageSpec::File,
                orientation: 0,
                bytes: 0,
                data: Vec::new(),
            })
            .expect("answers");
        let (words, dropped) = place(&candidate, &answer, None).unwrap();
        assert_eq!(
            words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["Hello "]
        );
        assert_eq!(dropped, 1);
    }

    /// A PNG's signature and `IHDR` stating `w` x `h` pixels, then bytes nobody decodes.
    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        b.extend(w.to_be_bytes());
        b.extend(h.to_be_bytes());
        b.extend(b" and the rest of the file");
        b
    }

    /// A JPEG's start, a JFIF segment, and a baseline frame header stating `w` x `h` pixels.
    fn jpeg(w: u16, h: u16) -> Vec<u8> {
        jpeg_with(b"", 0xC0, w, h)
    }

    /// A JPEG's start, a JFIF segment, `between`, and a frame header `sof` stating `w` x `h`.
    fn jpeg_with(between: &[u8], sof: u8, w: u16, h: u16) -> Vec<u8> {
        let mut b =
            b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00".to_vec();
        b.extend(between);
        b.extend([0xFF, sof, 0x00, 0x0B, 0x08]);
        b.extend(h.to_be_bytes());
        b.extend(w.to_be_bytes());
        b.extend(b"\x01\x01\x11\x00 and the rest of the file");
        b
    }

    #[test]
    fn an_image_file_is_one_page_of_its_own_pixels() {
        let profile = Profile::ocr_v0();
        // The fake decodes nothing: a signature and a header are all the engine reads of the file.
        let png = png(20, 10);
        let mut fake = Fake::new();
        let artifact = crate::extract_image(&png, &profile, &mut fake).expect("extracts");
        assert_eq!(fake.requests.len(), 1);
        assert!(matches!(fake.requests[0].image, ImageSpec::File));
        assert_eq!(
            (fake.requests[0].orientation, &fake.requests[0].data),
            (0, &png),
            "the file whole, as stored"
        );
        // The source is the file, not the page it was set on.
        assert_eq!(artifact.source.media_type, "image/png");
        assert_eq!(
            artifact.source.sha256.as_str(),
            format!("sha256:{}", ethos_parser_core::c14n::sha256_hex_bytes(&png))
        );
        // 20 x 10 pixels is 20 x 10 points, and a word lies where the recognizer read it.
        let page = &artifact.pages[0];
        assert_eq!((page.width, page.height), (2000, 1000));
        let r = &page.runs[0];
        assert_eq!(
            (r.text.as_str(), r.locator.origin_x, r.locator.origin_y),
            ("Hello ", 200, 800)
        );
        assert_eq!((r.locator.advance, r.font_size), (Some(700), 600));
        let source = r.recognized.as_ref().expect("recognized");
        assert_eq!(source.image, None, "no image object: the page is the image");
        assert_eq!(source.pixels, [[2, 2], [9, 2], [9, 8], [2, 8]]);
        assert!(codes(&artifact).contains(&ethos_parser_core::codes::IMAGE_RESOLUTION_NOT_READ));
        let rep = crate::to_representation(&artifact, &profile).expect("seals");
        assert_eq!(rep.payload().source.media_type, "image/png");
        assert!(
            rep.payload()
                .nodes
                .iter()
                .all(|n| n.kind != ethos_parser_core::NodeKind::Image),
            "the page is the image, so no image node is written"
        );
        let node = rep
            .payload()
            .nodes
            .iter()
            .find(|n| n.text == "Hello ")
            .expect("the run is a node");
        let NativeLocator::Recognized(at) = &node.native_locator else {
            panic!("a recognized run is addressed as one");
        };
        assert_eq!(at.image, None);
    }

    #[test]
    fn an_image_file_the_recognizer_refuses_is_refused_by_name() {
        let profile = Profile::ocr_v0();
        let mut fake = Fake::new();
        fake.refused = Some("not-decoded".into());
        let e = crate::extract_image(&jpeg(20, 10), &profile, &mut fake)
            .expect_err("an image file has no other page to count it against");
        assert!(
            e.to_string().contains("image/jpeg") && e.to_string().contains("not-decoded"),
            "{e}"
        );
    }

    #[test]
    fn an_image_file_the_recognizer_sizes_otherwise_is_refused_by_name() {
        // Decision #132: the page is built from the size the recognizer states, so a size the
        // file's own header does not state is refused, as `image-size-mismatch` refuses a PDF's.
        let profile = Profile::ocr_v0();
        for (bytes, media_type) in [
            (png(30, 10), "image/png"),
            (png(10, 20), "image/png"),
            (png(20, 30), "image/png"),
            (jpeg(10, 20), "image/jpeg"),
            (jpeg(30, 0), "image/jpeg"),
        ] {
            let e = crate::extract_image(&bytes, &profile, &mut Fake::new())
                .expect_err("the fake answers 20 x 10");
            assert!(
                e.to_string().contains(media_type) && e.to_string().contains("image-size-mismatch"),
                "{e}"
            );
        }
        // A JPEG may state a height of 0 and set it with a DNL marker after its first scan
        // (T.81 B.2.2): a height of 0 states none, so the width alone is compared.
        for bytes in [jpeg(20, 10), jpeg(20, 0)] {
            let artifact =
                crate::extract_image(&bytes, &profile, &mut Fake::new()).expect("extracts");
            assert_eq!(
                (artifact.pages[0].width, artifact.pages[0].height),
                (2000, 1000)
            );
        }
    }

    #[test]
    fn an_image_files_size_is_read_from_its_header_before_it_is_sent() {
        // Decision #132: a JPEG's size is its first frame header's, found as libjpeg finds it:
        // past tables, fill bytes and bytes that are no marker.
        let dqt = [&[0xFF, 0xDB, 0x00, 0x43][..], &[0; 65]].concat();
        let dht = [&[0xFF, 0xC4, 0x00, 0x1F][..], &[0; 29]].concat();
        for (label, between, sof) in [
            ("baseline", Vec::new(), 0xC0),
            ("tables first", [dqt.clone(), dht].concat(), 0xC0),
            ("progressive", dqt, 0xC2),
            ("fill bytes", vec![0xFF, 0xFF, 0xFF], 0xC0),
            ("stray bytes", b"junk".to_vec(), 0xC0),
        ] {
            assert_eq!(
                image_file_size(&jpeg_with(&between, sof, 20, 10)),
                Some((20, 10)),
                "{label}"
            );
        }
        assert_eq!(image_file_size(b"\xFF\xD8\xFF\xD9"), None, "it ends first");
        let profile = Profile::ocr_v0();
        let artifact = crate::extract_image(
            &jpeg_with(b"junk", 0xC0, 20, 10),
            &profile,
            &mut Fake::new(),
        )
        .expect("extracts");
        assert_eq!(artifact.pages[0].width, 2000);
        // A header that states no size is refused by its own name, before the recognizer is asked.
        for (bytes, media_type) in [
            (&b"\xFF\xD8\xFF\xE0 a JPEG"[..], "image/jpeg"),
            (b"\x89PNG\r\n\x1a\n and no header", "image/png"),
        ] {
            let mut fake = Fake::new();
            let e = crate::extract_image(bytes, &profile, &mut fake).expect_err("no size");
            assert!(
                e.to_string().contains(media_type) && e.to_string().contains("image-size-not-read"),
                "{e}"
            );
            assert!(fake.requests.is_empty(), "nothing is sent");
        }
    }

    #[test]
    fn only_png_and_jpeg_are_image_files() {
        assert_eq!(image_media_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
        assert_eq!(image_media_type(b"\xFF\xD8\xFF\xDB"), Some("image/jpeg"));
        assert_eq!(image_media_type(b"%PDF-1.7"), None);
        assert_eq!(
            image_media_type(b"\x89PNG"),
            None,
            "a signature is all of it"
        );
        assert_eq!(
            image_media_type(b"\xFF\xD8\x00"),
            None,
            "a JPEG's first marker follows"
        );
        let mut fake = Fake::new();
        let e = crate::extract_image(b"GIF89a", &Profile::ocr_v0(), &mut fake)
            .expect_err("refused by name");
        assert!(e.to_string().contains("PNG or a JPEG"), "{e}");
        assert!(
            fake.requests.is_empty(),
            "refused before the recognizer is asked"
        );
        // A profile that pins no recognizer is refused before the file is read, as a PDF is.
        let e = crate::extract_image(b"\x89PNG\r\n\x1a\n", &Profile::default(), &mut fake)
            .expect_err("no pin");
        assert!(e.to_string().contains("pins none"), "{e}");
        assert!(fake.requests.is_empty());
    }
}
