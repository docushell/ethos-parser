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

//! v2-S8's coverage, executable: **a stream with no container still binds a quote.**
//!
//! The seventh sibling of `docx_representation.rs`, and the first whose format is not a package.
//! Most of its assertions are the six before it's. The ones that are new are about two things: an
//! address with **no part in it**, and a page break the file writes in plain words and this engine
//! still refuses.
//!
//! **Every assertion about the fixture's contents reads the fixture's own bytes.** v2-S5's review
//! found a guard that passed because it grepped `make_fixtures.py` and matched a `#` comment.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ethos_parser_core::{
    GeometryAbsence, GeometryPresence, NativeLocator, NodeAttributes, Profile, RtfLocator,
    RtfParagraphBreak,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("document.rtf");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fixture's own bytes, as text. **Never the generator's source.**
fn source_of(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("an RTF stream is ASCII")
}

fn locators(sealed: &ethos_parser_core::DocumentRepresentation) -> Vec<RtfLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Rtf(l) => l.clone(),
            other => panic!("an RTF paragraph must carry an RTF address, not {other:?}"),
        })
        .collect()
}

fn text_at(sealed: &ethos_parser_core::DocumentRepresentation, paragraph: u32) -> Option<String> {
    sealed
        .payload()
        .nodes
        .iter()
        .find(|n| matches!(&n.native_locator, NativeLocator::Rtf(l) if l.paragraph == paragraph))
        .map(|n| n.text.clone())
}

fn all_text(sealed: &ethos_parser_core::DocumentRepresentation) -> String {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| n.text.as_str())
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

// -------------------------------------------------------------------------------------------
// The gate
// -------------------------------------------------------------------------------------------

/// **The slice's sentence, executable.** A known phrase is on a node, addressed by the stream.
#[test]
fn a_paragraph_resolves_to_a_node_addressed_by_the_stream_itself() {
    let sealed = ethos_parser_office::read(&fixture("rich-text-paragraphs")).expect("the fixture reads");

    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Evidence, not extraction.")
        .expect("the phrase is a paragraph in the document, so it is in the artifact");

    match (&node.native_locator, &node.attributes) {
        (NativeLocator::Rtf(l), NodeAttributes::RtfParagraph(a)) => {
            assert_eq!(l.paragraph, 1);
            assert_eq!(a.terminator, RtfParagraphBreak::Paragraph);
        }
        other => panic!("expected an RTF paragraph, got {other:?}"),
    }
}

