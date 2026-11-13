"""Run ParseBench against a local ethos-parser binary.

Usage: ETHOS_PARSER_BIN=/path/to/ethos-parser uv run python ethos_bench.py run ethos_markdown --test

Registers through parse_bench.extensions, so the upstream tree stays unmodified.
A refusal (non-zero exit) is a ProviderPermanentError and scores zero, as the benchmark intends.

Text dimensions read `markdown`, its pipe tables converted to HTML as every local provider's are —
except a table holding a merged cell, which GFM cannot carry: that one is the engine's own HTML
projection of it, `rowspan` and `colspan` kept. Visual Grounding reads the engine's own units with their boxes:
its `layout_unit`s (each `ground` element where a run carries none), each table, each image and
each figure region, placed on the page's visible box where the record states one (decision #62),
labelled only from what the record states — a run's page `furniture`, a tagged
role path, an `inferred_heading` run, a detected table, a drawn image, a figure region (decision
#46) — and `Text` otherwise. Nothing here infers a role or a unit the engine did not. An image file,
which `ground` refuses because Ethos's `ethos.grounding.v1` admits no image media type, is grounded
from `extract`'s own output: its runs, with the boxes and layout units `extract` states (decision #124).

`ETHOS_BENCH_OCR=engine` (decision #120) runs `extract --ocr`: the engine's own OCR lane reads every page
that shows no text and places one image (`docs/35-OCR-SCOPE.md`), and needs `ethos-parser-ocr` beside the
binary or named by `ETHOS_PARSER_OCR`.

`ETHOS_BENCH_OCR=1` (decision #45) adds an OCR pass outside the engine, which is never the default:
where ethos reads no text from a PDF, or refuses it because no font states its characters, the
pages are rasterized (Ghostscript, 300 dpi) and read by Tesseract (`ETHOS_BENCH_OCR_LANG`, default
`eng`) into a PDF whose invisible text layer ethos then extracts; an image file ethos does not take
goes to Tesseract directly. Results under it are "ethos + OCR pass", reported beside the
engine-only ones. Needs `gs`, `tesseract` and `qpdf` on PATH.

`ETHOS_BENCH_ASSIST=<answers.json>` (decision #123) adds an assist lane outside the engine, which is
never the default: a model's reading of each page, stored by `lightonocr_answers.py`, is put into the
Markdown as the block at the end says. Results under it are "ethos-parser + assist (LightOnOCR-3-0.8B)",
reported beside the engine-only ones; what the model wrote is not the engine's evidence. ParseBench's
LightOnOCR-3 provider imports `aiohttp`, which its project does not declare: run with
`uv run --with aiohttp`.

`ETHOS_BENCH_LEDGER=<dir>` (decision #121) reuses scores: an example whose output and test case are
byte-identical to one scored before takes the stored result instead of being scored again. The
scores are the same; a run that changes a few pages scores in under a minute instead of five. The
ledger block at the end says what the key holds.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

from parse_bench.cli import main
from parse_bench.evaluation.layout_adapters.adapters import LiteParseLayoutAdapter
from parse_bench.evaluation.layout_label_mappers.mappers import LiteParseLabelMapper
from parse_bench.extensions import (
    register_layout_adapter,
    register_layout_label_mapper,
    register_pipeline,
    register_provider,
)
from parse_bench.inference.providers.base import Provider, ProviderConfigError, ProviderPermanentError
from parse_bench.inference.providers.parse.liteparse import LiteParseProvider
from parse_bench.schemas.parse_output import LayoutItemIR, LayoutSegmentIR, PageIR, ParseLayoutPageIR, ParseOutput
from parse_bench.schemas.pipeline import PipelineSpec
from parse_bench.schemas.pipeline_io import InferenceRequest, InferenceResult, RawInferenceResult
from parse_bench.schemas.product import ProductType

# A tagged role path's last element, as the author declared it, to the benchmark's label.
_ROLE_LABEL = {"Title": "Title", "LI": "List-item", "LBody": "List-item", "Lbl": "List-item", "Caption": "Caption"}
_ROLE_LABEL.update({f"H{n}": "Section-header" for n in range(1, 7)} | {"H": "Section-header"})
# A run's page furniture, as the record states it (decision #41), to the benchmark's label.
_FURNITURE_LABEL = {"header": "Page-header", "footer": "Page-footer"}


def _run(binary: str, *args: str) -> str:
    proc = subprocess.run([binary, *args], capture_output=True, text=True)
    if proc.returncode != 0:
        raise ProviderPermanentError(f"ethos-parser exit {proc.returncode}: {proc.stderr.strip()[:500]}")
    return proc.stdout


def _layout(extract: dict, grounding: dict) -> dict:
    """The engine's units with their boxes and the labels the record states, per page."""
    rep = extract["representation"]
    nodes = {n["id"]: n for n in rep["nodes"]}
    page_of = {p["id"]: p["index"] for p in rep["pages"]}

    def label_of(run_ids: list[str]) -> str:
        # Page furniture first (decision #41): a running head the heading rule also read stays the
        # page's header, which is what the line is where it stands.
        for rid in run_ids:
            furniture = (nodes.get(rid, {}).get("attributes", {}).get("text_run") or {}).get("furniture")
            if furniture:
                return _FURNITURE_LABEL[furniture]
        for rid in run_ids:
            node = nodes.get(rid, {})
            tagged = (node.get("structural_locator") or {}).get("pdf_tagged") or {}
            for role in reversed(tagged.get("role_path") or []):
                if role in _ROLE_LABEL:
                    return _ROLE_LABEL[role]
            if node.get("attributes", {}).get("text_run", {}).get("inferred_heading"):
                return "Section-header"
        return "Text"

    # The unit is the engine's own `layout_unit` where its record carries one (decision #38): the
    # `ground` elements whose runs share a page and a unit are one item, boxed by their union. Where
    # a run carries none, the unit is the `ground` element itself — one geometric block, which on an
    # untagged page is one baseline's ink. This adapter groups by what the record states and makes
    # no unit of its own; grouping by the leading-gap `block` was measured and scored lower (element
    # pass 0.174 against 0.220), since a block often holds several paragraphs and a heading.
    runs_of: dict[str, list[str]] = {}
    for span in grounding.get("spans", []):
        runs_of.setdefault(span["element"], []).append(span["id"])

    def unit_of(run_ids: list[str]):
        for rid in run_ids:
            unit = ((nodes.get(rid) or {}).get("attributes", {}).get("text_run") or {}).get("layout_unit")
            if unit is not None:
                return unit
        return None

    # A table is one item, boxed by its own geometry, below; its runs are not items again. Each run a
    # reported table holds would otherwise come back as a `Text` item of its own, so every table
    # found was counted twice, once as the table and once as its cells' lines.
    tabled = {
        nid
        for table in rep.get("tables", [])
        if (table.get("geometry") or {}).get("state") == "measured"
        for cell in table.get("cells", [])
        for nid in cell.get("node_ids", [])
    }

    groups: list[dict] = []
    for element in grounding.get("elements", []):
        run_ids = runs_of.get(element["id"], [])
        if run_ids and all(rid in tabled for rid in run_ids):
            continue
        page, unit = page_of.get(element["page"], 1), unit_of(run_ids)
        last = groups[-1] if groups else None
        if unit is not None and last is not None and last["key"] == (page, unit):
            x0, y0, x1, y1 = last["bbox"]
            a, b, c, d = element["bbox"]
            last["bbox"] = [min(x0, a), min(y0, b), max(x1, c), max(y1, d)]
            last["text"] += " " + element.get("text", "")
            last["runs"] += run_ids
        else:
            key = (page, unit) if unit is not None else None
            groups.append({"key": key, "page": page, "bbox": list(element["bbox"]), "text": element.get("text", ""), "runs": list(run_ids)})
    items = [
        {"page": g["page"], "bbox": g["bbox"], "label": label_of(g["runs"]), "text": g["text"]} for g in groups
    ]
    for table in rep.get("tables", []):
        geometry = table.get("geometry") or {}
        if geometry.get("state") == "measured":
            text = " ".join(c.get("text", "") for c in table.get("cells", []))
            items.append({"page": page_of.get(table.get("page"), 1), "bbox": geometry["value"], "label": "Table", "text": text})
    # A picture carries the text drawn inside it (decision #97): the record names the runs whose
    # origin its box holds, and the picture's text is that of each unit with a run it names. The
    # scorer reads an item with no text as no text at all, so a picture whose ground truth lists
    # its labels failed however exactly it was placed. Measured against a unit counted only when
    # every run of it is named, or half: grounding 0.5805 against 0.5798 and 0.5802.
    def text_inside(named: list[str]) -> str:
        named = set(named)
        return " ".join(g["text"] for g in groups if any(r in named for r in g["runs"]))

    for node in rep["nodes"]:
        rect = ((node.get("native_locator") or {}).get("pdf_image") or {}).get("rect") or {}
        if node["kind"] == "image" and rect.get("state") == "painted":
            inside = text_inside(node["attributes"]["image"]["runs_inside"])
            items.append({"page": node["native_locator"]["pdf_image"]["page"], "bbox": rect["value"], "label": "Picture", "text": inside})
    for figure in rep.get("figures", []):
        geometry = figure.get("geometry") or {}
        if geometry.get("state") == "measured":
            inside = text_inside(figure["runs_inside"])
            items.append({"page": page_of.get(figure.get("page"), 1), "bbox": geometry["value"], "label": "Picture", "text": inside})
    pages = [{"index": p["index"], "width": p["width"], "height": p["height"], "visible": p.get("visible")} for p in rep["pages"]]
    return {"pages": pages, "items": items}



