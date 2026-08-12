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

//! `engine-core` — the contract, in types.
//!
//! Owns artifact identity, the profile and its hash, the coordinate-system declaration,
//! c14n v1, integer quanta, stable-id ordering, derivation classes, typed absence, and the
//! error taxonomy. See `docs/01-CONTRACT.md`.
//!
//! **Boundary:** this crate contains no PDF concept. No `lopdf`, no content-stream operator,
//! no page tree. If a PDF type appears here, the second format becomes a rewrite.
//!
//! **Status: M0 skeleton.** Types land at M1 (`docs/05-MILESTONES.md`).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// The crate name, used by the M0 harness to prove the workspace links.
pub const CRATE_NAME: &str = "engine-core";
