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

//! Auto-tagging, the writer's side: the page's bytes decoded strictly and tokenised with byte
//! positions (docs/23-AUTO-TAGGING-SCOPE.md §3.5; docs/24-AUTO-TAGGING-MILESTONES.md S2 items 2
//! and 3).
//!
//! # The two rules this module holds, and why
//!
//! The writer of docs/23 inserts `BDC` and `EMC` operators into a page's content at token
//! boundaries and re-encodes nothing (§3.5). That is safe only if two things are true of the
//! bytes it edits: they are the bytes `extract` interpreted, and every one of them sits in a token
//! whose position is known. Both are decided here, before any tag is written, and a page that
//! fails either is refused by name.
//!
//! **The bytes are read by this module, not through `get_page_content`.** That helper appends a
//! stream's still-encoded bytes when a decode fails and skips a `/Contents` entry that is not a
//! stream, silently either way — and `lopdf`'s own inflate returns the partial output of a
//! corrupt or truncated Flate stream as a success. [`page_content_strict`] resolves `/Contents`
//! itself, refuses an entry that is not a stream, inflates `FlateDecode` with a decoder that
//! requires the deflate data to end exactly where the input ends with its check intact, refuses
//! every other filter by name, and joins the streams with the `\n` `get_page_content` uses, so the
//! buffer equals the one `extract` interpreted on every page where that helper decoded cleanly.
//! `LZWDecode` and `ASCII85Decode`, which the scope routed through `lopdf`, are refused as well:
//! read closely, `lopdf` 0.44.0's LZW loop logs a decode error and returns what it had, and its
//! ASCII85 loop stops at the first byte outside the alphabet and returns what it had, so neither
//! can be relied on to fail, and this module decodes neither itself.
//!
//! **Tokenised with positions, every byte accounted for.** [`tokenise`] mirrors the grammar
//! `lopdf::content::Content::decode` accepts — `parser/mod.rs` of lopdf 0.44.0, rule for rule,
//! including the six-byte whitespace class inside arrays against the four-byte class between
//! operations, references inside arrays and not outside them, and prefix matches on `true`,
//! `false` and `null` — and returns the byte span of every operation. `lopdf`'s decoder is
//! lenient: it stops at the first operation it cannot parse and drops the remainder without a
//! word. The tokeniser refuses a buffer it cannot place to the last byte, and
//! [`agrees_with_lopdf`] then requires its operator sequence to equal `lopdf`'s, count and names
//! in order. Together the two prove that `lopdf` parsed the whole page, which is what makes an
//! operator index into its decode mean the same thing on both sides. An inline image is one
//! opaque token whose data ends by `lopdf`'s own rules — the computed length for unfiltered data
//! in a colour space it names, otherwise the first whitespace-`EI`-whitespace window — so the two
//! resynchronise at the same byte by construction and no inserted operator can land inside image
//! data.
//!
//! # Measured, 2026-09-17
//!
//! Over every PDF in `fixtures/engine` (49), `fixtures/gate` (8) and the oracle fixtures (35, of
//! which 3 exist not to open: a corrupt header, a non-PDF, a password) — 89 documents, 1 567
//! pages, 76 646 614 bytes of decoded content — the strict decoder returned `get_page_content`'s
//! bytes on all 1 567 pages and refused none: no page in these corpora carries a filter other than
//! none or `FlateDecode`, a predictor, a truncated stream or bytes after its deflate data. The
//! tokeniser placed every byte of all 1 567 pages, agreed with `lopdf` on all 6 697 547
//! operations, and refused none; `lopdf`'s strict parse succeeded on every one of them, so no page
//! in these corpora is one its lenient decoder truncates. Every refusal therefore has a unit test
//! and no corpus example yet; the two corpus tests at the end of this file print the census and
//! hold those numbers as floors.
//!
//! # What is here and what is not yet
//!
//! The strict decoder, the tokeniser, [`OpKind`] — the per-operator classification the placement
//! rule reads — and, since S2 item 4, the placement rule itself ([`plan_page`]), the splice that
//! inserts its sequences at token boundaries ([`splice`]), the tree and the stamp ([`write_tree`],
//! [`stamp_tags`]), with [`sequences_are_well_formed`] as the one statement of what a sequence may
//! hold, shared by the planner's tests and the self-check. `write_tags` and the self-check are
//! items 5 to 8 and are not here yet; nothing here is public.

use ethos_parser_core::EngineError;
use lopdf::{Object, ObjectId, Stream};

// ---------------------------------------------------------------------------------------------
// The strict decoder
// ---------------------------------------------------------------------------------------------

/// How many non-stream objects a `/Contents` reference may pass through before the chain is
/// refused: `lopdf`'s own `Document::DEREF_LIMIT`, so a chain this follows is one
/// `get_page_contents` follows too, and the buffers cannot differ on it.
const CONTENTS_DEREF_LIMIT: usize = 128;

/// The page's content, decoded strictly, as one buffer the interpreter ran over.
///
/// Resolves `/Contents` as `get_page_contents` does — a reference, followed through non-stream
/// objects up to [`CONTENTS_DEREF_LIMIT`] times, or an array of references — and refuses where
/// that helper skips: an entry that does not name a stream is `Malformed`. Each stream is decoded
/// by [`decode_stream_strict`]; the results are joined with one `\n` after each, exactly as
/// `get_page_content` joins them. A page without `/Contents` is an empty buffer, as it is there.
///
/// # Errors
///
/// - [`EngineError::Malformed`], naming the page object: the page dictionary does not resolve; a
///   `/Contents` entry is not a reference to a stream, names an object the document does not
///   hold, or is reached through too many references; a Flate stream is truncated, corrupt, fails
///   its check, or carries bytes after the end of its deflate data.
/// - [`EngineError::Unsupported`], naming the page object and the filter: a filter other than none
///   or `FlateDecode`; a `/Filter` that is not a name or a one-name array; a `/DecodeParms` that
///   is not a direct dictionary, or one whose `/Predictor` is greater than 1.
pub(crate) fn page_content_strict(
    doc: &lopdf::Document,
    page_id: ObjectId,
) -> Result<Vec<u8>, EngineError> {
    let page = format!("page object {} {}", page_id.0, page_id.1);
    let malformed = |detail: String| EngineError::Malformed {
        what: "content stream".into(),
        detail: format!("{page}: {detail}"),
    };
    let dict = doc
        .get_dictionary(page_id)
        .map_err(|e| malformed(format!("the page dictionary does not resolve: {e}")))?;
    let Ok(contents) = dict.get(b"Contents") else {
        return Ok(Vec::new());
    };
    let ids = content_stream_ids(doc, contents).map_err(malformed)?;

    let mut out = Vec::new();
    for id in ids {
        let stream = doc
            .get_object(id)
            .ok()
            .and_then(|o| o.as_stream().ok())
            .ok_or_else(|| {
                malformed(format!("/Contents entry {} {} is not a stream", id.0, id.1))
            })?;
        let decoded = decode_stream_strict(stream).map_err(|refusal| match refusal {
            StreamRefusal::Malformed(detail) => {
                malformed(format!("stream {} {}: {detail}", id.0, id.1))
            }
            StreamRefusal::Unsupported(detail) => EngineError::Unsupported {
                what: "content stream filter".into(),
                detail: format!("{page}, stream {} {}: {detail}", id.0, id.1),
            },
        })?;
        out.extend_from_slice(&decoded);
        out.push(b'\n');
    }
    Ok(out)
}

/// The stream ids a page's `/Contents` names, resolved as `get_page_contents` resolves them and
/// refused wherever it would silently skip.
fn content_stream_ids(doc: &lopdf::Document, contents: &Object) -> Result<Vec<ObjectId>, String> {
    let mut contents = contents;
    let mut hops = 0usize;
    loop {
        match contents {
            Object::Reference(id) => match doc.objects.get(id) {
                Some(Object::Stream(_)) => return Ok(vec![*id]),
                None => {
                    return Err(format!(
                        "/Contents names object {} {}, which the document does not hold",
                        id.0, id.1
                    ))
                }
                Some(other) => {
                    hops += 1;
                    if hops >= CONTENTS_DEREF_LIMIT {
                        return Err(format!(
                            "/Contents passes through {CONTENTS_DEREF_LIMIT} or more references \
                             without reaching a stream"
                        ));
                    }
                    contents = other;
                }
            },
            Object::Array(entries) => {
                let mut ids = Vec::with_capacity(entries.len());
                for (i, entry) in entries.iter().enumerate() {
                    match entry {
                        Object::Reference(id) => ids.push(*id),
                        other => {
                            return Err(format!(
                                "/Contents entry {i} is {} rather than a reference to a stream",
                                other.enum_variant()
                            ))
                        }
                    }
                }
                return Ok(ids);
            }
            other => {
                return Err(format!(
                    "/Contents is {} rather than a reference to a stream or an array of them",
                    other.enum_variant()
                ))
            }
        }
    }
}

/// Why one stream was not decoded: the document is wrong, or this decoder does not cover it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum StreamRefusal {
    Malformed(String),
    Unsupported(String),
}

/// One content stream's bytes as the interpreter saw them, or a refusal by name.
///
/// No `/Filter` is the raw bytes. `FlateDecode` goes through [`inflate_strict`]. Everything else
/// is refused: `LZWDecode` and `ASCII85Decode` because `lopdf`'s decoders return partial output
/// on a failure (module header), a filter array of any length but one because a chain is not
/// decoded here, and any other name by that name. A `/DecodeParms` is read as `lopdf` reads it —
/// a direct dictionary, `/Predictor` defaulting to 1 — and a predictor greater than 1 is refused
/// because no predictor is undone here; a `/DecodeParms` of any other shape is refused rather than
/// ignored, because what it carries cannot be said without resolving it.
fn decode_stream_strict(stream: &Stream) -> Result<Vec<u8>, StreamRefusal> {
    use StreamRefusal::{Malformed, Unsupported};

    let filter: Option<&[u8]> = match stream.dict.get(b"Filter") {
        Err(_) => None,
        Ok(Object::Name(name)) => Some(name.as_slice()),
        Ok(Object::Array(names)) => match names.as_slice() {
            [Object::Name(name)] => Some(name.as_slice()),
            [] => {
                return Err(Unsupported(
                    "/Filter is an empty array, which lopdf decodes to no bytes at all".into(),
                ))
            }
            [other] => {
                return Err(Unsupported(format!(
                    "/Filter is a one-entry array holding {} rather than a name",
                    other.enum_variant()
                )))
            }
            more => {
                return Err(Unsupported(format!(
                    "/Filter chains {} filters, and only a single filter is decoded here",
                    more.len()
                )))
            }
        },
        Ok(other) => {
            return Err(Unsupported(format!(
                "/Filter is {} rather than a name or an array of one name",
                other.enum_variant()
            )))
        }
    };
    let Some(filter) = filter else {
        return Ok(stream.content.clone());
    };

    match stream.dict.get(b"DecodeParms") {
        Err(_) => {}
        Ok(Object::Dictionary(parms)) => {
            let predictor = parms
                .get(b"Predictor")
                .and_then(Object::as_i64)
                .unwrap_or(1);
            if predictor > 1 {
                return Err(Unsupported(format!(
                    "/DecodeParms /Predictor {predictor}: no predictor is undone here"
                )));
            }
        }
        Ok(other) => {
            return Err(Unsupported(format!(
                "/DecodeParms is {} rather than a direct dictionary",
                other.enum_variant()
            )))
        }
    }

    match filter {
        b"FlateDecode" => inflate_strict(&stream.content).map_err(Malformed),
        b"LZWDecode" | b"ASCII85Decode" => Err(Unsupported(format!(
            "/{}: lopdf's decoder returns the partial output of a stream it could not decode to \
             the end as a success, so it cannot be relied on to fail, and this writer decodes \
             neither filter itself",
            String::from_utf8_lossy(filter)
        ))),
        other => Err(Unsupported(format!("/{}", String::from_utf8_lossy(other)))),
    }
}

/// Inflate a zlib stream, refusing anything short of a complete one.
///
/// Driven through `flate2::Decompress` rather than `flate2::read::ZlibDecoder`, and the choice is
/// load-bearing: the reader returns `Ok(0)` at end of input whenever the inflater reports that it
/// needs more, so a stream cut in the middle reads to end "successfully" with every input byte
/// consumed, and neither an error nor `total_in` says anything happened. The low-level status
/// does: the loop ends only on `StreamEnd`, which miniz reports after the Adler-32 check passed,
/// and a call that consumed nothing and produced nothing before that is the cut. Bytes left after
/// `StreamEnd` are refused too, because `lopdf` and this decoder would otherwise agree on the
/// content while the document carries bytes nobody read.
///
/// An empty input is an empty output, as it is in `lopdf`, which does not run the inflater on
/// zero bytes; there is nothing to decode and nothing to tokenise.
fn inflate_strict(input: &[u8]) -> Result<Vec<u8>, String> {
    use flate2::{Decompress, FlushDecompress, Status};

    if input.is_empty() {
        return Ok(Vec::new());
    }
    let mut inflater = Decompress::new(true);
    let mut out: Vec<u8> = Vec::with_capacity(input.len().saturating_mul(4).max(4096));
    loop {
        let consumed = usize::try_from(inflater.total_in())
            .map_err(|_| "the inflater consumed more than the address space holds".to_string())?;
        let remaining = input.get(consumed..).unwrap_or(&[]);
        if out.len() == out.capacity() {
            out.reserve(out.capacity().max(4096));
        }
        let (before_in, before_out) = (inflater.total_in(), inflater.total_out());
        let flush = if remaining.is_empty() {
            FlushDecompress::Finish
        } else {
            FlushDecompress::None
        };
        let status = inflater
            .decompress_vec(remaining, &mut out, flush)
            .map_err(|e| format!("the deflate data is corrupt or its check fails: {e}"))?;
        match status {
            Status::StreamEnd => break,
            Status::Ok | Status::BufError => {
                if inflater.total_in() == before_in && inflater.total_out() == before_out {
                    return Err(format!(
                        "the inflater stopped after {} of {} input byte(s) without reaching the \
                         end of the deflate data",
                        inflater.total_in(),
                        input.len()
                    ));
                }
            }
        }
    }
    let consumed = inflater.total_in();
    let len = input.len() as u64;
    if consumed != len {
        return Err(format!(
            "{} byte(s) follow the end of the deflate data, out of {len}",
            len - consumed
        ));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// The tokeniser
// ---------------------------------------------------------------------------------------------

/// `lopdf::reader::MAX_NESTING_DEPTH`: how deep arrays and dictionaries may nest before its
/// parser fails the whole stream rather than the one operand.
const MAX_NESTING_DEPTH: usize = 100;

/// `lopdf::reader::MAX_BRACKET`: how deep parentheses may nest inside a literal string before
/// the string stops parsing.
const MAX_BRACKET: usize = 100;

/// One operation of a content stream, by byte position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Op {
    /// The operator token as `lopdf` reports it — `Tj`, `T*`, `'`, `"`, and `BI` for a whole
    /// inline image.
    pub(crate) operator: String,
    /// The offset of the operation's first token: its first operand, or the operator when it has
    /// none. Whitespace and comments before it are not included.
    pub(crate) start: usize,
    /// The offset just after the operator token — after `EI` for an inline image. The
    /// whitespace that follows is not included.
    pub(crate) end: usize,
}

/// A content stream's operations with their positions, every byte of the buffer placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tokenised {
    /// In stream order. Between one operation's `end` and the next one's `start` lie only
    /// whitespace and comments.
    pub(crate) ops: Vec<Op>,
}

/// Tokenise a page's content buffer, placing every byte or refusing.
///
/// # Errors
///
/// [`EngineError::Unsupported`] with `what` = `content stream tokeniser`, naming the byte offset,
/// in two shapes. The tail: the buffer holds an operation `lopdf`'s grammar cannot parse, so its
/// decoder would stop there and drop the rest silently — the detail names where that operation
/// starts and where the parse stuck. The failure: a shape `lopdf` fails the whole stream on — an
/// inline image whose dictionary is not followed by `ID`, whose data of the computed length is
/// not followed by `EI`, or after whose data no `EI` window exists; nesting past
/// [`MAX_NESTING_DEPTH`]; an inline image dictionary without a colour space, on which `lopdf`'s
/// parser panics rather than fails.
pub(crate) fn tokenise(bytes: &[u8]) -> Result<Tokenised, EngineError> {
    let lx = Lexer { bytes };
    let refuse = |detail: String| EngineError::Unsupported {
        what: "content stream tokeniser".into(),
        detail,
    };
    let mut ops = Vec::new();
    // `_content`: content_space, many0(operation), many0(terminated(comment, content_space)).
    let mut pos = lx.content_space(0);
    let stuck;
    loop {
        match lx.operation(pos) {
            Err(fail) => {
                return Err(refuse(format!(
                    "byte {} of {}: {}; lopdf's decoder fails the whole page here",
                    fail.at,
                    bytes.len(),
                    fail.why
                )))
            }
            Ok(Ok((op, next))) => {
                ops.push(op);
                pos = next;
            }
            // nom's `many0` restores the input to before the operation it could not parse.
            Ok(Err(at)) => {
                stuck = at;
                break;
            }
        }
    }
    let mut tail = pos;
    while let Some(next) = lx.comment(tail) {
        tail = lx.content_space(next);
    }
    if tail != bytes.len() {
        return Err(refuse(format!(
            "byte {tail} of {}: the operation starting there cannot be parsed past byte {stuck}, \
             and lopdf's decoder stops at the same place and drops the remaining {} byte(s) \
             without a word",
            bytes.len(),
            bytes.len() - tail
        )));
    }
    Ok(Tokenised { ops })
}

/// Require the tokeniser's operator sequence to be `lopdf`'s: same count, same names in order.
///
/// # Errors
///
/// [`EngineError::Unsupported`] with `what` = `content stream tokeniser`, naming the first index
/// at which the two differ and the two operators there — or the count, when one side ends first.
pub(crate) fn agrees_with_lopdf(
    t: &Tokenised,
    ops: &[lopdf::content::Operation],
) -> Result<(), EngineError> {
    let refuse = |detail: String| EngineError::Unsupported {
        what: "content stream tokeniser".into(),
        detail,
    };
    for (i, (mine, theirs)) in t.ops.iter().zip(ops).enumerate() {
        if mine.operator != theirs.operator {
            return Err(refuse(format!(
                "operation {i}: the tokeniser reads `{}` where lopdf reads `{}`",
                mine.operator, theirs.operator
            )));
        }
    }
    if t.ops.len() != ops.len() {
        let i = t.ops.len().min(ops.len());
        return Err(refuse(format!(
            "operation {i}: the tokeniser reads {} operation(s) and lopdf {}, the first {i} \
             agreeing",
            t.ops.len(),
            ops.len()
        )));
    }
    Ok(())
}

/// A point past which `lopdf`'s parser would not recover either — nom's `Failure`, as opposed
/// to the `Error` a combinator backtracks over.
#[derive(Debug)]
struct Fail {
    at: usize,
    why: String,
}

/// nom's three outcomes: matched, ending at the offset; no match, so the caller backtracks; or a
/// failure the whole parse stops on.
type Step = Result<Option<usize>, Fail>;

/// A direct object's value, kept only as far as the inline-image rules read it.
#[derive(Debug, Clone, PartialEq)]
enum Val {
    Integer(i64),
    Boolean(bool),
    Name(Vec<u8>),
    Other,
}

