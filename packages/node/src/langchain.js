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
 * LangChain tools over the SDK (v1.2-S4) — the Python module in a second language.
 *
 * # The one thing this slice is for
 *
 * `docs/12-V12-SCOPE.md` §3, the corollary about where locators travel: **locators live in the
 * artifact, never in the prose a model reads and edits.** MCP says that with `structuredContent`
 * versus `content`; LangChain says it with a `ToolMessage`'s `artifact` versus its `content`, and
 * the parser memo §16.7's LangChain row names that split directly. So these tools declare
 * `responseFormat: "content_and_artifact"`, and the whole of S4 is getting the two sides right:
 *
 * | MCP | here |
 * | --- | --- |
 * | `structuredContent` | the tool's `artifact` — the SDK object, unaltered |
 * | `content` text | the tool's `content` — **counts, and nothing a pipeline would bind to** |
 *
 * A box in `content` is a locator a model can edit and then cite. That is the failure this whole
 * version is arranged to prevent, and it is why the summary strings below are copied from
 * `engine-cli/src/mcp.rs` rather than written afresh.
 *
 * # It is not a third implementation
 *
 * Every tool calls `./index.js` — the same functions a shell would reach through the CLI. Nothing
 * here spawns `engine`, and nothing here talks to MCP. `packages/python/src/ethos_engine/
 * langchain.py` is the contract, exactly as the Python SDK is the contract for `./index.js`; the
 * only differences are the ones the frameworks force.
 *
 * # There is no LangGraph adapter, deliberately
 *
 * §16.7 refused one: a tool built here is already what `bindTools` and a LangGraph `ToolNode`
 * take, so a graph, a node or a checkpointer here would be a second surface that adds nothing.
 * This module imports no `@langchain/langgraph`, and this slice adds no dependency on one.
 *
 * # It does not set trust state
 *
 * No field, and no word in a summary, says `grounded`, `verified`, an evidence tier, a score or a
 * degree of belief. `docs/07-VERIFY-BOUNDARY.md` is unchanged: this engine validates structure and
 * binding, and it never verifies a claim.
 *
 * # Installing
 *
 * `@langchain/core` is an **optional peer**, so the default import pulls nothing:
 *
 *     npm install @langchain/core
 *
 * Importing this module without it is a named failure, never a silent skip — the try/catch below
 * runs at module load, so it fails where Python's `ImportError` fails.
 */

import { readFileSync } from "node:fs";

import { extract as sdkExtract, ground as sdkGround, nodeGet as sdkNodeGet } from "./index.js";

let tool;
try {
  ({ tool } = await import("@langchain/core/tools"));
} catch (cause) {
  throw new Error(
    "`ethos-engine/langchain` needs @langchain/core, which is an optional peer so that the " +
      "default import pulls nothing. Install it with:\n\n" +
      "    npm install @langchain/core\n\n" +
      "This is a named failure rather than a degraded import: a tools() that returned an empty " +
      "array would look like a package with no tools.",
    { cause },
  );
}

/**
 * The argument schemas, **verbatim from what `engine mcp` advertises**.
 *
 * One wire shape across the three adapters, and `test/langchain.test.js` asserts these against
 * `tools/list` rather than against a reviewer's memory. That is also where the geometry ban is
 * enforced: no `page`, no `bbox`, no `x`/`y`, no row/column pair, in any of them.
 *
 * `node_id` keeps MCP's spelling even though this package's function parameter is `nodeId`: the
 * tool argument is the wire, and one wire has one name. Plain JSON Schema rather than zod, so the
 * three adapters can be compared object to object — and so this package still needs nothing but
 * `@langchain/core`.
 */
