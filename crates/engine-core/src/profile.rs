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

//! Profile-as-identity (`docs/01-CONTRACT.md` §2, `docs/04-ARCHITECTURE.md` §3).
//!
//! The profile is every knob that can change output. Its hash goes on every artifact, and two
//! artifacts are comparable if and only if their `profile_sha256` matches.
//!
//! This is the cheapest load-bearing idea in the design: one hash buys OCR isolation, backend
//! isolation, and comparability, with no other machinery. An OCR run gets a different profile,
//! so its output is non-comparable with a born-digital parse *by contract* rather than by
//! anyone remembering to check.
//!
//! **Anything that can change a byte of output belongs here or is a bug.** The sensitivity test
//! destructures `Profile` exhaustively, so adding a field without covering it fails to compile.

use serde::{Deserialize, Serialize};

use crate::c14n::{c14n_bytes, C14nError};
use crate::geom::QUANTUM_PER_POINT;
use crate::identity::{CoordinateSystem, Sha256Hex};

/// The reading-order rule v0 shipped: single column, no multi-column detection.
///
/// Versioned as a string because the *rule* is part of identity. pdf-inspector's multi-column
/// detection flips on `min_lines < 15`, so a one-line document edit reorders the whole page; a
/// cliff-shaped heuristic cannot sit under a determinism contract, and when a stable rule lands
/// it gets a new id here rather than silently replacing this one.
///
/// **v1-S5 landed that rule, and this id kept its meaning rather than acquiring a new one.**
/// `single-column-v1` means *content-stream order, no reordering* — it always did. The new rule
/// is [`READING_ORDER_RULE_V1`]. Bumping this string in place would have been the one move that
/// makes an old artifact and a new one look comparable while their orders disagree, so the
/// constant stays, exactly as spelled, for every artifact produced before the change and for the
/// tests that still name it.
pub const READING_ORDER_RULE_V0: &str = "single-column-v1";

/// The reading-order rule v1-S5 ships: geometric column gutters, then blocks within a column.
///
/// # What the id claims
///
/// Runs are ordered by **whitespace in page space**, not by stream position, not by the structure
/// tree, and not by a line count. A vertical band that no run's horizontal extent intersects, and
/// that is wide enough by the rule's own floor, splits a page into column bands read
/// left-to-right; within a band the same cut runs horizontally to order blocks top-to-bottom.
/// **A page with no such gutter keeps content-stream order** — the rule reorders where it has
/// geometric evidence and does nothing where it has none.
///
/// # Why it is not `single-column-v2`
///
/// A version bump on the old id would say "same rule, refined". This is a different rule reading
/// different evidence: the old one read the content stream's sequence, this one reads the page's
/// whitespace. An artifact under each can disagree about what a document says, in order, and the
/// two ids are what make that disagreement legible instead of silent.
///
/// # Why not the algorithm's name
///
/// `xy-cut-v1` would name the family the recursion belongs to. The id names the *evidence* —
/// gutters between columns — the way [`TABLE_DETECTION_V1`] names painted rectangles and
/// [`TABLE_DETECTION_UNRULED_V1`] names alignment. A reader deciding whether to trust an order
/// needs to know what was measured, not which paper the loop came from.
///
/// Its constants live with the rule, in `engine-pdf`'s `reading_order` module, and changing any
/// of them is a new id rather than a quiet redefinition of this one.
pub const READING_ORDER_RULE_V1: &str = "gutter-columns-v1";

/// The rule v1-S6 ships for images and text findings: what is observed, and how.
///
/// One id covering both because they are one pass over one content stream, reading the same
/// graphics state: the current transformation matrix places an image, and the text rendering mode
/// and the visible page box decide what a run is flagged with. Splitting them would suggest a
/// document could be under one and not the other.
///
/// What is part of it, and therefore what a change to it must move this string for:
///
/// - which `Do` calls become nodes — `/Subtype /Image` only, never `/Form`, never inline `BI`
/// - how a painted rectangle is derived — the CTM applied to the unit square, axis-aligned or
///   typed-absent, never the bitmap's pixel dimensions
/// - what the digest covers — the stream's stored bytes, still encoded
/// - what counts as invisible — text rendering modes 3 and 7
/// - what counts as off-page — the origin outside `/CropBox`, or `/MediaBox` where no crop box is
///   declared, after `/Rotate`
///
/// **It does not cover contrast.** Nothing here reads colour, and a profile under this id makes no
/// claim about whether text was legible — see `codes::LOW_CONTRAST_NOT_DETECTED`.
pub const OBSERVATION_RULE_V1: &str = "page-observations-v1";

/// The rule v1-S6.1 ships for turning a string operand into character codes.
///
/// **The font's own `/Subtype` decides the code width, and nothing else does.** A simple font is
/// one byte per code (PDF 32000-1 §9.6); a composite font's width belongs to the CMap its
/// `/Encoding` names (§9.7.5).
///
/// It is a versioned rule and not an implementation detail because it decides **what the text
/// says**. Through v1-S6 the width came from whichever decoder a font happened to get, so a simple
/// font shipping a two-byte `/ToUnicode` codespace had its codes fused in pairs — 8 417 runs
/// dropped from one corpus document, and the artifact declared that document's fonts damaged.
/// Artifacts from either side of this id disagree about a document's text, which is precisely what
/// a rule id exists to make legible.
///
/// What is part of it:
///
/// - which `/Subtype` values are simple — everything except `Type0`, including an absent one
/// - that a simple font's width is one byte regardless of any CMap it carries
/// - that a composite font's width comes, **for now**, from its `/ToUnicode` codespace, declared
///   as `codes::COMPOSITE_FONT_CODES_FROM_TOUNICODE` because `/Encoding` CMaps are not parsed
pub const TEXT_CODE_RULE_V1: &str = "declared-font-codes-v1";

/// The **ruled** table-detection rule: grids reconstructed from painted rectangles.
///
/// Named here rather than in `engine-pdf` because the profile is `engine-core`'s and a rule id is
/// data. The rule itself — the lattice tolerance, what counts as a grid — lives with the detector,
/// and a test asserts the two strings agree so they cannot drift into naming different things.
///
/// **The unruled half is not in this rule.** A table implied by alignment is a different
/// derivation under a different id ([`TABLE_DETECTION_UNRULED_V1`]), and rolling it into this one
/// would make two very different inferences share an identity — which is exactly what a versioned
/// rule id exists to prevent.
///
/// **Superseded by [`TABLE_DETECTION_V2`] at v1-S7b.** Kept, exactly as spelled, because artifacts
/// produced before that change name it and a reader must still be able to look it up — the same
/// reason [`READING_ORDER_RULE_V0`] survived v1-S5.
pub const TABLE_DETECTION_V1: &str = "ruled-rects-v1";

/// The **ruled** table-detection rule v1-S7b ships: a coherence witness may not be the border.
///
/// # What changed, and why it is a new id rather than a fix
///
/// [`TABLE_DETECTION_V1`] asked whether every face of the lattice was covered by **some** painted
/// rectangle. A page-background panel answers yes for every face at once, so the precondition did
/// no work on a page that paints decoration on top of a panel — while the detector, two hundred
/// lines away, already refused to emit that same panel as a cell, calling it the table's own
/// border. A rectangle cannot both be *"not a cell, it merely encloses the grid"* and *"proof the
/// grid's cells were drawn"*. `-v2` excludes a rectangle spanning the whole lattice from being a
/// witness, matching the exclusion that already applied to cells.
///
/// Measured on `cfpb-home-loan-toolkit`, which paints a 351 × 454 pt panel behind scattered
/// highlight bars on two pages: those pages emitted a 17 × 13 table holding 12 cells and a 23 × 8
/// holding 29, one of them on a page whose structure tree declares no table at all. Under `-v2`
/// both are refused, **every other detection on the corpus is unchanged and no true positive is
/// lost**, and detection precision goes from 900‰ to 1000‰.
///
/// Two artifacts either side of this id disagree about whether a page has a table, which is
/// exactly the disagreement a rule id exists to make legible.
pub const TABLE_DETECTION_V2: &str = "ruled-rects-v2";

