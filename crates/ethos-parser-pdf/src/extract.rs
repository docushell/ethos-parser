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

//! Extraction: the evidence itself (`docs/history/05-MILESTONES.md` M3).
//!
//! Takes the **same [`Document`] handle** [`crate::classify`] takes. Nothing here reopens a file:
//! two loads can disagree, and a classifier that saw a different object graph from the extractor
//! is a silent divergence with no diagnostic.
//!
//! # Coordinates
//!
//! PDF user space has its origin at the bottom-left with y increasing upward. The artifact
//! declares top-left with y increasing downward ([`ethos_parser_core::CoordinateSystem::V0`]), and page
//! `/Rotate` is applied so geometry matches the declaration rather than the raw stream. The
//! transform is this engine's job and the declaration is how a reader knows it happened.

use ethos_parser_core::{
    quantize, ArtifactIdentity, Assurance, DerivationClass, EngineError, IdAllocator, IdKind,
    PageState, PageStateEntry, Profile, Sha256Hex, TextFinding, QUANTUM_PER_POINT,
};
use serde::{Deserialize, Serialize};

use crate::classify::SourceRef;
use crate::content::Interpreter;
use crate::document::Document;
use crate::fonts::{load_page_fonts, WidthSource};
use crate::limitations as lim;
use crate::nodes::{PageExtract, PdfLocator, SynthesisReason, SynthesizedChar, TextRun};

/// Artifact type for an extract. **DRAFT** — see `docs/draft-schemas/`.
pub const EXTRACT_ARTIFACT_TYPE: &str = "ethos.parser.extract.v0";

/// Shape version of the extract artifact. **DRAFT**.
///
/// `0.4.0` at v2-S24: [`crate::nodes::PageExtract`] gained `tagged_tables`, the tables the
/// structure tree declares that no geometric detector matched. The field is omitted when empty, so
/// a document with no tagged tables serializes byte-identically to a `0.3.0` extract — but the
/// shape is `deny_unknown_fields`, so a `0.3.0` reader rejects an extract that carries the key, and
/// the version says the two are genuinely non-comparable rather than letting a reader guess.
///
/// The rule this constant follows: it moves when a reader could otherwise be misled about what it
/// is reading. A key added under a MINOR release that this parser refuses on read is named in the
/// release note instead and the version stays — the rule `docs/23-AUTO-TAGGING-SCOPE.md` §8 holds
/// the representation's `schema_version` to, applied to this wire as well. Three additions since
/// `0.4.0` follow it: `TextRun.block` at 0.55.0 (an `Option`, so a 0.54.0 extract still parses
/// here); `derivation` at auto-tagging S1, required with no default, on every `pdf_tagged` locator
/// a run carries and on `TaggedTableRecord`; and `TextRun.inferred_heading` at decision #29, absent
/// where false. So a 0.58.0 extract of any tagged PDF is refused here (`missing field
/// derivation`), and a 0.58.0 build, whose `TextRun` denies unknown fields, refuses this build's
/// extract of any tagged PDF (`unknown field derivation`) and of any PDF where a heading was
/// inferred (`unknown field inferred_heading`). That 0.58.0's `TaggedTableRecord` did not deny
/// unknown fields never comes into play: a tagged table's cells are tagged runs, refused first.
/// All three are named in their release notes. The artifact is a draft library surface with no
/// stored fixtures: only this build's own bytes are ever parsed back, in two round-trip tests and,
/// edited, in the one that holds the reader to the two shapes it reads — this one and `0.4.0`.
pub const EXTRACT_SCHEMA_VERSION: &str = "0.5.0";

/// The extract artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractArtifact {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    #[serde(deserialize_with = "pinned_identity")]
    pub identity: ArtifactIdentity,
    /// The bytes this extract describes.
    pub source: SourceRef,
    /// Version id of the reading-order rule that ordered the runs.
    ///
    /// Mirrors `profile.reading_order_rule`, on the artifact so a reader need not fetch the
    /// profile to know which rule produced this order.
    pub reading_order_rule: String,
    /// Total pages.
    pub page_count: u32,
    /// Per-page results for the **processed** pages, in page order.
    ///
    /// A page missing from this list was not read; [`Assurance::page_states`] says which of the
    /// reasons applied. Its absence is never evidence that the page holds no text.
    pub pages: Vec<PageExtract>,
    /// The outline the catalog declares, in `/First`/`/Next` order (`outlines-v3`).
    ///
    /// **Document-level, unlike `tables`, which ride their page.** An outline is one tree over
    /// the whole catalog and an entry names a page rather than belonging to one.
    ///
    /// Optional on the wire so an extract written before this slice still reads. An empty array
    /// means the reader looked and the catalog named none; `capabilities.outlines` is what says
    /// whether it looked at all.
    #[serde(default)]
    pub outlines: Vec<ethos_parser_core::OutlineRecord>,
    /// Declared capabilities, limitations, per-page state, coverage, and terminal state.
    ///
    /// **The L1 gate** (`docs/01-CONTRACT.md` §7). Absorbs what M3 emitted as `not_decoded`:
    /// absent font widths and undescended form XObjects are now limitations in
    /// `assurance.limitations`, alongside the capability-derived ones.
    ///
    /// This sentence ended *"— including the explicit multi-column reading-order limitation that
    /// `synthetic/two-columns` exists to pin"* until v2-S13.5, and stopped being true at
    /// **v1-S5**. That slice shipped the reading-order rule, so `multi-column-reading-order` — the
    /// code that said a multi-column document is read in the WRONG ORDER — left the default
    /// profile's arm entirely rather than being reworded; `READING_ORDER_GEOMETRIC_ONLY` is the
    /// narrower claim that replaced it. The old code still fires, but only for a profile that
    /// turns the capability OFF, which is a different and still-true statement.
    pub assurance: Assurance,
}

/// [`ExtractArtifact::identity`], read only as this module's type at this module's version — or at
/// `0.4.0`, the shape before the outline slice, which [`ExtractArtifact::outlines`] defaults so
/// that it still reads.
fn pinned_identity<'de, D: serde::Deserializer<'de>>(d: D) -> Result<ArtifactIdentity, D::Error> {
    ArtifactIdentity::deserialize_pinned(
        d,
        EXTRACT_ARTIFACT_TYPE,
        &["0.4.0", EXTRACT_SCHEMA_VERSION],
    )
}

impl ExtractArtifact {
    /// Canonical bytes, via `ethos-parser-core`'s c14n.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact cannot be canonicalized — unreachable through
    /// the public API, since every field is an integer, string, bool, or enum.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        ethos_parser_core::c14n::canonical_bytes_of(self).map_err(|e| EngineError::Malformed {
            what: "extract".into(),
            detail: e.to_string(),
        })
    }

    /// Every run across every page, in reading order.
    pub fn runs(&self) -> impl Iterator<Item = &TextRun> {
        self.pages.iter().flat_map(|p| p.runs.iter())
    }

    /// Whether this artifact may be presented as a complete reading of its source.
    ///
    /// The question a consumer must ask before treating the run list as the whole document.
    /// False whenever any authorized page did not reach `Processed`.
    pub fn is_complete(&self) -> bool {
        self.assurance.is_complete()
    }
}

/// Fold `more` things this reader saw and did not put in the record into `count`.
///
/// **Saturating, because a wrapped erasure count is a silent drop presented as a success.** These
/// are document-level accumulators over a page count bounded by nothing but the file, so a plain
/// `+=` can pass `u32::MAX` and come back small — an artifact reporting that it erased almost
/// nothing while it erased four billion things. A14's whole content is that the number is honest.
///
/// The width stays `u32`: a saturated count is honest at the ceiling, and widening would move the
/// ceiling rather than remove it. v2-S9's adversarial review reproduced this shape as an 85×
/// under-declaration in `ethos-parser-office`; v2-S9.1 repairs the two accumulators here that the other
/// six in this function already had right.
fn declare(count: u32, more: u32) -> u32 {
    count.saturating_add(more)
}

/// What `fonts` lack, declared once each: a composite font counted, absent widths, an assumed
/// encoding. A font whose encoding this profile does not read refuses the page where it is drawn
/// with (tracker I8), so until then it is described by nothing here.
fn describe_fonts<'f>(
    fonts: impl Iterator<Item = &'f std::sync::Arc<crate::fonts::Font>>,
    composite_fonts: &mut u32,
    font_limitations: &mut Vec<ethos_parser_core::Limitation>,
) {
    for font in fonts.filter(|f| f.refusal().is_none()) {
        if font.kind == crate::fonts::FontKind::Composite {
            *composite_fonts = declare(*composite_fonts, 1);
        }
        let mut declare_once = |entry: ethos_parser_core::Limitation| {
            if !font_limitations.contains(&entry) {
                font_limitations.push(entry);
            }
        };
        if let WidthSource::Absent { reason } = &font.widths {
            declare_once(lim::font_widths_absent(reason));
        }
        // v2.2-S6. `StandardEncoding` was applied to a font the specification does not give it
        // to. The characters still travel; the artifact now says they may be wrong.
        if let Some(detail) = &font.builtin_encoding_assumed {
            declare_once(lim::symbolic_font_builtin_encoding_assumed(detail));
        }
    }
}

/// The same ceiling for a count that arrives as a `usize`.
///
/// An `as u32` cast on an input-driven count is worse than a plain `+=`, because it wraps in debug
/// **and** release — so no test and no CI job can catch it, where `+=` at least panics in debug.
fn declared_len(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Decision #161: a page that showed text and placed a scan — [`stray_text`] — keeps its drawn
/// runs, and a recognized word is written only where its box lies on the page and shares no area
/// with a drawn run's box: the test the seal applies (`check_recognized_placement`), so nothing
/// written here is refused there. A word with no box, or beside a drawn run with none, is left
/// out, as nothing shows the two apart; a drawn run that drew no ink, a run of spaces, covers
/// nothing. [`clear_of_drawn`] is the test; returns the words kept and how many were left out.
fn beside_drawn(
    words: Vec<crate::recognize::PageWord>,
    clear: &dyn Fn(Option<ethos_parser_core::QRect>) -> bool,
) -> (Vec<crate::recognize::PageWord>, u32) {
    let read = words.len();
    let kept: Vec<_> = words.into_iter().filter(|w| clear(w.rect)).collect();
    let under = declared_len(read - kept.len());
    (kept, under)
}

/// Decision #161: whether a recognized word with box `rect` may be written on `page` — every word
/// on a page that drew no runs; beside drawn runs, one whose box lies on the page and shares no
/// area with a drawn run's box ([`beside_drawn`]).
fn clear_of_drawn(page: &PageExtract) -> impl Fn(Option<ethos_parser_core::QRect>) -> bool + '_ {
    let drawn: Vec<Option<ethos_parser_core::QRect>> = page
        .runs
        .iter()
        .filter_map(|r| drawn_area(r.geometry))
        .collect();
    move |rect| {
        page.runs.is_empty()
            || rect.is_some_and(|r| {
                r.x0() >= 0
                    && r.y0() >= 0
                    && r.x1() <= page.width
                    && r.y1() <= page.height
                    && drawn.iter().all(|d| d.is_some_and(|d| !d.overlaps(r)))
            })
    }
}

/// Decision #161: the area a drawn run covers, as the seal reads it — its box; `Some(None)`,
/// unknown, where it has none; and `None`, nothing, where it drew no ink, as a run of spaces.
fn drawn_area(g: ethos_parser_core::GeometryPresence) -> Option<Option<ethos_parser_core::QRect>> {
    match g {
        ethos_parser_core::GeometryPresence::Measured(r) => Some(Some(r)),
        ethos_parser_core::GeometryPresence::Absent(
            ethos_parser_core::GeometryAbsence::NoInkToMeasure,
        ) => None,
        ethos_parser_core::GeometryPresence::Absent(_) => Some(None),
    }
}

/// Decision #161: a page that showed text, all of it under
/// [`STRAY_TEXT_MAX_CHARS`](crate::thresholds::STRAY_TEXT_MAX_CHARS) characters and every run of
/// it boxed but what drew no ink, over images that paint at least
/// [`STRAY_TEXT_MIN_IMAGE_PERCENT`](crate::thresholds::STRAY_TEXT_MIN_IMAGE_PERCENT) of its
/// visible box together: a scan carrying a stray text layer, which `--ocr`'s trigger takes as it
/// takes a page that showed none. A reason code like the rest of the trigger, never a score.
/// `images` are the parts of the page's upright images its clips show; what two of them paint
/// over the same place counts once (PR 58's review).
fn stray_text(
    runs: &[crate::nodes::TextRun],
    images: &[crate::tables::QuantRect],
    visible: crate::tables::QuantRect,
) -> bool {
    let chars: usize = runs.iter().map(|r| r.text.chars().count()).sum();
    if runs.is_empty()
        || chars >= crate::thresholds::STRAY_TEXT_MAX_CHARS
        || runs.iter().any(|r| drawn_area(r.geometry) == Some(None))
    {
        return false;
    }
    let area = |x0: i64, y0: i64, x1: i64, y1: i64| -> i128 {
        i128::from((x1 - x0).max(0)) * i128::from((y1 - y0).max(0))
    };
    let shown = area(visible.x0, visible.y0, visible.x1, visible.y1);
    if shown == 0 {
        return false;
    }
    union_area(images, visible) * 100
        >= shown * i128::from(crate::thresholds::STRAY_TEXT_MIN_IMAGE_PERCENT)
}

/// Decision #161 (PR 58's review): the area `rects` cover together inside `within`, overlap
/// counted once. A sweep across x: each rectangle enters at its left edge and leaves at its right,
/// and a segment tree over the distinct y edges keeps the height covered between — O(n log n) in
/// the rectangles, as a page may place tens of thousands of strips.
fn union_area(rects: &[crate::tables::QuantRect], within: crate::tables::QuantRect) -> i128 {
    let inside: Vec<[i64; 4]> = rects
        .iter()
        .map(|r| {
            [
                r.x0.max(within.x0),
                r.y0.max(within.y0),
                r.x1.min(within.x1),
                r.y1.min(within.y1),
            ]
        })
        .filter(|[x0, y0, x1, y1]| x0 < x1 && y0 < y1)
        .collect();
    if inside.is_empty() {
        return 0;
    }
    let mut ys: Vec<i64> = inside.iter().flat_map(|r| [r[1], r[3]]).collect();
    ys.sort_unstable();
    ys.dedup();
    let mut edges: Vec<(i64, i32, usize, usize)> = Vec::with_capacity(2 * inside.len());
    for [x0, y0, x1, y1] in &inside {
        let (a, b) = (
            ys.partition_point(|y| y < y0),
            ys.partition_point(|y| y < y1),
        );
        edges.push((*x0, 1, a, b));
        edges.push((*x1, -1, a, b));
    }
    edges.sort_unstable();
    let mut cover = Cover {
        ys: &ys,
        count: vec![0; 4 * ys.len()],
        height: vec![0; 4 * ys.len()],
    };
    let (mut total, mut last) = (0i128, edges[0].0);
    for (x, enter, a, b) in edges {
        total += i128::from(cover.height[1]) * i128::from(x - last);
        cover.add(1, 0, ys.len() - 1, a, b, enter);
        last = x;
    }
    total
}

/// [`union_area`]'s segment tree over `ys`: node `at` holds the span `ys[lo]..ys[hi]`, `count` how
/// many rectangles cover all of it, and `height` how much of it is covered.
struct Cover<'a> {
    ys: &'a [i64],
    count: Vec<i32>,
    height: Vec<i64>,
}

impl Cover<'_> {
    /// A rectangle over the spans `a..b` enters (`enter` 1) or leaves (-1) under node `at`.
    fn add(&mut self, at: usize, lo: usize, hi: usize, a: usize, b: usize, enter: i32) {
        if b <= lo || hi <= a {
            return;
        }
        if a <= lo && hi <= b {
            self.count[at] += enter;
        } else {
            let mid = (lo + hi) / 2;
            self.add(2 * at, lo, mid, a, b, enter);
            self.add(2 * at + 1, mid, hi, a, b, enter);
        }
        self.height[at] = if self.count[at] > 0 {
            self.ys[hi] - self.ys[lo]
        } else if hi - lo == 1 {
            0
        } else {
            self.height[2 * at] + self.height[2 * at + 1]
        };
    }
}

/// Whether a document declares no **author** structure — decision #29's gate
/// (`docs/28-HEADINGS-SCOPE.md` §6.1).
///
/// **No `/StructTreeRoot` at all**, which is the predicate that declares
/// `untagged-structure-tree-absent`, so the gate and that declaration cannot disagree about one
/// document. **Or a tree this engine's own writer created, every element of it**: a tree whose
/// every binding reads back `Computed` is no declaration — `group_key`'s argument, *the licence
/// does not transfer* — and without this arm an engine-tagged document would stop inferring what
/// its untagged original infers, and its projections would stop equalling the original's
/// (docs/23 §4.3).
///
/// One author element closes the gate. That is stricter than §6.1's words — *no `Extracted`
/// binding* — only on a tree mixing an author's elements with this engine's, which is a shape the
/// writer never produces (it tags only a document with no tree), and the conservative reading of
/// a document someone else edited.
/// Whether a scalar lies in one of the right-to-left blocks, for
/// [`crate::limitations::right_to_left_not_reordered`].
///
/// **A block test, deliberately, and named so.** Unicode's `Bidi_Class` is the property that
/// actually answers *is this character right-to-left*, and the right-to-left rule reads it
/// (`crate::bidi`, decision #54); this count, which predates the rule, keeps reading the ranges the
/// right-to-left scripts occupy instead, the wider net for a statement of what was left undone:
/// `U+0590`–`U+08FF` is Hebrew through Arabic Extended-A (Syriac, Thaana, NKo, Samaritan and
/// Mandaic among them), and the two presentation-form ranges follow. The two astral ranges are the
/// ones Unicode reserves for right-to-left scripts, whose unassigned code points default to
/// `Bidi_Class` R or AL: `U+10800`–`U+10FFF` (Kharoshthi, Old Turkic, Hanifi Rohingya, Sogdian
/// and more) and `U+1E800`–`U+1EFFF` (Mende Kikakui, Adlam, the Arabic mathematical alphabet).
///
/// It therefore also matches a few scalars in those blocks that are not themselves right-to-left
/// — an Arabic-Indic digit is `Bidi_Class` `AN`, not `R` or `AL`. That is the safe direction: the
/// limitation it fires is a statement about what this reader did NOT do, and declaring it on a
/// document that needed no reordering costs a consumer nothing, while missing one costs them a
/// quote that silently will not match. The limitation's own wording claims blocks, not classes,
/// so what it says is true of what this measures.
fn is_right_to_left_block(c: char) -> bool {
    matches!(
        c as u32,
        0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x1_0800..=0x1_0FFF | 0x1_E800..=0x1_EFFF
    )
}

/// Whether a label's text opens as a figure's or a table's caption does (decision #159): a caption
/// word — `Figure`, `Fig.`, `Table`, `Chart`, `Exhibit`, `Scheme`, `Algorithm`, `Plate`,
/// `Listing`, or `Abbildung`, `Tabelle`, `Tableau`, `Figura`, `Tabla`, `Tabella` — then a colon,
/// or a label: a number (`3.3.`), a Roman numeral (`TABLE I.`; one letter only as `I`, `V` or `X`,
/// and none after `Exhibit`, whose letters head attachments) or a letter before a number (`S1`,
/// `A.2`). A caption names what stands by it and heads no section; `Table of Contents` and
/// `Tablespoon 2` are none. The text is its runs' text run together, since a page may draw a line
/// a glyph to a run.
fn names_a_figure(text: &str) -> bool {
    const WORDS: [&str; 15] = [
        "figure",
        "fig",
        "table",
        "chart",
        "exhibit",
        "scheme",
        "algorithm",
        "plate",
        "listing",
        "abbildung",
        "tabelle",
        "tableau",
        "figura",
        "tabla",
        "tabella",
    ];
    let lower = text.trim_start().to_lowercase();
    WORDS
        .iter()
        .filter_map(|word| lower.strip_prefix(word).map(|rest| (*word, rest)))
        .any(|(word, rest)| {
            let rest = rest.strip_prefix('.').unwrap_or(rest);
            if rest.starts_with(':') {
                return true;
            }
            let label = rest.trim_start();
            // The word ends where the label begins: a space, or a digit set against it.
            if label.len() == rest.len() && !label.starts_with(|c: char| c.is_ascii_digit()) {
                return false;
            }
            let token: String = label
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '.')
                .collect();
            let token = token.trim_end_matches('.');
            let mut chars = token.chars();
            match (chars.next(), chars.next()) {
                (Some(c), _) if c.is_ascii_digit() => true,
                (Some(c), Some(d)) if c.is_alphabetic() && (d.is_ascii_digit() || d == '.') => true,
                // A Roman numeral (`IV`), but not one letter of an exhibit's or of a schedule
                // lettered past `X`: `EXHIBIT C` heads its attachment (third review).
                (Some(_), _) => {
                    word != "exhibit"
                        && token.chars().all(|c| "ivxlc".contains(c))
                        && (token.len() > 1 || matches!(token, "i" | "v" | "x"))
                }
                (None, _) => false,
            }
        })
}

fn no_author_structure(tree: Option<&crate::structure::StructureTree>) -> bool {
    match tree {
        None => true,
        Some(tree) => tree
            .engine_written
            .as_ref()
            .is_some_and(|written| written.elements >= declared_len(tree.elements)),
    }
}

/// The page's runs as the tracks and chart rules read them: each run's origin, em and measured box
/// — its extent ending at its last inked character — and whether a table already holds its origin.
fn rule_runs<'a>(
    runs: &'a [crate::nodes::TextRun],
    ems: &[Option<i64>],
    ink_ends: &[Option<i64>],
    tables: &[crate::tables::DetectedTable],
) -> Vec<crate::tracks::TrackRun<'a>> {
    runs.iter()
        .zip(ems)
        .zip(ink_ends)
        .map(|((r, em), ink_end)| {
            let (x, y) = (r.locator.origin_x, r.locator.origin_y);
            crate::tracks::TrackRun {
                x,
                y,
                em: *em,
                rect: r.geometry.measured().map(|b| crate::tables::QuantRect {
                    x0: b.x0(),
                    y0: b.y0(),
                    x1: ink_end.unwrap_or(b.x1()),
                    y1: b.y1(),
                }),
                text: r.text.as_str(),
                claimed: tables
                    .iter()
                    .any(|t| x >= t.rect.x0 && x < t.rect.x1 && y >= t.rect.y0 && y < t.rect.y1),
            }
        })
        .collect()
}

/// Decision #105: the chart rules read no run the page clips away. It is marked as held, so every
/// run keeps its place.
fn hide_clipped(runs: &mut [crate::tracks::TrackRun<'_>], clipped: &[bool]) {
    for (run, &away) in runs.iter_mut().zip(clipped) {
        run.claimed |= away;
    }
}

/// A page's candidate heading lines: each line's run indices in the final order, and the line
/// reduced to what the verdict needs (decision #29).
/// The advance of a run's trailing whitespace, in the units of `per_code` — the run's advance per
/// code — where each code decoded to one character of `text` and this reader `inserted` none;
/// `None` where the run does not end in whitespace, is whitespace alone, or cannot be read so.
fn trailing_whitespace_advance(
    text: &str,
    per_code: Option<&[f64]>,
    inserted: bool,
) -> Option<f64> {
    let per = per_code?;
    if inserted || per.len() != text.chars().count() {
        return None;
    }
    let trailing = text.chars().rev().take_while(|c| c.is_whitespace()).count();
    (trailing > 0 && trailing < per.len()).then(|| per[per.len() - trailing..].iter().sum())
}

/// Decision #61: a run's travel along the page's x, where it runs forward along that axis — across
/// it by less than the artifact can express; `None` otherwise.
fn travel_along_x(geom: &PageGeometry, displacement: Option<(f64, f64)>) -> Option<f64> {
    displacement
        .map(|(dx, dy)| geom.to_top_left_linear(dx, dy))
        .filter(|&(along, across)| along > 0.0 && across.abs() < crate::content::COORD_EPSILON)
        .map(|(along, _)| along)
}

/// The advance of a run's leading whitespace, in the units of `per_code`, on the terms of
/// [`trailing_whitespace_advance`]; `None` where the run does not open with whitespace, is
/// whitespace alone, or cannot be read so.
fn leading_whitespace_advance(text: &str, per_code: Option<&[f64]>, inserted: bool) -> Option<f64> {
    let per = per_code?;
    if inserted || per.len() != text.chars().count() {
        return None;
    }
    let leading = text.chars().take_while(|c| c.is_whitespace()).count();
    (leading > 0 && leading < per.len()).then(|| per[..leading].iter().sum())
}

/// `ruled-rects-v13` (decision #71): where a run's first inked character stands, for the ruled
/// rule to place it by — its origin moved past its leading whitespace by the pen's own advances,
/// for a run whose travel runs forward along the page's x (`along_x`, as decision #61 reads it)
/// and whose codes are its characters one for one; its origin otherwise.
///
/// A page that sets a word apart by widening the space before it (`Tw`) draws ` None` from where
/// the figure before it ended: its origin lies in the column before, and its ink in its own.
fn first_glyph_x(
    origin_x: i64,
    along_x: bool,
    text: &str,
    per_code: Option<&[f64]>,
    inserted: bool,
) -> i64 {
    if !along_x {
        return origin_x;
    }
    leading_whitespace_advance(text, per_code, inserted)
        .and_then(|a| quantize(a, QUANTUM_PER_POINT).ok())
        .map_or(origin_x, |a| origin_x + a)
}

type HeadingLines = Vec<(Vec<usize>, crate::headings::Line)>;

/// How much heavier than its page's body a run's weight class must be to be read as bold
/// (`type-size-v7`, decision #64; the run's own `bold` since decision #82): one class, the step
/// between Light and Regular or Regular and Medium.
const WEIGHT_STEP: u32 = 100;

/// A weight class to the nearest hundred, as fonts state them.
fn weight_class(weight: u32) -> u32 {
    (weight + 50) / 100 * 100
}

/// Decision #82, folded into `page-observations-v4`: a run set a weight class or more heavier than
/// the class most of its page's characters are set in — Medium over Light, Regular over Light — is
/// bold, as a reader sees it and as `type-size-v7` (decision #64) read it for the heading rule
/// alone. The class itself is never on the wire; a run whose font states none is left as it is.
fn heavier_runs_bold(runs: &mut [TextRun]) {
    let mut by_weight: std::collections::BTreeMap<u32, u64> = std::collections::BTreeMap::new();
    for run in runs.iter() {
        if let Some(weight) = run.weight {
            *by_weight.entry(weight_class(weight)).or_default() +=
                run.text.trim().chars().count() as u64;
        }
    }
    let Some(body) = by_weight
        .iter()
        .max_by_key(|&(&weight, &chars)| (chars, std::cmp::Reverse(weight)))
        .map(|(&weight, _)| weight)
    else {
        return;
    };
    for run in runs.iter_mut() {
        if run
            .weight
            .is_some_and(|weight| weight_class(weight) >= body + WEIGHT_STEP)
        {
            run.bold = true;
        }
    }
}

/// One page's candidate heading lines (decision #29), and its share of the body em.
///
/// **The line is formed here and never in `headings.rs`.** A line is the runs sharing one band,
/// one `/Artifact` state and one baseline — `markdown.rs`'s `LineKey`, read for equality only —
/// and the rule's own file may not name the band: that is its guard, because it reads type and
/// never position. So this groups, and the rule sees each line's runs by their type alone — and,
/// since decision #38, the line's text and one bit: whether the leading-gap cut made the line a
/// block of its own.
///
/// Only lines that could be headings against *some* body em are kept, so the fold holds one
/// entry per display line rather than one per line of the document.
fn page_heading_lines(
    runs: &[TextRun],
    ems: &[Option<i64>],
    tables: &[crate::tables::DetectedTable],
) -> (HeadingLines, crate::headings::EmTally) {
    use crate::headings::{EmTally, Line, Typed};

    let owned: std::collections::BTreeSet<usize> = tables
        .iter()
        .flat_map(|t| t.cells.iter())
        .flat_map(|c| c.run_indices.iter().copied())
        .collect();
    let typed: Vec<Typed> = runs
        .iter()
        .zip(ems)
        .enumerate()
        .map(|(i, (run, em))| Typed {
            em: *em,
            chars: run.text.chars().count() as u64,
            blank: run.text.trim().is_empty(),
            artifact: matches!(
                run.structural,
                Some(ethos_parser_core::StructuralLocator::PdfArtifact(_))
            ),
            table_owned: owned.contains(&i),
            // `type-size-v7` (decision #64): a run set a weight class heavier than its page's body
            // is read as bold — carried since decision #82 by the run's own `bold`
            // ([`heavier_runs_bold`]), which the projections emphasise too.
            bold: run.bold,
        })
        .collect();

    let mut tally = EmTally::default();
    let mut by_line: std::collections::BTreeMap<(Option<u32>, bool, i64), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, (run, t)) in runs.iter().zip(&typed).enumerate() {
        tally.add(t);
        by_line
            .entry((run.region, t.artifact, run.locator.origin_y))
            .or_default()
            .push(i);
    }
    // Decision #38. The lines in reading order — `runs` is in its final order, so a line's first
    // run places it — because the bold clause reads whether a line stands apart: where the
    // leading-gap cut gave it a block of its own (`type-size-v3`; a bold line wrapped onto two is
    // not one, a caption as often as a heading), or, opening with a section number, where the
    // nearest measured line above it does not continue it by the layout-unit rule's own geometry
    // (`-v4`): a numbered heading is often set tight over its text. A line with no measured box
    // stands apart from nothing.
    let mut ordered: Vec<Vec<usize>> = by_line.into_values().collect();
    ordered.sort_unstable_by_key(|indices| indices.first().copied());
    let members: Vec<Vec<Typed>> = ordered
        .iter()
        .map(|indices| indices.iter().map(|&i| typed[i]).collect())
        .collect();
    let boxes: Vec<Option<(i64, i64, i64, i64)>> = ordered
        .iter()
        .map(|indices| {
            indices
                .iter()
                .filter(|&&i| !runs[i].text.trim().is_empty())
                .filter_map(|&i| runs[i].geometry.measured())
                .map(|b| (b.x0(), b.y0(), b.x1(), b.y1()))
                .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
        })
        .collect();
    let measured: Vec<usize> = (0..ordered.len()).filter(|&n| boxes[n].is_some()).collect();
    let apart = |a: usize, b: usize| match (boxes[a], boxes[b]) {
        (Some(a), Some(b)) => !crate::units::continues(a, b),
        _ => false,
    };
    // Every line counts toward the reference (`type-size-v2`'s body is the largest size that runs
    // on many lines); only a line that could be a heading is kept for the verdict.
    let mut lines = Vec::new();
    for (n, indices) in ordered.iter().enumerate() {
        tally.add_line(&members[n]);
        let text: String = indices.iter().map(|&i| runs[i].text.as_str()).collect();
        let alone_in_block = {
            let block_of = |k: usize| ordered[k].first().and_then(|&i| runs[i].block);
            let opens = n == 0 || block_of(n - 1) != block_of(n);
            let closes = n + 1 == ordered.len() || block_of(n + 1) != block_of(n);
            block_of(n).is_some() && opens && closes
        };
        let numbered_with_room_above = crate::headings::section_number(&text).is_some()
            && measured
                .binary_search(&n)
                .is_ok_and(|p| p.checked_sub(1).is_none_or(|q| apart(measured[q], n)));
        let isolated = alone_in_block || numbered_with_room_above;
        let line = Line::of(&members[n], &text, isolated);
        if line.is_candidate() {
            lines.push((indices.clone(), line));
        }
    }
    (lines, tally)
}

