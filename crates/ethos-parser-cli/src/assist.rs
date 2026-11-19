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

//! `extract --assist`'s drafting model, asked through `ethos-parser-assist`, a separate binary on
//! its own toolchain (`assist/README.md`, decision #149). This process finds it as it finds the
//! recognizer — `ETHOS_PARSER_ASSIST`, or beside `ethos-parser` — checks that it draws and asks as
//! this build reads, and runs `ethos-parser-assist ask SERVER FILE PAGES [MODEL] <PDF`, which writes
//! FILE. `extract` then reads FILE as `--assist-answers FILE` does, so the output is a function of
//! the PDF and FILE alone, and this process opens no socket and renders nothing. The API key,
//! `ETHOS_PARSER_ASSIST_KEY`, reaches the child in the environment it inherits, never in argv.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use ethos_parser_core::{EngineError, ASSIST_RENDERER, ASSIST_REQUEST_V1};

use crate::recognizer::{located, missing};

/// The environment variable that names the assist binary.
pub(crate) const ENV: &str = "ETHOS_PARSER_ASSIST";

/// Ask the server at `server` to read `pages` (1-based) of `pdf`, writing or resuming the replies
/// file `file`. The child's progress goes to stderr as it comes. Its exit is named: 3, pages it
/// could not render, is said on stderr and FILE is read as it stands; 4 is
/// `assist-server-unreachable`; 5 is the child's refusal, `assist-answers-*` as the merge refuses
/// it and any other as `assist-` and the child's name; anything else is the child's last word.
///
/// No timeout is set: the child bounds each request (its `curl` has a time limit), and a child
/// that hangs otherwise hangs `extract`, as a hung recognizer hangs `--ocr`.
pub(crate) fn ask(
    server: &str,
    file: &Path,
    pages: &[u32],
    model: Option<&str>,
    pdf: &[u8],
) -> Result<(), EngineError> {
    let path = located(ENV, "ethos-parser-assist")?;
    if !path.is_file() {
        return Err(missing(format!(
            "assist binary `{}`: --assist asks a model through ethos-parser-assist, which must sit \
             beside ethos-parser or be named by {ENV}",
            path.display()
        )));
    }
    let named = Command::new(&path)
        .arg("identity")
        .output()
        .map_err(|e| missing(format!("assist binary `{}`: {e}", path.display())))?;
    let identity: serde_json::Value = serde_json::from_slice(&named.stdout).unwrap_or_default();
    if identity["renderer"] != ASSIST_RENDERER || identity["request"] != ASSIST_REQUEST_V1 {
        return Err(missing(format!(
            "assist binary `{}` states {}; this build reads pages drawn by `{ASSIST_RENDERER}` and \
             asked as `{ASSIST_REQUEST_V1}`",
            path.display(),
            String::from_utf8_lossy(&named.stdout).trim()
        )));
    }

    let pages: Vec<String> = pages.iter().map(u32::to_string).collect();
    let mut child = Command::new(&path)
        .arg("ask")
        .arg(server)
        .arg(file)
        .arg(pages.join(","))
        .args(model)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| missing(format!("assist binary `{}`: {e}", path.display())))?;
    // Its stderr is passed on from a thread of its own while the PDF is written, so a child that
    // says much before it reads cannot stall either side, and read as bytes, a line that is not
    // UTF-8 passed on lossily rather than closing the pipe. A child that refuses before it reads
    // has closed its stdin, and says why there.
    let stderr = BufReader::new(child.stderr.take().expect("piped"));
    let echo = std::thread::spawn(move || {
        let mut said = String::new();
        for line in stderr.split(b'\n') {
            let Ok(line) = line else { break };
            said = String::from_utf8_lossy(&line).into_owned();
            let _ = writeln!(std::io::stderr(), "{said}");
        }
        said
    });
    let _ = child.stdin.take().expect("piped").write_all(pdf);
    let said = echo.join().unwrap_or_default();
    let status = child.wait()?;
    let refusal = said
        .strip_prefix("ethos-parser-assist: ")
        .and_then(|s| s.split_once(": "))
        .map(|(name, detail)| (name.to_owned(), detail.to_owned()));
    match (status.code(), refusal) {
        (Some(0), _) => Ok(()),
        (Some(3), _) => {
            let _ = writeln!(
                std::io::stderr(),
                "engine: ethos-parser-assist could not render every page asked; FILE holds no \
                 reply to those [assist-page-not-rendered]"
            );
            Ok(())
        }
        (Some(4), Some((_, detail))) => Err(EngineError::Io {
            detail: format!("assist-server-unreachable: {detail}"),
        }),
        (Some(5), Some((what, detail))) if what.starts_with("assist-answers-") => {
            Err(EngineError::Malformed { what, detail })
        }
        (Some(5), Some((name, detail))) => Err(EngineError::Unsupported {
            what: format!("assist-{name}"),
            detail,
        }),
        (code, _) => Err(EngineError::Io {
            detail: format!(
                "ethos-parser-assist stopped ({}): {said}",
                code.map_or("by a signal".into(), |c| format!("exit {c}"))
            ),
        }),
    }
}
