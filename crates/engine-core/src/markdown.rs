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

//! Safe Markdown: a projection of the representation that stays citable (v1.1-S1, S2).
//!
//! # Why a Markdown module exists here at all, after seven slices of refusing one
//!
//! `docs/01-CONTRACT.md` §12 says this contract *does not define a Markdown projection*, and gives
//! the reason — **Workbench rule 8: retrieval operates on the evidence record itself, and any
//! projection between what is ranked and what is cited is where a locator dies silently.** A
//! pipeline chunks Markdown, embeds it, ranks it, and hands the winner to a model; the model
//! quotes it; the citation then has to bind back to a run or a cell, and the Markdown threw that
//! away. Nobody notices, because the quote is real text and the answer looks right.
//!
//! The parity checklist calls that **O8 — Markdown only with the Anchor Map** — and records the
//! fallback plainly: *rule 8 prefers no projection at all.*
//!
//! v1.1 does not lift the objection. It pays it. The map is not a companion file somebody can
//! forget; it is a **field of the same artifact**, and the type will not construct without it.
//!
//! # The four laws, and where each one is enforced
//!
//! 1. **Never one without the other.** [`MarkdownArtifact`] holds `markdown` and `anchor_map`
//!    together. There is no constructor for one alone and no `--md-only` on the CLI.
//! 2. **The map total-tiles the bytes.** [`AnchorMap::new`] refuses anything that does not cover
//!    `0..markdown.len()` exactly — unsorted, overlapping, gapped, out of range, or empty against
//!    non-empty Markdown. The same check re-runs on parse, so a hand-edited file cannot smuggle a
//!    hole past the type.
//! 3. **Two segment kinds, and only two.** [`SegmentKind::Source`] inverts to node text;
//!    [`SegmentKind::Syntax`] is markup this exporter invented and says so. There is no
//!    "probably source" — that is a confidence field wearing a different hat (§9).
//! 4. **Coverage is a census.** `emitted + dropped == in_representation`, in *characters*, with
//!    every dropped character in a named bucket carrying a count. Checklist **A14**: if something
//!    is removed, the artifact says so and says how much.
//!
//! # What v1.1-S2 added, and what it had to admit to
//!
//! S1 emitted every table character as linear text and declared the lost grid as
//! `markdown-table-structure-not-projected`. S2 projects the grid as a **GFM table** — and that
//! limitation is gone, deleted rather than reworded, because a stale limitation is acted on.
//!
//! GFM is not a table model. It has no `rowspan`, no `colspan`, and its `---` row always reads as
//! a header. So the flattening costs something on every real document, and A14 obliges the
//! artifact to say *how much* rather than footnote it. That is [`Coverage::structural_erasures`]:
//! named codes with integer counts, in the same artifact, next to the character census.
//!
//! **A separate census from the character one, deliberately.** The characters are all still there
//! — a cell's text is a concatenation of runs that are already nodes, so nothing is dropped and a
//! `tables-flattened` character bucket would read `0` and disclose nothing (S1 decision 5). What a
//! merge costs is *slots*, and what the separator row costs is a *header claim the document never
//! made*. Neither is a number of characters, so neither is counted as one.
//!
//! # The consumer's rule, in one line
//!
//! **A quote that touches a `syntax` byte is not invertible.** It may be a fine thing to show a
//! human; it is not a citation. That is mechanical, which is the whole point — on a two-node
//! document the Markdown contains `First line\n\nSecond line`, and `line\n\nSecond` is real text
//! in the Markdown that the document never drew. From the `.md` alone nothing distinguishes it
//! from a sentence the page contains.
//!
//! # What this module is not
//!
//! - **Not a second IR.** Input is [`crate::DocumentRepresentation`]. No node id is minted here; a
//!   `source` segment names ids that already exist.
//! - **Not a layout engine.** A heading is a heading because the structure tree said so. No font
//!   size is consulted anywhere in this file — checklist L29 is REFUSE.
//! - **Not a verifier.** Nothing here produces a verdict (`docs/07-VERIFY-BOUNDARY.md`).

use serde::{Deserialize, Serialize};

use crate::{
    c14n::c14n_bytes, sha256_hex_bytes, ArtifactIdentity, DocumentRepresentation, EngineError,
    NodeKind, Sha256Hex,
};

/// The artifact type this module emits.
pub const MARKDOWN_ARTIFACT_TYPE: &str = "ethos.markdown.v1";

/// Semantic version of the artifact *shape*, independent of the parser build.
///
/// `1.1.0` at v1.1-S2: `coverage` gained `structural_erasures`. Additive, so a `1.0.0` reader
/// that ignores unknown keys still reads every field it knew — but a reader that *needs* to know
/// whether a merge was flattened can tell the two shapes apart, which is the only reason a shape
/// version exists.
pub const MARKDOWN_SCHEMA_VERSION: &str = "1.1.0";

/// The projection rule v1.1-S2 ships: block structure — GFM tables and tagged lists — with
/// headings still only from the structure tree.
///
/// A versioned id for the same reason every detector has one: it decides what comes out. A run
/// that projected headings from font sizes and a run that refused to would disagree about the
/// same document, and an artifact whose hash could not tell them apart would claim a
/// comparability it lacks.
///
/// **The id moved from `markdown-linear-v1` at S2, and the string is the only place it is
/// spelled.** A document with a table comes out differently under the two rules — one linear
/// paragraph per cell run, versus a grid — so a reader holding two artifacts must be able to see
/// which rule produced each. Bumping the parser version alone would not have said it: the
/// projection rule is what changed.
pub const MARKDOWN_RULE_BLOCKS_V1: &str = "markdown-blocks-v1";

// -------------------------------------------------------------------------------------------
// The structural erasures GFM causes, as codes
// -------------------------------------------------------------------------------------------

/// Slots that lost a merge, because GFM has no `rowspan` or `colspan`.
///
/// A cell spanning two columns is expanded to [`crate::CellSlot`]s: the text lives in the origin
/// slot and every other slot it covered comes out **empty**. The count is the number of those
/// other slots — `rowspan × colspan - 1`, summed over the table's cells.
///
/// **Not zero, and not a character lie.** The gold declares no spans on the v1 corpus, but real
/// documents merge constantly, and the shipped Ethos ODL adapter's defect (memo §16) is exactly
/// this erasure performed silently. Here it has a number.
pub const GFM_SPAN_SLOTS_UNREPRESENTABLE: &str = "gfm-span-slots-unrepresentable-v1";

/// A GFM table's `---` row reads as a header row, and the document did not say it was one.
///
/// GFM has no headerless table: the delimiter row after row 0 is required by the grammar, and
/// every renderer draws row 0 as a header because of it. So projecting *any* table asserts
/// something about row 0.
///
/// **The representation carries no header declaration at all** — neither detector reads `/TH`,
/// and [`crate::TaggedGridCheck`] compares grids rather than cell types — so this fires for every
/// table that becomes GFM. It is written as a condition rather than a constant because the day a
/// slice carries `/TH` through, the count drops on documents that declare one, and nothing here
/// needs rewording for that to be true.
///
/// **Counted once per table, not once per cell in row 0.** The erasure is one claim — *this
/// table has a header* — made once about one table. Counting its cells would make a wide table
/// look like a worse lie than a narrow one, when both told exactly one.
pub const GFM_ROW_ZERO_SEPARATOR: &str = "gfm-row-zero-separator-v1";

/// A run two cells both claimed, emitted under the first and not the second.
///
/// Reachable, not theoretical: overlapping painted rectangles produce lattice faces claimed
/// twice, which is what `fixtures/engine/ruled-table-overlap` exists to hold. A run emitted in
/// both cells would appear **twice** in the Markdown, and the census would then count one node's
/// characters two times — the projection's one arithmetical promise, broken to make a table look
/// tidy. So the first cell in row-major order keeps it and this counts the rest.
pub const GFM_CELL_RUN_CLAIMED_TWICE: &str = "gfm-cell-run-claimed-twice-v1";

/// A cell with no slot in the GFM grid: outside the declared dimensions, or on a slot another
/// cell already originates in.
///
/// **Its runs are not erased** — they fall through to the linear pass and project as paragraphs,
/// so every character still reaches the Markdown and the census still balances. What is lost is
/// the cell's place in the grid, which is why this is counted here and not as characters.
pub const GFM_CELL_NOT_PLACED: &str = "gfm-cell-not-placed-v1";

/// A table with no grid to project — zero rows or zero columns.
///
/// Should not occur on a sealed artifact; both detectors build a lattice before they build a
/// table. Counted rather than asserted because the alternative to a count here is a panic on a
/// record this engine did not write.
pub const GFM_TABLE_NOT_PROJECTED: &str = "gfm-table-not-projected-v1";

/// Runs appended to a list item that was already open, joined on a guess the tree did not make.
///
/// PDF 32000 pairs one `/Lbl` with one `/LBody`, so a body run directly after its label is the
/// **same item** by the standard and is not counted. A second body run is different: the role
/// path of two sibling `/LI`s is identical, so nothing in the representation distinguishes "the
/// rest of this item" from "the next item". This projection joins, and says how often.
pub const GFM_LIST_ITEM_RUN_JOINS: &str = "gfm-list-item-run-joins-v1";

// -------------------------------------------------------------------------------------------
// The map
// -------------------------------------------------------------------------------------------

/// What one stretch of the Markdown string is.
///
/// **Two values, deliberately.** A third — "probably source", "source-ish", a score — would be the
/// confidence field `docs/01-CONTRACT.md` §9 forbids, and it would put the consumer back in the
/// position this whole version exists to remove: guessing whether a quote is citable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    /// Bytes that invert to canonical node text.
    ///
    /// The segment names the representation node id(s) these bytes came from, so a quote falling
    /// wholly inside one is traceable to evidence without re-parsing anything.
    Source,
    /// Markup this exporter invented — `#`, blank lines, list bullets, fences.
    ///
    /// The document did not draw these bytes. A quote touching one is **not** invertible, and the
    /// artifact says so here rather than leaving a consumer to infer it from the text's shape.
    Syntax,
}