/// Decision #167: where the tree and a detector describe the same grid, a cell the tree tags `/TH`
/// is a header cell of the table the detector found — when the detected cell at its position holds
/// a run of the marked content the `/TH` cites on this page. Where the content does not meet, no
/// tag is carried onto another cell. `mcid_of` gives a detected cell's run's marked-content id.
fn carry_tagged_headers(
    cells: &mut [crate::tables::DetectedCell],
    tagged: &[crate::structure::TaggedCell],
    page: lopdf::ObjectId,
    mcid_of: impl Fn(usize) -> Option<i64>,
) {
    let heads: std::collections::BTreeMap<(u32, u32), Vec<i64>> = tagged
        .iter()
        .filter(|t| t.header && t.page == Some(page))
        .map(|t| {
            let mut mcids = t.mcids.clone();
            mcids.sort_unstable();
            ((t.row, t.column), mcids)
        })
        .collect();
    if heads.is_empty() {
        return;
    }
    for cell in cells {
        if let Some(mcids) = heads.get(&(cell.position.row, cell.position.column)) {
            cell.header |= cell
                .run_indices
                .iter()
                .any(|&i| mcid_of(i).is_some_and(|m| mcids.binary_search(&m).is_ok()));
        }
    }
}

/// Resolve one run's structural address (v1-S3).
///
/// The precedence is the point, so it is stated rather than left to fall out of the `if`s:
///
/// 1. **Artifact wins.** The page itself said this content is furniture and outside the structure
///    tree (§14.8.2.2). That is the document's own statement about its own content, and it
///    outranks anything a join could conclude. The run stays in the artifact — flagged, never
///    dropped.
/// 2. **A tree citation binds.** Only on exact `(page, mcid)` equality. Nothing fuzzy, nothing
///    nearest-match: an mcid means one thing on one page, and a looser join would file text under
///    a heading that does not claim it.
/// 3. **Otherwise the bare id**, exactly as v0 emitted it. This is a *smaller* claim than a bound
///    role path, and the difference is preserved rather than smoothed over.
/// 4. **Otherwise nothing**, because the page marked nothing here.
fn bind_structure(
    tree: Option<&crate::structure::StructureTree>,
    page: lopdf::ObjectId,
    mcid: Option<i64>,
    artifact: bool,
) -> Option<ethos_parser_core::StructuralLocator> {
    use ethos_parser_core::{PdfArtifactLocator, StructuralLocator};

    if artifact {
        return Some(StructuralLocator::PdfArtifact(PdfArtifactLocator { mcid }));
    }
    let mcid = mcid?;
    match tree.and_then(|t| t.locator_for(page, mcid)) {
        Some(found) => Some(StructuralLocator::PdfTagged(found.clone())),
        None => Some(StructuralLocator::PdfMcid(mcid)),
    }
}

/// Flip an annotation rectangle into the artifact's declared coordinate system (v1-S4).
///
/// The same transform a glyph origin goes through, for the same reason: geometry that lived in a
/// different coordinate system from the text around it would be uncheckable by construction.
/// A rectangle that will not quantize becomes `Malformed` rather than a guess — and never a
/// page-sized box.
fn to_top_left_rect(
    geom: &PageGeometry,
    rect: ethos_parser_core::AnnotationRect,
) -> ethos_parser_core::AnnotationRect {
    use ethos_parser_core::AnnotationRect;

    let Some(r) = rect.declared() else {
        return rect;
    };
    let q = f64::from(QUANTUM_PER_POINT);
    let (ax, ay) = geom.to_top_left(r.x0() as f64 / q, r.y0() as f64 / q);
    let (bx, by) = geom.to_top_left(r.x1() as f64 / q, r.y1() as f64 / q);
    let to_q = |v: f64| quantize(v, QUANTUM_PER_POINT).ok();
    match (to_q(ax), to_q(ay), to_q(bx), to_q(by)) {
        (Some(x0), Some(y0), Some(x1), Some(y1)) => {
            match ethos_parser_core::QRect::new(x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)) {
                Ok(r) => AnnotationRect::Declared(r),
                Err(_) => AnnotationRect::Malformed,
            }
        }
        _ => AnnotationRect::Malformed,
    }
}

/// Everything one admitted page contributes to the artifact, produced with a
/// page-local id allocator whose ids the sequential fold in [`extract`] rewrites.
struct PageYield {
    page: PageExtract,
    ruled_refusals: Vec<(u32, crate::tables::RuledRefusal)>,
    stroke_refusals: Vec<(u32, crate::stroke_ruled::Refusal)>,
    unruled_refusals: Vec<(u32, crate::unruled::Refusal)>,
    encoding_dropped_runs: u32,
    encoding_detail: String,
    /// Runs holding a code their font's declared map left unmapped and the font stated (#43).
    font_read_runs: u32,
    mcids_unbound: u32,
    unclaimed_tree_items: u32,
    /// Runs bound under an element this engine's own writer created (auto-tagging S1).
    computed_bound: u32,
    /// For every run in `page.runs`, in the same order, the index of the operation that showed
    /// it in the page's decoded content (auto-tagging S2). Permuted by `reorder_page` with the
    /// runs, so `op_indices[i]` describes `page.runs[i]` in the final order.
    op_indices: Vec<usize>,
    /// This page's candidate heading lines, each with its runs' indices in the final order
    /// (decision #29). Empty unless the heading rule runs on this document.
    heading_lines: HeadingLines,
    /// This page's share of the document's body em. Empty unless the heading rule runs.
    em_tally: crate::headings::EmTally,
    props_by_name: u32,
    alternate_texts: u32,
    partial_code_runs: u32,
    tagged_without_geometric: Vec<u32>,
    unresolved_field_parents: u32,
    field_states_not_determinable: u32,
    undecodable_object_texts: u32,
    inline_images: u32,
    unresolved_xobjects: u32,
    undescended_xobjects: u32,
    /// `/MCID`s opened inside forms, which bind nothing (`docs/30-FORM-XOBJECTS-SCOPE.md` §7).
    form_mcids: u32,
    /// Runs drawn inside forms, which the tag writer cannot mark (§8).
    form_runs: u32,
    composite_fonts: u32,
    findings_seen: std::collections::BTreeMap<&'static str, u32>,
    /// Document-scoped limitations a font on this page declared. Deduped by the fold.
    font_limitations: Vec<ethos_parser_core::Limitation>,
    /// The page-local allocator, counters included — the fold rebases every id by
    /// the document-global base and then replays the same NUMBER of allocations,
    /// because a refused candidate consumes an id it never ships (a cross-check
    /// rejection mints `t1` and emits nothing), and the sequential artifact keeps
    /// that hole. Rewriting only emitted entities would close it and renumber
    /// every id after it.
    local_ids: IdAllocator,
    /// v3 (`docs/35-OCR-SCOPE.md` §4): the page the OCR trigger took, on its first reading.
    candidate: Option<crate::recognize::Candidate>,
    /// v3: why a page the trigger would read was not, by name — the trigger's own refusal, the
    /// request's, or the recognizer's; `ocr-not-requested` under a profile that pins none.
    ocr_refusal: Option<String>,
    /// v3: recognized words not written because no viewer shows where they lie.
    recognized_outside: u32,
    /// Decision #161: recognized words not written because a drawn run lies where they would, or
    /// because no box tells them apart from one.
    recognized_under_text: u32,
}

