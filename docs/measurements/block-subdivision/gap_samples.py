#!/usr/bin/env python3
"""Measurement 2: what IS a gap of each size? Read the text either side and say.

A histogram cannot tell a paragraph break from a figure caption, a formula, or a table row. This
prints real line pairs at each ratio so the shape in `gap_dist.py` can be interpreted rather than
assumed, which is the difference between a measurement and a Rorschach test.
"""
import json
import sys
from collections import defaultdict

sys.path.insert(0, "/private/tmp/claude-501/-Users-saumildiwaker-Desktop-Stuff-project-repo-ethos-parser/b06430fd-969a-4694-b6d8-a5e5a3044056/scratchpad")
from gap_dist import body_lines, band_gaps, leading_of  # noqa: E402

BANDS = [(0.0, 0.9), (0.9, 1.1), (1.1, 1.3), (1.3, 1.5), (1.5, 1.8), (1.8, 2.2), (2.2, 3.0)]


def main(paths, per_band=4):
    buckets = defaultdict(list)
    for p in paths:
        doc = json.load(open(p))
        name = p.split("/")[-1].replace(".json", "")
        for (page, region), rows in body_lines(doc).items():
            gs = band_gaps(rows)
            lead = leading_of(gs)
            if not lead or len(gs) < 5:
                continue
            for (a, b) in zip(rows, rows[1:]):
                g = b[0] - a[0]
                if g <= 0:
                    continue
                r = g / lead
                for lo, hi in BANDS:
                    if lo <= r < hi:
                        buckets[(lo, hi)].append((r, name, page, region, a[3], b[3]))
                        break

    for lo, hi in BANDS:
        rows = buckets.get((lo, hi), [])
        if not rows:
            continue
        print(f"\n{'='*94}\n  GAP {lo}-{hi}x leading   ({len(rows)} occurrences)\n{'='*94}")
        step = max(1, len(rows) // per_band)
        for r, name, page, region, above, below in rows[::step][:per_band]:
            print(f"  [{r:.2f}x] {name} p{page} r{region}")
            print(f"     above: {above[:78]!r}")
            print(f"     below: {below[:78]!r}")


if __name__ == "__main__":
    main(sys.argv[1:])
