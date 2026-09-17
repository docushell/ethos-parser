#!/usr/bin/env python3
"""Save one `classify` artifact per corpus document. Same call the CLI help documents."""
from __future__ import annotations
import glob, json, os, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
BIN = os.environ["ETHOS_PARSER_BIN"]
OUT = Path(__file__).resolve().parent / "classify.json"
def one(path: str):
    p = subprocess.run([BIN, "classify", path], capture_output=True)
    stem = Path(path).stem
    if not p.stdout:
        return stem, {"exit": p.returncode, "err": p.stderr.decode(errors="replace")[:200]}
    c = json.loads(p.stdout)
    pg = c["pages"]
    return stem, {
        "exit": p.returncode,
        "page_count": c["page_count"],
        "layout_reasons": c.get("layout_reasons", []),
        "ocr_reasons": c.get("ocr_reasons", []),
        "needs_attention": c.get("needs_attention"),
        "rectangles": sum(p_.get("rectangles", 0) for p_ in pg),
        "path_operators": sum(p_.get("path_operators", 0) for p_ in pg),
        "image_count": sum(p_.get("image_count", 0) for p_ in pg),
        "text_bytes": sum(p_.get("text_bytes", 0) for p_ in pg),
        "text_operators": sum(p_.get("text_operators", 0) for p_ in pg),
        "annotations": sum(p_.get("annotations", 0) for p_ in pg),
        "per_page_layout_reasons": [p_.get("layout_reasons", []) for p_ in pg],
    }
if __name__ == "__main__":
    files = sorted(glob.glob(os.path.join(sys.argv[1], "*.pdf")))
    started = time.time()
    with ThreadPoolExecutor(4) as pool:
        rows = dict(pool.map(one, files))
    OUT.write_text(json.dumps(rows, indent=1))
    print(f"{len(rows)} classified in {time.time()-started:.0f}s, bin={BIN}")
    import collections
    print("layout_reasons:", collections.Counter(r for v in rows.values() for r in v.get("layout_reasons", [])))
    print("ocr_reasons:   ", collections.Counter(r for v in rows.values() for r in v.get("ocr_reasons", [])))
