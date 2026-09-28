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

//! **Every artifact the engine emits is described by its draft schema** (review 2026-09-26 N33).
//!
//! `docs/draft-schemas/README.md` says the drafts document a shape that already compiles, and by
//! 0.61.0 four of them rejected every artifact the engine emitted: 90 of 90 representations, 75 of
//! 75 classifications, 73 of 73 extracts and every profile, the profile draft its own example.
//! The guards beside them in `contract_invariants.rs` pin version constants and rule ids, and
//! nothing compared an artifact with its schema.
//!
//! This builds real artifacts from every engine and office fixture, through the calls the CLI
//! makes, and walks each against its draft: every key must be one the draft declares, every
//! required key present, every `const`, `enum` and `type` met, and some branch of every union
//! fitted. It is a subset of JSON Schema, not a validator — no minimum, pattern or length is
//! checked — which is the drift that went unseen: a field, a variant or a value the draft did not
//! know.
//!
//! # Why it lives in `ethos-parser-cli`
//!
//! The only crate that depends on both readers, as `library_surface.rs` says.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ethos_parser_core::{DocumentRepresentation, EngineError, PageBudget, Profile};
use ethos_parser_pdf::Document;
use serde_json::Value;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

/// Every draft, by its `$id`.
struct Drafts(BTreeMap<String, Value>);

impl Drafts {
    fn load() -> Self {
        let dir = repo_root().join("docs/draft-schemas");
        let mut drafts = BTreeMap::new();
        for entry in std::fs::read_dir(&dir).expect("docs/draft-schemas is readable") {
            let path = entry.expect("a directory entry").path();
            if !path.to_string_lossy().ends_with(".draft.json") {
                continue;
            }
            let schema: Value = serde_json::from_slice(&std::fs::read(&path).expect("readable"))
                .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
            let id = schema["$id"]
                .as_str()
                .expect("every draft has an $id")
                .to_string();
            drafts.insert(id, schema);
        }
        assert!(drafts.len() >= 14, "read only {} drafts", drafts.len());
        Self(drafts)
    }

    /// The draft named `name`, as `(its $id, its root)`.
    fn named(&self, name: &str) -> (&str, &Value) {
        let id = format!("urn:ethos-parser:draft-schema:{name}:0");
        let (id, root) = self.0.get_key_value(&id).expect("a draft by that name");
        (id, root)
    }

