# ethos-parser-assist

The assist lane's own process (decision #‹D4›). It renders a PDF's pages for a model to read. Later
steps add the model request; this one has no network code at all, and the engine does not call it
yet.

```text
ethos-parser-assist identity                  one JSON line: {"assist","version","renderer","request"}
ethos-parser-assist render PDF PAGE OUT.png   page PAGE (from 1) as a PNG; prints "dpi width height"
```

`render` exits 1 on a refusal (unreadable file, no such page, a page with no pixel size) and 2 on
bad usage.

## Why a workspace of its own

- hayro 0.7.1 needs Rust 1.92, and the engine is pinned to 1.88. This workspace uses the
  recognizer's toolchain, 1.98.0 (`rust-toolchain.toml`).
- In `ocr/`, hayro's crates would share `image`, `png` and `zune-jpeg` with the recognizer, so a
  change here could move `--ocr` output. So `assist/` has its own `Cargo.lock` and `deny.toml`, and
  the root and `ocr/` lockfiles do not change.

## The page rule, `pages-400dpi-5mp-v1`

This is decision #123's Ghostscript rule, rendered with hayro:

- dpi = min(400, floor(sqrt(5e6 / inches))), with the inches taken from the page's size in whole
  points.
- Pixel sizes are rounded as Ghostscript rounds them. hayro floors, which leaves 1 px short.
- A page still over 5e6 pixels is shrunk with Lanczos.
- hayro renders the CropBox, intersected with the MediaBox and rotated, on white, written as an
  RGB PNG.

The `renderer` string names this build, hayro's version and the rule. A unit test ties it to
`Cargo.toml` and `Cargo.lock`.

**Measured** (2026-11-15, Apple M4 Pro): page 1 of the 379 ParseBench PDFs sampled in Phase 4a
renders byte for byte the same as Phase 4a's renderer did, with the same `dpi width height` line.
vello_cpu picks SIMD paths at run time, so pixels on other CPUs are unmeasured.

## Build and check

```bash
cd assist
cargo build --release --locked
cargo fmt --check
cargo clippy --release --all-targets --locked -- -D warnings
cargo test --release --locked
```

`ci/assist.sh` runs the last four lines in `ci/gate.sh` and in CI. Both also run
`cargo deny --manifest-path assist/Cargo.toml check` from the repository root. cargo-deny then reads `assist/deny.toml`, which allows Apache-2.0, MIT, BSD-3-Clause, Zlib and
Unicode-3.0, has no GPL or AGPL, and copies the root's network bans.

hayro embeds data that is not a crate: the Foxit standard-14 fonts from PDFium (BSD-3-Clause),
Adobe's CMaps (BSD-3-Clause), a CGATS ICC profile (CC0) and LAB.icc (LCMS2). Their notices go in
`NOTICE` when the binary is released.
