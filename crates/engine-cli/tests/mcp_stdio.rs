// Copyright 2026 The ethos-engine maintainers
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

//! `engine mcp` — the first adapter, driven the way a host drives it (v1.2-S1).
//!
//! # What this suite exists to establish
//!
//! The version gate is not "the protocol works". It is **the handle law**, from
//! `docs/12-V12-SCOPE.md` §3 and memo §16.7: the engine mints every locator, hands it back as an
//! opaque handle, and re-validates it on the way in — and a handle it did not mint **fails
//! closed**.
//!
//! [`a_minted_handle_round_trips_and_a_forged_one_fails_closed`] is that sentence as an
//! executable, run against the real binary over a real pipe.
//!
//! The other half is that an artifact does not change by travelling through an adapter:
//! [`the_artifact_through_mcp_is_the_artifact_the_cli_prints`] compares the bytes.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn conformance(rel: &str) -> PathBuf {
    let p = repo_root().join("../ethos/fixtures").join(rel);
    assert!(p.is_file(), "fixture missing: {}", p.display());
    p
}

/// Speak a session to `engine mcp` over a real pipe and collect one response per line.
///
/// A spawned process rather than a call into the module: the thing under test is what a host gets,
/// and a host gets a subprocess reading stdin.
fn session(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engine"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the engine binary runs");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            writeln!(stdin, "{r}").expect("write request");
        }
    }

    let out = child.wait_with_output().expect("the server exits");
    assert_eq!(
        out.status.code(),
        Some(0),
        "the server must close cleanly: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("one JSON-RPC response per line"))
        .collect()
}

fn call(name: &str, arguments: Value, id: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": arguments }
    })
}

/// The representation `engine extract` prints, as parsed JSON.
fn cli_extract(pdf: &std::path::Path) -> (Vec<u8>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_engine"))
        .args(["extract", pdf.to_str().unwrap()])
        .output()
        .expect("engine extract runs");
    assert_eq!(out.status.code(), Some(0), "extract must succeed");
    let bytes = out
        .stdout
        .strip_suffix(b"\n")
        .unwrap_or(&out.stdout)
        .to_vec();
    let value = serde_json::from_slice(&bytes).expect("canonical JSON");
    (bytes, value)
}

// -------------------------------------------------------------------------------------------
// The protocol
// -------------------------------------------------------------------------------------------

/// A host's opening exchange: initialize, the notification that takes no reply, tools/list.
#[test]
fn the_server_completes_a_hosts_opening_handshake() {
    let responses = session(&[
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {} }),
        // No `id`. Answering it would be a protocol error, so the response count below is the
        // assertion that it was not answered.
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }),
        json!({ "jsonrpc": "2.0", "id": 3, "method": "ping", "params": {} }),
    ]);

    assert_eq!(
        responses.len(),
        3,
        "three requests carried an id; the notification must not be answered: {responses:?}"
    );

    let init = &responses[0]["result"];
    assert!(init["protocolVersion"].is_string());
    assert_eq!(init["serverInfo"]["name"], "ethos-engine");
    assert!(
        init["capabilities"]["tools"].is_object(),
        "tools is the only capability this server claims"
    );

    let names: Vec<&str> = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(names, vec!["extract", "ground", "node_get"]);
}

/// **No tool argument names a coordinate**, read off the wire rather than off the source.
///
/// The corollary of the handle law (`docs/12-V12-SCOPE.md` §3): a tool that grew a `bbox` argument
/// would let the model author a locator the engine then trusts, and no wording in a description
/// would stop it. The unit test in `mcp.rs` checks the table; this one checks what a host is
/// actually told.
#[test]
fn no_advertised_tool_argument_lets_the_model_author_a_locator() {
    let responses =
        session(&[json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} })]);
    let tools = responses[0]["result"]["tools"].as_array().expect("tools");
    assert!(!tools.is_empty(), "an empty list would pass vacuously");

    for tool in tools {
        let props = tool["inputSchema"]["properties"]
            .as_object()
            .expect("every tool declares its properties");
        for name in props.keys() {
            for banned in [
                "page", "bbox", "box", "rect", "x", "y", "w", "h", "width", "height", "row",
                "column", "col", "span", "region", "coords", "offset",
            ] {
                assert_ne!(
                    name.as_str(),
                    banned,
                    "tool `{}` advertises `{banned}`, which is a locator the model can compose",
                    tool["name"]
                );
            }
        }
        assert_eq!(
            tool["inputSchema"]["additionalProperties"],
            json!(false),
            "tool `{}` accepts extra arguments, which is where a locator sneaks in",
            tool["name"]
        );
    }
}

// -------------------------------------------------------------------------------------------
// The artifact
// -------------------------------------------------------------------------------------------

