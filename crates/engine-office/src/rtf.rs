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

//! A brace-group control-word stream → paragraphs (v2-S8).
//!
//! # The first v2 format that is not a package
//!
//! Six formats in, every reader in this crate opened by asking a **container** a question: does
//! the central directory list `word/document.xml`, does the first stored entry declare an
//! OpenDocument type, which part does this `r:id` resolve to. Rich Text Format has none of that.
//! It is one sequence of bytes: `{`, `}`, control words beginning with `\`, and everything else is
//! text. There is no manifest, no part, and no name the document has for itself.
//!
//! That is why this slice touched an invariant. `check_structure`'s page-less shape checked that
//! one part id means one part name — and there is no part name here to be a bijection between.
//! Inventing a constant would have let the old check run unchanged and would have put a string in
//! every citation that the document does not contain, which is `docs/14-V2-SCOPE.md` §3's
//! *"absent, not invented"* in a smaller place than usual. See
//! [`engine_core::NativeLocator::names_a_part`].
//!
//! # The page, said out loud, and still refused
//!
//! An ODT hides its page in `<text:soft-page-break/>` and an ODP in a `<draw:page>` element. RTF
//! writes `\page` — a page break, in the plainest words any of these formats use — and `\paperw`
//! beside it. Both are the producing application's print arithmetic, and `docs/06-STEAL-REFUSE.md`
//! L30 refuses invented pagination whether inventing it costs a renderer or costs nothing.
//! `\page` is matched, contributes no character, and produces no [`engine_core::PageRecord`].
//!
//! # Skip unless transparent, which is the inverted allowlist again
//!
//! `odt.rs` inverted its rule because ODF puts a great deal of non-displayed character data inside
//! a `<text:p>`. RTF has the same hazard in a different shape: a group's text belongs to the body
//! only if the group is *formatting*, and a group whose first control word names a **destination**
//! — `\fonttbl`, `\stylesheet`, `\pict`, `\header`, `\footnote`, `\field` — holds something else
//! entirely.
//!
//! There is no way to enumerate every destination a producer might write, so the rule is inverted
//! exactly as `odt.rs` inverts its own: a group is transparent only when its first control word is
//! in [`TRANSPARENT`], and **everything else is skipped and counted** (**A14**). The failure mode
//! is a phrase reported missing, never a phrase the document does not contain — which is the
//! direction every slice since v2-S5 has committed to, and the one that matters most here because
//! the alternative is A14 inverted: a header's words spliced into the body, where a consumer
//! cannot tell them from evidence.
//!
//! A `{\*\…}` group is skipped without consulting the list at all. `\*` is the format's own marker
//! for *"a destination a reader may ignore"*, so honouring it is reading the file.
//!
//! # What is read, and what is declared
//!
//! Read: plain characters, `\uN` as the scalar it names, and the small closed set of
//! special-character control words that each stand for exactly one character.
//!
//! Declared, never guessed: **`\'hh` above 0x7F**. That byte's meaning depends on a code page —
//! `\ansicpg1252`, `\ansicpg932` — and this reader carries no table for one. Emitting a Latin-1
//! character would be mojibake presented as a success. Below 0x80 the byte is the same character
//! in every ANSI code page, so reading it is reading rather than choosing.
//!
//! # What this reader was and was not measured against
//!
//! **No corpus of real `.rtf` files was available** — the fourth consecutive slice that has to say
//! so, repeated rather than quietly inherited. Every rule here is read off the RTF specification
//! and pinned against files this repository authors byte by byte.

use engine_core::{EngineError, RtfParagraphBreak};

/// The media type a Rich Text Format document is served as.
///
/// IANA registers both `application/rtf` and `text/rtf`. The former is used here because this
/// engine's `source.media_type` names *what the bytes are*, and an `.rtf` is a structured document
/// format rather than something a reader should treat as plain text.
pub const RTF_MEDIA_TYPE: &str = "application/rtf";

/// The bytes every Rich Text Format document begins with.
///
/// An open group followed immediately by the `rtf` control word, which the specification requires
/// to be the first thing in the file. A bare `{` is not enough, and neither is an OLE compound
/// file — a legacy `.doc` begins `D0 CF 11 E0` and is a different format with a different reader.
const MAGIC: &[u8] = b"{\\rtf";

/// The only major version this reader claims to understand.
///
/// `\rtf1` is what every producer writes. A different major version is a **named refusal** rather
/// than a best effort: the version is the file telling a reader which specification it was written
/// against, and reading it under the wrong one is how text goes silently missing.
const SUPPORTED_VERSION: i64 = 1;

/// A ceiling on how deeply groups may nest before this reader refuses the stream.
///
/// Real nesting is a handful. A stack this deep is a stack of frames rather than a parse depth, so
/// it is bounded here rather than left to grow — `zip.rs`'s rule that a bomb is "a named refusal
/// rather than an out-of-memory kill".
const MAX_GROUP_NESTING: usize = 256;

