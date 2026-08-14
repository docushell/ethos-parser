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

//! Form fields and annotations, as nodes of their own kind (v1-S4).
//!
//! # The one rule everything here serves: an annotation's text is not the page's text
//!
//! A form field's `/V` and an annotation's `/Contents` live in **dictionaries**, not in any
//! content stream. Nothing draws them; the page's operators never mention them. So they cannot be
//! text runs, and this module exists because without it they are simply absent from the record.
//!
//! LiteParse flattens widgets into the text layer, which is an undeclared document mutation
//! (checklist L13): downstream, a reviewer's private comment becomes indistinguishable from the
//! page's own words, and a citation "grounded" in the document is really grounded in a sticky
//! note. Two nodes of different kinds cost a consumer one match arm and buy that distinction back.
//!
//! **The converse is equally true and is not a loophole.** When a form is *flattened at save
//! time*, its values really were painted onto the page, and those glyphs are runs — read by the
//! interpreter like any other text. This module never touches them. The rule is about where this
//! engine reads a value *from*, not a claim that flattened ink is not ink.
//!
//! # Appearance streams are not page content
//!
//! A widget's `/AP` `/N` is a form XObject full of `Tj`s that draw the value into its box. It is
//! tempting to run the interpreter over it. This does not, for the same reason: the value is read
//! from `/V`, where the document states it, rather than from a rendering of it. Nothing here
//! rasterizes, and nothing OCRs.
//!
//! # Walked from the page, not from the form
//!
//! The obvious shape — recurse `/AcroForm` `/Fields` and emit a node per terminal field — has a
//! hole in it: a field's node needs a page, and a field dictionary does not name one. Its
//! *widget* does, by sitting in that page's `/Annots`.
//!
//! So the walk runs the other way. Each page's `/Annots` is read in order; a `/Widget` resolves
//! **up** its `/Parent` chain to gather the field's name, type and value, and everything else
//! becomes an annotation. Three things fall out of that rather than needing separate machinery:
//!
//! | Falls out | Why |
//! | --- | --- |
//! | Every node has a real page | it was found *on* one |
//! | One node per widget, not a field plus a clone | the widget *is* the field's visual |
//! | Orphans are detectable | a field whose widget no page carries is one no page walk reached |
//!
//! # No repair, ever
//!
//! LiteParse "repairs orphaned widgets in memory". If a widget's `/Parent` will not resolve, this
//! emits the widget with what it declares about *itself* and **declares the unresolved link**
//! (`form-field-parent-unresolved`). Source bytes are never touched and `/Kids` is never rewritten
//! — a repair nobody recorded is a document the reader was handed instead of the one they have.

use engine_core::{
    quantize, AnnotationAttributes, AnnotationRect, FieldValue, FormFieldAttributes, QRect,
    QUANTUM_PER_POINT,
};
use lopdf::{Dictionary, Object, ObjectId};

/// How deep a `/Parent` chain may be followed before this gives up on it.
///
/// Sixteen. A real field hierarchy is two or three deep — `form.address.line1`. The bound exists
/// because `/Parent` can cycle in a malformed file, and the answer to a cycle is a declared
/// unresolved parent rather than a hang.
pub const MAX_PARENT_DEPTH: usize = 16;

/// `/Ff` bits this profile names, by bit position (1-based, as PDF 32000-1 numbers them).
///
/// Only the bits that change how a **value** should be read. Presentation bits are left in
/// `unrecognized_flag_bits` rather than given names that imply this engine acts on them.
const FIELD_FLAGS: &[(u32, &str)] = &[
    (1, "read_only"),
    (2, "required"),
    (3, "no_export"),
    (13, "multiline"),
    (14, "password"),
    (15, "no_toggle_to_off"),
    (16, "radio"),
    (17, "pushbutton"),
    (18, "combo"),
    (22, "multi_select"),
    (26, "do_not_spell_check"),
];

