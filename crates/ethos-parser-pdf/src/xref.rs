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

//! One bounded cross-reference repair: 19-byte entries padded to the specified 20.
//!
//! # The malformation, exactly
//!
//! PDF 32000-1 §7.5.4 requires every cross-reference entry to be **exactly 20 bytes**:
//! ten digits, a space, five digits, a space, one type character, then a two-byte end-of-line.
//! The affected documents write a one-byte `\n` instead of `" \n"`, so each entry is 19 bytes:
//!
//! ```text
//! spec      "0000000015 00000 n \n"     20 bytes
//! observed  "0000000015 00000 n\n"      19 bytes
//! ```
//!
//! `lopdf` requires the specified stride and refuses the whole document. PDFium repairs it. On
//! the Ethos conformance corpus this is roughly one valid document in twenty-six —
//! `synthetic/table-regular-grid` is the known case, and v0 refused it as a declared limitation
//! with the repair-or-refuse call deferred to v0.1 (`docs/03-V0-SCOPE.md` §4).
//!
//! **v0.1 decides: repair, bounded and declared.** `docs/01-CONTRACT.md` §12 is the written
//! decision; this module is its implementation.
//!
//! # Why padding is safe here, and what makes it unsafe elsewhere
//!
//! Padding **grows the file**, so every byte after the first padded entry moves. That is fatal in
//! general: a cross-reference entry is a byte offset, and moving the byte an offset points at
//! turns a refusal into something far worse — a document that parses into the wrong objects and
//! produces a well-formed artifact that is silently wrong.
//!
//! It is safe only when nothing an offset points at can move. [`repair_xref`] therefore refuses
//! unless **all** of these hold, and each closes a specific way the repair could corrupt:
//!
//! | Precondition | Without it |
//! | --- | --- |
//! | Exactly one `xref` keyword table | A second section's offsets shift under the first section's padding |
//! | The trailer declares no `/Prev` | An incremental-update chain points into bytes that moved |
//! | `startxref` names the table's own start | The table is not where the document says, so this is not the class being repaired |
//! | **Every** entry matches the 19-byte class | A mixed-stride table is worse repaired than refused |
//! | Every in-use offset precedes the table | An object *after* the table would be shifted by the padding |
//!
//! The last is the load-bearing one. With every object before the table and the table last, the
//! only bytes that move are the table's own tail, the trailer, `startxref` and `%%EOF` — none of
//! which is addressed by offset. They are found by scanning back from the end of the file.
//!
//! # It is a fallback, never a first move
//!
//! [`crate::document::Document::open_bytes`] parses normally first and only reaches this after a
//! `Malformed` failure. A well-formed document never comes near this code, and an encrypted or
//! wrong-magic file is refused before it — repairing either was never on the table.
//!
//! # It is visible three ways
//!
//! A repaired open is never silent. The profile carries [`XREF_REPAIR_V1`], so
//! `profile_sha256` differs from a build that does not repair; every artifact from a repaired
//! open declares [`crate::limitations::XREF_ENTRY_PADDED`]; and the count of padded entries is
//! in the limitation's detail.

/// The repair's identity, as it appears in [`ethos_parser_core::Profile::xref_repair`].
///
/// Stable string, versioned: a future repair with different behaviour takes a new id rather than
/// changing this one's meaning, so artifacts stay correctly comparable.
pub const XREF_REPAIR_V1: &str = "pad-19-to-20-v1";

/// A repair that was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    /// The repaired bytes.
    pub bytes: Vec<u8>,
    /// How many entries were padded, for the limitation's detail.
    pub entries_padded: u32,
}

/// Why a candidate was not repaired.
///
/// Returned rather than logged: `open_bytes` reports the *original* parse error to the caller,
/// and this exists so the tests can assert which precondition fired rather than only that the
/// document was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// No `xref` keyword table, or more than one.
    NotExactlyOneTable,
    /// The trailer chains to an earlier cross-reference section.
    HasPrevChain,
    /// `startxref` does not name the table's start offset.
    StartxrefDisagrees,
    /// At least one entry is not the 19-byte class — including a table already conforming.
    NotTheNineteenByteClass,
    /// An in-use object offset points at or past the table, so padding would move it.
    ObjectFollowsTable,
    /// The table is structurally unreadable (no subsection header, no trailer).
    Unreadable,
}

