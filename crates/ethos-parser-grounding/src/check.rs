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

//! `grounding-check` — structure and source binding, and **nothing else**.
//!
//! # What this is not
//!
//! It does not check claims, does not emit `grounded`, does not compute an evidence tier, does
//! not match quotes, and never re-derives anything from a verifier's report
//! (`docs/07-VERIFY-BOUNDARY.md`). It answers exactly two questions — *is this artifact
//! structurally valid* and *do these bytes bind to it* — and a grep test forbids the
//! verification vocabulary from appearing here at all.
//!
//! Reimplementing verifier semantics is how a second authority is born by accident. The
//! validator is the one piece of Ethos this engine is permitted to have an opinion about,
//! because M6's whole criterion is that the two opinions agree byte for byte.
//!
//! # Why this mirrors Ethos's parser rather than only the JSON Schema
//!
//! The pinned schema is necessary and **not sufficient**. `ethos-core`'s
//! `parse_grounding_json` enforces a long list of invariants the schema cannot express — id
//! uniqueness, reference resolution, page ordering, boxes inside their page, capability/array
//! agreement, character offsets that actually index their element's text, table cell occupancy.
//! A checker that validated only the schema would call artifacts *valid* that the oracle calls
//! *invalid*, and the disagreement would show up as a failing oracle test with no obvious cause.
//!
//! So the rules below are transcribed from `../ethos/crates/ethos-core/src/grounding_json.rs`,
//! **in its order**, because the order decides which error code an artifact with two faults
//! reports. Every code and path spelling here is the one Ethos emits.

use serde::{Deserialize, Serialize};

use ethos_parser_core::{sha256_hex_bytes, EngineError, Sha256Hex};

use crate::{Element, GroundingSource, Page, Span, Table};

/// Artifact type of a validation report.
pub const VALIDATION_ARTIFACT_TYPE: &str = "ethos.grounding_validation.v1";

/// Schema version of a validation report.
pub const VALIDATION_SCHEMA_VERSION: &str = "1.0.0";

/// Limits, transcribed from `ethos-core::grounding_json`.
pub(crate) mod limits {
    pub const MAX_INPUT_BYTES: usize = 256 * 1024 * 1024;
    pub const MAX_PAGES: usize = 5_000;
    pub const MAX_ELEMENTS: usize = 1_000_000;
    pub const MAX_TABLES: usize = 100_000;
    pub const MAX_CELLS: usize = 1_000_000;
    pub const MAX_STRING_BYTES: usize = 16_384;
    /// A page-less element's locator, which Ethos bounds more tightly than any other string.
    pub const MAX_LOCATOR_BYTES: usize = 2_048;
    pub const MAX_ID_BYTES: usize = 256;
    /// `2^53 - 1`, the largest integer the canonical form admits.
    pub const MAX_SAFE_INT: i64 = 9_007_199_254_740_991;
}

/// Whether the artifact is structurally sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Structure {
    /// Parsed and every invariant held.
    Valid,
    /// Refused. [`ValidationReport::error`] says which rule and where.
    Invalid,
}

/// Whether the supplied source bytes are the bytes this artifact describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceBinding {
    /// The bytes hash to the artifact's `source.sha256`.
    Matched,
    /// The bytes were supplied and hash to something else.
    Mismatched,
    /// No source was supplied. **Not a pass** — a question that was not asked.
    NotChecked,
}

/// Array lengths, exactly as Ethos counts them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    /// `pages.len()`.
    pub pages: usize,
    /// `elements.len()`.
    pub elements: usize,
    /// `spans.len()`, or **0** when the key is absent.
    pub spans: usize,
    /// `tables.len()`, or **0** when the key is absent.
    ///
    /// Absent and empty both count 0 here, which is deliberate on Ethos's side and copied
    /// rather than improved: `counts` reports what is in the artifact, while the
    /// capability/array agreement rule is what distinguishes "no tables" from "did not look".
    pub tables: usize,
}

/// Why an artifact was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportError {
    /// Stable code, `^[a-z][a-z0-9_]*$`. Ethos's spelling.
    pub code: String,
    /// JSON pointer to the offending value.
    pub path: String,
    /// Human-readable detail.
    pub message: String,
}

/// The `ethos.grounding_validation.v1` report.
///
/// Emitted **bare**, matching `urn:ethos:schema:grounding-validation-report:1`. Ethos additionally
/// wraps its copy in an in-toto Statement (`_type` / `subject` / `predicateType` / `predicate`);
/// that envelope is Ethos's supply-chain framing and this engine makes no attestation claims, so
/// the report travels on its own. The oracle harness reads the report out of either shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    /// Const `ethos.grounding_validation.v1`.
    pub artifact_type: String,
    /// Const `1.0.0`.
    pub schema_version: String,
    /// Structural verdict.
    pub structure: Structure,
    /// Source-binding verdict.
    pub source_binding: SourceBinding,
    /// Present iff `structure == valid`. **The digest of the grounding file's raw bytes.**
    #[serde(skip_serializing_if = "Option::is_none")]
    pub representation_sha256: Option<Sha256Hex>,
    /// Present iff `structure == valid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<Counts>,
    /// Present iff `structure == invalid`, or the binding mismatched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ReportError>,
}

impl ValidationReport {
    fn invalid(code: &str, path: &str, message: &str) -> Self {
        Self {
            artifact_type: VALIDATION_ARTIFACT_TYPE.to_string(),
            schema_version: VALIDATION_SCHEMA_VERSION.to_string(),
            structure: Structure::Invalid,
            // Ethos reports `not_checked` on an invalid artifact even when a source was
            // supplied: there is no `source.sha256` to compare against, so the question was
            // never asked. Copied rather than reasoned about independently.
            source_binding: SourceBinding::NotChecked,
            representation_sha256: None,
            counts: None,
            error: Some(ReportError {
                code: code.to_string(),
                path: path.to_string(),
                message: message.to_string(),
            }),
        }
    }

    /// Canonical bytes.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the report will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        ethos_parser_core::c14n::canonical_bytes_of(self).map_err(|e| EngineError::Malformed {
            what: "validation report".into(),
            detail: e.to_string(),
        })
    }

    /// The engine's exit code for this outcome.
    ///
    /// | Outcome | engine | Ethos |
    /// | --- | --- | --- |
    /// | valid + `matched` / `not_checked` | **0** | 0 |
    /// | valid + `mismatched` | **1** | 2 |
    /// | invalid | **1** | 2 |
    /// | could not read the input at all | **2** | 2 |
    ///
    /// **The engine is finer, never contradictory.** Both agree on zero-versus-non-zero, which
    /// is what a shell predicate reads. Where they differ, the engine keeps its own three-code
    /// taxonomy (`docs/history/03-V0-SCOPE.md` §3.1): 1 is "I read it and the answer is no", 2 is "I
    /// could not read it". Collapsing those is the LiteParse defect this project exists to
    /// refuse — a caller cannot tell a failing check from an unreadable file. Ethos maps both to
    /// its `EXIT_USAGE`, which is a fair choice for a tool whose 1 already means *ungrounded*;
    /// it just is not this engine's.
    ///
    /// The oracle criterion is agreement on the **report**, not on the exit code, and the report
    /// is where the distinction is machine-readable either way.
    pub fn exit_code(&self) -> i32 {
        match (self.structure, self.source_binding) {
            (Structure::Valid, SourceBinding::Mismatched) => 1,
            (Structure::Valid, _) => 0,
            (Structure::Invalid, _) => 1,
        }
    }
}

