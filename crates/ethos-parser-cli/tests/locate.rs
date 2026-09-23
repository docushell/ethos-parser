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

//! **`locate` over the documents this repository authored** (`docs/26-LOCATE-SCOPE.md` §9, D1 S1).
//!
//! The unit tests beside the query (`ethos-parser-core/src/locate.rs`) run over hand-built
//! representations, which is right for the units and the refusals and cannot say anything about a
//! real document's runs. These run over parsed fixtures, and the fixtures are the ones whose own
//! comments were written about the strings being searched for here.
//!
//! # Why it lives in `ethos-parser-cli`
//!
//! The same reason `library_surface.rs` does: it is the only crate that depends on both readers,
//! and `ethos-parser-core` may not depend on either. Nothing here spawns the binary — there is no
//! `locate` subcommand yet, and S2 is where one arrives.

use std::path::PathBuf;

use ethos_parser_core::{
    locate, DocumentRepresentation, GeometryAbsence, GeometryPresence, Locations, OccurrencePart,
    Profile,
};
use ethos_parser_pdf::Document;
use serde_json::Value;

// -------------------------------------------------------------------------------------------
// Fixtures
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn path_in(root_name: &str, rel: &str) -> PathBuf {
    let m: Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest"),
    )
    .expect("valid JSON");
    let decl = &m["roots"][root_name];
    let root = match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    };
    let p = root.join(rel);
    assert!(
        p.is_file(),
        "fixture `{rel}` missing at {}. A missing corpus is a failure, never a skip.",
        p.display()
    );
    p
}

