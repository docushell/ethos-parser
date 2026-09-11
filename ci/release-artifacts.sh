#!/usr/bin/env bash
# Build, VERIFY and package the release binaries.
#
#   ci/release-artifacts.sh                 # every target this host can build
#   ci/release-artifacts.sh --target T      # just one
#   ci/release-artifacts.sh --allow-dirty   # for trying it out; never for a release
#
# # Why this exists, and why it is a script rather than a workflow
#
# 6.3 asks for GitHub Releases on three platforms. The obvious shape is a release workflow, and
# this repository cannot have one that means anything today: GitHub Actions will not allocate a
# runner on this account, so a `release.yml` would be a file that has never executed. The plan's
# own note about 6.1 applies here — a job that never ran proves nothing about the artifact it
# claims to produce. So the machinery is a script that runs, here, now, and a workflow can call it
# unchanged when runners return.
#
# # The rule this script exists to enforce
#
# **A packaged binary must have been EXECUTED and its artifacts compared.** For a parser whose
# product is byte-identical reruns, shipping a binary that was only compiled is shipping an
# untested claim. Cross-compilation makes that trivially easy to do by accident: `cargo build
# --target x86_64-pc-windows-gnu` on a Mac produces a plausible .exe that nobody has ever run.
#
# So every target is one of two states, and the manifest says which:
#
#   verified   built, executed on this host, and every artifact digest equal to the native build's
#   compiled   built only — this host cannot execute it
#
# `compiled` artifacts are still packaged, because a release that omits Linux helps nobody, but
# they are labelled in `SHA256SUMS.txt` and in the manifest so a release note cannot silently
# imply they were tested. Promoting one to `verified` needs a machine of that platform, which is
# what plan item 6.1 is for.
#
# # What this does NOT check
#
# The gate. `ci/release-preflight.sh` invokes `ci/gate.sh` and this script refuses to run unless
# the tree is clean, so the thing packaged is the thing the gate saw. Re-implementing a gate check
# here would be the defect this repository keeps finding — a second copy that goes stale.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

OUT="$ROOT/target/release-artifacts"
ALLOW_DIRTY=0
ONLY_TARGET=""

while [ $# -gt 0 ]; do
  case "$1" in
    --allow-dirty) ALLOW_DIRTY=1; shift ;;
    --target) ONLY_TARGET="${2:-}"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

fail() { printf '\n\033[1;31mrelease-artifacts: %s\033[0m\n' "$1" >&2; shift; for l in "$@"; do echo "  $l" >&2; done; exit 1; }
ok()   { printf '\033[1;32m  ok\033[0m       %s\n' "$1"; }
note() { printf '\033[1;33m  note\033[0m     %s\n' "$1"; }

version="$(grep -m1 '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
[ -n "$version" ] || fail "could not read the workspace version from Cargo.toml"

if [ "$ALLOW_DIRTY" -eq 0 ] && [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  fail "the working tree has uncommitted changes." \
       "The thing packaged would not be the thing the gate tested." \
       "Commit first, or pass --allow-dirty to try the script out."
fi

host_arch="$(uname -m)"
case "$host_arch" in
  arm64|aarch64) NATIVE="aarch64-apple-darwin" ;;
  x86_64)        NATIVE="x86_64-apple-darwin" ;;
  *)             fail "unrecognised host architecture: $host_arch" ;;
esac

# Can this host EXECUTE a binary for the given target? Conservative by design: a target is
# runnable only where that is a fact about this machine, never an assumption.
#   - the native target always runs
#   - an arm64 Mac runs x86_64 Mach-O through Rosetta 2, but only when Rosetta is installed
runnable() {
  case "$1" in
    "$NATIVE") return 0 ;;
    x86_64-apple-darwin)
      [ "$host_arch" = "arm64" ] && /usr/bin/pgrep -q oahd && return 0
      return 1 ;;
    *) return 1 ;;
  esac
}

# Targets worth attempting. A target rustup cannot install, or whose link step fails for want of a
# cross linker, is REPORTED and skipped — never silently dropped.
TARGETS=("$NATIVE" x86_64-apple-darwin x86_64-unknown-linux-musl x86_64-pc-windows-gnu)
[ -n "$ONLY_TARGET" ] && TARGETS=("$ONLY_TARGET")

echo "release-artifacts: ethos-parser $version"
echo "  host:    $host_arch ($NATIVE)"
echo "  out:     $OUT"
echo

rm -rf "$OUT"
mkdir -p "$OUT"

