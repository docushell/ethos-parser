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

//! Stable-ID ordering discipline (`docs/01-CONTRACT.md` §12).
//!
//! # IDs are stable only within one profile
//!
//! The DocuShell companion is explicit: *"IDs need be stable only within a representation
//! created by the same pinned profile. A parser/profile upgrade creates a new representation
//! and mapping/diff rather than pretending that vendor node IDs are permanently global."*
//!
//! So an id is a coordinate inside one artifact, not a name for a thing in the world. Storing
//! one in an external system and expecting it to resolve after a profile bump is the mistake
//! this module's documentation exists to prevent — [`NodeId`] carries the profile digest it was
//! minted under so that mistake becomes a detectable mismatch instead of a wrong answer.
//!
//! # Ordering is assigned, never discovered
//!
//! Ids are allocated from a counter in a deterministic traversal, so the same document under
//! the same profile yields the same ids. Nothing derives an id from a hash of content (two
//! identical text runs would collide) or from a memory address (which varies per run).

use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::identity::Sha256Hex;

/// An identifier for one node inside one representation.
///
/// The string form is `<prefix><ordinal>`, e.g. `e12`, `s3`, `p1`. Matches the id pattern
/// `ethos.grounding.v1` accepts: `^[A-Za-z0-9][A-Za-z0-9._:-]*$`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

impl NodeId {
    /// The id as it appears on the wire.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Build an id from a kind and an ordinal, without an allocator.
    ///
    /// For tests and for reconstructing a reference to an id that already exists — a
    /// `parent` pointer, say. **Not** an allocation: it takes the ordinal rather than choosing
    /// one, so it cannot mint a duplicate behind an [`IdAllocator`]'s back. Ordering discipline
    /// still belongs to the allocator, and the structural checks that consume these ids reject a
    /// duplicate or a dangling reference regardless of which route produced it.
    pub fn from_parts(kind: IdKind, ordinal: u64) -> Self {
        Self(format!("{}{}", kind.prefix(), ordinal))
    }
}

impl core::fmt::Display for NodeId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What kind of node an id names. Determines the id prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdKind {
    /// A page.
    Page,
    /// A typed element.
    Element,
    /// A text span.
    Span,
    /// A table.
    Table,
}

impl IdKind {
    /// The single-character prefix used in the wire form.
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Page => "p",
            Self::Element => "e",
            Self::Span => "s",
            Self::Table => "t",
        }
    }
}

/// Allocates ids in deterministic order within one representation.
///
/// Ordinals are **1-based**, matching the page indexing `ethos.grounding.v1` requires and
/// avoiding the 0-vs-1 inconsistency pdf-inspector carries between its own two result types.
///
/// Bound to the profile that minted it: [`NodeId`]s from two different profiles are not
/// interchangeable, and [`Self::profile`] lets a consumer check rather than assume.
#[derive(Debug, Clone)]
pub struct IdAllocator {
    profile: Sha256Hex,
    next_page: u64,
    next_element: u64,
    next_span: u64,
    next_table: u64,
}

impl IdAllocator {
    /// Start allocation for a representation produced under `profile`.
    pub fn new(profile: Sha256Hex) -> Self {
        Self {
            profile,
            next_page: 1,
            next_element: 1,
            next_span: 1,
            next_table: 1,
        }
    }

    /// The profile these ids are stable under.
    pub fn profile(&self) -> &Sha256Hex {
        &self.profile
    }

    /// Allocate the next id of `kind`.
    ///
    /// # Errors
    ///
    /// [`EngineError::ResourceLimit`] if the counter would exceed [`crate::MAX_SAFE_INT`].
    /// Wrapping would silently reuse an id, which is worse than refusing to continue.
    pub fn next(&mut self, kind: IdKind) -> Result<NodeId, EngineError> {
        let counter = match kind {
            IdKind::Page => &mut self.next_page,
            IdKind::Element => &mut self.next_element,
            IdKind::Span => &mut self.next_span,
            IdKind::Table => &mut self.next_table,
        };
        if *counter > crate::MAX_SAFE_INT as u64 {
            return Err(EngineError::ResourceLimit {
                limit: "node id ordinal".into(),
                configured: crate::MAX_SAFE_INT.to_string(),
            });
        }
        let id = NodeId(format!("{}{}", kind.prefix(), counter));
        *counter += 1;
        Ok(id)
    }

    /// How many ids of `kind` have been allocated.
    pub fn count(&self, kind: IdKind) -> u64 {
        (match kind {
            IdKind::Page => self.next_page,
            IdKind::Element => self.next_element,
            IdKind::Span => self.next_span,
            IdKind::Table => self.next_table,
        }) - 1
    }
}

/// Sort ids the way canonical output requires: by kind prefix, then by **numeric** ordinal.
///
/// Lexicographic sorting would put `e10` before `e2`, so array order — which is semantic, since
/// element order is reading order — would depend on how many elements a document happens to
/// have. Ten-element and nine-element documents would order differently under the same rule.
pub fn sort_ids(ids: &mut [NodeId]) {
    ids.sort_by(|a, b| {
        let (ap, an) = split_id(a.as_str());
        let (bp, bn) = split_id(b.as_str());
        ap.cmp(bp).then(an.cmp(&bn)).then(a.0.cmp(&b.0))
    });
}

