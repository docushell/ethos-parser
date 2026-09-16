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

//! Auto-tagging S1: a tag this engine writes is one it reads back as its own
//! (`docs/24-AUTO-TAGGING-MILESTONES.md` §S1, `docs/23-AUTO-TAGGING-SCOPE.md` §4).
//!
//! Every fixture here was written **by hand** in the writer's exact shape
//! (`fixtures/engine/make_fixtures.py`), before the writer exists, so a mistake the writer and
//! the reader might share cannot pass. The failure modes are made in the test by editing
//! `engine-tagged-blocks` through `lopdf`, so each is one edit away from the shape that reads
//! correctly — and a control proves the re-serialisation alone changes nothing.
//!
//! Fixtures resolve through `fixtures/manifest.json`'s `engine` root. **A missing fixture is a
//! failure, never a skip.**

use std::path::PathBuf;

use ethos_parser_core::{
    codes, DerivationClass, EngineError, GeometryAbsence, GeometryPresence, Limitation,
    LimitationScope, PdfTaggedLocator, Profile, StructuralLocator, TABLE_DETECTION_TAGGED_V1,
};
use ethos_parser_pdf::{Document, ExtractArtifact};
use lopdf::{Dictionary, Object, ObjectId};

// -------------------------------------------------------------------------------------------
// Fixture resolution
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn engine_fixture(name: &str) -> Vec<u8> {
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest readable"),
    )
    .expect("manifest is valid JSON");
    let decl = &manifest["roots"]["engine"];
    let root = match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    };
    let p = root.join(name).join("document.pdf");
    std::fs::read(&p).unwrap_or_else(|e| {
        panic!(
            "fixture `{name}` missing at {}: {e}. A missing fixture is a failure, never a skip.",
            p.display()
        )
    })
}

// -------------------------------------------------------------------------------------------
// Reading
// -------------------------------------------------------------------------------------------

/// The six lines of the leading-gap page, in stream order, which is also reading order.
const LINES: [&str; 6] = [
    "Water finds its level",
    "and stone keeps its shape",
    "through the long season",
    "Wind moves the grass",
    "and light moves the shade",
    "across the open field",
];

fn extract_bytes(bytes: &[u8]) -> Result<ExtractArtifact, EngineError> {
    let profile = Profile::default();
    let doc = Document::open_bytes(bytes, &profile)?;
    ethos_parser_pdf::extract(&doc, &profile)
}

/// What the reader said about a document: every run's text and address, and the limitations.
///
/// Compared whole between variants, because "reads identically" means the bindings AND the
/// declarations — a variant that bound the same runs and declared something different would
/// not be reading identically.
#[derive(Debug, PartialEq, Eq)]
struct Read {
    bindings: Vec<(String, Option<StructuralLocator>)>,
    limitations: Vec<Limitation>,
}

fn read(bytes: &[u8]) -> Read {
    let a = extract_bytes(bytes).expect("the document extracts");
    Read {
        bindings: a
            .runs()
            .map(|r| (r.text.clone(), r.structural.clone()))
            .collect(),
        limitations: a.assurance.limitations.clone(),
    }
}

fn tagged(locator: &Option<StructuralLocator>) -> &PdfTaggedLocator {
    match locator {
        Some(StructuralLocator::PdfTagged(t)) => t,
        other => panic!("expected a `pdf_tagged` locator, got {other:?}"),
    }
}

fn codes_of(limitations: &[Limitation]) -> Vec<&str> {
    limitations.iter().map(|l| l.code.as_str()).collect()
}

/// The one engine-written declaration, which must be document-scoped and present once.
fn engine_written(limitations: &[Limitation]) -> &Limitation {
    let found: Vec<&Limitation> = limitations
        .iter()
        .filter(|l| l.code == codes::STRUCTURE_TREE_ENGINE_WRITTEN)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "`structure-tree-engine-written` is declared once per document: {:?}",
        codes_of(limitations)
    );
    assert_eq!(found[0].scope, LimitationScope::Document);
    found[0]
}

