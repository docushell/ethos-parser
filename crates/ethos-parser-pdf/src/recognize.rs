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
    /// A page painted with several images (decision #156): what to compose, in paint order, on a
    /// white raster `width` by `height` pixels; the bytes are each image's stream, and its soft
    /// mask's after it, in layer order.
    Composite(CompositeSpec),
}

/// A page to compose (decision #156).
#[derive(Clone, Debug, Serialize)]
pub struct CompositeSpec {
    /// The raster's width, in pixels.
    pub width: u32,
    /// Its height.
    pub height: u32,
    /// Each image the page painted, once however often it painted it, in the order first painted.
    pub sources: Vec<SourceSpec>,
    /// What the page painted, in paint order.
    pub layers: Vec<LayerSpec>,
}

/// One image a composed page painted (decision #156): what its dictionary states, its stream's
/// length, and its soft mask, whose stream follows its own.
#[derive(Clone, Debug, Serialize)]
pub struct SourceSpec {
    /// What its dictionary states.
    pub image: PdfImageSpec,
    /// Its stream's length.
    pub bytes: u64,
    /// Its `/SMask`.
    pub soft_mask: Option<SoftMaskSpec>,
}

/// One paint of a [`CompositeSpec`]. Rectangles are `[x0, y0, x1, y1]` in the raster's pixels,
/// top-left system: a pixel whose top-left corner `(x, y)` has `x0 <= x < x1` and `y0 <= y < y1`
/// is inside.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "paint", rename_all = "snake_case")]
pub enum LayerSpec {
    /// The image [`CompositeSpec::sources`] holds at `source`: the rectangle its raster fills,
    /// stood upright as `orientation` says, the part of it the page's clip shows, and the colour a
    /// stencil mask paints.
    Image {
        source: u32,
        rect: [i64; 4],
        shown: [i64; 4],
        orientation: u8,
        colour: Option<[u8; 3]>,
    },
    /// A rectangle filled in one colour, red, green and blue.
    Fill { rect: [i64; 4], colour: [u8; 3] },
}

/// An image's `/SMask` (decision #156): a gray image whose samples are how much of the image
/// shows, over the same rectangle.
#[derive(Clone, Debug, Serialize)]
pub struct SoftMaskSpec {
    /// What its dictionary states.
    pub image: PdfImageSpec,
    /// Its stream's length, after the image's own.
    pub bytes: u64,
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
        composite: None,
    }
}

/// A page the trigger took: no text-showing operator, and exactly one image placed (§4), or
/// several, each image and rectangle fill it painted composed into one raster (decision #156).
#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub page_id: lopdf::ObjectId,
    pub object: lopdf::ObjectId,
    /// The unit square's corners (0,0), (1,0), (1,1), (0,1) on the page, in points, top-left
    /// system, `/Rotate` applied: where the image's pixels land. A composed page's raster lies
    /// over the visible box.
    pub corners: [(f64, f64); 4],
    /// The box a viewer shows, in the same system: (x0, y0, x1, y1).
    pub visible: (f64, f64, f64, f64),
    /// What a page painted with several images painted, in paint order (decision #156).
    pub composite: Option<Vec<Paint>>,
}

impl Candidate {
    /// A page painted with several images, composed over its visible box (decision #156).
    pub(crate) fn composed(
        page_id: lopdf::ObjectId,
        visible: (f64, f64, f64, f64),
        paints: Vec<Paint>,
    ) -> Self {
        let (x0, y0, x1, y1) = visible;
        Candidate {
            page_id,
            object: (0, 0),
            corners: [(x0, y1), (x1, y1), (x1, y0), (x0, y0)],
            visible,
            composite: Some(paints),
        }
    }
}

/// One paint of a composed page (decision #156), in the candidate's system.
#[derive(Clone, Debug)]
pub(crate) enum Paint {
    /// An image XObject: where the unit square's corners land, as [`Candidate::corners`] says; the
    /// box the clip shows of it, (x0, y0, x1, y1); and the colour a stencil mask paints, where it
    /// is one this reader knows.
    Image {
        object: lopdf::ObjectId,
        corners: [(f64, f64); 4],
        shown: (f64, f64, f64, f64),
        fill: Option<crate::text_state::Fill>,
    },
    /// A rectangle filled in a device colour, (x0, y0, x1, y1).
    Fill {
        rect: (f64, f64, f64, f64),
        fill: crate::text_state::Fill,
    },
}

/// The most pixels per point a composed page is drawn at (decision #156): 300 dpi, a scan's
/// usual resolution, which a finer mask is sampled down to.
const COMPOSITE_MAX_DENSITY: f64 = 300.0 / 72.0;

/// The most pixels a composed page holds (decision #156): about 40 megapixels, so a poster is drawn
/// coarser rather than refused or allowed a raster without bound.
const COMPOSITE_MAX_PIXELS: f64 = 40.0e6;

/// The most pixels a composed page's paints may cover between them, each counted as often as it
/// is painted (decision #156, review): ten of the largest rasters, so a page that paints one image
/// a thousand times is refused by name rather than drawn for minutes.
pub(crate) const COMPOSITE_MAX_WORK: u128 = 400_000_000;

/// The most paints a composed page may hold (decision #156, review).
pub(crate) const COMPOSITE_MAX_LAYERS: usize = 65_536;

