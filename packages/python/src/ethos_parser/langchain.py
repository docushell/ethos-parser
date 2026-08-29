# Copyright 2026 The ethos-parser maintainers
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""LangChain tools over the SDK (v1.2-S4).

# The one thing this slice is for

`docs/12-V12-SCOPE.md` §3, the corollary about where locators travel: **locators live in the
artifact, never in the prose a model reads and edits.** MCP says that with `structuredContent`
versus `content`; LangChain says it with a tool's ``artifact`` versus its ``content``, and the
parser memo §16.7's LangChain row names that split directly. So these tools are
``response_format="content_and_artifact"``, and the whole of S4 is getting the two sides right:

| MCP | here |
| --- | --- |
| ``structuredContent`` | the tool's ``artifact`` — the SDK object, unaltered |
| ``content`` text | the tool's ``content`` — **counts, and nothing a pipeline would bind to** |

A box in ``content`` is a locator a model can edit and then cite. That is the failure this whole
version is arranged to prevent, and it is why the summary strings below are copied from
``ethos-parser-cli/src/mcp.rs`` rather than written afresh: MCP already decided the words, and a second
adapter with a richer sentence is a second thing to keep honest forever.

# It is not a third implementation

Every tool calls :mod:`ethos_parser` — the same functions a shell would reach through the CLI.
Nothing here spawns `ethos-parser`, and nothing here talks to MCP. If a tool returned bytes the SDK
would not, the tool would be wrong.

# There is no LangGraph adapter, deliberately

§16.7 refused one: a `StructuredTool` is already what LangGraph binds, so a graph, a node or a
checkpointer here would be a second surface that adds nothing. This module imports no `langgraph`
package and this slice adds no dependency on one.

# It does not set trust state

No field, and no word in a summary, says `grounded`, `verified`, an evidence tier, a score or a
degree of belief. `docs/07-VERIFY-BOUNDARY.md` is unchanged: this engine validates structure and
binding, and it never verifies a claim. An adapter that attached a trust flag on a model's behalf
would be that violation wearing a framework.

# Installing

``langchain-core`` is an **optional extra**, so ``import ethos_parser`` still pulls nothing:

    pip install 'ethos-parser[langchain]'

