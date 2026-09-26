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
/// stored fixtures: only this build's own bytes are ever parsed back, in two round-trip tests.
pub const EXTRACT_SCHEMA_VERSION: &str = "0.5.0";

/// The extract artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractArtifact {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
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
    /// The outline the catalog declares, in `/First`/`/Next` order (`outlines-v1`).
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

/// The same ceiling for a count that arrives as a `usize`.
///
/// An `as u32` cast on an input-driven count is worse than a plain `+=`, because it wraps in debug
/// **and** release — so no test and no CI job can catch it, where `+=` at least panics in debug.
fn declared_len(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
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
/// actually answers *is this character right-to-left*, and this engine carries no Unicode
/// character database — so this reads the three ranges the right-to-left scripts occupy instead:
/// `U+0590`–`U+08FF` is Hebrew through Arabic Extended-A (Syriac, Thaana, NKo, Samaritan and
/// Mandaic among them), and the two presentation-form ranges follow.
///
/// It therefore also matches a few scalars in those blocks that are not themselves right-to-left
/// — an Arabic-Indic digit is `Bidi_Class` `AN`, not `R` or `AL`. That is the safe direction: the
/// limitation it fires is a statement about what this reader did NOT do, and declaring it on a
/// document that needed no reordering costs a consumer nothing, while missing one costs them a
/// quote that silently will not match. The limitation's own wording claims blocks, not classes,
/// so what it says is true of what this measures.
fn is_right_to_left_block(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
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

/// A page's candidate heading lines: each line's run indices in the final order, and the line
/// reduced to what the verdict needs (decision #29).
type HeadingLines = Vec<(Vec<usize>, crate::headings::Line)>;

/// One page's candidate heading lines (decision #29), and its share of the body em.
///
/// **The line is formed here and never in `headings.rs`.** A line is the runs sharing one band,
/// one `/Artifact` state and one baseline — `markdown.rs`'s `LineKey`, read for equality only —
/// and the rule's own file may not name the band: that is its guard, because it reads type and
/// never position. So this groups, and the rule sees each line's runs by their type alone.
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
    // Every line counts toward the reference (`type-size-v2`'s body is the largest size that runs
    // on many lines); only a line that could be a heading is kept for the verdict.
    let mut lines = Vec::new();
    for indices in by_line.into_values() {
        let members: Vec<Typed> = indices.iter().map(|&i| typed[i]).collect();
        tally.add_line(&members);
        let line = Line::of(&members);
        if line.is_candidate() {
            lines.push((indices, line));
        }
    }
    (lines, tally)
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
    tagged_without_geometric: Vec<u32>,
    unresolved_field_parents: u32,
    inline_images: u32,
    unresolved_xobjects: u32,
    undescended_xobjects: u32,
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
}

/// One page's whole extraction, exactly the body the sequential loop ran, with two
/// differences that the fold undoes: ids come from a page-local allocator (rewritten
/// into the document-global sequence afterwards), and cross-page accumulators are
/// returned as this page's deltas instead of mutated in place. Pure with respect to
/// the document handle, which is what lets pages run in parallel.
#[allow(clippy::too_many_lines)]
fn extract_page(
    doc: &Document,
    profile: &Profile,
    profile_sha256: &ethos_parser_core::Sha256Hex,
    structure: &Option<crate::structure::StructureTree>,
    tree_mcids_by_page: &std::collections::BTreeMap<lopdf::ObjectId, Vec<i64>>,
    page_number: u32,
    page_id: lopdf::ObjectId,
) -> Result<PageYield, EngineError> {
    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let mut encoding_dropped_runs: u32 = 0;
    let mut encoding_detail = String::new();
    let mut unruled_refusals: Vec<(u32, crate::unruled::Refusal)> = Vec::new();
    let mut ruled_refusals: Vec<(u32, crate::tables::RuledRefusal)> = Vec::new();
    let mut stroke_refusals: Vec<(u32, crate::stroke_ruled::Refusal)> = Vec::new();
    let mut mcids_unbound: u32 = 0;
    let mut unclaimed_tree_items: u32 = 0;
    let mut computed_bound: u32 = 0;
    let mut props_by_name: u32 = 0;
    let mut tagged_without_geometric: Vec<u32> = Vec::new();
    let mut unresolved_field_parents: u32 = 0;
    let mut inline_images: u32 = 0;
    let mut unresolved_xobjects: u32 = 0;
    let mut undescended_xobjects: u32 = 0;
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
    // The gate: the document declares no author structure (see `no_author_structure`), and the
    // profile names the rule — any other id, `not-run-for-this-format` included, runs nothing.
    let infer_headings = profile.heading_inference_rule
        == ethos_parser_core::HEADING_INFERENCE_RULE_V2
        && no_author_structure(structure.as_ref());
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
        // v1-S6. The frame an off-page finding is measured against, in the same coordinate system
        // the runs end up in. Computed once per page rather than per run.
        let visible = geom.visible_in_display_space();
        let fonts = load_page_fonts(doc, page_dict)?;

        composite_fonts = declare(
            composite_fonts,
            declared_len(
                fonts
                    .values()
                    .filter(|f| f.kind == crate::fonts::FontKind::Composite)
                    .count(),
            ),
        );

        for font in fonts.values() {
            let mut declare_once = |entry: ethos_parser_core::Limitation| {
                if !font_limitations.contains(&entry) {
                    font_limitations.push(entry);
                }
            };
            if let WidthSource::Absent { reason } = &font.widths {
                declare_once(lim::font_widths_absent(reason));
            }
            // v2.2-S6. `StandardEncoding` was applied to a font the specification does not give
            // it to. The characters still travel; the artifact now says they may be wrong.
            if let Some(detail) = &font.builtin_encoding_assumed {
                declare_once(lim::symbolic_font_builtin_encoding_assumed(detail));
            }
        }

        let operations = page_operations(doc.inner(), page_number, page_id)?;

        // v1-S6. The page's `/XObject` names, so `Do` can be resolved to an object number. The
        // interpreter still never holds a `Document` — it gets names and ids, and extraction
        // sorts `/Image` from `/Form` where the document is already in scope.
        let xobjects = crate::images::page_xobjects(doc.inner(), page_dict);
        let mut interp = Interpreter::new(&fonts).with_xobjects(xobjects);
        interp.run(&operations)?;
        inline_images = inline_images.saturating_add(interp.inline_images);
        unresolved_xobjects = unresolved_xobjects.saturating_add(interp.unresolved_xobjects);

        // v0.1: a font that cannot map a code drops its run rather than failing the document.
        // Accumulated across pages so the artifact declares one honest total.
        encoding_dropped_runs = encoding_dropped_runs.saturating_add(interp.dropped_runs);
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
        let mut runs = Vec::with_capacity(shown_runs.len());
        op_indices.reserve(shown_runs.len());
        ems.reserve(shown_runs.len());
        for shown in shown_runs {
            if shown.text.is_empty() {
                continue;
            }
            op_indices.push(shown.op_index);
            // A matrix with no vertical scale, or an em too large to quantize, is unmeasurable
            // rather than an error: the heading rule reads it as absent, never as zero.
            ems.push(
                quantize(shown.em_scale_pt, QUANTUM_PER_POINT)
                    .ok()
                    .filter(|em| *em > 0),
            );
            let font = fonts.get(&shown.font_id);

            let (ox_pt, oy_pt) = geom.to_top_left(shown.origin.0, shown.origin.1);
            let origin_x = quantize(ox_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let origin_y = quantize(oy_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let advance = shown
                .advance
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

            let synthesized: Vec<SynthesizedChar> = shown
                .synthesized_indices
                .iter()
                .map(|i| SynthesizedChar {
                    char_index: *i,
                    reason: SynthesisReason::TjGap,
                })
                .collect();

            let scalar_code_mismatch = shown.text.chars().count() != shown.codes.len();

            runs.push(TextRun {
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
                inferred_heading: false,
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
                // v1-S6. Observations about the run, never a reason to withhold it. Both are
                // computed from evidence the page supplies: the text rendering mode the content
                // stream set, and the visible box the page declares.
                findings: run_findings(shown.render_mode, origin_x, origin_y, &visible),
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
        let origins: Vec<crate::tables::RunOrigin<'_>> = runs
            .iter()
            .map(|r| crate::tables::RunOrigin {
                x: r.locator.origin_x,
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
        for seg in &interp.segments {
            let (ax, ay) = geom.to_top_left(seg.x0, seg.y0);
            let (bx, by) = geom.to_top_left(seg.x1, seg.y1);
            let r = crate::tables::quantize_rect(ax, ay, bx, by)?;
            if seg.is_horizontal() {
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
        let detected = crate::tables::detect(
            page_number,
            &table_rects,
            &stroke_rules,
            &uprights,
            &origins,
            &field_rects,
            &mut alloc,
        )?;
        let mut tables = detected.tables;

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
                        found.tagged_check =
                            Some(tagged.check_against(found.rows, found.columns, &positions));
                    }
                    // The tree says there is a table here and no detector found one. **Until
                    // v2-S24 no table was emitted** — a grid built from `/TD` elements alone would
                    // have cells this engine placed, and with nothing on the wire to distinguish
                    // them a consumer could not tell them from cells a detector reconstructed off
                    // the page. `DerivationClass` is now that distinction: the table is emitted as
                    // `Extracted` under `tagged-tables-v1` with geometry typed-absent, so it says
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

            reorder_page(
                &mut runs,
                &mut tables,
                &mut op_indices,
                &mut ems,
                &arranged.order,
            );
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
                    let mut text = String::new();
                    for &i in &idx {
                        text.push_str(&runs[i].text);
                    }
                    (idx, text)
                } else {
                    // A cell on another page of a multi-page table: its runs are not on this page,
                    // so it binds nothing here and carries the empty string. Honest rather than
                    // guessed — the page-granular emit cannot reach another page's runs.
                    (Vec::new(), String::new())
                };
                cells.push(crate::tables::TaggedCellRecord {
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
                rule: ethos_parser_core::TABLE_DETECTION_TAGGED_V1.to_string(),
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

        // v1-S6. One node per `Do`, in the order the page painted them. A `/Form` yields
        // nothing here — this profile does not descend into form XObjects, which stays declared
        // as `form-xobject-text-not-descended` — and neither does an XObject with no readable
        // `/Subtype`: emitting a node for an unlabelled stream would put a picture on the wire
        // the document never called one.
        let mut images = Vec::new();
        for placement in &interp.images {
            let Some(attributes) = crate::images::image_attributes(doc.inner(), placement.object)
            else {
                // v2.2-S2. **Counted, not merely skipped.** `image_attributes` returns `None` for
                // a `/Form` and for a stream with no readable `/Subtype`, and until now this
                // `continue` was the end of it: the placement was discarded and the artifact said
                // nothing. A page whose entire content is `q /Xf1 Do Q` — the shape a
                // page-slicing tool produces, and 4 of 104 sampled OmniDocBench documents — then
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
                undescended_xobjects = undescended_xobjects.saturating_add(1);
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
            images.push(crate::nodes::ImageRecord {
                id: alloc.next(IdKind::Image)?,
                locator: ethos_parser_core::PdfImageLocator {
                    page: page_number,
                    object: placement.object.0,
                    generation: u32::from(placement.object.1),
                    rect: crate::images::painted_rect(corners),
                },
                attributes,
            });
        }

        for run in &runs {
            for f in &run.findings {
                *findings_seen.entry(f.as_code()).or_insert(0) += 1;
            }
        }

        page_extract = PageExtract {
            tables,
            tagged_tables: page_tagged_tables,
            objects,
            images,
            index: page_number,
            width: quantize(geom.display_width, QUANTUM_PER_POINT).map_err(quantize_err)?,
            height: quantize(geom.display_height, QUANTUM_PER_POINT).map_err(quantize_err)?,
            rotation: geom.rotation,
            runs,
        };
    }
    Ok(PageYield {
        page: page_extract,
        ruled_refusals,
        stroke_refusals,
        unruled_refusals,
        encoding_dropped_runs,
        encoding_detail,
        mcids_unbound,
        unclaimed_tree_items,
        computed_bound,
        op_indices,
        heading_lines,
        em_tally,
        props_by_name,
        tagged_without_geometric,
        unresolved_field_parents,
        inline_images,
        unresolved_xobjects,
        undescended_xobjects,
        composite_fonts,
        findings_seen,
        font_limitations,
        local_ids: alloc,
    })
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
/// - [`EngineError::Malformed`] — operands of the wrong shape, or an unreadable page structure.
/// - [`EngineError::MissingPart`] — a font resource a `Tf` refers to is absent.
pub fn extract(doc: &Document, profile: &Profile) -> Result<ExtractArtifact, EngineError> {
    extract_with_positions(doc, profile).map(|(artifact, _)| artifact)
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
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| EngineError::Malformed {
            what: "profile".into(),
            detail: e.to_string(),
        })?;

    let mut alloc = IdAllocator::new(profile_sha256.clone());
    let mut pages = Vec::with_capacity(doc.pages().len());
    // Decision #29. Every page's candidate heading lines, aligned with `pages`, and the
    // document's rendered ems merged across pages — from which the body em is read once the
    // whole document has been.
    let mut heading_lines_by_page: Vec<HeadingLines> = Vec::new();
    let mut doc_em_tally = crate::headings::EmTally::default();
    let mut positions = RunPositions::new();
    let mut limitations = lim::extract_limitations();
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
    // `outlines-v1`. The other declaration this document may carry, read here for the same
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
    let mut tagged_without_geometric: Vec<u32> = Vec::new();
    // v1-S4. Widgets whose `/Parent` chain did not resolve. Counted, declared, never repaired.
    let mut unresolved_field_parents: u32 = 0;
    // v1-S6. Counted across pages, declared once, never repaired and never silently skipped.
    let mut inline_images: u32 = 0;
    let mut unresolved_xobjects: u32 = 0;
    let mut undescended_xobjects: u32 = 0;
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
    let outcomes: Vec<(u32, Option<Result<PageYield, EngineError>>)> = doc
        .pages()
        .par_iter()
        .map(|&(page_number, page_id)| {
            if !budget.admits(page_number) {
                return (page_number, None);
            }
            (
                page_number,
                Some(extract_page(
                    doc,
                    profile,
                    &profile_sha256,
                    &structure,
                    &tree_mcids_by_page,
                    page_number,
                    page_id,
                )),
            )
        })
        .collect();

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
            IdKind::Page | IdKind::Element | IdKind::Part => panic!(
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
            for _ in 0..y.local_ids.count(kind) {
                let _ = alloc.next(kind)?;
            }
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
        mcids_unbound = declare(mcids_unbound, y.mcids_unbound);
        unclaimed_tree_items = declare(unclaimed_tree_items, y.unclaimed_tree_items);
        computed_bound = declare(computed_bound, y.computed_bound);
        props_by_name = declare(props_by_name, y.props_by_name);
        tagged_without_geometric.extend(y.tagged_without_geometric);
        unresolved_field_parents = declare(unresolved_field_parents, y.unresolved_field_parents);
        inline_images = declare(inline_images, y.inline_images);
        unresolved_xobjects = declare(unresolved_xobjects, y.unresolved_xobjects);
        undescended_xobjects = declare(undescended_xobjects, y.undescended_xobjects);
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
    if let Some(body_em) = doc_em_tally.body_em() {
        let mut fired: u32 = 0;
        for (page, lines) in pages.iter_mut().zip(&heading_lines_by_page) {
            for (indices, line) in lines {
                if line.is_heading(body_em) {
                    fired = fired.saturating_add(1);
                    for &i in indices {
                        page.runs[i].inferred_heading = true;
                    }
                }
            }
        }
        if fired > 0 {
            limitations.push(lim::headings_inferred_from_type(
                fired,
                &profile.heading_inference_rule,
                body_em,
            ));
        }
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
    {
        let mut rtl_runs: u32 = 0;
        for page in &pages {
            for run in &page.runs {
                if run.text.chars().any(is_right_to_left_block) {
                    rtl_runs = rtl_runs.saturating_add(1);
                }
            }
        }
        if rtl_runs > 0 {
            limitations.push(lim::right_to_left_not_reordered(rtl_runs));
        }
    }
    if props_by_name > 0 {
        limitations.push(lim::mcid_property_list_by_name(props_by_name));
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
        source: SourceRef {
            media_type: "application/pdf".to_string(),
            sha256: Sha256Hex::parse(doc.source_sha256().as_str())?,
        },
        reading_order_rule: profile.reading_order_rule.clone(),
        page_count,
        pages,
        outlines: outline.records,
        assurance: Assurance::new(profile.capabilities, page_count, page_states, limitations)?,
    };
    Ok((artifact, positions))
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
        let operations = page_operations(doc.inner(), page_number, page_id)?;
        let xobjects = crate::images::page_xobjects(doc.inner(), page_dict);
        let mut interp = Interpreter::new(&fonts).with_xobjects(xobjects);
        interp.run(&operations)?;

        // Origins, exactly as `extract` builds them: every non-empty shown run at its top-left
        // origin, quantized. Owned first so the borrowed `RunOrigin` view stays valid for `detect`.
        let mut owned: Vec<(i64, i64, String)> = Vec::with_capacity(interp.shown.len());
        for shown in &interp.shown {
            if shown.text.is_empty() {
                continue;
            }
            let (ox_pt, oy_pt) = geom.to_top_left(shown.origin.0, shown.origin.1);
            let origin_x = quantize(ox_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            let origin_y = quantize(oy_pt, QUANTUM_PER_POINT).map_err(quantize_err)?;
            owned.push((origin_x, origin_y, shown.text.clone()));
        }
        let origins: Vec<crate::tables::RunOrigin<'_>> = owned
            .iter()
            .map(|(x, y, t)| crate::tables::RunOrigin {
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

        // Ruling segments, split by orientation exactly as `extract` splits them.
        let mut stroke_rules = Vec::new();
        let mut uprights = Vec::new();
        for seg in &interp.segments {
            let (ax, ay) = geom.to_top_left(seg.x0, seg.y0);
            let (bx, by) = geom.to_top_left(seg.x1, seg.y1);
            let r = crate::tables::quantize_rect(ax, ay, bx, by)?;
            if seg.is_horizontal() {
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
            &stroke_rules,
            &uprights,
            &origins,
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
/// Both findings come from evidence the page itself supplies — the rendering mode its content
/// stream set, and the box it declares as visible. Neither is an appearance judgement, and
/// neither removes anything: the run this describes is in the artifact with its text and its
/// origin intact, which is the whole of checklist O21.
fn run_findings(
    render_mode: i64,
    origin_x: i64,
    origin_y: i64,
    visible: &PageBox,
) -> Vec<TextFinding> {
    let mut out = Vec::new();
    // Modes 3 and 7 fill nothing and stroke nothing (32000-1 Table 106). Mode 7 also adds the
    // glyphs to the clip path, which is a different purpose and the same visible result: no ink.
    // Mode 4 through 6 DO paint and are deliberately not flagged — a rule that called every
    // clipping mode invisible would report ordinary text as hidden.
    if render_mode == 3 || render_mode == 7 {
        out.push(TextFinding::InvisibleRenderMode);
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
/// **Measured on a corpus, 2026-09-18.** Over OmniDocBench's 981 born-digital `v1_0` pages this
/// refuses one document, `jiaocaineedrop_chap10.pdf_8.pdf`, whose page content `lopdf` stops
/// reading at byte 4 of 125 718: the build before this check exits 0 on it and writes an artifact
/// whose two nodes carry no text, a complete-looking read of a page nobody read
/// (`docs/measurements/omnidocbench/README.md`). It is the first corpus example of the defect;
/// the other corpora this repository can reach hold none.
///
/// # Errors
///
/// - [`EngineError::Malformed`] with `what` = `content stream`, naming the page and the stream:
///   a `FlateDecode` stream whose deflate data does not reach its end.
/// - [`EngineError::Unsupported`] with `what` = `content stream tokeniser`, naming the page and
///   the byte: an operation `lopdf`'s grammar cannot parse with bytes after it, a shape its
///   decoder fails the whole page on, or one it panics on.
pub(crate) fn page_operations(
    doc: &lopdf::Document,
    page_number: u32,
    page_id: lopdf::ObjectId,
) -> Result<Vec<lopdf::content::Operation>, EngineError> {
    let mut content = Vec::new();
    for id in doc.get_page_contents(page_id) {
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
                        "page {page_number}: /Contents names {} {} R, which the cross-reference \
                         table lists but which did not load as a stream",
                        id.0, id.1
                    ),
                })
            }
        };
        let refuse_filter = |detail: String| EngineError::Unsupported {
            what: "content stream filter".into(),
            detail: format!("page {page_number}, stream {} {}: {detail}", id.0, id.1),
        };
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
                // Filters that do not decode fail the whole chain, which is refused below.
                let Ok(bytes) = before.decompressed_content() else {
                    break;
                };
                decoded_before = bytes;
                &decoded_before
            };
            crate::tagging::deflate_reaches_its_end(input).map_err(|detail| {
                EngineError::Malformed {
                    what: "content stream".into(),
                    detail: format!("page {page_number}, stream {} {}: {detail}", id.0, id.1),
                }
            })?;
        }
        // Decoded here rather than through `get_page_content`, whose fallback for a filter it
        // cannot decode is the stream's raw bytes: text the stream's own filter says is not there.
        // An empty chain is no filter, so the stream's own bytes, which `lopdf` decodes to none.
        let decoded = if filters.is_empty() {
            Ok(stream.content.clone())
        } else {
            stream.decompressed_content()
        };
        let bytes = decoded.map_err(|_| {
            let chain: Vec<String> = filters
                .iter()
                .map(|f| format!("/{}", String::from_utf8_lossy(f)))
                .collect();
            refuse_filter(format!(
                "the filter chain [{}] did not decode, and its raw bytes are not what the \
                 page draws",
                chain.join(" ")
            ))
        })?;
        content.extend_from_slice(&bytes);
        content.push(b'\n');
    }
    let on_page = |e: EngineError| match e {
        EngineError::Unsupported { what, detail } => EngineError::Unsupported {
            what,
            detail: format!("page {page_number}: {detail}"),
        },
        other => other,
    };
    let tokens = crate::tagging::tokenise(&content).map_err(on_page)?;
    let decoded =
        lopdf::content::Content::decode(&content).map_err(|e| EngineError::Malformed {
            what: "content stream".into(),
            detail: format!("page {page_number}: {e}"),
        })?;
    crate::tagging::agrees_with_lopdf(&tokens, &decoded.operations).map_err(on_page)?;
    Ok(decoded.operations)
}

/// Whether the cross-reference table lists `id`, its generation included, as an object in use.
fn in_use(doc: &lopdf::Document, id: lopdf::ObjectId) -> bool {
    use lopdf::xref::XrefEntry;
    match doc.reference_table.entries.get(&id.0) {
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
        match self.rotation {
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
        match self.rotation {
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
        match self.rotation {
            90 => (y + b.x0, x + b.y0),
            180 => (b.x1 - x, y + b.y0),
            270 => (b.x1 - y, b.y1 - x),
            _ => (x + b.x0, b.y1 - y),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn geom(w: f64, h: f64, rot: i64) -> PageGeometry {
        let (dw, dh) = if rot % 180 == 0 { (w, h) } else { (h, w) };
        let media = PageBox::from_corners(0.0, 0.0, w, h);
        PageGeometry {
            media,
            visible: media,
            rotation: rot,
            display_width: dw,
            display_height: dh,
        }
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
            ethos_parser_core::READING_ORDER_RULE_V3,
            "the block cut moved the default to the id that promises the block field too"
        );
        assert_eq!(
            ethos_parser_core::READING_ORDER_RULE_V3,
            "gutter-columns-v3",
            "the id is data on every artifact; changing the string is an identity event"
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
                id: alloc.next(IdKind::Span).expect("ids"),
                region: None,
                block: None,
                inferred_heading: false,
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
}
