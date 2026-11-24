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

//! An image's bytes to an 8-bit BGR raster: a PDF image XObject's stream as stored, or an image
//! file (`docs/35-OCR-SCOPE.md` §4). What is not read is refused by name, never guessed.

use std::io::Read;

use serde::Deserialize;

use crate::pipeline::Bgr;

/// What a PDF image XObject's dictionary says, as the engine resolved it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfImage {
    pub width: u32,
    pub height: u32,
    pub bits_per_component: u32,
    /// Absent for an image mask.
    pub colour: Option<Colour>,
    /// `/Filter`, in order.
    pub filters: Vec<String>,
    /// `/DecodeParms`, one per filter.
    pub decode_parms: Vec<Option<FlateParms>>,
    pub decode: Option<Vec<f64>>,
    pub image_mask: bool,
}

/// A colour space, resolved by the engine to the components it holds.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(tag = "space", rename_all = "snake_case", deny_unknown_fields)]
pub enum Colour {
    Gray,
    Rgb,
    Cmyk,
    Indexed {
        base: Base,
        hival: u32,
        lookup: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Base {
    Gray,
    Rgb,
    Cmyk,
}

impl Base {
    fn comps(self) -> usize {
        match self {
            Base::Gray => 1,
            Base::Rgb => 3,
            Base::Cmyk => 4,
        }
    }
}

/// `/DecodeParms` of a Flate stream (32000-1 Table 8).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlateParms {
    pub predictor: u32,
    pub colors: u32,
    pub bits_per_component: u32,
    pub columns: u32,
}

/// An image file: PNG or JPEG, told by its first bytes.
pub fn file(data: &[u8]) -> Result<Bgr, String> {
    let format = match data {
        [0x89, b'P', b'N', b'G', ..] => image::ImageFormat::Png,
        [0xFF, 0xD8, 0xFF, ..] => image::ImageFormat::Jpeg,
        _ => return Err("image-format-not-read".into()),
    };
    let img = image::load_from_memory_with_format(data, format)
        .map_err(|_| "image-not-decoded".to_string())?;
    if !img.color().has_alpha() {
        return Ok(from_rgb(img.to_rgb8()));
    }
    // A viewer shows a transparent pixel over white, so it is read over white: dropped, the alpha
    // would turn black text on a transparent black ground all black. Integers, so every CPU agrees.
    let rgba = img.to_rgba8();
    let over_white = |c: u8, a: u8| {
        let (c, a) = (u32::from(c), u32::from(a));
        ((c * a + 255 * (255 - a) + 127) / 255) as u8
    };
    Ok(from_rgb(image::RgbImage::from_fn(
        rgba.width(),
        rgba.height(),
        |x, y| {
            let [r, g, b, a] = rgba.get_pixel(x, y).0;
            image::Rgb([over_white(r, a), over_white(g, a), over_white(b, a)])
        },
    )))
}

fn from_rgb(rgb: image::RgbImage) -> Bgr {
    let (w, h) = (rgb.width() as usize, rgb.height() as usize);
    let px = rgb
        .as_raw()
        .as_chunks::<3>()
        .0
        .iter()
        .flat_map(|&[r, g, b]| [b, g, r])
        .collect();
    Bgr { w, h, px }
}

/// A PDF image XObject's stream, as stored, under `spec`.
pub fn pdf(spec: &PdfImage, data: &[u8]) -> Result<Bgr, String> {
    let (w, h) = (spec.width as usize, spec.height as usize);
    if w == 0 || h == 0 {
        return Err("image-empty".into());
    }
    let mut bytes = data.to_vec();
    for (i, f) in spec.filters.iter().enumerate() {
        match f.as_str() {
            "FlateDecode" | "Fl" => {
                let mut out = Vec::new();
                flate2::read::ZlibDecoder::new(&bytes[..])
                    .read_to_end(&mut out)
                    .map_err(|_| "stream-not-decoded".to_string())?;
                bytes = match spec.decode_parms.get(i).cloned().flatten() {
                    Some(p) if p.predictor > 1 => unpredict(&out, &p)?,
                    _ => out,
                };
            }
            "DCTDecode" | "DCT" if i + 1 == spec.filters.len() => return dct(spec, &bytes),
            other => return Err(format!("filter-not-decoded:{other}")),
        }
    }
    samples(spec, &bytes)
}

/// A JPEG stream: gray or RGB, inverted where `/Decode` says so; four inks or a palette's
/// indices by `dct_samples`.
fn dct(spec: &PdfImage, data: &[u8]) -> Result<Bgr, String> {
    let comps = match &spec.colour {
        Some(Colour::Gray) => 1,
        Some(Colour::Rgb) => 3,
        Some(Colour::Cmyk | Colour::Indexed { .. }) => return dct_samples(spec, data),
        None => return Err("colour-space-not-read:jpeg".into()),
    };
    let img = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
        .map_err(|_| "stream-not-decoded".to_string())?;
    let mut out = from_rgb(img.to_rgb8());
    if (out.w, out.h) != (spec.width as usize, spec.height as usize) {
        return Err("image-size-mismatch".into());
    }
    match spec.decode.as_deref() {
        None => {}
        Some(d) if d.chunks(2).all(|p| p == [0.0, 1.0]) => {}
        Some(d) if d.len() == 2 * comps && d.chunks(2).all(|p| p == [1.0, 0.0]) => {
            out.px.iter_mut().for_each(|v| *v = 255 - *v);
        }
        Some(_) => return Err("decode-array-not-read".into()),
    }
    Ok(out)
}

/// A JPEG stream of four inks, or of one index into a palette (decision #172): its samples as
/// stored, read as a Flate stream's are, through the colour space and `/Decode`. The filter undoes
/// only the JPEG's own colour transform (32000-1 §7.4.8) — an Adobe YCCK file's back to CMYK, as
/// libjpeg does — so the inverted inks an Adobe file stores are its `/Decode`'s to turn.
fn dct_samples(spec: &PdfImage, data: &[u8]) -> Result<Bgr, String> {
    use zune_jpeg::zune_core::{
        bytestream::ZCursor, colorspace::ColorSpace, options::DecoderOptions,
    };
    // The options `image` decodes the gray and RGB ones with.
    let options = DecoderOptions::default()
        .set_strict_mode(false)
        .set_max_width(usize::MAX)
        .set_max_height(usize::MAX);
    let mut jpeg = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), options);
    jpeg.decode_headers()
        .map_err(|_| "stream-not-decoded".to_string())?;
    let stored = jpeg.input_colorspace().ok_or("stream-not-decoded")?;
    let fits = match spec.colour {
        Some(Colour::Cmyk) => matches!(stored, ColorSpace::CMYK | ColorSpace::YCCK),
        _ => stored == ColorSpace::Luma,
    };
    if !fits {
        return Err("colour-space-not-read:jpeg".into());
    }
    jpeg.set_options(jpeg.options().jpeg_set_out_colorspace(stored));
    let mut px = jpeg
        .decode()
        .map_err(|_| "stream-not-decoded".to_string())?;
    if jpeg.dimensions() != Some((spec.width as usize, spec.height as usize)) {
        return Err("image-size-mismatch".into());
    }
    if stored == ColorSpace::YCCK {
        // libjpeg's YCbCr to RGB in 16-bit fixed point (jdcolor.c), each ink 255 less it.
        for p in px.as_chunks_mut::<4>().0 {
            let (y, cb, cr) = (
                i32::from(p[0]),
                i32::from(p[1]) - 128,
                i32::from(p[2]) - 128,
            );
            let ink = |d: i32| (255 - (y + d).clamp(0, 255)) as u8;
            p[0] = ink((91881 * cr + 32768) >> 16);
            p[1] = ink((-22554 * cb - 46802 * cr + 32768) >> 16);
            p[2] = ink((116130 * cb + 32768) >> 16);
        }
    }
    samples(spec, &px)
}

