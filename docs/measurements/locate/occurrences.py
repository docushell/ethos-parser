#!/usr/bin/env python3
"""D1 S3 — how many occurrences a realistic quote has over `fixtures/gate`, and what a call costs.

    python3 docs/measurements/locate/occurrences.py [WORKDIR]

Stdlib only, no third-party import: a measurement instrument in this repository carries no
dependency, and decision #14 keeps AGPL out of the tree including the instruments.

# What it measures, and why each number is here

Two quotes per document, because one number would answer the wrong question:

1. **A realistic quote**, lifted from the document's own text — the first text run of at least 60
   scalars, truncated to 60. That is citation-shaped, and it is guaranteed to occur at least once
   because a run's text is a substring of its own block's text. This is the case a caller has.
2. **The worst realistic case for the count**: the bare word ``the``. It is not a citation, and it
   is not presented as one; it is the shape that produces the largest occurrence list a caller
   could stumble into with a short quote, which is what a band needs at its top end.

For each it records the occurrence count, the node parts across those occurrences, the artifact's
size, the wall clock of the whole process, and the engine's own measured duration from
``--diagnostics`` — which excludes process start-up, so the two together say how much of a call is
the answer and how much is exec.

Representations are extracted once per document and cached in WORKDIR, because extracting the
733-page document is minutes and gigabytes and has nothing to do with what is being measured.
"""

import json
import os
import re
import pathlib
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[3]
GATE = pathlib.Path(os.environ.get("ETHOS_GATE_CORPUS") or REPO / "fixtures" / "gate")
BINARY = pathlib.Path(
    os.environ.get("ETHOS_PARSER") or REPO / "target" / "release" / "ethos-parser"
)

# Page order, smallest first, as `docs/measurements/memory-ceiling/README.md` lists them.
DOCUMENTS = [
    "irs-f1040sd-2025",
    "irs-fw9",
    "nist-sp-800-218",
    "nist-sp-800-207",
    "nist-sp-800-171r3",
    "nist-sp-800-37r2",
    "nist-sp-800-161r1",
    "nist-sp-800-53Ar5",
]

QUOTE_SCALARS = 60


def run(args, **kwargs):
    return subprocess.run([str(BINARY), *args], capture_output=True, check=False, **kwargs)


def representation(name, work):
    """The document's representation, extracted once and cached."""
    out = work / f"{name}.json"
    if not out.is_file():
        source = GATE / f"{name}.pdf"
        if not source.is_file():
            raise SystemExit(f"missing gate document: {source}")
        started = time.monotonic()
        completed = run(["extract", str(source)])
        if completed.returncode != 0:
            raise SystemExit(
                f"{name}: extract exited {completed.returncode}: "
                f"{completed.stderr.decode('utf-8', 'replace')[:400]}"
            )
        out.write_bytes(completed.stdout)
        print(
            f"  extracted {name}: {len(completed.stdout)} bytes in "
            f"{time.monotonic() - started:.1f} s",
            flush=True,
        )
    return out


RUN_FIELD = re.compile(r'"origin_y":(-?\d+)|"text":"((?:[^"\\]|\\.)*)"')


def placed_runs(path, want=4000, ceiling=256 * 1024 * 1024):
    """`(origin_y, text)` for the document's first runs, in node order, by scanning.

    A scan rather than a parse: the 733-page document's representation is most of a gigabyte, and
    parsing it whole to pick a quote would make the instrument's cost the thing being measured.

    **It cannot be a fixed prefix either**, which the first version of this got wrong. Canonical
    JSON sorts an object's keys, and the artifact's are `artifact_type`, `geometry`,
    `representation`, `representation_c14n_sha256`, `schema_version` — so the geometry sidecar
    comes first, and on `nist-sp-800-218` four megabytes of prefix were all geometry rows and no
    nodes. This reads forward until it finds `"nodes":[` and takes fields from there.

    Within one node the sorted keys put `native_locator` — which is where `origin_y` lives —
    before `text`, so one pass alternating the two pairs each run with its baseline.
    """
    runs = []
    buffer = ""
    seen_nodes = False
    read = 0
    pending_y = None
    with path.open("rb") as handle:
        while len(runs) < want and read < ceiling:
            chunk = handle.read(4 * 1024 * 1024)
            if not chunk:
                break
            read += len(chunk)
            buffer += chunk.decode("utf-8", "ignore")
            if not seen_nodes:
                at = buffer.find('"nodes":[')
                if at < 0:
                    buffer = buffer[-16:]
                    continue
                seen_nodes = True
                buffer = buffer[at:]
            last_end = 0
            for m in RUN_FIELD.finditer(buffer):
                last_end = m.end()
                if m.group(1) is not None:
                    pending_y = int(m.group(1))
                elif pending_y is not None:
                    runs.append((pending_y, json.loads('"' + m.group(2) + '"')))
            # Continue after the last match, or — having matched nothing — keep a tail long
            # enough for a match that straddles the chunk boundary. Re-scanning the tail would
            # duplicate runs, and the order of these runs is what a candidate quote is built on.
            buffer = buffer[last_end:] if last_end else buffer[-4096:]
    return runs


