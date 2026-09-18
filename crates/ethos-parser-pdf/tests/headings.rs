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

//! **Headings inferred from type, on the documents this repository authored for them**
//! (decision #29, `docs/28-HEADINGS-SCOPE.md` §10 S1).
//!
//! The rule itself — the cut, the body em, the whole-line test, the exclusions — is unit-tested
//! beside it in `src/headings.rs`. These are the acceptance tests that need a real page: that the
//! verdict reaches the representation, that the gate is the catalog's declaration, that the
//! rendered em and not the `Tf` operand is what is read, and that the profile's rule id is the
//! switch. **A missing corpus is a failure, never a skip.**

use std::path::PathBuf;

use ethos_parser_core::{DocumentRepresentation, NodeAttributes, Profile, NOT_RUN};
use ethos_parser_pdf::Document;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn engine_fx(name: &str) -> PathBuf {
    let p = repo_root().join(format!("fixtures/engine/{name}/document.pdf"));
    assert!(
        p.is_file(),
        "fixture `{name}` missing at {}. A missing corpus is a failure, never a skip.",
        p.display()
    );
    p
}

fn represent(name: &str, profile: &Profile) -> DocumentRepresentation {
    let doc = Document::open(&engine_fx(name), profile).expect("the fixture opens");
    let extract = ethos_parser_pdf::extract(&doc, profile).expect("the fixture extracts");
    ethos_parser_pdf::to_representation(&extract, profile).expect("it represents")
}

/// `(text, font_size, inferred_heading)` for every text run, in reading order.
fn runs(repr: &DocumentRepresentation) -> Vec<(String, i64, bool)> {
    repr.payload()
        .nodes
        .iter()
        .filter_map(|n| match &n.attributes {
            NodeAttributes::TextRun(a) => Some((n.text.clone(), a.font_size, a.inferred_heading)),
            _ => None,
        })
        .collect()
}

fn inferred(repr: &DocumentRepresentation) -> Vec<String> {
    runs(repr)
        .into_iter()
        .filter(|(_, _, h)| *h)
        .map(|(t, _, _)| t)
        .collect()
}

fn declared(repr: &DocumentRepresentation, code: &str) -> Option<String> {
    repr.payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == code)
        .map(|l| l.detail.clone())
}

const HEADINGS: &str = ethos_parser_core::codes::HEADINGS_INFERRED_FROM_TYPE;
const UNTAGGED: &str = ethos_parser_core::codes::UNTAGGED_STRUCTURE_TREE_ABSENT;

/// **One display line at twice the body type, on a page that declares no structure, is an
/// inferred heading — and nothing else on the page is.**
///
/// The artifact declares it: one line, the rule that fired, and the body reference measured on
/// this document, so a reader can re-derive the cut without the engine. The precondition stays
/// declared beside it, because the catalog still has no `/StructTreeRoot`.
#[test]
fn a_display_line_on_an_untagged_page_is_an_inferred_heading() {
    let repr = represent("heading-display-line", &Profile::default());

    assert_eq!(
        inferred(&repr),
        vec!["Display line".to_string()],
        "exactly the display line's run, and no body line"
    );
    let detail = declared(&repr, HEADINGS).expect("the inference is declared");
    for fact in [
        "1 line(s)",
        "`type-size-v2`",
        "1200 centipoints",
        "NONE is the author's",
    ] {
        assert!(
            detail.contains(fact),
            "the declaration states {fact}: {detail}"
        );
    }
    assert!(
        declared(&repr, UNTAGGED).is_some(),
        "`untagged-structure-tree-absent` stays declared: it is the precondition, not a duplicate"
    );
}