/// Undo a PNG (10-15) or TIFF (2, 8 bits) predictor.
fn unpredict(data: &[u8], p: &FlateParms) -> Result<Vec<u8>, String> {
    // A row's bits past u32, or a row of no bytes, is the page refused by name, not a panic that
    // stops `serve` and with it the whole document.
    let size = || "predictor-not-read:size".to_string();
    let bits = p
        .colors
        .checked_mul(p.bits_per_component)
        .ok_or_else(size)?;
    let bpp = (bits as usize).div_ceil(8).max(1);
    let row = (bits.checked_mul(p.columns).ok_or_else(size)? as usize).div_ceil(8);
    if row == 0 {
        return Err(size());
    }
    if p.predictor == 2 {
        if p.bits_per_component != 8 {
            return Err("predictor-not-read:2".into());
        }
        let mut out = data.to_vec();
        for r in out.chunks_mut(row) {
            for i in bpp..r.len() {
                r[i] = r[i].wrapping_add(r[i - bpp]);
            }
        }
        return Ok(out);
    }
    if !(10..=15).contains(&p.predictor) {
        return Err(format!("predictor-not-read:{}", p.predictor));
    }
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row];
    for chunk in data.chunks(row + 1) {
        if chunk.len() < row + 1 {
            break;
        }
        let (kind, src) = (chunk[0], &chunk[1..]);
        let mut cur = vec![0u8; row];
        for i in 0..row {
            let a = if i >= bpp { cur[i - bpp] as i16 } else { 0 };
            let b = prev[i] as i16;
            let c = if i >= bpp { prev[i - bpp] as i16 } else { 0 };
            let pred = match kind {
                0 => 0,
                1 => a,
                2 => b,
                3 => (a + b) / 2,
                4 => {
                    let pp = a + b - c;
                    let (pa, pb, pc) = ((pp - a).abs(), (pp - b).abs(), (pp - c).abs());
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => return Err("predictor-not-read:png-row".into()),
            };
            cur[i] = src[i].wrapping_add(pred as u8);
        }
        out.extend_from_slice(&cur);
        prev = cur;
    }
    Ok(out)
}

