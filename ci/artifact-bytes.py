#!/usr/bin/env python3
"""Digest every artifact this engine can emit, so two trees can be diffed for byte change.

    ci/artifact-bytes.py                       # every fixture, every artifact type
    ci/artifact-bytes.py > before.sha256       # record
    ci/artifact-bytes.py --check before.sha256 # compare, exit 1 on any difference
    ci/artifact-bytes.py --small               # skip the large gate PDFs
    ci/artifact-bytes.py --without nist-sp-800-53Ar5   # leave out one gate PDF; repeatable

    # the no-byte-change proof, the shape ci/code-lines.py already uses
    ci/artifact-bytes.py > /tmp/a && git stash && cargo build --release --locked \\
      && ci/artifact-bytes.py > /tmp/b && git stash pop && diff /tmp/a /tmp/b

# Why this exists

`representation.rs`'s `the_payload_cache_splice_is_the_full_serialization_by_another_route` is the
only place in the workspace where the warm splice route and the cold full route are compared, and
it runs on a one-page, one-node, one-box fixture. That is enough to catch a key spelled wrong and
not enough to catch anything that depends on scale, on a format, or on a field only some documents
carry. A change to the emit path can be byte-perfect on that fixture and wrong on a 61,739-node
tagged PDF or on an office document with no pages at all.

`ci/bench.py`'s size column is not the missing check either: it compares byte COUNT, so a
permutation of the five envelope members — exactly the defect canonical ordering exists to prevent
— passes it silently.

# What it covers, and why the coverage is the point

Four artifact types over every fixture in the tree: `extract` from source bytes, then `ground`,
`markdown` and `html` from that extract. `extract` is the only one that reads a document; the other
three are projections and take the representation. So a single pass exercises the emit path for
PDF and for all eight office formats — DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF, EPUB — plus the
page-less shape, which no PDF can reach.

A command that legitimately refuses an input (a `ground` of a representation with nothing
groundable, say) is recorded as a refusal rather than skipped: a run that silently dropped rows
would compare clean against a run that dropped different ones.

# It proves byte identity only AT EQUAL VERSION

`parser_version` and `profile_sha256` are inside the hashed payload, so every digest here moves on
a version bump whether or not a byte of behaviour changed. That is the identity mechanism working.
It also means this file answers exactly one question — *did this code change alter the output?* —
and answers it only when both sides are built at the same version.

# It runs on Windows too

CI's `cross-os-digests` job runs this on Linux, macOS and Windows and requires the three lists to
be byte-identical. So what it writes must not depend on the host: the binary carries `.exe` on
Windows, the null device is `os.devnull` rather than `/dev/null`, inputs are sorted by
case-sensitive path component where Windows pathlib would ignore case, and the list is written with
`\\n` line endings, where Python on Windows would otherwise translate each one to `\\r\\n`.

Two host differences remain. Glob matching ignores case on Windows too, so a fixture whose name
differed from `*.pdf` or `document.pdf` only by case would be read there and not on Linux; none
does. And a crash is recorded as the host reports it, a negative signal number on Linux and macOS
and an NTSTATUS above 255 on Windows, so the same crash on every system still differs. CI's
identity step therefore names any such row as a crash before it compares the lists.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import pathlib
import subprocess
import sys
from typing import Iterable

ROOT = pathlib.Path(__file__).resolve().parent.parent
BINARY = ROOT / "target" / "release" / ("ethos-parser.exe" if os.name == "nt" else "ethos-parser")
GATE = ROOT / "fixtures" / "gate"
PROJECTIONS = ("ground", "markdown", "html")
CHUNK = 1 << 20


def ordered(paths: Iterable[pathlib.Path]) -> list[pathlib.Path]:
    """Sorted by case-sensitive path component on every host, which is how POSIX pathlib sorts.

    Windows pathlib ignores case, so a fixture named `Zeta` beside `alpha` would move rows there
    and nowhere else.
    """
    return sorted(paths, key=lambda path: path.parts)


def inputs(small: bool, without: frozenset[str]) -> list[pathlib.Path]:
    """Every document in the tree, in a fixed order so two runs' lines align."""
    found: list[pathlib.Path] = []
    if not small:
        found += ordered(path for path in GATE.glob("*.pdf") if path.stem not in without)
    found += ordered((ROOT / "fixtures" / "engine").glob("*/document.pdf"))
    for path in ordered((ROOT / "fixtures" / "office").glob("*/*")):
        if path.is_file() and path.suffix not in {".py", ".md", ".json"}:
            found.append(path)
    return found