/// **The same page plus a `/StructTreeRoot` infers nothing**, and declares neither code.
///
/// The tree cites nothing and binds no run. The gate is still closed, because it is the
/// catalog's *declaration* of structure — a tree that reaches nothing is still a declaration,
/// and filling around an author's structure invents a relation they did not state (§6.1).
#[test]
fn the_same_page_with_a_tree_infers_nothing() {
    let repr = represent("heading-display-line-tagged", &Profile::default());

    assert!(
        inferred(&repr).is_empty(),
        "a document that declares structure gets no inferred heading, whatever its type"
    );
    assert!(declared(&repr, HEADINGS).is_none());
    assert!(
        declared(&repr, UNTAGGED).is_none(),
        "a tree was declared, so `untagged-structure-tree-absent` would be false"
    );
    assert_eq!(
        runs(&repr)
            .iter()
            .map(|(t, s, _)| (t.clone(), *s))
            .collect::<Vec<_>>(),
        runs(&represent("heading-display-line", &Profile::default()))
            .iter()
            .map(|(t, s, _)| (t.clone(), *s))
            .collect::<Vec<_>>(),
        "and it is the same page: the same runs at the same sizes, only the tree differs"
    );
}

/// **A `Tf` operand of 1 still finds the heading** — §3.1's measurement as a test.
///
/// This fixture carries its type in the text matrix under `/F1 1 Tf`, so `font_size` is 100 on
/// every run: the field `font_size` says nothing here, as it says nothing on 87 of the 200 bench
/// documents. The rule reads the rendered em, so the page reads exactly as its twin does. This is
/// the test that fails if a later edit reaches for the operand.
#[test]
fn a_tf_operand_of_one_still_finds_the_heading() {
    let tf_one = represent("heading-display-line-tf-one", &Profile::default());
    let twin = represent("heading-display-line", &Profile::default());

    assert!(
        runs(&tf_one).iter().all(|(_, size, _)| *size == 100),
        "the fixture is what it claims: every run's `Tf` operand is 1pt"
    );
    assert_eq!(inferred(&tf_one), inferred(&twin));
    assert_eq!(
        declared(&tf_one, HEADINGS),
        declared(&twin, HEADINGS),
        "the same line, and the same body reference measured"
    );
}

/// **The profile's rule id is the switch.** A profile naming anything but the rule runs no
/// inference, the way `READING_ORDER_RULE_V0` is the cut's switch — no capability flag, and
/// nothing to set anywhere but the id the artifact already carries.
#[test]
fn a_profile_that_names_no_heading_rule_infers_nothing() {
    let profile = Profile {
        heading_inference_rule: NOT_RUN.to_string(),
        ..Profile::default()
    };
    let repr = represent("heading-display-line", &profile);

    assert!(inferred(&repr).is_empty());
    assert!(declared(&repr, HEADINGS).is_none());
    assert!(
        declared(&repr, UNTAGGED).is_some(),
        "the document is still untagged, and still says so"
    );
}

/// **Absent where false, on the wire.** The key appears once — on the one heading run — so a
/// run the rule did not touch serializes exactly as it did before the rule existed, and a
/// document where it fires nowhere is byte-identical apart from the profile hash.
#[test]
fn the_field_is_on_the_wire_only_where_it_is_true() {
    let bytes = represent("heading-display-line", &Profile::default())
        .to_canonical_bytes()
        .expect("canonicalizes");
    let text = String::from_utf8(bytes).expect("utf-8");
    assert_eq!(text.matches("\"inferred_heading\":true").count(), 1);
    assert_eq!(
        text.matches("\"inferred_heading\"").count(),
        1,
        "never serialized as false"
    );
}