/// The six whitespace bytes of PDF 32000-1 §7.2.2, which `lopdf`'s `space` and `white_space`
/// accept inside arrays, dictionaries and hex strings.
fn is_whitespace(c: u8) -> bool {
    b" \t\n\r\0\x0C".contains(&c)
}

fn is_delimiter(c: u8) -> bool {
    b"()<>[]{}/%".contains(&c)
}

fn is_regular(c: u8) -> bool {
    !is_whitespace(c) && !is_delimiter(c)
}

/// The grammar of `lopdf` 0.44.0's `parser/mod.rs`, transliterated with positions.
///
/// Every method is named for the nom parser it mirrors and takes the offset that parser would be
/// handed. A method returning `Option<usize>` is a parser that only ever fails with `Error`; one
/// returning [`Step`] can also fail with `Failure`.
struct Lexer<'a> {
    bytes: &'a [u8],
}

impl Lexer<'_> {
    fn at(&self, p: usize) -> Option<u8> {
        self.bytes.get(p).copied()
    }

    fn starts(&self, p: usize, tag: &[u8]) -> bool {
        self.bytes
            .get(p..)
            .is_some_and(|rest| rest.starts_with(tag))
    }

    /// `content_space`: the four bytes between operations and operands.
    fn content_space(&self, mut p: usize) -> usize {
        while matches!(self.at(p), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            p += 1;
        }
        p
    }

    /// `white_space`: the six bytes, as many as there are.
    fn white_space(&self, mut p: usize) -> usize {
        while self.at(p).is_some_and(is_whitespace) {
            p += 1;
        }
        p
    }

    fn eol(&self, p: usize) -> Option<usize> {
        if self.starts(p, b"\r\n") {
            Some(p + 2)
        } else if matches!(self.at(p), Some(b'\n' | b'\r')) {
            Some(p + 1)
        } else {
            None
        }
    }

    /// `comment`: `%`, anything but a line end, then a line end — which is required.
    fn comment(&self, p: usize) -> Option<usize> {
        if self.at(p) != Some(b'%') {
            return None;
        }
        let mut q = p + 1;
        while self.at(q).is_some_and(|c| c != b'\r' && c != b'\n') {
            q += 1;
        }
        self.eol(q)
    }

    /// `many0(comment)`.
    fn comments(&self, mut p: usize) -> usize {
        while let Some(next) = self.comment(p) {
            p = next;
        }
        p
    }

    /// `space`: whitespace runs and comments, in any order, possibly none.
    fn space(&self, mut p: usize) -> usize {
        loop {
            let q = self.white_space(p);
            if q > p {
                p = q;
                continue;
            }
            if let Some(next) = self.comment(p) {
                p = next;
                continue;
            }
            return p;
        }
    }

    /// `digit0`.
    fn digits(&self, mut p: usize) -> usize {
        while self.at(p).is_some_and(|c| c.is_ascii_digit()) {
            p += 1;
        }
        p
    }

    /// `opt(one_of("+-"))`.
    fn sign(&self, p: usize) -> usize {
        if matches!(self.at(p), Some(b'+' | b'-')) {
            p + 1
        } else {
            p
        }
    }

    /// `integer`: a sign and digits that parse as an `i64`; one that overflows does not match.
    fn integer(&self, p: usize) -> Option<usize> {
        let q = self.sign(p);
        let r = self.digits(q);
        if r == q {
            return None;
        }
        std::str::from_utf8(&self.bytes[p..r])
            .ok()?
            .parse::<i64>()
            .ok()?;
        Some(r)
    }

    /// `real`: a sign, then digits `.` digits-or-none, or `.` digits; parsed as `f32`.
    fn real(&self, p: usize) -> Option<usize> {
        let q = self.sign(p);
        let d = self.digits(q);
        let r = if d > q && self.at(d) == Some(b'.') {
            self.digits(d + 1)
        } else if self.at(q) == Some(b'.') {
            let e = self.digits(q + 1);
            if e == q + 1 {
                return None;
            }
            e
        } else {
            return None;
        };
        std::str::from_utf8(&self.bytes[p..r])
            .ok()?
            .parse::<f32>()
            .ok()?;
        Some(r)
    }

    /// `hex_char`: exactly two hex digits.
    fn hex_pair(&self, p: usize) -> Option<usize> {
        let pair = self.bytes.get(p..p + 2)?;
        pair.iter().all(|c| c.is_ascii_hexdigit()).then_some(p + 2)
    }

    /// `name`: `/` then any run of regular bytes, `#xx` escapes included; the name ends at a `#`
    /// that is not followed by two hex digits.
    fn name(&self, p: usize) -> Option<usize> {
        if self.at(p) != Some(b'/') {
            return None;
        }
        let mut q = p + 1;
        loop {
            match self.at(q) {
                Some(b'#') => match self.hex_pair(q + 1) {
                    Some(next) => q = next,
                    None => break,
                },
                Some(c) if is_regular(c) => q += 1,
                _ => break,
            }
        }
        Some(q)
    }

    /// The bytes a name at `p..end` denotes, `#xx` decoded, for the inline image dictionary.
    fn name_value(&self, p: usize, end: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(end - p);
        let mut q = p + 1;
        while q < end {
            if self.bytes[q] == b'#' {
                let hex = std::str::from_utf8(&self.bytes[q + 1..q + 3]).unwrap_or("00");
                out.push(u8::from_str_radix(hex, 16).unwrap_or(0));
                q += 3;
            } else {
                out.push(self.bytes[q]);
                q += 1;
            }
        }
        out
    }

    /// `escape_sequence`: `\` then one to three octal digits, a line end, or any one byte.
    fn escape(&self, p: usize) -> Option<usize> {
        if self.at(p) != Some(b'\\') {
            return None;
        }
        let q = p + 1;
        let mut r = q;
        while r < q + 3 && self.at(r).is_some_and(|c| (b'0'..=b'7').contains(&c)) {
            r += 1;
        }
        if r > q {
            return Some(r);
        }
        if let Some(next) = self.eol(q) {
            return Some(next);
        }
        self.at(q).map(|_| q + 1)
    }

    /// `inner_literal_string(depth)`: direct runs, escapes, line ends and nested strings, as
    /// many as match. Never fails; the caller checks for the closing paren.
    fn inner_literal(&self, mut p: usize, depth: usize) -> usize {
        loop {
            let mut q = p;
            while self.at(q).is_some_and(|c| !b"()\\\r\n".contains(&c)) {
                q += 1;
            }
            if q > p {
                p = q;
                continue;
            }
            if let Some(next) = self.escape(p) {
                p = next;
                continue;
            }
            if let Some(next) = self.eol(p) {
                p = next;
                continue;
            }
            // `nested_literal_string(depth)`: at depth 0 it matches nothing.
            if depth > 0 && self.at(p) == Some(b'(') {
                let r = self.inner_literal(p + 1, depth - 1);
                if self.at(r) == Some(b')') {
                    p = r + 1;
                    continue;
                }
            }
            return p;
        }
    }

    /// `literal_string`.
    fn literal_string(&self, p: usize) -> Option<usize> {
        if self.at(p) != Some(b'(') {
            return None;
        }
        let q = self.inner_literal(p + 1, MAX_BRACKET);
        (self.at(q) == Some(b')')).then_some(q + 1)
    }

    /// `hexadecimal_string`: `<`, hex digits each preceded by any whitespace, whitespace, `>`.
    fn hex_string(&self, p: usize) -> Option<usize> {
        if self.at(p) != Some(b'<') {
            return None;
        }
        let mut q = p + 1;
        loop {
            let w = self.white_space(q);
            if self.at(w).is_some_and(|c| c.is_ascii_hexdigit()) {
                q = w + 1;
            } else {
                break;
            }
        }
        let q = self.white_space(q);
        (self.at(q) == Some(b'>')).then_some(q + 1)
    }

    /// `boolean`: a prefix match on `true` or `false`, as `tag` matches.
    fn boolean(&self, p: usize) -> Option<(usize, bool)> {
        if self.starts(p, b"true") {
            Some((p + 4, true))
        } else if self.starts(p, b"false") {
            Some((p + 5, false))
        } else {
            None
        }
    }

    /// `null`: a prefix match.
    fn null(&self, p: usize) -> Option<usize> {
        self.starts(p, b"null").then_some(p + 4)
    }

    /// `reference`: two unsigned integers that fit an `ObjectId`, each followed by `space`, then
    /// `R`. Only reachable inside arrays and dictionaries, as in `lopdf`.
    fn reference(&self, p: usize) -> Option<usize> {
        let a = self.digits(p);
        if a == p {
            return None;
        }
        std::str::from_utf8(&self.bytes[p..a])
            .ok()?
            .parse::<u32>()
            .ok()?;
        let b = self.space(a);
        let c = self.digits(b);
        if c == b {
            return None;
        }
        std::str::from_utf8(&self.bytes[b..c])
            .ok()?
            .parse::<u16>()
            .ok()?;
        let d = self.space(c);
        (self.at(d) == Some(b'R')).then_some(d + 1)
    }

    /// `_direct_objects(depth)`: the alternatives in `lopdf`'s order.
    fn direct_objects(&self, p: usize, depth: usize) -> Result<Option<(usize, Val)>, Fail> {
        if let Some(n) = self.null(p) {
            return Ok(Some((n, Val::Other)));
        }
        if let Some((n, b)) = self.boolean(p) {
            return Ok(Some((n, Val::Boolean(b))));
        }
        if let Some(n) = self.reference(p) {
            return Ok(Some((n, Val::Other)));
        }
        if let Some(n) = self.real(p) {
            return Ok(Some((n, Val::Other)));
        }
        if let Some(n) = self.integer(p) {
            let v = std::str::from_utf8(&self.bytes[p..n])
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .map_or(Val::Other, Val::Integer);
            return Ok(Some((n, v)));
        }
        if let Some(n) = self.name(p) {
            return Ok(Some((n, Val::Name(self.name_value(p, n)))));
        }
        if let Some(n) = self.literal_string(p) {
            return Ok(Some((n, Val::Other)));
        }
        if let Some(n) = self.hex_string(p) {
            return Ok(Some((n, Val::Other)));
        }
        if let Some(n) = self.array(p, depth)? {
            return Ok(Some((n, Val::Other)));
        }
        if let Some(n) = self.dictionary(p, depth)? {
            return Ok(Some((n, Val::Other)));
        }
        Ok(None)
    }

    /// `_direct_object(depth)`: a failure at depth 0, else an object then `space`.
    fn direct_object(&self, p: usize, depth: usize) -> Result<Option<(usize, Val)>, Fail> {
        if depth == 0 {
            return Err(Fail {
                at: p,
                why: format!("arrays or dictionaries nested more than {MAX_NESTING_DEPTH} deep"),
            });
        }
        Ok(self
            .direct_objects(p, depth - 1)?
            .map(|(n, v)| (self.space(n), v)))
    }

    /// `array(depth)`: `[`, `space`, objects, `]`.
    fn array(&self, p: usize, depth: usize) -> Step {
        if self.at(p) != Some(b'[') {
            return Ok(None);
        }
        let mut q = self.space(p + 1);
        while let Some((n, _)) = self.direct_object(q, depth)? {
            q = n;
        }
        Ok((self.at(q) == Some(b']')).then_some(q + 1))
    }

    /// `_dictionary(depth)`: `<<`, `space`, pairs, `>>`.
    fn dictionary(&self, p: usize, depth: usize) -> Step {
        if !self.starts(p, b"<<") {
            return Ok(None);
        }
        let q = self.space(p + 2);
        let q = self.inner_dictionary(q, depth, None)?;
        Ok(self.starts(q, b">>").then_some(q + 2))
    }

    /// `inner_dictionary(depth)`: `name`, `space`, object pairs, as many as match; a key without
    /// a value ends the dictionary before the key. Entries are collected when asked, last value
    /// per key winning as `Dictionary::set` makes it.
    fn inner_dictionary(
        &self,
        mut p: usize,
        depth: usize,
        mut entries: Option<&mut Vec<(Vec<u8>, Val)>>,
    ) -> Result<usize, Fail> {
        loop {
            let Some(k) = self.name(p) else {
                return Ok(p);
            };
            let s = self.space(k);
            match self.direct_object(s, depth)? {
                Some((n, v)) => {
                    if let Some(entries) = entries.as_deref_mut() {
                        entries.push((self.name_value(p, k), v));
                    }
                    p = n;
                }
                None => return Ok(p),
            }
        }
    }

    /// `operand`: the alternatives in `lopdf`'s order — no reference at this level — then
    /// `content_space`.
    fn operand(&self, p: usize) -> Step {
        let end = if let Some(n) = self.null(p) {
            n
        } else if let Some((n, _)) = self.boolean(p) {
            n
        } else if let Some(n) = self.real(p) {
            n
        } else if let Some(n) = self.integer(p) {
            n
        } else if let Some(n) = self.name(p) {
            n
        } else if let Some(n) = self.literal_string(p) {
            n
        } else if let Some(n) = self.hex_string(p) {
            n
        } else if let Some(n) = self.array(p, MAX_NESTING_DEPTH)? {
            n
        } else if let Some(n) = self.dictionary(p, MAX_NESTING_DEPTH)? {
            n
        } else {
            return Ok(None);
        };
        Ok(Some(self.content_space(end)))
    }

    /// `operator`: one or more of the ASCII letters, `*`, `'` and `"`.
    fn operator(&self, p: usize) -> Option<usize> {
        let mut q = p;
        while self
            .at(q)
            .is_some_and(|c| c.is_ascii_alphabetic() || b"*'\"".contains(&c))
        {
            q += 1;
        }
        (q > p).then_some(q)
    }

    /// `operation`: comments, then an inline image or operands and an operator, then
    /// `content_space`. `Ok(Ok)` is the operation and the offset after it; `Ok(Err)` is where
    /// the parse stuck on an operation it could not read, the caller having to backtrack to
    /// before the comments as `many0` does.
    fn operation(&self, p: usize) -> Result<Result<(Op, usize), usize>, Fail> {
        let start = self.comments(p);
        if let Some(end) = self.inline_image(start)? {
            let op = Op {
                operator: "BI".into(),
                start,
                end,
            };
            return Ok(Ok((op, self.content_space(end))));
        }
        let mut q = start;
        while let Some(n) = self.operand(q)? {
            q = n;
        }
        match self.operator(q) {
            Some(end) => {
                let op = Op {
                    operator: String::from_utf8_lossy(&self.bytes[q..end]).into_owned(),
                    start,
                    end,
                };
                Ok(Ok((op, self.content_space(end))))
            }
            None => Ok(Err(q)),
        }
    }

    /// `inline_image`: `BI`, `content_space`, then the image under `cut` — from here every
    /// non-match is a failure of the whole stream. Returns the offset just after `EI`.
    fn inline_image(&self, p: usize) -> Step {
        if !self.starts(p, b"BI") {
            return Ok(None);
        }
        let q = self.content_space(p + 2);
        let mut entries: Vec<(Vec<u8>, Val)> = Vec::new();
        let q = self.inner_dictionary(q, MAX_NESTING_DEPTH, Some(&mut entries))?;
        if !self.starts(q, b"ID") {
            return Err(Fail {
                at: q,
                why: "an inline image's dictionary is not followed by ID".into(),
            });
        }
        let data = self.content_space(q + 2);
        match self.image_data_end(&entries, data)? {
            // `image_data_stream` succeeded: `content_space`, `EI`, `content_space`.
            Some(after_data) => {
                let r = self.content_space(after_data);
                if !self.starts(r, b"EI") {
                    return Err(Fail {
                        at: r,
                        why: format!(
                            "an inline image's data of the computed length ({} byte(s)) is not \
                             followed by EI",
                            after_data - data
                        ),
                    });
                }
                Ok(Some(r + 2))
            }
            // `image_data_stream` failed: the first window of whitespace, `E`, `I`, whitespace
            // after the data, consumed through the `I`.
            None => {
                let rest = &self.bytes[data..];
                let found = rest.windows(4).position(|w| {
                    matches!(w[0], b' ' | b'\n' | b'\r')
                        && w[1] == b'E'
                        && w[2] == b'I'
                        && matches!(w[3], b' ' | b'\n' | b'\r')
                });
                match found {
                    Some(i) => Ok(Some(data + i + 3)),
                    None => Err(Fail {
                        at: data,
                        why: "an inline image whose data lopdf cannot size has no \
                              whitespace-EI-whitespace window after it"
                            .into(),
                    }),
                }
            }
        }
    }

    /// `image_data_stream`'s decision: `Some(end)` when `lopdf` sizes the data from the
    /// dictionary and the buffer holds that many bytes, `None` when any of its lookups fails and
    /// it falls back to scanning. The order of the lookups is `lopdf`'s, because the one shape it
    /// neither sizes nor scans — no colour space on an image that is not a mask, on which its
    /// parser panics — is only reached after the width, height and depth have been read.
    fn image_data_end(
        &self,
        entries: &[(Vec<u8>, Val)],
        data: usize,
    ) -> Result<Option<usize>, Fail> {
        let get = |key: &[u8]| entries.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v);
        let get_abbr = |abbr: &[u8], key: &[u8]| get(abbr).or_else(|| get(key));
        let as_usize = |v: Option<&Val>| match v {
            Some(Val::Integer(i)) => Some(*i),
            _ => None,
        };
        let Some(width) = as_usize(get_abbr(b"W", b"Width")) else {
            return Ok(None);
        };
        let Some(height) = as_usize(get_abbr(b"H", b"Height")) else {
            return Ok(None);
        };
        let Some(bpc) = as_usize(get_abbr(b"BPC", b"BitsPerComponent")) else {
            return Ok(None);
        };
        let colours: usize = match get_abbr(b"IM", b"ImageMask") {
            Some(Val::Boolean(true)) => 1,
            _ => match get_abbr(b"CS", b"ColorSpace") {
                None => {
                    return Err(Fail {
                        at: data,
                        why: "an inline image with neither a colour space nor /IM true; lopdf's \
                              parser panics on this shape rather than failing"
                            .into(),
                    })
                }
                Some(Val::Name(cs)) => match cs.as_slice() {
                    b"DeviceGray" | b"Gray" => 1,
                    b"DeviceRGB" | b"RGB" => 3,
                    b"DeviceRGBA" | b"RGBA" | b"DeviceCMYK" | b"CMYK" => 4,
                    _ => return Ok(None),
                },
                Some(_) => return Ok(None),
            },
        };
        // `as_i64()? as usize`: a negative dimension wraps to an enormous one in lopdf, and the
        // multiplication that follows overflows. lopdf panics there in a debug build and wraps in
        // a release one; neither can be mirrored, so the shape is refused.
        let (Ok(width), Ok(height), Ok(bpc)) = (
            usize::try_from(width),
            usize::try_from(height),
            usize::try_from(bpc),
        ) else {
            return Err(Fail {
                at: data,
                why: "an inline image with a negative width, height or bit depth, on which \
                      lopdf's length arithmetic overflows"
                    .into(),
            });
        };
        let length = colours
            .checked_mul(bpc)
            .and_then(|bits| bits.checked_mul(width))
            .map(|row_bits| row_bits.div_ceil(8))
            .and_then(|stride| stride.checked_mul(height))
            .ok_or_else(|| Fail {
                at: data,
                why: "an inline image whose dimensions overflow lopdf's length arithmetic".into(),
            })?;
        if get_abbr(b"F", b"Filter").is_some() {
            return Ok(None);
        }
        let end = data
            .checked_add(length)
            .filter(|&end| end <= self.bytes.len());
        Ok(end)
    }
}

