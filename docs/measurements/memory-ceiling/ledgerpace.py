#!/usr/bin/env python3
"""Repeat `node_get` on one large representation, with a pause between calls: peak memory by pause.

    ledgerpace.py REPR.json NODE_ID BINARY PAUSE [PAUSE ...]

Each PAUSE (seconds) is one MCP session of five calls, the pause taken before every call after the
first. Written for §14, where back-to-back calls through the ledger peaked ~650 MiB above the
stateless server and a pause brought the peak back down: it separates memory the process holds from
memory the allocator has freed and not yet returned. A call that errors aborts the run.
"""
import json, statistics, subprocess, sys, time

REPR, NODE, BINARY, PAUSES = sys.argv[1], sys.argv[2], sys.argv[3], [float(p) for p in sys.argv[4:]]
MIB = 1024 * 1024


def session(pause, calls=5):
    p = subprocess.Popen(["/usr/bin/time", "-l", BINARY, "mcp"],
                         stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    def ask(obj):
        p.stdin.write((json.dumps(obj) + "\n").encode())
        p.stdin.flush()
        return p.stdout.readline()

    ask({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}})
    seconds = []
    for i in range(calls):
        if i and pause:
            time.sleep(pause)
        t0 = time.monotonic()
        line = ask({"jsonrpc": "2.0", "id": i + 1, "method": "tools/call",
                    "params": {"name": "node_get", "arguments": {"representation": REPR, "node_id": NODE}}})
        seconds.append(time.monotonic() - t0)
        if b'"isError":false' not in line:
            sys.exit(f"call {i + 1} did not succeed: {line[:240]!r}")
    p.stdin.close()
    p.stdout.read()
    err = p.stderr.read().decode(errors="replace")
    p.wait()
    peak = lambda key: next(int(l.split()[0]) for l in err.splitlines() if key in l.lower()) / MIB
    return statistics.median(seconds[1:]), peak("maximum resident set size"), peak("peak memory footprint")


for pause in PAUSES:
    later, rss, fp = session(pause)
    print(f"pause {pause:4.1f}s  later calls {later:6.2f}s  peak RSS {rss:7.1f} MiB  footprint {fp:7.1f} MiB", flush=True)
