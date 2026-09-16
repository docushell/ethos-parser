"""What the engine does with every corpus PDF: exit codes, refusal codes, and per-page states.

Usage:
  engine_census.py run [--out FILE] [CORPUS ...]     run classify and extract over the corpora
  engine_census.py summary [FILE]                    tabulate a results file
  engine_census.py failing-page PDF                  bisect `--max-pages` to the first failing page

`run` writes one TSV row per PDF: the classify exit code and refusal code, the extract exit code,
refusal code and message, and — when extract produced an artifact — the artifact's
`assurance.coverage` buckets, its terminal state, and the limitation codes it declares. The refusal
code is the bracketed token the CLI prints at the end of its stderr line (`[encrypted]`,
`[unsupported]`, ...). Pages are counted from the artifact, never from qpdf, so a row says what the
engine saw.

The artifact is written to a temporary file and only its `assurance` object is parsed, by finding
the `"representation":{"assurance":` marker and matching braces: canonical JSON sorts keys, so the
assurance object is the first member of `representation`, and a 950 MB artifact does not have to be
loaded to read its coverage.

`failing-page` exists because the engine fails a document on its first page error, so an artifact
never records which page failed. Under `--max-pages N` pages past N are quarantined rather than
read, so the smallest N at which extract fails is the first failing page. Every probe is a full
extract run.

Environment: ETHOS_PARSER_BIN (default: target/release/ethos-parser), KNOBS_TMP (where artifacts
are written while their assurance is read; default: the system temp directory), plus corpora.py's.
"""
import collections
import json
import mmap
import os
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import corpora  # noqa: E402

BIN = os.environ.get("ETHOS_PARSER_BIN") or os.path.join(corpora.REPO, "target", "release", "ethos-parser")
TMP = os.environ.get("KNOBS_TMP") or tempfile.gettempdir()
MARKER = b'"representation":{"assurance":'

COLUMNS = [
    "corpus", "doc", "classify_exit", "classify_code", "extract_exit", "extract_code",
    "extract_message", "pages_authorized", "pages_processed", "pages_failed", "pages_unsupported",
    "pages_quarantined", "pages_not_attempted", "terminal", "limitation_codes", "artifact_bytes",
    "extract_wall_s",
]


def refusal_code(stderr):
    """The `[code]` the CLI ends its engine line with, or '' when it printed none."""
    for line in stderr.splitlines():
        line = line.strip()
        if line.startswith("engine:") and line.endswith("]"):
            return line[line.rfind("[") + 1:-1]
    return ""


def engine_message(stderr):
    for line in stderr.splitlines():
        if line.startswith("engine:"):
            msg = line[len("engine:"):].strip()
            return msg[: msg.rfind("[")].strip() if msg.endswith("]") else msg
    return stderr.strip().replace("\t", " ").replace("\n", " ")[:200]


def assurance_of(artifact_path):
    """The artifact's `representation.assurance` object, read without loading the artifact."""
    with open(artifact_path, "rb") as f:
        if os.fstat(f.fileno()).st_size == 0:
            return None
        m = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
        try:
            at = m.find(MARKER)
            if at < 0:
                return None
            i = at + len(MARKER)
            depth, in_str, esc, start = 0, False, False, i
            while i < len(m):
                c = m[i]
                if in_str:
                    if esc:
                        esc = False
                    elif c == 0x5C:  # backslash
                        esc = True
                    elif c == 0x22:  # quote
                        in_str = False
                elif c == 0x22:
                    in_str = True
                elif c == 0x7B:  # {
                    depth += 1
                elif c == 0x7D:  # }
                    depth -= 1
                    if depth == 0:
                        return json.loads(m[start:i + 1].decode("utf-8"))
                i += 1
            return None
        finally:
            m.close()


def run_one(corpus, path):
    row = dict((c, "") for c in COLUMNS)
    row["corpus"] = corpus
    row["doc"] = corpora.label(corpus, path)

    p = subprocess.run([BIN, "classify", path], capture_output=True, text=True)
    row["classify_exit"] = p.returncode
    row["classify_code"] = refusal_code(p.stderr)

    fd, artifact = tempfile.mkstemp(prefix="knobs-", suffix=".json", dir=TMP)
    os.close(fd)
    try:
        t0 = time.perf_counter()
        with open(artifact, "wb") as out:
            p = subprocess.run([BIN, "extract", path], stdout=out, stderr=subprocess.PIPE, text=True)
        row["extract_wall_s"] = "%.3f" % (time.perf_counter() - t0)
        row["extract_exit"] = p.returncode
        row["extract_code"] = refusal_code(p.stderr)
        row["extract_message"] = engine_message(p.stderr) if p.returncode != 0 else ""
        if p.returncode == 0:
            row["artifact_bytes"] = os.path.getsize(artifact)
            a = assurance_of(artifact)
            if a is None:
                row["terminal"] = "assurance-not-found"
            else:
                cov = a["coverage"]
                for k in ("pages_authorized", "pages_processed", "pages_failed", "pages_unsupported",
                          "pages_quarantined", "pages_not_attempted"):
                    row[k] = cov[k]
                row["terminal"] = a["terminal_state"]["state"]
                row["limitation_codes"] = ";".join(sorted(set(l["code"] for l in a["limitations"])))
    finally:
        os.unlink(artifact)
    return row


