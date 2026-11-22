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

//! A page painted with several images, composed into one raster (decision #156): a scan stored as
//! a background and the masks its text is stencilled with, or cut into strips. The engine sends
//! what the page painted, in paint order — each image with the rectangle it fills and the part of
//! it the clip shows, and each rectangle a fill painted — and this draws it on white, as a viewer
//! does, before the page is read.
//!
//! **Integers only, and nearest samples**: a raster pixel takes the stored pixel under its centre,
//! a soft mask's sample weighs the image over what lies beneath with one rounding, so every CPU
//! draws the same bytes. Nothing is smoothed: the recognizer reads text, not a picture.

use serde::Deserialize;

use crate::decode::{self, PdfImage};
use crate::pipeline::Bgr;

/// A page to compose, as the engine resolved it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Composite {
    pub width: u32,
    pub height: u32,
    /// Each image the page painted, once however often it painted it.
    pub sources: Vec<Source>,
    pub layers: Vec<Layer>,
}

/// One image a page painted: its dictionary, its stream (`bytes` long), and its soft mask, whose
/// stream follows its own.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub image: PdfImage,
    pub bytes: u64,
    pub soft_mask: Option<SoftMask>,
}

/// One paint, in the raster's pixels: rectangles are `[x0, y0, x1, y1]`, holding the pixels whose
/// top-left corner `(x, y)` has `x0 <= x < x1` and `y0 <= y < y1`.
#[derive(Debug, Deserialize)]
#[serde(tag = "paint", rename_all = "snake_case", deny_unknown_fields)]
pub enum Layer {
    /// The image [`Composite::sources`] holds at `source`: the rectangle its raster fills, stood
    /// upright as `orientation` says; the part of it shown; the colour a stencil mask paints, red,
    /// green, blue.
    Image {
        source: u32,
        rect: [i64; 4],
        shown: [i64; 4],
        orientation: u8,
        colour: Option<[u8; 3]>,
    },
    /// A rectangle filled in one colour, red, green, blue.
    Fill { rect: [i64; 4], colour: [u8; 3] },
}

/// An image's soft mask: a gray image, how much of the image shows at each sample.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftMask {
    pub image: PdfImage,
    pub bytes: u64,
}

/// The most pixels a composed raster may hold; the engine asks for at most 40 million.
const MAX_PIXELS: u64 = 64_000_000;

/// The most raster pixels a page's paints may cover between them, each as often as painted: the
/// engine asks for at most 400 million.
const MAX_WORK: u128 = 400_000_000;

/// The most pixels a page's images and soft masks may hold between them, each once: every one is
/// decoded whole before the page is drawn (second review: eight 6000 x 6000 masks took 2.6 GB).
/// The engine asks for at most 64 million.
const MAX_SOURCE_PIXELS: u128 = 64_000_000;

