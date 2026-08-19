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

//! The XML rules every office reader obeys, in one place (v2-S3, widened at v2-S4).
//!
//! # Why this module exists
//!
//! v2-S2 wrote the entity rule inside `docx.rs` because there was one reader. v2-S3 adds a
//! second, and two copies of a rule about *what counts as text* is two places for the answer to
//! drift — the drift being silent, because a reader that resolved one entity set and a reader
//! that resolved another would both produce artifacts that look fine. The rule is extracted
//! **unchanged**, message included, so the DOCX path is byte-identical across the move.
//!
//! **v2-S4 widened it from the rule to the plumbing**, for the same reason one slice later: the
//! reader setup, the truncation check, the CDATA and text decoding and the attribute unescape
//! were all about to be copied a third time. Three of the nine defects v2-S3's review found were
//! two copies of one rule disagreeing, so a third copy is the shape of the next one. Nothing in
//! the moved code changed.
//!
//! # The entity rule, and the one thing it refuses that is not an attack
//!
//! Five predefined entities resolve; every other name is a named refusal. An OOXML package has
//! no DTD and no custom entities, so a name arriving here is either malformed or an expansion
//! attack — and either way a name this reader cannot resolve must not become an empty string in
//! the evidence.
//!
//! **Measured at S3:** `quick-xml` 0.41 delivers a *numeric character reference* — `&#66;` — as
//! a `GeneralRef` event too, with the name `#66`. So this rule refuses those as well, even
//! though they are ordinary XML that needs no DTD to resolve. That is a **named refusal of a
//! valid document**, not a silent drop, so it fails in the safe direction; it is recorded here
//! and in `docs/15-V2-MILESTONES.md` S3 rather than quietly widened, because widening it would
//! change what a shipped DOCX artifact contains and no measurement in this slice asked for that.

use engine_core::EngineError;
use quick_xml::Reader;

/// The local name of a possibly-namespaced element or attribute: `w:p` → `p`, `r:id` → `id`.
///
/// By suffix rather than by resolving the namespace binding, and the tradeoff is stated: a
/// package that bound `w:` to something other than WordprocessingML would be read as if it had
/// not. No such package exists in practice, resolving prefixes properly is a real amount of code,
/// and the failure mode is a refusal to find content rather than wrong content.
pub(crate) fn local_name(qualified: &[u8]) -> &[u8] {
    match qualified.iter().rposition(|b| *b == b':') {
        Some(colon) => &qualified[colon + 1..],
        None => qualified,
    }
}

/// One of the five XML predefined entities, or a named refusal.
///
/// # Errors
///
/// [`EngineError::Malformed`] naming `part` for any other entity name.
pub(crate) fn resolve_entity(name: &[u8], part: &str) -> Result<&'static str, EngineError> {
    match name {
        b"amp" => Ok("&"),
        b"lt" => Ok("<"),
        b"gt" => Ok(">"),
        b"quot" => Ok("\""),
        b"apos" => Ok("'"),
        other => Err(EngineError::Malformed {
            what: part.to_string(),
            detail: format!(
                "`&{};` is not one of the five XML predefined entities. This reader resolves \
                 those and refuses the rest rather than dropping a character out of the text it \
                 is supposed to be evidence for.",
                String::from_utf8_lossy(other)
            ),
        }),
    }
}

/// Resolve entity references in a raw attribute value, under [`resolve_entity`]'s rule.
///
/// `quick-xml` hands an attribute back with its entities **unresolved**, and its own unescaping
/// helper is both deprecated in 0.41 and more permissive than the text path. Doing it here means
/// an ampersand in a sheet *name* is governed by the same rule as an ampersand in a cell's text —
/// which matters more, not less, because a dropped character in a name is a **wrong address**
/// rather than merely wrong text.
///
/// # Errors
///
/// [`EngineError::Malformed`] for an unresolvable entity name, or for an `&` with no `;`.
pub(crate) fn unescape_attribute(raw: &str, part: &str) -> Result<String, EngineError> {
    if !raw.contains('&') {
        return Ok(raw.to_string());
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        let Some(end) = after.find(';') else {
            return Err(EngineError::Malformed {
                what: part.to_string(),
                detail: "an attribute value contains `&` with no closing `;`, so it is not \
                         well-formed XML and the text after it cannot be trusted"
                    .into(),
            });
        };
        out.push_str(resolve_entity(&after.as_bytes()[..end], part)?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

pub(crate) fn new_reader<'a>(
    part: &'a [u8],
    part_name: &str,
) -> Result<Reader<&'a [u8]>, EngineError> {
    let text = std::str::from_utf8(part).map_err(|e| EngineError::Malformed {
        what: part_name.to_string(),
        detail: format!("the part is not UTF-8: {e}"),
    })?;
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    Ok(reader)
}

/// A reader that **resolves namespace prefixes**, for the one format that needs it.
///
/// The three OOXML readers match element names by suffix, and `local_name`'s own documentation
/// states why that is an acceptable trade there: the match only ever selects content, so the
/// failure mode is finding nothing rather than finding the wrong thing. `odt.rs` re-argues it,
/// because in that reader the same match feeds a **locator ordinal** and a **skip decision** —
/// where the failure mode is a wrong address and a mis-named gap. See `odt::classify`.
pub(crate) fn new_ns_reader<'a>(
    part: &'a [u8],
    part_name: &str,
) -> Result<quick_xml::NsReader<&'a [u8]>, EngineError> {
    let text = std::str::from_utf8(part).map_err(|e| EngineError::Malformed {
        what: part_name.to_string(),
        detail: format!("the part is not UTF-8: {e}"),
    })?;
    let mut reader = quick_xml::NsReader::from_str(text);
    reader.config_mut().trim_text(false);
    Ok(reader)
}

