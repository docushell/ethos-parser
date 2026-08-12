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

//! `engine-grounding` — the projection and its validator.
//!
//! Projects `DocumentRepresentation v0` into `ethos.grounding.v1`, and validates a grounding
//! artifact against its schema plus its source bytes. See `docs/01-CONTRACT.md` §11.
//!
//! **Boundary — two of them, and both matter:**
//!
//! 1. **No PDF concept.** This crate projects the *representation*, never the document. If it
//!    reaches for a page tree, DOCX support becomes a rewrite instead of a variant.
//! 2. **No verification.** `grounding-check` validates structure and source binding only. It
//!    does not check claims, does not emit `grounded`, does not compute `evidence_tier`, and
//!    never re-derives anything from a verifier's report. See `docs/07-VERIFY-BOUNDARY.md`.
//!
//! **Status: M0 skeleton.** The adapter lands at M5, the validator at M6.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The crate name, used by the M0 harness to prove the workspace links.
pub const CRATE_NAME: &str = "engine-grounding";
