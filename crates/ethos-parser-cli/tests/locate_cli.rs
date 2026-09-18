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

//! `ethos-parser locate` — the subcommand, its exit codes, and the bytes it prints
//! (`docs/26-LOCATE-SCOPE.md` §6.1, D1 S2).
//!
//! # What this file is for
//!
//! Three properties the library tests cannot reach, because they are the shell's:
//!
//! 1. **The printed bytes are the library's bytes.** `library_surface.rs` proves the artifact is
//!    reachable without the binary; this proves the binary adds nothing to it.
//! 2. **The quote's bytes are the caller's bytes.** A trailing newline, a NUL, non-UTF-8 — each
//!    one is a thing argv could not have carried faithfully, which is why the quote arrives as a
//!    file (scope §6.1).
//! 3. **There is no exit 1, ever.** An exit 1 meaning *not found* is decision #30's own
//!    re-refusal condition, one composition away from `locate … ; if [ $? -eq 1 ]`. So the codes
//!    are asserted over a matrix that includes both answers and every refusal here.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ethos_parser_core::{DocumentRepresentation, Profile};
use serde_json::Value;

// -------------------------------------------------------------------------------------------
// Harness
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

/// A directory of its own per test, removed when the test ends, including when it panics.
struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(label: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-locate-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    Scratch(dir)
}

/// A fixture's representation on disk, as the CLI writes it.
fn represented(dir: &Path, fixture: &str) -> PathBuf {
    let pdf = repo_root().join(format!("fixtures/engine/{fixture}/document.pdf"));
    assert!(pdf.is_file(), "fixture missing: {}", pdf.display());
    let out = engine(&["extract", pdf.to_str().expect("utf-8")]);
    assert_eq!(out.status.code(), Some(0), "the fixture extracts");
    let path = dir.join("representation.json");
    std::fs::write(&path, &out.stdout).expect("write");
    path
}

/// A file holding exactly these bytes, and no others.
fn quote_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write");
    path
}

fn locate(repr: &Path, quote: &Path) -> Output {
    engine(&[
        "locate",
        repr.to_str().expect("utf-8"),
        "--quote-file",
        quote.to_str().expect("utf-8"),
    ])
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

// -------------------------------------------------------------------------------------------
// The shell adds nothing
// -------------------------------------------------------------------------------------------

/// **The CLI prints the library's bytes** — the thin-shell rule, for this subcommand.
///
/// `docs/history/05-MILESTONES.md` M7: every subcommand behaviour is reachable through the
/// library. `library_surface.rs` proves the reachability without a process; this proves the other
/// half, that the process contributes no fixup, default or reordering of its own. Together they
/// are not circular: either alone is.
#[test]
fn the_cli_prints_the_bytes_the_library_produces() {
    let dir = scratch("equals-library");
    let repr_path = represented(&dir, "untagged-shredded-line");
    let quote = quote_file(&dir, "q.txt", b"arrow");

    let out = locate(&repr_path, &quote);
    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));

    let repr: DocumentRepresentation =
        serde_json::from_slice(&std::fs::read(&repr_path).expect("read")).expect("parses");
    let profile = Profile::default();
    let from_library = ethos_parser_core::locate(
        &repr,
        &profile.parser_version,
        &profile.profile_sha256().expect("hashes"),
        &profile.locate_rule,
        "arrow",
    )
    .expect("the library answers")
    .to_canonical_bytes()
    .expect("canonicalizes");

    let mut expected = from_library;
    expected.push(b'\n');
    assert_eq!(
        out.stdout, expected,
        "stdout is the library's canonical bytes and one newline, nothing else"
    );
}

// -------------------------------------------------------------------------------------------
// The quote's bytes are the caller's bytes
// -------------------------------------------------------------------------------------------