// ---------------------------------------------------------------------------------------------
// The per-operator classification the placement rule reads
// ---------------------------------------------------------------------------------------------

/// A `BDC`'s property list as written, or a `BMC`'s absence of one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum MarkedProps {
    /// An inline dictionary, which may carry `/MCID`.
    Inline(lopdf::Dictionary),
    /// A name into the page's `/Properties`, which the writer resolves itself (scope §3.6).
    Named(Vec<u8>),
    /// `BMC`, or a `BDC` whose second operand is neither — read as the interpreter reads it, as a
    /// sequence with no id.
    None,
}

/// What an operator means to the placement rule of scope §3.4.
///
/// Only the distinctions that rule draws are drawn: what shows text, what paints, what opens and
/// closes the three nestings a sequence may not straddle, and what is an inline image. Everything
/// else — state, positioning, colour, path construction, clipping, the marked-content points, the
/// compatibility markers, the Type 3 metrics — draws nothing and is `Other`. The interpreter's
/// exhaustive dispatch is what refuses an operator outside Table A.1, so by the time this is
/// asked every operator has been named there.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OpKind {
    /// `Tj`, `TJ`, `'`, `"`.
    TextShow,
    /// `S`, `s`, `f`, `F`, `f*`, `B`, `B*`, `b`, `b*`, `sh`, `Do`: ink reaches the page.
    Painting,
    /// `BT`.
    BeginText,
    /// `ET`.
    EndText,
    /// `q`.
    Save,
    /// `Q`.
    Restore,
    /// `BMC` or `BDC`, with its tag and property list as written. The tag is empty when the
    /// first operand is not a name, which the interpreter reads as a sequence that is not an
    /// artifact.
    BeginMarked {
        /// The tag: `Artifact`, `Span`, `P`, an optional-content `OC`.
        tag: Vec<u8>,
        /// The property list.
        props: MarkedProps,
    },
    /// `EMC`.
    EndMarked,
    /// `BI`, the whole image as `lopdf` collapses it — and a stray `ID` or `EI`, which `lopdf`
    /// never emits from a well-formed image and which a sequence must not enclose either.
    InlineImage,
    /// Everything else: draws nothing.
    Other,
}

impl OpKind {
    /// Classify one decoded operation.
    pub(crate) fn of(op: &lopdf::content::Operation) -> Self {
        match op.operator.as_str() {
            "Tj" | "TJ" | "'" | "\"" => Self::TextShow,
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "sh" | "Do" => Self::Painting,
            "BT" => Self::BeginText,
            "ET" => Self::EndText,
            "q" => Self::Save,
            "Q" => Self::Restore,
            "BMC" | "BDC" => Self::BeginMarked {
                tag: match op.operands.first() {
                    Some(Object::Name(n)) => n.clone(),
                    _ => Vec::new(),
                },
                props: match (op.operator.as_str(), op.operands.get(1)) {
                    ("BDC", Some(Object::Dictionary(d))) => MarkedProps::Inline(d.clone()),
                    ("BDC", Some(Object::Name(n))) => MarkedProps::Named(n.clone()),
                    _ => MarkedProps::None,
                },
            },
            "EMC" => Self::EndMarked,
            "BI" | "ID" | "EI" => Self::InlineImage,
            _ => Self::Other,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The placement rule (scope §3.4): what a sequence may enclose, decided per operation
// ---------------------------------------------------------------------------------------------

/// The `(region, block)` pair the cut gave a run, as the artifact spells it (S2 item 4).
///
/// Keyed as given: both `None` on an unsubdivided single-band page is one block, and
/// `(Some(r), None)` is one block per band the rule declined to divide. The pair rather than
/// `block` alone, because a block is numbered within its band and two bands both have a block 1.
pub(crate) type BlockKey = (Option<u32>, Option<u32>);

/// Whose text one text-showing operation showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shows {
    /// No run: the string was empty, or the reader dropped every run the operator showed
    /// (`broken-font-encoding`). The engine did not read it, so it does not claim it.
    Nothing,
    /// Every run it showed is in this block.
    Block(BlockKey),
    /// Its runs lie inside an `/Artifact` frame — furniture the author put outside the
    /// structure (§14.8.2.2), which stays outside.
    Artifact,
}

/// The three nestings a sequence may not straddle, as they stand before one operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Nesting {
    /// The innermost open marked-content frame: the index of its `BMC`/`BDC`, or `None` at the
    /// top level.
    pub(crate) frame: Option<usize>,
    /// How many `q` are open. Signed, so a stray `Q` reads as a dip rather than a wrap.
    pub(crate) q: i32,
    /// How many `BT` are open. Signed for the same reason.
    pub(crate) bt: i32,
    /// Whether any open frame is an `/Artifact` — the whole stack, as the interpreter asks it.
    pub(crate) artifact: bool,
}

/// The nesting before every operation, and after the last one: `ops.len() + 1` entries.
///
/// An `EMC` with no open frame pops nothing, as the interpreter's does.
pub(crate) fn nesting(ops: &[lopdf::content::Operation]) -> Vec<Nesting> {
    let mut out = Vec::with_capacity(ops.len() + 1);
    let mut stack: Vec<(usize, bool)> = Vec::new();
    let mut state = Nesting::default();
    for (i, op) in ops.iter().enumerate() {
        out.push(state);
        match OpKind::of(op) {
            OpKind::Save => state.q += 1,
            OpKind::Restore => state.q -= 1,
            OpKind::BeginText => state.bt += 1,
            OpKind::EndText => state.bt -= 1,
            OpKind::BeginMarked { tag, .. } => stack.push((i, tag == b"Artifact")),
            OpKind::EndMarked => {
                stack.pop();
            }
            _ => {}
        }
        state.frame = stack.last().map(|(at, _)| *at);
        state.artifact = stack.iter().any(|(_, artifact)| *artifact);
    }
    out.push(state);
    out
}

/// What a `BDC`'s property-list name resolves to through the page's `/Properties` (scope §3.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PropertyList {
    /// The page's `/Properties`, inherited through `/Parent`, does not hold the name, or the
    /// entry names an object the document does not hold.
    Unresolved,
    /// The entry resolves to something other than a dictionary, named by its variant.
    NotADictionary(String),
    /// A dictionary carrying `/MCID`: an id the reader declared by name and did not read.
    WithId,
    /// A dictionary without `/MCID` — an optional-content layer — which is an ordinary frame.
    WithoutId,
}

/// A page's property lists, resolved as the interpreter does not: the page's own `/Resources`,
/// or the nearest ancestor's (§7.7.3.4), then `/Properties`, then the name.
///
/// Bounded at [`crate::extract::INHERITANCE_MAX_DEPTH`] hops for the reason that bound exists.
pub(crate) fn page_property_lists<'a>(
    doc: &'a lopdf::Document,
    page_dict: &'a lopdf::Dictionary,
) -> impl Fn(&[u8]) -> PropertyList + 'a {
    let resources = {
        let mut found = crate::fonts::resolve_dict(doc, page_dict.get(b"Resources").ok());
        if found.is_none() {
            let mut node = crate::fonts::resolve_dict(doc, page_dict.get(b"Parent").ok());
            for _ in 0..crate::extract::INHERITANCE_MAX_DEPTH {
                let Some(dict) = node else {
                    break;
                };
                found = crate::fonts::resolve_dict(doc, dict.get(b"Resources").ok());
                if found.is_some() {
                    break;
                }
                node = crate::fonts::resolve_dict(doc, dict.get(b"Parent").ok());
            }
        }
        found
    };
    let properties =
        resources.and_then(|r| crate::fonts::resolve_dict(doc, r.get(b"Properties").ok()));
    move |name: &[u8]| {
        let Some(properties) = &properties else {
            return PropertyList::Unresolved;
        };
        let Ok(entry) = properties.get(name) else {
            return PropertyList::Unresolved;
        };
        match doc.dereference(entry) {
            Ok((_, Object::Dictionary(d))) => {
                if d.get(b"MCID").is_ok() {
                    PropertyList::WithId
                } else {
                    PropertyList::WithoutId
                }
            }
            Ok((_, other)) => PropertyList::NotADictionary(other.enum_variant().to_string()),
            Err(_) => PropertyList::Unresolved,
        }
    }
}

/// Refuse the marked-content ids scope §3.6 names, before any byte is written.
///
/// An inline property list carrying `/MCID`, and a named one that resolves to a dictionary
/// carrying it or to nothing, each refuse the document by name. A named list without an id is
/// an optional-content layer and an ordinary frame; a `BMC` has no list at all.
///
/// # Errors
///
/// [`EngineError::Unsupported`] with `what` = `tagging`, naming the page, the operation and the
/// tag.
pub(crate) fn refuse_ids(
    ops: &[lopdf::content::Operation],
    page: u32,
    resolve: &dyn Fn(&[u8]) -> PropertyList,
) -> Result<(), EngineError> {
    for (i, op) in ops.iter().enumerate() {
        let OpKind::BeginMarked { tag, props } = OpKind::of(op) else {
            continue;
        };
        let tag = String::from_utf8_lossy(&tag).into_owned();
        let refuse = |detail: String| EngineError::Unsupported {
            what: "tagging".into(),
            detail: format!("page {page}, operation {i}: {detail}"),
        };
        match props {
            MarkedProps::None => {}
            MarkedProps::Inline(dict) => {
                if let Ok(mcid) = dict.get(b"MCID") {
                    return Err(refuse(format!(
                        "`/{tag} <</MCID {mcid:?}>> BDC` carries a marked-content id in the \
                         content stream, and the document has no structure tree. The ids \
                         already have a meaning the document lost; writing another id inside \
                         the same sequence, or reusing one, would either shadow it or claim the \
                         parent tree already indexes it (docs/23-AUTO-TAGGING-SCOPE.md §3.6)"
                    )));
                }
            }
            MarkedProps::Named(name) => {
                let name = String::from_utf8_lossy(&name).into_owned();
                match resolve(name.as_bytes()) {
                    PropertyList::WithoutId => {}
                    PropertyList::WithId => {
                        return Err(refuse(format!(
                            "`/{tag} /{name} BDC` names a property list that carries `/MCID` \
                             through the page's `/Properties`, and the document has no \
                             structure tree: an id the reader declares as \
                             `mcid-property-list-by-name` and does not read, refused as an \
                             inline id is (docs/23-AUTO-TAGGING-SCOPE.md §3.6)"
                        )))
                    }
                    PropertyList::Unresolved => {
                        return Err(refuse(format!(
                            "`/{tag} /{name} BDC` names a property list the page's \
                             `/Properties` (inherited through `/Parent`) does not hold, so \
                             whether it carries an id cannot be said: an id unknown rather than \
                             absent (docs/23-AUTO-TAGGING-SCOPE.md §3.6)"
                        )))
                    }
                    PropertyList::NotADictionary(variant) => {
                        return Err(refuse(format!(
                            "`/{tag} /{name} BDC` names a property list that resolves to \
                             {variant} rather than a dictionary, so whether it carries an id \
                             cannot be said: an id unknown rather than absent \
                             (docs/23-AUTO-TAGGING-SCOPE.md §3.6)"
                        )))
                    }
                }
            }
        }
    }
    Ok(())
}

/// Whose text each operation showed, from the page's runs and the side table that names each
/// run's operator (`extract::RunPositions`).
///
/// `None` for an operation that is not text-showing. A text-showing operation none of the page's
/// runs name showed nothing the reader kept. An operation whose runs the cut placed in two blocks
/// is refused: a sequence holds an operator for one element, and a `TJ` whose strings straddle a
/// gutter is an operator for two.
///
/// # Errors
///
/// [`EngineError::Unsupported`] naming the operation and both blocks; [`EngineError::Malformed`]
/// if the side table names an operation the page does not have or one that shows no text, which
/// no document can cause.
pub(crate) fn shows_from_runs(
    ops: &[lopdf::content::Operation],
    page: u32,
    runs: &[crate::nodes::TextRun],
    positions: &[usize],
) -> Result<Vec<Option<Shows>>, EngineError> {
    let mut shows: Vec<Option<Shows>> = ops
        .iter()
        .map(|op| matches!(OpKind::of(op), OpKind::TextShow).then_some(Shows::Nothing))
        .collect();
    let mut first_run_of: std::collections::BTreeMap<usize, usize> =
        std::collections::BTreeMap::new();
    for (i, (run, &at)) in runs.iter().zip(positions).enumerate() {
        let slot =
            shows
                .get_mut(at)
                .and_then(Option::as_mut)
                .ok_or_else(|| EngineError::Malformed {
                    what: "tagging".into(),
                    detail: format!(
                    "page {page}: run {i} `{}` names operation {at}, which is not a text-showing \
                     operation of the page's {} — the side table does not describe this page",
                    run.text,
                    ops.len()
                ),
                })?;
        let this = if matches!(
            run.structural,
            Some(ethos_parser_core::StructuralLocator::PdfArtifact(_))
        ) {
            Shows::Artifact
        } else {
            Shows::Block((run.region, run.block))
        };
        match *slot {
            Shows::Nothing => {
                *slot = this;
                first_run_of.insert(at, i);
            }
            already if already == this => {}
            already => {
                let earlier = first_run_of.get(&at).copied().unwrap_or(i);
                let describe = |shows: Shows| match shows {
                    Shows::Block(key) => format!("block {key:?}"),
                    Shows::Artifact => "an /Artifact run".to_string(),
                    Shows::Nothing => "no run".to_string(),
                };
                return Err(EngineError::Unsupported {
                    what: "tagging".into(),
                    detail: format!(
                        "page {page}, operation {at} shows runs the cut placed in two blocks \
                         ({} for `{}` and {} for `{}`), and a marked-content sequence holds an \
                         operator for one element",
                        describe(already),
                        runs[earlier].text,
                        describe(this),
                        run.text
                    ),
                });
            }
        }
    }
    Ok(shows)
}

/// The page's blocks in reading order: first appearance in the artifact's run order, artifact
/// runs skipped.
pub(crate) fn blocks_in_reading_order(runs: &[crate::nodes::TextRun]) -> Vec<BlockKey> {
    let mut order: Vec<BlockKey> = Vec::new();
    for run in runs {
        if matches!(
            run.structural,
            Some(ethos_parser_core::StructuralLocator::PdfArtifact(_))
        ) {
            continue;
        }
        let key = (run.region, run.block);
        if !order.contains(&key) {
            order.push(key);
        }
    }
    order
}

// ---------------------------------------------------------------------------------------------
// The sequences
// ---------------------------------------------------------------------------------------------

/// Where a sequence's boundary came to rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placement {
    /// At the block's operators, inside a text object it shares.
    Operators,
    /// Around whole text objects.
    TextObject,
    /// Around `q … Q` pairs enclosing whole text objects.
    GraphicsState,
}

/// Why the previous sequence of the same block ended before this one (scope §7.1's buckets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SplitCause {
    /// The previous sequence could not leave its text object, because another block shares it.
    SharedTextObject,
    /// A `q`/`Q` the two could not be balanced across.
    GraphicsState,
    /// An existing marked-content frame opened or closed between them.
    ExistingFrame,
    /// A painting operator, an inline image included.
    PaintingOperator,
    /// A text-showing operator of another block, or of no run.
    ForeignRun,
    /// An `/Artifact` frame, or a run inside one.
    Artifact,
}

/// One marked-content sequence the writer will insert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sequence {
    /// The block whose text it holds.
    pub(crate) block: BlockKey,
    /// The first enclosed operation.
    pub(crate) first: usize,
    /// The last enclosed operation.
    pub(crate) last: usize,
    /// Its id: the sequence's rank by `first` on its page, dense from 0.
    pub(crate) mcid: i64,
    /// Where its boundary rests.
    pub(crate) placement: Placement,
    /// Why the block's previous sequence ended, or `None` for the block's first.
    pub(crate) split: Option<SplitCause>,
}

/// One page's sequences and the blocks that own them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PagePlan {
    /// In stream order; a sequence's index is its id.
    pub(crate) sequences: Vec<Sequence>,
    /// Blocks in reading order, each with its sequence ids ascending — the `/K` of its `/Div`.
    pub(crate) blocks: Vec<(BlockKey, Vec<i64>)>,
}

/// Draws nothing, and changes no frame: the operators a sequence may enclose beside its own
/// block's text, and the only ones a widening may cross.
fn is_inert(kind: &OpKind) -> bool {
    matches!(
        kind,
        OpKind::Other | OpKind::BeginText | OpKind::EndText | OpKind::Save | OpKind::Restore
    )
}

/// Table 108's text-positioning operators: the ones that say where the next text-showing
/// operator draws, and nothing else.
fn is_text_positioning(op: &lopdf::content::Operation) -> bool {
    matches!(op.operator.as_str(), "Td" | "TD" | "Tm" | "T*")
}

/// Whether the operations strictly between `a` and `b` draw nothing and keep both depths at or
/// above `s`'s, returning to `s` — with the same frame — by `b`.
fn gap_is_clear(kinds: &[OpKind], nesting: &[Nesting], a: usize, b: usize, s: Nesting) -> bool {
    for x in a + 1..b {
        if !is_inert(&kinds[x]) {
            return false;
        }
        let n = nesting[x];
        if n.q < s.q || n.bt < s.bt {
            return false;
        }
    }
    let end = nesting[b];
    end.q == s.q && end.bt == s.bt && end.frame == s.frame
}

/// The `BT … ET` enclosing `first..=last` when everything newly enclosed draws nothing, stays
/// at or above the sequence's depths and in its frame.
fn enclosing_text_object(
    kinds: &[OpKind],
    nesting: &[Nesting],
    first: usize,
    last: usize,
) -> Option<(usize, usize)> {
    let s = nesting[first];
    if s.bt < 1 {
        return None;
    }
    let mut i = first;
    let open = loop {
        if i == 0 {
            return None;
        }
        i -= 1;
        let n = nesting[i];
        if kinds[i] == OpKind::BeginText && n.bt == s.bt - 1 {
            if n.q != s.q || n.frame != s.frame {
                return None;
            }
            break i;
        }
        if !is_inert(&kinds[i]) || n.q < s.q || n.bt < s.bt {
            return None;
        }
    };
    let mut j = last + 1;
    let close = loop {
        if j >= kinds.len() {
            return None;
        }
        let n = nesting[j];
        if kinds[j] == OpKind::EndText && n.bt == s.bt {
            if n.q != s.q || n.frame != s.frame {
                return None;
            }
            break j;
        }
        if !is_inert(&kinds[j]) || n.q < s.q || n.bt < s.bt {
            return None;
        }
        j += 1;
    };
    Some((open, close))
}

/// The `q … Q` enclosing `first..=last` under the same conditions.
fn enclosing_graphics_state(
    kinds: &[OpKind],
    nesting: &[Nesting],
    first: usize,
    last: usize,
) -> Option<(usize, usize)> {
    let s = nesting[first];
    if s.q < 1 {
        return None;
    }
    let mut i = first;
    let open = loop {
        if i == 0 {
            return None;
        }
        i -= 1;
        let n = nesting[i];
        if kinds[i] == OpKind::Save && n.q == s.q - 1 {
            if n.bt != s.bt || n.frame != s.frame {
                return None;
            }
            break i;
        }
        if !is_inert(&kinds[i]) || n.q < s.q || n.bt < s.bt {
            return None;
        }
    };
    let mut j = last + 1;
    let close = loop {
        if j >= kinds.len() {
            return None;
        }
        let n = nesting[j];
        if kinds[j] == OpKind::Restore && n.q == s.q {
            if n.bt != s.bt || n.frame != s.frame {
                return None;
            }
            break j;
        }
        if !is_inert(&kinds[j]) || n.q < s.q || n.bt < s.bt {
            return None;
        }
        j += 1;
    };
    Some((open, close))
}