/// `/F` annotation flags, by bit position (PDF 32000-1 §12.5.3).
///
/// `hidden` and `no_view` are **named and reported, never acted on**. An annotation the document
/// asks a viewer not to draw is still content the document contains, and dropping it would be an
/// edit this engine made silently (checklist O21).
const ANNOT_FLAGS: &[(u32, &str)] = &[
    (1, "invisible"),
    (2, "hidden"),
    (3, "print"),
    (4, "no_zoom"),
    (5, "no_rotate"),
    (6, "no_view"),
    (7, "read_only"),
    (8, "locked"),
    (9, "toggle_no_view"),
    (10, "locked_contents"),
];

/// One annotation or form field found on a page, before it becomes a node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageObject {
    /// The object this was read from.
    pub id: ObjectId,
    /// Its `/Rect`, or a typed reason there is none.
    pub rect: AnnotationRect,
    /// The text this object carries: a field's value, or an annotation's `/Contents`.
    pub text: String,
    /// Which of the two this is.
    pub detail: PageObjectDetail,
}

/// A page object's kind-specific facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageObjectDetail {
    /// A widget: the visual of an interactive form field.
    Field(FormFieldAttributes),
    /// Any other annotation.
    Annotation(AnnotationAttributes),
}

/// What one page's `/Annots` produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageObjects {
    /// The objects, in the order the page lists them.
    ///
    /// **The document's own order**, not sorted by position. `/Annots` is an array the author
    /// wrote, and reordering it would be this engine imposing an order on annotations that came
    /// from a different source than the one governing the text.
    ///
    /// v1-S5 did not change this. Its rule orders `TextRun`s by page geometry; widgets are not
    /// runs and are not interleaved into that order by `/Rect`, because the two orders answer to
    /// two authorities — the page's whitespace and the author's array. They stay a documented
    /// second sequence, after all of a page's text and monotone within itself.
    pub objects: Vec<PageObject>,
    /// Widgets whose `/Parent` chain could not be resolved.
    pub unresolved_parents: u32,
}

/// Read one page's annotations and widgets.
///
/// Never fails: an annotation this profile cannot make sense of yields a node carrying what it
/// *does* declare, because refusing a page over one malformed comment would lose the whole page's
/// markup to fix nothing.
pub fn read_page_objects(doc: &lopdf::Document, page: &Dictionary) -> PageObjects {
    let mut out = PageObjects::default();

    let Some(annots) = resolve(doc, page.get(b"Annots").ok()) else {
        return out;
    };
    let Object::Array(items) = annots else {
        return out;
    };

    for item in items {
        // Only a direct reference gives an object id, which is the address this node is published
        // under. An inline annotation dictionary has no id to cite, so it is skipped rather than
        // given a fabricated one.
        let Object::Reference(id) = item else {
            continue;
        };
        let Ok(dict) = doc.get_dictionary(*id) else {
            continue;
        };

        let rect = rect_of(doc, dict);
        let subtype = name_at(doc, dict, b"Subtype").unwrap_or_else(|| "Unknown".into());

        if subtype == "Widget" {
            let (attrs, resolved) = field_attributes(doc, dict, *id);
            if !resolved {
                out.unresolved_parents += 1;
            }
            out.objects.push(PageObject {
                id: *id,
                rect,
                text: attrs.value.as_text(),
                detail: PageObjectDetail::Field(attrs),
            });
        } else {
            out.objects.push(PageObject {
                id: *id,
                rect,
                // `/Contents` is the annotation's own text: the comment, the note, the callout.
                // Absent means the annotation carries none, which is ordinary for a `/Link`.
                text: text_at(doc, dict, b"Contents").unwrap_or_default(),
                detail: PageObjectDetail::Annotation(AnnotationAttributes {
                    subtype,
                    name: text_at(doc, dict, b"NM"),
                    title: text_at(doc, dict, b"T"),
                    flags: named_bits(int_at(doc, dict, b"F").unwrap_or(0), ANNOT_FLAGS),
                    unrecognized_flag_bits: unnamed_bits(
                        int_at(doc, dict, b"F").unwrap_or(0),
                        ANNOT_FLAGS,
                    ),
                }),
            });
        }
    }

    out
}

