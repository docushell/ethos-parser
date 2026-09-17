#!/usr/bin/env python3
"""Save one `extract` artifact per corpus document, so E2/E3 read the engine's own words.

    ETHOS_PARSER_BIN=<frozen binary> python3 save_artifacts.py <bench>/pdfs

Same invocation the committed census.py makes (`<bin> extract <pdf>`); it keeps the stdout instead
of only counting it. Standard library only.
"""
from __future__ import annotations

import glob
import json
import os
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

BIN = os.environ["ETHOS_PARSER_BIN"]
OUT = Path(__file__).resolve().parent / "artifacts"


def one(path: str) -> tuple[str, int, str]:
    stem = Path(path).stem
    p = subprocess.run([BIN, "extract", path], capture_output=True)
    if p.returncode != 0:
        return stem, p.returncode, p.stderr.decode(errors="replace")[:400]
    (OUT / f"{stem}.json").write_bytes(p.stdout)
    return stem, 0, ""


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    files = sorted(glob.glob(os.path.join(sys.argv[1], "*.pdf")))
    started = time.time()
    with ThreadPoolExecutor(4) as pool:
        rows = list(pool.map(one, files))
    bad = [r for r in rows if r[1] != 0]
    print(f"{len(rows)} documents in {time.time() - started:.0f}s, {len(bad)} failed, bin={BIN}")
    for stem, code, err in bad:
        print(f"  FAILED {stem}: exit {code}: {err[:200]}")
    print(json.dumps({"artifacts": len(rows) - len(bad)}))
