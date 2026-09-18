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

"""**It cannot diverge from what the CLI prints**, which is the whole claim S2 makes.

The load-bearing test is :func:`test_extract_is_byte_identical_to_the_cli`: the SDK's return
value is re-canonicalized and compared against the bytes ``ethos-parser extract`` actually wrote to a
pipe. Structural equality would not do — it would pass a package that reordered a key or widened
an integer on the way through, and reordering a key is exactly how a fingerprint stops matching.

The rest of this file guards the conditions that make the claim cheap to keep: no runtime
dependency, no wrapped surface this slice refused, and a binary that is located rather than
guessed at.
"""

import ast
import copy
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

import ethos_parser
from ethos_parser import EngineFailed, EngineNotFound, NotARepresentation
from ethos_parser._c14n import c14n_bytes

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
#: Every source in the package, for the rules that bind regardless of entry point.
SOURCE_FILES = sorted((PACKAGE_ROOT / "src" / "ethos_parser").glob("*.py"))

#: What ``import ethos_parser`` reaches. The ``langchain`` submodule is deliberately not here:
#: it is behind an optional extra, and reaching it is a separate act.
DEFAULT_IMPORT_SOURCES = [p for p in SOURCE_FILES if p.name != "langchain.py"]


# --- byte identity --------------------------------------------------------------------------


def test_extract_is_byte_identical_to_the_cli(
    fixture_pdf, cli_representation_bytes, engine_binary
):
    artifact = ethos_parser.extract(fixture_pdf)
    assert c14n_bytes(artifact) == cli_representation_bytes, (
        "byte identity across the adapter, not merely structural equality: a key reordered or "
        "an integer widened on the way through is how a fingerprint stops matching"
    )
    # And the artifact's own declared digest still covers its own payload — the check
    # `node_get` runs, on bytes that went out through a pipe and came back through a parser.
    assert ethos_parser.node_get(artifact, artifact["representation"]["nodes"][0]["id"])


def test_extract_is_the_same_bytes_on_a_second_run(fixture_pdf):
    assert c14n_bytes(ethos_parser.extract(fixture_pdf)) == c14n_bytes(
        ethos_parser.extract(fixture_pdf)
    )


def test_ground_is_byte_identical_to_the_cli(fixture_pdf, cli, tmp_path):
    representation = ethos_parser.extract(fixture_pdf)

    on_disk = tmp_path / "representation.json"
    on_disk.write_bytes(c14n_bytes(representation))
    from_cli = cli("ground", str(on_disk))

    # The object, the way `ground(extract(pdf))` reads.
    assert c14n_bytes(ethos_parser.ground(representation)) == from_cli
    # And a path to bytes this engine wrote, which is the other half of what MCP accepts.
    assert c14n_bytes(ethos_parser.ground(on_disk)) == from_cli
    assert c14n_bytes(ethos_parser.ground(str(on_disk))) == from_cli


def test_ground_lets_the_engine_refuse_an_edited_representation(fixture_pdf):
    """The refusal is the engine's, not a second check here that could drift from it."""
    edited = copy.deepcopy(ethos_parser.extract(fixture_pdf))
    edited["representation"]["nodes"][0]["text"] = "Tampered"

    with pytest.raises(EngineFailed) as excinfo:
        ethos_parser.ground(edited)
    assert excinfo.value.returncode == 2
    assert "representation_c14n_sha256" in excinfo.value.stderr


def test_ground_refuses_an_object_that_will_not_canonicalize(fixture_pdf):
    """c14n v1 admits no float, so an object carrying one is not an artifact this engine wrote."""
    representation = ethos_parser.extract(fixture_pdf)
    representation["representation"]["nodes"][0]["probe"] = 1.5
    with pytest.raises(NotARepresentation):
        ethos_parser.ground(representation)