def lines(path):
    """The runs grouped into lines: consecutive runs sharing one `origin_y`.

    **A line, because a block never crosses a baseline.** `locate` searches the blocks of the
    reading-order cut, and the cut joins the next ink along ONE baseline — so a citation-length
    quote that spans two drawn lines is not an occurrence, whatever it looks like in a rendering.
    Building candidates per line rather than from an arbitrary run offset is what makes the first
    candidate usually the answer: an offset spread blindly across the runs lands mid-line, runs
    past the line's end, and is refused.

    This is the instrument PROPOSING, not deciding: every candidate below is handed to `locate`
    and kept only if the engine found it. The block rule is not reimplemented here, and could not
    be — a line is not always a block, because a table's cell and a letter-spaced title are not.
    """
    out = []
    for y, text in placed_runs(path):
        if out and out[-1][0] == y:
            out[-1][1].append(text)
        else:
            out.append((y, [text]))
    return [(y, "".join(texts)) for y, texts in out]


def realistic_quote(path, work):
    """A citation-length quote **the engine agrees occurs**, taken from the document's own lines.

    **No document in `fixtures/gate` has a single run 60 scalars long** — these producers shred a
    line into many runs, `nist-sp-800-218`'s first three being `NI`, `S` and `T`, which is the
    whole reason a block rather than a run is the unit `locate` searches. So a citation-length
    quote necessarily spans runs, and the instrument cannot lift one from a single node.

    Each candidate is one line's own text, truncated to `QUOTE_SCALARS`, and the count of
    candidates tried is recorded: a line that is not a block — a table row, a letter-spaced
    title — is refused, and how many of those a document leads with is worth knowing.
    """
    candidates = [text for _, text in lines(path) if len(text) >= QUOTE_SCALARS]
    attempts = 0
    for candidate in candidates[:16]:
        attempts += 1
        found = locate(path, candidate[:QUOTE_SCALARS], work)
        if found["occurrences"]:
            return candidate[:QUOTE_SCALARS], attempts, found
    return None, attempts, None


def locate(repr_path, quote, work):
    """One `locate` call, with its wall clock and the engine's own duration."""
    quote_file = work / "quote.txt"
    quote_file.write_bytes(quote.encode("utf-8"))
    started = time.monotonic()
    completed = run(
        ["--diagnostics", "locate", str(repr_path), "--quote-file", str(quote_file)]
    )
    wall_ms = (time.monotonic() - started) * 1000
    if completed.returncode != 0:
        raise SystemExit(
            f"locate exited {completed.returncode}: "
            f"{completed.stderr.decode('utf-8', 'replace')[:400]}"
        )
    artifact = json.loads(completed.stdout)
    engine_us = None
    for line in completed.stderr.decode("utf-8", "replace").splitlines():
        try:
            engine_us = json.loads(line).get("wall_micros")
        except json.JSONDecodeError:
            continue
    occurrences = artifact["occurrences"]
    return {
        "occurrences": len(occurrences),
        "parts": sum(len(o["parts"]) for o in occurrences),
        "synthesized": sum(o["synthesized"] for o in occurrences),
        "bytes": len(completed.stdout.rstrip(b"\n")),
        "searched": artifact["searched"],
        "wall_ms": round(wall_ms, 1),
        "engine_ms": round(engine_us / 1000, 1) if engine_us else None,
    }


def main():
    work = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "/tmp/locate-s3")
    work.mkdir(parents=True, exist_ok=True)
    version = run(["--version"]).stdout.decode().strip()
    print(f"binary: {BINARY} ({version})")
    print(f"gate:   {GATE}")
    print(f"work:   {work}\n")

    rows = []
    for name in DOCUMENTS:
        path = representation(name, work)
        quote, attempts, found = realistic_quote(path, work)
        if quote is None:
            raise SystemExit(
                f"{name}: no citation-length quote from its own runs occurs in one block, "
                f"after {attempts} candidate(s)"
            )
        row = {"document": name, "quote": quote, "quote_attempts": attempts}
        row["realistic"] = found
        row["short"] = locate(path, "the", work)
        rows.append(row)
        print(
            f"{name}: quote found on candidate {attempts} · "
            f"realistic {row['realistic']['occurrences']} occ "
            f"({row['realistic']['wall_ms']} ms wall, {row['realistic']['engine_ms']} ms engine) · "
            f"`the` {row['short']['occurrences']} occ "
            f"({row['short']['wall_ms']} ms wall, {row['short']['engine_ms']} ms engine)",
            flush=True,
        )

    (work / "results.json").write_text(json.dumps(rows, indent=1, ensure_ascii=False) + "\n")
    print(f"\nwrote {work / 'results.json'}")

    worst = max(rows, key=lambda r: r["short"]["occurrences"])
    print(
        f"worst short-quote document: {worst['document']} "
        f"({worst['short']['occurrences']} occurrences, {worst['short']['bytes']} artifact bytes)"
    )


if __name__ == "__main__":
    main()
