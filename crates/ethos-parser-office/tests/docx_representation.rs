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

//! **A quote resolves to a node, and nothing on the way there is a page** (v2-S2).
//!
//! `docs/history/15-V2-MILESTONES.md` S2's gate, run against the fixtures this repository authored. Under
//! v2-S1's decision (b) that sentence does **not** mean `ethos.grounding.v1`: it means the run is
//! in the representation, addressed by something the file contains, and reachable by the id the
//! engine minted for it.

use std::path::PathBuf;

use ethos_parser_core::{GeometryPresence, NativeLocator, Profile};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("document.docx");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The gate sentence, executable.** A known phrase is on a node, addressed by part/paragraph/run.
#[test]
fn a_quote_resolves_to_a_node_addressed_by_the_document_itself() {
    let sealed =
        ethos_parser_office::read(&fixture("simple-paragraphs")).expect("the fixture reads");
    let payload = sealed.payload();

    let node = payload
        .nodes
        .iter()
        .find(|n| n.text == "A quote binds to a run")
        .expect("the phrase the fixture was authored around");

    match &node.native_locator {
        NativeLocator::Docx(locator) => {
            assert_eq!(locator.part, "word/document.xml");
            assert_eq!(
                locator.paragraph, 2,
                "the second `<w:p>`, as the part orders them"
            );
            assert_eq!(locator.run, 1, "the first `<w:r>` inside it");
        }
        other => panic!("a DOCX run must carry a DOCX address, not {other:?}"),
    }

    // And the id is a handle: looking it up returns this node and nothing else.
    let found: Vec<_> = payload.nodes.iter().filter(|n| n.id == node.id).collect();
    assert_eq!(found.len(), 1);
    assert!(
        !payload.nodes.iter().any(|n| n.id.as_str() == "s-forged"),
        "an id nothing minted is absent, which is what makes a lookup for it fail closed"
    );
}

/// **Nothing on this path is a page.** The three places §3 said the law would show.
#[test]
fn the_artifact_has_no_pages_no_geometry_and_a_part_for_a_parent() {
    let sealed = ethos_parser_office::read(&fixture("simple-paragraphs")).expect("reads");
    let payload = sealed.payload();

    assert!(
        payload.pages.is_empty(),
        "a DOCX has no page and none was invented"
    );
    assert!(
        !payload.nodes.is_empty(),
        "an empty document would pass this vacuously"
    );

    for (index, node) in payload.nodes.iter().enumerate() {
        assert!(
            node.parent.as_str().starts_with('d'),
            "`{}` is parented by `{}`, which is not a part id",
            node.id,
            node.parent
        );
        assert!(
            matches!(
                sealed.geometry()[index].presence,
                GeometryPresence::Absent(_)
            ),
            "`{}` carries geometry, and there is no page to check it against",
            node.id
        );
    }
}

/// A DOCX artifact and a PDF artifact are **provably non-comparable** (`14-V2-SCOPE.md` §8).
#[test]
fn the_artifact_declares_its_own_profile_and_its_own_media_type() {
    let sealed = ethos_parser_office::read(&fixture("simple-paragraphs")).expect("reads");
    let payload = sealed.payload();

    assert_eq!(
        payload.source.media_type,
        ethos_parser_office::DOCX_MEDIA_TYPE
    );
    assert_eq!(
        payload.identity.profile_sha256,
        Profile::docx_v0().profile_sha256().expect("a digest")
    );
    assert_ne!(
        payload.identity.profile_sha256,
        Profile::default().profile_sha256().expect("a digest"),
        "sharing the PDF profile would make the two artifacts falsely comparable"
    );
}

/// Entities and `xml:space` come through as the document states them, on a real package.
#[test]
fn text_is_what_the_part_says_it_is() {
    let sealed = ethos_parser_office::read(&fixture("simple-paragraphs")).expect("reads");
    let texts: Vec<&str> = sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect();

    assert!(
        texts.contains(&"Rows & columns are S3."),
        "the entity is decoded, not carried as source: {texts:?}"
    );
    assert!(
        texts.contains(&" and never to a page."),
        "the preserved leading space survives: {texts:?}"
    );
}

