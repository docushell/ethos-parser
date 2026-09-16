#!/usr/bin/env bash
# Build, VERIFY and package the release binaries.
#
#   ci/release-artifacts.sh                          # every target this host can build
#   ci/release-artifacts.sh --target T               # just one
#   ci/release-artifacts.sh --native                 # only the host's own target: what each CI runner runs
#   ci/release-artifacts.sh --tag v0.58.0            # and refuse unless Cargo.toml says 0.58.0
#   ci/release-artifacts.sh --fingerprint-only BIN   # print BIN's fingerprint over the gate corpus; build nothing
#   ci/release-artifacts.sh --assemble DIR           # CI's verify job: one set of bytes from every runner's
#                                                    # output under DIR/*/, or no SHA256SUMS.txt at all
#   ci/release-artifacts.sh --allow-dirty            # for trying it out; never for a release
#
# # Why this is a script, and what the workflow does with it
#
# 6.3 asks for GitHub Releases on three platforms. When this was written, GitHub Actions would not
# allocate a runner on this account, so a `release.yml` would have been a file that had never
# executed — and the plan's own note about 6.1 applies: a job that never ran proves nothing about
# the artifact it claims to produce. So the machinery is a script that runs, here, now. Runners
# run since the repository went public, and `.github/workflows/release-artifacts.yml` calls this
# script unchanged: every runner with `--native`, so it builds and EXECUTES only the target it is,
# and the `verify` job with `--assemble` over what the runners uploaded. The workflow publishes
# nothing; the owner attaches its output by hand (docs/RELEASING.md §8).
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
# Across runners the comparison is stricter, because there is no native build to defer to: under
# `--assemble`, a target is `verified` only when its fingerprint is byte-identical to EVERY other
# runner's, and one difference fails the assembly with nothing written. Identical is not enough
# either — every fingerprint must also show the whole corpus PRODUCED (`require_produced`), or a
# corpus every runner refused would compare clean.
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
NATIVE_ONLY=0
TAG=""
FINGERPRINT_ONLY=""
ASSEMBLE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --allow-dirty) ALLOW_DIRTY=1; shift ;;
    --target) ONLY_TARGET="${2:-}"; shift 2 ;;
    --native) NATIVE_ONLY=1; shift ;;
    --tag) TAG="${2:-}"; shift 2 ;;
    --fingerprint-only) FINGERPRINT_ONLY="${2:-}"; shift 2 ;;
    --assemble) ASSEMBLE="${2:-}"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

fail() { printf '\n\033[1;31mrelease-artifacts: %s\033[0m\n' "$1" >&2; shift; for l in "$@"; do printf '%s\n' "$l" | sed 's/^/  /' >&2; done; exit 1; }
ok()   { printf '\033[1;32m  ok\033[0m       %s\n' "$1"; }
note() { printf '\033[1;33m  note\033[0m     %s\n' "$1"; }

version="$(grep -m1 '^version = ' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')"
[ -n "$version" ] || fail "could not read the workspace version from Cargo.toml"

# A tag names a version. `--tag` is CI's assertion that the ref it was handed is the one Cargo.toml
# describes, so a workflow started on the wrong tag stops before it builds anything.
if [ -n "$TAG" ] && [ "$TAG" != "v$version" ]; then
  fail "tag $TAG does not name version $version." \
       "Cargo.toml says $version, so the tag for this tree is v$version. A tag that says otherwise" \
       "names a different tree, and its artifacts would carry the wrong parser_version."
fi

# sha256 of stdin, as hex. `shasum -a 256` is what macOS has and what this script has always used;
# Linux and Git Bash on Windows have coreutils' `sha256sum`. Both print `hex  name`, so the first
# field is the digest on every host.
case "$(uname -s)" in
  Darwin) sha256() { shasum -a 256 | cut -d' ' -f1; } ;;
  *)      sha256() { sha256sum | cut -d' ' -f1; } ;;
esac

# sha256 of zero bytes: what a refused subcommand's stdout digests to.
EMPTY=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

