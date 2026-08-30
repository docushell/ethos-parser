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
 * The handle law in Node — `docs/history/12-V12-SCOPE.md` §3, as executables.
 *
 * | handed to `nodeGet` | expected |
 * | --- | --- |
 * | a node id the engine minted, copied off the artifact | the node record |
 * | an id nothing minted | **throws**, never `null` and never `{}` |
 * | a representation whose payload was edited on the way through | **throws**, at the fingerprint |
 *
 * The same three rows `packages/python/tests/test_handle_law.py` asserts, because this is one
 * surface in two languages rather than two designs.
 *
 * Plus the corollary that decides the signatures: **no exported function parameter names document
 * geometry.** That one is read out of `Function.prototype.toString` rather than off a reviewer's
 * memory, for the same reason the MCP suite reads it off the advertised schemas.
 */

import assert from "node:assert/strict";
import test from "node:test";

import { FIXTURE_PDF, clone, parameterNames } from "./helpers.js";
import * as sdk from "../src/index.js";
import {
  EngineError,
  FingerprintMismatch,
  NodeNotFound,
  NotARepresentation,
  extract,
  ground,
  nodeGet,
} from "../src/index.js";

/**
 * The same list `mcp.rs`, `mcp_stdio.rs` and Python's `BANNED_ARGUMENT_NAMES` ban, so the three
 * adapters cannot drift on what counts as a locator a caller could compose.
 */
const BANNED_ARGUMENT_NAMES = [
  "page",
  "bbox",
  "box",
  "rect",
  "x",
  "y",
  "w",
  "h",
  "width",
  "height",
  "row",
  "column",
  "col",
  "span",
  "offset",
  "coords",
  "region",
];

const representation = extract(FIXTURE_PDF);

/**
 * Read off the artifact rather than hardcoded. A hardcoded id that stopped existing would make
 * the forged-id half pass for the wrong reason.
 */
const mintedId = (() => {
  const nodes = representation.representation.nodes;
  assert.ok(nodes.length > 0, "extract minted no nodes; every assertion below would pass vacuously");
  return nodes[0].id;
})();

function edited() {
  const copy = clone(representation);
  copy.representation.nodes[0].text = "Tampered";
  return copy;
}

// --- mint, opaque, re-validate ---------------------------------------------------------------

test("a minted handle returns the node that artifact carries", () => {
  const node = nodeGet(representation, mintedId);
  assert.equal(node.id, mintedId);
  assert.equal(
    node,
    representation.representation.nodes[0],
    "the node record must be the one the artifact carries, verbatim — not a copy this package assembled from it",
  );
});

test("a forged handle fails closed", () => {
  assert.throws(
    () => nodeGet(representation, "s-forged"),
    (error) => {
      assert.ok(error instanceof NodeNotFound);
      assert.match(error.message, /s-forged/, "the refusal must name what was refused");
      assert.match(error.message, /is not a node/);
      assert.equal(error.nodeId, "s-forged");
      return true;
    },
  );
});

test("an edited payload fails at the fingerprint before any lookup", () => {
  assert.throws(
    () => nodeGet(edited(), mintedId),
    (error) => {
      assert.ok(error instanceof FingerprintMismatch);
      // Both digests, as `verify_fingerprint` names them, so the failure is diagnosable.
      assert.equal(error.declared, representation.representation_c14n_sha256);
      assert.notEqual(error.actual, error.declared);
      assert.match(error.message, new RegExp(error.declared));
      assert.match(error.message, new RegExp(error.actual));
      return true;
    },
  );
});

test("an edited payload fails even when the id is forged too", () => {
  // The fingerprint is checked first, so an edited artifact never reaches the lookup.
  assert.throws(() => nodeGet(edited(), "s-forged"), FingerprintMismatch);
});

test("a thing that is not a representation is refused rather than coerced", () => {
  for (const value of [
    null,
    undefined,
    {},
    [],
    "s1",
    42,
    { artifact_type: "ethos.grounding.v1", elements: [] },
    { artifact_type: "ethos.parser.representation.v0" },
  ]) {
    assert.throws(() => nodeGet(value, "s1"), NotARepresentation);
  }
});

test("a projection is not a representation", () => {
  // `ground`'s output carries locators too, and it is still not the record they were minted in.
  const projection = ground(representation);
  assert.equal(projection.artifact_type, "ethos.grounding.v1");
  assert.throws(() => nodeGet(projection, "s1"), NotARepresentation);
});

test("a payload that will not canonicalize is refused", () => {
  // c14n v1 admits no non-integer, and a payload it refuses cannot be the one that digest covers.
  // Same news as a mismatch, so it is an `EngineError` and not a bare `CanonicalizationError`
  // leaking out of the serializer.
  const copy = clone(representation);
  copy.representation.nodes[0].probe = 1.5;
  assert.throws(() => nodeGet(copy, "s1"), NotARepresentation);
});

test("no refusal is ever an empty answer", () => {
  // Every way of getting it wrong throws. None of them returns `null`, `undefined` or `{}`.
  // An empty answer would tell a caller its guess was merely unlucky; a throw tells it the guess
  // was not admissible.
  const cases = [
    [representation, "s-forged"],
    [representation, ""],
    [representation, `${mintedId}-almost`],
    [edited(), mintedId],
    [{}, mintedId],
  ];
  for (const [artifact, nodeId] of cases) {
    assert.throws(() => nodeGet(artifact, nodeId), EngineError);
  }
});

// --- the corollary: no argument names a locator -----------------------------------------------

test("no exported function signature names a coordinate", () => {
  // A `page` or a `bbox` here would let a caller author the locator the engine then speaks for,
  // which is the hazard memo §16.7 says decides whether an adapter is worth having.
  const functions = Object.entries(sdk).filter(([, value]) => typeof value === "function");
  const plain = functions.filter(([name]) => !/^[A-Z]/.test(name));
  assert.equal(plain.length, 3, `expected exactly extract/ground/nodeGet, saw ${plain.length}`);

  for (const [name, fn] of plain) {
    const names = parameterNames(fn);
    assert.ok(names.length > 0, `read no parameters off ${name}; the ban would be vacuous`);
    for (const banned of BANNED_ARGUMENT_NAMES) {
      assert.ok(
        !names.includes(banned),
        `\`${name}\` takes \`${banned}\`, which lets a caller compose a locator`,
      );
    }
  }
});

test("the public surface is the three functions and nothing else", () => {
  const plain = Object.entries(sdk)
    .filter(([name, value]) => typeof value === "function" && !/^[A-Z]/.test(name))
    .map(([name]) => name)
    .sort();
  assert.deepEqual(plain, ["extract", "ground", "nodeGet"]);
});

test("the parameter reader recovers the real names, or it proves nothing", () => {
  assert.deepEqual(parameterNames(extract), ["pdfPath"]);
  assert.deepEqual(parameterNames(ground), ["representation"]);
  assert.deepEqual(parameterNames(nodeGet), ["representation", "nodeId"]);
});

test("the error taxonomy is the Python one, name for name", () => {
  // If Node disagreed with Python about how a failure is spelled, Node would be the one that is
  // wrong. `EngineError` is the base of every failure the three functions raise.
  for (const name of [
    "EngineError",
    "EngineNotFound",
    "EngineFailed",
    "NotARepresentation",
    "FingerprintMismatch",
    "NodeNotFound",
  ]) {
    assert.equal(typeof sdk[name], "function", `${name} is not exported`);
    assert.ok(
      name === "EngineError" || sdk[name].prototype instanceof EngineError,
      `${name} does not extend EngineError`,
    );
  }
});