def _grounding_from_extract(extract: dict) -> dict:
    """Decision #124: an image file's `ground` elements as `extract` states them. `ground` refuses the
    file, since Ethos's grounding schema admits no image media type; each text run with a measured box,
    in reading order, stands in for the element `ground` would have made of it."""
    boxes = {g["node"]: g["presence"]["value"] for g in extract.get("geometry", [])
             if (g.get("presence") or {}).get("state") == "measured"}
    runs = sorted((n for n in extract["representation"]["nodes"] if n["kind"] == "text_run" and n["id"] in boxes),
                  key=lambda n: n["ordinal"])
    elements = [{"id": f"e{k}", "page": n["parent"], "bbox": boxes[n["id"]], "text": n.get("text", "").strip()}
                for k, n in enumerate(runs)]
    return {"elements": elements, "spans": [{"element": f"e{k}", "id": n["id"]} for k, n in enumerate(runs)]}


def _has_text(extract_json: str) -> bool:
    rep = json.loads(extract_json).get("representation", {})
    return any(n.get("kind") == "text_run" and (n.get("text") or "").strip() for n in rep.get("nodes", []))


def _ocr(source: str, scratch: Path, image: bool) -> str:
    """Decision #45: OCR outside the engine, to a PDF ethos reads like any other."""
    lang = os.environ.get("ETHOS_BENCH_OCR_LANG", "eng")
    if image:
        pages = [Path(source)]
    else:
        subprocess.run(["gs", "-q", "-dNOPAUSE", "-dBATCH", "-sDEVICE=png16m", "-r300",
                        f"-sOutputFile={scratch}/p%04d.png", source], check=True, capture_output=True)
        pages = sorted(scratch.glob("p*.png"))
    pdfs = []
    for i, page in enumerate(pages):
        out = scratch / f"ocr{i:04d}"
        subprocess.run(["tesseract", str(page), str(out), "-l", lang, "pdf"], check=True, capture_output=True)
        pdfs.append(f"{out}.pdf")
    merged = scratch / "ocr.pdf"
    subprocess.run(["qpdf", "--empty", "--pages", *pdfs, "--", str(merged)], check=True, capture_output=True)
    return str(merged)


