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

//! The content-stream interpreter (`docs/history/05-MILESTONES.md` M3).
//!
//! # Exhaustive dispatch, and what that buys
//!
//! [`Interpreter::run`] matches on [`Operator`] with **no wildcard arm**. Three consequences:
//!
//! 1. Adding a variant to `Operator` without handling it here is a **compile error**.
//! 2. A token that is not in the table is a **hard error** naming it — never a skip.
//! 3. Operators this profile does not need for text are *named* and acknowledged, so "we do not
//!    care about `rg`" is a decision in the source rather than an accident of a `_ =>` arm.
//!
//! pdf-inspector's match omits the `"` show-text operator entirely: its text vanishes, adjacent
//! runs merge with corrupt geometry, and the output is still well-formed JSON — undetectable
//! downstream (parity checklist P6). That failure is only possible with a permissive default.

use ethos_parser_core::EngineError;

use crate::fonts::Font;
use crate::ops::Operator;
use crate::text_state::{GraphicsState, Matrix, TextState};

/// How much a `TJ` adjustment must open a gap before it counts as a word space.
///
/// In thousandths of an em, negative meaning "move right". −180 is roughly a fifth of an em: wide
/// enough that ordinary kerning and justification tracking do not trip it, narrow enough to catch
/// a real inter-word gap.
///
/// Changing it changes output, so it is calibration and a `parser_version` event — the same
/// reasoning as [`crate::thresholds`].
pub const TJ_SPACE_GAP_THOUSANDTHS: f64 = -180.0;

/// A gap only ever *opens* to the right, so the threshold must be negative. A `const` assertion
/// rather than a test: comparing two constants is decided at compile time, so a runtime
/// `assert!` would be optimized away and prove nothing.
const _: () = assert!(
    TJ_SPACE_GAP_THOUSANDTHS < 0.0,
    "a positive TJ adjustment closes a gap and must never synthesize a space"
);

/// One decoded show-text event, before it becomes a node.
#[derive(Debug, Clone)]
pub struct ShownText {
    /// Decoded characters.
    pub text: String,
    /// Codes that produced them.
    pub codes: Vec<u32>,
    /// Baseline origin in **user space**, before the page transform.
    pub origin: (f64, f64),
    /// Advance in user space, or `None` when the font carries no widths.
    ///
    /// A length along the text matrix's x only, scaled by `ctm.x_scale()`: it keeps no direction,
    /// so a run turned by its text matrix advances 0 or less here. [`Self::displacement`] is the
    /// same travel as a vector, and it is what a box is built from.
    pub advance: Option<f64>,
    /// The pen's travel over the run in **user space**, as a vector, or `None` exactly when
    /// [`Self::advance`] is.
    ///
    /// PDF 32000-1 §9.4.4 moves the text matrix by `[1 0 0 1 tx 0] × Tm` after each glyph, so the
    /// travel is a text-space vector that the text matrix turns and the CTM carries on. Summing
    /// only `Tm.e` kept its x and lost its y, and `ctm.x_scale()` kept the CTM's length and lost
    /// its direction: text drawn up the page advanced 0, and text drawn under a turned CTM got a
    /// box along x. On upright text this is bit for bit `(advance, ±0)`.
    pub displacement: Option<(f64, f64)>,
    /// The rendered glyph's y axis in user space: the text rendering matrix's second row,
    /// `(c, d)`.
    ///
    /// Which side of the baseline the ascent lies on. Upright glyphs point it +y.
    pub glyph_up: (f64, f64),
    /// The same advance, per code, in the same space — or `None` on the same condition.
    ///
    /// **Aligned with [`Self::codes`], not with [`Self::text`].** The *n*th entry is the advance of
    /// the *n*th code, and a code may decode to more than one character (a ligature code, or any
    /// `ToUnicode` entry mapping to a string), so indexing this by a character offset is wrong on
    /// any run holding such a code. `extract.rs`'s `scalar_code_mismatch` is not the test for that:
    /// it compares two counts, so a synthesized space sets it on a run whose codes are all single,
    /// and a code whose `ToUnicode` destination is empty — which `cmap.rs` accepts — decodes to no
    /// character and can offset one that decodes to several. Nothing here records how many
    /// characters each code produced.
    ///
    /// `Some` only when [`Self::advance`] is `Some`, and then `len() == codes.len()` and the
    /// entries sum to it. The two travel together because a code with no width advances nothing
    /// and contributes no entry: a partial vector would look complete and silently misalign every
    /// entry after the gap.
    ///
    /// Nothing on the wire reads this yet. It exists because the per-glyph deltas are already
    /// computed here and summing them away is lossy, and a sub-run box cannot be recovered
    /// afterwards from the total.
    pub code_advances: Option<Vec<f64>>,
    /// Font resource name.
    pub font_id: String,
    /// Font size in text space — the raw `Tf` operand, as the document states it.
    ///
    /// **Not the rendered size**, and not what an ink box may be built from. A page may set
    /// `Tf /F 1` and carry the type size in the text matrix or the CTM, in which case this is 1
    /// and the glyphs are 10pt. Use [`Self::em_scale_pt`] for anything geometric; this stays the
    /// operand because the operand is what the document says.
    pub font_size: f64,
    /// The rendered height of one em, in the same space as [`Self::origin`].
    ///
    /// The vertical scale of the text rendering matrix — `Tfs` composed with the text matrix and
    /// the CTM, per §9.4.4 — so ascent and descent scaled by this land in the space the origin is
    /// already in. `origin` is `(trm.e, trm.f)` and the box's length along its baseline is
    /// [`Self::displacement`], carried through the CTM; this is the third side of that same
    /// triangle, and without it the box's height and width were in different spaces.
    pub em_scale_pt: f64,
    /// Marked-content id in force, if any.
    pub mcid: Option<i64>,
    /// Whether **any** enclosing marked-content sequence is an `/Artifact` (v1-S3).
    ///
    /// The whole stack, not the innermost frame: an artifact sequence with a `/Span` nested
    /// inside it is still artifact content, and asking only the innermost frame would call that
    /// span body text.
    pub artifact: bool,
    /// Indices into `text` this reader inserted.
    pub synthesized_indices: Vec<u32>,
    /// The text rendering mode in force when this run was shown (v1-S6).
    ///
    /// `Tr` was tracked in the text state from v0 onward and **read by nothing** — so a run
    /// painted in mode 3 arrived at a consumer indistinguishable from visible prose. The text was
    /// never dropped, which is the half that mattered; what was missing is that anyone could
    /// tell. This carries the mode out of the interpreter so extraction can flag it.
    pub render_mode: i64,
    /// The index, in the slice [`Interpreter::run`] received, of the operation that showed this
    /// text (auto-tagging S2).
    ///
    /// The writer's join key back into the page's content stream: a `Tj` is one operation and
    /// one run, a `TJ` is one operation and as many runs as it holds strings, so several runs
    /// may share one index and no index is ever skipped by a run that was shown. Crate-private
    /// and never on the wire — `TextRun` does not carry it and `ExtractArtifact` does not change
    /// (docs/23-AUTO-TAGGING-SCOPE.md §6); it leaves this crate only through
    /// `extract::extract_with_positions`.
    pub(crate) op_index: usize,
}

/// One image XObject a page painted with `Do`, in **user space** (v1-S6).
///
/// The rectangle is the current transformation matrix applied to the unit square every PDF image
/// is defined on (32000-1 §8.9.5.2), so it is the area the placement actually covered — not the
/// bitmap's pixel dimensions, which are a different quantity in different units.
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePlacement {
    /// The XObject's own object id, so a consumer can tell one picture placed five times from
    /// five pictures placed once.
    pub object: lopdf::ObjectId,
    /// The four corners of the transformed unit square, in user space, in the order
    /// `(0,0) (1,0) (1,1) (0,1)`.
    ///
    /// All four rather than a bounding box, because whether the placement is axis-aligned is
    /// decided from them: a rotated image covers a parallelogram, and its bounding box would
    /// claim page area the picture does not cover.
    pub corners: [(f64, f64); 4],
}

/// One axis-aligned rectangle a page's path operators drew, in **user space**.
///
/// v1-S1. Captured because a ruled table is a table the document *drew*: the ruling lines are
/// evidence, not an inference about layout. `docs/history/08-V1-SCOPE.md` §5 is why that distinction is
/// worth a slice boundary.
///
/// Held as `f64` here and quantized on the way to the artifact, exactly as a glyph origin is —
/// `docs/01-CONTRACT.md` §4 keeps floats off the wire, not out of the interpreter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathRect {
    /// Smaller x, in user space.
    pub x0: f64,
    /// Smaller y.
    pub y0: f64,
    /// Larger x.
    pub x1: f64,
    /// Larger y.
    pub y1: f64,
}

/// One **axis-aligned two-point stroked segment** the page painted, in user space (v1-S8).
///
/// # Not a rectangle, and kept apart from one on purpose
///
/// A `re` or a four-point closed path encloses an area: it is the author saying *this box is
/// here*. A two-point `m`/`l` stroked with `S` is a **line**: the author saying *this edge is
/// here*. `ruled-rects-v1` was built on the first and produces nothing from the second, which is
/// the `stroke-ruled-tables-not-detected` limitation the engine declared from v1-S1 to v1-S7b.
///
/// **It is retired.** v1-S8 shipped `stroke-ruled-v1` and removed the code rather than rewording
/// it, on the rule `assurance.rs` states — a stale limitation is worse than a missing one because
/// a reader acts on it — and `extraction.rs` now asserts its ABSENCE. This sentence said "has
/// declared since v1-S1" until v2-S13.5, written in the very commit that retired it.
///
/// Measured, that limitation costs `cfpb-home-loan-toolkit` 103 of its 159 gold cells: its page 13
/// draws an 8 × 4 loan worksheet as 32 horizontal rules and nothing else. Collapsing a segment
/// into a zero-height `PathRect` would have let the ruled lattice consume it, and a zero-area
/// "cell" is a cell nobody drew — so the two kinds of evidence stay in two fields and a separate
/// rule reads this one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathSegment {
    /// Smaller x, in user space.
    pub x0: f64,
    /// Smaller y.
    pub y0: f64,
    /// Larger x.
    pub x1: f64,
    /// Larger y.
    pub y1: f64,
}

