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
import stat
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
        input=("\n".join(lines) + "\n").encode("utf-8"),
        capture_output=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    # Split on the LF byte and nothing else. Canonical JSON writes U+2028, U+2029 and U+0085
    # literally, and text-mode `splitlines` would cut one reply carrying them into fragments.
    return [
        json.loads(line.decode("utf-8"))["result"]
        for line in completed.stdout.split(b"\n")
        if line.strip()
    ]


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


@pytest.mark.parametrize(
    "pdf_name",
    [
        "markdown-two-blocks",
        "off-page-and-offset-box",
        # Four runs in two blocks. Nodes-minus-elements, which this adapter used to report as the
        # omission, says 2 here; the engine omitted none.
        "untagged-shredded-line",
        # Five nodes, four without a measured box: a non-zero omission, counted by the engine.
        "stroke-ruled-field-boxes",
    ],
)
def test_the_summaries_are_the_ones_mcp_emits(by_name, engine_binary, repo_root, pdf_name):
    """Byte-for-byte against `ethos-parser mcp`, including documents whose blocks merge runs.

    This is what stops the second adapter inventing a richer sentence than the first. The first two
    fixtures are single-run blocks with nothing omitted, which is why they never noticed that
    nodes-minus-elements stopped being the omission count when an element became a block.
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


# --- `ground`'s summary is MCP's own, including what the schema's limits took -----------------------


def _fixture_representation(repo_root, pdf_name):
    return copy.deepcopy(
        ethos_parser.extract(repo_root / "fixtures" / "engine" / pdf_name / "document.pdf")
    )


def _resealed(representation):
    """Recompute the fingerprint of an edited representation, so the engine accepts the edit."""
    representation["representation_c14n_sha256"] = "sha256:" + ethos_parser._c14n.sha256_hex(
        representation["representation"]
    )
    return representation


def _set_text(node, text):
    node["text"] = text
    node["attributes"]["text_run"]["char_codes"] = [ord(c) for c in text]


def _lengthened(repo_root, pdf_name):
    """A representation with a run — and, on a table, the cell holding it — past the schema's
    16,384-byte string limit, re-sealed. 8,193 two-byte characters are 16,386 bytes."""
    rep = _fixture_representation(repo_root, pdf_name)
    payload = rep["representation"]
    long_text = "é" * 8193
    if payload["tables"]:
        cell = payload["tables"][0]["cells"][0]
        cell["text"] = long_text
        target = next(n for n in payload["nodes"] if n["id"] == cell["node_ids"][0])
    else:
        target = payload["nodes"][0]
    _set_text(target, long_text)
    return _resealed(rep)


@pytest.mark.parametrize(
    "pdf_name,tables_withheld",
    [("markdown-two-blocks", False), ("ruled-table-grid", True)],
)
def test_a_degraded_ground_summary_is_mcps_own(
    by_name, engine_binary, repo_root, tmp_path, pdf_name, tables_withheld
):
    """**A degradation fires, and the tool's words are still MCP's, byte for byte.**

    An element past the string limit is omitted — and on the table fixture every table is withheld.
    The old adapter called the omitted element one "omitted for having none", a box it never lacked.
    The oracle is a live MCP call with the object inline, a different route from the tool's path,
    so the comparison is not the tool agreeing with itself.
    """
    rep = _lengthened(repo_root, pdf_name)
    oracle = mcp(
        engine_binary,
        [("tools/call", {"name": "ground", "arguments": {"representation": rep}})],
    )[0]["content"][0]["text"]
    assert "element(s) omitted, a string over the schema's byte limit" in oracle
    assert ("table(s) withheld" in oracle) is tables_withheld

    message = call(by_name["ground"], {"representation": rep})
    assert message.content == oracle
    assert message.artifact == ethos_parser.ground(rep)

    path = tmp_path / "representation.json"
    path.write_bytes(ethos_parser._c14n.c14n_bytes(rep))
    assert call(by_name["ground"], {"representation": str(path)}).content == oracle


def test_a_line_separator_in_the_text_does_not_split_the_reply(by_name, engine_binary, repo_root):
    """Canonical JSON writes U+2028, U+0085 and U+2029 literally. A reply carrying them is still one
    reply, and the tool reads it whole."""
    rep = _fixture_representation(repo_root, "markdown-two-blocks")
    node = rep["representation"]["nodes"][0]
    _set_text(node, node["text"] + "\u2028\u0085\u2029")
    _resealed(rep)
    oracle = mcp(
        engine_binary,
        [("tools/call", {"name": "ground", "arguments": {"representation": rep}})],
    )[0]
    message = call(by_name["ground"], {"representation": rep})
    assert message.content == oracle["content"][0]["text"]
    assert message.artifact == oracle["structuredContent"]


def test_a_refused_record_raises_what_ground_raises(by_name, representation):
    """A representation edited without re-sealing is refused, and the tool raises exactly the error
    ``ground()`` raises — exit status and the engine's own stderr — not a sentence reworded here."""
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["text"] = "Tampered"
    with pytest.raises(ethos_parser.EngineFailed) as from_sdk:
        ethos_parser.ground(edited)
    with pytest.raises(ethos_parser.EngineFailed) as from_tool:
        call(by_name["ground"], {"representation": edited})
    assert from_tool.value.returncode == from_sdk.value.returncode == 2
    assert from_tool.value.stderr == from_sdk.value.stderr
    assert "representation_c14n_sha256" in from_tool.value.stderr


def test_the_ground_tool_spawns_mcp_once_with_a_path(
    by_name, representation, engine_binary, monkeypatch, tmp_path
):
    """**One `ethos-parser mcp`, handed a path — never the object — and `ground` only on a refusal.**

    A shim stands in for the binary and logs each run. The object is megabytes on a real document
    and the server holds an inline argument many times over, so the request's size is pinned too.
    """
    log = tmp_path / "runs.log"
    shim = tmp_path / "ethos-parser"
    shim.write_text(
        "#!/bin/sh\n"
        'echo "$1" >> "$LOG"\n'
        'if [ "$1" = mcp ]; then body=$(cat); echo "${#body}" >> "$LOG"; '
        "printf '%s\\n' \"$body\" | \"$REAL\" \"$@\"; else exec \"$REAL\" \"$@\"; fi\n"
    )
    shim.chmod(shim.stat().st_mode | stat.S_IEXEC)
    monkeypatch.setenv("ETHOS_PARSER", str(shim))
    monkeypatch.setenv("LOG", str(log))
    monkeypatch.setenv("REAL", engine_binary)

    call(by_name["ground"], {"representation": representation})
    runs = log.read_text().split()
    assert runs[0] == "mcp" and len(runs) == 2, runs
    assert int(runs[1]) < 4096, "the request carries a path, not the representation"

    log.write_text("")
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["text"] = "Tampered"
    with pytest.raises(ethos_parser.EngineFailed):
        call(by_name["ground"], {"representation": edited})
    runs = log.read_text().split()
    assert [runs[0], runs[2]] == ["mcp", "ground"], runs


def test_the_ground_summary_is_not_formatted_here(repo_root):
    """The sentence lives in `mcp.rs` alone. An adapter that formats it again is the drift this whole
    change removes."""
    source = (repo_root / "packages" / "python" / "src" / "ethos_parser" / "langchain.py").read_text()
    assert "omitted for having none" not in source
    assert "with a measured box" not in source
