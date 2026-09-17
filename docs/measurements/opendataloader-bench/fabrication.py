#!/usr/bin/env python3
"""Is every emitted table cell's text made of runs the page actually drew?

    python3 fabrication.py

`table-gate-v1.md` states the claim as "every cell's text is a concatenation of runs the page
actually drew", and calls fabrication "the one number with a required value". Two tests, over all
200 artifacts:

  1. a table emitted on a document whose ground truth holds none, and how many cells that is;
  2. a cell whose `text` is NOT the concatenation of the `text` of the nodes its `node_ids` name —
     which is the engine's own definition of a fabricated cell, checked here rather than trusted.
"""
from __future__ import annotations
import glob, json, re
from pathlib import Path

HERE = Path(__file__).resolve().parent
scores = json.loads((HERE / "scores.json").read_text())

tables = cells = docs = bad_text = empty_cells = 0
fab_docs, bad = [], []
for f in sorted(glob.glob(str(HERE / "artifacts" / "*.json"))):
    stem = Path(f).stem
    rep = json.loads(Path(f).read_text())["representation"]
    text = {n["id"]: (n.get("text") or "") for n in rep["nodes"]}
    ts = rep.get("tables") or []
    if not ts:
        continue
    docs += 1
    tables += len(ts)
    n_cells = sum(len(t.get("cells") or []) for t in ts)
    cells += n_cells
    if not scores[stem]["gt_tables"]:
        fab_docs.append((stem, len(ts), n_cells))
    for t in ts:
        for c in t.get("cells") or []:
            joined = "".join(text.get(i, "\x00MISSING") for i in c.get("node_ids") or [])
            if not (c.get("node_ids") or []):
                empty_cells += 1
                if (c.get("text") or "").strip():
                    bad.append((stem, c["id"], "text with no node_ids", c.get("text", "")[:60]))
                    bad_text += 1
                continue
            if re.sub(r"\s+", "", joined) != re.sub(r"\s+", "", c.get("text") or ""):
                bad.append((stem, c["id"], "text is not its runs", (c.get("text") or "")[:60] + " != " + joined[:60]))
                bad_text += 1

print(f"documents emitting a table        {docs}")
print(f"tables emitted                    {tables}")
print(f"cells emitted                     {cells}  ({empty_cells} with no node_ids, i.e. an empty slot)")
print(f"\n1. tables emitted where ground truth holds none: {len(fab_docs)} documents"
      f"  {fab_docs}")
print(f"   cells in them: {sum(c for _, _, c in fab_docs)}")
print(f"\n2. cells whose text is not the concatenation of the runs they name: {bad_text}")
for row in bad[:20]:
    print(f"   {row}")
