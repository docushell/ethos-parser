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

//! `DocumentRepresentation v0` — the canonical evidence record (`docs/01-CONTRACT.md` §2, §5).
//!
//! **The representation is the record; the grounding artifact is a projection of it.** Never
//! treat a grounding round-trip as proof the representation is intact — the projection is
//! deliberately lossy, and `engine-grounding` has a test enumerating exactly what it drops.
//!
//! # Why the locator union lives in this crate
//!
//! `docs/04-ARCHITECTURE.md` §1 says `engine-core` contains no PDF concept, and its own header
//! says that where it and `01-CONTRACT.md` disagree, **the contract is right and the
//! architecture doc is the bug**. §5.1 of the contract is explicit: *"`NativeLocator` is a
//! discriminated union. Adding a format adds a variant — `PdfLocator`, `DocxLocator`, … v0 ships
//! `PdfLocator` and nothing else."* The union is part of the artifact contract, so it lives with
//! the contract.
//!
//! The line that actually matters is **machinery, not vocabulary**: no `lopdf`, no
//! content-stream operator, no page tree, no font program. [`PdfLocator`] is four integers and a
//! discriminant. Nothing in this crate can parse a PDF, and nothing here knows one exists beyond
//! the name of an addressing scheme — the same line `BackendIdentity { name: "lopdf" }` already
//! sits on.
//!
//! **`engine-grounding` never reads a locator at all.** The projection addresses pages by node
//! id, so it never matches on a format variant; a test asserts the crate does not so much as
//! mention `NativeLocator`. That is what keeps the second format from becoming a rewrite, and it
//! is a stronger guarantee than hiding the type would have given.
//!
//! # The fingerprint, and why it is a subtree rather than a filter
//!
//! [`DocumentRepresentation::representation_c14n_sha256`] is
//! `"sha256:" + hex(sha256(c14n(doc["representation"])))` — the digest of a **literal subtree of
//! the emitted document**. A reader recomputes it with no domain knowledge: canonicalize that
//! one member, hash it.
//!
//! The alternative — hash the whole document minus a named set of keys — makes canonicalization
//! a second rule two implementations can drift on, and a drifted rule produces a mismatch that
//! looks exactly like tampering. Keeping the hashed bytes literal means a reader who gets it
//! wrong gets it *obviously* wrong.

use serde::{Deserialize, Serialize};

use crate::assurance::Assurance;
use crate::c14n::{c14n_bytes, sha256_hex_bytes};
use crate::derivation::{DerivationClass, GeometryPresence};
use crate::error::EngineError;
use crate::geom::QRect;
use crate::identity::{ArtifactIdentity, CoordinateSystem, Sha256Hex};
use crate::ids::NodeId;

/// Artifact type for the canonical record. **DRAFT** — see `docs/draft-schemas/`.
pub const REPRESENTATION_ARTIFACT_TYPE: &str = "ethos.engine.representation.v0";

/// Shape version of the representation artifact. **DRAFT**.
pub const REPRESENTATION_SCHEMA_VERSION: &str = "0.3.0";

/// What was read: the media type and the digest of the exact source bytes.
///
/// **Source identity, not representation identity.** `docs/01-CONTRACT.md` §2 requires both and
/// they are not the same thing: this is what went in, and
/// [`DocumentRepresentation::representation_c14n_sha256`] is what came out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    /// Always `application/pdf` at v0.
    pub media_type: String,
    /// Digest of the exact source bytes.
    pub sha256: Sha256Hex,
}

/// Who produced this representation, and under what pinned configuration.
///
/// The companion names a `ProcessingRun`/`StageRun` carrying processor, adapter, build and
/// profile identities. v0 has one processor and one stage, so this is the honest subset: naming
/// a `stages` array with a single hardcoded entry would be shape without information.
///
/// **No run id and no timestamp.** Either would make two runs over identical bytes under an
/// identical profile produce different artifacts, and byte identity across runs is a test here
/// (`docs/05-MILESTONES.md`, standing rule 6). A run's identity *is* its inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingRun {
    /// The processor that read the source.
    pub processor: ProcessorIdentity,
    /// Version id of the reading-order rule that ordered the nodes.
    ///
    /// On the artifact so a reader need not fetch the profile to know which rule produced this
    /// order. `single-column-v1` at v0 — and a two-column document is therefore in the wrong
    /// order, which `assurance.limitations` declares rather than hides.
    pub reading_order_rule: String,
}

/// Processor name, build, and the backend it read through.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessorIdentity {
    /// Engine name.
    pub name: String,
    /// Engine build.
    pub version: String,
    /// Backend name and version, e.g. `lopdf 0.44.0`.
    pub backend: String,
}

/// The format-native address of a node. **Required on every node** (`01-CONTRACT.md` §5.1).
///
/// A discriminated union: adding a format adds a variant, plus an adapter profile, fixtures and
/// inspection behaviour. It does not change the source, run, artifact or verification models.
///
/// Externally tagged so a new variant is additive on the wire and an unrecognised one fails
/// closed rather than being best-effort parsed (§8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum NativeLocator {
    /// The PDF variant. v0 ships this and nothing else.
    Pdf(PdfLocator),
}

