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

//! Auto-tagging S2: the writer, through the public surface (`docs/24-AUTO-TAGGING-MILESTONES.md`
//! §S2 acceptance; `docs/23-AUTO-TAGGING-SCOPE.md` §3).
//!
//! `write_tags` is tested against the reader S1 shipped and the fixtures a human wrote in the
//! writer's shape, never against itself: the tree it emits is walked with `lopdf` and compared to
//! `engine-tagged-blocks`, and the bindings it produces are read back by `extract`. The refusals
//! are made from the fixtures that carry each shape. The self-check is exercised on mutated bytes
//! in `tagging.rs`'s own tests, because it is crate-private.
//!
//! Fixtures resolve through `fixtures/manifest.json`'s roots. **A missing fixture is a failure,
//! never a skip.**

use std::collections::BTreeMap;
use std::path::PathBuf;

use ethos_parser_core::{codes, DerivationClass, EngineError, Profile, StructuralLocator};
use ethos_parser_pdf::{write_tags, Document, ExtractArtifact, TAGS_ARTIFACT_TYPE};
use lopdf::content::Operation;
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

fn corpus_root(name: &str) -> PathBuf {
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest readable"),
    )
    .expect("manifest is valid JSON");
    let decl = &manifest["roots"][name];
    match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    }
}

fn fixture(root: &str, rel: &str) -> Vec<u8> {
    let p = corpus_root(root).join(rel);
    std::fs::read(&p).unwrap_or_else(|e| {
        panic!(
            "fixture `{rel}` missing at {}: {e}. A missing fixture is a failure, never a skip.",
            p.display()
        )
    })
}

fn engine_fixture(name: &str) -> Vec<u8> {
    fixture("engine", &format!("{name}/document.pdf"))
}

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

fn open(bytes: &[u8]) -> Document {
    Document::open_bytes(bytes, &Profile::default()).expect("the document opens")
}

fn tag(bytes: &[u8]) -> Result<Vec<u8>, EngineError> {
    write_tags(&open(bytes), &Profile::default())
}

fn tagged(name: &str) -> Vec<u8> {
    tag(&engine_fixture(name)).unwrap_or_else(|e| panic!("{name} tags cleanly: {e}"))
}

fn extract(bytes: &[u8]) -> ExtractArtifact {
    ethos_parser_pdf::extract(&open(bytes), &Profile::default()).expect("extracts")
}

/// What the reader says about a document: every run's text and address, and the limitations.
#[derive(Debug, PartialEq, Eq)]
struct Read {
    bindings: Vec<(String, Option<StructuralLocator>, Option<i64>)>,
    limitations: Vec<String>,
}

fn read(bytes: &[u8]) -> Read {
    let a = extract(bytes);
    Read {
        bindings: a
            .runs()
            .map(|r| (r.text.clone(), r.structural.clone(), r.mcid))
            .collect(),
        limitations: a
            .assurance
            .limitations
            .iter()
            .map(|l| format!("{}: {}", l.code, l.detail))
            .collect(),
    }
}

fn codes_of(a: &ExtractArtifact) -> Vec<&str> {
    a.assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect()
}

/// One run's text record: page, text, origin, advance, font size, codes.
type RunRecord = (u32, String, i64, i64, Option<i64>, i64, Vec<u32>);

/// The text record of an extract: what the round trip must keep, run for run.
fn text_record(a: &ExtractArtifact) -> Vec<RunRecord> {
    a.pages
        .iter()
        .flat_map(|p| {
            p.runs.iter().map(move |r| {
                (
                    p.index,
                    r.text.clone(),
                    r.locator.origin_x,
                    r.locator.origin_y,
                    r.locator.advance,
                    r.font_size,
                    r.char_codes.clone(),
                )
            })
        })
        .collect()
}

fn catalog(doc: &lopdf::Document) -> Dictionary {
    doc.catalog().expect("a catalog").clone()
}

fn resolved(doc: &lopdf::Document, o: &Object) -> Object {
    doc.dereference(o).expect("resolves").1.clone()
}

fn dict(doc: &lopdf::Document, o: &Object) -> Dictionary {
    resolved(doc, o).as_dict().expect("a dictionary").clone()
}

fn name(o: &Object) -> String {
    String::from_utf8_lossy(o.as_name().expect("a name")).into_owned()
}

/// One structure element as a test compares it: its role, its attribute (sorted, values
/// rendered by `Debug`, the `/Rule` string by its bytes), whether it names a page, and the
/// marked-content ids its `/K` cites in order. Object numbers are left out on purpose: the
/// writer's and the hand-written fixture's differ and mean nothing.
#[derive(Debug, PartialEq, Eq)]
struct Element {
    role: String,
    attribute: Vec<(String, String)>,
    has_page: bool,
    ids: Vec<i64>,
    kids: Vec<Element>,
}

