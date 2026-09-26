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

//! Objects a later revision deleted, which `lopdf` loads anyway (review 2026-09-26 N06).
//!
//! An incremental update deletes an object by writing a free entry for its number in the newer
//! cross-reference section. `lopdf` 0.44 records no free entry — its table parser skips `f` rows
//! and its stream decoder skips type 0 — and merges sections newest first with `or_insert`, so an
//! older section's in-use entry fills the gap and the deleted object loads as live: a comment
//! removed before sharing, a superseded content stream, text a redaction by incremental update
//! replaced. [`resurrected`] walks the chain `lopdf` walked and names every number an older
//! section lists in use and a newer one frees; `open_bytes` refuses a document that loaded one. A
//! document of one section cannot have one.
//!
//! Sections are read by `lopdf`'s own grammar wherever its API reaches: a cross-reference stream
//! through [`lopdf::Reader::get_object`] and the decoding `lopdf` applied at load, a table's
//! trailer through the content parser, whose operands are its object grammar. Only a table's rows
//! and an object's `N G obj` header are read here, by the rules `lopdf` reads them with.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use lopdf::{Dictionary, Object};

/// Every object number an older cross-reference section lists in use and a newer one frees.
pub(crate) fn resurrected(bytes: &[u8], doc: &lopdf::Document) -> BTreeSet<u32> {
    // Number to in use, as the newest section naming it says.
    let mut newest: HashMap<u32, bool> = HashMap::new();
    let mut out = BTreeSet::new();
    let mut seen = HashSet::new();
    let mut at = doc.xref_start;
    while seen.insert(at) {
        let Some((mentions, trailer)) = section(bytes, at, doc) else {
            break;
        };
        for (number, in_use) in mentions {
            match newest.get(&number) {
                Some(false) if in_use => {
                    out.insert(number);
                }
                Some(_) => {}
                None => {
                    newest.insert(number, in_use);
                }
            }
        }
        match trailer
            .get(b"Prev")
            .and_then(Object::as_i64)
            .map(usize::try_from)
        {
            Ok(Ok(prev)) => at = prev,
            _ => break,
        }
    }
    out
}

/// One section's mentions, number to in use, and its trailer dictionary.
///
/// A table's `/XRefStm` stream belongs to the table's section (PDF 32000-1 §7.5.8.4). A hybrid
/// file may mark free in the table what it stores in an object stream, so within a section in use
/// wins.
fn section(
    bytes: &[u8],
    at: usize,
    doc: &lopdf::Document,
) -> Option<(HashMap<u32, bool>, Dictionary)> {
    if !bytes.get(at..)?.starts_with(b"xref") {
        return stream(bytes, at, doc);
    }
    let (mut mentions, trailer) = table(bytes, at)?;
    let hidden = trailer
        .get(b"XRefStm")
        .and_then(Object::as_i64)
        .ok()
        .and_then(|s| usize::try_from(s).ok())
        .and_then(|s| stream(bytes, s, doc));
    for (number, in_use) in hidden.map(|(m, _)| m).unwrap_or_default() {
        mention(&mut mentions, number, in_use);
    }
    Some((mentions, trailer))
}

/// A number named within one section. In use wins a number the section names twice.
fn mention(mentions: &mut HashMap<u32, bool>, number: u32, in_use: bool) {
    *mentions.entry(number).or_insert(in_use) |= in_use;
}

/// A table's rows and trailer, by `lopdf`'s rule (`parser/mod.rs` `xref`): after `xref`,
/// subsections of `first count`, rows of `offset generation n|f` ending ` \r`, ` \n` or `\r\n`,
/// then `trailer` and its dictionary.
fn table(bytes: &[u8], at: usize) -> Option<(HashMap<u32, bool>, Dictionary)> {
    let mut i = at + b"xref".len();
    i = eol(bytes, i + usize::from(bytes.get(i) == Some(&b' ')))?;
    let mut mentions = HashMap::new();
    while let Some((first, j)) = uint(bytes, i) {
        let (_, j) = uint(bytes, after(bytes, j, b" ")?)?;
        i = eol(bytes, j + usize::from(bytes.get(j) == Some(&b' ')))?;
        let mut number = first;
        while let Some((in_use, next)) = row(bytes, i) {
            mention(&mut mentions, u32::try_from(number).ok()?, in_use);
            number += 1;
            i = next;
        }
    }
    let rest = bytes.get(i..)?.strip_prefix(b"trailer")?;
    let end = rest
        .windows(9)
        .position(|w| w == b"startxref")
        .map_or(rest.len(), |p| p + 9);
    let operations = lopdf::content::Content::decode(&rest[..end])
        .ok()?
        .operations;
    match operations.first()?.operands.first()? {
        Object::Dictionary(trailer) => Some((mentions, trailer.clone())),
        _ => None,
    }
}

/// One table row: `offset generation n|f`, then ` \r`, ` \n` or `\r\n`.
fn row(bytes: &[u8], i: usize) -> Option<(bool, usize)> {
    let (_, j) = uint(bytes, i)?;
    let (_, j) = uint(bytes, after(bytes, j, b" ")?)?;
    let j = after(bytes, j, b" ")?;
    let in_use = match bytes.get(j)? {
        b'n' => true,
        b'f' => false,
        _ => return None,
    };
    match bytes.get(j + 1..j + 3)? {
        b" \r" | b" \n" | b"\r\n" => Some((in_use, j + 3)),
        _ => None,
    }
}