/// One page's whole extraction, exactly the body the sequential loop ran, with two
/// differences that the fold undoes: ids come from a page-local allocator (rewritten
/// into the document-global sequence afterwards), and cross-page accumulators are
/// returned as this page's deltas instead of mutated in place. Pure with respect to
/// the document handle, which is what lets pages run in parallel.
///
/// `Ok(None)` when the page holds more than `limit` content operations: [`page_operations`]'s
/// answer, passed up so the caller can read the page again alone.
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
fn extract_page(
    doc: &Document,
    profile: &Profile,
    profile_sha256: &ethos_parser_core::Sha256Hex,
    structure: &Option<crate::structure::StructureTree>,
    tree_mcids_by_page: &std::collections::BTreeMap<lopdf::ObjectId, Vec<i64>>,
    page_number: u32,
    page_id: lopdf::ObjectId,
    content: &crate::budget::ContentBudget,
    limit: usize,
    words: Option<&[crate::recognize::PageWord]>,
) -> Result<Option<PageYield>, EngineError> {
    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let mut encoding_dropped_runs: u32 = 0;
    let mut font_read_runs: u32 = 0;
    let mut encoding_detail = String::new();
    let mut unruled_refusals: Vec<(u32, crate::unruled::Refusal)> = Vec::new();
    let mut ruled_refusals: Vec<(u32, crate::tables::RuledRefusal)> = Vec::new();
    let mut stroke_refusals: Vec<(u32, crate::stroke_ruled::Refusal)> = Vec::new();
    let mut mcids_unbound: u32 = 0;
    let mut unclaimed_tree_items: u32 = 0;
    let mut computed_bound: u32 = 0;
    let mut props_by_name: u32 = 0;
    let mut alternate_texts: u32 = 0;
    let mut partial_code_runs: u32 = 0;
    let mut tagged_without_geometric: Vec<u32> = Vec::new();
    let mut unresolved_field_parents: u32 = 0;
    let mut field_states_not_determinable: u32 = 0;
    let mut undecodable_object_texts: u32 = 0;
    let mut inline_images: u32 = 0;
    let mut unresolved_xobjects: u32 = 0;
    let mut undescended_xobjects: u32 = 0;
    let mut form_mcids: u32 = 0;
    let mut form_runs: u32 = 0;
    let mut findings_seen: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();
    let mut composite_fonts: u32 = 0;
    let mut font_limitations: Vec<ethos_parser_core::Limitation> = Vec::new();
    // Auto-tagging S2. One entry per run pushed in the loop below, skipping exactly what the
    // loop skips, so the two vectors are aligned by construction and stay so through
    // `reorder_page`.
    let mut op_indices: Vec<usize> = Vec::new();
    // Decision #29. The rendered em of every run pushed below, aligned with `runs` exactly as
    // `op_indices` is and permuted beside it by `reorder_page`. Held only to reduce this page's
    // lines for the heading rule; like the operator indices, it lives beside the runs and never
    // on a `TextRun`, so it never reaches the wire.
    let mut ems: Vec<Option<i64>> = Vec::new();
    // `whitespace-tracks-v5`: where each run's last inked character ends, beside `ems` and aligned
    // with `runs` as it is, until the tracks pass reads it. Never on a `TextRun`.
    let mut ink_ends: Vec<Option<i64>> = Vec::new();
    // `ruled-rects-v13` (decision #71): where each run's first inked character stands.
    let mut glyph_starts: Vec<i64> = Vec::new();
    // Decision #54: how many characters each of a run's codes decoded to, aligned with `runs` in
    // content order until the right-to-left rule reads it, before `reorder_page`. Never on a
    // `TextRun`.
    let mut code_chars: Vec<Vec<usize>> = Vec::new();
    // v3: the OCR trigger's answer for this page, decided after its images are read.
    let mut candidate: Option<crate::recognize::Candidate> = None;
    let mut ocr_refusal: Option<String> = None;
    // The gate: the document declares no author structure (see `no_author_structure`) — or, since
    // decision #94, its tree tags no heading — and the profile names the rule; any other id,
    // `not-run-for-this-format` included, runs nothing.
    let infer_headings = profile.heading_inference_rule
        == ethos_parser_core::HEADING_INFERENCE_RULE_V8
        && (no_author_structure(structure.as_ref())
            || structure.as_ref().is_some_and(|t| !t.declares_headings()));
    let mut heading_lines: HeadingLines = Vec::new();
    let mut em_tally = crate::headings::EmTally::default();
    let page_extract;
    {
        let page_dict =
            doc.inner()
                .get_dictionary(page_id)
                .map_err(|e| EngineError::Malformed {
                    what: "page dictionary".into(),
                    detail: e.to_string(),
                })?;

        let geom = PageGeometry::resolve(doc, page_dict)?;
        let fonts = load_page_fonts(doc, page_dict)?;
        describe_fonts(fonts.values(), &mut composite_fonts, &mut font_limitations);

        let Some(operations) = page_operations(doc.inner(), page_number, page_id, content, limit)?
        else {
            return Ok(None);
        };

        // v1-S6. The page's `/XObject` names, so `Do` can be resolved to an object number. The
        // interpreter still never holds a `Document` — it gets names and ids, and extraction
        // sorts `/Image` from `/Form` where the document is already in scope.
        let xobjects = crate::images::page_xobjects(doc.inner(), page_dict);
        // `docs/30-FORM-XOBJECTS-SCOPE.md` §5. The forms the page draws, resolved as it draws
        // them, within what is left of its operation allowance; a page they push past it is read
        // again alone, and refused there under the page ceiling.
        let allowance = limit.saturating_sub(operations.len());
        let forms =
            crate::form_xobjects::PageForms::new(doc, page_dict, page_number, content, allowance);
        let resolve = |id| forms.resolve(id);
        // Decision #139: the names the page's `/Properties` give layers a viewer does not show.
        let resources = page_resources(doc.inner(), page_dict);
        let layers = doc.layers().names(doc.inner(), resources.as_ref());
        let plain = crate::layers::plain_states(doc.inner(), resources.as_ref());
        let spaces = crate::layers::device_like_spaces(doc.inner(), resources.as_ref());
        let mut interp = Interpreter::new(&fonts)
            .with_xobjects(xobjects)
            .with_forms(&resolve, allowance)
            .with_layers(layers)
            .with_plain_states(plain)
            .with_colour_spaces(spaces);
        match interp.run(&operations) {
            Err(_) if interp.form_ops_exhausted => return Ok(None),
            outcome => outcome?,
        }
        content.charge(0, interp.form_ops_run)?;
        for form in forms.drawn() {
            describe_fonts(
                form.fonts.values(),
                &mut composite_fonts,
                &mut font_limitations,
            );
        }
        form_mcids = declare(form_mcids, interp.form_mcids);
        inline_images = inline_images.saturating_add(interp.inline_images);
        unresolved_xobjects = unresolved_xobjects.saturating_add(interp.unresolved_xobjects);

        // v0.1: a font that cannot map a code drops its run rather than failing the document.
        // Accumulated across pages so the artifact declares one honest total.
        encoding_dropped_runs = encoding_dropped_runs.saturating_add(interp.dropped_runs);
        font_read_runs = font_read_runs.saturating_add(interp.fallback_runs);
        if interp.dropped_runs > 0 {
            if let Some(first) = interp.undecodable.first() {
                if encoding_detail.is_empty() {
                    encoding_detail = format!("page {page_number}: {first}");
                }
            }
        }

        // Taken by value: the interpreter's buffers are dead after this loop, and the
        // run text was the largest allocation in extraction to clone. The other
        // interpreter fields (rects, segments, counters) are read below and stay put.
        let shown_runs = std::mem::take(&mut interp.shown);
        // Decision #155: a page whose text is drawn turned is read in the turn its text is set in,
        // before any of it is measured, so every box, rule and table below is upright.
        let turn = page_turn(&shown_runs, geom.rotation);
        let geom = geom.turned(turn);
        // v1-S6. The frame an off-page finding is measured against, in the same coordinate system
        // the runs end up in. Computed once per page rather than per run.
        let visible = geom.visible_in_display_space();
        let mut runs = Vec::with_capacity(shown_runs.len());
        // Decision #105: which runs the page clips away, for the chart rules.
        let mut clipped = Vec::with_capacity(shown_runs.len());
        op_indices.reserve(shown_runs.len());
        ems.reserve(shown_runs.len());
        code_chars.reserve(shown_runs.len());
        for shown in shown_runs {
            if shown.text.is_empty() {
                continue;
            }
            op_indices.push(shown.op_index);
            form_runs = declare(form_runs, u32::from(shown.in_form));
            // A matrix with no vertical scale, or an em too large to quantize, is unmeasurable
            // rather than an error: the heading rule reads it as absent, never as zero.
            ems.push(
                quantize(shown.em_scale_pt, QUANTUM_PER_POINT)
                    .ok()
                    .filter(|em| *em > 0),
            );
            let font = Some(&shown.font);

            let (ox_pt, oy_pt) = geom.to_top_left(shown.origin.0, shown.origin.1);
            let origin_x = quantize(ox_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let origin_y = quantize(oy_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            // Decision #61, `docs/22-WORD-BOXES-SCOPE.md` §9 item 2's open half: the locator's
            // advance is the pen's travel in the declared system wherever that travel runs forward
            // along its x axis — across it by less than the artifact can express — as the run's
            // box already is. Measured along the text matrix's x alone, a run a producer turned
            // upright with its text matrix on a turned page advanced 0, and every projection that
            // reads the advance read its line one glyph apart. Upright text on an unturned page is
            // the same number bit for bit; a travel off the x axis, or backward along it, keeps
            // the text matrix's length.
            let travel = travel_along_x(&geom, shown.displacement);
            let advance = travel
                .or(shown.advance)
                .map(|a| quantize(a, QUANTUM_PER_POINT))
                .transpose()
                .map_err(quantize_err)?;

            // The per-code advances must account for the total, and must be present on exactly
            // the runs the total is present on. Checked here rather than asserted in a comment,
            // because a vector that silently drops an entry stays plausible: it is still
            // monotonic, still sums close, and every sub-run box built from it after the gap is
            // wrong by one glyph. `debug_assert` because this is an interpreter invariant rather
            // than a document property — a PDF cannot violate it, only a bug here can.
            //
            // The tolerance is deliberate and not a fudge. `advance` is `sum(deltas) * scale`
            // while this is `sum(delta * scale)`, and those differ in the last bits by
            // construction. The engine keeps the first spelling because it is the one every
            // artifact so far was quantized from; changing the arithmetic to make an assertion
            // exact would be the assertion editing its own subject.
            debug_assert_eq!(
                shown.code_advances.is_some(),
                shown.advance.is_some(),
                "code_advances and advance must agree on presence"
            );
            debug_assert_eq!(
                shown.displacement.is_some(),
                shown.advance.is_some(),
                "displacement and advance must agree on presence"
            );
            if let (Some(per), Some(total)) = (&shown.code_advances, shown.advance) {
                debug_assert_eq!(
                    per.len(),
                    shown.codes.len(),
                    "one advance per code, or the alignment is a lie"
                );
                let summed: f64 = per.iter().sum();
                debug_assert!(
                    (summed - total).abs() <= 1e-6 * total.abs().max(1.0),
                    "per-code advances sum to {summed}, total says {total}"
                );
            }

            let geometry = match (font, shown.displacement) {
                // v1-S6.2. **A run that draws no ink has no ink box.** `ink_box` builds its
                // rectangle from the font's ascent/descent envelope stretched over the pen's
                // travel — not from glyph outlines — so for a run of spaces it produced a
                // rectangle around nothing and labelled it `Measured`. On `nist-sp-800-53r5` 3 450
                // of those landed past the page edge and the seal refused the whole document: 491
                // of 492 pages unreadable over content that draws nothing.
                //
                // The test is `trim().is_empty()`, which is Unicode whitespace — so a space, a tab,
                // and a non-breaking space all qualify. Deliberately narrow: a zero-width space
                // (U+200B) is *not* Unicode whitespace and still gets a box, because over-claiming
                // absence would be the same mistake pointed the other way.
                (Some(_), Some(_)) if shown.text.trim().is_empty() => {
                    ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::NoInkToMeasure,
                    )
                }
                // docs/22 §9 items 1 and 2. The travel and the glyph axis are user-space vectors,
                // so they take the linear part of the same `/Rotate` the origin just took — a
                // quarter-turned page turns the box with it instead of laying it along x.
                (Some(f), Some((dx, dy))) => f.ink_box(
                    ox_pt,
                    oy_pt,
                    geom.to_top_left_linear(dx, dy),
                    geom.to_top_left_linear(shown.glyph_up.0, shown.glyph_up.1),
                    shown.em_scale_pt,
                ),
                // No advance means no width, so there is no box to measure — and a box guessed
                // from the font size is exactly what this project refuses.
                _ => ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::NotReportedByReader,
                ),
            };

            // D4-S5. **A box the document draws off the page is measured, and un-emittable.**
            // `seal` refuses a measured box outside its page on the grounds that it means the
            // measurement or the transform is wrong. For a page extracted from a wider original it
            // means neither — `01030000000029.pdf` sets `Tm` at x = −435.1181 pt against a media
            // box starting at 0, and the engine transformed that faithfully to −43512. Six of two
            // hundred DP-Bench documents did this and produced no artifact at all.
            //
            // Refused here rather than at the seal, and the difference is deliberate: the seal's
            // invariant is what catches a genuine transform bug, and it keeps that job unchanged.
            // This decides the narrower question the seal cannot see — whether the box is
            // *reportable* — while the page is still in scope, and answers it with the same typed
            // absence the rest of this function uses. The run keeps its text and its origin, and
            // its `OffPage` finding where this engine raised one against the visible box — which
            // tests the origin alone, so a run that starts on the page and whose box runs past its
            // edge carries none.
            let geometry = match geometry {
                ethos_parser_core::GeometryPresence::Measured(r) if !geom.contains(r) => {
                    ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::MeasuredOffPage,
                    )
                }
                g => g,
            };

            // The tracks rule reads a cell's extent off its runs' boxes, and a box spans the run's
            // trailing whitespace: a space drawn with a wide advance carried `3 ` across the gap
            // to the next column, and three cells read as one. Where the run is upright and its
            // codes are its characters one for one, the end of its last inked character.
            glyph_starts.push(first_glyph_x(
                origin_x,
                travel.is_some(),
                &shown.text,
                shown.code_advances.as_deref(),
                !shown.synthesized_indices.is_empty(),
            ));
            ink_ends.push(match geometry {
                ethos_parser_core::GeometryPresence::Measured(b)
                    if (b.x0() - origin_x).abs() <= crate::tracks::UPRIGHT_TOLERANCE =>
                {
                    trailing_whitespace_advance(
                        &shown.text,
                        shown.code_advances.as_deref(),
                        !shown.synthesized_indices.is_empty(),
                    )
                    .and_then(|a| quantize(a, QUANTUM_PER_POINT).ok())
                    .map(|a| b.x1() - a)
                    .filter(|&end| end > b.x0())
                }
                _ => None,
            });

            let findings = run_findings(&shown, origin_x, origin_y, &visible);
            code_chars.push(shown.code_chars);

            let synthesized: Vec<SynthesizedChar> = shown
                .synthesized_indices
                .iter()
                .map(|i| SynthesizedChar {
                    char_index: *i,
                    reason: SynthesisReason::TjGap,
                })
                .collect();

            let scalar_code_mismatch = shown.text.chars().count() != shown.codes.len();

            clipped.push(shown.clipped);
            // Decision #139: a run in a membership dictionary is counted, and carries no finding.
            if shown.layer == Some(crate::layers::Layer::NotEvaluated) {
                *findings_seen
                    .entry(ethos_parser_core::codes::OPTIONAL_CONTENT_MEMBERSHIP_NOT_EVALUATED)
                    .or_insert(0) += 1;
            }
            runs.push(TextRun {
                read_beside: None,
                id: alloc.next(IdKind::Span)?,
                text: shown.text,
                char_codes: shown.codes,
                scalar_code_mismatch,
                synthesized,
                font_id: shown.font_id,
                font_size: quantize(shown.font_size, QUANTUM_PER_POINT).map_err(quantize_err)?,
                // D4-S2. No cut has run yet — this page's runs are still being read off the
                // content stream, and the rule needs the whole page plus its accepted tables.
                // Filled in below, where `arrange_page` is called.
                region: None,
                block: None,
                layout_unit: None,
                inferred_heading: false,
                inferred_heading_level: None,
                // Decision #38: the typeface the document chose, as its font states it — and since
                // `page-observations-v4` a run painted fill-then-stroke (rendering mode 2 or 6),
                // which is how a writer with no bold face of a font draws its glyphs bold.
                bold: shown.font.bold || matches!(shown.render_mode, 2 | 6),
                italic: shown.font.italic,
                weight: shown.font.weight,
                script: None,
                strikeout: false,
                underline: false,
                furniture: None,
                reading: None,
                locator: PdfLocator {
                    page: page_number,
                    origin_x,
                    origin_y,
                    advance,
                },
                geometry,
                mcid: shown.mcid,
                // v1-S3. The join against the document's structure tree, or the honest lesser
                // answer when the tree does not reach this run. `bind_structure` never invents:
                // an unbound id stays an unbound id.
                structural: bind_structure(structure.as_ref(), page_id, shown.mcid, shown.artifact),
                // Text and origins are read from the document's own encoding.
                derivation: DerivationClass::Extracted,
                // v1-S6. Observations about the run, never a reason to withhold it. Each is
                // computed from evidence the page supplies: the text rendering mode the content
                // stream set, the visible box the page declares, and since decision #139 the size,
                // the layer and the fill the page drew it in.
                findings,
                recognized: None,
            });
        }

        // v3 (`docs/35-OCR-SCOPE.md` §6): a scanned page read again with the words its recognizer
        // read, as runs of derivation `Recognized` — beside no drawn run, since the trigger took
        // only a page that showed no text, and before every rule that reads runs, so reading
        // order, lines, blocks, headings and tables read them as they read drawn ones. Their
        // place is the one a text layer states: the line's foot under the word, the word's
        // width, the line's height as the size. No font, no codes, no operation drew them.
        for w in words.into_iter().flatten() {
            op_indices.push(usize::MAX);
            ems.push(Some(w.font_size).filter(|em| *em > 0));
            glyph_starts.push(w.origin_x);
            ink_ends.push(w.rect.map(|r| r.x1()));
            code_chars.push(vec![1; w.text.chars().count()]);
            clipped.push(false);
            runs.push(TextRun {
                read_beside: None,
                id: alloc.next(IdKind::Span)?,
                text: w.text.clone(),
                char_codes: Vec::new(),
                scalar_code_mismatch: !w.text.is_empty(),
                synthesized: Vec::new(),
                font_id: String::new(),
                font_size: w.font_size,
                region: None,
                block: None,
                layout_unit: None,
                inferred_heading: false,
                inferred_heading_level: None,
                bold: false,
                italic: false,
                weight: None,
                script: None,
                strikeout: false,
                underline: false,
                furniture: None,
                reading: None,
                locator: PdfLocator {
                    page: page_number,
                    origin_x: w.origin_x,
                    origin_y: w.origin_y,
                    advance: w.advance,
                },
                // As a drawn run's, above: a word whose centre is on the page and whose box runs
                // past its edge keeps its text, not a box the seal would refuse the document for;
                // a word whose box encloses nothing reports none, as a run without an advance.
                geometry: match w.rect {
                    Some(r) if geom.contains(r) => ethos_parser_core::GeometryPresence::Measured(r),
                    Some(_) => ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::MeasuredOffPage,
                    ),
                    None => ethos_parser_core::GeometryPresence::Absent(
                        ethos_parser_core::GeometryAbsence::NotReportedByReader,
                    ),
                },
                mcid: None,
                structural: None,
                derivation: DerivationClass::Recognized,
                findings: Vec::new(),
                recognized: Some(crate::nodes::RecognizedSource {
                    image: w.image,
                    pixels: w.pixels,
                }),
            });
        }

        // The mcids this page's runs carry — membership only, so it is immune to the
        // reorder below. The index-bearing map the tagged join needs is built later,
        // after `runs` reaches its final order, precisely because a prebuilt index
        // here would be the stale-index bug the join's comment warns about.
        let run_mcids: std::collections::BTreeSet<i64> =
            runs.iter().filter_map(|run| run.mcid).collect();

        // Which of the tree's citations this page's runs actually answered. A cited pair that no
        // run claims is a real hole — the tree says there is content there and the content stream
        // did not mark any — and it is counted rather than filled with a fabricated run.
        if structure.is_some() {
            for &mcid in tree_mcids_by_page.get(&page_id).into_iter().flatten() {
                if !run_mcids.contains(&mcid) {
                    unclaimed_tree_items = declare(unclaimed_tree_items, 1);
                }
            }
        }
        mcids_unbound = declare(
            mcids_unbound,
            declared_len(
                runs.iter()
                    .filter(|r| {
                        matches!(
                            r.structural,
                            Some(ethos_parser_core::StructuralLocator::PdfMcid(_))
                        )
                    })
                    .count(),
            ),
        );
        // Auto-tagging S1. Runs whose binding is this engine's own tag read back — the count
        // `structure-tree-engine-written` names. Counted off the locators the join produced,
        // never off the tree alone, because only the join knows which citations a run answered.
        computed_bound = declare(
            computed_bound,
            declared_len(
                runs.iter()
                    .filter(|r| {
                        matches!(
                            &r.structural,
                            Some(ethos_parser_core::StructuralLocator::PdfTagged(t))
                                if t.derivation == DerivationClass::Computed
                        )
                    })
                    .count(),
            ),
        );
        props_by_name = props_by_name.saturating_add(interp.props_by_name);
        alternate_texts = alternate_texts.saturating_add(interp.alternate_texts);
        partial_code_runs = partial_code_runs.saturating_add(interp.partial_code_runs);

        // v1-S1: ruled tables, from the rectangles this page actually painted. Rects arrive in
        // user space and go through the SAME transform and quantum as a glyph origin — a table
        // whose geometry lived in a different coordinate system from the text inside it would be
        // uncheckable by construction.
        let mut table_rects = Vec::with_capacity(interp.rects.len());
        for r in &interp.rects {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_rects.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        // `ruled-rects-v10`: those a fill alone painted white, through the same transform.
        let mut table_white = Vec::with_capacity(interp.white.len());
        for r in &interp.white {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_white.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        let mut table_inset = Vec::with_capacity(interp.inset.len());
        for r in &interp.inset {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_inset.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        let origins: Vec<crate::tables::RunOrigin<'_>> = runs
            .iter()
            .map(|r| crate::tables::RunOrigin {
                x: r.locator.origin_x,
                y: r.locator.origin_y,
                text: r.text.as_str(),
            })
            .collect();
        // `ruled-rects-v13` (decision #71): the same runs where their first inked character
        // stands, for the ruled rule alone to place them by.
        let glyphs: Vec<crate::tables::RunOrigin<'_>> = runs
            .iter()
            .zip(&glyph_starts)
            .map(|(r, &x)| crate::tables::RunOrigin {
                x,
                y: r.locator.origin_y,
                text: r.text.as_str(),
            })
            .collect();
        // v1-S8. The ruling LINES the page stroked, split by orientation before the rule sees
        // them. Both go through the same transform and quantum as a glyph origin, for the reason
        // the rectangles do: geometry in a different coordinate system from the text inside it
        // would be uncheckable by construction.
        //
        // Horizontal segments define the rows; vertical ones are read only as corroboration that
        // a column boundary was drawn. `stroke-ruled-v1` discarded the verticals here and that is
        // precisely what made it both refuse `cfpb-home-loan-toolkit` page 13 and accept six bands
        // on its Closing Disclosure pages — see `crate::stroke_ruled`.
        let mut stroke_rules = Vec::new();
        let mut uprights = Vec::new();
        // Decision #154: the level rules stroked solid, which alone draw a strikeout or underline.
        let mut solid_rules = Vec::new();
        for seg in &interp.segments {
            let (ax, ay) = geom.to_top_left(seg.x0, seg.y0);
            let (bx, by) = geom.to_top_left(seg.x1, seg.y1);
            let r = crate::tables::quantize_rect(ax, ay, bx, by)?;
            if geom.level(seg) {
                let rule = crate::stroke_ruled::Rule {
                    y: r.y0,
                    x0: r.x0,
                    x1: r.x1,
                };
                if seg.solid {
                    solid_rules.push(rule);
                }
                stroke_rules.push(rule);
            } else {
                uprights.push(crate::stroke_ruled::Upright {
                    x: r.x0,
                    y0: r.y0,
                    y1: r.y1,
                });
            }
        }

        // v1-S8. The rectangles this page's form-field widgets declare, flipped into page space.
        // Read straight off `/Annots` rather than through the forms walk, because that walk is
        // gated on a capability and reads text — see `crate::forms::widget_rects`.
        let field_rects: Vec<crate::tables::QuantRect> =
            crate::forms::widget_rects(doc.inner(), page_dict)
                .into_iter()
                .map(|r| {
                    let q = f64::from(QUANTUM_PER_POINT);
                    let (ax, ay) = geom.to_top_left(r.x0() as f64 / q, r.y0() as f64 / q);
                    let (bx, by) = geom.to_top_left(r.x1() as f64 / q, r.y1() as f64 / q);
                    crate::tables::quantize_rect(ax, ay, bx, by)
                })
                .collect::<Result<Vec<_>, _>>()?;

        // v1-S2, widened at v1-S8: ruled first, then the ruling lines, then the alignment rule on
        // whatever text neither has claimed.
        let mut detected = crate::tables::detect(
            page_number,
            &table_rects,
            &table_white,
            // `ruled-rects-v12`: a group's decoration is left out only where the document
            // declares no author structure — where it does, a grid read through its decoration
            // would take a declared table's place in the pairing below.
            no_author_structure(structure.as_ref()).then_some(&table_inset[..]),
            &stroke_rules,
            &uprights,
            &origins,
            &glyphs,
            &field_rects,
            &mut alloc,
        )?;
        // `ruled-rects-v13` (decision #100): two cells a ruled grid read on one band of fills,
        // one cell.
        let mut table_fills = Vec::with_capacity(interp.filled_alone.len());
        for f in &interp.filled_alone {
            let (ax, ay) = geom.to_top_left(f.rect.x0, f.rect.y0);
            let (bx, by) = geom.to_top_left(f.rect.x1, f.rect.y1);
            table_fills.push((crate::tables::quantize_rect(ax, ay, bx, by)?, f.fill));
        }
        for t in &mut detected.tables {
            if t.rule == ethos_parser_core::TABLE_DETECTION_V14 {
                crate::tables::join_band_cells(t, &table_fills, &table_rects, &uprights, &origins);
            }
        }
        let mut tables = detected.tables;

        // `ruled-rects-v13` (decision #74): a ruled table's header set above its top rule, read
        // off the text's alignment as the whitespace rule reads its grids — so, like that rule,
        // only where the document declares no author structure.
        if no_author_structure(structure.as_ref()) {
            let header_runs = rule_runs(&runs, &ems, &ink_ends, &tables);
            for t in tables
                .iter_mut()
                .filter(|t| t.rule == ethos_parser_core::TABLE_DETECTION_V14)
            {
                crate::tables::header_above(t, &header_runs);
            }
        }

        // Decision #38. Last, on the runs no table above holds: grids inferred from the whitespace
        // across a table's rows (`docs/31-TABLE-TRACKS-SCOPE.md`). Here, before reading order,
        // because the rule reads the order the content stream wrote the rows in. **Only where the
        // document declares no author structure**: a tagged document says what is a table, and an
        // inferred one would override a list or a paragraph it declared — and, on a page that
        // declares a `/Table`, take that table's place in the pairing below.
        if profile.table_detection.tracks == ethos_parser_core::TABLE_DETECTION_TRACKS_V16
            && no_author_structure(structure.as_ref())
        {
            let track_runs = rule_runs(&runs, &ems, &ink_ends, &tables);
            // Decision #125: a page of recognized text, whose stream order is the recognizer's.
            let recognized = runs
                .iter()
                .any(|r| r.derivation == DerivationClass::Recognized);
            // The page's drawn horizontal ink: its stroked rules, and the top and bottom of each
            // rectangle it paints in a colour of its own.
            let mut ink = stroke_rules.clone();
            for r in table_rects.iter().filter(|r| !table_white.contains(r)) {
                for y in [r.y0, r.y1] {
                    ink.push(crate::stroke_ruled::Rule {
                        y,
                        x0: r.x0,
                        x1: r.x1,
                    });
                }
            }
            for table in
                crate::tracks::detect(page_number, &track_runs, &ink, recognized, &mut alloc)?
            {
                if !tables.iter().any(|t| t.rect.overlaps(table.rect)) {
                    tables.push(table);
                }
            }
            // `-v4`: then inside each of the page's columns, where one is prose — the first
            // vertical cut of the reading-order rule over the runs no table holds yet.
            let held: Vec<bool> = runs
                .iter()
                .map(|r| {
                    let (x, y) = (r.locator.origin_x, r.locator.origin_y);
                    tables
                        .iter()
                        .any(|t| x >= t.rect.x0 && x < t.rect.x1 && y >= t.rect.y0 && y < t.rect.y1)
                })
                .collect();
            let free: Vec<usize> = (0..runs.len()).filter(|&i| !held[i]).collect();
            let geometry: Vec<crate::reading_order::RunGeometry> = free
                .iter()
                .map(|&i| crate::reading_order::RunGeometry {
                    x: runs[i].locator.origin_x,
                    y: runs[i].locator.origin_y,
                    advance: runs[i].locator.advance,
                })
                .collect();
            if let Some(columns) = crate::reading_order::columns(&geometry) {
                let columns: Vec<Vec<usize>> = columns
                    .into_iter()
                    .map(|column| column.into_iter().map(|k| free[k]).collect())
                    .collect();
                let unheld: Vec<crate::tracks::TrackRun<'_>> = track_runs
                    .iter()
                    .zip(&held)
                    .map(|(run, &h)| crate::tracks::TrackRun {
                        claimed: run.claimed || h,
                        ..*run
                    })
                    .collect();
                for table in crate::tracks::detect_in_columns(
                    page_number,
                    &unheld,
                    &columns,
                    &ink,
                    recognized,
                    &mut alloc,
                )? {
                    if !tables.iter().any(|t| t.rect.overlaps(table.rect)) {
                        tables.push(table);
                    }
                }
            }
        }

        // Decision #42. Then a bar chart's printed labels, as its table
        // (`docs/32-CHART-LABELS-SCOPE.md`): on the runs no table above holds, under the tracks
        // rule's gate, and kept only where it overlaps no table already found.
        let mut bars = Vec::with_capacity(
            interp.filled.len() + interp.filled_repeated.len() + interp.filled_rounded.len(),
        );
        let square = interp.filled.iter().chain(&interp.filled_repeated);
        for (f, rounded) in square
            .map(|f| (f, false))
            .chain(interp.filled_rounded.iter().map(|f| (f, true)))
            // Decision #105: a straight line stroked wide, rounded where its cap is round.
            .chain(interp.stroked.iter().map(|(f, round)| (f, *round)))
        {
            let (ax, ay) = geom.to_top_left(f.rect.x0, f.rect.y0);
            let (bx, by) = geom.to_top_left(f.rect.x1, f.rect.y1);
            bars.push(crate::charts::Bar {
                rect: crate::tables::quantize_rect(ax, ay, bx, by)?,
                fill: f.fill,
                rounded,
            });
        }
        let q = f64::from(QUANTUM_PER_POINT);
        let page_area = (geom.display_width * q) as i128 * (geom.display_height * q) as i128;
        if profile.table_detection.charts == ethos_parser_core::TABLE_DETECTION_CHARTS_V3
            && no_author_structure(structure.as_ref())
        {
            let mut chart_runs = rule_runs(&runs, &ems, &ink_ends, &tables);
            hide_clipped(&mut chart_runs, &clipped);
            // Decision #88: the small marks a fill painted, a legend's dots among them.
            let mut dots = Vec::with_capacity(interp.marks.len());
            for m in &interp.marks {
                let (ax, ay) = geom.to_top_left(m.rect.x0, m.rect.y0);
                let (bx, by) = geom.to_top_left(m.rect.x1, m.rect.y1);
                dots.push(crate::charts::Bar {
                    rect: crate::tables::quantize_rect(ax, ay, bx, by)?,
                    fill: m.fill,
                    rounded: false,
                });
            }
            for table in crate::charts::detect(
                page_number,
                &chart_runs,
                &bars,
                &dots,
                page_area,
                &mut alloc,
            )? {
                if !tables.iter().any(|t| t.rect.overlaps(table.rect)) {
                    tables.push(table);
                }
            }
        }

        // Decision #66. Then the numbers a line chart prints at its points, as its table
        // (`docs/32-CHART-LABELS-SCOPE.md` §9): on the runs no table above holds, under the tracks
        // rule's gate, and kept only where it overlaps no table already found.
        if profile.table_detection.lines == ethos_parser_core::TABLE_DETECTION_LINES_V1
            && no_author_structure(structure.as_ref())
        {
            let mut strokes = Vec::with_capacity(interp.polylines.len());
            for p in &interp.polylines {
                let mut points = Vec::with_capacity(p.points.len());
                for &(x, y) in &p.points {
                    let (ax, ay) = geom.to_top_left(x, y);
                    let r = crate::tables::quantize_rect(ax, ay, ax, ay)?;
                    points.push((r.x0, r.y0));
                }
                strokes.push(crate::lines::Stroke {
                    points,
                    colour: p.stroke,
                });
            }
            let mut marks = Vec::with_capacity(interp.marks.len());
            for m in &interp.marks {
                let (ax, ay) = geom.to_top_left(m.rect.x0, m.rect.y0);
                let (bx, by) = geom.to_top_left(m.rect.x1, m.rect.y1);
                marks.push(crate::lines::Mark {
                    rect: crate::tables::quantize_rect(ax, ay, bx, by)?,
                    colour: m.fill,
                });
            }
            let chart_bars = crate::charts::chart_bars(&bars, page_area);
            let mut line_runs = rule_runs(&runs, &ems, &ink_ends, &tables);
            hide_clipped(&mut line_runs, &clipped);
            for table in crate::lines::detect(
                page_number,
                &line_runs,
                &strokes,
                &marks,
                &chart_bars,
                &mut alloc,
            )? {
                if !tables.iter().any(|t| t.rect.overlaps(table.rect)) {
                    tables.push(table);
                }
            }
        }

        // Decision #90. Then the numbers a pie prints at its slices, as its table
        // (`docs/32-CHART-LABELS-SCOPE.md` §10): on the runs no table above holds, under the tracks
        // rule's gate, and kept only where it overlaps no table already found.
        if profile.table_detection.pies == ethos_parser_core::TABLE_DETECTION_PIES_V1
            && no_author_structure(structure.as_ref())
        {
            let place = |(x, y): (f64, f64)| -> Result<(i64, i64), EngineError> {
                let (ax, ay) = geom.to_top_left(x, y);
                let r = crate::tables::quantize_rect(ax, ay, ax, ay)?;
                Ok((r.x0, r.y0))
            };
            let mut slices = Vec::with_capacity(interp.wedges.len());
            for w in &interp.wedges {
                slices.push(crate::pies::Slice {
                    centre: place(w.centre)?,
                    rim: w.rim.iter().map(|&p| place(p)).collect::<Result<_, _>>()?,
                    fill: w.fill,
                });
            }
            let mut marks = Vec::with_capacity(interp.marks.len());
            for m in &interp.marks {
                let (ax, ay) = geom.to_top_left(m.rect.x0, m.rect.y0);
                let (bx, by) = geom.to_top_left(m.rect.x1, m.rect.y1);
                marks.push(crate::lines::Mark {
                    rect: crate::tables::quantize_rect(ax, ay, bx, by)?,
                    colour: m.fill,
                });
            }
            let mut pie_runs = rule_runs(&runs, &ems, &ink_ends, &tables);
            hide_clipped(&mut pie_runs, &clipped);
            for table in crate::pies::detect(page_number, &pie_runs, &slices, &marks, &mut alloc)? {
                if !tables.iter().any(|t| t.rect.overlaps(table.rect)) {
                    tables.push(table);
                }
            }
        }

        // Decision #46. Last, once every table rule has run: the paths the page painted, read as
        // one drawing where they cluster (`docs/34-FIGURE-REGIONS-SCOPE.md`). Under the tracks
        // rule's gate, for its reason: a tagged document says what is a figure.
        let mut figures: Vec<crate::figures::DetectedFigure> = Vec::new();
        if profile.figure_rule == ethos_parser_core::FIGURE_RULE_V1
            && no_author_structure(structure.as_ref())
        {
            let painted: Vec<(crate::tables::QuantRect, bool)> = interp
                .painted
                .iter()
                .filter_map(|&(r, shaped)| {
                    let (ax, ay) = geom.to_top_left(r.x0, r.y0);
                    let (bx, by) = geom.to_top_left(r.x1, r.y1);
                    // A path whose coordinates cannot be placed is no evidence of a figure.
                    crate::tables::quantize_rect(ax, ay, bx, by)
                        .ok()
                        .map(|q| (q, shaped))
                })
                .collect();
            let page_box =
                crate::tables::quantize_rect(0.0, 0.0, geom.display_width, geom.display_height)?;
            let table_boxes: Vec<crate::tables::QuantRect> =
                tables.iter().map(|t| t.rect).collect();
            let placed: Vec<(i64, i64, &str)> = runs
                .iter()
                .map(|r| (r.locator.origin_x, r.locator.origin_y, r.text.as_str()))
                .collect();
            figures = crate::figures::detect(&painted, page_box, &table_boxes, &placed);
        }

        // v1-S3: the document's own tags, compared against what the detectors found. The two
        // derivations meet here and nowhere else — the tree walk never saw a box, and no
        // detector ever saw a structure type.
        //
        // Paired by position: the nth `/Table` the tree describes on this page against the nth
        // table found on it. Anything cleverer would be matching two grids by geometry, and the
        // tagged half has no geometry to match with.
        // v2-S24. The tagged tables on this page a detector did NOT match, collected here and
        // emitted after reading order has settled the run list — so their cells' `run_indices`
        // address the final order rather than a pre-reorder one.
        let mut unmatched_tagged: Vec<&crate::structure::TaggedTable> = Vec::new();
        if let Some(tree) = structure.as_ref() {
            let tagged_here: Vec<&crate::structure::TaggedTable> = tree
                .tables
                .iter()
                .filter(|t| t.page == Some(page_id))
                .collect();
            for (i, tagged) in tagged_here.iter().enumerate() {
                match tables.get_mut(i) {
                    Some(found) => {
                        let positions: Vec<ethos_parser_core::TableCellPosition> =
                            found.cells.iter().map(|c| c.position.clone()).collect();
                        let check = tagged.check_against(found.rows, found.columns, &positions);
                        if matches!(check.outcome, ethos_parser_core::TaggedGridStatus::Ok) {
                            carry_tagged_headers(&mut found.cells, &tagged.cells, page_id, |i| {
                                runs[i].mcid
                            });
                        }
                        found.tagged_check = Some(check);
                    }
                    // The tree says there is a table here and no detector found one. **Until
                    // v2-S24 no table was emitted** — a grid built from `/TD` elements alone would
                    // have cells this engine placed, and with nothing on the wire to distinguish
                    // them a consumer could not tell them from cells a detector reconstructed off
                    // the page. `DerivationClass` is now that distinction: the table is emitted as
                    // `Extracted` under `tagged-tables-v2` with geometry typed-absent, so it says
                    // out loud that the document declared it and the engine read the tags rather
                    // than inferring a grid. This is the `None` arm the double-emission guard
                    // relies on — a tagged table that paired above is never emitted here.
                    None => unmatched_tagged.push(*tagged),
                }
            }
        }
        // A refused candidate is recorded once per page it happened on. Without this, a near-miss
        // page and a page with no grid-shaped text at all would both say `tables: []`, and only
        // one of them means "the alignment rule looked at something and decided against it".
        if let Some(r) = detected.refusal {
            unruled_refusals.push((page_number, r));
        }
        if let Some(r) = detected.ruled_refusal {
            ruled_refusals.push((page_number, r));
        }
        if let Some(r) = detected.stroke_refusal {
            stroke_refusals.push((page_number, r));
        }
        drop(origins);

        // Decision #40: superscripts and subscripts, from where each run sits beside its line.
        // Here, on the page's runs in content order, so `ems` still lines up with them.
        crate::scripts::assign(&mut runs, &ems);
        // Decision #154: strikeout and underline, from the thin lines the page drew through and
        // under its runs.
        let wide = interp
            .stroked
            .iter()
            .map(|(f, _)| {
                let (ax, ay) = geom.to_top_left(f.rect.x0, f.rect.y0);
                let (bx, by) = geom.to_top_left(f.rect.x1, f.rect.y1);
                crate::tables::quantize_rect(ax, ay, bx, by)
            })
            .collect::<Result<Vec<_>, _>>()?;
        // And (review) those a stroke outlined white or dashed, which draw no line either.
        let mut unseen = table_white.clone();
        for r in &interp.faint {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            unseen.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        let ink =
            crate::decorations::Ink::of(&solid_rules, &uprights, &table_rects, &unseen, &wide);
        crate::decorations::assign(&mut runs, &ems, &ink);

        // Decision #168: the empty boxes the page draws, beside the run of their line, as
        // content-stream positions until the page's runs take their reading order and their ids.
        let mut drawn: Vec<(crate::tables::QuantRect, usize, ethos_parser_core::BoxSide)> =
            Vec::new();
        if profile.drawn_box_rule == ethos_parser_core::DRAWN_BOX_RULE_V1 {
            let quant = |r: &crate::content::PathRect| {
                let (ax, ay) = geom.to_top_left(r.x0, r.y0);
                let (bx, by) = geom.to_top_left(r.x1, r.y1);
                crate::tables::quantize_rect(ax, ay, bx, by).ok()
            };
            let mut paths: Vec<crate::drawn_boxes::Path> = interp
                .painted
                .iter()
                .filter_map(|(r, shaped)| {
                    quant(r).map(|rect| crate::drawn_boxes::Path {
                        rect,
                        shaped: *shaped,
                        coloured: false,
                        unseen: false,
                    })
                })
                .collect();
            // Beside each path, what fills a colour into it and what no reader sees of it.
            let coloured = interp
                .filled
                .iter()
                .chain(&interp.marks)
                .filter(|f| !f.fill.is_white())
                .filter_map(|f| quant(&f.rect));
            paths.extend(coloured.map(|rect| crate::drawn_boxes::Path {
                rect,
                shaped: false,
                coloured: true,
                unseen: false,
            }));
            // An image placed in a box marks it as a fill does.
            let placed_images = interp.images.iter().filter_map(|i| {
                let xs = i.corners.map(|c| c.0);
                let ys = i.corners.map(|c| c.1);
                let lo = (
                    xs.iter().copied().fold(f64::INFINITY, f64::min),
                    ys.iter().copied().fold(f64::INFINITY, f64::min),
                );
                let hi = (
                    xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                );
                quant(&crate::content::PathRect {
                    x0: lo.0,
                    y0: lo.1,
                    x1: hi.0,
                    y1: hi.1,
                })
            });
            paths.extend(placed_images.map(|rect| crate::drawn_boxes::Path {
                rect,
                shaped: false,
                coloured: true,
                unseen: false,
            }));
            paths.extend(unseen.iter().map(|&rect| crate::drawn_boxes::Path {
                rect,
                shaped: false,
                coloured: false,
                unseen: true,
            }));
            let placed: Vec<crate::drawn_boxes::Run> = runs
                .iter()
                .map(|r| {
                    let (x, y) = (r.locator.origin_x, r.locator.origin_y);
                    let end = r
                        .locator
                        .advance
                        .map_or(r.geometry.measured().map_or(x, |b| b.x1()), |a| {
                            x.saturating_add(a)
                        });
                    // The ink's extent: the advance short of the whitespace at either end, by its
                    // share of the run's characters.
                    let chars = r.text.chars().count().max(1) as i64;
                    let lead = r.text.chars().take_while(|c| c.is_whitespace()).count() as i64;
                    let trail = r
                        .text
                        .chars()
                        .rev()
                        .take_while(|c| c.is_whitespace())
                        .count() as i64;
                    let span = end - x;
                    let (ink_x, ink_end) = (
                        x + span * lead.min(chars) / chars,
                        end - span * trail.min(chars - lead.min(chars)) / chars,
                    );
                    crate::drawn_boxes::Run {
                        x: ink_x,
                        y,
                        end: ink_end,
                        // Its glyphs' centre: across, its ink's; down, its measured box's.
                        centre: (
                            (ink_x + ink_end) / 2,
                            r.geometry.measured().map_or(y, |b| (b.y0() + b.y1()) / 2),
                        ),
                        words: !r.text.trim().is_empty(),
                    }
                })
                .collect();
            drawn = crate::drawn_boxes::detect(&paths, &placed);
        }

        // v1-S5. **Reading order, and the only order there is.**
        //
        // Runs are in content-stream order at this point. The rule reads the page's whitespace
        // and says what order a human reads it in; `reorder_page` then makes the run list *be*
        // that order — array position, span id and ordinal all together, so nothing downstream
        // has to consult a second index to know what comes first. A parallel `reading_order`
        // field beside a stream-ordered array would be two answers to one question, which is the
        // defect this slice is here to avoid rather than introduce.
        //
        // **After detection, deliberately.** Both detectors read origins, not sequence, so
        // neither cares — but a table's box is what makes its runs one atom, and it does not
        // exist until detection has accepted one. Running the rule first would let a cut fall
        // through a grid before anything knew it was a grid.
        let mut order: Vec<usize> = (0..runs.len()).collect();
        // Decision #164: (value, the run it was read beside), as content-stream positions until
        // the page's runs take their reading order and their ids.
        let mut read_beside: Vec<(usize, usize)> = Vec::new();
        if profile.capabilities.multi_column_reading_order {
            let geometry: Vec<crate::reading_order::RunGeometry> = runs
                .iter()
                .map(|r| crate::reading_order::RunGeometry {
                    x: r.locator.origin_x,
                    y: r.locator.origin_y,
                    advance: r.locator.advance,
                })
                .collect();
            let boxes: Vec<crate::tables::QuantRect> = tables.iter().map(|t| t.rect).collect();
            let arranged = crate::reading_order::arrange_page(&geometry, &boxes);
            // Decision #157: text drawn over the page after its own, in fonts none of it uses — a
            // form filler's values — is read where it sits, beside the caption it answers. In
            // content-stream order here, as the rule reads it.
            let placed = {
                let drawn: Vec<crate::drawn_last::Drawn<'_>> = runs
                    .iter()
                    .map(|r| crate::drawn_last::Drawn {
                        x: r.locator.origin_x,
                        y: r.locator.origin_y,
                        advance: r.locator.advance,
                        // The height of its measured box: a `Tf` operand is no size where the
                        // text matrix scales it.
                        size: r
                            .geometry
                            .measured()
                            .map_or(r.font_size, |b| b.y1() - b.y0()),
                        font: &r.font_id,
                        text: &r.text,
                        artifact: matches!(
                            r.structural,
                            Some(ethos_parser_core::StructuralLocator::PdfArtifact(_))
                        ),
                        recognized: r.derivation == DerivationClass::Recognized,
                    })
                    .collect();
                crate::drawn_last::place(&drawn, &boxes, &arranged.regions, &arranged.order)
            };
            let placed = placed.map(|(order, beside)| {
                read_beside = beside;
                order
            });
            let arranged = crate::reading_order::Arrangement {
                order: placed.unwrap_or(arranged.order),
                regions: arranged.regions,
            };

            // D4-S2. **Before `reorder_page`, deliberately.** `arranged.regions` is indexed by
            // stream position, which is what `runs` is in right now; `reorder_page` then moves
            // whole runs, so each region travels with the run it describes and no index has to be
            // fixed afterwards. Doing this after the move would mean re-deriving the mapping the
            // move already performed.
            //
            // `regions` is empty on a page the cut did not divide, so `zip` does nothing at all
            // there — no pass over `runs`, and no allocation was taken to say "no regions". That
            // is the common page, and the engine's run time is linear in what it emits.
            for (run, region) in runs.iter_mut().zip(&arranged.regions) {
                run.region = *region;
            }

            // The block cut, on the same discipline and for the same reason: before
            // `reorder_page`, indexed by stream position, empty when the rule declined.
            //
            // It takes `arranged.order` as well, because a block is bounded by the vertical cut
            // too — two columns of prose are never one block, whatever their leading — and it
            // takes `arranged.regions` to know where those bounds are.
            let blocks = crate::blocks::subdivide(&geometry, &arranged.order, &arranged.regions);
            for (run, block) in runs.iter_mut().zip(&blocks) {
                run.block = *block;
            }
            order = arranged.order;
        }

        // Decision #54. A right-to-left line read right to left: its runs take their places in
        // the order above where their glyphs are read, and each carries its glyphs as read. Here,
        // on the runs in content order, so `code_chars` still lines up with them and each
        // reading travels with its run through `reorder_page`, as `region` and `block` do.
        if profile.right_to_left_rule == ethos_parser_core::RIGHT_TO_LEFT_RULE_V1 {
            let (read, readings) = {
                let seen: Vec<crate::bidi::Seen<'_>> = runs
                    .iter()
                    .zip(&code_chars)
                    .map(|(r, chars)| crate::bidi::Seen {
                        text: &r.text,
                        code_chars: chars,
                        x: r.locator.origin_x,
                        y: r.locator.origin_y,
                        advance: r.locator.advance,
                        region: r.region,
                    })
                    .collect();
                crate::bidi::read(&seen, &order)
            };
            for (run, reading) in runs.iter_mut().zip(readings) {
                run.reading = reading;
            }
            order = read;
        }
        drop(code_chars);

        reorder_page(&mut runs, &mut tables, &mut op_indices, &mut ems, &order);
        // Decision #164: each value names the run it was read beside, by the id that run took in
        // reading order.
        if !read_beside.is_empty() {
            let mut position = vec![0usize; order.len()];
            for (new, &old) in order.iter().enumerate() {
                position[old] = new;
            }
            for &(value, anchor) in &read_beside {
                runs[position[value]].read_beside = Some(runs[position[anchor]].id.clone());
            }
        }
        // Decision #168: each box names the run it is drawn beside by the id that run took in
        // reading order, the boxes in the reading order of their runs.
        let mut drawn_boxes: Vec<(
            crate::tables::QuantRect,
            ethos_parser_core::NodeId,
            ethos_parser_core::BoxSide,
        )> = Vec::new();
        if !drawn.is_empty() {
            let mut position = vec![0usize; order.len()];
            for (new, &old) in order.iter().enumerate() {
                position[old] = new;
            }
            drawn.sort_by_key(|&(r, i, side)| {
                (
                    position[i],
                    side == ethos_parser_core::BoxSide::After,
                    r.y0,
                    r.x0,
                )
            });
            for (rect, i, side) in drawn {
                drawn_boxes.push((rect, runs[position[i]].id.clone(), side));
            }
        }

        // Decision #54. A cell holding a run read right to left reads as its runs do: their
        // readings, in reading order — the concatenation a cell's text has always been, of the
        // text each run is read as.
        for cell in tables.iter_mut().flat_map(|t| t.cells.iter_mut()) {
            if cell.run_indices.iter().any(|&i| runs[i].reading.is_some()) {
                cell.text = cell
                    .run_indices
                    .iter()
                    .map(|&i| runs[i].reading_text())
                    .collect();
            }
        }
        debug_assert_eq!(
            op_indices.len(),
            runs.len(),
            "one operator index per run, or the side table lies about every run after the gap"
        );
        debug_assert_eq!(
            ems.len(),
            runs.len(),
            "one rendered em per run, for the same reason"
        );

        // Decision #82. A run set a weight class heavier than its page's body is bold, before the
        // heading rule reads the page's lines.
        heavier_runs_bold(&mut runs);

        // Decision #29. This page's lines reduced for the heading rule, and its share of the
        // document's body em. Here, after `reorder_page`, so the indices address the final list;
        // the verdict waits for the fold, because the body em is a mode over every page.
        if infer_headings {
            (heading_lines, em_tally) = page_heading_lines(&runs, &ems, &tables);
        }

        // v2-S24. Emit the tagged tables collected above, now that `runs` is in its final order.
        // A cell's `run_indices` address that final list, and its text is the runs the tree bound
        // beneath it concatenated in reading order — so fabrication stays 0 by construction, and
        // reordering the page cannot leave a stale index behind. No box is invented anywhere: the
        // geometry is typed-absent.
        //
        // Indices per mcid over the FINAL run order — built here and not a line
        // earlier, because `reorder_page` above renumbers every index.
        let mut runs_by_mcid: std::collections::BTreeMap<i64, Vec<usize>> =
            std::collections::BTreeMap::new();
        for (i, run) in runs.iter().enumerate() {
            if let Some(mcid) = run.mcid {
                runs_by_mcid.entry(mcid).or_default().push(i);
            }
        }
        let mut page_tagged_tables = Vec::new();
        for tagged in &unmatched_tagged {
            // A `/Table` the walk found no cell for is not a table — its `rows`/`columns` are 0 —
            // and an empty grid is not emitted. Left uncounted rather than declared as a
            // without-geometric page: there is nothing there to have geometry.
            if tagged.cells.is_empty() || tagged.rows == 0 || tagged.columns == 0 {
                continue;
            }
            let id = alloc.next(IdKind::Table)?;
            let mut cells = Vec::with_capacity(tagged.cells.len());
            for cell in &tagged.cells {
                // Bind by `(page, mcid)`, the key v1-S3 already uses. Only a cell the tree places
                // on THIS page may claim this page's runs: mcids restart per page, so a cell on
                // another page of a straddling table must not collect a run whose id collides.
                let (run_indices, text) = if cell.page == Some(page_id) {
                    // Collected via the page's mcid index, re-sorted, and DEDUPED —
                    // the last step is load-bearing. A malformed-but-parseable cell
                    // can cite the same mcid twice (`/K [0 0]`), and nothing dedups
                    // `TaggedCell::mcids`; the old whole-list scan visited each run
                    // once regardless, so without the dedup a doubled citation
                    // doubled the run into the cell's text and node_ids. Caught by
                    // an adversarial byte-comparison against the pre-index build.
                    let mut idx: Vec<usize> = cell
                        .mcids
                        .iter()
                        .filter_map(|mcid| runs_by_mcid.get(mcid))
                        .flatten()
                        .copied()
                        .collect();
                    idx.sort_unstable();
                    idx.dedup();
                    // Decision #54: each run as read, so a cell of right-to-left text reads as
                    // its runs do.
                    let mut text = String::new();
                    for &i in &idx {
                        text.push_str(runs[i].reading_text());
                    }
                    (idx, text)
                } else {
                    // A cell on another page of a multi-page table: its runs are not on this page,
                    // so it binds nothing here and carries the empty string. Honest rather than
                    // guessed — the page-granular emit cannot reach another page's runs.
                    (Vec::new(), String::new())
                };
                cells.push(crate::tables::TaggedCellRecord {
                    header: cell.header,
                    position: ethos_parser_core::TableCellPosition {
                        row: cell.row,
                        column: cell.column,
                        rowspan: cell.rowspan,
                        colspan: cell.colspan,
                        table_id: id.clone(),
                    },
                    run_indices,
                    text,
                    geometry: crate::tables::TAGGED_TABLE_GEOMETRY,
                });
            }
            page_tagged_tables.push(crate::tables::TaggedTableRecord {
                id,
                page: page_number,
                rows: tagged.rows,
                columns: tagged.columns,
                cells,
                rule: ethos_parser_core::TABLE_DETECTION_TAGGED_V2.to_string(),
                check: crate::tables::tagged_not_applicable_check(),
                geometry: crate::tables::TAGGED_TABLE_GEOMETRY,
                // Whose element the `/Table` is, as the walk read it off the element's own
                // attributes (auto-tagging S1). `Extracted` on every document the writer
                // produces, because it never emits a `/Table`; read rather than assumed.
                derivation: tagged.derivation,
            });
        }
        // v2-S24. The repurposed disclosure: a page that emitted a tagged table carries a table
        // with no geometry, so a consumer reading only the assurance block learns some tables on
        // this document are not groundable and why. It fires where a table IS emitted now, not
        // where one was withheld — the meaning `tagged-table-without-geometric-table` used to have.
        if !page_tagged_tables.is_empty() {
            tagged_without_geometric.push(page_number);
        }

        // v1-S4. Annotations and form fields, from the page's own `/Annots`. Walked here rather
        // than from `/AcroForm` downward because a field's node needs a page and a field
        // dictionary does not name one — its widget does, by being on that page.
        //
        // **The interpreter above never saw these.** Their text comes from dictionaries; nothing
        // in the content stream draws it, and nothing here feeds it back into `runs`.
        let mut objects = Vec::new();
        if profile.capabilities.form_fields || profile.capabilities.annotations {
            let found = crate::forms::read_page_objects(doc.inner(), page_dict);
            unresolved_field_parents =
                unresolved_field_parents.saturating_add(found.unresolved_parents);
            if profile.capabilities.form_fields {
                field_states_not_determinable =
                    field_states_not_determinable.saturating_add(found.states_not_determinable);
            }
            undecodable_object_texts =
                undecodable_object_texts.saturating_add(found.undecodable_texts);

            for object in found.objects {
                let is_field = matches!(object.detail, crate::forms::PageObjectDetail::Field(_));
                // A profile with one capability off still reads the other. The two are separate
                // claims, so turning one off must not silently take the other with it.
                if is_field && !profile.capabilities.form_fields {
                    continue;
                }
                if !is_field && !profile.capabilities.annotations {
                    continue;
                }

                objects.push(crate::nodes::PageObjectRecord {
                    id: alloc.next(if is_field {
                        IdKind::FormField
                    } else {
                        IdKind::Annotation
                    })?,
                    locator: ethos_parser_core::PdfObjectLocator {
                        page: page_number,
                        object: object.id.0,
                        generation: u32::from(object.id.1),
                        // The `/Rect` arrives in user space and goes through the SAME transform
                        // and quantum as a glyph origin. A rectangle in a different coordinate
                        // system from the text around it would be uncheckable by construction.
                        rect: to_top_left_rect(&geom, object.rect),
                    },
                    text: object.text,
                    attributes: match object.detail {
                        crate::forms::PageObjectDetail::Field(a) => {
                            ethos_parser_core::NodeAttributes::FormField(a)
                        }
                        crate::forms::PageObjectDetail::Annotation(a) => {
                            ethos_parser_core::NodeAttributes::Annotation(a)
                        }
                    },
                    // v1-S4 decision 7: the structure tree may cite a widget by object
                    // reference. S3 walked `/OBJR` and bound nothing; this is where that
                    // binding would land. No role is invented when the tree is silent.
                    structural: structure
                        .as_ref()
                        .and_then(|t| t.locator_for_object(object.id))
                        .map(|l| ethos_parser_core::StructuralLocator::PdfTagged(l.clone())),
                });
            }
        }

        // v1-S6. One node per `Do`, in the order the page painted them. A `/Form` the
        // interpreter ran is not here at all — its text and images are — and one it did not enter
        // yields no node (`docs/30-FORM-XOBJECTS-SCOPE.md` §7), nor does an XObject with no
        // readable `/Subtype`: emitting a node for an unlabelled stream would put a picture on the
        // wire the document never called one.
        let mut images = Vec::new();
        // Decision #161: what of each upright image the page's clips show, for [`stray_text`].
        let mut image_areas = Vec::new();
        // Decision #156 (review): what an image's own `/OC` makes of it.
        let image_layer = |object: lopdf::ObjectId| {
            doc.inner()
                .get_object(object)
                .and_then(lopdf::Object::as_stream)
                .ok()
                .and_then(|s| s.dict.get(b"OC").ok())
                .and_then(|oc| doc.layers().layer(doc.inner(), oc))
        };
        // One stream's facts per object, however often the page draws it: hashing the stored
        // bytes again at every `Do` is the same answer at the stream's cost each time.
        let mut image_facts: std::collections::HashMap<
            lopdf::ObjectId,
            Option<ethos_parser_core::ImageAttributes>,
        > = std::collections::HashMap::new();
        for placement in &interp.images {
            let Some(attributes) = image_facts
                .entry(placement.object)
                .or_insert_with(|| crate::images::image_attributes(doc.inner(), placement.object))
                .clone()
            else {
                // v2.2-S2. **Counted, not merely skipped.** `image_attributes` returns `None` for
                // a `/Form` and for a stream with no readable `/Subtype`, and until now this
                // `continue` was the end of it: the placement was discarded and the artifact said
                // nothing. A page whose entire content is `q /Xf1 Do Q` — the shape a
                // page-slicing tool produces, and 4 of 104 documents sampled from a corpus — then
                // emitted zero nodes with `pages_failed: 0`, and a consumer could not tell it
                // from a blank page.
                //
                // The profile-scoped `form-xobject-text-not-descended` does not close that: it is
                // on EVERY artifact this engine writes, including documents with no XObject at
                // all, so it says what the engine never does rather than what happened here. This
                // is the same argument that already produced `unresolved_xobjects` and
                // `inline_images` two arms away in `content.rs` — *"no image nodes" must not be
                // able to mean "there were images and the reader lost them"* — applied to the one
                // case it had not been.
                //
                // An image that did not load as a stream is no form: it is a resource this reader
                // could not resolve. `lopdf` loads a stream whose direct `/Length` misses its data
                // as the bare dictionary, so an image drawn by every renderer that finds
                // `endstream` was declared a form not descended (tracker I17). A form lost the same
                // way stays counted as one: its text is absent from the page either way, and the
                // page-scoped count is what tells a search over that page so.
                let lost_image = match doc.inner().get_object(placement.object) {
                    Ok(lopdf::Object::Dictionary(d)) => d
                        .get(b"Subtype")
                        .and_then(lopdf::Object::as_name)
                        .is_ok_and(|s| s == b"Image"),
                    _ => false,
                };
                if lost_image {
                    unresolved_xobjects = unresolved_xobjects.saturating_add(1);
                } else {
                    undescended_xobjects = undescended_xobjects.saturating_add(1);
                }
                continue;
            };
            // **The capability gates the NODE, not the count above it**, and the order is the
            // whole reason this loop is no longer wrapped in the `if`. What the counter declares
            // is text this reader did not read; `capabilities.images` says whether this profile
            // emits picture nodes. A profile that turned images off and inherited silence about
            // form XObjects would be the identical hole one scope narrower — and it is the hole
            // this slice exists to close, so it is not worth reopening for the profile that does
            // not exist yet. No PDF profile ships with this false today, which is exactly why
            // the branch has to be written down rather than discovered later.
            if !profile.capabilities.images {
                continue;
            }
            let corners = placement.corners.map(|(x, y)| geom.to_top_left(x, y));
            let rect = crate::images::painted_rect(corners);
            // An image its own `/OC` switches off shows nothing (PR 58's review).
            let hidden = image_layer(placement.object) == Some(crate::layers::Layer::Off);
            if let (ethos_parser_core::PaintedRect::Painted(_), Some(s), false) =
                (&rect, placement.shown, hidden)
            {
                let ((ax, ay), (bx, by)) =
                    (geom.to_top_left(s.x0, s.y0), geom.to_top_left(s.x1, s.y1));
                if let Ok(area) = crate::tables::quantize_rect(ax, ay, bx, by) {
                    image_areas.push(area);
                }
            }
            // Decision #97: the runs drawn over the placement, now that `runs` is in reading order.
            let runs_inside = match &rect {
                ethos_parser_core::PaintedRect::Painted(q) => {
                    crate::nodes::runs_inside(&runs, (q.x0(), q.y0(), q.x1(), q.y1()))
                }
                _ => Vec::new(),
            };
            images.push(crate::nodes::ImageRecord {
                id: alloc.next(IdKind::Image)?,
                locator: ethos_parser_core::PdfImageLocator {
                    page: page_number,
                    object: placement.object.0,
                    generation: u32::from(placement.object.1),
                    rect,
                },
                attributes: ethos_parser_core::ImageAttributes {
                    runs_inside,
                    ..attributes
                },
            });
        }

        // v3 (`docs/35-OCR-SCOPE.md` §4): the trigger, a reason code and never a score. A page
        // that showed no text and placed an image is `classify`'s `scanned`; under a profile that
        // pins a recognizer it is read when it placed exactly one image, or several that the
        // recognizer can compose with the fills between them (decision #156), and refused by name
        // when it did not. Decided on the first reading only. Under a profile that pins none, the
        // page it would read is taken too, and counted after the pages are read (decision #132);
        // a page it would refuse here is not counted there. Since decision #161, a page that
        // showed a few words over a scan is taken too ([`stray_text`]), its drawn runs kept and
        // its recognized words written only beside them ([`beside_drawn`]).
        let stray = || {
            crate::tables::quantize_rect(visible.x0, visible.y0, visible.x1, visible.y1)
                .is_ok_and(|shown| stray_text(&runs, &image_areas, shown))
        };
        if words.is_none() && (interp.text_operators == 0 || stray()) {
            let placed = interp.images.len() + interp.inline_images as usize;
            let composite = &interp.composite;
            // Decision #156 (review): whether the page draws an annotation over itself, which a
            // composition does not hold.
            let composite_refusal = composite.refusal.or_else(|| {
                if draws_annotations(doc.inner(), page_dict) {
                    return Some("annotation");
                }
                composite.layers.iter().find_map(|layer| match *layer {
                    crate::content::CompositeLayer::Image { index, .. } => matches!(
                        image_layer(interp.images[index].object),
                        Some(crate::layers::Layer::NotEvaluated)
                    )
                    .then_some("layer-not-read"),
                    crate::content::CompositeLayer::Fill { .. } => None,
                })
            });
            let corner = |(x, y): (f64, f64)| geom.to_top_left(x, y);
            let shown = |r: &crate::content::PathRect| {
                let ((ax, ay), (bx, by)) = (corner((r.x0, r.y0)), corner((r.x1, r.y1)));
                (ax.min(bx), ay.min(by), ax.max(bx), ay.max(by))
            };
            if interp.images.len() == 1 && interp.inline_images == 0 && images.len() == 1 {
                let placement = &interp.images[0];
                // What a viewer sees of the image: the visible box, as far as the page's clips
                // show the image (PR 58's review). A word read outside it is text no viewer shows,
                // dropped and counted; an image they show none of shows no word.
                let seen = match placement.shown.map(|s| shown(&s)) {
                    Some((x0, y0, x1, y1)) => (
                        x0.max(visible.x0),
                        y0.max(visible.y0),
                        x1.min(visible.x1),
                        y1.min(visible.y1),
                    ),
                    None => (
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ),
                };
                candidate = Some(crate::recognize::Candidate {
                    page_id,
                    object: placement.object,
                    corners: placement.corners.map(corner),
                    visible: seen,
                    composite: None,
                });
            } else if interp.images.len() > 1
                && interp.inline_images == 0
                && composite_refusal.is_none()
            {
                let paints = composite
                    .layers
                    .iter()
                    .filter_map(|layer| match *layer {
                        crate::content::CompositeLayer::Image {
                            index,
                            shown: s,
                            fill,
                        } => {
                            let placement = &interp.images[index];
                            // An image in a layer a viewer hides is not painted (review).
                            image_layer(placement.object).is_none().then(|| {
                                crate::recognize::Paint::Image {
                                    object: placement.object,
                                    corners: placement.corners.map(corner),
                                    shown: shown(&s),
                                    fill,
                                }
                            })
                        }
                        crate::content::CompositeLayer::Fill { rect, fill } => {
                            Some(crate::recognize::Paint::Fill {
                                rect: shown(&rect),
                                fill,
                            })
                        }
                    })
                    .collect();
                candidate = Some(crate::recognize::Candidate::composed(
                    page_id,
                    (visible.x0, visible.y0, visible.x1, visible.y1),
                    paints,
                ));
            } else if matches!(
                profile.recognizer,
                ethos_parser_core::RecognizerPin::Pinned(_)
            ) {
                ocr_refusal = if interp.inline_images > 0 {
                    Some("inline-image".into())
                } else if placed > 1 {
                    // Decision #156: what the composition does not hold, by name.
                    Some(match composite_refusal {
                        Some(why) => format!("several-images:{why}"),
                        None => "several-images".into(),
                    })
                } else if placed == 1 || interp.unresolved_xobjects > 0 {
                    Some("image-not-loaded".into())
                } else {
                    None
                };
            }
        }

        for run in &runs {
            for f in &run.findings {
                *findings_seen.entry(f.as_code()).or_insert(0) += 1;
            }
        }

        let width = quantize(geom.display_width, QUANTUM_PER_POINT).map_err(quantize_err)?;
        let height = quantize(geom.display_height, QUANTUM_PER_POINT).map_err(quantize_err)?;
        // Decision #62: the box a viewer shows, where the page crops — the frame a consumer that
        // draws the page places its boxes in. Clipped to the media box already, so on the page.
        let shown = crate::tables::quantize_rect(visible.x0, visible.y0, visible.x1, visible.y1)?;
        let shown = crate::tables::QuantRect {
            x0: shown.x0.max(0),
            y0: shown.y0.max(0),
            x1: shown.x1.min(width),
            y1: shown.y1.min(height),
        };
        // Decision #168: a box on the page; one its edge crosses is none, and so is one a check
        // box widget covers half of or more where the widget states it on, or states a state this
        // engine cannot tell — the widget says what the field holds. A widget that states it off,
        // or states no button type, leaves the box.
        let drawn_boxes: Vec<ethos_parser_core::DrawnBox> = drawn_boxes
            .into_iter()
            .filter(|(r, _, _)| r.x0 >= 0 && r.y0 >= 0 && r.x1 <= width && r.y1 <= height)
            .filter(|(r, _, _)| {
                !objects.iter().any(|o| {
                    let ethos_parser_core::NodeAttributes::FormField(f) = &o.attributes else {
                        return false;
                    };
                    let says_on = matches!(
                        f.state,
                        Some(
                            ethos_parser_core::CheckState::Checked(_)
                                | ethos_parser_core::CheckState::NotDeterminable(_)
                        )
                    );
                    // Half the box or more under the widget: a field's edge brushing it is not it.
                    let covers = o.locator.rect.declared().is_some_and(|w| {
                        let across = (w.x1().min(r.x1) - w.x0().max(r.x0)).max(0);
                        let down = (w.y1().min(r.y1) - w.y0().max(r.y0)).max(0);
                        2 * i128::from(across) * i128::from(down)
                            >= i128::from(r.x1 - r.x0) * i128::from(r.y1 - r.y0)
                    });
                    says_on && covers
                })
            })
            .map(|(r, beside, side)| {
                Ok(ethos_parser_core::DrawnBox {
                    rect: r.as_qrect_checked()?,
                    beside,
                    side,
                })
            })
            .collect::<Result<Vec<_>, EngineError>>()?;
        page_extract = PageExtract {
            tables,
            tagged_tables: page_tagged_tables,
            figures,
            objects,
            images,
            index: page_number,
            width,
            height,
            rotation: geom.rotation,
            turn: geom.turn,
            visible: (shown.x0 > 0 || shown.y0 > 0 || shown.x1 < width || shown.y1 < height)
                .then_some(shown),
            drawn_boxes,
            runs,
        };
    }
    Ok(Some(PageYield {
        page: page_extract,
        ruled_refusals,
        stroke_refusals,
        unruled_refusals,
        encoding_dropped_runs,
        encoding_detail,
        font_read_runs,
        mcids_unbound,
        unclaimed_tree_items,
        computed_bound,
        op_indices,
        heading_lines,
        em_tally,
        props_by_name,
        alternate_texts,
        partial_code_runs,
        tagged_without_geometric,
        unresolved_field_parents,
        field_states_not_determinable,
        undecodable_object_texts,
        inline_images,
        unresolved_xobjects,
        undescended_xobjects,
        form_mcids,
        form_runs,
        composite_fonts,
        findings_seen,
        font_limitations,
        local_ids: alloc,
        candidate,
        ocr_refusal,
        recognized_outside: 0,
        recognized_under_text: 0,
    }))
}

/// For every processed page, keyed by its 1-based number ([`PageExtract::index`]), its
/// [`PageTrace`]: the index of the operation that showed each of its runs, aligned with
/// `page.runs`, and the page's counters before the fold summed them (auto-tagging S2).
///
/// The index counts operations in `lopdf::content::Content::decode` of the page's joined content
/// — the buffer `get_page_content` builds, one `\n` after every stream — which is what the
/// interpreter ran. A page the budget quarantined has no entry, because it has no runs. Several
/// runs may share one index: a `TJ` holding three strings shows three runs from one operation.
///
/// Crate-private on purpose. The writer needs to know which operator showed a run and nothing
/// parsed from JSON may reach that rule, so the mapping lives beside the artifact and never on
/// `TextRun` (docs/23-AUTO-TAGGING-SCOPE.md §6).
pub(crate) type RunPositions = std::collections::BTreeMap<u32, PageTrace>;

/// One page's side of the extraction, beside its [`PageExtract`] (auto-tagging S2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PageTrace {
    /// For every run of the page, in the artifact's order, the index of the operation that
    /// showed it — [`RunPositions`] says what the index counts.
    pub(crate) op_indices: Vec<usize>,
    /// The page's counters as `extract_page` returned them.
    pub(crate) counters: PageCounters,
    /// Runs drawn inside a form XObject: no operator of the page's shows them, so the writer has
    /// nothing to mark (`docs/30-FORM-XOBJECTS-SCOPE.md` §8).
    pub(crate) form_runs: u32,
}

/// The per-page counters docs/23-AUTO-TAGGING-SCOPE.md §3.7 names, taken from `PageYield` before
/// the fold sums them.
///
/// Each reaches the artifact only as a document total inside one limitation —
/// `inline-images-not-emitted(n)`, `xobject-name-unresolved(n)`,
/// `form-xobject-text-not-descended(n)`, `mcid-property-list-by-name(n)`,
/// `broken-font-encoding(n)` — so two pages that drift in opposite directions read back with
/// every total unchanged. Kept per page here so the writer's self-check can compare them page by
/// page, which is the comparison the scope asks for; the fold, the limitations and the artifact
/// are unchanged by this record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PageCounters {
    /// `BI … EI` images drawn on the page.
    pub(crate) inline_images: u32,
    /// `Do` operands that named no XObject this profile could resolve.
    pub(crate) unresolved_xobjects: u32,
    /// Form XObjects painted and not descended.
    pub(crate) undescended_xobjects: u32,
    /// `BDC` property lists given by name.
    pub(crate) props_by_name: u32,
    /// Runs dropped because their font could not map their codes.
    pub(crate) encoding_dropped_runs: u32,
}

