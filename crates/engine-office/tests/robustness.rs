// Copyright 2026 The ethos-engine maintainers
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

//! Office fixture mutation: every package in `fixtures/office/`, mechanically damaged.
//!
//! **A11's other half.** `docs/06-STEAL-REFUSE.md`'s **A11** — *mutation testing every fixture +
//! `cargo-fuzz` per format*, from Anydoc, due at **v0**. v2-S12 closed the fuzz half for office
//! and wrote the mutation half into A11's row as **OPEN**: no office fixture had ever been
//! mutated. This is that half.
//!
//! Same question as `crates/engine-pdf/tests/robustness.rs`, asked of packages instead of a byte
//! stream: damage the *inputs* and assert the engine's response. Not source-level mutation, which
//! asks whether the assertions are load-bearing — a different question, answered far more slowly.
//!
//! # What a mutant is allowed to do
//!
//! Exactly two things, and this suite asserts the boundary between them:
//!
//! | Outcome | Required |
//! | --- | --- |
//! | **Refused** | A named [`EngineError`] with one of the six taxonomy codes. No artifact, no partial output |
//! | **Read** | The artifact binds to the **mutant's** bytes — `source.sha256` is the mutant's digest, never the original's |
//!
//! Never allowed is a third: a panic, or a well-formed artifact that reads as though the original
//! package had been parsed. The first is a crash a caller cannot route; the second is worse,
//! because nothing downstream can tell it happened.
//!
//! # A second harness rather than a second root in the manifest — the decision, and its cost
//!
//! The office fixtures are **not** in `fixtures/manifest.json`, so the v0-M7 harness cannot reach
//! them. Two ways to fix that, and this file is the one that was chosen:
//!
//! **(a) A second harness here, enumerating `fixtures/office/` directly.** What this is. It costs
//! a second implementation of the damage kinds — and most of them had to be rewritten anyway,
//! which is the argument's core rather than a consolation.
//!
//! **(b) An `office` root in `fixtures/manifest.json`, with `robustness.rs` taught to skip
//! non-PDF roots.** Rejected, and not on taste. `all_fixtures()` there walks **every entry of
//! every root** and feeds each to `Document::open_bytes`; adding office entries hands sixteen ZIP
//! and RTF files to the PDF reader, which refuses all of them as `malformed` for having no
//! `%PDF-` header. Every assertion in that file would still pass — nothing panics, no artifact
//! binds wrongly, and an emptied survivor set matches zero survivors — while proving **nothing
//! about any office format**. A green suite that mutated the wrong corpus is a worse outcome than
//! no suite, because it reports coverage that does not exist. It would also edit a v0 harness and
//! a v0-frozen manifest, and the manifest's `counts` tripwire (`conformance_ethos_owned +
//! benchmark + engine_owned`) has no office term, so the sum would silently stop covering them.
//!
//! **The oracle is untouched either way, and that was checked rather than assumed.**
//! `ETHOS_OWNED_FIXTURE_COUNT` (15) and `ORACLE_AGREED_COUNT` (12) live in
//! `crates/engine-cli/tests/oracle.rs` and select fixtures by `owner == "ethos"`, never by root.
//! Option (a) touches no manifest at all, so the question does not arise; the test at the bottom
//! of this file asserts it anyway, because *"it cannot have moved"* is the kind of thing that is
//! true until it is not.
//!
//! # The directory is the manifest
//!
//! `all_fixtures()` reads `fixtures/office/` rather than a list written here. A hardcoded list is
//! exactly what left three OOXML fixtures without a media part until v2-S11 and left the third
//! fuzz target uncompiled until v2-S12.1; the only structure that can notice a seventeenth
//! package is the directory holding it.
//!
//! # Panic containment
//!
//! Each case runs under `catch_unwind` so one panic names its own fixture and mutant instead of
//! taking down the run at the first bad byte. The release profile is `panic = "abort"`, so this
//! is a test-build affordance for triage, never a runtime strategy.

use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use engine_core::EngineError;

// -------------------------------------------------------------------------------------------
// The corpus
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

/// The eight shapes this engine reads, by the extension the fixture author gave the file.
///
/// **This is not a detection path.** `engine_office::read` never sees a name — it is handed bytes
/// and decides from what they contain (**A4**), and a test in `engine-cli` pins that no `src` file
/// anywhere calls `.extension()`. Here the extension does one job the bytes cannot do: it tells
/// the harness that the corpus still holds two of each shape, so a fixture quietly replaced by
/// another copy of a format already covered is caught.
const KNOWN_EXTENSIONS: [&str; 8] = ["docx", "xlsx", "pptx", "odt", "ods", "odp", "rtf", "epub"];

/// One office fixture: the directory that names it, plus its single document.
struct Fixture {
    /// The directory name, which is what the rest of the repository calls this fixture.
    id: String,
    extension: String,
    bytes: Vec<u8>,
}

impl Fixture {
    /// A ZIP container, as opposed to the one format that is not one.
    fn is_zip(&self) -> bool {
        self.bytes.starts_with(b"PK\x03\x04")
    }

    fn is_rtf(&self) -> bool {
        self.extension == "rtf"
    }
}

/// **Every** package under `fixtures/office/`, read from the directory.
///
/// The skip list is `fuzz/seed-corpus.sh`'s, deliberately: that script already enumerates this
/// same corpus to seed `office_read`, and two enumerators of one directory that disagree about
/// what counts is a way for the fuzz corpus and the mutation corpus to drift apart silently.
///
/// `__pycache__` is the one that matters and it is not hypothetical: `make_fixtures.py` sits
/// beside the packages, `__pycache__/` is gitignored, and it exists on a developer's machine
/// while being absent from a fresh CI checkout. A walk that did not skip it would mutate a
/// different number of files locally than in CI.
fn all_fixtures() -> Vec<Fixture> {
    let dir = repo_root().join("fixtures/office");
    let mut out: Vec<Fixture> = Vec::new();

    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{} unreadable: {e}", dir.display()));

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            // `make_fixtures.py` and any README beside it. Not fixtures.
            continue;
        }
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 directory name")
            .to_string();
        if id.starts_with('_') || id.starts_with('.') {
            continue;
        }

        let mut documents: Vec<PathBuf> = std::fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("{} unreadable: {e}", path.display()))
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| KNOWN_EXTENSIONS.contains(&e))
            })
            .collect();
        documents.sort();

        assert_eq!(
            documents.len(),
            1,
            "`{id}` holds {} document(s) with a known office extension; every office fixture is \
             one directory holding exactly one package. A missing file is a failure, never a \
             skip.",
            documents.len()
        );

        let document = &documents[0];
        let extension = document
            .extension()
            .and_then(|e| e.to_str())
            .expect("checked above")
            .to_string();
        let bytes = std::fs::read(document)
            .unwrap_or_else(|e| panic!("{} unreadable: {e}", document.display()));

        out.push(Fixture {
            id,
            extension,
            bytes,
        });
    }

    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

// -------------------------------------------------------------------------------------------
// ZIP field readers
//
// Enough of the container to aim a mutation at a structure rather than at an offset. This is
// deliberately a reader and never a writer: a harness that repacks an archive is authoring a
// fixture, not damaging one, and the two prove different things.
// -------------------------------------------------------------------------------------------

const EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";
const LOCAL_SIGNATURE: &[u8; 4] = b"PK\x03\x04";

fn u16_at(bytes: &[u8], at: usize) -> Option<usize> {
    let raw = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([raw[0], raw[1]]) as usize)
}