/// The **unruled** table-detection rule v1-S2 ships: grids inferred from text alignment.
///
/// A separate id from [`TABLE_DETECTION_V1`], not a bump of it. The two answer different
/// questions about a document:
///
/// | Rule | Evidence | What it means when it fires |
/// | --- | --- | --- |
/// | `ruled-rects-v2` | rectangles the author **painted** | the document drew this grid |
/// | `unruled-align-v1` | where the author **placed text** | a detector inferred this grid |
///
/// Sharing one id between them would make an artifact unable to say which of those two happened,
/// and "the document drew it" is a far stronger claim than "we inferred it". Every table on the
/// wire names the rule that produced it (`TableRecord::detection_rule`).
///
/// Its tolerances — the cluster width, the gutter floor, the coherence precondition and the
/// lattice cap — are part of the rule. Changing any of them takes a new id rather than silently
/// redefining this one, so artifacts from two detectors stay correctly non-comparable.
pub const TABLE_DETECTION_UNRULED_V1: &str = "unruled-align-v1";

/// The **stroke-ruled** table-detection rule v1-S8 ships: grids drawn as ruling lines.
///
/// A third id rather than a widening of [`TABLE_DETECTION_V2`], for the reason the ruled and
/// unruled ids are separate: the evidence differs. A filled rectangle is the author saying *this
/// box is here*; a two-point stroked segment is the author saying *this edge is here*. Both are
/// ink the author put down — which is why this rule's claim is as strong as the ruled one's and
/// stronger than the alignment rule's — but they are different statements, and a table on the wire
/// says which one it was built from.
///
/// Measured in v1-S7b: every one of the nine tagged tables `cfpb-home-loan-toolkit` missed sits on
/// a page that strokes segments, 103 cells and 65% of that document's gold, because a two-point
/// segment produced no rectangle and never reached a lattice.
pub const TABLE_DETECTION_STROKE_V1: &str = "stroke-ruled-v1";

/// The structure-tree rule v1-S3 ships: read `/StructTreeRoot`, bind by `(page, mcid)`.
///
/// On the profile because it changes output. Which structure types are recognised, how `/RoleMap`
/// is applied, how deep `/K` may nest before the walk refuses, and the fact that the join is exact
/// equality rather than anything looser — all of them decide which runs come out with a role path.
/// Changing any takes a new id rather than silently redefining this one.
///
/// **It names a reading, not an inference.** Every role this rule reports is one the document
/// wrote down; a file with no tree produces no roles and says so.
pub const STRUCT_TREE_RULE_V1: &str = "struct-tree-v1";

/// The forms-and-annotations rule v1-S4 ships.
///
/// On the profile because it decides which nodes exist. Which flag bits are named, how a
/// fully-qualified field name is built, whether a default value stands in for a missing one, and
/// the fact that a widget's rendering is **not** read as page text are all part of it — change any
/// and the node set changes.
///
/// **Named for what it does, not for the structure it reads.** `engine-core` carries no PDF
/// concept (`docs/04-ARCHITECTURE.md` §1, and a test enforces it), so the field and the id name
/// *form fields and annotations* — document ideas any format can have — while the format-specific
/// walk lives in `engine-pdf`. The same split `table_detection` already uses: a generic field
/// holding `"ruled-rects-v2"`.
pub const FORM_ANNOTATION_RULE_V1: &str = "form-annotations-v1";

/// Identity of the character-decoding data this profile carries.
///
/// Names what is **actually** vendored rather than what was planned. At M3 that is the
/// PDF 32000-1 Annex D encoding tables — `WinAnsiEncoding`, the ASCII range of
/// `StandardEncoding`, and a glyph-name subset — held in `engine-pdf`'s `encoding` module.
///
/// The Adobe predefined CJK CMaps are **not** carried, so a document naming one is refused
/// rather than decoded approximately. That is a declared limitation, recorded in every extract
/// artifact's `not_decoded` list and in `vendor/README.md`. When those files land this string
/// changes, which moves `profile_sha256` — artifacts from before and after are then correctly
/// non-comparable, because they really were produced by different decoders.
pub const CMAP_DATA_VERSION: &str = "annex-d-encodings-1";

/// Identity of the object/xref backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendIdentity {
    /// Backend crate or library name.
    pub name: String,
    /// Exact version string.
    pub version: String,
}

impl Default for BackendIdentity {
    /// The v0 backend, resolved.
    ///
    /// The version moving changes the profile hash, which is the event we want visible: two
    /// artifacts produced by different backend builds are correctly non-comparable.
    fn default() -> Self {
        Self {
            name: "lopdf".into(),
            // The resolved `lopdf` version, wired in at M2 when the dependency landed. Bumping
            // the crate moves this string, which moves `profile_sha256` — which is the point:
            // a backend change is fingerprint-visible rather than silent.
            version: "0.44.0".into(),
        }
    }
}

/// Which table-detection rules are in force, named one by one.
///
/// # Why this is a structure and not a string
///
/// v1-S1 shipped `table_detection` as a single string, because there was a single rule. v1-S2
/// added a second one, and a single string then has to mean two things at once: a reader seeing
/// `"ruled-rects-v1"` cannot tell whether the run looked for unruled tables and found none, or
/// never looked. Those are different documents to a consumer, and collapsing them is precisely
/// the "empty array versus absent key" distinction the contract makes everywhere else.
///
/// So both rules are named, always. A future slice that retires or adds one changes this shape
/// and moves the hash, which is the event a reader wants visible.
///
/// # `deny_unknown_fields`, and why not an internally-tagged enum
///
/// v0.1 measured this: serde's internally-tagged representations **buffer through a map and drop
/// keys they do not recognise**, so `deny_unknown_fields` does not reach inside one. A profile
/// written by a newer engine — carrying a third rule id — would deserialize with that id silently
/// discarded and then re-hash to a different digest than the one it arrived with, claiming a
/// comparability it does not have. A plain struct denies unknown fields for real, so an
/// unrecognised rule fails closed instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableDetection {
    /// Version id of the rule that reconstructs grids from painted rectangles.
    ///
    /// See [`TABLE_DETECTION_V2`].
    pub ruled: String,
    /// Version id of the rule that infers grids from text alignment.
    ///
    /// See [`TABLE_DETECTION_UNRULED_V1`]. Present because this profile **runs** it: an artifact
    /// naming it and carrying `tables: []` means the alignment rule looked and refused, not that
    /// nobody looked.
    pub unruled: String,
    /// Version id of the rule that reconstructs grids from stroked **ruling lines** (v1-S8).
    ///
    /// See [`TABLE_DETECTION_STROKE_V1`]. A third field rather than a widening of either other
    /// one, for the same reason there were two: the evidence differs, and an artifact says which
    /// kind of evidence its table was built from. Its arrival is what lets the profile stop
    /// declaring `stroke-ruled-tables-not-detected`, which every build declared from v1-S1 to
    /// v1-S7b.
    pub stroke_ruled: String,
}

impl Default for TableDetection {
    /// All three rules, enabled — the two v1-S2 shipped and the one v1-S8 added.
    fn default() -> Self {
        Self {
            ruled: TABLE_DETECTION_V2.to_string(),
            unruled: TABLE_DETECTION_UNRULED_V1.to_string(),
            stroke_ruled: TABLE_DETECTION_STROKE_V1.to_string(),
        }
    }
}

