//! Shared by the CLI integration tests that read what the verifier printed.
//!
//! One function, and deliberately one: `oracle.rs` already carries `extract_report` for the
//! *validation* report, and the same unwrapping copied separately into `html_cli.rs`,
//! `markdown_cli.rs` and `verify_relay.rs` is how two readers of one wire format begin to
//! disagree about it.

use serde_json::Value;

/// Find the verification report in whatever the relay printed.
///
/// **Both shapes are accepted, and the reason is a rebuild, not defensiveness.** Ethos wraps this
/// report in an in-toto Statement (`_type` / `subject` / `predicateType` / `predicate`), so which
/// shape reaches a test depends on which Ethos binary answered. `oracle.rs` records the same
/// hazard for the validation report — there a binary was behind its own source and printed bare —
/// and defends against it the same way. These three files did not, and all seven of their
/// verifier-reading tests failed the first time the oracle was built from source: the verdict
/// inside was right, and `report["all_evidence_grounded"]` was reading one level too high.
///
/// Accepting both is what makes an `ETHOS_ORACLE_REF` bump a decision about verifier behaviour
/// rather than an envelope-shaped test failure.
///
/// Anything that is neither shape is a hard failure, never a guess. `Value`'s index operator
/// answers `Null` for a missing key, so a test that shrugged here would assert against `Null`
/// and report success — which is the outcome `docs/07-VERIFY-BOUNDARY.md` exists to prevent.
pub fn verification_report(stdout: &[u8]) -> Value {
    let text = String::from_utf8_lossy(stdout);
    let value: Value = serde_json::from_str(text.trim())
        .unwrap_or_else(|e| panic!("the verifier's report is not JSON ({e}). stdout was:\n{text}"));

    if value.get("all_evidence_grounded").is_some() {
        return value;
    }
    if let Some(predicate) = value.get("predicate") {
        if predicate.get("all_evidence_grounded").is_some() {
            return predicate.clone();
        }
    }
    panic!(
        "the report is neither a bare verification report nor a statement wrapping one. \
         Refusing to guess which field holds the verdict:\n{value}"
    );
}