fn u32_at(bytes: &[u8], at: usize) -> Option<usize> {
    let raw = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize)
}

/// The offset of the end-of-central-directory record — the **last** one, which is what
/// `zip::find_eocd` takes.
fn eocd_of(archive: &[u8]) -> Option<usize> {
    archive
        .windows(4)
        .rposition(|w| w == EOCD_SIGNATURE.as_slice())
}

/// The entry each format's reader must actually read to produce a document.
///
/// Used to aim a mutation at a part that is **read**, as the counterpart to the one that damages
/// whichever deflated part happens to come first. For the three OOXML packages those two are
/// different files, and that difference is the whole point — see [`Mutation::MainPartByteFlipped`].
///
/// EPUB is the one format where the two coincide: `META-INF/container.xml` is both its first
/// deflated entry and the entry point `epub::read` must resolve. Its spine documents have
/// package-chosen names, so there is no fixed name to aim at, and `container.xml` is the honest
/// stand-in — it is the part whose loss makes the publication unreadable.
const MAIN_PARTS: [&str; 5] = [
    "word/document.xml",
    "xl/workbook.xml",
    "ppt/presentation.xml",
    "content.xml",
    "META-INF/container.xml",
];

/// `(data offset, compressed length)` of the first deflate-compressed entry matching `want`.
///
/// Walks **local** headers from offset 0. The reader under test walks the *central directory*
/// instead, and says why — a local header can exist for an entry the directory does not list. That
/// asymmetry is fine here: this function only needs to find real compressed bytes to damage, and a
/// local header is where those bytes physically are.
fn deflated_entry(archive: &[u8], want: impl Fn(&str) -> bool) -> Option<(usize, usize)> {
    let mut at = 0usize;
    loop {
        if archive.get(at..at + 4)? != LOCAL_SIGNATURE.as_slice() {
            return None;
        }
        let method = u16_at(archive, at + 8)?;
        let compressed = u32_at(archive, at + 18)?;
        let name_len = u16_at(archive, at + 26)?;
        let extra_len = u16_at(archive, at + 28)?;
        let name = std::str::from_utf8(archive.get(at + 30..at + 30 + name_len)?).ok()?;
        let data = at + 30 + name_len + extra_len;

        if compressed == 0 {
            // A data descriptor, so the size is not in this header and the walk cannot continue
            // honestly. None is an answer, not a skip: it is counted and reported.
            return None;
        }
        if method == 8 && want(name) {
            return Some((data, compressed));
        }
        at = data + compressed;
    }
}

fn first_deflated_entry(archive: &[u8]) -> Option<(usize, usize)> {
    deflated_entry(archive, |_| true)
}

fn main_part_entry(archive: &[u8]) -> Option<(usize, usize)> {
    deflated_entry(archive, |name| MAIN_PARTS.contains(&name))
}

/// `(data offset, length)` of the first entry when it is **stored** and named `mimetype`.
///
/// That is the OCF rule ODF and EPUB both follow, and `zip::first_entry` reads exactly this
/// physical layout — the method at byte 8 and the name at 30 — so damaging it is damaging the one
/// place those four formats state what they are.
fn stored_mimetype_body(archive: &[u8]) -> Option<(usize, usize)> {
    if archive.get(..4)? != LOCAL_SIGNATURE.as_slice() {
        return None;
    }
    if u16_at(archive, 8)? != 0 {
        return None;
    }
    let compressed = u32_at(archive, 18)?;
    let name_len = u16_at(archive, 26)?;
    let extra_len = u16_at(archive, 28)?;
    if archive.get(30..30 + name_len)? != b"mimetype".as_slice() {
        return None;
    }
    let data = 30 + name_len + extra_len;
    (compressed > 0).then_some((data, compressed))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
        .filter(|_| !needle.is_empty())
}

/// The first offset of `\\{word}` as a **whole** RTF control word.
///
/// RTF delimits a control word by the first character that is not a letter, so `\\par` and
/// `\\pard` are different words and a plain substring search finds the wrong one. It found the
/// wrong one here first: searching for `\\par` matched the `\\pard` that opens every paragraph in
/// both fixtures, and mangling *that* changes nothing a reader can see, because `\\pard` only
/// resets paragraph properties that were already default.
///
/// This is the same trap `crates/engine-pdf/tests/robustness.rs` records for `find_operator`,
/// where a space-delimited search for ` Tj ` silently missed every fixture writing `(text) Tj\\n`.
/// A mutation that matches the wrong token still applies, still counts, and still proves nothing.
fn find_control_word(haystack: &[u8], word: &[u8]) -> Option<usize> {
    let full: Vec<u8> = std::iter::once(b'\\').chain(word.iter().copied()).collect();
    haystack
        .windows(full.len())
        .enumerate()
        .find_map(|(at, w)| {
            if w != full.as_slice() {
                return None;
            }
            // The delimiter: anything that is not a letter ends the control word.
            match haystack.get(at + full.len()) {
                Some(next) if next.is_ascii_alphabetic() => None,
                _ => Some(at),
            }
        })
}

// -------------------------------------------------------------------------------------------
// Mutations
// -------------------------------------------------------------------------------------------

