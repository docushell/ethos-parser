#!/usr/bin/env python3
"""Summarise how two ethos.grounding.v1 artifacts differ.

usage: grounddiff.py BASE NEW
"""
import json
import sys
from collections import Counter


def main():
    a = json.load(open(sys.argv[1], "rb"))
    b = json.load(open(sys.argv[2], "rb"))
    out = {"capabilities": [a.get("capabilities"), b.get("capabilities")] if a.get("capabilities") != b.get("capabilities") else "same"}
    c = Counter()
    for key in ("elements", "spans", "tables", "pages"):
        xa, xb = a.get(key) or [], b.get(key) or []
        c[key + ":count_base"] = len(xa)
        c[key + ":count_new"] = len(xb)
        ta = Counter(json.dumps(x, sort_keys=True) for x in xa)
        tb = Counter(json.dumps(x, sort_keys=True) for x in xb)
        c[key + ":only_base"] = sum((ta - tb).values())
        c[key + ":only_new"] = sum((tb - ta).values())
        if key == "spans":
            strip = lambda s: json.dumps({k: v for k, v in s.items() if k not in ("char_start", "char_end")}, sort_keys=True)
            sa = Counter(strip(x) for x in xa)
            sb = Counter(strip(x) for x in xb)
            c["spans:only_base_ignoring_offsets"] = sum((sa - sb).values())
            c["spans:only_new_ignoring_offsets"] = sum((sb - sa).values())
            c["spans:with_offsets_new"] = sum(1 for x in xb if "char_start" in x)
        if key == "elements":
            texts_a = Counter(x.get("text") for x in xa)
            texts_b = Counter(x.get("text") for x in xb)
            c["elements:texts_only_base"] = sum((texts_a - texts_b).values())
            c["elements:texts_only_new"] = sum((texts_b - texts_a).values())
    out["counts"] = dict(c)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
