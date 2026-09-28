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

//! MCP over stdio — the first adapter (v1.2-S1).
//!
//! # Why this host, and why it is also the hazard
//!
//! `docs/history/12-V12-SCOPE.md` §2 puts MCP first and says why: it is the only host whose native return
//! type carries a locator under an **enforced** schema (`outputSchema` + `structuredContent`),
//! where every other host on the list is effectively `Dict[str, Any]`. One server
//! reaches all of them — including the ones whose licences this repository refuses — without
//! dragging those licences toward us.
//!
//! The same section states the hazard in the same breath, and it is not a small one:
//!
//! > MCP tools are model-controlled — the model chooses the arguments. If any tool accepts a
//! > locator as a free-text argument that the engine then trusts, the model has become the
//! > citation authority in a single step.
//!
//! A model that can type `{"page": 3, "bbox": [10, 10, 90, 40]}` into a tool this engine believes
//! has *become* the thing the repository exists to prevent, and the result looks exactly like a
//! citation because it is shaped like one.
//!
//! # The handle law, which is why this file is shaped the way it is
//!
//! §16.7's mitigation is structural rather than advisory — **the engine mints every locator,
//! returns it as an opaque handle, and re-validates it on the way back in** — and
//! `docs/history/12-V12-SCOPE.md` §3 makes it three obligations:
//!
//! 1. **Mint.** Every locator a caller sees came out of an artifact this engine already emits.
//!    No tool here computes a page, a box or a cell.
//! 2. **Opaque.** A locator travels back as the bytes that were handed out. [`node_get`] takes a
//!    node id **string**, copied from a representation, and **no tool argument anywhere in this
//!    file names a coordinate** — not `page`, not `bbox`, not `x`/`y`/`width`/`height`, not a
//!    row/column pair. A test reads the advertised schemas and fails on those names.
//! 3. **Re-validate.** A representation is fingerprint-checked before it is read, exactly as
//!    `ethos-parser ground` checks it — in this call, or in an earlier call of this session over
//!    bytes this call proves identical by hashing every byte it read — and a node id is looked up
//!    among the nodes parsed from **this call's** bytes.
//!
//! **A handle this engine did not mint fails closed** — a tool error, never an empty result. An
//! empty result tells a model its guess was unlucky; an error tells it the guess was not
//! admissible.
//!
//! # What the server keeps between calls: which exact bytes already verified, and nothing else
//!
//! `docs/history/12-V12-SCOPE.md` §5 recorded *"No process-global document cache makes call 2
//! depend on call 1"*, and `docs/00-NORTH-STAR.md` decision 24 amends it: **no call's answer
//! depends on an earlier call; an earlier call may make a later call on byte-identical input
//! cheaper.** Verification rebuilds and hashes the whole payload, and it was two-thirds of a
//! `node_get` and most of a `ground` (`docs/measurements/memory-ceiling/` §14) — so a host asking
//! fifty questions of one artifact paid for it fifty times.
//!
//! [`ledger`] remembers the SHA-256 of each path-form buffer that parsed and verified, and a call
//! whose own bytes hash to one of them skips `verify_fingerprint` and nothing else: it still reads,
//! parses and structurally checks its own bytes, and answers from them. Verification is a
//! deterministic function of the bytes within one binary, so skipping it for identical bytes
//! cannot change a reply — every reply is the one a fresh process gives. What is deliberately NOT
//! the key:
//!
//! - **the path, the inode, the size or the modification time** — a same-length rewrite with its
//!   mtime restored defeats every one of them, and the ledger is never handed a path at all;
//! - **the declared fingerprint** — the caller wrote it, and an edited payload that kept it would
//!   share the key with the original and skip the one check that refuses it;
//! - **anything computed from a parsed `Value`** — its serialization is a build property under
//!   `preserve_order`, so inline arguments are never remembered and always verified.
//!
//! No document, tree, path or byte buffer is kept: 64 digests at most, least recently used first
//! out, about 6 KiB whatever the input. Nothing a model can name or enumerate lives there — no
//! argument takes a digest and no reply prints one. **One bit leaks through latency**: that these
//! exact bytes were verified earlier in this session. It needs the full preimage and reveals no
//! locator; the only way to close it is to verify every time, which is the cost this removes.
//!
//! # One process per line
//!
//! **This process answers no request itself.** Each line goes to a process of its own, this
//! binary as `mcp --call`, with the ledger line the session's last answered line left; that process
//! writes back the ledger line its request leaves, then the reply, which is copied to the host as
//! it arrives and never held here. The reason is memory the system allocator keeps: a call's live
//! heap went back to 5 MiB and the server's resident memory did not, so sixteen `locate` calls on
//! `nist-sp-800-161r1` took one server from 448 MiB to 4,480 in one run and 5,080 in another,
//! about 320 MiB a call (review 2026-09-26 N35). A process that ends returns all of it: the same
//! sixteen calls peak at about 735 MiB each, and between calls the server holds 2.4 MiB. Every
//! reply is byte-identical.
//! A call that aborts takes only its own process down: its line is answered with JSON-RPC's
//! internal error and the session goes on. The cost is a process start, 7.3 ms per line on macOS,
//! and one more thing: a host that kills the server mid-call leaves that call's process running
//! until it finishes or its first write fails. `docs/measurements/memory-ceiling/` §16 has the
//! numbers.
//!
//! The ledger crosses a pipe between processes of one session, one binary, and nothing else
//! writes to it. A line whose process ends without a whole reply carries none on. The ledger line
//! names this binary's version, and a process of another — the binary replaced under a running
//! server, whose next line runs the new one — starts from an empty ledger, so it verifies what the
//! old one verified. Run by hand, `mcp --call` believes the ledger line it is handed, which proves
//! nothing a caller could not prove by rewriting the declared fingerprint: it is a digest, not a
//! signature.
//!
//! # Why there is no framework here
//!
//! `deny.toml` bans `tokio`, `hyper`, `reqwest`, `ureq`, `rustls` and the rest of the reachable
//! network surface, and v1.2 does not file an ADR to widen that. The MCP crates available pull an
//! async runtime, which would spend the ban to save a few dozen lines of `match`. So this is a
//! loop over stdin with the `serde_json` the workspace already had.
//!
//! **stdio, newline-delimited JSON-RPC** — one request object per line in, one response per line
//! out. That is MCP's own stdio transport rather than a dialect invented here, and it is a pipe:
//! no socket, no TLS, no runtime.
//!
//! # Not logic
//!
//! `docs/04-ARCHITECTURE.md` §1 keeps logic out of the CLI, and this file holds none: every tool
//! calls the same library entry point the matching subcommand calls, and the artifacts are the
//! artifacts. What lives here is protocol plumbing, which is why it is a CLI module rather than a
//! fifth crate or a new concept in `ethos-parser-core`. [`ledger`] is plumbing too — it remembers a
//! verdict core already reached, per session — and it stays here on purpose: a core entry point
//! that skips verification for a digest would be the fingerprint-accepting constructor core
//! refuses, and it is sound only inside the session that did the verifying.

use std::io::{BufRead, Write};

use ethos_parser_core::{DocumentRepresentation, EngineError};
use serde_json::{json, Value};

/// The MCP revision this server implements.
///
/// Reported verbatim in `initialize`. A host that wants a different one gets this one and decides
/// for itself — guessing at a revision we do not implement would be the protocol equivalent of
/// inventing a coordinate.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// JSON-RPC's own codes, plus the one this server actually uses.
const INVALID_PARAMS: i64 = -32602;
const METHOD_NOT_FOUND: i64 = -32601;
const INTERNAL_ERROR: i64 = -32603;

/// The most a line's process may write before its ledger line ends: 64 digests of about 80 bytes
/// each, and the version.
const CARRIED_LIMIT: u64 = 16 * 1024;

/// Run the server until stdin closes.
///
/// One JSON object per line. A line that is not JSON, or is JSON this server has no method for,
/// gets a JSON-RPC error rather than a panic or a silent skip — a host that mis-frames a request
/// should learn that from the response, not from a closed pipe.
///
/// **Each line is answered by a process of its own**, this binary as `mcp --call`; this one carries
/// the lines out, the replies back and the ledger between them (*One process per line*, above).
pub fn serve(input: impl BufRead, output: impl Write) -> std::io::Result<()> {
    let program = std::env::current_exe()?;
    serve_with(input, output, |carried, line, out| {
        answer_in_child(&program, carried, line, out)
    })
}

