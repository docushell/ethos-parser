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

//! The assist lane's record: a drafting model's replies, read into `Proposed` blocks (decision
//! #145).
//!
//! A model that reads page images can supply what the deterministic reader missed, and its text is
//! never evidence (contract §6, rule 3). So this module never renders and never asks: it reads a
//! **replies file** another process wrote, checks that the file answers this document, and attaches
//! the model's blocks to the record beside the engine's runs, under a profile that pins the model
//! ([`Profile::with_assist`]). The output is a function of the source and the replies file alone.
//!
//! # The replies file
//!
//! One JSON object, read with unknown fields refused:
//!
//! | Key | Holds |
//! | --- | --- |
//! | `ethos_assist_answers` | `1`, this shape's version |
//! | `source_sha256` | `sha256:<hex>` of the source bytes the pages were rendered from |
//! | `renderer` | [`ASSIST_RENDERER`]: the program and page rule that drew each page |
//! | `request` | [`ASSIST_REQUEST_V1`]: how each page was asked, and the reply format read here |
//! | `model` | `{"id", "version", "asked"}`: the id the server reported, the version the caller named or `not-reported`, and the id the request named (optional; `ethos-parser-assist` resumes a file under it, and the record does not keep it) |
//! | `pages` | One object per page asked, in ascending page order, each page at most once |
//!
//! Each page holds `page` (1-based), `points` (the rendered box's width and height in centipoints,
//! after rotation), `dpi`, `width` and `height` (the image's pixels), `pixels_sha256`, and either
//! `reply` with `finish_reason`, or `error`. Every number is an integer.
//!
//! # The reply format
//!
//! [`ASSIST_REQUEST_V1`] reads LightOnOCR's grounding replies: blocks, each a marker
//! `![label](x0,y0,x1,y1)` and the text up to the next marker, trimmed. The box is on a 0-1000 grid
//! of the image, its digits ASCII, clamped to it, an inverted side swapped; a label ending `+` goes
//! on from the page or column before. Text before the first marker is a block with no label and no
//! box. A reply with no marker is read as no blocks.
//!
//! # Budget
//!
//! A file over [`ASSIST_ANSWERS_MAX_BYTES`], or a reply of more than [`ASSIST_MAX_BLOCKS_PER_PAGE`]
//! markers, is refused [`ASSIST_ANSWERS_OVER_BUDGET`]: a reply is at most 12,288 tokens, and every
//! block costs a marker of several.
//!
//! # The guard
//!
//! Each block with a box is checked against the engine's **legible** text runs whose measured box
//! has its centre inside the block's box, the grid mapped onto the page's visible box and widened by
//! 10 grid units on every side ([`ASSIST_GUARD_V1`], decision #146). A run holding U+FFFD, or one
//! the engine found hidden, invisible, off the page or too small to see, is no evidence. The runs
//! are joined as `ground` joins them into elements, with no table claiming its cells, so a page read
//! glyph by glyph keeps its words; then read **line by line**: runs on one line, in a block, join
//! into a piece unless more than a quarter of a run's height parts them, on either side (half,
//! after a space the engine read: a page letter-spacing digits sets one between them,
//! `19 ,5 2 1`); lines never join, and no value spans two pieces of a line, so two cells are never
//! one value. Nor does one
//! span two lines (`12` above `34` is not `1234`), but where the first ends in `. , : / -`, as a
//! range or a link the page wraps does (`2007-` above `2023`); a table row's never does.
//!
//! **Words** are decision #123's: the distinct runs of letters, digits and `_`, two characters or
//! more, lower-cased, from the model's text with its tags removed and its HTML character references
//! decoded (`&lt;`, `&gt;`, `&amp;`, `&quot;`, `&#39;`, `&apos;`, decimal and hexadecimal), and from
//! the page's text as it is. A **value** is the prototype's token, with any script's decimal digit
//! for a digit: a run of ASCII letters and digits holding a digit, going on through letters, digits
//! and `. , : / -` to its last letter or digit (`1,234.5`, `A113`, `٢٠٢٤-١١`; `A-113` gives `113`),
//! a dash of any width between two letters or digits (U+2010 to U+2015, U+2212) reading as `-`
//! on either side (`99‐R98CR` is `99-R98CR`), and in the model's text a LaTeX command, a
//! backslash and the lower-case letters after it, no part of one (`\pm20V` gives `20V`); read
//! with what goes with it: a minus touching it or brackets around it make it negative (an en
//! dash too), and a leading currency symbol or a percent sign is kept, brackets, a leading currency
//! symbol and a percent sign also across a space or a piece break. A value is **found** where it
//! occurs in the runs' text with whitespace removed, starting and ending outside the runs' own values, with the same sign and, where both
//! show one, the same currency symbol and percent sign: `1 234` holds `1234`; `12345` does not, nor
//! `1.234` `1,234`, nor `(1,234)` `1,234`, nor `€980` `$980`. Digits are compared as written. A
//! value the engine ran into its neighbour with no space between (`1983` read as `1983DODG`) is not
//! found either: the rule is "exactly", kept until the ParseBench run measures what it costs.
//!
//! A block's **lines**, counted from 1, are a table's or chart's cells, row by row, and the text
//! outside its cells (a caption, words after the table) line by line; or else its text's lines.
//! Blank lines outside a cell are not lines. **A table row's values are found on its own lines of
//! the engine's, in the row's order, read left to right**: the line holding most of them in order,
//! then most of the row's words, and the lines between it and the neighbouring rows' lines; a first
//! or last row reaches one line past its own. A cell set on two lines whose second falls on a
//! neighbouring row's line binds there too, in its column, between the row's values found either
//! side, where the cell also shows on its own line (another of its values, or a word of letters).
//! Brackets a right-to-left line draws mirrored, `)x(`, read as `(x)` where they touch. A cell the
//! page sets on two lines binds, and a swap between rows each on its own line binds neither cell;
//! one between rows whose lines interleave may pass.
//! Two rows swapped whole, labels kept, are caught by their **labels**, a row's words of letters no
//! other row holds: where each row's label lies on the other's own line and not on its own, neither
//! row's values are found. A label alone on another row's line is no swap: a table centres a label
//! over the rows it spans, and sets a header over two lines.
//! A table cell's words bind on its row's lines; any other line's values and words anywhere in the
//! box. **A box the model drew too small binds against the engine's table it overlaps**, or a
//! leading-gap block region of the engine's, no larger than half the page, that it overlaps; never
//! against the page at large, so a value copied from elsewhere stays unfound. The region is
//! **read** where its legible runs hold a value or three words and share at least one word or value
//! with the block, so a reply that is mostly wrong is checked as closely as one that is mostly
//! right, while text sharing nothing with it (a font that maps its codes to other letters) is no
//! reading of it. **A
//! box over half the page places nothing**: no line binds by it alone, though a table row still
//! binds on its engine lines.
//!
//! 1. **A chart's** value not found is an `estimate`, kept and labelled: a chart's values are read
//!    from its bars, so most are not in the page's text. Its first row and the first cell of each
//!    row name its series and categories and assert nothing: never `unsupported`, they bind or are
//!    unchecked.
//! 2. **Where the engine did not read the region**, every other line is unchecked and kept:
//!    supplying it is the model's point.
//! 3. **Where it did**, a line with a value not found is `contradicted`, and a line of three words
//!    or more, fewer than half of them the engine's, `unsupported`; both are dropped. A table cell
//!    whose values off its row's lines the region holds elsewhere is `disputed` and kept instead (a
//!    total the engine reads below its table, a code or a date the page prints once); one holding a
//!    value the region does not hold, or in a row swapped whole, stays `contradicted`. In a block
//!    fewer than half of whose words the engine holds, such a line is `disputed` instead and kept:
//!    a reply that much at odds with the engine is as often the engine's misreading as the
//!    model's. A line binds
//!    where all its values are found, or, holding none, half its words; any other is unchecked.
//!    A `/ToUnicode` that maps codes to wrong but valid characters cannot be told from the
//!    record, so its text is read as the engine's.
//! 4. **A reply the server cut** (`finish_reason` `length`) ends inside its last block, which is
//!    dropped: the page is `loop` where that block ends in four or more copies of one string of 1
//!    to 400 characters, 64 or more in all, and `truncated` otherwise.
//!
//! What is dropped stays in the record, marked per line ([`AssistGuard::flagged`]) or by the page's
//! outcome, and counted; a projection leaves it out, and empties a dropped table cell rather than
//! removing it. **A reply that is unusable is said by its page's outcome alone, never as an
//! `assurance` limitation**: the assurance states what the engine read, which no reply changes, so
//! an assisted record's `ground` is the engine's.

use serde::{Deserialize, Serialize};

use crate::derivation::DerivationClass;
use crate::error::EngineError;
use crate::geom::QRect;
use crate::identity::Sha256Hex;
use crate::ids::{IdAllocator, IdKind, NodeId};
use crate::profile::{AssistPin, Profile};
use crate::representation::DocumentRepresentation;

/// The replies file's shape version, its `ethos_assist_answers`.
pub const ASSIST_ANSWERS_VERSION: u32 = 1;

/// The rule that reads a reply into blocks, and that later places them: the profile's
/// `assist.rule`.
pub const ASSIST_RULE_V1: &str = "assist-lightonocr-v1";

/// The guard that checks each block against the engine's runs inside its box (see the module
/// documentation, decision #146): the profile's `assist.guard`.
pub const ASSIST_GUARD_V1: &str = "assist-guard-v1";

/// How far the guard widens a block's box on every side, in units of the 0-1000 grid: a model's
/// box is drawn round the ink, and a run's box round its font's whole height.
const GUARD_SLACK: i128 = 10;

/// How many distinct words the engine's legible runs inside a box must hold, short of a value, for
/// the guard to take the region as read and check every line in it.
const GUARD_READ_WORDS: usize = 3;

/// The share of a block's words, in thousandths, the engine's runs must hold for a line the guard
/// fails to be dropped; below it, the line is disputed and kept.
const GUARD_AGREEMENT_PERMILLE: usize = 500;

/// The renderer whose pages this build reads replies to: `ethos-parser-assist`, hayro, and the page
/// rule `pages-400dpi-5mp-v1` — dpi `min(400, floor(sqrt(5e6 / square inches)))`, shrunk to 5M
/// pixels if still over. A replies file naming another is refused.
pub const ASSIST_RENDERER: &str = "ethos-parser-assist 0.1.0; hayro 0.7.1; pages-400dpi-5mp-v1";

/// The request this build reads replies to: one PNG page, the prompt `grounding`, temperature 0,
/// 12,288 tokens at most, answered in LightOnOCR's grounding format.
pub const ASSIST_REQUEST_V1: &str = "lightonocr-3-grounding-v1";

/// The most bytes a replies file may hold: 64 MiB, about 13,000 pages at decision #123's mean
/// reply and 1,400 at its longest.
pub const ASSIST_ANSWERS_MAX_BYTES: usize = 64 * 1024 * 1024;

/// The most marked blocks one reply may hold: 2,048, four times the most decision #123's 1,577
/// replies gave (519), and more markers than 12,288 tokens can spell. Text before the first marker
/// is one block more.
pub const ASSIST_MAX_BLOCKS_PER_PAGE: usize = 2048;

/// A drafting model's replies for one document, read into blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistRecord {
    /// Digest of the replies file's bytes.
    pub answers_sha256: Sha256Hex,
    /// The model that replied.
    pub model: AssistModel,
    /// Each page the model was asked about that this record lists, in page order. A page never
    /// asked is not here.
    pub pages: Vec<AssistPage>,
    /// The document's page numbers the replies answer that this record lists no page for — cut by
    /// a page budget (`--max-pages`), or not read — in ascending order. A block hangs on a listed
    /// page, so their replies are left out, and counted here. Absent where there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages_left_out: Vec<u32>,
}

/// A drafting model, as its replies file names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistModel {
    /// The id the server reported for it.
    pub id: String,
    /// The version the caller named, or `not-reported`.
    pub version: String,
}

/// One page's reply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistPage {
    /// The page record this reply is about.
    pub page: NodeId,
    /// The resolution the page was drawn at.
    pub dpi: u32,
    /// The image's width in pixels.
    pub width: u32,
    /// The image's height in pixels.
    pub height: u32,
    /// Digest of the image's pixels, for provenance only: nothing here renders.
    pub pixels_sha256: Sha256Hex,
    /// Why the model stopped, as the server said; absent where the page was not answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// What the reply gave.
    pub outcome: AssistOutcome,
    /// Every block of the reply, in its order.
    pub blocks: Vec<AssistBlock>,
}

/// What one page's reply gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssistOutcome {
    /// A reply with block markers, read into blocks.
    Read,
    /// A reply with block markers that the server cut at its token cap: its last block, cut
    /// mid-way, is dropped (decision #146).
    Truncated,
    /// A reply cut at the cap whose last block ends in a loop, four or more copies of one string of
    /// 1 to 400 characters, 64 or more in all: that block is dropped (decision #146).
    Loop,
    /// A reply with no block marker: not one this request's format reads.
    NoBlocks,
    /// No reply: the replies file holds the request's error.
    Error,
}

/// One block of a reply: a model's text, never evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistBlock {
    /// Its id, `m`-prefixed: a proposal, never a node.
    pub id: NodeId,
    /// Its label, lower-cased, a continuation's `+` removed. Absent on text before the first
    /// marker, which the model labelled nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether its label ended `+`: it goes on from the page or column before.
    pub continues: bool,
    /// Its box on the image's 0-1000 grid, `[x0, y0, x1, y1]`. Absent where the model gave none.
    /// The image shows the page as its rotation declares it, so on a page read turned (decision
    /// #155) the grid is turned from the record's frame by the page's `turn`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<[u32; 4]>,
    /// Its text, trimmed.
    pub text: String,
    /// Always [`DerivationClass::Proposed`]; the seal refuses anything else.
    pub derivation: DerivationClass,
    /// What the guard found, where the block has a box to bind it by (decision #146).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard: Option<AssistGuard>,
}

/// What the engine's runs inside one block's box say of its lines (decision #146; the module
/// documentation has the rules). A line neither bound nor flagged is unchecked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistGuard {
    /// How many distinct words the runs inside the box hold.
    pub engine_words: u32,
    /// The share of the block's distinct words among them, in thousandths, rounded down; 0 for a
    /// block of none. Reported only: whether a line is checked depends on the engine's side alone.
    pub agreement_permille: u32,
    /// How many lines the block has: a table's or chart's cells and the text outside them, or its
    /// text's lines; blank lines outside a cell are not counted.
    pub lines: u32,
    /// How many of them the runs bear out.
    pub bound: u32,
    /// The lines marked, in line order, each once.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flagged: Vec<AssistFlag>,
}

/// One line the guard marked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistFlag {
    /// The line, counted from 1.
    pub line: u32,
    /// What the guard made of it.
    pub mark: AssistMark,
    /// The line's first value not found; absent on an `unsupported` line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// What the guard made of a line it marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssistMark {
    /// A value the runs do not hold as written, sign and symbols included, in a region the engine
    /// read, or a table row's value in a row swapped whole with another: dropped.
    Contradicted,
    /// Three words or more, fewer than half of them the runs', never a chart's names: dropped.
    Unsupported,
    /// A chart's value the runs do not hold: kept, labelled an estimate.
    Estimate,
    /// A line that would be contradicted or unsupported, in a block fewer than half of whose words
    /// the runs hold: kept, labelled disputed, since a reply mostly at odds with the engine's
    /// reading is as often the engine's misreading as the model's (decision #146); or a table cell
    /// whose values the region holds, one not on its row's lines. It names its first value not
    /// found, if a value failed.
    Disputed,
}

/// The replies file, as read (see the module documentation).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answers {
    ethos_assist_answers: u32,
    source_sha256: Sha256Hex,
    renderer: String,
    request: String,
    model: AnswersModel,
    pages: Vec<AnswersPage>,
}

/// The replies file's `model`: the record's [`AssistModel`], and the id the request named, which
/// `ethos-parser-assist` resumes the file under and the record does not keep (review).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnswersModel {
    id: String,
    version: String,
    #[serde(default, rename = "asked")]
    _asked: Option<String>,
}

/// One page of the replies file.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnswersPage {
    page: u32,
    points: [i64; 2],
    dpi: u32,
    width: u32,
    height: u32,
    pixels_sha256: Sha256Hex,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    reply: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// Not JSON, an unknown field, another shape version, pages out of order or repeated, or a page
/// with neither a reply nor an error, or both.
pub const ASSIST_ANSWERS_UNREADABLE: &str = "assist-answers-unreadable";
/// The replies file answers other source bytes.
pub const ASSIST_ANSWERS_OTHER_SOURCE: &str = "assist-answers-other-source";
/// The replies file's pages were drawn by a renderer this build does not pin.
pub const ASSIST_ANSWERS_OTHER_RENDERER: &str = "assist-answers-other-renderer";
/// The replies file's pages were asked with a request this build does not read.
pub const ASSIST_ANSWERS_OTHER_REQUEST: &str = "assist-answers-other-request";
/// A page the replies file answers is not in the record, or not of the size it was rendered at.
pub const ASSIST_ANSWERS_OTHER_PAGE: &str = "assist-answers-other-page";
/// The replies file's model is not the one the caller named.
pub const ASSIST_ANSWERS_OTHER_MODEL: &str = "assist-answers-other-model";
/// The replies file's model id is a file path — it starts `/`, `~` or `.`, or a drive letter, or
/// holds a backslash — as a server started with a local `--model` reports it. Every record would
/// carry the path, and its profile hash would differ from machine to machine.
pub const ASSIST_ANSWERS_MODEL_IS_PATH: &str = "assist-answers-model-is-path";
/// The replies file's model id names this engine's own processor or recognizer, compared trimmed
/// and ASCII case-folded: a model that proposes text is never the processor that read the evidence
/// (`docs/04-ARCHITECTURE.md` §6).
pub const ASSIST_ANSWERS_MODEL_IS_PROCESSOR: &str = "assist-answers-model-is-processor";
/// The replies file is over [`ASSIST_ANSWERS_MAX_BYTES`], or a reply over
/// [`ASSIST_MAX_BLOCKS_PER_PAGE`] markers.
pub const ASSIST_ANSWERS_OVER_BUDGET: &str = "assist-answers-over-budget";
/// The record already holds a model's blocks, or was not read under the profile given: a proposal
/// never replaces another (contract §6, rule 2).
pub const ASSIST_BASE: &str = "assist-base";

fn refused(what: &str, detail: String) -> EngineError {
    EngineError::Malformed {
        what: what.into(),
        detail,
    }
}