/// One stretch of the Markdown string, as `[start, end)` **UTF-8 byte offsets**.
///
/// # Bytes, not characters, and the field names say so
///
/// The consumer's job is to slice the string it was given. Every language's slice takes bytes or
/// takes chars, and saying which — here, in the artifact, and in `docs/11-V11-MILESTONES.md` — is
/// the difference between a map and a hint. Integers, like every other number this project emits
/// (§4: no float appears in canonical output).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    /// What these bytes are.
    pub kind: SegmentKind,
    /// First byte, inclusive.
    pub start: usize,
    /// Last byte, exclusive.
    pub end: usize,
    /// The representation nodes these bytes came from, in emission order.
    ///
    /// Present on [`SegmentKind::Source`] and absent on [`SegmentKind::Syntax`], because syntax
    /// came from no node. **Never a newly minted id** — every value here already exists in the
    /// representation this artifact projects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_ids: Vec<String>,
}

/// The map from Markdown bytes back to evidence.
///
/// Constructed only through [`AnchorMap::new`], which is what makes law 2 structural rather than
/// procedural: there is no way to hold an `AnchorMap` whose segments do not tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnchorMap {
    /// Sorted, contiguous, non-overlapping, covering the whole string.
    pub segments: Vec<Segment>,
}

impl AnchorMap {
    /// The map for `markdown`, or a named reason these segments do not describe it.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] when the segments do not **total-tile** `markdown`: a gap, an
    /// overlap, an out-of-order pair, an empty segment, an offset past the end, or an offset that
    /// is not a UTF-8 character boundary.
    ///
    /// **A hole is worse than no map**, because the hole is exactly where an unquotable byte
    /// hides, so every one of those is a hard error rather than a repair.
    pub fn new(segments: Vec<Segment>, markdown: &str) -> Result<Self, EngineError> {
        let len = markdown.len();
        let bad = |detail: String| EngineError::Malformed {
            what: "anchor map".into(),
            detail,
        };

        if segments.is_empty() {
            if len == 0 {
                return Ok(Self { segments });
            }
            return bad(format!(
                "no segments for {len} byte(s) of markdown; the map must tile the whole string"
            ))
            .pipe_err();
        }

        let mut cursor = 0usize;
        for (i, s) in segments.iter().enumerate() {
            if s.end <= s.start {
                return bad(format!(
                    "segment {i} is [{}, {}), which is empty or inverted",
                    s.start, s.end
                ))
                .pipe_err();
            }
            if s.start != cursor {
                return bad(format!(
                    "segment {i} starts at {} but the previous segment ended at {cursor} — the \
                     map must be contiguous, with no gap and no overlap",
                    s.start
                ))
                .pipe_err();
            }
            if s.end > len {
                return bad(format!(
                    "segment {i} ends at {} but the markdown is {len} byte(s)",
                    s.end
                ))
                .pipe_err();
            }
            if !markdown.is_char_boundary(s.start) || !markdown.is_char_boundary(s.end) {
                return bad(format!(
                    "segment {i} is [{}, {}), which splits a UTF-8 character; a segment that \
                     cannot be sliced cannot be quoted",
                    s.start, s.end
                ))
                .pipe_err();
            }
            if matches!(s.kind, SegmentKind::Syntax) && !s.node_ids.is_empty() {
                return bad(format!(
                    "segment {i} is syntax and names {} node id(s); syntax came from no node",
                    s.node_ids.len()
                ))
                .pipe_err();
            }
            if matches!(s.kind, SegmentKind::Source) && s.node_ids.is_empty() {
                return bad(format!(
                    "segment {i} is source and names no node; a source segment that cannot say \
                     where it came from is not invertible and must be syntax"
                ))
                .pipe_err();
            }
            cursor = s.end;
        }

        if cursor != len {
            return bad(format!(
                "the segments cover {cursor} of {len} byte(s); the map must tile the whole string"
            ))
            .pipe_err();
        }

        Ok(Self { segments })
    }

    /// The segments a byte range touches, in order.
    ///
    /// The consumer-side primitive behind law 3: a quote is invertible when every segment this
    /// returns is [`SegmentKind::Source`].
    pub fn segments_touching(&self, start: usize, end: usize) -> Vec<&Segment> {
        self.segments
            .iter()
            .filter(|s| s.start < end && start < s.end)
            .collect()
    }

    /// Whether a byte range inverts to evidence — every byte of it is `source`.
    ///
    /// An empty or inverted range is **not** invertible: a citation that selects nothing is not a
    /// citation, and answering `true` would be the one wrong answer a caller cannot detect.
    pub fn is_invertible(&self, start: usize, end: usize) -> bool {
        if end <= start {
            return false;
        }
        let touched = self.segments_touching(start, end);
        !touched.is_empty()
            && touched
                .iter()
                .all(|s| matches!(s.kind, SegmentKind::Source))
    }
}

/// A tiny helper so the tiling checks read as one expression each.
trait PipeErr {
    fn pipe_err<T>(self) -> Result<T, EngineError>;
}

impl PipeErr for EngineError {
    fn pipe_err<T>(self) -> Result<T, EngineError> {
        Err(self)
    }
}

// -------------------------------------------------------------------------------------------
// Coverage
// -------------------------------------------------------------------------------------------

/// One named class of source characters that did not reach the Markdown.
///
/// **A count, not an adjective.** Checklist A14: if something is removed, the artifact says so and
/// says how much. `"some content was omitted"` is not a disclosure;
/// `annotation-text-not-projected-v1: 37` is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DroppedBucket {
    /// Stable machine-readable reason.
    pub code: String,
    /// How many characters of node text this class accounts for.
    pub chars: usize,
    /// How many nodes.
    pub nodes: usize,
}

/// One named class of **structure** the projection could not carry, with a count.
///
/// # Why this is not a [`DroppedBucket`]
///
/// A dropped bucket counts *characters*, and the erasures GFM causes are not characters. A merged
/// cell's text is emitted in full; what the projection cannot represent is that the cell covered
/// four slots rather than one. Putting that in the character census would require inventing a
/// number of characters for it, and the honest number is `0` — which is how A14's own example of
/// a bad disclosure reads.
///
/// So: a second census, same artifact, integers only, each code documented at its constant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralErasure {
    /// Stable machine-readable reason. One of the `GFM_*` constants in this module.
    pub code: String,
    /// How many of them. **Never a severity and never a ratio** (`docs/01-CONTRACT.md` §9).
    pub count: usize,
}

/// The census: every source character the representation offered, accounted for.
///
/// # Characters, not bytes and not glyphs
///
/// The thing being conserved is *the text the representation offered*, so the unit is
/// `text.chars().count()` over node text. Bytes would make the number depend on the encoding of
/// the script rather than on the projection, and glyphs are not a thing this engine counts
/// anywhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    /// Sum of `text.chars().count()` over **every** node in the representation.
    pub source_chars_in_representation: usize,
    /// Characters that landed in a [`SegmentKind::Source`] segment.
    pub source_chars_emitted: usize,
    /// Characters that did not, summed over [`Coverage::dropped`].
    pub source_chars_dropped: usize,
    /// Where the dropped characters went, by named class, sorted by code.
    pub dropped: Vec<DroppedBucket>,
    /// Structure the projection flattened, by named class, sorted by code (v1.1-S2).
    ///
    /// Empty on a document with no tables and no tagged lists — an **empty array rather than an
    /// absent key**, so "nothing was flattened" and "this artifact does not track flattening" are
    /// distinguishable without reading the schema version.
    pub structural_erasures: Vec<StructuralErasure>,
}

impl Coverage {
    /// Whether the census balances.
    ///
    /// Asserted in CI on real fixtures rather than trusted: an unbalanced census is the failure
    /// this type exists to make visible, and a type that could hold one silently would be the
    /// footnote law 4 refuses.
    pub fn balances(&self) -> bool {
        self.source_chars_emitted + self.source_chars_dropped == self.source_chars_in_representation
            && self.dropped.iter().map(|b| b.chars).sum::<usize>() == self.source_chars_dropped
    }
}

// -------------------------------------------------------------------------------------------
// The artifact
// -------------------------------------------------------------------------------------------

/// Markdown and the map that makes it citable — **one artifact, both halves**.
///
/// There is no constructor for the Markdown alone, and no accessor that hands out the string
/// without the map beside it. That is law 1 made structural: a companion file is a thing a
/// pipeline strips, and a field is not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkdownArtifact {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    #[serde(flatten)]
    pub identity: ArtifactIdentity,
    /// Digest of the original source bytes, carried through from the representation.
    pub source_sha256: Sha256Hex,
    /// Digest of the representation this is a projection of.
    ///
    /// **The binding that makes the map checkable.** A consumer holding this artifact and the
    /// representation can confirm they are the same record; without it, a map could name node ids
    /// from a different parse of a different file and read exactly the same.
    pub representation_sha256: Sha256Hex,
    /// Which projection rule produced this.
    pub markdown_rule: String,
    /// The Markdown. Never emitted without [`Self::anchor_map`].
    pub markdown: String,
    /// The map from `markdown` bytes back to representation nodes.
    pub anchor_map: AnchorMap,
    /// The census of what reached the Markdown and what did not.
    pub coverage: Coverage,
}

impl MarkdownArtifact {
    /// The canonical bytes this artifact serializes to.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        let value = serde_json::to_value(self).map_err(|e| EngineError::Malformed {
            what: "markdown artifact".into(),
            detail: e.to_string(),
        })?;
        c14n_bytes(&value).map_err(|e| EngineError::Malformed {
            what: "markdown artifact".into(),
            detail: e.to_string(),
        })
    }

    /// Re-check both laws on an artifact that came from bytes.
    ///
    /// Parsing runs the same tiling check construction does, so a hand-edited file cannot smuggle
    /// a hole past the type, and the census is re-balanced for the same reason.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the map does not tile or the census does not balance.
    pub fn validate(&self) -> Result<(), EngineError> {
        AnchorMap::new(self.anchor_map.segments.clone(), &self.markdown)?;
        if !self.coverage.balances() {
            return Err(EngineError::Malformed {
                what: "markdown coverage".into(),
                detail: format!(
                    "{} emitted + {} dropped != {} in representation",
                    self.coverage.source_chars_emitted,
                    self.coverage.source_chars_dropped,
                    self.coverage.source_chars_in_representation
                ),
            });
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------
// The projection
// -------------------------------------------------------------------------------------------

/// The whitespace rule the projection runs, which is the table gate's rule.
///
/// NFC, then trim, then collapse every internal run of Unicode whitespace to a single `U+0020`.
/// The same three clauses `docs/table-gate-v1.md` publishes, for the same reason: a quote that has
/// to match must be produced by a rule the consumer can reproduce.
///
/// **This means a `source` segment's bytes are the normalization of `node.text`, not always
/// `node.text` verbatim.** Said out loud here and in the artifact's own documentation, because a
/// map that claimed exact bytes and delivered normalized ones would be the subtlest lie available
/// to this module.
pub fn normalize(s: &str) -> String {
    let trimmed = s.trim();
    let mut out = String::with_capacity(trimmed.len());
    let mut in_ws = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(ch);
            in_ws = false;
        }
    }
    out
}

