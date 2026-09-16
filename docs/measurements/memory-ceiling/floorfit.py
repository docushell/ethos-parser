#!/usr/bin/env python3
"""The tables and the sizing rule from floor.py's readings.

    floorfit.py floor-b4b4aa9.tsv bench-c14n-adopt.tsv

Prints, as Markdown: the corpus at this build beside the previous record's readings; the floor
table — `classify --sample-pages 0`, `classify`, `extract --max-pages 0`, the full extract — with
the per-document-page coefficient of the floor and the bracket on the structure tree's share;
every budget point against the chord; and the rule.

The rule is a BOUND, so every coefficient is the worst one observed and not a fit: the process
floor is the smallest gate document's `--max-pages 0` peak; the per-document-page term is the
largest `(max0 - floor) / pages` on any gate document; the per-admitted-page term is the largest
`(peak - max0) / admitted` at ANY budget on any gate document, including every full-extract
sample — a coefficient taken from the endpoints alone under-predicts a bounded run on a document
whose early pages are denser than its average. No least squares anywhere.
"""
from __future__ import annotations

import math
import pathlib
import statistics
import sys

MIB = 1024 * 1024
ROOT = pathlib.Path(__file__).resolve().parents[3]
CONTROL = "control:untagged-shredded-line"
OLD_RULE = (7.0, 0.35)  # MiB per admitted page, MiB per document page; no separate floor


def read_rows(tsv: pathlib.Path) -> list[dict]:
    rows = []
    for line in tsv.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        f = line.split("\t")
        rows.append({
            "fixture": f[0], "mode": f[1], "budget": None if f[2] == "-" else int(f[2]),
            "run": int(f[3]), "pages": None if f[4] == "-" else int(f[4]),
            "wall": None if f[5] == "-" else int(f[5]), "rss": int(f[6]),
            "footprint": None if f[7] == "-" else int(f[7]),
            "artifact": None if f[8] == "-" else int(f[8]),
        })
    return rows


def read_bench(tsv: pathlib.Path) -> dict[str, int]:
    out = {}
    for line in tsv.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        f = line.split("\t")
        out[f[0]] = int(f[3])
    return out


def mib(b: float) -> str:
    return f"{b / MIB:.1f}"


def ceil_to(x: float, step: float) -> float:
    return math.ceil(x / step - 1e-9) * step