/// Attach a replies file's blocks to an engine-only record read under `base` (decision #145).
///
/// `answers` is the file's bytes and `model`, where given, the `ID[@VERSION]` the caller expects
/// the file to name; a version left out is not compared. The result is sealed under
/// `base.with_assist(pin)`, its `profile_sha256` that profile's, and differs from `repr` only in
/// that hash, its `assist` key and its fingerprint.
///
/// # Errors
///
/// [`EngineError::Malformed`] named for what does not match: [`ASSIST_ANSWERS_UNREADABLE`],
/// [`ASSIST_ANSWERS_OTHER_SOURCE`], [`ASSIST_ANSWERS_OTHER_RENDERER`],
/// [`ASSIST_ANSWERS_OTHER_REQUEST`], [`ASSIST_ANSWERS_OTHER_MODEL`] or
/// [`ASSIST_ANSWERS_OTHER_PAGE`] (a page past the document's last; a page the document has and
/// the record does not list is left out, its number kept in `pages_left_out`),
/// [`ASSIST_ANSWERS_MODEL_IS_PATH`], [`ASSIST_ANSWERS_MODEL_IS_PROCESSOR`] or
/// [`ASSIST_ANSWERS_OVER_BUDGET`]; and [`ASSIST_BASE`] where `repr` already holds a model's blocks
/// or was not read under `base`, since a proposal never replaces another (contract §6, rule 2).
pub fn attach(
    repr: DocumentRepresentation,
    base: &Profile,
    answers: &[u8],
    model: Option<&str>,
) -> Result<DocumentRepresentation, EngineError> {
    let hash = |p: &Profile| {
        p.profile_sha256()
            .map_err(|e| refused("profile", e.to_string()))
    };
    let p = repr.payload();
    if p.assist.is_some() || base.assist.is_some() || p.identity.profile_sha256 != hash(base)? {
        return Err(refused(
            ASSIST_BASE,
            "a model's blocks attach once, to a record read under the profile given; a proposal \
             never replaces another"
                .into(),
        ));
    }

    if answers.len() > ASSIST_ANSWERS_MAX_BYTES {
        return Err(refused(
            ASSIST_ANSWERS_OVER_BUDGET,
            format!(
                "the replies file holds {} bytes; this build reads {ASSIST_ANSWERS_MAX_BYTES} at \
                 most",
                answers.len()
            ),
        ));
    }
    let file: Answers = serde_json::from_slice(answers)
        .map_err(|e| refused(ASSIST_ANSWERS_UNREADABLE, e.to_string()))?;
    if file.ethos_assist_answers != ASSIST_ANSWERS_VERSION {
        return Err(refused(
            ASSIST_ANSWERS_UNREADABLE,
            format!(
                "`ethos_assist_answers` is {}; this build reads {ASSIST_ANSWERS_VERSION}",
                file.ethos_assist_answers
            ),
        ));
    }
    if file.pages.windows(2).any(|w| w[0].page >= w[1].page) {
        return Err(refused(
            ASSIST_ANSWERS_UNREADABLE,
            "pages are in ascending order, each at most once".into(),
        ));
    }
    if let Some(page) = file.pages.iter().find(|a| {
        !matches!(
            (
                a.reply.is_some(),
                a.finish_reason.is_some(),
                a.error.is_some()
            ),
            (true, true, false) | (false, false, true)
        )
    }) {
        return Err(refused(
            ASSIST_ANSWERS_UNREADABLE,
            format!(
                "page {} holds a `reply` with its `finish_reason`, or an `error`, never both",
                page.page
            ),
        ));
    }
    if file.source_sha256 != p.source.sha256 {
        return Err(refused(
            ASSIST_ANSWERS_OTHER_SOURCE,
            format!(
                "the replies answer source {}, and this document is {}",
                file.source_sha256, p.source.sha256
            ),
        ));
    }
    if file.renderer != ASSIST_RENDERER {
        return Err(refused(
            ASSIST_ANSWERS_OTHER_RENDERER,
            format!(
                "the pages were drawn by `{}`; this build reads `{ASSIST_RENDERER}`",
                file.renderer
            ),
        ));
    }
    if file.request != ASSIST_REQUEST_V1 {
        return Err(refused(
            ASSIST_ANSWERS_OTHER_REQUEST,
            format!(
                "the pages were asked as `{}`; this build reads `{ASSIST_REQUEST_V1}`",
                file.request
            ),
        ));
    }
    if let Some((what, why)) = model_id_refusal(&file.model.id, &p.processing_run.processor.name) {
        return Err(refused(what, why));
    }
    if let Some(named) = model {
        let (id, version) = named
            .rsplit_once('@')
            .map_or((named, None), |(id, v)| (id, Some(v)));
        if id != file.model.id || version.is_some_and(|v| v != file.model.version) {
            return Err(refused(
                ASSIST_ANSWERS_OTHER_MODEL,
                format!(
                    "the replies are `{}@{}`'s, and `{named}` was named",
                    file.model.id, file.model.version
                ),
            ));
        }
    }

    let profile = base.clone().with_assist(AssistPin {
        rule: ASSIST_RULE_V1.into(),
        guard: ASSIST_GUARD_V1.into(),
        renderer: file.renderer,
        request: file.request,
        model: file.model.id.clone(),
        model_version: file.model.version.clone(),
    });
    let profile_sha256 = hash(&profile)?;
    let mut ids = IdAllocator::new(profile_sha256.clone());
    // The engine's ink by page, what the guard binds blocks to: its runs, grouped as `ground`
    // groups them into elements, but with no table claiming its cells, so a table's row is ink
    // along a line too. A run keeps its measured box; a run of spaces has none.
    let mut ink: std::collections::BTreeMap<&str, Vec<Ink>> = Default::default();
    for block in crate::markdown::geometric_blocks(&p.nodes, &Default::default()) {
        let members: Ink = block
            .members
            .iter()
            .map(|&i| {
                let node = &p.nodes[i];
                InkRun {
                    rect: repr.geometry_at(i).and_then(|g| g.measured()),
                    text: node.reading_text(),
                    legible: legible(node),
                }
            })
            .collect();
        if let Some(&first) = block.members.first() {
            ink.entry(p.nodes[first].parent.as_str())
                .or_default()
                .push(members);
        }
    }
    // The engine's regions by page, a box binds against where it overlaps one: each measured
    // table, and each leading-gap block's runs' union.
    let mut regions: std::collections::BTreeMap<&str, Vec<(QRect, bool)>> = Default::default();
    for table in &p.tables {
        if let Some(rect) = table.geometry.measured() {
            regions
                .entry(table.page.as_str())
                .or_default()
                .push((rect, true));
        }
    }
    let mut blocks: std::collections::BTreeMap<(&str, u32), [i64; 4]> = Default::default();
    for (node, geometry) in p.nodes.iter().zip(repr.geometry()) {
        let crate::representation::NodeAttributes::TextRun(run) = &node.attributes else {
            continue;
        };
        if let (Some(block), Some(r)) = (run.block, geometry.presence.measured()) {
            let edges = blocks.entry((node.parent.as_str(), block)).or_insert([
                r.x0(),
                r.y0(),
                r.x1(),
                r.y1(),
            ]);
            *edges = [
                edges[0].min(r.x0()),
                edges[1].min(r.y0()),
                edges[2].max(r.x1()),
                edges[3].max(r.y1()),
            ];
        }
    }
    for ((page, _), [x0, y0, x1, y1]) in blocks {
        if let Ok(rect) = QRect::new(x0, y0, x1, y1) {
            regions.entry(page).or_default().push((rect, false));
        }
    }
    let mut pages = Vec::with_capacity(file.pages.len());
    let mut pages_left_out = Vec::new();
    for answer in file.pages {
        let other_page = |why: String| {
            refused(
                ASSIST_ANSWERS_OTHER_PAGE,
                format!("the replies answer page {}, {why}", answer.page),
            )
        };
        let Some(page) = p.pages.iter().find(|r| r.index == answer.page) else {
            // A page the document has and the record does not list — cut by a page budget, or
            // not read — has no page record for a block to hang on: left out, and counted.
            if (1..=p.assurance.coverage.pages_authorized).contains(&answer.page) {
                pages_left_out.push(answer.page);
                continue;
            }
            return Err(other_page("which this document does not have".into()));
        };
        let frame @ [_, _, width, height] = frame_of(page);
        // The renderer draws the page as its rotation declares it, whatever turn it was read in.
        let (width, height) = if page.turn % 180 == 0 {
            (width, height)
        } else {
            (height, width)
        };
        let off = |rendered: i64, shown: i64| {
            rendered.abs_diff(shown) > u64::from(crate::geom::QUANTUM_PER_POINT)
        };
        if off(answer.points[0], width) || off(answer.points[1], height) {
            return Err(other_page(format!(
                "rendered {} x {} centipoints where this record shows {width} x {height}",
                answer.points[0], answer.points[1]
            )));
        }
        let (outcome, read) = match &answer.reply {
            Some(reply) => {
                let read = blocks_of(reply).ok_or_else(|| {
                    refused(
                        ASSIST_ANSWERS_OVER_BUDGET,
                        format!(
                            "the reply to page {} holds more than {ASSIST_MAX_BLOCKS_PER_PAGE} \
                             block markers",
                            answer.page
                        ),
                    )
                })?;
                // A reply cut at the cap ends inside its last block (guard rule 4).
                let outcome = if !read.iter().any(|b| b.label.is_some()) {
                    AssistOutcome::NoBlocks
                } else if answer.finish_reason.as_deref() != Some("length") {
                    AssistOutcome::Read
                } else if read.last().is_some_and(|b| loops(&b.text)) {
                    AssistOutcome::Loop
                } else {
                    AssistOutcome::Truncated
                };
                (outcome, read)
            }
            None => (AssistOutcome::Error, Vec::new()),
        };
        let on_page = ink.get(page.id.as_str()).map_or(&[][..], Vec::as_slice);
        let in_regions = regions.get(page.id.as_str()).map_or(&[][..], Vec::as_slice);
        let mut blocks = Vec::with_capacity(read.len());
        for b in read {
            let guard = b.grid.map(|grid| {
                guard_with(
                    b.label.as_deref(),
                    &b.text,
                    upright(grid, page.turn),
                    frame,
                    on_page,
                    in_regions,
                )
            });
            blocks.push(AssistBlock {
                id: ids.next(IdKind::Proposal)?,
                label: b.label,
                continues: b.continues,
                grid: b.grid,
                text: b.text,
                derivation: DerivationClass::Proposed,
                guard,
            });
        }
        pages.push(AssistPage {
            page: page.id.clone(),
            dpi: answer.dpi,
            width: answer.width,
            height: answer.height,
            pixels_sha256: answer.pixels_sha256,
            finish_reason: answer.finish_reason,
            outcome,
            blocks,
        });
    }

    let (mut payload, geometry) = repr.into_parts();
    payload.identity.profile_sha256 = profile_sha256;
    payload.assist = Some(AssistRecord {
        answers_sha256: Sha256Hex::of_bytes(answers),
        model: AssistModel {
            id: file.model.id,
            version: file.model.version,
        },
        pages,
        pages_left_out,
    });
    DocumentRepresentation::seal(payload, geometry)
}

/// The refusal a model id meets, if any: one rule for [`attach`] and the seal.
///
/// Empty or blank is [`ASSIST_ANSWERS_UNREADABLE`]. A path is [`ASSIST_ANSWERS_MODEL_IS_PATH`]: it
/// starts `/`, `~` or `.`, or a drive letter and `:`, or holds a backslash, all after trimming; a
/// hub id such as `org/name` is not one. This engine's processor or recognizer, compared trimmed
/// and ASCII case-folded, is [`ASSIST_ANSWERS_MODEL_IS_PROCESSOR`].
pub(crate) fn model_id_refusal(id: &str, processor: &str) -> Option<(&'static str, String)> {
    let id = id.trim();
    if id.is_empty() {
        return Some((ASSIST_ANSWERS_UNREADABLE, "the model's id is empty".into()));
    }
    let drive = matches!(id.as_bytes(), [letter, b':', ..] if letter.is_ascii_alphabetic());
    if id.starts_with(['/', '~', '.']) || drive || id.contains('\\') {
        return Some((
            ASSIST_ANSWERS_MODEL_IS_PATH,
            format!(
                "the model's id `{id}` is a file path, and every record would carry it: serve the \
                 model by a name, not a path"
            ),
        ));
    }
    let recognizer = crate::profile::RecognizerIdentity::ethos_parser_ocr().recognizer;
    if [processor, recognizer.as_str()]
        .iter()
        .any(|engine| engine.trim().eq_ignore_ascii_case(id))
    {
        return Some((
            ASSIST_ANSWERS_MODEL_IS_PROCESSOR,
            format!(
                "the drafting model `{id}` shares its identity with this engine's processor or \
                 recognizer; a model that proposes text is never the processor that read the \
                 evidence (North Star row 12)"
            ),
        ));
    }
    None
}

/// A block as read, before it has an id.
#[derive(Debug, PartialEq, Eq)]
struct Read {
    label: Option<String>,
    continues: bool,
    grid: Option<[u32; 4]>,
    text: String,
}

/// A reply's blocks, in order (see the module documentation), or `None` past
/// [`ASSIST_MAX_BLOCKS_PER_PAGE`] markers, counted as they are found.
fn blocks_of(reply: &str) -> Option<Vec<Read>> {
    let mut markers = Vec::new();
    let mut at = 0;
    while let Some(found) = reply[at..].find("![") {
        let start = at + found;
        match marker(&reply[start..]) {
            Some((len, label, grid)) => {
                markers.push((start, start + len, label, grid));
                if markers.len() > ASSIST_MAX_BLOCKS_PER_PAGE {
                    return None;
                }
                at = start + len;
            }
            None => at = start + 2,
        }
    }
    let mut out = Vec::with_capacity(markers.len() + 1);
    let first = markers.first().map_or(reply.len(), |m| m.0);
    let before = reply[..first].trim();
    if !before.is_empty() {
        out.push(Read {
            label: None,
            continues: false,
            grid: None,
            text: before.into(),
        });
    }
    for (i, (_, end, label, grid)) in markers.iter().enumerate() {
        let next = markers.get(i + 1).map_or(reply.len(), |m| m.0);
        out.push(Read {
            label: Some(label.trim_end_matches('+').to_lowercase()),
            continues: label.ends_with('+'),
            grid: Some(*grid),
            text: reply[*end..next].trim().into(),
        });
    }
    Some(out)
}

/// The marker `![label](x0,y0,x1,y1)` at the start of `s`: its length, its raw label and its box,
/// clamped and ordered. ParseBench's
/// `!\[([a-zA-Z_+]+)\]\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)[ \t]*`, digits ASCII; its
/// trailing blanks are left to the trim of the text after it.
fn marker(s: &str) -> Option<(usize, &str, [u32; 4])> {
    let rest = s.strip_prefix("![")?;
    let label_len = rest
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '_' || c == '+'))
        .unwrap_or(rest.len());
    if label_len == 0 {
        return None;
    }
    let (label, rest) = rest.split_at(label_len);
    let mut rest = rest.strip_prefix("](")?;
    let mut n = [0u32; 4];
    for (i, value) in n.iter_mut().enumerate() {
        rest = rest.trim_start();
        let digits = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if digits == 0 {
            return None;
        }
        // Clamped to the grid as read, so no length of digits overflows.
        *value = rest[..digits]
            .bytes()
            .fold(0u32, |v, d| (v * 10 + u32::from(d - b'0')).min(1000));
        rest = rest[digits..].trim_start();
        rest = rest.strip_prefix(if i < 3 { "," } else { ")" })?;
    }
    let [x0, y0, x1, y1] = n;
    Some((
        s.len() - rest.len(),
        label,
        [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)],
    ))
}

/// Whether the engine's `node` is evidence for or against a model's reading: not a run holding
/// U+FFFD, nor one the engine found hidden, invisible, off the page or too small to see (any
/// `TextFinding`), which a model reading the page's picture cannot have seen.
fn legible(node: &crate::Node) -> bool {
    let found = matches!(
        &node.attributes,
        crate::representation::NodeAttributes::TextRun(run) if !run.findings.is_empty()
    );
    !found && !node.text.contains('\u{FFFD}')
}

/// A page's visible box, `[left, top, width, height]` in centipoints: the frame a block's grid is
/// drawn on. The renderer draws exactly that box, turned, and the record's sizes are after the
/// turn, so no rotation enters (design §4.3).
pub(crate) fn frame_of(page: &crate::representation::PageRecord) -> [i64; 4] {
    match page.visible {
        Some(v) => [v.x0(), v.y0(), v.x1() - v.x0(), v.y1() - v.y0()],
        None => [0, 0, page.width, page.height],
    }
}

/// A block's box on the grid of the picture the renderer drew, which shows the page as its
/// rotation declares it, on the grid of the page as the record reads it, `turn` (its
/// [`PageRecord::turn`](crate::representation::PageRecord::turn)) further round (decision #155).
pub(crate) fn upright(grid: [u32; 4], turn: i64) -> [u32; 4] {
    let [x0, y0, x1, y1] = grid;
    let m = |g: u32| 1000 - g.min(1000);
    match turn {
        90 => [m(y1), x0, m(y0), x1],
        180 => [m(x1), m(y1), m(x0), m(y0)],
        270 => [y0, m(x1), y1, m(x0)],
        _ => grid,
    }
}

/// A block's box on the 0-1000 grid, mapped onto `frame` (as [`frame_of`] gives it), as
/// `[x0, y0, x1, y1]` in thousandths of a centipoint, so no division rounds.
pub(crate) fn placed(grid: [u32; 4], frame: [i64; 4]) -> [i128; 4] {
    let [left, top, width, height] = frame.map(i128::from);
    let at = |origin: i128, size: i128, g: u32| 1000 * origin + i128::from(g) * size;
    [
        at(left, width, grid[0]),
        at(top, height, grid[1]),
        at(left, width, grid[2]),
        at(top, height, grid[3]),
    ]
}

/// One run of the engine's ink: its measured box, if it has one, its text, and whether it is
/// [`legible`].
#[derive(Debug, Clone)]
struct InkRun<'a> {
    rect: Option<QRect>,
    text: &'a str,
    legible: bool,
}

/// One block of the engine's ink, its runs in order, as `ground` groups them.
type Ink<'a> = Vec<InkRun<'a>>;

/// What the engine's runs inside a block's box say of its lines (see the module documentation).
/// `frame` is the page's visible box, `[left, top, width, height]` in centipoints, and `ink` the
/// page's runs, grouped as the engine groups them.
fn guard_with(
    label: Option<&str>,
    text: &str,
    grid: [u32; 4],
    frame: [i64; 4],
    ink: &[Ink],
    regions: &[(QRect, bool)],
) -> AssistGuard {
    // A box the model drew too small binds against the engine's table it overlaps, or a block region
    // of the engine's no larger than half the page: never against the page at large.
    let [_, _, width, height] = frame.map(i128::from);
    let model = placed(grid, frame);
    let union: Vec<QRect> = regions
        .iter()
        .filter(|(r, table)| {
            let [x0, y0, x1, y1] = [r.x0(), r.y0(), r.x1(), r.y1()].map(|v| 1000 * i128::from(v));
            let overlaps = x0 < model[2] && x1 > model[0] && y0 < model[3] && y1 > model[1];
            let area = (x1 - x0) / 1000 * ((y1 - y0) / 1000);
            overlaps && (*table || 2 * area <= width * height)
        })
        .map(|(r, _)| *r)
        .collect();
    let bands = bands_inside(grid, frame, ink, &union);
    // Lines apart as pieces are, so `12` above `34` is not `1234` (review), but a line ending in a
    // separator a value runs through goes on: `2007-` above `2023`, a link broken after a `/`.
    let mut all = String::new();
    for line in &bands {
        if !all.is_empty() {
            let on = all.trim_end().ends_with(['.', ',', ':', '/', '-']);
            all.push_str(if on { "\n" } else { PIECE });
        }
        all.push_str(&joined(line));
    }
    let all = Engine::new(&all);

    // A box over half the page places nothing: no line binds by it alone.
    let [x0, y0, x1, y1] = grid;
    let placed = (x1 - x0) * (y1 - y0) <= 500_000;

    let whole = hyphenated(&plain(text));
    let own = words(&whole);
    let shared = own.intersection(&all.words).count();
    let agreement = (shared * 1000).checked_div(own.len()).unwrap_or(0);
    // The engine read the region where its legible runs there hold a value or a few words, and
    // share a word or a value with the block: then every line is checked, however much of it the
    // model got wrong. Text sharing nothing — a font mapping its codes to other letters — is none.
    let read = (all.has_values || all.words.len() >= GUARD_READ_WORDS)
        && (shared > 0 || values(&whole).iter().any(|v| all.find(v, 0).is_some()));
    // A block fewer than half of whose words the engine holds is at odds with its reading: what it
    // fails is disputed, kept, not dropped, as often the engine's misreading as the model's.
    let disputed = agreement < GUARD_AGREEMENT_PERMILLE;
    let chart = label == Some("chart");
    let lines: Vec<(Option<u32>, String)> = block_lines(label, text)
        .into_iter()
        .map(|(row, line)| (row, hyphenated(&line)))
        .collect();
    let (rows, row_words, swapped) = if label == Some("table") {
        rows_found(&lines, &bands)
    } else {
        Default::default()
    };
    // A chart's first row and the first cell of each row name its series and categories.
    let first_row = lines.iter().find_map(|(row, _)| *row);
    let mut rows_seen = std::collections::BTreeSet::new();
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let mut guard = AssistGuard {
        engine_words: count(all.words.len()),
        agreement_permille: count(agreement),
        lines: count(lines.len()),
        bound: 0,
        flagged: Vec::new(),
    };
    for (line, (index, (row, text))) in (1u32..).zip(lines.iter().enumerate()) {
        let values = values(text);
        // A table row's values are found on one line of the engine's, in order (`rows_found`);
        // any other line's anywhere in the box.
        let found = |k: usize, v: &Value| match row {
            Some(_) if label == Some("table") => rows.contains(&(index, k)),
            _ => placed && all.find(v, 0).is_some(),
        };
        let unfound = |k: usize, v: &Value| match row {
            Some(_) if label == Some("table") => !rows.contains(&(index, k)),
            _ => all.find(v, 0).is_none(),
        };
        let missing = (0..).zip(&values).find(|(k, v)| unfound(*k, v));
        // A table cell whose values off its row's lines the region holds elsewhere is disputed,
        // kept: a total the engine reads below its table, a date the page prints once. A value
        // the region does not hold, or a row swapped whole with another, stays contradicted.
        let misplaced = matches!(row, Some(r) if label == Some("table") && !swapped.contains(r))
            && (0..)
                .zip(&values)
                .all(|(k, v)| !unfound(k, v) || all.find(v, 0).is_some());
        // A chart's names assert nothing, so they are never unsupported: bound, or unchecked.
        let naming = chart && row.is_some_and(|r| Some(r) == first_row || rows_seen.insert(r));
        let own = words(text);
        let shared = own.intersection(&all.words).count();
        let (mark, value) = match missing {
            Some((_, v)) if chart => (AssistMark::Estimate, Some(v.token)),
            _ if !read => continue, // The engine did not read the region: unchecked.
            Some((_, v)) if disputed || misplaced => (AssistMark::Disputed, Some(v.token)),
            Some((_, v)) => (AssistMark::Contradicted, Some(v.token)),
            None if !naming && own.len() >= 3 && 2 * shared < own.len() => {
                let mark = if disputed {
                    AssistMark::Disputed
                } else {
                    AssistMark::Unsupported
                };
                (mark, None)
            }
            None => {
                // A table cell's words bind on its row's line; any other line's anywhere in a
                // placed box.
                let near = match row {
                    Some(r) if label == Some("table") => row_words.get(r),
                    _ => placed.then_some(&all.words),
                };
                let bound = if values.is_empty() {
                    near.is_some_and(|near| {
                        !own.is_empty() && 2 * own.intersection(near).count() >= own.len()
                    })
                } else {
                    (0..).zip(&values).all(|(k, v)| found(k, v))
                };
                guard.bound += u32::from(bound);
                continue;
            }
        };
        guard.flagged.push(AssistFlag {
            line,
            mark,
            value: value.map(Into::into),
        });
    }
    guard
}