impl PageCounters {
    /// Every counter beside its name, in one fixed order, so a comparison can say which differed.
    pub(crate) fn named(self) -> [(&'static str, u32); 5] {
        [
            ("inline_images", self.inline_images),
            ("unresolved_xobjects", self.unresolved_xobjects),
            ("undescended_xobjects", self.undescended_xobjects),
            ("props_by_name", self.props_by_name),
            ("encoding_dropped_runs", self.encoding_dropped_runs),
        ]
    }
}

/// Extract text runs from an already-open document.
///
/// # Errors
///
/// - [`EngineError::Unsupported`] — an operator outside PDF 32000-1 Table A.1, or a character
///   code this profile cannot decode. **Fails closed**: a skipped operator can move or delete
///   text, and a substituted character is a character the document does not contain.
/// - [`EngineError::Malformed`] — operands of the wrong shape, or an unreadable page structure; or
///   a `profile` whose `xref_repair` is not the one `doc` was opened under.
/// - [`EngineError::MissingPart`] — a font resource a `Tf` refers to is absent.
pub fn extract(doc: &Document, profile: &Profile) -> Result<ExtractArtifact, EngineError> {
    extract_with_positions(doc, profile).map(|(artifact, _)| artifact)
}

/// [`extract`] under a profile that pins a recognizer (v3, `docs/35-OCR-SCOPE.md`): every page
/// that showed no text and placed exactly one image is read by `recognizer`, and its words are
/// runs of derivation `Recognized`.
///
/// # Errors
///
/// [`extract`]'s; [`EngineError::Unsupported`] when `profile` pins no recognizer or pins one
/// other than `recognizer`, or when the recognizer answers as another; and what `recognizer`
/// returns when it fails. A page it refuses is counted on the artifact, not an error.
pub fn extract_recognized(
    doc: &Document,
    profile: &Profile,
    recognizer: &mut dyn crate::recognize::Recognize,
) -> Result<ExtractArtifact, EngineError> {
    extract_counted(
        doc,
        profile,
        &crate::budget::ContentBudget::default(),
        Some(recognizer),
        None,
    )
    .map(|(artifact, _)| artifact)
}

/// An image file's one page, read before the page existed: what [`extract_counted`] sets on it
/// and states as its source.
struct ImageFile {
    media_type: &'static str,
    sha256: Sha256Hex,
    words: Vec<crate::recognize::PageWord>,
    outside: u32,
}

/// An image file, PNG or JPEG, read under a profile that pins `recognizer` as a one-page document
/// (v3, `docs/35-OCR-SCOPE.md` §4): the recognizer reads the file whole, and its words are runs of
/// derivation `Recognized` on a page that is the image's own pixel grid, one pixel to one point.
/// The source is the file. No image node is written and no locator names an image object: the
/// page is the image.
///
/// # Errors
///
/// [`EngineError::Unsupported`] for bytes that are not a PNG or JPEG, under a profile that does not
/// pin `recognizer`, for a file whose header states no size, for an image the recognizer refuses
/// to read — an image file has no other page to count the refusal against — and for one it answers
/// at a size the file's header does not state; what `recognizer` returns when it fails.
pub fn extract_image(
    bytes: &[u8],
    profile: &Profile,
    recognizer: &mut dyn crate::recognize::Recognize,
) -> Result<ExtractArtifact, EngineError> {
    let media_type =
        crate::recognize::image_media_type(bytes).ok_or_else(|| EngineError::Unsupported {
            what: "media type".into(),
            detail: "an image file is read when it is a PNG or a JPEG, and these bytes state \
                     neither"
                .into(),
        })?;
    let pinned = pinned_recognizer(profile, Some(recognizer.identity()))?;
    // Decision #132: the page is built from the size the recognizer states, so it must be the size
    // the file's own header states, as a PDF image's must be its dictionary's — read before the
    // file is sent, and refused by its own name where the header states none.
    let (width, height) =
        crate::recognize::image_file_size(bytes).ok_or_else(|| EngineError::Unsupported {
            what: "image".into(),
            detail: format!("this {media_type}'s header states no size: `image-size-not-read`"),
        })?;
    let answer = recognizer.read(&crate::recognize::RecognitionRequest {
        ocr_request: 1,
        id: "image".into(),
        image: crate::recognize::ImageSpec::File,
        orientation: 0,
        bytes: bytes.len() as u64,
        data: bytes.to_vec(),
    })?;
    if Some(&answer.identity) != pinned.as_ref() {
        return Err(EngineError::Unsupported {
            what: "recognizer".into(),
            detail: format!(
                "the image was answered by `{} {}`, not the recognizer the profile pins",
                answer.identity.recognizer, answer.identity.version
            ),
        });
    }
    if let Some(why) = &answer.refused {
        return Err(EngineError::Unsupported {
            what: "image".into(),
            detail: format!("the recognizer did not read this {media_type}: `{why}`"),
        });
    }
    // A JPEG may state a height of 0 and set it with a DNL marker after its first scan (T.81
    // B.2.2): a height of 0 states none, and the width alone is compared.
    if width != answer.width || (height != 0 && height != answer.height) {
        return Err(EngineError::Unsupported {
            what: "image".into(),
            detail: format!(
                "the recognizer answered this {media_type} as {} x {} pixels, which its header \
                 does not state: `image-size-mismatch`",
                answer.width, answer.height
            ),
        });
    }
    let doc = Document::open_bytes(
        &crate::recognize::blank_page(answer.width, answer.height)?,
        profile,
    )?;
    let page_id = doc
        .pages()
        .first()
        .map(|&(_, id)| id)
        .ok_or_else(|| EngineError::Malformed {
            what: "image".into(),
            detail: "the page an image file is set on has no page".into(),
        })?;
    let place = crate::recognize::whole_page(page_id, answer.width, answer.height);
    let (words, outside) = crate::recognize::place(&place, &answer, None, &|_| true)?;
    let file = ImageFile {
        media_type,
        sha256: Sha256Hex::parse(format!(
            "sha256:{}",
            ethos_parser_core::c14n::sha256_hex_bytes(bytes)
        ))?,
        words,
        outside,
    };
    extract_counted(
        &doc,
        profile,
        &crate::budget::ContentBudget::default(),
        Some(recognizer),
        Some(file),
    )
    .map(|(artifact, _)| artifact)
}

/// The recognizer `profile` pins, checked against the one given (v3, `docs/35-OCR-SCOPE.md` §5):
/// an artifact whose profile names one recognizer and whose runs another read would be the
/// laundering the pin exists to prevent.
fn pinned_recognizer(
    profile: &Profile,
    given: Option<&ethos_parser_core::RecognizerIdentity>,
) -> Result<Option<ethos_parser_core::RecognizerIdentity>, EngineError> {
    match (&profile.recognizer, given) {
        (ethos_parser_core::RecognizerPin::NotPinned, None) => Ok(None),
        (ethos_parser_core::RecognizerPin::Pinned(id), Some(r)) if r == id => Ok(Some(id.clone())),
        (ethos_parser_core::RecognizerPin::Pinned(id), _) => Err(EngineError::Unsupported {
            what: "recognizer".into(),
            detail: format!(
                "the profile reads scanned pages through `{} {}` (`{}`), and {}",
                id.recognizer,
                id.version,
                id.rule,
                if given.is_some() {
                    "another recognizer answered"
                } else {
                    "none was given"
                }
            ),
        }),
        (ethos_parser_core::RecognizerPin::NotPinned, Some(_)) => Err(EngineError::Unsupported {
            what: "recognizer".into(),
            detail: "a recognizer was given to a profile that pins none".into(),
        }),
    }
}

/// [`extract`], returning beside the artifact where each run came from in its page's content
/// stream (auto-tagging S2).
///
/// The artifact is byte-for-byte the one [`extract`] returns — that function is this one with the
/// side table dropped — so nothing about the record changes when a caller asks for positions.
///
/// # Errors
///
/// Exactly [`extract`]'s.
pub(crate) fn extract_with_positions(
    doc: &Document,
    profile: &Profile,
) -> Result<(ExtractArtifact, RunPositions), EngineError> {
    extract_counted(
        doc,
        profile,
        &crate::budget::ContentBudget::default(),
        None,
        None,
    )
}

/// [`extract_with_positions`], charging `content` for the pages it reads: one budget per reading,
/// which a test can hand in already spent. `image_file` is set by [`extract_image`] alone.
fn extract_counted(
    doc: &Document,
    profile: &Profile,
    content: &crate::budget::ContentBudget,
    recognizer: Option<&mut dyn crate::recognize::Recognize>,
    image_file: Option<ImageFile>,
) -> Result<(ExtractArtifact, RunPositions), EngineError> {
    let pinned = pinned_recognizer(profile, recognizer.as_ref().map(|r| r.identity()))?;
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "profile".into(),
            detail: e.to_string(),
        })?;
    // The open decided whether the cross-reference repair could run, so an artifact must not
    // name a profile that would have refused the document it describes.
    if doc.opened_under() != profile.xref_repair {
        return Err(EngineError::Malformed {
            what: "profile".into(),
            detail: "the document was opened under a different `xref_repair` than this profile \
                     names; open and extract under one profile"
                .into(),
        });
    }

    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let mut pages = Vec::with_capacity(doc.pages().len());
    // Decision #29. Every page's candidate heading lines, aligned with `pages`, and the
    // document's rendered ems merged across pages — from which the body em is read once the
    // whole document has been.
    let mut heading_lines_by_page: Vec<HeadingLines> = Vec::new();
    let mut doc_em_tally = crate::headings::EmTally::default();
    let mut positions = RunPositions::new();
    let mut limitations = lim::extract_limitations();
    // v3: what a recognized run does not carry, on every artifact made under the OCR profile.
    if pinned.is_some() {
        limitations.extend(lim::recognized_text_limitations());
    }
    // A repaired open is never silent: every artifact derived from one says so.
    if let Some(padded) = doc.xref_entries_padded() {
        limitations.push(lim::xref_entry_padded(padded));
    }
    // Neither is an open the empty user password made possible (decision #31).
    if doc.opened_encrypted() {
        limitations.push(lim::encrypted_empty_user_password());
    }
    let mut page_states: Vec<PageStateEntry> = Vec::with_capacity(doc.pages().len());
    // Accumulated across pages: how much text is missing from this artifact because a font's
    // encoding could not map it, and the first failure's reason for the declaration's detail.
    let mut encoding_dropped_runs: u32 = 0;
    let mut font_read_runs: u32 = 0;
    let mut encoding_detail = String::new();
    // v1-S2. Pages where the alignment rule built a candidate lattice and refused it, with the
    // precondition that failed. Collected rather than declared per page so the artifact carries
    // one limitation naming every such page instead of one per page.
    let mut unruled_refusals: Vec<(u32, crate::unruled::Refusal)> = Vec::new();
    // v1-S7b. The same, for the ruled rule. It had no voice until `ruled-rects-v2` made its
    // coherence precondition a live path — under `-v1` a background panel satisfied coverage for
    // every face at once, so the check almost never fired.
    let mut ruled_refusals: Vec<(u32, crate::tables::RuledRefusal)> = Vec::new();
    // v1-S8. The same again, for the stroke-ruled rule. It is a live path on ordinary documents —
    // any page whose rules happen to end at common x positions builds a band and most of them are
    // refused — so without this a page where the ink implied a grid the author never divided reads
    // exactly like a page that drew no lines at all.
    let mut stroke_refusals: Vec<(u32, crate::stroke_ruled::Refusal)> = Vec::new();

    // v1-S3. Read the document's own structure tree ONCE, off the same handle every other stage
    // borrows (`docs/04-ARCHITECTURE.md` §2.1). `None` means the catalog declares no
    // `/StructTreeRoot` — an untagged document, which is an answer rather than a failure.
    let structure = crate::structure::read(doc.inner())?;
    // `outlines-v3`. The other declaration this document may carry, read here for the same
    // reason the tree is: it is the author's statement, not an inference over the page, and a
    // cycling chain is refused by name rather than followed or truncated.
    let outline = crate::outlines::read(doc)?;
    // The tree's citations grouped by page, built once: the per-page loop below
    // consults only its own page's keys, where iterating `tree.keys()` per page made
    // the reconciliation O(pages × total keys) across the document.
    let tree_mcids_by_page: std::collections::BTreeMap<lopdf::ObjectId, Vec<i64>> = structure
        .as_ref()
        .map(|tree| {
            let mut by_page: std::collections::BTreeMap<lopdf::ObjectId, Vec<i64>> =
                std::collections::BTreeMap::new();
            for &(page, mcid) in tree.keys() {
                by_page.entry(page).or_default().push(mcid);
            }
            by_page
        })
        .unwrap_or_default();
    // Counted while binding, declared afterwards, and only when non-zero.
    let mut mcids_unbound: u32 = 0;
    let mut unclaimed_tree_items: u32 = 0;
    // Auto-tagging S1. Runs bound under this engine's own elements, summed like `mcids_unbound`.
    let mut computed_bound: u32 = 0;
    let mut props_by_name: u32 = 0;
    let mut alternate_texts: u32 = 0;
    let mut partial_code_runs: u32 = 0;
    let mut tagged_without_geometric: Vec<u32> = Vec::new();
    // v1-S4. Widgets whose `/Parent` chain did not resolve. Counted, declared, never repaired.
    let mut unresolved_field_parents: u32 = 0;
    let mut field_states_not_determinable: u32 = 0;
    // Field values and annotation `/Contents` this engine will not decode (review 2026-09-26 N39).
    let mut undecodable_object_texts: u32 = 0;
    // v1-S6. Counted across pages, declared once, never repaired and never silently skipped.
    let mut inline_images: u32 = 0;
    let mut unresolved_xobjects: u32 = 0;
    let mut undescended_xobjects: u32 = 0;
    let mut recognized_outside: u32 = 0;
    let mut recognized_under_text: u32 = 0;
    let mut form_mcids: u32 = 0;
    let mut findings_seen: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();
    // v1-S6.1. Composite fonts whose code width came from `/ToUnicode` rather than from the
    // `/Encoding` CMap this profile does not parse. Counted by resource name per page, which
    // over-counts a font shared across pages — the declaration says "font(s) on this document",
    // and the number is a scale, not an inventory.
    let mut composite_fonts: u32 = 0;

    let budget = profile.page_budget;
    let page_count = doc.page_count();

    // Pages in parallel, folded in order (0.39.0). Each admitted page runs the same
    // body it always ran — `extract_page` — against the shared read-only handle, with
    // a page-local id allocator. The fold below then walks the results in page order:
    // it rewrites every id through one document-global allocator (per-kind counters
    // are independent and `reorder_page` already lays run ids contiguous per page, so
    // the rewritten sequence is byte-identical to the sequential one), folds each
    // page's counter deltas with the same saturating arithmetic in the same order,
    // and returns the FIRST page error in page order — the sequential loop's abort
    // point — so a failing document reports the same failure it always did.
    //
    // # Chunking this was measured and refused (v2-S15)
    //
    // A security review read this `collect()` as the tree's largest memory regression: it holds
    // every page's yield at once where the pre-0.39.0 sequential loop streamed, so peak was said
    // to scale with page count without bound. The first half is right and the conclusion does not
    // follow, which is why the fix was built, measured, and then thrown away rather than shipped.
    //
    // Folding in batches of 32 — same order, same first-error-in-page-order, byte-identical over
    // all 212 artifacts — moved peak RSS on `nist-sp-800-171r3` (120 pages) from 558.9 MiB to
    // 563.6 MiB. Nothing, and slightly the wrong way.
    //
    // The reason is three lines below the fold: `pages.push(y.page)`. Every page's `PageExtract`
    // is RETAINED, because it is the artifact — `pages` is returned at the bottom of this
    // function. `PageYield` is that same `PageExtract` plus a dozen counters and three small
    // refusal vectors, so bounding how many `PageYield`s are live bounds only the counters. The
    // pages themselves are required output and no batching can drop them.
    //
    // What IS true, measured across the corpus at v2-S15:
    //
    //     pages   file     peak RSS
    //         6   138K     17.6 MiB
    //        28   1.5M    124.2 MiB
    //        36   723K    219.1 MiB
    //        59   944K    272.6 MiB
    //        80   1.4M    411.6 MiB
    //       120   1.5M    566.1 MiB
    //
    // ~4.7 MiB per page, and independent of file size — the two 1.5M documents differ 4.5x on
    // page count alone. So the exposure is real and it is the ARTIFACT, not the parallelism. The
    // bound that helps is therefore a ceiling on how many pages one extract will admit, which is
    // what `--max-pages` and `PageBudget` are for, and not a smaller window onto the same
    // retained set. Do not re-propose the batching without re-measuring: it costs complexity in
    // the hottest loop here and bought 0 MiB.
    //
    // # Re-measured at 97fa562, and the per-page figure is a range rather than a constant
    //
    // "~4.7 MiB per page" above is the median of a spread. Measured pre-Arc it was 3.20 to 9.07
    // MiB/page with the 733-page document at 6.5 GiB; after the role-path sharing below it was
    // 2.98 to 6.89 at 4.56 GiB, and since c14n stopped copying the payload's largest field it is
    // 3.01 to 6.58, with that document at 3.65 GiB — 3.01 to 6.75 and 3.64 GiB re-measured at
    // 0.58.0, within noise of the last figure. Within ONE document the marginal cost per
    // admitted page IS constant — a pre-Arc `--max-pages` ladder gives 8.4 to 9.2 MiB/page from
    // 128 pages up, and the two post-Arc points (128 and 733) give 5.95 — so there is no
    // superlinear retention here and the cross-document spread is content density.
    //
    // The 30% that came off was not the parallelism and not the artifact buffer: it was the tagged
    // role path, deep-cloned once per run at `bind_structure` and again per node, ~21.9M
    // `Vec<String>` elements for THIRTY distinct paths. It is shared behind an `Arc` now. The
    // paragraph above is still right that the pages are the artifact and cannot be dropped; it was
    // wrong that what they hold is irreducible.
    //
    // What the paragraph above does not say, and what the ladder exposed: `--max-pages 0` on the
    // 733-page document still costs 221 MiB, and role-path sharing did not move it — a document
    // admitting no pages has no run whose path could be shared, which is a check on the mechanism
    // rather than a repeat reading. The budget is read at the `let budget` below, but
    // `structure::read` and `tree_mcids_by_page` are built above it over the WHOLE document, so
    // that term never responds to the flag. It is not most of the floor, though: measured at
    // 0.58.0 against `classify --sample-pages 0`, which opens the document and reads no tree,
    // the tree is 28 MiB of the 221, and the other 193 MiB is the source bytes and lopdf's
    // object graph, built in `Document::open_bytes` before this function runs — a cost no budget
    // consulted here can lower. See `docs/measurements/memory-ceiling/` §5 and §15.
    use rayon::prelude::*;
    let read = |page_number, page_id, limit, words: Option<&[crate::recognize::PageWord]>| {
        extract_page(
            doc,
            profile,
            &profile_sha256,
            &structure,
            &tree_mcids_by_page,
            page_number,
            page_id,
            content,
            limit,
            words,
        )
    };
    // A page denser than `HEAVY_PAGE_OPERATIONS` is left by the parallel read and read below, alone
    // and in page order, so dense pages cost one at a time whatever the core count (review
    // 2026-09-25 F08). The output does not depend on which pass read a page.
    let outcomes: Vec<(u32, Option<Result<PageYield, EngineError>>)> = doc
        .pages()
        .par_iter()
        .map(|&(page_number, page_id)| {
            if !budget.admits(page_number) {
                return (page_number, page_id, None);
            }
            let outcome = read(
                page_number,
                page_id,
                crate::budget::HEAVY_PAGE_OPERATIONS,
                None,
            );
            (page_number, page_id, Some(outcome))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(page_number, page_id, outcome)| {
            let outcome = outcome.map(|result| match result {
                Ok(Some(page)) => Ok(page),
                Ok(None) => read(
                    page_number,
                    page_id,
                    crate::budget::MAX_PAGE_OPERATIONS,
                    None,
                )?
                .ok_or_else(|| crate::budget::page_operations_past_ceiling(page_number)),
                Err(e) => Err(e),
            });
            (page_number, outcome)
        })
        .collect();

    // Ahead of every page's own error: the pages ran in parallel, so which of them crossed the
    // document's `ToUnicode` budget is not a property of the document, and an earlier page's
    // error would otherwise win or lose by timing (review 2026-09-26 N11). The content budget
    // is the same kind of refusal, for the same reason.
    doc.tounicode().refusal()?;
    content.refusal()?;

    // v3 (`docs/35-OCR-SCOPE.md` §5): every page the trigger took, asked of the recognizer in page
    // order — one at a time, because it uses every core itself — and read again with its words.
    let mut outcomes = outcomes;
    if let (Some(recognizer), Some(pinned)) = (recognizer, &pinned) {
        for (page_number, outcome) in &mut outcomes {
            let Some(Ok(y)) = outcome else {
                continue;
            };
            let Some(candidate) = y.candidate.take() else {
                continue;
            };
            let request = match crate::recognize::request(doc.inner(), &candidate, *page_number) {
                Ok(r) => r,
                Err(why) => {
                    y.ocr_refusal = Some(why);
                    continue;
                }
            };
            let answer = recognizer.read(&request)?;
            if &answer.identity != pinned {
                return Err(EngineError::Unsupported {
                    what: "recognizer".into(),
                    detail: format!(
                        "page {page_number} was answered by `{} {}`, not the pinned `{} {}`",
                        answer.identity.recognizer,
                        answer.identity.version,
                        pinned.recognizer,
                        pinned.version
                    ),
                });
            }
            if let Some(why) = answer.refused.clone() {
                y.ocr_refusal = Some(why);
                continue;
            }
            let size = match &request.image {
                crate::recognize::ImageSpec::Pdf(spec) => Some((spec.width, spec.height)),
                crate::recognize::ImageSpec::Composite(spec) => Some((spec.width, spec.height)),
                crate::recognize::ImageSpec::File => None,
            };
            if size.is_some_and(|s| s != (answer.width, answer.height)) {
                y.ocr_refusal = Some("image-size-mismatch".into());
                continue;
            }
            // A composed page's words were read from no one image (decision #156).
            let image =
                candidate
                    .composite
                    .is_none()
                    .then_some(ethos_parser_core::PdfObjectNumber {
                        object: candidate.object.0,
                        generation: u32::from(candidate.object.1),
                    });
            let (words, outside, under) = {
                let clear = clear_of_drawn(&y.page);
                let (words, outside) = crate::recognize::place(&candidate, &answer, image, &clear)?;
                let (words, under) = beside_drawn(words, &clear);
                (words, outside, under)
            };
            if !words.is_empty() {
                let again = read(
                    *page_number,
                    candidate.page_id,
                    crate::budget::MAX_PAGE_OPERATIONS,
                    Some(&words),
                )?
                .ok_or_else(|| crate::budget::page_operations_past_ceiling(*page_number))?;
                *y = again;
            }
            y.recognized_outside = declare(y.recognized_outside, outside);
            y.recognized_under_text = declare(y.recognized_under_text, under);
        }
    } else {
        // Decision #132: with no recognizer, a page the trigger took is counted as `--ocr` would
        // leave it — by the reason the request refuses it for, or as not requested — so a scan
        // read without the flag is not a blank page, and the flag is offered only where it reads.
        for (page_number, outcome) in &mut outcomes {
            if let Some(Ok(y)) = outcome {
                if let Some(candidate) = y.candidate.take() {
                    y.ocr_refusal = Some(
                        crate::recognize::request(doc.inner(), &candidate, *page_number)
                            .map_or_else(|why| why, |_| "ocr-not-requested".into()),
                    );
                }
            }
        }
    }
    // An image file's one page, which was read before it was made ([`extract_image`]).
    if let Some(file) = &image_file {
        if let (Some((page_number, Some(Ok(y)))), Some(&(_, page_id))) =
            (outcomes.first_mut(), doc.pages().first())
        {
            if !file.words.is_empty() {
                *y = read(
                    *page_number,
                    page_id,
                    crate::budget::MAX_PAGE_OPERATIONS,
                    Some(&file.words),
                )?
                .ok_or_else(|| crate::budget::page_operations_past_ceiling(*page_number))?;
            }
            y.recognized_outside = declare(y.recognized_outside, file.outside);
        }
    }

    for (page_number, outcome) in outcomes {
        let Some(result) = outcome else {
            page_states.push(PageStateEntry {
                index: page_number,
                state: PageState::Quarantined(
                    ethos_parser_core::codes::RESOURCE_LIMIT_PAGES.to_string(),
                ),
            });
            continue;
        };
        let mut y = result?;

        // Rebase the page-local ids onto the document-global sequence. An id is
        // prefix + ordinal; adding the global base to each local ordinal reproduces
        // the sequential numbering EXACTLY — holes included, because a refused
        // candidate consumed an ordinal it never shipped and the artifact keeps
        // that hole. The global counters then advance by replaying the same number
        // of allocations the page made, which also reproduces the sequential
        // MAX_SAFE_INT refusal at the same page it would always have fired.
        // Five counters read once, then indexed by a `match` rather than searched (v2-S15).
        //
        // This was an array of `(kind, base)` pairs behind `find(..).expect("kind is listed")`:
        // a linear scan per id — five comparisons each, ~5x10^5 on a page with 10^5 runs — to
        // read a value already in scope.
        //
        // The `expect` it replaced was **not** unreachable, and the compiler is what said so.
        // `IdKind` has eight variants; this path mints five. `Page`, `Element` and `Part` were
        // never in that array, so `expect("kind is listed")` was the branch they took — a message
        // asserting an invariant that three of the eight cases violate by construction. Writing
        // it as a `match` forced them into the open. They still cannot be rebased here (there is
        // no page-local counter for them to be rebased against), so they still panic — but the
        // panic now names the real condition instead of claiming the kind was not listed.
        let [span_base, table_base, image_base, field_base, annot_base] = [
            IdKind::Span,
            IdKind::Table,
            IdKind::Image,
            IdKind::FormField,
            IdKind::Annotation,
        ]
        .map(|kind| alloc.count(kind));
        let base = |kind: IdKind| match kind {
            IdKind::Span => span_base,
            IdKind::Table => table_base,
            IdKind::Image => image_base,
            IdKind::FormField => field_base,
            IdKind::Annotation => annot_base,
            // Not minted per page, so there is no page-local ordinal to rebase. Enumerated rather
            // than swept into a `_` arm: a sixth rebased kind added later must fail HERE, at the
            // counter, rather than silently taking a zero base and colliding every id it rewrites.
            IdKind::Page | IdKind::Element | IdKind::Part | IdKind::Proposal => panic!(
                "{kind:?} is not rebased on the page path: extraction mints no page-local ids of \
                 this kind, so there is no base to add. A new rebased kind needs a counter above."
            ),
        };
        // `strip_prefix` rather than `as_str()[kind.prefix().len()..]`. The byte slice assumes the
        // id actually carries the prefix for the kind it is being rebased as, and one caller below
        // does NOT derive `kind` from the id — it reads `object.attributes`, so a FormField
        // attribute on an Annotation-minted id would slice at the wrong offset and then either
        // parse the wrong number or panic inside `str` indexing. This makes the mismatch itself
        // the reported condition instead of its downstream symptom.
        //
        // Both remaining panics are engine-invariant violations rather than document properties,
        // and they stay panics deliberately: `EngineError` has no variant that means "this engine
        // is wrong" — routing them through `Malformed` would blame the document for a bug in here.
        // Widening the taxonomy is a decision for its own change; see the v2-S15 audit note.
        let rebase = |id: &ethos_parser_core::NodeId, kind: IdKind| {
            let rest = id.as_str().strip_prefix(kind.prefix()).unwrap_or_else(|| {
                panic!(
                    "id `{}` is being rebased as {kind:?}, whose prefix is `{}`, and does not \
                     carry it — the kind was derived from the wrong place",
                    id.as_str(),
                    kind.prefix()
                )
            });
            let ordinal: u64 = rest.parse().unwrap_or_else(|_| {
                panic!(
                    "engine-minted id `{}` carries `{rest}` after its prefix, which is not a \
                     decimal ordinal",
                    id.as_str()
                )
            });
            ethos_parser_core::NodeId::from_parts(kind, base(kind) + ordinal)
        };
        for run in &mut y.page.runs {
            run.id = rebase(&run.id, IdKind::Span);
            if let Some(beside) = &run.read_beside {
                run.read_beside = Some(rebase(beside, IdKind::Span));
            }
        }
        // Decision #168.
        for b in &mut y.page.drawn_boxes {
            b.beside = rebase(&b.beside, IdKind::Span);
        }
        for table in &mut y.page.tables {
            let id = rebase(&table.id, IdKind::Table);
            for cell in &mut table.cells {
                cell.position.table_id = id.clone();
            }
            table.id = id;
        }
        for table in &mut y.page.tagged_tables {
            let id = rebase(&table.id, IdKind::Table);
            for cell in &mut table.cells {
                cell.position.table_id = id.clone();
            }
            table.id = id;
        }
        for image in &mut y.page.images {
            image.id = rebase(&image.id, IdKind::Image);
        }
        for object in &mut y.page.objects {
            object.id = rebase(
                &object.id,
                match object.attributes {
                    ethos_parser_core::NodeAttributes::FormField(_) => IdKind::FormField,
                    _ => IdKind::Annotation,
                },
            );
        }
        for kind in [
            IdKind::Span,
            IdKind::Table,
            IdKind::Image,
            IdKind::FormField,
            IdKind::Annotation,
        ] {
            alloc.advance(kind, y.local_ids.count(kind))?;
        }

        ruled_refusals.extend(y.ruled_refusals);
        stroke_refusals.extend(y.stroke_refusals);
        unruled_refusals.extend(y.unruled_refusals);
        if y.encoding_dropped_runs > 0 {
            if encoding_dropped_runs == 0 {
                encoding_detail = y.encoding_detail;
            }
            encoding_dropped_runs = declare(encoding_dropped_runs, y.encoding_dropped_runs);
        }
        font_read_runs = declare(font_read_runs, y.font_read_runs);
        mcids_unbound = declare(mcids_unbound, y.mcids_unbound);
        unclaimed_tree_items = declare(unclaimed_tree_items, y.unclaimed_tree_items);
        computed_bound = declare(computed_bound, y.computed_bound);
        props_by_name = declare(props_by_name, y.props_by_name);
        alternate_texts = declare(alternate_texts, y.alternate_texts);
        partial_code_runs = declare(partial_code_runs, y.partial_code_runs);
        tagged_without_geometric.extend(y.tagged_without_geometric);
        unresolved_field_parents = declare(unresolved_field_parents, y.unresolved_field_parents);
        field_states_not_determinable = declare(
            field_states_not_determinable,
            y.field_states_not_determinable,
        );
        undecodable_object_texts = declare(undecodable_object_texts, y.undecodable_object_texts);
        inline_images = declare(inline_images, y.inline_images);
        unresolved_xobjects = declare(unresolved_xobjects, y.unresolved_xobjects);
        undescended_xobjects = declare(undescended_xobjects, y.undescended_xobjects);
        recognized_outside = declare(recognized_outside, y.recognized_outside);
        recognized_under_text = declare(recognized_under_text, y.recognized_under_text);
        if let Some(why) = &y.ocr_refusal {
            limitations.push(lim::scanned_page_not_recognized(page_number, why));
        }
        form_mcids = declare(form_mcids, y.form_mcids);
        if y.undescended_xobjects > 0 {
            limitations.push(lim::form_xobjects_not_descended_on_page(
                page_number,
                y.undescended_xobjects,
            ));
        }
        composite_fonts = declare(composite_fonts, y.composite_fonts);
        for (code, n) in y.findings_seen {
            *findings_seen.entry(code).or_insert(0) += n;
        }
        for entry in y.font_limitations {
            if !limitations.contains(&entry) {
                limitations.push(entry);
            }
        }

        // The page's counters, kept beside its operator indices before the sums above lose the
        // page (auto-tagging S2, scope §3.7's per-page comparison).
        positions.insert(
            page_number,
            PageTrace {
                op_indices: y.op_indices,
                counters: PageCounters {
                    inline_images: y.inline_images,
                    unresolved_xobjects: y.unresolved_xobjects,
                    undescended_xobjects: y.undescended_xobjects,
                    props_by_name: y.props_by_name,
                    encoding_dropped_runs: y.encoding_dropped_runs,
                },
                form_runs: y.form_runs,
            },
        );
        pages.push(y.page);
        heading_lines_by_page.push(y.heading_lines);
        doc_em_tally.merge(&y.em_tally);
        page_states.push(PageStateEntry {
            index: page_number,
            state: PageState::Processed,
        });
    }

    if let Some(b) = budget.max_pages_to_process() {
        if b < page_count {
            limitations.push(lim::resource_limit_pages(b, page_count));
        }
    }

    // v1-S2. Only when a candidate was actually built and refused — a page whose text implied
    // nothing grid-shaped produced no candidate and gets no declaration, because declaring a
    // refusal that did not happen is as misleading as omitting one that did.
    if !ruled_refusals.is_empty() {
        limitations.push(lim::ruled_candidate_refused(&ruled_refusals));
    }
    if !stroke_refusals.is_empty() {
        limitations.push(lim::stroke_ruled_candidate_refused(&stroke_refusals));
    }
    if !unruled_refusals.is_empty() {
        limitations.push(lim::unruled_candidate_refused(&unruled_refusals));
    }

    // v1-S3. Four facts about the structure tree, each declared only where it is true. A
    // capability that says "this profile looks" is worth having only if the artifact also says
    // what the looking found, and "found nothing" has more than one cause.
    //
    // Auto-tagging S1 adds a fifth, on the `Some` arm only: a tree this engine's own writer
    // created is declared as such, and `untagged-structure-tree-absent` stays on the `None` arm
    // alone — a tree was read, so its detail would be false (scope §4.2).
    match structure.as_ref() {
        None => limitations.push(lim::untagged_structure_tree_absent()),
        Some(tree) => {
            if mcids_unbound > 0 {
                limitations.push(lim::structure_mcid_unbound(mcids_unbound));
            }
            if unclaimed_tree_items > 0 {
                limitations.push(lim::structure_item_without_content(unclaimed_tree_items));
            }
            if let Some(written) = &tree.engine_written {
                limitations.push(lim::structure_tree_engine_written(
                    written.elements,
                    declared_len(tree.elements),
                    computed_bound,
                    &written.rules,
                ));
            }
        }
    }
    // Decision #29. The body em exists only now — it is a mode over every page — so this is where
    // each page's reduced lines are decided. Every page shares one gate, so a document the gate
    // closed returned no tally from any page, has no body em, and nothing below runs.
    //
    // Decision #38: `type-size-v3` ranks the sizes the headings are set in, so the ranks are read
    // over every page's lines first, and only then is each line given its level.
    let candidates = heading_lines_by_page.iter().flatten().map(|(_, line)| line);
    // Each line the rule read: its page, a run of it, and whether it is a label (2), bold alone
    // (1) or sized (0) — counted once units are read, which may read a lede as text (decision #77).
    let mut read_lines: Vec<(usize, usize, u8)> = Vec::new();
    let mut body_em = None;
    if let Some(levels) = crate::headings::Levels::new(candidates, &doc_em_tally) {
        for (p, (page, lines)) in pages.iter_mut().zip(&heading_lines_by_page).enumerate() {
            for (indices, line) in lines {
                let Some(level) = levels.of(*line) else {
                    continue;
                };
                let kind = if level == crate::headings::LABEL_LEVEL {
                    2
                } else {
                    u8::from(!line.is_heading(levels.body_em()))
                };
                for &i in indices {
                    page.runs[i].inferred_heading = true;
                    page.runs[i].inferred_heading_level = (level > 1).then_some(level);
                }
                if let Some(&i) = indices.first() {
                    read_lines.push((p, i, kind));
                }
            }
        }
        // `type-size-v8` (decision #140): where the document declares no author structure and
        // none of its lines ranks, labels that head its sections are level 1. A label heads a
        // section where the last inked run before it is not bold, so it opens no line of a bold
        // passage, and the two lines after it are text in no bold, so it is neither stacked over
        // another bold line nor a caption over one line of value; and its sections recur: two
        // such labels or more, set in one font at one size. A form's field, a resume's contact
        // label, a page's one bold line stay labels.
        //
        // Decision #159: where no style recurs, the document's first label that heads a section
        // is level 1 alone, if it heads a paragraph: the text after it opens as a sentence does —
        // not in a lower-case letter, which a bold phrase inside a paragraph runs on in — and its
        // next line follows that text no farther below than that text follows the label, so the
        // two are one paragraph, not a value over the text after it; and the label is no caption
        // ([`names_a_figure`]).
        if no_author_structure(structure.as_ref())
            && read_lines.iter().all(|&(_, _, kind)| kind == 2)
        {
            let inked = |r: &&TextRun| !r.text.trim().is_empty();
            // A label heading a section: its page, its runs, its style, and whether it may head
            // the document alone (decision #159).
            type Head<'a> = (usize, &'a Vec<usize>, (String, i64), bool);
            let mut heads: Vec<Head<'_>> = Vec::new();
            for (p, (page, lines)) in pages.iter().zip(&heading_lines_by_page).enumerate() {
                for (indices, _) in lines {
                    let (Some(&first), Some(&last)) = (indices.first(), indices.last()) else {
                        continue;
                    };
                    let Some(end) = indices.iter().rev().map(|&i| &page.runs[i]).find(inked) else {
                        continue;
                    };
                    if page.runs[first].inferred_heading_level != Some(crate::headings::LABEL_LEVEL)
                    {
                        continue;
                    }
                    // A run-in label — `Note:` and the text it opens, set on along its line a
                    // little off its baseline — heads nothing: the next run starts past its ink,
                    // within one of its glyphs of its baseline. No width, no verdict.
                    let runs_on = |r: &TextRun| {
                        let chars = i64::try_from(end.text.chars().count()).unwrap_or(1);
                        end.locator.advance.is_none_or(|w| {
                            let glyph = w / chars.max(1);
                            r.locator.origin_x >= end.locator.origin_x + w - glyph
                                && (r.locator.origin_y - end.locator.origin_y).abs() < glyph
                        })
                    };
                    let before = page.runs[..first].iter().rev().find(inked);
                    let mut after = page.runs[last + 1..].iter().filter(inked);
                    let next = after.next();
                    let below = next.and_then(|n| {
                        let y = n.locator.origin_y;
                        after.find(|r| r.bold || r.locator.origin_y != y)
                    });
                    if !before.is_some_and(|r| r.bold)
                        && next.is_some_and(|r| !r.bold && !runs_on(r))
                        && below.is_some_and(|r| !r.bold)
                    {
                        let opens = next
                            .is_some_and(|r| !r.text.trim_start().starts_with(char::is_lowercase));
                        // The line after the text, in reading order: the first run off its line by
                        // half the label's gap or more. It must stand below the text, no farther
                        // than the text from the label (review: a raised mark on the text's own
                        // line, or the next column's top, passed as the line below).
                        let paragraph = next.is_some_and(|n| {
                            let gap = n.locator.origin_y - end.locator.origin_y;
                            let line =
                                page.runs[last + 1..]
                                    .iter()
                                    .filter(inked)
                                    .skip(1)
                                    .find(|r| {
                                        2 * (r.locator.origin_y - n.locator.origin_y).abs() >= gap
                                    });
                            gap > 0
                                && line.is_some_and(|r| {
                                    let down = r.locator.origin_y - n.locator.origin_y;
                                    down > 0 && down <= gap
                                })
                        });
                        // Run together, as a page drawing a glyph to a run needs, and a space
                        // apart, as one drawing a word to a run needs (third review: `Table` and
                        // `A.2` ran into `TableA.2`).
                        let words: Vec<&str> = indices
                            .iter()
                            .map(|&i| page.runs[i].text.as_str())
                            .collect();
                        let caption =
                            names_a_figure(&words.concat()) || names_a_figure(&words.join(" "));
                        let alone = opens && paragraph && !caption;
                        heads.push((p, indices, (end.font_id.clone(), end.font_size), alone));
                    }
                }
            }
            // How many labels each style heads, counted once rather than once per label (review).
            // A style is a font resource name and a size, and the name is its page's own: two pages
            // naming different fonts `F1` at one size read as one style (known limit, review).
            let mut styles: std::collections::BTreeMap<&(String, i64), usize> = Default::default();
            for (_, _, style, _) in &heads {
                *styles.entry(style).or_default() += 1;
            }
            let lone = if styles.values().all(|&n| n < 2) {
                heads.iter().position(|&(_, _, _, alone)| alone)
            } else {
                None
            };
            for (k, (p, indices, style, _)) in heads.iter().enumerate() {
                if styles[style] >= 2 || lone == Some(k) {
                    for &i in *indices {
                        pages[*p].runs[i].inferred_heading_level = None;
                    }
                }
            }
        }
        body_em = Some(levels.body_em());
    }

    // Decision #38. Layout units, after headings — a heading line never shares a unit with body
    // text — and only where the document declares no author structure, whose own structure wins
    // there as it does for headings.
    if profile.layout_unit_rule == ethos_parser_core::LAYOUT_UNIT_RULE_V7
        && no_author_structure(structure.as_ref())
    {
        // Each page's own runs only, so pages are read side by side.
        pages.par_iter_mut().for_each(crate::units::assign);
    }
    if let Some(body_em) = body_em {
        let mut fired: u32 = 0;
        let mut bold_headings: u32 = 0;
        let mut labels: u32 = 0;
        let mut as_text: u32 = 0;
        for &(p, i, kind) in &read_lines {
            if !pages[p].runs[i].inferred_heading {
                as_text = as_text.saturating_add(1);
                continue;
            }
            fired = fired.saturating_add(1);
            match kind {
                2 => labels = labels.saturating_add(1),
                1 => bold_headings = bold_headings.saturating_add(1),
                _ => {}
            }
        }
        if fired > 0 {
            limitations.push(lim::headings_inferred_from_type(
                fired,
                bold_headings,
                labels,
                as_text,
                &profile.heading_inference_rule,
                body_em,
                !no_author_structure(structure.as_ref()),
            ));
        }
    }

    // Decision #41. Page furniture, where the document declares no author structure, for the
    // reason units run only there.
    if profile.furniture_rule == ethos_parser_core::FURNITURE_RULE_V1
        && no_author_structure(structure.as_ref())
    {
        // Each page's own runs only, so pages are read side by side.
        pages.par_iter_mut().for_each(crate::furniture::assign);
    }

    // Right-to-left text, declared where it occurs (`OPEN-WORK.md` §4, decided 2026-09-23).
    //
    // Counted here, at document level over the runs already built, for the reason the block above
    // is here: nothing per-page needs to know. No counter is threaded through the page struct.
    //
    // **The condition is measurable, so the scope is the document's and not the profile's.**
    // `low-contrast-not-detected` and `document-metadata-not-read` ride every artifact because
    // deciding whether they APPLY would mean reading what this profile never reads. Here the text
    // is already in hand, so a document that draws no right-to-left scalar says nothing — which
    // is what makes the declaration worth reading when it does appear.
    //
    // A `U+FFFD` in a run is counted over the same runs (review 2026-09-26 N21). This engine never
    // puts one there, so each is a character a font of the document maps a code to: read as
    // written, and declared, because nothing downstream can otherwise tell it from a substitution.
    {
        let mut rtl_runs: u32 = 0;
        let mut replacement_runs: u32 = 0;
        for page in &pages {
            for run in &page.runs {
                // Decision #54: a run the right-to-left rule read carries its reading, so only one
                // it left as drawn counts.
                if run.reading.is_none() && run.text.chars().any(is_right_to_left_block) {
                    rtl_runs = rtl_runs.saturating_add(1);
                }
                if run.text.contains('\u{FFFD}') {
                    replacement_runs = replacement_runs.saturating_add(1);
                }
            }
        }
        if rtl_runs > 0 {
            limitations.push(lim::right_to_left_not_reordered(rtl_runs));
        }
        if replacement_runs > 0 {
            limitations.push(lim::replacement_character_in_text(replacement_runs));
        }
    }
    if props_by_name > 0 {
        limitations.push(lim::mcid_property_list_by_name(props_by_name));
    }
    // Review 2026-09-26 N22: the document's own text for what it marks, in a marked-content
    // property list or on a structure element, none of it read.
    let alternate_elements = structure.as_ref().map_or(0, |t| t.alternate_texts);
    if alternate_texts > 0 || alternate_elements > 0 {
        limitations.push(lim::actual_text_not_read(
            alternate_texts,
            declared_len(alternate_elements),
        ));
    }
    if partial_code_runs > 0 {
        limitations.push(lim::string_ends_mid_code(partial_code_runs));
    }
    if !tagged_without_geometric.is_empty() {
        limitations.push(lim::tagged_table_without_geometric_table(
            &tagged_without_geometric,
        ));
    }

    // v1-S4. An XFA packet is DETECTED and declared, never parsed (checklist L15). Any static
    // AcroForm fields beside it are still read, which is why this is a limitation rather than a
    // refusal — but a sparse field set on such a document must not read as "this form is blank".
    if (profile.capabilities.form_fields || profile.capabilities.annotations)
        && crate::forms::has_xfa(doc.inner())
    {
        limitations.push(lim::xfa_forms_not_extracted());
    }
    if unresolved_field_parents > 0 {
        limitations.push(lim::form_field_parent_unresolved(unresolved_field_parents));
    }
    if field_states_not_determinable > 0 {
        limitations.push(lim::form_field_state_not_determinable(
            field_states_not_determinable,
        ));
    }
    if undecodable_object_texts > 0 {
        limitations.push(lim::form_annotation_text_undecodable(
            undecodable_object_texts,
        ));
    }

    // v1-S6. Findings are counted where they were observed and declared once, so a consumer
    // reading only the assurance block learns they exist. **Every counted run is still in the
    // artifact** — this is a summary of what is there, never a record of what was removed.
    for (code, count) in &findings_seen {
        limitations.push(lim::text_finding(code, *count));
    }
    if inline_images > 0 {
        limitations.push(lim::inline_images_not_emitted(inline_images));
    }
    if unresolved_xobjects > 0 {
        limitations.push(lim::xobject_name_unresolved(unresolved_xobjects));
    }
    if undescended_xobjects > 0 {
        limitations.push(lim::form_xobjects_not_descended(undescended_xobjects));
    }
    if recognized_outside > 0 {
        limitations.push(lim::recognized_words_outside_visible_box(
            recognized_outside,
        ));
    }
    if recognized_under_text > 0 {
        limitations.push(lim::recognized_words_under_drawn_text(
            recognized_under_text,
        ));
    }
    if image_file.is_some() {
        limitations.push(lim::image_resolution_not_read());
    }
    if form_mcids > 0 {
        limitations.push(lim::form_xobject_mcids_not_bound(form_mcids));
    }
    if composite_fonts > 0 {
        limitations.push(lim::composite_font_codes_from_tounicode(composite_fonts));
    }

    // **Encoding holes: declare, or refuse outright.**
    //
    // Some text decoded and some did not — say so, and say how much is missing. But a document
    // that showed text and decoded *none* of it has no usable text layer, and an artifact
    // carrying zero runs would be indistinguishable from a genuinely blank page. That is the
    // one case where refusing is the honest answer (`docs/01-CONTRACT.md` §8).
    if encoding_dropped_runs > 0 {
        let any_text = pages.iter().any(|p| !p.runs.is_empty());
        if !any_text {
            return Err(EngineError::Unsupported {
                what: "text encoding".into(),
                detail: format!(
                    "this document's text layer is unusable: {encoding_dropped_runs} run(s) were \
                     shown and none could be decoded, because no font supplied a `/ToUnicode` \
                     CMap or an encoding this profile can map. No artifact is emitted — an \
                     artifact with zero runs would be indistinguishable from a blank page, and \
                     substituting `U+FFFD` would put characters in the evidence that the \
                     document does not contain. First failure: {encoding_detail}"
                ),
            });
        }
        limitations.push(lim::broken_font_encoding(
            encoding_dropped_runs,
            &encoding_detail,
        ));
    }
    // Decision #43: runs read through what their font itself states, declared with their count.
    if font_read_runs > 0 {
        limitations.push(lim::unmapped_codes_read_from_font(font_read_runs));
    }

    // The document half of the outline declarations. `capabilities.outlines` says this profile
    // LOOKS; these say what it found here, and the three are different statements a consumer
    // cannot recover from an empty array on its own.
    if !outline.declared {
        limitations.push(lim::outline_absent());
    }
    if outline.undecodable_titles > 0 {
        limitations.push(lim::outline_title_undecodable(outline.undecodable_titles));
    }
    if outline.unresolved_destinations > 0 {
        limitations.push(lim::outline_destination_unresolved(
            outline.unresolved_destinations,
        ));
    }

    let artifact = ExtractArtifact {
        identity: ArtifactIdentity {
            artifact_type: EXTRACT_ARTIFACT_TYPE.to_string(),
            schema_version: EXTRACT_SCHEMA_VERSION.to_string(),
            parser_version: profile.parser_version.clone(),
            profile_sha256,
        },
        source: match image_file {
            Some(file) => SourceRef {
                media_type: file.media_type.to_string(),
                sha256: file.sha256,
            },
            None => SourceRef {
                media_type: "application/pdf".to_string(),
                sha256: Sha256Hex::parse(doc.source_sha256().as_str())?,
            },
        },
        reading_order_rule: profile.reading_order_rule.clone(),
        page_count,
        pages,
        outlines: outline.records,
        assurance: Assurance::new(profile.capabilities, page_count, page_states, limitations)?,
    };
    Ok((artifact, positions))
}

