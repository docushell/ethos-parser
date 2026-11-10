# rten-vecmath 0.26.0, patched

Upstream: <https://github.com/robertknight/rten>, crate `rten-vecmath` 0.26.0, licensed
MIT OR Apache-2.0 (used here under Apache-2.0). Vendored so ethos-parser's recognizer gives the
same bytes on every CPU (docs/35-OCR-SCOPE.md).

**The one change.** Upstream sums a slice per SIMD lane and then across the lanes, so the order of
the additions, and the result's last bits, depend on the CPU's vector width: 4 lanes on NEON, 8 on
AVX2, 16 on AVX-512. `src/sum.rs` adds `fixed_order_sum`: element `i` adds into partial
`i % 16` in input order, and the 16 partials add in one fixed tree. Every lane count divides 16,
so every CPU performs the same additions. `Sum`, `SumSquare`, `SumAbs`, `SumSquareSub`,
`SumExpSub` (`src/sum.rs`) and the exponential sums of `Softmax` and `LogSoftmax`
(`src/softmax.rs`) use it; the changed places are marked "Ethos fork". Nothing else differs from
the published crate but one comment: `src/ulp.rs` cites Java's `Math.ulp` documentation by name
rather than by its URL.
