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

/// **The same declaration rides a page-less artifact**, which is the half the core unit test
/// cannot prove: `declared_limitations` is pinned in `ethos-parser-core`, but that the code
/// survives `Assurance::new`'s normalize and reaches a real non-PDF artifact is a fact about this
/// crate. `docProps/core.xml` is opened by nothing, and the artifact says so rather than leaving
/// an absent author to be read as a declared absence.
#[test]
fn document_metadata_is_declared_unread_on_an_office_artifact() {
    let sealed = ethos_parser_office::read(&fixture("simple-paragraphs")).expect("reads");
    let limitation = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::DOCUMENT_METADATA_NOT_READ)
        .expect("declared under every profile this build can produce");

    assert!(
        limitation.detail.contains("docProps"),
        "the declaration must name what is not opened: {}",
        limitation.detail
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

/// **Two entries of one name are refused, not read as the first** (review 2026-09-26).
///
/// ZIP permits duplicate names and consumers disagree about which wins: this reader took the
/// first, exit 0 and nothing declared, where Python's `zipfile` and macOS's `textutil` read the
/// second. The ODF and EPUB readers always refused; the OOXML ones now do too. The single entry
/// reads, so the refusal is the duplicate's and nothing else's.
#[test]
fn two_entries_of_one_name_are_refused_rather_than_read_as_the_first() {
    let body = |text: &str| {
        format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:body></w:document>"#
        )
    };
    let (first, second) = (body("First"), body("Second"));
    let one = build_zip(&[("word/document.xml", first.as_str())]);
    let read = ethos_parser_office::read(&one).expect("one entry reads");
    assert_eq!(read.payload().nodes[0].text, "First");

    let twice = build_zip(&[
        ("word/document.xml", first.as_str()),
        ("word/document.xml", second.as_str()),
    ]);
    let error = ethos_parser_office::read(&twice).expect_err("two entries of one name");
    let text = error.to_string();
    assert!(
        matches!(error, ethos_parser_core::EngineError::Malformed { .. }),
        "{text}"
    );
    assert!(
        text.contains("`word/document.xml` more than once"),
        "{text}"
    );
}

// -------------------------------------------------------------------------------------------
// One paragraph, one block (decision #130)
// -------------------------------------------------------------------------------------------

/// `body`, one `<w:p>` per string, read and projected as Markdown and as HTML.
fn projected(paragraphs: &[&str]) -> (String, String) {
    let body: String = paragraphs
        .iter()
        .map(|p| format!("<w:p>{p}</w:p>"))
        .collect();
    let xml = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:wps="s" xmlns:v="v"><w:body>{body}</w:body></w:document>"#
    );
    let repr = ethos_parser_office::read(&build_zip(&[("word/document.xml", xml.as_str())]))
        .expect("reads");
    let profile = Profile::docx_v0().profile_sha256().expect("a digest");
    let md = ethos_parser_core::to_markdown(
        &repr,
        "test",
        &profile,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V14,
    )
    .expect("projects");
    let html = ethos_parser_core::to_html(
        &repr,
        "test",
        &profile,
        ethos_parser_core::HTML_RULE_BLOCKS_V14,
    )
    .expect("projects");
    (md.markdown, html.html)
}

/// **A label's tab is kept, on the label's line** (decision #130). Word writes `Company Name`, a
/// tab and `: Acme Ltd` as three runs, and each run was a block of its own, the tab dropped; a tab
/// inside one run read as a space.
#[test]
fn a_paragraphs_runs_are_one_block_with_its_tabs() {
    let (md, html) = projected(&[
        "<w:r><w:t>Company Name</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>: Acme Ltd</w:t></w:r>",
        "<w:r><w:t>Designation:</w:t><w:tab/><w:t>Senior Accountant</w:t></w:r>",
    ]);
    assert_eq!(
        md,
        "Company Name\t: Acme Ltd\n\nDesignation:\tSenior Accountant\n"
    );
    assert!(
        html.contains("<p>Company Name\t: Acme Ltd</p>")
            && html.contains("<p>Designation:\tSenior Accountant</p>"),
        "{html}"
    );
}

/// **A line break keeps its line**, in a run of its own and inside a run alike.
#[test]
fn a_paragraphs_line_breaks_are_kept() {
    let (md, html) = projected(&[
        "<w:r><w:t>Label A : one</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>Label B : two</w:t></w:r>",
        "<w:r><w:t>Label A : one</w:t><w:br/><w:t>Label B : two</w:t></w:r>",
    ]);
    assert_eq!(
        md,
        "Label A : one\nLabel B : two\n\nLabel A : one\nLabel B : two\n"
    );
    assert_eq!(
        html.matches("<p>Label A : one\nLabel B : two</p>").count(),
        2,
        "{html}"
    );
}