/// `spec` drawn on white from `data`, or why it is not, by name.
pub fn compose(spec: &Composite, data: &[u8]) -> Result<Bgr, String> {
    let (w, h) = (spec.width as usize, spec.height as usize);
    if w == 0 || h == 0 {
        return Err("image-empty".into());
    }
    if spec.width as u64 * spec.height as u64 > MAX_PIXELS {
        return Err("composite-too-large".into());
    }
    let mut canvas = Bgr {
        w,
        h,
        px: vec![255; w * h * 3],
    };
    let work: u128 = spec
        .layers
        .iter()
        .map(|layer| match layer {
            Layer::Image { rect, shown, .. } => area(&canvas, *rect, *shown),
            Layer::Fill { rect, .. } => area(&canvas, *rect, *rect),
        })
        .sum();
    let decoded: u128 = spec
        .sources
        .iter()
        .flat_map(|s| std::iter::once(&s.image).chain(s.soft_mask.as_ref().map(|m| &m.image)))
        .map(|i| u128::from(i.width) * u128::from(i.height))
        .sum();
    if work > MAX_WORK || decoded > MAX_SOURCE_PIXELS {
        return Err("composite-too-large".into());
    }
    // Each source's bytes, in order; each decoded once, and stood upright once for each
    // orientation a paint of it asks.
    let mut at = 0usize;
    let mut bytes = Vec::with_capacity(spec.sources.len());
    for source in &spec.sources {
        let image = take(data, &mut at, source.bytes)?;
        let mask = match &source.soft_mask {
            Some(m) => Some(take(data, &mut at, m.bytes)?),
            None => None,
        };
        bytes.push((image, mask));
    }
    if at != data.len() {
        return Err("request-bytes-not-read".into());
    }
    let mut decoded: Vec<Option<(Bgr, Option<Bgr>)>> = spec.sources.iter().map(|_| None).collect();
    let mut stood: std::collections::BTreeMap<(usize, u8), (Bgr, Option<Bgr>)> = Default::default();
    for layer in &spec.layers {
        match layer {
            Layer::Fill { rect, colour } => {
                let [r, g, b] = *colour;
                for (_, _, o) in pixels(&canvas, *rect, *rect) {
                    canvas.px[o..o + 3].copy_from_slice(&[b, g, r]);
                }
            }
            Layer::Image {
                source,
                rect,
                shown,
                orientation,
                colour,
            } => {
                if *orientation > 7 {
                    return Err(format!("orientation-not-read: {orientation}"));
                }
                let k = *source as usize;
                let src = spec.sources.get(k).ok_or("source-not-read")?;
                // A stencil is painted through its colour, not weighed by a mask.
                if src.image.image_mask && src.soft_mask.is_some() {
                    return Err("soft-mask-not-read".into());
                }
                let stencil = if src.image.image_mask {
                    Some(colour.ok_or("fill-colour-not-read")?)
                } else {
                    None
                };
                if decoded[k].is_none() && !stood.contains_key(&(k, *orientation)) {
                    let (image, mask) = bytes[k];
                    let alpha = match (&src.soft_mask, mask) {
                        (Some(m), Some(b)) => Some(decode::pdf(&m.image, b)?),
                        _ => None,
                    };
                    decoded[k] = Some((decode::pdf(&src.image, image)?, alpha));
                }
                let (img, alpha) = match stood.entry((k, *orientation)) {
                    std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                    std::collections::btree_map::Entry::Vacant(e) => {
                        let (stored, mask) = decoded[k].as_ref().expect("decoded above");
                        e.insert((
                            crate::upright(stored, *orientation),
                            mask.as_ref().map(|m| crate::upright(m, *orientation)),
                        ))
                    }
                };
                draw(&mut canvas, img, alpha.as_ref(), *rect, *shown, stencil);
            }
        }
    }
    Ok(canvas)
}

/// The next `n` bytes of `data` from `at`.
fn take<'a>(data: &'a [u8], at: &mut usize, n: u64) -> Result<&'a [u8], String> {
    let end = usize::try_from(n)
        .ok()
        .and_then(|n| at.checked_add(n))
        .filter(|&end| end <= data.len())
        .ok_or("request-bytes-short")?;
    let slice = &data[*at..end];
    *at = end;
    Ok(slice)
}

/// The raster pixels inside both `rect` and `shown`, as `x0, y0, x1, y1`.
fn inside(canvas: &Bgr, rect: [i64; 4], shown: [i64; 4]) -> [i64; 4] {
    let (w, h) = (canvas.w as i64, canvas.h as i64);
    [
        rect[0].max(shown[0]).max(0),
        rect[1].max(shown[1]).max(0),
        rect[2].min(shown[2]).min(w),
        rect[3].min(shown[3]).min(h),
    ]
}

/// How many raster pixels a paint covers.
fn area(canvas: &Bgr, rect: [i64; 4], shown: [i64; 4]) -> u128 {
    let [x0, y0, x1, y1] = inside(canvas, rect, shown);
    (x1 - x0).max(0) as u128 * (y1 - y0).max(0) as u128
}