def run(args: list[str], stdin_path: pathlib.Path | None = None) -> tuple[str, int]:
    """`(sha256 of stdout, exit code)`, streamed so a gigabyte artifact costs a megabyte here."""
    with open(stdin_path, "rb") if stdin_path else open(os.devnull, "rb") as source:
        proc = subprocess.Popen(
            [str(BINARY), *args],
            stdin=source,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        digest = hashlib.sha256()
        assert proc.stdout is not None
        while chunk := proc.stdout.read(CHUNK):
            digest.update(chunk)
        return digest.hexdigest(), proc.wait()


def emit(small: bool, without: frozenset[str], workdir: pathlib.Path) -> list[str]:
    lines: list[str] = []
    for source in inputs(small, without):
        name = source.relative_to(ROOT).as_posix()
        # The extract is kept on disk because the three projections consume it, not the document.
        extract = workdir / (name.replace("/", "_") + ".extract.json")
        with open(extract, "wb") as sink:
            code = subprocess.run(
                [str(BINARY), "extract", str(source)], stdout=sink, stderr=subprocess.DEVNULL
            ).returncode
        digest = hashlib.sha256()
        with open(extract, "rb") as handle:
            while chunk := handle.read(CHUNK):
                digest.update(chunk)
        lines.append(f"{digest.hexdigest()}  {name}  extract  exit={code}")
        if code != 0:
            # A refusal is recorded, never skipped — see the module docstring.
            for command in PROJECTIONS:
                lines.append(f"{'-' * 64}  {name}  {command}  exit=-")
            continue
        for command in PROJECTIONS:
            got, status = run([command, str(extract)])
            lines.append(f"{got}  {name}  {command}  exit={status}")
    return lines


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", type=pathlib.Path, help="compare against a recorded digest list")
    ap.add_argument("--small", action="store_true", help="skip the large gate PDFs")
    ap.add_argument(
        "--without",
        action="append",
        default=[],
        metavar="STEM",
        help="leave out the gate PDF with this file stem; repeatable",
    )
    args = ap.parse_args()

    if not BINARY.exists():
        print(
            f"engine: no release binary at {BINARY}. Run `cargo build --release --locked` first.",
            file=sys.stderr,
        )
        return 2

    # A name that matches nothing leaves nothing out, and would say nothing about it.
    unknown = sorted(set(args.without) - {path.stem for path in GATE.glob("*.pdf")})
    if unknown:
        print(f"engine: --without names no gate PDF: {', '.join(unknown)}", file=sys.stderr)
        return 2

    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        lines = emit(args.small, frozenset(args.without), pathlib.Path(tmp))

    # `\n` on every host. Python on Windows writes `\r\n` for each `\n` by default, and a digest
    # list is compared as bytes across operating systems.
    sys.stdout.reconfigure(newline="\n")
    if not args.check:
        print("\n".join(lines))
        return 0

    want = [line for line in args.check.read_text().splitlines() if line.strip()]
    if want == lines:
        print(f"artifact bytes: identical across {len(lines)} artifacts")
        return 0

    print(f"artifact bytes: {len(lines)} artifacts, differences below", file=sys.stderr)
    for old, new in zip(want, lines):
        if old != new:
            print(f"  - {old}\n  + {new}", file=sys.stderr)
    if len(want) != len(lines):
        print(f"  row count moved: {len(want)} -> {len(lines)}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