/// [`serve`], with how a line is answered supplied, so a test can stand in for the process.
///
/// `answer` is given the ledger line the session's last answered line left, and the line; it
/// writes the reply, if the line takes one, and returns the ledger line to carry on — `None` when
/// the line's process failed, which carries the one before it on unchanged.
fn serve_with(
    mut input: impl BufRead,
    mut output: impl Write,
    mut answer: impl FnMut(&str, &str, &mut dyn Write) -> std::io::Result<Option<String>>,
) -> std::io::Result<()> {
    let mut carried = String::new();
    // One buffer for every line, so the server holds its longest line and no more. A fresh one per
    // line grew it by a line per call: six calls carrying a 90 MB representation inline took it
    // from 90 to 520 MiB, where this one stays at 90.
    let mut buffer = String::new();
    loop {
        buffer.clear();
        if input.read_line(&mut buffer)? == 0 {
            return Ok(());
        }
        // `BufRead::lines`'s own trim: the newline, then one carriage return before it.
        let line = match buffer.strip_suffix('\n') {
            Some(line) => line.strip_suffix('\r').unwrap_or(line),
            None => &buffer,
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(next) = answer(&carried, line, &mut output)? {
            carried = next;
        }
        output.flush()?;
    }
}

/// One line of a session, in the process [`serve`] started for it: the ledger line on the first
/// line of `input` and the request on the second. Writes the ledger line as the request leaves
/// it, then the reply, if the request takes one.
pub fn serve_call(mut input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    let mut carried = String::new();
    input.read_line(&mut carried)?;
    let mut line = String::new();
    input.read_line(&mut line)?;
    let mut ledger = ledger::Ledger::carried(carried.trim_end_matches('\n'));
    // A notification (no `id`) takes no reply, which is JSON-RPC's rule rather than a shortcut:
    // answering `notifications/initialized` is a protocol error.
    let reply = handle_line(line.trim_end_matches('\n'), &mut ledger);
    writeln!(output, "{}", ledger.carry())?;
    if let Some(reply) = reply {
        reply.write_to(&mut output)?;
        writeln!(output)?;
    }
    output.flush()
}

/// Answer `line` in a process of its own running `program`, this binary, as `mcp --call`.
fn answer_in_child(
    program: &std::path::Path,
    carried: &str,
    line: &str,
    output: &mut dyn Write,
) -> std::io::Result<Option<String>> {
    use std::process::{Command, Stdio};

    let spawned = Command::new(program)
        .args(["mcp", "--call"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            let how = format!("it did not start: {e}");
            return settle(line, Relayed::default(), false, &how, output);
        }
    };
    // Both lines go in before anything is read, which cannot deadlock: the process reads both
    // before it writes. A failed write means it has already ended, and its status says how.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = [carried.as_bytes(), b"\n", line.as_bytes(), b"\n"]
            .iter()
            .try_for_each(|part| stdin.write_all(part));
    }
    let mut relayed = Relayed::default();
    if let Some(stdout) = child.stdout.take() {
        let mut stdout = std::io::BufReader::with_capacity(1 << 16, stdout);
        relayed = relay(&mut stdout, output)?;
        // Anything past what was relayed, which this binary never writes, is read and dropped so
        // that the process can end.
        let _ = std::io::copy(&mut stdout, &mut std::io::sink());
    }
    let status = child.wait()?;
    settle(line, relayed, status.success(), &status.to_string(), output)
}

/// What a line's process wrote back, as far as [`relay`] read it.
#[derive(Default)]
struct Relayed {
    /// Its first line, the ledger its line leaves, when that line arrived whole.
    carried: Option<String>,
    /// Whether any byte of a reply was copied to the host.
    forwarded: bool,
    /// Whether the last byte copied ended its line.
    ended: bool,
}

/// Read a line's process's ledger line, then copy what follows it, the reply, to `output` as it
/// arrives. The reply can be the whole representation `extract` returns, and it is never held
/// here.
///
/// A read error ends the copy, and is the process's failure, which its status reports. A write
/// error is the host's, and is returned.
fn relay(from: &mut impl BufRead, output: &mut dyn Write) -> std::io::Result<Relayed> {
    let mut relayed = Relayed::default();
    let mut carried = String::new();
    match std::io::Read::take(&mut *from, CARRIED_LIMIT).read_line(&mut carried) {
        Ok(_) if carried.ends_with('\n') => {
            carried.pop();
            relayed.carried = Some(carried);
        }
        _ => return Ok(relayed),
    }
    loop {
        let chunk = match from.fill_buf() {
            Ok([]) => break,
            Ok(chunk) => chunk,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        output.write_all(chunk)?;
        relayed.forwarded = true;
        relayed.ended = chunk.last() == Some(&b'\n');
        let n = chunk.len();
        from.consume(n);
    }
    Ok(relayed)
}

/// Settle a line whose process has ended: its ledger line, when it exited cleanly after a whole
/// reply or none. Otherwise `None`, which carries the session's ledger on unchanged, and an
/// internal error for the line's own id — after a newline if part of a reply went out, and not at
/// all if the whole of one did.
fn settle(
    line: &str,
    relayed: Relayed,
    clean: bool,
    how: &str,
    mut output: &mut dyn Write,
) -> std::io::Result<Option<String>> {
    let whole = !relayed.forwarded || relayed.ended;
    if clean && whole && relayed.carried.is_some() {
        return Ok(relayed.carried);
    }
    if relayed.forwarded && relayed.ended {
        // Answering the id a second time would break the protocol.
        return Ok(None);
    }
    if relayed.forwarded {
        writeln!(output)?;
    }
    if let Some(reply) = failed(line, how) {
        reply.write_to(&mut output)?;
        writeln!(output)?;
    }
    Ok(None)
}

/// The answer to a line whose process ended without a whole one: JSON-RPC's internal error for
/// the line's own id, or none for a notification. A line that does not parse gets the null id its
/// parse error would have.
fn failed(line: &str, how: &str) -> Option<Reply> {
    let id = match serde_json::from_str::<Value>(line) {
        Ok(request) => request.get("id")?.clone(),
        Err(_) => Value::Null,
    };
    Some(Reply::Json(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": INTERNAL_ERROR,
            "message": format!(
                "the process answering this request ended without a whole reply: {how}"
            )
        }
    })))
}

/// One line in, at most one response out.
fn handle_line(line: &str, ledger: &mut ledger::Ledger) -> Option<Reply> {
    let mut request: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        // No id to answer with, so this is the one case that answers with a null id — the JSON-RPC
        // spec's own provision for an unparseable request.
        Err(e) => {
            return Some(Reply::Json(json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("parse error: {e}") }
            })))
        }
    };

    // `params` is moved out rather than cloned: an inline representation is the bulk of the
    // request, and every copy of it is another tree the size of the artifact.
    let id = request.get("id").cloned();
    let params = request
        .get_mut("params")
        .map(Value::take)
        .unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");

    // A notification has no `id`. It is acted on and not answered.
    let id = id?;

    Some(match dispatch(method, params, ledger) {
        Ok(Outcome::Value(result)) => {
            Reply::Json(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
        }
        Ok(Outcome::Artifact { summary, artifact }) => Reply::Artifact {
            id,
            summary,
            artifact,
        },
        Err(e) => Reply::Json(json!({ "jsonrpc": "2.0", "id": id, "error": e.to_json() })),
    })
}

/// What [`dispatch`] produces: a result object, or a tool's artifact still in canonical bytes.
enum Outcome {
    Value(Value),
    Artifact { summary: String, artifact: Vec<u8> },
}

/// What a tool hands back for `structuredContent`.
enum Artifact {
    /// Canonical bytes, written into the response verbatim and never parsed into a tree.
    Bytes(Vec<u8>),
    /// A value small enough that serializing it with the envelope costs nothing — one node.
    Value(Value),
}

/// One response line, before its newline.
enum Reply {
    Json(Value),
    Artifact {
        id: Value,
        summary: String,
        artifact: Vec<u8>,
    },
}

impl Reply {
    /// Write this reply, without the trailing newline.
    ///
    /// **An artifact is never parsed back into a tree.** The route this replaces handed the
    /// canonical bytes to `serde_json::from_slice` so they could sit inside a `json!` envelope,
    /// then serialized the envelope — tree and all — back into one String to print it. The
    /// artifact existed three times over, and the tree is several times the text it came from:
    /// on a 4.6 MB PDF, peak memory was 1.4 GiB through `extract` on the CLI and 8.7 GiB
    /// through the same tool here, on the surface this module calls the one that matters most.
    ///
    /// **Canonical in every build, which the old route was not.** A release build's `serde_json`
    /// sorts object keys, so the envelope read `id`, `jsonrpc`, `result`, the result read
    /// `content`, `isError`, `structuredContent`, and the artifact was LAST at both levels — the
    /// old output was literally this prefix, the artifact's bytes, and `}}`. But that order is a
    /// property of the build, not of this code: `preserve_order`, which core enables in its
    /// dev-dependencies precisely because it is a hazard, unifies into `cargo test --workspace`,
    /// and there the old route printed the same envelope in insertion order. MCP's envelope bytes
    /// differed between the build that ships and the build the gate tests for as long as this
    /// module has existed. The artifact inside never did, because c14n sorts at write time.
    ///
    /// So every key of an artifact reply is written out here in sorted order — the envelope's,
    /// the result's, and the one summary object's in `content` — and the artifact arrives already
    /// canonical. That is exactly what a release build printed, so the shipped wire bytes do not
    /// move, and it no longer depends on which features unify.
    /// `an_artifact_reply_is_canonical_in_every_build` compares it with `c14n_bytes` of the same
    /// envelope, and because the gate runs that test under `preserve_order`, the hazard is
    /// exercised rather than described.
    fn write_to(&self, out: &mut impl Write) -> std::io::Result<()> {
        match self {
            Reply::Json(v) => write!(out, "{v}"),
            Reply::Artifact {
                id,
                summary,
                artifact,
            } => {
                let summary = Value::from(summary.as_str());
                write!(
                    out,
                    r#"{{"id":{id},"jsonrpc":"2.0","result":{{"content":[{{"text":{summary},"type":"text"}}],"isError":false,"structuredContent":"#
                )?;
                out.write_all(artifact)?;
                out.write_all(b"}}")
            }
        }
    }
}

