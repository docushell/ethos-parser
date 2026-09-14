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

`docs/history/12-V12-SCOPE.md` §3, the corollary about where locators travel: **locators live in the
artifact, never in the prose a model reads and edits.** MCP says that with `structuredContent`
versus `content`; LangChain says it with a tool's ``artifact`` versus its ``content``, and the
parser memo §16.7's LangChain row names that split directly. So these tools are
``response_format="content_and_artifact"``, and the whole of S4 is getting the two sides right:

| MCP | here |
| --- | --- |
| ``structuredContent`` | the tool's ``artifact`` — the SDK object, unaltered |
| ``content`` text | the tool's ``content`` — **counts, and nothing a pipeline would bind to** |

A box in ``content`` is a locator a model can edit and then cite. That is the failure this whole
version is arranged to prevent, and it is why no summary here is written afresh: MCP already decided
the words, and a second adapter with a richer sentence is a second thing to keep honest forever.
``extract``'s and ``node_get``'s summaries are counts copied from ``ethos-parser-cli/src/mcp.rs`` and
built from the artifact; ``ground``'s is ``ethos-parser mcp``'s own reply text, because it states
facts the artifact does not carry.

# It is not a third implementation

``extract`` and ``node_get`` call :mod:`ethos_parser` — the same functions a shell would reach
through the CLI. ``ground`` asks ``ethos-parser mcp`` for one ``tools/call``, by path and never with
the object, and takes the summary and the artifact from that one reply. It has to: since v2.2-S7 an
element is a block of runs, so no count this adapter could compute equals the engine's omission, and
the schema-limit declarations exist only in the projection. On a refusal it runs ``ethos-parser
ground`` on the same path, so the error raised is the SDK's own. If a tool returned bytes the SDK
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

from . import EngineError, _parse, _representation_path, _run
from . import extract as _extract
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
    # The summary and the artifact both come from `ethos-parser mcp`'s one reply, so the words are
    # MCP's by construction — the omission count, and every schema-limit clause, now and later.
    # `nodes - elements`, which this used to compute, stopped being the omission when an element
    # became a block, and the limit declarations were never in the artifact to count.
    with _representation_path(representation) as path:
        result = _mcp_ground(_mcp_path(path))
        if result.get("isError") is True:
            # A refusal. `ethos-parser ground` on the same bytes raises exactly what `ground()`
            # would — exit status, code and stderr — rather than a sentence reworded here.
            _run(["ground", "--", path])
            raise EngineError(
                "`ethos-parser mcp` refused this representation but `ethos-parser ground` "
                "projected it: {}".format(_mcp_text(result))
            )
    content = result.get("content")
    artifact = result.get("structuredContent")
    if not (
        isinstance(content, list)
        and len(content) == 1
        and isinstance(content[0], dict)
        and isinstance(content[0].get("text"), str)
        and isinstance(artifact, dict)
    ):
        raise EngineError("`ethos-parser mcp` replied to `ground` in a shape this adapter does not know")
    return content[0]["text"], artifact


def _mcp_path(path):
    """The path as ``ethos-parser mcp`` must receive it to open the file ``ethos-parser ground`` opens.

    The CLI gets a path as the bytes the operating system encodes it to, and the server gets a JSON
    string and opens its UTF-8 bytes. Those agree only when the encoded bytes are UTF-8, which is not
    so under a Latin-1 locale. So the string sent is decoded from the bytes the CLI would open, and a
    path whose bytes are not UTF-8 is refused here by name rather than sent as a different file.
    """
    try:
        return os.fsencode(path).decode("utf-8")
    except UnicodeDecodeError as e:
        raise EngineError(
            "this path's bytes are not UTF-8, so `ethos-parser mcp` cannot be handed it as JSON; "
            "call `ethos_parser.ground()` for the artifact instead: {}".format(e)
        ) from e


def _mcp_ground(path):
    """One ``tools/call`` to ``ethos-parser mcp``, by path, and its ``result``.

    No ``initialize``: the server answers a call without one, and each extra reply line would be a
    copy of nothing. The representation is sent **by path, never inline** — the server holds an
    inline argument as a parsed request several times over.

    The reply is one line, parsed whole. It is split on nothing: canonical JSON writes U+2028 and
    U+0085 literally, and a text-mode line split would cut a reply carrying one.
    """
    request = (
        json.dumps(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {"name": "ground", "arguments": {"representation": path}},
            }
        ).encode("ascii")
        + b"\n"
    )
    stdout = _run(["mcp"], stdin=request)
    if not stdout.endswith(b"\n") or stdout.count(b"\n") != 1:
        raise EngineError("`ethos-parser mcp` did not reply with exactly one line to one call")
    envelope = _parse(stdout)
    if not isinstance(envelope, dict):
        raise EngineError("`ethos-parser mcp` replied with something that is not an object")
    if "error" in envelope:
        error = envelope["error"] or {}
        raise EngineError(
            "`ethos-parser mcp` refused the call: {} {}".format(error.get("code"), error.get("message"))
        )
    result = envelope.get("result")
    if envelope.get("id") != 1 or not isinstance(result, dict):
        raise EngineError("`ethos-parser mcp` replied to a call this adapter did not make")
    return result


def _mcp_text(result):
    content = result.get("content")
    if isinstance(content, list) and content and isinstance(content[0], dict):
        return str(content[0].get("text", ""))
    return ""



def _tool_node_get(representation, node_id):
    node = _node_get(representation, node_id)
    # The kind, and nothing else. **The id stays in the artifact** — a summary carrying it would
    # be handing the model a handle through the one channel it can rewrite.
    return "1 node, kind `{}`.".format(node["kind"]), node
