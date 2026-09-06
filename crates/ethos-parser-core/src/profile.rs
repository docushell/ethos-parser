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
/// Its constants live with the rule, in `ethos-parser-pdf`'s `reading_order` module, and changing any
/// of them is a new id rather than a quiet redefinition of this one.
pub const READING_ORDER_RULE_V1: &str = "gutter-columns-v1";

/// The same cut, now reporting the regions it made (D4-S2).
///
/// # What changed, and what did not
///
/// **The rule did not.** Every constant is where `gutter-columns-v1` put it, the recursion is the
/// same recursion, and the order a page comes out in is byte-identical — the fifteen ordering
/// tests in the `reading_order` module were not edited to make that true. What changed is that the
/// cut stops discarding its own grouping: each text run carries the 1-based region it landed in,
/// as [`crate::TextRunAttributes::region`], absent where the cut made no division.
///
/// # Why the id moves anyway
///
/// Because the artifact does. [`crate::HTML_RULE_BLOCKS_V5`] settled this repository's answer when
/// it moved an id nothing had ever published under: *two builds in this repository's own history
/// producing different bytes under one id* is the state a rule id exists to make impossible, and
/// *a version that is cheap to move is exactly the one worth moving*. `docs/01-CONTRACT.md` §2
/// states the general form — **anything that can change a byte of output belongs in the profile,
/// or it is a bug** — and a rule id is how this particular knob reaches the profile.
///
/// So the reasoning is the opposite of [`READING_ORDER_RULE_V0`]'s. That id kept its spelling
/// because its *meaning* never changed while a different rule appeared beside it. This one moves
/// because its meaning did widen, even though its ordering did not: an artifact naming
/// `gutter-columns-v1` promises no region field, and one naming `gutter-columns-v2` promises the
/// field wherever a page divided. A consumer that cannot tell those apart cannot tell an
/// undivided page from an older build.
///
/// # Why a version bump and not a new name
///
/// The mirror of [`READING_ORDER_RULE_V1`]'s "why it is not `single-column-v2`". That id refused a
/// bump because it read *different evidence*. This reads exactly the same evidence — whitespace in
/// page space — and reports more of what it found, which is what a version bump means.
///
/// # No second id for the regions
///
/// [`crate::HTML_RULE_BLOCKS_V5`] is separate from the Markdown rule because those two can move
/// independently. The order and the regions cannot: one cut emits both, and a change to the cut
/// changes both together. Two ids for one rule would claim a precision that does not exist.
pub const READING_ORDER_RULE_V2: &str = "gutter-columns-v2";

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
/// Named here rather than in `ethos-parser-pdf` because the profile is `ethos-parser-core`'s and a rule id is
/// data. The rule itself — the lattice tolerance, what counts as a grid — lives with the detector.
///
/// **There are not two strings to keep in agreement, and this said a test asserted there were
/// until v2-S13.5.** `ethos-parser-pdf` does not restate the id: `tables.rs` says so in as many words
/// — *"the rule id lives in `ethos_parser_core::TABLE_DETECTION_V3` and is NOT restated here"* — and
/// builds its tables with `rule: ethos_parser_core::TABLE_DETECTION_V3.to_string()`. One constant, read
/// from one place, so the drift this described is structurally impossible rather than guarded.
/// Contrast [`crate::profile::XrefRepair::Pad19To20V1`], where the id genuinely **is** spelled
/// twice and nothing checks it.
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
///
/// **Superseded by [`TABLE_DETECTION_V3`] at v2-S20.** Kept, exactly as spelled, for the reason
/// [`TABLE_DETECTION_V1`] is kept: artifacts produced before that change name it.
pub const TABLE_DETECTION_V2: &str = "ruled-rects-v2";

/// The **ruled** table-detection rule v2-S20 ships: a table that fails its own cross-check is
/// refused rather than emitted.
///
/// # What changed, and why it is a new id rather than a fix
///
/// Under [`TABLE_DETECTION_V2`] the locator cross-check ([`crate::LOCATOR_CHECK_V1`]) was computed
/// on every emitted table and acted on by nothing. `nist-sp-800-218` is where that stopped being
/// theoretical: against **4** tagged tables it emitted **9** grids of up to 103 × 22, built from
/// page furniture its rectangles fold into one lattice, and **its own cross-check rejected every
/// one of the nine** — 1 028 structural faults on the first alone, each a slot two rectangles both
/// claim. Expanded to cell slots those nine contributed **11 295 false positives**, more than the
/// rest of the twelve-document gate corpus produces in either direction.
///
/// `-v3` refuses such a lattice, and the refusal is declared: the rejected candidate reaches the
/// artifact as `ruled-table-candidate-refused` with its page and its fault counts, so the
/// disagreement is reported rather than deleted. What it may no longer do is reach a consumer as
/// a grid, because `crate::markdown` and `crate::html` project every table on the wire and read
/// no check — a contradiction carried in a field neither projection consults is a disclosure in
/// name only.
///
/// **Measured on all twelve gate documents.** Those nine grids are the whole of the change: every
/// other detection in the corpus is unchanged, `irs-fw9` and `cfpb-home-loan-toolkit` keep every
/// cell they had, fabrication stays 0, and the three gold negatives stay at zero tables. The cost
/// is **12 cell slots on `nist-sp-800-218`**, and all twelve are the **empty string** — a blank
/// face of a phantom grid agreeing with a blank tagged cell. Not one character of extracted text
/// is lost anywhere in the corpus. `docs/table-gate-v1.md` carries the per-document table.
///
/// **The macro cannot see any of it**: 70‰ before, 70‰ after. That is a property of an average
/// over mostly zeros, not evidence the change did nothing, and it is why this decision is argued
/// from the artifact rather than from the gate.
pub const TABLE_DETECTION_V3: &str = "ruled-rects-v3";

/// The **unruled** table-detection rule v1-S2 ships: grids inferred from text alignment.
///
/// A separate id from [`TABLE_DETECTION_V1`], not a bump of it. The two answer different
/// questions about a document:
///
/// | Rule | Evidence | What it means when it fires |
/// | --- | --- | --- |
/// | `ruled-rects-v3` | rectangles the author **painted** | the document drew this grid |
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
/// A third id rather than a widening of [`TABLE_DETECTION_V3`], for the reason the ruled and
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

/// The **tagged** table-detection rule v2-S24 ships: emit a table for each `/Table` the structure
/// tree declares that no geometric detector matched.
///
/// A fourth id rather than a widening of any other, for the reason the first three are separate:
/// the evidence differs, and it differs in the direction that inverts the usual intuition. The
/// three geometric rules read **ink** — painted rectangles, stroked lines, text origins — and infer
/// a grid over it, so their tables are [`crate::DerivationClass::Computed`]. This rule reads the
/// document's **own tags**: `/TR`, `/TD`, `/TH`, `/RowSpan`, `/ColSpan`, with cell text joined from
/// the `/MCID`s beneath each cell. That is the document *stating* its table rather than the engine
/// inferring one, so a tagged table is [`crate::DerivationClass::Extracted`] — a stronger claim
/// than any geometric table, not a weaker one.
///
/// It carries **no geometry**. The tree names structure and never a coordinate, so the table's box
/// is [`crate::GeometryPresence::Absent`] with [`crate::GeometryAbsence::NotReportedByStructureTree`]
/// and no rectangle is invented. Its locator cross-check ([`crate::LOCATOR_CHECK_V1`]) is therefore
/// `NotApplicable`: that check compares a geometric derivation against a structural one, and with
/// no geometry there is nothing to compare.
///
/// v2-S22 measured the gap this closes: `unruled-align-v1` emitted 0 of 172 gold tables, and the
/// two working geometric rules require the producer to have drawn the grid — which the NIST
/// producers do not. Reading the tags recovers the tables the documents declare rather than the
/// grids they draw.
pub const TABLE_DETECTION_TAGGED_V1: &str = "tagged-tables-v1";

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
/// **Named for what it does, not for the structure it reads.** `ethos-parser-core` carries no PDF
/// concept (`docs/04-ARCHITECTURE.md` §1, and a test enforces it), so the field and the id name
/// *form fields and annotations* — document ideas any format can have — while the format-specific
/// walk lives in `ethos-parser-pdf`. The same split `table_detection` already uses: a generic field
/// holding `"ruled-rects-v3"`.
pub const FORM_ANNOTATION_RULE_V1: &str = "form-annotations-v1";

/// Identity of the character-decoding data this profile carries.
///
/// Names what is **actually** vendored rather than what was planned. At M3 that is the
/// PDF 32000-1 Annex D encoding tables — `WinAnsiEncoding`, the ASCII range of
/// `StandardEncoding`, and a glyph-name subset — held in `ethos-parser-pdf`'s `encoding` module.
///
/// The Adobe predefined CJK CMaps are **not** carried, so a document naming one is refused
/// rather than decoded approximately. That is a declared limitation: it travels as the code
/// `predefined-cmaps-not-vendored` in `assurance.limitations`, and it is argued in
/// `vendor/README.md`. The list was called `not_decoded` at M3 and M4 absorbed it into the L1
/// gate, so the old name named nothing from M4 until this sentence was repaired at v2-S13.3.
/// Being profile-scoped it rides on **classify and extract alike**, not only on an extract —
/// and on PDF artifacts only, because the office readers build their limitation lists fresh and
/// have no PDF backend to declare anything about. When those files land this string
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
    /// See [`TABLE_DETECTION_V3`].
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
    /// Version id of the rule that emits a table for each `/Table` the structure tree declares that
    /// no geometric detector matched (v2-S24).
    ///
    /// See [`TABLE_DETECTION_TAGGED_V1`]. A fourth field rather than a widening of any other, for
    /// the reason there were three: the evidence differs. The first three read ink and infer a grid
    /// (`Computed`); this one reads the document's own tags and reports what they state
    /// (`Extracted`), carrying no geometry. Its arrival is what the `table_detection` doc comment
    /// anticipated — *"a future slice that retires or adds one changes this shape and moves the
    /// hash"* — and moving the hash is correct: two builds disagree about whether a NIST document's
    /// tables reach the artifact.
    pub tagged: String,
}

