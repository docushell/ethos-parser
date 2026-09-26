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

# Peak memory is measured too, and it is the ceiling

Throughput is linear in emitted bytes; **memory is where this stops working.** Across the gate
corpus peak RSS runs 3.9x to 5.9x the artifact — `nist-sp-800-161r1` is 4.6 MB in and peaks at
1.10 GiB. The largest gate document is no longer an extrapolation: `nist-sp-800-53Ar5` is 7.12 MiB
in, emits a 950 MiB artifact, and **peaks at 3.65 GiB** — down from 6.5 GiB, after role-path
sharing (58a1342) and c14n adopting the payload's largest field instead of copying it took 44%
off between them, byte-identically.

Note what that change also settles about this paragraph's own former explanation. It used to blame
`canonical_bytes_of` returning one `Vec<u8>` while the nodes were still alive. That buffer is real
— 950 MiB on the largest document — but an A/B measured the print instant sitting 1.1-2.1 GiB
BELOW the high-water mark, so removing it moves no peak at all.
**Amended 2026-09-26:** that A/B ran at 97fa562, before the adopt change took ~950 MiB out of
`seal`. After it the print instant WAS the peak, and `extract` now streams the artifact instead of
assembling it: `nist-sp-800-53Ar5` peaks at 3059.9 MiB rather than 3825.2, and `nist-sp-800-161r1`
at 875.4 rather than 1161.4, byte-identical (`docs/measurements/memory-ceiling/` §6).

**An earlier draft of this paragraph said "roughly 300x the input" and that figure was withdrawn.**
Measured at 97fa562 it runs 143x to 933x — a 6.5x spread, so it is a corpus median and not a
bound, and anyone provisioning from 300x under-sizes the worst gate document threefold. Role-path
sharing did not rescue the model: at 58a1342 the same ratio is 133x to 655x, still a 4.9x spread,
and after the adopt change 135x to 524x, 3.9x.
The input-bytes model is wrong in kind, not merely mis-calibrated. Peak is
two terms, both knowable before the run: a floor that scales with the document's TOTAL page count
and does not respond to `--max-pages` at all, plus a constant marginal cost per ADMITTED page. The
readings, the model fit that rejected the input-bytes model, and the ladders that establish
linearity are in `docs/measurements/memory-ceiling/`.

None of that was visible from inside the process: `Diagnostics::resident_bytes` is `Some` only on
Linux and `None` on the platform this repository is developed on, which is a correct typed absence
for the artifact and a useless one for a gate. So the harness measures it from outside, with
`/usr/bin/time` wrapping the SAME runs the timing loop already makes — so it costs no extra process
and is medianed over exactly the samples wall time is.

**Medianed, because one sample of this is not a measurement.** The first draft took a single
unreplicated RSS reading beside `repeat` medianed timings. On a 1.3 GB document that reading moved
16% run to run, which is wide enough to manufacture a regression or hide one; on a 230 MB document
it moves about 2.6%, so the noise grows with the allocation and the small fixtures never showed it.
The number is still REPORTED rather than gated — the allocator decides when to return pages — but
it is now the same kind of number as the one beside it.

# Size is measured once, because the contract says it may be

Two runs over one document produce identical bytes, so artifact size needs one run and not
`--repeat` of them. It is reported because it is the cost driver: across the six large gate
fixtures, wall time per megabyte **of output** is flat at 0.015 s/MB while time per megabyte of
*input* varies more than threefold. The engine is linear in what it emits. A slice that adds a
field to every node is therefore a throughput change, and this column is where that shows up.

# Median, not mean

One descheduled run should not be able to fail a gate, and one lucky run should not be able to
pass one. The median of an odd `--repeat` is a measurement the machine actually took.

# A baseline is only comparable to a run taken under the same conditions

`--check` compares against a recorded file and has no way to know what the machine was doing when
that file was written. Both directions of that have now bitten, on this corpus, in one afternoon:
a busy machine turned a +0.7% document into a reported **+26%** regression, and a baseline recorded
while several agent workflows were running made an unrelated change look like a **-40%**
improvement — the same document and the same code measured 445,355 us into that baseline and
276,118 us against a quiet machine, a 61% gap with no code between them.

So `--check` is a REGRESSION WATCH within a session, not a verdict on a change. **To judge a
change, A/B it back to back:** measure the tree, `git stash`, rebuild, measure again, restore.
Same machine, same minute, same thermal state, medians on both sides. That is the only comparison
here that has ever survived scrutiny, and it is cheap.

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