/// Validate a grounding artifact, and optionally bind it to source bytes.
///
/// `grounding_json` is the **raw file bytes**, not a re-serialization: the report's
/// `representation_sha256` is their digest, so re-encoding first would produce a different
/// answer from the oracle's for the same file.
///
/// # Errors
///
/// [`EngineError::Unsupported`] when `source_pdf_bytes` is supplied and is not a PDF. That is a
/// refusal to answer rather than a `mismatched` verdict: Ethos rejects it before any report is
/// written, and reporting "these bytes are not the source" about a file that is not a document at
/// all would be a different, and wrong, statement. [`EngineError::ResourceLimit`] when the input
/// exceeds the accepted ceiling.
pub fn grounding_check(
    grounding_json: &[u8],
    source_pdf_bytes: Option<&[u8]>,
) -> Result<ValidationReport, EngineError> {
    if grounding_json.len() > limits::MAX_INPUT_BYTES {
        return Err(EngineError::ResourceLimit {
            limit: "grounding input bytes".into(),
            configured: limits::MAX_INPUT_BYTES.to_string(),
        });
    }

    // ORDER MATTERS, and it is Ethos's: the artifact is parsed and validated FIRST, and only a
    // structurally sound one goes on to have its source read. An earlier version checked the PDF
    // magic up front and claimed in a comment that this was "the same order Ethos uses" — it was
    // not, and the difference was visible: an invalid artifact handed an unusable source got no
    // report at all from the engine, while the oracle still reported the structural fault. A
    // report the verifier produces and the engine withholds is a disagreement.
    let artifact = match parse(grounding_json) {
        Ok(a) => a,
        Err(report) => return Ok(*report),
    };
    if let Err(report) = validate(&artifact) {
        return Ok(*report);
    }

    if let Some(bytes) = source_pdf_bytes {
        if !bytes.starts_with(b"%PDF-") {
            return Err(EngineError::Unsupported {
                what: "source artifact".into(),
                detail: "source artifact is not a PDF".into(),
            });
        }
    }

    let source_binding = match source_pdf_bytes {
        None => SourceBinding::NotChecked,
        Some(bytes) => {
            let actual = format!("sha256:{}", sha256_hex_bytes(bytes));
            if actual == artifact.source.sha256 {
                SourceBinding::Matched
            } else {
                SourceBinding::Mismatched
            }
        }
    };

    Ok(ValidationReport {
        artifact_type: VALIDATION_ARTIFACT_TYPE.to_string(),
        schema_version: VALIDATION_SCHEMA_VERSION.to_string(),
        structure: Structure::Valid,
        source_binding,
        // THE DIGEST OF THE FILE'S RAW BYTES. Measured from Ethos, not assumed: its
        // `parse_grounding_json` does `hash.update(bytes)` on the slice it was handed. This is
        // NOT `DocumentRepresentation::representation_c14n_sha256`, which digests a
        // representation's payload subtree — a different input for a different purpose, and the
        // two sharing most of a name is the reason this comment exists.
        representation_sha256: Some(
            Sha256Hex::from_hex(&sha256_hex_bytes(grounding_json))
                .expect("sha256 hex is always well formed"),
        ),
        counts: Some(Counts {
            pages: artifact.pages.len(),
            elements: artifact.elements.len(),
            spans: artifact.spans.as_ref().map_or(0, Vec::len),
            tables: artifact.tables.as_ref().map_or(0, Vec::len),
        }),
        error: (source_binding == SourceBinding::Mismatched).then(|| ReportError {
            code: "source_binding_mismatch".to_string(),
            path: "/source/sha256".to_string(),
            message: "source artifact bytes do not match source.sha256".to_string(),
        }),
    })
}

/// The keys each object in the contract may carry, and the path each lives at.
///
/// Transcribed from Ethos's `reject_unknown_fields`, which walks the parsed value and reports the
/// **precise path** of an unrecognised key. `deny_unknown_fields` on the typed structs catches the
/// same artifacts but can only say `/`, and the oracle compares the path.
fn reject_unknown_fields(value: &serde_json::Value) -> Result<(), (String, String)> {
    fn check(
        object: &serde_json::Map<String, serde_json::Value>,
        allowed: &[&str],
        path: &str,
    ) -> Result<(), (String, String)> {
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                let at = if path == "/" {
                    format!("/{key}")
                } else {
                    format!("{path}/{key}")
                };
                return Err(("unknown_field".to_string(), at));
            }
        }
        Ok(())
    }
    fn child(
        value: Option<&serde_json::Value>,
        allowed: &[&str],
        path: &str,
    ) -> Result<(), (String, String)> {
        if let Some(serde_json::Value::Object(o)) = value {
            check(o, allowed, path)?;
        }
        Ok(())
    }
    fn array(
        value: Option<&serde_json::Value>,
        allowed: &[&str],
        path: &str,
    ) -> Result<(), (String, String)> {
        if let Some(serde_json::Value::Array(items)) = value {
            for (i, item) in items.iter().enumerate() {
                if let Some(o) = item.as_object() {
                    check(o, allowed, &format!("{path}/{i}"))?;
                }
            }
        }
        Ok(())
    }

    let Some(root) = value.as_object() else {
        return Ok(());
    };
    check(
        root,
        &[
            "artifact_type",
            "schema_version",
            "source",
            "producer",
            "capabilities",
            "coordinate_system",
            "pages",
            "elements",
            "spans",
            "tables",
        ],
        "/",
    )?;
    child(root.get("source"), &["media_type", "sha256"], "/source")?;
    child(root.get("producer"), &["name", "version"], "/producer")?;
    child(
        root.get("capabilities"),
        &["spans", "char_offsets", "tables"],
        "/capabilities",
    )?;
    child(
        root.get("coordinate_system"),
        &["unit", "origin"],
        "/coordinate_system",
    )?;
    array(
        root.get("pages"),
        &["id", "index", "width", "height", "rotation"],
        "/pages",
    )?;
    array(
        root.get("elements"),
        &["id", "page", "bbox", "kind", "text", "locator"],
        "/elements",
    )?;
    array(
        root.get("spans"),
        &[
            "id",
            "page",
            "bbox",
            "text",
            "element",
            "char_start",
            "char_end",
        ],
        "/spans",
    )?;
    if let Some(serde_json::Value::Array(tables)) = root.get("tables") {
        for (i, table) in tables.iter().enumerate() {
            let path = format!("/tables/{i}");
            if let Some(o) = table.as_object() {
                check(o, &["id", "page", "bbox", "cells"], &path)?;
                array(
                    o.get("cells"),
                    &["row", "col", "row_span", "col_span", "bbox", "text"],
                    &format!("{path}/cells"),
                )?;
            }
        }
    }
    Ok(())
}

