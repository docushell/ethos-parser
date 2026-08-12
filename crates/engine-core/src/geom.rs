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

//! Integer quanta and rectangles (`docs/01-CONTRACT.md` §4, §5).
//!
//! Floats do not exist in canonical output. `quantize` is the only permitted float math on the
//! canonical path: an IEEE-754 double multiply plus floor/ceil gives identical results on
//! identical inputs across supported platforms, which is what makes byte identity achievable.

use serde::{Deserialize, Serialize};

/// Largest magnitude allowed in a canonical value: 2^53 − 1, the ecosystem-safe integer bound.
///
/// Beyond this a JSON reader backed by an IEEE-754 double silently loses precision, so a value
/// that survives our own round-trip could still be corrupted by a conforming consumer.
pub const MAX_SAFE_INT: i64 = 9_007_199_254_740_991;

/// Quanta per point on the canonical path. One centipoint = 1/100 pt.
pub const QUANTUM_PER_POINT: u32 = 100;

/// A coordinate could not be quantized: non-finite input, or a result outside [`MAX_SAFE_INT`].
///
/// Deliberately not a saturating conversion. A clamped coordinate is an invented coordinate,
/// and `docs/01-CONTRACT.md` §8 requires failing closed instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuantizeError;

impl core::fmt::Display for QuantizeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("non-finite or out-of-range coordinate")
    }
}

impl std::error::Error for QuantizeError {}

/// Quantize a point-space coordinate: `round_half_away_from_zero(pts × quantum_per_point)`.
///
/// Rounding is half-away-from-zero, not banker's rounding: `0.005 → 1` and `-0.005 → -1` at
/// quantum 100. Half-to-even would make the sign of a coordinate change its magnitude, which is
/// surprising in a geometry context and harder to reproduce in another language.
///
/// # Precision limit, stated rather than hidden
///
/// The `(x + 0.5).floor()` idiom loses exactness once `|x| ≥ 2^52`, because `x + 0.5` is no
/// longer representable and rounds to `x + 1`. **Measured:** `quantize(4503599627370497.0, 1)`
/// returns `4503599627370498`, one more than the exact product, and `MAX_SAFE_INT` itself is
/// unreachable — it errors, because the intermediate rounds past the bound.
///
/// This is **deliberately not fixed.** The implementation is byte-identical to Ethos's, and
/// byte-identical output is the property the parity vectors exist to protect; diverging here to
/// chase an unreachable input would trade a real guarantee for a theoretical one. The affected
/// domain is unreachable for page geometry: at quantum 100, `2^52` quanta is about
/// 4.5 × 10^13 points, roughly 6 × 10^11 inches. Real page coordinates live below 10^4 points.
///
/// **Safe input domain:** `|pts × quantum_per_point| < 2^52`. Above it, results remain
/// deterministic and reproducible — they are simply not exact.
///
/// # Errors
///
/// Returns [`QuantizeError`] for:
///
/// - `NaN` or `±∞` input, or a product that overflows to non-finite
/// - a result whose magnitude exceeds [`MAX_SAFE_INT`]
/// - `quantum_per_point == 0`, which would silently collapse every coordinate to the origin.
///   Ethos does not reject this; refusing it is *stricter on emission*, which is the one
///   direction of divergence the contract permits (`docs/01-CONTRACT.md` §11).
pub fn quantize(pts: f64, quantum_per_point: u32) -> Result<i64, QuantizeError> {
    if quantum_per_point == 0 {
        return Err(QuantizeError);
    }
    if !pts.is_finite() {
        return Err(QuantizeError);
    }
    let scaled = pts * f64::from(quantum_per_point);
    if !scaled.is_finite() {
        return Err(QuantizeError);
    }
    let rounded = if scaled >= 0.0 {
        (scaled + 0.5).floor()
    } else {
        (scaled - 0.5).ceil()
    };
    if rounded.abs() > MAX_SAFE_INT as f64 {
        return Err(QuantizeError);
    }
    Ok(rounded as i64)
}

