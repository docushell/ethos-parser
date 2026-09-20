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

//! Safe Markdown: a projection of the representation that stays citable (v1.1-S1, S2, S3).
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
//! # What v1.1-S3 added, and the thing it costs
//!
//! One cosmetic: a word the document broke across a line comes out closed up. `hyphen-` and
//! `ated` are two runs on the page and read as one word in the export.
//!
//! **The export joins; the evidence record does not.** `extract` still emits two `Extracted` runs
//! with the hyphen verbatim — `hyphenated_line_breaks_are_not_rejoined_and_that_is_the_policy` is
//! the standing test — because a rule that tells a soft break-hyphen from a real compound one
//! ("well-known" split across lines) needs a dictionary, and this project does not guess in the
//! record. `docs/history/10-V11-SCOPE.md` §5 puts cosmetics in the export or nowhere.
//!
//! So there is a quote that reads perfectly and does not ground: **`hyphenated`**, which no
//! element of `ethos.grounding.v1` contains. That is the correct answer and not a verifier defect
//! — the page drew two words. The artifact does not leave it to be inferred: the joined bytes are
//! **one `source` segment naming both runs**, and the hyphen that is no longer in the string is a
//! named character bucket, `hyphenation-rejoin-dropped-v1`, with a count. Law 4 still balances,
//! and a consumer that wants the citable strings has them in the map.
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
//!   **Amended 2026-09-18 by decision #29, which reversed row 29 for headings:** a heading is also
//!   one because the PDF reader measured the type an untagged page draws, and the artifact says
//!   which (`inferred_heading`, `headings-inferred-from-type`). This file still consults no font
//!   size — it reads the reader's flag, and the reader does the measuring.
//! - **Not a verifier.** Nothing here produces a verdict (`docs/07-VERIFY-BOUNDARY.md`).

use serde::{Deserialize, Serialize};

