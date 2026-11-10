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

//! PP-OCRv5 mobile, detection then recognition (`docs/35-OCR-SCOPE.md`).
//!
//! The steps around the two models follow PaddleOCR's reference pipeline, measured in Phase 0,
//! except where a step would call a platform maths library or depend on the thread count: the
//! line crop is an affine bilinear warp (PaddleOCR: a perspective bicubic one), each line is read
//! on its own (PaddleOCR: batches of six, which couples a line's result to its neighbours'), and
//! unclip grows the box by `area * 1.5 / perimeter` on every side, whose minimum-area box is
//! pyclipper's. Everything here is integer or IEEE arithmetic in a fixed order, so the output
//! depends only on the input bytes; the models run on rten with fixed-order sums
//! (`vendor/rten-vecmath/ETHOS-PATCH.md`).

use rayon::prelude::*;
use rten::Model;
use rten_imageproc::{find_contours, RetrievalMode};
use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

/// PaddleOCR's normalisation, applied in the image's BGR channel order.
const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];
const THRESH: f32 = 0.3;
const BOX_THRESH: f64 = 0.6;
const UNCLIP: f64 = 1.5;
const MAX_CANDIDATES: usize = 1000;
const MIN_SIZE: f64 = 3.0;
const REC_H: usize = 48;
const REC_W: usize = 320;

/// An 8-bit image, rows of BGR pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bgr {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Bgr {
    fn at(&self, x: usize, y: usize, c: usize) -> u8 {
        self.px[(y * self.w + x) * 3 + c]
    }
}

/// A line's four corners in pixels, clockwise from top-left.
pub type Quad = [[i32; 2]; 4];

/// One line read: its text as recognized, its quad, and its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub quad: Quad,
    pub words: Vec<Word>,
}

/// A word: the line's characters between spaces, and the part of the line's quad they were read
/// from — PaddleOCR's `return_word_box` arithmetic: a word runs from its first character's
/// recognition step to its last character's. A line the crop turned upright keeps the line's
/// quad for each of its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub quad: Quad,
}

/// The two models and the recognizer's alphabet.
pub struct Recognizer {
    det: Model,
    rec: Model,
    chars: Vec<String>,
}

pub const DET: &[u8] = include_bytes!("../models/ch_PP-OCRv5_det_mobile.onnx");
pub const REC: &[u8] = include_bytes!("../models/en_PP-OCRv5_rec_mobile.onnx");
pub const DICT: &str = include_str!("../models/ppocrv5_en_dict.txt");

impl Recognizer {
    pub fn new() -> Result<Recognizer, String> {
        let det = Model::load_static_slice(DET).map_err(|e| format!("detection model: {e}"))?;
        let rec = Model::load_static_slice(REC).map_err(|e| format!("recognition model: {e}"))?;
        let mut chars = vec![String::from("blank")];
        chars.extend(DICT.lines().map(String::from));
        chars.push(" ".into());
        Ok(Recognizer { det, rec, chars })
    }

    /// Every line in `img`, in PaddleOCR's reading order (top to bottom, then left to right
    /// within 10 pixels), its quad in `img`'s pixels.
    pub fn read(&self, img: &Bgr) -> Result<Vec<Line>, String> {
        let prob = run(&self.det, det_input(img))?;
        let (ph, pw) = (prob.shape()[2], prob.shape()[3]);
        let pv: Vec<f32> = prob.iter().copied().collect();
        let quads = boxes(&pv, ph, pw, img.h, img.w);
        // Lines are independent, so they are read in parallel; rten's bits do not depend on the
        // thread count, and the results are collected in order.
        let reads: Vec<Result<Option<Line>, String>> = quads
            .par_iter()
            .map(|q| {
                let Some((c, turned)) = crop(img, q) else {
                    return Ok(None);
                };
                let (x, img_w, rw) = rec_input(&c);
                let (text, chars, steps) = ctc(&run(&self.rec, x)?, &self.chars);
                let words = words(q, &chars, steps, img_w, rw, c.w, turned);
                Ok(Some(Line {
                    text,
                    quad: if turned { reading(q) } else { *q },
                    words,
                }))
            })
            .collect();
        let mut out = Vec::new();
        for r in reads {
            if let Some(line) = r? {
                if !line.text.is_empty() {
                    out.push(line);
                }
            }
        }
        Ok(out)
    }
}

