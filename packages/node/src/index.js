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
 * `packages/python/` shipped the same three functions at v1.2-S2 and **it is the contract**. If
 * this file disagreed with it about a signature, an error type, or what `ground` accepts, this
 * file would be the one that is wrong. The differences below are the two the languages force —
 * `nodeGet` rather than `node_get`, and a class hierarchy that throws rather than raises — and
 * nothing else.
 *
 * # What this is, and the one property it is arranged to have
 *
 * `docs/history/13-V12-MILESTONES.md` S3 asks for "the same surface for Node, on the same terms", and
 * S2's terms are that the surface **cannot diverge from what the CLI prints**. This package
 * spends one process spawn to make that a tautology rather than a promise: `extract` and
 * `ground` run the same subcommands a shell would run and hand back the bytes those subcommands
 * printed, parsed with `JSON.parse`. There is no second serialization anywhere in this package,
 * so there is nowhere for the artifact to change.
 *
 * That is also why there is no native addon. napi or neon would reach the library by a second
 * path, which is a second thing that can disagree with the first — plus a prebuild matrix across
 * platforms and ABI versions, for a saving nobody has measured a need for.
 *
 * # The handle law, which decides these three signatures
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
 * There is no MCP client here either. MCP is a process; this is a library.
 *
 * # Locating the binary
 *
 * `ETHOS_PARSER` first and **authoritatively** — a path named there and not present is
 * {@link EngineNotFound}, not a reason to go looking for some other build — then `ethos-parser` on
 * `PATH`. That is the precedent `VerifierBinary::resolve` sets for `ETHOS_BIN`, and the reason is
 * the same: resolving to a binary nobody chose means returning artifacts from a parser nobody
 * chose. Nothing here downloads or vendors one.
 */

