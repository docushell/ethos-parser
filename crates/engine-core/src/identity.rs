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

//! Artifact identity and the coordinate declaration (`docs/01-CONTRACT.md` §2, §3).

use serde::{Deserialize, Serialize};

use crate::error::EngineError;

/// A `sha256:<64 lowercase hex>` digest string.
///
/// A newtype rather than a `String` because these are compared, not read: a digest that is
/// uppercase, bare-hex, or truncated compares unequal to the same digest correctly formatted,
/// and the resulting "fingerprint mismatch" would be a formatting bug wearing an integrity
/// bug's clothes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Hex(String);

impl Sha256Hex {
    /// Wrap an already-formatted `sha256:<hex>` string.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] unless the value is exactly `sha256:` followed by 64
    /// lowercase hex digits. Uppercase is rejected rather than folded: silently accepting both
    /// cases would make two spellings of one digest compare unequal elsewhere.
    pub fn parse(s: impl Into<String>) -> Result<Self, EngineError> {
        let s = s.into();
        let Some(hex) = s.strip_prefix("sha256:") else {
            return Err(EngineError::Malformed {
                what: "digest".into(),
                detail: "missing `sha256:` prefix".into(),
            });
        };
        if hex.len() != 64 {
            return Err(EngineError::Malformed {
                what: "digest".into(),
                detail: format!("expected 64 hex digits, found {}", hex.len()),
            });
        }
        if !hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(EngineError::Malformed {
                what: "digest".into(),
                detail: "expected lowercase hex digits".into(),
            });
        }
        Ok(Self(s))
    }

    /// Build from a bare lowercase hex digest, adding the `sha256:` prefix.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if `hex` is not 64 lowercase hex digits.
    pub fn from_hex(hex: &str) -> Result<Self, EngineError> {
        Self::parse(format!("sha256:{hex}"))
    }

    /// The digest of some bytes. **Infallible by construction** (v1-S6).
    ///
    /// # Why this exists rather than `parse(sha256_hex_bytes(…))`
    ///
    /// Because that spelling is wrong in a way that compiles and then fails quietly.
    /// [`crate::c14n::sha256_hex_bytes`] returns *bare* hex and [`Self::parse`] requires the
    /// `sha256:` prefix, so the pair returns `Err` for every input — and a caller writing
    /// `.ok()?` gets `None`, which at a `let … else { continue }` is an element silently missing
    /// from an artifact. That is exactly what happened while v1-S6 was being written: every image
    /// node vanished, no error was raised anywhere, and the only symptom was an empty array that
    /// looked like an honest "found none".
    ///
    /// Hashing bytes cannot fail and the output is always 64 lowercase hex digits, so the
    /// fallible spelling was never describing a real possibility. Removing the failure mode beats
    /// handling it.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(format!("sha256:{}", crate::c14n::sha256_hex_bytes(bytes)))
    }

    /// The full `sha256:<hex>` form, as it appears on the wire.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The bare 64-character hex digest, without the prefix.
    pub fn hex(&self) -> &str {
        &self.0["sha256:".len()..]
    }
}

impl core::fmt::Display for Sha256Hex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Sha256Hex {
    type Error = EngineError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(s)
    }
}

impl From<Sha256Hex> for String {
    fn from(v: Sha256Hex) -> Self {
        v.0
    }
}

/// The unit in which canonical geometry is expressed.
///
/// One variant today. It is still declared on every artifact rather than implied, because a
/// const today is a discriminator tomorrow and a reader must never have to infer it
/// (`docs/01-CONTRACT.md` §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoordinateUnit {
    /// 1/100 of a PostScript point.
    Centipoint,
}

/// Where the origin sits and which way the axes run.
///
/// PDF's native origin is bottom-left; the artifact's is top-left. The transform is the
/// engine's job and this declaration is how a reader knows it happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoordinateOrigin {
    /// Origin at the top-left corner; x grows right, y grows down.
    TopLeft,
}

/// The coordinate declaration carried by every artifact bearing geometry.
///
/// Matches `ethos.grounding.v1`'s `coordinate_system` object exactly, so the M5 projection is a
/// move rather than a translation.
///
/// `deny_unknown_fields` matters here more than anywhere else in this module, because this type
/// is **nested inside [`crate::Profile`]** and the attribute is *not* recursive. Without it, a
/// profile carrying `coordinate_system.some_future_knob` deserialized with that knob dropped and
/// then re-hashed to the **unmodified default digest** — measured, not theorised — so an
/// artifact would claim comparability with a profile it does not actually match. Guarding the
/// outer struct alone left this exact hole one level down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordinateSystem {
    /// The measurement unit.
    pub unit: CoordinateUnit,
    /// The origin corner and axis directions.
    pub origin: CoordinateOrigin,
}

impl CoordinateSystem {
    /// The only system v0 emits: centipoints from the top-left.
    pub const V0: Self = Self {
        unit: CoordinateUnit::Centipoint,
        origin: CoordinateOrigin::TopLeft,
    };
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        Self::V0
    }
}

/// The four identity fields every artifact carries, before any payload
/// (`docs/01-CONTRACT.md` §2).
///
/// Fails closed on unknown fields, matching `artifact-identity.draft.json`'s
/// `additionalProperties: false`. §8 says a reader that does not recognise a shape refuses it
/// rather than best-effort parsing; an envelope silently shedding a field it did not expect is
/// that same best-effort parse wearing a struct definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactIdentity {
    /// Names the shape exactly, e.g. `ethos.grounding.v1`. A reader that does not recognise it
    /// fails closed rather than best-effort parsing.
    pub artifact_type: String,
    /// Semantic version of the artifact *shape*, independent of the parser build.
    pub schema_version: String,
    /// The engine build that produced this artifact. Distinct from `schema_version`: many
    /// builds can emit one shape.
    pub parser_version: String,
    /// Hash of the pinned configuration profile. Two artifacts are comparable if and only if
    /// this matches.
    pub profile_sha256: Sha256Hex,
}

