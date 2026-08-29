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

//! Open Packaging Conventions: how an `r:id` becomes a part name (v2-S4).
//!
//! # One rule, two formats
//!
//! Every OOXML format hides the same trap, and v2-S3 walked into it first: **the part that lists
//! things does not name their parts.** `xl/workbook.xml` gives each sheet a name and an `r:id`
//! and no path; `ppt/presentation.xml` gives each slide an id and an `r:id` and no path. Only the
//! relationship part says which package part an `r:id` means.
//!
//! The shortcut both formats invite — assume `sheet{n}.xml` or `slide{n}.xml` in list order — is
//! wrong in *ordinary* files. Reordering leaves part names alone, deleting one leaves a gap, and
//! part names are author-chosen in the first place. Every one of those failures attaches the
//! **wrong name to the right content**: a locator that is confidently wrong, which
//! `docs/01-CONTRACT.md` §5.2 exists to prevent and which is strictly worse than an absent one.
//!
//! S3 wrote this rule inside `xlsx.rs`. S4 needed it verbatim, and this crate has already learned
//! what a second copy of a rule costs — three of the nine defects S3's own review found were two
//! copies disagreeing. So it moved here rather than being copied, unchanged.
//!
//! # Two formats, not three, and DOCX is the one that does not need it
//!
//! This heading said *"three formats"* from v2-S4 to v2-S13.3 while the paragraph two lines above
//! it said *"the shortcut **both** formats invite"* — the body was right and the heading was not.
//! Exactly two readers use this module: `xlsx.rs` and `pptx.rs`. **`docx.rs` names `opc` nowhere
//! at all**, and that is the trap's own doing rather than an inconsistency: a workbook and a
//! presentation each have a part that *lists* things without naming their parts, so an `r:id` has
//! to be resolved. A DOCX has no such list. Its main part is `word/document.xml`, reached through
//! the package's own `_rels/.rels`, and there is nothing to resolve per item because there are no
//! items — which is why the trap S3 walked into first could not have been found in S2.

use ethos_parser_core::EngineError;
use quick_xml::events::{BytesStart, Event};

use crate::xml::{attribute_value, check_closed, local_name, new_reader, parse_error};

/// One `<Relationship>` of a rels part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    /// The `Id` other parts refer to it by.
    pub id: String,
    /// The `Type` URI, which says what kind of part is on the other end.
    pub kind: String,
    /// The `Target`, verbatim — still relative, and still possibly external.
    pub target: String,
    /// Whether `TargetMode="External"`, in which case the target is not a package part at all.
    pub external: bool,
}

/// Read a `_rels` part into its relationships.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse or a `<Relationship>` lacks `Id` or
/// `Target`.
pub fn read_relationships(part: &[u8], part_name: &str) -> Result<Vec<Relationship>, EngineError> {
    let mut reader = new_reader(part, part_name)?;
    let mut rels = Vec::new();
    let mut depth: i32 = 0;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, part_name, &e)),
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                depth += 1;
                if local_name(start.name().as_ref()) == b"Relationship" {
                    rels.push(relationship(&start, part_name)?);
                }
            }
            Ok(Event::Empty(start)) => {
                if local_name(start.name().as_ref()) == b"Relationship" {
                    rels.push(relationship(&start, part_name)?);
                }
            }
            Ok(Event::End(_)) => depth -= 1,
            Ok(_) => {}
        }
    }

    check_closed(depth, part_name)?;
    Ok(rels)
}

fn relationship(start: &BytesStart<'_>, part_name: &str) -> Result<Relationship, EngineError> {
    let mut id = None;
    let mut kind = String::new();
    let mut target = None;
    let mut external = false;
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        let value = attribute_value(&attribute, part_name)?;
        match local_name(attribute.key.as_ref()) {
            b"Id" => id = Some(value),
            b"Type" => kind = value,
            b"Target" => target = Some(value),
            b"TargetMode" => external = value == "External",
            _ => {}
        }
    }
    match (id, target) {
        (Some(id), Some(target)) => Ok(Relationship {
            id,
            kind,
            target,
            external,
        }),
        _ => Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: "a `<Relationship>` carries no `Id` or no `Target`, so nothing it is supposed \
                     to bind can be resolved"
                .into(),
        }),
    }
}