/// A JSON-RPC error, or a tool error carried inside a successful result.
///
/// The distinction is MCP's and it matters: a *protocol* failure is a JSON-RPC `error`, while a
/// *tool* failure is a result with `isError: true` so the model can see and react to it. A forged
/// handle is a tool failure — the call was well-formed, the answer is no.
#[derive(Debug)]
struct Failure {
    code: i64,
    message: String,
}

impl Failure {
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn to_json(&self) -> Value {
        json!({ "code": self.code, "message": self.message })
    }
}

impl From<&EngineError> for Failure {
    fn from(e: &EngineError) -> Self {
        Failure::new(INVALID_PARAMS, e.to_string())
    }
}

fn dispatch(method: &str, params: Value, ledger: &mut ledger::Ledger) -> Result<Outcome, Failure> {
    match method {
        "initialize" => Ok(Outcome::Value(json!({
            "protocolVersion": PROTOCOL_VERSION,
            // `tools` and nothing else. No resources, no prompts, no sampling: each is a surface
            // that would have to obey the handle law, and none of them is needed to return an
            // artifact.
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "ethos-parser", "version": env!("CARGO_PKG_VERSION") },
        }))),
        "tools/list" => Ok(Outcome::Value(json!({ "tools": tools() }))),
        "tools/call" => call_tool(params, ledger),
        "ping" => Ok(Outcome::Value(json!({}))),
        other => Err(Failure::new(
            METHOD_NOT_FOUND,
            format!("no method `{other}`"),
        )),
    }
}

// -------------------------------------------------------------------------------------------
// The tools
// -------------------------------------------------------------------------------------------

/// The advertised tool list.
///
/// **Read the argument schemas as the enforcement point.** Not one of them names a coordinate,
/// and that is the corollary of the handle law rather than an oversight: a caller that needs a
/// node passes the id this engine minted, and a caller that needs a region is asking for a slice
/// that has not shipped.
fn tools() -> Value {
    json!([
        {
            "name": "extract",
            "description":
                "Read a document — PDF, DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF or EPUB — and return \
                 `DocumentRepresentation v0`, the canonical evidence record. Byte-identical to `ethos-parser extract`. Every locator a later call needs is \
                 minted here; pass this artifact back rather than composing one.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["path"],
                "properties": {
                    "path": { "type": "string", "description": "Path to the document." }
                }
            }
        },
        {
            "name": "ground",
            "description":
                "Project a representation into `ethos.grounding.v1`. Takes the artifact `extract` \
                 returned; the representation is fingerprint-checked before it is read.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["representation"],
                "properties": {
                    "representation": {
                        "description":
                            "The artifact `extract` returned — the object itself, or a path to \
                             bytes this engine wrote.",
                        "type": ["object", "string"]
                    }
                }
            }
        },
        {
            "name": "node_get",
            "description":
                "Return one node from a representation. `node_id` is an OPAQUE HANDLE: copy it \
                 from a representation this engine returned. It is re-validated against that \
                 artifact, and an id this engine did not mint is an error. There is no way to ask \
                 for a node by page or by position.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["representation", "node_id"],
                "properties": {
                    "representation": {
                        "description":
                            "The artifact `extract` returned — the object itself, or a path to \
                             bytes this engine wrote.",
                        "type": ["object", "string"]
                    },
                    "node_id": {
                        "type": "string",
                        "description":
                            "A node id copied verbatim from that representation's `nodes`."
                    }
                }
            }
        },
        {
            "name": "locate",
            "description":
                "Report WHERE a string lies in a representation, and nothing else. Returns \
                 `ethos.parser.locations.v0`: occurrences as node ids, character offsets into \
                 each node's own text, and that node's own geometry. It answers no question \
                 about whether anything is true or supported, and a string that occurs nowhere \
                 is an empty list of occurrences — the same answer shape, never an error. The \
                 match is exact on Unicode scalars with no normalization or case folding, over \
                 each block of the reading order, so a match may join runs inside one block and \
                 never across two.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["representation", "quote"],
                "properties": {
                    "representation": {
                        "description":
                            "The artifact `extract` returned — the object itself, or a path to \
                             bytes this engine wrote.",
                        "type": ["object", "string"]
                    },
                    "quote": {
                        "type": "string",
                        "description":
                            "The string to find, verbatim. It is matched exactly: no trimming, \
                             no case folding, no Unicode normalization."
                    }
                }
            }
        }
    ])
}

fn call_tool(mut params: Value, ledger: &mut ledger::Ledger) -> Result<Outcome, Failure> {
    let mut args = params
        .get_mut("arguments")
        .map(Value::take)
        .unwrap_or(json!({}));
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`name` is required"))?;

    let outcome = match name {
        "extract" => tool_extract(&args),
        "ground" => tool_ground(&mut args, ledger),
        "node_get" => tool_node_get(&mut args, ledger),
        "locate" => tool_locate(&mut args, ledger),
        other => return Err(Failure::new(INVALID_PARAMS, format!("no tool `{other}`"))),
    };

    match outcome {
        // **The summary is for the model; the artifact is for the pipeline.** Counts only — no
        // box, no id, no cell. §16.7's split, and a test asserts no coordinate reaches here.
        Ok((summary, Artifact::Bytes(artifact))) => Ok(Outcome::Artifact { summary, artifact }),
        Ok((summary, Artifact::Value(artifact))) => Ok(Outcome::Value(json!({
            "content": [{ "type": "text", "text": summary }],
            "structuredContent": artifact,
            "isError": false,
        }))),
        // **A tool failure, not a protocol failure.** The call was well-formed and the answer is
        // no — which is what a forged handle must produce, so the model sees a refusal rather
        // than a plausible guess.
        Err(f) => Ok(Outcome::Value(json!({
            "content": [{ "type": "text", "text": f.message }],
            "isError": true,
        }))),
    }
}

/// `path` → the canonical representation, and the same bytes `ethos-parser extract` prints.
fn tool_extract(args: &Value) -> Result<(String, Artifact), Failure> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`path` is required and must be a string"))?;

    // Through the same router the CLI uses — a DOCX over MCP used to be handed
    // straight to the PDF reader and refused for lacking a `%PDF-` header, the
    // wrong-cause refusal three CLI slices had already retired for their formats.
    // The same ceiling the CLI applies (v2-S15). MCP is a long-lived process handling untrusted
    // documents repeatedly, so an unbounded read here is the one that matters most.
    let head = read_supplied_file(path).map_err(|e| Failure::from(&e))?;
    // The default profile: MCP exposes no knobs, so an artifact from this surface is the
    // unbounded one, exactly as it was before `extract --max-pages` existed.
    let artifact = crate::representation_for_bytes(&head, &ethos_parser_core::Profile::default())
        .map_err(|e| Failure::from(&e))?;

    let summary = format!(
        "{} page(s), {} node(s). Locators are in the artifact.",
        artifact.payload().pages.len(),
        artifact.payload().nodes.len()
    );
    let bytes = artifact
        .to_canonical_bytes()
        .map_err(|e| Failure::from(&e))?;
    Ok((summary, Artifact::Bytes(bytes)))
}

/// A representation → `ethos.grounding.v1`.
fn tool_ground(
    args: &mut Value,
    ledger: &mut ledger::Ledger,
) -> Result<(String, Artifact), Failure> {
    let repr = representation_arg(args, ledger)?;
    let projection = ethos_parser_grounding::project(&repr).map_err(|e| Failure::from(&e))?;
    let bytes = ethos_parser_grounding::to_canonical_bytes(&projection.source)
        .map_err(|e| Failure::from(&e))?;
    let summary = ground_summary(&projection);
    Ok((summary, Artifact::Bytes(bytes)))
}

/// The words `ground` reports: the count of elements, the geometry omission, and one clause for
/// each thing the schema's limits took.
///
/// **The one place this sentence is written.** The Python and Node LangChain adapters return it
/// verbatim from this server's reply rather than composing their own, so the test that pins it here
/// is the pin for all three. The destructure is exhaustive on purpose: a field added to
/// [`ethos_parser_grounding::Projection`] fails to compile here until this summary says what it
/// reports about it.
fn ground_summary(projection: &ethos_parser_grounding::Projection) -> String {
    let ethos_parser_grounding::Projection {
        source,
        omission,
        spans_withheld,
        elements_omitted,
        tables_withheld,
    } = projection;
    let mut summary = format!(
        "{} element(s) with a measured box; {} omitted for having none.",
        source.elements.len(),
        omission.nodes_omitted
    );
    if let Some(w) = spans_withheld {
        summary.push_str(&format!(
            " {} span(s) withheld, more than the {} the schema admits: elements only.",
            w.spans, w.limit
        ));
    }
    if let Some(o) = elements_omitted {
        summary.push_str(&format!(
            " {} element(s) omitted, a string over the schema's byte limit.",
            o.elements
        ));
    }
    if let Some(t) = tables_withheld {
        summary.push_str(&format!(
            " {} table(s) withheld, over the schema's limits: no tables.",
            t.tables
        ));
    }
    summary
}