use crate::{
    sha256_hex_bytes, ArtifactIdentity, DocumentRepresentation, EngineError, NodeKind, Sha256Hex,
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

/// The projection rule in force: block structure — GFM tables and tagged lists — with headings
/// from what the document declares **or, since decision #29, from the type an untagged page
/// draws**, and a word broken across a line closed up.
///
/// A versioned id for the same reason every detector has one: it decides what comes out. A run
/// that projected headings from font sizes and a run that refused to would disagree about the
/// same document, and an artifact whose hash could not tell them apart would claim a
/// comparability it lacks. **That sentence was written as the argument against inferring a
/// heading; it is now the argument for moving this id when one is inferred** — `-v8` is the id
/// under which a `#` may be the engine's reading of type rather than the author's word, and it is
/// the whole of the disclosure a Markdown artifact can carry (`docs/28-HEADINGS-SCOPE.md` §5.3).
///
/// **One value per slice that changed what comes out**, and this string is the only
/// place the current one is spelled:
///
/// | slice | value | what it did that the one before did not |
/// | --- | --- | --- |
/// | v1.1-S1 | `markdown-linear-v1` | a table's cell runs as consecutive paragraphs, no grid |
/// | v1.1-S2 | `markdown-blocks-v1` | a GFM table, and a list item from a tagged `/L` |
/// | v1.1-S3 | `markdown-blocks-v4` | a word broken across a line closed up in the export |
/// | v2.2-S0 | `markdown-blocks-v3` | an EPUB's own `<h1>`..`<h6>` projects as a heading |
/// | v2.2-S1 | `markdown-blocks-v4` | runs in one marked-content sequence become one block |
/// | v2.2-S5 | `markdown-blocks-v7` | runs the document declared nothing about join along a baseline |
/// | C1 S2 | `markdown-blocks-v8` | a line the reader read as a heading from its type projects as `#` |
/// | v2.4 | `markdown-blocks-v10` | an ODT or ODP `<text:h>` projects at the level it declared |
/// | v2.4 | `markdown-blocks-v11` | a declared heading that projects as a paragraph is counted |
///
/// **The `slice` column above disagrees with the `value` column on two rows and did so before
/// this slice** — `v1.1-S3` is listed against `-v4` and `v2.2-S0` against `-v3`. Left as found
/// rather than silently corrected, because which id shipped in which release is a fact about
/// published artifacts and belongs to `CHANGELOG.md`, not to a guess made while editing.
///
/// A document with a table comes out differently under the first two; a document with a hyphenated
/// line break comes out differently under the last two — `hyphen-\n\nated` against `hyphenated`. A
/// reader holding two artifacts must be able to see which rule produced each, and bumping the
/// parser version alone would not have said it: the projection rule is what changed.
/// `-v9` at the block-join repair: `ink_sequenced` gained a third way to accept a gap — narrower
/// than the space the font itself draws on this page. A document set with tracking drew every
/// glyph as its own run with a few centipoints between the boxes, and every letter became its own
/// block: `01030000000103` projected 944 blocks of 1.2 characters. It now projects 48. Nothing
/// else moved, and a document whose fonts draw no space at all projects byte for byte what `-v8`
/// projected. Both projection ids move together, as they did at `-v8`, because the join lives in
/// `markdown` and `html` calls it.
///
/// `-v10` at the ODF heading slice: `heading_level` gained a **fourth** source, and a declared one
/// — the `text:outline-level` an ODT or ODP `<text:h>` states. A bump rather than a new name,
/// under the test this repository applies: a new name means different evidence, and a bump means
/// the same evidence, more of it. The evidence is unchanged — the document saying *heading, level
/// two* in as many words, which is what `/H2` and `<h2>` already said — and what moved is how many
/// formats can say it. It is the same move `-v3` made when EPUB's XHTML element name became a
/// source, made now for a third vocabulary.
///
/// A document with an ODT heading comes out differently: `# Evidence, not extraction.` where it
/// projected a bare paragraph before. Everything else projects byte for byte what `-v9` projected
/// — no PDF document projects a different byte, and a `<text:h>` that stated no level still
/// projects as a paragraph, because an absent `text:outline-level` is not level one.
///
/// `-v11` at the erasure declaration, and **it is the first move where not one character of
/// `markdown` or `html` changes.** The projection does exactly what `-v10` did; what ends is the
/// silence about it. A block the document called a heading that comes out as body text now
/// carries a count — [`HEADING_LEVEL_UNRESOLVED`] or [`HEADING_LEVEL_UNREPRESENTABLE`] — in
/// `coverage.structural_erasures`.
///
/// **The census is output.** `fixtures/office/presentation-pages/presentation.odp` yields
/// different `ethos.markdown.v1` bytes after this slice than before it, and
/// `crate::html`'s own header names the state that forbids leaving the id alone: *"a rule id that
/// stayed put while its output changed is the one dishonesty a version id exists to prevent."*
/// A bump rather than a new name, on the same test as every bump here: no new source is read and
/// no new fact is found — the condition counted is the one `odf_heading_level` already branches
/// on, which is *why* the block projects as a paragraph. Same evidence, counted.
pub const MARKDOWN_RULE_BLOCKS_V11: &str = "markdown-blocks-v11";

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
/// **The representation carries no header declaration at all** — no detector reads `/TH`,
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

/// Runs joined into one block because the document put them in one marked-content sequence
/// (v2.2-S1).
///
/// **Symmetric with [`GFM_LIST_ITEM_RUN_JOINS`], and the reason it is not optional.** That code
/// exists because joining two runs loses the boundary between them; this projection was already
/// declaring 14 863 such joins on one document while committing 78 233 more of the same kind
/// without a word. Counting one and not the other is the asymmetry, not the disclosure.
///
/// It is **not** a `GFM_*` code: GFM is not what causes it. The join is caused by the document
/// declaring a group and this exporter honouring it, which would be true of any output format.
pub const MCID_RUN_JOINS: &str = "mcid-run-joins-v1";

/// Runs joined into one block because they abut on one baseline, with no declaration to license
/// it — this engine's geometry, not the producer's (v2.2-S5). **No space anywhere**: the join
/// asserts the two runs are one word.
///
/// This is the fabrication-capable half and it is counted separately for that reason, following
/// [`dropped_code`]'s precedent of one code per kind rather than one catch-all. A reader
/// comparing this against [`MCID_RUN_JOINS`] is reading a per-document derivation profile: on a
/// tagged document the producer's declaration dominates; on an untagged one every boundary this
/// projection removed, it removed on geometry alone. An absent code means zero.
pub const BASELINE_RUN_JOINS_ABUTTED: &str = "baseline-run-joins-abutted-v1";

/// The same join where **the page itself drew the space** — in either run's bytes or as a
/// whitespace run of its own (v2.2-S5). The projection inserts one space and invents no word,
/// which is a materially weaker claim than [`BASELINE_RUN_JOINS_ABUTTED`] and so is not pooled
/// with it.
pub const BASELINE_RUN_JOINS_SPACED: &str = "baseline-run-joins-spaced-v1";

/// A block the document declared a **heading** whose depth this engine did not resolve, projected
/// as a paragraph (v2.4).
///
/// One per block, counted where the block opens. ODF makes `text:outline-level` optional, and a
/// `<text:h>` that omits it takes its depth from an outline style in `styles.xml` — a part the ODF
/// readers declare unread on the same artifact. So `heading_level` returns `None`, the block comes
/// out as body text, and the fact that the document called it a heading survives only on the
/// representation. This is the number that says how often.
///
/// **Distinct from [`HEADING_LEVEL_UNREPRESENTABLE`], and the axis is the one
/// `crate::assurance::codes::NON_TEXT_NODES_NOT_PROJECTED` names:** that one means the node was
/// read perfectly well and the target schema has nowhere to put it; this one is a gap in what was
/// *read*. The practical difference is a date. A slice that opens `styles.xml` drives this count
/// toward zero on the same documents; nothing will ever make `<h300>` an element. One integer over
/// both would tell a consumer deciding whether to wait for a better build of this engine exactly
/// the wrong thing.
///
/// **Not a character bucket.** Every character of the heading is emitted — only the `#` that was
/// never written is missing, and a `dropped` entry reading `0` is the disclosure-shaped noise
/// [`DroppedBucket`]'s own doc refuses. **Not counted on a block that opened none**: a `<text:h>`
/// whose text normalizes empty emits nothing at all, so there is no paragraph it was flattened
/// into and no erasure to declare.
///
/// Reachable today, on a committed fixture: `fixtures/office/presentation-pages/presentation.odp`
/// writes two bare `<text:h>` and trips this twice. Format-neutral by intent — `heading_level`'s
/// own documentation names DOCX as the next reader slice, and a `<w:pStyle>` whose built-in name
/// lives in `word/styles.xml` reaches this code with the same meaning and no rename.
pub const HEADING_LEVEL_UNRESOLVED: &str = "heading-level-unresolved-v1";

/// A block the document declared a heading **at a depth neither projection can write**, projected
/// as a paragraph (v2.4).
///
/// One per block, counted where the block opens. ODF types `text:outline-level` as a positive
/// integer and names no ceiling, and the reader carries what it finds rather than clamping a
/// document that is not broken — so `text:outline-level="300"` reaches the projection intact.
/// Markdown has six `#` depths and HTML six `<h>` elements. Emitting `#######` or `<h300>` would
/// be this exporter inventing a depth neither format has, so the block drops to body text and the
/// loss is counted here.
///
/// The complement of `1..=6` rather than "past six", which is why the name does not say so: a
/// record hand-built with level `0` lands here too. The ODF readers refuse a zero at read time, so
/// that path arrives only from a representation this engine did not write — which is
/// [`GFM_TABLE_NOT_PROJECTED`]'s situation and gets [`GFM_TABLE_NOT_PROJECTED`]'s answer, a count
/// rather than a panic.
///
/// `unrepresentable` is deliberately the word [`GFM_SPAN_SLOTS_UNREPRESENTABLE`] uses: both are
/// this census's target-format ceilings, and a reader who has read one should read the other
/// correctly. **Distinct from [`HEADING_LEVEL_UNRESOLVED`]** — see that code for the axis.
///
/// **No committed fixture trips it**, and it ships anyway for the reason
/// [`GFM_SPAN_SLOTS_UNREPRESENTABLE`] shipped at corpus zero: real documents are not the corpus.
/// An absent code means zero.
pub const HEADING_LEVEL_UNREPRESENTABLE: &str = "heading-level-unrepresentable-v1";

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
/// takes chars, and saying which — here, in the artifact, and in `docs/history/11-V11-MILESTONES.md` — is
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
        crate::c14n::canonical_bytes_of(self).map_err(|e| EngineError::Malformed {
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
///
/// Since v1.1-S3 there is one more departure, and it is the reason [`Segment::node_ids`] is a list:
/// a segment naming **two** nodes is a hyphenated line break closed up, so its bytes are the two
/// normalizations concatenated with the trailing `-` of the first removed. The removed character
/// is counted, in `hyphenation-rejoin-dropped-v1`. Inverting such a segment still lands on real
/// node text — both runs of it — which is what the page drew.
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

/// Heading level from what the **document declared**, or from what the reader **measured of its
/// type** where the document declared nothing — or `None` for anything that is not a heading.
///
/// Three sources, in this order. A PDF's own `/S` types — `H`, `H1` … `H6` — after its `/RoleMap`
/// has been applied, which `crate::representation::PdfTaggedLocator` already carries as
/// `standard_role_path`; an EPUB's own XHTML element name, which `crate::EpubBlockAttributes::element`
/// carries verbatim (missing until v2.2-S0); and, since decision #29, a run the PDF reader read as
/// a heading from its type, `crate::TextRunAttributes::inferred_heading`, which is always level 1.
///
/// **No font size is consulted here.** The first two sources are the document saying *heading,
/// level one* in as many words — `<h1>` exactly as `/H1` — and both are `Extracted`. The third is an
/// inference, and it is made where the type is measurable: in the reader, from the rendered em,
/// declared on the artifact as `headings-inferred-from-type`. This function reads the reader's
/// flag and never a size. The sentence it kept until decision #29 — *"a heading is a heading
/// because the document said so"* — narrows to **"or because this engine measured its type, and
/// the artifact says which"**.
///
/// **The declared sources win, structurally rather than by convention.** They return first, and
/// the reader never sets the flag on a document that declares author structure (its gate), so the
/// two can never both be present; a later edit reordering this function would still find the flag
/// absent wherever a declaration exists.
///
/// **ODT and ODP**, since v2.4, are a fourth source and a declared one: the `text:outline-level`
/// a `<text:h>` states, which `crate::OfficeParagraphAttributes::outline_level` and
/// `crate::OfficeOdfShapeAttributes::outline_level` carry. The slice that added them settled the
/// question this comment used to leave open — **an absent `text:outline-level` is not level one**,
/// because the level a bare `<text:h>` displays at comes from an outline style in `styles.xml`,
/// which the reader declares it did not open. So a heading that stated no level projects as a
/// paragraph rather than as `#`.
///
/// # What this does NOT reach, and why each is a different job
///
/// **ODS.** `crate::OdfBlockKind` reaches the wire for ODT and ODP and not for a spreadsheet: the
/// reader reads `heading` off the block and discards it when the block closes, and
/// `crate::OfficeOdfCellAttributes` has nowhere to put it. A level there would be a qualifier
/// outliving the thing it qualifies, so ODS carries neither.
///
/// **DOCX.** Earlier still: the reader keeps no `<w:pStyle>`, so no heading reaches the wire at all
/// and this function cannot see one to project. Two things stand in the way and the first is
/// structural: `docx.rs` has no `Event::Empty` arm, and `<w:pStyle/>` and `<w:outlineLvl/>` are
/// always self-closing, so they are invisible to that reader as written. The second is the
/// honesty question — `w:val="Heading1"` is a **styleId**, an author-chosen token, while the
/// built-in name ECMA-376 fixes lives in `<w:name>` inside `word/styles.xml`. Mapping the id to a
/// level without opening that part is matching a convention rather than reading a declaration, so
/// DOCX is a reader slice of its own and not a match arm here.
pub(crate) fn heading_level(node: &crate::Node) -> Option<u8> {
    if let crate::NodeAttributes::EpubBlock(e) = &node.attributes {
        return xhtml_heading_level(&e.element);
    }
    if let Some((block, level)) = odf_block(node) {
        return odf_heading_level(block, level);
    }
    if let Some(crate::StructuralLocator::PdfTagged(t)) = node.structural_locator.as_ref() {
        let path = t.standard_role_path.as_ref().unwrap_or(&t.role_path);
        let declared = match path.last().map(String::as_str) {
            Some("H" | "H1") => Some(1),
            Some("H2") => Some(2),
            Some("H3") => Some(3),
            Some("H4") => Some(4),
            Some("H5") => Some(5),
            Some("H6") => Some(6),
            _ => None,
        };
        if declared.is_some() {
            return declared;
        }
    }
    // Decision #29: the reader's measurement of type, last. A run under this engine's own `/Div`
    // reaches here too — its role path names no heading — which is what keeps an engine-tagged
    // document's projections equal to its untagged original's (docs/23 §4.3).
    text_run_attributes(node)
        .is_some_and(|a| a.inferred_heading)
        .then_some(1)
}

/// An ODF block's level: the one its own `<text:h>` stated, where that is a depth these two
/// formats have somewhere to put it.
///
/// **Two ways to be `None`, and neither is a guess.** A `<text:p>` is not a heading, which the
/// element name settles. And a `<text:h>` that stated no `text:outline-level` is a heading whose
/// depth this record does not know: ODF lets the attribute be omitted and resolves the level
/// through an outline style in `styles.xml`, a part the reader declares it did not open, so `#`
/// here would be level one on no evidence at all. It projects as a paragraph instead — the honest
/// output for an exporter that has the fact of a heading and not its depth.
///
/// **A level past six also projects as a paragraph.** ODF names no ceiling and the reader clamps
/// nothing, so `text:outline-level="300"` arrives here intact; Markdown has six `#` depths and
/// HTML six `<h>` elements, and emitting `#######` or `<h300>` would be this exporter inventing a
/// depth neither format has. Dropping to a paragraph loses the depth and states nothing false,
/// which is the trade every other clause here makes.
fn odf_heading_level(block: crate::OdfBlockKind, level: Option<u32>) -> Option<u8> {
    if !matches!(block, crate::OdfBlockKind::Heading) {
        return None;
    }
    match level {
        Some(l @ 1..=6) => u8::try_from(l).ok(),
        _ => None,
    }
}

/// The two facts an ODF block states about itself, from whichever attribute variant carries it.
///
/// One accessor rather than two `if let` arms, because `heading_level` and
/// [`odf_heading_erasure`] must agree about what an ODF block is or the census would count a
/// flattening the projection did not commit. `None` for every other node, which is what keeps the
/// PDF and EPUB paths out of both.
fn odf_block(node: &crate::Node) -> Option<(crate::OdfBlockKind, Option<u32>)> {
    match &node.attributes {
        crate::NodeAttributes::OfficeParagraph(a) => Some((a.block, a.outline_level)),
        crate::NodeAttributes::OfficeOdfShape(a) => Some((a.block, a.outline_level)),
        _ => None,
    }
}

/// The erasure a block commits by projecting as a paragraph the document called a heading, or
/// `None` where nothing was flattened (v2.4).
///
/// **The complement of [`odf_heading_level`], and the reason it is a second function rather than a
/// wider return type on `heading_level`:** that signature is `Option<u8>` and three callers read
/// it as a boolean — `hyphen_tail` twice, to refuse welding a word across a heading. Widening it
/// would put a census decision inside a predicate about hyphens.
///
/// Both projections call this at block open, each into its own `erasures` map, because `to_html`
/// builds its own and the two are not the same set. The *predicate* is here once so the two counts
/// are equal by construction.
///
/// Three ways to return `None`, and only the first is a non-event: the block is not a heading; the
/// block is a heading that projected at its own depth; the node is not ODF at all.
pub(crate) fn odf_heading_erasure(node: &crate::Node) -> Option<&'static str> {
    let (block, level) = odf_block(node)?;
    if !matches!(block, crate::OdfBlockKind::Heading) || odf_heading_level(block, level).is_some() {
        return None;
    }
    Some(match level {
        None => HEADING_LEVEL_UNRESOLVED,
        Some(_) => HEADING_LEVEL_UNREPRESENTABLE,
    })
}

/// `h1` … `h6` to a level, and `None` for every other XHTML element name.
///
/// Exact match on the six names HTML defines, and exact is right rather than lenient: XHTML is XML,
/// which is case-sensitive, and the EPUB reader's own `BLOCK_ELEMENTS` test is a byte comparison
/// against lowercase — so an element that reaches the wire as a block was spelled `h1` in the file.
/// **Not a prefix test**: `hgroup` starts with `h` and is not a heading, and an element this does
/// not recognise projects as a paragraph rather than as a guess.
fn xhtml_heading_level(element: &str) -> Option<u8> {
    match element {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
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
/// inventing a list the document did not draw. **Still refused, where headings no longer are**:
/// decision #29 lets the reader infer a heading from measured type, declared on the artifact,
/// and nothing comparable measures a list — a bullet glyph is a character, not a type size, and
/// reading structure off it would be the inference with neither the measurement nor the
/// declaration.
///
/// **No bullet glyph, no hanging indent, no font name.** An untagged document grows no list here;
/// its bullet characters are runs like any other and project as the text they are.
pub(crate) fn list_role(node: &crate::Node) -> Option<ListRole> {
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
pub(crate) struct ListRole {
    /// Nesting depth, zero-based: one `/L` deep is `0`.
    pub(crate) depth: usize,
    /// Whether this run is the item's `/Lbl` — the marker the document itself drew.
    pub(crate) label: bool,
}

/// The next run, when this one ends in a line-break hyphen and that run finishes the word.
///
/// Returns the run to consume and the joined text: the head's emitted text without its trailing
/// `-`, followed by the tail's.
///
/// # The rule, and why each clause is in it
///
/// The head's **emitted** text — after [`normalize`], the same bytes the map would have covered —
/// ends in an ASCII `-` with a letter in front of it; the very next node in reading order is a
/// plain paragraph run on the same page **and on a different baseline**; and its emitted text
/// starts with a letter.
///
/// - **A different baseline**, so only a hyphen at the end of a *line* is a candidate. See
///   [`on_different_lines`] — without this the rule welds two fragments of one line together and
///   deletes a compound hyphen the author wrote, which is measured and not hypothetical.
/// - **A letter in front of the hyphen**, so a run ending `foo -` is left alone. A dash standing
///   as its own word is punctuation the page drew, and closing it up would delete a character the
///   document meant.
/// - **The immediately next node**, so an annotation, a form field or a run of another table
///   between the halves stops the join. A rule that reached past intervening nodes would be
///   reordering the document to make a word.
/// - **The same page**, by parent page record.
/// - **Neither half is a heading, a list item or a cell.** Joining across a block boundary would
///   pull source text through syntax this exporter wrote — and by the time this is asked, a
///   heading's `#` marker is already on the string. The head is known to be neither a cell run nor
///   a list item because the projection has already routed those elsewhere; the tail is checked
///   here because nothing else has looked at it.
/// - **Both halves are page furniture, or neither is.** See [`is_page_artifact`]: a running head
///   is the one block boundary no other clause here can see, and welding one onto body text
///   fabricates a word and destroys the addressability rule 1 promises.
///
/// **Pairwise, once.** Three runs breaking one word twice join the first pair and leave the second
/// hyphen where it is. A bound rather than an oversight: the census still balances, the segment
/// still names exactly the runs it came from, and no fixture or corpus document does it.
///
/// # This rejoins nothing in the representation
///
/// `docs/history/10-V11-SCOPE.md` §5: a cosmetic is export-only. `extract` still emits `hyphen-` and
/// `ated` as two `Extracted` runs with the hyphen verbatim, and
/// `hyphenated_line_breaks_are_not_rejoined_and_that_is_the_policy` is the test that keeps it that
/// way. **A quote of the joined word therefore does not ground** — no element contains it — which
/// is the right answer for a word the page drew in two pieces, not a defect in the verifier.
pub(crate) fn hyphen_tail<'a>(
    head: &crate::Node,
    head_text: &str,
    next: Option<&'a crate::Node>,
    owner: &std::collections::BTreeMap<&str, usize>,
) -> Option<(&'a crate::Node, String)> {
    let stem = head_text.strip_suffix('-')?;
    if !stem.chars().next_back().is_some_and(char::is_alphabetic) || heading_level(head).is_some() {
        return None;
    }

    let next = next?;
    if next.parent != head.parent
        || dropped_code(next.kind).is_some()
        || owner.contains_key(next.id.as_str())
        || heading_level(next).is_some()
        || list_role(next).is_some()
        || is_page_artifact(head) != is_page_artifact(next)
        || region_of(head) != region_of(next)
        || !on_different_lines(head, next)
    {
        return None;
    }

    let tail = normalize(&next.text);
    if !tail.starts_with(char::is_alphabetic) {
        return None;
    }
    Some((next, format!("{stem}{tail}")))
}

/// Whether the content stream marked this run as page furniture rather than flow content.
///
/// **Neither [`heading_level`] nor [`list_role`] can see this**, because both bail unless the
/// locator is [`crate::StructuralLocator::PdfTagged`] — so without an explicit test a running head
/// or a footer passes every other clause of [`hyphen_tail`] and joins onto the body run before it.
///
/// # Why that is a defect and not a curiosity
///
/// A page whose last body line ends in a soft hyphen and whose footer is the next node in reading
/// order projects `Rates may be recalcu-` + `Confidential draft` as **`recalcuConfidential`** — a
/// word that appears nowhere on the page, welded out of two streams the document itself declared
/// separate (PDF 32000 §14.8.2.2, and `ethos-parser-pdf`'s own binding rule: *artifact wins*).
///
/// It also breaks a promise [`to_markdown`] rule 1 makes out loud. Artifacts are kept in the
/// projection precisely so *a consumer that wants them gone drops them itself, knowing it did* —
/// and the per-run `source` segment is the only handle a consumer has for that. One segment
/// spanning body text and a running head takes the handle away.
///
/// **Equality rather than exclusion**, because a two-line running head may hyphenate exactly like
/// a paragraph. What may not happen is a join *across* the boundary.
pub(crate) fn is_page_artifact(node: &crate::Node) -> bool {
    matches!(
        node.structural_locator,
        Some(crate::StructuralLocator::PdfArtifact(_))
    )
}

/// The reading-order region a run landed in, or `None` for a node that has none (D4-S3).
///
/// # Why the join needs it
///
/// The same reason it needs [`is_page_artifact`], one boundary over. The bottom of one column and
/// the top of the next are **adjacent in reading order, on the same page, on different
/// baselines**, and neither is a heading, a list item, a cell or page furniture — so every other
/// clause of [`hyphen_tail`] passes and a trailing hyphen welds the two columns together.
/// `a_hyphen_is_not_joined_across_a_column_boundary` is that defect, measured: `recalcu-` at the
/// foot of the left column and `Confidential` at the head of the right produced
/// **`recalcuConfidential`**, a word the page draws nowhere.
///
/// # Where, never what
///
/// This reads the region as a **boundary** and nothing else. It does not ask which region, does
/// not order them, and derives no heading, paragraph or column name from one — comparing two runs
/// for inequality is the whole use. A region is where text sits; what text *is* comes from the
/// structure tree or from nowhere (`docs/16-D4-SCOPE.md` §3, checklist P14).
///
/// `None` for every non-run node and for every run on a page the cut did not divide, so two such
/// nodes compare equal and the join behaves exactly as it did before D4 — which is what
/// `two_runs_in_one_region_still_join` and the untouched hyphen tests assert.
pub(crate) fn region_of(node: &crate::Node) -> Option<u32> {
    match &node.attributes {
        crate::NodeAttributes::TextRun(a) => a.region,
        _ => None,
    }
}

/// The marked-content group a run belongs to, **when the document declares one** (v2.2-S1).
///
/// A PDF marks its content: `BDC` opens a sequence and every glyph inside it carries that
/// sequence's id. Two runs sharing one are two fragments of one thing *the producer said was one
/// thing* — so joining them claims nothing this engine inferred. That is the whole licence for
/// this rule, and it is why the key is built from the declaration rather than from geometry.
///
/// **Absence is never a group.** A run with no structural locator, or a page artifact whose `mcid`
/// is absent — which `PdfArtifactLocator` documents as the usual case, and which is *every* one of
/// `nist-sp-800-207`'s 4 826 artifact runs — returns `None` and therefore never joins with
/// anything. Reading absence as a group would have welded that document's vertical margin stamp,
/// its running head and its folio into one block per page:
/// `This publication is available free of charge from:https://doi.org/…NISTSP800-207ZEROTRUST…`,
/// across 156 pt of white space. That is `recalcuConfidential` (D4-S3) again, 59 times per
/// document, and it is what an earlier draft of this rule did.
///
/// `region` is in the key **read for inequality only** — a block never spans a column boundary,
/// which is the same clause `hyphen_tail` needed at D4-S3. It says where a run is not, never what
/// it is, so decision 19's line is untouched.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupKey {
    page: u32,
    region: Option<u32>,
    /// Which locator kind declared the id, so a tagged `4` and an artifact `4` are never one group.
    kind: u8,
    mcid: i64,
}

/// The [`GroupKey`] of a run, or `None` where no producer declared one.
///
/// **A sequence this engine wrote is no declaration** (auto-tagging S3,
/// `docs/23-AUTO-TAGGING-SCOPE.md` §4.3). The declared join's licence is the producer's own
/// statement that these runs are one thing; on an engine-tagged document the producer of the
/// sequence is this engine, and the licence does not transfer. A `PdfTagged` locator whose
/// `derivation` is `Computed` — the writer's `/Div` read back — therefore returns `None`, and
/// the undeclared path applies exactly as it does on the untagged original. Reading the written
/// sequence as a declaration would reach, through the file, the weld across a baseline that
/// v2.2-S5 refused to make on geometry alone, and the anchor map — node ids and no derivation —
/// could not say so. Author structure is untouched: an `Extracted` locator keys exactly as it
/// has since v2.2-S1.
pub(crate) fn group_key(node: &crate::Node) -> Option<GroupKey> {
    let crate::NativeLocator::Pdf(loc) = &node.native_locator else {
        return None;
    };
    let (kind, mcid) = match node.structural_locator.as_ref()? {
        // No projection rule id moves for this arm: no input that existed before it projects
        // differently — every locator a reader minted before auto-tagging is `Extracted` — and
        // the new input class projects as its untagged twin, block for block and byte for byte
        // apart from the assurance block (scope §4.3, pinned by
        // `crates/ethos-parser-cli/tests/tag_roundtrip.rs`).
        crate::StructuralLocator::PdfTagged(t)
            if t.derivation == crate::DerivationClass::Computed =>
        {
            return None;
        }
        crate::StructuralLocator::PdfTagged(t) => (0u8, t.mcid),
        crate::StructuralLocator::PdfMcid(m) => (1u8, *m),
        crate::StructuralLocator::PdfArtifact(a) => (2u8, a.mcid?),
    };
    Some(GroupKey {
        page: loc.page,
        region: region_of(node),
        kind,
        mcid,
    })
}

/// The text-run attributes a run carries, or `None` for any other node kind.
pub(crate) fn text_run_attributes(node: &crate::Node) -> Option<&crate::TextRunAttributes> {
    match &node.attributes {
        crate::NodeAttributes::TextRun(a) => Some(a),
        _ => None,
    }
}

/// The line a run was drawn on — **where, never what** (v2.2-S5).
///
/// Four values, all read for equality and none for order, which is the same posture `region`
/// already takes at [`region_of`]: it says two runs are not on one line, never what either line
/// is. Nothing downstream may read a role from it, and nothing here names one — decision #19's
/// line, and `docs/06-STEAL-REFUSE.md` P14.
///
/// **`baseline` is why this cannot touch decision #21.** `origin_y` is compared for equality, so
/// a key can never span two baselines, so the fallback below can neither merge two lines into a
/// flow nor split one. `docs/19-BLOCK-SUBDIVISION-SCOPE.md` measures *vertical* gaps between
/// baselines and is parked; this is *horizontal*, within one baseline, and the two rules cannot
/// reach each other's evidence.
///
/// **`artifact` separates two streams the document itself declared separate.** Page furniture and
/// flow content share a page and sometimes a baseline; welding one onto the other is disaster
/// D4-S3 in its horizontal form.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct LineKey {
    page: u32,
    region: Option<u32>,
    artifact: bool,
    baseline: i64,
}

pub(crate) fn line_key(node: &crate::Node) -> Option<LineKey> {
    let crate::NativeLocator::Pdf(loc) = &node.native_locator else {
        return None;
    };
    Some(LineKey {
        page: loc.page,
        region: region_of(node),
        artifact: is_page_artifact(node),
        baseline: loc.origin_y,
    })
}

/// How wide one glyph is in this document, per `(font resource, size)`, as the document draws it.
///
/// Keyed exactly as the document keys its own fonts, and measured on the document being parsed —
/// so it is not a constant in this source at all. A key with fewer than two runs is **absent**:
/// one observation cannot corroborate itself, and a run whose font appears nowhere else is
/// refused rather than trusted.
pub(crate) type PitchReference = std::collections::HashMap<(String, i64), FontMeasure>;

/// What one font at one size measures on this page, taken from the page's own drawing.
///
/// Two numbers, both medians over the document rather than constants: `pitch`, the width of one
/// glyph, which bounds how far a run's ink can reach; and `space`, the width of the space this
/// font actually draws, where it draws one at all. Neither is a threshold anybody chose.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct FontMeasure {
    pub(crate) pitch: i64,
    pub(crate) space: Option<i64>,
}

pub(crate) fn pitch_reference(nodes: &[crate::Node]) -> PitchReference {
    let mut per_font: std::collections::HashMap<(String, i64), Vec<i64>> =
        std::collections::HashMap::new();
    // **The space the page draws, kept apart from the glyphs it draws beside it.** A run whose
    // text is nothing but whitespace and whose characters the reader did not insert is the
    // document stating, in its own metrics, how wide a word gap is in this font at this size.
    // Nothing here decides what that width should be.
    let mut per_font_space: std::collections::HashMap<(String, i64), Vec<i64>> =
        std::collections::HashMap::new();
    for node in nodes {
        let (Some(a), crate::NativeLocator::Pdf(loc)) =
            (text_run_attributes(node), &node.native_locator)
        else {
            continue;
        };
        let (Some(advance), glyphs) = (loc.advance, i64::try_from(a.char_codes.len()).unwrap_or(0))
        else {
            continue;
        };
        if advance <= 0 || glyphs == 0 {
            continue;
        }
        let key = (a.font_id.clone(), a.font_size);
        if a.synthesized.is_empty() && !node.text.is_empty() && node.text.trim().is_empty() {
            per_font_space
                .entry(key.clone())
                .or_default()
                .push(advance / glyphs);
            continue;
        }
        per_font.entry(key).or_default().push(advance / glyphs);
    }
    let median = |v: &mut Vec<i64>| {
        v.sort_unstable();
        v[v.len() / 2]
    };
    per_font
        .into_iter()
        .filter_map(|(k, mut v)| {
            (v.len() >= 2).then(|| {
                let pitch = median(&mut v);
                let space = per_font_space.get(&k).cloned().map(|mut s| median(&mut s));
                (k, FontMeasure { pitch, space })
            })
        })
        .collect()
}

/// Where `a`'s ink ends, and the width of one of its glyphs — both taken from the document's own
/// measure of the font rather than from `a`'s advance alone.
///
/// **`advance` is not an ink width, and this is the whole reason v2.2-S5 needs a second opinion.**
/// A table cell is commonly drawn as one run whose advance is the CELL PITCH, so
/// `origin_x + advance` lands on the *next cell* and a gap test reads ~0 across 120 pt of white
/// space. Measured on `docstructbench_llm-raw-scihub-o.O-ceat.200600410`: `AC` is drawn at
/// `origin_x` 31 181 with an advance of 12 053 — 6 026 per glyph, where the same font's median is
/// ~330, eighteen times over — and the next cell's `AA` begins at 43 229. A rule reading `advance`
/// alone joins them and emits `ACAA`, a token the page draws nowhere.
///
/// Capping the reach is what refuses that, and the bound it buys is **provable rather than
/// measured**: acceptance needs
/// `next.origin_x <= prev.origin_x + (glyphs + 1) × reference + INK_EPSILON_CENTIPOINTS`, so reach
/// per glyph can never exceed one reference glyph plus `(reference + 12) / glyphs`, however badly
/// `advance` lies.
///
/// **The slack is one glyph on each side, and it was one-sided until this was measured.** The cap
/// read `glyphs × reference`, which allowed a run to overlap the next by a whole glyph and yet
/// refused it a single centipoint of reach beyond the estimate. `reference` is a **median**, so
/// half of all runs exceed `glyphs × reference` by construction — and the epsilon is 12
/// centipoints, so a fraction of one percent is enough to break a join that is really abutting.
/// Measured on `docstructbench_llm-raw-scihub-o.O-chem.200700133.pdf_6`: `coordi` advances 2 509
/// over 6 glyphs against a font median of 415, so the cap truncated its reach by 21 centipoints
/// and turned a true 8-centipoint gap into a computed 29 — `coordi` and `nation` came out as two
/// blocks, and the whole page projected one word per block.
///
/// The multiplier is still 1 everywhere it appears — one glyph's width per glyph, one glyph of
/// tolerance above, one glyph of permitted overlap below. The `AC` cell-pitch run above is
/// refused by a factor of eighteen and is nowhere near the widened bound.
pub(crate) fn ink_reach(node: &crate::Node, pitch: &PitchReference) -> Option<(i64, FontMeasure)> {
    let crate::NativeLocator::Pdf(loc) = &node.native_locator else {
        return None;
    };
    let advance = loc.advance?;
    if advance < 0 {
        return None;
    }
    let a = text_run_attributes(node)?;
    let glyphs = i64::try_from(a.char_codes.len()).ok()?;
    if glyphs == 0 {
        return None;
    }
    let measure = *pitch.get(&(a.font_id.clone(), a.font_size))?;
    Some((
        loc.origin_x + advance.min((glyphs + 1) * measure.pitch),
        measure,
    ))
}

/// Whether `a` ends with a space **this reader inserted**, rather than one the page drew.
///
/// A PDF may open a word gap by moving the text cursor instead of drawing a space glyph. The
/// reader recognises that and writes the space into the run's own `text`, flagging it in
/// [`crate::TextRunAttributes::synthesized`] so nothing downstream mistakes it for a character the
/// document contains.
///
/// **This asks the flag rather than the reason string.** `SynthesizedAt::reason` is "in the
/// format's own vocabulary" — `tj-gap` is the PDF reader's word — and core has no business
/// knowing it. That a space was inserted at all is the format-neutral fact, and it is the one
/// this needs.
fn reader_inserted_trailing_space(a: &crate::Node) -> bool {
    if !a.text.ends_with(char::is_whitespace) {
        return false;
    }
    let Some(attrs) = text_run_attributes(a) else {
        return false;
    };
    let Some(last) = a.text.chars().count().checked_sub(1) else {
        return false;
    };
    attrs
        .synthesized
        .iter()
        .any(|s| usize::try_from(s.char_index).is_ok_and(|i| i == last))
}

/// Whether `b` is the next ink after `a` on one line — drawn **after** it, not over it, no gap.
///
/// Two-sided where [`ink_contiguous`] is one-sided, because a declaration is no longer supplying
/// the order. `a_end` and `a_reference` come from [`ink_reach`]; `a` may be a whitespace-only run
/// the projection skipped, which is how contiguity carries across a space the page drew.
pub(crate) fn ink_sequenced(
    a: &crate::Node,
    a_end: i64,
    a_measure: FontMeasure,
    b: &crate::Node,
) -> bool {
    let (crate::NativeLocator::Pdf(x), crate::NativeLocator::Pdf(y)) =
        (&a.native_locator, &b.native_locator)
    else {
        return false;
    };
    // 1. Drawn after, not over. `dx == 0` is exact overprint and `dx < 0` a backward jump; both
    //    are refused, because neither is "the next ink along this line".
    y.origin_x > x.origin_x
        // 2. No gap the page drew — **or** a gap this reader already read as a space and wrote
        //    into `a`'s own text. The second clause is the whole of v2.2-S7.
        //
        //    Without it an untagged page whose word gaps are cursor moves rather than space
        //    glyphs emits ONE WORD PER BLOCK: the reader inserts the space, and the block rule
        //    then measures the same gap against a 12-centipoint quantization epsilon and calls it
        //    a break. 253 of 735 OmniDocBench documents were more than half sub-3-character
        //    blocks, holding 53% of all their text.
        //
        //    **It joins on the reader's own prior finding, not on a new threshold.** A gap
        //    epsilon sized to word spaces was measured and refused at 0.47.0 — *"Latin has a
        //    trough to site it in and CJK has none"* — and re-measured here over 749 409
        //    same-baseline pairs: the Latin distribution decays monotonically from its
        //    word-space mode with no empty band anywhere, so any ceiling would be a tuned knob.
        //    There is none. What bounds this instead is that the space is **already in `a`'s
        //    text either way**, so the join moves a block boundary and changes no byte of text.
        //
        //    The declared path above has always joined on exactly this test — `drew_space`, with
        //    no gap check at all — because an mcid group is the document's own statement that two
        //    runs belong together. This is the same clause reaching the undeclared path, where
        //    the reader's insertion is the only statement available.
        //    **Third clause, `-v9`: a gap narrower than the space this font draws is not a word
        //    gap, and the page is what says so.** A page that sets its type with tracking draws
        //    each glyph as its own run and leaves a few centipoints between the boxes — 22 on
        //    `01030000000103`, against a 12-centipoint quantization epsilon sized for rounding —
        //    so every letter became its own block and the document projected one character per
        //    paragraph. 54 of the 200 opendataloader-bench documents averaged under 20 characters
        //    a block, and their mean NID was 0.81 against 0.90 for the rest
        //    (`docs/measurements/liteparse-head-to-head/README.md` §6).
        //
        //    **It is not the gap epsilon 0.47.0 refused**, and the difference is where the number
        //    comes from. That epsilon would have been a constant chosen to sit in a trough that
        //    Latin has and CJK does not. This is the median advance of the runs *this document*
        //    draws whose text is nothing but whitespace, in *this* font at *this* size: the page's
        //    own statement of what a word gap measures. A font that draws no space at all supplies
        //    no measure and joins nothing new — the clause simply does not apply, and every
        //    document whose word gaps are cursor moves keeps the behaviour clause two gives it.
        //
        //    Strictly narrower, so a gap the width of the drawn space is still a word gap; and
        //    the space glyph itself is a run on the same baseline, so it joins, and its character
        //    reaches the text the way the page drew it.
        && (y.origin_x - a_end <= INK_EPSILON_CENTIPOINTS
            || reader_inserted_trailing_space(a)
            || a_measure
                .space
                .is_some_and(|space| y.origin_x - a_end < space))
        // 3. Overlapping by at most one of `a`'s own glyphs. Negative tracking is ordinary — CJK
        //    medians sit near -42 centipoints — but an overlap deeper than a glyph means the two
        //    runs are stacked rather than sequenced.
        && y.origin_x - a_end >= -a_measure.pitch
}

/// The gap, in centipoints, at or below which the page drew no space between two runs.
///
/// A quantization epsilon, not a tuned threshold. Measured on the gate corpus, the gap between
/// same-baseline pairs in one group is 0 for 64 120 of 64 143 pairs on `nist-sp-800-207` and the
/// histogram between 13 and 150 centipoints is **empty** — raw gaps cluster at -1, +1 and +2
/// centipoints, which is per-glyph coordinate rounding. Any epsilon in that valley gives the same
/// answer; an epsilon of 0 breaks 28 785 boundaries, which is the measurement proving a tolerance
/// is needed and that its value is not a knob.
///
/// **What that justification does NOT cover, stated because v2.2-S5 now leans on this number
/// outside the population it was measured on.** The valley was measured on *declared* pairs of
/// *one Latin document*. On undeclared pairs of the OmniDocBench corpus — roughly four fifths of
/// which set CJK — the 13-150 centipoint band is not empty: CJK draws no word space, so its gap
/// distribution decays monotonically and has no valley to site an epsilon in. The number is kept
/// because it is the one already in the tree and because raising it would be a per-script knob,
/// but a CJK reader is being told less by `ink_contiguous` than a Latin one.
pub(crate) const INK_EPSILON_CENTIPOINTS: i64 = 12;

/// Whether two runs in one group are **ink-contiguous** — no gap the page drew between them.
///
/// One-sided: it bounds how far apart two runs may be and says nothing about overlap or about
/// which was drawn first. Inside a marked-content group the producer's declaration supplies both,
/// which is why v2.2-S1 needed no more. [`ink_sequenced`] is the two-sided form the undeclared
/// fallback needs.
pub(crate) fn ink_contiguous(a: &crate::Node, b: &crate::Node) -> bool {
    let (crate::NativeLocator::Pdf(x), crate::NativeLocator::Pdf(y)) =
        (&a.native_locator, &b.native_locator)
    else {
        return false;
    };
    let Some(advance) = x.advance else {
        return false;
    };
    y.origin_x - (x.origin_x + advance) <= INK_EPSILON_CENTIPOINTS
}

/// Whether two runs sit on different baselines — the test for "broken across a line".
///
/// # Measured, not assumed
///
/// Without this clause the rule joins any run ending in `-` to the run after it, **including two
/// fragments of one line**. That is not hypothetical: `cfpb-home-loan-toolkit` page 24 draws
/// `non-escrowed` as a string of tiny runs at one baseline — `non-`, `escr`, `o`, `w` … — and the
/// rule fired on `non-` + `escr` and produced **`nonescr`**, deleting a compound hyphen the
/// document meant and yielding a word that is not one. It was the only place the rule fired on the
/// whole benchmark corpus, and it fired wrongly.
///
/// A hyphen inside a line is a hyphen the author wrote. Only a hyphen at a line's end is a
/// candidate for having been inserted by the line break, which is the whole premise of checklist
/// **P15** and of the `hyphenated-line-break` fixture.
///
/// **Conservative on purpose.** Runs without a glyph-run locator, or on a page whose lines do not
/// separate in `origin_y`, simply do not join. A missed join reads as two words, which is what the
/// page drew; a wrong join invents one. Given `docs/01-CONTRACT.md`'s posture on fabrication, those
/// are not comparable costs.
pub(crate) fn on_different_lines(a: &crate::Node, b: &crate::Node) -> bool {
    match (&a.native_locator, &b.native_locator) {
        (crate::NativeLocator::Pdf(x), crate::NativeLocator::Pdf(y)) => x.origin_y != y.origin_y,
        _ => false,
    }
}

/// Hyphens removed closing up a word the document broke across a line (v1.1-S3).
///
/// **A character bucket, not a [`StructuralErasure`]**, and that is the whole test for which
/// census a disclosure belongs in: a hyphen *is* a character of node text, and it really is not in
/// the Markdown. The GFM erasures are counted separately because a merged cell's text is emitted
/// in full and a `tables-flattened` character bucket would read `0`.
///
/// `chars` is the number of hyphens dropped; `nodes` is the number of runs that lost one, which is
/// the same number — a run is the head of at most one join.
pub(crate) const HYPHENATION_REJOIN_DROPPED: &str = "hyphenation-rejoin-dropped-v1";

/// The bucket a non-projected node belongs to.
///
/// One per node kind rather than one catch-all, because each is a different fact about the
/// document and collapsing them would put "we dropped 40 characters" where "a reviewer's note and
/// a form value are different things" belongs.
pub(crate) fn dropped_code(kind: NodeKind) -> Option<&'static str> {
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

/// One geometric block: the nodes it holds, in order, and the text they join to (v2.2-S7).
///
/// **The text is verbatim concatenation and nothing else.** A space the page drew is a run of its
/// own, with its own characters, so joining member texts reproduces exactly what the document
/// drew and invents no separator. That is why this carries no space rule where the Markdown
/// projection needs one: Markdown normalizes whitespace and must render a drawn space as its own
/// single byte, and this does not render anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometricBlock {
    /// Indices into the node slice this was built from, in order.
    pub members: Vec<usize>,
    /// The members' own text, concatenated.
    pub text: String,
}