/// A ceiling on the characters one stream may expand to.
///
/// On the total, because that is where the amplification is: `\u` is six source bytes producing
/// one scalar, but a stream of them is bounded only by the file, and a caller handing over a large
/// file should get a named refusal rather than an allocation failure.
const MAX_TEXT_BYTES: usize = 64 * 1024 * 1024;

/// How many characters a `\uN` fallback occupies, when the stream states none.
const DEFAULT_UNICODE_SKIP: u32 = 1;

/// A ceiling on `\ucN`, so a stream cannot ask this reader to skip forever.
const MAX_UNICODE_SKIP: u32 = 4096;

/// Control words whose group is **formatting**, so its text is the body's.
///
/// # Deliberately short and deliberately closed
///
/// The same discipline `odt.rs`'s `InlineText` arm is under, and for a sharper reason. A group
/// this list does not name has its text **declared** rather than spliced, so the cost of a missing
/// entry is a phrase reported absent — recoverable, and visible in the artifact's own limitation
/// count. The cost of the opposite rule is a `{\footer …}`'s words appearing in the body, which is
/// A14 inverted: a silent *extra* a consumer cannot tell from evidence.
///
/// Only the **first** control word of a group is looked up here. Everything after it is ordinary
/// stream content, so this is a list of things that legitimately *open* a group, not a list of
/// every control word RTF has.
const TRANSPARENT: &[&str] = &[
    // The document itself.
    "rtf",
    // Character formatting.
    "b",
    "i",
    "ul",
    "ulnone",
    "strike",
    "outl",
    "shad",
    "scaps",
    "caps",
    "v",
    "sub",
    "super",
    "nosupersub",
    "plain",
    "f",
    "fs",
    "cf",
    "cb",
    "highlight",
    "lang",
    "langfe",
    "langnp",
    "noproof",
    "rtlch",
    "ltrch",
    "loch",
    "hich",
    "dbch",
    "af",
    "afs",
    "expnd",
    "expndtw",
    "kerning",
    "charscalex",
    "cs",
    "insrsid",
    "rsid",
    // Paragraph formatting and the breaks themselves.
    "pard",
    "par",
    "s",
    "ql",
    "qc",
    "qr",
    "qj",
    "li",
    "ri",
    "fi",
    "sa",
    "sb",
    "sl",
    "slmult",
    "keep",
    "keepn",
    "widctlpar",
    "nowidctlpar",
    "intbl",
    "itap",
    "sect",
    "cell",
    "row",
    "line",
    "tab",
    // Unicode.
    "u",
    "uc",
];

/// Control words that stand for exactly one character.
///
/// **Read, not inferred.** Each of these is the file spelling a character it could not write
/// literally, which is the same claim `odt.rs` makes about `<text:tab/>` and `<text:line-break/>`.
/// A control word this list does not name contributes no character, because the overwhelming
/// majority of them are formatting — and that is stated as a limitation rather than left implicit.
const SPECIAL_CHARACTERS: &[(&str, char)] = &[
    ("tab", '\t'),
    ("line", '\n'),
    ("lquote", '\u{2018}'),
    ("rquote", '\u{2019}'),
    ("ldblquote", '\u{201C}'),
    ("rdblquote", '\u{201D}'),
    ("emdash", '\u{2014}'),
    ("endash", '\u{2013}'),
    ("bullet", '\u{2022}'),
    ("emspace", '\u{2003}'),
    ("enspace", '\u{2002}'),
];

/// One paragraph the stream delimits, with the address it states for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    /// 1-based position in the stream's own order, counting paragraphs inside destinations this
    /// reader does not read.
    pub ordinal: u32,
    /// Which control word ended it.
    pub terminator: RtfParagraphBreak,
    /// The characters the stream states, under the rules in this module's header.
    pub text: String,
}

/// What one stream yielded, plus what it passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The paragraphs that carry text, in the stream's own order.
    pub paragraphs: Vec<Paragraph>,
    /// Destinations that held characters and were not read: a font table, a style sheet, a
    /// picture, a header, a footnote, a field, or any group this reader does not recognise.
    pub destinations_not_read: u32,
    /// Bytes written as `\'hh` above 0x7F, or appearing raw above 0x7F.
    ///
    /// Their meaning depends on a code page this reader does not read, so each is counted and
    /// contributes no character. **Not a repair and not a guess** — see the module header.
    pub undecodable_bytes: u32,
}