impl Refusal {
    /// A short, stable reason. Test-only: `open_bytes` reports the original parse error, so
    /// nothing in the shipped path renders a refusal — the names exist so a failing test can say
    /// which precondition fired rather than only that the document was refused.
    #[cfg(test)]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotExactlyOneTable => "not-exactly-one-xref-table",
            Self::HasPrevChain => "trailer-declares-prev",
            Self::StartxrefDisagrees => "startxref-disagrees",
            Self::NotTheNineteenByteClass => "not-the-19-byte-class",
            Self::ObjectFollowsTable => "object-follows-table",
            Self::Unreadable => "xref-table-unreadable",
        }
    }
}

/// Attempt the one repair this engine performs.
///
/// # Errors
///
/// Never — a refusal is a [`Refusal`], not an [`EngineError`]. The caller already holds the real
/// parse error and reports that one; inventing a second error here would tell a caller the
/// document failed for a reason it did not.
pub fn repair_xref(bytes: &[u8]) -> Result<Repair, Refusal> {
    // Exactly one cross-reference table. `find_all` rather than `find`, because a second section
    // is precisely the case where padding one shifts the other.
    let tables = find_xref_tables(bytes);
    if tables.len() != 1 {
        return Err(Refusal::NotExactlyOneTable);
    }
    let table_at = tables[0];

    // `startxref` must name this table. If it names something else, whatever is wrong with this
    // document is not the malformation being repaired.
    let declared = startxref_offset(bytes).ok_or(Refusal::Unreadable)?;
    if declared != table_at {
        return Err(Refusal::StartxrefDisagrees);
    }

    let trailer_at = find_last(bytes, b"trailer").ok_or(Refusal::Unreadable)?;
    if trailer_at < table_at {
        return Err(Refusal::Unreadable);
    }
    // An incremental-update chain reaches back into bytes this repair would move.
    if bytes[trailer_at..].windows(5).any(|w| w == b"/Prev") {
        return Err(Refusal::HasPrevChain);
    }

    // The entry region: after the `xref` keyword and its subsection header, up to `trailer`.
    let after_keyword = table_at + b"xref".len();
    let region = &bytes[after_keyword..trailer_at];

    let mut entries_padded = 0u32;
    let mut offsets: Vec<usize> = Vec::new();
    let mut out = Vec::with_capacity(bytes.len() + 32);
    out.extend_from_slice(&bytes[..after_keyword]);

    let mut saw_subsection_header = false;
    for line in split_keep_newline(region) {
        let trimmed = trim_ascii(line);
        if trimmed.is_empty() {
            out.extend_from_slice(line);
            continue;
        }
        if let Some(entry) = parse_entry(line) {
            // The whole point: the line must be the 19-byte class. A conforming 20-byte entry
            // returns None here, so a table that is already correct refuses rather than being
            // "repaired" into 21-byte entries.
            if entry.in_use {
                offsets.push(entry.offset);
            }
            out.extend_from_slice(&line[..line.len() - 1]);
            out.extend_from_slice(b" \n");
            entries_padded += 1;
            continue;
        }
        if is_subsection_header(trimmed) {
            saw_subsection_header = true;
            out.extend_from_slice(line);
            continue;
        }
        // Anything else inside the entry region means this is not the shape being repaired.
        return Err(Refusal::NotTheNineteenByteClass);
    }

    if !saw_subsection_header || entries_padded == 0 {
        return Err(Refusal::Unreadable);
    }

    // The load-bearing check. Every in-use object must live before the table, so padding the
    // table cannot move a byte any offset points at.
    if offsets.iter().any(|&o| o >= table_at) {
        return Err(Refusal::ObjectFollowsTable);
    }

    out.extend_from_slice(&bytes[trailer_at..]);
    Ok(Repair {
        bytes: out,
        entries_padded,
    })
}

