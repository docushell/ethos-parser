use rten_simd::ops::{BitOps, FloatOps, NumOps};
use rten_simd::{Isa, Simd, SimdOp, SimdUnaryOp};

use crate::Exp;

/// Partial sums every reduction keeps, whatever the CPU's vector width.
///
/// Ethos fork: element `i` adds into partial `i % SUM_LANES`, in input order, and the
/// partials then add in one fixed tree. Every instruction set's lane count (4, 8 or 16)
/// divides `SUM_LANES`, so the additions, and the result's bits, are the same on every CPU.
/// Upstream folds per hardware lane, so its bits change with the vector width.
pub const SUM_LANES: usize = 16;

/// Folds `xs` into [`SUM_LANES`] partials with `fold(partial, x)`, then adds the partials
/// in a fixed tree.
#[inline(always)]
pub(crate) fn fixed_order_sum<O: NumOps<f32>>(
    ops: O,
    xs: &[f32],
    mut fold: impl FnMut(O::Simd, O::Simd) -> O::Simd,
) -> f32 {
    let n = ops.len();
    assert!(SUM_LANES % n == 0 && SUM_LANES / n <= 4);
    let regs = SUM_LANES / n;
    let mut acc = [ops.zero(); 4];
    let mut rest = xs;
    while let Some((block, tail)) = rest.split_at_checked(SUM_LANES) {
        for (k, a) in acc.iter_mut().enumerate().take(regs) {
            // Safety: `block` holds `SUM_LANES = regs * n` elements.
            let x = unsafe { ops.load_ptr(block.as_ptr().add(k * n)) };
            *a = fold(*a, x);
        }
        rest = tail;
    }
    // Element `j` of the tail still adds into partial `j`; padding lanes keep their partial.
    for (k, a) in acc.iter_mut().enumerate().take(regs) {
        if k * n >= rest.len() {
            break;
        }
        let (x, mask) = ops.load_pad(&rest[k * n..]);
        *a = ops.select(fold(*a, x), *a, mask);
    }
    let mut parts = [0f32; SUM_LANES];
    for (k, a) in acc.iter().enumerate().take(regs) {
        parts[k * n..(k + 1) * n].copy_from_slice(a.to_array().as_ref());
    }
    let mut width = SUM_LANES / 2;
    while width > 0 {
        for i in 0..width {
            parts[i] += parts[i + width];
        }
        width /= 2;
    }
    parts[0]
}

/// Computes the sum of a sequence of numbers.
///
/// This is more efficient than `slice.iter().sum()` as it computes multiple
/// partial sums in parallel using SIMD and then sums across the SIMD lanes at
/// the end. This will produce very slightly different results because the
/// additions are happening in a different order.
pub struct Sum<'a> {
    input: &'a [f32],
}

impl<'a> Sum<'a> {
    pub fn new(input: &'a [f32]) -> Self {
        Sum { input }
    }
}

impl SimdOp for Sum<'_> {
    type Output = f32;

    #[inline(always)]
    fn eval<I: Isa>(self, isa: I) -> Self::Output {
        let ops = isa.f32();
        fixed_order_sum(ops, self.input, |sum, x| ops.add(sum, x))
    }
}

/// Computes the sum of squares of a sequence of numbers.
///
/// This is conceptually equivalent to `slice.iter().map(|&x| x * x).sum()` but
/// more efficient as it computes multiple partial sums in parallel using SIMD
/// and then sums across the SIMD lanes at the end. This will produce very
/// slightly different results because the additions are happening in a
/// different order.
pub struct SumSquare<'a> {
    input: &'a [f32],
}

impl<'a> SumSquare<'a> {
    pub fn new(input: &'a [f32]) -> Self {
        SumSquare { input }
    }
}

impl SimdOp for SumSquare<'_> {
    type Output = f32;

    #[inline(always)]
    fn eval<I: Isa>(self, isa: I) -> Self::Output {
        let ops = isa.f32();
        fixed_order_sum(ops, self.input, |sum, x| ops.mul_add(x, x, sum))
    }
}

/// Computes the sum of absolute values of a sequence of numbers.
pub struct SumAbs<'a> {
    input: &'a [f32],
}

impl<'a> SumAbs<'a> {
    pub fn new(input: &'a [f32]) -> Self {
        SumAbs { input }
    }
}

