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

//! `ask SERVER FILE PAGES [MODEL] <PDF`: each page asked is rendered, sent to SERVER's
//! `/v1/chat/completions` as decision #123 asked it, and its reply written to the replies file
//! (`assist/README.md`). FILE is replaced atomically after each page, so a stopped run resumes:
//! a page FILE holds a reply for is kept, and a page it holds an error for is asked again.
//!
//! Exit 0 when every page asked has a reply or a named error; 4 when the server cannot be
//! reached (FILE keeps the pages done); 5 on a refusal, named; 1 when FILE cannot be written.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::OpenOptions;
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::time::Instant;

use hayro::hayro_syntax::Pdf;

use crate::digest::{base64, sha256};
use crate::json::{quote, Json};
use crate::transport::{self, Failure};
use crate::{render_page, RENDERER, REQUEST};

/// The replies file's shape version, its `ethos_assist_answers`.
const ANSWERS_VERSION: i64 = 1;
/// Decision #123's cap on a reply's tokens.
pub const MAX_TOKENS: u32 = 12_288;
/// `model.version` when none is named: OpenAI-compatible servers report no revision.
pub const NOT_REPORTED: &str = "not-reported";
/// The environment variable the API key is read from, and nothing else.
pub const KEY_VAR: &str = "ETHOS_PARSER_ASSIST_KEY";

/// Why a run stopped: its exit code and what it says on stderr.
pub struct Stop {
    pub code: u8,
    pub message: String,
}

fn refused(name: &str, detail: impl std::fmt::Display) -> Stop {
    Stop {
        code: 5,
        message: format!("{name}: {detail}"),
    }
}

/// A failure that ends a run whatever page it hit; `None` for one recorded against its page.
pub fn fatal(f: &Failure) -> Option<Stop> {
    match f {
        Failure::Unreachable => Some(Stop {
            code: 4,
            message: "server-unreachable: no connection to SERVER; is the server running?".into(),
        }),
        Failure::NoCurl => Some(refused(
            "curl-not-found",
            "the assist lane sends its requests with the system `curl`; install it, or put it on PATH",
        )),
        Failure::NotRun(_) => Some(refused("curl-not-run", f.name())),
        Failure::KeyInClear => Some(refused(
            "key-needs-https",
            format!(
                "{KEY_VAR} is set, and SERVER is an http:// address off this machine, so the key \
                 would cross the network unencrypted; name an https:// address, or a server on \
                 loopback"
            ),
        )),
        _ => None,
    }
}

/// SERVER without a trailing `/`. Only `http://` and `https://` addresses, in any case, are sent
/// to; the address is never echoed, so no output holds it.
pub fn server_base(server: &str) -> Result<&str, String> {
    let scheme = server.get(..8).unwrap_or(server).to_ascii_lowercase();
    let http = scheme.starts_with("http://") || scheme.starts_with("https://");
    if http && !server.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Ok(server.trim_end_matches('/'))
    } else {
        Err("server-not-http: SERVER is an http:// or https:// address".into())
    }
}

/// The API key, from `ETHOS_PARSER_ASSIST_KEY` only; unset or empty is none. Never a flag, so no
/// argv holds it, and never written.
pub fn key() -> Result<Option<String>, String> {
    match std::env::var(KEY_VAR) {
        Err(std::env::VarError::NotPresent) => Ok(None),
        Ok(k) if k.is_empty() => Ok(None),
        Ok(k) if !k.chars().any(char::is_control) => Ok(Some(k)),
        _ => Err(format!(
            "key-not-a-header-value: {KEY_VAR} holds a control character, or is not UTF-8"
        )),
    }
}

/// `ID[@VERSION]` as an id and a version, [`NOT_REPORTED`] when none is named.
pub fn split_model(m: &str) -> (String, String) {
    match m.rsplit_once('@') {
        Some((id, v)) => (id.into(), v.into()),
        None => (m.into(), NOT_REPORTED.into()),
    }
}