/// Heading level from a tagged role path, or `None` for anything that is not a heading.
///
/// Reads the document's own `/S` types — `H`, `H1` … `H6` — after its `/RoleMap` has been applied,
/// which `crate::representation::PdfTaggedLocator` already carries as `standard_role_path`.
///
/// **No font size is consulted.** Checklist L29 is REFUSE, and a heading inferred from 14pt bold
/// is a claim about layout that no code in this repository makes.
fn heading_level(node: &crate::Node) -> Option<u8> {
    let Some(crate::StructuralLocator::PdfTagged(t)) = node.structural_locator.as_ref() else {
        return None;
    };
    let path = t.standard_role_path.as_ref().unwrap_or(&t.role_path);
    let last = path.last()?;
    match last.as_str() {
        "H" => Some(1),
        "H1" => Some(1),
        "H2" => Some(2),
        "H3" => Some(3),
        "H4" => Some(4),
        "H5" => Some(5),
        "H6" => Some(6),
        _ => None,
    }
}

/// Where a run sits in a tagged list, or `None` for anything that is not in one.
///
/// Reads the document's own `/S` types — `L`, `LI`, `Lbl`, `LBody` (PDF 32000 §14.8.4.3) — after
/// its `/RoleMap` has been applied.
///
/// **An `L` ancestor is required, not just an `Lbl` leaf.** `/Lbl` is also the label of a table of
/// contents item and of a note, and a projection that turned those into bullets would be
/// inventing a list the document did not draw — the same mistake as reading a heading off a font
/// size, one structure level up.
///
/// **No bullet glyph, no hanging indent, no font name.** An untagged document grows no list here;
/// its bullet characters are runs like any other and project as the text they are.
fn list_role(node: &crate::Node) -> Option<ListRole> {
    let Some(crate::StructuralLocator::PdfTagged(t)) = node.structural_locator.as_ref() else {
        return None;
    };
    let path = t.standard_role_path.as_ref().unwrap_or(&t.role_path);
    let label = match path.last()?.as_str() {
        "Lbl" => true,
        "LBody" | "LI" => false,
        _ => return None,
    };
    // Nesting comes from nested `/L`, which is how PDF spells a sub-list. Depth 0 is the
    // outermost list, and an `Lbl` with no `/L` above it is not a list item at all.
    let lists = path.iter().filter(|s| s.as_str() == "L").count();
    Some(ListRole {
        depth: lists.checked_sub(1)?,
        label,
    })
}

/// A run's position in a tagged list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ListRole {
    /// Nesting depth, zero-based: one `/L` deep is `0`.
    depth: usize,
    /// Whether this run is the item's `/Lbl` — the marker the document itself drew.
    label: bool,
}

/// The bucket a non-projected node belongs to.
///
/// One per node kind rather than one catch-all, because each is a different fact about the
/// document and collapsing them would put "we dropped 40 characters" where "a reviewer's note and
/// a form value are different things" belongs.
fn dropped_code(kind: NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::TextRun => None,
        // v1-S4: a field's `/V` lives in the AcroForm tree and no content stream draws it. That
        // slice exists to keep it distinguishable from page text; pasting it into a Markdown body
        // would undo exactly that.
        NodeKind::FormField => Some("form-field-values-not-projected-v1"),
        // A reviewer's note is markup OVER the document. A consumer who cannot tell it from the
        // page's own words cannot cite either one safely.
        NodeKind::Annotation => Some("annotation-text-not-projected-v1"),
        // A placement, not a picture — zero characters. The bucket is almost always 0 and exists
        // so the census is exhaustive by construction rather than by the reader trusting that an
        // image node carries no text.
        NodeKind::Image => Some("image-nodes-carry-no-text-v1"),
    }
}

// -------------------------------------------------------------------------------------------
// The emitter
// -------------------------------------------------------------------------------------------

/// The string being built and the map being built beside it, in lockstep.
///
/// Every byte of the Markdown goes through [`Emit::syntax`] or [`Emit::source`], which is what
/// makes law 2 hold **by construction** rather than by a final audit: there is no `push_str` in
/// the projection that does not also record what it pushed.
struct Emit {
    markdown: String,
    segments: Vec<Segment>,
    /// Characters that landed in a `source` segment, which is `coverage.source_chars_emitted`.
    emitted_chars: usize,
}

impl Emit {
    fn new() -> Self {
        Self {
            markdown: String::new(),
            segments: Vec::new(),
            emitted_chars: 0,
        }
    }

    /// Bytes this exporter invented. Coalesced with an immediately preceding syntax segment.
    ///
    /// **Coalescing is cosmetic and safe**: two adjacent syntax segments and one syntax segment
    /// say exactly the same thing about every byte, and a GFM row would otherwise produce four
    /// segments per empty cell. It can never merge across a `source` segment, because the match
    /// requires the previous segment to end where this one starts *and* to be syntax.
    fn syntax(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let start = self.markdown.len();
        self.markdown.push_str(s);
        let end = self.markdown.len();
        match self.segments.last_mut() {
            Some(last) if last.kind == SegmentKind::Syntax && last.end == start => last.end = end,
            _ => self.segments.push(Segment {
                kind: SegmentKind::Syntax,
                start,
                end,
                node_ids: Vec::new(),
            }),
        }
    }

    /// Bytes that invert to `node`'s canonical text.
    ///
    /// **Never coalesced**, even with an adjacent source segment naming the same node: a segment
    /// is the unit a consumer inverts, and merging two would claim a contiguity in the document
    /// that only exists in this string.
    fn source(&mut self, s: &str, node: &str) {
        if s.is_empty() {
            return;
        }
        let start = self.markdown.len();
        self.markdown.push_str(s);
        self.segments.push(Segment {
            kind: SegmentKind::Source,
            start,
            end: self.markdown.len(),
            node_ids: vec![node.to_string()],
        });
        self.emitted_chars += s.chars().count();
    }

    /// A node's text inside a GFM cell, with the two characters GFM reads as structure escaped.
    ///
    /// # The backslash is syntax and the character it escapes is source
    ///
    /// GFM ends a cell at `|`, so a cell holding `a|b` has to be written `a\|b`. Emitting `\|` as
    /// one `source` segment would be the lie this module exists to avoid — the map would claim the
    /// document drew a backslash it never drew, and a consumer slicing that segment would get two
    /// characters where the page has one.
    ///
    /// So the escape is `syntax` and the escaped character stays `source`. A quote containing the
    /// pipe still inverts; a quote that reaches back over the backslash does not, which is the
    /// correct answer for a byte the exporter invented.
    ///
    /// **`\` is escaped too, not only `|`.** A cell whose text is literally `\|` would otherwise
    /// come out as `\` + `\|`, and GFM reads that as an escaped backslash followed by a *live*
    /// pipe — the cell would split at a character the document merely printed.
    fn cell_source(&mut self, text: &str, node: &str) {
        let mut rest = text;
        while let Some(at) = rest.find(['|', '\\']) {
            self.source(&rest[..at], node);
            self.syntax("\\");
            let width = rest[at..].chars().next().map_or(0, char::len_utf8);
            self.source(&rest[at..at + width], node);
            rest = &rest[at + width..];
        }
        self.source(rest, node);
    }
}

/// What the last block emitted was, so the separator between two of them is the right one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    /// A paragraph, a heading, or a table: separated from anything by a blank line.
    Standalone,
    /// A list item at this depth. Two list items are separated by **one** newline whatever their
    /// depths, because a blank line between them makes GFM a *loose* list — every item wrapped in
    /// a paragraph — and because a deeper item on the next line is exactly how GFM spells a
    /// sub-list.
    ListItem(usize),
}

/// Write the separator between the previous block and the one about to be written.
///
/// Nothing at the start of the document; a single newline between two list items; a blank line
/// everywhere else. All `syntax` — the document drew no separators, this exporter did, and a
/// quote spanning one is the string v1.1 exists to mark unquotable.
fn separate(e: &mut Emit, last: &mut Option<Block>, next: Block) {
    match (*last, next) {
        (None, _) => {}
        (Some(Block::ListItem(_)), Block::ListItem(_)) => e.syntax("\n"),
        _ => e.syntax("\n\n"),
    }
    *last = Some(next);
}

