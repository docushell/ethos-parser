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

//! **Superscripts and subscripts from where a run sits beside its line** — part of
//! `ethos_parser_core::OBSERVATION_RULE_V3` (decision #40).
//!
//! A page's inked runs are grouped by baseline — within the leading-gap cut's own tolerance of a
//! group's first — and each group into **fragments** wherever the gap from one run's box to the
//! next is wider than an em. A fragment is set as a script when, beside a run `h` on another
//! baseline, it is:
//!
//! 1. **short** — [`MAX_SCRIPT_CHARS`] characters or fewer: a footnote mark, an ordinal's letters,
//!    an index, never a line of small type;
//! 2. **smaller** — its largest rendered em at most nine tenths of `h`'s;
//! 3. **off `h`'s baseline** by at least a tenth of `h`'s em and less than half of it;
//! 4. **beside `h`** — starting where `h` ends or ending where it starts, overlapping it by at
//!    most a fifth of `h`'s em and apart from it by at most half.
//!
//! Raised, it is a superscript; lowered, a subscript. A space between two runs of one script
//! carries it, so a span does not break at every word.
//!
//! **What it reads**: positions and rendered ems, which the document states, and nothing else. A
//! script is `Computed`; the text is the document's, and the projections only wrap it in `<sup>`
//! or `<sub>`. Measured on ParseBench's formatting pages before it shipped
//! (`docs/measurements/parsebench/README.md`).

use ethos_parser_core::Script;

use crate::nodes::TextRun;

/// The longest fragment, in characters, that can be a script.
pub const MAX_SCRIPT_CHARS: usize = 6;

/// One run as the rule reads it, in content order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Seen {
    /// Inked, with a measured box and a rendered em: the box's `x0` and `x1`, the baseline `y`,
    /// all top-left centipoints, and its characters.
    Placed {
        x0: i64,
        x1: i64,
        y: i64,
        em: i64,
        chars: usize,
    },
    /// Whitespace: it carries the script of the runs either side when they share one.
    Space,
    /// Inked, with no box or no em to read: never a script, and no host.
    Unread,
}

struct Placed {
    run: usize,
    x0: i64,
    x1: i64,
    y: i64,
    em: i64,
    chars: usize,
}

/// Each run's script, in the order given.
pub(crate) fn scripts(runs: &[Seen]) -> Vec<Option<Script>> {
    let mut placed: Vec<Placed> = runs
        .iter()
        .enumerate()
        .filter_map(|(run, seen)| match *seen {
            Seen::Placed {
                x0,
                x1,
                y,
                em,
                chars,
            } if em > 0 => Some(Placed {
                run,
                x0,
                x1,
                y,
                em,
                chars,
            }),
            _ => None,
        })
        .collect();
    placed.sort_by_key(|p| (p.y, p.x0, p.run));

    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (k, p) in placed.iter().enumerate() {
        match groups.last_mut() {
            Some(g) if (p.y - placed[g[0]].y).abs() <= crate::blocks::LINE_TOLERANCE => g.push(k),
            _ => groups.push(vec![k]),
        }
    }
    let mut fragments: Vec<(usize, Vec<usize>)> = Vec::new();
    for (gi, group) in groups.iter().enumerate() {
        let mut members = group.clone();
        members.sort_by_key(|&k| (placed[k].x0, placed[k].run));
        let mut fragment: Vec<usize> = Vec::new();
        let (mut end, mut em) = (i64::MIN, 0);
        for k in members {
            let p = &placed[k];
            if !fragment.is_empty() && p.x0 - end > em.max(p.em) {
                fragments.push((gi, std::mem::take(&mut fragment)));
                (end, em) = (i64::MIN, 0);
            }
            end = end.max(p.x1);
            em = em.max(p.em);
            fragment.push(k);
        }
        if !fragment.is_empty() {
            fragments.push((gi, fragment));
        }
    }

    let mut out: Vec<Option<Script>> = vec![None; runs.len()];
    let reach = placed.iter().map(|p| p.em).max().unwrap_or(0) / 2;
    let group_y: Vec<i64> = groups.iter().map(|g| placed[g[0]].y).collect();
    for (gi, fragment) in &fragments {
        let chars: usize = fragment.iter().map(|&k| placed[k].chars).sum();
        if chars == 0 || chars > MAX_SCRIPT_CHARS {
            continue;
        }
        let em = fragment.iter().map(|&k| placed[k].em).max().unwrap_or(0);
        let x0 = fragment.iter().map(|&k| placed[k].x0).min().unwrap_or(0);
        let x1 = fragment.iter().map(|&k| placed[k].x1).max().unwrap_or(0);
        let y = placed[fragment[0]].y;
        let near = group_y.partition_point(|&gy| gy < y - reach)
            ..group_y.partition_point(|&gy| gy <= y + reach);
        let offset = near
            .filter(|gj| gj != gi)
            .flat_map(|gj| groups[gj].iter())
            .map(|&h| &placed[h])
            .find(|h| {
                let off = (h.y - y).abs();
                let tol = h.em / 5;
                10 * em <= 9 * h.em
                    && 10 * off >= h.em
                    && 2 * off < h.em
                    && ((x0 >= h.x1 - tol && x0 <= h.x1 + h.em / 2)
                        || (x1 <= h.x0 + tol && x1 >= h.x0 - h.em / 2))
            })
            .map(|h| h.y - y);
        if let Some(offset) = offset {
            let script = if offset > 0 {
                Script::Superscript
            } else {
                Script::Subscript
            };
            for &k in fragment {
                out[placed[k].run] = Some(script);
            }
        }
    }

    // A space between two runs of one script carries it.
    let mut before: Option<usize> = None;
    let mut previous_inked: Vec<Option<usize>> = vec![None; runs.len()];
    for (i, seen) in runs.iter().enumerate() {
        previous_inked[i] = before;
        if *seen != Seen::Space {
            before = Some(i);
        }
    }
    let mut after: Option<usize> = None;
    for i in (0..runs.len()).rev() {
        if runs[i] == Seen::Space {
            if let (Some(p), Some(n)) = (previous_inked[i], after) {
                if out[p].is_some() && out[p] == out[n] {
                    out[i] = out[p];
                }
            }
        } else {
            after = Some(i);
        }
    }
    out
}