/// Nesting at or past this depth is `limit_exceeded`.
const MAX_DEPTH: usize = 64;

/// Ethos's `StrictValueSeed`, rule for rule, refusing **in the order the bytes arrive**.
///
/// Nulls and floats are refused outright; integer magnitude (as `unsigned_abs`, so `i64::MIN` is
/// over it), string bytes, nesting depth and array length are capped; a key repeated within one
/// object is refused. The messages matter: [`parse_as_ethos`] classifies them exactly as
/// `parse_grounding_json` does — "duplicate object key", then anything saying "limit exceeded",
/// then everything else as invalid JSON.
///
/// Building the value is separate from refusing, so each [`Pass`] does only what its caller needs.
#[derive(Clone, Copy)]
struct Strict {
    depth: usize,
    pass: Pass,
}

/// What a [`Strict`] walk does beyond applying the value rules.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    /// Nothing: no value, and repeated keys go unlooked-for. [`parse`] says why that is enough.
    Scan,
    /// Repeated keys too, so its first error is the seed's first error, found without building.
    Refusals,
    /// Everything the seed does: refuses as [`Pass::Refusals`] does, and builds the value with each
    /// object's members **in sorted key order** — the order Ethos's `BTreeMap`-backed map iterates
    /// in, whether or not this build unifies serde_json's `preserve_order` — so
    /// `reject_unknown_fields` names the same key and `from_value` meets fields in the same order.
    Build,
}

impl<'de> serde::de::DeserializeSeed<'de> for Strict {
    type Value = Option<serde_json::Value>;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for Strict {
    type Value = Option<serde_json::Value>;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("strict JSON value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(self.built(|| v.into()))
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if v.unsigned_abs() > limits::MAX_SAFE_INT as u64 {
            return Err(E::custom("integer limit exceeded"));
        }
        Ok(self.built(|| v.into()))
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        if v > limits::MAX_SAFE_INT as u64 {
            return Err(E::custom("integer limit exceeded"));
        }
        Ok(self.built(|| v.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
        Err(E::custom("floating point values are not allowed"))
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        if v.len() > limits::MAX_STRING_BYTES {
            return Err(E::custom("string limit exceeded"));
        }
        Ok(self.built(|| v.into()))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Err(E::custom("null values are not allowed"))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        use serde::de::Error as _;
        let inner = self.inner::<A::Error>()?;
        let (mut len, mut out) = (0, Vec::new());
        // Counted after each item is read, as Ethos does: a fault inside the item past the limit
        // is reported before the limit is.
        while let Some(item) = seq.next_element_seed(inner)? {
            if len >= limits::MAX_ELEMENTS {
                return Err(A::Error::custom("array limit exceeded"));
            }
            len += 1;
            out.extend(item);
        }
        Ok(self.built(|| serde_json::Value::Array(out)))
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        use serde::de::Error as _;
        let inner = self.inner::<A::Error>()?;
        if self.pass == Pass::Scan {
            while map.next_key::<serde::de::IgnoredAny>()?.is_some() {
                map.next_value_seed(inner)?;
            }
            return Ok(None);
        }
        let mut keys = std::collections::HashSet::new();
        let mut members = Vec::new();
        while let Some(key) = map.next_key_seed(Key)? {
            if !keys.insert(key.clone()) {
                return Err(A::Error::custom("duplicate object key"));
            }
            if let Some(value) = map.next_value_seed(inner)? {
                members.push((key.into_owned(), value));
            }
        }
        Ok(self.built(|| {
            members.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            serde_json::Value::Object(members.into_iter().collect())
        }))
    }
}

impl Strict {
    /// The seed for a container's members, or the depth refusal Ethos makes on entering it.
    fn inner<E: serde::de::Error>(self) -> Result<Self, E> {
        if self.depth >= MAX_DEPTH {
            return Err(E::custom("depth limit exceeded"));
        }
        Ok(Self {
            depth: self.depth + 1,
            ..self
        })
    }

    fn built(self, value: impl FnOnce() -> serde_json::Value) -> Option<serde_json::Value> {
        (self.pass == Pass::Build).then(value)
    }

    /// Walk all of `bytes`, then require that nothing follows the value.
    fn walk(self, bytes: &[u8]) -> Result<Option<serde_json::Value>, WalkError> {
        use serde::de::DeserializeSeed as _;
        let mut de = serde_json::Deserializer::from_slice(bytes);
        let value = self.deserialize(&mut de).map_err(WalkError::Value)?;
        de.end().map_err(WalkError::Trailing)?;
        Ok(value)
    }
}

/// Where a [`Strict::walk`] stopped: inside the value, or after it.
enum WalkError {
    Value(serde_json::Error),
    Trailing(serde_json::Error),
}

/// An object key, borrowed from the input unless it has escapes, so looking for a repeated one
/// allocates only the keys a build keeps.
struct Key;

impl<'de> serde::de::DeserializeSeed<'de> for Key {
    type Value = std::borrow::Cow<'de, str>;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_str(self)
    }
}

impl<'de> serde::de::Visitor<'de> for Key {
    type Value = std::borrow::Cow<'de, str>;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an object key")
    }
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E> {
        Ok(v.into())
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(v.to_owned().into())
    }
}

/// Parse, with Ethos's lexical refusals: `parse_grounding_json` up to, not including, `validate`.
fn parse(bytes: &[u8]) -> Result<GroundingSource, Box<ValidationReport>> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(Box::new(ValidationReport::invalid(
            "bom_not_allowed",
            "/",
            "input begins with a UTF-8 byte order mark",
        )));
    }

    // Every artifact that parses takes this route, and builds no `Value` — which was
    // `grounding-check`'s peak, at up to 13.5 times the artifact's bytes. It accepts exactly what
    // `parse_as_ethos` accepts. The scan applies every rule of the seed but one, repeated keys, and
    // a typed parse that succeeds has none: every object it reads is a derived struct, which
    // refuses a repeated field and, being `deny_unknown_fields`, any other. For the same reason it
    // leaves `reject_unknown_fields` nothing to find, since every type that walk visits allows
    // exactly its keys. And the typed parse accepts the bytes exactly when it accepts the value the
    // seed would build.
    let scan = Strict {
        depth: 0,
        pass: Pass::Scan,
    };
    let scanned = scan.walk(bytes).is_ok();
    if scanned {
        if let Ok(artifact) = serde_json::from_slice::<GroundingSource>(bytes) {
            return Ok(artifact);
        }
    }
    // A scan that refused found a fault the seed finds too, so Ethos's answer is a refusal, and
    // `Pass::Refusals` names it without building. A scan that passed leaves the seed only a repeated
    // key to refuse, so the one pass that builds looks for it on the way.
    parse_as_ethos(bytes, if scanned { Pass::Build } else { Pass::Refusals })
}