/// **The page is in the file, in the plainest words any v2 format uses, and stays out.**
///
/// `\page` is a page break and `\paperw` a paper width. Both are asserted present in the fixture's
/// own bytes, so `pages: []` below is a refusal rather than an absence.
#[test]
fn the_page_break_in_the_stream_is_read_and_is_still_not_a_page() {
    let bytes = fixture("rich-text-paragraphs");
    let source = source_of(&bytes);
    assert!(
        source.contains(r"\page ") && source.contains(r"\paperw"),
        "the fixture must really carry the producing application's own page break and paper \
         size, or the assertions below are vacuous: {source}"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let payload = sealed.payload();
    assert!(
        payload.pages.is_empty(),
        "`\\page` is where the producer broke a page, not a page this engine measured"
    );
    assert!(!payload.nodes.is_empty(), "and that is not vacuous");
    assert_eq!(
        text_at(&sealed, 4).as_deref(),
        Some("Split by the producer."),
        "the break contributes no character either — it is a layout mark, not text"
    );

    for (index, node) in payload.nodes.iter().enumerate() {
        assert!(
            !node.native_locator.is_paginated(),
            "an RTF address is page-less: {:?}",
            node.native_locator
        );
        assert_eq!(
            sealed.geometry()[index].presence,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        );
    }
}

/// The locator's field set is exactly one, and it refuses a page **by name**.
#[test]
fn the_locator_carries_one_field_and_refuses_a_page_by_name() {
    let parsed: RtfLocator =
        serde_json::from_str(r#"{"paragraph":1}"#).expect("one field is the shape");
    assert_eq!(parsed.paragraph, 1);

    for forbidden in [
        r#"{"paragraph":1,"page":1}"#,
        r#"{"paragraph":1,"bbox":[0,0,1,1]}"#,
        r#"{"paragraph":1,"x":0}"#,
        r#"{"paragraph":1,"part":"document.rtf"}"#,
        r#"{"paragraph":1,"sect":1}"#,
    ] {
        assert!(
            serde_json::from_str::<RtfLocator>(forbidden).is_err(),
            "`deny_unknown_fields` must refuse {forbidden}"
        );
    }
}

/// **The address names no part, and the invariant knows it.**
///
/// Every format before this one is a package, and `check_structure` checks that one part id means
/// one part name. RTF has no parts, so the claim it can still make is that one document has one
/// container id — and a locator that answers `names_a_part` false is checked on that instead.
#[test]
fn the_address_names_no_part_and_every_node_shares_one_container() {
    let sealed = ethos_parser_office::read(&fixture("rich-text-paragraphs")).expect("the fixture reads");
    let payload = sealed.payload();

    let containers: BTreeSet<&str> = payload.nodes.iter().map(|n| n.parent.as_str()).collect();
    assert_eq!(containers.len(), 1, "one stream, one container id");

    for node in &payload.nodes {
        assert!(
            !node.native_locator.names_a_part(),
            "an RTF address has no part to name"
        );
        assert_eq!(
            node.native_locator.part(),
            None,
            "and it does not invent one either"
        );
        assert!(
            node.parent.as_str().starts_with('d'),
            "node {} is parented by `{}`, which is not a part id",
            node.id,
            node.parent
        );
    }
}

// -------------------------------------------------------------------------------------------
// The text, and what never reaches it
// -------------------------------------------------------------------------------------------

/// The stated characters are the stream's own, including a table's cells.
#[test]
fn a_paragraphs_text_is_what_the_stream_states() {
    let bytes = fixture("rich-text-paragraphs");
    let source = source_of(&bytes);
    for written in [r"\'26", r"\u233 ?", r"\tab ", r"{\b displayed}", r"\cell"] {
        assert!(
            source.contains(written),
            "the fixture must really contain `{written}`"
        );
    }

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 2).as_deref(),
        Some("Rows & columns bind to a paragraph\tand never to a page."),
        "`\\'26` is `&` in every ANSI code page, and `\\tab` is the tab the stream states"
    );
    assert_eq!(
        text_at(&sealed, 3).as_deref(),
        Some("A quote binds to the displayed text, café included."),
        "a bold group is formatting, so its text is the body's, and `\\u233` is the scalar the \
         stream names"
    );

    let cells: Vec<String> = sealed
        .payload()
        .nodes
        .iter()
        .filter(|n| {
            matches!(&n.attributes, NodeAttributes::RtfParagraph(a)
                if a.terminator == RtfParagraphBreak::Cell)
        })
        .map(|n| n.text.clone())
        .collect();
    assert_eq!(
        cells,
        vec!["Left cell", "Right cell"],
        "a cell is its own paragraph and says so — and is **not** a TableRecord"
    );
    assert!(
        sealed.payload().tables.is_empty(),
        "`\\cell` is a terminator the file writes, not a grid a detector inferred"
    );
}

/// **Every destination this reader passes over, absent from the body and present in the count.**
///
/// The load-bearing honesty test. Each string is asserted present in the fixture's own bytes and
/// absent from every node — so it fails both if the reader starts splicing and if the fixture
/// stops containing the construct.
#[test]
fn destination_text_is_absent_from_every_paragraph_and_is_declared() {
    let bytes = fixture("rich-text-unread-destinations");
    let source = source_of(&bytes);
    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let all = all_text(&sealed);

    for (what, marker) in [
        ("a font table", "FONT-TABLE-NAME"),
        ("a style sheet", "STYLE-SHEET-NAME"),
        ("document information", "INFO-TITLE"),
        ("a document author", "INFO-AUTHOR"),
        ("an ignorable destination", "IGNORABLE-GENERATOR"),
        ("a header", "HEADER-TEXT"),
        ("a footer", "FOOTER-TEXT"),
        ("a footnote body", "FOOTNOTE-BODY"),
        ("a field's cached result", "FIELD-RESULT"),
    ] {
        assert!(
            source.contains(marker),
            "{what}: the fixture must really contain `{marker}`, or the assertion below is vacuous"
        );
        assert!(
            !all.contains(marker),
            "{what}: `{marker}` reached a paragraph's text — all of it was `{all}`"
        );
    }

    // A field's **instruction** is skipped with its result: nothing here evaluates a field, and
    // `PAGE` resolving to a number would be a page this engine did not measure.
    assert!(source.contains(r"\fldinst PAGE"), "{source}");
    assert!(!all.contains("PAGE"), "{all}");

    // And a picture's binary data cannot re-enter the stream as text.
    assert!(source.contains(r"\bin4"), "{source}");
    assert!(!all.contains(r"\pngblip"), "{all}");

    // The body's own text survives all of it.
    assert_eq!(text_at(&sealed, 1).as_deref(), Some("Kept sentence."));
    assert_eq!(text_at(&sealed, 4).as_deref(), Some("Footnoted and kept."));
    assert_eq!(text_at(&sealed, 5).as_deref(), Some("Page  of many."));
    assert_eq!(
        text_at(&sealed, 6).as_deref(),
        Some("A picture sat above this line.")
    );
}