def cmd_run(argv):
    out = None
    only = []
    i = 0
    while i < len(argv):
        if argv[i] == "--out":
            out = argv[i + 1]
            i += 2
        else:
            only.append(argv[i])
            i += 1
    if out is None:
        out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "results", "engine-census.tsv")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    docs = corpora.pdfs(only or None)
    with open(out, "w") as f:
        f.write("\t".join(COLUMNS) + "\n")
        for n, (corpus, path) in enumerate(docs, 1):
            row = run_one(corpus, path)
            f.write("\t".join(str(row[c]) for c in COLUMNS) + "\n")
            f.flush()
            print("%d/%d %s %s classify=%s extract=%s %s" % (
                n, len(docs), corpus, row["doc"], row["classify_exit"], row["extract_exit"],
                row["extract_code"]), file=sys.stderr)
    print("wrote %s (%d rows)" % (out, len(docs)), file=sys.stderr)


def read_tsv(path):
    with open(path) as f:
        header = f.readline().rstrip("\n").split("\t")
        return [dict(zip(header, line.rstrip("\n").split("\t"))) for line in f if line.strip()]


def cmd_summary(argv):
    path = argv[0] if argv else os.path.join(os.path.dirname(os.path.abspath(__file__)), "results", "engine-census.tsv")
    rows = read_tsv(path)
    by = collections.OrderedDict()
    for r in rows:
        by.setdefault(r["corpus"], []).append(r)
    print("corpus\tdocs\tclassify_exit_2\tclassify_encrypted\textract_exit_0\textract_exit_2\textract_encrypted\tpages_authorized\tpages_processed\tpages_failed\tpages_quarantined\tpartial")
    tot = collections.Counter()
    for corpus, rs in by.items():
        c = collections.Counter()
        c["docs"] = len(rs)
        c["classify_exit_2"] = sum(1 for r in rs if r["classify_exit"] == "2")
        c["classify_encrypted"] = sum(1 for r in rs if r["classify_code"] == "encrypted")
        c["extract_exit_0"] = sum(1 for r in rs if r["extract_exit"] == "0")
        c["extract_exit_2"] = sum(1 for r in rs if r["extract_exit"] == "2")
        c["extract_encrypted"] = sum(1 for r in rs if r["extract_code"] == "encrypted")
        for k in ("pages_authorized", "pages_processed", "pages_failed", "pages_quarantined"):
            c[k] = sum(int(r[k]) for r in rs if r[k] != "")
        c["partial"] = sum(1 for r in rs if r["terminal"] == "partial")
        tot.update(c)
        print("\t".join([corpus] + [str(c[k]) for k in ("docs", "classify_exit_2", "classify_encrypted", "extract_exit_0", "extract_exit_2", "extract_encrypted", "pages_authorized", "pages_processed", "pages_failed", "pages_quarantined", "partial")]))
    print("\t".join(["ALL"] + [str(tot[k]) for k in ("docs", "classify_exit_2", "classify_encrypted", "extract_exit_0", "extract_exit_2", "extract_encrypted", "pages_authorized", "pages_processed", "pages_failed", "pages_quarantined", "partial")]))

    print("\nextract refusals (exit 2), by code and message:")
    for r in rows:
        if r["extract_exit"] != "0":
            print("  %s\t%s\t%s\t%s" % (r["corpus"], r["doc"], r["extract_code"], r["extract_message"]))

    print("\nclassify and extract disagree on exit 2:")
    n = 0
    for r in rows:
        if (r["classify_exit"] == "2") != (r["extract_exit"] == "2"):
            n += 1
            print("  %s\t%s\tclassify=%s [%s]\textract=%s [%s]" % (r["corpus"], r["doc"], r["classify_exit"], r["classify_code"], r["extract_exit"], r["extract_code"]))
    if n == 0:
        print("  none")

    print("\nartifacts whose terminal state is not `complete`:")
    n = 0
    for r in rows:
        if r["extract_exit"] == "0" and r["terminal"] != "complete":
            n += 1
            print("  %s\t%s\t%s" % (r["corpus"], r["doc"], r["terminal"]))
    if n == 0:
        print("  none")

    print("\nextract wall time (s) per corpus: total, max (doc):")
    for corpus, rs in by.items():
        ws = [(float(r["extract_wall_s"]), r["doc"]) for r in rs if r["extract_wall_s"] != ""]
        if ws:
            print("  %s\t%.1f\t%.1f (%s)" % (corpus, sum(w for w, _ in ws), max(ws)[0], max(ws)[1]))


def cmd_failing_page(argv):
    pdf = argv[0]
    npages = int(subprocess.run(["qpdf", "--show-npages", pdf], capture_output=True, text=True).stdout.strip())

    def fails(n):
        p = subprocess.run([BIN, "extract", "--max-pages", str(n), pdf], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
        return p.returncode != 0, p.stderr.strip()

    ok0, err0 = fails(0)
    if ok0:
        print("fails at --max-pages 0: not a page-local error: %s" % err0)
        return
    full, errf = fails(npages)
    if not full:
        print("does not fail at --max-pages %d (the full document)" % npages)
        return
    lo, hi = 0, npages  # fails(lo) is False, fails(hi) is True
    probes = 2
    while hi - lo > 1:
        mid = (lo + hi) // 2
        probes += 1
        if fails(mid)[0]:
            hi = mid
        else:
            lo = mid
    print("%s: %d pages; first failing page %d (%d probes); pages 1..%d extract cleanly; error: %s" % (
        pdf, npages, hi, probes, lo, errf))


if __name__ == "__main__":
    if len(sys.argv) < 2 or sys.argv[1] not in ("run", "summary", "failing-page"):
        print(__doc__)
        sys.exit(2)
    {"run": cmd_run, "summary": cmd_summary, "failing-page": cmd_failing_page}[sys.argv[1]](sys.argv[2:])
