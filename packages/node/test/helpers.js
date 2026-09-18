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
 *
 * And a suite that went green against the *wrong build* of the right version is the same outcome
 * wearing a version number, which is why {@link locateBinary} checks a capability and not only a
 * number. It has happened here; the measurement is in that function's own comment.
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
 * The subcommands this suite actually drives — every `cli(...)` call in this package's tests and
 * every argument vector the SDK's own `run` builds. A binary that cannot answer them is not this
 * tree's, whatever its `--version` says.
 *
 * `node_get` is deliberately absent: it has no subcommand, being ported into both packages and
 * carried over MCP, which is why `mcp` is on the list instead. Keep this list to what the suite
 * calls — a longer one would reject a binary over a capability nothing here exercises.
 */
const REQUIRED_SUBCOMMANDS = ["extract", "ground", "locate", "mcp"];

/**
 * The subcommand names `--help` lists, or `null` if it will not run or lists none.
 *
 * One spawn per candidate. clap prints every subcommand in a `Commands:` block, one per line, so
 * the block is read and the first token of each line taken. Reading only that block rather than
 * every indented line in the output is the difference between a check that fails closed and one a
 * wrapped description could satisfy by accident.
 */
function subcommands(path) {
  const result = spawnSync(path, ["--help"], { encoding: "utf8" });
  if (result.error || result.status !== 0) return null;

  const lines = result.stdout.split("\n");
  const start = lines.indexOf("Commands:");
  if (start === -1) return null;
  const names = new Set();
  for (const line of lines.slice(start + 1)) {
    if (!line.trim()) break;
    names.add(line.trim().split(/\s+/)[0]);
  }
  // Nothing read is "could not read", never "carries nothing": the caller turns `null` into every
  // requirement being missing, so an unparseable help text rejects rather than passes.
  return names.size ? names : null;
}

/** Which of {@link REQUIRED_SUBCOMMANDS} this binary does not carry, in the listed order. */
function missingSubcommands(path) {
  if (REQUIRED_SUBCOMMANDS.length === 0) {
    throw new Error("an empty requirement list would make this guard vacuous");
  }
  const listed = subcommands(path);
  if (listed === null) return [...REQUIRED_SUBCOMMANDS];
  return REQUIRED_SUBCOMMANDS.filter((name) => !listed.has(name));
}

/**
 * Find a binary that is **this workspace's**, and refuse one that is not — by version *and* by
 * capability.
 *
 * The version check is not decoration. A stale `target/release/ethos-parser` is preferred over
 * `target/debug` and answers every question plausibly, so a suite that took the first file it
 * found would compare the SDK against a parser from six minor versions ago and go green — it
 * would still prove the SDK does not alter what the CLI prints, but about the wrong CLI.
 * Measured, not feared: it is what a `0.11.0` release build did here at v1.2-S4.
 *
 * **And the version check alone does not make that impossible, which this comment used to claim
 * it did.** A version string cannot distinguish two builds of one *unreleased* version. Measured
 * on 2026-09-18, while `locate` was being added at workspace 0.58.0 unreleased: a `target/release`
 * build from 00:45 reported `0.58.0`, exactly the number the tree wanted, and was preferred over
 * the `target/debug` build that actually had `locate` in it. That release build had no `locate`
 * subcommand and advertised three MCP tools instead of four — and **both suites went green against
 * it**, including the parity test whose entire job is to fail when the server advertises a tool the
 * LangChain adapters do not wrap. The version had not moved, so nothing about a version could have
 * caught it.
 *
 * So a candidate is asked what it can *do* as well as what it is called: {@link
 * REQUIRED_SUBCOMMANDS} is the list this suite drives, read off one `--help` run, and a candidate
 * missing any of them falls through exactly as a version mismatch does. The final error names
 * staleness apart from a version problem, because on the measured failure there was no version
 * problem to go looking for, and a message about versions would have sent the reader after the
 * wrong cause.
 */
function locateBinary() {
  const want = workspaceVersion();

  const pinned = process.env.ETHOS_PARSER;
  if (pinned) {
    // An explicit pin is authoritative in both directions: it is never silently overridden, and a
    // pin that is the wrong build is an error rather than a reason to look elsewhere. Both halves
    // of "wrong build" apply — a pinned stale binary is an error for the same reason, and passing
    // it silently is the failure measured above.
    const got = binaryVersion(pinned);
    if (got !== want) {
      throw new Error(
        `ETHOS_PARSER=${JSON.stringify(pinned)} reports ${got ?? "nothing"} but this workspace ` +
          `is ${want}. Rebuild it, or point ETHOS_PARSER at a build of this tree.`,
      );
    }
    const missing = missingSubcommands(pinned);
    if (missing.length > 0) {
      throw new Error(
        `ETHOS_PARSER=${JSON.stringify(pinned)} reports ${want}, which is this workspace's ` +
          `version, but does not carry: ${missing.map((m) => `\`${m}\``).join(", ")}. ` +
          "**This is staleness, not a version problem** — it is a build of this tree from " +
          "before those subcommands existed, and its `--version` cannot say so. Rebuild it:" +
          "\n\n    cargo build --locked\n\n" +
          "A pin is authoritative in both directions, so this is an error rather than a reason " +
          "to look elsewhere.",
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
  let stale = false;
  for (const candidate of candidates) {
    const got = binaryVersion(candidate);
    if (got !== want) {
      tried.push(`  ${candidate} (version ${got ?? "absent"})`);
      continue;
    }
    const missing = missingSubcommands(candidate);
    if (missing.length === 0) return candidate;
    // Right version, wrong build. Named distinctly from a version mismatch above, because the two
    // need different fixes and the message is all a reader gets.
    stale = true;
    tried.push(`  ${candidate} (version ${got}, but STALE: no ${missing.join(", ")})`);
  }

  throw new Error(
    `no \`engine\` binary at ${want} carrying ${REQUIRED_SUBCOMMANDS.join(", ")}. Tried, in ` +
      `order:\n${tried.join("\n") || "  (nothing)"}\n\n` +
      "    cargo build --locked\n\n" +
      "or point ETHOS_PARSER at a build of this tree. This is a failure and not a skip: a green " +
      "run against a parser from another version would prove something about the wrong engine." +
      (stale
        ? "\n\nA candidate marked STALE reported the right version and is still the wrong " +
          "build — a version string cannot tell two builds of one unreleased version apart. " +
          "Rebuild it rather than looking for a version problem you do not have."
        : ""),
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
