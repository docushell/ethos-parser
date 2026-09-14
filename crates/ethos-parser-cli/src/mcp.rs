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
//!    `ethos-parser ground` checks it — in this call, or in an earlier call of this process over
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
//! exact bytes were verified earlier in this process. It needs the full preimage and reveals no
//! locator; the only way to close it is to verify every time, which is the cost this removes.
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
//! verdict core already reached, per process — and it stays here on purpose: a core entry point
//! that skips verification for a digest would be the fingerprint-accepting constructor core
//! refuses, and it is sound only inside the process that did the verifying.

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

/// Run the server until stdin closes.
///
/// One JSON object per line. A line that is not JSON, or is JSON this server has no method for,
/// gets a JSON-RPC error rather than a panic or a silent skip — a host that mis-frames a request
/// should learn that from the response, not from a closed pipe.
pub fn serve(input: impl BufRead, output: impl Write) -> std::io::Result<()> {
    serve_with(input, output, &mut ledger::Ledger::new())
}

/// [`serve`], with the session's [`ledger::Ledger`] supplied, so a test can inspect it afterwards.
fn serve_with(
    input: impl BufRead,
    mut output: impl Write,
    ledger: &mut ledger::Ledger,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Some(response) = handle_line(&line, ledger) else {
            // A notification (no `id`) takes no reply, which is JSON-RPC's rule rather than a
            // shortcut: answering `notifications/initialized` is a protocol error.
            continue;
        };
        response.write_to(&mut output)?;
        writeln!(output)?;
        output.flush()?;
    }
    Ok(())
}

/// One line in, at most one response out.
fn handle_line(line: &str, ledger: &mut ledger::Ledger) -> Option<Reply> {
    let request: Value = match serde_json::from_str(line) {
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

    let id = request.get("id").cloned();
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    // A notification has no `id`. It is acted on and not answered.
    let id = id?;

    Some(match dispatch(method, &params, ledger) {
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

fn dispatch(method: &str, params: &Value, ledger: &mut ledger::Ledger) -> Result<Outcome, Failure> {
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
                "Read a PDF and return `DocumentRepresentation v0` — the canonical evidence \
                 record. Byte-identical to `ethos-parser extract`. Every locator a later call needs is \
                 minted here; pass this artifact back rather than composing one.",
            "inputSchema": {
                "type": "object",
                "additionalProperties": false,
                "required": ["path"],
                "properties": {
                    "path": { "type": "string", "description": "Path to a PDF file." }
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
        }
    ])
}

fn call_tool(params: &Value, ledger: &mut ledger::Ledger) -> Result<Outcome, Failure> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`name` is required"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let outcome = match name {
        "extract" => tool_extract(&args),
        "ground" => tool_ground(&args, ledger),
        "node_get" => tool_node_get(&args, ledger),
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
    let head = crate::read_source(std::path::Path::new(path)).map_err(|e| Failure::from(&e))?;
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
fn tool_ground(args: &Value, ledger: &mut ledger::Ledger) -> Result<(String, Artifact), Failure> {
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
fn tool_node_get(args: &Value, ledger: &mut ledger::Ledger) -> Result<(String, Artifact), Failure> {
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

/// Read the `representation` argument and **re-validate it before anything reads it**.
///
/// Accepts the artifact inline or as a path to bytes this engine wrote — both are the same
/// artifact and both get the same check. `verify_fingerprint` is the whole point: a representation
/// whose payload does not hash to its declared digest is not a record this engine will speak for,
/// so a model that edited the JSON on the way through fails here instead of getting an answer
/// about a document that never existed.
///
/// **A path is read once and its bytes handed to the ledger by value**, which parses, hashes and —
/// unless these exact bytes already verified in this process — verifies that one buffer. The
/// ledger is never given the path. An inline object is verified every time, exactly as before.
fn representation_arg(
    args: &Value,
    ledger: &mut ledger::Ledger,
) -> Result<DocumentRepresentation, Failure> {
    let raw = args
        .get("representation")
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`representation` is required"))?;

    match raw {
        Value::String(path) => {
            let bytes = crate::read_source(std::path::Path::new(path))
                .map_err(|e| Failure::new(INVALID_PARAMS, format!("{path}: {e}")))?;
            ledger.load(bytes)
        }
        other => {
            let repr: DocumentRepresentation = serde_json::from_value(other.clone())
                .map_err(|e| Failure::new(INVALID_PARAMS, format!("representation: {e}")))?;
            repr.verify_fingerprint().map_err(|e| Failure::from(&e))?;
            Ok(repr)
        }
    }
}

/// **Which exact bytes already verified in this process** — digests, and nothing else.
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
    /// No `Default` and no `Clone`, and one constructor: a key exists only because some buffer was
    /// hashed.
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
            3,
            "{} tool(s) advertised, not three",
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
        // nothing — the shape v2-S13.1 went looking for. Three tools are advertised: `extract`,
        // `ground` and `node_get`. A fourth is a decision, and it arrives through this line.
        assert_eq!(
            tools.len(),
            3,
            "{} tool(s) advertised; three is the number this server has argued for, and the \
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

    /// The failure this catches: a ledger made per line or per call inside the loop, which every test above — each
    /// handing in its own — would miss.
    #[test]
    fn serve_threads_one_ledger_through_the_session() {
        let dir = scratch("session");
        let p = dir.join("a.json");
        write(&p, &artifact());
        let call = json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": "node_get", "arguments": { "representation": at(&p), "node_id": "s1" } }
        });
        let ping = json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" });
        let input = format!("{call}\n{ping}\n{call}\n");
        let mut out = Vec::new();
        let mut ledger = ledger::Ledger::new();
        serve_with(std::io::Cursor::new(input), &mut out, &mut ledger).expect("serve");
        assert_eq!((ledger.stats.verifies, ledger.stats.hits), (1, 1));

        let mut expected = Vec::new();
        for line in [&call, &ping, &call] {
            serve_with(
                std::io::Cursor::new(format!("{line}\n")),
                &mut expected,
                &mut ledger::Ledger::new(),
            )
            .expect("serve");
        }
        assert_eq!(
            out, expected,
            "a session's replies are three fresh servers' replies"
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
