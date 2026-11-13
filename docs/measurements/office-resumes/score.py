#!/usr/bin/env python3
"""Score `ethos-parser markdown --source` against the answer keys a generator wrote.

    ETHOS_PARSER_BIN=target/release/ethos-parser python3 docs/measurements/office-resumes/score.py \
        --set target/measure-sets/office-resumes --out docs/measurements/office-resumes/results.json

Shared by both measurement sets: `../financial-en/` is scored by this same file with its own
`--set`. Standard library only; dedicated to the public domain under CC0 1.0.

Every input named by a `*.key.json` in the set is projected once, its Markdown is written beside it
as `<source>.md`, and each key item is one pass or fail. A document the binary refuses scores every
item as failed rather than leaving the denominator short.

**How a Markdown line is read before matching**: one leading block marker (`#`, `>`, `-`, `*`, `+`,
`1.`) is removed, then emphasis markers, then backslash escapes, then a trailing hard break; `<br>`
counts as a line break. Pipe-table rows, and HTML `<tr>` rows should they ever appear, are split
into cells. "Normalised" means every whitespace run collapsed to one space.

Checks (a key may hold any subset):
  lines_strict          the line, tabs kept, equals one whole Markdown line
  lines_normalised      the same with whitespace normalised; also broken down by the line's shape
  one_line              a paragraph that must stay one line (a bold phrase in the middle)
  headings_line         heading text is one whole line
  headings_level        ... and carries exactly `level` `#` marks
  list_items_line       list item text is one whole line
  list_items_marked     ... and carries a list marker
  table_rows            every non-empty cell of the row appears, in order, in ONE table row
  table_rows_exact      ... and cell by cell in place, empty cells included (trailing empties
                        ignored), so an amount read into the wrong column fails
  table_cells_text      each cell is a whole line or a table cell somewhere (text kept at all)
  <present group>       the string appears verbatim (whitespace normalised, line breaks included),
                        not inside a longer number or word
  <groups group>        every line of the group is a whole line (a job's lines)
  <pairs group>         label and value on one line, label first (totals)
  rupee_adjacent        `₹` then at most one space then the amount, on one line
"""
import argparse
import hashlib
import html
import json
import os
import pathlib
import re
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[3]
MARK = re.compile(r"^\s*(?:#{1,6}\s+|>\s?|(?:[-*+]|\d+[.)])\s+)")
HEADING = re.compile(r"^\s*(#{1,6})\s")
LIST = re.compile(r"^\s*(?:[-*+]|\d+[.)])\s+")
ROW = re.compile(r"^\s*\|(.*)\|\s*$")
SEP = re.compile(r"^[\s|:-]+$")
BR = re.compile(r"<br\s*/?>", re.I)


def norm(s):
    return " ".join(s.split())


def cellnorm(s):
    return re.sub(r"₹ ", "₹", norm(s))


def plain(line):
    """A Markdown line as text: block marker, emphasis, escapes and a trailing hard break removed."""
    s = MARK.sub("", line, count=1)
    s = re.sub(r"(?<!\\)[*_]+", "", s)
    s = re.sub(r"\\([!-/:-@\[-`{-~])", r"\1", s)
    return s.rstrip(" \\").strip()


def found(needle, hay):
    """`needle` in `hay`, not as part of a longer number or word."""
    return re.search(r"(?<![\w.,])" + re.escape(norm(needle)) + r"(?!\w)", hay) is not None


def trim(cells):
    while cells and not cells[-1]:
        cells = cells[:-1]
    return cells


def subseq(want, got):
    it = iter(got)
    return all(any(w == g for g in it) for w in want)


class View:
    """One document's Markdown, read the ways the checks need."""

    def __init__(self, md):
        self.raw = [part for line in md.split("\n") for part in BR.split(line)]
        self.plain = [plain(r) for r in self.raw]
        self.lines = {norm(p) for p in self.plain}
        self.text = norm("\n".join(self.plain))
        self.rows = [[cellnorm(plain(BR.sub(" ", c))) for c in re.split(r"(?<!\\)\|", m.group(1))]
                     for line in md.split("\n") if (m := ROW.match(line)) and not SEP.match(line)]
        for tr in re.findall(r"<tr[^>]*>(.*?)</tr>", md, re.S | re.I):
            self.rows.append([cellnorm(html.unescape(re.sub(r"<[^>]+>", " ", c)))
                              for c in re.findall(r"<t[dh][^>]*>(.*?)</t[dh]>", tr, re.S | re.I)])
        self.cells = {c for r in self.rows for c in r} | {cellnorm(x) for x in self.lines}
        self.flat = [norm(p) for p in self.plain] + [" | ".join(r) for r in self.rows]


def pair_on_line(label, value, line):
    at = line.find(norm(label))
    return at >= 0 and found(value, line[at + len(norm(label)):])


