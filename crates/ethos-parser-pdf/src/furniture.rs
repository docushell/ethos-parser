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

//! **Page furniture: what a page sets apart at its top and bottom edges** — running heads and
//! folios, under `ethos_parser_core::FURNITURE_RULE_V1` (decision #41).
//!
//! A page's lines — the runs sharing one region and one baseline — and the boxes of its tables are
//! read top to bottom and grouped into **bands**: a box joins the band above it when its top sits
//! less than half a **body line** below the band's lowest edge. The body line is the median height
//! of the page's inked runs' measured boxes, each run counted once per character. The page's
//! first band is its **header**, and its last its **footer**, when the band
//!
//! 1. lies wholly inside the page's outer tenth — the top tenth for a header, the bottom for a
//!    footer;
//! 2. holds no table;
//! 3. is not the page's only band.
//!
//! **Where, never what was meant.** A running head and a page title set apart at the top read
//! alike here: the rule says only that a line stands apart at its page's edge. A furniture run
//! stays in the record, in its reading order and in every projection — page furniture is flagged,
//! never dropped, as an `/Artifact` is.
//!
//! The constants were measured, not chosen, on ParseBench's visual-grounding pages
//! (`docs/measurements/parsebench/README.md`), and three clauses were measured and refused: no
//! line taller than the body's, which kept page titles out but cost more running heads set large;
//! a cap on the band's length, which cost long running heads and copyright lines; and counting a
//! painted image as a box, which let a logo beside a running head, or a picture behind the whole
//! page, keep the header out.

use std::collections::{BTreeMap, BTreeSet};

use ethos_parser_core::Furniture;

use crate::nodes::{PageExtract, TextRun};

/// A header lies inside the top one-in-this of its page's height, a footer inside the bottom.
pub const MARGIN_ONE_IN: i64 = 10;

/// A box: `x0, y0, x1, y1`, top-left system, centipoints.
type Rect = (i64, i64, i64, i64);

/// One box on the page as the rule reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Piece {
    /// Its box.
    pub rect: Rect,
    /// A line of text, rather than a table.
    pub text: bool,
}

/// Each piece's furniture, in the order given, on a page `page_height` tall whose body line is
/// `body`.
pub(crate) fn furniture(pieces: &[Piece], page_height: i64, body: i64) -> Vec<Option<Furniture>> {
    let mut out = vec![None; pieces.len()];
    if body <= 0 || page_height <= 0 {
        return out;
    }
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_unstable_by_key(|&k| (pieces[k].rect.1, pieces[k].rect.0, k));
    // Each band: its members, and its lowest edge.
    let mut bands: Vec<(Vec<usize>, i64)> = Vec::new();
    for k in order {
        let rect = pieces[k].rect;
        match bands.last_mut() {
            Some((members, bottom)) if 2 * (rect.1 - *bottom) < body => {
                members.push(k);
                *bottom = (*bottom).max(rect.3);
            }
            _ => bands.push((vec![k], rect.3)),
        }
    }
    let [head, .., foot] = bands.as_slice() else {
        return out;
    };
    let text_only = |members: &[usize]| members.iter().all(|&k| pieces[k].text);
    if MARGIN_ONE_IN * head.1 <= page_height && text_only(&head.0) {
        for &k in &head.0 {
            out[k] = Some(Furniture::Header);
        }
    }
    let top = foot.0.iter().map(|&k| pieces[k].rect.1).min().unwrap_or(0);
    if MARGIN_ONE_IN * (page_height - top) <= page_height && text_only(&foot.0) {
        for &k in &foot.0 {
            out[k] = Some(Furniture::Footer);
        }
    }
    out
}

/// The median height of the inked runs' measured boxes, each run counted once per character; `0`
/// where none is measured.
pub(crate) fn body_line(runs: &[TextRun]) -> i64 {
    let mut heights: Vec<(i64, usize)> = runs
        .iter()
        .filter(|run| !run.text.trim().is_empty())
        .filter_map(|run| {
            let b = run.geometry.measured()?;
            Some((b.y1() - b.y0(), run.text.chars().count()))
        })
        .filter(|&(height, _)| height > 0)
        .collect();
    median_by_chars(&mut heights)
}

/// The height at which half the characters are set lower: `heights` is each run's height and its
/// characters. `0` for none.
fn median_by_chars(heights: &mut [(i64, usize)]) -> i64 {
    heights.sort_unstable();
    let total: usize = heights.iter().map(|&(_, chars)| chars).sum();
    let mut seen = 0;
    for &(height, chars) in heights.iter() {
        seen += chars;
        if 2 * seen > total {
            return height;
        }
    }
    0
}

