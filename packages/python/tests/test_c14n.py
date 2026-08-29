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

"""c14n parity — the Rust module's own vectors, run through the Python port.

These are lifted verbatim from ``crates/ethos-parser-core/src/c14n.rs``, which lifted them from Ethos's
committed vectors, which are cross-checked against a Python reference. Matching them is what
makes :func:`ethos_parser.node_get`'s fingerprint the engine's fingerprint rather than one that
resembles it.

They are the cheap half of the proof. The load-bearing half is in ``test_cli_surface.py``, where
a whole artifact the engine actually printed is re-canonicalized and compared byte for byte.
"""

import json

import pytest

from ethos_parser._c14n import (
    MAX_SAFE_INT,
    CanonicalizationError,
    c14n_bytes,
    sha256_hex,
)


def c14n_str(value):
    return c14n_bytes(value).decode("utf-8")


# --- parity vectors ------------------------------------------------------------------------


def test_parity_empty_object():
    assert c14n_str({}) == "{}"
    assert (
        sha256_hex({})
        == "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    )


def test_parity_key_order():
    value = {"b": 2, "a": 1, "_": 0, "Z": -3}
    assert c14n_str(value) == '{"Z":-3,"_":0,"a":1,"b":2}'
    assert (
        sha256_hex(value)
        == "9e8c5fa78b63297991b5b7b45bd334ccc61bd1058c5cd8ca6ee0451f78cd6cc1"
    )


def test_parity_strings_and_ints():
    value = {
        "text": "líne1\nl\"ine2\tend — \U0001F4A1",
        "n_zero": 0,
        "n_neg": -42,
        "arr": [3, 1, 2],
        "flag": True,
        "nothing": None,
    }
    assert c14n_str(value) == (
        '{"arr":[3,1,2],"flag":true,"n_neg":-42,"n_zero":0,"nothing":null,'
        '"text":"líne1\\nl\\"ine2\\tend — \U0001F4A1"}'
    )
    assert (
        sha256_hex(value)
        == "86b355efaa571cac1ddb71d422a9971e6042c55ec5369305cce095f2c181426e"
    )


def test_parity_controls_and_backslash():
    value = {"bel": "\u0007", "backslash": "a\\b"}
    assert c14n_str(value) == '{"backslash":"a\\\\b","bel":"\\u0007"}'
    assert (
        sha256_hex(value)
        == "a1cc2b96cfaf4e1d27ca13e7c2e56faadf76bd027d233fce5a57124e36ea6dfd"
    )


def test_parity_fingerprint_manifest():
    value = {
        "config_sha256": "68cc61753d299917cc7773f069c18aca31c8ac68f43736a94cb57eee05144084",
        "payload_sha256": "dad47d0ac4ab90f60691eb884c4c7e58d38ef7b87ef3df4bf602cd6087c9c757",
        "profile_id": "ethos-deterministic-v1",
        "profile_sha256": "d6145b9210845db39ad592ea549788432b52a649778c9947f5b2d91173e38070",
        "schema_version": "1.0.0",
        "source_fingerprint": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef",
    }
    assert (
        sha256_hex(value)
        == "b5d30710d0c25cc38d8dec924ecaf57ae4f81276dd5dc14d75cb3b5b6bde62d3"
    )


# --- float rejection -----------------------------------------------------------------------


@pytest.mark.parametrize(
    "value",
    [
        1.5,
        {"x": 1.5},
        [0.1],
        {"a": {"b": [1, 2, {"c": 0.25}]}},
        [[[[2.5]]]],
        {"ok": 1, "bad": [{"deep": -3.5}]},
    ],
)
def test_floats_are_rejected_at_every_depth(value):
    with pytest.raises(CanonicalizationError):
        c14n_bytes(value)


def test_a_float_is_an_error_not_a_rounding():
    with pytest.raises(CanonicalizationError) as excinfo:
        c14n_bytes({"x": 1.5})
    assert str(excinfo.value) == "non-integer number in canonical value"
    # The nearest integer must never appear in output as a silent repair.
    with pytest.raises(CanonicalizationError):
        c14n_bytes({"x": 2.0})


@pytest.mark.parametrize("text", ["1.0", "-0.0", "0.0", "1e2", "1E2", "1.5e3", "2.0"])
def test_float_shaped_text_stays_rejected(text):
    """A value whose JSON text is float-shaped is refused even when it is mathematically integral.

    Python's parser stores these as :class:`float`, which is the property the rejection reads.

    The Rust list carries one more entry — bare ``-0`` — and it is **deliberately absent here**,
    because the two parsers disagree about it: ``serde_json`` stores ``-0`` as a float and Python
    stores it as :class:`int` ``0``. The disagreement is in the parsers, not in c14n, and it is
    unreachable through this package: the engine never prints ``-0``, and the fingerprint of
    anything it did print is computed over the bytes it printed. Asserting the Rust behaviour
    here would be asserting something false about Python.
    """
    with pytest.raises(CanonicalizationError):
        c14n_bytes(json.loads(text))


@pytest.mark.parametrize("text", ["1", "0", "2", "100", "-1", "-100"])
def test_integer_spellings_remain_canonical(text):
    assert c14n_bytes(json.loads(text))


def test_integers_at_the_2_53_boundary():
    assert c14n_bytes(MAX_SAFE_INT) == str(MAX_SAFE_INT).encode()
    assert c14n_bytes(-MAX_SAFE_INT) == str(-MAX_SAFE_INT).encode()
    for out_of_range in (MAX_SAFE_INT + 1, -MAX_SAFE_INT - 1):
        with pytest.raises(CanonicalizationError):
            c14n_bytes(out_of_range)


def test_a_bool_is_not_written_as_an_integer():
    """`bool` subclasses `int` in Python, so an unordered type check would emit `1` for `True`
    and silently change every fingerprint covering a boolean field."""
    assert c14n_str({"flag": True, "off": False}) == '{"flag":true,"off":false}'
    assert c14n_str([True, 1, False, 0]) == "[true,1,false,0]"


# --- escaping ------------------------------------------------------------------------------


def test_control_characters_use_lowercase_four_digit_escapes():
    assert c14n_str("\u0001") == '"\\u0001"'
    assert c14n_str("\u001f") == '"\\u001f"'
    assert c14n_str("\u000b") == '"\\u000b"'
    # The named short escapes take precedence where they exist.
    assert c14n_str("\b") == '"\\b"'
    assert c14n_str("\t") == '"\\t"'
    assert c14n_str("\n") == '"\\n"'
    assert c14n_str("\f") == '"\\f"'
    assert c14n_str("\r") == '"\\r"'


def test_non_ascii_is_literal_and_never_normalized():
    # Space is the first code point at or above the escape threshold.
    assert c14n_str(" ") == '" "'
    assert c14n_str("é") == '"é"'
    assert c14n_str("日本語") == '"日本語"'
    assert c14n_str("\U0001F4A1") == '"\U0001F4A1"'

    # Precomposed vs decomposed are different byte sequences and must stay different:
    # normalizing would rewrite evidence a citation may quote verbatim.
    precomposed = c14n_bytes("\u00e9")
    decomposed = c14n_bytes("\u0065\u0301")
    assert precomposed != decomposed
    assert len(precomposed) == 4  # quote + 2 UTF-8 bytes + quote
    assert len(decomposed) == 5  # quote + 1 + 2 UTF-8 bytes + quote


def test_only_quote_and_backslash_are_escaped_above_the_control_range():
    assert c14n_str('"') == '"\\""'
    assert c14n_str("\\") == '"\\\\"'
    # Solidus is NOT escaped — `\/` would be valid JSON but different bytes.
    assert c14n_str("/") == '"/"'


def test_keys_are_escaped_by_the_same_rule_as_values():
    assert c14n_str({"a\nb": 1}) == '{"a\\nb":1}'


# --- structure -----------------------------------------------------------------------------


def test_no_whitespace_anywhere():
    text = c14n_str({"a": [1, 2, {"b": "c"}], "d": None})
    assert " " not in text
    assert "\n" not in text
    assert text == '{"a":[1,2,{"b":"c"}],"d":null}'


def test_array_order_is_preserved_because_it_is_semantic():
    assert c14n_str([3, 1, 2]) == "[3,1,2]"


def test_key_order_is_by_code_point_not_ascii_case_or_locale():
    assert c14n_str({"a": 1, "A": 2, "_": 3, "b": 4, "B": 5}) == (
        '{"A":2,"B":5,"_":3,"a":1,"b":4}'
    )
    assert c14n_str({"é": 1, "z": 2, "A": 3}) == '{"A":3,"z":2,"é":1}'


def test_nested_objects_are_sorted_at_every_level():
    value = {"z": {"y": 1, "x": 2}, "a": {"c": 3, "b": 4}}
    assert c14n_str(value) == '{"a":{"b":4,"c":3},"z":{"x":2,"y":1}}'


def test_key_insertion_order_is_irrelevant():
    keys = ["q", "b", "zz", "a"]
    forward = {k: 1 for k in keys}
    backward = {k: 1 for k in reversed(keys)}
    assert c14n_bytes(forward) == c14n_bytes(backward)


def test_c14n_is_idempotent():
    value = {"z": [1, {"b": "x\ty", "a": None}], "é": True, "n": -7}
    once = c14n_bytes(value)
    twice = c14n_bytes(json.loads(once.decode("utf-8")))
    assert once == twice


def test_a_non_json_value_is_an_error_rather_than_a_repr():
    with pytest.raises(CanonicalizationError):
        c14n_bytes({1: "an integer key is not a JSON key"})
    with pytest.raises(CanonicalizationError):
        c14n_bytes({"s"})