fn element(doc: &lopdf::Document, o: &Object) -> Element {
    let d = dict(doc, o);
    let attribute = match d.get(b"A") {
        Ok(a) => {
            let mut pairs: Vec<(String, String)> = dict(doc, a)
                .iter()
                .map(|(k, v)| {
                    let key = String::from_utf8_lossy(k).into_owned();
                    let value = match v {
                        Object::String(bytes, _) => {
                            format!("({})", String::from_utf8_lossy(bytes))
                        }
                        other => format!("{other:?}"),
                    };
                    (key, value)
                })
                .collect();
            pairs.sort();
            pairs
        }
        Err(_) => Vec::new(),
    };
    let mut ids = Vec::new();
    let mut kids = Vec::new();
    let mut take = |item: &Object| match resolved(doc, item) {
        Object::Integer(id) => ids.push(id),
        Object::Dictionary(_) => kids.push(element(doc, item)),
        other => panic!("a /K item that is neither an id nor an element: {other:?}"),
    };
    match d.get(b"K") {
        Ok(Object::Array(items)) => items.iter().for_each(&mut take),
        Ok(single) => take(single),
        Err(_) => {}
    }
    Element {
        role: name(d.get(b"S").expect("an element has /S")),
        attribute,
        has_page: d.get(b"Pg").is_ok(),
        ids,
        kids,
    }
}

/// The tree as a test compares it: the root's elements, and the parent tree as
/// `key -> [element role and its position among the /Document's kids, per id]`.
#[derive(Debug, PartialEq, Eq)]
struct Tree {
    elements: Vec<Element>,
    parent_tree: Vec<(i64, Vec<(String, usize)>)>,
    next_key: i64,
}

fn tree(bytes: &[u8]) -> Tree {
    let doc = lopdf::Document::load_mem(bytes).expect("lopdf loads");
    let root = dict(&doc, catalog(&doc).get(b"StructTreeRoot").expect("a tree"));
    assert_eq!(name(root.get(b"Type").unwrap()), "StructTreeRoot");
    let root_kids: Vec<Object> = match root.get(b"K").expect("the root has kids") {
        Object::Array(items) => items.clone(),
        single => vec![single.clone()],
    };
    let elements: Vec<Element> = root_kids.iter().map(|k| element(&doc, k)).collect();
    // The document element's kids by object id, so a parent-tree entry can be named by position.
    let document = dict(&doc, &root_kids[0]);
    let div_ids: Vec<ObjectId> = match document.get(b"K").unwrap() {
        Object::Array(items) => items
            .iter()
            .map(|i| i.as_reference().expect("a /Div is an indirect object"))
            .collect(),
        single => vec![single.as_reference().unwrap()],
    };
    let parent_tree = dict(&doc, root.get(b"ParentTree").expect("a parent tree"));
    let nums = parent_tree.get(b"Nums").unwrap().as_array().unwrap();
    let mut entries = Vec::new();
    for pair in nums.chunks(2) {
        let key = pair[0].as_i64().unwrap();
        let refs: Vec<(String, usize)> = pair[1]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let id = r.as_reference().unwrap();
                let position = div_ids.iter().position(|d| *d == id).expect("a /Div");
                (name(dict(&doc, r).get(b"S").unwrap()), position)
            })
            .collect();
        entries.push((key, refs));
    }
    Tree {
        elements,
        parent_tree: entries,
        next_key: root.get(b"ParentTreeNextKey").unwrap().as_i64().unwrap(),
    }
}

/// The output page's operations, decoded by lopdf.
fn page_ops(bytes: &[u8], page: u32) -> Vec<Operation> {
    let doc = lopdf::Document::load_mem(bytes).expect("lopdf loads");
    let id = doc.get_pages()[&page];
    lopdf::content::Content::decode(&doc.get_page_content(id))
        .expect("decodes")
        .operations
}

fn page_tokens(bytes: &[u8], page: u32) -> Vec<String> {
    let doc = lopdf::Document::load_mem(bytes).expect("lopdf loads");
    let id = doc.get_pages()[&page];
    String::from_utf8_lossy(&doc.get_page_content(id))
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// A written sequence read back out of the operations: its id, the operations it encloses, and
/// the tag of the frame it opened inside, if any.
#[derive(Debug)]
struct Written {
    mcid: i64,
    first: usize,
    last: usize,
    inside: Option<String>,
}

fn written_sequences(ops: &[Operation]) -> Vec<Written> {
    let mut stack: Vec<(String, Option<(i64, usize)>)> = Vec::new();
    let mut out = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match op.operator.as_str() {
            "BDC" | "BMC" => {
                let tag = op
                    .operands
                    .first()
                    .and_then(|o| o.as_name().ok())
                    .map(|n| String::from_utf8_lossy(n).into_owned())
                    .unwrap_or_default();
                let written = match op.operands.get(1) {
                    Some(Object::Dictionary(d)) if tag == "Div" => d
                        .get(b"MCID")
                        .ok()
                        .and_then(|m| m.as_i64().ok())
                        .map(|m| (m, i)),
                    _ => None,
                };
                stack.push((tag, written));
            }
            "EMC" => {
                let (_, written) = stack.pop().expect("frames are balanced");
                if let Some((mcid, opened)) = written {
                    out.push(Written {
                        mcid,
                        first: opened + 1,
                        last: i - 1,
                        inside: stack.last().map(|(tag, _)| tag.clone()),
                    });
                }
            }
            _ => {}
        }
    }
    out.sort_by_key(|w| w.first);
    out
}