/// Whether `page`'s `/Annots` holds an annotation a viewer draws over the page: one neither hidden
/// nor unviewed, with an appearance, or with none of a kind a viewer fills or strokes from its own
/// entries — a square, a circle, a polygon or polyline, a line, ink, free text, a stamp, a
/// redaction (decision #156, review: a `/Square` filled white with no `/AP` hid its text in a
/// viewer and was read; second review: a note, a highlight, a link or a field widget with no `/AP`
/// draws nothing over the text, and refusing them dropped scans read before). A composed page holds
/// none, so text one covers would be read.
fn draws_annotations(doc: &lopdf::Document, page: &lopdf::Dictionary) -> bool {
    let Some(annots) = crate::fonts::resolve_array(doc, page.get(b"Annots").ok()) else {
        return false;
    };
    annots.iter().any(|a| {
        let Some(lopdf::Object::Dictionary(d)) = crate::fonts::resolve_object(doc, Some(a)) else {
            return false;
        };
        let flags = crate::fonts::resolve_object(doc, d.get(b"F").ok())
            .and_then(|f| f.as_i64().ok())
            .unwrap_or(0);
        let drawn_without_appearance = matches!(
            d.get(b"Subtype").and_then(lopdf::Object::as_name),
            Ok(b"Square"
                | b"Circle"
                | b"Polygon"
                | b"PolyLine"
                | b"Line"
                | b"Ink"
                | b"FreeText"
                | b"Stamp"
                | b"Redact")
        );
        // 32000-1 Table 165: bit 2 Hidden, bit 6 NoView.
        (d.has(b"AP") || drawn_without_appearance) && flags & (2 | 32) == 0
    })
}