/// **A sentence with one bold phrase is one paragraph**, its spaces where its runs hold them, and a
/// word its runs split stays one word.
#[test]
fn a_sentence_cut_into_runs_is_one_paragraph() {
    let (md, html) = projected(&[
        r#"<w:r><w:t xml:space="preserve">Your annual cost is </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>Rs. 14,40,000</w:t></w:r><w:r><w:t>, paid monthly.</w:t></w:r>"#,
        "<w:r><w:t>Accou</w:t></w:r><w:r><w:t>ntant</w:t></w:r>",
    ]);
    assert_eq!(
        md,
        "Your annual cost is Rs. 14,40,000, paid monthly.\n\nAccountant\n"
    );
    assert!(
        html.contains("<p>Your annual cost is Rs. 14,40,000, paid monthly.</p>")
            && html.contains("<p>Accountant</p>"),
        "{html}"
    );
}

/// **A line that would read as Markdown of its own stays on the line before**, as decision #126
/// writes it, so a break before `- item` makes no list.
#[test]
fn a_break_before_a_line_markdown_reads_as_a_block_is_a_space() {
    let (md, _) = projected(&[
        "<w:r><w:t>Items:</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>- apples</w:t></w:r>",
        "<w:r><w:t>Total</w:t><w:br/><w:t>---</w:t></w:r>",
    ]);
    assert_eq!(md, "Items: - apples\n\nTotal ---\n");
}

/// **The line is read across the runs Word cut it into** (review of decision #130). Word splits a
/// run anywhere, so `1` and `. apples`, `` ` `` and ` `` `, `<` and `div>x`, `=` and `==` each
/// make one line that CommonMark reads as a block — an ordered list, a fence swallowing the
/// paragraphs after it, an HTML block, a heading's underline.
#[test]
fn a_line_markdown_reads_as_a_block_is_read_across_runs() {
    let (md, html) = projected(&[
        "<w:r><w:t>Items:</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:t>. apples</w:t></w:r>",
        "<w:r><w:t>Code:</w:t><w:br/><w:t>`</w:t></w:r><w:r><w:t>``</w:t></w:r>",
        "<w:r><w:t>Tag:</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>&lt;</w:t></w:r><w:r><w:t>div&gt;x</w:t></w:r>",
        "<w:r><w:t>Total</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>=</w:t></w:r><w:r><w:t>==</w:t></w:r>",
        "<w:r><w:t>Second paragraph</w:t></w:r>",
    ]);
    assert_eq!(
        md,
        "Items: 1. apples\n\nCode: ```\n\nTag: <div>x\n\nTotal ===\n\nSecond paragraph\n"
    );
    assert!(html.contains("<p>Items: 1. apples</p>"), "{html}");
}

/// **A text box is a paragraph of its own, and the text around it is its anchor's** (review of
/// decision #130): `Ira Testcase` and `Experience` were welded into one word, because the run after
/// the box took the box's paragraph.
#[test]
fn a_text_box_does_not_weld_into_the_paragraph_around_it() {
    let (md, html) = projected(&[
        "<w:r><w:t>Before</w:t></w:r><w:r><w:drawing><wps:txbx><w:txbxContent><w:p><w:r><w:t>Ira Testcase</w:t></w:r></w:p></w:txbxContent></wps:txbx></w:drawing></w:r><w:r><w:t>Experience</w:t></w:r>",
        "<w:r><w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>Box text</w:t></w:r></w:p></w:txbxContent></v:textbox></v:shape></w:pict></w:r><w:r><w:t>After</w:t></w:r>",
    ]);
    assert_eq!(
        md,
        "Before\n\nIra Testcase\n\nExperience\n\nBox text\n\nAfter\n"
    );
    assert!(
        html.contains("<p>Ira Testcase</p>") && html.contains("<p>After</p>"),
        "{html}"
    );
}

/// **Text around a text box in one run is kept** (review of decision #130): the box's run took the
/// anchoring run's place, so only `Box` reached the Markdown. The anchoring run is read whole,
/// before the box.
#[test]
fn text_around_a_text_box_in_one_run_is_kept() {
    let (md, html) = projected(&[
        r#"<w:r><w:t xml:space="preserve">Before </w:t><w:drawing><wps:txbx><w:txbxContent><w:p><w:r><w:t>Box</w:t></w:r></w:p></w:txbxContent></wps:txbx></w:drawing><w:t>After</w:t></w:r>"#,
    ]);
    assert_eq!(md, "Before After\n\nBox\n");
    assert!(
        html.contains("<p>Before After</p>") && html.contains("<p>Box</p>"),
        "{html}"
    );
}

/// **Whitespace a `<w:t>` does not preserve is the XML's own layout, not a line break** (review of
/// decision #130): a pretty-printed `<w:t>` read as `Pretty` and `printed` on two lines. A tab or
/// a break the run states stays, and so does a line feed `xml:space="preserve"` keeps. A
/// positional tab is a tab.
#[test]
fn whitespace_a_text_element_does_not_preserve_is_no_line_break() {
    let (md, _) = projected(&[
        "<w:r><w:t>\n        Pretty\n      </w:t></w:r><w:r><w:t>\n        printed\n      </w:t></w:r>",
        "<w:r><w:t>Indented\n    text</w:t></w:r>",
        "<w:r><w:t xml:space=\"preserve\">Hello\nWorld</w:t></w:r>",
        r#"<w:r><w:t>Software Engineer</w:t></w:r><w:r><w:ptab w:alignment="right"/></w:r><w:r><w:t>2019 - 2022</w:t></w:r>"#,
    ]);
    assert_eq!(
        md,
        "Pretty printed\n\nIndented text\n\nHello\nWorld\n\nSoftware Engineer\t2019 - 2022\n"
    );
}