/// What this profile claims it can do.
///
/// The first three mirror `ethos.grounding.v1`'s required `capabilities` object exactly, so the
/// M5 projection is a move rather than a translation. The rest are representation-level and
/// have no grounding counterpart yet.
///
/// L1's achievement condition names capability declarations explicitly, so an artifact without
/// them has not reached "extracted" regardless of how good its text is.
///
/// # Two rules bind this type, in both directions
///
/// 1. **No `true` without a proof test.** `crates/engine-pdf/tests/capabilities.rs` maps every
///    `true` field to a named test and fails the build when one is missing — a capability
///    asserted without a passing test is the failure mode this type exists to prevent.
/// 2. **No `false` without a declared limitation.** [`Capabilities::declared_limitations`]
///    derives one per `false` field, exhaustively, so a caller never has to infer a gap from
///    silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    /// Text spans are emitted. (grounding-aligned)
    pub spans: bool,
    /// Spans carry character offsets into their element's text. (grounding-aligned)
    ///
    /// **v0: false.** M5 built `DocumentRepresentation v0` and left this false on purpose: v0
    /// performs no line or block grouping, so an element and a span are the *same object* and an
    /// offset would always be `0..len` — advertising sub-element addressing the engine cannot do.
    /// Ethos's own validator also ties the two together (offsets present must equal the
    /// capability), so claiming it would oblige every span to carry them. It flips at v1, when
    /// grouping makes elements coarser than spans and the offsets start carrying information.
    pub char_offsets: bool,
    /// Tables are detected and emitted. (grounding-aligned)
    ///
    /// **True since v1-S1**, and what it claims is precise — *this profile looked for tables*. It
    /// does not claim every table is found. `ethos.grounding.v1` encodes exactly that
    /// distinction: an absent `tables` key means the producer did not look, an empty array means
    /// it looked and found none, and a non-empty one is tables.
    ///
    /// What "looked" covers has widened twice. S1 looked only for grids the document painted and
    /// declared the alignment case as a limitation; S2 added the alignment rule and retired it;
    /// **S8 added the stroke-ruled rule and retired the one S2 left behind**. `tables: []` now
    /// means all *three* rules looked and none found one. The leftover is narrower again and
    /// still declared: an edge the document never drew is not supplied to complete a grid
    /// (`codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED`).
    pub tables: bool,
    /// Ink boxes come from measured font metrics rather than being absent.
    pub measured_ink_boxes: bool,
    /// Multi-column reading order is detected.
    ///
    /// **True since v1-S5**, and what it claims is the usual narrow thing — *this profile orders
    /// text by page geometry rather than by stream position*. It does not claim every layout is
    /// resolved. Where the page shows no column gutter the rule reorders nothing, and that is an
    /// answer rather than a gap: content-stream order is what a single-column page means.
    ///
    /// v0 through v1-S4 left this false and said so on every artifact, because the known
    /// heuristic flips on `min_lines < 15` and a one-line edit reordered a whole page. What
    /// replaced it is a rule over whitespace, whose id is [`READING_ORDER_RULE_V1`] and whose
    /// floors are compile-time constants, so a document's line count cannot move its order.
    ///
    /// A profile may still set this `false`; the partnering limitation is still declared, and
    /// such a profile must also name [`READING_ORDER_RULE_V0`] so the two agree.
    pub multi_column_reading_order: bool,
    /// Structural locators (`mcid`, tagged-structure roles) are captured.
    ///
    /// **True since v1-S3.** v0 through v1-S2 left this false on purpose and said why: a
    /// marked-content id was captured where a page's content stream supplied one, but the
    /// tagged-structure tree was never read, so there was no role path and an id resolved
    /// against nothing. Claiming the capability on the strength of that half would have promised
    /// an address consumers could not rely on.
    ///
    /// v1-S3 reads `/StructTreeRoot` and binds runs to it by exact `(page, mcid)` equality, so
    /// the claim is now the one this flag is for: **this profile looks**. It does not claim every
    /// document has structure. An untagged file produces no role paths and declares
    /// `untagged-structure-tree-absent`; a tagged file whose tree misses some marked content
    /// declares `structure-mcid-unbound` with a count. Both are real answers, and neither is a
    /// role invented to fill the gap.
    pub structural_locators: bool,
    /// Interactive form fields are read from the AcroForm tree and emitted as nodes (v1-S4).
    ///
    /// **True since v1-S4**, and what it claims is that *this profile looks*. A document with no
    /// `/AcroForm` yields no field nodes and that is a real answer, not a gap — `ethos`'s own
    /// empty-array-versus-absent-key distinction, applied to a node kind.
    ///
    /// It does **not** claim every field is read: XFA packets are not parsed (`xfa-forms-not-\
    /// extracted`), and a value in a shape this profile cannot express is declared rather than
    /// guessed at.
    pub form_fields: bool,
    /// Annotations are read from each page's `/Annots` and emitted as nodes (v1-S4).
    ///
    /// **True since v1-S4.** Separate from [`Self::form_fields`] because they are separately
    /// provable and separately absent: a document can carry comments and no form, or a form and
    /// no comments, and one flag covering both would be true on the strength of either.
    pub annotations: bool,
    /// Images a page paints with `Do` are emitted as nodes (v1-S6).
    ///
    /// **True since v1-S6**, and the claim is the narrow one this type always makes — *this
    /// profile looks*. A page that paints none yields no image nodes, and that is an answer.
    ///
    /// It does not claim every image is found: an image drawn inside a form XObject is not seen,
    /// because this profile does not descend into them, and an **inline** image (`BI`/`ID`/`EI`)
    /// is counted and declared rather than emitted. Both are separately declared limitations.
    ///
    /// It emphatically does not claim to know what any picture *shows*. No node carries a
    /// description, and none ever will under this profile: that is OCR or a model's opinion, and
    /// neither is evidence.
    pub images: bool,
    /// Page rasters are emitted at a pinned DPI (v1-S6).
    ///
    /// **False, and this is the honest answer rather than a deferral.** Rendering a page means a
    /// PDF renderer — glyph rasterization, shadings, blend modes, image filters — and this
    /// workspace has none, deliberately: `docs/00-NORTH-STAR.md` #14 admits PDFium only
    /// caller-provided under an explicit ADR, and `deny.toml` is an allowlist that no AGPL
    /// renderer clears. Shelling out to `pdftoppm` would put an unpinned binary between the
    /// document and the artifact.
    ///
    /// The partnering limitation is declared on every artifact, and
    /// [`Profile::raster_dpi`] carries the not-emitted state explicitly so that a future renderer
    /// arriving is a profile-hash event rather than a silent change of meaning.
    pub page_screenshots: bool,
    /// A Markdown projection is emitted, always with its Anchor Map (v1.1-S1).
    ///
    /// **True since v1.1-S1**, and what it claims is narrow on purpose — *this profile can project
    /// the representation into Markdown that stays citable*. It does not claim the Markdown is
    /// pretty, complete, or a good chunking unit.
    ///
    /// The capability is about the **pair**. `docs/01-CONTRACT.md` §12 refused a Markdown
    /// projection outright for seven v1 slices, on Workbench rule 8: a projection between what is
    /// ranked and what is cited is where a locator dies silently, and checklist O8 records that
    /// rule 8 *prefers no projection at all*. What flips this flag is not "we wrote a serializer"
    /// but "the map is a field of the same artifact and the type will not construct without it".
    ///
    /// A profile may still set this `false`; the partnering limitation is declared, and
    /// [`Profile::markdown_rule`] must then be absent from the emitted surface in the same way
    /// every other off capability behaves.
    pub markdown: bool,
}

impl Capabilities {
    /// What v0 actually claims.
    ///
    /// Note how much is `false`. Two of these were `true` in the M1 sketch and are `false` here
    /// because M4 asked for the proof and the proof did not exist: `char_offsets` has no
    /// hierarchy to index into until grouping lands, and `structural_locators` would be claiming a full
    /// structural address on the strength of a best-effort `mcid`. Narrowing a declaration when
    /// the evidence does not support it is the mechanism working, not a regression.
    pub const V0: Self = Self {
        spans: true,
        char_offsets: false,
        tables: true,
        measured_ink_boxes: true,
        multi_column_reading_order: true,
        structural_locators: true,
        form_fields: true,
        annotations: true,
        images: true,
        page_screenshots: false,
        markdown: true,
    };
}

/// The resolution page rasters are emitted at, or a declared reason there are none (v1-S6).
///
/// # A declared state, not an absent field
///
/// The same discipline [`PageBudget`] is under, for the same reason. `{"mode":"not_emitted"}` says
/// *this profile considered rasters and emits none*; an omitted `Option` would leave a reader
/// unable to tell that from a build that has no such knob at all — and would let a future renderer
/// arrive without moving `profile_sha256`, so an artifact with rasters and one without would
/// compare as though they came from the same reader.
///
/// It is on the profile because a DPI is output-affecting in the strongest sense: raster pixels are
/// a **second coordinate system**, and two artifacts rendered at different resolutions describe the
/// same page with different numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
// Adjacently tagged with `deny_unknown_fields`, matching `XrefRepair` and `PageBudget`, and for
// the reason recorded there: serde does NOT honour `deny_unknown_fields` on an *internally*
// tagged enum, so `{"mode":"not_emitted","dpi":300}` would parse, drop the field, and re-hash to
// a digest different from the one it arrived with. A profile knob that can be silently discarded
// is worse than no knob.
#[serde(
    rename_all = "snake_case",
    tag = "mode",
    content = "dpi",
    deny_unknown_fields
)]
#[non_exhaustive]
pub enum RasterDpi {
    /// No raster is produced. See [`Capabilities::page_screenshots`].
    NotEmitted,
}

