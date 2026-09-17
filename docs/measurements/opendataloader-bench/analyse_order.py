#!/usr/bin/env python3
"""E3: decompose NID into what is MISSING and what is MIS-ORDERED, per document.

    ETHOS_BENCH=... <bench>/.venv/bin/python analyse_order.py

`evaluate_reading_order` is `fuzz.ratio` over the two whole normalized strings. That is an
indel similarity, so a document loses NID for two different reasons and the metric does not say
which: characters absent from the prediction, and characters present in the wrong order. E3 asks
for the permutation error, so the two are separated here before any document is read:

  char_recall   order-free. Multiset of lowercase alphanumerics: sum(min(gt, pred)) / sum(gt).
                1.00 means every character of the ground truth is somewhere in the prediction.
  nid_squashed  `fuzz.ratio` on the same lowercase-alphanumeric strings — the ordering cost with
                whitespace differences removed, since this projection injects spaces inside
                kerned words and the harness's NID pays for that as if text were missing.

A document with char_recall near 1.00 and a low nid_squashed lost ORDER. A document with a low
char_recall lost TEXT, and its NID is not a reading-order result at all.

The two are then separated exactly rather than by eye. `fuzz.ratio` is indel similarity, so

    nid = 2 * LCS(gt, pred) / (len(gt) + len(pred))

and the multiset intersection of the two character populations is an upper bound on that LCS.
So

    nid_upper = 2 * |gt & pred| / (len(gt) + len(pred))

is the highest NID these two character populations could reach IF every shared character were in
the same order, and the identity

    (1 - nid_sq) = (1 - nid_upper)  +  (nid_upper - nid_sq)
                 = population cost  +  ordering cost

splits every document's shortfall into the part no reordering can fix and the part that is
reading order and nothing else. `population cost` is text missing from the prediction or text the
prediction carries that the ground truth does not; `ordering cost` is permutation.

Also reads, from each artifact: the run's `reading_order_rule`, how many `region` values the
page's text runs carry (the multi-column cut), how many `block` values, and the y-monotonicity of
the emitted run order — the fraction of consecutive run pairs whose origin_y moves DOWN the page.
"""
from __future__ import annotations

import collections
import json
import os
import re
import sys
from pathlib import Path

BENCH = Path(os.environ.get("ETHOS_BENCH", Path.home() / "ethos-external-benchmarks/opendataloader-bench")).expanduser()
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(BENCH / "src"))

from rapidfuzz import fuzz  # noqa: E402


def squash(s: str) -> str:
    return re.sub(r"[^0-9a-z]+", "", s.lower())


def row(stem: str, score: dict) -> dict:
    gt = (BENCH / "ground-truth" / "markdown" / f"{stem}.md").read_text(encoding="utf-8")
    pred = (HERE / "pred" / f"{stem}.md").read_text(encoding="utf-8")
    g, p = squash(gt), squash(pred)
    cg, cp = collections.Counter(g), collections.Counter(p)
    art = json.loads((HERE / "artifacts" / f"{stem}.json").read_text())
    rep = art["representation"]
    runs = [n for n in rep["nodes"] if n["kind"] == "text_run"]
    attrs = [n.get("attributes", {}).get("text_run", {}) for n in runs]
    regions = [a.get("region") for a in attrs]
    blocks = [a.get("block") for a in attrs]
    ys = [n["native_locator"]["pdf"]["origin_y"] for n in runs if "pdf" in n.get("native_locator", {})]
    pages = [n["native_locator"]["pdf"]["page"] for n in runs if "pdf" in n.get("native_locator", {})]
    # y-monotonicity within a page: a run emitted ABOVE its predecessor is a jump back up the page.
    back_up = same = 0
    for i in range(1, len(ys)):
        if pages[i] != pages[i - 1]:
            continue
        if ys[i] < ys[i - 1]:
            back_up += 1
        elif ys[i] == ys[i - 1]:
            same += 1
    shared = sum((cg & cp).values())
    nid_sq = fuzz.ratio(g, p) / 100.0
    nid_upper = 2 * shared / max(len(g) + len(p), 1)
    return {
        "doc": stem,
        "nid": score.get("nid"),
        "nid_squashed": nid_sq,
        "nid_upper": nid_upper,
        "population_cost": 1 - nid_upper,
        "ordering_cost": nid_upper - nid_sq,
        "char_recall": shared / max(sum(cg.values()), 1),
        "char_precision": shared / max(sum(cp.values()), 1),
        "gt_chars": len(gt),
        "pred_chars": len(pred),
        "gt_sq": len(g),
        "pred_sq": len(p),
        "pages": len(rep["pages"]),
        "reading_order_rule": rep["processing_run"]["reading_order_rule"],
        "text_runs": len(runs),
        "regions": sorted({r for r in regions if r is not None}),
        "runs_with_region": sum(1 for r in regions if r is not None),
        "blocks": len({b for b in blocks if b is not None}),
        "runs_back_up_page": back_up,
        "runs_same_y": same,
        "back_up_fraction": back_up / max(len(ys) - 1, 1),
        "gt_tables": score["gt_tables"],
        "image_nodes": sum(1 for n in rep["nodes"] if n["kind"] == "image"),
    }


