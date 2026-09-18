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
 * **It cannot diverge from what the CLI prints**, which is the whole claim S2 made and S3 inherits.
 *
 * The load-bearing test is the first one below: the SDK's return value is re-canonicalized and
 * compared against the bytes `ethos-parser extract` actually wrote to a pipe. Structural equality would
 * not do — it would pass a package that reordered a key on the way through, and reordering a key
 * is exactly how a fingerprint stops matching.
 *
 * The rest guards the conditions that make the claim cheap to keep: no runtime dependency, no
 * wrapped surface this slice refused, and a binary that is located rather than guessed at.
 */

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { pathToFileURL } from "node:url";

import { FIXTURE_PDF, PACKAGE_ROOT, REPO_ROOT, cli, clone, engineBinary } from "./helpers.js";
import * as sdk from "../src/index.js";
import {
  EngineFailed,
  EngineNotFound,
  NotARepresentation,
  extract,
  ground,
  locate,
} from "../src/index.js";
import { c14nBytes } from "../src/c14n.js";

/** What `import "ethos-parser"` reaches. The langchain subpath is deliberately not here. */
const DEFAULT_ENTRY_SOURCES = ["src/index.js", "src/c14n.js", "src/engine.js"];

/** Every source in the package, for the rules that bind regardless of entry point. */
const ALL_SOURCES = [...DEFAULT_ENTRY_SOURCES, "src/langchain.js"];

