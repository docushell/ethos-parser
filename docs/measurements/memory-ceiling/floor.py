#!/usr/bin/env python3
"""Separate the `--max-pages` floor from the per-page term, and bound the structure tree's share.

    floor.py BINARY OUT.tsv SCRATCH_DIR [--modes classify,classify0,max0,full,size,budget]
             [--docs name,...] [--ladder DOC=8,32,64] [--full-repeat N]

Each reading is `/usr/bin/time -l` around one process with stdout to `/dev/null` — the method
`ci/bench.py` established — with `peak memory footprint` read beside `maximum resident set size`,
because §9 showed the two can move differently. The modes:

- `classify`: reads the source under the ceiling, opens it (lopdf parses the whole object graph),
  and tallies the operators of eight pages' content streams. It never calls `structure::read`.
- `classify0`: `classify --sample-pages 0` — the same read and open and no tally at all, so its
  peak is the object graph and a small artifact. The control for the floor.
- `max0`: `extract --max-pages 0` — the read and open, then `structure::read` over the whole
  document and `tree_mcids_by_page` over the whole tree, a rayon fold that admits no page, and
  the sealing of an artifact with no pages. §5 named this the floor the flag cannot lower.
- `full`: every page admitted. `size` is bench.py's chunk-counted size pass, wrapped in `time` as
  a second sample whose sink is a pipe. `budget` and `--ladder` take intermediate budgets, so the
  rule is checked between its endpoints and not only at them.

So `max0 - classify0` is the structure tree plus what extract does document-wide that classify
does not; the untagged control has no tree, so on it that difference is the remainder alone. And
`full - max0` is the per-admitted-page term on the same document, with no process floor in it.

`classify`, `classify0` and `max0` run three times and the median is reported: the derived
coefficients are differences of small numbers. `full` runs once by default — this machine is
shared while this runs. Rows are appended to OUT.tsv, each invocation under its own header.
"""
from __future__ import annotations

import argparse
import json
import os
import pathlib
import statistics
import subprocess
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[3]
CORPUS = ROOT / "fixtures" / "gate"
CONTROL_NAME = "control:untagged-shredded-line"
CONTROL = ROOT / "fixtures" / "engine" / "untagged-shredded-line" / "document.pdf"
REPEAT_CHEAP = 3
BUDGET_POINTS = {"nist-sp-800-53Ar5": 128, "nist-sp-800-171r3": 64}
DEFAULT_MODES = "classify,max0,full,size,budget"


def time_report(stderr: str, key: str) -> int | None:
    """One `/usr/bin/time -l` figure in bytes, or `None` where the platform did not print it."""
    for raw in stderr.splitlines():
        if key in raw.lower():
            digits = [t for t in raw.lower().split() if t.isdigit()]
            if digits:
                return int(digits[0])
    return None


def diagnostics_wall(stderr: str) -> int | None:
    """`wall_micros` from the engine's own diagnostics line, found by shape as bench.py does."""
    for raw in stderr.splitlines():
        raw = raw.strip()
        if not raw.startswith("{"):
            continue
        try:
            obj = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if "wall_micros" in obj:
            return int(obj["wall_micros"])
    return None


def reading(stderr: str, exit_code: int) -> dict:
    return {
        "exit": exit_code,
        "wall_micros": diagnostics_wall(stderr),
        "rss": time_report(stderr, "maximum resident set size"),
        "footprint": time_report(stderr, "peak memory footprint"),
    }


def run(binary: str, args: list[str], sink_path: pathlib.Path | None, ok_exits: set[int]) -> dict:
    """One measured process. stdout goes to `sink_path`, or `/dev/null` when it is `None`."""
    sink = open(sink_path if sink_path else "/dev/null", "wb")
    try:
        done = subprocess.run(
            ["/usr/bin/time", "-l", binary, *args],
            stdout=sink,
            stderr=subprocess.PIPE,
        )
    finally:
        sink.close()
    err = done.stderr.decode(errors="replace")
    if done.returncode not in ok_exits:
        raise RuntimeError(f"{args} exited {done.returncode}: {err[-800:]}")
    return reading(err, done.returncode)


