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

"""Shared fixtures.

**The binary's absence is a failure, never a skip** (`docs/04-ARCHITECTURE.md` §4). A suite that
went green because it could not find the thing it tests would be the worst outcome available, so
:func:`engine_binary` raises with the command that fixes it rather than calling ``pytest.skip``.

And a suite that went green against the *wrong build* of the right version is the same outcome
wearing a version number, which is why :func:`_locate_binary` checks a capability and not only a
number. It has happened here; the measurement is in that function's own comment.
"""

import os
import pathlib
import re
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]

#: An in-tree fixture, so this suite depends on nothing outside the repository. Two text runs on
#: one page: enough that a node lookup is a lookup rather than a coin flip, and small enough that
#: a byte-identity failure is readable.
FIXTURE_PDF = REPO_ROOT / "fixtures" / "engine" / "markdown-two-blocks" / "document.pdf"


def _workspace_version():
    """The version `Cargo.toml` declares — the one a located binary has to agree with."""
    cargo = (REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([^"]+)"$', cargo, re.MULTILINE)
    assert match, "no workspace version in Cargo.toml"
    return match.group(1)


def _binary_version(path):
    """`ethos-parser --version`, or ``None`` if it will not run."""
    try:
        completed = subprocess.run(
            [str(path), "--version"], capture_output=True, text=True, check=False
        )
    except OSError:
        return None
    if completed.returncode != 0:
        return None
    return completed.stdout.split()[-1].strip()


#: The subcommands these suites actually drive — every ``cli(...)`` call in this package's tests
#: and every argument vector the SDK's own ``_run`` builds. A binary that cannot answer them is
#: not this tree's, whatever its ``--version`` says.
#:
#: ``node_get`` is deliberately absent: it has no subcommand, being ported into both packages and
#: carried over MCP, which is why ``mcp`` is on the list instead. Keep this list to what the suite
#: calls — a longer one would reject a binary over a capability nothing here exercises.
REQUIRED_SUBCOMMANDS = ("extract", "ground", "locate", "mcp")


def _subcommands(path):
    """The subcommand names ``--help`` lists, or ``None`` if it will not run or lists none.

    One spawn per candidate. clap prints every subcommand in a ``Commands:`` block, one per line,
    so the block is read and the first token of each line taken. Reading only that block rather
    than every indented line in the output is the difference between a check that fails closed and
    one a wrapped description could satisfy by accident.
    """
    try:
        completed = subprocess.run(
            [str(path), "--help"], capture_output=True, text=True, check=False
        )
    except OSError:
        return None
    if completed.returncode != 0:
        return None

    lines = completed.stdout.splitlines()
    if "Commands:" not in lines:
        return None
    names = set()
    for line in lines[lines.index("Commands:") + 1 :]:
        if not line.strip():
            break
        names.add(line.split()[0])
    # Nothing read is "could not read", never "carries nothing": the caller turns ``None`` into
    # every requirement being missing, so an unparseable help text rejects rather than passes.
    return names or None


def _missing_subcommands(path):
    """Which of :data:`REQUIRED_SUBCOMMANDS` this binary does not carry, in the listed order."""
    assert REQUIRED_SUBCOMMANDS, "an empty requirement list would make this guard vacuous"
    listed = _subcommands(path)
    if listed is None:
        return list(REQUIRED_SUBCOMMANDS)
    return [name for name in REQUIRED_SUBCOMMANDS if name not in listed]


def _locate_binary():
    """Find a binary that is **this workspace's**, and refuse one that is not — by version *and*
    by capability.

    The version check is not decoration. A stale `target/release/ethos-parser` is preferred over
    `target/debug` and answers every question plausibly, so a suite that took the first file it
    found would compare the SDK against a parser from six minor versions ago and go green — it
    would still prove the SDK does not alter what the CLI prints, but about the wrong CLI.
    Measured, not feared: it is what a `0.11.0` release build did here at v1.2-S4.

    **And the version check alone does not make that impossible, which this comment used to claim
    it did.** A version string cannot distinguish two builds of one *unreleased* version. Measured
    on 2026-09-18, while `locate` was being added at workspace 0.58.0 unreleased: a
    `target/release` build from 00:45 reported `0.58.0`, exactly the number the tree wanted, and
    was preferred over the `target/debug` build that actually had `locate` in it. That release
    build had no `locate` subcommand and advertised three MCP tools instead of four — and **both
    suites went green against it**, including the parity test whose entire job is to fail when the
    server advertises a tool the LangChain adapters do not wrap. The version had not moved, so
    nothing about a version could have caught it.

    So a candidate is asked what it can *do* as well as what it is called: `REQUIRED_SUBCOMMANDS`
    is the list these suites drive, read off one `--help` run, and a candidate missing any of them
    falls through exactly as a version mismatch does. The final error names staleness apart from a
    version problem, because on the measured failure there was no version problem to go looking
    for, and a message about versions would have sent the reader after the wrong cause.
    """
    want = _workspace_version()

    pinned = os.environ.get("ETHOS_PARSER")
    if pinned:
        # An explicit pin is authoritative in both directions: it is never silently overridden,
        # and a pin that is the wrong build is an error rather than a reason to look elsewhere.
        # Both halves of "wrong build" apply — a pinned stale binary is an error for the same
        # reason, and passing it silently is the failure measured above.
        got = _binary_version(pinned)
        if got != want:
            raise RuntimeError(
                "ETHOS_PARSER={!r} reports {} but this workspace is {}. Rebuild it, or point "
                "ETHOS_PARSER at a build of this tree.".format(pinned, got or "nothing", want)
            )
        missing = _missing_subcommands(pinned)
        if missing:
            raise RuntimeError(
                "ETHOS_PARSER={!r} reports {}, which is this workspace's version, but does not "
                "carry: {}. **This is staleness, not a version problem** — it is a build of this "
                "tree from before those subcommands existed, and its `--version` cannot say so. "
                "Rebuild it:\n\n    cargo build --locked\n\n"
                "A pin is authoritative in both directions, so this is an error rather than a "
                "reason to look elsewhere.".format(
                    pinned, want, ", ".join("`{}`".format(m) for m in missing)
                )
            )
        return pinned

    tried = []
    stale = False
    candidates = [
        REPO_ROOT / "target" / "release" / "ethos-parser",
        REPO_ROOT / "target" / "debug" / "ethos-parser",
    ]
    found_on_path = shutil.which("ethos-parser")
    if found_on_path:
        candidates.append(pathlib.Path(found_on_path))

    for candidate in candidates:
        got = _binary_version(candidate)
        if got != want:
            tried.append("  {} (version {})".format(candidate, got or "absent"))
            continue
        missing = _missing_subcommands(candidate)
        if not missing:
            return str(candidate)
        # Right version, wrong build. Named distinctly from a version mismatch above, because the
        # two need different fixes and the message is all a reader gets.
        stale = True
        tried.append(
            "  {} (version {}, but STALE: no {})".format(
                candidate, got, ", ".join(missing)
            )
        )

    raise RuntimeError(
        "no `ethos-parser` binary at {} carrying {}. Tried, in order:\n{}\n\n"
        "    cargo build --locked\n\n"
        "or point ETHOS_PARSER at a build of this tree. This is a failure and not a skip: a "
        "green run against a parser from another version would prove something about the wrong "
        "engine.{}".format(
            want,
            ", ".join(REQUIRED_SUBCOMMANDS),
            "\n".join(tried) or "  (nothing)",
            "\n\nA candidate marked STALE reported the right version and is still the wrong "
            "build — a version string cannot tell two builds of one unreleased version apart. "
            "Rebuild it rather than looking for a version problem you do not have."
            if stale
            else "",
        )
    )


@pytest.fixture(scope="session", autouse=True)
def engine_binary():
    """Pin the binary for the whole session, the way a caller would."""
    binary = _locate_binary()
    os.environ["ETHOS_PARSER"] = binary
    return binary


@pytest.fixture(scope="session")
def fixture_pdf():
    assert FIXTURE_PDF.is_file(), "fixture missing: {}".format(FIXTURE_PDF)
    return FIXTURE_PDF


@pytest.fixture(scope="session")
def repo_root():
    return REPO_ROOT


@pytest.fixture(scope="session")
def cli(engine_binary):
    """Run the CLI directly, so a test can compare the SDK against it rather than against itself."""

    def run(*args):
        completed = subprocess.run(
            [engine_binary, *args],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
        assert completed.returncode == 0, completed.stderr.decode("utf-8", "replace")
        # The CLI ends its canonical JSON with a newline; the artifact is what precedes it.
        return completed.stdout.rstrip(b"\n")

    return run


@pytest.fixture(scope="session")
def cli_representation_bytes(cli, fixture_pdf):
    """The exact bytes ``ethos-parser extract`` printed — the thing byte-identity is measured against."""
    return cli("extract", str(fixture_pdf))
