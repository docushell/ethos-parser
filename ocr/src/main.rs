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

//! `ethos-parser-ocr`: the recognizer behind `ethos-parser --ocr` (`docs/35-OCR-SCOPE.md`).
//!
//! ```text
//! ethos-parser-ocr serve          requests on stdin, answers on stdout, until stdin closes
//! ethos-parser-ocr identity       this recognizer's identity, one JSON line
//! ethos-parser-ocr image FILE...  read PNG or JPEG files, one answer per file
//! ```
//!
//! **A request** is one JSON line, then exactly `bytes` raw bytes:
//! `{"ocr_request":1,"id":"..","image":{"kind":"file"}|{"kind":"pdf",..},"orientation":0,"bytes":N}`.
//! `orientation` stands the stored raster upright: mirrored left to right first when 4 or more,
//! then turned `orientation % 4` quarter turns clockwise.
//!
//! **An answer** is one JSON line: the request's `id`, this recognizer's identity, the stored
//! raster's size, and its lines — text, and the quad of the text's top-left, top-right,
//! bottom-right and bottom-left corners as read, in the stored raster's pixels — or a named
//! refusal. No score is written: what a recognizer reports about its own certainty is never a
//! filter (`docs/07-VERIFY-BOUNDARY.md` §7).
//!
//! The same request gets the same bytes on every CPU this runs on; an x86-64 CPU without AVX2 and
//! FMA computes differently and is refused before anything is read.

mod decode;
mod pipeline;

use std::io::{BufRead, Read, Write};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

use pipeline::{Bgr, Line, Quad, Recognizer};

/// The pipeline's rule id: moves whenever any step around the models changes what is written.
pub const RULE: &str = "ppocrv5-mobile-v1";
pub const DET_SHA256: &str = "4d97c44a20d30a81aad087d6a396b08f786c4635742afc391f6621f5c6ae78ae";
pub const REC_SHA256: &str = "c3461add59bb4323ecba96a492ab75e06dda42467c9e3d0c18db5d1d21924be8";
pub const DICT_SHA256: &str = "e025a66d31f327ba0c232e03f407ae8d105e1e709e7ccb3f408aa778c24e70d6";
pub const RUNTIME: &str = "rten 0.26.0; rten-vecmath 0.26.0 with fixed-order sums";

#[derive(Serialize, Clone)]
struct Identity {
    recognizer: &'static str,
    version: &'static str,
    rule: &'static str,
    det_sha256: &'static str,
    rec_sha256: &'static str,
    dict_sha256: &'static str,
    runtime: &'static str,
}

