#!/usr/bin/env python3
"""Measurement 3: can a gap threshold separate boundaries the DOCUMENT declares?

# The ground truth this uses, and the ground truth it does not have

`element_id` on a tagged locator is never populated by this engine, and `mcid` is line-like on
this corpus — `mcid_probe.py` measured a median of ONE baseline per mcid on five of six gate
documents. So there is no way to tell two consecutive `/P` elements apart from the wire, and a
paragraph-to-paragraph boundary CANNOT be labelled here.

What CAN be labelled is a boundary where the declared ROLE changes: `P` -> `H1`, `P` -> `LI`,
`H2` -> `P`. Those are boundaries the structure tree states outright. They are a SUBSET of true
block boundaries, and that asymmetry is the whole point of the test:

  * Every ROLE_CHANGE pair is a real boundary.
  * A SAME_ROLE pair is unknown — it is either mid-paragraph continuation or a P->P break.

So the test is one-sided and still decisive in the negative. If the gaps at declared boundaries
are NOT separable from the bulk, then a threshold cannot find even the boundaries the document
spells out, and it certainly cannot find the harder unlabelled ones.

# Line numbers

NIST SP documents number their lines in the margin. Those numerals share the body's region and
land between body baselines, so they were adding spurious sub-leading gaps. A line whose text is
nothing but digits is dropped, and the count of dropped lines is reported rather than assumed
harmless.
"""
import json
import sys
from collections import Counter, defaultdict

SAME_LINE_TOL = 150

# PDF 32000-1 divides structure elements into BLOCK-level (BLSE) and INLINE-level (ILSE). A
# `Link` or a `Reference` is an ILSE: a URL inside flowing prose is a CHILD of its paragraph, not
# a sibling block. Reading the leaf role called every inline link a boundary — 69 of the 149
# "declared boundaries" that showed no extra vertical space were `P -> Link`, which is not a
# boundary at all but a paragraph containing a link. A line's role is therefore its nearest
# BLOCK-level ancestor, and this correction is the difference between measuring block structure
# and measuring inline markup.
INLINE_ROLES = {
    "Span", "Quote", "Note", "Reference", "BibEntry", "Code", "Link", "Annot",
    "Ruby", "RB", "RT", "RP", "Warichu", "WT", "WP", "Lbl", "Em", "Strong",
}


def block_role(path):
    """The nearest block-level role on a role path, or None if the path is empty."""
    for r in reversed(path):
        if r not in INLINE_ROLES:
            return r
    return None



def lines_with_roles(doc):
    """(page, region) -> sorted [(y, role, text)] for body lines, line numbers removed."""
    acc = defaultdict(lambda: defaultdict(list))
    for n in doc["representation"]["nodes"]:
        if n["kind"] != "text_run":
            continue
        loc = (n.get("native_locator") or {}).get("pdf")
        if not loc or not n.get("text", "").strip():
            continue
        sl = n.get("structural_locator") or {}
        if "pdf_artifact" in sl:
            continue
        t = sl.get("pdf_tagged") or {}
        role = block_role(t.get("standard_role_path") or t.get("role_path") or [])
        attrs = (n.get("attributes") or {}).get("text_run") or {}
        acc[(loc["page"], attrs.get("region"))][loc["origin_y"]].append((role, n["text"]))
    out, dropped = {}, 0
    for key, by_y in acc.items():
        merged = {}
        for y in sorted(by_y):
            hit = next((m for m in merged if abs(m - y) <= SAME_LINE_TOL), None)
            merged.setdefault(hit if hit is not None else y, []).extend(by_y[y])
        rows = []
        for y, parts in sorted(merged.items()):
            text = "".join(p[1] for p in parts).strip()
            if text.isdigit():          # a margin line number, not a body line
                dropped += 1
                continue
            role = Counter(p[0] for p in parts).most_common(1)[0][0]
            rows.append((y, role, text))
        if len(rows) >= 6:
            out[key] = rows
    return out, dropped


def leading_of(gaps):
    if not gaps:
        return None
    return Counter(round(g / 10) * 10 for g in gaps).most_common(1)[0][0] or None


def main(paths):
    changed, same, dropped_total = [], [], 0
    for p in paths:
        doc = json.load(open(p))
        bands, dropped = lines_with_roles(doc)
        dropped_total += dropped
        for _key, rows in bands.items():
            gs = [b[0] - a[0] for a, b in zip(rows, rows[1:]) if b[0] > a[0]]
            lead = leading_of(gs)
            if not lead:
                continue
            for a, b in zip(rows, rows[1:]):
                g = b[0] - a[0]
                if g <= 0:
                    continue
                r = g / lead
                if r > 5.0:
                    continue
                (changed if a[1] != b[1] else same).append(r)

    print(f"margin line-number lines dropped: {dropped_total}")
    print(f"ROLE_CHANGE pairs (declared boundaries): {len(changed)}")
    print(f"SAME_ROLE   pairs (unknown)            : {len(same)}\n")

    bins = [(0.0, 0.9), (0.9, 1.1), (1.1, 1.3), (1.3, 1.6), (1.6, 2.0), (2.0, 2.6), (2.6, 5.0)]
    print(f"{'gap (x leading)':<18}{'DECLARED BOUNDARY':>22}{'SAME ROLE':>18}")
    for lo, hi in bins:
        c = sum(1 for r in changed if lo <= r < hi)
        s = sum(1 for r in same if lo <= r < hi)
        cp = 100 * c / len(changed) if changed else 0
        sp = 100 * s / len(same) if same else 0
        print(f"  {lo:4.1f}-{hi:4.1f}x     {c:>8} ({cp:5.1f}%)   {s:>8} ({sp:5.1f}%)")

    # The question a threshold rule has to answer.
    print("\nIf a rule fired on gap > T, over the pairs above:")
    print(f"  {'T':>6} {'boundaries found':>18} {'same-role also fired':>22}")
    for T in (1.15, 1.25, 1.4, 1.6, 1.8, 2.0):
        rec = sum(1 for r in changed if r > T) / len(changed) if changed else 0
        fp = sum(1 for r in same if r > T) / len(same) if same else 0
        print(f"  {T:>6.2f} {rec:>17.1%} {fp:>21.1%}")


if __name__ == "__main__":
    main(sys.argv[1:])