/// The status of `GET /v1/models` and the ids it lists (none unless 200).
pub fn listed_models(base: &str, key: Option<&str>) -> Result<(u16, Vec<String>), Failure> {
    let r = transport::request(&format!("{base}/v1/models"), key, None)?;
    let ids = Json::parse(&r.body)
        .filter(|_| r.status == 200)
        .and_then(|v| {
            let ids = v.get("data")?.arr()?.iter();
            Some(
                ids.filter_map(|m| Some(m.get("id")?.str()?.to_owned()))
                    .collect(),
            )
        })
        .unwrap_or_default();
    Ok((r.status, ids))
}

/// Why no model was chosen from a listing.
pub fn unchosen(status: u16, ids: &[String]) -> String {
    let why = match ids.len() {
        _ if status != 200 => format!("GET /v1/models answered http {status}"),
        0 => "the server lists no model".into(),
        n => format!("the server lists {n}: {}", ids.join(", ")),
    };
    format!("model-not-chosen: {why}; name one as MODEL")
}

/// Decision #123's request: one PNG data URL and the prompt `grounding`, temperature 0, 12,288
/// tokens at most, not streamed (Phase 4a REPORT, "Request body").
pub fn request_body(model: &str, png: &[u8]) -> Vec<u8> {
    format!(
        "{{\"model\":{},\"max_tokens\":{MAX_TOKENS},\"temperature\":0.0,\"stream\":false,\
         \"messages\":[{{\"role\":\"user\",\"content\":[{{\"type\":\"image_url\",\"image_url\":\
         {{\"url\":\"data:image/png;base64,{}\"}}}},{{\"type\":\"text\",\"text\":\"grounding\"}}]}}]}}",
        quote(model),
        base64(png)
    )
    .into_bytes()
}

/// What a chat completion holds that the lane keeps.
pub struct Completion {
    /// The id the server reports, verbatim.
    pub model: Option<String>,
    pub finish_reason: String,
    pub content: String,
    pub tokens: Option<i64>,
}

/// The first choice of an OpenAI chat completion, or `None` when `body` is not one.
pub fn completion(body: &[u8]) -> Option<Completion> {
    let v = Json::parse(body)?;
    let choice = v.get("choices")?.arr()?.first()?;
    Some(Completion {
        model: v.get("model").and_then(Json::str).map(str::to_owned),
        finish_reason: choice.get("finish_reason")?.str()?.to_owned(),
        content: choice.get("message")?.get("content")?.str()?.to_owned(),
        tokens: v
            .get("usage")
            .and_then(|u| u.get("completion_tokens"))
            .and_then(Json::int),
    })
}

/// A page's outcome: the model's reply, or the name of what went wrong.
enum Outcome {
    Reply {
        finish_reason: String,
        reply: String,
    },
    Error(String),
}

struct Page {
    points: [i64; 2],
    dpi: u32,
    width: u32,
    height: u32,
    pixels_sha256: String,
    outcome: Outcome,
}

/// The replies file (DESIGN §2): never the server's address, the key or a timing, so the same
/// replies give the same bytes.
struct Answers {
    source_sha256: String,
    model_id: String,
    model_version: String,
    /// The id the request named (MODEL, or the one model the server listed): with `model_id`, the
    /// names a later run may resume the file under (review).
    model_asked: String,
    pages: BTreeMap<u32, Page>,
}

fn obj<const N: usize>(fields: [(&str, Json); N]) -> Json {
    Json::Obj(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn int(n: impl Into<i64>) -> Json {
    Json::Num(n.into().to_string())
}

fn text(s: &str) -> Json {
    Json::Str(s.into())
}

/// `v`'s fields, when it is an object holding no key outside `known`.
fn fields<'a>(v: &'a Json, known: &[&str]) -> Option<&'a BTreeMap<String, Json>> {
    match v {
        Json::Obj(m) if m.keys().all(|k| known.contains(&k.as_str())) => Some(m),
        _ => None,
    }
}