/// A cross-reference stream's entries and dictionary, read and decoded as `lopdf` read it at load
/// (`parser_aux.rs` `decode_xref_stream_with_limit`): type 0 is free, types 1 and 2 are in use,
/// and any other type goes unrecorded, as `lopdf` leaves it.
fn stream(
    bytes: &[u8],
    at: usize,
    doc: &lopdf::Document,
) -> Option<(HashMap<u32, bool>, Dictionary)> {
    let id = object_id_at(bytes, at)?;
    let mut reader = lopdf::Reader {
        buffer: bytes,
        document: lopdf::Document::new(),
        encryption_state: None,
        raw_objects: BTreeMap::new(),
        password: None,
        strict: false,
        max_decompressed_size: None,
    };
    // `lopdf`'s own table, so an indirect `/Length` resolves as it did at load, and this object.
    reader.document.reference_table = doc.reference_table.clone();
    reader.document.reference_table.entries.insert(
        id.0,
        lopdf::xref::XrefEntry::Normal {
            offset: u32::try_from(at).ok()?,
            generation: id.1,
        },
    );
    let object = reader.get_object(id, &mut HashSet::new()).ok()?;
    let stream = object.as_stream().ok()?;
    let data = if stream.is_compressed() {
        stream.decompressed_content().ok()?
    } else {
        stream.content.clone()
    };
    let ints = |key: &[u8]| -> Option<Vec<i64>> {
        let array = stream.dict.get(key).ok()?.as_array().ok()?;
        array.iter().map(|o| o.as_i64().ok()).collect()
    };
    let size = stream.dict.get(b"Size").and_then(Object::as_i64).ok()?;
    let widths = ints(b"W")?
        .into_iter()
        .take(3)
        .map(|w| usize::try_from(w).ok())
        .collect::<Option<Vec<usize>>>()?;
    let [kind_width, second, third] = widths[..] else {
        return None;
    };
    let width = kind_width + second + third;
    if width == 0 {
        return None;
    }
    let mut rows = data.chunks_exact(width);
    let mut mentions = HashMap::new();
    for pair in ints(b"Index")
        .unwrap_or_else(|| vec![0, size])
        .chunks_exact(2)
    {
        for number in pair[0]..pair[0].saturating_add(pair[1]) {
            let Some(row) = rows.next() else {
                return Some((mentions, stream.dict.clone()));
            };
            // Big-endian, accumulated as `lopdf` accumulates it; a zero width means type 1.
            let kind = match kind_width {
                0 => 1,
                w => row[..w].iter().fold(0u32, |v, &b| (v << 8) + u32::from(b)),
            };
            if let (0..=2, Ok(number)) = (kind, u32::try_from(number)) {
                mention(&mut mentions, number, kind != 0);
            }
        }
    }
    Some((mentions, stream.dict.clone()))
}

/// The `N G` of the indirect object at `at`, by `lopdf`'s rule: `space`, then `N`, `space`, `G`,
/// `space`, `obj`.
fn object_id_at(bytes: &[u8], at: usize) -> Option<lopdf::ObjectId> {
    let (number, i) = uint(bytes, space(bytes, at))?;
    let (generation, i) = uint(bytes, space(bytes, i))?;
    after(bytes, space(bytes, i), b"obj")?;
    Some((u32::try_from(number).ok()?, u16::try_from(generation).ok()?))
}

/// `lopdf`'s `space`: white space and `%` comments, a comment running to its end of line.
fn space(bytes: &[u8], mut i: usize) -> usize {
    loop {
        match bytes.get(i) {
            Some(b' ' | b'\t' | b'\n' | b'\r' | b'\0' | b'\x0c') => i += 1,
            Some(b'%') => match bytes[i..].iter().position(|c| matches!(c, b'\r' | b'\n')) {
                Some(end) => i += end,
                None => return i,
            },
            _ => return i,
        }
    }
}

/// The leading ASCII digits at `i` as a number, and where they end.
fn uint(bytes: &[u8], i: usize) -> Option<(u64, usize)> {
    let digits = bytes
        .get(i..)?
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    let value = std::str::from_utf8(&bytes[i..i + digits])
        .ok()?
        .parse()
        .ok()?;
    Some((value, i + digits))
}

/// Where `token` ends, if it starts at `i`.
fn after(bytes: &[u8], i: usize, token: &[u8]) -> Option<usize> {
    bytes
        .get(i..)?
        .starts_with(token)
        .then_some(i + token.len())
}

/// `lopdf`'s `eol`: `\r\n`, `\n` or `\r`.
fn eol(bytes: &[u8], i: usize) -> Option<usize> {
    let rest = bytes.get(i..)?;
    if rest.starts_with(b"\r\n") {
        Some(i + 2)
    } else if matches!(rest.first(), Some(b'\n' | b'\r')) {
        Some(i + 1)
    } else {
        None
    }
}
