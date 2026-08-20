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

//! `xl/workbook.xml` and its worksheets → cells, at the addresses the package states.
//!
//! # What this reads, and why it is four parts and not three
//!
//! The workbook's `<sheets>` list, the relationship part that binds each `<sheet>` to a package
//! part, each worksheet's `<sheetData>`, and the shared string table. The relationship part is
//! the one that looks skippable and is not: **`xl/workbook.xml` contains no part names at all.**
//! A `<sheet>` carries a name, a `sheetId` and an `r:id`, and only `xl/_rels/workbook.xml.rels`
//! says which part that `r:id` means.
//!
//! The tempting shortcut — assume `xl/worksheets/sheet{n}.xml` in `<sheets>` order — is wrong in
//! ordinary files, not exotic ones. Reordering sheets in Excel reorders the `<sheet>` elements
//! and leaves the part names alone, so the first sheet is routinely `sheet3.xml`; deleting a
//! sheet leaves a gap, so three sheets can be `sheet1`, `sheet3`, `sheet4`; and part names are
//! author-chosen in the first place. Every one of those failures attaches the **wrong sheet name
//! to the right cells**, which is a locator that is confidently wrong — strictly worse than one
//! that is absent, and the thing `docs/01-CONTRACT.md` §5.2 exists to prevent.
//!
//! # No layout, and for a spreadsheet that is a sharper rule than for a document
//!
//! A workbook is the format where the forbidden thing is easiest to reach for: it has column
//! widths, row heights, print areas, page breaks and a "fit to page" setting, and every one of
//! them is a rendering instruction rather than part of a cell's name. `docs/06-STEAL-REFUSE.md`
//! L30 refuses the printer, and none of those knobs is read here. What is read is
//! `<c r="B12">` — an address the file wrote down.
//!
//! # `<f>` is not a second authority
//!
//! There is no evaluator here and there will not be one. A cell's text is the value the workbook
//! **stored**: its cached `<v>`, or, when nothing was cached, the formula source as written — and
//! [`engine_core::CellTextSource`] says which, so `SUM(B2:B2)` can never be mistaken for a number
//! the sheet displayed.

use engine_core::{CellTextSource, CellValueType, EngineError};
use quick_xml::events::{BytesStart, Event};

use crate::opc::resolve_target;
use crate::xml::{
    attribute_value, cdata_text, check_closed, decode, local_name, new_reader, parse_error,
    resolve_entity,
};

// The OPC relationship machinery moved to `opc.rs` at v2-S4 so the slide reader could use the
// same rule rather than a second copy of it. Re-exported so this module's surface is unchanged.
pub use crate::opc::{read_relationships, Relationship};

/// The part every workbook package keeps its sheet list in.
pub const WORKBOOK_PART: &str = "xl/workbook.xml";

/// The part that binds each `<sheet>`'s `r:id` to a package part. See the module docs.
pub const WORKBOOK_RELS_PART: &str = "xl/_rels/workbook.xml.rels";

/// The part a workbook keeps its shared string table in. Absent when no cell uses one.
pub const SHARED_STRINGS_PART: &str = "xl/sharedStrings.xml";

/// The relationship type a `<sheet>` has when it is a worksheet with cells in it.
///
/// A `<sheet>` may equally name a **chart sheet** or a **dialog sheet**, neither of which has a
/// `<sheetData>`. Reading the type is how those are told apart and declared rather than returned
/// as empty worksheets.
const WORKSHEET_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

/// The relationship type that binds a workbook to its shared string table.
const SHARED_STRINGS_REL_TYPE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

/// Parts that carry text and that v2-S3 does not read, matched by prefix.
///
/// `xl/chartsheets/` is here as well as in the non-worksheet sheet count: a chart sheet is both a
/// listed tab this reader passes over and a part with text in it, and the two counts answer
/// different questions.
const UNREAD_TEXT_PART_PREFIXES: [&str; 6] = [
    "xl/charts/",
    "xl/chartsheets/",
    "xl/drawings/",
    "xl/comments",
    "xl/threadedComments/",
    "xl/pivotCache",
];

/// One `<sheet>` as `xl/workbook.xml` lists it, before its part is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetRef {
    /// The workbook's own name for the sheet, entities resolved.
    pub name: String,
    /// The `r:id` that names its relationship. Meaningless without [`WORKBOOK_RELS_PART`].
    pub rel_id: String,
}

/// A sheet whose package part is known: the pairing this reader had to resolve to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// The workbook's own name for it.
    pub name: String,
    /// The package part it lives in, e.g. `xl/worksheets/sheet1.xml`.
    pub part: String,
}

/// One `<c>` that carries text, with the address the worksheet states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    /// The digits of the cell's `r` attribute, 1-based.
    pub row: u32,
    /// The letters of the cell's `r` attribute, e.g. `B`, `AA`.
    pub column: String,
    /// The text, as stored. Never a formatted or evaluated value.
    pub text: String,
    /// What the cell's `t` attribute says the stored value is.
    pub value_type: CellValueType,
    /// Where [`Self::text`] came from.
    pub text_source: CellTextSource,
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

/// The package part holding the shared string table, or `None` if this workbook has none.
///
/// Resolved through the workbook's relationships for the reason a worksheet is: the part name is
/// author-chosen and `xl/sharedStrings.xml` is only a convention. The conventional name is still
/// accepted as a fallback when the package carries it but declares no relationship for it —
/// reading a part that is demonstrably there is not a guess, and refusing it would reject
/// workbooks that open everywhere else.
pub fn shared_strings_part(rels: &[Relationship], entry_names: &[String]) -> Option<String> {
    if let Some(rel) = rels
        .iter()
        .find(|r| !r.external && r.kind == SHARED_STRINGS_REL_TYPE)
    {
        return Some(resolve_target(&rel.target, "xl"));
    }
    entry_names
        .iter()
        .any(|n| n == SHARED_STRINGS_PART)
        .then(|| SHARED_STRINGS_PART.to_string())
}

