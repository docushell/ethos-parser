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

//! Volatile observations, quarantined (`docs/01-CONTRACT.md` §4).
//!
//! # The rule
//!
//! Timings, memory, host and source paths are **the only things** that may vary between two runs
//! over the same bytes. The contract's answer is not "keep them out of the artifact by
//! convention" — it is to give them a separate object that is **off by default**, never
//! fingerprinted, and never written to stdout.
//!
//! ```text
//! default             stdout = artifact                     (byte-identical across runs)
//! --diagnostics       stdout = artifact, unchanged
//!                     stderr = one JSON object, this type   (varies, and is meant to)
//! ```
//!
//! # Why this type is deliberately not an artifact
//!
//! It carries no `artifact_type`, no `schema_version`, no `profile_sha256`, and it is **not**
//! canonicalized — [`Diagnostics::to_json_line`] uses plain `serde_json`, not [`crate::c14n`].
//! Every one of those absences is load-bearing. An object that looked like an artifact would
//! eventually be consumed like one, and then a timing would be inside somebody's hash.
//!
//! # Why nothing here can reach a fingerprint
//!
//! Structurally, not by review: no artifact type has a field of this type, and nothing in this
//! module is reachable from `to_canonical_bytes` on any of them. The stage functions do not take
//! a [`Diagnostics`] and cannot observe one. `no_diagnostics_field_name_appears_in_any_artifact`
//! in `tests/contract_invariants.rs` asserts the field *names* are absent too, the way Ethos
//! property-tests its own payload projection — a field that arrived under a different type but
//! the same name would still be the same mistake.
//!
//! # What is not measured at v0
//!
//! Resident memory on non-Linux hosts. [`Diagnostics::resident_bytes`] is `None` there, and that
//! is a typed absence rather than a zero: reading it portably needs either a dependency or
//! platform code this crate does not carry, and a `0` would be a measurement nobody took.

use serde::{Deserialize, Serialize};

use crate::EngineError;

/// The version of this envelope's shape.
///
/// Named `diagnostics_version`, **not** `schema_version`: this object is not an artifact and
/// must not be mistaken for one by a consumer that keys off familiar field names.
pub const DIAGNOSTICS_VERSION: &str = "0";

/// Which subcommand produced an observation.
///
/// One variant per v0 subcommand, so a caller collecting diagnostics from a pipeline can tell
/// the four apart without parsing a command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stage {
    /// `engine classify`
    Classify,
    /// `engine extract`
    Extract,
    /// `engine ground`
    Ground,
    /// `engine grounding-check`
    GroundingCheck,
}

impl Stage {
    /// The stage's wire name, matching the subcommand.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Classify => "classify",
            Self::Extract => "extract",
            Self::Ground => "ground",
            Self::GroundingCheck => "grounding-check",
        }
    }
}

/// The host a run happened on.
///
/// Deliberately coarse. `std::env::consts` is compile-time, so this records the *target* the
/// binary was built for, which is what a "did these two artifacts come from comparable builds"
/// question actually wants. A hostname would be personal data in a file people paste into bug
/// reports, and it answers nothing this does not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostInfo {
    /// Target operating system (`std::env::consts::OS`).
    pub os: String,
    /// Target architecture (`std::env::consts::ARCH`).
    pub arch: String,
}

impl HostInfo {
    /// The host this binary was built for.
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        }
    }
}

/// A stage's volatile observations.
///
/// Obtained by [`DiagnosticsRun::finish`]; there is no public constructor, so the fields cannot
/// be assembled from something that is not a real measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostics {
    /// Envelope version. See [`DIAGNOSTICS_VERSION`].
    pub diagnostics_version: String,
    /// The subcommand this describes.
    pub stage: Stage,
    /// The engine build that ran, matching `--version`.
    pub engine_version: String,
    /// Wall-clock microseconds for the measured region.
    ///
    /// Microseconds, not nanoseconds: nanosecond precision on a wall clock is noise dressed as
    /// data, and it invites someone to diff two runs and believe the difference.
    pub wall_micros: u64,
    /// The input path as given on the command line, or `None` for a library caller working from
    /// memory.
    pub input_path: Option<String>,
    /// Input size in bytes, where the caller read one.
    pub input_bytes: Option<u64>,
    /// The build target.
    pub host: HostInfo,
    /// Resident set size in bytes, where the platform reports one cheaply.
    ///
    /// `Some` on Linux (`/proc/self/statm`), `None` everywhere else. See the module header:
    /// absence here means "not measured", never "zero".
    pub resident_bytes: Option<u64>,
}