fn identity() -> Identity {
    Identity {
        recognizer: "ethos-parser-ocr",
        version: env!("CARGO_PKG_VERSION"),
        rule: RULE,
        det_sha256: DET_SHA256,
        rec_sha256: REC_SHA256,
        dict_sha256: DICT_SHA256,
        runtime: RUNTIME,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    ocr_request: u32,
    id: String,
    image: ImageSpec,
    orientation: u8,
    bytes: u64,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ImageSpec {
    File,
    Pdf(decode::PdfImage),
}

#[derive(Serialize)]
struct LineOut {
    text: String,
    quad: Quad,
    words: Vec<WordOut>,
}

#[derive(Serialize)]
struct WordOut {
    text: String,
    quad: Quad,
}

#[derive(Serialize)]
struct Answer {
    ocr_response: u32,
    id: String,
    identity: Identity,
    width: usize,
    height: usize,
    lines: Vec<LineOut>,
    refused: Option<String>,
}

/// The CPU check: rten's kernels compute differently without AVX2 and FMA.
fn refusal_for_cpu() -> Option<&'static str> {
    #[cfg(target_arch = "x86_64")]
    if !(is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma")) {
        return Some("cpu-lacks-avx2-fma");
    }
    None
}

/// Stand `img` upright as `orientation` says.
fn upright(img: &Bgr, orientation: u8) -> Bgr {
    let m = if orientation >= 4 {
        pipeline::mirror(img)
    } else {
        img.clone()
    };
    pipeline::turn(&m, orientation % 4)
}

/// A point of the upright raster, back in the stored raster's pixels. `w` and `h` are the stored
/// raster's size; points are pixel-corner coordinates.
fn stored_point(p: [i32; 2], orientation: u8, w: usize, h: usize) -> [i32; 2] {
    let turns = orientation % 4;
    // The size after `turns` quarter turns, undone one at a time: a clockwise turn maps (x, y) of a
    // raster of width cw to (cw' - y, x), so its inverse maps (x, y) to (y, cw' - x).
    let (mut x, mut y) = (p[0], p[1]);
    let (mut cw, mut chh) = if turns % 2 == 1 {
        (h as i32, w as i32)
    } else {
        (w as i32, h as i32)
    };
    for _ in 0..turns {
        (x, y) = (y, cw - x);
        (cw, chh) = (chh, cw);
    }
    if orientation >= 4 {
        x = w as i32 - x;
    }
    [x, y]
}

fn answer(rec: &Recognizer, id: String, stored: Result<Bgr, String>, orientation: u8) -> Answer {
    let (img, refused) = match stored {
        Ok(img) => (Some(img), None),
        Err(why) => (None, Some(why)),
    };
    let (w, h) = img.as_ref().map_or((0, 0), |i| (i.w, i.h));
    let mut lines = Vec::new();
    let mut refused = refused;
    if let Some(img) = img {
        match rec.read(&upright(&img, orientation)) {
            Ok(read) => {
                let back = |q: Quad| q.map(|p| stored_point(p, orientation, w, h));
                for Line { text, quad, words } in read {
                    let words = words
                        .into_iter()
                        .map(|wd| WordOut {
                            text: wd.text,
                            quad: back(wd.quad),
                        })
                        .collect();
                    lines.push(LineOut {
                        text,
                        quad: back(quad),
                        words,
                    });
                }
            }
            Err(e) => refused = Some(format!("recognizer-failed:{e}")),
        }
    }
    Answer {
        ocr_response: 1,
        id,
        identity: identity(),
        width: w,
        height: h,
        lines,
        refused,
    }
}

fn serve(rec: &Recognizer) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    loop {
        let mut header = String::new();
        if input.read_line(&mut header).map_err(|e| e.to_string())? == 0 {
            return Ok(());
        }
        let req: Request = serde_json::from_str(header.trim_end())
            .map_err(|e| format!("request-not-read: {e}"))?;
        if req.ocr_request != 1 {
            return Err(format!("request-version-not-read: {}", req.ocr_request));
        }
        if req.orientation > 7 {
            return Err(format!("orientation-not-read: {}", req.orientation));
        }
        let mut data = vec![0u8; req.bytes as usize];
        input
            .read_exact(&mut data)
            .map_err(|e| format!("request-bytes-short: {e}"))?;
        let stored = match &req.image {
            ImageSpec::File => decode::file(&data),
            ImageSpec::Pdf(spec) => decode::pdf(spec, &data),
        };
        let a = answer(rec, req.id, stored, req.orientation);
        serde_json::to_writer(&mut out, &a).map_err(|e| e.to_string())?;
        out.write_all(b"\n")
            .and_then(|_| out.flush())
            .map_err(|e| e.to_string())?;
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str);
    if cmd == Some("identity") {
        println!(
            "{}",
            serde_json::to_string(&identity()).expect("identity serializes")
        );
        return ExitCode::SUCCESS;
    }
    if let Some(why) = refusal_for_cpu() {
        eprintln!(
            "ethos-parser-ocr: refused: {why}: OCR output on this CPU would differ from other CPUs"
        );
        return ExitCode::from(3);
    }
    let rec = match Recognizer::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ethos-parser-ocr: {e}");
            return ExitCode::from(1);
        }
    };
    let result = match cmd {
        Some("serve") if args.len() == 2 => serve(&rec),
        Some("image") if args.len() > 2 => {
            let mut out = std::io::stdout().lock();
            args[2..].iter().try_for_each(|path| {
                let stored = std::fs::read(path)
                    .map_err(|e| format!("{path}: {e}"))
                    .and_then(|d| decode::file(&d));
                let a = answer(&rec, path.clone(), stored, 0);
                serde_json::to_writer(&mut out, &a).map_err(|e| e.to_string())?;
                out.write_all(b"\n").map_err(|e| e.to_string())
            })
        }
        _ => Err("usage: ethos-parser-ocr serve | identity | image FILE...".into()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ethos-parser-ocr: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_names_the_models_built_in() {
        use sha2::{Digest, Sha256};
        let hex = |b: &[u8]| {
            Sha256::digest(b)
                .iter()
                .map(|x| format!("{x:02x}"))
                .collect::<String>()
        };
        assert_eq!(hex(pipeline::DET), DET_SHA256);
        assert_eq!(hex(pipeline::REC), REC_SHA256);
        assert_eq!(hex(pipeline::DICT.as_bytes()), DICT_SHA256);
    }

    #[test]
    fn points_return_to_the_stored_grid() {
        // A 4 x 2 raster: each orientation's upright image maps its corners back to the stored ones.
        let (w, h) = (4usize, 2usize);
        for o in 0..8u8 {
            let img = Bgr {
                w,
                h,
                px: vec![0; w * h * 3],
            };
            let up = upright(&img, o);
            let corners = [
                [0, 0],
                [up.w as i32, 0],
                [up.w as i32, up.h as i32],
                [0, up.h as i32],
            ];
            let mut back: Vec<[i32; 2]> =
                corners.iter().map(|&p| stored_point(p, o, w, h)).collect();
            back.sort();
            assert_eq!(
                back,
                vec![[0, 0], [0, 2], [4, 0], [4, 2]],
                "orientation {o}"
            );
        }
    }

    #[test]
    fn a_clockwise_turn_maps_the_upright_top_left_to_the_stored_bottom_left() {
        assert_eq!(stored_point([0, 0], 1, 4, 2), [0, 2]);
        assert_eq!(stored_point([0, 0], 0, 4, 2), [0, 0]);
        assert_eq!(stored_point([0, 0], 4, 4, 2), [4, 0]);
    }
}
