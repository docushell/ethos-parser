"""ethos-parser adapter for `opendataloader-project/opendataloader-bench`.

Copy into that harness's `src/` and register as **`ethos-parser`** — see the README beside this
file. **Not `ethos`**: that name is already taken in the harness by an adapter shelling out to
`ethos doc parse`, the Ethos *verifier* CLI, which is a different tool by the same authors.
Overwriting it silently replaces a prior run's results.

# What this harness can and cannot see

The adapter contract is one function returning Markdown per document. Everything ethos-parser is
built for — the versioned output contract, `profile_sha256`, declared coordinate systems, integer
centipoint quanta, typed absence, derivation classes, the anchor map that makes a quote bindable —
is flattened to a string before any metric runs. **This benchmark scores ethos-parser as a Markdown
emitter**, which is not the axis it was designed for, and saying so once is cheaper than inferring
it from a number.

# It emits the projection verbatim, and an earlier version did not

Until 0.44.0 the Markdown projection put every text run in its own block: `nist-sp-800-207` came
out as 68 112 blocks averaging two characters, and `Yarrow` as `Y` `arr` `o` `w`. NID normalizes
whitespace, so that read as `Y arr o w` and the metric scored a typesetting decision as a
reading-order error.

This adapter therefore used to rebuild lines itself, from `origin_y` and `origin_x`. **That variant
is deleted rather than kept behind a flag, because 0.44.0 made it strictly worse** — measured on the
same 200 documents:

    projection verbatim (0.44.0)   NID 0.8471   TEDS 0.1038
    adapter joining lines          NID 0.8440   TEDS 0.0000

The engine now assembles blocks from the document's own marked-content sequences, which the adapter
could not see, and it keeps the GFM tables that joining by baseline flattened to nothing. An adapter
that second-guesses the projection is measuring the adapter.
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path
from typing import Iterable


def _binary() -> str:
    """The engine, from `ETHOS_PARSER_BIN` or a build in this checkout."""
    if pinned := os.environ.get("ETHOS_PARSER_BIN"):
        return pinned
    # …/docs/measurements/opendataloader-bench/this.py -> repository root
    root = Path(__file__).resolve().parents[3]
    for candidate in (root / "target/release/ethos-parser", root / "target/debug/ethos-parser"):
        if candidate.is_file():
            return str(candidate)
    raise ImportError(
        "no ethos-parser binary. Run `cargo build --release`, or set ETHOS_PARSER_BIN. "
        "A missing binary is an ImportError so the harness reports it rather than scoring zeros."
    )


_BIN = _binary()


def _run(args: list[str]) -> bytes:
    p = subprocess.run([_BIN, *args], capture_output=True)
    if p.returncode != 0:
        raise RuntimeError(f"ethos-parser {args[0]} exited {p.returncode}: {p.stderr.decode()[:400]}")
    return p.stdout


def to_markdown(document_paths: Iterable[Path], input_path: Path, output_dir: Path) -> None:
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    for pdf in document_paths:
        pdf = Path(pdf)
        try:
            representation = _run(["extract", str(pdf)])
            # `markdown` takes a path, not stdin, so the representation goes through a temp file.
            with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as fh:
                fh.write(representation)
                repr_path = fh.name
            try:
                md = json.loads(_run(["markdown", repr_path]))["markdown"]
            finally:
                os.unlink(repr_path)
        except Exception as exc:
            # A document that fails produces an empty prediction and scores zero, which is the
            # honest outcome — the harness must not silently omit it from the denominator.
            print(f"  ethos-parser FAILED {pdf.name}: {exc}")
            md = ""
        (output_dir / f"{pdf.stem}.md").write_text(md, encoding="utf-8")
