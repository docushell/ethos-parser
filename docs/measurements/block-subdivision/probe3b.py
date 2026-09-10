#!/usr/bin/env python3
"""Probe 3, TEST B — reconstructed, because its instrument was never committed.

# Why this file exists

[`19-BLOCK-SUBDIVISION-SCOPE.md`](../../19-BLOCK-SUBDIVISION-SCOPE.md) §11.2 publishes the numbers
that decide the whole block-cut rule:

    rule              recall   false-fire   precision
    fixed 1.15x       66.1%      1.1%         91.3%
    fixed 1.60x       63.0%      0.0%        100.0%
    adaptive          46.5%      1.1%         88.1%

§11.4 then chooses fixed over adaptive on them, and that choice is what the implementation plan
carries. **No committed instrument produces them.** `probe3.py` implements Test A only;
`structelem.py` answers the one-`/P`-per-line gate question and stops. So the most load-bearing
number in the document is, as of 2026-09-10, unreproducible — the exact failure the file's own
header sets out to prevent: *"a question was asked, an instrument was built, and what it found is
written down before anything is built on it."*

This reconstructs it, so the number can be re-derived rather than believed, and re-derived on a
tree eight releases newer than the one that produced it.

# Method

Real paragraph-to-paragraph labels, which §9.1 established exist on exactly one gate document
(`nist-sp-800-207`, 3.63 lines per `/P`).

1. `qpdf --qdf --object-streams=disable` exposes the structure tree; every `/S /P` element names
   its page and its MCIDs (`structelem.p_elements`).
2. The engine's own artifact maps `(page, mcid)` to baselines, so a line is joined to the `/P`
   element that owns it.
3. Within a `(page, region)` band, consecutive lines whose owning `/P` elements DIFFER are a real
   P->P boundary. Consecutive lines in the SAME element are mid-paragraph pairs.
4. Gaps are expressed against the band's own modal leading, binned to 10 centipoints, per §2.
5. The §5 plausibility guard excludes bands that are not text flows: modal gap >= 600 centipoints
   and mode share >= 25%.

# What it deliberately excludes, and the cost

**A line inside an inline child is dropped, not relabelled.** A `/P` whose `/K` array holds
`N 0 R` references has those MCIDs owned by the child (`Link`, `Reference`), so such a line joins
no `/P` and is excluded from both populations. That is conservative in the right direction — §3's
error 2 was counting `P -> Link` as a boundary, and 69 of 149 apparent boundaries were exactly
that — but it also shrinks the label set on link-heavy prose, so the counts here are a floor.

    docs/measurements/block-subdivision/probe3b.py <artifact.json> <source.pdf>
"""

import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from labelled import SAME_LINE_TOL, leading_of  # noqa: E402
from structelem import objects, page_index, p_elements  # noqa: E402

BIN = 10
MIN_MODAL_GAP = 600   # §5 plausibility guard
MIN_SHARE = 0.25


def owner_by_mcid(pdf):
    """`(page, mcid) -> element ordinal` for every `/S /P` element."""
    qdf = subprocess.run(
        ["qpdf", "--qdf", "--object-streams=disable", str(pdf), "-"],
        capture_output=True, check=True,
    ).stdout
    objs = objects(qdf)
    pidx = page_index(objs)
    out = {}
    for ordinal, (page, mcids) in enumerate(p_elements(objs, pidx)):
        for m in mcids:
            out[(page, m)] = ordinal
    return out


