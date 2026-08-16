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
 * c14n parity — the Rust module's own vectors, run through the JavaScript port.
 *
 * These are lifted verbatim from `crates/engine-core/src/c14n.rs`, which lifted them from Ethos's
 * committed vectors, which are cross-checked against a Python reference. The Python SDK runs the
 * same five. Matching them is what makes `nodeGet`'s fingerprint the engine's fingerprint rather
 * than one that resembles it.
 *
 * They are the cheap half of the proof. The load-bearing half is in `cli-surface.test.js`, where
 * a whole artifact the engine actually printed is re-canonicalized and compared byte for byte.
 */

import assert from "node:assert/strict";
import test from "node:test";

import { MAX_SAFE_INT, CanonicalizationError, c14nBytes, sha256Hex } from "../src/c14n.js";

const c14nStr = (value) => c14nBytes(value).toString("utf8");

// --- parity vectors ------------------------------------------------------------------------

test("parity: empty object", () => {
  assert.equal(c14nStr({}), "{}");
  assert.equal(sha256Hex({}), "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a");
});

test("parity: key order", () => {
  const value = { b: 2, a: 1, _: 0, Z: -3 };
  assert.equal(c14nStr(value), '{"Z":-3,"_":0,"a":1,"b":2}');
  assert.equal(
    sha256Hex(value),
    "9e8c5fa78b63297991b5b7b45bd334ccc61bd1058c5cd8ca6ee0451f78cd6cc1",
  );
});

test("parity: strings and ints", () => {
  const value = {
    text: 'líne1\nl"ine2\tend — \u{1F4A1}',
    n_zero: 0,
    n_neg: -42,
    arr: [3, 1, 2],
    flag: true,
    nothing: null,
  };
  assert.equal(
    c14nStr(value),
    '{"arr":[3,1,2],"flag":true,"n_neg":-42,"n_zero":0,"nothing":null,' +
      '"text":"líne1\\nl\\"ine2\\tend — \u{1F4A1}"}',
  );
  assert.equal(
    sha256Hex(value),
    "86b355efaa571cac1ddb71d422a9971e6042c55ec5369305cce095f2c181426e",
  );
});

test("parity: controls and backslash", () => {
  const value = { bel: "\u0007", backslash: "a\\b" };
  assert.equal(c14nStr(value), '{"backslash":"a\\\\b","bel":"\\u0007"}');
  assert.equal(
    sha256Hex(value),
    "a1cc2b96cfaf4e1d27ca13e7c2e56faadf76bd027d233fce5a57124e36ea6dfd",
  );
});

test("parity: fingerprint manifest", () => {
  const value = {
    config_sha256: "68cc61753d299917cc7773f069c18aca31c8ac68f43736a94cb57eee05144084",
    payload_sha256: "dad47d0ac4ab90f60691eb884c4c7e58d38ef7b87ef3df4bf602cd6087c9c757",
    profile_id: "ethos-deterministic-v1",
    profile_sha256: "d6145b9210845db39ad592ea549788432b52a649778c9947f5b2d91173e38070",
    schema_version: "1.0.0",
    source_fingerprint: "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef",
  };
  assert.equal(
    sha256Hex(value),
    "b5d30710d0c25cc38d8dec924ecaf57ae4f81276dd5dc14d75cb3b5b6bde62d3",
  );
});

// --- number discipline ---------------------------------------------------------------------

test("non-integers are rejected at every depth", () => {
  for (const value of [
    1.5,
    { x: 1.5 },
    [0.1],
    { a: { b: [1, 2, { c: 0.25 }] } },
    [[[[2.5]]]],
    { ok: 1, bad: [{ deep: -3.5 }] },
  ]) {
    assert.throws(() => c14nBytes(value), CanonicalizationError);
  }
});

test("a non-integer is an error, not a rounding", () => {
  assert.throws(() => c14nBytes({ x: 1.5 }), {
    name: "CanonicalizationError",
    message: "non-integer number in canonical value",
  });
  // The nearest integer must never appear in output as a silent repair.
  assert.equal(c14nStr({ x: 1 }), '{"x":1}');
});

