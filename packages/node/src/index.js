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
 * The Node SDK — a thin surface over the `ethos-parser` CLI (v1.2-S3).
 *
 * # This is not a second design
 *
 * `packages/python/` shipped these same functions — three at v1.2-S2, `locate` beside them — and
 * **it is the contract**. If this file disagreed with it about a signature, an error type, or what
 * `ground` accepts, this file would be the one that is wrong. The differences below are the two
 * the languages force — `nodeGet` rather than `node_get`, and a class hierarchy that throws rather
 * than raises — and nothing else.
 *
 * # What this is, and the one property it is arranged to have
 *
 * `docs/history/13-V12-MILESTONES.md` S3 asks for "the same surface for Node, on the same terms", and
 * S2's terms are that the surface **cannot diverge from what the CLI prints**. This package
 * spends one process spawn to make that a tautology rather than a promise: `extract`, `ground`
 * and `locate` run the same subcommands a shell would run and hand back the bytes those
 * subcommands printed, parsed with `JSON.parse`. There is no second serialization anywhere in
 * this package, so there is nowhere for the artifact to change.
 *
 * That is also why there is no native addon. napi or neon would reach the library by a second
 * path, which is a second thing that can disagree with the first — plus a prebuild matrix across
 * platforms and ABI versions, for a saving nobody has measured a need for.
 *
 * # The handle law, which decides these four signatures
 *
 * `docs/history/12-V12-SCOPE.md` §3, carried here unchanged from MCP and Python: **the engine mints every
 * locator, returns it as an opaque handle, and re-validates it on the way back in.**
 *
 * - **A locator is returned, never accepted as prose.** No exported function here takes a page, a
 *   box, an `x`/`y`, a width, a height or a row/column pair. Not optionally, not in an options
 *   object, not behind a flag. `test/handle-law.test.js` reads the parameter names out of
 *   `Function.prototype.toString` and fails on those names.
 * - **A handle travels back as the bytes that were handed out.** {@link nodeGet} takes a node id
 *   string copied out of a representation this engine returned.
 * - **A handle this engine did not mint fails closed** — {@link NodeNotFound}, never `null`,
 *   never `undefined` and never `{}`. An empty answer tells a caller its guess was merely
 *   unlucky; a throw tells it the guess was not admissible.
 *
 * # What is deliberately absent
 *
 * `markdown` and `html` exist on the CLI and are not wrapped here: neither proves anything this
 * slice claims, and a function that exists because it was cheap is a surface to keep honest
 * forever. `verify` is absent for a stronger reason — it relays the pinned Ethos CLI, and
 * `docs/07-VERIFY-BOUNDARY.md` is the boundary a host's convenience must not bend. A JavaScript
 * function named `verify` would look like this package had an opinion about whether a claim is
 * supported. It does not, and neither does the engine.
 *
 * The four functions here are not an MCP client. MCP is a process; this is a library. The `ground`
 * tool on this package's tools subpath is the one exception: it makes one `tools/call` to
 * `ethos-parser mcp`, because the words it returns are the server's.
 *
 * # Locating the binary
 *
 * `ETHOS_PARSER` first and **authoritatively** — a path named there and not present is
 * {@link EngineNotFound}, not a reason to go looking for some other build — then `ethos-parser` on
 * `PATH`. That is the precedent `VerifierBinary::resolve` sets for `ETHOS_BIN`, and the reason is
 * the same: resolving to a binary nobody chose means returning artifacts from a parser nobody
 * chose. Nothing here downloads or vendors one.
 */

import { CanonicalizationError, sha256Hex } from "./c14n.js";
import {
  EngineError,
  EngineNotFound,
  EngineFailed,
  EngineTimeout,
  ArtifactTooLarge,
  NotARepresentation,
  FingerprintMismatch,
  NodeNotFound,
  parse,
  run,
  withQuotePath,
  withRepresentationPath,
} from "./engine.js";

export {
  EngineError,
  EngineNotFound,
  EngineFailed,
  EngineTimeout,
  ArtifactTooLarge,
  NotARepresentation,
  FingerprintMismatch,
  NodeNotFound,
};

