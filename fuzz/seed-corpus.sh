#!/usr/bin/env bash
# Populate a fuzz target's working corpus from the committed seeds.
#
# Two sources, and the split is deliberate:
#
#   fuzz/seeds/            hand-authored degenerate and minimal inputs, committed here
#   fixtures/engine/       the five CC0 fixtures this repo owns, already committed
#
# The Ethos conformance corpus is NOT seeded from. `docs/04-ARCHITECTURE.md` §4 makes those
# fixtures read-only and referenced by hash, never copied into this tree, and a fuzz corpus is a
# copy. The benchmark documents are excluded for a different reason: the 492-page NIST PDF is
# 5.9 MB, and libFuzzer mutates whole inputs — a corpus entry that large makes every iteration
# slower without exploring anything the 1 KB fixtures do not already reach.
#
# Usage:
#   fuzz/seed-corpus.sh open_and_classify
#   fuzz/seed-corpus.sh open_and_extract

set -euo pipefail

target="${1:?usage: seed-corpus.sh <fuzz target name>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(dirname "$here")"
corpus="$here/corpus/$target"

mkdir -p "$corpus"

for f in "$here"/seeds/*; do
  cp "$f" "$corpus/seed-$(basename "$f")"
done

for f in "$repo"/fixtures/engine/*/document.pdf; do
  cp "$f" "$corpus/engine-$(basename "$(dirname "$f")").pdf"
done

echo "seeded $corpus with $(find "$corpus" -type f | wc -l | tr -d ' ') input(s)"