impl Answers {
    fn to_json(&self) -> Json {
        let pages = self.pages.iter().map(|(&n, p)| {
            let mut page = obj([
                ("page", int(n)),
                (
                    "points",
                    Json::Arr(vec![int(p.points[0]), int(p.points[1])]),
                ),
                ("dpi", int(p.dpi)),
                ("width", int(p.width)),
                ("height", int(p.height)),
                ("pixels_sha256", text(&p.pixels_sha256)),
            ]);
            if let Json::Obj(m) = &mut page {
                match &p.outcome {
                    Outcome::Reply {
                        finish_reason,
                        reply,
                    } => {
                        m.insert("finish_reason".into(), text(finish_reason));
                        m.insert("reply".into(), text(reply));
                    }
                    Outcome::Error(e) => {
                        m.insert("error".into(), text(e));
                    }
                }
            }
            page
        });
        obj([
            ("ethos_assist_answers", int(ANSWERS_VERSION)),
            ("source_sha256", text(&self.source_sha256)),
            ("renderer", text(RENDERER)),
            ("request", text(REQUEST)),
            (
                "model",
                obj([
                    ("asked", text(&self.model_asked)),
                    ("id", text(&self.model_id)),
                    ("version", text(&self.model_version)),
                ]),
            ),
            ("pages", Json::Arr(pages.collect())),
        ])
    }

    /// A replies file as written by this build; refused by the names the engine refuses it by.
    fn read(bytes: &[u8]) -> Result<Answers, Stop> {
        let unreadable = |why: &str| refused("assist-answers-unreadable", format!("FILE {why}"));
        let v = Json::parse(bytes).ok_or_else(|| unreadable("is not JSON"))?;
        let top = [
            "ethos_assist_answers",
            "source_sha256",
            "renderer",
            "request",
            "model",
            "pages",
        ];
        fields(&v, &top).ok_or_else(|| unreadable("holds a field this build does not write"))?;
        let s = |v: &Json, k: &str| v.get(k).and_then(Json::str).map(str::to_owned);
        if v.get("ethos_assist_answers").and_then(Json::int) != Some(ANSWERS_VERSION) {
            return Err(unreadable("is not version 1 of the replies file"));
        }
        if s(&v, "renderer").as_deref() != Some(RENDERER) {
            return Err(refused(
                "assist-answers-other-renderer",
                format!("FILE's pages were not drawn by `{RENDERER}`"),
            ));
        }
        if s(&v, "request").as_deref() != Some(REQUEST) {
            return Err(refused(
                "assist-answers-other-request",
                format!("FILE's pages were not asked as `{REQUEST}`"),
            ));
        }
        let model = v
            .get("model")
            .filter(|m| fields(m, &["asked", "id", "version"]).is_some())
            .ok_or_else(|| unreadable("has no model"))?;
        let model_id = s(model, "id").ok_or_else(|| unreadable("has no model id"))?;
        let mut answers = Answers {
            source_sha256: s(&v, "source_sha256").ok_or_else(|| unreadable("has no source"))?,
            // A file that names no `asked` was asked by the id its server reported.
            model_asked: s(model, "asked").unwrap_or_else(|| model_id.clone()),
            model_id,
            model_version: s(model, "version").ok_or_else(|| unreadable("has no version"))?,
            pages: BTreeMap::new(),
        };
        let page_keys = [
            "page",
            "points",
            "dpi",
            "width",
            "height",
            "pixels_sha256",
            "finish_reason",
            "reply",
            "error",
        ];
        let pages = v.get("pages").and_then(Json::arr);
        for p in pages.ok_or_else(|| unreadable("has no pages"))? {
            let bad = || unreadable("holds a page this build does not write");
            fields(p, &page_keys).ok_or_else(bad)?;
            let n = |k: &str| {
                p.get(k)
                    .and_then(Json::int)
                    .and_then(|n| u32::try_from(n).ok())
            };
            let points = p.get("points").and_then(Json::arr).unwrap_or_default();
            let outcome = match (s(p, "finish_reason"), s(p, "reply"), s(p, "error")) {
                (Some(finish_reason), Some(reply), None) => Outcome::Reply {
                    finish_reason,
                    reply,
                },
                (None, None, Some(e)) => Outcome::Error(e),
                _ => return Err(bad()),
            };
            let page = Page {
                points: match points {
                    [w, h] => [w.int().ok_or_else(bad)?, h.int().ok_or_else(bad)?],
                    _ => return Err(bad()),
                },
                dpi: n("dpi").ok_or_else(bad)?,
                width: n("width").ok_or_else(bad)?,
                height: n("height").ok_or_else(bad)?,
                pixels_sha256: s(p, "pixels_sha256").ok_or_else(bad)?,
                outcome,
            };
            let number = n("page").ok_or_else(bad)?;
            if answers
                .pages
                .keys()
                .next_back()
                .is_some_and(|&last| last >= number)
            {
                return Err(unreadable("holds pages out of order, or a page twice"));
            }
            answers.pages.insert(number, page);
        }
        Ok(answers)
    }