/// The codes an engine-tagged page must NOT declare: a tree was read and every cited id bound.
const NOT_ON_AN_ENGINE_TAGGED_PAGE: [&str; 3] = [
    codes::UNTAGGED_STRUCTURE_TREE_ABSENT,
    codes::STRUCTURE_MCID_UNBOUND,
    codes::STRUCTURE_ITEM_WITHOUT_CONTENT,
];

/// Assert the shape every engine-tagged variant of the leading-gap page reads back to: six runs,
/// each bound `Computed` under `Document/Div` to the id `mcids` names for it.
fn assert_engine_tagged(what: &str, a: &ExtractArtifact, mcids: [i64; 6]) {
    let runs: Vec<_> = a.runs().collect();
    assert_eq!(runs.len(), 6, "{what}: six runs");
    for (i, run) in runs.iter().enumerate() {
        assert_eq!(run.text, LINES[i], "{what}: run {} text", i + 1);
        let t = tagged(&run.structural);
        assert_eq!(
            t.role_path,
            vec!["Document".to_string(), "Div".to_string()],
            "{what}: run {} role path",
            i + 1
        );
        assert_eq!(
            t.derivation,
            DerivationClass::Computed,
            "{what}: run {} binds under this engine's own element",
            i + 1
        );
        assert_eq!(t.mcid, mcids[i], "{what}: run {} mcid", i + 1);
        assert_eq!(
            run.mcid,
            Some(mcids[i]),
            "{what}: run {} carries its id",
            i + 1
        );
        assert!(
            t.standard_role_path.is_none(),
            "{what}: nothing was remapped"
        );
        assert!(t.element_id.is_none(), "{what}: the writer mints no /ID");
        // The TEXT is the document's own; only the address was computed (scope §4.2).
        assert_eq!(
            run.derivation,
            DerivationClass::Extracted,
            "{what}: run {} text is read from the document's own encoding",
            i + 1
        );
    }
    let codes = codes_of(&a.assurance.limitations);
    for code in NOT_ON_AN_ENGINE_TAGGED_PAGE {
        assert!(
            !codes.contains(&code),
            "{what}: `{code}` must not be declared on an engine-tagged page: {codes:?}"
        );
    }
    let written = engine_written(&a.assurance.limitations);
    for needle in [
        "3 of its 3 structure element(s)",
        "6 text run(s)",
        "gutter-columns-v3",
        "no author structure tree",
        "NONE is the author's",
    ] {
        assert!(
            written.detail.contains(needle),
            "{what}: the declaration must say {needle:?}: {}",
            written.detail
        );
    }
}

// -------------------------------------------------------------------------------------------
// Editing the fixture through lopdf
// -------------------------------------------------------------------------------------------

/// Load, edit, and re-serialise `engine-tagged-blocks`.
///
/// The edit sees the whole document so a test can reach the catalog, an element, or add an
/// object. `save_to` rewrites every object, which is why every test that uses this also reads
/// the UNEDITED re-serialisation once: the refusal or the change has to be the edit's.
fn edited(edit: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    edited_from("engine-tagged-blocks", edit)
}

/// Load, edit, and re-serialise any engine fixture.
fn edited_from(name: &str, edit: impl FnOnce(&mut lopdf::Document)) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(&engine_fixture(name)).expect("lopdf loads");
    edit(&mut doc);
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("lopdf saves");
    out
}

/// An attribute object under this engine's owner: the writer's shape when `derivation` is
/// `Computed`, and a shape it never writes otherwise.
fn owner_attribute(derivation: &[u8], rule: &str) -> Dictionary {
    let mut a = Dictionary::new();
    a.set("O", Object::Name(b"EthosParser".to_vec()));
    a.set("Derivation", Object::Name(derivation.to_vec()));
    a.set("Rule", Object::string_literal(rule));
    a
}