/// A PDF node's native address: page plus character origin plus advance.
///
/// The origin is the fingerprint-critical primitive. Two independent PDF stacks agree on
/// character origin to 0.001 pt while disagreeing on box height by 6.174 pt — which is the whole
/// reason origins are identity and boxes are inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfLocator {
    /// 1-based page number, as the document numbers its own pages.
    pub page: u32,
    /// Baseline origin x, integer centipoints, in the declared coordinate system.
    pub origin_x: i64,
    /// Baseline origin y, integer centipoints, in the declared coordinate system.
    pub origin_y: i64,
    /// Advance width in centipoints, or **absent** when the document carries no widths.
    ///
    /// `None` is not zero. Absent means the reader does not know, and says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advance: Option<i64>,
}

/// A structural address, where the node kind defines one.
///
/// # Three answers, and they are not the same answer (v1-S3)
///
/// v0 through v1-S2 emitted marked-content ids only: the `BDC` operand, captured verbatim, with
/// no idea what it referred to. v1-S3 reads the document's `/StructTreeRoot`, so a node can now
/// carry the address the **author** gave it. The variants are the three distinct things that can
/// be true, and collapsing any two of them would lose information a consumer needs:
///
/// | Variant | What happened |
/// | --- | --- |
/// | [`Self::PdfTagged`] | the tree cites this `(page, mcid)`, so the author placed this text here |
/// | [`Self::PdfMcid`] | the content stream gave an id and **the tree did not cite it** |
/// | [`Self::PdfArtifact`] | the content stream marked this as page furniture, outside the tree |
/// | absent | the content stream marked nothing here at all |
///
/// A `PdfMcid` on a tagged document is a real gap and is counted as one; it is not the same as a
/// document with no tree, and neither is the same as text the author deliberately excluded from
/// its structure. Nothing here is inferred: no role is guessed from a font size or a text prefix,
/// which is the defect the parity checklist records as P14.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum StructuralLocator {
    /// A PDF marked-content id, captured verbatim from `BDC`, **which no structure element
    /// claims**.
    ///
    /// Either the document has no structure tree, or it has one and this id is not in it. The
    /// assurance block's limitations say which; this variant alone does not, because a node
    /// cannot see the document.
    PdfMcid(i64),
    /// The address the document's own structure tree gives this content.
    PdfTagged(PdfTaggedLocator),
    /// Content the page marked as an **artifact**: running heads, folios, rules, decoration.
    ///
    /// Page furniture the author deliberately kept out of the structure tree (PDF 32000-1
    /// §14.8.2.2). It is flagged, **never dropped** — the parity checklist's O21/O22 are about
    /// exactly this: a reader that deletes running heads has silently edited the document, and a
    /// consumer cannot tell the difference between text that was not there and text that was
    /// removed on its behalf.
    PdfArtifact(PdfArtifactLocator),
}

/// A node's address in the document's tagged-structure tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfTaggedLocator {
    /// The marked-content id the tree cited to reach this content.
    pub mcid: i64,
    /// Structure types from the root down, **exactly as the document wrote them** in `/S`.
    ///
    /// Never laundered. A document using a custom type gets its custom name here, so a consumer
    /// reading this path sees what the file says rather than what this engine made of it.
    pub role_path: Vec<String>,
    /// The same path after the document's own `/RoleMap` is applied.
    ///
    /// **Present only when the map actually changed something**, so its presence is the signal
    /// that a custom type was in play. A `/RoleMap` is the document telling us what its custom
    /// types mean; applying it is reading the file, and guessing without one would not be
    /// (`docs/09-V1-MILESTONES.md` S3, decision 8).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standard_role_path: Option<Vec<String>>,
    /// The innermost element's `/ID`, when it declares one.
    ///
    /// Absent means the document supplied none. Never minted here — an identifier this engine
    /// invented would not address anything in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
}

/// A node the page marked as an artifact rather than as content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdfArtifactLocator {
    /// The marked-content id, when the artifact sequence carried one.
    ///
    /// Usually absent: an artifact is by definition not a structure content item, so it has no
    /// reason to be numbered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcid: Option<i64>,
}

/// What a node is.
///
/// v0 emits one kind. The enum exists so that adding `Paragraph` or `TableCell` later is a new
/// variant rather than a change of meaning for an existing one — and so a reader that meets an
/// unknown kind fails closed instead of guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NodeKind {
    /// One run of text shown by a single show-text operator.
    ///
    /// **Not a line and not a paragraph.** v0 performs no grouping, so a run is exactly what the
    /// content stream drew in one operation. Calling it a block would be a claim about layout
    /// that no code here makes.
    TextRun,
}

impl NodeKind {
    /// The wire spelling, which is also the grounding artifact's `kind`.
    ///
    /// Constrained to `^[a-z0-9][a-z0-9_-]*$` by `ethos.grounding.v1`; a test pins that the
    /// spelling still matches after any rename.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextRun => "text_run",
        }
    }
}

/// Format-specific facts about a node that do not fit the common fields.
///
/// Kept as a typed struct rather than an open map: an open map is a place for a future field to
/// arrive unreviewed, and `deny_unknown_fields` cannot police one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRunAttributes {
    /// The character codes that produced the text, in order.
    ///
    /// **Not 1:1 with `text`.** A ligature is one code and several scalars.
    pub char_codes: Vec<u32>,
    /// True when `text.chars().count() != char_codes.len()`. Declared, not reconciled.
    pub scalar_code_mismatch: bool,
    /// Characters this reader inserted, by index into `text`. Empty for verbatim text.
    pub synthesized: Vec<SynthesizedAt>,
    /// Font resource name, as the document names it.
    pub font_id: String,
    /// Font size in integer centipoints. **Never used as a box height.**
    pub font_size: i64,
}