fn operator_at(ops: &[Operation], operator: &str, tag: Option<&str>) -> usize {
    ops.iter()
        .position(|o| {
            o.operator == operator
                && tag.is_none_or(|t| {
                    o.operands.first() == Some(&Object::Name(t.as_bytes().to_vec()))
                })
        })
        .unwrap_or_else(|| panic!("no `{operator}` {tag:?}"))
}

fn mcids(a: &ExtractArtifact) -> Vec<(u32, String, Option<i64>)> {
    a.pages
        .iter()
        .flat_map(|p| {
            p.runs
                .iter()
                .map(move |r| (p.index, r.text.clone(), r.mcid))
        })
        .collect()
}

fn declaration<'a>(a: &'a ExtractArtifact, code: &str) -> &'a ethos_parser_core::Limitation {
    a.assurance
        .limitations
        .iter()
        .find(|l| l.code == code)
        .unwrap_or_else(|| panic!("`{code}` is declared"))
}

// -------------------------------------------------------------------------------------------
// The tests
// -------------------------------------------------------------------------------------------

/// **The writer emits the reader's fixture shape** (acceptance 1).
///
/// `write_tags` on `leading-gap-two-blocks` produces a tree whose elements, roles, attributes
/// and `/K` ids, and whose parent tree, are `engine-tagged-blocks`'s, and whose runs read back
/// with the same bindings and the same declarations — a file a human wrote before the writer
/// existed, so a mistake the two might share cannot pass.
#[test]
fn the_writer_emits_the_readers_fixture_shape() {
    let written = tagged("leading-gap-two-blocks");
    let by_hand = engine_fixture("engine-tagged-blocks");

    assert_eq!(
        tree(&written),
        tree(&by_hand),
        "the structure tree, element by element"
    );
    let expected_tree = Tree {
        elements: vec![Element {
            role: "Document".into(),
            attribute: vec![
                ("Derivation".into(), "/Computed".into()),
                ("O".into(), "/EthosParser".into()),
                ("Rule".into(), "(gutter-columns-v3)".into()),
            ],
            has_page: false,
            ids: vec![],
            kids: vec![
                Element {
                    role: "Div".into(),
                    attribute: vec![
                        ("Derivation".into(), "/Computed".into()),
                        ("O".into(), "/EthosParser".into()),
                        ("Rule".into(), "(gutter-columns-v3)".into()),
                    ],
                    has_page: true,
                    ids: vec![0],
                    kids: vec![],
                },
                Element {
                    role: "Div".into(),
                    attribute: vec![
                        ("Derivation".into(), "/Computed".into()),
                        ("O".into(), "/EthosParser".into()),
                        ("Rule".into(), "(gutter-columns-v3)".into()),
                    ],
                    has_page: true,
                    ids: vec![1],
                    kids: vec![],
                },
            ],
        }],
        parent_tree: vec![(0, vec![("Div".into(), 0), ("Div".into(), 1)])],
        next_key: 1,
    };
    assert_eq!(tree(&written), expected_tree, "and the shape spelled out");

    assert_eq!(
        read(&written),
        read(&by_hand),
        "the reader binds and declares both alike"
    );
    let a = extract(&written);
    for (i, run) in a.runs().enumerate() {
        match &run.structural {
            Some(StructuralLocator::PdfTagged(t)) => {
                assert_eq!(t.derivation, DerivationClass::Computed, "run {i}");
                assert_eq!(t.role_path, ["Document", "Div"], "run {i}");
                assert_eq!(t.mcid, if i < 3 { 0 } else { 1 }, "run {i}");
            }
            other => panic!("run {i} is not bound: {other:?}"),
        }
    }
    let written_doc = lopdf::Document::load_mem(&written).unwrap();
    let page = written_doc.get_pages()[&1];
    assert_eq!(
        written_doc
            .get_dictionary(page)
            .unwrap()
            .get(b"StructParents")
            .ok(),
        Some(&Object::Integer(0))
    );
}

