# Installing ethos-parser

The short version is in the root [`README.md`](../README.md#try-it-in-30-seconds). This page has the details:
what each archive holds, each platform's requirements, Windows, macOS quarantine, and building
from source.

## What a release carries

Every GitHub Release from 0.61.0 on — 0.61.0, 0.64.0, 0.65.0 and 0.66.0 so far; 0.62.0 and 0.63.0
are tags with no Release — has a prebuilt binary for Linux, Windows and macOS (Apple silicon and Intel), plus
a `SHA256SUMS.txt`; the [CHANGELOG](../CHANGELOG.md) says what earlier releases carry.
A release is assembled only when all four binaries print byte-identical artifacts over the gate
documents: eight public PDFs kept in [`fixtures/gate/`](../fixtures/gate/).

From 0.66.0, each archive also carries two more binaries beside `ethos-parser`:

- `ethos-parser-ocr`, the recognizer `--ocr` runs;
- `ethos-parser-assist`, the assist lane's own process.

`ethos-parser-ocr` has its recognition networks built in (PP-OCRv5 and an orientation
classifier; see `NOTICE`). Its recognition model is PP-OCRv5's English one, so it reads English
only. No assist model ships: the assist lane asks a model server you run.

## Platform requirements

- **Linux:** glibc 2.39 or newer (Ubuntu 24.04 and later) up to 0.65.0. From 0.66.0, glibc 2.35 or
  newer (Ubuntu 22.04, Debian 12 and later).
- **x86-64 CPUs:** without AVX2 and FMA, the recognizer refuses by name.
- **Windows:** the binary needs the Microsoft Visual C++ runtime. The assist binary's `ask` and
  `check` are refused `curl-not-run`; `render` and `identity` work.

## macOS and Linux, with the install script

```bash
curl -fsSL https://raw.githubusercontent.com/docushell/ethos-parser/main/install.sh | sh
```

[`install.sh`](../install.sh) picks the archive for this machine (Apple silicon or Intel macOS, or
x86-64 Linux) from the latest release. It checks the archive against the release's
`SHA256SUMS.txt`, checks that its `ethos-parser` starts on this machine, and copies the binaries the
archive holds into `~/.local/bin`. Nothing is installed if either check fails. Installing an older
release removes a newer recognizer or assist binary it does not carry, so none is left beside an
engine it does not match. That check catches a corrupted download, not a compromised
release, because both files come from the same place. Read the script before you run it if that
matters to you; it is short.

Three environment variables change what it does:

| Variable | Default | Effect |
| --- | --- | --- |
| `ETHOS_PARSER_VERSION` | the latest release | Install a given release from 0.61.0 on, such as `0.65.0`; 0.62.0 and 0.63.0 have no Release to install |
| `ETHOS_PARSER_INSTALL_DIR` | `~/.local/bin` | Where the binaries go |
| `ETHOS_PARSER_RELEASE_URL` | the release on GitHub | A directory holding a release's files, such as a mirror |

For example:

```bash
curl -fsSL https://raw.githubusercontent.com/docushell/ethos-parser/main/install.sh | ETHOS_PARSER_VERSION=0.65.0 sh
```

## macOS and Linux, by hand

Set `T` to your platform and run:

```bash
T=aarch64-apple-darwin   # or x86_64-apple-darwin, x86_64-unknown-linux-gnu
curl -fLO https://github.com/docushell/ethos-parser/releases/latest/download/SHA256SUMS.txt
A=$(grep -o "ethos-parser-[^ ]*-$T\.tar\.gz" SHA256SUMS.txt)
curl -fLO "https://github.com/docushell/ethos-parser/releases/latest/download/$A"
shasum -a 256 -c SHA256SUMS.txt --ignore-missing   # Linux: sha256sum -c SHA256SUMS.txt --ignore-missing
tar -xzf "$A"
"./${A%.tar.gz}/ethos-parser" --version
```

`OK` beside your archive means the digest check passed. The one line it calls improperly formatted
is the blank line in the file's header.

The macOS binaries are not notarized. `curl` leaves them runnable, but a copy downloaded through a
browser is quarantined and killed on launch. Once its digest checks out, this clears it:

```bash
xattr -d com.apple.quarantine ethos-parser-*/ethos-parser*
```

## Windows

Take the `x86_64-pc-windows-msvc` archive from the
[releases page](https://github.com/docushell/ethos-parser/releases). Compare `Get-FileHash` with
`SHA256SUMS.txt`, extract it with `tar -xzf`, and run `ethos-parser.exe`.

## Keep the binaries together

Keep `ethos-parser` and `ethos-parser-ocr` in one directory. `--ocr` looks beside the path the
engine was started from. A copy of the engine on its own finds no recognizer, and on macOS neither
does a symlink to it, unless `ETHOS_PARSER_OCR` names it.

## Build from source

With the toolchain `rust-toolchain.toml` pins:

```bash
cargo build --release --locked
```

The recognizer builds in `ocr/`, with the toolchain `ocr/rust-toolchain.toml` pins:

```bash
cd ocr && cargo build --release --locked
```

Put `ocr/target/release/ethos-parser-ocr` beside `ethos-parser`, or name it with
`ETHOS_PARSER_OCR`.

The assist binary builds the same way, in `assist/`:

```bash
cd assist && cargo build --release --locked
```

## The assist lane

The assist lane is opt-in. A model you run reads the pages you choose, and its text reaches Markdown
and HTML labelled `Proposed`, never as evidence. `ethos-parser-assist` renders the pages and asks
your model server through the system `curl`; `extract --assist-answers` attaches the replies.

- Setup, commands and limits: [`assist/README.md`](../assist/README.md).
- The network exception: [`37-ASSIST-TRANSPORT.md`](37-ASSIST-TRANSPORT.md).

## First run

With the binary on your `PATH`:

```bash
ethos-parser extract contract.pdf > repr.json
ethos-parser ground repr.json > grounding.json
```