/// A `Target` from a rels part, as a package part name, resolved against the owning part's folder.
///
/// `xl/workbook.xml`'s targets resolve against `xl`, `ppt/presentation.xml`'s against `ppt`; a
/// leading `/` makes the target absolute instead.
///
/// `.` and `..` segments are normalised, because real packages use them: measured across 18 real
/// presentations, **2318** relationship targets were of the form `../slideLayouts/slideLayout1.xml`
/// and **99** were package-absolute. A reader that joined them literally would look for
/// `ppt/../slideLayouts/…`, match no entry, and refuse a package that opens everywhere else.
///
/// A target that climbs **out** of the package resolves to the empty string, which matches no
/// entry and is therefore a named part-not-found refusal rather than a path this reader would go
/// looking for.
///
/// **Percent-encoding is deliberately not decoded.** A target with an escaped character simply
/// will not match a central-directory entry, and the result is `read_entry`'s named
/// part-not-found refusal rather than a wrong part — the same trade `xml.rs` states for namespace
/// prefixes, failing toward finding nothing rather than finding the wrong thing.
pub fn resolve_target(target: &str, folder: &str) -> String {
    let joined = match target.strip_prefix('/') {
        Some(absolute) => absolute.to_string(),
        None => format!("{folder}/{target}"),
    };

    let mut segments: Vec<&str> = Vec::new();
    for segment in joined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    // Climbing above the package root. Nothing outside the package is a part, so
                    // this becomes a name no central directory contains.
                    return String::new();
                }
            }
            other => segments.push(other),
        }
    }
    segments.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_resolves_against_the_folder_of_the_part_that_names_it() {
        assert_eq!(
            resolve_target("worksheets/s.xml", "xl"),
            "xl/worksheets/s.xml"
        );
        assert_eq!(
            resolve_target("slides/slide1.xml", "ppt"),
            "ppt/slides/slide1.xml"
        );
        assert_eq!(
            resolve_target("/ppt/slides/slide1.xml", "xl"),
            "ppt/slides/slide1.xml",
            "a leading slash is an absolute part name, and the folder does not apply"
        );
    }

    /// The dominant form in real packages: 2318 of the measured targets climb a directory.
    #[test]
    fn dot_segments_are_normalised_rather_than_joined_literally() {
        assert_eq!(
            resolve_target("../slideLayouts/slideLayout1.xml", "ppt/slides"),
            "ppt/slideLayouts/slideLayout1.xml"
        );
        assert_eq!(
            resolve_target("./slide1.xml", "ppt/slides"),
            "ppt/slides/slide1.xml"
        );
    }

    #[test]
    fn a_target_that_climbs_out_of_the_package_names_no_part() {
        // Not a path this reader goes looking for: it matches no central-directory entry, so the
        // caller gets a named part-not-found refusal.
        assert_eq!(resolve_target("../../../etc/passwd", "ppt"), "");
        assert_eq!(resolve_target("/../secret", "ppt"), "");
    }

    #[test]
    fn relationships_are_read_from_self_closing_elements() {
        // Which is the only form any writer emits, so the `Empty` arm is the one that fires.
        let rels = read_relationships(
            br#"<Relationships xmlns="x">
                  <Relationship Id="rId1" Type="t/slide" Target="slides/slide1.xml"/>
                  <Relationship Id="rId2" Type="t/x" Target="http://e" TargetMode="External"/>
                </Relationships>"#,
            "rels",
        )
        .expect("well-formed");
        assert_eq!(rels.len(), 2);
        assert_eq!(rels[0].id, "rId1");
        assert_eq!(rels[0].target, "slides/slide1.xml");
        assert!(!rels[0].external);
        assert!(rels[1].external, "an external target is not a package part");
    }

    #[test]
    fn a_relationship_missing_its_binding_is_refused() {
        assert!(read_relationships(
            br#"<Relationships><Relationship Type="t"/></Relationships>"#,
            "r"
        )
        .is_err());
    }
}
