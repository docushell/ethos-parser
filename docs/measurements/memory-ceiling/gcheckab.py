#!/usr/bin/env python3
"""Interleaved A/B of `grounding-check`: wall, peak RSS, peak footprint, and whether reports agree.

`abload.py` treats a non-zero exit as a failed run, and `grounding-check` exits 1 on an invalid
artifact, so it cannot measure one. One warm-up run per arm, then 5 repetitions with the arms
alternating inside each; medians. Every report of every run is hashed with its exit code, and the
line says IDENTICAL only if all of them agree. Usage: gcheckab.py label=binary label=binary file ...
"""
import hashlib, statistics, subprocess, sys, time

ARMS = [a.split("=", 1) for a in sys.argv[1:3]]
FILES = sys.argv[3:]
MIB = 1024 * 1024


def one(binary, path):
    t0 = time.monotonic()
    p = subprocess.run(["/usr/bin/time", "-l", binary, "grounding-check", path], capture_output=True)
    wall = time.monotonic() - t0
    peak = {}
    for raw in p.stderr.decode(errors="replace").splitlines():
        for key in ("maximum resident set size", "peak memory footprint"):
            if key in raw.lower():
                peak[key] = int(raw.split()[0]) / MIB
    digest = hashlib.sha256(p.stdout + bytes([p.returncode])).hexdigest()
    return wall, peak["maximum resident set size"], peak["peak memory footprint"], digest, p.returncode


for _, binary in ARMS:
    one(binary, FILES[0])
for path in FILES:
    runs = {label: [] for label, _ in ARMS}
    for _ in range(5):
        for label, binary in ARMS:
            runs[label].append(one(binary, path))
    digests = {r[3] for v in runs.values() for r in v}
    print(f"### {path.split('/')[-1]}  reports {'IDENTICAL' if len(digests) == 1 else 'DIFFER'}  "
          f"exit {runs[ARMS[0][0]][0][4]}")
    for label, _ in ARMS:
        wall, rss, fp = (statistics.median(r[i] for r in runs[label]) for i in range(3))
        print(f"  {label:<10} wall {wall:.3f}s  RSS {rss:7.1f}M  fp {fp:7.1f}M")