/// Project a representation into Markdown plus its map.
///
/// # The rule, in full — `markdown-blocks-v1`
///
/// 1. **Text runs only.** Every other node kind is dropped into its own named bucket. **Page
///    artifacts are NOT dropped**: a running head is a `text_run` carrying
///    `structural_locator: pdf_artifact`, and checklist O21/O22 is explicit that a reader deleting
///    running heads has silently edited the document. The flag stays in the representation and a
///    consumer that wants them gone drops them itself, knowing it did.
/// 2. **A heading when the tree says so**, `#` through `######`; a paragraph otherwise.
/// 3. **A GFM table** for every table on the representation, at the position of the first run one
///    of its cells claims. A run emitted inside a table is **not** also emitted as a paragraph —
///    the characters move from linear source to cell source, they are not duplicated and they are
///    not dropped.
/// 4. **A list item** for a run the structure tree places in an `/L`; nothing for a bullet
///    glyph, a hanging indent or a font name.
/// 5. **Blank line between blocks**, which is `syntax` — the document drew no such bytes.
/// 6. **Node text is normalized** by [`normalize`], and a node whose normalization is empty
///    contributes no segment at all rather than an empty one.
///
/// # Where the tables go
///
/// At the position of the **first run in reading order that any of the table's cells claims** —
/// not at the end of the document, and never interleaved with the cell text as paragraphs. A
/// table whose cells enclose no run at all has no position in reading order, so it is emitted
/// after the text, in document order; it contributes syntax and nothing else.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the map this builds does not tile its own output, which would be
/// a defect in this function rather than in its input — checked anyway, because law 2 is the one
/// thing a consumer cannot verify for itself without the source document.
pub fn to_markdown(
    repr: &DocumentRepresentation,
    parser_version: &str,
    profile_sha256: &Sha256Hex,
    markdown_rule: &str,
) -> Result<MarkdownArtifact, EngineError> {
    let payload = repr.payload();

    let mut e = Emit::new();
    let mut buckets: std::collections::BTreeMap<&'static str, (usize, usize)> =
        std::collections::BTreeMap::new();
    let mut erasures: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    let mut in_representation = 0usize;

    let plans = plan_tables(payload, &mut erasures);
    // Which table, if any, owns each run. A run in this map is emitted **inside** its table and
    // nowhere else; that is decision 1 of v1.1-S2, and it is the difference between a projection
    // that moves characters and one that duplicates them.
    let mut owner: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (t, plan) in plans.iter().enumerate() {
        for slot in &plan.slots {
            for node in slot {
                owner.insert(node.id.as_str(), t);
            }
        }
    }

    let mut emitted_tables = vec![false; plans.len()];
    let mut last: Option<Block> = None;
    // The list item currently open, and whether the previous run in it was the item's own `/Lbl`.
    let mut open_item: Option<(usize, bool)> = None;

    for node in &payload.nodes {
        in_representation += node.text.chars().count();

        if let Some(code) = dropped_code(node.kind) {
            let b = buckets.entry(code).or_insert((0, 0));
            b.0 += node.text.chars().count();
            b.1 += 1;
            continue;
        }

        // A cell's run. The table is emitted the first time one of its runs comes up in reading
        // order, and every later run of the same table is already inside it.
        if let Some(&t) = owner.get(node.id.as_str()) {
            if !emitted_tables[t] {
                emitted_tables[t] = true;
                separate(&mut e, &mut last, Block::Standalone);
                emit_table(&mut e, &plans[t]);
                open_item = None;
            }
            continue;
        }

        let text = normalize(&node.text);
        if text.is_empty() {
            // Whitespace-only runs exist (a `Tj` of spaces is a real operator). They contribute no
            // Markdown, and they contribute no dropped characters either: `normalize` removed
            // whitespace, and whitespace is not content this projection claims to have lost.
            continue;
        }

        if let Some(role) = list_role(node) {
            let continues = !role.label && open_item.map(|(d, _)| d) == Some(role.depth);
            if continues {
                // A second run inside an item the tree did not itself close. Joined here, and
                // counted there — see `GFM_LIST_ITEM_RUN_JOINS`.
                if !matches!(open_item, Some((_, true))) {
                    *erasures.entry(GFM_LIST_ITEM_RUN_JOINS).or_insert(0) += 1;
                }
                e.syntax(" ");
            } else {
                separate(&mut e, &mut last, Block::ListItem(role.depth));
                // Two spaces per level, which is what GFM reads as a sub-list. `syntax`, like
                // the marker: the document drew an indent in *coordinates*, not in spaces.
                for _ in 0..role.depth {
                    e.syntax("  ");
                }
                // **Always `- `, never `1. `.** The representation carries no `/ListNumbering`,
                // and choosing an ordered marker without one would be this exporter deciding the
                // document meant a numbered list. Where the document DID draw its own number, it
                // drew it as an `/Lbl` — which is source text, emitted right here, so the item
                // reads `- 1. First`. A doubled marker is ugly; deleting the document's own
                // characters to make it pretty is the erasure A14 is about.
                e.syntax("- ");
            }
            e.source(&text, node.id.as_str());
            open_item = Some((role.depth, role.label));
            last = Some(Block::ListItem(role.depth));
            continue;
        }

        open_item = None;
        separate(&mut e, &mut last, Block::Standalone);

        // The heading marker, when the tree said so. Also `syntax`: `## ` is this exporter's
        // rendering of a role, not bytes the page drew.
        if let Some(level) = heading_level(node) {
            e.syntax(&"#".repeat(level as usize));
            e.syntax(" ");
        }

        e.source(&text, node.id.as_str());
    }

    // A table whose cells enclose no run has no first run to sit behind, so it has no position in
    // reading order at all. Emitting it here is the one place the grid is not where the document
    // put it, and it is the only honest option left: the alternative is to drop a grid the
    // document drew because nobody wrote in it.
    for (t, plan) in plans.iter().enumerate() {
        if plan.projected && !emitted_tables[t] {
            separate(&mut e, &mut last, Block::Standalone);
            emit_table(&mut e, plan);
        }
    }

    // Trailing newline, so the string is a well-formed text file. `syntax`, like every other byte
    // this exporter chose.
    if !e.markdown.is_empty() {
        e.syntax("\n");
    }

    let emitted_chars = e.emitted_chars;
    let anchor_map = AnchorMap::new(e.segments, &e.markdown)?;
    let markdown = e.markdown;

    // **The whitespace the rule collapsed is a bucket, not a rounding.** `emitted` counts
    // NORMALIZED characters and normalization can only shrink a string, so measuring
    // `in_representation` over the same normalization would balance the census by moving the goal
    // posts — a run drawn as `Hello   world` would report 11 characters in a representation that
    // holds 13, and the two missing ones would be invisible.
    //
    // So the denominator is the RAW text of every node, and the difference gets its own named
    // class. Those characters really did not reach the Markdown; saying how many is what
    // checklist A14 asks for, and it costs one bucket that is usually zero.
    let collapsed: usize = in_representation
        - emitted_chars
        - buckets.values().map(|(chars, _)| *chars).sum::<usize>();
    if collapsed > 0 {
        let nodes = payload
            .nodes
            .iter()
            .filter(|n| {
                dropped_code(n.kind).is_none()
                    && normalize(&n.text).chars().count() != n.text.chars().count()
            })
            .count();
        buckets.insert("whitespace-collapsed-v1", (collapsed, nodes));
    }

    let dropped: Vec<DroppedBucket> = buckets
        .into_iter()
        .map(|(code, (chars, nodes))| DroppedBucket {
            code: code.to_string(),
            chars,
            nodes,
        })
        .collect();
    let dropped_chars = dropped.iter().map(|b| b.chars).sum();

    let coverage = Coverage {
        source_chars_in_representation: in_representation,
        source_chars_emitted: emitted_chars,
        source_chars_dropped: dropped_chars,
        dropped,
        // Only non-zero classes appear, the same way a dropped bucket does: a list of codes all
        // reading `0` is the disclosure-shaped noise a reader learns to skip, and the codes are
        // documented at their constants whether or not this document tripped them.
        structural_erasures: erasures
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(code, count)| StructuralErasure {
                code: code.to_string(),
                count,
            })
            .collect(),
    };

    let artifact = MarkdownArtifact {
        identity: ArtifactIdentity {
            artifact_type: MARKDOWN_ARTIFACT_TYPE.to_string(),
            schema_version: MARKDOWN_SCHEMA_VERSION.to_string(),
            parser_version: parser_version.to_string(),
            profile_sha256: profile_sha256.clone(),
        },
        source_sha256: payload.source.sha256.clone(),
        representation_sha256: repr.fingerprint().clone(),
        markdown_rule: markdown_rule.to_string(),
        markdown,
        anchor_map,
        coverage,
    };
    artifact.validate()?;
    Ok(artifact)
}

// -------------------------------------------------------------------------------------------
// Tables
// -------------------------------------------------------------------------------------------

/// How one table will be laid out as GFM, decided before a byte is written.
struct TablePlan<'a> {
    /// Whether this table becomes a GFM grid at all.
    projected: bool,
    /// The declared grid, row-major, `rows × columns` entries. Each is the ordered list of runs
    /// whose text belongs in that slot — empty for a slot a merge covers, for a hole, and for an
    /// origin cell that enclosed no run.
    slots: Vec<Vec<&'a crate::Node>>,
    rows: usize,
    columns: usize,
}

/// Decide the layout of every table, and count what GFM cannot say about them.
///
/// Runs are claimed here rather than during emission so that **claiming and emitting cannot
/// disagree**: the linear pass skips exactly the runs some slot will print, because it is reading
/// the same list this built.
fn plan_tables<'a>(
    payload: &'a crate::representation::RepresentationPayload,
    erasures: &mut std::collections::BTreeMap<&'static str, usize>,
) -> Vec<TablePlan<'a>> {
    let by_id: std::collections::BTreeMap<&str, &crate::Node> =
        payload.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    // Document-wide, not per table: two tables on one page can only claim the same run if the
    // detectors overlapped, and the first one still keeps it.
    let mut claimed: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    let mut plans = Vec::with_capacity(payload.tables.len());

    for table in &payload.tables {
        let (rows, columns) = (table.rows as usize, table.columns as usize);
        if rows == 0 || columns == 0 {
            // No grid to draw. The cells' runs are left unclaimed, so they project as paragraphs
            // and no character is lost — only the (empty) grid is.
            *erasures.entry(GFM_TABLE_NOT_PROJECTED).or_insert(0) += 1;
            plans.push(TablePlan {
                projected: false,
                slots: Vec::new(),
                rows,
                columns,
            });
            continue;
        }

        let mut slots: Vec<Vec<&crate::Node>> = vec![Vec::new(); rows * columns];
        let mut placed = vec![false; rows * columns];

        for cell in &table.cells {
            let (r, c) = (cell.position.row as usize, cell.position.column as usize);
            let index = r * columns + c;
            if r >= rows || c >= columns || placed[index] {
                // Outside the grid the table declares, or on a slot another cell already
                // originates in. Both are cross-check faults the artifact already reports; here
                // the consequence is only that this cell has nowhere to print, so its runs stay
                // unclaimed and come out as paragraphs instead.
                *erasures.entry(GFM_CELL_NOT_PLACED).or_insert(0) += 1;
                continue;
            }
            placed[index] = true;

            // **The merge, counted.** `rowspan × colspan - 1` slots held this cell and GFM cannot
            // say so; the text goes in the origin slot and the rest come out empty.
            let covered =
                (cell.position.rowspan as usize).saturating_mul(cell.position.colspan as usize);
            if covered > 1 {
                *erasures.entry(GFM_SPAN_SLOTS_UNREPRESENTABLE).or_insert(0) += covered - 1;
            }

            for id in &cell.node_ids {
                // A cell naming a node this record does not carry is refused at seal time
                // (`DocumentRepresentation::check_cell_runs_are_declared_nodes`), so a miss here
                // is a record that never existed rather than a case to guess at.
                let Some(node) = by_id.get(id.as_str()) else {
                    continue;
                };
                if claimed.insert(id.as_str()) {
                    slots[index].push(*node);
                } else {
                    *erasures.entry(GFM_CELL_RUN_CLAIMED_TWICE).or_insert(0) += 1;
                }
            }
        }

        // GFM's delimiter row makes row 0 a header on every renderer there is, and nothing in the
        // representation says the document declared one. Once per table — see the constant.
        *erasures.entry(GFM_ROW_ZERO_SEPARATOR).or_insert(0) += 1;

        plans.push(TablePlan {
            projected: true,
            slots,
            rows,
            columns,
        });
    }

    plans
}

