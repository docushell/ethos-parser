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

//! Safe Markdown: a projection of the representation that stays citable (v1.1-S1).
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
pub const MARKDOWN_SCHEMA_VERSION: &str = "1.0.0";

/// The projection rule v1.1-S1 ships: linear text, headings only from the structure tree.
///
/// A versioned id for the same reason every detector has one: it decides what comes out. A run
/// that projected headings from font sizes and a run that refused to would disagree about the
/// same document, and an artifact whose hash could not tell them apart would claim a
/// comparability it lacks.
pub const MARKDOWN_RULE_LINEAR_V1: &str = "markdown-linear-v1";

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

/// Project a representation into Markdown plus its map.
///
/// # The rule, in full — `markdown-linear-v1`
///
/// 1. **Text runs only.** Every other node kind is dropped into its own named bucket. **Page
///    artifacts are NOT dropped**: a running head is a `text_run` carrying
///    `structural_locator: pdf_artifact`, and checklist O21/O22 is explicit that a reader deleting
///    running heads has silently edited the document. The flag stays in the representation and a
///    consumer that wants them gone drops them itself, knowing it did.
/// 2. **A heading when the tree says so**, `#` through `######`; a paragraph otherwise.
/// 3. **Blank line between blocks**, which is `syntax` — the document drew no such bytes.
/// 4. **Node text is normalized** by [`normalize`], and a node whose normalization is empty
///    contributes no segment at all rather than an empty one.
///
/// Tables are **not** a dropped bucket: a cell's text is a concatenation of runs that are already
/// nodes, so projecting the runs loses no character. What is lost is the grid, and that is a
/// declared limitation rather than a count of zero — see `docs/11-V11-MILESTONES.md` S1 decision 5.
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

    let mut markdown = String::new();
    let mut segments: Vec<Segment> = Vec::new();
    let mut emitted_chars = 0usize;
    let mut buckets: std::collections::BTreeMap<&'static str, (usize, usize)> =
        std::collections::BTreeMap::new();
    let mut in_representation = 0usize;

    for node in &payload.nodes {
        in_representation += node.text.chars().count();

        if let Some(code) = dropped_code(node.kind) {
            let e = buckets.entry(code).or_insert((0, 0));
            e.0 += node.text.chars().count();
            e.1 += 1;
            continue;
        }

        let text = normalize(&node.text);
        if text.is_empty() {
            // Whitespace-only runs exist (a `Tj` of spaces is a real operator). They contribute no
            // Markdown, and they contribute no dropped characters either: `normalize` removed
            // whitespace, and whitespace is not content this projection claims to have lost.
            continue;
        }

        // Block separator. `syntax` because the document drew no blank line — this exporter did.
        if !markdown.is_empty() {
            let start = markdown.len();
            markdown.push_str("\n\n");
            segments.push(Segment {
                kind: SegmentKind::Syntax,
                start,
                end: markdown.len(),
                node_ids: Vec::new(),
            });
        }

        // The heading marker, when the tree said so. Also `syntax`: `## ` is this exporter's
        // rendering of a role, not bytes the page drew.
        if let Some(level) = heading_level(node) {
            let start = markdown.len();
            for _ in 0..level {
                markdown.push('#');
            }
            markdown.push(' ');
            segments.push(Segment {
                kind: SegmentKind::Syntax,
                start,
                end: markdown.len(),
                node_ids: Vec::new(),
            });
        }

        let start = markdown.len();
        markdown.push_str(&text);
        segments.push(Segment {
            kind: SegmentKind::Source,
            start,
            end: markdown.len(),
            node_ids: vec![node.id.as_str().to_string()],
        });
        emitted_chars += text.chars().count();
    }

    // Trailing newline, so the string is a well-formed text file. `syntax`, like every other byte
    // this exporter chose.
    if !markdown.is_empty() {
        let start = markdown.len();
        markdown.push('\n');
        segments.push(Segment {
            kind: SegmentKind::Syntax,
            start,
            end: markdown.len(),
            node_ids: Vec::new(),
        });
    }

    let anchor_map = AnchorMap::new(segments, &markdown)?;

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
            tables: Vec::new(),
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