/// How many pages a run may process before it stops.
///
/// A **declared state, not an absent field**: `{"mode":"unlimited"}` says the budget was
/// considered and found unbounded, where an omitted `Option` would leave a reader unable to
/// distinguish "unbounded" from "this build has no such knob". It is also the difference between
/// a profile that round-trips honestly and one that does not — an `Option` field missing from
/// incoming JSON deserializes to `None` and silently re-hashes as though it had been there.
///
/// # Why a page count is the right knob
///
/// `failure/memory-limit-simulated` is **byte-identical to `synthetic/simple-text`** (both
/// `sha256:f2f6ab91…`). The fixture name means *the limit is simulated by configuration*, not
/// *this PDF is huge* — so a meaningful test must set the budget explicitly. A page count is the
/// bound that actually governs peak cost in a page-at-a-time reader, which is why it is the knob
/// rather than a byte ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "mode",
    content = "pages",
    deny_unknown_fields
)]
pub enum PageBudget {
    /// No page bound. The v0 default.
    Unlimited,
    /// Process at most this many pages; the rest are quarantined and declared.
    ///
    /// Zero is legal and means "process none" — the only budget that can bite on a one-page
    /// document, and therefore the one `memory-limit-simulated` needs.
    AtMost(u32),
}

impl PageBudget {
    /// The bound as a count, or `None` when unbounded.
    pub fn max_pages_to_process(self) -> Option<u32> {
        match self {
            Self::Unlimited => None,
            Self::AtMost(n) => Some(n),
        }
    }

    /// Whether a **1-based** page number is inside the budget.
    pub fn admits(self, page: u32) -> bool {
        match self {
            Self::Unlimited => true,
            Self::AtMost(n) => page <= n,
        }
    }
}

/// Whether the one bounded cross-reference repair is in force.
///
/// **On the profile because it changes which documents produce an artifact at all.** A build that
/// repairs reads `synthetic/table-regular-grid`; a build that refuses exits 2 on it. Two artifacts
/// from those builds are not comparable, and the profile hash is what says so.
///
/// The repair itself is `engine_pdf::xref` and is bounded to one malformation — 19-byte entries
/// where PDF 32000-1 §7.5.4 requires 20 — under preconditions that make it offset-preserving.
/// This enum only records *whether* it runs. `engine-core` owns no PDF machinery
/// (`docs/04-ARCHITECTURE.md` §1); a mode name is data, and nothing here can parse anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
// Adjacently tagged, matching `PageBudget`, and that is load-bearing rather than cosmetic:
// serde does NOT honour `deny_unknown_fields` on an *internally* tagged enum, so
// `{"mode":"refuse","future_knob":true}` would parse, drop the knob, and re-hash to a digest
// different from the one it arrived with. The nested-field test catches exactly that.
#[serde(tag = "mode", content = "detail", deny_unknown_fields)]
pub enum XrefRepair {
    /// Refuse the malformation, as v0 did.
    #[serde(rename = "refuse")]
    Refuse,
    /// Pad 19-byte entries to the specified 20. The v0.1 default.
    ///
    /// Renamed explicitly rather than derived: `rename_all = "kebab-case"` turns `Pad19To20V1`
    /// into `pad19-to20-v1`, which is not the id the repair publishes as
    /// `engine_pdf::xref::XREF_REPAIR_V1`. Two spellings of one repair is exactly the drift a
    /// versioned id exists to prevent, and a test asserts the two strings are equal.
    #[serde(rename = "pad-19-to-20-v1")]
    Pad19To20V1,
}

impl XrefRepair {
    /// Whether a repair may be attempted.
    pub fn is_enabled(self) -> bool {
        matches!(self, Self::Pad19To20V1)
    }
}

/// Which verifier this profile is bound to, if any.
///
/// **The engine does not verify** (`docs/07-VERIFY-BOUNDARY.md`). It can *invoke* a verifier, and
/// from v0.1 `engine verify` spawns the Ethos CLI and relays its report bytes. That makes which
/// verifier answered part of the run's identity, so it is pinned here: a verifier swap is
/// fingerprint-visible, exactly as `backend` makes a `lopdf` swap visible.
///
/// [`VerifierPin::NotPinned`] is the default and is a real statement rather than a missing field:
/// *this profile describes a run that did not consult a verifier.* Every classify, extract and
/// ground artifact carries it, because none of them verify anything.
///
/// A version string alone would not be enough — two builds of the same Ethos version can differ —
/// so the digest of the binary's bytes is carried with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// Adjacently tagged for the same reason as `XrefRepair` — see the note there.
#[serde(
    rename_all = "snake_case",
    tag = "mode",
    content = "identity",
    deny_unknown_fields
)]
pub enum VerifierPin {
    /// No verifier was consulted.
    NotPinned,
    /// A specific verifier binary answered.
    Pinned {
        /// What the binary reports for `--version`, trimmed.
        version: String,
        /// Digest of the verifier binary's bytes.
        sha256: Sha256Hex,
    },
}

impl VerifierPin {
    /// Pin a verifier by its reported version and the digest of its bytes.
    pub fn pinned(version: impl Into<String>, sha256: Sha256Hex) -> Self {
        Self::Pinned {
            version: version.into().trim().to_string(),
            sha256,
        }
    }

    /// Whether a verifier is pinned.
    pub fn is_pinned(&self) -> bool {
        matches!(self, Self::Pinned { .. })
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self::V0
    }
}

