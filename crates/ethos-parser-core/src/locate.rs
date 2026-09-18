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

//! **Where a string lies in a representation, and nothing else**
//! ([`docs/26-LOCATE-SCOPE.md`](../../../docs/26-LOCATE-SCOPE.md), North Star decision #30).
//!
//! It answers this engine's own question — *what does this document contain, and exactly where?*
//! — for a caller holding a string instead of a node id. What it must never answer is *whether*:
//! there is no verdict here, no boolean, no score and no evidence tier, and a string occurring
//! nowhere is an empty answer with the same exit code as one occurring five times. The bound the
//! design is arranged around is `docs/07-VERIFY-BOUNDARY.md` §2's *the engine has no claim
//! input*: this takes a representation and a string, and a string is not a claim.
//!
//! # The match rule is this engine's own, and it is not the verifier's
//!
//! [`LOCATE_RULE_V1`] is code-point-exact on Unicode scalars: no normalisation, no case folding,
//! no whitespace folding. It searches each **block** of the reading-order cut — the string
//! [`crate::markdown::geometric_blocks`] builds, which is also the string
//! `ethos.grounding.v1`'s element text is — so a match may join runs inside one block and may
//! **not** join across two. A block boundary is a gap the page drew, and joining it would assert
//! an adjacency the document does not have. The scope's §4.2 tabulates the five places the
//! verifier resolves a quote differently; whether a location supports a claim stays its business.
//!
//! # What an occurrence carries, and what it never carries
//!
//! Node ids copied verbatim out of the record, character offsets into **each node's own text**,
//! and the representation's own geometry for those nodes, copied rather than derived. No union
//! box, no sub-run box (word boxes are refused on measurement, `docs/22-WORD-BOXES-SCOPE.md`
//! §7), no page, and no element id of its own. Everything an occurrence holds is a copy of
//! something the record already states, which is what keeps it from becoming a second evidence
//! tier.

use serde::{Deserialize, Serialize};

use crate::representation::DocumentRepresentation;
use crate::{ArtifactIdentity, EngineError, GeometryPresence, NodeId, Sha256Hex};

/// The artifact type this emits.
pub const LOCATIONS_ARTIFACT_TYPE: &str = "ethos.parser.locations.v0";

/// Its schema version.
pub const LOCATIONS_SCHEMA_VERSION: &str = "0.1.0";

/// The match rule, versioned because the answer depends on it.
pub const LOCATE_RULE_V1: &str = "locate-scalar-exact-v1";

/// The longest quote this accepts, in UTF-8 bytes.
///
/// The number is `ethos.grounding.v1`'s own longest admissible string, so a longer quote is longer
/// than any element text a citation could carry. **It is re-declared here rather than imported**,
/// because `ethos-parser-grounding` depends on this crate and not the other way round; a test in
/// that crate, which can see both, asserts the two numbers are equal.
pub const LOCATE_MAX_QUOTE_BYTES: usize = 16_384;

/// The most occurrences this returns locators for.
///
/// `ethos.grounding.v1`'s own element ceiling, re-declared for the reason above. Past it the count
/// travels and the locators do not — all of them, never the excess alone, because a truncated list
/// would locate some of a document's occurrences and silently drop the rest.
pub const LOCATE_MAX_OCCURRENCES: usize = 1_000_000;

/// Where one string lies in one representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Locations {
    /// `artifact_type`, `schema_version`, `parser_version`, `profile_sha256`.
    #[serde(flatten)]
    pub identity: ArtifactIdentity,
    /// Digest of the original source bytes, carried through from the representation.
    pub source_sha256: Sha256Hex,
    /// Digest of the representation searched — the binding that makes an occurrence checkable.
    pub representation_sha256: Sha256Hex,
    /// Which match rule produced this.
    pub locate_rule: String,
    /// The quote's length in Unicode scalars, so a consumer can check an offset range's width
    /// without holding the quote.
    pub quote_scalars: u32,
    /// What was searched, because the rule searches blocks and not the whole record's text.
    pub searched: Searched,
    /// Every occurrence, in reading order. **Reading order is not a ranking**; nothing here
    /// prefers one occurrence to another.
    pub occurrences: Vec<Occurrence>,
    /// Set only when the count passed [`LOCATE_MAX_OCCURRENCES`], in which case `occurrences` is
    /// empty and this carries the count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrences_withheld: Option<OccurrencesWithheld>,
}

