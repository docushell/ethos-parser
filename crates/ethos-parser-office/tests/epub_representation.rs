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

//! v2-S9's coverage, executable: **the spine is the reading order, and the page-list is not a
//! page.**
//!
//! The eighth sibling of `docx_representation.rs`. Two of its assertions carry weight the earlier
//! ones could not:
//!
//! 1. **Reading order.** The fixture stores its chapters in the *opposite* order to the spine, so
//!    a reader that took archive order — or sorted by name — puts chapter two first. Both
//!    orderings are read out of the package's own bytes, so the test cannot pass by accident.
//! 2. **The page.** This is the first format whose file may genuinely name pages: the second
//!    fixture carries an EPUB 3 navigation document with a `page-list` of print page numbers.
//!    `pages: []` is therefore a refusal rather than an absence, and the numbers are asserted
//!    present in the bytes before it is checked.
//!
//! **Every assertion about a fixture's contents inflates that package's own entries.** v2-S5's
//! review found a guard that passed because it grepped `make_fixtures.py` and matched a `#`
//! comment.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ethos_parser_core::{
    EpubLocator, GeometryAbsence, GeometryPresence, NativeLocator, NodeAttributes, Profile,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(name)
        .join("book.epub");
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// One of the package's own entries, inflated. **Never the generator's source.**
fn entry_of(bytes: &[u8], name: &str) -> String {
    let part = ethos_parser_office::zip::read_entry(bytes, name)
        .unwrap_or_else(|e| panic!("`{name}` inflates: {e}"));
    String::from_utf8(part).expect("the entry is UTF-8")
}

fn locators(sealed: &ethos_parser_core::DocumentRepresentation) -> Vec<EpubLocator> {
    sealed
        .payload()
        .nodes
        .iter()
        .map(|n| match &n.native_locator {
            NativeLocator::Epub(l) => l.clone(),
            other => panic!("an EPUB block must carry an EPUB address, not {other:?}"),
        })
        .collect()
}

