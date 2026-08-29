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
 * c14n v1 in JavaScript — the same canonical serialization `ethos-parser-core/src/c14n.rs` writes,
 * and the same one `packages/python/src/ethos_parser/_c14n.py` writes.
 *
 * **Why this file exists at all.** `nodeGet` has no CLI subcommand behind it, so it is the one
 * function here that computes something rather than relaying it. What it computes is a
 * fingerprint, and a fingerprint that is *nearly* the engine's is worse than none: it would
 * accept an artifact the engine would refuse, or refuse one the engine minted, and either way a
 * caller would be told something false about a document.
 *
 * So this is a port and not an approximation. The properties are the Rust module's, in its order:
 *
 * - UTF-8, no whitespace between tokens
 * - object keys sorted by Unicode **code point**, explicitly at write time
 * - minimal escaping; no Unicode normalization
 * - integers only — any non-integer number is a hard error
 * - `|n| <= 2**53 - 1`
 *
 * `test/c14n.test.js` runs the Rust module's own parity vectors through this one, and
 * `test/cli-surface.test.js` re-canonicalizes a whole artifact the engine actually printed and
 * compares the bytes. The second is the load-bearing check.
 *
 * # Two places where JavaScript is not Python, stated rather than papered over
 *
 * 1. **JavaScript has one number type.** `JSON.parse("1.0")` yields the value `1`, which is
 *    indistinguishable from the integer `1` — the language has no float that is separately
 *    `1.0`. Python's parser keeps them apart, so `_c14n.py` can reject float-*shaped* text and
 *    this module cannot. What both reject identically is a value that is genuinely not an
 *    integer: `1.5` throws here at every depth, which is the property c14n actually needs. The
 *    unreachable half does not matter in practice, because the engine never prints `1.0` — Rust
 *    c14n forbids it, and the CLI's stdout *is* c14n bytes.
 * 2. **`JSON.parse` rejects `NaN` and `Infinity` outright**, where Python's accepts them by
 *    default and `_c14n.py` has to disarm that with `parse_constant`. Nothing to disarm here;
 *    and were such a value handed in directly, `Number.isInteger` refuses it.
 */

import { createHash } from "node:crypto";

/** The integer bound c14n v1 declares, matching `ethos_parser_core::MAX_SAFE_INT`. */
export const MAX_SAFE_INT = 9007199254740991;

/**
 * A value could not be canonicalized.
 *
 * Carries a deterministic message, exactly as `C14nError` does: the same offending value
 * produces the same text, so a failure is reproducible from the message alone.
 *
 * Deliberately **not** an `EngineError`. This module is a serializer and knows nothing about the
 * engine; `index.js` re-raises what escapes here as `NotARepresentation`, which is what
 * `_c14n.py` does too.
 */
export class CanonicalizationError extends Error {
  constructor(message) {
    super(message);
    this.name = "CanonicalizationError";
  }
}

// The named short escapes, which take precedence over the `\u00xx` form where they exist.
const ESCAPES = new Map([
  ['"', '\\"'],
  ["\\", "\\\\"],
  ["\b", "\\b"],
  ["\t", "\\t"],
  ["\n", "\\n"],
  ["\f", "\\f"],
  ["\r", "\\r"],
]);

/**
 * Serialize a JSON value to canonical bytes.
 *
 * @param {unknown} value
 * @returns {Buffer}
 * @throws {CanonicalizationError} on a non-integer number, an integer past `MAX_SAFE_INT`, a
 *   lone surrogate, or a value that is not JSON.
 */
export function c14nBytes(value) {
  const out = [];
  write(value, out);
  return Buffer.from(out.join(""), "utf8");
}

/**
 * Lowercase hex sha256 over the canonical bytes of `value`.
 *
 * @param {unknown} value
 * @returns {string}
 */
export function sha256Hex(value) {
  return createHash("sha256").update(c14nBytes(value)).digest("hex");
}