/// What the rule looked at, stated because it is less than the whole record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Searched {
    /// Nodes in the representation.
    pub nodes: u32,
    /// Blocks the cut grouped them into — the strings actually searched.
    pub blocks: u32,
    /// Unicode scalars across those blocks.
    pub scalars: u32,
}

/// One place the string occurs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Occurrence {
    /// The nodes its scalars touch, in order. Their slices, concatenated, equal the quote.
    pub parts: Vec<OccurrencePart>,
    /// How many of this occurrence's scalars this reader synthesized rather than read.
    ///
    /// Stated always, never omitted at zero: a consumer matching a quote against a space the
    /// source does not contain is the failure `docs/01-CONTRACT.md` §10 names, and this is the
    /// place that can say so.
    pub synthesized: u32,
}

/// One node's share of an occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccurrencePart {
    /// The node id, copied out of the representation searched.
    pub node: NodeId,
    /// First scalar of this node's own `text`, inclusive.
    pub char_start: u32,
    /// One past the last scalar, exclusive.
    pub char_end: u32,
    /// That node's geometry, copied from the representation's sidecar.
    ///
    /// **A box here is bound to the record by a digest that never covered it**: geometry sits
    /// outside the representation's fingerprint by design (`docs/01-CONTRACT.md` §4), and nobody
    /// should read `representation_sha256` as covering a rectangle it never touched.
    pub geometry: GeometryPresence,
}

/// The count, where the locators are withheld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccurrencesWithheld {
    /// How many occurrences there are.
    pub occurrences: u64,
    /// The ceiling they passed.
    pub limit: u32,
}

impl Locations {
    /// The canonical bytes this artifact serializes to.
    ///
    /// # Errors
    ///
    /// [`EngineError::Malformed`] if the artifact will not canonicalize.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, EngineError> {
        crate::c14n::canonical_bytes_of(self).map_err(|e| EngineError::Malformed {
            what: "locations artifact".into(),
            detail: e.to_string(),
        })
    }
}

fn unsupported(detail: String) -> EngineError {
    EngineError::Unsupported {
        what: "locate".into(),
        detail,
    }
}