/// A character the reader authored rather than read, flagged where it was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynthesizedAt {
    /// Index into the node's `text`, counted in `char`s.
    pub char_index: u32,
    /// Why it was inserted, in the format's own vocabulary.
    pub reason: String,
}

/// One page of the source, as a record rather than a node.
///
/// **Pages are deliberately not nodes.** A node carries a required `NativeLocator`, and the PDF
/// variant of that is a *character* origin — a page has none, so making a page a node would
/// force the emitter to write an origin it does not have. `{"origin_x":0,"origin_y":0}` for a
/// page is indistinguishable on the wire from a run drawn in the corner, and inventing a
/// coordinate is forbidden outright (`01-CONTRACT.md` §5.2, Workbench rule 3). Keeping pages in
/// their own list costs one array and removes the temptation entirely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRecord {
    /// Stable id within this representation, e.g. `p1`.
    pub id: NodeId,
    /// **1-based page number as the document numbers it** — not this record's position in the
    /// array. The two differ the moment a page is quarantined or fails, and a citation renders
    /// against the document's number, not against our array index.
    pub index: u32,
    /// Page width in integer centipoints, after rotation.
    pub width: i64,
    /// Page height in integer centipoints, after rotation.
    pub height: i64,
    /// Page rotation as declared: 0, 90, 180 or 270.
    pub rotation: i64,
}

/// One ordered, typed node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// Stable within this representation **and this profile**, never globally.
    ///
    /// A profile upgrade creates a new representation plus a mapping, rather than pretending
    /// node ids are permanently global (companion, and `ids` module).
    pub id: NodeId,
    /// What this node is.
    pub kind: NodeKind,
    /// The page this node was drawn on, by page-record id.
    pub parent: NodeId,
    /// Position among the nodes sharing this parent, **1-based**, in reading order.
    pub ordinal: u32,
    /// The decoded text.
    pub text: String,
    /// The required format-native address.
    pub native_locator: NativeLocator,
    /// A structural address where the document supplied one. Never invented.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structural_locator: Option<StructuralLocator>,
    /// How this node came to exist. `Extracted` for text read from the content stream.
    pub derivation: DerivationClass,
    /// Format-specific facts.
    pub attributes: TextRunAttributes,
}

/// Geometry for one node, carried **outside** the fingerprinted payload.
///
/// `docs/01-CONTRACT.md` §4 excludes geometry from the fingerprint deliberately: rectangles are
/// preserved for display and inspection but are not fingerprint-critical, because text is
/// anchored by stable origin locators rather than by boxes. Fingerprinting the boxes would make
/// artifact identity hostage to the least reliable number in it.
///
/// Implemented by **keeping the type out of the payload** rather than by filtering it out at
/// hashing time. A filter is a rule someone can quietly change; a type that is not there cannot
/// be hashed by accident.
///
/// # The trust boundary, stated plainly
///
/// Because these rows are outside the digest, **a verified representation attests to the text,
/// the reading order and the origins — not to the rectangles.** That matters more than it first
/// appears, because the grounding projection drops the locators and keeps only the boxes: the one
/// thing the fingerprint does not cover is the only spatial claim that survives into
/// `ethos.grounding.v1`. §4's reasoning still holds for the *record* (boxes are the least reliable
/// number in it, and identity must not be hostage to them); this note exists so nobody reads a
/// verified fingerprint as covering a bbox it never touched.
///
/// What *is* bound across the seam is the **count**: a representation whose sidecar disagrees with
/// its own `geometry-absent-not-groundable` declaration is refused at construction and at parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeGeometry {
    /// The node this row describes. Rows are in the same order as [`RepresentationPayload::nodes`].
    pub node: NodeId,
    /// A measured ink box, or a typed reason there is none.
    pub presence: GeometryPresence,
}

/// Everything the fingerprint covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationPayload {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    pub identity: ArtifactIdentity,
    /// What was read.
    pub source: SourceIdentity,
    /// Who read it, under what pinned configuration.
    pub processing_run: ProcessingRun,
    /// Declared on every artifact carrying geometry, never implied by the format (§3).
    pub coordinate_system: CoordinateSystem,
    /// Pages of the source, in page order.
    pub pages: Vec<PageRecord>,
    /// Ordered typed nodes, in reading order, grouped by page.
    pub nodes: Vec<Node>,
    /// Ruled tables, in page order (v1-S1).
    ///
    /// **An empty array means the detector looked and found none**, never that it did not look —
    /// `capabilities.tables` says which. Carried as its own array rather than as `nodes`: a table
    /// cell's text is a concatenation of runs that are *already* nodes, and emitting it twice
    /// would create a second place for the same text to live and a second place for it to drift.
    #[serde(default)]
    pub tables: Vec<crate::tables::TableRecord>,
    /// M4's L1 gate, carried forward by value: capabilities, limitations, per-page state,
    /// coverage, terminal state.
    pub assurance: Assurance,
}

