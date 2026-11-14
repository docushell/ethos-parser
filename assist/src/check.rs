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

//! `check SERVER [MODEL]`: whether a server is ready for the assist lane. It asks for the
//! committed test page (`data/check-page.pdf`: made-up content, a heading, a sentence and a ruled
//! 3 x 3 table of numbers) as `ask` would, and says what came back, a line a step.
//!
//! Exit 0 when the reply is read as [`REQUEST`]; 1 when it is not (`reply-format-not-read`: a
//! general chat model) or the server answered with an error; 2 when the server cannot be
//! reached, or curl cannot be run. It never prints the key or the server's address.

use std::collections::BTreeSet;
use std::fmt::Display;
use std::time::Instant;

use hayro::hayro_syntax::Pdf;

use crate::ask::{
    completion, fatal, key, listed_models, request_body, server_base, split_model, unchosen,
    NOT_REPORTED,
};
use crate::transport::{self, Failure};
use crate::{render_page, RENDERER, REQUEST};

const PAGE: &[u8] = include_bytes!("../data/check-page.pdf");
/// The test page's words and numbers, and its table's size.
const WORDS: [&str; 9] = [
    "harbor", "report", "seven", "lanterns", "guard", "the", "quiet", "northern", "bay",
];
const NUMBERS: [&str; 9] = [
    "14", "27", "39", "152", "268", "371", "4085", "5190", "6243",
];
const ROWS: usize = 3;
const COLS: usize = 3;

fn line(label: &str, text: impl Display) {
    println!("{label:<10}{text}");
}

fn stop(code: u8, label: &str, text: impl Display) -> (u8, String) {
    (code, format!("{label:<10}{text}"))
}

/// A transport failure as `check` reports it: 2 when the server cannot be reached, or curl
/// cannot be run.
fn failed(label: &str, f: &Failure) -> (u8, String) {
    match fatal(f) {
        Some(s) => stop(2, label, s.message),
        None => stop(1, label, f.name()),
    }
}

pub fn check(server: &str, model: Option<&str>) -> u8 {
    match run(server, model) {
        Ok(()) => 0,
        Err((code, last)) => {
            println!("{last}");
            code
        }
    }
}

fn run(server: &str, model: Option<&str>) -> Result<(), (u8, String)> {
    let base = server_base(server).map_err(|m| stop(2, "server", m))?;
    let key = key().map_err(|m| stop(1, "key", m))?;
    let started = Instant::now();
    let (status, ids) = listed_models(base, key.as_deref()).map_err(|f| failed("server", &f))?;
    let ms = started.elapsed().as_millis();
    line(
        "server",
        format!("answers: GET /v1/models {status} in {ms} ms"),
    );

    let (id, version) = match model {
        Some(m) => split_model(m),
        None if ids.len() == 1 => (ids[0].clone(), NOT_REPORTED.into()),
        None => return Err(stop(1, "model", unchosen(status, &ids))),
    };
    line("model", format!("{id}   <- every output records this id"));
    if version == NOT_REPORTED {
        line(
            "version",
            "not reported by the server; record one with --assist-model ID@VERSION",
        );
    } else {
        line("version", format!("{version}   <- named with the model"));
    }

    let pdf = Pdf::new(PAGE.to_vec()).map_err(|e| stop(1, "page", format!("{e:?}")))?;
    let page = render_page(&pdf, 1).map_err(|e| stop(1, "page", e))?;
    let pt = |c: i64| match c % 100 {
        0 => (c / 100).to_string(),
        _ => format!("{:.2}", c as f64 / 100.0),
    };
    let hayro = RENDERER.split("; ").nth(1).unwrap_or(RENDERER);
    line(
        "page",
        format!(
            "1 test page, {} x {} pt -> {} x {} px at {} dpi ({hayro})",
            pt(page.points[0]),
            pt(page.points[1]),
            page.width,
            page.height,
            page.dpi
        ),
    );

    let started = Instant::now();
    let url = format!("{base}/v1/chat/completions");
    let sent = transport::request(&url, key.as_deref(), Some(&request_body(&id, &page.png)));
    let secs = started.elapsed().as_secs_f64();
    let reply = match sent {
        Err(f) => return Err(failed("reply", &f)),
        Ok(r) if r.status != 200 => return Err(stop(1, "reply", format!("http {}", r.status))),
        Ok(r) => completion(&r.body).ok_or_else(|| {
            stop(
                1,
                "reply",
                "reply-unreadable: not an OpenAI chat completion",
            )
        })?,
    };
    let recorded = reply.model.clone().unwrap_or(id.clone());
    if recorded != id {
        line(
            "model",
            format!("the server reports {recorded}   <- every output records this id"),
        );
    }
    // The rule the engine refuses a replies file by (`assist-answers-model-is-path`, decision
    // #145): an id starting `/`, `~` or `.`, or with a drive letter, or holding a backslash.
    let id_trimmed = recorded.trim();
    let drive = matches!(id_trimmed.as_bytes(), [letter, b':', ..] if letter.is_ascii_alphabetic());
    if id_trimmed.starts_with(['/', '~', '.']) || drive || id_trimmed.contains('\\') {
        line(
            "warning",
            "the id is a path: the engine refuses replies recorded under it \
             (assist-answers-model-is-path); serve the model by a relative --model \
             (assist/README.md)",
        );
    }

    let blocks = blocks(&reply.content);
    let tokens = reply
        .tokens
        .map(|t| format!(", {t} tokens"))
        .unwrap_or_default();
    let labels: Vec<&str> = blocks.iter().map(|b| b.0).collect();
    let listed = match labels.len() {
        0 => "no blocks".into(),
        n => format!("{n} blocks: {}", labels.join(", ")),
    };
    line(
        "reply",
        format!("{}{tokens}, {secs:.1} s; {listed}", reply.finish_reason),
    );
    if blocks.is_empty() {
        return Err(stop(
            1,
            "not ready",
            format!(
                "reply-format-not-read: the reply holds no block marker `![label](x0,y0,x1,y1)`, \
                 so it is not read as {REQUEST}; a general chat model answers so"
            ),
        ));
    }
    line("reads", reads(&blocks));
    line("ready", format!("replies parse as {REQUEST}"));
    Ok(())
}