/// **The content stream is the source's with the sequences inserted and nothing else changed**
/// (scope §3.5). Removing the inserted token groups from the tagged page gives the untagged
/// page's bytes back; the page has exactly one `FlateDecode` stream; the superseded stream is
/// gone from the object map.
#[test]
fn the_page_is_one_flate_stream_holding_the_source_bytes_plus_the_inserted_tokens() {
    let source = engine_fixture("leading-gap-two-blocks");
    let written = tagged("leading-gap-two-blocks");
    let before = lopdf::Document::load_mem(&source).unwrap();
    let after = lopdf::Document::load_mem(&written).unwrap();
    let page_before = before.get_pages()[&1];
    let page_after = after.get_pages()[&1];

    let old_streams = before.get_page_contents(page_before);
    let new_streams = after.get_page_contents(page_after);
    assert_eq!(old_streams.len(), 1);
    assert_eq!(new_streams.len(), 1, "one stream per rewritten page");
    assert_ne!(
        old_streams[0], new_streams[0],
        "a new object, never the old one edited"
    );
    assert!(
        after.get_object(old_streams[0]).is_err(),
        "the superseded stream is removed from the object map"
    );
    let stream = after
        .get_object(new_streams[0])
        .unwrap()
        .as_stream()
        .unwrap();
    assert_eq!(
        stream.dict.get(b"Filter").unwrap(),
        &Object::Name(b"FlateDecode".to_vec())
    );
    assert_eq!(
        stream.dict.get(b"Length").unwrap().as_i64().unwrap() as usize,
        stream.content.len(),
        "/Length is the stream's own length"
    );

    let original = String::from_utf8(before.get_page_content(page_before)).unwrap();
    let spliced = String::from_utf8(after.get_page_content(page_after)).unwrap();
    assert_ne!(original, spliced);
    let stripped = spliced
        .replace("/Div <</MCID 0>> BDC\n", "")
        .replace("/Div <</MCID 1>> BDC\n", "")
        .replace("\nEMC", "");
    // The one byte beside the inserted tokens: the `\n` `get_page_content` puts after every
    // stream, which the writer reads as part of the page's buffer and therefore emits inside the
    // single stream the page ends up with (scope §3.5 says so).
    assert_eq!(
        stripped,
        format!("{original}\n"),
        "every source byte survives in order; only the inserted tokens and the separator differ"
    );
    assert_eq!(
        spliced,
        "BT /F1 12 Tf /Div <</MCID 0>> BDC\n1 0 0 1 72 700 Tm (Water finds its level) Tj 1 0 0 \
         1 72 686 Tm (and stone keeps its shape) Tj 1 0 0 1 72 672 Tm (through the long season) \
         Tj\nEMC /Div <</MCID 1>> BDC\n1 0 0 1 72 644 Tm (Wind moves the grass) Tj 1 0 0 1 72 \
         630 Tm (and light moves the shade) Tj 1 0 0 1 72 616 Tm (across the open field) \
         Tj\nEMC ET\n\n",
        "each sequence opens before its block's first Tm and closes after its last Tj; the \
         trailing newlines are the one the source stream ended with and the separator \
         `get_page_content` adds"
    );
}

/// **The catalog stamp says what the tags were computed from** (scope §3.3), and `/MarkInfo`
/// is never written — one already there is left exactly as found.
#[test]
fn the_stamp_names_the_source_and_the_profile_and_no_mark_info_is_written() {
    let source = engine_fixture("leading-gap-two-blocks");
    let doc = open(&source);
    let profile = Profile::default();
    let written = write_tags(&doc, &profile).expect("tags");
    let out = lopdf::Document::load_mem(&written).unwrap();
    let cat = catalog(&out);
    let stamp = dict(&out, cat.get(b"EthosParserTags").expect("the stamp"));
    let text = |key: &[u8]| {
        String::from_utf8_lossy(stamp.get(key).unwrap().as_str().unwrap()).into_owned()
    };
    assert_eq!(text(b"ArtifactType"), TAGS_ARTIFACT_TYPE);
    assert_eq!(TAGS_ARTIFACT_TYPE, "ethos.parser.tags.v0");
    assert_eq!(text(b"SourceSha256"), doc.source_sha256().as_str());
    assert_eq!(
        text(b"ProfileSha256"),
        profile.profile_sha256().unwrap().as_str()
    );
    assert_eq!(text(b"ParserVersion"), profile.parser_version);
    assert!(cat.get(b"MarkInfo").is_err(), "no /MarkInfo is written");

    // A /MarkInfo the source already carries survives byte for byte in meaning.
    let mut with_mark_info = lopdf::Document::load_mem(&source).unwrap();
    let mut info = Dictionary::new();
    info.set("Marked", Object::Boolean(false));
    with_mark_info
        .catalog_mut()
        .unwrap()
        .set("MarkInfo", Object::Dictionary(info.clone()));
    let mut bytes = Vec::new();
    with_mark_info.save_to(&mut bytes).unwrap();
    let written = tag(&bytes).expect("tags");
    let out = lopdf::Document::load_mem(&written).unwrap();
    assert_eq!(
        catalog(&out).get(b"MarkInfo").ok(),
        Some(&Object::Dictionary(info)),
        "left as found"
    );
}

