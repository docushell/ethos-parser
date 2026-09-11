#!/usr/bin/env python3
"""Interleaved A/B over the commands that READ a representation: peak RSS, peak footprint, wall.

Usage: abload.py REPRS_DIR REPS label=binary ... -- sub:doc sub:doc ...
  e.g. ground:nist-sp-800-53Ar5 markdown:nist-sp-800-161r1

6.2 measured `extract` and MCP `extract` and nothing that loads an artifact back in. Those commands
read the whole file into one buffer, parse it into the typed representation, rebuild the canonical
payload to verify the fingerprint, then project and emit. The first arm is the baseline; every
delta is against it. Arms alternate inside each repetition. Output is hashed as it streams and every
run of every arm must produce the same digest — a measurement arm that skips verification still has
to produce identical output on a valid input, or it is measuring a different program.
"""
import hashlib, pathlib, statistics, subprocess, sys, time

args = sys.argv[1:]
REPRS, REPS = pathlib.Path(args[0]), int(args[1])
sep = args.index("--")
ARMS = [a.split("=", 1) for a in args[2:sep]]
CASES = [c.split(":", 1) for c in args[sep + 1:]]
MIB = 1024 * 1024


def one(binary, sub, path):
    t0 = time.monotonic()
    p = subprocess.Popen(["/usr/bin/time", "-l", binary, sub, str(path)],
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    h = hashlib.sha256()
    for c in iter(lambda: p.stdout.read(1 << 20), b""):
        h.update(c)
    err = p.stderr.read().decode(errors="replace")
    p.wait()
    wall = time.monotonic() - t0
    rss = fp = None
    for raw in err.splitlines():
        low = raw.lower()
        nums = [t for t in low.replace(":", " ").split() if t.isdigit()]
        if not nums:
            continue
        if "maximum resident set size" in low:
            rss = int(nums[0]) * (1024 if "Maximum resident" in raw else 1)
        elif "peak memory footprint" in low:
            fp = int(nums[0])
    return rss, fp, wall, h.hexdigest(), p.returncode


broken = False
for sub, doc in CASES:
    path = REPRS / f"{doc}.json"
    runs = {l: [] for l, _ in ARMS}
    digests = set()
    for _ in range(REPS):
        for label, binary in ARMS:
            r = one(binary, sub, path)
            if r[4] != 0 or r[0] is None:
                print(f"  *** {label} {sub} failed on {doc}: exit {r[4]}"); broken = True; continue
            runs[label].append(r); digests.add(r[3])
    med = {l: [statistics.median(x[i] for x in v) for i in range(3)] for l, v in runs.items() if v}
    base = med[ARMS[0][0]]
    print(f"### {sub} {doc}   ({REPS} interleaved)   input {path.stat().st_size/MIB:.0f} MiB   bytes: "
          f"{'IDENTICAL' if len(digests) == 1 else f'*** {len(digests)} DIGESTS ***'}")
    for label, _ in ARMS:
        r, f, w = med[label]
        tag = "" if label == ARMS[0][0] else (
            f"   RSS {(r-base[0])/MIB:+8.1f}  fp {(f-base[1])/MIB:+8.1f}  wall {(w/base[2]-1)*100:+5.1f}%")
        print(f"  {label:<16} RSS {r/MIB:8.1f}M  fp {f/MIB:8.1f}M  wall {w:6.2f}s{tag}")
        print(f"  {'':<16} RSS runs {' '.join(f'{x[0]/MIB:.0f}' for x in runs[label])}")
    broken |= len(digests) != 1
    print(flush=True)
sys.exit(1 if broken else 0)