/**
 * Tracks the workspace version, and `test/cli-surface.test.js` asserts it against `Cargo.toml`
 * and `package.json`. An SDK claiming a version the engine does not is the same class of lie
 * `parser_version` exists to prevent.
 */
export const version = "0.59.0";

/** The `artifact_type` `ethos-parser extract` stamps on a representation. */
export const REPRESENTATION_ARTIFACT_TYPE = "ethos.parser.representation.v0";

/** Everything under this prefix is a representation this package will read. */
const REPRESENTATION_PREFIX = "ethos.parser.representation.";

// -------------------------------------------------------------------------------------------
// The public surface — four functions, and not one of them names a coordinate
// -------------------------------------------------------------------------------------------

/**
 * Read a PDF and return `DocumentRepresentation v0`, as `ethos-parser extract` prints it.
 *
 * Every locator a later call needs is minted here. Pass this object back to {@link ground} or
 * {@link nodeGet} rather than composing one.
 *
 * @param {string} pdfPath Path to a PDF.
 * @returns {object} The parsed canonical artifact.
 * @throws {EngineNotFound} No binary.
 * @throws {EngineFailed} The engine could not read the document.
 */
export function extract(pdfPath) {
  return parse(run(["extract", String(pdfPath)]));
}

/**
 * Project a representation into `ethos.grounding.v1`, as `ethos-parser ground` prints it.
 *
 * Takes the artifact {@link extract} returned — the object itself, or a path to bytes this engine
 * wrote — mirroring the MCP tool of the same name and the Python `ground`. **The engine
 * re-validates it**: a payload that does not hash to its declared digest is refused there, by the
 * same `verify_fingerprint` every other subcommand runs, rather than by a second check here that
 * could drift from it.
 *
 * It takes no quote and no page, because `ethos-parser ground` takes neither: it projects the record.
 *
 * On success the engine may write up to four declarations to stderr, and this function returns the
 * artifact and discards them: nodes omitted for having no measurable ink box, which the
 * representation's `geometry-absent-not-groundable` limitation also carries; spans withheld past
 * the schema's span cap, shown in the artifact as `capabilities.spans` false without the count;
 * elements omitted for a string past the schema's byte limit, which leaves no trace in the artifact
 * or the representation; and tables withheld past the schema's limits, shown as
 * `capabilities.tables` false without the count. The `ground` tool on this package's tools subpath
 * carries `ethos-parser mcp`'s summary of all four.
 *
 * @param {object|string} representation The artifact object, or a path to it.
 * @returns {object} The parsed `ethos.grounding.v1` artifact.
 * @throws {NotARepresentation} The object will not canonicalize.
 * @throws {EngineFailed} The representation was refused, fingerprint included.
 */
export function ground(representation) {
  // `--` so a path beginning with `-` is a path, as it is to `ethos-parser mcp`, and never a flag.
  return withRepresentationPath(representation, (path) => parse(run(["ground", "--", path])));
}

/**
 * Report where `quote` lies in a representation, as `ethos-parser locate` prints it.
 *
 * Takes the artifact {@link extract} returned — the object itself, or a path to bytes this engine
 * wrote — on the same terms as {@link ground}, and a string. **It answers where, and nothing
 * else**: no boolean, no score, no evidence tier. A string that occurs nowhere is an `occurrences`
 * array of length zero and the same artifact a found one produces, which is decision #30's own
 * bound — so this throws nothing for it, and a caller reading the array's length is reading the
 * only answer there is.
 *
 * **The match rule is not ported here.** One process spawn, and the engine's own bytes back.
 * `docs/26-LOCATE-SCOPE.md` §6.3 gives the reason and it is the asymmetry with {@link nodeGet},
 * the right way round: a ported rule would be a second implementation of the *answer*, and two
 * implementations of a text-matching rule that can disagree is the one thing `locate` must not be.
 *
 * **The quote travels in a file, never on argv** — see {@link withQuotePath} for why argv cannot
 * carry every string a representation can contain. The file is removed whether or not the call
 * succeeded.
 *
 * @param {object|string} representation The artifact object, or a path to it.
 * @param {string} quote The string to look for, taken verbatim — untrimmed, unnormalized, unfolded.
 * @returns {object} The parsed `ethos.parser.locations.v0` artifact.
 * @throws {NotARepresentation} The object will not canonicalize.
 * @throws {EngineFailed} The representation was refused, fingerprint included, or the quote was
 *   refused — empty, or past the 16,384-byte ceiling. A refusal is not the not-found answer, and
 *   §4.1 is explicit that the two must not be read as one.
 */