impl PathSegment {
    /// Whether this segment runs left-to-right. A segment is one or the other, never both: a
    /// zero-length one is not captured at all.
    pub fn is_horizontal(self) -> bool {
        approx_eq(self.y0, self.y1)
    }
}

impl PathRect {
    fn from_corners(a: (f64, f64), b: (f64, f64)) -> Self {
        Self {
            x0: a.0.min(b.0),
            y0: a.1.min(b.1),
            x1: a.0.max(b.0),
            y1: a.1.max(b.1),
        }
    }
}

/// One open marked-content sequence (v1-S3).
///
/// v0 tracked only the id, which was enough to copy an `/MCID` onto a run and not enough for
/// anything else. Keeping the tag as well is what lets `/Artifact` be told from `/P` — and the
/// alternative to telling them apart is either calling running heads body text or deleting them,
/// the second being an undeclared edit to the document.
#[derive(Debug, Clone, Copy)]
struct MarkedContent {
    /// Whether this sequence's tag is `/Artifact`.
    artifact: bool,
    /// Its `/MCID`, when the property list was inline and carried one.
    mcid: Option<i64>,
}

/// A subpath under construction.
///
/// Tracks only what is needed to decide "is this an axis-aligned rectangle": the points, and
/// whether anything disqualifying happened. A curve or a diagonal disqualifies it permanently —
/// **the alternative is tessellating a Bézier into fake ruling lines**, which would invent a
/// table edge the document never drew.
#[derive(Debug, Default)]
struct Subpath {
    points: Vec<(f64, f64)>,
    disqualified: bool,
}

/// Interprets one page's content stream.
pub struct Interpreter<'a> {
    fonts: &'a std::collections::BTreeMap<String, std::sync::Arc<Font>>,
    /// The page's `/XObject` resources, by resource name (v1-S6).
    ///
    /// `None` when the caller supplied none, which is not the same as an empty map: an empty map
    /// means the page declares no XObjects, and `None` means nobody looked. A `Do` under `None`
    /// counts as unresolved rather than silently succeeding.
    xobjects: Option<std::collections::BTreeMap<String, lopdf::ObjectId>>,
    gs_stack: Vec<GraphicsState>,
    gs: GraphicsState,
    ts: TextState,
    mc_stack: Vec<MarkedContent>,
    /// Runs collected so far.
    pub shown: Vec<ShownText>,
    /// Whether a string has put at least one code on the page since the pen was last placed.
    ///
    /// A `TJ` number moves the pen from where it stands (PDF 32000-1 §9.4.3), so it opens a gap
    /// after text only when text was drawn since. `BT`, `Td`, `TD`, `T*`, `Tm`, the line move of
    /// `'` and `"`, and a `cm` or `Q` that changes the CTM place the pen and clear this. Every
    /// string with codes sets it, including a run later dropped for an undecodable code.
    ///
    /// A dropped run is not in [`Self::shown`], and since 2026-09-18 its pen travels over every
    /// one of its codes anyway, so a later run in the same text object sits where the document
    /// draws it. The gap after it is written onto the last run kept, as before, because
    /// suppressing it would fuse two words across text the reader lost.
    text_drawn_since_placement: bool,
    /// Codes the decoder could not map, as a typed diagnostic rather than a silent drop.
    pub undecodable: Vec<String>,
    /// Axis-aligned rectangles this page painted, in user space (v1-S1).
    ///
    /// Only **painted** subpaths land here. A path built and then discarded with `n`, or used
    /// solely as a clip, drew no ink and is not a ruling line.
    pub rects: Vec<PathRect>,
    /// Axis-aligned two-point stroked segments this page painted, in user space (v1-S8).
    ///
    /// Only **painted** ones, exactly as for [`Interpreter::rects`]: a path ended with `n` or used
    /// as a clip drew no ink and is not a ruling line.
    pub segments: Vec<PathSegment>,
    /// The subpath being built.
    subpath: Subpath,
    /// Rectangles from the current path, pending a painting operator.
    pending: Vec<PathRect>,
    /// Segments from the current path, pending a painting operator (v1-S8).
    pending_segments: Vec<PathSegment>,
    /// How many runs were dropped because a font could not map one of their codes.
    ///
    /// Separate from [`Interpreter::undecodable`]'s length: one run can carry several
    /// unmappable codes, and what a caller needs to know is how many pieces of text are
    /// **missing from the artifact**, not how many diagnostics were produced.
    pub dropped_runs: u32,
    /// How many `BDC` sequences supplied their property list **by name** (v1-S3).
    ///
    /// Those may carry an `/MCID` this profile did not resolve, so their content can look
    /// untagged when it is not. Counted rather than assumed away.
    pub props_by_name: u32,
    /// Image XObjects this page painted, in the order it painted them (v1-S6).
    pub images: Vec<ImagePlacement>,
    /// `Do` calls naming an XObject this profile could not resolve (v1-S6).
    ///
    /// A name absent from `/Resources /XObject`, or a resource that is not a stream. Counted and
    /// declared rather than refused: a `Do` whose name does not resolve drew nothing this reader
    /// can describe, and rejecting the whole document over it would turn files that parse today
    /// into failures. Counted rather than ignored, because "no image nodes" must not be able to
    /// mean "there were images and the reader lost them".
    pub unresolved_xobjects: u32,
    /// Inline images (`BI`/`ID`/`EI`) this page drew (v1-S6).
    ///
    /// Not emitted as nodes: an inline image has no object number to address and no independent
    /// stream to digest, because its samples live in the content stream itself. Counted so the
    /// absence of a node for it is visible on the artifact.
    pub inline_images: u32,
    /// The index of the operation [`Self::run_inner`] is dispatching, recorded before every
    /// dispatch and read by [`Self::show`] (auto-tagging S2).
    ///
    /// On the interpreter rather than threaded through `dispatch` as an argument, because the
    /// four text-showing arms reach `show` through two helpers and the index is a fact about the
    /// loop, not about any operand.
    op_index: usize,
}

impl<'a> Interpreter<'a> {
    /// Start an interpreter over a page's fonts.
    pub fn new(fonts: &'a std::collections::BTreeMap<String, std::sync::Arc<Font>>) -> Self {
        Self {
            fonts,
            gs_stack: Vec::new(),
            gs: GraphicsState::default(),
            ts: TextState::default(),
            mc_stack: Vec::new(),
            shown: Vec::new(),
            text_drawn_since_placement: false,
            undecodable: Vec::new(),
            rects: Vec::new(),
            segments: Vec::new(),
            subpath: Subpath::default(),
            pending: Vec::new(),
            pending_segments: Vec::new(),
            dropped_runs: 0,
            props_by_name: 0,
            images: Vec::new(),
            unresolved_xobjects: 0,
            inline_images: 0,
            xobjects: None,
            op_index: 0,
        }
    }

    /// Supply the page's `/XObject` resource dictionary, so `Do` can be resolved (v1-S6).
    ///
    /// Separate from [`Self::new`] because a caller that has no resources — every unit test in
    /// this module — must still be able to build an interpreter, and because it makes the
    /// dependency visible at the one call site that has a document to hand.
    pub fn with_xobjects(
        mut self,
        xobjects: std::collections::BTreeMap<String, lopdf::ObjectId>,
    ) -> Self {
        self.xobjects = Some(xobjects);
        self
    }

    /// The `/MCID` of the **innermost** open sequence.
    ///
    /// Innermost rather than nearest-enclosing-with-an-id, which is the v0 behaviour preserved
    /// verbatim. A nested sequence that declares no id is its own content item, and reaching past
    /// it to borrow an ancestor's id would file this text under a structure element that does not
    /// claim it.
    fn current_mcid(&self) -> Option<i64> {
        self.mc_stack.last().and_then(|m| m.mcid)
    }

    /// Whether any open sequence is an `/Artifact`.
    ///
    /// The whole stack: artifact content stays artifact content however deeply it is nested.
    fn inside_artifact(&self) -> bool {
        self.mc_stack.iter().any(|m| m.artifact)
    }

    /// Interpret a decoded content stream.
    ///
    /// # Errors
    ///
    /// - [`EngineError::Unsupported`] — a token that is not in PDF 32000-1 Table A.1. **This is
    ///   the fail-closed rule**: an unrecognised operator stops the parse rather than being
    ///   skipped, because a skipped operator can silently move or delete text.
    /// - [`EngineError::Malformed`] — an operator whose operands are the wrong shape.
    pub fn run(&mut self, ops: &[lopdf::content::Operation]) -> Result<(), EngineError> {
        match self.run_inner(ops) {
            Ok(()) => Ok(()),
            Err(e) => {
                // **Fail closed means the partial output goes too.** Through M6 this method
                // returned `Err` and left everything shown before the bad operator sitting in
                // `self.shown`. Nothing read it — `extract` propagates with `?` and drops the
                // interpreter — so no artifact was ever wrong. But the guarantee was the call
                // site's, not the type's, and the test that was supposed to cover it passed
                // only because it never showed text before the refusal (M7 triage).
                //
                // Discarding here makes it the type's guarantee: there is no state a later
                // caller could read and mistake for a complete parse of a document this
                // interpreter refused.
                self.shown.clear();
                self.undecodable.clear();
                self.rects.clear();
                self.segments.clear();
                self.pending.clear();
                self.subpath = Subpath::default();
                self.dropped_runs = 0;
                Err(e)
            }
        }
    }

    fn run_inner(&mut self, ops: &[lopdf::content::Operation]) -> Result<(), EngineError> {
        for (index, op) in ops.iter().enumerate() {
            self.op_index = index;
            let token = op.operator.as_str();
            let Some(operator) = Operator::from_token(token) else {
                return Err(EngineError::Unsupported {
                    what: "pdf operator".into(),
                    detail: format!(
                        "`{token}` is not in PDF 32000-1 Table A.1. Refusing rather than skipping: \
                         an unrecognised operator may move or delete text, and skipping it \
                         produces a well-formed artifact that is silently wrong."
                    ),
                });
            };
            self.dispatch(operator, &op.operands)?;
        }
        Ok(())
    }