/// The one `/StructElem` whose `/S` is `role`.
fn elem_with_role(doc: &lopdf::Document, role: &[u8]) -> ObjectId {
    let ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter(|(_, o)| {
            let Ok(d) = o.as_dict() else {
                return false;
            };
            d.get(b"Type").ok().and_then(|t| t.as_name().ok()) == Some(b"StructElem".as_slice())
                && d.get(b"S").ok().and_then(|s| s.as_name().ok()) == Some(role)
        })
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(
        ids.len(),
        1,
        "exactly one /{} element",
        String::from_utf8_lossy(role)
    );
    ids[0]
}

/// The `/StructTreeRoot` dictionary, reached through the catalog.
fn struct_root_mut(doc: &mut lopdf::Document) -> &mut Dictionary {
    let root = catalog_mut(doc)
        .get(b"StructTreeRoot")
        .expect("the fixture is tagged")
        .as_reference()
        .expect("the root is a reference");
    doc.get_object_mut(root)
        .expect("root exists")
        .as_dict_mut()
        .expect("root is a dictionary")
}

/// Repaint `tagged-table-agrees`'s four cells in one column with no ruling, so no detector finds
/// a table and the tree's `/Table` reaches the wire as a tagged-table record.
///
/// A tree table is paired by position with the nth table a detector found on its page, and then
/// becomes that record's `tagged_check` rather than a record of its own. On this page the painted
/// grid pairs it; with the grid merely stripped the 2x2 of letters still pairs it, because the
/// unruled rule reads them as a table (`unruled-align-v1`, 2 rows, 2 columns — measured with the
/// CLI on a stripped copy). Four lines in one column at one x is a shape no rule calls a table.
/// The same four `Tj`s keep the same mcids, so the cells still claim their text through the
/// tree's own join, and nothing but the page's geometry moves.
fn lay_the_cells_in_one_column(doc: &mut lopdf::Document) {
    // `build_pdf` numbers every engine fixture's content stream 4.
    let Ok(Object::Stream(stream)) = doc.get_object_mut((4, 0)) else {
        panic!("object 4 is the content stream");
    };
    let plain = String::from_utf8(stream.content.clone()).expect("the stream is plain text");
    for cell in ["(A) Tj", "(B) Tj", "(C) Tj", "(D) Tj"] {
        assert!(plain.contains(cell), "the fixture shows {cell}");
    }
    stream.set_plain_content(
        b"BT /F1 12 Tf \
          /TD <</MCID 0>> BDC 1 0 0 1 50 94 Tm (A) Tj EMC \
          /TD <</MCID 1>> BDC 1 0 0 1 50 80 Tm (B) Tj EMC \
          /TD <</MCID 2>> BDC 1 0 0 1 50 66 Tm (C) Tj EMC \
          /TD <</MCID 3>> BDC 1 0 0 1 50 52 Tm (D) Tj EMC ET"
            .to_vec(),
    );
}

/// The object ids of every `/StructElem` in the document, in object order: 7, 8, 9.
fn struct_elems(doc: &lopdf::Document) -> Vec<ObjectId> {
    let mut ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter(|(_, o)| {
            o.as_dict()
                .ok()
                .and_then(|d| d.get(b"Type").ok())
                .and_then(|t| t.as_name().ok())
                == Some(b"StructElem".as_slice())
        })
        .map(|(id, _)| *id)
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids.len(),
        3,
        "the fixture holds /Document and two /Div elements"
    );
    ids
}

fn elem_mut(doc: &mut lopdf::Document, id: ObjectId) -> &mut Dictionary {
    doc.get_object_mut(id)
        .expect("element exists")
        .as_dict_mut()
        .expect("element is a dictionary")
}

