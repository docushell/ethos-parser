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
//! a `GeneralRef` event too, with the name `#66`. Through v2 this rule refused those as well —
//! a **named refusal of a valid document**, recorded at S3 and left standing because widening
//! it would change what a shipped DOCX artifact contains and no measurement had asked for that.
//! **0.38.0 made the widening**, as the decision S9 recorded rather than made: every reader that
//! reaches an entity through this module now resolves numeric character references via
//! [`resolve_reference`], in text and in attribute values alike, and the six `text_code_rule`
//! ids moved with the behaviour they name. [`resolve_entity`] keeps the five-only rule as the
//! named-entity core `resolve_reference` falls back to — `&nbsp;` is still a refusal, because an
//! XML parser without a DTD cannot resolve an HTML name, and a name that silently became an
//! empty string would be a character dropped from evidence.

use ethos_parser_core::EngineError;
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

/// One of the five predefined entities, **or a numeric character reference** (v2-S9).
///
/// # Why this is a second function rather than a widening of the first
///
/// `quick-xml` delivers `&#233;` as a `GeneralRef` event named `#233`, so [`resolve_entity`] sees
/// it and refuses it — a **named refusal of a well-formed document**, recorded at v2-S3 and left
/// alone because no measurement then asked for more. XHTML asks. An EPUB content document is
/// hand-authored XML with no DTD, so its authors reach for `&#160;` and `&#8217;` constantly, and
/// a character reference needs no DTD to resolve: XML 1.0 §4.1 defines it as a scalar value
/// written another way.
///
/// It is **not** folded into [`resolve_entity`], and the reason is the profile rather than taste.
/// **Nine shipped profiles name a `text_code_rule`** — the PDF default and one for each of the
/// eight office readers — and **six of those readers reach an XML entity through this function**:
/// `docx`, `xlsx`, `pptx`, `odt`, `ods` and `odp`. `rtf` names a rule and parses no XML; the PDF
/// profile names one and none of this applies to it.
///
/// This sentence said *"six shipped readers name a `text_code_rule`"* from v2-S9 to v2-S13.3. The
/// six was right and its subject was not: it counts the readers this function would change, not
/// the rule ids that exist, and eight rule ids already existed when it was written. Every later
/// "six" in this paragraph — six profiles changed, the other six refuse them, six hash moves —
/// counts the same correct set and stands. A rule id has to move when the behaviour it
/// names moves. Widening the shared function would change what a DOCX reader does with a document
/// it currently refuses, which is a behaviour change in six profiles for a slice that measured one
/// format. So the EPUB reader — whose rule id is new — resolves character references, the other
/// six still refused them until 0.38.0, when the decision S9 recorded was made and the six hash
/// moves were paid: every XML reader in this crate now resolves references through this
/// function, and the per-reader `text_code_rule` ids moved to v2 with the behaviour.
///
/// **Named entities are still the five.** `&nbsp;` is an HTML name, not an XML one, and an XML
/// parser without the DTD cannot resolve it — refusing it is what the specification says to do.
///
/// # Errors
///
/// [`EngineError::Malformed`] for a name that is neither a predefined entity nor a character
/// reference this reader can read, and for a reference that names no Unicode scalar — a
/// surrogate, or a value above `U+10FFFF`.
pub(crate) fn resolve_reference(
    name: &[u8],
    part: &str,
) -> Result<std::borrow::Cow<'static, str>, EngineError> {
    let Some(digits) = name.strip_prefix(b"#") else {
        return Ok(std::borrow::Cow::Borrowed(resolve_entity(name, part)?));
    };
    let text = std::str::from_utf8(digits).unwrap_or("");
    // **The digits, and only digits.** XML 1.0 §4.1 writes a character reference as `&#` followed
    // by decimal digits or `&#x` followed by hexadecimal ones, with no sign — and Rust's integer
    // parsers accept a leading `+`, so `&#+66;` would otherwise resolve to `B` and be spliced into
    // the text as though the document had written it. That is the substitution this function's own
    // refusal exists to prevent, so the shape is checked before the value is parsed.
    let scalar = match text.strip_prefix(['x', 'X']) {
        Some(hex) if !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()) => {
            u32::from_str_radix(hex, 16).ok()
        }
        Some(_) => None,
        None if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) => {
            text.parse::<u32>().ok()
        }
        None => None,
    };
    // A surrogate and a value above the Unicode range both fail here, and both are refused by name
    // rather than replaced: a reference this reader cannot resolve names a character, and
    // substituting a different one would be a guess presented as the document's own text.
    match scalar.and_then(char::from_u32) {
        Some(character) => Ok(std::borrow::Cow::Owned(character.to_string())),
        None => Err(EngineError::Malformed {
            what: part.to_string(),
            detail: format!(
                "`&{};` names no Unicode scalar value. A character reference is how XML writes \
                 a character it cannot spell literally, so one this reader cannot resolve \
                 is a character of unknown identity rather than one it may choose.",
                String::from_utf8_lossy(name)
            ),
        }),
    }
}

