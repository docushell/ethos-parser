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

//! `ethos-parser-grounding` — `DocumentRepresentation v0` → `ethos.grounding.v1`.
//!
//! **The representation is the record; this is a projection of it.** The projection is lossy by
//! design — it carries no derivation class, no `mcid`, no synthesized flags, no coverage state,
//! no char codes — because it is a verifier's *input*, not the canonical record.
//! `lossiness_is_asserted_not_assumed` enumerates exactly what disappears, so nobody later
//! mistakes a grounding round-trip for proof the representation is intact.
//!
//! # Two boundaries, both enforced by tests rather than by review
//!
//! 1. **No PDF concept.** This crate projects the representation, never the document. It does not
//!    import `lopdf` or `ethos-parser-pdf`, and it never so much as mentions `NativeLocator` — the
//!    projection addresses pages by node id, so it never needs to know what a page *is* in any
//!    format. That is what keeps the second format a variant instead of a rewrite.
//! 2. **No verification.** No claim, no verdict, no `grounded`, no `evidence_tier`. M5 projects;
//!    `grounding-check` is M6 and validates structure and source binding only.
//!
//! # The omission rule, and why it is a type
//!
//! `ethos.grounding.v1` requires a `bbox` on every element and span, while
//! `docs/01-CONTRACT.md` §5.2 says a node with no measurable font metrics gets **typed absence**.
//! The decision (§11) is: keep the node in the representation, **omit it here, count it, and
//! declare it**.
//!
//! The dangerous version of that rule is one where "omit" can be reached from a quality
//! judgement — a classifier reason code, a page state, a boolean someone set. So the only
//! constructor for an emittable box, [`GroundedBox`], takes an [`ethos_parser_core::GeometryPresence`]
//! and builds
//! nothing from any other input. There is no `GroundedBox::new(x0, y0, x1, y1)`. A future edit
//! that wanted to omit a node "because the page looked bad" would have to add a constructor, and
//! adding one is a visible act.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod check;

pub use check::{
    grounding_check, Counts, ReportError, SourceBinding, Structure, ValidationReport,
    VALIDATION_ARTIFACT_TYPE, VALIDATION_SCHEMA_VERSION,
};

use serde::{Deserialize, Serialize};

use ethos_parser_core::{Capabilities, DocumentRepresentation, EngineError, PageRecord};

/// Artifact type. A const in the schema, so a const here.
pub const GROUNDING_ARTIFACT_TYPE: &str = "ethos.grounding.v1";

/// The one media type `ethos.grounding.v1` can name as its source.
///
/// The schema's own `{"const": "application/pdf"}`, mirrored here so [`project`] can refuse a
/// source this artifact has no shape for. **A media type is a string, not a format concept** —
/// nothing in this crate parses one, and the M5 line in `docs/04-ARCHITECTURE.md` is explicit
/// that a format name as data is permitted where machinery is not.
pub const SOURCE_MEDIA_TYPE: &str = "application/pdf";

/// Schema version emitted for a paginated (PDF) source — the shape M5 froze,
/// byte-identical ever since.
pub const GROUNDING_SCHEMA_VERSION: &str = "1.0.0";

/// Schema version emitted for a page-less source (0.39.0).
///
/// The revision `page_less_source.rs` recorded as *owned by Ethos, not refused*
/// was made on the Ethos side first: `ethos.grounding.v1` schema 1.1.0 admits
/// eight page-less media types whose elements carry the producer's native
/// locator string in place of the page/bbox pair, with `pages: []` — a
/// page-less source states no page, and synthesizing one is what
/// `docs/history/14-V2-SCOPE.md` §3 refuses. A PDF artifact keeps `1.0.0` and its
/// exact bytes.
pub const GROUNDING_SCHEMA_VERSION_PAGE_LESS: &str = "1.1.0";