/// Set `script` on every run of one page `scripts` reads as a superscript or a subscript. `runs`
/// is the page in content order and `ems` each run's rendered em.
pub(crate) fn assign(runs: &mut [TextRun], ems: &[Option<i64>]) {
    let seen: Vec<Seen> = runs
        .iter()
        .enumerate()
        .map(|(i, run)| {
            let text = run.text.trim();
            if text.is_empty() {
                return Seen::Space;
            }
            match (run.geometry.measured(), ems.get(i).copied().flatten()) {
                (Some(b), Some(em)) => Seen::Placed {
                    x0: b.x0(),
                    x1: b.x1(),
                    y: run.locator.origin_y,
                    em,
                    chars: text.chars().count(),
                },
                _ => Seen::Unread,
            }
        })
        .collect();
    for (run, script) in runs.iter_mut().zip(scripts(&seen)) {
        run.script = script;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A run `w` points wide at `x` points on the baseline `y` points, set at `em` points.
    fn at(x: i64, y: i64, w: i64, em: i64, text: &str) -> Seen {
        Seen::Placed {
            x0: x * 100,
            x1: (x + w) * 100,
            y: y * 100,
            em: em * 100,
            chars: text.chars().count(),
        }
    }

    /// **A footnote mark set smaller and raised after its word is a superscript**, and the word
    /// and the text after it are not.
    #[test]
    fn a_raised_smaller_mark_after_its_word_is_a_superscript() {
        let runs = [
            at(100, 200, 40, 10, "colors."),
            at(140, 196, 6, 6, "49"),
            Seen::Space,
            at(150, 200, 30, 10, "This"),
        ];
        assert_eq!(
            scripts(&runs),
            [None, Some(Script::Superscript), None, None]
        );
    }

    /// **Lowered, it is a subscript** — the 2 of H₂O.
    #[test]
    fn a_lowered_smaller_index_is_a_subscript() {
        let runs = [
            at(100, 200, 7, 10, "H"),
            at(107, 202, 4, 6, "2"),
            at(111, 200, 7, 10, "O"),
        ];
        assert_eq!(scripts(&runs), [None, Some(Script::Subscript), None]);
    }

    /// **Each clause refuses what it is there for**: the same size, raised as far as half a line,
    /// standing under the word rather than beside it, or too long to be a mark.
    #[test]
    fn same_size_far_apart_or_long_text_is_no_script() {
        let word = at(100, 200, 40, 10, "colors.");
        for (mark, why) in [
            (at(140, 196, 6, 10, "49"), "the same size"),
            (at(140, 195, 6, 6, "49"), "half an em off the baseline"),
            (at(110, 196, 6, 6, "49"), "over the word, not beside it"),
            (at(140, 196, 30, 6, "1234567"), "seven characters"),
            (at(140, 200, 6, 6, "49"), "on the word's own baseline"),
        ] {
            assert_eq!(scripts(&[word, mark]), [None, None], "{why}");
        }
        // Off a 20pt word's baseline by 1.7pt: past the line tolerance, short of a tenth of its em.
        let word = Seen::Placed {
            x0: 10000,
            x1: 14000,
            y: 20000,
            em: 2000,
            chars: 7,
        };
        let mark = Seen::Placed {
            x0: 14000,
            x1: 14600,
            y: 19830,
            em: 1200,
            chars: 2,
        };
        assert_eq!(scripts(&[word, mark]), [None, None], "a tenth of an em");
    }

    /// **A space between two runs of one script carries it**; a space beside one does not.
    #[test]
    fn a_space_inside_a_script_carries_it() {
        let runs = [
            at(100, 200, 40, 10, "Authors"),
            at(140, 196, 3, 6, "1"),
            Seen::Space,
            at(146, 196, 3, 6, "2"),
            Seen::Space,
            at(155, 200, 20, 10, "and"),
        ];
        let sup = Some(Script::Superscript);
        assert_eq!(scripts(&runs), [None, sup, sup, sup, None, None]);
    }

    /// A run with no box or no em is no script and no host.
    #[test]
    fn an_unread_run_is_neither_script_nor_host() {
        assert_eq!(
            scripts(&[Seen::Unread, at(140, 196, 6, 6, "49")]),
            [None, None]
        );
    }
}
