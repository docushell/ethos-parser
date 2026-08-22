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

"""The Python SDK — a thin surface over the ``engine`` CLI (v1.2-S2).

# What this is, and the one property it is arranged to have

`docs/13-V12-MILESTONES.md` S2 asks for "a thin Python surface over the same library or CLI, **so
it cannot diverge from what the CLI prints**". This package spends one process spawn to make that
a tautology rather than a promise: :func:`extract` and :func:`ground` run the same subcommands a
shell would run and hand back the bytes those subcommands printed, parsed as JSON. There is no
second serialization anywhere in this package, so there is nowhere for the artifact to change.

That is also why there is no native extension. PyO3 would reach the library by a second path,
which is a second thing that can disagree with the first — plus a wheel matrix and a fifth build
surface, for a saving nobody has measured a need for.

# The handle law, which decides these three signatures

`docs/12-V12-SCOPE.md` §3, carried here unchanged from MCP: **the engine mints every locator,
returns it as an opaque handle, and re-validates it on the way back in.** In Python that means:

- **A locator is returned, never accepted as prose.** No public function here takes a page, a
  box, an ``x``/``y``, a width, a height or a row/column pair. Not optionally, not keyword-only,
  not behind a flag. ``tests/test_handle_law.py`` reads the signatures and fails on those names.
- **A handle travels back as the bytes that were handed out.** :func:`node_get` takes a node id
  string copied out of a representation this engine returned.
- **A handle this engine did not mint fails closed** — :class:`NodeNotFound`, never ``None`` and
  never ``{}``. An empty answer tells a caller its guess was merely unlucky; an exception tells
  it the guess was not admissible.

# What is deliberately absent

``markdown`` and ``html`` exist on the CLI and are not wrapped here: neither proves anything this
slice claims, and a function that exists because it was cheap is a surface to keep honest
forever. ``verify`` is not here for a stronger reason — it relays the pinned Ethos CLI, and
`docs/07-VERIFY-BOUNDARY.md` is the boundary a host's convenience must not bend. A Python
function named ``verify`` would look like this package had an opinion about whether a claim is
supported. It does not, and neither does the engine.

There is no MCP client here either. MCP is a process; this is a library. They are two callers of
the same binary, not layers.

# Locating the binary

``ETHOS_ENGINE`` first and **authoritatively** — a path named there and not present is
:class:`EngineNotFound`, not a reason to go looking for some other build — then ``engine`` on
``PATH``. That is the precedent ``VerifierBinary::resolve`` sets for ``ETHOS_BIN``, and the
reason is the same: resolving to a binary nobody chose means relaying bytes from a parser nobody
chose. Nothing here downloads or vendors one.
"""

import json
import os
import shutil
import subprocess
import tempfile

from ._c14n import CanonicalizationError, c14n_bytes, sha256_hex

__all__ = [
    "__version__",
    "REPRESENTATION_ARTIFACT_TYPE",
    "EngineError",
    "EngineFailed",
    "EngineNotFound",
    "FingerprintMismatch",
    "NodeNotFound",
    "NotARepresentation",
    "extract",
    "ground",
    "node_get",
]

#: Tracks the workspace version, and `tests/test_cli_surface.py` asserts it against
#: `Cargo.toml`. An SDK claiming a version the engine does not is the same class of lie
#: `parser_version` exists to prevent.
__version__ = "0.34.0"

#: The ``artifact_type`` ``engine extract`` stamps on a representation.
REPRESENTATION_ARTIFACT_TYPE = "ethos.engine.representation.v0"

#: Everything under this prefix is a representation this package will read.
_REPRESENTATION_PREFIX = "ethos.engine.representation."

#: The environment variable that pins the binary, named to match ``ETHOS_BIN``.
_BINARY_ENV = "ETHOS_ENGINE"


# -----------------------------------------------------------------------------------------
# Failures, each one named
# -----------------------------------------------------------------------------------------
#
# Distinct types rather than one, for the reason the CLI keeps three exit codes apart: a caller
# that cannot tell "the check failed" from "the check did not run" is the defect this project
# refuses. A forged handle and a missing binary are not the same news.


class EngineError(Exception):
    """Base class for every failure the three public functions raise.

    Everything, deliberately: a serialization refusal from :mod:`ethos_engine._c14n` is
    re-raised as :class:`NotARepresentation` rather than escaping as a bare
    :class:`ValueError`, so ``except EngineError`` is a complete catch and a caller never has to
    know this package canonicalizes anything.
    """


class EngineNotFound(EngineError):
    """No ``engine`` binary could be located, or the one pinned by ``ETHOS_ENGINE`` is absent."""


