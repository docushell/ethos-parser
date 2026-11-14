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
//! | `model` | `{"id", "version"}`: the id the server reported, and the version the caller named or `not-reported` |
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

use serde::{Deserialize, Serialize};

use crate::derivation::DerivationClass;
use crate::error::EngineError;
use crate::identity::Sha256Hex;
use crate::ids::{IdAllocator, IdKind, NodeId};
use crate::profile::{AssistPin, Profile};
use crate::representation::DocumentRepresentation;

/// The replies file's shape version, its `ethos_assist_answers`.
pub const ASSIST_ANSWERS_VERSION: u32 = 1;

/// The rule that reads a reply into blocks, and that later guards and places them: the profile's
/// `assist.rule`.
pub const ASSIST_RULE_V1: &str = "assist-lightonocr-v1";

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<[u32; 4]>,
    /// Its text, trimmed.
    pub text: String,
    /// Always [`DerivationClass::Proposed`]; the seal refuses anything else.
    pub derivation: DerivationClass,
}

/// The replies file, as read (see the module documentation).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answers {
    ethos_assist_answers: u32,
    source_sha256: Sha256Hex,
    renderer: String,
    request: String,
    model: AssistModel,
    pages: Vec<AnswersPage>,
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
        renderer: file.renderer,
        request: file.request,
        model: file.model.id.clone(),
        model_version: file.model.version.clone(),
    });
    let profile_sha256 = hash(&profile)?;
    let mut ids = IdAllocator::new(profile_sha256.clone());
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
        let (width, height) = match page.visible {
            Some(v) => (v.x1() - v.x0(), v.y1() - v.y0()),
            None => (page.width, page.height),
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
                let outcome = if read.iter().any(|b| b.label.is_some()) {
                    AssistOutcome::Read
                } else {
                    AssistOutcome::NoBlocks
                };
                (outcome, read)
            }
            None => (AssistOutcome::Error, Vec::new()),
        };
        let mut blocks = Vec::with_capacity(read.len());
        for b in read {
            blocks.push(AssistBlock {
                id: ids.next(IdKind::Proposal)?,
                label: b.label,
                continues: b.continues,
                grid: b.grid,
                text: b.text,
                derivation: DerivationClass::Proposed,
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
        model: file.model,
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
                visible: None,
            },
            PageRecord {
                id: ids.next(IdKind::Page).unwrap(),
                index: 2,
                width: 30000,
                height: 40000,
                rotation: 0,
                visible: Some(QRect::new(5000, 5000, 25000, 35000).unwrap()),
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
        // Unedited, and with a page the document has left out, it reads.
        read(&|_| {}).unwrap();
        read(&|a| a["pages_left_out"] = json!([3])).unwrap();

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
}
