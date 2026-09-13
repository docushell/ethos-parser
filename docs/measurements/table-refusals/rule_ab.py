#!/usr/bin/env python3
"""The ruled rule under several engines, side by side: what each emits, and where.

    ETHOS_BENCH_CORPUS=~/ethos-external-benchmarks/opendataloader-bench \\
      docs/measurements/table-refusals/rule_ab.py \\
        v5=/path/to/ethos-parser-at-v5 v6=target/release/ethos-parser \\
        [--extra ../ethos/fixtures ...]

Written for §4g, where `ruled-rects-v5` had passed the benchmark at 100% precision and was then
found emitting a paragraph as a 14 x 5 table on five of the eight gate documents. The benchmark's
table documents are not NIST's, so a rule judged only there can fabricate where nobody is looking.
This script therefore reads three populations, and prints each separately:

1. **The benchmark**, against ground truth: documents emitting a ruled table, how many of those hold
   a `Table` element, and the same counted over every table rule. Then every document on which the
   first and last engine named disagree.
2. **The gate and engine fixtures**, which have no ground truth: every ruled table as
   `(page, rows, columns, cells with text, cells)` — a table whose cells carry no text, or one line
   of a paragraph each, is the thing to look at — and the set of pages the ruled rule refused.
3. **Any extra directories** given, deduplicated by content, same columns as 2.

Engines are compared at whatever version each was built at. Nothing here reads a byte digest, so
`parser_version` differing between arms moves no number; for byte identity use `ci/artifact-bytes.py`
at equal version.
"""

import collections
import hashlib
import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
BENCH = Path(os.environ.get("ETHOS_BENCH_CORPUS", "~/ethos-external-benchmarks/opendataloader-bench")).expanduser()
REFUSED_PAGE = re.compile(r"page (\d+) \(")


def artifact(engine, pdf):
    r = subprocess.run([engine, "extract", str(pdf)], capture_output=True)
    return json.loads(r.stdout) if r.returncode == 0 else None


def ruled_view(art):
    """`(tables, refused pages, rules)` for one artifact, or `None` for a refusal."""
    if art is None:
        return None
    rep = art["representation"]
    tables = sorted(
        (t["page"], t["rows"], t["columns"], sum(1 for c in t["cells"] if c["text"].strip()), len(t["cells"]))
        for t in rep.get("tables") or []
        if t["detection_rule"].startswith("ruled-rects")
    )
    rules = collections.Counter(t["detection_rule"].rsplit("-v", 1)[0] for t in rep.get("tables") or [])
    details = []

    def walk(o):
        if isinstance(o, dict):
            if o.get("code") == "ruled-table-candidate-refused":
                details.append(o.get("detail", ""))
            for v in o.values():
                walk(v)
        elif isinstance(o, list):
            for v in o:
                walk(v)

    walk(art)
    return tables, set(REFUSED_PAGE.findall("".join(details))), rules


def run(engines, pdfs):
    with ThreadPoolExecutor(os.cpu_count() or 4) as pool:
        return {
            name: dict(zip(pdfs, pool.map(lambda p, e=engine: ruled_view(artifact(e, p)), pdfs)))
            for name, engine in engines.items()
        }


def benchmark(engines):
    gt = json.loads((BENCH / "ground-truth/reference.json").read_text())
    holds = {d for d, v in gt.items() if any(e.get("category") == "Table" for e in v["elements"])}
    docs = sorted(gt)
    views = run(engines, [BENCH / "pdfs" / d for d in docs])
    print(f"## benchmark — {len(docs)} documents, {len(holds)} holding a table\n")
    for name in engines:
        v = {d: views[name][BENCH / "pdfs" / d] for d in docs}
        ruled = {d for d in docs if v[d] and v[d][0]}
        anyt = {d for d in docs if v[d] and v[d][2]}
        print(
            f"  {name:>6}  ruled: {len(ruled):>3} documents, {len(ruled & holds)} holding a table, "
            f"{len(ruled - holds)} not  |  any rule: {len(anyt):>3}, {len(anyt & holds)} holding, {len(anyt - holds)} not"
        )
    first, last = list(engines)[0], list(engines)[-1]
    for d in docs:
        a, b = views[first][BENCH / "pdfs" / d], views[last][BENCH / "pdfs" / d]
        if (a and a[0]) != (b and b[0]):
            print(f"    {d}  table in ground truth: {d in holds}  {first} {a and a[0]}  {last} {b and b[0]}")
    print()


def fixtures(engines, title, pdfs):
    views = run(engines, pdfs)
    names = list(engines)
    print(f"## {title} — {len(pdfs)} documents; rows print only where an engine disagrees\n")
    for pdf in pdfs:
        vs = [views[n][pdf] for n in names]
        if all(json.dumps(v, default=sorted) == json.dumps(vs[0], default=sorted) for v in vs):
            continue
        label = str(pdf).replace(str(REPO) + "/", "")
        print(f"  {label}")
        for n, v in zip(names, vs):
            if v is None:
                print(f"    {n:>6}  refused")
            else:
                print(f"    {n:>6}  tables {v[0]}  refused pages {len(v[1])}")
        base, top = views[names[0]][pdf], views[names[-1]][pdf]
        if base and top:
            print(f"    pages {names[-1]} refuses that {names[0]} did not: {len(top[1] - base[1])}")
    print()


def main():
    args = sys.argv[1:]
    extra = []
    while "--extra" in args:
        i = args.index("--extra")
        extra.append(Path(args[i + 1]))
        del args[i : i + 2]
    engines = dict(a.split("=", 1) for a in args)
    if len(engines) < 2:
        sys.exit(__doc__)
    if BENCH.exists():
        benchmark(engines)
    else:
        print(f"(no benchmark at {BENCH}; set ETHOS_BENCH_CORPUS)\n")
    local = sorted((REPO / "fixtures/gate").glob("*.pdf")) + sorted((REPO / "fixtures/engine").glob("*/document.pdf"))
    fixtures(engines, "gate and engine fixtures", local)
    if extra:
        seen, pdfs = set(), []
        for pdf in sorted(p for d in extra for p in d.rglob("*.pdf")):
            digest = hashlib.sha256(pdf.read_bytes()).hexdigest()
            if digest not in seen:
                seen.add(digest)
                pdfs.append(pdf)
        fixtures(engines, "extra corpora, deduplicated by content", pdfs)


if __name__ == "__main__":
    main()