impl Default for TableDetection {
    /// All four rules, enabled — the two v1-S2 shipped, the one v1-S8 added, and the one v2-S24 did.
    fn default() -> Self {
        Self {
            ruled: TABLE_DETECTION_V3.to_string(),
            unruled: TABLE_DETECTION_UNRULED_V1.to_string(),
            stroke_ruled: TABLE_DETECTION_STROKE_V1.to_string(),
            tagged: TABLE_DETECTION_TAGGED_V1.to_string(),
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
/// 1. **No `true` without a proof test.** `crates/ethos-parser-pdf/tests/capabilities.rs` maps every
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
    /// An HTML projection is emitted, always with its Anchor Map (v1.1-S4).
    ///
    /// **True since v1.1-S4**, and it claims the same narrow thing [`Self::markdown`] does — *this
    /// profile can project the representation into HTML that stays citable*. Separate from
    /// `markdown` because the two artifacts say different things about the same document: GFM has
    /// to flatten a merged cell and count what that cost, `<td rowspan>` carries it. A consumer
    /// choosing between them is choosing which erasures it can afford, and one flag could not
    /// tell it that either was available.
    pub html: bool,
}

impl Capabilities {
    /// What v0 actually claims.
    ///
    /// Two of these were `true` in the M1 sketch and were narrowed to `false` at M4, because M4
    /// asked for the proof and the proof did not exist: `char_offsets` had no hierarchy to index
    /// into until grouping landed, and `structural_locators` would have been claiming a full
    /// structural address on the strength of a best-effort `mcid`. Narrowing a declaration when
    /// the evidence does not support it is the mechanism working, not a regression.
    ///
    /// **`structural_locators` has been `true` since v1-S3**, when the tagged-structure tree
    /// landed and the address stopped being best-effort. `char_offsets` is still `false`. This
    /// comment opened *"Note how much is `false`"* and described both as narrowed, three lines
    /// above a literal that had said `structural_locators: true` since v1-S3 — repaired at
    /// v2-S13.3.
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
        html: true,
    };
}

/// The value a rule field carries when that rule does not run for this profile's format (v2-S2).
///
/// **A declared state, not an empty string and not a PDF rule id borrowed for the shape.** The
/// same discipline `PageBudget::Unlimited` and `RasterDpi::NotEmitted` are under: a reader of a
/// DOCX profile can see that no table detector ran, rather than seeing `ruled-rects-v3` and
/// wondering whether it did.
pub const NOT_RUN: &str = "not-run-for-this-format";

/// v2-S2's DOCX reading order: the document order of `word/document.xml`.
///
/// **Not a rule that decides anything.** OOXML states its own order; this engine reads runs in the
/// order the part lists them and does no column detection, no sorting and no grouping. The id
/// exists so an artifact says which order it was read in, the way every other rule id does.
pub const DOCX_READING_ORDER_RULE_V1: &str = "docx-document-order-v1";

/// v2-S2's DOCX text rule: the characters `<w:t>` carries, verbatim.
///
/// The DOCX counterpart to `declared-font-codes-v1`, and a much smaller claim: OOXML text is
/// already Unicode, so there is no glyph-code-to-scalar step to get wrong and no ligature caveat
/// to declare.
/// v2 (0.38.0): numeric character references resolve. `&#233;` is a scalar written
/// another way (XML 1.0 §4.1, no DTD required), and the hardened resolver the EPUB
/// reader shipped at v2-S9 now serves this reader too — in text and in the names
/// attributes carry. Named entities beyond the five predefined stay refused: `&nbsp;`
/// is an HTML name an XML parser without a DTD cannot resolve. The id moves because
/// the behaviour it names moves: a document this rule refused at v1 now reads.
pub const DOCX_TEXT_CODE_RULE_V2: &str = "docx-wt-verbatim-v2";

/// v2-S3's XLSX reading order: sheets in the order `xl/workbook.xml` lists them, cells in the
/// order their worksheet part lists them.
///
/// **Not a rule that decides anything**, for the same reason [`DOCX_READING_ORDER_RULE_V1`] is
/// not. The workbook states its own sheet order and each sheet states its own cell order; this
/// engine follows both and sorts nothing. In particular it does **not** re-order cells into
/// row-major address order — a sheet whose part lists `B1` before `A1` is read that way, because
/// the alternative is this engine deciding a reading order the file did not state.
pub const XLSX_READING_ORDER_RULE_V1: &str = "xlsx-workbook-then-sheet-order-v1";

/// v2-S3's XLSX text rule: the characters the cell's stored value carries, verbatim.
///
/// Shared strings are resolved by index, inline strings are taken from `<is>`, and everything
/// else is the `<v>` as stored. **No number formatting is applied**: `42` under a currency format
/// is `42` here, because `$42.00` is a string `xl/styles.xml` would have to be read and *run* to
/// produce, and a rendered string is not a stored one.
/// v2 (0.38.0): numeric character references resolve. `&#233;` is a scalar written
/// another way (XML 1.0 §4.1, no DTD required), and the hardened resolver the EPUB
/// reader shipped at v2-S9 now serves this reader too — in text and in the names
/// attributes carry. Named entities beyond the five predefined stay refused: `&nbsp;`
/// is an HTML name an XML parser without a DTD cannot resolve. The id moves because
/// the behaviour it names moves: a document this rule refused at v1 now reads.
pub const XLSX_TEXT_CODE_RULE_V2: &str = "xlsx-stored-value-verbatim-v2";

/// v2-S4's PPTX reading order: slides in the order `ppt/presentation.xml` lists them, shapes and
/// runs in the order each slide part lists them.
///
/// **Not a rule that decides anything**, for the reason its two siblings are not. The
/// presentation states its own slide order and each slide states its own shape order; this engine
/// follows both and sorts nothing. In particular it does **not** re-order shapes into reading
/// order on the slide canvas — where a shape sits is a position this engine did not read, and
/// sorting by it would be a layout decision wearing a rule id.
pub const PPTX_READING_ORDER_RULE_V1: &str = "pptx-presentation-then-slide-order-v1";

/// v2-S4's PPTX text rule: the characters `<a:t>` carries, verbatim.
///
/// The DrawingML counterpart to `docx-wt-verbatim-v2`, and the same small claim: the text is
/// already Unicode, so there is no glyph-code step to get wrong. **No placeholder inheritance is
/// resolved**: text that a slide layout or master would supply is not substituted in, because
/// this reader did not read those parts and a substituted string is not one the slide stated.
/// v2 (0.38.0): numeric character references resolve. `&#233;` is a scalar written
/// another way (XML 1.0 §4.1, no DTD required), and the hardened resolver the EPUB
/// reader shipped at v2-S9 now serves this reader too — in text and in the names
/// attributes carry. Named entities beyond the five predefined stay refused: `&nbsp;`
/// is an HTML name an XML parser without a DTD cannot resolve. The id moves because
/// the behaviour it names moves: a document this rule refused at v1 now reads.
pub const PPTX_TEXT_CODE_RULE_V2: &str = "pptx-at-verbatim-v2";

/// v2-S5's ODT reading order: paragraphs in the order `content.xml` lists them.
///
/// **Not a rule that decides anything**, for the reason its three siblings are not. One part, one
/// order, stated by the file. In particular the order is **not** the order a reader would
/// encounter the paragraphs on a rendered page: `<text:soft-page-break/>` records where the
/// producing application broke a page, and this rule neither reads it nor sorts by it.
pub const ODT_READING_ORDER_RULE_V1: &str = "odt-content-document-order-v1";

/// v2-S5's ODT text rule: the characters the paragraph's own content states, verbatim.
///
/// Verbatim with three exceptions, and each of them is a character the file spells as an element
/// because XML would otherwise collapse it: `<text:s text:c="n">` is **n** spaces — the count the
/// file states, never a count inferred from where anything sits — `<text:tab/>` is a tab and
/// `<text:line-break/>` is a line feed. Nothing else is substituted: a field's cached rendering, a
/// list's number and a footnote's mark are all produced by a layout this reader does not perform.
/// v2 (0.38.0): numeric character references resolve. `&#233;` is a scalar written
/// another way (XML 1.0 §4.1, no DTD required), and the hardened resolver the EPUB
/// reader shipped at v2-S9 now serves this reader too — in text and in the names
/// attributes carry. Named entities beyond the five predefined stay refused: `&nbsp;`
/// is an HTML name an XML parser without a DTD cannot resolve. The id moves because
/// the behaviour it names moves: a document this rule refused at v1 now reads.
pub const ODT_TEXT_CODE_RULE_V2: &str = "odt-text-content-verbatim-v2";

/// v2-S6's ODS reading-order rule: the cells in the part's own document order (v2-S6).
///
/// **Not row-major over a grid**, which is the shape a reader would get by sorting on the address.
/// The two coincide for every document a producer writes, and they stop coinciding the moment one
/// does not — and the file's order is the one this engine read. The address is stated separately,
/// on [`OdsLocator`](crate::OdsLocator), so a consumer that wants grid order can sort and know it
/// did.
pub const ODS_READING_ORDER_RULE_V1: &str = "ods-content-document-order-v1";

/// v2-S6's ODS text rule: the cell's own blocks, verbatim, joined by a line feed.
///
/// [`ODT_TEXT_CODE_RULE_V2`]'s three exceptions apply unchanged, because it is the same engine
/// reading the same `<text:p>`. The one addition is the join: a cell holding two paragraphs
/// displays two lines, and the line feed is what the file states by writing two blocks rather than
/// one. Nothing here reads `office:value` — the text is what the document **displays**, and the
/// stored typed value is a separate declared fact.
/// v2 (0.38.0): numeric character references resolve. `&#233;` is a scalar written
/// another way (XML 1.0 §4.1, no DTD required), and the hardened resolver the EPUB
/// reader shipped at v2-S9 now serves this reader too — in text and in the names
/// attributes carry. Named entities beyond the five predefined stay refused: `&nbsp;`
/// is an HTML name an XML parser without a DTD cannot resolve. The id moves because
/// the behaviour it names moves: a document this rule refused at v1 now reads.
pub const ODS_TEXT_CODE_RULE_V2: &str = "ods-cell-blocks-verbatim-v2";

/// v2-S7's ODP reading order: draw pages, shapes and blocks in the part's own document order.
///
/// **Not a rule that decides anything**, for the reason its five siblings are not. One part, one
/// order, stated by the file. In particular it is **not** reading order on the slide canvas: where
/// a shape sits is a position this engine did not read, and sorting by it would be a layout
/// decision wearing a rule id — [`PPTX_READING_ORDER_RULE_V1`]'s sentence, and it survives the
/// change of vocabulary because the temptation does.
pub const ODP_READING_ORDER_RULE_V1: &str = "odp-content-document-order-v1";

/// v2-S7's ODP text rule: the block's own content, verbatim.
///
/// [`ODT_TEXT_CODE_RULE_V2`] unchanged — the same engine reading the same `<text:p>`, so
/// `<text:s text:c="n">` is n spaces, `<text:tab/>` is a tab and `<text:line-break/>` is a line
/// feed. **No placeholder inheritance is resolved**: text a master page or a presentation layout
/// would supply is not substituted in, because this reader did not read those and a substituted
/// string is not one the draw page stated. That is [`PPTX_TEXT_CODE_RULE_V2`]'s claim in ODF's
/// spelling, and it is why a slide whose title lives only on its master reads as having none.
/// v2 (0.38.0): numeric character references resolve. `&#233;` is a scalar written
/// another way (XML 1.0 §4.1, no DTD required), and the hardened resolver the EPUB
/// reader shipped at v2-S9 now serves this reader too — in text and in the names
/// attributes carry. Named entities beyond the five predefined stay refused: `&nbsp;`
/// is an HTML name an XML parser without a DTD cannot resolve. The id moves because
/// the behaviour it names moves: a document this rule refused at v1 now reads.
pub const ODP_TEXT_CODE_RULE_V2: &str = "odp-shape-blocks-verbatim-v2";

/// v2-S8's RTF reading order: paragraphs in the stream's own order.
///
/// **Not a rule that decides anything**, for the reason its six siblings are not. One stream, one
/// order, stated by the file. In particular it is **not** the order a reader meets the paragraphs
/// on a printed page: `\page` and `\sect` record where the producing application broke one, and
/// this rule neither reads them as pages nor sorts by them.
pub const RTF_READING_ORDER_RULE_V1: &str = "rtf-stream-document-order-v1";

/// v2-S8's RTF text rule: the characters the stream states, and **nothing decoded from a code
/// page**.
///
/// Three sources, all of them things the file writes down: plain 7-bit characters, `\uN` as the
/// Unicode scalar it names, and the small closed set of special-character control words (`\tab`,
/// `\emdash`, `\lquote`, …) that stand for exactly one character each.
///
/// **`\'hh` above 0x7F is declared, never guessed.** The byte's meaning depends on a code page
/// this reader does not read and does not carry a table for, so it contributes no character and is
/// counted instead. Emitting a Latin-1 character for it would be mojibake presented as a success,
/// which is the failure `docs/01-CONTRACT.md` §5.2 calls worse than an absent value.
pub const RTF_TEXT_CODE_RULE_V1: &str = "rtf-stated-characters-v1";

/// v2-S9's EPUB reading order: **the spine**, then each document's own order.
///
/// **The first reading-order rule in this contract that decides something**, and that is the
/// format's doing rather than a change of posture. Its **seven** siblings say "the order the part
/// lists them" — DOCX, XLSX, PPTX, ODT, ODS, ODP and RTF — because each of those formats has one
/// part, names its parts in one place, or is a single stream. An EPUB is a
/// ZIP of documents, and the archive's own ordering is not the publication's: reading order lives
/// in the package document's `<spine>`, as `<itemref idref="…">` resolved through the
/// `<manifest>`. Following it is reading; sorting the XHTML entries by name, or taking them in
/// central-directory order, is a guess that attaches the right content to the wrong position.
///
/// Within a document the rule is its seven siblings' again: the order the file lists its blocks,
/// with nothing sorted and nothing laid out.
///
/// **Both numbers were wrong from v2-S9 to v2-S13.3**, and they were wrong when written rather
/// than overtaken: RTF's rule shipped at v2-S8 and ODP's at v2-S7, so seven sibling rule ids
/// already existed the day this constant was added. They also disagreed with each other by two,
/// which is the tell — a count nobody could check against anything, written twice.
pub const EPUB_READING_ORDER_RULE_V1: &str = "epub-spine-then-document-order-v1";

/// v2-S9's EPUB text rule: the block's own character data, under XHTML's whitespace rule.
///
/// The rule XHTML states for `white-space: normal` — a run of spaces, tabs, carriage returns and
/// line feeds is one space, and one at either end of a block is not part of it — which is
/// character-for-character the rule [`ODT_TEXT_CODE_RULE_V2`] already implements, so the engine is
/// shared rather than restated. `<pre>` is the one divergence and it is read as the file writes
/// it, because `white-space: pre` is the document saying those spaces are content.
///
/// **Nothing is styled and nothing is resolved.** A style sheet may hide a block, reorder it, or
/// insert generated text through `::before`; none of that is read, so the text here is what the
/// document *states* rather than what a reading system would display. `<br/>` is a line feed
/// because the element states one.
pub const EPUB_TEXT_CODE_RULE_V1: &str = "epub-xhtml-blocks-verbatim-v1";

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
/// The repair itself is `ethos_parser_pdf::xref` and is bounded to one malformation — 19-byte entries
/// where PDF 32000-1 §7.5.4 requires 20 — under preconditions that make it offset-preserving.
/// This enum only records *whether* it runs. `ethos-parser-core` owns no PDF machinery
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
    /// `ethos_parser_pdf::xref::XREF_REPAIR_V1`. Two spellings of one repair is exactly the drift a
    /// versioned id exists to prevent.
    ///
    /// **No test asserts the two strings are equal, and this said one did from v0.1 until
    /// v2-S13.5.** `git log --all -S'XREF_REPAIR_V1'` returns the single commit that introduced
    /// both the const and this sentence. `the_default_profile_is_pinned` pins the serde spelling
    /// inside the canonical JSON and never reads `ethos_parser_pdf`'s const. The assertion cannot live
    /// in this crate at all: `ethos-parser-pdf` depends on `ethos-parser-core`, so importing it back would be
    /// a dependency cycle — which is why the check belongs on the `ethos-parser-pdf` side, where
    /// `xref.rs` already has both strings in scope. It is a real unguarded seam: changing
    /// `XREF_REPAIR_V1` alone fails nothing today. Recorded in `docs/history/15-V2-MILESTONES.md` S13.5
    /// rather than papered over.
    #[serde(rename = "pad-19-to-20-v1")]
    Pad19To20V1,
    /// The format has no cross-reference table, so no repair policy applies (v2-S2).
    ///
    /// **A declared state rather than borrowing `Refuse`.** `Refuse` says this run would reject a
    /// malformation it might meet; a DOCX cannot meet one, and saying it would is a claim about
    /// machinery that never ran.
    #[serde(rename = "not-run-for-this-format")]
    NotRun,
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
/// from v0.1 `ethos-parser verify` spawns the Ethos CLI and relays its report bytes. That makes which
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
    /// See [`crate::markdown::MARKDOWN_RULE_BLOCKS_V5`]. On the profile because it decides what
    /// comes out: a run that projected headings from font sizes and a run that refused to would
    /// disagree about the same document, and an artifact whose hash could not tell them apart
    /// would claim a comparability it lacks.
    ///
    /// **A profile JSON predating v1.1-S1 — one with no `markdown_rule` — is refused, not
    /// defaulted.** The same posture `table_detection.stroke_ruled` took at v1-S8: a field
    /// defaulted in is a claim the run never made.
    pub markdown_rule: String,
    /// Version id of the HTML projection rule in force (v1.1-S4).
    ///
    /// See [`crate::html::HTML_RULE_BLOCKS_V5`]. A **separate** id from
    /// [`Self::markdown_rule`], and it moves independently: a change to how a `<td>` is spelled is
    /// not a change to how a GFM row is, and one id covering both would make two artifacts
    /// non-comparable every time either projection moved.
    ///
    /// **A profile JSON predating v1.1-S4 — one with no `html_rule` — is refused, not
    /// defaulted**, the same posture `markdown_rule` and `table_detection.stroke_ruled` took: a
    /// field defaulted in is a claim the run never made.
    pub html_rule: String,
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
            reading_order_rule: READING_ORDER_RULE_V2.to_string(),
            table_detection: TableDetection::default(),
            struct_tree_rule: STRUCT_TREE_RULE_V1.to_string(),
            markdown_rule: crate::markdown::MARKDOWN_RULE_BLOCKS_V5.to_string(),
            html_rule: crate::html::HTML_RULE_BLOCKS_V5.to_string(),
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
    /// The profile a page-less OOXML word-processing document is read under (v2-S2).
    ///
    /// **A separate profile so the two artifacts are provably non-comparable.** `14-V2-SCOPE.md`
    /// §8: a format is a new *value*, not a new mechanism, and the value that has to differ is
    /// `profile_sha256`. A DOCX read under the PDF profile would claim measured ink boxes,
    /// multi-column reading order and a table detector that never ran on it.
    ///
    /// Every capability below is `false` because **v2-S2 reads one thing**: the runs of
    /// `word/document.xml`. `spans` stays true because a run is a span, and that is the one claim
    /// this reader can make. Tables, images, annotations and form fields are real OOXML features
    /// this slice does not read, and the profile says so rather than letting a consumer infer
    /// capability from the PDF default. `markdown` and `html` are false because `14-V2-SCOPE.md`
    /// §4 does not teach those projections a second format in this slice.
    ///
    /// **`measured_ink_boxes: false` is the load-bearing one.** A DOCX has no geometry and this
    /// engine invents none, so every node's geometry is typed absence — and `check_structure`
    /// refuses a measured box on a page-less node outright.
    ///
    /// # One thing this profile declares that it cannot mean, named rather than papered over
    ///
    /// `coordinate_system` is a required field and there is no page-less spelling of it, so this
    /// profile carries the same `centipoint`/`top-left` pair a PDF does. **Nothing under this
    /// profile ever emits a coordinate** — `measured_ink_boxes` is false, every geometry row is
    /// absent, and `DocxLocator` has no geometry field — so the declaration is inert rather than
    /// wrong.
    ///
    /// **v2-S3 measured it and left it inert.** S2 deferred the question to the slice that would
    /// have a second page-less format to judge it by; that slice found **no consumer that acts on
    /// the value for a page-less artifact**. `ethos-parser-grounding::project` hard-codes the pair
    /// rather than copying it, and refuses a non-`application/pdf` source 179 lines earlier; the
    /// only branch on the value in the workspace is inside the `ethos.grounding.v1` validator,
    /// downstream of that same refusal; both SDKs re-hash the field as opaque bytes and never
    /// parse it. A mode enum would move every PDF artifact's `profile_sha256` to give an honest
    /// spelling to a value nothing reads. [`Self::xlsx_v0`] therefore carries the same inert
    /// declaration, and the pair is pinned by tests instead of respelled.
    pub fn docx_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            // A DOCX is not classified by sampling pages it does not have, and no table detector,
            // struct-tree reader, CMap or xref repair runs on it. These are PDF rules, and the
            // honest value for a rule a format never reaches is the declared "did not run".
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: DOCX_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: DOCX_TEXT_CODE_RULE_V2.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile a page-less OOXML **workbook** is read under (v2-S3).
    ///
    /// **Its own hash, not [`Self::docx_v0`]'s.** The two formats are read by two rules over two
    /// package shapes, and an artifact that could not tell a cell from a run apart by profile
    /// would claim a comparability it does not have — the same argument that separated
    /// `docx_v0` from [`Self::default`] at S2, applied one format further along.
    ///
    /// # `tables: false`, on a format made of grids
    ///
    /// The load-bearing declaration on this profile, and it is not modesty. `capabilities.tables`
    /// means *this run emitted this engine's table IR* — [`crate::tables::TableRecord`], with
    /// cell slots, spans and a locator cross-check. v2-S3 emits **cells**: one node per `<c>`,
    /// addressed by sheet, row and column. A workbook is obviously tabular and that is exactly
    /// why the claim has to stay false — a consumer reading `tables: true` would go looking for
    /// `TableRecord`s and find none, and the PDF table detectors that produce them never ran here
    /// and must not be pointed at a workbook.
    ///
    /// `measured_ink_boxes: false` is the other one, for the reason it is false on `docx_v0`: a
    /// cell has no ink box until something lays the sheet out, and `check_structure` refuses a
    /// measured box on a page-less node outright.
    ///
    /// `coordinate_system` carries the same inert `centipoint`/`top-left` pair `docx_v0` does,
    /// and the paragraph on that method is the decision and the evidence for it.
    pub fn xlsx_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: XLSX_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: XLSX_TEXT_CODE_RULE_V2.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile a page-less OOXML **presentation** is read under (v2-S4).
    ///
    /// **Its own hash**, for the reason `xlsx_v0` has one: three formats read by three rules over
    /// three package shapes, and an artifact that could not tell a slide run from a cell by
    /// profile would claim a comparability it does not have.
    ///
    /// # `measured_ink_boxes: false`, on the format that draws everything in boxes
    ///
    /// A slide *is* a canvas — every shape carries an `<a:xfrm>` with an offset and an extent in
    /// EMUs — and that is exactly why the declaration matters here. Those numbers describe where
    /// an authoring tool placed a shape, not where ink was measured, and `check_structure`
    /// refuses a measured box on a page-less node outright, so this profile could not emit one
    /// even if a later edit read `<a:xfrm>`.
    ///
    /// `coordinate_system` carries the same inert `centipoint`/`top-left` pair the other two
    /// page-less profiles do; `Profile::docx_v0`'s doc comment is the decision and the evidence,
    /// and v2-S4 did not reopen it.
    pub fn pptx_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: PPTX_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: PPTX_TEXT_CODE_RULE_V2.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile a page-less OpenDocument **text** document is read under (v2-S5).
    ///
    /// **Its own hash**, for the reason the other three page-less profiles have one: four formats
    /// read by four rules over three container shapes, and an artifact that could not tell an ODF
    /// paragraph from a `<w:r>` by profile would claim a comparability it does not have.
    ///
    /// # `measured_ink_boxes: false`, on the first format that writes its own page breaks down
    ///
    /// A DOCX has no page until a renderer invents one, a workbook's is a printer's and a slide is
    /// a part — but an ODT's `content.xml` contains `<text:soft-page-break/>`, a position the
    /// producing application computed and **stored**. That is the closest any v2 format comes to
    /// handing this engine a page for free, and it is still somebody else's rendering: it moves
    /// when the font stack, the paper size or the producer changes, which is precisely why
    /// `docs/06-STEAL-REFUSE.md` L30 refuses the LibreOffice bridge rather than treating it as a
    /// fallback. It is not read, `pages` stays empty, and `check_structure` refuses a measured box
    /// on a page-less node outright, so this profile could not emit one even if a later edit read
    /// the break.
    ///
    /// `coordinate_system` carries the same inert `centipoint`/`top-left` pair the other three
    /// page-less profiles do; [`Self::docx_v0`]'s doc comment is the decision and the evidence, and
    /// v2-S5 did not reopen it.
    pub fn odt_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: ODT_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: ODT_TEXT_CODE_RULE_V2.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile v2-S6's OpenDocument **spreadsheet** reader runs under.
    ///
    /// **Its own profile, and its own hash**, for §5.1's reason: an artifact from an `.ods` and one
    /// from an `.odt` are not comparable, and the hash is what makes saying so mechanical rather
    /// than a matter of trusting a media type. `capabilities.tables` is **false** and that is not
    /// modesty — a `TableRecord` is the PDF detector's finding about a grid it inferred, and a
    /// spreadsheet's cells are addresses the file states. Claiming `tables` would say a detector
    /// ran.
    pub fn ods_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: ODS_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: ODS_TEXT_CODE_RULE_V2.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile v2-S7's OpenDocument **presentation** reader runs under.
    ///
    /// **Its own profile and its own hash**, for §5.1's reason, and this is the format where the
    /// hash has to carry the most weight: an `.odp` is the one input from which a [`PageRecord`]
    /// could have been minted with no arithmetic at all — `<draw:page>` elements are discrete and
    /// ordered, a master page states `fo:page-width` — so the profile that says
    /// `measured_ink_boxes: false` and emits `pages: []` is the mechanical record that the
    /// temptation was refused rather than merely discussed.
    ///
    /// `coordinate_system` carries the same inert `centipoint`/`top-left` pair the other five
    /// page-less profiles do; [`Self::docx_v0`]'s doc comment is the decision and the evidence, and
    /// v2-S3 closed the question of a mode enum. `capabilities.tables` is **false**: this reader
    /// emits no [`crate::tables::TableRecord`] at all, and a true capability would say a detector
    /// ran.
    pub fn odp_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: ODP_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: ODP_TEXT_CODE_RULE_V2.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile v2-S8's Rich Text Format reader runs under.
    ///
    /// **Its own profile and its own hash**, for §5.1's reason, and this is the first one whose
    /// format is not a package at all: an `.rtf` is a brace-group byte stream with no parts, so an
    /// artifact from one is not comparable with an artifact from any package format and the hash
    /// is what makes saying so mechanical.
    ///
    /// `coordinate_system` carries the same inert `centipoint`/`top-left` pair the other six
    /// page-less profiles do; [`Self::docx_v0`]'s doc comment is the decision and the evidence, and
    /// v2-S3 closed the question of a mode enum. `capabilities.tables` is **false** even though
    /// RTF writes `\cell` and `\row`: those are recorded as a paragraph's terminator, where they
    /// are a fact the file states, and a [`crate::tables::TableRecord`] would say a detector ran.
    pub fn rtf_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: RTF_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: RTF_TEXT_CODE_RULE_V1.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

