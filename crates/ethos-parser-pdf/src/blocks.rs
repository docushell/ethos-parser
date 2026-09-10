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

//! Block subdivision: where a band's own leading says one run of text ended and the next began.
//!
//! # What this is, in one sentence
//!
//! A **`Computed` index and nothing else**. It says two runs are in different blocks; it never
//! says either is a paragraph, a heading, a list item or a section.
//!
//! # Why it is not [`crate::reading_order`]'s horizontal cut
//!
//! That recursion already subdivides horizontally, and `reading_order`'s `Regions` doc says
//! exactly why its leaves are not numbered: `horizontal_cut` cuts at *the widest gap and every gap
//! tied with it*, and body text set with uniform leading has **every** baseline gap tied. So
//! numbering those leaves would put one index on every line — *a line number wearing a block's
//! name.*
//!
//! This rule replaces the criterion rather than the machinery. A gap is a boundary when it clears
//! **[`CUT_NUM`]/[`CUT_DEN`] × the band's own modal leading** — an absolute test against a
//! statistic of the band, not a relative test against its widest gap. On uniform body text no gap
//! clears it and no block opens; at a paragraph break the gap does clear it and exactly one opens.
//! That is the whole difference, and it is why the leaves can be numbered now.
//!
//! # The measurement this rule is, not a rule it was tuned to
//!
//! [`docs/19-BLOCK-SUBDIVISION-SCOPE.md`] §11.2, re-derived at 0.54.0 by
//! `docs/measurements/block-subdivision/probe3b.py`: over **135 real paragraph-to-paragraph
//! boundaries** and **719 mid-paragraph pairs** on `nist-sp-800-207` — the one gate document §9.1
//! proved can carry a real P→P label —
//!
//! | rule | recall | fires mid-paragraph | precision |
//! | --- | --- | --- | --- |
//! | fixed 1.15× | 68.9% | 0.0% | 100.0% |
//! | **fixed 1.60×** | **63.7%** | **0.0%** | **100.0%** |
//! | adaptive second mode | 51.1% | 0.0% | 100.0% |
//!
//! **1.60× ships, and 1.15× is measurably better on that document.** §11.4 forbids acting on it:
//! one document cannot carry a threshold, and Test A puts fixed-1.6×'s per-document recall between
//! 32.3% and 97.1%, so a constant fitted to the only labellable document is fitted to a sample of
//! one. What changed at 0.54.0 is that 1.15×'s published 1.1% false-fire went to zero, so the
//! margin that chose 1.60× is gone — see §11.2b. **A second labellable document would now decide a
//! live question**, which is a better reason to acquire one than confirming a settled answer.
//!
//! **Adaptive is refused on the same evidence**, not on principle: it loses by 12.6 points here and
//! wins on five of six documents under proxy labels, and §11.3 explains that the one document able
//! to validate it is the one it hurts. That contradiction is unresolved and the shipped rule is the
//! one measured against real labels.
//!
//! # What is deliberately absent
//!
//! **No confidence, no near-miss score.** A band either clears the guard and gets cut or it does
//! not. `docs/01-CONTRACT.md` §9.
//!
//! **No indent branch.** §4.3 measured that 43% of a scholarly corpus marks paragraphs by a 1–2 em
//! indent with no extra leading, and this rule is blind to those by construction. That is a
//! *declared* blindness: the affected band simply gets no cut, which is the absent-means-declined
//! state below, and never a guessed one. Building the branch needs a corpus this repository does
//! not own — §4.3's own conclusion.
//!
//! **No float.** `gap ≥ 1.6 × leading` is `5 · gap ≥ 8 · leading`.

use crate::reading_order::RunGeometry;

/// How close two baselines must be, in centipoints, to be one line.
///
/// 150 — one and a half points. The same constant
/// `docs/measurements/block-subdivision/labelled.py` used as `SAME_LINE_TOL` to build the labels
/// this rule is measured against, so the rule and its measurement group lines identically. A
/// superscript or a subscript within a line sits inside it; two set lines never do.
pub const LINE_TOLERANCE: i64 = 150;

/// Gap widths are binned to this, in centipoints, before the mode is taken.
///
/// 10 — a tenth of a point. Without binning, "modal leading" is meaningless: a page's baselines
/// are quantized to centipoints and a run of body text produces gaps of 1329, 1330 and 1328 that
/// are one leading and three modes. §5 measured the sensitivity — the result moves by at most one
/// percentage point for any bin from 1 to 50 centipoints — so this is not a fitted constant.
pub const BIN: i64 = 10;

