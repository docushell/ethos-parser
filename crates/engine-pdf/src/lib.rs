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

//! `engine-pdf` — the PDF reader.
//!
//! # The v0 surface
//!
//! Three stages over one handle. [`Document`] opens a PDF **once** and every stage borrows it
//! (`docs/04-ARCHITECTURE.md` §2.1):
//!
//! | Call | Produces |
//! | --- | --- |
//! | [`classify`] | Per-page counts and named reason codes on two orthogonal axes, plus a derived boolean |
//! | [`extract`] | Position-aware text runs with native locators, measured ink boxes or typed absence, and synthesized-character flags |
//! | [`to_representation`] | `DocumentRepresentation v0` — the canonical record, sealed with its fingerprint |
//!
//! Nothing here renders a verdict, scores quality, or decides where a document should be routed.
//! The classifier emits counts and reasons; the caller owns the policy.
//!
//! # Boundary
//!
//! This crate contains no grounding concept. It produces observations and representation nodes;
//! projecting them is `engine-grounding`'s job.
//!
//! # What is public, and what is not
//!
//! The items re-exported below, plus [`exit`] and [`limitations`], are the supported surface —
//! `docs/PUBLIC-API.md` is the list, and `crates/engine-cli/tests/public_api.rs` fails if this
//! file grows an export that document does not name.
//!
//! The parsing machinery — the operator table, the content-stream interpreter, font and CMap
//! resolution, encoding tables, text state, thresholds — is **private**. It was public through
//! M6 and narrowed at M7: those modules carry `f64` fields, borrow-scoped handles and calibration
//! constants that are implementation, not contract. Anything genuinely needed by a caller is
//! re-exported here by name.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod limitations;

pub(crate) mod classify;
pub(crate) mod cmap;
pub(crate) mod content;
pub(crate) mod document;
pub(crate) mod encoding;
pub(crate) mod extract;
pub(crate) mod fonts;
pub(crate) mod magic;
pub(crate) mod metrics;
pub(crate) mod nodes;
pub(crate) mod ops;
pub(crate) mod reasons;
pub(crate) mod represent;
pub(crate) mod text_state;
pub(crate) mod thresholds;
pub(crate) mod xref;

#[cfg(test)]
mod test_support;

pub use classify::{
    classify, Classification, PageClassification, SourceRef, CLASSIFICATION_ARTIFACT_TYPE,
    CLASSIFICATION_SCHEMA_VERSION,
};
pub use document::Document;
pub use extract::{extract, ExtractArtifact, EXTRACT_ARTIFACT_TYPE, EXTRACT_SCHEMA_VERSION};
pub use magic::check_pdf_magic;
pub use nodes::{PageExtract, PdfLocator, SynthesisReason, SynthesizedChar, TextRun};
pub use reasons::{LayoutComplexityReason, OcrNeedReason};
pub use represent::{to_representation, PROCESSOR_NAME};

/// The crate name, asserted by the M0 harness to prove the workspace links.
pub const CRATE_NAME: &str = "engine-pdf";

/// Process exit codes (`docs/03-V0-SCOPE.md` §3.1).
///
/// **Three outcomes, three codes, never collapsed.** LiteParse's own README predicate
/// (`lit is-complex doc.pdf --quiet && lit parse …`) returns 1 for password-protected,
/// invalid-header, corrupt-header **and** a missing file — identically to "this document is
/// complex" (parity checklist L16). It fails closed, but with an indistinguishable signal, which
/// is still a defect: a caller cannot tell "hard" from "I could not open this".
pub mod exit {
    use engine_core::EngineError;

    use crate::classify::Classification;

    /// No reason code fired on either axis.
    pub const SIMPLE: i32 = 0;
    /// The document was read, and at least one reason code fired.
    pub const NEEDS_ATTENTION: i32 = 1;
    /// The document could not be read.
    pub const COULD_NOT_READ: i32 = 2;

    /// Map a classification outcome to its exit code.
    ///
    /// Shared by the CLI and its tests so the mapping is asserted where it is defined rather than
    /// re-derived at the call site.
    pub fn exit_code(result: &Result<Classification, EngineError>) -> i32 {
        match result {
            Ok(c) if c.needs_attention => NEEDS_ATTENTION,
            Ok(_) => SIMPLE,
            Err(_) => COULD_NOT_READ,
        }
    }
}