/// The detail string for the limitation a repaired open declares.
pub fn repaired_detail(entries_padded: u32) -> String {
    format!(
        "The cross-reference table carried {entries_padded} entries of 19 bytes where PDF \
         32000-1 §7.5.4 requires exactly 20 (`0000000015 00000 n \\n`, with the trailing space). \
         Each was padded to the specified width and the document was then parsed normally. The \
         repair is offset-preserving by construction: it is attempted only when the table is the \
         last structure in the file, `startxref` names it, no `/Prev` chain exists, and every \
         in-use object offset precedes the table — so no byte any offset points at can move. \
         Nothing else was altered. The profile records `{XREF_REPAIR_V1}`, so an artifact from a \
         repairing build is correctly non-comparable with one from a build that refuses."
    )
}

/// A parsed 19-byte cross-reference entry.
struct Entry {
    offset: usize,
    in_use: bool,
}

/// Parse one line as a **19-byte** entry, or `None`.
///
/// Exact by construction: ten digits, space, five digits, space, `n` or `f`, newline. A
/// conforming 20-byte entry has a space before its newline and does not match, which is what
/// keeps this from "repairing" a correct table.
fn parse_entry(line: &[u8]) -> Option<Entry> {
    if line.len() != 19 || line[18] != b'\n' {
        return None;
    }
    let digits = |r: &[u8]| r.iter().all(|c| c.is_ascii_digit());
    if !digits(&line[0..10]) || line[10] != b' ' || !digits(&line[11..16]) || line[16] != b' ' {
        return None;
    }
    let in_use = match line[17] {
        b'n' => true,
        b'f' => false,
        _ => return None,
    };
    let offset = std::str::from_utf8(&line[0..10]).ok()?.parse().ok()?;
    Some(Entry { offset, in_use })
}

/// `<first> <count>`, the subsection header PDF 32000-1 §7.5.4 puts before a run of entries.
fn is_subsection_header(t: &[u8]) -> bool {
    let mut parts = t.split(|c| *c == b' ').filter(|p| !p.is_empty());
    let (Some(a), Some(b), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !a.is_empty()
        && !b.is_empty()
        && a.iter().all(|c| c.is_ascii_digit())
        && b.iter().all(|c| c.is_ascii_digit())
}

/// Offsets of every `xref` keyword that starts a table.
///
/// Anchored to a line start so the `xref` inside `startxref` is not counted — that substring is
/// why a naive search finds one table too many on every document in existence.
fn find_xref_tables(bytes: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = find_from(bytes, b"xref", i) {
        let at_line_start = rel == 0 || bytes[rel - 1] == b'\n' || bytes[rel - 1] == b'\r';
        let followed_by_eol = matches!(bytes.get(rel + 4), Some(b'\n' | b'\r'));
        if at_line_start && followed_by_eol {
            out.push(rel);
        }
        i = rel + 4;
    }
    out
}

/// The integer `startxref` declares.
fn startxref_offset(bytes: &[u8]) -> Option<usize> {
    let at = find_last(bytes, b"startxref")?;
    let rest = &bytes[at + b"startxref".len()..];
    let digits: Vec<u8> = rest
        .iter()
        .skip_while(|c| c.is_ascii_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .copied()
        .collect();
    std::str::from_utf8(&digits).ok()?.parse().ok()
}

fn find_from(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from >= haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + from)
}

fn find_last(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

/// Split on `\n`, keeping the newline on each line so byte lengths stay exact.
fn split_keep_newline(mut region: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    while !region.is_empty() {
        match region.iter().position(|c| *c == b'\n') {
            Some(at) => {
                out.push(&region[..=at]);
                region = &region[at + 1..];
            }
            None => {
                out.push(region);
                break;
            }
        }
    }
    out
}

fn trim_ascii(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|c| !c.is_ascii_whitespace())
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|c| !c.is_ascii_whitespace())
        .map_or(start, |p| p + 1);
    &s[start..end]
}