    /// Whether the current CTM maps axis-aligned rectangles to axis-aligned rectangles.
    ///
    /// True for scale-and-translate (and axis-swapping 90° rotations). False for any skew or
    /// off-axis rotation, where a rectangle's image is a parallelogram and its bounding box would
    /// be four edges the document did not draw.
    fn ctm_is_axis_aligned(&self) -> bool {
        let m = self.gs.ctm;
        (approx_eq(m.b, 0.0) && approx_eq(m.c, 0.0)) || (approx_eq(m.a, 0.0) && approx_eq(m.d, 0.0))
    }

    /// Turn the subpath under construction into a rectangle, if it is one.
    ///
    /// A rectangle is four or five points (the fifth closing back to the first), all segments
    /// axis-aligned, spanning exactly two distinct x values and two distinct y values. Anything
    /// else — an L, a triangle, a polyline, a curve — is discarded rather than approximated.
    fn flush_subpath(&mut self) {
        let sub = std::mem::take(&mut self.subpath);
        if sub.disqualified || !self.ctm_is_axis_aligned() {
            return;
        }
        let pts = &sub.points;
        // v1-S8. A two-point axis-aligned run is a ruling LINE, not a box. Captured separately —
        // see `PathSegment`. A zero-length one is a `m`/`l` pair that moved nowhere and drew no
        // edge, so it is not evidence of anything and is dropped.
        if pts.len() == 2 {
            let (a, b) = (pts[0], pts[1]);
            let horizontal = approx_eq(a.1, b.1) && !approx_eq(a.0, b.0);
            let vertical = approx_eq(a.0, b.0) && !approx_eq(a.1, b.1);
            if horizontal || vertical {
                self.pending_segments.push(PathSegment {
                    x0: a.0.min(b.0),
                    y0: a.1.min(b.1),
                    x1: a.0.max(b.0),
                    y1: a.1.max(b.1),
                });
            }
            return;
        }
        if pts.len() < 4 || pts.len() > 5 {
            return;
        }
        let xs: Vec<f64> = distinct(pts.iter().map(|p| p.0));
        let ys: Vec<f64> = distinct(pts.iter().map(|p| p.1));
        if xs.len() != 2 || ys.len() != 2 {
            return;
        }
        self.pending
            .push(PathRect::from_corners((xs[0], ys[0]), (xs[1], ys[1])));
    }

    /// The exhaustive match. **No wildcard arm.**
    fn dispatch(&mut self, op: Operator, operands: &[lopdf::Object]) -> Result<(), EngineError> {
        use Operator::*;
        match op {
            // --- graphics state: affects glyph placement through the CTM ---
            SaveState => self.gs_stack.push(self.gs),
            RestoreState => {
                let restored = self.gs_stack.pop().unwrap_or_default();
                // A CTM change moves where the next glyph lands (§9.4.4), so it places the pen.
                // Exact comparison: an epsilon would be a threshold, and NaN compares unequal.
                if restored.ctm != self.gs.ctm {
                    self.text_drawn_since_placement = false;
                }
                self.gs = restored;
            }
            ConcatMatrix => {
                let m = matrix_operands(operands)?;
                let ctm = m.then(self.gs.ctm);
                if ctm != self.gs.ctm {
                    self.text_drawn_since_placement = false;
                }
                self.gs.ctm = ctm;
            }

            // --- text object boundaries ---
            BeginText => {
                self.ts.begin_text();
                self.text_drawn_since_placement = false;
            }
            EndText => {}

            // --- text state ---
            CharSpacing => self.ts.char_spacing = num(operands, 0)?,
            WordSpacing => self.ts.word_spacing = num(operands, 0)?,
            HorizontalScale => self.ts.horizontal_scale = num(operands, 0)? / 100.0,
            Leading => self.ts.leading = num(operands, 0)?,
            RenderMode => self.ts.render_mode = num(operands, 0)? as i64,
            Rise => self.ts.rise = num(operands, 0)?,
            SelectFont => {
                let name = name_operand(operands, 0)?;
                self.ts.font_size = num(operands, 1)?;
                self.ts.font_id = Some(name);
            }

            // --- text positioning ---
            NextLine => {
                let (tx, ty) = (num(operands, 0)?, num(operands, 1)?);
                self.ts.next_line_offset(tx, ty);
                self.text_drawn_since_placement = false;
            }
            NextLineSetLeading => {
                let (tx, ty) = (num(operands, 0)?, num(operands, 1)?);
                self.ts.leading = -ty;
                self.ts.next_line_offset(tx, ty);
                self.text_drawn_since_placement = false;
            }
            SetTextMatrix => {
                let m = matrix_operands(operands)?;
                self.ts.set_matrix(m);
                self.text_drawn_since_placement = false;
            }
            NextLineByLeading => {
                self.ts.next_line();
                self.text_drawn_since_placement = false;
            }

            // --- text showing: all four, including the one pdf-inspector omits ---
            ShowText => {
                let bytes = string_operand(operands, 0)?;
                self.show(&bytes, &[])?;
            }
            ShowTextAdjusted => {
                let items = array_operand(operands, 0)?;
                self.show_adjusted(&items)?;
            }
            NextLineShowText => {
                self.ts.next_line();
                self.text_drawn_since_placement = false;
                let bytes = string_operand(operands, 0)?;
                self.show(&bytes, &[])?;
            }
            NextLineShowTextSpacing => {
                // `aw ac string "` — sets word and char spacing, then behaves as `'`.
                self.ts.word_spacing = num(operands, 0)?;
                self.ts.char_spacing = num(operands, 1)?;
                self.ts.next_line();
                self.text_drawn_since_placement = false;
                let bytes = string_operand(operands, 2)?;
                self.show(&bytes, &[])?;
            }

            // --- marked content: the mcid bridge ---
            //
            // v1-S3 also keeps the **tag**, which v0 discarded. Without it an `/Artifact`
            // sequence is indistinguishable from body text, and the only two ways to handle
            // artifacts are then "treat running heads as prose" or "delete them" — the second
            // being an undeclared document mutation (parity checklist O21/O22).
            BeginMarkedContentProps => {
                let artifact = is_artifact(marked_content_tag(operands));
                if props_given_by_name(operands) {
                    // The id may exist and this reader did not see it. Counted so the artifact
                    // can say so: an unread id and an absent id are different facts, and only
                    // one of them means "this content is outside the structure tree".
                    self.props_by_name = self.props_by_name.saturating_add(1);
                }
                self.mc_stack.push(MarkedContent {
                    artifact,
                    mcid: mcid_from_props(operands),
                });
            }
            BeginMarkedContent => {
                self.mc_stack.push(MarkedContent {
                    artifact: is_artifact(marked_content_tag(operands)),
                    // `BMC` carries no property list, so there is no id to capture and none is
                    // invented.
                    mcid: None,
                });
            }
            EndMarkedContent => {
                self.mc_stack.pop();
            }
            MarkedPoint | MarkedPointProps => {}

            // --- acknowledged no-ops -------------------------------------------------------
            //
            // Every arm below names a standard operator this profile does not need in order to
            // place a glyph. They are listed individually rather than collapsed into a wildcard
            // so that "we do not interpret `rg`" is a decision someone wrote down, and so that a
            // NEW operator cannot join them by accident.
            //
            // Line and colour state:
            LineWidth | LineCap | LineJoin | MiterLimit | DashPattern | RenderingIntent
            | Flatness | ExtGState => {}
            StrokeColorSpace | FillColorSpace | StrokeColor | StrokeColorN | FillColor
            | FillColorN | StrokeGray | FillGray | StrokeRgb | FillRgb | StrokeCmyk | FillCmyk => {}
            // --- path construction: interpreted as of v1-S1 ---------------------------------
            //
            // These were acknowledged-skips through v0.1. A ruled table is a table the document
            // *drew*, so the ruling lines are evidence and have to be read (`docs/history/08-V1-SCOPE.md`
            // §5). Only axis-aligned geometry is captured, and only after a painting operator
            // says ink reached the page.
            Rectangle => {
                // `re x y w h` — a complete subpath in its own right.
                let (x, y, w, h) = (
                    num(operands, 0)?,
                    num(operands, 1)?,
                    num(operands, 2)?,
                    num(operands, 3)?,
                );
                let a = self.gs.ctm.apply(x, y);
                let b = self.gs.ctm.apply(x + w, y + h);
                // A rotated or skewed CTM turns a rectangle into a parallelogram. Capturing its
                // bounding box would invent edges the document never drew, so it is dropped.
                if self.ctm_is_axis_aligned() {
                    self.pending.push(PathRect::from_corners(a, b));
                }
                self.subpath = Subpath::default();
            }
            MoveTo => {
                self.flush_subpath();
                let p = self.gs.ctm.apply(num(operands, 0)?, num(operands, 1)?);
                self.subpath.points.push(p);
            }
            LineTo => {
                let p = self.gs.ctm.apply(num(operands, 0)?, num(operands, 1)?);
                if let Some(&prev) = self.subpath.points.last() {
                    // A diagonal is not a ruling line, and a rectangle inferred from one would be
                    // an edge nobody drew.
                    if !approx_eq(prev.0, p.0) && !approx_eq(prev.1, p.1) {
                        self.subpath.disqualified = true;
                    }
                }
                self.subpath.points.push(p);
            }
            ClosePath => self.flush_subpath(),
            // **Curves are never tessellated into ruling lines.** Flattening a Bézier would
            // manufacture straight edges for a table the document drew with curves — the
            // clearest possible case of inventing geometry. The subpath is disqualified instead.
            CurveTo | CurveToV | CurveToY => self.subpath.disqualified = true,

            // --- painting: ink reached the page, so the pending rectangles are real ----------
            Stroke
            | CloseStroke
            | Fill
            | FillObsolete
            | FillEvenOdd
            | FillStroke
            | FillStrokeEvenOdd
            | CloseFillStroke
            | CloseFillStrokeEvenOdd => {
                self.flush_subpath();
                self.rects.append(&mut self.pending);
                self.segments.append(&mut self.pending_segments);
            }
            // `n` ends a path **without painting it**. Nothing was drawn, so nothing is a ruling
            // line — this is also the operator that ends a clip-only path.
            EndPath => {
                self.subpath = Subpath::default();
                self.pending.clear();
                self.pending_segments.clear();
            }
            // A clip path bounds what is visible; it draws nothing. Treating one as a table edge
            // would find a grid in every document that clips to its margins.
            Clip | ClipEvenOdd => {
                self.subpath = Subpath::default();
                self.pending.clear();
                self.pending_segments.clear();
            }
            // v1-S6. `Do` names an XObject in the page's resources. An `/Image` is a placement
            // this profile records; a `/Form` is content this profile still does not descend
            // into, and `form-xobject-text-not-descended` is where that stays declared.
            //
            // **Fail-closed still applies to the OPERATOR, not to the operand.** An unrecognised
            // operator stops the parse, as it always has. A `Do` naming a resource that does not
            // resolve is a different situation: the document is malformed in a bounded way, the
            // page drew something this reader cannot describe, and refusing the whole document
            // over it would turn files that parse today into failures. It is counted, and the
            // count is declared.
            XObject => self.draw_xobject(operands),
            Shading => {}
            // Inline images carry their samples in the content stream itself, so there is no
            // object to address and no separate stream to digest. Counted, and declared — an
            // uncounted skip here would let "this page has no image nodes" read as "this page has
            // no images", which is the silent drop this project refuses.
            //
            // lopdf collapses the whole `BI … ID … EI` sequence into one operand-less `BI`
            // operation, and on a *filtered* inline image it recovers by scanning forward to
            // `EI`. So this arm is the only place the engine can learn one happened at all.
            BeginInlineImage => self.inline_images = self.inline_images.saturating_add(1),
            InlineImageData | EndInlineImage => {}
            // Type 3 glyph metrics, meaningful only inside a glyph procedure.
            Type3Width | Type3WidthBBox => {}
            // Compatibility sections: PDF 32000-1 §7.8.2 says unrecognised operators inside
            // BX/EX may be ignored. This profile does not honour that, because "may be ignored"
            // is exactly the permission that loses text. The markers themselves are no-ops.
            BeginCompat | EndCompat => {}
        }
        Ok(())
    }