/// **The handle law, made mechanical.**
///
/// The representation is re-validated (fingerprint) and the id is looked up among *that*
/// artifact's own nodes. Not found is an error, deliberately: an empty result would tell a model
/// its guess was merely unlucky.
fn tool_node_get(
    args: &mut Value,
    ledger: &mut ledger::Ledger,
) -> Result<(String, Artifact), Failure> {
    let repr = representation_arg(args, ledger)?;
    let node_id = args.get("node_id").and_then(Value::as_str).ok_or_else(|| {
        Failure::new(INVALID_PARAMS, "`node_id` is required and must be a string")
    })?;

    let node = repr
        .payload()
        .nodes
        .iter()
        .find(|n| n.id.as_str() == node_id)
        .ok_or_else(|| {
            Failure::new(
                INVALID_PARAMS,
                format!(
                    "`{node_id}` is not a node of this representation ({}). A node id is an \
                     opaque handle this engine minted: copy one from the artifact rather than \
                     composing it.",
                    repr.fingerprint()
                ),
            )
        })?;

    let value = serde_json::to_value(node)
        .map_err(|e| Failure::new(INVALID_PARAMS, format!("node will not serialize: {e}")))?;
    Ok((
        format!("1 node, kind `{:?}`.", node.kind),
        Artifact::Value(value),
    ))
}

/// A representation and a string → `ethos.parser.locations.v0`.
///
/// **No verdict reaches the model, and none can be composed from what does.** A string that occurs
/// nowhere is `0 occurrence(s)` with `isError: false` — the same reply shape a found one gets — so
/// the tool cannot be used as a truth oracle by reading its error channel. That is decision #30's
/// own re-refusal condition, met here rather than described.
fn tool_locate(
    args: &mut Value,
    ledger: &mut ledger::Ledger,
) -> Result<(String, Artifact), Failure> {
    let repr = representation_arg(args, ledger)?;
    let quote = args
        .get("quote")
        .and_then(Value::as_str)
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`quote` is required and must be a string"))?;

    let profile = ethos_parser_core::Profile::default();
    let profile_sha256 = profile
        .profile_sha256()
        .map_err(|e| Failure::new(INVALID_PARAMS, format!("profile will not hash: {e}")))?;
    let found = ethos_parser_core::locate(
        &repr,
        &profile.parser_version,
        &profile_sha256,
        &profile.locate_rule,
        quote,
    )
    .map_err(|e| Failure::from(&e))?;

    let summary = locate_summary(&found);
    let bytes = found.to_canonical_bytes().map_err(|e| Failure::from(&e))?;
    Ok((summary, Artifact::Bytes(bytes)))
}

/// The words `locate` reports: **counts, and nothing else.**
///
/// No node id, no offset, no box and neither digest — a summary carrying one would hand the model a
/// locator through the one channel it can rewrite, which is `12-V12-SCOPE.md` §3's Opaque
/// obligation. The phrase *not found* does not appear either, at any occurrence count: it is a
/// verdict in one sentence, and the count says the same thing without one.
///
/// The destructure is exhaustive on the same footing as [`ground_summary`]'s: a field added to
/// [`ethos_parser_core::Locations`] fails to compile here until this summary says whether it
/// reports it.
fn locate_summary(found: &ethos_parser_core::Locations) -> String {
    let ethos_parser_core::Locations {
        // The artifact's identity and both digests are the pipeline's, not the model's.
        identity: _,
        source_sha256: _,
        representation_sha256: _,
        // The rule id travels in the artifact. It is not a count, and a model that could read it
        // here could report a rule that did not run.
        locate_rule: _,
        quote_scalars,
        searched,
        occurrences,
        occurrences_withheld,
    } = found;

    if let Some(withheld) = occurrences_withheld {
        return format!(
            "{} occurrence(s) of {} scalar(s), more than the {} this artifact carries locators \
             for: the count only.",
            withheld.occurrences, quote_scalars, withheld.limit
        );
    }
    let mut summary = format!(
        "{} occurrence(s) of {quote_scalars} scalar(s), across {} block(s) of {} node(s) searched.",
        occurrences.len(),
        searched.blocks,
        searched.nodes,
    );
    if !occurrences.is_empty() {
        // Summed per occurrence, and said so: two overlapping occurrences that share a
        // synthesized scalar count it twice, because each of them matched it.
        let parts: usize = occurrences.iter().map(|o| o.parts.len()).sum();
        let synthesized: u32 = occurrences.iter().map(|o| o.synthesized).sum();
        summary.push_str(&format!(
            " {parts} node part(s), and {synthesized} synthesized scalar(s) counted per \
             occurrence. Locators are in the artifact."
        ));
    }
    summary
}

/// Read the `representation` argument and **re-validate it before anything reads it**.
///
/// Accepts the artifact inline or as a path to bytes this engine wrote — both are the same
/// artifact and both get the same check. `verify_fingerprint` is the whole point: a representation
/// whose payload does not hash to its declared digest is not a record this engine will speak for,
/// so a model that edited the JSON on the way through fails here instead of getting an answer
/// about a document that never existed.
///
/// **A path is read once and its bytes handed to the ledger by value**, which parses, hashes and —
/// unless these exact bytes already verified in this session — verifies that one buffer. The
/// ledger is never given the path. An inline object is verified every time, exactly as before.
fn representation_arg(
    args: &mut Value,
    ledger: &mut ledger::Ledger,
) -> Result<DocumentRepresentation, Failure> {
    let raw = args
        .get_mut("representation")
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`representation` is required"))?;

    match raw {
        Value::String(path) => {
            let bytes = read_supplied_file(path)
                .map_err(|e| Failure::new(INVALID_PARAMS, format!("{path}: {e}")))?;
            ledger.load(bytes)
        }
        other => {
            let repr: DocumentRepresentation = serde_json::from_value(other.take())
                .map_err(|e| Failure::new(INVALID_PARAMS, format!("representation: {e}")))?;
            repr.verify_fingerprint().map_err(|e| Failure::from(&e))?;
            Ok(repr)
        }
    }
}

/// Read a path a tool argument named — **a regular file, and nothing else.**
///
/// A model chooses these paths. `/dev/stdin` would read this server's own protocol stream, and a
/// FIFO with no writer blocks the open forever, so either one hung the session. A CLI caller names
/// its own pipes and keeps them; here a path that exists and is not a regular file is refused before
/// it is opened. A missing path falls through to the read, so its error reads as it always did.
/// Unix only in effect: Windows reports anything that is not a directory as a file.
fn read_supplied_file(path: &str) -> Result<Vec<u8>, EngineError> {
    match std::fs::metadata(path) {
        Ok(meta) if !meta.is_file() => Err(EngineError::Io {
            detail: format!("{path}: not a regular file, so this server will not read it"),
        }),
        _ => crate::read_source(std::path::Path::new(path)),
    }
}

/// **Which exact bytes already verified in this session** — digests, and nothing else.
///
/// The module doc's *What the server keeps between calls* is the argument; this is the mechanism.
/// Its code never touches the filesystem or a parsed tree's serialization, and
/// `the_ledger_never_sees_a_path_or_the_filesystem` reads this module's source to hold it there.
mod ledger {
    use std::collections::VecDeque;

    use ethos_parser_core::DocumentRepresentation;

    use super::{Failure, INVALID_PARAMS};

    /// How many verified digests are remembered. A constant, not a knob: MCP exposes none.
    pub(super) const CAPACITY: usize = 64;

    /// The identity of one buffer: its length and the SHA-256 of every byte of it.
    ///
    /// No `Default` and no `Clone`, and two ways in: [`Key::of`] hashes a buffer, and
    /// [`Ledger::carried`] takes back a key an earlier process of this session hashed. A key exists
    /// only because some buffer was hashed.
    #[derive(PartialEq, Eq)]
    struct Key {
        len: u64,
        sha256: String,
    }

    impl Key {
        fn of(bytes: &[u8]) -> Self {
            Key {
                len: bytes.len() as u64,
                sha256: ethos_parser_core::sha256_hex_bytes(bytes),
            }
        }
    }

    /// Digests of buffers that parsed and verified, least recently used at the front.
    pub(super) struct Ledger {
        verified: VecDeque<Key>,
        #[cfg(test)]
        pub(super) stats: Stats,
    }

    /// What the ledger did, for tests to hold it to.
    ///
    /// Counted inside `load` rather than observed from outside because some of it cannot be
    /// observed any other way: that a parse failure was never hashed leaves no trace in a reply.
    #[cfg(test)]
    #[derive(Default, Clone, Copy, Debug, PartialEq)]
    pub(super) struct Stats {
        pub(super) parses: u32,
        pub(super) hashes: u32,
        pub(super) verifies: u32,
        pub(super) hits: u32,
    }

    impl Ledger {
        pub(super) fn new() -> Self {
            Ledger {
                verified: VecDeque::new(),
                #[cfg(test)]
                stats: Stats::default(),
            }
        }