/// Read the `<sheets>` list of an `xl/workbook.xml`.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, or if a `<sheet>` carries no `name` or
/// no `r:id` — an unnamed sheet has no address and an unbound one has no part, and guessing
/// either is the confidently-wrong locator this module exists to avoid.
pub fn read_sheets(part: &[u8]) -> Result<Vec<SheetRef>, EngineError> {
    let mut reader = new_reader(part, WORKBOOK_PART)?;
    let mut sheets = Vec::new();
    let mut depth: i32 = 0;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, WORKBOOK_PART, &e)),
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                depth += 1;
                if local_name(start.name().as_ref()) == b"sheet" {
                    sheets.push(sheet_ref(&start)?);
                }
            }
            Ok(Event::Empty(start)) => {
                // A `<sheet>` is self-closing in every workbook a writer produces, so this arm
                // is the one that actually fires. `Start` is handled too rather than assumed
                // away: a reader that found nothing would report an empty workbook.
                if local_name(start.name().as_ref()) == b"sheet" {
                    sheets.push(sheet_ref(&start)?);
                }
            }
            Ok(Event::End(_)) => depth -= 1,
            Ok(_) => {}
        }
    }

    check_closed(depth, WORKBOOK_PART)?;
    if sheets.is_empty() {
        return Err(EngineError::Malformed {
            what: WORKBOOK_PART.into(),
            detail: "this workbook lists no sheets. An empty `<sheets>` is not a workbook this \
                     reader can produce evidence from, and returning an artifact with no nodes \
                     would let a caller conclude a phrase is absent from a file nobody read."
                .into(),
        });
    }
    Ok(sheets)
}

fn sheet_ref(start: &BytesStart<'_>) -> Result<SheetRef, EngineError> {
    let mut name = None;
    let mut rel_id = None;
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: WORKBOOK_PART.into(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        let value = attribute_value(&attribute, WORKBOOK_PART)?;
        // `sheetId` keeps its own local name, so it cannot be mistaken for `r:id`, whose local
        // name is `id`. The prefix is dropped rather than resolved — `xml.rs` states that trade.
        match local_name(attribute.key.as_ref()) {
            b"name" => name = Some(value),
            b"id" => rel_id = Some(value),
            _ => {}
        }
    }
    match (name, rel_id) {
        (Some(name), Some(rel_id)) => Ok(SheetRef { name, rel_id }),
        (name, _) => Err(EngineError::Malformed {
            what: WORKBOOK_PART.into(),
            detail: format!(
                "a `<sheet>` element carries no {}. Every sheet in this list has to resolve to a \
                 name and a part, and a sheet this reader cannot address is refused rather than \
                 numbered or skipped.",
                if name.is_none() { "`name`" } else { "`r:id`" }
            ),
        }),
    }
}

/// Pair every listed sheet with the worksheet part it names, through the relationship part.
///
/// Returns the worksheets in the workbook's own order, plus the count of listed sheets that are
/// **not** worksheets — a chart sheet or a dialog sheet, which has no `<sheetData>` to read. That
/// count is a declared erasure (**A14**), not a silent skip: a workbook whose second tab is a
/// chart still has a second tab, and a caller has to be able to see that this reader passed over
/// it.
///
/// # Errors
///
/// [`EngineError::Malformed`] if a sheet's `r:id` matches no relationship — a package that names
/// a binding it does not contain is broken, and picking a part by position is the guess this
/// module refuses.
pub fn resolve_sheets(
    sheets: &[SheetRef],
    rels: &[Relationship],
) -> Result<(Vec<Sheet>, u32), EngineError> {
    let mut worksheets = Vec::with_capacity(sheets.len());
    let mut other_kinds = 0u32;

    for sheet in sheets {
        let Some(rel) = rels.iter().find(|r| r.id == sheet.rel_id) else {
            return Err(EngineError::Malformed {
                what: WORKBOOK_RELS_PART.into(),
                detail: format!(
                    "sheet `{}` names relationship `{}`, which `{WORKBOOK_RELS_PART}` does not \
                     declare. The part a sheet lives in is only stated there, so an unresolved \
                     `r:id` is a refusal — choosing `xl/worksheets/sheet{{n}}.xml` by position \
                     would attach this sheet's name to another sheet's cells.",
                    sheet.name, sheet.rel_id
                ),
            });
        };
        if rel.external || rel.kind != WORKSHEET_REL_TYPE {
            other_kinds = crate::declare(other_kinds, 1);
            continue;
        }
        worksheets.push(Sheet {
            name: sheet.name.clone(),
            part: resolve_target(&rel.target, "xl"),
        });
    }

    Ok((worksheets, other_kinds))
}

/// Read `xl/sharedStrings.xml` into the table cells index into.
///
/// A `<si>` may be one `<t>` or several `<r><t>` runs, and the runs concatenate — that is the
/// string the workbook shows in the cell, so it is the string a citation has to match.
///
/// **`<rPh>` is skipped.** Phonetic guide text is furigana attached to a string, not part of it;
/// concatenating it would turn `漢字` into `漢字かんじ`, which is text no cell ever contained.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse or carries an unresolvable entity.
pub fn read_shared_strings(part: &[u8], part_name: &str) -> Result<Vec<String>, EngineError> {
    let name = part_name;
    let mut reader = new_reader(part, name)?;
    let mut strings = Vec::new();
    let mut current = String::new();
    let mut in_si = false;
    let mut in_text = false;
    let mut in_phonetic = false;
    let mut depth: i32 = 0;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, name, &e)),
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => {
                depth += 1;
                match local_name(start.name().as_ref()) {
                    b"si" => {
                        in_si = true;
                        current.clear();
                    }
                    b"rPh" => in_phonetic = true,
                    b"t" => in_text = true,
                    _ => {}
                }
            }
            // **A self-closing `<si/>` is an entry too**, and it arrives as its own event rather
            // than as a `Start`/`End` pair. Without this arm it would take no slot in the table
            // and every index after it would resolve to the string one place along — the exact
            // silent repointing the comment below exists to prevent, arriving through the one
            // door that comment did not cover. Any XML round-trip collapses `<si></si>` to
            // `<si/>`, so this needs no unusual writer.
            Ok(Event::Empty(start)) if local_name(start.name().as_ref()) == b"si" => {
                strings.push(String::new());
            }
            Ok(Event::End(end)) => {
                depth -= 1;
                match local_name(end.name().as_ref()) {
                    b"si" => {
                        in_si = false;
                        // Pushed even when empty. The index a cell carries is a **position** in
                        // this list, so an entry skipped for being empty would shift every
                        // string after it and silently repoint every later citation.
                        strings.push(std::mem::take(&mut current));
                    }
                    b"rPh" => in_phonetic = false,
                    b"t" => in_text = false,
                    _ => {}
                }
            }
            Ok(Event::Text(text)) if in_si && in_text && !in_phonetic => {
                current.push_str(decode(&text, name)?.as_ref());
            }
            Ok(Event::CData(cdata)) if in_si && in_text && !in_phonetic => {
                current.push_str(cdata_text(&cdata, name)?.as_ref());
            }
            Ok(Event::GeneralRef(entity)) if in_si && in_text && !in_phonetic => {
                current.push_str(resolve_entity(entity.as_ref(), name)?);
            }
            Ok(_) => {}
        }
    }

    check_closed(depth, name)?;
    Ok(strings)
}

