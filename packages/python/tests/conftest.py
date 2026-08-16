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

"""Shared fixtures.

**The binary's absence is a failure, never a skip** (`docs/04-ARCHITECTURE.md` §4). A suite that
went green because it could not find the thing it tests would be the worst outcome available, so
:func:`engine_binary` raises with the command that fixes it rather than calling ``pytest.skip``.
"""

import os
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]

#: An in-tree fixture, so this suite depends on nothing outside the repository. Two text runs on
#: one page: enough that a node lookup is a lookup rather than a coin flip, and small enough that
#: a byte-identity failure is readable.
FIXTURE_PDF = REPO_ROOT / "fixtures" / "engine" / "markdown-two-blocks" / "document.pdf"


def _locate_binary():
    pinned = os.environ.get("ETHOS_ENGINE")
    if pinned:
        return pinned
    for candidate in (
        REPO_ROOT / "target" / "release" / "engine",
        REPO_ROOT / "target" / "debug" / "engine",
    ):
        if candidate.is_file():
            return str(candidate)
    found = shutil.which("engine")
    if found:
        return found
    raise RuntimeError(
        "no `engine` binary to test against. Build one:\n\n"
        "    cargo build --locked\n\n"
        "or point ETHOS_ENGINE at an existing build. This is a failure and not a skip: a green "
        "run that never reached the engine would prove nothing."
    )


@pytest.fixture(scope="session", autouse=True)
def engine_binary():
    """Pin the binary for the whole session, the way a caller would."""
    binary = _locate_binary()
    os.environ["ETHOS_ENGINE"] = binary
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
    """The exact bytes ``engine extract`` printed — the thing byte-identity is measured against."""
    return cli("extract", str(fixture_pdf))