/// The element's inline `/A` dictionary, taken out of the element.
fn take_attribute(elem: &mut Dictionary) -> Dictionary {
    match elem.remove(b"A") {
        Some(Object::Dictionary(d)) => d,
        other => panic!("the fixture writes /A inline: {other:?}"),
    }
}

fn catalog_mut(doc: &mut lopdf::Document) -> &mut Dictionary {
    let root = doc
        .trailer
        .get(b"Root")
        .expect("trailer names a root")
        .as_reference()
        .expect("root is a reference");
    doc.get_object_mut(root)
        .expect("catalog exists")
        .as_dict_mut()
        .expect("catalog is a dictionary")
}

// -------------------------------------------------------------------------------------------
// The tests
// -------------------------------------------------------------------------------------------

/// **The reads-back clause, on the file the writer will emit** (milestones S1, acceptance 1).
///
/// All six runs bind `pdf_tagged` under `Document/Div` with `derivation: computed`, runs 1–3 to
/// mcid 0 and 4–6 to mcid 1; the text stays `Extracted`; the artifact says whose tree it read
/// and does not say the document is untagged. The representation round-trips through JSON with
/// its fingerprint intact, so the new key is on the wire and hashed.
#[test]
fn an_engine_written_tag_reads_back_as_computed() {
    let a = extract_bytes(&engine_fixture("engine-tagged-blocks")).expect("extracts");
    assert_engine_tagged("engine-tagged-blocks", &a, [0, 0, 0, 1, 1, 1]);

    let repr = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let text_nodes: Vec<_> = repr
        .payload()
        .nodes
        .iter()
        .filter(|n| n.kind == ethos_parser_core::NodeKind::TextRun)
        .collect();
    assert_eq!(text_nodes.len(), 6);
    for node in &text_nodes {
        assert_eq!(
            node.derivation,
            DerivationClass::Extracted,
            "`Node.derivation` stays Extracted: the text was read, the address was computed"
        );
        assert_eq!(
            tagged(&node.structural_locator).derivation,
            DerivationClass::Computed
        );
    }

    // The wire: written as `"derivation":"computed"` on every tagged locator, parsed back, and
    // the fingerprint still verifies — decision #20's field is hashed like any other.
    let json = serde_json::to_string(&repr).expect("serialises");
    assert_eq!(
        json.matches("\"derivation\":\"computed\"").count(),
        6,
        "one `computed` per tagged run, on the locator and nowhere else"
    );
    let back: ethos_parser_core::DocumentRepresentation =
        serde_json::from_str(&json).expect("parses back");
    back.verify_fingerprint().expect("the fingerprint verifies");
    assert_eq!(back, repr);
}

/// **A representation from before the field is refused, not read as the author's** (scope §8).
///
/// Drop `derivation` from one tagged locator on the wire: `deny_unknown_fields` on the way in is
/// only half of decision #20; the other half is that a missing field is missing, never
/// `extracted`.
#[test]
fn a_locator_without_its_derivation_is_refused_on_the_wire() {
    let a = extract_bytes(&engine_fixture("engine-tagged-blocks")).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
    let json = serde_json::to_string(&repr).expect("serialises");
    let stripped = json.replacen(",\"derivation\":\"computed\"", "", 1);
    assert_ne!(stripped, json, "the key was present to strip");
    let err = serde_json::from_str::<ethos_parser_core::DocumentRepresentation>(&stripped)
        .expect_err("a tagged locator with no derivation must not parse");
    assert!(
        err.to_string().contains("missing field `derivation`"),
        "refused by name, never defaulted: {err}"
    );
}