/// Group runs into blocks by the rule both projections already use (v2.2-S7).
///
/// **This is `where`, never `what`** — decision #19. A block is a set of runs the page drew as one
/// piece of ink; it is not a paragraph, a heading, a section or a column, and nothing downstream
/// may read a role from one.
///
/// The clauses are the ones [`to_markdown`] joins on, called rather than restated so the two
/// cannot drift: a declared marked-content group ([`group_key`]), or, where the document declared
/// nothing, the next ink along one baseline ([`line_key`] + [`ink_sequenced`], with the reach
/// capped by [`ink_reach`]). What this does NOT carry is the projection-specific breaks — list
/// items, hyphen rejoins and heading markers are renderings, and a grounding consumer wants the
/// ink.
///
/// `table_owned` names the node ids a table already claims. A cell's runs are grounded through
/// the table, so merging them into a prose block would put one run in two places.
pub fn geometric_blocks(
    nodes: &[crate::Node],
    table_owned: &std::collections::BTreeSet<&str>,
) -> Vec<GeometricBlock> {
    let pitch = pitch_reference(nodes);
    let mut out: Vec<GeometricBlock> = Vec::new();
    let mut open_group: Option<GroupKey> = None;
    let mut open_line: Option<LineKey> = None;
    let mut line_ink: Option<(&crate::Node, i64, FontMeasure)> = None;
    let mut open_prev: Option<&crate::Node> = None;

    for (i, node) in nodes.iter().enumerate() {
        // A node of another kind, or one a table already claims, is its own block and closes
        // whatever was open — the same two resets the emitter makes.
        let standalone =
            node.kind != crate::NodeKind::TextRun || table_owned.contains(node.id.as_str());

        let key = group_key(node);
        let joins = !standalone
            && match (open_group, key, open_prev) {
                (Some(open), Some(k), Some(_)) if open == k => true,
                (None, None, Some(_)) => match (open_line, line_ink) {
                    (Some(line), Some((ink, end, reference))) => {
                        Some(line) == line_key(node) && ink_sequenced(ink, end, reference, node)
                    }
                    _ => false,
                },
                _ => false,
            };

        match out.last_mut() {
            Some(block) if joins => {
                block.members.push(i);
                block.text.push_str(&node.text);
            }
            _ => out.push(GeometricBlock {
                members: vec![i],
                text: node.text.clone(),
            }),
        }

        if standalone {
            open_group = None;
            open_prev = None;
            open_line = None;
            line_ink = None;
        } else {
            open_group = key;
            open_prev = Some(node);
            open_line = line_key(node);
            line_ink = ink_reach(node, &pitch).map(|(x, r)| (node, x, r));
            if line_ink.is_none() {
                open_line = None;
            }
        }
    }
    out
}

// -------------------------------------------------------------------------------------------
// The emitter
// -------------------------------------------------------------------------------------------