/// **Two writes agree byte for byte** (acceptance 3): on one handle twice, on two handles, on
/// the two-column fixture, and on a real producer's untagged document from the oracle corpus.
#[test]
fn a_tagged_document_is_byte_identical_across_runs() {
    for (label, bytes) in [
        (
            "leading-gap-two-blocks",
            engine_fixture("leading-gap-two-blocks"),
        ),
        ("two-column-15-lines", engine_fixture("two-column-15-lines")),
        (
            "foreign/opendataloader/real",
            fixture("conformance", "foreign/opendataloader/real/source.pdf"),
        ),
    ] {
        let doc = open(&bytes);
        let first =
            write_tags(&doc, &Profile::default()).unwrap_or_else(|e| panic!("{label}: {e}"));
        let second = write_tags(&doc, &Profile::default()).unwrap();
        assert!(first == second, "{label}: the same handle, twice");
        let third = tag(&bytes).unwrap();
        assert!(first == third, "{label}: a fresh handle");
        assert!(first.starts_with(b"%PDF"), "{label}: a PDF");
    }
}

/// **A tagged document is refused** (acceptance 4), with the fills-absence reason, before any
/// byte is written — and so is the writer's own output (S3's `a_second_write_is_refused`,
/// held here because the writer is where it belongs).
#[test]
fn a_tagged_document_is_refused() {
    for name in [
        "tagged-structure-roles",
        "engine-tagged-blocks",
        "tagged-table-agrees",
    ] {
        let e = tag(&engine_fixture(name)).expect_err("a tree is present");
        assert_eq!(e.code(), "unsupported", "{name}: {e}");
        assert!(
            e.to_string()
                .starts_with("unsupported tagging: the catalog declares /StructTreeRoot: a written tag fills absence only"),
            "{name}: {e}"
        );
    }
}

#[test]
fn a_second_write_is_refused() {
    let once = tagged("leading-gap-two-blocks");
    let e = tag(&once).expect_err("the output carries a tree");
    assert_eq!(e.code(), "unsupported");
    assert!(e.to_string().contains("fills absence only"), "{e}");
}

/// **The round trip on the four untagged fixtures S3's first test names**: tag, extract the
/// tagged bytes, every run binds `pdf_tagged` with `derivation: computed`, the text record is
/// the source's run for run, and the artifact declares the engine-written tree and not an
/// untagged document.
#[test]
fn the_round_trip_keeps_the_text_record_and_binds_every_run_computed() {
    for name in [
        "leading-gap-two-blocks",
        "two-column-15-lines",
        "untagged-shredded-line",
        "markdown-two-blocks",
    ] {
        let source = engine_fixture(name);
        let before = extract(&source);
        let written = tag(&source).unwrap_or_else(|e| panic!("{name}: {e}"));
        let after = extract(&written);
        assert_eq!(
            text_record(&after),
            text_record(&before),
            "{name}: the text record is unchanged"
        );
        // And every other field of every page record — box, rotation, images, tables, tagged
        // tables, objects — so the round trip is checked on the whole page, not the runs alone.
        assert_eq!(after.pages.len(), before.pages.len(), "{name}");
        for (was, now) in before.pages.iter().zip(&after.pages) {
            let mut was = was.clone();
            let mut now = now.clone();
            was.runs.clear();
            now.runs.clear();
            assert_eq!(now, was, "{name} page {}: every field but runs", was.index);
        }
        assert!(after.runs().count() > 0, "{name}: the fixture shows text");
        let mut ids: BTreeMap<u32, Vec<i64>> = BTreeMap::new();
        for page in &after.pages {
            for run in &page.runs {
                match &run.structural {
                    Some(StructuralLocator::PdfTagged(t)) => {
                        assert_eq!(t.derivation, DerivationClass::Computed, "{name}");
                        assert_eq!(t.role_path, ["Document", "Div"], "{name}");
                        assert_eq!(run.mcid, Some(t.mcid), "{name}");
                        assert_eq!(
                            run.derivation,
                            DerivationClass::Extracted,
                            "{name}: the text was read"
                        );
                        ids.entry(page.index).or_default().push(t.mcid);
                    }
                    other => panic!("{name}: `{}` is not bound: {other:?}", run.text),
                }
            }
        }
        for (page, mut page_ids) in ids {
            page_ids.sort_unstable();
            page_ids.dedup();
            assert_eq!(
                page_ids,
                (0..page_ids.len() as i64).collect::<Vec<_>>(),
                "{name} page {page}: ids are dense from 0"
            );
        }
        let codes = codes_of(&after);
        assert!(
            codes.contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
            "{name}: {codes:?}"
        );
        for absent in [
            codes::UNTAGGED_STRUCTURE_TREE_ABSENT,
            codes::STRUCTURE_MCID_UNBOUND,
            codes::STRUCTURE_ITEM_WITHOUT_CONTENT,
        ] {
            assert!(!codes.contains(&absent), "{name}: {codes:?}");
        }
        assert!(
            codes_of(&before).contains(&codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
            "{name}: the source was untagged"
        );
        // Every other declaration is the source's.
        let without = |a: &ExtractArtifact| -> Vec<String> {
            a.assurance
                .limitations
                .iter()
                .filter(|l| {
                    l.code != codes::UNTAGGED_STRUCTURE_TREE_ABSENT
                        && l.code != codes::STRUCTURE_TREE_ENGINE_WRITTEN
                })
                .map(|l| format!("{}: {}", l.code, l.detail))
                .collect()
        };
        assert_eq!(without(&after), without(&before), "{name}");
    }
}

/// **The declaration names what the writer wrote**: on the two-column page, three elements
/// (the `/Document` and one `/Div` per band) and fifteen runs, and the `/Div` order is the
/// reading order — the left column's element first, holding id 1, because the right column
/// was written first and took id 0.
#[test]
fn the_two_column_page_gets_one_div_per_band_in_reading_order() {
    let written = tagged("two-column-15-lines");
    let t = tree(&written);
    assert_eq!(t.elements.len(), 1);
    let document = &t.elements[0];
    assert_eq!(document.kids.len(), 2);
    assert_eq!(
        document.kids[0].ids,
        [1],
        "the left column, read first, written second"
    );
    assert_eq!(document.kids[1].ids, [0], "the right column, written first");
    assert_eq!(
        t.parent_tree,
        vec![(0, vec![("Div".into(), 1), ("Div".into(), 0)])]
    );
    let a = extract(&written);
    let declaration = a
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == codes::STRUCTURE_TREE_ENGINE_WRITTEN)
        .expect("declared");
    for needle in [
        "3 of its 3 structure element(s)",
        "15 text run(s)",
        "gutter-columns-v3",
    ] {
        assert!(
            declaration.detail.contains(needle),
            "{}",
            declaration.detail
        );
    }
    // Reading order is what `extract` reports: the left column's runs first, each bound to
    // id 1 on the page.
    let runs: Vec<_> = a.runs().collect();
    assert_eq!(runs[0].text, "L1");
    assert_eq!(runs[0].mcid, Some(1));
    assert_eq!(runs[8].text, "R1");
    assert_eq!(runs[8].mcid, Some(0));
}

