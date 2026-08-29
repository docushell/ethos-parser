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

"""The handle law in Python — `docs/12-V12-SCOPE.md` §3, as executables.

| handed to :func:`ethos_parser.node_get` | expected |
| --- | --- |
| a node id the engine minted, copied off the artifact | the node record |
| an id nothing minted | **raises**, never ``None`` and never ``{}`` |
| a representation whose payload was edited on the way through | **raises**, at the fingerprint |

Plus the corollary that decides the signatures: **no public function argument names document
geometry.** That one is read off :func:`inspect.signature` rather than off a reviewer's memory,
for the same reason the MCP suite reads it off the advertised schemas.
"""

import copy
import inspect

import pytest

import ethos_parser
from ethos_parser import (
    EngineError,
    FingerprintMismatch,
    NodeNotFound,
    NotARepresentation,
)

#: The same list `mcp.rs` and `mcp_stdio.rs` ban, so the two adapters cannot drift on what counts
#: as a locator a caller could compose.
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


@pytest.fixture(scope="module")
def representation(fixture_pdf):
    return ethos_parser.extract(fixture_pdf)


@pytest.fixture(scope="module")
def minted_id(representation):
    """Read off the artifact rather than hardcoded.

    A hardcoded id that stopped existing would make the forged-id half pass for the wrong reason.
    """
    nodes = representation["representation"]["nodes"]
    assert nodes, "extract minted no nodes; every assertion below would pass vacuously"
    return nodes[0]["id"]


# --- mint, opaque, re-validate ---------------------------------------------------------------


def test_a_minted_handle_returns_the_node_that_artifact_carries(
    representation, minted_id
):
    node = ethos_parser.node_get(representation, minted_id)
    assert node["id"] == minted_id
    assert node is representation["representation"]["nodes"][0], (
        "the node record must be the one the artifact carries, verbatim — not a copy this "
        "package assembled from it"
    )


def test_a_forged_handle_fails_closed(representation):
    with pytest.raises(NodeNotFound) as excinfo:
        ethos_parser.node_get(representation, "s-forged")

    message = str(excinfo.value)
    assert "s-forged" in message, "the refusal must name what was refused: {!r}".format(
        message
    )
    assert "not a node" in message
    assert excinfo.value.node_id == "s-forged"


def test_an_edited_payload_fails_at_the_fingerprint_before_any_lookup(
    representation, minted_id
):
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["text"] = "Tampered"

    with pytest.raises(FingerprintMismatch) as excinfo:
        ethos_parser.node_get(edited, minted_id)

    # Both digests, as `verify_fingerprint` names them, so the failure is diagnosable.
    assert excinfo.value.declared == representation["representation_c14n_sha256"]
    assert excinfo.value.actual != excinfo.value.declared
    assert excinfo.value.declared in str(excinfo.value)
    assert excinfo.value.actual in str(excinfo.value)


def test_an_edited_payload_fails_even_when_the_id_is_forged_too(representation):
    """The fingerprint is checked first, so an edited artifact never reaches the lookup."""
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["text"] = "Tampered"
    with pytest.raises(FingerprintMismatch):
        ethos_parser.node_get(edited, "s-forged")


@pytest.mark.parametrize(
    "not_a_representation",
    [
        None,
        {},
        [],
        "s1",
        {"artifact_type": "ethos.grounding.v1", "elements": []},
        {"artifact_type": "ethos.parser.representation.v0"},
    ],
)
def test_a_thing_that_is_not_a_representation_is_refused_rather_than_coerced(
    not_a_representation,
):
    with pytest.raises(NotARepresentation):
        ethos_parser.node_get(not_a_representation, "s1")


def test_a_payload_that_will_not_canonicalize_is_refused(representation):
    """c14n v1 admits no float, and a payload it refuses cannot be the one that digest covers.

    Same news as a mismatch, so it is an :class:`EngineError` and not a bare ``ValueError``
    leaking out of the serializer.
    """
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["probe"] = 1.5
    with pytest.raises(NotARepresentation):
        ethos_parser.node_get(edited, "s1")


def test_a_projection_is_not_a_representation(representation):
    """`ground`'s output carries locators too, and it is still not the record they were minted in."""
    projection = ethos_parser.ground(representation)
    assert projection["artifact_type"] == "ethos.grounding.v1"
    with pytest.raises(NotARepresentation):
        ethos_parser.node_get(projection, "s1")


def test_no_refusal_is_ever_an_empty_answer(representation, minted_id):
    """Every way of getting it wrong raises. None of them returns ``None`` or ``{}``.

    An empty answer would tell a caller its guess was merely unlucky; an exception tells it the
    guess was not admissible.
    """
    edited = copy.deepcopy(representation)
    edited["representation"]["nodes"][0]["text"] = "Tampered"

    for artifact, node_id in [
        (representation, "s-forged"),
        (representation, ""),
        (representation, minted_id + "-almost"),
        (edited, minted_id),
        ({}, minted_id),
    ]:
        with pytest.raises(EngineError):
            ethos_parser.node_get(artifact, node_id)


# --- the corollary: no argument names a locator ------------------------------------------------


def test_no_public_function_signature_names_a_coordinate():
    """A locator is **returned**, never accepted as prose.

    A ``page=`` or a ``bbox=`` here would let a caller author the locator the engine then speaks
    for, which is the hazard memo §16.7 says decides whether an adapter is worth having.
    """
    checked = 0
    for name in ethos_parser.__all__:
        member = getattr(ethos_parser, name)
        if not inspect.isfunction(member):
            continue
        checked += 1
        parameters = list(inspect.signature(member).parameters)
        for banned in BANNED_ARGUMENT_NAMES:
            assert banned not in parameters, (
                "`{}` takes `{}`, which lets a caller compose a locator".format(
                    name, banned
                )
            )
    assert checked == 3, "expected exactly extract/ground/node_get, saw {}".format(
        checked
    )


def test_the_public_surface_is_the_three_functions_and_nothing_else():
    functions = sorted(
        name
        for name in ethos_parser.__all__
        if inspect.isfunction(getattr(ethos_parser, name))
    )
    assert functions == ["extract", "ground", "node_get"]


def test_node_get_takes_a_handle_and_an_artifact_and_nothing_else():
    assert list(inspect.signature(ethos_parser.node_get).parameters) == [
        "representation",
        "node_id",
    ]