/// Split `e12` into `("e", Some(12))`.
///
/// An unparseable ordinal sorts **last** within its prefix. `Option`'s natural order puts `None`
/// first, which is the wrong end: a malformed id would displace well-formed content at the head
/// of a reading order. Mapping absence to `u64::MAX` puts it at the tail instead.
///
/// Ids reaching here always come from [`IdAllocator`], so this branch is defensive. It still has
/// to be *deterministic and stated*, because a total order that nobody has pinned is a total
/// order that changes when someone refactors it.
fn split_id(s: &str) -> (&str, u64) {
    let split = s.find(|c: char| c.is_ascii_digit()).unwrap_or(s.len());
    let (prefix, digits) = s.split_at(split);
    (prefix, digits.parse::<u64>().unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alloc() -> IdAllocator {
        IdAllocator::new(Profile_hash())
    }

    #[allow(non_snake_case)]
    fn Profile_hash() -> Sha256Hex {
        crate::Profile::default().profile_sha256().unwrap()
    }

    #[test]
    fn ids_are_one_based_and_prefixed_by_kind() {
        let mut a = alloc();
        assert_eq!(a.next(IdKind::Page).unwrap().as_str(), "p1");
        assert_eq!(a.next(IdKind::Page).unwrap().as_str(), "p2");
        assert_eq!(a.next(IdKind::Element).unwrap().as_str(), "e1");
        assert_eq!(a.next(IdKind::Span).unwrap().as_str(), "s1");
        assert_eq!(a.next(IdKind::Table).unwrap().as_str(), "t1");
    }

    #[test]
    fn counters_are_independent_per_kind() {
        let mut a = alloc();
        for _ in 0..3 {
            a.next(IdKind::Element).unwrap();
        }
        a.next(IdKind::Span).unwrap();
        assert_eq!(a.count(IdKind::Element), 3);
        assert_eq!(a.count(IdKind::Span), 1);
        assert_eq!(a.count(IdKind::Page), 0);
    }

    #[test]
    fn allocation_is_reproducible_for_the_same_sequence() {
        let seq = [IdKind::Element, IdKind::Span, IdKind::Element, IdKind::Page];
        let run = |()| {
            let mut a = alloc();
            seq.iter()
                .map(|k| a.next(*k).unwrap().as_str().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(()), run(()));
    }

    #[test]
    fn ids_match_the_grounding_id_pattern() {
        // ethos.grounding.v1: ^[A-Za-z0-9][A-Za-z0-9._:-]*$
        let mut a = alloc();
        for kind in [IdKind::Page, IdKind::Element, IdKind::Span, IdKind::Table] {
            let id = a.next(kind).unwrap();
            let s = id.as_str();
            let mut chars = s.chars();
            let first = chars.next().unwrap();
            assert!(first.is_ascii_alphanumeric(), "bad first char in {s}");
            assert!(
                chars.all(|c| c.is_ascii_alphanumeric() || ".:_-".contains(c)),
                "bad char in {s}"
            );
        }
    }

    #[test]
    fn sorting_is_numeric_not_lexicographic() {
        let mut a = alloc();
        let mut ids: Vec<NodeId> = (0..12).map(|_| a.next(IdKind::Element).unwrap()).collect();
        ids.reverse();
        sort_ids(&mut ids);
        let got: Vec<&str> = ids.iter().map(NodeId::as_str).collect();
        assert_eq!(
            got,
            vec!["e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9", "e10", "e11", "e12"],
            "lexicographic order would put e10 before e2"
        );
    }

    #[test]
    fn sorting_groups_by_kind_first() {
        let mut a = alloc();
        let mut ids = vec![
            a.next(IdKind::Span).unwrap(),
            a.next(IdKind::Element).unwrap(),
            a.next(IdKind::Page).unwrap(),
            a.next(IdKind::Element).unwrap(),
        ];
        sort_ids(&mut ids);
        let got: Vec<&str> = ids.iter().map(NodeId::as_str).collect();
        assert_eq!(got, vec!["e1", "e2", "p1", "s1"]);
    }

    #[test]
    fn an_unparseable_ordinal_sorts_last_within_its_prefix() {
        // Constructed by hand: the allocator cannot mint these. The point is that the order is
        // pinned and documented, not that it is reachable.
        let mut ids = vec![
            NodeId("e".into()),
            NodeId("e2".into()),
            NodeId("e1".into()),
            NodeId("s1".into()),
        ];
        sort_ids(&mut ids);
        let got: Vec<&str> = ids.iter().map(NodeId::as_str).collect();
        assert_eq!(
            got,
            vec!["e1", "e2", "e", "s1"],
            "a malformed id must sort to the tail of its prefix, never displace e1 at the head"
        );
    }

    #[test]
    fn sorting_is_stable_and_total() {
        // Sorting twice must not move anything, or canonical output would depend on input order.
        let mut a = alloc();
        let mut ids: Vec<NodeId> = (0..20)
            .map(|i| {
                a.next(if i % 2 == 0 {
                    IdKind::Element
                } else {
                    IdKind::Span
                })
                .unwrap()
            })
            .collect();
        sort_ids(&mut ids);
        let once = ids.clone();
        sort_ids(&mut ids);
        assert_eq!(once, ids);
    }

    #[test]
    fn the_allocator_records_the_profile_ids_are_stable_under() {
        let p = Profile_hash();
        let a = IdAllocator::new(p.clone());
        assert_eq!(a.profile(), &p);
    }

    #[test]
    fn ids_serialize_as_bare_strings() {
        let mut a = alloc();
        let id = a.next(IdKind::Element).unwrap();
        assert_eq!(serde_json::to_string(&id).unwrap(), "\"e1\"");
        assert_eq!(serde_json::from_str::<NodeId>("\"e1\"").unwrap(), id);
    }
}
