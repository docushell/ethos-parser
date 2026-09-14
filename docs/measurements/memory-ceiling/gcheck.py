#!/usr/bin/env python3
"""Peak memory of `grounding-check` over grounding artifacts `ground` produced.

`grounding_check` parsed the grounding artifact TWICE — into a `serde_json::Value` tree
(check.rs:483), then into the typed `GroundingSource` (check.rs:511) — the tree-multiplier shape
MCP had before it was fixed; an artifact that parses no longer builds the tree (README §12). Run without the optional source PDF: that step only hashes the PDF,
which costs nothing next to the two parses. The exit code is recorded, not treated as a failure,
because it reports validation findings. Median of 3. Usage: gcheck.py BINARY GROUNDINGS_DIR doc ...
"""
import pathlib, statistics, subprocess, sys

BIN, DIR, DOCS = sys.argv[1], pathlib.Path(sys.argv[2]), sys.argv[3:]
MIB = 1024 * 1024

print(f"{'document':<20} {'grounding':>10} {'RSS':>9} {'footprint':>10} {'RSS/input':>10}  exit")
print("-" * 72)
for d in DOCS:
    g = DIR / f"{d}.json"
    rs = []
    for _ in range(3):
        p = subprocess.run(["/usr/bin/time", "-l", BIN, "grounding-check", str(g)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        rss = fp = None
        for raw in p.stderr.decode(errors="replace").splitlines():
            low = raw.lower()
            nums = [t for t in low.replace(":", " ").split() if t.isdigit()]
            if nums and "maximum resident set size" in low:
                rss = int(nums[0])
            elif nums and "peak memory footprint" in low:
                fp = int(nums[0])
        rs.append((rss, fp, p.returncode))
    rss = statistics.median(r[0] for r in rs) / MIB
    fp = statistics.median(r[1] for r in rs) / MIB
    size = g.stat().st_size / MIB
    print(f"{d:<20} {size:9.1f}M {rss:8.1f}M {fp:9.1f}M {rss/size:9.1f}x  {sorted({r[2] for r in rs})}", flush=True)
