#!/usr/bin/env python3
"""Measure `extract` wall time and artifact size over the gate corpus, so "no regression" is a
reading rather than an opinion.

    ci/bench.py                          # measure the working tree's release binary
    ci/bench.py --repeat 5               # more repetitions, median reported
    ci/bench.py > baseline.tsv           # record
    ci/bench.py --check baseline.tsv     # compare, exit 1 on a regression

# Why this exists

Local performance is a stated priority and nothing measured it. `--diagnostics` reports one run's
`wall_micros` and there is no harness around it, so every claim that a change "costs nothing" has
been an argument rather than a number. `docs/CAPABILITY.md` says a capability moves with a
measurement and not with a sentence; this file is the instrument for the throughput half of that
rule, and its absence is why no earlier slice could make the claim at all.

# What is measured, and what that number contains

`wall_micros` from the engine's own `--diagnostics`, **not** a wall clock kept here. `main.rs`
wraps the whole subcommand in `timed(...)`, so the reported duration includes serializing the
artifact and writing it out. That is the honest boundary for a throughput claim — a caller waits
for the bytes, not for the parse — but it means the measurement is sensitive to how fast the
consumer drains stdout. Every timing run therefore writes to `/dev/null`, and a pipe is never in
the measured path.

# Size is measured once, because the contract says it may be

Two runs over one document produce identical bytes, so artifact size needs one run and not
`--repeat` of them. It is reported because it is the cost driver: across the six large gate
fixtures, wall time per megabyte **of output** is flat at 0.015 s/MB while time per megabyte of
*input* varies more than threefold. The engine is linear in what it emits. A slice that adds a
field to every node is therefore a throughput change, and this column is where that shows up.

# Median, not mean

One descheduled run should not be able to fail a gate, and one lucky run should not be able to
pass one. The median of an odd `--repeat` is a measurement the machine actually took.

# The tolerance is loose on purpose

`--check` allows 10% by default. This is a laptop-grade instrument on a shared machine: thermal
state and background load move a run by more than a careful slice does. A gate tight enough to
catch a 2% regression here would fire on the weather, and a gate that fires on the weather gets
switched off. Real regressions in this engine's hot path are multiples, not percentages.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import statistics
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "release" / "ethos-parser"
CORPUS = ROOT / "fixtures" / "gate"
DEFAULT_TOLERANCE = 0.10


def fixtures() -> list[pathlib.Path]:
    """The gate corpus, in a fixed order so two runs' rows line up."""
    return sorted(CORPUS.glob("*.pdf"))


def measure(pdf: pathlib.Path, repeat: int) -> tuple[int, int]:
    """`(median wall_micros, artifact bytes)` for one document."""
    times = []
    for _ in range(repeat):
        with open("/dev/null", "wb") as sink:
            done = subprocess.run(
                [str(BINARY), "extract", "--diagnostics", str(pdf)],
                stdout=sink,
                stderr=subprocess.PIPE,
                check=True,
            )
        # The diagnostics line is the last thing on stderr; anything before it is an omission
        # note, which is a fact about the document rather than about this run.
        line = done.stderr.decode().strip().splitlines()[-1]
        times.append(int(json.loads(line)["wall_micros"]))

    # Counted in chunks rather than with `subprocess.run(capture_output=True)`. The largest gate
    # artifact is most of a gigabyte, and a harness that needs a gigabyte of its own to weigh one
    # is an instrument that fails on exactly the document worth measuring.
    sized = subprocess.Popen(
        [str(BINARY), "extract", str(pdf)],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    total = 0
    assert sized.stdout is not None
    while chunk := sized.stdout.read(1 << 20):
        total += len(chunk)
    if sized.wait() != 0:
        raise RuntimeError(f"extract failed on {pdf.name}")
    return int(statistics.median(times)), total


def emit(repeat: int) -> list[tuple[str, int, int]]:
    rows = []
    for pdf in fixtures():
        micros, size = measure(pdf, repeat)
        rows.append((pdf.stem, micros, size))
    return rows


def read_baseline(path: pathlib.Path) -> dict[str, tuple[int, int]]:
    out = {}
    for line in path.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        name, micros, size = line.split("\t")
        out[name] = (int(micros), int(size))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repeat", type=int, default=3, help="timing runs per document (odd)")
    ap.add_argument("--check", type=pathlib.Path, help="compare against a recorded baseline")
    ap.add_argument(
        "--tolerance",
        type=float,
        default=DEFAULT_TOLERANCE,
        help=f"fractional slowdown allowed by --check (default {DEFAULT_TOLERANCE})",
    )
    args = ap.parse_args()

    if not BINARY.exists():
        print(
            f"engine: no release binary at {BINARY}. Run `cargo build --release --locked` first.",
            file=sys.stderr,
        )
        return 2

    rows = emit(args.repeat)

    if not args.check:
        print("# fixture\twall_micros_median\tartifact_bytes")
        for name, micros, size in rows:
            print(f"{name}\t{micros}\t{size}")
        return 0

    base = read_baseline(args.check)
    failed = False
    print(f"# comparing against {args.check}, tolerance {args.tolerance:.0%}")
    for name, micros, size in rows:
        if name not in base:
            print(f"NEW      {name}: {micros}us {size}B — not in the baseline")
            continue
        was_micros, was_size = base[name]
        drift = (micros - was_micros) / was_micros if was_micros else 0.0
        # A size change is reported at any magnitude and gates at none: the artifact growing is a
        # contract change, and a contract change is a decision somebody made on purpose.
        size_note = "" if size == was_size else f"  artifact {was_size}B -> {size}B"
        if drift > args.tolerance:
            failed = True
            print(f"SLOWER   {name}: {was_micros}us -> {micros}us ({drift:+.1%}){size_note}")
        else:
            print(f"ok       {name}: {was_micros}us -> {micros}us ({drift:+.1%}){size_note}")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
