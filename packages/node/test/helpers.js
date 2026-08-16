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
 * Shared fixtures.
 *
 * **The binary's absence is a failure, never a skip** (`docs/04-ARCHITECTURE.md` §4). A suite that
 * went green because it could not find the thing it tests would be the worst outcome available,
 * so {@link engineBinary} throws with the command that fixes it rather than calling `t.skip`.
 */

import { spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";
import { delimiter, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const PACKAGE_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
export const REPO_ROOT = dirname(dirname(PACKAGE_ROOT));

/**
 * An in-tree fixture — the same one the Python suite uses — so this suite depends on nothing
 * outside the repository. Two text runs on one page: enough that a node lookup is a lookup rather
 * than a coin flip, and small enough that a byte-identity failure is readable.
 */
export const FIXTURE_PDF = join(
  REPO_ROOT,
  "fixtures",
  "engine",
  "markdown-two-blocks",
  "document.pdf",
);

function isFile(path) {
  try {
    return statSync(path).isFile();
  } catch {
    return false;
  }
}

function locateBinary() {
  const pinned = process.env.ETHOS_ENGINE;
  if (pinned) return pinned;
  for (const candidate of [
    join(REPO_ROOT, "target", "release", "engine"),
    join(REPO_ROOT, "target", "debug", "engine"),
  ]) {
    if (isFile(candidate)) return candidate;
  }
  for (const dir of (process.env.PATH ?? "").split(delimiter).filter(Boolean)) {
    if (isFile(join(dir, "engine"))) return join(dir, "engine");
  }
  throw new Error(
    "no `engine` binary to test against. Build one:\n\n" +
      "    cargo build --locked\n\n" +
      "or point ETHOS_ENGINE at an existing build. This is a failure and not a skip: a green " +
      "run that never reached the engine would prove nothing.",
  );
}

/** Pin the binary for the whole run, the way a caller would. */
export const engineBinary = locateBinary();
process.env.ETHOS_ENGINE = engineBinary;

if (!existsSync(FIXTURE_PDF)) {
  throw new Error(`fixture missing: ${FIXTURE_PDF}`);
}

/**
 * Run the CLI directly, so a test can compare the SDK against it rather than against itself.
 *
 * @returns {Buffer} stdout with the trailing newline removed — the CLI ends its canonical JSON
 *   with one, and the artifact is what precedes it.
 */
export function cli(...args) {
  const result = spawnSync(engineBinary, args, { maxBuffer: Infinity });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`engine ${args.join(" ")} exited ${result.status}: ${result.stderr}`);
  }
  const out = result.stdout;
  return out.length && out[out.length - 1] === 0x0a ? out.subarray(0, out.length - 1) : out;
}

/**
 * The parameter names a function declares, read out of its own source.
 *
 * `Function.prototype.length` stops at the first default and says nothing about names, so it
 * cannot enforce the handle law's corollary. This reads the real header. Every exported function
 * here is a plain declaration with plain identifier parameters, and the test asserts the names it
 * recovers — so a parse that silently returned nothing could not make the ban vacuous.
 */
export function parameterNames(fn) {
  const source = Function.prototype.toString.call(fn);
  const open = source.indexOf("(");
  const close = source.indexOf(")", open);
  if (open === -1 || close === -1) {
    throw new Error(`cannot read the parameter list of ${fn.name}: ${source.slice(0, 80)}`);
  }
  return source
    .slice(open + 1, close)
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
}

/** A deep copy, so a test can edit an artifact without disturbing the one it came from. */
export function clone(value) {
  return structuredClone(value);
}
