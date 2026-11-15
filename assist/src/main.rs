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

//! `ethos-parser-assist`: the assist lane's own process (`assist/README.md`).
//!
//! ```text
//! ethos-parser-assist identity                         this build's identity, one JSON line
//! ethos-parser-assist ask SERVER FILE PAGES [MODEL] <PDF   pages PAGES asked; replies to FILE
//! ethos-parser-assist check SERVER [MODEL]             is SERVER ready for the lane?
//! ethos-parser-assist render PDF PAGE OUT.png          page PAGE (from 1) as a PNG; prints `dpi width height`
//! ```
//!
//! `ask` and `check` reach SERVER through the system `curl` and nothing else
//! (`docs/37-ASSIST-TRANSPORT.md`).
//!
//! A page is rendered with hayro by decision #123's rule, which rendered it with Ghostscript: dpi =
//! min(400, floor(sqrt(5e6 / inches))), the inches from the page's size in whole points; pixel
//! sizes rounded as Ghostscript rounds them (hayro floors); a page still over 5e6 pixels shrunk
//! with Lanczos. hayro renders the CropBox, intersected with the MediaBox and rotated. White
//! background, written as an RGB PNG.

mod ask;
mod check;
mod digest;
mod json;
mod transport;

use std::path::Path;
use std::process::ExitCode;

use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{render, RenderCache, RenderSettings};
use image::{imageops, ImageFormat, RgbImage};

/// What renders the pages a model reads: this binary, hayro, and the page rule. It moves whenever
/// any of the three would change a page's pixels.
pub const RENDERER: &str = "ethos-parser-assist 0.1.0; hayro 0.7.1; pages-400dpi-5mp-v1";
/// The one request and reply format this build sends and reads.
pub const REQUEST: &str = "lightonocr-3-grounding-v1";

const MAX_DPI: f64 = 400.0;
const MAX_PIXELS: f64 = 5_000_000.0;

fn identity() -> String {
    format!(
        "{{\"assist\":\"ethos-parser-assist\",\"version\":\"{}\",\"renderer\":\"{RENDERER}\",\"request\":\"{REQUEST}\"}}",
        env!("CARGO_PKG_VERSION")
    )
}

/// The page rule for a page of `w` x `h` points as hayro measures it: the dpi, hayro's scale, and
/// the pixel size. `None` when the size is not one hayro can render (empty, or past `u16`).
fn page_size(w: f32, h: f32) -> Option<(u32, f32, u16, u16)> {
    let inches = ((w as f64).round() / 72.0) * ((h as f64).round() / 72.0);
    let dpi = MAX_DPI.min((MAX_PIXELS / inches).sqrt().floor());
    let scale = (dpi / 72.0) as f32;
    let px = |v: f32| {
        let r = ((v * scale) as f64).round();
        (1.0..=f64::from(u16::MAX)).contains(&r).then_some(r as u16)
    };
    Some((dpi as u32, scale, px(w)?, px(h)?))
}

/// A page drawn by the page rule.
pub struct Rendered {
    pub dpi: u32,
    /// The rendered box's width and height in centipoints: the CropBox, intersected with the
    /// MediaBox and rotated, as hayro draws it.
    pub points: [i64; 2],
    pub width: u32,
    pub height: u32,
    /// The image, as `render` writes it and `ask` sends it.
    pub png: Vec<u8>,
}

/// Page `number` (from 1) of `pdf`, by the page rule. A panic inside the renderer (hayro's
/// JPEG 2000 decoder divides by zero on one ParseBench page) is this page's failure, not the
/// run's: it comes back as an error naming the page.
fn render_page(pdf: &Pdf, number: usize) -> Result<Rendered, String> {
    unpanicked(number, || render_unguarded(pdf, number))
}

/// `f`'s result, or an error naming page `number` where `f` panicked.
fn unpanicked<T>(number: usize, f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
        .unwrap_or_else(|_| Err(format!("page {number}: the renderer failed on this page")))
}

fn render_unguarded(pdf: &Pdf, number: usize) -> Result<Rendered, String> {
    let pages = pdf.pages();
    let page = number
        .checked_sub(1)
        .and_then(|i| pages.get(i))
        .ok_or_else(|| format!("no page {number}: the document has {}", pages.len()))?;
    let (w, h) = page.render_dimensions();
    let (dpi, scale, width, height) =
        page_size(w, h).ok_or_else(|| format!("page {number} is {w} x {h} pt: no pixel size"))?;
    let settings = RenderSettings {
        x_scale: scale,
        y_scale: scale,
        width: Some(width),
        height: Some(height),
        bg_color: WHITE,
    };
    let pixmap = render(
        page,
        &RenderCache::new(),
        &InterpreterSettings::default(),
        &settings,
    );
    let (pw, ph) = (u32::from(pixmap.width()), u32::from(pixmap.height()));
    // On an opaque background premultiplied RGBA is straight RGBA: alpha is dropped.
    let rgb = pixmap
        .data_as_u8_slice()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|&[r, g, b, _]| [r, g, b])
        .collect();
    let mut img = RgbImage::from_raw(pw, ph, rgb).ok_or("hayro's pixmap is not its size")?;
    if f64::from(pw) * f64::from(ph) > MAX_PIXELS {
        let s = (MAX_PIXELS / (f64::from(pw) * f64::from(ph))).sqrt();
        let (sw, sh) = ((f64::from(pw) * s) as u32, (f64::from(ph) * s) as u32);
        img = imageops::resize(&img, sw, sh, imageops::FilterType::Lanczos3);
    }
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| format!("page {number}: {e}"))?;
    let centi = |v: f32| (f64::from(v) * 100.0).round() as i64;
    Ok(Rendered {
        dpi,
        points: [centi(w), centi(h)],
        width: img.width(),
        height: img.height(),
        png,
    })
}