/// Unpacked samples to BGR, through the colour space and `/Decode`.
fn samples(spec: &PdfImage, data: &[u8]) -> Result<Bgr, String> {
    let (w, h, bpc) = (
        spec.width as usize,
        spec.height as usize,
        spec.bits_per_component,
    );
    if !matches!(bpc, 1 | 2 | 4 | 8 | 16) {
        return Err(format!("bits-per-component-not-read:{bpc}"));
    }
    let comps = match (&spec.colour, spec.image_mask) {
        (_, true) => 1,
        (Some(Colour::Gray), _) => 1,
        (Some(Colour::Rgb), _) => 3,
        (Some(Colour::Cmyk), _) => 4,
        (Some(Colour::Indexed { .. }), _) => 1,
        (None, false) => return Err("colour-space-not-read".into()),
    };
    let stride = (w * comps * bpc as usize).div_ceil(8);
    if data.len() < stride * h {
        return Err("stream-short".into());
    }
    let max = ((1u32 << bpc.min(16)) - 1) as f64;
    let sample = |row: &[u8], i: usize| -> u32 {
        match bpc {
            8 => row[i] as u32,
            16 => u16::from_be_bytes([row[2 * i], row[2 * i + 1]]) as u32,
            _ => {
                let bit = i * bpc as usize;
                let byte = row[bit / 8] as u32;
                (byte >> (8 - bpc as usize - bit % 8)) & ((1 << bpc) - 1)
            }
        }
    };
    // Decode: component value v of `max` maps to dmin + v * (dmax - dmin) / max. An index's
    // default is [0 max], the identity (32000-1 Table 90).
    let default: Vec<f64> = match &spec.colour {
        Some(Colour::Indexed { .. }) if !spec.image_mask => vec![0.0, max],
        _ => [0.0, 1.0].repeat(comps),
    };
    let dec = spec.decode.clone().unwrap_or(default);
    if dec.len() != 2 * comps {
        return Err("decode-array-not-read".into());
    }
    let to8 = |v: u32, k: usize| -> u8 {
        let d = dec[2 * k] + v as f64 * (dec[2 * k + 1] - dec[2 * k]) / max;
        (d * 255.0).round_ties_even().clamp(0.0, 255.0) as u8
    };
    let mut px = vec![0u8; w * h * 3];
    for y in 0..h {
        let row = &data[y * stride..(y + 1) * stride];
        for x in 0..w {
            let bgr: [u8; 3] = if spec.image_mask {
                // Sample 0 paints under the default /Decode [0 1]; [1 0] makes 1 paint.
                let paint = (sample(row, x) == 0) == (dec[0] == 0.0);
                if paint {
                    [0, 0, 0]
                } else {
                    [255, 255, 255]
                }
            } else {
                match spec.colour.as_ref().ok_or("colour-space-not-read")? {
                    Colour::Gray => {
                        let g = to8(sample(row, x), 0);
                        [g, g, g]
                    }
                    Colour::Rgb => {
                        let (r, g, b) = (
                            to8(sample(row, 3 * x), 0),
                            to8(sample(row, 3 * x + 1), 1),
                            to8(sample(row, 3 * x + 2), 2),
                        );
                        [b, g, r]
                    }
                    Colour::Cmyk => {
                        let c: [u8; 4] = std::array::from_fn(|k| to8(sample(row, 4 * x + k), k));
                        cmyk(c)
                    }
                    Colour::Indexed {
                        base,
                        hival,
                        lookup,
                    } => {
                        let d = dec[0] + sample(row, x) as f64 * (dec[1] - dec[0]) / max;
                        let i = (d.round_ties_even().clamp(0.0, *hival as f64)) as usize;
                        let n = base.comps();
                        let e = lookup
                            .get(i * n..(i + 1) * n)
                            .ok_or("indexed-lookup-short")?;
                        match base {
                            Base::Gray => [e[0], e[0], e[0]],
                            Base::Rgb => [e[2], e[1], e[0]],
                            Base::Cmyk => cmyk([e[0], e[1], e[2], e[3]]),
                        }
                    }
                }
            };
            px[(y * w + x) * 3..(y * w + x) * 3 + 3].copy_from_slice(&bgr);
        }
    }
    Ok(Bgr { w, h, px })
}