/// What stands between two pieces of one line: no value spans it, and it is no word. Two of the
/// page's cells are never one value of the model's, while a split `1 234` inside one piece is.
const PIECE: &str = "\u{1F}";

/// [`PIECE`] as a character.
const PIECE_MARK: char = '\u{1F}';

/// The engine's legible text inside a block's box, and inside the engine regions `union` it binds
/// against, line by line from the top: on each line, its pieces from the left, each with its left
/// edge, a piece being runs the engine joined on that line. The box
/// is the grid's on the visible box, widened by `GUARD_SLACK` units, and a run is inside where its
/// box's centre is; compared in `i128`, so no page is too large.
fn bands_inside(grid: [u32; 4], frame: [i64; 4], ink: &[Ink], union: &[QRect]) -> Vec<Line> {
    let [left, top, width, height] = frame.map(i128::from);
    // A centre, (x0 + x1) / 2, against `left + (g -/+ slack) * width / 1000`: both sides times 2,000.
    let edges = |origin: i128, size: i128, from: u32, to: u32| {
        let at = |g: i128| 2 * (1000 * origin + g * size);
        at(i128::from(from) - GUARD_SLACK)..=at(i128::from(to) + GUARD_SLACK)
    };
    let (xs, ys) = (
        edges(left, width, grid[0], grid[2]),
        edges(top, height, grid[1], grid[3]),
    );
    let centre = |a: i64, b: i64| 1000 * (i128::from(a) + i128::from(b));
    let inside = |r: &QRect| {
        let (x, y) = (centre(r.x0(), r.x1()), centre(r.y0(), r.y1()));
        (xs.contains(&x) && ys.contains(&y))
            || union.iter().any(|u| {
                let edges = |a: i64, b: i64| 2000 * i128::from(a)..=2000 * i128::from(b);
                edges(u.x0(), u.x1()).contains(&x) && edges(u.y0(), u.y1()).contains(&y)
            })
    };
    // Pieces, as [top, bottom, left, right] and text. A run joins the piece before it when it is of
    // the same block, inside, legible, its centre lies within the piece's height, and it lies no
    // more than a quarter of its own height beyond the piece on either side: glyphs of a word touch,
    // whichever way they run, while two cells the engine read as one block stand apart, and a run
    // the reading order jumps back left to starts afresh. After a space the engine read, less than
    // half its height: a page that letter-spaces digits sets a space between them (`19 ,5 2 1`).
    // A run of spaces, which has no box, joins whatever piece is open.
    let near = |piece: &str, gap: i64, height: i64| {
        if piece.ends_with(char::is_whitespace) {
            2 * gap < height
        } else {
            4 * gap <= height
        }
    };
    let mut pieces: Vec<([i64; 4], String)> = Vec::new();
    for runs in ink {
        let mut open = false;
        for run in runs {
            match run.rect {
                Some(r) if run.legible && inside(&r) => {
                    let mid = r.y0() + r.y1();
                    match pieces.last_mut() {
                        Some(([t, b, left, right], piece))
                            if open
                                && 2 * *t <= mid
                                && mid <= 2 * *b
                                && near(piece, r.x0() - *right, r.y1() - r.y0())
                                && near(piece, *left - r.x1(), r.y1() - r.y0()) =>
                        {
                            piece.push_str(run.text);
                            *left = (*left).min(r.x0());
                            *right = (*right).max(r.x1());
                        }
                        _ => pieces.push(([r.y0(), r.y1(), r.x0(), r.x1()], run.text.to_string())),
                    }
                    open = true;
                }
                None if run.legible && open => {
                    if let Some((_, piece)) = pieces.last_mut() {
                        piece.push_str(run.text);
                    }
                }
                _ => open = false,
            }
        }
    }
    // Lines: pieces by their middle, each joining the line above where its middle lies within the
    // line's height so far.
    pieces.sort_by_key(|([t, b, l, _], _)| (t + b, *l));
    // Each line: its [top, bottom] so far, and its pieces with their left edges.
    let mut lines: Vec<([i64; 2], Line)> = Vec::new();
    for ([t, b, l, _], text) in pieces {
        match lines.last_mut() {
            Some(([top, bottom], members)) if 2 * *top <= t + b && t + b <= 2 * *bottom => {
                *top = (*top).min(t);
                *bottom = (*bottom).max(b);
                members.push((l, text));
            }
            _ => lines.push(([t, b], vec![(l, text)])),
        }
    }
    lines
        .into_iter()
        .map(|(_, mut members)| {
            members.sort_by_key(|(l, _)| *l);
            members
        })
        .collect()
}

/// One line of the engine's text inside a box: its pieces, each with its left edge, left to right.
type Line = Vec<(i64, String)>;

/// A line's pieces, [`PIECE`] apart.
fn joined(line: &[(i64, String)]) -> String {
    let texts: Vec<&str> = line.iter().map(|(_, t)| t.as_str()).collect();
    texts.join(PIECE)
}

/// What [`rows_found`] gives: the `(line, value)` pairs found, the words of each row's lines, and
/// the rows swapped whole with another.
type RowsFound = (
    std::collections::BTreeSet<(usize, usize)>,
    std::collections::BTreeMap<u32, std::collections::BTreeSet<String>>,
    std::collections::BTreeSet<u32>,
);

/// Where a table's rows lie among the engine's lines, and which of their values are found there.
/// Each row's own line is the one holding most of its values in the row's order, then most of its
/// words; its lines run from there to the lines of the rows around it, not including them, and a
/// first or last row reaches one line past its own. A row's values are found in its lines in the
/// row's order, read left to right across them: so a cell the page sets on two lines binds, and a
/// value found only on another row's lines does not. The pairs are `(line, value)` by index into
/// `lines` and into the line's values; a row sharing nothing with any line has no lines.
fn rows_found(lines: &[(Option<u32>, String)], bands: &[Line]) -> RowsFound {
    type Row<'a> = (
        Vec<(usize, usize, Value<'a>)>,
        std::collections::BTreeSet<String>,
    );
    let mut rows: std::collections::BTreeMap<u32, Row> = Default::default();
    for (index, (row, text)) in lines.iter().enumerate() {
        if let Some(row) = row {
            let (held, own) = rows.entry(*row).or_default();
            held.extend(
                values(text)
                    .into_iter()
                    .enumerate()
                    .map(|(k, v)| (index, k, v)),
            );
            own.extend(words(text));
        }
    }
    // The pairs a text holds of a row's values, in order.
    let hits = |engine: &Engine, row: &[(usize, usize, Value)]| {
        let mut from = 0;
        let mut hit = Vec::new();
        for (at, &(_, _, v)) in row.iter().enumerate() {
            if let Some(end) = engine.find(&v, from) {
                from = end;
                hit.push((at, end));
            }
        }
        hit
    };
    let engines: Vec<Engine> = bands
        .iter()
        .map(|line| Engine::new(&joined(line)))
        .collect();
    let mut own_lines = std::collections::BTreeMap::new();
    for (&number, (row, own)) in &rows {
        // Most values found in order, then most of the row's words, then the line nearest the top.
        let mut best = ((0, 0), 0);
        for (at, engine) in engines.iter().enumerate() {
            let score = (
                hits(engine, row).len(),
                own.intersection(&engine.words).count(),
            );
            if score > best.0 {
                best = (score, at);
            }
        }
        if best.0 > (0, 0) {
            own_lines.insert(number, best.1);
        }
    }
    // Two rows swapped whole (review): each one's label — its words of letters no other row
    // holds — lies on the other's own line and not on its own. Their values were each found on the
    // other's line, so neither binds. A label alone on another row's line is no swap: a table
    // centres a label over the rows it spans, and sets a header over two lines.
    let mut holders: std::collections::BTreeMap<&str, usize> = Default::default();
    for (_, own) in rows.values() {
        for w in own.iter().filter(|w| !w.chars().any(is_digit)) {
            *holders.entry(w).or_default() += 1;
        }
    }
    let holds = |number: u32, line: usize| {
        rows[&number].1.iter().any(|w| {
            holders.get(w.as_str()) == Some(&1) && engines[line].words.contains(w.as_str())
        })
    };
    let mut by_line: std::collections::BTreeMap<usize, Vec<u32>> = Default::default();
    for (&number, &line) in &own_lines {
        by_line.entry(line).or_default().push(number);
    }
    let swapped: std::collections::BTreeSet<u32> = own_lines
        .iter()
        .filter(|&(&a, &own_a)| {
            !holds(a, own_a)
                && by_line.iter().any(|(&line, others)| {
                    holds(a, line) && others.iter().any(|&b| holds(b, own_a) && !holds(b, line))
                })
        })
        .map(|(&a, _)| a)
        .collect();
    let mut taken: Vec<usize> = own_lines.values().copied().collect();
    taken.sort_unstable();
    taken.dedup();
    let mut found = std::collections::BTreeSet::new();
    let mut spread = std::collections::BTreeMap::new();
    for (&number, &own_line) in &own_lines {
        let at = taken.partition_point(|&l| l < own_line);
        let from = match at {
            0 => own_line.saturating_sub(1),
            _ => taken[at - 1] + 1,
        };
        let to = match taken.get(at + 1) {
            Some(&next) => next - 1,
            None => (own_line + 1).min(bands.len() - 1),
        };
        let mut pieces: Vec<(i64, usize, &str)> = (from..=to)
            .flat_map(|line| bands[line].iter().map(move |(l, t)| (*l, line, t.as_str())))
            .collect();
        pieces.sort_unstable();
        let lefts: Vec<i64> = pieces.iter().map(|(l, _, _)| *l).collect();
        let texts: Vec<&str> = pieces.into_iter().map(|(_, _, t)| t).collect();
        let span = Engine::new(&texts.join(PIECE));
        let row = &rows[&number].0;
        if swapped.contains(&number) {
            spread.insert(number, span.words);
            continue;
        }
        // Each value found, by its place in the row, with its piece's left edge.
        let placed: std::collections::BTreeMap<usize, i64> = hits(&span, row)
            .into_iter()
            .map(|(at, end)| (at, lefts[span.squashed[..end].matches(PIECE_MARK).count()]))
            .collect();
        for (at, &(index, k, v)) in row.iter().enumerate() {
            if placed.contains_key(&at) {
                found.insert((index, k));
                continue;
            }
            // A cell set on two lines whose second falls on a neighbouring row's line: its value
            // on the line just before or after the row's, in its column, from the row's value
            // found before it to the one found after. The cell shows on its own row's lines too —
            // another of its values, or one of its words of letters — so a single wrong value never
            // borrows its neighbour's; past the last value found, nothing binds.
            let shows = row
                .iter()
                .enumerate()
                .any(|(other, &(cell, _, _))| cell == index && placed.contains_key(&other))
                || words(&lines[index].1)
                    .iter()
                    .any(|w| !w.chars().any(is_digit) && span.words.contains(w));
            let Some((_, &right)) = placed.range(at..).next().filter(|_| shows) else {
                continue;
            };
            let left = placed.range(..at).next_back().map_or(i64::MIN, |(_, &x)| x);
            let near = [from.checked_sub(1), Some(to + 1)];
            let stacked = near
                .into_iter()
                .flatten()
                .filter(|&l| l < bands.len())
                .any(|l| {
                    let column: Vec<&str> = bands[l]
                        .iter()
                        .filter(|(x, _)| (left..right).contains(x))
                        .map(|(_, t)| t.as_str())
                        .collect();
                    Engine::new(&column.join(PIECE)).find(&v, 0).is_some()
                });
            if stacked {
                found.insert((index, k));
            }
        }
        spread.insert(number, span.words);
    }
    (found, spread, swapped)
}

/// A value as written: its token, and what touches it — a minus sign or brackets, which make it
/// negative, a currency symbol, a percent sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Value<'a> {
    token: &'a str,
    negative: bool,
    currency: Option<char>,
    percent: Option<char>,
}

impl Value<'_> {
    /// Whether `other`, the same token, reads as this value: the same sign, and the same currency
    /// symbol and percent sign where both show one.
    fn agrees(&self, other: &Value) -> bool {
        let both = |a: Option<char>, b: Option<char>| a.is_none() || b.is_none() || a == b;
        self.negative == other.negative
            && both(self.currency, other.currency)
            && both(self.percent, other.percent)
    }
}

/// The value whose token is `text[start..end]`, with what goes with it: up to two marks on each
/// side, a minus touching it, brackets, a currency symbol or a percent sign across a space or a piece
/// break.
fn value_at(text: &str, start: usize, end: usize) -> Value<'_> {
    let currency = |c: char| {
        matches!(
            c,
            '$' | '¢' | '£' | '¤' | '¥' | '\u{20A0}'
                ..='\u{20C0}'
                    | '\u{0E3F}'
                    | '\u{FDFC}'
                    | '\u{FF04}'
                    | '\u{FFE0}'
                    | '\u{FFE1}'
                    | '\u{FFE5}'
                    | '\u{FFE6}'
        )
    };
    let mut v = Value {
        token: &text[start..end],
        negative: false,
        currency: None,
        percent: None,
    };
    // Up to two marks on each side, across spaces and the engine's piece breaks: brackets, a leading
    // currency symbol or a percent sign set apart still go with the value. A minus or a trailing
    // currency symbol only touching it, so the `-` of `3 - 5` is no sign and the `$` of `5 $7` is
    // the 7's.
    let gap = |c: char| c.is_whitespace() || c == PIECE_MARK;
    let (mut minus, mut open, mut close, mut apart, mut marks) = (false, false, false, false, 0);
    // Brackets a right-to-left line draws mirrored, `)x(`, read as `(x)` where they touch it.
    let mirrored = text[..start].ends_with(')') && text[end..].starts_with('(');
    for c in text[..start].chars().rev() {
        if gap(c) {
            apart = true;
            continue;
        }
        match c {
            '-' | '\u{2212}' | '\u{2013}' if !apart => minus = true,
            '(' => open = true,
            c if currency(c) => v.currency = v.currency.or(Some(c)),
            _ => break,
        }
        marks += 1;
        if marks == 2 {
            break;
        }
    }
    (marks, apart) = (0, false);
    for c in text[end..].chars() {
        if gap(c) {
            apart = true;
            continue;
        }
        match c {
            ')' => close = true,
            '%' | '\u{2030}' | '\u{FF05}' => v.percent = v.percent.or(Some(c)),
            c if currency(c) && !apart => v.currency = v.currency.or(Some(c)),
            _ => break,
        }
        marks += 1;
        if marks == 2 {
            break;
        }
    }
    v.negative = minus || (open && close) || mirrored;
    v
}

/// The engine's text inside one box, or one line of it, as the guard reads it.
struct Engine {
    /// The text as the engine joined it.
    text: String,
    words: std::collections::BTreeSet<String>,
    /// The text with no whitespace.
    squashed: String,
    /// For each byte boundary of `squashed`, whether it falls strictly inside one of the text's
    /// values.
    within: Vec<bool>,
    /// For each byte of `squashed`, its byte in `text`.
    origin: Vec<usize>,
    /// Whether the text holds any value.
    has_values: bool,
}

impl Engine {
    /// The engine's text is the page's, never markup: a page that prints `<` and `>` keeps what
    /// stands between them (the prototype removed it as a tag).
    fn new(text: &str) -> Self {
        let text = &hyphenated(text);
        let spans = value_spans(text);
        let mut next = spans.iter().peekable();
        let mut squashed = String::with_capacity(text.len());
        let mut within = Vec::with_capacity(text.len() + 1);
        let mut origin = Vec::with_capacity(text.len());
        for (at, c) in text.char_indices().filter(|(_, c)| !c.is_whitespace()) {
            while next.next_if(|&&(_, end)| end <= at).is_some() {}
            within.push(next.peek().is_some_and(|&&(start, _)| start < at));
            // Inside a character no value starts or ends.
            within.resize(within.len() + c.len_utf8() - 1, true);
            origin.extend(at..at + c.len_utf8());
            squashed.push(c);
        }
        within.push(false);
        Self {
            text: text.into(),
            words: words(text),
            squashed,
            within,
            origin,
            has_values: !spans.is_empty(),
        }
    }

    /// Where `value` is found at or after byte `from` of the squashed text, as the byte after it:
    /// it occurs there, starting and ending outside the text's own values, and reads as the same
    /// value in the text (`Value::agrees`).
    fn find(&self, value: &Value, from: usize) -> Option<usize> {
        let mut from = from;
        while let Some(i) = self.squashed.get(from..)?.find(value.token) {
            let at = from + i;
            let end = at + value.token.len();
            if !self.within[at] && !self.within[end] {
                let here = value_at(&self.text, self.origin[at], self.origin[end - 1] + 1);
                if here.agrees(value) {
                    return Some(end);
                }
            }
            // A value starts with a character, so the next character starts after its first byte.
            from = at + self.squashed[at..].chars().next().map_or(1, char::len_utf8);
        }
        None
    }
}

/// Decision #123's words: the distinct runs of letters, digits and `_` of two characters or more,
/// lower-cased.
pub(crate) fn words(text: &str) -> std::collections::BTreeSet<String> {
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| w.chars().nth(1).is_some())
        .map(String::from)
        .collect()
}

/// The values of the model's `text`, in order, each with what touches it (`value_at`). A LaTeX
/// command, a backslash and the lower-case letters after it, is no part of a value: `\pm20V` holds
/// `20V`, while a path's `\O365` keeps its value.
fn values(text: &str) -> Vec<Value<'_>> {
    value_spans(text)
        .into_iter()
        .map(|(start, end)| {
            let command = if text[..start].ends_with('\\') {
                text[start..end]
                    .bytes()
                    .take_while(u8::is_ascii_lowercase)
                    .count()
            } else {
                0
            };
            value_at(text, start + command, end)
        })
        .collect()
}

/// `text` with each dash between two letters or digits written `-`: U+2010 to U+2015 and U+2212
/// join a value as `-` does, so the engine's `99‐R98CR` is the model's `99-R98CR`, whichever dash
/// each side wrote. A dash touching a value on one side only is left as written, so a minus stays
/// one.
fn hyphenated(text: &str) -> String {
    let joins = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphabetic() || is_digit(c));
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut before = None;
    while let Some(c) = chars.next() {
        let dash = matches!(c, '\u{2010}'..='\u{2015}' | '\u{2212}');
        let between = joins(before) && joins(chars.peek().copied());
        out.push(if dash && between { '-' } else { c });
        before = Some(c);
    }
    out
}

/// The first code point of each block of ten Unicode decimal digits (general category `Nd`),
/// Unicode 15.1: derived by `[c for c in range(0x110000) if unicodedata.category(chr(c)) == "Nd"
/// and unicodedata.decimal(chr(c)) == 0]` under Python 3.13; every block holds its ten digits.
const DECIMAL_ZEROS: [u32; 68] = [
    0x30, 0x660, 0x6F0, 0x7C0, 0x966, 0x9E6, 0xA66, 0xAE6, 0xB66, 0xBE6, 0xC66, 0xCE6, 0xD66,
    0xDE6, 0xE50, 0xED0, 0xF20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
    0x104A0, 0x10D30, 0x11066, 0x110F0, 0x11136, 0x111D0, 0x112F0, 0x11450, 0x114D0, 0x11650,
    0x116C0, 0x11730, 0x118E0, 0x11950, 0x11C50, 0x11D50, 0x11DA0, 0x11F50, 0x16A60, 0x16AC0,
    0x16B50, 0x1D7CE, 0x1D7D8, 0x1D7E2, 0x1D7EC, 0x1D7F6, 0x1E140, 0x1E2F0, 0x1E4F0, 0x1E950,
    0x1FBF0,
];

/// Whether `c` is a decimal digit of any script.
///
/// Public for the PDF reader's section numbers alone (`type-size-v8`, decision #174), and no
/// supported API (`docs/PUBLIC-API.md`). **A change here changes both**: it moves that rule id, and
/// `profile_sha256` with it, as well as `assist-guard-v1`.
#[doc(hidden)]
pub fn is_digit(c: char) -> bool {
    let c = u32::from(c);
    let at = DECIMAL_ZEROS.partition_point(|&zero| zero <= c);
    at > 0 && c - DECIMAL_ZEROS[at - 1] < 10
}

