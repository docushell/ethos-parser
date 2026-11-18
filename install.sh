#!/bin/sh
# Install ethos-parser from a GitHub release on macOS or Linux:
#
#   curl -fsSL https://raw.githubusercontent.com/docushell/ethos-parser/main/install.sh | sh
#
# It picks this machine's archive from the release, checks it against the release's
# SHA256SUMS.txt, checks the binary starts here, and only then copies the binaries the archive
# holds (ethos-parser, and from 0.66.0 ethos-parser-ocr and ethos-parser-assist) into one
# directory. The digest check catches a corrupted download, not a compromised release: both files
# come from the same place.
#
# Optional settings, as environment variables:
#   ETHOS_PARSER_VERSION      a release to install, such as 0.65.0 (default: the latest)
#   ETHOS_PARSER_INSTALL_DIR  where the binaries go (default: $HOME/.local/bin)
#   ETHOS_PARSER_RELEASE_URL  the directory holding the release's files (wins over the version)
#
# Archive names follow ci/release-artifacts.sh: ethos-parser-<version>-<target>.tar.gz, holding one
# directory of the same name. Windows is not handled here; see docs/INSTALL.md.
#
# Everything runs inside main, called on the last line, so a download cut short runs nothing.
set -eu

fail() {
  printf 'install.sh: %s\n' "$1" >&2
  exit 1
}

main() {
  case "$(uname -s) $(uname -m)" in
    "Darwin arm64") target=aarch64-apple-darwin ;;
    "Darwin x86_64")
      # A shell under Rosetta reports x86_64 on Apple silicon; the native build is the one to take.
      if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then
        target=aarch64-apple-darwin
      else
        target=x86_64-apple-darwin
      fi
      ;;
    "Linux x86_64") target=x86_64-unknown-linux-gnu ;;
    *) fail "no prebuilt binary for $(uname -s) $(uname -m); build from source: https://github.com/docushell/ethos-parser/blob/main/docs/INSTALL.md" ;;
  esac

  command -v curl >/dev/null 2>&1 || fail "needs curl"
  if command -v sha256sum >/dev/null 2>&1; then
    digest() { sha256sum "$1" | cut -d ' ' -f 1; }
  elif command -v shasum >/dev/null 2>&1; then
    digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
  else
    fail "needs sha256sum or shasum to check the download"
  fi

  releases=https://github.com/docushell/ethos-parser/releases
  if [ -n "${ETHOS_PARSER_RELEASE_URL:-}" ]; then
    url=$ETHOS_PARSER_RELEASE_URL
  elif [ -n "${ETHOS_PARSER_VERSION:-}" ]; then
    url=$releases/download/v${ETHOS_PARSER_VERSION#v}
  else
    url=$releases/latest/download
  fi
  dir=${ETHOS_PARSER_INSTALL_DIR:-${HOME:?set HOME or ETHOS_PARSER_INSTALL_DIR}/.local/bin}
  dir=${dir%/}

  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"; rm -f "$dir"/.ethos-parser*."$$"' EXIT
  trap 'exit 1' HUP INT TERM

  curl -fsSL "$url/SHA256SUMS.txt" -o "$tmp/SHA256SUMS.txt" || fail "could not fetch $url/SHA256SUMS.txt"

  # The one line naming this target's archive: `<64 hex digits>  ./ethos-parser-<version>-<target>.tar.gz`.
  line=$(awk -v tail="-$target.tar.gz" '
    length($1) == 64 && $1 ~ /^[0-9a-f]+$/ && $2 ~ /^(\.\/)?ethos-parser-[^\/]+$/ &&
    substr($2, length($2) - length(tail) + 1) == tail { print $1, $2 }
  ' "$tmp/SHA256SUMS.txt")
  [ -n "$line" ] || fail "this release has no archive for $target"
  [ "$(printf '%s\n' "$line" | wc -l | tr -d ' ')" = 1 ] || fail "SHA256SUMS.txt names more than one archive for $target"
  expected=${line%% *}
  archive=${line#* }
  archive=${archive#./}

  curl -fsSL "$url/$archive" -o "$tmp/$archive" || fail "could not fetch $url/$archive"
  [ "$(digest "$tmp/$archive")" = "$expected" ] || fail "$archive does not match SHA256SUMS.txt; nothing was installed"

  tar -xzf "$tmp/$archive" -C "$tmp"
  from=$tmp/${archive%.tar.gz}
  [ -f "$from/ethos-parser" ] || fail "$archive holds no ethos-parser binary"
  # A binary this machine cannot start (an older glibc, say) must not replace a working one.
  "$from/ethos-parser" --version >/dev/null 2>&1 || fail "the ethos-parser in $archive does not start on this machine; nothing was installed"

  mkdir -p "$dir"
  # Copy every binary beside its target first, so a copy that fails (a full disk) replaces none.
  for name in ethos-parser ethos-parser-ocr ethos-parser-assist; do
    [ -f "$from/$name" ] || continue
    cp "$from/$name" "$dir/.$name.$$"
    chmod 755 "$dir/.$name.$$"
  done
  installed=
  for name in ethos-parser ethos-parser-ocr ethos-parser-assist; do
    if [ ! -f "$from/$name" ]; then
      # An older release has no such binary: a newer one left beside it would not match the engine.
      rm -f "$dir/$name"
      continue
    fi
    # Then rename each over its target, so a running copy is never half written.
    mv -f "$dir/.$name.$$" "$dir/$name"
    installed="$installed $name"
  done

  printf 'Installed%s into %s\n' "$installed" "$dir"
  "$dir/ethos-parser" --version
  case ":$PATH:" in
    *":$dir:"*) ;;
    *) printf '%s is not on your PATH. Add it, for example:\n  export PATH="%s:$PATH"\n' "$dir" "$dir" ;;
  esac
}

main "$@"