/// The mechanical damage applied to every fixture.
///
/// Deterministic by construction — no RNG anywhere, for the reason the PDF harness gives: a
/// mutation suite whose inputs vary run to run cannot sit in a repository whose central claim is
/// byte identity, and a survivor set that changed with the weather could not be pinned.
///
/// # Why this is not the PDF harness's six
///
/// A PDF is a byte stream and an office document is mostly a **container**, so the six did not
/// transfer unexamined. Five carry over with their mechanics rewritten around the container, one
/// does not carry over at all, and four are new because the container has hazards a byte stream
/// does not have.
///
/// **`unknown-operator` is deliberately absent, and that is a result rather than an omission.**
/// The PDF kind substitutes a same-length token into a plaintext content stream so `/Length` stays
/// honest and the parse must stop on the *operator*. Every XML part in every package here is
/// deflated; the only stored entry anywhere is `mimetype`. A same-length substitution into
/// compressed bytes cannot reach an XML reader — it fails on inflation or on the length check
/// against the directory entry, both of which [`Mutation::DeflatedPartByteFlipped`] already covers
/// under a name that describes what actually happens. Doing it honestly would mean inflate,
/// substitute, re-deflate and rewrite the CRC and both size fields, which is **authoring a
/// fixture, not damaging one**. Vocabulary-level negatives belong in the per-format suites, which
/// already build packages that way. RTF is the exception and keeps the kind under its own name:
/// its stream is plaintext, so the substitution is trivial and it is the only mutant in this set
/// that produces a wrong-but-plausible artifact instead of a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mutation {
    /// Zero bytes.
    Empty,
    /// The first 16 bytes and nothing else.
    ///
    /// Sharper here than in PDF: sixteen bytes is a partial *local* file header, so
    /// `looks_like_zip` still answers true and the refusal has to come from `find_eocd` rather
    /// than from the first predicate.
    Truncate16,
    /// Everything from the start of the central directory onward, removed.
    ///
    /// The truncated-central-directory case, aimed rather than approximated: the offset comes from
    /// the archive's own end-of-central-directory record, so the cut lands exactly where the
    /// directory begins on every package regardless of its size.
    CentralDirectoryTruncated,
    /// One byte inverted in the last twentieth of the file.
    ///
    /// On packages of one to four kilobytes that lands inside the central directory or the EOCD,
    /// which is `zip.rs`'s main hazard surface. On an RTF stream it lands in trailing text.
    FlipTailByte,
    /// One byte inverted inside the **first** deflate-compressed entry's data.
    ///
    /// The compressed-entry case, aimed at whatever part comes first physically. What that part
    /// *is* differs by family, and the difference turned out to be the finding: for the three
    /// OOXML packages the first deflated entry is `[Content_Types].xml`, which **no reader in this
    /// crate reads** — its only appearance in `crates/engine-office/src` is inside a `docx.rs`
    /// unit test's list of names. Damaging it is therefore invisible, and correctly so.
    ///
    /// Paired with [`Mutation::MainPartByteFlipped`], which aims at a part that *is* read. The two
    /// together are the answer to *"a damaged `[Content_Types].xml` versus a damaged
    /// `word/document.xml`"*: the first is not noticed because nothing reads it, the second is
    /// refused by name.
    FirstDeflatedPartByteFlipped,
    /// One byte inverted inside the deflate-compressed data of the part the reader must read.
    ///
    /// `word/document.xml`, `xl/workbook.xml`, `ppt/presentation.xml`, `content.xml`, or
    /// `META-INF/container.xml` — see [`MAIN_PARTS`].
    ///
    /// **What this found, and it is worth stating rather than only pinning.** The expected outcome
    /// is that inflation fails, or that `zip.rs`'s `out.len() != uncompressed_size` check fires,
    /// and a named refusal follows. That is what happens most of the time. It is not what always
    /// happens: a single flipped byte can leave a deflate stream that still inflates, to *exactly*
    /// the declared length, with **different bytes**. `zip.rs` checks the declared length and
    /// **never verifies the CRC-32** the central directory carries, so the container layer cannot
    /// tell. The corrupted part reaches the XML reader, which either refuses it as malformed XML
    /// or parses it into a different document.
    ///
    /// **That is not a contract violation and this slice does not change a reader over it.** The
    /// artifact binds to the mutant's digest, so nothing claims to be the original — which is
    /// exactly the property `a_surviving_mutant_never_claims_to_be_the_original` exists to hold.
    /// Whether `zip.rs` should verify CRC-32 is a real question about a hand-rolled reader, and it
    /// is recorded in `docs/15-V2-MILESTONES.md` S13 rather than answered here.
    MainPartByteFlipped,
    /// The first four bytes overwritten, so content-based detection has nothing to recognise.
    ///
    /// A4 in office spelling: `PK\x03\x04` for a package, `{\rtf` for a stream.
    HeaderOverwritten,
    /// Trailing junk appended.
    JunkAfterEof,
    /// A second, forged end-of-central-directory record appended, declaring zero entries.
    ///
    /// **No PDF analogue, and the highest-value mutant here.** `zip::find_eocd` takes the
    /// **last** `PK\x05\x06` in the tail window and never validates the record's comment-length
    /// field, so a forged trailing record relocates the whole directory read. This is the mutant
    /// that makes that property visible instead of implied.
    SecondEocdAppended,
    /// The declared media type in a stored `mimetype` entry, overwritten in place.
    ///
    /// The only mutant that reaches the `Unsupported` *"media type"* arm or drops an OCF package's
    /// self-declaration, which is the evidence `is_opendocument` and `is_epub` read.
    MimetypeBodyOverwritten,
    /// `\rtf1` to `\rtf9` — the version digit alone.
    ///
    /// RTF's version gate fires on the first control word only, and overwriting the magic instead
    /// kills `is_rtf` first, so the router refuses before the gate is ever reached. Changing the
    /// one digit is the only way in. This is the closest true analogue the office surface has to
    /// PDF's Table A.1 check.
    RtfVersionBumped,
    /// `\par` to `\pzr` — an unrecognised control word, same length.
    ///
    /// **The wrong-but-plausible one.** An unrecognised RTF control word falls through the reader's
    /// `other =>` arm with no refusal and no declared erasure, so this merges two paragraphs and
    /// shifts every later ordinal. It is the office analogue of PDF's `unknown-operator`, and the
    /// only mutant in this set expected to produce an artifact that is *quietly different* rather
    /// than refused.
    RtfControlWordMangled,
}

impl Mutation {
    const ALL: [Mutation; 12] = [
        Mutation::Empty,
        Mutation::Truncate16,
        Mutation::CentralDirectoryTruncated,
        Mutation::FlipTailByte,
        Mutation::FirstDeflatedPartByteFlipped,
        Mutation::MainPartByteFlipped,
        Mutation::HeaderOverwritten,
        Mutation::JunkAfterEof,
        Mutation::SecondEocdAppended,
        Mutation::MimetypeBodyOverwritten,
        Mutation::RtfVersionBumped,
        Mutation::RtfControlWordMangled,
    ];

    fn name(self) -> &'static str {
        match self {
            Mutation::Empty => "empty",
            Mutation::Truncate16 => "truncate-16",
            Mutation::CentralDirectoryTruncated => "central-directory-truncated",
            Mutation::FlipTailByte => "flip-tail-byte",
            Mutation::FirstDeflatedPartByteFlipped => "first-deflated-part-byte-flipped",
            Mutation::MainPartByteFlipped => "main-part-byte-flipped",
            Mutation::HeaderOverwritten => "header-overwritten",
            Mutation::JunkAfterEof => "junk-after-eof",
            Mutation::SecondEocdAppended => "second-eocd-appended",
            Mutation::MimetypeBodyOverwritten => "mimetype-body-overwritten",
            Mutation::RtfVersionBumped => "rtf-version-bumped",
            Mutation::RtfControlWordMangled => "rtf-control-word-mangled",
        }
    }

    /// Apply the mutation, or return `None` where it does not apply to this input.
    ///
    /// `None` is a real answer and never a quiet skip: a container mutation has nothing to do to
    /// an RTF stream and an RTF mutation has nothing to do to a package. Every one is counted and
    /// pinned in [`EXPECTED_INAPPLICABLE`], because a silent skip is indistinguishable from
    /// coverage.
    fn apply(self, original: &[u8]) -> Option<Vec<u8>> {
        match self {
            Mutation::Empty => Some(Vec::new()),

            Mutation::Truncate16 => {
                if original.len() <= 16 {
                    return None;
                }
                Some(original[..16].to_vec())
            }

            Mutation::CentralDirectoryTruncated => {
                let eocd = eocd_of(original)?;
                let directory_at = u32_at(original, eocd + 16)?;
                if directory_at == 0 || directory_at >= original.len() {
                    return None;
                }
                Some(original[..directory_at].to_vec())
            }

            Mutation::FlipTailByte => {
                if original.len() < 4 {
                    return None;
                }
                // The midpoint of the last twentieth, the same formula the PDF harness uses. On
                // every package in this corpus it lands inside the central directory — verified
                // per fixture rather than assumed, and reported by the coverage test below.
                let idx = original.len() - original.len() / 20 - 1;
                let mut out = original.to_vec();
                out[idx] ^= 0xFF;
                Some(out)
            }

            Mutation::FirstDeflatedPartByteFlipped | Mutation::MainPartByteFlipped => {
                let (data, len) = if self == Mutation::MainPartByteFlipped {
                    main_part_entry(original)?
                } else {
                    first_deflated_entry(original)?
                };
                if len < 4 || data + len > original.len() {
                    return None;
                }
                // The middle of the compressed run: past the deflate block header, so the damage
                // is to the compressed data rather than to the framing.
                let idx = data + len / 2;
                let mut out = original.to_vec();
                out[idx] ^= 0xFF;
                Some(out)
            }

            Mutation::HeaderOverwritten => {
                if original.len() < 8 {
                    return None;
                }
                let mut out = original.to_vec();
                out[..4].copy_from_slice(b"XXXX");
                Some(out)
            }

            Mutation::JunkAfterEof => {
                let mut out = original.to_vec();
                out.extend_from_slice(b"\nthis is not part of any document\n");
                Some(out)
            }

            Mutation::SecondEocdAppended => {
                eocd_of(original)?;
                let mut out = original.to_vec();
                // A minimal, well-formed EOCD: signature, zero disks, zero entries, an empty
                // directory at offset zero, no comment. Well-formed on purpose — a malformed one
                // would be refused for its own shape rather than for relocating the read.
                out.extend_from_slice(EOCD_SIGNATURE);
                out.extend_from_slice(&[0u8; 8]); // disk numbers, entry counts (all zero)
                out.extend_from_slice(&0u32.to_le_bytes()); // directory size
                out.extend_from_slice(&0u32.to_le_bytes()); // directory offset
                out.extend_from_slice(&0u16.to_le_bytes()); // comment length
                Some(out)
            }

            Mutation::MimetypeBodyOverwritten => {
                let (data, len) = stored_mimetype_body(original)?;
                if data + len > original.len() {
                    return None;
                }
                let mut out = original.to_vec();
                // Same length, so the entry still matches its own directory sizes and CRC is the
                // only thing that could complain. The point is a package that is structurally
                // intact and declares a type nothing here reads.
                for byte in &mut out[data..data + len] {
                    *byte = b'x';
                }
                Some(out)
            }

            Mutation::RtfVersionBumped => {
                let at = find(original, b"\\rtf1")?;
                let mut out = original.to_vec();
                out[at + 4] = b'9';
                Some(out)
            }

            Mutation::RtfControlWordMangled => {
                let at = find_control_word(original, b"par")?;
                let mut out = original.to_vec();
                // Same length, so no offset in the stream moves. `\pzr` is not an RTF control
                // word; the reader's `other =>` arm swallows it.
                out[at + 2] = b'z';
                Some(out)
            }
        }
    }
}

