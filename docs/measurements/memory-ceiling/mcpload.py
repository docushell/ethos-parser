#!/usr/bin/env python3
"""Peak memory of MCP's representation-taking tools: `ground` and `node_get`.

Both go through `representation_arg`, which reads the file, parses it into the typed
DocumentRepresentation, and verifies its fingerprint by rebuilding the canonical payload.
`node_get` is given an id that does not exist: it pays the whole load and verify, scans the nodes,
and fails closed — so its peak IS the load path's cost, with no projection on top. `ground` adds
the projection and its own artifact. Median of 3; one request per process.
Usage: mcpload.py BINARY REPRS_DIR doc doc ...
"""
import json, pathlib, statistics, subprocess, sys

BIN, REPRS, DOCS = sys.argv[1], pathlib.Path(sys.argv[2]), sys.argv[3:]
MIB = 1024 * 1024


def run(tool, path):
    args = {"representation": str(path)}
    if tool == "node_get":
        args["node_id"] = "no-such-node"
    reqs = [{"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}},
            {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": tool, "arguments": args}}]
    p = subprocess.run(["/usr/bin/time", "-l", BIN, "mcp"],
                       input="".join(json.dumps(r) + "\n" for r in reqs).encode(),
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    rss = fp = None
    for raw in p.stderr.decode(errors="replace").splitlines():
        low = raw.lower()
        nums = [t for t in low.replace(":", " ").split() if t.isdigit()]
        if nums and "maximum resident set size" in low:
            rss = int(nums[0])
        elif nums and "peak memory footprint" in low:
            fp = int(nums[0])
    last = p.stdout.decode(errors="replace").splitlines()[-1] if p.stdout else ""
    tool_error = '"isError":true' in last
    return rss, fp, tool_error, p.returncode


print(f"{'document':<20} {'input':>8} {'tool':<9} {'RSS':>9} {'footprint':>10}  note")
print("-" * 76)
for d in DOCS:
    path = REPRS / f"{d}.json"
    for tool in ("node_get", "ground"):
        rs = [run(tool, path) for _ in range(3)]
        rss = statistics.median(r[0] for r in rs) / MIB
        fp = statistics.median(r[1] for r in rs) / MIB
        note = ("fails closed after load+verify, as intended" if tool == "node_get" and rs[0][2]
                else ("TOOL ERROR" if rs[0][2] else "ok"))
        print(f"{d:<20} {path.stat().st_size/MIB:7.0f}M {tool:<9} {rss:8.1f}M {fp:9.1f}M  {note}", flush=True)
