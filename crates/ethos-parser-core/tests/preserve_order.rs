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

//! Key ordering survives `serde_json/preserve_order`.
//!
//! This is the specific hazard `c14n` was written against, and it is not hypothetical: cargo
//! features are **additive and unify across the whole graph**, so one crate anywhere in a
//! consumer's dependency tree enabling `preserve_order` switches `serde_json::Map` from
//! `BTreeMap` to `IndexMap` — for us too. Every fingerprint this engine has ever emitted would
//! silently change, and nothing would fail.
//!
//! `ethos-parser-core`'s dev-dependencies enable that feature so this test runs in exactly that
//! world. The first assertion in each test proves the feature really is active; without it,
//! these tests would pass vacuously under a `BTreeMap` that sorts for free, which is a worse
//! outcome than not having them.

use ethos_parser_core::c14n_bytes;
use serde_json::{Map, Value};

/// Build an object by inserting keys in the given order.
fn object_in_order(keys: &[&str]) -> Value {
    let mut m = Map::new();
    for (i, k) in keys.iter().enumerate() {
        m.insert((*k).to_string(), Value::from(i as i64));
    }
    Value::Object(m)
}

/// `serde_json`'s own output must show insertion order, or `preserve_order` is not active and
/// every test in this file is meaningless.
fn assert_preserve_order_is_actually_enabled() {
    let v = object_in_order(&["z", "a"]);
    let plain = serde_json::to_string(&v).unwrap();
    assert_eq!(
        plain, r#"{"z":0,"a":1}"#,
        "serde_json emitted sorted order, so `preserve_order` is NOT enabled for this build. \
         This test file cannot prove anything in that state — fix the dev-dependency feature \
         in crates/ethos-parser-core/Cargo.toml rather than deleting the assertion."
    );
}

#[test]
fn the_hazard_this_file_tests_for_is_real_and_active() {
    assert_preserve_order_is_actually_enabled();
}

#[test]
fn c14n_sorts_keys_even_under_preserve_order() {
    assert_preserve_order_is_actually_enabled();

    let v = object_in_order(&["z", "a", "m", "_", "B"]);
    let canonical = String::from_utf8(c14n_bytes(&v).unwrap()).unwrap();

    // Code-point order: 'B'(0x42) < '_'(0x5F) < 'a'(0x61) < 'm'(0x6D) < 'z'(0x7A)
    assert_eq!(canonical, r#"{"B":4,"_":3,"a":1,"m":2,"z":0}"#);
}

#[test]
fn insertion_order_cannot_change_canonical_bytes() {
    assert_preserve_order_is_actually_enabled();

    let forward = object_in_order(&["a", "b", "c", "d"]);
    let backward = object_in_order(&["d", "c", "b", "a"]);

    // Different values per key (the index), so this compares structure, not just key order.
    let f = c14n_bytes(&forward).unwrap();
    let b = c14n_bytes(&backward).unwrap();

    let fs = String::from_utf8(f).unwrap();
    let bs = String::from_utf8(b).unwrap();
    assert_eq!(fs, r#"{"a":0,"b":1,"c":2,"d":3}"#);
    assert_eq!(bs, r#"{"a":3,"b":2,"c":1,"d":0}"#);

    // Same key order in both, despite opposite insertion order.
    let keys = |s: &str| -> Vec<String> {
        s.split(',')
            .map(|p| {
                p.split(':')
                    .next()
                    .unwrap()
                    .trim_matches(['{', '}', '"'])
                    .to_string()
            })
            .collect()
    };
    assert_eq!(keys(&fs), keys(&bs));
}

#[test]
fn nested_objects_are_sorted_under_preserve_order_too() {
    assert_preserve_order_is_actually_enabled();

    let mut inner = Map::new();
    inner.insert("y".into(), Value::from(1));
    inner.insert("x".into(), Value::from(2));

    let mut outer = Map::new();
    outer.insert("z".into(), Value::Object(inner));
    outer.insert("a".into(), Value::from(3));

    let canonical = String::from_utf8(c14n_bytes(&Value::Object(outer)).unwrap()).unwrap();
    assert_eq!(canonical, r#"{"a":3,"z":{"x":2,"y":1}}"#);
}

#[test]
fn the_committed_parity_hashes_still_hold_under_preserve_order() {
    assert_preserve_order_is_actually_enabled();

    // If the map flavour could affect output, this Ethos-derived hash would move. It is the
    // same vector asserted in the c14n unit tests, re-run in the hostile configuration.
    let mut m = Map::new();
    m.insert("b".into(), Value::from(2));
    m.insert("a".into(), Value::from(1));
    m.insert("_".into(), Value::from(0));
    m.insert("Z".into(), Value::from(-3));

    assert_eq!(
        ethos_parser_core::sha256_hex(&Value::Object(m)).unwrap(),
        "9e8c5fa78b63297991b5b7b45bd334ccc61bd1058c5cd8ca6ee0451f78cd6cc1"
    );
}

#[test]
fn the_profile_hash_is_unaffected_by_map_flavour() {
    assert_preserve_order_is_actually_enabled();

    // The profile hash is the engine's identity. It must not depend on a feature flag chosen
    // by some unrelated crate three levels down a consumer's dependency graph.
    let a = ethos_parser_core::Profile::default().profile_sha256().unwrap();
    let b = ethos_parser_core::Profile::default().profile_sha256().unwrap();
    assert_eq!(a, b);

    // Round-tripping through a parsed (insertion-ordered) value must not move it either.
    let bytes = ethos_parser_core::Profile::default().canonical_bytes().unwrap();
    let reparsed: Value = serde_json::from_slice(&bytes).unwrap();
    let recanonicalized = c14n_bytes(&reparsed).unwrap();
    assert_eq!(bytes, recanonicalized);
}