/// Where `quote` lies in `repr`.
///
/// # Errors
///
/// [`EngineError::Unsupported`] on an empty quote, or one longer than
/// [`LOCATE_MAX_QUOTE_BYTES`]. **Neither is the not-found answer**: a string that occurs nowhere
/// is `Ok` with an empty `occurrences`, and the empty string is refused because it occurs at every
/// offset of every block, so *every place it occurs* has no answer with a meaning.
///
/// [`EngineError::Malformed`] if a count will not fit the wire's integers, which needs a
/// representation larger than the address space allows.
pub fn locate(
    repr: &DocumentRepresentation,
    parser_version: &str,
    profile_sha256: &Sha256Hex,
    locate_rule: &str,
    quote: &str,
) -> Result<Locations, EngineError> {
    if quote.is_empty() {
        return Err(unsupported(
            "the quote is empty, and the empty string occurs at every offset of every block, so \
             there is no answer to where it occurs. This is not the not-found answer: a string \
             that occurs nowhere is an empty occurrence list and the same exit code"
                .into(),
        ));
    }
    if quote.len() > LOCATE_MAX_QUOTE_BYTES {
        return Err(unsupported(format!(
            "the quote is {} bytes, past the {LOCATE_MAX_QUOTE_BYTES}-byte limit, which is the \
             longest string `ethos.grounding.v1` admits — so it is longer than any element text a \
             citation could carry",
            quote.len()
        )));
    }

    let payload = repr.payload();
    let table_owned: std::collections::BTreeSet<&str> = payload
        .tables
        .iter()
        .flat_map(|t| t.cells.iter())
        .flat_map(|c| c.node_ids.iter())
        .map(NodeId::as_str)
        .collect();
    let blocks = crate::markdown::geometric_blocks(&payload.nodes, &table_owned);

    let quote_scalars = quote.chars().count();
    let mut searched_scalars: usize = 0;
    let mut occurrences: Vec<Occurrence> = Vec::new();
    let mut total: u64 = 0;

    for block in &blocks {
        // Where each member's own text starts, in block scalars. **Computed once per block, not
        // once per match**: with the member lengths re-counted per occurrence, one block of a
        // million scalars holding a million matches costs a million passes over it, and the
        // ceiling below would be a ceiling on an answer nobody can wait for.
        let mut member_starts: Vec<usize> = Vec::with_capacity(block.members.len() + 1);
        member_starts.push(0);
        let mut acc = 0usize;
        for &member in &block.members {
            acc += node_at(&payload.nodes, member)?.text.chars().count();
            member_starts.push(acc);
        }
        searched_scalars = searched_scalars.saturating_add(acc);

        // Every start position, overlapping included: `aa` in `aaa` occurs twice. `find` on the
        // remainder is the fast path, and the cursor advances one scalar at a time so nothing is
        // skipped past.
        let mut from = 0usize;
        let mut scalars_before = 0usize;
        while let Some(rel) = block.text.get(from..).and_then(|rest| rest.find(quote)) {
            let at = from + rel;
            // Byte offsets arrive in increasing order, so the scalar index is counted forward
            // from the last one rather than from the start of the block.
            scalars_before += block.text[from..at].chars().count();
            total = total.saturating_add(1);
            if occurrences.len() < LOCATE_MAX_OCCURRENCES {
                occurrences.push(occurrence_at(
                    repr,
                    block,
                    &member_starts,
                    scalars_before,
                    quote_scalars,
                )?);
            }
            // One scalar on from the match's start.
            let step = block.text[at..].chars().next().map_or(1, char::len_utf8);
            from = at + step;
            scalars_before += 1;
        }
    }

    let withheld = usize::try_from(total).map_or(true, |n| n > LOCATE_MAX_OCCURRENCES);
    if withheld {
        occurrences.clear();
    }

    let count = |n: usize, what: &str| -> Result<u32, EngineError> {
        u32::try_from(n).map_err(|_| EngineError::Malformed {
            what: "locations artifact".into(),
            detail: format!("{what} does not fit the wire's integers: {n}"),
        })
    };

    Ok(Locations {
        identity: ArtifactIdentity {
            artifact_type: LOCATIONS_ARTIFACT_TYPE.to_string(),
            schema_version: LOCATIONS_SCHEMA_VERSION.to_string(),
            parser_version: parser_version.to_string(),
            profile_sha256: profile_sha256.clone(),
        },
        source_sha256: payload.source.sha256.clone(),
        representation_sha256: repr.fingerprint().clone(),
        locate_rule: locate_rule.to_string(),
        quote_scalars: count(quote_scalars, "the quote's scalar count")?,
        searched: Searched {
            nodes: count(payload.nodes.len(), "the node count")?,
            blocks: count(blocks.len(), "the block count")?,
            scalars: count(searched_scalars, "the searched scalar count")?,
        },
        occurrences,
        occurrences_withheld: withheld.then_some(OccurrencesWithheld {
            occurrences: total,
            limit: LOCATE_MAX_OCCURRENCES as u32,
        }),
    })
}

/// A block member, as a node of the representation it came from.
fn node_at(nodes: &[crate::Node], member: usize) -> Result<&crate::Node, EngineError> {
    nodes.get(member).ok_or_else(|| EngineError::Malformed {
        what: "locations artifact".into(),
        detail: format!("block member {member} is not a node of this representation"),
    })
}

