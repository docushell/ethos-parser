#!/usr/bin/env bash
# The Node and Python SDK suites, run the same way by CI and by `ci/gate.sh`.
#
#   ci/sdk-suites.sh
#
# # Why this exists
#
# Both SDKs carry a test asserting their version against `[workspace.package] version` in
# `Cargo.toml`, and each says so in a comment next to the constant. At v2-S15 the four numbers
# were:
#
#   Cargo.toml                                    0.42.1
#   packages/python/src/ethos_parser/__init__.py  0.36.1
#   packages/node/src/index.js                    0.36.1
#   packages/node/package.json                    0.36.3
#
# Six minor versions of drift behind three guards, and the two node files disagreed with each
# other inside one package. The guards were correct; nothing had ever executed them, because
# `ci.yml` ran no `node --test` and no `pytest`. That is the same liveness failure as 0.31.1's
# "guards that were never there" — an assertion nobody runs is indistinguishable from no
# assertion, and it is worse, because the comment beside it says the number is checked.
#
# # A script rather than two workflow steps
#
# `the_local_gate_runs_what_ci_runs` compares `ci/gate.sh` against the `check` job character for
# character, and reads the script's commands as the lines beginning `cargo ` or `ci/`. So a check
# that both must run is a `ci/*.sh` invocation — the shape `ci/forbidden-tokens.sh` already uses.
# Putting the suites here is what lets CI and the local gate run the identical command.
#
# # The virtualenv
#
# The Python SDK's suite needs pytest, and its `langchain` extra, because `tests/test_langchain.py`
# imports `ethos_parser.langchain` — which raises a named `ImportError` without it rather than
# degrading, so omitting the extra turns a real suite into a collection error.
#
# Those go into a venv under `target/`, not into whatever interpreter happens to be active. This
# script runs on contributors' machines, and a gate that pip-installs into a developer's global
# environment is a gate that changes the machine it was asked to measure.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$(pwd)"

# The binary's absence is a failure, never a skip (`docs/04-ARCHITECTURE.md` §4). Both SDK
# locators additionally REFUSE a binary whose `--version` disagrees with the workspace, because a
# stale `target/release` answers every question plausibly and would prove the SDK does not alter
# what the CLI prints — about the wrong CLI. So build before running, and say so if it is missing.
if [ ! -x target/debug/ethos-parser ] && [ ! -x target/release/ethos-parser ]; then
  echo "ci/sdk-suites.sh: no ethos-parser binary. Run:" >&2
  echo >&2
  echo "    cargo build --locked" >&2
  echo >&2
  echo "Both SDK suites locate a binary and check its version against Cargo.toml; a missing one" >&2
  echo "is a failure rather than a skip, so this stops here instead of running a partial gate." >&2
  exit 1
fi

printf '\n\033[1m[sdk 1/2] node — packages/node\033[0m\n'
if ! command -v node >/dev/null 2>&1; then
  echo "ci/sdk-suites.sh: no \`node\` on PATH. The Node SDK suite is a gate, not an optional" >&2
  echo "extra — it is what caught 0.36.1 sitting in a 0.42.0 tree. Install Node >= 18." >&2
  exit 1
fi
( cd packages/node && node --test )

printf '\n\033[1m[sdk 2/2] python — packages/python\033[0m\n'

# `packages/python/pyproject.toml` declares `requires-python = ">=3.10"`, and pip enforces it —
# on a machine whose `python3` is older the install fails with "requires a different Python",
# which is true but says nothing about what to do. macOS still ships 3.9 as `python3`, so this is
# the common case rather than the exotic one. Search the named interpreters before giving up, and
# if none qualifies, fail with the floor and the reason rather than with pip's resolver error.
PY_BIN=""
for candidate in python3.13 python3.12 python3.11 python3.10 python3; do
  command -v "$candidate" >/dev/null 2>&1 || continue
  if "$candidate" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null; then
    PY_BIN="$candidate"
    break
  fi
done
if [ -z "$PY_BIN" ]; then
  echo "ci/sdk-suites.sh: no Python >= 3.10 on PATH." >&2
  echo >&2
  echo "  packages/python/pyproject.toml declares requires-python = \">=3.10\", so the suite" >&2
  echo "  cannot run on an older interpreter. Tried: python3.13 python3.12 python3.11" >&2
  echo "  python3.10 python3 (found $(python3 -V 2>&1 || echo none))." >&2
  echo >&2
  echo "  This is a failure and not a skip: the version guard here is what caught 0.36.1" >&2
  echo "  sitting in a 0.42.0 tree, and a gate that quietly omitted it is how it got there." >&2
  exit 1
fi
printf 'python: %s (%s)\n' "$PY_BIN" "$("$PY_BIN" -V 2>&1)"

# Keyed by interpreter version, so switching pythons rebuilds rather than reusing a venv built
# against the old one.
VENV="$ROOT/target/sdk-venv-$("$PY_BIN" -c 'import sys; print("%d.%d" % sys.version_info[:2])')"
if [ ! -x "$VENV/bin/python" ]; then
  printf 'creating %s\n' "$VENV"
  "$PY_BIN" -m venv "$VENV"
fi
# `--quiet` twice over: this is a gate, and its output should be the suites' output.
"$VENV/bin/python" -m pip install --quiet --upgrade pip >/dev/null
"$VENV/bin/python" -m pip install --quiet -e "packages/python[langchain]" pytest
( cd packages/python && "$VENV/bin/python" -m pytest tests/ -q )

printf '\n\033[1msdk suites: green\033[0m\n'
