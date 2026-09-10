#!/usr/bin/env python3
"""T2b: run the whole unruled precondition chain at block scope, and see which gate decides.

# What `block_scoped.py` established, and what it left open

Scoping the candidate to a block collapses the lattice from a median of 4 628-6 672 faces to
154-208, and takes the fraction under `MAX_FACES` from 41% to 95%. That is necessary and not
sufficient: clearing the size cap only means the candidate now REACHES the preconditions that
actually test whether a grid is there.

Page-scoped, most candidates died at the cap or at the leftmost word gap. So the occupancy
precondition — *every face of the lattice must hold a run, or the grid is one this engine drew
rather than one the document implied* — had almost never run on real text. This runs the full chain
at both scopes and reports **which gate decides**, split by whether ground truth puts a table there.

# The chain, in `detect`'s own order

1. **>= 2x2** — below it there is no candidate at all, which is not a refusal.
2. **`faces <= MAX_FACES`** — tested before the gutter floor since 2026-09-10.
3. **The gutter floor** — every adjacent column line >= 1200 centipoints apart, every row line
   >= 600. This is the one page scope made meaningless: the leftmost sub-12pt gap on any page
   holding prose is word spacing, found and reported whether or not a table sits elsewhere. At
   block scope a block that IS a table has real gutters, so the test may mean something again.
4. **Occupancy** — `occupants == faces`, no empty face.
5. Emission order is not simulated: it needs the page's run sequence and cannot change a refusal
   into an acceptance.

# What a pass means and does not mean

A candidate passing all four is one the rule **would emit**. It is not necessarily a table: this
measures the rule, not the truth. Ground truth says whether a `Table` is on that page, not whether
it is this block, so a pass on a table page is corroboration and not a hit. That asymmetry is why
the output reports both columns rather than a score.

    ETHOS_BENCH_CORPUS=~/ethos-external-benchmarks/opendataloader-bench \\
      docs/measurements/table-refusals/preconditions.py
"""

import collections
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
ENGINE = REPO / "target/release/ethos-parser"
BENCH = Path(
    os.environ.get("ETHOS_BENCH_CORPUS", Path.home() / "ethos-external-benchmarks/opendataloader-bench")
).expanduser()

ALIGN_TOLERANCE = 150
MAX_FACES = 4096
COLUMN_GUTTER_MIN = 1200
ROW_GUTTER_MIN = 600


def fold(values):
    out = []
    for v in sorted(values):
        if not out or v - out[-1] > ALIGN_TOLERANCE:
            out.append(v)
    return out


def line_of(lines, v):
    lo, hi = 0, len(lines) - 1
    best = 0
    while lo <= hi:
        mid = (lo + hi) // 2
        if lines[mid] <= v:
            best, lo = mid, mid + 1
        else:
            hi = mid - 1
    return best


def verdict(xs, ys):
    """Which gate decides this candidate: one of the chain's names, or 'would emit'."""
    cols, rows = fold(xs), fold(ys)
    if len(cols) < 2 or len(rows) < 2:
        return "no candidate"
    faces = len(cols) * len(rows)
    if faces > MAX_FACES:
        return "lattice_too_large"
    if any(b - a < COLUMN_GUTTER_MIN for a, b in zip(cols, cols[1:])):
        return "gutter_below_floor"
    if any(b - a < ROW_GUTTER_MIN for a, b in zip(rows, rows[1:])):
        return "gutter_below_floor"
    occupied = {(line_of(rows, y), line_of(cols, x)) for x, y in zip(xs, ys)}
    if len(occupied) != faces:
        return "faces_without_text"
    return "WOULD EMIT"


def ground_truth():
    ref = json.loads((BENCH / "ground-truth/reference.json").read_text())
    out = {}
    for doc, v in ref.items():
        pages = collections.defaultdict(set)
        for e in v.get("elements", []):
            pages[e.get("page")].add(e.get("category"))
        out[doc] = dict(pages)
    return out


def group(nodes, scope):
    g = collections.defaultdict(lambda: ([], []))
    for n in nodes:
        if n.get("kind") != "text_run":
            continue
        loc = (n.get("native_locator") or {}).get("pdf")
        if not loc:
            continue
        attrs = (n.get("attributes") or {}).get("text_run") or {}
        if scope == "page":
            key = loc["page"]
        else:
            b = attrs.get("block")
            if b is None:
                continue
            key = (loc["page"], b)
        xs, ys = g[key]
        xs.append(loc["origin_x"])
        ys.append(loc["origin_y"])
    return g


def main():
    if not ENGINE.exists():
        sys.exit(f"no engine at {ENGINE}")
    gt = ground_truth()
    tally = {s: collections.Counter() for s in ("page", "block")}
    emit_docs = {s: set() for s in ("page", "block")}
    gt_table_docs = set()

    for n, (doc, pages) in enumerate(sorted(gt.items()), 1):
        pdf = BENCH / "pdfs" / doc
        if not pdf.exists():
            continue
        r = subprocess.run([str(ENGINE), "extract", str(pdf)], capture_output=True)
        if r.returncode != 0:
            continue
        nodes = json.loads(r.stdout)["representation"]["nodes"]
        table_pages = {p for p, cats in pages.items() if "Table" in cats}
        if table_pages:
            gt_table_docs.add(doc)

        for scope in ("page", "block"):
            for key, (xs, ys) in group(nodes, scope).items():
                page = key if scope == "page" else key[0]
                v = verdict(xs, ys)
                tally[scope][(v, page in table_pages)] += 1
                if v == "WOULD EMIT":
                    emit_docs[scope].add(doc)
        if n % 50 == 0:
            print(f"  ...{n}/{len(gt)}", flush=True)

    order = ["WOULD EMIT", "faces_without_text", "gutter_below_floor",
             "lattice_too_large", "no candidate"]
    for scope in ("page", "block"):
        print("\n" + "=" * 74)
        print(f"{scope.upper()} SCOPE — which gate decides each candidate")
        print("=" * 74)
        print(f"{'gate':<24}{'total':>8}{'page HAS table':>17}{'page has none':>16}")
        for name in order:
            y = tally[scope][(name, True)]
            n_ = tally[scope][(name, False)]
            if y + n_ == 0:
                continue
            print(f"{name:<24}{y + n_:>8}{y:>17}{n_:>16}")
        e = emit_docs[scope]
        print(f"\n  documents with >=1 candidate that would emit: {len(e)}")
        print(f"  of those, ground truth holds a Table         : {len(e & gt_table_docs)}")
        print(f"  documents whose ground truth holds a Table   : {len(gt_table_docs)}")


if __name__ == "__main__":
    main()
