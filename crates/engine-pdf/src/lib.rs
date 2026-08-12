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
//! # What exists at M2
//!
//! [`Document`] opens a PDF once, and [`classify`] reports what it observed: per-page counts and
//! named reason codes on two orthogonal axes, with a derived boolean. Nothing here renders a
//! verdict, scores quality, or decides where a document should be routed.
//!
//! # Boundary
//!
//! This crate contains no grounding concept. It produces observations and, at M3, representation
//! nodes; projecting them is `engine-grounding`'s job.
//!
//! # Not yet implemented
//!
//! Extraction (M3): content-stream interpretation with an enumerated operator set, `NativeLocator`
//! emission, measured ink boxes, vendored CMaps, synthesized-character flags. The classifier walks
//! content streams to *count* operators; it does not interpret them, and it decodes no text.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod classify;
pub mod cmap;
pub mod content;
pub mod document;
pub mod encoding;
pub mod extract;
pub mod fonts;
pub mod limitations;
pub mod magic;
pub mod metrics;
pub mod nodes;
pub mod ops;
pub mod reasons;
pub mod text_state;
pub mod thresholds;

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