    /// Show a string, optionally with per-element adjustments already applied.
    fn show(&mut self, bytes: &[u8], synthesized: &[u32]) -> Result<(), EngineError> {
        let Some(font_id) = self.ts.font_id.clone() else {
            return Err(EngineError::Malformed {
                what: "content stream".into(),
                detail: "text shown before any Tf selected a font".into(),
            });
        };
        let Some(font) = self.fonts.get(&font_id) else {
            return Err(EngineError::MissingPart {
                part: format!("font resource /{font_id} referenced by Tf"),
            });
        };

        let codes = font.split_codes(bytes);
        if codes.is_empty() {
            return Ok(());
        }
        // Set before decoding, so a run dropped below still counts as drawn: its text was on the
        // page, and the pen moved for the codes before the refused one.
        self.text_drawn_since_placement = true;
        // PDF 32000-1 §9.3.3: `Tw` is added to the SINGLE-BYTE code 32 only. A simple font's
        // codes are one byte (§9.6). A composite font reaches an advance only under Identity-H or
        // Identity-V (`fonts::load_cid_widths`), whose codes are all two bytes, so none of its
        // codes takes `Tw`: not `<0020>`, and not a byte 0x20 the interim `/ToUnicode`-derived
        // split read alone (`composite-font-codes-from-tounicode`). If composite advances ever
        // reach a CMap with single-byte codes, this must ask the code's own length instead
        // (docs/22-WORD-BOXES-SCOPE.md §9).
        let word_spacing_applies = font.kind == crate::fonts::FontKind::Simple;

        let trm = self.ts.rendering_matrix(self.gs.ctm);
        let origin = (trm.e, trm.f);

        let mut text = String::new();
        let mut kept_codes = Vec::with_capacity(codes.len());
        let mut advance_total = 0.0f64;
        // The y half of the same travel, which `advance_total` never read (docs/22 §9 item 1).
        let mut advance_total_f = 0.0f64;
        let mut per_code = Vec::with_capacity(codes.len());
        let mut advance_known = true;

        // Set at the first code this font cannot decode. The run is then dropped, and the pen
        // still travels over every one of its codes: an undecodable code is a character this
        // reader could not name, not a glyph the document did not draw, and the codes after it
        // are drawn too. Until 2026-09-18 the loop returned here, leaving the text matrix where
        // the refused code stood, so every later run in the same text object was reported a
        // dropped run's width to the left of where the document places it (`docs/OPEN-WORK.md`
        // §6; doc 22's amendments). Ghostscript 10.06 renders the run after a 12-point
        // undecodable glyph 12 points right of where this engine used to put it.
        let mut dropped = false;

        for code in codes {
            if !dropped {
                match font.decode_code(code) {
                    Ok(s) => {
                        text.push_str(s);
                        kept_codes.push(code);
                    }
                    Err(e) => {
                        // **v0.1: drop this run, keep the page.** Through v0 this returned `Err`
                        // and failed the whole document — one unmappable glyph anywhere and a
                        // caller got nothing, which is fail-closed but far more than the evidence
                        // requires. (The comment here claimed the run continued. It did not; the
                        // code was the truth and the comment was aspiration.)
                        //
                        // The run is dropped **whole**, not patched. Two alternatives were
                        // rejected: emitting `U+FFFD` for the hole would put a character in the
                        // evidence that the document does not contain, and silently omitting just
                        // the bad code would splice the surrounding glyphs into a word the
                        // document never wrote — undetectable downstream, and worse than losing
                        // the run.
                        //
                        // Losing a run is itself a real loss, so it is counted and declared:
                        // `extract` turns a non-zero count into `broken-font-encoding`, and a
                        // document that decodes *nothing* is still refused outright. Decoding
                        // stops at the first failure, so one string is one dropped run however
                        // many of its codes this font cannot name.
                        self.undecodable.push(e.to_string());
                        self.dropped_runs = self.dropped_runs.saturating_add(1);
                        dropped = true;
                    }
                }
            }

            match font.advance_glyph_space(code) {
                Some(w0) => {
                    let is_space = word_spacing_applies && code == 32;
                    let before = self.ts.text_matrix.e;
                    let before_f = self.ts.text_matrix.f;
                    self.ts.advance(w0, is_space);
                    let delta = self.ts.text_matrix.e - before;
                    advance_total += delta;
                    advance_total_f += self.ts.text_matrix.f - before_f;
                    per_code.push(delta);
                }
                None => advance_known = false,
            }
        }

        // The pen has travelled; the run itself is not evidence and is not pushed.
        if dropped {
            return Ok(());
        }

        let scale = self.gs.ctm.x_scale();
        self.shown.push(ShownText {
            text,
            codes: kept_codes,
            origin,
            advance: advance_known.then_some(advance_total * scale),
            displacement: advance_known
                .then(|| self.gs.ctm.apply_linear(advance_total, advance_total_f)),
            glyph_up: (trm.c, trm.d),
            code_advances: advance_known
                .then(|| per_code.iter().map(|d| d * scale).collect::<Vec<_>>()),
            font_id,
            font_size: self.ts.font_size,
            em_scale_pt: trm.y_scale(),
            mcid: self.current_mcid(),
            artifact: self.inside_artifact(),
            synthesized_indices: synthesized.to_vec(),
            // v1-S6. Carried out of the text state so extraction can flag it. The run is pushed
            // either way — this is an observation about the run, never a reason to withhold it.
            render_mode: self.ts.render_mode,
            op_index: self.op_index,
        });

        Ok(())
    }

    /// `Do` — record an image placement, or count a name that did not resolve (v1-S6).
    ///
    /// # The unit square is the whole trick
    ///
    /// A PDF image is defined on the unit square: whatever its pixel dimensions, the content
    /// stream draws it into `(0,0)…(1,1)` and the current transformation matrix decides where
    /// that lands and how big it is (32000-1 §8.9.5.2). So the area a placement covers is exactly
    /// the CTM applied to four corners — a real measurement of the document's own matrix, not an
    /// inference, and specifically **not** the bitmap's `/Width` and `/Height`, which are a
    /// different quantity in different units and would be a box derived from a property that is
    /// not a box.
    ///
    /// All four corners are kept rather than a bounding box, because whether the placement is
    /// axis-aligned is decided from them further downstream. A rotated image genuinely covers a
    /// parallelogram, and its bounding box claims page area the picture does not cover.
    ///
    /// # What is not resolved here
    ///
    /// The XObject's `/Subtype`. This module has an object id and no document, by design — the
    /// interpreter never held a `Document` and giving it one to look up a subtype would hand the
    /// content-stream reader the whole object graph. Extraction sorts `/Image` from `/Form`,
    /// where the document is already in scope.
    fn draw_xobject(&mut self, operands: &[lopdf::Object]) {
        let name = match operands.first() {
            Some(lopdf::Object::Name(n)) => String::from_utf8_lossy(n).into_owned(),
            // `Do` with no operand, or an operand that is not a name. The document is malformed;
            // nothing here can say what it meant to draw.
            _ => {
                self.unresolved_xobjects = self.unresolved_xobjects.saturating_add(1);
                return;
            }
        };
        let Some(object) = self.xobjects.as_ref().and_then(|x| x.get(&name)).copied() else {
            self.unresolved_xobjects = self.unresolved_xobjects.saturating_add(1);
            return;
        };
        let m = self.gs.ctm;
        self.images.push(ImagePlacement {
            object,
            corners: [
                m.apply(0.0, 0.0),
                m.apply(1.0, 0.0),
                m.apply(1.0, 1.0),
                m.apply(0.0, 1.0),
            ],
        });
    }