    /// Replaces FILE whole: written beside it as `FILE.N.part`, a name no file holds yet (never
    /// following a link, never another run's), then renamed over it.
    fn save(&self, file: &Path) -> Result<(), Stop> {
        let failed = |e: std::io::Error| Stop {
            code: 1,
            message: format!("{}: {e}", file.display()),
        };
        let text = self.to_json().canonical();
        for n in 0..1000 {
            let mut part = file.as_os_str().to_owned();
            part.push(format!(".{n}.part"));
            let mut out = match OpenOptions::new().write(true).create_new(true).open(&part) {
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                opened => opened.map_err(failed)?,
            };
            let done = out.write_all(text.as_bytes()).and_then(|()| {
                drop(out);
                std::fs::rename(&part, file)
            });
            if done.is_err() {
                let _ = std::fs::remove_file(&part);
            }
            return done.map_err(failed);
        }
        Err(failed(std::io::Error::other(
            "1000 `FILE.N.part` files are in the way",
        )))
    }
}

/// PAGES (`1,3,7`) as page numbers, each in the document.
fn page_list(pages: &str, count: usize) -> Result<BTreeSet<u32>, Stop> {
    let mut list = BTreeSet::new();
    for p in pages.split(',') {
        let n: u32 = p.trim().parse().ok().filter(|&n| n >= 1).ok_or_else(|| {
            refused(
                "pages-unreadable",
                "PAGES lists page numbers from 1, as `1,3,7`",
            )
        })?;
        if n as usize > count {
            return Err(refused(
                "page-not-in-document",
                format!("page {n} was asked; the document has {count}"),
            ));
        }
        list.insert(n);
    }
    Ok(list)
}