class EngineFailed(EngineError):
    """The engine ran and refused.

    Carries the exit status and the engine's own stderr, unedited: the CLI already says what
    went wrong and in what terms, and paraphrasing it here would be a second account of the same
    failure that could drift from the first.
    """

    def __init__(self, args, returncode, stderr):
        self.args_ = list(args)
        self.returncode = returncode
        self.stderr = stderr
        super().__init__(
            "`engine {}` exited {}: {}".format(
                " ".join(self.args_), returncode, stderr.strip() or "(no stderr)"
            )
        )


class NotARepresentation(EngineError):
    """The value handed in is not a ``DocumentRepresentation`` this package will read."""


class FingerprintMismatch(EngineError):
    """A representation's payload does not hash to its own declared digest.

    Names both digests, as ``verify_fingerprint`` does, so the failure is diagnosable rather
    than merely detected.
    """

    def __init__(self, declared, actual):
        self.declared = declared
        self.actual = actual
        super().__init__(
            "declared representation_c14n_sha256 is {} but the payload hashes to {}. This is not "
            "a record this engine will speak for: something edited the artifact after it was "
            "minted.".format(declared, actual)
        )


class NodeNotFound(EngineError):
    """A node id is not a node of the representation it was looked up in.

    **This is the handle law failing closed.** Not ``None``, not ``{}``, not the nearest node.
    """

    def __init__(self, node_id, fingerprint):
        self.node_id = node_id
        self.fingerprint = fingerprint
        super().__init__(
            "`{}` is not a node of this representation ({}). A node id is an opaque handle this "
            "engine minted: copy one from the artifact rather than composing it.".format(
                node_id, fingerprint
            )
        )


# -----------------------------------------------------------------------------------------
# The public surface — three functions, and not one of them names a coordinate
# -----------------------------------------------------------------------------------------


def extract(pdf_path):
    """Read a PDF and return ``DocumentRepresentation v0``, as ``engine extract`` prints it.

    Every locator a later call needs is minted here. Pass this object back to :func:`ground` or
    :func:`node_get` rather than composing one.

    :param pdf_path: path to a PDF, as :class:`str` or :class:`os.PathLike`.
    :returns: the parsed canonical artifact.
    :raises EngineNotFound: no binary.
    :raises EngineFailed: the engine could not read the document.
    """
    return _parse(_run(["extract", os.fspath(pdf_path)]))


def ground(representation):
    """Project a representation into ``ethos.grounding.v1``, as ``engine ground`` prints it.

    Takes the artifact :func:`extract` returned — the object itself, or a path to bytes this
    engine wrote — mirroring the MCP tool of the same name. **The engine re-validates it**: a
    payload that does not hash to its declared digest is refused there, by the same
    ``verify_fingerprint`` every other subcommand runs, rather than by a second check here that
    could drift from it.

    Nodes with no measurable ink box are omitted from the projection and counted by the engine
    on stderr; the representation this came from is where that declaration lives, which is the
    CLI's own arrangement and not a drop introduced here.

    :param representation: the artifact object, or a path to it.
    :returns: the parsed ``ethos.grounding.v1`` artifact.
    :raises EngineFailed: the representation was refused, fingerprint included.
    """
    if isinstance(representation, dict):
        # Written as canonical bytes — this package's one serializer, the same one the
        # fingerprint is computed over — so what reaches the engine is what the engine printed.
        try:
            body = c14n_bytes(representation)
        except CanonicalizationError as e:
            raise NotARepresentation(
                "this object will not canonicalize, so it is not the artifact `extract` "
                "printed: {}".format(e)
            ) from e
        with tempfile.TemporaryDirectory(prefix="ethos-engine-") as directory:
            path = os.path.join(directory, "representation.json")
            with open(path, "wb") as handle:
                handle.write(body)
            return _parse(_run(["ground", path]))
    return _parse(_run(["ground", os.fspath(representation)]))


