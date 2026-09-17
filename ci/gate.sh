#!/usr/bin/env bash
# The whole gate, run locally, exactly as CI runs it.
#
#   ci/gate.sh
#
# # Why this exists
#
# `.github/workflows/ci.yml` did not run at all until 0.41.0, when this repository gained a
# remote at `docushell/ethos-parser`. Every "green" any record in `CHANGELOG.md` claims before
# that is a local partial run — somebody's `cargo test`, with whichever checks they happened to
# remember — and this script is still what makes green a fact BEFORE a push rather than after. `cargo fmt --all --check` was red from v2-S14
# until v2-S18 repaired it, across four slices that each recorded a green, because nothing ran it.
#
# A script rather than a paragraph in a record, for the reason `ci/forbidden-tokens.sh` already
# gives: so the gate can be run locally, exactly as CI runs it, and so the two cannot drift.
#
# # The second source of truth, and the guard that stops it
#
# A convenience script nobody verified against CI is worse than no script, because a local green
# then means something CI does not enforce — it manufactures confidence instead of measuring it.
#
# So this file is **not the authority**. `ci.yml` is, and
# `crates/ethos-parser-cli/tests/v0_exit_criteria.rs::the_local_gate_runs_what_ci_runs` asserts the two
# carry the same commands in **both** directions: every check CI runs appears here, and every
# check here appears in CI. That test runs inside `cargo test --workspace`, which is step 6
# below — so running this script is itself the proof that this script still matches the workflow.
#
# The commands are therefore written out **verbatim**, character for character as `ci.yml` spells
# them. That is deliberate and the guard depends on it: a "tidied" flag ordering here would be a
# different command from the one CI runs, and the guard would say so.
#
# # What it does not run, and why each is excluded rather than forgotten
#
# The exclusions are the part worth reading. The guard asserts this list is **complete** — a new
# step added to the `check` job is neither listed here nor excluded there, and fails the test.
#
#   * **The oracle build** (`check` → "Build the oracle", and the two steps around it). CI
#     compiles `docushell/ethos` at `ETHOS_ORACLE_REF` into `ethos-oracle/target`. Building it
#     here would write into a tree this repository does not own, so this script locates one
#     instead of producing one — see the `ETHOS_BIN` block below. If none is found the oracle
#     tests fail loudly rather than skipping — `docs/04-ARCHITECTURE.md` §4 — which is the
#     correct signal and is left alone.
#
#     **This paragraph used to say the suite resolved a sibling checkout "on its own, so no
#     environment pin is needed", and that was wrong.** `VerifierBinary::resolve` does try
#     `../ethos/target/release/ethos`, but relative to the *caller's working directory*, and
#     `cargo test` runs each test binary with the **crate** directory as its cwd — so the
#     candidate becomes `crates/ethos/target/release/ethos` and is never found. Nine tests in
#     `html_cli`, `markdown_cli` and `verify_relay` therefore failed on every local run, for a
#     reason unrelated to whatever the developer had changed. A gate that is red for everybody
#     all the time is a gate people learn to skip.
#
#   * **The toolchain-pin tripwire** (`check` → "Assert the toolchain pin is in force"). It exists
#     because a CI runner installs a toolchain from the workflow and could disagree with
#     `rust-toolchain.toml`. Locally rustup reads that file on every `cargo` invocation and there
#     is no second toolchain to disagree with it.
#
#   * **`v0-fuzz-smoke`.** Needs a nightly toolchain and `cargo-fuzz` for `-Z sanitizer=address`.
#     The workspace MSRV is 1.88 and no other job installs nightly.
#
#   * **`deny-policy-is-enforced`.** It flips `crates/ethos-parser-core/Cargo.toml` to AGPL and restores
#     it with `git checkout --`, which silently discards uncommitted work in that file. Its own
#     step comment says not to run it by hand in a dirty tree, and a gate a developer runs *while
#     working* is exactly a dirty tree.
#
#   * **Every matrix entry in `v0-exit-criteria`, `v01-gates`, `v1s1-gates`, `v1s7-table-gate`,
#     and the `v0-office-mutation` job.** These re-run subsets of `cargo test --workspace` under a
#     criterion's own name, so a reviewer can see *which line* is green rather than one
#     undifferentiated tick (`docs/history/05-MILESTONES.md` M7). Step 6 runs the superset, so running
#     them again buys a label and not a check. **Two are not subsets and are therefore steps 1 and
#     2**: `v0-no-confidence` and `v0-no-verify` are greps over source text that no test executes,
#     and since 2026-09-17 they are the `check` job's first two steps as well.
#
#   * **`cross-os-digests` and `cross-os-identity`.** They compare the engine's artifact digests
#     across Linux, macOS and Windows. One host runs one leg, and one leg compares nothing.
#
# # Order
#
# Every step here is one of the `check` job's. The two greps come first there and here for the same
# reason: they finish in seconds, and a forbidden token should fail the gate before a six-minute
# compile rather than after one.

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# CI sets this per-step rather than at workflow level, and says why: at workflow level it would
# also apply to the oracle build, holding another repository's source to a lint gate it does not
# run on itself. This script never builds the oracle, so that hazard does not exist here and one
# export is equivalent. It reaches the three steps CI sets it on — clippy, build and test — and
# the two it does not are `fmt` and `deny`, neither of which compiles anything.
export RUSTFLAGS="-D warnings"