def score(key, md):
    """{check: [(passed, wanted)]} for one document."""
    v, out = View(md), {}

    def add(check, ok, want):
        out.setdefault(check, []).append((bool(ok), want))

    for e in key["lines"]:
        add("lines_strict", any(p == e["strict"] for p in v.plain), e["strict"])
        ok = norm(e["normalised"]) in v.lines
        add("lines_normalised", ok, e["normalised"])
        add(f"lines_normalised:{e['shape']}", ok, e["normalised"])
    for t in key["one_line"]:
        add("one_line", norm(t) in v.lines, t)
    for h in key["headings"]:
        add("headings_line", norm(h["text"]) in v.lines, h["text"])
        add("headings_level", any((m := HEADING.match(r)) and len(m.group(1)) == h["level"] and norm(p) == norm(h["text"])
                                  for r, p in zip(v.raw, v.plain)), "#" * h["level"] + " " + h["text"])
    for li in key["list_items"]:
        add("list_items_line", norm(li["text"]) in v.lines, li["text"])
        add("list_items_marked", any(LIST.match(r) and norm(p) == norm(li["text"]) for r, p in zip(v.raw, v.plain)),
            li["text"])
    for row in key["table_rows"]:
        full = trim([cellnorm(c) for c in row])
        want = [c for c in full if c]
        add("table_rows", any(subseq(want, r) for r in v.rows), " | ".join(want))
        add("table_rows_exact", any(trim(r) == full for r in v.rows), " | ".join(full))
        for c in want:
            add("table_cells_text", c in v.cells, c)
    for group, items in key["present"].items():
        for s in items:
            add(group, found(s, v.text), s)
    for group, groups in key["groups"].items():
        for lines in groups:
            add(group, all(norm(x) in v.lines for x in lines), " / ".join(lines))
    for group, pairs in key["pairs"].items():
        for label, value in pairs:
            add(group, any(pair_on_line(label, value, f) for f in v.flat), f"{label} … {value}")
    for a in key["rupee"]:
        add("rupee_adjacent", any(re.search("₹ ?" + re.escape(a) + r"(?!\w)", f) for f in v.flat), "₹" + a)
    return out


def rel(p):
    p = pathlib.Path(p).resolve()
    return str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else p.name


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--set", type=pathlib.Path, required=True, help="directory holding the inputs and *.key.json")
    ap.add_argument("--bin", default=os.environ.get("ETHOS_PARSER_BIN"), help="ethos-parser binary (or $ETHOS_PARSER_BIN)")
    ap.add_argument("--out", type=pathlib.Path, help="write the JSON summary here")
    args = ap.parse_args()
    if not args.bin:
        ap.error("no binary: pass --bin or set ETHOS_PARSER_BIN")
    version = subprocess.run([args.bin, "--version"], capture_output=True, text=True, check=True).stdout.strip()

    checks, by_format, documents, generators, producers = {}, {}, {}, set(), set()
    for key_path in sorted(args.set.glob("*.key.json")):
        key = json.loads(key_path.read_text(encoding="utf-8"))
        src = args.set / key["source"]
        generators.add(key["generator"])
        if key.get("producer"):
            producers.add(key["producer"])
        proc = subprocess.run([args.bin, "markdown", "--source", str(src)], capture_output=True, timeout=600)
        md = json.loads(proc.stdout)["markdown"] if proc.returncode == 0 else ""
        (args.set / (key["source"] + ".md")).write_text(md, encoding="utf-8")
        result = score(key, md)
        fmt = src.suffix.lstrip(".")
        doc = {"sha256": hashlib.sha256(src.read_bytes()).hexdigest(), "exit": proc.returncode,
               "markdown_sha256": hashlib.sha256(md.encode()).hexdigest(), "checks": {}, "failed": []}
        for check, items in result.items():
            passed = sum(ok for ok, _ in items)
            doc["checks"][check] = [passed, len(items)]
            doc["failed"] += [[check, want] for ok, want in items if not ok]
            for table in (checks, by_format.setdefault(fmt, {})):
                t = table.setdefault(check, [0, 0])
                t[0] += passed
                t[1] += len(items)
        documents[key["source"]] = doc

    def rates(table):
        return {c: {"passed": p, "total": n, "rate": round(p / n, 4) if n else None} for c, (p, n) in table.items()}

    summary = {"set": rel(args.set), "generator": sorted(generators), "producers": sorted(producers),
               "binary": {"path": rel(args.bin), "version": version,
                          "sha256": hashlib.sha256(pathlib.Path(args.bin).read_bytes()).hexdigest()},
               "documents_scored": len(documents),
               "checks": rates(checks), "by_format": {f: rates(t) for f, t in sorted(by_format.items())},
               "documents": documents}
    fmts = sorted(by_format)
    print(f"{version} on {rel(args.set)}: {len(documents)} documents")
    print(f"{'check':34} {'all':>13}" + "".join(f" {f:>13}" for f in fmts if len(fmts) > 1))
    for c, (p, n) in checks.items():
        cols = "".join(f" {'%d/%d' % tuple(by_format[f][c]) if c in by_format[f] else '-':>13}" for f in fmts if len(fmts) > 1)
        print(f"{c:34} {p:>5}/{n:<4} {p / n:4.0%}{cols}")
    print(json.dumps({"binary": version, "checks": {c: r["rate"] for c, r in rates(checks).items()}}, ensure_ascii=False))
    if args.out:
        args.out.write_text(json.dumps(summary, ensure_ascii=False, indent=1, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
