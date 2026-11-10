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

//! The recognizer answers the fixture requests with exactly the bytes `data/expected.jsonl` holds,
//! on whatever CPU runs this: `docs/35-OCR-SCOPE.md` §10.1, run in CI on every supported platform.
//! The requests cover each decode path (a PNG file, a turned PNG stood upright, a gray JPEG stream,
//! an 8-bit Flate stream with a PNG predictor, a 1-bit Flate stream) and a refusal.
//!
//! `expected.jsonl` was written on Apple silicon. To write it again after a deliberate change, run
//! with `ETHOS_OCR_BLESS=1`, and say in the commit why the bytes moved.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

fn data(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

/// Every fixture request through one `serve` process; the answers, one per line.
fn answers() -> Vec<String> {
    let reqs: Vec<Value> =
        serde_json::from_slice(&std::fs::read(data("requests.json")).unwrap()).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_ethos-parser-ocr"))
        .arg("serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut out = Vec::new();
    for r in &reqs {
        let bytes = std::fs::read(data(r["file"].as_str().unwrap())).unwrap();
        let header = serde_json::json!({"ocr_request": 1, "id": r["id"], "image": r["image"],
                                        "orientation": r["orientation"], "bytes": bytes.len()});
        writeln!(stdin, "{header}").unwrap();
        stdin.write_all(&bytes).unwrap();
        stdin.flush().unwrap();
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        out.push(line.trim_end().to_string());
    }
    drop(stdin);
    assert!(child.wait().unwrap().success());
    out
}

/// Whether this CPU is one the recognizer refuses (`main.rs`: x86-64 without AVX2 and FMA).
fn refused_cpu() -> bool {
    #[cfg(target_arch = "x86_64")]
    return !(is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma"));
    #[cfg(not(target_arch = "x86_64"))]
    return false;
}

#[test]
fn the_fixtures_read_the_same_bytes_on_every_cpu() {
    if refused_cpu() {
        // Then the refusal is what is tested: by name, with its own exit code, before any read.
        let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser-ocr"))
            .arg("serve")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert!(String::from_utf8_lossy(&out.stderr).contains("refused: cpu-lacks-avx2-fma"));
        return;
    }
    let got = answers();
    let path = data("expected.jsonl");
    if std::env::var_os("ETHOS_OCR_BLESS").is_some() {
        std::fs::write(&path, got.join("\n") + "\n").unwrap();
    }
    let want: Vec<String> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(&want) {
        let id = serde_json::from_str::<Value>(w).unwrap()["id"].clone();
        assert!(g == w, "{id}: the answer's bytes moved");
    }

    // What the bytes say, so a blessed regression cannot pass unread.
    let by_id = |id: &str| -> Value {
        got.iter()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .find(|v| v["id"] == id)
            .unwrap()
    };
    let texts = |v: &Value| -> Vec<String> {
        v["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["text"].as_str().unwrap().to_string())
            .collect()
    };
    let upright = by_id("png-file");
    assert!(texts(&upright).len() > 20, "a text page reads as lines");
    // A raster stored turned reads as the upright one does, its quads in the stored grid.
    let turned = by_id("png-file-turned");
    assert_eq!(texts(&turned), texts(&upright));
    assert_eq!(
        (turned["width"].clone(), turned["height"].clone()),
        (upright["height"].clone(), upright["width"].clone())
    );
    for id in ["pdf-dct-gray", "pdf-flate-gray-png-up", "pdf-flate-1bit"] {
        let v = by_id(id);
        assert!(
            v["refused"].is_null() && texts(&v).len() > 10,
            "{id} is read"
        );
    }
    let refused = by_id("pdf-jpx-refused");
    assert_eq!(refused["refused"], "filter-not-decoded:JPXDecode");
    assert!(refused["lines"].as_array().unwrap().is_empty());
}
