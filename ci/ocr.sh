#!/usr/bin/env bash
# The recognizer behind `--ocr` (docs/35-OCR-SCOPE.md): fmt, clippy and its tests, in its own
# workspace on its own pinned toolchain (ocr/rust-toolchain.toml, 1.98.0; the engine stays on
# 1.88.0). Its fixture test compares every answer's bytes with ocr/tests/data/expected.jsonl,
# written on Apple silicon, so on any other CPU this is the same-bytes check of §10.1.
#
# Release mode: the fixture test runs both models over five pages, which a debug build of rten
# takes minutes to do.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../ocr"
want="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' rust-toolchain.toml | head -1)"
rustup toolchain install "$want" --profile minimal --component rustfmt,clippy >/dev/null
got="$(cargo --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)"
echo "ocr/rust-toolchain.toml wants $want; cargo reports $got"
[ "$want" = "$got" ] || { echo "ocr: toolchain pin not in force" >&2; exit 1; }
cargo fmt --check
cargo clippy --release --all-targets --locked -- -D warnings
cargo test --release --locked
