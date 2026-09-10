#!/usr/bin/env python3
"""What does an extract cost when the page budget admits (almost) nothing?

The within-document ladder is linear with a LARGE positive intercept: the 733-page document
already costs 249 MiB at `--max-pages 8`, where eight pages of marginal cost is ~70 MiB. If that
intercept scales with the document's TOTAL page count rather than with the admitted count, then
`--max-pages` does not bound it, and the flag is not a sufficient ceiling for a document with
very many pages however low the budget is set.

`--max-pages 0` is legal and documented as "process none", so the intercept is directly
measurable rather than inferred from a fit.
"""
import pathlib, statistics, subprocess

MIB = 1024 * 1024
BIN = "./target/release/ethos-parser"


def rss(stderr):
    for raw in stderr.splitlines():
        if "maximum resident set size" in raw.lower():
            d = [t for t in raw.lower().replace(":", " ").split() if t.isdigit()]
            if d:
                return int(d[0]) * (1024 if "Maximum resident" in raw else 1)
    return None


def measure(pdf, budget, repeat=3):
    got, wall = [], []
    for _ in range(repeat):
        with open("/dev/null", "wb") as sink:
            d = subprocess.run(["/usr/bin/time", "-l", BIN, "extract",
                                "--max-pages", str(budget), str(pdf)],
                               stdout=sink, stderr=subprocess.PIPE)
        v = rss(d.stderr.decode(errors="replace"))
        if v:
            got.append(v)
    return int(statistics.median(got)) if got else None


for name, total in (("nist-sp-800-53Ar5", 733), ("nist-sp-800-161r1", 327),
                    ("nist-sp-800-171r3", 120), ("irs-fw9", 6)):
    pdf = pathlib.Path("fixtures/gate") / f"{name}.pdf"
    print(f"### {name} — {total} pages total")
    base = None
    for b in (0, 1, 2, 4):
        peak = measure(pdf, b)
        if base is None:
            base = peak
        print(f"  --max-pages {b:>2}: {peak/MIB:>8.1f} MiB", flush=True)
    print(f"  intercept per TOTAL page: {base/MIB/total:.3f} MiB "
          f"(this is the term --max-pages cannot lower)")
    print()