/// The pinned configuration whose hash is the engine's identity.
///
/// Host-varying data (paths, timings, thread counts, machine identity) is deliberately absent:
/// including it would make every machine produce a different fingerprint for identical work,
/// which is the opposite of what this is for.
/// `deny_unknown_fields` is load-bearing, not tidiness. Without it, a profile written by a newer
/// engine — carrying a knob this build does not know about — would deserialize with that knob
/// silently dropped and then **re-hash to a different digest than the one it arrived with**. The
/// artifact would claim comparability it does not have. Failing closed on an unrecognised field
/// is `docs/01-CONTRACT.md` §8 applied to our own artifacts, which is where it matters most.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// The engine build that this profile describes.
    pub parser_version: String,
    /// Object/xref backend identity and version.
    pub backend: BackendIdentity,
    /// How many pages classification samples before it stops. Default 8.
    ///
    /// On the profile because changing it changes classification output. It is also the knob
    /// whose *boundedness* is the point: cost must not scale with total page count.
    pub classify_sample_pages: u32,
    /// Quanta per point. 100 (centipoints).
    pub quantum_per_point: u32,
    /// The declared coordinate system.
    pub coordinate_system: CoordinateSystem,
    /// Declared capabilities.
    pub capabilities: Capabilities,
    /// How many pages any stage may process before it stops. Default [`PageBudget::Unlimited`].
    ///
    /// On the profile because it changes output: a budgeted run emits a **partial** artifact
    /// with quarantined pages and a declared limitation, and an artifact produced under a
    /// different budget is correctly non-comparable with one produced under none.
    pub page_budget: PageBudget,
    /// Version id of the reading-order rule in force.
    ///
    /// [`READING_ORDER_RULE_V1`] by default since v1-S5. [`READING_ORDER_RULE_V0`] is still a
    /// legal value and still means what it always meant — content-stream order — which is what a
    /// profile setting [`Capabilities::multi_column_reading_order`] back to `false` must also say
    /// here, so the rule id and the capability cannot disagree about whether anything was
    /// reordered.
    pub reading_order_rule: String,
    /// The table-detection rules in force. New at v1-S1, widened to a structure at v1-S2.
    ///
    /// The *rules* are the identity: the lattice tolerances, what counts as a grid, and how a
    /// merged cell is recognised are all part of them. Changing any of that takes a new id rather
    /// than silently redefining one, so artifacts from two detectors are correctly
    /// non-comparable. [`TableDetection`] says why this stopped being a single string.
    pub table_detection: TableDetection,
    /// Version id of the structure-tree rule in force. New at v1-S3.
    ///
    /// See [`STRUCT_TREE_RULE_V1`]. On the profile because it decides which runs come out
    /// carrying a role path: the recognised structure types, the `/RoleMap` handling, the depth
    /// bound and the exactness of the `(page, mcid)` join are all part of it.
    pub struct_tree_rule: String,
    /// Version id of the Markdown projection rule in force (v1.1-S1).
    ///
    /// See [`crate::markdown::MARKDOWN_RULE_LINEAR_V1`]. On the profile because it decides what
    /// comes out: a run that projected headings from font sizes and a run that refused to would
    /// disagree about the same document, and an artifact whose hash could not tell them apart
    /// would claim a comparability it lacks.
    ///
    /// **A profile JSON predating v1.1-S1 — one with no `markdown_rule` — is refused, not
    /// defaulted.** The same posture `table_detection.stroke_ruled` took at v1-S8: a field
    /// defaulted in is a claim the run never made.
    pub markdown_rule: String,
    /// Version id of the forms-and-annotations rule in force. New at v1-S4.
    ///
    /// See [`FORM_ANNOTATION_RULE_V1`].
    pub form_annotation_rule: String,
    /// Identity of the vendored character-decoding data. See [`CMAP_DATA_VERSION`].
    pub cmap_data_version: String,
    /// Version id of the character-code rule in force. New at v1-S6.1.
    ///
    /// See [`TEXT_CODE_RULE_V1`]. Distinct from [`CMAP_DATA_VERSION`], which names the vendored
    /// encoding *tables*: this names how a string is divided into codes before any table is
    /// consulted, and the two can move independently.
    pub text_code_rule: String,
    /// Version id of the image and text-finding rule in force. New at v1-S6.
    ///
    /// See [`OBSERVATION_RULE_V1`]. On the profile because it decides which `Do` calls become
    /// nodes, how a painted rectangle is derived, and what counts as invisible or off-page —
    /// every one of which changes what an artifact says the document contains.
    pub observation_rule: String,
    /// The resolution page rasters are emitted at. New at v1-S6, [`RasterDpi::NotEmitted`] today.
    pub raster_dpi: RasterDpi,
    /// Whether the bounded cross-reference repair runs. New at v0.1.
    ///
    /// On the profile because it changes *which documents produce an artifact at all* — the
    /// strongest kind of output-affecting knob there is.
    pub xref_repair: XrefRepair,
    /// Which verifier this run is bound to. New at v0.1, [`VerifierPin::NotPinned`] by default.
    pub verifier: VerifierPin,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            parser_version: env!("CARGO_PKG_VERSION").to_string(),
            backend: BackendIdentity::default(),
            classify_sample_pages: 8,
            quantum_per_point: QUANTUM_PER_POINT,
            coordinate_system: CoordinateSystem::V0,
            capabilities: Capabilities::V0,
            page_budget: PageBudget::Unlimited,
            reading_order_rule: READING_ORDER_RULE_V1.to_string(),
            table_detection: TableDetection::default(),
            struct_tree_rule: STRUCT_TREE_RULE_V1.to_string(),
            markdown_rule: crate::markdown::MARKDOWN_RULE_LINEAR_V1.to_string(),
            form_annotation_rule: FORM_ANNOTATION_RULE_V1.to_string(),
            cmap_data_version: CMAP_DATA_VERSION.to_string(),
            text_code_rule: TEXT_CODE_RULE_V1.to_string(),
            observation_rule: OBSERVATION_RULE_V1.to_string(),
            raster_dpi: RasterDpi::NotEmitted,
            xref_repair: XrefRepair::Pad19To20V1,
            verifier: VerifierPin::NotPinned,
        }
    }
}

impl Profile {
    /// The canonical bytes this profile hashes over.
    ///
    /// # Errors
    ///
    /// [`C14nError`] if the profile contains a value c14n rejects. Not reachable through the
    /// public API today — every field is a string, bool, or `u32` — but returned rather than
    /// unwrapped so a future field cannot make this panic.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, C14nError> {
        // serde_json::to_value only fails for types this struct does not contain (maps with
        // non-string keys, types whose Serialize impl errors). Reported rather than unwrapped
        // so a future field cannot turn a contract violation into a panic.
        let value = serde_json::to_value(self)
            .map_err(|e| C14nError::new(format!("profile is not serializable: {e}")))?;
        c14n_bytes(&value)
    }

    /// `sha256:<hex>` over [`Self::canonical_bytes`].
    ///
    /// # Errors
    ///
    /// Propagates [`C14nError`].
    pub fn profile_sha256(&self) -> Result<Sha256Hex, C14nError> {
        let bytes = self.canonical_bytes()?;
        let hex = crate::c14n::sha256_hex_bytes(&bytes);
        // `from_hex` can only fail on a malformed digest, which sha256 cannot produce.
        Ok(Sha256Hex::from_hex(&hex).expect("sha256 hex is always 64 lowercase hex digits"))
    }
}

