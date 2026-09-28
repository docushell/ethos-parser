#!/usr/bin/env python3
"""Per-line cost: one MCP session, N requests sent one at a time, each awaited; median and p90 in
milliseconds.

    mcplatency.py BINARY N ping
    mcplatency.py BINARY N node_get REPR.json NODE_ID

On a request this small nearly all of it is what the server spends per line. Written for §16.
"""
import json, statistics, subprocess, sys, time

BIN, N, KIND = sys.argv[1], int(sys.argv[2]), sys.argv[3]
p = subprocess.Popen([BIN, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)


def ask(obj):
    p.stdin.write((json.dumps(obj) + "\n").encode()); p.stdin.flush()
    return p.stdout.readline()


ask({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}})
ms = []
for i in range(1, N + 1):
    if KIND == "ping":
        req = {"jsonrpc": "2.0", "id": i, "method": "ping"}
    else:
        req = {"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {
            "name": "node_get", "arguments": {"representation": sys.argv[4], "node_id": sys.argv[5]}}}
    t = time.perf_counter()
    reply = ask(req)
    ms.append((time.perf_counter() - t) * 1000)
    if not reply or b'"error"' in reply or b'"isError":true' in reply:
        sys.exit(f"request {i} did not succeed: {reply[:200]!r}")
p.stdin.close(); p.wait()
ms.sort()
print(f"{KIND} x{N}: median {statistics.median(ms):.2f} ms, p90 {ms[int(0.9 * N)]:.2f} ms")
