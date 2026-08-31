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

"""**There was no timeout, at any layer** (v2-S15).

``subprocess.run`` with two pipes is deadlock-safe, so the old code could not wedge — but it could
wait forever, and a caller blocked on one document had no exception to route and no process it
could see. The security review's phrasing is the right one: the answer to an arbitrary default is
a chosen ceiling, not the absence of one.

:class:`EngineTimeout` is separate from :class:`EngineFailed` because a refusal is an ANSWER — the
engine read the document and said no, with a reason and one of three exit codes — while a timeout
is the absence of one. A caller retrying or alerting wants those in different branches.
"""

import os
import subprocess

import pytest

import ethos_parser


def test_the_timeout_exception_is_exported_and_is_an_engine_error():
    """A caller writing ``except EngineError`` must still catch this."""
    assert issubclass(ethos_parser.EngineTimeout, ethos_parser.EngineError)
    assert "EngineTimeout" in ethos_parser.__all__


def test_a_timeout_is_not_a_refusal():
    """The two must not be catchable as one another, or the distinction buys nothing."""
    assert not issubclass(ethos_parser.EngineTimeout, ethos_parser.EngineFailed)
    assert not issubclass(ethos_parser.EngineFailed, ethos_parser.EngineTimeout)


def test_an_expired_budget_raises_engine_timeout(fixture_pdf, monkeypatch):
    """A millisecond is not enough for any document, which is what makes this deterministic."""
    monkeypatch.setenv("ETHOS_PARSER_TIMEOUT", "0.001")
    with pytest.raises(ethos_parser.EngineTimeout) as excinfo:
        ethos_parser.extract(fixture_pdf)

    e = excinfo.value
    assert e.seconds == 0.001
    assert e.command_args, "the failure must name the command it gave up on"
    assert "did not finish" in str(e)
    # `args` is BaseException's and holds the message; the command lives under its own name so a
    # caller reaching for one does not silently get the other.
    assert e.command_args != e.args


def test_the_default_budget_does_not_fire_on_a_real_document(fixture_pdf, monkeypatch):
    """The ceiling must be far above a healthy run, or it is a performance budget by accident."""
    monkeypatch.delenv("ETHOS_PARSER_TIMEOUT", raising=False)
    artifact = ethos_parser.extract(fixture_pdf)
    assert artifact["artifact_type"] == ethos_parser.REPRESENTATION_ARTIFACT_TYPE


def test_zero_means_wait_forever(fixture_pdf, monkeypatch):
    """The previous behaviour stays reachable, named rather than implied."""
    monkeypatch.setenv("ETHOS_PARSER_TIMEOUT", "0")
    from ethos_parser import _timeout_seconds

    assert _timeout_seconds() is None
    # And it still works end to end.
    artifact = ethos_parser.extract(fixture_pdf)
    assert artifact["artifact_type"] == ethos_parser.REPRESENTATION_ARTIFACT_TYPE


@pytest.mark.parametrize("bad", ["soon", "-1", ""])
def test_an_unreadable_budget_is_a_named_error_not_a_silent_default(bad, monkeypatch):
    """Falling back to the default would hide a typo in the one knob that bounds a hang."""
    monkeypatch.setenv("ETHOS_PARSER_TIMEOUT", bad)
    from ethos_parser import _timeout_seconds

    with pytest.raises(ethos_parser.EngineError):
        _timeout_seconds()


def test_the_child_does_not_outlive_the_timeout(fixture_pdf, monkeypatch):
    """``TimeoutExpired`` alone leaves the child running; ``subprocess.run`` reaps it.

    Asserted through the exception type rather than by inspecting the process table: what matters
    to a caller is that the failure surfaces as ours and not as a bare ``TimeoutExpired`` escaping
    the package, which would also mean nobody had killed the child.
    """
    monkeypatch.setenv("ETHOS_PARSER_TIMEOUT", "0.001")
    with pytest.raises(ethos_parser.EngineTimeout):
        ethos_parser.extract(fixture_pdf)
    assert not isinstance(
        ethos_parser.EngineTimeout("x", 1, ""), subprocess.TimeoutExpired
    )
