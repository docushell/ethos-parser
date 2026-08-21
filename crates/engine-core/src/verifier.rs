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

//! Invoking a verifier. **Not verifying** (`docs/07-VERIFY-BOUNDARY.md` Stage 1).
//!
//! # The distinction this module exists to hold
//!
//! The engine spawns the Ethos CLI, hands it a grounding artifact and a citations file, and
//! **forwards the bytes it prints**. It does not read them. There is no type here for a report,
//! a claim, a check, an evidence tier or a result — not because they would be hard to write, but
//! because writing them is how a second authority is born by accident. Two engines that disagree
//! about whether a document supports a claim is the failure this whole project is arranged to
//! prevent, and the cheapest way to guarantee agreement is to have only one opinion in existence.
//!
//! So the contract is narrow enough to state in full:
//!
//! | The engine does | The engine never does |
//! | --- | --- |
//! | Resolve a verifier binary and pin its identity | Parse the report |
//! | Spawn it with the arguments a caller asked for | Re-compute any field of it |
//! | Relay stdout verbatim, byte for byte | Summarize, filter, or annotate it |
//! | Map the child's exit status onto its own | Invent a report when the child produced none |
//!
//! `ci/forbidden-tokens.sh verification` is the enforcement: the identifiers a verifier owns are
//! forbidden in `crates/*/src`, and this module is inside that scan.
//!
//! # Why it lives in `engine-core`
//!
//! Because it must be reachable as a **library** call — `docs/05-MILESTONES.md`'s thin-shell rule
//! applies to `engine verify` exactly as it applies to the other eight subcommands, and
//! `engine-cli` exports nothing. `engine-core` is the only crate every other one depends on, and
//! this module keeps the crate's rules: it holds no PDF concept and no verification concept. It
//! knows how to run a program and how to pin what it ran.
//!
//! # Absence is a named error, never a skip
//!
//! Same rule the M0 oracle harness has run under since the first commit. A missing verifier is a
//! loud, deterministic failure with exit 2 and **no report on stdout** — never a stub, never a
//! default-pass, never a quiet success. An engine that "verified" because it could not find a
//! verifier would be the worst possible outcome.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::identity::Sha256Hex;
use crate::profile::VerifierPin;
use crate::EngineError;

/// Exit code for a run the verifier completed and was content with.
pub const RELAY_OK: i32 = 0;

/// Exit code for a run the verifier completed and was **not** content with.
///
/// Only reachable when the caller asked for `--fail-on-ungrounded`: without it the verifier
/// writes its report and exits 0 whatever it found, and the engine forwards that unchanged.
pub const RELAY_REFUSED: i32 = 1;

/// Exit code for "the run did not happen, or did not finish".
///
/// A missing binary, a spawn failure, or any child status the engine does not have a meaning
/// for. Deliberately the same code the other subcommands use for could-not-read: a caller
/// distinguishing "the check failed" from "the check did not run" is the whole point of keeping
/// 1 and 2 apart (`docs/03-V0-SCOPE.md` §3.1).
pub const RELAY_UNAVAILABLE: i32 = 2;

/// A resolved verifier binary, with the identity that pins it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierBinary {
    path: PathBuf,
    version: String,
    sha256: Sha256Hex,
}

impl VerifierBinary {
    /// Locate a verifier and read its identity.
    ///
    /// Resolution order, matching the oracle harness exactly so one binary answers for both:
    ///
    /// 1. `explicit` (the `ETHOS_BIN` environment variable, when the caller passes it through).
    ///    **Authoritative, not a hint** — if it is set and names no file, that is a hard error
    ///    rather than a fallback. An operator who pinned a verifier and silently got a different
    ///    one is in the worst position available: they believe they know which one answered.
    /// 2. `../ethos/target/release/ethos` relative to `repo_relative`, when one is given.
    /// 3. `ethos` on `PATH`.
    ///
    /// # Errors
    ///
    /// [`EngineError::MissingPart`] when no verifier can be located, listing what was tried.
    /// [`EngineError::Io`] when a located binary will not report its version.
    pub fn resolve(
        explicit: Option<&Path>,
        repo_relative: Option<&Path>,
    ) -> Result<Self, EngineError> {
        let mut tried: Vec<String> = Vec::new();

        if let Some(p) = explicit {
            if p.is_file() {
                return Self::identify(p);
            }
            return Err(EngineError::MissingPart {
                part: format!(
                    "verifier binary at `{}`, which was named explicitly and is not a file. \
                     An explicit pin is authoritative: resolving to some other binary would mean \
                     relaying a report from a verifier nobody chose.",
                    p.display()
                ),
            });
        }
        tried.push("  explicit pin (unset)".to_string());

        if let Some(root) = repo_relative {
            let sibling = root.join("../ethos/target/release/ethos");
            if sibling.is_file() {
                return Self::identify(&sibling);
            }
            tried.push(format!("  {} (absent)", sibling.display()));
        }

        if let Ok(out) = Command::new("sh")
            .arg("-c")
            .arg("command -v ethos")
            .output()
        {
            let found = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !found.is_empty() {
                return Self::identify(Path::new(&found));
            }
        }
        tried.push("  `command -v ethos` (not found)".to_string());

        Err(EngineError::MissingPart {
            part: format!(
                "a verifier binary. Tried, in order:\n{}\n\n\
                 The engine does not verify — it invokes a verifier — so without one there is \
                 nothing to relay. This is a named failure and never a skip: a run that reported \
                 success because it could not find a verifier would be the worst outcome \
                 available (docs/07-VERIFY-BOUNDARY.md).",
                tried.join("\n")
            ),
        })
    }