        /// Parse `bytes`, and verify them unless these exact bytes already verified.
        ///
        /// The order is the argument. A parse failure returns before anything is hashed, so a
        /// mistaken path to a 950 MiB PDF costs what it did. The buffer is freed before
        /// verification, as it was. And the only insertion is here, after verification succeeded
        /// on the tree parsed from the very buffer the key was hashed from — so an entry always
        /// means *these bytes verified*, and an error is never remembered.
        pub(super) fn load(&mut self, bytes: Vec<u8>) -> Result<DocumentRepresentation, Failure> {
            #[cfg(test)]
            {
                self.stats.parses += 1;
            }
            let repr: DocumentRepresentation = serde_json::from_slice(&bytes)
                .map_err(|e| Failure::new(INVALID_PARAMS, format!("representation: {e}")))?;

            let key = self.key_of(&bytes);
            drop(bytes);

            if let Some(i) = self.verified.iter().position(|k| *k == key) {
                if let Some(k) = self.verified.remove(i) {
                    self.verified.push_back(k);
                }
                #[cfg(test)]
                {
                    self.stats.hits += 1;
                }
                return Ok(repr);
            }

            #[cfg(test)]
            {
                self.stats.verifies += 1;
            }
            repr.verify_fingerprint().map_err(|e| Failure::from(&e))?;
            self.insert(key);
            Ok(repr)
        }

        /// The only route `load` takes to a key, so the test counter cannot drift from the hash.
        fn key_of(&mut self, bytes: &[u8]) -> Key {
            #[cfg(test)]
            {
                self.stats.hashes += 1;
            }
            Key::of(bytes)
        }

        fn insert(&mut self, key: Key) {
            if self.verified.len() == CAPACITY {
                self.verified.pop_front();
            }
            self.verified.push_back(key);
        }

        /// The ledger as the session's last answered line left it, from the line [`Ledger::carry`]
        /// wrote there: this binary's version, then `length:sha256` per key, least recently used
        /// first. Empty when the line is empty, malformed or longer than [`CAPACITY`] keys, and
        /// when it names another version: a binary replaced under a running server does not take
        /// the verdicts of the one it replaced. An empty ledger costs a verification, nothing else.
        pub(super) fn carried(line: &str) -> Self {
            let mut ledger = Ledger::new();
            let mut fields = line.split(' ');
            if fields.next() != Some(env!("CARGO_PKG_VERSION")) {
                return ledger;
            }
            for field in fields {
                let key = field.split_once(':').and_then(|(len, sha256)| {
                    Some(Key {
                        len: len.parse().ok()?,
                        sha256: sha256.to_owned(),
                    })
                });
                match key {
                    Some(key) if ledger.verified.len() < CAPACITY => ledger.verified.push_back(key),
                    _ => return Ledger::new(),
                }
            }
            ledger
        }

        /// This ledger as one line, for the server to carry to its next line's process.
        pub(super) fn carry(&self) -> String {
            let mut line = String::from(env!("CARGO_PKG_VERSION"));
            for key in &self.verified {
                line.push_str(&format!(" {}:{}", key.len, key.sha256));
            }
            line
        }

        #[cfg(test)]
        pub(super) fn len(&self) -> usize {
            self.verified.len()
        }

        #[cfg(test)]
        pub(super) fn knows(&self, bytes: &[u8]) -> bool {
            let key = Key::of(bytes);
            self.verified.iter().any(|k| *k == key)
        }

