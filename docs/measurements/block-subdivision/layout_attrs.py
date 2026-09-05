#!/usr/bin/env python3
"""Probe 2 of `docs/19-BLOCK-SUBDIVISION-SCOPE.md` §7: do documents declare their own spacing?

# The question, and why it is D1-shaped

PDF 32000-1 §14.8.5 gives structure elements LAYOUT attributes under the `/Layout` owner:
`/SpaceBefore`, `/SpaceAfter`, `/StartIndent`, `/EndIndent`, `/TextIndent`. Those are the document
STATING its own paragraph spacing and indentation, where §4 of `19` infers them from a gap
distribution. A declared value beats an inferred threshold on this repository's own grounds.

So the probe is decisive either way, exactly as `/Collection` was for D1:

  * They occur -> declared beats measured, and the rule shape in `19` §7 is wrong: it should read
    the attribute where present and measure only where absent.
  * They are 0 -> that branch is cleanly refused on evidence, and the case for the measured cut is
    STRENGTHENED rather than weakened, because the alternative was tried and found absent.

# Two ways this probe could produce a false zero, and what is done about each

1. **Object streams.** Structure elements almost always live in compressed object streams, so a
   grep over the raw file finds nothing whatever the document contains. `17-D1-SCOPE.md` counted
   "raw object occurrences" for catalog-level keys, where that is safe; it is NOT safe here. Every
   file is expanded with `qpdf --qdf --object-streams=disable` first.

2. **The ClassMap.** An element may carry attributes directly in `/A`, or name a CLASS in `/C` whose
   attributes live in `/ClassMap` on the structure tree root. A probe that reads only `/A` reports
   zero on any producer that uses classes — which is most of them. Both are counted, separately.

A POSITIVE CONTROL runs alongside: the count of `/O /Layout` owner markers and of `/ClassMap`
dictionaries. If the attributes are zero AND the owner marker is zero AND no ClassMap exists, the
finding is "these documents do not declare layout"; if the owner marker is present while the
attributes are zero, the finding is "they declare layout and not spacing", which is different. A
zero with no control is not a measurement.
"""
import re
import subprocess
import sys
from pathlib import Path

ATTRS = ["SpaceBefore", "SpaceAfter", "StartIndent", "EndIndent", "TextIndent"]
CONTROLS = ["/O /Layout", "/ClassMap", "/StructTreeRoot", "/A ", "/C "]


def expand(pdf: Path) -> str:
    r = subprocess.run(["qpdf", "--qdf", "--object-streams=disable", str(pdf), "-"],
                       capture_output=True)
    if r.returncode not in (0, 3):      # 3 is qpdf's "warnings but recovered"
        return ""
    return r.stdout.decode("latin-1", "replace")


def classmap_attrs(text: str):
    """Attribute keys defined inside a /ClassMap, which /A-only probes miss entirely."""
    m = re.search(r"/ClassMap\s+(\d+) 0 R", text)
    body = None
    if m:
        o = re.search(rf"^{m.group(1)} 0 obj(.*?)endobj", text, re.S | re.M)
        body = o.group(1) if o else None
    else:
        m2 = re.search(r"/ClassMap\s*<<(.*?)>>\s*/", text, re.S)
        body = m2.group(1) if m2 else None
    if not body:
        return {}, False
    return {a: len(re.findall(rf"/{a}\b", body)) for a in ATTRS}, True


def main(paths):
    tot = {a: 0 for a in ATTRS}
    tot_cm = {a: 0 for a in ATTRS}
    ctl = {c: 0 for c in CONTROLS}
    rows, unreadable = [], []
    for p in paths:
        text = expand(Path(p))
        if not text:
            unreadable.append(Path(p).name)
            continue
        counts = {a: len(re.findall(rf"/{a}\b", text)) for a in ATTRS}
        cm, has_cm = classmap_attrs(text)
        for a in ATTRS:
            tot[a] += counts[a]
            tot_cm[a] += cm.get(a, 0)
        for c in CONTROLS:
            ctl[c] += text.count(c)
        if any(counts.values()) or has_cm:
            rows.append((Path(p).name, counts, cm, has_cm))

    print(f"documents scanned: {len(paths) - len(unreadable)}"
          + (f"   unreadable: {unreadable}" if unreadable else ""))
    print("\n=== Layout attribute occurrences, ALL documents, after decompression ===")
    for a in ATTRS:
        print(f"  /{a:<12} {tot[a]:>6}     (of which inside a /ClassMap: {tot_cm[a]})")
    print("\n=== positive control — is the probe capable of finding anything? ===")
    for c in CONTROLS:
        print(f"  {c!r:<20} {ctl[c]:>6}")
    if rows:
        print("\n=== documents with any occurrence or a ClassMap ===")
        for name, counts, cm, has_cm in rows:
            hits = ", ".join(f"{a}={counts[a]}" for a in ATTRS if counts[a])
            print(f"  {name:<34} {hits or '(none directly)':<40} ClassMap={has_cm}")


if __name__ == "__main__":
    main(sys.argv[1:])