# Point the oracle tests at a verifier, the way CI's `check` job does with its own checkout.
#
# Three rules, in this order. An operator's own `ETHOS_BIN` wins and is never second-guessed —
# `VerifierBinary::resolve` treats an explicit pin as authoritative and so does this. Otherwise
# the sibling `ethos-oracle` checkout is used, which is where CI puts `ETHOS_ORACLE_REF` and is
# the *pinned* ref rather than whatever `main` happens to be; a plain `../ethos` clone is
# deliberately NOT consulted, because docs/07-VERIFY-BOUNDARY.md §4 requires a verifier swap to
# be visible and "whatever main was that afternoon" is exactly the invisible swap. Absent both,
# nothing is exported and the oracle tests fail by name.
if [ -z "${ETHOS_BIN:-}" ] && [ -x ../ethos-oracle/target/release/ethos ]; then
  ETHOS_BIN="$(cd ../ethos-oracle/target/release && pwd)/ethos"
  export ETHOS_BIN
  printf 'gate: oracle at %s (%s, %s)\n' "$ETHOS_BIN" "$("$ETHOS_BIN" --version 2>/dev/null || echo '?')" \
    "$(git -C ../ethos-oracle rev-parse --short=12 HEAD 2>/dev/null || echo 'commit ?')"
fi

step=0
total=9
announce() {
  step=$((step + 1))
  printf '\n\033[1m[%d/%d] %s\033[0m\n' "$step" "$total" "$1"
}

announce 'v0-no-confidence — grep -ri confidence over sources returns nothing'
ci/forbidden-tokens.sh confidence

announce 'v0-no-verify — no verification code, type, or field exists in the tree'
ci/forbidden-tokens.sh verification

announce 'doc-version — every stated version matches the workspace'
ci/doc-version.sh

announce 'fmt'
cargo fmt --all --check

announce 'clippy'
cargo clippy --workspace --all-targets --locked -- -D warnings

announce 'build'
cargo build --workspace --locked

announce 'test'
cargo test --workspace --locked

announce 'deny'
cargo deny check

announce 'sdk suites — node and python, the version guards that had never run'
ci/sdk-suites.sh

printf '\n\033[1;32mgate: all %d checks passed.\033[0m\n' "$total"
printf 'Not run, by design: the oracle build, the toolchain tripwire, v0-fuzz-smoke,\n'
printf 'deny-policy-is-enforced, and the labelled re-runs of the suite above.\n'
printf 'The SDK suites stay hand-run (v1.2): packages/node and packages/python.\n'