        /// A ledger that believes `bytes` verified when they never did — to prove a skip is decided
        /// by the key alone, and so that nothing else can be what makes a test pass.
        #[cfg(test)]
        pub(super) fn remembering_unverified(bytes: &[u8]) -> Self {
            let mut ledger = Ledger::new();
            ledger.verified.push_back(Key::of(bytes));
            ledger
        }
    }
}
// end mod ledger

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, params: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        render(
            handle_line(&line.to_string(), &mut ledger::Ledger::new())
                .expect("a request with an id gets a response"),
        )
    }

    /// A reply as the wire carries it, parsed back — the only form a host ever sees.
    fn render(reply: Reply) -> Value {
        let mut bytes = Vec::new();
        reply
            .write_to(&mut bytes)
            .expect("writing to a Vec cannot fail");
        serde_json::from_slice(&bytes).expect("a reply is one JSON value")
    }

    /// **An artifact reply is canonical in every build.**
    ///
    /// `Reply::write_to` writes every key order by hand, so it must equal the canonical
    /// serialization of the envelope the old `Value` route built — which is what a release build's
    /// `serde_json` prints. The reference is `c14n_bytes`, which sorts at write time whatever
    /// features unify, because `serde_json`'s own order is a build property: the gate runs this test
    /// under `preserve_order`, where the old route printed this envelope in insertion order. The
    /// summary carries a quote, a backslash and a non-ASCII character, so escaping is compared too.
    #[test]
    fn an_artifact_reply_is_canonical_in_every_build() {
        let value =
            json!({ "nested": { "b": [1, 2], "a": "caf\u{e9}" }, "artifact_type": "x", "n": 3 });
        let artifact = ethos_parser_core::c14n_bytes(&value).expect("canonical");
        let summary = "a \"quoted\" summary \\ caf\u{e9}".to_string();
        let id = json!(7);

        let mut spliced = Vec::new();
        Reply::Artifact {
            id: id.clone(),
            summary: summary.clone(),
            artifact,
        }
        .write_to(&mut spliced)
        .expect("write");

        let tree = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": summary }],
                "structuredContent": value,
                "isError": false,
            }
        });

        assert_eq!(
            String::from_utf8_lossy(&spliced),
            String::from_utf8_lossy(&ethos_parser_core::c14n_bytes(&tree).expect("canonical")),
            "an artifact reply must be the canonical envelope, whatever features this build unified"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&spliced).expect("one JSON value"),
            tree,
            "and it must carry the values the Value route did"
        );
    }

    #[test]
    fn initialize_reports_tools_and_nothing_else() {
        let r = call("initialize", json!({}));
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(r["result"]["serverInfo"]["name"], "ethos-parser");
        let caps = &r["result"]["capabilities"];
        assert!(caps.get("tools").is_some());
        for surface in ["resources", "prompts", "sampling", "logging"] {
            assert!(
                caps.get(surface).is_none(),
                "`{surface}` would be a second surface owing the handle law"
            );
        }
    }

    /// **The handle law, checked against the wire rather than against memory.**
    ///
    /// A tool that grew a `bbox` argument "for convenience" would let a model author a locator the
    /// engine then trusts, which `docs/history/12-V12-SCOPE.md` §3 forbids and which no amount of prose in
    /// a description would prevent.
    #[test]
    fn no_tool_argument_names_a_coordinate() {
        let tools = tools();
        let tools = tools.as_array().expect("tools");
        // Same floor, same reason: the banned-name loop below never runs if the list is empty.
        assert_eq!(
            tools.len(),
            4,
            "{} tool(s) advertised, not four",
            tools.len()
        );
        let mut properties_checked = 0usize;
        for tool in tools {
            let schema = &tool["inputSchema"]["properties"];
            let names: Vec<&str> = schema
                .as_object()
                .expect("properties")
                .keys()
                .map(String::as_str)
                .collect();
            for banned in [
                "page", "bbox", "box", "rect", "x", "y", "w", "h", "width", "height", "row",
                "column", "col", "span", "offset", "coords", "region",
            ] {
                assert!(
                    !names.contains(&banned),
                    "tool `{}` takes `{banned}`, which lets the model author a locator",
                    tool["name"]
                );
            }
            properties_checked += names.len();
        }
        // And a tool advertising no properties at all would satisfy the loop above while
        // accepting anything the caller sent.
        assert!(
            properties_checked >= 4,
            "only {properties_checked} argument(s) across all tools; the schemas are empty, so \
             nothing above was actually checked"
        );
    }

    #[test]
    fn every_tool_refuses_unknown_arguments() {
        let tools = tools();
        let tools = tools.as_array().expect("tools");
        // A per-tool property asserted over an empty list is a test that passes having checked
        // nothing — the shape v2-S13.1 went looking for. Four tools are advertised: `extract`,
        // `ground`, `node_get` and — since v2.3, decision #30 — `locate`. A fifth is a decision,
        // and it arrives through this line.
        assert_eq!(
            tools.len(),
            4,
            "{} tool(s) advertised; four is the number this server has argued for, and the \
             per-tool assertions below check nothing at all if the list is short",
            tools.len()
        );
        for tool in tools {
            assert_eq!(
                tool["inputSchema"]["additionalProperties"],
                json!(false),
                "tool `{}` accepts extra arguments, which is where a locator sneaks in",
                tool["name"]
            );
        }
    }

    #[test]
    fn a_notification_gets_no_reply() {
        let line = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert!(
            handle_line(&line.to_string(), &mut ledger::Ledger::new()).is_none(),
            "answering a notification is a protocol error, not a courtesy"
        );
    }

    #[test]
    fn an_unparseable_line_is_an_error_rather_than_a_panic() {
        let r = render(handle_line("{not json", &mut ledger::Ledger::new()).expect("a reply"));
        assert_eq!(r["error"]["code"], -32700);
    }

    #[test]
    fn an_unknown_method_is_refused() {
        let r = call("tools/nonesuch", json!({}));
        assert_eq!(r["error"]["code"], METHOD_NOT_FOUND);
    }

    #[test]
    fn an_unknown_tool_is_refused() {
        let r = call("tools/call", json!({ "name": "delete_everything" }));
        assert!(r["error"]["message"].as_str().unwrap().contains("no tool"));
    }

    /// A forged handle is a **tool error**, not an empty result.
    #[test]
    fn a_node_id_from_a_representation_that_is_not_there_fails_closed() {
        let r = call(
            "tools/call",
            json!({
                "name": "node_get",
                "arguments": { "representation": {}, "node_id": "s-forged" }
            }),
        );
        // Malformed representation, so it never reaches the lookup — and it still fails closed.
        assert_eq!(r["result"]["isError"], json!(true));
    }

    // ---------------------------------------------------------------------------------------
    // The ledger. Every test below compares raw reply bytes with a fresh ledger's reply to the
    // same request at the same file state — the claim is that earlier calls change cost, never
    // content — and each names the wrong implementation it exists to catch.
    // ---------------------------------------------------------------------------------------

    /// The canonical artifact of an in-repo one-node PDF, minted here rather than read from disk.
    fn artifact() -> Vec<u8> {
        let pdf = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/engine/measured-ink-box/document.pdf"
        ))
        .expect("fixture");
        crate::representation_for_bytes(&pdf, &ethos_parser_core::Profile::default())
            .expect("extracts")
            .to_canonical_bytes()
            .expect("canonical")
    }

    /// The same length, the declared fingerprint kept, one letter of the node's text flipped — a
    /// payload edit `verify_fingerprint` refuses and nothing cheaper can see.
    fn tampered(bytes: &[u8]) -> Vec<u8> {
        let at = bytes
            .windows(8)
            .rposition(|w| w == b"\"text\":\"")
            .expect("a text field")
            + 8;
        let mut out = bytes.to_vec();
        assert!(
            out[at].is_ascii_alphabetic(),
            "the text starts with a letter"
        );
        out[at] ^= 0x20;
        out
    }

    /// A directory of its own per test, so parallel tests never share a file, removed when the test
    /// ends — including when it panics.
    struct Scratch(std::path::PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = std::path::Path;
        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn scratch(name: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("ethos-mcp-ledger-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Scratch(dir)
    }

    fn write(path: &std::path::Path, bytes: &[u8]) {
        std::fs::write(path, bytes).expect("write");
    }

    /// One tool call through the real line handler, as the wire bytes of its reply.
    fn tool(ledger: &mut ledger::Ledger, name: &str, arguments: Value) -> Vec<u8> {
        let line = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let mut out = Vec::new();
        handle_line(&line.to_string(), ledger)
            .expect("a reply")
            .write_to(&mut out)
            .expect("write");
        out
    }

    fn fresh(name: &str, arguments: Value) -> Vec<u8> {
        tool(&mut ledger::Ledger::new(), name, arguments)
    }

    fn is_error(reply: &[u8]) -> bool {
        serde_json::from_slice::<Value>(reply).expect("json")["result"]["isError"] == json!(true)
    }

    fn at(path: &std::path::Path) -> Value {
        json!(path.to_str().expect("utf-8 path"))
    }

    /// The failure this catches: a ledger that never records, which would make every equality below vacuous; a hit
    /// that skips the parse and with it `check_structure`; a key taken from anything but a hash of
    /// this call's bytes.
    #[test]
    fn a_path_is_verified_once_then_answered_identically() {
        let dir = scratch("once");
        let p = dir.join("a.json");
        write(&p, &artifact());
        let mut ledger = ledger::Ledger::new();
        for _ in 0..3 {
            let args = json!({ "representation": at(&p), "node_id": "s1" });
            let reply = tool(&mut ledger, "node_get", args.clone());
            assert!(!is_error(&reply));
            assert_eq!(reply, fresh("node_get", args));
        }
        for _ in 0..2 {
            let args = json!({ "representation": at(&p) });
            assert_eq!(
                tool(&mut ledger, "ground", args.clone()),
                fresh("ground", args)
            );
        }
        assert_eq!(
            ledger.stats,
            ledger::Stats {
                parses: 5,
                hashes: 5,
                verifies: 1,
                hits: 4
            }
        );
    }

    /// The failure this catches: skipping verification whenever a key of the same length exists, or
    /// whenever the ledger is not empty; a key over a prefix, the first and last blocks, or the
    /// declared fingerprint. The edit sits 256 KiB from the start and 1 MiB from the end, away from
    /// the middle, and the poisoned ledger proves the key is the whole test.
    #[test]
    fn the_skip_is_decided_by_the_bytes_alone() {
        let mut a = vec![b' '; 256 * 1024];
        a.extend_from_slice(&artifact());
        a.extend(std::iter::repeat_n(b' ', 1024 * 1024));
        let t = tampered(&a);
        assert_eq!(a.len(), t.len());

        assert!(
            ledger::Ledger::remembering_unverified(&t)
                .load(t.clone())
                .is_ok(),
            "a remembered key skips verification — the only thing that decides a skip"
        );
        let refused = ledger::Ledger::remembering_unverified(&a)
            .load(t.clone())
            .expect_err("different bytes are verified, whatever else is remembered");
        let fresh_refusal = ledger::Ledger::new()
            .load(t.clone())
            .expect_err("and refused");
        assert!(refused
            .message
            .contains("declared representation_c14n_sha256 is"));
        assert_eq!(refused.message, fresh_refusal.message);

        let mut warmed = ledger::Ledger::new();
        assert!(warmed.load(a).is_ok());
        assert!(
            warmed.load(t).is_err(),
            "a verified original does not vouch for its edit"
        );
    }

    /// The failure this catches: a key made of the path, the inode, the size or the modification time. The file is
    /// rewritten in place — same inode, same length — and its mtime put back.
    #[test]
    fn a_same_length_rewrite_with_its_mtime_restored_is_verified_again() {
        let dir = scratch("rewrite");
        let p = dir.join("a.json");
        let good = artifact();
        write(&p, &good);
        let args = json!({ "representation": at(&p), "node_id": "s1" });
        let mut ledger = ledger::Ledger::new();
        assert!(!is_error(&tool(&mut ledger, "node_get", args.clone())));

        let before = std::fs::metadata(&p).expect("meta");
        {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new()
                .write(true)
                .open(&p)
                .expect("open");
            f.write_all(&tampered(&good)).expect("rewrite in place");
            f.set_modified(before.modified().expect("mtime"))
                .expect("restore mtime");
        }
        let after = std::fs::metadata(&p).expect("meta");
        assert_eq!(before.len(), after.len());
        assert_eq!(before.modified().ok(), after.modified().ok());

        let reply = tool(&mut ledger, "node_get", args.clone());
        assert!(
            is_error(&reply),
            "the edited bytes must be verified, and refused"
        );
        assert_eq!(reply, fresh("node_get", args.clone()));
        assert_eq!(ledger.stats.verifies, 2);

        write(&p, &good);
        assert!(!is_error(&tool(&mut ledger, "node_get", args)));
        assert_eq!(ledger.stats.hits, 1, "the original bytes are still known");
    }

    /// The failure this catches: a change-time or inode key no timestamp test can reach; a second read or `stat`
    /// inside the ledger; a key hashed from a `Value`; a second insertion site, such as one before
    /// verification. Comment lines are skipped, so the module may explain what it does not do.
    #[test]
    fn the_ledger_never_sees_a_path_or_the_filesystem() {
        let src = include_str!("mcp.rs");
        let start = src.find("mod ledger {").expect("the module");
        let end = src.find("// end mod ledger").expect("its end marker");
        let code: String = src[start..end]
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for banned in [
            "std::fs",
            "read_source",
            "Path",
            "metadata",
            "File",
            "modified",
            "Value",
            "to_string(",
            "serde_json::to_",
        ] {
            assert!(
                !code.contains(banned),
                "the ledger's code mentions `{banned}`"
            );
        }
        assert_eq!(
            code.matches("insert(").count(),
            2,
            "one definition and one call: an entry is made in exactly one place"
        );
        assert_eq!(
            code.matches("push_back(").count(),
            4,
            "a hit moved to the back, `insert`, `carried` and the test-only \
             `remembering_unverified`: a key enters nowhere else"
        );
        let load = &code[code.find("fn load(").expect("load")..];
        assert!(
            load.find("from_slice(").expect("a parse") < load.find("key_of(").expect("a hash"),
            "`load` must parse before it hashes"
        );
    }

    /// The failure this catches: hashing before parsing, which would make a mistaken path to a large PDF wait for
    /// a SHA-256 of it; remembering a failure.
    #[test]
    fn a_parse_failure_is_neither_hashed_nor_remembered() {
        let dir = scratch("parse");
        let good = artifact();
        let pdf = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/engine/measured-ink-box/document.pdf"
        );
        let half = dir.join("half.json");
        write(&half, &good[..good.len() / 2]);
        let other = dir.join("other.json");
        write(&other, br#"{"nope":true}"#);
        let mut ledger = ledger::Ledger::new();
        for path in [half.as_path(), other.as_path(), std::path::Path::new(pdf)] {
            let args = json!({ "representation": at(path), "node_id": "s1" });
            let reply = tool(&mut ledger, "node_get", args.clone());
            assert!(is_error(&reply));
            assert_eq!(reply, fresh("node_get", args));
        }
        assert_eq!(ledger.stats.hashes, 0);
        assert_eq!(ledger.len(), 0);
    }

    /// The failure this catches: an insertion before, or regardless of, verification; a remembered refusal replayed
    /// after the file is fixed.
    #[test]
    fn a_verification_failure_is_never_remembered() {
        let dir = scratch("refused");
        let p = dir.join("a.json");
        let good = artifact();
        let bad = tampered(&good);
        write(&p, &bad);
        let args = json!({ "representation": at(&p), "node_id": "s1" });
        let mut ledger = ledger::Ledger::new();
        let first = tool(&mut ledger, "node_get", args.clone());
        assert!(is_error(&first));
        assert_eq!(first, tool(&mut ledger, "node_get", args.clone()));
        assert_eq!(ledger.stats.verifies, 2);
        assert!(!ledger.knows(&bad));

        write(&p, &good);
        assert!(!is_error(&tool(&mut ledger, "node_get", args)));
        assert_eq!(ledger.stats.verifies, 3);
    }

    /// The failure this catches: an inline route keyed on a `Value`'s serialization — a build property under
    /// `preserve_order` — or inline input seeding, or being answered from, a path's entry.
    #[test]
    fn inline_never_consults_or_fills_the_ledger() {
        let dir = scratch("inline");
        let p = dir.join("a.json");
        let good = artifact();
        write(&p, &good);
        let inline: Value = serde_json::from_slice(&good).expect("json");
        let tampered_inline: Value = serde_json::from_slice(&tampered(&good)).expect("json");

        let mut ledger = ledger::Ledger::new();
        tool(
            &mut ledger,
            "node_get",
            json!({ "representation": at(&p), "node_id": "s1" }),
        );
        let stats = ledger.stats;
        let args = json!({ "representation": tampered_inline, "node_id": "s1" });
        assert!(is_error(&tool(&mut ledger, "node_get", args)));
        let args = json!({ "representation": inline.clone(), "node_id": "s1" });
        assert_eq!(
            tool(&mut ledger, "node_get", args.clone()),
            fresh("node_get", args)
        );
        assert_eq!(ledger.stats, stats);
        assert_eq!(ledger.len(), 1);

        let mut ledger = ledger::Ledger::new();
        tool(
            &mut ledger,
            "node_get",
            json!({ "representation": inline, "node_id": "s1" }),
        );
        tool(
            &mut ledger,
            "node_get",
            json!({ "representation": at(&p), "node_id": "s1" }),
        );
        assert_eq!((ledger.stats.verifies, ledger.stats.hits), (1, 0));
    }

    /// The failure this catches: a reply that starts naming the path, which would make sharing by bytes unsound; a
    /// key that quietly includes the path.
    #[cfg(unix)]
    #[test]
    fn identical_bytes_share_verification_whatever_the_path() {
        let dir = scratch("aliases");
        let good = artifact();
        let p = dir.join("a.json");
        write(&p, &good);
        let copy = dir.join("copy.json");
        write(&copy, &good);
        let link = dir.join("link.json");
        std::os::unix::fs::symlink(&p, &link).expect("symlink");
        let hard = dir.join("hard.json");
        std::fs::hard_link(&p, &hard).expect("hard link");

        let mut ledger = ledger::Ledger::new();
        let mut replies = Vec::new();
        for path in [&p, &link, &hard, &copy] {
            for id in ["s1", "s-forged"] {
                let args = json!({ "representation": at(path), "node_id": id });
                let reply = tool(&mut ledger, "node_get", args.clone());
                assert_eq!(reply, fresh("node_get", args));
                let text = String::from_utf8_lossy(&reply).into_owned();
                assert!(
                    !text.contains(dir.to_str().expect("utf-8")),
                    "a reply names the path"
                );
                replies.push((id, reply));
            }
        }
        assert_eq!(ledger.stats.verifies, 1);
        for (id, reply) in &replies {
            assert_eq!(
                reply,
                &replies.iter().find(|(i, _)| i == id).expect("first").1
            );
        }
    }

    /// The failure this catches: a normalizing key — a hash of re-serialized or canonicalized content — which would
    /// couple the gate's build to the release build and cost a walk the size of verification.
    #[test]
    fn whitespace_and_formatting_are_part_of_the_key() {
        let dir = scratch("format");
        let good = artifact();
        let mut newline = good.clone();
        newline.push(b'\n');
        let pretty =
            serde_json::to_vec_pretty(&serde_json::from_slice::<Value>(&good).expect("json"))
                .expect("pretty");
        let reference = fresh(
            "node_get",
            json!({ "representation": at(&{
            let p = dir.join("canonical.json");
            write(&p, &good);
            p
        }), "node_id": "s1" }),
        );
        let mut ledger = ledger::Ledger::new();
        for (name, bytes) in [
            ("canonical.json", &good),
            ("newline.json", &newline),
            ("pretty.json", &pretty),
        ] {
            let p = dir.join(name);
            write(&p, bytes);
            let reply = tool(
                &mut ledger,
                "node_get",
                json!({ "representation": at(&p), "node_id": "s1" }),
            );
            assert_eq!(reply, reference);
        }
        assert_eq!(ledger.stats.verifies, 3);
    }

    /// The failure this catches: first-in-first-out eviction, which forgets a key in use; growth without a bound.
    #[test]
    fn the_ledger_is_bounded_and_least_recently_used() {
        let good = artifact();
        let variant = |k: usize| {
            let mut v = good.clone();
            v.extend(std::iter::repeat_n(b' ', k));
            v
        };
        let mut ledger = ledger::Ledger::new();
        for k in 0..ledger::CAPACITY {
            ledger.load(variant(k)).expect("verifies");
        }
        assert!(ledger.load(variant(0)).is_ok());
        assert_eq!(ledger.stats.hits, 1);
        ledger.load(variant(ledger::CAPACITY)).expect("verifies");
        assert_eq!(ledger.len(), ledger::CAPACITY);
        let verifies = ledger.stats.verifies;
        ledger.load(variant(0)).expect("still known");
        assert_eq!(
            ledger.stats.verifies, verifies,
            "the recently used key survived eviction"
        );
        ledger.load(variant(1)).expect("verifies again");
        assert_eq!(
            ledger.stats.verifies,
            verifies + 1,
            "the least recently used key was evicted"
        );
    }

    /// The failure this catches: the `node_id` check hoisted above the load, or a hit that returns before the
    /// arguments are validated.
    #[test]
    fn error_precedence_is_unchanged_on_a_warm_ledger() {
        let dir = scratch("precedence");
        let good = artifact();
        let p = dir.join("a.json");
        let t = dir.join("t.json");
        write(&p, &good);
        write(&t, &tampered(&good));
        let mut ledger = ledger::Ledger::new();
        tool(
            &mut ledger,
            "node_get",
            json!({ "representation": at(&p), "node_id": "s1" }),
        );
        for (args, says) in [
            (
                json!({ "representation": at(&t) }),
                "declared representation_c14n_sha256",
            ),
            (json!({ "representation": at(&p) }), "`node_id` is required"),
            (
                json!({ "representation": at(&p), "node_id": 7 }),
                "`node_id` is required",
            ),
        ] {
            let reply = tool(&mut ledger, "node_get", args.clone());
            assert!(is_error(&reply));
            assert!(
                String::from_utf8_lossy(&reply).contains(says),
                "expected `{says}`"
            );
            assert_eq!(reply, fresh("node_get", args));
        }
    }

    /// The failure this catches: recording, hashing or rewording around a read error.
    #[test]
    fn a_read_error_is_todays_error_and_touches_nothing() {
        let dir = scratch("unreadable");
        let mut ledger = ledger::Ledger::new();
        for path in [dir.join("missing.json"), dir.to_path_buf()] {
            let args = json!({ "representation": at(&path), "node_id": "s1" });
            let reply = tool(&mut ledger, "node_get", args.clone());
            assert!(is_error(&reply));
            let text = serde_json::from_slice::<Value>(&reply).expect("json")["result"]["content"]
                [0]["text"]
                .as_str()
                .expect("text")
                .to_string();
            assert!(text.starts_with(path.to_str().expect("utf-8")), "{text}");
            assert_eq!(reply, fresh("node_get", args));
        }
        assert_eq!(ledger.stats, ledger::Stats::default());
    }

    /// One line through the entry its process runs, [`serve_call`], and back through [`relay`] and
    /// [`settle`]: everything a line's process does, done in this one.
    fn in_process(
        carried: &str,
        line: &str,
        out: &mut dyn Write,
    ) -> std::io::Result<Option<String>> {
        let mut written = Vec::new();
        serve_call(
            std::io::Cursor::new(format!("{carried}\n{line}\n")),
            &mut written,
        )?;
        let relayed = relay(&mut std::io::Cursor::new(written), out)?;
        settle(line, relayed, true, "exit status: 0", out)
    }

    /// The failure this catches: a ledger line dropped between one line and the next, or made anew
    /// per line, which every test handing in its own ledger would miss; a failed line's ledger line
    /// kept; a blank line sent to a process; a line trimmed otherwise than `BufRead::lines` trims
    /// it — a CRLF's carriage return kept, or a last line without a newline lost or cut.
    #[test]
    fn serve_carries_each_lines_ledger_to_the_next() {
        let mut seen = Vec::new();
        let mut out = Vec::new();
        serve_with(
            std::io::Cursor::new("a\r\n\n  \nb\nc\nd\ne\r"),
            &mut out,
            |carried, line, out| {
                seen.push(carried.to_string());
                writeln!(out, "{line}")?;
                Ok((line != "b").then(|| format!("after {line}")))
            },
        )
        .expect("serve");
        assert_eq!(seen, ["", "after a", "after a", "after c", "after d"]);
        assert_eq!(out, b"a\nb\nc\nd\ne\r\n");
    }

    /// The failure this catches: a reply the relay changes, a ledger that does not reach the next
    /// line, and an answer that depends on an earlier line.
    #[test]
    fn a_session_answers_as_fresh_sessions_do_and_carries_its_ledger() {
        let dir = scratch("session");
        let p = dir.join("a.json");
        let good = artifact();
        write(&p, &good);
        let call = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "node_get", "arguments": { "representation": at(&p), "node_id": "s1" } }
        });
        let ping = json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" });
        let input = format!("{call}\n{ping}\n{call}\n");
        let mut carried = Vec::new();
        let mut out = Vec::new();
        serve_with(std::io::Cursor::new(input), &mut out, |c, line, o| {
            let next = in_process(c, line, o)?;
            carried.push(next.clone());
            Ok(next)
        })
        .expect("serve");
        for (i, line) in carried.iter().enumerate() {
            let ledger = ledger::Ledger::carried(line.as_deref().expect("every line carries on"));
            assert!(ledger.knows(&good), "after line {i}");
            assert_eq!(ledger.len(), 1, "after line {i}");
        }

        let mut expected = Vec::new();
        for line in [&call, &ping, &call] {
            serve_with(
                std::io::Cursor::new(format!("{line}\n")),
                &mut expected,
                in_process,
            )
            .expect("serve");
        }
        assert_eq!(
            out, expected,
            "a session's replies are three fresh servers' replies"
        );
    }

    /// **A carried ledger decides a skip as this process's own does, and only one this version
    /// wrote.**
    ///
    /// The failure this catches: a line's process that ignores the ledger it is handed, which no
    /// reply shows, because a skip changes none; a carried line that drops a key or reorders them;
    /// one version taking another's verdicts; a carried line past the bound taken.
    #[test]
    fn a_carried_ledger_decides_the_skip_and_only_at_this_version() {
        let dir = scratch("carried");
        let p = dir.join("a.json");
        let bad = tampered(&artifact());
        write(&p, &bad);
        let line = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "node_get", "arguments": { "representation": at(&p), "node_id": "s1" } }
        });
        let refused = |carried: &str| {
            let mut out = Vec::new();
            serve_call(
                std::io::Cursor::new(format!("{carried}\n{line}\n")),
                &mut out,
            )
            .expect("serve_call");
            is_error(out.split(|&b| b == b'\n').nth(1).expect("a reply"))
        };
        let poisoned = ledger::Ledger::remembering_unverified(&bad).carry();
        assert!(!refused(&poisoned), "a carried key skips verification");
        assert!(refused(""), "no ledger: verified, and refused");
        let elsewhere = poisoned.replacen(env!("CARGO_PKG_VERSION"), "0.0.0", 1);
        assert!(refused(&elsewhere), "another version's ledger is not taken");

        let good = artifact();
        let variant = |k: usize| {
            let mut v = good.clone();
            v.extend(std::iter::repeat_n(b' ', k));
            v
        };
        let mut ledger = ledger::Ledger::new();
        for k in 0..ledger::CAPACITY {
            ledger.load(variant(k)).expect("verifies");
        }
        ledger
            .load(variant(0))
            .expect("a hit, now most recently used");
        let mut carried = ledger::Ledger::carried(&ledger.carry());
        assert_eq!(carried.len(), ledger::CAPACITY);
        carried.load(variant(ledger::CAPACITY)).expect("verifies");
        let before = carried.stats;
        carried.load(variant(0)).expect("still known");
        assert_eq!(
            carried.stats.hits,
            before.hits + 1,
            "the most recently used key survived the carry and the eviction"
        );
        carried.load(variant(1)).expect("verifies again");
        assert_eq!(
            carried.stats.verifies,
            before.verifies + 1,
            "the least recently used key left first"
        );

        let over = format!("{} 1:{}", ledger.carry(), "0".repeat(64));
        assert_eq!(ledger::Ledger::carried(&over).len(), 0);
        let malformed = format!("{} 1:{} x", env!("CARGO_PKG_VERSION"), "0".repeat(64));
        assert_eq!(ledger::Ledger::carried(&malformed).len(), 0);
    }

    /// The failure this catches: a ledger line taken from anywhere but the first line, or a first
    /// line past the limit copied to the host as if it were a reply.
    #[test]
    fn relay_takes_the_first_line_and_copies_the_rest() {
        let mut out = Vec::new();
        let relayed = relay(
            &mut std::io::Cursor::new(b"0.1 5:ab\n{\"id\":1}\n".to_vec()),
            &mut out,
        )
        .expect("relay");
        assert_eq!(relayed.carried.as_deref(), Some("0.1 5:ab"));
        assert!(relayed.forwarded && relayed.ended);
        assert_eq!(out, b"{\"id\":1}\n");

        let mut out = Vec::new();
        let mut long = vec![b'x'; usize::try_from(CARRIED_LIMIT).expect("small") + 1];
        long.extend_from_slice(b"\n{\"id\":1}\n");
        let relayed = relay(&mut std::io::Cursor::new(long), &mut out).expect("relay");
        assert!(relayed.carried.is_none() && !relayed.forwarded);
        assert!(out.is_empty());
    }

    /// **A line whose process fails is answered once, under its own id, and carries no ledger
    /// on.**
    ///
    /// The failure this catches: a failed line left unanswered, answered twice or under another id,
    /// or its answer run into the reply it cut short; a notification answered; a failed process's
    /// ledger line kept.
    #[test]
    fn a_failed_line_is_answered_once_under_its_own_id() {
        let line = r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#;
        let settled = |carried: Option<&str>, forwarded, ended, clean, line: &str| {
            let relayed = Relayed {
                carried: carried.map(String::from),
                forwarded,
                ended,
            };
            let mut out = Vec::new();
            let next =
                settle(line, relayed, clean, "signal: 6 (SIGABRT)", &mut out).expect("settle");
            (next, String::from_utf8(out).expect("utf-8"))
        };
        let error_for_7 = |reply: &str| {
            let v: Value = serde_json::from_str(reply).expect("a reply");
            v["id"] == json!(7) && v["error"]["code"] == json!(INTERNAL_ERROR)
        };

        assert_eq!(
            settled(Some("L"), true, true, true, line),
            (Some("L".to_string()), String::new())
        );
        let (next, out) = settled(Some("L"), false, false, false, line);
        assert_eq!(next, None);
        assert!(out.ends_with('\n') && out.lines().count() == 1 && error_for_7(out.trim_end()));
        let (next, out) = settled(Some("L"), true, false, false, line);
        assert_eq!(next, None);
        let lines: Vec<&str> = out.lines().collect();
        assert!(
            lines.len() == 2 && lines[0].is_empty() && error_for_7(lines[1]),
            "{out}"
        );
        assert_eq!(
            settled(Some("L"), true, true, false, line),
            (None, String::new())
        );
        let (next, out) = settled(None, false, false, true, line);
        assert!(next.is_none() && error_for_7(out.trim_end()));
        let notification = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert_eq!(
            settled(None, false, false, false, notification),
            (None, String::new())
        );
    }

    /// **The whole `ground` sentence, every clause firing, in the order a reader meets them.**
    ///
    /// The SDK adapters return this text verbatim, so this is where its wording is pinned — and the
    /// only place G1's clause can be, since firing it for real takes a million spans. Built on a real
    /// projection with its declarations set, so the base counts are a projection's own.
    #[test]
    fn the_ground_summary_names_every_declaration_in_order() {
        let pdf = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/engine/markdown-two-blocks/document.pdf"
        ))
        .expect("fixture");
        let repr = crate::representation_for_bytes(&pdf, &ethos_parser_core::Profile::default())
            .expect("extracts");
        let base = ethos_parser_grounding::project(&repr).expect("projects");
        assert_eq!(
            ground_summary(&base),
            "2 element(s) with a measured box; 0 omitted for having none.",
            "nothing fired, so no clause and no trailing space"
        );

        let every = ethos_parser_grounding::Projection {
            spans_withheld: Some(ethos_parser_grounding::SpansWithheld {
                spans: 1_000_001,
                limit: 1_000_000,
                limitation_code: ethos_parser_grounding::SPANS_WITHHELD_OVER_LIMIT,
            }),
            elements_omitted: Some(ethos_parser_grounding::ElementsOmitted {
                elements: 3,
                spans: 4,
                text_limit: 16_384,
                locator_limit: 2_048,
                limitation_code: ethos_parser_grounding::ELEMENTS_OMITTED_OVER_LIMIT,
            }),
            tables_withheld: Some(ethos_parser_grounding::TablesWithheld {
                tables: 5,
                table_limit: 100_000,
                oversized_cells: 1,
                oversized_grids: 0,
                string_limit: 16_384,
                limitation_code: ethos_parser_grounding::TABLES_WITHHELD_OVER_LIMIT,
            }),
            ..base
        };
        assert_eq!(
            ground_summary(&every),
            "2 element(s) with a measured box; 0 omitted for having none. 1000001 span(s) \
             withheld, more than the 1000000 the schema admits: elements only. 3 element(s) \
             omitted, a string over the schema's byte limit. 5 table(s) withheld, over the \
             schema's limits: no tables."
        );
    }
}