# The assist lane (decision #123), opt in with `ETHOS_BENCH_ASSIST=<answers.json>`. The answers are
# LightOnOCR-3-0.8B's raw `grounding` replies under `<group dir>/<file>`, rendered to Markdown by
# ParseBench's own LightOnOCR-3 provider code, as its leaderboard row is. A page the engine read keeps
# its Markdown and gains the model's table and chart blocks, each with the title or caption block set
# right before it. A page whose Markdown holds under 30% of the distinct words (two letters or more) the model reads there, where the
# model reads at least half as many words as the engine, is the model's page instead: a scan under a
# stray text layer, a script or a hand the engine's OCR does not read, a font whose `/ToUnicode`
# misleads. Grounding reads the engine's boxes, so it does not move; a page the engine refused stays
# refused.
_ASSIST = os.environ.get("ETHOS_BENCH_ASSIST")
if _ASSIST:
    import importlib

    from parse_bench.inference.providers.parse.qwen import QwenProvider

    try:
        _LIGHTON = importlib.import_module("parse_bench.inference.providers.parse.lightonocr-3")
    except ModuleNotFoundError as missing:
        raise SystemExit(f"ETHOS_BENCH_ASSIST: {missing}; run with `uv run --with aiohttp`") from missing
    _ANSWERS = json.loads(Path(_ASSIST).read_text())

    def _words(markdown: str) -> set[str]:
        return {w for w in re.findall(r"\w+", re.sub(r"<[^>]+>", " ", markdown).lower()) if len(w) > 1}

    def _assisted(markdown: str, source: str) -> str:
        path = Path(source)
        answer = (_ANSWERS.get(f"{path.parent.name}/{path.name}") or {}).get("raw")
        if not answer:
            return markdown
        blocks = [
            (b["label"], _LIGHTON._repair_dollar_delimiters(b["text"]))
            for b in _LIGHTON.LightOnOcr3Provider.parse_blocks(answer)
        ]

        def render(chosen: list) -> str:
            text = "\n\n".join(md for md in (_LIGHTON._render_block(label, body) for label, body in chosen) if md)
            return QwenProvider._sanitize_html_attributes(QwenProvider._convert_md_tables_to_html(text)) if text else ""

        page = render(blocks)
        theirs, ours = _words(page), _words(markdown)
        if theirs and len(theirs & ours) / len(theirs) < 0.3 and len(theirs) >= 0.5 * len(ours):
            return page
        kept: list = []
        for k, (label, body) in enumerate(blocks):
            if _LIGHTON._base_label(label) in ("table", "chart"):
                before = blocks[k - 1] if k else None
                if before and _LIGHTON._base_label(before[0]) in ("title", "caption") and before not in kept:
                    kept.append(before)
                kept.append((label, body))
        extra = render(kept)
        return f"{markdown}\n\n{extra}" if extra else markdown


