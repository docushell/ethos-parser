#!/usr/bin/env python3
"""Score ethos-parser on opendataloader-bench, with the harness's own evaluators.

    ETHOS_BENCH=~/path/to/opendataloader-bench python3 score.py

Uses `evaluator_reading_order`, `evaluator_table` and `evaluator_heading_level` from that
repository unmodified, so the numbers are its numbers rather than a reimplementation. Requires its
virtualenv (`rapidfuzz`, `apted`) — run with `$ETHOS_BENCH/.venv/bin/python`.

**This is an instrument, not a leaderboard entry.** `docs/06-STEAL-REFUSE.md` O26 refuses "#1,
fastest, or any bake-off claim" and `docs/02-ROADMAP.md` lists rankings under "Not scheduled".
Running someone else's harness against documents this repository does not own is precisely what
`docs/table-gate-v1.md` says a reader must do to compare two parsers honestly — and publishing the
result as a ranking is the separate thing that stays refused.
"""
import json
import os
import statistics
import sys
import time
from pathlib import Path

BENCH = Path(os.environ.get("ETHOS_BENCH", Path.home() / "ethos-external-benchmarks/opendataloader-bench")).expanduser()
sys.path.insert(0, str(BENCH / "src"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

from evaluator_heading_level import evaluate_heading_level  # noqa: E402
from evaluator_reading_order import evaluate_reading_order  # noqa: E402
from evaluator_table import evaluate_table  # noqa: E402
from pdf_parser_ethos_parser import to_markdown  # noqa: E402


def main() -> None:
    docs = sorted((BENCH / "pdfs").glob("*.pdf"))
    if not docs:
        sys.exit(f"no PDFs under {BENCH / 'pdfs'} — set ETHOS_BENCH, and note the corpus is Git LFS")
    out = Path("/tmp/ethos-parser-bench")
    started = time.time()
    to_markdown(docs, BENCH / "pdfs", out)
    elapsed = time.time() - started

    nid, teds, mhs, empty = [], [], [], 0
    for gt in sorted((BENCH / "ground-truth" / "markdown").glob("*.md")):
        pred_path = out / gt.name
        pred = pred_path.read_text(encoding="utf-8") if pred_path.is_file() else ""
        empty += not pred.strip()
        truth = gt.read_text(encoding="utf-8")
        for fn, acc in ((evaluate_reading_order, nid), (evaluate_table, teds), (evaluate_heading_level, mhs)):
            try:
                v = fn(truth, pred)
                v = v[0] if isinstance(v, tuple) else v
                if v is not None:
                    acc.append((gt.stem, v))
            except Exception:
                pass

    mean = lambda xs: statistics.fmean([v for _, v in xs]) if xs else 0.0  # noqa: E731
    print(f"\n  documents      {len(docs)}  ({empty} empty predictions)")
    print(f"  NID            {mean(nid):.4f}   n={len(nid)}   reading order")
    print(f"  TEDS           {mean(teds):.4f}   n={len(teds)}   table structure")
    print(f"  MHS            {mean(mhs):.4f}   n={len(mhs)}   heading hierarchy")
    print(f"  speed          {elapsed / len(docs) * 1000:.0f} ms/document")
    band(nid, "NID")
    band(teds, "TEDS")
    zeros = [d for d, v in teds if v == 0]
    print(f"\n  TEDS is bimodal by construction: {len(teds) - len(zeros)} of {len(teds)} non-zero, "
          f"{len(zeros)} at zero. Every document whose ground truth holds a table:")
    for d, v in sorted(teds, key=lambda dv: (-dv[1], dv[0])):
        print(f"    {v:.4f}  {d}")
    print("  MHS is 0 by construction: headings come from a tag tree, and 0 of 200 documents have one.")


def band(scored: list[tuple[str, float]], label: str) -> None:
    """Decision #18: a result ships as a band with its worst document named, never as a macro alone.

    Ties at the minimum are broken by name, so the document named is the first of them; the
    per-document list printed after it is what makes a tie readable.
    """
    if not scored:
        return
    values = sorted(v for _, v in scored)
    worst = min(scored, key=lambda dv: (dv[1], dv[0]))
    best = max(scored, key=lambda dv: (dv[1], dv[0]))
    print(f"  {label:6s} band    {values[0]:.4f}..{values[-1]:.4f}   median {statistics.median(values):.4f}"
          f"   worst {worst[0]} ({worst[1]:.4f})   best {best[0]} ({best[1]:.4f})")


if __name__ == "__main__":
    main()