pub(crate) fn parse_error(
    reader: &Reader<&[u8]>,
    part_name: &str,
    e: &quick_xml::Error,
) -> EngineError {
    parse_error_at(reader.buffer_position(), part_name, e)
}

/// The same refusal, for a reader this module does not own the type of.
pub(crate) fn parse_error_at(at: u64, part_name: &str, e: &quick_xml::Error) -> EngineError {
    EngineError::Malformed {
        what: part_name.to_string(),
        detail: format!("XML will not parse at byte {at}: {e}"),
    }
}

/// The text of a `<![CDATA[…]]>` section, which is character data with no escaping in it.
///
/// **Matched rather than ignored.** An unhandled `CData` event falls through to the catch-all arm
/// and the text inside it disappears with no error — a silent drop, which is the one failure the
/// v2 standing rules name first. CDATA is unusual in an OOXML part and entirely legal in one.
pub(crate) fn cdata_text<'a>(
    cdata: &'a quick_xml::events::BytesCData<'_>,
    part_name: &str,
) -> Result<std::borrow::Cow<'a, str>, EngineError> {
    match std::str::from_utf8(cdata.as_ref()) {
        Ok(text) => Ok(std::borrow::Cow::Borrowed(text)),
        Err(e) => Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("a CDATA section is not UTF-8: {e}"),
        }),
    }
}

pub(crate) fn decode<'a>(
    text: &'a quick_xml::events::BytesText<'_>,
    part_name: &str,
) -> Result<std::borrow::Cow<'a, str>, EngineError> {
    text.decode().map_err(|e| EngineError::Malformed {
        what: part_name.to_string(),
        detail: format!("text will not decode: {e}"),
    })
}

pub(crate) fn attribute_value(
    attribute: &quick_xml::events::attributes::Attribute<'_>,
    part_name: &str,
) -> Result<String, EngineError> {
    let raw =
        std::str::from_utf8(attribute.value.as_ref()).map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("an attribute value is not UTF-8: {e}"),
        })?;
    unescape_attribute(raw, part_name)
}

/// One attribute, matched on its **resolved namespace** and local name (v2-S6, shared at v2-S7).
///
/// Not a suffix match. Every caller here reads either an address component or a declared fact, and
/// `docs/15-V2-MILESTONES.md` S5 states the rule: a suffix match is acceptable where it can only
/// select content, and not where it selects an address. `name`, `value-type` and `formula` are
/// local names other vocabularies use.
///
/// **One copy for both ODF readers.** v2-S6 wrote it inside `ods.rs`; v2-S7 needed the identical
/// question for `draw:name` and moved it here rather than restating it, on the same argument the
/// shared allowlist is built on — two copies of *how an attribute is matched* drift silently, and
/// a reader that started suffix-matching would pick up a foreign `name` and put it in an address
/// with nothing failing anywhere.
pub(crate) fn resolved_attribute(
    reader: &quick_xml::NsReader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    namespace: &[u8],
    want: &[u8],
    part_name: &str,
) -> Result<Option<String>, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        let (resolved, local) = reader.resolver().resolve_attribute(attribute.key);
        if local.as_ref() != want {
            continue;
        }
        if matches!(resolved, quick_xml::name::ResolveResult::Bound(ns) if ns.as_ref() == namespace)
        {
            return Ok(Some(attribute_value(&attribute, part_name)?));
        }
    }
    Ok(None)
}

/// A part that ends with elements still open is truncated, and a truncated part read as far as
/// it went would be a shorter part that still looked whole — `docs/01-CONTRACT.md` §8.
pub(crate) fn check_closed(depth: i32, part_name: &str) -> Result<(), EngineError> {
    if depth != 0 {
        return Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!(
                "the part ends with {depth} element(s) still open; it is truncated, and a part \
                 read as far as it went would be a shorter part that still looked whole"
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_names_drop_the_prefix_and_keep_the_bare_name() {
        assert_eq!(local_name(b"w:p"), b"p");
        assert_eq!(local_name(b"r:id"), b"id");
        assert_eq!(local_name(b"sheetId"), b"sheetId");
        assert_eq!(local_name(b"c"), b"c");
    }

    #[test]
    fn the_five_resolve_and_every_other_name_is_refused() {
        for (name, want) in [
            (&b"amp"[..], "&"),
            (b"lt", "<"),
            (b"gt", ">"),
            (b"quot", "\""),
            (b"apos", "'"),
        ] {
            assert_eq!(resolve_entity(name, "part").expect("predefined"), want);
        }
        for name in [&b"nbsp"[..], b"copy", b"xxe", b"#66"] {
            let error = resolve_entity(name, "xl/sharedStrings.xml").expect_err("refused");
            assert!(
                error.to_string().contains("five XML predefined entities"),
                "the refusal names its reason: {error}"
            );
        }
    }

    #[test]
    fn attribute_entities_follow_the_same_rule_as_text() {
        assert_eq!(
            unescape_attribute("Notes &amp; sources", "xl/workbook.xml").expect("well-formed"),
            "Notes & sources"
        );
        assert_eq!(
            unescape_attribute("plain", "xl/workbook.xml").expect("well-formed"),
            "plain"
        );
        assert_eq!(
            unescape_attribute("&lt;a&gt; &amp; &quot;b&quot;", "p").expect("well-formed"),
            "<a> & \"b\""
        );
        assert!(
            unescape_attribute("half &amp", "p").is_err(),
            "an unterminated reference is malformed, not a literal ampersand"
        );
        assert!(
            unescape_attribute("&nbsp;", "p").is_err(),
            "the attribute path refuses exactly what the text path refuses"
        );
    }
}
