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

//! Bytes that state no format at all, refused for the cause they actually have (v2-S10).
//!
//! # This file is the argument, mechanized
//!
//! **S10 ships no CSV reader.** The refusal is the slice, and its honesty is not a sentence in a
//! doc — it is one assertion here: **a `.csv` and a letter containing a shopping list receive
//! byte-identical stderr**. That can only pass if nothing sniffed. Count a comma, measure a line,
//! read an extension, and the two messages part company and this file goes red.
//!
//! It is the assertion no detector-shaped implementation can satisfy, which is why it is written
//! from both directions: prose with commas, prose without, a log line with a uniform comma count,
//! and the `.csv` itself. Each test drives the real binary over a real file's own bytes.
//!
//! # Why no reader, when a CSV parse fabricates nothing
//!
//! Not because a one-column parse of prose would invent content — **it would not**. Every field's
//! `text` would be bytes genuinely present in the stream, every record ordinal a true line count,
//! and `pages: []` simply true. None of **L30**'s invented pagination is present.
//!
//! **Exactly one field would be false.** `SourceIdentity.media_type` would say `text/csv` about a
//! file nobody measured to be one — an invented identifier, which standing rule 4 forbids — and
//! `SourceIdentity` is `deny_unknown_fields` with two fields and **no room to say "asserted"**. So
//! a caller-supplied `--format csv` would not break **A4**'s rule; it would break A4's *guarantee*,
//! with nowhere in the artifact to record that it had. That is `docs/history/13-V12-MILESTONES.md`'s
//! v1.2-S5 finding word for word — *an identity that can be asserted is an identity that can
//! disagree with what it describes* — which was decisive enough there to refuse a whole
//! integration.
//!
//! It settles the follow-on question too. If a caller asserted CSV and the bytes were not CSV,
//! **the engine could not tell**: that is definitionally what "no detector" means, so a wrong
//! assertion would always produce a successful artifact, and fail-closed is not reachable from
//! inside that design.

use std::path::{Path, PathBuf};
use std::process::Command;

fn tempdir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "ethos-no-format-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&path).expect("scratch directory");
    path
}