    /// Read a located binary's version and digest.
    fn identify(path: &Path) -> Result<Self, EngineError> {
        let bytes = std::fs::read(path).map_err(|e| EngineError::Io {
            detail: format!("cannot read the verifier at {}: {e}", path.display()),
        })?;
        let sha256 = Sha256Hex::from_hex(&crate::c14n::sha256_hex_bytes(&bytes))
            .expect("sha256 hex is always well formed");

        let out = Command::new(path)
            .arg("--version")
            .output()
            .map_err(|e| EngineError::Io {
                detail: format!("the verifier at {} would not run: {e}", path.display()),
            })?;
        if !out.status.success() {
            return Err(EngineError::Io {
                detail: format!(
                    "the verifier at {} exited {:?} for `--version`; it cannot be pinned, so it \
                     will not be used",
                    path.display(),
                    out.status.code()
                ),
            });
        }

        Ok(Self {
            path: path.to_path_buf(),
            version: String::from_utf8_lossy(&out.stdout).trim().to_string(),
            sha256,
        })
    }

    /// Where it is.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// What it reports for `--version`.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Digest of its bytes.
    pub fn sha256(&self) -> &Sha256Hex {
        &self.sha256
    }

    /// The profile pin for a run that consulted this binary.
    ///
    /// Version **and** digest: two builds of the same version can differ, and a pin that could
    /// not tell them apart would not make a swap fingerprint-visible.
    pub fn pin(&self) -> VerifierPin {
        VerifierPin::pinned(self.version.clone(), self.sha256.clone())
    }
}

/// What to ask the verifier for.
///
/// The minimal flag set (`docs/02-ROADMAP.md` v0.1). Every field maps to a flag the pinned
/// verifier already has; nothing here is invented, and nothing is translated.
#[derive(Debug, Clone)]
pub struct RelayRequest<'a> {
    /// The grounding artifact, as `engine ground` emits it.
    pub grounding: &'a Path,
    /// The citations file.
    pub citations: &'a Path,
    /// The grounding adapter id to declare.
    ///
    /// Always sent. `ethos verify` can sniff the artifact type, but declaring it is what makes a
    /// wrong input a **usage error** instead of a silent fall back to native-document loading —
    /// measured against the pinned binary, which refuses a mismatched adapter with exit 2.
    pub adapter: &'a str,
    /// Ask the verifier to exit non-zero when it is not satisfied.
    pub fail_on_ungrounded: bool,
    /// Optional verification config.
    pub config: Option<&'a Path>,
    /// Optional output path. When set, the verifier writes there instead of to stdout.
    pub out: Option<&'a Path>,
}

/// The adapter id for artifacts `engine ground` produces.
///
/// Measured against the pinned binary rather than remembered: it accepts `ethos-json`,
/// `ethos-grounding-json` and `opendataloader-json`, and only the middle one loads an
/// `ethos.grounding.v1` artifact.
pub const GROUNDING_ADAPTER: &str = "ethos-grounding-json";

/// What the verifier said, forwarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relayed {
    /// The verifier's stdout, **verbatim**. Not parsed, not re-encoded, not validated.
    pub stdout: Vec<u8>,
    /// The verifier's stderr, verbatim.
    pub stderr: Vec<u8>,
    /// The exit code the engine should use. See [`RELAY_OK`], [`RELAY_REFUSED`],
    /// [`RELAY_UNAVAILABLE`].
    pub exit: i32,
}

