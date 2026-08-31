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

//! **Every entry point read the whole file before checking anything** (v2-S15).
//!
//! `std::fs::read` on a caller-supplied path, at six sites in `main.rs` and two in `mcp.rs`, with
//! no size ceiling anywhere — so the file was resident before any check beyond the five-byte
//! magic number. The office crate has had `zip::MAX_INFLATED_BYTES` since v2-S13 for exactly this
//! reason; the path that reads the file in the first place had nothing.
//!
//! The refusal has to come from `metadata`, not from the read: checking after reading would have
//! to allocate the thing it means to refuse, which is the same defect
//! `zip::read_entry`'s `Vec::with_capacity(declared)` had.
//!
//! **The fixture is sparse**, so this suite asserts a 3 GiB refusal while occupying no disk.
//! `set_len` past the end of a file allocates nothing on APFS or ext4; the bytes are never
//! written and never read, which is the whole point — a ceiling that must be reached to be tested
//! would be a ceiling nobody tests.

use std::path::PathBuf;
use std::process::{Command, Output};

/// Matches `MAX_SOURCE_BYTES` in `main.rs`. Deliberately duplicated rather than imported: this is
/// an integration test over the BINARY, and a constant read from the crate under test would agree
/// with itself even if the binary stopped enforcing it.
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn scratch(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "ethos-parser-source-ceiling-{}-{name}",
        std::process::id()
    ));
    p
}

/// A file that CLAIMS `len` bytes and occupies none of them.
fn sparse(name: &str, len: u64) -> PathBuf {
    let path = scratch(name);
    let f = std::fs::File::create(&path).expect("create the sparse fixture");
    f.set_len(len)
        .expect("set_len past the end is a sparse extend");
    assert_eq!(
        std::fs::metadata(&path).expect("stat").len(),
        len,
        "the fixture must report the size it claims, or this test proves nothing"
    );
    path
}

fn extract(path: &PathBuf) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("extract")
        .arg(path)
        .output()
        .expect("the engine binary runs")
}

/// **An oversized file is refused by name, and refused without being read.**
#[test]
fn a_file_over_the_ceiling_is_a_named_resource_limit() {
    let path = sparse("over", MAX_SOURCE_BYTES + 1);
    let out = extract(&path);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_file(&path);

    assert_eq!(
        out.status.code(),
        Some(2),
        "an unreadable source is exit 2; stderr: {stderr}"
    );
    assert!(
        stderr.contains("resource_limit"),
        "the refusal must carry the `resource_limit` code so a caller can route it, rather than \
         arriving as an OOM kill with no stderr at all; got: {stderr}"
    );
    assert!(
        stderr.contains(&MAX_SOURCE_BYTES.to_string()),
        "the refusal must name the ceiling it enforced, or the caller cannot tell how far over \
         the file was; got: {stderr}"
    );
}

/// The ceiling must not have made ordinary documents unreadable — the failure mode of a bound
/// that is measured in the wrong unit or applied to the wrong thing.
#[test]
fn an_ordinary_document_is_unaffected() {
    let m: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .unwrap()
                .join("fixtures/manifest.json"),
        )
        .expect("manifest"),
    )
    .expect("valid JSON");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join(m["roots"]["gate"]["default"].as_str().unwrap());

    let out = extract(&root.join("irs-fw9.pdf"));
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A file exactly AT the ceiling is admitted. The comparison is `>`, and an off-by-one here would
/// refuse a legal document for being precisely the size the ceiling permits.
///
/// It is admitted and then refused for what it actually is — 2 GiB of zeroes is not a PDF — which
/// is the correct cause and a different one from the resource limit above.
#[test]
fn a_file_exactly_at_the_ceiling_passes_the_size_check() {
    let path = sparse("at", MAX_SOURCE_BYTES);
    let out = extract(&path);
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_file(&path);

    assert!(
        !stderr.contains("resource_limit"),
        "a file exactly at the ceiling is inside it; the check is `>`, not `>=`. Got: {stderr}"
    );
}