/// One page's table-detection evidence, exposed for the v2-S22 diagnostic (`cfg(test)`).
///
/// The ink the three rules read, and the [`crate::tables::Detected`] they produced — the typed
/// refusals and rule ids that `extract` above folds into a page-grouped limitation *string* and
/// the artifact then carries only as prose. `docs/table-gate-v1.md`'s v2-S22 question is per gold
/// table — *what ink does this page carry where a gold table is, and which precondition rejected
/// it* — and prose grouped by page cannot answer it without being parsed back into structure.
#[cfg(test)]
pub(crate) struct PageTableDiagnostic {
    /// 1-based page.
    pub page: u32,
    /// Rectangles the page painted (`interp.rects`), which is the ruled rule's whole evidence.
    /// Filled cell boxes and thin stroked-line rectangles both arrive here.
    pub rects: usize,
    /// Horizontal ruling segments (`interp.segments`), the stroke-ruled rule's rows.
    pub horizontal_segments: usize,
    /// Vertical ruling segments, which that rule reads only as column corroboration.
    pub vertical_segments: usize,
    /// Form-field widget rectangles, which the stroke-ruled rule excludes as field boxes.
    pub field_rects: usize,
    /// What the three rules produced here: the emitted tables and each rule's refusal, if any.
    pub detected: crate::tables::Detected,
}

/// **Run the per-page table pipeline and return its structured result** (`cfg(test)`, v2-S22).
///
/// It changes nothing about detection. It runs the SAME per-page ink transform `extract` runs —
/// `interp.rects` and `interp.segments` through the SAME `geom.to_top_left` and quantum as a glyph
/// origin — and calls the SAME [`crate::tables::detect`]. The only thing it does that `extract`
/// does not is *keep* the typed `Detected` instead of discarding it into a limitation string.
///
/// It is `cfg(test)`, so it never enters the shipped graph and cannot move `profile_sha256`. It is
/// a **mirror** of the block inside `extract`, and a mirror can drift: `accuracy::tests`'s
/// `the_ten_documents_where_nothing_is_detected` cross-checks the tables this returns against
/// `extract`'s own artifact, so a divergence is a test failure rather than a silently wrong report.
#[cfg(test)]
pub(crate) fn per_page_table_diagnostics(
    doc: &Document,
    profile: &Profile,
) -> Result<Vec<PageTableDiagnostic>, EngineError> {
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "profile".into(),
            detail: e.to_string(),
        })?;
    let mut alloc = IdAllocator::new(profile_sha256);
    let mut out = Vec::with_capacity(doc.pages().len());
    let content = crate::budget::ContentBudget::default();
    let structure = crate::structure::read(doc.inner())?;

    for &(page_number, page_id) in doc.pages() {
        let page_dict =
            doc.inner()
                .get_dictionary(page_id)
                .map_err(|e| EngineError::Malformed {
                    what: "page dictionary".into(),
                    detail: e.to_string(),
                })?;
        let geom = PageGeometry::resolve(doc, page_dict)?;
        let fonts = load_page_fonts(doc, page_dict)?;
        let operations = page_operations_whole(doc.inner(), page_number, page_id, &content)?;
        let xobjects = crate::images::page_xobjects(doc.inner(), page_dict);
        // The forms the page draws, as `extract_page` runs them.
        let allowance = crate::budget::MAX_PAGE_OPERATIONS.saturating_sub(operations.len());
        let forms =
            crate::form_xobjects::PageForms::new(doc, page_dict, page_number, &content, allowance);
        let resolve = |id| forms.resolve(id);
        let mut interp = Interpreter::new(&fonts)
            .with_xobjects(xobjects)
            .with_forms(&resolve, allowance);
        match interp.run(&operations) {
            Err(_) if interp.form_ops_exhausted => {
                return Err(crate::budget::page_operations_past_ceiling(page_number))
            }
            outcome => outcome?,
        }
        // Read in the turn `extract` reads the page in (decision #155).
        let turn = page_turn(&interp.shown, geom.rotation);
        let geom = geom.turned(turn);

        // Origins, exactly as `extract` builds them: every non-empty shown run at its top-left
        // origin, quantized. Owned first so the borrowed `RunOrigin` view stays valid for `detect`.
        let mut owned: Vec<(i64, i64, i64, String)> = Vec::with_capacity(interp.shown.len());
        for shown in &interp.shown {
            if shown.text.is_empty() {
                continue;
            }
            let (ox_pt, oy_pt) = geom.to_top_left(shown.origin.0, shown.origin.1);
            let origin_x = quantize(ox_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let origin_y = quantize(oy_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let glyph_x = first_glyph_x(
                origin_x,
                travel_along_x(&geom, shown.displacement).is_some(),
                &shown.text,
                shown.code_advances.as_deref(),
                !shown.synthesized_indices.is_empty(),
            );
            owned.push((origin_x, origin_y, glyph_x, shown.text.clone()));
        }
        let origins: Vec<crate::tables::RunOrigin<'_>> = owned
            .iter()
            .map(|(x, y, _, t)| crate::tables::RunOrigin {
                x: *x,
                y: *y,
                text: t.as_str(),
            })
            .collect();
        let glyphs: Vec<crate::tables::RunOrigin<'_>> = owned
            .iter()
            .map(|(_, y, x, t)| crate::tables::RunOrigin {
                x: *x,
                y: *y,
                text: t.as_str(),
            })
            .collect();

        // Rects, exactly as `extract` transforms them.
        let mut table_rects = Vec::with_capacity(interp.rects.len());
        for r in &interp.rects {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_rects.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        let mut table_white = Vec::with_capacity(interp.white.len());
        for r in &interp.white {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_white.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }
        let mut table_inset = Vec::with_capacity(interp.inset.len());
        for r in &interp.inset {
            let (ax, ay) = geom.to_top_left(r.x0, r.y0);
            let (bx, by) = geom.to_top_left(r.x1, r.y1);
            table_inset.push(crate::tables::quantize_rect(ax, ay, bx, by)?);
        }

        // Ruling segments, split by orientation exactly as `extract` splits them.
        let mut stroke_rules = Vec::new();
        let mut uprights = Vec::new();
        for seg in &interp.segments {
            let (ax, ay) = geom.to_top_left(seg.x0, seg.y0);
            let (bx, by) = geom.to_top_left(seg.x1, seg.y1);
            let r = crate::tables::quantize_rect(ax, ay, bx, by)?;
            if geom.level(seg) {
                stroke_rules.push(crate::stroke_ruled::Rule {
                    y: r.y0,
                    x0: r.x0,
                    x1: r.x1,
                });
            } else {
                uprights.push(crate::stroke_ruled::Upright {
                    x: r.x0,
                    y0: r.y0,
                    y1: r.y1,
                });
            }
        }

        // Field rects, exactly as `extract` flips them.
        let field_rects: Vec<crate::tables::QuantRect> =
            crate::forms::widget_rects(doc.inner(), page_dict)
                .into_iter()
                .map(|r| {
                    let q = f64::from(QUANTUM_PER_POINT);
                    let (ax, ay) = geom.to_top_left(r.x0() as f64 / q, r.y0() as f64 / q);
                    let (bx, by) = geom.to_top_left(r.x1() as f64 / q, r.y1() as f64 / q);
                    crate::tables::quantize_rect(ax, ay, bx, by)
                })
                .collect::<Result<Vec<_>, _>>()?;

        let detected = crate::tables::detect(
            page_number,
            &table_rects,
            &table_white,
            // `ruled-rects-v12`: a group's decoration is left out only where the document
            // declares no author structure — where it does, a grid read through its decoration
            // would take a declared table's place in the pairing below.
            no_author_structure(structure.as_ref()).then_some(&table_inset[..]),
            &stroke_rules,
            &uprights,
            &origins,
            &glyphs,
            &field_rects,
            &mut alloc,
        )?;

        out.push(PageTableDiagnostic {
            page: page_number,
            rects: interp.rects.len(),
            horizontal_segments: stroke_rules.len(),
            vertical_segments: uprights.len(),
            field_rects: field_rects.len(),
            detected,
        });
    }
    Ok(out)
}

/// Put one page's runs into reading order, and put its identity there with them (v1-S5).
///
/// `order[i]` is the stream index of the run that belongs at position `i`.
///
/// # One order, not an order and an index
///
/// Three things move together here, and the point is that they cannot come apart:
///
/// 1. **The array.** `runs` ends up in reading order, so a consumer that iterates it reads the
///    document. There is no second field saying "…but actually read them like this".
/// 2. **The span ids.** The allocator handed out `s1…sN` down the content stream; they are
///    re-laid over the permuted list so `s1` is the first run a human should read. Since the
///    allocator's span counter is independent of the table and object counters, re-laying the
///    same contiguous ids in the new positions is **identical** to having allocated them after
///    the reorder — which is what the slice asks for, without moving table and annotation ids
///    that have nothing to do with reading order. `ordinal` is array position, assigned in
///    `crate::represent`, so it follows for free and stays monotone.
/// 3. **`DetectedCell::run_indices`.** These address the page's run list, and the list just
///    moved, so they are remapped. Left alone they would silently point at whatever run now
///    occupies the old slot — a cell claiming text it does not contain, which is the one failure
///    here that no artifact would show.
///
/// 4. **The operator indices** (auto-tagging S2). `op_indices[i]` says which operation showed
///    `runs[i]`, and it is permuted by the same `order` in the same call, so the alignment
///    survives the move without an index of its own — the discipline `region` and `block` follow
///    by travelling inside the run. Left in stream order it would name, for every moved run, the
///    operator of whichever run used to sit at its slot, and the writer would tag the wrong
///    operators while every run's text stayed right.
///
/// A table's runs are contiguous in the new order and keep their relative sequence (they are one
/// atom), so a cell's remapped indices stay ascending and still concatenate to the `text` the
/// detector built.
///
/// **There is no test asserting exactly that, and this comment claimed one from v1-S5 until
/// v2-S13.5.** It named `cell_text_survives_the_reordering`, which has never existed: `git log
/// --all -S` on that identifier returns the single commit that wrote this sentence, and a test
/// added then removed would show two. What does exist covers cell-text *composition* rather than
/// its survival across the reorder — `tables::tests::every_cell_text_is_a_concatenation_of_assigned_runs`,
/// `unruled::tests::cell_text_is_exactly_the_runs_assigned_to_it` and
/// `extraction.rs`'s `every_cell_text_is_built_only_from_extracted_runs` — and the reordering
/// tests beside them (`one_added_line_does_not_reorder_the_page`,
/// `ordinals_and_ids_follow_the_reading_order_on_a_reordered_page`) assert order, not cell text.
/// The property is argued above and **unpinned**; naming the gap is the honest form until a slice
/// writes the test, and `docs/history/15-V2-MILESTONES.md` S13.5 records it.
fn reorder_page(
    runs: &mut Vec<TextRun>,
    tables: &mut [crate::tables::DetectedTable],
    op_indices: &mut Vec<usize>,
    ems: &mut Vec<Option<i64>>,
    order: &[usize],
) {
    // The single-column case, which is most pages: the rule found no gutter and returned the
    // identity. Returning early is not just an optimization — it is the assertion that such a
    // page is byte-identical to what v0 emitted, because nothing at all happened to it.
    if order.iter().enumerate().all(|(i, &old)| i == old) {
        return;
    }

    let ids: Vec<ethos_parser_core::NodeId> = runs.iter().map(|r| r.id.clone()).collect();
    let mut slot: Vec<Option<TextRun>> = runs.drain(..).map(Some).collect();
    let mut position = vec![0usize; slot.len()];

    for (new, &old) in order.iter().enumerate() {
        position[old] = new;
        let mut run = slot[old]
            .take()
            .expect("`order` is a permutation, so no index is visited twice");
        run.id = ids[new].clone();
        runs.push(run);
    }

    // The same permutation, applied to the side table in the same breath (item 4 above).
    let stream_ordered = std::mem::take(op_indices);
    op_indices.extend(order.iter().map(|&old| stream_ordered[old]));
    // And the rendered ems (decision #29), the same way and for the same reason.
    let stream_ordered = std::mem::take(ems);
    ems.extend(order.iter().map(|&old| stream_ordered[old]));

    for table in tables {
        for cell in &mut table.cells {
            for i in &mut cell.run_indices {
                *i = position[*i];
            }
            cell.run_indices.sort_unstable();
        }
    }
}

/// What was observed about one run, beyond its text (v1-S6).
///
/// Every finding comes from evidence the page itself supplies — the rendering mode its content
/// stream set, the box it declares as visible, and since decision #139 the size its matrices draw
/// the run at, the optional content it lies in, and the fill it and what lies beneath it were
/// painted in. None is an appearance judgement, and none removes anything: the run this describes
/// is in the artifact with its text and its origin intact, which is the whole of checklist O21.
fn run_findings(
    shown: &crate::content::ShownText,
    origin_x: i64,
    origin_y: i64,
    visible: &PageBox,
) -> Vec<TextFinding> {
    let mut out = Vec::new();
    let render_mode = shown.render_mode;
    // Modes 3 and 7 fill nothing and stroke nothing (32000-1 Table 106). Mode 7 also adds the
    // glyphs to the clip path, which is a different purpose and the same visible result: no ink.
    // Mode 4 through 6 DO paint and are deliberately not flagged — a rule that called every
    // clipping mode invisible would report ordinary text as hidden.
    //
    // Decision #139: mode 7 only where nothing is painted through the clip it sets before the state
    // holding it is restored — a heading set as a clip and filled with a shading or an image shows.
    // Not in a Type 3 font, whose glyphs mode 3 alone hides (32000-1 §9.3.6; fifth review).
    let type3 = matches!(
        shown.font.widths,
        crate::fonts::WidthSource::Widths {
            type3_scale_x: Some(_),
            ..
        }
    );
    if render_mode == 3 || (render_mode == 7 && !shown.clips_paint && !type3) {
        out.push(TextFinding::InvisibleRenderMode);
    }
    // Decision #139: three more, on a run that draws ink — whitespace draws nothing to hide. The
    // rendered em, the text rendering matrix's vertical scale, under a point — except where a Type
    // 3 font's `/FontMatrix` does not scale its glyph space by a thousandth, and nothing says how
    // tall its em is drawn; a layer switched off; and the fill of what lies beneath.
    if !shown.text.trim().is_empty() {
        let type3_em_unknown = matches!(
            shown.font.widths,
            crate::fonts::WidthSource::Widths { type3_scale_x: Some(s), .. }
                if s as f32 != 0.001_f32
        );
        if shown.em_scale_pt < 1.0 && !type3_em_unknown {
            out.push(TextFinding::TinyText);
        }
        if shown.layer == Some(crate::layers::Layer::Off) {
            out.push(TextFinding::HiddenLayer);
        }
        if shown.fill_matches_background {
            out.push(TextFinding::FillMatchesBackground);
        }
    }
    // The origin, because the origin is the run's address and the one coordinate this engine
    // treats as identity. An ink box would be a better test and most runs do not have one.
    let q = f64::from(QUANTUM_PER_POINT);
    let (x, y) = (origin_x as f64 / q, origin_y as f64 / q);
    if x < visible.x0 || x > visible.x1 || y < visible.y0 || y > visible.y1 {
        out.push(TextFinding::OffPage);
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// A page's operations as `lopdf` decodes them — the buffer `get_page_content` joins, through
/// `Content::decode` — refused by name wherever `lopdf` would lose part of the page without a
/// word, or panic on it.
///
/// Three leniencies the reader must not inherit silently (`docs/OPEN-WORK.md` §6, 2026-09-17): a
/// `FlateDecode` stream truncated or corrupt part way decodes to what came before the damage, as
/// a success; the decoder stops at the first operation its grammar cannot parse and drops the
/// rest of the page; and an inline image with neither a colour space nor `/IM true` makes its
/// parser panic, which the release profile turns into an abort. The tag writer's tokeniser
/// mirrors that grammar rule for rule, places every byte or refuses, and must agree with the
/// decode operator for operator, which together prove the page was read to its end. The operations
/// returned are `lopdf`'s own, so an operation index means what it meant before these checks.
///
/// A `FlateDecode` filter is checked wherever it sits in the chain; a `LZWDecode` or
/// `ASCII85Decode` stream that is corrupt part way is not caught here.
///
/// **Measured on a corpus, 2026-09-18.** Over a private corpus's 981 born-digital pages this
/// refuses one document, whose page content `lopdf` stops
/// reading at byte 4 of 125 718: the build before this check exits 0 on it and writes an artifact
/// whose two nodes carry no text, a complete-looking read of a page nobody read.
/// It is the first corpus example of the defect;
/// the other corpora this repository can reach hold none.
///
/// # Errors
///
/// - [`EngineError::Malformed`] with `what` = `content stream`, naming the page and the stream:
///   a `FlateDecode` stream whose deflate data does not reach its end.
/// - [`EngineError::Unsupported`] with `what` = `content stream tokeniser`, naming the page and
///   the byte: an operation `lopdf`'s grammar cannot parse with bytes after it, a shape its
///   decoder fails the whole page on, or one it panics on.
/// - [`EngineError::ResourceLimit`]: the page's content streams decode to more than
///   [`crate::budget::MAX_DECODED_BYTES`] between them, naming the page; or `budget` is spent.
///
/// # What a page may cost (review 2026-09-25 F08)
///
/// `Ok(None)` when the page holds more than `limit` operations, counted as they are tokenised, so
/// no more than `limit` ever exist. A caller reading pages in parallel passes
/// [`crate::budget::HEAVY_PAGE_OPERATIONS`] and reads such a page again alone; every other caller
/// goes through [`page_operations_whole`]. A page read whole is charged to `budget`.
pub(crate) fn page_operations(
    doc: &lopdf::Document,
    page_number: u32,
    page_id: lopdf::ObjectId,
    budget: &crate::budget::ContentBudget,
    limit: usize,
) -> Result<Option<Vec<lopdf::content::Operation>>, EngineError> {
    budget.refusal()?;
    let label = format!("page {page_number}");
    let Some((operations, bytes)) =
        content_operations(doc, &label, doc.get_page_contents(page_id), limit)?
    else {
        return Ok(None);
    };
    budget.charge(bytes, operations.len())?;
    Ok(Some(operations))
}

/// The operations of `streams` joined, as [`page_operations`] reads a page's `/Contents`, beside
/// the bytes they decoded to, or `Ok(None)` past `limit` operations. `label` names them in a
/// refusal — `page 3`, or `page 3, form 12 0 R` (`docs/30-FORM-XOBJECTS-SCOPE.md` §5) — and
/// nothing is charged: the caller charges.
///
/// # Errors
///
/// As [`page_operations`].
pub(crate) fn content_operations(
    doc: &lopdf::Document,
    label: &str,
    streams: Vec<lopdf::ObjectId>,
    limit: usize,
) -> Result<Option<(Vec<lopdf::content::Operation>, usize)>, EngineError> {
    use crate::budget::{NotDecoded, MAX_DECODED_BYTES};
    use crate::tagging::Inflation;
    let past_ceiling = || crate::budget::stream_past_ceiling(format!("{label}'s content streams"));
    // The page's decoded bytes, the `\n` after each stream aside.
    let mut decoded_bytes = 0usize;
    let mut content = Vec::new();
    // The offset of the `\n` after each stream, where a token may not continue.
    let mut ends = Vec::new();
    for id in streams {
        let stream = match doc.get_object(id) {
            Ok(lopdf::Object::Stream(stream)) => stream,
            // No in-use cross-reference entry: an undefined object, which PDF 32000-1 §7.3.10
            // reads as null. This entry draws nothing, and that is what the page says.
            Err(_) if !in_use(doc, id) => continue,
            // An explicit `null` is that same null.
            Ok(lopdf::Object::Null) => continue,
            // Listed and not loadable, or not a stream: the page draws something this reader
            // cannot read, and an empty page in its place would be a read nobody made.
            _ => {
                return Err(EngineError::Malformed {
                    what: "content stream".into(),
                    detail: format!(
                        "{label}: /Contents names {} {} R, which the cross-reference \
                         table lists but which did not load as a stream",
                        id.0, id.1
                    ),
                })
            }
        };
        let refuse_filter = |detail: String| EngineError::Unsupported {
            what: "content stream filter".into(),
            detail: format!("{label}, stream {} {}: {detail}", id.0, id.1),
        };
        let remaining = MAX_DECODED_BYTES - decoded_bytes;
        let filters = if stream.dict.get(b"Filter").is_ok() {
            stream.filters().map_err(|e| {
                refuse_filter(format!("/Filter is not a name or an array of names: {e}"))
            })?
        } else {
            Vec::new()
        };
        // `lopdf` returns a truncated inflate's partial output as a success wherever `FlateDecode`
        // sits in the chain, so each one's input is checked: the stream's bytes for the first
        // filter, and for a later one what the filters before it decode them to.
        for at in (0..filters.len()).filter(|&at| filters[at] == b"FlateDecode") {
            let decoded_before;
            let input: &[u8] = if at == 0 {
                &stream.content
            } else {
                let mut before = stream.clone();
                let names = filters[..at]
                    .iter()
                    .map(|f| lopdf::Object::Name(f.to_vec()));
                before.dict.set("Filter", names.collect::<Vec<_>>());
                decoded_before = match crate::budget::decoded(&before, remaining) {
                    Ok(bytes) => bytes,
                    Err(NotDecoded::PastLimit) => return Err(past_ceiling()),
                    // Filters that do not decode fail the whole chain, which is refused below.
                    Err(NotDecoded::Failed) => break,
                };
                &decoded_before
            };
            let reach =
                crate::tagging::deflate_reaches_its_end(input, remaining).map_err(|detail| {
                    EngineError::Malformed {
                        what: "content stream".into(),
                        detail: format!("{label}, stream {} {}: {detail}", id.0, id.1),
                    }
                })?;
            if reach == Inflation::PastLimit {
                return Err(past_ceiling());
            }
        }
        // Decoded here rather than through `get_page_content`, whose fallback for a filter it
        // cannot decode is the stream's raw bytes: text the stream's own filter says is not there.
        // An empty chain is no filter, so the stream's own bytes, which `lopdf` decodes to none.
        let decoded = if filters.is_empty() {
            if stream.content.len() > remaining {
                return Err(past_ceiling());
            }
            Ok(stream.content.clone())
        } else {
            crate::budget::decoded(stream, remaining)
        };
        let bytes = match decoded {
            Ok(bytes) => bytes,
            Err(NotDecoded::PastLimit) => return Err(past_ceiling()),
            Err(NotDecoded::Failed) => {
                let chain: Vec<String> = filters
                    .iter()
                    .map(|f| format!("/{}", String::from_utf8_lossy(f)))
                    .collect();
                return Err(refuse_filter(format!(
                    "the filter chain [{}] did not decode, and its raw bytes are not what the \
                     page draws",
                    chain.join(" ")
                )));
            }
        };
        decoded_bytes += bytes.len();
        // A stream of nothing but comments and whitespace draws nothing: PDF 32000-1 §7.2.4 reads
        // a comment as whitespace. `lopdf`'s grammar takes a comment only where the next token
        // follows its line break, so joined to the stream after it, it drops the rest of the page;
        // Canon scanners write `% CANON_PFINF_TYPE0_TEXTON` as a page's first stream.
        if only_comments(&bytes) {
            continue;
        }
        content.extend_from_slice(&bytes);
        content.push(b'\n');
        ends.push(content.len() - 1);
    }
    let on_page = |e: EngineError| match e {
        EngineError::Unsupported { what, detail } => EngineError::Unsupported {
            what,
            detail: format!("{label}: {detail}"),
        },
        other => other,
    };
    let Some(tokens) = crate::tagging::tokenise_within(&content, limit).map_err(on_page)? else {
        return Ok(None);
    };
    crate::tagging::read_as_rendered(&content, &tokens, &ends).map_err(|detail| {
        EngineError::Malformed {
            what: "content stream".into(),
            detail: format!("{label}: {detail}"),
        }
    })?;
    let decoded =
        lopdf::content::Content::decode(&content).map_err(|e| EngineError::Malformed {
            what: "content stream".into(),
            detail: format!("{label}: {e}"),
        })?;
    crate::tagging::agrees_with_lopdf(&tokens, &decoded.operations).map_err(on_page)?;
    Ok(Some((decoded.operations, decoded_bytes)))
}

/// [`page_operations`] for a caller reading one page at a time: every page is read, up to
/// [`crate::budget::MAX_PAGE_OPERATIONS`], and past it refused naming the page.
pub(crate) fn page_operations_whole(
    doc: &lopdf::Document,
    page_number: u32,
    page_id: lopdf::ObjectId,
    budget: &crate::budget::ContentBudget,
) -> Result<Vec<lopdf::content::Operation>, EngineError> {
    let limit = crate::budget::MAX_PAGE_OPERATIONS;
    page_operations(doc, page_number, page_id, budget, limit)?
        .ok_or_else(|| crate::budget::page_operations_past_ceiling(page_number))
}

/// Whether `bytes` hold nothing but comments and whitespace.
fn only_comments(bytes: &[u8]) -> bool {
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .all(|line| matches!(line.trim_ascii_start().first(), None | Some(b'%')))
}

/// Whether the cross-reference table lists `id`, its generation included, as an object in use.
/// A row at offset 0 lists none: byte 0 is `%PDF-` (`document.rs`, `unloaded_in_use`).
fn in_use(doc: &lopdf::Document, id: lopdf::ObjectId) -> bool {
    use lopdf::xref::XrefEntry;
    match doc.reference_table.entries.get(&id.0) {
        Some(XrefEntry::Normal { offset: 0, .. }) => false,
        Some(XrefEntry::Normal { generation, .. }) => *generation == id.1,
        Some(XrefEntry::Compressed { .. }) => id.1 == 0,
        _ => false,
    }
}

fn quantize_err(_: ethos_parser_core::QuantizeError) -> EngineError {
    EngineError::Malformed {
        what: "coordinate".into(),
        detail: "a coordinate is non-finite or outside the canonical integer range".into(),
    }
}

/// A page box, normalised so the low corner really is the low corner.
///
/// PDF permits either diagonal — `[612 792 0 0]` describes the same page as `[0 0 612 792]`
/// (32000-1 §7.9.5) — so every box is normalised on the way in and nothing downstream has to
/// wonder which corner it holds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PageBox {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl PageBox {
    fn from_corners(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            x0: a.min(c),
            y0: b.min(d),
            x1: a.max(c),
            y1: b.max(d),
        }
    }

    fn width(self) -> f64 {
        self.x1 - self.x0
    }

    fn height(self) -> f64 {
        self.y1 - self.y0
    }

    /// The part of `self` that `other` also covers, or `None` when they do not overlap.
    ///
    /// A `/CropBox` reaching outside the `/MediaBox` is clipped to it, per 32000-1 §14.11.2:
    /// the visible page is the intersection, not whichever box is larger.
    fn intersect(self, other: Self) -> Option<Self> {
        let r = Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        (r.x1 > r.x0 && r.y1 > r.y0).then_some(r)
    }
}

/// A page reads turned where this many characters or more travel one way (decision #155).
pub const TURN_MIN_CHARS: usize = 40;

/// ...on this many lines or more: a stamp or a caption down the margin is a line or three.
pub const TURN_MIN_LINES: usize = 4;

/// Quarter turns clockwise beyond `rotation` in which a page's `shown` text reads upright
/// (decision #155): where nine tenths of its characters that are not spaces, or more, travel one
/// way in user space — right, up, left or down — that a declared rotation other than `rotation`
/// would stand upright, on [`TURN_MIN_LINES`] lines or more, and [`TURN_MIN_CHARS`] or more travel
/// that way. A run that says no way counts against every way: one of no known travel, one that
/// does not move (a font whose widths are 0), one more than a hundredth off an axis (a diagonal
/// watermark) and one mirrored, its glyphs standing on its right. 0 otherwise: a page with a few
/// turned labels, a vertical axis title or a sideways stamp, reads as declared.
pub(crate) fn page_turn(shown: &[crate::content::ShownText], rotation: i64) -> i64 {
    turn_of(
        shown.iter().map(|s| {
            let n = s.text.chars().filter(|c| !c.is_whitespace()).count();
            (s.displacement, s.glyph_up, s.origin, n)
        }),
        rotation,
    )
}

/// One run for [`turn_of`]: its user-space travel, its glyphs' up, its origin, its characters.
type TurnRun = (Option<(f64, f64)>, (f64, f64), (f64, f64), usize);

/// [`page_turn`] over each run's travel, glyph up, origin and characters.
fn turn_of(runs: impl Iterator<Item = TurnRun>, rotation: i64) -> i64 {
    // Characters, and the lines they are on, by the rotation that stands their travel upright:
    // 0, 90, 180, 270.
    let mut by = [0usize; 4];
    let mut lines: [std::collections::BTreeSet<i64>; 4] = Default::default();
    let mut total = 0usize;
    for (travel, (ux, uy), (ox, oy), n) in runs {
        total += n;
        // A run of spaces inks no line (review).
        let Some((dx, dy)) = travel.filter(|_| n > 0) else {
            continue;
        };
        // Off the axes, or its glyphs not on its left: a run that does not move is neither.
        let along_an_axis = dx.abs().min(dy.abs()) * 100.0 <= dx.abs().max(dy.abs());
        if !along_an_axis || dx * uy - dy * ux <= 0.0 {
            continue;
        }
        let (k, line) = if dx.abs() > dy.abs() {
            (if dx > 0.0 { 0 } else { 2 }, oy)
        } else {
            (if dy > 0.0 { 1 } else { 3 }, ox)
        };
        by[k] += n;
        lines[k].insert(line.round() as i64);
    }
    let Some((k, &most)) = by.iter().enumerate().max_by_key(|&(_, n)| n) else {
        return 0;
    };
    if most < TURN_MIN_CHARS || 10 * most < 9 * total || lines[k].len() < TURN_MIN_LINES {
        return 0;
    }
    (90 * k as i64 - rotation).rem_euclid(360)
}

/// Page boxes and rotation, and the transform into the declared coordinate system.
pub(crate) struct PageGeometry {
    /// The page's `/MediaBox`, normalised. **Its origin is load-bearing** — see
    /// [`PageGeometry::to_top_left`].
    media: PageBox,
    /// The box a viewer actually shows: `/CropBox` clipped to the media box, or the media box
    /// where the page declares no crop box (v1-S6).
    ///
    /// Read for one reason: an off-page finding has to be measured against the box content is
    /// *visible* in, and that is this one. Measuring against `/MediaBox` on a page that crops
    /// would report ordinary trimmed content as off-page — a fabricated finding, which is worse
    /// than no finding at all.
    visible: PageBox,
    rotation: i64,
    /// Quarter turns clockwise beyond `rotation` in which the page is read (decision #155): 0
    /// where its text reads as declared. Every transform below takes the two together.
    turn: i64,
    display_width: f64,
    display_height: f64,
}

impl PageGeometry {
    pub(crate) fn resolve(
        doc: &Document,
        page_dict: &lopdf::Dictionary,
    ) -> Result<Self, EngineError> {
        let media =
            Self::box_from(doc, page_dict, b"MediaBox").ok_or(EngineError::MissingPart {
                part: "/MediaBox".into(),
            })?;

        // v1-S6. The visible box: `/CropBox` clipped to the media box, or the media box where no
        // crop box is declared. A crop box that does not overlap the media box at all describes
        // nothing visible, and rather than emit an empty page geometry the media box stands —
        // the document contradicted itself and the larger, always-present box is the safer of the
        // two answers.
        let visible = Self::box_from(doc, page_dict, b"CropBox")
            .and_then(|c| c.intersect(media))
            .unwrap_or(media);

        // `/Rotate` is inheritable (32000-1 Table 30), and its value may be an indirect
        // reference or a real. Reading it with `as_i64()` off the page dictionary alone made a
        // `/Rotate 90` on the `/Pages` node — or `/Rotate 90 0 R` — silently mean zero, and a
        // page rotated by a wrong amount puts every coordinate in the wrong place.
        let rotation = Self::inherited(doc, page_dict, b"Rotate")
            .and_then(
                |o| match crate::fonts::resolve_object(doc.inner(), Some(&o)) {
                    Some(lopdf::Object::Integer(v)) => Some(v),
                    Some(lopdf::Object::Real(v)) => Some(f64::from(v) as i64),
                    _ => None,
                },
            )
            .unwrap_or(0)
            .rem_euclid(360);
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err(EngineError::Malformed {
                what: "/Rotate".into(),
                detail: format!("{rotation} is not a multiple of 90"),
            });
        }

        // A quarter turn swaps the visible dimensions.
        //
        // **Taken from the MEDIA box, not the visible one, and the difference is load-bearing.**
        // Every coordinate in this artifact is expressed in the media box's frame — `to_top_left`
        // subtracts *its* origin — so the page's declared width and height have to describe that
        // same frame. Reporting the crop box's size beside media-box coordinates puts two frames
        // on one page, and `DocumentRepresentation::seal` REFUSES an artifact whose measured box
        // falls outside its page: a document that crops would stop producing an artifact at all.
        //
        // Measured, not reasoned: a probe page with `/MediaBox [0 0 300 200]` and
        // `/CropBox [50 50 250 150]`, text at user-space y=180 with a font carrying real metrics,
        // exited 2 with *"node `s1` has a measured box [6000, 277, 20400, 2497] outside its page
        // [0, 0, 20000, 10000]"*. It is the `crop-box-smaller-than-media` fixture now.
        //
        // The crop box is still read, and is still what an off-page finding is measured against —
        // that is a different question ("is this content visible?") with its own answer on the
        // run, and `off-page-text` says so in as many words.
        let (display_width, display_height) = if rotation % 180 == 0 {
            (media.width(), media.height())
        } else {
            (media.height(), media.width())
        };

        Ok(Self {
            media,
            visible,
            rotation,
            turn: 0,
            display_width,
            display_height,
        })
    }

    /// A page box by name, normalised, resolving indirect references and one `/Parent` hop.
    fn box_from(doc: &Document, page_dict: &lopdf::Dictionary, key: &[u8]) -> Option<PageBox> {
        let array = crate::fonts::resolve_array(
            doc.inner(),
            Self::inherited(doc, page_dict, key).as_ref(),
        )?;
        if array.len() != 4 {
            return None;
        }
        let n = |i: usize| -> Option<f64> {
            match crate::fonts::resolve_object(doc.inner(), array.get(i))? {
                lopdf::Object::Integer(v) => Some(v as f64),
                lopdf::Object::Real(v) => Some(f64::from(v)),
                _ => None,
            }
        };
        let (a, b, c, d) = (n(0)?, n(1)?, n(2)?, n(3)?);
        if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
            return None;
        }
        let r = PageBox::from_corners(a, b, c, d);
        (r.width() > 0.0 && r.height() > 0.0).then_some(r)
    }

    /// A key from the page dictionary, or from an ancestor that declares it.
    ///
    /// Bounded at [`INHERITANCE_MAX_DEPTH`] hops. An unbounded walk over a `/Parent` chain a
    /// document controls is a hang a document can cause, and a cycle is a document this reader
    /// must survive rather than spin on.
    fn inherited(
        doc: &Document,
        page_dict: &lopdf::Dictionary,
        key: &[u8],
    ) -> Option<lopdf::Object> {
        if let Ok(v) = page_dict.get(key) {
            return Some(v.clone());
        }
        let mut node = crate::fonts::resolve_dict(doc.inner(), page_dict.get(b"Parent").ok())?;
        for _ in 0..INHERITANCE_MAX_DEPTH {
            if let Ok(v) = node.get(key) {
                return Some(v.clone());
            }
            node = crate::fonts::resolve_dict(doc.inner(), node.get(b"Parent").ok())?;
        }
        None
    }

    /// Map a user-space point into the declared top-left system, applying `/Rotate`.
    ///
    /// Derived by asking where each corner lands under a clockwise quarter turn, rather than by
    /// pattern-matching a formula: for `/Rotate 90` the bottom-left corner becomes the top-left,
    /// which fixes the mapping uniquely.
    ///
    /// # The box origin is subtracted, and that was a repair (v1-S6)
    ///
    /// Through v1-S5 this used only the box's *width and height* and threw its origin away, so a
    /// page whose `/MediaBox` is `[0 20 612 812]` — a legal and not unusual box — had every
    /// coordinate in the artifact shifted by 20 points, with the top of the page landing at
    /// `y = -20`. Nothing caught it because **not one document in either corpus declares a box
    /// whose origin is other than `(0, 0)`**, measured across all 67 PDFs available to this
    /// repository; on such a page `x0 = y0 = 0` and the subtraction below is the identity, which
    /// is why this change moves no existing golden.
    ///
    /// It had to be fixed before v1-S6 could ship an off-page finding at all: a bounds test
    /// against a frame the content is systematically offset from reports ordinary text at the top
    /// of a page as off-page, and a **fabricated** security finding is worse than no finding.
    fn to_top_left(&self, x: f64, y: f64) -> (f64, f64) {
        let b = self.media;
        match self.read_as() {
            90 => (y - b.y0, x - b.x0),
            180 => (b.x1 - x, y - b.y0),
            270 => (b.y1 - y, b.x1 - x),
            // 0, and anything else is refused before reaching here.
            _ => (x - b.x0, b.y1 - y),
        }
    }

    /// Map a user-space vector into the declared top-left system: the linear part of
    /// [`Self::to_top_left`], with the box origin's translation left out.
    ///
    /// For a run's travel and its glyphs' y axis, which are directions rather than places.
    fn to_top_left_linear(&self, dx: f64, dy: f64) -> (f64, f64) {
        match self.read_as() {
            90 => (dy, dx),
            180 => (-dx, dy),
            270 => (-dy, -dx),
            _ => (dx, -dy),
        }
    }

    /// Whether a quantized box lies inside this page, by **the seal's own predicate** (D4-S5).
    ///
    /// Deliberately a restatement rather than a near-miss.
    /// `DocumentRepresentation::check_box_within_page` tests
    /// `x0 < 0 || y0 < 0 || x1 > page.width || y1 > page.height` against a `PageRecord` whose
    /// width and height are `quantize(display_width)` and `quantize(display_height)` — the two
    /// lines that build it are in `page_record` above. This quantizes the same two fields the same
    /// way and asks the same question, so a box this accepts is a box the seal accepts. A test
    /// pins the agreement, because two spellings of one invariant is exactly the drift
    /// `docs/06-STEAL-REFUSE.md` warns a second copy of a table produces.
    ///
    /// A dimension that will not quantize returns `false`: the box cannot be shown to be on a page
    /// this engine cannot express, and claiming containment against a number it failed to compute
    /// would be the fabrication the caller exists to avoid.
    fn contains(&self, r: ethos_parser_core::QRect) -> bool {
        let (Ok(w), Ok(h)) = (
            quantize(self.display_width, QUANTUM_PER_POINT),
            quantize(self.display_height, QUANTUM_PER_POINT),
        ) else {
            return false;
        };
        r.x0() >= 0 && r.y0() >= 0 && r.x1() <= w && r.y1() <= h
    }

    /// Map a point in the declared top-left system back into PDF user space (v1-S6).
    ///
    /// The exact inverse of [`Self::to_top_left`], and it exists for one caller: the annotated
    /// overlay, which holds rectangles in the artifact's coordinate system and has to write them
    /// into a PDF, where annotations are in user space. Deriving the inverse here rather than at
    /// the call site keeps the two transforms in one place, where a test can check that composing
    /// them is the identity.
    pub(crate) fn to_user_space(&self, x: f64, y: f64) -> (f64, f64) {
        let b = self.media;
        match self.read_as() {
            90 => (y + b.x0, x + b.y0),
            180 => (b.x1 - x, y + b.y0),
            270 => (b.x1 - y, b.y1 - x),
            _ => (x + b.x0, b.y1 - y),
        }
    }

    /// The rotation the page is read in: its declared `/Rotate` and its turn (decision #155).
    fn read_as(&self) -> i64 {
        (self.rotation + self.turn) % 360
    }

    /// Whether `seg` is level as this page is read (review of decision #155): on a page read a
    /// quarter turn round, a line drawn up the page as declared. A page whose `/Rotate` alone
    /// turns it keeps the reading `stroke-ruled-v2` released, user space's.
    fn level(&self, seg: &crate::content::PathSegment) -> bool {
        if self.turn == 0 {
            seg.is_horizontal()
        } else {
            seg.is_horizontal() == (self.read_as() % 180 == 0)
        }
    }

    /// This page read `turn` quarter turns clockwise beyond its declared rotation (decision #155):
    /// its display width and height exchange where the turn is odd.
    pub(crate) fn turned(self, turn: i64) -> Self {
        let (display_width, display_height) = if turn % 180 == 0 {
            (self.display_width, self.display_height)
        } else {
            (self.display_height, self.display_width)
        };
        Self {
            turn,
            display_width,
            display_height,
            ..self
        }
    }

    /// The visible page box, mapped into the declared top-left system (v1-S6).
    ///
    /// Both corners go through [`Self::to_top_left`] and are then normalised, because a rotation
    /// can exchange which corner is which — deriving the frame from the same transform the
    /// content goes through is what makes the comparison meaningful.
    fn visible_in_display_space(&self) -> PageBox {
        let (ax, ay) = self.to_top_left(self.visible.x0, self.visible.y0);
        let (bx, by) = self.to_top_left(self.visible.x1, self.visible.y1);
        PageBox::from_corners(ax, ay, bx, by)
    }
}