/// Gather a widget's field facts by walking **up** its `/Parent` chain.
///
/// Returns the attributes and whether the chain resolved cleanly. `/T`, `/FT`, `/V` and `/Ff` are
/// all inheritable in PDF, so the nearest declaration wins and the search continues upward only
/// for what is still missing.
fn field_attributes(
    doc: &lopdf::Document,
    widget: &Dictionary,
    widget_id: ObjectId,
) -> (FormFieldAttributes, bool) {
    let mut names: Vec<String> = Vec::new();
    let mut field_type: Option<String> = None;
    let mut value: Option<FieldValue> = None;
    let mut flag_bits: i64 = 0;
    let mut resolved = true;

    let mut seen: Vec<ObjectId> = vec![widget_id];
    let mut current: Option<&Dictionary> = Some(widget);
    let mut depth = 0usize;

    while let Some(dict) = current {
        if let Some(t) = text_at(doc, dict, b"T") {
            names.push(t);
        }
        if field_type.is_none() {
            field_type = name_at(doc, dict, b"FT");
        }
        if value.is_none() {
            value = field_value(doc, dict);
        }
        if flag_bits == 0 {
            flag_bits = int_at(doc, dict, b"Ff").unwrap_or(0);
        }

        depth += 1;
        if depth > MAX_PARENT_DEPTH {
            // A chain this deep is a cycle in practice. Declared, not followed further.
            resolved = false;
            break;
        }

        current = match dict.get(b"Parent").ok() {
            None => None,
            Some(Object::Reference(pid)) => {
                if seen.contains(pid) {
                    // `/Parent` points at an ancestor. Not repaired and not walked.
                    resolved = false;
                    break;
                }
                seen.push(*pid);
                match doc.get_dictionary(*pid) {
                    Ok(d) => Some(d),
                    Err(_) => {
                        // The orphan case: a widget naming a parent the file does not contain.
                        // The widget still becomes a node with whatever it declares itself.
                        resolved = false;
                        break;
                    }
                }
            }
            Some(Object::Dictionary(d)) => Some(d),
            Some(_) => {
                resolved = false;
                break;
            }
        };
    }

    // Root first, which is how PDF 32000-1 §12.7.3.2 spells a fully-qualified name. The walk
    // collected them leaf-first.
    names.reverse();

    (
        FormFieldAttributes {
            field_name: (!names.is_empty()).then(|| names.join(".")),
            field_type,
            value: value.unwrap_or(FieldValue::Absent),
            flags: named_bits(flag_bits, FIELD_FLAGS),
            unrecognized_flag_bits: unnamed_bits(flag_bits, FIELD_FLAGS),
        },
        resolved,
    )
}

/// `/V`, in the shape the document wrote it.
///
/// **`/DV` is deliberately not a fallback.** A default value is what the field would hold if it
/// were reset, not what it holds — reporting one as the value would put a string in the record
/// that the document does not currently assert.
fn field_value(doc: &lopdf::Document, dict: &Dictionary) -> Option<FieldValue> {
    let v = resolve(doc, dict.get(b"V").ok())?;
    Some(match v {
        Object::String(bytes, _) => FieldValue::Text(decode_text(bytes)),
        Object::Name(n) => FieldValue::Name(String::from_utf8_lossy(n).into_owned()),
        Object::Integer(i) => FieldValue::Integer(*i),
        Object::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                match item {
                    Object::String(b, _) => out.push(decode_text(b)),
                    Object::Name(n) => out.push(String::from_utf8_lossy(n).into_owned()),
                    // A mixed array is a shape this profile does not express. Declared rather
                    // than partially reported, because a half-read selection is worse than a
                    // named refusal.
                    _ => return Some(FieldValue::Unsupported),
                }
            }
            FieldValue::Choice(out)
        }
        // A stream, a dictionary, a real number: legal PDF this profile has no shape for.
        _ => FieldValue::Unsupported,
    })
}