/// Read one worksheet's `<sheetData>` into the cells that carry text.
///
/// # What becomes a node, and what does not
///
/// A cell with no stored value and no formula — `<c r="B12"/>`, or one carrying only a style
/// index — is **not** a node. There was never a character in it, which is the same reason a
/// `<w:r>` with no `<w:t>` is not a node in `docx.rs`: skipping it is not an erasure because
/// nothing was erased.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the XML will not parse, a `<c>` carries no `r` attribute, an `r`
/// is not an A1 reference, a `<c>`'s row disagrees with its `<row>`'s, a `t` is not a legal
/// `ST_CellType`, or a shared-string index does not exist.
pub fn read_cells(
    part: &[u8],
    part_name: &str,
    shared: &[String],
) -> Result<Vec<Cell>, EngineError> {
    let mut reader = new_reader(part, part_name)?;
    let mut cells = Vec::new();
    let mut open: Option<OpenCell> = None;
    let mut declared_row: Option<u32> = None;
    let mut collect = Collect::None;
    let mut in_inline = false;
    let mut in_phonetic = false;
    // Every address this sheet has already stated. A workbook is not allowed two cells at one
    // address, and emitting both would leave a citation with two answers.
    let mut seen: std::collections::BTreeSet<(String, u32)> = Default::default();
    let mut depth: i32 = 0;

    loop {
        match reader.read_event() {
            Err(e) => return Err(parse_error(&reader, part_name, &e)),
            Ok(Event::Eof) => break,

            Ok(Event::Start(start)) => {
                depth += 1;
                match local_name(start.name().as_ref()) {
                    b"row" => declared_row = row_number(&start, part_name)?,
                    b"c" => {
                        // A `<c>` inside a `<c>` is well-formed XML and not SpreadsheetML. The
                        // inner `</c>` would close the inner cell and the outer one would find
                        // nothing open, so the outer cell would vanish without a word — a silent
                        // drop, refused here by name instead.
                        if open.is_some() {
                            return Err(EngineError::Malformed {
                                what: part_name.to_string(),
                                detail: "a `<c>` opens inside another `<c>`; a cell does not \
                                         contain a cell, and reading this would lose one of them \
                                         without saying so"
                                    .into(),
                            });
                        }
                        open = Some(open_cell(&start, part_name, declared_row)?);
                    }
                    b"v" => {
                        if let Some(cell) = open.as_mut() {
                            cell.saw(Child::Value, part_name)?;
                            collect = Collect::Value;
                        }
                    }
                    b"f" => {
                        if let Some(cell) = open.as_mut() {
                            cell.saw(Child::Formula, part_name)?;
                            collect = Collect::Formula;
                        }
                    }
                    b"is" => {
                        if let Some(cell) = open.as_mut() {
                            cell.saw(Child::Inline, part_name)?;
                            in_inline = true;
                        }
                    }
                    // **The same `<rPh>` exclusion the shared-string table applies.** An `<is>`
                    // and an `<si>` are the same content type, so furigana can appear in either,
                    // and concatenating it would put `漢字かんじ` in the evidence for a cell that
                    // shows `漢字` — characters this reader authored.
                    b"rPh" => in_phonetic = true,
                    b"t" if in_inline && !in_phonetic => collect = Collect::Inline,
                    _ => {}
                }
            }

            Ok(Event::Empty(start)) => match local_name(start.name().as_ref()) {
                // A self-closing `<c/>` holds nothing, so there is no cell to open. Its `r` is
                // not validated either: an address with no text behind it is not evidence, and
                // refusing a whole workbook over the spelling of an empty cell would delete
                // content because of a cell that never had any.
                b"row" => declared_row = row_number(&start, part_name)?,
                b"v" => {
                    if let Some(cell) = open.as_mut() {
                        cell.saw(Child::Value, part_name)?;
                    }
                }
                b"f" => {
                    if let Some(cell) = open.as_mut() {
                        cell.saw(Child::Formula, part_name)?;
                    }
                }
                b"is" => {
                    if let Some(cell) = open.as_mut() {
                        cell.saw(Child::Inline, part_name)?;
                    }
                }
                _ => {}
            },

            Ok(Event::End(end)) => {
                depth -= 1;
                match local_name(end.name().as_ref()) {
                    b"c" => {
                        if let Some(cell) = open.take() {
                            if let Some(cell) = cell.finish(part_name, shared)? {
                                // One address, one cell. A sheet that states two is not
                                // describing one grid, and a citation to that address would have
                                // two answers with nothing to choose between them.
                                if !seen.insert((cell.column.clone(), cell.row)) {
                                    return Err(EngineError::Malformed {
                                        what: part_name.to_string(),
                                        detail: format!(
                                            "this sheet states cell `{}{}` more than once. A \
                                             cell has one value, and an artifact carrying both \
                                             would leave a citation to that address with two \
                                             answers.",
                                            cell.column, cell.row
                                        ),
                                    });
                                }
                                cells.push(cell);
                            }
                        }
                        in_inline = false;
                        in_phonetic = false;
                        collect = Collect::None;
                    }
                    b"is" => {
                        in_inline = false;
                        collect = Collect::None;
                    }
                    b"rPh" => in_phonetic = false,
                    // Cleared with the element that declared it, so a cell outside any `<row>`
                    // is not cross-checked against a row that already ended.
                    b"row" => declared_row = None,
                    b"v" | b"f" | b"t" => collect = Collect::None,
                    _ => {}
                }
            }

            Ok(Event::Text(text)) => {
                if let Some(cell) = open.as_mut() {
                    let decoded = decode(&text, part_name)?;
                    collect.push(cell, decoded.as_ref());
                }
            }
            Ok(Event::CData(cdata)) => {
                if let Some(cell) = open.as_mut() {
                    let text = cdata_text(&cdata, part_name)?;
                    collect.push(cell, text.as_ref());
                }
            }
            Ok(Event::GeneralRef(entity)) => {
                if let Some(cell) = open.as_mut() {
                    let resolved = resolve_entity(entity.as_ref(), part_name)?;
                    collect.push(cell, resolved);
                }
            }
            Ok(_) => {}
        }
    }

    check_closed(depth, part_name)?;
    Ok(cells)
}