/// A rectangle in integer quanta, as `[x0, y0, x1, y1]` — left, top, right, bottom.
///
/// **Not `[x, y, width, height]`.** The grounding wire format is edge-based
/// (`docs/01-CONTRACT.md` §11), and a silent reinterpretation between the two conventions would
/// place every citation highlight in the wrong spot while remaining structurally valid.
///
/// Construction validates `x1 > x0 && y1 > y0`. A zero-area rectangle is rejected here even
/// though Ethos's own `QRect` tolerates `x0 == x1` — the engine is deliberately stricter on
/// emission than the oracle is on acceptance, which is the only safe direction (§11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "[i64; 4]", into = "[i64; 4]")]
pub struct QRect {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl QRect {
    /// Construct from edges, validating ordering and positive area.
    ///
    /// # Errors
    ///
    /// Returns [`QRectError`] when either edge pair is not strictly increasing, or when any
    /// coordinate exceeds [`MAX_SAFE_INT`].
    pub fn new(x0: i64, y0: i64, x1: i64, y1: i64) -> Result<Self, QRectError> {
        for v in [x0, y0, x1, y1] {
            if v.unsigned_abs() > MAX_SAFE_INT as u64 {
                return Err(QRectError::OutOfRange);
            }
        }
        if x1 <= x0 || y1 <= y0 {
            return Err(QRectError::NonPositiveArea);
        }
        Ok(Self { x0, y0, x1, y1 })
    }

    /// Left edge.
    pub fn x0(self) -> i64 {
        self.x0
    }
    /// Top edge.
    pub fn y0(self) -> i64 {
        self.y0
    }
    /// Right edge.
    pub fn x1(self) -> i64 {
        self.x1
    }
    /// Bottom edge.
    pub fn y1(self) -> i64 {
        self.y1
    }

    /// The wire form: `[x0, y0, x1, y1]`.
    pub fn to_array(self) -> [i64; 4] {
        [self.x0, self.y0, self.x1, self.y1]
    }
}

/// A rectangle could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QRectError {
    /// `x1 <= x0` or `y1 <= y0`. Includes the zero-area (degenerate) case.
    NonPositiveArea,
    /// A coordinate magnitude exceeds [`MAX_SAFE_INT`].
    OutOfRange,
}

impl core::fmt::Display for QRectError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NonPositiveArea => {
                f.write_str("rectangle has non-positive area (require x1 > x0 and y1 > y0)")
            }
            Self::OutOfRange => f.write_str("rectangle coordinate exceeds 2^53-1"),
        }
    }
}

impl std::error::Error for QRectError {}

impl TryFrom<[i64; 4]> for QRect {
    type Error = QRectError;

    fn try_from(v: [i64; 4]) -> Result<Self, Self::Error> {
        Self::new(v[0], v[1], v[2], v[3])
    }
}

impl From<QRect> for [i64; 4] {
    fn from(r: QRect) -> Self {
        r.to_array()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vectors from `docs/05-MILESTONES.md` M1. These are normative: they are also Ethos's own
    // vectors, so matching them is what makes the two implementations interchangeable.
    #[test]
    fn quantize_vectors_are_normative() {
        assert_eq!(quantize(0.005, 100).unwrap(), 1);
        assert_eq!(quantize(0.004, 100).unwrap(), 0);
        assert_eq!(quantize(-0.005, 100).unwrap(), -1);
        assert_eq!(quantize(612.0, 100).unwrap(), 61200);
        assert_eq!(quantize(0.0, 100).unwrap(), 0);
        assert_eq!(quantize(-0.0, 100).unwrap(), 0);

        assert_eq!(quantize(f64::NAN, 100), Err(QuantizeError));
        assert_eq!(quantize(f64::INFINITY, 100), Err(QuantizeError));
        assert_eq!(quantize(f64::NEG_INFINITY, 100), Err(QuantizeError));
        assert_eq!(quantize(1e17, 100), Err(QuantizeError));
    }

    #[test]
    fn negative_zero_never_reaches_the_wire() {
        // -0.0 and 0.0 are distinct floats but must produce the same integer, or two runs over
        // the same document could differ by a byte.
        let a = quantize(-0.0, QUANTUM_PER_POINT).unwrap();
        let b = quantize(0.0, QUANTUM_PER_POINT).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "0", "must not serialize as -0");
    }