    /// `TJ` — strings interleaved with positioning adjustments.
    ///
    /// A sufficiently negative adjustment opens a word gap that the document never wrote as a
    /// space glyph. Emitting the space unflagged would put a character in the evidence the
    /// document does not contain, so it is inserted **and flagged at the point of creation**.
    ///
    /// The gap is measured from the pen, so it follows the text drawn since the pen was last
    /// placed. After a placement with nothing drawn, the number measures from the new origin and
    /// says nothing about the space after an earlier run: a space there would be invented, inside
    /// a word ('i ncluded', 'Y OUR') or beside a space the document drew.
    fn show_adjusted(&mut self, items: &[lopdf::Object]) -> Result<(), EngineError> {
        for item in items {
            match item {
                lopdf::Object::String(bytes, _) => self.show(bytes, &[])?,
                lopdf::Object::Integer(_) | lopdf::Object::Real(_) => {
                    let amount = match item {
                        lopdf::Object::Integer(i) => *i as f64,
                        lopdf::Object::Real(r) => f64::from(*r),
                        _ => unreachable!(),
                    };
                    if amount <= TJ_SPACE_GAP_THOUSANDTHS && self.text_drawn_since_placement {
                        // Attach the synthesized space to the run that just ended, so the flag
                        // sits with the character rather than being reconstructed later. Only
                        // when text was drawn since the pen was placed: otherwise this number
                        // opens a gap from the new origin, not after that run.
                        if let Some(last) = self.shown.last_mut() {
                            let idx = last.text.chars().count() as u32;
                            last.text.push(' ');
                            last.synthesized_indices.push(idx);
                        }
                    }
                    self.ts.adjust(amount);
                }
                other => {
                    return Err(EngineError::Malformed {
                        what: "TJ array".into(),
                        detail: format!("expected strings and numbers, found {other:?}"),
                    })
                }
            }
        }
        Ok(())
    }
}

// --- operand helpers --------------------------------------------------------------------

fn num(operands: &[lopdf::Object], i: usize) -> Result<f64, EngineError> {
    match operands.get(i) {
        Some(lopdf::Object::Integer(v)) => Ok(*v as f64),
        Some(lopdf::Object::Real(v)) => Ok(f64::from(*v)),
        Some(other) => Err(EngineError::Malformed {
            what: "operand".into(),
            detail: format!("expected a number at position {i}, found {other:?}"),
        }),
        None => Err(EngineError::Malformed {
            what: "operand".into(),
            detail: format!("missing numeric operand at position {i}"),
        }),
    }
}

/// Tolerance for "the same coordinate", in user-space units.
///
/// One hundredth of a point — the quantum text origins are already rounded to
/// (`docs/01-CONTRACT.md` §4). Coordinates closer together than the artifact can express are the
/// same coordinate, and using a different tolerance here than the wire uses would let the
/// interpreter distinguish points the artifact cannot.
pub(crate) const COORD_EPSILON: f64 = 1.0 / ethos_parser_core::QUANTUM_PER_POINT as f64;

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < COORD_EPSILON
}

