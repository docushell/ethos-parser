# 37 — The assist lane's transport: the system `curl`, run by `ethos-parser-assist`

**Status: DRAFT, decision #144.** Written 2026-11-15. The owner chose the transport on 2026-11-15
(Phase 4b question Q1): "the system `curl`, run as a subprocess of `ethos-parser-assist`". This is
the ADR that [`deny.toml`](../deny.toml) requires for an exception to the dependency posture. As
[`20`](20-STANDARD-14-METRICS-SCOPE.md) explains, this repository records a decision as a numbered
document plus a numbered row in [`00-NORTH-STAR.md`](00-NORTH-STAR.md). It has no `docs/adr/`, so
this document is that ADR.

## 1. The exception, in one sentence

**`ethos-parser-assist`, and no other binary, may reach the network, only by running the system
`curl` as a subprocess. No HTTP, TLS or socket crate goes into any lockfile.**

It amends [`04-ARCHITECTURE.md`](04-ARCHITECTURE.md) §5, whose "Network crates: Forbidden" row
stands for every crate. The root `deny.toml` is unchanged. `assist/deny.toml` keeps the root's
network bans whole, `curl` and `curl-sys` included. `ethos-parser` and `ethos-parser-ocr` gain no
network surface: the engine only spawns the assist binary, as `--ocr` spawns the recognizer.

## 2. Why a subprocess, and why `curl`

The assist lane's model is the user's own: a server they run on this machine (mlx-vlm on a Mac,
vLLM on Linux) or a cloud API. Each page goes to it as one HTTP request. Something has to send
those requests.

- **`curl`, chosen.** It adds no crate and needs no C toolchain. It uses the OS trust store and the
  proxy variables, so a corporate inspection CA works as it does for every other tool. macOS ships
  it, and Linux distributions package it.
- **`ureq 3.4.2` with rustls, not chosen.** It brings about 15 crates: rustls, ring (which needs a
  C compiler), webpki and its roots (CDLA-Permissive-2.0, not cached here), and http. Its own roots
  do not trust an inspection CA.
- **The costs of `curl`.**
  - A run needs `curl` on `PATH`. Without it, every `ask` and `check` is refused by name,
    `curl-not-found`.
  - Each request starts a process. That costs milliseconds, against about 35 s a page from the
    model (Phase 4a).
  - Only long-standing options are used: `config`, `proto`, `globoff`, `noproxy`, `max-time`,
    `connect-timeout`, `max-filesize`, `write-out`, `header` and `data-binary`. The binary was
    tested with curl 8.7.1 on macOS.
  - **Unix only, for now.** The page reaches curl as `/dev/fd/2` (§3). A platform without
    `/dev/fd`, Windows among them, is refused `curl-not-run`, rather than given a file holding
    the page. Windows is a question for the release step (D8).

## 3. How the binary runs `curl`

Every request runs `curl -q -K -`, and that argv never changes.
- **`-q`** comes first, so a user's `.curlrc` is not read. A `.curlrc` could add `--trace`, which
  writes every header, the key's included, to a file.