def _quote_file(directory, quote):
    """A file whose bytes are the quote's UTF-8 encoding, and nothing appended.

    What the SDK writes for itself, written here by hand, so the CLI reads exactly the bytes the
    adapter sent rather than a file this test formatted differently.
    """
    path = directory / "quote.txt"
    path.write_bytes(quote.encode("utf-8"))
    return path


def test_locate_is_byte_identical_to_the_cli(fixture_pdf, cli, tmp_path):
    representation = ethos_parser.extract(fixture_pdf)

    on_disk = tmp_path / "representation.json"
    on_disk.write_bytes(c14n_bytes(representation))
    from_cli = cli(
        "locate", str(on_disk), "--quote-file", str(_quote_file(tmp_path, "block"))
    )

    # Re-canonicalizing what came back reproduces the CLI's stdout bytes, so the answer IS the
    # engine's. That is the whole of why the match rule is not ported into this package
    # (`docs/26-LOCATE-SCOPE.md` §6.3): a ported rule would be a second implementation of the
    # answer, and two implementations of a text-matching rule can disagree.
    assert c14n_bytes(ethos_parser.locate(representation, "block")) == from_cli
    # And a path to bytes this engine wrote, which is the other half of what `ground` accepts.
    assert c14n_bytes(ethos_parser.locate(on_disk, "block")) == from_cli
    assert c14n_bytes(ethos_parser.locate(str(on_disk), "block")) == from_cli
    # Two blocks carry the word, so byte identity is measured on a found answer and not only on
    # the empty one below — an adapter that dropped `occurrences` would pass that and fail this.
    assert len(ethos_parser.locate(representation, "block")["occurrences"]) == 2


@pytest.mark.parametrize(
    "quote",
    ["a\nb", "a\x00b", "one\ntwo\x00three\n"],
    ids=["newline", "nul", "both"],
)
def test_a_quote_carrying_a_newline_or_a_nul_survives_the_adapter(
    quote, fixture_pdf, cli, tmp_path
):
    """**The test argv could not have passed**, which is why the quote travels in a file.

    `docs/26-LOCATE-SCOPE.md` §6.1: a NUL cannot appear in an argument at all, a newline survives
    only through correct quoting, and a quoting mistake changes the searched string *silently* —
    which changes what was searched with nothing on the wire saying so. So this reads the number
    of scalars the engine searched for back off the artifact and compares it with the string that
    was handed in, and then compares the whole artifact against the CLI's own bytes for a file
    holding that same string.
    """
    representation = ethos_parser.extract(fixture_pdf)
    on_disk = tmp_path / "representation.json"
    on_disk.write_bytes(c14n_bytes(representation))

    artifact = ethos_parser.locate(representation, quote)
    # `len` of a `str` is its scalar count, which is the unit `quote_scalars` counts in.
    assert artifact["quote_scalars"] == len(quote), (
        "the engine searched for a different number of scalars than the quote has, so the "
        "adapter did not deliver the string it was given"
    )
    assert c14n_bytes(artifact) == cli(
        "locate", str(on_disk), "--quote-file", str(_quote_file(tmp_path, quote))
    ), "and it is the artifact the CLI prints for a quote file holding those same bytes"


def test_a_quote_that_occurs_nowhere_is_an_answer_and_not_a_failure(fixture_pdf):
    """Decision #30's own bound: exit 0 and an empty list, never a refusal and never an exit 1.

    An empty ``occurrences`` list is the whole of the not-found answer, and it is the same
    artifact a found one produces — so nothing raises, and there is no field a caller could read
    as an opinion about whether anything holds.
    """
    quote = "zzz-nowhere-zzz"
    artifact = ethos_parser.locate(ethos_parser.extract(fixture_pdf), quote)
    assert artifact["artifact_type"] == "ethos.parser.locations.v0"
    assert artifact["occurrences"] == []
    assert artifact["quote_scalars"] == len(quote)
    assert artifact["searched"]["blocks"] > 0, (
        "a record nothing was searched in would report absence vacuously"
    )


