# Copyright 2026 The ethos-engine maintainers
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

"""c14n v1 in Python — the same canonical serialization ``engine-core/src/c14n.rs`` writes.

**Why this file exists at all.** :func:`ethos_engine.node_get` has no CLI subcommand behind it,
so it is the one function here that computes something rather than relaying it. What it computes
is a fingerprint, and a fingerprint that is *nearly* the engine's is worse than none: it would
accept an artifact the engine would refuse, or refuse one the engine minted, and either way a
caller would be told something false about a document.

So this is a port and not an approximation. The properties are the Rust module's, in its order:

- UTF-8, no whitespace between tokens
- object keys sorted by Unicode code point, **explicitly at write time**
- minimal escaping; no Unicode normalization
- integers only — any non-integer number is a hard error
- ``|n| <= 2**53 - 1``

``tests/test_c14n.py`` runs the Rust module's own parity vectors through this one, and
``tests/test_cli_surface.py`` re-canonicalizes what ``engine extract`` printed and compares the
bytes. The second is the load-bearing check: it is the whole artifact, not a hand-written value.
"""

import hashlib

__all__ = ["MAX_SAFE_INT", "CanonicalizationError", "c14n_bytes", "sha256_hex"]

#: The integer bound c14n v1 declares, matching ``engine_core::MAX_SAFE_INT``.
MAX_SAFE_INT = 2**53 - 1


class CanonicalizationError(ValueError):
    """A value could not be canonicalized.

    Carries a deterministic message, exactly as ``C14nError`` does: the same offending value
    produces the same text, so a failure is reproducible from the message alone.
    """


# The named short escapes, which take precedence over the ``\\u00xx`` form where they exist.
_ESCAPES = {
    '"': '\\"',
    "\\": "\\\\",
    "\b": "\\b",
    "\t": "\\t",
    "\n": "\\n",
    "\f": "\\f",
    "\r": "\\r",
}


def c14n_bytes(value):
    """Serialize a JSON value to canonical bytes.

    :raises CanonicalizationError: on a non-integer number, an integer past
        :data:`MAX_SAFE_INT`, a non-string object key, or a type that is not JSON.
    """
    out = []
    _write(value, out)
    text = "".join(out)
    try:
        return text.encode("utf-8")
    except UnicodeEncodeError as e:
        # A lone surrogate. Rust strings cannot hold one, so this is a value the engine could
        # never have produced — an error rather than a lossy replacement.
        raise CanonicalizationError(
            "value contains text that is not valid UTF-8: {}".format(e)
        ) from e


def sha256_hex(value):
    """Lowercase hex sha256 over the canonical bytes of ``value``."""
    return hashlib.sha256(c14n_bytes(value)).hexdigest()


def _write(value, out):
    # `bool` first: in Python it is a subclass of `int`, so an unordered check would serialize
    # `True` as `1` and silently change the bytes a fingerprint covers.
    if value is None:
        out.append("null")
    elif isinstance(value, bool):
        out.append("true" if value else "false")
    elif isinstance(value, int):
        # `abs` is safe here — Python integers do not overflow — so the bound is the only check.
        if abs(value) > MAX_SAFE_INT:
            raise CanonicalizationError("integer exceeds 2^53-1 in canonical value")
        out.append(str(value))
    elif isinstance(value, float):
        # Including `1.0`. A float that happens to be integral is still a float, and rounding it
        # would be a silent repair of the exact kind `docs/01-CONTRACT.md` refuses.
        raise CanonicalizationError("non-integer number in canonical value")
    elif isinstance(value, str):
        _write_string(value, out)
    elif isinstance(value, (list, tuple)):
        out.append("[")
        for i, item in enumerate(value):
            if i:
                out.append(",")
            _write(item, out)
        out.append("]")
    elif isinstance(value, dict):
        out.append("{")
        for key in value:
            if not isinstance(key, str):
                raise CanonicalizationError(
                    "object key {!r} is not a string in canonical value".format(key)
                )
        # Python compares `str` by code point, which is exactly the contract sort. Sorted here
        # at write time rather than relying on the mapping's own order, for the reason the Rust
        # module gives: insertion order must never reach the output.
        for i, key in enumerate(sorted(value)):
            if i:
                out.append(",")
            _write_string(key, out)
            out.append(":")
            _write(value[key], out)
        out.append("}")
    else:
        raise CanonicalizationError(
            "{} is not a canonical JSON value".format(type(value).__name__)
        )


def _write_string(s, out):
    """Minimal escaping. Non-ASCII is emitted literally and never normalized: extracted text is
    evidence, and NFC-folding it would silently change bytes a citation may quote."""
    out.append('"')
    for ch in s:
        escape = _ESCAPES.get(ch)
        if escape is not None:
            out.append(escape)
        elif ch < " ":
            out.append("\\u{:04x}".format(ord(ch)))
        else:
            out.append(ch)
    out.append('"')