/// The string being built and the map being built beside it, in lockstep.
///
/// Every byte of the Markdown goes through [`Emit::syntax`] or [`Emit::source`], which is what
/// makes law 2 hold **by construction** rather than by a final audit: there is no `push_str` in
/// the projection that does not also record what it pushed.
pub(crate) struct Emit {
    pub(crate) markdown: String,
    pub(crate) segments: Vec<Segment>,
    /// Characters that landed in a `source` segment, which is `coverage.source_chars_emitted`.
    pub(crate) emitted_chars: usize,
}

impl Emit {
    pub(crate) fn new() -> Self {
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
    pub(crate) fn syntax(&mut self, s: &str) {
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
    pub(crate) fn source(&mut self, s: &str, node: &str) {
        self.source_encoded(s, s.chars().count(), node);
    }

    /// Append to the previous `source` segment, naming this node alongside the ones already there.
    ///
    /// **The exception `Self::source`'s "never coalesced" states the rule for** (v2.2-S1). That
    /// rule refuses to merge two segments whose contiguity exists only in this string. Here the
    /// contiguity is the document's: both runs sit in one marked-content sequence and no gap was
    /// drawn between them, so the bytes really are adjacent on the page. `joined_source` already
    /// emits one segment naming two nodes for the hyphen join, on the same argument; this is that
    /// mechanism at block scale.
    ///
    /// A segment naming several nodes **reads and does not ground** — a consumer sees
    /// `node_ids.len() > 1` and knows the quote spans runs. That is the identical signal the
    /// hyphen join has carried since v1.1-S3.
    pub(crate) fn source_continuing(&mut self, s: &str, node: &str) {
        if s.is_empty() {
            return;
        }
        let start = self.markdown.len();
        let chars = s.chars().count();
        match self.segments.last_mut() {
            Some(last) if last.kind == SegmentKind::Source && last.end == start => {
                self.markdown.push_str(s);
                last.end = self.markdown.len();
                if last.node_ids.iter().all(|id| id != node) {
                    last.node_ids.push(node.to_string());
                }
            }
            _ => {
                self.markdown.push_str(s);
                self.segments.push(Segment {
                    kind: SegmentKind::Source,
                    start,
                    end: self.markdown.len(),
                    node_ids: vec![node.to_string()],
                });
            }
        }
        self.emitted_chars += chars;
    }

    /// Bytes that stand for `chars` characters of one node's text.
    ///
    /// # Why the count is a parameter (v1.1-S4)
    ///
    /// For Markdown the two are always the same: a `source` segment's bytes *are* the normalized
    /// text. HTML has to encode three characters as entities — `<` becomes `&lt;` — and those
    /// four bytes still stand for exactly **one** character the document drew.
    ///
    /// Law 4 counts **characters of node text**, not emitted bytes, so letting an entity inflate
    /// the count would break the census in the direction that hides things: `emitted` would exceed
    /// what the representation holds and the whitespace residue would underflow. Passing the
    /// unescaped count keeps `emitted + dropped == in_representation` exactly true, and keeps it
    /// true for the same reason on both artifacts.
    pub(crate) fn source_encoded(&mut self, s: &str, chars: usize, node: &str) {
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
        self.emitted_chars += chars;
    }

    /// The two halves of a word the document broke across a line, written as one word (v1.1-S3).
    ///
    /// # One segment naming two nodes
    ///
    /// [`Segment::node_ids`] has always been a list; this is the first emitter to put more than
    /// one id in it. It has to be one segment: the hyphen that marked where the halves met is no
    /// longer in the string, so there is no byte offset at which the first run stops being the
    /// answer and the second starts. Splitting it into two adjacent segments would put that
    /// boundary somewhere and claim a precision the join threw away.
    ///
    /// A consumer inverting these bytes gets both runs, which is what the page has.
    pub(crate) fn joined_source(&mut self, s: &str, first: &str, second: &str) {
        self.joined_source_encoded(s, s.chars().count(), first, second);
    }

    /// [`Self::joined_source`], with the character count given rather than measured.
    ///
    /// The HTML projection needs it for the reason [`Self::source_encoded`] exists: its bytes are
    /// entity-encoded, so they may stand for fewer characters than they occupy.
    pub(crate) fn joined_source_encoded(
        &mut self,
        s: &str,
        chars: usize,
        first: &str,
        second: &str,
    ) {
        if s.is_empty() {
            return;
        }
        let start = self.markdown.len();
        self.markdown.push_str(s);
        self.segments.push(Segment {
            kind: SegmentKind::Source,
            start,
            end: self.markdown.len(),
            node_ids: vec![first.to_string(), second.to_string()],
        });
        self.emitted_chars += chars;
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
/// # The rule, in full — `markdown-blocks-v11`
///
/// 1. **Text runs only.** Every other node kind is dropped into its own named bucket. **Page
///    artifacts are NOT dropped**: a running head is a `text_run` carrying
///    `structural_locator: pdf_artifact`, and checklist O21/O22 is explicit that a reader deleting
///    running heads has silently edited the document. The flag stays in the representation and a
///    consumer that wants them gone drops them itself, knowing it did.
/// 2. **A heading when the tree says so**, `#` through `######` — or `#` where the reader read a
///    line as a heading from its type (decision #29); a paragraph otherwise.
/// 3. **A GFM table** for every table on the representation, at the position of the first run one
///    of its cells claims. A run emitted inside a table is **not** also emitted as a paragraph —
///    the characters move from linear source to cell source, they are not duplicated and they are
///    not dropped.
/// 4. **A list item** for a run the structure tree places in an `/L`; nothing for a bullet
///    glyph, a hanging indent or a font name.
/// 5. **Blank line between blocks**, which is `syntax` — the document drew no such bytes.
/// 6. **Node text is normalized** by [`normalize`], and a node whose normalization is empty
///    contributes no segment at all rather than an empty one.
/// 7. **A word broken across a line is closed up** — v1.1-S3's one cosmetic, and the only clause
///    here that removes a character that is not whitespace (rule 6 collapses runs of it, and
///    counts them too). The conditions are [`hyphen_tail`]'s, the joined bytes are one `source`
///    segment naming both runs, and the hyphen is counted in `HYPHENATION_REJOIN_DROPPED`.
///    **The representation is untouched**: `extract` still holds both halves with the hyphen
///    verbatim, so the joined word is readable and *not* citable, and the map says which two
///    strings are.
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
    // Set when the previous node's hyphen join already emitted this one's text, as the tail of a
    // joined word. Its characters still count toward `in_representation` — they are in the record
    // whether or not this pass reaches them separately — so the flag is read after that.
    let mut joined_tail = false;

    // v2.2-S1. The marked-content group currently open, the run that last emitted into it, and
    // whether a whitespace-only run was skipped since — `pending_space`, which exists because the
    // empty-text `continue` below drops those runs before any join state could see them. That
    // omission is how an earlier draft of this rule produced `backupwithholding` on `irs-fw9`:
    // the space between the two words IS a run, of its own, and it is skipped.
    let mut open_group: Option<GroupKey> = None;
    let mut open_prev: Option<&crate::Node> = None;
    let mut pending_space = false;

    // v2.2-S5. The undeclared fallback. `open_line` is the line the open block was drawn on and
    // `line_ink` is how far along it the ink has reached — the run that last extended it, the x
    // that run's ink ends at, and that font's reference glyph width. One pre-pass measures the
    // document's own fonts, because `advance` alone cannot be trusted to be an ink width; see
    // `ink_reach`.
    let pitch = pitch_reference(&payload.nodes);
    let mut open_line: Option<LineKey> = None;
    let mut line_ink: Option<(&crate::Node, i64, FontMeasure)> = None;

    for (i, node) in payload.nodes.iter().enumerate() {
        in_representation += node.text.chars().count();

        if joined_tail {
            joined_tail = false;
            continue;
        }

        if let Some(code) = dropped_code(node.kind) {
            let b = buckets.entry(code).or_insert((0, 0));
            b.0 += node.text.chars().count();
            b.1 += 1;
            // v2.2-S1. A node of another kind between two runs ends the block: whatever the
            // producer marked, this projection put something else between them.
            open_group = None;
            open_prev = None;
            pending_space = false;
            open_line = None;
            line_ink = None;
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
            // v2.2-S1. A table's runs are consumed by `emit_table`, so the block around them is
            // over whether or not this run opened it. Not resetting here is how a naive join
            // destroys every GFM table it touches — measured at TEDS 0.104 -> 0.000 when an
            // earlier draft of this rule was tried outside the projection.
            open_group = None;
            open_prev = None;
            pending_space = false;
            open_line = None;
            line_ink = None;
            continue;
        }

        let text = normalize(&node.text);
        if text.is_empty() {
            // Whitespace-only runs exist (a `Tj` of spaces is a real operator). They contribute no
            // Markdown, and they contribute no dropped characters either: `normalize` removed
            // whitespace, and whitespace is not content this projection claims to have lost.
            //
            // v2.2-S1: they DO contribute the knowledge that the page drew a space here, which the
            // join rule below needs and which dropping them silently destroyed. The group is not
            // closed — a space inside a marked-content sequence does not end it.
            if !node.text.is_empty() {
                pending_space = true;
                // v2.2-S5. A drawn space is ink on the line even though it projects to nothing,
                // so the reach moves past it rather than the line being abandoned. Without this
                // the fallback breaks at every space the page drew as its own run.
                match (open_line, line_ink) {
                    (Some(line), Some((ink, end, reference)))
                        if Some(line) == line_key(node)
                            && ink_sequenced(ink, end, reference, node) =>
                    {
                        line_ink = ink_reach(node, &pitch).map(|(e, r)| (node, e, r));
                        if line_ink.is_none() {
                            open_line = None;
                        }
                    }
                    _ => {
                        open_line = None;
                        line_ink = None;
                    }
                }
            }
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
            // The list path has its own join and its own census code; a prose block never
            // continues into or out of one.
            open_group = None;
            open_prev = None;
            pending_space = false;
            open_line = None;
            line_ink = None;
            continue;
        }

        open_item = None;

        // v2.2-S1. **Join into the open block, or start a new one.** Before this, every run became
        // its own block: `nist-sp-800-207` projected as 68 112 blocks averaging two characters, and
        // "NIST Special Publication 800-207" arrived as forty of them.
        //
        // Four clauses, in order, and **the default is to break**. That posture is
        // `on_different_lines`' own, quoted because it decides this rule too: *"A missed join reads
        // as two words, which is what the page drew; a wrong join invents one."* A rule that joined
        // by default and looked for a reason to insert a space would fabricate a word wherever the
        // page drew its space by positioning rather than by a glyph — 467 word boundaries on
        // `nist-sp-800-171r3` alone, and the `SynthesisReason::TjGap` that might have covered them
        // fires **zero** times on four of the five gate documents.
        // v2.2-S5. Computed here rather than below so it keeps **first refusal** over the
        // undeclared join: a run that is the head of a word the page broke across a line opens a
        // block, so `hyphen_tail` can close the word up. Joining it into the block above instead
        // would strand the tail and re-open `nonescr` from the other side.
        let ht = hyphen_tail(node, &text, payload.nodes.get(i + 1), &owner);
        let key = group_key(node);
        let joining = match (open_group, key, open_prev) {
            (Some(open), Some(k), Some(prev)) if open == k => {
                // 1. The page drew a space — in either run's own bytes, or as a run of its own.
                let drew_space = pending_space
                    || prev.text.ends_with(char::is_whitespace)
                    || node.text.starts_with(char::is_whitespace);
                // 2. A line break inside one marked-content sequence. Rendering it as a space
                //    invents no word; welding across it would. `hyphen_tail` has already had first
                //    refusal, so a word the line break split is closed up instead. Same outcome as
                //    clause 1 and a different reason, which is why they are named separately here
                //    rather than written as two arms.
                let broke_a_line = on_different_lines(prev, node);
                if drew_space || broke_a_line {
                    Some(true)
                } else if ink_contiguous(prev, node) {
                    // 3. Same baseline, no gap the page drew: two fragments of one word.
                    Some(false)
                } else {
                    // 4. A visible gap with no whitespace anywhere, or no advance to judge with.
                    None
                }
            }
            // v2.2-S5. **Neither run carries a declaration**, so geometry is the whole licence
            // and every clause a declaration licensed is gone. `LineKey` equality holds the join
            // to one page, one region, one stream and one baseline; `ink_sequenced` holds it to
            // ink the page drew next along that line, with the reach capped by the document's own
            // measure of the font. Where v2.2-S1 could join across a line break because the
            // producer said the two runs were one sequence, here nothing said so — and welding
            // across a baseline with no declaration is exactly the disaster `group_key`'s
            // "absence is never a group" was written against.
            (None, None, Some(_)) => match (open_line, line_ink) {
                (Some(line), Some((ink, end, reference)))
                    if ht.is_none()
                        && Some(line) == line_key(node)
                        && ink_sequenced(ink, end, reference, node) =>
                {
                    Some(
                        pending_space
                            || ink.text.ends_with(char::is_whitespace)
                            || node.text.starts_with(char::is_whitespace),
                    )
                }
                _ => None,
            },
            _ => None,
        };

        match joining {
            Some(space) => {
                let declared = key.is_some();
                if space {
                    // `syntax`, not source: the document drew a space somewhere, and this is this
                    // exporter's single normalized rendering of it. `html.rs`'s cell join settled
                    // the same question the same way.
                    e.syntax(" ");
                    e.source(&text, node.id.as_str());
                } else if declared {
                    e.source_continuing(&text, node.id.as_str());
                } else {
                    // v2.2-S5. `source`, never `source_continuing`. That method's licence is that
                    // "the contiguity is the document's"; under a geometric join it is this
                    // engine's, so the exception does not apply and the seam stays addressable —
                    // two abutting `source` segments naming different nodes.
                    e.source(&text, node.id.as_str());
                }
                let code = if declared {
                    MCID_RUN_JOINS
                } else if space {
                    BASELINE_RUN_JOINS_SPACED
                } else {
                    BASELINE_RUN_JOINS_ABUTTED
                };
                *erasures.entry(code).or_insert(0) += 1;
                pending_space = false;
                open_prev = Some(node);
                line_ink = ink_reach(node, &pitch).map(|(x, r)| (node, x, r));
                if line_ink.is_none() {
                    open_line = None;
                }
                continue;
            }
            None => {
                separate(&mut e, &mut last, Block::Standalone);
                open_group = key;
                open_prev = Some(node);
                pending_space = false;
                open_line = line_key(node);
                line_ink = ink_reach(node, &pitch).map(|(x, r)| (node, x, r));
                if line_ink.is_none() {
                    open_line = None;
                }
            }
        }

        // The heading marker, when the tree said so. Also `syntax`: `## ` is this exporter's
        // rendering of a role, not bytes the page drew. Emitted at block open only — a run joined
        // into an open block never re-emits it, which is why the `continue` above is above this.
        if let Some(level) = heading_level(node) {
            e.syntax(&"#".repeat(level as usize));
            e.syntax(" ");
        } else if let Some(code) = odf_heading_erasure(node) {
            // A block the document called a heading, coming out as body text. Counted here rather
            // than beside the predicate, for the reason the marker is emitted here: this is block
            // open, and a run joined into an open block must not count a second time.
            *erasures.entry(code).or_insert(0) += 1;
        }

        // A word the page broke across a line, closed up here and nowhere else. The two halves
        // become one `source` segment naming both runs, no separator goes between them, and the
        // hyphen that is no longer in the string is counted — see `HYPHENATION_REJOIN_DROPPED`.
        if let Some((tail, joined)) = ht {
            e.joined_source(&joined, node.id.as_str(), tail.id.as_str());
            let b = buckets.entry(HYPHENATION_REJOIN_DROPPED).or_insert((0, 0));
            b.0 += 1;
            b.1 += 1;
            // v2.2-S5. The block now holds text from two baselines, so the reach measured for
            // this run no longer describes where its ink ends. Nothing may join onto it by
            // geometry.
            open_line = None;
            line_ink = None;
            joined_tail = true;
            continue;
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

    let coverage = census(payload, in_representation, emitted_chars, buckets, erasures);

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

/// Close the character census, for **either** projection (v1.1-S4).
///
/// # Why this is one function and not two
///
/// `ethos.html.v1` accounts for exactly the same characters as `ethos.markdown.v1` — the same
/// nodes are dropped for the same reasons, the same whitespace is collapsed by the same rule, and
/// the same hyphen is removed by the same predicate. Two copies of this arithmetic would be two
/// places for law 4 to stop holding, and only one of them would have a failing test.
///
/// The whitespace bucket is computed here rather than accumulated by the caller because it is a
/// *residue*: `emitted` counts NORMALIZED characters and normalization can only shrink a string,
/// so measuring `in_representation` over the same normalization would balance the census by moving
/// the goal posts — a run drawn as `Hello   world` would report 11 characters in a representation
/// that holds 13, and the two missing ones would be invisible. The denominator is the RAW text of
/// every node, and the difference gets its own named class.
///
/// `erasures` is **not** the same set on both artifacts, and the caller decides which it passes.
/// The test is whether that projection commits the erasure: HTML drops `gfm-row-zero-separator-v1`
/// (it asserts no header) and recomputes `gfm-span-slots-unrepresentable-v1` from what its grid
/// could not hold, while keeping every code that describes a fault in the **record** rather than a
/// limit of GFM. See `crate::html`'s module documentation for the table. Copying a code onto an
/// artifact that does not commit the erasure would disclose nothing; dropping one it does commit
/// would be worse.
pub(crate) fn census(
    payload: &crate::representation::RepresentationPayload,
    in_representation: usize,
    emitted_chars: usize,
    mut buckets: std::collections::BTreeMap<&'static str, (usize, usize)>,
    erasures: std::collections::BTreeMap<&'static str, usize>,
) -> Coverage {
    // **Saturating, not wrapping, and the difference matters.** This is a residue: whatever the
    // representation offered that neither reached a `source` segment nor landed in a named bucket.
    // A plain `usize` subtraction underflows if the two projections ever double-count a character
    // — panicking in debug, and in release producing a bucket of 18 446 744 073 709 551 578 that
    // `Coverage::balances()` accepts, because both sides wrap by the same amount. That is the one
    // failure mode law 4 cannot afford: a census that is wrong AND self-consistent.
    //
    // Reachable today only from a hand-authored record — a table cell may name any declared node,
    // including an annotation, and such a node is charged to its dropped bucket AND emitted inside
    // the table. `ethos-parser extract` never writes one, but `ethos-parser markdown` and `ethos-parser html` accept
    // any correctly-fingerprinted representation, so the arithmetic fails closed instead.
    let accounted = emitted_chars + buckets.values().map(|(chars, _)| *chars).sum::<usize>();
    let collapsed: usize = in_representation.saturating_sub(accounted);
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

    Coverage {
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
    }
}

// -------------------------------------------------------------------------------------------
// Tables
// -------------------------------------------------------------------------------------------

/// How one table will be laid out as GFM, decided before a byte is written.
pub(crate) struct TablePlan<'a> {
    /// Whether this table becomes a grid at all.
    pub(crate) projected: bool,
    /// The declared grid, row-major, `rows × columns` entries. Each is the ordered list of runs
    /// whose text belongs in that slot — empty for a slot a merge covers, for a hole, and for an
    /// origin cell that enclosed no run.
    pub(crate) slots: Vec<Vec<&'a crate::Node>>,
    /// What each slot **is**, row-major and the same length as [`Self::slots`] (v1.1-S4).
    ///
    /// GFM never needed this: it expands every merge, so a covered slot and an empty one both come
    /// out as an empty cell and telling them apart changes nothing. **HTML does need it**, because
    /// `<td rowspan>` carries the merge — the origin cell has to know its own span, and the slots
    /// it covers must produce no `<td>` at all rather than an empty one.
    pub(crate) roles: Vec<SlotRole>,
    /// Slots a declared merge asked for and this grid could not give it (v1.1-S4).
    ///
    /// Zero on every well-formed table. Non-zero when a span ran off the grid edge or collided
    /// with another cell's origin — see the resolution pass in [`plan_tables`]. A projection that
    /// carries merges declares this as [`GFM_SPAN_SLOTS_UNREPRESENTABLE`], because a merge the
    /// record asked for and the output does not show is exactly what that code counts.
    pub(crate) spans_clamped: usize,
    pub(crate) rows: usize,
    pub(crate) columns: usize,
}

/// What one slot of a planned grid is (v1.1-S4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotRole {
    /// No cell originates or lands here: a hole in the grid the document drew.
    Empty,
    /// A cell starts here, covering `rowspan × colspan` slots. `1 × 1` is the ordinary case.
    Origin { rowspan: usize, colspan: usize },
    /// A merge from an earlier origin reaches this slot. **HTML emits nothing for it** — that is
    /// what `rowspan`/`colspan` on the origin already said, and a second `<td>` would put the
    /// merged cell's own width back into the row.
    Covered,
}

/// Decide the layout of every table, and count what GFM cannot say about them.
///
/// Runs are claimed here rather than during emission so that **claiming and emitting cannot
/// disagree**: the linear pass skips exactly the runs some slot will print, because it is reading
/// the same list this built.
pub(crate) fn plan_tables<'a>(
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
                roles: Vec::new(),
                spans_clamped: 0,
                rows,
                columns,
            });
            continue;
        }

