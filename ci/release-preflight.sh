#!/usr/bin/env bash
# The checks that must pass before a release, run locally, before anything irreversible.
#
#   ci/release-preflight.sh
#
# # What this does NOT check, and why
#
# Three of the four invariants a release depends on are already tests, and they run inside
# `ci/gate.sh`, which this script invokes:
#
#   * the four version strings agree — `sdk_versions.rs::every_sdk_version_is_the_workspace_version`
#   * `profile_sha256` matches what this build produces — `profile.rs::the_default_profile_is_pinned`
#   * the local gate runs what CI runs — `v0_exit_criteria.rs::the_local_gate_runs_what_ci_runs`
#
# **Re-implementing any of them here would be the defect this repository keeps finding.** A second
# copy of a check goes stale exactly the way `00-NORTH-STAR.md` row 10 and `table-gate-v1.md` §2
# did, and then two things disagree about the same fact. So this script checks only what nothing
# else checks, and delegates.
#
# # What it cannot check
#
# That the version number is the RIGHT one. `docs/RELEASING.md` §4 is about that, and 0.42.1 is the
# worked example of getting it wrong: labelled a PATCH on the strength of one slice while the tree
# it shipped also carried a feature and two reader changes. No script can read that intent.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

fail() { printf '\n\033[1;31mrelease-preflight: %s\033[0m\n' "$1" >&2; shift; for l in "$@"; do echo "  $l" >&2; done; exit 1; }
ok()   { printf '\033[1;32m  ok\033[0m  %s\n' "$1"; }

version=$(sed -n '/^\[workspace\.package\]/,/^\[/{ s/^version *= *"\([^"]*\)".*/\1/p; }' Cargo.toml | head -1)
[ -n "$version" ] || fail "could not read the workspace version from Cargo.toml" \
  "Expected a \`version = \"...\"\` line under [workspace.package]."
printf '\n\033[1mrelease-preflight: %s\033[0m\n\n' "$version"

# 1. A dirty tree means the thing being tagged is not the thing being tested.
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  fail "the working tree has uncommitted changes." \
    "A tag names a commit. Releasing from a dirty tree publishes bytes that no commit contains" \
    "and no gate has run against. Commit or stash first:" "" "    git status --short" ""
fi
ok "working tree is clean"

# 2. A CHANGELOG entry for THIS version. Guarded by nothing until now, which is how 0.42.1 shipped
#    twenty-one commits and described one.
if ! grep -q "^## \[${version}\]" CHANGELOG.md; then
  fail "CHANGELOG.md has no entry for ${version}." \
    "Every released version gets an entry, and it is written before the release rather than" \
    "reconstructed from \`git log\` afterwards — 0.42.1 is the worked example of what that costs." \
    "" "    ## [${version}] — <what changed, in a sentence someone can act on>" ""
fi
ok "CHANGELOG.md has an entry for ${version}"

# 3. The tag must not already exist. A re-tagged version is two different trees under one name,
#    which is the state a version number exists to make impossible.
if git rev-parse -q --verify "refs/tags/v${version}" >/dev/null; then
  fail "tag v${version} already exists." \
    "That version has been tagged before. If it was never pushed or published, delete it" \
    "deliberately; if it WAS published, the number is spent and the next release takes the" \
    "next one — see docs/RELEASING.md §7." "" "    git tag -d v${version}" ""
fi
ok "no tag v${version} yet"

# 4. Everything else. The gate carries the three guards named in the header, so a green gate is
#    what makes them a fact here rather than an assumption.
printf '\n\033[1mrunning ci/gate.sh\033[0m\n\n'
if ! ci/gate.sh; then
  fail "the gate is red." \
    "A release is the one moment a red gate cannot be deferred: the three invariants in this" \
    "script's header are gate tests, and an unpublish does not exist."
fi

cat <<EOF

$(printf '\033[1;32mrelease-preflight: %s is ready to tag.\033[0m' "$version")

  Nothing irreversible has happened. The next steps are docs/RELEASING.md §5.2 onward, and
  the first of them that cannot be undone is 5.4 — the crates.io publish.

  Read §4 before tagging if the version number has not already been argued somewhere.
EOF