/// **The attribute is read through `/C` and the root's `/ClassMap` too** (acceptance 2).
///
/// `engine-tagged-classmap` carries no `/A` anywhere; `engine-tagged-mixed` carries a FOREIGN
/// owner under `/A` and the engine's class through `/C`. Both must read identically to the
/// inline form — bindings and declarations alike — because a tool that factors a repeated
/// attribute dictionary into a class is the *ignored* case decision #23 names.
#[test]
fn the_attribute_is_read_through_the_class_map_too() {
    let inline = read(&engine_fixture("engine-tagged-blocks"));
    for name in ["engine-tagged-classmap", "engine-tagged-mixed"] {
        let a = extract_bytes(&engine_fixture(name)).expect("extracts");
        assert_engine_tagged(name, &a, [0, 0, 0, 1, 1, 1]);
        assert_eq!(
            read(&engine_fixture(name)),
            inline,
            "{name} must read exactly as the inline form"
        );
    }
}

/// **A written sequence inside an existing frame still binds** (acceptance 3).
///
/// Block 1's first line sits in `/Span BMC … EMC` and its second in `/OC /oc1 BDC … EMC` given
/// by name; the written `/Div` sequence opens inside each frame, so the innermost open sequence
/// — the one the reader binds by — is the written one. Block 1 is therefore ids 0, 1 and 2 and
/// block 2 is id 3. The named `/OC` list is declared exactly as it is today.
#[test]
fn a_nested_frame_does_not_hide_the_binding() {
    let a = extract_bytes(&engine_fixture("engine-tagged-nested-frames")).expect("extracts");
    assert_engine_tagged("engine-tagged-nested-frames", &a, [0, 1, 2, 3, 3, 3]);

    let by_name: Vec<&Limitation> = a
        .assurance
        .limitations
        .iter()
        .filter(|l| l.code == codes::MCID_PROPERTY_LIST_BY_NAME)
        .collect();
    assert_eq!(by_name.len(), 1, "one named property list, declared once");
    assert_eq!(
        *by_name[0],
        ethos_parser_pdf::limitations::mcid_property_list_by_name(1),
        "declared exactly as any named list is today: the frame is the document's, not the tag's"
    );
}

/// **An author's tag still reads back as the author's** (acceptance 4).
///
/// The other half of the clause: every `pdf_tagged` locator on a document an author tagged says
/// `extracted`, and no engine-written declaration appears — on the structural-locator golden and
/// on a tagged table, whose record now carries the class it was read with.
#[test]
fn an_authors_tag_still_reads_back_as_extracted() {
    for name in [
        "tagged-structure-roles",
        "tagged-table-agrees",
        "tagged-list-items",
    ] {
        let a = extract_bytes(&engine_fixture(name)).expect("extracts");
        let mut bound = 0;
        for run in a.runs() {
            if let Some(StructuralLocator::PdfTagged(t)) = &run.structural {
                bound += 1;
                assert_eq!(
                    t.derivation,
                    DerivationClass::Extracted,
                    "{name}: an author's tree binds as the author's: {t:?}"
                );
            }
        }
        assert!(bound > 0, "{name}: the fixture binds at least one run");
        for page in &a.pages {
            for table in &page.tagged_tables {
                assert_eq!(table.derivation, DerivationClass::Extracted, "{name}");
            }
        }
        assert!(
            !codes_of(&a.assurance.limitations).contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
            "{name}: nothing here is this engine's: {:?}",
            codes_of(&a.assurance.limitations)
        );
    }
}

