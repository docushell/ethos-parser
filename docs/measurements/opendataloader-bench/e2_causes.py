#!/usr/bin/env python3
"""E2: assign exactly one cause to each TEDS-zero document, from measured facts only.

    python3 e2_causes.py

The rules are applied in this order, first match wins, so the sum is the count of zeros by
construction. Each test names the measurement it reads.

  1 TABLE IS A PICTURE       cell_coverage < 0.50 — under half the ground truth's table cell text
                             is anywhere in the prediction — AND the artifact carries an image
                             node. The grid is not the problem: the characters are not in the
                             file's text layer.
  2 DRAWN GRID REFUSED       the artifact declares `ruled-table-candidate-refused` or
                             `stroke-ruled-table-candidate-refused`: the page painted rectangles
                             that implied a lattice and a rule judged it incoherent.
  3 INFERRED GRID, CEILING   only `unruled-table-candidate-refused` fires, and its reason is the
                             4096-cell ceiling.
  4 INFERRED GRID, GUTTER    only `unruled-table-candidate-refused` fires, and its reason is the
                             1200-centipoint gutter floor.
  5 NO REFUSAL AT ALL        no table-candidate refusal is declared. Reported separately because
                             it would mean the engine was silent, which is the case the refusal
                             codes exist to prevent.
"""
from __future__ import annotations
import collections
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
rows = {r["doc"]: r for r in json.loads((HERE / "e2-rows.json").read_text())}
cov = json.loads((HERE / "cell-coverage.json").read_text())
cl = json.loads((HERE / "classify.json").read_text())

CAUSE = {}
for doc, r in rows.items():
    if r["teds"] != 0:
        continue
    ref = r["refusals"]
    unruled = ref.get("unruled-table-candidate-refused", {}).get("kinds", [])
    if cov[doc]["coverage"] is not None and cov[doc]["coverage"] < 0.50 and r["image_nodes"]:
        CAUSE[doc] = "1 the truth's table is a picture"
    elif "ruled-table-candidate-refused" in ref or "stroke-ruled-table-candidate-refused" in ref:
        CAUSE[doc] = "2 a drawn grid was built and refused"
    elif "past the cell ceiling" in unruled:
        CAUSE[doc] = "3 an inferred grid past the 4096-cell ceiling"
    elif "gutter below floor" in unruled:
        CAUSE[doc] = "4 an inferred grid under the gutter floor"
    elif unruled:
        CAUSE[doc] = "3b an inferred grid refused for another reason: " + ";".join(unruled)
    else:
        CAUSE[doc] = "5 no refusal declared at all"

counts = collections.Counter(CAUSE.values())
print(f"TEDS-zero documents classified: {sum(counts.values())}")
for cause, n in sorted(counts.items()):
    docs = sorted(d for d, c in CAUSE.items() if c == cause)
    print(f"\n  {n:2d}  {cause}")
    for d in docs:
        r = rows[d]
        c = cl[d]
        ref = r["refusals"]
        why = "; ".join(f"{k.replace('-table-candidate-refused', '')}={'/'.join(v['kinds'])}"
                        for k, v in ref.items()) or "none"
        print(f"        {d}  gtTables={r['gt_tables']}  predTables={r['pred_tables']}  "
              f"emitted={len(r['emitted'])}  cellCover={cov[d]['coverage']:.2f}  "
              f"rects={c['rectangles']}  imgNodes={r['image_nodes']}  [{why}]")

print(f"\nsum {sum(counts.values())} == {sum(1 for r in rows.values() if r['teds'] == 0)} zeros")
print(f"documents emitting a table whose structure overlaps the truth's not at all: "
      f"{sum(1 for d, r in rows.items() if r['teds'] == 0 and r['emitted'])} "
      f"(a zero can only be scored this way if a table WAS emitted)")
(HERE / "e2-causes.json").write_text(json.dumps(CAUSE, indent=1), encoding="utf-8")