/// Whether the catalog declares an XFA packet.
///
/// Detected so it can be **declared**, never parsed (checklist L15).
pub fn has_xfa(doc: &lopdf::Document) -> bool {
    doc.catalog()
        .ok()
        .and_then(|c| c.get(b"AcroForm").ok().cloned())
        .and_then(|a| match a {
            Object::Reference(id) => doc.get_dictionary(id).ok().cloned(),
            Object::Dictionary(d) => Some(d),
            _ => None,
        })
        .is_some_and(|form| form.get(b"XFA").is_ok())
}

/// `/Rect`, quantized, or a typed reason there is none.
///
/// Note what this does **not** do when `/Rect` is missing or unusable: invent a page-sized box.
/// A rectangle nobody wrote is a claim about where something sits, and a wrong one is worse than
/// an absent one because it looks like an answer.
fn rect_of(doc: &lopdf::Document, dict: &Dictionary) -> AnnotationRect {
    let Some(Object::Array(items)) = resolve(doc, dict.get(b"Rect").ok()) else {
        return AnnotationRect::NotDeclared;
    };
    if items.len() != 4 {
        return AnnotationRect::Malformed;
    }
    let mut q = [0i64; 4];
    for (i, item) in items.iter().enumerate() {
        let Some(f) = as_f64(item) else {
            return AnnotationRect::Malformed;
        };
        match quantize(f, QUANTUM_PER_POINT) {
            Ok(v) => q[i] = v,
            Err(_) => return AnnotationRect::Malformed,
        }
    }
    // PDF writes `/Rect` as two opposite corners in either order; normalizing is reading it, not
    // changing it. Note this is **user space** — the caller flips it into the artifact's
    // top-left system, exactly as a glyph origin is flipped.
    match QRect::new(
        q[0].min(q[2]),
        q[1].min(q[3]),
        q[0].max(q[2]),
        q[1].max(q[3]),
    ) {
        Ok(r) => AnnotationRect::Declared(r),
        Err(_) => AnnotationRect::Malformed,
    }
}

/// Bit positions set in `bits` that this profile names, in ascending order.
fn named_bits(bits: i64, table: &[(u32, &str)]) -> Vec<String> {
    table
        .iter()
        .filter(|(pos, _)| is_set(bits, *pos))
        .map(|(_, name)| (*name).to_string())
        .collect()
}

/// Bit positions set in `bits` that this profile has no name for.
fn unnamed_bits(bits: i64, table: &[(u32, &str)]) -> Vec<u32> {
    (1..=32)
        .filter(|pos| is_set(bits, *pos) && !table.iter().any(|(p, _)| p == pos))
        .collect()
}

fn is_set(bits: i64, position: u32) -> bool {
    (1..=32).contains(&position) && (bits >> (position - 1)) & 1 == 1
}

fn as_f64(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(f64::from(*r)),
        _ => None,
    }
}

