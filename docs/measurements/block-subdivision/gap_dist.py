#!/usr/bin/env python3
"""Measurement 1 of the paragraph cut: is the baseline-gap distribution bimodal?

`reading_order.rs` opens a region only on a VERTICAL cut, because within a band `horizontal_cut`
cuts at the widest gap "and every gap tied with it", and body text set with uniform leading has
every baseline gap tied — so its leaves are lines, not paragraphs.

The paragraph cut's premise is that a paragraph break leaves a LARGER vertical gap than the
leading inside a paragraph, so gaps are bimodal and a threshold separates them. That premise is
measurable without labels: if the gaps are unimodal, no threshold exists whatever labels would
have said.

# Two corrections this file carries, because the first version of it was wrong twice

1. **Artifacts were not being filtered.** A `PdfArtifact` locator serializes as
   `{"pdf_artifact": {}}`, and `.get("pdf_artifact")` returns `{}`, which is FALSY — so the skip
   never fired and NIST's vertically-set DOI notice, whose every character has its own baseline,
   was being read as ~50 body lines per page. Key presence, not truthiness.

2. **`font_size` is not the type size.** Every run in `nist-sp-800-207` reports `font_size: 100`,
   because these documents set `Tf /F 1` and carry the scale in the text matrix. Dividing by it
   divided by a constant. Gaps are therefore normalized by each BAND'S OWN modal gap — its
   leading — which is scale-free, document-independent, and the only scale a real rule could use,
   since the rule cannot see the point size either.
"""
import json
import sys
from collections import Counter, defaultdict

SAME_LINE_TOL = 150  # centipoints. Absorbs sub/superscript shift; leading is an order larger.


def body_lines(doc):
    """(page, region) -> sorted [(baseline_y, x_min, x_max, text)] for its distinct body lines."""
    acc = defaultdict(lambda: defaultdict(list))
    for n in doc["representation"]["nodes"]:
        if n["kind"] != "text_run":
            continue
        loc = (n.get("native_locator") or {}).get("pdf")
        if not loc or not n.get("text", "").strip():
            continue
        sl = n.get("structural_locator") or {}
        # Page furniture the DOCUMENT marks, not this script's judgement. Correction 1 above.
        if "pdf_artifact" in sl:
            continue
        attrs = (n.get("attributes") or {}).get("text_run") or {}
        acc[(loc["page"], attrs.get("region"))][loc["origin_y"]].append(
            (loc["origin_x"], loc.get("advance") or 0, n["text"])
        )
    out = {}
    for key, by_y in acc.items():
        merged = {}
        for y in sorted(by_y):
            hit = next((m for m in merged if abs(m - y) <= SAME_LINE_TOL), None)
            merged.setdefault(hit if hit is not None else y, []).extend(by_y[y])
        rows = []
        for y, parts in sorted(merged.items()):
            xs = [p[0] for p in parts]
            rows.append((y, min(xs), max(x + a for x, a, _ in parts),
                         "".join(t for _, _, t in sorted(parts))))
        out[key] = rows
    return out


def band_gaps(rows):
    """Consecutive baseline gaps within one band, in centipoints."""
    return [b[0] - a[0] for a, b in zip(rows, rows[1:]) if b[0] > a[0]]


def leading_of(gaps):
    """The band's own leading: the modal gap, to the nearest 10 centipoints.

    Modal rather than median, because a band that is half paragraph breaks would drag a median
    upward while the mode stays on the leading that actually repeats.
    """
    if not gaps:
        return None
    c = Counter(round(g / 10) * 10 for g in gaps)
    return c.most_common(1)[0][0] or None


def main(paths):
    allr = []
    per_doc = {}
    for p in paths:
        doc = json.load(open(p))
        ratios = []
        for _key, rows in body_lines(doc).items():
            gs = band_gaps(rows)
            lead = leading_of(gs)
            if not lead or len(gs) < 5:
                continue
            for g in gs:
                r = g / lead
                if 0 < r <= 5.0:  # beyond 5x leading is a section/figure break, not a paragraph
                    ratios.append(r)
        name = p.split("/")[-1].replace(".json", "")
        per_doc[name] = ratios
        allr += ratios

    for name, v in per_doc.items():
        if not v:
            print(f"{name:<24} no bands")
            continue
        s = sorted(v)
        near1 = sum(1 for r in v if 0.9 <= r <= 1.1) / len(v)
        print(f"{name:<24} n={len(v):>6}  median={s[len(s)//2]:.2f}x  "
              f"p90={s[9*len(s)//10]:.2f}x  within +-10% of leading: {near1:.1%}")

    s = sorted(allr)
    print(f"\n=== ALL  n={len(s)} (gap as a multiple of that band's own leading) ===")
    bins = [(0.0, 0.9), (0.9, 1.1), (1.1, 1.3), (1.3, 1.5), (1.5, 1.8), (1.8, 2.2),
            (2.2, 2.6), (2.6, 3.2), (3.2, 4.0), (4.0, 5.0)]
    for lo, hi in bins:
        c = sum(1 for r in s if lo <= r < hi)
        if c:
            print(f"  {lo:4.1f}-{hi:4.1f}x {c:>7} {'#' * max(1, round(60 * c / len(s)))}")
    print()
    for q in (50, 75, 90, 95, 99):
        print(f"  p{q:<3} {s[min(len(s)-1, q*len(s)//100)]:.2f}x")


if __name__ == "__main__":
    main(sys.argv[1:])