    /// What `reference` names, read from the draft whose `$id` is `base`.
    fn resolve<'a>(&'a self, base: &'a str, reference: &str) -> (&'a str, &'a Value) {
        let (draft, pointer) = reference.split_once('#').unwrap_or((reference, ""));
        let draft = if draft.is_empty() { base } else { draft };
        let (id, root) = self
            .0
            .get_key_value(draft)
            .unwrap_or_else(|| panic!("$ref {reference} names no draft"));
        let target = root
            .pointer(pointer)
            .unwrap_or_else(|| panic!("$ref {reference} names nothing in {id}"));
        (id, target)
    }

    /// Where `value` departs from `schema`, read in the draft whose `$id` is `base`.
    ///
    /// **A key the schema does not declare is a departure** unless the schema says otherwise,
    /// where JSON Schema admits it by default: "every emitted key is declared" is the claim.
    fn departures(
        &self,
        base: &str,
        schema: &Value,
        value: &Value,
        at: &str,
        out: &mut Vec<String>,
    ) {
        if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
            let (base, target) = self.resolve(base, reference);
            return self.departures(base, target, value, at, out);
        }
        let mut union = false;
        for key in ["oneOf", "anyOf"] {
            let Some(branches) = schema.get(key).and_then(Value::as_array) else {
                continue;
            };
            union = true;
            let tries: Vec<Vec<String>> = branches
                .iter()
                .map(|branch| {
                    let mut found = Vec::new();
                    self.departures(base, branch, value, at, &mut found);
                    found
                })
                .collect();
            if !tries.iter().any(Vec::is_empty) {
                let nearest = tries.into_iter().min_by_key(Vec::len).unwrap_or_default();
                out.push(format!(
                    "{at}: fits no branch of its {key}: {}",
                    nearest.join("; ")
                ));
            }
        }
        if let Some(expected) = schema.get("const") {
            if value != expected {
                out.push(format!("{at}: {value} where the draft says {expected}"));
            }
        }
        if let Some(allowed) = schema.get("enum").and_then(Value::as_array) {
            if !allowed.contains(value) {
                out.push(format!("{at}: {value} is not one of {allowed:?}"));
            }
        }
        if let Some(types) = schema.get("type") {
            let types: Vec<&str> = match types {
                Value::Array(list) => list.iter().filter_map(Value::as_str).collect(),
                other => other.as_str().into_iter().collect(),
            };
            if !types.iter().any(|t| is_type(t, value)) {
                out.push(format!("{at}: {value} is not {types:?}"));
            }
        }
        match value {
            Value::Object(map) => {
                for key in schema
                    .get("required")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let key = key.as_str().expect("required keys are strings");
                    if !map.contains_key(key) {
                        out.push(format!("{at}: required `{key}` is missing"));
                    }
                }
                let declared = schema.get("properties").and_then(Value::as_object);
                for (key, child) in map {
                    let path = format!("{at}/{key}");
                    match (
                        declared.and_then(|d| d.get(key)),
                        schema.get("additionalProperties"),
                    ) {
                        (Some(property), _) => self.departures(base, property, child, &path, out),
                        (None, Some(rest @ Value::Object(_))) => {
                            self.departures(base, rest, child, &path, out);
                        }
                        (None, Some(Value::Bool(true))) => {}
                        // A union's branches declare what this level does not.
                        (None, _) if union => {}
                        (None, _) => out.push(format!("{path}: not a key the draft declares")),
                    }
                }
            }
            Value::Array(items) => {
                if let Some(item) = schema.get("items") {
                    for (i, child) in items.iter().enumerate() {
                        self.departures(base, item, child, &format!("{at}/{i}"), out);
                    }
                }
            }
            _ => {}
        }
    }
}

/// An artifact as the JSON it serializes to.
macro_rules! as_json {
    ($artifact:expr) => {
        serde_json::to_value(&$artifact).expect("an artifact serializes")
    };
}

fn is_type(name: &str, value: &Value) -> bool {
    match name {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        "integer" => value.is_i64() || value.is_u64(),
        "number" => value.is_number(),
        other => panic!("a type the walker does not know: {other}"),
    }
}

