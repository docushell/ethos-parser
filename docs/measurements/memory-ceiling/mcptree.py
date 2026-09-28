#!/usr/bin/env python3
"""One MCP session, N calls of one tool: seconds per call, the peak resident memory of the server
AND every process it started while each call runs, and the server's own after each reply.

    mcptree.py BINARY TOOL TARGET N [QUOTE] [--inline]

TOOL is extract (TARGET a PDF), or ground, node_get (id s1) or locate (QUOTE) on a representation
at TARGET, sent by path — or, with --inline, as the object itself. Memory is `ps` RSS, sampled every
20 ms: `/usr/bin/time -l` sees one process, and since §16 a session is several. A reply that is a
tool or protocol error aborts the run, because an error returns at once. Written for §16.
"""
import json, subprocess, sys, threading, time

inline = "--inline" in sys.argv
argv = [a for a in sys.argv if a != "--inline"]
BIN, TOOL, TARGET, N = argv[1], argv[2], argv[3], int(argv[4])
QUOTE = argv[5] if len(argv) > 5 else None
body = open(TARGET, "rb").read().rstrip(b"\n") if inline else None
p = subprocess.Popen([BIN, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE)


def rss():
    """The server's own RSS, and the sum over the processes it started, in MiB."""
    out = subprocess.run(["ps", "-A", "-o", "pid=,ppid=,rss="], capture_output=True, text=True).stdout
    rows = [tuple(int(x) for x in l.split()) for l in out.splitlines() if l.strip()]
    own = sum(r for pid, _, r in rows if pid == p.pid)
    started = sum(r for _, ppid, r in rows if ppid == p.pid)
    return own / 1024, started / 1024


def request(i):
    if TOOL == "extract":
        args = {"path": TARGET}
    else:
        args = {"representation": "@"} if inline else {"representation": TARGET}
        if TOOL == "node_get":
            args["node_id"] = "s1"
        if TOOL == "locate":
            args["quote"] = QUOTE
    line = json.dumps({"jsonrpc": "2.0", "id": i, "method": "tools/call",
                       "params": {"name": TOOL, "arguments": args}}).encode()
    return line.replace(b'"@"', body) if inline else line


p.stdin.write(b'{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}\n'); p.stdin.flush()
p.stdout.readline()
print(f"{TOOL} x{N} on {TARGET.split('/')[-1]}{' inline' if inline else ''}: "
      f"after initialize, server {rss()[0]:.1f} MiB")
peaks = []
for i in range(1, N + 1):
    done, peak = threading.Event(), [0.0]

    def sample():
        while not done.is_set():
            peak[0] = max(peak[0], sum(rss()))
            time.sleep(0.02)

    sampler = threading.Thread(target=sample)
    sampler.start()
    t = time.monotonic()
    p.stdin.write(request(i) + b"\n"); p.stdin.flush()
    reply = p.stdout.readline()
    seconds = time.monotonic() - t
    done.set(); sampler.join()
    if not reply or b'"isError":false' not in reply:
        sys.exit(f"call {i} did not succeed, so nothing here is a measurement: {reply[:300]!r}")
    own, started = rss()
    peaks.append(peak[0])
    print(f"  call {i:2d}: {seconds:6.2f} s  peak {peak[0]:8.1f} MiB  after the reply: server "
          f"{own:7.1f} MiB, started processes {started:5.1f} MiB")
p.stdin.close(); p.stdout.read(); p.wait()
print(f"  session peak {max(peaks):.1f} MiB")
