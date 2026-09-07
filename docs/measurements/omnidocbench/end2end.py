"""Run OmniDocBench's own `end2end` evaluation against ethos-parser.

    python3 docs/measurements/omnidocbench/end2end.py <work-dir>

Companion to `census.py` beside it, and the same posture: **a measurement script, not product
code.** Nothing imports it, it is not on the gate, and it reads artifacts this engine emits rather
than reaching into it.

The difference from `census.py` is whose numbers come out. The census is this repository's own
instrument answering this repository's own questions. This runs **somebody else's harness against
their own ground truth**, so the numbers are theirs and are comparable to numbers other projects
publish — which is exactly why decision **O26** refuses to publish a row from it. Run it to find
defects; do not quote it.

**Why this file exists.** The first end2end run was assembled by hand over an afternoon — clone,
interpreter, dependencies, corpus, ground truth, prediction format, config — and the working
directory was then destroyed by a single mistyped `rm`. Everything here is re-derivable, which is
the point, but re-deriving it cost an afternoon twice. This is that afternoon, written down.

**The corpus is not vendored and must not be.** OmniDocBench's dataset card states research use
only. This fetches it into a work directory you name, outside the repository.

# What it does, in order

1. Clones OmniDocBench, if the work directory has no copy. **The clone must not be named `ODB` on
   a case-insensitive filesystem beside a directory called `odb`** — see the note at `WORK`.
2. Builds a Python 3.11 venv with `uv`. OmniDocBench requires `>=3.10,<3.12`; macOS system Python
   is 3.9 and cannot run it.
3. Fetches the 981 `v1_0` `ori_pdfs`, which are the only pages of the benchmark that ship as PDFs.
   The main branch has shipped page images alone since 2025-09-25.
4. Fetches `OmniDocBench.json`, the ground truth for all 1 651 pages, and **filters it to the 981**
   this engine has predictions for. Scoring 1 651 pages against 981 predictions would report the
   670 missing ones as total failures and call it a score.
5. Emits one `<stem>.md` per document. A document that produces no artifact gets an **empty file,
   never a skipped one** — a harness that silently omits its failures reports a better number than
   the engine earned.
6. Writes a config and runs the harness.

# What it cannot do here, and why that is not a skip

**CDM is disabled.** OmniDocBench computes the formula term with a LaTeX render loop needing TeX
Live, Ghostscript and ImageMagick, and its own `pyproject.toml` calls for a Linux environment. So
the composite **Overall cannot be computed** — only its text term. `display_formula` Edit_dist
still runs and is reported. State that alongside any number from this script rather than presenting
a partial Overall as an Overall.
"""

import glob
import json
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor

REPO = "https://github.com/opendatalab/OmniDocBench.git"
DATASET = "https://huggingface.co/datasets/opendatalab/OmniDocBench"
LISTING = "https://huggingface.co/api/datasets/opendatalab/OmniDocBench/tree/v1_0/ori_pdfs"
GT_URL = f"{DATASET}/resolve/main/OmniDocBench.json"
PDF_BASE = f"{DATASET}/resolve/v1_0/"

# The clone is `harness`, never `ODB`. APFS and HFS+ are case-insensitive by default, so a
# directory named `ODB` beside one named `odb` IS that directory — and `rm -rf ODB` deletes it.
# That is not hypothetical: it destroyed a 981-PDF corpus and a day of working notes once.
CLONE_DIR = "harness"


def run(cmd, **kw):
    return subprocess.run(cmd, check=True, **kw)


def clone(work):
    dst = os.path.join(work, CLONE_DIR)
    if os.path.isdir(os.path.join(dst, "src")):
        return dst
    run(["git", "clone", "--depth", "1", REPO, dst])
    return dst


def venv(work, harness):
    py = os.path.join(work, "venv", "bin", "python")
    if os.path.exists(py):
        return py
    uv = shutil.which("uv") or os.path.expanduser("~/.local/bin/uv")
    # 3.11 because OmniDocBench pins `>=3.10,<3.12`, and `uv` fetches one rather than requiring a
    # system install. Anything satisfying that range does.
    run([uv, "venv", "--python", "3.11", os.path.join(work, "venv")])
    env = dict(os.environ, VIRTUAL_ENV=os.path.join(work, "venv"))
    run([uv, "pip", "install", "-q", "-e", harness], env=env)
    return py


