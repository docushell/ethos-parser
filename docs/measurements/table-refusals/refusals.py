#!/usr/bin/env python3
"""T1 of the table work: what the unruled rule refuses, and whether it was right to.

# The question

`docs/table-gate-v1.md` reports fabrication 0 and a geometric macro F1 whose median is 0. The
opendataloader-bench record reports TEDS 0.1038 with 37 of 42 documents at zero. Those are two
views of ONE mechanism: `unruled-table-candidate-refused` fires on 199 of 200 documents and
`ruled-table-candidate-refused` on 57. The engine is not failing to find tables. It builds
candidates and refuses them.

So the scoping question for what comes next is not "can a detector be built" but "what may a
refused candidate emit". That cannot be answered by counting refusals, because a refusal on a page
with no table is the rule working. It needs the refusals CROSSED with ground truth.

# What this measures

For each of the 200 benchmark documents:

  * every refusal the artifact declares, parsed back into the `unruled::Refusal` variant that
    produced it, with its measured parameters;
  * whether the document's ground truth actually contains a `Table` element;
  * whether the engine emitted any table at all.

Then it crosses them. The number that matters is the gutter distribution split by whether the
page really holds a table: if refused-with-a-table and refused-without-a-table are the same
distribution, the floor cannot separate them and no threshold move will help. If they separate,
the floor is in the wrong place and the measurement says where.

# What this does NOT measure

**Not per-page ground truth for the refusal.** A refusal names its page; ground truth names its
page; but a document's tables and its refusals can sit on different pages, and this script does
not currently require them to coincide beyond the page number. Read the per-page cross-tab as
"a table is on this page and a refusal fired on this page", not as "this refusal ate that table".

**Not a score.** No TEDS, no F1, no grade. It reports what was refused and what was there.

**Not a proposal.** What a refused candidate may emit is a decision (plan D1), and this exists to
make that decision answerable rather than to pre-empt it.

# Running it

    cargo build --release --locked
    ETHOS_BENCH_CORPUS=~/ethos-external-benchmarks/opendataloader-bench \\
      docs/measurements/table-refusals/refusals.py

Reproduces bit-identically: the engine is deterministic and this only reads its output.
"""

import collections
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
ENGINE = REPO / "target/release/ethos-parser"
BENCH = Path(
    os.environ.get("ETHOS_BENCH_CORPUS", Path.home() / "ethos-external-benchmarks/opendataloader-bench")
).expanduser()

REFUSAL_CODES = (
    "unruled-table-candidate-refused",
    "ruled-table-candidate-refused",
    "stroke-ruled-table-candidate-refused",
)

# One pattern per `unruled::Refusal` variant, matched against the per-page prose the engine
# writes in `Refusal::detail`. Anchored on the distinctive clause of each so a reworded tail
# does not silently stop matching -- and anything that matches none is counted as `unparsed`
# and printed, because a parser that quietly drops a variant reports a cleaner corpus than
# the one it read.
PATTERNS = [
    ("gutter_below_floor",
     re.compile(r"two adjacent (?P<axis>column|row) lines sat (?P<gap>\d+) centipoints apart, "
                r"under the (?P<floor>\d+)-centipoint floor")),
    ("faces_without_text",
     re.compile(r"the text implied a grid of (?P<faces>\d+) cells but only filled (?P<filled>\d+): "
                r"(?P<empty>\d+) would have been empty")),
    ("lattice_too_large",
     re.compile(r"the text implied (?P<faces>\d+) cells, past the (?P<ceiling>\d+)-cell ceiling")),
    ("emission_order_not_row_major",
     re.compile(r"wrote it DOWN the columns rather than ACROSS the rows")),
]

PAGE_LINE = re.compile(r"^\s*-\s*page (?P<page>\d+):\s*(?P<body>.*)$")


def ground_truth():
    """`{doc: {page: set(categories)}}` from the benchmark's own reference file."""
    ref = json.loads((BENCH / "ground-truth/reference.json").read_text())
    out = {}
    for doc, v in ref.items():
        pages = collections.defaultdict(set)
        for e in v.get("elements", []):
            pages[e.get("page")].add(e.get("category"))
        out[doc] = dict(pages)
    return out


def parse_detail(detail):
    """Per-page refusals as `[(page, variant, params)]`, plus any line that matched nothing."""
    found, unparsed = [], []
    for raw in detail.splitlines():
        m = PAGE_LINE.match(raw)
        if not m:
            continue
        page, body = int(m.group("page")), m.group("body")
        for name, pat in PATTERNS:
            hit = pat.search(body)
            if hit:
                found.append((page, name, hit.groupdict()))
                break
        else:
            unparsed.append(body[:120])
    return found, unparsed