        let mut slots: Vec<Vec<&crate::Node>> = vec![Vec::new(); rows * columns];
        let mut placed = vec![false; rows * columns];
        let mut roles = vec![SlotRole::Empty; rows * columns];
        // Origins with the span they DECLARED, resolved after every cell is placed — a span
        // cannot be clamped against origins that have not been seen yet.
        let mut declared: Vec<(usize, usize, usize, usize, usize)> = Vec::new();

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
            let (rowspan, colspan) = (
                (cell.position.rowspan as usize).max(1),
                (cell.position.colspan as usize).max(1),
            );
            let covered =
                (cell.position.rowspan as usize).saturating_mul(cell.position.colspan as usize);
            if covered > 1 {
                *erasures.entry(GFM_SPAN_SLOTS_UNREPRESENTABLE).or_insert(0) += covered - 1;
            }

            // The declared span, kept for a second pass. **`placed` is left alone deliberately**:
            // marking covered slots there would change which later cells count as
            // `gfm-cell-not-placed-v1`, and S2's numbers are not this slice's to move.
            declared.push((index, r, c, rowspan, colspan));

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

        // **Resolve the spans, now that every origin is known.**
        //
        // A declared span is clamped to the slots it can actually reach: it stops at the grid
        // edge, and it stops at a slot another cell originates in. Both matter, and the second one
        // is not hypothetical — `fixtures/engine/ruled-table-overlap` declares a 2x2 whose row 1
        // holds BOTH a `colspan: 2` cell at (1,0) and an ordinary cell at (1,1). Without the
        // clamp the covered slot is promoted back to an origin by the later cell and the row emits
        // three cells wide in a two-column table.
        //
        // GFM never had to care: it expands every merge, so a covered slot and an empty one come
        // out identically and the arithmetic is the same either way. A projection that USES the
        // span has to resolve the collision, and the clamp is the only answer that neither drops a
        // cell nor widens the row. What the clamp costs is a merge the record asked for and this
        // grid cannot hold — which is what `GFM_SPAN_SLOTS_UNREPRESENTABLE` already means, so
        // `TablePlan::spans_clamped` carries it to whichever projection wants to declare it.
        let mut spans_clamped = 0usize;
        for &(index, r, c, rowspan, colspan) in &declared {
            // Clamped to the grid first, so a cell declaring `rowspan: 4_000_000_000` costs a
            // bounded walk rather than one proportional to a number the document chose.
            let max_rows = rows - r;
            let max_columns = columns - c;
            let mut fit_rows = rowspan.min(max_rows);
            let mut fit_columns = colspan.min(max_columns);

            // Then shrunk until it collides with no other origin.
            while fit_columns > 1
                && (0..fit_rows).any(|dr| placed[(r + dr) * columns + c + fit_columns - 1])
            {
                fit_columns -= 1;
            }
            while fit_rows > 1
                && (0..fit_columns).any(|dc| placed[(r + fit_rows - 1) * columns + c + dc])
            {
                fit_rows -= 1;
            }

            spans_clamped += rowspan.saturating_mul(colspan) - fit_rows * fit_columns;
            roles[index] = SlotRole::Origin {
                rowspan: fit_rows,
                colspan: fit_columns,
            };
            for dr in 0..fit_rows {
                for dc in 0..fit_columns {
                    if dr != 0 || dc != 0 {
                        roles[(r + dr) * columns + c + dc] = SlotRole::Covered;
                    }
                }
            }
        }

        // GFM's delimiter row makes row 0 a header on every renderer there is, and nothing in the
        // representation says the document declared one. Once per table — see the constant.
        *erasures.entry(GFM_ROW_ZERO_SEPARATOR).or_insert(0) += 1;