test("NaN and either infinity are refused", () => {
  for (const value of [Number.NaN, Infinity, -Infinity, { n: Number.NaN }, [Infinity]]) {
    assert.throws(() => c14nBytes(value), CanonicalizationError);
  }
  // And they cannot arrive through the parser at all, where Python's accepts them by default.
  assert.throws(() => JSON.parse("NaN"), SyntaxError);
  assert.throws(() => JSON.parse("Infinity"), SyntaxError);
});

/**
 * **The one place JavaScript cannot follow the Rust, stated rather than papered over.**
 *
 * `crates/engine-core/src/c14n.rs` has `float_shaped_text_stays_rejected_regardless_of_number_
 * representation`, which asserts that JSON text like `1.0` or `2.0` is refused even though the
 * value is mathematically integral. Python's SDK ports it, because Python's parser stores those
 * as `float`. **JavaScript has one number type**: `JSON.parse("1.0")` yields the same value as
 * `JSON.parse("1")`, and there is no observation that separates them. Rejecting it would mean
 * rejecting the integer 1.
 *
 * This test pins the divergence so it stays a known, named property rather than a surprise. What
 * matters for c14n — that a value which is genuinely not a whole number never reaches the output
 * — is asserted above and holds identically in both languages. The unreachable half costs
 * nothing here: the engine never prints `1.0`, because Rust c14n forbids it and the CLI's stdout
 * *is* c14n bytes.
 */
test("float-shaped text is indistinguishable from an integer in this language", () => {
  for (const text of ["1.0", "2.0", "1e2", "-0.0"]) {
    const parsed = JSON.parse(text);
    assert.equal(Number.isInteger(parsed), true, `${text} parsed to a non-integer`);
    assert.equal(c14nStr(parsed), String(parsed));
  }
  assert.equal(c14nStr(JSON.parse("1.0")), "1");
  assert.equal(c14nStr(JSON.parse("-0.0")), "0");
});

test("integers at the 2^53 boundary", () => {
  assert.equal(c14nStr(MAX_SAFE_INT), String(MAX_SAFE_INT));
  assert.equal(c14nStr(-MAX_SAFE_INT), String(-MAX_SAFE_INT));
  for (const outOfRange of [MAX_SAFE_INT + 1, -MAX_SAFE_INT - 1]) {
    assert.throws(() => c14nBytes(outOfRange), CanonicalizationError);
  }
});

test("a bigint is refused rather than narrowed", () => {
  assert.throws(() => c14nBytes(1n), CanonicalizationError);
  assert.throws(() => c14nBytes({ n: 1n }), CanonicalizationError);
});

test("a boolean is not written as a number", () => {
  assert.equal(c14nStr({ flag: true, off: false }), '{"flag":true,"off":false}');
  assert.equal(c14nStr([true, 1, false, 0]), "[true,1,false,0]");
});

// --- escaping ------------------------------------------------------------------------------

test("control characters use lowercase four-digit escapes", () => {
  assert.equal(c14nStr("\u0001"), '"\\u0001"');
  assert.equal(c14nStr("\u001f"), '"\\u001f"');
  assert.equal(c14nStr("\u000b"), '"\\u000b"');
  // The named short escapes take precedence where they exist.
  assert.equal(c14nStr("\b"), '"\\b"');
  assert.equal(c14nStr("\t"), '"\\t"');
  assert.equal(c14nStr("\n"), '"\\n"');
  assert.equal(c14nStr("\f"), '"\\f"');
  assert.equal(c14nStr("\r"), '"\\r"');
});

test("non-ASCII is literal and never normalized", () => {
  // Space is the first code point at or above the escape threshold.
  assert.equal(c14nStr(" "), '" "');
  assert.equal(c14nStr("é"), '"é"');
  assert.equal(c14nStr("日本語"), '"日本語"');
  assert.equal(c14nStr("\u{1F4A1}"), '"\u{1F4A1}"');

  // Precomposed vs decomposed are different byte sequences and must stay different: normalizing
  // would rewrite evidence a citation may quote verbatim.
  const precomposed = c14nBytes("\u00e9");
  const decomposed = c14nBytes("\u0065\u0301");
  assert.notDeepEqual(precomposed, decomposed);
  assert.equal(precomposed.length, 4); // quote + 2 UTF-8 bytes + quote
  assert.equal(decomposed.length, 5); // quote + 1 + 2 UTF-8 bytes + quote
});