async function withTempDir(body) {
  const directory = await mkdtemp(join(tmpdir(), "ethos-node-test-"));
  try {
    return await body(directory);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// --- byte identity --------------------------------------------------------------------------

test("extract is byte-identical to the CLI", () => {
  const fromCli = cli("extract", FIXTURE_PDF);
  const artifact = extract(FIXTURE_PDF);
  assert.deepEqual(
    c14nBytes(artifact),
    fromCli,
    "byte identity across the adapter, not merely structural equality: a key reordered on the way through is how a fingerprint stops matching",
  );
  // And the artifact's own declared digest still covers its own payload — the check `nodeGet`
  // runs, on bytes that went out through a pipe and came back through a parser.
  assert.ok(sdk.nodeGet(artifact, artifact.representation.nodes[0].id));
});

test("extract is the same bytes on a second run", () => {
  assert.deepEqual(c14nBytes(extract(FIXTURE_PDF)), c14nBytes(extract(FIXTURE_PDF)));
});

test("ground is byte-identical to the CLI, from the object and from a path", async () => {
  const representation = extract(FIXTURE_PDF);
  await withTempDir((directory) => {
    const onDisk = join(directory, "representation.json");
    writeFileSync(onDisk, c14nBytes(representation));
    const fromCli = cli("ground", onDisk);

    // The object, the way `ground(extract(pdf))` reads.
    assert.deepEqual(c14nBytes(ground(representation)), fromCli);
    // And a path to bytes this engine wrote, which is the other half of what MCP accepts.
    assert.deepEqual(c14nBytes(ground(onDisk)), fromCli);
  });
});

test("ground lets the engine refuse an edited representation", () => {
  // The refusal is the engine's, not a second check here that could drift from it.
  const edited = clone(extract(FIXTURE_PDF));
  edited.representation.nodes[0].text = "Tampered";
  assert.throws(
    () => ground(edited),
    (error) => {
      assert.ok(error instanceof EngineFailed);
      assert.equal(error.status, 2);
      assert.match(error.stderr, /representation_c14n_sha256/);
      return true;
    },
  );
});

test("ground refuses an object that will not canonicalize", () => {
  const representation = clone(extract(FIXTURE_PDF));
  representation.representation.nodes[0].probe = 1.5;
  assert.throws(() => ground(representation), NotARepresentation);
});

/**
 * A file whose bytes are the quote's UTF-8 encoding, and nothing appended.
 *
 * What the SDK writes for itself, written here by hand, so the CLI reads exactly the bytes the
 * adapter sent rather than a file this test formatted differently.
 */
function quoteFile(directory, quote) {
  const path = join(directory, "quote.txt");
  writeFileSync(path, Buffer.from(quote, "utf8"));
  return path;
}

/**
 * The quote's scalar count, which is the unit `quote_scalars` counts in.
 *
 * NOT `String.prototype.length`: that is UTF-16 code units, and an astral character would make it
 * disagree with the engine by one. Decision #20 records the same defect on the other side of the
 * wire.
 */
function scalarCount(quote) {
  return [...quote].length;
}

test("locate is byte-identical to the CLI, from the object and from a path", async () => {
  const representation = extract(FIXTURE_PDF);
  await withTempDir((directory) => {
    const onDisk = join(directory, "representation.json");
    writeFileSync(onDisk, c14nBytes(representation));
    const fromCli = cli("locate", onDisk, "--quote-file", quoteFile(directory, "block"));

    // Re-canonicalizing what came back reproduces the CLI's stdout bytes, so the answer IS the
    // engine's. That is the whole of why the match rule is not ported into this package
    // (`docs/26-LOCATE-SCOPE.md` §6.3): a ported rule would be a second implementation of the
    // answer, and two implementations of a text-matching rule can disagree.
    assert.deepEqual(c14nBytes(locate(representation, "block")), fromCli);
    // And a path to bytes this engine wrote, which is the other half of what `ground` accepts.
    assert.deepEqual(c14nBytes(locate(onDisk, "block")), fromCli);
    // Two blocks carry the word, so byte identity is measured on a found answer and not only on
    // the empty one below — an adapter that dropped `occurrences` would pass that and fail this.
    assert.equal(locate(representation, "block").occurrences.length, 2);
  });
});

// **The test argv could not have passed**, which is why the quote travels in a file.
//
// `docs/26-LOCATE-SCOPE.md` §6.1: a NUL cannot appear in an argument at all, a newline survives
// only through correct quoting, and a quoting mistake changes the searched string *silently* —
// which changes what was searched with nothing on the wire saying so. So each case reads the
// number of scalars the engine searched for back off the artifact and compares it with the string
// that was handed in, and then compares the whole artifact against the CLI's own bytes for a file
// holding that same string.
for (const [label, quote] of [
  ["newline", "a\nb"],
  ["nul", "a\0b"],
  ["both", "one\ntwo\0three\n"],
]) {
  test(`a quote carrying a newline or a NUL survives the adapter (${label})`, async () => {
    const representation = extract(FIXTURE_PDF);
    await withTempDir((directory) => {
      const onDisk = join(directory, "representation.json");
      writeFileSync(onDisk, c14nBytes(representation));

      const artifact = locate(representation, quote);
      assert.equal(
        artifact.quote_scalars,
        scalarCount(quote),
        "the engine searched for a different number of scalars than the quote has, so the adapter did not deliver the string it was given",
      );
      assert.deepEqual(
        c14nBytes(artifact),
        cli("locate", onDisk, "--quote-file", quoteFile(directory, quote)),
        "and it is the artifact the CLI prints for a quote file holding those same bytes",
      );
    });
  });
}

test("a quote that occurs nowhere is an answer and not a failure", () => {
  // Decision #30's own bound: exit 0 and an empty array, never a refusal and never an exit 1. An
  // empty `occurrences` array is the whole of the not-found answer, and it is the same artifact a
  // found one produces — so nothing throws, and there is no field a caller could read as an
  // opinion about whether anything holds.
  const quote = "zzz-nowhere-zzz";
  const artifact = locate(extract(FIXTURE_PDF), quote);
  assert.equal(artifact.artifact_type, "ethos.parser.locations.v0");
  assert.deepEqual(artifact.occurrences, []);
  assert.equal(artifact.quote_scalars, scalarCount(quote));
  assert.ok(artifact.searched.blocks > 0, "a record nothing was searched in would report absence vacuously");
});

test("an empty quote is a refusal and not the not-found answer", () => {
  // §4.1: the empty string occurs at every offset of every block, so *where* has no answer. The
  // adapter keeps that apart from the empty answer above rather than collapsing the two —
  // collapsing them would tell a caller that a string occurring at every position occurs at none.
  assert.throws(
    () => locate(extract(FIXTURE_PDF), ""),
    (error) => {
      assert.ok(error instanceof EngineFailed);
      assert.equal(error.status, 2);
      assert.match(error.stderr, /empty/);
      return true;
    },
  );
});

test("a document the engine cannot read is a named failure", async () => {
  await withTempDir((directory) => {
    const notAPdf = join(directory, "document.pdf");
    writeFileSync(notAPdf, "this is not a PDF");
    assert.throws(
      () => extract(notAPdf),
      (error) => {
        assert.ok(error instanceof EngineFailed);
        assert.equal(error.status, 2);
        assert.ok(error.stderr.trim(), "the engine's own words must survive the wrapper");
        return true;
      },
    );
  });
});

test("stdout larger than one megabyte is not truncated", () => {
  // `spawnSync`'s default `maxBuffer` is 1 MiB and a real document's representation is bigger
  // than that. Silent truncation would arrive as a JSON parse error standing in for a size limit
  // nobody chose, so the option is set and this is the assertion that it is set.
  // Matched on the spawn option itself. The pattern this replaced, `maxBuffer: Infinity`, matched
  // only a doc comment once the option became a bound, and so asserted nothing.
  const source = readFileSync(join(PACKAGE_ROOT, "src", "engine.js"), "utf8");
  assert.match(source, /maxBuffer:\s*maxBuffer === 0 \? Infinity : maxBuffer/);
});

// --- the surface this slice refused ------------------------------------------------------------

test("the surfaces this slice did not wrap are absent", () => {
  // `markdown` and `html` arrive when a caller needs them; `verify` is a boundary.
  // `docs/07-VERIFY-BOUNDARY.md`: the engine invokes a verifier, it never verifies. A JavaScript
  // function of that name would look like this package had an opinion about whether a claim is
  // supported.
  for (const absent of ["markdown", "html", "verify", "mcp", "classify"]) {
    assert.equal(sdk[absent], undefined, `\`${absent}\` is exported`);
  }
});

test("no source file carries the vocabulary the repository bans", () => {
  // The tokens `ci/forbidden-tokens.sh` scans crate sources for, applied to this language too.
  // Scoped to `src/` for the reason that script scopes itself to `crates/*/src`: this file lists
  // the tokens as data, and scanning it would fail the build for containing the check.
  const banned = [
    "quality_score",
    "trust_score",
    "evidence_tier",
    "is_grounded",
    "verdict",
    "verify_claim",
    "claim_verified",
  ];
  assert.ok(ALL_SOURCES.length > 0, "an empty scan would pass vacuously");
  for (const relative of ALL_SOURCES) {
    const text = readFileSync(join(PACKAGE_ROOT, relative), "utf8").toLowerCase();
    for (const token of banned) {
      assert.equal(text.includes(token), false, `\`${token}\` appears in ${relative}`);
    }
  }
});

// --- no runtime dependency ----------------------------------------------------------------------

test("package.json declares no runtime dependency", () => {
  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"));
  for (const field of ["dependencies", "optionalDependencies"]) {
    assert.deepEqual(
      manifest[field] ?? {},
      {},
      `${field} must be empty; a package that pulled something to hand back bytes the CLI already printed adds a place for those bytes to change`,
    );
  }
  // v1.2-S4 added exactly one peer, and it is OPTIONAL: `@langchain/core`, reached only through
  // the `./langchain` subpath. A peer that were not optional would make every consumer install it
  // to use `extract`, which is the empty-install promise broken by another name.
  assert.deepEqual(Object.keys(manifest.peerDependencies ?? {}), ["@langchain/core"]);
  assert.equal(manifest.peerDependenciesMeta["@langchain/core"].optional, true);
  // No native addon, and no build step that could produce one.
  for (const field of ["gypfile", "binary", "devDependencies", "scripts"]) {
    const value = JSON.stringify(manifest[field] ?? "");
    for (const forbidden of ["napi", "neon", "node-gyp", "prebuild", "node-pre-gyp"]) {
      assert.equal(value.includes(forbidden), false, `${field} mentions ${forbidden}`);
    }
  }
});

test("every module the default entry point imports is a node: builtin", () => {
  // Read off the import statements, so the declaration above cannot be true only on paper.
  // Scoped to the DEFAULT entry: `src/langchain.js` reaches its optional peer, and the test below
  // is the one that bounds what it may reach.
  for (const relative of DEFAULT_ENTRY_SOURCES) {
    const source = readFileSync(join(PACKAGE_ROOT, relative), "utf8");
    const specifiers = [...source.matchAll(/^import[^"']*["']([^"']+)["']/gm)].map((m) => m[1]);
    assert.ok(specifiers.length > 0, `read no imports off ${relative}`);
    for (const specifier of specifiers) {
      assert.ok(
        specifier.startsWith("node:") || specifier.startsWith("./"),
        `${relative} imports \`${specifier}\`, which is neither a node: builtin nor this package`,
      );
    }
  }
});

test("the package resolves by name, with no install step and no lockfile", async () => {
  // `import "ethos-parser"` must reach the same module `./src/index.js` is. Resolved through the
  // manifest's own `exports` map rather than through a registry, because this package is not
  // published and has nothing to install.
  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"));
  const entry = manifest.exports["."];
  const resolved = await import(pathToFileURL(join(PACKAGE_ROOT, entry)).href);
  assert.equal(resolved.extract, sdk.extract);
  assert.equal(resolved.nodeGet, sdk.nodeGet);
  assert.equal(manifest.private, true, "not published, and mechanically so rather than by promise");
});

test("the langchain subpath reaches its peer and nothing else", () => {
  // `@langchain/core/tools` is the one non-builtin, non-relative specifier allowed anywhere in
  // this package — and it is imported DYNAMICALLY inside a try/catch, so a missing peer is a
  // named failure at load rather than Node's generic module-not-found.
  const source = readFileSync(join(PACKAGE_ROOT, "src", "langchain.js"), "utf8");

  const statics = [...source.matchAll(/^import[^"']*["']([^"']+)["']/gm)].map((m) => m[1]);
  for (const specifier of statics) {
    assert.ok(
      specifier.startsWith("node:") || specifier.startsWith("./"),
      `a static import of \`${specifier}\` would make the peer mandatory at load`,
    );
  }

  const dynamics = [...source.matchAll(/await import\(\s*["']([^"']+)["']/g)].map((m) => m[1]);
  assert.deepEqual(dynamics, ["@langchain/core/tools"]);
  assert.match(source, /npm install @langchain\/core/, "the failure must carry the install command");
});

// --- identity -------------------------------------------------------------------------------------

test("the SDK version is the workspace version", () => {
  // An SDK claiming a version the engine does not is the lie `parser_version` exists to stop.
  const cargo = readFileSync(join(REPO_ROOT, "Cargo.toml"), "utf8");
  const match = cargo.match(/^version = "([^"]+)"$/m);
  assert.ok(match, "no workspace version in Cargo.toml");
  assert.equal(sdk.version, match[1]);

  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"));
  assert.equal(manifest.version, match[1]);

  // And the Python SDK tracks the same number, because two adapters at different versions over
  // one binary is exactly the disagreement a version string exists to make legible.
  const python = readFileSync(
    join(REPO_ROOT, "packages", "python", "src", "ethos_parser", "__init__.py"),
    "utf8",
  );
  assert.match(python, new RegExp(`^__version__ = "${match[1]}"$`, "m"));
});

test("the representation artifact type is the one the engine stamps", () => {
  assert.equal(extract(FIXTURE_PDF).artifact_type, sdk.REPRESENTATION_ARTIFACT_TYPE);
});

// --- locating the binary ------------------------------------------------------------------------------

test("an explicit pin is authoritative", async (t) => {
  // A named path that is not there is an error, never a reason to find some other build.
  // `VerifierBinary::resolve` sets the precedent for `ETHOS_BIN` and the reason carries: resolving
  // to a binary nobody chose means returning artifacts from a parser nobody chose.
  const original = process.env.ETHOS_PARSER;
  t.after(() => {
    process.env.ETHOS_PARSER = original;
  });
  process.env.ETHOS_PARSER = join(tmpdir(), "no-such-engine");
  assert.throws(
    () => extract(FIXTURE_PDF),
    (error) => {
      assert.ok(error instanceof EngineNotFound);
      assert.match(error.message, /no-such-engine/);
      return true;
    },
  );
});

test("no binary anywhere is a named failure", async (t) => {
  const original = { engine: process.env.ETHOS_PARSER, path: process.env.PATH };
  t.after(() => {
    process.env.ETHOS_PARSER = original.engine;
    process.env.PATH = original.path;
  });
  delete process.env.ETHOS_PARSER;
  process.env.PATH = "";
  assert.throws(
    () => extract(FIXTURE_PDF),
    (error) => {
      assert.ok(error instanceof EngineNotFound);
      assert.match(error.message, /ETHOS_PARSER/);
      assert.match(error.message, /PATH/);
      assert.match(error.message, /cargo build/, "the failure must carry the command that fixes it");
      return true;
    },
  );
});

test("the engine inherits the environment rather than one this package composed", () => {
  // No `env:` anywhere: handing the engine an environment this package assembled would be one
  // more input to a deterministic parser that nobody declared.
  for (const file of ALL_SOURCES) {
    const source = readFileSync(join(PACKAGE_ROOT, file), "utf8");
    // Wherever the option appears in an object literal — `env: x`, shorthand `env,`, or one line —
    // and not in a comment, where `env` follows a backtick.
    assert.equal(/(^|[{,])\s*env\s*[:,}]/m.test(source), false, `a spawn call in ${file} sets \`env\``);
  }
});

test("it is the engine that runs", () => {
  // The SDK is another caller of the same binary, so the same invocation is available to a shell.
  const completed = spawnSync(engineBinary, ["extract", FIXTURE_PDF], { maxBuffer: Infinity });
  assert.equal(completed.status, 0);
  const stdout = completed.stdout.subarray(0, completed.stdout.length - 1);
  assert.deepEqual(c14nBytes(extract(FIXTURE_PDF)), stdout);
});
