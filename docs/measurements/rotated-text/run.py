#!/usr/bin/env python3
"""Run a base and a new ethos-parser over PDFs and compare every artifact.

usage: run.py BASE_BIN NEW_BIN OUT_DIR [--jobs N] [--no-overlay] PDF...

For each PDF, with each binary: extract, classify and overlay on the PDF; ground, markdown and html
on that binary's own extract. Every artifact pair is byte-compared and every exit code recorded.
A changed extract is categorised node by node (reprdiff.py) and a changed grounding artifact
summarised (grounddiff.py). Artifacts of a PDF with ANY difference are kept in OUT_DIR/<name>/;
identical ones are deleted.

Writes OUT_DIR/results.jsonl (one line per PDF, rewritten from scratch) and prints a summary:
the documents with any difference, per-command counts, and category totals.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys
from collections import Counter
from concurrent.futures import ProcessPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def run(binary, args, out):
    with open(out, "wb") as f:
        p = subprocess.run([binary] + args, stdout=f, stderr=subprocess.PIPE)
    return p.returncode, p.stderr.decode(errors="replace").strip()[-400:]


def name_of(pdf):
    if os.path.basename(pdf) in ("document.pdf", "source.pdf"):
        parent = os.path.basename(os.path.dirname(pdf))
        grand = os.path.basename(os.path.dirname(os.path.dirname(pdf)))
        return grand + "__" + parent
    return os.path.basename(pdf).rsplit(".", 1)[0]


def one(job):
    base, new, out, pdf, overlay = job
    name = name_of(pdf)
    d = os.path.join(out, name)
    os.makedirs(d, exist_ok=True)
    rec = {"pdf": pdf, "name": name, "diff": []}
    files = {}
    for side, binary in (("a", base), ("b", new)):
        e = os.path.join(d, "extract." + side)
        rec["extract_exit_" + side], rec["extract_err_" + side] = run(binary, ["extract", pdf], e)
        files.setdefault("extract", []).append(e)
        c = os.path.join(d, "classify." + side)
        rec["classify_exit_" + side] = run(binary, ["classify", pdf], c)[0]
        files.setdefault("classify", []).append(c)
        if overlay:
            o = os.path.join(d, "overlay." + side)
            rec["overlay_exit_" + side] = run(binary, ["overlay", pdf], o)[0]
            files.setdefault("overlay", []).append(o)
        if rec["extract_exit_" + side] == 0:
            for cmd in ("ground", "markdown", "html"):
                g = os.path.join(d, cmd + "." + side)
                rec[cmd + "_exit_" + side] = run(binary, [cmd, e], g)[0]
                files.setdefault(cmd, []).append(g)
    for cmd, pair in files.items():
        same_exit = rec.get(cmd + "_exit_a") == rec.get(cmd + "_exit_b")
        if len(pair) != 2 or not same_exit or sha(pair[0]) != sha(pair[1]):
            rec["diff"].append(cmd)
    if "extract" in rec["diff"] and rec["extract_exit_a"] == 0 and rec["extract_exit_b"] == 0:
        p = subprocess.run([sys.executable, os.path.join(HERE, "reprdiff.py"), *files["extract"]], capture_output=True, text=True)
        rec["extract_categories"] = json.loads(p.stdout) if p.returncode == 0 else {"error": p.stderr[-500:]}
    if "ground" in rec["diff"] and len(files.get("ground", [])) == 2 and rec.get("ground_exit_a") == 0 == rec.get("ground_exit_b"):
        p = subprocess.run([sys.executable, os.path.join(HERE, "grounddiff.py"), *files["ground"]], capture_output=True, text=True)
        rec["ground_summary"] = json.loads(p.stdout) if p.returncode == 0 else {"error": p.stderr[-500:]}
    if not rec["diff"]:
        shutil.rmtree(d)
    return rec


def main():
    argv = sys.argv[1:]
    jobs = 4
    overlay = True
    if "--jobs" in argv:
        i = argv.index("--jobs")
        jobs = int(argv[i + 1])
        del argv[i : i + 2]
    if "--no-overlay" in argv:
        argv.remove("--no-overlay")
        overlay = False
    base, new, out = argv[0], argv[1], argv[2]
    pdfs = argv[3:]
    os.makedirs(out, exist_ok=True)
    total = Counter()
    changed = []
    per_cmd = Counter()
    with open(os.path.join(out, "results.jsonl"), "w") as log, ProcessPoolExecutor(jobs) as pool:
        for rec in pool.map(one, [(base, new, out, p, overlay) for p in pdfs]):
            log.write(json.dumps(rec) + "\n")
            log.flush()
            if rec["diff"]:
                changed.append(rec["name"])
                for c in rec["diff"]:
                    per_cmd[c] += 1
                cats = rec.get("extract_categories", {}).get("categories", {})
                for k, v in cats.items():
                    total[k] += v
                print("CHANGED", rec["name"], rec["diff"], json.dumps(cats), flush=True)
    print("documents:", len(pdfs), "changed:", len(changed))
    print("changed per artifact:", dict(sorted(per_cmd.items())))
    print("extract categories total:", dict(sorted(total.items())))


if __name__ == "__main__":
    main()