    #[test]
    fn a_zero_quantum_is_refused_rather_than_collapsing_every_coordinate() {
        // Without this guard the product is always 0.0 and every coordinate on the page maps to
        // the origin, silently and with an Ok result — the worst shape a failure can take.
        assert_eq!(quantize(612.0, 0), Err(QuantizeError));
        assert_eq!(quantize(0.0, 0), Err(QuantizeError));
        assert_eq!(quantize(-1.0, 0), Err(QuantizeError));
    }

    /// The large-magnitude precision limit, pinned as behaviour rather than left as folklore.
    ///
    /// These values are *wrong* in the arithmetic sense and *right* in the contract sense: they
    /// match Ethos byte for byte, and byte identity is the property that matters. Documented in
    /// `quantize`'s rustdoc. If a future change makes these exact, it has diverged from the
    /// oracle and the parity vectors are no longer meaningful — that is the failure this test is
    /// here to make loud.
    #[test]
    fn the_precision_limit_above_2_pow_52_is_pinned() {
        // Exactly representable, and exact.
        assert_eq!(
            quantize(4_503_599_627_370_496.0, 1).unwrap(),
            4_503_599_627_370_496
        );

        // One above 2^52: `x + 0.5` is not representable and rounds up, so the result is one
        // more than the exact product.
        assert_eq!(
            quantize(4_503_599_627_370_497.0, 1).unwrap(),
            4_503_599_627_370_498
        );

        // MAX_SAFE_INT itself is unreachable: the intermediate rounds past the bound.
        assert_eq!(quantize(MAX_SAFE_INT as f64, 1), Err(QuantizeError));
        assert_eq!(
            quantize(9_007_199_254_740_990.0, 1).unwrap(),
            9_007_199_254_740_990
        );
    }

    #[test]
    fn the_safe_domain_is_exact() {
        // Everything a page can actually contain. At quantum 100 a US Letter page is 61200 x
        // 79200 quanta, thirteen orders of magnitude below the precision limit.
        for pts in [0.0, 1.0, 72.0, 612.0, 792.0, 1_000_000.0, 1e10] {
            let got = quantize(pts, 100).unwrap();
            let exact = (pts * 100.0) as i64;
            assert_eq!(
                got, exact,
                "quantize({pts}, 100) must be exact in the safe domain"
            );
        }
    }

    #[test]
    fn quantize_is_symmetric_about_zero() {
        for pts in [0.005, 0.004, 1.0, 612.0, 0.015, 2.5] {
            let pos = quantize(pts, 100).unwrap();
            let neg = quantize(-pts, 100).unwrap();
            assert_eq!(
                pos, -neg,
                "half-away-from-zero must be sign-symmetric at {pts}"
            );
        }
    }

    #[test]
    fn qrect_rejects_non_positive_area() {
        assert_eq!(QRect::new(10, 10, 10, 20), Err(QRectError::NonPositiveArea));
        assert_eq!(QRect::new(10, 10, 20, 10), Err(QRectError::NonPositiveArea));
        assert_eq!(QRect::new(20, 10, 10, 20), Err(QRectError::NonPositiveArea));
        assert!(QRect::new(10, 10, 20, 20).is_ok());
    }

    #[test]
    fn qrect_wire_order_is_edges_not_extents() {
        let r = QRect::new(100, 200, 300, 500).unwrap();
        assert_eq!(r.to_array(), [100, 200, 300, 500]);
        // If this were [x, y, w, h] the last two would be 200 and 300.
        assert_ne!(r.to_array(), [100, 200, 200, 300]);
    }

    #[test]
    fn qrect_round_trips_through_its_wire_form() {
        let r = QRect::new(1, 2, 3, 4).unwrap();
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "[1,2,3,4]");
        assert_eq!(serde_json::from_str::<QRect>(&json).unwrap(), r);
    }

    #[test]
    fn qrect_deserialization_enforces_the_same_rule_as_construction() {
        // A degenerate rectangle must not sneak in through serde.
        assert!(serde_json::from_str::<QRect>("[10,10,10,20]").is_err());
        assert!(serde_json::from_str::<QRect>("[10,10,20,20]").is_ok());
    }
}