/// **An artifact does not change by travelling through an adapter.**
///
/// `structuredContent` is the canonical artifact, and it is compared against what `engine extract`
/// prints. A summary the model reads is a separate field, and it carries no locator.
#[test]
fn the_artifact_through_mcp_is_the_artifact_the_cli_prints() {
    let pdf = conformance("synthetic/simple-text/document.pdf");
    let (cli_bytes, cli_value) = cli_extract(&pdf);

    let responses = session(&[call("extract", json!({ "path": pdf.to_str().unwrap() }), 1)]);
    let result = &responses[0]["result"];
    assert_eq!(result["isError"], json!(false));
    assert_eq!(
        result["structuredContent"], cli_value,
        "the adapter must return the artifact, not a re-serialization of it"
    );

    // And re-canonicalizing what came back reproduces the CLI's bytes exactly — the check that
    // would catch a field reordered or an integer widened on the way through.
    let round_tripped = engine_core::c14n_bytes(&result["structuredContent"])
        .expect("the returned artifact canonicalizes");
    assert_eq!(
        round_tripped, cli_bytes,
        "byte identity across the adapter, not merely structural equality"
    );

    // **The summary is counts, and nothing a pipeline would bind to.**
    let content = result["content"][0]["text"].as_str().expect("a summary");
    for locator in ["bbox", "[", "x0", "origin", "sha256:", "s1"] {
        assert!(
            !content.contains(locator),
            "`{locator}` reached the model-facing summary: {content:?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The handle law — the version gate
// -------------------------------------------------------------------------------------------

/// **Mint, opaque, re-validate — and a handle this engine did not mint fails closed.**
///
/// The sentence memo §16.7 says decides whether MCP is the best host or the worst, run against the
/// real binary:
///
/// | handed to `node_get` | expected |
/// | --- | --- |
/// | a node id the engine minted, copied from the artifact | the node record |
/// | an id nothing minted | **error**, not an empty result |
/// | a representation whose text was edited on the way through | **error**, at the fingerprint |
///
/// The third row is the one a prompt could never enforce: a model that rewrites the artifact it
/// was handed does not get an answer about a document that never existed.
#[test]
fn a_minted_handle_round_trips_and_a_forged_one_fails_closed() {
    let pdf = conformance("synthetic/simple-text/document.pdf");
    let (_, repr) = cli_extract(&pdf);

    // The id is READ OFF the artifact rather than hardcoded. A hardcoded id that stopped existing
    // would make the forged-id half pass for the wrong reason.
    let minted = repr["representation"]["nodes"][0]["id"]
        .as_str()
        .expect("extract minted at least one node")
        .to_string();

    let mut tampered = repr.clone();
    tampered["representation"]["nodes"][0]["text"] = json!("Tampered");

    let responses = session(&[
        call(
            "node_get",
            json!({ "representation": repr, "node_id": minted }),
            1,
        ),
        call(
            "node_get",
            json!({ "representation": repr, "node_id": "s-forged" }),
            2,
        ),
        call(
            "node_get",
            json!({ "representation": tampered, "node_id": minted }),
            3,
        ),
    ]);

    // 1. The minted handle returns the node from THAT artifact.
    let ok = &responses[0]["result"];
    assert_eq!(ok["isError"], json!(false), "{ok:?}");
    assert_eq!(ok["structuredContent"]["id"], json!(minted));
    assert_eq!(
        ok["structuredContent"], repr["representation"]["nodes"][0],
        "the node record must be the one the artifact carries, verbatim"
    );

    // 2. A forged handle is an ERROR, not an empty result. An empty result would tell the model
    //    its guess was merely unlucky.
    let forged = &responses[1]["result"];
    assert_eq!(
        forged["isError"],
        json!(true),
        "a forged handle must fail closed: {forged:?}"
    );
    assert!(
        forged.get("structuredContent").is_none(),
        "a refusal carries no artifact, or the refusal is decorative"
    );
    let message = forged["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        message.contains("s-forged") && message.contains("not a node"),
        "the refusal must name what was refused: {message:?}"
    );

    // 3. An edited representation fails at the fingerprint, before any lookup happens.
    let tampered = &responses[2]["result"];
    assert_eq!(
        tampered["isError"],
        json!(true),
        "a representation that does not hash to its own digest is not a record this engine speaks \
         for: {tampered:?}"
    );
}

/// `ground` takes the artifact back — inline or as a path — and re-validates it the same way.
#[test]
fn ground_accepts_the_minted_artifact_and_checks_it() {
    let pdf = conformance("synthetic/two-lines/document.pdf");
    let (_, repr) = cli_extract(&pdf);

    let responses = session(&[
        call("ground", json!({ "representation": repr }), 1),
        call("ground", json!({ "representation": { "nope": true } }), 2),
    ]);

    let ok = &responses[0]["result"];
    assert_eq!(ok["isError"], json!(false), "{ok:?}");
    assert_eq!(
        ok["structuredContent"]["artifact_type"], "ethos.grounding.v1",
        "the projection is the one `engine ground` emits"
    );

    assert_eq!(
        responses[1]["result"]["isError"],
        json!(true),
        "a thing that is not a representation is refused rather than coerced"
    );
}

/// The relay boundary holds: no tool here emits a verdict, and `verify` is not among them.
///
/// `docs/07-VERIFY-BOUNDARY.md` — the engine invokes a verifier, it does not verify. Wrapping the
/// relay in a second protocol would be a second place for a verdict to be re-derived, so this
/// slice ships no `verify` tool at all.
#[test]
fn no_tool_emits_a_verdict_and_there_is_no_verify_tool() {
    let responses =
        session(&[json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {} })]);
    let tools = responses[0]["result"]["tools"].as_array().expect("tools");

    assert!(
        !tools.iter().any(|t| t["name"] == "verify"),
        "a `verify` tool would put a relay behind a second protocol"
    );

    let advertised = serde_json::to_string(&responses[0]["result"]).expect("serializes");
    for word in [
        "grounded",
        "evidence_tier",
        "verdict",
        "confidence",
        "trust_score",
    ] {
        assert!(
            !advertised.contains(word),
            "`{word}` appears in the advertised surface"
        );
    }
}
