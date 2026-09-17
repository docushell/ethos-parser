#!/usr/bin/env python3
"""E2/E3 measurement: score.py adapted to keep every per-document number on disk.

    ETHOS_BENCH=~/ethos-external-benchmarks/opendataloader-bench \
    ETHOS_PARSER_BIN=<frozen binary> \
      <bench>/.venv/bin/python score_e23.py

Copied from docs/measurements/opendataloader-bench/score.py and changed in exactly three ways,
so the numbers stay the committed scorer's numbers:

  1. predictions are written under this directory instead of /tmp, and kept;
  2. every per-document (NID, TEDS) pair is written to scores.json along with the number of
     `<table>` elements the *evaluator itself* sees in ground truth and in the prediction — the
     harness's own `convert_to_markdown_with_html_tables` + `extract_tables`, not a re-count;
  3. MHS is not computed (E2/E3 do not ask for it), so nothing here can be quoted as an MHS run.

`to_markdown` is imported from the committed adapter unmodified: the predictions scored here are
the adapter's, not this script's.
"""
import json
import os
import statistics
import sys
import time
from pathlib import Path

BENCH = Path(os.environ.get("ETHOS_BENCH", Path.home() / "ethos-external-benchmarks/opendataloader-bench")).expanduser()
REPO = Path("/Users/saumildiwaker/Desktop/Stuff/project/repo/ethos-parser")
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(BENCH / "src"))
sys.path.insert(0, str(REPO / "docs/measurements/opendataloader-bench"))

from converter_markdown_table import convert_to_markdown_with_html_tables  # noqa: E402
from evaluator_reading_order import evaluate_reading_order  # noqa: E402
from evaluator_table import evaluate_table, extract_tables  # noqa: E402
from pdf_parser_ethos_parser import to_markdown  # noqa: E402


def main() -> None:
    docs = sorted((BENCH / "pdfs").glob("*.pdf"))
    if not docs:
        sys.exit(f"no PDFs under {BENCH / 'pdfs'}")
    out = HERE / "pred"
    started = time.time()
    to_markdown(docs, BENCH / "pdfs", out)
    elapsed = time.time() - started

    rows, nid, teds, empty = {}, [], [], 0
    for gt_path in sorted((BENCH / "ground-truth" / "markdown").glob("*.md")):
        stem = gt_path.stem
        pred_path = out / gt_path.name
        pred = pred_path.read_text(encoding="utf-8") if pred_path.is_file() else ""
        empty += not pred.strip()
        truth = gt_path.read_text(encoding="utf-8")
        row = {
            "empty_prediction": not pred.strip(),
            "gt_tables": len(extract_tables(convert_to_markdown_with_html_tables(truth))),
            "pred_tables": len(extract_tables(convert_to_markdown_with_html_tables(pred))),
            "gt_chars": len(truth),
            "pred_chars": len(pred),
        }
        for fn, acc, key in ((evaluate_reading_order, nid, "nid"), (evaluate_table, teds, "teds")):
            try:
                v = fn(truth, pred)
                v = v[0] if isinstance(v, tuple) else v
                if v is not None:
                    acc.append((stem, v))
                    row[key] = v
            except Exception as exc:  # recorded, never swallowed silently
                row[key + "_error"] = f"{type(exc).__name__}: {exc}"[:200]
        rows[stem] = row

    (HERE / "scores.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")

    mean = lambda xs: statistics.fmean([v for _, v in xs]) if xs else 0.0  # noqa: E731
    print(f"\n  binary         {os.environ.get('ETHOS_PARSER_BIN', '(unset)')}")
    print(f"  documents      {len(docs)}  ({empty} empty predictions)")
    print(f"  NID            {mean(nid):.4f}   n={len(nid)}   reading order")
    print(f"  TEDS           {mean(teds):.4f}   n={len(teds)}   table structure")
    print(f"  speed          {elapsed / len(docs) * 1000:.0f} ms/document (extract+markdown, loaded machine)")
    band(nid, "NID")
    band(teds, "TEDS")
    zeros = [d for d, v in teds if v == 0]
    print(f"\n  TEDS: {len(teds) - len(zeros)} of {len(teds)} non-zero, {len(zeros)} at zero")
    print(f"  fabrication candidates (pred has a table, ground truth has none): "
          f"{sorted(d for d, r in rows.items() if r['pred_tables'] and not r['gt_tables'])}")
    print("\n  NID ten lowest:")
    for d, v in sorted(nid, key=lambda dv: (dv[1], dv[0]))[:10]:
        print(f"    {v:.4f}  {d}")


def band(scored: list[tuple[str, float]], label: str) -> None:
    if not scored:
        return
    values = sorted(v for _, v in scored)
    worst = min(scored, key=lambda dv: (dv[1], dv[0]))
    best = max(scored, key=lambda dv: (dv[1], dv[0]))
    print(f"  {label:6s} band    {values[0]:.4f}..{values[-1]:.4f}   median {statistics.median(values):.4f}"
          f"   worst {worst[0]} ({worst[1]:.4f})   best {best[0]} ({best[1]:.4f})")


if __name__ == "__main__":
    main()