def lines_with_owner(artifact, owners):
    """`(page, region) -> [(y, owner_ordinal_or_None)]`, body lines only, line numbers dropped."""
    acc = defaultdict(lambda: defaultdict(list))
    for n in artifact["representation"]["nodes"]:
        if n["kind"] != "text_run" or not n.get("text", "").strip():
            continue
        loc = (n.get("native_locator") or {}).get("pdf")
        sl = n.get("structural_locator") or {}
        if not loc or "pdf_artifact" in sl:
            continue
        t = sl.get("pdf_tagged") or {}
        mcid = t.get("mcid")
        owner = owners.get((loc["page"], mcid)) if mcid is not None else None
        attrs = (n.get("attributes") or {}).get("text_run") or {}
        acc[(loc["page"], attrs.get("region"))][loc["origin_y"]].append((owner, n["text"]))

    out = {}
    for key, by_y in acc.items():
        merged = {}
        for y in sorted(by_y):
            hit = next((m for m in merged if abs(m - y) <= SAME_LINE_TOL), None)
            merged.setdefault(hit if hit is not None else y, []).extend(by_y[y])
        rows = []
        for y, parts in sorted(merged.items()):
            if "".join(p[1] for p in parts).strip().isdigit():
                continue                      # a margin line number, not a body line
            owns = [p[0] for p in parts if p[0] is not None]
            rows.append((y, Counter(owns).most_common(1)[0][0] if owns else None))
        if len(rows) >= 6:
            out[key] = rows
    return out


def band_second_mode(gaps, leading):
    """The adaptive cut: midway beneath the band's own second mode, or None."""
    binned = Counter(round(g / BIN) * BIN for g in gaps)
    need = max(2, int(0.05 * len(gaps)))
    cands = [b for b, c in binned.items() if b > 1.15 * leading and c >= need]
    return (leading + max(cands)) / 2 if cands else None


def collect(artifact_path, pdf_path):
    art = json.load(open(artifact_path))
    owners = owner_by_mcid(pdf_path)
    pairs, skipped_bands = [], 0

    for _key, rows in lines_with_owner(art, owners).items():
        gaps = [rows[i + 1][0] - rows[i][0] for i in range(len(rows) - 1)]
        if not gaps:
            continue
        leading = leading_of(gaps)
        if leading is None:
            continue
        binned = Counter(round(g / BIN) * BIN for g in gaps)
        if leading < MIN_MODAL_GAP or binned[leading] / len(gaps) < MIN_SHARE:
            skipped_bands += 1          # not a text flow; §5's guard
            continue
        cut = band_second_mode(gaps, leading)
        for i, gap in enumerate(gaps):
            a, b = rows[i][1], rows[i + 1][1]
            if a is None or b is None:
                continue                # inline-owned line: excluded from both populations
            pairs.append((gap, leading, cut, a != b))
    return pairs, skipped_bands


def report(pairs, label, fires):
    """recall over real boundaries, false-fire over mid-paragraph pairs, precision."""
    real = [p for p in pairs if p[3]]
    mid = [p for p in pairs if not p[3]]
    tp = sum(1 for p in real if fires(p))
    fp = sum(1 for p in mid if fires(p))
    recall = tp / len(real) if real else 0.0
    ff = fp / len(mid) if mid else 0.0
    prec = tp / (tp + fp) if (tp + fp) else 0.0
    print(f"  {label:<16}{recall:>9.1%}{ff:>13.1%}{prec:>12.1%}")


def main(argv):
    if len(argv) != 2:
        sys.exit(__doc__.strip().splitlines()[-1].strip())
    artifact, pdf = argv
    pairs, skipped = collect(artifact, pdf)
    real = sum(1 for p in pairs if p[3])
    print(f"\n{Path(pdf).name}: {real} real P->P boundaries, {len(pairs) - real} mid-paragraph "
          f"pairs, {skipped} bands failed the plausibility guard\n")
    if not real:
        sys.exit("no real P->P labels on this document — §9.1 says only nist-sp-800-207 has them")
    print(f"  {'rule':<16}{'recall':>9}{'false-fire':>13}{'precision':>12}")
    for t in (1.15, 1.40, 1.60, 1.80):
        report(pairs, f"fixed {t:.2f}x", lambda p, t=t: p[0] >= t * p[1])
    report(pairs, "adaptive", lambda p: p[2] is not None and p[0] >= p[2])


if __name__ == "__main__":
    main(sys.argv[1:])