/// **The paragraph counter advances through destinations this reader does not read.**
///
/// The header's and the footer's own `\par` each move it, so paragraphs 2 and 3 are missing from
/// the artifact and paragraph 4 is the next one a consumer counting breaks in the bytes finds.
/// `OdtLocator::paragraph`'s rule, in RTF's spelling, and asserted rather than assumed.
#[test]
fn the_counter_advances_through_a_destination_it_does_not_read() {
    let bytes = fixture("rich-text-unread-destinations");
    let source = source_of(&bytes);
    for written in [r"{\header HEADER-TEXT\par }", r"{\footer FOOTER-TEXT\par }"] {
        assert!(
            source.contains(written),
            "the fixture must really put a `\\par` inside a destination this reader skips, or \
             the gap asserted below is not a gap: {written}"
        );
    }

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let addressed: Vec<u32> = locators(&sealed).iter().map(|l| l.paragraph).collect();
    assert_eq!(
        addressed,
        vec![1, 4, 5, 6, 7, 8],
        "2 and 3 are the header's and the footer's own paragraphs — counted in the file, absent \
         from the record"
    );
}

/// **A byte above 0x7F is declared, never rendered as a guess.**
#[test]
fn an_undecodable_byte_is_counted_and_a_decodable_one_is_read() {
    let bytes = fixture("rich-text-unread-destinations");
    let source = source_of(&bytes);
    assert!(
        source.contains(r"\'e9"),
        "the fixture must really carry one"
    );
    assert!(source.contains(r"\'26"), "and one below 0x80");

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    assert_eq!(
        text_at(&sealed, 7).as_deref(),
        Some("Undecodable: caf and rsum."),
        "the byte's meaning depends on a code page this reader does not read, so it contributes \
         no character rather than a Latin-1 guess"
    );
    assert_eq!(
        text_at(&sealed, 8).as_deref(),
        Some("Decodable: & and é are both read."),
        "0x26 is `&` in every ANSI code page, and `\\u233` states its scalar outright"
    );

    let detail = a14_detail(&sealed);
    assert!(
        detail.contains("3 byte(s) above 0x7F"),
        "and the count is on the wire: {detail}"
    );
}

/// Deleting a destination from the **bytes** moves the count — so the count is not decoration.
#[test]
fn removing_a_destination_from_the_bytes_moves_the_declared_count() {
    let bytes = fixture("rich-text-unread-destinations");
    let before = destinations_declared(&bytes);

    let stripped = source_of(&bytes).replace(r"{\footer FOOTER-TEXT\par }", "");
    assert_ne!(
        stripped,
        source_of(&bytes),
        "the mutation must actually change the stream"
    );

    let sealed = ethos_parser_office::read(stripped.as_bytes()).expect("the mutated stream reads");
    assert_eq!(
        (before, destinations_declared(stripped.as_bytes())),
        (10, 9),
        "removing the footer must lower the declared destination count by exactly one — an exact \
         pair rather than `after < before`, so a mutation that broke the stream into declaring \
         nothing could not pass as a success"
    );
    assert!(
        !all_text(&sealed).contains("FOOTER-TEXT"),
        "and nothing was promoted into the body by its removal"
    );
}

/// The A14 declaration names both kinds, and the clean stream declares neither.
#[test]
fn the_clean_stream_declares_no_erasure_and_the_other_declares_two_kinds() {
    let clean = ethos_parser_office::read(&fixture("rich-text-paragraphs")).expect("reads");
    assert!(
        !clean
            .payload()
            .assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ),
        "the clean stream carries only what the reader consumes, so it erases nothing"
    );

    let detail =
        a14_detail(&ethos_parser_office::read(&fixture("rich-text-unread-destinations")).expect("reads"));
    assert!(detail.contains("destination(s)"), "{detail}");
    assert!(detail.contains("byte(s) above 0x7F"), "{detail}");
    assert!(
        detail.contains("header's or footnote's words are the ones to check first"),
        "the one a caller most needs to know is named: {detail}"
    );
}

// -------------------------------------------------------------------------------------------
// Identity and detection
// -------------------------------------------------------------------------------------------

/// The RTF profile is its own, and all eight are mutually distinct.
#[test]
fn the_rtf_profile_is_its_own_and_all_eight_are_distinct() {
    let hashes: BTreeSet<String> = [
        Profile::default(),
        Profile::docx_v0(),
        Profile::xlsx_v0(),
        Profile::pptx_v0(),
        Profile::odt_v0(),
        Profile::ods_v0(),
        Profile::odp_v0(),
        Profile::rtf_v0(),
    ]
    .iter()
    .map(|p| p.profile_sha256().expect("hashes").to_string())
    .collect();
    assert_eq!(
        hashes.len(),
        8,
        "eight readers, eight profiles, eight hashes"
    );

    let profile = Profile::rtf_v0();
    assert!(
        !profile.capabilities.tables,
        "`\\cell` is a terminator the file writes, not a grid a detector inferred"
    );
    assert!(!profile.capabilities.measured_ink_boxes);
    assert!(!profile.capabilities.structural_locators);
}