/// **A page this engine tagged infers exactly what its untagged original infers** — the gate's
/// second arm (§6.1).
///
/// `tag` writes a tree whose every element is this engine's own and whose every binding reads
/// back `Computed`, which is no declaration: the licence to be the author's structure does not
/// transfer to a tree the engine wrote. Without this arm the tagged page would lose its heading,
/// and its projections would stop equalling the original's (docs/23 §4.3).
#[test]
fn an_engine_tagged_page_infers_what_its_untagged_original_does() {
    let profile = Profile::default();
    let original = Document::open(&engine_fx("heading-display-line"), &profile).expect("opens");
    let tagged_bytes =
        ethos_parser_pdf::write_tags(&original, &profile).expect("the writer tags the page");
    let tagged = Document::open_bytes(&tagged_bytes, &profile).expect("the tagged page opens");
    let extract = ethos_parser_pdf::extract(&tagged, &profile).expect("it extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("it represents");
    let twin = represent("heading-display-line", &profile);

    assert!(
        declared(
            &repr,
            ethos_parser_core::codes::STRUCTURE_TREE_ENGINE_WRITTEN
        )
        .is_some(),
        "the precondition: this is a tree the engine wrote, and the artifact says so"
    );
    assert_eq!(inferred(&repr), vec!["Display line".to_string()]);
    assert_eq!(
        declared(&repr, HEADINGS),
        declared(&twin, HEADINGS),
        "the same line and the same body reference as the untagged original"
    );
}

// -------------------------------------------------------------------------------------------
// S3: the tree-stripped twin
// -------------------------------------------------------------------------------------------

fn gate_fx(name: &str) -> PathBuf {
    let root = std::env::var_os("ETHOS_GATE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("fixtures/gate"));
    let p = root.join(format!("{name}.pdf"));
    assert!(
        p.is_file(),
        "gate document `{name}` missing at {}. A missing corpus is a failure, never a skip.",
        p.display()
    );
    p
}

fn represent_bytes(bytes: &[u8], profile: &Profile) -> DocumentRepresentation {
    let doc = Document::open_bytes(bytes, profile).expect("the document opens");
    let extract = ethos_parser_pdf::extract(&doc, profile).expect("it extracts");
    ethos_parser_pdf::to_representation(&extract, profile).expect("it represents")
}

/// Every text run's `(text, page, baseline, band)` — the facts the rule's line is built from.
fn lines_of(repr: &DocumentRepresentation) -> Vec<(String, u32, i64, Option<u32>)> {
    repr.payload()
        .nodes
        .iter()
        .filter_map(|n| match (&n.attributes, &n.native_locator) {
            (NodeAttributes::TextRun(a), ethos_parser_core::NativeLocator::Pdf(loc)) => {
                Some((n.text.clone(), loc.page, loc.origin_y, a.region))
            }
            _ => None,
        })
        .collect()
}

/// **The tree-stripped twin** (`docs/28-HEADINGS-SCOPE.md` §7.3, S3): a gate document with its
/// `/StructTreeRoot` removed inside the test, through `lopdf`.
///
/// `docs/measurements/headings/falsepos.py` measures the shipped rule's false-positive rate on the
/// eleven documents whose authors declared headings, and it can only do that by stripping each
/// document's tree — the gate keeps the verdict off a tagged document's wire. This test holds the
/// two things that measurement stands on: **the shipped build fires on a gate document once its tree
/// is gone**, so the instrument is measuring the rule and not an empty set; and **stripping the tree
/// changes nothing the rule reads** — the same runs, in the same order, on the same baselines and in
/// the same bands — so the instrument's node-by-node join of the original's labels to the stripped
/// copy's verdicts is a join of one document to itself.
#[test]
fn a_gate_document_stripped_of_its_tree_is_where_the_rule_fires() {
    let profile = Profile::default();
    let bytes = std::fs::read(gate_fx("irs-fw9")).expect("reads");
    let mut doc = lopdf::Document::load_mem(&bytes).expect("lopdf loads");
    let root = doc
        .trailer
        .get(b"Root")
        .and_then(lopdf::Object::as_reference)
        .expect("a /Root");
    let removed = doc
        .get_object_mut(root)
        .and_then(lopdf::Object::as_dict_mut)
        .expect("a catalog")
        .remove(b"StructTreeRoot");
    assert!(
        removed.is_some(),
        "the gate document declares a tree to strip"
    );
    let mut stripped = Vec::new();
    doc.save_to(&mut stripped).expect("saves");

    let original = represent_bytes(&bytes, &profile);
    let twin = represent_bytes(&stripped, &profile);

    assert_eq!(
        lines_of(&original),
        lines_of(&twin),
        "stripping the tree changed a run the rule reads, so a join by position would compare two \
         different documents"
    );
    assert!(
        inferred(&original).is_empty(),
        "the gate is closed on the author's tagged original"
    );
    assert!(
        !inferred(&twin).is_empty(),
        "and the shipped build fires once the tree is gone, so the instrument measures a rule and \
         not an empty set"
    );
    assert!(declared(&twin, HEADINGS).is_some());
    assert!(declared(&twin, UNTAGGED).is_some());
}
