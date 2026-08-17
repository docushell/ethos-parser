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

//! The XML rules both office readers obey, in one place (v2-S3).
//!
//! # Why this module exists
//!
//! v2-S2 wrote the entity rule inside `docx.rs` because there was one reader. v2-S3 adds a
//! second, and two copies of a rule about *what counts as text* is two places for the answer to
//! drift — the drift being silent, because a reader that resolved one entity set and a reader
//! that resolved another would both produce artifacts that look fine. The rule is extracted
//! **unchanged**, message included, so the DOCX path is byte-identical across the move.
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