/// One occurrence, from the block-scalar range it covers.
///
/// The parts are the members the range touches, each with offsets into **its own** text — the
/// invariant a test asserts on every fixture: the parts' slices, concatenated, equal the quote.
/// `member_starts` is the block's prefix of member scalar starts, one longer than its members.
fn occurrence_at(
    repr: &DocumentRepresentation,
    block: &crate::markdown::GeometricBlock,
    member_starts: &[usize],
    start: usize,
    scalars: usize,
) -> Result<Occurrence, EngineError> {
    let nodes = &repr.payload().nodes;
    let end = start + scalars;
    let mut parts = Vec::new();
    let mut synthesized: u32 = 0;

    // The member `start` falls in, found rather than walked to. A member of empty text shares
    // its predecessor's start, and lands after it here, which is right: an empty member holds
    // none of the match and the overlap below drops it anyway.
    let first = member_starts
        .partition_point(|&s| s <= start)
        .saturating_sub(1);
    for (i, &member) in block.members.iter().enumerate().skip(first) {
        let node = node_at(nodes, member)?;
        let node_start = member_starts[i];
        let node_end = member_starts[i + 1];
        if node_start >= end {
            break;
        }

        // The overlap of [start, end) with this node's own [node_start, node_end).
        let from = start.max(node_start);
        let to = end.min(node_end);
        if from >= to {
            continue;
        }
        let in_node_start = from - node_start;
        let in_node_end = to - node_start;

        // A character this reader inserted is still one of the occurrence's scalars, and the
        // count is what lets a consumer tell a matched space from a drawn one.
        if let crate::NodeAttributes::TextRun(attrs) = &node.attributes {
            synthesized += u32::try_from(
                attrs
                    .synthesized
                    .iter()
                    .filter(|s| {
                        let at = s.char_index as usize;
                        at >= in_node_start && at < in_node_end
                    })
                    .count(),
            )
            .unwrap_or(u32::MAX);
        }

        parts.push(OccurrencePart {
            node: node.id.clone(),
            char_start: u32::try_from(in_node_start).unwrap_or(u32::MAX),
            char_end: u32::try_from(in_node_end).unwrap_or(u32::MAX),
            geometry: repr
                .geometry_at(member)
                .ok_or_else(|| EngineError::Malformed {
                    what: "locations artifact".into(),
                    detail: format!("node {} has no geometry row", node.id.as_str()),
                })?,
        });
    }

    Ok(Occurrence { parts, synthesized })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::tests::{repr_of_placed, LineSpec};
    use crate::{DocumentRepresentation, Profile};

    /// The hand-built representations these tests run over read `node.text` and the placement,
    /// which is everything the block rule and this module look at. The helper's `char_codes` are
    /// the text's bytes — wrong for a non-ASCII run, and unread here, which
    /// [`an_astral_scalar_is_one_scalar_one_offset_and_four_bytes`] says again where it matters.
    fn repr_of(specs: &[LineSpec<'_>]) -> DocumentRepresentation {
        repr_of_placed(specs)
    }

    /// One run on one baseline.
    fn one_run(text: &str) -> DocumentRepresentation {
        repr_of(&[(text, None, 7200, 7200, Some(1000), None)])
    }

    fn located(repr: &DocumentRepresentation, quote: &str) -> Locations {
        let profile = Profile::default();
        locate(
            repr,
            &profile.parser_version,
            &profile.profile_sha256().unwrap(),
            &profile.locate_rule,
            quote,
        )
        .expect("a representation and a quote locate")
    }

    fn refusal(repr: &DocumentRepresentation, quote: &str) -> EngineError {
        let profile = Profile::default();
        locate(
            repr,
            &profile.parser_version,
            &profile.profile_sha256().unwrap(),
            &profile.locate_rule,
            quote,
        )
        .expect_err("refused")
    }

    /// One part's own slice of its own node's text.
    fn slice_of(repr: &DocumentRepresentation, part: &OccurrencePart) -> String {
        let node = repr
            .payload()
            .nodes
            .iter()
            .find(|n| n.id.as_str() == part.node.as_str())
            .expect("a part names a node of the representation it came from");
        node.text
            .chars()
            .skip(part.char_start as usize)
            .take((part.char_end - part.char_start) as usize)
            .collect()
    }

    /// The invariant this module exists to protect (scope §9 T12): **the parts' own slices,
    /// concatenated in order, equal the quote.** It fails the moment an offset is written against
    /// the block's text instead of the node's.
    fn parts_reconstruct_the_quote(repr: &DocumentRepresentation, found: &Locations, quote: &str) {
        for (i, occurrence) in found.occurrences.iter().enumerate() {
            let joined: String = occurrence
                .parts
                .iter()
                .map(|p| slice_of(repr, p))
                .collect::<Vec<_>>()
                .concat();
            assert_eq!(
                joined, quote,
                "occurrence {i}'s parts, concatenated, must be the quote — offsets are into each \
                 node's own text, never the block's"
            );
            assert!(
                !occurrence.parts.is_empty(),
                "occurrence {i} has no part, so it locates nothing"
            );
        }
    }

    /// Every emitted geometry is the sidecar's own row, copied rather than derived (T13).
    fn geometry_is_the_sidecars(repr: &DocumentRepresentation, found: &Locations) {
        for occurrence in &found.occurrences {
            for part in &occurrence.parts {
                let index = repr
                    .payload()
                    .nodes
                    .iter()
                    .position(|n| n.id.as_str() == part.node.as_str())
                    .expect("the part names a node");
                assert_eq!(
                    Some(part.geometry),
                    repr.geometry_at(index),
                    "an occurrence's geometry must be the representation's own row for that node"
                );
            }
        }
    }

    // ---------------------------------------------------------------------------------------
    // Found, and not found
    // ---------------------------------------------------------------------------------------

    /// **T1** — one run, one occurrence, one part, offsets into that run's own text.
    #[test]
    fn a_quote_inside_one_run_is_one_occurrence_of_one_part() {
        let repr = one_run("Measured");
        let found = located(&repr, "Measured");

        assert_eq!(found.occurrences.len(), 1);
        let o = &found.occurrences[0];
        assert_eq!(o.parts.len(), 1);
        assert_eq!((o.parts[0].char_start, o.parts[0].char_end), (0, 8));
        assert_eq!(o.synthesized, 0, "nothing here was synthesized");
        assert_eq!(found.quote_scalars, 8);
        assert_eq!(
            found.searched,
            Searched {
                nodes: 1,
                blocks: 1,
                scalars: 8
            }
        );
        assert!(found.occurrences_withheld.is_none());
        parts_reconstruct_the_quote(&repr, &found, "Measured");
        geometry_is_the_sidecars(&repr, &found);
    }

    /// **T2** — a string that occurs nowhere is an **answer**, not a refusal: `Ok`, an empty
    /// list, and the same identity as a found one. The absence of a verdict is the point
    /// (decision #30, `docs/07-VERIFY-BOUNDARY.md` §2).
    #[test]
    fn a_string_that_occurs_nowhere_is_an_empty_answer_and_not_an_error() {
        let repr = one_run("Measured");
        let found = located(&repr, "Absent");

        assert!(found.occurrences.is_empty());
        assert!(found.occurrences_withheld.is_none(), "nothing was withheld");
        assert_eq!(found.searched.blocks, 1, "the block was searched");
        assert_eq!(
            found.identity.artifact_type, LOCATIONS_ARTIFACT_TYPE,
            "the same artifact a found quote produces"
        );
        assert_eq!(found.locate_rule, LOCATE_RULE_V1);
    }

    /// **T3** — every start position is an occurrence, overlapping included, and their order is
    /// the block's order. Nothing here ranks them.
    #[test]
    fn overlapping_starts_are_each_an_occurrence() {
        let repr = one_run("aaa");
        let found = located(&repr, "aa");

        assert_eq!(found.occurrences.len(), 2, "`aa` occurs twice in `aaa`");
        let spans: Vec<(u32, u32)> = found
            .occurrences
            .iter()
            .map(|o| (o.parts[0].char_start, o.parts[0].char_end))
            .collect();
        assert_eq!(spans, vec![(0, 2), (1, 3)]);
        parts_reconstruct_the_quote(&repr, &found, "aa");
    }

    /// **T4** — a quote crossing runs inside one block has a part per run, each with offsets
    /// into its own text. `untagged-shredded-line`'s shape, hand-built: three abutting runs on
    /// one baseline whose block text is `Yarrow`.
    #[test]
    fn a_quote_crossing_runs_of_one_block_has_a_part_per_run() {
        let repr = repr_of(&[
            ("Yar", None, 7200, 7200, Some(1000), None),
            ("ro", None, 8200, 7200, Some(660), None),
            ("w", None, 8860, 7200, Some(330), None),
        ]);
        let found = located(&repr, "arrow");

        assert_eq!(found.searched.blocks, 1, "the three runs are one block");
        assert_eq!(found.occurrences.len(), 1);
        let parts = &found.occurrences[0].parts;
        assert_eq!(parts.len(), 3, "one part per run the quote touches");
        assert_eq!((parts[0].char_start, parts[0].char_end), (1, 3), "`ar`");
        assert_eq!((parts[1].char_start, parts[1].char_end), (0, 2), "`ro`");
        assert_eq!((parts[2].char_start, parts[2].char_end), (0, 1), "`w`");
        parts_reconstruct_the_quote(&repr, &found, "arrow");
        geometry_is_the_sidecars(&repr, &found);
    }

    /// **T5** — a quote crossing a block boundary is not an occurrence. A boundary is a gap the
    /// page drew, and joining it would assert an adjacency the document does not have.
    #[test]
    fn a_quote_crossing_a_block_boundary_is_not_an_occurrence() {
        let repr = repr_of(&[
            ("First block", None, 7200, 7200, Some(2000), None),
            ("Second block", None, 7200, 9600, Some(2000), None),
        ]);
        assert_eq!(located(&repr, "blockSecond").occurrences.len(), 0);
        assert_eq!(
            located(&repr, "First block").occurrences.len(),
            1,
            "each side is still found on its own"
        );
        assert_eq!(located(&repr, "Second block").occurrences.len(), 1);
        assert_eq!(located(&repr, "First block").searched.blocks, 2);
    }

    /// **T6's unit half** — text that exists only after a projection is not in the document.
    /// The Markdown a reader would print joins these two with a blank line; the block text does
    /// not, and this searches the block text.
    #[test]
    fn a_string_only_a_projection_produces_is_not_an_occurrence() {
        let repr = repr_of(&[
            ("First block", None, 7200, 7200, Some(2000), None),
            ("Second block", None, 7200, 9600, Some(2000), None),
        ]);
        assert_eq!(located(&repr, "block\n\nSecond").occurrences.len(), 0);
    }

    // ---------------------------------------------------------------------------------------
    // The refusals, which are not the not-found answer
    // ---------------------------------------------------------------------------------------

    /// **T7** — the empty quote is refused, and the refusal is a different path from T2's
    /// empty answer. The empty string occurs at every offset of every block, so *where it
    /// occurs* has no answer with a meaning.
    #[test]
    fn the_empty_quote_is_refused_and_that_is_not_the_not_found_answer() {
        let repr = one_run("Measured");
        let err = refusal(&repr, "");
        assert!(
            matches!(err, EngineError::Unsupported { .. }),
            "the empty quote is unsupported, not malformed: {err}"
        );
        assert!(
            format!("{err}").contains("the quote is empty"),
            "the message names the empty quote: {err}"
        );
        // The contrast is the test: one is an error, the other an artifact.
        assert!(located(&repr, "Absent").occurrences.is_empty());
    }

    /// A quote past the byte ceiling is refused, and the message carries both numbers.
    #[test]
    fn a_quote_past_the_byte_ceiling_is_refused() {
        let repr = one_run("Measured");
        let long = "a".repeat(LOCATE_MAX_QUOTE_BYTES + 1);
        let err = refusal(&repr, &long);
        let text = format!("{err}");
        assert!(matches!(err, EngineError::Unsupported { .. }), "{text}");
        assert!(
            text.contains("16385"),
            "the message states the size: {text}"
        );
        assert!(text.contains("16384"), "and the ceiling: {text}");
        // One byte under it is not refused, so the boundary is where it is stated to be.
        let at_limit = "a".repeat(LOCATE_MAX_QUOTE_BYTES);
        assert!(located(&repr, &at_limit).occurrences.is_empty());
    }

    // ---------------------------------------------------------------------------------------
    // The units are scalars, and the rule folds nothing
    // ---------------------------------------------------------------------------------------

    /// **T10** — an astral scalar is one scalar, four UTF-8 bytes and two UTF-16 code units, and
    /// the offsets are in **scalars**. No committed fixture carries a non-BMP scalar
    /// (`fixtures/engine/make_fixtures.py`, checked), so this is a hand-built representation —
    /// the precedent `docs/history/11-V11-MILESTONES.md` sets for a path the corpus cannot reach.
    #[test]
    fn an_astral_scalar_is_one_scalar_one_offset_and_four_bytes() {
        let repr = one_run("a\u{1f600}b");
        let found = located(&repr, "\u{1f600}");

        assert_eq!("\u{1f600}".len(), 4, "four UTF-8 bytes");
        assert_eq!("\u{1f600}".encode_utf16().count(), 2, "two UTF-16 units");
        assert_eq!(found.quote_scalars, 1, "one scalar, which is the unit here");
        assert_eq!(found.searched.scalars, 3, "three scalars in the run");
        assert_eq!(found.occurrences.len(), 1);
        let p = &found.occurrences[0].parts[0];
        assert_eq!(
            (p.char_start, p.char_end),
            (1, 2),
            "scalar offsets, not byte or UTF-16 offsets"
        );
        parts_reconstruct_the_quote(&repr, &found, "\u{1f600}");
    }

    /// **T11** — a combining mark is two scalars and is not folded into its precomposed form.
    /// `docs/01-CONTRACT.md` §4: no Unicode normalization, anywhere.
    #[test]
    fn a_combining_mark_is_two_scalars_and_is_never_folded() {
        let decomposed = one_run("cafe\u{301}");
        assert_eq!(
            located(&decomposed, "caf\u{e9}").occurrences.len(),
            0,
            "precomposed `é` must not match `e` + U+0301: that would be normalization"
        );
        let found = located(&decomposed, "e\u{301}");
        assert_eq!(found.quote_scalars, 2, "the mark is a scalar of its own");
        assert_eq!(found.occurrences.len(), 1);
        let p = &found.occurrences[0].parts[0];
        assert_eq!((p.char_start, p.char_end), (3, 5));
        parts_reconstruct_the_quote(&decomposed, &found, "e\u{301}");

        // And the other direction: a precomposed run does not answer a decomposed quote.
        let precomposed = one_run("caf\u{e9}");
        assert_eq!(located(&precomposed, "e\u{301}").occurrences.len(), 0);
        assert_eq!(located(&precomposed, "caf\u{e9}").occurrences.len(), 1);
    }

    /// Case is not folded either, and neither is whitespace.
    #[test]
    fn the_rule_folds_neither_case_nor_whitespace() {
        let repr = one_run("Measured  twice");
        assert_eq!(located(&repr, "measured").occurrences.len(), 0);
        assert_eq!(located(&repr, "Measured twice").occurrences.len(), 0);
        assert_eq!(located(&repr, "Measured  twice").occurrences.len(), 1);
    }

    // ---------------------------------------------------------------------------------------
    // The artifact
    // ---------------------------------------------------------------------------------------

    /// The artifact names what produced it, what it searched, and canonicalizes.
    #[test]
    fn the_artifact_carries_its_identity_its_source_and_the_rule_that_ran() {
        let repr = one_run("Measured");
        let profile = Profile::default();
        let found = located(&repr, "Measured");

        assert_eq!(found.identity.artifact_type, "ethos.parser.locations.v0");
        assert_eq!(found.identity.schema_version, LOCATIONS_SCHEMA_VERSION);
        assert_eq!(found.identity.parser_version, profile.parser_version);
        assert_eq!(
            found.identity.profile_sha256,
            profile.profile_sha256().unwrap()
        );
        assert_eq!(
            &found.representation_sha256,
            repr.fingerprint(),
            "the binding that makes an occurrence checkable"
        );
        assert_eq!(&found.source_sha256, &repr.payload().source.sha256);
        assert_eq!(found.locate_rule, profile.locate_rule);

        let bytes = found.to_canonical_bytes().expect("canonicalizes");
        let back: Locations = serde_json::from_slice(&bytes).expect("parses back");
        assert_eq!(
            back, found,
            "the artifact round-trips through its own bytes"
        );
        assert!(
            !String::from_utf8(bytes)
                .unwrap()
                .contains("occurrences_withheld"),
            "nothing withheld, so the field is absent rather than null"
        );
    }

    /// The profile's rule is the one that travels, so a re-pinned profile cannot silently answer
    /// under a rule the artifact does not name.
    #[test]
    fn the_default_profile_runs_the_versioned_rule() {
        assert_eq!(Profile::default().locate_rule, LOCATE_RULE_V1);
    }

    // ---------------------------------------------------------------------------------------
    // The ceiling, measured
    // ---------------------------------------------------------------------------------------

    /// **The cap, on a measured number** (`docs/27-LOCATE-MILESTONES.md` S1, and the figure in
    /// `docs/measurements/locate/README.md`). Deliberate, never in CI — it builds a million
    /// occurrences twice:
    ///
    /// `cargo test -p ethos-parser-core --lib -- --ignored --nocapture the_ceiling`
    ///
    /// Ignored for the reason `accuracy.rs`'s generator is: a gate step that allocates a
    /// hundred-odd megabytes to re-derive a number already in the tree buys nothing the
    /// committed figure does not already say.
    #[test]
    #[ignore = "allocates a million occurrences; run with --ignored to re-measure the cap"]
    fn the_ceiling_is_a_measured_number_and_the_excess_withholds_every_locator() {
        // Each phase is its own scope, so a peak RSS read from outside the process is one
        // measurement's own and not the sum of three held alive to the end of a function.

        // One run whose text is the quote, a million times over: a million occurrences, all in
        // one block, which is the worst shape for both the cost and the size.
        {
            let at_cap = one_run(&"x".repeat(LOCATE_MAX_OCCURRENCES));
            let found = located(&at_cap, "x");
            assert_eq!(found.occurrences.len(), LOCATE_MAX_OCCURRENCES);
            assert!(
                found.occurrences_withheld.is_none(),
                "exactly at the cap is returned, not withheld"
            );
            let bytes = found
                .to_canonical_bytes()
                .expect("canonicalizes at the cap");
            // Integer arithmetic, because `contract_invariants.rs`'s float guard covers this
            // crate's whole source and a measurement's print is no reason to widen it.
            let per = bytes.len() * 100 / LOCATE_MAX_OCCURRENCES;
            println!(
                "locations at {} occurrences: {} bytes ({} KiB, {}.{:02} bytes/occurrence)",
                LOCATE_MAX_OCCURRENCES,
                bytes.len(),
                bytes.len() / 1024,
                per / 100,
                per % 100,
            );
        }

        // One past it: the count travels and **every** locator is withheld, never the excess
        // alone. A truncated list would locate some of a document's occurrences and drop the
        // rest without saying which.
        {
            let past = one_run(&"x".repeat(LOCATE_MAX_OCCURRENCES + 1));
            let found = located(&past, "x");
            assert!(found.occurrences.is_empty(), "all of them, or none");
            let withheld = found.occurrences_withheld.expect("the count travels");
            assert_eq!(withheld.occurrences, LOCATE_MAX_OCCURRENCES as u64 + 1);
            assert_eq!(withheld.limit, LOCATE_MAX_OCCURRENCES as u32);
            let bytes = found.to_canonical_bytes().expect("canonicalizes withheld");
            println!(
                "locations at {} occurrences, withheld: {} bytes",
                LOCATE_MAX_OCCURRENCES + 1,
                bytes.len()
            );
        }

        // **The figure above is the floor.** An occurrence of one part in a two-character node
        // id is the cheapest one there is, and a quote crossing runs costs a part per run. The
        // slope is measured at a tractable count and stated per occurrence, because a million
        // three-part occurrences need three million runs in one block, and then the
        // representation rather than the answer is the thing being measured.
        let crossing = 10_000usize;
        let mut specs: Vec<LineSpec<'_>> = Vec::with_capacity(crossing * 3);
        let mut x = 7200i64;
        for _ in 0..crossing {
            for (text, advance) in [("Yar", 1000i64), ("ro", 660), ("w", 330)] {
                specs.push((text, None, x, 7200, Some(advance), None));
                x += advance;
            }
        }
        let repr = repr_of(&specs);
        let found = located(&repr, "arrow");
        assert_eq!(found.occurrences.len(), crossing, "one per `Yarrow`");
        assert!(
            found.occurrences.iter().all(|o| o.parts.len() == 3),
            "each crosses three runs"
        );
        let bytes = found.to_canonical_bytes().expect("canonicalizes");
        let per = bytes.len() * 100 / crossing;
        println!(
            "locations at {crossing} three-part occurrences: {} bytes ({}.{:02} bytes/occurrence)",
            bytes.len(),
            per / 100,
            per % 100,
        );
    }
}