/// The artifacts the engine emits for the repository's own fixtures, each beside the name of the
/// draft that describes it.
fn artifacts() -> Vec<(String, &'static str, Value)> {
    let mut out = Vec::new();
    let profile = Profile::default();
    for (name, p) in [
        ("pdf", Profile::default()),
        ("docx", Profile::docx_v0()),
        ("xlsx", Profile::xlsx_v0()),
        ("pptx", Profile::pptx_v0()),
        ("odt", Profile::odt_v0()),
        ("ods", Profile::ods_v0()),
        ("odp", Profile::odp_v0()),
        ("rtf", Profile::rtf_v0()),
        ("epub", Profile::epub_v0()),
    ] {
        out.push((format!("{name} profile"), "profile", as_json!(p)));
    }
    for e in [
        EngineError::Unsupported {
            what: "pdf operator".into(),
            detail: "\"".into(),
        },
        EngineError::Malformed {
            what: "xref entry".into(),
            detail: "19 bytes, expected 20".into(),
        },
        EngineError::Encrypted {
            detail: "standard security handler".into(),
        },
        EngineError::ResourceLimit {
            limit: "max nesting depth".into(),
            configured: "64".into(),
        },
        EngineError::MissingPart {
            part: "xref table".into(),
        },
        EngineError::Io {
            detail: "no such file".into(),
        },
    ] {
        out.push((e.code().into(), "error-taxonomy", as_json!(e)));
    }

    let mut representations = Vec::new();
    let engine = repo_root().join("fixtures/engine");
    let mut pdfs: Vec<PathBuf> = std::fs::read_dir(&engine)
        .expect("fixtures/engine is readable")
        .map(|e| e.expect("an entry").path().join("document.pdf"))
        .filter(|p| p.is_file())
        .collect();
    pdfs.sort();
    // A budget of none quarantines every page, so the partial states are walked too.
    let budgeted = Profile {
        page_budget: PageBudget::AtMost(0),
        ..Profile::default()
    };
    let runs = pdfs
        .iter()
        .map(|p| (p, &profile))
        .chain(std::iter::once((&pdfs[0], &budgeted)));
    for (path, profile) in runs {
        let name = path
            .parent()
            .expect("a fixture dir")
            .file_name()
            .expect("a name");
        let name = format!("{} ({:?})", name.to_string_lossy(), profile.page_budget);
        let Ok(doc) = Document::open(path, profile) else {
            continue;
        };
        if let Ok(c) = ethos_parser_pdf::classify(&doc, profile) {
            out.push((name.clone(), "classification", as_json!(c)));
        }
        let Ok(extract) = ethos_parser_pdf::extract(&doc, profile) else {
            continue;
        };
        out.push((name.clone(), "extract", as_json!(extract)));
        let repr = ethos_parser_pdf::to_representation(&extract, profile).expect("represents");
        representations.push((name, repr));
    }
    let office = repo_root().join("fixtures/office");
    let mut documents: Vec<PathBuf> = std::fs::read_dir(&office)
        .expect("fixtures/office is readable")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.is_dir())
        .flat_map(|dir| {
            std::fs::read_dir(dir)
                .expect("readable")
                .map(|e| e.expect("an entry").path())
        })
        .collect();
    documents.sort();
    for path in documents {
        let bytes = std::fs::read(&path).expect("readable");
        if let Ok(repr) = ethos_parser_office::read(&bytes) {
            representations.push((
                path.file_name().expect("a name").to_string_lossy().into(),
                repr,
            ));
        }
    }

    let sha = profile
        .profile_sha256()
        .expect("the default profile hashes");
    for (name, repr) in representations {
        out.push((name.clone(), "document-representation", as_json!(repr)));
        project(&name, &repr, &profile, &sha, &mut out);
    }
    out
}