/// Convenience wrapper matching the API named in `docs/05-MILESTONES.md` M1.
///
/// # Errors
///
/// Propagates [`C14nError`].
pub fn profile_sha256(profile: &Profile) -> Result<Sha256Hex, C14nError> {
    profile.profile_sha256()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutate one field, hash, compare, restore.
    fn hash(p: &Profile) -> String {
        p.profile_sha256().unwrap().to_string()
    }

    /// Every field on `Profile` must change the hash when it changes.
    ///
    /// The destructuring binding below is the enforcement mechanism: adding a field to
    /// `Profile` without adding a case here is a **compile error**, not a silently uncovered
    /// knob. A knob that does not move the hash is a silent-drift bug waiting to happen.
    #[test]
    fn every_profile_field_is_hash_sensitive() {
        let base = Profile::default();
        let base_hash = hash(&base);

        // Exhaustiveness gate. If this fails to compile, a field was added — cover it below.
        //
        // Destructured to the LEAF, not just the top level: a field added to `Capabilities` or
        // `BackendIdentity` is as output-affecting as one added to `Profile`, and a gate that
        // stopped at `capabilities: _` would wave it straight through.
        let Profile {
            parser_version: _,
            backend:
                BackendIdentity {
                    name: _,
                    version: _,
                },
            classify_sample_pages: _,
            quantum_per_point: _,
            coordinate_system: CoordinateSystem { unit: _, origin: _ },
            capabilities:
                Capabilities {
                    spans: _,
                    char_offsets: _,
                    tables: _,
                    measured_ink_boxes: _,
                    multi_column_reading_order: _,
                    structural_locators: _,
                    form_fields: _,
                    annotations: _,
                    images: _,
                    page_screenshots: _,
                    markdown: _,
                },
            page_budget: _,
            reading_order_rule: _,
            text_code_rule: _,
            observation_rule: _,
            raster_dpi: _,
            table_detection:
                TableDetection {
                    ruled: _,
                    unruled: _,
                    stroke_ruled: _,
                },
            struct_tree_rule: _,
            markdown_rule: _,
            form_annotation_rule: _,
            cmap_data_version: _,
            xref_repair: _,
            verifier: _,
        } = &base;

        /// One named single-field mutation.
        type Mutation = (&'static str, Box<dyn Fn(&mut Profile)>);

        let mutations: Vec<Mutation> = vec![
            (
                "parser_version",
                Box::new(|p: &mut Profile| p.parser_version = "9.9.9-mutated".into()),
            ),
            (
                // v1-S1. Which ruled grids are found, and therefore which cells exist.
                "table_detection.ruled",
                Box::new(|p: &mut Profile| p.table_detection.ruled = "other-rule-v9".into()),
            ),
            (
                // v1-S2. The unruled rule is a SEPARATE knob: a run that inferred grids from
                // alignment produced different tables from one that did not, and an artifact
                // whose hash could not tell those apart would claim a comparability it lacks.
                "table_detection.unruled",
                Box::new(|p: &mut Profile| p.table_detection.unruled = "other-align-v9".into()),
            ),
            (
                // v1-S8. A third separate knob, for the third kind of evidence. A run that read
                // the page's ruling lines emitted tables a run that did not could not have, and
                // `cfpb-home-loan-toolkit` page 13 is the measured instance of exactly that.
                "table_detection.stroke_ruled",
                Box::new(|p: &mut Profile| {
                    p.table_detection.stroke_ruled = "other-stroke-v9".into()
                }),
            ),
            (
                // v1-S4. Which form and annotation nodes exist at all.
                "form_annotation_rule",
                Box::new(|p: &mut Profile| p.form_annotation_rule = "other-forms-v9".into()),
            ),
            (
                "capabilities.form_fields",
                Box::new(|p: &mut Profile| p.capabilities.form_fields = false),
            ),
            (
                "capabilities.annotations",
                Box::new(|p: &mut Profile| p.capabilities.annotations = false),
            ),
            (
                // v1-S3. Which runs come out with a role path, and therefore whether a consumer
                // can address text structurally at all.
                "struct_tree_rule",
                Box::new(|p: &mut Profile| p.struct_tree_rule = "other-tree-v9".into()),
            ),
            (
                // v0.1. The strongest output-affecting knob in the set: it changes which
                // documents produce an artifact at all.
                "xref_repair",
                Box::new(|p: &mut Profile| p.xref_repair = XrefRepair::Refuse),
            ),
            (
                // v0.1. `engine verify` spawns a verifier, so which one answered is part of the
                // run's identity — a swap must be fingerprint-visible, the way a backend swap is.
                "verifier",
                Box::new(|p: &mut Profile| {
                    p.verifier = VerifierPin::pinned(
                        "ethos 9.9.9",
                        Sha256Hex::from_hex(&"ab".repeat(32)).expect("well formed"),
                    )
                }),
            ),
            (
                "backend.name",
                Box::new(|p: &mut Profile| p.backend.name = "other-backend".into()),
            ),
            (
                "backend.version",
                Box::new(|p: &mut Profile| p.backend.version = "1.2.3-mutated".into()),
            ),
            (
                "classify_sample_pages",
                Box::new(|p: &mut Profile| p.classify_sample_pages = 16),
            ),
            (
                "quantum_per_point",
                Box::new(|p: &mut Profile| p.quantum_per_point = 1000),
            ),
            (
                "capabilities.spans",
                Box::new(|p: &mut Profile| p.capabilities.spans = false),
            ),
            (
                // Mutated toward `true`: `char_offsets` is false in V0, and a mutation to the
                // value a field already holds tests nothing.
                "capabilities.char_offsets",
                Box::new(|p: &mut Profile| p.capabilities.char_offsets = true),
            ),
            (
                // Mutated toward `false`: `tables` is TRUE as of v1-S1, and a mutation to the
                // value a field already holds tests nothing.
                "capabilities.tables",
                Box::new(|p: &mut Profile| p.capabilities.tables = false),
            ),
            (
                "capabilities.measured_ink_boxes",
                Box::new(|p: &mut Profile| p.capabilities.measured_ink_boxes = false),
            ),
            (
                // Mutated toward `false` since v1-S5, which flipped the capability. A mutation
                // that sets a field to the value it already holds is a test that passes without
                // testing anything, and the `assert_ne!` below is what catches that.
                "capabilities.multi_column_reading_order",
                Box::new(|p: &mut Profile| p.capabilities.multi_column_reading_order = false),
            ),
            (
                // Mutated toward `false`: it is TRUE as of v1-S3, and a mutation to the value a
                // field already holds tests nothing.
                "capabilities.structural_locators",
                Box::new(|p: &mut Profile| p.capabilities.structural_locators = false),
            ),
            (
                "page_budget",
                Box::new(|p: &mut Profile| p.page_budget = PageBudget::AtMost(4)),
            ),
            (
                // Zero is a distinct budget from four, and from unlimited. A knob whose
                // *value* did not move the hash would be as bad as one whose presence did not.
                "page_budget.pages",
                Box::new(|p: &mut Profile| p.page_budget = PageBudget::AtMost(0)),
            ),
            (
                // `xy-cut-v1` is deliberately a name the engine does not ship. v1-S5 considered
                // it for the real rule and chose `gutter-columns-v1` instead, which leaves this
                // probe distinct from every id in use — a mutation colliding with the live value
                // would silently stop testing the field.
                "reading_order_rule",
                Box::new(|p: &mut Profile| p.reading_order_rule = "xy-cut-v1".into()),
            ),
            (
                "cmap_data_version",
                Box::new(|p: &mut Profile| p.cmap_data_version = "adobe-2026-01".into()),
            ),
        ];

        let mut seen = std::collections::BTreeSet::new();
        seen.insert(base_hash.clone());

        for (name, mutate) in mutations {
            let mut p = Profile::default();
            mutate(&mut p);

            // Guard the guard. A "mutation" that writes back the value the field already holds
            // proves nothing about hash sensitivity while passing every assertion below — which
            // is exactly what happened to `capabilities.char_offsets` when its V0 value flipped
            // to `false` at M4 and the mutation still set `false`.
            assert_ne!(
                p, base,
                "the {name} mutation left the profile unchanged, so it tests nothing"
            );

            let h = hash(&p);
            assert_ne!(
                h, base_hash,
                "mutating {name} did not change profile_sha256"
            );
            assert!(
                seen.insert(h),
                "mutating {name} collided with another profile's hash"
            );
        }
    }

    #[test]
    fn coordinate_system_is_hash_sensitive_in_principle() {
        // Only one variant exists today, so this cannot be mutated into a different value. The
        // test records that the field IS hashed, by asserting it appears in canonical bytes —
        // otherwise adding a second origin later would silently not change identity.
        let bytes = Profile::default().canonical_bytes().unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("\"coordinate_system\":"), "field must be hashed");
        assert!(s.contains("\"origin\":\"top-left\""));
        assert!(s.contains("\"unit\":\"centipoint\""));
    }

    /// The default profile's canonical bytes and hash, pinned.
    ///
    /// Not redundant with the sensitivity test: that one proves a change is *detectable*, this
    /// one proves a change was *intended*. The v0 profile is the engine's identity, so it moves
    /// only in a commit that says so — the same discipline Ethos applies to its own profile
    /// artifact.
    #[test]
    fn the_default_profile_is_pinned() {
        let bytes = Profile::default().canonical_bytes().unwrap();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            r#"{"backend":{"name":"lopdf","version":"0.44.0"},"capabilities":{"annotations":true,"char_offsets":false,"form_fields":true,"images":true,"markdown":true,"measured_ink_boxes":true,"multi_column_reading_order":true,"page_screenshots":false,"spans":true,"structural_locators":true,"tables":true},"classify_sample_pages":8,"cmap_data_version":"annex-d-encodings-1","coordinate_system":{"origin":"top-left","unit":"centipoint"},"form_annotation_rule":"form-annotations-v1","markdown_rule":"markdown-linear-v1","observation_rule":"page-observations-v1","page_budget":{"mode":"unlimited"},"parser_version":"0.11.0","quantum_per_point":100,"raster_dpi":{"mode":"not_emitted"},"reading_order_rule":"gutter-columns-v1","struct_tree_rule":"struct-tree-v1","table_detection":{"ruled":"ruled-rects-v2","stroke_ruled":"stroke-ruled-v1","unruled":"unruled-align-v1"},"text_code_rule":"declared-font-codes-v1","verifier":{"mode":"not_pinned"},"xref_repair":{"mode":"pad-19-to-20-v1"}}"#,
            "the v0 profile changed. Expected causes: a crate version bump (parser_version is \
             part of identity, so a new build IS a new profile — that is by design), or a new \
             field. Update this vector and say why in the commit. Unexpected cause: something \
             added an output-affecting knob by accident.\n\n\
             Moved deliberately at M4, twice: `capabilities.char_offsets` true -> false (v0 \
             emits no element/span hierarchy for an offset to index into; it lands at M5), and \
             the new `page_budget` knob. Moved again at M7, for the workspace 0.0.0 -> 0.1.0 \
             bump that freezes v0: `parser_version` is a profile field, so the version bump IS a \
             profile change. Moved a third time at v0.1 (0.2.0), for three reasons at once — the \
             version bump, the new `xref_repair` knob, and the new `verifier` pin. That one is \
             the largest identity change since M1: `xref_repair` decides whether a 19-byte xref \
             table produces an artifact at all. Artifacts from before and after are correctly \
             non-comparable, because the profile that produced them really did change. Moved a \
             fourth time at v1-S1 (0.3.0): the version, the new `table_detection` rule, and \
             `capabilities.tables` flipping false -> true. That last one is not a knob but a \
             CLAIM — artifacts before it did not look for tables and artifacts after it did, \
             which is exactly the kind of difference a profile hash exists to make visible.\n\n\
             Moved a fifth time at v1-S2 (0.4.0): the version, and `table_detection` widening \
             from a single string to a structure naming BOTH rules. Same class of change as the \
             fourth — an artifact from before this looked for ruled grids only, and one from \
             after also inferred grids from alignment, so the two really did come from different \
             detectors and must not be compared cell for cell.\n\n\
             Moved a sixth time at v1-S3 (0.5.0): the version, the new `struct_tree_rule`, and \
             `capabilities.structural_locators` flipping false -> true. Another CLAIM rather \
             than a knob — artifacts before it never read a document's structure tree and \
             artifacts after it do, so a role path present in one and absent from the other says \
             nothing about the two documents and everything about the two profiles.\n\n\
             Moved a seventh time at v1-S4 (0.6.0): the version, the new `form_annotation_rule`, \
             and `capabilities.form_fields` and `capabilities.annotations` both arriving true. \
             Two more CLAIMS: an artifact from before this carried no form field and no \
             annotation because none was ever looked for, and one from after carries them or \
             says the document has none. Comparing the two node counts would be comparing two \
             different questions.\n\n\
             Moved an eighth time at v1-S5 (0.7.0): the version, `reading_order_rule` moving \
             from `single-column-v1` to `gutter-columns-v1`, and \
             `capabilities.multi_column_reading_order` flipping false -> true. This is the one \
             identity change so far that reorders EVIDENCE rather than adding it. Two artifacts \
             either side of it can list the same runs, with the same text and the same origins, \
             in a different sequence — and a consumer that concatenated them would get two \
             different documents. That is precisely why the rule id is a profile field and why \
             it took a new name instead of a version bump on the old one.\n\n\
             Moved a ninth time at v1-S6 (0.8.0): the version, the new `observation_rule` and \
             `raster_dpi` fields, and TWO capability flips — `images` false -> true, and \
             `page_screenshots` arriving as an explicit false. The first is a claim: artifacts \
             before it carried no image node because none was ever looked for, and ones after \
             carry them or say the document paints none. The second is the opposite kind of \
             entry and just as load-bearing — a field that says `no raster, and that was decided` \
             rather than leaving a reader unable to tell a build with no renderer from a build \
             where nobody thought about it. This release also REPAIRED the page-box transform, \
             which discarded the box origin; on every document in either corpus that origin is \
             (0, 0) and the repair is the identity, so no coordinate in any existing artifact \
             moves — but a document with an offset box would have been wrong before and is right \
             now, which is a difference the hash should carry.\n\n\
             Moved a tenth time at v1-S6.1 (0.8.1): the version and the new `text_code_rule`. \
             This one is the largest change to what an artifact SAYS since M1 — not a new field \
             or a new claim, but different TEXT. A simple font's codes were being fused in pairs, \
             so 8 417 runs were missing from one corpus document's artifact and what survived was \
             garbled. Two artifacts either side of this hash disagree about what a document says, \
             which is exactly the disagreement a profile hash exists to make legible rather than \
             silent.\n\n\
             Moved an eleventh time at v1-S6.2 (0.8.2): the version alone. No field changed and no \
             capability moved, but the GEOMETRY did: a run that draws no ink now reports \
             `no_ink_to_measure` instead of a rectangle built from the font's envelope and the \
             run's advance. 150 425 nodes across the four real corpus documents lost a box that \
             was never ink, and two documents that could not be read at all — 491 of \
             nist-sp-800-53r5's 492 pages — produce artifacts again. Two artifacts either side of \
             this hash disagree about which nodes have geometry, which is a difference the hash \
             has to carry even though the profile's own shape is unchanged.\n\n\
             Moved a twelfth time at v1-S7b (0.9.0): the version alone, and this time NOTHING \
             else moved — not a field, not a capability, not a rule id, not a coordinate, not a \
             character. S7b set out to recalibrate `unruled-align-v1` and measured instead that \
             no value of its column-gutter floor changes any number on the corpus: with the floor \
             disabled outright the four real documents score identically. So the rule kept its \
             `-v1` id, which is the honest label for a rule that did not change. Two artifacts \
             either side of this hash say the same thing about the same document. The hash still \
             moves, because `parser_version` is in it and a version that claimed otherwise would \
             be the one lie this field cannot afford.\n\n\
             Moved a THIRTEENTH time at v1-S8 (0.10.0), and this time a field arrived: \
             `table_detection` grows `stroke_ruled`, because a third rule runs. An artifact \
             either side of this hash says something different about the same document — \
             `cfpb-home-loan-toolkit` page 13's loan worksheet is a 7 x 4 table on one side and \
             absent on the other — which is exactly what a profile identity is for.\n\n\
             Moved a FOURTEENTH time at v1.1-S1 (0.11.0), and this time TWO fields arrived: \
             `markdown_rule` and `capabilities.markdown`. v1.1 adds an OUTPUT rather than \
             changing a detector — `ethos.markdown.v1`, a Markdown projection carried with the \
             Anchor Map that inverts it back to nodes. A profile predating it is REFUSED rather \
             than defaulted, for the reason `table_detection.stroke_ruled` is: a field defaulted \
             in is a claim the run never made."
        );
        assert_eq!(
            Profile::default().profile_sha256().unwrap().to_string(),
            "sha256:881474f72ab0ede8f04078e87828fe4e5e4fb92c646a7def5d8981415057875d"
        );
    }

    /// All three rules are named on the profile, and any one moving moves the identity.
    ///
    /// The reason `table_detection` stopped being a string at v1-S2, extended to the third rule at
    /// v1-S8. With one id, an artifact could not distinguish "looked for this kind of table and
    /// found none" from "never looked", and a reader comparing two such artifacts cell for cell
    /// would be comparing different detectors.
    #[test]
    fn the_profile_names_every_table_rule_and_any_one_moves_the_hash() {
        let base = Profile::default();
        assert_eq!(base.table_detection.ruled, TABLE_DETECTION_V2);
        assert_eq!(base.table_detection.unruled, TABLE_DETECTION_UNRULED_V1);
        assert_eq!(base.table_detection.stroke_ruled, TABLE_DETECTION_STROKE_V1);
        for (a, b) in [
            (TABLE_DETECTION_V2, TABLE_DETECTION_UNRULED_V1),
            (TABLE_DETECTION_V2, TABLE_DETECTION_STROKE_V1),
            (TABLE_DETECTION_UNRULED_V1, TABLE_DETECTION_STROKE_V1),
        ] {
            assert_ne!(
                a, b,
                "two inferences sharing one identity is what a versioned rule id exists to prevent"
            );
        }

        // All three appear on the wire, under their own keys.
        let s = String::from_utf8(base.canonical_bytes().unwrap()).unwrap();
        assert!(
            s.contains(
                r#""table_detection":{"ruled":"ruled-rects-v2","stroke_ruled":"stroke-ruled-v1","unruled":"unruled-align-v1"}"#
            ),
            "{s}"
        );

        let mut ruled_moved = base.clone();
        ruled_moved.table_detection.ruled = "ruled-rects-v3".into();
        assert_ne!(hash(&base), hash(&ruled_moved), "the ruled id is identity");

        let mut unruled_moved = base.clone();
        unruled_moved.table_detection.unruled = "unruled-align-v2".into();
        assert_ne!(
            hash(&base),
            hash(&unruled_moved),
            "and so is the unruled one — a run that inferred grids from alignment produced \
             different tables from one that did not"
        );

        // And the two knobs are not each other: moving one must not produce the other's digest.
        assert_ne!(hash(&ruled_moved), hash(&unruled_moved));
    }

    /// An unknown rule id fails closed rather than deserializing with the key dropped.
    ///
    /// `deny_unknown_fields` on a plain struct denies for real. v0.1 measured that an
    /// internally-tagged enum does not: it buffers through a map and discards keys it does not
    /// know, so a profile from a newer engine would re-hash to a digest different from the one it
    /// arrived with — claiming a comparability it does not have.
    #[test]
    fn an_unknown_table_rule_key_is_refused() {
        let bad = r#"{"ruled":"ruled-rects-v2","unruled":"unruled-align-v1","stroke_ruled":"stroke-ruled-v1","tagged":"x-v1"}"#;
        assert!(
            serde_json::from_str::<TableDetection>(bad).is_err(),
            "a fourth rule id must fail closed, not vanish and change the hash"
        );

        // And a profile MISSING the field v1-S8 added is refused too, rather than defaulted into
        // one that claims a rule it never ran.
        let stale = r#"{"ruled":"ruled-rects-v2","unruled":"unruled-align-v1"}"#;
        assert!(
            serde_json::from_str::<TableDetection>(stale).is_err(),
            "a pre-S8 profile must not silently acquire the stroke-ruled rule"
        );

        let good = r#"{"ruled":"ruled-rects-v2","unruled":"unruled-align-v1","stroke_ruled":"stroke-ruled-v1"}"#;
        assert_eq!(
            serde_json::from_str::<TableDetection>(good).unwrap(),
            TableDetection::default()
        );
    }

    #[test]
    fn the_hash_is_stable_across_calls_and_clones() {
        let p = Profile::default();
        assert_eq!(hash(&p), hash(&p));
        assert_eq!(hash(&p), hash(&p.clone()));
    }

    #[test]
    fn the_hash_is_well_formed() {
        let d = Profile::default().profile_sha256().unwrap();
        assert!(d.as_str().starts_with("sha256:"));
        assert_eq!(d.hex().len(), 64);
        assert!(d
            .hex()
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    }

    #[test]
    fn the_free_function_matches_the_method() {
        let p = Profile::default();
        assert_eq!(profile_sha256(&p).unwrap(), p.profile_sha256().unwrap());
    }

    #[test]
    fn v0_capabilities_are_honest_about_what_is_missing() {
        let c = Capabilities::V0;
        // `tables` flipped to true at v1-S1. What it claims is "this profile looked", not "every
        // table is found". v1-S2 widened what "looked" covers: ruled AND unruled, which is why
        // the blanket `unruled-tables-not-detected` limitation is gone rather than reworded.
        assert!(c.tables, "v1-S2 looks for ruled and unruled tables");
        // Flipped at v1-S5. v0 through v1-S4 left this false and said why on every artifact: no
        // stable rule existed, and the known one flipped on a line count. `gutter-columns-v1`
        // reads whitespace, so the claim this flag makes — *this profile orders by geometry* —
        // is now true. It still does not claim every layout is resolved: a page with no gutter
        // is left in content-stream order, and `reading-order-geometric-only` says so.
        assert!(
            c.multi_column_reading_order,
            "v1-S5 orders by page geometry under a versioned rule"
        );
        assert!(
            !c.char_offsets,
            "v0 emits runs with no element/span hierarchy, so there is nothing an offset could \
             index into. M5 built the record and left this false: with no line grouping an \
             element and a span are the same object, so an offset would always be 0..len. It \
             flips at v1 with grouping — and with a test"
        );
        // Flipped at v1-S3. Through v1-S2 this was false and the reason was exact: an `mcid`
        // captured from `BDC` is not a structural address, because with the tree unread it
        // resolves against nothing. S3 reads `/StructTreeRoot`, so the claim this flag makes —
        // *this profile looks* — is now true. It still does not claim every document has
        // structure: an untagged file declares `untagged-structure-tree-absent` and gets no
        // roles, which is a real answer rather than a gap.
        assert!(
            c.structural_locators,
            "v1-S3 reads the tagged-structure tree and binds runs to it by (page, mcid)"
        );
    }

    #[test]
    fn the_default_budget_is_unlimited_and_says_so_on_the_wire() {
        assert_eq!(Profile::default().page_budget, PageBudget::Unlimited);
        assert_eq!(PageBudget::Unlimited.max_pages_to_process(), None);
        assert_eq!(PageBudget::AtMost(3).max_pages_to_process(), Some(3));

        // Declared, not omitted: a reader must not have to infer "unbounded" from a missing key.
        let s = String::from_utf8(Profile::default().canonical_bytes().unwrap()).unwrap();
        assert!(s.contains(r#""page_budget":{"mode":"unlimited"}"#), "{s}");
    }

    #[test]
    fn a_budget_admits_pages_up_to_its_bound_and_no_further() {
        assert!(PageBudget::Unlimited.admits(1));
        assert!(PageBudget::Unlimited.admits(u32::MAX));

        let b = PageBudget::AtMost(2);
        assert!(b.admits(1), "pages are 1-based");
        assert!(b.admits(2));
        assert!(!b.admits(3));

        // Zero admits nothing — the only budget that bites on a one-page document.
        assert!(!PageBudget::AtMost(0).admits(1));
    }

    #[test]
    fn canonical_bytes_contain_no_host_varying_data() {
        let s = String::from_utf8(Profile::default().canonical_bytes().unwrap()).unwrap();
        for forbidden in [
            "/Users",
            "/home",
            "/tmp",
            "timestamp",
            "elapsed",
            "hostname",
        ] {
            assert!(
                !s.contains(forbidden),
                "profile must not carry host-varying data, found {forbidden}: {s}"
            );
        }
    }

    /// An unknown field is refused rather than dropped.
    ///
    /// The failure this prevents: a profile from a newer engine deserializes with its unknown
    /// knob silently discarded, then re-hashes to a *different* digest than it arrived with —
    /// so an artifact claims comparability it does not have.
    #[test]
    fn an_unknown_profile_field_fails_closed() {
        let mut v = serde_json::to_value(Profile::default()).unwrap();
        v.as_object_mut()
            .unwrap()
            .insert("future_knob".into(), serde_json::Value::from(1));
        let bytes = crate::c14n::c14n_bytes(&v).unwrap();

        let parsed: Result<Profile, _> = serde_json::from_slice(&bytes);
        assert!(
            parsed.is_err(),
            "a profile carrying an unknown field must be refused, not silently truncated"
        );
    }

    /// Every nested object in `Profile`, not just the ones someone remembered.
    ///
    /// `deny_unknown_fields` is **not recursive**. This test originally covered `capabilities`
    /// and `backend` and missed `coordinate_system`, which left the exact hole it was written to
    /// prevent: a profile carrying `coordinate_system.future_knob` parsed cleanly and re-hashed
    /// to the unmodified default digest. The list below is derived from the serialized value, so
    /// a nested object added later is covered automatically rather than by memory.
    #[test]
    fn an_unknown_nested_field_also_fails_closed() {
        let default = serde_json::to_value(Profile::default()).unwrap();
        let nested: Vec<String> = default
            .as_object()
            .expect("profile is an object")
            .iter()
            .filter(|(_, v)| v.is_object())
            .map(|(k, _)| k.clone())
            .collect();

        assert!(
            nested.len() >= 4,
            "expected at least backend, capabilities, coordinate_system and page_budget as \
             nested objects; found {nested:?}"
        );
        for required in [
            "backend",
            "capabilities",
            "coordinate_system",
            "page_budget",
        ] {
            assert!(
                nested.iter().any(|n| n == required),
                "`{required}` must be among the nested objects under test; found {nested:?}"
            );
        }

        for path in &nested {
            let mut v = serde_json::to_value(Profile::default()).unwrap();
            v[path]
                .as_object_mut()
                .unwrap()
                .insert("future_knob".into(), serde_json::Value::from(true));
            let parsed: Result<Profile, _> = serde_json::from_value(v);
            assert!(
                parsed.is_err(),
                "an unknown field inside `{path}` must be refused; accepting it means the knob \
                 is dropped and the profile re-hashes as though it never existed"
            );
        }
    }

    /// The failure that makes nested unknown fields a correctness bug, not a tidiness one.
    ///
    /// If a truncating parse were ever allowed, the reconstructed profile would produce a digest
    /// identical to the default — so an artifact would claim comparability with a profile it does
    /// not match. This asserts the parse is refused *and* records why it has to be.
    #[test]
    fn a_truncated_profile_can_never_reproduce_the_default_digest() {
        let baseline = Profile::default().profile_sha256().unwrap();

        for path in [
            "backend",
            "capabilities",
            "coordinate_system",
            "page_budget",
        ] {
            let mut v = serde_json::to_value(Profile::default()).unwrap();
            v[path]
                .as_object_mut()
                .unwrap()
                .insert("future_knob".into(), serde_json::Value::from(7));

            match serde_json::from_value::<Profile>(v) {
                Err(_) => {} // correct: refused before it could be re-hashed
                Ok(truncated) => panic!(
                    "a profile with an unknown field in `{path}` parsed, and re-hashed to {} \
                     (default is {baseline}). Comparability is now a lie.",
                    truncated.profile_sha256().unwrap()
                ),
            }
        }
    }

    #[test]
    fn the_profile_round_trips() {
        let p = Profile::default();
        let bytes = p.canonical_bytes().unwrap();
        let back: Profile = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.profile_sha256().unwrap(), p.profile_sha256().unwrap());
    }
}