/// The eight page-less media types schema 1.1.0 admits, mirrored from the
/// schema exactly as [`SOURCE_MEDIA_TYPE`] mirrors its PDF const.
pub const PAGE_LESS_MEDIA_TYPES: [&str; 8] = [
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/vnd.oasis.opendocument.text",
    "application/vnd.oasis.opendocument.spreadsheet",
    "application/vnd.oasis.opendocument.presentation",
    "application/rtf",
    "application/epub+zip",
];

/// The limitation code naming what the projection dropped.
///
/// The same code the representation declares, so the two halves of "omit and declare" name the
/// same thing and a consumer can join them.
pub const GEOMETRY_ABSENT_OMITTED: &str = "geometry-absent-not-groundable";

/// The box type, in its own module so the privacy is real.
///
/// **`project()` lives outside this module and therefore cannot construct a [`GroundedBox`]
/// either.** That is the whole point, and it is a correction: an earlier version put the type
/// beside the projection with only a private field, which stops *other crates* while leaving the
/// one function that matters free to build a box from anything — and the grep-style test meant to
/// cover the gap could be walked past by writing the new constructor in the obvious shape. The
/// module boundary is the compiler enforcing it instead.
mod grounded_box {
    use ethos_parser_core::{GeometryPresence, QRect};

    /// A box that is allowed onto the wire.
    ///
    /// The only way to obtain one is [`GroundedBox::from_presence`], which takes a
    /// [`GeometryPresence`]. There is no `GroundedBox::new(x0, y0, x1, y1)` and no way to write
    /// one outside this module, so the omission decision takes a **measurement state** by
    /// construction — never a boolean, never a classifier reason code, never a page state. That
    /// is what `docs/01-CONTRACT.md` §11 requires, and what `docs/history/05-MILESTONES.md` M5's review
    /// checklist asks a reviewer to grep for. There is nothing to grep: the type system says it.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GroundedBox(QRect);

    impl GroundedBox {
        /// The only way to obtain one.
        ///
        /// Returns `None` for every absence variant — "could not measure", "nothing to measure",
        /// "not asked to measure" — because the grounding schema can express none of them and a
        /// fabricated box would be indistinguishable from a measured one downstream.
        pub fn from_presence(presence: GeometryPresence) -> Option<Self> {
            match presence {
                GeometryPresence::Measured(r) => Some(Self(r)),
                GeometryPresence::Absent(_) => None,
            }
        }

        /// `[x0, y0, x1, y1]` in integer centipoints, exactly as measured.
        ///
        /// A pure projection of the rectangle it was built from. Nothing is recomputed here, so
        /// the emitted box is the measured box or the type was never built.
        pub fn to_array(self) -> [i64; 4] {
            [self.0.x0(), self.0.y0(), self.0.x1(), self.0.y1()]
        }
    }
}

pub use grounded_box::GroundedBox;

/// `{media_type, sha256}` — the source bytes this artifact speaks about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// `application/pdf` — a const in the schema.
    pub media_type: String,
    /// Digest of the exact source bytes, `sha256:<64 hex>`.
    ///
    /// **A plain `String` on purpose, and it is the checker that decides this.** Emission always
    /// builds it from a validated [`ethos_parser_core::Sha256Hex`], so nothing this engine writes can
    /// be malformed. Parsing is the other direction: Ethos reads this field as a string and
    /// reports a bad digest as `invalid_field` at **`/source`**. A self-validating type here
    /// would reject it earlier, during deserialization, and report path `/` — the same verdict
    /// at the wrong place, which is half an agreement. The strictness lives where it belongs, in
    /// `check::validate`, at Ethos's path.
    pub sha256: String,
}

/// `{name, version}` — who produced the artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    /// Engine name.
    pub name: String,
    /// Engine build.
    pub version: String,
}

/// The three booleans `ethos.grounding.v1` requires. A strict subset of the engine's own set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundingCapabilities {
    /// Spans are emitted.
    pub spans: bool,
    /// Spans carry character offsets into their element's text.
    pub char_offsets: bool,
    /// Tables are detected and emitted.
    pub tables: bool,
}

/// `{unit, origin}` — both consts in the schema, and declared anyway (§3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundingCoordinateSystem {
    /// `centipoint`.
    pub unit: String,
    /// `top-left`.
    pub origin: String,
}