def corpus(work):
    out = os.path.join(work, "pdfs")
    os.makedirs(out, exist_ok=True)
    if len(glob.glob(os.path.join(out, "*.pdf"))) == 981:
        return out
    with urllib.request.urlopen(LISTING, timeout=90) as r:
        paths = [e["path"] for e in json.load(r) if e["path"].endswith(".pdf")]

    def one(p):
        dst = os.path.join(out, os.path.basename(p))
        if os.path.exists(dst) and os.path.getsize(dst):
            return
        with urllib.request.urlopen(PDF_BASE + urllib.parse.quote(p), timeout=120) as r, open(
            dst, "wb"
        ) as f:
            f.write(r.read())

    with ThreadPoolExecutor(max_workers=12) as ex:
        list(ex.map(one, paths))
    return out


def predictions(binary, pdfs, out):
    """One `<stem>.md` per document. A failure writes an EMPTY file, never no file."""
    os.makedirs(out, exist_ok=True)

    def one(f):
        dst = os.path.join(out, os.path.splitext(os.path.basename(f))[0] + ".md")
        p = subprocess.run([binary, "extract", f], capture_output=True)
        if p.returncode:
            open(dst, "w").write("")
            return "no-artifact"
        with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as fh:
            fh.write(p.stdout)
            tmp = fh.name
        m = subprocess.run([binary, "markdown", tmp], capture_output=True)
        os.unlink(tmp)
        md = json.loads(m.stdout)["markdown"] if m.returncode == 0 else ""
        open(dst, "w").write(md)
        return "ok" if md.strip() else "empty"

    files = sorted(glob.glob(os.path.join(pdfs, "*.pdf")))
    with ThreadPoolExecutor(max_workers=8) as ex:
        res = list(ex.map(one, files))
    import collections as c

    print(f"predictions: {len(files)} ->", dict(c.Counter(res)))
    return out


def ground_truth(work, preds):
    """The full GT, filtered to the pages predictions exist for."""
    full = os.path.join(work, "OmniDocBench.json")
    if not os.path.exists(full):
        with urllib.request.urlopen(GT_URL, timeout=300) as r, open(full, "wb") as f:
            f.write(r.read())
    have = {os.path.splitext(os.path.basename(p))[0] for p in glob.glob(os.path.join(preds, "*.md"))}
    gt = json.load(open(full))
    sub = [
        x
        for x in gt
        if os.path.splitext(os.path.basename(x["page_info"]["image_path"]))[0] in have
    ]
    dst = os.path.join(work, "ground_truth_subset.json")
    json.dump(sub, open(dst, "w"))
    print(f"ground truth: {len(gt)} pages -> {len(sub)} with a prediction")
    return dst


CONFIG = """end2end_eval:
  metrics:
    text_block:
      metric: [Edit_dist]
    display_formula:
      metric: [Edit_dist]
    table:
      metric: [TEDS, Edit_dist]
      teds_workers: 13
    reading_order:
      metric: [Edit_dist]
  dataset:
    dataset_name: end2end_dataset
    ground_truth:
      data_path: {gt}
    prediction:
      data_path: {preds}
    match_method: quick_match
    match_workers: 13
    quick_match_truncated_timeout_sec: 300
    match_timeout_sec: 420
    timeout_fallback_max_chunk_span: 10
    timeout_fallback_order_penalty: 0.10
"""


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    work = os.path.abspath(sys.argv[1])
    os.makedirs(work, exist_ok=True)
    binary = os.environ.get("ETHOS_PARSER_BIN", "target/release/ethos-parser")

    harness = clone(work)
    py = venv(work, harness)
    pdfs = corpus(work)
    preds = predictions(binary, pdfs, os.path.join(work, "predictions"))
    gt = ground_truth(work, preds)

    cfg = os.path.join(work, "end2end.yaml")
    open(cfg, "w").write(CONFIG.format(gt=gt, preds=preds))

    # `python -m src.cli` re-imports the package and does nothing; call `main` directly.
    code = f"from src.cli import main; import sys; sys.argv=['e','--config','{cfg}']; main()"
    subprocess.run([py, "-c", code], cwd=harness, check=False)

    results = os.path.join(harness, "result", "predictions_quick_match_metric_result.json")
    if os.path.exists(results):
        d = json.load(open(results))
        print("\n=== scores (Edit_dist lower is better, TEDS higher) ===")
        for mod in ("text_block", "reading_order", "display_formula", "table"):
            for metric, vals in d.get(mod, {}).get("all", {}).items():
                v = vals.get("ALL_page_avg", vals.get("all"))
                print(f"  {mod:18} {metric:20} {v}")
        print("\n  Overall is NOT computed: CDM needs TeX Live, Ghostscript and ImageMagick.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