Importing this module without it is a named failure, never a silent skip.
"""

import json
import os

from . import extract as _extract
from . import ground as _ground
from . import node_get as _node_get

try:
    from langchain_core.tools import StructuredTool
except ImportError as e:  # pragma: no cover - exercised by the packaging, not the suite
    raise ImportError(
        "`ethos_parser.langchain` needs langchain-core, which is an optional extra so that "
        "`import ethos_parser` pulls nothing. Install it with:\n\n"
        "    pip install 'ethos-parser[langchain]'\n\n"
        "This is a named failure rather than a degraded import: a tools() that returned an "
        "empty list would look like a package with no tools."
    ) from e

__all__ = ["TOOL_SCHEMAS", "tools"]

#: The argument schemas, **verbatim from what `ethos-parser mcp` advertises**.
#:
#: One wire shape across the three adapters, and `tests/test_langchain.py` asserts these against
#: `tools/list` rather than against a reviewer's memory. That is also where the geometry ban is
#: enforced: no `page`, no `bbox`, no `x`/`y`, no row/column pair, in any of them. A caller that
#: needs a node passes the id the engine minted.
#:
#: `node_id` keeps MCP's spelling even though the Node SDK's function parameter is `nodeId`: the
#: tool argument is the wire, and one wire has one name.
TOOL_SCHEMAS = {
    "extract": {
        "type": "object",
        "additionalProperties": False,
        "required": ["path"],
        "properties": {"path": {"type": "string", "description": "Path to a PDF file."}},
    },
    "ground": {
        "type": "object",
        "additionalProperties": False,
        "required": ["representation"],
        "properties": {
            "representation": {
                "description": (
                    "The artifact `extract` returned — the object itself, or a path to "
                    "bytes this engine wrote."
                ),
                "type": ["object", "string"],
            }
        },
    },
    "node_get": {
        "type": "object",
        "additionalProperties": False,
        "required": ["representation", "node_id"],
        "properties": {
            "representation": {
                "description": (
                    "The artifact `extract` returned — the object itself, or a path to "
                    "bytes this engine wrote."
                ),
                "type": ["object", "string"],
            },
            "node_id": {
                "type": "string",
                "description": "A node id copied verbatim from that representation's `nodes`.",
            },
        },
    },
}

_DESCRIPTIONS = {
    "extract": (
        "Read a PDF and return `DocumentRepresentation v0` — the canonical evidence record. "
        "Every locator a later call needs is minted here and travels in the tool artifact; "
        "pass that artifact back rather than composing one."
    ),
    "ground": (
        "Project a representation into `ethos.grounding.v1`. Takes the artifact `extract` "
        "returned; the representation is fingerprint-checked before it is read."
    ),
    "node_get": (
        "Return one node from a representation. `node_id` is an OPAQUE HANDLE: copy it from a "
        "representation this engine returned. It is re-validated against that artifact, and an "
        "id this engine did not mint is an error. There is no way to ask for a node by page or "
        "by position."
    ),
}


def tools():
    """The three tools, ready for ``bind_tools`` or a LangGraph ``ToolNode``.

    :returns: ``[extract, ground, node_get]`` as :class:`~langchain_core.tools.StructuredTool`.
    """
    return [
        StructuredTool.from_function(
            func=func,
            name=name,
            description=_DESCRIPTIONS[name],
            args_schema=TOOL_SCHEMAS[name],
            # The whole slice. Without this the artifact would be stringified into `content`,
            # which is a locator a model can edit and then cite.
            response_format="content_and_artifact",
        )
        for name, func in (
            ("extract", _tool_extract),
            ("ground", _tool_ground),
            ("node_get", _tool_node_get),
        )
    ]


# -----------------------------------------------------------------------------------------
# The three, each `(content, artifact)`
# -----------------------------------------------------------------------------------------
#
# A failure RAISES rather than returning a sentence. LangChain surfaces that as a tool error,
# which is MCP's `isError: true` path in this framework's currency — and the reason is the one
# `12-V12-SCOPE.md` §3 gives: an empty result tells a model its guess was merely unlucky, and an
# error tells it the guess was not admissible. Nothing below catches what the SDK raises.


def _tool_extract(path):
    artifact = _extract(path)
    payload = artifact["representation"]
    summary = "{} page(s), {} node(s). Locators are in the artifact.".format(
        len(payload["pages"]), len(payload["nodes"])
    )
    return summary, artifact


def _tool_ground(representation):
    artifact = _ground(representation)
    # `nodes - elements`, which is the question the summary asks: how many nodes did not become
    # elements. Counting geometry rows instead would re-encode `GeometryPresence::is_groundable`
    # out here, and a count derived from a different question than the one being asked is a count
    # that goes wrong the first time a second absence variant appears. `tests/test_langchain.py`
    # pins this string against the one `ethos-parser mcp` emits, which uses the engine's own
    # `omission.nodes_omitted`.
    omitted = _node_count(representation) - len(artifact["elements"])
    summary = "{} element(s) with a measured box; {} omitted for having none.".format(
        len(artifact["elements"]), omitted
    )
    return summary, artifact


def _tool_node_get(representation, node_id):
    node = _node_get(representation, node_id)
    # The kind, and nothing else. **The id stays in the artifact** — a summary carrying it would
    # be handing the model a handle through the one channel it can rewrite.
    return "1 node, kind `{}`.".format(node["kind"]), node


def _node_count(representation):
    """How many nodes the representation carries, for the omission arithmetic above.

    Accepts what the SDK accepts. A path is read rather than re-projected: `ground` has already
    run and refused anything malformed, so this only ever counts an artifact the engine wrote.
    """
    if isinstance(representation, dict):
        payload = representation.get("representation")
    else:
        with open(os.fspath(representation), "rb") as handle:
            payload = json.load(handle).get("representation")
    return len(payload["nodes"])
