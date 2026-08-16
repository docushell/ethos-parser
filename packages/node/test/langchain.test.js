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

/**
 * The LangChain tools — v1.2-S4, and the split is the whole slice.
 *
 * `docs/12-V12-SCOPE.md` §3: **locators live in the artifact, never in the prose a model reads and
 * edits.** In this framework that is `ToolMessage.artifact` versus `ToolMessage.content`, and
 * these tests are that sentence as executables.
 *
 * **MCP is the oracle here, not this file's own opinion.** `engine mcp` already decided both the
 * argument schemas and the summary wording, so the assertions below compare against what the
 * server actually advertises and emits rather than against strings retyped from it.
 *
 * **The peer is optional, so this file is the one that may not run.** `@langchain/core` is an
 * optional peer by design — the default import of this package pulls nothing — so when it is
 * absent these tests skip with the command that installs it. That is not the engine binary going
 * missing, which is always a hard failure; it is a declared-optional dependency being absent.
 */

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { FIXTURE_PDF, PACKAGE_ROOT, REPO_ROOT, clone, engineBinary } from "./helpers.js";
import { extract, ground, nodeGet } from "../src/index.js";

/** The same list `mcp.rs`, `mcp_stdio.rs`, `handle-law.test.js` and the Python suite ban. */
const BANNED_ARGUMENT_NAMES = [
  "page",
  "bbox",
  "box",
  "rect",
  "x",
  "y",
  "w",
  "h",
  "width",
  "height",
  "row",
  "column",
  "col",
  "span",
  "offset",
  "coords",
  "region",
];

/**
 * No tool result, summary or annotation may say this engine believes anything.
 * `docs/07-VERIFY-BOUNDARY.md`: it validates structure and binding, and never verifies a claim.
 */
const TRUST_STATE_WORDS = [
  "grounded",
  "verified",
  "evidence_tier",
  "trusted",
  "trust_score",
  "quality_score",
  "all_evidence_grounded",
];

let sdk = null;
let skip = false;
try {
  await import("@langchain/core/tools");
  sdk = await import("../src/langchain.js");
} catch {
  skip =
    "@langchain/core is not installed. It is an optional peer, so the default import of this " +
    "package pulls nothing. Install it to run these: npm install @langchain/core";
}

const options = skip ? { skip } : {};

/** Speak a session to `engine mcp` and return its results, so MCP can be the oracle. */
function mcp(calls) {
  const lines = calls.map(([method, params], i) =>
    JSON.stringify({ jsonrpc: "2.0", id: i + 1, method, params }),
  );
  const result = spawnSync(engineBinary, ["mcp"], {
    input: `${lines.join("\n")}\n`,
    encoding: "utf8",
    maxBuffer: Infinity,
  });
  assert.equal(result.status, 0, result.stderr);
  return result.stdout
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => JSON.parse(line).result);
}

function byName() {
  return Object.fromEntries(sdk.tools().map((tool) => [tool.name, tool]));
}

/** Invoke as a host does, so the return is a `ToolMessage` and not a bare tuple. */
function call(tool, args) {
  return tool.invoke({ name: tool.name, args, id: "call-1", type: "tool_call" });
}

// --- the split ------------------------------------------------------------------------------

test("the three tools are content_and_artifact", options, () => {
  const tools = sdk.tools();
  assert.deepEqual(
    tools.map((t) => t.name).sort(),
    ["extract", "ground", "node_get"],
  );
  for (const tool of tools) {
    assert.equal(
      tool.responseFormat,
      "content_and_artifact",
      "without this the artifact is stringified into `content`, which is a locator a model can edit and then cite",
    );
  }
});

test("extract puts the artifact in the artifact", options, async () => {
  const message = await call(byName().extract, { path: FIXTURE_PDF });
  assert.equal(message.constructor.name, "ToolMessage");
  assert.deepEqual(
    message.artifact,
    extract(FIXTURE_PDF),
    "the artifact must be the object the SDK returned, not a re-serialization of it",
  );
  assert.equal(typeof message.content, "string");
});

