// Copyright 2026 The ethos-parser maintainers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The transport: the system `curl`, one process per request (`docs/37-ASSIST-TRANSPORT.md`).
//! No HTTP crate is linked; the binary's only network surface is the `curl` it runs.
//!
//! curl runs as `curl -q -K -`. `-q` keeps a user's `.curlrc` out (it could turn on tracing that
//! writes headers to a file). Everything else, the URL and the key's header included, goes in
//! as a config on curl's stdin, a pipe only curl reads: not argv, which any local user can list;
//! not curl's environment, from which the key's variable is removed; and no file on disk. The
//! body goes in on a second pipe, curl's fd 2, read as `@/dev/fd/2`, so no file ever holds the
//! page, even when the run is killed. `silent` writes nothing to curl's stderr.
//!
//! A loopback server is never reached through a proxy: `noproxy` always lists loopback, then the
//! user's own `no_proxy`. Any other server is reached as the user's proxy settings say.

use std::io::{Read as _, Write as _};
use std::process::{Command, Stdio};

use crate::ask::KEY_VAR;

/// The longest a request may take, connection and reply: Phase 4a's per-page limit.
const MAX_SECONDS: u32 = 1800;
/// The longest a connection may take to open.
const CONNECT_SECONDS: u32 = 10;
/// The largest reply taken: a page's reply is tens of kilobytes. curl stops at it
/// (`max-filesize`; an unannounced length only since curl 8.4), and so does the read below.
const MAX_REPLY_BYTES: u64 = 64 << 20;
/// Hosts never reached through a proxy.
const LOOPBACK: &str = "localhost,127.0.0.1,::1";

/// A server's answer: its status and body.
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request got no answer.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// No `curl` on `PATH`.
    NoCurl,
    /// curl could not be started, waited for or read from.
    NotRun(String),
    /// curl could not resolve the host or open a connection (exit 5, 6 or 7).
    Unreachable,
    /// The request ran past its time limit (exit 28).
    Timeout,
    /// The reply is over [`MAX_REPLY_BYTES`] (exit 63, or more read than that).
    TooLarge,
    /// Any other curl failure, by its exit code; `None` when killed by a signal.
    Curl(Option<i32>),
    /// The config or the body could not be handed to curl.
    NotSent(String),
}

impl Failure {
    /// The failure's name, as the replies file and the output give it. Never the server's
    /// address: curl's own messages, which name it, are not read.
    pub fn name(&self) -> String {
        match self {
            Failure::NoCurl => "curl-not-found".into(),
            Failure::NotRun(e) => format!("curl-not-run ({e})"),
            Failure::Unreachable => "server-unreachable".into(),
            Failure::Timeout => "timeout".into(),
            Failure::TooLarge => "reply-too-large".into(),
            Failure::Curl(Some(code)) => format!("curl-exit-{code}"),
            Failure::Curl(None) => "curl-killed".into(),
            Failure::NotSent(e) => format!("body-not-written ({e})"),
        }
    }
}