@register_provider("ethos")
class EthosProvider(Provider):
    def run_inference(self, pipeline: PipelineSpec, request: InferenceRequest) -> RawInferenceResult:
        binary = os.environ.get("ETHOS_PARSER_BIN")
        if not binary:
            raise ProviderConfigError("set ETHOS_PARSER_BIN to an ethos-parser binary")
        started_at = datetime.now()
        source = str(Path(request.source_file_path))
        with tempfile.TemporaryDirectory() as scratch:
            extract_path = Path(scratch) / "extract.json"
            if os.environ.get("ETHOS_BENCH_OCR") == "1":
                proc = subprocess.run([binary, "extract", source], capture_output=True, text=True)
                no_font = "unsupported text encoding" in proc.stderr
                image = "unsupported media type" in proc.stderr
                if proc.returncode == 0 and _has_text(proc.stdout):
                    extract_path.write_text(proc.stdout)
                elif proc.returncode == 0 or no_font or image:
                    ocr_dir = Path(scratch) / "ocr"
                    ocr_dir.mkdir()
                    extract_path.write_text(_run(binary, "extract", _ocr(source, ocr_dir, image)))
                else:
                    extract_path.write_text(_run(binary, "extract", source))
            elif os.environ.get("ETHOS_BENCH_OCR") == "engine":
                # Decision #120: the engine's own OCR lane. Every document is read with `--ocr`, and
                # the engine decides page by page: a page that shows no text and places one image is
                # read by `ethos-parser-ocr` (beside the binary, or `ETHOS_PARSER_OCR`); every other
                # page reads exactly as it does without the flag.
                extract_path.write_text(_run(binary, "extract", "--ocr", source))
            else:
                extract_path.write_text(_run(binary, "extract", source))
            artifact = json.loads(_run(binary, "markdown", str(extract_path)))
            html = json.loads(_run(binary, "html", str(extract_path))).get("html", "")
            try:
                grounding = json.loads(_run(binary, "ground", str(extract_path)))
            except ProviderPermanentError:
                grounding = {}
            extract = json.loads(extract_path.read_text())
            media_type = extract["representation"]["source"].get("media_type", "")
            if not grounding.get("elements") and media_type.startswith("image/"):
                grounding = _grounding_from_extract(extract)
            layout = _layout(extract, grounding)
        completed_at = datetime.now()
        raw = {k: v for k, v in artifact.items() if k != "anchor_map"}
        raw["layout"] = layout
        raw["html_tables"] = re.findall(r"<table>.*?</table>", html, re.S)
        return RawInferenceResult(
            request=request,
            pipeline=pipeline,
            pipeline_name=pipeline.pipeline_name,
            product_type=request.product_type,
            raw_output=raw,
            started_at=started_at,
            completed_at=completed_at,
            latency_in_ms=int((completed_at - started_at).total_seconds() * 1000),
        )

    def normalize(self, raw_result: RawInferenceResult) -> InferenceResult:
        markdown = raw_result.raw_output.get("markdown", "")
        # The table scorer reads only HTML tables; every local provider converts its pipe tables,
        # and this is LiteParse's own conversion.
        markdown = LiteParseProvider._convert_md_tables_to_html(markdown)
        # Both projections hold the same tables in the same order; where one holds a merged cell,
        # its HTML projection replaces the converted pipe table GFM flattened the merge out of.
        projected = raw_result.raw_output.get("html_tables") or []
        converted = re.findall(r"<table>.*?</table>", markdown, re.S)
        if len(projected) == len(converted):
            spans = re.compile(r'<t[dh] [^>]*(?:row|col)span="')
            chosen = iter([p if spans.search(p) else c for p, c in zip(projected, converted)])
            markdown = re.sub(r"<table>.*?</table>", lambda _: next(chosen), markdown, flags=re.S)
        if _ASSIST:
            markdown = _assisted(markdown, str(raw_result.request.source_file_path))
        layout = raw_result.raw_output.get("layout") or {"pages": [], "items": []}
        layout_pages = []
        for page in layout["pages"]:
            width, height = float(page["width"]), float(page["height"])
            # The benchmark draws the page it scores: where the record states the box a viewer shows
            # (decision #62), the items are placed on that box, as the page's picture holds them.
            dx = dy = 0.0
            if page.get("visible"):
                vx0, vy0, vx1, vy1 = page["visible"]
                dx, dy, width, height = float(vx0), float(vy0), float(vx1 - vx0), float(vy1 - vy0)
            items = []
            for item in layout["items"]:
                if item["page"] != page["index"]:
                    continue
                x0, y0, x1, y1 = item["bbox"]
                x0, y0, x1, y1 = x0 - dx, y0 - dy, x1 - dx, y1 - dy
                if x1 <= x0 or y1 <= y0:
                    continue
                segment = LayoutSegmentIR(
                    x=x0 / width, y=y0 / height, w=(x1 - x0) / width, h=(y1 - y0) / height, confidence=1.0, label=item["label"]
                )
                items.append(
                    LayoutItemIR(type=item["label"], value=item["text"], md=item["text"], bbox=segment, layout_segments=[segment])
                )
            if items:
                layout_pages.append(
                    ParseLayoutPageIR(page_number=page["index"], width=width / 100, height=height / 100, md="", items=items)
                )
        output = ParseOutput(
            task_type="parse",
            example_id=raw_result.request.example_id,
            pipeline_name=raw_result.pipeline_name,
            pages=[PageIR(page_index=0, markdown=markdown)],
            markdown=markdown,
            layout_pages=layout_pages,
        )
        return InferenceResult(
            request=raw_result.request,
            pipeline_name=raw_result.pipeline_name,
            product_type=raw_result.product_type,
            raw_output=raw_result.raw_output,
            output=output,
            started_at=raw_result.started_at,
            completed_at=raw_result.completed_at,
            latency_in_ms=raw_result.latency_in_ms,
        )