def main() -> int:
    rows = read_rows(pathlib.Path(sys.argv[1]))
    previous = read_bench(pathlib.Path(sys.argv[2])) if len(sys.argv) > 2 else {}
    names = []
    for r in rows:
        if r["fixture"] not in names:
            names.append(r["fixture"])
    per: dict[str, dict] = {}
    for n in names:
        mine = [r for r in rows if r["fixture"] == n]
        pages = next((r["pages"] for r in mine if r["pages"]), None)
        d = {
            "pages": pages,
            "classify": [r["rss"] for r in mine if r["mode"] == "classify"],
            "classify0": [r["rss"] for r in mine if r["mode"] == "classify0"],
            "max0": [r["rss"] for r in mine if r["mode"] == "max0"],
            "full": [r["rss"] for r in mine if r["mode"] == "full"],
            "full_pipe": [r["rss"] for r in mine if r["mode"] == "full-size-pass"],
            "full_fp": [r["footprint"] for r in mine if r["mode"] == "full"],
            "max0_fp": [r["footprint"] for r in mine if r["mode"] == "max0"],
            "classify0_fp": [r["footprint"] for r in mine if r["mode"] == "classify0"],
            "artifact": next((r["artifact"] for r in mine if r["artifact"]), None),
            "wall_full": [r["wall"] for r in mine if r["mode"] == "full"],
            "budget": {},
            "budget_all": {},
        }
        # A repeated budget keeps its WORST sample for the bound, and every sample for the record.
        for r in mine:
            if r["mode"] == "budget":
                d["budget_all"].setdefault(r["budget"], []).append(r["rss"])
                d["budget"][r["budget"]] = max(d["budget"].get(r["budget"], 0), r["rss"])
        src = ROOT / "fixtures" / "gate" / f"{n}.pdf"
        d["input"] = src.stat().st_size if src.exists() else None
        per[n] = d

    gate = [n for n in names if n != CONTROL]
    by_pages = sorted(gate, key=lambda n: per[n]["pages"])

    # ---- 1. the corpus at this build, beside the previous record
    print("### The corpus at this build\n")
    print("| fixture | pages | input | artifact | peak RSS (/dev/null) | second sample (pipe) | §11 | Δ vs §11 | /input | /artifact | MiB/page | wall |")
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    for n in by_pages:
        d = per[n]
        primary = d["full"][0]
        prev = previous.get(n)
        delta = f"{(primary - prev) / prev:+.1%}" if prev else "—"
        print(f"| {n} | {d['pages']} | {d['input'] / MIB:.2f}M | {d['artifact'] / MIB:.2f}M | {mib(primary)}M | "
              f"{mib(d['full_pipe'][0])}M | {mib(prev) + 'M' if prev else '—'} | {delta} | {primary / d['input']:.0f}x | "
              f"{primary / d['artifact']:.2f}x | {primary / MIB / d['pages']:.2f} | {d['wall_full'][0] / 1e6:.2f} s |")
    extra = {n: per[n]["full"][1:] for n in by_pages if len(per[n]["full"]) > 1}
    if extra:
        print("\nAdditional full samples: " + "; ".join(f"{n} {', '.join(mib(x) + 'M' for x in xs)}" for n, xs in extra.items()))
    print()

    # ---- 2. the floor
    print("### The floor, separated\n")
    print("| fixture | pages | `classify --sample-pages 0` | `classify` | `extract --max-pages 0` | full extract | max0 − classify0 | share of max0 | max0 − floor, per document page | full − max0, per admitted page |")
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")
    floor_doc = by_pages[0]
    floor = statistics.median(per[floor_doc]["max0"])
    widest, widest_at = 0, ""
    for n in by_pages + [CONTROL]:
        d = per[n]
        if d["pages"] is None or not d["max0"]:
            continue
        c0 = statistics.median(d["classify0"]) if d["classify0"] else None
        c = statistics.median(d["classify"]) if d["classify"] else None
        m0 = statistics.median(d["max0"])
        fl = max(d["full"] + d["full_pipe"])
        tree = (m0 - c0) if c0 is not None else None
        for mode in ("classify0", "classify", "max0"):
            if d[mode] and max(d[mode]) - min(d[mode]) > widest:
                widest, widest_at = max(d[mode]) - min(d[mode]), f"{n} {mode}"
        print(f"| {n} | {d['pages']} | {mib(c0) + 'M' if c0 is not None else '—'} | {mib(c) + 'M' if c is not None else '—'} | "
              f"{mib(m0)}M | {mib(fl)}M | "
              f"{'%+.1f' % (tree / MIB) + 'M' if tree is not None else '—'} | "
              f"{'%.1f%%' % (100 * tree / m0) if tree is not None else '—'} | "
              f"{(m0 - floor) / MIB / d['pages']:.3f} | {(fl - m0) / MIB / d['pages']:.2f} |")
    print(f"\nMedians of three runs; the widest spread of any three is {mib(widest)} MiB ({widest_at}). "
          f"Process floor = `{floor_doc}` at `--max-pages 0`, median {mib(floor)} MiB "
          f"(the control, outside the corpus, peaks at {mib(statistics.median(per[CONTROL]['max0']))} MiB).\n")

    # ---- 3. budget points against the chord
    print("### Budget points against the chord from max0 to the full extract\n")
    print("| fixture | budget | peak RSS | cumulative MiB per admitted page | chord (full − max0)/pages | ratio |")
    print("| --- | ---: | ---: | ---: | ---: | ---: |")
    worst_a, worst_a_at = 0.0, ""
    for n in by_pages:
        d = per[n]
        m0 = statistics.median(d["max0"])
        chord = (max(d["full"] + d["full_pipe"]) - m0) / MIB / d["pages"]
        for b, rss in sorted(d["budget"].items()):
            admitted = min(b, d["pages"])
            cum = (rss - m0) / MIB / admitted
            print(f"| {n} | {b} | {mib(rss)}M | {cum:.2f} | {chord:.2f} | {cum / chord:.2f}x |")
            if cum > worst_a:
                worst_a, worst_a_at = cum, f"{n} at --max-pages {b}"
        if chord > worst_a:
            worst_a, worst_a_at = chord, f"{n}, every page admitted"
    print()

    # ---- 4. the rule
    worst_b, worst_b_at = 0.0, ""
    for n in by_pages:
        d = per[n]
        m0 = statistics.median(d["max0"])
        coef = (m0 - floor) / MIB / d["pages"]
        if coef > worst_b:
            worst_b, worst_b_at = coef, n
    F = ceil_to(floor / MIB, 1.0)
    a = ceil_to(worst_a, 0.1)
    b = ceil_to(worst_b, 0.01)
    print("### The rule\n")
    print(f"Worst observed: floor {mib(floor)} MiB ({floor_doc}); per admitted page {worst_a:.2f} MiB ({worst_a_at}); "
          f"per document page {worst_b:.3f} MiB ({worst_b_at}).")
    print(f"\n**peak ≤ {F:.0f} MiB + {a:.1f} MiB × admitted pages + {b:.2f} MiB × pages in the document**\n")
    print("| fixture | admitted | measured (worst sample) | new rule | over by | old rule (7 + 0.35, no floor) | over by |")
    print("| --- | ---: | ---: | ---: | ---: | ---: | ---: |")
    points = []
    for n in by_pages:
        d = per[n]
        points.append((n, d["pages"], max(d["full"] + d["full_pipe"])))
        for bud, rss in sorted(d["budget"].items()):
            points.append((n, min(bud, d["pages"]), rss))
    tightest = None
    for n, admitted, measured in points:
        P = per[n]["pages"]
        new = F + a * admitted + b * P
        old = OLD_RULE[0] * admitted + OLD_RULE[1] * P
        over_new = new / (measured / MIB) - 1
        over_old = old / (measured / MIB) - 1
        label = f"{n} ({admitted} of {P})" if admitted != P else n
        print(f"| {label} | {admitted} | {mib(measured)}M | {new:.0f}M | {over_new:+.1%} | {old:.0f}M | {over_old:+.1%} |")
        if tightest is None or over_new < tightest[1]:
            tightest = (label, over_new)
    print(f"\nTightest point: {tightest[0]}, over by {tightest[1]:+.1%}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