// -------------------------------------------------------------------------------------------
// The refusals and placements the writer's own fixtures hold (S2 acceptance)
// -------------------------------------------------------------------------------------------

/// **Ids in the content stream and no tree are refused, inline and by name** (scope §3.6, rows
/// two and three). The inline id is caught on the run the reader bound `pdf_mcid`; the named one
/// is caught by resolving the name the reader declares and does not read.
#[test]
fn ids_without_a_tree_are_refused() {
    let e = tag(&engine_fixture("untagged-mcid-no-tree")).expect_err("an inline id");
    assert_eq!(e.code(), "unsupported");
    assert!(
        e.to_string().starts_with(
            "unsupported tagging: page 1, run 1 `and stone keeps its shape` carries \
             marked-content id 0 and the document has no structure tree"
        ),
        "{e}"
    );
    let e = tag(&engine_fixture("untagged-mcid-by-name")).expect_err("an id by name");
    assert_eq!(e.code(), "unsupported");
    let msg = e.to_string();
    assert!(
        msg.starts_with("unsupported tagging: page 1, operation "),
        "{msg}"
    );
    assert!(
        msg.contains("`/P /MC0 BDC` names a property list that carries `/MCID`"),
        "{msg}"
    );
    // The reader itself read both pages without complaint: the refusal is the writer's.
    for name in ["untagged-mcid-no-tree", "untagged-mcid-by-name"] {
        assert_eq!(extract(&engine_fixture(name)).runs().count(), 6, "{name}");
    }
}