/// The smallest modal gap, in centipoints, that may be treated as a leading.
///
/// 600 — six points. Half of §5's plausibility guard, and it exists because
/// `(page, region)` bands are not all text flows: 9% of them on the gate documents interleave
/// table columns, and five `nist-sp-800-218` pages report a modal "leading" of 200 centipoints,
/// which is a four-column interleave pitch rather than a line spacing. A band whose mode is under
/// this is declined rather than cut against a fiction.
pub const MIN_MODAL_LEADING: i64 = 600;

/// The modal bin must hold at least this fraction of a band's gaps: 1/[`MIN_SHARE_DEN`].
///
/// A quarter. The other half of §5's guard. A band whose most common gap is not actually common
/// has no leading to speak of — its gaps are a scatter, and the mode of a scatter is an artifact
/// of the binning rather than a property of the text.
pub const MIN_SHARE_DEN: i64 = 4;

/// Numerator of the cut threshold: a gap is a boundary at [`CUT_NUM`]/[`CUT_DEN`] × leading.
pub const CUT_NUM: i64 = 8;
/// Denominator of the cut threshold. 8/5 is 1.6.
pub const CUT_DEN: i64 = 5;

/// The 1-based block of each run, in reading order, or empty when nothing was subdivided.
///
/// **Empty rather than a vector of `None`**, on exactly the contract
/// [`crate::reading_order::Arrangement::regions`] carries and for the same reason: a caller `zip`s
/// this and does no work at all on a page the rule declined, with no allocation taken to say so.
/// Otherwise it is `runs.len()` long and every entry is `Some`.
///
/// `regions` is [`crate::reading_order::Arrangement::regions`] — empty when the vertical cut made
/// no division, in which case the whole page is one band.
///
/// # A block is bounded by both axes
///
/// A vertical cut ends a block as surely as a wide gap does: two columns of prose are never one
/// block, whatever their leading. So the identity is `(band, subdivisions before me in that
/// band)`, and the ordinal advances whenever that identity changes along the finished order — the
/// same walk [`crate::reading_order`] does for regions, over a finer key.
pub fn subdivide(
    runs: &[RunGeometry],
    order: &[usize],
    regions: &[Option<u32>],
) -> Vec<Option<u32>> {
    if runs.is_empty() {
        return Vec::new();
    }

    // Band per run. An empty `regions` means the vertical cut divided nothing, so one band.
    let band_of = |i: usize| -> u32 {
        if regions.is_empty() {
            0
        } else {
            regions[i].unwrap_or(0)
        }
    };

    // Distinct line baselines per band, and the runs that sit on each.
    let mut bands: std::collections::BTreeMap<u32, Vec<i64>> = std::collections::BTreeMap::new();
    for (i, r) in runs.iter().enumerate() {
        bands.entry(band_of(i)).or_default().push(r.y);
    }
    let mut lines_of_band: std::collections::BTreeMap<u32, Vec<i64>> =
        std::collections::BTreeMap::new();
    for (band, mut ys) in bands {
        ys.sort_unstable();
        let mut lines: Vec<i64> = Vec::new();
        for y in ys {
            match lines.last() {
                Some(&last) if y - last <= LINE_TOLERANCE => {}
                _ => lines.push(y),
            }
        }
        lines_of_band.insert(band, lines);
    }

    // Per band: which line indices start a new subdivision.
    let mut cuts_of_band: std::collections::BTreeMap<u32, Vec<bool>> =
        std::collections::BTreeMap::new();
    for (band, lines) in &lines_of_band {
        let mut starts = vec![false; lines.len()];
        if let Some(leading) = modal_leading(lines) {
            for i in 1..lines.len() {
                let gap = lines[i] - lines[i - 1];
                if CUT_DEN * gap >= CUT_NUM * leading {
                    starts[i] = true;
                }
            }
        }
        cuts_of_band.insert(*band, starts);
    }

    // Subdivision index per run: how many cuts precede its line within its band.
    let mut subdivision = vec![0u32; runs.len()];
    for (i, r) in runs.iter().enumerate() {
        let band = band_of(i);
        let lines = &lines_of_band[&band];
        let starts = &cuts_of_band[&band];
        // The line this run sits on: the last line at or below its baseline.
        let li = match lines.binary_search(&r.y) {
            Ok(k) => k,
            Err(0) => 0,
            Err(k) => k - 1,
        };
        subdivision[i] = starts[..=li].iter().filter(|s| **s).count() as u32;
    }

    // Number the (band, subdivision) pairs by FIRST APPEARANCE along the finished order.
    //
    // **Not "advance on every change", which is what `reading_order` does for regions.** That
    // walk is correct there because the arrangement guarantees each band is visited contiguously
    // — it built the order by band. This rule's key is finer than the order's own grouping, so
    // nothing guarantees the same: a run that revisits an earlier key would take a second ordinal
    // under that walk, and one block would arrive as two. First appearance cannot do that, and
    // costs one map.
    //
    // In practice reading order does visit a block contiguously, so the two agree on every real
    // page measured. This is the cheaper assumption to drop, not a defect being worked around.
    let mut assigned: std::collections::BTreeMap<(u32, u32), u32> =
        std::collections::BTreeMap::new();
    let mut out = vec![None; runs.len()];
    let mut block = 0u32;
    for &r in order {
        let key = (band_of(r), subdivision[r]);
        let n = *assigned.entry(key).or_insert_with(|| {
            block += 1;
            block
        });
        out[r] = Some(n);
    }

    // Fewer than two blocks is no subdivision, and says so by being empty — the region contract.
    if block < 2 {
        return Vec::new();
    }
    out
}