fn extract(path: &Path) -> (i32, Vec<u8>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["extract", path.to_str().expect("utf-8 path")])
        .output()
        .expect("the engine binary runs");
    (
        out.status.code().unwrap_or(-1),
        out.stdout,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Write `bytes` to `name` and refuse it, returning stderr.
///
/// The file is written and then **read back through the binary**, so every assertion below is
/// about a real file's own bytes rather than about a string this test kept in hand.
fn refuse(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write the file");
    let (code, stdout, stderr) = extract(&path);
    assert_eq!(code, 2, "{name}: a refusal exits 2 — {stderr}");
    assert!(stdout.is_empty(), "{name}: a refusal prints no artifact");
    stderr
}

/// The four shapes, and the point of each.
///
/// `rows.csv` is the format this slice is named for. `letter.txt` is prose that **contains
/// commas**, including a shopping list, so a comma-counting detector would claim it. `plain.txt`
/// is prose with none, so a detector using commas as a floor would reject the `.csv` beside it for
/// the same reason. `app.log` has a **uniform comma count per line**, which is the one property a
/// naive CSV detector is usually built on — and it is a log.
const NO_FORMAT: [(&str, &[u8]); 4] = [
    ("rows.csv", b"name,role\nAda,engineer\nGrace,admiral\n"),
    (
        "letter.txt",
        b"Dear Ada,\n\nOn your way home, please pick up milk, eggs, bread, and coffee.\n\nYours, Grace\n",
    ),
    (
        "plain.txt",
        b"The engine reads what a file states about itself and refuses to guess the rest.\n",
    ),
    (
        "app.log",
        b"2026-08-20T10:00:00Z,INFO,started\n2026-08-20T10:00:01Z,WARN,slow\n2026-08-20T10:00:02Z,INFO,done\n",
    ),
];

// -------------------------------------------------------------------------------------------
// The whole slice, in one assertion
// -------------------------------------------------------------------------------------------

/// **A `.csv` and a letter containing a shopping list receive byte-identical stderr.**
///
/// This is the proof that nothing sniffed, and it is the assertion a detector cannot satisfy: any
/// implementation that measured commas, counted fields, compared line shapes or read the extension
/// would have something different to say about one of these two files, and would say it here.
///
/// The other two shapes are in the same comparison for the same reason from the other side —
/// prose with no commas at all, and a log whose comma count is perfectly uniform.
#[test]
fn every_file_that_states_no_format_is_refused_in_byte_identical_words() {
    let dir = tempdir();

    let refusals: Vec<(&str, String)> = NO_FORMAT
        .iter()
        .map(|(name, bytes)| (*name, refuse(&dir, name, bytes)))
        .collect();

    let (first_name, first) = &refusals[0];
    for (name, stderr) in &refusals[1..] {
        assert_eq!(
            stderr, first,
            "`{name}` and `{first_name}` must be refused in the same words to the byte. A \
             difference here means something measured one of them, which is the detector this \
             slice refused to write."
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The refusal names what was **looked for**, and never a format it did not measure.
#[test]
fn the_refusal_names_what_was_looked_for_and_claims_no_format() {
    let dir = tempdir();
    let stderr = refuse(&dir, "rows.csv", NO_FORMAT[0].1);

    assert!(
        stderr.contains("these bytes state no format this engine reads"),
        "the cause, stated: {stderr}"
    );
    for looked_for in ["ZIP local file header", "RTF brace group", "PDF header"] {
        assert!(
            stderr.contains(looked_for),
            "the refusal must name {looked_for} as something looked for: {stderr}"
        );
    }
    assert!(
        stderr.contains("not the file's name"),
        "**A4**: no extension was read, and the message says so: {stderr}"
    );
    assert!(
        stderr.contains("no signature at any other offset"),
        "`MAX_HEADER_OFFSET` is zero and the message says so: {stderr}"
    );

    // And it claims nothing about what the file IS. Naming a format nobody measured is the
    // invented identifier standing rule 4 forbids — `SourceIdentity` has no room to record that a
    // type was asserted rather than read.
    for claim in ["text/csv", "CSV", "comma", "spreadsheet", "plain text"] {
        assert!(
            !stderr.contains(claim),
            "the refusal must not name a format it did not measure, and it named {claim}: {stderr}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// **The wrong cause is gone.** These bytes never aimed at the PDF reader.
///
/// The defect v2-S6 fixed for an `.ods`, v2-S8 for an `.rtf` and for the ZIP *shape*, and this
/// slice for the last member of that shape: bytes carrying no signature, no container and no
/// declaration were handed to the PDF reader anyway and refused for a header they never claimed.
#[test]
fn no_file_that_states_no_format_is_told_it_is_a_broken_pdf() {
    let dir = tempdir();
    for (name, bytes) in NO_FORMAT {
        let stderr = refuse(&dir, name, bytes);
        assert!(
            !stderr.contains("%PDF-"),
            "{name} is not a broken PDF and must not be told it is: {stderr}"
        );
        assert!(
            !stderr.contains("expected a PDF header"),
            "{name}: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The edge that must NOT move
// -------------------------------------------------------------------------------------------

/// **A truncated PDF keeps the PDF reader's own message, down to the zero-byte file.**
///
/// A file that is a proper prefix of `%PDF-` **did** aim at this reader — a PDF cut short in
/// transit is a PDF whose length is the story — so *"file is 3 bytes, shorter than the 5-byte PDF
/// header"* is its honest cause, and displacing it would be the same wrong-cause defect pointing
/// the other way. The zero-byte case is the sharpest: nothing about it says PDF, and nothing about
/// it says anything else either.
#[test]
fn a_truncated_pdf_still_gets_the_pdf_readers_own_message() {
    let dir = tempdir();

    for (name, bytes) in [
        ("empty.pdf", &b""[..]),
        ("one.pdf", b"%"),
        ("two.pdf", b"%P"),
        ("three.pdf", b"%PD"),
        ("four.pdf", b"%PDF"),
    ] {
        let stderr = refuse(&dir, name, bytes);
        assert!(
            stderr.contains("shorter than the 5-byte PDF header"),
            "{name} aimed at the PDF reader and must keep its message: {stderr}"
        );
        assert!(
            stderr.contains(&format!("file is {} bytes", bytes.len())),
            "{name} must still be told its own length: {stderr}"
        );
        assert!(
            !stderr.contains("state no format"),
            "{name} is not the no-format case: {stderr}"
        );
    }

    // And one byte short of the header that is NOT a prefix of it takes the other branch, because
    // nothing about it aimed here.
    let stderr = refuse(&dir, "%pdx.bin", b"%PDX");
    assert!(
        stderr.contains("state no format"),
        "`%PDX` states no format and was never aimed at the PDF reader: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A real PDF is untouched by the branch: it still reaches the reader and still extracts.
#[test]
fn a_pdf_still_reaches_the_pdf_reader() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/engine/absent-font-metrics/document.pdf");
    let (code, stdout, stderr) = extract(&path);
    assert_eq!(code, 0, "a real PDF still extracts: {stderr}");
    assert!(!stdout.is_empty(), "and it prints an artifact");

    // And a file that claims the header but is broken behind it still reaches the PDF reader,
    // which is the branch's other side: the header is what routes, not the parse.
    let dir = tempdir();
    let broken = refuse(&dir, "broken.pdf", b"%PDF-1.7\nnothing else at all\n");
    assert!(
        !broken.contains("state no format"),
        "a `%PDF-`-headed file is the PDF reader's to refuse: {broken}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// The exit contract, and the divergence this slice did not close
// -------------------------------------------------------------------------------------------

/// Exit **2**, code `unsupported`, empty stdout — unchanged from before this slice.
///
/// Only the **cause** moved. A caller routing on the exit code or on the machine-readable code
/// sees exactly what it saw at 0.28.1.
#[test]
fn the_exit_contract_did_not_move_with_the_message() {
    let dir = tempdir();
    for (name, bytes) in NO_FORMAT {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write the file");
        let (code, stdout, stderr) = extract(&path);
        assert_eq!(code, 2, "{name}");
        assert!(stdout.is_empty(), "{name}: a refusal prints no artifact");
        assert!(
            stderr.trim_end().ends_with("[unsupported]"),
            "{name}: the machine-readable code is still `unsupported`: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// **`ethos-parser classify` did NOT move with `ethos-parser extract`, and that is stated rather than left
/// to be discovered.**
///
/// `classify` never reaches the office router at all — it opens the file with the PDF reader
/// directly — so a `.csv` handed to it is still refused for having no `%PDF-` header. This slice
/// did not change that, deliberately: `classify` is the **PDF classifier**, and a caller who ran
/// it named the PDF reader by naming the subcommand, which is the same argument that keeps the
/// truncated-PDF message where it is. The divergence is recorded in `docs/history/15-V2-MILESTONES.md`
/// S10 rather than closed in passing.
///
/// This test exists so the divergence cannot quietly change in either direction without a note.
#[test]
fn classify_still_answers_as_the_pdf_classifier_and_the_divergence_is_named() {
    let dir = tempdir();
    let path = dir.join("rows.csv");
    std::fs::write(&path, NO_FORMAT[0].1).expect("write the file");

    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["classify", path.to_str().expect("utf-8 path")])
        .output()
        .expect("the engine binary runs");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(2), "still exit 2");
    assert!(out.stdout.is_empty(), "and no artifact");
    assert!(
        stderr.contains("%PDF-"),
        "`classify` is the PDF classifier and still names the PDF header. If this changed, say so \
         in docs/history/15-V2-MILESTONES.md S10 rather than here: {stderr}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// Guard the guard
// -------------------------------------------------------------------------------------------

/// The one place a file name is legitimately read, **named rather than skipped**.
///
/// `thresholds.rs` ends in its own `#[cfg(test)]` guard that walks this same tree and filters
/// `.rs` files by extension. That is test code *about source files*, not a detection path.
///
/// It is an exemption by (file, token) rather than a region skip, and the difference matters. The
/// first version of this test skipped everything after each file's first `#[cfg(test)]`, which
/// looked equivalent and was not: `ethos-parser-pdf/src/lib.rs` carries a `#[cfg(test)]` at line 56, so
/// the skip hid that crate's **entire public re-export block** from the scan — including the very
/// export this slice added. A guard that passes because it read nothing is the vacuous shape this
/// repository has been caught by before.
const NAME_READING_EXEMPTIONS: [(&str, &str); 2] = [
    ("thresholds.rs", ".file_name()"),
    ("thresholds.rs", ".extension()"),
];

/// **No detection path in this engine reads a file name, and no signature is scanned for.**
///
/// The refusal above is only honest if the code behind it stayed honest. This reads the source
/// rather than the output, because a message can be identical for two files while the code that
/// produced it consulted something it should not have and happened to agree.
///
/// Every line of every `src` file is scanned — comment prose excluded, since naming the trap is
/// expected and welcome. Real occurrences are exempted one at a time by [`NAME_READING_EXEMPTIONS`].
#[test]
fn no_source_file_routes_on_a_name_or_sniffs_a_csv() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("crates");

    let mut checked = 0usize;
    let mut used_exemptions = 0usize;
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("readable directory") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                // Only `src`, never `tests`: a test may legitimately build a path from a name.
                if path.file_name().is_some_and(|n| n == "tests") {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("readable source");
            let display = path.display().to_string();
            let base = path
                .file_name()
                .expect("a file")
                .to_str()
                .expect("utf-8 file name");

            for (number, line) in source.lines().enumerate() {
                // Prose about the trap is expected and welcome; code is not.
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for forbidden in [
                    "is_csv",
                    "text/csv",
                    ".extension()",
                    ".file_name()",
                    ".file_stem()",
                ] {
                    if !line.contains(forbidden) {
                        continue;
                    }
                    if NAME_READING_EXEMPTIONS.contains(&(base, forbidden)) {
                        used_exemptions += 1;
                        continue;
                    }
                    panic!(
                        "{display}:{} contains `{forbidden}`. **A4**: a file name is a claim \
                         anybody can make, and a format this engine cannot measure is refused \
                         rather than guessed at. If this is genuinely test code about source \
                         files, add it to NAME_READING_EXEMPTIONS by name.",
                        number + 1
                    );
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 40, "the walk found only {checked} source files");
    assert_eq!(
        used_exemptions,
        NAME_READING_EXEMPTIONS.len(),
        "an exemption stopped matching. A stale exemption is a hole nobody is watching — delete \
         it in the same commit that removes what it covered."
    );
}