/// A value in a curl config line: quoted, with curl's escapes, so no value can end its line.
fn quoted(v: &str) -> String {
    let mut out = String::from("\"");
    for c in v.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// curl's `noproxy` list: loopback always, then the user's `no_proxy` (curl's own order), which
/// the option would otherwise replace.
fn noproxy(user: Option<&str>) -> String {
    match user.map(str::trim) {
        Some("*") => "*".into(),
        Some(u) if !u.is_empty() => format!("{LOOPBACK},{u}"),
        _ => LOOPBACK.into(),
    }
}

/// Hands curl the body on a pipe, its fd 2, read as `@/dev/fd/2`: no file holds the page.
#[cfg(unix)]
fn body_pipe(cmd: &mut Command, config: &mut String) -> Result<std::io::PipeWriter, Failure> {
    let (reader, writer) = std::io::pipe().map_err(|e| Failure::NotSent(e.to_string()))?;
    cmd.stderr(reader);
    config.push_str("data-binary = \"@/dev/fd/2\"\n");
    Ok(writer)
}

/// No `/dev/fd` here: refused, rather than writing the page to a file.
#[cfg(not(unix))]
fn body_pipe(_: &mut Command, _: &mut String) -> Result<std::io::PipeWriter, Failure> {
    Err(Failure::NotRun(
        "the page reaches curl as /dev/fd/2, which this platform does not have".into(),
    ))
}

/// `GET url`, or `POST url` with a JSON `body`. `key`, when given, is sent as a bearer token.
pub fn request(url: &str, key: Option<&str>, body: Option<&[u8]>) -> Result<Response, Failure> {
    let user = std::env::var("no_proxy").or_else(|_| std::env::var("NO_PROXY"));
    let mut config = format!(
        "url = {}\nproto = \"=http,https\"\ngloboff\nnoproxy = {}\nsilent\n\
         max-time = {MAX_SECONDS}\nconnect-timeout = {CONNECT_SECONDS}\n\
         max-filesize = {MAX_REPLY_BYTES}\nwrite-out = \"\\n%{{http_code}}\"\n",
        quoted(url),
        quoted(&noproxy(user.ok().as_deref()))
    );
    if let Some(key) = key {
        config += &format!(
            "header = {}\n",
            quoted(&format!("Authorization: Bearer {key}"))
        );
    }
    let mut cmd = Command::new("curl");
    cmd.args(["-q", "-K", "-"])
        .env_remove(KEY_VAR)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let body = match body {
        Some(body) => {
            config += "header = \"Content-Type: application/json\"\n";
            Some((body, body_pipe(&mut cmd, &mut config)?))
        }
        None => None,
    };
    let spawned = cmd.spawn();
    // Our copy of the body pipe's read end goes with `cmd`: a curl that exits unread then
    // fails the write below rather than blocking it.
    drop(cmd);
    let mut child = spawned.map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Failure::NoCurl,
        _ => Failure::NotRun(e.to_string()),
    })?;
    // The config first: curl reads the body while it reads the config, before any transfer.
    let mut sent = child
        .stdin
        .take()
        .map_or(Ok(()), |mut stdin| stdin.write_all(config.as_bytes()));
    if let Some((body, mut pipe)) = body {
        sent = sent.and_then(|()| pipe.write_all(body));
    }
    let mut out = Vec::new();
    // `write-out` adds a line after the reply, and curl may pass its limit by a buffer.
    let limit = MAX_REPLY_BYTES + (1 << 20);
    let read = child
        .stdout
        .take()
        .map_or(Ok(0), |stdout| stdout.take(limit).read_to_end(&mut out));
    let over = out.len() as u64 >= limit;
    if over {
        let _ = child.kill();
    }
    let status = child.wait().map_err(|e| Failure::NotRun(e.to_string()))?;
    match status.code() {
        _ if over => return Err(Failure::TooLarge),
        Some(0) => {}
        Some(5..=7) => return Err(Failure::Unreachable),
        Some(28) => return Err(Failure::Timeout),
        Some(63) => return Err(Failure::TooLarge),
        code => return Err(Failure::Curl(code)),
    }
    sent.map_err(|e| Failure::NotSent(e.to_string()))?;
    read.map_err(|e| Failure::NotRun(e.to_string()))?;
    // `write-out` puts the status after the body, on a line of its own.
    let cut = out.iter().rposition(|&b| b == b'\n').unwrap_or(0);
    let status = std::str::from_utf8(&out[cut..])
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    out.truncate(cut);
    Ok(Response { status, body: out })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No value can end its config line and start another option.
    #[test]
    fn a_config_value_stays_on_its_line() {
        let v = quoted("a\"b\\c\nurl = \"file:///etc/passwd\"\r\t\u{b}");
        assert!(!v.contains('\n') && !v.contains('\r'));
        assert_eq!(v, r#""a\"b\\c\nurl = \"file:///etc/passwd\"\r\t\v""#);
    }

    /// Loopback is never proxied, and the user's own list is kept beside it.
    #[test]
    fn noproxy_always_lists_loopback() {
        assert_eq!(noproxy(None), "localhost,127.0.0.1,::1");
        assert_eq!(noproxy(Some(" ")), "localhost,127.0.0.1,::1");
        assert_eq!(
            noproxy(Some("corp.example,10.0.0.0/8")),
            "localhost,127.0.0.1,::1,corp.example,10.0.0.0/8"
        );
        assert_eq!(noproxy(Some("*")), "*");
    }
}