/// Which of a cell's three text-bearing children the reader is inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Collect {
    None,
    Value,
    Formula,
    Inline,
}

impl Collect {
    fn push(self, cell: &mut OpenCell, text: &str) {
        match self {
            Self::None => {}
            Self::Value => cell.value.push_str(text),
            Self::Formula => cell.formula.push_str(text),
            Self::Inline => cell.inline.push_str(text),
        }
    }
}

/// A `<c>` being read, before its text is decided.
#[derive(Debug)]
struct OpenCell {
    row: u32,
    column: String,
    value_type: CellValueType,
    value: String,
    has_value: bool,
    inline: String,
    has_inline: bool,
    formula: String,
    has_formula: bool,
}

/// Which text-bearing child of a `<c>` an event just opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Child {
    Value,
    Formula,
    Inline,
}

impl Child {
    fn spelling(self) -> &'static str {
        match self {
            Self::Value => "<v>",
            Self::Formula => "<f>",
            Self::Inline => "<is>",
        }
    }
}

impl OpenCell {
    /// Record that a text-bearing child opened, refusing a second one of the same kind.
    ///
    /// **A cell has at most one of each.** Two `<v>` elements would otherwise *append*, so
    /// `<c t="s"><v>1</v><v>2</v></c>` would resolve shared string **12** — an index the
    /// workbook never wrote, pointing at a string it never referenced. That is fabrication rather
    /// than a dropped character, so it is refused for the same reason a `<c>` inside a `<c>` is.
    fn saw(&mut self, child: Child, part_name: &str) -> Result<(), EngineError> {
        let seen = match child {
            Child::Value => &mut self.has_value,
            Child::Formula => &mut self.has_formula,
            Child::Inline => &mut self.has_inline,
        };
        if *seen {
            return Err(EngineError::Malformed {
                what: part_name.to_string(),
                detail: format!(
                    "cell `{}{}` carries more than one `{}`. A cell states one value; reading \
                     both would concatenate them into a third that the file does not contain.",
                    self.column,
                    self.row,
                    child.spelling()
                ),
            });
        }
        *seen = true;
        Ok(())
    }

    /// Decide this cell's text and where it came from, or that it has none.
    ///
    /// The precedence is the `t` attribute's, not a search of whichever child happens to be
    /// present: a cell that says it is a shared string is resolved as one and **fails closed** if
    /// its index does not exist, rather than falling through to some other child's text.
    fn finish(self, part_name: &str, shared: &[String]) -> Result<Option<Cell>, EngineError> {
        // **A cell stores its value in one place.** `<v>` and `<is>` are alternatives, never a
        // pair, and a cell carrying both would have one of them read and the other dropped
        // without a word — whichever the `t` attribute happened to name. A formula may accompany
        // either, so `<f>` is not part of this check.
        if self.has_value && self.has_inline {
            return Err(EngineError::Malformed {
                what: part_name.to_string(),
                detail: format!(
                    "cell `{}{}` carries both a `<v>` and an `<is>`. Those are two answers to \
                     where its value is, and reading one would silently discard the other.",
                    self.column, self.row
                ),
            });
        }

        // **A cell whose `t` names a child it does not have, while carrying a different one, is
        // refused rather than emptied.** Returning `Ok(None)` here would be indistinguishable
        // from the legitimate empty-cell case, so the characters that *are* in the part would
        // vanish and a caller could conclude a phrase is absent from a workbook containing it.
        // A cell with no text-bearing child at all is still simply empty.
        let declared_child = match self.value_type {
            CellValueType::SharedString => Some((Child::Value, self.has_value)),
            CellValueType::InlineString => Some((Child::Inline, self.has_inline)),
            _ => None,
        };
        if let Some((child, present)) = declared_child {
            let carries_something = self.has_value || self.has_inline || self.has_formula;
            if !present && carries_something {
                return Err(EngineError::Malformed {
                    what: part_name.to_string(),
                    detail: format!(
                        "cell `{}{}` declares `t=\"{}\"` but carries no `{}` — its text is in a \
                         child the type does not name. The file contradicts itself about where \
                         this cell's value is, and this reader does not choose which half to \
                         believe.",
                        self.column,
                        self.row,
                        match self.value_type {
                            CellValueType::SharedString => "s",
                            _ => "inlineStr",
                        },
                        child.spelling()
                    ),
                });
            }
        }

        let text = match self.value_type {
            CellValueType::SharedString => {
                if !self.has_value {
                    return Ok(None);
                }
                shared_string(&self.value, shared, part_name, &self.column, self.row)?
            }
            CellValueType::InlineString => {
                if !self.has_inline {
                    return Ok(None);
                }
                self.inline
            }
            _ => {
                if self.has_value {
                    self.value
                } else if self.has_inline {
                    self.inline
                } else if self.has_formula {
                    self.formula
                } else {
                    return Ok(None);
                }
            }
        };

        if text.is_empty() {
            return Ok(None);
        }

        // A cached value and a formula source are different facts about the same cell, and the
        // whole point of the field is that they cannot be confused.
        let text_source = match (self.has_formula, self.has_value || self.has_inline) {
            (false, _) => CellTextSource::StoredValue,
            (true, true) => CellTextSource::CachedFormulaResult,
            (true, false) => CellTextSource::FormulaSource,
        };

        Ok(Some(Cell {
            row: self.row,
            column: self.column,
            text,
            value_type: self.value_type,
            text_source,
        }))
    }
}