/// Two runs over one document produce identical bytes.
#[test]
fn two_runs_over_one_stream_produce_identical_bytes() {
    for name in ["rich-text-paragraphs", "rich-text-unread-destinations"] {
        let bytes = fixture(name);
        let a = ethos_parser_office::read(&bytes).expect("reads");
        let b = ethos_parser_office::read(&bytes).expect("reads again");
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap(),
            "{name} must be byte-identical across runs"
        );
    }
}

/// **Detection reads the bytes, and the near misses are the point.**
#[test]
fn detection_reads_the_bytes_and_refuses_the_near_misses() {
    let bytes = fixture("rich-text-paragraphs");
    assert!(ethos_parser_office::is_rtf(&bytes));
    assert!(!ethos_parser_office::is_docx(&bytes));
    assert!(!ethos_parser_office::is_xlsx(&bytes));
    assert!(!ethos_parser_office::is_pptx(&bytes));
    assert!(
        !ethos_parser_office::is_opendocument(&bytes),
        "an RTF has no `mimetype` entry, because it has no container at all"
    );

    for (what, near) in [
        ("a bare open brace", b"{ not rtf }".to_vec()),
        ("another first control word", br"{\ansi text}".to_vec()),
        (
            "an OLE compound file, which is a legacy `.doc`",
            vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        ),
        ("a ZIP package", b"PK\x03\x04".to_vec()),
        ("a PDF", b"%PDF-1.7\n".to_vec()),
    ] {
        assert!(
            !ethos_parser_office::is_rtf(&near),
            "{what} must not be claimed as RTF"
        );
    }

    // And the ODF family question is untouched by this slice: an `.epub` is still not OpenDocument.
    let epub = build_ocf_zip("application/epub+zip");
    assert!(
        !ethos_parser_office::is_opendocument(&epub),
        "an EPUB uses OCF's first-and-stored `mimetype` too, and is NOT OpenDocument"
    );
    assert!(!ethos_parser_office::is_rtf(&epub));

    // And an `.odg` is still refused as OpenDocument, by name.
    let odg = build_ocf_zip("application/vnd.oasis.opendocument.graphics");
    assert!(ethos_parser_office::is_opendocument(&odg));
    let error = ethos_parser_office::read(&odg).expect_err("a drawing has no reader");
    let text = error.to_string();
    assert!(
        text.contains("OpenDocument") && text.contains("graphics"),
        "{text}"
    );
    assert!(!text.contains("%PDF-"), "{text}");
}

/// A stream that is neither RTF nor a package names **both**, not a missing PDF header.
#[test]
fn bytes_that_are_neither_are_refused_naming_both_shapes() {
    let error = ethos_parser_office::read(b"plain text, no container").expect_err("refused");
    let text = error.to_string();
    assert!(
        text.contains(r"{\rtf"),
        "the refusal names the stream shape: {text}"
    );
    assert!(text.contains("ZIP"), "and the container shape: {text}");
    assert!(!text.contains("%PDF-"), "{text}");
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

fn a14_detail(sealed: &ethos_parser_core::DocumentRepresentation) -> String {
    sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .map(|l| l.detail.clone())
        .expect("this stream declares an erasure")
}

fn destinations_declared(bytes: &[u8]) -> usize {
    let sealed = ethos_parser_office::read(bytes).expect("reads");
    let detail = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .map(|l| l.detail.clone())
        .unwrap_or_default();
    detail
        .split_once(" destination(s)")
        .and_then(|(head, _)| head.rsplit(' ').next().map(str::to_string))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// A minimal OCF-shaped package declaring `media_type`: `mimetype` first and **stored**.
///
/// A local writer rather than a dependency: `deny.toml` bans a ZIP crate, and the container rule
/// this has to honour — first entry, uncompressed — is three lines.
fn build_ocf_zip(media_type: &str) -> Vec<u8> {
    build_zip(&[
        ("mimetype", media_type),
        (
            "META-INF/manifest.xml",
            r#"<?xml version="1.0"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"><manifest:file-entry manifest:full-path="content.xml"/></manifest:manifest>"#,
        ),
        ("content.xml", r#"<?xml version="1.0"?><office/>"#),
    ])
}

fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for (name, body) in entries {
        let offset = out.len() as u32;
        let data = body.as_bytes();
        let crc = crc32(data);

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&[10, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }

    let directory_offset = out.len() as u32;
    let directory_size = directory.len() as u32;
    out.extend_from_slice(&directory);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&directory_size.to_le_bytes());
    out.extend_from_slice(&directory_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
