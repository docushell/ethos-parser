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
# The optional peer has to be INSTALLED or the half of this suite that covers it does not run.
#
# `@langchain/core` is an optional `peerDependency`, which is correct for consumers — importing
# this package must pull nothing, and `the_default_import_does_not_reach_langchain` asserts it.
# But `node --test` with no install step meant `import("@langchain/core/tools")` threw, the suite
# took its documented skip, and **fourteen of eighty tests never ran while the run exited 0**. The
# Python half installs its `[langchain]` extra thirty lines below and has always executed its
# equivalents; the Node half never has. v1.2 ships "LangChain tools over both" and only one was
# gated.
#
# It is a devDependency rather than an `npm install <spec>`: npm resolves a spec that is already an
# OPTIONAL peer as "up to date" and installs nothing, exiting 0 while doing so — measured, on the
# commit that added this. devDependencies are not installed for consumers, so the runtime contract
# is unchanged and the published dependency surface does not move.
if ! ( cd packages/node && npm install --no-audit --no-fund --loglevel=error ); then
  echo >&2
  echo "ci/sdk-suites.sh: \`npm install\` failed in packages/node." >&2
  echo >&2
  echo "  The Node suite needs \`@langchain/core\` — a devDependency here, an OPTIONAL peer for" >&2
  echo "  consumers — or fourteen of its eighty tests skip and the run still exits 0. That is the" >&2
  echo "  state this repository shipped in until 0.42.1, so the install is not optional here." >&2
  echo >&2
  echo "  Same posture as the Python half below: a missing dependency is a failure and never a" >&2
  echo "  skip. If this is a registry or proxy problem rather than a real resolution failure," >&2
  echo "  \`npm config get registry\` is the first thing to check — a registry that cannot be" >&2
  echo "  reached fails here rather than silently producing a partial gate." >&2
  echo >&2
  echo "  Behind a corporate proxy, the registry is reachable but the step still fails, twice" >&2
  echo "  over: ENOTFOUND with the VPN down, then CERT_HAS_EXPIRED with it up, both against an" >&2
  echo "  internal mirror. Exporting the proxy FOR THE RUN fixes it and changes no npm config:" >&2
  echo >&2
  echo "    export http_proxy=http://www-proxy:80/" >&2
  echo "    export https_proxy=http://www-proxy.us.oracle.com:80/" >&2
  echo "    export no_proxy=localhost,127.0.0.1,.us.oracle.com,.au.oracle.com,.oraclecorp.com" >&2
  echo >&2
  echo "  Those hosts are one organisation\'s; the shape is what transfers. Setting a registry" >&2
  echo "  with \`npm config set\` would outlive the run and is deliberately not what is advised" >&2
  echo "  here — a gate that edits your tooling to pass is a gate nobody can trust." >&2
  exit 1
fi

# And the skip is now a failure, for the reason the Python block below gives about its own version
# guard: a suite that quietly omits a third of itself is how it omitted a third of itself for six
# minor versions. `node --test` reports skips on stdout and still exits 0, so the count is read.
#
# Written this way for two reasons, both of them defects the first draft of this guard had:
#
#   * The output is captured to a file rather than piped, because a pipeline takes the exit status
#     of its LAST command — `node --test | sed` reports sed's success and swallows a real test
#     failure. The status is read from the run itself.
#   * The count is matched with `.*skipped[[:space:]]*\([0-9][0-9]*\)` rather than anchored at the
#     start of the line. Node prefixes its summary with `ℹ`, a multi-byte character, so an anchored
#     `^. skipped` matched nothing, the count parsed EMPTY, and `${x:-0}` turned that into zero —
#     a guard that read its own subject wrongly and passed forever, which is the defect this whole
#     block exists to remove. It was measured failing that way before it was measured working.
node_log=$( mktemp )
( cd packages/node && node --test ) > "$node_log" 2>&1
node_status=$?
cat "$node_log"
node_skips=$( sed -n 's/.*skipped[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$node_log" | tail -1 )
rm -f "$node_log"
if [ "$node_status" -ne 0 ]; then
  echo "ci/sdk-suites.sh: the Node suite failed." >&2
  exit 1
fi
if [ -z "$node_skips" ]; then
  echo "ci/sdk-suites.sh: could not read a skip count from the Node suite output." >&2
  echo "  The guard cannot confirm every test ran, so it fails rather than assume they did." >&2
  exit 1
fi
if [ "$node_skips" != "0" ]; then
  echo >&2
  echo "ci/sdk-suites.sh: ${node_skips} Node test(s) SKIPPED, and a skip here is a failure." >&2
  echo >&2
  echo "  The suite skips when \`@langchain/core\` cannot be imported. It is a devDependency, so" >&2
  echo "  the \`npm install\` above should have supplied it — a skip means that install did not" >&2
  echo "  do what it reported. npm exits 0 having installed nothing when it decides a package is" >&2
  echo "  an optional peer it may ignore, which is exactly the state this guard exists to catch." >&2
  echo >&2
  echo "  Not a skip, because the tests it hides are the only coverage the Node LangChain adapter" >&2
  echo "  has, and they were absent from every green this repository recorded before 0.42.1." >&2
  exit 1
fi

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
