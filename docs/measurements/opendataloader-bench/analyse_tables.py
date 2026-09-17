#!/usr/bin/env python3
"""E2: one measured row per document whose ground truth holds a table.

    python3 analyse_tables.py            # after score_e23.py and save_artifacts.py

Reads scores.json (the harness evaluators' own numbers) and artifacts/*.json (the engine's own
words). Computes nothing the engine or the evaluator does not say, except the arithmetic named in
the output: characters, ratios, counts.
"""
from __future__ import annotations

import collections
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
GT = Path.home() / "ethos-external-benchmarks/opendataloader-bench/ground-truth/markdown"

# The refusal kinds, as the source spells them. unruled.rs::Refusal has no kind() so its four
# arms are matched on the distinguishing phrase of Refusal::detail().
UNRULED_KINDS = [
    ("past the cell ceiling", "-cell ceiling for an inferred grid"),
    ("faces without text", "but only filled"),
    ("gutter below floor", "centipoints apart, under the"),
    ("emission order not row-major", "DOWN the columns rather than ACROSS the rows"),
]
RULED_KINDS = [
    ("a grid line the ink does not trace", "a grid line the ink does not trace"),
    ("past the cell ceiling", "past the cell ceiling"),
    ("a grid that contradicts itself", "a grid that contradicts itself"),
]
STROKE_KINDS = [
    ("a column line the page never drew", "a column line the page never drew"),
    ("a cell that is a form field's own box", "a cell that is a form field's own box"),
    ("past the cell ceiling", "past the cell ceiling"),
]


def kinds(detail: str, table: list[tuple[str, str]]) -> list[str]:
    return [name for name, needle in table if needle in detail]


def numbers(detail: str) -> list[str]:
    """The per-page parenthetical or bullet, which carries the counts."""
    out = []
    for line in detail.split("\n"):
        s = line.strip()
        if s.startswith("Refused on ") or s.startswith("- page "):
            out.append(re.sub(r"\s+", " ", s)[:220])
    return out


def row(stem: str, score: dict) -> dict:
    art = json.loads((HERE / "artifacts" / f"{stem}.json").read_text())
    rep = art["representation"]
    nodes = rep.get("nodes", [])
    lim = {l["code"]: (l.get("detail") or "") for l in rep["assurance"]["limitations"]}
    tables = rep.get("tables") or []
    r = {
        "doc": stem,
        "teds": score.get("teds"),
        "nid": score.get("nid"),
        "gt_tables": score["gt_tables"],
        "pred_tables": score["pred_tables"],
        "gt_chars": score["gt_chars"],
        "pred_chars": score["pred_chars"],
        "pages": len(rep["pages"]),
        "text_runs": sum(1 for n in nodes if n["kind"] == "text_run"),
        "image_nodes": sum(1 for n in nodes if n["kind"] == "image"),
        "artifact_text_chars": sum(len(n.get("text") or "") for n in nodes if n["kind"] == "text_run"),
        "emitted": [
            {
                "rule": t["detection_rule"],
                "rows": t["rows"],
                "columns": t["columns"],
                "cells": len(t.get("cells") or []),
                "derivation": t.get("derivation"),
                "check": (t.get("locator_check") or {}).get("outcome", {}).get("status"),
            }
            for t in tables
        ],
        "table_codes": sorted(c for c in lim if "table" in c),
        "refusals": {},
        "image_codes": sorted(c for c in lim if "image" in c or "xobject" in c),
    }
    for code, table in (
        ("unruled-table-candidate-refused", UNRULED_KINDS),
        ("ruled-table-candidate-refused", RULED_KINDS),
        ("stroke-ruled-table-candidate-refused", STROKE_KINDS),
    ):
        if code in lim:
            r["refusals"][code] = {"kinds": kinds(lim[code], table), "pages": numbers(lim[code])}
    return r


def main() -> None:
    scores = json.loads((HERE / "scores.json").read_text())
    rows = [row(stem, s) for stem, s in sorted(scores.items()) if s["gt_tables"]]
    (HERE / "e2-rows.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")

    print(f"documents whose ground truth holds a table: {len(rows)}")
    print(f"  emitting >= 1 table: {sum(1 for r in rows if r['emitted'])}")
    print(f"  TEDS zero:           {sum(1 for r in rows if r['teds'] == 0)}")
    print(f"  TEDS non-zero:       {sum(1 for r in rows if r['teds'])}")

    # Fabrication: a table in the prediction the ground truth has none of.
    fab = [(stem, s) for stem, s in scores.items() if s["pred_tables"] and not s["gt_tables"]]
    print(f"\nfabrication (prediction holds a table, ground truth holds none): {len(fab)} documents")
    for stem, s in fab:
        print(f"  {stem}: pred_tables={s['pred_tables']}")
    # and the same question asked of the artifact rather than the Markdown
    emit_no_gt = []
    for stem, s in scores.items():
        if s["gt_tables"]:
            continue
        art = json.loads((HERE / "artifacts" / f"{stem}.json").read_text())
        t = art["representation"].get("tables") or []
        if t:
            emit_no_gt.append((stem, sum(len(x.get("cells") or []) for x in t), len(t)))
    print(f"artifact emits a table where ground truth holds none: {len(emit_no_gt)} documents"
          f"  {emit_no_gt}")
    print(f"total cells emitted on those documents: {sum(c for _, c, _ in emit_no_gt)}")

    print("\n=== every document whose ground truth holds a table ===")
    print(f"{'doc':>16} {'TEDS':>7} {'NID':>6} {'gtT':>4} {'pT':>3} {'pg':>3} {'runs':>5} "
          f"{'img':>4} {'gtCh':>6} {'prCh':>6} emitted / refusal kinds")
    for r in sorted(rows, key=lambda r: (-(r["teds"] or 0), r["doc"])):
        em = ";".join(f"{e['rule']} {e['rows']}x{e['columns']}({e['cells']}c,{e['check']})" for e in r["emitted"]) or "-"
        ref = ";".join(f"{c.split('-table')[0]}:{'/'.join(v['kinds'])}" for c, v in r["refusals"].items())
        print(f"{r['doc']:>16} {r['teds']:7.4f} {r['nid']:6.3f} {r['gt_tables']:4d} {r['pred_tables']:3d} "
              f"{r['pages']:3d} {r['text_runs']:5d} {r['image_nodes']:4d} {r['gt_chars']:6d} {r['pred_chars']:6d} "
              f"{em} | {ref}")

    print("\n=== refusal kind frequency among the TEDS-zero documents ===")
    c: collections.Counter = collections.Counter()
    for r in rows:
        if r["teds"] != 0:
            continue
        for code, v in r["refusals"].items():
            for k in v["kinds"]:
                c[f"{code} :: {k}"] += 1
    for k, n in c.most_common():
        print(f"  {n:3d}  {k}")

    print("\n=== per-page refusal numbers, TEDS-zero documents ===")
    for r in sorted(rows, key=lambda r: r["doc"]):
        if r["teds"] != 0:
            continue
        print(f"  {r['doc']}  runs={r['text_runs']} img={r['image_nodes']} "
              f"gtCh={r['gt_chars']} prCh={r['pred_chars']} ratio={r['pred_chars']/max(r['gt_chars'],1):.2f}")
        for code, v in r["refusals"].items():
            for p in v["pages"]:
                print(f"      {code}: {p}")


if __name__ == "__main__":
    main()