    /// The profile v2-S9's EPUB reader runs under.
    ///
    /// **Its own profile and its own hash**, for §5.1's reason, and this is the one where the hash
    /// carries a claim the others do not have to make: an EPUB is the first input this engine
    /// reads that could have supplied a page **from the file**. An EPUB 3 navigation document may
    /// carry a `page-list` naming the pages of a print edition, and §3's law is *no page this
    /// engine did not read from the file* rather than "no page ever" — so refusing it needed an
    /// argument rather than a rule. The argument is that a publisher's label about somebody else's
    /// paper has no width and no height, so nothing could be validated against it, and
    /// `measured_ink_boxes: false` with `pages: []` is the mechanical record of that.
    ///
    /// `coordinate_system` carries the same inert `centipoint`/`top-left` pair the other seven
    /// page-less profiles do; [`Self::docx_v0`]'s doc comment is the decision and the evidence, and
    /// v2-S3 closed the question of a mode enum. `capabilities.tables` is **false** even though
    /// XHTML has `<table>`: a cell's text is read as the block it is, and a
    /// [`crate::tables::TableRecord`] would say a detector ran.
    pub fn epub_v0() -> Self {
        Self {
            backend: BackendIdentity {
                name: "ethos-parser-office".into(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            capabilities: Capabilities {
                spans: true,
                char_offsets: false,
                tables: false,
                measured_ink_boxes: false,
                multi_column_reading_order: false,
                structural_locators: false,
                form_fields: false,
                annotations: false,
                images: false,
                page_screenshots: false,
                markdown: false,
                html: false,
            },
            classify_sample_pages: 0,
            table_detection: TableDetection {
                ruled: NOT_RUN.into(),
                unruled: NOT_RUN.into(),
                stroke_ruled: NOT_RUN.into(),
                tagged: NOT_RUN.into(),
            },
            reading_order_rule: EPUB_READING_ORDER_RULE_V1.to_string(),
            struct_tree_rule: NOT_RUN.into(),
            markdown_rule: NOT_RUN.into(),
            html_rule: NOT_RUN.into(),
            form_annotation_rule: NOT_RUN.into(),
            cmap_data_version: NOT_RUN.into(),
            text_code_rule: EPUB_TEXT_CODE_RULE_V1.to_string(),
            observation_rule: NOT_RUN.into(),
            xref_repair: XrefRepair::NotRun,
            ..Self::default()
        }
    }

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

/// Convenience wrapper matching the API named in `docs/history/05-MILESTONES.md` M1.
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

    /// Every field on `Profile` that is mutated below changes the hash when it changes.
    ///
    /// # What the destructuring enforces, and what it does not
    ///
    /// The binding below is a **compile-time gate on the type**: adding a field to `Profile`,
    /// `Capabilities`, `CoordinateSystem`, `BackendIdentity` or `TableDetection` fails to compile
    /// here until someone writes it into the pattern. That is real, and it is the reason a new
    /// field cannot land without a human reading this test.
    ///
    /// It does **not** enforce that the field is then mutated. Adding `new_field: _,` to the
    /// pattern satisfies the compiler and leaves the knob untested, and this comment used to
    /// claim otherwise — *"adding a field to `Profile` without adding a case here is a compile
    /// error, not a silently uncovered knob."* At v2-S13.0 the pattern had thirty-four leaves and
    /// the list below had twenty-four mutations covering twenty-three of them, so **eleven knobs
    /// were uncovered** while the sentence said none could be.
    ///
    /// # The eight that were repairable, and the three that are not
    ///
    /// Eight are now mutated here: `capabilities.images`, `capabilities.page_screenshots`,
    /// `capabilities.markdown`, `capabilities.html`, `text_code_rule`, `observation_rule`,
    /// `markdown_rule` and `html_rule`. Each moves the digest, none collides, and each was
    /// demonstrated rather than argued — which matters, because the argument available before
    /// was that [`the_default_profile_is_pinned`] pins the whole canonical JSON and every field
    /// appears in it by name. That proves each field *reaches* the hashed bytes. It is a
    /// different statement from "changing it changes the digest", and the gap between the two is
    /// exactly the kind a reader should not have to reconstruct.
    ///
    /// Three cannot be mutated at all, because their types have exactly one legal value:
    /// `coordinate_system`'s `unit` and `origin`, and `raster_dpi`, whose `RasterDpi` enum has
    /// the single variant `NotEmitted`. For those, "is it hashed?" is the only question
    /// available, and the assertions at the end of the neighbouring test answer it by finding
    /// each on the wire. A second `origin` or a second `RasterDpi` variant makes them mutable,
    /// and the pinned count below is what brings someone back here to do it.
    ///
    /// # What this test adds over the pin
    ///
    /// Per field: that a *changed* value moves the digest, that the mutation is not a no-op, and
    /// that no two mutations collide. The name is still narrower than it reads — thirty-two of
    /// thirty-five leaves since v2-S24 added `table_detection.tagged` and its mutation together —
    /// and saying so is the same repair v2-S12.1 made to
    /// `every_profile_is_distinct_from_every_other`, which checked four of nine while its name
    /// said every.
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
                    html: _,
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
                    tagged: _,
                },
            struct_tree_rule: _,
            markdown_rule: _,
            html_rule: _,
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
                // v2-S24. A fourth separate knob, for a different kind of evidence again: a run
                // that reads the document's own `/Table` tags emitted tables a run that read only
                // ink could not have, and every NIST document in the gate corpus is the measured
                // instance of exactly that.
                "table_detection.tagged",
                Box::new(|p: &mut Profile| p.table_detection.tagged = "other-tagged-v9".into()),
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
                // v0.1. `ethos-parser verify` spawns a verifier, so which one answered is part of the
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
            // The eight the pattern named and this list did not. Each is a knob that changes what
            // an artifact contains, and until v2-S13.1 nothing anywhere demonstrated that moving
            // it moves the digest — the pin in `the_default_profile_is_pinned` shows each field
            // reaches the canonical bytes, which is a different statement from this one.
            (
                "capabilities.images",
                Box::new(|p: &mut Profile| p.capabilities.images = false),
            ),
            (
                "capabilities.page_screenshots",
                Box::new(|p: &mut Profile| p.capabilities.page_screenshots = true),
            ),
            (
                "capabilities.markdown",
                Box::new(|p: &mut Profile| p.capabilities.markdown = false),
            ),
            (
                "capabilities.html",
                Box::new(|p: &mut Profile| p.capabilities.html = false),
            ),
            (
                "text_code_rule",
                Box::new(|p: &mut Profile| p.text_code_rule = "other-codes-v9".into()),
            ),
            (
                "observation_rule",
                Box::new(|p: &mut Profile| p.observation_rule = "other-observations-v9".into()),
            ),
            (
                "markdown_rule",
                Box::new(|p: &mut Profile| p.markdown_rule = "other-markdown-v9".into()),
            ),
            (
                "html_rule",
                Box::new(|p: &mut Profile| p.html_rule = "other-html-v9".into()),
            ),
        ];