/// CMYK to BGR, the naive way (32000-1 §10.4.2 with no undercolour removal).
fn cmyk([c, m, y, k]: [u8; 4]) -> [u8; 3] {
    let inv = |v: u8| 255 - (v as u16 + k as u16).min(255) as u8;
    [inv(y), inv(m), inv(c)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(w: u32, h: u32, bpc: u32, colour: Option<Colour>) -> PdfImage {
        PdfImage {
            width: w,
            height: h,
            bits_per_component: bpc,
            colour,
            filters: vec![],
            decode_parms: vec![],
            decode: None,
            image_mask: false,
        }
    }

    #[test]
    fn one_bit_gray_reads_zero_as_black() {
        let img = pdf(&spec(8, 1, 1, Some(Colour::Gray)), &[0b1010_0000]).unwrap();
        let first: Vec<u8> = img.px.chunks(3).map(|p| p[0]).collect();
        assert_eq!(first, [255, 0, 255, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn an_image_mask_paints_its_zeros_unless_decode_inverts() {
        let mut s = spec(4, 1, 1, None);
        s.image_mask = true;
        let img = pdf(&s, &[0b0101_0000]).unwrap();
        assert_eq!(
            img.px.chunks(3).map(|p| p[0]).collect::<Vec<_>>(),
            [0, 255, 0, 255]
        );
        s.decode = Some(vec![1.0, 0.0]);
        let img = pdf(&s, &[0b0101_0000]).unwrap();
        assert_eq!(
            img.px.chunks(3).map(|p| p[0]).collect::<Vec<_>>(),
            [255, 0, 255, 0]
        );
    }

    #[test]
    fn png_predictors_round_trip() {
        // Two rows of three gray bytes, filtered Sub then Up.
        let p = FlateParms {
            predictor: 15,
            colors: 1,
            bits_per_component: 8,
            columns: 3,
        };
        let data = [1, 10, 5, 5, 2, 1, 1, 1];
        assert_eq!(unpredict(&data, &p).unwrap(), [10, 15, 20, 11, 16, 21]);
    }

    #[test]
    fn a_transparent_pixel_is_read_over_white() {
        // PR 53's review: with the alpha dropped, black text on a transparent black ground was all
        // black. A transparent black pixel then an opaque one: white, then black.
        let mut png = Vec::new();
        image::RgbaImage::from_raw(2, 1, vec![0, 0, 0, 0, 0, 0, 0, 255])
            .unwrap()
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        assert_eq!(file(&png).unwrap().px, [255, 255, 255, 0, 0, 0]);
    }

    #[test]
    fn a_predictor_row_past_u32_or_of_no_bytes_is_refused_by_name() {
        // PR 53's review: 2^29 columns of one 8-bit colour is 2^32 bits a row, which wrapped to a
        // row of no bytes, and `chunks_mut(0)` panicked; `/Columns 0` reached the same panic.
        let mut p = FlateParms {
            predictor: 2,
            colors: 1,
            bits_per_component: 8,
            columns: 1 << 29,
        };
        assert_eq!(
            unpredict(&[0; 8], &p).unwrap_err(),
            "predictor-not-read:size"
        );
        p.columns = 0;
        assert_eq!(
            unpredict(&[0; 8], &p).unwrap_err(),
            "predictor-not-read:size"
        );
    }

    #[test]
    fn indexed_rgb_looks_up_its_palette() {
        let c = Colour::Indexed {
            base: Base::Rgb,
            hival: 1,
            lookup: vec![255, 0, 0, 0, 0, 255],
        };
        let img = pdf(&spec(2, 1, 8, Some(c)), &[1, 0]).unwrap();
        assert_eq!(img.px, [255, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn an_unread_filter_is_refused_by_name() {
        let mut s = spec(1, 1, 8, Some(Colour::Gray));
        s.filters = vec!["JPXDecode".into()];
        assert_eq!(pdf(&s, &[0]).unwrap_err(), "filter-not-decoded:JPXDecode");
    }

    /// A baseline JPEG of 8 by 8 samples, each component one flat value: a quantisation table of
    /// 8s, so a value v is the DC coefficient v - 128; a DC table whose code for each category is
    /// the category in four bits; an AC table of the end of block alone. `adobe` writes an APP14
    /// segment with that colour transform.
    fn flat_jpeg(values: &[u8], adobe: Option<u8>) -> Vec<u8> {
        let n = values.len() as u8;
        let mut j = vec![0xFF, 0xD8];
        if let Some(t) = adobe {
            j.extend([0xFF, 0xEE, 0, 14]);
            j.extend(b"Adobe");
            j.extend([0, 100, 0, 0, 0, 0, t]);
        }
        j.extend([0xFF, 0xDB, 0, 67, 0]);
        j.extend([8; 64]);
        j.extend([0xFF, 0xC0, 0, 8 + 3 * n, 8, 0, 8, 0, 8, n]);
        (1..=n).for_each(|c| j.extend([c, 0x11, 0]));
        j.extend([0xFF, 0xC4, 0, 31, 0x00, 0, 0, 0, 12]);
        j.extend([0; 12]);
        j.extend(0..12);
        j.extend([0xFF, 0xC4, 0, 20, 0x10, 1]);
        j.extend([0; 16]);
        j.extend([0xFF, 0xDA, 0, 6 + 2 * n, n]);
        (1..=n).for_each(|c| j.extend([c, 0]));
        j.extend([0, 63, 0]);
        let mut bits = Vec::new();
        for &v in values {
            let d = i32::from(v) - 128;
            let category = 32 - d.unsigned_abs().leading_zeros();
            let extra = if d < 0 { d - 1 } else { d };
            bits.extend((0..4).rev().map(|k| (category >> k) & 1 == 1));
            bits.extend((0..category).rev().map(|k| (extra >> k) & 1 == 1));
            bits.push(false);
        }
        bits.resize(bits.len().div_ceil(8) * 8, true);
        for byte in bits.chunks(8) {
            let b = byte.iter().fold(0u8, |a, &x| (a << 1) | u8::from(x));
            j.push(b);
            if b == 0xFF {
                j.push(0);
            }
        }
        j.extend([0xFF, 0xD9]);
        j
    }

    fn dct_spec(width: u32, colour: Colour, decode: Option<Vec<f64>>) -> PdfImage {
        let mut s = spec(width, 8, 8, Some(colour));
        s.filters = vec!["DCTDecode".into()];
        s.decode = decode;
        s
    }

    #[test]
    fn a_cmyk_jpeg_is_read_as_its_inks_through_decode() {
        // Decision #172: inks C 64, M 32, Y 16, K 8 as stored, and stored inverted, as an Adobe
        // file stores them, under `/Decode [1 0 1 0 1 0 1 0]`: the same colour.
        let inks = pdf(
            &dct_spec(8, Colour::Cmyk, None),
            &flat_jpeg(&[64, 32, 16, 8], Some(0)),
        )
        .unwrap();
        assert!(inks.px.chunks(3).all(|p| p == [231, 215, 183]));
        let inverted = pdf(
            &dct_spec(8, Colour::Cmyk, Some([1.0, 0.0].repeat(4))),
            &flat_jpeg(&[191, 223, 239, 247], Some(0)),
        )
        .unwrap();
        assert_eq!(inverted.px, inks.px);
    }

    #[test]
    fn an_adobe_ycck_jpeg_is_turned_back_to_its_inks() {
        // Y 150, Cb 168, Cr 98 is RGB 108, 158, 221 by libjpeg, as MuPDF decodes it: inks 147, 97
        // and 34, over K 10.
        let img = pdf(
            &dct_spec(8, Colour::Cmyk, None),
            &flat_jpeg(&[150, 168, 98, 10], Some(2)),
        )
        .unwrap();
        assert!(img.px.chunks(3).all(|p| p == [211, 148, 98]));
    }

    #[test]
    fn an_indexed_jpeg_looks_up_its_palette() {
        let c = Colour::Indexed {
            base: Base::Rgb,
            hival: 1,
            lookup: vec![255, 0, 0, 0, 0, 255],
        };
        let img = pdf(&dct_spec(8, c, None), &flat_jpeg(&[1], None)).unwrap();
        assert!(img.px.chunks(3).all(|p| p == [255, 0, 0]));
    }

    #[test]
    fn a_jpeg_its_colour_space_does_not_hold_or_of_another_size_is_refused_by_name() {
        let gray = Colour::Indexed {
            base: Base::Gray,
            hival: 255,
            lookup: (0..=255).collect(),
        };
        for (colour, values, adobe) in [
            (Colour::Cmyk, &[200, 90, 60][..], None),
            (Colour::Cmyk, &[1], None),
            (gray, &[64, 32, 16, 8], Some(0)),
        ] {
            let jpeg = flat_jpeg(values, adobe);
            let refused = pdf(&dct_spec(8, colour, None), &jpeg).unwrap_err();
            assert_eq!(refused, "colour-space-not-read:jpeg");
        }
        let jpeg = flat_jpeg(&[64, 32, 16, 8], Some(0));
        let refused = pdf(&dct_spec(4, Colour::Cmyk, None), &jpeg).unwrap_err();
        assert_eq!(refused, "image-size-mismatch");
    }
}