- **`-K -`** reads the request from a config on curl's stdin. The config holds:
  - the URL, with `proto = "=http,https"`, so no other scheme is used, and `globoff`, so the URL
    is never a curl glob (`{a,b}` would be two requests). No redirect is followed, since
    `location` is not set;
  - `noproxy`, which always lists `localhost,127.0.0.1,::1`, then the user's own `no_proxy` or
    `NO_PROXY` (the option replaces them, so they are carried over; a lone `*` stays `*`);
  - `max-time = 1800` (Phase 4a's per-page limit) and `connect-timeout = 10`;
  - `max-filesize` at 64 MiB: a page's reply is tens of kilobytes. curl stops an announced
    length at it; an unannounced one only since curl 8.4. So the binary also stops reading past
    the cap and stops curl, whatever its version;
  - `write-out` with the status code;
  - the key's `Authorization: Bearer` header, when the key is set;
  - the body, as `data-binary = "@/dev/fd/2"`.
- **Every config value is quoted** with curl's escapes, so no value can end its line and add an
  option. A key holding a control character is refused, `key-not-a-header-value`, before curl runs.
- **The body goes in on a second pipe**, curl's fd 2, which the config reads as `/dev/fd/2`.
  curl reads it whole while it reads the config, before it connects.
  - No file holds the page, not even for an instant, so a run killed by any signal, `SIGKILL`
    included, leaves no page on disk.
  - A config value cannot carry the body: real pages reach 13 MB in base64, and curl takes config
    lines up to 10 MB (100 KB before 8.2).
  - `silent` writes nothing to curl's stderr, so the body pipe is the only use of fd 2, and
    curl's messages, which name the server's address, are never read.
- **curl's environment** is the binary's, minus `ETHOS_PARSER_ASSIST_KEY`.

**Proxies.** A loopback server is never reached through a proxy, whatever `http_proxy`,
`HTTPS_PROXY` or `ALL_PROXY` say, so a page sent to a local model never leaves the machine. curl
8.7.1 has no loopback exemption of its own. A server the user names elsewhere is reached as the
user's proxy settings say, as curl applies them: naming a remote server is the user's choice to
send the page there.

**Why the key goes in on stdin (`-K -`) and not from a header file (`-H @file`).**
- Not argv: any local user can list a process's argv.
- Not a file: `-H @file` puts the key on disk. A crash or a kill leaves the file behind, and its
  permissions are one more thing to get right.
- Not curl's environment: another process of the same user can read it.
- Stdin is a pipe only curl reads, and nothing of it outlives the request.

**What is never written.** The key comes from `ETHOS_PARSER_ASSIST_KEY` and nowhere else. It is
never a flag, and never written to the replies file, stdout or stderr. The server's address is
never written to the replies file or to stdout.

**The replies file** is replaced through `FILE.N.part`. N is the first number no file holds,
created exclusively (`O_EXCL`), so a link planted there is never followed, and two runs on one FILE
never share one. It is then renamed over FILE. A run killed during that write can leave a
`FILE.N.part`, holding replies but no page, and later runs pass over it.

## 4. Names

| Name | When | `ask` | `check` |
| --- | --- | --- | --- |
| `server-unreachable` | curl exits 5, 6 or 7: no name, no connection | exit 4; FILE keeps the pages done | exit 2 |
| `curl-not-found` | no `curl` on `PATH` | exit 5 | exit 2 |
| `curl-not-run` | curl could not be started, waited for or read from, or this platform has no `/dev/fd` | exit 5 | exit 2 |
| `timeout` | curl exits 28 | the page's `error`; asked again next run | exit 1 |
| `http N` | the server answers N, not 200 | the page's `error`; asked again next run | exit 1 |
| `reply-unreadable` | a 200 that is not an OpenAI chat completion | the page's `error`; asked again next run | exit 1 |
| `reply-too-large` | curl exits 63, or the reply passes 64 MiB | the page's `error`; asked again next run | exit 1 |
| `curl-exit-N` | any other curl failure, as 52 for an empty reply | the page's `error`; asked again next run | exit 1 |
| `curl-killed` | curl was ended by a signal | the page's `error`; asked again next run | exit 1 |
| `body-not-written` | the config or the page could not be written to curl's pipes | the page's `error`; asked again next run | exit 1 |
| `reply-capped` | `finish_reason: length` | kept as a reply with `finish_reason: "length"`, named on stderr | reported |
| `reply-format-not-read` | a reply with no block marker | (the merge reads it as `no-blocks`) | exit 1 |

## 5. How it is tested

`assist/tests/transport.rs` runs the binary against a fake OpenAI-compatible server: a std-only
`TcpListener` on 127.0.0.1, port 0. The binary runs with an empty environment, so no proxy
variable and no real key reaches it. `PATH` holds one script, which logs curl's argv and
environment, then runs the system curl with a 3 s cap. The tests assert:
- the request bytes;
- the key in the `Authorization` header, and nowhere in argv, curl's environment, the file,
  stdout or stderr;
- a loopback server reached directly with `http_proxy`, `HTTPS_PROXY` or `ALL_PROXY` set, the
  proxy given no request;
- a redirect not followed, and a URL with `{a,b}` asked once;
- no file left on disk when a run is killed (`SIGKILL`) mid-request;
- a link planted at `FILE.0.part` not followed;
- every name in §4 but `curl-not-run`, `curl-killed` and `body-not-written`, which need a broken
  system;
- resume and the model checks;
- two runs giving identical files.

A mutation sweep removed each guard in turn: `-q`, `globoff`, `noproxy`, `max-filesize`, the key's
removal from curl's environment, `O_EXCL`, and the scheme's case. Adding `location` was swept too.
Each change failed a test (2026-11-15).

Nothing leaves loopback.

## 6. What this does not decide

- How `ethos-parser extract --assist` spawns the binary, and how it reports exits 4 and 5. A later
  step decides that.
- Shipping the binary in releases (Q2, step D8).
- A streaming request, concurrent pages, or retries within one run. A failed page is retried by
  running `ask` again.