fn open_cell(
    start: &BytesStart<'_>,
    part_name: &str,
    declared_row: Option<u32>,
) -> Result<OpenCell, EngineError> {
    let mut reference = None;
    let mut value_type = CellValueType::Number;
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        match local_name(attribute.key.as_ref()) {
            b"r" => reference = Some(attribute_value(&attribute, part_name)?),
            b"t" => {
                value_type = cell_value_type(&attribute_value(&attribute, part_name)?, part_name)?
            }
            _ => {}
        }
    }

    let Some(reference) = reference else {
        return Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: "a `<c>` carries no `r` attribute. SpreadsheetML allows the address to be \
                     implied by element order, and this reader refuses that rather than counting \
                     one: a counted address is a number this engine produced, not one the file \
                     states, and it would be wrong for every sparse sheet."
                .into(),
        });
    };

    let (column, row) = split_reference(&reference, part_name)?;

    // A worksheet states each cell's row twice. When the two disagree the file is not describing
    // one grid, and picking a winner would put a cell at an address no part of the file gives it.
    if let Some(declared) = declared_row {
        if declared != row {
            return Err(EngineError::Malformed {
                what: part_name.to_string(),
                detail: format!(
                    "cell `{reference}` is inside `<row r=\"{declared}\">`, so the worksheet \
                     states two different rows for it. A cell has one address, and this reader \
                     does not choose between a file's contradictory claims about where its \
                     evidence is."
                ),
            });
        }
    }

    Ok(OpenCell {
        row,
        column,
        value_type,
        value: String::new(),
        has_value: false,
        inline: String::new(),
        has_inline: false,
        formula: String::new(),
        has_formula: false,
    })
}

/// Split an A1 reference into its letters and its digits: `B12` → (`B`, 12).
///
/// Uppercase letters only, which is what `ST_CellRef` states. A lowercase reference is refused
/// rather than upcased: normalising it would emit an address spelled differently from the one in
/// the file, and this locator's whole claim is that it is a quotation.
fn split_reference(reference: &str, part_name: &str) -> Result<(String, u32), EngineError> {
    let split = reference
        .find(|c: char| !c.is_ascii_uppercase())
        .unwrap_or(reference.len());
    let (column, digits) = reference.split_at(split);

    // **Parsed, then checked against what was parsed.** `u32::from_str` accepts a leading `+` and
    // leading zeros, so `A+1` and `A01` would both come back as row 1 — and the locator would
    // then spell an address (`A1`) that is not the one the file wrote, quietly breaking
    // `XlsxLocator`'s own claim that its two halves concatenate back to the `r` attribute.
    let well_formed = !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit())
        && !digits.starts_with('0');
    let row: u32 = if well_formed {
        digits.parse().unwrap_or(0)
    } else {
        0
    };
    if column.is_empty() || digits.is_empty() || row == 0 {
        return Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!(
                "`{reference}` is not a cell reference: an A1 address is one or more uppercase \
                 letters followed by a 1-based row number. Refused rather than repaired — a \
                 repaired address points somewhere, and nothing here knows where."
            ),
        });
    }
    Ok((column.to_string(), row))
}

fn row_number(start: &BytesStart<'_>, part_name: &str) -> Result<Option<u32>, EngineError> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|e| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!("attribute will not parse: {e}"),
        })?;
        if local_name(attribute.key.as_ref()) == b"r" {
            let raw = attribute_value(&attribute, part_name)?;
            return match raw.parse::<u32>() {
                Ok(n) if n >= 1 => Ok(Some(n)),
                _ => Err(EngineError::Malformed {
                    what: part_name.to_string(),
                    detail: format!("`<row r=\"{raw}\">` is not a 1-based row number"),
                }),
            };
        }
    }
    // Absent is legal and means "the row after the previous one". Nothing is inferred from it:
    // the cross-check simply does not run, and each cell's own `r` remains the address.
    Ok(None)
}

fn cell_value_type(raw: &str, part_name: &str) -> Result<CellValueType, EngineError> {
    match raw {
        "n" => Ok(CellValueType::Number),
        "s" => Ok(CellValueType::SharedString),
        "inlineStr" => Ok(CellValueType::InlineString),
        "str" => Ok(CellValueType::FormulaString),
        "b" => Ok(CellValueType::Boolean),
        "e" => Ok(CellValueType::Error),
        "d" => Ok(CellValueType::Date),
        other => Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!(
                "`t=\"{other}\"` is not one of SpreadsheetML's cell types. Refused rather than \
                 read as a number or a string, because which one it is decides whether the \
                 stored value is text or an index into another part."
            ),
        }),
    }
}

