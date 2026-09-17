#!/usr/bin/env python3
"""E3: the shape of the permutation, read off the emitted run order.

    python3 sweeps.py [doc ...]

`reading_order.rs` states the rule in five steps and step 5 is the one that decides most of this
corpus: *"No gutter, no reordering. A page whose first vertical cut fails comes out in
content-stream order, unchanged, byte for byte."* So on a page the rule did not cut, the emitted
order IS the producer's content-stream order, and the permutation E3 is asking about is the
producer's, left in place.

That is visible without guessing. Walk the emitted runs in order and cut a new SWEEP wherever the
next run's `origin_y` jumps back UP the page by more than one line. A page in visual reading order
is one sweep. A page emitted as n unrelated text boxes is n sweeps, and each sweep's x/y extent
says what the box was: a column, a slide panel, a header, a table row band.

The line height is the document's own — the median of the positive y steps between consecutive
runs, floored at 400 centipoints (4 pt) so a document of one text line cannot produce a zero
threshold. Stated because it is a threshold and thresholds must be stated.
"""
from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def sweeps(stem: str) -> dict:
    rep = json.loads((HERE / "artifacts" / f"{stem}.json").read_text())["representation"]
    runs = [n for n in rep["nodes"] if n["kind"] == "text_run" and "pdf" in n.get("native_locator", {})]
    pts = [(n["native_locator"]["pdf"]["page"], n["native_locator"]["pdf"]["origin_y"],
            n["native_locator"]["pdf"]["origin_x"], n["native_locator"]["pdf"].get("advance", 0),
            n.get("text") or "") for n in runs]
    steps = [pts[i][1] - pts[i - 1][1] for i in range(1, len(pts)) if pts[i][0] == pts[i - 1][0]]
    down = [s for s in steps if s > 0]
    line = max(int(statistics.median(down)) if down else 0, 400)

    cuts, cur = [], [pts[0]] if pts else []
    for i in range(1, len(pts)):
        new_page = pts[i][0] != pts[i - 1][0]
        jump_up = pts[i][1] < pts[i - 1][1] - line
        if new_page or jump_up:
            cuts.append(cur)
            cur = []
        cur.append(pts[i])
    if cur:
        cuts.append(cur)

    out = []
    for s in cuts:
        ys = [p[1] for p in s]
        xs = [p[2] for p in s]
        txt = "".join(p[4] for p in s)
        out.append({
            "runs": len(s),
            "page": s[0][0],
            "y": [min(ys), max(ys)],
            "x": [min(xs), max(xs + [p[2] + p[3] for p in s])],
            "chars": len(txt),
            "head": " ".join(txt.split())[:88],
        })
    return {
        "doc": stem,
        "pages": len(rep["pages"]),
        "page_size": [rep["pages"][0]["width"], rep["pages"][0]["height"]],
        "rotation": rep["pages"][0]["rotation"],
        "line_height_centipoints": line,
        "runs": len(pts),
        "sweeps": len(cuts),
        "sweeps_per_page": len(cuts) / len(rep["pages"]),
        "detail": out,
    }


def main() -> None:
    scores = json.loads((HERE / "scores.json").read_text())
    if len(sys.argv) > 1:
        for stem in sys.argv[1:]:
            s = sweeps(stem)
            print(f"\n===== {stem}  NID {scores[stem]['nid']:.4f}  page "
                  f"{s['page_size'][0] / 100:.0f}x{s['page_size'][1] / 100:.0f}pt rot {s['rotation']}  "
                  f"{s['runs']} runs in {s['sweeps']} sweep(s), line={s['line_height_centipoints']}cp")
            for i, d in enumerate(s["detail"], 1):
                print(f"  sweep {i:2d} p{d['page']} y {d['y'][0] / 100:6.1f}..{d['y'][1] / 100:6.1f} "
                      f"x {d['x'][0] / 100:6.1f}..{d['x'][1] / 100:6.1f} {d['runs']:5d} runs {d['chars']:5d} ch  "
                      f"{d['head']}")
        return

    rows = [sweeps(stem) for stem in sorted(scores)]
    (HERE / "sweeps.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")
    by = {r["doc"]: r for r in rows}
    print(f"{'doc':>16} {'NID':>7} {'pages':>5} {'runs':>6} {'sweeps':>7} {'per page':>9}")
    for stem, s in sorted(scores.items(), key=lambda kv: (kv[1]["nid"], kv[0]))[:25]:
        r = by[stem]
        print(f"{stem:>16} {s['nid']:7.4f} {r['pages']:5d} {r['runs']:6d} {r['sweeps']:7d} "
              f"{r['sweeps_per_page']:9.1f}")
    import collections
    print("\nsweeps per page, whole corpus:",
          collections.Counter("1" if r["sweeps_per_page"] <= 1.0 else
                              "2-3" if r["sweeps_per_page"] <= 3 else
                              "4-9" if r["sweeps_per_page"] <= 9 else "10+" for r in rows))
    single = [r for r in rows if r["sweeps_per_page"] <= 1.0]
    many = [r for r in rows if r["sweeps_per_page"] > 3]
    print(f"  mean NID, <=1 sweep/page  ({len(single):3d} docs): "
          f"{statistics.fmean([scores[r['doc']]['nid'] for r in single]):.4f}")
    print(f"  mean NID, >3 sweeps/page  ({len(many):3d} docs): "
          f"{statistics.fmean([scores[r['doc']]['nid'] for r in many]):.4f}")


if __name__ == "__main__":
    main()