def size_pass(binary: str, pdf: pathlib.Path) -> tuple[int, dict]:
    """bench.py's size pass — counted in chunks so the harness never holds the artifact — with
    `time` around it as a second peak sample. The sink is a pipe here, not `/dev/null`."""
    proc = subprocess.Popen(
        ["/usr/bin/time", "-l", binary, "extract", "--diagnostics", str(pdf)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    total = 0
    assert proc.stdout is not None
    while chunk := proc.stdout.read(1 << 20):
        total += len(chunk)
    _, err_bytes = proc.communicate()
    err = err_bytes.decode(errors="replace")
    if proc.returncode != 0:
        raise RuntimeError(f"size pass on {pdf.name} exited {proc.returncode}: {err[-800:]}")
    return total, reading(err, proc.returncode)


def pages_of(max0_artifact: pathlib.Path) -> tuple[int, list[str]]:
    """`coverage.pages_authorized` and the limitation codes of a `--max-pages 0` artifact."""
    doc = json.loads(max0_artifact.read_text())
    assurance = doc["representation"]["assurance"]
    codes = [lim["code"] for lim in assurance.get("limitations", [])]
    return int(assurance["coverage"]["pages_authorized"]), codes


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("binary")
    ap.add_argument("out", type=pathlib.Path)
    ap.add_argument("scratch", type=pathlib.Path)
    ap.add_argument("--modes", default=DEFAULT_MODES)
    ap.add_argument("--docs", default="", help="comma-separated fixture stems; default all plus the control")
    ap.add_argument("--ladder", action="append", default=[], help="DOC=b1,b2,... intermediate budgets, once each")
    ap.add_argument("--full-repeat", type=int, default=1)
    args = ap.parse_args()
    binary, out_path, scratch = args.binary, args.out, args.scratch
    modes = set(args.modes.split(","))
    scratch.mkdir(parents=True, exist_ok=True)
    version = subprocess.run([binary, "--version"], capture_output=True, text=True).stdout.strip()
    rows: list[str] = []
    started = time.time()
    load_start = os.getloadavg()

    def record(fixture: str, mode: str, budget: str, n: int, pages, r: dict, artifact):
        rows.append(
            "\t".join(
                str(x) if x is not None else "-"
                for x in (fixture, mode, budget, n, pages, r["wall_micros"], r["rss"], r["footprint"], artifact, r["exit"])
            )
        )

    docs = [(p.stem, p) for p in sorted(CORPUS.glob("*.pdf"))] + [(CONTROL_NAME, CONTROL)]
    if args.docs:
        wanted = set(args.docs.split(","))
        docs = [(n, p) for n, p in docs if n in wanted]
    ladders = {}
    for spec in args.ladder:
        name, budgets = spec.split("=", 1)
        ladders[name] = [int(b) for b in budgets.split(",")]

    notes_path = scratch / "notes.json"
    notes: dict[str, dict] = json.loads(notes_path.read_text()) if notes_path.exists() else {}
    for name, pdf in docs:
        print(f"## {name}", file=sys.stderr, flush=True)
        note = notes.setdefault(name, {})
        page_count = note.get("page_count")
        if "classify" in modes:
            cls = []
            for i in range(REPEAT_CHEAP):
                sink = scratch / f"{name}.classify.json" if i == 0 else None
                r = run(binary, ["classify", "--diagnostics", str(pdf)], sink, {0, 1})
                cls.append(r)
                record(name, "classify", "-", i + 1, None, r, None)
            page_count = int(json.loads((scratch / f"{name}.classify.json").read_text())["page_count"])
            note["page_count"] = page_count
            note["classify_rss_median"] = int(statistics.median(r["rss"] for r in cls))
        if "classify0" in modes:
            cls0 = []
            for i in range(REPEAT_CHEAP):
                sink = scratch / f"{name}.classify0.json" if i == 0 else None
                r = run(binary, ["classify", "--sample-pages", "0", "--diagnostics", str(pdf)], sink, {0, 1})
                cls0.append(r)
                record(name, "classify0", "-", i + 1, None, r, None)
            scanned = json.loads((scratch / f"{name}.classify0.json").read_text())["pages_content_scanned"]
            if scanned != 0:
                raise RuntimeError(f"classify --sample-pages 0 scanned {scanned} pages on {name}")
            note["classify0_rss_median"] = int(statistics.median(r["rss"] for r in cls0))
        if "max0" in modes:
            m0 = []
            for i in range(REPEAT_CHEAP):
                sink = scratch / f"{name}.max0.json" if i == 0 else None
                r = run(binary, ["extract", "--max-pages", "0", "--diagnostics", str(pdf)], sink, {0})
                m0.append(r)
                record(name, "max0", "0", i + 1, page_count, r, None)
            authorized, codes = pages_of(scratch / f"{name}.max0.json")
            note["pages_authorized_at_max0"] = authorized
            note["max0_limitation_codes"] = codes
            note["max0_rss_median"] = int(statistics.median(r["rss"] for r in m0))
        if "full" in modes:
            for i in range(args.full_repeat):
                full = run(binary, ["extract", "--diagnostics", str(pdf)], None, {0})
                record(name, "full", "-", i + 1, page_count, full, None)
                seen = note.get("full_rss")
                note["full_rss"] = (seen if isinstance(seen, list) else [seen] if seen else []) + [full["rss"]]
        if "size" in modes:
            artifact, full2 = size_pass(binary, pdf)
            record(name, "full-size-pass", "-", 0, page_count, full2, artifact)
            note["full_rss_size_pass"] = full2["rss"]
            note["artifact_bytes"] = artifact
        budgets = []
        if "budget" in modes and name in BUDGET_POINTS:
            budgets.append(BUDGET_POINTS[name])
        budgets += ladders.get(name, [])
        for b in budgets:
            r = run(binary, ["extract", "--max-pages", str(b), "--diagnostics", str(pdf)], None, {0})
            record(name, "budget", str(b), 1, page_count, r, None)
            note.setdefault("budget_rss", {})[str(b)] = r["rss"]
        print(json.dumps({name: note}), file=sys.stderr, flush=True)

    load_end = os.getloadavg()
    header = [
        f"# floor.py — {version} — binary {binary} — modes {','.join(sorted(modes))}"
        + (f" — ladders {ladders}" if ladders else ""),
        f"# started {time.strftime('%Y-%m-%d %H:%M:%S %z', time.localtime(started))}, "
        f"{time.time() - started:.0f} s; load average at start {load_start}, at end {load_end}",
        "# rss and footprint are bytes from `/usr/bin/time -l`; wall_micros is the engine's own diagnostics "
        "line; the full-size-pass row's sink is a pipe, every other row's is /dev/null or a scratch file",
        "# fixture\tmode\tbudget\trun\tpages\twall_micros\tpeak_rss_bytes\tpeak_footprint_bytes\tartifact_bytes\texit",
    ]
    with out_path.open("a") as out:
        out.write("\n".join(header + rows) + "\n")
    notes_path.write_text(json.dumps(notes, indent=1))
    print(f"appended {len(rows)} rows to {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
