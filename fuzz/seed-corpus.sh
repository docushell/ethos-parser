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
# v2-S12 adds a third source, for the office target only:
#
#   fixtures/office/       the sixteen office packages this repo owns, one of every shape
#
# Same rule as `fixtures/engine/`: engine-owned, already committed, small. Every one is a valid
# package of a DIFFERENT shape — an OOXML central directory, an ODF `mimetype` stored first, an
# EPUB OCF container chain, an RTF brace-group stream that is not a container at all — which is
# the seed set A11 asks for, already in the tree. libFuzzer mutating a valid package is how the
# central-directory reader gets reached at all: random bytes almost never open like a ZIP, so an
# unseeded office campaign would spend its whole budget being refused at the first predicate.
#
# Usage:
#   fuzz/seed-corpus.sh open_and_classify
#   fuzz/seed-corpus.sh open_and_extract
#   fuzz/seed-corpus.sh office_read

set -euo pipefail

target="${1:?usage: seed-corpus.sh <fuzz target name>}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(dirname "$here")"
corpus="$here/corpus/$target"

mkdir -p "$corpus"

# The office target takes the office packages and nothing else: a PDF is refused by
# `engine_office::read` at the first predicate, so seeding it with one buys no coverage and
# costs an entry in every mutation round.
if [ "$target" = "office_read" ]; then
  for f in "$repo"/fixtures/office/*/*; do
    case "$f" in
      *.py | *.md | */__pycache__/*) continue ;;
    esac
    [ -f "$f" ] || continue
    cp "$f" "$corpus/office-$(basename "$(dirname "$f")")-$(basename "$f")"
  done
  echo "seeded $corpus with $(find "$corpus" -type f | wc -l | tr -d ' ') input(s)"
  exit 0
fi

for f in "$here"/seeds/*; do
  cp "$f" "$corpus/seed-$(basename "$f")"
done

for f in "$repo"/fixtures/engine/*/document.pdf; do
  cp "$f" "$corpus/engine-$(basename "$(dirname "$f")").pdf"
done

echo "seeded $corpus with $(find "$corpus" -type f | wc -l | tr -d ' ') input(s)"
