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
//! # What is here and what is not yet
//!
//! The strict decoder, the tokeniser, and [`OpKind`], the per-operator classification the
//! placement rule of §3.4 reads. The placement rule, the tree, the stamp, the self-check and
//! `write_tags` are S2 items 4 to 8 and are not in this module yet; nothing here is public.

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

#[cfg(test)]
mod tests {
    use super::*;
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
    pub(super) fn assert_every_byte_placed(bytes: &[u8], t: &Tokenised) {
        let mut cursor = 0usize;
        for (i, op) in t.ops.iter().enumerate() {
            assert!(
                cursor <= op.start && op.start < op.end && op.end <= bytes.len(),
                "op {i} {op:?} is out of order or out of range (cursor {cursor}, len {})",
                bytes.len()
            );
            assert_gap_is_blank(bytes, cursor, op.start);
            let slice = &bytes[op.start..op.end];
            if op.operator == "BI" {
                assert!(slice.starts_with(b"BI"), "op {i}: {:?}", lossy(slice));
                assert!(slice.ends_with(b"EI"), "op {i}: {:?}", lossy(slice));
            } else {
                assert!(
                    slice.ends_with(op.operator.as_bytes()),
                    "op {i}: {:?} does not end with `{}`",
                    lossy(slice),
                    op.operator
                );
                assert!(
                    !b" \t\r\n%".contains(&slice[0]),
                    "op {i}: {:?} starts on whitespace or a comment",
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
            assert_every_byte_placed(bytes, t);
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
}