/// **Two runs, one artifact.** The double-run rule every artifact in this repository is under.
#[test]
fn two_reads_of_one_document_produce_identical_bytes() {
    let bytes = fixture("simple-paragraphs");
    let first = ethos_parser_office::read(&bytes)
        .expect("reads")
        .to_canonical_bytes();
    let second = ethos_parser_office::read(&bytes)
        .expect("reads")
        .to_canonical_bytes();
    assert_eq!(first.expect("canonical"), second.expect("canonical"));
}

// -------------------------------------------------------------------------------------------
// Declared erasure — Anydoc's A14
// -------------------------------------------------------------------------------------------

/// **What was not read is counted and named**, so an absent phrase is not read as absent evidence.
#[test]
fn parts_this_slice_does_not_read_are_declared_with_a_count() {
    let sealed = ethos_parser_office::read(&fixture("unread-parts")).expect("reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("a package with headers declares that it has them");

    assert!(
        limitation.detail.contains('3'),
        "a header, a footer and a footnotes part: {}",
        limitation.detail
    );
    // And the header's text is genuinely absent, which is what makes the declaration necessary
    // rather than decorative.
    assert!(
        !sealed
            .payload()
            .nodes
            .iter()
            .any(|n| n.text.contains("Confidential")),
        "the header was not read, and the artifact says so rather than implying it was"
    );
}

/// The clean fixture declares no erasure, so the code above means something when it appears.
#[test]
fn a_package_with_nothing_unread_declares_no_erasure() {
    let sealed = ethos_parser_office::read(&fixture("simple-paragraphs")).expect("reads");
    assert!(!sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ));
}

// -------------------------------------------------------------------------------------------
// Detection is content-based, and every failure is closed
// -------------------------------------------------------------------------------------------

/// **A4: the bytes decide, never the extension.**
#[test]
fn detection_reads_the_bytes() {
    let docx = fixture("simple-paragraphs");
    assert!(
        ethos_parser_office::is_docx(&docx),
        "a real package is recognised"
    );

    // The same bytes would be `report.bin` on disk and would still read: nothing in `is_docx` or
    // `read` has ever seen a file name.
    assert!(ethos_parser_office::read(&docx).is_ok());

    for not_a_docx in [
        &b"%PDF-1.7\n"[..],
        &b""[..],
        &b"PK\x03\x04 but truncated right here"[..],
        &b"<w:document/>"[..],
    ] {
        assert!(
            !ethos_parser_office::is_docx(not_a_docx),
            "{not_a_docx:?} is not an OOXML word-processing document"
        );
    }
}

/// A ZIP that is not a word-processing document is refused **by name**, not read as an empty one.
#[test]
fn a_zip_without_the_main_part_is_a_named_refusal() {
    // The fixture with its central directory intact but its main part renamed out from under it:
    // the shape an `.xlsx` has, and the reason `is_docx` asks for the part rather than the magic.
    let mut bytes = fixture("simple-paragraphs");
    let needle = b"word/document.xml";
    // The same length, so every offset in the archive stays valid — and in BOTH the local header
    // and the central directory, so the package is consistently something else rather than
    // internally contradictory.
    let replacement = b"xl/workbook00.xml";
    assert_eq!(needle.len(), replacement.len());
    let mut renamed = 0;
    let mut at = 0;
    while at + needle.len() <= bytes.len() {
        if &bytes[at..at + needle.len()] == needle {
            bytes[at..at + needle.len()].copy_from_slice(replacement);
            renamed += 1;
        }
        at += 1;
    }
    assert!(
        renamed >= 2,
        "the name appears in the local header and the directory"
    );

    assert!(!ethos_parser_office::is_docx(&bytes));
    let error = ethos_parser_office::read(&bytes).expect_err("refused");
    assert!(
        error.to_string().contains("word/document.xml"),
        "the refusal names what was missing: {error}"
    );
}

/// A truncated package fails closed rather than yielding the runs it managed to reach.
#[test]
fn a_truncated_package_is_refused() {
    let full = fixture("simple-paragraphs");
    for cut in [8, full.len() / 2, full.len() - 4] {
        let error =
            ethos_parser_office::read(&full[..cut]).expect_err("a truncated archive is refused");
        assert!(
            !error.to_string().is_empty(),
            "every refusal is named, never a silent empty document"
        );
    }
}