function write(value, out) {
  if (value === null) {
    out.push("null");
    return;
  }
  switch (typeof value) {
    case "boolean":
      out.push(value ? "true" : "false");
      return;
    case "number":
      // `Number.isInteger` is the whole test: it is false for 1.5, for NaN and for either
      // Infinity, so a value that is not a whole number never reaches the output. See the note
      // at the top of this file about what JavaScript cannot separate.
      if (!Number.isInteger(value)) {
        throw new CanonicalizationError("non-integer number in canonical value");
      }
      if (Math.abs(value) > MAX_SAFE_INT) {
        throw new CanonicalizationError("integer exceeds 2^53-1 in canonical value");
      }
      // `String(-0)` is already `"0"`, which is the spelling Rust would write for the same
      // mathematical value. JavaScript has no separate negative zero integer to preserve.
      out.push(String(value));
      return;
    case "string":
      writeString(value, out);
      return;
    case "bigint":
      // A BigInt is not a JSON value and cannot be one; silently narrowing it to a Number is a
      // repair, so it is an error instead.
      throw new CanonicalizationError("bigint is not a canonical JSON value");
    default:
      break;
  }

  if (Array.isArray(value)) {
    out.push("[");
    for (let i = 0; i < value.length; i += 1) {
      if (i > 0) out.push(",");
      write(value[i], out);
    }
    out.push("]");
    return;
  }

  if (typeof value === "object" && isPlainObject(value)) {
    // `Object.entries` rather than key-then-lookup: it reads own enumerable pairs directly, so a
    // key named `__proto__` is a key and never a prototype question.
    const entries = Object.entries(value);
    // Sorted HERE, at write time, and by CODE POINT. JavaScript's default sort compares UTF-16
    // code units, which disagrees with Rust's `String: Ord` for anything above the BMP — under
    // code-unit order a key starting U+10000 sorts before one starting U+FFFD, and under code
    // point order it sorts after. One such key would move a fingerprint.
    entries.sort((a, b) => compareCodePoints(a[0], b[0]));
    out.push("{");
    for (let i = 0; i < entries.length; i += 1) {
      if (i > 0) out.push(",");
      writeString(entries[i][0], out);
      out.push(":");
      write(entries[i][1], out);
    }
    out.push("}");
    return;
  }

  throw new CanonicalizationError(`${describe(value)} is not a canonical JSON value`);
}

/**
 * Minimal escaping. Non-ASCII is emitted literally and never normalized: extracted text is
 * evidence, and NFC-folding it would silently change bytes a citation may quote.
 */
function writeString(s, out) {
  out.push('"');
  // `for...of` iterates CODE POINTS, matching Rust's `chars()`. A lone surrogate arrives as a
  // single unit here, and is refused below rather than encoded as U+FFFD — a replacement
  // character is a silent repair, and Rust strings cannot hold the input that would produce one.
  for (const ch of s) {
    const escape = ESCAPES.get(ch);
    if (escape !== undefined) {
      out.push(escape);
      continue;
    }
    const cp = ch.codePointAt(0);
    if (cp >= 0xd800 && cp <= 0xdfff) {
      throw new CanonicalizationError(
        `lone surrogate U+${cp.toString(16).toUpperCase()} is not text this engine could have produced`,
      );
    }
    if (cp < 0x20) {
      out.push(`\\u${cp.toString(16).padStart(4, "0")}`);
      continue;
    }
    out.push(ch);
  }
  out.push('"');
}

/** Lexicographic comparison by Unicode code point — the contract sort. */
function compareCodePoints(a, b) {
  const left = a[Symbol.iterator]();
  const right = b[Symbol.iterator]();
  for (;;) {
    const x = left.next();
    const y = right.next();
    if (x.done && y.done) return 0;
    if (x.done) return -1;
    if (y.done) return 1;
    const cx = x.value.codePointAt(0);
    const cy = y.value.codePointAt(0);
    if (cx !== cy) return cx < cy ? -1 : 1;
  }
}

/**
 * Only plain objects are canonical JSON objects.
 *
 * A `Date`, a `Map` or a class instance is `typeof "object"` with no enumerable own keys, so a
 * looser check would serialize it as `{}` — a wrong answer that looks like a right one. This is
 * what `JSON.parse` produces and nothing else.
 */
function isPlainObject(value) {
  const proto = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

function describe(value) {
  if (value === undefined) return "undefined";
  const name = Object.getPrototypeOf(value)?.constructor?.name;
  return name ? `a ${name}` : typeof value;
}
