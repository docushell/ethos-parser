"""Two probes on files made for the purpose: encryption variants, and a page-local error.

Usage: probes.py SOURCE.pdf [WORKDIR]

The corpora hold one encrypted document (a user password nobody has) and no document that fails
on one page while the rest reads, so the two behaviours the knobs scope document rests on are
shown here on generated files. Nothing is committed: WORKDIR (default: a fresh temp directory)
receives the files, and this script prints what happened.

Encryption. SOURCE.pdf is encrypted with qpdf five ways — an owner password only (empty user
password) at AES-256, RC4-128 and RC4-40, and a user password `secret` at AES-256 and RC4-128 —
and the engine runs `classify` and `extract` on each. For every artifact it prints the node count,
the text, any limitation code mentioning encryption, and which fields differ from the unencrypted
SOURCE's own artifact. `qpdf --requires-password` is printed beside each: exit 3 is "encrypted, the
empty password opens it", exit 0 is "a secret is required".

Page error. `synthetic_page_error.py` writes a three-page file whose page 2 executes `foo`; the
engine runs `extract` on the whole file, then under `--max-pages 1` and `--max-pages 2`, then
`engine_census.py failing-page` bisects to the page. `classify` runs on it too, because it does
not interpret content streams and so cannot see the error `extract` refuses on.

Environment: ETHOS_PARSER_BIN (default: target/release/ethos-parser), QPDF (default: qpdf).
"""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import corpora  # noqa: E402

BIN = os.environ.get("ETHOS_PARSER_BIN") or os.path.join(corpora.REPO, "target", "release", "ethos-parser")
QPDF = os.environ.get("QPDF") or "qpdf"

VARIANTS = [
    # name, qpdf arguments before `--`
    ("owner-only-aes256", ["--encrypt", "--owner-password=owner-only", "--bits=256"]),
    ("owner-only-rc4-128", ["--allow-weak-crypto", "--encrypt", "--owner-password=owner-only", "--bits=128"]),
    ("owner-only-rc4-40", ["--allow-weak-crypto", "--encrypt", "--owner-password=owner-only", "--bits=40"]),
    ("user-secret-aes256", ["--encrypt", "--user-password=secret", "--owner-password=owner", "--bits=256"]),
    ("user-secret-rc4-128", ["--allow-weak-crypto", "--encrypt", "--user-password=secret", "--owner-password=owner", "--bits=128"]),
]


def run(args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def engine_line(stderr):
    for line in stderr.splitlines():
        if line.startswith("engine:"):
            return line.strip()
    return stderr.strip()[:200]


def diff(a, b, path="", out=None):
    out = [] if out is None else out
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            diff(a.get(k), b.get(k), path + "/" + k, out)
    elif a != b:
        out.append(path)
    return out


def encryption(src, work):
    print("== encryption variants of %s, engine %s" % (src, BIN))
    plain = json.loads(run([BIN, "extract", src]).stdout)
    for name, args in VARIANTS:
        out = os.path.join(work, name + ".pdf")
        q = run([QPDF] + args + ["--", src, out])
        if q.returncode != 0:
            print("  %s: qpdf refused to write it: %s" % (name, q.stderr.strip().splitlines()[0]))
            continue
        rp = run([QPDF, "--requires-password", out]).returncode
        enc = [l for l in run([QPDF, "--show-encryption", out]).stdout.splitlines()
               if l.startswith("R = ") or l.startswith("file encryption method")]
        c = run([BIN, "classify", out])
        e = run([BIN, "extract", out])
        print("  %s: qpdf --requires-password exit %d; %s" % (name, rp, "; ".join(enc)))
        print("    classify exit %d%s" % (c.returncode, "" if c.returncode == 0 else ": " + engine_line(c.stderr)))
        print("    extract exit %d%s" % (e.returncode, "" if e.returncode == 0 else ": " + engine_line(e.stderr)))
        if e.returncode == 0:
            a = json.loads(e.stdout)
            r = a["representation"]
            codes = [l["code"] for l in r["assurance"]["limitations"] if "crypt" in l["code"] or "password" in l["code"]]
            print("    nodes %d, text %r, terminal %s, limitation codes about encryption: %s" % (
                len(r["nodes"]), [n["text"] for n in r["nodes"] if n["kind"] == "text_run"][:3],
                r["assurance"]["terminal_state"]["state"], codes or "none"))
            print("    fields differing from the unencrypted source's artifact: %s" % ", ".join(diff(plain, a)))


def page_error(work):
    pdf = os.path.join(work, "page2-unknown-operator.pdf")
    subprocess.run([sys.executable, os.path.join(HERE, "synthetic_page_error.py"), pdf], check=True)
    print("== page-local error: %s (qpdf --check: %s)" % (
        pdf, run([QPDF, "--check", pdf]).stdout.strip().splitlines()[-2]))
    e = run([BIN, "extract", pdf])
    print("  extract: exit %d: %s" % (e.returncode, engine_line(e.stderr)))
    for n in (1, 2):
        e = run([BIN, "extract", "--max-pages", str(n), pdf])
        line = "  extract --max-pages %d: exit %d" % (n, e.returncode)
        if e.returncode == 0:
            r = json.loads(e.stdout)["representation"]
            A = r["assurance"]
            line += "; terminal %s; coverage %s; pages %s; nodes %s" % (
                json.dumps(A["terminal_state"]), json.dumps(A["coverage"]),
                [(p["id"], p["index"]) for p in r["pages"]],
                [(x["id"], x["text"]) for x in r["nodes"] if x["kind"] == "text_run"])
        else:
            line += ": " + engine_line(e.stderr)
        print(line)
    b = run([sys.executable, os.path.join(HERE, "engine_census.py"), "failing-page", pdf])
    print("  failing-page: " + b.stdout.strip())
    c = run([BIN, "classify", pdf])
    print("  classify: exit %d" % c.returncode)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    work = sys.argv[2] if len(sys.argv) > 2 else tempfile.mkdtemp(prefix="knobs-probes-")
    os.makedirs(work, exist_ok=True)
    print("engine: %s (%s)" % (BIN, run([BIN, "--version"]).stdout.strip()))
    print("qpdf: %s" % run([QPDF, "--version"]).stdout.strip().splitlines()[0])
    encryption(sys.argv[1], work)
    page_error(work)