test("ground and node_get put the artifact in the artifact", options, async () => {
  const representation = extract(FIXTURE_PDF);
  const tools = byName();

  const grounding = await call(tools.ground, { representation });
  assert.deepEqual(grounding.artifact, ground(representation));
  assert.equal(grounding.artifact.artifact_type, "ethos.grounding.v1");

  const minted = representation.representation.nodes[0].id;
  const node = await call(tools.node_get, { representation, node_id: minted });
  assert.deepEqual(node.artifact, nodeGet(representation, minted));
  assert.equal(node.artifact.id, minted, "the id lives here, and only here");
});

// --- `content` carries nothing a pipeline would bind to -----------------------------------------

for (const name of ["markdown-two-blocks", "off-page-and-offset-box"]) {
  test(`the summaries are the ones MCP emits (${name})`, options, async () => {
    // Byte-for-byte against `engine mcp`, on a document where nothing is omitted and one where
    // everything is. This is what stops the second adapter inventing a richer sentence than the
    // first — and it is also the proof that `ground`'s omitted count, computed out here as
    // nodes-minus-elements, equals the engine's own `omission.nodes_omitted`.
    const pdf = join(REPO_ROOT, "fixtures", "engine", name, "document.pdf");
    const representation = extract(pdf);
    const tools = byName();

    const fromMcp = mcp([
      ["tools/call", { name: "extract", arguments: { path: pdf } }],
      ["tools/call", { name: "ground", arguments: { representation } }],
    ]);

    assert.equal(
      (await call(tools.extract, { path: pdf })).content,
      fromMcp[0].content[0].text,
    );
    assert.equal(
      (await call(tools.ground, { representation })).content,
      fromMcp[1].content[0].text,
    );
  });
}

test("no summary carries a locator", options, async () => {
  // The `mcp_stdio.rs` check, extended: the tokens that suite bans, plus the real minted id and
  // fingerprint read off this artifact rather than a hardcoded `s1`.
  const representation = extract(FIXTURE_PDF);
  const minted = representation.representation.nodes[0].id;
  const fingerprint = representation.representation_c14n_sha256;
  const tools = byName();

  const summaries = [
    (await call(tools.extract, { path: FIXTURE_PDF })).content,
    (await call(tools.ground, { representation })).content,
    (await call(tools.node_get, { representation, node_id: minted })).content,
  ];
  for (const summary of summaries) {
    for (const locator of ["bbox", "[", "x0", "origin", "sha256:", minted, fingerprint]) {
      assert.equal(
        summary.includes(locator),
        false,
        `\`${locator}\` reached the model-facing summary: ${JSON.stringify(summary)}`,
      );
    }
  }
});

test("node_get names the kind the artifact names", options, async () => {
  // The kind is a category, not a handle — and it is spelled the way the artifact spells it.
  // `engine mcp` prints Rust's `Debug` of the enum (`TextRun`); the artifact carries the serde
  // name (`text_run`). This adapter reports what the artifact says, because reshaping it into the
  // other spelling would be the adapter inventing a name for a thing it did not read.
  const representation = extract(FIXTURE_PDF);
  const minted = representation.representation.nodes[0].id;
  const message = await call(byName().node_get, { representation, node_id: minted });
  assert.equal(message.content, `1 node, kind \`${message.artifact.kind}\`.`);
});

test("no tool sets trust state", options, async () => {
  const representation = extract(FIXTURE_PDF);
  const minted = representation.representation.nodes[0].id;
  const tools = byName();

  const messages = [
    await call(tools.extract, { path: FIXTURE_PDF }),
    await call(tools.ground, { representation }),
    await call(tools.node_get, { representation, node_id: minted }),
  ];
  for (const message of messages) {
    const haystack = `${message.content} ${JSON.stringify(message.additional_kwargs ?? {})}`.toLowerCase();
    for (const word of TRUST_STATE_WORDS) {
      assert.equal(haystack.includes(word), false, `\`${word}\` appears on a tool result`);
    }
    assert.equal(message.status, "success", "a success is a success, not a degree of one");
  }

  for (const tool of Object.values(tools)) {
    const described = `${tool.description}${JSON.stringify(sdk.TOOL_SCHEMAS[tool.name])}`.toLowerCase();
    for (const word of ["confidence", "verdict", "evidence_tier", "trust_score"]) {
      assert.equal(described.includes(word), false);
    }
  }
});

// --- the handle law, through the framework ------------------------------------------------------

