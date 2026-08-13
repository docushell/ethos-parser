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

//! A JSON Schema **subset** validator, driven by the pinned schema file itself.
//!
//! # Why not a JSON Schema crate
//!
//! `deny.toml` is a licence allowlist that also bans network-capable crates, and the obvious
//! candidates pull remote-reference machinery by default. Adding one would mean new licence
//! entries and a feature audit to prove the network path is off — real review cost for a check
//! that has to run against exactly one schema. `docs/04-ARCHITECTURE.md` §5 asks for a
//! dependency to be justified; this one is not.
//!
//! # Why not hand-written assertions either
//!
//! Because they drift. Assertions copied out of a schema stop matching it the moment the schema
//! moves, and nothing notices. This validator **reads the pinned schema** and interprets it, so
//! the rules it enforces are the schema's own.
//!
//! # The vacuity problem, and how this avoids it
//!
//! A subset validator that silently ignores a keyword it does not implement is worse than no
//! validator: it reports success over rules it never checked. So [`compile`] walks the schema
//! and **fails loudly on any keyword outside the implemented set**, and
//! `every_keyword_in_the_schema_is_implemented` asserts the schema uses nothing else. A new
//! keyword upstream stops the build rather than being waved through.
//!
//! Separately, `the_validator_rejects_each_deliberate_break` runs a corpus of artifacts broken
//! one rule at a time. Without it, a validator that returned `Ok(())` unconditionally would pass
//! every positive test in this file.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value;

// -------------------------------------------------------------------------------------------
// The pinned schema
// -------------------------------------------------------------------------------------------

/// The workspace root.
///
/// Derived by walking up from `CARGO_MANIFEST_DIR` to the directory holding `crates/`, rather
/// than by joining a fixed relative path: this module is compiled into two different crates'
/// test binaries (here and `engine-cli`, by `#[path]` include), so a crate-relative path would
/// silently resolve to the wrong place in one of them — which is exactly what it did the first
/// time.
fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("crates").is_dir() {
        assert!(
            dir.pop(),
            "no ancestor of CARGO_MANIFEST_DIR contains crates/"
        );
    }
    dir
}

fn schema_path() -> PathBuf {
    repo_root().join("crates/engine-grounding/schemas/ethos-grounding-source.schema.json")
}

/// The snapshot's digest, recorded in `schemas/README.md`.
const PINNED_SCHEMA_SHA256: &str =
    "8d41c1e08f49ec0ca4878ac0ec3ccf3a79f7b9ffa31aa26ad0a60a6f27b319de";

pub fn schema() -> Value {
    let bytes = std::fs::read(schema_path()).unwrap_or_else(|e| {
        panic!(
            "the pinned grounding schema is missing at {}: {e}. Conformance testing does not \
             depend on a sibling checkout — the snapshot IS the input, so its absence is a hard \
             failure, never a skip.",
            schema_path().display()
        )
    });
    serde_json::from_slice(&bytes).expect("the pinned schema is valid JSON")
}

/// The snapshot is the file it claims to be.
#[test]
fn the_pinned_schema_matches_its_recorded_digest() {
    let bytes = std::fs::read(schema_path()).expect("readable");
    let got = engine_core::sha256_hex_bytes(&bytes);
    assert_eq!(
        got, PINNED_SCHEMA_SHA256,
        "the pinned schema changed without its digest being updated. Re-pin deliberately, in a \
         commit that says what moved upstream and what it means for the projection."
    );
}

/// The snapshot still matches the Ethos tree, **when that tree is present**.
///
/// A deliberate asymmetry, and worth stating plainly because it is the opposite of the fixture
/// rule where a missing corpus is a hard failure: a fixture is *input the test needs*, while this
/// is *a second opinion about a file we already have*. A missing Ethos tree weakens the drift
/// check and never weakens conformance.
#[test]
fn the_snapshot_has_not_drifted_from_the_ethos_tree() {
    let upstream = repo_root().join("../ethos/schemas/ethos-grounding-source.schema.json");

    let Ok(upstream_bytes) = std::fs::read(&upstream) else {
        eprintln!(
            "note: {} is absent, so the drift check did not run. Conformance still used the \
             pinned snapshot.",
            upstream.display()
        );
        return;
    };

    let ours = std::fs::read(schema_path()).expect("readable");
    assert_eq!(
        engine_core::sha256_hex_bytes(&ours),
        engine_core::sha256_hex_bytes(&upstream_bytes),
        "the pinned snapshot has drifted from {}. Either check out the ref the snapshot was \
         taken from, or re-pin deliberately and say what changed.",
        upstream.display()
    );
}

// -------------------------------------------------------------------------------------------
// The subset validator
// -------------------------------------------------------------------------------------------