/// Set `furniture` on every run of `page` whose line the rule reads as a header or a footer.
pub(crate) fn assign(page: &mut PageExtract) {
    let owned: BTreeSet<usize> = page
        .tables
        .iter()
        .flat_map(|t| t.cells.iter())
        .flat_map(|c| c.run_indices.iter().copied())
        .collect();
    let mut by_line: BTreeMap<(Option<u32>, i64), Vec<usize>> = BTreeMap::new();
    for (i, run) in page.runs.iter().enumerate() {
        if !owned.contains(&i) {
            by_line
                .entry((run.region, run.locator.origin_y))
                .or_default()
                .push(i);
        }
    }
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut pieces: Vec<Piece> = Vec::new();
    for line in by_line.into_values() {
        let rect = line
            .iter()
            .map(|&i| &page.runs[i])
            .filter(|run| !run.text.trim().is_empty())
            .filter_map(|run| run.geometry.measured())
            .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
            .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)));
        if let Some(rect) = rect {
            pieces.push(Piece { rect, text: true });
            lines.push(line);
        }
    }
    for table in &page.tables {
        let r = table.rect;
        pieces.push(Piece {
            rect: (r.x0, r.y0, r.x1, r.y1),
            text: false,
        });
    }
    let read = furniture(&pieces, page.height, body_line(&page.runs));
    for (line, furniture) in lines.iter().zip(read) {
        for &i in line {
            page.runs[i].furniture = furniture;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: Option<Furniture> = Some(Furniture::Header);
    const FOOTER: Option<Furniture> = Some(Furniture::Footer);

    /// A line of text from `top` to `bottom` points, across the page.
    fn line(top: i64, bottom: i64) -> Piece {
        Piece {
            rect: (7200, top * 100, 54000, bottom * 100),
            text: true,
        }
    }

    /// Body text from 100 to 880 points, lines 12 points apart: one band.
    fn body() -> Vec<Piece> {
        (0..66).map(|n| line(100 + 12 * n, 110 + 12 * n)).collect()
    }

    /// On a 1000-point page with a 10-point body line.
    fn read(pieces: &[Piece]) -> Vec<Option<Furniture>> {
        furniture(pieces, 100_000, 1000)
    }

    /// **A line set apart inside the top tenth is a header, and one inside the bottom tenth a
    /// footer**; the body between them is neither.
    #[test]
    fn lines_apart_at_the_top_and_bottom_tenths_are_header_and_footer() {
        let mut pieces = vec![line(40, 50)];
        pieces.extend(body());
        pieces.push(line(950, 960));
        let read = read(&pieces);
        assert_eq!(read[0], HEADER);
        assert_eq!(read[pieces.len() - 1], FOOTER);
        assert!(read[1..pieces.len() - 1].iter().all(Option::is_none));
    }

    /// **The outer tenth is where it is stated**: a header band ending exactly at a tenth of the
    /// page is one, a point lower is not; a footer band starting exactly nine tenths down is one, a
    /// point higher is not.
    #[test]
    fn the_band_must_lie_wholly_inside_the_outer_tenth() {
        for (top, bottom, want) in [(80, 100, HEADER), (81, 101, None)] {
            let mut pieces = vec![line(top, bottom)];
            pieces.extend((0..60).map(|n| line(200 + 12 * n, 210 + 12 * n)));
            assert_eq!(read(&pieces)[0], want, "a header ending at {bottom}");
        }
        for (top, want) in [(900, FOOTER), (899, None)] {
            let mut pieces = body();
            pieces.push(line(top, top + 10));
            assert_eq!(read(&pieces)[pieces.len() - 1], want, "a footer at {top}");
        }
    }

    /// **A line closer than half a body line to the text joins its band**, and a band reaching
    /// into the body is no header: 4 points under the line above joins, 5 opens a band.
    #[test]
    fn half_a_body_line_apart_opens_a_band_and_less_joins() {
        for (gap, want) in [(4, None), (5, HEADER)] {
            let mut pieces = vec![line(40, 50)];
            pieces.extend((0..60).map(|n| line(50 + gap + 12 * n, 60 + gap + 12 * n)));
            assert_eq!(read(&pieces)[0], want, "{gap} points apart");
        }
    }

    /// **A table in the band keeps it out**: a table set at the head of the page.
    #[test]
    fn a_band_holding_a_table_is_not_furniture() {
        let mut pieces = vec![
            line(40, 50),
            Piece {
                rect: (50000, 3000, 56000, 6000),
                text: false,
            },
        ];
        pieces.extend(body());
        assert_eq!(read(&pieces)[0], None);
    }

    /// **A page of one band has no furniture**: a short page set entirely in its top tenth, or a
    /// page whose text runs edge to edge.
    #[test]
    fn a_page_of_one_band_has_none() {
        assert_eq!(read(&[line(40, 50), line(52, 62)]), [None, None]);
    }

    /// **The body line is counted by character**: one long line of 10-point text outweighs three
    /// short 24-point headings.
    #[test]
    fn the_body_line_is_the_median_by_character() {
        let mut heights = [(2400, 8), (1000, 60), (2400, 9), (2400, 7)];
        assert_eq!(median_by_chars(&mut heights), 1000);
        let mut heights = [(2400, 30), (1000, 20)];
        assert_eq!(median_by_chars(&mut heights), 2400);
        assert_eq!(median_by_chars(&mut []), 0);
    }

    /// **Every line of the band is furniture**: a running head with its folio beside it, and a
    /// two-line foot.
    #[test]
    fn every_line_of_the_band_is_furniture() {
        let folio = Piece {
            rect: (54000, 4100, 56000, 5000),
            text: true,
        };
        let mut pieces = vec![line(40, 50), folio];
        pieces.extend(body());
        pieces.extend([line(950, 960), line(962, 972)]);
        let read = read(&pieces);
        assert_eq!(read[..2], [HEADER, HEADER]);
        assert_eq!(read[read.len() - 2..], [FOOTER, FOOTER]);
    }
}
