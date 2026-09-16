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

//! M5 acceptance — `DocumentRepresentation v0` + the `ethos.grounding.v1` projection.
//!
//! This lives in `ethos-parser-cli` because it is the only crate that depends on both `ethos-parser-pdf`
//! (which produces a representation from a PDF) and `ethos-parser-grounding` (which projects one).
//! That is a fact about the dependency graph, not a decision about where logic belongs: every
//! assertion below drives library calls, and `the_cli_path_matches_the_library` proves the
//! binary is the same thing with argument parsing on top.
//!
//! The schema-subset validator is shared with `ethos-parser-grounding`'s own tests by path include,
//! rather than copied — a second copy would drift, and the whole point of a schema-driven
//! validator is that it cannot.

use std::path::{Path, PathBuf};
use std::process::Command;

use ethos_parser_core::{DocumentRepresentation, GeometryPresence, Profile};
use ethos_parser_grounding::GroundingSource;
use serde_json::Value;

#[path = "../../ethos-parser-grounding/tests/schema_subset.rs"]
mod schema_subset;

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

fn manifest() -> Value {
    serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest"),
    )
    .expect("valid JSON")
}

fn path_in(root_name: &str, rel: &str) -> PathBuf {
    let m = manifest();
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

fn conformance(rel: &str) -> PathBuf {
    path_in("conformance", rel)
}
fn engine_fx(name: &str) -> PathBuf {
    path_in("engine", &format!("{name}/document.pdf"))
}
fn bench(name: &str) -> PathBuf {
    path_in("benchmark", name)
}

/// Every conformance fixture that opens, so coverage is not one hand-picked document.
const OPENABLE: [&str; 10] = [
    "synthetic/heading-export",
    "synthetic/hyphenated-line-break",
    "synthetic/ligature-fi-embedded-font",
    "synthetic/list-items",
    "synthetic/rotation-90",
    "synthetic/simple-text",
    "synthetic/two-columns",
    "synthetic/two-lines",
    "failure/image-only-or-blank-page",
    "failure/memory-limit-simulated",
];

fn represent(path: &Path) -> DocumentRepresentation {
    let profile = Profile::default();
    let doc = ethos_parser_pdf::Document::open(path, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    ethos_parser_pdf::to_representation(&extract, &profile).expect("represents")
}

fn ground(path: &Path) -> ethos_parser_grounding::Projection {
    ethos_parser_grounding::project(&represent(path)).expect("projects")
}

/// Every `.rs` file under a crate's `src/`, comments stripped.
///
/// **Recursive on purpose.** The first version of the boundary guards read `src/lib.rs` alone, so
/// a second file in the crate was invisible to all of them — a guard that inspects one file is a
/// guard against one file.
fn crate_code(crate_name: &str) -> String {
    fn walk(dir: &Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).expect("readable") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let src = std::fs::read_to_string(&path).expect("readable");
                for line in src.lines() {
                    if !line.trim_start().starts_with("//") {
                        out.push_str(line);
                        out.push('\n');
                    }
                }
            }
        }
    }
    let dir = repo_root().join(format!("crates/{crate_name}/src"));
    let mut out = String::new();
    walk(&dir, &mut out);
    assert!(
        out.len() > 500,
        "the source scan for {crate_name} found almost nothing"
    );
    out
}

fn as_value(g: &GroundingSource) -> Value {
    serde_json::from_slice(&ethos_parser_grounding::to_canonical_bytes(g).expect("canonical"))
        .expect("valid JSON")
}

// -------------------------------------------------------------------------------------------
// 1. Schema conformance
// -------------------------------------------------------------------------------------------

/// Every emitted artifact validates against the **pinned Ethos schema**.
#[test]
fn every_emitted_grounding_artifact_validates_against_the_schema() {
    let mut checked = 0;
    for id in OPENABLE
        .iter()
        .map(|id| conformance(&format!("{id}/document.pdf")))
        .chain(
            [
                "measured-ink-box",
                "absent-font-metrics",
                "show-text-quote-operators",
            ]
            .iter()
            .map(|n| engine_fx(n)),
        )
    {
        let projection = ground(&id);
        // G2's degradations exist for documents past the schema's limits. None here is, so any
        // that fires on this corpus is a regression or a real document reaching a limit — either
        // way worth seeing rather than absorbing.
        assert_eq!(
            (projection.elements_omitted, projection.tables_withheld),
            (None, None),
            "{} engaged a schema-limit degradation",
            id.display()
        );
        let v = as_value(&projection.source);
        schema_subset::validate(&v).unwrap_or_else(|errs| {
            panic!(
                "{} does not validate:\n  {}",
                id.display(),
                errs.join("\n  ")
            )
        });
        checked += 1;
    }
    assert!(
        checked >= 13,
        "expected the whole openable corpus, got {checked}"
    );
}

