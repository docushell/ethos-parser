#!/usr/bin/env python3
"""Probe 1 of `docs/19-BLOCK-SUBDIVISION-SCOPE.md` §7: the StructElem sibling probe.

# The gate question

Does a producer emit one `/P` structure element per PARAGRAPH, or one per LINE?

`19` §7 names this the highest-value probe because it decides whether a real paragraph-to-paragraph
label exists on documents this repository already owns. `element_id` is never populated by the
engine and `mcid` is line-like (median one baseline per mcid on five of six gate documents), so the
structure tree itself is the only remaining source.

  * One `/P` per paragraph -> consecutive lines in DIFFERENT `/P` elements are a declared P->P
    boundary, the measurement in `19` can be redone against real labels, and its recall figures
    stop being proxy-bound.
  * One `/P` per line       -> the route dies. Every recall figure in `19` stays proxy-bound with a
    stated confidence ceiling, and no amount of further work on this corpus changes that.

# Method

Read the structure tree from the PDF directly, via `qpdf --qdf --object-streams=disable`, rather
than through the engine. The engine reads the tree but flattens it to a role path, and adding
element identity to the wire to measure whether element identity is useful would be changing the
product to justify changing the product.

Each `/S /P` element names its page (`/Pg`) and its content (`/K`, an MCID integer, an array, or a
marked-content reference dict). The engine's own artifact maps `(page, mcid)` to baselines. Joining
the two gives baselines per `/P` element, which is the answer.
"""
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict

SAME_LINE_TOL = 150

OBJ = re.compile(rb"^(\d+) 0 obj\s*$", re.M)


def objects(qdf: bytes):
    """object number -> raw body text, for every object in a QDF file."""
    out, marks = {}, [(m.start(), int(m.group(1)), m.end()) for m in OBJ.finditer(qdf)]
    for i, (_s, num, end) in enumerate(marks):
        stop = marks[i + 1][0] if i + 1 < len(marks) else len(qdf)
        body = qdf[end:stop]
        cut = body.find(b"endobj")
        out[num] = body[: cut if cut >= 0 else len(body)].decode("latin-1")
    return out


def page_index(objs):
    """page object number -> 1-based page index, from the page tree's own Kids order."""
    kids_of, root = {}, None
    for num, body in objs.items():
        if "/Type /Pages" in body:
            k = re.search(r"/Kids\s*\[(.*?)\]", body, re.S)
            kids_of[num] = [int(x) for x in re.findall(r"(\d+) 0 R", k.group(1))] if k else []
            if "/Parent" not in body:
                root = num
    order, seen = [], set()

    def walk(n):
        if n in seen:
            return
        seen.add(n)
        if n in kids_of:
            for c in kids_of[n]:
                walk(c)
        else:
            order.append(n)

    if root is not None:
        walk(root)
    if not order:  # flat or unusual page tree: fall back to /Type /Page in object order
        order = sorted(n for n, b in objs.items() if "/Type /Page" in b and "/Type /Pages" not in b)
    return {n: i + 1 for i, n in enumerate(order)}


def p_elements(objs, pidx):
    """[(page_index, {mcids})] for every `/S /P` structure element."""
    out = []
    for _num, body in objs.items():
        if not re.search(r"/S\s*/P\b", body):
            continue
        pg = re.search(r"/Pg\s+(\d+) 0 R", body)
        if not pg:
            continue
        page = pidx.get(int(pg.group(1)))
        if page is None:
            continue
        k = re.search(r"/K\s*(\[.*?\]|<<.*?>>|\d+)", body, re.S)
        if not k:
            continue
        raw = k.group(1)
        if raw.startswith("["):
            # integers are MCIDs; `N 0 R` are child elements and are not this element's content
            mcids = {int(x) for x in re.findall(r"(?<![\d ])\b(\d+)\b(?!\s+0 R)", raw)}
        elif raw.startswith("<<"):
            mcids = {int(m) for m in re.findall(r"/MCID\s+(\d+)", raw)}
        else:
            mcids = {int(raw)}
        if mcids:
            out.append((page, mcids))
    return out


def baselines_by_mcid(artifact):
    """(page, mcid) -> set of baselines, from the engine's own artifact."""
    acc = defaultdict(set)
    for n in artifact["representation"]["nodes"]:
        if n["kind"] != "text_run" or not n.get("text", "").strip():
            continue
        loc = (n.get("native_locator") or {}).get("pdf")
        t = (n.get("structural_locator") or {}).get("pdf_tagged")
        if not loc or not t or t.get("mcid") is None:
            continue
        acc[(loc["page"], t["mcid"])].add(loc["origin_y"])
    return acc


def distinct(ys):
    out = []
    for y in sorted(ys):
        if not out or y - out[-1] > SAME_LINE_TOL:
            out.append(y)
    return len(out)


def main(pairs):
    print(f"{'document':<24}{'/P elems':>9}{'joined':>8}{'1 line':>8}{'2+ lines':>10}{'median':>8}")
    print("-" * 67)
    totals = Counter()
    for pdf, art in pairs:
        qdf = subprocess.run(["qpdf", "--qdf", "--object-streams=disable", pdf, "-"],
                             capture_output=True).stdout
        objs = objects(qdf)
        els = p_elements(objs, page_index(objs))
        bym = baselines_by_mcid(json.load(open(art)))
        counts = []
        for page, mcids in els:
            ys = set()
            for m in mcids:
                ys |= bym.get((page, m), set())
            if ys:
                counts.append(distinct(ys))
        name = pdf.split("/")[-1].replace(".pdf", "")
        if not counts:
            print(f"{name:<24}{len(els):>9}{0:>8}{'-':>8}{'-':>10}{'-':>8}")
            continue
        one = sum(1 for c in counts if c == 1)
        multi = sum(1 for c in counts if c > 1)
        med = sorted(counts)[len(counts) // 2]
        totals["one"] += one
        totals["multi"] += multi
        print(f"{name:<24}{len(els):>9}{len(counts):>8}{one:>8}{multi:>10}{med:>8}")
    t = totals["one"] + totals["multi"]
    if t:
        print(f"\n  ONE baseline  : {totals['one']:>6}  ({totals['one']/t:.1%})")
        print(f"  TWO OR MORE   : {totals['multi']:>6}  ({totals['multi']/t:.1%})")
        print("\n  VERDICT: " + ("one `/P` per LINE — the P->P label route dies"
                                 if totals["one"] / t > 0.6 else
                                 "one `/P` per PARAGRAPH — real P->P labels are available"))


if __name__ == "__main__":
    main([(sys.argv[i], sys.argv[i + 1]) for i in range(1, len(sys.argv), 2)])
