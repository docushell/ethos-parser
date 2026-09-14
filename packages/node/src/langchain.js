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

/**
 * LangChain tools over the SDK (v1.2-S4) — the Python module in a second language.
 *
 * # The one thing this slice is for
 *
 * `docs/history/12-V12-SCOPE.md` §3, the corollary about where locators travel: **locators live in the
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
 * version is arranged to prevent, and it is why no summary here is written afresh. `extract`'s and
 * `node_get`'s are counts copied from `ethos-parser-cli/src/mcp.rs` and built from the artifact;
 * `ground`'s is `ethos-parser mcp`'s own reply text, because it states facts the artifact does not
 * carry.
 *
 * # It is not a third implementation
 *
 * `extract` and `node_get` call `./index.js` — the same functions a shell would reach through the
 * CLI. `ground` asks `ethos-parser mcp` for one `tools/call`, by path and never with the object, and
 * takes the summary and the artifact from that one reply: since v2.2-S7 an element is a block of
 * runs, so no count this adapter could compute equals the engine's omission, and the schema-limit
 * declarations exist only in the projection. On a refusal it runs `ethos-parser ground` on the same
 * path, so the error thrown is the SDK's own. `packages/python/src/ethos_parser/langchain.py` is the
 * contract, exactly as the Python SDK is the contract for `./index.js`; the only differences are
 * the ones the frameworks force.
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

import { EngineError, parse, run, withRepresentationPath } from "./engine.js";
import { extract as sdkExtract, nodeGet as sdkNodeGet } from "./index.js";

let tool;
try {
  ({ tool } = await import("@langchain/core/tools"));
} catch (cause) {
  throw new Error(
    "`ethos-parser/langchain` needs @langchain/core, which is an optional peer so that the " +
      "default import pulls nothing. Install it with:\n\n" +
      "    npm install @langchain/core\n\n" +
      "This is a named failure rather than a degraded import: a tools() that returned an empty " +
      "array would look like a package with no tools.",
    { cause },
  );
}

/**
 * The argument schemas, **verbatim from what `ethos-parser mcp` advertises**.
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
    // The summary and the artifact both come from `ethos-parser mcp`'s one reply, so the words are
    // MCP's by construction — the omission count, and every schema-limit clause, now and later.
    // `nodes - elements`, which this used to compute, stopped being the omission when an element
    // became a block, and the limit declarations were never in the artifact to count.
    const result = withRepresentationPath(representation, (path) => {
      const reply = mcpGround(path);
      if (reply.isError === true) {
        // A refusal. `ethos-parser ground` on the same bytes throws exactly what `ground()` would —
        // exit status, code and stderr — rather than a sentence reworded here.
        run(["ground", "--", path]);
        throw new EngineError(
          "`ethos-parser mcp` refused this representation but `ethos-parser ground` projected it: " +
            mcpText(reply),
        );
      }
      return reply;
    });
    const content = result.content;
    const artifact = result.structuredContent;
    if (
      !Array.isArray(content) ||
      content.length !== 1 ||
      content[0] === null ||
      typeof content[0] !== "object" ||
      typeof content[0].text !== "string" ||
      artifact === null ||
      typeof artifact !== "object" ||
      Array.isArray(artifact)
    ) {
      throw new EngineError("`ethos-parser mcp` replied to `ground` in a shape this adapter does not know");
    }
    return [content[0].text, artifact];
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
 * One `tools/call` to `ethos-parser mcp`, by path, and its `result`.
 *
 * No `initialize`: the server answers a call without one, and each extra reply line would be a copy
 * of nothing. The representation is sent **by path, never inline** — the server holds an inline
 * argument as a parsed request several times over. The reply is one line, parsed whole and split
 * on nothing.
 */
function mcpGround(path) {
  const request = Buffer.from(
    JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "tools/call",
      params: { name: "ground", arguments: { representation: path } },
    }) + "\n",
    "utf8",
  );
  const stdout = run(["mcp"], request);
  if (stdout.length === 0 || stdout[stdout.length - 1] !== 0x0a || stdout.indexOf(0x0a) !== stdout.length - 1) {
    throw new EngineError("`ethos-parser mcp` did not reply with exactly one line to one call");
  }
  const envelope = parse(stdout);
  if (envelope === null || typeof envelope !== "object" || Array.isArray(envelope)) {
    throw new EngineError("`ethos-parser mcp` replied with something that is not an object");
  }
  if ("error" in envelope) {
    const error = envelope.error ?? {};
    throw new EngineError(`\`ethos-parser mcp\` refused the call: ${error.code} ${error.message}`);
  }
  const result = envelope.result;
  if (envelope.id !== 1 || result === null || typeof result !== "object" || Array.isArray(result)) {
    throw new EngineError("`ethos-parser mcp` replied to a call this adapter did not make");
  }
  return result;
}

function mcpText(result) {
  const content = result.content;
  return Array.isArray(content) && content[0] && typeof content[0] === "object"
    ? String(content[0].text ?? "")
    : "";
}
