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

"""The LangChain tools — v1.2-S4, and the split is the whole slice.

`docs/history/12-V12-SCOPE.md` §3: **locators live in the artifact, never in the prose a model reads and
edits.** In this framework that is ``ToolMessage.artifact`` versus ``ToolMessage.content``, and
these tests are that sentence as executables.

**MCP is the oracle here, not this file's own opinion.** `ethos-parser mcp` already decided both the
argument schemas and the summary wording, so the assertions below compare against what the server
actually advertises and emits rather than against strings retyped from it. A second adapter that
drifted from the first would fail here rather than in a reviewer's memory.
"""

import copy
import json
import subprocess

import pytest

import ethos_parser
from ethos_parser import FingerprintMismatch, NodeNotFound
from ethos_parser.langchain import TOOL_SCHEMAS, tools

#: The same list `mcp.rs`, `mcp_stdio.rs`, the Node SDK and `test_handle_law.py` ban.
BANNED_ARGUMENT_NAMES = [
    "page",
    "bbox",
    "box",
    "rect",
    "x",
    "y",
    "w",
    "h",
    "width",
    "height",
    "row",
    "column",
    "col",
    "span",
    "offset",
    "coords",
    "region",
]

#: No tool result, summary or annotation may say this engine believes anything.
#: `docs/07-VERIFY-BOUNDARY.md`: it validates structure and binding, and never verifies a claim.
TRUST_STATE_WORDS = [
    "grounded",
    "verified",
    "evidence_tier",
    "trusted",
    "trust_score",
    "quality_score",
    "all_evidence_grounded",
]