pub fn ask(server: &str, file: &Path, pages: &str, model: Option<&str>) -> Result<(), Stop> {
    let named = |message| Stop { code: 5, message };
    let base = server_base(server).map_err(named)?;
    let key = key().map_err(named)?;
    let mut bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut bytes)
        .map_err(|e| refused("not-a-pdf", format!("stdin: {e}")))?;
    let source_sha256 = sha256(&bytes);
    let pdf = Pdf::new(bytes).map_err(|e| refused("not-a-pdf", format!("{e:?}")))?;
    let wanted = page_list(pages, pdf.pages().len())?;
    let (asked, version) = match model {
        Some(m) => split_model(m),
        None => match listed_models(base, key.as_deref()) {
            Ok((_, ids)) if ids.len() == 1 => (ids[0].clone(), NOT_REPORTED.into()),
            Ok((status, ids)) => return Err(named(unchosen(status, &ids))),
            Err(f) => {
                let why = format!("GET /v1/models: {}", f.name());
                return Err(fatal(&f).unwrap_or_else(|| refused("model-not-chosen", why)));
            }
        },
    };

    let mut answers = match std::fs::read(file) {
        Ok(bytes) => Answers::read(&bytes)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Answers {
            source_sha256: source_sha256.clone(),
            model_id: asked.clone(),
            model_version: version.clone(),
            model_asked: asked.clone(),
            pages: BTreeMap::new(),
        },
        Err(e) => {
            return Err(Stop {
                code: 1,
                message: format!("{}: {e}", file.display()),
            })
        }
    };
    if answers.source_sha256 != source_sha256 {
        return Err(refused(
            "assist-answers-other-source",
            "FILE answers another PDF",
        ));
    }
    if answers.model_version != version {
        return Err(refused(
            "assist-answers-other-model",
            format!(
                "FILE holds version `{}`'s replies, and `{version}` was named",
                answers.model_version
            ),
        ));
    }
    // Replies are another model's unless the model named is the one they were asked by, or the
    // one the server reported, so a server's alias resumes under either name (review). Checked
    // before any page is passed over as answered: a run whose pages are all answered asks nothing.
    let replied = |p: &Page| matches!(p.outcome, Outcome::Reply { .. });
    if !answers.pages.values().any(replied) {
        answers.model_id = asked.clone();
        answers.model_asked = asked.clone();
    } else if asked != answers.model_asked && asked != answers.model_id {
        return Err(refused(
            "assist-answers-other-model",
            format!(
                "FILE holds `{}`'s replies, asked as `{}`, and `{asked}` was named",
                answers.model_id, answers.model_asked
            ),
        ));
    }

    let url = format!("{base}/v1/chat/completions");
    for n in wanted {
        if let Some(Page {
            outcome: Outcome::Reply { .. },
            ..
        }) = answers.pages.get(&n)
        {
            eprintln!("page {n}: answered before; kept");
            continue;
        }
        let page = render_page(&pdf, n as usize).map_err(|e| refused("page-not-rendered", e))?;
        let started = Instant::now();
        let sent = transport::request(&url, key.as_deref(), Some(&request_body(&asked, &page.png)));
        let secs = started.elapsed().as_secs_f64();
        let outcome = match sent {
            Err(f) => match fatal(&f) {
                Some(stop) => return Err(stop),
                None => Outcome::Error(f.name()),
            },
            Ok(r) if r.status != 200 => Outcome::Error(format!("http {}", r.status)),
            Ok(r) => match completion(&r.body) {
                None => Outcome::Error("reply-unreadable".into()),
                Some(c) => {
                    let id = c.model.unwrap_or_else(|| asked.clone());
                    if !answers.pages.values().any(replied) {
                        answers.model_id = id;
                    } else if id != answers.model_id {
                        return Err(refused(
                            "assist-answers-other-model",
                            format!(
                                "the server reports `{id}`, and FILE holds `{}`'s replies",
                                answers.model_id
                            ),
                        ));
                    }
                    Outcome::Reply {
                        finish_reason: c.finish_reason,
                        reply: c.content,
                    }
                }
            },
        };
        match &outcome {
            Outcome::Reply { finish_reason, .. } if finish_reason == "length" => eprintln!(
                "page {n}: reply-capped: the reply reached {MAX_TOKENS} tokens and was cut; kept \
                 with finish_reason `length`, {secs:.1} s"
            ),
            Outcome::Reply { finish_reason, .. } => {
                eprintln!("page {n}: {finish_reason}, {secs:.1} s")
            }
            Outcome::Error(e) => eprintln!("page {n}: {e}; recorded, and asked again next run"),
        }
        answers.pages.insert(
            n,
            Page {
                points: page.points,
                dpi: page.dpi,
                width: page.width,
                height: page.height,
                pixels_sha256: sha256(&page.png),
                outcome,
            },
        );
        answers.save(file)?;
    }
    Ok(())
}