/// Resolve entity references in a raw attribute value, under [`resolve_reference`]'s rule.
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
        out.push_str(&resolve_reference(&after.as_bytes()[..end], part)?);
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

/// A reader that **resolves namespace prefixes**, for the four formats that need it.
///
/// The three OOXML readers match element names by suffix, and `local_name`'s own documentation
/// states why that is an acceptable trade there: the match only ever selects content, so the
/// failure mode is finding nothing rather than finding the wrong thing. `odt.rs` re-argues it,
/// because in that reader the same match feeds a **locator ordinal** and a **skip decision** —
/// where the failure mode is a wrong address and a mis-named gap. See `odt::classify`.
///
/// **This said "the one format that needs it" from v2-S5 until v2-S13.5**, when ODT was the only
/// caller. It stopped being true one slice later, at **v2-S6**, and twice more after that:
/// `ods.rs` at v2-S6, `odp.rs` at v2-S7 and `epub.rs` at v2-S9. **Four readers, seven call
/// sites** — the three ODF readers take one each because they share `odt::classify`'s allowlist,
/// and EPUB takes four because `container.xml`, the package document, the encryption declaration
/// and each spine document are all addressed by namespace rather than by suffix. The reason the
/// function exists is unchanged; only the count moved.
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

/// One **unprefixed** attribute, matched on its local name (v2-S9).
///
/// The sibling of [`resolved_attribute`], and a genuinely different question rather than a
/// convenience. XML Namespaces puts an unprefixed *element* in the default namespace and an
/// unprefixed **attribute** in **no namespace at all** — so `href`, `idref`, `media-type` and
/// `full-path` on an EPUB package document resolve to nothing, and asking
/// [`resolved_attribute`] for them under the package namespace finds none of them. That is not a
/// laxer match: it is the exact rule the specification states, and matching them under a namespace
/// they do not have would silently read no attributes.
///
/// A namespace declaration is skipped before the match, for the reason `odt::spaces` skips one:
/// `xmlns:href` has the local name `href`, and treating a binding as an attribute of the element
/// would read a URI where a package path belongs.
pub(crate) fn unprefixed_attribute(
    reader: &quick_xml::NsReader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
    want: &[u8],
    part_name: &str,
) -> Result<Option<String>, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        if attribute.key.as_ref().starts_with(b"xmlns") {
            continue;
        }
        let (resolved, local) = reader.resolver().resolve_attribute(attribute.key);
        if local.as_ref() != want {
            continue;
        }
        if matches!(resolved, quick_xml::name::ResolveResult::Unbound) {
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
    fn attribute_numeric_references_resolve_since_v2() {
        // A sheet named with a character reference is an address, and refusing the
        // whole document over a well-formed name was a wrong-cause refusal.
        assert_eq!(
            unescape_attribute("R&#233;sum&#233; &#x2013; final", "xl/workbook.xml")
                .expect("well-formed"),
            "R\u{e9}sum\u{e9} \u{2013} final"
        );
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