/// Each raster pixel inside both `rect` and `shown`: its column, its row, and its first byte.
fn pixels(
    canvas: &Bgr,
    rect: [i64; 4],
    shown: [i64; 4],
) -> impl Iterator<Item = (i64, i64, usize)> + use<> {
    let w = canvas.w as i64;
    let [x0, y0, x1, y1] = inside(canvas, rect, shown);
    (y0..y1).flat_map(move |y| (x0..x1).map(move |x| (x, y, ((y * w + x) * 3) as usize)))
}

/// The stored sample under the centre of raster pixel `v` of a span from `origin`, `span` pixels
/// long, over `n` samples. In 128 bits: a placement millions of samples wide and far off the page
/// overflows 64.
fn sample(v: i64, origin: i64, span: i64, n: usize) -> usize {
    let n = n as i128;
    let s = (2 * (i128::from(v) - i128::from(origin)) + 1) * n / (2 * i128::from(span));
    s.clamp(0, n - 1) as usize
}

/// `img` over `rect`, inside `shown`: a stencil mask paints `stencil` where it is black, an image
/// shows where `alpha` says, each over what lies beneath.
fn draw(
    canvas: &mut Bgr,
    img: &Bgr,
    alpha: Option<&Bgr>,
    rect: [i64; 4],
    shown: [i64; 4],
    stencil: Option<[u8; 3]>,
) {
    let (rw, rh) = (rect[2] - rect[0], rect[3] - rect[1]);
    if rw <= 0 || rh <= 0 || img.w == 0 || img.h == 0 {
        return;
    }
    for (x, y, o) in pixels(canvas, rect, shown) {
        let s = (sample(y, rect[1], rh, img.h) * img.w + sample(x, rect[0], rw, img.w)) * 3;
        let src = [img.px[s], img.px[s + 1], img.px[s + 2]];
        if let Some([r, g, b]) = stencil {
            if src == [0, 0, 0] {
                canvas.px[o..o + 3].copy_from_slice(&[b, g, r]);
            }
            continue;
        }
        let a = match alpha {
            Some(m) if m.w > 0 && m.h > 0 => {
                let k = (sample(y, rect[1], rh, m.h) * m.w + sample(x, rect[0], rw, m.w)) * 3;
                u32::from(m.px[k])
            }
            _ => 255,
        };
        for (dst, fg) in canvas.px[o..o + 3].iter_mut().zip(src) {
            let (fg, bg) = (u32::from(fg), u32::from(*dst));
            *dst = ((fg * a + bg * (255 - a) + 127) / 255) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A gray image `w` by `h`, 8 bits a sample.
    fn gray(w: u32, h: u32) -> PdfImage {
        serde_json::from_value(serde_json::json!({
            "width": w, "height": h, "bits_per_component": 8, "colour": {"space": "gray"},
            "filters": [], "decode_parms": [], "decode": null, "image_mask": false
        }))
        .expect("a gray image")
    }

    /// A one-bit stencil mask `w` by `h`.
    fn stencil(w: u32, h: u32) -> PdfImage {
        serde_json::from_value(serde_json::json!({
            "width": w, "height": h, "bits_per_component": 1, "colour": null,
            "filters": [], "decode_parms": [], "decode": null, "image_mask": true
        }))
        .expect("a stencil mask")
    }

    fn source(image: PdfImage, bytes: u64) -> Source {
        Source {
            image,
            bytes,
            soft_mask: None,
        }
    }

    /// Source `k` over `rect`, all of it shown, upright.
    fn paint(k: u32, rect: [i64; 4]) -> Layer {
        Layer::Image {
            source: k,
            rect,
            shown: rect,
            orientation: 0,
            colour: None,
        }
    }

    fn at(c: &Bgr, x: usize, y: usize) -> [u8; 3] {
        let o = (y * c.w + x) * 3;
        [c.px[o], c.px[o + 1], c.px[o + 2]]
    }

    #[test]
    fn a_background_and_its_stencilled_text_make_one_page() {
        // A 2 x 2 gray background over the whole 4 x 4 raster, then a 4 x 4 stencil painting red
        // where its samples are 0 (the default /Decode): its first row only.
        let spec = Composite {
            width: 4,
            height: 4,
            sources: vec![source(gray(2, 2), 4), source(stencil(4, 4), 4)],
            layers: vec![
                paint(0, [0, 0, 4, 4]),
                Layer::Image {
                    source: 1,
                    rect: [0, 0, 4, 4],
                    shown: [0, 0, 4, 4],
                    orientation: 0,
                    colour: Some([255, 0, 0]),
                },
            ],
        };
        let data = [10, 20, 30, 40, 0x00, 0xF0, 0xF0, 0xF0];
        let c = compose(&spec, &data).expect("composes");
        assert_eq!(at(&c, 0, 0), [0, 0, 255], "the stencil paints red, as BGR");
        assert_eq!(at(&c, 3, 0), [0, 0, 255]);
        // Below it the background, each sample over a 2 x 2 block.
        assert_eq!(at(&c, 0, 1), [10, 10, 10]);
        assert_eq!(at(&c, 3, 2), [40, 40, 40]);
        assert_eq!(at(&c, 1, 3), [30, 30, 30]);
    }

    #[test]
    fn one_source_painted_twice_is_sent_once() {
        // The left and right halves of a 2 x 1 raster, both from one gray sample.
        let spec = Composite {
            width: 2,
            height: 1,
            sources: vec![source(gray(1, 1), 1)],
            layers: vec![paint(0, [0, 0, 1, 1]), paint(0, [1, 0, 2, 1])],
        };
        let c = compose(&spec, &[70]).expect("composes");
        assert_eq!((at(&c, 0, 0), at(&c, 1, 0)), ([70; 3], [70; 3]));
    }

    #[test]
    fn a_raster_pixel_takes_the_sample_under_its_centre() {
        // Three samples over two pixels: the centres fall on the first and the last.
        let spec = Composite {
            width: 2,
            height: 1,
            sources: vec![source(gray(3, 1), 3)],
            layers: vec![paint(0, [0, 0, 2, 1])],
        };
        let c = compose(&spec, &[0, 100, 200]).expect("composes");
        assert_eq!((at(&c, 0, 0), at(&c, 1, 0)), ([0, 0, 0], [200, 200, 200]));
    }

    #[test]
    fn a_fill_paints_over_what_is_beneath_inside_its_rectangle() {
        let spec = Composite {
            width: 4,
            height: 2,
            sources: vec![source(gray(1, 1), 1)],
            layers: vec![
                paint(0, [0, 0, 4, 2]),
                Layer::Fill {
                    rect: [1, 0, 3, 1],
                    colour: [0, 0, 255],
                },
            ],
        };
        let c = compose(&spec, &[50]).expect("composes");
        assert_eq!(at(&c, 0, 0), [50, 50, 50]);
        assert_eq!(at(&c, 1, 0), [255, 0, 0], "blue, as BGR");
        assert_eq!(at(&c, 2, 0), [255, 0, 0]);
        assert_eq!(at(&c, 3, 0), [50, 50, 50]);
        assert_eq!(at(&c, 1, 1), [50, 50, 50]);
    }

    #[test]
    fn a_clip_shows_part_of_an_image_and_the_rest_stays_white() {
        let mut layer = paint(0, [0, 0, 4, 1]);
        if let Layer::Image { shown, .. } = &mut layer {
            *shown = [2, 0, 9, 1];
        }
        let spec = Composite {
            width: 4,
            height: 1,
            sources: vec![source(gray(1, 1), 1)],
            layers: vec![layer],
        };
        let c = compose(&spec, &[0]).expect("composes");
        assert_eq!(c.px, [255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn a_soft_mask_weighs_an_image_over_what_lies_beneath() {
        // Black under a mask of 0, 128 and 255: white, half gray, black.
        let spec = Composite {
            width: 3,
            height: 1,
            sources: vec![Source {
                image: gray(1, 1),
                bytes: 1,
                soft_mask: Some(SoftMask {
                    image: gray(3, 1),
                    bytes: 3,
                }),
            }],
            layers: vec![paint(0, [0, 0, 3, 1])],
        };
        let c = compose(&spec, &[0, 0, 128, 255]).expect("composes");
        assert_eq!(at(&c, 0, 0), [255, 255, 255]);
        assert_eq!(at(&c, 1, 0), [127, 127, 127]);
        assert_eq!(at(&c, 2, 0), [0, 0, 0]);
    }

    #[test]
    fn an_image_is_stood_upright_before_it_is_drawn() {
        // A 2 x 1 image, samples 0 then 255, turned a quarter clockwise: 1 x 2, 0 on top.
        let mut layer = paint(0, [0, 0, 1, 2]);
        if let Layer::Image { orientation, .. } = &mut layer {
            *orientation = 1;
        }
        let spec = Composite {
            width: 1,
            height: 2,
            sources: vec![source(gray(2, 1), 2)],
            layers: vec![layer],
        };
        let c = compose(&spec, &[0, 255]).expect("composes");
        assert_eq!(at(&c, 0, 0), [0, 0, 0]);
        assert_eq!(at(&c, 0, 1), [255, 255, 255]);
    }

    #[test]
    fn bytes_short_or_left_over_are_refused_by_name() {
        let spec = Composite {
            width: 1,
            height: 1,
            sources: vec![source(gray(1, 1), 1)],
            layers: vec![paint(0, [0, 0, 1, 1])],
        };
        assert_eq!(compose(&spec, &[]).unwrap_err(), "request-bytes-short");
        assert_eq!(
            compose(&spec, &[1, 2]).unwrap_err(),
            "request-bytes-not-read"
        );
    }

    #[test]
    fn a_stencil_with_no_colour_or_with_a_soft_mask_is_refused_by_name() {
        let spec = Composite {
            width: 1,
            height: 1,
            sources: vec![source(stencil(1, 1), 1)],
            layers: vec![paint(0, [0, 0, 1, 1])],
        };
        assert_eq!(compose(&spec, &[0]).unwrap_err(), "fill-colour-not-read");
        let spec = Composite {
            width: 1,
            height: 1,
            sources: vec![Source {
                image: stencil(1, 1),
                bytes: 1,
                soft_mask: Some(SoftMask {
                    image: gray(1, 1),
                    bytes: 1,
                }),
            }],
            layers: vec![Layer::Image {
                source: 0,
                rect: [0, 0, 1, 1],
                shown: [0, 0, 1, 1],
                orientation: 0,
                colour: Some([0, 0, 0]),
            }],
        };
        assert_eq!(compose(&spec, &[0, 0]).unwrap_err(), "soft-mask-not-read");
    }

    #[test]
    fn paints_covering_too_much_are_refused_by_name() {
        // A 20,000 x 2,000 raster painted eleven times over is past the 400 million.
        let spec = Composite {
            width: 20_000,
            height: 2_000,
            sources: vec![source(gray(1, 1), 1)],
            layers: (0..11).map(|_| paint(0, [0, 0, 20_000, 2_000])).collect(),
        };
        assert_eq!(compose(&spec, &[0]).unwrap_err(), "composite-too-large");
    }

    #[test]
    fn images_too_large_to_decode_together_are_refused_by_name() {
        // Second review: every source is decoded whole, so 120 million samples painted into one
        // pixel are refused before any is decoded.
        let spec = Composite {
            width: 10,
            height: 10,
            sources: vec![source(gray(12_000, 10_000), 1)],
            layers: vec![paint(0, [0, 0, 1, 1])],
        };
        assert_eq!(compose(&spec, &[0]).unwrap_err(), "composite-too-large");
    }
}
