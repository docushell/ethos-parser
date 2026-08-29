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

//! v2-S11: an embedded asset is an erasure, and the three OOXML readers now say how many.
//!
//! **The gap this file was written to reproduce.** `02-ROADMAP.md`'s v2 row names *embedded
//! assets* as v2 content. v2-S2 listed them **Out** for that slice and no later slice picked them
//! up, so they were neither delivered nor descoped. That alone would be an omission. What makes it
//! an **A14** violation — *if something is removed, the artifact says so and says how much* — is
//! that in three readers of eight they were not merely unread but **uncounted**: `word/media/`,
//! `xl/media/` and `ppt/media/` matched no bucket at all, so a DOCX with forty embedded images
//! declared **zero** parts not read for them, while ODT, ODS, ODP, RTF and EPUB had counted theirs
//! since the slice that added each reader.
//!
//! **It was also unexercised.** None of the three OOXML fixtures contained a single media entry
//! when this slice began, so nothing in the suite could have noticed. The fixtures gained media
//! first and this file asserted the declared count before the readers changed — at which point
//! every assertion below failed, because the count did not move.

use std::path::PathBuf;

use ethos_parser_core::assurance::codes;

fn fixture(dir: &str, file: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("fixtures/office")
        .join(dir)
        .join(file);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The detail of one limitation code, or `None` if the artifact does not carry it.
fn limitation(bytes: &[u8], code: &str) -> Option<String> {
    let sealed = ethos_parser_office::read(bytes).expect("the fixture reads");
    let found = sealed
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == code)
        .map(|l| l.detail.clone());
    found
}

// -------------------------------------------------------------------------------------------
// Each OOXML reader declares its embedded assets, and the count is the package's
// -------------------------------------------------------------------------------------------

#[test]
fn a_docx_declares_the_two_images_it_embeds() {
    let detail = limitation(
        &fixture("unread-parts", "document.docx"),
        codes::OFFICE_EMBEDDED_PARTS_NOT_READ,
    )
    .expect("a package with `word/media/` entries declares them");
    assert!(
        detail.starts_with("2 part(s)"),
        "the fixture embeds two images and the count is the package's: {detail}"
    );
}

#[test]
fn an_xlsx_declares_the_one_image_it_embeds() {
    let detail = limitation(
        &fixture("workbook-unread-parts", "workbook.xlsx"),
        codes::OFFICE_EMBEDDED_PARTS_NOT_READ,
    )
    .expect("a package with `xl/media/` entries declares them");
    assert!(detail.starts_with("1 part(s)"), "{detail}");
}

#[test]
fn a_pptx_declares_the_three_images_it_embeds() {
    let detail = limitation(
        &fixture("deck-unread-parts", "deck.pptx"),
        codes::OFFICE_EMBEDDED_PARTS_NOT_READ,
    )
    .expect("a package with `ppt/media/` entries declares them");
    assert!(detail.starts_with("3 part(s)"), "{detail}");
}

// -------------------------------------------------------------------------------------------
// And a package with none declares none — the code is absent, not present reading zero
// -------------------------------------------------------------------------------------------

#[test]
fn a_package_with_no_embedded_asset_declares_none() {
    for (dir, file) in [
        ("simple-paragraphs", "document.docx"),
        ("workbook-cells", "workbook.xlsx"),
        ("deck-slides", "deck.pptx"),
    ] {
        let bytes = fixture(dir, file);
        assert!(
            limitation(&bytes, codes::OFFICE_EMBEDDED_PARTS_NOT_READ).is_none(),
            "{dir} embeds nothing, so the code must be ABSENT rather than present reading zero — \
             `docs/01-CONTRACT.md` §9: a list of codes all reading 0 is disclosure-shaped noise"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The two buckets stay two, and each message stays true of what it fires on
// -------------------------------------------------------------------------------------------

/// **The reason this is a second code and not a widened first one.**
///
/// `office-parts-not-read`'s message says its parts *"carry text"*. A PNG does not. Adding
/// `word/media/` to that bucket's prefix list would have made a shipped sentence false about every
/// file it fired on — a counter's *meaning* changed in place, which is what v2-S9.1 was forbidden
/// from doing. So the text count must be exactly the three text parts, unchanged by media landing
/// in the same package.
#[test]
fn embedded_assets_do_not_enter_the_text_bucket() {
    let cases = [
        ("unread-parts", "document.docx", "3 part(s)"),
        ("workbook-unread-parts", "workbook.xlsx", "3 part(s)"),
        ("deck-unread-parts", "deck.pptx", "3 part(s)"),
    ];
    for (dir, file, expected) in cases {
        let detail = limitation(&fixture(dir, file), codes::OFFICE_PARTS_NOT_READ)
            .expect("each fixture carries unread text parts");
        assert!(
            detail.starts_with(expected),
            "{dir}: the text count must not have absorbed the media parts: {detail}"
        );
        assert!(
            detail.contains("carry text"),
            "{dir}: the shipped sentence is intact"
        );
    }
}

/// Neither message may claim the other's kind. A PNG is never described as carrying text.
#[test]
fn no_message_says_carry_text_about_an_embedded_asset() {
    for (dir, file) in [
        ("unread-parts", "document.docx"),
        ("workbook-unread-parts", "workbook.xlsx"),
        ("deck-unread-parts", "deck.pptx"),
    ] {
        let detail = limitation(&fixture(dir, file), codes::OFFICE_EMBEDDED_PARTS_NOT_READ)
            .expect("declared");
        assert!(
            !detail.contains("carry text"),
            "{dir}: an embedded asset carries no text and the message must not say it does: \
             {detail}"
        );
    }
}