/// **An owned object in a shape the writer does not emit is refused** (acceptance 5).
///
/// `/Derivation /Computed` is required under `/O /EthosParser`. Removing it from one element,
/// or setting it to `/Extracted`, is `Malformed` naming the element — not read as the author's,
/// not skipped. The unedited re-serialisation is read first so the refusal is provably the edit's.
#[test]
fn the_owner_without_its_derivation_is_refused() {
    let control = edited(|_| {});
    assert_eq!(
        read(&control),
        read(&engine_fixture("engine-tagged-blocks")),
        "the control: re-serialising through lopdf changes nothing"
    );

    let without = edited(|doc| {
        let div = struct_elems(doc)[1];
        let mut a = take_attribute(elem_mut(doc, div));
        assert!(a.remove(b"Derivation").is_some());
        elem_mut(doc, div).set("A", Object::Dictionary(a));
    });
    let e = extract_bytes(&without).expect_err("an owned object with no /Derivation is refused");
    assert_eq!(e.code(), "malformed", "{e}");
    let msg = e.to_string();
    // The refusal names the OBJECT, not only the role: both `/Div` elements share the role, and
    // in the writer's shape every element below the root does, so the role alone points at
    // nothing. `struct_elems(doc)[1]` is object 8.
    for needle in [
        "structure element",
        "8 0 R",
        "`/Div`",
        "absent",
        "/EthosParser",
    ] {
        assert!(msg.contains(needle), "the refusal names {needle:?}: {msg}");
    }

    let extracted = edited(|doc| {
        let document = struct_elems(doc)[0];
        let mut a = take_attribute(elem_mut(doc, document));
        a.set("Derivation", Object::Name(b"Extracted".to_vec()));
        elem_mut(doc, document).set("A", Object::Dictionary(a));
    });
    let e = extract_bytes(&extracted).expect_err(
        "`/Derivation /Extracted` under this engine's owner is a shape it never writes",
    );
    assert_eq!(e.code(), "malformed", "{e}");
    let msg = e.to_string();
    for needle in ["structure element", "7 0 R", "`/Document`", "`/Extracted`"] {
        assert!(msg.contains(needle), "the refusal names {needle:?}: {msg}");
    }
}

/// **Every owned object in the union is checked, and `/A` decides** (scope §4.1).
///
/// One predicate applies over the union of `/A` and the classes reached through `/C`, so a class
/// this engine owns in a shape it does not write refuses the document even beside a well-formed
/// `/A` — not a short-circuit on the first owned object found. And when both are well-formed,
/// `/A` decides: the declaration names its `/Rule` and not the class's. The writer never emits
/// `/C`, so this holds a hand-made shape, as `an_attribute_under_another_owner_is_an_authors`
/// does.
#[test]
fn a_malformed_class_is_refused_beside_a_well_formed_a() {
    let with_class = |derivation: &'static [u8]| {
        edited(move |doc| {
            let mut map = Dictionary::new();
            map.set(
                "X",
                Object::Dictionary(owner_attribute(derivation, "other-rule-v9")),
            );
            struct_root_mut(doc).set("ClassMap", Object::Dictionary(map));
            let div = struct_elems(doc)[1];
            elem_mut(doc, div).set("C", Object::Name(b"X".to_vec()));
        })
    };

    let e = extract_bytes(&with_class(b"Extracted")).expect_err(
        "an owned class in a shape the writer never emits is refused beside a valid /A",
    );
    assert_eq!(e.code(), "malformed", "{e}");
    let msg = e.to_string();
    for needle in [
        "structure element",
        "8 0 R",
        "`/Div`",
        "`/Extracted`",
        "/EthosParser",
    ] {
        assert!(msg.contains(needle), "the refusal names {needle:?}: {msg}");
    }

    let a = extract_bytes(&with_class(b"Computed")).expect("two well-formed owned objects extract");
    assert_engine_tagged("a well-formed class beside /A", &a, [0, 0, 0, 1, 1, 1]);
    let written = engine_written(&a.assurance.limitations);
    assert!(
        !written.detail.contains("other-rule-v9"),
        "`/A` decides the rule set, so the class's `/Rule` is not declared beside it: {}",
        written.detail
    );
}