impl RepresentationPayload {
    /// The canonical bytes this payload hashes over.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the payload will not canonicalize.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        let value = serde_json::to_value(self).map_err(|e| EngineError::Malformed {
            what: "representation payload".into(),
            detail: e.to_string(),
        })?;
        c14n_bytes(&value).map_err(|e| EngineError::Malformed {
            what: "representation payload".into(),
            detail: e.to_string(),
        })
    }

    /// `sha256:<hex>` over [`Self::canonical_bytes`].
    ///
    /// # Errors
    ///
    /// Propagates [`Self::canonical_bytes`].
    pub fn fingerprint(&self) -> Result<Sha256Hex, EngineError> {
        let bytes = self.canonical_bytes()?;
        Ok(Sha256Hex::from_hex(&sha256_hex_bytes(&bytes))
            .expect("sha256 hex is always 64 lowercase hex digits"))
    }
}

/// The emitted canonical record.
///
/// Constructed only through [`Self::seal`], which computes the fingerprint and validates every
/// geometry box against its page. There is no constructor that *accepts* a fingerprint, for the
/// same reason [`Assurance`] derives its terminal state instead of accepting one: a value that
/// can be asserted independently of what it describes is a value that can disagree with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RepresentationWire", into = "RepresentationWire")]
pub struct DocumentRepresentation {
    artifact_type: String,
    schema_version: String,
    representation: RepresentationPayload,
    representation_c14n_sha256: Sha256Hex,
    geometry: Vec<NodeGeometry>,
}

/// The wire form, so `deny_unknown_fields` and the structural checks both apply on parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepresentationWire {
    artifact_type: String,
    schema_version: String,
    representation: RepresentationPayload,
    representation_c14n_sha256: Sha256Hex,
    geometry: Vec<NodeGeometry>,
}

impl From<DocumentRepresentation> for RepresentationWire {
    fn from(d: DocumentRepresentation) -> Self {
        Self {
            artifact_type: d.artifact_type,
            schema_version: d.schema_version,
            representation: d.representation,
            representation_c14n_sha256: d.representation_c14n_sha256,
            geometry: d.geometry,
        }
    }
}

impl TryFrom<RepresentationWire> for DocumentRepresentation {
    type Error = EngineError;

    /// Structural well-formedness, checked at **parse** time.
    ///
    /// The fingerprint is deliberately *not* checked here — see
    /// [`DocumentRepresentation::verify_fingerprint`] for why a validator that cannot load the
    /// artifact it is judging cannot describe it.
    fn try_from(w: RepresentationWire) -> Result<Self, Self::Error> {
        let d = Self {
            artifact_type: w.artifact_type,
            schema_version: w.schema_version,
            representation: w.representation,
            representation_c14n_sha256: w.representation_c14n_sha256,
            geometry: w.geometry,
        };
        d.check_structure()?;
        Ok(d)
    }
}

impl DocumentRepresentation {
    /// Seal a payload: validate it, attach its geometry, compute the fingerprint.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the geometry sidecar does not describe exactly the nodes in
    /// order, if a node's parent is not a declared page, if ordinals are not 1-based and
    /// contiguous per page, or if a **measured box falls outside its page**.
    pub fn seal(
        payload: RepresentationPayload,
        geometry: Vec<NodeGeometry>,
    ) -> Result<Self, EngineError> {
        let fingerprint = payload.fingerprint()?;
        let d = Self {
            artifact_type: REPRESENTATION_ARTIFACT_TYPE.to_string(),
            schema_version: REPRESENTATION_SCHEMA_VERSION.to_string(),
            representation: payload,
            representation_c14n_sha256: fingerprint,
            geometry,
        };
        d.check_structure()?;
        Ok(d)
    }

    fn malformed(detail: String) -> EngineError {
        EngineError::Malformed {
            what: "document representation".into(),
            detail,
        }
    }

    /// Every structural invariant, in one place, run on both construction paths.
    fn check_structure(&self) -> Result<(), EngineError> {
        let p = &self.representation;

        if self.artifact_type != REPRESENTATION_ARTIFACT_TYPE
            || self.schema_version != REPRESENTATION_SCHEMA_VERSION
        {
            return Err(Self::malformed(format!(
                "unrecognised artifact `{}` version `{}`; this build reads only \
                 `{REPRESENTATION_ARTIFACT_TYPE}` `{REPRESENTATION_SCHEMA_VERSION}`. Refusing \
                 rather than best-effort parsing an unknown shape.",
                self.artifact_type, self.schema_version
            )));
        }
        // The top-level tags exist so a reader can fail closed BEFORE parsing a payload. They
        // are a mirror of the payload's own identity, and a mirror that can disagree is worse
        // than no mirror.
        if p.identity.artifact_type != self.artifact_type
            || p.identity.schema_version != self.schema_version
        {
            return Err(Self::malformed(
                "the top-level artifact type/version disagree with the payload's identity".into(),
            ));
        }

        if self.geometry.len() != p.nodes.len() {
            return Err(Self::malformed(format!(
                "{} geometry rows for {} nodes; every node needs exactly one row, because a \
                 missing row would make absence ambiguous with omission",
                self.geometry.len(),
                p.nodes.len()
            )));
        }

        let mut pages_by_id = std::collections::BTreeMap::new();
        for page in &p.pages {
            if pages_by_id.insert(page.id.as_str(), page).is_some() {
                return Err(Self::malformed(format!("duplicate page id `{}`", page.id)));
            }
        }

        let mut seen_nodes = std::collections::BTreeSet::new();
        let mut ordinal_by_page: std::collections::BTreeMap<&str, u32> = Default::default();

        for (i, node) in p.nodes.iter().enumerate() {
            if !seen_nodes.insert(node.id.as_str()) {
                return Err(Self::malformed(format!("duplicate node id `{}`", node.id)));
            }
            if self.geometry[i].node != node.id {
                return Err(Self::malformed(format!(
                    "geometry row {i} describes `{}` but node {i} is `{}`; the sidecar is \
                     index-aligned with the node list",
                    self.geometry[i].node, node.id
                )));
            }
            let Some(page) = pages_by_id.get(node.parent.as_str()) else {
                return Err(Self::malformed(format!(
                    "node `{}` names parent page `{}`, which is not a declared page",
                    node.id, node.parent
                )));
            };
            let next = ordinal_by_page.entry(node.parent.as_str()).or_insert(0);
            *next += 1;
            if node.ordinal != *next {
                return Err(Self::malformed(format!(
                    "node `{}` has ordinal {} where {} was expected; ordinals are 1-based and \
                     contiguous within a page, in reading order",
                    node.id, node.ordinal, *next
                )));
            }

            if let GeometryPresence::Measured(r) = self.geometry[i].presence {
                Self::check_box_within_page(&node.id, r, page)?;
            }
        }

        self.check_geometry_matches_its_declaration()?;

        Ok(())
    }

