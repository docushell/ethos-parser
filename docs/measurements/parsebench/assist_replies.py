"""Stored LightOnOCR-3-0.8B answers as replies files for the product assist lane (decision #147).

Each answer `lightonocr_answers.py` stores under `<group dir>/<file>` becomes `<group dir>__<stem>.json` in
OUT_DIR: the replies file `extract --assist-answers` reads (`crates/ethos-parser-core/src/assist.rs`), holding
page 1, the page those answers are of, with its reply and finish reason, under the model and revision decision
#123 asked. `ethos_bench.py` reads the directory under `ETHOS_BENCH_ASSIST_PRODUCT`.

Each page is drawn again by `ethos-parser-assist render`, and the file stamps that renderer only where the new
PNG is byte for byte the one the model read (KEPT_PNGS/<group dir>/<stem>.png) at the stored dpi and size; any
other page, and a page whose answer is an error, gets no file and is listed. `points` is the rendered size in
centipoints, read back from the pixels (`render` prints the dpi and the pixel size, not the points): within
36 / dpi points of hayro's, well inside the 1 pt the record checks it against.

Run from the ParseBench checkout:
  python3 -I assist_replies.py ANSWERS.json KEPT_PNGS OUT_DIR ASSIST_BIN
"""

import hashlib
import json
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

MODEL = {"id": "lightonai/LightOnOCR-3-0.8B", "version": "36d8636925327c671bd8e479c517a004f7566bd9"}
RENDERER = "ethos-parser-assist 0.1.0; hayro 0.7.1; pages-400dpi-5mp-v1"
REQUEST = "lightonocr-3-grounding-v1"


def sha256(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def convert(key: str, answer: dict, kept: Path, out: Path, assist: str, scratch: Path):
    """Write `key`'s replies file; or say why there is none."""
    if answer.get("error"):
        return f"answer is an error: {answer['error'][:120]}"
    group, name = key.split("/", 1)
    pdf = Path("data/docs") / key
    png = scratch / f"{group}__{Path(name).stem}.png"
    proc = subprocess.run([assist, "render", str(pdf), "1", str(png)], capture_output=True, text=True)
    if proc.returncode:
        return f"render exit {proc.returncode}: {proc.stderr.strip()[-120:]}"
    dpi, width, height = map(int, proc.stdout.split())
    pixels = png.read_bytes()
    png.unlink()
    if [dpi, width, height] != [int(answer["dpi"]), *json.loads(str(answer["size"]))]:
        return f"render {dpi} dpi {width} x {height}, answered {answer['dpi']} dpi {answer['size']}"
    if pixels != (kept / group / f"{Path(name).stem}.png").read_bytes():
        return "render differs from the PNG the model read"
    page = {"page": 1, "points": [round(width * 7200 / dpi), round(height * 7200 / dpi)], "dpi": dpi,
            "width": width, "height": height, "pixels_sha256": sha256(pixels),
            "finish_reason": answer["finish_reason"], "reply": answer["raw"]}
    replies = {"ethos_assist_answers": 1, "source_sha256": sha256(pdf.read_bytes()), "renderer": RENDERER,
               "request": REQUEST, "model": MODEL, "pages": [page]}
    (out / f"{group}__{Path(name).stem}.json").write_text(
        json.dumps(replies, sort_keys=True, separators=(",", ":"), ensure_ascii=False))
    return None


def main() -> None:
    answers, kept, out, assist = json.loads(Path(sys.argv[1]).read_text()), Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4]
    out.mkdir(parents=True, exist_ok=True)
    keys = sorted(answers)
    with tempfile.TemporaryDirectory() as scratch, ThreadPoolExecutor(8) as pool:
        why = list(pool.map(lambda k: convert(k, answers[k], kept, out, assist, Path(scratch)), keys))
    left = [(k, w) for k, w in zip(keys, why) if w]
    print(f"{len(keys) - len(left)} replies files written; {len(left)} pages left out")
    for k, w in left:
        print(f"  {k}: {w}")


if __name__ == "__main__":
    main()