fn shared_string(
    raw: &str,
    shared: &[String],
    part_name: &str,
    column: &str,
    row: u32,
) -> Result<String, EngineError> {
    let Ok(index) = raw.trim().parse::<usize>() else {
        return Err(EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!(
                "cell `{column}{row}` says its value is shared string `{raw}`, which is not an \
                 index. There is no text to fall back to and an empty string would be a claim \
                 the cell is blank."
            ),
        });
    };
    shared
        .get(index)
        .cloned()
        .ok_or_else(|| EngineError::Malformed {
            what: part_name.to_string(),
            detail: format!(
                "cell `{column}{row}` points at shared string {index}, and the table has {}. The \
             cell's text is in a part this package does not contain what it claims to, so the \
             cell fails closed: an empty string here would report a blank cell in a workbook \
             that shows one with words in it.",
                shared.len()
            ),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHEET: &str = "xl/worksheets/sheet1.xml";

    fn sheet(body: &str) -> String {
        format!(
            r#"<?xml version="1.0"?><worksheet xmlns="x"><sheetData>{body}</sheetData></worksheet>"#
        )
    }

    fn cells(body: &str, shared: &[&str]) -> Result<Vec<Cell>, EngineError> {
        let shared: Vec<String> = shared.iter().map(|s| s.to_string()).collect();
        read_cells(sheet(body).as_bytes(), SHEET, &shared)
    }

    // ---------------------------------------------------------------------------------------
    // Addresses are read, never derived
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_cell_carries_the_address_its_own_r_attribute_states() {
        let read = cells(
            r#"<row r="12"><c r="AA12" t="inlineStr"><is><t>far right</t></is></c></row>"#,
            &[],
        )
        .expect("well-formed");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].column, "AA", "the letters, not a base-26 index");
        assert_eq!(read[0].row, 12);
    }

    #[test]
    fn a_cell_with_no_r_attribute_is_refused_rather_than_counted() {
        let error = cells(
            r#"<row r="1"><c t="inlineStr"><is><t>where?</t></is></c></row>"#,
            &[],
        )
        .expect_err("an implied address is not an address the file states");
        assert!(error.to_string().contains("`r` attribute"), "{error}");
    }

    #[test]
    fn a_reference_that_is_not_an_a1_address_is_refused() {
        for bad in ["b12", "12", "B", "B0", "B12X"] {
            let body = format!(r#"<c r="{bad}" t="inlineStr"><is><t>x</t></is></c>"#);
            assert!(
                cells(&body, &[]).is_err(),
                "`{bad}` is not an A1 reference and is not repaired into one"
            );
        }
    }

    #[test]
    fn a_cell_whose_row_disagrees_with_its_row_element_is_refused() {
        let error = cells(
            r#"<row r="5"><c r="B12" t="inlineStr"><is><t>which row?</t></is></c></row>"#,
            &[],
        )
        .expect_err("a file that states two rows for one cell is not describing one grid");
        assert!(error.to_string().contains("two different rows"), "{error}");
    }

    #[test]
    fn a_cell_nested_in_a_cell_is_refused_rather_than_losing_one() {
        let error = cells(
            r#"<c r="A1"><c r="B1" t="inlineStr"><is><t>inner</t></is></c></c>"#,
            &[],
        )
        .expect_err("one of the two would have vanished");
        assert!(
            error.to_string().contains("a cell does not contain a cell"),
            "{error}"
        );
    }

    #[test]
    fn a_row_without_r_still_lets_each_cell_state_its_own_address() {
        let read = cells(
            r#"<row><c r="C7" t="inlineStr"><is><t>ok</t></is></c></row>"#,
            &[],
        )
        .expect("the cross-check simply does not run");
        assert_eq!((read[0].column.as_str(), read[0].row), ("C", 7));
    }

    // ---------------------------------------------------------------------------------------
    // Every failure closed
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_shared_string_index_that_does_not_exist_fails_closed() {
        let error = cells(r#"<c r="A1" t="s"><v>7</v></c>"#, &["only one"])
            .expect_err("an index past the end of the table is not an empty cell");
        let text = error.to_string();
        assert!(text.contains("A1"), "the failure names the cell: {text}");
        assert!(text.contains('7') && text.contains('1'), "{text}");
    }

    #[test]
    fn a_shared_string_index_that_is_not_a_number_fails_closed() {
        assert!(cells(r#"<c r="A1" t="s"><v>x</v></c>"#, &["a"]).is_err());
    }

    #[test]
    fn a_workbook_with_no_shared_table_still_fails_closed_on_a_shared_cell() {
        // The empty table is what an absent `xl/sharedStrings.xml` produces, and a cell that
        // claims an index into it must not resolve to `""`.
        assert!(cells(r#"<c r="A1" t="s"><v>0</v></c>"#, &[]).is_err());
    }

    #[test]
    fn an_unknown_cell_type_is_refused_rather_than_guessed() {
        let error = cells(r#"<c r="A1" t="zzz"><v>1</v></c>"#, &[])
            .expect_err("which type it is decides whether `<v>` is text or an index");
        assert!(error.to_string().contains("zzz"), "{error}");
    }

    #[test]
    fn a_truncated_sheet_is_refused_rather_than_read_as_far_as_it_goes() {
        assert!(read_cells(b"<worksheet><sheetData><row><c r=\"A1\">", SHEET, &[]).is_err());
    }

    #[test]
    fn an_entity_this_reader_cannot_resolve_is_refused() {
        assert!(cells(r#"<c r="A1" t="inlineStr"><is><t>&xxe;</t></is></c>"#, &[]).is_err());
    }

    // ---------------------------------------------------------------------------------------
    // Text, and where it came from
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_formula_with_no_cached_value_carries_its_source_and_is_labelled_as_such() {
        let read = cells(r#"<c r="A1"><f>SUM(B1:B9)</f></c>"#, &[]).expect("well-formed");
        assert_eq!(read[0].text, "SUM(B1:B9)");
        assert_eq!(
            read[0].text_source,
            CellTextSource::FormulaSource,
            "so nothing can mistake a formula's source for a value the sheet showed"
        );
    }

    #[test]
    fn a_formula_with_a_cached_value_carries_the_value() {
        let read = cells(r#"<c r="A1"><f>SUM(B1:B9)</f><v>9</v></c>"#, &[]).expect("well-formed");
        assert_eq!(read[0].text, "9");
        assert_eq!(read[0].text_source, CellTextSource::CachedFormulaResult);
    }

    #[test]
    fn a_self_closing_cell_is_not_a_node() {
        let read = cells(
            r#"<c r="A1"/><c r="B1" s="3"/><c r="C1" t="inlineStr"><is><t>only this</t></is></c>"#,
            &[],
        )
        .expect("well-formed");
        assert_eq!(read.len(), 1, "self-closing cells hold nothing");
        assert_eq!(read[0].column, "C");
    }

    #[test]
    fn cdata_is_read_rather_than_silently_dropped() {
        let read = cells(
            r#"<c r="A1" t="inlineStr"><is><t><![CDATA[a & b]]></t></is></c>"#,
            &[],
        )
        .expect("CDATA is ordinary character data");
        assert_eq!(
            read[0].text, "a & b",
            "an unmatched CData arm would have made this the empty string"
        );
    }

    #[test]
    fn booleans_and_errors_are_carried_as_stored() {
        let read = cells(
            r#"<c r="A1" t="b"><v>1</v></c><c r="B1" t="e"><v>#DIV/0!</v></c>"#,
            &[],
        )
        .expect("well-formed");
        assert_eq!(
            (read[0].text.as_str(), read[0].value_type),
            ("1", CellValueType::Boolean),
            "`1` is what the file stored; `true` is a translation"
        );
        assert_eq!(
            (read[1].text.as_str(), read[1].value_type),
            ("#DIV/0!", CellValueType::Error)
        );
    }

    // ---------------------------------------------------------------------------------------
    // The shared string table
    // ---------------------------------------------------------------------------------------

    fn sst(body: &str) -> Result<Vec<String>, EngineError> {
        read_shared_strings(
            format!(r#"<sst xmlns="x">{body}</sst>"#).as_bytes(),
            SHARED_STRINGS_PART,
        )
    }

    #[test]
    fn rich_text_runs_concatenate_with_no_separator() {
        let table =
            sst(r#"<si><r><t>Split </t></r><r><t>across runs</t></r></si>"#).expect("well-formed");
        assert_eq!(table, vec!["Split across runs"]);
    }

    #[test]
    fn an_empty_entry_still_holds_its_position() {
        let table = sst(r#"<si><t>first</t></si><si><t></t></si><si><t>third</t></si>"#)
            .expect("well-formed");
        assert_eq!(
            table,
            vec!["first", "", "third"],
            "dropping the empty one would repoint every citation after it"
        );
    }

    /// **The regression that a self-closing entry must never reintroduce.**
    ///
    /// `<si/>` is what any XML round-trip turns `<si></si>` into, and it arrives as its own
    /// event. An entry that took no slot would shift every index after it, so every later
    /// citation would resolve to the neighbouring string — the right address with the wrong text,
    /// which is the failure this module calls worse than no address at all.
    #[test]
    fn a_self_closing_entry_still_holds_its_position() {
        let table = sst(r#"<si><t>Alpha</t></si><si/><si><t>Gamma</t></si>"#).expect("well-formed");
        assert_eq!(table, vec!["Alpha", "", "Gamma"]);

        let cells = read_cells(
            sheet(r#"<c r="A1" t="s"><v>2</v></c>"#).as_bytes(),
            SHEET,
            &table,
        )
        .expect("well-formed");
        assert_eq!(
            cells[0].text, "Gamma",
            "index 2 is the third entry, as the workbook counts them"
        );
    }

    #[test]
    fn phonetic_guide_text_is_not_part_of_an_inline_string_either() {
        let read = cells(
            r#"<c r="A1" t="inlineStr"><is><t>KANJI</t><rPh sb="0" eb="2"><t>furigana</t></rPh></is></c>"#,
            &[],
        )
        .expect("well-formed");
        assert_eq!(
            read[0].text, "KANJI",
            "an `<is>` is the same content type as an `<si>`, so it strips furigana too"
        );
    }

    #[test]
    fn a_second_value_in_one_cell_is_refused_rather_than_concatenated() {
        // Without the guard this resolves shared string **12** — an index the file never wrote.
        let error = cells(r#"<c r="A1" t="s"><v>1</v><v>2</v></c>"#, &["a"; 20])
            .expect_err("two values would concatenate into a third the file does not contain");
        assert!(error.to_string().contains("more than one `<v>`"), "{error}");

        assert!(cells(r#"<c r="A1"><is><t>a</t></is><is><t>b</t></is></c>"#, &[]).is_err());
        assert!(cells(r#"<c r="A1"><f>A</f><f>B</f><v>1</v></c>"#, &[]).is_err());
    }

    #[test]
    fn a_type_that_names_a_child_the_cell_does_not_have_is_refused() {
        // The text is right there in the part; returning `Ok(None)` would delete it silently.
        let error = cells(
            r#"<c r="A1" t="s"><is><t>Total revenue</t></is></c>"#,
            &["x"],
        )
        .expect_err("the file contradicts itself about where the value is");
        assert!(error.to_string().contains("contradicts itself"), "{error}");

        assert!(cells(r#"<c r="A1" t="inlineStr"><v>2026</v></c>"#, &[]).is_err());
        // But a cell with a type and no children at all is simply empty, not a contradiction.
        assert_eq!(
            cells(r#"<c r="A1" t="s"></c>"#, &["x"])
                .expect("an empty cell")
                .len(),
            0
        );
    }

    #[test]
    fn a_cell_storing_its_value_in_two_places_is_refused() {
        // Whichever `t` named would be read and the other silently discarded.
        let error = cells(
            r#"<c r="A1" t="s"><v>0</v><is><t>and also this</t></is></c>"#,
            &["x"],
        )
        .expect_err("two answers for where the value is");
        assert!(error.to_string().contains("two answers"), "{error}");

        // A formula alongside either is legal and stays legal.
        assert!(cells(r#"<c r="A1"><f>SUM(A2:A3)</f><v>9</v></c>"#, &[]).is_ok());
    }

    #[test]
    fn one_address_means_one_cell() {
        let error = cells(
            r#"<row r="12"><c r="B12" t="inlineStr"><is><t>1,000</t></is></c><c r="B12" t="inlineStr"><is><t>9,000</t></is></c></row>"#,
            &[],
        )
        .expect_err("two answers for one citation is not an artifact this engine emits");
        assert!(error.to_string().contains("more than once"), "{error}");
    }

    #[test]
    fn the_shared_string_table_is_bound_by_relationship_before_convention() {
        let rels = vec![Relationship {
            id: "rId9".into(),
            kind: SHARED_STRINGS_REL_TYPE.into(),
            target: "strings.xml".into(),
            external: false,
        }];
        assert_eq!(
            shared_strings_part(&rels, &[]).as_deref(),
            Some("xl/strings.xml"),
            "the part name is author-chosen, exactly as a worksheet's is"
        );
        // No relationship, but the conventional part is demonstrably in the package.
        assert_eq!(
            shared_strings_part(&[], &[SHARED_STRINGS_PART.to_string()]).as_deref(),
            Some(SHARED_STRINGS_PART)
        );
        assert_eq!(
            shared_strings_part(&[], &[]),
            None,
            "a workbook may have none"
        );
    }

    #[test]
    fn phonetic_guide_text_is_not_part_of_the_string() {
        let table = sst(r#"<si><t>KANJI</t><rPh sb="0" eb="2"><t>furigana</t></rPh></si>"#)
            .expect("well-formed");
        assert_eq!(
            table,
            vec!["KANJI"],
            "furigana is attached to a string, not part of it"
        );
    }

    // ---------------------------------------------------------------------------------------
    // Sheets, and the relationship part
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_sheet_is_bound_to_its_part_by_relationship_id_not_by_position() {
        let sheets = vec![
            SheetRef {
                name: "First".into(),
                rel_id: "rId9".into(),
            },
            SheetRef {
                name: "Second".into(),
                rel_id: "rId1".into(),
            },
        ];
        // Deliberately reversed: position and `r:id` disagree, and the `r:id` is what counts.
        let rels = vec![
            Relationship {
                id: "rId1".into(),
                kind: WORKSHEET_REL_TYPE.into(),
                target: "worksheets/sheetA.xml".into(),
                external: false,
            },
            Relationship {
                id: "rId9".into(),
                kind: WORKSHEET_REL_TYPE.into(),
                target: "worksheets/sheetB.xml".into(),
                external: false,
            },
        ];
        let (resolved, others) = resolve_sheets(&sheets, &rels).expect("both resolve");
        assert_eq!(others, 0);
        assert_eq!(resolved[0].part, "xl/worksheets/sheetB.xml");
        assert_eq!(resolved[1].part, "xl/worksheets/sheetA.xml");
    }

    #[test]
    fn a_sheet_whose_relationship_is_missing_is_a_named_refusal() {
        let sheets = vec![SheetRef {
            name: "Orphan".into(),
            rel_id: "rId4".into(),
        }];
        let error = resolve_sheets(&sheets, &[]).expect_err("an unresolved `r:id` is refused");
        let text = error.to_string();
        assert!(text.contains("Orphan") && text.contains("rId4"), "{text}");
    }

    #[test]
    fn a_chart_sheet_is_counted_rather_than_read_as_an_empty_worksheet() {
        let sheets = vec![SheetRef {
            name: "Chart".into(),
            rel_id: "rId1".into(),
        }];
        let rels = vec![Relationship {
            id: "rId1".into(),
            kind: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet"
                .into(),
            target: "chartsheets/sheet1.xml".into(),
            external: false,
        }];
        let (resolved, others) = resolve_sheets(&sheets, &rels).expect("resolves");
        assert!(resolved.is_empty());
        assert_eq!(others, 1, "declared, not silently dropped");
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// `other_kinds` is this reader's A14 count — sheet-list entries that are not worksheets — and
    /// it reaches the artifact as a declared limitation. It was the site v2-S9.1 first missed,
    /// because that slice searched for names ending `_not_read` and this one does not end that
    /// way. A site list is not a search.
    #[test]
    fn a_workbook_erasure_count_saturates_rather_than_wrapping() {
        let sheets = vec![SheetRef {
            name: "Chart".into(),
            rel_id: "rId1".into(),
        }];
        let rels = vec![Relationship {
            id: "rId1".into(),
            kind: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chartsheet"
                .into(),
            target: "chartsheets/sheet1.xml".into(),
            external: false,
        }];
        let (_, others) = resolve_sheets(&sheets, &rels).expect("resolves");
        assert_eq!(others, 1, "the chartsheet is counted");

        // A workbook's sheet list is bounded by nothing but its own `<sheets>` element, so the
        // fold this count goes through must report the ceiling rather than a small number.
        let mut folded = u32::MAX - 1;
        for _ in 0..3 {
            folded = crate::declare(folded, others);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );
    }

    #[test]
    fn a_sheet_name_keeps_the_entity_the_workbook_wrote() {
        let sheets = read_sheets(
            br#"<workbook xmlns:r="x"><sheets><sheet name="Notes &amp; sources" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        )
        .expect("well-formed");
        assert_eq!(sheets[0].name, "Notes & sources");
        assert_eq!(sheets[0].rel_id, "rId1");
    }

    #[test]
    fn a_workbook_that_lists_no_sheets_is_refused() {
        assert!(read_shared_strings(b"<sst/>", SHARED_STRINGS_PART).is_ok());
        let error = read_sheets(br#"<workbook><sheets/></workbook>"#)
            .expect_err("an artifact with no nodes would look like a workbook with no words");
        assert!(error.to_string().contains("lists no sheets"), "{error}");
    }

    #[test]
    fn unread_text_parts_are_counted_by_prefix() {
        let names: Vec<String> = [
            "xl/workbook.xml",
            "xl/worksheets/sheet1.xml",
            "xl/charts/chart1.xml",
            "xl/drawings/drawing1.xml",
            "xl/comments1.xml",
            "xl/styles.xml",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        // `styles.xml` carries no text a citation could land in, so it is not an erasure.
        assert_eq!(unread_text_parts(&names), 3);
    }
}