/// **A named list without an id is a frame, and the sequence sits inside it** (scope §3.4,
/// §3.6): on `untagged-oc-by-name` the writer tags cleanly, line 2's sequence opens as the next
/// operation after `/OC /oc1 BDC` and closes as the operation before the frame's `EMC`, and the
/// reader — binding by the innermost open sequence — binds line 2 computed under id 1, while the
/// named list is declared exactly as it is today.
#[test]
fn a_named_list_without_an_id_is_placed_inside() {
    let written = tagged("untagged-oc-by-name");
    let ops = page_ops(&written, 1);
    let seqs = written_sequences(&ops);
    assert_eq!(
        seqs.iter().map(|w| w.mcid).collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert_eq!(seqs[1].inside.as_deref(), Some("OC"), "{seqs:?}");
    for other in [0, 2, 3] {
        assert_eq!(seqs[other].inside, None, "{seqs:?}");
    }
    let frame = operator_at(&ops, "BDC", Some("OC"));
    assert_eq!(
        seqs[1].first,
        frame + 2,
        "the written BDC follows the frame's"
    );
    assert_eq!(ops[seqs[1].last + 1].operator, "EMC", "the written EMC");
    assert_eq!(
        ops[seqs[1].last + 2].operator,
        "EMC",
        "and the frame's after it"
    );

    let t = tree(&written);
    assert_eq!(t.elements[0].kids.len(), 2);
    assert_eq!(t.elements[0].kids[0].ids, [0, 1, 2]);
    assert_eq!(t.elements[0].kids[1].ids, [3]);

    let a = extract(&written);
    assert_eq!(
        mcids(&a),
        [
            (1, "Water finds its level".into(), Some(0)),
            (1, "and stone keeps its shape".into(), Some(1)),
            (1, "through the long season".into(), Some(2)),
            (1, "Wind moves the grass".into(), Some(3)),
            (1, "and light moves the shade".into(), Some(3)),
            (1, "across the open field".into(), Some(3)),
        ]
    );
    for run in a.runs() {
        match &run.structural {
            Some(StructuralLocator::PdfTagged(t)) => {
                assert_eq!(t.derivation, DerivationClass::Computed)
            }
            other => panic!("`{}` is not bound: {other:?}", run.text),
        }
    }
    assert_eq!(
        *declaration(&a, codes::MCID_PROPERTY_LIST_BY_NAME),
        ethos_parser_pdf::limitations::mcid_property_list_by_name(1),
        "the frame is the document's, declared as any named list is"
    );
}

/// **An artifact run stays outside the tree** (scope §3.4): on `untagged-artifact-furniture`
/// the head is its own block and furniture entirely, so it gets no element, still binds
/// `pdf_artifact`, and its frame is enclosed by no written sequence; the two body blocks come
/// out as on the leading-gap page.
#[test]
fn an_artifact_run_stays_outside_the_tree() {
    let written = tagged("untagged-artifact-furniture");
    let a = extract(&written);
    let runs: Vec<_> = a.runs().collect();
    assert_eq!(runs.len(), 7);
    assert_eq!(runs[0].text, "Running head");
    assert_eq!(
        runs[0].structural,
        Some(StructuralLocator::PdfArtifact(
            ethos_parser_core::PdfArtifactLocator { mcid: None }
        ))
    );
    assert_eq!(runs[0].mcid, None);
    assert_eq!(
        runs[0].block,
        Some(1),
        "the head is block 1 by the leading-gap cut"
    );
    for (i, run) in runs.iter().enumerate().skip(1) {
        assert_eq!(run.mcid, Some(if i <= 3 { 0 } else { 1 }), "`{}`", run.text);
        assert!(
            matches!(&run.structural, Some(StructuralLocator::PdfTagged(t)) if t.derivation == DerivationClass::Computed),
            "`{}`",
            run.text
        );
    }
    let t = tree(&written);
    assert_eq!(t.elements[0].kids.len(), 2, "no element for the head");
    assert_eq!(t.elements[0].kids[0].ids, [0]);
    assert_eq!(t.elements[0].kids[1].ids, [1]);
    let detail = &declaration(&a, codes::STRUCTURE_TREE_ENGINE_WRITTEN).detail;
    assert!(
        detail.contains("3 of its 3 structure element(s)"),
        "{detail}"
    );
    assert!(detail.contains("6 text run(s)"), "{detail}");

    let ops = page_ops(&written, 1);
    let seqs = written_sequences(&ops);
    let artifact = operator_at(&ops, "BMC", Some("Artifact"));
    assert!(
        seqs.iter().all(|w| artifact < w.first || w.last < artifact),
        "the artifact frame is enclosed in nothing: {seqs:?}"
    );
    assert_eq!(seqs.len(), 2);
}

/// **A shared content stream is never edited in place** (scope §3.5): the two pages of
/// `shared-content-stream` get two distinct new streams whose bytes differ, the shared object is
/// gone, each page's tree cites its own ids, and the self-check passed on both — page A binds
/// seven runs, page B six, with the dropped run's operator foreign on B.
#[test]
fn a_shared_content_stream_is_not_edited_in_place() {
    let source = engine_fixture("shared-content-stream");
    let before = lopdf::Document::load_mem(&source).unwrap();
    let pages_before = before.get_pages();
    let shared = before.get_page_contents(pages_before[&1]);
    assert_eq!(
        shared,
        before.get_page_contents(pages_before[&2]),
        "one stream, two pages"
    );
    assert_eq!(shared.len(), 1);

    let written = tag(&source).expect("tags cleanly through the self-check");
    let after = lopdf::Document::load_mem(&written).unwrap();
    let pages = after.get_pages();
    let a = after.get_page_contents(pages[&1]);
    let b = after.get_page_contents(pages[&2]);
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert_ne!(a, b, "two streams");
    assert!(
        after.get_object(shared[0]).is_err(),
        "the shared object is gone"
    );
    assert_ne!(
        after.get_page_content(pages[&1]),
        after.get_page_content(pages[&2]),
        "and they differ, because the plans differ"
    );
    assert_eq!(written_sequences(&page_ops(&written, 1)).len(), 2);
    assert_eq!(written_sequences(&page_ops(&written, 2)).len(), 3);

    let t = tree(&written);
    let divs = &t.elements[0].kids;
    assert_eq!(divs.len(), 4, "two blocks per page");
    assert_eq!(divs[0].ids, [0]);
    assert_eq!(divs[1].ids, [1]);
    assert_eq!(
        divs[2].ids,
        [0, 1],
        "page B's block 1 is split around the dropped run"
    );
    assert_eq!(divs[3].ids, [2]);
    assert_eq!(t.next_key, 2);
    assert_eq!(
        t.parent_tree,
        vec![
            (0, vec![("Div".into(), 0), ("Div".into(), 1)]),
            (
                1,
                vec![("Div".into(), 2), ("Div".into(), 2), ("Div".into(), 3)]
            )
        ]
    );

    let artifact = extract(&written);
    let ids = mcids(&artifact);
    assert_eq!(ids.len(), 13, "seven runs on A, six on B");
    assert_eq!(
        ids[2],
        (1, "É".into(), Some(0)),
        "page A reads the seventh run"
    );
    assert_eq!(ids[7], (2, "Water finds its level".into(), Some(0)));
    assert_eq!(ids[8], (2, "and stone keeps its shape".into(), Some(0)));
    assert_eq!(ids[9], (2, "through the long season".into(), Some(1)));
    assert_eq!(ids[10], (2, "Wind moves the grass".into(), Some(2)));
    let detail = &declaration(&artifact, codes::STRUCTURE_TREE_ENGINE_WRITTEN).detail;
    assert!(
        detail.contains("5 of its 5 structure element(s)"),
        "{detail}"
    );
    assert!(detail.contains("13 text run(s)"), "{detail}");
    assert!(
        codes_of(&artifact).contains(&ethos_parser_pdf::limitations::BROKEN_FONT_ENCODING),
        "the dropped run is still declared"
    );
}

/// **An inline image is never enclosed** (scope §3.4, §3.5): on `inline-image-filtered` the
/// image is one operation outside every written sequence, block 1 is two sequences around it,
/// and the image is still counted on the tagged output.
#[test]
fn an_inline_image_is_never_enclosed() {
    let written = tagged("inline-image-filtered");
    let ops = page_ops(&written, 1);
    let image = operator_at(&ops, "BI", None);
    let seqs = written_sequences(&ops);
    assert_eq!(seqs.len(), 3, "{seqs:?}");
    assert!(
        seqs.iter().all(|w| image < w.first || w.last < image),
        "{seqs:?}"
    );
    assert!(
        seqs[0].last < image && image < seqs[1].first,
        "between block 1's two sequences"
    );
    let t = tree(&written);
    assert_eq!(t.elements[0].kids[0].ids, [0, 1]);
    assert_eq!(t.elements[0].kids[1].ids, [2]);
    let a = extract(&written);
    assert_eq!(
        mcids(&a).iter().map(|(_, _, m)| *m).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(1), Some(2), Some(2), Some(2)]
    );
    assert_eq!(
        *declaration(&a, codes::INLINE_IMAGES_NOT_EMITTED),
        ethos_parser_pdf::limitations::inline_images_not_emitted(1)
    );
}

/// **The nested-frames twin walks to its fixture's tree** (acceptance 1, second half): the
/// writer on `leading-gap-nested-frames` produces `engine-tagged-nested-frames`'s tree, reads
/// back identically, and its content stream is that fixture's token for token.
#[test]
fn the_nested_frames_twin_matches_its_fixture() {
    let written = tagged("leading-gap-nested-frames");
    let by_hand = engine_fixture("engine-tagged-nested-frames");
    assert_eq!(tree(&written), tree(&by_hand));
    assert_eq!(read(&written), read(&by_hand));
    assert_eq!(page_tokens(&written, 1), page_tokens(&by_hand, 1));
    let t = tree(&written);
    assert_eq!(t.elements[0].kids[0].ids, [0, 1, 2]);
    assert_eq!(t.elements[0].kids[1].ids, [3]);
    let seqs = written_sequences(&page_ops(&written, 1));
    assert_eq!(seqs[0].inside.as_deref(), Some("Span"));
    assert_eq!(seqs[1].inside.as_deref(), Some("OC"));
    assert_eq!(seqs[2].inside, None);
    assert_eq!(seqs[3].inside, None);
}