fn run(model: &Model, x: Tensor<f32>) -> Result<Tensor<f32>, String> {
    let y = model
        .run_one(x.view().into(), None)
        .map_err(|e| e.to_string())?;
    y.try_into()
        .map_err(|_| String::from("model output is not f32"))
}

/// One axis of cv2's INTER_LINEAR on 8-bit data: per destination index, the two source indices
/// and their 11-bit fixed-point weights.
fn taps(src: usize, dst: usize) -> Vec<(usize, usize, i64, i64)> {
    let scale = src as f64 / dst as f64;
    (0..dst)
        .map(|d| {
            let f = ((d as f64 + 0.5) * scale - 0.5) as f32;
            let (mut s, mut frac) = (f.floor(), f - f.floor());
            if s < 0.0 {
                (s, frac) = (0.0, 0.0);
            }
            if s >= (src - 1) as f32 {
                (s, frac) = ((src - 1) as f32, 0.0);
            }
            let s = s as usize;
            let w0 = ((1.0 - frac) * 2048.0).round_ties_even() as i64;
            let w1 = (frac * 2048.0).round_ties_even() as i64;
            (s, (s + 1).min(src - 1), w0, w1)
        })
        .collect()
}

/// cv2.resize INTER_LINEAR: integer weights, one rounding at the end.
pub fn resize(img: &Bgr, dw: usize, dh: usize) -> Bgr {
    let (tx, ty) = (taps(img.w, dw), taps(img.h, dh));
    let mut px = vec![0u8; dw * dh * 3];
    for (y, &(y0, y1, b0, b1)) in ty.iter().enumerate() {
        for (x, &(x0, x1, a0, a1)) in tx.iter().enumerate() {
            for c in 0..3 {
                let row = |yy| img.at(x0, yy, c) as i64 * a0 + img.at(x1, yy, c) as i64 * a1;
                let v = (row(y0) * b0 + row(y1) * b1 + (1 << 21)) >> 22;
                px[(y * dw + x) * 3 + c] = v.clamp(0, 255) as u8;
            }
        }
    }
    Bgr { w: dw, h: dh, px }
}

/// PaddleOCR's detection size: no downscale, each side to a multiple of 32, at most 4000.
fn det_size(h: usize, w: usize) -> (usize, usize) {
    let (mut rh, mut rw) = (h as f64, w as f64);
    let m = h.min(w) as f64;
    if m < 64.0 {
        (rh, rw) = ((h as f64 * 64.0 / m).trunc(), (w as f64 * 64.0 / m).trunc());
    }
    if rh.max(rw) > 4000.0 {
        let r = 4000.0 / rh.max(rw);
        (rh, rw) = ((rh * r).trunc(), (rw * r).trunc());
    }
    let r32 = |v: f64| (((v / 32.0).round_ties_even() * 32.0) as usize).max(32);
    (r32(rh), r32(rw))
}

fn det_input(img: &Bgr) -> Tensor<f32> {
    let (rh, rw) = det_size(img.h, img.w);
    let r = resize(img, rw, rh);
    let n = rh * rw;
    let mut data = vec![0f32; 3 * n];
    for (c, plane) in data.chunks_exact_mut(n).enumerate() {
        for (i, v) in plane.iter_mut().enumerate() {
            *v = (r.px[i * 3 + c] as f32 / 255.0 - MEAN[c]) / STD[c];
        }
    }
    Tensor::from_data(&[1, 3, rh, rw], data)
}