    /// The sidecar and the payload's own declaration must agree about what is groundable.
    ///
    /// **This is the seam the fingerprint cannot cover.** Geometry sits outside the digest by
    /// §4, deliberately — but the payload declares, under
    /// `geometry-absent-not-groundable`, how many nodes cannot be projected. Without this check
    /// the two can be made to contradict each other while `verify_fingerprint` still passes:
    ///
    /// - flip a row `Measured` → `Absent` and the node silently vanishes from any projection
    ///   while the record declares no gap at all;
    /// - flip one `Absent` → `Measured` and a **fabricated box** is emitted while the record
    ///   still says that node has no measurable ink.
    ///
    /// Binding the two costs nothing and closes both. What it deliberately does **not** do is
    /// authenticate box *values* — §4's rationale is that boxes are the least reliable number in
    /// the artifact and identity must not be hostage to them. So the honest statement of the
    /// trust boundary, which `NodeGeometry` and the CLI both repeat, is: a verified
    /// representation attests to the text, the order and the origins. It does not attest to the
    /// rectangles.
    fn check_geometry_matches_its_declaration(&self) -> Result<(), EngineError> {
        let not_groundable = self.nodes_not_groundable();
        let declared = self
            .representation
            .assurance
            .limitations
            .iter()
            .any(|l| l.code == crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE);

        if (not_groundable > 0) != declared {
            return Err(Self::malformed(format!(
                "{not_groundable} node(s) have no measurable ink box, but the payload {} declare                  `{}`. The geometry sidecar sits outside the fingerprint by design, so this                  agreement is what stops a record and its own projection contradicting each other.",
                if declared { "does" } else { "does not" },
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
            )));
        }
        Ok(())
    }

    /// A measured box must lie inside its page. **Hard error, never a clamp.**
    ///
    /// Reachable in practice, not theoretical: an ink box is `baseline_y - ascent × size` in a
    /// top-left system, so a run whose baseline sits within one ascent of the page top produces
    /// a negative `y0`, and `QRect::new` accepts it — it validates ordering and magnitude, not
    /// page containment. The grounding schema requires `x0, y0 >= 0`, and Ethos additionally
    /// requires `x1 <= page.width && y1 <= page.height`, so such a box is un-emittable.
    ///
    /// Three responses were available and two are forbidden. **Clamping fabricates** a
    /// coordinate the document does not contain (Workbench rule 3). **Omitting** it would have
    /// to travel the geometry-omission path, and that path takes a typed absence by construction
    /// — an out-of-page *measured* box is not an absence, and routing it through would be
    /// exactly the "omit for a non-geometry reason" hole §11 forbids. So the box is refused, and
    /// loudly: it means the measurement or the coordinate transform is wrong, and that is worth
    /// finding rather than hiding behind a plausible rectangle.
    fn check_box_within_page(
        node: &NodeId,
        r: QRect,
        page: &PageRecord,
    ) -> Result<(), EngineError> {
        if r.x0() < 0 || r.y0() < 0 || r.x1() > page.width || r.y1() > page.height {
            return Err(Self::malformed(format!(
                "node `{node}` has a measured box [{}, {}, {}, {}] outside its page \
                 [0, 0, {}, {}]. A box is not clamped to fit and not silently dropped: this \
                 means the measurement or the coordinate transform is wrong.",
                r.x0(),
                r.y0(),
                r.x1(),
                r.y1(),
                page.width,
                page.height
            )));
        }
        Ok(())
    }

    /// The payload the fingerprint covers.
    pub fn payload(&self) -> &RepresentationPayload {
        &self.representation
    }

    /// The declared fingerprint.
    pub fn fingerprint(&self) -> &Sha256Hex {
        &self.representation_c14n_sha256
    }

    /// Geometry rows, index-aligned with `payload().nodes`.
    pub fn geometry(&self) -> &[NodeGeometry] {
        &self.geometry
    }

    /// Geometry for the node at `index` in `payload().nodes`.
    pub fn geometry_at(&self, index: usize) -> Option<GeometryPresence> {
        self.geometry.get(index).map(|g| g.presence)
    }

