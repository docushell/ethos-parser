#!/usr/bin/env bash
# Every place that states a version must state the workspace's version.
#
#   ci/doc-version.sh
#
# # Why this exists
#
# `docs/CAPABILITY.md` opens with its own rule — *"When the workspace version moves, this page moves
# with it or it is wrong"* — and nothing enforced it. At 0.54.0 three files still said 0.50.0:
# `README.md`, `docs/README.md` and `docs/CAPABILITY.md`. That is four releases of drift on the page
# whose entire job is to say what the engine can do today, and the drift was invisible because the
# rule lived in prose.
#
# The same failure `ci/forbidden-tokens.sh` was written for: a rule a human is expected to remember
# is a rule that goes red silently. A script rather than a paragraph, so the gate can run it locally
# exactly as CI runs it.
#
# # What it does NOT check
#
# It checks the version STRING, not whether the claims beside it are still true. Nothing can check
# that. `CAPABILITY.md`'s rule is about the page moving with the version, and this catches only the
# half a machine can see — the number. Re-reading the rows is still a human step at release time,
# and `ci/release-preflight.sh` is where that belongs.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# The workspace version is the single source of truth. `[workspace.package]` carries it.
want="$(awk '/^\[workspace\.package\]/{f=1;next} f&&/^version = /{gsub(/[":]/,"");print $3;exit}' Cargo.toml)"
if [[ -z "$want" ]]; then
  printf 'doc-version: could not read the workspace version from Cargo.toml\n' >&2
  exit 2
fi
printf 'doc-version: workspace is %s\n' "$want"

fail=0
check() {
  local file="$1" label="$2" pattern="$3"
  if [[ ! -f "$file" ]]; then
    printf '  MISSING  %s\n' "$file" >&2
    fail=1
    return
  fi
  # Every version-shaped string on the matching lines must equal $want.
  local found
  found="$(grep -nE "$pattern" "$file" 2>/dev/null || true)"
  if [[ -z "$found" ]]; then
    printf '  NO CLAIM %s — expected a %s line; the pattern moved and this check went blind\n' \
      "$file" "$label" >&2
    fail=1
    return
  fi
  while IFS= read -r line; do
    local got
    got="$(printf '%s' "$line" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)"
    if [[ "$got" != "$want" ]]; then
      printf '  STALE    %s:%s says %s, workspace is %s\n' \
        "$file" "${line%%:*}" "$got" "$want" >&2
      fail=1
    else
      printf '  ok       %s (%s)\n' "$file" "$label"
    fi
  done <<< "$found"
}

check README.md              'Version N.N.N.'                  '^Version [0-9]+\.[0-9]+\.[0-9]+\.'
check docs/README.md         '**Version N.N.N.**'              '^\*\*Version [0-9]+\.[0-9]+\.[0-9]+\.\*\*'
check docs/CAPABILITY.md     'These tables describe N.N.N.'    'These tables describe [0-9]+\.[0-9]+\.[0-9]+\.'
check packages/node/package.json '"version"'                   '^\s*"version":\s*"[0-9]+\.[0-9]+\.[0-9]+"'

# `packages/python/pyproject.toml` declares `dynamic = ["version"]`, so it has no literal to drift.
# Recorded here rather than left silent: a reader looking for the Python SDK in this list should
# find the reason it is absent, not conclude the check forgot it.

if (( fail )); then
  printf '\ndoc-version: FAILED. Update the file, or the workspace version, so they agree.\n' >&2
  exit 1
fi
printf 'doc-version: all version claims agree\n'