/// Write one table as GFM: header row, delimiter row, body rows.
///
/// Every pipe, space, dash and newline is `syntax`; only a cell's text is `source`. So a quote
/// lifted out of a cell inverts to the runs it came from, and a quote that picked up the table
/// chrome around it does not — which is the right answer, because the document drew a ruling
/// line there and not a `|`.
fn emit_table(e: &mut Emit, plan: &TablePlan) {
    for row in 0..plan.rows {
        if row > 0 {
            e.syntax("\n");
        }
        e.syntax("|");
        for column in 0..plan.columns {
            e.syntax(" ");
            let mut first = true;
            for node in &plan.slots[row * plan.columns + column] {
                let text = normalize(&node.text);
                if text.is_empty() {
                    continue;
                }
                if !first {
                    // Two runs in one cell. The detector concatenates their raw text with nothing
                    // between; a space here is this exporter's, and saying so is why it is
                    // syntax — a quote spanning the join does not invert, exactly like the blank
                    // line between two paragraphs.
                    e.syntax(" ");
                }
                first = false;
                e.cell_source(&text, node.id.as_str());
            }
            e.syntax(" |");
        }
        if row == 0 {
            // The delimiter row, which is the whole reason `gfm-row-zero-separator-v1` exists.
            e.syntax("\n|");
            for _ in 0..plan.columns {
                e.syntax(" --- |");
            }
        }
    }
}