/// `parse_grounding_json`'s steps after the BOM, in its order and with its classification.
///
/// With [`Pass::Build`] this is Ethos's walk exactly. With [`Pass::Refusals`] it refuses without
/// building, which reaches the same answer — every refusal is decided by the bytes read so far,
/// never by what has been built — and keeps an artifact refused for a limit, such as more than a
/// million spans, from costing a value of everything before the limit. Should that pass refuse
/// nothing, the value is built after all.
fn parse_as_ethos(bytes: &[u8], pass: Pass) -> Result<GroundingSource, Box<ValidationReport>> {
    let invalid = |code: &str, path: &str, message: &str| {
        Box::new(ValidationReport::invalid(code, path, message))
    };

    let value = match (Strict { depth: 0, pass }).walk(bytes) {
        Ok(Some(value)) => value,
        Ok(None) => return parse_as_ethos(bytes, Pass::Build),
        Err(WalkError::Value(e)) => {
            let text = e.to_string();
            let code = if text.contains("duplicate object key") {
                "duplicate_key"
            } else if text.contains("limit exceeded") {
                "limit_exceeded"
            } else {
                "invalid_json"
            };
            return Err(invalid(code, "/", &text));
        }
        Err(WalkError::Trailing(e)) => return Err(invalid("invalid_json", "/", &e.to_string())),
    };
    if let Err((code, path)) = reject_unknown_fields(&value) {
        return Err(invalid(
            &code,
            &path,
            "object carries a field outside the contract",
        ));
    }
    // Ethos's two-way rule, verbatim, including that the text it reads can quote the input.
    serde_json::from_value::<GroundingSource>(value).map_err(|e| {
        let text = e.to_string();
        let code = if text.contains("unknown field") {
            "unknown_field"
        } else {
            "invalid_field"
        };
        invalid(code, "/", &text)
    })
}

/// Ethos's `validate`, rule for rule and **in its order**.
///
/// Order is load-bearing: an artifact with two faults reports the first one reached, and the
/// oracle compares the code and the path. Reordering these to read more nicely would produce a
/// checker that agrees on the verdict and disagrees on the reason.
fn validate(a: &GroundingSource) -> Result<(), Box<ValidationReport>> {
    // Boxed because the "error" here is a full report, not a small code — and it is a verdict
    // rather than a failure, so it travels the `Err` channel purely to short-circuit.
    let bad = |code: &str, path: &str, message: &str| -> Result<(), Box<ValidationReport>> {
        Err(Box::new(ValidationReport::invalid(code, path, message)))
    };

    if a.artifact_type != crate::GROUNDING_ARTIFACT_TYPE
        || !matches!(a.schema_version.as_str(), "1.0.0" | "1.1.0")
    {
        return bad(
            "unsupported_version",
            "/artifact_type",
            "use ethos.grounding.v1 with schema_version 1.0.0 or 1.1.0",
        );
    }

    // Mirrors the Ethos intake exactly: 1.0.0 is PDF and nothing else; 1.1.0
    // adds the eight page-less types, each held to the page-less shape below.
    let page_less = a.source.media_type != "application/pdf";
    let media_admitted = !page_less
        || (a.schema_version == "1.1.0"
            && crate::PAGE_LESS_MEDIA_TYPES.contains(&a.source.media_type.as_str()));
    let sha = &a.source.sha256;
    if !media_admitted
        || !sha.starts_with("sha256:")
        || sha.len() != 71
        || !sha[7..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return bad("invalid_field", "/source", "source identity is malformed");
    }

    if a.coordinate_system.unit != "centipoint" || a.coordinate_system.origin != "top-left" {
        return bad(
            "invalid_invariant",
            "/coordinate_system",
            "coordinate system must be centipoint / top-left",
        );
    }

    if a.capabilities.char_offsets && !a.capabilities.spans
        || a.capabilities.spans != a.spans.is_some()
        || a.capabilities.tables != a.tables.is_some()
    {
        return bad(
            "invalid_capabilities",
            "/capabilities",
            "declared capabilities disagree with the arrays present",
        );
    }

    if page_less {
        if !a.pages.is_empty() {
            return bad(
                "invalid_invariant",
                "/pages",
                "a page-less source states no page",
            );
        }
        if a.spans.as_ref().is_some_and(|s| !s.is_empty())
            || a.tables.as_ref().is_some_and(|t| !t.is_empty())
        {
            return bad(
                "invalid_invariant",
                "/spans",
                "a page-less source carries no spans and no tables",
            );
        }
        for (i, e) in a.elements.iter().enumerate() {
            let path = format!("/elements/{i}");
            if e.page.is_some() || e.bbox.is_some() {
                return bad(
                    "invalid_field",
                    &path,
                    "a page-less element states no page and no bbox",
                );
            }
            match e.locator.as_deref() {
                Some(locator)
                    if !locator.is_empty() && locator.len() <= limits::MAX_LOCATOR_BYTES => {}
                _ => {
                    return bad(
                        "invalid_field",
                        &format!("{path}/locator"),
                        "a page-less element carries its native locator",
                    )
                }
            }
        }
    }
    if a.pages.len() > limits::MAX_PAGES
        || a.elements.len() > limits::MAX_ELEMENTS
        || a.spans
            .as_ref()
            .is_some_and(|v| v.len() > limits::MAX_ELEMENTS)
        || a.tables
            .as_ref()
            .is_some_and(|v| v.len() > limits::MAX_TABLES)
    {
        return bad(
            "limit_exceeded",
            "/",
            "an accepted structural limit was exceeded",
        );
    }

    // Producer strings are bounded during Ethos's deserialization rather than in `validate`,
    // so they are checked here to keep the same artifacts acceptable to both.
    for (field, value) in [("name", &a.producer.name), ("version", &a.producer.version)] {
        if value.len() > limits::MAX_STRING_BYTES {
            return bad(
                "limit_exceeded",
                &format!("/producer/{field}"),
                "string exceeds the accepted byte limit",
            );
        }
    }

    let mut page_by_id: std::collections::HashMap<&str, &Page> = Default::default();
    let mut expected = 1u32;
    for (i, p) in a.pages.iter().enumerate() {
        let path = format!("/pages/{i}");
        if !valid_id(&p.id) {
            return bad("invalid_field", &format!("{path}/id"), "malformed id");
        }
        if page_by_id.insert(p.id.as_str(), p).is_some() {
            return bad("duplicate_id", &format!("{path}/id"), "repeated id");
        }
        if p.index != expected {
            return bad(
                "invalid_order",
                &format!("{path}/index"),
                "page indices must ascend from 1 with no gaps",
            );
        }
        if p.width <= 0
            || p.height <= 0
            || p.width > limits::MAX_SAFE_INT
            || p.height > limits::MAX_SAFE_INT
        {
            return bad(
                "invalid_invariant",
                &path,
                "page dimensions are out of range",
            );
        }
        if !matches!(p.rotation, 0 | 90 | 180 | 270) {
            return bad(
                "invalid_invariant",
                &format!("{path}/rotation"),
                "rotation must be 0, 90, 180 or 270",
            );
        }
        expected += 1;
    }

    let mut ids: std::collections::HashSet<&str> = Default::default();
    for (i, e) in a.elements.iter().enumerate() {
        let path = format!("/elements/{i}");
        if !valid_id(&e.id) {
            return bad("invalid_field", &format!("{path}/id"), "malformed id");
        }
        if !ids.insert(e.id.as_str()) {
            return bad("duplicate_id", &format!("{path}/id"), "repeated id");
        }
        if !page_less {
            let (Some(page), Some(bbox)) = (e.page.as_deref(), e.bbox) else {
                return bad(
                    "invalid_field",
                    &path,
                    "a paginated element states its page and bbox",
                );
            };
            if e.locator.is_some() {
                return bad(
                    "invalid_field",
                    &format!("{path}/locator"),
                    "a locator belongs only to the page-less shape",
                );
            }
            if !page_by_id.contains_key(page) {
                return bad(
                    "unknown_reference",
                    &format!("{path}/page"),
                    "page reference does not resolve",
                );
            }
            if !valid_bbox(bbox, page_by_id.get(page).copied()) {
                return bad(
                    "invalid_bbox",
                    &format!("{path}/bbox"),
                    "bbox is malformed or outside its page",
                );
            }
        }
        if !valid_kind(&e.kind) {
            return bad("invalid_field", &format!("{path}/kind"), "malformed kind");
        }
        if e.text
            .as_ref()
            .is_some_and(|t| t.len() > limits::MAX_STRING_BYTES)
        {
            return bad(
                "limit_exceeded",
                &format!("{path}/text"),
                "string exceeds the accepted byte limit",
            );
        }
    }

    if let Some(spans) = &a.spans {
        let element_by_id: std::collections::HashMap<&str, &Element> =
            a.elements.iter().map(|e| (e.id.as_str(), e)).collect();
        let mut seen: std::collections::HashSet<&str> = Default::default();
        for (i, s) in spans.iter().enumerate() {
            let path = format!("/spans/{i}");
            if !valid_id(&s.id) {
                return bad("invalid_field", &format!("{path}/id"), "malformed id");
            }
            if !seen.insert(s.id.as_str()) {
                return bad("duplicate_id", &format!("{path}/id"), "repeated id");
            }
            if !page_by_id.contains_key(s.page.as_str()) {
                return bad(
                    "unknown_reference",
                    &format!("{path}/page"),
                    "page reference does not resolve",
                );
            }
            if !valid_bbox(s.bbox, page_by_id.get(s.page.as_str()).copied()) {
                return bad(
                    "invalid_bbox",
                    &format!("{path}/bbox"),
                    "bbox is malformed or outside its page",
                );
            }
            if s.element
                .as_ref()
                .is_some_and(|id| !ids.contains(id.as_str()))
            {
                return bad(
                    "unknown_reference",
                    &format!("{path}/element"),
                    "element reference does not resolve",
                );
            }
            if s.text.len() > limits::MAX_STRING_BYTES {
                return bad(
                    "limit_exceeded",
                    &format!("{path}/text"),
                    "string exceeds the accepted byte limit",
                );
            }

            // Offsets index the ELEMENT's text in Unicode scalars, and are present exactly when
            // the capability is claimed. Both halves are Ethos's, measured from its source.
            let present = s.char_start.is_some() || s.char_end.is_some();
            let complete = s.char_start.is_some() && s.char_end.is_some();
            let matches = match (s.element.as_ref(), s.char_start, s.char_end) {
                (Some(id), Some(start), Some(end)) => element_by_id
                    .get(id.as_str())
                    .copied()
                    .and_then(|e| e.text.as_ref())
                    .map(|text| {
                        let chars: Vec<char> = text.chars().collect();
                        start <= end
                            && end as usize <= chars.len()
                            && chars[start as usize..end as usize]
                                .iter()
                                .collect::<String>()
                                == s.text
                    })
                    .unwrap_or(false),
                _ => false,
            };
            if present != a.capabilities.char_offsets
                || (a.capabilities.char_offsets && (!complete || !matches))
            {
                return bad(
                    "invalid_offsets",
                    &path,
                    "character offsets do not match the owning element's text",
                );
            }
        }
    }

    if let Some(tables) = &a.tables {
        let mut seen: std::collections::HashSet<&str> = Default::default();
        for (i, t) in tables.iter().enumerate() {
            let path = format!("/tables/{i}");
            if !valid_id(&t.id) {
                return bad("invalid_field", &format!("{path}/id"), "malformed id");
            }
            if !seen.insert(t.id.as_str()) {
                return bad("duplicate_id", &format!("{path}/id"), "repeated id");
            }
            if !page_by_id.contains_key(t.page.as_str()) {
                return bad(
                    "unknown_reference",
                    &format!("{path}/page"),
                    "page reference does not resolve",
                );
            }
            if !valid_bbox(t.bbox, page_by_id.get(t.page.as_str()).copied()) {
                return bad(
                    "invalid_bbox",
                    &format!("{path}/bbox"),
                    "bbox is malformed or outside its page",
                );
            }
            if t.cells.len() > limits::MAX_CELLS {
                return bad("limit_exceeded", &format!("{path}/cells"), "too many cells");
            }
            if let Err(code) = validate_cells(t, page_by_id.get(t.page.as_str()).copied()) {
                return bad(
                    code,
                    &format!("{path}/cells"),
                    "table cell invariant failed",
                );
            }
        }
    }

    Ok(())
}