/// **An owned `/Table` reads back `Computed` under `tagged-tables-v1`** (scope §4.1 on §3.2).
///
/// The writer never emits a `/Table`, and nothing forbids a hand or a later writer from putting
/// the owner attribute on one, so the shape is held rather than left reachable by accident: the
/// tagged-table record and the representation's `TableRecord` carry the class the element
/// states, `detection_rule` still says `tagged-tables-v1`, and the geometry stays typed-absent.
/// `detection_rule`, not `derivation`, is what separates a tagged table from a geometric one;
/// `derivation` says whose statement the grid is. The cells are laid in one column with no
/// ruling in both halves so the tree's `/Table` reaches the wire at all (see
/// `lay_the_cells_in_one_column`), and the owner attribute is the only difference between them.
#[test]
fn an_owned_table_element_reads_back_as_computed_under_the_tagged_rule() {
    let tagged_records = |bytes: &[u8]| {
        let a = extract_bytes(bytes).expect("extracts");
        let on_extract: Vec<DerivationClass> = a
            .pages
            .iter()
            .flat_map(|p| p.tagged_tables.iter())
            .map(|t| {
                assert_eq!(t.rule, TABLE_DETECTION_TAGGED_V1);
                t.derivation
            })
            .collect();
        let repr = ethos_parser_pdf::to_representation(&a, &Profile::default()).expect("projects");
        let tables = &repr.payload().tables;
        assert!(
            tables
                .iter()
                .all(|t| t.detection_rule == TABLE_DETECTION_TAGGED_V1),
            "no detector may find a table on the repainted page, or the tree's /Table pairs with \
             it and never reaches the wire as its own record: {tables:?}"
        );
        let on_wire: Vec<ethos_parser_core::TableRecord> = tables.clone();
        (a, on_extract, on_wire)
    };

    let authors = edited_from("tagged-table-agrees", lay_the_cells_in_one_column);
    let (a, on_extract, on_wire) = tagged_records(&authors);
    assert_eq!(on_extract, vec![DerivationClass::Extracted]);
    assert_eq!(
        on_wire.len(),
        1,
        "one tagged table, once the grid is not painted"
    );
    assert_eq!(on_wire[0].derivation, DerivationClass::Extracted);
    assert!(
        !codes_of(&a.assurance.limitations).contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
        "an author's table declares nothing engine-written: {:?}",
        codes_of(&a.assurance.limitations)
    );

    let owned = edited_from("tagged-table-agrees", |doc| {
        lay_the_cells_in_one_column(doc);
        let table = elem_with_role(doc, b"Table");
        elem_mut(doc, table).set(
            "A",
            Object::Dictionary(owner_attribute(b"Computed", "gutter-columns-v3")),
        );
    });
    let (a, on_extract, on_wire) = tagged_records(&owned);
    assert_eq!(
        on_extract,
        vec![DerivationClass::Computed],
        "the record carries the class its element states"
    );
    assert_eq!(on_wire.len(), 1);
    assert_eq!(
        on_wire[0].derivation,
        DerivationClass::Computed,
        "never restored to the constant on the way to the wire: {:?}",
        on_wire[0]
    );
    assert_eq!(on_wire[0].detection_rule, TABLE_DETECTION_TAGGED_V1);
    assert!(
        matches!(
            on_wire[0].geometry,
            GeometryPresence::Absent(GeometryAbsence::NotReportedByStructureTree)
        ),
        "no box is invented for an owned tagged table either: {:?}",
        on_wire[0].geometry
    );
    // Only the `/Table` carries the owner, so the tree is declared mixed and the declaration
    // counts one element — and no run: the cells' runs bind under the `/TD` elements, which are
    // the author's, so `computed` runs are none even though the table is this engine's.
    let written = engine_written(&a.assurance.limitations);
    for needle in ["1 of its", "MIXES", "gutter-columns-v3", "0 text run(s)"] {
        assert!(
            written.detail.contains(needle),
            "the declaration must say {needle:?}: {}",
            written.detail
        );
    }
}