/// The four corners ordered as PaddleOCR's `get_mini_boxes`: top-left, top-right, bottom-right,
/// bottom-left, from the two leftmost and the two rightmost.
fn mini_box(c: [[f32; 2]; 4]) -> [[f32; 2]; 4] {
    let mut p = c.to_vec();
    p.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let (i1, i4) = if p[1][1] > p[0][1] { (0, 1) } else { (1, 0) };
    let (i2, i3) = if p[3][1] > p[2][1] { (2, 3) } else { (3, 2) };
    [p[i1], p[i2], p[i3], p[i4]]
}

/// Whether (x, y) lies inside or on the convex quad `q`.
fn inside(q: &[[i64; 2]; 4], x: i64, y: i64) -> bool {
    let (mut pos, mut neg) = (false, false);
    for k in 0..4 {
        let (a, b) = (q[k], q[(k + 1) % 4]);
        let c = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0]);
        pos |= c > 0;
        neg |= c < 0;
    }
    !(pos && neg)
}

/// PaddleOCR's "fast" box score: the mean probability over the pixels the box covers.
fn score_fast(prob: &[f32], ph: usize, pw: usize, q: &[[f32; 2]; 4]) -> f64 {
    let lo = |v: f32, n: usize| (v.floor() as i64).clamp(0, n as i64 - 1);
    let hi = |v: f32, n: usize| (v.ceil() as i64).clamp(0, n as i64 - 1);
    let (xs, ys) = (q.map(|p| p[0]), q.map(|p| p[1]));
    let min = |v: [f32; 4]| v.iter().copied().fold(f32::MAX, f32::min);
    let max = |v: [f32; 4]| v.iter().copied().fold(f32::MIN, f32::max);
    let (xmin, xmax) = (lo(min(xs), pw), hi(max(xs), pw));
    let (ymin, ymax) = (lo(min(ys), ph), hi(max(ys), ph));
    let iq = q.map(|p| [(p[0] - xmin as f32) as i64, (p[1] - ymin as f32) as i64]);
    let (mut sum, mut n) = (0f64, 0u64);
    for y in ymin..=ymax {
        for x in xmin..=xmax {
            if inside(&iq, x - xmin, y - ymin) {
                sum += prob[y as usize * pw + x as usize] as f64;
                n += 1;
            }
        }
    }
    if n == 0 {
        0.0
    } else {
        sum / n as f64
    }
}

/// PaddleOCR's `_clockwise` on an integer box: top-left and bottom-right by x + y, the other two
/// by y - x; the first index on ties.
fn clockwise(b: Quad) -> Quad {
    let pick = |key: &dyn Fn(&[i32; 2]) -> i32, want_max: bool, from: &[usize]| -> usize {
        let mut best = from[0];
        for &i in &from[1..] {
            let (k, kb) = (key(&b[i]), key(&b[best]));
            if (want_max && k > kb) || (!want_max && k < kb) {
                best = i;
            }
        }
        best
    };
    let all = [0, 1, 2, 3];
    let s = |p: &[i32; 2]| p[0] + p[1];
    let (tl, br) = (pick(&s, false, &all), pick(&s, true, &all));
    let rest: Vec<usize> = all
        .iter()
        .copied()
        .filter(|&i| i != tl && i != br)
        .collect();
    let d = |p: &[i32; 2]| p[1] - p[0];
    let (tr, bl) = (pick(&d, false, &rest), pick(&d, true, &rest));
    [b[tl], b[tr], b[br], b[bl]]
}

fn edge(a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dy) = (a[0] - b[0], a[1] - b[1]);
    (dx * dx + dy * dy).sqrt()
}