        // Pinned, so shrinking the list is a decision someone makes here rather than a line that
        // quietly disappears. Thirty-three mutations cover thirty-two of the pattern's thirty-five
        // leaves — `page_budget` takes two, for its mode and its value — and the three that are
        // not covered are named in this test's doc comment, each because its type has exactly one
        // legal value and cannot be mutated at all. Thirty-two was the number at v2-S13.1; the
        // one it grew by is `table_detection.tagged`, which arrived at v2-S24 with its mutation
        // in the same commit.
        assert_eq!(
            mutations.len(),
            33,
            "{} single-field mutation(s); thirty-three is the number at v2-S24",
            mutations.len()
        );

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
            r#"{"backend":{"name":"lopdf","version":"0.44.0"},"capabilities":{"annotations":true,"char_offsets":false,"form_fields":true,"html":true,"images":true,"markdown":true,"measured_ink_boxes":true,"multi_column_reading_order":true,"page_screenshots":false,"spans":true,"structural_locators":true,"tables":true},"classify_sample_pages":8,"cmap_data_version":"annex-d-encodings-1","coordinate_system":{"origin":"top-left","unit":"centipoint"},"form_annotation_rule":"form-annotations-v1","html_rule":"html-blocks-v5","markdown_rule":"markdown-blocks-v5","observation_rule":"page-observations-v1","page_budget":{"mode":"unlimited"},"parser_version":"0.50.0","quantum_per_point":100,"raster_dpi":{"mode":"not_emitted"},"reading_order_rule":"gutter-columns-v2","struct_tree_rule":"struct-tree-v1","table_detection":{"ruled":"ruled-rects-v3","stroke_ruled":"stroke-ruled-v1","tagged":"tagged-tables-v1","unruled":"unruled-align-v1"},"text_code_rule":"declared-font-codes-v1","verifier":{"mode":"not_pinned"},"xref_repair":{"mode":"pad-19-to-20-v1"}}"#,
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
             in is a claim the run never made.\n\n\
             Moved a FIFTEENTH time at v1.1-S2 (0.12.0), and no field arrived: `markdown_rule` \
             CHANGED VALUE, from `markdown-linear-v1` to `markdown-blocks-v1`. Two artifacts \
             either side of this hash say something different about the same document — a table \
             is a run of paragraphs on one side and a GFM grid on the other — which is precisely \
             what a rule id on the profile is for. A version bump alone would not have carried \
             it: the projection rule is what moved.\n\n\
             Moved a SIXTEENTH time at v1.1-S3 (0.13.0), and again no field arrived: \
             `markdown_rule` moved from `markdown-blocks-v1` to `markdown-blocks-v2`, because the \
             export now closes up a word the page broke across a line. `hyphenated-line-break` \
             projects as `hyphen-\\n\\nated` on one side of this hash and `hyphenated` on the \
             other. Nothing in the REPRESENTATION moved — `extract` still emits two `Extracted` \
             runs with the hyphen verbatim, which is why this is a projection rule id and not a \
             new derivation class.\n\n\
             Moved a SEVENTEENTH time at v1.1-S4 (0.14.0), and TWO fields arrived: `html_rule` \
             and `capabilities.html`. v1.1-S4 adds a second OUTPUT rather than changing either \
             detector or the Markdown rule — `ethos.html.v1`, under the same four laws. The two \
             projections say different things about the same table: GFM cannot hold a merge, so \
             `markdown` expands it and counts the slots in `gfm-span-slots-unrepresentable-v1`, \
             while `html` emits one `<td colspan>` and carries it. `markdown_rule` did NOT move, \
             so a reader holding artifacts either side of this hash gets byte-identical Markdown \
             and a new artifact type beside it.\n\n\
             Moved an EIGHTEENTH time at v1.1-S4's repair (0.14.1): `html_rule` moved from \
             `html-blocks-v1` to `html-blocks-v2`. `-v1` existed for one commit and got two \
             tables wrong — a span colliding with another origin widened a row past the declared \
             grid, and a list whose tree skipped a depth came out unbalanced. Nothing was ever \
             published under it, so no consumer holds such an artifact; the id and the version \
             move anyway, because two builds in this repository's own history producing \
             different bytes under one id is the state a rule id exists to make impossible.\n\n\
             Moved a NINETEENTH time at v1.2-S1 (0.15.0), and NOTHING but the version moved — the \
             second time that has happened, after v1-S7b. v1.2 is adoption: `ethos-parser mcp` serves \
             the existing stages over MCP on stdio and adds no parse feature, so no field, no \
             rule id and no capability changed. A transport is not a parse capability, and \
             `capabilities.mcp` would put a flag on every artifact that no consumer could act on. \
             Two artifacts either side of this hash say exactly the same thing about the same \
             document; the hash moves because `parser_version` is in it, and a version claiming \
             otherwise is the one lie this field cannot afford.\n\n\
             Moved a TWENTIETH time at v1.2-S2 (0.16.0), and again NOTHING but the version moved \
             — the third time, after v1-S7b and v1.2-S1. S2 is the Python SDK, and it adds no \
             Rust at all: `packages/python/` shells out to this binary, so `extract` and `ground` \
             return the bytes the CLI printed and cannot diverge from them. An SDK is an adopter \
             rather than a parse capability, so there is no `capabilities.python` for the reason \
             there is no `capabilities.mcp` — a flag on every artifact that no consumer can act \
             on. What DID have to be got right lives outside this hash: `node_get` has no \
             subcommand behind it, so c14n v1 is ported to Python and pinned against this \
             module's own parity vectors, because a fingerprint that is merely NEARLY this one \
             would accept an artifact the engine refuses.\n\n\
             Moved a TWENTY-FIRST time at v1.2-S3 (0.17.0), and again NOTHING but the version \
             moved — the fourth time. S3 is the Node SDK, and it is the Python one in a second \
             language rather than a second design: the same three functions, spawning this same \
             binary, with no native addon for the reason S2 had no PyO3. A language binding is an \
             adopter, so there is no `capabilities.node` any more than there is a \
             `capabilities.python`. c14n is ported a second time and pinned against this module's \
             vectors again — and where JavaScript cannot follow, it says so rather than \
             approximating: that language has one number type, so `1.0` and `1` are the same \
             value and no port can tell them apart. What both ports reject identically is a value \
             that is genuinely not an integer, which is the property this serializer needs.\n\n\
             Moved a TWENTY-SECOND time at v1.2-S4 (0.18.0), and again NOTHING but the version \
             moved — the fifth time. S4 is LangChain tools over the two SDKs, and it adds no Rust \
             at all: three callable tools per language, each calling the SDK rather than spawning \
             this binary a third way. Its whole content is a split this profile already describes \
             elsewhere — locators in the artifact, counts in the prose — so a framework binding \
             is an adopter and there is no `capabilities.langchain`, for the reason there is no \
             `capabilities.node`. The tools are behind an optional extra and an optional peer, so \
             neither SDK's empty runtime install moved either.\n\n\
             Moved a TWENTY-THIRD time at v1.2-S5 (0.19.0), and this one shipped NO ADAPTER AT \
             ALL. S5 asked for a `liteparse -> ethos.grounding.v1` mapper *if it is worth it*, and \
             it is not: their output cannot name its own producer and their boxes are loose em \
             boxes this schema has no way to declare. Both walls are properties of the artifact \
             type, not of a document, so no per-document adapter could clear them. The version \
             moves anyway on the precedent of v1-S7b, where a slice measured a dead end and \
             bumped: two builds that disagree about what this repository decided must not both \
             call themselves 0.18.0.\n\n\
             Moved a TWENTY-FOURTH time at v2-S1 (0.20.0), and this one is a CONTRACT DECISION \
             with no reader. `ethos.grounding.v1` stays PDF-only: revising it is a change to the \
             verifier's contract, not the engine's, and the oracle agrees with the pinned Ethos \
             CLI on this exact schema. The slice's finding is that the page assumption is not \
             where v2-S0 thought it was — `DocumentRepresentation::seal` refuses a node whose \
             parent is not a declared page, so a page-less document cannot become a \
             representation at all, and that invariant lives in THIS crate rather than in \
             grounding. No detector moved, no rule id moved, no capability moved.\n\n\
             Moved a TWENTY-FIFTH time at v2-S2 (0.21.0) on `parser_version` ALONE, and the \
             interesting part is what did NOT move: every other byte of this profile is \
             identical, because v2's first format got a profile of its OWN rather than a \
             capability on this one. `Profile::docx_v0` has its own hash so a DOCX artifact and a \
             PDF artifact are provably non-comparable — the same discipline that will keep an \
             OCR'd page from comparing equal to a born-digital one. `XrefRepair` gained a \
             `not-run-for-this-format` state for that profile to use, and this one still says \
             `pad-19-to-20-v1`, so the new variant is invisible here. That is the point: a second \
             format is a new VALUE, not a change to what this profile claims.\n\n\
             Moved a TWENTY-SIXTH time at v2-S3 (0.22.0) on `parser_version` ALONE, and this \
             entry exists to record a decision NOT to move it further. v2-S2 left \
             `coordinate_system` carrying `centipoint`/`top-left` on a page-less profile — inert, \
             because nothing under that profile emits a coordinate — and deferred the question of \
             an honest page-less spelling to the slice that would have a second such format to \
             judge by. S3 measured it: no consumer ACTS on the value for a page-less artifact. \
             `ethos_parser_grounding::project` hard-codes the pair rather than copying it, and refuses \
             a non-`application/pdf` source long before reaching it; the only branch on the value \
             anywhere is inside the `ethos.grounding.v1` validator, downstream of that refusal; \
             both SDKs re-hash the field as opaque bytes and never parse it. A mode enum would \
             have moved THIS hash — and every PDF artifact's — to respell a value nothing reads, \
             so it was not added. `Profile::xlsx_v0` carries the same inert declaration and its \
             own hash, and tests pin that both page-less profiles agree with this one on the \
             field while emitting zero measured geometry rows.\n\n\
             Moved a TWENTY-SEVENTH time at v2-S4 (0.23.0) on `parser_version` ALONE, and this \
             entry records a third page-less format that changed nothing here. PPTX is the format \
             that most looks like it has pages — a deck HAS slides, a slide HAS a size in EMUs, \
             and a person counts them out loud — so the temptation was to give this profile a \
             raster or a page budget that meant something. It has neither: a slide is a PART, \
             `pages` is empty, `measured_ink_boxes` is false, and `Profile::pptx_v0` carries the \
             same inert `coordinate_system` v2-S3 decided to leave alone. Three formats now share \
             that declaration and none of them emits a coordinate.\n\n\
             Moved a TWENTY-EIGHTH time at v2-S5 (0.24.0) on `parser_version` ALONE, and the \
             format behind it is the one that could have moved more than the version. An ODT's \
             `content.xml` contains `<text:soft-page-break/>` — the position at which the \
             PRODUCING APPLICATION broke the page, written down at save time — and its \
             `styles.xml` contains an `fo:page-width`. Between them a `PageRecord` needs no \
             arithmetic at all, which is the first time that has been true in this version. It is \
             still a measurement of a word processor rather than of the document, and it moves \
             when the font stack or the paper does, so `docs/06-STEAL-REFUSE.md` L30 refuses it as \
             a refusal rather than a fallback: the break is read, discarded, and `pages` stays \
             empty. `Profile::odt_v0` carries the same inert `coordinate_system` the other three \
             page-less profiles do — four formats now share that declaration and none of them \
             emits a coordinate — and this profile's own bytes are untouched, because a fourth \
             page-less format is a new VALUE rather than a change to what this one claims.\n\n\
             Moved a TWENTY-NINTH time at v2-S6 (0.25.0) on `parser_version` ALONE, and the format \
             behind it is the one that states no address. SpreadsheetML writes `<c r=\"B12\">` and \
             `XlsxLocator` READS it; OpenDocument writes neither a row number nor a column letter \
             anywhere, and states a cell's position by where it sits among its siblings — \
             compressed by `table:number-columns-repeated`, which is the file saying \"and n more \
             of these\". Honouring that is reading the only statement of position the format \
             makes. `Profile::ods_v0` carries the same inert `coordinate_system` the other four \
             page-less profiles do — five formats now share that declaration and none of them \
             emits a coordinate — its `capabilities.tables` is FALSE because a spreadsheet's cells \
             are addresses the file states rather than a grid a detector inferred, and this \
             profile's own bytes are untouched for the reason they were at S5.\n\n\
             Moved a THIRTIETH time at v2-S7 (0.26.0) on `parser_version` ALONE, and the format \
             behind it is the one that could have had a page for free. An `.odp` lists \
             `<draw:page>` elements — discrete, ordered, named, and counted out loud by anybody \
             describing a deck — and a master page states `fo:page-width` beside them, so a \
             `PageRecord` needed NO arithmetic at all for the first time in this engine's \
             history. It is still not a page this engine measured: a draw page is a part of the \
             presentation's structure, and `docs/06-STEAL-REFUSE.md` L30 refuses invented \
             pagination whether the invention costs a renderer or costs nothing. `pages` stays \
             empty and `OdpLocator` carries a `draw_page` POSITION named for the element rather \
             than for what it resembles. `Profile::odp_v0` carries the same inert \
             `coordinate_system` the other five page-less profiles do — six formats now share \
             that declaration and none of them emits a coordinate — and this profile's own bytes \
             are untouched, because a sixth page-less format is a new VALUE rather than a change \
             to what this one claims.\n\n\
             Moved a THIRTY-FIRST time at v2-S8 (0.27.0) on `parser_version` ALONE, and the format \
             behind it is the first that is not a package. Every v2 format before RTF is a ZIP \
             with parts, and every one of their locators opens with a part name; an `.rtf` is one \
             brace-group byte stream with no parts, no manifest and no name for itself. That did \
             not move this profile and it did move an INVARIANT: `check_structure`'s page-less \
             shape grew a fourth rule, because its part-id bijection has no part name to be a \
             bijection between, and a constant standing in for a part the format lacks would have \
             been `docs/history/14-V2-SCOPE.md` §3's invented value in a small place. `Profile::rtf_v0` \
             carries the same inert `coordinate_system` the other six page-less profiles do — \
             seven formats now share that declaration and none of them emits a coordinate — and \
             its `capabilities.tables` is FALSE even though RTF writes `\\cell` and `\\row`, \
             because those are recorded as a paragraph's terminator rather than as a grid a \
             detector inferred.\n\n\
             Moved a THIRTY-SECOND time at v2-S9 (0.28.0) on `parser_version` ALONE, and the \
             format behind it is the first that could have supplied a page FROM THE FILE. \
             `docs/history/14-V2-SCOPE.md` §3's law has always been \"no page this engine did not read \
             from the file\" rather than \"no page ever\", and every format before this one \
             failed the reading half: a DOCX has no page until a renderer picks one, a slide is a \
             part, an ODT's break is a word processor's arithmetic, a draw page is structure, and \
             `\\page` is a producer's mark. An EPUB 3 navigation document may carry a `page-list` \
             naming the pages of a PRINT edition, and an EPUB 2 NCX may carry page targets — \
             actual page identifiers, written down, readable. They are still refused, and the \
             reason had to be argued rather than looked up: a publisher's label about somebody \
             else's paper has no width and no height, so a box could never be validated against \
             it, and `check_structure` refuses a measured box on a page-less node precisely \
             because a rectangle nobody can check is the fabrication that law exists to prevent. \
             `pages` stays empty. `Profile::epub_v0` carries the same inert `coordinate_system` \
             the other seven page-less profiles do — eight formats now share that declaration and \
             none of them emits a coordinate — and this profile's own bytes are untouched, for the \
             reason they were at S5.\n\n\
             Moved a THIRTY-THIRD time at v2-S9.1 (0.28.1) on `parser_version` ALONE, which is \
             the third time a PATCH release has moved it — v1-S6.1 (0.8.1) and v1-S6.2 (0.8.2) \
             came first. Nothing about what any profile CLAIMS changed. \
             What changed is that eight readers stopped reaching an A14 erasure count by an \
             operation that can wrap: v2-S9's adversarial review reproduced a 31 MB EPUB exiting 0 \
             while declaring 51,032,704 passed-over runs against a true 4,346,000,000, fixed the \
             EPUB sites, and left the same plain `+=` in seven other readers plus two of \
             `ethos-parser-pdf`'s eight document-level accumulators. `parser_version` is part of \
             identity precisely so that a build which counts differently is a different profile — \
             an artifact produced before this bump and one produced after can declare different \
             numbers for the same bytes, and a consumer must be able to tell them apart.\n\n\
             Moved a THIRTY-FOURTH time at v2-S10 (0.29.0) on `parser_version` ALONE, and it is \
             the only thing that moved, because the slice added NO READER. There is no \
             `Profile::csv_v0`; there are still NINE profiles and this is still the only one with \
             a page. v2-S10 is a REFUSAL: bytes that state no format — no signature, no container, \
             no declaration — are now refused by naming what was looked for, instead of being \
             handed to this profile's reader and told they lack a `%PDF-` header. That message was \
             fail-closed and named the WRONG CAUSE, which is the defect v2-S6 fixed for an `.ods`, \
             v2-S8 for an `.rtf` and for the ZIP shape, and this slice for the last member of that \
             shape.\n\n\
             The reason there is no CSV reader is one field rather than the parse. A CSV parse of \
             arbitrary text fabricates NOTHING — every field's text is bytes genuinely present, \
             every record ordinal a true line count, and `pages` empty is simply true. What would \
             be false is `SourceIdentity.media_type` naming a CSV media type for a file nobody \
             measured to be one, and `SourceIdentity` is `deny_unknown_fields` with two fields and \
             no room to record that a type was ASSERTED rather than READ. \
             `docs/history/13-V12-MILESTONES.md` settled that at v1.2-S5: an identity that can be asserted \
             is an identity that can disagree with what it describes.\n\n\
             Moved a THIRTY-FIFTH time at v2-S10.2 (0.29.1) on `parser_version` ALONE, which is \
             the fourth time a PATCH release has moved it — v1-S6.1 (0.8.1), v1-S6.2 (0.8.2) and \
             v2-S9.1 (0.28.1) came first. NOTHING in this crate's behaviour changed and no \
             profile field moved: the slice is a documentation repair, and it moves the version \
             because a build is a build. Every statement it corrected was a measured claim about \
             this tree that had stopped being true — the oldest since v2-S5, five slices — and a \
             repository whose whole value is that a decision can be audited from the tree alone \
             cannot let a reader check a sentence and find it false.\n\n\
             Moved a THIRTY-SIXTH time at v2-S11 (0.30.0) on `parser_version` ALONE. No field on \
             any profile moved and this profile's own bytes are otherwise untouched — an embedded \
             asset is not a capability and `capabilities.images` still means what it meant. What \
             changed is that the three OOXML readers stopped being BLIND to a kind of erasure the \
             other five already counted: `word/media/`, `xl/media/` and `ppt/media/` matched no \
             A14 bucket at all, so a DOCX with forty embedded images declared ZERO parts not read \
             for them. A second code — `office-embedded-parts-not-read` — counts them, rather \
             than widening the first, whose shipped message says its parts CARRY TEXT and would \
             have become false about every file it fires on. `parser_version` is part of identity \
             precisely so that a build which declares differently is a different profile.\n\n\
             Moved a THIRTY-SEVENTH time at v2-S12 (0.31.0) on `parser_version` ALONE. The slice \
             added a fuzz target and NOTHING ELSE — no reader changed, because the campaign that \
             target ran found nothing to change. `A11`'s `cargo-fuzz` lane had been open since v0 \
             and deferred at v2-S2 on the condition that a SECOND office format would reopen it; \
             there were eight. This profile does not move because the engine now behaves \
             differently. It moves because a build is a build, and the version is the only field \
             that can say WHICH build a reader is holding.\n\n\
             Moved a THIRTY-EIGHTH time at v2-S12.1 (0.31.1) on `parser_version` ALONE, and this \
             one changed no code at all — it added the CI step that compiles `office_read`, and \
             repaired statements that had stopped being true. v2-S9.1 and v2-S10.2 set the \
             precedent and it holds for the same reason: a reader holding an artifact cannot see \
             which comments were right, only which build produced it, and two builds that differ \
             must not answer to one version.\n\n\
             Moved a THIRTY-NINTH time at v2-S13 (0.32.0) on `parser_version` ALONE. The slice \
             closed `A11`'s MUTATION lane for office — every package in `fixtures/office/` damaged \
             twelve ways, 148 mutants, no panic — and changed NO READER, because the two permitted \
             outcomes held on every one of them. It did find that `zip.rs` verifies a part's \
             declared LENGTH and never its CRC-32, so a corrupted compressed part that still \
             inflates to the right size is read as though intact; that is stated for the owner in \
             `15`'s S13 rather than fixed here, because the artifact still binds to the bytes it \
             actually read and changing a hand-rolled reader deserves its own measurement.\n\n\
             Moved a FORTIETH time at v2-S13.1 (0.32.1) on `parser_version` ALONE, and this one \
             changed no shipping line at all — every edit under `crates/*/src` is a comment or \
             sits inside `#[cfg(test)]`, proven by diffing the 19,249 non-comment lines outside \
             `mod tests` at HEAD against the working tree. It repaired the guards that checked \
             nothing: a CI filter parser blind to five quoted commands, one filter token dead \
             since v1-S2, and seventeen guard sites whose names promised more than their bodies \
             held \
             — including EIGHT fields on this very struct that the list below never mutated \
             while the comment above it said an uncovered knob was impossible.\n\n\
             Moved a FORTY-FIRST time at v2-S13.2 (0.32.2), the roadmap reorder, which did not \
             record the move here. Moved a FORTY-SECOND time at v2-S13.3 (0.32.3), on \
             `parser_version` ALONE again: the prose half of the fifty-two statements v2-S12.1 \
             confirmed, plus the ones a 1,789-candidate sweep found on top of them. No shipping \
             line moved in either.\n\n\
             Moved a FORTY-THIRD time at v2-S13.4 (0.32.4), on `parser_version` ALONE, and this \
             one changed no source line at all: the owner settled the v2 gate's verb as BIND and \
             embedded assets as COUNTED, and the slice is those two decisions written into \
             `00-NORTH-STAR.md` and the sentences that carried them reworded.\n\n\
             Moved a FORTY-FOURTH time at v2-S13.5 (0.32.5), on `parser_version` ALONE, and every \
             `crates/*/src` edit is a comment — proven by diffing the 19,249 non-comment lines \
             outside `mod tests` at HEAD against the working tree. It ran the two sweeps v2-S13.3 \
             recorded as UNRUN on a usage limit: 12,805 candidate comment lines and 1,372 \
             backticked identifiers examined. The finding that justifies the slice is that \
             `07-VERIFY-BOUNDARY.md` claimed to hold this repository's forced decisions \
             *verbatim, identically* while holding FOURTEEN of SEVENTEEN — missing exactly the \
             three the owner amended on 2026-08-21, which v2-S13.4 wrote into the other copy so \
             that no later slice would re-escalate them from a stale sentence.\n\n\
             Moved a FORTY-FIFTH time at v2-S14 (0.33.0), and this one is NOT `parser_version` \
             alone in spirit even though it is in fields: a READER CHANGED. `zip.rs` now verifies \
             the CRC-32 the central directory carries for each part and refuses a mismatch, where \
             every build before this one checked the declared LENGTH and nothing else. A MINOR \
             bump rather than a patch, because a package that 0.32.5 read and this build refuses \
             is a different answer to the same bytes — which is exactly what a version has to be \
             able to say. It shipped on a measurement: ZERO false refusals across 40 valid \
             packages and 2,370 entries. The office mutation harness recorded the change as \
             survivors falling 36 to 31, emptying the `main-part-byte-flipped` class outright.\n\n\
             Moved a FORTY-SIXTH time at v2-S14.1 (0.33.1), on `parser_version` alone. That slice \
             wrote the two guards v2-S13.5 found named in doc comments and never written — \
             `cell_text_survives_the_reordering` and the assertion that this enum's serde \
             spelling and `ethos_parser_pdf::xref::XREF_REPAIR_V1` are the same string — and repaired \
             the `four words` cluster at its four live sites, leaving the two that sit in frozen \
             history named rather than rewritten. No \
             behaviour changed: the extractor that proved it reports an empty diff over 17,728 \
             non-comment lines outside `mod tests`.\n\n\
             Moved a FORTY-SEVENTH time at v2-S15 (0.34.0), and this one is NOT `parser_version` \
             alone: an ARTIFACT CHANGED. Fifteen sites said `neither detector` of a `/TH` no \
             detector reads, and there have been THREE detectors since v1-S8 — but two of the \
             fifteen are EMITTED WIRE STRINGS, so the cluster could only move as one or not at \
             all. Repairing the comments alone would have left the wire saying *neither* and the \
             comments saying otherwise, a NEW inconsistency worse than the one it fixed. A MINOR \
             bump rather than a patch, because an artifact this build emits differs from one \
             0.33.1 emitted for the same bytes. The blast radius was measured before the wording \
             was chosen: `representation_sha256` moves, THIS digest does not move on the message \
             (only on `parser_version`), the Ethos oracle does not move because a grounding \
             artifact carries a limitation CODE and never its text, and the two mutation \
             harnesses do not move because `EXPECTED_SURVIVORS` pins outcomes. The wording \
             carries no ordinal, so a fourth detector cannot re-rot it.\n\n\
             Moved a FORTY-EIGHTH time at v2-S16 (0.34.1), on `parser_version` alone. That slice \
             committed `ci/code-lines.py`, the extractor every patch slice's `no behaviour \
             change` proof depends on and which had never been in the tree: four slices rebuilt \
             it by hand and THREE of the four were wrong, in three different ways — one blind to \
             every emitted wire string, one leaking test code through a brace inside a string \
             literal, and the record left carrying four different absolute counts for one rule. \
             No line of `crates/*/src` outside `mod tests` changed, proven with the committed \
             instrument: an empty diff over 19,281 lines.\n\n\
             Moved a FORTY-NINTH time at v2-S17 (0.34.2), on `parser_version` alone. Two guards \
             outside `crates/*/src`, both named at v2-S13.5 and deferred twice. \
             `verify_relay.rs` said the relay is reachable through the library `like the other \
             four subcommands` when there are NINE, and `diagnostics.rs` asserted a floor of FOUR \
             against an enum with FIVE variants while its own message said all of them had to be \
             covered — a guard set one below its population, which is the shape v2-S13.1 exists \
             for. The floor is now DERIVED from `Stage` and asserts WHICH stages were walked \
             rather than how many, so a sixth variant stops the file compiling. `Stage::Verify` \
             stays outside the walk and the exclusion is argued rather than hidden: `ethos-parser \
             verify` forwards the verifier bytes and composes nothing, and `verify_relay.rs` \
             already asserts that stdout byte-identically, which is stronger than the key walk. \
             No line of `crates/*/src` changed at all.\n\n\
             Moved a FIFTIETH time at v2-S18 (0.34.3), on `parser_version` alone. The slice that \
             made the gate real: `cargo fmt --all --check` had exited NON-ZERO at every commit \
             since v2-S14, under four consecutive records each claiming a green, because \
             `.github/workflows/ci.yml` HAD NEVER RUN — no remote, no tag, 85 commits at that \
             point — so every \
             green this repository has ever recorded was a local partial run with whichever \
             checks somebody remembered. Four formatting sites repaired, three clippy warnings in \
             `crates/*/tests/` repaired (the third invisible until the first two were, because \
             `-D warnings` aborts compilation), and `ethos-parser-cli`'s package description stopped \
             naming four subcommands of nine. `ci/gate.sh` now runs the whole gate locally, and \
             `the_local_gate_runs_what_ci_runs` asserts it carries the same commands as the \
             workflow in BOTH directions — a convenience script nobody verified against CI is \
             worse than no script, because a local green would then mean something the remote \
             does not enforce. No line of `crates/*/src` changed but one blank line.\n\n\
             Moved a FIFTY-FIRST time at v2-S19 (0.35.0), on `parser_version` alone — and \
             `table_detection` is BYTE-IDENTICAL across this bump, which is the proof the slice \
             owes: S19 grew the gate corpus from four documents to twelve and re-measured, and a \
             slice that moved the corpus AND the detector in one commit could not tell you which \
             one moved the number. A MINOR rather than a patch because the corpus is not \
             cosmetic: `fixtures/manifest.json` went from 55 fixtures to 64, which moves the PDF \
             mutation harness off its pinned counts. Macro cell-slot F1 read 70‰ over twelve \
             where it read 64‰ over four — and the band, which is the number that matters, is \
             0‰ to 590‰ with NINE OF TWELVE scoring exactly 0. See `docs/table-gate-v1.md` and \
             `00-NORTH-STAR.md` #18, which is written and NOT decided.\n\n\
             Moved a FIFTY-SECOND time at v2-S20 (0.36.0), on `parser_version` AND \
             `table_detection.ruled`, which goes `ruled-rects-v2` to `ruled-rects-v3`. Unlike the \
             move before it this one is a DETECTOR change and the profile says so: a grid whose \
             own structural cross-check rejects it is now refused rather than emitted with the \
             contradiction recorded beside it, because the Markdown and HTML projections draw \
             every table the artifact carries and read no check. Measured on all twelve gate \
             documents: `nist-sp-800-218` loses nine phantom grids and 11 295 false-positive cell \
             slots, every other document is unchanged, fabrication stays 0, and the cost is TWELVE \
             cell slots that are all the empty string. The macro is 70‰ either way, which is the \
             finding. `stroke-ruled-v1` and `unruled-align-v1` are byte-identical and keep their \
             ids: their cells tile by construction, so their check cannot fail. See \
             `docs/table-gate-v1.md` §\"v2-S20\".\n\n\
             Moved a FIFTY-THIRD time at v2-S21 (0.36.1), on `parser_version` alone — and this \
             one changes NO reader: `ci/code-lines.py` diffs empty across the commit, so every \
             artifact this build writes is byte-identical to 0.36.0's but for this field. What \
             moved is a test: `flip-tail-byte` now seeks the file trailer's cross-reference \
             pointer instead of a fixed fraction of the file's length, which it had claimed to \
             reach since M7 and never did on a large document.\n\n\
             Moved a FIFTY-FOURTH time at v2-S22 (0.36.2), on `parser_version` alone, and this is \
             the whole proof that the slice changed no detector. v2-S22 is a diagnostic: it \
             measures why ten of the twelve gate documents produce no table at all, and the \
             finding is that `unruled-align-v1` has emitted none on any of them — the ruled and \
             stroke-ruled rules carry the entire gate number. No rule id, tolerance or field \
             below moves; the JSON above differs from 0.36.1's in `parser_version` and nothing \
             else, which is what a diagnostic slice is allowed to move and all it is allowed to \
             move. See `docs/table-gate-v1.md` §\"v2-S22\".\n\n\
             Moved a FIFTY-FIFTH time at v2-S23 (0.36.3), on `parser_version` alone. v2-S23 \
             revisits the two coverages v2-S20 and v2-S21 retired — the on-wire \
             `CheckStatus::Mismatch` and `lopdf`'s catalog-scan recovery — and changes no reader: \
             `ci/code-lines.py` diffs empty and the only source edits are inside `mod tests`. The \
             JSON above differs from 0.36.2's in `parser_version` and nothing else.\n\n\
             Moved a FIFTY-SIXTH time at v2-S24 (0.37.0), on `parser_version` AND a NEW field: \
             `table_detection` gains a fourth key, `tagged` = `tagged-tables-v1`. This is a MINOR \
             because a reader changed — the PDF extractor now emits a table for each `/Table` the \
             structure tree declares that no geometric detector matched, so a NIST document whose \
             tables are tagged but not drawn gains tables on the artifact where 0.36.3 emitted \
             none. The `table_detection` doc anticipated this exact move — *\"a future slice that \
             retires or adds one changes this shape and moves the hash\"* — and it moves the hash \
             twice over, by the new field and by `parser_version`. A tagged table is `Extracted` \
             and carries no geometry (`GeometryAbsence::NotReportedByStructureTree`); the geometric \
             detectors, the gate, and the three ids they write are byte-identical, so this is an \
             ADDITION to what the profile runs, not a change to any rule it already ran. See \
             `docs/table-gate-v1.md` §\"v2-S24\".\n\n\
             Moved a FIFTY-SEVENTH time at 0.37.1, on `parser_version` alone — a performance \
             repair whose whole claim is that nothing else moved. Canonical serialization \
             streams instead of building a `serde_json::Value` tree, fonts parse once per \
             document instead of once per page, and the interpreter's run buffers move instead \
             of cloning; `ethos-parser extract` on `nist-sp-800-53Ar5` fell from 100.4 s to 59.9 s. \
             The proof is byte comparison, not assertion: at equal version the 932 MB \
             `nist-sp-800-53Ar5` and the `nist-sp-800-218` extract artifacts are byte-identical \
             to pre-change output, and the c14n suite gained the equivalence law \
             `canonical_bytes_of(v) == c14n_bytes(&to_value(v))`. The JSON above differs from \
             0.37.0's in `parser_version` and nothing else. See CHANGELOG \"0.37.1\".\n\n\
             Moved a FIFTY-EIGHTH time at 0.37.2, on `parser_version` alone — the second half of \
             the same performance repair, with the same proof. The ruled rule's face-coverage \
             scan becomes a difference grid, its line lookups and the unruled rule's become \
             binary searches, and the cross-check's overlap test becomes a sweep — each rewrite \
             argued equivalent in the source and then proven the only way that counts: the gate \
             corpus artifacts are byte-identical at equal version. The JSON above differs from \
             0.37.1's in `parser_version` and nothing else. See CHANGELOG \"0.37.2\".\n\n\
             Moved a FIFTY-NINTH time at 0.38.0, on `parser_version` — and, in the six office \
             profiles, on their `text_code_rule` ids, which go verbatim-v1 to verbatim-v2 \
             because the behaviour they name moved: the DOCX, XLSX, PPTX, ODT, ODS and ODP \
             readers now resolve numeric character references, in text and in the names \
             attributes carry, through the hardened resolver the EPUB reader shipped at v2-S9. \
             This is the decision v2-S9 recorded rather than made — a MINOR, because readers \
             changed: a document those rules refused by name at v1 now reads, and the six hash \
             moves are the receipt. The JSON above is the PDF profile and carries no office \
             rule, so it moves on `parser_version` alone; the same slice gave the MCP surface \
             the CLI's format router, which changes no profile at all. See CHANGELOG \"0.38.0\".\n\n\
             Moved a SIXTIETH time at 0.38.1, on `parser_version` alone — and this one, like the \
             FIFTY-FOURTH, is a diagnostic whose whole point is that nothing else moved. The \
             estate audit's component-clustering lead for the ruled rule was implemented in \
             full, measured on the twelve-document gate, and REFUSED on its numbers: macro fell \
             70 to 63 permille, the corpus's best document split into single-dimension \
             fragments, and sixty-five furniture grids were admitted with fabrication still 0 — \
             which is #18's \"worse than a zero\" shape, real text arranged into grids that are \
             not there. The code is reverted; `docs/table-gate-v1.md` carries the per-document \
             table, the two mechanisms, and the finding the next attempt has to answer. The \
             JSON above differs from 0.38.0's in `parser_version` and nothing else. See \
             CHANGELOG \"0.38.1\".\n\n\
             Moved a SIXTY-FIRST time at 0.38.2, on `parser_version` alone — pages extract in \
             parallel, and the artifact cannot tell. Each page runs the body the sequential \
             loop always ran, against the shared read-only handle, with a page-local id \
             allocator; a sequential fold then rebases every ordinal onto the document-global \
             sequence — holes included, because a refused candidate consumes an id it never \
             ships and the artifact keeps that hole — and replays the allocation counts so \
             even the MAX_SAFE_INT refusal fires at the page it always fired at. Proof is \
             byte comparison across the gate documents at equal version, and the honest \
             number is modest: the emit tail is still sequential, so the 492-page document \
             falls from ~57 s to ~50 s and the tail is now the named next slice. The JSON \
             above differs from 0.38.1's in `parser_version` and nothing else. See CHANGELOG \
             \"0.38.2\".\n\n\
             Moved a SIXTY-SECOND time at 0.38.3, on `parser_version` alone — the emit tail the \
             SIXTY-FIRST move named as the next slice. `seal` keeps the canonical payload bytes \
             it hashes, and the print splices them into the envelope through one duplicate-\
             refusing joiner in c14n, so the payload is walked once per artifact instead of \
             twice; a parsed artifact carries no cache and takes the full pass, and a test pins \
             the two routes byte-equal. Proof is byte comparison across the gate documents at \
             equal version; the 492-page document falls from ~50 s to ~40 s. The JSON above \
             differs from 0.38.2's in `parser_version` and nothing else. See CHANGELOG \
             \"0.38.3\".\n\n\
             Moved a SIXTY-THIRD time at 0.39.0, on `parser_version` alone — and this MINOR is \
             the one v2-S1 priced and parked. The Ethos-side revision the page-less refusal \
             waited on landed as `ethos.grounding.v1` schema 1.1.0, and `ethos-parser ground` now \
             projects a page-less office representation into its page-less shape: `pages: []`, \
             every element under its own node id, the native locator serialized beside the \
             text, no geometry anywhere. A PDF projection is byte-identical to 0.38.3's — \
             1.0.0, same bytes — which is why this JSON moves on `parser_version` alone. \
             Measured end to end with real binaries on both sides: a DOCX quote extracted, \
             grounded, and verified comes back grounded at element scope. See CHANGELOG \
             \"0.39.0\".\n\n\
             Moved a SIXTY-FOURTH time at 0.40.0, on `parser_version` alone — but unlike the \
             three moves before it, this one travels with artifacts that change. \
             `check_structure` has always required `geometry-absent-not-groundable` exactly when \
             some node is non-groundable, and an annotation, a form field and an image are all \
             non-groundable by construction; the PDF producer triggered the declaration on \
             ink-absent TEXT RUNS only. A document with measurable text and one annotation was \
             therefore REFUSED — `ethos-parser extract` exited 2 and produced nothing — and real files \
             escaped only because one whitespace-only run or one metric-less font supplied a \
             text-run absence that fired the declaration for another reason. The same slice \
             corrects the ink sentence's denominator, which counted every node while its \
             numerator counted text runs, so a document with two annotations read \"1 of 3 text \
             node(s)\" about its one text node. Six of fifty-two fixtures move, every one of \
             them carrying a non-text node; the office readers are untouched and byte-identical. \
             See CHANGELOG \"0.40.0\".\n\n\
             Moved a SIXTY-FIFTH time at 0.40.1, on `parser_version` alone, and nothing this \
             engine EMITS changes at all — the move is the version, not the shape. `Assurance` \
             documents that a record claiming `Complete` while carrying a quarantined page is \
             \"not a bug this type can have\", and that held for everything the crate builds and \
             nothing it reads: a derived `Deserialize` over five `pub` fields took the wire's \
             word for the capabilities, the tally and the terminal state alike. The fingerprint \
             does not help, because it binds a payload to itself rather than to the truth of \
             what the payload asserts. Deserialization now re-derives the coverage from the page \
             states and the terminal state from the coverage, exactly as `Assurance::new` does, \
             and refuses a disagreement. Sixty artifacts across the fixture and gate corpora \
             round-trip unchanged; a forged one with a recomputed fingerprint, which `ethos-parser \
             ground` used to accept and project, is refused by name. See CHANGELOG \"0.40.1\".\n\n\
             Moved a SIXTY-SIXTH time at 0.40.2, on `parser_version` alone, and again nothing in \
             the representation changes — the move is the overlay's. Since v2-S24 a tagged table \
             is a first-class record carrying absent geometry, which makes it exactly the case \
             the overlay's per-page note exists to disclose: found, in the artifact, and \
             impossible to draw. The note counted `page.tables` alone, so on \
             `irs-f1040sd-2025` page 1 it read \"0 table(s) ... 0 marked item(s) have NO \
             rectangle\" about a page holding two of them, and a reader using the note to tell a \
             missing box from a missed node was told the page had neither. Both numbers now \
             cover both table populations, and the note's prose names the third cause it counts. \
             See CHANGELOG \"0.40.2\".\n\n\
             Moved a SIXTY-SEVENTH time at 0.41.0, on `parser_version` alone for THIS profile, \
             as part of renaming the product from `ethos-engine` to `ethos-parser`. This \
             vector's `backend.name` is `lopdf` and did not move: the PDF profile names the \
             library that reads the bytes, not the crate that calls it. The eight office \
             profiles are the ones the rename actually touched — their `BackendIdentity.name` \
             went `engine-office` -> `ethos-parser-office`, so a DOCX, XLSX, PPTX, ODT, ODS, \
             ODP, RTF or EPUB artifact moves for two reasons where a PDF artifact moves for \
             one. Nothing about what any document says changed. See CHANGELOG \"0.41.0\".\n\n\
             Moved a SIXTY-EIGHTH time at 0.42.0 (D4-S2), and this one is the first widening of what a \
             text run SAYS since v1-S6.1: `reading_order_rule` goes `gutter-columns-v1` -> \
             `gutter-columns-v2`, and nothing else in this vector moves. The cut is unchanged \
             — same constants, same recursion, and the fifteen ordering tests were not edited \
             — so two artifacts either side of this hash list the same runs in the same \
             sequence. What differs is that the newer one carries the regions the cut made, as \
             `TextRunAttributes::region`, wherever a page divided. That is why the id moved \
             rather than staying put on the grounds that the order held: an artifact naming \
             `-v1` promises no such field, and a reader who could not tell the two apart could \
             not tell an undivided page from an older build. `docs/16-D4-SCOPE.md` §10.\n\n\
             Moved a SIXTY-NINTH time at 0.42.1 (D4-S5), on `parser_version` alone. No rule id \
             moves and no capability flag moves: the cut, the detectors and the projections are \
             untouched, and every artifact this build writes for a document 0.42.0 could read is \
             byte-identical to the one 0.42.0 wrote — measured across all eight gate documents \
             before the version moved. What changed is which documents produce an artifact AT \
             ALL. `GeometryAbsence::MeasuredOffPage` gives the PDF reader a spelling for a box \
             it measured correctly and the document draws outside its own page, so six DP-Bench \
             documents that exited 2 with no artifact now seal. The version moves because that \
             is a real difference between two builds — a reader holding an artifact needs to \
             know whether the absence of one is a document this engine could not read or a \
             document it refused — and because a `measured_off_page` value is one 0.42.0 could \
             never emit. See CHANGELOG \"0.42.1\".\n\n\
             Moved a SEVENTIETH time at 0.43.0 (v2.2-S0), and it is the first time BOTH \
             projection ids move together: `markdown_rule` goes `markdown-blocks-v2` -> \
             `markdown-blocks-v4` and `html_rule` goes `html-blocks-v2` -> `html-blocks-v4`. \
             They are separate ids so that they CAN move apart, which is not a promise that \
             they always will — this change went through `heading_level`, which both \
             projections call, so a document that differs under one differs under the other. \
             What differs: an EPUB whose XHTML declares `<h1>` projects `# ` and `<h1>` where \
             `-v2` projected a paragraph and `<p>`. No PDF artifact changes — the tagged \
             `/H1`..`/H6` path is untouched — and no representation changes at all, because \
             this is a projection rule and the wire the projections read did not move.\n\n\
             Moved a SEVENTY-FIRST time at 0.44.0 (v2.2-S1), and both projection ids move again \
             together: `markdown-blocks-v3` -> `-v4` and `html-blocks-v3` -> `-v4`. The rule that \
             changed is one both projections call, as at 0.43.0, so the pair moving in step is the \
             same fact twice rather than two coincidences.\n\n\
             What differs: a text run is no longer its own block. `nist-sp-800-207` projected as \
             68 112 Markdown blocks averaging TWO characters — \"NIST Special Publication \
             800-207\" arrived as forty of them — and now projects as 4 698 averaging 35. Runs \
             join when the document itself put them in one marked-content sequence and either drew \
             a space between them or drew no gap at all; anything else still breaks the block, \
             because a missed join reads as two words the page drew and a wrong join invents one. \
             **No representation changes** — this is a projection rule, and `extract` output is \
             byte-identical at equal version.\n\n\
             Moved a SEVENTY-SECOND time at 0.45.0 (v2.2-S2), and this one moves on \
             `parser_version` ALONE. **No rule id moves, because no rule governs a limitation** — \
             the profile names the rules that decide what an artifact contains, and the set of \
             limitations it declares is not one of them. Saying so is the point: a reader who \
             found only the version different could otherwise conclude nothing had changed. \
             What differs: a `/Subtype /Form` XObject the page painted with `Do` now produces a \
             document-scoped `form-xobjects-not-descended` carrying a count. Before this, the \
             placement was discarded and the artifact said nothing, so a page whose whole content \
             is `q /Xf1 Do Q` emitted zero nodes with `pages_failed: 0` and a consumer could not \
             tell it from a blank page. The profile-scoped `form-xobject-text-not-descended` is \
             untouched and never covered this: it rides on EVERY artifact this engine writes, \
             including ones for documents holding no XObject at all.\n\n\
             Moved a SEVENTY-THIRD time at 0.46.0 (v2.2-S3), on `parser_version` alone again, and \
             this one is the largest CORRECTNESS move since 0.42.1. `load_widths` asked a /Type0 \
             font dictionary for `/Widths` — a key PDF 32000-1 §9.7.4.3 never puts there, because \
             a composite font's widths live on its descendant CIDFont as `/W` with `/DW` as the \
             default — so every composite font reported an unknown advance while the document \
             supplied a perfectly good one. What differs on the wire: those runs now carry an \
             `advance`, and with an advance they carry a MEASURED ink box instead of a typed \
             absence, so they reach `ethos.grounding.v1` where they used to be omitted from it. \
             Measured on 200 documents: ungroundable text nodes 14 683 -> 8 770 of 109 500. No \
             text is gained or lost and the node count is identical — only what can be expressed \
             downstream changed, which is exactly the axis this engine exists on.\n\n\
             Moved a SEVENTY-FOURTH time at 0.46.1 (v2.2-S4), and this one changes NOTHING a \
             reader can observe. The version moves because `parser_version` is a profile field \
             and a build is a build; the slice is guards. `xhtml_heading_level`'s h2..h6 arms \
             were reached by no test at all — replacing them with `None` failed zero of ~1 300 — \
             so 0.43.0's declared-heading feature was verified at one level of six. It is now \
             verified at all six in both projections, and the mapping was already correct, which \
             is why nothing moves but the number.\n\n\
             Moved a SEVENTY-FIFTH time at 0.47.0 (v2.2-S5), and this one moves BOTH projection \
             rule ids — `markdown-blocks-v4` -> `-v5`, `html-blocks-v4` -> `-v5` — because the \
             clauses live in `markdown.rs` and `html.rs` calls them. Until now a run the document \
             declared nothing about joined with nothing, so an untagged PDF projected one block \
             per run: a median of TWO characters per block across 981 OmniDocBench documents, \
             with 163 of 733 documents at 95% or more sub-3-character blocks holding 52% of all \
             the text. Those runs now join when they are the next ink along one baseline — same \
             page, same region, same stream, same `origin_y`, no gap the page drew. What differs \
             on the wire: fewer blocks, and two new census codes, \
             `baseline-run-joins-abutted-v1` and `baseline-run-joins-spaced-v1`, counted apart \
             from `mcid-run-joins-v1` because a join this engine measured is a weaker claim than \
             one the producer declared. No text is gained or lost — the coverage census balances \
             on all 961 artifacts, 1 623 979 emitted + 106 184 dropped = 1 730 163 in \
             representation — and every one of the 40 engine fixtures is byte-identical.\n\n\
             Moved a SEVENTY-SIXTH time at 0.48.0 (v2.2-S6), on `parser_version` alone: two \
             correctness fixes, neither of which changes a rule's DEFINITION, so no rule id \
             moves. First, `hex_of` refused a hexadecimal string containing white space, which \
             PDF 32000-1 §7.3.4.3 says shall be ignored — and the refusal was document-fatal, so \
             one stray space in one font's `ToUnicode` cost the whole page. Two documents of 981 \
             hit it; both now read, one of them recovering 4 970 characters from nothing. \
             Second, a font that declares itself SYMBOLIC, supplies no `/ToUnicode` and names no \
             base encoding was decoded through `StandardEncoding` — which §9.6.6.2 gives to a \
             NONSYMBOLIC font — and said nothing about it. It still is, because the flag does not \
             separate the TeX math fonts where the decode is wrong from the ordinary prose fonts \
             that merely set the bit, and refusing both would drop correct text to fix a \
             minority. What changed is that the artifact now says so, under \
             `symbolic-font-builtin-encoding-assumed`, on 36 of 981 documents.\n\n\
             Moved a SEVENTY-SEVENTH time at 0.49.0 (v2.2-S7), on `parser_version` alone — there \
             is no grounding rule id to move, which is itself worth noticing. \
             `ethos.grounding.v1` offers two granularities, coarse citable ELEMENTS and finer \
             SPANS inside them, and v0 could populate only one of them: with no grouping, a run \
             WAS the element and WAS the span. A consumer wanting to highlight one quoted \
             sentence on `irs-fw9` therefore held 970 glyph-run rectangles and no rectangle for \
             the sentence. The element is now the BLOCK and the span stays the run, grouped by \
             `markdown::geometric_blocks` — the projections' own join clauses, called rather than \
             restated. What differs on the wire: fewer elements, the same spans, each naming its \
             block, and an element box that is the UNION of its members' measured boxes. Nothing \
             is inferred: a union of measured rectangles is measured, and an element's text is \
             its members' own characters concatenated, because a space the page drew is a run \
             with its own text. Runs per citable element: 13.55 across the gate corpus \
             (`nist-sp-800-207` 19.72), and 1.24 across OmniDocBench — the same statistic meaning \
             opposite things on a tagged and an untagged corpus, because where nothing is \
             declared only the baseline join fires.\n\n\
             Moved a SEVENTY-EIGHTH time at 0.50.0 (v2.2-S8), and this one changes NO ARTIFACT \
             BYTE — only the text of one refusal. Two different absences reached one message and \
             it named the wrong one for half of them: `/Identity-H` was refused with `Predefined \
             CMaps (the Adobe CJK set) are not vendored`, which is true of `/GBK-EUC-H` and false \
             of an identity CMap. §9.7.4.2 makes that mapping the identity, so the code IS the \
             CID and nothing about the CMap is missing; what is absent is CID to Unicode, which \
             here has no source because the font supplies no `/ToUnicode`. 8 of the 20 \
             OmniDocBench documents that produce no artifact are that kind, and the old sentence \
             would have sent a reader after a dataset that could not have helped them. MINOR \
             rather than PATCH because §4's rule is about bytes for the same input and stderr is \
             bytes; a reviewer who reads `output` as the artifact alone would call it a PATCH, \
             and 0.42.1's entry is why this errs the other way."
        );
        assert_eq!(
            Profile::default().profile_sha256().unwrap().to_string(),
            "sha256:0742f0c593da1220f68853b084e27c46fe6a70c2682d9be04b9ed8462f49add2"
        );
    }

    /// All four rules are named on the profile, and any one moving moves the identity.
    ///
    /// The reason `table_detection` stopped being a string at v1-S2, extended to the third rule at
    /// v1-S8 and the fourth at v2-S24. With one id, an artifact could not distinguish "looked for
    /// this kind of table and found none" from "never looked", and a reader comparing two such
    /// artifacts cell for cell would be comparing different detectors.
    #[test]
    fn the_profile_names_every_table_rule_and_any_one_moves_the_hash() {
        let base = Profile::default();
        assert_eq!(base.table_detection.ruled, TABLE_DETECTION_V3);
        assert_eq!(base.table_detection.unruled, TABLE_DETECTION_UNRULED_V1);
        assert_eq!(base.table_detection.stroke_ruled, TABLE_DETECTION_STROKE_V1);
        assert_eq!(base.table_detection.tagged, TABLE_DETECTION_TAGGED_V1);
        for (a, b) in [
            (TABLE_DETECTION_V3, TABLE_DETECTION_UNRULED_V1),
            (TABLE_DETECTION_V3, TABLE_DETECTION_STROKE_V1),
            (TABLE_DETECTION_V3, TABLE_DETECTION_TAGGED_V1),
            (TABLE_DETECTION_UNRULED_V1, TABLE_DETECTION_STROKE_V1),
            (TABLE_DETECTION_UNRULED_V1, TABLE_DETECTION_TAGGED_V1),
            (TABLE_DETECTION_STROKE_V1, TABLE_DETECTION_TAGGED_V1),
        ] {
            assert_ne!(
                a, b,
                "two inferences sharing one identity is what a versioned rule id exists to prevent"
            );
        }

        // All four appear on the wire, under their own keys.
        let s = String::from_utf8(base.canonical_bytes().unwrap()).unwrap();
        assert!(
            s.contains(
                r#""table_detection":{"ruled":"ruled-rects-v3","stroke_ruled":"stroke-ruled-v1","tagged":"tagged-tables-v1","unruled":"unruled-align-v1"}"#
            ),
            "{s}"
        );

        let mut ruled_moved = base.clone();
        // A NEXT id, not the current one. This read `"ruled-rects-v2"` until v2-S20 promoted that
        // string to the default's predecessor and a bulk rename made the line assert that the
        // default differs from itself — which it does not, and the test said so.
        ruled_moved.table_detection.ruled = "ruled-rects-v4".into();
        assert_ne!(hash(&base), hash(&ruled_moved), "the ruled id is identity");

        let mut unruled_moved = base.clone();
        unruled_moved.table_detection.unruled = "unruled-align-v2".into();
        assert_ne!(
            hash(&base),
            hash(&unruled_moved),
            "and so is the unruled one — a run that inferred grids from alignment produced \
             different tables from one that did not"
        );

        // The third, and it was missing. v1-S8 added `stroke_ruled`, added it to the three
        // assertions above, and did not add it here — so from v1-S8 to v2-S13.1 the name said
        // *any one* over two of the three rules, and the one left out was the one just added.
        let mut stroke_moved = base.clone();
        stroke_moved.table_detection.stroke_ruled = "stroke-ruled-v2".into();
        assert_ne!(
            hash(&base),
            hash(&stroke_moved),
            "and so is the stroke-ruled one — a run that read the page's ruling lines emitted \
             tables a run that did not could not have"
        );

        // The fourth, added at v2-S24. A run that reads the structure tree's `/Table` tags emits
        // tables a run that only reads ink cannot, so its id is identity too.
        let mut tagged_moved = base.clone();
        tagged_moved.table_detection.tagged = "tagged-tables-v2".into();
        assert_ne!(
            hash(&base),
            hash(&tagged_moved),
            "and so is the tagged one — a run that reads the document's own table tags emitted \
             tables a run that read only ink could not have"
        );

        // And no knob is another: moving one must not produce a second one's digest.
        assert_ne!(hash(&ruled_moved), hash(&unruled_moved));
        assert_ne!(hash(&ruled_moved), hash(&stroke_moved));
        assert_ne!(hash(&ruled_moved), hash(&tagged_moved));
        assert_ne!(hash(&unruled_moved), hash(&stroke_moved));
        assert_ne!(hash(&unruled_moved), hash(&tagged_moved));
        assert_ne!(hash(&stroke_moved), hash(&tagged_moved));
    }

    /// An unknown rule id fails closed rather than deserializing with the key dropped.
    ///
    /// `deny_unknown_fields` on a plain struct denies for real. v0.1 measured that an
    /// internally-tagged enum does not: it buffers through a map and discards keys it does not
    /// know, so a profile from a newer engine would re-hash to a digest different from the one it
    /// arrived with — claiming a comparability it does not have.
    #[test]
    fn an_unknown_table_rule_key_is_refused() {
        // The probe key was `tagged` until v2-S24 made that a real field — the exact promotion
        // this test exists to catch, and it caught it: the assertion failed the moment the key
        // stopped being unknown, and the probe moved to a fifth key instead of being deleted.
        let bad = r#"{"ruled":"ruled-rects-v3","unruled":"unruled-align-v1","stroke_ruled":"stroke-ruled-v1","tagged":"tagged-tables-v1","recognized":"x-v1"}"#;
        assert!(
            serde_json::from_str::<TableDetection>(bad).is_err(),
            "a fifth rule id must fail closed, not vanish and change the hash"
        );

        // And a profile MISSING the field v1-S8 added is refused too, rather than defaulted into
        // one that claims a rule it never ran.
        let stale = r#"{"ruled":"ruled-rects-v3","unruled":"unruled-align-v1"}"#;
        assert!(
            serde_json::from_str::<TableDetection>(stale).is_err(),
            "a pre-S8 profile must not silently acquire the stroke-ruled rule"
        );

        // The same claim one field later: a profile from before v2-S24 must be refused, not
        // defaulted into one that says the tagged rule ran when it did not.
        let pre_tagged = r#"{"ruled":"ruled-rects-v3","unruled":"unruled-align-v1","stroke_ruled":"stroke-ruled-v1"}"#;
        assert!(
            serde_json::from_str::<TableDetection>(pre_tagged).is_err(),
            "a pre-S24 profile must not silently acquire the tagged rule"
        );

        let good = r#"{"ruled":"ruled-rects-v3","unruled":"unruled-align-v1","stroke_ruled":"stroke-ruled-v1","tagged":"tagged-tables-v1"}"#;
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
