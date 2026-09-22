#!/usr/bin/env python3
"""Size the /Form XObject descent prize, without building anything.

For each PDF: run `extract`, read the document-scoped limitation
`form-xobjects-not-descended` (emitted only when the count is non-zero), and
record the count. Nothing is written to disk but this script's own output.
"""
import json
import subprocess
import sys
from pathlib import Path

BIN = sys.argv[1]
roots = [Path(p) for p in sys.argv[2:]]

pdfs = []
for r in roots:
    pdfs.extend(sorted(r.glob("*.pdf")) if r.is_dir() else [r])

rows, failures = [], []
for p in pdfs:
    try:
        out = subprocess.run([BIN, "extract", str(p)], capture_output=True, timeout=300)
    except subprocess.TimeoutExpired:
        failures.append((p.name, "timeout"))
        continue
    if out.returncode != 0:
        failures.append((p.name, f"exit {out.returncode}: {out.stderr.decode()[:120].strip()}"))
        continue
    try:
        art = json.loads(out.stdout)
    except json.JSONDecodeError as e:
        failures.append((p.name, f"unparsable: {e}"))
        continue
    lims = art.get("representation", {}).get("assurance", {}).get("limitations", [])
    hit = [l for l in lims if l.get("code") == "form-xobjects-not-descended"]
    nodes = len(art.get("representation", {}).get("nodes", []))
    text_nodes = sum(1 for n in art.get("representation", {}).get("nodes", []) if n.get("kind") == "text_run")
    rows.append({
        "doc": p.name,
        "count": hit[0].get("count") if hit and "count" in hit[0] else (len(hit) and -1),
        "detail": hit[0].get("detail", "")[:160] if hit else "",
        "nodes": nodes,
        "text_nodes": text_nodes,
        "pages": len(art.get("representation", {}).get("pages", [])),
    })

with_forms = [r for r in rows if r["detail"]]
print(f"documents read      : {len(rows)}")
print(f"documents failed    : {len(failures)}")
for n, why in failures[:10]:
    print(f"   ! {n}: {why}")
print(f"declaring the code  : {len(with_forms)}"
      f"  ({100*len(with_forms)/max(len(rows),1):.1f}% of those read)")
print()
zero_text = [r for r in with_forms if r["text_nodes"] == 0]
print(f"of those, emitting ZERO text nodes: {len(zero_text)}")
for r in zero_text[:15]:
    print(f"   {r['doc']}  pages={r['pages']} nodes={r['nodes']}")
print()
print("the ten declaring documents with the fewest text nodes:")
for r in sorted(with_forms, key=lambda r: r["text_nodes"])[:10]:
    print(f"   {r['doc']:34} text_nodes={r['text_nodes']:6} pages={r['pages']:4}")
print()
if with_forms:
    print("one detail string, verbatim:")
    print("   " + with_forms[0]["detail"])
json.dump({"rows": rows, "failures": failures}, open("/tmp/xobj_census.json", "w"), indent=1)
print("\nwrote /tmp/xobj_census.json")
