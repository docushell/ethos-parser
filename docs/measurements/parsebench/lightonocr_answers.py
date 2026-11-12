"""LightOnOCR-3-0.8B's answers for `ethos_bench.py`'s assist lane (decision #123), outside the engine.

Each PDF the named groups list is rendered as ParseBench's own `lightonocr_3_vllm_parse` pipeline renders
it — 400 dpi, or the dpi that fits 5,000,000 pixels — but by Ghostscript, and the model is asked
`grounding` with greedy decoding and up to 12,288 tokens. Each raw answer is stored under
`<group dir>/<file>` (ParseBench repeats 37 file names across its directories), and a run resumes from
ANSWERS.json. Run from the ParseBench checkout, in a Python of its own:

  LIGHTON_MLX=<converted model> python lightonocr_answers.py ANSWERS.json GROUP...  # mlx-vlm, Apple silicon
  python lightonocr_answers.py ANSWERS.json GROUP...                                # Transformers
"""

import json
import math
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from PIL import Image

MODEL, REVISION = "lightonai/LightOnOCR-3-0.8B", "36d8636925327c671bd8e479c517a004f7566bd9"
DPI, MAX_PIXELS, MAX_TOKENS = 400, 5_000_000, 12288


def gs(pdf: Path, png: Path, dpi: int, *smooth: str) -> None:
    subprocess.run(["gs", "-q", "-dSAFER", "-dBATCH", "-dNOPAUSE", "-sDEVICE=png16m", f"-r{dpi}", *smooth,
                    "-dFirstPage=1", "-dLastPage=1", f"-sOutputFile={png}", str(pdf)], check=True)


def render(pdf: Path, png: Path) -> tuple[int, list[int]]:
    """The first page at ParseBench's dpi, its size in inches read from a 72 dpi render of it."""
    gs(pdf, png, 72)
    with Image.open(png) as image:
        inches = (image.width / 72) * (image.height / 72)
    dpi = min(DPI, math.floor(math.sqrt(MAX_PIXELS / inches)))
    gs(pdf, png, dpi, "-dTextAlphaBits=4", "-dGraphicsAlphaBits=4")
    with Image.open(png) as image:
        if image.width * image.height > MAX_PIXELS:
            scale = math.sqrt(MAX_PIXELS / (image.width * image.height))
            image.resize((int(image.width * scale), int(image.height * scale)), Image.LANCZOS).save(png)
    with Image.open(png) as image:
        return dpi, [image.width, image.height]


def reader():
    """`infer(png) -> (text, tokens)` on mlx-vlm where LIGHTON_MLX names a converted model, else Transformers."""
    if os.environ.get("LIGHTON_MLX"):
        from mlx_vlm import generate, load
        from mlx_vlm.prompt_utils import apply_chat_template

        model, processor = load(os.environ["LIGHTON_MLX"])
        prompt = apply_chat_template(processor, model.config, "grounding", num_images=1)

        def infer(png: Path) -> tuple[str, int]:
            r = generate(model, processor, prompt, [str(png)], max_tokens=MAX_TOKENS, temperature=0.0, verbose=False)
            return r.text, r.generation_tokens

        return infer
    import torch
    from transformers import AutoProcessor, Qwen3_5ForConditionalGeneration

    device = "cuda" if torch.cuda.is_available() else "mps" if torch.backends.mps.is_available() else "cpu"
    model = Qwen3_5ForConditionalGeneration.from_pretrained(MODEL, revision=REVISION, dtype=torch.bfloat16)
    model = model.to(device).eval()
    processor = AutoProcessor.from_pretrained(MODEL, revision=REVISION)

    def infer(png: Path) -> tuple[str, int]:
        content = [{"type": "image", "path": str(png)}, {"type": "text", "text": "grounding"}]
        inputs = processor.apply_chat_template([{"role": "user", "content": content}], add_generation_prompt=True,
                                               tokenize=True, return_dict=True, return_tensors="pt",
                                               enable_thinking=False).to(model.device)
        with torch.inference_mode():  # the model's generation_config turns the KV cache off; vLLM ignores that
            ids = model.generate(**inputs, max_new_tokens=MAX_TOKENS, do_sample=False, use_cache=True)
        new = ids[0, inputs["input_ids"].shape[1]:]
        return processor.decode(new, skip_special_tokens=True), int(new.shape[0])

    return infer


def main() -> None:
    out_path, groups = Path(sys.argv[1]), sys.argv[2:]
    pdfs = sorted({Path("data") / json.loads(line)["pdf"] for g in groups for line in open(Path("data") / f"{g}.jsonl")})
    done = json.loads(out_path.read_text()) if out_path.exists() else {}
    todo = [pdf for pdf in pdfs if f"{pdf.parent.name}/{pdf.name}" not in done]
    infer = reader() if todo else None
    with tempfile.TemporaryDirectory() as scratch:
        png = Path(scratch) / "page.png"
        for i, pdf in enumerate(todo, 1):
            started = time.perf_counter()
            try:
                dpi, size = render(pdf, png)
                raw, tokens = infer(png)
                entry = {"raw": raw, "tokens": tokens, "dpi": dpi, "size": size}
            except Exception as error:  # noqa: BLE001 - a page that fails is recorded, not fatal
                entry = {"raw": "", "error": repr(error)}
            entry["seconds"] = round(time.perf_counter() - started, 3)
            done[f"{pdf.parent.name}/{pdf.name}"] = entry
            part = out_path.with_suffix(".part")
            part.write_text(json.dumps(done))
            os.replace(part, out_path)
            print(f"{i}/{len(todo)} {pdf.parent.name}/{pdf.name} {entry['seconds']}s {entry.get('tokens')} tokens", flush=True)
    print(f"{len(done)} answers; {sum(1 for v in done.values() if v.get('error'))} errors")


if __name__ == "__main__":
    main()
