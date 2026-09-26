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

//! The outline a PDF's catalog declares — `docs/29-OUTLINES-SCOPE.md`, rule `outlines-v1`.
//!
//! **Consume, never synthesise**, the rule `crate::structure` opens with. A `/First`/`/Next`
//! chain is a hierarchy the author wrote down, so this reads it and reports it; nothing here
//! infers a heading, a boundary or a section end. `P14` refuses role inferred from *presentation*
//! and does not bear on a declaration.
//!
//! # What this refuses, and why each is a refusal rather than a repair
//!
//! - **A cycling `/First` or `/Next`**, and nesting past [`MAX_DEPTH`]. Both are fail-closed, on
//!   `crate::structure`'s grounds for a cycling `/K`: walking forever is a denial of service and
//!   walking "far enough" silently truncates a declared structure.
//! - **Renumbering the depth.** Where a document skips a level, so does this. A depth that closed
//!   a gap would be a position in this reader's walk rather than the tree's own statement.
//! - **Inferring where a section ends.** An entry declares where one *begins*.
//!   `docs/17-D1-SCOPE.md`:130 refuses a boundary inferred from a bookmark, and this repository's
//!   own corpus contains **two entries whose target page precedes their predecessor's**, where an
//!   inferred end would run backwards.
//! - **Guessing a title byte.** `0x80`–`0x9F` is where PDFDocEncoding, Latin-1 and Windows-1252
//!   disagree, and `0xA0` and `0xAD` are where PDFDocEncoding and Latin-1 do; this engine vendors
//!   no table for them, so such a title is absent and counted (`crate::forms::decode_text_strict`).
//! - **Dropping an entry.** An unresolved destination, an undecodable title and a page outside the
//!   budget are each *absent and counted* on a record that is still emitted.

use std::collections::BTreeSet;

use ethos_parser_core::{DerivationClass, EngineError, OutlineRecord};
use lopdf::{Dictionary, Object, ObjectId};

/// The nesting this reader will follow before refusing, matching `crate::structure::MAX_DEPTH`.
///
/// The deepest tree in this repository's corpus declares **5** levels, so this is two orders of
/// magnitude of headroom over anything measured rather than a number tuned to a document.
pub const MAX_DEPTH: usize = 64;

/// What one document's outline read produced.
pub(crate) struct OutlineRead {
    /// The entries, in `/First`/`/Next` order.
    pub(crate) records: Vec<OutlineRecord>,
    /// Entries whose `/Title` holds a byte this engine will not decode.
    pub(crate) undecodable_titles: u32,
    /// Entries whose destination named no page of this document.
    pub(crate) unresolved_destinations: u32,
    /// Whether the catalog named an `/Outlines` at all.
    ///
    /// Distinct from `records.is_empty()`: a catalog may declare an outline dictionary with no
    /// `/First`, and "the document declares none" and "the document declares an empty one" are
    /// different statements.
    pub(crate) declared: bool,
}

/// Read the document's declared outline.
///
/// # Errors
///
/// [`EngineError::Malformed`] when the chain cycles or nests past [`MAX_DEPTH`]. Fail closed: an
/// outline this reader cannot follow to the end is not one it may report half of.
pub(crate) fn read(doc: &crate::document::Document) -> Result<OutlineRead, EngineError> {
    let mut out = OutlineRead {
        records: Vec::new(),
        undecodable_titles: 0,
        unresolved_destinations: 0,
        declared: false,
    };
    let inner = doc.inner();
    let Ok(catalog) = inner.catalog() else {
        // No catalog is a malformed document, and extraction is where that is reported.
        return Ok(out);
    };
    let Ok(Object::Reference(root_id)) = catalog.get(b"Outlines") else {
        return Ok(out);
    };
    let Ok(root) = inner.get_dictionary(*root_id) else {
        return Ok(out);
    };
    out.declared = true;

    // ObjectId -> 1-based page number, from the one map `document.rs` already keeps. `lopdf`
    // keys pages from 1 and that origin is preserved rather than re-derived, so there is no
    // place here for an off-by-one to enter.
    let pages: std::collections::HashMap<ObjectId, u32> =
        doc.pages().iter().map(|(n, id)| (*id, *n)).collect();

    let Ok(Object::Reference(first)) = root.get(b"First") else {
        return Ok(out);
    };
    let mut seen = BTreeSet::new();
    walk(inner, *first, 1, &mut seen, &pages, &mut out)?;
    Ok(out)
}