/// The digest of an artifact's canonical bytes, for callers that want to pin one.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the artifact will not canonicalize.
pub fn fingerprint(artifact: &MarkdownArtifact) -> Result<Sha256Hex, EngineError> {
    let bytes = artifact.to_canonical_bytes()?;
    Sha256Hex::parse(format!("sha256:{}", sha256_hex_bytes(&bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assurance::{Assurance, Limitation};
    use crate::derivation::GeometryPresence;
    use crate::representation::{
        NativeLocator, Node, NodeAttributes, NodeGeometry, PageRecord, PdfLocator,
        PdfTaggedLocator, ProcessingRun, ProcessorIdentity, RepresentationPayload, SourceIdentity,
        StructuralLocator, TextRunAttributes,
    };
    use crate::{
        Capabilities, CoordinateSystem, DerivationClass, IdAllocator, IdKind, NodeId, Profile,
        QRect,
    };

    fn seg(kind: SegmentKind, start: usize, end: usize, ids: &[&str]) -> Segment {
        Segment {
            kind,
            start,
            end,
            node_ids: ids.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn src(start: usize, end: usize) -> Segment {
        seg(SegmentKind::Source, start, end, &["s1"])
    }

    fn syn(start: usize, end: usize) -> Segment {
        seg(SegmentKind::Syntax, start, end, &[])
    }

    // ---------------------------------------------------------------------------------------
    // Law 2 — the map total-tiles the bytes
    // ---------------------------------------------------------------------------------------

    /// The shape the projection actually produces.
    #[test]
    fn a_map_that_tiles_is_accepted() {
        let md = "Hello Ethos\n";
        let m = AnchorMap::new(vec![src(0, 11), syn(11, 12)], md).expect("tiles");
        assert_eq!(m.segments.len(), 2);
    }

    /// **A hole is worse than no map**, because the hole is where an unquotable byte hides.
    #[test]
    fn a_gap_is_refused() {
        let md = "Hello Ethos\n";
        let err = AnchorMap::new(vec![src(0, 5), syn(6, 12)], md).expect_err("gap");
        assert!(format!("{err}").contains("contiguous"), "{err}");
    }

    #[test]
    fn an_overlap_is_refused() {
        let md = "Hello Ethos\n";
        let err = AnchorMap::new(vec![src(0, 8), syn(5, 12)], md).expect_err("overlap");
        assert!(format!("{err}").contains("contiguous"), "{err}");
    }

    #[test]
    fn a_short_map_is_refused() {
        let md = "Hello Ethos\n";
        let err = AnchorMap::new(vec![src(0, 11)], md).expect_err("short");
        assert!(format!("{err}").contains("tile the whole string"), "{err}");
    }

    #[test]
    fn a_map_running_past_the_end_is_refused() {
        let md = "Hi\n";
        let err = AnchorMap::new(vec![src(0, 99)], md).expect_err("past end");
        assert!(format!("{err}").contains("byte(s)"), "{err}");
    }

    #[test]
    fn no_segments_for_a_non_empty_string_is_refused() {
        let err = AnchorMap::new(vec![], "text").expect_err("empty map");
        assert!(format!("{err}").contains("tile the whole string"), "{err}");
    }

    /// An empty document maps to an empty map, which is the one case where zero segments tile.
    #[test]
    fn an_empty_string_takes_an_empty_map() {
        assert!(AnchorMap::new(vec![], "").is_ok());
    }

    /// **A segment that cannot be sliced cannot be quoted.**
    #[test]
    fn a_segment_splitting_a_utf8_character_is_refused() {
        let md = "é\n"; // 'é' is two bytes.
        let err = AnchorMap::new(vec![src(0, 1), syn(1, 3)], md).expect_err("mid-char");
        assert!(format!("{err}").contains("UTF-8"), "{err}");
    }

    // ---------------------------------------------------------------------------------------
    // Law 3 — two kinds, and each says what it can do
    // ---------------------------------------------------------------------------------------

    /// A source segment that cannot say where it came from is not invertible, so it is not source.
    #[test]
    fn a_source_segment_naming_no_node_is_refused() {
        let err = AnchorMap::new(vec![seg(SegmentKind::Source, 0, 2, &[])], "hi")
            .expect_err("no node ids");
        assert!(format!("{err}").contains("not invertible"), "{err}");
    }

    /// Syntax came from no node, so claiming one would be a false address.
    #[test]
    fn a_syntax_segment_naming_a_node_is_refused() {
        let err = AnchorMap::new(vec![seg(SegmentKind::Syntax, 0, 2, &["s1"])], "hi")
            .expect_err("syntax with ids");
        assert!(format!("{err}").contains("came from no node"), "{err}");
    }

    /// **The consumer's rule, mechanically.** A quote inside a source segment inverts; one that
    /// touches invented markup does not.
    #[test]
    fn a_quote_touching_syntax_is_not_invertible() {
        let md = "First line\n\nSecond line\n";
        let m = AnchorMap::new(
            vec![
                src(0, 10),
                syn(10, 12),
                seg(SegmentKind::Source, 12, 23, &["s2"]),
                syn(23, 24),
            ],
            md,
        )
        .expect("tiles");

        assert!(m.is_invertible(0, 10), "a whole source segment inverts");
        assert!(m.is_invertible(2, 6), "and so does part of one");

        // `line\n\nSecond` is real text in the Markdown that the document never drew. This is the
        // string a consumer reading the `.md` alone cannot tell from a sentence on the page.
        let start = md.find("line\n\nSecond").expect("present in the markdown");
        assert!(
            !m.is_invertible(start, start + "line\n\nSecond".len()),
            "a quote spanning the blank line must not invert — the page drew no such bytes"
        );
        assert!(!m.is_invertible(10, 12), "and neither does the join alone");
        assert!(!m.is_invertible(5, 5), "an empty range is not a citation");
    }

    // ---------------------------------------------------------------------------------------
    // Law 4 — the census balances
    // ---------------------------------------------------------------------------------------

    #[test]
    fn an_unbalanced_census_is_refused_on_validate() {
        let mut a = artifact_of(simple_repr());
        a.coverage.source_chars_emitted += 1;
        let err = a.validate().expect_err("must not balance");
        assert!(format!("{err}").contains("in representation"), "{err}");
    }

    // ---------------------------------------------------------------------------------------
    // The whitespace rule
    // ---------------------------------------------------------------------------------------

    /// **The published rule is the one that runs** — the same three clauses `table-gate-v1.md`
    /// pins for the gate, pinned again here because a `source` segment's bytes are the
    /// normalization of `node.text` rather than `node.text` verbatim.
    #[test]
    fn the_published_whitespace_rule_is_the_one_the_projection_runs() {
        assert_eq!(normalize("  padded  "), "padded", "trim");
        assert_eq!(normalize("a\t\tb"), "a b", "collapse to one U+0020");
        assert_eq!(normalize("a\n b"), "a b", "newlines are whitespace too");
        assert_eq!(normalize("a\u{00a0}b"), "a b", "so is NBSP");
        assert_eq!(normalize("keep  inner"), "keep inner");
        assert_eq!(normalize("   "), "", "whitespace-only normalizes away");
        // NOT case folding, NOT punctuation stripping, NOT edit distance.
        assert_eq!(normalize("A,b"), "A,b");
    }

    // ---------------------------------------------------------------------------------------
    // The projection
    // ---------------------------------------------------------------------------------------

    fn payload(nodes: Vec<Node>, pages: Vec<PageRecord>) -> RepresentationPayload {
        payload_with_tables(nodes, pages, Vec::new())
    }

    fn payload_with_tables(
        nodes: Vec<Node>,
        pages: Vec<PageRecord>,
        tables: Vec<crate::TableRecord>,
    ) -> RepresentationPayload {
        let profile = Profile::default();
        let authorized = pages.len() as u32;
        let states: Vec<crate::assurance::PageStateEntry> = pages
            .iter()
            .map(|p| crate::assurance::PageStateEntry {
                index: p.index,
                state: crate::assurance::PageState::Processed,
            })
            .collect();
        RepresentationPayload {
            identity: ArtifactIdentity {
                artifact_type: crate::representation::REPRESENTATION_ARTIFACT_TYPE.into(),
                schema_version: crate::representation::REPRESENTATION_SCHEMA_VERSION.into(),
                parser_version: profile.parser_version.clone(),
                profile_sha256: profile.profile_sha256().unwrap(),
            },
            source: SourceIdentity {
                media_type: "application/pdf".into(),
                sha256: Sha256Hex::parse(format!("sha256:{}", "0".repeat(64))).unwrap(),
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
            tables,
            assurance: Assurance::new(
                Capabilities::V0,
                authorized,
                states,
                vec![Limitation::document(
                    crate::assurance::codes::RESOURCE_LIMIT_PAGES,
                    "test fixture",
                )],
            )
            .unwrap(),
        }
    }

    fn text_node(
        alloc: &mut IdAllocator,
        parent: &NodeId,
        ordinal: u32,
        text: &str,
        role: Option<&str>,
    ) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: parent.clone(),
            ordinal,
            text: text.into(),
            native_locator: NativeLocator::Pdf(PdfLocator {
                page: 1,
                origin_x: 7200,
                origin_y: 7200,
                advance: Some(1000),
            }),
            structural_locator: role.map(|r| {
                StructuralLocator::PdfTagged(PdfTaggedLocator {
                    mcid: 0,
                    role_path: vec!["Document".into(), r.into()],
                    standard_role_path: None,
                    element_id: None,
                })
            }),
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::TextRun(TextRunAttributes {
                char_codes: text.bytes().map(u32::from).collect(),
                scalar_code_mismatch: false,
                synthesized: Vec::new(),
                findings: Vec::new(),
                font_id: "F1".into(),
                font_size: 2400,
            }),
        }
    }

    fn repr_of(specs: &[(&str, Option<&str>)]) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 30000,
            height: 14400,
            rotation: 0,
        };
        let nodes: Vec<Node> = specs
            .iter()
            .enumerate()
            .map(|(i, (t, role))| text_node(&mut alloc, &page.id, i as u32 + 1, t, *role))
            .collect();
        let geometry = nodes
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
            })
            .collect();
        DocumentRepresentation::seal(payload(nodes, vec![page]), geometry).unwrap()
    }

    fn simple_repr() -> DocumentRepresentation {
        repr_of(&[("Hello Ethos", None)])
    }

    /// A node carrying a full role path rather than the one-role shorthand `text_node` takes.
    ///
    /// Lists need the whole chain — `Document/L/LI/LBody` — because depth is the number of `/L`
    /// in it and because an `Lbl` with no `/L` above it is deliberately **not** a list item.
    fn tagged_node(
        alloc: &mut IdAllocator,
        parent: &NodeId,
        ordinal: u32,
        text: &str,
        path: &[&str],
    ) -> Node {
        let mut n = text_node(alloc, parent, ordinal, text, Some("P"));
        n.structural_locator = Some(StructuralLocator::PdfTagged(PdfTaggedLocator {
            mcid: ordinal as i64,
            role_path: path.iter().map(|s| (*s).to_string()).collect(),
            standard_role_path: None,
            element_id: None,
        }));
        n
    }

    /// A representation whose nodes carry explicit role paths. `None` is an untagged run.
    fn repr_of_paths(specs: &[(&str, Option<&[&str]>)]) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 30000,
            height: 14400,
            rotation: 0,
        };
        let nodes: Vec<Node> = specs
            .iter()
            .enumerate()
            .map(|(i, (t, path))| match path {
                Some(p) => tagged_node(&mut alloc, &page.id, i as u32 + 1, t, p),
                None => text_node(&mut alloc, &page.id, i as u32 + 1, t, None),
            })
            .collect();
        let geometry = nodes
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
            })
            .collect();
        DocumentRepresentation::seal(payload(nodes, vec![page]), geometry).unwrap()
    }

    /// One cell of a hand-built table: position, spans, and which of the document's runs it holds.
    struct Cell {
        row: u32,
        column: u32,
        rowspan: u32,
        colspan: u32,
        /// Indices into the `texts` given to [`repr_with_table`].
        runs: Vec<usize>,
    }

    fn cell(row: u32, column: u32, runs: &[usize]) -> Cell {
        Cell {
            row,
            column,
            rowspan: 1,
            colspan: 1,
            runs: runs.to_vec(),
        }
    }

    fn spanning(row: u32, column: u32, rowspan: u32, colspan: u32, runs: &[usize]) -> Cell {
        Cell {
            row,
            column,
            rowspan,
            colspan,
            runs: runs.to_vec(),
        }
    }

    /// A representation with `texts` as untagged runs and one table over the named ones.
    ///
    /// The table's cells name **real node ids**, which is what v1.1-S2 added to the record and
    /// what the projection needs before it can emit a cell as `source` at all.
    fn repr_with_table(
        texts: &[&str],
        rows: u32,
        columns: u32,
        cells: &[Cell],
    ) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 30000,
            height: 14400,
            rotation: 0,
        };
        let nodes: Vec<Node> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| text_node(&mut alloc, &page.id, i as u32 + 1, t, None))
            .collect();
        let table_id = alloc.next(IdKind::Table).unwrap();
        let records: Vec<crate::TableCellRecord> = cells
            .iter()
            .map(|c| crate::TableCellRecord {
                id: alloc.next(IdKind::Element).unwrap(),
                position: crate::TableCellPosition {
                    row: c.row,
                    column: c.column,
                    rowspan: c.rowspan,
                    colspan: c.colspan,
                    table_id: table_id.clone(),
                },
                bbox: QRect::new(0, 0, 100, 100).unwrap(),
                text: c.runs.iter().map(|i| texts[*i]).collect(),
                node_ids: c.runs.iter().map(|i| nodes[*i].id.clone()).collect(),
            })
            .collect();
        let table = crate::TableRecord {
            id: table_id,
            page: page.id.clone(),
            bbox: QRect::new(0, 0, 100, 100).unwrap(),
            rows,
            columns,
            cells: records,
            derivation: DerivationClass::Computed,
            detection_rule: crate::TABLE_DETECTION_V2.to_string(),
            locator_check: crate::LocatorCheck {
                check_id: crate::LOCATOR_CHECK_V1.into(),
                check_version: "1".into(),
                outcome: crate::CheckStatus::Ok,
            },
            tagged_check: None,
        };
        let geometry = nodes
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
            })
            .collect();
        DocumentRepresentation::seal(
            payload_with_tables(nodes, vec![page], vec![table]),
            geometry,
        )
        .unwrap()
    }

    /// The count a code carries, or `0` when the projection did not declare it.
    fn erasure(a: &MarkdownArtifact, code: &str) -> usize {
        a.coverage
            .structural_erasures
            .iter()
            .find(|e| e.code == code)
            .map_or(0, |e| e.count)
    }

    fn artifact_of(repr: DocumentRepresentation) -> MarkdownArtifact {
        let profile = Profile::default();
        to_markdown(
            &repr,
            &profile.parser_version,
            &profile.profile_sha256().unwrap(),
            &profile.markdown_rule,
        )
        .expect("projects")
    }

    #[test]
    fn one_run_projects_to_one_source_segment_and_a_trailing_newline() {
        let a = artifact_of(simple_repr());
        assert_eq!(a.markdown, "Hello Ethos\n");
        assert_eq!(a.anchor_map.segments.len(), 2);
        assert_eq!(a.anchor_map.segments[0].kind, SegmentKind::Source);
        assert_eq!(a.anchor_map.segments[1].kind, SegmentKind::Syntax);
        assert!(a.coverage.balances());
        assert_eq!(a.coverage.source_chars_emitted, 11);
        assert!(a.coverage.dropped.is_empty());
    }

    /// **The branch the corpus cannot reach.** No fixture in either corpus carries a heading role,
    /// so the only honest way to prove this path is a hand-built representation.
    #[test]
    fn a_heading_role_from_the_tree_projects_as_a_heading() {
        let a = artifact_of(repr_of(&[
            ("Chapter One", Some("H1")),
            ("Body text", None),
            ("Sub", Some("H3")),
        ]));
        assert_eq!(a.markdown, "# Chapter One\n\nBody text\n\n### Sub\n");

        // The marker is SYNTAX — `# ` is this exporter's rendering of a role, not bytes the page
        // drew — so a quote including it does not invert.
        let hash = a.markdown.find("# Chapter").unwrap();
        assert!(!a.anchor_map.is_invertible(hash, hash + 11));
        let title = a.markdown.find("Chapter One").unwrap();
        assert!(a.anchor_map.is_invertible(title, title + 11));
        assert!(a.coverage.balances());
    }

    /// **No font size is consulted.** The same text with no role is a paragraph, and the node's
    /// `font_size` is 2400 either way.
    #[test]
    fn a_big_font_is_not_a_heading() {
        let a = artifact_of(repr_of(&[("Chapter One", None)]));
        assert_eq!(a.markdown, "Chapter One\n");
        assert!(!a.markdown.contains('#'));
    }

    /// A role the document wrote that is not a heading stays a paragraph.
    #[test]
    fn a_non_heading_role_stays_a_paragraph() {
        let a = artifact_of(repr_of(&[("Just a para", Some("P"))]));
        assert_eq!(a.markdown, "Just a para\n");
    }

    /// The blank line between blocks is syntax, and the string it creates is not in the document.
    #[test]
    fn the_join_between_two_runs_is_syntax_and_is_not_quotable() {
        let a = artifact_of(repr_of(&[("First line", None), ("Second line", None)]));
        assert_eq!(a.markdown, "First line\n\nSecond line\n");
        let s = a.markdown.find("line\n\nSecond").unwrap();
        assert!(
            !a.anchor_map.is_invertible(s, s + "line\n\nSecond".len()),
            "the page drew no such string"
        );
    }

    /// A whitespace-only run contributes no Markdown and no dropped characters.
    #[test]
    fn a_whitespace_only_run_produces_nothing_and_loses_nothing_countable() {
        let a = artifact_of(repr_of(&[("Real text", None), ("   ", None)]));
        assert_eq!(a.markdown, "Real text\n");
        assert!(a.coverage.balances());
        // The three spaces ARE counted as dropped, in the collapse bucket, rather than vanishing.
        assert_eq!(a.coverage.source_chars_in_representation, 12);
        assert_eq!(a.coverage.source_chars_emitted, 9);
        assert_eq!(a.coverage.source_chars_dropped, 3);
    }

    /// **Collapsed whitespace is a named bucket, not a rounding.**
    #[test]
    fn collapsed_whitespace_is_counted_rather_than_absorbed() {
        let a = artifact_of(repr_of(&[("a   b", None)]));
        assert_eq!(a.markdown, "a b\n");
        assert!(a.coverage.balances());
        let b = a
            .coverage
            .dropped
            .iter()
            .find(|b| b.code == "whitespace-collapsed-v1")
            .expect("the two collapsed spaces are declared");
        assert_eq!(b.chars, 2);
    }

    /// Every artifact this module builds validates, on every fixture shape above.
    #[test]
    fn every_projection_tiles_and_balances() {
        for specs in [
            vec![],
            vec![("Only one", None)],
            vec![("a", None), ("b", None), ("c", None)],
            vec![("Head", Some("H2")), ("Body", None)],
            vec![("  spaced  ", None), ("x\t\ty", None)],
            vec![("héllo wörld", None)],
        ] {
            let a = artifact_of(repr_of(&specs));
            a.validate().expect("tiles and balances");
            // Independently: the segments really do reconstruct the string.
            let mut rebuilt = String::new();
            for s in &a.anchor_map.segments {
                rebuilt.push_str(&a.markdown[s.start..s.end]);
            }
            assert_eq!(rebuilt, a.markdown, "{specs:?}");
        }
    }

    /// An empty representation projects to an empty document with an empty map — not to a stray
    /// newline nobody's text produced.
    #[test]
    fn an_empty_representation_projects_to_nothing() {
        let a = artifact_of(repr_of(&[]));
        assert_eq!(a.markdown, "");
        assert!(a.anchor_map.segments.is_empty());
        assert!(a.coverage.balances());
    }

    /// Byte identity across runs — standing rule 6.
    #[test]
    fn two_projections_of_one_representation_are_byte_identical() {
        let repr = repr_of(&[("Alpha", Some("H1")), ("Beta", None)]);
        let a = artifact_of(repr.clone()).to_canonical_bytes().unwrap();
        let b = artifact_of(repr).to_canonical_bytes().unwrap();
        assert_eq!(a, b);
    }

    /// **No node id is minted here.** Every id a segment names already exists upstream.
    #[test]
    fn the_projection_mints_no_identifiers() {
        let repr = repr_of(&[("One", None), ("Two", Some("H1"))]);
        let known: std::collections::BTreeSet<String> = repr
            .payload()
            .nodes
            .iter()
            .map(|n| n.id.as_str().to_string())
            .collect();
        let a = artifact_of(repr);
        for s in &a.anchor_map.segments {
            for id in &s.node_ids {
                assert!(known.contains(id), "{id} is not a representation node");
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // v1.1-S2 — the GFM table
    // ---------------------------------------------------------------------------------------

    /// The shape, and the one property that makes it a projection rather than a copy: **a cell's
    /// characters appear once**.
    #[test]
    fn a_table_projects_as_gfm_and_its_runs_are_not_also_paragraphs() {
        let a = artifact_of(repr_with_table(
            &["Name", "Q1", "Alpha", "10"],
            2,
            2,
            &[
                cell(0, 0, &[0]),
                cell(0, 1, &[1]),
                cell(1, 0, &[2]),
                cell(1, 1, &[3]),
            ],
        ));
        assert_eq!(a.markdown, "| Name | Q1 |\n| --- | --- |\n| Alpha | 10 |\n");
        for text in ["Name", "Q1", "Alpha", "10"] {
            assert_eq!(
                a.markdown.matches(text).count(),
                1,
                "`{text}` must appear exactly once — a cell emitted as GFM AND as a paragraph \
                 would count one node's characters twice and the census would be arithmetic \
                 about a document nobody has"
            );
        }
        assert!(a.coverage.balances());
        assert_eq!(a.coverage.source_chars_emitted, 13);
    }

    /// The pipes, dashes and spaces are `syntax`; only the cell text is `source`.
    #[test]
    fn table_chrome_is_syntax_and_cell_text_is_source() {
        let a = artifact_of(repr_with_table(
            &["Name", "Q1", "Alpha", "10"],
            2,
            2,
            &[
                cell(0, 0, &[0]),
                cell(0, 1, &[1]),
                cell(1, 0, &[2]),
                cell(1, 1, &[3]),
            ],
        ));

        let alpha = a.markdown.find("Alpha").unwrap();
        assert!(
            a.anchor_map.is_invertible(alpha, alpha + 5),
            "a quote copied out of a cell must invert to the run that drew it"
        );

        // `| Name | Q1 |` is a plausible-looking string that the page never drew: the document
        // painted ruling lines, not pipes.
        let row = a.markdown.find("| Name").unwrap();
        assert!(
            !a.anchor_map.is_invertible(row, row + "| Name | Q1 |".len()),
            "table chrome is not page text and must not ground as if it were"
        );
        let sep = a.markdown.find("---").unwrap();
        assert!(!a.anchor_map.is_invertible(sep, sep + 3));
    }

    /// **The merge, counted.** GFM has no colspan, so the covered slot comes out empty — and the
    /// artifact says how many slots that cost rather than leaving it to be noticed.
    #[test]
    fn a_merged_cell_is_expanded_and_the_lost_slots_are_counted() {
        let a = artifact_of(repr_with_table(
            &["Beta", "n/a"],
            2,
            3,
            &[
                cell(0, 0, &[0]),
                spanning(1, 0, 1, 3, &[1]),
                cell(0, 1, &[]),
                cell(0, 2, &[]),
            ],
        ));
        assert_eq!(
            a.markdown,
            "| Beta |  |  |\n| --- | --- | --- |\n| n/a |  |  |\n"
        );
        assert_eq!(
            erasure(&a, GFM_SPAN_SLOTS_UNREPRESENTABLE),
            2,
            "a 1x3 cell held three slots and GFM can say one; two were lost"
        );

        // The origin cell's text is still source, and still inverts.
        let at = a.markdown.find("n/a").unwrap();
        assert!(a.anchor_map.is_invertible(at, at + 3));
        assert!(a.coverage.balances());
    }

    /// **Row 0 is claimed as a header the document never declared** — once per table.
    #[test]
    fn the_delimiter_row_declares_the_header_it_asserts() {
        let a = artifact_of(repr_with_table(
            &["A", "B"],
            2,
            1,
            &[cell(0, 0, &[0]), cell(1, 0, &[1])],
        ));
        assert!(a.markdown.contains("\n| --- |\n"));
        assert_eq!(
            erasure(&a, GFM_ROW_ZERO_SEPARATOR),
            1,
            "counted once per TABLE, not once per cell in row 0: the erasure is one claim about \
             one table, and counting its cells would make a wide table look like a worse lie"
        );

        // Pinned: a three-column table still counts 1.
        let wide = artifact_of(repr_with_table(
            &["A", "B", "C"],
            1,
            3,
            &[cell(0, 0, &[0]), cell(0, 1, &[1]), cell(0, 2, &[2])],
        ));
        assert_eq!(erasure(&wide, GFM_ROW_ZERO_SEPARATOR), 1);
    }

    /// **Trailing empties stay.** Truncating them is the competitor erasure A14 names: a prettier
    /// table and a different document.
    #[test]
    fn an_empty_trailing_row_and_column_are_not_truncated() {
        let a = artifact_of(repr_with_table(
            &["Only"],
            3,
            3,
            &[
                cell(0, 0, &[0]),
                cell(0, 1, &[]),
                cell(0, 2, &[]),
                cell(1, 0, &[]),
                cell(1, 1, &[]),
                cell(1, 2, &[]),
                cell(2, 0, &[]),
                cell(2, 1, &[]),
                cell(2, 2, &[]),
            ],
        ));
        assert_eq!(
            a.markdown,
            "| Only |  |  |\n| --- | --- | --- |\n|  |  |  |\n|  |  |  |\n"
        );
        assert_eq!(
            a.markdown.lines().count(),
            4,
            "three declared rows plus the delimiter"
        );
    }

    /// A `|` in the document's own text is escaped, and **the escape is syntax while the pipe is
    /// source** — so a quote containing the pipe still inverts.
    #[test]
    fn a_pipe_in_cell_text_is_escaped_without_claiming_the_document_drew_the_backslash() {
        let a = artifact_of(repr_with_table(&["A|B"], 1, 1, &[cell(0, 0, &[0])]));
        assert_eq!(a.markdown, "| A\\|B |\n| --- |\n");

        let a_at = a.markdown.find("A\\|B").unwrap();
        assert!(
            a.anchor_map.is_invertible(a_at, a_at + 1),
            "the `A` is source"
        );
        let escaped = a_at + 1; // the backslash
        assert!(
            !a.anchor_map.is_invertible(escaped, escaped + 1),
            "the backslash is this exporter's; the page drew no such character"
        );
        assert!(
            a.anchor_map.is_invertible(escaped + 1, escaped + 2),
            "and the pipe it escapes is still the document's own character"
        );

        // The census counts the pipe once and the backslash never: three characters in, three out.
        assert_eq!(a.coverage.source_chars_emitted, 3);
        assert!(a.coverage.balances());

        // A literal backslash is escaped too, or GFM would read the pipe after it as live.
        let b = artifact_of(repr_with_table(&["A\\|B"], 1, 1, &[cell(0, 0, &[0])]));
        assert_eq!(b.markdown, "| A\\\\\\|B |\n| --- |\n");
        assert_eq!(b.coverage.source_chars_emitted, 4);
    }

    /// A run two cells both claim is emitted under the first, and the second claim is counted.
    ///
    /// Reachable: overlapping painted rectangles produce contested lattice faces, which is what
    /// `fixtures/engine/ruled-table-overlap` holds.
    #[test]
    fn a_run_claimed_by_two_cells_is_emitted_once_and_the_second_claim_is_counted() {
        let a = artifact_of(repr_with_table(
            &["Shared"],
            1,
            2,
            &[cell(0, 0, &[0]), cell(0, 1, &[0])],
        ));
        assert_eq!(a.markdown, "| Shared |  |\n| --- | --- |\n");
        assert_eq!(erasure(&a, GFM_CELL_RUN_CLAIMED_TWICE), 1);
        assert_eq!(
            a.coverage.source_chars_emitted, 6,
            "six characters in the representation and six in the Markdown — emitting the run in \
             both cells would report twelve"
        );
        assert!(a.coverage.balances());
    }

    /// A cell with nowhere to go keeps its **text** — it falls through to the linear pass — and
    /// loses only its place in the grid, which is what is counted.
    #[test]
    fn a_cell_outside_the_declared_grid_projects_as_a_paragraph_and_is_counted() {
        let a = artifact_of(repr_with_table(
            &["In", "Out"],
            1,
            1,
            &[cell(0, 0, &[0]), cell(0, 9, &[1])],
        ));
        assert_eq!(a.markdown, "| In |\n| --- |\n\nOut\n");
        assert_eq!(erasure(&a, GFM_CELL_NOT_PLACED), 1);
        assert!(a.coverage.balances());
        assert_eq!(a.coverage.source_chars_emitted, 5);
    }

    /// A table with no grid to draw projects nothing, keeps every character, and says so.
    #[test]
    fn a_table_with_no_columns_is_declared_rather_than_drawn() {
        let a = artifact_of(repr_with_table(&["Text"], 0, 0, &[cell(0, 0, &[0])]));
        assert_eq!(a.markdown, "Text\n", "the run still reaches the Markdown");
        assert_eq!(erasure(&a, GFM_TABLE_NOT_PROJECTED), 1);
        assert!(a.coverage.balances());
    }

    /// **Reading order, not table order.** The grid sits where its first run sits.
    #[test]
    fn a_table_is_emitted_at_its_first_run_rather_than_at_the_end() {
        let a = artifact_of(repr_with_table(
            &["Before", "Cell", "After"],
            1,
            1,
            &[cell(0, 0, &[1])],
        ));
        assert_eq!(a.markdown, "Before\n\n| Cell |\n| --- |\n\nAfter\n");
    }

    /// A table nobody wrote in has no position in reading order, so it goes after the text.
    #[test]
    fn a_table_enclosing_no_run_is_emitted_after_the_body() {
        let a = artifact_of(repr_with_table(
            &["Body"],
            1,
            2,
            &[cell(0, 0, &[]), cell(0, 1, &[])],
        ));
        assert_eq!(a.markdown, "Body\n\n|  |  |\n| --- | --- |\n");
        assert!(a.coverage.balances());
    }

    /// Two runs in one cell are joined by a space **this exporter chose**, so a quote spanning
    /// the join does not invert — exactly like the blank line between two paragraphs.
    #[test]
    fn two_runs_in_one_cell_are_joined_by_syntax() {
        let a = artifact_of(repr_with_table(
            &["Total", "due"],
            1,
            1,
            &[cell(0, 0, &[0, 1])],
        ));
        assert_eq!(a.markdown, "| Total due |\n| --- |\n");
        let join = a.markdown.find("Total due").unwrap();
        assert!(
            !a.anchor_map.is_invertible(join, join + "Total due".len()),
            "the detector concatenates these runs with nothing between them; the space here is \
             this exporter's and a quote spanning it is not a citation"
        );
        assert!(a.anchor_map.is_invertible(join, join + 5));
    }

    // ---------------------------------------------------------------------------------------
    // v1.1-S2 — lists, from the tree or not at all
    // ---------------------------------------------------------------------------------------

    /// The shape: the document's own `/Lbl` text survives beside the marker this exporter added.
    #[test]
    fn a_tagged_list_projects_as_list_items() {
        let a = artifact_of(repr_of_paths(&[
            ("1.", Some(&["Document", "L", "LI", "Lbl"])),
            ("First item", Some(&["Document", "L", "LI", "LBody"])),
            ("2.", Some(&["Document", "L", "LI", "Lbl"])),
            ("Second item", Some(&["Document", "L", "LI", "LBody"])),
        ]));
        assert_eq!(a.markdown, "- 1. First item\n- 2. Second item\n");

        // `- ` is syntax; `1.` is the document's own text and inverts.
        let marker = a.markdown.find("- 1.").unwrap();
        assert!(!a.anchor_map.is_invertible(marker, marker + 2));
        assert!(a.anchor_map.is_invertible(marker + 2, marker + 4));
        assert!(a.coverage.balances());
    }

    /// Depth comes from **nested `/L`**, never from an indent or a bullet glyph.
    #[test]
    fn a_nested_list_indents_from_the_tree() {
        let a = artifact_of(repr_of_paths(&[
            ("Outer", Some(&["Document", "L", "LI", "LBody"])),
            (
                "Inner",
                Some(&["Document", "L", "LI", "LBody", "L", "LI", "LBody"]),
            ),
        ]));
        assert_eq!(a.markdown, "- Outer\n  - Inner\n");
    }

    /// **A second body run is a guess, and the artifact says how many it made.**
    #[test]
    fn a_second_body_run_joins_the_open_item_and_is_counted() {
        let a = artifact_of(repr_of_paths(&[
            ("3.", Some(&["Document", "L", "LI", "Lbl"])),
            ("Third item", Some(&["Document", "L", "LI", "LBody"])),
            ("continued", Some(&["Document", "L", "LI", "LBody"])),
        ]));
        assert_eq!(a.markdown, "- 3. Third item continued\n");
        assert_eq!(
            erasure(&a, GFM_LIST_ITEM_RUN_JOINS),
            1,
            "the label-to-body pairing is guaranteed by PDF 32000 and is NOT counted; the second \
             body run is the one the role path cannot distinguish from a sibling item"
        );

        // The label's own item is not a join.
        let paired = artifact_of(repr_of_paths(&[
            ("1.", Some(&["Document", "L", "LI", "Lbl"])),
            ("One", Some(&["Document", "L", "LI", "LBody"])),
        ]));
        assert_eq!(erasure(&paired, GFM_LIST_ITEM_RUN_JOINS), 0);
    }

    /// A run outside the list closes it, and the next block is separated by a blank line.
    #[test]
    fn a_non_list_run_ends_the_list() {
        let a = artifact_of(repr_of_paths(&[
            ("Item", Some(&["Document", "L", "LI", "LBody"])),
            ("Paragraph", None),
            ("Item again", Some(&["Document", "L", "LI", "LBody"])),
        ]));
        assert_eq!(a.markdown, "- Item\n\nParagraph\n\n- Item again\n");
    }

    /// **No list without a tree.** The refusal L29 makes about font sizes, one structure level up.
    #[test]
    fn an_untagged_bullet_is_not_a_list() {
        let a = artifact_of(repr_of(&[("\u{2022} Looks like a bullet", None)]));
        assert_eq!(a.markdown, "\u{2022} Looks like a bullet\n");
        assert!(
            !a.markdown.starts_with("- "),
            "a bullet glyph is a character the page drew, not a role it declared"
        );

        // And `/Lbl` outside a list — a table of contents entry, a note — is not one either.
        let toc = artifact_of(repr_of_paths(&[(
            "1.",
            Some(&["Document", "TOC", "TOCI", "Lbl"]),
        )]));
        assert_eq!(toc.markdown, "1.\n");
    }

    /// Every S2 shape tiles and balances, and the segments really do rebuild the string.
    #[test]
    fn every_block_projection_tiles_and_balances() {
        let cases = vec![
            repr_with_table(&["A", "B"], 1, 2, &[cell(0, 0, &[0]), cell(0, 1, &[1])]),
            repr_with_table(&["A|B"], 1, 1, &[cell(0, 0, &[0])]),
            repr_with_table(&["M"], 2, 2, &[spanning(0, 0, 2, 2, &[0])]),
            repr_with_table(&["x"], 2, 2, &[cell(0, 0, &[0])]),
            repr_with_table(&["p", "q"], 1, 1, &[cell(0, 0, &[1])]),
            repr_of_paths(&[
                ("1.", Some(&["Document", "L", "LI", "Lbl"])),
                ("a", Some(&["Document", "L", "LI", "LBody"])),
                ("b", Some(&["Document", "L", "LI", "LBody"])),
                ("tail", None),
            ]),
        ];
        for (i, repr) in cases.into_iter().enumerate() {
            let a = artifact_of(repr);
            a.validate().expect("tiles and balances");
            let mut rebuilt = String::new();
            for s in &a.anchor_map.segments {
                rebuilt.push_str(&a.markdown[s.start..s.end]);
            }
            assert_eq!(rebuilt, a.markdown, "case {i}");
            for s in &a.anchor_map.segments {
                match s.kind {
                    SegmentKind::Source => assert!(!s.node_ids.is_empty(), "case {i}"),
                    SegmentKind::Syntax => assert!(s.node_ids.is_empty(), "case {i}"),
                }
            }
        }
    }

    /// **No id is minted for a cell either.** Every id a table segment names is a node upstream.
    #[test]
    fn a_cell_segment_names_a_node_that_already_exists() {
        let repr = repr_with_table(&["A", "B"], 1, 2, &[cell(0, 0, &[0]), cell(0, 1, &[1])]);
        let known: std::collections::BTreeSet<String> = repr
            .payload()
            .nodes
            .iter()
            .map(|n| n.id.as_str().to_string())
            .collect();
        let a = artifact_of(repr);
        let mut source_segments = 0;
        for s in &a.anchor_map.segments {
            for id in &s.node_ids {
                source_segments += 1;
                assert!(known.contains(id), "{id} is not a representation node");
            }
        }
        assert_eq!(source_segments, 2);
    }

    /// A parsed artifact re-runs both laws, so a hand-edited file cannot smuggle a hole past.
    #[test]
    fn a_hand_edited_artifact_is_refused_on_parse() {
        let a = artifact_of(simple_repr());
        let mut v: serde_json::Value =
            serde_json::from_slice(&a.to_canonical_bytes().unwrap()).unwrap();
        // Delete the trailing-newline segment: the map now covers 11 of 12 bytes.
        v["anchor_map"]["segments"].as_array_mut().unwrap().pop();
        let parsed: MarkdownArtifact = serde_json::from_value(v).expect("still parses as JSON");
        let err = parsed.validate().expect_err("but does not validate");
        assert!(format!("{err}").contains("tile the whole string"), "{err}");
    }
}