/// Keywords this validator interprets. Anything else is a hard error.
const IMPLEMENTED: [&str; 20] = [
    // assertions
    "type",
    "const",
    "enum",
    "required",
    "properties",
    "additionalProperties",
    "pattern",
    "minLength",
    "maxLength",
    "minimum",
    "maximum",
    "items",
    "prefixItems",
    "minItems",
    "maxItems",
    "$ref",
    // annotations, ignored on purpose but named so they are not "unknown"
    "$schema",
    "$id",
    "title",
    "$defs",
];

/// Anchored patterns the schema actually uses, each with a hand-written matcher.
///
/// Hand-written because this workspace has no regex crate, and honest because the set is closed:
/// `only_known_patterns_appear_in_the_schema` fails if the schema ever introduces a fourth. A
/// matcher for an unknown pattern would have to be written before the schema could be used, which
/// is the correct order.
fn matches_pattern(pattern: &str, s: &str) -> bool {
    match pattern {
        // ^[A-Za-z0-9][A-Za-z0-9._:-]*$
        "^[A-Za-z0-9][A-Za-z0-9._:-]*$" => {
            let mut it = s.chars();
            match it.next() {
                Some(c) if c.is_ascii_alphanumeric() => {}
                _ => return false,
            }
            it.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
        }
        // ^sha256:[0-9a-f]{64}$
        "^sha256:[0-9a-f]{64}$" => {
            let Some(hex) = s.strip_prefix("sha256:") else {
                return false;
            };
            hex.len() == 64
                && hex
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        }
        // ^[a-z0-9][a-z0-9_-]*$
        "^[a-z0-9][a-z0-9_-]*$" => {
            let mut it = s.chars();
            match it.next() {
                Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
                _ => return false,
            }
            it.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '-'))
        }
        other => panic!(
            "no matcher for pattern `{other}`. Write one — silently accepting an uninterpreted \
             pattern is how a validator starts passing over rules it never checked."
        ),
    }
}

struct Ctx<'a> {
    root: &'a Value,
    errors: Vec<String>,
}

impl<'a> Ctx<'a> {
    fn err(&mut self, at: &str, msg: String) {
        self.errors.push(format!("{at}: {msg}"));
    }