/// The most pixels a composed page's images and their soft masks may hold between them, each
/// counted once (decision #156, second review): the recognizer decodes every one whole before it
/// draws, so eight one-bit masks of 6000 x 6000, a few kilobytes stored, took 2.6 GB. Seven
/// full-page scans at 300 dpi fit; a background and its text's masks need two or three.
pub(crate) const COMPOSITE_MAX_SOURCE_PIXELS: u128 = 64_000_000;

/// A device colour as 8-bit red, green and blue (decision #156): a gray, an RGB, or a CMYK
/// converted as the recognizer converts one (32000-1 §10.4.2, no undercolour removal); `None` for a
/// colour this reader does not know.
fn rgb(fill: crate::text_state::Fill) -> Option<[u8; 3]> {
    let byte = |v: i32| ((v.clamp(0, 1000) * 255 + 500) / 1000) as u8;
    let [a, b, c, k] = fill.components;
    match fill.count {
        1 => Some([byte(a); 3]),
        3 => Some([byte(a), byte(b), byte(c)]),
        4 => {
            let ink = |v: i32| byte(1000 - (v.clamp(0, 1000) + k.clamp(0, 1000)).min(1000));
            Some([ink(a), ink(b), ink(c)])
        }
        _ => None,
    }
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

/// The request for `candidate`'s image, or its composed page's, or the name of what this protocol
/// cannot carry.
pub(crate) fn request(
    doc: &lopdf::Document,
    candidate: &Candidate,
    page_number: u32,
) -> Result<RecognitionRequest, String> {
    if let Some(paints) = &candidate.composite {
        return composed(doc, candidate, paints, page_number);
    }
    let (spec, data, _) = image_spec(doc, candidate.object, false)?;
    let orientation = orientation(candidate.corners, spec.width, spec.height)
        .ok_or("placement-not-axis-aligned")?;
    Ok(RecognitionRequest {
        ocr_request: 1,
        id: format!("page {page_number}"),
        image: ImageSpec::Pdf(spec),
        orientation,
        bytes: data.len() as u64,
        data,
    })
}

/// The request for a page painted with several images (decision #156): each image's stream once,
/// and each paint, placed on a raster over the visible box at the finest density an image of it
/// holds, up to [`COMPOSITE_MAX_DENSITY`] and [`COMPOSITE_MAX_PIXELS`]. An image painted over no
/// area paints nothing and is left out.
fn composed(
    doc: &lopdf::Document,
    candidate: &Candidate,
    paints: &[Paint],
    page_number: u32,
) -> Result<RecognitionRequest, String> {
    let (vx0, vy0, vx1, vy1) = candidate.visible;
    let (pw, ph) = (vx1 - vx0, vy1 - vy0);
    if !(pw > 0.0 && ph > 0.0) {
        return Err("page-empty".into());
    }
    if paints.len() > COMPOSITE_MAX_LAYERS {
        return Err("composite-too-large".into());
    }
    let span = |c: &[(f64, f64); 4]| {
        let (xs, ys) = (c.map(|p| p.0), c.map(|p| p.1));
        let lo = |v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = |v: [f64; 4]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (lo(xs), lo(ys), hi(xs), hi(ys))
    };
    // Each image read once, first: what it is, how each paint of it stands, and the density the
    // page is drawn at.
    let mut sources: Vec<SourceSpec> = Vec::new();
    let mut data = Vec::new();
    let mut read: std::collections::BTreeMap<lopdf::ObjectId, usize> = Default::default();
    let mut placed = Vec::with_capacity(paints.len());
    let mut density: f64 = 1.0;
    for paint in paints {
        let Paint::Image {
            object,
            corners,
            fill,
            ..
        } = paint
        else {
            placed.push(None);
            continue;
        };
        let (x0, y0, x1, y1) = span(corners);
        if !(x1 > x0 && y1 > y0) {
            placed.push(None);
            continue;
        }
        let source = match read.get(object) {
            Some(&k) => k,
            None => {
                let (image, bytes, mask) = image_spec(doc, *object, true)?;
                // A stencil is painted through its colour, not weighed by a mask (review).
                if image.image_mask && mask.is_some() {
                    return Err("soft-mask-not-read".into());
                }
                data.extend_from_slice(&bytes);
                let soft_mask = mask.map(|(image, bytes)| {
                    data.extend_from_slice(&bytes);
                    SoftMaskSpec {
                        image,
                        bytes: bytes.len() as u64,
                    }
                });
                sources.push(SourceSpec {
                    image,
                    bytes: bytes.len() as u64,
                    soft_mask,
                });
                read.insert(*object, sources.len() - 1);
                sources.len() - 1
            }
        };
        let spec = &sources[source].image;
        let orientation =
            orientation(*corners, spec.width, spec.height).ok_or("placement-not-axis-aligned")?;
        let colour = match (spec.image_mask, fill) {
            (true, Some(f)) => Some(rgb(*f).ok_or("fill-colour-not-read")?),
            (true, None) => return Err("fill-colour-not-read".into()),
            (false, _) => None,
        };
        let (across, down) = if orientation % 2 == 1 {
            (spec.height, spec.width)
        } else {
            (spec.width, spec.height)
        };
        density = density
            .max(f64::from(across) / (x1 - x0))
            .max(f64::from(down) / (y1 - y0));
        placed.push(Some((source as u32, orientation, colour)));
    }
    let density = density
        .min(COMPOSITE_MAX_DENSITY)
        .min((COMPOSITE_MAX_PIXELS / (pw * ph)).sqrt());
    let width = ((pw * density).round() as u32).max(1);
    let height = ((ph * density).round() as u32).max(1);
    let (sx, sy) = (f64::from(width) / pw, f64::from(height) / ph);
    // A placement so far off the page that its pixels are no integer is refused, not clamped:
    // clamped, it would sample another part of its image (review).
    let rect = |(x0, y0, x1, y1): (f64, f64, f64, f64)| -> Result<[i64; 4], String> {
        let at = |v: f64, origin: f64, s: f64| {
            let p = ((v - origin) * s).round();
            (p.abs() <= 1e12).then_some(p as i64)
        };
        match (
            at(x0, vx0, sx),
            at(y0, vy0, sy),
            at(x1, vx0, sx),
            at(y1, vy0, sy),
        ) {
            (Some(a), Some(b), Some(c), Some(d)) => Ok([a, b, c, d]),
            _ => Err("placement-not-read".into()),
        }
    };
    // How many raster pixels a paint covers, as the recognizer draws it.
    let area = |r: [i64; 4], s: [i64; 4]| -> u128 {
        let (x0, y0) = (r[0].max(s[0]).max(0), r[1].max(s[1]).max(0));
        let (x1, y1) = (
            r[2].min(s[2]).min(i64::from(width)),
            r[3].min(s[3]).min(i64::from(height)),
        );
        (x1 - x0).max(0) as u128 * (y1 - y0).max(0) as u128
    };
    let mut layers = Vec::new();
    let mut work: u128 = 0;
    for (paint, placed) in paints.iter().zip(placed) {
        let layer = match (paint, placed) {
            (Paint::Image { corners, shown, .. }, Some((source, orientation, colour))) => {
                let (r, s) = (rect(span(corners))?, rect(*shown)?);
                work += area(r, s);
                LayerSpec::Image {
                    source,
                    rect: r,
                    shown: s,
                    orientation,
                    colour,
                }
            }
            (Paint::Image { .. }, None) => continue,
            (Paint::Fill { rect: r, fill }, _) => {
                let r = rect(*r)?;
                work += area(r, r);
                LayerSpec::Fill {
                    rect: r,
                    colour: rgb(*fill).ok_or("fill-colour-not-read")?,
                }
            }
        };
        layers.push(layer);
    }
    let decoded: u128 = sources
        .iter()
        .flat_map(|s| std::iter::once(&s.image).chain(s.soft_mask.as_ref().map(|m| &m.image)))
        .map(|i| u128::from(i.width) * u128::from(i.height))
        .sum();
    if work > COMPOSITE_MAX_WORK || decoded > COMPOSITE_MAX_SOURCE_PIXELS {
        return Err("composite-too-large".into());
    }
    Ok(RecognitionRequest {
        ocr_request: 1,
        id: format!("page {page_number}"),
        image: ImageSpec::Composite(CompositeSpec {
            width,
            height,
            sources,
            layers,
        }),
        orientation: 0,
        bytes: data.len() as u64,
        data,
    })
}

/// A soft mask's dictionary and stream (decision #156).
type SoftMask = (PdfImageSpec, Vec<u8>);

/// What image `object`'s dictionary states, its stream as stored, and, where `soft_masks` allows
/// one, its `/SMask`'s; or the name of what this protocol cannot carry.
fn image_spec(
    doc: &lopdf::Document,
    object: lopdf::ObjectId,
    soft_masks: bool,
) -> Result<(PdfImageSpec, Vec<u8>, Option<SoftMask>), String> {
    let stream = doc
        .get_object(object)
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
    // what a stencil hides could be read as text no viewer shows (PR 53's review). A composed
    // page carries a soft mask (decision #156) where it is a gray image with no `/Matte`.
    let mut soft_mask = None;
    if dict.has(b"SMask") {
        let read = soft_masks
            .then(|| {
                dict.get(b"SMask")
                    .and_then(lopdf::Object::as_reference)
                    .ok()
            })
            .flatten()
            .and_then(|id| {
                let unmatted = doc
                    .get_object(id)
                    .and_then(lopdf::Object::as_stream)
                    .is_ok_and(|s| !s.dict.has(b"Matte"));
                let (spec, data, _) = image_spec(doc, id, false).ok()?;
                let gray = matches!(spec.colour, Some(Colour::Gray)) && !spec.image_mask;
                (unmatted && gray).then_some((spec, data))
            });
        soft_mask = Some(read.ok_or("soft-mask-not-read")?);
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
    let spec = PdfImageSpec {
        width,
        height,
        bits_per_component,
        colour,
        filters,
        decode_parms,
        decode,
        image_mask,
    };
    Ok((spec, stream.content.clone(), soft_mask))
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
        let mut ops = String::new();
        for _ in 0..images {
            ops.push_str(&format!("q {cm} cm /Im0 Do Q "));
        }
        if let Some(t) = text {
            ops.push_str(&format!("BT /F1 12 Tf 10 50 Td ({t}) Tj ET"));
        }
        page_with(&ops)
    }

    /// The page [`scanned`] makes, drawing `ops`: `/Im0` its 20 x 10 gray image, `/F1` Helvetica.
    fn page_with(ops: &str) -> Vec<u8> {
        page_with_resources(ops, |_, _| {})
    }

    /// [`page_with`], its resources as `more` leaves them.
    fn page_with_resources(
        ops: &str,
        more: impl FnOnce(&mut lopdf::Document, &mut lopdf::Dictionary),
    ) -> Vec<u8> {
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
        let ops = ops.to_string();
        let content = doc.add_object(Stream::new(dictionary! {}, ops.into_bytes()));
        let mut resources = dictionary! {
            "XObject" => dictionary! { "Im0" => image },
            "Font" => dictionary! { "F1" => font },
        };
        more(&mut doc, &mut resources);
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
            "Resources" => resources,
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
        /// The size it answers a composed page at, where not the size asked for.
        composed_at: Option<(u32, u32)>,
    }

    impl Fake {
        fn new() -> Self {
            Fake {
                identity: RecognizerIdentity::ethos_parser_ocr(),
                answers_as: None,
                refused: None,
                requests: Vec::new(),
                reach: 18,
                composed_at: None,
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
            // A composed page is answered at the size it was asked for (decision #156).
            let (width, height) = match &request.image {
                ImageSpec::Composite(c) => self.composed_at.unwrap_or((c.width, c.height)),
                _ => (20, 10),
            };
            Ok(Recognition {
                ocr_response: 1,
                id: request.id.clone(),
                identity: self
                    .answers_as
                    .clone()
                    .unwrap_or_else(|| self.identity.clone()),
                width,
                height,
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
    fn two_images_are_composed_and_read() {
        // Decision #156: a page painted with several images is drawn as one raster over its
        // visible box and read once; its words are placed on the page through that raster.
        let profile = Profile::ocr_v0();
        let doc =
            Document::open_bytes(&scanned("200 0 0 100 0 0", 2, None), &profile).expect("opens");
        let mut fake = Fake::new();
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert_eq!(fake.requests.len(), 1);
        let ImageSpec::Composite(spec) = &fake.requests[0].image else {
            panic!("composed: {:?}", fake.requests[0].image);
        };
        // 20 x 10 samples over 200 x 100 points: under a pixel a point, so one pixel a point.
        assert_eq!((spec.width, spec.height), (200, 100));
        assert_eq!(spec.layers.len(), 2);
        for layer in &spec.layers {
            let LayerSpec::Image {
                source,
                rect,
                shown,
                orientation,
                ..
            } = layer
            else {
                panic!("an image: {layer:?}");
            };
            assert_eq!(
                (*source, *rect, *shown, *orientation),
                (0, [0, 0, 200, 100], [0, 0, 200, 100], 0)
            );
        }
        // One image painted twice is one source, its stream sent once (review).
        assert_eq!(spec.sources.len(), 1);
        assert_eq!(fake.requests[0].data.len(), 200);
        let runs = &artifact.pages[0].runs;
        assert_eq!(
            runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            ["Hello ", "world"]
        );
        // Pixel 2 of 200 across 200 points; the line's foot 8 of 100 pixels down 100 points.
        assert_eq!(
            (runs[0].locator.origin_x, runs[0].locator.origin_y),
            (200, 800)
        );
        assert!(!codes(&artifact).contains(&ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED));
        // Read from no one image, so its locator names none.
        let rep = crate::to_representation(&artifact, &profile).expect("seals");
        let node = rep
            .payload()
            .nodes
            .iter()
            .find(|n| n.text == "Hello ")
            .expect("the run is a node");
        let NativeLocator::Recognized(at) = &node.native_locator else {
            panic!("a recognized run is addressed as one");
        };
        assert!(at.image.is_none());
    }

    #[test]
    fn a_composed_page_is_drawn_at_its_finest_images_density_up_to_300_dpi() {
        let profile = Profile::ocr_v0();
        let size = |ops: &str| {
            let doc = Document::open_bytes(&page_with(ops), &profile).expect("opens");
            let mut fake = Fake::new();
            crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            let ImageSpec::Composite(spec) = &fake.requests[0].image else {
                panic!("composed");
            };
            (spec.width, spec.height)
        };
        // 20 x 10 samples over 10 x 5 points is two a point; the other image's tenth is coarser.
        let fine = "q 200 0 0 100 0 0 cm /Im0 Do Q q 10 0 0 5 0 0 cm /Im0 Do Q";
        assert_eq!(size(fine), (400, 200));
        // Over 2 x 1 points it is ten a point: drawn at 300 dpi, 200 x 100 points at 833 x 417.
        let finer = "q 200 0 0 100 0 0 cm /Im0 Do Q q 2 0 0 1 0 0 cm /Im0 Do Q";
        assert_eq!(size(finer), (833, 417));
    }

    #[test]
    fn a_composed_page_answered_at_another_size_is_refused_by_name() {
        let profile = Profile::ocr_v0();
        let doc =
            Document::open_bytes(&scanned("200 0 0 100 0 0", 2, None), &profile).expect("opens");
        let mut fake = Fake::new();
        fake.composed_at = Some((20, 10));
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert!(artifact.pages[0].runs.is_empty());
        assert_eq!(reason(&artifact).as_deref(), Some("image-size-mismatch"));
    }

    #[test]
    fn paint_a_composition_does_not_hold_refuses_the_page_by_name() {
        // Decision #156: composed without what it painted, a page could show text no viewer sees
        // — a word under a redaction drawn as a curve, or blended away — so it is not composed.
        let profile = Profile::ocr_v0();
        let image = "q 200 0 0 100 0 0 cm /Im0 Do Q";
        for (between, why) in [
            ("0 0 m 50 50 l S", "stroke"),
            ("0 0 m 50 50 50 0 100 0 c f", "path-not-rectangular"),
            ("0 0 10 10 re 20 20 10 10 re f", "path-not-rectangular"),
            ("/Sh0 sh", "shading"),
            ("/GsX gs 0 0 10 10 re f", "graphics-state-not-read"),
            ("0 0 m 100 0 l 50 50 l h W n", "clip-not-rectangular"),
            // Second review: four points doubling back on an edge enclose nothing.
            ("0 0 m 100 0 l 100 50 l 100 0 l f", "path-not-rectangular"),
            ("0 0 m 100 0 l 100 50 l 100 0 l W n", "clip-not-rectangular"),
            ("/P0 cs /Pat scn 0 0 10 10 re f", "fill-colour-not-read"),
            // Four operands in an RGB space: a viewer reads the last three (review).
            (
                "/DeviceRGB cs 0 1 1 1 sc 0 0 10 10 re f",
                "fill-colour-not-read",
            ),
        ] {
            let ops = format!("{image} {between} {image}");
            let doc = Document::open_bytes(&page_with(&ops), &profile).expect("opens");
            let mut fake = Fake::new();
            let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            assert!(fake.requests.is_empty(), "{why}: nothing is sent");
            assert_eq!(
                reason(&artifact).as_deref(),
                Some(format!("several-images:{why}").as_str())
            );
        }
    }

    #[test]
    fn a_space_named_and_no_colour_set_paints_its_initial_black() {
        // Second review: `/DeviceRGB cs` sets the fill to the space's initial colour, black, and a
        // viewer paints it; it was refused `fill-colour-not-read`. In CMYK, `0 0 0 1`.
        let profile = Profile::ocr_v0();
        let image = "q 200 0 0 100 0 0 cm /Im0 Do Q";
        for space in ["DeviceRGB", "DeviceCMYK", "DeviceGray"] {
            let ops = format!("{image} /{space} cs 0 0 10 10 re f {image}");
            let doc = Document::open_bytes(&page_with(&ops), &profile).expect("opens");
            let mut fake = Fake::new();
            crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            assert_eq!(fake.requests.len(), 1, "{space}");
            let ImageSpec::Composite(spec) = &fake.requests[0].image else {
                panic!("composed: {:?}", fake.requests[0].image);
            };
            assert!(
                spec.layers.iter().any(|l| matches!(
                    l,
                    LayerSpec::Fill {
                        colour: [0, 0, 0],
                        ..
                    }
                )),
                "{space}: {:?}",
                spec.layers
            );
        }
    }

    #[test]
    fn a_stencil_paints_a_colour_set_in_a_space_that_reads_as_a_device_ones() {
        // Decision #156: a scan's text stencilled in a colour set with `/Cs1 cs … sc`, where `/Cs1`
        // is an ICC profile of three components, paints that colour. In a space whose components
        // do not read as a device colour's — an ICC profile of two, whatever the operands; `/Lab`,
        // whose three are no RGB — it is not known, and the page is refused by name rather than
        // painted a guessed colour.
        use lopdf::{dictionary, Object, Stream};
        let profile = Profile::ocr_v0();
        let icc = |doc: &mut lopdf::Document, n: i64| {
            let profile = doc.add_object(Stream::new(dictionary! { "N" => n }, vec![]));
            Object::Array(vec![Object::Name(b"ICCBased".to_vec()), profile.into()])
        };
        let lab = |_: &mut lopdf::Document, _: i64| {
            Object::Array(vec![
                Object::Name(b"Lab".to_vec()),
                Object::Dictionary(
                    dictionary! { "WhitePoint" => vec![0.95.into(), 1.into(), 1.09.into()] },
                ),
            ])
        };
        type Space<'a> = &'a dyn Fn(&mut lopdf::Document, i64) -> Object;
        let unknown: [(Space, &str); 2] = [(&icc, "0.5 0.5 0.5 sc"), (&lab, "50 0 0 sc")];
        for (space, colour) in unknown {
            let bytes = page_with_resources(
                &format!(
                    "q 200 0 0 100 0 0 cm /Im0 Do Q /Cs1 cs 0.2 0.4 0.6 sc \
                     q 200 0 0 100 0 0 cm /M0 Do Q /Cs2 cs {colour} q 200 0 0 100 0 0 cm /M0 Do Q"
                ),
                |doc, resources| {
                    let mask = doc.add_object(Stream::new(
                        dictionary! {
                            "Type" => "XObject", "Subtype" => "Image", "Width" => 8, "Height" => 1,
                            "ImageMask" => true,
                        },
                        vec![0x0F],
                    ));
                    let (cs1, cs2) = (icc(doc, 3), space(doc, 2));
                    resources.set("ColorSpace", dictionary! { "Cs1" => cs1, "Cs2" => cs2 });
                    if let Ok(Object::Dictionary(x)) = resources.get_mut(b"XObject") {
                        x.set("M0", mask);
                    }
                },
            );
            let doc = Document::open_bytes(&bytes, &profile).expect("opens");
            let mut fake = Fake::new();
            let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            assert!(
                fake.requests.is_empty(),
                "{colour}: the stencil's colour is not known"
            );
            assert_eq!(
                reason(&artifact).as_deref(),
                Some("fill-colour-not-read"),
                "named by the request, as a stencil with no colour is"
            );
        }
        // Without the second stencil the page is composed, the first painting its colour.
        let bytes = page_with_resources(
            "q 200 0 0 100 0 0 cm /Im0 Do Q /Cs1 cs 0.2 0.4 0.6 sc q 200 0 0 100 0 0 cm /M0 Do Q",
            |doc, resources| {
                let mask = doc.add_object(Stream::new(
                    dictionary! {
                        "Type" => "XObject", "Subtype" => "Image", "Width" => 8, "Height" => 1,
                        "ImageMask" => true,
                    },
                    vec![0x0F],
                ));
                let profile = doc.add_object(Stream::new(dictionary! { "N" => 3 }, vec![]));
                let cs1 = Object::Array(vec![Object::Name(b"ICCBased".to_vec()), profile.into()]);
                resources.set("ColorSpace", dictionary! { "Cs1" => cs1 });
                if let Ok(Object::Dictionary(x)) = resources.get_mut(b"XObject") {
                    x.set("M0", mask);
                }
            },
        );
        let doc = Document::open_bytes(&bytes, &profile).expect("opens");
        let mut fake = Fake::new();
        crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        let ImageSpec::Composite(spec) = &fake.requests[0].image else {
            panic!("composed");
        };
        let LayerSpec::Image { colour, .. } = &spec.layers[1] else {
            panic!("the stencil");
        };
        assert_eq!(*colour, Some([51, 102, 153]));
    }

    /// `bytes` with its document as `edit` leaves it.
    fn edited(bytes: &[u8], edit: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
        let mut doc = lopdf::Document::load_mem(bytes).expect("loads");
        edit(&mut doc);
        let mut out = Vec::new();
        doc.save_to(&mut out).expect("saves");
        out
    }

    /// The reason `--ocr` names for `bytes`'s page, and the requests it sent.
    fn read_ocr(bytes: &[u8]) -> (Option<String>, Vec<RecognitionRequest>) {
        let profile = Profile::ocr_v0();
        let doc = Document::open_bytes(bytes, &profile).expect("opens");
        let mut fake = Fake::new();
        let artifact = crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        (reason(&artifact), fake.requests)
    }

    #[test]
    fn what_a_viewer_hides_or_draws_over_a_composition_is_honoured_or_refused() {
        // Decision #156's review: each probe hid text from a viewer that the first build read.
        use lopdf::{dictionary, Object, Stream};
        let image = "q 200 0 0 100 0 0 cm /Im0 Do Q";
        let two = format!("{image} {image}");
        let first_page = |doc: &lopdf::Document| doc.page_iter().next().expect("a page");
        // An annotation a viewer draws over the page.
        for (flags, why) in [(0, Some("several-images:annotation")), (2, None)] {
            let bytes = edited(&page_with(&two), |doc| {
                let ap = doc.add_object(Stream::new(
                    dictionary! { "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()] },
                    b"0 g 0 0 100 100 re f".to_vec(),
                ));
                let annot = doc.add_object(dictionary! {
                    "Type" => "Annot", "Subtype" => "Square", "F" => flags,
                    "Rect" => vec![0.into(), 0.into(), 100.into(), 100.into()],
                    "AP" => dictionary! { "N" => ap },
                });
                let page = first_page(doc);
                doc.get_object_mut(page)
                    .and_then(Object::as_dict_mut)
                    .expect("the page")
                    .set("Annots", vec![annot.into()]);
            });
            let (got, requests) = read_ocr(&bytes);
            match why {
                Some(why) => assert_eq!(got.as_deref(), Some(why), "flags {flags}"),
                None => assert_eq!(requests.len(), 1, "a hidden annotation draws nothing"),
            }
        }
        // Second review: one with no appearance a viewer draws from its own entries — a square
        // filled white hid the text — but a link, a note, a highlight or a field draws nothing
        // over the page (third review).
        for (subtype, why) in [
            ("Square", Some("several-images:annotation")),
            ("FreeText", Some("several-images:annotation")),
            ("Link", None),
            ("Text", None),
            ("Highlight", None),
            ("Widget", None),
        ] {
            let bytes = edited(&page_with(&two), |doc| {
                let annot = doc.add_object(dictionary! {
                    "Type" => "Annot", "Subtype" => subtype,
                    "Rect" => vec![0.into(), 0.into(), 100.into(), 100.into()],
                    "IC" => vec![1.into(), 1.into(), 1.into()],
                });
                let page = first_page(doc);
                doc.get_object_mut(page)
                    .and_then(Object::as_dict_mut)
                    .expect("the page")
                    .set("Annots", vec![annot.into()]);
            });
            let (got, requests) = read_ocr(&bytes);
            match why {
                Some(why) => assert_eq!(got.as_deref(), Some(why), "{subtype}"),
                None => assert_eq!(requests.len(), 1, "{subtype} draws nothing"),
            }
        }
        // A transfer function maps what is painted to another colour.
        let bytes = page_with_resources(&format!("{image} /Gs1 gs {image}"), |doc, resources| {
            let f = doc.add_object(dictionary! {
                "FunctionType" => 2, "Domain" => vec![0.into(), 1.into()], "N" => 1,
                "C0" => vec![1.into()], "C1" => vec![1.into()],
            });
            resources.set(
                "ExtGState",
                dictionary! { "Gs1" => dictionary! { "TR" => f } },
            );
        });
        assert_eq!(
            read_ocr(&bytes).0.as_deref(),
            Some("several-images:graphics-state-not-read")
        );
        // A form's box turned by its matrix clips to no one rectangle.
        let bytes = page_with_resources(&format!("{image} /Fm0 Do"), |doc, resources| {
            let im0 = match resources.get(b"XObject") {
                Ok(Object::Dictionary(x)) => x.get(b"Im0").expect("Im0").clone(),
                _ => panic!("the page's images"),
            };
            let form = doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form",
                    "BBox" => vec![0.into(), 0.into(), 100.into(), 100.into()],
                    "Matrix" => vec![0.6.into(), 0.8.into(), (-0.8).into(), 0.6.into(), 100.into(), 0.into()],
                    "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => im0 } },
                },
                b"q 0.6 -0.8 0.8 0.6 0 0 cm 200 0 0 100 0 0 cm /Im0 Do Q".to_vec(),
            ));
            if let Ok(Object::Dictionary(x)) = resources.get_mut(b"XObject") {
                x.set("Fm0", form);
            }
        });
        assert_eq!(
            read_ocr(&bytes).0.as_deref(),
            Some("several-images:clip-not-rectangular")
        );
        // An image's own `/OC`: switched off it is not painted; a membership dictionary is not
        // evaluated, and refuses the page.
        for (ocmd, why) in [(false, None), (true, Some("several-images:layer-not-read"))] {
            let bytes = page_with_resources(
                "q 200 0 0 100 0 0 cm /Im0 Do Q q 100 0 0 50 0 0 cm /Im1 Do Q",
                |doc, resources| {
                    let ocg = doc.add_object(dictionary! { "Type" => "OCG", "Name" => "hidden" });
                    let oc: Object = if ocmd {
                        doc.add_object(dictionary! { "Type" => "OCMD", "OCGs" => vec![ocg.into()] })
                            .into()
                    } else {
                        ocg.into()
                    };
                    let hidden = doc.add_object(Stream::new(
                        dictionary! {
                            "Type" => "XObject", "Subtype" => "Image", "Width" => 20,
                            "Height" => 10, "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                            "OC" => oc,
                        },
                        vec![0u8; 200],
                    ));
                    let shown = doc.add_object(Stream::new(
                        dictionary! {
                            "Type" => "XObject", "Subtype" => "Image", "Width" => 20,
                            "Height" => 10, "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                        },
                        vec![255u8; 200],
                    ));
                    if let Ok(Object::Dictionary(x)) = resources.get_mut(b"XObject") {
                        x.set("Im0", hidden);
                        x.set("Im1", shown);
                    }
                    // The catalog is made after the page; it takes these once it is.
                    doc.trailer.set(
                        "PendingOC",
                        dictionary! {
                            "OCGs" => vec![ocg.into()],
                            "D" => dictionary! { "OFF" => vec![ocg.into()] },
                        },
                    );
                },
            );
            let bytes = edited(&bytes, |doc| {
                let props = doc.trailer.remove(b"PendingOC").expect("set above");
                let root = doc
                    .trailer
                    .get(b"Root")
                    .and_then(Object::as_reference)
                    .expect("root");
                doc.get_object_mut(root)
                    .and_then(Object::as_dict_mut)
                    .expect("the catalog")
                    .set("OCProperties", props);
            });
            let (got, requests) = read_ocr(&bytes);
            match why {
                Some(why) => assert_eq!(got.as_deref(), Some(why)),
                None => {
                    let ImageSpec::Composite(spec) = &requests[0].image else {
                        panic!("composed");
                    };
                    assert_eq!(spec.layers.len(), 1, "the hidden image is not painted");
                    assert_eq!(
                        requests[0].data,
                        vec![255u8; 200],
                        "only the shown one sent"
                    );
                }
            }
        }
    }

    #[test]
    fn a_stencil_with_a_soft_mask_is_refused_by_name() {
        use lopdf::{dictionary, Stream};
        let bytes = page_with_resources(
            "q 200 0 0 100 0 0 cm /Im0 Do Q 0 g q 200 0 0 100 0 0 cm /M0 Do Q",
            |doc, resources| {
                let soft = doc.add_object(Stream::new(
                    dictionary! {
                        "Type" => "XObject", "Subtype" => "Image", "Width" => 8, "Height" => 1,
                        "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                    },
                    vec![0u8; 8],
                ));
                let mask = doc.add_object(Stream::new(
                    dictionary! {
                        "Type" => "XObject", "Subtype" => "Image", "Width" => 8, "Height" => 1,
                        "ImageMask" => true, "SMask" => soft,
                    },
                    vec![0x00],
                ));
                if let Ok(lopdf::Object::Dictionary(x)) = resources.get_mut(b"XObject") {
                    x.set("M0", mask);
                }
            },
        );
        assert_eq!(read_ocr(&bytes).0.as_deref(), Some("soft-mask-not-read"));
    }

    #[test]
    fn placements_that_paint_nothing_are_left_out_and_ones_past_reach_refused() {
        // An image painted over no area paints nothing (review: it refused the whole page).
        let ops = "q 0 0 0 0 0 0 cm /Im0 Do Q q 200 0 0 100 0 0 cm /Im0 Do Q";
        let (why, requests) = read_ocr(&page_with(ops));
        assert_eq!(why, None);
        let ImageSpec::Composite(spec) = &requests[0].image else {
            panic!("composed");
        };
        assert_eq!(spec.layers.len(), 1);
        // One whose pixels are past a trillion is refused, not clamped onto another part of it.
        let ops = "q 10000000000000 0 0 10000000000000 0 0 cm /Im0 Do Q \
                   q 200 0 0 100 0 0 cm /Im0 Do Q";
        assert_eq!(
            read_ocr(&page_with(ops)).0.as_deref(),
            Some("placement-not-read")
        );
    }

    #[test]
    fn a_page_painted_over_and_over_is_refused_by_name() {
        // 833 x 417 pixels painted 1,200 times is past the 400 million (review: one image drawn a
        // thousand times took gigabytes).
        let mut ops = String::from("q 2 0 0 1 0 0 cm /Im0 Do Q ");
        for _ in 0..1200 {
            ops.push_str("q 200 0 0 100 0 0 cm /Im0 Do Q ");
        }
        let (why, requests) = read_ocr(&page_with(&ops));
        assert!(requests.is_empty());
        assert_eq!(why.as_deref(), Some("composite-too-large"));
    }

    #[test]
    fn images_too_large_to_decode_together_are_refused_by_name() {
        // Second review: the recognizer decodes every image whole, so 120 million samples stored
        // in a few bytes, painted into a corner, are refused before they are sent.
        use lopdf::{dictionary, Stream};
        let ops = "q 200 0 0 100 0 0 cm /Im0 Do Q q 2 0 0 2 0 0 cm /Im1 Do Q";
        let bytes = page_with_resources(ops, |doc, resources| {
            let big = doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image", "Width" => 12_000,
                    "Height" => 10_000, "ImageMask" => true, "BitsPerComponent" => 1,
                },
                vec![0u8; 8],
            ));
            let xobjects = resources
                .get_mut(b"XObject")
                .and_then(lopdf::Object::as_dict_mut)
                .expect("the page's images");
            xobjects.set("Im1", big);
        });
        let (why, requests) = read_ocr(&bytes);
        assert!(requests.is_empty());
        assert_eq!(why.as_deref(), Some("composite-too-large"));
    }

    #[test]
    fn a_device_space_named_outright_paints_its_colour() {
        // Second review: `/ColorSpace << /Cs1 /DeviceRGB >>` names a device space outright, not in
        // an array; it was read as no colour and the page refused.
        use lopdf::dictionary;
        let profile = Profile::ocr_v0();
        let image = "q 200 0 0 100 0 0 cm /Im0 Do Q";
        let ops = format!("{image} /Cs1 cs 1 0 0 sc 0 0 10 10 re f {image}");
        let bytes = page_with_resources(&ops, |_, resources| {
            resources.set("ColorSpace", dictionary! { "Cs1" => "DeviceRGB" });
        });
        let doc = Document::open_bytes(&bytes, &profile).expect("opens");
        let mut fake = Fake::new();
        crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        assert_eq!(fake.requests.len(), 1);
        let ImageSpec::Composite(spec) = &fake.requests[0].image else {
            panic!("composed: {:?}", fake.requests[0].image);
        };
        assert!(spec.layers.iter().any(|l| matches!(
            l,
            LayerSpec::Fill {
                colour: [255, 0, 0],
                ..
            }
        )));
    }

    #[test]
    fn a_rectangle_naming_a_point_twice_is_composed() {
        // Third review: `m a l a l b …` names a point again at once and draws a rectangle, which
        // the second review's walk refused as a fill and as a clip.
        let profile = Profile::ocr_v0();
        let image = "q 200 0 0 100 0 0 cm /Im0 Do Q";
        for between in [
            "10 10 m 10 10 l 60 10 l 60 40 l 10 40 l h f",
            "10 10 m 60 10 l 60 40 l 10 40 l 10 40 l h f",
            "0 0 m 0 0 l 200 0 l 200 100 l 0 100 l h W n",
        ] {
            let ops = format!("{image} {between} {image}");
            let doc = Document::open_bytes(&page_with(&ops), &profile).expect("opens");
            let mut fake = Fake::new();
            crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
            assert_eq!(fake.requests.len(), 1, "{between}");
        }
    }

    #[test]
    fn fills_and_clips_reach_the_composition_in_paint_order() {
        let profile = Profile::ocr_v0();
        let ops = "q 0 0 100 100 re W n q 200 0 0 100 0 0 cm /Im0 Do Q Q \
                   1 0 0 rg 10 10 20 20 re f q 200 0 0 100 0 0 cm /Im0 Do Q";
        let doc = Document::open_bytes(&page_with(ops), &profile).expect("opens");
        let mut fake = Fake::new();
        crate::extract_recognized(&doc, &profile, &mut fake).expect("extracts");
        let ImageSpec::Composite(spec) = &fake.requests[0].image else {
            panic!("composed");
        };
        let shown: Vec<_> = spec
            .layers
            .iter()
            .map(|l| match l {
                LayerSpec::Image { shown, .. } => ("image", *shown, None),
                LayerSpec::Fill { rect, colour } => ("fill", *rect, Some(*colour)),
            })
            .collect();
        assert_eq!(
            shown,
            [
                ("image", [0, 0, 100, 100], None),
                // (10, 10)-(30, 30) up from the foot is 70..90 down from the top.
                ("fill", [10, 70, 30, 90], Some([255, 0, 0])),
                ("image", [0, 0, 200, 100], None),
            ]
        );
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
        // Decision #156: a page of several images it would compose, too.
        let two = extract(&scanned("200 0 0 100 0 0", 2, None));
        assert_eq!(reason(&two).as_deref(), Some("ocr-not-requested"));
        // Never on a page that shows text, nor on one the trigger itself does not take.
        let image = "q 200 0 0 100 0 0 cm /Im0 Do Q";
        for bytes in [
            scanned("200 0 0 100 0 0", 1, Some("drawn")),
            page_with(&format!("{image} 0 0 m 50 50 l S {image}")),
        ] {
            let other = extract(&bytes);
            assert!(!codes(&other).contains(&ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED));
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
            composite: None,
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
            composite: None,
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