/// The validator rejects each rule broken on its own.
///
/// **The most important test in this file.** Without it, every assertion above would be
/// satisfied by a validator that returns `Ok(())` unconditionally — and a validator nobody has
/// watched fail is a validator nobody has tested.
#[test]
fn the_validator_rejects_each_deliberate_break() {
    let base = as_value(&ground(&engine_fx("measured-ink-box")).source);
    schema_subset::validate(&base).expect("the base must be valid, or the breaks prove nothing");

    // (what is broken, how) — each violates exactly one schema rule.
    /// One named way to break exactly one schema rule.
    type Break = (&'static str, Box<dyn Fn(&mut Value)>);
    let breaks: Vec<Break> = vec![
        (
            "artifact_type const",
            Box::new(|v: &mut Value| v["artifact_type"] = Value::from("ethos.grounding.v2")),
        ),
        (
            "schema_version const",
            Box::new(|v: &mut Value| v["schema_version"] = Value::from("2.0.0")),
        ),
        (
            "required key removed",
            Box::new(|v: &mut Value| {
                v.as_object_mut().unwrap().remove("pages");
            }),
        ),
        (
            "additionalProperties at the root",
            Box::new(|v: &mut Value| v["extra"] = Value::from(1)),
        ),
        (
            "additionalProperties inside an element",
            Box::new(|v: &mut Value| v["elements"][0]["extra"] = Value::from(1)),
        ),
        (
            "id pattern",
            Box::new(|v: &mut Value| v["pages"][0]["id"] = Value::from("-nope")),
        ),
        (
            "kind pattern",
            Box::new(|v: &mut Value| v["elements"][0]["kind"] = Value::from("Text_Run")),
        ),
        (
            "sha256 pattern",
            Box::new(|v: &mut Value| v["source"]["sha256"] = Value::from("sha256:zz")),
        ),
        (
            "media_type const",
            Box::new(|v: &mut Value| v["source"]["media_type"] = Value::from("application/x-pdf")),
        ),
        (
            "coordinate unit const",
            Box::new(|v: &mut Value| v["coordinate_system"]["unit"] = Value::from("point")),
        ),
        (
            "rotation enum",
            Box::new(|v: &mut Value| v["pages"][0]["rotation"] = Value::from(45)),
        ),
        (
            "page index minimum",
            Box::new(|v: &mut Value| v["pages"][0]["index"] = Value::from(0)),
        ),
        (
            "bbox x1 minimum",
            Box::new(|v: &mut Value| v["elements"][0]["bbox"][2] = Value::from(0)),
        ),
        (
            "bbox x0 minimum",
            Box::new(|v: &mut Value| v["elements"][0]["bbox"][0] = Value::from(-1)),
        ),
        (
            "bbox arity — items:false",
            Box::new(|v: &mut Value| {
                v["elements"][0]["bbox"] = Value::from(vec![1, 2, 3, 4, 5]);
            }),
        ),
        (
            "wrong type",
            Box::new(|v: &mut Value| v["pages"][0]["width"] = Value::from("wide")),
        ),
        (
            "producer name minLength",
            Box::new(|v: &mut Value| v["producer"]["name"] = Value::from("")),
        ),
    ];

    for (label, mutate) in breaks {
        let mut v = base.clone();
        mutate(&mut v);
        assert!(
            schema_subset::validate(&v).is_err(),
            "the validator accepted an artifact with a broken {label}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// 2-5. Field-exact literals, source binding, bbox shape, pages
// -------------------------------------------------------------------------------------------

#[test]
fn the_field_exact_literals_are_what_the_schema_names() {
    let g = ground(&engine_fx("measured-ink-box")).source;
    assert_eq!(g.artifact_type, "ethos.grounding.v1");
    assert_eq!(g.schema_version, "1.0.0");
    assert_eq!(g.source.media_type, "application/pdf");
    assert_eq!(g.coordinate_system.unit, "centipoint");
    assert_eq!(g.coordinate_system.origin, "top-left");
    // v1-S1: the capability is true — the detector looked. Whether it FOUND anything is the
    // array's business, and on this fixture (no ruling lines) it found none.
    assert!(g.capabilities.tables, "v1-S1 looks for ruled tables");
    assert!(g.capabilities.spans);
    // 0.58.0: every span says where its text lies in its element's, in Unicode scalars — proved
    // end to end by `char_offsets_index_the_element_text_in_unicode_scalars`.
    assert!(g.capabilities.char_offsets);
    assert_eq!(g.producer.name, "ethos-parser");
}

#[test]
fn the_source_digest_binds_to_the_fixture_bytes() {
    for id in OPENABLE {
        let path = conformance(&format!("{id}/document.pdf"));
        let g = ground(&path).source;
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            g.source.sha256,
            format!("sha256:{}", ethos_parser_core::sha256_hex_bytes(&bytes)),
            "{id}: the artifact must bind to the exact bytes it was produced from"
        );
    }
}

#[test]
fn every_emitted_box_is_ordered_and_inside_its_page() {
    let mut boxes = 0;
    for name in ["measured-ink-box"] {
        let g = ground(&engine_fx(name)).source;
        let page_by_id: std::collections::BTreeMap<&str, &ethos_parser_grounding::Page> =
            g.pages.iter().map(|p| (p.id.as_str(), p)).collect();

        for (label, page_id, bbox) in g
            .elements
            .iter()
            .map(|e| {
                (
                    "element",
                    e.page.as_deref().expect("paginated"),
                    e.bbox.expect("paginated"),
                )
            })
            .chain(
                g.spans
                    .iter()
                    .flatten()
                    .map(|s| ("span", s.page.as_str(), s.bbox)),
            )
        {
            let p = page_by_id.get(page_id).expect("page reference resolves");
            let [x0, y0, x1, y1] = bbox;
            assert!(x1 > x0 && y1 > y0, "{label}: zero-area box {bbox:?}");
            assert!(x0 >= 0 && y0 >= 0, "{label}: negative origin {bbox:?}");
            assert!(
                x1 <= p.width && y1 <= p.height,
                "{label}: box {bbox:?} outside page {}x{}",
                p.width,
                p.height
            );
            boxes += 1;
        }
    }
    assert!(boxes > 0, "this test is vacuous with no boxes to check");
}

#[test]
fn pages_are_one_indexed_and_rotation_is_carried() {
    for id in OPENABLE {
        let g = ground(&conformance(&format!("{id}/document.pdf"))).source;
        for p in &g.pages {
            assert!(p.index >= 1, "{id}: pages are 1-indexed");
            assert!(matches!(p.rotation, 0 | 90 | 180 | 270), "{id}");
            assert!(p.width >= 1 && p.height >= 1, "{id}");
        }
    }
    let rotated = ground(&conformance("synthetic/rotation-90/document.pdf")).source;
    assert_eq!(rotated.pages[0].rotation, 90);
}

// -------------------------------------------------------------------------------------------
// 6-8. The omission rule
// -------------------------------------------------------------------------------------------

/// The record keeps the node; the projection omits it, counts it, and the record declares it.
#[test]
fn a_node_without_measurable_geometry_is_kept_counted_and_declared() {
    let repr = represent(&engine_fx("absent-font-metrics"));

    // The record keeps it, in full.
    assert_eq!(repr.payload().nodes.len(), 1);
    let node = &repr.payload().nodes[0];
    assert_eq!(node.text, "No ink metrics");
    assert!(
        matches!(repr.geometry_at(0), Some(GeometryPresence::Absent(_))),
        "geometry is typed-absent, not a fabricated box"
    );
    // The native locator survives, which is what makes the node still evidence.
    match &node.native_locator {
        ethos_parser_core::NativeLocator::Pdf(l) => {
            assert_eq!(l.page, 1);
            assert_eq!(
                l.advance,
                Some(16800),
                "the advance is KNOWN — the absence is metrics"
            );
        }
        other => panic!("v0 emits only the PDF locator; got {other:?}"),
    }

    // The projection omits it and counts it.
    let p = ethos_parser_grounding::project(&repr).unwrap();
    assert!(p.source.elements.is_empty());
    assert_eq!(p.omission.nodes_omitted, 1);
    assert_eq!(p.omission.nodes_total, 1);
    assert!(p.omission.is_lossy());

    // The record declares it, with the count, under the same code.
    let declared = repr
        .payload()
        .assurance
        .limitations
        .iter()
        .find(|l| l.code == ethos_parser_grounding::GEOMETRY_ABSENT_OMITTED)
        .expect("the omission must be declared where a consumer can find it");
    assert!(declared.detail.contains("1 of 1 text node(s)"));
    // v1-S6.2 split the count by reason. This node's font supplies no metrics, so it is the
    // reader's own limitation — as distinct from a run of spaces, which has nothing to measure and
    // is not a limitation of anything.
    assert!(declared.detail.contains("1 node(s) could NOT be measured"));
    assert!(declared.detail.contains("0 node(s) had NOTHING to measure"));

    // And the artifact is still schema-valid with an empty elements array.
    schema_subset::validate(&as_value(&p.source)).expect("empty elements is legal");
}

/// **Turned text grounds along its baseline, and a run along neither axis is omitted and counted**
/// (docs/22 §9 items 1 and 2).
///
/// Six of `rotated-and-mirrored-text`'s seven runs have an axis-aligned box — four of them typed
/// `no_ink_to_measure` through 0.57.0 and omitted for it — and the 45-degree run has none, so it
/// is the one node the projection leaves out.
#[test]
fn turned_text_grounds_and_an_off_axis_run_is_omitted() {
    let repr = represent(&engine_fx("rotated-and-mirrored-text"));
    let p = ethos_parser_grounding::project(&repr).expect("projects");

    assert_eq!(p.omission.nodes_total, 7);
    assert_eq!(p.omission.nodes_omitted, 1, "only the diagonal run");
    assert_eq!(
        p.source.elements.len(),
        6,
        "no two runs share a line, so each grounded run is its own element"
    );
    assert!(
        p.source
            .elements
            .iter()
            .all(|e| e.text.as_deref() != Some("Diagonal")),
        "a run with no axis-aligned box never enters the artifact"
    );
    schema_subset::validate(&as_value(&p.source)).expect("the artifact validates");
}

/// The omission **selects**, proved inside a single artifact with mixed geometry.
///
/// The version of this test that compared two all-or-nothing documents proved nothing: an emitter
/// that drops every box as soon as *any* node lacks one satisfied it completely, which is exactly
/// the "omit for a non-geometry reason" hole §11 forbids. `irs-form-1040-2025` is the fixture that
/// can tell the difference — a real 2-page form where all but one node has a measurable box.
#[test]
fn omission_selects_rather_than_empties() {
    let path = bench("irs-form-1040-2025.pdf");
    let repr = represent(&path);
    let p = ethos_parser_grounding::project(&repr).expect("projects");

    let total = repr.payload().nodes.len();
    assert!(
        p.omission.nodes_omitted > 0 && (p.omission.nodes_omitted as usize) < total,
        "this test is worthless unless the document has BOTH kinds: {} omitted of {total}",
        p.omission.nodes_omitted
    );

    // The set that survived is exactly the set with measured geometry — not a prefix, not a
    // count that happens to match. An emitter that dropped the wrong nodes would pass a count
    // assertion and fail this one.
    let expected: std::collections::BTreeSet<&str> = repr
        .payload()
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| matches!(repr.geometry_at(*i), Some(GeometryPresence::Measured(_))))
        .map(|(_, n)| n.id.as_str())
        .collect();
    let emitted: std::collections::BTreeSet<&str> = p
        .source
        .spans
        .as_ref()
        .expect("spans are claimed")
        .iter()
        .map(|s| s.id.as_str())
        .collect();
    assert_eq!(emitted, expected, "the projection kept the wrong nodes");
    assert_eq!(total - expected.len(), p.omission.nodes_omitted as usize);
    // v2.2-S7: the element is the BLOCK and the span is the run, so elements are no longer 1:1
    // with grounded nodes. What must still hold is that every span sits in an element and no
    // element is empty — the hierarchy is populated, not merely present.
    assert!(
        p.source.elements.len() <= expected.len(),
        "blocks cannot outnumber the runs they group"
    );
    let element_ids: std::collections::BTreeSet<&str> =
        p.source.elements.iter().map(|e| e.id.as_str()).collect();
    let referenced: std::collections::BTreeSet<&str> = p
        .source
        .spans
        .as_ref()
        .expect("spans are claimed")
        .iter()
        .map(|s| s.element.as_deref().expect("every span names its element"))
        .collect();
    assert_eq!(
        referenced, element_ids,
        "every element holds at least one span, and every span names a real element"
    );

    schema_subset::validate(&as_value(&p.source)).expect("a mixed document still validates");
}

/// Every emitted box **is the box that was measured**, not merely a well-shaped one.
///
/// Shape and page-containment assertions pass for a fabricated `[x0, y0, x0+1, y0+1]`. For a
/// project whose thesis is that no geometry is invented, fidelity is the assertion that matters.
#[test]
fn every_emitted_box_is_the_measured_box() {
    let mut compared = 0;
    for path in [
        bench("irs-form-1040-2025.pdf"),
        engine_fx("measured-ink-box"),
    ] {
        let repr = represent(&path);
        let g = ethos_parser_grounding::project(&repr)
            .expect("projects")
            .source;

        let by_id: std::collections::BTreeMap<&str, [i64; 4]> = repr
            .payload()
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| match repr.geometry_at(i) {
                Some(GeometryPresence::Measured(r)) => {
                    Some((n.id.as_str(), [r.x0(), r.y0(), r.x1(), r.y1()]))
                }
                _ => None,
            })
            .collect();

        for span in g.spans.iter().flatten() {
            let measured = by_id
                .get(span.id.as_str())
                .expect("every emitted span names a node with measured geometry");
            assert_eq!(
                &span.bbox, measured,
                "span {} carries a box that was not measured",
                span.id
            );
            compared += 1;
        }
        // v2.2-S7: an element is a block, so its box is the UNION of its spans' measured boxes —
        // a stronger claim than the equality this asserted while the two granularities coincided.
        // Nothing is inferred: a union of measured rectangles is measured.
        let mut union: std::collections::BTreeMap<&str, [i64; 4]> =
            std::collections::BTreeMap::new();
        for span in g.spans.iter().flatten() {
            let e = span
                .element
                .as_deref()
                .expect("every span names its element");
            union
                .entry(e)
                .and_modify(|u| {
                    *u = [
                        u[0].min(span.bbox[0]),
                        u[1].min(span.bbox[1]),
                        u[2].max(span.bbox[2]),
                        u[3].max(span.bbox[3]),
                    ]
                })
                .or_insert(span.bbox);
        }
        for e in &g.elements {
            assert_eq!(
                e.bbox.expect("paginated"),
                union[e.id.as_str()],
                "element {} is not the union of its spans",
                e.id
            );
        }
    }
    assert!(
        compared > 100,
        "expected real coverage, compared only {compared}"
    );
}

/// Measured geometry is never omitted for a reason that is not about geometry.
///
/// Asserted structurally rather than by inspection: the only constructor for an emittable box
/// takes a `GeometryPresence`. There is no overload taking a bool, a reason code, or a page
/// state, so no quality judgement can reach the omission path — and this test is what fails if
/// someone adds one.
#[test]
fn the_omission_call_site_takes_a_measurement_state_and_nothing_else() {
    use ethos_parser_core::{GeometryAbsence, QRect};
    use ethos_parser_grounding::GroundedBox;

    let r = QRect::new(1, 2, 3, 4).unwrap();
    assert!(GroundedBox::from_presence(GeometryPresence::Measured(r)).is_some());
    for a in [
        GeometryAbsence::NotReportedByReader,
        GeometryAbsence::NotApplicableToKind,
        GeometryAbsence::CapabilityNotEnabled,
    ] {
        assert!(
            GroundedBox::from_presence(GeometryPresence::Absent(a)).is_none(),
            "every absence variant omits — the schema can express none of them"
        );
    }

    // A measured box survives regardless of what the rest of the document looks like. This is
    // the "never omitted for a non-geometry reason" half: the fixture that produces a measured
    // box also fires classification reasons and carries limitations, and the box is still there.
    let g = ground(&engine_fx("measured-ink-box")).source;
    assert_eq!(g.elements.len(), 1);

    // The guarantee is the **module boundary**, not a source scan. `GroundedBox` lives in its own
    // module with the tuple field private to it, so `project()` — which is outside that module —
    // cannot construct one either. An earlier version kept the type beside the projection with
    // only a private field and leaned on a grep to cover the gap; an audit walked straight past
    // that grep by writing `fn from_raw(x0, y0, x1, y1) -> Self`, which matched none of its
    // patterns. What is asserted here is the structure that makes such a function impossible to
    // write outside one small module a reviewer can read in full.
    let code = crate_code("ethos-parser-grounding");
    assert!(
        code.contains("mod grounded_box"),
        "GroundedBox must live in its own module — that boundary is what stops `project()` from \
         building a box out of something that is not a measurement state"
    );
    let outside = code
        .split("mod grounded_box")
        .next()
        .expect("text before the module")
        .to_string()
        + code
            .split("pub use grounded_box::GroundedBox;")
            .nth(1)
            .unwrap_or("");
    assert!(
        !outside.contains("GroundedBox(") && !outside.contains("Self(QRect"),
        "a GroundedBox is constructed outside its own module, which the private field is meant \
         to make impossible"
    );
}

// -------------------------------------------------------------------------------------------
// 9. Lossiness
// -------------------------------------------------------------------------------------------

/// What the projection drops, enumerated — so nobody mistakes a grounding round-trip for proof
/// the representation is intact.
#[test]
fn lossiness_is_asserted_not_assumed() {
    let repr = represent(&conformance(
        "synthetic/ligature-fi-embedded-font/document.pdf",
    ));
    let g = as_value(
        &ground(&conformance(
            "synthetic/ligature-fi-embedded-font/document.pdf",
        ))
        .source,
    );
    let text = serde_json::to_string(&g).unwrap();

    // Present in the record...
    let r = serde_json::to_string(&serde_json::to_value(&repr).unwrap()).unwrap();
    for present in [
        "derivation",
        "char_codes",
        "scalar_code_mismatch",
        "synthesized",
        "native_locator",
        "assurance",
        "coverage",
        "terminal_state",
        "limitations",
        "font_id",
        "font_size",
        "representation_c14n_sha256",
    ] {
        assert!(r.contains(present), "the record must carry `{present}`");
    }

    // ...and gone from the projection. Every one of these is a thing a consumer might assume
    // survived, and none does.
    for dropped in [
        "derivation",
        "char_codes",
        "scalar_code_mismatch",
        "synthesized",
        "native_locator",
        "structural_locator",
        "mcid",
        "assurance",
        "coverage",
        "terminal_state",
        "limitations",
        "page_states",
        "font_id",
        "font_size",
        "reading_order_rule",
        "processing_run",
        "representation_c14n_sha256",
    ] {
        assert!(
            !text.contains(dropped),
            "`{dropped}` survived into the grounding artifact; either the projection grew a \
             field the schema forbids, or this inventory is now wrong"
        );
    }
}

// -------------------------------------------------------------------------------------------
// 10-13. Determinism, boundaries, invariants, no confidence
// -------------------------------------------------------------------------------------------

#[test]
fn representation_and_grounding_are_byte_identical_across_runs() {
    for id in OPENABLE {
        let path = conformance(&format!("{id}/document.pdf"));
        let r1 = represent(&path).to_canonical_bytes().unwrap();
        let r2 = represent(&path).to_canonical_bytes().unwrap();
        assert_eq!(r1, r2, "{id}: representation is not byte-identical");

        let g1 = ethos_parser_grounding::to_canonical_bytes(&ground(&path).source).unwrap();
        let g2 = ethos_parser_grounding::to_canonical_bytes(&ground(&path).source).unwrap();
        assert_eq!(g1, g2, "{id}: grounding is not byte-identical");
    }
}

/// `ethos-parser-grounding` contains no PDF concept, and never reads a locator.
#[test]
fn ethos_parser_grounding_has_no_pdf_concept() {
    let code = crate_code("ethos-parser-grounding");
    for banned in [
        "lopdf",
        "ethos_parser_pdf",
        "content_stream",
        "page_tree",
        "font_descriptor",
    ] {
        assert!(
            !code.contains(banned),
            "`{banned}` appears in ethos-parser-grounding code"
        );
    }
    // Stronger than "no PDF type": the projection never inspects a native locator at all, so a
    // second format needs no change here. Prose about it is fine; a reference in code is not.
    for banned in ["NativeLocator", "PdfLocator", "StructuralLocator"] {
        assert!(
            !code.contains(banned),
            "ethos-parser-grounding reads `{banned}`; the projection addresses pages by node id \
             precisely so it never has to"
        );
    }

    let manifest =
        std::fs::read_to_string(repo_root().join("crates/ethos-parser-grounding/Cargo.toml"))
            .unwrap();
    assert!(
        !manifest.contains("ethos-parser-pdf"),
        "ethos-parser-grounding must not depend on ethos-parser-pdf"
    );
    assert!(!manifest.contains("lopdf"));
}

/// The invariants Ethos enforces that the JSON Schema does not express.
///
/// M5 owes schema conformance. But the engine may be stricter on emission and never looser, and
/// emitting something the consuming verifier would reject is a defect worth catching now rather
/// than at M6 — these were measured from `../ethos/crates/ethos-core/src/grounding_json.rs`.
#[test]
fn the_artifact_satisfies_the_invariants_the_schema_cannot_express() {
    for path in OPENABLE
        .iter()
        .map(|id| conformance(&format!("{id}/document.pdf")))
        .chain(
            ["measured-ink-box", "absent-font-metrics"]
                .iter()
                .map(|n| engine_fx(n)),
        )
    {
        let g = ground(&path).source;
        let v = as_value(&g);
        let label = path.display().to_string();

        // capabilities.spans <=> spans present; capabilities.tables <=> tables present.
        assert_eq!(
            g.capabilities.spans,
            g.spans.is_some(),
            "{label}: the spans array must be present exactly when the capability is claimed"
        );
        assert_eq!(g.capabilities.tables, g.tables.is_some(), "{label}");
        // v1-S1 inverted this one, and the inversion is the whole point of the encoding:
        // `tables: true` means the key is PRESENT — possibly an empty array, which says "looked,
        // found none". An absent key would say "did not look", which is no longer true.
        assert!(
            v.get("tables").is_some(),
            "{label}: tables:true means the key is PRESENT, empty array included"
        );
        // char_offsets requires spans, and offsets are present exactly when claimed.
        assert!(
            !g.capabilities.char_offsets || g.capabilities.spans,
            "{label}"
        );
        let element_text: std::collections::BTreeMap<&str, &str> = g
            .elements
            .iter()
            .filter_map(|e| e.text.as_deref().map(|t| (e.id.as_str(), t)))
            .collect();
        for s in g.spans.iter().flatten() {
            assert_eq!(
                s.char_start.is_some() || s.char_end.is_some(),
                g.capabilities.char_offsets,
                "{label}: offsets present must equal the capability"
            );
            if !g.capabilities.char_offsets {
                continue;
            }
            // The rule the verifier applies, applied here: the offsets are complete, ordered, and
            // the element's text SLICED BY SCALARS is exactly the span's text. On these fixtures it
            // cannot tell a wrong cursor from the right one: their 15 spans are ASCII and each is
            // its element's only span, so a byte cursor and a boxed-only cursor write the same
            // numbers. The unit and the member cursor are pinned by the grounding crate's
            // `offsets_count_every_member_of_the_block_in_unicode_scalars` and
            // `offsets_come_from_each_members_position_not_from_searching_the_text`, and end to
            // end by `char_offsets_index_the_element_text_in_unicode_scalars`.
            let (start, end) = (
                s.char_start.expect("claimed") as usize,
                s.char_end.expect("claimed") as usize,
            );
            assert!(start <= end, "{label}: {start}..{end} is not ordered");
            let id = s.element.as_deref().expect("a span names its element");
            let text = element_text.get(id).unwrap_or_else(|| {
                panic!(
                    "{label}: span {} names element {id}, which carries no text",
                    s.id
                )
            });
            assert_eq!(
                text.chars()
                    .skip(start)
                    .take(end - start)
                    .collect::<String>(),
                s.text,
                "{label}: span {} does not lie at {start}..{end} of its element's text",
                s.id
            );
        }

        // Page indices ascend from 1 with no gaps, which Ethos checks and the schema does not.
        for (i, p) in g.pages.iter().enumerate() {
            assert_eq!(p.index as usize, i + 1, "{label}: page order");
        }

        // Ids unique across elements and spans; every reference resolves.
        let page_ids: std::collections::BTreeSet<&str> =
            g.pages.iter().map(|p| p.id.as_str()).collect();
        let mut seen = std::collections::BTreeSet::new();
        for e in &g.elements {
            assert!(seen.insert(e.id.as_str()), "{label}: duplicate element id");
            assert!(
                page_ids.contains(e.page.as_deref().expect("paginated")),
                "{label}: dangling page ref"
            );
        }
        let element_ids: std::collections::BTreeSet<&str> =
            g.elements.iter().map(|e| e.id.as_str()).collect();
        for s in g.spans.iter().flatten() {
            assert!(seen.insert(s.id.as_str()), "{label}: id collides");
            assert!(
                page_ids.contains(s.page.as_str()),
                "{label}: dangling page ref"
            );
            if let Some(e) = &s.element {
                assert!(
                    element_ids.contains(e.as_str()),
                    "{label}: dangling element ref"
                );
            }
        }

        // Strings are bounded in BYTES, which is how the verifier measures them.
        let bytes = ethos_parser_grounding::to_canonical_bytes(&g).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        // Scanned OUTSIDE quoted strings. Document text legitimately contains full stops, and a
        // substring search for '.' flags the content instead of the hazard — which is the wrong
        // instrument, and the one this check reached for first.
        let mut in_string = false;
        let chars: Vec<char> = s.chars().collect();
        for i in 0..chars.len() {
            match chars[i] {
                '"' if i == 0 || chars[i - 1] != '\\' => in_string = !in_string,
                '.' if !in_string => panic!("{label}: a bare decimal reached the artifact: {s}"),
                _ => {}
            }
        }
        for e in &g.elements {
            if let Some(t) = &e.text {
                assert!(
                    t.len() <= 16384,
                    "{label}: element text exceeds the byte cap"
                );
            }
        }
    }
}

#[test]
fn no_confidence_in_a_representation_or_a_grounding_artifact() {
    let repr = represent(&conformance("synthetic/simple-text/document.pdf"));
    let g = ground(&conformance("synthetic/simple-text/document.pdf")).source;
    for (label, text) in [
        (
            "representation",
            String::from_utf8(repr.to_canonical_bytes().unwrap()).unwrap(),
        ),
        (
            "grounding",
            String::from_utf8(ethos_parser_grounding::to_canonical_bytes(&g).unwrap()).unwrap(),
        ),
    ] {
        let lower = text.to_lowercase();
        for banned in [
            "confidence",
            "score",
            "quality",
            "grade",
            "certainty",
            "verdict",
            "grounded",
        ] {
            assert!(
                !lower.contains(banned),
                "`{banned}` appears in the {label} artifact"
            );
        }
    }
}

// -------------------------------------------------------------------------------------------
// The CLI is a thin shell
// -------------------------------------------------------------------------------------------

#[test]
fn the_cli_path_matches_the_library() {
    let pdf = engine_fx("measured-ink-box");

    let extract_out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("extract")
        .arg(&pdf)
        .output()
        .expect("runs");
    assert_eq!(extract_out.status.code(), Some(0));

    let expected_repr = represent(&pdf).to_canonical_bytes().unwrap();
    assert_eq!(
        extract_out.stdout.trim_ascii_end(),
        expected_repr.as_slice(),
        "`ethos-parser extract` must emit exactly the library's representation bytes"
    );

    // Round-trip through a file, exactly as a caller would.
    let dir = std::env::temp_dir().join(format!("ethos-parser-m5-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let repr_path = dir.join("repr.json");
    std::fs::write(&repr_path, &extract_out.stdout).unwrap();

    let ground_out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("ground")
        .arg(&repr_path)
        .output()
        .expect("runs");
    assert_eq!(
        ground_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&ground_out.stderr)
    );

    let expected_ground = ethos_parser_grounding::to_canonical_bytes(&ground(&pdf).source).unwrap();
    assert_eq!(
        ground_out.stdout.trim_ascii_end(),
        expected_ground.as_slice()
    );
    schema_subset::validate(&serde_json::from_slice(ground_out.stdout.trim_ascii_end()).unwrap())
        .expect("the binary's output validates too");

    let _ = std::fs::remove_dir_all(&dir);
}

/// A representation whose payload does not hash to its declared digest is refused.
#[test]
fn ground_refuses_a_representation_whose_fingerprint_disagrees() {
    let pdf = engine_fx("measured-ink-box");
    let mut v: Value =
        serde_json::from_slice(&represent(&pdf).to_canonical_bytes().unwrap()).unwrap();
    // Change the evidence without changing the digest — the tampering case the field exists for.
    v["representation"]["nodes"][0]["text"] = Value::from("Fabricated");

    let dir = std::env::temp_dir().join(format!("ethos-parser-m5-tamper-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("tampered.json");
    std::fs::write(&p, serde_json::to_vec(&v).unwrap()).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("ground")
        .arg(&p)
        .output()
        .expect("runs");
    assert_eq!(
        out.status.code(),
        Some(2),
        "a disagreeing digest must refuse"
    );
    assert!(
        out.stdout.is_empty(),
        "no artifact may be emitted from a tampered record"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The omission count reaches a human, and never the artifact.
#[test]
fn the_omission_report_goes_to_stderr_so_stdout_stays_byte_identical() {
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("extract")
        .arg(engine_fx("absent-font-metrics"))
        .output()
        .expect("runs");
    let dir = std::env::temp_dir().join(format!("ethos-parser-m5-omit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("repr.json");
    std::fs::write(&p, &out.stdout).unwrap();

    let g = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("ground")
        .arg(&p)
        .output()
        .expect("runs");
    let stderr = String::from_utf8_lossy(&g.stderr);
    assert!(
        stderr.contains("1 of 1 node(s) omitted"),
        "stderr was: {stderr}"
    );
    assert!(
        !String::from_utf8_lossy(&g.stdout).contains("omitted"),
        "the count must not leak into the artifact — the schema forbids the field and byte \
         identity forbids the noise"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A budgeted run: the record stays coherent when pages are quarantined.
///
/// The interaction M4 and M5 create between them, and the one most likely to go wrong quietly.
/// With a page budget the representation carries *fewer page records than the document has
/// pages*, so page `index` stops equalling array position — and every downstream number that was
/// silently relying on that equality breaks. The grounding artifact must still validate, its page
/// indices must still be the document's own, and the coverage must still say what happened to the
/// page nobody read.
#[test]
fn a_budgeted_run_still_produces_a_coherent_record_and_projection() {
    let profile = Profile {
        page_budget: ethos_parser_core::PageBudget::AtMost(1),
        ..Profile::default()
    };
    let path = {
        let m = manifest();
        let decl = &m["roots"]["benchmark"];
        let root = match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
            Some(v) => PathBuf::from(v),
            None => repo_root().join(decl["default"].as_str().unwrap()),
        };
        root.join("irs-form-1040-2025.pdf")
    };

    let doc = ethos_parser_pdf::Document::open(&path, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");

    // The record carries one page of a two-page document, and says so rather than implying it.
    assert_eq!(repr.payload().pages.len(), 1);
    assert_eq!(repr.payload().pages[0].index, 1);
    let coverage = repr.payload().assurance.coverage;
    assert_eq!(coverage.pages_authorized, 2);
    assert_eq!(coverage.pages_processed, 1);
    assert_eq!(coverage.pages_quarantined, 1);
    assert!(
        !repr.payload().assurance.is_complete(),
        "a gap is not a complete reading"
    );

    // Every node still parents to a page that exists, with coherent ordinals.
    for node in &repr.payload().nodes {
        assert!(repr.payload().pages.iter().any(|p| p.id == node.parent));
        assert!(node.ordinal >= 1);
    }

    // And the projection is still a valid artifact — it just describes less of the document.
    let p = ethos_parser_grounding::project(&repr).expect("projects");
    schema_subset::validate(&as_value(&p.source)).expect("a partial record still projects validly");
    assert_eq!(p.source.pages.len(), 1);
    assert_eq!(p.source.pages[0].index, 1);

    // The gap is NOT visible in the grounding artifact — it cannot be, the schema has nowhere to
    // put it. Pinned here so the asymmetry is a recorded fact rather than a surprise: a consumer
    // who needs to know a document was partially read has to read the representation.
    let text = serde_json::to_string(&as_value(&p.source)).unwrap();
    assert!(!text.contains("quarantined") && !text.contains("coverage"));
}