/// Whether these bytes are a Rich Text Format document, **read from the bytes** (**A4**).
///
/// The specification requires the file to begin with `{\rtf`, so that is the whole question. Never
/// the extension, in both directions: a document named `report.bin` reads, and a `.rtf` full of
/// something else does not.
pub fn is_rtf(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// A group being read.
struct Group {
    /// How many characters a `\uN` fallback occupies inside this group.
    ///
    /// Inherited from the enclosing group, because `\uc` is scoped the way formatting is.
    unicode_skip: u32,
    /// Whether this group's first token has been seen, which is what decides transparency.
    decided: bool,
}

/// A destination being passed over: where it began, and what it turned out to hold.
struct Skip {
    from_depth: usize,
    held_text: bool,
}

/// Read a Rich Text Format stream into the paragraphs that carry text.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the stream does not begin with `{\rtf`, if it ends with groups
/// still open, if it closes a group it never opened, if `\bin` names more bytes than remain, or if
/// the paragraph counter runs past what a `u32` holds. [`EngineError::Unsupported`] if the major
/// version is not [`SUPPORTED_VERSION`]. [`EngineError::ResourceLimit`] if groups nest past
/// [`MAX_GROUP_NESTING`], if a `\uc` exceeds [`MAX_UNICODE_SKIP`], or if the text expands past
/// [`MAX_TEXT_BYTES`].
pub fn read(stream: &[u8]) -> Result<Document, EngineError> {
    if !is_rtf(stream) {
        return Err(EngineError::Malformed {
            what: "rtf document".into(),
            detail: "these bytes do not begin with `{\\rtf`, which the Rich Text Format \
                     specification requires of every document, so there is no RTF here to read"
                .into(),
        });
    }

    let mut paragraphs: Vec<Paragraph> = Vec::new();
    let mut destinations_not_read = 0u32;
    let mut undecodable_bytes = 0u32;

    let mut groups: Vec<Group> = Vec::new();
    let mut skip: Option<Skip> = None;
    let mut text = String::new();
    let mut text_bytes = 0usize;
    let mut ordinal: u32 = 1;
    // A `\uN` high surrogate waiting for its low half. Word writes an astral scalar as two of
    // them, so pairing is reading the file rather than repairing it.
    let mut pending_high: Option<u16> = None;

    let mut at = 0usize;
    let mut version_checked = false;

    while at < stream.len() {
        let byte = stream[at];
        match byte {
            b'{' => {
                at += 1;
                if groups.len() >= MAX_GROUP_NESTING {
                    return Err(EngineError::ResourceLimit {
                        limit: "nested groups in the rtf stream".into(),
                        configured: MAX_GROUP_NESTING.to_string(),
                    });
                }
                // **The ENCLOSING group is what this token decides**, and it decides it is not a
                // destination: only a control word can name one, and a group whose first token is
                // another group has already said it does not start with one. Marking the new group
                // instead was this reader's first behaviour, and it made every destination
                // transparent — the whole rule, silently off.
                mark_decided(&mut groups);
                let inherited = groups
                    .last()
                    .map_or(DEFAULT_UNICODE_SKIP, |g| g.unicode_skip);
                groups.push(Group {
                    unicode_skip: inherited,
                    decided: false,
                });
            }
            b'}' => {
                at += 1;
                if groups.pop().is_none() {
                    return Err(EngineError::Malformed {
                        what: "rtf document".into(),
                        detail: "this stream closes a group it never opened. A stream read as far \
                                 as it went would be a shorter document that still looked whole."
                            .into(),
                    });
                }
                if let Some(open) = &skip {
                    if groups.len() < open.from_depth {
                        let open = skip.take().expect("checked above");
                        if open.held_text {
                            destinations_not_read = crate::declare(destinations_not_read, 1);
                        }
                    }
                }
            }
            // Carriage returns and line feeds are stream layout, not text: the specification says
            // to ignore them, and a paragraph break is `\par`.
            b'\r' | b'\n' => at += 1,
            b'\\' => {
                let token = control_token(stream, at)?;
                at = token.next;
                let entering = decide(&mut groups, &token);
                if entering && skip.is_none() {
                    skip = Some(Skip {
                        from_depth: groups.len(),
                        held_text: false,
                    });
                }

                match &token.kind {
                    TokenKind::Word(word) => {
                        if !version_checked {
                            // The first control word of the stream is `\rtf`, and its parameter is
                            // the specification this document was written against.
                            version_checked = true;
                            if word != "rtf" || token.parameter.unwrap_or(SUPPORTED_VERSION) != 1 {
                                return Err(unsupported_version(token.parameter));
                            }
                        }
                        match word.as_str() {
                            "par" | "sect" | "cell" | "row" => {
                                let terminator = match word.as_str() {
                                    "par" => RtfParagraphBreak::Paragraph,
                                    "sect" => RtfParagraphBreak::Section,
                                    "cell" => RtfParagraphBreak::Cell,
                                    _ => RtfParagraphBreak::Row,
                                };
                                // **The counter advances whether or not this paragraph is read.**
                                // A `\par` inside a `{\footer …}` is a paragraph the stream
                                // delimits, so a consumer counting breaks in the bytes finds it —
                                // `OdtLocator::paragraph`'s rule, in RTF's spelling.
                                if skip.is_none() && !text.is_empty() {
                                    paragraphs.push(Paragraph {
                                        ordinal,
                                        terminator,
                                        text: std::mem::take(&mut text),
                                    });
                                }
                                text.clear();
                                ordinal = advance(ordinal)?;
                            }
                            "uc" => {
                                let stated = token.parameter.unwrap_or(0);
                                if !(0..=i64::from(MAX_UNICODE_SKIP)).contains(&stated) {
                                    return Err(EngineError::ResourceLimit {
                                        limit: "characters one `\\uc` may make this reader skip"
                                            .into(),
                                        configured: MAX_UNICODE_SKIP.to_string(),
                                    });
                                }
                                if let Some(group) = groups.last_mut() {
                                    group.unicode_skip = stated as u32;
                                }
                            }
                            "u" => {
                                let Some(stated) = token.parameter else {
                                    return Err(EngineError::Malformed {
                                        what: "rtf document".into(),
                                        detail: "a `\\u` carries no code point. That parameter is \
                                                 the character the stream states, so a `\\u` \
                                                 without one is a character of unknown identity \
                                                 rather than one this reader may choose."
                                            .into(),
                                    });
                                };
                                // RTF writes the scalar as a **signed 16-bit** number, so a value
                                // above 0x7FFF arrives negative. Adding 65536 is reading the
                                // format's own encoding, not repairing it.
                                let unit = if stated < 0 { stated + 65_536 } else { stated };
                                let unit =
                                    u16::try_from(unit).map_err(|_| EngineError::Malformed {
                                        what: "rtf document".into(),
                                        detail: format!(
                                        "`\\u{stated}` is not a 16-bit code unit. RTF states a \
                                         scalar as signed 16 bits, so a value outside that range \
                                         is one this reader cannot identify."
                                    ),
                                    })?;
                                match resolve_unit(unit, &mut pending_high) {
                                    Some(character) => {
                                        push(&mut text, character, &mut text_bytes, &mut skip)?
                                    }
                                    None => {
                                        if pending_high.is_none() {
                                            undecodable_bytes = undecodable_bytes.saturating_add(1);
                                        }
                                    }
                                }
                                let skip_count = groups
                                    .last()
                                    .map_or(DEFAULT_UNICODE_SKIP, |g| g.unicode_skip);
                                at = skip_fallback(stream, at, skip_count);
                            }
                            "bin" => {
                                // Binary data, whose bytes may contain braces and backslashes. Not
                                // read, and **bounds-checked rather than trusted**: a length past
                                // the end of the stream is a truncated document.
                                let stated = token.parameter.unwrap_or(0).max(0);
                                let stated = usize::try_from(stated).unwrap_or(usize::MAX);
                                if stated > stream.len() - at {
                                    return Err(EngineError::Malformed {
                                        what: "rtf document".into(),
                                        detail: format!(
                                            "a `\\bin{stated}` names more bytes than the stream \
                                             has left. The document is truncated, and reading it \
                                             as far as it went would be a shorter document that \
                                             still looked whole."
                                        ),
                                    });
                                }
                                if stated > 0 {
                                    if let Some(open) = &mut skip {
                                        open.held_text = true;
                                    }
                                }
                                at += stated;
                            }
                            // Read, recognised, refused — the one line `odt.rs` spends on a soft
                            // page break, for a control word that says the word out loud. `\page`
                            // is where the producing application broke a page; it contributes no
                            // character and produces no `PageRecord`.
                            "page" | "column" | "softpage" => {}
                            other => {
                                if let Some((_, character)) =
                                    SPECIAL_CHARACTERS.iter().find(|(name, _)| *name == other)
                                {
                                    push(&mut text, *character, &mut text_bytes, &mut skip)?;
                                }
                            }
                        }
                    }
                    TokenKind::Hex(value) => {
                        if *value < 0x80 {
                            push(&mut text, *value as char, &mut text_bytes, &mut skip)?;
                        } else if skip.is_some() {
                            if let Some(open) = &mut skip {
                                open.held_text = true;
                            }
                        } else {
                            undecodable_bytes = undecodable_bytes.saturating_add(1);
                        }
                    }
                    TokenKind::Literal(character) => {
                        push(&mut text, *character, &mut text_bytes, &mut skip)?
                    }
                    // `\*` marks a destination the format itself says a reader may ignore. The
                    // group is already entering a skip; there is nothing else to do with it.
                    TokenKind::Ignorable => {}
                }
            }
            _ => {
                at += 1;
                mark_decided(&mut groups);
                if byte < 0x80 {
                    push(&mut text, byte as char, &mut text_bytes, &mut skip)?;
                } else if skip.is_some() {
                    if let Some(open) = &mut skip {
                        open.held_text = true;
                    }
                } else {
                    // A raw byte above 0x7F is not legal RTF — the format spells one `\'hh` — and
                    // it means what a code page says it means either way, so it takes the same
                    // answer: declared, never guessed.
                    undecodable_bytes = undecodable_bytes.saturating_add(1);
                }
            }
        }
    }

    if !groups.is_empty() {
        return Err(EngineError::Malformed {
            what: "rtf document".into(),
            detail: format!(
                "this stream ends with {} group(s) still open; it is truncated, and a stream read \
                 as far as it went would be a shorter document that still looked whole",
                groups.len()
            ),
        });
    }

    // A writer is not required to put `\par` after the last paragraph, so text still held here is
    // the ordinary shape of a final paragraph rather than a defect. Declared as such.
    if !text.is_empty() {
        paragraphs.push(Paragraph {
            ordinal,
            terminator: RtfParagraphBreak::EndOfStream,
            text,
        });
    }

    Ok(Document {
        paragraphs,
        destinations_not_read,
        undecodable_bytes,
    })
}

/// What one `\…` token is.
enum TokenKind {
    /// A control word: letters, with an optional signed parameter.
    Word(String),
    /// `\'hh` — one byte, written as hex.
    Hex(u8),
    /// A control symbol standing for a character: `\\`, `\{`, `\}`, `\~`, `\_`.
    Literal(char),
    /// `\*` — the marker for a destination a reader may ignore.
    Ignorable,
}

struct Token {
    kind: TokenKind,
    parameter: Option<i64>,
    next: usize,
}

/// Parse one `\…` token, leaving the cursor after its delimiter.
fn control_token(stream: &[u8], at: usize) -> Result<Token, EngineError> {
    let mut cursor = at + 1;
    let Some(&first) = stream.get(cursor) else {
        return Err(EngineError::Malformed {
            what: "rtf document".into(),
            detail: "this stream ends with a bare `\\`, so its last token is incomplete".into(),
        });
    };

    if first.is_ascii_alphabetic() {
        let start = cursor;
        while stream.get(cursor).is_some_and(|b| b.is_ascii_alphabetic()) {
            cursor += 1;
        }
        let word = String::from_utf8_lossy(&stream[start..cursor]).into_owned();

        let mut parameter: Option<i64> = None;
        let negative = stream.get(cursor) == Some(&b'-');
        if negative {
            cursor += 1;
        }
        if stream.get(cursor).is_some_and(|b| b.is_ascii_digit()) {
            let digits = cursor;
            while stream.get(cursor).is_some_and(|b| b.is_ascii_digit()) {
                cursor += 1;
            }
            // Saturating rather than refusing: an over-long run of digits is a parameter no
            // control word this reader acts on can use, and every one that it does act on is
            // range-checked at its own arm.
            let value: i64 = String::from_utf8_lossy(&stream[digits..cursor])
                .parse()
                .unwrap_or(i64::MAX);
            parameter = Some(if negative { -value } else { value });
        }
        // **One space, and it is a delimiter rather than text.** The specification ends a control
        // word at the first character that cannot be part of it, and consumes a single space when
        // that is what ended it. A second space is text.
        if stream.get(cursor) == Some(&b' ') {
            cursor += 1;
        }
        return Ok(Token {
            kind: TokenKind::Word(word),
            parameter,
            next: cursor,
        });
    }

    if first == b'\'' {
        let hex = stream
            .get(cursor + 1..cursor + 3)
            .ok_or_else(|| EngineError::Malformed {
                what: "rtf document".into(),
                detail: "this stream ends inside a `\\'` escape, so its last byte is incomplete"
                    .into(),
            })?;
        let value = u8::from_str_radix(&String::from_utf8_lossy(hex), 16).map_err(|_| {
            EngineError::Malformed {
                what: "rtf document".into(),
                detail: format!(
                    "`\\'{}` is not two hexadecimal digits. That escape is how RTF writes one \
                     byte, so a value this reader cannot read is a character of unknown identity \
                     rather than one it may choose.",
                    String::from_utf8_lossy(hex)
                ),
            }
        })?;
        return Ok(Token {
            kind: TokenKind::Hex(value),
            parameter: None,
            next: cursor + 3,
        });
    }

    cursor += 1;
    let kind = match first {
        b'*' => TokenKind::Ignorable,
        b'\\' => TokenKind::Literal('\\'),
        b'{' => TokenKind::Literal('{'),
        b'}' => TokenKind::Literal('}'),
        // A non-breaking space and a non-breaking hyphen are characters the stream states.
        b'~' => TokenKind::Literal('\u{00A0}'),
        b'_' => TokenKind::Literal('\u{2011}'),
        // An optional hyphen displays nothing unless a line breaks on it, and where a line breaks
        // is the layout this reader does not perform. Matched, and contributes no character.
        b'-' => TokenKind::Word("softhyphen".into()),
        other => TokenKind::Word(format!("symbol-{other:02x}")),
    };
    Ok(Token {
        kind,
        parameter: None,
        next: cursor,
    })
}

/// Record that the innermost group's first token has been seen, without deciding anything.
fn mark_decided(groups: &mut [Group]) {
    if let Some(group) = groups.last_mut() {
        group.decided = true;
    }
}

/// Decide whether this token makes its group a destination, and mark the group decided.
///
/// Returns whether the group is one this reader passes over. Only the **first** token of a group
/// can make that decision, which is what keeps `{\b bold}` transparent while `{\fonttbl …}` is not.
fn decide(groups: &mut [Group], token: &Token) -> bool {
    let Some(group) = groups.last_mut() else {
        return false;
    };
    if group.decided {
        return false;
    }
    group.decided = true;
    match &token.kind {
        TokenKind::Ignorable => true,
        TokenKind::Word(word) => !TRANSPARENT.contains(&word.as_str()),
        // A group opening with a literal character or a hex byte is text, not a destination.
        TokenKind::Hex(_) | TokenKind::Literal(_) => false,
    }
}

/// Combine a `\u` code unit into a scalar, pairing surrogates the way the stream writes them.
///
/// Returns `None` for a high surrogate that is now pending, and for a low surrogate with no high
/// half — the second is a lone surrogate, which names no character.
fn resolve_unit(unit: u16, pending_high: &mut Option<u16>) -> Option<char> {
    if let Some(high) = pending_high.take() {
        if (0xDC00..=0xDFFF).contains(&unit) {
            let scalar = 0x1_0000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
            return char::from_u32(scalar);
        }
    }
    if (0xD800..=0xDBFF).contains(&unit) {
        *pending_high = Some(unit);
        return None;
    }
    char::from_u32(u32::from(unit))
}

/// Skip the ANSI characters a `\uN` states as its fallback.
///
/// **As the format specifies**, which is why this is a count rather than a heuristic: `\ucN` says
/// how many characters follow a `\u` as its replacement, and reading them as well would put the
/// same character in the record twice. A brace ends the skip, because a group boundary is not a
/// character the fallback may swallow.
fn skip_fallback(stream: &[u8], mut at: usize, count: u32) -> usize {
    for _ in 0..count {
        match stream.get(at) {
            None | Some(b'{') | Some(b'}') => return at,
            Some(b'\\') => {
                // A `\'hh` is one fallback character; so is a control symbol. A control word is
                // one as well, and its parameter and delimiter go with it.
                match control_token(stream, at) {
                    Ok(token) => at = token.next,
                    Err(_) => return at,
                }
            }
            Some(b'\r') | Some(b'\n') => {
                at += 1;
            }
            Some(_) => at += 1,
        }
    }
    at
}

/// Append one character, to the paragraph or to the skipped destination's tally.
fn push(
    text: &mut String,
    character: char,
    text_bytes: &mut usize,
    skip: &mut Option<Skip>,
) -> Result<(), EngineError> {
    if let Some(open) = skip {
        if !character.is_whitespace() {
            open.held_text = true;
        }
        return Ok(());
    }
    let len = character.len_utf8();
    if *text_bytes + len > MAX_TEXT_BYTES {
        return Err(EngineError::ResourceLimit {
            limit: "text bytes expanded from the rtf stream".into(),
            configured: MAX_TEXT_BYTES.to_string(),
        });
    }
    *text_bytes += len;
    text.push(character);
    Ok(())
}

/// Move the paragraph counter forward, refusing rather than pinning at the ceiling.
///
/// **A saturated ordinal is a wrong address, not a large one.** `saturating_add` would give every
/// later paragraph the same `u32::MAX`, so distinct paragraphs would share one address — the
/// locator `docs/01-CONTRACT.md` §5.2 calls strictly worse than an absent one.
fn advance(ordinal: u32) -> Result<u32, EngineError> {
    ordinal
        .checked_add(1)
        .ok_or_else(|| EngineError::Malformed {
            what: "rtf document".into(),
            detail:
                "this stream delimits more paragraphs than a paragraph index can hold. Clamping \
                 would give distinct paragraphs one address, so the document is refused rather \
                 than addressed wrongly."
                    .into(),
        })
}

fn unsupported_version(stated: Option<i64>) -> EngineError {
    EngineError::Unsupported {
        what: "rtf version".into(),
        detail: format!(
            "this stream declares `\\rtf{}`, and this reader implements version \
             {SUPPORTED_VERSION} only. The version is the file naming the specification it was \
             written against, so reading it under a different one is how text goes silently \
             missing.",
            stated.map_or_else(|| "".to_string(), |v| v.to_string())
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_body(body: &str) -> Document {
        // The space after `\ansi` is the control word's delimiter, not text. Without it the
        // body's first character would be read as part of the control word — which is the first
        // thing this helper got wrong.
        read(format!("{{\\rtf1\\ansi {body}}}").as_bytes()).expect("the stream reads")
    }

    fn texts(document: &Document) -> Vec<&str> {
        document
            .paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect()
    }

    // ---------------------------------------------------------------------------------------
    // The address
    // ---------------------------------------------------------------------------------------

    #[test]
    fn paragraphs_bind_at_the_position_the_stream_states() {
        let document = read_body(r"First paragraph.\par Second paragraph.\par Third.");
        assert_eq!(
            texts(&document),
            vec!["First paragraph.", "Second paragraph.", "Third."]
        );
        assert_eq!(
            document
                .paragraphs
                .iter()
                .map(|p| p.ordinal)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            document.paragraphs[2].terminator,
            RtfParagraphBreak::EndOfStream,
            "a writer need not put `\\par` after the last paragraph"
        );
    }

    /// **The counter advances through a destination this reader does not read.**
    #[test]
    fn a_paragraph_inside_a_skipped_destination_still_moves_the_counter() {
        let document = read_body(r"Body one.\par {\footer A footer\par }Body two.");
        assert_eq!(texts(&document), vec!["Body one.", "Body two."]);
        assert_eq!(
            document.paragraphs[1].ordinal, 3,
            "the footer's own `\\par` is a paragraph the stream delimits"
        );
        assert_eq!(document.destinations_not_read, 1);
    }

    /// Each terminator is recorded, and a table's cells are their own paragraphs.
    #[test]
    fn a_table_cell_is_its_own_paragraph_and_says_so() {
        let document = read_body(r"\trowd\intbl Left\cell Right\cell \row Below.");
        assert_eq!(texts(&document), vec!["Left", "Right", "Below."]);
        assert_eq!(document.paragraphs[0].terminator, RtfParagraphBreak::Cell);
        assert_eq!(document.paragraphs[1].terminator, RtfParagraphBreak::Cell);
        assert_eq!(
            document.paragraphs[2].terminator,
            RtfParagraphBreak::EndOfStream
        );
    }

    /// `\row` is a terminator in its own right, and it is the one a real stream rarely reaches:
    /// a producer writes `\cell\row`, so the row's own paragraph is usually empty and mints no
    /// node. Exercised here so the variant is not a spelling nothing ever produces.
    #[test]
    fn text_between_the_last_cell_and_the_row_ends_the_paragraph_at_the_row() {
        let document = read_body(r"\trowd\intbl Left\cell Trailing\row After.");
        assert_eq!(texts(&document), vec!["Left", "Trailing", "After."]);
        assert_eq!(document.paragraphs[1].terminator, RtfParagraphBreak::Row);
    }

    #[test]
    fn a_section_break_ends_the_paragraph_before_it() {
        let document = read_body(r"Before.\sect After.");
        assert_eq!(texts(&document), vec!["Before.", "After."]);
        assert_eq!(
            document.paragraphs[0].terminator,
            RtfParagraphBreak::Section
        );
    }

    /// **`\page` is a page break, and it is not a page.**
    #[test]
    fn a_page_break_contributes_no_character_and_no_page() {
        let document = read_body(r"Before the break.\par\page After it.");
        assert_eq!(texts(&document), vec!["Before the break.", "After it."]);
    }

    /// An empty paragraph is not a node, for the reason a `<w:r>` with no `<w:t>` is not.
    #[test]
    fn an_empty_paragraph_is_not_a_node_and_still_holds_its_position() {
        let document = read_body(r"\par\par Third.");
        assert_eq!(texts(&document), vec!["Third."]);
        assert_eq!(document.paragraphs[0].ordinal, 3);
    }

    // ---------------------------------------------------------------------------------------
    // Transparent groups and destinations
    // ---------------------------------------------------------------------------------------

    #[test]
    fn a_formatting_group_is_transparent() {
        let document = read_body(r"The {\b important} part.");
        assert_eq!(texts(&document), vec!["The important part."]);
        assert_eq!(document.destinations_not_read, 0);
    }

    #[test]
    fn a_nested_formatting_group_is_transparent() {
        let document = read_body(r"{{\b a}{\i b}c}");
        assert_eq!(texts(&document), vec!["abc"]);
    }

    /// **A group this list does not name is declared, never spliced.**
    #[test]
    fn an_unrecognised_destination_is_skipped_and_counted() {
        let document =
            read_body(r"Body.{\fonttbl{\f0 Times New Roman;}}{\madeupdest Hidden words}");
        assert_eq!(texts(&document), vec!["Body."]);
        assert_eq!(
            document.destinations_not_read, 2,
            "one per top-level destination that held characters"
        );
    }

    /// A destination that held nothing declares nothing.
    #[test]
    fn an_empty_destination_declares_no_erasure() {
        let document = read_body(r"Body.{\stylesheet }");
        assert_eq!(texts(&document), vec!["Body."]);
        assert_eq!(document.destinations_not_read, 0);
    }

    /// `\*` is the format's own marker, honoured without consulting the list.
    #[test]
    fn an_ignorable_destination_is_skipped() {
        let document = read_body(r"Body.{\*\generator Some Writer 1.0;}");
        assert_eq!(texts(&document), vec!["Body."]);
        assert_eq!(document.destinations_not_read, 1);
    }

    /// A field's instruction **and** its cached result are both a destination's, not the body's.
    #[test]
    fn a_field_is_skipped_rather_than_evaluated() {
        let document = read_body(r"Page {\field{\*\fldinst PAGE }{\fldrslt 17}} of many.");
        assert_eq!(texts(&document), vec!["Page  of many."]);
        assert!(!texts(&document).concat().contains("17"));
        assert_eq!(document.destinations_not_read, 1);
    }

    /// A picture's data is skipped, and `\bin` bytes cannot re-enter the stream as text.
    #[test]
    fn a_pictures_binary_data_is_skipped() {
        let stream = b"{\\rtf1 Body.{\\pict\\pngblip\\bin4 {}\\a}}".to_vec();
        let document = read(&stream).expect("the stream reads");
        assert_eq!(texts(&document), vec!["Body."]);
        assert_eq!(document.destinations_not_read, 1);
    }

    // ---------------------------------------------------------------------------------------
    // Characters
    // ---------------------------------------------------------------------------------------

    #[test]
    fn the_special_character_control_words_are_the_characters_they_name() {
        let document = read_body(r"a\tab b\emdash c\ldblquote d\rdblquote e\bullet f");
        assert_eq!(texts(&document), vec!["a\tb—c“d”e•f"]);
    }

    #[test]
    fn escaped_braces_and_backslashes_are_text() {
        let document = read_body(r"\{a\\b\}");
        assert_eq!(texts(&document), vec!["{a\\b}"]);
    }

    #[test]
    fn one_space_ends_a_control_word_and_a_second_is_text() {
        let document = read_body(r"a\b  b");
        assert_eq!(texts(&document), vec!["a b"]);
    }

    #[test]
    fn a_unicode_escape_is_the_scalar_it_names() {
        let document = read_body(r"caf\u233 ?");
        assert_eq!(texts(&document), vec!["café"]);
    }

    /// A negative `\u` is RTF's signed 16-bit spelling, and pairing surrogates is reading it.
    #[test]
    fn a_surrogate_pair_is_one_scalar() {
        let document = read_body(r"\u-10179 ?\u-8704 ?");
        assert_eq!(texts(&document), vec!["\u{1F600}"]);
    }

    /// `\ucN` says how many fallback characters follow, and reading them would duplicate.
    #[test]
    fn the_stated_fallback_length_is_honoured() {
        let document = read_body(r"\uc3 x\u233 ???y");
        assert_eq!(texts(&document), vec!["xéy"]);
    }

    /// **A byte above 0x7F is declared, never guessed.**
    #[test]
    fn an_undecodable_byte_is_counted_rather_than_rendered() {
        let document = read_body(r"caf\'e9");
        assert_eq!(texts(&document), vec!["caf"]);
        assert_eq!(
            document.undecodable_bytes, 1,
            "its meaning depends on a code page this reader does not read"
        );
    }

    #[test]
    fn a_hex_escape_below_0x80_is_the_ascii_character_it_names() {
        let document = read_body(r"a\'26b");
        assert_eq!(
            texts(&document),
            vec!["a&b"],
            "0x26 is `&` in every ANSI code page, so reading it is reading"
        );
    }

    #[test]
    fn a_line_feed_in_the_source_is_not_a_paragraph_break() {
        let document = read(b"{\\rtf1 one\r\ntwo}").expect("the stream reads");
        assert_eq!(texts(&document), vec!["onetwo"]);
    }

    // ---------------------------------------------------------------------------------------
    // Detection and refusals
    // ---------------------------------------------------------------------------------------

    #[test]
    fn detection_reads_the_bytes() {
        assert!(is_rtf(b"{\\rtf1\\ansi}"));
        assert!(!is_rtf(b"{"), "an open brace alone is not RTF");
        assert!(!is_rtf(b"{\\ansi}"), "and neither is any other first word");
        assert!(
            !is_rtf(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]),
            "an OLE compound file is a legacy `.doc`, a different format with a different reader"
        );
        assert!(!is_rtf(b"PK\x03\x04"), "and a ZIP is a package");
        assert!(!is_rtf(b"%PDF-1.7"));
        assert!(!is_rtf(b""));
    }

    #[test]
    fn a_stream_that_is_not_rtf_is_refused_by_name() {
        let error = read(b"not rtf").expect_err("refused");
        assert!(error.to_string().contains("{\\rtf"), "{error}");
    }

    #[test]
    fn a_truncated_stream_is_refused() {
        assert!(read(b"{\\rtf1 open forever").is_err());
        assert!(read(b"{\\rtf1 }}").is_err(), "and so is an extra close");
        assert!(
            read(b"{\\rtf1 \\").is_err(),
            "and a bare trailing backslash"
        );
    }

    #[test]
    fn a_bin_past_the_end_of_the_stream_is_refused() {
        let error = read(b"{\\rtf1{\\pict\\bin9999 ab}}").expect_err("refused");
        assert!(error.to_string().contains("truncated"), "{error}");
    }

    #[test]
    fn an_unsupported_major_version_is_refused_by_name() {
        let error = read(b"{\\rtf2 body}").expect_err("refused");
        let text = error.to_string();
        assert!(text.contains("version"), "{text}");
        assert!(!text.contains("%PDF-"), "{text}");
    }

    #[test]
    fn groups_nested_past_the_cap_are_refused() {
        let deep = format!("{{\\rtf1{}{}", "{".repeat(300), "}".repeat(300));
        assert!(read(deep.as_bytes()).is_err());
    }

    /// **v2-S9.1: a wrapped erasure count is a silent drop presented as a success.**
    ///
    /// RTF is the format with no per-part bound at all — one brace-group stream, as long as the
    /// file is — so its single counter is the one an ordinary large document could reach.
    #[test]
    fn an_rtf_erasure_count_saturates_rather_than_wrapping() {
        let document = read_body(r"Body one.\par {\footer A footer\par }Body two.");
        assert_eq!(document.destinations_not_read, 1, "the footer held text");

        let mut folded = u32::MAX - 1;
        for _ in 0..3 {
            folded = crate::declare(folded, document.destinations_not_read);
        }
        assert_eq!(
            folded,
            u32::MAX,
            "the ceiling, not the small number a wrap would report"
        );
    }
}
