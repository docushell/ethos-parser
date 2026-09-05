#!/usr/bin/env python3
"""Feasibility gate for the paragraph-cut measurement.

Question: is a PDF marked-content id (mcid) PARAGRAPH-like or LINE-like on this corpus?

If one mcid spans several baselines, the document's own tagging gives a paragraph unit and can
label a gap as intra- or inter-paragraph. If one mcid is one baseline, mcid is a line id, carries
no paragraph information, and cannot label anything.

Nothing here is a claim about the cut. This only decides whether ground truth exists.
"""
import json
import sys
from collections import defaultdict

# A baseline "line" is a set of runs whose origin_y agree within this many centipoints.
# 1pt = 100 centipoints; a line's runs share a baseline exactly in a well-formed stream, and the
# tolerance absorbs sub/superscript shifts rather than merging adjacent lines (leading is >= 1000).
SAME_LINE_TOL = 150


def runs_of(doc):
    for n in doc["representation"]["nodes"]:
        if n["kind"] != "text_run":
            continue
        loc = (n.get("native_locator") or {}).get("pdf")
        if not loc:
            continue
        sl = (n.get("structural_locator") or {}).get("pdf_tagged")
        if not sl:
            continue  # artifacts and untagged content carry no paragraph claim
        yield {
            "page": loc["page"],
            "y": loc["origin_y"],
            "x": loc["origin_x"],
            "mcid": sl.get("mcid"),
            "role": (sl.get("standard_role_path") or sl.get("role_path") or [None])[-1],
            "region": ((n.get("attributes") or {}).get("text_run") or {}).get("region"),
            "text": n.get("text", ""),
        }


def lines_in(rs):
    """Distinct baselines among a set of runs, as a sorted list of y values."""
    ys = sorted(r["y"] for r in rs)
    out = []
    for y in ys:
        if not out or y - out[-1] > SAME_LINE_TOL:
            out.append(y)
    return out


def main(paths):
    print(f"{'document':<26} {'mcids':>7} {'1-line':>7} {'2+line':>7} {'median':>7} {'P-role':>7}")
    print("-" * 68)
    for p in paths:
        doc = json.load(open(p))
        by_mcid = defaultdict(list)
        prole = 0
        for r in runs_of(doc):
            if r["mcid"] is None:
                continue
            by_mcid[(r["page"], r["mcid"])].append(r)
            if r["role"] == "P":
                prole += 1
        counts = sorted(len(lines_in(v)) for v in by_mcid.values())
        if not counts:
            print(f"{p.split('/')[-1][:26]:<26} {'no tagged mcids':>7}")
            continue
        one = sum(1 for c in counts if c == 1)
        multi = sum(1 for c in counts if c > 1)
        med = counts[len(counts) // 2]
        name = p.split("/")[-1].replace(".json", "")
        print(f"{name:<26} {len(counts):>7} {one:>7} {multi:>7} {med:>7} {prole:>7}")


if __name__ == "__main__":
    main(sys.argv[1:])
