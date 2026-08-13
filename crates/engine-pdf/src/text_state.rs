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

//! Matrices and text state (PDF 32000-1 §9.4.4).
//!
//! Float math lives here and inside `quantize`, and nowhere else. Everything that leaves this
//! module for the wire has been through [`engine_core::quantize`] into integer centipoints.

/// A 2-D affine transform, `[a b c d e f]`, as PDF writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// Row 1, column 1.
    pub a: f64,
    /// Row 1, column 2.
    pub b: f64,
    /// Row 2, column 1.
    pub c: f64,
    /// Row 2, column 2.
    pub d: f64,
    /// Translation in x.
    pub e: f64,
    /// Translation in y.
    pub f: f64,
}

impl Matrix {
    /// The identity transform.
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Construct from the six operands of `cm` or `Tm`.
    pub fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Pure translation.
    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            e: tx,
            f: ty,
            ..Self::IDENTITY
        }
    }

    /// `self × other`, in PDF's row-vector convention: apply `self` first, then `other`.
    pub fn then(self, other: Self) -> Self {
        Self {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    /// Transform a point. Test-only.
    ///
    /// The extractor composes matrices and reads [`Matrix::x_scale`]; it never transforms a bare
    /// point through this. Kept because it is what the composition tests assert against, and
    /// scoped to `cfg(test)` at M7 so that stays visible.
    #[cfg(test)]
    pub fn apply(self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }

    /// The horizontal scale this matrix applies, for turning a text-space advance into user
    /// space.
    pub fn x_scale(self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }
}

/// Graphics state that matters to text placement.
///
/// Colour, line width and dash patterns are deliberately absent: they cannot move a glyph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphicsState {
    /// The current transformation matrix.
    pub ctm: Matrix,
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: Matrix::IDENTITY,
        }
    }
}

/// Text state, per PDF 32000-1 Table 105, restricted to what affects position.
#[derive(Debug, Clone, PartialEq)]
pub struct TextState {
    /// Current font resource name, set by `Tf`.
    pub font_id: Option<String>,
    /// Font size in text-space units, set by `Tf`.
    pub font_size: f64,
    /// `Tc` — character spacing, unscaled text units.
    pub char_spacing: f64,
    /// `Tw` — word spacing, applied to single-byte code 32 only.
    pub word_spacing: f64,
    /// `Tz` — horizontal scaling, stored as a **fraction** (100% → 1.0).
    ///
    /// pdf-inspector implements no `Tz` at all, so its advance widths are wrong on any document
    /// that uses it (parity checklist P6-adjacent).
    pub horizontal_scale: f64,
    /// `TL` — leading.
    pub leading: f64,
    /// `Ts` — rise.
    pub rise: f64,
    /// `Tr` — rendering mode. Mode 3 is invisible text.
    pub render_mode: i64,
    /// The text matrix.
    pub text_matrix: Matrix,
    /// The text line matrix — where the current line began.
    pub line_matrix: Matrix,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            font_id: None,
            font_size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            horizontal_scale: 1.0,
            leading: 0.0,
            rise: 0.0,
            render_mode: 0,
            text_matrix: Matrix::IDENTITY,
            line_matrix: Matrix::IDENTITY,
        }
    }
}

impl TextState {
    /// `BT` — reset both text matrices to the identity.
    pub fn begin_text(&mut self) {
        self.text_matrix = Matrix::IDENTITY;
        self.line_matrix = Matrix::IDENTITY;
    }

    /// `Td` — move to the start of the next line, offset from the current line start.
    pub fn next_line_offset(&mut self, tx: f64, ty: f64) {
        self.line_matrix = Matrix::translate(tx, ty).then(self.line_matrix);
        self.text_matrix = self.line_matrix;
    }

    /// `T*` — move down by the leading.
    pub fn next_line(&mut self) {
        let leading = self.leading;
        self.next_line_offset(0.0, -leading);
    }

    /// `Tm` — set both matrices.
    pub fn set_matrix(&mut self, m: Matrix) {
        self.text_matrix = m;
        self.line_matrix = m;
    }

    /// The rendering matrix for a glyph, per §9.4.4.
    pub fn rendering_matrix(&self, ctm: Matrix) -> Matrix {
        let scale = Matrix::new(
            self.font_size * self.horizontal_scale,
            0.0,
            0.0,
            self.font_size,
            0.0,
            self.rise,
        );
        scale.then(self.text_matrix).then(ctm)
    }

    /// Advance the text matrix after showing a glyph.
    ///
    /// `tx = ((w0 − Tj/1000) × Tfs + Tc + Tw) × Th`, where `Tw` applies only to a single-byte
    /// code 32. **`Th` multiplies the whole expression**, including the spacing terms — omitting
    /// it, as pdf-inspector does by not implementing `Tz` at all, misplaces every subsequent
    /// glyph on the line.
    pub fn advance(&mut self, w0: f64, is_single_byte_space: bool) {
        let tx = (w0 * self.font_size
            + self.char_spacing
            + if is_single_byte_space {
                self.word_spacing
            } else {
                0.0
            })
            * self.horizontal_scale;
        self.text_matrix = Matrix::translate(tx, 0.0).then(self.text_matrix);
    }