    /// How many nodes cannot be projected into a grounding artifact.
    ///
    /// Keyed to [`GeometryPresence::is_groundable`] — the **same predicate the omission uses** —
    /// rather than to one absence variant. A count derived from a different question than the
    /// one the adapter asks is a count that goes wrong the first time a second absence variant
    /// occurs.
    pub fn nodes_not_groundable(&self) -> u32 {
        self.geometry
            .iter()
            .filter(|g| !g.presence.is_groundable())
            .count() as u32
    }

    /// Recompute the fingerprint from the payload as it stands.
    ///
    /// # Errors
    ///
    /// Propagates canonicalization failure.
    pub fn recompute_fingerprint(&self) -> Result<Sha256Hex, EngineError> {
        self.representation.fingerprint()
    }

    /// Check the declared fingerprint against the payload.
    ///
    /// Deliberately **not** run by `Deserialize`. `grounding-check` (M6) must be able to load an
    /// artifact and then *report* on it — Ethos's own report distinguishes `structure` from
    /// `source_binding` — and a validator that refuses to parse what it is judging can only ever
    /// say "invalid JSON".
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] naming **both** digests, so the failure is diagnosable rather
    /// than merely detected.
    pub fn verify_fingerprint(&self) -> Result<(), EngineError> {
        let actual = self.recompute_fingerprint()?;
        if actual != self.representation_c14n_sha256 {
            return Err(Self::malformed(format!(
                "declared representation_c14n_sha256 is {} but the payload hashes to {}",
                self.representation_c14n_sha256, actual
            )));
        }
        Ok(())
    }

    /// Canonical bytes of the whole artifact.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        let value = serde_json::to_value(self).map_err(|e| EngineError::Malformed {
            what: "representation".into(),
            detail: e.to_string(),
        })?;
        c14n_bytes(&value).map_err(|e| EngineError::Malformed {
            what: "representation".into(),
            detail: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assurance::{PageState, PageStateEntry};
    use crate::derivation::GeometryAbsence;
    use crate::ids::{IdAllocator, IdKind};
    use crate::profile::{Capabilities, Profile};

    fn payload(nodes: Vec<Node>, pages: Vec<PageRecord>) -> RepresentationPayload {
        let profile = Profile::default();
        // Authorize every page up to the highest one present, and mark the ones this
        // representation does not carry as never attempted — the shape a budgeted run really
        // produces, rather than one where the page list and the coverage disagree.
        let authorized = pages.iter().map(|p| p.index).max().unwrap_or(0);
        let states: Vec<PageStateEntry> = (1..=authorized)
            .map(|index| PageStateEntry {
                index,
                state: if pages.iter().any(|p| p.index == index) {
                    PageState::Processed
                } else {
                    PageState::NotAttempted(crate::assurance::codes::RESOURCE_LIMIT_PAGES.into())
                },
            })
            .collect();
        RepresentationPayload {
            identity: ArtifactIdentity {
                artifact_type: REPRESENTATION_ARTIFACT_TYPE.into(),
                schema_version: REPRESENTATION_SCHEMA_VERSION.into(),
                parser_version: profile.parser_version.clone(),
                profile_sha256: profile.profile_sha256().unwrap(),
            },
            source: SourceIdentity {
                media_type: "application/pdf".into(),
                sha256: Sha256Hex::from_hex(&sha256_hex_bytes(b"x")).unwrap(),
            },
            processing_run: ProcessingRun {
                processor: ProcessorIdentity {
                    name: "ethos-engine".into(),
                    version: profile.parser_version.clone(),
                    backend: "lopdf 0.44.0".into(),
                },
                reading_order_rule: profile.reading_order_rule.clone(),
            },
            coordinate_system: CoordinateSystem::V0,
            pages,
            nodes,
            tables: Vec::new(),
            assurance: Assurance::new(
                Capabilities::V0,
                authorized,
                states,
                vec![crate::assurance::Limitation::document(
                    crate::assurance::codes::RESOURCE_LIMIT_PAGES,
                    "test fixture: pages this representation does not carry",
                )],
            )
            .unwrap(),
        }
    }

    fn page(alloc: &mut IdAllocator, index: u32) -> PageRecord {
        PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index,
            width: 30000,
            height: 14400,
            rotation: 0,
        }
    }

    fn node(alloc: &mut IdAllocator, parent: &NodeId, ordinal: u32, page: u32) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: parent.clone(),
            ordinal,
            text: "hello".into(),
            native_locator: NativeLocator::Pdf(PdfLocator {
                page,
                origin_x: 7200,
                origin_y: 7200,
                advance: Some(1000),
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: TextRunAttributes {
                char_codes: vec![0x68, 0x65, 0x6c, 0x6c, 0x6f],
                scalar_code_mismatch: false,
                synthesized: Vec::new(),
                font_id: "F1".into(),
                font_size: 2400,
            },
        }
    }

    fn geom(node: &Node, presence: GeometryPresence) -> NodeGeometry {
        NodeGeometry {
            node: node.id.clone(),
            presence,
        }
    }

    /// Seal, declaring the geometry omission when the sidecar actually has one.
    ///
    /// The declaration is not optional garnish: `check_structure` binds the sidecar to it, so a
    /// helper that skipped it would make every absent-geometry test fail for the wrong reason.
    fn seal(
        nodes: Vec<Node>,
        pages: Vec<PageRecord>,
        geometry: Vec<NodeGeometry>,
    ) -> Result<DocumentRepresentation, EngineError> {
        let mut p = payload(nodes, pages);
        if geometry.iter().any(|g| !g.presence.is_groundable()) {
            let mut limitations = p.assurance.limitations.clone();
            limitations.push(crate::assurance::Limitation::document(
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
                "test fixture: nodes with no measurable ink box",
            ));
            p.assurance = Assurance::new(
                p.assurance.capabilities,
                p.assurance.coverage.pages_authorized,
                p.assurance.page_states.clone(),
                limitations,
            )
            .unwrap();
        }
        DocumentRepresentation::seal(p, geometry)
    }

    fn simple() -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
        );
        seal(vec![n], vec![p], vec![g]).unwrap()
    }

    #[test]
    fn the_fingerprint_is_the_digest_of_the_representation_subtree() {
        let d = simple();
        let v = serde_json::to_value(&d).unwrap();

        // The reader algorithm, written out exactly as an external implementer would do it:
        // canonicalize the `representation` member, hash it, prefix with `sha256:`.
        let subtree = c14n_bytes(&v["representation"]).unwrap();
        let expected = format!("sha256:{}", sha256_hex_bytes(&subtree));

        assert_eq!(v["representation_c14n_sha256"].as_str().unwrap(), expected);
        assert_eq!(d.fingerprint().to_string(), expected);
        d.verify_fingerprint().unwrap();
    }

    #[test]
    fn geometry_is_outside_the_fingerprint() {
        let mut a = simple();
        let before = a.fingerprint().clone();

        // Mutate the one measured box. The fingerprint must not move — `01-CONTRACT.md` §4
        // excludes geometry deliberately, because boxes are the least reliable number in the
        // artifact and identity must not be hostage to them.
        let measured = a
            .geometry
            .iter()
            .filter(|g| matches!(g.presence, GeometryPresence::Measured(_)))
            .count();
        assert!(
            measured > 0,
            "this test is vacuous without a measured box to mutate"
        );
        a.geometry[0].presence = GeometryPresence::Measured(QRect::new(1, 1, 99, 99).unwrap());

        assert_eq!(
            a.recompute_fingerprint().unwrap(),
            before,
            "moving a box must not move the fingerprint"
        );
    }

    #[test]
    fn changing_a_text_origin_does_move_the_fingerprint() {
        // The mirror of the test above, and the reason it is safe: origins ARE identity, so the
        // exclusion must be narrow. Without this, "geometry is excluded" could silently become
        // "locators are excluded" and nobody would notice.
        let mut d = simple();
        let before = d.fingerprint().clone();
        d.representation.nodes[0].native_locator = NativeLocator::Pdf(PdfLocator {
            page: 1,
            origin_x: 7201,
            origin_y: 7200,
            advance: Some(1000),
        });
        assert_ne!(d.recompute_fingerprint().unwrap(), before);
    }

    #[test]
    fn a_tampered_payload_is_detected_by_verify_but_still_parses() {
        let mut d = simple();
        d.representation.nodes[0].text = "goodbye".into();
        let err = d.verify_fingerprint().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(d.fingerprint().as_str()),
            "the failure must name both digests: {msg}"
        );
    }

    #[test]
    fn a_measured_box_outside_its_page_is_refused() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);

        // Negative y0: the reachable case — a baseline within one ascent of the page top.
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, -10, 100, 100).unwrap()),
        );
        let err = DocumentRepresentation::seal(payload(vec![n.clone()], vec![p.clone()]), vec![g])
            .unwrap_err();
        assert!(err.to_string().contains("outside its page"), "{err}");

        // Overrunning the right margin.
        let g = geom(
            &n,
            GeometryPresence::Measured(QRect::new(0, 0, 30001, 100).unwrap()),
        );
        assert!(seal(vec![n], vec![p], vec![g]).is_err());
    }

    #[test]
    fn typed_absence_is_never_refused_for_being_outside_a_page() {
        // Absence has no coordinates to be outside anything. Guards against a future
        // containment check that reaches for a box that is not there.
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        let d = seal(vec![n], vec![p], vec![g]).unwrap();
        assert_eq!(d.nodes_not_groundable(), 1);
    }

    /// Both flip attacks across the fingerprint seam.
    ///
    /// The sidecar is outside the digest by §4, so these edits do not move it. Without the
    /// declaration cross-check, each produces a record that is fingerprint-valid and flatly
    /// contradicts its own projection.
    #[test]
    fn a_sidecar_that_contradicts_its_declaration_is_refused() {
        // Measured -> Absent: the node would vanish from any projection while the record
        // declares no gap at all.
        let mut d = simple();
        assert!(d.verify_fingerprint().is_ok());
        d.geometry[0].presence = GeometryPresence::Absent(GeometryAbsence::NotReportedByReader);
        assert!(
            d.verify_fingerprint().is_ok(),
            "the premise: geometry is outside the digest, so this edit is invisible to it"
        );
        assert!(
            d.check_structure().is_err(),
            "a node that became ungroundable with no declaration must be refused"
        );

        // Absent -> Measured: a FABRICATED box would be emitted while the record still says the
        // node has no measurable ink.
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let pg = page(&mut alloc, 1);
        let n = node(&mut alloc, &pg.id, 1, 1);
        let mut d = seal(
            vec![n.clone()],
            vec![pg],
            vec![geom(
                &n,
                GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
            )],
        )
        .unwrap();
        d.geometry[0].presence = GeometryPresence::Measured(QRect::new(1, 1, 10, 10).unwrap());
        assert!(d.verify_fingerprint().is_ok());
        assert!(
            d.check_structure().is_err(),
            "a fabricated box beside a standing omission declaration must be refused"
        );

        // And the refusal survives a round trip through the wire, not just in memory.
        let v = serde_json::to_value(&d).unwrap();
        assert!(serde_json::from_value::<DocumentRepresentation>(v).is_err());
    }

    #[test]
    fn a_misaligned_geometry_sidecar_is_refused() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let n = node(&mut alloc, &p.id, 1, 1);

        // Too few rows.
        assert!(DocumentRepresentation::seal(
            payload(vec![n.clone()], vec![p.clone()]),
            Vec::new()
        )
        .is_err());

        // Right count, wrong node.
        let wrong = NodeGeometry {
            node: NodeId::from_parts(IdKind::Span, 99),
            presence: GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        };
        assert!(seal(vec![n], vec![p], vec![wrong]).is_err());
    }

    #[test]
    fn a_node_whose_parent_is_not_a_page_is_refused() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let mut n = node(&mut alloc, &p.id, 1, 1);
        n.parent = NodeId::from_parts(IdKind::Page, 77);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        let err = seal(vec![n], vec![p], vec![g]).unwrap_err();
        assert!(err.to_string().contains("not a declared page"), "{err}");
    }

    #[test]
    fn ordinals_must_be_one_based_and_contiguous_within_a_page() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let p = page(&mut alloc, 1);
        let mut n = node(&mut alloc, &p.id, 1, 1);
        n.ordinal = 2; // skips 1
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        assert!(seal(vec![n], vec![p], vec![g]).is_err());
    }

    #[test]
    fn a_page_index_is_the_documents_number_not_the_array_position() {
        // The distinction that matters the moment a page is quarantined: the second processed
        // page of a document may be page 7, and a citation renders against 7.
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let mut p = page(&mut alloc, 7);
        p.index = 7;
        let n = node(&mut alloc, &p.id, 1, 7);
        let g = geom(
            &n,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
        );
        let d = seal(vec![n], vec![p], vec![g]).unwrap();
        assert_eq!(d.payload().pages[0].index, 7);
        match &d.payload().nodes[0].native_locator {
            NativeLocator::Pdf(l) => assert_eq!(l.page, 7),
        }
    }

    #[test]
    fn an_unknown_artifact_type_is_refused_rather_than_best_effort_parsed() {
        let d = simple();
        let mut v = serde_json::to_value(&d).unwrap();
        v["artifact_type"] = serde_json::Value::String("ethos.engine.something.v9".into());
        let parsed: Result<DocumentRepresentation, _> = serde_json::from_value(v);
        assert!(
            parsed.is_err(),
            "§8: never best-effort parse an unknown shape"
        );
    }

    #[test]
    fn the_top_level_tags_cannot_disagree_with_the_payload() {
        let d = simple();
        let mut v = serde_json::to_value(&d).unwrap();
        v["representation"]["identity"]["schema_version"] =
            serde_json::Value::String("9.9.9".into());
        let parsed: Result<DocumentRepresentation, _> = serde_json::from_value(v);
        assert!(
            parsed.is_err(),
            "a mirror that can disagree is worse than no mirror"
        );
    }

    #[test]
    fn the_artifact_round_trips_through_c14n() {
        let d = simple();
        let bytes = d.to_canonical_bytes().unwrap();
        let back: DocumentRepresentation = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, d);
        assert_eq!(back.fingerprint(), d.fingerprint());
    }

    #[test]
    fn an_unknown_field_anywhere_fails_closed() {
        let d = simple();
        for pointer in [
            "".to_string(),
            "/representation".into(),
            "/representation/source".into(),
            "/representation/processing_run".into(),
            "/representation/processing_run/processor".into(),
        ] {
            let mut v = serde_json::to_value(&d).unwrap();
            let target = if pointer.is_empty() {
                &mut v
            } else {
                v.pointer_mut(&pointer).expect("pointer resolves")
            };
            target
                .as_object_mut()
                .expect("object")
                .insert("future_knob".into(), serde_json::Value::from(1));
            let parsed: Result<DocumentRepresentation, _> = serde_json::from_value(v);
            assert!(
                parsed.is_err(),
                "unknown field at `{pointer}` must be refused"
            );
        }
    }

    #[test]
    fn the_node_kind_wire_spelling_matches_the_grounding_id_pattern() {
        // `ethos.grounding.v1` constrains `kind` to ^[a-z0-9][a-z0-9_-]*$. The representation's
        // spelling becomes the grounding artifact's `kind` verbatim, so it has to satisfy that
        // here rather than be rewritten at the boundary.
        let k = NodeKind::TextRun.as_str();
        assert_eq!(k, "text_run");
        let mut chars = k.chars();
        let first = chars.next().unwrap();
        assert!(first.is_ascii_lowercase() || first.is_ascii_digit());
        assert!(chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'));
    }

    #[test]
    fn no_float_reaches_the_wire() {
        simple().to_canonical_bytes().expect("integers only");
    }
}
