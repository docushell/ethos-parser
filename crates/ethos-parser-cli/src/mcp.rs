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
//!    `ethos-parser ground` checks it, and a node id is looked up among **that artifact's** own nodes.
//!
//! **A handle this engine did not mint fails closed** — a tool error, never an empty result. An
//! empty result tells a model its guess was unlucky; an error tells it the guess was not
//! admissible.
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
//! fifth crate or a new concept in `ethos-parser-core`.

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
pub fn serve(input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Some(response) = handle_line(&line) else {
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
fn handle_line(line: &str) -> Option<Reply> {
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

    Some(match dispatch(method, &params) {
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

fn dispatch(method: &str, params: &Value) -> Result<Outcome, Failure> {
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
        "tools/call" => call_tool(params),
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

fn call_tool(params: &Value) -> Result<Outcome, Failure> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`name` is required"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    let outcome = match name {
        "extract" => tool_extract(&args),
        "ground" => tool_ground(&args),
        "node_get" => tool_node_get(&args),
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
fn tool_ground(args: &Value) -> Result<(String, Artifact), Failure> {
    let repr = representation_arg(args)?;
    let projection = ethos_parser_grounding::project(&repr).map_err(|e| Failure::from(&e))?;
    let bytes = ethos_parser_grounding::to_canonical_bytes(&projection.source)
        .map_err(|e| Failure::from(&e))?;
    let mut summary = format!(
        "{} element(s) with a measured box; {} omitted for having none.",
        projection.source.elements.len(),
        projection.omission.nodes_omitted
    );
    if let Some(w) = projection.spans_withheld {
        summary.push_str(&format!(
            " {} span(s) withheld, more than the {} the schema admits: elements only.",
            w.spans, w.limit
        ));
    }
    Ok((summary, Artifact::Bytes(bytes)))
}

/// **The handle law, made mechanical.**
///
/// The representation is re-validated (fingerprint) and the id is looked up among *that*
/// artifact's own nodes. Not found is an error, deliberately: an empty result would tell a model
/// its guess was merely unlucky.
fn tool_node_get(args: &Value) -> Result<(String, Artifact), Failure> {
    let repr = representation_arg(args)?;
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
fn representation_arg(args: &Value) -> Result<DocumentRepresentation, Failure> {
    let raw = args
        .get("representation")
        .ok_or_else(|| Failure::new(INVALID_PARAMS, "`representation` is required"))?;

    let repr: DocumentRepresentation = match raw {
        Value::String(path) => {
            let bytes = crate::read_source(std::path::Path::new(path))
                .map_err(|e| Failure::new(INVALID_PARAMS, format!("{path}: {e}")))?;
            serde_json::from_slice(&bytes)
        }
        other => serde_json::from_value(other.clone()),
    }
    .map_err(|e| Failure::new(INVALID_PARAMS, format!("representation: {e}")))?;

    repr.verify_fingerprint().map_err(|e| Failure::from(&e))?;
    Ok(repr)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(method: &str, params: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        render(handle_line(&line.to_string()).expect("a request with an id gets a response"))
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
            handle_line(&line.to_string()).is_none(),
            "answering a notification is a protocol error, not a courtesy"
        );
    }

    #[test]
    fn an_unparseable_line_is_an_error_rather_than_a_panic() {
        let r = render(handle_line("{not json").expect("a reply"));
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
}