/// Spawn the verifier and relay what it says.
///
/// # Errors
///
/// [`EngineError::Io`] if the process cannot be spawned at all. A child that *ran* and failed is
/// not an error here — it is a [`Relayed`] with a non-zero [`Relayed::exit`] and the child's own
/// output, because that is a real answer and the caller needs to see it.
pub fn relay(binary: &VerifierBinary, request: &RelayRequest<'_>) -> Result<Relayed, EngineError> {
    let mut cmd = Command::new(binary.path());
    cmd.arg("verify")
        .arg(request.grounding)
        .arg("--citations")
        .arg(request.citations)
        .arg("--grounding")
        .arg(request.adapter);

    if request.fail_on_ungrounded {
        cmd.arg("--fail-on-ungrounded");
    }
    if let Some(c) = request.config {
        cmd.arg("--config").arg(c);
    }
    if let Some(o) = request.out {
        cmd.arg("--out").arg(o);
    }

    let out = cmd.output().map_err(|e| EngineError::Io {
        detail: format!(
            "could not spawn the verifier at {}: {e}",
            binary.path().display()
        ),
    })?;

    Ok(Relayed {
        exit: map_status(out.status.code()),
        // Moved, not inspected. There is deliberately no `String::from_utf8` here: the report is
        // the verifier's bytes, and re-encoding them — even losslessly — would make the engine a
        // participant in a format it has no opinion about.
        stdout: out.stdout,
        stderr: out.stderr,
    })
}

/// The child's status, as the engine's.
///
/// Only two codes are forwarded, and everything else collapses to "did not run". That is
/// deliberate: an unrecognised child status means the engine does not know what happened, and
/// the honest report of not knowing is the same code a document it could not read produces.
fn map_status(code: Option<i32>) -> i32 {
    match code {
        Some(0) => RELAY_OK,
        Some(1) => RELAY_REFUSED,
        _ => RELAY_UNAVAILABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_pin_that_names_no_file_is_a_hard_error() {
        let e = VerifierBinary::resolve(Some(Path::new("/no/such/verifier")), None)
            .expect_err("an explicit pin is authoritative");
        assert_eq!(e.code(), "missing_part");
        assert!(
            e.to_string().contains("named explicitly"),
            "the error must say why it did not fall back: {e}"
        );
    }

    #[test]
    fn an_absent_verifier_is_named_never_skipped() {
        // No explicit pin, no sibling root. Whether `ethos` happens to be on this machine's PATH
        // decides the outcome, and both are correct — what must never happen is a silent success.
        match VerifierBinary::resolve(None, Some(Path::new("/definitely/not/a/repo"))) {
            Err(e) => {
                assert_eq!(e.code(), "missing_part");
                assert!(e.to_string().contains("Tried, in order"));
                assert!(
                    e.to_string().contains("never a skip"),
                    "the message must say absence is a failure: {e}"
                );
            }
            Ok(found) => {
                assert!(found.path().is_file());
                assert!(
                    !found.version().is_empty(),
                    "a pinned binary reports a version"
                );
            }
        }
    }

    #[test]
    fn the_status_map_forwards_only_what_it_understands() {
        assert_eq!(map_status(Some(0)), RELAY_OK);
        assert_eq!(map_status(Some(1)), RELAY_REFUSED);
        // Anything else is "the engine does not know what happened".
        for unknown in [2, 3, 101, 255, -1] {
            assert_eq!(
                map_status(Some(unknown)),
                RELAY_UNAVAILABLE,
                "child status {unknown} must not be forwarded as if understood"
            );
        }
        // Killed by a signal: no code at all.
        assert_eq!(map_status(None), RELAY_UNAVAILABLE);
    }

    #[test]
    fn the_three_relay_codes_are_distinct() {
        // The same discipline as the classify exit codes: "refused" and "did not run" must never
        // collapse, or a caller's `&&` chain means two different things.
        let codes = [RELAY_OK, RELAY_REFUSED, RELAY_UNAVAILABLE];
        let mut unique = codes;
        unique.sort_unstable();
        let before = unique.len();
        unique.iter().for_each(drop);
        let mut dedup = unique.to_vec();
        dedup.dedup();
        assert_eq!(dedup.len(), before);
    }

    #[test]
    fn a_pin_carries_both_version_and_digest() {
        let b = VerifierBinary {
            path: PathBuf::from("/tmp/ethos"),
            version: "ethos 0.6.0".into(),
            sha256: Sha256Hex::from_hex(&"cd".repeat(32)).unwrap(),
        };
        match b.pin() {
            VerifierPin::Pinned { version, sha256 } => {
                assert_eq!(version, "ethos 0.6.0");
                assert_eq!(sha256, *b.sha256());
            }
            VerifierPin::NotPinned => panic!("a resolved binary pins"),
        }
    }

    #[test]
    fn the_adapter_is_declared_rather_than_left_to_sniffing() {
        // Measured against the pinned binary: `ethos-grounding-json` is the only one of its three
        // adapters that loads an `ethos.grounding.v1` artifact.
        assert_eq!(GROUNDING_ADAPTER, "ethos-grounding-json");
    }
}