def node_get(representation, node_id):
    """Return one node from ``representation``, by the id the engine minted for it.

    **The handle law, made mechanical**, and the one function here with no subcommand behind it.
    ``engine node-get`` does not exist and this slice does not add it — MCP already carries the
    tool, and a third CLI verb nobody asked for is surface to keep honest forever. So the checks
    are ported rather than shelled out, in the order ``mcp.rs`` runs them:

    1. the value is a representation;
    2. its payload is re-canonicalized and re-hashed, and must equal its declared digest —
       an artifact edited on the way through is refused **before** any lookup happens;
    3. ``node_id`` is looked up among *that* artifact's own nodes.

    A miss raises. There is no argument by which to ask for a node by page or by position.

    :param representation: the artifact :func:`extract` returned.
    :param node_id: a node id copied verbatim out of that artifact.
    :returns: the node record the artifact carries, verbatim.
    :raises NotARepresentation: the value is not a representation.
    :raises FingerprintMismatch: the payload does not hash to its declared digest.
    :raises NodeNotFound: nothing minted that id — the forged-handle case, failing closed.
    """
    payload, declared = _validated_payload(representation)
    for node in payload["nodes"]:
        if isinstance(node, dict) and node.get("id") == node_id:
            return node
    raise NodeNotFound(node_id, declared)


# -----------------------------------------------------------------------------------------
# Internals
# -----------------------------------------------------------------------------------------


def _validated_payload(representation):
    """Re-validate a representation and return ``(payload, declared_fingerprint)``."""
    if not isinstance(representation, dict):
        raise NotARepresentation(
            "expected a representation object, got {}".format(
                type(representation).__name__
            )
        )

    artifact_type = representation.get("artifact_type")
    if not isinstance(artifact_type, str) or not artifact_type.startswith(
        _REPRESENTATION_PREFIX
    ):
        raise NotARepresentation(
            "artifact_type is {!r}; expected one under `{}`. `ground` and `node_get` read the "
            "artifact `extract` minted, not a projection of it.".format(
                artifact_type, _REPRESENTATION_PREFIX
            )
        )

    declared = representation.get("representation_c14n_sha256")
    payload = representation.get("representation")
    if not isinstance(declared, str) or not isinstance(payload, dict):
        raise NotARepresentation(
            "a representation carries a `representation` object and a "
            "`representation_c14n_sha256` string; this one does not"
        )

    try:
        actual = "sha256:" + sha256_hex(payload)
    except CanonicalizationError as e:
        # A payload the canonical serializer refuses cannot be the payload that digest covers,
        # so this is the same news as a mismatch and not a lookup that half-happened.
        raise NotARepresentation(
            "this payload will not canonicalize, so it cannot be the one `{}` covers: {}".format(
                declared, e
            )
        ) from e
    if actual != declared:
        raise FingerprintMismatch(declared, actual)

    nodes = payload.get("nodes")
    if not isinstance(nodes, list):
        raise NotARepresentation("the payload carries no `nodes` list")
    return payload, declared


def _binary():
    """Locate the engine. ``ETHOS_ENGINE`` is authoritative; ``PATH`` is the fallback."""
    pinned = os.environ.get(_BINARY_ENV)
    if pinned:
        if os.path.isfile(pinned):
            return pinned
        raise EngineNotFound(
            "{}={!r} names a file that is not there. An explicit pin is authoritative: falling "
            "back to some other build would mean returning artifacts from a parser nobody "
            "chose.".format(_BINARY_ENV, pinned)
        )

    found = shutil.which("engine")
    if found:
        return found

    raise EngineNotFound(
        "no `engine` binary. Tried, in order:\n"
        "  {} (unset)\n"
        "  `engine` on PATH (not found)\n\n"
        "This package is a surface over that binary and computes nothing without it. Build it "
        "with `cargo build --release`, then put it on PATH or point {} at it. Nothing here "
        "downloads one.".format(_BINARY_ENV, _BINARY_ENV)
    )


def _run(args):
    """Run a subcommand and return its stdout. A non-zero exit is raised, never swallowed."""
    binary = _binary()
    try:
        # No `env=`: the engine is deterministic and handing it an environment this package
        # composed would be one more input nobody declared.
        completed = subprocess.run(
            [binary] + list(args),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as e:
        raise EngineNotFound("`{}` could not be run: {}".format(binary, e)) from e

    if completed.returncode != 0:
        raise EngineFailed(
            args,
            completed.returncode,
            completed.stderr.decode("utf-8", errors="replace"),
        )
    return completed.stdout


def _parse(stdout):
    """Parse canonical JSON the engine printed.

    ``parse_constant`` refuses ``NaN`` and ``Infinity``, which Python's parser accepts by default
    and c14n v1 does not admit at all. The engine never prints them; a parser that would accept
    them is a hole in the one place this package reads bytes it did not write.
    """
    try:
        return json.loads(stdout.decode("utf-8"), parse_constant=_reject_constant)
    except (UnicodeDecodeError, ValueError) as e:
        raise EngineError(
            "the engine exited 0 but its output is not canonical JSON: {}".format(e)
        ) from e


def _reject_constant(name):
    raise ValueError("`{}` is not a value c14n v1 admits".format(name))