/// Follow one level of indirection, if there is one.
fn resolve<'a>(doc: &'a lopdf::Document, o: Option<&'a Object>) -> Option<&'a Object> {
    match o? {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

fn name_at(doc: &lopdf::Document, dict: &Dictionary, key: &[u8]) -> Option<String> {
    match resolve(doc, dict.get(key).ok())? {
        Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
        _ => None,
    }
}

fn int_at(doc: &lopdf::Document, dict: &Dictionary, key: &[u8]) -> Option<i64> {
    match resolve(doc, dict.get(key).ok())? {
        Object::Integer(i) => Some(*i),
        _ => None,
    }
}

fn text_at(doc: &lopdf::Document, dict: &Dictionary, key: &[u8]) -> Option<String> {
    match resolve(doc, dict.get(key).ok())? {
        Object::String(bytes, _) => Some(decode_text(bytes)),
        _ => None,
    }
}

/// Decode a PDF text string.
///
/// Two encodings are legal here (PDF 32000-1 §7.9.2.2): UTF-16BE behind a byte-order mark, and
/// PDFDocEncoding otherwise. The ASCII range of PDFDocEncoding is Latin-1, which is what this
/// reads — a byte outside it becomes `U+FFFD` **only here**, in a dictionary string, and never in
/// page text, where an undecodable code refuses the run instead.
///
/// The asymmetry is deliberate. A run's text is evidence a citation is checked against, so a
/// substituted character there would be a character the document does not contain sitting in the
/// evidence. A field's title or an annotation's author name is a label, and losing the whole
/// annotation over one unmappable byte in its author's name would delete content to protect a
/// string nobody cites.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    bytes.iter().map(|b| char::from(*b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_and_unnamed_flag_bits_partition_the_word() {
        // Bit 1 (read_only) and bit 20, which this profile does not name.
        let bits = 0b1000_0000_0000_0000_0001i64;
        assert_eq!(named_bits(bits, FIELD_FLAGS), vec!["read_only"]);
        assert_eq!(unnamed_bits(bits, FIELD_FLAGS), vec![20]);
    }

    #[test]
    fn an_unnamed_flag_is_kept_rather_than_dropped() {
        // A bit nobody named is still something the document said. Dropping it would make an
        // unread flag indistinguishable from an unset one.
        let bits = 1 << 27;
        assert!(named_bits(bits, ANNOT_FLAGS).is_empty());
        assert_eq!(unnamed_bits(bits, ANNOT_FLAGS), vec![28]);
    }

    #[test]
    fn hidden_is_reported_and_never_acted_on() {
        // `/F` bit 2. The flag is data; nothing in this module filters on it.
        assert!(named_bits(0b10, ANNOT_FLAGS).contains(&"hidden".to_string()));
        let src = include_str!("forms.rs");
        let code = src
            .split("\n#[cfg(test)]\nmod tests {")
            .next()
            .expect("split yields a first part");
        assert!(
            code.contains("pub fn read_page_objects"),
            "the guard did not reach the walk"
        );
        for dropping in ["if hidden", "!hidden", "retain(", "filter(|a|"] {
            assert!(
                !code.contains(dropping),
                "`{dropping}` looks like the walk filtering annotations. Flags are DATA: an \
                 annotation the document asked a viewer not to draw is still content it contains, \
                 and deleting it is an edit this engine made on the reader's behalf"
            );
        }
    }

    #[test]
    fn a_utf16_text_string_decodes_through_its_byte_order_mark() {
        let mut bytes = vec![0xFE, 0xFF];
        for u in "héllo".encode_utf16() {
            bytes.extend_from_slice(&u.to_be_bytes());
        }
        assert_eq!(decode_text(&bytes), "héllo");
    }

    #[test]
    fn a_pdfdoc_text_string_decodes_as_latin1() {
        assert_eq!(decode_text(b"Total due"), "Total due");
        assert_eq!(decode_text(&[0x41, 0xE9]), "Aé");
    }

    #[test]
    fn a_field_value_never_becomes_a_boolean() {
        // `/Off` and `/Yes` are the common spellings and not the only legal ones. Converting them
        // would be this engine's reading rather than the document's text.
        let v = FieldValue::Name("Off".into());
        assert_eq!(v.as_text(), "Off");
        let s = format!("{v:?}");
        assert!(!s.contains("false") && !s.contains("true"), "{s}");
    }

    #[test]
    fn an_absent_value_is_not_an_empty_string() {
        // A blank form is a real state, and it is not the same as a field filled in with nothing.
        assert_eq!(FieldValue::Absent.as_text(), "");
        assert_ne!(FieldValue::Absent, FieldValue::Text(String::new()));
    }
}
