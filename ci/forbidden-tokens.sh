#!/usr/bin/env bash
# Two of `docs/03-V0-SCOPE.md` §5's lines are greps, so they are a grep — run by CI as the jobs
# `v0-no-confidence` and `v0-no-verify`.
#
#   ci/forbidden-tokens.sh confidence     # §5: `grep -ri confidence` returns nothing
#   ci/forbidden-tokens.sh verification   # §5: no verification code, type, or field exists
#
# A script rather than inline YAML so the gate can be run locally, exactly as CI runs it, and so
# the two cannot drift.
#
# # Scope: production sources only
#
# `crates/*/src/**.rs`. Not `tests/`, not `docs/`, not this file. The rule is about what the
# engine *is*, and the repository argues the rule at length in prose — `docs/07-VERIFY-BOUNDARY.md`
# exists to explain why verification is absent, and it cannot do that without naming it.
#
# # Two exclusions, both narrow
#
# 1. **`//` comments.** Same reason: the crate docs argue these rules, and a doc comment
#    explaining why there is no confidence field is not a confidence field. Rust's own guard
#    tests (`ethos-parser-core/tests/contract_invariants.rs`) strip comments the same way, and there
#    are no `/* */` block comments anywhere in `crates/*/src` — asserted below rather than hoped.
#
# 2. **The `mod tests { … }` block, and only that block.** Two test modules list the banned
#    tokens *as data* to assert their absence, so grepping them would fail the build for
#    containing the test that enforces the rule.
#
#    The skip runs from `^mod tests {` to the matching `^}` — **not** to end of file. The first
#    version did run to EOF, on the reasoning that test modules come last. They do today, and
#    that was still wrong: a probe appended after a test module sailed through the gate clean.
#    Scanning resumes after the closing brace, so nothing below a test module is invisible.
#
# Both exclusions err toward reporting: a token in a construct this does not understand shows up
# as a hit, and a false alarm is cheap next to a missed one.

set -euo pipefail

mode="${1:?usage: forbidden-tokens.sh <confidence|verification>}"
cd "$(dirname "${BASH_SOURCE[0]}")/.."

case "$mode" in
  confidence)
    # `docs/01-CONTRACT.md` §9 and Workbench rule 9. The measured case for the rule was produced
    # by the feature itself: pdf-inspector reports `TEXT-BASED, Confidence: 50%, Pages with text:
    # 0` — a verdict its own evidence contradicts.
    pattern='confidence|quality_score|trust_score|is_good'
    why='docs/01-CONTRACT.md §9 forbids any public confidence field, score, grade, or quality
summary — at any version, including "diagnostic only". The v2.1 OCR lane may record a
processor-reported uncertainty, and when it does it goes on a recognition type under its own
profile, never on these.'
    ;;
  verification)
    # `docs/07-VERIFY-BOUNDARY.md`: Stage 0. The engine validates structure and binding; it never
    # verifies a claim. Bare `grounded` is deliberately NOT in this list — `GroundedBox` is real,
    # supported API for "a box that has geometry", and banning the substring would forbid the
    # type whose whole job is refusing to invent one.
    pattern='evidence_tier|is_grounded|all_evidence_grounded|verdict|verify_claim|claim_verified'
    why='docs/07-VERIFY-BOUNDARY.md: the engine does not verify. No claim, no verdict, no
`grounded`, no evidence tier. Reimplementing verifier semantics is how a second authority is
born by accident — and two engines that disagree about whether a document supports a claim is
the failure this whole project is arranged to prevent.'
    ;;
  *)
    echo "unknown mode \`$mode\`; expected confidence or verification" >&2
    exit 2
    ;;
esac

files=$(find crates -path '*/src/*' -name '*.rs' | sort)
if [ -z "$files" ]; then
  echo "::error::no crate sources found; the scan would pass vacuously" >&2
  exit 1
fi

# The scannable corpus: comments stripped, test modules skipped. Computed once, because the
# block-comment guard below has to inspect the same text this scans and not the raw file.
code=$(
  for f in $files; do
    awk -v F="$f" '
      # Skip the test module only — two of them list these tokens as data — and resume at its
      # closing brace, so code after a test module is still scanned.
      /^mod tests \{/ { in_tests = 1; next }
      in_tests && /^\}/ { in_tests = 0; next }
      in_tests { next }
      { line = $0; sub(/[ \t]*\/\/.*$/, "", line); if (line ~ /[^ \t]/) print F ":" NR ": " line }
    ' "$f"
  done
)

# Guard the exclusion: `//`-stripping is only safe because there is no block comment to hide a
# token inside. If one ever lands, say so rather than silently under-scanning.
#
# Checked against the STRIPPED text, not the raw file. The first version scanned the raw file and
# fired on `crates/*/src` written in a doc comment — prose about the scan's own scope, containing
# `/*`, failing the scan. A `/*` inside a `//` comment is not a block comment, and a guard that
# cannot tell the difference is one somebody eventually disables.
if printf '%s\n' "$code" | grep -q '/\*'; then
  echo "::error::a /* */ block comment appeared in crates/*/src; this scan only strips // comments" >&2
  printf '%s\n' "$code" | grep -n '/\*' >&2
  exit 1
fi

hits=$(printf '%s\n' "$code" | grep -inE "$pattern" || true)

scanned=$(echo "$files" | wc -l | tr -d ' ')

if [ -n "$hits" ]; then
  echo "::error::forbidden \`$mode\` token(s) in production sources:"
  echo "$hits"
  echo
  echo "$why"
  exit 1
fi

echo "clean: no \`$mode\` token in $scanned crate source file(s)."
echo "pattern: $pattern"