import { spawnSync } from "node:child_process";
import { accessSync, constants, mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";

import { CanonicalizationError, c14nBytes, sha256Hex } from "./c14n.js";

/**
 * Tracks the workspace version, and `test/cli-surface.test.js` asserts it against `Cargo.toml`
 * and `package.json`. An SDK claiming a version the engine does not is the same class of lie
 * `parser_version` exists to prevent.
 */
export const version = "0.54.0";

/** The `artifact_type` `ethos-parser extract` stamps on a representation. */
export const REPRESENTATION_ARTIFACT_TYPE = "ethos.parser.representation.v0";

/** Everything under this prefix is a representation this package will read. */
const REPRESENTATION_PREFIX = "ethos.parser.representation.";

/** The environment variable that pins the binary, named to match `ETHOS_BIN`. */
const BINARY_ENV = "ETHOS_PARSER";

// -------------------------------------------------------------------------------------------
// Failures, each one named
// -------------------------------------------------------------------------------------------
//
// Distinct classes rather than one, for the reason the CLI keeps three exit codes apart: a caller
// that cannot tell "the check failed" from "the check did not run" is the defect this project
// refuses. A forged handle and a missing binary are not the same news.

/**
 * Base class for every failure the three exported functions throw.
 *
 * Everything, deliberately: a serialization refusal from `./c14n.js` is re-thrown as
 * {@link NotARepresentation} rather than escaping as a bare `CanonicalizationError`, so
 * `catch (e) { if (e instanceof EngineError) ... }` is a complete catch and a caller never has to
 * know this package canonicalizes anything.
 */
export class EngineError extends Error {
  constructor(message) {
    super(message);
    this.name = "EngineError";
  }
}

/** No `ethos-parser` binary could be located, or the one pinned by `ETHOS_PARSER` is absent. */
export class EngineNotFound extends EngineError {
  constructor(message) {
    super(message);
    this.name = "EngineNotFound";
  }
}

/**
 * The engine ran and refused.
 *
 * Carries the exit status and the engine's own stderr, unedited: the CLI already says what went
 * wrong and in what terms, and paraphrasing it here would be a second account of the same failure
 * that could drift from the first.
 */
export class EngineFailed extends EngineError {
  constructor(commandArgs, status, stderr) {
    super(
      `\`ethos-parser ${commandArgs.join(" ")}\` exited ${status}: ${stderr.trim() || "(no stderr)"}`,
    );
    this.name = "EngineFailed";
    this.commandArgs = [...commandArgs];
    this.status = status;
    this.stderr = stderr;
  }
}

/** The value handed in is not a `DocumentRepresentation` this package will read. */
export class EngineTimeout extends EngineError {
  /**
   * The engine was still running when the wall clock ran out, and was killed.
   *
   * Separate from {@link EngineFailed} on purpose. A refusal is an ANSWER — the engine read the
   * document and said no, with a reason on stderr and one of three exit codes — while a timeout
   * is the absence of one. A caller retrying, alerting or falling back wants those in different
   * branches, and "non-zero exit" would put "this PDF is encrypted" beside "this never came
   * back".
   *
   * Until v2-S15 there was no timeout at any layer, so this was an unbounded hang.
   */
  constructor(commandArgs, ms, stderr) {
    super(
      `\`ethos-parser ${commandArgs.join(" ")}\` did not finish within ${ms} ms and was killed` +
        (stderr.trim() ? `: ${stderr.trim()}` : ""),
    );
    this.name = "EngineTimeout";
    this.commandArgs = [...commandArgs];
    this.ms = ms;
    this.stderr = stderr;
  }
}

export class ArtifactTooLarge extends EngineError {
  /**
   * The engine printed more than {@link MAX_ARTIFACT_BYTES} of stdout.
   *
   * `maxBuffer: Infinity` was here until v2-S15, with a comment correctly criticising Node's
   * one-megabyte default as "a size limit nobody chose" — and then drawing the wrong conclusion
   * from it. The answer to an arbitrary default is a CHOSEN ceiling, not the absence of one:
   * unbounded, the artifact is buffered, `.toString("utf8")` copies it, and `JSON.parse` builds a
   * graph from it, so peak is several times the on-wire size with nothing bounding any of it.
   *
   * A truncated read would still arrive as the confusing JSON parse error that comment warned
   * about, which is why this is its own named failure instead.
   */
  constructor(commandArgs, limit) {
    super(
      `\`ethos-parser ${commandArgs.join(" ")}\` produced more than ${limit} bytes of stdout. ` +
        `Raise ETHOS_PARSER_MAX_BYTES if this document is genuinely that large.`,
    );
    this.name = "ArtifactTooLarge";
    this.commandArgs = [...commandArgs];
    this.limit = limit;
  }
}

export class NotARepresentation extends EngineError {
  constructor(message) {
    super(message);
    this.name = "NotARepresentation";
  }
}

/**
 * A representation's payload does not hash to its own declared digest.
 *
 * Names both digests, as `verify_fingerprint` does, so the failure is diagnosable rather than
 * merely detected.
 */
export class FingerprintMismatch extends EngineError {
  constructor(declared, actual) {
    super(
      `declared representation_c14n_sha256 is ${declared} but the payload hashes to ${actual}. ` +
        "This is not a record this engine will speak for: something edited the artifact after " +
        "it was minted.",
    );
    this.name = "FingerprintMismatch";
    this.declared = declared;
    this.actual = actual;
  }
}

/**
 * A node id is not a node of the representation it was looked up in.
 *
 * **This is the handle law failing closed.** Not `null`, not `undefined`, not `{}`, not the
 * nearest node.
 */
export class NodeNotFound extends EngineError {
  constructor(nodeId, fingerprint) {
    super(
      `\`${nodeId}\` is not a node of this representation (${fingerprint}). A node id is an ` +
        "opaque handle this engine minted: copy one from the artifact rather than composing it.",
    );
    this.name = "NodeNotFound";
    this.nodeId = nodeId;
    this.fingerprint = fingerprint;
  }
}

// -------------------------------------------------------------------------------------------
// The public surface — three functions, and not one of them names a coordinate
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
 * Nodes with no measurable ink box are omitted from the projection and counted by the engine on
 * stderr; the representation this came from is where that declaration lives, which is the CLI's
 * own arrangement and not a drop introduced here.
 *
 * @param {object|string} representation The artifact object, or a path to it.
 * @returns {object} The parsed `ethos.grounding.v1` artifact.
 * @throws {NotARepresentation} The object will not canonicalize.
 * @throws {EngineFailed} The representation was refused, fingerprint included.
 */
export function ground(representation) {
  if (typeof representation === "string") {
    return parse(run(["ground", representation]));
  }

  // Written as canonical bytes — this package's one serializer, the same one the fingerprint is
  // computed over — so what reaches the engine is what the engine printed. `JSON.stringify`
  // would be a second serialization, which is the thing this package exists not to have.
  let body;
  try {
    body = c14nBytes(representation);
  } catch (e) {
    if (e instanceof CanonicalizationError) {
      throw new NotARepresentation(
        `this object will not canonicalize, so it is not the artifact \`extract\` printed: ${e.message}`,
      );
    }
    throw e;
  }

  const directory = mkdtempSync(join(tmpdir(), "ethos-parser-"));
  try {
    const path = join(directory, "representation.json");
    writeFileSync(path, body);
    return parse(run(["ground", path]));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
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

/** Locate the engine. `ETHOS_PARSER` is authoritative; `PATH` is the fallback. */
function binary() {
  const pinned = process.env[BINARY_ENV];
  if (pinned) {
    // `isFile` and not an executable-bit check, matching Python's `os.path.isfile` on the same
    // branch: a pin that names a file which turns out not to run should fail with the operating
    // system's own reason, not be quietly treated as absent.
    if (isFile(pinned)) {
      return pinned;
    }
    throw new EngineNotFound(
      `${BINARY_ENV}=${JSON.stringify(pinned)} names a file that is not there. An explicit pin ` +
        "is authoritative: falling back to some other build would mean returning artifacts from " +
        "a parser nobody chose.",
    );
  }

  const found = onPath("ethos-parser");
  if (found) {
    return found;
  }

  throw new EngineNotFound(
    "no `ethos-parser` binary. Tried, in order:\n" +
      `  ${BINARY_ENV} (unset)\n` +
      "  `ethos-parser` on PATH (not found)\n\n" +
      "This package is a surface over that binary and computes nothing without it. Build it " +
      "with `cargo build --release`, then put it on PATH or point " +
      `${BINARY_ENV} at it. Nothing here downloads one.`,
  );
}

function isFile(path) {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

/**
 * The first executable of this name on `PATH`.
 *
 * Node has no `shutil.which`, so this is it — and it checks the executable bit for the reason
 * `which` does: a non-executable file of the right name earlier on the path is not the binary the
 * caller meant, and skipping it is what every shell already does.
 */
function onPath(name) {
  const parts = (process.env.PATH ?? "").split(delimiter).filter(Boolean);
  for (const dir of parts) {
    const candidate = join(dir, name);
    try {
      accessSync(candidate, constants.X_OK);
    } catch {
      continue;
    }
    if (isFile(candidate)) {
      return candidate;
    }
  }
  return null;
}

/**
 * Wall clock for one engine invocation, in milliseconds. Override with `ETHOS_PARSER_TIMEOUT`
 * (seconds, matching the Python SDK); `0` waits forever, which is what every version before
 * v2-S15 did unconditionally.
 *
 * Ten minutes is chosen rather than derived, and deliberately far above any healthy run — the
 * largest document in the corpus extracts in seconds, so this is not a performance budget. It is
 * the line past which "slow" has become "never".
 */
const DEFAULT_TIMEOUT_MS = 10 * 60 * 1000;

/**
 * Ceiling on one artifact's stdout, in bytes. Override with `ETHOS_PARSER_MAX_BYTES`.
 *
 * 512 MiB is far above any artifact this engine has produced and far below "unbounded". The
 * number matters less than its existence: it is a decision, where `Infinity` was the absence of
 * one.
 */
const MAX_ARTIFACT_BYTES = 512 * 1024 * 1024;

/** A numeric environment override, or the default. Refuses nonsense rather than falling back. */
function numericEnv(name, fallback, scale = 1) {
  const raw = process.env[name];
  if (raw === undefined || raw === "") return fallback;
  const value = Number(raw);
  if (!Number.isFinite(value) || value < 0) {
    throw new EngineError(
      `${name}=${JSON.stringify(raw)} is not a non-negative number. Use 0 for no limit.`,
    );
  }
  return value === 0 ? 0 : value * scale;
}

/** Run a subcommand and return its stdout. A non-zero exit throws, never returns empty. */
function run(commandArgs) {
  const engine = binary();
  const timeoutMs = numericEnv("ETHOS_PARSER_TIMEOUT", DEFAULT_TIMEOUT_MS, 1000);
  const maxBuffer = numericEnv("ETHOS_PARSER_MAX_BYTES", MAX_ARTIFACT_BYTES);
  const result = spawnSync(engine, commandArgs, {
    // No `env`: the engine is deterministic and handing it an environment this package composed
    // would be one more input nobody declared. `spawnSync` inherits `process.env` by default.
    //
    // Two bounds, both chosen (v2-S15). Node's one-megabyte `maxBuffer` default is too small for
    // a real representation, and `Infinity` was the previous answer to that — but unbounded means
    // the artifact is buffered, copied by `.toString("utf8")` and then expanded by `JSON.parse`,
    // several times the on-wire size with nothing bounding any of it. And with no `timeout`, a
    // document that stalls the engine hung the caller with no exception to route.
    maxBuffer: maxBuffer === 0 ? Infinity : maxBuffer,
    ...(timeoutMs === 0 ? {} : { timeout: timeoutMs, killSignal: "SIGKILL" }),
  });

  const stderrEarly = (result.stderr ?? Buffer.alloc(0)).toString("utf8");

  // Order matters, and the order is ENOBUFS first. `spawnSync` reports BOTH bounds through
  // `error`, and it kills the child with `killSignal` in BOTH cases — so with `killSignal:
  // "SIGKILL"` set for the timeout, an over-large buffer also arrives carrying `signal ===
  // "SIGKILL"`. Testing the signal before the code therefore reported every ArtifactTooLarge as
  // an EngineTimeout; `bounds.test.js` caught exactly that. Dispatch on the specific `code` the
  // condition sets, and treat the signal only as the fallback the timeout case needs.
  if (result.error) {
    const code = result.error.code;
    if (code === "ENOBUFS") {
      throw new ArtifactTooLarge(commandArgs, maxBuffer);
    }
    if (code === "ETIMEDOUT" || result.signal === "SIGKILL") {
      throw new EngineTimeout(commandArgs, timeoutMs, stderrEarly);
    }
    throw new EngineNotFound(`\`${engine}\` could not be run: ${result.error.message}`);
  }

  const stderr = (result.stderr ?? Buffer.alloc(0)).toString("utf8");
  if (result.status !== 0) {
    // `status` is null when a signal killed the child; naming the signal is more useful than
    // reporting an exit code that never happened.
    const how = result.status === null ? `signal ${result.signal}` : result.status;
    throw new EngineFailed(commandArgs, how, stderr);
  }
  return result.stdout;
}

/** Parse canonical JSON the engine printed. */
function parse(stdout) {
  try {
    return JSON.parse(stdout.toString("utf8"));
  } catch (e) {
    throw new EngineError(`the engine exited 0 but its output is not canonical JSON: ${e.message}`);
  }
}
