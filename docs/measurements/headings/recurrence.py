#!/usr/bin/env python3
"""PI-C — would a cross-page recurrence gate remove real headings?

    python3 docs/measurements/headings/recurrence.py [WORKDIR]

`docs/06-STEAL-REFUSE.md` row **PI-C**. The proposal is a seventh clause on the inferred-heading
rule: *a line whose normalized text appears on N or more distinct pages is not an inferred heading*
— text identity across pages and nothing else, because row 29's rider says the rule reads type and
never position, and `headings.rs`'s `the_rule_reads_no_region_and_no_block` enforces it.

The clause is **subtractive**: it can only withhold a heading the shipped rule already emitted, so
it cannot fabricate one. That makes §7.5's bar 1 (false-positive rate) safe by construction and
**bar 2 the whole question**: *"MHS rises, and falls on no document"*. A subtractive clause raises
MHS only by removing more false headings than true ones, and "falls on no document" means it must
remove **zero** declared headings anywhere. A genuinely recurrent real heading — a part title
repeated as a running head, a repeated `Appendix` — breaches it.

So this measures exactly that, before any code is written, the way §7.5 requires.

# What it reuses, and why

Everything expensive: `falsepos.strip_tree`, `falsepos.extract` and `falsepos.DOCUMENTS`. That
instrument already measures each document twice from one file — the original, whose `pdf_tagged`
locators carry the author's labels, and the same file with `/StructTreeRoot` removed, on which the
gate opens and the shipped build writes its verdict — and joins them node by node. This asks a
different question of the same join, so it imports rather than re-derives. Its guards (equal node
counts, no leaked flag on the original, the stripped copy really reads as untagged) run unchanged.

# What a "withheld" line is

A line the shipped rule fired on whose normalized text also appears on at least N-1 other pages.
Normalization here is deliberately crude and stated rather than hidden: casefold, collapse runs of
whitespace, strip leading and trailing whitespace. **Digits are NOT folded**, so `Chapter 3` and
`Chapter 4` are different strings and a numbered heading is not withheld by its siblings. Folding
digits is the obvious next variant and is reported separately so the cost of each is visible.

A real clause would need its own versioned rule id for whichever normalization ships, on
`locate-scalar-exact-v1`'s precedent — a flag would make two answers under one id.
"""

import collections
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import falsepos  # noqa: E402  — sibling instrument, reused rather than re-derived

WS = re.compile(r"\s+")
DIGITS = re.compile(r"\d+")


def norm(text, fold_digits=False):
    out = WS.sub(" ", text).strip().casefold()
    return DIGITS.sub("#", out) if fold_digits else out


def lines_of(pdf, work):
    """One record per non-artifact, non-blank line: page, text, declared-heading, did-fire."""
    stripped = work / "stripped.pdf"
    falsepos.strip_tree(pdf, stripped, work)
    original, _ = falsepos.extract(pdf, work, "original")
    verdicts, s_codes = falsepos.extract(stripped, work, "stripped")
    stripped.unlink()

    # falsepos.measure's guards, unchanged: the join is by position and means nothing otherwise.
    if len(original) != len(verdicts):
        raise SystemExit(f"{pdf.name}: {len(original)} nodes against {len(verdicts)} once stripped")
    if any(o and o["flag"] for o in original):
        raise SystemExit(f"{pdf.name}: the tagged original carries an inferred heading; the gate leaked")
    if "untagged-structure-tree-absent" not in s_codes:
        raise SystemExit(f"{pdf.name}: the stripped copy still reads as tagged")

    grouped = {}
    for o, v in zip(original, verdicts):
        if o is None:
            continue
        grouped.setdefault(v["key"], []).append((o, v))

    out = []
    for key, members in grouped.items():
        if key[2] or all(not o["text"].strip() for o, _ in members):
            continue  # an /Artifact line, or whitespace only
        roles = {o["role"] for o, _ in members if o["role"] is not None}
        out.append({
            "page": key[0],
            "text": "".join(o["text"] for o, _ in members),
            "declared": bool(roles & falsepos.HEADING_ROLES),
            "fired": any(v["flag"] for _, v in members),
        })
    return out


def sweep(lines, fold_digits):
    """For each N, what the clause would withhold, split by whether the author declared it."""
    pages = collections.defaultdict(set)
    for ln in lines:
        pages[norm(ln["text"], fold_digits)].add(ln["page"])
    rows = {}
    fired = [ln for ln in lines if ln["fired"]]
    for n in (2, 3, 4, 5, 10):
        withheld = [ln for ln in fired if len(pages[norm(ln["text"], fold_digits)]) >= n]
        rows[n] = {
            "withheld": len(withheld),
            "withheld_true": sum(1 for ln in withheld if ln["declared"]),
            "withheld_false": sum(1 for ln in withheld if not ln["declared"]),
        }
    return rows, len(fired), sum(1 for ln in fired if ln["declared"])


def main():
    work = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/headings-recurrence")
    work.mkdir(parents=True, exist_ok=True)
    print(f"engine: {falsepos.BINARY}\n")
    report = []
    for pdf in falsepos.DOCUMENTS:
        if not pdf.is_file():
            raise SystemExit(f"missing: {pdf}. A missing document is a failure, never a skip.")
        lines = lines_of(pdf, work)
        plain, fired, fired_true = sweep(lines, False)
        folded, _, _ = sweep(lines, True)
        report.append({
            "document": pdf.stem, "lines": len(lines),
            "fired": fired, "fired_declared": fired_true,
            "plain": plain, "digits_folded": folded,
        })
        print(f"{pdf.stem:24} lines {len(lines):>6}  fired {fired:>5} (declared {fired_true:>4})  "
              + "  ".join(f"N={n}:{plain[n]['withheld']:>4}/{plain[n]['withheld_true']:>3}T" for n in (2, 3, 5)),
              flush=True)

    (work / "recurrence.json").write_text(json.dumps(report, indent=1) + "\n")
    print("\n`N=w/tT` is: w lines withheld, of which t were DECLARED headings by the author.")
    print("A single declared heading withheld anywhere is a bar-2 breach: MHS falls on that document.\n")
    for n in (2, 3, 4, 5, 10):
        tot = sum(r["plain"][n]["withheld"] for r in report)
        true = sum(r["plain"][n]["withheld_true"] for r in report)
        docs = sum(1 for r in report if r["plain"][n]["withheld_true"] > 0)
        ftot = sum(r["digits_folded"][n]["withheld"] for r in report)
        ftrue = sum(r["digits_folded"][n]["withheld_true"] for r in report)
        print(f"  N={n:<3} plain: withheld {tot:>5}, of which declared {true:>4} "
              f"on {docs} document(s)   |   digits folded: {ftot:>5} / {ftrue:>4} declared")
    print(f"\nwrote {work / 'recurrence.json'}")


if __name__ == "__main__":
    main()
