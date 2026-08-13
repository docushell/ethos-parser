// Copyright 2026 The ethos-engine maintainers
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

//! The content-stream interpreter (`docs/05-MILESTONES.md` M3).
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

use engine_core::EngineError;

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
    pub advance: Option<f64>,
    /// Font resource name.
    pub font_id: String,
    /// Font size in text space.
    pub font_size: f64,
    /// Marked-content id in force, if any.
    pub mcid: Option<i64>,
    /// Indices into `text` this reader inserted.
    pub synthesized_indices: Vec<u32>,
}

/// Interprets one page's content stream.
pub struct Interpreter<'a> {
    fonts: &'a std::collections::BTreeMap<String, Font>,
    gs_stack: Vec<GraphicsState>,
    gs: GraphicsState,
    ts: TextState,
    mcid_stack: Vec<Option<i64>>,
    /// Runs collected so far.
    pub shown: Vec<ShownText>,
    /// Codes the decoder could not map, as a typed diagnostic rather than a silent drop.
    pub undecodable: Vec<String>,
}

impl<'a> Interpreter<'a> {
    /// Start an interpreter over a page's fonts.
    pub fn new(fonts: &'a std::collections::BTreeMap<String, Font>) -> Self {
        Self {
            fonts,
            gs_stack: Vec::new(),
            gs: GraphicsState::default(),
            ts: TextState::default(),
            mcid_stack: Vec::new(),
            shown: Vec::new(),
            undecodable: Vec::new(),
        }
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
                Err(e)
            }
        }
    }

    fn run_inner(&mut self, ops: &[lopdf::content::Operation]) -> Result<(), EngineError> {
        for op in ops {
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

    /// The exhaustive match. **No wildcard arm.**
    fn dispatch(&mut self, op: Operator, operands: &[lopdf::Object]) -> Result<(), EngineError> {
        use Operator::*;
        match op {
            // --- graphics state: affects glyph placement through the CTM ---
            SaveState => self.gs_stack.push(self.gs),
            RestoreState => {
                self.gs = self.gs_stack.pop().unwrap_or_default();
            }
            ConcatMatrix => {
                let m = matrix_operands(operands)?;
                self.gs.ctm = m.then(self.gs.ctm);
            }

            // --- text object boundaries ---
            BeginText => self.ts.begin_text(),
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
            }
            NextLineSetLeading => {
                let (tx, ty) = (num(operands, 0)?, num(operands, 1)?);
                self.ts.leading = -ty;
                self.ts.next_line_offset(tx, ty);
            }
            SetTextMatrix => {
                let m = matrix_operands(operands)?;
                self.ts.set_matrix(m);
            }
            NextLineByLeading => self.ts.next_line(),

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
                let bytes = string_operand(operands, 0)?;
                self.show(&bytes, &[])?;
            }
            NextLineShowTextSpacing => {
                // `aw ac string "` — sets word and char spacing, then behaves as `'`.
                self.ts.word_spacing = num(operands, 0)?;
                self.ts.char_spacing = num(operands, 1)?;
                self.ts.next_line();
                let bytes = string_operand(operands, 2)?;
                self.show(&bytes, &[])?;
            }

            // --- marked content: the mcid bridge ---
            BeginMarkedContentProps => {
                self.mcid_stack.push(mcid_from_props(operands));
            }
            BeginMarkedContent => self.mcid_stack.push(None),
            EndMarkedContent => {
                self.mcid_stack.pop();
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
            // Path construction and painting: geometry that is not text.
            MoveTo | LineTo | CurveTo | CurveToV | CurveToY | ClosePath | Rectangle => {}
            Stroke
            | CloseStroke
            | Fill
            | FillObsolete
            | FillEvenOdd
            | FillStroke
            | FillStrokeEvenOdd
            | CloseFillStroke
            | CloseFillStrokeEvenOdd
            | EndPath => {}
            Clip | ClipEvenOdd => {}
            // Shading and images. `Do` may draw a form XObject containing text; this profile
            // does not descend into them, and that is a declared limitation rather than a
            // silent omission — see `extract`'s `not_decoded`.
            Shading | XObject => {}
            BeginInlineImage | InlineImageData | EndInlineImage => {}
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

        let trm = self.ts.rendering_matrix(self.gs.ctm);
        let origin = (trm.e, trm.f);

        let mut text = String::new();
        let mut kept_codes = Vec::with_capacity(codes.len());
        let mut advance_total = 0.0f64;
        let mut advance_known = true;

        for code in codes {
            match font.decode_code(code) {
                Ok(s) => {
                    text.push_str(&s);
                    kept_codes.push(code);
                }
                Err(e) => {
                    // A typed diagnostic, not a silent drop. The run continues so the rest of
                    // the page is still extractable, but the gap is recorded and surfaces in the
                    // artifact's `not_decoded` list.
                    self.undecodable.push(e.to_string());
                    return Err(e);
                }
            }

            match font.advance_glyph_space(code) {
                Some(w0) => {
                    let is_space = code == 32;
                    let before = self.ts.text_matrix.e;
                    self.ts.advance(w0, is_space);
                    advance_total += self.ts.text_matrix.e - before;
                }
                None => advance_known = false,
            }
        }

        let scale = self.gs.ctm.x_scale();
        self.shown.push(ShownText {
            text,
            codes: kept_codes,
            origin,
            advance: advance_known.then_some(advance_total * scale),
            font_id,
            font_size: self.ts.font_size,
            mcid: self.mcid_stack.last().copied().flatten(),
            synthesized_indices: synthesized.to_vec(),
        });

        Ok(())
    }

    /// `TJ` — strings interleaved with positioning adjustments.
    ///
    /// A sufficiently negative adjustment opens a word gap that the document never wrote as a
    /// space glyph. Emitting the space unflagged would put a character in the evidence the
    /// document does not contain, so it is inserted **and flagged at the point of creation**.
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
                    if amount <= TJ_SPACE_GAP_THOUSANDTHS {
                        // Attach the synthesized space to the run that just ended, so the flag
                        // sits with the character rather than being reconstructed later.
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
fn mcid_from_props(operands: &[lopdf::Object]) -> Option<i64> {
    match operands.get(1)? {
        lopdf::Object::Dictionary(d) => d.get(b"MCID").ok()?.as_i64().ok(),
        _ => None,
    }
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

    fn no_fonts() -> BTreeMap<String, Font> {
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

    /// A usable font, so a test can actually reach the show-text path.
    ///
    /// `no_fonts()` cannot: `Tf` on an empty map fails first, with `missing_part`, which is
    /// correct behaviour and the wrong thing to be testing when the subject is an operator.
    fn one_font() -> BTreeMap<String, Font> {
        use crate::encoding::{BaseEncoding, SimpleEncoding};
        use crate::fonts::{Decoder, WidthSource};
        use engine_core::GeometryAbsence;

        let mut m = BTreeMap::new();
        m.insert(
            "F1".to_string(),
            Font {
                id: "F1".into(),
                decoder: Decoder::Simple(SimpleEncoding::new(
                    BaseEncoding::WinAnsi,
                    BTreeMap::new(),
                )),
                widths: WidthSource::Widths {
                    first_char: 32,
                    widths: vec![500.0; 95],
                    type3_scale_x: None,
                },
                ink: crate::fonts::FontInk::Absent(GeometryAbsence::NotReportedByReader),
            },
        );
        m
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
        assert!(i.mcid_stack.is_empty(), "EMC pops the scope");

        let mut j = Interpreter::new(&fonts);
        j.run(&ops("/P <</MCID 7>> BDC")).unwrap();
        assert_eq!(j.mcid_stack.last().copied().flatten(), Some(7));

        let mut k = Interpreter::new(&fonts);
        k.run(&ops("/P BMC")).unwrap();
        assert_eq!(
            k.mcid_stack.last().copied().flatten(),
            None,
            "BMC has no MCID, and none is invented"
        );
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
}