impl Diagnostics {
    /// Serialize as one JSON object on a single line, newline-terminated.
    ///
    /// Plain `serde_json`, **not** c14n. Canonicalizing this would give a volatile object the one
    /// property that marks a canonical artifact, which is exactly the confusion this type exists
    /// to prevent.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if serialization fails, which for this shape means a `serde_json`
    /// bug rather than bad data.
    pub fn to_json_line(&self) -> Result<Vec<u8>, EngineError> {
        let mut bytes = serde_json::to_vec(self).map_err(|e| EngineError::Malformed {
            what: "diagnostics".into(),
            detail: e.to_string(),
        })?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

/// A started clock.
///
/// The timing region is a value with a `finish`, rather than two free calls, so a stage cannot
/// report a duration it did not measure.
#[derive(Debug)]
pub struct DiagnosticsRun {
    stage: Stage,
    started: std::time::Instant,
}

impl DiagnosticsRun {
    /// Start measuring a stage.
    pub fn begin(stage: Stage) -> Self {
        Self {
            stage,
            started: std::time::Instant::now(),
        }
    }

    /// Stop measuring and collect.
    ///
    /// Consumes the run: a region is measured once, and a second reading of the same clock would
    /// describe a different region than the one the caller thinks it timed.
    pub fn finish(self, input_path: Option<String>, input_bytes: Option<u64>) -> Diagnostics {
        Diagnostics {
            diagnostics_version: DIAGNOSTICS_VERSION.to_string(),
            stage: self.stage,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            wall_micros: u64::try_from(self.started.elapsed().as_micros()).unwrap_or(u64::MAX),
            input_path,
            input_bytes,
            host: HostInfo::current(),
            resident_bytes: resident_bytes(),
        }
    }
}

/// Resident set size, on the one platform where it is a file read.
///
/// `/proc/self/statm`'s second field is resident pages. The page size is not read from `sysconf`
/// — that would be a libc call in a crate that forbids unsafe — so 4096 is assumed, which is
/// right on every Linux target this engine builds for. A diagnostic that is approximately right
/// is worth having; one that requires unsafe to be exactly right is not.
#[cfg(target_os = "linux")]
fn resident_bytes() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    pages.checked_mul(4096)
}

/// Not measured off Linux. See the module header: `None` is "not measured", never zero.
#[cfg(not(target_os = "linux"))]
fn resident_bytes() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_reports_the_stage_it_measured() {
        let d = DiagnosticsRun::begin(Stage::Extract).finish(Some("/tmp/x.pdf".into()), Some(12));
        assert_eq!(d.stage, Stage::Extract);
        assert_eq!(d.input_path.as_deref(), Some("/tmp/x.pdf"));
        assert_eq!(d.input_bytes, Some(12));
        assert_eq!(d.engine_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(d.diagnostics_version, DIAGNOSTICS_VERSION);
    }

    #[test]
    fn the_json_line_is_one_line_and_parses() {
        let d = DiagnosticsRun::begin(Stage::Classify).finish(None, None);
        let bytes = d.to_json_line().unwrap();
        assert_eq!(bytes.iter().filter(|b| **b == b'\n').count(), 1);
        assert!(bytes.ends_with(b"\n"));

        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["stage"], "classify");
        assert_eq!(v["diagnostics_version"], DIAGNOSTICS_VERSION);
        assert!(v["wall_micros"].is_u64());
        assert!(v["input_path"].is_null(), "a memory caller has no path");
    }

    #[test]
    fn the_stage_names_are_the_subcommand_names() {
        // A diagnostics line that named stages differently from the CLI would be one more thing
        // to translate in a bug report.
        for (stage, name) in [
            (Stage::Classify, "classify"),
            (Stage::Extract, "extract"),
            (Stage::Ground, "ground"),
            (Stage::GroundingCheck, "grounding-check"),
        ] {
            assert_eq!(stage.as_str(), name);
            assert_eq!(serde_json::to_value(stage).unwrap(), name);
        }
    }

    #[test]
    fn the_envelope_carries_no_artifact_identity() {
        // If this ever grows `artifact_type` / `schema_version` / `profile_sha256`, something is
        // treating diagnostics as a record. It is not one.
        let d = DiagnosticsRun::begin(Stage::Ground).finish(None, None);
        let v = serde_json::to_value(&d).unwrap();
        let obj = v.as_object().unwrap();
        for banned in ["artifact_type", "schema_version", "profile_sha256"] {
            assert!(
                !obj.contains_key(banned),
                "diagnostics grew `{banned}`; it is not an artifact and must not look like one"
            );
        }
    }

    #[test]
    fn two_runs_over_the_same_input_may_differ_and_that_is_the_point() {
        // The inverse of the artifact contract, asserted so nobody "fixes" the volatility.
        let a = DiagnosticsRun::begin(Stage::Classify).finish(None, None);
        let b = DiagnosticsRun::begin(Stage::Classify).finish(None, None);
        assert_eq!(a.stage, b.stage);
        // Durations may coincide at this resolution; the type is what matters, not the value.
        assert!(a.wall_micros < u64::MAX && b.wall_micros < u64::MAX);
    }
}