/// Map a refusal onto the error a caller sees.
///
/// Unused today — `open_bytes` reports the original parse error, deliberately — and kept as the
/// one place that would name these if a future caller wanted them.
#[cfg(test)]
fn _refusal_error(r: Refusal) -> ethos_parser_core::EngineError {
    ethos_parser_core::EngineError::Malformed {
        what: "pdf xref".into(),
        detail: format!("not the repairable class: {}", r.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact shape of the corpus fixture, built here so the unit tests do not need the
    /// Ethos tree.
    fn nineteen_byte_doc() -> Vec<u8> {
        let objs: Vec<Vec<u8>> = vec![
            b"1 0 obj\n<< /Type /Catalog >>\nendobj\n".to_vec(),
            b"2 0 obj\n<< /Type /Pages >>\nendobj\n".to_vec(),
        ];

        let mut body = b"%PDF-1.7\n".to_vec();
        let mut offsets = Vec::new();
        for o in &objs {
            offsets.push(body.len());
            body.extend_from_slice(o);
        }
        let table_at = body.len();
        body.extend_from_slice(b"xref\n0 3\n");
        body.extend_from_slice(b"0000000000 65535 f\n");
        for off in &offsets {
            body.extend_from_slice(format!("{off:010} 00000 n\n").as_bytes());
        }
        body.extend_from_slice(b"trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n");
        body.extend_from_slice(format!("{table_at}\n%%EOF\n").as_bytes());
        body
    }

    #[test]
    fn the_nineteen_byte_class_is_padded_to_twenty() {
        let src = nineteen_byte_doc();
        let r = repair_xref(&src).expect("the canonical case repairs");
        assert_eq!(r.entries_padded, 3, "one free entry and two in-use");
        assert_eq!(r.bytes.len(), src.len() + 3, "one byte added per entry");

        // Every entry in the output is now the specified width.
        let table = r.bytes.windows(4).position(|w| w == b"xref").unwrap();
        let after = &r.bytes[table..];
        for line in after.split(|c| *c == b'\n').skip(2).take(3) {
            assert_eq!(line.len(), 19, "18 bytes plus the added space: {line:?}");
            assert!(line.ends_with(b" "), "the trailing space is the repair");
        }
    }

    #[test]
    fn a_conforming_table_is_not_touched() {
        // The repair must be a no-op-by-refusal on a correct document, or a well-formed file
        // could be "repaired" into 21-byte entries.
        let src = String::from_utf8(nineteen_byte_doc())
            .unwrap()
            .replace(" n\n", " n \n")
            .replace(" f\n", " f \n")
            .into_bytes();
        assert_eq!(
            repair_xref(&src),
            Err(Refusal::NotTheNineteenByteClass),
            "a 20-byte table is not the repairable class"
        );
    }

    #[test]
    fn an_object_after_the_table_refuses() {
        // The hostile case the whole precondition set exists for: padding would move a byte an
        // offset points at, turning a refusal into a silently wrong parse.
        let src = nineteen_byte_doc();
        let table_at = find_xref_tables(&src)[0];
        let mut hostile = src.clone();
        // Point object 1 past the table.
        let victim = format!("{:010} 00000 n\n", table_at + 10);
        let first = find_from(&hostile, b"0000000009 00000 n\n", 0)
            .expect("the generated doc puts object 1 at offset 9");
        hostile.splice(first..first + 19, victim.bytes());
        assert_eq!(repair_xref(&hostile), Err(Refusal::ObjectFollowsTable));
    }

    #[test]
    fn an_incremental_update_chain_refuses() {
        let src = String::from_utf8(nineteen_byte_doc())
            .unwrap()
            .replace("/Size 3", "/Size 3 /Prev 9")
            .into_bytes();
        assert_eq!(repair_xref(&src), Err(Refusal::HasPrevChain));
    }

    #[test]
    fn a_second_xref_table_refuses() {
        let mut src = nineteen_byte_doc();
        let at = find_xref_tables(&src)[0];
        src.splice(at..at, b"xref\n0 1\n0000000000 65535 f\n".iter().copied());
        assert_eq!(repair_xref(&src), Err(Refusal::NotExactlyOneTable));
    }

    #[test]
    fn a_startxref_naming_something_else_refuses() {
        let src = String::from_utf8(nineteen_byte_doc())
            .unwrap()
            .replace("startxref\n", "startxref\n1")
            .into_bytes();
        assert_eq!(repair_xref(&src), Err(Refusal::StartxrefDisagrees));
    }

    #[test]
    fn a_mixed_stride_table_refuses() {
        // Half repaired, half not, is worse than either. One conforming entry is enough to
        // disqualify the table.
        let src = String::from_utf8(nineteen_byte_doc())
            .unwrap()
            .replacen(" f\n", " f \n", 1)
            .into_bytes();
        assert_eq!(repair_xref(&src), Err(Refusal::NotTheNineteenByteClass));
    }

    #[test]
    fn garbage_is_not_a_repair_candidate() {
        assert!(repair_xref(b"").is_err());
        assert!(repair_xref(b"%PDF-1.7\nnothing here\n").is_err());
        assert_eq!(
            repair_xref(b"%PDF-1.7\nxref\n").unwrap_err(),
            Refusal::Unreadable
        );
    }

    #[test]
    fn the_startxref_substring_is_not_mistaken_for_a_table() {
        // `startxref` contains `xref`. A search that counted it would see two tables on every
        // document and refuse everything — the failure would look like the repair "not working".
        let src = nineteen_byte_doc();
        assert_eq!(
            find_xref_tables(&src).len(),
            1,
            "only the real table counts"
        );
    }

    /// The repair id is spelled twice, and this is what keeps the two spellings the same.
    ///
    /// `ethos-parser-core` names the mode with an explicit `#[serde(rename = "pad-19-to-20-v1")]`
    /// because `rename_all = "kebab-case"` would derive `pad19-to20-v1`, which is not what
    /// [`XREF_REPAIR_V1`] publishes. Two spellings of one repair is exactly the drift a versioned
    /// id exists to prevent, and until v2-S14.1 nothing compared them: `the_default_profile_is_
    /// pinned` pins the serde spelling inside the canonical JSON and never reads this const, so
    /// changing this const alone failed nothing.
    ///
    /// The check lives here rather than in `ethos-parser-core` because `ethos-parser-pdf` depends on
    /// `ethos-parser-core` and importing back would be a dependency cycle. Both directions are asserted:
    /// the variant must serialize to this string, and this string must deserialize to the variant.
    /// One direction alone would pass if a second variant were given the same rename.
    #[test]
    fn the_repair_id_is_spelled_the_same_in_the_profile_and_in_this_module() {
        let emitted = serde_json::to_value(ethos_parser_core::XrefRepair::Pad19To20V1)
            .expect("an adjacently tagged unit variant serializes");
        assert_eq!(
            emitted["mode"].as_str(),
            Some(XREF_REPAIR_V1),
            "`Profile::xref_repair` puts this string on the wire; it must be the id this module \
             publishes, or an artifact names a repair no reader can look up"
        );

        let parsed: ethos_parser_core::XrefRepair =
            serde_json::from_value(serde_json::json!({ "mode": XREF_REPAIR_V1 }))
                .expect("the published id must name a mode `ethos-parser-core` understands");
        assert_eq!(
            parsed,
            ethos_parser_core::XrefRepair::Pad19To20V1,
            "the id must round-trip to the variant that performs this repair, not merely to some \
             variant that happens to accept the string"
        );
    }

    #[test]
    fn every_refusal_has_a_stable_name() {
        for r in [
            Refusal::NotExactlyOneTable,
            Refusal::HasPrevChain,
            Refusal::StartxrefDisagrees,
            Refusal::NotTheNineteenByteClass,
            Refusal::ObjectFollowsTable,
            Refusal::Unreadable,
        ] {
            assert!(!r.as_str().is_empty());
            assert!(_refusal_error(r).code() == "malformed");
        }
    }
}