def _time_flag() -> str:
    """`-l` on BSD/macOS, `-v` on GNU. Probed once rather than guessed from the platform name."""
    for flag in ("-l", "-v"):
        try:
            done = subprocess.run(
                ["/usr/bin/time", flag, "true"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
            )
        except OSError:
            return "-l"
        if done.returncode == 0 and "resident set size" in done.stderr.decode(errors="replace").lower():
            return flag
    return "-l"


TIME_FLAG = _time_flag()


def fixtures() -> list[pathlib.Path]:
    """The gate corpus, in a fixed order so two runs' rows line up."""
    return sorted(CORPUS.glob("*.pdf"))


def rss_from_time_report(stderr: str) -> int | None:
    """Peak RSS out of a `/usr/bin/time` report, or `None` where the platform will not say.

    BSD/macOS `-l` prints "maximum resident set size" in BYTES; GNU `-v` prints "Maximum resident
    set size" in KILOBYTES. Both spellings are read and the unit difference applied. Anything else
    returns `None` rather than a number nobody took.
    """
    for raw in stderr.splitlines():
        low = raw.lower()
        if "maximum resident set size" in low:
            digits = [t for t in low.replace(":", " ").split() if t.isdigit()]
            if digits:
                # GNU spells it "Maximum" and reports kilobytes; BSD spells it "maximum" and
                # reports bytes. The capital is the only discriminator either tool offers.
                return int(digits[0]) * (1024 if "Maximum resident" in raw else 1)
    return None


def measure(pdf: pathlib.Path, repeat: int) -> tuple[int, int, int | None]:
    """`(median wall_micros, artifact bytes, peak rss bytes or None)` for one document."""
    times: list[int] = []
    rss: list[int] = []
    # `/usr/bin/time` wraps the SAME runs the timing loop already makes, so peak memory costs no
    # extra process and is medianed over exactly the samples wall time is. An earlier draft took
    # one unreplicated RSS sample beside `repeat` medianed timings; on a 1.3 GB document that
    # single sample moved 16% run to run, which is a measurement nobody can act on.
    for _ in range(repeat):
        with open("/dev/null", "wb") as sink:
            done = subprocess.run(
                ["/usr/bin/time", TIME_FLAG, str(BINARY), "extract", "--diagnostics", str(pdf)],
                stdout=sink,
                stderr=subprocess.PIPE,
                check=True,
            )
        err = done.stderr.decode(errors="replace")
        # The diagnostics line is found by SHAPE, not by position: `/usr/bin/time` appends its own
        # report after the child exits, so "the last line" stopped being the engine's line the
        # moment this was wrapped. Anything else on stderr is an omission note, which is a fact
        # about the document rather than about this run.
        for raw in err.splitlines():
            raw = raw.strip()
            if not raw.startswith("{"):
                continue
            try:
                obj = json.loads(raw)
            except json.JSONDecodeError:
                continue
            if "wall_micros" in obj:
                times.append(int(obj["wall_micros"]))
                break
        else:
            raise RuntimeError(f"no diagnostics line from extract on {pdf.name}")
        peak = rss_from_time_report(err)
        if peak is not None:
            rss.append(peak)

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
    median_rss = int(statistics.median(rss)) if len(rss) == repeat else None
    return int(statistics.median(times)), total, median_rss


def emit(repeat: int) -> list[tuple[str, int, int, int | None]]:
    rows = []
    for pdf in fixtures():
        micros, size, rss = measure(pdf, repeat)
        rows.append((pdf.stem, micros, size, rss))
    return rows


def read_baseline(path: pathlib.Path) -> dict[str, tuple[int, int, int | None]]:
    """Rows from a recorded run. A three-column baseline predates the RSS column and still reads.

    Kept tolerant on purpose: a baseline is a record of what a machine did, and refusing to read
    one taken last week because a column was added since would throw away the only evidence a
    comparison has.
    """
    out = {}
    for line in path.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        parts = line.split("\t")
        name, micros, size = parts[0], int(parts[1]), int(parts[2])
        rss = int(parts[3]) if len(parts) > 3 and parts[3] not in ("", "-") else None
        out[name] = (micros, size, rss)
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

    # A gate on one sample is not a gate. `--repeat 1` makes "median" a synonym for "whichever
    # run happened", and it fired: a slice measured at +26% on the smallest fixture was, over
    # eleven runs, +0.7% — the whole excursion was one descheduled sample on a 15 ms document.
    # Recording a baseline with `--repeat 1` is allowed, because a baseline is a record rather
    # than a verdict; checking against one is not.
    if args.check and args.repeat < 3:
        print(
            f"engine: --check needs --repeat 3 or more, got {args.repeat}. The median of one "
            f"sample is that sample, and this instrument's tolerance assumes a median.",
            file=sys.stderr,
        )
        return 2

    if not BINARY.exists():
        print(
            f"engine: no release binary at {BINARY}. Run `cargo build --release --locked` first.",
            file=sys.stderr,
        )
        return 2

    rows = emit(args.repeat)

    if not args.check:
        print("# fixture\twall_micros_median\tartifact_bytes\tpeak_rss_bytes")
        for name, micros, size, rss in rows:
            print(f"{name}\t{micros}\t{size}\t{rss if rss is not None else '-'}")
        return 0

    base = read_baseline(args.check)
    failed = False
    print(f"# comparing against {args.check}, tolerance {args.tolerance:.0%}")
    for name, micros, size, rss in rows:
        if name not in base:
            print(f"NEW      {name}: {micros}us {size}B — not in the baseline")
            continue
        was_micros, was_size, was_rss = base[name]
        drift = (micros - was_micros) / was_micros if was_micros else 0.0
        # A size change is reported at any magnitude and gates at none: the artifact growing is a
        # contract change, and a contract change is a decision somebody made on purpose.
        size_note = "" if size == was_size else f"  artifact {was_size}B -> {size}B"
        # Peak memory is REPORTED and gates at nothing. It is the ceiling rather than the speed,
        # it is noisier than wall time because the allocator decides when to return pages, and a
        # baseline taken before the column existed has nothing to compare against.
        if rss is not None and was_rss:
            size_note += f"  rss {was_rss // 1_000_000}MB -> {rss // 1_000_000}MB"
        elif rss is not None:
            size_note += f"  rss {rss // 1_000_000}MB"
        if drift > args.tolerance:
            failed = True
            print(f"SLOWER   {name}: {was_micros}us -> {micros}us ({drift:+.1%}){size_note}")
        else:
            print(f"ok       {name}: {was_micros}us -> {micros}us ({drift:+.1%}){size_note}")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