const USAGE: &str = "usage: ethos-parser-assist identity | ask SERVER FILE PAGES [MODEL] <PDF \
                     | check SERVER [MODEL] | render PDF PAGE OUT.png";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let argv: Vec<&str> = args.iter().skip(1).map(String::as_str).collect();
    let result = match argv.as_slice() {
        ["identity"] => {
            println!("{}", identity());
            Ok(())
        }
        ["ask", server, file, pages, model @ ..] if model.len() <= 1 => {
            return match ask::ask(server, Path::new(file), pages, model.first().copied()) {
                Ok(0) => ExitCode::SUCCESS,
                // Done, but some pages were not rendered: they are not in FILE.
                Ok(_) => ExitCode::from(3),
                Err(stop) => {
                    eprintln!("ethos-parser-assist: {}", stop.message);
                    ExitCode::from(stop.code)
                }
            };
        }
        ["check", server, model @ ..] if model.len() <= 1 => {
            return ExitCode::from(check::check(server, model.first().copied()));
        }
        ["render", pdf, page, out] => match page.parse::<usize>() {
            Ok(number) => std::fs::read(pdf)
                .map_err(|e| format!("{pdf}: {e}"))
                .and_then(|data| Pdf::new(data).map_err(|e| format!("not read as a PDF: {e:?}")))
                .and_then(|doc| render_page(&doc, number))
                .and_then(|page| {
                    std::fs::write(out, &page.png).map_err(|e| format!("{out}: {e}"))?;
                    println!("{} {} {}", page.dpi, page.width, page.height);
                    Ok(())
                }),
            Err(_) => Err(format!("PAGE is a page number from 1, not `{page}`")),
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ethos-parser-assist: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A panic in the renderer is the page's error, and a result passes through untouched.
    #[test]
    fn a_renderer_panic_is_the_page_error() {
        let caught: Result<u8, String> = unpanicked(7, || panic!("divide by zero"));
        assert_eq!(
            caught,
            Err("page 7: the renderer failed on this page".into())
        );
        assert_eq!(unpanicked(7, || Ok(1u8)), Ok(1));
        assert_eq!(
            unpanicked::<u8>(7, || Err("no page 7".into())),
            Err("no page 7".into())
        );
    }

    /// The renderer's name moves with what renders: this package's version and hayro's locked one.
    #[test]
    fn the_renderer_names_this_build() {
        let lock = include_str!("../Cargo.lock");
        let hayro = lock
            .split("[[package]]")
            .find_map(|p| p.strip_prefix("\nname = \"hayro\"\nversion = \""))
            .and_then(|v| v.split('"').next())
            .expect("Cargo.lock locks hayro");
        assert_eq!(
            RENDERER,
            format!(
                "ethos-parser-assist {}; hayro {hayro}; pages-400dpi-5mp-v1",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert!(identity().contains(&format!("\"renderer\":\"{RENDERER}\"")));
    }

    /// Ghostscript's sizes for decision #123's pages, which hayro's floor would make 1 px short.
    #[test]
    fn sizes_are_rounded_as_ghostscript_rounds_them() {
        // US Letter and two A4s, as #123's Ghostscript run rendered them (571, 372 and 44 pages).
        assert_eq!(
            page_size(612.0, 792.0).map(|s| (s.0, s.2, s.3)),
            Some((231, 1964, 2541))
        );
        let a4 = page_size(595.28, 841.89).map(|s| (s.0, s.2, s.3));
        assert_eq!(a4, Some((227, 1877, 2654)));
        assert_eq!(
            page_size(595.0, 842.0).map(|s| (s.0, s.2, s.3)),
            Some((227, 1876, 2655))
        );
        // Small pages stop at 400 dpi; nothing renders at no size.
        assert_eq!(
            page_size(72.0, 72.0).map(|s| (s.0, s.2, s.3)),
            Some((400, 400, 400))
        );
        assert_eq!(page_size(0.0, 792.0), None);
        assert_eq!(page_size(f32::NAN, 792.0), None);
    }
}