def test_an_empty_quote_is_a_refusal_and_not_the_not_found_answer(fixture_pdf):
    """§4.1: the empty string occurs at every offset of every block, so *where* has no answer.

    The adapter keeps that apart from the empty answer above rather than collapsing the two —
    collapsing them would tell a caller that a string occurring at every position occurs at none.
    """
    with pytest.raises(EngineFailed) as excinfo:
        ethos_parser.locate(ethos_parser.extract(fixture_pdf), "")
    assert excinfo.value.returncode == 2
    assert "empty" in excinfo.value.stderr


def test_a_document_the_engine_cannot_read_is_a_named_failure(tmp_path):
    not_a_pdf = tmp_path / "document.pdf"
    not_a_pdf.write_bytes(b"this is not a PDF")
    with pytest.raises(EngineFailed) as excinfo:
        ethos_parser.extract(not_a_pdf)
    assert excinfo.value.returncode == 2
    assert excinfo.value.stderr.strip(), "the engine's own words must survive the wrapper"


# --- the surface this slice refused ----------------------------------------------------------


@pytest.mark.parametrize("absent", ["markdown", "html", "verify", "mcp", "classify"])
def test_the_surfaces_this_slice_did_not_wrap_are_absent(absent):
    """`markdown` and `html` arrive when a caller needs them; `verify` is a boundary.

    `docs/07-VERIFY-BOUNDARY.md`: the engine invokes a verifier, it never verifies. A Python
    function of that name would look like this package had an opinion about whether a claim is
    supported.
    """
    assert absent not in ethos_parser.__all__
    assert not callable(getattr(ethos_parser, absent, None))


def test_no_source_file_carries_the_vocabulary_the_repository_bans():
    """The tokens `ci/forbidden-tokens.sh` scans crate sources for, applied to this language too.

    Scoped to ``src/`` for the reason that script scopes itself to ``crates/*/src``: this file
    lists the tokens as data, and scanning it would fail the build for containing the check.
    """
    banned = [
        "quality_score",
        "trust_score",
        "evidence_tier",
        "is_grounded",
        "verdict",
        "verify_claim",
        "claim_verified",
    ]
    assert SOURCE_FILES, "an empty scan would pass vacuously"
    for path in SOURCE_FILES:
        text = path.read_text(encoding="utf-8").lower()
        for token in banned:
            assert token not in text, "`{}` appears in {}".format(token, path.name)


# --- no runtime dependency -------------------------------------------------------------------


def test_pyproject_declares_no_runtime_dependency():
    text = (PACKAGE_ROOT / "pyproject.toml").read_text(encoding="utf-8")
    assert re.search(r"^dependencies = \[\]$", text, re.MULTILINE), (
        "the runtime dependency list must be literally empty; a wheel that pulled a package to "
        "hand back bytes the CLI already printed adds a place for those bytes to change"
    )


def test_every_module_the_default_import_reaches_is_stdlib():
    """Read off the import statements, so the declaration above cannot be true only on paper.

    Scoped to the default import surface: ``ethos_parser.langchain`` reaches its optional extra,
    and :func:`test_the_langchain_module_reaches_its_extra_and_nothing_else` is what bounds it.
    """
    stdlib = sys.stdlib_module_names
    assert DEFAULT_IMPORT_SOURCES, "an empty scan would pass vacuously"
    for path in DEFAULT_IMPORT_SOURCES:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                roots = [alias.name.split(".")[0] for alias in node.names]
            elif isinstance(node, ast.ImportFrom):
                # `level > 0` is a relative import — this package importing itself.
                roots = [] if node.level else [(node.module or "").split(".")[0]]
            else:
                continue
            for root in roots:
                assert root in stdlib or root == "ethos_parser", (
                    "{} imports `{}`, which is neither stdlib nor this package".format(
                        path.name, root
                    )
                )