    fn resolve(&self, schema: &'a Value) -> &'a Value {
        match schema.get("$ref").and_then(Value::as_str) {
            Some(r) => {
                // A `$ref` alongside its own assertions would mean this resolver silently drops
                // them — the exact vacuity this file exists to avoid. The pinned schema uses only
                // bare refs today; if that changes, stop rather than validate less than the
                // schema says.
                let siblings: Vec<&String> = schema
                    .as_object()
                    .expect("a $ref lives on an object")
                    .keys()
                    .filter(|k| k.as_str() != "$ref")
                    .collect();
                assert!(
                    siblings.is_empty(),
                    "`{r}` carries sibling keywords {siblings:?} that this resolver would drop. \
                     Implement them before relying on the result."
                );

                let name = r
                    .strip_prefix("#/$defs/")
                    .unwrap_or_else(|| panic!("unsupported $ref form `{r}`"));
                self.root
                    .get("$defs")
                    .and_then(|d| d.get(name))
                    .unwrap_or_else(|| panic!("$ref `{r}` does not resolve"))
            }
            None => schema,
        }
    }

    fn check(&mut self, schema: &'a Value, instance: &Value, at: &str) {
        let schema = self.resolve(schema);
        let Some(obj) = schema.as_object() else {
            // `items: false` — nothing may appear here.
            if schema == &Value::Bool(false) {
                self.err(at, "no value is permitted at this position".into());
            }
            return;
        };

        for key in obj.keys() {
            assert!(
                IMPLEMENTED.contains(&key.as_str()),
                "schema keyword `{key}` at {at} is not implemented by this validator"
            );
        }

        if let Some(t) = obj.get("type").and_then(Value::as_str) {
            let ok = match t {
                "object" => instance.is_object(),
                "array" => instance.is_array(),
                "string" => instance.is_string(),
                "boolean" => instance.is_boolean(),
                "integer" => instance.is_i64() || instance.is_u64(),
                other => panic!("unimplemented type `{other}`"),
            };
            if !ok {
                self.err(at, format!("expected type {t}"));
                return;
            }
        }
        if let Some(c) = obj.get("const") {
            if instance != c {
                self.err(at, format!("expected const {c}"));
            }
        }
        if let Some(Value::Array(vals)) = obj.get("enum") {
            if !vals.contains(instance) {
                self.err(at, format!("value {instance} is not in the enum"));
            }
        }
        if let Some(p) = obj.get("pattern").and_then(Value::as_str) {
            if let Some(s) = instance.as_str() {
                if !matches_pattern(p, s) {
                    self.err(at, format!("`{s}` does not match {p}"));
                }
            }
        }
        if let (Some(n), Some(s)) = (
            obj.get("minLength").and_then(Value::as_u64),
            instance.as_str(),
        ) {
            if (s.chars().count() as u64) < n {
                self.err(at, format!("shorter than minLength {n}"));
            }
        }
        if let (Some(n), Some(s)) = (
            obj.get("maxLength").and_then(Value::as_u64),
            instance.as_str(),
        ) {
            if (s.chars().count() as u64) > n {
                self.err(at, format!("longer than maxLength {n}"));
            }
        }
        if let (Some(m), Some(v)) = (
            obj.get("minimum").and_then(Value::as_i64),
            instance.as_i64(),
        ) {
            if v < m {
                self.err(at, format!("{v} is below minimum {m}"));
            }
        }
        if let (Some(m), Some(v)) = (
            obj.get("maximum").and_then(Value::as_i64),
            instance.as_i64(),
        ) {
            if v > m {
                self.err(at, format!("{v} is above maximum {m}"));
            }
        }

        if let Some(inst) = instance.as_object() {
            if let Some(Value::Array(req)) = obj.get("required") {
                for r in req {
                    let name = r.as_str().expect("required names are strings");
                    if !inst.contains_key(name) {
                        self.err(at, format!("missing required property `{name}`"));
                    }
                }
            }
            let props = obj.get("properties").and_then(Value::as_object);
            if obj.get("additionalProperties") == Some(&Value::Bool(false)) {
                for k in inst.keys() {
                    let known = props.is_some_and(|p| p.contains_key(k));
                    if !known {
                        self.err(at, format!("additional property `{k}` is not permitted"));
                    }
                }
            }
            if let Some(props) = props {
                for (k, sub) in props {
                    if let Some(v) = inst.get(k) {
                        self.check(sub, v, &format!("{at}/{k}"));
                    }
                }
            }
        }

        if let Some(items) = instance.as_array() {
            if let Some(n) = obj.get("minItems").and_then(Value::as_u64) {
                if (items.len() as u64) < n {
                    self.err(at, format!("fewer than minItems {n}"));
                }
            }
            if let Some(n) = obj.get("maxItems").and_then(Value::as_u64) {
                if (items.len() as u64) > n {
                    self.err(at, format!("more than maxItems {n}"));
                }
            }
            let prefix = obj.get("prefixItems").and_then(Value::as_array);
            if let Some(prefix) = prefix {
                for (i, sub) in prefix.iter().enumerate() {
                    if let Some(v) = items.get(i) {
                        self.check(sub, v, &format!("{at}/{i}"));
                    }
                }
            }
            if let Some(rest) = obj.get("items") {
                let start = prefix.map(|p| p.len()).unwrap_or(0);
                for (i, v) in items.iter().enumerate().skip(start) {
                    self.check(rest, v, &format!("{at}/{i}"));
                }
            }
        }
    }
}

/// Validate an instance against the pinned schema. `Ok(())` or every violation found.
pub fn validate(instance: &Value) -> Result<(), Vec<String>> {
    let root = schema();
    let mut ctx = Ctx {
        root: &root,
        errors: Vec::new(),
    };
    ctx.check(&root, instance, "");
    if ctx.errors.is_empty() {
        Ok(())
    } else {
        Err(ctx.errors)
    }
}

// -------------------------------------------------------------------------------------------
// Proving the validator is not vacuous
// -------------------------------------------------------------------------------------------

/// Every keyword the schema uses is one this validator interprets.
#[test]
fn every_keyword_in_the_schema_is_implemented() {
    fn walk(v: &Value, out: &mut BTreeSet<String>, in_props: bool) {
        match v {
            Value::Object(o) => {
                for (k, sub) in o {
                    // Under `properties`/`$defs` the keys are names, not keywords.
                    if !in_props {
                        out.insert(k.clone());
                    }
                    let next_in_props = matches!(k.as_str(), "properties" | "$defs");
                    walk(sub, out, next_in_props && !in_props);
                }
            }
            Value::Array(a) => {
                for sub in a {
                    walk(sub, out, false);
                }
            }
            _ => {}
        }
    }

    let mut used = BTreeSet::new();
    walk(&schema(), &mut used, false);
    // `description` is annotation-only and absent from this schema; assert we did not miss one.
    let unimplemented: Vec<&String> = used
        .iter()
        .filter(|k| !IMPLEMENTED.contains(&k.as_str()))
        .collect();
    assert!(
        unimplemented.is_empty(),
        "the pinned schema uses keywords this validator does not interpret: {unimplemented:?}. \
         Implement them — ignoring a keyword makes every passing test here mean less than it \
         appears to."
    );
}

/// The schema uses only patterns this file has a matcher for.
#[test]
fn only_known_patterns_appear_in_the_schema() {
    fn walk(v: &Value, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                for (k, sub) in o {
                    if k == "pattern" {
                        if let Some(s) = sub.as_str() {
                            out.insert(s.to_string());
                        }
                    }
                    walk(sub, out);
                }
            }
            Value::Array(a) => a.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut found = BTreeSet::new();
    walk(&schema(), &mut found);
    let expected: BTreeSet<String> = [
        "^[A-Za-z0-9][A-Za-z0-9._:-]*$",
        "^sha256:[0-9a-f]{64}$",
        "^[a-z0-9][a-z0-9_-]*$",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(found, expected, "a new pattern needs a matcher before use");
}

/// The hand-written matchers behave like the regexes they stand in for.
#[test]
fn the_pattern_matchers_accept_and_reject_the_right_strings() {
    let id = "^[A-Za-z0-9][A-Za-z0-9._:-]*$";
    for good in ["p1", "e12", "page-1", "A", "a.b:c-d_e", "0"] {
        assert!(matches_pattern(id, good), "{good} should match");
    }
    for bad in ["", "-p1", ".x", "p 1", "p/1", "p\n", "é1"] {
        assert!(!matches_pattern(id, bad), "{bad:?} should not match");
    }

    let sha = "^sha256:[0-9a-f]{64}$";
    let hex64 = "0123456789abcdef".repeat(4);
    assert!(matches_pattern(sha, &format!("sha256:{hex64}")));
    assert!(!matches_pattern(sha, &format!("sha256:{}", &hex64[..63])));
    assert!(!matches_pattern(sha, &format!("sha256:{hex64}0")));
    assert!(!matches_pattern(sha, &format!("SHA256:{hex64}")));
    assert!(!matches_pattern(
        sha,
        &format!("sha256:{}", hex64.to_uppercase())
    ));

    let kind = "^[a-z0-9][a-z0-9_-]*$";
    for good in ["text_run", "t", "a-b_c", "9x"] {
        assert!(matches_pattern(kind, good));
    }
    for bad in ["", "_x", "Text_run", "text run", "-x"] {
        assert!(!matches_pattern(kind, bad), "{bad:?} should not match");
    }
}

/// The **subset validator** never becomes production validation code.
///
/// M6 gave `engine-grounding` a real `check` module, so "no validation in src/" is no longer the
/// rule and would now be false. The rule that still matters is narrower and is the one that keeps
/// the oracle honest: **the production checker must not be this file**.
///
/// If `grounding-check` graded artifacts with the same schema-driven subset validator that the
/// conformance tests use, then "the engine agrees with Ethos" would partly mean "the engine agrees
/// with itself". The production checker is an independent transcription of Ethos's own
/// `grounding_json.rs` rules; this file is a JSON Schema interpreter. Two different instruments,
/// deliberately, and the machinery of this one must not appear in `src/`.
#[test]
fn the_subset_validator_is_test_only() {
    // Recursive: a guard that inspects one file is a guard against one file.
    fn walk(dir: &std::path::Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).expect("readable") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                for line in std::fs::read_to_string(&path).expect("readable").lines() {
                    if !line.trim_start().starts_with("//") {
                        out.push_str(line);
                        out.push('\n');
                    }
                }
            }
        }
    }
    let mut code = String::new();
    walk(&repo_root().join("crates/engine-grounding/src"), &mut code);
    assert!(code.len() > 500, "the source scan found almost nothing");

    for banned in [
        // The subset interpreter's own machinery. Its presence in `src/` would mean the
        // production path had been pointed at the test instrument.
        "matches_pattern",
        "IMPLEMENTED",
        "schema_path",
        "PINNED_SCHEMA_SHA256",
        // And the production checker must not read the schema file at runtime at all: it
        // mirrors Ethos's parser, and a checker that needed a data file on disk would fail
        // differently depending on how it was installed.
        "include_str!",
        ".schema.json",
    ] {
        assert!(
            !code.contains(banned),
            "`{banned}` appears in engine-grounding's library. The production checker mirrors \
             Ethos's own rules; it must not be, or read, this file's schema interpreter, or \
             agreement with the oracle becomes partly agreement with ourselves."
        );
    }

    // And this file really is a test file, not a module compiled into the library.
    assert!(
        !repo_root()
            .join("crates/engine-grounding/src/schema_subset.rs")
            .exists(),
        "the validator must live under tests/, never under src/"
    );
}