        plans.push(TablePlan {
            projected: true,
            slots,
            roles,
            spans_clamped,
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
pub(crate) mod tests {
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
                    name: "ethos-parser".into(),
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
                StructuralLocator::PdfTagged(
                    PdfTaggedLocator {
                        mcid: 0,
                        role_path: vec!["Document".into(), r.into()],
                        standard_role_path: None,
                        element_id: None,
                        derivation: DerivationClass::Extracted,
                    }
                    .into(),
                )
            }),
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::TextRun(TextRunAttributes {
                char_codes: text.bytes().map(u32::from).collect(),
                scalar_code_mismatch: false,
                synthesized: Vec::new(),
                findings: Vec::new(),
                font_id: "F1".into(),
                font_size: 2400,
                region: None,
                block: None,
                inferred_heading: false,
            }),
        }
    }

    pub(crate) fn repr_of(specs: &[(&str, Option<&str>)]) -> DocumentRepresentation {
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

    /// `repr` with the reader's heading flag set on the runs at `indices` — what the reader
    /// produces for a line it read as a heading from its type (decision #29) — resealed, since a
    /// sealed record is not edited in place.
    pub(crate) fn with_inferred_headings(
        repr: DocumentRepresentation,
        indices: &[usize],
    ) -> DocumentRepresentation {
        let mut payload = repr.payload().clone();
        for &i in indices {
            if let NodeAttributes::TextRun(a) = &mut payload.nodes[i].attributes {
                a.inferred_heading = true;
            }
        }
        DocumentRepresentation::seal(payload, repr.geometry().to_vec()).unwrap()
    }

    pub(crate) fn simple_repr() -> DocumentRepresentation {
        repr_of(&[("Hello Ethos", None)])
    }

    /// The same as [`repr_of`], but each run gets **its own baseline** — one run per line.
    ///
    /// `text_node` puts every node at `origin_y: 7200`, which is one line, and that is the right
    /// default for every other test here. The hyphenation join is the one rule that reads the
    /// baseline ([`on_different_lines`]), so it needs a representation whose lines actually
    /// differ, and a test that used `repr_of` would be asserting against a page whose runs the
    /// document drew side by side.
    pub(crate) fn repr_of_lines(specs: &[&str]) -> DocumentRepresentation {
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
            .map(|(i, t)| {
                let mut n = text_node(&mut alloc, &page.id, i as u32 + 1, t, None);
                n.native_locator = NativeLocator::Pdf(PdfLocator {
                    page: 1,
                    origin_x: 7200,
                    origin_y: 7200 + (i as i64) * 2400,
                    advance: Some(1000),
                });
                n
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

    /// Re-seal a representation with node `index` marked as **page furniture**.
    ///
    /// `text_node` can only build a `PdfTagged` locator, and the artifact case is exactly the one
    /// no tagged role can express: an artifact is content the page declared to be *outside* the
    /// structure tree, which is why no `heading_level` or `list_role` clause can see it.
    fn mark_artifact(repr: &DocumentRepresentation, index: usize) -> DocumentRepresentation {
        let mut nodes = repr.payload().nodes.clone();
        nodes[index].structural_locator = Some(StructuralLocator::PdfArtifact(
            crate::representation::PdfArtifactLocator { mcid: None },
        ));
        let geometry = nodes
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
            })
            .collect();
        let pages = repr.payload().pages.clone();
        DocumentRepresentation::seal(payload(nodes, pages), geometry).unwrap()
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
        n.structural_locator = Some(StructuralLocator::PdfTagged(
            PdfTaggedLocator {
                mcid: ordinal as i64,
                role_path: path.iter().map(|s| (*s).to_string()).collect(),
                standard_role_path: None,
                element_id: None,
                derivation: DerivationClass::Extracted,
            }
            .into(),
        ));
        n
    }

    /// A representation whose nodes carry explicit role paths. `None` is an untagged run.
    pub(crate) fn repr_of_paths(specs: &[(&str, Option<&[&str]>)]) -> DocumentRepresentation {
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
    pub(crate) struct Cell {
        pub(crate) row: u32,
        pub(crate) column: u32,
        pub(crate) rowspan: u32,
        pub(crate) colspan: u32,
        /// Indices into the `texts` given to [`repr_with_table`].
        pub(crate) runs: Vec<usize>,
    }

    pub(crate) fn cell(row: u32, column: u32, runs: &[usize]) -> Cell {
        Cell {
            row,
            column,
            rowspan: 1,
            colspan: 1,
            runs: runs.to_vec(),
        }
    }

    pub(crate) fn spanning(
        row: u32,
        column: u32,
        rowspan: u32,
        colspan: u32,
        runs: &[usize],
    ) -> Cell {
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
    pub(crate) fn repr_with_table(
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
                geometry: crate::GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
                text: c.runs.iter().map(|i| texts[*i]).collect(),
                node_ids: c.runs.iter().map(|i| nodes[*i].id.clone()).collect(),
            })
            .collect();
        let table = crate::TableRecord {
            id: table_id,
            page: page.id.clone(),
            geometry: crate::GeometryPresence::Measured(QRect::new(0, 0, 100, 100).unwrap()),
            rows,
            columns,
            cells: records,
            derivation: DerivationClass::Computed,
            detection_rule: crate::TABLE_DETECTION_V3.to_string(),
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

    // ---------------------------------------------------------------------------------------
    // v1.1-S3 — the hyphenation join, and the line test that keeps it honest
    // ---------------------------------------------------------------------------------------

    /// A hyphen at the end of a **line** is closed up, and the hyphen is counted.
    #[test]
    fn a_hyphen_at_a_line_end_is_joined_and_the_hyphen_is_counted() {
        let a = artifact_of(repr_of_lines(&["hyphen-", "ated"]));
        assert_eq!(a.markdown, "hyphenated\n");

        // One segment over the joined letters, naming both runs — there is no offset inside them
        // at which the first run stops being the answer, because the hyphen that marked it is gone.
        assert_eq!(a.anchor_map.segments.len(), 2);
        assert_eq!(a.anchor_map.segments[0].kind, SegmentKind::Source);
        assert_eq!(a.anchor_map.segments[0].node_ids.len(), 2);

        assert!(a.coverage.balances());
        assert_eq!(a.coverage.source_chars_emitted, 10);
        assert_eq!(a.coverage.source_chars_in_representation, 11);
        let b = a
            .coverage
            .dropped
            .iter()
            .find(|b| b.code == HYPHENATION_REJOIN_DROPPED)
            .expect("the removed hyphen is a named bucket");
        assert_eq!((b.chars, b.nodes), (1, 1));
    }

    /// A running head is **not** the second half of a body word.
    ///
    /// # The one block boundary no other clause can see
    ///
    /// `heading_level` and `list_role` both bail unless the locator is `PdfTagged`, so a
    /// `pdf_artifact` run passes every other tail guard. Before [`is_page_artifact`], a page whose
    /// last body line ended in a soft hyphen and whose footer came next in reading order projected
    /// `Rates may be recalcu-` + `Confidential draft` as **`recalcuConfidential`** — a word on no
    /// page, welded from two streams the document itself declared separate.
    ///
    /// It also broke `to_markdown` rule 1 out loud: artifacts are kept *so a consumer that wants
    /// them gone drops them itself, knowing it did*, and the per-run `source` segment is the only
    /// handle for that. One segment over body text and a footer removes it.
    /// Put two runs in different reading-order regions, keeping everything else joinable.
    fn in_regions(repr: &DocumentRepresentation, regions: &[u32]) -> DocumentRepresentation {
        let mut payload = repr.payload().clone();
        for (node, region) in payload.nodes.iter_mut().zip(regions) {
            if let NodeAttributes::TextRun(a) = &mut node.attributes {
                a.region = Some(*region);
            }
        }
        DocumentRepresentation::seal(payload, repr.geometry().to_vec()).unwrap()
    }

    /// **D4-S3: a column boundary is a block boundary this join could not see.**
    ///
    /// Exactly the defect [`is_page_artifact`] was added for, one boundary over. The bottom of
    /// column one and the top of column two are adjacent in reading order, on the same page, on
    /// different baselines, and neither is a heading, a list item, a cell or page furniture — so
    /// every other clause of [`hyphen_tail`] passes and the two weld together. `recalcu-` at the
    /// foot of the left column and `Confidential` at the head of the right column projected as
    /// **`recalcuConfidential`**, a word the page draws nowhere and that no citation can ground.
    ///
    /// The region says where the cut put each run, so the join can decline. **It is used as a
    /// boundary and never as a role** — nothing here reads a heading, a paragraph or a column
    /// name out of it, which is the line `docs/16-D4-SCOPE.md` §3 draws and P14 forbids crossing.
    #[test]
    fn a_hyphen_is_not_joined_across_a_column_boundary() {
        let repr = in_regions(
            &repr_of_lines(&["Rates may be recalcu-", "Confidential draft"]),
            &[1, 2],
        );
        let a = artifact_of(repr);
        assert!(
            !a.markdown.contains("recalcuConfidential"),
            "a word welded across a column gutter is on no page, got {:?}",
            a.markdown
        );
        assert_eq!(
            a.markdown, "Rates may be recalcu-\n\nConfidential draft\n",
            "each column keeps its own text, and the left column keeps the hyphen the page drew"
        );
        assert!(a.coverage.balances());
    }

    /// The guard is about the *boundary*, not about the field being present.
    ///
    /// Two runs in the SAME region join exactly as they always did — the clause added for the
    /// column boundary must not become a blanket refusal on any document the cut divided.
    #[test]
    fn two_runs_in_one_region_still_join() {
        let a = artifact_of(in_regions(&repr_of_lines(&["hyphen-", "ated"]), &[1, 1]));
        assert!(
            a.markdown.contains("hyphenated"),
            "a hyphen broken within one column still closes up, got {:?}",
            a.markdown
        );
        assert!(a.coverage.balances());
    }

    #[test]
    fn a_page_artifact_is_not_joined_onto_body_text() {
        let repr = repr_of_lines(&["Rates may be recalcu-", "Confidential draft"]);
        let a = artifact_of(mark_artifact(&repr, 1));
        assert_eq!(
            a.markdown, "Rates may be recalcu-\n\nConfidential draft\n",
            "the footer stays its own block, and the body keeps the hyphen the page drew"
        );
        assert!(
            !a.markdown.contains("recalcuConfidential"),
            "a word welded out of body text and page furniture is on no page"
        );
        // Rule 1's promise, mechanically: each run still has its own segment, so a consumer can
        // drop the running head without taking body text with it.
        for s in a
            .anchor_map
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Source)
        {
            assert_eq!(
                s.node_ids.len(),
                1,
                "no segment spans the furniture boundary"
            );
        }
        assert!(a.coverage.balances());
    }

    /// The mirrored case: an artifact head does not swallow the body run after it.
    #[test]
    fn body_text_is_not_joined_onto_a_page_artifact() {
        let repr = repr_of_lines(&["Chapter inter-", "national text"]);
        let a = artifact_of(mark_artifact(&repr, 0));
        assert_eq!(a.markdown, "Chapter inter-\n\nnational text\n");
        assert!(!a.markdown.contains("international"));
    }

    /// Two lines of one running head hyphenate like any paragraph — **equality, not exclusion**.
    #[test]
    fn a_running_head_broken_across_its_own_two_lines_still_joins() {
        let repr = mark_artifact(&repr_of_lines(&["Confiden-", "tial"]), 0);
        let a = artifact_of(mark_artifact(&repr, 1));
        assert_eq!(a.markdown, "Confidential\n");
        assert_eq!(a.anchor_map.segments[0].node_ids.len(), 2);
    }

    /// A hyphen **inside** a line is the author's, and it stays.
    ///
    /// # This is a measurement, not a hypothetical
    ///
    /// Without the baseline test the rule joined any run ending in `-` to the run after it, and
    /// `cfpb-home-loan-toolkit` page 24 is the case that found it: the document draws
    /// `non-escrowed` as a string of tiny runs at one baseline — `non-`, `escr`, `o`, `w` … — so
    /// the rule produced **`nonescr`**, deleting a compound hyphen the author wrote and yielding a
    /// word that is not one. It was the *only* place the rule fired on the whole benchmark corpus,
    /// and it fired wrongly.
    ///
    /// Two runs at the same `origin_y` were drawn side by side. A hyphen between them was never a
    /// line break, so there is nothing for this rule to repair.
    #[test]
    fn a_hyphen_inside_a_line_is_the_authors_and_is_not_joined() {
        // `repr_of` puts every run on one baseline, which is exactly the shape being pinned.
        let a = artifact_of(repr_of(&[("non-", None), ("escr", None)]));
        assert_eq!(
            a.markdown, "non-\n\nescr\n",
            "the hyphen the author wrote survives, and the runs stay two blocks"
        );
        assert!(
            !a.markdown.contains("nonescr"),
            "the join must not invent a word the page never drew"
        );
        assert!(
            a.coverage
                .dropped
                .iter()
                .all(|b| b.code != HYPHENATION_REJOIN_DROPPED),
            "nothing was joined, so the bucket is absent rather than reading 0"
        );
        assert!(a.coverage.balances());
    }

    /// A dash standing as its own word is punctuation, not half of one.
    #[test]
    fn a_dangling_dash_at_a_line_end_is_not_joined() {
        let a = artifact_of(repr_of_lines(&["foo -", "bar"]));
        assert_eq!(a.markdown, "foo -\n\nbar\n");
        assert!(a
            .coverage
            .dropped
            .iter()
            .all(|b| b.code != HYPHENATION_REJOIN_DROPPED));
    }

    /// A word broken twice joins its first pair and leaves the second hyphen alone.
    ///
    /// The stated bound of a pairwise rule, pinned so it is a decision rather than a surprise: the
    /// census still balances and the segment still names exactly the runs it came from.
    #[test]
    fn a_word_broken_twice_joins_once_and_says_so_by_leaving_the_second_hyphen() {
        let a = artifact_of(repr_of_lines(&["hy-", "phen-", "ated"]));
        assert_eq!(a.markdown, "hyphen-\n\nated\n");
        let b = a
            .coverage
            .dropped
            .iter()
            .find(|b| b.code == HYPHENATION_REJOIN_DROPPED)
            .expect("one join happened");
        assert_eq!(b.chars, 1, "one hyphen removed, not two");
        assert!(a.coverage.balances());
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

    /// One ODF block node, whose block kind and stated outline level are the facts under test.
    ///
    /// The sibling of [`epub_node`], built for the same reason it was: without a builder, every
    /// test of the ODF half of `heading_level` would have to go through a real package, and the
    /// two ODF packages in the corpus carry one level between them — `outline-level="1"` in the
    /// text document and a bare `<text:h>` in the presentation. Neither reaches `h2`..`h6`, and
    /// neither reaches a level past the six these formats can express.
    fn odf_node(
        alloc: &mut IdAllocator,
        parent: &NodeId,
        ordinal: u32,
        block: crate::OdfBlockKind,
        level: Option<u32>,
    ) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: parent.clone(),
            ordinal,
            text: match level {
                Some(l) => format!("Text at level {l}"),
                None => "Text with no level".to_string(),
            },
            native_locator: NativeLocator::Odt(crate::OdtLocator {
                part: "content.xml".into(),
                paragraph: ordinal,
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::OfficeParagraph(crate::OfficeParagraphAttributes {
                block,
                outline_level: level,
            }),
        }
    }

    /// A page-less representation of ODF blocks, one per `(kind, level)` pair.
    pub(crate) fn odf_repr_of(
        blocks: &[(crate::OdfBlockKind, Option<u32>)],
    ) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let root = alloc.next(IdKind::Part).unwrap();
        let nodes: Vec<Node> = blocks
            .iter()
            .enumerate()
            .map(|(i, (block, level))| odf_node(&mut alloc, &root, i as u32 + 1, *block, *level))
            .collect();
        let geometry = nodes
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: GeometryPresence::Absent(crate::GeometryAbsence::NotApplicableToKind),
            })
            .collect();
        let mut payload = payload(nodes, Vec::new());
        payload.source.media_type = "application/vnd.oasis.opendocument.text".into();
        payload.assurance = crate::assurance::Assurance::new(
            Capabilities::V0,
            0,
            Vec::new(),
            vec![Limitation::document(
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
                "every node here is an ODF block, and an ODF block has no ink box by \
                 construction — `content.xml` states the text and a layout this reader does not \
                 perform would state the boxes.",
            )],
        )
        .unwrap();
        DocumentRepresentation::seal(payload, geometry).unwrap()
    }

    /// One EPUB block node, whose XHTML element name is the fact under test.
    ///
    /// The sibling of [`text_node`], and its absence is why the XHTML half of `heading_level` went
    /// unguarded above level 1: with no builder for an `EpubBlock` node, every test of that
    /// function had to go through a real publication, and the only publication in the corpus
    /// carries nothing but `<h1>`.
    fn epub_node(alloc: &mut IdAllocator, parent: &NodeId, ordinal: u32, element: &str) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: parent.clone(),
            ordinal,
            text: format!("Text inside {element}"),
            native_locator: NativeLocator::Epub(crate::EpubLocator {
                part: "text/body.xhtml".into(),
                block: ordinal,
            }),
            structural_locator: None,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::EpubBlock(crate::EpubBlockAttributes {
                element: element.into(),
                linear: true,
            }),
        }
    }

    /// A page-less representation of EPUB blocks, one per element name.
    pub(crate) fn epub_repr_of(elements: &[&str]) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        // **A part, not a page.** `seal` refuses a page-less node parented by a page id, in as
        // many words: *"a page-less node is parented by the part it was read from, never by a
        // page this engine invented for it."* Reaching for `IdKind::Page` here — the reflex, from
        // every other builder in this module — produced exactly that refusal, which is the
        // invariant doing its job on a test that was about to build a document no reader emits.
        let root = alloc.next(IdKind::Part).unwrap();
        let nodes: Vec<Node> = elements
            .iter()
            .enumerate()
            .map(|(i, e)| epub_node(&mut alloc, &root, i as u32 + 1, e))
            .collect();
        // Page-less and box-less: an EPUB has no page and its blocks have no ink box, which is
        // the shape v2-S24 taught the seal to accept. Building it with geometry would be building
        // a document this reader never produces.
        let geometry = nodes
            .iter()
            .map(|n| NodeGeometry {
                node: n.id.clone(),
                presence: GeometryPresence::Absent(crate::GeometryAbsence::NotApplicableToKind),
            })
            .collect();
        // The seal requires the payload to DECLARE what the sidecar shows, because the sidecar
        // sits outside the fingerprint: a record whose geometry contradicts its own assurance
        // block is exactly what that check exists to refuse. Two invariants fired while this
        // helper was being written, and both were the seal working rather than in the way.
        let mut payload = payload(nodes, Vec::new());
        payload.source.media_type = "application/epub+zip".into();
        payload.assurance = crate::assurance::Assurance::new(
            Capabilities::V0,
            0,
            Vec::new(),
            vec![Limitation::document(
                crate::assurance::codes::GEOMETRY_ABSENT_NOT_GROUNDABLE,
                "every node here is an EPUB block, and an EPUB block has no ink box by \
                 construction — there is nothing to measure until something lays the publication \
                 out, and laying it out is invented pagination.",
            )],
        )
        .unwrap();
        DocumentRepresentation::seal(payload, geometry).unwrap()
    }

    /// **Every ODF level projects at its own depth, and every way of having no usable level
    /// projects as a paragraph.** The sibling of the XHTML test below, written at the same time as
    /// the arm it guards rather than after it — `heading_level`'s XHTML half shipped unguarded
    /// above level 1 and stayed that way for a release, invisible to a suite of 1380.
    #[test]
    fn every_odf_outline_level_projects_at_its_own_depth() {
        use crate::OdfBlockKind::Heading;
        let a = artifact_of(odf_repr_of(&[
            (Heading, Some(1)),
            (Heading, Some(2)),
            (Heading, Some(3)),
            (Heading, Some(4)),
            (Heading, Some(5)),
            (Heading, Some(6)),
        ]));
        assert_eq!(
            a.markdown,
            "# Text at level 1\n\n\
             ## Text at level 2\n\n\
             ### Text at level 3\n\n\
             #### Text at level 4\n\n\
             ##### Text at level 5\n\n\
             ###### Text at level 6\n"
        );
        assert!(a.coverage.balances());
        assert!(
            a.coverage.structural_erasures.is_empty(),
            "a heading that projected at its own depth flattened nothing: {:?}",
            a.coverage.structural_erasures
        );
    }

    /// The three ways an ODF block is not a heading this projection can write, each of which would
    /// be a false claim if it came out as `#`.
    #[test]
    fn an_odf_block_with_no_usable_level_projects_as_a_paragraph() {
        use crate::OdfBlockKind::{Heading, Paragraph};
        let a = artifact_of(odf_repr_of(&[
            // A `<text:h>` that stated no level. It is a heading, and its depth lives in
            // `styles.xml`, which the reader declares unread — so `#` would be level one on no
            // evidence.
            (Heading, None),
            // A level past the six `#` depths Markdown has. `#######` is not a heading.
            (Heading, Some(7)),
            (Heading, Some(300)),
            // A `<text:p>` is not a heading whatever attribute rides along on it.
            (Paragraph, Some(1)),
        ]));
        assert_eq!(
            a.markdown,
            "Text with no level\n\n\
             Text at level 7\n\n\
             Text at level 300\n\n\
             Text at level 1\n",
            "no `#` anywhere: each of these is a block this projection has the fact of and not a \
             depth it can honestly write"
        );
        assert!(a.coverage.balances());

        // **The silence ends here.** Before v2.4 all four of these flattened a declared heading
        // and the artifact said nothing at all; the `#` that was never written is not a dropped
        // character, so the character census cannot carry it and this second census must.
        assert_eq!(
            erasure(&a, HEADING_LEVEL_UNRESOLVED),
            1,
            "the `<text:h>` that stated no level"
        );
        assert_eq!(
            erasure(&a, HEADING_LEVEL_UNREPRESENTABLE),
            2,
            "levels 7 and 300 — one per block, and `0` would reach this arm too"
        );
        // And the paragraph is not a case: nothing was flattened, because a `<text:p>` was never
        // a heading. Two codes and two only, which is also the sort order a reader sees.
        let codes: Vec<&str> = a
            .coverage
            .structural_erasures
            .iter()
            .map(|e| e.code.as_str())
            .collect();
        assert_eq!(
            codes,
            vec![HEADING_LEVEL_UNREPRESENTABLE, HEADING_LEVEL_UNRESOLVED],
            "sorted by code, and the pair prints adjacent"
        );
    }

    /// **All six XHTML heading levels, and this is a table because a fixture cannot be one**
    /// (v2.2-S4).
    ///
    /// The PDF half of `heading_level` was guarded at every level from the day it shipped, by
    /// `a_heading_role_from_the_tree_projects_as_a_heading` — which is a hand-built
    /// representation for the reason it states: *"no fixture in either corpus carries a heading
    /// role"*. The XHTML half, added at v2.2-S0, got no such test. Its only coverage was one
    /// end-to-end assertion over `fixtures/office/book-spine/book.epub`, and that publication
    /// contains `<h1>` and nothing else.
    ///
    /// So five of the six arms were reached by nothing. **Measured rather than suspected**:
    /// replacing `"h2"`..`"h6"` with `None` and running the whole workspace failed **zero** of
    /// roughly 1 300 tests. Every `<h2>`–`<h6>` in every EPUB could have projected as a paragraph
    /// and the suite would have stayed green — 0.43.0's headline feature, silently half-delivered.
    ///
    /// A fixture cannot close this the way a table can. Covering six levels through a publication
    /// means six headings in a real EPUB, which is a fixture edit, a digest move and a golden
    /// move for a fact that has nothing to do with any of them. The end-to-end path is already
    /// proved at `h1` by `an_epubs_own_heading_element_projects_as_an_h_element`; what was missing
    /// is that the LEVEL follows the element, and that is a mapping, so it is tested as one.
    #[test]
    fn every_xhtml_heading_level_projects_at_its_own_depth() {
        let a = artifact_of(epub_repr_of(&["h1", "h2", "h3", "h4", "h5", "h6"]));
        assert_eq!(
            a.markdown,
            "# Text inside h1\n\n\
             ## Text inside h2\n\n\
             ### Text inside h3\n\n\
             #### Text inside h4\n\n\
             ##### Text inside h5\n\n\
             ###### Text inside h6\n",
            "each level projects at its own depth; before v2.2-S4 only the first was checked"
        );
        assert!(a.coverage.balances());
    }

    /// **The near misses, which are the other half of an exact match** (v2.2-S4).
    ///
    /// `xhtml_heading_level`'s doc comment says the match is exact and not a prefix test, and
    /// names `hgroup` as the reason. Nothing tested that either. XHTML is XML and therefore
    /// case-sensitive, so `H1` is a different element from `h1` and must not become a heading —
    /// and `h7` does not exist in any HTML specification.
    ///
    /// Each of these projects as a paragraph rather than as a guess, which is what the module
    /// header means by *"a heading is a heading because the document said so"*.
    #[test]
    fn an_element_that_merely_looks_like_a_heading_stays_a_paragraph() {
        for element in ["hgroup", "h7", "h0", "H1", "H2", "header", "hr", "h", "h11"] {
            let a = artifact_of(epub_repr_of(&[element]));
            assert_eq!(
                a.markdown,
                format!("Text inside {element}\n"),
                "`{element}` is not one of the six names HTML defines and must not project as a \
                 heading"
            );
            assert!(
                !a.markdown.starts_with('#'),
                "`{element}` produced a heading marker"
            );
        }
    }

    /// **A big font is not a heading where the document declares structure.** Rewritten from
    /// `a_big_font_is_not_a_heading` rather than deleted, because a deleted refusal test is a
    /// refusal nobody can see was reversed — and what decision #29 reversed is narrower than that
    /// name said (`docs/28-HEADINGS-SCOPE.md` §9).
    ///
    /// This projection consults no font size, and where the document declares structure the reader
    /// never sets its heading flag (decision #29's gate). So a 2400-centipoint run with no role and
    /// no flag is exactly what a big font on a tagged document reaches this function as, and it is
    /// a paragraph.
    #[test]
    fn a_big_font_is_not_a_heading_where_the_document_declares_structure() {
        let a = artifact_of(repr_of(&[("Chapter One", None)]));
        assert_eq!(a.markdown, "Chapter One\n");
        assert!(!a.markdown.contains('#'));
    }

    /// **...and it is one where the document declares none.** The same run, flagged by the reader,
    /// is a level-one heading — the flag, never the size, is what this reads — and the `# ` is
    /// syntax: the exporter's rendering of an inference, which no quote may include and invert.
    #[test]
    fn a_run_the_reader_read_as_a_heading_projects_as_one() {
        let a = artifact_of(with_inferred_headings(
            repr_of(&[("Chapter One", None), ("Body text", None)]),
            &[0],
        ));
        assert_eq!(a.markdown, "# Chapter One\n\nBody text\n");
        let hash = a.markdown.find("# Chapter").unwrap();
        assert!(!a.anchor_map.is_invertible(hash, hash + 11));
        let title = a.markdown.find("Chapter One").unwrap();
        assert!(a.anchor_map.is_invertible(title, title + 11));
        assert!(a.coverage.balances());
    }

    /// **A declared heading wins over the flag**, structurally: the reader never sets both, and
    /// were a hand-edited record to carry both, the document's own `/H2` is what projects.
    #[test]
    fn a_declared_level_wins_over_an_inferred_one() {
        let a = artifact_of(with_inferred_headings(
            repr_of(&[("Section", Some("H2"))]),
            &[0],
        ));
        assert_eq!(a.markdown, "## Section\n");
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

    /// **No list without a tree** — the refusal L29 made about font sizes, which survives here for
    /// lists after decision #29 narrowed it for headings.
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

    // --------------------------------------------------------------------------------------
    // v2.2-S1 — block assembly
    // --------------------------------------------------------------------------------------

    /// A run whose declaration and geometry the test controls.
    ///
    /// [`text_node`] hard-codes `mcid: 0`, one baseline and one `origin_x`, which is right for
    /// every rule that came before and cannot express any of the cases below.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn placed_run(
        alloc: &mut IdAllocator,
        parent: &crate::ids::NodeId,
        ordinal: u32,
        text: &str,
        loc: Option<StructuralLocator>,
        x: i64,
        y: i64,
        advance: Option<i64>,
        region: Option<u32>,
    ) -> Node {
        Node {
            id: alloc.next(IdKind::Span).unwrap(),
            kind: NodeKind::TextRun,
            parent: parent.clone(),
            ordinal,
            text: text.into(),
            native_locator: NativeLocator::Pdf(PdfLocator {
                page: 1,
                origin_x: x,
                origin_y: y,
                advance,
            }),
            structural_locator: loc,
            derivation: DerivationClass::Extracted,
            attributes: NodeAttributes::TextRun(TextRunAttributes {
                char_codes: text.bytes().map(u32::from).collect(),
                scalar_code_mismatch: false,
                synthesized: Vec::new(),
                findings: Vec::new(),
                font_id: "F1".into(),
                font_size: 2400,
                region,
                block: None,
                inferred_heading: false,
            }),
        }
    }

    /// An author's declaration: a `/P` element citing `mcid`, bound `Extracted`.
    pub(crate) fn tagged_at(mcid: i64) -> Option<StructuralLocator> {
        tagged_as(mcid, DerivationClass::Extracted)
    }

    /// A tagged locator of either class (auto-tagging S3). `Computed` is the writer's `/Div`
    /// read back; the role path is the one the writer emits, so the builder is the shape the
    /// projections meet rather than an author's path with a class swapped in.
    pub(crate) fn tagged_as(mcid: i64, derivation: DerivationClass) -> Option<StructuralLocator> {
        let role = match derivation {
            DerivationClass::Computed => "Div",
            _ => "P",
        };
        Some(StructuralLocator::PdfTagged(
            PdfTaggedLocator {
                mcid,
                role_path: vec!["Document".into(), role.into()],
                standard_role_path: None,
                element_id: None,
                derivation,
            }
            .into(),
        ))
    }

    type RunSpec<'a> = (
        &'a str,
        Option<StructuralLocator>,
        i64,
        Option<i64>,
        Option<u32>,
    );

    /// A run spec that also names the baseline it was drawn on (v2.2-S5).
    ///
    /// `RunSpec` pins every run at `origin_y = 7200`, which is why no test written before this
    /// slice could express a baseline difference — and why the fallback's refusal to cross one
    /// had no in-unit tripwire. `(text, locator, x, y, advance, region)`.
    pub(crate) type LineSpec<'a> = (
        &'a str,
        Option<StructuralLocator>,
        i64,
        i64,
        Option<i64>,
        Option<u32>,
    );

    pub(crate) fn project_runs(specs: &[RunSpec<'_>]) -> MarkdownArtifact {
        let lines: Vec<LineSpec<'_>> = specs
            .iter()
            .map(|(t, l, x, a, r)| (*t, l.clone(), *x, 7200, *a, *r))
            .collect();
        project_lines(&lines)
    }

    pub(crate) fn project_lines(specs: &[LineSpec<'_>]) -> MarkdownArtifact {
        artifact_of(repr_of_placed(specs))
    }

    /// The sealed representation [`project_lines`] projects — the runs, placed where the caller
    /// put them.
    ///
    /// Split out of `project_lines` for [`crate::locate`]'s tests, which need the representation
    /// and not the Markdown. A second builder beside this one would be a second answer to where
    /// the runs are, and the block rule reads nothing else.
    pub(crate) fn repr_of_placed(specs: &[LineSpec<'_>]) -> DocumentRepresentation {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 61200,
            height: 79200,
            rotation: 0,
        };
        let nodes: Vec<Node> = specs
            .iter()
            .enumerate()
            .map(|(i, (t, l, x, y, a, r))| {
                placed_run(
                    &mut alloc,
                    &page.id,
                    i as u32 + 1,
                    t,
                    l.clone(),
                    *x,
                    *y,
                    *a,
                    *r,
                )
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

    fn blocks_of(a: &MarkdownArtifact) -> Vec<String> {
        a.markdown
            .split("\n\n")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// **A whitespace-only run between two joined runs is the space the page drew.**
    ///
    /// Kills the mutant that reads only the two joined runs' own bytes. `markdown.rs`'s empty-text
    /// `continue` drops whitespace-only runs before any join state sees them, and on `irs-fw9`
    /// that turns `...subject to backup` + ` ` + `withholding` into **`backupwithholding`** — 3 292
    /// word-boundary welds on `nist-sp-800-207` alone.
    #[test]
    fn a_skipped_whitespace_run_is_the_space_the_page_drew() {
        let a = project_runs(&[
            ("backup", tagged_at(7), 7200, Some(1000), None),
            (" ", tagged_at(7), 8200, Some(200), None),
            ("withholding", tagged_at(7), 8400, Some(2000), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["backup withholding"]);
    }

    /// **Ink-contiguous runs join with no separator and become ONE source segment.**
    ///
    /// Two fragments of one word. The segment names both runs, which is the signal a consumer
    /// reads to know the quote spans them — `node_ids.len() > 1`, exactly as the hyphen join has
    /// carried since v1.1-S3.
    #[test]
    fn ink_contiguous_runs_become_one_block_and_one_segment() {
        let a = project_runs(&[
            ("Yarr", tagged_at(3), 7200, Some(1000), None),
            ("ow", tagged_at(3), 8200, Some(400), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["Yarrow"]);
        let multi: Vec<_> = a
            .anchor_map
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Source && s.node_ids.len() > 1)
            .collect();
        assert_eq!(
            multi.len(),
            1,
            "one segment naming both runs: {:?}",
            a.anchor_map.segments
        );
    }

    /// **A visible gap with no whitespace anywhere BREAKS the block.**
    ///
    /// The default is to break, and this is the clause that makes it so. A rule that joined here
    /// would invent a word wherever the page drew its space by positioning rather than by a glyph
    /// — 467 word boundaries on `nist-sp-800-171r3`, and `SynthesisReason::TjGap` fires **zero**
    /// times on four of the five gate documents, so nothing else catches them.
    #[test]
    fn a_drawn_gap_with_no_whitespace_breaks_the_block() {
        let a = project_runs(&[
            ("is", tagged_at(9), 7200, Some(1000), None),
            ("separate", tagged_at(9), 10000, Some(2000), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["is", "separate"]);
    }

    /// **Different marked-content ids are different blocks**, whatever the geometry says.
    #[test]
    fn different_mcids_never_join() {
        let a = project_runs(&[
            ("first", tagged_at(1), 7200, Some(1000), None),
            ("second", tagged_at(2), 8200, Some(1000), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["first", "second"]);
    }

    /// **Absence is not a group, but a baseline is a line** (v2.2-S5).
    ///
    /// Until this slice a run the document did not mark joined with nothing, and on an untagged
    /// PDF every run became its own block — a median of two characters per block over the 981
    /// OmniDocBench documents. Absence is still never a *group*: what licenses the join is not
    /// the missing declaration but the ink, drawn next along one baseline.
    ///
    /// **Both halves are asserted here on purpose.** The second is the sole in-unit guard for the
    /// welding disaster `group_key` was written against, and dropping it for brevity would leave
    /// a mutant that ignores `LineKey::baseline` alive.
    #[test]
    fn runs_with_no_declaration_join_only_along_one_baseline() {
        let joined = project_lines(&[
            ("first", None, 7200, 7200, Some(1000), None),
            ("second", None, 8200, 7200, Some(1000), None),
        ]);
        assert_eq!(blocks_of(&joined), vec!["firstsecond"]);

        // The same runs, one baseline apart. The x clause still passes — 8200 - (7200 + 1000) is
        // 0 — so `LineKey::baseline` is the only thing that can refuse, which is what puts it
        // under test rather than merely present.
        let apart = project_lines(&[
            ("first", None, 7200, 7200, Some(1000), None),
            ("second", None, 8200, 9600, Some(1000), None),
        ]);
        assert_eq!(blocks_of(&apart), vec!["first", "second"]);
    }

    /// **A page artifact with no `mcid` is not a group either** — the case above, on the locator
    /// whose `mcid` is documented as usually absent and is absent on all 4 826 artifact runs of
    /// `nist-sp-800-207`.
    #[test]
    fn artifacts_without_an_mcid_join_only_along_one_baseline() {
        let art = || {
            Some(StructuralLocator::PdfArtifact(
                crate::representation::PdfArtifactLocator { mcid: None },
            ))
        };
        // **Ink-contiguous on purpose.** Placed apart, the gap clause would break them and this
        // test would pass without the key ever being consulted — it did, and a mutant making
        // `mcid: None` a group survived it. Adjacent on one baseline they are one run of ink and
        // v2.2-S5 joins them; the claim that matters is the one below.
        let a = project_lines(&[
            ("NIST SP 800-207", art(), 7200, 7200, Some(1000), None),
            ("ZERO TRUST", art(), 8200, 7200, Some(1000), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["NIST SP 800-207ZERO TRUST"]);

        // A page artifact and body text sharing a baseline are still two streams the document
        // itself declared separate — `LineKey::artifact`. Corpus-wide this field refuses only
        // about ten pairs, so no measurement would catch a mutant that dropped it.
        let mixed = project_lines(&[
            ("NIST SP 800-207", art(), 7200, 7200, Some(1000), None),
            ("body", None, 8200, 7200, Some(1000), None),
        ]);
        assert_eq!(blocks_of(&mixed), vec!["NIST SP 800-207", "body"]);
    }

    /// **A column boundary breaks a block even inside one marked-content sequence.**
    ///
    /// `region` is in the key for the reason `hyphen_tail` needed it at D4-S3: 58 of 974 groups on
    /// `nist-sp-800-207` span two regions, and joining across one welds text over a gutter.
    #[test]
    fn a_region_boundary_breaks_the_block() {
        let a = project_runs(&[
            ("left", tagged_at(5), 7200, Some(1000), Some(1)),
            ("right", tagged_at(5), 8200, Some(1000), Some(2)),
        ]);
        assert_eq!(blocks_of(&a), vec!["left", "right"]);
    }

    /// **Every join is counted.** Declaring 14 863 list-item joins while committing 78 233 prose
    /// joins in silence was the asymmetry that made this code mandatory rather than optional.
    #[test]
    fn joins_are_declared_in_the_census() {
        let a = project_runs(&[
            ("Yarr", tagged_at(3), 7200, Some(1000), None),
            ("ow", tagged_at(3), 8200, Some(400), None),
        ]);
        let n = a
            .coverage
            .structural_erasures
            .iter()
            .find(|e| e.code == MCID_RUN_JOINS)
            .map(|e| e.count)
            .unwrap_or(0);
        assert_eq!(n, 1, "erasures: {:?}", a.coverage.structural_erasures);
    }

    // -----------------------------------------------------------------------------------
    // v2.2-S5 — the undeclared fallback
    //
    // Every test here builds the geometry so the clause under test is the DECIDING one. That
    // discipline is not decoration: at v2.2-S1 a mutant survived because the fixture placed its
    // runs 224 pt apart, so the gap clause broke them and the key was never consulted.
    // -----------------------------------------------------------------------------------

    /// `n` runs of one font, so the pitch reference exists.
    ///
    /// [`ink_reach`] refuses a font it has seen once, so a bare two-run fixture would refuse for
    /// that reason rather than the one under test. Placed on their own baselines, far to the
    /// right, so they are their own blocks and disturb nothing.
    fn corroborating(n: u32) -> Vec<LineSpec<'static>> {
        (0..n)
            .map(|i| {
                (
                    "ab",
                    None,
                    40000,
                    40000 + i64::from(i) * 2400,
                    Some(660),
                    None,
                )
            })
            .collect()
    }

    fn joined_block(specs: &[LineSpec<'_>], needle: &str) -> bool {
        blocks_of(&project_lines(specs))
            .iter()
            .any(|b| b.contains(needle))
    }

    /// **The vertical margin stamp, in the geometry the page draws it** (disaster 1).
    ///
    /// `nist-sp-800-207` sets "This publication is available free of charge from: …" down the
    /// margin of every page as page artifacts with no `mcid`, one fragment per baseline at a
    /// constant `origin_x`. An earlier draft read `mcid: None` as a group and welded them into
    /// `Thispublicationisavailable…` across 156 pt of white space, 59 times per document.
    #[test]
    fn an_artifact_stamp_down_the_margin_is_never_welded_into_one_block() {
        let art = || {
            Some(StructuralLocator::PdfArtifact(
                crate::representation::PdfArtifactLocator { mcid: None },
            ))
        };
        let a = project_lines(&[
            ("T", art(), 1968, 23274, Some(500), None),
            ("hi", art(), 1968, 23826, Some(500), None),
            ("s", art(), 1968, 24522, Some(500), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["T", "hi", "s"]);
    }

    /// **A run a shade wider than the font's median glyph keeps its join.**
    ///
    /// The cap in [`ink_reach`] is `(glyphs + 1) × reference`, and it read `glyphs × reference`
    /// until this was measured. `reference` is a **median**, so half of all runs exceed
    /// `glyphs × reference` by construction, and the epsilon is 12 centipoints — a fraction of one
    /// percent over the median was enough to truncate a run's reach past the epsilon and split a
    /// word in half.
    ///
    /// From `docstructbench_llm-raw-scihub-o.O-chem.200700133.pdf_6`, in the corroborating font's
    /// units: `coordi` advances 2 000 over 6 glyphs where the median is 330, so the old cap of
    /// 1 980 put its reach 20 centipoints short, turning a true 8-centipoint gap into 28 and
    /// emitting `coordi` and `nation` as two blocks. That page projected one word per block and
    /// scored a flat 1.0 on OmniDocBench text edit distance.
    #[test]
    fn a_run_wider_than_the_median_glyph_still_joins() {
        let mut specs = corroborating(6);
        specs.push(("coordi", None, 7200, 7200, Some(2000), None));
        specs.push(("nation", None, 9208, 7200, Some(1980), None));
        assert!(
            joined_block(&specs, "coordination"),
            "a run 1% over the median glyph lost its join to the cap"
        );
    }

    /// **A run whose advance is the column pitch is not joined onto the next cell** — the test
    /// this rule is shaped around.
    ///
    /// `advance` is not an ink width. `docstructbench_llm-raw-scihub-o.O-ceat.200600410` draws a
    /// table row's `AC` at `origin_x` 31 181 with an advance of 12 053 — 6 026 per glyph against
    /// a font median of ~330 — so `origin_x + advance` lands inside the *next cell* and a
    /// one-sided gap test reads ~0 across 120 pt of white space, emitting `ACAA`. Capping the
    /// reach at `glyphs × the document's own reference` is the only clause that refuses it: the
    /// pair is ink-contiguous under [`ink_contiguous`], and it also passes a bound scaled by the
    /// run's *own* pitch, because an inflated advance loosens that bound in step.
    #[test]
    fn a_run_whose_advance_is_the_column_pitch_is_not_joined() {
        let mut specs = corroborating(6);
        specs.push(("AC", None, 31181, 7200, Some(12053), None));
        specs.push(("AA", None, 43229, 7200, Some(660), None));
        assert!(
            !joined_block(&specs, "ACAA"),
            "the cell pitch was read as an ink width"
        );
    }

    /// **A run drawn at the same origin is drawn over, not after.** 103 such pairs occur in 153
    /// corpus documents and 98 are byte-identical duplicates — an overprinted bold effect.
    #[test]
    fn a_run_drawn_at_the_same_origin_is_not_joined() {
        let mut specs = corroborating(2);
        specs.push(("图说", None, 7200, 7200, Some(660), None));
        specs.push(("图说", None, 7200, 7200, Some(660), None));
        assert!(!joined_block(&specs, "图说图说"));
    }

    /// **A run drawn to the left of the open one is a backward jump, not the next ink.**
    #[test]
    fn a_run_drawn_to_the_left_is_not_joined() {
        let mut specs = corroborating(2);
        specs.push(("Diversified", None, 27000, 7200, Some(660), None));
        specs.push(("Fast", None, 7407, 7200, Some(660), None));
        assert!(!joined_block(&specs, "DiversifiedFast"));
    }

    /// **A hyphen inside a line is the author's, and a geometric join keeps it** (disaster 4).
    ///
    /// `cfpb-home-loan-toolkit` p24 draws `non-escrowed` as `non-`, `escr`, `o`, `w` … at one
    /// baseline, and the hyphen rule produced `nonescr`. The fallback concatenates verbatim, so
    /// the only way it can fail is by deleting a character it was never asked to touch.
    #[test]
    fn a_hyphen_inside_a_line_is_kept_when_geometry_joins() {
        let mut specs = corroborating(2);
        specs.push(("non-", None, 7200, 7200, Some(660), None));
        specs.push(("escr", None, 7860, 7200, Some(660), None));
        assert!(joined_block(&specs, "non-escr"), "the hyphen was dropped");
    }

    /// **A space the page drew as its own run is still the space the page drew** (disaster 3).
    ///
    /// The whitespace-only run is skipped before any join state sees it, which is how an earlier
    /// draft produced `backupwithholding` on `irs-fw9`. Here the three runs carry no declaration
    /// at all, so the fallback must carry `pending_space` across the skip — and must carry the
    /// *reach* across it too, or the block breaks at every drawn space.
    #[test]
    fn a_skipped_whitespace_run_is_the_space_the_page_drew_here_too() {
        let mut specs = corroborating(2);
        specs.push(("backup", None, 7200, 7200, Some(660), None));
        specs.push((" ", None, 7860, 7200, Some(200), None));
        specs.push(("withholding", None, 8060, 7200, Some(660), None));
        assert!(joined_block(&specs, "backup withholding"));
    }

    /// **A declared run never joins an undeclared one, in either order.** The fallback arm
    /// matches only when both sides carry no key; no corpus measurement exercises this.
    #[test]
    fn a_declared_run_never_joins_an_undeclared_one() {
        let mut fwd = corroborating(2);
        fwd.push(("left", None, 7200, 7200, Some(660), None));
        fwd.push(("right", tagged_at(1), 7860, 7200, Some(660), None));
        assert!(!joined_block(&fwd, "leftright"));

        let mut back = corroborating(2);
        back.push(("left", tagged_at(1), 7200, 7200, Some(660), None));
        back.push(("right", None, 7860, 7200, Some(660), None));
        assert!(!joined_block(&back, "leftright"));
    }

    /// **A visible gap breaks an undeclared block, and no advance is not contiguity.**
    #[test]
    fn a_visible_gap_breaks_an_undeclared_block() {
        let mut gap = corroborating(2);
        gap.push(("left", None, 7200, 7200, Some(660), None));
        gap.push(("right", None, 20000, 7200, Some(660), None));
        assert!(!joined_block(&gap, "leftright"));

        let mut none = corroborating(2);
        none.push(("left", None, 7200, 7200, None, None));
        none.push(("right", None, 7860, 7200, Some(660), None));
        assert!(!joined_block(&none, "leftright"));
    }

    /// **Geometric joins are declared under their own codes, never pooled with declared ones.**
    #[test]
    fn geometric_joins_are_declared_under_their_own_code() {
        let count = |a: &MarkdownArtifact, code: &str| {
            a.coverage
                .structural_erasures
                .iter()
                .find(|e| e.code == code)
                .map(|e| e.count)
                .unwrap_or(0)
        };
        let abutted = project_lines(&[
            ("Yarr", None, 7200, 7200, Some(1000), None),
            ("ow", None, 8200, 7200, Some(400), None),
        ]);
        assert_eq!(count(&abutted, BASELINE_RUN_JOINS_ABUTTED), 1);
        assert_eq!(count(&abutted, BASELINE_RUN_JOINS_SPACED), 0);
        assert_eq!(count(&abutted, MCID_RUN_JOINS), 0);

        // Corroborated, like every other fixture here: without it the reference pitch is a
        // median of the three runs under test, and the space's own advance is capped below its
        // width by the run it is supposed to be measured against.
        let mut runs = corroborating(2);
        runs.push(("backup", None, 7200, 7200, Some(660), None));
        runs.push((" ", None, 7860, 7200, Some(200), None));
        runs.push(("withholding", None, 8060, 7200, Some(660), None));
        let spaced = project_lines(&runs);
        assert_eq!(count(&spaced, BASELINE_RUN_JOINS_SPACED), 1);
        assert_eq!(count(&spaced, BASELINE_RUN_JOINS_ABUTTED), 0);
    }

    /// **A geometric join never puts two nodes in one segment.** The fallback calls `source`, not
    /// `source_continuing`, so the seam between two runs *this engine* joined stays addressable
    /// to the byte — which is the whole difference between a producer's join and a measured one.
    #[test]
    fn a_geometric_join_never_puts_two_nodes_in_one_segment() {
        let a = project_lines(&[
            ("Yarr", None, 7200, 7200, Some(1000), None),
            ("ow", None, 8200, 7200, Some(400), None),
        ]);
        assert_eq!(blocks_of(&a), vec!["Yarrow"]);
        for seg in a
            .anchor_map
            .segments
            .iter()
            .filter(|s| s.kind == SegmentKind::Source)
        {
            assert_eq!(
                seg.node_ids.len(),
                1,
                "segments: {:?}",
                a.anchor_map.segments
            );
        }
    }

    // -----------------------------------------------------------------------------------
    // Auto-tagging S3 — a sequence this engine wrote is no declaration
    // -----------------------------------------------------------------------------------

    /// **Two runs of one computed sequence on different baselines do NOT join; the same runs
    /// under an author's declaration do** (scope §4.3).
    ///
    /// Under `Extracted` the arm is v2.2-S1's: the producer said the two runs were one thing,
    /// and a line break inside one sequence renders as a space. Under `Computed` the producer is
    /// this engine, `group_key` answers `None`, and the undeclared path refuses to cross a
    /// baseline exactly as it does on the untagged original — the pair is the one
    /// `runs_with_no_declaration_join_only_along_one_baseline` keeps apart. Both projections and
    /// the grounding's grouping go through the one function, so the block rule is asserted on
    /// the Markdown and on `geometric_blocks` here.
    #[test]
    fn a_computed_sequence_never_joins_across_a_baseline() {
        let apart = |derivation: DerivationClass| -> Vec<LineSpec<'static>> {
            vec![
                (
                    "first",
                    tagged_as(4, derivation),
                    7200,
                    7200,
                    Some(1000),
                    None,
                ),
                (
                    "second",
                    tagged_as(4, derivation),
                    8200,
                    9600,
                    Some(1000),
                    None,
                ),
            ]
        };
        assert_eq!(
            blocks_of(&project_lines(&apart(DerivationClass::Extracted))),
            vec!["first second"],
            "an author's sequence joins across its own line break"
        );
        assert_eq!(
            blocks_of(&project_lines(&apart(DerivationClass::Computed))),
            vec!["first", "second"],
            "this engine's sequence is no licence to weld across a baseline"
        );
        assert_eq!(
            blocks_of_nodes(&apart(DerivationClass::Extracted)),
            vec!["firstsecond"]
        );
        assert_eq!(
            blocks_of_nodes(&apart(DerivationClass::Computed)),
            vec!["first", "second"]
        );
    }

    /// **A computed sequence takes the undeclared path, not a third one.** Ink-contiguous along
    /// one baseline the two runs still join; the join is declared under the geometric code and
    /// never `MCID_RUN_JOINS`; the seam stays addressable — two `source` segments, never one —
    /// because the contiguity is this engine's and not the document's; and the artifact is the
    /// untagged pair's, field for field. That equality is what lets an engine-tagged document
    /// project as its untagged twin.
    #[test]
    fn a_computed_sequence_joins_only_as_an_undeclared_run_would() {
        let count = |a: &MarkdownArtifact, code: &str| {
            a.coverage
                .structural_erasures
                .iter()
                .find(|e| e.code == code)
                .map(|e| e.count)
                .unwrap_or(0)
        };
        let computed = project_lines(&[
            (
                "Yarr",
                tagged_as(3, DerivationClass::Computed),
                7200,
                7200,
                Some(1000),
                None,
            ),
            (
                "ow",
                tagged_as(3, DerivationClass::Computed),
                8200,
                7200,
                Some(400),
                None,
            ),
        ]);
        let untagged = project_lines(&[
            ("Yarr", None, 7200, 7200, Some(1000), None),
            ("ow", None, 8200, 7200, Some(400), None),
        ]);
        assert_eq!(blocks_of(&computed), vec!["Yarrow"]);
        assert_eq!(computed.markdown, untagged.markdown);
        assert_eq!(computed.anchor_map, untagged.anchor_map);
        assert_eq!(computed.coverage, untagged.coverage);
        assert_eq!(count(&computed, BASELINE_RUN_JOINS_ABUTTED), 1);
        assert_eq!(count(&computed, MCID_RUN_JOINS), 0);
        assert!(
            computed
                .anchor_map
                .segments
                .iter()
                .filter(|s| s.kind == SegmentKind::Source)
                .all(|s| s.node_ids.len() == 1),
            "the seam between two runs this engine joined stays addressable: {:?}",
            computed.anchor_map.segments
        );
    }

    // -----------------------------------------------------------------------------------
    // v2.2-S7 — geometric blocks, the grouping the grounding projection cites
    // -----------------------------------------------------------------------------------

    /// Flag a run's trailing space as one this reader inserted, the way `content.rs` flags a
    /// `TJ` gap it read as a word space.
    ///
    /// A local helper rather than a seventh field on `LineSpec`: one test needs this and every
    /// other caller of `placed_run` would have to carry it.
    fn reader_spaced(mut n: Node) -> Node {
        let last = u32::try_from(n.text.chars().count() - 1).expect("short fixture text");
        if let NodeAttributes::TextRun(a) = &mut n.attributes {
            a.synthesized.push(crate::SynthesizedAt {
                char_index: last,
                reason: "tj-gap".into(),
            });
        }
        n
    }

    /// **A word gap the page opened by moving the cursor keeps the line together** (v2.2-S7).
    ///
    /// The page draws no space glyph; the reader recognises the `TJ` gap, writes a space into the
    /// run's own text and flags it. Before this, the block rule then measured that same gap
    /// against the 12-centipoint quantization epsilon, called it a break, and emitted one word per
    /// block — 296 of 735 OmniDocBench documents projected that way.
    ///
    /// The gap here is 300 centipoints against a 330-centipoint reference: far outside the
    /// epsilon, and exactly the size of the space that is already sitting in `with `'s text.
    #[test]
    fn a_cursor_moved_word_gap_does_not_break_the_line() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 61200,
            height: 79200,
            rotation: 0,
        };
        let mut nodes: Vec<Node> = Vec::new();
        // Two corroborating runs so the pitch reference exists, then the pair under test.
        for i in 0..4u32 {
            nodes.push(placed_run(
                &mut alloc,
                &page.id,
                i + 1,
                "ab",
                None,
                40000,
                40000 + i64::from(i) * 2400,
                Some(660),
                None,
            ));
        }
        nodes.push(reader_spaced(placed_run(
            &mut alloc,
            &page.id,
            5,
            "with ",
            None,
            7200,
            7200,
            Some(1320),
            None,
        )));
        nodes.push(placed_run(
            &mut alloc,
            &page.id,
            6,
            "a",
            None,
            8820,
            7200,
            Some(330),
            None,
        ));
        let owned = std::collections::BTreeSet::new();
        let joined = geometric_blocks(&nodes, &owned)
            .into_iter()
            .any(|b| b.text.contains("with a"));
        assert!(
            joined,
            "a gap the reader already read as a space broke the line anyway"
        );
    }

    fn blocks_of_nodes(specs: &[LineSpec<'_>]) -> Vec<String> {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 61200,
            height: 79200,
            rotation: 0,
        };
        let nodes: Vec<Node> = specs
            .iter()
            .enumerate()
            .map(|(i, (t, l, x, y, a, r))| {
                placed_run(
                    &mut alloc,
                    &page.id,
                    i as u32 + 1,
                    t,
                    l.clone(),
                    *x,
                    *y,
                    *a,
                    *r,
                )
            })
            .collect();
        geometric_blocks(&nodes, &std::collections::BTreeSet::new())
            .into_iter()
            .map(|b| b.text)
            .collect()
    }

    /// **A block is the ink, and its text is the runs' own characters concatenated.**
    ///
    /// No separator is invented, because a space the page drew is a run with its own text — which
    /// is why this needs none of the Markdown projection's space rule.
    #[test]
    fn geometric_blocks_join_the_next_ink_along_one_baseline() {
        assert_eq!(
            blocks_of_nodes(&[
                ("Yar", None, 7200, 7200, Some(1000), None),
                ("row", None, 8200, 7200, Some(1000), None),
            ]),
            vec!["Yarrow"]
        );
    }

    /// **A drawn space is a member, not a separator**, so the block text carries it verbatim.
    #[test]
    fn a_drawn_space_is_a_member_of_the_block() {
        assert_eq!(
            blocks_of_nodes(&[
                ("ab", None, 40000, 40000, Some(660), None),
                ("ab", None, 40000, 42400, Some(660), None),
                ("backup", None, 7200, 7200, Some(660), None),
                (" ", None, 7860, 7200, Some(200), None),
                ("withholding", None, 8060, 7200, Some(660), None),
            ])
            .last()
            .unwrap(),
            "backup withholding"
        );
    }

    /// **Tracking is not a word gap, and the page's own space is what says so** (`-v9`).
    ///
    /// A page set with letter-spacing draws each glyph as its own run and leaves a few centipoints
    /// between the boxes — wider than the 12-centipoint quantization epsilon, narrower than
    /// anything the page calls a space. Before this clause every such letter became its own block:
    /// `01030000000103` projected 944 blocks averaging 1.2 characters, and its NID was 0.4456.
    ///
    /// The three runs below are 200 wide with 40 of tracking between them, and the same font draws
    /// a space of 200. Tracking joins; a gap the width of that drawn space does not, so the two
    /// words stay two blocks and no space the page drew is swallowed.
    #[test]
    fn tracking_joins_and_a_drawn_space_width_gap_does_not() {
        // Four runs of one word, 40 centipoints of tracking between each, plus a space run so the
        // font has a measure at all, then a second word one space-width further on.
        assert_eq!(
            blocks_of_nodes(&[
                ("O", None, 10000, 7200, Some(200), None),
                ("n", None, 10240, 7200, Some(200), None),
                ("c", None, 10480, 7200, Some(200), None),
                ("e", None, 10720, 7200, Some(200), None),
                (" ", None, 10960, 7200, Some(200), None),
                ("t", None, 11200, 7200, Some(200), None),
            ]),
            vec!["Once t"],
            "tracking narrower than the drawn space is one block"
        );

        // The same font, and a gap of exactly the space it draws: not narrower, so not joined.
        assert_eq!(
            blocks_of_nodes(&[
                ("a", None, 10000, 7200, Some(200), None),
                (" ", None, 10200, 7200, Some(200), None),
                ("b", None, 10400, 7200, Some(200), None),
                ("c", None, 10640, 7200, Some(200), None),
                ("far", None, 10840 + 200, 7200, Some(200), None),
            ])
            .len(),
            2,
            "a gap the width of the page's own space is a word gap, and breaks the block"
        );
    }

    /// **A block never crosses a baseline**, which is `LineKey` doing the same job it does in the
    /// projections — and the reason decision #21's territory is untouched.
    #[test]
    fn geometric_blocks_never_cross_a_baseline() {
        assert_eq!(
            blocks_of_nodes(&[
                ("first", None, 7200, 7200, Some(1000), None),
                ("second", None, 8200, 9600, Some(1000), None),
            ]),
            vec!["first", "second"]
        );
    }

    /// **A run a table already claims is its own block**, so no run is grounded twice.
    #[test]
    fn a_table_owned_run_is_never_joined_into_a_prose_block() {
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = PageRecord {
            id: alloc.next(IdKind::Page).unwrap(),
            index: 1,
            width: 61200,
            height: 79200,
            rotation: 0,
        };
        let nodes: Vec<Node> = [("Yar", 7200i64), ("row", 8200)]
            .iter()
            .enumerate()
            .map(|(i, (t, x))| {
                placed_run(
                    &mut alloc,
                    &page.id,
                    i as u32 + 1,
                    t,
                    None,
                    *x,
                    7200,
                    Some(1000),
                    None,
                )
            })
            .collect();
        let mut owned = std::collections::BTreeSet::new();
        owned.insert(nodes[1].id.as_str());
        let blocks: Vec<String> = geometric_blocks(&nodes, &owned)
            .into_iter()
            .map(|b| b.text)
            .collect();
        assert_eq!(blocks, vec!["Yar", "row"]);
    }
}
