#!/usr/bin/env python3
"""C1 S3 — how often the shipped heading rule fires where the author's tree says "not a heading".

    python3 docs/measurements/headings/falsepos.py [WORKDIR]

`docs/28-HEADINGS-SCOPE.md` §7.3 and §10 S3. Stdlib only, plus two binaries: this repository's
release `ethos-parser` (`ETHOS_PARSER` overrides) and `qpdf` (Apache-2.0) on PATH. Decision #14
keeps AGPL out of the tree, instruments included; qpdf is not.

# Why the tree is stripped

The rule's inputs — the rendered em, the line, the `/Artifact` marks, the tables — do not depend on
the structure tree, so its verdict on a tagged document is fixed by the untouched original (§7.3).
But the shipped gate closes on a document that declares author structure, so the verdict is never
on that original's wire. So each document is measured twice from one file: **the original**, whose
`pdf_tagged` locators carry the author's labels, and **the same file with `/StructTreeRoot` removed
from its catalog**, on which the gate opens and the shipped build writes its verdict. qpdf makes the
copy (a JSON round trip of the catalog alone, which works inside object streams), and the two
extracts are joined node by node after checking they hold the same runs in the same order.

# What a false positive is

§7.3's definition, unchanged. A line is the runs sharing one page, band, `/Artifact` state and
baseline — the rule's own unit. Whitespace-only and `/Artifact` lines are excluded first. A line is
**labelled** when a run of it carries a declared block-level role: the innermost role of its
`pdf_tagged` locator (`standard_role_path` after the `/RoleMap`) walking up past inline-level
elements — `paragraphs.py`'s `block_role`, and its role set. A **false positive** is a line the rule
fired on whose declared role is not `H` or `H1`..`H6`, over the labelled lines. Recall is reported
beside it and is **not** what the bound is set against (§7.3: these producers write structure
differently, and one sets 180 declared headings at body type).
"""

import json
import os
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[3]
BINARY = os.environ.get("ETHOS_PARSER") or str(REPO / "target" / "release" / "ethos-parser")
GATE = pathlib.Path(os.environ.get("ETHOS_GATE_CORPUS") or REPO / "fixtures" / "gate")
GATE_ZERO = pathlib.Path(
    os.environ.get("ETHOS_BENCH_CORPUS")
    or REPO.parent / "ethos-oracle" / "benchmarks" / "gate-zero" / "corpus"
)

# §7.1.1: the eight gate PDFs, and the three gate-zero documents that declare a heading role.
DOCUMENTS = [GATE / f"{n}.pdf" for n in (
    "irs-f1040sd-2025", "irs-fw9", "nist-sp-800-218", "nist-sp-800-207",
    "nist-sp-800-171r3", "nist-sp-800-37r2", "nist-sp-800-161r1", "nist-sp-800-53Ar5",
)] + [GATE_ZERO / f"{n}.pdf" for n in (
    "nist-sp-800-53r5", "cfpb-home-loan-toolkit", "irs-form-1040-2025",
)]

# §7.1.1 and §10 S3: denominators too small to carry a rate. Reported as counts, never excluded.
COUNTS_ONLY = {"irs-f1040sd-2025", "nist-sp-800-218"}
BOUND_PERCENT = 5

HEADING_ROLES = {"H", "H1", "H2", "H3", "H4", "H5", "H6"}
# PDF 32000-1 §14.8.4.4 Table 338, the inline-level structure elements, plus Em and Strong from
# ISO 32000-2 — `docs/measurements/auto-tagging/paragraphs.py`'s set, copied rather than imported
# because every instrument here stands alone.
INLINE_ROLES = {
    "Span", "Quote", "Note", "Reference", "BibEntry", "Code", "Link", "Annot",
    "Ruby", "RB", "RT", "RP", "Warichu", "WT", "WP", "Em", "Strong",
}


def block_role(path):
    """The nearest block-level role on a role path, or None — `paragraphs.py`'s `block_role`."""
    for role in reversed(path):
        if role not in INLINE_ROLES:
            return role
    return None


