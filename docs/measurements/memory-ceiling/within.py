#!/usr/bin/env python3
"""Does peak RSS scale LINEARLY in pages *within one document*?

The cross-document table cannot answer this. There, MiB/page ranges 3.20 to 9.07, but every
document has different content density — a page of dense tables emits more nodes than a title
page — so the spread conflates "the engine is superlinear" with "these documents differ".

`--max-pages` gives the controlled experiment the cross-document table cannot: same document,
same content, only the page count varies. If MiB/page is flat as the budget rises, the engine is
linear and the cross-document spread is density, which makes a per-page coefficient a usable
bound. If MiB/page RISES with the budget, there is superlinear retention and no per-page
coefficient bounds anything — the number would understate every document larger than the one it
was measured on.

Run on the corpus's worst case (733 pages, 6.6 GiB) and on the v2-S15 reference document, whose
recorded figures are in `extract_budget.rs` and can therefore be re-checked rather than trusted.
"""
import json, pathlib, statistics, subprocess, sys

MIB = 1024 * 1024
BIN = "./target/release/ethos-parser"


def rss(stderr: str):
    for raw in stderr.splitlines():
        if "maximum resident set size" in raw.lower():
            d = [t for t in raw.lower().replace(":", " ").split() if t.isdigit()]
            if d:
                return int(d[0]) * (1024 if "Maximum resident" in raw else 1)
    return None


def measure(pdf, budget, repeat=3):
    got = []
    args = [BIN, "extract"]
    if budget is not None:
        args += ["--max-pages", str(budget)]
    args.append(str(pdf))
    for _ in range(repeat):
        with open("/dev/null", "wb") as sink:
            d = subprocess.run(["/usr/bin/time", "-l"] + args,
                               stdout=sink, stderr=subprocess.PIPE, check=True)
        v = rss(d.stderr.decode(errors="replace"))
        if v:
            got.append(v)
    return int(statistics.median(got)) if got else None


DOCS = [("nist-sp-800-53Ar5", 733, [8, 32, 64, 128, 256, 512, None]),
        ("nist-sp-800-171r3", 120, [8, 32, 64, None])]

for name, total, budgets in DOCS:
    pdf = pathlib.Path("fixtures/gate") / f"{name}.pdf"
    print(f"### {name} — {total} pages, {pdf.stat().st_size/MIB:.2f} MiB in")
    print(f"{'budget':>8} {'peak RSS':>11} {'MiB/page':>9}")
    prev = None
    for b in budgets:
        peak = measure(pdf, b)
        n = b if b is not None else total
        n = min(n, total)
        per = peak / MIB / n if peak else 0
        flag = ""
        if prev and per > prev * 1.15:
            flag = "  <- rising"
        print(f"{str(b or 'none'):>8} {peak/MIB:>10.1f}M {per:>8.2f}{flag}", flush=True)
        prev = per
    print()