export const TOOL_SCHEMAS = {
  extract: {
    type: "object",
    additionalProperties: false,
    required: ["path"],
    properties: { path: { type: "string", description: "Path to a PDF file." } },
  },
  ground: {
    type: "object",
    additionalProperties: false,
    required: ["representation"],
    properties: {
      representation: {
        description:
          "The artifact `extract` returned — the object itself, or a path to bytes this engine wrote.",
        type: ["object", "string"],
      },
    },
  },
  node_get: {
    type: "object",
    additionalProperties: false,
    required: ["representation", "node_id"],
    properties: {
      representation: {
        description:
          "The artifact `extract` returned — the object itself, or a path to bytes this engine wrote.",
        type: ["object", "string"],
      },
      node_id: {
        type: "string",
        description: "A node id copied verbatim from that representation's `nodes`.",
      },
    },
  },
};

const DESCRIPTIONS = {
  extract:
    "Read a PDF and return `DocumentRepresentation v0` — the canonical evidence record. Every " +
    "locator a later call needs is minted here and travels in the tool artifact; pass that " +
    "artifact back rather than composing one.",
  ground:
    "Project a representation into `ethos.grounding.v1`. Takes the artifact `extract` returned; " +
    "the representation is fingerprint-checked before it is read.",
  node_get:
    "Return one node from a representation. `node_id` is an OPAQUE HANDLE: copy it from a " +
    "representation this engine returned. It is re-validated against that artifact, and an id " +
    "this engine did not mint is an error. There is no way to ask for a node by page or by " +
    "position.",
};

// -------------------------------------------------------------------------------------------
// The three, each returning `[content, artifact]`
// -------------------------------------------------------------------------------------------
//
// A failure THROWS rather than returning a sentence. LangChain surfaces that as a tool error,
// which is MCP's `isError: true` path in this framework's currency — and the reason is the one
// `12-V12-SCOPE.md` §3 gives: an empty result tells a model its guess was merely unlucky, and an
// error tells it the guess was not admissible. Nothing below catches what the SDK throws.

const IMPLEMENTATIONS = {
  extract({ path }) {
    const artifact = sdkExtract(path);
    const payload = artifact.representation;
    return [
      `${payload.pages.length} page(s), ${payload.nodes.length} node(s). Locators are in the artifact.`,
      artifact,
    ];
  },

  ground({ representation }) {
    const artifact = sdkGround(representation);
    // `nodes - elements`, which is the question the summary asks: how many nodes did not become
    // elements. Counting geometry rows instead would re-encode `GeometryPresence::is_groundable`
    // out here, and a count derived from a different question than the one being asked is a count
    // that goes wrong the first time a second absence variant appears. `test/langchain.test.js`
    // pins this string against the one `engine mcp` emits, which uses the engine's own
    // `omission.nodes_omitted`.
    const omitted = nodeCount(representation) - artifact.elements.length;
    return [
      `${artifact.elements.length} element(s) with a measured box; ${omitted} omitted for having none.`,
      artifact,
    ];
  },

  node_get({ representation, node_id: nodeId }) {
    const node = sdkNodeGet(representation, nodeId);
    // The kind, and nothing else. **The id stays in the artifact** — a summary carrying it would
    // be handing the model a handle through the one channel it can rewrite.
    return [`1 node, kind \`${node.kind}\`.`, node];
  },
};

/**
 * The three tools, ready for `bindTools` or a LangGraph `ToolNode`.
 *
 * @returns {Array} `[extract, ground, node_get]`.
 */
export function tools() {
  return Object.keys(TOOL_SCHEMAS).map((name) =>
    tool(async (args) => IMPLEMENTATIONS[name](args), {
      name,
      description: DESCRIPTIONS[name],
      schema: TOOL_SCHEMAS[name],
      // The whole slice. Without this the artifact would be stringified into `content`, which is
      // a locator a model can edit and then cite.
      responseFormat: "content_and_artifact",
    }),
  );
}

/**
 * How many nodes the representation carries, for the omission arithmetic above.
 *
 * Accepts what the SDK accepts. A path is read rather than re-projected: `ground` has already run
 * and refused anything malformed, so this only ever counts an artifact the engine wrote.
 */
function nodeCount(representation) {
  const payload =
    typeof representation === "string"
      ? JSON.parse(readFileSync(representation, "utf8")).representation
      : representation.representation;
  return payload.nodes.length;
}
