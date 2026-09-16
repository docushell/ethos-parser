#!/usr/bin/env python3
"""Limitation census of ethos-parser over opendataloader-bench's 200 PDFs.

    ETHOS_PARSER_BIN=target/release/ethos-parser python3 docs/measurements/opendataloader-bench/census.py <bench>/pdfs

Prints the two tables the README beside this file quotes under "the corpus is also a limitation
census": which limitation codes fire on how many documents, and how many text nodes carry no
measured ink box, by the absence the engine typed on each. Committed so those numbers can be
re-derived rather than believed; until it existed they were assembled by hand.

**It computes nothing the engine does not say.** Every count is read from the `extract` artifact —
`representation.assurance.limitations` for the codes, the `geometry` sidecar for the boxes — and
no document is scored. Standard library only, so it runs without the harness's virtualenv.

**Codes that fire on every artifact are named, not counted.** Most are the profile's own standing
declarations and say nothing per document; on this corpus `untagged-structure-tree-absent` is among
them only because no document carries a structure tree, which is why the README's table still
lists it. A second argument writes the per-document rows as JSON.
"""
from __future__ import annotations

import collections
import glob
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

BIN = os.environ.get("ETHOS_PARSER_BIN", "target/release/ethos-parser")


def one(path: str) -> dict:
    row: dict = {"file": os.path.basename(path)}
    p = subprocess.run([BIN, "extract", path], capture_output=True)
    row["extract_exit"] = p.returncode
    if p.returncode != 0:
        row["extract_err"] = p.stderr.decode(errors="replace").strip()[:400]
        return row
    artifact = json.loads(p.stdout)
    rep = artifact["representation"]
    row["limitations"] = sorted({l["code"] for l in rep.get("assurance", {}).get("limitations", [])})
    row["tables"] = len(rep.get("tables") or [])
    nodes = {n["id"]: n for n in rep.get("nodes", [])}
    row["text_nodes"] = sum(1 for n in nodes.values() if n.get("kind") == "text_run")
    # A text node with no measured ink box is omitted from `ethos.grounding.v1`; the sidecar types
    # why, and the reason is the whole point of counting. Whether the node's text is whitespace
    # only is counted beside it, so "whitespace runs" is measured rather than inferred from the
    # absence's name.
    absent: collections.Counter = collections.Counter()
    for g in artifact.get("geometry") or []:
        node = nodes.get(g.get("node"))
        if not node or node.get("kind") != "text_run":
            continue
        presence = g.get("presence") or {}
        if presence.get("state") != "measured":
            absent[str(presence.get("value"))] += 1
            if not (node.get("text") or "").strip():
                absent[str(presence.get("value")) + " (whitespace only)"] += 1
    row["text_nodes_absent"] = dict(absent)
    return row


def report(rows: list[dict]) -> None:
    n = len(rows)
    art = [r for r in rows if r["extract_exit"] == 0]
    print(f"\n=== reach ({n} documents) ===")
    print(f"  artifact produced        {len(art):4d}")
    print(f"  emitting >= 1 table      {sum(1 for r in art if r['tables']):4d}   ({sum(r['tables'] for r in art)} tables)")
    for r in rows:
        if r["extract_exit"] != 0:
            print(f"  NO ARTIFACT {r['file']}: exit {r['extract_exit']}: {r.get('extract_err', '')[:120]}")

    counts = collections.Counter(c for r in art for c in r["limitations"])
    constant = sorted(c for c, k in counts.items() if k == len(art))
    print(f"\n=== limitation codes on every one of the {len(art)} artifacts (named, not counted) ===")
    for c in constant:
        print(f"  {c}")
    print(f"\n=== document-scoped limitation codes (code -> documents) ===")
    for c, k in sorted(counts.items(), key=lambda ck: (-ck[1], ck[0])):
        if k < len(art):
            print(f"  {c:44s} {k:4d}  {k / len(art) * 100:3.0f}%")

    total = sum(r["text_nodes"] for r in art)
    absent: collections.Counter = collections.Counter()
    for r in art:
        absent.update(r["text_nodes_absent"])
    absent_total = sum(k for reason, k in absent.items() if " (" not in reason)
    print(f"\n=== text nodes with no measured ink box ({total:,} text nodes) ===")
    print(f"  {'total absent':40s} {absent_total:8,d}  {absent_total / total * 100:.1f}%")
    for reason, k in sorted(absent.items(), key=lambda rk: (rk[0].split(" (")[0], " (" in rk[0])):
        print(f"  {'  ' if ' (' in reason else ''}{reason:40s} {k:8,d}")


if __name__ == "__main__":
    files = sorted(glob.glob(os.path.join(sys.argv[1] if len(sys.argv) > 1 else "pdfs", "*.pdf")))
    if not files:
        sys.exit("no PDFs found — pass <bench>/pdfs, and note the corpus is Git LFS")
    started = time.time()
    with ThreadPoolExecutor(4) as pool:
        rows = list(pool.map(one, files))
    print(f"{len(rows)} documents in {time.time() - started:.0f}s wall, {BIN}")
    if len(sys.argv) > 2:
        with open(sys.argv[2], "w", encoding="utf-8") as fh:
            json.dump(rows, fh, indent=1)
    report(rows)