/// One page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    /// Page id, referenced by elements and spans.
    pub id: String,
    /// **1-based**, the document's own page number.
    pub index: u32,
    /// Width in integer centipoints, after rotation.
    pub width: i64,
    /// Height in integer centipoints, after rotation.
    pub height: i64,
    /// 0, 90, 180 or 270.
    ///
    /// **`u16`, matching Ethos's own field type**, so a negative rotation is refused during
    /// deserialization — `invalid_field` at `/` — exactly as the verifier refuses it. Typing it
    /// `i64` and range-checking later reached the same verdict at a different path, which the
    /// oracle compares.
    pub rotation: u16,
}

/// One citable element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Element {
    /// Element id.
    pub id: String,
    /// The **page's id**, not its index — the schema models this as an id reference.
    ///
    /// `Some` on every paginated element; `None` on a page-less one (schema
    /// 1.1.0), where there is no page id to give and the locator below is the
    /// address instead. Skipped when absent, so paginated artifacts keep their
    /// exact 1.0.0 bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// `[x0, y0, x1, y1]`. `Some` on every paginated element; `None` page-less.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[i64; 4]>,
    /// `^[a-z0-9][a-z0-9_-]*$`.
    pub kind: String,
    /// The element's text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The producer's native locator, canonically serialized (page-less only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
}

/// One text span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    /// Span id.
    pub id: String,
    /// The page's id.
    pub page: String,
    /// `[x0, y0, x1, y1]`.
    pub bbox: [i64; 4],
    /// The span's text.
    pub text: String,
    /// The element this span sits inside.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
    /// Start offset in **Unicode scalars** into the element's text.
    ///
    /// Absent at v0, and that is not an oversight — see [`project`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub char_start: Option<u32>,
    /// End offset in Unicode scalars, exclusive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub char_end: Option<u32>,
}

/// The `ethos.grounding.v1` artifact.
///
/// Field-for-field with the schema, which is `additionalProperties: false` throughout — so
/// `deny_unknown_fields` here is the same rule read from the other side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroundingSource {
    /// const `ethos.grounding.v1`.
    pub artifact_type: String,
    /// const `1.0.0`.
    pub schema_version: String,
    /// The source bytes.
    pub source: Source,
    /// Who produced it.
    pub producer: Producer,
    /// The three declared booleans.
    pub capabilities: GroundingCapabilities,
    /// The declared coordinate system.
    pub coordinate_system: GroundingCoordinateSystem,
    /// Pages.
    pub pages: Vec<Page>,
    /// Elements. Required, and legitimately empty when nothing could be grounded.
    pub elements: Vec<Element>,
    /// Spans. **Present iff `capabilities.spans`** — a rule the consuming verifier enforces, so
    /// an empty array and an absent key mean different things and only one of them is legal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<Span>>,
    /// Tables. **Absent iff `!capabilities.tables`**. An empty array here is not "no tables
    /// found", it is a claim to have looked — and since v1-S2 that claim covers both the ruled
    /// and the unruled rule.
    ///
    /// Which rule found a given table is **not** carried here. `ethos.grounding.v1` is
    /// `additionalProperties: false` and this crate's copy of its schema is byte-pinned against
    /// Ethos's, so the projection stays a move of cells, boxes and text. The rule id lives on the
    /// representation's `TableRecord::detection_rule`, which is where a consumer that needs it
    /// looks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables: Option<Vec<Table>>,
}

/// A table, as `ethos.grounding.v1` carries one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    /// Table id.
    pub id: String,
    /// The page's id.
    pub page: String,
    /// `[x0, y0, x1, y1]`.
    pub bbox: [i64; 4],
    /// Cells.
    pub cells: Vec<Cell>,
}

/// One table cell, as `ethos.grounding.v1` carries one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    /// Zero-indexed row.
    pub row: u32,
    /// Zero-indexed column.
    pub col: u32,
    /// 1 means not merged.
    pub row_span: u32,
    /// 1 means not merged.
    pub col_span: u32,
    /// `[x0, y0, x1, y1]`.
    pub bbox: [i64; 4],
    /// Cell text.
    pub text: String,
}