fn text_at(
    sealed: &ethos_parser_core::DocumentRepresentation,
    part: &str,
    block: u32,
) -> Option<String> {
    sealed
        .payload()
        .nodes
        .iter()
        .find(|n| {
            matches!(&n.native_locator, NativeLocator::Epub(l)
                if l.part == part && l.block == block)
        })
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

/// **The slice's sentence, executable.** A known phrase is on a node, addressed by the package.
#[test]
fn a_block_resolves_to_a_node_addressed_by_the_package_itself() {
    let sealed = ethos_parser_office::read(&fixture("book-spine")).expect("the fixture reads");

    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Evidence, not extraction.")
        .expect("the phrase is a heading in the publication, so it is in the artifact");

    match (&node.native_locator, &node.attributes) {
        (NativeLocator::Epub(l), NodeAttributes::EpubBlock(a)) => {
            assert_eq!(l.part, "OEBPS/zz-first.xhtml");
            assert_eq!(
                l.block, 2,
                "`<body>` is a block too and takes position 1, minting no node"
            );
            assert_eq!(
                a.element, "h1",
                "the element name is the file's own word for it"
            );
            assert!(a.linear);
        }
        other => panic!("expected an EPUB block, got {other:?}"),
    }
}

/// **Reading order is the spine, and the archive's own order is the trap.**
///
/// The load-bearing test of this slice. The package stores `OEBPS/aa-second.xhtml` **before**
/// `OEBPS/zz-first.xhtml`, and its spine lists them the other way round — so archive order and
/// name order both disagree with the answer. Both facts are read out of the package's own bytes,
/// so a reader that took either shortcut fails here rather than quietly disagreeing with the
/// publication.
#[test]
fn the_spine_states_the_order_and_the_archive_does_not() {
    let bytes = fixture("book-spine");

    let names =
        ethos_parser_office::zip::entry_names(&bytes).expect("the archive lists its entries");
    let archive_order: Vec<&String> = names
        .iter()
        .filter(|n| n.ends_with(".xhtml"))
        .collect::<Vec<_>>();
    assert_eq!(
        archive_order.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
        vec![
            "OEBPS/aa-second.xhtml",
            "OEBPS/zz-first.xhtml",
            "OEBPS/notes.xhtml"
        ],
        "the fixture must really store its chapters in an order the spine disagrees with, or the \
         assertion below proves nothing"
    );

    let package = entry_of(&bytes, "OEBPS/content.opf");
    let first = package.find("zz-first.xhtml").expect("the spine names it");
    let second = package.find("aa-second.xhtml").expect("and the other");
    assert!(
        package.find(r#"idref="c1""#).unwrap() < package.find(r#"idref="c2""#).unwrap(),
        "and the spine must really list c1 before c2: {package}"
    );
    assert!(
        first < second,
        "with c1 the chapter stored second: {package}"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let parts: Vec<String> = locators(&sealed)
        .iter()
        .map(|l| l.part.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(parts.len(), 3, "three spine documents produced nodes");

    // The order that matters is the order the NODES arrive in, which is the spine's.
    let mut seen: Vec<String> = Vec::new();
    for locator in locators(&sealed) {
        if seen.last() != Some(&locator.part) {
            seen.push(locator.part);
        }
    }
    assert_eq!(
        seen,
        vec![
            "OEBPS/zz-first.xhtml".to_string(),
            "OEBPS/aa-second.xhtml".to_string(),
            "OEBPS/notes.xhtml".to_string()
        ],
        "the spine's order, not the archive's and not the file names'"
    );
}

/// **The page is nameable in this format, and the artifact still carries none.**
///
/// The second package's navigation document lists the page numbers of a print edition. They are
/// asserted present in its own bytes, so `pages: []` below is a refusal rather than an absence —
/// and the labels are asserted absent from every node, so they were not copied in under another
/// name either.
#[test]
fn a_page_list_names_print_pages_and_still_produces_no_page() {
    let bytes = fixture("book-unread-parts");
    let nav = entry_of(&bytes, "OEBPS/nav.xhtml");
    assert!(
        nav.contains(r#"epub:type="page-list""#)
            && nav.contains("PRINT-PAGE-17")
            && nav.contains("PRINT-PAGE-18"),
        "the fixture must really name the pages of a print edition, or the assertions below are \
         vacuous: {nav}"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let payload = sealed.payload();
    assert!(
        payload.pages.is_empty(),
        "a publisher's label about somebody else's paper has no width and no height, so nothing \
         could ever be validated against it"
    );
    assert!(!payload.nodes.is_empty(), "and that is not vacuous");

    let all = all_text(&sealed);
    for label in ["PRINT-PAGE-17", "PRINT-PAGE-18", "TOC-ENTRY-TEXT"] {
        assert!(
            !all.contains(label),
            "`{label}` reached a node's text, so a navigation document became content: {all}"
        );
    }

    for (index, node) in payload.nodes.iter().enumerate() {
        assert!(
            !node.native_locator.is_paginated(),
            "an EPUB address is page-less: {:?}",
            node.native_locator
        );
        assert_eq!(
            sealed.geometry()[index].presence,
            GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        );
    }
}

/// The locator's field set is exactly two, and it refuses a page **by name**.
#[test]
fn the_locator_carries_two_fields_and_refuses_a_page_by_name() {
    let json = r#"{"part":"OEBPS/c1.xhtml","block":1}"#;
    let parsed: EpubLocator = serde_json::from_str(json).expect("two fields are the shape");
    assert_eq!(parsed.part, "OEBPS/c1.xhtml");
    assert_eq!(parsed.block, 1);

    for forbidden in [
        r#"{"part":"OEBPS/c1.xhtml","block":1,"page":1}"#,
        r#"{"part":"OEBPS/c1.xhtml","block":1,"bbox":[0,0,1,1]}"#,
        r#"{"part":"OEBPS/c1.xhtml","block":1,"x":0}"#,
        r#"{"part":"OEBPS/c1.xhtml","block":1,"page_list_label":"17"}"#,
        r#"{"part":"OEBPS/c1.xhtml","block":1,"spine_index":1}"#,
    ] {
        assert!(
            serde_json::from_str::<EpubLocator>(forbidden).is_err(),
            "`deny_unknown_fields` must refuse {forbidden}"
        );
    }
}

/// **This format has parts, so it takes the bijection half of the shape v2-S8 split apart.**
#[test]
fn every_part_name_has_one_id_and_every_id_one_part_name() {
    let sealed = ethos_parser_office::read(&fixture("book-spine")).expect("the fixture reads");
    let payload = sealed.payload();

    let mut by_id: std::collections::BTreeMap<&str, &str> = Default::default();
    let mut by_part: std::collections::BTreeMap<&str, &str> = Default::default();
    for node in &payload.nodes {
        assert!(
            node.native_locator.names_a_part(),
            "an EPUB address names the spine document it came from"
        );
        let part = node.native_locator.part().expect("and it is present");
        let id = node.parent.as_str();
        assert_eq!(*by_id.entry(id).or_insert(part), part);
        assert_eq!(*by_part.entry(part).or_insert(id), id);
        assert!(id.starts_with('d'), "parented by a part id, not a page id");
    }
    assert_eq!(by_id.len(), 3, "three spine documents, three part ids");

    // And the ordinal restarts per part, while the block address does not.
    for part in by_part.keys() {
        let ordinals: Vec<u32> = payload
            .nodes
            .iter()
            .filter(|n| n.native_locator.part() == Some(*part))
            .map(|n| n.ordinal)
            .collect();
        assert_eq!(
            ordinals,
            (1..=ordinals.len() as u32).collect::<Vec<_>>(),
            "`{part}` must carry a contiguous 1-based sequence of its own"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The text, and what never reaches it
// -------------------------------------------------------------------------------------------

/// The stated characters are the document's own, under XHTML's rules.
#[test]
fn a_blocks_text_is_what_the_document_states() {
    let bytes = fixture("book-spine");
    let chapter = entry_of(&bytes, "OEBPS/zz-first.xhtml");
    for written in ["&amp;", "&#233;", "<br/>", "<pre>", "<em>"] {
        assert!(
            chapter.contains(written),
            "the fixture must really contain `{written}`"
        );
    }

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let part = "OEBPS/zz-first.xhtml";
    assert_eq!(
        text_at(&sealed, part, 3).as_deref(),
        Some("Rows & columns bind to a block\nand never to a page."),
        "`&amp;` survives and `<br/>` is the line feed the element states"
    );
    assert_eq!(
        text_at(&sealed, part, 4).as_deref(),
        Some("A quote binds to the displayed text, café included."),
        "an `<em>` is inline, so its characters are the sentence's — and `&#233;` is the scalar \
         the reference names, which the shared entity rule alone would have refused"
    );
    assert_eq!(
        text_at(&sealed, part, 5).as_deref(),
        Some("  two leading spaces\n  and a second line"),
        "`white-space: pre` is the document saying those spaces are content"
    );
}

/// A table's cells are blocks, and **not** a `TableRecord`.
#[test]
fn a_table_cell_is_a_block_and_no_detector_ran() {
    let sealed = ethos_parser_office::read(&fixture("book-spine")).expect("the fixture reads");
    let cells: Vec<String> = sealed
        .payload()
        .nodes
        .iter()
        .filter(|n| matches!(&n.attributes, NodeAttributes::EpubBlock(a) if a.element == "td"))
        .map(|n| n.text.clone())
        .collect();
    assert_eq!(cells, vec!["Left cell", "Right cell"]);
    assert!(
        sealed.payload().tables.is_empty(),
        "a cell's text is read as the block it is; a TableRecord would say a detector ran"
    );
}

/// **A non-linear spine item is read and labelled**, which is what keeps it from being a silent
/// extra.
#[test]
fn a_non_linear_spine_item_is_read_and_says_so() {
    let bytes = fixture("book-spine");
    assert!(
        entry_of(&bytes, "OEBPS/content.opf").contains(r#"linear="no""#),
        "the fixture must really mark a spine item non-linear"
    );

    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let node = sealed
        .payload()
        .nodes
        .iter()
        .find(|n| n.text == "Auxiliary content the spine marks non-linear.")
        .expect("it is read, because it is a document the spine lists");
    match &node.attributes {
        NodeAttributes::EpubBlock(a) => assert!(
            !a.linear,
            "and it is labelled, so a consumer can tell it from the main flow"
        ),
        other => panic!("expected an EPUB block, got {other:?}"),
    }
    assert!(
        sealed
            .payload()
            .nodes
            .iter()
            .any(|n| matches!(&n.attributes, NodeAttributes::EpubBlock(a) if a.linear)),
        "and the flag is not simply false everywhere"
    );
}

/// **Everything this reader passes over, absent from the body and present in the count.**
#[test]
fn skipped_content_is_absent_from_every_block_and_is_declared() {
    let bytes = fixture("book-unread-parts");
    let body = entry_of(&bytes, "OEBPS/text/body.xhtml");
    let sealed = ethos_parser_office::read(&bytes).expect("the fixture reads");
    let all = all_text(&sealed);

    for (what, in_file, marker) in [
        ("a script's source", "<script>", "SCRIPT_SOURCE"),
        ("a style sheet's rules", "<style>", "STYLE-SHEET-RULE"),
        ("an inline SVG's title", "<svg:title>", "SVG-TITLE-TEXT"),
        ("a MathML annotation", "<m:annotation>", "MATHML-ANNOTATION"),
        ("a ruby annotation", "<rt>", "RUBY-GUIDE"),
        ("a template's content", "<template>", "TEMPLATE-CONTENT"),
        ("a document title", "<title>", "HEAD-TITLE-TEXT"),
    ] {
        assert!(
            body.contains(in_file) && body.contains(marker),
            "{what}: the fixture must really contain `{in_file}` and `{marker}`, or the assertion \
             below is vacuous"
        );
        assert!(
            !all.contains(marker),
            "{what}: `{marker}` reached a block's text — all of it was `{all}`"
        );
    }

    // The entries the reader never opened are counted, and their text never appears.
    for (entry, marker) in [
        ("OEBPS/toc.ncx", "NCX-LABEL-TEXT"),
        ("OEBPS/rendition2.opf", "SECOND-RENDITION-TITLE"),
        ("OEBPS/images/cover.svg", "COVER-SVG-TITLE"),
    ] {
        assert!(entry_of(&bytes, entry).contains(marker), "{entry}");
        assert!(!all.contains(marker), "{marker} in {all}");
    }

    // And the body's own text survives all of it.
    assert_eq!(
        text_at(&sealed, "OEBPS/text/body.xhtml", 2).as_deref(),
        Some("Kept sentence.")
    );
    assert!(all.contains("Last kept sentence."), "{all}");
    assert!(
        all.contains("kanji"),
        "a ruby base is the word; only the guide is passed over: {all}"
    );
}

/// Deleting a region from the **bytes** moves the count — so the count is not decoration.
#[test]
fn removing_a_region_from_the_bytes_moves_the_declared_count() {
    let bytes = fixture("book-unread-parts");
    let before = regions_declared(&bytes);

    let body = entry_of(&bytes, "OEBPS/text/body.xhtml");
    let stripped = body.replace("<script>var SCRIPT_SOURCE = 1;</script>", "");
    assert_ne!(stripped, body, "the mutation must actually change the part");

    let mutated = with_entry(&bytes, "OEBPS/text/body.xhtml", &stripped);
    assert_eq!(
        (before, regions_declared(&mutated)),
        (6, 5),
        "removing the script must lower the declared region count by exactly one — an exact pair \
         rather than `after < before`, so a mutation that broke the package into declaring nothing \
         could not pass as a success"
    );
    assert!(
        !all_text(&ethos_parser_office::read(&mutated).expect("the mutated package reads"))
            .contains("SCRIPT_SOURCE"),
        "and nothing was promoted into the body by its removal"
    );
}

/// The A14 declaration names every kind, and the clean publication declares none of them.
#[test]
fn the_clean_publication_declares_no_erasure_and_the_other_declares_every_kind() {
    let clean = ethos_parser_office::read(&fixture("book-spine")).expect("reads");
    assert!(
        !clean
            .payload()
            .assurance
            .limitations
            .iter()
            .any(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ),
        "the clean publication carries only what the reader consumes, so it erases nothing"
    );

    // **Exact counts, not just the words.** An adversarial review of this slice injected wrong
    // multipliers into four of these counters and every "the message names this category" check
    // still passed — so each number is pinned to what the fixture actually contains.
    let detail =
        a14_detail(&ethos_parser_office::read(&fixture("book-unread-parts")).expect("reads"));
    for (what, expected) in [
        // rendition2.opf, images/cover.svg, images/pic.png, toc.ncx, style/book.css
        ("5 entry(ies)", "the entries no reader opened"),
        // The cover, whose manifest media type is `image/svg+xml`.
        (
            "1 spine item(s)",
            "the one spine item this slice does not read",
        ),
        // `container.xml` lists two rootfiles; the first is the default rendition.
        ("1 additional rendition(s)", "the second rendition"),
        // nav's two `<nav>`s, plus body's script, style, `<rt>` and `<template>`.
        ("6 region(s)", "the regions that held text"),
        // The blocks holding an SVG `<title>` and a MathML `<annotation>`.
        ("2 block(s)", "the blocks with foreign characters"),
    ] {
        assert!(
            detail.contains(what),
            "{expected}: expected `{what}` in: {detail}"
        );
    }
    assert!(
        detail.contains("A navigation document is the one to check first"),
        "and the one a caller most needs to know is named: {detail}"
    );
    assert!(
        detail.contains("attributes are never read as text"),
        "including the gap an `alt` falls into: {detail}"
    );
}

// -------------------------------------------------------------------------------------------
// Refusals
// -------------------------------------------------------------------------------------------

/// **An encrypted spine document is refused by name**, and font obfuscation alone is not.
///
/// Nearly every real publication carrying `META-INF/encryption.xml` carries it to obfuscate a
/// font, and its text is in the clear. Refusing those would refuse readable books; ignoring the
/// file would hand ciphertext to the XML reader, which reports malformed XML and names the wrong
/// cause. So the declaration is read and checked **against the spine**.
#[test]
fn an_encrypted_spine_document_is_refused_and_an_obfuscated_font_is_not() {
    let bytes = fixture("book-spine");

    let font_only = with_entry(
        &bytes,
        "META-INF/encryption.xml",
        &encryption_declaring("OEBPS/fonts/x.otf"),
    );
    let sealed =
        ethos_parser_office::read(&font_only).expect("a clear-text publication still reads");
    assert!(!sealed.payload().nodes.is_empty());

    let content_encrypted = with_entry(
        &bytes,
        "META-INF/encryption.xml",
        &encryption_declaring("OEBPS/zz-first.xhtml"),
    );
    let error = ethos_parser_office::read(&content_encrypted).expect_err("refused");
    let text = error.to_string();
    assert!(text.contains("encrypt"), "{text}");
    assert!(text.contains("OEBPS/zz-first.xhtml"), "{text}");
    assert!(
        !text.contains("XML will not parse"),
        "and it names the cause rather than the symptom: {text}"
    );
}

/// A publication with no `container.xml` names the entry it needed.
#[test]
fn a_publication_with_no_container_is_a_named_refusal() {
    let broken = build_ocf(&[
        ("mimetype", "application/epub+zip"),
        ("OEBPS/content.opf", "<package/>"),
    ]);
    let error = ethos_parser_office::read(&broken).expect_err("refused");
    let text = error.to_string();
    assert!(text.contains("META-INF/container.xml"), "{text}");
    assert!(!text.contains("%PDF-"), "{text}");
}

/// A duplicated archive entry is refused before anything reads either copy.
///
/// `zip::read_entry` takes the first of a duplicated name, so a second `content.xml` would be
/// neither read nor counted and a consumer preferring the last would see a different book.
#[test]
fn a_duplicated_entry_is_refused() {
    let bytes = fixture("book-spine");
    let names = ethos_parser_office::zip::entry_names(&bytes).expect("entries");
    let mut owned: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|n| {
            (
                n.clone(),
                ethos_parser_office::zip::read_entry(&bytes, n).expect("inflates"),
            )
        })
        .collect();
    owned.push(("OEBPS/content.opf".into(), b"<duplicate/>".to_vec()));
    let borrowed: Vec<(&str, &[u8])> = owned
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();

    let error = ethos_parser_office::read(&build_zip(&borrowed)).expect_err("refused");
    assert!(error.to_string().contains("more than once"), "{error}");
}

// -------------------------------------------------------------------------------------------
// Identity and detection
// -------------------------------------------------------------------------------------------

/// The EPUB profile is its own, and all nine are mutually distinct.
#[test]
fn the_epub_profile_is_its_own_and_all_nine_are_distinct() {
    let hashes: BTreeSet<String> = [
        Profile::default(),
        Profile::docx_v0(),
        Profile::xlsx_v0(),
        Profile::pptx_v0(),
        Profile::odt_v0(),
        Profile::ods_v0(),
        Profile::odp_v0(),
        Profile::rtf_v0(),
        Profile::epub_v0(),
    ]
    .iter()
    .map(|p| p.profile_sha256().expect("hashes").to_string())
    .collect();
    assert_eq!(hashes.len(), 9, "nine readers, nine profiles, nine hashes");

    let profile = Profile::epub_v0();
    assert!(
        !profile.capabilities.tables,
        "XHTML has `<table>`, and a cell's text is read as the block it is — no detector ran"
    );
    assert!(!profile.capabilities.measured_ink_boxes);
    assert!(!profile.capabilities.structural_locators);
    assert_eq!(
        profile.reading_order_rule,
        ethos_parser_core::EPUB_READING_ORDER_RULE_V1
    );
}

/// Two runs over one publication produce identical bytes.
#[test]
fn two_runs_over_one_publication_produce_identical_bytes() {
    for name in ["book-spine", "book-unread-parts"] {
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

/// **Detection reads the bytes, and the ODF family question is untouched.**
#[test]
fn detection_reads_the_bytes_and_an_epub_is_never_opendocument() {
    let bytes = fixture("book-spine");
    assert!(ethos_parser_office::is_epub(&bytes));
    assert!(
        !ethos_parser_office::is_opendocument(&bytes),
        "an EPUB writes its type into the same first, stored `mimetype` entry an ODF package does \
         — which is precisely why the family question asks the declared TYPE (v2-S6)"
    );
    for (kind, claimed) in [
        ("odt", ethos_parser_office::is_odt(&bytes)),
        ("ods", ethos_parser_office::is_ods(&bytes)),
        ("odp", ethos_parser_office::is_odp(&bytes)),
        ("docx", ethos_parser_office::is_docx(&bytes)),
        ("xlsx", ethos_parser_office::is_xlsx(&bytes)),
        ("pptx", ethos_parser_office::is_pptx(&bytes)),
        ("rtf", ethos_parser_office::is_rtf(&bytes)),
    ] {
        assert!(!claimed, "an EPUB is not {kind}");
    }

    // And the reverse: an ODF package is not an EPUB.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf();
    let odt = std::fs::read(root.join("fixtures/office/text-paragraphs/document.odt"))
        .expect("the ODT fixture is there");
    assert!(!ethos_parser_office::is_epub(&odt));
    assert!(ethos_parser_office::is_opendocument(&odt));
}

/// **Exact, not prefixed**, and the `mimetype` entry must be first and stored.
#[test]
fn a_near_miss_declaration_is_not_claimed() {
    for near in [
        "application/epub",
        "application/epub+zip ",
        "application/vnd.oasis.opendocument.text",
    ] {
        let package = build_ocf(&[
            ("mimetype", near),
            ("META-INF/container.xml", "<container/>"),
        ]);
        // A trailing newline or space is trimmed by the OCF helper, so the middle case IS claimed
        // — asserted rather than assumed, because that trim is a decision v2-S5 made.
        assert_eq!(
            ethos_parser_office::is_epub(&package),
            near.trim() == "application/epub+zip",
            "`{near}`"
        );
    }

    // And the entry has to be first: a package whose `mimetype` is not is not an OCF container.
    let reordered = build_ocf(&[
        ("META-INF/container.xml", "<container/>"),
        ("mimetype", "application/epub+zip"),
    ]);
    assert!(!ethos_parser_office::is_epub(&reordered));
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
        .expect("this publication declares an erasure")
}

fn regions_declared(bytes: &[u8]) -> usize {
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
        .split_once(" region(s)")
        .and_then(|(head, _)| head.rsplit(' ').next().map(str::to_string))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

fn encryption_declaring(uri: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><encryption xmlns="urn:oasis:names:tc:opendocument:xmlns:container" xmlns:enc="http://www.w3.org/2001/04/xmlenc#"><enc:EncryptedData><enc:CipherData><enc:CipherReference URI="{uri}"/></enc:CipherData></enc:EncryptedData></encryption>"#
    )
}

/// Rebuild the publication with one entry **replaced in place**, or appended when it is new.
///
/// In place, because the reader refuses a duplicated entry name before it reads either copy — so
/// a mutation that appended would exercise that refusal rather than the thing under test. Bytes
/// throughout, so a picture entry survives the round trip.
///
/// A minimal writer rather than a dependency: `deny.toml` bans a ZIP crate, and the OCF rule this
/// has to honour — first entry, uncompressed — is three lines.
fn with_entry(original: &[u8], name: &str, body: &str) -> Vec<u8> {
    let names =
        ethos_parser_office::zip::entry_names(original).expect("the archive lists its entries");
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut replaced = false;
    for existing in &names {
        let raw = if existing == name {
            replaced = true;
            body.as_bytes().to_vec()
        } else {
            ethos_parser_office::zip::read_entry(original, existing).expect("inflates")
        };
        entries.push((existing.clone(), raw));
    }
    if !replaced {
        entries.push((name.to_string(), body.as_bytes().to_vec()));
    }
    let borrowed: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    build_zip(&borrowed)
}

/// A package whose `mimetype` is first and stored, which is what makes it an OCF container.
fn build_ocf(entries: &[(&str, &str)]) -> Vec<u8> {
    let borrowed: Vec<(&str, &[u8])> = entries.iter().map(|(n, b)| (*n, b.as_bytes())).collect();
    build_zip(&borrowed)
}

fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for (name, data) in entries {
        let offset = out.len() as u32;
        let data = *data;
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
