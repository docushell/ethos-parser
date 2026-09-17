#!/usr/bin/env python3
"""Does the ground truth's table TEXT reach the prediction at all?

    ETHOS_BENCH=... <bench>/.venv/bin/python cell_coverage.py

The discriminator E2 needs. A TEDS of zero says no grid was emitted; it does not say whether the
table's characters are in the document's text. This asks the second question separately: take
every cell string the harness's own converter finds in the ground truth's tables, and look for it
in the prediction with whitespace collapsed. A document whose cells are all present lost a GRID;
a document whose cells are missing lost TEXT, and the two need different fixes.

Both sides are reduced to LOWERCASE ALPHANUMERICS with every space, punctuation mark and quote
removed before the test. That is not tidiness: this engine's projection injects spaces inside
words where a producer kerned them — on `01030000000052` the header cell reads `RE GIONS` and the
region reads `Cordiller a Autonomous R egion` — and a whitespace-collapsing comparison scores
those as missing text when every character is present. Measured both ways below.

Cells with fewer than 3 alphanumeric characters are excluded from the ratio and counted, because
"1" or "%" hits any page by accident.
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path

BENCH = Path(os.environ.get("ETHOS_BENCH", Path.home() / "ethos-external-benchmarks/opendataloader-bench")).expanduser()
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(BENCH / "src"))

from bs4 import BeautifulSoup  # noqa: E402
from converter_markdown_table import convert_to_markdown_with_html_tables  # noqa: E402


def norm(s: str) -> str:
    return re.sub(r"\s+", " ", s).strip()


def squash(s: str) -> str:
    """Lowercase alphanumerics only — immune to the injected intra-word spaces."""
    return re.sub(r"[^0-9a-z]+", "", s.lower())


def gt_cells(md: str) -> list[str]:
    soup = BeautifulSoup(convert_to_markdown_with_html_tables(md), "html.parser")
    out = []
    for table in soup.find_all("table"):
        for cell in table.find_all(["td", "th"]):
            out.append(norm(cell.get_text()))
    return out


def main() -> None:
    scores = json.loads((HERE / "scores.json").read_text())
    rows = {}
    for stem, s in sorted(scores.items()):
        if not s["gt_tables"]:
            continue
        gt = (BENCH / "ground-truth" / "markdown" / f"{stem}.md").read_text(encoding="utf-8")
        pred_raw = (HERE / "pred" / f"{stem}.md").read_text(encoding="utf-8")
        pred_ws, pred_sq = norm(pred_raw), squash(pred_raw)
        cells = gt_cells(gt)
        scored = [c for c in cells if len(squash(c)) >= 3]
        hit = [c for c in scored if squash(c) in pred_sq]
        hit_ws = [c for c in scored if c in pred_ws]
        rows[stem] = {
            "teds": s["teds"],
            "gt_tables": s["gt_tables"],
            "gt_cells": len(cells),
            "gt_cells_scored": len(scored),
            "gt_cells_short_or_empty": len(cells) - len(scored),
            "gt_cells_found_in_prediction": len(hit),
            "gt_cells_found_whitespace_exact": len(hit_ws),
            "coverage": len(hit) / len(scored) if scored else None,
            "missing_sample": [c for c in scored if squash(c) not in pred_sq][:6],
        }
    (HERE / "cell-coverage.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")

    print(f"{'doc':>16} {'TEDS':>7} {'cells':>6} {'scored':>7} {'found':>6} {'wsEx':>5} {'cover':>6}  missing sample")
    for stem, r in sorted(rows.items(), key=lambda kv: (kv[1]["coverage"] if kv[1]["coverage"] is not None else 1, kv[0])):
        cov = "n/a" if r["coverage"] is None else f"{r['coverage']:.2f}"
        print(f"{stem:>16} {r['teds']:7.4f} {r['gt_cells']:6d} {r['gt_cells_scored']:7d} "
              f"{r['gt_cells_found_in_prediction']:6d} {r['gt_cells_found_whitespace_exact']:5d} {cov:>6}  "
              f"{str(r['missing_sample'])[:110]}")

    zeros = {k: v for k, v in rows.items() if v["teds"] == 0}
    full = [k for k, v in zeros.items() if v["coverage"] is not None and v["coverage"] >= 0.9]
    part = [k for k, v in zeros.items() if v["coverage"] is not None and 0.5 <= v["coverage"] < 0.9]
    poor = [k for k, v in zeros.items() if v["coverage"] is not None and v["coverage"] < 0.5]
    print(f"\nTEDS-zero documents: {len(zeros)}")
    print(f"  cell text >= 90% present in prediction (a GRID was lost, not text): {len(full)}  {sorted(full)}")
    print(f"  cell text 50-90% present:                                          {len(part)}  {sorted(part)}")
    print(f"  cell text < 50% present (TEXT was lost):                           {len(poor)}  {sorted(poor)}")


if __name__ == "__main__":
    main()
