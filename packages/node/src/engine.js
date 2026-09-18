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
 * The engine, as a process: locating the binary, running it, and naming how that fails.
 *
 * Private to this package — not in `package.json`'s `exports`, so no caller imports it. It exists
 * so the tools subpath can run the same binary with the same bounds and the same failure classes as
 * the four public functions, rather than a second copy of any of it. The public module re-exports
 * the failure classes; nothing else here is public.
 */

import { spawnSync } from "node:child_process";
import { accessSync, constants, mkdtempSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";

import { CanonicalizationError, c14nBytes } from "./c14n.js";

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
 * Base class for every failure the four exported functions throw.
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

/** The value handed in is not a `DocumentRepresentation` this package will read. */
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

/**
 * Run a subcommand and return its stdout. A non-zero exit throws, never returns empty.
 *
 * `input` is bytes for the engine's stdin, for the one caller that speaks to `ethos-parser mcp`;
 * every other call passes nothing, as before.
 */
export function run(commandArgs, input) {
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
    ...(input === undefined ? {} : { input }),
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
export function parse(stdout) {
  try {
    return JSON.parse(stdout.toString("utf8"));
  } catch (e) {
    throw new EngineError(`the engine exited 0 but its output is not canonical JSON: ${e.message}`);
  }
}

/**
 * Call `body` with a path to the representation's bytes, and remove any file made for it after.
 *
 * A string is a path and is used as given. An object is written as canonical bytes — this
 * package's one serializer, the same one the fingerprint is computed over — so what reaches the
 * engine is what the engine printed. `JSON.stringify` would be a second serialization, which is the
 * thing this package exists not to have.
 */
export function withRepresentationPath(representation, body) {
  if (typeof representation === "string") {
    return body(representation);
  }
  let bytes;
  try {
    bytes = c14nBytes(representation);
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
    writeFileSync(path, bytes);
    return body(path);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

/**
 * Call `body` with a path to a file holding the quote's UTF-8 bytes, and remove it after.
 *
 * **`locate`'s quote travels in a file and never on argv** (`docs/26-LOCATE-SCOPE.md` §6.1): argv
 * cannot carry every string a representation can contain — a NUL cannot appear in an argument at
 * all, a newline survives only through correct quoting, and a quoting mistake changes the searched
 * string *silently*, which changes what was searched with nothing on the wire saying so.
 *
 * The same `node:fs` facility {@link withRepresentationPath} uses, so there is one way this
 * package hands the engine bytes it wrote, and the file goes whether or not `body` threw.
 */
export function withQuotePath(quote, body) {
  const directory = mkdtempSync(join(tmpdir(), "ethos-parser-"));
  try {
    const path = join(directory, "quote.txt");
    // The encoding and nothing else — no newline appended, no BOM. The CLI reads these bytes
    // verbatim, so a byte added here would change the string that was searched.
    writeFileSync(path, Buffer.from(quote, "utf8"));
    return body(path);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}