/// **Read verbatim: no trim, no trailing-newline strip.**
///
/// Stripping one trailing LF would make `printf %s` and `echo` agree and would make a quote that
/// genuinely ends in a newline unaskable — a silent edit of the caller's input. The two calls
/// below differ by exactly that byte and must differ in their answer.
#[test]
fn the_quote_file_is_read_verbatim() {
    let dir = scratch("verbatim");
    let repr = represented(&dir, "untagged-shredded-line");

    let bare = locate(&repr, &quote_file(&dir, "bare.txt", b"Yar"));
    let with_lf = locate(&repr, &quote_file(&dir, "lf.txt", b"Yar\n"));
    assert_eq!(bare.status.code(), Some(0), "{}", stderr_of(&bare));
    assert_eq!(with_lf.status.code(), Some(0), "{}", stderr_of(&with_lf));

    let count = |out: &Output| -> usize {
        let v: Value = serde_json::from_slice(&out.stdout).expect("canonical JSON");
        v["occurrences"].as_array().expect("occurrences").len()
    };
    let scalars = |out: &Output| -> u64 {
        let v: Value = serde_json::from_slice(&out.stdout).expect("canonical JSON");
        v["quote_scalars"].as_u64().expect("a count")
    };

    assert_eq!(count(&bare), 1, "`Yar` is on the page");
    assert_eq!(scalars(&bare), 3);
    assert_eq!(
        count(&with_lf),
        0,
        "`Yar\\n` is not: the run holds no newline, and the trailing byte was not trimmed away"
    );
    assert_eq!(scalars(&with_lf), 4, "the newline is one of the scalars");
}

/// **A NUL and a newline both reach the engine intact** — the test argv could not have passed.
///
/// A NUL cannot appear in an argument at all and a newline survives only through correct quoting,
/// which is scope §6.1's decisive reason for the file. Neither string is in the document, so both
/// answers are empty; what is being asserted is that the engine received four scalars and three
/// scalars respectively and said so, rather than receiving a truncated string silently.
#[test]
fn a_quote_holding_a_nul_or_a_newline_survives_the_file() {
    let dir = scratch("nul");
    let repr = represented(&dir, "untagged-shredded-line");

    for (name, bytes, scalars) in [
        ("nul.txt", b"Yar\0w".to_vec(), 5u64),
        ("nl.txt", b"Yar\nrow".to_vec(), 7),
    ] {
        let out = locate(&repr, &quote_file(&dir, name, &bytes));
        assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
        let v: Value = serde_json::from_slice(&out.stdout).expect("canonical JSON");
        assert_eq!(
            v["quote_scalars"].as_u64(),
            Some(scalars),
            "the whole string arrived: {name}"
        );
        assert_eq!(v["occurrences"].as_array().expect("array").len(), 0);
    }
}

/// Non-UTF-8 quote bytes are a **named** refusal, and the name says what to do about it.
///
/// A representation's text is a Rust `String`, so a byte sequence that is not UTF-8 cannot occur
/// in one and there is nothing to search for. The refusal states the offset rather than the whole
/// file, which is the difference between a diagnosable exit 2 and "invalid input".
#[test]
fn non_utf8_quote_bytes_are_a_named_refusal() {
    let dir = scratch("utf8");
    let repr = represented(&dir, "untagged-shredded-line");
    let out = locate(&repr, &quote_file(&dir, "bad.txt", &[0x59, 0x61, 0xff]));

    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "a refusal prints no artifact");
    let err = stderr_of(&out);
    assert!(err.contains("not UTF-8"), "{err}");
    assert!(err.contains("offset 2"), "the offset is named: {err}");
}

// -------------------------------------------------------------------------------------------
// The refusals, and the two ceilings
// -------------------------------------------------------------------------------------------

/// The empty quote is refused, because *every place the empty string occurs* has no answer with a
/// meaning. **This is not the not-found answer** — the next test is.
#[test]
fn the_empty_quote_is_refused() {
    let dir = scratch("empty");
    let repr = represented(&dir, "untagged-shredded-line");
    let out = locate(&repr, &quote_file(&dir, "empty.txt", b""));

    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(
        stderr_of(&out).contains("the quote is empty"),
        "{}",
        stderr_of(&out)
    );
}

/// **A string the document does not contain is an answer, and its exit code is 0.**
#[test]
fn a_string_the_document_does_not_contain_exits_zero() {
    let dir = scratch("absent");
    let repr = represented(&dir, "untagged-shredded-line");
    let out = locate(&repr, &quote_file(&dir, "q.txt", b"Absent"));

    assert_eq!(out.status.code(), Some(0), "{}", stderr_of(&out));
    let v: Value = serde_json::from_slice(&out.stdout).expect("canonical JSON");
    assert_eq!(v["occurrences"].as_array().expect("array").len(), 0);
    assert_eq!(v["artifact_type"], "ethos.parser.locations.v0");
    assert!(
        v.get("occurrences_withheld").is_none(),
        "nothing was withheld, so the field is absent rather than null"
    );
}