test("a forged handle is a tool error and not an empty artifact", options, async () => {
  // `12-V12-SCOPE.md` §3: fails closed means an error, not an empty answer. Throwing is how a tool
  // says that here — LangChain surfaces it as a tool failure, which is MCP's `isError: true` in
  // this framework's currency.
  const representation = extract(FIXTURE_PDF);
  await assert.rejects(() => call(byName().node_get, { representation, node_id: "s-forged" }), {
    name: "NodeNotFound",
  });
});

test("an edited representation fails at the fingerprint", options, async () => {
  const representation = extract(FIXTURE_PDF);
  const edited = clone(representation);
  edited.representation.nodes[0].text = "Tampered";
  const minted = representation.representation.nodes[0].id;
  await assert.rejects(() => call(byName().node_get, { representation: edited, node_id: minted }), {
    name: "FingerprintMismatch",
  });
});

test("a document the engine cannot read is a tool error", options, async () => {
  const directory = await mkdtemp(join(tmpdir(), "ethos-lc-test-"));
  try {
    const notAPdf = join(directory, "document.pdf");
    writeFileSync(notAPdf, "this is not a PDF");
    await assert.rejects(() => call(byName().extract, { path: notAPdf }), { name: "EngineFailed" });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

// --- one wire shape -------------------------------------------------------------------------------

test("the argument schemas are the ones MCP advertises", options, () => {
  // One wire shape across the three adapters, checked against the server rather than remembered.
  // `node_id` keeps MCP's spelling here even though this package's parameter is `nodeId`: the tool
  // argument is the wire, and one wire has one name.
  const advertised = mcp([["tools/list", {}]])[0].tools;
  assert.deepEqual(
    advertised.map((t) => t.name).sort(),
    Object.keys(sdk.TOOL_SCHEMAS).sort(),
  );
  for (const tool of advertised) {
    assert.deepEqual(
      sdk.TOOL_SCHEMAS[tool.name],
      tool.inputSchema,
      `the LangChain schema for \`${tool.name}\` has drifted from what \`engine mcp\` advertises`,
    );
  }
});

test("no tool argument names a coordinate", options, () => {
  for (const [name, schema] of Object.entries(sdk.TOOL_SCHEMAS)) {
    const properties = Object.keys(schema.properties);
    assert.ok(properties.length > 0, "an empty schema would pass vacuously");
    for (const banned of BANNED_ARGUMENT_NAMES) {
      assert.equal(
        properties.includes(banned),
        false,
        `tool \`${name}\` takes \`${banned}\`, which lets a model author a locator`,
      );
    }
    assert.equal(
      schema.additionalProperties,
      false,
      `tool \`${name}\` accepts extra arguments, which is where a locator sneaks in`,
    );
  }
});

test("the surfaces this slice did not wrap are absent", options, () => {
  const names = new Set(Object.keys(sdk.TOOL_SCHEMAS));
  for (const absent of ["markdown", "html", "verify", "mcp", "classify"]) {
    assert.equal(names.has(absent), false);
  }
});

// --- the peer stays optional ------------------------------------------------------------------------

test("the default import does not reach langchain", () => {
  // No `options`: this one holds whether or not the peer is installed, and it is the promise S2
  // and S3 made — the default install pulls nothing. The tools live on a subpath precisely so
  // that `import "ethos-engine"` never touches the peer.
  const source = readFileSync(join(PACKAGE_ROOT, "src", "index.js"), "utf8");
  assert.equal(source.includes("langchain"), false, "the default entry point mentions langchain");

  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"));
  assert.deepEqual(manifest.dependencies ?? {}, {}, "runtime dependencies must stay empty");
  assert.deepEqual(manifest.peerDependenciesMeta["@langchain/core"], { optional: true });
  assert.equal(manifest.exports["./langchain"], "./src/langchain.js");
});

test("this slice added no LangGraph dependency", () => {
  // §16.7 refused a separate LangGraph adapter: a tool built here is what LangGraph binds. A
  // `@langchain/langgraph` pin would be a second surface bought with a dependency, to prove
  // something a bindable tool already proves.
  const manifest = readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8");
  assert.equal(manifest.includes("langgraph"), false);
  const source = readFileSync(join(PACKAGE_ROOT, "src", "langchain.js"), "utf8");
  assert.equal(/from "@langchain\/langgraph/.test(source), false);
});
