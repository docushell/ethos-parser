#!/usr/bin/env python3
"""One MCP session, N tool calls on one representation: seconds per call, whole-session peak memory,
and a digest of every reply line so two builds can be compared byte for byte.

    mcpsession.py BINARY REPR.json TOOL N [node_id]

A call whose reply is a tool or protocol error aborts the run: an error returns instantly and would
read as a perfect speedup. Peak RSS and footprint are `/usr/bin/time -l` over the whole session.
Written for §13; see there for the two readings it exists to refuse.
"""
import hashlib, json, subprocess, sys, time
BIN, REPR, TOOL, N = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
node = sys.argv[5] if len(sys.argv) > 5 else None
p = subprocess.Popen(["/usr/bin/time", "-l", BIN, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
def ask(obj):
    p.stdin.write((json.dumps(obj) + "\n").encode()); p.stdin.flush()
    return p.stdout.readline()
ask({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}})
times, digest = [], hashlib.sha256()
for i in range(N):
    args = {"representation": REPR}
    if TOOL == "node_get":
        args["node_id"] = node
    t = time.perf_counter()
    line = ask({"jsonrpc": "2.0", "id": i + 1, "method": "tools/call", "params": {"name": TOOL, "arguments": args}})
    times.append(time.perf_counter() - t)
    if not line or b'"isError":true' in line or b'"error":' in line:
        sys.exit(f"call {i + 1} did not succeed, so nothing here is a measurement: {line[:300]!r}")
    digest.update(line)
p.stdin.close(); p.stdout.read(); err = p.stderr.read().decode(errors="replace"); p.wait()
rss = fp = 0
for raw in err.splitlines():
    low = raw.lower(); nums = [x for x in low.split() if x.isdigit()]
    if nums and "maximum resident set size" in low: rss = int(nums[0])
    if nums and "peak memory footprint" in low: fp = int(nums[0])
print(json.dumps({"tool": TOOL, "calls": N, "seconds": [round(x, 3) for x in times], "rss_mib": round(rss / 1048576, 1), "footprint_mib": round(fp / 1048576, 1), "replies_sha256": digest.hexdigest()[:16]}))