# The corpus a verified binary must reproduce. Small and committed, so this stays cheap: the gate
# corpus is eight documents this repository owns.
corpus() { ls fixtures/gate/*.pdf 2>/dev/null; }

# Digests of every canonical-byte subcommand over the corpus, as one stream.
#
# `extract` and `classify` take a PDF. `markdown`, `html` and `ground` take a
# DocumentRepresentation JSON — handing them a PDF makes them refuse, which compares a refusal
# message and NOT a projection. An earlier draft of this function did exactly that and would have
# marked a binary 'verified' on the strength of comparisons that never projected anything.
#
# So the representation is produced ONCE, by whichever binary is being fingerprinted, and the
# projections run against it. The two largest gate documents are skipped for the projection half
# only — 53Ar5's representation is 950 MB — and they are still compared through `extract` and
# `classify`, which is where a codegen difference would show up first anyway.
fingerprint() {
  local bin="$1" doc sub repr
  repr="$(mktemp -d)"
  for doc in $(corpus); do
    for sub in extract classify; do
      printf '%s %s ' "$(basename "$doc")" "$sub"
      "$bin" "$sub" "$doc" 2>/dev/null | shasum -a 256 | cut -d' ' -f1
    done
    case "$(basename "$doc" .pdf)" in
      nist-sp-800-53Ar5|nist-sp-800-161r1) continue ;;
    esac
    "$bin" extract "$doc" > "$repr/r.json" 2>/dev/null || continue
    for sub in markdown html ground; do
      printf '%s %s ' "$(basename "$doc")" "$sub"
      "$bin" "$sub" "$repr/r.json" 2>/dev/null | shasum -a 256 | cut -d' ' -f1
    done
  done
  rm -rf "$repr"
}

built=(); states=()
native_fp=""

for target in "${TARGETS[@]}"; do
  echo "[$target]"

  if ! rustup target list --installed 2>/dev/null | grep -qx "$target"; then
    if ! rustup target add "$target" >/dev/null 2>&1; then
      note "rustup cannot install this target here — skipped"
      echo; continue
    fi
  fi

  if ! cargo build --release --target "$target" -p ethos-parser-cli >/dev/null 2>&1; then
    note "build failed — almost always a missing cross LINKER, not a code problem."
    note "  linux-musl needs musl-gcc or cargo-zigbuild; windows-gnu needs mingw-w64."
    note "  Reported rather than skipped silently: a release must not quietly omit a platform."
    echo; continue
  fi

  bin="target/$target/release/ethos-parser"
  [ -x "$bin" ] || bin="$bin.exe"
  [ -x "$bin" ] || { note "no binary produced at target/$target/release — skipped"; echo; continue; }
  ok "built  $(wc -c < "$bin" | tr -d ' ') bytes"

  state="compiled"
  if runnable "$target"; then
    fp="$(fingerprint "$bin")"
    if [ "$target" = "$NATIVE" ]; then
      native_fp="$fp"
      state="verified"
      ok "executed on this host — this is the reference fingerprint"
    elif [ -n "$native_fp" ] && [ "$fp" = "$native_fp" ]; then
      state="verified"
      ok "executed on this host — every artifact digest equal to $NATIVE"
    elif [ -n "$native_fp" ]; then
      fail "$target produced DIFFERENT artifacts than $NATIVE." \
           "This is the determinism claim failing, not a packaging problem." \
           "Do not release. Diff the fingerprints and find out why."
    fi
  else
    note "this host cannot execute $target — packaged as 'compiled', not 'verified'"
  fi

  stage="$OUT/ethos-parser-$version-$target"
  mkdir -p "$stage"
  cp "$bin" "$stage/"
  cp LICENSE "$stage/" 2>/dev/null || true
  cp README.md "$stage/" 2>/dev/null || true
  tar -czf "$OUT/ethos-parser-$version-$target.tar.gz" -C "$OUT" "$(basename "$stage")"
  rm -rf "$stage"
  ok "packaged ethos-parser-$version-$target.tar.gz"

  built+=("$target"); states+=("$state")
  echo
done

[ ${#built[@]} -gt 0 ] || fail "nothing was built"

{
  echo "# ethos-parser $version — release artifacts"
  echo "#"
  echo "# state 'verified' means the binary was EXECUTED on the build host and every artifact"
  echo "# digest over the gate corpus equalled the native build's. State 'compiled' means it was"
  echo "# built and never run: this host could not execute it. A compiled artifact is not evidence"
  echo "# of anything about the platform it targets. See plan item 6.1."
  echo "#"
  for i in "${!built[@]}"; do
    echo "# ${built[$i]}  ->  ${states[$i]}"
  done
  echo
  cd "$OUT" && shasum -a 256 ./*.tar.gz
} > "$OUT/SHA256SUMS.txt"

echo "SHA256SUMS.txt"
sed 's/^/  /' "$OUT/SHA256SUMS.txt"
echo
verified=0
for s in "${states[@]}"; do [ "$s" = "verified" ] && verified=$((verified+1)); done
echo "release-artifacts: ${#built[@]} artifact(s), $verified verified, $(( ${#built[@]} - verified )) compiled-only."
[ "$verified" -eq "${#built[@]}" ] || echo "A release note must not imply the compiled-only artifacts were tested."