/// Why two consecutive sequences of one block stayed apart: the first barrier between them by
/// kind, or, when only inert operators lie between, the depth that kept them apart.
fn split_cause(
    kinds: &[OpKind],
    nesting: &[Nesting],
    shows: &[Option<Shows>],
    prev_last: usize,
    next_first: usize,
) -> SplitCause {
    for x in prev_last + 1..next_first {
        match &kinds[x] {
            OpKind::Painting | OpKind::InlineImage => return SplitCause::PaintingOperator,
            OpKind::BeginMarked { tag, .. } => {
                return if tag == b"Artifact" {
                    SplitCause::Artifact
                } else {
                    SplitCause::ExistingFrame
                }
            }
            OpKind::EndMarked => {
                return if nesting[x].artifact {
                    SplitCause::Artifact
                } else {
                    SplitCause::ExistingFrame
                }
            }
            OpKind::TextShow => {
                return match shows[x] {
                    Some(Shows::Artifact) => SplitCause::Artifact,
                    _ => SplitCause::ForeignRun,
                }
            }
            _ => {}
        }
    }
    let after = nesting[prev_last + 1];
    let text_object_kept_them_apart = (prev_last + 1..=next_first)
        .any(|x| nesting[x].bt < after.bt)
        || nesting[next_first].bt != after.bt;
    if text_object_kept_them_apart {
        SplitCause::SharedTextObject
    } else {
        SplitCause::GraphicsState
    }
}

/// The placement rule of scope §3.4 over one page.
///
/// Each block's text-showing operations are grouped greedily in stream order — the next joins the
/// open sequence when everything between draws nothing, opens or closes no frame, and keeps both
/// depths at or above the sequence's before returning to them — and each sequence then opens
/// before the text-positioning operators (`Td`, `TD`, `Tm`, `T*`) immediately preceding its
/// first text-showing operator, so its first line's position rides inside it. Every sequence is
/// widened outward to the enclosing `BT … ET`, then to a `q … Q` enclosing exactly that, while
/// everything newly enclosed draws nothing, belongs to no other block and sits inside the same
/// frame at no lower depth; adjacent sequences of one block separated only by such operators at
/// the same frame and depth merge; the two steps repeat until nothing moves. Ids are the rank by
/// first operation, dense from 0, and each block's `/K` is its ids ascending.
///
/// `order` is the page's blocks in reading order and decides the order of the `/Div` elements;
/// a block in it with no text-showing operation of its own gets no sequence and no element.
pub(crate) fn plan_page(
    ops: &[lopdf::content::Operation],
    nesting: &[Nesting],
    shows: &[Option<Shows>],
    order: &[BlockKey],
) -> PagePlan {
    let kinds: Vec<OpKind> = ops.iter().map(OpKind::of).collect();

    // The greedy grouping, per block, in stream order.
    let mut sequences: Vec<Sequence> = Vec::new();
    for &block in order {
        let mut open: Option<(usize, usize)> = None;
        for (i, show) in shows.iter().enumerate() {
            if *show != Some(Shows::Block(block)) {
                continue;
            }
            match open {
                Some((first, last)) if gap_is_clear(&kinds, nesting, last, i, nesting[last]) => {
                    open = Some((first, i));
                }
                Some((first, last)) => {
                    sequences.push(placed(block, first, last));
                    open = Some((i, i));
                }
                None => open = Some((i, i)),
            }
        }
        if let Some((first, last)) = open {
            sequences.push(placed(block, first, last));
        }
    }

    // The positioning that belongs to the first line rides inside the sequence.
    for seq in &mut sequences {
        while seq.first > 0 && is_text_positioning(&ops[seq.first - 1]) {
            seq.first -= 1;
        }
    }

    // Widen and merge until nothing moves. Each step only grows a sequence, so this ends.
    loop {
        let mut moved = false;
        for seq in &mut sequences {
            loop {
                let mut widened = false;
                if let Some((open, close)) =
                    enclosing_text_object(&kinds, nesting, seq.first, seq.last)
                {
                    seq.first = open;
                    seq.last = close;
                    widened = true;
                }
                while let Some((open, close)) =
                    enclosing_graphics_state(&kinds, nesting, seq.first, seq.last)
                {
                    seq.first = open;
                    seq.last = close;
                    widened = true;
                }
                if !widened {
                    break;
                }
                moved = true;
            }
        }
        sequences.sort_by_key(|s| s.first);
        let mut merged: Vec<Sequence> = Vec::with_capacity(sequences.len());
        for seq in sequences.drain(..) {
            let joins = merged.last().is_some_and(|prev: &Sequence| {
                prev.block == seq.block
                    && gap_is_clear(&kinds, nesting, prev.last, seq.first, nesting[prev.first])
            });
            if joins {
                merged.last_mut().expect("checked above").last = seq.last;
                moved = true;
            } else {
                merged.push(seq);
            }
        }
        sequences = merged;
        if !moved {
            break;
        }
    }

    // Ids by stream order, placements, and the causes per block.
    sequences.sort_by_key(|s| s.first);
    let mut last_of_block: std::collections::BTreeMap<BlockKey, usize> =
        std::collections::BTreeMap::new();
    for (rank, seq) in sequences.iter_mut().enumerate() {
        seq.mcid = i64::try_from(rank).unwrap_or(i64::MAX);
        seq.placement = match kinds[seq.first] {
            OpKind::Save => Placement::GraphicsState,
            OpKind::BeginText => Placement::TextObject,
            _ => Placement::Operators,
        };
        seq.split = last_of_block
            .get(&seq.block)
            .map(|&prev_last| split_cause(&kinds, nesting, shows, prev_last, seq.first));
        last_of_block.insert(seq.block, seq.last);
    }
    let blocks: Vec<(BlockKey, Vec<i64>)> = order
        .iter()
        .map(|&block| {
            let ids: Vec<i64> = sequences
                .iter()
                .filter(|s| s.block == block)
                .map(|s| s.mcid)
                .collect();
            (block, ids)
        })
        .filter(|(_, ids)| !ids.is_empty())
        .collect();
    PagePlan { sequences, blocks }
}

fn placed(block: BlockKey, first: usize, last: usize) -> Sequence {
    Sequence {
        block,
        first,
        last,
        mcid: 0,
        placement: Placement::Operators,
        split: None,
    }
}

