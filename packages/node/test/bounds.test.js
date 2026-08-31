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
 * **`spawnSync` had `maxBuffer: Infinity` and no `timeout`** (v2-S15).
 *
 * The old comment criticised Node's one-megabyte default as "a size limit nobody chose", which is
 * right, and then concluded no limit at all — the answer to an arbitrary default is a CHOSEN
 * ceiling. Unbounded, the artifact is buffered, copied by `.toString("utf8")` and expanded again
 * by `JSON.parse`, so peak is several times the on-wire size with nothing bounding any of it.
 * And with no timeout, a document that stalled the engine hung the caller forever.
 *
 * Both bounds are overridable, and `0` restores the previous behaviour by name rather than by
 * accident.
 */

import { test } from "node:test";
import assert from "node:assert/strict";

import { FIXTURE_PDF } from "./helpers.js";
import * as sdk from "../src/index.js";

/** Run `body` with env vars set, restoring them afterwards whatever happens. */
function withEnv(vars, body) {
  const saved = new Map(Object.keys(vars).map((k) => [k, process.env[k]]));
  Object.entries(vars).forEach(([k, v]) => {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  });
  try {
    return body();
  } finally {
    for (const [k, v] of saved) {
      if (v === undefined) delete process.env[k];
      else process.env[k] = v;
    }
  }
}

test("both new failures are EngineErrors, so `catch (e instanceof EngineError)` stays complete", () => {
  assert.ok(new sdk.EngineTimeout(["extract"], 1, "") instanceof sdk.EngineError);
  assert.ok(new sdk.ArtifactTooLarge(["extract"], 1) instanceof sdk.EngineError);
});

test("a timeout is not a refusal", () => {
  // A refusal is an answer; a timeout is the absence of one. Catching them as one another would
  // put "this PDF is encrypted" and "this never came back" in the same branch.
  const timeout = new sdk.EngineTimeout(["extract"], 1, "");
  assert.ok(!(timeout instanceof sdk.EngineFailed));
  assert.ok(!(new sdk.EngineFailed(["extract"], 2, "") instanceof sdk.EngineTimeout));
});

test("an expired budget throws EngineTimeout", () => {
  // One millisecond is not enough for any document, which is what makes this deterministic.
  withEnv({ ETHOS_PARSER_TIMEOUT: "0.001" }, () => {
    assert.throws(() => sdk.extract(FIXTURE_PDF), (e) => {
      assert.ok(e instanceof sdk.EngineTimeout, `got ${e.name}: ${e.message}`);
      assert.match(e.message, /did not finish within/);
      assert.deepEqual(e.commandArgs, ["extract", FIXTURE_PDF]);
      return true;
    });
  });
});

test("a stdout ceiling that bites throws ArtifactTooLarge, not a JSON parse error", () => {
  // The failure the old comment feared — a truncated read arriving as a confusing parse error —
  // is exactly what this named error exists to prevent.
  withEnv({ ETHOS_PARSER_MAX_BYTES: "64" }, () => {
    assert.throws(() => sdk.extract(FIXTURE_PDF), (e) => {
      assert.ok(e instanceof sdk.ArtifactTooLarge, `got ${e.name}: ${e.message}`);
      assert.equal(e.limit, 64);
      return true;
    });
  });
});

test("the defaults do not fire on a real document", () => {
  // A ceiling that trips on the corpus would be a performance budget by accident.
  withEnv({ ETHOS_PARSER_TIMEOUT: undefined, ETHOS_PARSER_MAX_BYTES: undefined }, () => {
    const artifact = sdk.extract(FIXTURE_PDF);
    assert.equal(artifact.artifact_type, sdk.REPRESENTATION_ARTIFACT_TYPE);
  });
});

test("zero restores the old unbounded behaviour, by name", () => {
  withEnv({ ETHOS_PARSER_TIMEOUT: "0", ETHOS_PARSER_MAX_BYTES: "0" }, () => {
    const artifact = sdk.extract(FIXTURE_PDF);
    assert.equal(artifact.artifact_type, sdk.REPRESENTATION_ARTIFACT_TYPE);
  });
});

for (const bad of ["soon", "-1"]) {
  test(`an unreadable bound is a named error, not a silent default (${bad})`, () => {
    // Falling back would hide a typo in the one knob that bounds a hang.
    withEnv({ ETHOS_PARSER_TIMEOUT: bad }, () => {
      assert.throws(() => sdk.extract(FIXTURE_PDF), sdk.EngineError);
    });
  });
}
