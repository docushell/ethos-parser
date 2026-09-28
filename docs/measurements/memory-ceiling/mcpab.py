#!/usr/bin/env python3
"""Old vs new MCP server: one session each over the same requests, every reply line compared byte
for byte. Requests: initialize, tools/list, ping, then per document extract, ground, node_get and
locate by path; then error shapes — a tampered representation, a missing path, an unknown tool, an
unknown method, a line that is not JSON — and a notification, which must get no reply. The
tampered copy is written beside the first representation, as <repr>.tampered.json.

usage: mcpab.py <old-bin> <new-bin> <pdf> <repr> [<pdf> <repr> ...]

Written for §16.
"""
import json, subprocess, sys

old, new = sys.argv[1], sys.argv[2]
pairs = [(sys.argv[i], sys.argv[i + 1]) for i in range(3, len(sys.argv), 2)]
reqs = [{"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}},
        {"jsonrpc": "2.0", "id": "a", "method": "tools/list"},
        {"jsonrpc": "2.0", "id": None, "method": "ping"}]
k = 1
for pdf, rep in pairs:
    for name, args in (("extract", {"path": pdf}), ("ground", {"representation": rep}),
                       ("node_get", {"representation": rep, "node_id": "s1"}),
                       ("locate", {"representation": rep, "quote": "the "})):
        reqs.append({"jsonrpc": "2.0", "id": k, "method": "tools/call", "params": {"name": name, "arguments": args}})
        k += 1
tampered = pairs[0][1] + ".tampered.json"
b = bytearray(open(pairs[0][1], "rb").read())
at = b.rindex(b'"text":"') + 8
b[at] ^= 0x20
open(tampered, "wb").write(b)
reqs += [{"jsonrpc": "2.0", "id": 90, "method": "tools/call", "params": {"name": "node_get", "arguments": {"representation": tampered, "node_id": "s1"}}},
         {"jsonrpc": "2.0", "id": 91, "method": "tools/call", "params": {"name": "extract", "arguments": {"path": "/nonexistent.pdf"}}},
         {"jsonrpc": "2.0", "id": 92, "method": "tools/call", "params": {"name": "nope", "arguments": {}}},
         {"jsonrpc": "2.0", "id": 93, "method": "nope"},
         "{not json",
         {"jsonrpc": "2.0", "method": "notifications/initialized"},
         {"jsonrpc": "2.0", "id": 94, "method": "ping"}]


def session(binary):
    lines = [(r if isinstance(r, str) else json.dumps(r)) for r in reqs]
    p = subprocess.run([binary, "mcp"], input=("\n".join(lines) + "\n").encode(), capture_output=True)
    assert p.returncode == 0, p.stderr[:300]
    return p.stdout.split(b"\n")[:-1]


a, c = session(old), session(new)
print(f"replies: old {len(a)}, new {len(c)} (requests {len(reqs)}, one a notification)")
same = sum(1 for x, y in zip(a, c) if x == y)
print(f"identical: {same}/{max(len(a), len(c))}; bytes old {sum(map(len, a))}, new {sum(map(len, c))}")
for i, (x, y) in enumerate(zip(a, c)):
    if x != y:
        print("DIFF at reply", i, x[:160], y[:160])
