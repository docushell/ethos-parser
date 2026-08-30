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

//! `word/document.xml` → runs, in the part's own document order.
//!
//! # What this reads, and why it is so little
//!
//! `<w:p>` and `<w:r>` and `<w:t>`. That is the whole reader, and `docs/history/15-V2-MILESTONES.md` S2
//! is explicit that it should be: the gate is *a quote resolves to a node*, and one paragraph of
//! runs proves that. Styles, numbering, fields, drawings, comments, track-changes and embedded
//! workbooks are all real OOXML and all absent — **and counted**, because a reader that silently
//! returned the body would let a caller conclude a phrase is absent from a document that contains
//! it (Anydoc's **A14**, declared erasure).
//!
//! # Why XML is a dependency when ZIP was not
//!
//! `zip.rs` explains the container. XML is the opposite call: entities, namespaces, CDATA and
//! encoding declarations are exactly where a hand-rolled reader silently gets *text* wrong, and
//! text is the evidence this engine exists to carry. `quick-xml` is MIT, and its only dependency
//! is `memchr`, which this graph already has.
//!
//! # No layout, anywhere
//!
//! Nothing here computes a position, and nothing could: `docs/history/14-V2-SCOPE.md` §3 forbids a
//! locator that addresses a rendering, and a paragraph does not know what page it falls on until
//! something lays it out. The ordinals below are positions in the XML, which is a fact the file
//! states about itself.

use ethos_parser_core::EngineError;
use quick_xml::events::Event;
use quick_xml::Reader;

/// The part every word-processing package keeps its body in.
pub const MAIN_PART: &str = "word/document.xml";

/// Parts that carry text and that v2-S2 does not read, matched by prefix.
///
/// Prefix rather than exact name because a package numbers them — `word/header1.xml`,
/// `word/header2.xml`. Counting them is the point; naming each one is not.
const UNREAD_TEXT_PART_PREFIXES: [&str; 5] = [
    "word/header",
    "word/footer",
    "word/footnotes",
    "word/endnotes",
    "word/comments",
];

/// One `<w:r>`, with the addresses OOXML states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// 1-based position of the containing `<w:p>` in the part's document order.
    pub paragraph: u32,
    /// 1-based position of this `<w:r>` within that paragraph.
    pub run: u32,
    /// The text, concatenated from this run's `<w:t>` elements, verbatim.
    pub text: String,
    /// Whether any `<w:t>` in this run carried `xml:space="preserve"`.
    pub space_preserved: bool,
}

/// Parts that hold an **embedded asset** and that no slice reads, matched by prefix.
///
/// `word/media/` is where a word-processing package stores a picture and `word/embeddings/` is
/// where it stores an embedded object — a workbook dropped into a report, most often.
///
/// **Prefix, and the package's own layout, is the whole of the identification** (Anydoc's **A4**).
/// No byte is sniffed and no extension is read: OOXML *states* where a package puts these, the
/// same way it states that a header is `word/header1.xml`. An `.png` sitting somewhere else is
/// not counted here, and a media part with no extension at all still is.
const EMBEDDED_PART_PREFIXES: [&str; 2] = ["word/media/", "word/embeddings/"];