/// The distinct values in an iterator, within [`COORD_EPSILON`], sorted.
fn distinct(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    for v in values {
        if !out.iter().any(|e| approx_eq(*e, v)) {
            out.push(v);
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

fn matrix_operands(operands: &[lopdf::Object]) -> Result<Matrix, EngineError> {
    if operands.len() < 6 {
        return Err(EngineError::Malformed {
            what: "matrix operands".into(),
            detail: format!("expected six numbers, found {}", operands.len()),
        });
    }
    Ok(Matrix::new(
        num(operands, 0)?,
        num(operands, 1)?,
        num(operands, 2)?,
        num(operands, 3)?,
        num(operands, 4)?,
        num(operands, 5)?,
    ))
}

fn name_operand(operands: &[lopdf::Object], i: usize) -> Result<String, EngineError> {
    match operands.get(i) {
        Some(lopdf::Object::Name(n)) => Ok(String::from_utf8_lossy(n).to_string()),
        _ => Err(EngineError::Malformed {
            what: "operand".into(),
            detail: format!("expected a name at position {i}"),
        }),
    }
}

fn string_operand(operands: &[lopdf::Object], i: usize) -> Result<Vec<u8>, EngineError> {
    match operands.get(i) {
        Some(lopdf::Object::String(b, _)) => Ok(b.clone()),
        _ => Err(EngineError::Malformed {
            what: "operand".into(),
            detail: format!("expected a string at position {i}"),
        }),
    }
}

fn array_operand(operands: &[lopdf::Object], i: usize) -> Result<Vec<lopdf::Object>, EngineError> {
    match operands.get(i) {
        Some(lopdf::Object::Array(a)) => Ok(a.clone()),
        _ => Err(EngineError::Malformed {
            what: "operand".into(),
            detail: format!("expected an array at position {i}"),
        }),
    }
}

/// Pull `/MCID` out of a `BDC` property list.
///
/// Returns `None` when the document does not supply one. **Never invented** — Workbench rule 3.
///
/// A property list given as a *name* referring to the page's `/Properties` resource is not
/// resolved here, so its id is absent rather than wrong. That absence is a declared gap
/// (`mcid-property-list-by-name`), not a claim that the sequence had no id.
fn mcid_from_props(operands: &[lopdf::Object]) -> Option<i64> {
    match operands.get(1)? {
        lopdf::Object::Dictionary(d) => d.get(b"MCID").ok()?.as_i64().ok(),
        _ => None,
    }
}

/// Whether a `BDC`/`BMC` property list was supplied **by name** rather than inline.
///
/// PDF 32000-1 §14.6.2 allows either. A name indirects through the page's `/Properties`
/// resource dictionary, which this profile does not resolve — so the sequence may well carry an
/// `/MCID` this reader did not see. Detected so it can be declared: silently treating it as
/// "no id" would make an unread id indistinguishable from an absent one.
fn props_given_by_name(operands: &[lopdf::Object]) -> bool {
    matches!(operands.get(1), Some(lopdf::Object::Name(_)))
}

/// The tag operand of a `BDC`/`BMC`, which is always the first.
///
/// Returns `None` for a malformed operand rather than erroring: an unreadable tag means this
/// reader does not know whether the sequence is an artifact, and the honest consequence is to
/// treat it as ordinary content rather than to refuse a document over a marked-content label.
fn marked_content_tag(operands: &[lopdf::Object]) -> Option<&[u8]> {
    match operands.first()? {
        lopdf::Object::Name(n) => Some(n.as_slice()),
        _ => None,
    }
}

/// Whether a marked-content tag is `/Artifact`.
///
/// Compared exactly. `/Artifact` is a standard tag with a fixed spelling (§14.8.2.2), and
/// accepting near-misses would mean deciding that some other tag the document chose means
/// "furniture" — which is the document's call, not this reader's.
fn is_artifact(tag: Option<&[u8]>) -> bool {
    tag == Some(b"Artifact".as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn ops(src: &str) -> Vec<lopdf::content::Operation> {
        lopdf::content::Content::decode(src.as_bytes())
            .expect("test content decodes")
            .operations
    }

    fn no_fonts() -> BTreeMap<String, std::sync::Arc<Font>> {
        BTreeMap::new()
    }

    #[test]
    fn an_unknown_operator_is_refused_by_name() {
        // The table's own answer first: a token outside PDF 32000-1 Table A.1 resolves to
        // nothing, and the interpreter turns that into a hard error rather than a skip.
        assert_eq!(crate::ops::Operator::from_token("UnknownOp"), None);

        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        let e = i.run(&ops("q 1 0 0 1 0 0 cm UnknownOp Q")).unwrap_err();
        assert_eq!(e.code(), "unsupported");
        assert!(
            e.to_string().contains("UnknownOp"),
            "the error must name the token: {e}"
        );
        assert!(
            i.shown.is_empty(),
            "no partial output may survive a fail-closed parse"
        );
    }

    /// **A NaN path coordinate is sorted, not a panic.** Nine `1e38` scalings overflow the CTM to
    /// infinity, and a point at the origin then maps to `0 × ∞`, NaN on both axes. The rectangle
    /// test sorted those with `partial_cmp(..).expect(..)`, and under the release profile's
    /// `panic = "abort"` one page ended the process.
    #[test]
    fn a_nan_path_coordinate_is_not_a_panic() {
        let scale = format!("1{}.0 0 0 1 0 0 cm ", "0".repeat(38));
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops(&format!(
            "{}0 0 m 0 0 l 0 0 l 0 0 l S",
            scale.repeat(9)
        )))
        .expect("a path at a NaN point is still a path");
    }

    /// A usable font, so a test can actually reach the show-text path.
    ///
    /// `no_fonts()` cannot: `Tf` on an empty map fails first, with `missing_part`, which is
    /// correct behaviour and the wrong thing to be testing when the subject is an operator.
    fn one_font() -> BTreeMap<String, std::sync::Arc<Font>> {
        use crate::encoding::{BaseEncoding, SimpleEncoding};
        use crate::fonts::{Decoder, FontKind, WidthSource};
        use ethos_parser_core::GeometryAbsence;

        let mut m = BTreeMap::new();
        m.insert(
            "F1".to_string(),
            std::sync::Arc::new(Font {
                id: "F1".into(),
                kind: FontKind::Simple,
                decoder: Decoder::Simple(SimpleEncoding::new(
                    BaseEncoding::WinAnsi,
                    BTreeMap::new(),
                )),
                widths: WidthSource::Widths {
                    first_char: 32,
                    widths: vec![500.0; 95],
                    type3_scale_x: None,
                },
                builtin_encoding_assumed: None,
                ink: crate::fonts::FontInk::Absent(GeometryAbsence::NotReportedByReader),
            }),
        );
        m
    }

    /// A composite font decoding through `tounicode`, with Identity-style CID widths of 500.
    ///
    /// Its code length is whatever that CMap's codespace makes `Font::split_codes` read.
    fn one_composite_font(tounicode: &[u8]) -> BTreeMap<String, std::sync::Arc<Font>> {
        use crate::fonts::{Decoder, FontKind, WidthSource};
        use ethos_parser_core::GeometryAbsence;

        let mut m = BTreeMap::new();
        m.insert(
            "F1".to_string(),
            std::sync::Arc::new(Font {
                id: "F1".into(),
                kind: FontKind::Composite,
                decoder: Decoder::ToUnicode(
                    crate::cmap::ToUnicode::parse(tounicode).expect("test ToUnicode parses"),
                ),
                widths: WidthSource::Cid {
                    spans: BTreeMap::new(),
                    default: 500.0,
                },
                builtin_encoding_assumed: None,
                ink: crate::fonts::FontInk::Absent(GeometryAbsence::NotReportedByReader),
            }),
        );
        m
    }

    // ---------------------------------------------------------------------------------------
    // v1-S1 — path capture
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_painted_rectangle_is_captured_in_user_space() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("40 80 100 40 re S")).unwrap();

        assert_eq!(i.rects.len(), 1);
        let r = i.rects[0];
        assert!(approx_eq(r.x0, 40.0) && approx_eq(r.y0, 80.0));
        assert!(approx_eq(r.x1, 140.0) && approx_eq(r.y1, 120.0));
    }

    #[test]
    fn an_unpainted_path_draws_no_ruling_line() {
        // `n` ends the path without painting. Nothing reached the page, so nothing is evidence
        // of a rule — this is also how a clip-only path ends.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("40 80 100 40 re n")).unwrap();
        assert!(i.rects.is_empty(), "an unpainted path is not a ruling line");
    }

    #[test]
    fn a_clip_path_is_not_a_ruling_line() {
        // Every document that clips to its margins would otherwise contain a one-cell table.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("0 0 612 792 re W n")).unwrap();
        assert!(i.rects.is_empty());
    }

    #[test]
    fn a_rectangle_drawn_as_four_lines_is_captured() {
        // Real documents draw grids both ways. `m`/`l`/`h` closing back on itself is a rectangle
        // as much as `re` is.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("40 80 m 140 80 l 140 120 l 40 120 l h S"))
            .unwrap();
        assert_eq!(i.rects.len(), 1, "got {:?}", i.rects);
        let r = i.rects[0];
        assert!(approx_eq(r.x0, 40.0) && approx_eq(r.x1, 140.0));
    }

    #[test]
    fn a_diagonal_is_never_turned_into_a_rectangle() {
        // The bounding box of a triangle is three edges the document did not draw.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("40 80 m 140 120 l 40 120 l h S")).unwrap();
        assert!(i.rects.is_empty(), "got {:?}", i.rects);
    }

    #[test]
    fn a_curve_is_never_flattened_into_ruling_lines() {
        // Tessellating a Bezier would manufacture straight edges for a table drawn with curves —
        // the clearest possible case of inventing geometry (docs/history/09-V1-MILESTONES.md S1).
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("40 80 m 140 80 l 140 120 40 120 40 80 c h S"))
            .unwrap();
        assert!(i.rects.is_empty(), "got {:?}", i.rects);
    }

    #[test]
    fn a_rotated_ctm_drops_the_rectangle_rather_than_boxing_it() {
        // Under a 45-degree CTM a rectangle's image is a diamond, and its bounding box is four
        // edges nobody drew. Dropped, not approximated.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        let c = std::f64::consts::FRAC_1_SQRT_2;
        i.run(&ops(&format!("{c} {c} -{c} {c} 0 0 cm 40 80 100 40 re S")))
            .unwrap();
        assert!(i.rects.is_empty(), "got {:?}", i.rects);
    }

    #[test]
    fn the_ctm_is_applied_to_captured_geometry() {
        // A scale-and-translate CTM keeps a rectangle a rectangle, so it is captured — in the
        // coordinates the page actually paints it at.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("2 0 0 2 10 20 cm 40 80 100 40 re S")).unwrap();
        assert_eq!(i.rects.len(), 1);
        let r = i.rects[0];
        assert!(approx_eq(r.x0, 90.0), "got {r:?}");
        assert!(approx_eq(r.y0, 180.0), "got {r:?}");
        assert!(
            approx_eq(r.x1, 290.0) && approx_eq(r.y1, 260.0),
            "got {r:?}"
        );
    }

    #[test]
    fn a_refused_parse_discards_captured_geometry_too() {
        // The same fail-closed rule the shown runs are under (M7): nothing a refused parse
        // touched may survive to be read as a complete reading of the page.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        let e = i.run(&ops("40 80 100 40 re S UnknownOp")).unwrap_err();
        assert_eq!(e.code(), "unsupported");
        assert!(i.rects.is_empty(), "geometry must go with the text");
    }

    // ---------------------------------------------------------------------------------------
    // Per-code advances
    // ---------------------------------------------------------------------------------------

    /// One advance per code, summing to the total, in the total's space.
    #[test]
    fn per_code_advances_align_with_codes_and_sum_to_the_total() {
        let fonts = one_font();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 0 0 Td (AB) Tj ET")).unwrap();

        let sh = &i.shown[0];
        let per = sh
            .code_advances
            .as_ref()
            .expect("this font carries widths, so the advance is known");

        assert_eq!(per.len(), sh.codes.len(), "one entry per code");
        assert_eq!(per.len(), 2, "two codes were shown");

        // `one_font` gives every glyph 500/1000 em, so at 10pt each advances 5pt.
        for (n, a) in per.iter().enumerate() {
            assert!(approx_eq(*a, 5.0), "code {n} advanced {a}, expected 5.0");
        }

        let summed: f64 = per.iter().sum();
        let total = sh.advance.expect("advance is known");
        assert!(
            (summed - total).abs() < 1e-9,
            "per-code {summed} vs total {total}"
        );
    }

    /// The per-code advances are in user space, like the total — the CTM applies to both.
    ///
    /// Worth its own test because the two are computed on different lines: the total is summed in
    /// text space and scaled once, each entry is scaled as it is collected. A refactor that scaled
    /// only one of them would still pass the alignment test above.
    #[test]
    fn per_code_advances_carry_the_ctm_like_the_total_does() {
        let fonts = one_font();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("2 0 0 2 0 0 cm BT /F1 10 Tf 0 0 Td (AB) Tj ET"))
            .unwrap();

        let sh = &i.shown[0];
        let per = sh.code_advances.as_ref().expect("widths are known");
        for a in per {
            assert!(
                approx_eq(*a, 10.0),
                "a doubling CTM must double 5pt, got {a}"
            );
        }
        let summed: f64 = per.iter().sum();
        assert!(approx_eq(summed, sh.advance.unwrap()));
    }

    /// A code that decodes to several characters advances the pen once.
    ///
    /// `/Differences` names the `fi` glyph for code 100 (`d`), so `(dle)` is three codes and the
    /// four characters of `file`. An entry per character would misalign every advance after the
    /// ligature, and a total by character would be one glyph too wide.
    #[test]
    fn a_code_decoding_to_several_characters_advances_the_pen_once() {
        use crate::encoding::{BaseEncoding, SimpleEncoding};
        use crate::fonts::Decoder;

        let mut font = (*one_font()["F1"]).clone();
        font.decoder = Decoder::Simple(SimpleEncoding::new(
            BaseEncoding::WinAnsi,
            BTreeMap::from([(100u8, "fi".to_string())]),
        ));
        let fonts = BTreeMap::from([("F1".to_string(), std::sync::Arc::new(font))]);

        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 0 0 Td (dle) Tj ET")).unwrap();

        let sh = &i.shown[0];
        assert_eq!(sh.text, "file");
        assert_eq!(sh.codes, vec![100, 108, 101]);

        let per = sh.code_advances.as_ref().expect("this font carries widths");
        assert_eq!(per.len(), 3, "one entry per code, not per character");
        for (n, a) in per.iter().enumerate() {
            assert!(approx_eq(*a, 5.0), "code {n} advanced {a}, expected 5.0");
        }
        assert!(
            approx_eq(sh.advance.expect("advance is known"), 15.0),
            "three glyphs, not four"
        );
    }

    /// `Tw` reaches a single-byte code 32 only (PDF 32000-1 §9.3.3; docs/22-WORD-BOXES-SCOPE.md §9
    /// item 3).
    ///
    /// Every glyph in (i) to (iii) is 500/1000 em at 10 pt under `10 Tw`, so a code 32 that took
    /// word spacing advances 15 and one that did not advances 5; (iv) is Courier, 600/1000 em. All
    /// values are exact in binary floating point.
    #[test]
    fn word_spacing_is_added_to_a_single_byte_code_32_only() {
        // (i) A simple font: code 32 is one byte and takes `Tw`; its neighbours do not.
        let fonts = one_font();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 10 Tw 0 0 Td (A A) Tj ET"))
            .unwrap();
        let sh = &i.shown[0];
        assert_eq!(sh.codes, vec![65, 32, 65]);
        assert_eq!(sh.code_advances, Some(vec![5.0, 15.0, 5.0]));
        assert_eq!(sh.advance, Some(25.0));

        // (ii) A composite font with a two-byte codespace: the same code 32, in two bytes.
        let fonts = one_composite_font(
            b"1 begincodespacerange <0000> <ffff> endcodespacerange \
              2 beginbfchar <0001> <0041> <0020> <0042> endbfchar",
        );
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 10 Tw 0 0 Td <000100200001> Tj ET"))
            .unwrap();
        let sh = &i.shown[0];
        assert_eq!(sh.codes, vec![1, 32, 1], "the same code 32, in two bytes");
        assert_eq!(sh.code_advances, Some(vec![5.0, 5.0, 5.0]));
        assert_eq!(sh.advance, Some(15.0));

        // (iii) A composite font whose `/ToUnicode` declares no codespace, so the interim split
        // reads one byte at a time. The byte 0x20 still takes no `Tw`.
        let fonts = one_composite_font(b"2 beginbfchar <41> <0041> <20> <0042> endbfchar");
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 10 Tw 0 0 Td <412041> Tj ET"))
            .unwrap();
        let sh = &i.shown[0];
        assert_eq!(sh.codes, vec![65, 32, 65], "the split really was one byte");
        assert_eq!(
            sh.code_advances.as_ref().map(|a| a[1]),
            Some(5.0),
            "a composite font under Identity defines no single-byte code (PDF 32000-1 §9.7.5.2), \
             so its byte 0x20 takes no word spacing"
        );
        assert_eq!(sh.advance, Some(15.0));

        // (iv) A simple font as documents usually ship one: decoding through a `/ToUnicode`, with
        // standard-14 widths instead of `/Widths`. The gate reads the font's kind, not its decoder
        // or its width source, so code 32 still takes `Tw`.
        let mut courier = (*one_font()["F1"]).clone();
        courier.decoder = crate::fonts::Decoder::ToUnicode(
            crate::cmap::ToUnicode::parse(b"2 beginbfchar <41> <0041> <20> <0020> endbfchar")
                .expect("test ToUnicode parses"),
        );
        let metrics = crate::afm::for_base_font("Courier").expect("Courier is standard 14");
        courier.widths = crate::fonts::WidthSource::Standard14 {
            face: metrics.face,
            metrics,
        };
        let fonts = BTreeMap::from([("F1".to_string(), std::sync::Arc::new(courier))]);
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 10 Tw 0 0 Td (A A) Tj ET"))
            .unwrap();
        let sh = &i.shown[0];
        assert_eq!(sh.codes, vec![65, 32, 65]);
        assert_eq!(sh.code_advances, Some(vec![6.0, 16.0, 6.0]));
        assert_eq!(sh.advance, Some(28.0));
    }

    // ---------------------------------------------------------------------------------------
    // The pen's travel as a vector (docs/22 §9 items 1 and 2)
    // ---------------------------------------------------------------------------------------

    fn approx_pair(got: (f64, f64), want: (f64, f64)) -> bool {
        approx_eq(got.0, want.0) && approx_eq(got.1, want.1)
    }

    /// A text matrix turned a quarter moves the pen up the page, and the travel says so.
    #[test]
    fn a_turned_text_matrix_moves_the_pen_along_y() {
        let fonts = one_font();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 0 1 -1 0 50 50 Tm (AB) Tj ET"))
            .unwrap();

        let sh = &i.shown[0];
        let d = sh.displacement.expect("widths are known");
        assert!(
            approx_pair(d, (0.0, 10.0)),
            "two 5pt glyphs up +y, got {d:?}"
        );
        assert!(
            approx_pair(sh.glyph_up, (-10.0, 0.0)),
            "the glyph tops point -x, got {:?}",
            sh.glyph_up
        );
        // The wire advance keeps its legacy value, measured along the text matrix's x alone.
        // Re-expressing it is left open — a known defect recorded in docs/22 — not this change.
        assert_eq!(sh.advance, Some(0.0));
    }

    /// A CTM turned a quarter carries the travel with it; `x_scale` kept only its length.
    #[test]
    fn a_turned_ctm_carries_the_displacement_with_it() {
        let fonts = one_font();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("0 1 -1 0 200 0 cm BT /F1 10 Tf 50 50 Td (AB) Tj ET"))
            .unwrap();

        let sh = &i.shown[0];
        let d = sh.displacement.expect("widths are known");
        assert!(
            approx_pair(d, (0.0, 10.0)),
            "the CTM turns +x to +y, got {d:?}"
        );
        assert!(
            approx_pair(sh.glyph_up, (-10.0, 0.0)),
            "got {:?}",
            sh.glyph_up
        );
        assert_eq!(sh.advance, Some(10.0), "the legacy advance is a length");
    }

    /// **On upright text the travel is the legacy advance, bit for bit.**
    ///
    /// Every box an upright run has ever had was quantized from `advance_total * ctm.x_scale()`,
    /// and the box is now built from `displacement.0`. With `Tm.b` and `CTM.b` zero and `CTM.a`
    /// positive, `ctm.a * total + ctm.c * ±0` is `total * sqrt(a * a)` exactly, because a binary64
    /// `sqrt(x²)` is `|x|`. An approximate comparison would pass a change that moved the last bit
    /// and, through quantization, a centipoint on some document.
    #[test]
    fn upright_displacement_is_bitwise_the_legacy_advance() {
        let fonts = one_font();
        let mut cases = 0;
        for ctm in [
            "1 0 0 1 0 0",
            "0.24 0 0 0.24 0 0",
            "1 0 0 -1 0 792",
            "1.3 0 0.4 0.9 0 0",
        ] {
            for tm in ["1 0 0 1 72 700", "9.96 0 0 9.96 30 40", "1 0 0.2 1 5 5"] {
                for tc in ["0", "0.37"] {
                    for tz in ["100", "83"] {
                        for tf in ["10", "1"] {
                            let src = format!(
                                "q {ctm} cm BT /F1 {tf} Tf {tc} Tc {tz} Tz {tm} Tm (AB CDEF) Tj ET Q"
                            );
                            let mut i = Interpreter::new(&fonts);
                            i.run(&ops(&src)).unwrap();
                            let sh = &i.shown[0];
                            let advance = sh.advance.expect("widths are known");
                            let (dx, dy) = sh.displacement.expect("present with the advance");
                            assert!(
                                dx.to_bits() == advance.to_bits() || (dx == 0.0 && advance == 0.0),
                                "{src}: displacement {dx:e} is not the legacy advance {advance:e}"
                            );
                            assert_eq!(dy, 0.0, "{src}");
                            cases += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 96);
    }

    /// Fail-closed means *nothing* survives, including text shown before the bad token.
    ///
    /// Moved here at M7 from `tests/extraction.rs`, which reached into this module to assert it
    /// and so made `content` and `ops` public for one test. Strengthened in the move: the
    /// original ran against an interpreter that had shown nothing yet, so it would have held even
    /// if partial output were kept. This one shows real text first, then hits the bad token.
    #[test]
    fn text_shown_before_an_unknown_operator_is_discarded_too() {
        let fonts = one_font();

        // Control: without the bad token the same stream does show text, so the assertion below
        // is about the refusal and not about a stream that never produced anything.
        let mut ok = Interpreter::new(&fonts);
        ok.run(&ops("BT /F1 12 Tf 10 10 Td (hello) Tj ET"))
            .expect("the control stream is valid");
        assert_eq!(ok.shown.len(), 1, "the control must show text");

        let mut i = Interpreter::new(&fonts);
        let e = i
            .run(&ops("BT /F1 12 Tf 10 10 Td (hello) Tj UnknownOp ET"))
            .unwrap_err();
        assert_eq!(e.code(), "unsupported");
        assert!(
            i.shown.is_empty(),
            "a run that refused must not leave {} shown item(s) behind",
            i.shown.len()
        );
    }

    #[test]
    fn a_known_no_op_operator_does_not_stop_the_parse() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        // Colour, path and clipping operators are acknowledged, not refused.
        i.run(&ops("q 0 0 1 RG 1 1 1 rg 10 10 m 20 20 l S W n Q"))
            .expect("standard operators are handled");
    }

    #[test]
    fn graphics_state_stacks() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("q 2 0 0 2 0 0 cm Q")).unwrap();
        assert_eq!(
            i.gs.ctm,
            Matrix::IDENTITY,
            "Q must restore the matrix q saved"
        );
    }

    #[test]
    fn an_unbalanced_restore_does_not_panic() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("Q Q Q")).expect("a stray Q is survivable");
        assert_eq!(i.gs.ctm, Matrix::IDENTITY);
    }

    #[test]
    fn showing_text_without_a_font_is_malformed_not_silent() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        let e = i.run(&ops("BT (hello) Tj ET")).unwrap_err();
        assert_eq!(e.code(), "malformed");
    }

    #[test]
    fn a_missing_font_resource_is_named() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        let e = i.run(&ops("BT /F9 12 Tf (hi) Tj ET")).unwrap_err();
        assert_eq!(e.code(), "missing_part");
        assert!(e.to_string().contains("F9"));
    }

    #[test]
    fn mcid_is_captured_from_bdc_and_never_invented() {
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        i.run(&ops("/P <</MCID 7>> BDC EMC")).unwrap();
        assert_eq!(i.current_mcid(), None, "EMC pops the scope");

        let mut j = Interpreter::new(&fonts);
        j.run(&ops("/P <</MCID 7>> BDC")).unwrap();
        assert_eq!(j.current_mcid(), Some(7));

        let mut k = Interpreter::new(&fonts);
        k.run(&ops("/P BMC")).unwrap();
        assert_eq!(
            k.current_mcid(),
            None,
            "BMC has no MCID, and none is invented"
        );
    }

    #[test]
    fn an_artifact_sequence_is_flagged_through_any_nesting() {
        // v1-S3. `/Artifact` marks page furniture — running heads, folios, rules. The whole
        // stack is asked, not the innermost frame: a `/Span` nested inside an artifact is still
        // artifact content, and reading only the innermost would call it body text.
        let fonts = no_fonts();

        let mut i = Interpreter::new(&fonts);
        i.run(&ops("/Artifact BMC")).unwrap();
        assert!(i.inside_artifact());

        let mut j = Interpreter::new(&fonts);
        j.run(&ops("/Artifact BMC /Span <</MCID 3>> BDC")).unwrap();
        assert!(j.inside_artifact(), "nesting does not un-mark an artifact");
        assert_eq!(j.current_mcid(), Some(3), "the inner id is still captured");

        let mut k = Interpreter::new(&fonts);
        k.run(&ops("/Artifact BMC EMC")).unwrap();
        assert!(!k.inside_artifact(), "EMC pops the artifact scope");

        // And ordinary content is not an artifact just because something was marked.
        let mut m = Interpreter::new(&fonts);
        m.run(&ops("/P <</MCID 0>> BDC")).unwrap();
        assert!(!m.inside_artifact());
    }

    #[test]
    fn a_property_list_given_by_name_is_counted_rather_than_read_as_absent() {
        // v1-S3. `BDC` may name a `/Properties` entry instead of writing the dictionary inline
        // (§14.6.2). This profile does not resolve that, so the id is unknown — and an UNREAD id
        // is not an ABSENT one. Counted so the artifact can declare the difference.
        let fonts = no_fonts();

        let mut i = Interpreter::new(&fonts);
        i.run(&ops("/P /MC0 BDC")).unwrap();
        assert_eq!(i.current_mcid(), None, "nothing is invented for it");
        assert_eq!(i.props_by_name, 1, "but the gap is counted");

        let mut j = Interpreter::new(&fonts);
        j.run(&ops("/P <</MCID 4>> BDC")).unwrap();
        assert_eq!(j.props_by_name, 0, "an inline property list is not a gap");
    }

    #[test]
    fn malformed_operands_are_refused() {
        let fonts = no_fonts();
        // `cm` with four operands instead of six.
        let mut i = Interpreter::new(&fonts);
        assert!(i.run(&ops("1 0 0 1 cm")).is_err());
        // `Tf` with a number where the font name goes.
        let mut j = Interpreter::new(&fonts);
        assert!(j.run(&ops("BT 5 12 Tf ET")).is_err());
    }

    #[test]
    fn a_positive_tj_adjustment_never_synthesizes_a_space() {
        // A positive adjustment closes a gap; only a sufficiently negative one opens a word
        // break. Asserted behaviourally, because a comparison between two constants is decided
        // by the compiler and would be optimized away.
        let fonts = no_fonts();
        let mut i = Interpreter::new(&fonts);
        // No font, so the strings would error — but the numeric arm runs first and must not
        // push a synthesized space onto an empty run list.
        let _ = i.show_adjusted(&[lopdf::Object::Integer(500)]);
        assert!(i.shown.is_empty(), "a positive adjustment produced output");

        let _ = i.show_adjusted(&[lopdf::Object::Integer(-2000)]);
        assert!(
            i.shown.is_empty(),
            "a negative adjustment with no preceding run must not invent one"
        );
    }

    // ---------------------------------------------------------------------------------------
    // A TJ gap follows the pen (docs/22-WORD-BOXES-SCOPE.md §9 item 4)
    // ---------------------------------------------------------------------------------------

    /// Every run's text and flagged indices, so a row is compared whole.
    fn texts_and_flags(i: &Interpreter<'_>) -> Vec<(String, Vec<u32>)> {
        i.shown
            .iter()
            .map(|s| (s.text.clone(), s.synthesized_indices.clone()))
            .collect()
    }

    fn run_of(text: &str, flags: &[u32]) -> (String, Vec<u32>) {
        (text.to_string(), flags.to_vec())
    }

    /// After the pen is placed again, a `TJ` number measures from the new origin (PDF 32000-1
    /// §9.4.3), so it is no evidence of a gap after the run drawn before. The corpus shape is the
    /// `Tm` row: a producer re-placing the pen on the same line wrote 'i ncluded' and 'Y OUR'.
    #[test]
    fn a_tj_gap_after_the_pen_was_placed_again_writes_no_space() {
        let fonts = one_font();
        for (op, src) in [
            ("BT", "BT /F1 10 Tf (ab) Tj ET BT [-400 (cd)] TJ ET"),
            ("Td", "BT /F1 10 Tf (ab) Tj 0 -12 Td [-400 (cd)] TJ ET"),
            ("TD", "BT /F1 10 Tf (ab) Tj 0 -12 TD [-400 (cd)] TJ ET"),
            ("T*", "BT /F1 10 Tf 12 TL (ab) Tj T* [-400 (cd)] TJ ET"),
            (
                "Tm",
                "BT /F1 10 Tf (ab) Tj 1 0 0 1 0 0 Tm [-400 (cd)] TJ ET",
            ),
            ("'", "BT /F1 10 Tf 12 TL (ab) Tj () ' [-400 (cd)] TJ ET"),
            (
                "\"",
                "BT /F1 10 Tf 12 TL (ab) Tj 0 0 () \" [-400 (cd)] TJ ET",
            ),
            (
                "a cm that moves",
                "BT /F1 10 Tf (ab) Tj 1 0 0 1 0 -12 cm [-400 (cd)] TJ ET",
            ),
            (
                "a Q restoring a CTM a cm changed",
                "BT /F1 10 Tf q 1 0 0 1 0 12 cm (ab) Tj Q [-400 (cd)] TJ ET",
            ),
            (
                "BT on another line (probe p25)",
                "BT /F1 10 Tf 50 50 Td (ab) Tj ET BT /F1 10 Tf 50 100 Td [-400 (cd)] TJ ET",
            ),
        ] {
            let mut i = Interpreter::new(&fonts);
            i.run(&ops(src)).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(
                texts_and_flags(&i),
                [run_of("ab", &[]), run_of("cd", &[])],
                "{op} placed the pen, so the number opened no gap after 'ab': {src}"
            );
            if op == "Tm" {
                // The number still moves the pen: only the space is withheld.
                assert!(
                    approx_eq(i.shown[1].origin.0, 4.0),
                    "-400 at 10 pt moves 'cd' 4 pt from the new origin, got {}",
                    i.shown[1].origin.0
                );
            }
        }
    }

    /// A number with nothing between it and the last run but operators that leave the pen where
    /// it stands still opens a gap after that run. The colour row is the corpus shape: 147
    /// synthesized spaces cross only `rg`/`RG`/`g`/`G`/`k`/`K`, all genuine word gaps.
    ///
    /// `BI`/`ID`/`EI` is not covered: lopdf collapses the inline image into one operation, and a
    /// unit-test stream for it is fragile.
    #[test]
    fn a_tj_gap_the_pen_opened_from_the_last_run_still_writes_its_space() {
        let fonts = one_font();
        let between = |x: &str| format!("BT /F1 10 Tf (ab) Tj {x} [-400 (cd)] TJ ET");
        let rows = [
            (
                "same array",
                "BT /F1 10 Tf [(ab) -400 (cd)] TJ ET".to_string(),
            ),
            (
                "across TJ",
                "BT /F1 10 Tf [(ab)] TJ [-400 (cd)] TJ ET".to_string(),
            ),
            ("Tj then TJ", between("")),
            (
                "trailing number",
                "BT /F1 10 Tf [(ab) -400] TJ (cd) Tj ET".to_string(),
            ),
            ("colour", between("0 0 0.5 rg 0 0 0.5 RG")),
            ("gray and cmyk", between("0 g 0 G 0 0 0 1 k 0 0 0 1 K")),
            ("Tf", between("/F1 12 Tf")),
            ("Tc", between("1 Tc")),
            ("Tw", between("2 Tw")),
            ("Tz", between("90 Tz")),
            ("TL", between("14 TL")),
            ("Ts", between("3 Ts")),
            ("Tr", between("3 Tr")),
            (
                "marked content",
                between("/Span <</MCID 1>> BDC EMC /P BMC EMC /X MP"),
            ),
            ("q alone", between("q")),
            (
                "q, colour, Q with the CTM unchanged",
                between("q 1 0 0 rg Q"),
            ),
            ("an identity cm", between("1 0 0 1 0 0 cm")),
            ("general graphics state", between("1 w [] 0 d /GS0 gs")),
            (
                "path, paint, clip, shading, XObject, Type 3, compatibility",
                between(
                    "0 0 m 10 0 l 0 0 1 1 2 2 c h S 0 0 5 5 re W n /Sh0 sh /Im0 Do 0 0 d0 BX EX",
                ),
            ),
            (
                "an empty string",
                "BT /F1 10 Tf (ab) Tj [() -400 (cd)] TJ ET".to_string(),
            ),
        ];
        for (what, src) in rows {
            let mut i = Interpreter::new(&fonts);
            i.run(&ops(&src)).unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_eq!(
                texts_and_flags(&i),
                [run_of("ab ", &[2]), run_of("cd", &[])],
                "{what} left the pen where 'ab' ended, so the gap is after 'ab': {src}"
            );
        }
    }

    /// Once the pen is placed, the next string it draws is what a following number measures from.
    /// A string later dropped as undecodable counts too: its text was on the page, and suppressing
    /// the gap after it would fuse the last kept run with the next one across text the reader
    /// lost (the 01030000000159 shape).
    #[test]
    fn a_placed_pen_is_anchored_by_the_next_string_it_draws() {
        let fonts = one_font();

        let mut i = Interpreter::new(&fonts);
        i.run(&ops(
            "BT /F1 10 Tf (ab) Tj 1 0 0 1 0 0 Tm [-400 (cd) -400 (ef)] TJ ET",
        ))
        .unwrap();
        assert_eq!(
            texts_and_flags(&i),
            [run_of("ab", &[]), run_of("cd ", &[2]), run_of("ef", &[])]
        );

        let mut i = Interpreter::new(&fonts);
        i.run(&ops("BT /F1 10 Tf 12 TL (ab) Tj (cd) ' [-400 (ef)] TJ ET"))
            .unwrap();
        assert_eq!(
            texts_and_flags(&i),
            [run_of("ab", &[]), run_of("cd ", &[2]), run_of("ef", &[])],
            "' draws 'cd' after its line move, so the gap is after 'cd'"
        );

        let mut i = Interpreter::new(&fonts);
        i.run(&ops(
            "BT /F1 10 Tf 12 TL (ab) Tj 0 0 (cd) \" [-400 (ef)] TJ ET",
        ))
        .unwrap();
        assert_eq!(
            texts_and_flags(&i),
            [run_of("ab", &[]), run_of("cd ", &[2]), run_of("ef", &[])],
            "\" draws 'cd' after its line move, so the gap is after 'cd'"
        );

        let mut i = Interpreter::new(&fonts);
        i.run(&ops(
            "BT /F1 10 Tf (ab) Tj 1 0 0 1 0 0 Tm [<81> -400 (cd)] TJ ET",
        ))
        .unwrap();
        // WinAnsi leaves 0x81 unmapped, so this proves the drop path was reached.
        assert_eq!(i.dropped_runs, 1, "the <81> run must be dropped");
        assert_eq!(
            texts_and_flags(&i),
            [run_of("ab ", &[2]), run_of("cd", &[])],
            "a dropped string still drew text, so the gap after it lands on the last run kept"
        );
    }
}
