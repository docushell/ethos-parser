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
//! `docs/01-CONTRACT.md` expressed as Rust. After M1 the artifact shape stops being negotiable
//! and starts being a compile error.
//!
//! # What lives here
//!
//! | Module | Owns |
//! | --- | --- |
//! | [`c14n`] | The one canonical JSON serialization. Integers only, keys sorted at write time |
//! | [`geom`] | Integer quanta, [`quantize`], and [`QRect`] as `[x0, y0, x1, y1]` |
//! | [`identity`] | [`ArtifactIdentity`], [`Sha256Hex`], [`CoordinateSystem`] |
//! | [`profile`] | [`Profile`] and its hash — the identity everything else hangs off |
//! | [`derivation`] | [`DerivationClass`] and typed geometric absence |
//! | [`assurance`] | The L1 gate: [`Limitation`], [`PageState`], [`CoverageSummary`], terminal state |
//! | [`ids`] | Stable-id allocation and the ordering discipline |
//! | [`error`] | The six-variant error taxonomy |
//! | [`diagnostics`] | Volatile observations, quarantined off the artifact and off by default |
//! | [`verifier`] | Spawning a verifier and relaying its bytes — **never** reading them |
//! | [`tables`] | Table occupancy: `CellSlot`, spans, and the structural half of the cross-check |
//!
//! # Boundary
//!
//! **No PDF concept appears in this crate.** No `lopdf`, no content-stream operator, no page
//! tree. If a PDF type appears here, the second format becomes a rewrite instead of a variant
//! (`docs/04-ARCHITECTURE.md` §1).
//!
//! **No verification concept appears either.** No claim, no verdict, no `grounded`, no
//! `evidence_tier` (`docs/07-VERIFY-BOUNDARY.md`).
//!
//! # Three rules this crate enforces in the type system
//!
//! 1. **Floats do not exist in canonical output.** [`c14n::c14n_bytes`] rejects any non-integer
//!    number, at any depth. Geometry arrives pre-quantized as `i64`.
//! 2. **No public confidence field**, score, grade, or quality summary — anywhere, at any
//!    version (`docs/01-CONTRACT.md` §9). A test scans this crate's own sources to enforce it,
//!    because the rule is easiest to break with good intentions.
//! 3. **Absence is typed.** [`derivation::GeometryPresence`] distinguishes "could not measure"
//!    from "nothing to measure" from "not asked to measure". `Option<QRect>` would collapse all
//!    three, and only the first is a declarable capability limitation.
//! 4. **A gap cannot be presented as a success.** [`assurance::Assurance`] derives its coverage
//!    and terminal state from the page states it is given, so no artifact can claim `Complete`
//!    while carrying a page nobody read (`docs/01-CONTRACT.md` §7).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod assurance;
pub mod c14n;
pub mod derivation;
pub mod diagnostics;
pub mod error;
pub mod geom;
pub mod identity;
pub mod ids;
pub mod profile;
pub mod representation;
pub mod tables;
pub mod verifier;

pub use assurance::{
    codes, page_binding_status, Assurance, CoverageSummary, Limitation, LimitationScope,
    PageBindingResult, PageState, PageStateEntry, ProcessingGaps, ProcessingTerminalState,
    RefusalCode,
};
pub use c14n::{c14n_bytes, sha256_hex, sha256_hex_bytes, C14nError};
pub use derivation::{DerivationClass, GeometryAbsence, GeometryPresence};
pub use diagnostics::{Diagnostics, DiagnosticsRun, HostInfo, Stage, DIAGNOSTICS_VERSION};
pub use error::EngineError;
pub use geom::{quantize, QRect, QRectError, QuantizeError, MAX_SAFE_INT, QUANTUM_PER_POINT};
pub use identity::{
    ArtifactBinding, ArtifactIdentity, CoordinateOrigin, CoordinateSystem, CoordinateUnit,
    Sha256Hex,
};
pub use ids::{sort_ids, IdAllocator, IdKind, NodeId};
pub use profile::{
    profile_sha256, BackendIdentity, Capabilities, PageBudget, Profile, RasterDpi, TableDetection,
    VerifierPin, XrefRepair, CMAP_DATA_VERSION, FORM_ANNOTATION_RULE_V1, OBSERVATION_RULE_V1,
    READING_ORDER_RULE_V0, READING_ORDER_RULE_V1, STRUCT_TREE_RULE_V1, TABLE_DETECTION_STROKE_V1,
    TABLE_DETECTION_UNRULED_V1, TABLE_DETECTION_V1, TABLE_DETECTION_V2, TEXT_CODE_RULE_V1,
};
pub use tables::{
    CellSlot, CheckStatus, GeometricFault, LocatorCheck, SlotCover, SlotFault, TableCellPosition,
    TableCellRecord, TableRecord, TaggedGridCheck, TaggedGridFault, TaggedGridStatus,
    LOCATOR_CHECK_V1, TAGGED_GRID_CHECK_V1,
};

pub use verifier::{
    relay, RelayRequest, Relayed, VerifierBinary, GROUNDING_ADAPTER, RELAY_OK, RELAY_REFUSED,
    RELAY_UNAVAILABLE,
};

pub use representation::{
    AnnotationAttributes, AnnotationRect, DocumentRepresentation, FieldValue, FormFieldAttributes,
    ImageAttributes, ImageMediaType, NativeLocator, Node, NodeAttributes, NodeGeometry, NodeKind,
    PageRecord, PaintedRect, PdfArtifactLocator, PdfImageLocator, PdfLocator, PdfObjectLocator,
    PdfTaggedLocator, ProcessingRun, ProcessorIdentity, RepresentationPayload, SourceIdentity,
    StructuralLocator, SynthesizedAt, TextFinding, TextRunAttributes, REPRESENTATION_ARTIFACT_TYPE,
    REPRESENTATION_SCHEMA_VERSION,
};

/// The crate name, asserted by the M0 harness to prove the workspace links.
pub const CRATE_NAME: &str = "engine-core";
