#!/usr/bin/env python3
"""Which quantity predicts peak RSS: input bytes, artifact bytes, or page count?

6.2's row says "verify the figure first". The tree carries two figures that cannot both be
ceilings: `ci/bench.py` says peak RSS is "roughly 300x the *input*", and `extract.rs` says
"~4.7 MiB per page, and independent of file size". A caller choosing a bound needs to know
which one is a law and which is a coincidence of this corpus.

The test is not "what is the average ratio" — any of the three produces one. It is whether the
ratio HOLDS: a predictor whose coefficient varies 4x across the corpus cannot bound anything.
So this reports, per candidate, the spread of the implied coefficient (max/min). The winner is
the one whose coefficient is flattest, and a spread near 1.0 is what "a ceiling" means.

Only input bytes and page count are admissible as ceilings regardless of fit: a caller knows
both BEFORE running. Artifact size is knowable only after the run it was meant to bound, so it
is reported for diagnosis and excluded from the verdict.
"""
import sys, json, pathlib, statistics

MIB = 1024 * 1024


def spread(vals):
    lo, hi = min(vals), max(vals)
    return hi / lo if lo else float("inf")


def main():
    tsv, pages_json = sys.argv[1], sys.argv[2]
    pages = json.loads(pathlib.Path(pages_json).read_text())
    rows = []
    for line in pathlib.Path(tsv).read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        f = line.split("\t")
        name, wall, art, rss = f[0], int(f[1]), int(f[2]), f[3]
        if rss in ("", "None", "-"):
            print(f"  {name}: no RSS reading — platform would not say", file=sys.stderr)
            continue
        src = pathlib.Path("fixtures/gate") / f"{name}.pdf"
        if not src.exists():
            src = next(pathlib.Path("fixtures/gate").glob(f"{name}*"), None)
        rows.append({
            "name": name, "wall": wall, "artifact": art, "rss": int(rss),
            "input": src.stat().st_size if src else None,
            "pages": pages.get(name),
        })

    print(f"{'fixture':<26} {'pages':>5} {'input':>9} {'artifact':>10} {'peak RSS':>10} "
          f"{'/input':>8} {'/artifact':>10} {'MiB/page':>9}")
    print("-" * 92)
    per_in, per_art, per_pg = [], [], []
    for r in sorted(rows, key=lambda r: r["pages"] or 0):
        ri = r["rss"] / r["input"] if r["input"] else None
        ra = r["rss"] / r["artifact"] if r["artifact"] else None
        rp = r["rss"] / MIB / r["pages"] if r["pages"] else None
        if ri: per_in.append(ri)
        if ra: per_art.append(ra)
        if rp: per_pg.append(rp)
        print(f"{r['name']:<26} {r['pages'] or 0:>5} {r['input']/MIB:>8.2f}M "
              f"{r['artifact']/MIB:>9.2f}M {r['rss']/MIB:>9.1f}M "
              f"{ri or 0:>7.0f}x {ra or 0:>9.2f}x {rp or 0:>8.2f}")

    print()
    print("MODEL FIT — a ceiling needs a FLAT coefficient, not merely an average")
    print("-" * 92)
    for label, vals, unit, admissible in (
        ("peak = k x input bytes", per_in, "x", True),
        ("peak = k x artifact bytes", per_art, "x", False),
        ("peak = k MiB x pages", per_pg, " MiB/page", True),
    ):
        if not vals:
            continue
        note = "" if admissible else "   [NOT a ceiling: unknown before the run]"
        print(f"  {label:<28} k = {min(vals):.2f}-{max(vals):.2f}{unit}   "
              f"median {statistics.median(vals):.2f}   spread {spread(vals):.2f}x{note}")
    print()
    cands = [("input bytes", spread(per_in)), ("pages", spread(per_pg))]
    cands.sort(key=lambda c: c[1])
    print(f"  Flattest admissible predictor: {cands[0][0]} (spread {cands[0][1]:.2f}x) "
          f"vs {cands[1][0]} ({cands[1][1]:.2f}x)")


main()