def strip_tree(src, dst, work):
    """`src` with `/StructTreeRoot` removed from its catalog, written to `dst`, checked by qpdf."""
    def qjson(*args):
        r = subprocess.run(["qpdf", "--json-output=2", "--json-key=qpdf", *args, str(src)],
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit(f"qpdf could not read {src}: {r.stderr[:300]}")
        return json.loads(r.stdout)["qpdf"]

    header, objects = qjson("--json-object=trailer")
    root = objects["trailer"]["value"]["/Root"]
    catalog = qjson(f"--json-object={root.split()[0]}")[1][f"obj:{root}"]["value"]
    if "/StructTreeRoot" not in catalog:
        raise SystemExit(f"{src.name}: the catalog declares no /StructTreeRoot to strip")
    catalog = {k: v for k, v in catalog.items() if k != "/StructTreeRoot"}
    patch = work / "patch.json"
    patch.write_text(json.dumps({"qpdf": [header, {f"obj:{root}": {"value": catalog}}]}))
    for cmd in (["qpdf", str(src), str(dst), f"--update-from-json={patch}"], ["qpdf", "--check", str(dst)]):
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit(f"{' '.join(cmd[:2])} failed on {src.name}: {r.stderr[:300]}")


def extract(pdf, work, tag):
    """The representation's text-run nodes, reduced to what this measurement reads."""
    out = work / f"{tag}.json"
    with open(out, "wb") as fh:
        r = subprocess.run([BINARY, "extract", str(pdf)], stdout=fh, stderr=subprocess.PIPE)
    if r.returncode != 0:
        raise SystemExit(f"extract {pdf.name} ({tag}) exited {r.returncode}: {r.stderr[:300]!r}")
    rep = json.loads(out.read_bytes())["representation"]
    out.unlink()
    codes = {l["code"] for l in rep["assurance"]["limitations"]}
    rows = []
    for node in rep["nodes"]:
        attrs = node["attributes"].get("text_run")
        if attrs is None:
            rows.append(None)
            continue
        loc = node["native_locator"]["pdf"]
        sl = node.get("structural_locator") or {}
        role = None
        if "pdf_tagged" in sl:
            t = sl["pdf_tagged"]
            role = block_role(t.get("standard_role_path") or t["role_path"])
        rows.append({
            "text": node["text"],
            "key": (loc["page"], attrs.get("region"), "pdf_artifact" in sl, loc["origin_y"]),
            "role": role,
            "mcid": (sl.get("pdf_tagged") or {}).get("mcid"),
            "flag": bool(attrs.get("inferred_heading")),
        })
    return rows, codes


def measure(pdf, work):
    stripped = work / "stripped.pdf"
    strip_tree(pdf, stripped, work)
    original, o_codes = extract(pdf, work, "original")
    verdicts, s_codes = extract(stripped, work, "stripped")
    stripped.unlink()

    # The join is by position, so it must be the same runs in the same order, or it measures
    # nothing. The tree changes locators and nothing about the runs.
    if len(original) != len(verdicts):
        raise SystemExit(f"{pdf.name}: {len(original)} nodes against {len(verdicts)} once stripped")
    for i, (o, v) in enumerate(zip(original, verdicts)):
        if (o is None) != (v is None) or (o and (o["text"] != v["text"] or o["key"][0] != v["key"][0])):
            raise SystemExit(f"{pdf.name}: node {i} differs between the original and the stripped copy")
    if any(o and o["flag"] for o in original):
        raise SystemExit(f"{pdf.name}: the tagged original carries an inferred heading; the gate leaked")
    if "untagged-structure-tree-absent" not in s_codes:
        raise SystemExit(f"{pdf.name}: the stripped copy still reads as tagged")

    lines = {}
    for o, v in zip(original, verdicts):
        if o is None:
            continue
        lines.setdefault(v["key"], []).append((o, v))
    labelled = declared = fired = tp = fp = fired_unlabelled = 0
    items = set()
    for key, members in lines.items():
        if key[2] or all(not o["text"].strip() for o, _ in members):
            continue  # an /Artifact line, or whitespace only
        roles = {o["role"] for o, _ in members if o["role"] is not None}
        is_heading = bool(roles & HEADING_ROLES)
        for o, _ in members:
            if o["role"] in HEADING_ROLES:
                items.add((key[0], o["mcid"]))
        did_fire = any(v["flag"] for _, v in members)
        fired += did_fire
        if not roles:
            fired_unlabelled += did_fire
            continue
        labelled += 1
        declared += is_heading
        tp += did_fire and is_heading
        fp += did_fire and not is_heading
    return {
        "document": pdf.stem,
        "heading_items": len(items),
        "labelled_lines": labelled,
        "declared_heading_lines": declared,
        "fired_lines": fired,
        "true_positives": tp,
        "false_positives": fp,
        "fired_unlabelled": fired_unlabelled,
        "fp_rate_bp": round(10000 * fp / labelled) if labelled else None,
        "recall_bp": round(10000 * tp / declared) if declared else None,
        "declared_headings_inferred": "headings-inferred-from-type" in s_codes,
    }


def pct(bp):
    return "-" if bp is None else f"{bp // 100}.{bp % 100:02d}%"


def main():
    work = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/headings-s3")
    work.mkdir(parents=True, exist_ok=True)
    version = subprocess.run([BINARY, "--version"], capture_output=True, text=True).stdout.strip()
    qpdf = subprocess.run(["qpdf", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    print(f"engine: {BINARY} ({version})\nqpdf:   {qpdf}\n")
    rows = []
    for pdf in DOCUMENTS:
        if not pdf.is_file():
            raise SystemExit(f"missing: {pdf}. A missing document is a failure, never a skip.")
        row = measure(pdf, work)
        rows.append(row)
        print(f"{row['document']:24} items {row['heading_items']:>4}  labelled {row['labelled_lines']:>6}  "
              f"declared {row['declared_heading_lines']:>4}  fired {row['fired_lines']:>4}  "
              f"TP {row['true_positives']:>4}  FP {row['false_positives']:>4}  "
              f"FP rate {pct(row['fp_rate_bp']):>7}  recall {pct(row['recall_bp']):>7}", flush=True)
    (work / "falsepos.json").write_text(json.dumps(rows, indent=1) + "\n")

    bounded = [r for r in rows if r["document"] not in COUNTS_ONLY]
    worst = max(bounded, key=lambda r: r["fp_rate_bp"] or 0)
    band = (min(r["fp_rate_bp"] or 0 for r in bounded), worst["fp_rate_bp"] or 0)
    print(f"\nband over the {len(bounded)} bounded documents: {pct(band[0])}..{pct(band[1])}, "
          f"worst {worst['document']}")
    for r in rows:
        if r["document"] in COUNTS_ONLY:
            print(f"counts only: {r['document']}: {r['false_positives']} FP of {r['labelled_lines']} "
                  f"labelled lines, {r['declared_heading_lines']} declared heading line(s)")
    met = all((r["fp_rate_bp"] or 0) <= BOUND_PERCENT * 100 for r in bounded)
    print(f"§7.5's bound, {BOUND_PERCENT}% on every bounded document: {'MET' if met else 'NOT MET'}")


if __name__ == "__main__":
    main()
