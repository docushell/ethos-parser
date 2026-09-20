#!/usr/bin/env python3
"""ethos-parser against liteparse on opendataloader-bench: quality, speed and peak memory.

    ETHOS_BENCH=~/path/to/opendataloader-bench \
    ETHOS_PARSER_BIN=…/target/release/ethos-parser \
    LIT_BIN=~/.cargo/bin/lit \
    $ETHOS_BENCH/.venv/bin/python headtohead.py

Scores with `evaluator_reading_order`, `evaluator_table` and `evaluator_heading_level` from that
harness, unmodified, so the numbers are its numbers rather than a reimplementation — the same three
this repository already runs in `../opendataloader-bench/score.py`. Needs its virtualenv
(`rapidfuzz`, `apted`).

**This is an instrument, and its result is not published.** `docs/06-STEAL-REFUSE.md` O26 refuses
"#1, fastest, or any bake-off claim", and whether a head-to-head may be *published* is North Star
decision D3, which the owner has not taken. Running one internally needs no decision; saying "we
win" in public is the separate thing that stays refused. Every number here is a measurement of two
programs on one corpus on one machine, and the README beside it says what it does not measure.

**Each engine is run as its own CLI, one document at a time, and nothing is second-guessed.**
ethos-parser goes through `extract` then `markdown`, which is what the harness's own adapter does
(`pdf_parser_ethos_parser.py`, whose docstring records that an adapter which rebuilt lines itself
measured the adapter). liteparse goes through `lit parse --format markdown`. A document that fails
writes an empty prediction and scores zero rather than leaving the denominator short.

**Peak memory is the process's own**, from `wait4`'s `ru_maxrss` — bytes on macOS, kilobytes on
Linux, normalised here. For ethos-parser, whose CLI path is two processes and a representation on
disk, the peak reported is the larger of the two and the seconds are their sum: that is what a
caller pays for one Markdown file.
"""
from __future__ import annotations

import json
import os
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

BENCH = Path(os.environ.get("ETHOS_BENCH", Path.home() / "ethos-external-benchmarks/opendataloader-bench")).expanduser()
ETHOS = os.environ.get("ETHOS_PARSER_BIN", str(Path(__file__).resolve().parents[3] / "target/release/ethos-parser"))
LIT = os.environ.get("LIT_BIN", str(Path.home() / ".cargo/bin/lit"))
OUT = Path(os.environ.get("HEAD_TO_HEAD_OUT", "/tmp/liteparse-head-to-head"))

sys.path.insert(0, str(BENCH / "src"))
from evaluator_heading_level import evaluate_heading_level  # noqa: E402
from evaluator_reading_order import evaluate_reading_order  # noqa: E402
from evaluator_table import evaluate_table  # noqa: E402

# liteparse's own knobs decide what reaches the Markdown, so it is measured under three of them
# rather than one. Its defaults strip running headers and footers and emit an image placeholder
# per raster image; `--no-links` and `--image-mode off` exist for benchmark parity by its own
# help text. ethos-parser has no such knob — it emits the projection verbatim — so `furniture`
# is the set whose content selection is closest to what this engine does.
FLAG_SETS: dict[str, list[str]] = {
    "default": [],
    "parity": ["--no-links", "--image-mode", "off"],
    "furniture": ["--no-links", "--image-mode", "off", "--keep-headers-footers"],
}

KB = 1 if sys.platform == "darwin" else 1024  # ru_maxrss is bytes on macOS, kilobytes on Linux


def measured(cmd: list[str], stdout_path: Path) -> tuple[float, int, int]:
    """Run `cmd`, returning (wall seconds, peak RSS bytes, exit code).

    stdout and stderr go to files rather than pipes: a pipe that fills while nothing reads it
    deadlocks, and these engines write megabytes to stdout.
    """
    with open(stdout_path, "wb") as out, tempfile.TemporaryFile() as err:
        started = time.monotonic()
        proc = subprocess.Popen(cmd, stdout=out, stderr=err)
        _, status, ru = os.wait4(proc.pid, 0)
        seconds = time.monotonic() - started
        proc.returncode = os.waitstatus_to_exitcode(status)
    return seconds, ru.ru_maxrss * KB, proc.returncode