/// [`values`] as byte ranges: the prototype's `[A-Za-z0-9]*\d(?:[A-Za-z0-9.,:/\-]*[A-Za-z0-9])?`,
/// found left to right, with any script's decimal digit for `0-9`. A value starts a run of ASCII
/// letters and digits that holds a digit, and goes on through letters, digits and `. , : / -` to
/// its last letter or digit.
fn value_spans(text: &str) -> Vec<(usize, usize)> {
    let alnum = |c: char| c.is_ascii_alphabetic() || is_digit(c);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(c) = text[at..].chars().next() {
        let run: Vec<char> = text[at..].chars().take_while(|&c| alnum(c)).collect();
        if !run.iter().any(|&c| is_digit(c)) {
            at += run
                .iter()
                .map(|c| c.len_utf8())
                .sum::<usize>()
                .max(c.len_utf8());
            continue;
        }
        let mut end = at;
        let mut last = at;
        for c in text[at..]
            .chars()
            .take_while(|&c| alnum(c) || ".,:/-".contains(c))
        {
            end += c.len_utf8();
            if alnum(c) {
                last = end;
            }
        }
        out.push((at, last));
        at = last;
    }
    out
}

/// The model's `text` as the guard reads it: its tags removed, then its character references
/// decoded.
fn plain(text: &str) -> String {
    decoded(&untagged(text))
}

/// `text` with the HTML character references a reply writes decoded: `&lt;`, `&gt;`, `&amp;`,
/// `&quot;`, `&#39;` and `&apos;`, and any decimal or hexadecimal reference to a character. Any
/// other `&` stays as written.
pub(crate) fn decoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        // The longest reference read, `#x10FFFF`, is eight bytes before its `;`.
        let name = rest.bytes().skip(1).take(9).position(|b| b == b';');
        let name = name.map(|end| &rest[1..=end]);
        let c = name.and_then(|name| match name {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => {
                let number = name.strip_prefix('#')?;
                let (digits, radix) = match number.strip_prefix(['x', 'X']) {
                    Some(hex) => (hex, 16),
                    None => (number, 10),
                };
                let all = !digits.is_empty() && digits.chars().all(|d| d.is_digit(radix));
                all.then(|| u32::from_str_radix(digits, radix).ok())
                    .flatten()
                    .and_then(char::from_u32)
            }
        });
        match (c, name) {
            (Some(c), Some(name)) => {
                out.push(c);
                rest = &rest[name.len() + 2..];
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// `text` with each tag, a `<` and the next `>` with something between, made one space.
pub(crate) fn untagged(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open + 1..].find('>') else {
            break;
        };
        if close == 0 {
            out.push_str(&rest[..open + 2]); // `<>` is no tag.
        } else {
            out.push_str(&rest[..open]);
            out.push(' ');
        }
        rest = &rest[open + 2 + close..];
    }
    out.push_str(rest);
    out
}

/// `text` with its whitespace runs made one space, and none at either end.
pub(crate) fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A block's lines, in its order, as the guard reads them, whitespace collapsed: for a table or chart, each cell with
/// its row, and the text outside any cell (a caption, words after the table) line by line; for
/// any other block, its text line by line. Blank lines outside a cell are not lines.
pub(crate) fn block_lines(label: Option<&str>, text: &str) -> Vec<(Option<u32>, String)> {
    let free = |text: &str| -> Vec<(Option<u32>, String)> {
        text_lines(text).into_iter().map(|l| (None, l)).collect()
    };
    if !matches!(label, Some("table" | "chart")) {
        return free(text);
    }
    table_parts(text)
        .into_iter()
        .flat_map(|(row, part)| match row {
            Some(_) => vec![(row, collapsed(&plain(part)))],
            None => free(part),
        })
        .collect()
}

/// Text outside a table's cells as the guard reads it, line by line: its tags removed, its
/// character references decoded, each line's whitespace collapsed, blank lines left out.
pub(crate) fn text_lines(text: &str) -> Vec<String> {
    plain(text)
        .lines()
        .map(collapsed)
        .filter(|l| !l.is_empty())
        .collect()
}

/// A table's parts, in order: what each `<td>` and `<th>` holds, up to the next tag of a cell, row
/// or table, with its row's number (each `<tr>` starts one); and what lies outside every cell.
/// A tag that no `>` closes ends the table: nothing after it is read.
fn table_parts(html: &str) -> Vec<(Option<u32>, &str)> {
    cells_of(html)
        .into_iter()
        .map(|(row, _, part)| (row, part))
        .collect()
}

/// [`table_parts`], each cell with its opening tag (`<td colspan="2">`), which the projection reads
/// a cell's kind and spans from (decision #147); a part outside every cell has an empty tag.
pub(crate) fn cells_of(html: &str) -> Vec<(Option<u32>, &str, &str)> {
    let lower = html.to_ascii_lowercase();
    let mut out = Vec::new();
    let (mut row, mut cell, mut free, mut at) = (0u32, None, 0, 0);
    while let Some(i) = lower[at..].find('<') {
        let start = at + i;
        at = start + 1;
        let closing = lower[at..].starts_with('/');
        let name = lower[at + usize::from(closing)..]
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()
            .unwrap_or_default();
        if !matches!(name, "td" | "th" | "tr" | "table") {
            continue;
        }
        match cell.take() {
            Some((tag, from)) => out.push((Some(row), &html[tag..from], &html[from..start])),
            None => out.push((None, "", &html[free..start])),
        }
        let Some(end) = lower[start..].find('>') else {
            return out;
        };
        at = start + end + 1;
        free = at;
        if !closing && name == "tr" {
            row += 1;
        }
        if !closing && matches!(name, "td" | "th") {
            cell = Some((start, at));
        }
    }
    match cell {
        Some((tag, from)) => out.push((Some(row), &html[tag..from], &html[from..])),
        None => out.push((None, "", &html[free..])),
    }
    out
}

/// Whether `text` ends in four or more copies of one string of 1 to 400 characters, 64 characters
/// or more in all, the last copy perhaps cut: the tail of a model that loops until the cap.
fn loops(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    (1..=400).take_while(|&p| 4 * p <= n).any(|p| {
        let repeated = (p..n).rev().take_while(|&i| c[i] == c[i - p]).count();
        p + repeated >= (4 * p).max(64)
    })
}

/// The fewest distinct words a page's runs may hold without the trigger asking a model to read the
/// page: [`words`], over the page's runs joined in order.
const TRIGGER_WORDS: usize = 20;

/// The pages, by number, `extract --assist` asks a model to read under `--assist-pages trigger`
/// (design §6, decision #150): each page of the sealed record where one of six signals fires.
///
/// - **S1, a scan:** the page's `scanned-page-not-recognized`, or a recognized run on it.
/// - **S2, few words:** under 20 distinct words (decision #123's) in its runs, joined in order,
///   a space between two lines.
/// - **S3, a figure:** a figure region on it, a table read from it or not.
/// - **S4, a table:** a table on it.
/// - **S5, a picture:** an image painted over a quarter of the page's visible box or more.
/// - **S6, U+FFFD:** a run holding it.
pub fn assist_trigger(repr: &DocumentRepresentation) -> Vec<u32> {
    triggered(repr.payload())
}