def extract(pdf):
    r = subprocess.run([str(ENGINE), "extract", str(pdf)], capture_output=True)
    if r.returncode != 0:
        return None
    try:
        return json.loads(r.stdout)
    except json.JSONDecodeError:
        return None


def main():
    if not ENGINE.exists():
        sys.exit(f"no engine at {ENGINE}; run `cargo build --release --locked` first")
    if not BENCH.exists():
        sys.exit(f"no corpus at {BENCH}; set ETHOS_BENCH_CORPUS")

    gt = ground_truth()
    print(f"corpus: {BENCH}\ndocuments in ground truth: {len(gt)}\n", flush=True)

    by_variant = collections.Counter()
    variant_by_table = collections.Counter()   # (variant, page_has_table)
    gutters = {True: [], False: []}            # column gaps, split by page-has-table
    unparsed_all = []
    no_artifact = []
    docs_with_gt_table = set()
    docs_emitting_table = set()
    docs_gt_table_no_emit = set()

    for n, (doc, pages) in enumerate(sorted(gt.items()), 1):
        pdf = BENCH / "pdfs" / doc
        if not pdf.exists():
            continue
        art = extract(pdf)
        if art is None:
            no_artifact.append(doc)
            continue
        rep = art["representation"]

        gt_table_pages = {p for p, cats in pages.items() if "Table" in cats}
        if gt_table_pages:
            docs_with_gt_table.add(doc)
        if rep.get("tables"):
            docs_emitting_table.add(doc)
        if gt_table_pages and not rep.get("tables"):
            docs_gt_table_no_emit.add(doc)

        for lim in rep.get("assurance", {}).get("limitations", []):
            if lim.get("code") not in REFUSAL_CODES:
                continue
            found, unparsed = parse_detail(lim.get("detail", ""))
            unparsed_all.extend(unparsed)
            for page, variant, params in found:
                has_table = page in gt_table_pages
                by_variant[variant] += 1
                variant_by_table[(variant, has_table)] += 1
                if variant == "gutter_below_floor" and params.get("axis") == "column":
                    gutters[has_table].append(int(params["gap"]))

        if n % 50 == 0:
            print(f"  ...{n}/{len(gt)}", flush=True)

    print("\n" + "=" * 78)
    print("1. Does a document with a table get a table?")
    print("=" * 78)
    print(f"documents whose ground truth holds >=1 Table : {len(docs_with_gt_table)}")
    print(f"  of those, engine emitted >=1 table         : "
          f"{len(docs_with_gt_table & docs_emitting_table)}")
    print(f"  of those, engine emitted none              : {len(docs_gt_table_no_emit)}")
    print(f"documents where the engine emitted a table   : {len(docs_emitting_table)}")
    if no_artifact:
        print(f"documents producing no artifact              : {len(no_artifact)} {no_artifact[:5]}")

    print("\n" + "=" * 78)
    print("2. Refusals by variant, crossed with whether that page holds a table")
    print("=" * 78)
    print(f"{'variant':<32}{'total':>8}{'page HAS table':>17}{'page has none':>16}")
    for variant, total in by_variant.most_common():
        yes = variant_by_table[(variant, True)]
        no = variant_by_table[(variant, False)]
        print(f"{variant:<32}{total:>8}{yes:>17}{no:>16}")

    print("\n" + "=" * 78)
    print("3. Column gutter that failed the floor, split by whether a table is there")
    print("=" * 78)
    print("The floor is 1200 centipoints (12pt). If these two distributions overlap, the floor")
    print("cannot separate a table from prose and moving it will not help.\n")
    for has_table in (True, False):
        v = sorted(gutters[has_table])
        label = "page HAS a table" if has_table else "page has no table"
        if not v:
            print(f"{label:<20} n=0")
            continue
        q = lambda p: v[min(len(v) - 1, int(len(v) * p))]
        print(f"{label:<20} n={len(v):<6} min={v[0]:<6} p25={q(.25):<6} "
              f"median={q(.5):<6} p75={q(.75):<6} max={v[-1]}")

    if unparsed_all:
        print("\n" + "=" * 78)
        print(f"UNPARSED refusal lines: {len(unparsed_all)} -- the taxonomy above is INCOMPLETE")
        print("=" * 78)
        for u in unparsed_all[:10]:
            print("  ", u)


if __name__ == "__main__":
    main()
