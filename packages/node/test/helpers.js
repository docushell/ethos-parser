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
 * Shared fixtures.
 *
 * **The binary's absence is a failure, never a skip** (`docs/04-ARCHITECTURE.md` §4). A suite that
 * went green because it could not find the thing it tests would be the worst outcome available,
 * so {@link engineBinary} throws with the command that fixes it rather than calling `t.skip`.
 */

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
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

/** The version `Cargo.toml` declares — the one a located binary has to agree with. */
function workspaceVersion() {
  const cargo = readFileSync(join(REPO_ROOT, "Cargo.toml"), "utf8");
  const match = cargo.match(/^version = "([^"]+)"$/m);
  if (!match) throw new Error("no workspace version in Cargo.toml");
  return match[1];
}

/** `ethos-parser --version`, or `null` if it will not run. */
function binaryVersion(path) {
  const result = spawnSync(path, ["--version"], { encoding: "utf8" });
  if (result.error || result.status !== 0) return null;
  return result.stdout.trim().split(/\s+/).pop();
}

/**
 * Find a binary that is **this workspace's**, and refuse one that is not.
 *
 * The version check is not decoration. A stale `target/release/ethos-parser` is preferred over nothing
 * at all and answers every question plausibly, so a suite that took the first file it found would
 * compare the SDK against a parser from six minor versions ago and go green — it would still
 * prove the SDK does not alter what the CLI prints, but about the wrong CLI. That is a false
 * green, and it is the one this locator exists to make impossible. Measured, not feared: it is
 * what a `0.11.0` release build did here at v1.2-S4.
 */
function locateBinary() {
  const want = workspaceVersion();

  const pinned = process.env.ETHOS_PARSER;
  if (pinned) {
    // An explicit pin is authoritative in both directions: it is never silently overridden, and a
    // pin that is the wrong build is an error rather than a reason to look elsewhere.
    const got = binaryVersion(pinned);
    if (got !== want) {
      throw new Error(
        `ETHOS_PARSER=${JSON.stringify(pinned)} reports ${got ?? "nothing"} but this workspace ` +
          `is ${want}. Rebuild it, or point ETHOS_PARSER at a build of this tree.`,
      );
    }
    return pinned;
  }

  const candidates = [
    join(REPO_ROOT, "target", "release", "ethos-parser"),
    join(REPO_ROOT, "target", "debug", "ethos-parser"),
  ];
  for (const dir of (process.env.PATH ?? "").split(delimiter).filter(Boolean)) {
    const onPath = join(dir, "ethos-parser");
    if (isFile(onPath)) candidates.push(onPath);
  }

  const tried = [];
  for (const candidate of candidates) {
    const got = binaryVersion(candidate);
    if (got === want) return candidate;
    tried.push(`  ${candidate} (${got ?? "absent"})`);
  }

  throw new Error(
    `no \`engine\` binary at ${want}. Tried, in order:\n${tried.join("\n") || "  (nothing)"}\n\n` +
      "    cargo build --locked\n\n" +
      "or point ETHOS_PARSER at a build of this tree. This is a failure and not a skip: a green " +
      "run against a parser from another version would prove something about the wrong engine.",
  );
}

/** Pin the binary for the whole run, the way a caller would. */
export const engineBinary = locateBinary();
process.env.ETHOS_PARSER = engineBinary;

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
    throw new Error(`ethos-parser ${args.join(" ")} exited ${result.status}: ${result.stderr}`);
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