/// Cells ascend by `(row, col)`, span at least one, sit inside the page, and never overlap.
fn validate_cells(t: &Table, page: Option<&Page>) -> Result<(), &'static str> {
    let mut occupied: std::collections::HashSet<(u32, u32)> = Default::default();
    let mut previous: Option<(u32, u32)> = None;
    for c in &t.cells {
        let row_end = c.row.checked_add(c.row_span);
        let col_end = c.col.checked_add(c.col_span);
        if c.text.len() > limits::MAX_STRING_BYTES {
            // Ethos bounds every string during deserialization, so an oversized cell text is
            // refused before `validate` runs and reports `limit_exceeded` at `/`. Element and
            // span text were bounded here from the start; the cell was simply missed, and the
            // gap let the engine say `valid` where the verifier says `invalid` — the one
            // direction `docs/01-CONTRACT.md` §11 forbids.
            return Err("limit_exceeded");
        }
        if c.row_span == 0
            || c.col_span == 0
            || !valid_bbox(c.bbox, page)
            || previous.is_some_and(|(row, col)| (c.row, c.col) <= (row, col))
            || row_end.is_none()
            || col_end.is_none()
        {
            return Err("invalid_table");
        }
        previous = Some((c.row, c.col));
        for row in c.row..row_end.expect("checked") {
            for col in c.col..col_end.expect("checked") {
                if !occupied.insert((row, col)) {
                    return Err("invalid_table");
                }
                if occupied.len() > limits::MAX_CELLS {
                    return Err("limit_exceeded");
                }
            }
        }
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= limits::MAX_ID_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
        && value.as_bytes()[0].is_ascii_alphanumeric()
}