def mcp(engine_binary, calls):
    """Speak a session to `ethos-parser mcp` and return its results, so MCP can be the oracle."""
    lines = [
        json.dumps({"jsonrpc": "2.0", "id": i, "method": method, "params": params})
        for i, (method, params) in enumerate(calls, start=1)
    ]
    completed = subprocess.run(
        [engine_binary, "mcp"],
        input="\n".join(lines) + "\n",
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    return [json.loads(line)["result"] for line in completed.stdout.splitlines() if line.strip()]


@pytest.fixture(scope="module")
def by_name():
    return {tool.name: tool for tool in tools()}


@pytest.fixture(scope="module")
def representation(fixture_pdf):
    return ethos_parser.extract(fixture_pdf)


def call(tool, args):
    """Invoke as a host does, so the return is a ``ToolMessage`` and not a bare tuple."""
    return tool.invoke({"name": tool.name, "args": args, "id": "call-1", "type": "tool_call"})


# --- the split ------------------------------------------------------------------------------


def test_the_three_tools_are_content_and_artifact(by_name):
    assert sorted(by_name) == ["extract", "ground", "node_get"]
    for tool in by_name.values():
        assert tool.response_format == "content_and_artifact", (
            "without this the artifact is stringified into `content`, which is a locator a model "
            "can edit and then cite"
        )


def test_extract_puts_the_artifact_in_the_artifact(by_name, fixture_pdf):
    message = call(by_name["extract"], {"path": str(fixture_pdf)})
    assert message.artifact == ethos_parser.extract(fixture_pdf), (
        "the artifact must be the object the SDK returned, not a re-serialization of it"
    )
    assert isinstance(message.content, str)


def test_ground_and_node_get_put_the_artifact_in_the_artifact(by_name, representation):
    grounding = call(by_name["ground"], {"representation": representation})
    assert grounding.artifact == ethos_parser.ground(representation)
    assert grounding.artifact["artifact_type"] == "ethos.grounding.v1"

    minted = representation["representation"]["nodes"][0]["id"]
    node = call(by_name["node_get"], {"representation": representation, "node_id": minted})
    assert node.artifact == ethos_parser.node_get(representation, minted)
    assert node.artifact["id"] == minted, "the id lives here, and only here"


# --- `content` carries nothing a pipeline would bind to -----------------------------------------


@pytest.mark.parametrize("pdf_name", ["markdown-two-blocks", "off-page-and-offset-box"])
def test_the_summaries_are_the_ones_mcp_emits(by_name, engine_binary, repo_root, pdf_name):
    """Byte-for-byte against `ethos-parser mcp`, on a document where nothing is omitted and one where
    everything is.

    This is what stops the second adapter inventing a richer sentence than the first — and it is
    also the proof that `ground`'s omitted count, computed out here as nodes-minus-elements,
    equals the engine's own `omission.nodes_omitted`.
    """
    pdf = repo_root / "fixtures" / "engine" / pdf_name / "document.pdf"
    representation = ethos_parser.extract(pdf)

    from_mcp = mcp(
        engine_binary,
        [
            ("tools/call", {"name": "extract", "arguments": {"path": str(pdf)}}),
            ("tools/call", {"name": "ground", "arguments": {"representation": representation}}),
        ],
    )

    assert call(by_name["extract"], {"path": str(pdf)}).content == (
        from_mcp[0]["content"][0]["text"]
    )
    assert call(by_name["ground"], {"representation": representation}).content == (
        from_mcp[1]["content"][0]["text"]
    )


def test_no_summary_carries_a_locator(by_name, representation, fixture_pdf):
    """The `mcp_stdio.rs` check, extended: the tokens that suite bans, plus the **real** minted id
    and fingerprint read off this artifact rather than a hardcoded `s1`."""
    minted = representation["representation"]["nodes"][0]["id"]
    fingerprint = representation["representation_c14n_sha256"]

    summaries = [
        call(by_name["extract"], {"path": str(fixture_pdf)}).content,
        call(by_name["ground"], {"representation": representation}).content,
        call(by_name["node_get"], {"representation": representation, "node_id": minted}).content,
    ]
    for summary in summaries:
        for locator in ["bbox", "[", "x0", "origin", "sha256:", minted, fingerprint]:
            assert locator not in summary, (
                "`{}` reached the model-facing summary: {!r}".format(locator, summary)
            )


def test_node_get_names_the_kind_the_artifact_names(by_name, representation):
    """The kind is a category, not a handle — and it is spelled the way the **artifact** spells it.

    `ethos-parser mcp` prints Rust's `Debug` of the enum (`TextRun`); the artifact carries the serde
    name (`text_run`). This adapter reports what the artifact says, because reshaping it into the
    other spelling would be the adapter inventing a name for a thing it did not read.
    """
    minted = representation["representation"]["nodes"][0]["id"]
    message = call(by_name["node_get"], {"representation": representation, "node_id": minted})
    assert message.content == "1 node, kind `{}`.".format(message.artifact["kind"])


def test_no_tool_sets_trust_state(by_name, representation, fixture_pdf):
    minted = representation["representation"]["nodes"][0]["id"]
    messages = [
        call(by_name["extract"], {"path": str(fixture_pdf)}),
        call(by_name["ground"], {"representation": representation}),
        call(by_name["node_get"], {"representation": representation, "node_id": minted}),
    ]
    for message in messages:
        haystack = (message.content + " " + json.dumps(message.additional_kwargs)).lower()
        for word in TRUST_STATE_WORDS:
            assert word not in haystack, "`{}` appears on a tool result".format(word)
        assert message.status == "success", "a success is a success, not a degree of one"

    for tool in by_name.values():
        described = (tool.description + json.dumps(TOOL_SCHEMAS[tool.name])).lower()
        for word in ["confidence", "verdict", "evidence_tier", "trust_score"]:
            assert word not in described


# --- the handle law, through the framework ------------------------------------------------------


def test_a_forged_handle_is_a_tool_error_and_not_an_empty_artifact(by_name, representation):
    """`12-V12-SCOPE.md` §3: fails closed means an error, not an empty answer.

    Raising is how a tool says that here — LangChain surfaces it as a tool failure, which is
    MCP's `isError: true` in this framework's currency.
    """
    with pytest.raises(NodeNotFound):
        call(by_name["node_get"], {"representation": representation, "node_id": "s-forged"})


def test_an_edited_representation_fails_at_the_fingerprint(by_name, representation):
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["text"] = "Tampered"

    minted = representation["representation"]["nodes"][0]["id"]
    with pytest.raises(FingerprintMismatch):
        call(by_name["node_get"], {"representation": edited, "node_id": minted})


def test_a_document_the_engine_cannot_read_is_a_tool_error(by_name, tmp_path):
    not_a_pdf = tmp_path / "document.pdf"
    not_a_pdf.write_bytes(b"this is not a PDF")
    with pytest.raises(ethos_parser.EngineFailed):
        call(by_name["extract"], {"path": str(not_a_pdf)})


# --- one wire shape -------------------------------------------------------------------------------


def test_the_argument_schemas_are_the_ones_mcp_advertises(engine_binary):
    """One wire shape across the three adapters, checked against the server rather than remembered.

    `node_id` keeps MCP's spelling here even though the Node SDK's parameter is `nodeId`: the tool
    argument is the wire, and one wire has one name.
    """
    advertised = mcp(engine_binary, [("tools/list", {})])[0]["tools"]
    assert {tool["name"] for tool in advertised} == set(TOOL_SCHEMAS)
    for tool in advertised:
        assert TOOL_SCHEMAS[tool["name"]] == tool["inputSchema"], (
            "the LangChain schema for `{}` has drifted from what `ethos-parser mcp` advertises".format(
                tool["name"]
            )
        )


def test_no_tool_argument_names_a_coordinate(by_name):
    for name, schema in TOOL_SCHEMAS.items():
        properties = list(schema["properties"])
        assert properties, "an empty schema would pass vacuously"
        for banned in BANNED_ARGUMENT_NAMES:
            assert banned not in properties, (
                "tool `{}` takes `{}`, which lets a model author a locator".format(name, banned)
            )
        assert schema["additionalProperties"] is False, (
            "tool `{}` accepts extra arguments, which is where a locator sneaks in".format(name)
        )

    # And the same read off what the tools actually expose to a host, not only off the table.
    for name, tool in by_name.items():
        for banned in BANNED_ARGUMENT_NAMES:
            assert banned not in tool.args, "tool `{}` exposes `{}`".format(name, banned)


def test_the_surfaces_this_slice_did_not_wrap_are_absent(by_name):
    for absent in ["markdown", "html", "verify", "mcp", "classify"]:
        assert absent not in by_name


def test_importing_the_sdk_does_not_import_langchain():
    """`import ethos_parser` must still pull nothing — the extra is an extra."""
    probe = (
        "import sys, ethos_parser;"
        "print('langchain_core' in sys.modules or 'pydantic' in sys.modules)"
    )
    completed = subprocess.run(
        [__import__("sys").executable, "-c", probe], capture_output=True, text=True, check=True
    )
    assert completed.stdout.strip() == "False"


def test_this_slice_added_no_langgraph_dependency(repo_root):
    """§16.7 refused a separate LangGraph adapter: a `StructuredTool` is what LangGraph binds.

    A `langgraph` pin would be a second surface bought with a dependency, to prove something a
    bindable tool already proves.
    """
    pyproject = (repo_root / "packages" / "python" / "pyproject.toml").read_text(encoding="utf-8")
    assert "langgraph" not in pyproject
    assert "dependencies = []" in pyproject, "the runtime install must stay empty"