impl SimdOp for SumAbs<'_> {
    type Output = f32;

    #[inline(always)]
    fn eval<I: Isa>(self, isa: I) -> Self::Output {
        let ops = isa.f32();
        fixed_order_sum(ops, self.input, |sum, x| ops.add(sum, ops.abs(x)))
    }
}

/// Compute the sum of squares of input with a bias subtracted.
///
/// This is a variant of [`SumSquare`] which subtracts a constant value from each
/// element before squaring it. A typical use case is to compute the variance of
/// a sequence, which is defined as `mean((X - x_mean)^2)`.
pub struct SumSquareSub<'a> {
    input: &'a [f32],
    offset: f32,
}

impl<'a> SumSquareSub<'a> {
    pub fn new(input: &'a [f32], offset: f32) -> Self {
        SumSquareSub { input, offset }
    }
}

impl SimdOp for SumSquareSub<'_> {
    type Output = f32;

    #[inline(always)]
    fn eval<I: Isa>(self, isa: I) -> Self::Output {
        let ops = isa.f32();
        let offset_vec = ops.splat(self.offset);

        fixed_order_sum(ops, self.input, |sum, x| {
            let x_offset = ops.sub(x, offset_vec);
            ops.mul_add(x_offset, x_offset, sum)
        })
    }
}

/// Compute the sum of `exp(x - offset)` over the input.
pub struct SumExpSub<'a> {
    input: &'a [f32],
    offset: f32,
}

impl<'a> SumExpSub<'a> {
    pub fn new(input: &'a [f32], offset: f32) -> Self {
        SumExpSub { input, offset }
    }
}

impl SimdOp for SumExpSub<'_> {
    type Output = f32;

    #[inline(always)]
    fn eval<I: Isa>(self, isa: I) -> Self::Output {
        let ops = isa.f32();
        let offset_vec = ops.splat(self.offset);

        fixed_order_sum(ops, self.input, |sum, x| {
            let exp = Exp::apply(isa, ops.sub(x, offset_vec));
            ops.add(sum, exp)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ulp::assert_ulp_diff_le;

    use super::{Sum, SumAbs, SumExpSub, SumSquare, SumSquareSub};
    use rten_simd::SimdOp;

    // Chosen to not be a multiple of vector size, so that tail handling is
    // exercised.
    const LEN: usize = 100;

    #[test]
    fn test_sum() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let expected_sum: f64 = xs.iter().map(|x| *x as f64).sum();
        let sum = Sum::new(&xs).dispatch();
        assert_ulp_diff_le!(sum, expected_sum as f32, 1.0);
    }

    #[test]
    fn test_sum_square() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let expected_sum: f64 = xs.iter().copied().map(|x| x as f64 * x as f64).sum();
        let sum = SumSquare::new(&xs).dispatch();
        assert_ulp_diff_le!(sum, expected_sum as f32, 2.0);
    }

    #[test]
    fn test_sum_abs() {
        let xs: Vec<f32> = (0..LEN).map(|i| (i as f32 * 0.1) - 5.0).collect();
        let expected_sum: f64 = xs.iter().map(|x| (*x as f64).abs()).sum();
        let sum = SumAbs::new(&xs).dispatch();
        assert_ulp_diff_le!(sum, expected_sum as f32, 2.0);
    }

    #[test]
    fn test_sum_square_sub() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let mean = xs.iter().sum::<f32>() / xs.len() as f32;
        let expected_sum: f64 = xs
            .iter()
            .copied()
            .map(|x| (x as f64 - mean as f64) * (x as f64 - mean as f64))
            .sum();
        let sum = SumSquareSub::new(&xs, mean).dispatch();
        assert_ulp_diff_le!(sum, expected_sum as f32, 2.0);
    }

    #[test]
    fn test_sum_exp_sub() {
        let xs: Vec<f32> = (0..LEN).map(|i| i as f32 * 0.1).collect();
        let max = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let expected_sum: f64 = xs
            .iter()
            .copied()
            .map(|x| (x as f64 - max as f64).exp())
            .sum();
        let sum = SumExpSub::new(&xs, max).dispatch();
        assert_ulp_diff_le!(sum, expected_sum as f32, 2.0);
    }
}
