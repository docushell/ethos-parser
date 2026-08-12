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

//! The error taxonomy (`docs/01-CONTRACT.md` §8, `docs/03-V0-SCOPE.md` §1 item 13).
//!
//! Six variants, borrowed in shape from Anydoc (checklist A5). The point is that a caller can
//! *route* on the variant: "this document is encrypted" and "this document is complex" must
//! never arrive as the same signal, which is the defect LiteParse's single exit code has
//! (checklist L16).
//!
//! Messages are stable enough to assert on. A fail-closed error nobody can match against is
//! only marginally better than a silent failure.

use serde::{Deserialize, Serialize};

/// Everything that can go wrong, in six routable shapes.
///
/// `non_exhaustive` so adding a variant later is not a breaking change for matchers that
/// already handle a `_` arm — but note that *producing* a new variant without a caller-visible
/// reason code is the thing to avoid, not the enum growing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum EngineError {
    /// The input is a format, version, or feature this profile does not claim to handle.
    ///
    /// Distinct from [`Self::Malformed`]: the document may be perfectly valid, and the gap is
    /// ours. This is the variant that maps to a capability-limited result rather than a
    /// negative conclusion.
    Unsupported {
        /// What was not supported, e.g. `"pdf operator"`, `"media type"`.
        what: String,
        /// The specific unsupported value.
        detail: String,
    },

    /// The input violates its own format specification.
    Malformed {
        /// The structure that failed to parse.
        what: String,
        /// What was wrong with it.
        detail: String,
    },

    /// The source is encrypted or password-protected.
    ///
    /// Its own variant rather than a `Malformed` sub-case because it is the canonical example
    /// of an outcome a caller routes differently: a human can supply a password, and no amount
    /// of retrying will fix it otherwise.
    Encrypted {
        /// What kind of protection was detected, where known.
        detail: String,
    },

    /// A declared resource bound was hit — page count, memory, time, nesting depth.
    ///
    /// Fail-closed by design. The partial work is discarded rather than emitted, because a
    /// truncated representation that looks complete is worse than no representation.
    ResourceLimit {
        /// The limit that was hit, e.g. `"max nesting depth"`.
        limit: String,
        /// The configured value.
        configured: String,
    },

    /// A structure the format requires was absent.
    MissingPart {
        /// The part that should have been there, e.g. `"xref table"`, `"font descriptor"`.
        part: String,
    },

    /// An I/O failure reading the source.
    ///
    /// Carries a string rather than `std::io::Error` so the type stays `Clone`, `PartialEq`,
    /// and serializable — an error that cannot appear in a diagnostic artifact is not much use
    /// to a fail-closed design.
    Io {
        /// The underlying message.
        detail: String,
    },
}

impl EngineError {
    /// A short, stable machine-routable code.
    ///
    /// Stable across releases: callers write policy against these, so renaming one is a
    /// breaking change to be treated like a schema change.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "unsupported",
            Self::Malformed { .. } => "malformed",
            Self::Encrypted { .. } => "encrypted",
            Self::ResourceLimit { .. } => "resource_limit",
            Self::MissingPart { .. } => "missing_part",
            Self::Io { .. } => "io",
        }
    }
}

impl core::fmt::Display for EngineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unsupported { what, detail } => write!(f, "unsupported {what}: {detail}"),
            Self::Malformed { what, detail } => write!(f, "malformed {what}: {detail}"),
            Self::Encrypted { detail } => write!(f, "encrypted source: {detail}"),
            Self::ResourceLimit { limit, configured } => {
                write!(
                    f,
                    "resource limit exceeded: {limit} (configured {configured})"
                )
            }
            Self::MissingPart { part } => write!(f, "missing required part: {part}"),
            Self::Io { detail } => write!(f, "io error: {detail}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<std::io::Error> for EngineError {
    fn from(e: std::io::Error) -> Self {
        Self::Io {
            detail: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c14n::c14n_bytes;

    fn all_variants() -> Vec<EngineError> {
        vec![
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
        ]
    }

    #[test]
    fn every_variant_has_a_distinct_code() {
        let codes: Vec<&str> = all_variants().iter().map(EngineError::code).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            codes.len(),
            "codes must be distinct: {codes:?}"
        );
        assert_eq!(codes.len(), 6, "the taxonomy is six variants");
    }

    #[test]
    fn messages_are_stable_enough_to_assert_on() {
        // These strings are a contract with tests and with callers grepping logs. Changing one
        // is a deliberate act, which is exactly why they are pinned here.
        assert_eq!(
            all_variants()[0].to_string(),
            "unsupported pdf operator: \""
        );
        assert_eq!(
            all_variants()[1].to_string(),
            "malformed xref entry: 19 bytes, expected 20"
        );
        assert_eq!(
            all_variants()[2].to_string(),
            "encrypted source: standard security handler"
        );
        assert_eq!(
            all_variants()[3].to_string(),
            "resource limit exceeded: max nesting depth (configured 64)"
        );
        assert_eq!(
            all_variants()[4].to_string(),
            "missing required part: xref table"
        );
        assert_eq!(all_variants()[5].to_string(), "io error: no such file");
    }

    #[test]
    fn encrypted_is_not_conflated_with_malformed() {
        // The LiteParse defect: password-protected and corrupt-header producing one signal.
        let enc = EngineError::Encrypted { detail: "x".into() };
        let mal = EngineError::Malformed {
            what: "header".into(),
            detail: "x".into(),
        };
        assert_ne!(enc.code(), mal.code());
    }

    #[test]
    fn errors_survive_canonicalization() {
        for e in all_variants() {
            let v = serde_json::to_value(&e).unwrap();
            let bytes = c14n_bytes(&v).unwrap();
            let back: EngineError = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(back, e);
        }
    }

    #[test]
    fn io_errors_convert_without_losing_the_message() {
        let e: EngineError = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
        assert_eq!(e.code(), "io");
        assert!(e.to_string().contains("gone"));
    }
}