fn triggered(p: &crate::representation::RepresentationPayload) -> Vec<u32> {
    use std::collections::{BTreeMap, BTreeSet};
    // One pass over the nodes, grouped by page: a page's own scan of every node would cost pages
    // times nodes, minutes on a document of a million runs.
    let mut by_page: BTreeMap<&str, Vec<&crate::Node>> = BTreeMap::new();
    for n in &p.nodes {
        by_page.entry(n.parent.as_str()).or_default().push(n);
    }
    let figures: BTreeSet<&str> = p.figures.iter().map(|f| f.page.as_str()).collect();
    let tables: BTreeSet<&str> = p.tables.iter().map(|t| t.page.as_str()).collect();
    let scanned: BTreeSet<u32> = p
        .assurance
        .limitations
        .iter()
        .filter(|l| l.code == crate::assurance::codes::SCANNED_PAGE_NOT_RECOGNIZED)
        .filter_map(|l| match l.scope {
            crate::LimitationScope::Page(index) => Some(index),
            _ => None,
        })
        .collect();
    let fires = |page: &crate::representation::PageRecord| {
        let on_page = by_page.get(page.id.as_str()).map_or(&[][..], Vec::as_slice);
        let runs = || {
            on_page
                .iter()
                .filter(|n| n.kind == crate::NodeKind::TextRun)
        };
        // Runs on one baseline join as they stand, since the engine splits a word it kerns; a
        // run on another baseline opens a new line, after a space, so no two words fuse.
        let (mut text, mut baseline) = (String::new(), None);
        for n in runs() {
            let at = n.native_locator.run_locator().map(|at| at.origin_y);
            if baseline.is_some_and(|b| b != at) {
                text.push(' ');
            }
            baseline = Some(at);
            text.push_str(n.reading_text());
        }
        let [left, top, width, height] = frame_of(page).map(i128::from);
        let covers = |r: QRect| {
            let side = |a0: i64, a1: i64, from: i128, size: i128| {
                (i128::from(a1).min(from + size) - i128::from(a0).max(from)).max(0)
            };
            4 * side(r.x0(), r.x1(), left, width) * side(r.y0(), r.y1(), top, height)
                >= width * height
        };
        scanned.contains(&page.index)
            || runs().any(|n| n.derivation == DerivationClass::Recognized)
            || words(&text).len() < TRIGGER_WORDS
            || figures.contains(page.id.as_str())
            || tables.contains(page.id.as_str())
            // An image's box is where its `Do` painted it, never a measured geometry.
            || on_page.iter().any(|n| {
                matches!(&n.native_locator, crate::representation::NativeLocator::PdfImage(at)
                    if at.rect.painted().is_some_and(covers))
            })
            || text.contains('\u{FFFD}')
    };
    p.pages
        .iter()
        .filter(|page| fires(page))
        .map(|page| page.index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assurance::{Assurance, PageState, PageStateEntry};
    use crate::geom::QRect;
    use crate::identity::{ArtifactIdentity, CoordinateSystem};
    use crate::profile::Capabilities;
    use crate::representation::{
        PageRecord, ProcessingRun, ProcessorIdentity, RepresentationPayload, SourceIdentity,
        REPRESENTATION_ARTIFACT_TYPE, REPRESENTATION_SCHEMA_VERSION,
    };
    use serde_json::{json, Value};

    const SOURCE: &[u8] = b"made-up source bytes";

    /// A record of two 300 x 400 point pages, the second showing a 200 x 300 point crop, read
    /// under `profile`, with no nodes: the replies attach beside nodes, never to them. The document
    /// has a third page, held back by a page budget, which the record does not list.
    fn record_under(profile: &Profile) -> DocumentRepresentation {
        let mut ids = IdAllocator::new(profile.profile_sha256().unwrap());
        let pages = vec![
            PageRecord {
                id: ids.next(IdKind::Page).unwrap(),
                index: 1,
                width: 30000,
                height: 40000,
                rotation: 0,
                turn: 0,
                visible: None,
                drawn_boxes: Vec::new(),
            },
            PageRecord {
                id: ids.next(IdKind::Page).unwrap(),
                index: 2,
                width: 30000,
                height: 40000,
                rotation: 0,
                turn: 0,
                visible: Some(QRect::new(5000, 5000, 25000, 35000).unwrap()),
                drawn_boxes: Vec::new(),
            },
        ];
        let mut states: Vec<PageStateEntry> = (1..=2)
            .map(|index| PageStateEntry {
                index,
                state: PageState::Processed,
            })
            .collect();
        states.push(PageStateEntry {
            index: 3,
            state: PageState::Quarantined("resource-limit-pages".into()),
        });
        let payload = RepresentationPayload {
            identity: ArtifactIdentity {
                artifact_type: REPRESENTATION_ARTIFACT_TYPE.into(),
                schema_version: REPRESENTATION_SCHEMA_VERSION.into(),
                parser_version: profile.parser_version.clone(),
                profile_sha256: profile.profile_sha256().unwrap(),
            },
            source: SourceIdentity {
                media_type: "application/pdf".into(),
                sha256: Sha256Hex::of_bytes(SOURCE),
            },
            processing_run: ProcessingRun {
                processor: ProcessorIdentity {
                    name: "ethos-parser".into(),
                    version: profile.parser_version.clone(),
                    backend: "lopdf 0.44.0".into(),
                },
                reading_order_rule: profile.reading_order_rule.clone(),
            },
            coordinate_system: CoordinateSystem::V0,
            pages,
            nodes: Vec::new(),
            tables: Vec::new(),
            outlines: Vec::new(),
            figures: Vec::new(),
            assist: None,
            assurance: Assurance::new(Capabilities::V0, 3, states, Vec::new()).unwrap(),
        };
        DocumentRepresentation::seal(payload, Vec::new()).unwrap()
    }

    fn record() -> DocumentRepresentation {
        record_under(&Profile::default())
    }

    /// A made-up replies file for [`record`]: page 1 read, page 2 an error.
    fn file() -> Value {
        let pixels = Sha256Hex::of_bytes(b"made-up pixels").to_string();
        json!({
            "ethos_assist_answers": 1,
            "source_sha256": Sha256Hex::of_bytes(SOURCE).to_string(),
            "renderer": ASSIST_RENDERER,
            "request": ASSIST_REQUEST_V1,
            "model": {"id": "example/made-up-model", "version": "r1"},
            "pages": [
                {
                    "page": 1, "points": [30000, 40000], "dpi": 200, "width": 833,
                    "height": 1111, "pixels_sha256": pixels, "finish_reason": "stop",
                    "reply": "![title](100,50,900,90) # A made-up heading\n\n\
                              ![table](100,120,900,400) <table><tr><td>12</td></tr></table>"
                },
                {
                    "page": 2, "points": [20000, 30000], "dpi": 200, "width": 555,
                    "height": 833, "pixels_sha256": pixels, "error": "http 503"
                }
            ]
        })
    }

    fn bytes(v: &Value) -> Vec<u8> {
        serde_json::to_vec(v).unwrap()
    }

    fn attach_file(v: &Value) -> Result<DocumentRepresentation, EngineError> {
        attach(record(), &Profile::default(), &bytes(v), None)
    }

    /// The name of the refusal `v` meets.
    fn refusal(v: &Value, model: Option<&str>) -> String {
        match attach(record(), &Profile::default(), &bytes(v), model) {
            Err(EngineError::Malformed { what, .. }) => what,
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn a_replies_file_reads_into_proposed_blocks_under_a_pinned_profile() {
        let v = file();
        let repr = attach_file(&v).unwrap();
        let assist = repr.payload().assist.as_ref().unwrap();
        assert_eq!(assist.answers_sha256, Sha256Hex::of_bytes(&bytes(&v)));
        assert_eq!(assist.model.id, "example/made-up-model");
        assert_eq!(assist.model.version, "r1");

        let [read, failed] = &assist.pages[..] else {
            panic!("two pages");
        };
        assert_eq!(read.page.as_str(), "p1");
        assert_eq!(
            (read.dpi, read.width, read.height, read.outcome),
            (200, 833, 1111, AssistOutcome::Read)
        );
        assert_eq!(read.finish_reason.as_deref(), Some("stop"));
        let ids: Vec<&str> = read.blocks.iter().map(|b| b.id.as_str()).collect();
        assert_eq!(ids, ["m1", "m2"]);
        let got: Vec<Read> = read
            .blocks
            .iter()
            .map(|b| Read {
                label: b.label.clone(),
                continues: b.continues,
                grid: b.grid,
                text: b.text.clone(),
            })
            .collect();
        assert_eq!(
            got,
            [
                self::read(
                    Some("title"),
                    false,
                    Some([100, 50, 900, 90]),
                    "# A made-up heading"
                ),
                self::read(
                    Some("table"),
                    false,
                    Some([100, 120, 900, 400]),
                    "<table><tr><td>12</td></tr></table>"
                ),
            ]
        );
        assert!(read
            .blocks
            .iter()
            .all(|b| b.derivation == DerivationClass::Proposed));

        assert_eq!(failed.page.as_str(), "p2");
        assert_eq!(failed.outcome, AssistOutcome::Error);
        assert_eq!(failed.finish_reason, None);
        assert!(failed.blocks.is_empty());

        let pin = AssistPin {
            rule: ASSIST_RULE_V1.into(),
            guard: ASSIST_GUARD_V1.into(),
            renderer: ASSIST_RENDERER.into(),
            request: ASSIST_REQUEST_V1.into(),
            model: "example/made-up-model".into(),
            model_version: "r1".into(),
        };
        assert_eq!(
            repr.payload().identity.profile_sha256,
            Profile::default()
                .with_assist(pin)
                .profile_sha256()
                .unwrap()
        );
    }

    /// The record differs from the engine's in the profile hash, the `assist` key and the
    /// fingerprint alone, and the hand-written hashing path agrees with the serialized one.
    #[test]
    fn an_assisted_record_differs_only_in_its_hash_and_assist_key() {
        let engine = record();
        let assisted = attach_file(&file()).unwrap();
        assisted.verify_fingerprint().unwrap();
        assert_ne!(engine.fingerprint(), assisted.fingerprint());

        let strip = |r: &DocumentRepresentation| {
            let mut v = serde_json::to_value(r).unwrap();
            let payload = v["representation"].as_object_mut().unwrap();
            payload.remove("assist");
            payload["identity"]["profile_sha256"] = Value::Null;
            v["representation_c14n_sha256"] = Value::Null;
            v
        };
        assert_eq!(strip(&engine), strip(&assisted));
        assert!(serde_json::to_value(&engine).unwrap()["representation"]
            .get("assist")
            .is_none());

        // And the bytes read back as the same record, checks and all.
        let text = assisted.to_canonical_bytes().unwrap();
        let back: DocumentRepresentation = serde_json::from_slice(&text).unwrap();
        back.verify_fingerprint().unwrap();
        assert_eq!(back, assisted);
        // The parsed record prints through the serialized route, the minted one through the
        // hand-written walk: the two agree, `assist` and all, and so does the fingerprint.
        assert_eq!(back.to_canonical_bytes().unwrap(), text);
        assert_eq!(
            assisted.fingerprint(),
            &crate::Sha256Hex::of_bytes(&assisted.payload().canonical_bytes().unwrap())
        );
    }

    #[test]
    fn the_same_source_and_replies_give_the_same_bytes() {
        let a = attach_file(&file()).unwrap().to_canonical_bytes().unwrap();
        let b = attach_file(&file()).unwrap().to_canonical_bytes().unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn an_unreadable_replies_file_is_refused_by_name() {
        let mut cases: Vec<(&str, Vec<u8>)> = vec![("not JSON", b"{\"pages\": [".to_vec())];
        let mut add = |why: &'static str, edit: &dyn Fn(&mut Value)| {
            let mut v = file();
            edit(&mut v);
            cases.push((why, bytes(&v)));
        };
        add("an unknown field", &|v| v["server"] = json!("http://x"));
        add("an unknown page field", &|v| {
            v["pages"][0]["seconds"] = json!(3)
        });
        add("another shape version", &|v| {
            v["ethos_assist_answers"] = json!(2)
        });
        add("pages out of order", &|v| {
            v["pages"].as_array_mut().unwrap().reverse();
        });
        add("a page twice", &|v| v["pages"][1]["page"] = json!(1));
        add("a reply and an error", &|v| {
            v["pages"][0]["error"] = json!("http 500");
        });
        add("neither a reply nor an error", &|v| {
            v["pages"][1].as_object_mut().unwrap().remove("error");
        });
        add("a reply with no finish reason", &|v| {
            v["pages"][0]
                .as_object_mut()
                .unwrap()
                .remove("finish_reason");
        });
        add("an error with a finish reason", &|v| {
            v["pages"][1]["finish_reason"] = json!("stop");
        });
        add("a digest not in `sha256:` form", &|v| {
            v["source_sha256"] = json!("abc");
        });
        add("a fractional number", &|v| {
            v["pages"][0]["dpi"] = json!(200.5)
        });
        add("an empty model id", &|v| v["model"]["id"] = json!(""));
        add("a blank model id", &|v| v["model"]["id"] = json!(" \t"));
        for (why, b) in cases {
            match attach(record(), &Profile::default(), &b, None) {
                Err(EngineError::Malformed { what, .. }) => {
                    assert_eq!(what, ASSIST_ANSWERS_UNREADABLE, "{why}");
                }
                other => panic!("{why}: expected a refusal, got {other:?}"),
            }
        }
    }

    /// The id the model was asked by is read beside the one its server reported (review), for
    /// `ethos-parser-assist` to resume the file under; the record keeps the id and version alone,
    /// and any other key under `model` is refused.
    #[test]
    fn the_id_a_model_was_asked_by_is_read_and_not_recorded() {
        let mut v = file();
        v["model"]["asked"] = json!("made-up-alias");
        let repr = attach_file(&v).unwrap();
        let model = &repr.payload().assist.as_ref().unwrap().model;
        assert_eq!(
            (model.id.as_str(), model.version.as_str()),
            ("example/made-up-model", "r1")
        );
        v["model"]["server"] = json!("http://x");
        assert_eq!(refusal(&v, None), ASSIST_ANSWERS_UNREADABLE);
    }

    #[test]
    fn replies_to_other_source_bytes_are_refused() {
        let mut v = file();
        v["source_sha256"] = json!(Sha256Hex::of_bytes(b"other bytes").to_string());
        assert_eq!(refusal(&v, None), ASSIST_ANSWERS_OTHER_SOURCE);
    }

    #[test]
    fn pages_another_renderer_drew_are_refused() {
        let mut v = file();
        v["renderer"] = json!("ghostscript 10.06.0; pages-400dpi-5mp-v1");
        assert_eq!(refusal(&v, None), ASSIST_ANSWERS_OTHER_RENDERER);
    }

    #[test]
    fn replies_to_another_request_are_refused() {
        let mut v = file();
        v["request"] = json!("some-other-request-v1");
        assert_eq!(refusal(&v, None), ASSIST_ANSWERS_OTHER_REQUEST);
    }

    #[test]
    fn replies_from_another_model_than_the_one_named_are_refused() {
        let v = file();
        for named in ["example/other-model", "example/made-up-model@r2", "r1"] {
            assert_eq!(
                refusal(&v, Some(named)),
                ASSIST_ANSWERS_OTHER_MODEL,
                "{named}"
            );
        }
        for named in ["example/made-up-model", "example/made-up-model@r1"] {
            attach(record(), &Profile::default(), &bytes(&v), Some(named))
                .unwrap_or_else(|e| panic!("{named}: {e}"));
        }
    }

    #[test]
    fn a_page_the_record_does_not_hold_or_show_at_that_size_is_refused() {
        let mut cases: Vec<(&str, Value)> = Vec::new();
        let mut add = |why: &'static str, edit: &dyn Fn(&mut Value)| {
            let mut v = file();
            edit(&mut v);
            cases.push((why, v));
        };
        add("page 4 of 3", &|v| v["pages"][1]["page"] = json!(4));
        add("page 0", &|v| {
            v["pages"].as_array_mut().unwrap().remove(1);
            v["pages"][0]["page"] = json!(0);
        });
        add("wider by over a point", &|v| {
            v["pages"][0]["points"] = json!([30101, 40000]);
        });
        add("shorter by over a point", &|v| {
            v["pages"][0]["points"] = json!([30000, 39899]);
        });
        // The second page shows a crop: its media box is not what was drawn.
        add("the media box of a cropped page", &|v| {
            v["pages"][1]["points"] = json!([30000, 40000]);
        });
        for (why, v) in cases {
            assert_eq!(refusal(&v, None), ASSIST_ANSWERS_OTHER_PAGE, "{why}");
        }

        // Within a point either way is the same page.
        let mut v = file();
        v["pages"][0]["points"] = json!([30100, 39900]);
        v["pages"][1]["points"] = json!([19900, 30100]);
        attach_file(&v).unwrap();
    }

    /// A page the document has and the record does not list — cut by `--max-pages`, or not read —
    /// has nowhere for its blocks to hang: its reply is left out, and its number kept.
    #[test]
    fn a_page_the_record_does_not_list_is_left_out_and_counted() {
        let mut v = file();
        let mut third = v["pages"][0].clone();
        third["page"] = json!(3);
        third["points"] = json!([1, 1]);
        v["pages"].as_array_mut().unwrap().push(third);
        let repr = attach_file(&v).unwrap();
        let assist = repr.payload().assist.as_ref().unwrap();
        let listed: Vec<&str> = assist.pages.iter().map(|p| p.page.as_str()).collect();
        assert_eq!(listed, ["p1", "p2"]);
        assert_eq!(assist.pages_left_out, [3]);
        let blocks: usize = assist.pages.iter().map(|p| p.blocks.len()).sum();
        assert_eq!(blocks, 2, "page 3's blocks are not kept");
        // And none left out is not written.
        let text = attach_file(&file()).unwrap().to_canonical_bytes().unwrap();
        assert!(!String::from_utf8(text).unwrap().contains("pages_left_out"));
    }

    #[test]
    fn a_proposal_never_replaces_another_or_lands_on_another_profile() {
        let once = attach_file(&file()).unwrap();
        let again = attach(once, &Profile::default(), &bytes(&file()), None);
        assert!(
            matches!(&again, Err(EngineError::Malformed { what, .. }) if what == "assist-base"),
            "{again:?}"
        );
        let other = attach(record(), &Profile::ocr_v0(), &bytes(&file()), None);
        assert!(
            matches!(&other, Err(EngineError::Malformed { what, .. }) if what == "assist-base"),
            "{other:?}"
        );
        // A record holding a model's blocks under the base profile's own hash, as a library
        // caller can seal one, is refused too: the hash alone does not say it was assisted.
        let (mut payload, geometry) = attach_file(&file()).unwrap().into_parts();
        payload.identity.profile_sha256 = Profile::default().profile_sha256().unwrap();
        let sealed = DocumentRepresentation::seal(payload, geometry).unwrap();
        let again = attach(sealed, &Profile::default(), &bytes(&file()), None);
        assert!(
            matches!(&again, Err(EngineError::Malformed { what, .. }) if what == ASSIST_BASE),
            "{again:?}"
        );
        // Under the OCR profile it reads was read under, it attaches.
        attach(
            record_under(&Profile::ocr_v0()),
            &Profile::ocr_v0(),
            &bytes(&file()),
            None,
        )
        .unwrap();
    }

    /// North Star row 12, mechanised (`docs/04-ARCHITECTURE.md` §6): the processor and the
    /// recognizer, however spelled.
    #[test]
    fn a_model_named_as_the_engine_is_refused_by_name() {
        for id in [
            "ethos-parser",
            " Ethos-Parser ",
            "ethos-parser-ocr",
            "ETHOS-PARSER-OCR",
        ] {
            let mut v = file();
            v["model"]["id"] = json!(id);
            assert_eq!(
                refusal(&v, None),
                ASSIST_ANSWERS_MODEL_IS_PROCESSOR,
                "{id:?}"
            );
        }
    }

    /// A server started with a local `--model` reports the path; every record would carry it.
    #[test]
    fn a_model_id_that_is_a_path_is_refused_by_name() {
        for id in [
            "/Users/someone/LightOnOCR-3-0.8B",
            "~/lighton-mlx",
            "./LightOnOCR-3-0.8B",
            "../models/m",
            ".cache/m",
            "C:\\models\\m",
            "c:/models/m",
            "models\\m",
            " /srv/m",
        ] {
            let mut v = file();
            v["model"]["id"] = json!(id);
            assert_eq!(refusal(&v, None), ASSIST_ANSWERS_MODEL_IS_PATH, "{id:?}");
        }
        for id in [
            "lightonai/LightOnOCR-3-0.8B",
            "LightOnOCR-3-0.8B",
            "org/name:q4",
        ] {
            let mut v = file();
            v["model"]["id"] = json!(id);
            attach_file(&v).unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }

    /// A reply of more blocks than 12,288 tokens can spell, or a file over its byte budget.
    #[test]
    fn replies_over_budget_are_refused_by_name() {
        let markers = |n: usize| "![text](1,2,3,4) x ".repeat(n);
        let mut v = file();
        v["pages"][0]["reply"] = json!(markers(ASSIST_MAX_BLOCKS_PER_PAGE));
        let repr = attach_file(&v).unwrap();
        assert_eq!(
            repr.payload().assist.as_ref().unwrap().pages[0]
                .blocks
                .len(),
            ASSIST_MAX_BLOCKS_PER_PAGE
        );
        v["pages"][0]["reply"] = json!(markers(ASSIST_MAX_BLOCKS_PER_PAGE + 1));
        assert_eq!(refusal(&v, None), ASSIST_ANSWERS_OVER_BUDGET);

        let mut b = bytes(&file());
        b.resize(ASSIST_ANSWERS_MAX_BYTES, b' ');
        attach(record(), &Profile::default(), &b, None).unwrap();
        b.push(b' ');
        match attach(record(), &Profile::default(), &b, None) {
            Err(EngineError::Malformed { what, .. }) => {
                assert_eq!(what, ASSIST_ANSWERS_OVER_BUDGET)
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    /// A record edited after the seal is refused when it is read, whichever page or block the edit
    /// touches: the seal holds what `attach` writes.
    #[test]
    fn an_edited_assist_record_is_refused_on_read() {
        let text = attach_file(&file()).unwrap().to_canonical_bytes().unwrap();
        let v: Value = serde_json::from_slice(&text).unwrap();
        let read = |edit: &dyn Fn(&mut Value)| {
            let mut edited = v.clone();
            edit(&mut edited["representation"]["assist"]);
            serde_json::from_value::<DocumentRepresentation>(edited)
        };
        // Unedited, and with a page the document has left out, it reads; so does a guard flagging
        // its one line, and a page the server cut, with its outcome.
        read(&|_| {}).unwrap();
        read(&|a| a["pages_left_out"] = json!([3])).unwrap();
        read(&|a| {
            a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                json!([{"line": 1, "mark": "contradicted", "value": "12"}]);
        })
        .unwrap();
        for flagged in [
            json!([{"line": 1, "mark": "disputed", "value": "12"}]),
            json!([{"line": 1, "mark": "disputed"}]),
        ] {
            read(&|a| a["pages"][0]["blocks"][1]["guard"]["flagged"] = flagged.clone()).unwrap();
        }
        for outcome in ["truncated", "loop"] {
            read(&|a| {
                a["pages"][0]["finish_reason"] = json!("length");
                a["pages"][0]["outcome"] = json!(outcome);
            })
            .unwrap();
        }

        type Edit = fn(&mut Value);
        let cases: &[(&str, Edit)] = &[
            ("an extracted first block", |a| {
                a["pages"][0]["blocks"][0]["derivation"] = json!("extracted");
            }),
            ("an extracted later block", |a| {
                a["pages"][0]["blocks"][1]["derivation"] = json!("computed");
            }),
            ("an undeclared first page", |a| {
                a["pages"][0]["page"] = json!("p9")
            }),
            ("an undeclared later page", |a| {
                a["pages"][1]["page"] = json!("p9")
            }),
            ("a page twice", |a| a["pages"][1]["page"] = json!("p1")),
            ("pages descending", |a| {
                a["pages"].as_array_mut().unwrap().reverse();
            }),
            ("an error with blocks", |a| {
                a["pages"][0]["outcome"] = json!("error")
            }),
            ("a node's id", |a| {
                a["pages"][0]["blocks"][0]["id"] = json!("e1")
            }),
            ("a page's id", |a| {
                a["pages"][0]["blocks"][1]["id"] = json!("p1")
            }),
            ("an id of no number", |a| {
                a["pages"][0]["blocks"][0]["id"] = json!("m")
            }),
            ("an id of a bad number", |a| {
                a["pages"][0]["blocks"][0]["id"] = json!("m1a");
            }),
            ("an id twice", |a| {
                a["pages"][0]["blocks"][1]["id"] = json!("m1")
            }),
            ("a box past the grid", |a| {
                a["pages"][0]["blocks"][1]["grid"] = json!([0, 0, 1001, 5]);
            }),
            ("a box below the grid", |a| {
                a["pages"][0]["blocks"][1]["grid"] = json!([0, 0, 5, 1001]);
            }),
            ("a box upside down", |a| {
                a["pages"][0]["blocks"][0]["grid"] = json!([0, 500, 5, 100]);
            }),
            ("an inverted box", |a| {
                a["pages"][0]["blocks"][0]["grid"] = json!([500, 0, 100, 5]);
            }),
            ("a box of three", |a| {
                a["pages"][0]["blocks"][0]["grid"] = json!([1, 2, 3])
            }),
            ("the processor's identity", |a| {
                a["model"]["id"] = json!("ethos-parser")
            }),
            ("the processor's, recased", |a| {
                a["model"]["id"] = json!(" ETHOS-parser")
            }),
            ("the recognizer's identity", |a| {
                a["model"]["id"] = json!("ethos-parser-ocr");
            }),
            ("an empty model id", |a| a["model"]["id"] = json!("")),
            ("a path for a model id", |a| a["model"]["id"] = json!("~/m")),
            ("a listed page left out", |a| {
                a["pages_left_out"] = json!([1])
            }),
            ("left out twice", |a| a["pages_left_out"] = json!([3, 3])),
            ("page 0 left out", |a| a["pages_left_out"] = json!([0])),
            ("a page past the last left out", |a| {
                a["pages_left_out"] = json!([4])
            }),
            // Decision #146: the cut outcomes, and the guard's own bounds.
            ("a page cut that the server did not cut", |a| {
                a["pages"][0]["outcome"] = json!("truncated")
            }),
            ("a loop the server did not cut", |a| {
                a["pages"][0]["outcome"] = json!("loop")
            }),
            ("a page the server cut read whole", |a| {
                a["pages"][0]["finish_reason"] = json!("length")
            }),
            ("a cut page with no block", |a| {
                a["pages"][0]["finish_reason"] = json!("length");
                a["pages"][0]["outcome"] = json!("truncated");
                a["pages"][0]["blocks"] = json!([]);
            }),
            ("a guard on a block of no box", |a| {
                a["pages"][0]["blocks"][1]
                    .as_object_mut()
                    .unwrap()
                    .remove("grid");
            }),
            ("an agreement over 1,000", |a| {
                a["pages"][0]["blocks"][1]["guard"]["agreement_permille"] = json!(1001);
            }),
            ("more lines bound than the block has", |a| {
                a["pages"][0]["blocks"][1]["guard"]["bound"] = json!(2);
            }),
            ("a line flagged past the last", |a| {
                a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                    json!([{"line": 2, "mark": "contradicted", "value": "12"}]);
            }),
            ("a line 0 flagged", |a| {
                a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                    json!([{"line": 0, "mark": "contradicted", "value": "12"}]);
            }),
            ("a line flagged twice", |a| {
                a["pages"][0]["blocks"][1]["guard"]["lines"] = json!(3);
                a["pages"][0]["blocks"][1]["guard"]["flagged"] = json!([
                    {"line": 1, "mark": "contradicted", "value": "12"},
                    {"line": 1, "mark": "unsupported"}
                ]);
            }),
            ("lines flagged out of order", |a| {
                a["pages"][0]["blocks"][1]["guard"]["lines"] = json!(3);
                a["pages"][0]["blocks"][1]["guard"]["flagged"] = json!([
                    {"line": 2, "mark": "unsupported"},
                    {"line": 1, "mark": "contradicted", "value": "12"}
                ]);
            }),
            ("a block with a box and no guard", |a| {
                a["pages"][0]["blocks"][1]
                    .as_object_mut()
                    .unwrap()
                    .remove("guard");
            }),
            ("a contradicted line naming no value", |a| {
                a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                    json!([{"line": 1, "mark": "contradicted"}]);
            }),
            ("an estimate naming no value", |a| {
                a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                    json!([{"line": 1, "mark": "estimate"}]);
            }),
            ("an unsupported line naming a value", |a| {
                a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                    json!([{"line": 1, "mark": "unsupported", "value": "12"}]);
            }),
            ("more lines bound and flagged than the block has", |a| {
                a["pages"][0]["blocks"][1]["guard"]["bound"] = json!(1);
                a["pages"][0]["blocks"][1]["guard"]["flagged"] =
                    json!([{"line": 1, "mark": "contradicted", "value": "12"}]);
            }),
        ];
        for (why, edit) in cases {
            assert!(read(edit).is_err(), "{why} was read");
        }
    }

    fn blocks(reply: &str) -> Vec<Read> {
        blocks_of(reply).expect("within the block budget")
    }

    fn read(label: Option<&str>, continues: bool, grid: Option<[u32; 4]>, text: &str) -> Read {
        Read {
            label: label.map(Into::into),
            continues,
            grid,
            text: text.into(),
        }
    }

    #[test]
    fn a_reply_reads_into_its_blocks_as_parsebench_parses_them() {
        assert_eq!(
            blocks(
                "  stray words\n![Text+]( 1 , 2 ,3,4 )\t continued text \n\n\
                 ![table](1200,90,10,5)<table></table>\n\n![chart](0,0,1000,1000)"
            ),
            [
                read(None, false, None, "stray words"),
                read(Some("text"), true, Some([1, 2, 3, 4]), "continued text"),
                read(
                    Some("table"),
                    false,
                    Some([10, 5, 1000, 90]),
                    "<table></table>"
                ),
                read(Some("chart"), false, Some([0, 0, 1000, 1000]), ""),
            ]
        );
        // A malformed marker is text; a marker inside a body splits it, as `finditer` does.
        assert_eq!(
            blocks(
                "![text](1,2,3) a ![](1,2,3,4) b ![x1](1,2,3,4) ![t](,1,2,3) c![image](5,6,7,8)d"
            ),
            [
                read(
                    None,
                    false,
                    None,
                    "![text](1,2,3) a ![](1,2,3,4) b ![x1](1,2,3,4) ![t](,1,2,3) c"
                ),
                read(Some("image"), false, Some([5, 6, 7, 8]), "d"),
            ]
        );
        // A `+` inside a label is part of it; only a last one continues.
        assert_eq!(
            blocks("![te+xt](1,2,3,4) y"),
            [read(Some("te+xt"), false, Some([1, 2, 3, 4]), "y")]
        );
        // Digits past the grid, however many, are its edge.
        assert_eq!(
            blocks("![text](99999999999999999999,0,1,2) x"),
            [read(Some("text"), false, Some([1, 0, 1000, 2]), "x")]
        );
        assert!(blocks("").is_empty());
        assert!(blocks(" \n ").is_empty());
    }

    #[test]
    fn a_reply_with_no_marker_reads_as_no_blocks() {
        let mut v = file();
        v["pages"][0]["reply"] = json!("Here is the text of the page, as a chat model writes it.");
        let repr = attach_file(&v).unwrap();
        let page = &repr.payload().assist.as_ref().unwrap().pages[0];
        assert_eq!(page.outcome, AssistOutcome::NoBlocks);
        assert_eq!(page.blocks.len(), 1);
        assert_eq!(page.blocks[0].label, None);
        // And an empty one is no blocks with none kept.
        v["pages"][0]["reply"] = json!("");
        let repr = attach_file(&v).unwrap();
        let page = &repr.payload().assist.as_ref().unwrap().pages[0];
        assert_eq!(page.outcome, AssistOutcome::NoBlocks);
        assert!(page.blocks.is_empty());
    }

    // The guard (decision #146). Pages here are 1,000 x 1,000 points, so a grid unit is a point.

    const FRAME: [i64; 4] = [0, 0, 100_000, 100_000];

    /// The guard on a page with no engine table or block region.
    fn guard(
        label: Option<&str>,
        text: &str,
        grid: [u32; 4],
        frame: [i64; 4],
        ink: &[Ink],
    ) -> AssistGuard {
        guard_with(label, text, grid, frame, ink, &[])
    }

    fn ink(rect: QRect, text: &'static str) -> InkRun<'static> {
        InkRun {
            rect: Some(rect),
            text,
            legible: true,
        }
    }

    /// A run of spaces, which has no box.
    fn space() -> InkRun<'static> {
        InkRun {
            rect: None,
            text: " ",
            legible: true,
        }
    }

    /// A run one grid unit square, centred on grid point (x, y), as tenths of a unit, alone in
    /// its ink block.
    fn run_at(x10: i64, y10: i64, text: &'static str) -> Ink<'static> {
        let (x, y) = (x10 * 10, y10 * 10);
        vec![ink(
            QRect::new(x - 50, y - 50, x + 50, y + 50).unwrap(),
            text,
        )]
    }

    fn run(x: i64, y: i64, text: &'static str) -> Ink<'static> {
        run_at(x * 10, y * 10, text)
    }

    fn flag(line: u32, mark: AssistMark, value: Option<&str>) -> AssistFlag {
        AssistFlag {
            line,
            mark,
            value: value.map(Into::into),
        }
    }

    /// (lines, bound, flagged) of a guard.
    fn tally(g: &AssistGuard) -> (u32, u32, Vec<AssistFlag>) {
        (g.lines, g.bound, g.flagged.clone())
    }

    fn html(rows: &[&[&str]]) -> String {
        let row = |r: &[&str]| format!("<tr><td>{}</td></tr>", r.join("</td><td>"));
        format!(
            "<table>{}</table>",
            rows.iter().map(|r| row(r)).collect::<String>()
        )
    }

    /// A made-up ruled table as the engine reads it, a run per cell, rows at y = 110 to 170: one
    /// amount printed with a point and one split by a space.
    fn table_runs() -> Vec<Ink<'static>> {
        let rows = [
            ["Item", "Amount", "Code", "Date"],
            ["Alpha", "1,234", "A-113", "2024-03-01"],
            ["Beta", "5.678", "B-220", "2024-04-15"],
            ["Gamma", "9 012", "C-310", "2024-05-30"],
        ];
        let mut runs = Vec::new();
        for (y, row) in (110..).step_by(20).zip(rows) {
            for (x, text) in (100..).step_by(200).zip(row) {
                runs.push(run(x, y, text));
            }
        }
        runs
    }

    const TRUE_TABLE: [&[&str]; 4] = [
        &["Item", "Amount", "Code", "Date"],
        &["Alpha", "1,234", "A-113", "2024-03-01"],
        &["Beta", "5.678", "B-220", "2024-04-15"],
        &["Gamma", "9012", "C-310", "2024-05-30"],
    ];

    /// The model's reading of [`table_runs`], with four planted errors: a digit in an amount, an
    /// id and a date, and a comma for a point.
    const PLANTED_TABLE: &str = "<table>\
        <tr><th>Item</th><th>Amount</th><th>Code</th><th>Date</th></tr>\
        <tr><td>Alpha</td><td>1,243</td><td>A-118</td><td>2024-03-07</td></tr>\
        <tr><td>Beta</td><td>5,678</td><td>B-220</td><td>2024-04-15</td></tr>\
        <tr><td>Gamma</td><td>9012</td><td>C-310</td><td>2024-05-30</td></tr></table>";

    const TABLE_BOX: [u32; 4] = [80, 100, 800, 180];

    /// **A table's planted wrong numbers are contradicted, one line each, and the true ones
    /// bind**, the split `9 012` among them.
    #[test]
    fn a_tables_planted_wrong_numbers_are_contradicted_and_the_rest_bind() {
        let g = guard(
            Some("table"),
            PLANTED_TABLE,
            TABLE_BOX,
            FRAME,
            &table_runs(),
        );
        let c = AssistMark::Contradicted;
        assert_eq!(
            tally(&g),
            (
                16,
                12,
                vec![
                    flag(6, c, Some("1,243")),
                    flag(7, c, Some("118")),
                    flag(8, c, Some("2024-03-07")),
                    flag(10, c, Some("5,678")),
                ]
            )
        );
        assert_eq!(g.engine_words, 20);
        assert_eq!(g.agreement_permille, 800);
        let g = guard(
            Some("table"),
            &html(&TRUE_TABLE),
            TABLE_BOX,
            FRAME,
            &table_runs(),
        );
        assert_eq!(tally(&g), (16, 16, vec![]));
    }

    /// Whether the engine's `text` holds the first value of `model`, signs and symbols included.
    fn holds(engine: &str, model: &str) -> bool {
        Engine::new(engine).find(&values(model)[0], 0).is_some()
    }

    /// A value is found where it occurs with whitespace removed, starting and ending outside the
    /// engine's own values, however the occurrences overlap; a different separator is a different
    /// value.
    #[test]
    fn a_value_is_found_whole_with_whitespace_removed_and_never_inside_another() {
        for (engine, value) in [
            ("1,234", "1,234"),
            ("1 234", "1234"),
            ("total 1 234 5", "1234"),
            ("ISO 27001", "ISO27001"),
            ("1 2 < 0.9549 1.0538 4 > 1.0768", "0.9549"),
            ("1, 234", "1,234"),
            ("113 11", "11"),
            ("1 11", "11"),
        ] {
            assert!(holds(engine, value), "{engine:?} holds {value:?}");
        }
        for (engine, value) in [
            ("1,234", "1,243"),
            ("1.234", "1,234"),
            ("1,234", "1.234"),
            ("12345", "1234"),
            ("51234", "1234"),
            ("1.234", "234"),
            ("1,234", "1"),
            ("113", "11"),
            ("", "1"),
        ] {
            assert!(!holds(engine, value), "{engine:?} does not hold {value:?}");
        }
    }

    /// **A value's sign must agree, brackets reading as a minus, and so must a currency symbol or
    /// percent sign touching it wherever both sides show one.**
    #[test]
    fn a_value_reads_with_its_sign_brackets_currency_and_percent() {
        for (engine, value) in [
            ("(1,234)", "(1,234)"),
            ("(1,234)", "-1,234"),
            ("-5.2", "-5.2"),
            ("\u{2212}5.2", "-5.2"),
            ("5.2", "+5.2"),
            ("$1,234", "$1,234"),
            ("$1,234", "1,234"),
            ("1,234", "$1,234"),
            ("12%", "12%"),
            ("12%", "12"),
            ("($1,234)", "-$1,234"),
            ("1,234€", "€1,234"),
            ("3 - 5.2", "5.2"),
        ] {
            assert!(holds(engine, value), "{engine:?} holds {value:?}");
        }
        for (engine, value) in [
            ("(1,234)", "1,234"),
            ("1,234", "-1,234"),
            ("1,234", "(1,234)"),
            ("-5.2", "5.2"),
            ("-5.2", "+5.2"),
            ("$1,234", "€1,234"),
            ("12%", "-12%"),
            ("7.5%", "7.5\u{2030}"),
            ("980", "-980"),
        ] {
            assert!(!holds(engine, value), "{engine:?} does not hold {value:?}");
        }
        let v = values("(1,234) -5.2 $7 8% 9€ ($10) (11");
        let read: Vec<(&str, bool, Option<char>, Option<char>)> = v
            .iter()
            .map(|v| (v.token, v.negative, v.currency, v.percent))
            .collect();
        assert_eq!(
            read,
            [
                ("1,234", true, None, None),
                ("5.2", true, None, None),
                ("7", false, Some('$'), None),
                ("8", false, None, Some('%')),
                ("9", false, Some('€'), None),
                ("10", true, Some('$'), None),
                ("11", false, None, None),
            ]
        );
    }

    /// The prototype's value tokenizer, with any script's decimal digits; its tag removal; decision
    /// #123's words.
    #[test]
    fn values_words_and_tags_are_read_as_the_prototype_reads_them() {
        fn tokens(s: &str) -> Vec<&str> {
            values(s).into_iter().map(|v| v.token).collect()
        }
        assert_eq!(
            tokens("abc 1,234.5 A-113 2024-11- x.1 ab-1 1-ab a1b ab. 7 (12) 3/4: 1..2"),
            ["1,234.5", "113", "2024-11", "1", "1", "1-ab", "a1b", "7", "12", "3/4", "1..2"]
        );
        assert!(tokens("no digits here, none-at all").is_empty());
        assert_eq!(
            tokens("٣ ab٣ ² ١,٢٣٤ １,２４３ x𝟎"),
            ["٣", "ab٣", "١,٢٣٤", "１,２４３", "x𝟎"]
        );
        for c in ['0', '9', '٣', '۹', '०', '๙', '０', '９', '𝟎', '𝟿'] {
            assert!(is_digit(c), "{c:?}");
        }
        for c in [
            'a',
            '/',
            '²',
            '½',
            'Ⅻ',
            '①',
            '\u{2069}',
            '\u{FF1A}',
            '\u{1FBFA}',
        ] {
            assert!(!is_digit(c), "{c:?}");
        }
        assert_eq!(untagged("a<td>b</td>c"), "a b c");
        assert_eq!(untagged("a <> b"), "a <> b");
        assert_eq!(untagged("<<a>b"), " b");
        assert_eq!(untagged("a < b"), "a < b");
        assert_eq!(untagged("a<b<>c>d"), "a c>d");
        let w = |s: &str| words(s).into_iter().collect::<Vec<_>>();
        assert_eq!(
            w("The THE a 1,234 snake_case Über x2"),
            ["234", "snake_case", "the", "x2", "über"]
        );
    }

    /// **Digits of any script are values, compared as written**: a reply's full-width or
    /// Arabic-Indic digits are checked, and the page's ASCII ones do not bind them.
    #[test]
    fn digits_of_any_script_are_checked_as_written() {
        let model = html(&[
            &["Item", "Amount", "Code", "Date"],
            &["Alpha", "１,２３４", "A-113", "2024-03-01"],
            &["Beta", "٥.٦٧٨", "B-220", "2024-04-15"],
            &["Gamma", "9012", "C-310", "2024-05-30"],
        ]);
        let g = guard(Some("table"), &model, TABLE_BOX, FRAME, &table_runs());
        let c = AssistMark::Contradicted;
        assert_eq!(
            tally(&g),
            (
                16,
                14,
                vec![flag(6, c, Some("１,２３４")), flag(10, c, Some("٥.٦٧٨"))]
            )
        );
    }

    /// A table's or chart's lines are its cells, row-major, each with its row, and the text
    /// outside its cells — a caption, words after the table — line by line; a tag no `>` closes
    /// ends the table, so an unclosed tag repeated costs no more than its length.
    #[test]
    fn a_tables_lines_are_its_cells_and_the_text_outside_them() {
        let lines = |label, text| block_lines(Some(label), text);
        assert_eq!(
            lines(
                "table",
                "<table><caption>Revenue <b>2024</b></caption><thead><tr><th colspan=\"2\">A</th>\
                 </tr></thead><TBODY><TR><TD>1<br>2</TD><td/><td>3</tr> 99 <tr><td>4</table>\n\
                 Note: in 1,000s\n\n"
            ),
            [
                (None, "Revenue 2024".into()),
                (Some(1), "A".into()),
                (Some(2), "1 2".into()),
                (Some(2), "".into()),
                (Some(2), "3".into()),
                (None, "99".into()),
                (Some(3), "4".into()),
                (None, "Note: in 1,000s".into()),
            ]
        );
        assert_eq!(
            lines("table", "<table><tr><td>cut mid-cell"),
            [(Some(1), "cut mid-cell".into())]
        );
        assert_eq!(
            lines("chart", "Item 42\n\n  \nItem 43"),
            [(None, "Item 42".into()), (None, "Item 43".into())]
        );
        assert_eq!(
            lines("text", "<td>1</td>\n<b>x</b>"),
            [(None, "1".into()), (None, "x".into())]
        );
        // Linear, however many unclosed tags.
        let unclosed = "<td".repeat(200_000);
        assert_eq!(table_parts(&unclosed), [(None, "")]);
        assert!(block_lines(Some("table"), &unclosed).is_empty());
        // A table block with no cell is read by its text's lines.
        let g = guard(
            Some("table"),
            "Item 42\nItem 43",
            [0, 0, 400, 400],
            FRAME,
            &[run(100, 100, "Item 42")],
        );
        assert_eq!(
            tally(&g),
            (2, 1, vec![flag(2, AssistMark::Contradicted, Some("43"))])
        );
    }

    /// **A chart's values the page does not print are estimates, never dropped**, whether or not
    /// the engine read the region; the values it prints bind; its text lines are checked as any
    /// other block's, so an invented one is unsupported and a label the engine did not read is
    /// unchecked, never bound.
    #[test]
    fn a_charts_values_not_in_the_text_are_estimates_and_its_words_are_checked() {
        const CHART: &str = "<table><tr><th>Quarter</th><th>Sales</th></tr>\
            <tr><td>Q1</td><td>42</td></tr><tr><td>Q2</td><td>57</td></tr>\
            <tr><td>Q3</td><td>61</td></tr></table>\nRevenue tripled after the merger closed";
        let runs = [
            run(300, 300, "Sales by quarter"),
            run(200, 380, "42"),
            run(200, 480, "Q1"),
            run(300, 480, "Q2"),
            run(400, 480, "Q3"),
        ];
        let e = AssistMark::Estimate;
        let g = guard(Some("chart"), CHART, [150, 290, 450, 490], FRAME, &runs);
        assert_eq!(
            tally(&g),
            (
                9,
                6,
                vec![
                    flag(6, e, Some("57")),
                    flag(8, e, Some("61")),
                    flag(9, AssistMark::Disputed, None)
                ]
            )
        );
        // The same chart drawn as a picture: every value an estimate, none dropped, no line bound.
        let g = guard(Some("chart"), CHART, [150, 290, 450, 490], FRAME, &[]);
        assert_eq!(
            tally(&g),
            (
                9,
                0,
                [
                    (3, "Q1"),
                    (4, "42"),
                    (5, "Q2"),
                    (6, "57"),
                    (7, "Q3"),
                    (8, "61")
                ]
                .map(|(line, v)| flag(line, e, Some(v)))
                .to_vec()
            )
        );
    }

    /// **A chart's first row and first column name its series and categories, and are never
    /// unsupported**: a name the engine did not read is unchecked. Its other cells and the text
    /// outside them are checked as any other block's.
    #[test]
    fn a_charts_names_are_never_unsupported() {
        const CHART: &str = "Figure seven shows national averages across eleven world regions \
            between nineteen eighty and twenty two inclusive\n\
            <table><tr><th>Group</th><th>Pretax income share</th></tr>\
            <tr><td>Top tenth decile</td><td>42</td></tr>\
            <tr><td>Middle</td><td>not yet reported</td></tr></table>\n\
            Revenue tripled after merger";
        let runs = [
            run(
                300,
                300,
                "Figure seven shows national averages across eleven world regions between \
                 nineteen eighty and twenty two inclusive",
            ),
            run(200, 380, "42"),
        ];
        let g = guard(Some("chart"), CHART, [150, 290, 450, 490], FRAME, &runs);
        assert!(g.agreement_permille >= 500, "checked, not disputed");
        let u = AssistMark::Unsupported;
        assert_eq!(tally(&g), (8, 2, vec![flag(7, u, None), flag(8, u, None)]));
        // A table's header and first column are cells like any other.
        let g = guard(Some("table"), CHART, [150, 290, 450, 490], FRAME, &runs);
        let flagged: Vec<u32> = g.flagged.iter().map(|f| f.line).collect();
        assert_eq!(flagged, [3, 4, 7, 8]);
    }

    /// **A line of three words or more, fewer than half of them the engine's, is unsupported**;
    /// a line of words half of which are found binds; a shorter line none of whose words are
    /// found is unchecked; a line both unsupported and holding a value not found is contradicted.
    #[test]
    fn an_invented_line_is_unsupported() {
        let runs = [run(
            500,
            500,
            "The made-up paragraph says nothing of note here, 76.",
        )];
        let g = guard(
            Some("text"),
            "The made-up paragraph says nothing of note here.\n\
             Quarterly revenue rose sharply\n\
             Revenue rose\n\
             note here rose sharply\n\
             rose sharply here\n\
             Quarterly revenue rose 77",
            [100, 450, 900, 550],
            FRAME,
            &runs,
        );
        let u = AssistMark::Unsupported;
        assert_eq!(
            tally(&g),
            (
                6,
                2,
                vec![
                    flag(2, u, None),
                    flag(5, u, None),
                    flag(6, AssistMark::Contradicted, Some("77"))
                ]
            )
        );
    }

    /// **The engine read a region where its legible runs there hold a value or three words**, and
    /// then every line is checked, however few of the block's words it holds: a reply that is
    /// mostly wrong is checked as closely as one that is mostly right. Fewer, or runs the engine
    /// marks illegible, and every line is unchecked.
    #[test]
    fn a_region_is_read_by_what_the_engine_holds_there() {
        let at = |runs: &[Ink<'static>], block: &str| {
            guard(Some("text"), block, [100, 100, 200, 200], FRAME, runs)
        };
        let c = AssistMark::Contradicted;
        // Three words and a value of the engine's against a reply of mostly other words: checked,
        // and what fails is disputed, kept rather than dropped.
        let d = AssistMark::Disputed;
        let g = at(
            &[run(150, 150, "alpha beta gamma 76")],
            "alpha zeta eta theta iota kappa\n77",
        );
        assert!(g.agreement_permille < 500);
        assert_eq!(
            tally(&g),
            (2, 0, vec![flag(1, d, None), flag(2, d, Some("77"))])
        );
        // A value alone reads the region; so do three words alone.
        let g = at(&[run(150, 150, "76")], "alpha beta gamma\n76 77");
        assert_eq!(tally(&g).2, [flag(1, d, None), flag(2, d, Some("77"))]);
        // Text the engine holds that shares no word and no value with the block — a font that
        // maps its codes to other letters — is no reading of it.
        let g = at(
            &[run(150, 150, "Hfaa]V ZWV VeZcV R 76")],
            "alpha beta gamma\n77",
        );
        assert_eq!(tally(&g), (2, 0, vec![]));
        let g = at(&[run(150, 150, "alpha beta gamma")], "alpha beta gamma\n77");
        assert_eq!(tally(&g), (2, 1, vec![flag(2, c, Some("77"))]));
        // Two words and no value: not read, every line unchecked.
        let g = at(&[run(150, 150, "alpha beta")], "alpha beta gamma\n77");
        assert_eq!(tally(&g), (2, 0, vec![]));
        // Runs the engine marks illegible are no reading.
        let mut hidden = run(150, 150, "alpha beta gamma 76");
        hidden[0].legible = false;
        let g = at(&[hidden], "alpha beta gamma\n77");
        assert_eq!((g.engine_words, tally(&g)), (0, (2, 0, vec![])));
        // A numbers-only table the model reads twelve cells of sixteen wrong: all twelve dropped.
        let truth = [
            "1,204", "3,517", "8,842", "6,031", "2,975", "4,410", "7,123", "5,688", "9,306",
            "1,847", "2,269", "3,794", "6,152", "8,437", "4,903", "7,560",
        ];
        let mut cells: Vec<Ink<'static>> = Vec::new();
        for (k, t) in truth.iter().enumerate() {
            let k = i64::try_from(k).unwrap();
            cells.push(run(100 + 200 * (k % 4), 110 + 20 * (k / 4), t));
        }
        let wrong: Vec<String> = truth
            .iter()
            .enumerate()
            .map(|(k, t)| {
                let last = (t.as_bytes()[t.len() - 1] - b'0' + 1) % 10;
                if k < 12 {
                    format!("{}{last}", &t[..t.len() - 1])
                } else {
                    t.to_string()
                }
            })
            .collect();
        let rows: Vec<Vec<&str>> = wrong
            .chunks(4)
            .map(|r| r.iter().map(String::as_str).collect())
            .collect();
        let rows: Vec<&[&str]> = rows.iter().map(Vec::as_slice).collect();
        let g = guard(Some("table"), &html(&rows), TABLE_BOX, FRAME, &cells);
        assert!(g.agreement_permille < 500);
        assert_eq!((g.lines, g.bound, g.flagged.len()), (16, 4, 12));
        assert!(g.flagged.iter().all(|f| f.mark == d));
    }

    /// A run holding U+FFFD, or one the engine found hidden, is no evidence.
    #[test]
    fn a_run_with_a_replacement_character_or_a_finding_is_not_legible() {
        let mut ids = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let page = ids.next(IdKind::Page).unwrap();
        let mut node =
            |text: &str, findings: Vec<crate::representation::TextFinding>| crate::Node {
                id: ids.next(IdKind::Span).unwrap(),
                kind: crate::NodeKind::TextRun,
                parent: page.clone(),
                ordinal: 1,
                text: text.into(),
                native_locator: crate::representation::NativeLocator::Pdf(
                    crate::representation::PdfLocator {
                        page: 1,
                        origin_x: 7200,
                        origin_y: 7200,
                        advance: Some(1000),
                    },
                ),
                structural_locator: None,
                derivation: DerivationClass::Extracted,
                attributes: crate::representation::NodeAttributes::TextRun(
                    crate::representation::TextRunAttributes {
                        read_beside: None,
                        char_codes: vec![0x31],
                        scalar_code_mismatch: false,
                        synthesized: Vec::new(),
                        findings,
                        font_id: "F1".into(),
                        font_size: 1000,
                        region: None,
                        block: None,
                        layout_unit: None,
                        inferred_heading: false,
                        inferred_heading_level: None,
                        bold: false,
                        italic: false,
                        script: None,
                        strikeout: false,
                        underline: false,
                        furniture: None,
                        reading: None,
                    },
                ),
            };
        use crate::representation::TextFinding as F;
        assert!(legible(&node("1,234", Vec::new())));
        assert!(!legible(&node("1,2\u{FFFD}4", Vec::new())));
        for finding in [
            F::InvisibleRenderMode,
            F::OffPage,
            F::TinyText,
            F::HiddenLayer,
            F::FillMatchesBackground,
        ] {
            assert!(!legible(&node("1,234", vec![finding])), "{finding:?}");
        }
    }

    /// **A table row's values are found on one engine line, in the row's order**: swapped cells,
    /// a row shifted by a column, a wrong value printed just outside the table and a box over
    /// the whole page bind no wrong cell. Each is disputed, the region holding it off its row; a
    /// value the region does not hold is contradicted.
    #[test]
    fn a_rows_values_are_found_on_one_line_in_order() {
        let (c, d) = (AssistMark::Contradicted, AssistMark::Disputed);
        let check = |rows: &[&[&str]], grid: [u32; 4], runs: &[Ink<'static>]| {
            tally(&guard(Some("table"), &html(rows), grid, FRAME, runs))
        };
        let mut swapped = TRUE_TABLE;
        swapped[1] = &["Alpha", "9012", "A-113", "2024-03-01"];
        swapped[3] = &["Gamma", "1,234", "C-310", "2024-05-30"];
        assert_eq!(
            check(&swapped, TABLE_BOX, &table_runs()),
            (
                16,
                14,
                vec![flag(6, d, Some("9012")), flag(14, d, Some("1,234"))]
            )
        );
        let mut shifted = TRUE_TABLE;
        shifted[1] = &["Alpha", "A-113", "2024-03-01", "1,234"];
        assert_eq!(
            check(&shifted, TABLE_BOX, &table_runs()),
            (16, 15, vec![flag(8, d, Some("1,234"))])
        );
        // `1,243` printed on its own line inside the box's slack, and elsewhere on the page.
        let mut wrong = TRUE_TABLE;
        wrong[1] = &["Alpha", "1,243", "A-113", "2024-03-01"];
        let mut runs = table_runs();
        runs.push(run(500, 186, "1,243"));
        runs.push(run(100, 900, "Footnote 1,243 units"));
        assert_eq!(
            check(&wrong, TABLE_BOX, &runs),
            (16, 15, vec![flag(6, d, Some("1,243"))])
        );
        assert_eq!(
            check(&wrong, [0, 0, 1000, 1000], &runs),
            (16, 15, vec![flag(6, d, Some("1,243"))]),
            "over the whole page, each row binds on its line"
        );
        // A row's repeated values, in order.
        let repeated: [&[&str]; 1] = [&["1.000", "2.540", "1.000"]];
        let runs = [run(100, 110, "1.000 1.000 2.540 1.000")];
        assert_eq!(check(&repeated, [80, 100, 800, 120], &runs), (3, 3, vec![]));
        // Cells the engine read as one block, apart on their line, are cells; so is a gap of half a
        // line's height, while glyphs that touch make one value.
        let block = vec![
            ink(QRect::new(9400, 13200, 10900, 14300).unwrap(), "MN"),
            ink(QRect::new(15300, 13200, 16500, 14300).unwrap(), "35"),
            ink(QRect::new(17050, 13200, 17600, 14300).unwrap(), "1"),
            ink(QRect::new(17600, 13200, 18200, 14300).unwrap(), "3"),
            ink(QRect::new(18200, 13200, 18800, 14300).unwrap(), "6"),
        ];
        let row: [&[&str]; 1] = [&["MN", "35", "136"]];
        assert_eq!(check(&row, [80, 120, 200, 150], &[block]), (3, 3, vec![]));
        // The line holding most of the row, not the first holding any of it.
        let row: [&[&str]; 1] = [&["1.000", "3.000", "4.000"]];
        let runs = [
            run(100, 110, "1.000 2.000"),
            run(100, 130, "1.000 3.000 4.000"),
        ];
        assert_eq!(check(&row, [80, 100, 800, 140], &runs), (3, 3, vec![]));
        // Two cells of the page's are never one value of the model's: `12` and `34` do not make
        // `1234`, while a split `5 678` inside one run does make `5678`.
        let rows: [&[&str]; 2] = [
            &["Rank", "Score", "Year", "Ref"],
            &["1234", "", "2024", "5678"],
        ];
        let runs = [
            run(100, 110, "Rank"),
            run(200, 110, "Score"),
            run(300, 110, "Year"),
            run(400, 110, "Ref"),
            run(100, 130, "12"),
            run(200, 130, "34"),
            run(300, 130, "20"),
            run(400, 130, "24"),
            run(500, 130, "5 678"),
        ];
        assert_eq!(
            check(&rows, [80, 100, 600, 140], &runs),
            (
                8,
                5,
                vec![flag(5, c, Some("1234")), flag(7, c, Some("2024"))]
            )
        );
        // Lines matching a row's values equally: the one holding the row's words.
        let rows: [&[&str]; 2] = [&["Alpha", "$1,234", "-5.2"], &["Beta", "1,234", "980"]];
        let runs = [
            run(100, 110, "Alpha"),
            run(200, 110, "$1,234"),
            run(300, 110, "-5.2"),
            run(100, 130, "Beta"),
            run(200, 130, "(1,234)"),
            run(300, 130, "980"),
        ];
        assert_eq!(
            check(&rows, [80, 100, 400, 140], &runs),
            (6, 5, vec![flag(5, d, Some("1,234"))])
        );
    }

    /// **A table cell whose values the region holds, one not on its row's lines, is disputed and
    /// kept**: a total the engine reads below its table, a date the page prints once for rows the
    /// model repeats it in. A cell holding a value the region does not hold is contradicted.
    #[test]
    fn a_value_the_region_holds_off_its_row_is_disputed() {
        let (c, d) = (AssistMark::Contradicted, AssistMark::Disputed);
        let mut runs = table_runs();
        runs.push(run(700, 186, "4,321"));
        let mut model = TRUE_TABLE;
        model[1] = &["Alpha", "1,234", "A-113", "2024-03-01 4,321"];
        model[2] = &["Beta", "5.678", "B-220", "2024-03-01"];
        model[3] = &["Gamma", "9012", "C-310", "1,234 8,888"];
        let g = guard(Some("table"), &html(&model), TABLE_BOX, FRAME, &runs);
        assert_eq!(
            tally(&g),
            (
                16,
                13,
                vec![
                    flag(8, d, Some("4,321")),
                    flag(12, d, Some("2024-03-01")),
                    flag(16, c, Some("1,234")),
                ]
            )
        );
    }

    /// **A box over half the page binds no line by itself**: a line whose values are found
    /// somewhere on the page is unchecked, not bound, while a value found nowhere is contradicted.
    #[test]
    fn a_box_over_half_the_page_binds_nothing_by_itself() {
        let runs = [run(500, 500, "Total 42 and more words")];
        let at = |grid| {
            tally(&guard(
                Some("text"),
                "Total 42\nTotal 43\nmore words",
                grid,
                FRAME,
                &runs,
            ))
        };
        let c = AssistMark::Contradicted;
        assert_eq!(
            at([400, 400, 600, 600]),
            (3, 2, vec![flag(2, c, Some("43"))])
        );
        assert_eq!(at([0, 0, 1000, 1000]), (3, 0, vec![flag(2, c, Some("43"))]));
        assert_eq!(
            at([0, 0, 1000, 500]),
            (3, 2, vec![flag(2, c, Some("43"))]),
            "half binds"
        );
        assert_eq!(at([0, 0, 1000, 501]), (3, 0, vec![flag(2, c, Some("43"))]));
    }

    /// **A block's box turns with the page it is placed on** (decision #155): at every turn, the
    /// box [`upright`] gives, placed on the page as the record reads it and mapped back to the
    /// declared frame, is the box as placed on the picture the renderer drew.
    #[test]
    fn a_grid_box_turns_with_the_page() {
        let at = |grid: [u32; 4], frame: [i64; 4]| {
            let [x0, y0, x1, y1] = placed(grid, frame).map(|v| i64::try_from(v / 1000).unwrap());
            QRect::new(x0, y0, x1, y1).unwrap()
        };
        let grid = [100, 200, 300, 650];
        for turn in [0, 90, 180, 270] {
            let (width, height) = if turn % 180 == 0 {
                (30000, 40000)
            } else {
                (40000, 30000)
            };
            let page = PageRecord {
                id: NodeId::from_parts(IdKind::Page, 1),
                index: 1,
                width,
                height,
                rotation: 0,
                turn,
                visible: None,
                drawn_boxes: Vec::new(),
            };
            let read = at(upright(grid, turn), frame_of(&page));
            assert_eq!(
                page.to_declared(read),
                Some(at(grid, [0, 0, 30000, 40000])),
                "turn {turn}"
            );
        }
    }

    /// The runs bound to a block are those whose box centre lies in its box, mapped onto the
    /// visible box and widened by ten grid units on every side, edges included; on a page of any
    /// size.
    #[test]
    fn runs_bind_by_their_centre_inside_the_widened_box() {
        let at = |runs: &[Ink<'static>], frame: [i64; 4]| {
            guard(Some("text"), "alpha 42", [100, 100, 200, 200], frame, runs).engine_words
        };
        for (x10, y10) in [(900, 1500), (2100, 1500), (1500, 900), (1500, 2100)] {
            assert_eq!(
                at(&[run_at(x10, y10, "alpha 42")], FRAME),
                2,
                "({x10}, {y10})"
            );
        }
        for (x10, y10) in [(899, 1500), (2101, 1500), (1500, 899), (1500, 2101)] {
            assert_eq!(
                at(&[run_at(x10, y10, "alpha 42")], FRAME),
                0,
                "({x10}, {y10})"
            );
        }
        // A cropped page: the grid is on its visible box, 200 x 300 points from (50, 50).
        let cropped = [5000, 5000, 20000, 30000];
        let centre = || vec![ink(QRect::new(7950, 9450, 8050, 9550).unwrap(), "alpha 42")];
        assert_eq!(at(&[centre()], cropped), 2);
        assert_eq!(at(&[centre()], [0, 0, 20000, 30000]), 0);
        assert_eq!(at(&[centre()], [5000, 0, 20000, 30000]), 0);
        // A page 8e13 points square.
        let side = 8_000_000_000_000_000_i64;
        let middle = side / 2;
        let huge = vec![ink(
            QRect::new(middle - 50, middle - 50, middle + 50, middle + 50).unwrap(),
            "alpha 42",
        )];
        assert_eq!(
            guard(
                Some("text"),
                "alpha 42",
                [400, 400, 600, 600],
                [0, 0, side, side],
                &[huge]
            )
            .engine_words,
            2
        );
        let g = guard(
            Some("text"),
            "alpha 42",
            [100, 100, 200, 200],
            FRAME,
            &[run_at(900, 1500, "alpha 42")],
        );
        assert_eq!(tally(&g), (1, 1, vec![]));
    }

    /// A page read glyph by glyph: a block's runs inside the box are joined as the engine joins
    /// them, its runs of spaces (which have no box) kept, so its words are words; a block with no
    /// run inside the box adds nothing; and a run on the next line is a new line, never glued to
    /// the last.
    #[test]
    fn runs_join_as_the_engine_joins_them_line_by_line() {
        let glyph = |x: i64, t: &'static str| {
            let x = x * 100;
            ink(QRect::new(x - 50, 14950, x + 50, 15050).unwrap(), t)
        };
        let letters = |from: i64, word: &[&'static str]| -> Ink {
            word.iter().zip(from..).map(|(t, x)| glyph(x, t)).collect()
        };
        let mut line = letters(100, &["N", "e", "t"]);
        line.push(space());
        line.extend(letters(104, &["r", "e", "v", "e", "n", "u", "e"]));
        line.push(space());
        line.extend(letters(112, &["4", "2"]));
        // A glyph past the box's edge is left out.
        line.push(glyph(300, "x"));
        let stray = vec![InkRun {
            rect: None,
            text: "stray words here",
            legible: true,
        }];
        let g = guard(
            Some("text"),
            "Net revenue 42\nNet revenue 43",
            [90, 140, 200, 160],
            FRAME,
            &[line, stray],
        );
        assert_eq!(g.engine_words, 3, "net, revenue and 42");
        assert_eq!(g.agreement_permille, 750);
        assert_eq!(
            tally(&g),
            (2, 1, vec![flag(2, AssistMark::Contradicted, Some("43"))])
        );
        // One block wrapping to the next line with no space drawn between.
        let wrapped = vec![
            ink(
                QRect::new(10000, 10000, 30000, 11000).unwrap(),
                "rose to 1,234",
            ),
            ink(
                QRect::new(10000, 11200, 30000, 12200).unwrap(),
                "million dollars",
            ),
        ];
        let g = guard(
            Some("text"),
            "rose to 1,234\nmillion dollars",
            [90, 90, 310, 130],
            FRAME,
            &[wrapped],
        );
        assert_eq!(tally(&g), (2, 2, vec![]));
        // No value spans two lines: `Total 12` above `34` does not hold `1234` (review).
        let stacked = vec![
            ink(QRect::new(10000, 10000, 30000, 11000).unwrap(), "Total 12"),
            ink(QRect::new(10000, 11200, 30000, 12200).unwrap(), "34"),
        ];
        let g = guard(
            Some("text"),
            "Total 1234",
            [90, 90, 310, 130],
            FRAME,
            &[stacked],
        );
        assert_eq!(
            tally(&g),
            (1, 0, vec![flag(1, AssistMark::Contradicted, Some("1234"))])
        );
        // A line ending in a separator a value runs through goes on: `2007-` above `2023`.
        let range = vec![
            ink(
                QRect::new(10000, 10000, 30000, 11000).unwrap(),
                "over 2007-",
            ),
            ink(
                QRect::new(10000, 11200, 30000, 12200).unwrap(),
                "2023 it rose",
            ),
        ];
        let g = guard(
            Some("text"),
            "over 2007-2023 it rose",
            [90, 90, 310, 130],
            FRAME,
            &[range],
        );
        assert_eq!(tally(&g), (1, 1, vec![]));
        // A run of spaces joins a value the engine split inside one block.
        let split = vec![
            ink(QRect::new(10000, 10000, 10500, 11000).unwrap(), "1"),
            space(),
            ink(QRect::new(10700, 10000, 12000, 11000).unwrap(), "234"),
        ];
        let g = guard(
            Some("text"),
            "Total 1234",
            [90, 90, 130, 120],
            FRAME,
            &[split],
        );
        assert_eq!(tally(&g), (1, 1, vec![]));
        // An illegible run between two digits, however narrow, is a hole, not a join: `1` and `2`
        // are not `12`.
        let mut holed = vec![
            ink(QRect::new(10000, 10000, 10500, 11000).unwrap(), "1"),
            ink(QRect::new(10500, 10000, 10700, 11000).unwrap(), "\u{FFFD}"),
            ink(QRect::new(10700, 10000, 11200, 11000).unwrap(), "2"),
        ];
        holed[1].legible = false;
        let label = vec![ink(QRect::new(9000, 10000, 9800, 11000).unwrap(), "Total")];
        let g = guard(
            Some("text"),
            "Total 12",
            [90, 90, 130, 120],
            FRAME,
            &[label, holed],
        );
        assert_eq!(
            tally(&g),
            (1, 0, vec![flag(1, AssistMark::Contradicted, Some("12"))])
        );
    }

    /// The stored agreement: the share of the block's distinct words, in thousandths, rounded
    /// down; nought for a block of no words.
    #[test]
    fn the_agreement_is_rounded_down_and_nought_without_words() {
        let runs = [run(150, 150, "alpha beta 7")];
        let at = |text: &str| {
            guard(Some("text"), text, [100, 100, 200, 200], FRAME, &runs).agreement_permille
        };
        assert_eq!(at("alpha beta gamma"), 666);
        assert_eq!(at("— 7 —"), 0);
        assert_eq!(at(""), 0);
        // An empty cell holds nothing to bind.
        let g = guard(
            Some("table"),
            "<table><tr><td></td><td>alpha</td></tr></table>",
            [100, 100, 200, 200],
            FRAME,
            &[run(150, 150, "alpha beta gamma")],
        );
        assert_eq!(tally(&g), (2, 1, vec![]));
    }

    /// **Rule 4's loop**: four copies or more of one string of 1 to 400 characters, 64 or more in
    /// all, the last copy perhaps cut.
    #[test]
    fn a_loop_is_four_copies_of_up_to_400_characters_64_in_all() {
        let cell = "<td>1.000</td>\n"; // 15 characters
        assert!(loops(&format!("head {}", "0 ".repeat(32))));
        assert!(loops(&format!("{}<td>1.0", cell.repeat(5))));
        assert!(!loops(&cell.repeat(4)), "60 characters");
        assert!(
            !loops(&format!("{}x", "abcdefghijklmnop".repeat(4))),
            "a tail that breaks it"
        );
        assert!(loops(&"abcdefghijklmnop".repeat(4)), "64 characters");
        let long = |n: usize| {
            (0..n)
                .map(|i| char::from(b'a' + (i % 26) as u8))
                .collect::<String>()
                + "#"
        };
        assert!(!loops(&long(29).repeat(3)), "three copies, 90 characters");
        assert!(loops(&long(399).repeat(4)), "400 characters");
        assert!(!loops(&long(400).repeat(4)), "401 characters");
        let wide: String = (0..150)
            .map(|i| char::from_u32(0x4E00 + i).unwrap())
            .collect();
        assert!(loops(&wide.repeat(4)), "150 characters, 450 bytes");
        assert!(!loops("A made-up reply that ends."));
        assert!(!loops(""));
    }

    /// **Rule 4**: a reply the server cut loses its last block, as a `loop` or `truncated`, and a
    /// reply it did not cut is read whole, loop or not. The blocks stay in the record.
    #[test]
    fn a_capped_reply_is_a_loop_or_truncated() {
        let outcome = |finish: &str, last: &str| {
            let mut v = file();
            v["pages"][0]["finish_reason"] = json!(finish);
            v["pages"][0]["reply"] = json!(format!(
                "![title](1,2,3,4) # Heading\n\n![table](5,6,7,8) {last}"
            ));
            let repr = attach_file(&v).unwrap();
            let page = &repr.payload().assist.as_ref().unwrap().pages[0];
            assert_eq!(page.blocks.len(), 2);
            page.outcome
        };
        let looping = format!("<table>{}", "<tr><td>0</td></tr>\n".repeat(8));
        assert_eq!(outcome("length", &looping), AssistOutcome::Loop);
        assert_eq!(
            outcome("length", "<table><tr><td>12"),
            AssistOutcome::Truncated
        );
        assert_eq!(outcome("stop", &looping), AssistOutcome::Read);
        // A reply of no marker stays one of no blocks, cut or not.
        let mut v = file();
        v["pages"][0]["finish_reason"] = json!("length");
        v["pages"][0]["reply"] = json!("words and no marker");
        let repr = attach_file(&v).unwrap();
        assert_eq!(
            repr.payload().assist.as_ref().unwrap().pages[0].outcome,
            AssistOutcome::NoBlocks
        );
    }

    /// A block with a box carries the guard's marks; one with none carries no guard.
    #[test]
    fn a_block_with_a_box_carries_its_guard() {
        let mut v = file();
        v["pages"][0]["reply"] = json!("stray words\n![text](1,2,3,4) alpha 42");
        let repr = attach_file(&v).unwrap();
        let blocks = &repr.payload().assist.as_ref().unwrap().pages[0].blocks;
        assert_eq!(blocks[0].guard, None);
        assert_eq!(
            blocks[1].guard,
            Some(AssistGuard {
                engine_words: 0,
                agreement_permille: 0,
                lines: 1,
                bound: 0,
                flagged: Vec::new(),
            })
        );
    }

    /// A reply's HTML character references are decoded before the guard reads it: the five named
    /// ones LightOnOCR writes, and decimal and hexadecimal references. Anything else stays as written.
    #[test]
    fn a_replys_character_references_are_decoded_before_the_guard() {
        assert_eq!(
            decoded(
                "&lt;5 &gt; &amp; &quot;x&quot; &#39;a&apos; &#49;&#x32;&#X33; &amp;lt; &copy; \
                 AT&T; &#xD800; &#99999999999; &#; &#x; &#+1; &#x10FFFF; &"
            ),
            "<5 > & \"x\" 'a' 123 &lt; &copy; AT&T; &#xD800; &#99999999999; &#; &#x; &#+1; \u{10FFFF} &"
        );
        // A comma written as a reference is the value the page prints, and `&lt;` is no word.
        let runs = [run(150, 150, "< alpha beta 1,234")];
        let g = guard(
            Some("table"),
            "<table><tr><td>&lt; alpha</td><td>beta</td><td>1&#44;234</td></tr></table>",
            [100, 100, 200, 200],
            FRAME,
            &runs,
        );
        assert_eq!(g.agreement_permille, 1000);
        assert_eq!(tally(&g), (3, 3, vec![]));
    }

    /// **A table row's values may lie on the engine lines between its neighbours' lines**, read
    /// left to right: a header cell the page sets on two lines binds. A first or last row reaches
    /// one line past its own, and no further.
    #[test]
    fn a_rows_values_may_spread_over_the_lines_between_its_neighbours() {
        let rows: [&[&str]; 2] = [
            &[
                "in millions",
                "Classification",
                "February 2, 2025",
                "January 28, 2024",
            ],
            &["Total", "Other", "1,234", "5,678"],
        ];
        let runs = [
            run(300, 110, "February 2,"),
            run(400, 110, "January 28,"),
            run(100, 120, "in millions"),
            run(200, 120, "Classification"),
            run(300, 120, "2025"),
            run(400, 120, "2024"),
            run(100, 140, "Total"),
            run(200, 140, "Other"),
            run(300, 140, "1,234"),
            run(400, 140, "5,678"),
        ];
        let g = guard(
            Some("table"),
            &html(&rows),
            [80, 100, 500, 150],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g), (8, 8, vec![]));
        // Neighbouring rows that swap a value, each on its own line, are both disputed: neither
        // binds, and the region holds both values.
        let mut swapped = TRUE_TABLE;
        swapped[1] = &["Alpha", "5.678", "A-113", "2024-03-01"];
        swapped[2] = &["Beta", "1,234", "B-220", "2024-04-15"];
        let (c, d) = (AssistMark::Contradicted, AssistMark::Disputed);
        assert_eq!(
            tally(&guard(
                Some("table"),
                &html(&swapped),
                TABLE_BOX,
                FRAME,
                &table_runs()
            )),
            (
                16,
                14,
                vec![flag(6, d, Some("5.678")), flag(10, d, Some("1,234"))]
            )
        );
        // Neighbouring rows that swap every value, labels kept, are both contradicted (review):
        // each row's values lie on the other's line, which does not hold its label.
        let mut swapped = TRUE_TABLE;
        swapped[1] = &["Alpha", "5.678", "B-220", "2024-04-15"];
        swapped[2] = &["Beta", "1,234", "A-113", "2024-03-01"];
        assert_eq!(
            tally(&guard(
                Some("table"),
                &html(&swapped),
                TABLE_BOX,
                FRAME,
                &table_runs()
            )),
            (
                16,
                8,
                vec![
                    flag(6, c, Some("5.678")),
                    flag(7, c, Some("220")),
                    flag(8, c, Some("2024-04-15")),
                    flag(10, c, Some("1,234")),
                    flag(11, c, Some("113")),
                    flag(12, c, Some("2024-03-01")),
                ]
            )
        );
        // A label the page centres over the rows it spans lies on another row's line, and is no
        // swap: that row's own label is not on this one's.
        let centred: [&[&str]; 2] = [&["PaLM", "8B", "25.6"], &["", "62B", "59.5"]];
        let runs = [
            run(200, 110, "8B"),
            run(300, 110, "25.6"),
            run(100, 130, "PaLM"),
            run(200, 130, "62B"),
            run(300, 130, "59.5"),
        ];
        let g = guard(
            Some("table"),
            &html(&centred),
            [80, 100, 400, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g), (6, 4, vec![]));
        // A label is the words of letters one row alone holds: a unit both rows show hides no swap.
        let units: [&[&str]; 2] = [&["Alpha", "20 kg"], &["Beta", "10 kg"]];
        let runs = [
            run(100, 110, "Alpha"),
            run(300, 110, "10 kg"),
            run(100, 130, "Beta"),
            run(300, 130, "20 kg"),
        ];
        let g = guard(
            Some("table"),
            &html(&units),
            [80, 100, 400, 140],
            FRAME,
            &runs,
        );
        assert_eq!(
            tally(&g),
            (4, 0, vec![flag(2, c, Some("20")), flag(4, c, Some("10"))])
        );
        // A row whose label is on its own line is in place, though the page prints the label on
        // another row's line too.
        let both: [&[&str]; 2] = [&["Alpha", "10"], &["Beta", "20"]];
        let runs = [
            run(100, 110, "Alpha Beta"),
            run(300, 110, "10"),
            run(100, 130, "Alpha"),
            run(300, 130, "20"),
        ];
        let g = guard(
            Some("table"),
            &html(&both),
            [80, 100, 400, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g), (4, 3, vec![]));
        // The last row's wrong value printed on the line below binds; two lines below, it does
        // not, and is disputed.
        let mut wrong = TRUE_TABLE;
        wrong[3] = &["Gamma", "9,999", "C-310", "2024-05-30"];
        let mut runs = table_runs();
        runs.push(run(300, 186, "9,999"));
        let at = |runs: &[Ink<'static>]| {
            tally(&guard(
                Some("table"),
                &html(&wrong),
                [80, 100, 800, 200],
                FRAME,
                runs,
            ))
        };
        assert_eq!(
            at(&runs),
            (16, 16, vec![]),
            "one line below the last row: accepted"
        );
        let mut runs = table_runs();
        runs.push(run(100, 186, "Note"));
        runs.push(run(300, 200, "9,999"));
        assert_eq!(at(&runs), (16, 15, vec![flag(14, d, Some("9,999"))]));
    }

    /// **A box the model drew too small binds against the engine's table it overlaps**, or a block
    /// region of no more than half the page; never against the rest of the page.
    #[test]
    fn a_small_box_binds_against_the_engine_table_it_overlaps() {
        let model = html(&TRUE_TABLE);
        let small = [80, 100, 300, 180];
        let table = QRect::new(8000, 10000, 80000, 18000).unwrap();
        let at = |regions: &[(QRect, bool)]| {
            tally(&guard_with(
                Some("table"),
                &model,
                small,
                FRAME,
                &table_runs(),
                regions,
            ))
        };
        assert_eq!(at(&[(table, true)]), (16, 16, vec![]));
        let (lines, bound, flagged) = at(&[]);
        assert_eq!((lines, bound), (16, 8));
        assert!(
            flagged.iter().all(|f| f.mark == AssistMark::Disputed),
            "a box this small shares fewer than half its words with the engine"
        );
        // An engine table elsewhere on the page is no part of the box, though it prints the values.
        let elsewhere = QRect::new(8000, 50000, 80000, 60000).unwrap();
        let mut copied = table_runs();
        for (y, row) in (510..).step_by(20).zip(TRUE_TABLE) {
            for (x, text) in (300..).step_by(200).zip(&row[1..]) {
                copied.push(run(x, y, text));
            }
        }
        let g = guard_with(
            Some("table"),
            &model,
            small,
            FRAME,
            &copied,
            &[(elsewhere, true)],
        );
        assert_eq!(g.bound, 8);
        // Nor is one level with the table but beside it.
        let beside = QRect::new(82000, 10000, 99000, 60000).unwrap();
        let g = guard_with(
            Some("table"),
            &model,
            small,
            FRAME,
            &copied,
            &[(beside, true), (elsewhere, true)],
        );
        assert_eq!(g.bound, 8);
        // A block region binds where it is no more than half the page, and not where it is more.
        assert_eq!(at(&[(table, false)]), (16, 16, vec![]));
        let most = QRect::new(0, 10000, 100_000, 60100).unwrap();
        assert_eq!(at(&[(most, false)]).1, 8);
        let half = QRect::new(0, 10000, 100_000, 60000).unwrap();
        assert_eq!(at(&[(half, false)]), (16, 16, vec![]));
        // An engine table binds at any size: its rows still bind each on its own lines.
        let large = QRect::new(0, 10000, 100_000, 70000).unwrap();
        assert_eq!(at(&[(large, true)]), (16, 16, vec![]));
        // The model's wrong value printed in the engine's table but off its row does not bind: it
        // is disputed.
        let mut wrong = TRUE_TABLE;
        wrong[1] = &["Alpha", "5.678", "A-113", "2024-03-01"];
        let g = guard_with(
            Some("table"),
            &html(&wrong),
            small,
            FRAME,
            &table_runs(),
            &[(table, true)],
        );
        assert_eq!(tally(&g).2, [flag(6, AssistMark::Disputed, Some("5.678"))]);
    }

    /// A block whose reading order jumps back left starts a new piece there: `0.` and `92` drawn
    /// left of the `0.77` before them are `0.92`, not `0.770.92`.
    #[test]
    fn a_run_left_of_its_piece_starts_another() {
        let line = |x0, x1, t| ink(QRect::new(x0, 45357, x1, 46137).unwrap(), t);
        let block = vec![
            line(3570, 5130, "1989"),
            line(14172, 15541, "0.77 "),
            line(17472, 18450, "0.7"),
            line(18455, 18846, "7"),
            line(7566, 8152, "0."),
            line(8160, 8941, "92 "),
            line(10873, 12239, "0.92"),
        ];
        let row: [&[&str]; 1] = [&["1989", "0.92", "0.92", "0.77", "0.77"]];
        let g = guard(
            Some("table"),
            &html(&row),
            [30, 450, 200, 465],
            FRAME,
            &[block],
        );
        assert_eq!(tally(&g), (5, 5, vec![]));
        // Right-to-left glyphs, each left of the last, still make one piece.
        let rtl = vec![
            line(12000, 12500, "1"),
            line(11500, 12000, "2"),
            line(11000, 11500, "3"),
        ];
        let g = guard(
            Some("text"),
            "Total 123",
            [100, 450, 130, 465],
            FRAME,
            &[rtl],
        );
        assert_eq!(tally(&g), (1, 1, vec![]));
    }

    /// Brackets, a currency symbol or a percent sign the engine drew apart from a value, or the
    /// model wrote with a space, still go with it; a minus only touching it; an en dash is a minus.
    #[test]
    fn brackets_and_symbols_reach_across_a_gap_and_a_minus_does_not() {
        for (engine, value) in [
            ("(\u{1F}15,610\u{1F})", "(15,610)"),
            ("$\u{1F}(\u{1F}15,610\u{1F})", "$ (15,610)"),
            ("(2)", "( 2 )"),
            ("$\u{1F}10,631", "$ 10,631"),
            ("12 %", "12%"),
            ("\u{2013}0.8", "\u{2212}0.8"),
            ("3 - 5.2", "5.2"),
        ] {
            assert!(holds(engine, value), "{engine:?} holds {value:?}");
        }
        for (engine, value) in [
            ("(\u{1F}15,610\u{1F})", "15,610"),
            ("$\u{1F}10,631", "€10,631"),
            ("\u{2013}0.8", "0.8"),
            ("3 - 5.2", "-5.2"),
        ] {
            assert!(!holds(engine, value), "{engine:?} does not hold {value:?}");
        }
    }

    /// **A table cell set on two lines, its second on the next row's line, binds to its own row**
    /// where it lies in its column, between the row's values found either side of it, and shows on
    /// its own line too; a single value the next row prints does not, nor one the model adds past
    /// the last one found: each is disputed, the region holding it.
    #[test]
    fn a_stacked_cell_binds_to_its_row_in_its_column() {
        let runs = [
            run(100, 110, "Alpha"),
            run(300, 110, "1,234"),
            run(500, 110, "A-113"),
            run(100, 130, "Beta"),
            run(300, 130, "2,345"),
            run(500, 130, "B-220"),
            run(700, 130, "5.678"),
        ];
        let rows: [&[&str]; 2] = [&["Alpha", "1,234 2,345", "A-113"], &["Beta", "", "B-220"]];
        let g = guard(
            Some("table"),
            &html(&rows),
            [80, 100, 800, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g), (6, 5, vec![]));
        let rows: [&[&str]; 2] = [
            &["Alpha", "1,234", "A-113", "5.678"],
            &["Beta", "", "B-220", ""],
        ];
        let g = guard(
            Some("table"),
            &html(&rows),
            [80, 100, 800, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g).2, [flag(4, AssistMark::Disputed, Some("5.678"))]);
        // Out of its column, or past the row's last value found, the next row's line lends nothing,
        // though the cell shows on its own line.
        let runs = [
            run(100, 110, "Alpha"),
            run(300, 110, "1,234"),
            run(500, 110, "A-113"),
            run(700, 110, "Total"),
            run(100, 130, "Beta"),
            run(500, 130, "B-220"),
            run(700, 130, "5.678"),
        ];
        let rows: [&[&str]; 2] = [&["Alpha", "1,234 5.678", "A-113"], &["Beta", "", "B-220"]];
        let g = guard(
            Some("table"),
            &html(&rows),
            [80, 100, 800, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g).2, [flag(2, AssistMark::Disputed, Some("5.678"))]);
        let rows: [&[&str]; 2] = [
            &["Alpha", "1,234", "A-113", "Total 5.678"],
            &["Beta", "", "B-220", ""],
        ];
        let g = guard(
            Some("table"),
            &html(&rows),
            [80, 100, 800, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g).2, [flag(4, AssistMark::Disputed, Some("5.678"))]);
        // A single wrong value does not borrow the next row's, printed in its column.
        let rows: [&[&str]; 2] = [&["Alpha", "5,678", "A-113"], &["Beta", "5,678", "B-220"]];
        let runs = [
            run(100, 110, "Alpha"),
            run(300, 110, "1,234"),
            run(500, 110, "A-113"),
            run(100, 130, "Beta"),
            run(300, 130, "5,678"),
            run(500, 130, "B-220"),
        ];
        let g = guard(
            Some("table"),
            &html(&rows),
            [80, 100, 800, 140],
            FRAME,
            &runs,
        );
        assert_eq!(tally(&g).2, [flag(2, AssistMark::Disputed, Some("5,678"))]);
    }

    /// Brackets the engine read mirrored, as a right-to-left line draws them, touching the value:
    /// `)2(` is `(2)`; apart, as in `(1) 2 (3)`, they are two other values' brackets.
    #[test]
    fn mirrored_brackets_touching_a_value_read_as_brackets() {
        assert!(holds(")2(", "(2)"));
        assert!(!holds(")2(", "2"));
        assert!(holds("(1) 2 (3)", "2"));
        assert!(!holds("(1) 2 (3)", "(2)"));
    }

    /// The guard on one line of text whose engine reading is `engine`, one run.
    fn line_against(engine: &str, model: &str) -> (u32, u32, Vec<AssistFlag>) {
        let runs = [vec![InkRun {
            rect: Some(QRect::new(10000, 10000, 30000, 11000).unwrap()),
            text: engine,
            legible: true,
        }]];
        tally(&guard(
            Some("text"),
            model,
            [90, 90, 310, 120],
            FRAME,
            &runs,
        ))
    }

    /// **A dash of any width between two letters or digits reads as `-`, on either side**: the
    /// engine's `99‐R98CR` (U+2010) holds the model's `99-R98CR`, and the other way round, and an
    /// en dash one side wrote holds the other's em dash. A dash touching a value on one side only is
    /// left as written.
    #[test]
    fn a_dash_between_letters_or_digits_is_a_hyphen() {
        for dash in [
            '\u{2010}', '\u{2011}', '\u{2012}', '\u{2013}', '\u{2014}', '\u{2015}',
        ] {
            let page = format!("Form 99{dash}R98CR filed");
            assert_eq!(line_against(&page, "Form 99-R98CR filed"), (1, 1, vec![]));
            assert_eq!(line_against("Form 99-R98CR filed", &page), (1, 1, vec![]));
        }
        let page = "Part 9\u{2014}Internal 4\u{2013}G";
        assert_eq!(
            line_against(page, "Part 9\u{2013}Internal 4\u{2014}G"),
            (1, 1, vec![])
        );
        let page = "Years 2007\u{2212}2023 filed";
        assert_eq!(line_against(page, "Years 2007-2023 filed"), (1, 1, vec![]));
        // Touching one side only, a dash is as written: U+2212 is a minus, U+2010 is not.
        assert_eq!(
            line_against("Loss \u{2212}5 units", "Loss -5 units"),
            (1, 1, vec![])
        );
        assert_eq!(
            line_against("Loss \u{2010}5 units", "Loss 5 units"),
            (1, 1, vec![])
        );
    }

    /// **A LaTeX command in the model's text is no part of a value**: a backslash and the
    /// lower-case letters after it, so `$\pm20V$` holds `20V`. A path's `\O365` keeps its value.
    #[test]
    fn a_latex_command_is_no_part_of_a_value() {
        let tokens = |s| values(s).iter().map(|v| v.token).collect::<Vec<_>>();
        assert_eq!(
            tokens("$V_{GS}=\\pm20V$, $\\pm100$, 3\\times10^{6}"),
            ["20V", "100", "3", "10", "6"]
        );
        assert_eq!(tokens("C:\\Users\\O365-[Sec]\\MSS194"), ["O365", "MSS194"]);
        assert_eq!(
            line_against(
                "Gate voltage \u{B1}20V rated",
                "Gate voltage $\\pm20V$ rated"
            ),
            (1, 1, vec![])
        );
    }

    /// **Digits the page letter-spaces with a space read between them are one piece**: a run
    /// after a piece ending in a space joins it within less than half its height, so the engine's
    /// `19 ,5 2 1` holds `19,521`. Without the space, or half a height apart, they are two.
    #[test]
    fn letter_spaced_digits_with_spaces_between_are_one_piece() {
        // The runs as the engine reads one page's cell, a run 771 high, after a label.
        let at = |x0, x1, text| ink(QRect::new(x0, 27352, x1, 28123).unwrap(), text);
        let check = |digits: &[(i64, i64, &'static str)], model: &str| {
            let mut runs = vec![at(26000, 27800, "Total")];
            runs.extend(digits.iter().map(|&(x0, x1, t)| at(x0, x1, t)));
            tally(&guard(
                Some("text"),
                model,
                [255, 270, 310, 285],
                FRAME,
                &[runs],
            ))
        };
        let spaced = [
            (28428, 28885, "19 "),
            (29100, 29155, ","),
            (29232, 29461, "5 "),
            (29664, 29893, "2 "),
            (30060, 30289, "1"),
        ];
        assert_eq!(check(&spaced, "Total 19,521"), (1, 1, vec![]));
        let c = AssistMark::Contradicted;
        // No space read between them: a quarter of the height parts them, as before.
        let unspaced = [(28428, 28885, "12"), (29100, 29329, "3")];
        assert_eq!(
            check(&unspaced, "Total 123"),
            (1, 0, vec![flag(1, c, Some("123"))])
        );
        // Half the height parts them, a space read or not.
        let apart = [(28428, 28885, "12 "), (29271, 29500, "3")];
        assert_eq!(
            check(&apart, "Total 123"),
            (1, 0, vec![flag(1, c, Some("123"))])
        );
    }

    /// **The trigger asks for a page where one of its six signals fires, and for no other**
    /// (design §6, decision #150). Page 1 is twenty made-up words and nothing else; pages 2 to 10
    /// are page 1 with one change each: a recognized run, its `scanned-page-not-recognized`, a
    /// figure, a table, a picture painted over a quarter of the page, U+FFFD in a run, a word
    /// fewer; a picture as large, half of it off the page, which asks nothing; a picture over a
    /// quarter of a crop; and page 1's words set on two lines, which asks nothing. Runs split a
    /// word as the engine's do, so the page's runs are read joined, and a space parts two lines.
    #[test]
    fn the_trigger_asks_for_the_pages_a_signal_fires_on() {
        use crate::markdown::tests::{payload, placed_run};
        use crate::representation::{NativeLocator, PageRecord, PaintedRect, PdfImageLocator};
        let mut alloc = IdAllocator::new(Profile::default().profile_sha256().unwrap());
        let pages: Vec<PageRecord> = (1..=11)
            .map(|index| PageRecord {
                id: alloc.next(IdKind::Page).unwrap(),
                index,
                width: 40000,
                height: 50000,
                rotation: 0,
                turn: 0,
                // The last page shows a 200 x 250 point crop: a quarter of it is a sixteenth of
                // the page.
                visible: (index == 10).then(|| QRect::new(0, 0, 20000, 25000).unwrap()),
                drawn_boxes: Vec::new(),
            })
            .collect();
        let words = "Seven made-up lanterns guard the quiet northern bay while twelve sailors \
                     mend nets, count gulls and sing old songs";
        let mut nodes = Vec::new();
        for page in &pages {
            let mut add_at = |text: &str, baseline: i64| {
                nodes.push(placed_run(
                    &mut alloc, &page.id, 1, text, None, 7200, baseline, None, None,
                ));
            };
            match page.index {
                // Nineteen words: `songs` is gone.
                8 => add_at(words.trim_end_matches(" songs"), 7200),
                // `twelve` ends one line and `sailors` opens the next, with no space read.
                11 => {
                    let (head, tail) = words.split_at(words.find(" sailors").unwrap());
                    add_at(head, 7200);
                    add_at(tail.trim_start(), 8600);
                }
                // `up` split across two runs, as the engine splits words it kerns.
                _ => {
                    let (head, tail) = words.split_at(12);
                    add_at(head, 7200);
                    add_at(tail, 7200);
                }
            }
            // 200 x 250 points of a 400 x 500 point page is a quarter; the same off the page's
            // foot is under one, clipped; 100 x 125 is a quarter of the last page's crop.
            let painted = match page.index {
                6 => Some(QRect::new(10000, 20000, 30000, 45000)),
                9 => Some(QRect::new(10000, 37500, 30000, 62500)),
                10 => Some(QRect::new(0, 0, 10000, 12500)),
                _ => None,
            };
            if let Some(rect) = painted {
                add_at("", 7200);
                let image = nodes.last_mut().unwrap();
                image.kind = crate::NodeKind::Image;
                image.native_locator = NativeLocator::PdfImage(PdfImageLocator {
                    page: page.index,
                    object: 9,
                    generation: 0,
                    rect: PaintedRect::Painted(rect.unwrap()),
                });
            }
            match page.index {
                2 => nodes.last_mut().unwrap().derivation = DerivationClass::Recognized,
                7 => nodes.last_mut().unwrap().text.push('\u{FFFD}'),
                _ => {}
            }
        }
        let line = QRect::new(7200, 6000, 30000, 7200).unwrap();
        let mut p = payload(nodes, pages);
        p.assurance.limitations.push(crate::Limitation {
            code: crate::assurance::codes::SCANNED_PAGE_NOT_RECOGNIZED.into(),
            detail: "test fixture".into(),
            scope: crate::LimitationScope::Page(3),
        });
        p.figures.push(crate::figures::FigureRecord {
            page: p.pages[3].id.clone(),
            geometry: crate::derivation::GeometryPresence::Measured(line),
            derivation: DerivationClass::Computed,
            detection_rule: crate::FIGURE_RULE_V1.into(),
            runs_inside: Vec::new(),
        });
        p.tables.push(crate::TableRecord {
            id: alloc.next(IdKind::Table).unwrap(),
            page: p.pages[4].id.clone(),
            geometry: crate::derivation::GeometryPresence::Measured(line),
            rows: 0,
            columns: 0,
            cells: Vec::new(),
            derivation: DerivationClass::Computed,
            detection_rule: crate::TABLE_DETECTION_V3.into(),
            locator_check: crate::LocatorCheck {
                check_id: crate::LOCATOR_CHECK_V1.into(),
                check_version: "1".into(),
                outcome: crate::CheckStatus::Ok,
            },
            tagged_check: None,
        });
        assert_eq!(triggered(&p), [2, 3, 4, 5, 6, 7, 8, 10]);
    }
}