// -------------------------------------------------------------------------------------------
// The outcome of one mutant
// -------------------------------------------------------------------------------------------

/// The six codes `EngineError::code()` can return. A refusal outside this set is not named.
///
/// **Only four are reachable from `engine-office`** — `malformed`, `unsupported`,
/// `resource_limit` and `missing_part`. `EngineError::Encrypted` is constructed nowhere in the
/// crate (encryption is refused as `Unsupported`, by name: *"encrypted OpenDocument package"*,
/// *"encrypted EPUB spine document"*), and `Io` belongs to the caller that read the file. The set
/// asserted against is still all six, because the contract is *"one of the taxonomy"* rather than
/// *"one of the four this crate happens to use today"*.
const TAXONOMY: [&str; 6] = [
    "unsupported",
    "malformed",
    "encrypted",
    "resource_limit",
    "missing_part",
    "io",
];

/// What a mutant that was **read** produced.
#[derive(Debug)]
struct Read {
    /// The digest the artifact bound itself to.
    bound_sha256: String,
    /// How many nodes the representation carries, for the mutants whose interest is that the count
    /// moved rather than that the read was refused.
    nodes: usize,
    /// Every node's text, concatenated. Enough to answer *"did the appended bytes become document
    /// text"* without this harness growing an opinion about node shape.
    text: String,
}

