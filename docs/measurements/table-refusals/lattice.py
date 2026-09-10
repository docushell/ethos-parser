#!/usr/bin/env python3
"""T1b: rebuild the candidate lattice out-of-band, and ask whether it separates a table from prose.

# Why this is a probe and not an engine change

[`README.md`](README.md) §4 asked for a wider refusal disclosure: how many faults rather than the
first, the candidate's gutter distribution rather than one sample, the extent it reached. The
obvious way to get that is to add it to `Refusal::GutterBelowFloor` and its wire detail.

That would be the wrong order. The limitation detail sits inside `representation`, so every word of
it is covered by `representation_c14n_sha256` — and `unruled-table-candidate-refused` fires on 199
of 200 documents here, so enriching the prose moves the artifact hash of essentially the whole
corpus. Doing that to find out whether the enrichment is useful is the move
[`structelem.py`](../block-subdivision/structelem.py) refused in as many words: *"adding element
identity to the wire to measure whether element identity is useful would be changing the product to
justify changing the product."*

Everything needed is already emitted. Every `text_run` node carries `origin_x`, `origin_y` and
`page`, and `unruled::fold` is eight lines. So the lattice is rebuilt here instead.

# Fidelity, and where it is approximate

`fold` is reproduced exactly: sort, then keep a value only when it exceeds the last kept one by
more than `ALIGN_TOLERANCE` (150 centipoints).

`unruled::detect` runs on `leftover` — the runs no accepted ruled table already claims
(`tables.rs:353`). On this corpus the engine accepts a ruled table on 5 documents of 200, so on
the other 195 `leftover` is every run and the reproduction is exact. **The 5 are excluded from the
tables below** rather than approximated, and named.

Artifact runs are NOT filtered: `RunOrigin` at `extract.rs:518` is built from every run on the
page, headers and footers included, so this does the same.

# The question this answers

T1 showed the reported gap is `.find(|gap| *gap < floor)` — the FIRST fault, leftmost over sorted
lines — and that on single-page documents holding prose it is always word spacing. So the reported
gap said nothing about tables.

The lattice does not have that problem. If table pages carry more gutters ABOVE the floor than
prose pages do, the signal exists and the engine is discarding it by reporting one fault. If they
do not, no disclosure change will help and 5.3 needs a different lever entirely.

    ETHOS_BENCH_CORPUS=~/ethos-external-benchmarks/opendataloader-bench \\
      docs/measurements/table-refusals/lattice.py
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

ALIGN_TOLERANCE = 150     # unruled.rs
COLUMN_GUTTER_MIN = 1_200
ROW_GUTTER_MIN = 600


def fold(values):
    """`unruled::fold`, reproduced. Sort, keep a value only if it clears the last by > tolerance."""
    out = []
    for v in sorted(values):
        if not out or v - out[-1] > ALIGN_TOLERANCE:
            out.append(v)
    return out


def gaps(lines):
    return [b - a for a, b in zip(lines, lines[1:])]


def pct(v, p):
    return v[min(len(v) - 1, int(len(v) * p))] if v else None


def ground_truth():
    ref = json.loads((BENCH / "ground-truth/reference.json").read_text())
    out = {}
    for doc, v in ref.items():
        pages = collections.defaultdict(set)
        for e in v.get("elements", []):
            pages[e.get("page")].add(e.get("category"))
        out[doc] = dict(pages)
    return out


def main():
    if not ENGINE.exists():
        sys.exit(f"no engine at {ENGINE}")
    if not BENCH.exists():
        sys.exit(f"no corpus at {BENCH}; set ETHOS_BENCH_CORPUS")

    gt = ground_truth()
    # page-level records: (has_table, n_col_lines, col_faults, col_clear, col_gaps)
    rows = []
    emitted_a_table = []

    for n, (doc, pages) in enumerate(sorted(gt.items()), 1):
        pdf = BENCH / "pdfs" / doc
        if not pdf.exists():
            continue
        r = subprocess.run([str(ENGINE), "extract", str(pdf)], capture_output=True)
        if r.returncode != 0:
            continue
        rep = json.loads(r.stdout)["representation"]

        if rep.get("tables"):
            # `leftover` is not every run on these, so the reproduction would be wrong. Excluded.
            emitted_a_table.append(doc)
            continue

        table_pages = {p for p, cats in pages.items() if "Table" in cats}

        by_page = collections.defaultdict(lambda: ([], []))
        for node in rep.get("nodes", []):
            loc = (node.get("native_locator") or {}).get("pdf")
            if not loc or node.get("kind") != "text_run":
                continue
            xs, ys = by_page[loc["page"]]
            xs.append(loc["origin_x"])
            ys.append(loc["origin_y"])

        for page, (xs, ys) in by_page.items():
            cols = fold(xs)
            rws = fold(ys)
            if len(cols) < 2 or len(rws) < 2:
                continue  # no candidate at all, per detect() step 3
            cg = gaps(cols)
            rows.append({
                "has_table": page in table_pages,
                "col_lines": len(cols),
                "row_lines": len(rws),
                "col_faults": sum(1 for g in cg if g < COLUMN_GUTTER_MIN),
                "col_clear": sum(1 for g in cg if g >= COLUMN_GUTTER_MIN),
                "col_gaps": cg,
                "faces": len(cols) * len(rws),
            })
        if n % 50 == 0:
            print(f"  ...{n}/{len(gt)}", flush=True)

    print(f"\ncorpus: {BENCH}")
    print(f"pages with a candidate lattice: {len(rows)}")
    print(f"documents excluded (engine accepted a ruled table, so leftover != all runs): "
          f"{len(emitted_a_table)} {emitted_a_table}\n")

    for label, want in (("page HAS a table", True), ("page has no table", False)):
        sel = [r for r in rows if r["has_table"] == want]
        if not sel:
            print(f"{label}: n=0")
            continue
        print("=" * 78)
        print(f"{label}  (n={len(sel)} pages)")
        print("=" * 78)
        for field, name in (("col_lines", "column lines"), ("col_faults", "gaps UNDER the floor"),
                            ("col_clear", "gaps AT/OVER the floor"), ("faces", "faces implied")):
            v = sorted(r[field] for r in sel)
            print(f"  {name:<24} min={v[0]:<6} p25={pct(v,.25):<6} median={pct(v,.5):<6} "
                  f"p75={pct(v,.75):<6} max={v[-1]}")
        allg = sorted(g for r in sel for g in r["col_gaps"])
        print(f"  {'every column gap':<24} min={allg[0]:<6} p25={pct(allg,.25):<6} "
              f"median={pct(allg,.5):<6} p75={pct(allg,.75):<6} max={allg[-1]}")
        widest = sorted(max(r["col_gaps"]) for r in sel)
        print(f"  {'WIDEST gap on the page':<24} min={widest[0]:<6} p25={pct(widest,.25):<6} "
              f"median={pct(widest,.5):<6} p75={pct(widest,.75):<6} max={widest[-1]}")
        anyclear = sum(1 for r in sel if r["col_clear"] > 0)
        print(f"  pages with >=1 gap at/over the floor: {anyclear}/{len(sel)} "
              f"({100*anyclear//max(len(sel),1)}%)")
        print()


if __name__ == "__main__":
    main()
