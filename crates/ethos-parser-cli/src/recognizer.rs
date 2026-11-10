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

//! `--ocr`'s recognizer: `ethos-parser-ocr`, a separate binary on its own toolchain, which this
//! process finds, asks for its identity, and talks to over its stdin and stdout
//! (`docs/35-OCR-SCOPE.md` §5).
//!
//! **Found, never fallen back from.** `ETHOS_PARSER_OCR` names it when set, and a path that is not
//! a file is a named error; otherwise it is `ethos-parser-ocr` beside this executable. A missing
//! recognizer fails `--ocr` by name rather than producing an artifact without OCR
//! (`docs/07-VERIFY-BOUNDARY.md` §8.6). The `serve` process starts on the first scanned page, so a
//! document with none costs one `identity` call.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use ethos_parser_core::{EngineError, RecognizerIdentity};
use ethos_parser_pdf::{Recognition, RecognitionRequest, Recognize};

/// The environment variable that names the recognizer binary.
pub(crate) const ENV: &str = "ETHOS_PARSER_OCR";

/// A running (or not yet started) `ethos-parser-ocr serve`.
pub(crate) struct Process {
    path: PathBuf,
    identity: RecognizerIdentity,
    serving: Option<(Child, ChildStdin, BufReader<ChildStdout>)>,
}

fn missing(part: String) -> EngineError {
    EngineError::MissingPart { part }
}

impl Process {
    /// Find the recognizer and read its identity.
    pub(crate) fn find() -> Result<Self, EngineError> {
        let name = if cfg!(windows) {
            "ethos-parser-ocr.exe"
        } else {
            "ethos-parser-ocr"
        };
        let path = match std::env::var_os(ENV) {
            Some(p) => PathBuf::from(p),
            None => std::env::current_exe()
                .map_err(|e| EngineError::Io {
                    detail: e.to_string(),
                })?
                .with_file_name(name),
        };
        if !path.is_file() {
            return Err(missing(format!(
                "recognizer `{}`: --ocr reads scanned pages with ethos-parser-ocr, which must sit \
                 beside ethos-parser or be named by {ENV}",
                path.display()
            )));
        }
        let out = Command::new(&path)
            .arg("identity")
            .output()
            .map_err(|e| missing(format!("recognizer `{}`: {e}", path.display())))?;
        if !out.status.success() {
            return Err(missing(format!(
                "recognizer `{}` did not state its identity: {}",
                path.display(),
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let identity = serde_json::from_slice(&out.stdout).map_err(|e| EngineError::Malformed {
            what: "recognizer identity".into(),
            detail: e.to_string(),
        })?;
        Ok(Self {
            path,
            identity,
            serving: None,
        })
    }

    fn serving(&mut self) -> Result<&mut (Child, ChildStdin, BufReader<ChildStdout>), EngineError> {
        if self.serving.is_none() {
            let mut child = Command::new(&self.path)
                .arg("serve")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| missing(format!("recognizer `{}`: {e}", self.path.display())))?;
            let stdin = child.stdin.take().expect("piped");
            let stdout = BufReader::new(child.stdout.take().expect("piped"));
            self.serving = Some((child, stdin, stdout));
        }
        Ok(self.serving.as_mut().expect("started above"))
    }

    /// The recognizer stopped answering: say why, from its exit status and what it wrote.
    fn stopped(&mut self) -> EngineError {
        let Some((mut child, stdin, _)) = self.serving.take() else {
            return EngineError::Io {
                detail: "the recognizer is not running".into(),
            };
        };
        drop(stdin);
        let status = child.wait().ok();
        let mut stderr = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = std::io::Read::read_to_string(&mut e, &mut stderr);
        }
        let said = stderr.trim();
        if status.and_then(|s| s.code()) == Some(3) {
            return EngineError::Unsupported {
                what: "cpu".into(),
                detail: said.to_string(),
            };
        }
        EngineError::Io {
            detail: format!("the recognizer stopped: {said}"),
        }
    }
}

impl Recognize for Process {
    fn identity(&self) -> &RecognizerIdentity {
        &self.identity
    }

    fn read(&mut self, request: &RecognitionRequest) -> Result<Recognition, EngineError> {
        let header = serde_json::to_string(request).map_err(|e| EngineError::Malformed {
            what: "recognizer request".into(),
            detail: e.to_string(),
        })?;
        let (_, stdin, stdout) = self.serving()?;
        let sent = writeln!(stdin, "{header}")
            .and_then(|()| stdin.write_all(&request.data))
            .and_then(|()| stdin.flush());
        let mut line = String::new();
        let read = sent.and_then(|()| stdout.read_line(&mut line));
        if !matches!(read, Ok(n) if n > 0) {
            return Err(self.stopped());
        }
        serde_json::from_str(line.trim_end()).map_err(|e| EngineError::Malformed {
            what: "recognizer answer".into(),
            detail: e.to_string(),
        })
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if let Some((mut child, stdin, _)) = self.serving.take() {
            drop(stdin);
            let _ = child.wait();
        }
    }
}