/// Convex hull of integer points (Andrew's monotone chain), collinear points dropped. Exact.
///
/// Not rten-imageproc's `convex_hull`: its Graham scan visits the points collinear with the lowest
/// one nearest first, so on an unsimplified contour (a text line's flat bottom edge) the hull
/// folds, `min_area_rect` returns a rectangle of height zero, and the line is lost (35 of 49 on a
/// text page, measured 2026-11-10).
pub fn hull(points: &[[i64; 2]]) -> Vec<[i64; 2]> {
    let mut p = points.to_vec();
    p.sort_unstable();
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: [i64; 2], a: [i64; 2], b: [i64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let chain = |pts: &mut dyn Iterator<Item = [i64; 2]>| {
        let mut c: Vec<[i64; 2]> = Vec::new();
        for q in pts {
            while c.len() >= 2 && cross(c[c.len() - 2], c[c.len() - 1], q) <= 0 {
                c.pop();
            }
            c.push(q);
        }
        c.pop();
        c
    };
    let mut lower = chain(&mut p.iter().copied());
    lower.extend(chain(&mut p.iter().rev().copied()));
    lower
}

/// A rotated rectangle: centre, unit axes `u` and `v`, and its extents along them.
#[derive(Clone, Copy, Debug)]
pub struct Rect {
    c: [f64; 2],
    u: [f64; 2],
    v: [f64; 2],
    pub w: f64,
    pub h: f64,
}

impl Rect {
    fn corners(&self) -> [[f32; 2]; 4] {
        let at = |a: f64, b: f64| {
            [
                (self.c[0] + self.u[0] * a + self.v[0] * b) as f32,
                (self.c[1] + self.u[1] * a + self.v[1] * b) as f32,
            ]
        };
        let (hw, hh) = (self.w / 2.0, self.h / 2.0);
        [at(-hw, -hh), at(hw, -hh), at(hw, hh), at(-hw, hh)]
    }
}

/// The minimum-area rectangle around a hull: the one with a side on a hull edge, the first such
/// edge on ties. Areas are compared exactly in integers; only the corners are floating point.
pub fn min_rect(h: &[[i64; 2]]) -> Option<Rect> {
    if h.is_empty() {
        return None;
    }
    if h.len() < 3 {
        let (x0, x1) = (h.iter().map(|p| p[0]).min()?, h.iter().map(|p| p[0]).max()?);
        let (y0, y1) = (h.iter().map(|p| p[1]).min()?, h.iter().map(|p| p[1]).max()?);
        let c = [(x0 + x1) as f64 / 2.0, (y0 + y1) as f64 / 2.0];
        let (w, ht) = ((x1 - x0) as f64, (y1 - y0) as f64);
        return Some(Rect {
            c,
            u: [1.0, 0.0],
            v: [0.0, 1.0],
            w,
            h: ht,
        });
    }
    // Per edge: the spans along and across it, each scaled by the edge's length, and |edge|^2.
    let mut best: Option<(i128, i128, usize, [i64; 4])> = None;
    for i in 0..h.len() {
        let (a, b) = (h[i], h[(i + 1) % h.len()]);
        let e = [b[0] - a[0], b[1] - a[1]];
        let (mut s0, mut s1, mut t0, mut t1) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
        for q in h {
            let d = [q[0] - a[0], q[1] - a[1]];
            let (s, t) = (e[0] * d[0] + e[1] * d[1], e[0] * d[1] - e[1] * d[0]);
            (s0, s1, t0, t1) = (s0.min(s), s1.max(s), t0.min(t), t1.max(t));
        }
        let len2 = (e[0] * e[0] + e[1] * e[1]) as i128;
        let area = (s1 - s0) as i128 * (t1 - t0) as i128; // area * len2
        if best.is_none_or(|(ba, bl, _, _)| area * bl < ba * len2) {
            best = Some((area, len2, i, [s0, s1, t0, t1]));
        }
    }
    let (_, len2, i, [s0, s1, t0, t1]) = best?;
    let (a, b) = (h[i], h[(i + 1) % h.len()]);
    let len = (len2 as f64).sqrt();
    let u = [(b[0] - a[0]) as f64 / len, (b[1] - a[1]) as f64 / len];
    let v = [-u[1], u[0]];
    let (sm, tm) = ((s0 + s1) as f64 / 2.0 / len, (t0 + t1) as f64 / 2.0 / len);
    let c = [
        a[0] as f64 + u[0] * sm + v[0] * tm,
        a[1] as f64 + u[1] * sm + v[1] * tm,
    ];
    Some(Rect {
        c,
        u,
        v,
        w: (s1 - s0) as f64 / len,
        h: (t1 - t0) as f64 / len,
    })
}

/// DB post-processing (PaddleOCR `DBPostProcess`, score mode "fast"), then `filter_tag_det_res`
/// and `sorted_boxes`. `prob` is the detection map, `ph` x `pw`; the quads are in the source
/// image's pixels, `h` x `w`.
fn boxes(prob: &[f32], ph: usize, pw: usize, h: usize, w: usize) -> Vec<Quad> {
    let mask = NdTensor::from_data(
        [ph, pw],
        prob.iter().map(|&p| p > THRESH).collect::<Vec<bool>>(),
    );
    let mut out = Vec::new();
    for poly in find_contours(mask.view(), RetrievalMode::List)
        .iter()
        .take(MAX_CANDIDATES)
    {
        let pts: Vec<[i64; 2]> = poly.iter().map(|p| [p.x as i64, p.y as i64]).collect();
        let Some(rect) = min_rect(&hull(&pts)) else {
            continue;
        };
        if rect.w.min(rect.h) < MIN_SIZE
            || score_fast(prob, ph, pw, &mini_box(rect.corners())) < BOX_THRESH
        {
            continue;
        }
        let d = rect.w * rect.h * UNCLIP / (2.0 * (rect.w + rect.h));
        let grown = Rect {
            w: rect.w + 2.0 * d,
            h: rect.h + 2.0 * d,
            ..rect
        };
        if grown.w.min(grown.h) < MIN_SIZE + 2.0 {
            continue;
        }
        let scale = |v: f32, from: usize, to: usize| {
            (v / from as f32 * to as f32)
                .round_ties_even()
                .clamp(0.0, to as f32) as i32
        };
        let q = mini_box(grown.corners()).map(|p| [scale(p[0], pw, w), scale(p[1], ph, h)]);
        let q = clockwise(q).map(|p| [p[0].clamp(0, w as i32 - 1), p[1].clamp(0, h as i32 - 1)]);
        let f = q.map(|p| [p[0] as f32, p[1] as f32]);
        if (edge(f[0], f[1]) as i32) <= 3 || (edge(f[0], f[3]) as i32) <= 3 {
            continue;
        }
        out.push(q);
    }
    out.sort_by_key(|q| (q[0][1], q[0][0]));
    for i in 0..out.len().saturating_sub(1) {
        for j in (0..=i).rev() {
            if (out[j + 1][0][1] - out[j][0][1]).abs() < 10 && out[j + 1][0][0] < out[j][0][0] {
                out.swap(j, j + 1);
            } else {
                break;
            }
        }
    }
    out
}

/// A turned line's corners in reading order. `crop` turns a tall line a quarter anticlockwise to read
/// it, so its text's top-left is the box's top-right, and so on round: the quad a caller places
/// the text by, rather than the box's corners in the image's order.
fn reading(q: &Quad) -> Quad {
    [q[1], q[2], q[3], q[0]]
}

/// The size of the crop `crop` cuts for quad `q`: its longer top or bottom edge by its longer side.
fn crop_size(q: &Quad) -> (usize, usize) {
    let p = q.map(|v| [v[0] as f32, v[1] as f32]);
    let cw = edge(p[0], p[1]).max(edge(p[2], p[3])) as usize;
    let ch = edge(p[0], p[3]).max(edge(p[1], p[2])) as usize;
    (cw, ch)
}

/// The line under quad `q`, warped upright: the affine map of corners 0, 1 and 3 onto the crop,
/// sampled bilinearly with the border repeated; turned a quarter, and so reported, when 1.5 times
/// taller than wide.
fn crop(img: &Bgr, q: &Quad) -> Option<(Bgr, bool)> {
    let p = q.map(|v| [v[0] as f32, v[1] as f32]);
    let (cw, ch) = crop_size(q);
    if cw == 0 || ch == 0 {
        return None;
    }
    let mut px = vec![0u8; cw * ch * 3];
    let (ux, uy) = (
        (p[1][0] - p[0][0]) / cw as f32,
        (p[1][1] - p[0][1]) / cw as f32,
    );
    let (vx, vy) = (
        (p[3][0] - p[0][0]) / ch as f32,
        (p[3][1] - p[0][1]) / ch as f32,
    );
    let clampi = |v: f32, n: usize| (v as i64).clamp(0, n as i64 - 1) as usize;
    for v in 0..ch {
        for u in 0..cw {
            let sx = p[0][0] + u as f32 * ux + v as f32 * vx;
            let sy = p[0][1] + u as f32 * uy + v as f32 * vy;
            let (fx, fy) = (sx.floor(), sy.floor());
            let (ax, ay) = (sx - fx, sy - fy);
            let (x0, x1) = (clampi(fx, img.w), clampi(fx + 1.0, img.w));
            let (y0, y1) = (clampi(fy, img.h), clampi(fy + 1.0, img.h));
            for c in 0..3 {
                let top = img.at(x0, y0, c) as f32 * (1.0 - ax) + img.at(x1, y0, c) as f32 * ax;
                let bot = img.at(x0, y1, c) as f32 * (1.0 - ax) + img.at(x1, y1, c) as f32 * ax;
                let val = top * (1.0 - ay) + bot * ay;
                px[(v * cw + u) * 3 + c] = val.round_ties_even().clamp(0.0, 255.0) as u8;
            }
        }
    }
    let out = Bgr { w: cw, h: ch, px };
    if ch as f32 / cw as f32 >= 1.5 {
        return Some((turn(&out, 3), true)); // np.rot90: a quarter turn anticlockwise
    }
    Some((out, false))
}

/// The words of a line read from quad `q`: `chars` are its characters with the first and last
/// recognition step each was read at, out of `steps`; the recognition input was `img_w` columns,
/// the crop `fed_w` columns scaled to `rw` of them.
fn words(
    q: &Quad,
    chars: &[(String, usize, usize)],
    steps: usize,
    img_w: usize,
    rw: usize,
    fed_w: usize,
    turned: bool,
) -> Vec<Word> {
    let mut spans: Vec<(String, usize, usize)> = Vec::new();
    let mut open = false;
    for (c, s0, s1) in chars {
        if c == " " {
            open = false;
        } else if open {
            let w = spans.last_mut().expect("an open word");
            w.0.push_str(c);
            w.2 = *s1;
        } else {
            spans.push((c.clone(), *s0, *s1));
            open = true;
        }
    }
    let (cw, ch) = crop_size(q);
    let p = q.map(|v| [v[0] as f64, v[1] as f64]);
    let at = |x: f64, y: f64| -> [i32; 2] {
        let mut out = [0i32; 2];
        for (k, o) in out.iter_mut().enumerate() {
            let v =
                p[0][k] + x * (p[1][k] - p[0][k]) / cw as f64 + y * (p[3][k] - p[0][k]) / ch as f64;
            *o = v.round_ties_even() as i32;
        }
        out
    };
    // One recognition step covers img_w / steps input columns, which are fed_w / rw crop columns.
    let col = |s: usize| {
        (s as f64 * img_w as f64 * fed_w as f64 / (steps as f64 * rw as f64)).min(cw as f64)
    };
    spans
        .into_iter()
        .map(|(text, s0, s1)| {
            let quad = if turned {
                reading(q)
            } else {
                let (xa, xb) = (col(s0), col(s1 + 1));
                [
                    at(xa, 0.0),
                    at(xb, 0.0),
                    at(xb, ch as f64),
                    at(xa, ch as f64),
                ]
            };
            Word { text, quad }
        })
        .collect()
}

/// `img` turned `quarters` quarter turns clockwise.
pub fn turn(img: &Bgr, quarters: u8) -> Bgr {
    let (w, h) = (img.w, img.h);
    let q = quarters % 4;
    let (nw, nh) = if q % 2 == 1 { (h, w) } else { (w, h) };
    let mut px = vec![0u8; nw * nh * 3];
    for y in 0..nh {
        for x in 0..nw {
            let (sx, sy) = match q {
                0 => (x, y),
                1 => (y, h - 1 - x),
                2 => (w - 1 - x, h - 1 - y),
                _ => (w - 1 - y, x),
            };
            for c in 0..3 {
                px[(y * nw + x) * 3 + c] = img.at(sx, sy, c);
            }
        }
    }
    Bgr { w: nw, h: nh, px }
}

/// `img` mirrored left to right.
pub fn mirror(img: &Bgr) -> Bgr {
    let mut px = vec![0u8; img.px.len()];
    for y in 0..img.h {
        for x in 0..img.w {
            for c in 0..3 {
                px[(y * img.w + x) * 3 + c] = img.at(img.w - 1 - x, y, c);
            }
        }
    }
    Bgr {
        w: img.w,
        h: img.h,
        px,
    }
}

/// One line at the recognition height, padded right with zeros to at least 320 columns; with the
/// input's width and the width the line itself was scaled to.
fn rec_input(c: &Bgr) -> (Tensor<f32>, usize, usize) {
    let ratio = c.w as f64 / c.h as f64;
    let img_w = (REC_H as f64 * (REC_W as f64 / REC_H as f64).max(ratio)) as usize;
    let want = (REC_H as f64 * ratio).ceil() as usize;
    let rw = want.min(img_w).max(1);
    let r = resize(c, rw, REC_H);
    let mut data = vec![0f32; 3 * REC_H * img_w];
    for ch in 0..3 {
        for y in 0..REC_H {
            for x in 0..rw {
                let v = r.px[(y * rw + x) * 3 + ch] as f32 / 255.0;
                data[ch * REC_H * img_w + y * img_w + x] = (v - 0.5) / 0.5;
            }
        }
    }
    (Tensor::from_data(&[1, 3, REC_H, img_w], data), img_w, rw)
}

/// CTC greedy decoding: the first maximum per step, repeats collapsed, blanks dropped. The text,
/// each character with the first and last step it held, and the number of steps.
fn ctc(probs: &Tensor<f32>, chars: &[String]) -> (String, Vec<(String, usize, usize)>, usize) {
    let (t, k) = (probs.shape()[1], probs.shape()[2]);
    let v: Vec<f32> = probs.iter().copied().collect();
    let (mut text, mut out, mut prev) = (String::new(), Vec::new(), usize::MAX);
    for s in 0..t {
        let row = &v[s * k..(s + 1) * k];
        let mut best = 0;
        for i in 1..k {
            if row[i] > row[best] {
                best = i;
            }
        }
        if best != 0 && best != prev {
            text.push_str(&chars[best]);
            out.push((chars[best].clone(), s, s));
        } else if best != 0 {
            if let Some(last) = out.last_mut() {
                last.2 = s;
            }
        }
        prev = best;
    }
    (text, out, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_edged_contour_keeps_its_height() {
        // A 40 x 6 bar traced point by point: the case rten-imageproc's hull folded.
        let mut pts = Vec::new();
        for x in 0..40 {
            pts.push([x, 0]);
            pts.push([x, 5]);
        }
        for y in 0..6 {
            pts.push([0, y]);
            pts.push([39, y]);
        }
        let r = min_rect(&hull(&pts)).unwrap();
        assert_eq!((r.w.min(r.h), r.w.max(r.h)), (5.0, 39.0));
    }

    #[test]
    fn four_quarter_turns_are_the_identity() {
        let img = Bgr {
            w: 3,
            h: 2,
            px: (0..18).collect(),
        };
        assert_eq!(turn(&turn(&img, 1), 3), img);
        assert_eq!(turn(&turn(&img, 2), 2), img);
        assert_eq!(mirror(&mirror(&img)), img);
        let t = turn(&img, 1);
        assert_eq!((t.w, t.h), (2, 3));
        // The top-left pixel of a clockwise turn is the source's bottom-left.
        assert_eq!(t.px[..3], img.px[3 * 3..3 * 3 + 3]);
    }

    #[test]
    fn resizing_to_the_same_size_copies() {
        let img = Bgr {
            w: 5,
            h: 4,
            px: (0..60).map(|v| (v * 7 % 251) as u8).collect(),
        };
        assert_eq!(resize(&img, 5, 4), img);
    }
}