/// The quote ceiling is the read's ceiling, and the boundary is where it is stated to be.
///
/// 16,384 bytes is `ethos.grounding.v1`'s own longest admissible string, so a longer quote is
/// longer than any element text a citation could carry. One byte over is refused **by name**, with
/// the number; one byte under is answered.
#[test]
fn a_quote_file_past_the_ceiling_is_refused_by_name() {
    let dir = scratch("ceiling");
    let repr = represented(&dir, "untagged-shredded-line");
    let ceiling = ethos_parser_core::LOCATE_MAX_QUOTE_BYTES;

    let at = locate(&repr, &quote_file(&dir, "at.txt", &vec![b'a'; ceiling]));
    assert_eq!(at.status.code(), Some(0), "{}", stderr_of(&at));

    let over = locate(
        &repr,
        &quote_file(&dir, "over.txt", &vec![b'a'; ceiling + 1]),
    );
    assert_eq!(over.status.code(), Some(2));
    assert!(over.stdout.is_empty());
    let err = stderr_of(&over);
    assert!(
        err.contains(&ceiling.to_string()),
        "the ceiling is named: {err}"
    );

    // And a file far past it is refused too, without being read: the ceiling on a *source* is 2
    // GiB, and a quote is not a source.
    let huge = locate(&repr, &quote_file(&dir, "huge.txt", &vec![b'a'; 1 << 20]));
    assert_eq!(huge.status.code(), Some(2));
    assert!(stderr_of(&huge).contains(&ceiling.to_string()));
}

/// **The fingerprint is checked before anything is searched** (T16, the CLI's third of it).
///
/// The same-length edit with the declared digest kept is `mcp_stdio.rs`'s `tampered_bytes` shape.
/// A representation whose payload does not hash to its declared digest is not a record this engine
/// will speak for, and answering about it anyway would launder the disagreement into a fresh
/// artifact whose node ids nobody can now confirm.
#[test]
fn a_tampered_representation_is_refused_before_anything_is_searched() {
    let dir = scratch("tampered");
    let good = represented(&dir, "untagged-shredded-line");
    let bytes = std::fs::read(&good).expect("read");

    let at = bytes
        .windows(8)
        .rposition(|w| w == b"\"text\":\"")
        .expect("a text field")
        + 8;
    let mut edited = bytes.clone();
    assert!(edited[at].is_ascii_alphabetic());
    edited[at] ^= 0x20;
    let tampered = dir.join("tampered.json");
    std::fs::write(&tampered, &edited).expect("write");

    let out = locate(&tampered, &quote_file(&dir, "q.txt", b"arrow"));
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty(), "no occurrence is reported");
    let err = stderr_of(&out);
    assert!(
        err.contains("representation_c14n_sha256"),
        "the refusal names the digest that disagreed: {err}"
    );
}

// -------------------------------------------------------------------------------------------
// There is no exit 1
// -------------------------------------------------------------------------------------------

/// **Every outcome is 0 or 2, over every case this file can produce.**
///
/// `extract`'s doc comment gives the general reason — overloading 1 would make a caller's `&&`
/// chain mean two different things depending on which subcommand ran — and here it would make the
/// chain mean a verdict, which is decision #30's re-refusal condition. Asserted as a matrix rather
/// than per test so a future case cannot be added without meeting it.
#[test]
fn no_outcome_of_this_subcommand_is_exit_one() {
    let dir = scratch("codes");
    let repr = represented(&dir, "untagged-shredded-line");
    let missing = dir.join("nope.txt");

    let cases: Vec<(&str, PathBuf)> = vec![
        ("found", quote_file(&dir, "found.txt", b"arrow")),
        ("absent", quote_file(&dir, "absent.txt", b"Absent")),
        ("empty", quote_file(&dir, "empty.txt", b"")),
        ("non-utf8", quote_file(&dir, "bad.txt", &[0xff])),
        (
            "over the ceiling",
            quote_file(
                &dir,
                "over.txt",
                &vec![b'a'; ethos_parser_core::LOCATE_MAX_QUOTE_BYTES + 1],
            ),
        ),
        ("no quote file at all", missing),
    ];

    for (label, quote) in cases {
        let code = locate(&repr, &quote).status.code();
        assert!(
            code == Some(0) || code == Some(2),
            "`{label}` exited {code:?}; this subcommand has no exit 1, ever"
        );
    }

    // And the representation's own failures, for completeness of the matrix.
    let quote = quote_file(&dir, "q.txt", b"arrow");
    for label in ["nope.json", "Cargo.toml"] {
        let code = locate(&repo_root().join(label), &quote).status.code();
        assert_eq!(code, Some(2), "`{label}` must be 2, not 1");
    }
}