def test_the_langchain_module_reaches_its_extra_and_nothing_else():
    """``langchain_core`` is the one non-stdlib import allowed anywhere in this package.

    And it is guarded: importing the module without the extra is a named ``ImportError`` carrying
    the install command, not Python's bare "No module named".
    """
    path = PACKAGE_ROOT / "src" / "ethos_parser" / "langchain.py"
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    roots = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            roots.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and not node.level:
            roots.add((node.module or "").split(".")[0])

    allowed = set(sys.stdlib_module_names) | {"ethos_parser", "langchain_core"}
    assert roots <= allowed, "langchain.py reaches {}".format(sorted(roots - allowed))

    # `langgraph` is absent from `roots` above, which is the rule. The word itself appears in the
    # module docstring explaining why there is no LangGraph adapter — and banning a word a
    # document needs in order to argue against it is the mistake `ci/forbidden-tokens.sh` strips
    # comments to avoid.
    assert "langgraph" not in roots

    source = path.read_text(encoding="utf-8")
    assert "pip install 'ethos-parser[langchain]'" in source, (
        "the failure must carry the install command"
    )


# --- identity ---------------------------------------------------------------------------------


def test_the_sdk_version_is_the_workspace_version(repo_root):
    """An SDK claiming a version the engine does not is the lie `parser_version` exists to stop."""
    cargo = (repo_root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([^"]+)"$', cargo, re.MULTILINE)
    assert match, "no workspace version in Cargo.toml"
    assert ethos_parser.__version__ == match.group(1)


def test_the_representation_artifact_type_is_the_one_the_engine_stamps(fixture_pdf):
    artifact = ethos_parser.extract(fixture_pdf)
    assert (
        artifact["artifact_type"] == ethos_parser.REPRESENTATION_ARTIFACT_TYPE
    ), "the constant this package matches on must be the string the engine actually writes"


# --- locating the binary -----------------------------------------------------------------------


def test_an_explicit_pin_is_authoritative(monkeypatch, tmp_path, fixture_pdf):
    """A named path that is not there is an error, never a reason to find some other build.

    `VerifierBinary::resolve` sets the precedent for `ETHOS_BIN` and the reason carries: resolving
    to a binary nobody chose means returning artifacts from a parser nobody chose.
    """
    monkeypatch.setenv("ETHOS_PARSER", str(tmp_path / "no-such-engine"))
    with pytest.raises(EngineNotFound) as excinfo:
        ethos_parser.extract(fixture_pdf)
    assert "no-such-engine" in str(excinfo.value)


def test_no_binary_anywhere_is_a_named_failure(monkeypatch, fixture_pdf):
    monkeypatch.delenv("ETHOS_PARSER", raising=False)
    monkeypatch.setattr(shutil, "which", lambda _name: None)
    with pytest.raises(EngineNotFound) as excinfo:
        ethos_parser.extract(fixture_pdf)
    message = str(excinfo.value)
    assert "ETHOS_PARSER" in message
    assert "PATH" in message
    assert "cargo build" in message, "the failure must carry the command that fixes it"


def test_the_engine_inherits_the_environment_rather_than_one_this_package_composed():
    """No ``env=`` anywhere: handing the engine an environment this package assembled would be
    one more input to a deterministic parser that nobody declared."""
    source = (PACKAGE_ROOT / "src" / "ethos_parser" / "__init__.py").read_text(
        encoding="utf-8"
    )
    tree = ast.parse(source)
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            for keyword in node.keywords:
                assert keyword.arg != "env", "a subprocess call sets `env=`"


def test_it_is_the_engine_that_runs(engine_binary, fixture_pdf):
    """The SDK is another caller of the same binary, so the same invocation is available to a shell."""
    assert os.path.isfile(engine_binary)
    completed = subprocess.run(
        [engine_binary, "extract", str(fixture_pdf)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    assert completed.returncode == 0
    assert c14n_bytes(ethos_parser.extract(fixture_pdf)) == completed.stdout.rstrip(
        b"\n"
    )