/// The three projections of `repr`, each where the engine produces one.
fn project(
    name: &str,
    repr: &DocumentRepresentation,
    profile: &Profile,
    sha: &ethos_parser_core::Sha256Hex,
    out: &mut Vec<(String, &'static str, Value)>,
) {
    let v = &profile.parser_version;
    if let Ok(md) = ethos_parser_core::to_markdown(repr, v, sha, &profile.markdown_rule) {
        out.push((name.into(), "markdown", as_json!(md)));
    }
    if let Ok(html) = ethos_parser_core::to_html(repr, v, sha, &profile.html_rule) {
        out.push((name.into(), "html", as_json!(html)));
    }
    // A word the record holds, so an occurrence is walked and not only an empty answer.
    let payload = as_json!(repr);
    let quote = payload["representation"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| n["text"].as_str())
        .find_map(|t| t.split_whitespace().find(|w| w.chars().count() >= 3));
    if let Some(quote) = quote {
        if let Ok(found) = ethos_parser_core::locate(repr, v, sha, &profile.locate_rule, quote) {
            out.push((name.into(), "locations", as_json!(found)));
        }
    }
}

/// **Every artifact the engine emits for the repository's fixtures is one its draft describes.**
///
/// The failure this catches: a field, a variant or a value added to an artifact and not to its
/// draft, or a draft's `additionalProperties: false` left closed over a key the engine writes —
/// `outlines` was missing from the capabilities draft for a release while every artifact carried
/// it. Deleting `outlines` from `profile.draft.json`'s `$defs/capabilities` turns this red.
#[test]
fn every_artifact_the_engine_emits_is_described_by_its_draft() {
    let drafts = Drafts::load();
    let artifacts = artifacts();
    let mut walked: BTreeMap<&str, usize> = BTreeMap::new();
    let mut departures = Vec::new();
    for (name, draft, value) in &artifacts {
        let (base, schema) = drafts.named(draft);
        let mut found = Vec::new();
        drafts.departures(base, schema, value, "", &mut found);
        departures.extend(
            found
                .into_iter()
                .take(5)
                .map(|d| format!("{draft} of {name}: {d}")),
        );
        *walked.entry(draft).or_default() += 1;
    }
    assert!(
        departures.is_empty(),
        "{} departures from the drafts:\n{}",
        departures.len(),
        departures.join("\n")
    );
    for draft in [
        "profile",
        "classification",
        "extract",
        "document-representation",
        "markdown",
        "html",
        "locations",
        "error-taxonomy",
    ] {
        assert!(
            walked.get(draft).is_some_and(|&n| n > 0),
            "no {draft} was walked: {walked:?}"
        );
    }

    // **And every variant the representation draft declares met a real node.** A variant no
    // fixture produces is one this walk never checked, so a draft could spell it wrong unseen.
    let (_, repr) = drafts.named("document-representation");
    for (def, field) in [
        ("native_locator", "native_locator"),
        ("node_attributes", "attributes"),
    ] {
        let declared: BTreeSet<&str> = repr["$defs"][def]["properties"]
            .as_object()
            .expect("a union of named variants")
            .keys()
            .map(String::as_str)
            .collect();
        let met: BTreeSet<&str> = artifacts
            .iter()
            .filter(|(_, draft, _)| *draft == "document-representation")
            .flat_map(|(_, _, v)| {
                v["representation"]["nodes"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .filter_map(|n| n[field].as_object()?.keys().next().map(String::as_str))
            .collect();
        assert_eq!(
            declared, met,
            "{def}: the variants the draft declares and the ones met differ"
        );
    }
}

/// **The walk reports each departure it exists to find, and nothing on a value that conforms.**
///
/// The failure this catches: a walk that stopped looking. The test above passes on a walk that
/// reports nothing, so each rule is held to a value that breaks it.
#[test]
fn the_walk_reports_each_departure_it_exists_to_find() {
    let schema = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "at"],
        "properties": {
            "kind": { "enum": ["a", "b"] },
            "at": { "$ref": "#/$defs/at" },
            "tag": { "const": 1 },
            "list": { "type": "array", "items": { "type": "integer" } },
            "open": { "type": "object", "additionalProperties": true }
        },
        "$defs": {
            "at": {
                "oneOf": [
                    {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["page"],
                        "properties": { "page": { "type": "integer" } }
                    },
                    {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["part"],
                        "properties": { "part": { "type": "string" } }
                    }
                ]
            }
        }
    });
    let drafts = Drafts(BTreeMap::from([("urn:t".to_string(), schema.clone())]));
    let walk = |value: &Value| {
        let mut found = Vec::new();
        drafts.departures("urn:t", &schema, value, "", &mut found);
        found
    };
    let conforming = serde_json::json!({
        "kind": "a", "at": { "page": 1 }, "tag": 1, "list": [2], "open": { "x": 0 }
    });
    assert_eq!(walk(&conforming), Vec::<String>::new());
    for (value, expected) in [
        (
            serde_json::json!({ "kind": "a", "at": { "page": 1 }, "extra": 0 }),
            "/extra: not a key the draft declares",
        ),
        (
            serde_json::json!({ "kind": "a" }),
            ": required `at` is missing",
        ),
        (
            serde_json::json!({ "kind": "c", "at": { "page": 1 } }),
            "/kind: \"c\" is not one of",
        ),
        (
            serde_json::json!({ "kind": "a", "at": { "page": 1 }, "tag": 2 }),
            "/tag: 2 where the draft says 1",
        ),
        (
            serde_json::json!({ "kind": "a", "at": { "page": 1 }, "list": ["x"] }),
            "/list/0: \"x\" is not",
        ),
        (
            serde_json::json!({ "kind": "a", "at": { "page": "1" } }),
            "/at: fits no branch of its oneOf",
        ),
        (
            serde_json::json!({ "kind": "a", "at": { "page": 1, "part": "p" } }),
            "/at: fits no branch of its oneOf",
        ),
    ] {
        let found = walk(&value);
        assert!(
            found.iter().any(|d| d.contains(expected)),
            "{value}: expected {expected:?}, found {found:?}"
        );
    }
}