/// Source identity and representation identity, which are **not** the same thing.
///
/// The DocuShell companion is explicit that both must be retained: source identity is the hash
/// of the original bytes ("what was read"), representation identity is the hash of the
/// canonical evidence produced under a pinned profile ("what was produced"). Collapsing them
/// makes it impossible to tell a re-parse from a different document.
///
/// Fails closed on unknown fields, for the same reason as [`ArtifactIdentity`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactBinding {
    /// Digest of the exact original source bytes.
    pub source_sha256: Sha256Hex,
    /// Digest of the canonical representation produced from them.
    pub representation_sha256: Sha256Hex,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c14n::{c14n_bytes, sha256_hex_bytes};

    const VALID: &str = "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";

    #[test]
    fn digest_accepts_the_canonical_form() {
        let d = Sha256Hex::parse(VALID).unwrap();
        assert_eq!(d.as_str(), VALID);
        assert_eq!(d.hex().len(), 64);
        assert!(!d.hex().starts_with("sha256"));
    }

    #[test]
    fn digest_rejects_everything_else() {
        assert!(Sha256Hex::parse("44136fa3").is_err(), "bare hex");
        assert!(Sha256Hex::parse("sha256:").is_err(), "empty");
        assert!(Sha256Hex::parse("sha256:abc").is_err(), "too short");
        assert!(
            Sha256Hex::parse(format!("sha256:{}", "a".repeat(65))).is_err(),
            "too long"
        );
        assert!(
            Sha256Hex::parse(format!("sha256:{}", "A".repeat(64))).is_err(),
            "uppercase must be rejected, not folded"
        );
        assert!(
            Sha256Hex::parse(format!("sha256:{}", "g".repeat(64))).is_err(),
            "non-hex"
        );
        assert!(Sha256Hex::parse(format!("md5:{}", "a".repeat(64))).is_err());
    }

    #[test]
    fn digest_from_hex_adds_the_prefix() {
        let raw = sha256_hex_bytes(b"hello");
        let d = Sha256Hex::from_hex(&raw).unwrap();
        assert_eq!(d.hex(), raw);
        assert!(d.as_str().starts_with("sha256:"));
    }

    #[test]
    fn digest_round_trips_as_a_plain_string_on_the_wire() {
        let d = Sha256Hex::parse(VALID).unwrap();
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(json, format!("\"{VALID}\""));
        assert_eq!(serde_json::from_str::<Sha256Hex>(&json).unwrap(), d);
    }

    #[test]
    fn a_malformed_digest_cannot_be_deserialized() {
        assert!(serde_json::from_str::<Sha256Hex>("\"sha256:zz\"").is_err());
    }

    #[test]
    fn coordinate_system_serializes_as_the_grounding_shape() {
        let v = serde_json::to_value(CoordinateSystem::V0).unwrap();
        assert_eq!(
            String::from_utf8(c14n_bytes(&v).unwrap()).unwrap(),
            r#"{"origin":"top-left","unit":"centipoint"}"#
        );
    }

    #[test]
    fn envelope_types_refuse_unknown_fields() {
        let identity = serde_json::json!({
            "artifact_type": "ethos.engine.representation.v0",
            "schema_version": "0.1.0",
            "parser_version": "0.0.0",
            "profile_sha256": VALID,
            "future_knob": 1
        });
        assert!(
            serde_json::from_value::<ArtifactIdentity>(identity).is_err(),
            "ArtifactIdentity must fail closed on an unknown field"
        );

        let binding = serde_json::json!({
            "source_sha256": VALID,
            "representation_sha256": VALID,
            "future_knob": 1
        });
        assert!(
            serde_json::from_value::<ArtifactBinding>(binding).is_err(),
            "ArtifactBinding must fail closed on an unknown field"
        );

        let coords = serde_json::json!({
            "unit": "centipoint",
            "origin": "top-left",
            "future_knob": 1
        });
        assert!(
            serde_json::from_value::<CoordinateSystem>(coords).is_err(),
            "CoordinateSystem must fail closed — it is nested inside Profile, where a dropped \
             field silently reproduces the default digest"
        );
    }

    #[test]
    fn envelope_types_still_accept_their_exact_shape() {
        // Guard the guard: `deny_unknown_fields` must not have broken the happy path.
        let identity = serde_json::json!({
            "artifact_type": "ethos.engine.representation.v0",
            "schema_version": "0.1.0",
            "parser_version": "0.0.0",
            "profile_sha256": VALID
        });
        assert!(serde_json::from_value::<ArtifactIdentity>(identity).is_ok());
        assert!(serde_json::from_value::<CoordinateSystem>(
            serde_json::json!({"unit": "centipoint", "origin": "top-left"})
        )
        .is_ok());
    }

    #[test]
    fn artifact_identity_round_trips_through_c14n() {
        let id = ArtifactIdentity {
            artifact_type: "ethos.engine.representation.v0".into(),
            schema_version: "0.1.0".into(),
            parser_version: "0.0.0".into(),
            profile_sha256: Sha256Hex::parse(VALID).unwrap(),
        };
        let v = serde_json::to_value(&id).unwrap();
        let bytes = c14n_bytes(&v).unwrap();
        let back: ArtifactIdentity = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, id);
    }
}