fn walk(
    doc: &lopdf::Document,
    first: ObjectId,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    pages: &std::collections::HashMap<ObjectId, u32>,
    out: &mut OutlineRead,
) -> Result<(), EngineError> {
    if depth > MAX_DEPTH {
        return Err(EngineError::Malformed {
            what: "outline".into(),
            detail: format!(
                "`/First` nests past {MAX_DEPTH} levels. Refused rather than truncated: an \
                 outline this reader cannot follow to the end is not one it may report half of."
            ),
        });
    }
    let mut cur = Some(first);
    while let Some(id) = cur {
        if !seen.insert(id) {
            return Err(EngineError::Malformed {
                what: "outline".into(),
                detail: format!(
                    "the outline revisits object {} {} through `/First` or `/Next`, so the chain \
                     is a cycle rather than a tree. Refused by name: walking it forever is a \
                     denial of service and walking it partly would report a declared structure \
                     this reader could not follow.",
                    id.0, id.1
                ),
            });
        }
        let Ok(item) = doc.get_dictionary(id) else {
            return Err(EngineError::Malformed {
                what: "outline".into(),
                detail: format!(
                    "the outline names object {} {}, which does not resolve to a dictionary. An \
                     entry this reader cannot open is refused rather than skipped: skipping it \
                     would drop whatever it declared and everything nested under it, silently.",
                    id.0, id.1
                ),
            });
        };

        // A `/Title` may be an indirect object (§7.3.10); it is read where it points.
        let title_object = item
            .get(b"Title")
            .and_then(|o| doc.dereference(o).map(|(_, o)| o));
        let title = match title_object {
            Ok(Object::String(bytes, _)) => match crate::forms::decode_text_strict(bytes) {
                Some(t) => Some(t),
                None => {
                    out.undecodable_titles = out.undecodable_titles.saturating_add(1);
                    None
                }
            },
            // A `/Title` is required by §12.3.3. Its absence is this reader's to report, not to
            // fill: an entry with no title still declares a place in the hierarchy.
            _ => {
                out.undecodable_titles = out.undecodable_titles.saturating_add(1);
                None
            }
        };

        let page = dest_page(doc, item, pages);
        if page.is_none() {
            out.unresolved_destinations = out.unresolved_destinations.saturating_add(1);
        }

        out.records.push(OutlineRecord {
            derivation: DerivationClass::Extracted,
            title,
            depth: u32::try_from(depth).unwrap_or(u32::MAX),
            object: id.0,
            generation: id.1,
            page,
        });

        if let Ok(Object::Reference(child)) = item.get(b"First") {
            walk(doc, *child, depth + 1, seen, pages, out)?;
        }
        cur = match item.get(b"Next") {
            Ok(Object::Reference(n)) => Some(*n),
            _ => None,
        };
    }
    Ok(())
}

/// The 1-based page an entry points at, per §12.3.2, or `None`.
///
/// Three shapes are resolved and a fourth is deliberately not. `/Dest` may be an explicit array,
/// or a name or byte string into the `/Names` `/Dests` name tree or the catalog's older `/Dests`
/// dictionary; `/A` may be a `/GoTo` action carrying the same in `/D`. **A first array element
/// that is an integer is a remote destination's page number, in another file** — it is not this
/// document's page and is not coerced into one.
fn dest_page(
    doc: &lopdf::Document,
    item: &Dictionary,
    pages: &std::collections::HashMap<ObjectId, u32>,
) -> Option<u32> {
    let target = match item.get(b"Dest") {
        Ok(d) => d.clone(),
        Err(_) => {
            let action = match item.get(b"A").ok()? {
                Object::Reference(id) => doc.get_dictionary(*id).ok()?.clone(),
                Object::Dictionary(d) => d.clone(),
                _ => return None,
            };
            if action.get(b"S").ok()?.as_name().ok()? != b"GoTo" {
                return None;
            }
            action.get(b"D").ok()?.clone()
        }
    };
    let array = match target {
        Object::Array(items) => items,
        Object::Name(ref n) => named_dest(doc, n)?,
        Object::String(ref s, _) => named_dest(doc, s)?,
        Object::Reference(id) => match doc.get_object(id).ok()? {
            Object::Array(items) => items.clone(),
            _ => return None,
        },
        _ => return None,
    };
    match array.first()? {
        Object::Reference(id) => pages.get(id).copied(),
        _ => None,
    }
}

/// Resolve a named destination through `/Names` `/Dests` or the catalog's older `/Dests`.
fn named_dest(doc: &lopdf::Document, name: &[u8]) -> Option<Vec<Object>> {
    let catalog = doc.catalog().ok()?;
    let unwrap = |o: &Object| -> Option<Vec<Object>> {
        let o = match o {
            Object::Reference(id) => doc.get_object(*id).ok()?,
            other => other,
        };
        match o {
            Object::Array(items) => Some(items.clone()),
            // A destination dictionary carries the array under `/D` (§12.3.2.3).
            Object::Dictionary(d) => match d.get(b"D").ok()? {
                Object::Array(items) => Some(items.clone()),
                Object::Reference(id) => match doc.get_object(*id).ok()? {
                    Object::Array(items) => Some(items.clone()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    };
    if let Ok(dests) = catalog.get(b"Dests") {
        let d = match dests {
            Object::Reference(id) => doc.get_dictionary(*id).ok()?,
            Object::Dictionary(d) => d,
            _ => return None,
        };
        if let Ok(hit) = d.get(name) {
            return unwrap(hit);
        }
    }
    // The `/Names` name tree. Bounded the same way the walk is: a malformed `/Kids` graph is a
    // cycle risk, and this returns nothing rather than looping.
    let names = match catalog.get(b"Names").ok()? {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?,
        Object::Dictionary(d) => d,
        _ => return None,
    };
    let mut stack = vec![match names.get(b"Dests").ok()? {
        Object::Reference(id) => doc.get_dictionary(*id).ok()?.clone(),
        Object::Dictionary(d) => d.clone(),
        _ => return None,
    }];
    let mut visited = 0usize;
    while let Some(node) = stack.pop() {
        visited += 1;
        if visited > 10_000 {
            return None;
        }
        if let Ok(Object::Array(pairs)) = node.get(b"Names") {
            for pair in pairs.chunks(2) {
                if let (Some(Object::String(k, _)), Some(v)) = (pair.first(), pair.get(1)) {
                    if k.as_slice() == name {
                        return unwrap(v);
                    }
                }
            }
        }
        if let Ok(Object::Array(kids)) = node.get(b"Kids") {
            for kid in kids {
                if let Object::Reference(id) = kid {
                    if let Ok(d) = doc.get_dictionary(*id) {
                        stack.push(d.clone());
                    }
                }
            }
        }
    }
    None
}
