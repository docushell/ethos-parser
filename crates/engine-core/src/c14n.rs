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

//! c14n v1 — the one canonical JSON serialization (`docs/01-CONTRACT.md` §4).
//!
//! A clean-room implementation of the same contract Ethos implements, so the two produce
//! byte-identical output for the same value. The parity vectors in this module's tests are
//! Ethos's own committed vectors, independently re-derived from the Python reference in
//! `ethos/docs/determinism-contract.md` §2.
//!
//! Note what that does and does not buy: these vectors catch drift **on this side**. Ethos
//! changing its implementation would not fail anything here — it would fail Ethos's own copy of
//! the same vectors, which is where it belongs. The M6 oracle test is what actually compares
//! the two systems at runtime.
//!
//! Properties, all tested below:
//!
//! - UTF-8, no whitespace between tokens
//! - object keys sorted by Unicode code point, **explicitly at write time**
//! - minimal escaping; no Unicode normalization
//! - integers only — any non-integer number is a hard error
//! - `|n| ≤ 2^53 − 1`
//! - idempotent: `c14n(parse(c14n(v))) == c14n(v)`
//!
//! No other module in this workspace hand-rolls output JSON. One serializer means one place
//! where byte identity can break.

use serde_json::Value;
use sha2::{Digest, Sha256};

pub use crate::geom::MAX_SAFE_INT;

/// A value could not be canonicalized.
///
/// Carries a deterministic message: the same offending value produces the same text, so a
/// failure is reproducible from the message alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C14nError {
    message: String,
}

