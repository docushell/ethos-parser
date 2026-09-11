#!/usr/bin/env python3
"""Exact sizes of a grounding artifact against ethos.grounding.v1's structural limits.

The limits are the checker's (crates/ethos-parser-grounding/src/check.rs, `mod limits`), which is
differential-tested against Ethos down to verdict, code and path: MAX_PAGES 5,000; MAX_ELEMENTS
1,000,000 — applied to `elements` AND to `spans`; MAX_TABLES 100,000; MAX_STRING_BYTES 16,384.
`ground`'s projection enforces none of them, so this states which ones an emitted artifact crosses.
Usage: gcount.py grounding.json [grounding.json ...]
"""
import json, sys

LIMITS = {"pages": 5_000, "elements": 1_000_000, "spans": 1_000_000, "tables": 100_000}
MAX_STRING_BYTES = 16_384


def longest_string(v, best=(0, "")):
    stack = [("", v)]
    top = best
    while stack:
        path, x = stack.pop()
        if isinstance(x, str):
            n = len(x.encode("utf-8"))
            if n > top[0]:
                top = (n, path)
        elif isinstance(x, dict):
            stack.extend((f"{path}/{k}", y) for k, y in x.items())
        elif isinstance(x, list):
            stack.extend((f"{path}/{i}", y) for i, y in enumerate(x))
    return top


for path in sys.argv[1:]:
    with open(path, "rb") as f:
        g = json.load(f)
    print(f"### {path.rsplit('/', 1)[-1]}")
    crossed = []
    for key, lim in LIMITS.items():
        n = len(g.get(key) or [])
        flag = "  <-- EXCEEDS" if n > lim else ""
        if n > lim:
            crossed.append(key)
        print(f"  {key:<9} {n:>10,}   limit {lim:>10,}{flag}")
    n, where = longest_string(g)
    flag = "  <-- EXCEEDS" if n > MAX_STRING_BYTES else ""
    if n > MAX_STRING_BYTES:
        crossed.append("string")
    print(f"  longest string {n:>6,} bytes at {where}   limit {MAX_STRING_BYTES:,}{flag}")
    print(f"  => {'crosses ' + ', '.join(crossed) if crossed else 'within every limit'}\n", flush=True)