/// How far a `/Parent` chain is walked for an inheritable page attribute.
///
/// Same reasoning and the same shape as the structure walk's depth bound: a chain the document
/// controls must not decide how long this process runs, and a cycle must be survived rather than
/// spun on.
pub(crate) const INHERITANCE_MAX_DEPTH: usize = 32;

/// A page's resource dictionary: its own `/Resources`, or the nearest ancestor's (PDF 32000-1
/// Table 30 makes the key inheritable).
///
/// A `/Resources` that does not resolve to a dictionary — null, or a reference to no object — is
/// read as absent, as §7.3.7 reads a null entry, and the walk goes on up. Every reader of a page's
/// resources asks this, so fonts, images, marked-content property lists and the tag writer find
/// the same dictionary: until review 2026-09-26 N43 only the property lists and the writer
/// inherited, and a page taking its fonts from `/Pages` was refused for a missing font.
pub(crate) fn page_resources(
    doc: &lopdf::Document,
    page_dict: &lopdf::Dictionary,
) -> Option<lopdf::Dictionary> {
    if let Some(own) = crate::fonts::resolve_dict(doc, page_dict.get(b"Resources").ok()) {
        return Some(own);
    }
    let mut node = crate::fonts::resolve_dict(doc, page_dict.get(b"Parent").ok());
    for _ in 0..INHERITANCE_MAX_DEPTH {
        let dict = node?;
        if let Some(found) = crate::fonts::resolve_dict(doc, dict.get(b"Resources").ok()) {
            return Some(found);
        }
        node = crate::fonts::resolve_dict(doc, dict.get(b"Parent").ok());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decision #161 (PR 58's review): the images' area is their union inside the visible box — what
    /// two paint over the same place counts once, what lies outside the box not at all, and parts
    /// apart add up.
    #[test]
    fn union_area_counts_overlap_once() {
        let r = |x0, y0, x1, y1| crate::tables::QuantRect { x0, y0, x1, y1 };
        let page = r(0, 0, 100, 100);
        assert_eq!(union_area(&[], page), 0);
        assert_eq!(
            union_area(&[r(0, 0, 20, 100), r(0, 0, 20, 100)], page),
            2_000
        );
        assert_eq!(
            union_area(&[r(0, 0, 20, 100), r(50, 0, 70, 100)], page),
            4_000
        );
        // Two crossing: 30 x 30 each, 10 x 10 shared.
        assert_eq!(
            union_area(&[r(0, 0, 30, 30), r(20, 20, 50, 50)], page),
            1_700
        );
        // Past the box's right and bottom edges.
        assert_eq!(union_area(&[r(90, 95, 150, 150)], page), 50);
        // One inside another, one half off the box, one with no area.
        assert_eq!(
            union_area(
                &[
                    r(10, 10, 90, 90),
                    r(20, 20, 30, 30),
                    r(-50, 0, 10, 10),
                    r(5, 5, 5, 50)
                ],
                page
            ),
            6_400 + 100
        );
    }

    /// **A `/TH` reaches only the detected cell that holds its content** (decision #167): on a grid
    /// the tree agrees with, the cell at the `/TH`'s position is a header where it holds a run of
    /// the marked content the `/TH` cites on this page, and not where it holds other content, nor
    /// where the `/TH` names another page.
    #[test]
    fn a_th_is_carried_only_onto_the_cell_holding_its_content() {
        let mut alloc = IdAllocator::new(
            ethos_parser_core::Profile::default()
                .profile_sha256()
                .expect("hashes"),
        );
        let table_id = alloc.next(ethos_parser_core::IdKind::Table).expect("an id");
        let detected = |column: u32, run: usize| crate::tables::DetectedCell {
            position: ethos_parser_core::TableCellPosition {
                row: 0,
                column,
                rowspan: 1,
                colspan: 1,
                table_id: table_id.clone(),
            },
            rect: crate::tables::QuantRect {
                x0: 0,
                y0: 0,
                x1: 1,
                y1: 1,
            },
            run_indices: vec![run],
            text: String::new(),
            header: false,
        };
        let th = |column: u32, mcid: i64, page: lopdf::ObjectId| crate::structure::TaggedCell {
            row: 0,
            column,
            rowspan: 1,
            colspan: 1,
            mcids: vec![mcid],
            page: Some(page),
            header: true,
        };
        let here = (4, 0);
        // Run `i` carries marked-content id `10 + i`.
        let mut cells = vec![detected(0, 0), detected(1, 1), detected(2, 2)];
        let tags = [th(0, 10, here), th(1, 99, here), th(2, 12, (5, 0))];
        carry_tagged_headers(&mut cells, &tags, here, |i| Some(10 + i as i64));
        let got: Vec<bool> = cells.iter().map(|c| c.header).collect();
        assert_eq!(got, [true, false, false]);
    }

    /// **A caption names a figure or a table and heads no section** (decision #159): a caption
    /// word, then a number, spaced or not, as a page drawing a glyph to a run gives it.
    #[test]
    fn a_caption_word_and_a_number_name_a_figure() {
        for caption in [
            "Figure 3.3. Biomass use",
            "Fig. 2",
            "FIGURE1",
            "Table 2: Rates",
            " Exhibit 12",
            "TABLE I. Rates by site",
            "Table IV: Rates",
            "Figure S1. Rates",
            "Fig. A1",
            "Table A.2",
            "Abbildung 3: Raten",
            "Tableau 2. Taux",
            "Scheme 1.",
            "Algorithm 1",
            "Figure: Rates by site",
        ] {
            assert!(names_a_figure(caption), "{caption}");
        }
        for heading in [
            "Table of Contents",
            "Figures 2",
            "Tablespoon 2",
            "Chart",
            "Scope of work",
            "Tables and Figures 2024",
            "Figure skating rules",
            "Table in the hall",
            "Figural analysis",
            "EXHIBIT C",
            "Exhibit L",
            "Table L of fees",
            "EXHIBIT IV",
            "Tablet2 setup",
        ] {
            assert!(!names_a_figure(heading), "{heading}");
        }
    }

    fn geom(w: f64, h: f64, rot: i64) -> PageGeometry {
        let (dw, dh) = if rot % 180 == 0 { (w, h) } else { (h, w) };
        let media = PageBox::from_corners(0.0, 0.0, w, h);
        PageGeometry {
            media,
            visible: media,
            rotation: rot,
            turn: 0,
            display_width: dw,
            display_height: dh,
        }
    }

    /// **A run's trailing whitespace is its last codes' advance** — the wide space after `3` the
    /// tracks rule must not read as part of the cell — and nothing where the run ends inked, is
    /// whitespace alone, or its codes and characters do not pair one for one.
    #[test]
    fn trailing_whitespace_is_the_last_codes_advance() {
        assert_eq!(
            trailing_whitespace_advance("3 ", Some(&[5.0, 80.0]), false),
            Some(80.0)
        );
        assert_eq!(
            trailing_whitespace_advance("$1,000  ", Some(&[5.0; 8]), false),
            Some(10.0)
        );
        for (text, per, inserted, why) in [
            ("3", &[5.0][..], false, "no trailing whitespace"),
            ("  ", &[3.0, 3.0][..], false, "whitespace alone"),
            (
                "fi ",
                &[5.0, 3.0][..],
                false,
                "a ligature: two characters, one code",
            ),
            ("3 ", &[5.0, 80.0][..], true, "a space this reader inserted"),
        ] {
            assert_eq!(
                trailing_whitespace_advance(text, Some(per), inserted),
                None,
                "{why}"
            );
        }
        assert_eq!(
            trailing_whitespace_advance("3 ", None, false),
            None,
            "no widths"
        );
    }

    /// **A run's leading whitespace is its first codes' advance** — the widened space a page draws
    /// ` None` after — and nothing where the run opens inked, is whitespace alone, or its codes and
    /// characters do not pair one for one.
    #[test]
    fn leading_whitespace_is_the_first_codes_advance() {
        assert_eq!(
            leading_whitespace_advance(" None", Some(&[182.8, 7.2, 5.6, 5.6, 5.6]), false),
            Some(182.8)
        );
        assert_eq!(
            leading_whitespace_advance("  $1", Some(&[2.5, 2.5, 5.0, 5.0]), false),
            Some(5.0)
        );
        for (text, per, inserted, why) in [
            (
                "None ",
                &[7.2, 5.6, 5.6, 5.6, 2.8][..],
                false,
                "no leading whitespace",
            ),
            ("  ", &[3.0, 3.0][..], false, "whitespace alone"),
            (
                " fi",
                &[3.0, 5.0][..],
                false,
                "a ligature: two characters, one code",
            ),
            (
                " ab",
                &[3.0, 1.0, 5.0, 5.0][..],
                false,
                "a code that decoded to no character",
            ),
            (
                " None",
                &[182.8, 7.2, 5.6, 5.6, 5.6][..],
                true,
                "a space this reader inserted",
            ),
        ] {
            assert_eq!(
                leading_whitespace_advance(text, Some(per), inserted),
                None,
                "{why}"
            );
        }
        assert_eq!(
            leading_whitespace_advance(" 3", None, false),
            None,
            "no widths"
        );
    }

    /// **The ruled rule's position is a run's first inked character where its pen runs along the
    /// page's x**, and its origin wherever it does not or no leading whitespace can be read.
    #[test]
    fn a_run_is_placed_by_its_first_glyph_only_along_x() {
        let per = [182.8, 7.2, 5.6, 5.6, 5.6];
        assert_eq!(
            first_glyph_x(13224, true, " None", Some(&per), false),
            31504
        );
        assert_eq!(
            first_glyph_x(13224, false, " None", Some(&per), false),
            13224
        );
        assert_eq!(
            first_glyph_x(13224, true, "None", Some(&per[1..]), false),
            13224
        );
        assert_eq!(first_glyph_x(13224, true, " None", None, false), 13224);
    }

    #[test]
    fn unrotated_pages_flip_the_y_axis_only() {
        let g = geom(300.0, 144.0, 0);
        // Bottom-left in user space is the origin in a top-left system.
        assert_eq!(g.to_top_left(0.0, 144.0), (0.0, 0.0));
        // Top-left in user space is the bottom-left of the display.
        assert_eq!(g.to_top_left(0.0, 0.0), (0.0, 144.0));
        assert_eq!(g.to_top_left(72.0, 72.0), (72.0, 72.0));
    }

    /// The corner test that pins the 90° mapping.
    #[test]
    fn a_quarter_turn_sends_the_bottom_left_corner_to_the_top_left() {
        // MediaBox [0 0 144 300] with /Rotate 90 — the rotation-90 fixture's shape.
        let g = geom(144.0, 300.0, 90);
        assert_eq!(
            g.display_width, 300.0,
            "a quarter turn swaps the dimensions"
        );
        assert_eq!(g.display_height, 144.0);

        assert_eq!(
            g.to_top_left(0.0, 0.0),
            (0.0, 0.0),
            "bottom-left -> top-left"
        );
        assert_eq!(
            g.to_top_left(144.0, 0.0),
            (0.0, 144.0),
            "bottom-right -> bottom-left"
        );
        assert_eq!(
            g.to_top_left(0.0, 300.0),
            (300.0, 0.0),
            "top-left -> top-right"
        );
    }

    /// **A page reads turned where nine tenths of its characters travel one way** (decision #155):
    /// right reads as declared; up, left and down a quarter, half and three-quarter turn; a few
    /// turned labels, under nine tenths, or too few characters, read as declared; and a page whose
    /// rotation already stands its text upright needs no turn.
    #[test]
    fn a_page_reads_turned_where_its_text_travels_one_way() {
        // `n` characters travelling `d`, their glyphs standing on its left, over `lines` lines,
        // beside `rest` characters set upright.
        let up_of = |(dx, dy): (f64, f64)| (-dy, dx);
        let runs = |d: (f64, f64), up: (f64, f64), n: usize, lines: usize, rest: usize| {
            // Each line 12 points further across the travel than the last.
            let across = |i: usize| (12.0 * i as f64 * d.1.abs(), 12.0 * i as f64 * d.0.abs());
            let mut v: Vec<_> = (0..lines)
                .map(|i| (Some(d), up, across(i), n / lines))
                .collect();
            v.push((Some((1.0, 0.0)), (0.0, 1.0), (0.0, 0.0), rest));
            v
        };
        let page = |d: (f64, f64), n: usize, rest: usize, rotation: i64| {
            turn_of(runs(d, up_of(d), n, 4, rest).into_iter(), rotation)
        };
        assert_eq!(page((1.0, 0.0), 100, 0, 0), 0);
        assert_eq!(page((0.0, 1.0), 100, 0, 0), 90);
        assert_eq!(page((-1.0, 0.0), 100, 0, 0), 180);
        assert_eq!(page((0.0, -1.0), 100, 0, 0), 270);
        assert_eq!(page((0.0, 1.0), 180, 20, 0), 90, "nine tenths");
        assert_eq!(page((0.0, 1.0), 176, 24, 0), 0, "under nine tenths");
        assert_eq!(page((0.0, 1.0), 36, 0, 0), 0, "too few characters");
        assert_eq!(page((0.0, 1.0), 40, 0, 0), 90);
        assert_eq!(page((0.0, 1.0), 100, 0, 90), 0, "declared already");
        assert_eq!(page((0.0, -1.0), 100, 0, 90), 180);

        // A run that says no way counts against a turn: one that does not move (a font whose
        // widths are 0), one off the axes (a diagonal watermark), one mirrored (its glyphs on
        // its right) and one of no known travel.
        let still = (0..4).map(|i| (Some((0.0, 0.0)), (0.0, 1.0), (12.0 * i as f64, 0.0), 25));
        assert_eq!(turn_of(still, 0), 0, "runs that do not move");
        let mirror = runs((-1.0, 0.0), (0.0, 1.0), 100, 4, 0);
        assert_eq!(turn_of(mirror.into_iter(), 0), 0, "a page mirrored");
        let diagonal = (Some((1.0, 1.0)), (-1.0, 1.0), (0.0, 0.0), 100);
        let mirrored = (Some((-1.0, 0.0)), (0.0, 1.0), (0.0, 0.0), 100);
        let unknown = (None, (0.0, 1.0), (0.0, 0.0), 100);
        for (name, other) in [
            ("diagonal", diagonal),
            ("mirrored", mirrored),
            ("unknown", unknown),
        ] {
            let mut v = runs((0.0, 1.0), (-1.0, 0.0), 100, 4, 0);
            v.push(other);
            assert_eq!(turn_of(v.into_iter(), 0), 0, "{name}");
        }
        let skewed = |d| turn_of(runs(d, up_of(d), 100, 4, 0).into_iter(), 0);
        assert_eq!(skewed((0.009, 0.999)), 90, "within a hundredth of the axis");
        assert_eq!(skewed((0.02, 0.999)), 0, "off the axis");

        // A stamp or a running caption down the margin is a line or three, not a page.
        let lined = |lines| turn_of(runs((0.0, 1.0), (-1.0, 0.0), 120, lines, 0).into_iter(), 0);
        assert_eq!(lined(3), 0, "three lines");
        assert_eq!(lined(4), 90, "four lines");
        let mut spaced = runs((0.0, 1.0), (-1.0, 0.0), 120, 1, 0);
        spaced.extend((1..4).map(|i| (Some((0.0, 1.0)), (-1.0, 0.0), (12.0 * i as f64, 0.0), 0)));
        assert_eq!(
            turn_of(spaced.into_iter(), 0),
            0,
            "one line, three of spaces"
        );
    }

    /// **A turned page's boxes map back to the declared frame through the record** (decision
    /// #155): at every declared rotation and every turn, a box measured on the turned page and
    /// mapped back by `PageRecord::to_declared` is the box the declared page measures.
    #[test]
    fn a_turned_page_maps_back_through_its_record() {
        let q = |v: f64| (v * 100.0).round() as i64;
        for rotation in [0i64, 90, 180, 270] {
            for turn in [0i64, 90, 180, 270] {
                let declared = geom(300.0, 400.0, rotation);
                let turned = geom(300.0, 400.0, rotation).turned(turn);
                let rect = |g: &PageGeometry| {
                    let (ax, ay) = g.to_top_left(30.0, 50.0);
                    let (bx, by) = g.to_top_left(120.0, 70.0);
                    ethos_parser_core::QRect::new(
                        q(ax.min(bx)),
                        q(ay.min(by)),
                        q(ax.max(bx)),
                        q(ay.max(by)),
                    )
                    .unwrap()
                };
                let page = ethos_parser_core::PageRecord {
                    id: ethos_parser_core::NodeId::from_parts(ethos_parser_core::IdKind::Page, 1),
                    index: 1,
                    width: q(turned.display_width),
                    height: q(turned.display_height),
                    rotation,
                    turn,
                    visible: None,
                    drawn_boxes: Vec::new(),
                };
                assert_eq!(
                    page.to_declared(rect(&turned)),
                    Some(rect(&declared)),
                    "rotation {rotation}, turn {turn}"
                );
            }
        }
    }

    /// A vector lands where the difference of its two endpoints lands, at every rotation and on a
    /// box whose origin is not (0, 0) — so the translation really is what was left out.
    #[test]
    fn the_linear_part_maps_a_vector_as_the_point_map_does() {
        let media = PageBox::from_corners(10.0, 20.0, 310.0, 420.0);
        for rotation in [0i64, 90, 180, 270] {
            let g = PageGeometry {
                media,
                visible: media,
                rotation,
                turn: 0,
                display_width: 300.0,
                display_height: 400.0,
            };
            for (x, y) in [(10.0, 20.0), (72.0, 350.0)] {
                for (dx, dy) in [(1.0, 0.0), (0.0, 1.0), (25.0, -7.0), (-3.0, 40.0)] {
                    let (ax, ay) = g.to_top_left(x, y);
                    let (bx, by) = g.to_top_left(x + dx, y + dy);
                    assert_eq!(
                        g.to_top_left_linear(dx, dy),
                        (bx - ax, by - ay),
                        "rotation {rotation}: ({dx}, {dy}) from ({x}, {y})"
                    );
                }
            }
        }
    }

    #[test]
    fn every_rotation_keeps_points_inside_the_display_box() {
        for rot in [0i64, 90, 180, 270] {
            let g = geom(144.0, 300.0, rot);
            for (x, y) in [
                (0.0, 0.0),
                (144.0, 0.0),
                (0.0, 300.0),
                (144.0, 300.0),
                (72.0, 150.0),
            ] {
                let (dx, dy) = g.to_top_left(x, y);
                assert!(
                    (0.0..=g.display_width).contains(&dx),
                    "rot {rot}: x {dx} outside 0..{}",
                    g.display_width
                );
                assert!(
                    (0.0..=g.display_height).contains(&dy),
                    "rot {rot}: y {dy} outside 0..{}",
                    g.display_height
                );
            }
        }
    }

    /// The rule id the default profile names, and the one it no longer does (v1-S5).
    ///
    /// v0 named `single-column-v1`, meaning content-stream order with nothing reordered. The new
    /// rule got a **new id** rather than a bump of that one, because the old string still has a
    /// true meaning and a profile that turns the capability off still uses it. Pinning both here
    /// is what stops a later slice from quietly redefining either.
    #[test]
    fn the_reading_order_rule_is_the_gutter_rule_and_not_the_v0_id() {
        assert_eq!(
            Profile::default().reading_order_rule,
            ethos_parser_core::READING_ORDER_RULE_V5,
            "the side-band join moved the default to the id that promises it (decision #133)"
        );
        assert_eq!(
            ethos_parser_core::READING_ORDER_RULE_V5,
            "gutter-columns-v5",
            "the id is data on every artifact; changing the string is an identity event"
        );
        // **`-v4` keeps its exact spelling**: artifacts exist under it, and on a page with a
        // narrow band set on its neighbour's rows they list runs in the order `-v5` changes.
        assert_eq!(
            ethos_parser_core::READING_ORDER_RULE_V4,
            "gutter-columns-v4",
            "an id under which artifacts were produced is frozen, not renamed"
        );
        // **`-v3` keeps its exact spelling**: artifacts exist under it, and on a page the cut never
        // divided they list runs in the order `-v4` changes.
        assert_eq!(
            ethos_parser_core::READING_ORDER_RULE_V3,
            "gutter-columns-v3",
            "an id under which artifacts were produced is frozen, not renamed"
        );
        // **`-v2` keeps its exact spelling for the reason the older two do.** Artifacts exist
        // under it, and it promises a region and no block. A spelling that moved would make one
        // of those and a `-v3` artifact look comparable while they promise different fields.
        assert_eq!(
            ethos_parser_core::READING_ORDER_RULE_V2,
            "gutter-columns-v2",
            "an id under which artifacts were produced is frozen, not renamed"
        );
        // **The older ids keep their exact spellings.** `single-column-v1` means content-stream
        // order and `gutter-columns-v1` means the same cut without the regions; artifacts exist
        // under both, and a spelling that moved under them would make an old artifact and a new
        // one look comparable while they promise different fields.
        assert_eq!(
            ethos_parser_core::READING_ORDER_RULE_V1,
            "gutter-columns-v1",
            "an id under which artifacts were produced is frozen, not renamed"
        );
        assert_eq!(ethos_parser_core::READING_ORDER_RULE_V0, "single-column-v1");
        for (a, b) in [
            (
                ethos_parser_core::READING_ORDER_RULE_V5,
                ethos_parser_core::READING_ORDER_RULE_V4,
            ),
            (
                ethos_parser_core::READING_ORDER_RULE_V4,
                ethos_parser_core::READING_ORDER_RULE_V3,
            ),
            (
                ethos_parser_core::READING_ORDER_RULE_V2,
                ethos_parser_core::READING_ORDER_RULE_V1,
            ),
            (
                ethos_parser_core::READING_ORDER_RULE_V1,
                ethos_parser_core::READING_ORDER_RULE_V0,
            ),
            (
                ethos_parser_core::READING_ORDER_RULE_V2,
                ethos_parser_core::READING_ORDER_RULE_V0,
            ),
        ] {
            assert_ne!(
                a, b,
                "two rules a consumer must tell apart cannot share an id"
            );
        }
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// Two of this function's eight document-level accumulators were still on a plain `+=`, and
    /// one of those also cast a `usize` count with `as u32` — which wraps in debug as well as
    /// release, so neither a test nor a CI job could have caught it. Both now saturate, because a
    /// count that comes back small is an artifact reporting that it erased almost nothing while it
    /// erased four billion things.
    /// The page `reorder_page`'s own doc comment argues about: a table beside a column of text,
    /// with the table's runs **interleaved** in the content stream so the remap is not a no-op.
    ///
    /// Interleaved deliberately. A page whose table runs are already contiguous would let a
    /// remap that did nothing still produce the right answer, and a guard that passes without the
    /// code under it is the defect this repository keeps finding.
    fn a_table_beside_a_column() -> (Vec<TextRun>, Vec<crate::tables::DetectedTable>, Vec<usize>) {
        use crate::tables::{QuantRect, RunOrigin};

        // Origin, text, in content-stream order. Two runs share cell (0,0), because a cell with
        // one run cannot tell a preserved concatenation from a lucky one.
        //
        // The grid is `grid_2x2()`'s: four 100pt cells spanning 0..20_000 centipoints on both
        // axes. The loose runs sit at x = 40_000, which is a gutter's width clear of the table's
        // right edge, so the rule really does cut this page into two columns.
        const PAGE: [(i64, i64, &str); 7] = [
            (40_000, 2_000, "R1"),  // loose, right column
            (2_000, 2_000, "He"),   // cell (0,0), first half
            (40_000, 12_000, "R2"), // loose, right column
            (12_000, 2_000, "b"),   // cell (0,1)
            (5_000, 2_000, "llo"),  // cell (0,0), second half
            (2_000, 12_000, "c"),   // cell (1,0)
            (12_000, 12_000, "d"),  // cell (1,1)
        ];

        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().expect("hashes"));
        let grid: Vec<QuantRect> = [(0, 0), (100, 0), (0, 100), (100, 100)]
            .iter()
            .map(|&(x, y)| {
                let q = i64::from(QUANTUM_PER_POINT);
                QuantRect {
                    x0: x * q,
                    y0: y * q,
                    x1: (x + 100) * q,
                    y1: (y + 100) * q,
                }
            })
            .collect();

        // The detector builds the cells, so `text` and `run_indices` are its output and not this
        // test's opinion of what they should be. That is what the comment claims survives.
        let origins: Vec<RunOrigin<'_>> = PAGE
            .iter()
            .map(|&(x, y, text)| RunOrigin { x, y, text })
            .collect();
        let tables = crate::tables::detect_ruled(1, &grid, &origins, &mut alloc)
            .expect("a well-formed grid detects")
            .0;
        assert_eq!(
            tables.len(),
            1,
            "the fixture must produce exactly one table"
        );

        let runs: Vec<TextRun> = PAGE
            .iter()
            .map(|&(x, y, text)| TextRun {
                read_beside: None,
                id: alloc.next(IdKind::Span).expect("ids"),
                region: None,
                block: None,
                layout_unit: None,
                inferred_heading: false,
                inferred_heading_level: None,
                bold: false,
                italic: false,
                weight: None,
                script: None,
                strikeout: false,
                underline: false,
                furniture: None,
                reading: None,
                text: text.to_string(),
                char_codes: text.chars().map(|c| c as u32).collect(),
                scalar_code_mismatch: false,
                synthesized: Vec::new(),
                font_id: "F1".into(),
                font_size: 1_200,
                locator: PdfLocator {
                    page: 1,
                    origin_x: x,
                    origin_y: y,
                    advance: None,
                },
                geometry: ethos_parser_core::GeometryPresence::Absent(
                    ethos_parser_core::GeometryAbsence::NotReportedByReader,
                ),
                mcid: None,
                structural: None,
                derivation: DerivationClass::Extracted,
                findings: Vec::new(),
                recognized: None,
            })
            .collect();

        let geometry: Vec<crate::reading_order::RunGeometry> = runs
            .iter()
            .map(|r| crate::reading_order::RunGeometry {
                x: r.locator.origin_x,
                y: r.locator.origin_y,
                advance: r.locator.advance,
            })
            .collect();
        let boxes: Vec<QuantRect> = tables.iter().map(|t| t.rect).collect();
        let order = crate::reading_order::order(&geometry, &boxes);

        (runs, tables, order)
    }

    /// **The property `reorder_page`'s comment argues and nothing asserted until v2-S14.1.**
    ///
    /// The comment claims a table's runs stay one atom across the reorder, so a cell's remapped
    /// indices stay ascending and *still concatenate to the `text` the detector built*. It named
    /// `cell_text_survives_the_reordering` as the proof from v1-S5 until v2-S13.5 found that no
    /// such test had ever been written: `git log --all -S` on the identifier returned the single
    /// commit that wrote the sentence.
    ///
    /// This asserts the claim itself rather than something adjacent to it. The tests that already
    /// existed cover cell-text **composition** before any reorder
    /// (`tables::tests::every_cell_text_is_a_concatenation_of_assigned_runs` and its siblings) and
    /// run **order** after one (`one_added_line_does_not_reorder_the_page`,
    /// `ordinals_and_ids_follow_the_reading_order_on_a_reordered_page`). Neither reads a cell's
    /// text on a page that moved, which is the one failure here that no artifact would show — a
    /// cell claiming text it does not contain.
    #[test]
    fn cell_text_survives_the_reordering() {
        let (mut runs, mut tables, order) = a_table_beside_a_column();

        // The floor, because a reorder that never happened proves nothing: `reorder_page` returns
        // early on the identity permutation, and every assertion below would then pass against a
        // page nothing touched.
        assert_ne!(
            order,
            (0..runs.len()).collect::<Vec<_>>(),
            "the fixture must actually reorder, or this guard reads nothing"
        );

        let before: Vec<(Vec<usize>, String)> = tables[0]
            .cells
            .iter()
            .map(|c| (c.run_indices.clone(), c.text.clone()))
            .collect();
        assert!(
            before.iter().any(|(idx, _)| idx.len() > 1),
            "at least one cell must hold two runs, or concatenation is untested"
        );
        assert!(
            before.iter().any(|(idx, _)| !idx.is_empty()),
            "a table whose cells hold no runs would assert nothing about text"
        );

        let mut op_indices: Vec<usize> = (0..runs.len()).collect();
        let mut ems = vec![None; runs.len()];
        reorder_page(&mut runs, &mut tables, &mut op_indices, &mut ems, &order);

        for (cell, (was, text)) in tables[0].cells.iter().zip(&before) {
            assert_eq!(&cell.text, text, "the reorder must not rewrite cell text");
            assert!(
                cell.run_indices.windows(2).all(|w| w[0] < w[1]),
                "a cell's remapped indices must stay strictly ascending: {:?} was {was:?}",
                cell.run_indices
            );
            let rebuilt: String = cell
                .run_indices
                .iter()
                .map(|&i| runs[i].text.as_str())
                .collect();
            assert_eq!(
                &rebuilt, text,
                "cell text must still be the runs it addresses, after the page moved: \
                 indices {:?} were {was:?}",
                cell.run_indices
            );
        }
    }

    /// **D4-S2: the region is attached before the renumber, and this is the only thing that says
    /// so.**
    ///
    /// `arrange_page` returns regions indexed by **stream** position, and `reorder_page` then
    /// moves whole `TextRun` values into reading order. Assigning one line later — after the move
    /// — would label every run with the region belonging to whatever run used to sit at its index,
    /// and the artifact would still be well-formed: every run would carry a plausible region, the
    /// count would be right, the numbers would be contiguous, and nothing downstream would
    /// disagree. A mislabelled page is indistinguishable from a correct one **on the wire**, which
    /// is why the check has to live here against the permutation rather than against an artifact.
    ///
    /// The guard is the one `cell_text_survives_the_reordering` uses: a fixture that does not
    /// actually reorder proves nothing, because the two orders coincide.
    #[test]
    fn the_region_follows_its_own_run_through_the_reordering() {
        let (mut runs, mut tables, order) = a_table_beside_a_column();
        assert_ne!(
            order,
            (0..runs.len()).collect::<Vec<_>>(),
            "the fixture must actually reorder, or stream and reading order coincide and this \
             test cannot tell a correct assignment from a late one"
        );

        let geometry: Vec<crate::reading_order::RunGeometry> = runs
            .iter()
            .map(|r| crate::reading_order::RunGeometry {
                x: r.locator.origin_x,
                y: r.locator.origin_y,
                advance: r.locator.advance,
            })
            .collect();
        let boxes: Vec<crate::tables::QuantRect> = tables.iter().map(|t| t.rect).collect();
        let arranged = crate::reading_order::arrange_page(&geometry, &boxes);
        assert!(
            !arranged.regions.is_empty(),
            "this page divides, or there is no region to follow"
        );

        // Remember which TEXT each region belongs to, before anything moves. Text is the handle
        // that survives the permutation; an index is exactly what does not.
        let expected: Vec<(String, Option<u32>)> = runs
            .iter()
            .zip(&arranged.regions)
            .map(|(r, region)| (r.text.clone(), *region))
            .collect();

        for (run, region) in runs.iter_mut().zip(&arranged.regions) {
            run.region = *region;
        }
        let mut op_indices: Vec<usize> = (0..runs.len()).collect();
        let mut ems = vec![None; runs.len()];
        reorder_page(
            &mut runs,
            &mut tables,
            &mut op_indices,
            &mut ems,
            &arranged.order,
        );

        for run in &runs {
            let (_, want) = expected
                .iter()
                .find(|(text, _)| *text == run.text)
                .expect("reordering is a permutation, so every text survives it");
            assert_eq!(
                run.region, *want,
                "`{}` came out of the reordering carrying region {:?}, but the cut put it in \
                 {:?} — the assignment is running on the wrong side of `reorder_page`",
                run.text, run.region, want
            );
        }

        // And the fixture is worth having: the table's column and the loose column really did
        // land in different regions, so a swap would have been visible above.
        let distinct: std::collections::BTreeSet<Option<u32>> =
            runs.iter().map(|r| r.region).collect();
        assert_eq!(
            distinct.len(),
            2,
            "the grid and the column beside it are two regions, got {distinct:?}"
        );
    }

    /// The atom argument the claim rests on, asserted separately from its consequence.
    ///
    /// A cell's text could survive by luck on a page whose table happened not to move. This says
    /// the table's runs really are contiguous in the new order and really do keep their relative
    /// sequence — the premise — so a future change that breaks the premise is reported here
    /// rather than only wherever it first happens to alter a string.
    #[test]
    fn a_tables_runs_are_contiguous_after_the_reorder() {
        let (mut runs, mut tables, order) = a_table_beside_a_column();
        let claimed: Vec<usize> = {
            let mut v: Vec<usize> = tables[0]
                .cells
                .iter()
                .flat_map(|c| c.run_indices.iter().copied())
                .collect();
            v.sort_unstable();
            v
        };
        assert!(claimed.len() >= 2, "the premise needs more than one run");

        let mut op_indices: Vec<usize> = (0..runs.len()).collect();
        let mut ems = vec![None; runs.len()];
        reorder_page(&mut runs, &mut tables, &mut op_indices, &mut ems, &order);

        let mut after: Vec<usize> = tables[0]
            .cells
            .iter()
            .flat_map(|c| c.run_indices.iter().copied())
            .collect();
        after.sort_unstable();
        assert_eq!(
            after.len(),
            claimed.len(),
            "no run may be lost or duplicated by the remap"
        );
        let (lo, hi) = (after[0], after[after.len() - 1]);
        assert_eq!(
            hi - lo + 1,
            after.len(),
            "the table's runs must occupy one unbroken span of the new order: {after:?}"
        );
        assert!(
            after.windows(2).all(|w| w[0] < w[1]),
            "and each exactly once: {after:?}"
        );
    }

    /// **Auto-tagging S2: the operator index follows its own run through the reordering.**
    ///
    /// The same shape as `the_region_follows_its_own_run_through_the_reordering`, for the same
    /// reason: an index left in stream order would still be a plausible vector of the right
    /// length, and the writer would tag the operator of whichever run used to occupy the slot.
    /// Text is the handle that survives the permutation, so the expectation is recorded by text.
    #[test]
    fn the_operator_index_follows_its_own_run_through_the_reordering() {
        let (mut runs, mut tables, order) = a_table_beside_a_column();
        assert_ne!(
            order,
            (0..runs.len()).collect::<Vec<_>>(),
            "the fixture must actually reorder, or a stale side table is indistinguishable from \
             a permuted one"
        );

        // Distinct, non-identity indices, so a vector left untouched or permuted the wrong way
        // round cannot coincide with the right answer.
        let mut op_indices: Vec<usize> = (0..runs.len()).map(|i| i * 7 + 3).collect();
        let expected: Vec<(String, usize)> = runs
            .iter()
            .zip(&op_indices)
            .map(|(r, &op)| (r.text.clone(), op))
            .collect();

        let mut ems = vec![None; runs.len()];
        reorder_page(&mut runs, &mut tables, &mut op_indices, &mut ems, &order);

        assert_eq!(
            op_indices.len(),
            runs.len(),
            "one index per run, after as before"
        );
        for (run, &op) in runs.iter().zip(&op_indices) {
            let (_, want) = expected
                .iter()
                .find(|(text, _)| *text == run.text)
                .expect("reordering is a permutation, so every text survives it");
            assert_eq!(
                op, *want,
                "`{}` came out of the reordering paired with operator {op}, but it was shown by \
                 operator {want}",
                run.text
            );
        }
    }

    /// **The public run carries no operator index** (docs/23-AUTO-TAGGING-SCOPE.md §6).
    ///
    /// `TextRun` is `deny_unknown_fields` and on the wire; the operator index is a join key into
    /// bytes a JSON consumer never sees, and a `TextRun` that carried it would let an artifact
    /// parsed from JSON reach the writer's placement rule. A source scan, because the absence of
    /// a field is not something a type can assert about itself.
    #[test]
    fn the_public_run_carries_no_operator_index() {
        let nodes = include_str!("nodes.rs");
        assert!(
            !nodes.contains("op_index"),
            "`op_index` reached nodes.rs: the operator index must stay in the crate-private side \
             table `extract_with_positions` returns, never on `TextRun`"
        );
        // And the side table really is where it lives, or the assertion above guards nothing.
        let here = include_str!("extract.rs");
        assert!(here.contains("pub(crate) type RunPositions"));
    }

    /// **Every position names a text-showing operation of its page, one per run.**
    ///
    /// On `leading-gap-two-blocks`, whose runs the block cut reorders into two blocks, the side
    /// table has exactly one entry per run and every entry indexes a `Tj`, `TJ`, `'` or `"` in
    /// lopdf's decode of the page — the same decode the interpreter ran. An index that pointed
    /// at a `Td` would be a run claiming to have been shown by an operator that shows nothing.
    #[test]
    fn every_run_position_indexes_a_text_showing_operation() {
        let bytes = crate::test_support::engine_fixture("leading-gap-two-blocks/document.pdf");
        let profile = Profile::default();
        let doc = Document::open_bytes(&bytes, &profile).expect("opens");
        let (artifact, positions) = extract_with_positions(&doc, &profile).expect("extracts");

        assert_eq!(
            artifact,
            extract(&doc, &profile).expect("extracts"),
            "asking for positions must not change the artifact"
        );
        assert_eq!(
            positions.len(),
            artifact.pages.len(),
            "one entry per processed page"
        );

        let mut checked = 0usize;
        for page in &artifact.pages {
            let ops = &positions
                .get(&page.index)
                .unwrap_or_else(|| panic!("page {} has no positions", page.index))
                .op_indices;
            assert_eq!(
                ops.len(),
                page.runs.len(),
                "page {}: one index per run",
                page.index
            );
            assert!(!page.runs.is_empty(), "the fixture page shows text");

            let page_id = doc
                .pages()
                .iter()
                .find(|(n, _)| *n == page.index)
                .map(|(_, id)| *id)
                .expect("the page exists");
            let decoded = lopdf::content::Content::decode(&doc.inner().get_page_content(page_id))
                .expect("the fixture's content decodes");
            for (run, &op) in page.runs.iter().zip(ops) {
                let operator = decoded
                    .operations
                    .get(op)
                    .map(|o| o.operator.as_str())
                    .unwrap_or_else(|| {
                        panic!("run `{}` indexes operation {op} past the end", run.text)
                    });
                assert!(
                    matches!(operator, "Tj" | "TJ" | "'" | "\""),
                    "run `{}` says operation {op} showed it, and that operation is `{operator}`",
                    run.text
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 6,
            "the fixture holds at least six runs, checked {checked}"
        );
    }

    #[test]
    fn a_document_level_erasure_count_saturates_rather_than_wrapping() {
        // `unclaimed_tree_items`: one per unanswered citation, folded across every page.
        let mut folded = u32::MAX - 1;
        for _ in 0..3 {
            folded = declare(folded, 1);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );

        // `mcids_unbound`: a per-page `usize` count folded in. `as u32` reported zero for exactly
        // `u32::MAX + 1`; the ceiling is the honest answer.
        let past = usize::try_from(u32::MAX).expect("64-bit") + 1;
        assert_eq!(declared_len(past), u32::MAX);
        assert_eq!(declare(u32::MAX - 1, declared_len(past)), u32::MAX);
    }

    /// Two Helvetica pages; the first carries `/Rotate first_rotate`.
    fn two_pages(first_rotate: i64) -> Vec<u8> {
        use lopdf::{dictionary, Object, Stream};
        let mut doc = lopdf::Document::with_version("1.7");
        let pages = doc.new_object_id();
        let font = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"
        });
        let mut kids = Vec::new();
        for (i, rotate) in [first_rotate, 0].into_iter().enumerate() {
            let text = format!("BT /F1 12 Tf 72 720 Td (page{i}) Tj ET").into_bytes();
            let content = doc.add_object(Stream::new(dictionary! {}, text));
            let page = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Rotate" => rotate,
                "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
                "Contents" => content,
            });
            kids.push(Object::Reference(page));
        }
        doc.objects.insert(
            pages,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 2 }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("saves");
        bytes
    }

    /// **A spent content budget is the document's refusal, ahead of any page's own error**
    /// (review 2026-09-25 F08).
    ///
    /// Pages run in parallel, so which page crossed the budget is timing, and the refusal must
    /// not be. Page 1 here fails on its own `/Rotate 45` before it reads its content, and the
    /// budget arrives spent: the budget answers. The failure this catches: the budget's refusal
    /// checked after the pages' errors, where a page's error would win or lose by scheduling.
    #[test]
    fn a_spent_content_budget_is_refused_ahead_of_any_pages_own_error() {
        let profile = Profile::default();
        let doc = Document::open_bytes(&two_pages(45), &profile).expect("opens");

        let e = extract_counted(
            &doc,
            &profile,
            &crate::budget::ContentBudget::default(),
            None,
            None,
        )
        .expect_err("page 1 refuses its rotation");
        assert!(e.to_string().contains("/Rotate"), "{e}");

        let spent = crate::budget::ContentBudget::default();
        let past = usize::try_from(crate::budget::MAX_DOCUMENT_OPERATIONS).expect("64-bit") + 1;
        assert!(spent.charge(0, past).is_err());
        let e =
            extract_counted(&doc, &profile, &spent, None, None).expect_err("the budget is spent");
        assert!(
            matches!(&e, EngineError::ResourceLimit { limit, .. }
                if limit == "content operations in the document's pages"),
            "{e}"
        );

        // Reading charges the budget: one at its ceiling is tipped over by a page's operations.
        let doc = Document::open_bytes(&two_pages(0), &profile).expect("opens");
        let full = crate::budget::ContentBudget::default();
        full.charge(0, past - 1).expect("exactly at the ceiling");
        let e =
            extract_counted(&doc, &profile, &full, None, None).expect_err("the pages tip it over");
        assert!(
            matches!(&e, EngineError::ResourceLimit { limit, .. }
                if limit == "content operations in the document's pages"),
            "{e}"
        );

        // And a document within every ceiling reads, pages in order.
        let (artifact, _) = extract_with_positions(&doc, &profile).expect("reads");
        let texts: Vec<&str> = artifact
            .pages
            .iter()
            .flat_map(|p| p.runs.iter().map(|r| r.text.as_str()))
            .collect();
        assert_eq!(texts, ["page0", "page1"]);
    }
}