export function locate(representation, quote) {
  return withRepresentationPath(representation, (path) =>
    withQuotePath(quote, (quoteFile) =>
      // `--` so a path beginning with `-` is a path, as it is to `ethos-parser mcp`, and never a flag.
      parse(run(["locate", "--quote-file", quoteFile, "--", path])),
    ),
  );
}

/**
 * Return one node from `representation`, by the id the engine minted for it.
 *
 * **The handle law, made mechanical**, and the one function here with no subcommand behind it.
 * `ethos-parser node-get` does not exist and this slice does not add it — MCP already carries the tool,
 * and a third CLI verb nobody asked for is surface to keep honest forever. So the checks are
 * ported rather than shelled out, in the order `mcp.rs` and the Python SDK run them:
 *
 * 1. the value is a representation;
 * 2. its payload is re-canonicalized and re-hashed, and must equal its declared digest — an
 *    artifact edited on the way through is refused **before** any lookup happens;
 * 3. `nodeId` is looked up among *that* artifact's own nodes.
 *
 * A miss throws. There is no argument by which to ask for a node by page or by position.
 *
 * @param {object} representation The artifact {@link extract} returned.
 * @param {string} nodeId A node id copied verbatim out of that artifact.
 * @returns {object} The node record the artifact carries, verbatim — the same object, not a copy.
 * @throws {NotARepresentation} The value is not a representation.
 * @throws {FingerprintMismatch} The payload does not hash to its declared digest.
 * @throws {NodeNotFound} Nothing minted that id — the forged-handle case, failing closed.
 */
export function nodeGet(representation, nodeId) {
  const { payload, declared } = validatedPayload(representation);
  for (const node of payload.nodes) {
    if (node !== null && typeof node === "object" && node.id === nodeId) {
      return node;
    }
  }
  throw new NodeNotFound(nodeId, declared);
}

// -------------------------------------------------------------------------------------------
// Internals
// -------------------------------------------------------------------------------------------

/** Re-validate a representation and return its payload and declared fingerprint. */
function validatedPayload(representation) {
  if (representation === null || typeof representation !== "object" || Array.isArray(representation)) {
    throw new NotARepresentation(
      `expected a representation object, got ${representation === null ? "null" : typeof representation}`,
    );
  }

  const artifactType = representation.artifact_type;
  if (typeof artifactType !== "string" || !artifactType.startsWith(REPRESENTATION_PREFIX)) {
    throw new NotARepresentation(
      `artifact_type is ${JSON.stringify(artifactType) ?? "absent"}; expected one under ` +
        `\`${REPRESENTATION_PREFIX}\`. \`ground\` and \`nodeGet\` read the artifact \`extract\` ` +
        "minted, not a projection of it.",
    );
  }

  const declared = representation.representation_c14n_sha256;
  const payload = representation.representation;
  if (
    typeof declared !== "string" ||
    payload === null ||
    typeof payload !== "object" ||
    Array.isArray(payload)
  ) {
    throw new NotARepresentation(
      "a representation carries a `representation` object and a `representation_c14n_sha256` " +
        "string; this one does not",
    );
  }

  let actual;
  try {
    actual = `sha256:${sha256Hex(payload)}`;
  } catch (e) {
    if (e instanceof CanonicalizationError) {
      // A payload the canonical serializer refuses cannot be the payload that digest covers, so
      // this is the same news as a mismatch and not a lookup that half-happened.
      throw new NotARepresentation(
        `this payload will not canonicalize, so it cannot be the one \`${declared}\` covers: ${e.message}`,
      );
    }
    throw e;
  }
  if (actual !== declared) {
    throw new FingerprintMismatch(declared, actual);
  }

  if (!Array.isArray(payload.nodes)) {
    throw new NotARepresentation("the payload carries no `nodes` list");
  }
  return { payload, declared };
}