register_pipeline(
    PipelineSpec(pipeline_name="ethos_markdown", provider_name="ethos", product_type=ProductType.PARSE, config={})
)
register_layout_adapter("ethos", priority=90)(LiteParseLayoutAdapter)
register_layout_label_mapper("ethos", priority=90)(LiteParseLabelMapper)

# The score ledger (decision #121), opt in with `ETHOS_BENCH_LEDGER=<dir>`. ParseBench's scorer is a
# function of what it reads, so a result is stored under the SHA-256 of that, and an example whose
# inputs are byte-identical to one scored before takes the stored result instead of being scored
# again. The key is everything the scorer receives — the inference result, the test case, the
# scorer's own arguments, the command line but its output directory, and ParseBench's commit with
# its uncommitted diff — less the run's clock and the four fields every engine version changes,
# which ParseBench never reads. Only a success is stored; a stored result's stats are rebuilt from
# this run's inference, so its latency is this run's. An entry that will not read is scored again,
# and a write that fails stores nothing and keeps the score. Both of ParseBench's paths go through it:
# the worker its process pool runs, and the multi-task path it runs in its own process.
_LEDGER = os.environ.get("ETHOS_BENCH_LEDGER")
if _LEDGER:
    import hashlib

    from parse_bench.evaluation import runner as _runner
    from parse_bench.evaluation.stats import build_operational_stats
    from parse_bench.schemas.evaluation import EvaluationResult

    def _git(*args: str) -> str:
        root = Path(_runner.__file__).resolve().parents[3]
        return subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True, check=True).stdout

    _ARGV = sys.argv[1:]
    if "--output_dir" in _ARGV:
        _ARGV = _ARGV[: _ARGV.index("--output_dir")] + _ARGV[_ARGV.index("--output_dir") + 2 :]
    _SCORER = [_git("rev-parse", "HEAD").strip(), hashlib.sha256(_git("diff", "HEAD").encode()).hexdigest(), *_ARGV]
    _RUN_CLOCK = ("started_at", "completed_at", "latency_in_ms")
    _VERSION_FIELDS = ("parser_version", "profile_sha256", "representation_sha256", "markdown_rule")

    def _ledgered(inference: dict, rest: list, score) -> dict:
        """The stored result for these inputs, or `score()`'s, stored when it succeeded."""
        keyed = {k: v for k, v in inference.items() if k not in _RUN_CLOCK}
        if isinstance(keyed.get("raw_output"), dict):
            keyed["raw_output"] = {k: v for k, v in keyed["raw_output"].items() if k not in _VERSION_FIELDS}
        key = hashlib.sha256(json.dumps([_SCORER, keyed, rest], sort_keys=True, default=str).encode()).hexdigest()
        path = Path(_LEDGER) / key[:2] / f"{key}.json"
        if path.exists():
            try:
                result = json.loads(path.read_text())
            except (OSError, ValueError):  # an entry that will not read is scored again
                result = None
            if result is not None:
                stats = build_operational_stats(InferenceResult.model_validate(inference))
                result["stats"] = [s.model_dump(mode="json") for s in stats]
                return result
        result = EvaluationResult.model_validate(score()).model_dump(mode="json")
        if result["success"]:
            try:
                path.parent.mkdir(parents=True, exist_ok=True)
                part = path.with_name(f"{key}.{os.getpid()}.part")
                part.write_text(json.dumps(result))
                os.replace(part, path)
            except OSError as error:  # a full or read-only ledger stores nothing; the score stands
                print(f"ethos_bench: score ledger not written: {error}", file=sys.stderr)
        return result

    _worker = _runner._evaluate_single_worker

    def _ledgered_worker(inference_result_dict: dict, test_case_dict: dict, *rest):
        def score():
            return _worker(inference_result_dict, test_case_dict, *rest)

        return _ledgered(inference_result_dict, [test_case_dict, *rest], score)

    _single = _runner.EvaluationRunner._evaluate_single

    def _ledgered_single(self, inference_result, test_case, evaluator, eval_mode):
        if eval_mode != "multi_task":
            return _single(self, inference_result, test_case, evaluator, eval_mode)

        def score():
            return _single(self, inference_result, test_case, evaluator, eval_mode).model_dump(mode="json")

        inference = inference_result.model_dump(mode="json")
        return EvaluationResult.model_validate(_ledgered(inference, [test_case.model_dump(mode="json"), eval_mode], score))

    _runner._evaluate_single_worker = _ledgered_worker
    _runner.EvaluationRunner._evaluate_single = _ledgered_single

if __name__ == "__main__":
    sys.exit(main())
