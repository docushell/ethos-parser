#!/usr/bin/env python3
"""Interleaved A/B of MCP sessions: what the verification ledger costs and saves.

    ledgerab.py REPRS_DIR REPS A=binary B=binary

REPRS_DIR holds `<doc>.json` representations extracted by arm A. Each workload is one MCP session
(one server process) making a sequence of calls; arms alternate inside every repetition. For every
session this records seconds per call, peak RSS and footprint over the session (`/usr/bin/time -l`),
the server's resident memory two seconds after its last reply, and a digest of every reply line.

A reply that is a tool error aborts the run unless the workload expects errors, and every arm must
produce the same reply digests for the same workload — a ledger that changed an answer is not being
measured, it is broken. Written for §14, whose refuse-to-ship lines these numbers are held to:

  - a first call (miss) on the largest document no more than +6% slower than A;
  - a parse-error call no more than +2% slower (it must not hash before parsing);
  - peak memory no more than A's worst run + max(1%, 32 MiB);
  - later calls at least 35% faster than A's later calls.
"""
import hashlib, json, pathlib, statistics, subprocess, sys, time

REPRS, REPS = pathlib.Path(sys.argv[1]), int(sys.argv[2])
ARMS = [a.split("=", 1) for a in sys.argv[3:]]
MIB = 1024 * 1024


def node_id(doc):
    with open(REPRS / f"{doc}.json") as f:
        nodes = json.load(f)["representation"]["nodes"]
    return nodes[len(nodes) // 2]["id"]


def session(binary, calls, expect_errors=False):
    t_proc = subprocess.Popen(["/usr/bin/time", "-l", binary, "mcp"],
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    server = None
    for _ in range(50):
        out = subprocess.run(["pgrep", "-P", str(t_proc.pid)], capture_output=True, text=True).stdout.split()
        if out:
            server = int(out[0])
            break
        time.sleep(0.02)
    comm = subprocess.run(["ps", "-o", "comm=", "-p", str(server)], capture_output=True, text=True).stdout.strip()
    assert pathlib.Path(comm).name == pathlib.Path(binary).name, f"sampling the wrong process: {comm!r}"

    def ask(obj):
        t_proc.stdin.write((json.dumps(obj) + "\n").encode())
        t_proc.stdin.flush()
        return t_proc.stdout.readline()

    ask({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {}})
    seconds, digest = [], hashlib.sha256()
    for i, (tool, args) in enumerate(calls):
        t0 = time.monotonic()
        line = ask({"jsonrpc": "2.0", "id": i + 1, "method": "tools/call",
                    "params": {"name": tool, "arguments": args}})
        seconds.append(time.monotonic() - t0)
        errored = (not line) or b'"isError":true' in line or b'"error":' in line
        if errored and not expect_errors:
            sys.exit(f"call {i + 1} errored, so nothing here is a measurement: {line[:240]!r}")
        if expect_errors and not errored:
            sys.exit(f"call {i + 1} was expected to fail and did not: {line[:240]!r}")
        digest.update(line)
    time.sleep(2)
    resting = int(subprocess.run(["ps", "-o", "rss=", "-p", str(server)], capture_output=True, text=True).stdout.strip() or 0) * 1024
    t_proc.stdin.close()
    t_proc.stdout.read()
    err = t_proc.stderr.read().decode(errors="replace")
    t_proc.wait()
    rss = fp = 0
    for raw in err.splitlines():
        low = raw.lower()
        nums = [x for x in low.split() if x.isdigit()]
        if nums and "maximum resident set size" in low:
            rss = int(nums[0])
        if nums and "peak memory footprint" in low:
            fp = int(nums[0])
    return {"seconds": seconds, "rss": rss, "fp": fp, "resting": resting, "digest": digest.hexdigest()[:16]}


def workloads():
    big, mid, small = "nist-sp-800-53Ar5", "nist-sp-800-161r1", "nist-sp-800-171r3"
    path = lambda d: str(REPRS / f"{d}.json")
    half = REPRS / f"{big}.half.json"
    if not half.exists():
        data = (REPRS / f"{big}.json").read_bytes()
        half.write_bytes(data[: len(data) // 2])
    pdf = str(pathlib.Path(__file__).resolve().parents[3] / "fixtures/gate" / f"{big}.pdf")
    yield "W1 ground once (miss)", big, [("ground", {"representation": path(big)})], False
    yield "W1 ground once (miss)", mid, [("ground", {"representation": path(mid)})], False
    for d, n in [(big, 5), (mid, 8), (small, 12)]:
        yield f"W3 node_get x{n}", d, [("node_get", {"representation": path(d), "node_id": node_id(d)})] * n, False
    yield "W4 ground x3", big, [("ground", {"representation": path(big)})] * 3, False
    yield "W8 truncated file", big, [("node_get", {"representation": str(half), "node_id": "s1"})] * 2, True
    yield "W8 PDF passed as a representation", big, [("node_get", {"representation": pdf, "node_id": "s1"})] * 2, True


def main():
    results = []
    for name, doc, calls, expect_errors in workloads():
        runs = {label: [] for label, _ in ARMS}
        for _ in range(REPS):
            for label, binary in ARMS:
                runs[label].append(session(binary, calls, expect_errors))
        digests = {r["digest"] for rs in runs.values() for r in rs}
        print(f"### {name} — {doc}  ({REPS} interleaved)  replies: {'IDENTICAL' if len(digests) == 1 else 'DIFFER ' + str(digests)}")
        base = ARMS[0][0]
        first_a = statistics.median(r["seconds"][0] for r in runs[base])
        later_a = statistics.median(s for r in runs[base] for s in r["seconds"][1:]) if len(calls) > 1 else None
        for label, _ in ARMS:
            rs = runs[label]
            first = statistics.median(r["seconds"][0] for r in rs)
            later = statistics.median(s for r in rs for s in r["seconds"][1:]) if len(calls) > 1 else None
            rss = max(r["rss"] for r in rs) / MIB
            fp = max(r["fp"] for r in rs) / MIB
            resting = statistics.median(r["resting"] for r in rs) / MIB
            line = f"  {label:<8} first {first:7.3f}s ({(first / first_a - 1) * 100:+6.1f}%)"
            if later is not None:
                line += f"  later {later:7.3f}s ({(later / later_a - 1) * 100:+6.1f}%)"
            line += f"  peak RSS {rss:7.1f}M  fp {fp:7.1f}M  resting {resting:7.1f}M"
            print(line)
        results.append({"workload": name, "doc": doc, "runs": runs})
        sys.stdout.flush()
    out = REPRS / "ledgerab.json"
    out.write_text(json.dumps(results))
    print(f"\nraw runs: {out}")


if __name__ == "__main__":
    main()