test("a lone surrogate is an error, not a replacement character", () => {
  // `Buffer.from` would encode this as U+FFFD, which is a silent repair of evidence. Rust
  // strings cannot hold the input that would produce one.
  assert.throws(() => c14nBytes("\ud800"), CanonicalizationError);
  assert.throws(() => c14nBytes({ "\udfff": 1 }), CanonicalizationError);
  // A well-formed pair is one code point and comes through untouched.
  assert.equal(c14nStr("\u{1F4A1}"), '"\u{1F4A1}"');
});

test("only quote and backslash are escaped above the control range", () => {
  assert.equal(c14nStr('"'), '"\\""');
  assert.equal(c14nStr("\\"), '"\\\\"');
  // Solidus is NOT escaped — `\/` would be valid JSON but different bytes.
  assert.equal(c14nStr("/"), '"/"');
});

test("keys are escaped by the same rule as values", () => {
  assert.equal(c14nStr({ "a\nb": 1 }), '{"a\\nb":1}');
});

// --- structure -----------------------------------------------------------------------------

test("no whitespace anywhere", () => {
  const text = c14nStr({ a: [1, 2, { b: "c" }], d: null });
  assert.equal(text.includes(" "), false);
  assert.equal(text.includes("\n"), false);
  assert.equal(text, '{"a":[1,2,{"b":"c"}],"d":null}');
});

test("array order is preserved because it is semantic", () => {
  assert.equal(c14nStr([3, 1, 2]), "[3,1,2]");
});

test("key order is by code point, not ASCII case and not UTF-16 code unit", () => {
  assert.equal(c14nStr({ a: 1, A: 2, _: 3, b: 4, B: 5 }), '{"A":2,"B":5,"_":3,"a":1,"b":4}');
  assert.equal(c14nStr({ "é": 1, z: 2, A: 3 }), '{"A":3,"z":2,"é":1}');

  // The case JavaScript's default sort gets wrong. U+1F4A1 is above the BMP, so its UTF-16 form
  // starts with 0xD83D — below 0xFFFD — and `Array.prototype.sort` would put it first. By code
  // point it sorts last, which is what Rust's `String: Ord` does.
  const value = { "\u{1F4A1}": 1, "\ufffd": 2 };
  assert.equal(c14nStr(value), '{"\ufffd":2,"\u{1F4A1}":1}');
  assert.notEqual(
    Object.keys(value).sort()[0],
    "\ufffd",
    "the default sort should disagree here, or this test proves nothing",
  );
});

test("nested objects are sorted at every level", () => {
  assert.equal(
    c14nStr({ z: { y: 1, x: 2 }, a: { c: 3, b: 4 } }),
    '{"a":{"b":4,"c":3},"z":{"x":2,"y":1}}',
  );
});

test("key insertion order is irrelevant", () => {
  const keys = ["q", "b", "zz", "a"];
  const forward = Object.fromEntries(keys.map((k) => [k, 1]));
  const backward = Object.fromEntries([...keys].reverse().map((k) => [k, 1]));
  assert.deepEqual(c14nBytes(forward), c14nBytes(backward));
});

test("c14n is idempotent", () => {
  const value = { z: [1, { b: "x\ty", a: null }], "é": true, n: -7 };
  const once = c14nBytes(value);
  const twice = c14nBytes(JSON.parse(once.toString("utf8")));
  assert.deepEqual(once, twice);
});

test("a value that is not JSON is an error rather than a shape", () => {
  // Each of these is `typeof "object"` with no enumerable own keys, so a looser check would
  // serialize it as `{}` — a wrong answer that looks like a right one.
  for (const value of [new Date(0), new Map(), new Set(), undefined, () => {}, Symbol("s")]) {
    assert.throws(() => c14nBytes(value), CanonicalizationError);
  }
  assert.throws(() => c14nBytes({ nested: new Date(0) }), CanonicalizationError);
});