def main() -> None:
    scores = json.loads((HERE / "scores.json").read_text())
    cl = json.loads((HERE / "classify.json").read_text())
    rows = [row(stem, s) for stem, s in sorted(scores.items()) if s.get("nid") is not None]
    for r in rows:
        r["classify_layout"] = cl[r["doc"]]["layout_reasons"]
        r["classify_ocr"] = cl[r["doc"]]["ocr_reasons"]
    (HERE / "e3-rows.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")

    vals = sorted(r["nid"] for r in rows)
    import statistics
    print(f"NID n={len(rows)}  mean {statistics.fmean(vals):.4f}  band {vals[0]:.4f}..{vals[-1]:.4f}  "
          f"median {statistics.median(vals):.4f}")
    worst = min(rows, key=lambda r: (r["nid"], r["doc"]))
    best = max(rows, key=lambda r: (r["nid"], r["doc"]))
    print(f"  worst {worst['doc']} ({worst['nid']:.4f})   best {best['doc']} ({best['nid']:.4f})")
    print(f"  reading_order_rule values: {collections.Counter(r['reading_order_rule'] for r in rows)}")
    print(f"  documents whose runs carry >=1 `region`: {sum(1 for r in rows if r['regions'])}")
    print(f"  documents with a `multi-column` classify reason: "
          f"{sum(1 for r in rows if 'multi-column' in r['classify_layout'])}")

    print("\n=== the twenty lowest NID ===")
    print(f"{'doc':>16} {'NID':>7} {'NIDsq':>7} {'ceil':>6} {'popCost':>8} {'ordCost':>8} {'rec':>5} "
          f"{'prec':>5} {'gtCh':>6} {'prCh':>6} {'runs':>5} {'blk':>4} {'rgn':>4} {'img':>4}  classify")
    for r in sorted(rows, key=lambda r: (r["nid"], r["doc"]))[:20]:
        print(f"{r['doc']:>16} {r['nid']:7.4f} {r['nid_squashed']:7.4f} {r['nid_upper']:6.3f} "
              f"{r['population_cost']:8.3f} {r['ordering_cost']:8.3f} {r['char_recall']:5.2f} "
              f"{r['char_precision']:5.2f} {r['gt_chars']:6d} {r['pred_chars']:6d} {r['text_runs']:5d} "
              f"{r['blocks']:4d} {len(r['regions']):4d} {r['image_nodes']:4d}  "
              f"{','.join(r['classify_layout'] + r['classify_ocr'])}")

    print("\n=== corpus-wide split of the NID shortfall ===")
    loss = [r for r in rows if r["char_recall"] < 0.90]
    order = [r for r in rows if r["char_recall"] >= 0.90]
    print(f"  char_recall <  0.90 (text missing):  {len(loss):3d} documents, mean NID "
          f"{statistics.fmean([r['nid'] for r in loss]):.4f}")
    print(f"  char_recall >= 0.90 (text present):  {len(order):3d} documents, mean NID "
          f"{statistics.fmean([r['nid'] for r in order]):.4f}, mean NIDsq "
          f"{statistics.fmean([r['nid_squashed'] for r in order]):.4f}")
    print(f"  whole corpus mean NIDsq: {statistics.fmean([r['nid_squashed'] for r in rows]):.4f} "
          f"(vs NID {statistics.fmean([r['nid'] for r in rows]):.4f}) — the difference is what "
          f"whitespace placement alone costs")
    print("\n=== the corpus shortfall, split by the identity ===")
    pop = statistics.fmean([r["population_cost"] for r in rows])
    ordc = statistics.fmean([r["ordering_cost"] for r in rows])
    nsq = statistics.fmean([r["nid_squashed"] for r in rows])
    print(f"  mean population cost (1 - ceiling):    {pop:.4f}")
    print(f"  mean ordering cost   (ceiling - NIDsq): {ordc:.4f}")
    print(f"  sum {pop + ordc:.4f}  =  1 - mean NIDsq {1 - nsq:.4f}")
    ten = sorted(rows, key=lambda r: (r["nid"], r["doc"]))[:10]
    print(f"\n  the ten lowest NID: mean population cost "
          f"{statistics.fmean([r['population_cost'] for r in ten]):.4f}, mean ordering cost "
          f"{statistics.fmean([r['ordering_cost'] for r in ten]):.4f}")
    print(f"  documents where ordering cost EXCEEDS population cost: "
          f"{sum(1 for r in rows if r['ordering_cost'] > r['population_cost'])} of {len(rows)}")
    print("\n=== the fifteen largest ORDERING costs (permutation, population held constant) ===")
    for r in sorted(rows, key=lambda r: -r["ordering_cost"])[:15]:
        print(f"  {r['doc']}  ord {r['ordering_cost']:.3f}  pop {r['population_cost']:.3f}  "
              f"NID {r['nid']:.3f}  ceil {r['nid_upper']:.3f}  runs {r['text_runs']:5d}  "
              f"regions {r['regions']}  {','.join(r['classify_layout'])}")


if __name__ == "__main__":
    main()
