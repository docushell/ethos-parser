"""Run ParseBench against a local ethos-parser binary.

Usage: ETHOS_PARSER_BIN=/path/to/ethos-parser uv run python ethos_bench.py run ethos_markdown --test

Registers through parse_bench.extensions, so the upstream tree stays unmodified.
A refusal (non-zero exit) is a ProviderPermanentError and scores zero, as the benchmark intends.

Text dimensions read `markdown`. Visual Grounding reads the engine's own units with their boxes:
its `layout_unit`s (each `ground` element where a run carries none), each table and each image,
labelled only from what the record states — a tagged role path, an `inferred_heading` run, a
detected table, a drawn image — and `Text` otherwise. Nothing here infers a role or a unit the
engine did not.
"""

import json
import os
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
    for node in rep["nodes"]:
        rect = ((node.get("native_locator") or {}).get("pdf_image") or {}).get("rect") or {}
        if node["kind"] == "image" and rect.get("state") == "painted":
            items.append({"page": node["native_locator"]["pdf_image"]["page"], "bbox": rect["value"], "label": "Picture", "text": ""})
    pages = [{"index": p["index"], "width": p["width"], "height": p["height"]} for p in rep["pages"]]
    return {"pages": pages, "items": items}


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
            extract_path.write_text(_run(binary, "extract", source))
            artifact = json.loads(_run(binary, "markdown", str(extract_path)))
            try:
                grounding = json.loads(_run(binary, "ground", str(extract_path)))
            except ProviderPermanentError:
                grounding = {}
            layout = _layout(json.loads(extract_path.read_text()), grounding)
        completed_at = datetime.now()
        raw = {k: v for k, v in artifact.items() if k != "anchor_map"}
        raw["layout"] = layout
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
        layout = raw_result.raw_output.get("layout") or {"pages": [], "items": []}
        layout_pages = []
        for page in layout["pages"]:
            width, height = float(page["width"]), float(page["height"])
            items = []
            for item in layout["items"]:
                if item["page"] != page["index"]:
                    continue
                x0, y0, x1, y1 = item["bbox"]
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

if __name__ == "__main__":
    sys.exit(main())