/// The marker `![label](x0,y0,x1,y1)` at the start of `s` (ParseBench's pattern): its label and
/// its length.
fn marker(s: &str) -> Option<(&str, usize)> {
    let rest = s.strip_prefix("![")?;
    let n = rest
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '_' || c == '+'))
        .unwrap_or(rest.len());
    let after = rest[n..].strip_prefix("](")?;
    let close = after.find(')')?;
    let parts: Vec<&str> = after[..close].split(',').collect();
    let numbers = parts.iter().all(|p| {
        let p = p.trim();
        !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())
    });
    (n > 0 && parts.len() == 4 && numbers).then_some((&rest[..n], 2 + n + 2 + close + 1))
}

/// A reply's blocks: each marker's label and the text up to the next marker.
fn blocks(reply: &str) -> Vec<(&str, &str)> {
    let mut marks = Vec::new();
    let mut from = 0;
    while let Some(i) = reply[from..].find("![") {
        let at = from + i;
        match marker(&reply[at..]) {
            Some((label, len)) => {
                marks.push((label, at, at + len));
                from = at + len;
            }
            None => from = at + 2,
        }
    }
    marks
        .iter()
        .enumerate()
        .map(|(k, &(label, _, end))| {
            let next = marks.get(k + 1).map_or(reply.len(), |m| m.1);
            (label, reply[end..next].trim())
        })
        .collect()
}

/// How much of the test page the reply holds.
fn reads(blocks: &[(&str, &str)]) -> String {
    let tokens: BTreeSet<String> = blocks
        .iter()
        .flat_map(|b| b.1.split(|c: char| !c.is_alphanumeric()))
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect();
    let found = |list: &[&str]| list.iter().filter(|w| tokens.contains(**w)).count();
    let cells = |row: &str| {
        ["<td>", "<td ", "<th>", "<th "]
            .iter()
            .map(|t| row.matches(t).count())
            .sum::<usize>()
    };
    let table = match blocks.iter().map(|b| b.1).find(|t| t.contains("<table")) {
        None => format!("no table; drawn {ROWS} x {COLS}"),
        Some(html) => {
            let rows: Vec<usize> = html.split("<tr").skip(1).map(cells).collect();
            if rows.len() == ROWS && rows.iter().all(|&c| c == COLS) {
                format!("table {ROWS} x {COLS} as drawn")
            } else {
                let n: usize = rows.iter().sum();
                format!(
                    "table of {} rows, {n} cells; drawn {ROWS} x {COLS}",
                    rows.len()
                )
            }
        }
    };
    format!(
        "{} of {} words, {} of {} numbers; {table}",
        found(&WORDS),
        WORDS.len(),
        found(&NUMBERS),
        NUMBERS.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_are_read_as_parsebench_reads_them() {
        let reply = "![title](102,61,898,95) # Harbor Report\n\n![text+]( 1 , 2,3,4 ) body\n\n\
                     ![](1,2,3,4) not one ![x](1,2,3) nor ![x](a,2,3,4)";
        let got = blocks(reply);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("title", "# Harbor Report"));
        assert_eq!(got[1].0, "text+");
        assert!(got[1].1.starts_with("body"));
        assert!(blocks("A plain answer, in prose.").is_empty());
    }

    #[test]
    fn reads_counts_the_test_pages_words_numbers_and_table() {
        let table = "<table><tr><td>14</td><td>27</td><td>39</td></tr>\
                     <tr><td>152</td><td>268</td><td>371</td></tr>\
                     <tr><td>4085</td><td>5190</td><td>6243</td></tr></table>";
        let reply = format!(
            "![title](1,1,9,9) # Harbor Report\n\n![text](1,1,9,9) Seven lanterns guard the \
             quiet northern bay.\n\n![table](1,1,9,9) {table}"
        );
        assert_eq!(
            reads(&blocks(&reply)),
            "9 of 9 words, 9 of 9 numbers; table 3 x 3 as drawn"
        );
        let short = "![table](1,1,9,9) <table><thead><tr><th>14</th></tr></thead></table>";
        assert_eq!(
            reads(&blocks(short)),
            "0 of 9 words, 1 of 9 numbers; table of 1 rows, 1 cells; drawn 3 x 3"
        );
    }
}