/// **Rename the owner and the tag is the author's** (acceptance 6).
///
/// This documents a FAILURE MODE, not a feature. Decision #23 binds the auto-tagging row to the
/// attribute: *if the attribute is ever dropped or ignored, that distinction is gone and this row
/// goes with it.* Here it is, one edit away — every run reads back `extracted`, nothing is
/// declared, and the tree is indistinguishable from an author's. Held as a test so that the day
/// it stops being reproducible is noticed rather than assumed.
#[test]
fn an_attribute_under_another_owner_is_an_authors() {
    let renamed = edited(|doc| {
        for id in struct_elems(doc) {
            let mut a = take_attribute(elem_mut(doc, id));
            a.set("O", Object::Name(b"SomeoneElse".to_vec()));
            elem_mut(doc, id).set("A", Object::Dictionary(a));
        }
    });
    let a = extract_bytes(&renamed).expect("somebody else's attribute is not a refusal");
    let runs: Vec<_> = a.runs().collect();
    assert_eq!(runs.len(), 6);
    for run in &runs {
        let t = tagged(&run.structural);
        assert_eq!(t.role_path, vec!["Document", "Div"]);
        assert_eq!(
            t.derivation,
            DerivationClass::Extracted,
            "under another owner the row goes with it: the tag reads as the author's"
        );
    }
    let codes = codes_of(&a.assurance.limitations);
    assert!(
        !codes.contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
        "and nothing says otherwise: {codes:?}"
    );
    assert!(
        !codes.contains(&codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
        "a tree was read, whoever's it is: {codes:?}"
    );
}

/// **The attribute is read behind an indirect reference, in every shape §14.7.5.2 allows**
/// (acceptance 7).
///
/// `/A 12 0 R` to the dictionary; `/A [<<…>> 0]` with a revision number; and `/A 12 0 R` to an
/// array that holds a reference to the dictionary and a revision number. Each reads identically
/// to the inline form.
#[test]
fn an_attribute_behind_a_reference_is_read() {
    let inline = read(&engine_fixture("engine-tagged-blocks"));

    let referenced = edited(|doc| {
        for id in struct_elems(doc) {
            let a = take_attribute(elem_mut(doc, id));
            let a_id = doc.add_object(Object::Dictionary(a));
            elem_mut(doc, id).set("A", Object::Reference(a_id));
        }
    });
    assert_eq!(read(&referenced), inline, "`/A n 0 R` to the dictionary");

    let with_revision = edited(|doc| {
        for id in struct_elems(doc) {
            let a = take_attribute(elem_mut(doc, id));
            elem_mut(doc, id).set(
                "A",
                Object::Array(vec![Object::Dictionary(a), Object::Integer(0)]),
            );
        }
    });
    assert_eq!(
        read(&with_revision),
        inline,
        "`/A [<<…>> 0]` with a revision number"
    );

    let referenced_array = edited(|doc| {
        for id in struct_elems(doc) {
            let a = take_attribute(elem_mut(doc, id));
            let a_id = doc.add_object(Object::Dictionary(a));
            let array_id = doc.add_object(Object::Array(vec![
                Object::Reference(a_id),
                Object::Integer(0),
            ]));
            elem_mut(doc, id).set("A", Object::Reference(array_id));
        }
    });
    assert_eq!(
        read(&referenced_array),
        inline,
        "`/A n 0 R` to an array holding a reference and a revision number"
    );
}

/// **`/MarkInfo` is never read** (acceptance 8).
///
/// `/Marked true` is the Tagged PDF conformance claim; the writer sets none and the reader
/// consults none. With no `/MarkInfo`, with `/Marked true` and with `/Marked false` the document
/// binds and declares identically.
#[test]
fn mark_info_is_never_read() {
    let plain = read(&engine_fixture("engine-tagged-blocks"));
    for marked in [true, false] {
        let with_mark_info = edited(|doc| {
            let mut info = Dictionary::new();
            info.set("Marked", Object::Boolean(marked));
            catalog_mut(doc).set("MarkInfo", Object::Dictionary(info));
        });
        assert_eq!(
            read(&with_mark_info),
            plain,
            "`/MarkInfo << /Marked {marked} >>` must change nothing"
        );
    }
}