/// Non-empty, and lowercase/digit/underscore/hyphen. **No length bound**, deliberately.
///
/// The pinned JSON Schema says `maxLength: 256` for `kind`; Ethos's parser does not, and bounds it
/// only by the generic 16 384-byte string limit. The two authorities genuinely disagree, and for a
/// *checker* the oracle wins: being stricter than the verifier here does not make the engine
/// safer, it makes the two of them return different verdicts on the same file, which is the one
/// thing M6 exists to prevent. (Stricter-on-*emission* is still the permitted direction, and the
/// engine emits `text_run`.)
fn valid_kind(kind: &str) -> bool {
    !kind.is_empty()
        && kind
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

/// A box is ordered, non-negative, and **inside its page**.
fn valid_bbox(b: [i64; 4], page: Option<&Page>) -> bool {
    page.is_some_and(|p| {
        b.iter().all(|v| *v >= 0 && *v <= limits::MAX_SAFE_INT)
            && b[2] > b[0]
            && b[3] > b[1]
            && b[2] <= p.width
            && b[3] <= p.height
    })
}

/// Unused-import guard: `Span` is named in the signatures above via `spans`.
#[allow(dead_code)]
fn _span_type_is_used(_: &Span) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The shape a document past the span cap is projected into is a valid artifact (G1).**
    ///
    /// `project` withholds the spans of a document with more than `limits::MAX_ELEMENTS` of them,
    /// and says so with `capabilities.spans: false` and no `spans` key. That is only a fix if the
    /// validator accepts it for a PDF source — the page-less path has always emitted it, so this
    /// pins it for the path that never did.
    #[test]
    fn a_pdf_artifact_with_its_spans_withheld_is_valid() {
        let mut v: serde_json::Value = serde_json::from_slice(&valid_bytes()).unwrap();
        v["capabilities"]["spans"] = serde_json::Value::Bool(false);
        v.as_object_mut().unwrap().remove("spans");
        let bytes = serde_json::to_vec(&v).unwrap();
        let r = grounding_check(&bytes, None).unwrap();
        assert_eq!(
            r.exit_code(),
            0,
            "{}",
            String::from_utf8_lossy(&r.to_canonical_bytes().unwrap())
        );
    }

    /// **The shape a document's tables are withheld into is a valid artifact, and the shape it
    /// replaces is not (G2).** A table holding a cell past the string limit is refused whole, at `/`;
    /// the same artifact with `capabilities.tables: false` and no `tables` key is accepted; and an
    /// EMPTY array under a false capability is refused too — which pins that withholding means the
    /// key is absent.
    #[test]
    fn a_pdf_artifact_with_its_tables_withheld_is_valid() {
        let mut with: serde_json::Value = serde_json::from_slice(&valid_bytes()).unwrap();
        with["capabilities"]["tables"] = serde_json::Value::Bool(true);
        with["tables"] = serde_json::json!([{
            "id": "t1", "page": "p1", "bbox": [10, 10, 100, 100],
            "cells": [{ "row": 0, "col": 0, "row_span": 1, "col_span": 1,
                        "bbox": [10, 10, 100, 100], "text": "x".repeat(limits::MAX_STRING_BYTES + 1) }]
        }]);
        let r = grounding_check(&serde_json::to_vec(&with).unwrap(), None).unwrap();
        assert_eq!(r.structure, Structure::Invalid);
        let e = r.error.as_ref().expect("an error");
        assert_eq!((e.code.as_str(), e.path.as_str()), ("limit_exceeded", "/"));

        let withheld = valid_bytes();
        let r = grounding_check(&withheld, None).unwrap();
        assert_eq!(
            r.exit_code(),
            0,
            "tables: false with no key is the withheld shape"
        );

        let mut empty: serde_json::Value = serde_json::from_slice(&valid_bytes()).unwrap();
        empty["tables"] = serde_json::json!([]);
        let r = grounding_check(&serde_json::to_vec(&empty).unwrap(), None).unwrap();
        assert_eq!(
            r.structure,
            Structure::Invalid,
            "an empty array is not a withheld table set"
        );
    }

    /// A minimal valid artifact, built through the public types so it cannot drift.
    fn valid_bytes() -> Vec<u8> {
        let g = GroundingSource {
            artifact_type: crate::GROUNDING_ARTIFACT_TYPE.into(),
            schema_version: crate::GROUNDING_SCHEMA_VERSION.into(),
            source: crate::Source {
                media_type: "application/pdf".into(),
                sha256: format!("sha256:{}", sha256_hex_bytes(b"%PDF-1.7 fake")),
            },
            producer: crate::Producer {
                name: "ethos-parser".into(),
                version: "0.0.0".into(),
            },
            capabilities: crate::GroundingCapabilities {
                spans: true,
                char_offsets: false,
                tables: false,
            },
            coordinate_system: crate::GroundingCoordinateSystem {
                unit: "centipoint".into(),
                origin: "top-left".into(),
            },
            pages: vec![Page {
                id: "p1".into(),
                index: 1,
                width: 30000,
                height: 14400,
                rotation: 0,
            }],
            elements: vec![Element {
                id: "e1".into(),
                page: Some("p1".into()),
                bbox: Some([10, 10, 100, 100]),
                kind: "text_run".into(),
                text: Some("hello".into()),
                locator: None,
            }],
            spans: Some(vec![Span {
                id: "s1".into(),
                page: "p1".into(),
                bbox: [10, 10, 100, 100],
                text: "hello".into(),
                element: Some("e1".into()),
                char_start: None,
                char_end: None,
            }]),
            tables: None,
        };
        crate::to_canonical_bytes(&g).unwrap()
    }

    #[test]
    fn a_valid_artifact_reports_valid_with_counts_and_the_file_digest() {
        let bytes = valid_bytes();
        let r = grounding_check(&bytes, None).unwrap();
        assert_eq!(r.structure, Structure::Valid);
        assert_eq!(r.source_binding, SourceBinding::NotChecked);
        assert_eq!(
            r.counts.expect("valid carries counts"),
            Counts {
                pages: 1,
                elements: 1,
                spans: 1,
                tables: 0
            }
        );
        // The digest is of the FILE, not of anything re-serialized.
        assert_eq!(
            r.representation_sha256
                .as_ref()
                .expect("valid carries a digest")
                .hex(),
            sha256_hex_bytes(&bytes)
        );
        assert_eq!(r.exit_code(), 0);
    }

    #[test]
    fn the_digest_follows_the_bytes_not_the_meaning() {
        // Two files with identical meaning and different bytes must digest differently — that is
        // what makes this a file digest rather than a canonical one, and it is the property the
        // oracle agreement depends on.
        let a = valid_bytes();
        let mut b = a.clone();
        b.push(b'\n');
        let ra = grounding_check(&a, None).unwrap();
        let rb = grounding_check(&b, None).unwrap();
        assert_ne!(ra.representation_sha256, rb.representation_sha256);
        assert_eq!(ra.counts, rb.counts, "the meaning is the same");
    }

    #[test]
    fn the_source_binding_trichotomy() {
        let bytes = valid_bytes();

        let matched = grounding_check(&bytes, Some(b"%PDF-1.7 fake")).unwrap();
        assert_eq!(matched.source_binding, SourceBinding::Matched);
        assert!(matched.error.is_none());
        assert_eq!(matched.exit_code(), 0);

        let mismatched = grounding_check(&bytes, Some(b"%PDF-1.7 other")).unwrap();
        assert_eq!(mismatched.source_binding, SourceBinding::Mismatched);
        assert_eq!(
            mismatched.structure,
            Structure::Valid,
            "the artifact is fine"
        );
        let e = mismatched
            .error
            .as_ref()
            .expect("mismatch carries an error");
        assert_eq!(e.code, "source_binding_mismatch");
        assert_eq!(e.path, "/source/sha256");
        assert_eq!(mismatched.exit_code(), 1);

        let not_checked = grounding_check(&bytes, None).unwrap();
        assert_eq!(not_checked.source_binding, SourceBinding::NotChecked);
    }

    #[test]
    fn a_non_pdf_source_is_refused_rather_than_called_a_mismatch() {
        // "These bytes are not the source" is a different statement from "this is not a
        // document". Ethos refuses before writing any report; so does this.
        let err = grounding_check(&valid_bytes(), Some(b"not a pdf")).unwrap_err();
        assert_eq!(err.code(), "unsupported");
        assert!(err.to_string().contains("not a PDF"));
    }

    #[test]
    fn each_structural_rule_is_reachable_with_ethoss_code_and_path() {
        // One break per rule. The code AND the path are asserted, because the oracle compares
        // both and a checker that agrees on `invalid` while disagreeing on why is only half
        // agreeing.
        /// (label, expected code, expected path, how to break exactly one rule)
        type Case = (
            &'static str,
            &'static str,
            &'static str,
            Box<dyn Fn(&mut serde_json::Value)>,
        );
        let cases: Vec<Case> = vec![
            (
                "artifact_type",
                "unsupported_version",
                "/artifact_type",
                Box::new(|v| v["artifact_type"] = "ethos.grounding.v2".into()),
            ),
            (
                "source sha",
                "invalid_field",
                "/source",
                Box::new(|v| v["source"]["sha256"] = "sha256:XY".into()),
            ),
            (
                "coordinate system",
                "invalid_invariant",
                "/coordinate_system",
                Box::new(|v| v["coordinate_system"]["unit"] = "point".into()),
            ),
            (
                "capabilities vs arrays",
                "invalid_capabilities",
                "/capabilities",
                Box::new(|v| v["capabilities"]["tables"] = true.into()),
            ),
            (
                "page id",
                "invalid_field",
                "/pages/0/id",
                Box::new(|v| v["pages"][0]["id"] = "-bad".into()),
            ),
            (
                "page order",
                "invalid_order",
                "/pages/0/index",
                Box::new(|v| v["pages"][0]["index"] = 2.into()),
            ),
            (
                "page dims",
                "invalid_invariant",
                "/pages/0",
                Box::new(|v| v["pages"][0]["width"] = 0.into()),
            ),
            (
                "rotation",
                "invalid_invariant",
                "/pages/0/rotation",
                Box::new(|v| v["pages"][0]["rotation"] = 45.into()),
            ),
            (
                "element page ref",
                "unknown_reference",
                "/elements/0/page",
                Box::new(|v| v["elements"][0]["page"] = "p9".into()),
            ),
            (
                "element bbox outside page",
                "invalid_bbox",
                "/elements/0/bbox",
                Box::new(|v| v["elements"][0]["bbox"][3] = 99999.into()),
            ),
            (
                "element kind",
                "invalid_field",
                "/elements/0/kind",
                Box::new(|v| v["elements"][0]["kind"] = "Text_Run".into()),
            ),
            (
                "span element ref",
                "unknown_reference",
                "/spans/0/element",
                Box::new(|v| v["spans"][0]["element"] = "e9".into()),
            ),
            (
                "offsets without the capability",
                "invalid_offsets",
                "/spans/0",
                Box::new(|v| v["spans"][0]["char_start"] = 0.into()),
            ),
        ];

        for (label, code, path, mutate) in cases {
            let mut v: serde_json::Value = serde_json::from_slice(&valid_bytes()).unwrap();
            mutate(&mut v);
            let bytes = serde_json::to_vec(&v).unwrap();
            let r = grounding_check(&bytes, None).unwrap();
            assert_eq!(r.structure, Structure::Invalid, "{label} should be invalid");
            let e = r.error.as_ref().expect("invalid carries an error");
            assert_eq!(e.code, code, "{label}: wrong code");
            assert_eq!(e.path, path, "{label}: wrong path");
            assert!(r.representation_sha256.is_none(), "{label}");
            assert!(r.counts.is_none(), "{label}");
            assert_eq!(r.exit_code(), 1);
        }
    }

    #[test]
    fn lexical_refusals_match_ethoss_codes() {
        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend_from_slice(&valid_bytes());
        assert_eq!(
            grounding_check(&bom, None).unwrap().error.unwrap().code,
            "bom_not_allowed"
        );

        assert_eq!(
            grounding_check(b"{", None).unwrap().error.unwrap().code,
            "invalid_json"
        );

        // An unknown field, and a repeated one. Both are caught by the derived parse because
        // every wire type here denies unknown fields.
        let extra = br#"{"artifact_type":"ethos.grounding.v1","extra":1}"#;
        assert_eq!(
            grounding_check(extra, None).unwrap().error.unwrap().code,
            "unknown_field"
        );
        let dup = br#"{"artifact_type":"a","artifact_type":"b"}"#;
        assert_eq!(
            grounding_check(dup, None).unwrap().error.unwrap().code,
            "duplicate_key"
        );
    }

    /// **Ethos's answer, in each of the ways this checker used to give a different one.** Every
    /// expectation was read from `ethos grounding check` v0.6.0 on these exact bytes, and 10 of these
    /// 15 differed before the checker ran Ethos's seed: stream order decides between two faults, the
    /// classification reads the error text as Ethos does, keys are walked in sorted order, and an
    /// array root's absent `spans` and `tables` default as Ethos's do.
    #[test]
    fn parsing_answers_as_ethos_does() {
        let base = String::from_utf8(valid_bytes()).unwrap();
        let edit = |from: &str, to: &str| {
            assert!(base.contains(from), "{from}");
            base.replacen(from, to, 1)
        };
        let root = |members: &str| format!("{{{members}{}", &base[1..]);
        let deep = format!("{}{}", "[".repeat(130), "]".repeat(130));
        let items = |n: usize, last: &str| format!("[{}{last}]", "0,".repeat(n));
        let sha = format!("sha256:{}", sha256_hex_bytes(b"%PDF-1.7 fake"));
        let array_root = |spans: bool| {
            format!(
                r#"["ethos.grounding.v1","1.0.0",["application/pdf","{sha}"],["n","v"],[{spans},false,false],["centipoint","top-left"],[],[]]"#
            )
        };
        /// (label, input, the code and path Ethos reports, or `None` for valid)
        type Case = (&'static str, String, Option<(&'static str, &'static str)>);
        let cases: Vec<Case> = vec![
            (
                "an integer at i64::MIN",
                edit(r#""width":30000"#, r#""width":-9223372036854775808"#),
                Some(("limit_exceeded", "/")),
            ),
            (
                "an unknown key, repeated",
                root(r#""zzz":1,"zzz":1,"#),
                Some(("duplicate_key", "/")),
            ),
            (
                "a null before its key repeats",
                edit(r#""kind":"text_run""#, r#""kind":"text_run","text":null"#),
                Some(("invalid_json", "/")),
            ),
            (
                "two unknown keys, out of order",
                root(r#""zzz":1,"aaa":1,"#),
                Some(("unknown_field", "/aaa")),
            ),
            (
                "nesting past serde_json's own limit",
                root(&format!(r#""zzz":{deep},"#)),
                Some(("limit_exceeded", "/")),
            ),
            (
                "a null, then that nesting",
                root(&format!(r#""zzz":[null,{deep}],"#)),
                Some(("invalid_json", "/")),
            ),
            (
                "a bbox with a fifth item",
                edit("[10,10,100,100]", "[10,10,100,100,1]"),
                Some(("invalid_field", "/")),
            ),
            (
                "a type error quoting \"unknown field\"",
                edit(r#""index":1"#, r#""index":"unknown field""#),
                Some(("unknown_field", "/")),
            ),
            (
                "a type error quoting \"EOF\"",
                edit(r#""index":1"#, r#""index":"EOF""#),
                Some(("invalid_field", "/")),
            ),
            (
                "a million and one items",
                root(&format!(r#""zzz":{},"#, items(1_000_000, "0"))),
                Some(("limit_exceeded", "/")),
            ),
            (
                "a million items",
                root(&format!(r#""zzz":{},"#, items(999_999, "0"))),
                Some(("unknown_field", "/zzz")),
            ),
            (
                "a null as the item past a million",
                root(&format!(r#""zzz":{},"#, items(1_000_000, "null"))),
                Some(("invalid_json", "/")),
            ),
            (
                "an array root claiming absent spans",
                array_root(true),
                Some(("invalid_capabilities", "/capabilities")),
            ),
            ("an array root claiming nothing", array_root(false), None),
            (
                "an unknown key, then a float",
                edit(r#""element":"e1""#, r#""zzz":1,"element":"e1""#).replacen(
                    r#""width":30000"#,
                    r#""width":0.5"#,
                    1,
                ),
                Some(("invalid_json", "/")),
            ),
        ];
        for (label, input, expected) in cases {
            let r = grounding_check(input.as_bytes(), None).unwrap();
            let got = r.error.as_ref().map(|e| (e.code.as_str(), e.path.as_str()));
            assert_eq!(got, expected, "{label}");
            assert_eq!(
                r.structure == Structure::Valid,
                expected.is_none(),
                "{label}"
            );
        }
    }

    #[test]
    fn a_value_the_typed_parse_accepts_is_still_refused_before_typing() {
        // Each of these types cleanly, so only the scan stands between it and the route that skips
        // the seed.
        type Case = (
            &'static str,
            &'static str,
            Box<dyn Fn(&mut serde_json::Value)>,
        );
        let cases: Vec<Case> = vec![
            (
                "null for an absent array",
                "invalid_json",
                Box::new(|v| v["spans"] = serde_json::Value::Null),
            ),
            (
                "null for an absent field",
                "invalid_json",
                Box::new(|v| v["elements"][0]["page"] = serde_json::Value::Null),
            ),
            (
                "an integer past 2^53-1",
                "limit_exceeded",
                Box::new(|v| v["pages"][0]["width"] = 9_007_199_254_740_992_i64.into()),
            ),
            (
                "a negative integer past -(2^53-1)",
                "limit_exceeded",
                Box::new(|v| v["elements"][0]["bbox"][0] = (-9_007_199_254_740_992_i64).into()),
            ),
            (
                "a string past the byte limit",
                "limit_exceeded",
                Box::new(|v| v["producer"]["version"] = "é".repeat(8193).into()),
            ),
        ];
        for (label, code, mutate) in cases {
            let mut v: serde_json::Value = serde_json::from_slice(&valid_bytes()).unwrap();
            mutate(&mut v);
            let bytes = serde_json::to_vec(&v).unwrap();
            assert!(
                serde_json::from_slice::<GroundingSource>(&bytes).is_ok(),
                "{label}: the typed parse must accept it, or this case tests nothing"
            );
            let e = grounding_check(&bytes, None).unwrap().error.unwrap();
            assert_eq!((e.code.as_str(), e.path.as_str()), (code, "/"), "{label}");
        }
    }

    #[test]
    fn every_key_the_typed_structs_write_is_one_the_walk_allows() {
        // A typed parse that succeeds is taken as `reject_unknown_fields` finding nothing. That holds
        // only while no wire type carries a key the walk does not list, so every field is written here.
        let mut g: GroundingSource = serde_json::from_slice(&valid_bytes()).unwrap();
        g.elements = vec![Element {
            id: "e1".into(),
            page: Some("p1".into()),
            bbox: Some([10, 10, 100, 100]),
            kind: "text_run".into(),
            text: Some("hello".into()),
            locator: Some("l".into()),
        }];
        g.spans = Some(vec![Span {
            id: "s1".into(),
            page: "p1".into(),
            bbox: [10, 10, 100, 100],
            text: "hello".into(),
            element: Some("e1".into()),
            char_start: Some(0),
            char_end: Some(5),
        }]);
        g.tables = Some(vec![Table {
            id: "t1".into(),
            page: "p1".into(),
            bbox: [10, 10, 100, 100],
            cells: vec![crate::Cell {
                row: 0,
                col: 0,
                row_span: 1,
                col_span: 1,
                bbox: [10, 10, 100, 100],
                text: "hello".into(),
            }],
        }]);
        assert_eq!(
            reject_unknown_fields(&serde_json::to_value(&g).unwrap()),
            Ok(())
        );
    }

    #[test]
    fn an_invalid_artifact_reports_not_checked_even_with_a_source() {
        // There is no `source.sha256` to compare against, so the question was never asked.
        let mut v: serde_json::Value = serde_json::from_slice(&valid_bytes()).unwrap();
        v["artifact_type"] = "nope".into();
        let bytes = serde_json::to_vec(&v).unwrap();
        let r = grounding_check(&bytes, Some(b"%PDF-1.7 fake")).unwrap();
        assert_eq!(r.source_binding, SourceBinding::NotChecked);
    }

    #[test]
    fn the_report_round_trips_and_carries_no_verification_concept() {
        let r = grounding_check(&valid_bytes(), None).unwrap();
        let bytes = r.to_canonical_bytes().unwrap();
        let back: ValidationReport = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, r);

        let s = String::from_utf8(bytes).unwrap().to_lowercase();
        for banned in [
            "grounded",
            "evidence_tier",
            "verdict",
            "claim",
            "confidence",
        ] {
            assert!(
                !s.contains(banned),
                "`{banned}` reached a validation report"
            );
        }
    }
}