def run_ethos(pdf: Path, out_dir: Path) -> dict:
    """`extract` then `markdown`, the two-process path the harness's own adapter uses."""
    with tempfile.TemporaryDirectory() as tmp:
        repr_path = Path(tmp) / "repr.json"
        secs_a, peak_a, code_a = measured([ETHOS, "extract", str(pdf)], repr_path)
        md_json = Path(tmp) / "md.json"
        if code_a == 0:
            secs_b, peak_b, code_b = measured([ETHOS, "markdown", str(repr_path)], md_json)
        else:
            secs_b, peak_b, code_b = 0.0, 0, code_a
        markdown = ""
        if code_a == 0 and code_b == 0:
            try:
                markdown = json.loads(md_json.read_text(encoding="utf-8"))["markdown"]
            except Exception:
                markdown = ""
    (out_dir / f"{pdf.stem}.md").write_text(markdown, encoding="utf-8")
    return {"seconds": secs_a + secs_b, "peak_bytes": max(peak_a, peak_b),
            "ok": bool(markdown), "exit": code_a or code_b}


def run_lit(pdf: Path, out_dir: Path, flags: list[str]) -> dict:
    """`lit parse --format markdown`, with OCR off: this build has no tesseract feature."""
    target = out_dir / f"{pdf.stem}.md"
    secs, peak, code = measured(
        [LIT, "parse", "--format", "markdown", "--no-ocr", "-q", *flags, str(pdf)], target)
    if code != 0:
        target.write_text("", encoding="utf-8")
    return {"seconds": secs, "peak_bytes": peak, "ok": code == 0 and target.stat().st_size > 0,
            "exit": code}


def score(out_dir: Path, stems: set[str]) -> dict[str, list[tuple[str, float]]]:
    """NID, TEDS and MHS per document, by the harness's own evaluators.

    Only the documents this run parsed are scored. Scoring a ground truth whose document was never
    run would add a zero for a prediction nobody made, which on a short run is most of the corpus.
    """
    scored: dict[str, list[tuple[str, float]]] = {"nid": [], "teds": [], "mhs": []}
    for gt in sorted((BENCH / "ground-truth" / "markdown").glob("*.md")):
        if gt.stem not in stems:
            continue
        pred_path = out_dir / gt.name
        pred = pred_path.read_text(encoding="utf-8") if pred_path.is_file() else ""
        truth = gt.read_text(encoding="utf-8")
        for key, fn in (("nid", evaluate_reading_order), ("teds", evaluate_table),
                        ("mhs", evaluate_heading_level)):
            try:
                v = fn(truth, pred)
                v = v[0] if isinstance(v, tuple) else v
                if v is not None:
                    scored[key].append((gt.stem, v))
            except Exception:
                pass
    return scored


def band(scored: list[tuple[str, float]], label: str, unit: str = "", worst_is_min: bool = True) -> dict:
    """Decision #18: a band with its worst document named, never a macro alone."""
    if not scored:
        return {}
    values = sorted(v for _, v in scored)
    worst = (min if worst_is_min else max)(scored, key=lambda dv: (dv[1], dv[0]))
    best = (max if worst_is_min else min)(scored, key=lambda dv: (dv[1], dv[0]))
    stat = {"mean": statistics.fmean(values), "median": statistics.median(values),
            "low": values[0], "high": values[-1], "worst": worst[0], "worst_value": worst[1],
            "best": best[0], "best_value": best[1], "n": len(values)}
    if not unit:
        fmt = lambda v: f"{v:.4f}"  # noqa: E731
    elif unit == "s":
        fmt = lambda v: f"{v:.3f}s"  # noqa: E731
    else:
        fmt = lambda v: f"{v:.1f}{unit}"  # noqa: E731
    print(f"    {label:22s} mean {fmt(stat['mean'])}  median {fmt(stat['median'])}  "
          f"band {fmt(stat['low'])}..{fmt(stat['high'])}  worst {stat['worst']} ({fmt(stat['worst_value'])})")
    return stat