/// The band's modal binned gap, or `None` when the band is not a text flow.
///
/// Returns `None` rather than a fallback: §5's guard is the difference between a leading and a
/// table-column interleave pitch, and a band that fails it has no leading this rule may use.
fn modal_leading(lines: &[i64]) -> Option<i64> {
    if lines.len() < 2 {
        return None;
    }
    let gaps: Vec<i64> = lines.windows(2).map(|w| w[1] - w[0]).collect();
    let mut counts: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();
    for g in &gaps {
        // Round half up, deterministically and in integers. `probe3b.py` bins the same way.
        *counts.entry((g + BIN / 2) / BIN * BIN).or_insert(0) += 1;
    }
    // `max_by_key` on a BTreeMap iterator takes the LAST maximum; take the smallest bin among
    // ties instead, so the mode of a tie is stable and does not depend on iteration direction.
    let best = counts.values().copied().max()?;
    let leading = counts
        .iter()
        .filter(|(_, c)| **c == best)
        .map(|(b, _)| *b)
        .next()?;
    if leading < MIN_MODAL_LEADING {
        return None;
    }
    if MIN_SHARE_DEN * best < gaps.len() as i64 {
        return None;
    }
    Some(leading)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(x: i64, y: i64) -> RunGeometry {
        RunGeometry {
            x,
            y,
            advance: Some(500),
        }
    }

    /// Stream order, for a page the vertical cut never divided.
    fn straight(n: usize) -> (Vec<usize>, Vec<Option<u32>>) {
        ((0..n).collect(), Vec::new())
    }

    /// **The property `reading_order`'s horizontal cut could not have.**
    ///
    /// Ten lines of body text at one leading. `horizontal_cut` would tie every gap and cut at all
    /// of them; this must cut at none, because nothing in the page says a block ended.
    #[test]
    fn uniform_leading_is_one_block_and_therefore_no_block() {
        let runs: Vec<RunGeometry> = (0..10).map(|i| run(7200, 7200 + i * 1329)).collect();
        let (order, regions) = straight(runs.len());
        assert!(
            subdivide(&runs, &order, &regions).is_empty(),
            "uniform leading must produce no subdivision at all"
        );
    }

    #[test]
    fn a_gap_at_the_threshold_opens_a_block_and_one_below_it_does_not() {
        // Leading 1000; the modal bin must dominate, so give it six ordinary gaps either side.
        let mut ys: Vec<i64> = (0..6).map(|i| 7200 + i * 1000).collect();
        let last = *ys.last().unwrap();
        // 8/5 * 1000 = 1600 exactly: the boundary is inclusive.
        ys.push(last + 1600);
        for i in 1..6 {
            ys.push(last + 1600 + i * 1000);
        }
        let runs: Vec<RunGeometry> = ys.iter().map(|y| run(7200, *y)).collect();
        let (order, regions) = straight(runs.len());
        let out = subdivide(&runs, &order, &regions);
        assert!(!out.is_empty(), "a 1.6x gap must open a block");
        assert_eq!(out[5], Some(1), "the line before the gap is in block 1");
        assert_eq!(out[6], Some(2), "the line after it is in block 2");

        // One centipoint under the threshold is not a boundary.
        let mut ys2: Vec<i64> = (0..6).map(|i| 7200 + i * 1000).collect();
        let last2 = *ys2.last().unwrap();
        ys2.push(last2 + 1599);
        for i in 1..6 {
            ys2.push(last2 + 1599 + i * 1000);
        }
        let runs2: Vec<RunGeometry> = ys2.iter().map(|y| run(7200, *y)).collect();
        let (order2, regions2) = straight(runs2.len());
        assert!(
            subdivide(&runs2, &order2, &regions2).is_empty(),
            "1599 against a 1000 leading is under 1.6x and must not cut"
        );
    }

    /// §5's guard, first half: a modal gap under six points is not a leading.
    #[test]
    fn a_band_whose_mode_is_an_interleave_pitch_is_declined() {
        // Modal gap 200 centipoints — the four-column interleave pitch §5 measured on
        // `nist-sp-800-218`, not a line spacing. Even with a huge gap present, no cut.
        let mut ys: Vec<i64> = (0..10).map(|i| 7200 + i * 200).collect();
        ys.push(7200 + 10 * 200 + 9000);
        let runs: Vec<RunGeometry> = ys.iter().map(|y| run(7200, *y)).collect();
        let (order, regions) = straight(runs.len());
        assert!(
            subdivide(&runs, &order, &regions).is_empty(),
            "a sub-600 mode has no leading this rule may use"
        );
    }

    /// §5's guard, second half: the mode of a scatter is an artifact of the binning.
    #[test]
    fn a_band_whose_gaps_are_a_scatter_is_declined() {
        // Every gap distinct and none repeated: the modal bin holds 1 of 9, under a quarter.
        let mut y = 7200;
        let mut ys = vec![y];
        for step in [700, 1500, 2300, 3100, 3900, 4700, 5500, 6300, 7100] {
            y += step;
            ys.push(y);
        }
        let runs: Vec<RunGeometry> = ys.iter().map(|y| run(7200, *y)).collect();
        let (order, regions) = straight(runs.len());
        assert!(
            subdivide(&runs, &order, &regions).is_empty(),
            "no bin holds a quarter of the gaps, so there is no leading"
        );
    }

    /// A block is bounded by the vertical cut too: two columns are never one block.
    #[test]
    fn two_columns_at_one_leading_are_two_blocks() {
        // Six lines in each of two regions, identical leading, no wide gap anywhere.
        let mut runs = Vec::new();
        for i in 0..6 {
            runs.push(run(7200, 7200 + i * 1329));
        }
        for i in 0..6 {
            runs.push(run(30000, 7200 + i * 1329));
        }
        let order: Vec<usize> = (0..runs.len()).collect();
        let regions: Vec<Option<u32>> = (0..6)
            .map(|_| Some(1))
            .chain((0..6).map(|_| Some(2)))
            .collect();
        let out = subdivide(&runs, &order, &regions);
        assert!(!out.is_empty(), "two columns must be two blocks");
        assert_eq!(out[0], Some(1));
        assert_eq!(out[6], Some(2), "the second column opens a second block");
    }

    /// The region contract: fewer than two blocks is empty, not a vector of `Some(1)`.
    #[test]
    fn a_single_block_reports_nothing_rather_than_one() {
        let runs: Vec<RunGeometry> = (0..8).map(|i| run(7200, 7200 + i * 1329)).collect();
        let (order, regions) = straight(runs.len());
        let out = subdivide(&runs, &order, &regions);
        assert!(
            out.is_empty(),
            "one block is no subdivision, and says so by being empty"
        );
    }

    /// Runs sharing a baseline within the tolerance are one line, so they cannot straddle a block.
    #[test]
    fn runs_on_one_line_share_a_block() {
        let mut ys: Vec<i64> = Vec::new();
        for i in 0..6 {
            ys.push(7200 + i * 1000);
        }
        let last = *ys.last().unwrap();
        ys.push(last + 1600);
        for i in 1..6 {
            ys.push(last + 1600 + i * 1000);
        }
        let mut runs: Vec<RunGeometry> = ys.iter().map(|y| run(7200, *y)).collect();
        // A second run on the last line of block 1, 100 centipoints off its baseline.
        runs.push(run(20000, ys[5] + 100));
        let order: Vec<usize> = (0..runs.len()).collect();
        let out = subdivide(&runs, &order, &Vec::new());
        assert!(!out.is_empty());
        assert_eq!(
            out[runs.len() - 1],
            out[5],
            "a run within LINE_TOLERANCE of another sits on its line and in its block"
        );
    }

    #[test]
    fn an_empty_page_subdivides_into_nothing() {
        assert!(subdivide(&[], &[], &[]).is_empty());
    }
}