/// Scope §3.4 on a page's sequences, as a check: each holds only its block's text-showing
/// operators and operators that draw nothing, encloses no painting operator, no inline image, no
/// foreign text and no frame boundary, opens and closes at one frame and depth without dipping
/// below it, and the ids are dense in stream order.
///
/// Shared by the planner's tests and by the self-check on the written page (scope §3.7), where
/// the sequences are the ones read back out of the output and `shows` comes from the output's own
/// runs — so the rule is proved on every document rather than on the fixtures alone.
///
/// # Errors
///
/// The first failing condition, naming the sequence's id and the operation.
pub(crate) fn sequences_are_well_formed(
    ops: &[lopdf::content::Operation],
    nesting: &[Nesting],
    shows: &[Option<Shows>],
    sequences: &[Sequence],
) -> Result<(), String> {
    let kinds: Vec<OpKind> = ops.iter().map(OpKind::of).collect();
    let mut previous_last: Option<usize> = None;
    for (rank, seq) in sequences.iter().enumerate() {
        let id = seq.mcid;
        if usize::try_from(id) != Ok(rank) {
            return Err(format!(
                "sequence {rank} in stream order carries id {id}: ids must be dense from 0 in \
                 stream order"
            ));
        }
        if seq.first > seq.last || seq.last >= ops.len() {
            return Err(format!(
                "sequence {id} spans operations {}..={} of {}",
                seq.first,
                seq.last,
                ops.len()
            ));
        }
        if previous_last.is_some_and(|prev| prev >= seq.first) {
            return Err(format!(
                "sequence {id} starts at operation {} inside the previous sequence",
                seq.first
            ));
        }
        previous_last = Some(seq.last);

        let s = nesting[seq.first];
        if s.artifact {
            return Err(format!(
                "sequence {id} sits inside an /Artifact frame (operation {})",
                seq.first
            ));
        }
        let mut own_text = 0usize;
        for x in seq.first..=seq.last {
            match &kinds[x] {
                OpKind::TextShow => match shows.get(x).copied().flatten() {
                    Some(Shows::Block(block)) if block == seq.block => own_text += 1,
                    Some(Shows::Block(other)) => {
                        return Err(format!(
                            "sequence {id} for block {:?} encloses operation {x}, a \
                             text-showing operator of block {other:?}",
                            seq.block
                        ))
                    }
                    Some(Shows::Artifact) => {
                        return Err(format!(
                            "sequence {id} encloses operation {x}, a text-showing operator \
                             inside an /Artifact frame"
                        ))
                    }
                    Some(Shows::Nothing) | None => {
                        return Err(format!(
                            "sequence {id} encloses operation {x}, a text-showing operator that \
                             showed no run this engine kept"
                        ))
                    }
                },
                OpKind::Painting => {
                    return Err(format!(
                        "sequence {id} encloses operation {x}, the painting operator `{}`",
                        ops[x].operator
                    ))
                }
                OpKind::InlineImage => {
                    return Err(format!(
                        "sequence {id} encloses operation {x}, an inline image"
                    ))
                }
                OpKind::BeginMarked { tag, .. } => {
                    return Err(format!(
                        "sequence {id} encloses operation {x}, the opening of a{} frame `/{}`",
                        if tag == b"Artifact" {
                            "n /Artifact"
                        } else {
                            " marked-content"
                        },
                        String::from_utf8_lossy(tag)
                    ))
                }
                OpKind::EndMarked => {
                    return Err(format!(
                        "sequence {id} encloses operation {x}, the closing of a{} frame",
                        if nesting[x].artifact {
                            "n /Artifact"
                        } else {
                            " marked-content"
                        }
                    ))
                }
                _ => {}
            }
            let n = nesting[x];
            if n.q < s.q || n.bt < s.bt {
                return Err(format!(
                    "sequence {id} dips below its own depth at operation {x} (q {} bt {} against \
                     q {} bt {})",
                    n.q, n.bt, s.q, s.bt
                ));
            }
        }
        if own_text == 0 {
            return Err(format!(
                "sequence {id} encloses no text-showing operator of its block"
            ));
        }
        let e = nesting[seq.last + 1];
        if e.q != s.q || e.bt != s.bt || e.frame != s.frame {
            return Err(format!(
                "sequence {id} opens at frame {:?} q {} bt {} and closes at frame {:?} q {} bt {}",
                s.frame, s.q, s.bt, e.frame, e.q, e.bt
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The splice: inserted at token boundaries, nothing else changed
// ---------------------------------------------------------------------------------------------

/// The page's bytes with each sequence's `BDC` and `EMC` inserted at its token boundaries.
///
/// Before the first enclosed operation's first token: `/Div <</MCID n>> BDC` and a newline. After
/// the last enclosed operation's operator: a newline, `EMC`, and a newline unless the next byte
/// already is whitespace. Every byte of the input survives in order; nothing is re-encoded.
pub(crate) fn splice(bytes: &[u8], t: &Tokenised, sequences: &[Sequence]) -> Vec<u8> {
    // (offset, closes-before-opens, text): a closing `EMC` and the next sequence's opening `BDC`
    // can meet at one offset, and the close must come first.
    let mut insertions: Vec<(usize, u8, Vec<u8>)> = Vec::with_capacity(sequences.len() * 2);
    for seq in sequences {
        let open = format!("/Div <</MCID {}>> BDC\n", seq.mcid).into_bytes();
        insertions.push((t.ops[seq.first].start, 1, open));
        let at = t.ops[seq.last].end;
        let mut close = b"\nEMC".to_vec();
        if bytes
            .get(at)
            .is_some_and(|&b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        {
            close.push(b'\n');
        }
        insertions.push((at, 0, close));
    }
    insertions.sort();
    let mut out =
        Vec::with_capacity(bytes.len() + insertions.iter().map(|i| i.2.len()).sum::<usize>());
    let mut cursor = 0usize;
    for (at, _, text) in insertions {
        out.extend_from_slice(&bytes[cursor..at]);
        out.extend_from_slice(&text);
        cursor = at;
    }
    out.extend_from_slice(&bytes[cursor..]);
    out
}

// ---------------------------------------------------------------------------------------------
// The tree (scope §3.2–§3.4) and the stamp (§3.3)
// ---------------------------------------------------------------------------------------------

/// The attribute object every written element carries (scope §3.3), the rule spelled as the
/// profile spells it.
fn attribute(rule: &str) -> lopdf::Dictionary {
    let mut a = lopdf::Dictionary::new();
    a.set(
        "O",
        Object::Name(crate::structure::STRUCT_ATTRIBUTE_OWNER.as_bytes().to_vec()),
    );
    a.set(
        "Derivation",
        Object::Name(
            crate::structure::OWNER_DERIVATION_COMPUTED
                .as_bytes()
                .to_vec(),
        ),
    );
    a.set("Rule", Object::string_literal(rule));
    a
}

/// Write the structure tree for the pages that received sequences, in page order.
///
/// One `/Document` root element and one `/Div` per block in reading order, each carrying the
/// attribute; a `/StructTreeRoot` whose `/ParentTree` has one key per rewritten page (0-based in
/// page order) mapping each id to the element that cites it; `/StructParents` on each such page;
/// `/StructTreeRoot` on the catalog. No `/MarkInfo` is written and one already there is left as
/// found. Objects are added in a fixed order — the root, the document element, then the `/Div`s
/// page by page in reading order — so two writes number them alike.
///
/// Returns how many structure elements were written.
///
/// # Errors
///
/// [`EngineError::Malformed`] if the catalog or a page dictionary does not resolve.
pub(crate) fn write_tree(
    out: &mut lopdf::Document,
    pages: &[(ObjectId, &PagePlan)],
    rule: &str,
) -> Result<u32, EngineError> {
    let malformed = |detail: String| EngineError::Malformed {
        what: "tagging".into(),
        detail,
    };
    let root_id = out.new_object_id();
    let document_id = out.new_object_id();

    let mut divs: Vec<Object> = Vec::new();
    let mut nums: Vec<Object> = Vec::new();
    for (key, (page_id, plan)) in pages.iter().enumerate() {
        let mut by_mcid: Vec<Option<ObjectId>> = vec![None; plan.sequences.len()];
        for (block, ids) in &plan.blocks {
            let mut element = lopdf::Dictionary::new();
            element.set("Type", Object::Name(b"StructElem".to_vec()));
            element.set("S", Object::Name(b"Div".to_vec()));
            element.set("P", Object::Reference(document_id));
            element.set("Pg", Object::Reference(*page_id));
            element.set(
                "K",
                Object::Array(ids.iter().map(|&id| Object::Integer(id)).collect()),
            );
            element.set("A", Object::Dictionary(attribute(rule)));
            let div_id = out.add_object(Object::Dictionary(element));
            divs.push(Object::Reference(div_id));
            for &id in ids {
                let slot = usize::try_from(id)
                    .ok()
                    .and_then(|i| by_mcid.get_mut(i))
                    .ok_or_else(|| {
                        malformed(format!(
                            "block {block:?} cites id {id}, which its page's {} sequence(s) do \
                             not include",
                            plan.sequences.len()
                        ))
                    })?;
                *slot = Some(div_id);
            }
        }
        let mut refs = Vec::with_capacity(by_mcid.len());
        for (id, slot) in by_mcid.iter().enumerate() {
            let div_id = slot.ok_or_else(|| {
                malformed(format!(
                    "id {id} on page object {page_id:?} belongs to no block"
                ))
            })?;
            refs.push(Object::Reference(div_id));
        }
        nums.push(Object::Integer(i64::try_from(key).unwrap_or(i64::MAX)));
        nums.push(Object::Array(refs));
        out.get_dictionary_mut(*page_id)
            .map_err(|e| malformed(format!("page object {page_id:?}: {e}")))?
            .set(
                "StructParents",
                Object::Integer(i64::try_from(key).unwrap_or(i64::MAX)),
            );
    }

    let mut document = lopdf::Dictionary::new();
    document.set("Type", Object::Name(b"StructElem".to_vec()));
    document.set("S", Object::Name(b"Document".to_vec()));
    document.set("P", Object::Reference(root_id));
    document.set("K", Object::Array(divs.clone()));
    document.set("A", Object::Dictionary(attribute(rule)));
    out.set_object(document_id, Object::Dictionary(document));

    let mut parent_tree = lopdf::Dictionary::new();
    parent_tree.set("Nums", Object::Array(nums));
    let mut root = lopdf::Dictionary::new();
    root.set("Type", Object::Name(b"StructTreeRoot".to_vec()));
    root.set("K", Object::Array(vec![Object::Reference(document_id)]));
    root.set("ParentTree", Object::Dictionary(parent_tree));
    root.set(
        "ParentTreeNextKey",
        Object::Integer(i64::try_from(pages.len()).unwrap_or(i64::MAX)),
    );
    out.set_object(root_id, Object::Dictionary(root));

    out.catalog_mut()
        .map_err(|e| malformed(format!("the catalog does not resolve: {e}")))?
        .set("StructTreeRoot", Object::Reference(root_id));

    Ok(u32::try_from(1 + divs.len()).unwrap_or(u32::MAX))
}

/// The catalog stamp of scope §3.3: which bytes the tags were computed from and under which
/// profile. Provenance, never the derivation — the reader does not consult it.
pub(crate) fn stamp_tags(
    out: &mut lopdf::Document,
    source_sha256: &str,
    profile_sha256: &str,
    parser_version: &str,
) -> Result<(), EngineError> {
    let mut stamp = lopdf::Dictionary::new();
    stamp.set("ArtifactType", Object::string_literal(TAGS_ARTIFACT_TYPE));
    stamp.set("SourceSha256", Object::string_literal(source_sha256));
    stamp.set("ProfileSha256", Object::string_literal(profile_sha256));
    stamp.set("ParserVersion", Object::string_literal(parser_version));
    out.catalog_mut()
        .map_err(|e| EngineError::Malformed {
            what: "tagging".into(),
            detail: format!("the catalog does not resolve: {e}"),
        })?
        .set("EthosParserTags", Object::Dictionary(stamp));
    Ok(())
}

/// The writer's artifact type, stamped on the catalog (scope §8): what moves when the written
/// shape — the role, the attribute keys, the placement rule — changes.
pub const TAGS_ARTIFACT_TYPE: &str = "ethos.parser.tags.v0";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::TextRun;
    use lopdf::content::{Content, Operation};

    fn lossy(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    fn operators(t: &Tokenised) -> Vec<&str> {
        t.ops.iter().map(|o| o.operator.as_str()).collect()
    }

    /// Between two operations, and before the first and after the last, only whitespace and
    /// comments — a comment running to its line end, which is whitespace.
    fn assert_gap_is_blank(bytes: &[u8], from: usize, to: usize) {
        let mut p = from;
        while p < to {
            match bytes[p] {
                b' ' | b'\t' | b'\r' | b'\n' => p += 1,
                b'%' => {
                    while p < to && bytes[p] != b'\r' && bytes[p] != b'\n' {
                        p += 1;
                    }
                }
                other => panic!(
                    "byte {p} ({other:#04x}) lies between operations and is neither whitespace \
                     nor a comment"
                ),
            }
        }
    }

    /// The coverage condition of scope §3.5: the spans are in order, each slice ends with its
    /// operator (`EI` for an inline image) and starts on its own first token, and the gaps hold
    /// nothing but whitespace and comments. Shared with the corpus tests.
    fn assert_every_byte_placed(label: &str, bytes: &[u8], t: &Tokenised) {
        let mut cursor = 0usize;
        for (i, op) in t.ops.iter().enumerate() {
            assert!(
                cursor <= op.start && op.start < op.end && op.end <= bytes.len(),
                "{label}: op {i} {op:?} is out of order or out of range (cursor {cursor}, len {})",
                bytes.len()
            );
            assert_gap_is_blank(bytes, cursor, op.start);
            let slice = &bytes[op.start..op.end];
            if op.operator == "BI" {
                assert!(
                    slice.starts_with(b"BI"),
                    "{label}: op {i}: {:?}",
                    lossy(slice)
                );
                assert!(
                    slice.ends_with(b"EI"),
                    "{label}: op {i}: {:?}",
                    lossy(slice)
                );
            } else {
                assert!(
                    slice.ends_with(op.operator.as_bytes()),
                    "{label}: op {i}: {:?} does not end with `{}`",
                    lossy(slice),
                    op.operator
                );
                assert!(
                    !b" \t\r\n%".contains(&slice[0]),
                    "{label}: op {i}: {:?} starts on whitespace or a comment",
                    lossy(slice)
                );
            }
            cursor = op.end;
        }
        assert_gap_is_blank(bytes, cursor, bytes.len());
    }

    /// The whole contract on one buffer: the tokeniser's verdict is `lopdf`'s strict parse's,
    /// and on success its operators are `lopdf`'s lenient decode's and every byte is placed.
    /// Returns the tokeniser's own answer for the assertions particular to the case.
    fn check(bytes: &[u8]) -> Result<Tokenised, EngineError> {
        let strict = Content::decode_strict(bytes).is_ok();
        let result = tokenise(bytes);
        assert_eq!(
            result.is_ok(),
            strict,
            "the tokeniser and lopdf's strict parse disagree on {:?}: {result:?}",
            lossy(bytes)
        );
        if let Ok(t) = &result {
            let lenient = Content::decode(bytes).expect("strict succeeded, so lenient does");
            agrees_with_lopdf(t, &lenient.operations).expect("operators agree");
            assert_every_byte_placed(&lossy(bytes), bytes, t);
        }
        result
    }

    fn ok(bytes: &[u8]) -> Tokenised {
        check(bytes).unwrap_or_else(|e| panic!("{:?} must tokenise: {e}", lossy(bytes)))
    }

    fn refused(bytes: &[u8]) -> EngineError {
        let e = check(bytes).expect_err("must be refused");
        assert_eq!(e.code(), "unsupported");
        assert!(
            e.to_string()
                .starts_with("unsupported content stream tokeniser:"),
            "{e}"
        );
        e
    }

    // ---------------------------------------------------------------------------------------
    // The tokeniser, operand by operand
    // ---------------------------------------------------------------------------------------

    #[test]
    fn nested_literal_strings_with_escapes_and_a_line_continuation() {
        let src = b"BT (a(b(c))\\)\\\\ \\(x\\\ny\\101\r\nz) Tj ET";
        let t = ok(src);
        assert_eq!(operators(&t), ["BT", "Tj", "ET"]);
        assert_eq!(
            t.ops[1].start, 3,
            "the string is the operation's first token"
        );
        assert!(lossy(&src[t.ops[1].start..t.ops[1].end]).ends_with(") Tj"));
    }

    #[test]
    fn a_hex_string_with_whitespace_inside() {
        let t = ok(b"<48 65\n6C\t6C\x0C6F\0 > Tj");
        assert_eq!(operators(&t), ["Tj"]);
        assert_eq!((t.ops[0].start, t.ops[0].end), (0, 21));
    }

    #[test]
    fn a_name_with_a_hash_escape() {
        let t = ok(b"/F#201 12 Tf");
        assert_eq!(operators(&t), ["Tf"]);
        assert_eq!((t.ops[0].start, t.ops[0].end), (0, 12));
        // A `#` not followed by two hex digits ends the name, and what follows is unparseable —
        // exactly where lopdf stops.
        refused(b"/F#2 12 Tf");
    }

    #[test]
    fn a_comment_between_operations_is_a_gap() {
        let src = b"1 0 0 RG\n% a note, with (parens) and [brackets]\n0 g\n";
        let t = ok(src);
        assert_eq!(operators(&t), ["RG", "g"]);
        assert_eq!(
            t.ops[1].start, 48,
            "the second operation starts after the comment"
        );
        // A comment where the grammar does not allow one — between two operands — is where
        // lopdf's decoder stops, and the tokeniser says so.
        refused(b"1 % no\n0 0 RG");
        // And one at the very end without a line end cannot be placed.
        refused(b"q Q %tail");
        ok(b"q Q %tail\n");
    }

    #[test]
    fn a_dictionary_operand_with_a_nested_array() {
        let src = b"/Span << /MCID 3 /K [1 [2 3] << /A /B >> 4 0 R] % inside\n /T (x) >> BDC EMC";
        let t = ok(src);
        assert_eq!(operators(&t), ["BDC", "EMC"]);
        assert_eq!(t.ops[0].start, 0);
        assert!(lossy(&src[..t.ops[0].end]).ends_with(">> BDC"));
    }

    #[test]
    fn references_are_read_inside_arrays_and_not_between_operands() {
        // Inside an array `1 0 R` is one object; at the top level `R` is an operator with two
        // integer operands, which is what lopdf reads and the interpreter later refuses.
        let t = ok(b"[1 0 R] TJ 1 0 R");
        assert_eq!(operators(&t), ["TJ", "R"]);
    }

    #[test]
    fn true_false_and_null_match_as_prefixes() {
        // `tag` matches a prefix, so `nullify` is the operand `null` and the operator `ify`.
        let t = ok(b"nullify trueTj false Tj");
        assert_eq!(operators(&t), ["ify", "Tj", "Tj"]);
    }

    #[test]
    fn numbers_take_the_forms_lopdf_takes() {
        let t = ok(b"5. .5 -.5 +5 -5.25 3.14.15 10Tj");
        assert_eq!(operators(&t), ["Tj"]);
        assert_eq!(t.ops[0].start, 0);
        // A lone sign, a lone dot, and an integer past `i64` are not numbers and nothing else.
        refused(b"- 5 Td");
        refused(b". Td");
        refused(b"99999999999999999999 Td");
    }

    #[test]
    fn an_inline_image_with_a_known_colour_space_ends_by_its_length() {
        // 4 × 1 pixels, RGB, 8 bits: 12 data bytes, and they contain a false ` EI ` window at
        // their second byte. The length rule must win, so the image ends at the real `EI`.
        let mut src = b"q BI /W 4 /H 1 /CS /RGB /BPC 8 ID ".to_vec();
        let data_start = src.len();
        src.extend_from_slice(b"\x01 EI \x02\x03\x04\x05\x06\x07\x08");
        assert_eq!(src.len() - data_start, 12);
        src.extend_from_slice(b" EI Q");
        let t = ok(&src);
        assert_eq!(operators(&t), ["q", "BI", "Q"]);
        assert_eq!(t.ops[1].start, 2);
        assert_eq!(
            t.ops[1].end,
            src.len() - 2,
            "the image ends after the real EI"
        );
        // The same image with the false window and the real `EI` swapped out — the data of the
        // computed length not followed by `EI` — fails the whole stream, as lopdf's `cut` does.
        let mut cut = src.clone();
        cut.truncate(data_start + 12);
        cut.extend_from_slice(b" Q");
        let e = refused(&cut);
        assert!(e.to_string().contains("not followed by EI"), "{e}");
    }

    #[test]
    fn an_inline_image_with_a_filter_ends_at_the_first_ei_window() {
        // With `/F` present lopdf sizes nothing and scans for the first whitespace-EI-whitespace
        // window; the twelve bytes the length rule would take run past that window.
        let src = b"q BI /W 4 /H 1 /CS /RGB /BPC 8 /F /AHx ID 0102> EI Q 1 0 0 RG";
        let t = ok(src);
        assert_eq!(operators(&t), ["q", "BI", "Q", "RG"]);
        let ei = lossy(src).find(" EI ").expect("the window") + 3;
        assert_eq!(t.ops[1].end, ei);
        // An abbreviation lopdf does not know is scanned too: `/G` is not in its table.
        let t = ok(b"BI /W 1 /H 1 /CS /G /BPC 8 ID \xff EI Q");
        assert_eq!(operators(&t), ["BI", "Q"]);
        // A mask needs no colour space; without `/IM true` and without a colour space lopdf's
        // parser panics, and the tokeniser refuses rather than mirroring that. This case cannot
        // go through `check`, because `check` hands the bytes to lopdf: the panic is caught here
        // instead, as the evidence for the refusal's wording.
        let t = ok(b"BI /W 8 /H 1 /IM true /BPC 1 ID \xaa EI Q");
        assert_eq!(operators(&t), ["BI", "Q"]);
        let no_colour_space: &[u8] = b"BI /W 8 /H 1 /BPC 1 ID \xaa EI Q";
        let e = tokenise(no_colour_space).expect_err("refused, not asked of lopdf");
        assert!(e.to_string().contains("panics"), "{e}");
        let quiet = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let lopdf = std::panic::catch_unwind(|| Content::decode(no_colour_space).is_ok());
        std::panic::set_hook(quiet);
        assert!(
            lopdf.is_err(),
            "lopdf 0.44.0 panics on this shape; if it no longer does, the refusal above should \
             become a comparison"
        );
        // No window at all fails the stream.
        let e = refused(b"BI /W 1 /H 1 /CS /G /BPC 8 ID \xff EI");
        assert!(e.to_string().contains("window"), "{e}");
    }

    #[test]
    fn the_line_and_quote_operators_are_single_tokens() {
        let t = ok(b"BT 12 TL T* (a) ' 1 2 (b) \" ET");
        assert_eq!(operators(&t), ["BT", "TL", "T*", "'", "\"", "ET"]);
        let t = ok(b"0 0 10 10 re f* W* n B* b*");
        assert_eq!(operators(&t), ["re", "f*", "W*", "n", "B*", "b*"]);
    }

    #[test]
    fn a_tail_lopdf_drops_is_refused_rather_than_agreed_with() {
        let src = b"BT (a) Tj ) ET";
        let lenient = Content::decode(src).expect("lopdf decodes leniently");
        assert_eq!(
            lenient.operations.len(),
            2,
            "lopdf reads two operations and drops `) ET` without a word"
        );
        let e = refused(src);
        assert!(
            e.to_string().contains("byte 10 of 14"),
            "the refusal names where the dropped tail starts: {e}"
        );
        assert!(e.to_string().contains("remaining 4 byte(s)"), "{e}");
    }

    #[test]
    fn nesting_past_lopdfs_limit_fails_the_page() {
        let nested = |depth: usize| {
            let mut s = "[".repeat(depth);
            s.push_str(&"]".repeat(depth));
            s.push_str(" TJ");
            s.into_bytes()
        };
        ok(&nested(MAX_NESTING_DEPTH));
        let e = refused(&nested(MAX_NESTING_DEPTH + 1));
        assert!(e.to_string().contains("nested more than 100 deep"), "{e}");
        // Parentheses have their own limit, and past it the string simply stops parsing — an
        // error lopdf backtracks over, so the page is refused as a dropped tail.
        let parens = |depth: usize| {
            let mut s = "(".repeat(depth);
            s.push_str(&")".repeat(depth));
            s.push_str(" Tj");
            s.into_bytes()
        };
        ok(&parens(MAX_BRACKET + 1));
        refused(&parens(MAX_BRACKET + 2));
    }

    #[test]
    fn whitespace_classes_differ_inside_and_between() {
        // `\0` and form feed are whitespace inside an array and not between operations.
        ok(b"[1\0 2\x0C3] TJ");
        refused(b"1\0 2 Td");
        refused(b"q\x0CQ");
    }

    #[test]
    fn an_empty_buffer_and_a_blank_one_tokenise_to_nothing() {
        assert_eq!(ok(b"").ops, []);
        assert_eq!(ok(b" \n\t\r%c\n").ops, []);
    }

    #[test]
    fn agreement_is_checked_by_count_and_by_name() {
        let t = ok(b"q Q");
        let same = Content::decode(b"q Q").unwrap().operations;
        agrees_with_lopdf(&t, &same).expect("identical");

        let renamed = Content::decode(b"q q").unwrap().operations;
        let e = agrees_with_lopdf(&t, &renamed).unwrap_err();
        assert!(
            e.to_string()
                .contains("operation 1: the tokeniser reads `Q` where lopdf reads `q`"),
            "{e}"
        );

        let shorter = Content::decode(b"q").unwrap().operations;
        let e = agrees_with_lopdf(&t, &shorter).unwrap_err();
        assert!(
            e.to_string()
                .contains("operation 1: the tokeniser reads 2 operation(s) and lopdf 1"),
            "{e}"
        );
        let longer = Content::decode(b"q Q Q").unwrap().operations;
        let e = agrees_with_lopdf(&t, &longer).unwrap_err();
        assert!(e.to_string().contains("operation 2:"), "{e}");
    }

    // ---------------------------------------------------------------------------------------
    // The classification
    // ---------------------------------------------------------------------------------------

    /// Every operator in Table A.1 maps to the kind the placement rule needs, decided by an
    /// exhaustive match over `Operator` so that a variant added to `ops.rs` fails here rather
    /// than falling into `Other` unnoticed.
    #[test]
    fn every_operator_in_the_table_has_a_kind() {
        use crate::ops::Operator::{self, *};

        fn expected(op: Operator) -> OpKind {
            match op {
                ShowText | ShowTextAdjusted | NextLineShowText | NextLineShowTextSpacing => {
                    OpKind::TextShow
                }
                Stroke
                | CloseStroke
                | Fill
                | FillObsolete
                | FillEvenOdd
                | FillStroke
                | FillStrokeEvenOdd
                | CloseFillStroke
                | CloseFillStrokeEvenOdd
                | Shading
                | XObject => OpKind::Painting,
                BeginText => OpKind::BeginText,
                EndText => OpKind::EndText,
                SaveState => OpKind::Save,
                RestoreState => OpKind::Restore,
                BeginMarkedContent | BeginMarkedContentProps => OpKind::BeginMarked {
                    tag: Vec::new(),
                    props: MarkedProps::None,
                },
                EndMarkedContent => OpKind::EndMarked,
                BeginInlineImage | InlineImageData | EndInlineImage => OpKind::InlineImage,
                ConcatMatrix | LineWidth | LineCap | LineJoin | MiterLimit | DashPattern
                | RenderingIntent | Flatness | ExtGState | MoveTo | LineTo | CurveTo | CurveToV
                | CurveToY | ClosePath | Rectangle | EndPath | Clip | ClipEvenOdd | CharSpacing
                | WordSpacing | HorizontalScale | Leading | SelectFont | RenderMode | Rise
                | NextLine | NextLineSetLeading | SetTextMatrix | NextLineByLeading
                | Type3Width | Type3WidthBBox | StrokeColorSpace | FillColorSpace | StrokeColor
                | StrokeColorN | FillColor | FillColorN | StrokeGray | FillGray | StrokeRgb
                | FillRgb | StrokeCmyk | FillCmyk | MarkedPoint | MarkedPointProps
                | BeginCompat | EndCompat => OpKind::Other,
            }
        }

        for op in Operator::ALL {
            let got = OpKind::of(&Operation::new(op.token(), vec![]));
            assert_eq!(got, expected(op), "`{}` ({op:?})", op.token());
        }
        // Nothing outside the table is anything but `Other`; the interpreter refuses it first.
        assert_eq!(OpKind::of(&Operation::new("Xy", vec![])), OpKind::Other);
    }

    #[test]
    fn marked_content_carries_its_tag_and_property_list_as_written() {
        let ops = Content::decode(
            b"/Artifact BMC /Span << /MCID 4 >> BDC /P /MC0 BDC /Span 5 BDC 5 BMC EMC",
        )
        .unwrap()
        .operations;
        let kinds: Vec<OpKind> = ops.iter().map(OpKind::of).collect();
        let mut mcid = lopdf::Dictionary::new();
        mcid.set("MCID", Object::Integer(4));
        assert_eq!(
            kinds,
            [
                OpKind::BeginMarked {
                    tag: b"Artifact".to_vec(),
                    props: MarkedProps::None,
                },
                OpKind::BeginMarked {
                    tag: b"Span".to_vec(),
                    props: MarkedProps::Inline(mcid),
                },
                OpKind::BeginMarked {
                    tag: b"P".to_vec(),
                    props: MarkedProps::Named(b"MC0".to_vec()),
                },
                OpKind::BeginMarked {
                    tag: b"Span".to_vec(),
                    props: MarkedProps::None,
                },
                OpKind::BeginMarked {
                    tag: Vec::new(),
                    props: MarkedProps::None,
                },
                OpKind::EndMarked,
            ]
        );
    }

    // ---------------------------------------------------------------------------------------
    // The strict decoder
    // ---------------------------------------------------------------------------------------

    fn deflated(bytes: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(bytes).unwrap();
        enc.finish().unwrap()
    }

    fn flate_stream(content: Vec<u8>) -> Stream {
        let mut dict = lopdf::Dictionary::new();
        dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
        Stream::new(dict, content)
    }

    const PAGE: &[u8] = b"BT /F1 12 Tf 72 700 Td (Hello, strict decoder) Tj ET\n";

    #[test]
    fn a_complete_flate_stream_inflates_to_what_lopdf_reads() {
        let stream = flate_stream(deflated(PAGE));
        assert_eq!(decode_stream_strict(&stream).unwrap(), PAGE);
        assert_eq!(stream.decompressed_content().unwrap(), PAGE);
    }

    #[test]
    fn a_truncated_flate_stream_is_malformed() {
        let whole = deflated(PAGE);
        for keep in [whole.len() - 1, whole.len() - 5, whole.len() / 2, 3] {
            let stream = flate_stream(whole[..keep].to_vec());
            let e = decode_stream_strict(&stream).unwrap_err();
            assert!(
                matches!(&e, StreamRefusal::Malformed(d) if d.contains("without reaching the end")),
                "{keep} of {} bytes: {e:?}",
                whole.len()
            );
        }
    }

    /// Why `inflate_strict` drives `flate2::Decompress` itself: the reader API accepts the cut.
    #[test]
    fn the_reader_api_accepts_a_cut_stream_and_consumes_all_of_it() {
        use std::io::Read;
        let whole = deflated(PAGE);
        let cut = &whole[..whole.len() - 5];
        let mut reader = flate2::read::ZlibDecoder::new(cut);
        let mut out = Vec::new();
        reader
            .read_to_end(&mut out)
            .expect("the reader reports no error on a truncated stream");
        assert_eq!(
            reader.total_in(),
            cut.len() as u64,
            "and every input byte was consumed, so `total_in` cannot tell either"
        );
        assert!(
            out.len() < PAGE.len(),
            "while the output is short: {} of {} bytes",
            out.len(),
            PAGE.len()
        );
    }

    #[test]
    fn a_flate_stream_whose_check_fails_is_malformed() {
        let mut broken = deflated(PAGE);
        let last = broken.len() - 1;
        broken[last] ^= 0xff;
        let stream = flate_stream(broken);
        let e = decode_stream_strict(&stream).unwrap_err();
        assert!(
            matches!(&e, StreamRefusal::Malformed(d) if d.contains("corrupt or its check fails")),
            "{e:?}"
        );
        // lopdf keeps the partial output and reports success: the reason the reader's raw-bytes
        // fallback cannot be relied on to fail (scope §3.6).
        assert_eq!(stream.decompressed_content().unwrap(), PAGE);
    }

    #[test]
    fn bytes_after_the_deflate_data_are_malformed() {
        let mut long = deflated(PAGE);
        long.extend_from_slice(b"\r\n");
        let stream = flate_stream(long);
        let e = decode_stream_strict(&stream).unwrap_err();
        assert!(
            matches!(&e, StreamRefusal::Malformed(d) if d.contains("2 byte(s) follow the end")),
            "{e:?}"
        );
    }

    #[test]
    fn an_empty_flate_stream_is_empty_as_lopdf_reads_it() {
        let stream = flate_stream(Vec::new());
        assert_eq!(decode_stream_strict(&stream).unwrap(), b"");
        assert_eq!(stream.decompressed_content().unwrap(), b"");
    }

    #[test]
    fn filters_other_than_flate_are_unsupported_by_name() {
        for (filter, named) in [
            ("ASCIIHexDecode", "/ASCIIHexDecode"),
            (
                "LZWDecode",
                "/LZWDecode: lopdf's decoder returns the partial output",
            ),
            (
                "ASCII85Decode",
                "/ASCII85Decode: lopdf's decoder returns the partial output",
            ),
            ("RunLengthDecode", "/RunLengthDecode"),
            ("DCTDecode", "/DCTDecode"),
        ] {
            let mut dict = lopdf::Dictionary::new();
            dict.set("Filter", Object::Name(filter.as_bytes().to_vec()));
            let e = decode_stream_strict(&Stream::new(dict, b"anything".to_vec())).unwrap_err();
            assert!(
                matches!(&e, StreamRefusal::Unsupported(d) if d.starts_with(named)),
                "{filter}: {e:?}"
            );
        }
    }

    #[test]
    fn filter_arrays_and_odd_filter_values_are_unsupported_by_name() {
        let cases: Vec<(Object, &str)> = vec![
            (
                Object::Array(vec![Object::Name(b"FlateDecode".to_vec())]),
                "",
            ),
            (Object::Array(vec![]), "/Filter is an empty array"),
            (
                Object::Array(vec![
                    Object::Name(b"ASCII85Decode".to_vec()),
                    Object::Name(b"FlateDecode".to_vec()),
                ]),
                "/Filter chains 2 filters",
            ),
            (
                Object::Array(vec![Object::Integer(1)]),
                "/Filter is a one-entry array holding Integer",
            ),
            (
                Object::Reference((9, 0)),
                "/Filter is Reference rather than a name",
            ),
        ];
        for (filter, expect) in cases {
            let mut dict = lopdf::Dictionary::new();
            dict.set("Filter", filter.clone());
            let stream = Stream::new(dict, deflated(PAGE));
            match decode_stream_strict(&stream) {
                Ok(bytes) => assert!(expect.is_empty() && bytes == PAGE, "{filter:?}"),
                Err(e) => assert!(
                    !expect.is_empty()
                        && matches!(&e, StreamRefusal::Unsupported(d) if d.starts_with(expect)),
                    "{filter:?}: {e:?}"
                ),
            }
        }
    }

    #[test]
    fn a_predictor_is_unsupported_and_a_plain_decode_parms_is_not() {
        let with_parms = |parms: Object| {
            let mut dict = lopdf::Dictionary::new();
            dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
            dict.set("DecodeParms", parms);
            Stream::new(dict, deflated(PAGE))
        };
        let mut png = lopdf::Dictionary::new();
        png.set("Predictor", Object::Integer(12));
        png.set("Columns", Object::Integer(4));
        let e = decode_stream_strict(&with_parms(Object::Dictionary(png))).unwrap_err();
        assert!(
            matches!(&e, StreamRefusal::Unsupported(d) if d.starts_with("/DecodeParms /Predictor 12")),
            "{e:?}"
        );
        let mut tiff = lopdf::Dictionary::new();
        tiff.set("Predictor", Object::Integer(2));
        let e = decode_stream_strict(&with_parms(Object::Dictionary(tiff))).unwrap_err();
        assert!(
            matches!(&e, StreamRefusal::Unsupported(d) if d.starts_with("/DecodeParms /Predictor 2")),
            "{e:?}"
        );
        let mut none = lopdf::Dictionary::new();
        none.set("Predictor", Object::Integer(1));
        assert_eq!(
            decode_stream_strict(&with_parms(Object::Dictionary(none))).unwrap(),
            PAGE
        );
        assert_eq!(
            decode_stream_strict(&with_parms(Object::Dictionary(lopdf::Dictionary::new())))
                .unwrap(),
            PAGE
        );
        let e = decode_stream_strict(&with_parms(Object::Reference((3, 0)))).unwrap_err();
        assert!(
            matches!(&e, StreamRefusal::Unsupported(d) if d.starts_with("/DecodeParms is Reference")),
            "{e:?}"
        );
    }

    #[test]
    fn an_unfiltered_stream_is_returned_as_written() {
        let stream = Stream::new(lopdf::Dictionary::new(), PAGE.to_vec());
        assert_eq!(decode_stream_strict(&stream).unwrap(), PAGE);
    }

    /// A document with two content streams, one raw and one deflated, joined as
    /// `get_page_content` joins them; and the shapes that helper skips, refused by name.
    #[test]
    fn a_page_is_the_streams_joined_with_a_newline_after_each() {
        let mut doc = lopdf::Document::with_version("1.5");
        let raw = doc.add_object(Stream::new(
            lopdf::Dictionary::new(),
            b"q 1 0 0 1 0 0 cm".to_vec(),
        ));
        let inflated = doc.add_object(flate_stream(deflated(PAGE)));
        let page = |contents: Object, doc: &mut lopdf::Document| {
            let mut d = lopdf::Dictionary::new();
            d.set("Type", Object::Name(b"Page".to_vec()));
            d.set("Contents", contents);
            doc.add_object(Object::Dictionary(d))
        };

        let two = page(
            Object::Array(vec![Object::Reference(raw), Object::Reference(inflated)]),
            &mut doc,
        );
        let expected = {
            let mut v = b"q 1 0 0 1 0 0 cm\n".to_vec();
            v.extend_from_slice(PAGE);
            v.push(b'\n');
            v
        };
        assert_eq!(page_content_strict(&doc, two).unwrap(), expected);
        assert_eq!(doc.get_page_content(two), expected);

        let one = page(Object::Reference(inflated), &mut doc);
        assert_eq!(
            page_content_strict(&doc, one).unwrap(),
            doc.get_page_content(one)
        );

        // A reference to a reference to the stream: followed, as `get_page_contents` follows it.
        let hop = doc.add_object(Object::Reference(raw));
        let hopped = page(Object::Reference(hop), &mut doc);
        assert_eq!(
            page_content_strict(&doc, hopped).unwrap(),
            doc.get_page_content(hopped)
        );

        // No `/Contents` at all is an empty page on both sides.
        let mut d = lopdf::Dictionary::new();
        d.set("Type", Object::Name(b"Page".to_vec()));
        let blank = doc.add_object(Object::Dictionary(d));
        assert_eq!(page_content_strict(&doc, blank).unwrap(), b"");
        assert_eq!(doc.get_page_content(blank), b"");

        // The shapes `get_page_content` skips silently, each refused naming the page.
        let not_a_stream = doc.add_object(Object::Integer(7));
        for (contents, expect) in [
            (
                Object::Reference(not_a_stream),
                "/Contents is Integer rather than a reference",
            ),
            (
                Object::Array(vec![Object::Reference(raw), Object::Integer(7)]),
                "/Contents entry 1 is Integer rather than a reference",
            ),
            (
                Object::Array(vec![Object::Reference(not_a_stream)]),
                "is not a stream",
            ),
            (
                Object::Reference((999, 0)),
                "names object 999 0, which the document does not hold",
            ),
            (Object::Null, "/Contents is Null rather than"),
        ] {
            let id = page(contents, &mut doc);
            let e = page_content_strict(&doc, id).unwrap_err();
            assert_eq!(e.code(), "malformed", "{e}");
            assert!(
                e.to_string().contains(expect),
                "expected `{expect}` in: {e}"
            );
            assert!(
                e.to_string()
                    .contains(&format!("page object {} {}", id.0, id.1)),
                "the page is named: {e}"
            );
        }

        // A truncated stream reaches the page with the page and the stream named. The cut here
        // removes exactly the four-byte Adler-32 trailer, so the deflate data is whole and the
        // check is missing.
        let whole = deflated(PAGE);
        let cut = doc.add_object(flate_stream(whole[..whole.len() - 4].to_vec()));
        let id = page(Object::Reference(cut), &mut doc);
        let e = page_content_strict(&doc, id).unwrap_err();
        assert_eq!(e.code(), "malformed");
        assert!(
            e.to_string()
                .contains(&format!("stream {} {}: the inflater stopped", cut.0, cut.1)),
            "{e}"
        );
        // And lopdf's helper hands the interpreter the whole text, check or no check — the
        // silent path scope §3.6 says cannot be relied on to fail.
        let mut full = PAGE.to_vec();
        full.push(b'\n');
        assert_eq!(doc.get_page_content(id), full);

        // An unsupported filter names the page, the stream and the filter.
        let mut hex = lopdf::Dictionary::new();
        hex.set("Filter", Object::Name(b"ASCIIHexDecode".to_vec()));
        let hex = doc.add_object(Stream::new(hex, b"71 51 3e".to_vec()));
        let id = page(Object::Reference(hex), &mut doc);
        let e = page_content_strict(&doc, id).unwrap_err();
        assert_eq!(e.code(), "unsupported");
        assert!(
            e.to_string()
                .starts_with("unsupported content stream filter: page object"),
            "{e}"
        );
        assert!(e.to_string().ends_with("/ASCIIHexDecode"), "{e}");
    }

    // ---------------------------------------------------------------------------------------
    // The corpora: every PDF in fixtures/engine, fixtures/gate and the oracle fixtures
    // ---------------------------------------------------------------------------------------

    /// Every PDF in the three corpora, opened and handed to `f` with a label; the documents
    /// that do not open are returned by name rather than skipped. The oracle corpus holds
    /// fixtures that exist not to open — a corrupt header, a password — and a census that did
    /// not say so would be a census of a different corpus.
    fn for_each_corpus_document(
        mut f: impl FnMut(&str, &crate::document::Document),
    ) -> (usize, Vec<String>) {
        use crate::test_support::{corpus_root, pdfs_under};
        let profile = ethos_parser_core::Profile::default();
        let mut opened = 0usize;
        let mut unopened = Vec::new();
        for root in ["engine", "gate", "conformance"] {
            let dir = corpus_root(root);
            let pdfs = pdfs_under(&dir);
            assert!(
                !pdfs.is_empty(),
                "corpus `{root}` at {} holds no PDF. A missing corpus is a failure, never a skip.",
                dir.display()
            );
            for path in pdfs {
                let label = format!(
                    "{root}/{}",
                    path.strip_prefix(&dir).unwrap_or(&path).display()
                );
                let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{label}: {e}"));
                match crate::document::Document::open_bytes(&bytes, &profile) {
                    Ok(doc) => {
                        opened += 1;
                        f(&label, &doc);
                    }
                    Err(e) => unopened.push(format!("{label}: {}", e.code())),
                }
            }
        }
        (opened, unopened)
    }

    /// **Scope §3.5's first condition, on every page the repository can reach:** the strict
    /// buffer is `get_page_content`'s byte for byte, or the page is refused by name.
    ///
    /// The census — documents, pages, bytes, and every refusal with its reason — is printed so
    /// the commit that changes any of it can record the numbers; the floors asserted are what
    /// the corpora held when this was written, so a walk over an empty or moved root fails
    /// rather than passing on nothing.
    #[test]
    fn the_strict_decoder_matches_the_lenient_one_or_refuses_by_name() {
        let (mut pages, mut equal, mut bytes) = (0usize, 0usize, 0usize);
        let mut refusals: Vec<String> = Vec::new();
        let (opened, unopened) = for_each_corpus_document(|label, doc| {
            for &(number, id) in doc.pages() {
                pages += 1;
                let lenient = doc.inner().get_page_content(id);
                match page_content_strict(doc.inner(), id) {
                    Ok(strict) => {
                        assert!(
                            strict == lenient,
                            "{label} page {number}: the strict buffer ({} bytes) differs from \
                             get_page_content's ({} bytes)",
                            strict.len(),
                            lenient.len()
                        );
                        equal += 1;
                        bytes += strict.len();
                    }
                    Err(e) => {
                        assert!(
                            matches!(e.code(), "malformed" | "unsupported"),
                            "{label} page {number}: {e}"
                        );
                        refusals.push(format!("{label} page {number}: {e}"));
                    }
                }
            }
        });
        println!(
            "strict decoder census: {opened} document(s) opened, {} did not open, {pages} \
             page(s), {equal} equal to get_page_content over {bytes} byte(s), {} refused",
            unopened.len(),
            refusals.len()
        );
        for u in &unopened {
            println!("  did not open: {u}");
        }
        for r in &refusals {
            println!("  refused: {r}");
        }
        assert!(opened >= 89, "{opened} documents opened");
        assert!(pages >= 1_567, "{pages} pages walked");
        assert_eq!(
            equal + refusals.len(),
            pages,
            "every page is one or the other"
        );
    }

    /// **Scope §3.5's second condition, on every page whose strict decode succeeds:** the
    /// tokeniser places every byte exactly when `lopdf`'s strict parse does, and then its
    /// operators are `lopdf`'s decode's, in order, and its spans cover the buffer.
    ///
    /// A page the tokeniser refuses while `lopdf`'s lenient decode succeeds is a page on which
    /// `extract` interpreted fewer operations than the page holds and said nothing — the shape
    /// scope §3.5 says the self-check could never catch later. Each such page is printed.
    #[test]
    fn the_tokeniser_accounts_for_every_byte_and_agrees_with_lopdf() {
        let (mut pages, mut ops, mut bytes, mut undecoded) = (0usize, 0usize, 0usize, 0usize);
        let mut refusals: Vec<String> = Vec::new();
        let (opened, unopened) = for_each_corpus_document(|label, doc| {
            for &(number, id) in doc.pages() {
                let Ok(buffer) = page_content_strict(doc.inner(), id) else {
                    undecoded += 1;
                    continue;
                };
                let strict = Content::decode_strict(&buffer).is_ok();
                let lenient = Content::decode(&buffer);
                let page = format!("{label} page {number}");
                match tokenise(&buffer) {
                    Ok(t) => {
                        assert!(
                            strict,
                            "{page}: the tokeniser placed every byte and lopdf's strict parse \
                             did not"
                        );
                        let lenient = lenient.expect("strict parsed, so lenient does");
                        agrees_with_lopdf(&t, &lenient.operations)
                            .unwrap_or_else(|e| panic!("{page}: {e}"));
                        assert_every_byte_placed(&page, &buffer, &t);
                        pages += 1;
                        ops += t.ops.len();
                        bytes += buffer.len();
                    }
                    Err(e) => {
                        assert!(
                            !strict,
                            "{page}: lopdf's strict parse placed every byte and the tokeniser \
                             refused: {e}"
                        );
                        refusals.push(format!(
                            "{page}: {e} (lopdf's lenient decode {})",
                            match lenient {
                                Ok(c) => format!("reads {} operation(s)", c.operations.len()),
                                Err(_) => "fails too".to_string(),
                            }
                        ));
                    }
                }
            }
        });
        println!(
            "tokeniser census: {opened} document(s) opened, {} did not open, {pages} page(s) \
             tokenised, {ops} operation(s), {bytes} byte(s), {undecoded} page(s) not decoded \
             strictly, {} refused by the tokeniser",
            unopened.len(),
            refusals.len()
        );
        for r in &refusals {
            println!("  refused: {r}");
        }
        assert!(opened >= 89, "{opened} documents opened");
        assert!(pages >= 1_567, "{pages} pages tokenised");
        assert!(ops >= 6_697_547, "{ops} operations checked");
    }

    // ---------------------------------------------------------------------------------------
    // The placement rule, on synthetic streams
    // ---------------------------------------------------------------------------------------

    const BLOCK_A: BlockKey = (None, Some(1));
    const BLOCK_B: BlockKey = (None, Some(2));

    /// The legend every synthetic stream below is read by: a shown string's first letter names
    /// its owner — `a`/`b` a block, `h` a run inside an /Artifact frame, `x` a run the reader
    /// dropped or an empty string.
    fn legend(text: &str) -> Option<Shows> {
        match text.chars().next() {
            Some('a') => Some(Shows::Block(BLOCK_A)),
            Some('b') => Some(Shows::Block(BLOCK_B)),
            Some('h') => Some(Shows::Artifact),
            Some('x') => Some(Shows::Nothing),
            _ => panic!("the legend does not know `{text}`"),
        }
    }

    /// The text a text-showing operation shows, for the legend.
    fn shown_text(op: &Operation) -> String {
        let mut out = String::new();
        for operand in &op.operands {
            match operand {
                Object::String(bytes, _) => out.push_str(&lossy(bytes)),
                Object::Array(items) => {
                    for item in items {
                        if let Object::String(bytes, _) = item {
                            out.push_str(&lossy(bytes));
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    struct Synthetic {
        ops: Vec<Operation>,
        t: Tokenised,
        nest: Vec<Nesting>,
        shows: Vec<Option<Shows>>,
        /// Blocks by first appearance in the stream, which a test may override.
        order: Vec<BlockKey>,
    }

    fn synthetic(src: &[u8], owner: impl Fn(&str) -> Option<Shows>) -> Synthetic {
        let t = ok(src);
        let ops = Content::decode(src).expect("decodes").operations;
        agrees_with_lopdf(&t, &ops).expect("agrees");
        let nest = nesting(&ops);
        let mut order = Vec::new();
        let shows: Vec<Option<Shows>> = ops
            .iter()
            .map(|op| {
                if !matches!(OpKind::of(op), OpKind::TextShow) {
                    return None;
                }
                let s = owner(&shown_text(op));
                if let Some(Shows::Block(key)) = s {
                    if !order.contains(&key) {
                        order.push(key);
                    }
                }
                s
            })
            .collect();
        Synthetic {
            ops,
            t,
            nest,
            shows,
            order,
        }
    }

    /// Plan and splice a synthetic stream, holding everything a written page must hold: the plan
    /// passes the shared §3.4 check, the spliced bytes re-tokenise, agree with lopdf and hold
    /// exactly two more operations per sequence, and read back by the interpreter's own rule —
    /// the innermost open sequence's id — every block's text carries the id of the sequence the
    /// plan put its operator in and nothing else carries one.
    fn tagged_in_order(
        src: &[u8],
        owner: impl Fn(&str) -> Option<Shows>,
        order: Option<&[BlockKey]>,
    ) -> (String, PagePlan) {
        let s = synthetic(src, owner);
        let order = order.unwrap_or(&s.order);
        let plan = plan_page(&s.ops, &s.nest, &s.shows, order);
        sequences_are_well_formed(&s.ops, &s.nest, &s.shows, &plan.sequences)
            .unwrap_or_else(|e| panic!("{:?}: {e}", lossy(src)));
        for (block, ids) in &plan.blocks {
            assert!(ids.windows(2).all(|w| w[0] < w[1]), "{block:?}: {ids:?}");
        }
        let out = splice(src, &s.t, &plan.sequences);
        let t2 = ok(&out);
        assert_eq!(
            t2.ops.len(),
            s.ops.len() + 2 * plan.sequences.len(),
            "two operations per sequence and nothing else: {:?}",
            lossy(&out)
        );
        let ops2 = Content::decode(&out)
            .expect("the spliced stream decodes")
            .operations;
        let mut original = 0usize;
        // `Some(id)` for a written frame, `None` for one the stream already had.
        let mut stack: Vec<Option<i64>> = Vec::new();
        for op in &ops2 {
            match OpKind::of(op) {
                OpKind::BeginMarked {
                    tag,
                    props: MarkedProps::Inline(d),
                } if tag == b"Div" && d.get(b"MCID").is_ok() => {
                    stack.push(d.get(b"MCID").ok().and_then(|o| o.as_i64().ok()));
                    continue;
                }
                OpKind::BeginMarked { .. } => stack.push(None),
                OpKind::EndMarked => {
                    if let Some(Some(_)) = stack.last() {
                        stack.pop();
                        continue;
                    }
                    stack.pop();
                }
                OpKind::TextShow => {
                    let innermost = stack.last().copied().flatten();
                    let expected = plan
                        .sequences
                        .iter()
                        .find(|q| q.first <= original && original <= q.last)
                        .map(|q| q.mcid);
                    assert_eq!(
                        innermost,
                        expected,
                        "operation {original} `{}` reads back under {innermost:?}",
                        shown_text(op)
                    );
                    match s.shows[original] {
                        Some(Shows::Block(_)) => {
                            assert!(expected.is_some(), "operation {original} is left untagged")
                        }
                        _ => assert!(expected.is_none(), "operation {original} is not a block's"),
                    }
                }
                _ => {}
            }
            original += 1;
        }
        assert_eq!(
            original,
            s.ops.len(),
            "every original operation is still there"
        );
        (String::from_utf8(out).expect("ASCII in, ASCII out"), plan)
    }

    fn tagged(src: &[u8], owner: impl Fn(&str) -> Option<Shows>) -> (String, PagePlan) {
        tagged_in_order(src, owner, None)
    }

    fn splits(plan: &PagePlan) -> Vec<Option<SplitCause>> {
        plan.sequences.iter().map(|s| s.split).collect()
    }

    fn placements(plan: &PagePlan) -> Vec<Placement> {
        plan.sequences.iter().map(|s| s.placement).collect()
    }

    #[test]
    fn a_block_across_two_whole_text_objects_is_one_sequence() {
        let (out, plan) = tagged(b"BT (a1) Tj ET BT (a2) Tj ET", legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nBT (a1) Tj ET BT (a2) Tj ET\nEMC"
        );
        assert_eq!(plan.sequences.len(), 1);
        assert_eq!(placements(&plan), [Placement::TextObject]);
        assert_eq!(splits(&plan), [None]);
        assert_eq!(plan.blocks, vec![(BLOCK_A, vec![0])]);
    }

    #[test]
    fn a_block_written_as_one_saved_state_per_line_widens_through_the_pairs() {
        let (out, plan) = tagged(b"q BT (a1) Tj ET Q q BT (a2) Tj ET Q", legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nq BT (a1) Tj ET Q q BT (a2) Tj ET Q\nEMC"
        );
        assert_eq!(placements(&plan), [Placement::GraphicsState]);
        // A colour set inside the saved state, and a save nested inside the text object, widen
        // the same way: the order of the two steps is decided by what encloses what.
        let (out, _) = tagged(b"q 0 g BT (a1) Tj ET Q BT q 1 g (a2) Tj Q ET", legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nq 0 g BT (a1) Tj ET Q BT q 1 g (a2) Tj Q ET\nEMC"
        );
    }

    #[test]
    fn a_painting_operator_between_two_lines_splits_the_block() {
        let (out, plan) = tagged(b"BT (a1) Tj ET /Im1 Do BT (a2) Tj ET", legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nBT (a1) Tj ET\nEMC /Im1 Do /Div <</MCID 1>> BDC\nBT (a2) Tj \
             ET\nEMC"
        );
        assert_eq!(splits(&plan), [None, Some(SplitCause::PaintingOperator)]);
        assert_eq!(plan.blocks, vec![(BLOCK_A, vec![0, 1])]);
        // A path painted, and a shading, split the same way; a path merely constructed and
        // clipped does not.
        let (_, plan) = tagged(b"BT (a1) Tj ET 0 0 1 1 re f BT (a2) Tj ET", legend);
        assert_eq!(splits(&plan), [None, Some(SplitCause::PaintingOperator)]);
        let (_, plan) = tagged(b"BT (a1) Tj ET /Sh sh BT (a2) Tj ET", legend);
        assert_eq!(splits(&plan), [None, Some(SplitCause::PaintingOperator)]);
        let (out, _) = tagged(b"BT (a1) Tj ET 0 0 1 1 re W n BT (a2) Tj ET", legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nBT (a1) Tj ET 0 0 1 1 re W n BT (a2) Tj ET\nEMC"
        );
    }

    #[test]
    fn an_inline_image_is_never_enclosed() {
        let src = b"BT (a1) Tj ET BI /W 1 /H 1 /CS /G /BPC 8 ID \xff EI BT (a2) Tj ET";
        let s = synthetic(src, legend);
        let plan = plan_page(&s.ops, &s.nest, &s.shows, &s.order);
        assert_eq!(splits(&plan), [None, Some(SplitCause::PaintingOperator)]);
        let image = s
            .ops
            .iter()
            .position(|o| o.operator == "BI")
            .expect("the image is one operation");
        assert!(
            plan.sequences
                .iter()
                .all(|q| image < q.first || q.last < image),
            "the image sits between the sequences: {:?}",
            plan.sequences
        );
        let out = splice(src, &s.t, &plan.sequences);
        assert_eq!(
            out,
            b"/Div <</MCID 0>> BDC\nBT (a1) Tj ET\nEMC BI /W 1 /H 1 /CS /G /BPC 8 ID \xff EI \
              /Div <</MCID 1>> BDC\nBT (a2) Tj ET\nEMC"
                .as_slice()
        );
    }

    #[test]
    fn a_named_frame_splits_the_block_and_the_sequence_sits_inside_it() {
        let src = b"BT (a1) Tj ET /OC /oc1 BDC BT (a2) Tj ET EMC BT (a3) Tj ET";
        let resolve = |name: &[u8]| {
            assert_eq!(name, b"oc1");
            PropertyList::WithoutId
        };
        let ops = Content::decode(src).unwrap().operations;
        refuse_ids(&ops, 1, &resolve).expect("a named list without an id is a frame");
        let (out, plan) = tagged(src, legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nBT (a1) Tj ET\nEMC /OC /oc1 BDC /Div <</MCID 1>> BDC\nBT \
             (a2) Tj ET\nEMC EMC /Div <</MCID 2>> BDC\nBT (a3) Tj ET\nEMC"
        );
        assert_eq!(
            splits(&plan),
            [
                None,
                Some(SplitCause::ExistingFrame),
                Some(SplitCause::ExistingFrame)
            ]
        );
        let frame = ops.iter().position(|o| o.operator == "BDC").unwrap();
        let s = synthetic(src, legend);
        assert_eq!(
            s.nest[plan.sequences[1].first].frame,
            Some(frame),
            "the second sequence opens inside the frame"
        );
        assert_eq!(s.nest[plan.sequences[0].first].frame, None);
        assert_eq!(s.nest[plan.sequences[2].first].frame, None);
    }

    #[test]
    fn an_artifact_frame_between_two_lines_splits_and_is_never_enclosed() {
        let src = b"BT (a1) Tj ET /Artifact BMC BT (h1) Tj ET EMC BT (a2) Tj ET";
        let (out, plan) = tagged(src, legend);
        assert_eq!(
            out,
            "/Div <</MCID 0>> BDC\nBT (a1) Tj ET\nEMC /Artifact BMC BT (h1) Tj ET EMC /Div \
             <</MCID 1>> BDC\nBT (a2) Tj ET\nEMC"
        );
        assert_eq!(splits(&plan), [None, Some(SplitCause::Artifact)]);
        // Furniture with no frame of its own — a run the page marked /Artifact by nesting — is
        // foreign too, and named as such.
        let (_, plan) = tagged(b"BT (a1) Tj (h1) Tj (a2) Tj ET", legend);
        assert_eq!(splits(&plan), [None, Some(SplitCause::Artifact)]);
    }

    #[test]
    fn a_foreign_operator_inside_the_text_object_splits_at_the_operators() {
        let (out, plan) = tagged(b"BT (a1) Tj (b1) Tj (a2) Tj ET", legend);
        assert_eq!(
            out,
            "BT /Div <</MCID 0>> BDC\n(a1) Tj\nEMC /Div <</MCID 1>> BDC\n(b1) Tj\nEMC /Div \
             <</MCID 2>> BDC\n(a2) Tj\nEMC ET"
        );
        assert_eq!(
            splits(&plan),
            [None, None, Some(SplitCause::ForeignRun)],
            "the third sequence is block A's second, ended by block B's operator"
        );
        assert_eq!(
            placements(&plan),
            [
                Placement::Operators,
                Placement::Operators,
                Placement::Operators
            ]
        );
        assert_eq!(
            plan.blocks,
            vec![(BLOCK_A, vec![0, 2]), (BLOCK_B, vec![1])],
            "ids dense in stream order, each block's ascending"
        );
        // A dropped or empty run is foreign the same way, and is never enclosed.
        let (out, plan) = tagged(b"BT (a1) Tj (x1) Tj (a2) Tj ET", legend);
        assert_eq!(
            out,
            "BT /Div <</MCID 0>> BDC\n(a1) Tj\nEMC (x1) Tj /Div <</MCID 1>> BDC\n(a2) \
             Tj\nEMC ET"
        );
        assert_eq!(splits(&plan), [None, Some(SplitCause::ForeignRun)]);
    }

    #[test]
    fn a_shared_text_object_keeps_a_block_from_leaving_it() {
        // Block A's first line shares a text object with block B; its second has one of its own.
        // The first sequence cannot widen past B's operator, so the two never meet at one depth.
        let (out, plan) = tagged(b"BT (b1) Tj (a1) Tj ET BT (a2) Tj ET", legend);
        assert_eq!(
            out,
            "BT /Div <</MCID 0>> BDC\n(b1) Tj\nEMC /Div <</MCID 1>> BDC\n(a1) Tj\nEMC ET /Div \
             <</MCID 2>> BDC\nBT (a2) Tj ET\nEMC"
        );
        assert_eq!(
            splits(&plan),
            [None, None, Some(SplitCause::SharedTextObject)]
        );
        // An unbalanced save between two text objects of one block is the graphics-state cause.
        let (_, plan) = tagged(b"BT (a1) Tj ET q BT (a2) Tj ET", legend);
        assert_eq!(splits(&plan), [None, Some(SplitCause::GraphicsState)]);
    }

    #[test]
    fn the_sequence_opens_before_the_first_lines_positioning() {
        // The convention `engine-tagged-blocks` is written to: inside a shared text object the
        // sequence opens before the `Tm` of its first line and after the `Tf` that serves every
        // block, so its own position rides inside it and state that persists past it does not.
        let (out, plan) = tagged(
            b"BT /F1 12 Tf 1 0 0 1 72 700 Tm (a1) Tj 1 0 0 1 72 686 Tm (a2) Tj 1 0 0 1 72 644 \
              Tm (b1) Tj ET",
            legend,
        );
        assert_eq!(
            out,
            "BT /F1 12 Tf /Div <</MCID 0>> BDC\n1 0 0 1 72 700 Tm (a1) Tj 1 0 0 1 72 686 Tm \
             (a2) Tj\nEMC /Div <</MCID 1>> BDC\n1 0 0 1 72 644 Tm (b1) Tj\nEMC ET"
        );
        assert_eq!(plan.blocks, vec![(BLOCK_A, vec![0]), (BLOCK_B, vec![1])]);
        // Every operator of Table 108 is positioning; a text-state operator between the
        // positioning and the text ends the walk back.
        let (out, _) = tagged(b"BT 0 -14 Td (a1) Tj 0 -14 TD T* (b1) Tj ET", legend);
        assert_eq!(
            out,
            "BT /Div <</MCID 0>> BDC\n0 -14 Td (a1) Tj\nEMC /Div <</MCID 1>> BDC\n0 -14 TD T* \
             (b1) Tj\nEMC ET"
        );
        let (out, _) = tagged(b"BT 1 0 0 1 0 0 Tm /F1 12 Tf (a1) Tj (b1) Tj ET", legend);
        assert_eq!(
            out,
            "BT 1 0 0 1 0 0 Tm /F1 12 Tf /Div <</MCID 0>> BDC\n(a1) Tj\nEMC /Div <</MCID 1>> \
             BDC\n(b1) Tj\nEMC ET"
        );
    }

    #[test]
    fn ids_follow_stream_order_and_elements_follow_reading_order() {
        // The two-column shape: block B is written first, and block A is read first.
        let (out, plan) = tagged_in_order(
            b"BT (b1) Tj (b2) Tj (a1) Tj (a2) Tj ET",
            legend,
            Some(&[BLOCK_A, BLOCK_B]),
        );
        assert_eq!(
            out,
            "BT /Div <</MCID 0>> BDC\n(b1) Tj (b2) Tj\nEMC /Div <</MCID 1>> BDC\n(a1) Tj (a2) \
             Tj\nEMC ET"
        );
        assert_eq!(plan.blocks, vec![(BLOCK_A, vec![1]), (BLOCK_B, vec![0])]);
        // Interleaved, so each block is several sequences and every `/K` stays ascending.
        let (_, plan) = tagged_in_order(
            b"BT (b1) Tj (a1) Tj (b2) Tj (a2) Tj ET",
            legend,
            Some(&[BLOCK_A, BLOCK_B]),
        );
        assert_eq!(
            plan.blocks,
            vec![(BLOCK_A, vec![1, 3]), (BLOCK_B, vec![0, 2])]
        );
        // A block in the reading order with no operator of its own gets no element.
        let (_, plan) = tagged_in_order(b"BT (a1) Tj ET", legend, Some(&[BLOCK_B, BLOCK_A]));
        assert_eq!(plan.blocks, vec![(BLOCK_A, vec![0])]);
    }

    #[test]
    fn the_splice_inserts_at_token_boundaries_and_nothing_else_moves() {
        // Operations that abut without whitespace, a comment, and a trailing newline: the
        // inserted tokens land where the tokeniser said an operation starts and ends.
        let (out, _) = tagged(b"BT(a1)Tj(b1)Tj%note\nET\n", legend);
        assert_eq!(
            out,
            "BT/Div <</MCID 0>> BDC\n(a1)Tj\nEMC\n/Div <</MCID 1>> BDC\n(b1)Tj\nEMC\n%note\nET\n"
        );
        // Whitespace already after the last operator gets no second newline, and a space there
        // is kept as it was.
        let (out, _) = tagged(b"BT (a1) Tj\n(b1) Tj ET", legend);
        assert_eq!(
            out,
            "BT /Div <</MCID 0>> BDC\n(a1) Tj\nEMC\n/Div <</MCID 1>> BDC\n(b1) Tj\nEMC ET"
        );
    }

    #[test]
    fn nesting_tracks_frames_depths_and_artifacts() {
        let ops = Content::decode(b"q BT /Artifact BMC /Span BDC (h) Tj EMC EMC ET Q EMC Q")
            .unwrap()
            .operations;
        let n = nesting(&ops);
        assert_eq!(n.len(), ops.len() + 1);
        let at = |i: usize| (n[i].frame, n[i].q, n[i].bt, n[i].artifact);
        assert_eq!(at(0), (None, 0, 0, false), "before q");
        assert_eq!(at(2), (None, 1, 1, false), "before the artifact BMC");
        assert_eq!(at(3), (Some(2), 1, 1, true), "before the nested BDC");
        assert_eq!(at(4), (Some(3), 1, 1, true), "the whole stack is asked");
        assert_eq!(at(6), (Some(2), 1, 1, true), "before the outer EMC");
        assert_eq!(at(7), (None, 1, 1, false), "before ET");
        assert_eq!(at(9), (None, 0, 0, false), "a stray EMC pops nothing");
        assert_eq!(at(10), (None, 0, 0, false), "before the stray Q");
        assert_eq!(at(11), (None, -1, 0, false), "and a stray Q dips");
    }

    #[test]
    fn ids_in_the_content_stream_are_refused_by_name() {
        let refused = |src: &[u8], resolve: &dyn Fn(&[u8]) -> PropertyList, needle: &str| {
            let ops = Content::decode(src).unwrap().operations;
            let e = refuse_ids(&ops, 3, resolve).expect_err("refused");
            assert_eq!(e.code(), "unsupported");
            let msg = e.to_string();
            assert!(
                msg.starts_with("unsupported tagging: page 3, operation "),
                "{msg}"
            );
            assert!(msg.contains(needle), "{msg}");
        };
        let never = |_: &[u8]| panic!("an inline list is not resolved");
        refused(
            b"BT /P <</MCID 0>> BDC (a) Tj EMC ET",
            &never,
            "carries a marked-content id in the content stream",
        );
        refused(
            b"/Span /MC0 BDC EMC",
            &|_: &[u8]| PropertyList::WithId,
            "`/Span /MC0 BDC` names a property list that carries `/MCID`",
        );
        refused(
            b"/P /MC1 BDC EMC",
            &|_: &[u8]| PropertyList::Unresolved,
            "does not hold, so whether it carries an id cannot be said",
        );
        refused(
            b"/P /MC2 BDC EMC",
            &|_: &[u8]| PropertyList::NotADictionary("Stream".into()),
            "resolves to Stream rather than a dictionary",
        );
        // A named list without an id, a `BMC`, and a `BDC` whose second operand is neither a
        // name nor a dictionary are frames, not refusals.
        let ops = Content::decode(b"/OC /oc1 BDC /Artifact BMC /Span 5 BDC EMC EMC EMC")
            .unwrap()
            .operations;
        refuse_ids(&ops, 3, &|_: &[u8]| PropertyList::WithoutId).expect("frames only");
    }

    fn a_run(text: &str, region: Option<u32>, block: Option<u32>, artifact: bool) -> TextRun {
        use ethos_parser_core::{GeometryAbsence, GeometryPresence, PdfArtifactLocator};
        let mut alloc = ethos_parser_core::IdAllocator::new(
            ethos_parser_core::Profile::default()
                .profile_sha256()
                .unwrap(),
        );
        TextRun {
            id: alloc.next(ethos_parser_core::IdKind::Span).unwrap(),
            text: text.to_string(),
            char_codes: text.bytes().map(u32::from).collect(),
            scalar_code_mismatch: false,
            synthesized: Vec::new(),
            font_id: "F1".into(),
            font_size: 1200,
            locator: crate::nodes::PdfLocator {
                page: 1,
                origin_x: 7200,
                origin_y: 2000,
                advance: Some(1000),
            },
            geometry: GeometryPresence::Absent(GeometryAbsence::NotReportedByReader),
            region,
            block,
            mcid: None,
            structural: artifact.then_some(ethos_parser_core::StructuralLocator::PdfArtifact(
                PdfArtifactLocator { mcid: None },
            )),
            derivation: ethos_parser_core::DerivationClass::Extracted,
            findings: Vec::new(),
        }
    }

    #[test]
    fn what_each_operator_shows_comes_from_the_runs_and_their_positions() {
        let ops = Content::decode(b"BT (a) Tj [(b) (c)] TJ ( ) Tj (h) Tj ET")
            .unwrap()
            .operations;
        let runs = [
            a_run("a", None, Some(1), false),
            a_run("b", None, Some(1), false),
            a_run("c", None, Some(1), false),
            a_run("h", None, Some(2), true),
        ];
        // The empty-looking run at operation 3 was dropped and has no position at all.
        let positions = [1, 2, 2, 4];
        let shows = shows_from_runs(&ops, 1, &runs, &positions).expect("consistent");
        assert_eq!(
            shows,
            [
                None,
                Some(Shows::Block((None, Some(1)))),
                Some(Shows::Block((None, Some(1)))),
                Some(Shows::Nothing),
                Some(Shows::Artifact),
                None
            ]
        );
        assert_eq!(
            blocks_in_reading_order(&runs),
            [(None, Some(1))],
            "an artifact run opens no block"
        );

        // One `TJ` whose strings the cut placed in two blocks is refused by name.
        let split = [
            a_run("a", None, Some(1), false),
            a_run("b", Some(1), Some(1), false),
            a_run("c", Some(2), Some(1), false),
        ];
        let e = shows_from_runs(&ops, 7, &split, &[1, 2, 2]).expect_err("two blocks");
        assert_eq!(e.code(), "unsupported");
        assert!(
            e.to_string().contains(
                "page 7, operation 2 shows runs the cut placed in two blocks (block (Some(1), \
                 Some(1)) for `b` and block (Some(2), Some(1)) for `c`)"
            ),
            "{e}"
        );
        // A position naming an operation that shows nothing cannot come from extraction.
        let e = shows_from_runs(&ops, 7, &runs[..1], &[0]).expect_err("BT shows nothing");
        assert_eq!(e.code(), "malformed");
        assert!(e.to_string().contains("names operation 0"), "{e}");
    }

    #[test]
    fn the_well_formedness_check_names_the_first_failing_condition() {
        let s = synthetic(
            b"BT (a1) Tj ET /Im Do BT (b1) Tj ET /Artifact BMC BT (h1) Tj ET EMC",
            legend,
        );
        let seq = |block: BlockKey, first: usize, last: usize, mcid: i64| Sequence {
            block,
            first,
            last,
            mcid,
            placement: Placement::Operators,
            split: None,
        };
        let check = |seqs: &[Sequence]| {
            sequences_are_well_formed(&s.ops, &s.nest, &s.shows, seqs).expect_err("ill-formed")
        };
        assert_eq!(
            check(&[seq(BLOCK_A, 0, 2, 1)]),
            "sequence 0 in stream order carries id 1: ids must be dense from 0 in stream order"
        );
        assert!(check(&[seq(BLOCK_A, 0, 4, 0)]).contains("the painting operator `Do`"));
        assert!(check(&[seq(BLOCK_A, 4, 7, 0)]).contains("text-showing operator of block"));
        assert!(check(&[seq(BLOCK_A, 0, 0, 0)]).contains("encloses no text-showing operator"));
        assert!(check(&[seq(BLOCK_A, 0, 1, 0)]).contains("opens at frame None q 0 bt 0 and closes"));
        assert!(check(&[seq(BLOCK_A, 0, 2, 0), seq(BLOCK_B, 2, 7, 1)])
            .contains("inside the previous sequence"));
        assert!(check(&[seq(BLOCK_A, 0, 2, 0), seq(BLOCK_A, 8, 12, 1)])
            .contains("spans operations 8..=12 of 12"));
        assert!(check(&[seq(BLOCK_B, 4, 8, 0)]).contains("the opening of an /Artifact frame"));
        assert!(check(&[seq(BLOCK_A, 9, 11, 0)]).contains("sits inside an /Artifact frame"));
        // A sequence that leaves one text object and enters the next dips below its own depth,
        // and the boundary check would not see it: both ends are inside a text object.
        let two = synthetic(b"BT (a1) Tj ET BT (a2) Tj ET", legend);
        let e =
            sequences_are_well_formed(&two.ops, &two.nest, &two.shows, &[seq(BLOCK_A, 1, 4, 0)])
                .expect_err("straddles");
        assert!(e.contains("dips below its own depth at operation 3"), "{e}");
        sequences_are_well_formed(
            &s.ops,
            &s.nest,
            &s.shows,
            &[seq(BLOCK_A, 0, 2, 0), seq(BLOCK_B, 4, 6, 1)],
        )
        .expect("the plan the rule makes");
    }

    // ---------------------------------------------------------------------------------------
    // The placement rule, on the fixtures
    // ---------------------------------------------------------------------------------------

    /// One page of an engine fixture, planned exactly as the writer plans it, and its spliced
    /// stream.
    fn planned_fixture(name: &str, page_number: u32) -> (PagePlan, Vec<u8>, Vec<TextRun>) {
        let bytes = crate::test_support::engine_fixture(&format!("{name}/document.pdf"));
        let profile = ethos_parser_core::Profile::default();
        let doc = crate::document::Document::open_bytes(&bytes, &profile).expect("opens");
        let (artifact, positions) =
            crate::extract::extract_with_positions(&doc, &profile).expect("extracts");
        let page = artifact
            .pages
            .iter()
            .find(|p| p.index == page_number)
            .expect("the page was processed");
        let page_id = doc
            .pages()
            .iter()
            .find(|(n, _)| *n == page_number)
            .map(|(_, id)| *id)
            .unwrap();
        let buffer = page_content_strict(doc.inner(), page_id).expect("decodes strictly");
        let t = tokenise(&buffer).expect("tokenises");
        let ops = Content::decode(&buffer).unwrap().operations;
        agrees_with_lopdf(&t, &ops).expect("agrees");
        let nest = nesting(&ops);
        let shows = shows_from_runs(&ops, page_number, &page.runs, &positions[&page_number])
            .expect("consistent");
        let order = blocks_in_reading_order(&page.runs);
        let plan = plan_page(&ops, &nest, &shows, &order);
        sequences_are_well_formed(&ops, &nest, &shows, &plan.sequences).expect("well formed");
        let out = splice(&buffer, &t, &plan.sequences);
        (plan, out, page.runs.clone())
    }

    fn tokens(bytes: &[u8]) -> Vec<String> {
        lossy(bytes).split_whitespace().map(str::to_owned).collect()
    }

    /// **Scope §3.4's shared-text-object case, on the block cut's own fixture.** Six `Tj`s in
    /// one text object, two blocks: two sequences, ids 0 and 1, each opened inside the text
    /// object before its block's first `Tm` and closed after its last `Tj` — the token sequence
    /// `engine-tagged-blocks` is written to on `fix/s1-review`, byte for byte apart from the
    /// newlines the writer puts around each inserted token group.
    #[test]
    fn the_leading_gap_page_is_two_sequences_inside_its_one_text_object() {
        let (plan, out, _) = planned_fixture("leading-gap-two-blocks", 1);
        assert_eq!(plan.sequences.len(), 2);
        assert_eq!(
            placements(&plan),
            [Placement::Operators, Placement::Operators]
        );
        assert_eq!(splits(&plan), [None, None]);
        assert_eq!(
            plan.blocks,
            vec![((None, Some(1)), vec![0]), ((None, Some(2)), vec![1])]
        );
        assert_eq!(
            out,
            b"BT /F1 12 Tf /Div <</MCID 0>> BDC\n1 0 0 1 72 700 Tm (Water finds its level) Tj \
              1 0 0 1 72 686 Tm (and stone keeps its shape) Tj 1 0 0 1 72 672 Tm (through the \
              long season) Tj\nEMC /Div <</MCID 1>> BDC\n1 0 0 1 72 644 Tm (Wind moves the \
              grass) Tj 1 0 0 1 72 630 Tm (and light moves the shade) Tj 1 0 0 1 72 616 Tm \
              (across the open field) Tj\nEMC ET\n"
                .as_slice()
        );
        assert_eq!(
            tokens(&out),
            tokens(
                b"BT /F1 12 Tf /Div <</MCID 0>> BDC 1 0 0 1 72 700 Tm (Water finds its level) \
                  Tj 1 0 0 1 72 686 Tm (and stone keeps its shape) Tj 1 0 0 1 72 672 Tm \
                  (through the long season) Tj EMC /Div <</MCID 1>> BDC 1 0 0 1 72 644 Tm \
                  (Wind moves the grass) Tj 1 0 0 1 72 630 Tm (and light moves the shade) Tj \
                  1 0 0 1 72 616 Tm (across the open field) Tj EMC ET"
            ),
            "the convention of the revised engine-tagged-blocks stream"
        );
    }

    /// The two-column page: one text object shared by two bands written right column first.
    /// Each band is one block and one sequence; ids follow the stream (the right column is 0)
    /// and the elements follow the reading order (the left column's `/Div` first).
    #[test]
    fn the_two_column_page_is_one_sequence_per_band_with_the_left_element_first() {
        let (plan, out, runs) = planned_fixture("two-column-15-lines", 1);
        assert_eq!(runs.len(), 15);
        assert_eq!(plan.sequences.len(), 2);
        assert_eq!(
            plan.blocks,
            vec![((Some(1), Some(1)), vec![1]), ((Some(2), Some(2)), vec![0])],
            "each band is one block, numbered per page in reading order"
        );
        assert_eq!(
            placements(&plan),
            [Placement::Operators, Placement::Operators]
        );
        let text = lossy(&out);
        assert!(
            text.contains("/Div <</MCID 0>> BDC\n1 0 0 1 240 260 Tm (R1) Tj"),
            "{text}"
        );
        assert!(
            text.contains("(R7) Tj\nEMC /Div <</MCID 1>> BDC\n1 0 0 1 40 260 Tm (L1) Tj"),
            "{text}"
        );
        assert!(text.ends_with("(L8) Tj\nEMC ET\n"), "{text}");
    }

    /// Four runs on one baseline, each in its own text object: the block cut does not divide
    /// a single line, so the page is one block, and four whole text objects separated by nothing
    /// widen and merge into one sequence.
    #[test]
    fn the_shredded_line_is_one_sequence_over_its_four_text_objects() {
        let (plan, out, runs) = planned_fixture("untagged-shredded-line", 1);
        let keys: std::collections::BTreeSet<BlockKey> =
            runs.iter().map(|r| (r.region, r.block)).collect();
        assert_eq!(keys.len(), 1, "one block: {keys:?}");
        assert_eq!(plan.sequences.len(), 1, "{:?}", plan.sequences);
        assert_eq!(placements(&plan), [Placement::TextObject]);
        assert_eq!(
            out,
            b"/Div <</MCID 0>> BDC\nBT /F1 24 Tf 72 100 Td (Yar) Tj ET BT /F1 24 Tf 108 100 Td \
              (ro) Tj ET BT /F1 24 Tf 132 100 Td (w) Tj ET BT /F1 24 Tf 184 100 Td (Separate) \
              Tj ET\nEMC\n"
                .as_slice()
        );
    }

    // ---------------------------------------------------------------------------------------
    // The tree and the stamp
    // ---------------------------------------------------------------------------------------

    /// A document with `pages` empty pages under a catalog, as lopdf builds one.
    fn blank_document(pages: usize) -> (lopdf::Document, Vec<ObjectId>) {
        let mut doc = lopdf::Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut kids = Vec::new();
        for _ in 0..pages {
            let mut page = lopdf::Dictionary::new();
            page.set("Type", Object::Name(b"Page".to_vec()));
            page.set("Parent", Object::Reference(pages_id));
            kids.push(doc.add_object(Object::Dictionary(page)));
        }
        let mut tree = lopdf::Dictionary::new();
        tree.set("Type", Object::Name(b"Pages".to_vec()));
        tree.set(
            "Kids",
            Object::Array(kids.iter().map(|k| Object::Reference(*k)).collect()),
        );
        tree.set("Count", Object::Integer(pages as i64));
        doc.set_object(pages_id, Object::Dictionary(tree));
        let mut catalog = lopdf::Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));
        (doc, kids)
    }

    fn dict_of(doc: &lopdf::Document, o: &Object) -> lopdf::Dictionary {
        doc.dereference(o)
            .expect("resolves")
            .1
            .as_dict()
            .expect("a dictionary")
            .clone()
    }

    #[test]
    fn the_tree_is_one_document_over_one_div_per_block_with_a_parent_tree() {
        let (mut doc, pages) = blank_document(3);
        let seq = |block: BlockKey, mcid: i64| Sequence {
            block,
            first: 0,
            last: 0,
            mcid,
            placement: Placement::Operators,
            split: None,
        };
        // Page 1: block A is ids 0 and 2, block B is id 1. Page 2 has no sequence and is not
        // rewritten. Page 3: one block, one sequence.
        let first = PagePlan {
            sequences: vec![seq(BLOCK_A, 0), seq(BLOCK_B, 1), seq(BLOCK_A, 2)],
            blocks: vec![(BLOCK_A, vec![0, 2]), (BLOCK_B, vec![1])],
        };
        let third = PagePlan {
            sequences: vec![seq(BLOCK_A, 0)],
            blocks: vec![(BLOCK_A, vec![0])],
        };
        let before = doc.max_id;
        let elements = write_tree(
            &mut doc,
            &[(pages[0], &first), (pages[2], &third)],
            "some-rule-v9",
        )
        .expect("writes");
        assert_eq!(elements, 4, "one /Document and three /Div");
        assert_eq!(
            doc.max_id,
            before + 5,
            "the root, the document element and three /Div, numbered in that order"
        );

        let catalog = doc.catalog().unwrap().clone();
        let root = dict_of(&doc, catalog.get(b"StructTreeRoot").expect("attached"));
        assert_eq!(
            root.get(b"Type").unwrap().as_name().unwrap(),
            b"StructTreeRoot"
        );
        assert_eq!(
            root.get(b"ParentTreeNextKey").unwrap().as_i64().unwrap(),
            2,
            "one key per rewritten page"
        );
        let document = dict_of(&doc, &root.get(b"K").unwrap().as_array().unwrap()[0]);
        assert_eq!(document.get(b"S").unwrap().as_name().unwrap(), b"Document");
        assert_eq!(
            document.get(b"P").unwrap(),
            catalog.get(b"StructTreeRoot").unwrap()
        );
        let expected_attribute = {
            let mut a = lopdf::Dictionary::new();
            a.set("O", Object::Name(b"EthosParser".to_vec()));
            a.set("Derivation", Object::Name(b"Computed".to_vec()));
            a.set("Rule", Object::string_literal("some-rule-v9"));
            a
        };
        assert_eq!(
            document.get(b"A").unwrap().as_dict().unwrap(),
            &expected_attribute,
            "the rule string comes from the caller, never a literal"
        );
        let divs = document.get(b"K").unwrap().as_array().unwrap().clone();
        assert_eq!(divs.len(), 3, "blocks in page order, then reading order");
        let div = |i: usize| dict_of(&doc, &divs[i]);
        for i in 0..3 {
            let d = div(i);
            assert_eq!(d.get(b"Type").unwrap().as_name().unwrap(), b"StructElem");
            assert_eq!(d.get(b"S").unwrap().as_name().unwrap(), b"Div");
            assert_eq!(
                d.get(b"P").unwrap(),
                &root.get(b"K").unwrap().as_array().unwrap()[0]
            );
            assert_eq!(d.get(b"A").unwrap().as_dict().unwrap(), &expected_attribute);
        }
        assert_eq!(div(0).get(b"Pg").unwrap(), &Object::Reference(pages[0]));
        assert_eq!(
            div(0).get(b"K").unwrap(),
            &Object::Array(vec![Object::Integer(0), Object::Integer(2)])
        );
        assert_eq!(
            div(1).get(b"K").unwrap(),
            &Object::Array(vec![Object::Integer(1)])
        );
        assert_eq!(div(2).get(b"Pg").unwrap(), &Object::Reference(pages[2]));
        assert_eq!(
            div(2).get(b"K").unwrap(),
            &Object::Array(vec![Object::Integer(0)])
        );

        // The parent tree indexes each page's ids by position to the element that cites them.
        let parent_tree = root.get(b"ParentTree").unwrap().as_dict().unwrap();
        assert_eq!(
            parent_tree.get(b"Nums").unwrap(),
            &Object::Array(vec![
                Object::Integer(0),
                Object::Array(vec![divs[0].clone(), divs[1].clone(), divs[0].clone()]),
                Object::Integer(1),
                Object::Array(vec![divs[2].clone()]),
            ])
        );
        let struct_parents = |page: ObjectId| {
            doc.get_dictionary(page)
                .unwrap()
                .get(b"StructParents")
                .ok()
                .cloned()
        };
        assert_eq!(struct_parents(pages[0]), Some(Object::Integer(0)));
        assert_eq!(struct_parents(pages[1]), None, "not rewritten, not keyed");
        assert_eq!(struct_parents(pages[2]), Some(Object::Integer(1)));
        assert!(
            catalog.get(b"MarkInfo").is_err(),
            "no /MarkInfo is written (scope §3.4)"
        );

        // The stamp rides on the catalog beside the tree.
        stamp_tags(&mut doc, "sha256:aa", "sha256:bb", "0.0.0-test").expect("stamps");
        let stamp = doc
            .catalog()
            .unwrap()
            .get(b"EthosParserTags")
            .unwrap()
            .as_dict()
            .unwrap()
            .clone();
        let text = |key: &[u8]| lossy(stamp.get(key).unwrap().as_str().unwrap());
        assert_eq!(text(b"ArtifactType"), TAGS_ARTIFACT_TYPE);
        assert_eq!(text(b"SourceSha256"), "sha256:aa");
        assert_eq!(text(b"ProfileSha256"), "sha256:bb");
        assert_eq!(text(b"ParserVersion"), "0.0.0-test");
        assert_eq!(TAGS_ARTIFACT_TYPE, "ethos.parser.tags.v0");
    }

    #[test]
    fn a_plan_whose_ids_do_not_cover_its_sequences_is_refused_by_the_tree() {
        let (mut doc, pages) = blank_document(1);
        let seq = |mcid: i64| Sequence {
            block: BLOCK_A,
            first: 0,
            last: 0,
            mcid,
            placement: Placement::Operators,
            split: None,
        };
        let orphan = PagePlan {
            sequences: vec![seq(0), seq(1)],
            blocks: vec![(BLOCK_A, vec![0])],
        };
        let e = write_tree(&mut doc, &[(pages[0], &orphan)], "r").expect_err("id 1 is nobody's");
        assert!(e.to_string().contains("id 1 on page object"), "{e}");
        let beyond = PagePlan {
            sequences: vec![seq(0)],
            blocks: vec![(BLOCK_A, vec![0, 5])],
        };
        let e = write_tree(&mut doc, &[(pages[0], &beyond)], "r").expect_err("id 5 does not exist");
        assert!(e.to_string().contains("cites id 5"), "{e}");
    }
}
