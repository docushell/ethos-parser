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
 * **It cannot diverge from what the CLI prints**, which is the whole claim S2 made and S3 inherits.
 *
 * The load-bearing test is the first one below: the SDK's return value is re-canonicalized and
 * compared against the bytes `engine extract` actually wrote to a pipe. Structural equality would
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
import { EngineFailed, EngineNotFound, NotARepresentation, extract, ground } from "../src/index.js";
import { c14nBytes } from "../src/c14n.js";

const SOURCE_FILES = ["src/index.js", "src/c14n.js"];

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
  const source = readFileSync(join(PACKAGE_ROOT, "src", "index.js"), "utf8");
  assert.match(source, /maxBuffer:\s*Infinity/);
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
  assert.ok(SOURCE_FILES.length > 0, "an empty scan would pass vacuously");
  for (const relative of SOURCE_FILES) {
    const text = readFileSync(join(PACKAGE_ROOT, relative), "utf8").toLowerCase();
    for (const token of banned) {
      assert.equal(text.includes(token), false, `\`${token}\` appears in ${relative}`);
    }
  }
});

// --- no runtime dependency ----------------------------------------------------------------------

test("package.json declares no runtime dependency", () => {
  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"));
  for (const field of ["dependencies", "optionalDependencies", "peerDependencies"]) {
    assert.deepEqual(
      manifest[field] ?? {},
      {},
      `${field} must be empty; a package that pulled something to hand back bytes the CLI already printed adds a place for those bytes to change`,
    );
  }
  // No native addon, and no build step that could produce one.
  for (const field of ["gypfile", "binary", "devDependencies", "scripts"]) {
    const value = JSON.stringify(manifest[field] ?? "");
    for (const forbidden of ["napi", "neon", "node-gyp", "prebuild", "node-pre-gyp"]) {
      assert.equal(value.includes(forbidden), false, `${field} mentions ${forbidden}`);
    }
  }
});

test("every module this package imports is a node: builtin", () => {
  // Read off the import statements, so the declaration above cannot be true only on paper.
  for (const relative of SOURCE_FILES) {
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
  // `import "ethos-engine"` must reach the same module `./src/index.js` is. Resolved through the
  // manifest's own `exports` map rather than through a registry, because this package is not
  // published and has nothing to install.
  const manifest = JSON.parse(readFileSync(join(PACKAGE_ROOT, "package.json"), "utf8"));
  const entry = manifest.exports["."];
  const resolved = await import(pathToFileURL(join(PACKAGE_ROOT, entry)).href);
  assert.equal(resolved.extract, sdk.extract);
  assert.equal(resolved.nodeGet, sdk.nodeGet);
  assert.equal(manifest.private, true, "not published, and mechanically so rather than by promise");
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
    join(REPO_ROOT, "packages", "python", "src", "ethos_engine", "__init__.py"),
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
  const original = process.env.ETHOS_ENGINE;
  t.after(() => {
    process.env.ETHOS_ENGINE = original;
  });
  process.env.ETHOS_ENGINE = join(tmpdir(), "no-such-engine");
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
  const original = { engine: process.env.ETHOS_ENGINE, path: process.env.PATH };
  t.after(() => {
    process.env.ETHOS_ENGINE = original.engine;
    process.env.PATH = original.path;
  });
  delete process.env.ETHOS_ENGINE;
  process.env.PATH = "";
  assert.throws(
    () => extract(FIXTURE_PDF),
    (error) => {
      assert.ok(error instanceof EngineNotFound);
      assert.match(error.message, /ETHOS_ENGINE/);
      assert.match(error.message, /PATH/);
      assert.match(error.message, /cargo build/, "the failure must carry the command that fixes it");
      return true;
    },
  );
});

test("the engine inherits the environment rather than one this package composed", () => {
  // No `env:` anywhere: handing the engine an environment this package assembled would be one
  // more input to a deterministic parser that nobody declared.
  const source = readFileSync(join(PACKAGE_ROOT, "src", "index.js"), "utf8");
  assert.equal(/^\s*env:/m.test(source), false, "a spawn call sets `env:`");
});

test("it is the engine that runs", () => {
  // The SDK is another caller of the same binary, so the same invocation is available to a shell.
  const completed = spawnSync(engineBinary, ["extract", FIXTURE_PDF], { maxBuffer: Infinity });
  assert.equal(completed.status, 0);
  const stdout = completed.stdout.subarray(0, completed.stdout.length - 1);
  assert.deepEqual(c14nBytes(extract(FIXTURE_PDF)), stdout);
});
