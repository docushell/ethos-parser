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
//! Owns classification (reason codes on two orthogonal axes, counts, bounded sampling) and
//! extraction (position-aware text runs, native locators, measured font metrics, fail-closed
//! operator handling). See `docs/03-V0-SCOPE.md`.
//!
//! **Boundary:** this crate contains no grounding concept. It produces representation nodes;
//! projecting them is `engine-grounding`'s job.
//!
//! **Status: M0 skeleton.** Classification lands at M2, extraction at M3.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The crate name, used by the M0 harness to prove the workspace links.
pub const CRATE_NAME: &str = "engine-pdf";