// -------------------------------------------------------------------------------------------
// What a paragraph and a table declare, and the parts a section references (decision #136)
// -------------------------------------------------------------------------------------------

const W: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;
const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

/// A Word package: `body` with a section naming a header and a footer, its styles, and the header,
/// footer and footnotes parts, each bound by relationship.
fn letter(body: &str) -> Vec<u8> {
    let document = format!(
        r#"<w:document {W}><w:body>{body}<w:sectPr><w:headerReference w:type="default" r:id="rH"/><w:footerReference w:type="default" r:id="rF"/><w:headerReference w:type="even" r:id="rH"/></w:sectPr></w:body></w:document>"#
    );
    let styles = format!(
        r#"<w:styles {W}><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/></w:style></w:styles>"#
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rS" Type="{REL}/styles" Target="styles.xml"/><Relationship Id="rH" Type="{REL}/header" Target="header1.xml"/><Relationship Id="rF" Type="{REL}/footer" Target="footer1.xml"/><Relationship Id="rN" Type="{REL}/footnotes" Target="footnotes.xml"/></Relationships>"#
    );
    let header = format!(
        r#"<w:hdr {W}><w:p><w:r><w:t>Asha Patil</w:t></w:r><w:r><w:tab/></w:r><w:r><w:t>asha@example.com</w:t></w:r></w:p></w:hdr>"#
    );
    let footer =
        format!(r#"<w:ftr {W}><w:p><w:r><w:t>Registered office</w:t></w:r></w:p></w:ftr>"#);
    let footnotes = format!(
        r#"<w:footnotes {W}><w:footnote w:type="separator" w:id="-1"><w:p><w:r><w:separator/></w:r></w:p></w:footnote><w:footnote w:id="1"><w:p><w:r><w:footnoteRef/></w:r><w:r><w:t xml:space="preserve"> TDS is deducted.</w:t></w:r></w:p></w:footnote></w:footnotes>"#
    );
    build_zip(&[
        ("word/document.xml", document.as_str()),
        ("word/_rels/document.xml.rels", rels.as_str()),
        ("word/styles.xml", styles.as_str()),
        ("word/header1.xml", header.as_str()),
        ("word/footer1.xml", footer.as_str()),
        ("word/footnotes.xml", footnotes.as_str()),
        ("word/header2.xml", header.as_str()),
    ])
}

/// `repr` projected as Markdown and as HTML, leaving out page furniture where `drop` says.
fn project(repr: &ethos_parser_core::DocumentRepresentation, drop: bool) -> (String, String) {
    let profile = Profile::docx_v0().profile_sha256().expect("a digest");
    let mut options = ethos_parser_core::ProjectionOptions::default();
    options.drop_furniture = drop;
    let md = ethos_parser_core::to_markdown_with(
        repr,
        "test",
        &profile,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V14,
        options,
    )
    .expect("projects");
    let html = ethos_parser_core::to_html_with(
        repr,
        "test",
        &profile,
        ethos_parser_core::HTML_RULE_BLOCKS_V14,
        options,
    )
    .expect("projects");
    (md.markdown, html.html)
}

const LETTER_BODY: &str = concat!(
    r#"<w:p><w:pPr><w:pStyle w:val="Title"/></w:pPr><w:r><w:t>Offer Letter</w:t></w:r></w:p>"#,
    r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>1. Terms</w:t></w:r><w:r><w:br/></w:r><w:r><w:t>of work</w:t></w:r></w:p>"#,
    r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t xml:space="preserve">Basic </w:t></w:r><w:r><w:t>pay</w:t></w:r></w:p>"#,
    r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Metro rate</w:t></w:r></w:p>"#,
    r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Special</w:t></w:r></w:p>"#,
    r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Salary</w:t></w:r></w:p>"#,
    r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Component</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Monthly</w:t></w:r></w:p></w:tc></w:tr>"#,
    r#"<w:tr><w:tc><w:p><w:r><w:t>Bas</w:t></w:r><w:r><w:t>ic</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>60,000</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#,
    r#"<w:p><w:r><w:t>See the note</w:t></w:r><w:r><w:footnoteReference w:id="1"/></w:r><w:r><w:t xml:space="preserve"> for tax.</w:t></w:r></w:p>"#,
);

/// **Headings, list items, a table and the header, footer and footnote, each as the file declares
/// it** (decision #136). A heading is a style that declares a level — its own `<w:outlineLvl>` or
/// a built-in `heading N` name — and never a styleId alone, so `Title` stays a paragraph. A list
/// item is a `<w:numPr>`, at its `<w:ilvl>`, and always `- `: the number Word renders is in no
/// byte of the paragraph. The header, once though two references name it, comes before the body;
/// the footer after it; the footnote last, with no number, since Word renders that too.
#[test]
fn a_documents_declared_structure_reaches_both_projections() {
    let repr = ethos_parser_office::read(&letter(LETTER_BODY)).expect("reads");
    let (md, html) = project(&repr, false);
    assert_eq!(
        md,
        "Asha Patil\tasha@example.com\n\nOffer Letter\n\n# 1. Terms of work\n\n- Basic pay\n  - Metro rate\n- Special\n\n## Salary\n\n| Component | Monthly |\n| --- | --- |\n| Basic | 60,000 |\n\nSee the note for tax.\n\nRegistered office\n\nTDS is deducted.\n"
    );
    for want in [
        "<h1>1. Terms of work</h1>",
        "<li>Basic pay",
        "<li>Metro rate</li>",
        "<h2>Salary</h2>",
        "<td>Basic</td>",
        "<p>Asha Patil\tasha@example.com</p>",
        "<p>TDS is deducted.</p>",
    ] {
        assert!(html.contains(want), "{want} in {html}");
    }

    let payload = repr.payload();
    let parts: Vec<&str> = payload
        .nodes
        .iter()
        .filter_map(|n| match &n.native_locator {
            NativeLocator::Docx(d) => Some(d.part.as_str()),
            _ => None,
        })
        .fold(Vec::new(), |mut v, p| {
            if v.last() != Some(&p) {
                v.push(p);
            }
            v
        });
    assert_eq!(
        parts,
        [
            "word/header1.xml",
            "word/document.xml",
            "word/footer1.xml",
            "word/footnotes.xml"
        ]
    );
    let [table] = payload.tables.as_slice() else {
        panic!("one table: {:?}", payload.tables);
    };
    assert_eq!((table.rows, table.columns, table.cells.len()), (2, 2, 4));
    assert_eq!(table.detection_rule, "docx-tbl-v1");
    assert_eq!(
        table.derivation,
        ethos_parser_core::DerivationClass::Extracted
    );
    assert!(matches!(table.geometry, GeometryPresence::Absent(_)));
    assert_eq!(table.cells[2].text, "Basic", "its runs, concatenated");
    let body = payload
        .nodes
        .iter()
        .find(|n| n.text == "Offer Letter")
        .expect("the body");
    assert_eq!(table.page, body.parent, "the part it was read from");
    assert!(payload.assurance.capabilities.tables);
    // Neither `tables-not-extracted`, now false, nor the drawn-grid limitation, which a table read
    // from markup does not share.
    for code in [
        ethos_parser_core::assurance::codes::TABLES_NOT_EXTRACTED,
        ethos_parser_core::assurance::codes::UNDRAWN_TABLE_EDGES_NOT_SUPPLIED,
    ] {
        assert!(
            !payload.assurance.limitations.iter().any(|l| l.code == code),
            "{code}"
        );
    }

    // `header2.xml` is named by nothing, so it is still unread, and counted.
    let unread = payload
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .expect("an unreferenced header is declared");
    assert!(unread.detail.starts_with("1 part(s)"), "{}", unread.detail);
}

/// **A header and a footer are page furniture, left out on request and counted** (decision #136,
/// with #134's option): a section sets them on its pages by declaration. A footnote is no
/// furniture and stays.
#[test]
fn a_header_and_a_footer_go_on_request_and_a_footnote_stays() {
    let repr = ethos_parser_office::read(&letter(LETTER_BODY)).expect("reads");
    let (md, html) = project(&repr, true);
    assert!(md.starts_with("Offer Letter\n\n"), "{md}");
    assert!(
        md.ends_with("See the note for tax.\n\nTDS is deducted.\n"),
        "{md}"
    );
    assert!(
        !html.contains("Asha Patil") && !html.contains("Registered"),
        "{html}"
    );
}

/// **A level or a numbering a tracked change replaced is not the paragraph's**, and a level past
/// six projects as a paragraph, as an ODF one does.
#[test]
fn a_replaced_or_unrepresentable_level_projects_as_a_paragraph() {
    let repr = ethos_parser_office::read(&letter(concat!(
        r#"<w:p><w:pPr><w:pPrChange><w:pPr><w:pStyle w:val="Heading1"/><w:numPr><w:numId w:val="2"/></w:numPr></w:pPr></w:pPrChange></w:pPr><w:r><w:t>Was a heading</w:t></w:r></w:p>"#,
        r#"<w:p><w:pPr><w:outlineLvl w:val="7"/></w:pPr><w:r><w:t>Level eight</w:t></w:r></w:p>"#,
    )))
    .expect("reads");
    let (md, html) = project(&repr, true);
    assert_eq!(md, "Was a heading\n\nLevel eight\n\nTDS is deducted.\n");
    assert!(html.contains("<p>Level eight</p>"), "{html}");
    let profile = Profile::docx_v0().profile_sha256().expect("a digest");
    let artifact = ethos_parser_core::to_markdown(
        &repr,
        "test",
        &profile,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V14,
    )
    .expect("projects");
    assert!(
        artifact
            .coverage
            .structural_erasures
            .iter()
            .any(|e| e.code == ethos_parser_core::HEADING_LEVEL_UNREPRESENTABLE),
        "the flattened level is counted: {:?}",
        artifact.coverage.structural_erasures
    );
}

/// A Word package: `body` (its own `<w:sectPr>` included), the relationships `rels` as
/// `(id, type, target)`, and the parts `parts` as `(name, bytes)`.
fn word(body: &str, rels: &[(&str, &str, &str)], parts: &[(&str, &str)]) -> Vec<u8> {
    let document = format!(r#"<w:document {W}><w:body>{body}</w:body></w:document>"#);
    let rels: String = rels
        .iter()
        .map(|(id, kind, target)| {
            format!(r#"<Relationship Id="{id}" Type="{REL}/{kind}" Target="{target}"/>"#)
        })
        .collect();
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
    );
    let mut entries = vec![
        ("word/document.xml", document.as_str()),
        ("word/_rels/document.xml.rels", rels.as_str()),
    ];
    entries.extend_from_slice(parts);
    build_zip(&entries)
}

fn header(text: &str) -> String {
    format!(r#"<w:hdr {W}><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:hdr>"#)
}

fn unread(repr: &ethos_parser_core::DocumentRepresentation) -> Option<String> {
    repr.payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_core::assurance::codes::OFFICE_PARTS_NOT_READ)
        .map(|l| l.detail.clone())
}

/// **A first-page header is read only where its section states `<w:titlePg/>`, an even-page one
/// only where the settings state `<w:evenAndOddHeaders/>`** (review of decision #136): Word shows
/// neither otherwise, and both printed. One passed over is counted as not read.
#[test]
fn a_header_word_does_not_show_is_not_read_and_is_counted() {
    let rels = [
        ("h1", "header", "header1.xml"),
        ("h2", "header", "header2.xml"),
        ("h3", "header", "header3.xml"),
        ("s", "settings", "settings.xml"),
    ];
    let (h1, h2, h3) = (header("Running"), header("First only"), header("Even only"));
    let sect = |title: &str| {
        format!(
            r#"<w:p><w:r><w:t>Body</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="h1"/><w:headerReference w:type="first" r:id="h2"/><w:headerReference w:type="even" r:id="h3"/>{title}</w:sectPr>"#
        )
    };
    let read = |title: &str, even: &str| {
        let settings = format!(r#"<w:settings {W}>{even}</w:settings>"#);
        let repr = ethos_parser_office::read(&word(
            &sect(title),
            &rels,
            &[
                ("word/header1.xml", h1.as_str()),
                ("word/header2.xml", h2.as_str()),
                ("word/header3.xml", h3.as_str()),
                ("word/settings.xml", settings.as_str()),
            ],
        ))
        .expect("reads");
        (project(&repr, false).0, unread(&repr))
    };
    let (md, declared) = read("", "");
    assert_eq!(md, "Running\n\nBody\n");
    assert!(declared.expect("counted").starts_with("2 part(s)"));
    let (md, declared) = read("<w:titlePg/>", "<w:evenAndOddHeaders/>");
    assert_eq!(md, "Running\n\nFirst only\n\nEven only\n\nBody\n");
    assert_eq!(declared, None);
}

/// **A header, footer or styles part that will not read is counted, and the rest is read**
/// (review of decision #136): a truncated or non-UTF-8 header, or a style name with a bare `&`, refused
/// the whole document, where before this decision it read.
#[test]
fn a_part_that_will_not_read_is_counted_and_the_rest_read() {
    let body = r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Body</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#;
    // Truncated: elements still open at its end, which `read_part` refuses.
    let bad_header = format!("<w:hdr {W}><w:p><w:r><w:t>Header</w:t></w:r>");
    let repr = ethos_parser_office::read(&word(
        body,
        &[("h1", "header", "header1.xml")],
        &[("word/header1.xml", bad_header.as_str())],
    ))
    .expect("the body still reads");
    assert_eq!(project(&repr, false).0, "Body\n");
    assert!(unread(&repr).expect("counted").starts_with("1 part(s)"));

    let styles = format!(
        r#"<w:styles {W}><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading & 1"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr></w:style></w:styles>"#
    );
    let repr = ethos_parser_office::read(&word(
        body,
        &[("s", "styles", "styles.xml")],
        &[("word/styles.xml", styles.as_str())],
    ))
    .expect("the body still reads");
    assert_eq!(
        project(&repr, false).0,
        "Body\n",
        "no level from styles not read"
    );
    assert!(
        unread(&repr).expect("counted").contains("word/styles.xml"),
        "{:?}",
        unread(&repr)
    );
}

/// **A list opens at the top and steps in one level at a time** (review of decision #136): an item
/// at `<w:ilvl w:val="2"/>` with no list open wrote `    - Starts deep`, an indented code block,
/// and a jump from level one to four lazy text; in HTML, a `<ul>` holding only `<ul>`s.
#[test]
fn a_list_steps_in_one_level_at_a_time() {
    let item = |level: u32, text: &str| {
        format!(
            r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
        )
    };
    let body = [
        item(2, "Starts deep"),
        "<w:p><w:r><w:t>Between</w:t></w:r></w:p>".into(),
        item(0, "Top"),
        item(3, "Jump"),
        item(3, "Same"),
        item(0, "Back"),
    ]
    .concat();
    let repr = ethos_parser_office::read(&word(&body, &[], &[])).expect("reads");
    let (md, html) = project(&repr, false);
    assert_eq!(
        md,
        "- Starts deep\n\nBetween\n\n- Top\n  - Jump\n  - Same\n- Back\n"
    );
    assert!(
        !html.contains("<ul><ul>") && !html.contains("<ul>\n<ul>"),
        "{html}"
    );
}

/// **A referenced part that will not read is counted whatever its name** (review of decision #136):
/// a truncated header at `word/custom/header.xml` matched no conventional name, and went
/// uncounted; one at a conventional name is counted once, not twice.
#[test]
fn a_referenced_part_that_will_not_read_is_counted_whatever_its_name() {
    let body = r#"<w:p><w:r><w:t>Body</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#;
    let bad_header = format!("<w:hdr {W}><w:p><w:r><w:t>Header</w:t></w:r>");
    for (target, part) in [
        ("custom/header.xml", "word/custom/header.xml"),
        ("header1.xml", "word/header1.xml"),
    ] {
        let repr = ethos_parser_office::read(&word(
            body,
            &[("h1", "header", target)],
            &[(part, bad_header.as_str())],
        ))
        .expect("the body still reads");
        assert_eq!(project(&repr, false).0, "Body\n");
        assert!(
            unread(&repr).expect("counted").starts_with("1 part(s)"),
            "{part}: {:?}",
            unread(&repr)
        );
    }
}

/// **The element view keeps the labels a DOCX declares** (review of decision #137): a referenced
/// header's paragraph is a `page-header`, and a paragraph with a declared level a `section-header`
/// at that level, as both projections read them; both were `text`.
#[test]
fn the_element_view_keeps_a_headers_and_a_headings_label() {
    use ethos_parser_core::ElementLabel::{PageHeader, SectionHeader, Text};
    let header = format!(r#"<w:hdr {W}><w:p><w:r><w:t>Running head</w:t></w:r></w:p></w:hdr>"#);
    let repr = ethos_parser_office::read(&word(
        r#"<w:p><w:pPr><w:outlineLvl w:val="1"/></w:pPr><w:r><w:t>Scope</w:t></w:r></w:p><w:p><w:r><w:t>Body text</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#,
        &[("h1", "header", "header1.xml")],
        &[("word/header1.xml", header.as_str())],
    ))
    .expect("reads");
    let elements = ethos_parser_core::to_elements(&repr, "test")
        .expect("projects")
        .elements;
    let got: Vec<_> = elements
        .iter()
        .map(|e| (e.label, e.text.as_str(), e.level))
        .collect();
    assert_eq!(
        got,
        vec![
            (PageHeader, "Running head", None),
            (SectionHeader, "Scope", Some(2)),
            (Text, "Body text", None),
        ]
    );
}

/// **A list goes back out to the depth its level was set at** (review of decision #136): levels 2,
/// 3, 2 wrote the third item under the second, where Word sets it beside the first, in Markdown and
/// HTML alike; a level between two set ones goes under the shallower.
#[test]
fn a_list_goes_back_out_to_the_depth_its_level_was_set_at() {
    let item = |level: u32, text: &str| {
        format!(
            r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
        )
    };
    let body = [
        item(2, "First"),
        item(3, "Nested"),
        item(2, "Back"),
        item(4, "Deep"),
        item(3, "Between"),
        "<w:p><w:r><w:t>Prose</w:t></w:r></w:p>".into(),
        item(3, "Again"),
    ]
    .concat();
    let repr = ethos_parser_office::read(&word(&body, &[], &[])).expect("reads");
    let (md, html) = project(&repr, false);
    assert_eq!(
        md,
        "- First\n  - Nested\n- Back\n  - Deep\n  - Between\n\nProse\n\n- Again\n"
    );
    assert_eq!(
        html,
        "<ul>\n<li>First<ul>\n<li>Nested</li>\n</ul>\n</li>\n<li>Back<ul>\n<li>Deep</li>\n<li>Between</li>\n</ul>\n</li>\n</ul>\n<p>Prose</p>\n<ul>\n<li>Again</li>\n</ul>\n"
    );
}

/// **A header's table goes with the header, on request, and is counted** (review of decision #136):
/// a letterhead table survived `--drop-furniture`, uncounted. By default it stays.
#[test]
fn a_headers_table_goes_with_it_on_request() {
    let header = format!(
        r#"<w:hdr {W}><w:tbl><w:tr><w:tc><w:p><w:r><w:t>Logo co</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Ref 123</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:hdr>"#
    );
    let repr = ethos_parser_office::read(&word(
        r#"<w:p><w:r><w:t>Body text</w:t></w:r></w:p><w:sectPr><w:headerReference w:type="default" r:id="h1"/></w:sectPr>"#,
        &[("h1", "header", "header1.xml")],
        &[("word/header1.xml", header.as_str())],
    ))
    .expect("reads");
    assert_eq!(
        project(&repr, false).0,
        "| Logo co | Ref 123 |\n| --- | --- |\n\nBody text\n"
    );
    let (md, html) = project(&repr, true);
    assert_eq!(md, "Body text\n");
    assert!(!html.contains("Logo"), "{html}");
    let profile = Profile::docx_v0().profile_sha256().expect("a digest");
    let mut options = ethos_parser_core::ProjectionOptions::default();
    options.drop_furniture = true;
    let artifact = ethos_parser_core::to_markdown_with(
        &repr,
        "test",
        &profile,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V14,
        options,
    )
    .expect("projects");
    let bucket = artifact
        .coverage
        .dropped
        .iter()
        .find(|b| b.code == "page-furniture-not-projected-v1")
        .expect("counted");
    assert_eq!((bucket.chars, bucket.nodes), (14, 2));
    assert!(artifact.coverage.balances());
}

/// **A table with no run in it is not written at the document's end** (review of decision #136):
/// it has no place in reading order, so it is counted as a table with no grid is, and its cells'
/// text stays in the record. Two paragraphs of one cell are two words in the record, not one, and
/// two blocks in the projection, the table being a layout table.
#[test]
fn an_empty_table_is_counted_and_a_cells_paragraphs_are_two_words() {
    let body = concat!(
        "<w:p><w:r><w:t>Para one</w:t></w:r></w:p>",
        "<w:tbl><w:tr><w:tc><w:p/></w:tc><w:tc><w:p/></w:tc></w:tr></w:tbl>",
        "<w:p><w:r><w:t>Para two</w:t></w:r></w:p>",
        "<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Line one</w:t></w:r></w:p><w:p><w:r><w:t>Line two</w:t></w:r></w:p></w:tc></w:tr></w:tbl>",
    );
    let repr = ethos_parser_office::read(&word(body, &[], &[])).expect("reads");
    let (md, html) = project(&repr, false);
    assert_eq!(md, "Para one\n\nPara two\n\nLine one\n\nLine two\n");
    assert!(!html.contains("<td></td>"), "{html}");
    assert_eq!(repr.payload().tables[1].cells[0].text, "Line one Line two");
    let profile = Profile::docx_v0().profile_sha256().expect("a digest");
    let artifact = ethos_parser_core::to_markdown(
        &repr,
        "test",
        &profile,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V14,
    )
    .expect("projects");
    assert!(artifact
        .coverage
        .structural_erasures
        .iter()
        .any(|e| e.code == ethos_parser_core::GFM_TABLE_NOT_PROJECTED));
}

/// The structure census of `repr`'s Markdown: each code with its count.
fn erasures(repr: &ethos_parser_core::DocumentRepresentation) -> Vec<(String, usize)> {
    let profile = Profile::docx_v0().profile_sha256().expect("a digest");
    ethos_parser_core::to_markdown(
        repr,
        "test",
        &profile,
        ethos_parser_core::MARKDOWN_RULE_BLOCKS_V14,
    )
    .expect("projects")
    .coverage
    .structural_erasures
    .into_iter()
    .map(|e| (e.code, e.count))
    .collect()
}

fn p(text: &str) -> String {
    format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
}

/// **A resume laid out as one Word table reads a paragraph per line, in document order** (decision
/// #136, real resumes): a cell holding two paragraphs or more makes a layout table, written as the
/// blocks it holds, row by row and cell by cell, a heading and a list item projecting as they do
/// outside a table, where the cell's paragraphs were one line of one grid cell. Its record stays a
/// table, and it is counted. A grid table, one paragraph a cell, stays GFM.
#[test]
fn a_resume_laid_out_as_a_table_reads_a_paragraph_per_line() {
    let heading = |level: u32, text: &str| {
        format!(
            r#"<w:p><w:pPr><w:outlineLvl w:val="{level}"/></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
        )
    };
    let item = r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Built dashboards</w:t></w:r></w:p>"#;
    let body = [
        "<w:tbl><w:tr><w:tc>".to_string(),
        heading(0, "Asha Patil"),
        p("asha@example.com"),
        "</w:tc><w:tc>".into(),
        heading(1, "Experience"),
        p("Analyst, Acme Ltd"),
        item.into(),
        "</w:tc></w:tr></w:tbl>".into(),
        "<w:tbl><w:tr><w:tc>".into(),
        p("Degree"),
        // A paragraph holding only a tab is no second paragraph with text.
        "<w:p><w:r><w:tab/></w:r></w:p>".into(),
        "</w:tc><w:tc>".into(),
        p("Year"),
        "</w:tc></w:tr></w:tbl>".into(),
    ]
    .concat();
    let repr = ethos_parser_office::read(&word(&body, &[], &[])).expect("reads");
    let (md, html) = project(&repr, false);
    assert_eq!(
        md,
        "# Asha Patil\n\nasha@example.com\n\n## Experience\n\nAnalyst, Acme Ltd\n\n- Built dashboards\n\n| Degree | Year |\n| --- | --- |\n"
    );
    assert!(
        html.contains("<h1>Asha Patil</h1>") && html.contains("<li>Built dashboards</li>"),
        "{html}"
    );
    assert_eq!(html.matches("<table>").count(), 1, "{html}");
    assert_eq!(
        repr.payload().tables.len(),
        2,
        "the record keeps both tables"
    );
    assert!(erasures(&repr).contains(&("layout-table-as-blocks-v1".to_string(), 1)));
}

/// **A table nested in a cell is written where it stands** (decision #136, real resumes): after the
/// cell's paragraphs before it and before the cell after it, a grid of its own, where it was
/// written after the whole outer table and a section heading lost its content. A table holding a
/// nested table is a layout table.
#[test]
fn a_nested_table_is_written_where_it_stands() {
    let body = [
        "<w:tbl><w:tr><w:tc>".to_string(),
        p("Education"),
        "<w:tbl><w:tr><w:tc>".into(),
        p("Degree"),
        "</w:tc><w:tc>".into(),
        p("Year"),
        "</w:tc></w:tr><w:tr><w:tc>".into(),
        p("B.Com"),
        "</w:tc><w:tc>".into(),
        p("2019"),
        "</w:tc></w:tr></w:tbl><w:p/></w:tc><w:tc>".into(),
        p("Skills"),
        "</w:tc></w:tr></w:tbl>".into(),
    ]
    .concat();
    let repr = ethos_parser_office::read(&word(&body, &[], &[])).expect("reads");
    let (md, html) = project(&repr, false);
    assert_eq!(
        md,
        "Education\n\n| Degree | Year |\n| --- | --- |\n| B.Com | 2019 |\n\nSkills\n"
    );
    let at = |s: &str| html.find(s).unwrap_or_else(|| panic!("{s} in {html}"));
    assert!(
        at("Education") < at("<table>") && at("</table>") < at("Skills"),
        "{html}"
    );
    assert!(erasures(&repr).contains(&("layout-table-as-blocks-v1".to_string(), 1)));
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// A minimal stored-entry ZIP, so a negative case can be authored without a fixture file.
fn build_zip(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();

    for (name, body) in entries {
        let offset = out.len() as u32;
        let data = body.as_bytes();
        let crc = crc32(data);

        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[10, 0, 0, 0, 0, 0, 0, 0, 0, 0]); // version, flags, method, time
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
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// **A paragraph's declared numbering is a list item in the element view** (decision #166), as
/// both projections write it `- `; a heading stays a section header and a `Title` style text.
#[test]
fn a_declared_list_item_is_a_list_item_element() {
    let repr = ethos_parser_office::read(&letter(LETTER_BODY)).expect("reads");
    let elements = ethos_parser_core::to_elements(&repr, "test").expect("projects");
    let label = |text: &str| {
        elements
            .elements
            .iter()
            .find(|e| e.text == text)
            .map(|e| e.label)
            .unwrap_or_else(|| panic!("no element `{text}`"))
    };
    use ethos_parser_core::ElementLabel;
    assert_eq!(label("Basic pay"), ElementLabel::ListItem);
    assert_eq!(label("Metro rate"), ElementLabel::ListItem);
    assert_eq!(label("Special"), ElementLabel::ListItem);
    assert_eq!(label("Offer Letter"), ElementLabel::Text);
    assert_eq!(label("Salary"), ElementLabel::SectionHeader);
}

/// **A DOCX table's leading `<w:tblHeader/>` rows are its header** (decision #167): their cells
/// say so on the record, and in the element view each other cell names the header over it. A
/// marked row below an unmarked one is no header, as Word repeats none.
#[test]
fn a_tables_marked_leading_rows_are_its_header() {
    let body = concat!(
        r#"<w:tbl><w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p><w:r><w:t>Item</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cost</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"<w:tr><w:tc><w:p><w:r><w:t>Pen</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc></w:tr>"#,
        r#"<w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p><w:r><w:t>Ink</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>5</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#,
    );
    let repr = ethos_parser_office::read(&letter(body)).expect("reads");
    let table = &repr.payload().tables[0];
    let headers: Vec<(&str, bool)> = table
        .cells
        .iter()
        .map(|c| (c.text.as_str(), c.header))
        .collect();
    assert_eq!(
        headers,
        [
            ("Item", true),
            ("Cost", true),
            ("Pen", false),
            ("2", false),
            ("Ink", false),
            ("5", false)
        ]
    );
    let elements = ethos_parser_core::to_elements(&repr, "test").expect("projects");
    let grid = elements
        .elements
        .iter()
        .find_map(|e| e.table.as_ref())
        .expect("a table element");
    let cost = &table.cells[1].id;
    let two = grid.cells.iter().find(|c| c.text == "2").expect("a cell");
    assert_eq!(two.column_headers, [cost.clone()]);
    assert!(two.row_headers.is_empty());
    assert!(
        grid.cells[1].column_headers.is_empty(),
        "a header names none"
    );
}
