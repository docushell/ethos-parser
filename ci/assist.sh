#!/usr/bin/env bash
# The assist binary (assist/README.md): fmt, clippy and its tests, in its own workspace on its own
# pinned toolchain (assist/rust-toolchain.toml, 1.98.0, the recognizer's; the engine stays on
# 1.88.0). Its cargo-deny check is the gate's `deny` step, run from the repository root.
#
# Release mode, as ci/ocr.sh: clippy and the tests then share one optimized build of hayro.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../assist"
want="$(grep -E '^channel' rust-toolchain.toml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')"
rustup toolchain install "$want" --profile minimal --component rustfmt,clippy >/dev/null
got="$(cargo --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)"
echo "assist/rust-toolchain.toml wants $want; cargo reports $got"
[ "$want" = "$got" ] || { echo "assist: toolchain pin not in force" >&2; exit 1; }
cargo fmt --check
cargo clippy --release --all-targets --locked -- -D warnings
cargo test --release --locked