/// Run a mutant all the way through, and describe what happened.
///
/// One call rather than the PDF path's four stages: `engine_office::read` is the whole public
/// surface, and it seals the representation itself. Re-deriving the fingerprint and canonicalizing
/// afterwards are nearly free and turn *"it produced something"* into *"it produced something
/// internally consistent"* — and they put the c14n encoder on the path, which is where a string a
/// package supplied is range-checked one last time. That is the same pair `fuzz_targets/
/// office_read.rs` runs, on purpose: the fuzzer and the mutation harness should not disagree about
/// what counts as having read a document.
fn run_mutant(bytes: &[u8]) -> Result<Read, EngineError> {
    let repr = engine_office::read(bytes)?;
    repr.verify_fingerprint()?;
    let _ = repr.to_canonical_bytes()?;

    let payload = repr.payload();
    Ok(Read {
        bound_sha256: payload.source.sha256.to_string(),
        nodes: payload.nodes.len(),
        text: payload
            .nodes
            .iter()
            .map(|n| n.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

/// `"<fixture id>/<mutation>"` for every mutant that still parses.
///
/// Pinned rather than merely permitted. A mutant that *starts* surviving is a fail-closed path
/// that stopped firing, and this suite makes that a red test rather than a quiet drift.
///
/// **Every entry is explained, because an unexplained pinned survivor is a tolerated failure.**
/// **Thirty-one** survivors fall into five classes — one of which is now empty — and not one of
/// them is the PDF harness's, which is the strongest argument that a second harness was the right
/// call rather than a second corpus root pointed at the first.
///
/// **It was thirty-six until v2-S14.** The CRC-32 check moved five into refusals: all four of
/// class 5, which is the class this harness was built to produce and is now empty, and the one
/// member of class 4 that class 4's own paragraph had already identified as a class-5 case
/// arriving early. **Five, where the finding as first stated named four** — the fifth was in the
/// prose and not in the count.
///
/// 1. **`junk-after-eof`, on all sixteen — one name, two mechanisms.** On the fourteen packages,
///    `zip::find_eocd` scans backward for the last `PK\x05\x06` in the tail window and every
///    central-directory offset is absolute from the start of file, so appended bytes sit outside
///    everything the archive declares and the package really is intact. On the two RTF streams it
///    is **not** that: RTF has no end-of-file marker, `rtf::read` runs to `stream.len()`, and the
///    appended bytes become document text. Same verdict, different reason, and the RTF half has
///    its own test — *"survived"* and *"survived and grew a paragraph"* are different facts.
///
/// 2. **`rtf-control-word-mangled`, on both RTF streams.** `\pzr` is not a control word, the
///    reader's `other =>` arm swallows it, and two paragraphs merge. Surviving is correct: RTF
///    readers are required to skip words they do not know, and refusing would break every document
///    written by a newer producer. Its own test pins that the node count actually falls, which is
///    what stops this pin from quietly covering a mutation that changed nothing.
///
/// 3. **`flip-tail-byte` on seven, and the seven are exactly where the byte lands on a field
///    nothing reads.** Inspected per fixture rather than assumed, and the split is the result:
///    the mutation lands inside the central directory on every package, and survival is decided
///    entirely by which *field* it hits. `deck-unread-parts` and `workbook-cells` take it in an
///    entry's external-attributes; `presentation-unread-parts` in a CRC-32 `zip.rs` never reads;
///    `sheet-unread-parts` in the uncompressed-size of `meta.xml`, which no reader opens;
///    `workbook-unread-parts` in a modification date; `simple-paragraphs` in the comment-length of
///    the **last** directory entry, where an inflated length can no longer push the walk past a
///    header that still has to be read. `rich-text-paragraphs` is the RTF one: the byte is a space
///    inside cell text, and a flipped space becomes an undecodable byte this engine **declares**.
///    The nine that refuse land on an entry name (non-UTF-8, so `entry_names` refuses by name), a
///    local-header offset, a name length, or the uncompressed size of a part that *is* read.
///
/// 4. **`first-deflated-part-byte-flipped` on six, and the six are one fact.** For all six OOXML
///    packages the first deflated entry is `[Content_Types].xml`, and **no reader in this crate
///    reads it** — its only appearance in `crates/engine-office/src` is inside a `docx.rs` unit
///    test's list of names. Damaging a part nobody opens is invisible, and correctly so.
///
///    **This was seven until v2-S14.** `presentation-pages` was the seventh, and this paragraph
///    already named it as *"class 5 arriving early: its first deflated entry is
///    `META-INF/manifest.xml`, which is read, and the corrupted stream still inflated to its
///    declared length."* The CRC-32 check refuses it for exactly the reason it emptied class 5, so
///    it left with them. The class is now one fact rather than one fact and an exception.
///
/// 5. **`main-part-byte-flipped` — the class this harness existed to produce, and it is now
///    EMPTY.** All fourteen packages refuse.
///
///    It held four until v2-S14. On `deck-slides`, `unread-parts`, `workbook-cells` and
///    `workbook-unread-parts` the flipped byte left a deflate stream that `miniz_oxide` still
///    inflated to **exactly** the declared length, substituting a NUL where an invalid
///    back-reference was — zlib refuses the same bytes, so the permissiveness was the backend's
///    rather than the format's. `zip.rs` compared that length and **never verified the CRC-32**
///    the directory carries, so the corruption reached the XML reader, landed in a namespace URI
///    the OOXML readers match by suffix, and the extracted text came out **byte-identical to the
///    original's**. The artifact differed from an undamaged one in exactly one field —
///    `source.sha256` — which was the designed safety property working with nothing behind it.
///
///    **v2-S14 put something behind it.** `zip::read_entry` now compares the computed CRC-32
///    against the directory's and refuses a mismatch as `Malformed { what: "ooxml part checksum" }`
///    — a distinct `what` from a length or signature failure, so a caller writing policy can tell
///    the causes apart. The refusal shipped only after the false-refusal rate was measured at
///    **zero** across 40 valid packages and 2,370 entries.
///
///    **An emptied class is still pinned, deliberately.** It is named here rather than deleted so
///    that a mutant reappearing in it is read as a regression in the CRC check rather than as a
///    new discovery — the same reason `EXPECTED_INAPPLICABLE` pins what cannot be built.
///
/// An entry appearing here that is not one of those five classes is a fail-closed path that
/// stopped firing — triage it before pinning it. An entry disappearing is a path that started
/// firing, which is usually good and still wants a commit message.
const EXPECTED_SURVIVORS: [&str; 31] = [
    "book-spine/junk-after-eof",
    "book-unread-parts/junk-after-eof",
    "deck-slides/first-deflated-part-byte-flipped",
    "deck-slides/junk-after-eof",
    "deck-unread-parts/first-deflated-part-byte-flipped",
    "deck-unread-parts/flip-tail-byte",
    "deck-unread-parts/junk-after-eof",
    "presentation-pages/junk-after-eof",
    "presentation-unread-parts/flip-tail-byte",
    "presentation-unread-parts/junk-after-eof",
    "rich-text-paragraphs/flip-tail-byte",
    "rich-text-paragraphs/junk-after-eof",
    "rich-text-paragraphs/rtf-control-word-mangled",
    "rich-text-unread-destinations/junk-after-eof",
    "rich-text-unread-destinations/rtf-control-word-mangled",
    "sheet-cells/junk-after-eof",
    "sheet-unread-parts/flip-tail-byte",
    "sheet-unread-parts/junk-after-eof",
    "simple-paragraphs/first-deflated-part-byte-flipped",
    "simple-paragraphs/flip-tail-byte",
    "simple-paragraphs/junk-after-eof",
    "text-paragraphs/junk-after-eof",
    "text-unread-parts/junk-after-eof",
    "unread-parts/first-deflated-part-byte-flipped",
    "unread-parts/junk-after-eof",
    "workbook-cells/first-deflated-part-byte-flipped",
    "workbook-cells/flip-tail-byte",
    "workbook-cells/junk-after-eof",
    "workbook-unread-parts/first-deflated-part-byte-flipped",
    "workbook-unread-parts/flip-tail-byte",
    "workbook-unread-parts/junk-after-eof",
];

/// Mutation/fixture pairs that cannot be built, and why.
///
/// Pinned rather than tolerated, for the same reason the survivors are: a silent skip is
/// indistinguishable from coverage. Each entry names a mutation that has nothing to work with, not
/// one that was found inconvenient.
///
/// - **The five container mutations on the two RTF fixtures** — `central-directory-truncated`,
///   `first-deflated-part-byte-flipped`, `main-part-byte-flipped`, `second-eocd-appended` and
///   `mimetype-body-overwritten`. An RTF stream has no container at all, which is why `read` asks
///   `is_rtf` before it asks anything about ZIPs. **This is the honest form of the observation that
///   the PDF harness's six kinds may reduce to fewer meaningful ones for a format with no
///   container:** they reduce, the reduction is named here rather than papered over, and RTF gains
///   two kinds of its own — `rtf-version-bumped` and `rtf-control-word-mangled` — so the trade is
///   five lost for two gained, and the two gained are the only ones that reach its version gate
///   and its unknown-control-word arm.
/// - **The two RTF mutations on all fourteen packages.** There is no `\rtf1` and no `\par` in a
///   ZIP.
/// - **`mimetype-body-overwritten` on the six OOXML packages.** An OOXML package does not declare
///   its own type; it is told apart by which main part its central directory lists, and its first
///   entry is `[Content_Types].xml`, deflated. There is no stored `mimetype` body to overwrite.
///   That asymmetry between the two container families is what the `claimed` list in
///   `engine_office::read` exists to reconcile, and here it shows up as eight fixtures taking a
///   mutation that six cannot.
const EXPECTED_INAPPLICABLE: [&str; 44] = [
    "book-spine/rtf-control-word-mangled",
    "book-spine/rtf-version-bumped",
    "book-unread-parts/rtf-control-word-mangled",
    "book-unread-parts/rtf-version-bumped",
    "deck-slides/mimetype-body-overwritten",
    "deck-slides/rtf-control-word-mangled",
    "deck-slides/rtf-version-bumped",
    "deck-unread-parts/mimetype-body-overwritten",
    "deck-unread-parts/rtf-control-word-mangled",
    "deck-unread-parts/rtf-version-bumped",
    "presentation-pages/rtf-control-word-mangled",
    "presentation-pages/rtf-version-bumped",
    "presentation-unread-parts/rtf-control-word-mangled",
    "presentation-unread-parts/rtf-version-bumped",
    "rich-text-paragraphs/central-directory-truncated",
    "rich-text-paragraphs/first-deflated-part-byte-flipped",
    "rich-text-paragraphs/main-part-byte-flipped",
    "rich-text-paragraphs/mimetype-body-overwritten",
    "rich-text-paragraphs/second-eocd-appended",
    "rich-text-unread-destinations/central-directory-truncated",
    "rich-text-unread-destinations/first-deflated-part-byte-flipped",
    "rich-text-unread-destinations/main-part-byte-flipped",
    "rich-text-unread-destinations/mimetype-body-overwritten",
    "rich-text-unread-destinations/second-eocd-appended",
    "sheet-cells/rtf-control-word-mangled",
    "sheet-cells/rtf-version-bumped",
    "sheet-unread-parts/rtf-control-word-mangled",
    "sheet-unread-parts/rtf-version-bumped",
    "simple-paragraphs/mimetype-body-overwritten",
    "simple-paragraphs/rtf-control-word-mangled",
    "simple-paragraphs/rtf-version-bumped",
    "text-paragraphs/rtf-control-word-mangled",
    "text-paragraphs/rtf-version-bumped",
    "text-unread-parts/rtf-control-word-mangled",
    "text-unread-parts/rtf-version-bumped",
    "unread-parts/mimetype-body-overwritten",
    "unread-parts/rtf-control-word-mangled",
    "unread-parts/rtf-version-bumped",
    "workbook-cells/mimetype-body-overwritten",
    "workbook-cells/rtf-control-word-mangled",
    "workbook-cells/rtf-version-bumped",
    "workbook-unread-parts/mimetype-body-overwritten",
    "workbook-unread-parts/rtf-control-word-mangled",
    "workbook-unread-parts/rtf-version-bumped",
];

// -------------------------------------------------------------------------------------------
// The suite
// -------------------------------------------------------------------------------------------

/// **No mutant panics, and every refusal is named.**
///
/// The load-bearing test. A panic here is a release blocker; an unnamed refusal is a caller that
/// cannot route the failure, which `docs/03-V0-SCOPE.md` §3.1 exists to prevent. v2-S9's first
/// adversarial finding was a panic in the percent-decoder reachable from a crafted `href`, and the
/// review record says it survived to review precisely because office code was unexercised this
/// way.
#[test]
fn no_mutant_panics_and_every_refusal_is_named() {
    let mut panics: Vec<String> = Vec::new();
    let mut unnamed: Vec<String> = Vec::new();
    let mut cases = 0usize;

    for fixture in all_fixtures() {
        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            cases += 1;
            let label = format!("{}/{}", fixture.id, mutation.name());

            match catch_unwind(AssertUnwindSafe(|| run_mutant(&mutant))) {
                Err(_) => panics.push(label),
                Ok(Err(e)) => {
                    if !TAXONOMY.contains(&e.code()) {
                        unnamed.push(format!("{label}: code `{}` ({e})", e.code()));
                    }
                }
                Ok(Ok(_)) => {}
            }
        }
    }

    // Sixteen fixtures across twelve mutations, less the twenty pinned inapplicable pairs.
    // Derived rather than guessed, and asserted exactly rather than as a floor, because a floor
    // would let a mutation quietly stop applying to half the corpus.
    assert_eq!(
        cases,
        16 * 12 - EXPECTED_INAPPLICABLE.len(),
        "{cases} mutants ran; the matrix says {}. Coverage moved without anyone saying so.",
        16 * 12 - EXPECTED_INAPPLICABLE.len()
    );
    assert!(
        panics.is_empty(),
        "{} mutant(s) panicked. A panic is a release blocker (docs/05-MILESTONES.md M7): the \
         engine must refuse damaged input with a named error, never crash on it.\n  {}",
        panics.len(),
        panics.join("\n  ")
    );
    assert!(
        unnamed.is_empty(),
        "{} mutant(s) were refused outside the six-code taxonomy:\n  {}",
        unnamed.len(),
        unnamed.join("\n  ")
    );
}

/// **A mutant that parses binds to its own bytes, never to the original's.**
///
/// The subtle failure this catches: a reader that opened the mutant but produced an artifact
/// describing the file it was derived from. Nothing downstream — not the oracle, not
/// `grounding-check --source-artifact` — could tell that had happened, because every field would
/// look right.
#[test]
fn a_surviving_mutant_never_claims_to_be_the_original() {
    let mut offenders = Vec::new();
    let mut checked = 0usize;

    for fixture in all_fixtures() {
        let original_digest = format!("sha256:{}", engine_core::sha256_hex_bytes(&fixture.bytes));

        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            let mutant_digest = format!("sha256:{}", engine_core::sha256_hex_bytes(&mutant));

            if let Ok(Ok(Read { bound_sha256, .. })) =
                catch_unwind(AssertUnwindSafe(|| run_mutant(&mutant)))
            {
                checked += 1;
                let label = format!("{}/{}", fixture.id, mutation.name());
                if bound_sha256 == original_digest {
                    offenders.push(format!("{label}: bound to the ORIGINAL digest"));
                } else if bound_sha256 != mutant_digest {
                    offenders.push(format!(
                        "{label}: bound to {bound_sha256}, which is neither original nor mutant"
                    ));
                }
            }
        }
    }

    // The assertion below is vacuous if nothing survives, and almost everything here is expected
    // to be refused. The floor is what keeps this test from passing by proving nothing.
    assert_eq!(
        checked,
        EXPECTED_SURVIVORS.len(),
        "{checked} mutant(s) were read, but {} are pinned as survivors. This test only says \
         something about mutants that parse, so a corpus where nothing parses makes it vacuous.",
        EXPECTED_SURVIVORS.len()
    );
    assert!(
        offenders.is_empty(),
        "an artifact bound to bytes it did not read:\n  {}",
        offenders.join("\n  ")
    );
}

/// **The survivor set is exactly what is pinned.**
///
/// Triage, as a test. A new survivor means a fail-closed path stopped firing on damage it used to
/// catch, and the only way to make this green again is to explain the entry in
/// [`EXPECTED_SURVIVORS`] or fix the path.
#[test]
fn the_surviving_mutants_are_the_pinned_ones() {
    let mut survivors: BTreeSet<String> = BTreeSet::new();

    for fixture in all_fixtures() {
        for mutation in Mutation::ALL {
            let Some(mutant) = mutation.apply(&fixture.bytes) else {
                continue;
            };
            if catch_unwind(AssertUnwindSafe(|| run_mutant(&mutant))).is_ok_and(|r| r.is_ok()) {
                survivors.insert(format!("{}/{}", fixture.id, mutation.name()));
            }
        }
    }

    let expected: BTreeSet<String> = EXPECTED_SURVIVORS.iter().map(|s| s.to_string()).collect();
    let new: Vec<&String> = survivors.difference(&expected).collect();
    let gone: Vec<&String> = expected.difference(&survivors).collect();

    assert!(
        new.is_empty() && gone.is_empty(),
        "the survivor set moved.\n\
         newly surviving (a fail-closed path stopped firing — triage before pinning):\n  {}\n\
         no longer surviving (usually good; still say so in the commit):\n  {}\n\n\
         Full current set, for pasting into EXPECTED_SURVIVORS:\n{}",
        new.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
        gone.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  "),
        survivors
            .iter()
            .map(|s| format!("    \"{s}\","))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// **Appending junk to an RTF stream becomes document text, and that is the finding.**
///
/// Every other format in this corpus treats `junk-after-eof` as bytes nothing reads: a ZIP reader
/// reaches its entries through offsets the central directory states, so anything past the last one
/// is outside the document by construction. **RTF has no end-of-file marker at all.** `rtf::read`
/// loops to `stream.len()`, and whatever follows the outermost `}` is still stream.
///
/// So the mutant does not merely survive — it produces a **different document**, and the artifact
/// says so by binding to the mutant's digest.
///
/// **Which shape the difference takes depends on the fixture, and that was measured rather than
/// assumed.** `rich-text-paragraphs` ends `\\row }`, so its last content is terminated and the
/// appended bytes open a **new** paragraph: six nodes become seven. `rich-text-unread-destinations`
/// ends `are both read.}` with no trailing break, so the appended bytes are **absorbed into the
/// final paragraph**: six nodes stay six and the last node's text grows. An earlier version of
/// this test asserted "exactly one more node" on both and was wrong on the second — the fixture
/// corrected the assertion, which is the right way round.
#[test]
fn appending_junk_to_an_rtf_stream_becomes_document_text() {
    const JUNK: &str = "this is not part of any document";
    let mut checked = 0usize;

    for fixture in all_fixtures().into_iter().filter(Fixture::is_rtf) {
        let original = run_mutant(&fixture.bytes).expect("the fixture reads");
        let mutant = Mutation::JunkAfterEof
            .apply(&fixture.bytes)
            .expect("appending applies to every input");
        let damaged = run_mutant(&mutant).expect("an RTF stream has no EOF to append past");

        assert!(
            !original.text.contains(JUNK),
            "`{}`: the fixture already contains the junk string; this test proves nothing",
            fixture.id
        );
        assert!(
            damaged.text.contains(JUNK),
            "`{}`: appended bytes must become document text — RTF has no end-of-file marker, so \
             `rtf::read` runs to `stream.len()` and the tail is still stream. Text was {:?}",
            fixture.id,
            damaged.text
        );

        // Either shape is correct; what matters is that one of them happened, so a future reader
        // change that silently dropped the tail would be caught.
        let grew_a_node = damaged.nodes == original.nodes + 1;
        let grew_the_last =
            damaged.nodes == original.nodes && damaged.text.len() > original.text.len();
        assert!(
            grew_a_node || grew_the_last,
            "`{}`: appended bytes should either open a new paragraph or extend the final one; \
             nodes went {} -> {} and text {} -> {} bytes",
            fixture.id,
            original.nodes,
            damaged.nodes,
            original.text.len(),
            damaged.text.len()
        );
        assert_ne!(
            damaged.bound_sha256, original.bound_sha256,
            "`{}`: the document changed, so it must not answer to the original's digest",
            fixture.id
        );
        checked += 1;
    }

    assert_eq!(
        checked, 2,
        "{checked} RTF fixture(s) exercised; the corpus holds two and this test says nothing \
         without them"
    );
}

/// **An unrecognised RTF control word is swallowed, and the document silently changes.**
///
/// The office analogue of PDF's injected unknown operator, and the answer is the opposite one.
/// PDF's interpreter stops on a token outside Table A.1 and refuses as `unsupported`. RTF's reader
/// ends its `match` on an `other =>` arm that swallows the word, so `\\par` becoming `\\pzr` loses a
/// paragraph break: two paragraphs merge, every later ordinal shifts, and the result is a
/// **plausible artifact for a stream nobody wrote** — no refusal, no declared erasure.
///
/// That is not a defect to fix here. An unknown control word genuinely is not a malformed stream,
/// and RTF readers are required to skip what they do not know; refusing would break every valid
/// document written by a newer producer. It is a property worth pinning, because it is one of only
/// two places in this corpus where damage is neither refused nor visible in the output's shape,
/// and the only thing standing between it and a silent wrong answer is that the artifact binds to
/// the bytes it actually read.
#[test]
fn an_unrecognised_rtf_control_word_is_swallowed_and_merges_two_paragraphs() {
    let mut checked = 0usize;

    for fixture in all_fixtures().into_iter().filter(Fixture::is_rtf) {
        let original = run_mutant(&fixture.bytes).expect("the fixture reads");
        let mutant = Mutation::RtfControlWordMangled
            .apply(&fixture.bytes)
            .expect("both RTF fixtures contain a whole `\\par` control word");
        let damaged = run_mutant(&mutant).expect("an unknown control word is not a refusal");

        assert_eq!(
            damaged.nodes,
            original.nodes - 1,
            "`{}`: mangling one whole `\\par` should merge exactly two paragraphs, so the node \
             count should fall from {} to {} — it is {}. If this reads as though nothing happened, \
             check that the mutation matched `\\par` and not `\\pard`: that is the trap \
             `find_control_word` exists for.",
            fixture.id,
            original.nodes,
            original.nodes - 1,
            damaged.nodes
        );
        assert_ne!(
            damaged.bound_sha256, original.bound_sha256,
            "`{}`: the only thing that makes this safe is that the artifact binds to the bytes it \
             read",
            fixture.id
        );
        checked += 1;
    }

    assert_eq!(
        checked, 2,
        "{checked} RTF fixture(s) exercised; the corpus holds two"
    );
}

/// **Coverage is exact, and reported.**
///
/// Every fixture in the directory is mutated, every mutation that did not apply is named, and the
/// corpus is asserted to still be the shape it claims: sixteen packages, two of each of the eight
/// formats. That last part is what replaces the manifest `counts` tripwire the PDF harness gets
/// for free — office fixtures are in no manifest, so the only cross-check available is the
/// directory against itself.
#[test]
fn every_fixture_is_mutated_and_the_coverage_is_reported() {
    let fixtures = all_fixtures();
    let mut report = Vec::new();
    let mut inapplicable: BTreeSet<String> = BTreeSet::new();
    let mut by_extension: BTreeMap<String, usize> = BTreeMap::new();
    let mut total = 0usize;

    for fixture in &fixtures {
        *by_extension.entry(fixture.extension.clone()).or_default() += 1;

        let mut applied = Vec::new();
        for m in Mutation::ALL {
            match m.apply(&fixture.bytes) {
                Some(_) => applied.push(m.name()),
                None => {
                    inapplicable.insert(format!("{}/{}", fixture.id, m.name()));
                }
            }
        }
        total += applied.len();
        report.push(format!(
            "{:<32} {:<5} {}",
            fixture.id,
            fixture.extension,
            applied.join(", ")
        ));
    }

    println!(
        "office mutation coverage: {} fixtures, {total} mutants",
        fixtures.len()
    );
    for line in &report {
        println!("  {line}");
    }
    println!("  inapplicable ({}):", inapplicable.len());
    for pair in &inapplicable {
        println!("    {pair}");
    }

    assert_eq!(
        fixtures.len(),
        16,
        "found {} office fixture(s); the corpus is sixteen packages, two of each of the eight \
         formats this engine reads. A seventeenth is welcome and is a decision — it wants a line \
         here and, if a mutation cannot apply to it, an entry in EXPECTED_INAPPLICABLE.",
        fixtures.len()
    );
    for extension in KNOWN_EXTENSIONS {
        assert_eq!(
            by_extension.get(extension).copied().unwrap_or(0),
            2,
            "the corpus should hold two `{extension}` fixtures, it holds {}. A format down to one \
             fixture is a format whose second shape stopped being tested.",
            by_extension.get(extension).copied().unwrap_or(0)
        );
    }

    let expected: BTreeSet<String> = EXPECTED_INAPPLICABLE
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(
        inapplicable,
        expected,
        "the inapplicable set moved. A mutation that quietly stops applying is coverage lost \
         without a red test, which is why this is pinned exactly rather than as a floor.\n\n\
         Full current set, for pasting into EXPECTED_INAPPLICABLE:\n{}",
        inapplicable
            .iter()
            .map(|s| format!("    \"{s}\","))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// **The oracle did not move, and this harness is why that needed asserting.**
///
/// The design decision at the top of this file turns on it. Option (b) — an `office` root in
/// `fixtures/manifest.json` — was rejected partly because the manifest drives
/// `ETHOS_OWNED_FIXTURE_COUNT` and the 12 / 3 oracle partition, an **M6 exit criterion**. Option
/// (a) touches no manifest, so the risk is zero by construction; this asserts it anyway, because
/// *"by construction"* is a claim and the manifest is one file away.
///
/// The mechanism worth writing down: the 15 is selected by `owner == "ethos"`, **never by root**.
/// Owner and root happen to correlate perfectly today — every `conformance` entry is `ethos`,
/// every `engine` entry is `engine` — which is precisely why someone adding a root could believe
/// the count is root-scoped and be wrong in a way that only shows up later.
#[test]
fn adding_this_harness_did_not_touch_the_fixture_manifest_or_the_oracle_count() {
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest readable"),
    )
    .expect("manifest is valid JSON");

    // Sorted, because the order `serde_json` yields depends on whether `preserve_order` is
    // active — and that is decided by feature unification across the whole workspace, so this
    // test passed alone and failed under `cargo test --workspace` until it stopped caring.
    let mut roots: Vec<&str> = manifest["roots"]
        .as_object()
        .expect("a roots table")
        .keys()
        .map(String::as_str)
        .collect();
    roots.sort_unstable();
    assert_eq!(
        roots,
        ["benchmark", "conformance", "engine"],
        "the manifest declares {roots:?}. The office corpus is deliberately not a root of it — see \
         this file's header for why adding one would feed sixteen packages to the PDF reader."
    );

    let fixtures = manifest["fixtures"].as_array().expect("a fixtures array");
    let ethos_owned = fixtures
        .iter()
        .filter(|f| f["owner"].as_str() == Some("ethos"))
        .count();
    assert_eq!(
        ethos_owned, 15,
        "the M6 oracle criterion counts exactly the `owner: \"ethos\"` entries and there are \
         {ethos_owned}. If this moved, an exit criterion moved with it."
    );

    // And no office package leaked in under any spelling.
    let office_paths = fixtures
        .iter()
        .filter(|f| {
            f["path"]
                .as_str()
                .is_some_and(|p| !p.to_ascii_lowercase().ends_with(".pdf"))
        })
        .count();
    assert_eq!(
        office_paths, 0,
        "{office_paths} manifest entry/entries do not end in `.pdf`. Every entry in that file is \
         read by `crates/engine-pdf/tests/robustness.rs` through `Document::open_bytes`."
    );
}

// -------------------------------------------------------------------------------------------
// v2-S14 — the CRC-32 check, and that its cause is tellable from the others
// -------------------------------------------------------------------------------------------

/// A part whose stored CRC-32 does not match its bytes is refused **under its own name**.
///
/// The mutation harness above proves the check fires — five mutants moved out of
/// [`EXPECTED_SURVIVORS`] when it landed. It does **not** prove the refusal is
/// *distinguishable*, and that is the half a caller writing policy depends on: "this archive is
/// truncated" and "this part's checksum is wrong" are different facts about a document and a
/// consumer must not have to parse prose to tell them apart.
///
/// So this damages the DECLARED CRC rather than the data — the bytes stay intact and every other
/// check in `read_entry` still passes — and asserts the `what` that comes back.
#[test]
fn a_part_whose_checksum_disagrees_with_its_bytes_is_refused_under_its_own_name() {
    let mut checked = 0usize;

    for fixture in all_fixtures() {
        if !fixture.is_zip() {
            continue; // the two RTF streams have no container and no CRC to disagree with
        }
        let bytes = fixture.bytes.clone();
        let names = engine_office::zip::entry_names(&bytes).expect("readable directory");
        let first = names.first().expect("a package has entries").clone();

        // Locate the first central-directory header and flip one bit of its CRC-32 field, at
        // offset 16. Nothing else moves: the data, both sizes and every offset stay correct.
        let eocd = bytes
            .windows(4)
            .rposition(|w| w == [b'P', b'K', 5, 6])
            .expect("an EOCD");
        let dir_off = u32::from_le_bytes([
            bytes[eocd + 16],
            bytes[eocd + 17],
            bytes[eocd + 18],
            bytes[eocd + 19],
        ]) as usize;
        assert_eq!(
            &bytes[dir_off..dir_off + 4],
            b"PK\x01\x02",
            "{}: the EOCD does not point at a central header",
            fixture.id
        );

        let mut damaged = bytes.clone();
        damaged[dir_off + 16] ^= 0x01;

        let err = engine_office::zip::read_entry(&damaged, &first)
            .expect_err("a part whose declared CRC does not match its bytes must be refused");

        match &err {
            engine_core::EngineError::Malformed { what, detail } => {
                assert_eq!(
                    what, "ooxml part checksum",
                    "{}: a checksum failure must name itself, not borrow the container's `what` \
                     — a caller switching on the cause cannot tell a corrupt part from a \
                     truncated archive if both say `ooxml package`",
                    fixture.id
                );
                assert!(
                    detail.contains("CRC-32"),
                    "{}: the detail must say which check failed, got: {detail}",
                    fixture.id
                );
            }
            other => panic!("{}: expected Malformed, got {other:?}", fixture.id),
        }

        // The undamaged package still reads, so the assertion above is about the CRC and not
        // about some unrelated breakage this edit introduced.
        engine_office::zip::read_entry(&bytes, &first)
            .expect("the undamaged package must still read");

        checked += 1;
    }

    // A floor, because a loop that checked nothing would pass. Fourteen of the sixteen fixtures
    // are ZIP containers; the two RTF streams are skipped above and have no CRC.
    assert_eq!(
        checked, 14,
        "every ZIP-container fixture must have been checked; a shrinking corpus makes this \
         vacuous rather than false"
    );
}

/// A length failure and a checksum failure are **different** named causes.
///
/// The pair is the point. Without this, `read_entry` could name every integrity failure
/// `ooxml part checksum` and the test above would still pass while telling a caller nothing.
#[test]
fn a_length_failure_and_a_checksum_failure_do_not_share_a_name() {
    let bytes = all_fixtures()
        .into_iter()
        .find(|f| f.id == "simple-paragraphs")
        .expect("the simple-paragraphs fixture")
        .bytes;
    let names = engine_office::zip::entry_names(&bytes).expect("readable directory");
    let first = names.first().expect("entries").clone();

    let eocd = bytes
        .windows(4)
        .rposition(|w| w == [b'P', b'K', 5, 6])
        .expect("an EOCD");
    let dir_off = u32::from_le_bytes([
        bytes[eocd + 16],
        bytes[eocd + 17],
        bytes[eocd + 18],
        bytes[eocd + 19],
    ]) as usize;

    // (a) CRC damaged, sizes intact.
    let mut crc_bad = bytes.clone();
    crc_bad[dir_off + 16] ^= 0x01;
    let crc_what = match engine_office::zip::read_entry(&crc_bad, &first) {
        Err(engine_core::EngineError::Malformed { what, .. }) => what,
        other => panic!("expected a Malformed refusal, got {other:?}"),
    };

    // (b) Declared uncompressed size damaged, at offset 24 of the same header.
    let mut len_bad = bytes.clone();
    len_bad[dir_off + 24] ^= 0x01;
    let len_what = match engine_office::zip::read_entry(&len_bad, &first) {
        Err(engine_core::EngineError::Malformed { what, .. }) => what,
        other => panic!("expected a Malformed refusal, got {other:?}"),
    };

    assert_ne!(
        crc_what, len_what,
        "a checksum failure and a length failure must not answer to one name: a caller writing \
         policy on `what` would be unable to distinguish a corrupt part from a mis-declared one"
    );
    assert_eq!(crc_what, "ooxml part checksum");
    assert_eq!(len_what, "ooxml package");
}
