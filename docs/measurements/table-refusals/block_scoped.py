#!/usr/bin/env python3
"""T2: does scoping the unruled candidate to a block collapse the lattice?

# The prediction this tests

[`README.md`](README.md) §4b found the unruled rule's candidate is not a table candidate at all: a
median of **4 628 faces on a page of prose and 6 672 on one holding a table**, against a
[`MAX_FACES`](../../../crates/ethos-parser-pdf/src/unruled.rs) ceiling of 4 096. `fold` over every
run's x-origin on a whole page — header, footer, body and all — is a histogram of where words start.
Its conclusion was that a candidate needs a **bounded region**, and that the block cut is what
produces one.

The block cut now ships (`gutter-columns-v3`), so the prediction is testable: **a block-scoped
lattice should come under the cap the page-scoped one blows through.** If it does, the unruled rule
has somewhere to stand and §5.3 is scopeable. If it does not, blocks are the wrong region and the
next lever is elsewhere.

# Why this is still a probe

The engine is not changed to find out whether changing it would help — `structelem.py`'s rule, and
the one §4b already followed once. Every `text_run` node carries `origin_x`, `origin_y`, `page` and
now `block`, and `unruled::fold` is eight lines, so both scopes are rebuilt here from one artifact.

# What it reports

Per document, for the page scope and the block scope side by side: faces implied, whether the
candidate clears `MAX_FACES`, and the same split by whether ground truth puts a `Table` on that
page. The number that decides §5.3 is **what fraction of candidates come under the cap**, because a
candidate over the cap is refused before any table question is asked.

    ETHOS_BENCH_CORPUS=~/ethos-external-benchmarks/opendataloader-bench \\
      docs/measurements/table-refusals/block_scoped.py
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

ALIGN_TOLERANCE = 150   # unruled.rs
MAX_FACES = 4096        # unruled.rs
MIN_LATTICE = 2         # detect() step 3: below 2x2 there is no candidate at all


def fold(values):
    out = []
    for v in sorted(values):
        if not out or v - out[-1] > ALIGN_TOLERANCE:
            out.append(v)
    return out


def pct(v, p):
    return v[min(len(v) - 1, int(len(v) * p))] if v else 0


def ground_truth():
    ref = json.loads((BENCH / "ground-truth/reference.json").read_text())
    out = {}
    for doc, v in ref.items():
        pages = collections.defaultdict(set)
        for e in v.get("elements", []):
            pages[e.get("page")].add(e.get("category"))
        out[doc] = dict(pages)
    return out


def candidates(nodes, scope):
    """`{key: faces}` for every candidate the rule would build at this scope.

    `scope` is "page" or "block". A run with no block is excluded from the block scope and
    counted separately — the rule declined there and no bounded region exists.
    """
    groups = collections.defaultdict(lambda: ([], []))
    unscoped = 0
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
                unscoped += 1
                continue
            key = (loc["page"], b)
        xs, ys = groups[key]
        xs.append(loc["origin_x"])
        ys.append(loc["origin_y"])

    out = {}
    for key, (xs, ys) in groups.items():
        cols, rows = fold(xs), fold(ys)
        if len(cols) < MIN_LATTICE or len(rows) < MIN_LATTICE:
            continue          # no candidate at all, per detect() step 3
        out[key] = len(cols) * len(rows)
    return out, unscoped


def main():
    if not ENGINE.exists():
        sys.exit(f"no engine at {ENGINE}")
    if not BENCH.exists():
        sys.exit(f"no corpus at {BENCH}; set ETHOS_BENCH_CORPUS")

    gt = ground_truth()
    faces = {"page": {True: [], False: []}, "block": {True: [], False: []}}
    unscoped_runs = 0
    total_runs = 0
    no_candidate = {"page": 0, "block": 0}

    for n, (doc, pages) in enumerate(sorted(gt.items()), 1):
        pdf = BENCH / "pdfs" / doc
        if not pdf.exists():
            continue
        r = subprocess.run([str(ENGINE), "extract", str(pdf)], capture_output=True)
        if r.returncode != 0:
            continue
        nodes = json.loads(r.stdout)["representation"]["nodes"]
        total_runs += sum(1 for x in nodes if x.get("kind") == "text_run")
        table_pages = {p for p, cats in pages.items() if "Table" in cats}

        for scope in ("page", "block"):
            cands, unscoped = candidates(nodes, scope)
            if scope == "block":
                unscoped_runs += unscoped
            if not cands:
                no_candidate[scope] += 1
            for key, f in cands.items():
                page = key if scope == "page" else key[0]
                faces[scope][page in table_pages].append(f)
        if n % 50 == 0:
            print(f"  ...{n}/{len(gt)}", flush=True)

    print(f"\ncorpus: {BENCH}")
    print(f"runs with no block (rule declined; no bounded region): "
          f"{unscoped_runs} of {total_runs} ({100*unscoped_runs//max(total_runs,1)}%)\n")

    print("=" * 78)
    print(f"Faces implied per candidate.  MAX_FACES = {MAX_FACES}")
    print("=" * 78)
    print(f"{'scope':<8}{'page holds':<13}{'n':>7}{'median':>9}{'p75':>9}{'max':>9}"
          f"{'under cap':>12}")
    for scope in ("page", "block"):
        for has in (True, False):
            v = sorted(faces[scope][has])
            if not v:
                continue
            under = sum(1 for f in v if f <= MAX_FACES)
            print(f"{scope:<8}{'a table' if has else 'no table':<13}{len(v):>7}"
                  f"{pct(v,.5):>9}{pct(v,.75):>9}{v[-1]:>9}"
                  f"{f'{100*under//len(v)}%':>12}")

    print()
    for scope in ("page", "block"):
        allv = faces[scope][True] + faces[scope][False]
        under = sum(1 for f in allv if f <= MAX_FACES)
        print(f"  {scope:<6} candidates under the cap: {under}/{len(allv)} "
              f"({100*under//max(len(allv),1)}%)")
        print(f"  {scope:<6} documents with no candidate at all: {no_candidate[scope]}")


if __name__ == "__main__":
    main()