/// How many parts of this package hold an embedded asset that this engine does not read.
///
/// Separate from [`unread_text_parts`] because the two answer different questions and their
/// limitation messages make different claims — see
/// `ethos_parser_core::assurance::codes::OFFICE_EMBEDDED_PARTS_NOT_READ`. Nothing here reads, decodes or
/// hashes a byte of the asset: the count is the claim.
pub fn unread_embedded_parts(entry_names: &[String]) -> u32 {
    let matched = entry_names
        .iter()
        .filter(|name| {
            EMBEDDED_PART_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .count();
    crate::declared_len(matched)
}

/// How many parts in this package carry text that this slice does not read.
pub fn unread_text_parts(entry_names: &[String]) -> u32 {
    let matched = entry_names
        .iter()
        .filter(|name| {
            UNREAD_TEXT_PART_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .count();
    crate::declared_len(matched)
}

/// Read the runs of a `word/document.xml`.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse. Malformed XML is refused rather than
/// read as far as it goes: a truncated body would be a shorter document that still looked whole,
/// which is the failure `docs/01-CONTRACT.md` §8 refuses for an unknown content-stream operator.
pub fn read_runs(part: &[u8]) -> Result<Vec<Run>, EngineError> {
    let text = std::str::from_utf8(part).map_err(|e| EngineError::Malformed {
        what: MAIN_PART.into(),
        detail: format!("the part is not UTF-8: {e}"),
    })?;

    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);

    let mut runs: Vec<Run> = Vec::new();
    let mut paragraph: u32 = 0;
    let mut run_in_paragraph: u32 = 0;
    // `Some` while inside a `<w:r>`: the run being built.
    let mut open_run: Option<Run> = None;
    // `true` while inside a `<w:t>`, so text outside one is not collected.
    let mut in_text = false;
    // Element depth, checked at EOF. `quick-xml` reports a *mismatched* close tag on its own but
    // reaching the end of input with elements still open is not an error to it — and a body that
    // stopped halfway is a shorter document that still looks whole, which is exactly what
    // `01-CONTRACT.md` §8 refuses.
    let mut depth: i32 = 0;

    loop {
        match reader.read_event() {
            Err(e) => {
                return Err(EngineError::Malformed {
                    what: MAIN_PART.into(),
                    detail: format!(
                        "XML will not parse at byte {}: {e}",
                        reader.buffer_position()
                    ),
                })
            }
            Ok(Event::Eof) => break,

            Ok(Event::Start(start)) => {
                depth += 1;
                match local_name(start.name().as_ref()) {
                    b"p" => {
                        paragraph += 1;
                        run_in_paragraph = 0;
                    }
                    b"r" => {
                        run_in_paragraph += 1;
                        open_run = Some(Run {
                            paragraph,
                            run: run_in_paragraph,
                            text: String::new(),
                            space_preserved: false,
                        });
                    }
                    b"t" => {
                        in_text = true;
                        if let Some(run) = open_run.as_mut() {
                            if has_preserve(&start)? {
                                run.space_preserved = true;
                            }
                        }
                    }
                    _ => {}
                }
            }

            Ok(Event::End(end)) => {
                depth -= 1;
                match local_name(end.name().as_ref()) {
                    b"t" => in_text = false,
                    b"r" => {
                        if let Some(run) = open_run.take() {
                            // A run with no `<w:t>` — a tab, a break, a drawing anchor — carries no
                            // text to cite. It is not an erasure either: there was never a character
                            // in it. Skipped without a count, deliberately.
                            if !run.text.is_empty() {
                                runs.push(run);
                            }
                        }
                    }
                    _ => {}
                }
            }

            Ok(Event::Text(text)) if in_text => {
                if let Some(run) = open_run.as_mut() {
                    let decoded = text.decode().map_err(|e| EngineError::Malformed {
                        what: MAIN_PART.into(),
                        detail: format!("text will not decode: {e}"),
                    })?;
                    run.text.push_str(decoded.as_ref());
                }
            }
            // **`quick-xml` 0.41 delivers an entity as its own event**, so a reader that only
            // handled `Text` would drop `&amp;` silently and hand back `a  b` for `a &amp; b` —
            // measured, not feared: that is what the first version of this file did, and the
            // fixture's ampersand is in the corpus because of it.
            //
            // The rule itself moved to `xml.rs` at v2-S3, unchanged and message-identical,
            // because the workbook reader needs the same one and two copies of "what counts as
            // text" is two places for the answer to drift.
            Ok(Event::GeneralRef(entity)) if in_text => {
                let resolved = crate::xml::resolve_reference(entity.as_ref(), MAIN_PART)?;
                let resolved = resolved.as_ref();
                if let Some(run) = open_run.as_mut() {
                    run.text.push_str(resolved);
                }
            }
            Ok(_) => {}
        }
    }

    if depth != 0 {
        return Err(EngineError::Malformed {
            what: MAIN_PART.into(),
            detail: format!(
                "the part ends with {depth} element(s) still open; it is truncated, and a body \
                 read as far as it went would be a shorter document that still looked whole"
            ),
        });
    }
    Ok(runs)
}

use crate::xml::local_name;

fn has_preserve(start: &quick_xml::events::BytesStart<'_>) -> Result<bool, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: MAIN_PART.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        if local_name(attribute.key.as_ref()) == b"space" && attribute.value.as_ref() == b"preserve"
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_carry_the_positions_the_part_states() {
        let xml = r#"<?xml version="1.0"?>
            <w:document xmlns:w="x"><w:body>
              <w:p><w:r><w:t>First</w:t></w:r><w:r><w:t xml:space="preserve"> second</w:t></w:r></w:p>
              <w:p><w:r><w:t>Third</w:t></w:r></w:p>
            </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(runs.len(), 3);
        assert_eq!(
            (runs[0].paragraph, runs[0].run, runs[0].text.as_str()),
            (1, 1, "First")
        );
        assert_eq!(
            (runs[1].paragraph, runs[1].run, runs[1].text.as_str()),
            (1, 2, " second")
        );
        assert_eq!(
            (runs[2].paragraph, runs[2].run, runs[2].text.as_str()),
            (2, 1, "Third")
        );
        assert!(!runs[0].space_preserved);
        assert!(
            runs[1].space_preserved,
            "the attribute is read, not guessed"
        );
    }

    #[test]
    fn a_run_with_no_text_is_not_a_node() {
        let xml = r#"<w:document xmlns:w="x"><w:body>
            <w:p><w:r><w:br/></w:r><w:r><w:t>only this</w:t></w:r></w:p>
        </w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "only this");
        assert_eq!(
            runs[0].run, 2,
            "the empty run still counted for the address"
        );
    }

    #[test]
    fn entities_are_decoded_rather_than_carried_as_source() {
        let xml = r#"<w:document xmlns:w="x"><w:body><w:p><w:r>
            <w:t>a &amp; b &lt; c</w:t></w:r></w:p></w:body></w:document>"#;
        let runs = read_runs(xml.as_bytes()).expect("well-formed");
        assert_eq!(
            runs[0].text, "a & b < c",
            "this is why XML is not hand-rolled"
        );
    }

    #[test]
    fn malformed_xml_is_refused_rather_than_read_as_far_as_it_goes() {
        let xml = "<w:document><w:body><w:p><w:r><w:t>truncated";
        assert!(read_runs(xml.as_bytes()).is_err());
    }

    #[test]
    fn unread_text_parts_are_counted_by_prefix() {
        let names: Vec<String> = [
            "word/document.xml",
            "word/header1.xml",
            "word/header2.xml",
            "word/footer1.xml",
            "word/styles.xml",
            "[Content_Types].xml",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // `styles.xml` carries no text a citation could land in, so it is not an erasure.
        assert_eq!(unread_text_parts(&names), 3);
    }
}