/// What the projection dropped, and why.
///
/// Returned beside the artifact rather than inside it: the schema is
/// `additionalProperties: false`, so the artifact physically cannot carry this. The record it was
/// projected from declares the same count under the same code, which is where a consumer holding
/// only the artifact goes to find out what is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OmissionReport {
    /// Nodes in the representation.
    pub nodes_total: u32,
    /// Nodes with no measurable box, therefore absent from `elements` and `spans`.
    pub nodes_omitted: u32,
    /// The limitation code naming the omission.
    pub limitation_code: &'static str,
}

impl OmissionReport {
    /// Whether anything was dropped.
    pub fn is_lossy(&self) -> bool {
        self.nodes_omitted > 0
    }
}

/// A projection and its omission report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    /// The artifact.
    pub source: GroundingSource,
    /// What it could not carry.
    pub omission: OmissionReport,
}

/// Project a representation into `ethos.grounding.v1`.
///
/// # What maps to what
///
/// | Representation | Grounding |
/// | --- | --- |
/// | [`PageRecord`] | `pages[]`, keyed by the page's own id |
/// | Node with a measurable box | one `elements[]` entry **and** one `spans[]` entry referencing it |
/// | Node without one | omitted from both, counted in [`OmissionReport`] |
///
/// **Why a node becomes both an element and a span.** The schema offers two granularities —
/// coarse citable elements and finer spans inside them — and v0 performs no line or block
/// grouping, so the two coincide: a run *is* the element and *is* the span. Emitting only spans
/// would leave `elements` empty on a document full of text, and an empty required array reads as
/// "nothing here" — the exact absence-as-evidence misreading this project refuses. When grouping
/// lands at v1 the element becomes the block and the span stays the run, and this shape is
/// already the right one.
///
/// **Why `char_offsets` stays false.** Ethos's own validator ties the capability to the fields:
/// with `char_offsets: true` every span must carry complete offsets that index its element's
/// text, and with it false no span may carry any. Since v0's element and span are the same
/// object, an offset would always be `0..len` and would advertise sub-element addressing the
/// engine cannot actually do. It flips at v1 with grouping, when the offsets start carrying
/// information.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a node's parent page is not declared, or if the representation
/// claims a capability this projection cannot honestly express.
pub fn project(repr: &DocumentRepresentation) -> Result<Projection, EngineError> {
    // **`ethos.grounding.v1` is a PDF artifact, and v2-S1 decided it stays one.** Its schema says
    // `source.media_type` is `{"const": "application/pdf"}`, every element requires a `page` and a
    // `bbox`, and every page requires integer geometry — none of which a page-less source has.
    //
    // Refused **here, by name**, rather than left to fail further down on a parent lookup that
    // would report "not a declared page" about a document that has no pages. And refused on the
    // media type rather than on the locator, because this crate never reads a locator: the check
    // is this projection enforcing its own output contract, not learning what a second format is.
    if repr.payload().source.media_type != SOURCE_MEDIA_TYPE {
        // The page-less shape (schema 1.1.0), which exists on the Ethos side
        // since the revision this branch used to record as owned-elsewhere.
        return project_page_less(repr);
    }

    let payload = repr.payload();
    let caps = payload.assurance.capabilities;

    let pages: Vec<Page> = payload.pages.iter().map(page_of).collect();

    let mut elements = Vec::new();
    let mut spans = Vec::new();
    let mut omitted = 0u32;
    let mut element_ordinal = 0u32;

    for (i, node) in payload.nodes.iter().enumerate() {
        let presence = repr
            .geometry_at(i)
            .ok_or_else(|| malformed(format!("node {i} has no geometry row")))?;

        // The omission decision, and the only one. It takes a measurement state; there is no
        // overload that takes anything else.
        let Some(bbox) = GroundedBox::from_presence(presence) else {
            omitted += 1;
            continue;
        };

        let page_id = node.parent.as_str().to_string();
        if !payload.pages.iter().any(|p| p.id == node.parent) {
            return Err(malformed(format!(
                "node `{}` names parent page `{page_id}`, which is not a declared page",
                node.id
            )));
        }

        // The span keeps the representation node's own id, so a consumer holding only a
        // grounding artifact can join a span back to the node it came from without a mapping
        // table. Elements get their own counter because they are a different granularity —
        // one that stops being 1:1 with spans the moment line grouping lands at v1.
        element_ordinal += 1;
        let element_id = format!("e{element_ordinal}");
        elements.push(Element {
            id: element_id.clone(),
            page: Some(page_id.clone()),
            bbox: Some(bbox.to_array()),
            kind: node.kind.as_str().to_string(),
            text: Some(node.text.clone()),
            locator: None,
        });
        spans.push(Span {
            id: node.id.as_str().to_string(),
            page: page_id,
            bbox: bbox.to_array(),
            text: node.text.clone(),
            element: Some(element_id),
            // Tied to the capability by the consuming validator: `offsets_present` must equal
            // `capabilities.char_offsets`, so these stay absent exactly while it is false.
            char_start: None,
            char_end: None,
        });
    }

    // Exhaustiveness gate. A capability added to the engine's set without a decision about what
    // the projection does with it fails to compile here rather than being waved through.
    let Capabilities {
        spans: spans_claimed,
        char_offsets,
        tables,
        measured_ink_boxes: _,
        multi_column_reading_order: _,
        structural_locators: _,
        // v1-S4. Both deliberately do nothing here, and the decision is the point.
        //
        // `ethos.grounding.v1` has `elements` and `spans` and nothing else, and every one of them
        // carries a `bbox` that means **measured ink**. A form field's value and an annotation's
        // comment are neither: their rectangles are numbers the author wrote into a dictionary
        // saying where a widget sits, and projecting them would put declared rectangles and
        // measured ones under one key with nothing on the wire to tell them apart.
        //
        // So those nodes are omitted, counted, and declared in the representation as
        // `non-text-nodes-not-projected`. The gap is in what the target schema can express, not
        // in what was read: the nodes are all still in the record, with their text, their object
        // ids and their rectangles intact.
        form_fields: _,
        annotations: _,
        // v1-S6. Also deliberately nothing, and for a **third** reason rather than the same one.
        //
        // An image node's rectangle is not a declared `/Rect` and not measured ink: it is the
        // page's own transformation matrix applied to the unit square. `ethos.grounding.v1` has
        // one kind of box and it means measured ink, so a painted rectangle projected as a `bbox`
        // would be the third provenance flattened into the first — the distinction v1-S4 refused
        // to flatten, refused again here.
        //
        // Omitted, counted, and declared in the representation. The node keeps its object number,
        // its digest and its rectangle in the record, which is where a consumer that needs them
        // goes.
        images: _,
        // Nothing to project: no raster is produced at all under this profile.
        page_screenshots: _,
        // v1.1-S1. Nothing here either, and for the plainest reason of the set: a Markdown
        // projection is a SIBLING of this one, not an input to it. Both read the same
        // representation and neither reads the other — `ethos.grounding.v1` carries boxes for a
        // verifier, `ethos.markdown.v1` carries text and an anchor map for a retriever.
        //
        // Keeping them apart is the point rather than an omission. A grounding artifact that
        // embedded Markdown would put a projection inside the record a citation binds to, which
        // is precisely the layering Workbench rule 8 warns about.
        markdown: _,
        // v1.1-S4. Nothing, for the same reason as `markdown` directly above: HTML is a THIRD
        // sibling projection of the same representation, not an input to this one. That there
        // are now two of them is exactly why none of them belongs inside the record a citation
        // binds to.
        html: _,
    } = caps;

    if char_offsets {
        return Err(malformed(
            "the profile claims char_offsets, which this projection does not emit. Ethos's \
             validator requires every span to carry complete, matching offsets when the \
             capability is true, so emitting the claim without the fields would produce an \
             artifact the verifier rejects."
                .into(),
        ));
    }
    // v1-S1: the claim is now honoured rather than refused. `tables: true` means the detector
    // looked, so the array is present — **possibly empty**, which is the artifact saying it
    // looked and found none. `None` and `Some(vec![])` are different artifacts and the consuming
    // validator treats the difference as meaning what it says.
    // v2-S24. A table with no measured box cannot enter this projection: `ethos.grounding.v1`
    // requires a `bbox` on every table and cell, and fabricating one is forbidden. A **tagged**
    // table is exactly that case — its grid is the document's tags and it carries no geometry — so
    // it is OMITTED here, the same omit-plus-count-plus-declare the representation applies to text
    // runs with no ink box. The count is not lost: the representation carries
    // `tagged-table-without-geometric-table`, and this schema is `additionalProperties: false` and
    // cannot hold a limitation list, so a consumer comes back to the record to find what is
    // missing. `filter_map` drops a table whose own box OR any cell's box is absent — the tagged
    // case has both absent, and a geometric table has both measured, so this only ever drops the
    // tagged tables and leaves every geometric one byte-identical.
    let projected_tables: Vec<Table> = payload
        .tables
        .iter()
        .filter_map(|t| {
            let table_bbox = t.geometry.measured()?;
            let cells: Vec<Cell> = t
                .cells
                .iter()
                .map(|c| {
                    c.geometry.measured().map(|cell_bbox| Cell {
                        row: c.position.row,
                        col: c.position.column,
                        row_span: c.position.rowspan,
                        col_span: c.position.colspan,
                        bbox: cell_bbox.to_array(),
                        // The concatenation the record already holds. Not recomputed here: two
                        // places deriving the same text is two places for it to drift.
                        text: c.text.clone(),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Table {
                id: t.id.as_str().to_string(),
                page: t.page.as_str().to_string(),
                bbox: table_bbox.to_array(),
                cells,
            })
        })
        .collect();

    Ok(Projection {
        source: GroundingSource {
            artifact_type: GROUNDING_ARTIFACT_TYPE.to_string(),
            schema_version: GROUNDING_SCHEMA_VERSION.to_string(),
            source: Source {
                media_type: payload.source.media_type.clone(),
                // From a validated `Sha256Hex`, so the wire string is well formed by
                // construction even though the field itself does not enforce it.
                sha256: payload.source.sha256.to_string(),
            },
            producer: Producer {
                name: payload.processing_run.processor.name.clone(),
                version: payload.processing_run.processor.version.clone(),
            },
            capabilities: GroundingCapabilities {
                spans: spans_claimed,
                char_offsets,
                tables,
            },
            coordinate_system: GroundingCoordinateSystem {
                unit: "centipoint".to_string(),
                origin: "top-left".to_string(),
            },
            pages,
            elements,
            // Present iff claimed. `None` and `Some(vec![])` are different artifacts, and the
            // consuming validator treats the mismatch as an error rather than a nicety.
            spans: if spans_claimed { Some(spans) } else { None },
            // Present iff claimed, exactly like `spans`. From v1-S1 the capability is true, so
            // this is `Some` — and an empty vec is a real answer, not a missing one.
            tables: if tables { Some(projected_tables) } else { None },
        },
        omission: OmissionReport {
            nodes_total: payload.nodes.len() as u32,
            nodes_omitted: omitted,
            limitation_code: GEOMETRY_ABSENT_OMITTED,
        },
    })
}

/// Project a page-less representation into `ethos.grounding.v1` schema 1.1.0.
///
/// Every node becomes an element under **its own node id** — a page-less
/// artifact carries no spans, so the id a consumer joins back to the record by
/// has to live on the element itself. The element's address is the node's
/// native locator, canonically serialized: opaque to the verifier, which
/// resolves by element id, and exactly reversible by a consumer holding the
/// representation. No geometry exists anywhere in the shape, so nothing is
/// omitted for lacking a box — the omission ledger the PDF path keeps for
/// ink-less runs has nothing to count here.
fn project_page_less(repr: &DocumentRepresentation) -> Result<Projection, EngineError> {
    let payload = repr.payload();
    if !PAGE_LESS_MEDIA_TYPES.contains(&payload.source.media_type.as_str()) {
        return Err(EngineError::Unsupported {
            what: "grounding source".into(),
            detail: format!(
                "`{}` names media type `{}`, which `{GROUNDING_ARTIFACT_TYPE}` admits under \
                 neither its paginated (1.0.0) nor its page-less (1.1.0) shape. The schema's \
                 media list is the contract; a type outside it is refused by name rather than \
                 projected into a shape nobody defined for it.",
                payload.identity.artifact_type, payload.source.media_type
            ),
        });
    }
    if !payload.pages.is_empty() {
        return Err(malformed(format!(
            "a page-less source declares {} page(s); the representation law says it must \
             declare none, and a page here would be the invented pagination \
             `docs/14-V2-SCOPE.md` §3 refuses",
            payload.pages.len()
        )));
    }

    let mut elements = Vec::with_capacity(payload.nodes.len());
    for node in &payload.nodes {
        let locator_bytes = ethos_parser_core::c14n::canonical_bytes_of(&node.native_locator)
            .map_err(|e| malformed(e.to_string()))?;
        let locator = String::from_utf8(locator_bytes)
            .map_err(|e| malformed(format!("locator serialization is not UTF-8: {e}")))?;
        elements.push(Element {
            id: node.id.as_str().to_string(),
            page: None,
            bbox: None,
            kind: node.kind.as_str().to_string(),
            text: Some(node.text.clone()),
            locator: Some(locator),
        });
    }

    Ok(Projection {
        source: GroundingSource {
            artifact_type: GROUNDING_ARTIFACT_TYPE.to_string(),
            schema_version: GROUNDING_SCHEMA_VERSION_PAGE_LESS.to_string(),
            source: Source {
                media_type: payload.source.media_type.clone(),
                sha256: payload.source.sha256.to_string(),
            },
            producer: Producer {
                name: payload.processing_run.processor.name.clone(),
                version: payload.processing_run.processor.version.clone(),
            },
            // The artifact carries no spans, no offsets, no tables — the
            // capabilities describe THIS artifact, not the reader's talents.
            capabilities: GroundingCapabilities {
                spans: false,
                char_offsets: false,
                tables: false,
            },
            coordinate_system: GroundingCoordinateSystem {
                unit: "centipoint".to_string(),
                origin: "top-left".to_string(),
            },
            pages: Vec::new(),
            elements,
            spans: None,
            tables: None,
        },
        omission: OmissionReport {
            nodes_total: u32::try_from(payload.nodes.len()).unwrap_or(u32::MAX),
            nodes_omitted: 0,
            limitation_code: GEOMETRY_ABSENT_OMITTED,
        },
    })
}

fn page_of(p: &PageRecord) -> Page {
    Page {
        id: p.id.as_str().to_string(),
        index: p.index,
        width: p.width,
        height: p.height,
        // The representation validates rotation into {0, 90, 180, 270} before this runs, so the
        // narrowing cannot lose information; it is checked rather than asserted all the same.
        rotation: u16::try_from(p.rotation).unwrap_or(u16::MAX),
    }
}

fn malformed(detail: String) -> EngineError {
    EngineError::Malformed {
        what: "grounding projection".into(),
        detail,
    }
}

/// Canonical bytes of a grounding artifact.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the artifact will not canonicalize — which, since c14n rejects
/// any non-integer number, also means no float ever reaches a grounding artifact.
pub fn to_canonical_bytes(g: &GroundingSource) -> Result<Vec<u8>, EngineError> {
    ethos_parser_core::c14n::canonical_bytes_of(g).map_err(|e| malformed(e.to_string()))
}

/// The crate name, used by the M0 harness to prove the workspace links.
pub const CRATE_NAME: &str = "ethos-parser-grounding";