def main() -> None:
    docs = sorted((BENCH / "pdfs").glob("*.pdf"))
    if not docs:
        sys.exit(f"no PDFs under {BENCH / 'pdfs'} — set ETHOS_BENCH, and note the corpus is Git LFS")
    # A smoke run before the real one: HEAD_TO_HEAD_LIMIT=5 exercises every path in a few seconds.
    # The published figures are the full corpus, and the JSON records how many documents it saw.
    if limit := int(os.environ.get("HEAD_TO_HEAD_LIMIT", "0")):
        docs = docs[:limit]
    engines = [("ethos-parser", None)] + [(f"liteparse:{name}", name) for name in FLAG_SETS]
    results: dict[str, dict] = {}

    for engine, flag_set in engines:
        out_dir = OUT / engine.replace(":", "-")
        out_dir.mkdir(parents=True, exist_ok=True)
        print(f"\n  {engine} over {len(docs)} documents…", flush=True)
        per_doc = {}
        for pdf in docs:
            per_doc[pdf.stem] = (run_ethos(pdf, out_dir) if flag_set is None
                                 else run_lit(pdf, out_dir, FLAG_SETS[flag_set]))
        scored = score(out_dir, {pdf.stem for pdf in docs})
        failed = [d for d, r in per_doc.items() if not r["ok"]]
        print(f"    {len(docs) - len(failed)} of {len(docs)} produced Markdown"
              + (f"; failed: {', '.join(sorted(failed)[:5])}" if failed else ""))
        stats = {
            "nid": band(scored["nid"], "NID reading order"),
            "teds": band(scored["teds"], "TEDS table structure"),
            "mhs": band(scored["mhs"], "MHS headings"),
            "seconds": band([(d, r["seconds"]) for d, r in per_doc.items()], "seconds/document",
                            unit="s", worst_is_min=False),
            "peak_mb": band([(d, r["peak_bytes"] / 1e6) for d, r in per_doc.items()],
                            "peak RSS", unit=" MB", worst_is_min=False),
        }
        results[engine] = {"per_document": per_doc, "stats": stats,
                           "scored": {k: dict(v) for k, v in scored.items()},
                           "failed": failed,
                           "flags": FLAG_SETS.get(flag_set or "", [])}

    # A control this run did not produce: the harness's own stored liteparse predictions, scored by
    # the same evaluators. It is the check on the invocation above — a fresh run far below the
    # harness's own would mean the flags, not the engine, were being measured. No timing: those
    # Markdown files were written on another machine on another day.
    stored = BENCH / "prediction" / "liteparse" / "markdown"
    if stored.is_dir():
        stems = {p.stem for p in stored.glob("*.md")} & {pdf.stem for pdf in docs}
        scored = score(stored, stems)
        print(f"\n  liteparse stored by the harness ({len(stems)} documents, scored only)…")
        results["liteparse:stored-by-harness"] = {
            "stats": {"nid": band(scored["nid"], "NID reading order"),
                      "teds": band(scored["teds"], "TEDS table structure"),
                      "mhs": band(scored["mhs"], "MHS headings")},
            "scored": {k: dict(v) for k, v in scored.items()},
            "note": "the harness's own run, version per prediction/liteparse/summary.json; not timed here",
        }

    meta = {
        "host": subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True,
                               text=True).stdout.strip() or os.uname().machine,
        "ethos_parser": subprocess.run([ETHOS, "--version"], capture_output=True, text=True).stdout.strip(),
        "liteparse": subprocess.run([LIT, "--version"], capture_output=True, text=True).stdout.strip(),
        "documents": len(docs),
        "corpus": str(BENCH),
        "date": time.strftime("%Y-%m-%d"),
    }
    payload = {"meta": meta, "engines": results}
    out_json = Path(__file__).resolve().parent / "results.json"
    out_json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"\n  wrote {out_json}")
    print(f"  {meta['ethos_parser']} against {meta['liteparse']} on {meta['host']}")


if __name__ == "__main__":
    main()