impl C14nError {
    /// Construct with a deterministic message.
    ///
    /// Public so callers that canonicalize their own values (for example [`crate::Profile`])
    /// can report a serialization failure in the same currency, rather than unwrapping and
    /// turning a contract violation into a panic.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The deterministic failure message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl core::fmt::Display for C14nError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for C14nError {}

fn err(message: &str) -> C14nError {
    C14nError {
        message: message.to_string(),
    }
}

/// Serialize a JSON value to canonical bytes.
///
/// Object keys are sorted **explicitly here**, never by relying on the map's own iteration
/// order. That is not defensive style, it is a specific hazard: `serde_json`'s `preserve_order`
/// feature is additive, so any crate anywhere in the final dependency graph enabling it would
/// switch `Map` to insertion order and silently change every fingerprint this crate produces.
/// Sorting at write time makes the output correct under either map flavour, and
/// `tests/preserve_order.rs` builds the whole workspace with that feature forced on to prove it.
///
/// # Errors
///
/// Returns [`C14nError`] if the value contains a non-integer number, or an integer whose
/// magnitude exceeds [`MAX_SAFE_INT`].
pub fn c14n_bytes(value: &Value) -> Result<Vec<u8>, C14nError> {
    let mut out = Vec::with_capacity(256);
    write_value(value, &mut out)?;
    Ok(out)
}

fn write_value(value: &Value, out: &mut Vec<u8>) -> Result<(), C14nError> {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                // `unsigned_abs`, not `abs`: `i64::MIN.abs()` overflows — it panics in debug and
                // wraps in release, where the wrapped value would slip past a naive range check.
                if i.unsigned_abs() > MAX_SAFE_INT as u64 {
                    return Err(err("integer exceeds 2^53-1 in canonical value"));
                }
                out.extend_from_slice(i.to_string().as_bytes());
            } else if let Some(u) = n.as_u64() {
                if u > MAX_SAFE_INT as u64 {
                    return Err(err("integer exceeds 2^53-1 in canonical value"));
                }
                out.extend_from_slice(u.to_string().as_bytes());
            } else {
                return Err(err("non-integer number in canonical value"));
            }
        }
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_value(item, out)?;
            }
            out.push(b']');
        }
        Value::Object(map) => {
            out.push(b'{');
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            // String `Ord` is Unicode code point order — exactly the contract sort.
            entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
            for (i, (k, v)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_string(k, out);
                out.push(b':');
                write_value(v, out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// Minimal escaping. Non-ASCII is emitted literally and never normalized: extracted text is
/// evidence, and NFC-folding it would silently change bytes a citation may quote.
fn write_string(s: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for c in s.chars() {
        match c {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{0008}' => out.extend_from_slice(b"\\b"),
            '\t' => out.extend_from_slice(b"\\t"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\u{000C}' => out.extend_from_slice(b"\\f"),
            '\r' => out.extend_from_slice(b"\\r"),
            c if (c as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes());
            }
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// Lowercase hex sha256 over the canonical bytes of `value`.
///
/// # Errors
///
/// Propagates [`C14nError`] from [`c14n_bytes`].
pub fn sha256_hex(value: &Value) -> Result<String, C14nError> {
    Ok(hex(&Sha256::digest(c14n_bytes(value)?)))
}

/// Lowercase hex sha256 over raw bytes, for source-artifact identity.
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(digest: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::json;

    fn c14n_str(v: &Value) -> String {
        String::from_utf8(c14n_bytes(v).unwrap()).unwrap()
    }

    // --- parity vectors -------------------------------------------------------------------
    //
    // Lifted from Ethos's committed c14n tests (`ethos-core/src/c14n.rs`), which are themselves
    // cross-checked against a Python reference. Matching these hashes is the practical proof
    // that this clean-room implementation is byte-compatible with Ethos c14n v1 — without
    // taking a Cargo dependency on Ethos.

    #[test]
    fn parity_empty_object() {
        let v = json!({});
        assert_eq!(c14n_str(&v), "{}");
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }

    #[test]
    fn parity_key_order() {
        let v = json!({"b": 2, "a": 1, "_": 0, "Z": -3});
        assert_eq!(c14n_str(&v), r#"{"Z":-3,"_":0,"a":1,"b":2}"#);
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "9e8c5fa78b63297991b5b7b45bd334ccc61bd1058c5cd8ca6ee0451f78cd6cc1"
        );
    }

    #[test]
    fn parity_strings_and_ints() {
        let v = json!({
            "text": "líne1\nl\"ine2\tend — \u{1F4A1}",
            "n_zero": 0, "n_neg": -42, "arr": [3, 1, 2], "flag": true, "nothing": null
        });
        assert_eq!(
            c14n_str(&v),
            "{\"arr\":[3,1,2],\"flag\":true,\"n_neg\":-42,\"n_zero\":0,\"nothing\":null,\"text\":\"líne1\\nl\\\"ine2\\tend — \u{1F4A1}\"}"
        );
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "86b355efaa571cac1ddb71d422a9971e6042c55ec5369305cce095f2c181426e"
        );
    }

    #[test]
    fn parity_controls_and_backslash() {
        let v = json!({"bel": "\u{0007}", "backslash": "a\\b"});
        assert_eq!(
            c14n_str(&v),
            "{\"backslash\":\"a\\\\b\",\"bel\":\"\\u0007\"}"
        );
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "a1cc2b96cfaf4e1d27ca13e7c2e56faadf76bd027d233fce5a57124e36ea6dfd"
        );
    }

    #[test]
    fn parity_fingerprint_manifest() {
        let v = json!({
            "config_sha256": "68cc61753d299917cc7773f069c18aca31c8ac68f43736a94cb57eee05144084",
            "payload_sha256": "dad47d0ac4ab90f60691eb884c4c7e58d38ef7b87ef3df4bf602cd6087c9c757",
            "profile_id": "ethos-deterministic-v1",
            "profile_sha256": "d6145b9210845db39ad592ea549788432b52a649778c9947f5b2d91173e38070",
            "schema_version": "1.0.0",
            "source_fingerprint": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
        });
        assert_eq!(
            sha256_hex(&v).unwrap(),
            "b5d30710d0c25cc38d8dec924ecaf57ae4f81276dd5dc14d75cb3b5b6bde62d3"
        );
    }

    // --- float rejection ------------------------------------------------------------------

    #[test]
    fn floats_are_rejected_at_every_depth() {
        assert!(c14n_bytes(&json!(1.5)).is_err());
        assert!(c14n_bytes(&json!({"x": 1.5})).is_err());
        assert!(c14n_bytes(&json!([0.1])).is_err());
        assert!(c14n_bytes(&json!({"a": {"b": [1, 2, {"c": 0.25}]}})).is_err());
        assert!(c14n_bytes(&json!([[[[2.5]]]])).is_err());
        assert!(c14n_bytes(&json!({"ok": 1, "bad": [{"deep": -3.5}]})).is_err());
    }

    #[test]
    fn a_float_is_an_error_not_a_rounding() {
        let e = c14n_bytes(&json!({"x": 1.5})).unwrap_err();
        assert_eq!(e.message(), "non-integer number in canonical value");
        // The nearest integer must never appear in output as a silent repair.
        assert!(c14n_bytes(&json!({"x": 1.5})).is_err());
    }

    #[test]
    fn integers_at_the_2_53_boundary() {
        assert!(c14n_bytes(&json!(MAX_SAFE_INT)).is_ok());
        assert!(c14n_bytes(&json!(MAX_SAFE_INT + 1)).is_err());
        assert!(c14n_bytes(&json!(-MAX_SAFE_INT)).is_ok());
        assert!(c14n_bytes(&json!(-MAX_SAFE_INT - 1)).is_err());
    }

    #[test]
    fn i64_min_errors_rather_than_panicking() {
        // `i64::MIN.abs()` overflows; `unsigned_abs` must catch it cleanly.
        assert!(c14n_bytes(&json!(i64::MIN)).is_err());
        assert!(c14n_bytes(&json!({"n": i64::MIN})).is_err());
        assert!(c14n_bytes(&json!(u64::MAX)).is_err());
    }

    // --- escaping -------------------------------------------------------------------------

    #[test]
    fn control_characters_use_lowercase_four_digit_escapes() {
        assert_eq!(c14n_str(&json!("\u{0001}")), r#""\u0001""#);
        assert_eq!(c14n_str(&json!("\u{001f}")), r#""\u001f""#);
        assert_eq!(c14n_str(&json!("\u{000b}")), r#""\u000b""#);
        // The named short escapes take precedence where they exist.
        assert_eq!(c14n_str(&json!("\u{0008}")), r#""\b""#);
        assert_eq!(c14n_str(&json!("\t")), r#""\t""#);
        assert_eq!(c14n_str(&json!("\n")), r#""\n""#);
        assert_eq!(c14n_str(&json!("\u{000c}")), r#""\f""#);
        assert_eq!(c14n_str(&json!("\r")), r#""\r""#);
    }

    #[test]
    fn non_ascii_is_literal_and_never_normalized() {
        // Space (U+0020) is the first codepoint at or above the escape threshold, so it is the
        // boundary case that must come through literally.
        assert_eq!(c14n_str(&json!(" ")), r#"" ""#);
        assert_eq!(c14n_str(&json!("é")), "\"é\"");
        assert_eq!(c14n_str(&json!("日本語")), "\"日本語\"");
        assert_eq!(c14n_str(&json!("\u{1F4A1}")), "\"\u{1F4A1}\"");

        // Precomposed é (U+00E9) and decomposed e + combining acute (U+0065 U+0301) are
        // different byte sequences and must stay different. Normalizing would silently rewrite
        // evidence a citation may quote verbatim.
        let precomposed = c14n_str(&json!("\u{00E9}"));
        let decomposed = c14n_str(&json!("\u{0065}\u{0301}"));
        assert_ne!(precomposed, decomposed);
        assert_eq!(precomposed.len(), 4); // quote + 2 UTF-8 bytes + quote
        assert_eq!(decomposed.len(), 5); // quote + 1 + 2 UTF-8 bytes + quote
    }

    #[test]
    fn only_quote_and_backslash_are_escaped_above_the_control_range() {
        assert_eq!(c14n_str(&json!("\"")), r#""\"""#);
        assert_eq!(c14n_str(&json!("\\")), r#""\\""#);
        // Solidus is NOT escaped — a `\/` would be valid JSON but different bytes.
        assert_eq!(c14n_str(&json!("/")), r#""/""#);
    }

    #[test]
    fn keys_are_escaped_by_the_same_rule_as_values() {
        let v = json!({"a\nb": 1});
        assert_eq!(c14n_str(&v), "{\"a\\nb\":1}");
    }

    // --- structure ------------------------------------------------------------------------

    #[test]
    fn no_whitespace_anywhere() {
        let s = c14n_str(&json!({"a": [1, 2, {"b": "c"}], "d": null}));
        assert!(
            !s.contains(' '),
            "canonical output must not contain spaces: {s}"
        );
        assert!(!s.contains('\n'));
        assert_eq!(s, r#"{"a":[1,2,{"b":"c"}],"d":null}"#);
    }

    #[test]
    fn array_order_is_preserved_because_it_is_semantic() {
        // Element order IS reading order; sorting arrays would destroy meaning.
        assert_eq!(c14n_str(&json!([3, 1, 2])), "[3,1,2]");
    }

    #[test]
    fn key_order_is_by_code_point_not_ascii_case_or_locale() {
        // Uppercase sorts before lowercase; '_' (0x5F) sits between them.
        let v = json!({"a": 1, "A": 2, "_": 3, "b": 4, "B": 5});
        assert_eq!(c14n_str(&v), r#"{"A":2,"B":5,"_":3,"a":1,"b":4}"#);

        // Non-ASCII keys sort by code point, above all ASCII.
        let v = json!({"é": 1, "z": 2, "A": 3});
        assert_eq!(c14n_str(&v), "{\"A\":3,\"z\":2,\"é\":1}");
    }

    #[test]
    fn nested_objects_are_sorted_at_every_level() {
        let v = json!({"z": {"y": 1, "x": 2}, "a": {"c": 3, "b": 4}});
        assert_eq!(c14n_str(&v), r#"{"a":{"b":4,"c":3},"z":{"x":2,"y":1}}"#);
    }

    // --- property tests -------------------------------------------------------------------

    fn arb_canonical_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::from),
            (-MAX_SAFE_INT..=MAX_SAFE_INT).prop_map(Value::from),
            "\\PC*".prop_map(Value::from),
        ];
        leaf.prop_recursive(4, 32, 8, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..6).prop_map(Value::Array),
                proptest::collection::btree_map("\\PC*", inner, 0..6)
                    .prop_map(|m| Value::Object(m.into_iter().collect())),
            ]
        })
    }

    proptest! {
        /// The idempotence gate: `c14n(parse(c14n(v))) == c14n(v)`.
        #[test]
        fn c14n_is_idempotent(v in arb_canonical_value()) {
            let once = c14n_bytes(&v).unwrap();
            let reparsed: Value = serde_json::from_slice(&once).unwrap();
            let twice = c14n_bytes(&reparsed).unwrap();
            prop_assert_eq!(once, twice);
        }

        /// Canonical output is always valid JSON that parses back to an equal value.
        #[test]
        fn c14n_output_reparses_equal(v in arb_canonical_value()) {
            let bytes = c14n_bytes(&v).unwrap();
            let reparsed: Value = serde_json::from_slice(&bytes).unwrap();
            prop_assert_eq!(&reparsed, &v);
        }

        /// Canonical output is always valid UTF-8.
        #[test]
        fn c14n_output_is_utf8(v in arb_canonical_value()) {
            prop_assert!(String::from_utf8(c14n_bytes(&v).unwrap()).is_ok());
        }

        /// Key insertion order never reaches the output.
        #[test]
        fn key_insertion_order_is_irrelevant(mut keys in proptest::collection::vec("[a-z]{1,4}", 1..8)) {
            keys.sort();
            keys.dedup();
            let forward: serde_json::Map<String, Value> =
                keys.iter().map(|k| (k.clone(), Value::from(1))).collect();
            let backward: serde_json::Map<String, Value> =
                keys.iter().rev().map(|k| (k.clone(), Value::from(1))).collect();
            prop_assert_eq!(
                c14n_bytes(&Value::Object(forward)).unwrap(),
                c14n_bytes(&Value::Object(backward)).unwrap()
            );
        }
    }
}