/// A fixture's representation, as `ethos-parser extract` would write it.
fn pdf_repr(name: &str) -> DocumentRepresentation {
    let profile = Profile::default();
    let path = path_in("engine", &format!("{name}/document.pdf"));
    let doc = Document::open(&path, &profile).expect("the fixture opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("the fixture extracts");
    ethos_parser_pdf::to_representation(&extract, &profile).expect("it represents")
}

/// An office fixture's representation. The office corpus is not a manifest root — the readers'
/// own tests resolve it by repository path, and so does this.
fn office_repr(name: &str, file: &str) -> DocumentRepresentation {
    let path = repo_root().join("fixtures/office").join(name).join(file);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    ethos_parser_office::read(&bytes).expect("the fixture reads")
}

fn found_in(repr: &DocumentRepresentation, quote: &str) -> Locations {
    let profile = Profile::default();
    locate(
        repr,
        &profile.parser_version,
        &profile.profile_sha256().expect("the profile hashes"),
        &profile.locate_rule,
        quote,
    )
    .expect("a representation and a quote locate")
}

/// One part's own slice of its own node's text — the only way to read an offset correctly.
fn slice_of(repr: &DocumentRepresentation, part: &OccurrencePart) -> String {
    let node = repr
        .payload()
        .nodes
        .iter()
        .find(|n| n.id.as_str() == part.node.as_str())
        .expect("a part names a node of the representation it came from");
    node.text
        .chars()
        .skip(part.char_start as usize)
        .take((part.char_end - part.char_start) as usize)
        .collect()
}

/// **T12, the invariant** — the parts' own slices, concatenated, equal the quote. And **T13** —
/// every emitted geometry is the sidecar's own row for that node, copied and not derived.
///
/// Called by every test below rather than written once as a test of its own: an invariant asserted
/// on one document is an invariant nobody checked on the others.
fn the_invariants_hold(repr: &DocumentRepresentation, found: &Locations, quote: &str) {
    for (i, occurrence) in found.occurrences.iter().enumerate() {
        assert!(
            !occurrence.parts.is_empty(),
            "occurrence {i} locates nothing"
        );
        let joined: String = occurrence
            .parts
            .iter()
            .map(|p| slice_of(repr, p))
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(
            joined, quote,
            "occurrence {i}'s parts, concatenated, must be the quote — offsets are into each \
             node's own text, never the block's"
        );
        for part in &occurrence.parts {
            let index = repr
                .payload()
                .nodes
                .iter()
                .position(|n| n.id.as_str() == part.node.as_str())
                .expect("the part names a node");
            assert_eq!(
                Some(part.geometry),
                repr.geometry_at(index),
                "an occurrence's geometry is the representation's own row, never a new rectangle"
            );
        }
    }
}

/// Each occurrence as `(node text, char_start, char_end)` per part — what a failure should print.
fn shape(repr: &DocumentRepresentation, found: &Locations) -> Vec<Vec<(String, u32, u32)>> {
    found
        .occurrences
        .iter()
        .map(|o| {
            o.parts
                .iter()
                .map(|p| {
                    let node = repr
                        .payload()
                        .nodes
                        .iter()
                        .find(|n| n.id.as_str() == p.node.as_str())
                        .expect("a part names a node");
                    (node.text.clone(), p.char_start, p.char_end)
                })
                .collect()
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// Found, and not found
// -------------------------------------------------------------------------------------------

/// **T1** — one run, one occurrence, one part, and the geometry the sidecar already carried.
///
/// `measured-ink-box` is the only fixture whose geometry takes the measured branch, which is why
/// it is the one that can say a box travelled rather than a typed absence.
#[test]
fn a_measured_run_answers_with_one_occurrence_of_one_part() {
    let repr = pdf_repr("measured-ink-box");
    let found = found_in(&repr, "Measured");

    assert_eq!(shape(&repr, &found), vec![vec![("Measured".into(), 0, 8)]]);
    assert_eq!(found.occurrences[0].synthesized, 0);
    assert!(
        matches!(
            found.occurrences[0].parts[0].geometry,
            GeometryPresence::Measured(_)
        ),
        "the fixture's box is measured, and the occurrence carries that and not a re-derivation"
    );
    assert_eq!(found.quote_scalars, 8);
    assert_eq!(found.searched.nodes, 1);
    assert_eq!(found.searched.blocks, 1);
    assert_eq!(found.searched.scalars, 8);
    the_invariants_hold(&repr, &found, "Measured");
}

/// **T2** — a string the document does not contain is an **answer**: `Ok`, an empty list, and the
/// same artifact a found quote produces. Decision #30's whole point is that *whether* is not this
/// engine's question, so *not found* cannot be an error.
#[test]
fn a_string_the_document_does_not_contain_is_an_empty_answer() {
    let repr = pdf_repr("measured-ink-box");
    let found = found_in(&repr, "Absent");

    assert!(found.occurrences.is_empty());
    assert!(found.occurrences_withheld.is_none());
    assert_eq!(found.identity.artifact_type, "ethos.parser.locations.v0");
    assert_eq!(found.searched.blocks, 1, "the block was searched");
    assert_eq!(
        &found.representation_sha256,
        repr.fingerprint(),
        "and the answer names the representation it searched"
    );
    found
        .to_canonical_bytes()
        .expect("an empty answer is an artifact like any other");
}

/// **T3** — every occurrence, in reading order, across blocks. **Reading order is not a ranking**
/// and nothing here prefers one to another.
///
/// `untagged-shredded-line` draws `Yar`, `ro`, `w` abutting on one baseline — one block, text
/// `Yarrow` — and `Separate` 4 000 centipoints along, which is its own. So `r` occurs three times
/// in two blocks, and the order is the order the page drew them.
#[test]
fn every_occurrence_arrives_in_reading_order_across_blocks() {
    let repr = pdf_repr("untagged-shredded-line");
    let found = found_in(&repr, "r");

    assert_eq!(found.searched.blocks, 2, "`Yarrow`, and `Separate`");
    assert_eq!(
        shape(&repr, &found),
        vec![
            vec![("Yar".into(), 2, 3)],
            vec![("ro".into(), 0, 1)],
            vec![("Separate".into(), 4, 5)],
        ]
    );
    the_invariants_hold(&repr, &found, "r");
}

/// **T4** — a quote crossing runs inside one block gets a part per run, each with offsets into
/// **its own** text. This is the occurrence a caller could not have assembled from the runs alone.
#[test]
fn a_quote_crossing_runs_of_one_block_has_a_part_per_run() {
    let repr = pdf_repr("untagged-shredded-line");
    let found = found_in(&repr, "arrow");

    assert_eq!(
        shape(&repr, &found),
        vec![vec![
            ("Yar".into(), 1, 3),
            ("ro".into(), 0, 2),
            ("w".into(), 0, 1),
        ]]
    );
    the_invariants_hold(&repr, &found, "arrow");
}

/// **T5** — a quote crossing a block boundary is not an occurrence. A boundary is a gap the page
/// drew; joining it would assert an adjacency the document does not have.
///
/// `YarrowSeparate` is the string `untagged-shredded-line`'s own comment was written about.
#[test]
fn a_quote_crossing_a_block_boundary_is_not_an_occurrence() {
    let shredded = pdf_repr("untagged-shredded-line");
    assert!(found_in(&shredded, "YarrowSeparate").occurrences.is_empty());
    assert_eq!(
        found_in(&shredded, "Yarrow").occurrences.len(),
        1,
        "the block's own text is still found whole"
    );

    let two = pdf_repr("markdown-two-blocks");
    assert!(found_in(&two, "First blockSecond block")
        .occurrences
        .is_empty());
    assert_eq!(found_in(&two, "First block").occurrences.len(), 1);
    assert_eq!(found_in(&two, "Second block").occurrences.len(), 1);
}

/// **T6** — text that exists only after a projection is not text the document contains.
///
/// Both strings are the ones `docs/history/11-V11-MILESTONES.md` built these fixtures to mark
/// unquotable: the hyphen rejoin's `recalculated`, which no run holds, and the Markdown's own
/// paragraph break, which is a rendering.
#[test]
fn text_that_exists_only_after_a_projection_is_not_an_occurrence() {
    let hyphen = pdf_repr("markdown-hyphen-break");
    assert!(
        found_in(&hyphen, "recalculated").occurrences.is_empty(),
        "the join is the Markdown's; the document drew a hyphen and a line break"
    );
    assert_eq!(found_in(&hyphen, "recalcu-").occurrences.len(), 1);
    assert_eq!(found_in(&hyphen, "lated at closing").occurrences.len(), 1);

    let two = pdf_repr("markdown-two-blocks");
    assert!(found_in(&two, "block\n\nSecond").occurrences.is_empty());
}

/// **T8** — a character this reader synthesized is still one of the occurrence's scalars, and the
/// count says so. A consumer matching a quote against a space the source does not contain is the
/// failure `docs/01-CONTRACT.md` §10 names, and this is the field that can warn them.
#[test]
fn a_synthesized_character_inside_an_occurrence_is_declared() {
    let repr = pdf_repr("synthesized-space-tj");
    let found = found_in(&repr, "one two");

    assert_eq!(
        shape(&repr, &found),
        vec![vec![("one ".into(), 0, 4), ("two".into(), 0, 3)]],
        "the gap `[(one) -500 (two)] TJ` drew is a space in the first run's own text"
    );
    assert_eq!(
        found.occurrences[0].synthesized, 1,
        "one of these seven scalars was inserted by this reader, not read from the page"
    );
    the_invariants_hold(&repr, &found, "one two");

    // The same run, quoted without the synthesized scalar, declares none.
    let without = found_in(&repr, "one");
    assert_eq!(without.occurrences[0].synthesized, 0);
    // And T1's fixture, where nothing was ever inserted.
    let measured = pdf_repr("measured-ink-box");
    assert_eq!(
        found_in(&measured, "Measured").occurrences[0].synthesized,
        0
    );
}

/// **T9** — right-to-left text is searched **as it was drawn**, because that is what the run
/// holds. `rtl-hebrew-visual-order` draws four CIDs in visual order, so the logical word does not
/// occur and its reversal does.
///
/// This test records a limitation rather than hiding one: `docs/CAPABILITY.md`'s bidi row states
/// it, and **since 2026-09-23 the artifact states it too** — `right-to-left-not-reordered`,
/// document-scoped, counting the runs affected. `locate` itself is unchanged and still matches on
/// scalars exactly; what changed is that a caller getting an empty answer here can now see from
/// the representation why. A reader who wants the logical word needs a bidi reordering this engine does
/// not do.
#[test]
fn right_to_left_text_is_searched_as_the_page_drew_it() {
    let repr = pdf_repr("rtl-hebrew-visual-order");
    // Shin, lamed, vav, final mem — the word as it is typed and stored.
    let logical = "\u{5e9}\u{5dc}\u{5d5}\u{5dd}";
    // The same four scalars in the order the content stream paints them, which is the run's text.
    let as_drawn = "\u{5dd}\u{5d5}\u{5dc}\u{5e9}";

    assert!(
        found_in(&repr, logical).occurrences.is_empty(),
        "the logical word is not in the run, and inventing it would be a reordering nobody asked \
         this engine for"
    );
    let found = found_in(&repr, as_drawn);
    assert_eq!(shape(&repr, &found), vec![vec![(as_drawn.into(), 0, 4)]]);
    assert_eq!(found.quote_scalars, 4);
    the_invariants_hold(&repr, &found, as_drawn);
}

/// **T18** — an office representation answers too, with geometry **typed absent** rather than
/// omitted. A DOCX run has no box, and the occurrence says which kind of nothing that is.
///
/// It also fixes what a page-less document's blocks are: the block rule reads ink along a
/// baseline, a DOCX run has neither, so **every run is its own block**. A quote crossing two runs
/// of one paragraph is therefore not an occurrence here — the same rule as a PDF's block
/// boundary, reaching the same answer for a different reason, and the second assertion is the one
/// that would notice if it ever silently joined them.
#[test]
fn an_office_representation_answers_with_typed_absent_geometry() {
    let repr = office_repr("simple-paragraphs", "document.docx");
    let found = found_in(&repr, "Evidence, not extraction.");

    assert_eq!(
        shape(&repr, &found),
        vec![vec![("Evidence, not extraction.".into(), 0, 25)]]
    );
    assert_eq!(
        found.occurrences[0].parts[0].geometry,
        GeometryPresence::Absent(GeometryAbsence::NotApplicableToKind),
        "typed absence, never an omitted field and never a zero box"
    );
    the_invariants_hold(&repr, &found, "Evidence, not extraction.");

    assert_eq!(
        (found.searched.nodes, found.searched.blocks),
        (4, 4),
        "four runs, four blocks: a run with no baseline joins nothing"
    );
    assert!(
        found_in(&repr, "a run and never to a page")
            .occurrences
            .is_empty(),
        "`A quote binds to a run` and ` and never to a page.` are two runs of one paragraph, and \
         two blocks"
    );
    assert_eq!(
        found_in(&repr, "A quote binds to a run").occurrences.len(),
        1
    );
}