# The corpus a verified binary must reproduce. Small and committed, so this stays cheap: the gate
# corpus is eight documents this repository owns. `LC_ALL=C` so the rows come out in the same
# order on every host — `ls` sorts its arguments by the locale's collation, and a runner's locale
# is not this machine's.
corpus() { LC_ALL=C ls fixtures/gate/*.pdf 2>/dev/null; }

# Whether a document takes the projection half of the fingerprint. The two largest gate documents
# do not — 53Ar5's representation is 950 MB — and they are still compared through `extract` and
# `classify`, which is where a codegen difference would show up first anyway.
projected() {
  case "$(basename "$1" .pdf)" in
    nist-sp-800-53Ar5|nist-sp-800-161r1) return 1 ;;
    *) return 0 ;;
  esac
}

# Digests of every canonical-byte subcommand over the corpus, as one stream.
#
# `extract` and `classify` take a PDF. `markdown`, `html` and `ground` take a
# DocumentRepresentation JSON — handing them a PDF makes them refuse, which compares a refusal
# message and NOT a projection. An earlier draft of this function did exactly that and would have
# marked a binary 'verified' on the strength of comparisons that never projected anything.
#
# So the representation is produced ONCE, by whichever binary is being fingerprinted, and the
# projections run against it. A subcommand that refuses still gets its row — the digest of the
# nothing it wrote — so `require_produced` can name it, rather than the refusal ending this
# function under `pipefail` with no row and no message.
fingerprint() {
  local bin="$1" doc sub repr
  repr="$(mktemp -d)"
  for doc in $(corpus); do
    for sub in extract classify; do
      printf '%s %s ' "$(basename "$doc")" "$sub"
      "$bin" "$sub" "$doc" 2>/dev/null | sha256 || true
    done
    projected "$doc" || continue
    "$bin" extract "$doc" > "$repr/r.json" 2>/dev/null || continue
    for sub in markdown html ground; do
      printf '%s %s ' "$(basename "$doc")" "$sub"
      "$bin" "$sub" "$repr/r.json" 2>/dev/null | sha256 || true
    done
  done
  rm -rf "$repr"
}

# **Identical is not enough.** Two binaries that refuse every document agree perfectly, and so do
# two whose corpus checkout was empty, so a fingerprint must also show the corpus PRODUCED: every
# row present and every row a non-empty artifact. The count is derived from the corpus on disk,
# never written here — every gate document must extract and classify, and every projected one
# must project, so a gate document a release binary refuses fails the release, not the label.
require_produced() {
  local fp="$1" what="$2" doc docs=0 skipped=0 expected produced refused
  for doc in $(corpus); do
    docs=$((docs + 1))
    projected "$doc" || skipped=$((skipped + 1))
  done
  [ "$docs" -gt 0 ] || fail "fixtures/gate holds no PDF; nothing could have been fingerprinted."
  expected=$(( 2 * docs + 3 * (docs - skipped) ))
  produced="$(printf '%s\n' "$fp" | awk -v empty="$EMPTY" 'NF == 3 && $3 != empty { n++ } END { print n + 0 }')"
  if [ "$produced" -ne "$expected" ]; then
    refused="$(printf '%s\n' "$fp" | awk -v empty="$EMPTY" 'NF == 3 && $3 == empty { print "  refused: " $1 " " $2 }')"
    fail "$what produced $produced non-empty artifact(s) over the gate corpus; all $expected are required." \
         "Every gate document must extract and classify, and every projected one must project." \
         "A row digesting to the empty digest is a refusal; a missing row is a document whose" \
         "extract failed. Find out which and why. Do not release." "$refused"
  fi
}

# `hash  ./name` for every tarball in the current directory: the line `shasum -a 256 ./*.tar.gz`
# has always printed here, produced the same way on every host.
sums() {
  local f
  for f in ./*.tar.gz; do printf '%s  %s\n' "$(sha256 < "$f")" "$f"; done
}

# --- --fingerprint-only: one binary's fingerprint, and nothing built or removed ------------------
#
# For comparing a binary this host built against what the workflow's runners produced: their
# `.fingerprint` files ride in the release bundle, and `diff` settles it. The path is relative to
# the repository root, which is the working directory here.
if [ -n "$FINGERPRINT_ONLY" ]; then
  [ -x "$FINGERPRINT_ONLY" ] || fail "no executable at $FINGERPRINT_ONLY"
  fp="$(fingerprint "$FINGERPRINT_ONLY")"
  require_produced "$fp" "$FINGERPRINT_ONLY"
  printf '%s\n' "$fp"
  exit 0
fi

# --- --assemble: CI's verify job ----------------------------------------------------------------
#
# Each DIR/*/ is one runner's target/release-artifacts: a tarball, the fingerprint of the binary
# inside it, and that runner's SHA256SUMS.txt. Every fingerprint must be whole and byte-identical
# to every other, and every tarball must still digest to what its runner recorded; only then is
# DIR/SHA256SUMS.txt written, labelling every target `verified`, with the tarballs and fingerprints
# copied flat beside it. Anything short of that writes nothing, so a bundle without a
# SHA256SUMS.txt is a bundle that failed.
assemble() {
  local dir="$1" run name target recorded digest fp i differing
  local -a runs fps tarballs targets fp_files tarball_files digests
  [ -d "$dir" ] || fail "$dir is not a directory"
  dir="$(cd "$dir" && pwd)"
  # A stale manifest from an earlier attempt must not outlive a failed assembly.
  rm -f "$dir/SHA256SUMS.txt"

  shopt -s nullglob
  runs=("$dir"/*/)
  shopt -u nullglob
  [ ${#runs[@]} -ge 2 ] || fail "found ${#runs[@]} runner output(s) under $dir; at least two are needed." \
       "One runner has nothing to be compared against, and 'verified' here means compared."

  for run in "${runs[@]}"; do
    run="${run%/}"
    shopt -s nullglob
    fps=("$run"/*.fingerprint)
    tarballs=("$run"/*.tar.gz)
    shopt -u nullglob
    [ ${#fps[@]} -eq 1 ] || fail "$run holds ${#fps[@]} fingerprint file(s); one runner's output holds exactly one." \
         "Assemble the runners' outputs only — not an earlier bundle, which holds all of them."
    [ ${#tarballs[@]} -eq 1 ] || fail "$run holds ${#tarballs[@]} tarball(s); one runner's output holds exactly one."
    [ -f "$run/SHA256SUMS.txt" ] || fail "$run has no SHA256SUMS.txt"

    name="$(basename "${fps[0]}" .fingerprint)"
    case "$name" in
      "ethos-parser-$version-"*) target="${name#ethos-parser-$version-}" ;;
      *) fail "${fps[0]} was not built at $version, which this tree is." \
              "A runner built a different commit. Assemble outputs of one ref only." ;;
    esac
    [ "$(basename "${tarballs[0]}")" = "$name.tar.gz" ] || \
      fail "$run: the tarball and the fingerprint name different targets"
    for i in ${targets[@]+"${targets[@]}"}; do
      [ "$i" = "$target" ] && fail "two runner outputs claim $target"
    done

    fp="$(cat "${fps[0]}")"
    require_produced "$fp" "$target"

    # The runner digested the tarball it built; digest what arrived. Equal, or the download is not
    # the build and nothing about it is known.
    recorded="$(awk -v want="./$name.tar.gz" 'length($1) == 64 && $2 == want { print $1 }' "$run/SHA256SUMS.txt")"
    [ -n "$recorded" ] || fail "$run/SHA256SUMS.txt has no digest line for $name.tar.gz"
    digest="$(sha256 < "${tarballs[0]}")"
    [ "$digest" = "$recorded" ] || \
      fail "$name.tar.gz digests to $digest here and to $recorded on its runner; the download is not the build."

    targets+=("$target"); fp_files+=("${fps[0]}"); tarball_files+=("${tarballs[0]}"); digests+=("$digest")
    ok "$target: whole fingerprint, tarball digest as recorded"
  done

  for i in "${!fp_files[@]}"; do
    [ "$i" -eq 0 ] && continue
    if ! cmp -s "${fp_files[0]}" "${fp_files[$i]}"; then
      differing="$(diff "${fp_files[0]}" "${fp_files[$i]}" | awk '/^[<>]/ { print "  " $2 " " $3 }' | sort -u || true)"
      fail "${targets[0]} and ${targets[$i]} produced DIFFERENT artifacts." \
           "This is the determinism claim failing across operating systems, not a packaging problem." \
           "Do not release. The rows that differ (document, subcommand):" "$differing" \
           "  diff ${fp_files[0]} ${fp_files[$i]}"
    fi
  done
  ok "one fingerprint on all ${#targets[@]} runners: ${targets[*]}"

  for i in "${!targets[@]}"; do
    cp -f "${tarball_files[$i]}" "$dir/"
    cp -f "${fp_files[$i]}" "$dir/"
  done

  {
    echo "# ethos-parser $version — release artifacts, assembled from ${#targets[@]} runners"
    echo "#"
    echo "# state 'verified' means the binary was EXECUTED on the runner that built it and every artifact"
    echo "# digest over the gate corpus equalled every other runner's: one fingerprint on every system"
    echo "# below. That is plan item 6.1's operating-system axis, measured on these binaries. Nothing"
    echo "# here is 'compiled' — a fingerprint that differed would have failed the assembly, and this"
    echo "# file would not exist."
    echo "#"
    for i in "${!targets[@]}"; do
      echo "# ${targets[$i]}  ->  verified"
    done
    echo "#"
    echo "# commit $(git rev-parse HEAD 2>/dev/null || echo unknown)"
    if [ -n "${GITHUB_RUN_ID:-}" ]; then
      echo "# run    ${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-}/actions/runs/$GITHUB_RUN_ID"
    fi
    echo
    for i in "${!targets[@]}"; do
      printf '%s  %s\n' "${digests[$i]}" "./ethos-parser-$version-${targets[$i]}.tar.gz"
    done
  } > "$dir/SHA256SUMS.txt"

  echo
  echo "SHA256SUMS.txt"
  sed 's/^/  /' "$dir/SHA256SUMS.txt"
  echo
  echo "release-artifacts: ${#targets[@]} artifact(s) assembled, every one verified across ${#targets[@]} runners."
}

if [ -n "$ASSEMBLE" ]; then
  echo "release-artifacts: ethos-parser $version — assembling $ASSEMBLE"
  echo
  assemble "$ASSEMBLE"
  exit 0
fi

# --- build, execute, package ---------------------------------------------------------------------

if [ "$ALLOW_DIRTY" -eq 0 ] && [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  fail "the working tree has uncommitted changes." \
       "The thing packaged would not be the thing the gate tested." \
       "Commit first, or pass --allow-dirty to try the script out."
fi

host_os="$(uname -s)"
host_arch="$(uname -m)"
# The host's own target, from the compiler rather than from `uname`: `uname -m` says arm64 on
# macOS and aarch64 on Linux for one architecture, and says nothing about the OS or the libc.
NATIVE="$(rustc -vV | sed -n 's/^host: //p')"
[ -n "$NATIVE" ] || fail "could not read the host target from rustc -vV"

# Can this host EXECUTE a binary for the given target? Conservative by design: a target is
# runnable only where that is a fact about this machine, never an assumption.
#   - the native target always runs
#   - an arm64 Mac runs x86_64 Mach-O through Rosetta 2, but only when Rosetta is installed
runnable() {
  case "$1" in
    "$NATIVE") return 0 ;;
    x86_64-apple-darwin)
      [ "$host_os" = Darwin ] && [ "$host_arch" = arm64 ] && /usr/bin/pgrep -q oahd && return 0
      return 1 ;;
    *) return 1 ;;
  esac
}

# Targets worth attempting. A target rustup cannot install, or whose link step fails for want of a
# cross linker, is REPORTED and skipped — never silently dropped. `--native` is what a CI runner
# passes: it is one machine, it builds the target it is, and a cross build there would be the
# `compiled` state this script exists to keep out of a release.
TARGETS=("$NATIVE" x86_64-apple-darwin x86_64-unknown-linux-musl x86_64-pc-windows-gnu)
[ "$NATIVE_ONLY" -eq 1 ] && TARGETS=("$NATIVE")
[ -n "$ONLY_TARGET" ] && TARGETS=("$ONLY_TARGET")

echo "release-artifacts: ethos-parser $version"
echo "  host:    $host_arch ($NATIVE)"
echo "  out:     $OUT"
echo

rm -rf "$OUT"
mkdir -p "$OUT"

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

  # `--locked`, as every cargo invocation in ci.yml: the lockfile the gate tested is the one built.
  # The compiler's output goes to a log — noise on success locally, the only record of what was
  # built on a runner, and the diagnosis on failure either way.
  log="$OUT/build-$target.log"
  if ! cargo build --release --locked --target "$target" -p ethos-parser-cli >"$log" 2>&1; then
    if [ "$target" = "$NATIVE" ]; then
      tail -n 40 "$log" >&2
      fail "the native build failed; the full log is $log" \
           "A native build has no cross linker to be missing. This is a code or toolchain problem."
    fi
    note "build failed — almost always a missing cross LINKER, not a code problem. Log: $log"
    note "  linux-musl needs musl-gcc or cargo-zigbuild; windows-gnu needs mingw-w64."
    note "  Reported rather than skipped silently: a release must not quietly omit a platform."
    echo; continue
  fi
  [ -z "${CI:-}" ] || cat "$log"

  bin="target/$target/release/ethos-parser"
  [ -x "$bin" ] || bin="$bin.exe"
  [ -x "$bin" ] || { note "no binary produced at target/$target/release — skipped"; echo; continue; }
  ok "built  $(wc -c < "$bin" | tr -d ' ') bytes"

  state="compiled"
  if runnable "$target"; then
    fp="$(fingerprint "$bin")"
    require_produced "$fp" "$target"
    # Kept beside the tarball: it is what `--assemble` compares across runners, and what a locally
    # built binary is compared against with `--fingerprint-only`.
    printf '%s\n' "$fp" > "$OUT/ethos-parser-$version-$target.fingerprint"
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
  (cd "$OUT" && sums)
} > "$OUT/SHA256SUMS.txt"

echo "SHA256SUMS.txt"
sed 's/^/  /' "$OUT/SHA256SUMS.txt"
echo
verified=0
for s in "${states[@]}"; do [ "$s" = "verified" ] && verified=$((verified+1)); done
echo "release-artifacts: ${#built[@]} artifact(s), $verified verified, $(( ${#built[@]} - verified )) compiled-only."
[ "$verified" -eq "${#built[@]}" ] || echo "A release note must not imply the compiled-only artifacts were tested."