    /// Apply a `TJ` positioning adjustment, in thousandths of a text-space unit.
    ///
    /// The sign is inverted: a positive number moves *left*.
    pub fn adjust(&mut self, amount_thousandths: f64) {
        let tx = -amount_thousandths / 1000.0 * self.font_size * self.horizontal_scale;
        self.text_matrix = Matrix::translate(tx, 0.0).then(self.text_matrix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn identity_is_neutral() {
        let m = Matrix::new(2.0, 0.0, 0.0, 3.0, 4.0, 5.0);
        assert_eq!(m.then(Matrix::IDENTITY), m);
        assert_eq!(Matrix::IDENTITY.then(m), m);
    }

    #[test]
    fn translation_composes_in_pdf_order() {
        // Apply a translation, then a scale: the translation is scaled too.
        let t = Matrix::translate(10.0, 20.0);
        let s = Matrix::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0);
        let m = t.then(s);
        let (x, y) = m.apply(0.0, 0.0);
        assert!(approx(x, 20.0) && approx(y, 40.0));
    }

    #[test]
    fn td_offsets_from_the_line_start_not_the_current_position() {
        let mut ts = TextState::default();
        ts.next_line_offset(72.0, 100.0);
        assert!(approx(ts.text_matrix.e, 72.0) && approx(ts.text_matrix.f, 100.0));

        // A second Td is relative to the LINE matrix, so the x resets rather than accumulating
        // from wherever the last glyph left off.
        ts.advance(0.5, false); // moves the text matrix only
        ts.next_line_offset(0.0, -36.0);
        assert!(
            approx(ts.text_matrix.e, 72.0),
            "x returns to the line origin, got {}",
            ts.text_matrix.e
        );
        assert!(approx(ts.text_matrix.f, 64.0));
    }

    #[test]
    fn t_star_moves_down_by_the_leading() {
        let mut ts = TextState {
            leading: 14.0,
            ..Default::default()
        };
        ts.next_line_offset(10.0, 100.0);
        ts.next_line();
        assert!(approx(ts.text_matrix.f, 86.0));
        assert!(approx(ts.text_matrix.e, 10.0));
    }

    /// `Tz` scales the advance. pdf-inspector implements none of this.
    #[test]
    fn horizontal_scaling_multiplies_the_advance() {
        let base = {
            let mut ts = TextState {
                font_size: 10.0,
                ..Default::default()
            };
            ts.advance(0.5, false);
            ts.text_matrix.e
        };
        assert!(approx(base, 5.0));

        let scaled = {
            let mut ts = TextState {
                font_size: 10.0,
                horizontal_scale: 0.5,
                ..Default::default()
            };
            ts.advance(0.5, false);
            ts.text_matrix.e
        };
        assert!(
            approx(scaled, 2.5),
            "Tz 50 should halve the advance, got {scaled}"
        );
    }

    #[test]
    fn horizontal_scaling_also_multiplies_the_spacing_terms() {
        // The common mistake is scaling only the glyph width. Tc and Tw are inside the bracket.
        let mut ts = TextState {
            font_size: 10.0,
            char_spacing: 2.0,
            horizontal_scale: 2.0,
            ..Default::default()
        };
        ts.advance(0.5, false);
        // ((0.5 * 10) + 2) * 2 = 14
        assert!(approx(ts.text_matrix.e, 14.0), "got {}", ts.text_matrix.e);
    }

    #[test]
    fn word_spacing_applies_only_to_a_single_byte_space() {
        let mut with = TextState {
            font_size: 10.0,
            word_spacing: 5.0,
            ..Default::default()
        };
        with.advance(0.5, true);
        assert!(approx(with.text_matrix.e, 10.0));

        let mut without = TextState {
            font_size: 10.0,
            word_spacing: 5.0,
            ..Default::default()
        };
        without.advance(0.5, false);
        assert!(approx(without.text_matrix.e, 5.0));
    }

    #[test]
    fn tj_adjustments_move_left_for_positive_values() {
        let mut ts = TextState {
            font_size: 10.0,
            ..Default::default()
        };
        ts.adjust(1000.0);
        assert!(approx(ts.text_matrix.e, -10.0), "positive TJ moves left");

        let mut ts2 = TextState {
            font_size: 10.0,
            ..Default::default()
        };
        ts2.adjust(-500.0);
        assert!(approx(ts2.text_matrix.e, 5.0), "negative TJ opens a gap");
    }

    #[test]
    fn tj_adjustments_are_scaled_by_tz_too() {
        let mut ts = TextState {
            font_size: 10.0,
            horizontal_scale: 0.5,
            ..Default::default()
        };
        ts.adjust(-1000.0);
        assert!(approx(ts.text_matrix.e, 5.0), "got {}", ts.text_matrix.e);
    }

    #[test]
    fn bt_resets_both_matrices() {
        let mut ts = TextState::default();
        ts.set_matrix(Matrix::new(1.0, 0.0, 0.0, 1.0, 50.0, 60.0));
        ts.begin_text();
        assert_eq!(ts.text_matrix, Matrix::IDENTITY);
        assert_eq!(ts.line_matrix, Matrix::IDENTITY);
    }

    #[test]
    fn the_rendering_matrix_carries_font_size_and_scale() {
        let ts = TextState {
            font_size: 24.0,
            horizontal_scale: 0.5,
            ..Default::default()
        };
        let m = ts.rendering_matrix(Matrix::IDENTITY);
        assert!(approx(m.a, 12.0), "font size times Tz");
        assert!(approx(m.d, 24.0), "vertical scale is unaffected by Tz");
    }

    #[test]
    fn rise_lifts_the_baseline() {
        let ts = TextState {
            font_size: 10.0,
            rise: 3.0,
            ..Default::default()
        };
        let m = ts.rendering_matrix(Matrix::IDENTITY);
        assert!(approx(m.f, 3.0));
    }
}
