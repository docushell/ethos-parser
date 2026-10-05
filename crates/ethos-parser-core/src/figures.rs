// Copyright 2026 The ethos-parser maintainers
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

//! Figure regions — `docs/34-FIGURE-REGIONS-SCOPE.md`.
//!
//! A box this engine inferred from the paths a page paints: a chart, a diagram or a logo drawn
//! with lines and curves rather than placed as a raster image.
//!
//! # Why this is not a [`crate::Node`]
//!
//! An image node states *these bytes were drawn here* and carries a digest of the stream. A figure
//! region states *this engine read these paths as one figure*, and there are no bytes behind it.
//! Folding the second into the first would leave a consumer unable to tell a placement the document
//! made from a grouping this engine made. So it is a record of its own, on
//! [`crate::TableRecord`]'s precedent, and it carries no text: the runs inside it stay nodes, in
//! reading order, and nothing here claims one.

use serde::{Deserialize, Serialize};

use crate::ids::NodeId;

/// One figure region, as it appears on the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FigureRecord {
    /// The page this region is on.
    pub page: NodeId,
    /// Its box, in the artifact's declared coordinate system: always `Measured`, the union of the
    /// painted paths' boxes the rule clustered.
    pub geometry: crate::derivation::GeometryPresence,
    /// Always [`crate::DerivationClass::Computed`]: the paths are the document's, the grouping is
    /// this engine's.
    pub derivation: crate::derivation::DerivationClass,
    /// The rule that drew the box — `figure-regions-v1`
    /// ([`crate::FIGURE_RULE_V1`]).
    pub detection_rule: String,
}
