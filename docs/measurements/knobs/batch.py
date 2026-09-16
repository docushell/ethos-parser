"""Wall time of `extract` over a directory: a plain sequential loop against `xargs -P N`.

Usage:
  batch.py DIR [--parallel N ...] [--repeat R] [--label NAME]

For each repetition, runs `extract` on every PDF in DIR three ways and times each whole pass:
sequentially, one process after another, as a shell `for` loop would; and through
`xargs -P N -n 1`, for each N given (default 4). Output goes to /dev/null; exit codes are
tallied so a pass that refused everything cannot pass for a fast one — the instrument bug
`docs/measurements/cross-architecture/` recorded. Prints one line per pass and a final table of
the minimum and median per method.

The engine already runs a document's pages in parallel (rayon, `extract.rs`), so the gain from
running documents in parallel depends on how many pages each has: single-page documents leave
every core but one idle inside the engine, multi-page ones do not. Run it on both kinds.

Environment: ETHOS_PARSER_BIN (default: target/release/ethos-parser).
"""
import collections
import os
import statistics
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpora  # noqa: E402

BIN = os.environ.get("ETHOS_PARSER_BIN") or os.path.join(corpora.REPO, "target", "release", "ethos-parser")


def sequential(pdfs):
    codes = collections.Counter()
    t0 = time.perf_counter()
    for p in pdfs:
        r = subprocess.run([BIN, "extract", p], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        codes[r.returncode] += 1
    return time.perf_counter() - t0, codes


def xargs(pdfs, n):
    # `sh -c` so each child's stdout and stderr go to /dev/null as they would in a shell loop;
    # exit codes are recovered from a per-file marker line rather than xargs's summary.
    script = '"$0" extract "$1" >/dev/null 2>&1; echo "$1 $?"'
    t0 = time.perf_counter()
    r = subprocess.run(["xargs", "-P", str(n), "-n", "1", "sh", "-c", script, BIN],
                       input="\n".join(pdfs) + "\n", capture_output=True, text=True)
    wall = time.perf_counter() - t0
    codes = collections.Counter(int(line.rsplit(" ", 1)[1]) for line in r.stdout.splitlines() if line.strip())
    return wall, codes


def main(argv):
    d, par, rep, label = None, [], 3, None
    i = 0
    while i < len(argv):
        if argv[i] == "--parallel":
            par.append(int(argv[i + 1])); i += 2
        elif argv[i] == "--repeat":
            rep = int(argv[i + 1]); i += 2
        elif argv[i] == "--label":
            label = argv[i + 1]; i += 2
        else:
            d = argv[i]; i += 1
    if d is None:
        print(__doc__); sys.exit(2)
    par = par or [4]
    pdfs = sorted(os.path.join(d, f) for f in os.listdir(d) if f.lower().endswith(".pdf"))
    label = label or os.path.basename(os.path.normpath(d))
    print("%s: %d PDFs, %d repetitions, %s" % (label, len(pdfs), rep, BIN))
    results = collections.defaultdict(list)
    for k in range(rep):
        w, c = sequential(pdfs)
        results["sequential"].append(w)
        print("  rep %d sequential      %7.2f s  exits %s" % (k + 1, w, dict(c)))
        for n in par:
            w, c = xargs(pdfs, n)
            results["xargs -P %d" % n].append(w)
            print("  rep %d xargs -P %-3d    %7.2f s  exits %s" % (k + 1, n, w, dict(c)))
    print("\nmethod\tmin_s\tmedian_s\tspeedup_vs_sequential_min")
    base = min(results["sequential"])
    for m, ws in results.items():
        print("%s\t%.2f\t%.2f\t%.2fx" % (m, min(ws), statistics.median(ws), base / min(ws)))


if __name__ == "__main__":
    main(sys.argv[1:])
