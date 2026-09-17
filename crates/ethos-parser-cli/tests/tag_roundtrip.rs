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

//! Auto-tagging S3: the round trip — write, read back, project, ground, verify
//! (`docs/24-AUTO-TAGGING-MILESTONES.md` §S3; `docs/23-AUTO-TAGGING-SCOPE.md` §2, §4.2, §4.3).
//!
//! v2.2's second clause is *a tag this engine writes is one it can read back and ground
//! against*, and this file is where the three verbs meet on one document: the writer's bytes are
//! opened by the reader, the representation is projected three ways, the grounding artifact is
//! checked, and the pinned verifier answers the same claim on the tagged document and on its
//! untagged original. It lives in `ethos-parser-cli` for the reason `grounding.rs` gives: this is
//! the only crate that depends on the writer and the reader, the projections and the grounding
//! projection at once, and it is where the relay to the verifier is driven.
//!
//! # What S2 already holds, and is not repeated here
//!
//! `crates/ethos-parser-pdf/tests/tagging_write.rs` reads the **extract artifact** of the tagged
//! bytes on these same four fixtures (`the_round_trip_keeps_the_text_record_and_binds_every_run_computed`):
//! every run bound `pdf_tagged` with `derivation: computed`, the text record the source's run for
//! run, the engine-written tree declared and the untagged declaration absent. It also holds
//! `a_second_write_is_refused`, because the writer is where that belongs. This file takes the
//! other half of the clause, on the **representation** — the artifact `markdown`, `html` and
//! `ground` read, whose assurance `represent.rs` rebuilds — and everything downstream of it.
//!
//! # The verifier is required, and its absence is a failure
//!
//! Same posture as `verify_relay.rs`: the binary is resolved the way the oracle harness resolves
//! it, and a machine with no verifier fails these tests by name rather than skipping them. The
//! verify boundary is not crossed (`docs/07-VERIFY-BOUNDARY.md`): the report is read only through
//! `common::verification_report`, the harness's existing helper, and nothing here derives a
//! verdict of its own.
//!
//! Fixtures resolve through `fixtures/manifest.json`'s roots. **A missing fixture is a failure,
//! never a skip.**

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ethos_parser_core::{
    codes, DerivationClass, DocumentRepresentation, HtmlArtifact, Limitation, LimitationScope,
    MarkdownArtifact, NodeKind, Profile, StructuralLocator,
};
use ethos_parser_grounding::{Projection, SourceBinding, Structure};
use ethos_parser_pdf::{write_tags, Document};
use lopdf::Object;
use serde_json::Value;

mod common;

// -------------------------------------------------------------------------------------------
// Fixtures and the verifier
// -------------------------------------------------------------------------------------------

/// The four untagged fixtures §S3 names: two blocks by the leading-gap cut, two column bands,
/// four runs on one baseline, and the anchor-map golden.
const ROUND_TRIP: [&str; 4] = [
    "leading-gap-two-blocks",
    "two-column-15-lines",
    "untagged-shredded-line",
    "markdown-two-blocks",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn engine_fixture(name: &str) -> Vec<u8> {
    let manifest: Value = serde_json::from_slice(
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

/// The verifier, resolved exactly as `verify_relay.rs` and the oracle harness resolve it.
fn ethos_binary() -> PathBuf {
    if let Some(v) = std::env::var_os("ETHOS_BIN") {
        let p = PathBuf::from(&v);
        assert!(
            p.is_file(),
            "ETHOS_BIN is set to `{}`, which is not a file. An explicit pin is authoritative.",
            p.display()
        );
        return p;
    }
    let sibling = repo_root().join("../ethos/target/release/ethos");
    if sibling.is_file() {
        return sibling;
    }
    if let Ok(out) = Command::new("sh")
        .arg("-c")
        .arg("command -v ethos")
        .output()
    {
        let found = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !found.is_empty() {
            return PathBuf::from(found);
        }
    }
    panic!(
        "the Ethos CLI could not be located. It is a test-time dependency and its absence is a \
         FAILURE, never a skip (docs/04-ARCHITECTURE.md §4). Build it with `cargo build \
         --release` in the Ethos repo, or set ETHOS_BIN."
    );
}

/// `ethos-parser verify`, with the verifier the harness resolved pinned for the child too, so
/// the relay cannot fall through to a different binary than the one this test named.
fn verify(grounding: &Path, citations: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .arg("verify")
        .arg(grounding)
        .arg("--citations")
        .arg(citations)
        .env("ETHOS_BIN", ethos_binary())
        .output()
        .expect("the engine binary runs")
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-s3-roundtrip-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

// -------------------------------------------------------------------------------------------
// The twin: one fixture, untagged and engine-tagged
// -------------------------------------------------------------------------------------------

fn open(bytes: &[u8]) -> Document {
    Document::open_bytes(bytes, &Profile::default()).expect("the document opens")
}

fn represent(bytes: &[u8]) -> DocumentRepresentation {
    let profile = Profile::default();
    let doc = open(bytes);
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    ethos_parser_pdf::to_representation(&extract, &profile).expect("represents")
}

/// One fixture on both sides of the writer.
struct Twin {
    name: &'static str,
    untagged_pdf: Vec<u8>,
    tagged_pdf: Vec<u8>,
    untagged: DocumentRepresentation,
    tagged: DocumentRepresentation,
}

fn twin(name: &'static str) -> Twin {
    let untagged_pdf = engine_fixture(name);
    let tagged_pdf = write_tags(&open(&untagged_pdf), &Profile::default())
        .unwrap_or_else(|e| panic!("{name} tags cleanly: {e}"));
    Twin {
        name,
        untagged: represent(&untagged_pdf),
        tagged: represent(&tagged_pdf),
        untagged_pdf,
        tagged_pdf,
    }
}

fn twins() -> Vec<Twin> {
    ROUND_TRIP.iter().map(|n| twin(n)).collect()
}

fn codes_of(repr: &DocumentRepresentation) -> Vec<&str> {
    repr.payload()
        .assurance
        .limitations
        .iter()
        .map(|l| l.code.as_str())
        .collect()
}

// -------------------------------------------------------------------------------------------
// The three projections, and the members that name the input rather than describe it
// -------------------------------------------------------------------------------------------

fn markdown(repr: &DocumentRepresentation) -> MarkdownArtifact {
    let profile = Profile::default();
    ethos_parser_core::to_markdown(
        repr,
        &profile.parser_version,
        &profile.profile_sha256().unwrap(),
        &profile.markdown_rule,
    )
    .expect("projects to Markdown")
}

fn html(repr: &DocumentRepresentation) -> HtmlArtifact {
    let profile = Profile::default();
    ethos_parser_core::to_html(
        repr,
        &profile.parser_version,
        &profile.profile_sha256().unwrap(),
        &profile.html_rule,
    )
    .expect("projects to HTML")
}

fn ground(repr: &DocumentRepresentation) -> Projection {
    ethos_parser_grounding::project(repr).expect("projects to grounding")
}

/// Canonical bytes of a JSON value with the named members deleted. Each path must exist: a
/// comparison that silently skipped a member it meant to remove would be comparing the wrong
/// thing and passing.
fn canonical_without(bytes: &[u8], paths: &[&[&str]]) -> Vec<u8> {
    let mut v: Value = serde_json::from_slice(bytes).expect("canonical JSON");
    for path in paths {
        let (last, parents) = path.split_last().expect("a non-empty path");
        let mut cursor = &mut v;
        for key in parents {
            cursor = cursor
                .get_mut(key)
                .unwrap_or_else(|| panic!("no `{key}` in {path:?}"));
        }
        assert!(
            cursor
                .as_object_mut()
                .unwrap_or_else(|| panic!("{path:?} does not end in an object"))
                .remove(*last)
                .is_some(),
            "no `{last}` to delete at {path:?}"
        );
    }
    ethos_parser_core::c14n_bytes(&v).expect("canonicalises")
}

// -------------------------------------------------------------------------------------------
// 1. Read back, grounded, verified
// -------------------------------------------------------------------------------------------

/// **A written tag is read back and grounds** (§S3, first acceptance).
///
/// On each of the four: every text run of the tagged representation binds `pdf_tagged` with
/// `derivation: computed` under `Document/Div` and stays an `extracted` run — the text was read,
/// the address was computed, and the address says so (scope §4.2); the text record is the
/// source's run for run, node for node, geometry row for geometry row, so the only thing the
/// writer changed is the address; `ground` succeeds and `grounding_check` says the artifact is
/// valid and bound to the tagged bytes; and the pinned verifier, asked about one run's exact text
/// on the tagged document and on the untagged original, gives the same answer.
///
/// The verifier's report is read through `common::verification_report` and nowhere else. The one
/// member the two reports may differ in is `document_fingerprint`, the digest of the grounding
/// file it was handed, and the two grounding files differ by `source.sha256` alone — which is the
/// second test.
#[test]
fn a_written_tag_is_read_back_and_grounds() {
    let dir = scratch("verify");
    for t in twins() {
        let name = t.name;
        let nodes = &t.tagged.payload().nodes;
        assert!(!nodes.is_empty(), "{name}: the fixture shows text");
        for (i, node) in nodes.iter().enumerate() {
            if node.kind != NodeKind::TextRun {
                continue;
            }
            match node.structural_locator.as_ref() {
                Some(StructuralLocator::PdfTagged(l)) => {
                    assert_eq!(l.derivation, DerivationClass::Computed, "{name} node {i}");
                    assert_eq!(l.role_path, ["Document", "Div"], "{name} node {i}");
                }
                other => panic!("{name} node {i} `{}` is not bound: {other:?}", node.text),
            }
            assert_eq!(
                node.derivation,
                DerivationClass::Extracted,
                "{name} node {i}: the text was read; only the address was computed"
            );
        }

        // The record, run for run: everything a node carries except the address the writer
        // added, and the geometry row beside it.
        let strip = |repr: &DocumentRepresentation| -> Vec<_> {
            repr.payload()
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    let mut n = n.clone();
                    n.structural_locator = None;
                    (n, repr.geometry_at(i))
                })
                .collect()
        };
        assert_eq!(
            strip(&t.tagged),
            strip(&t.untagged),
            "{name}: the text record and its geometry are the source's, run for run"
        );
        assert_eq!(
            t.tagged.payload().pages,
            t.untagged.payload().pages,
            "{name}: the pages are the source's"
        );

        // Grounded: the projection succeeds, and the checker binds it to the tagged bytes.
        let tagged_ground = ground(&t.tagged);
        let tagged_bytes =
            ethos_parser_grounding::to_canonical_bytes(&tagged_ground.source).expect("canonical");
        let report = ethos_parser_grounding::grounding_check(&tagged_bytes, Some(&t.tagged_pdf))
            .expect("the check runs");
        assert_eq!(report.structure, Structure::Valid, "{name}: {report:?}");
        assert_eq!(
            report.source_binding,
            SourceBinding::Matched,
            "{name}: the artifact names the tagged bytes"
        );
        assert_eq!(report.exit_code(), 0, "{name}");

        // And the original binds to its own bytes the same way, so the two sides of the verify
        // below are two valid artifacts, each naming the document it was projected from.
        let untagged_ground = ground(&t.untagged);
        let untagged_bytes =
            ethos_parser_grounding::to_canonical_bytes(&untagged_ground.source).expect("canonical");
        let report =
            ethos_parser_grounding::grounding_check(&untagged_bytes, Some(&t.untagged_pdf))
                .expect("the check runs");
        assert_eq!(
            (report.structure, report.source_binding),
            (Structure::Valid, SourceBinding::Matched),
            "{name}: the original's artifact names the original's bytes"
        );

        // Verified: the same claim, relayed to the pinned verifier, on both documents.
        let quote = |p: &Projection| -> (String, String) {
            let span = p
                .source
                .spans
                .as_ref()
                .and_then(|s| s.first())
                .unwrap_or_else(|| panic!("{name}: the projection carries spans"));
            (span.text.clone(), span.page.clone())
        };
        let (text, page) = quote(&untagged_ground);
        assert_eq!(
            quote(&tagged_ground),
            (text.clone(), page.clone()),
            "{name}: the first run is the same run on both sides"
        );
        let (u_report, t_report) = {
            let mut reports = Vec::new();
            for (side, bytes) in [("untagged", &untagged_bytes), ("tagged", &tagged_bytes)] {
                let grounding = dir.join(format!("{name}-{side}.grounding.json"));
                std::fs::write(&grounding, bytes).expect("write grounding");
                // The fingerprint the verifier binds a citations file to is the digest of the
                // grounding file it loaded — `verify_relay.rs` measured this against the pin.
                let fingerprint = format!("sha256:{}", ethos_parser_core::sha256_hex_bytes(bytes));
                let citations = dir.join(format!("{name}-{side}.claims.json"));
                let body = serde_json::json!({
                    "document_fingerprint": fingerprint,
                    "claims": [{
                        "kind": "quote",
                        "text": text,
                        "citation": { "page": page, "element_id": "e1" }
                    }]
                });
                std::fs::write(&citations, serde_json::to_vec_pretty(&body).unwrap())
                    .expect("write claims");
                let out = verify(&grounding, &citations);
                assert_eq!(
                    out.status.code(),
                    Some(0),
                    "{name} {side}: a grounded quote exits 0; stderr: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                reports.push(common::verification_report(&out.stdout));
            }
            let t_report = reports.pop().unwrap();
            (reports.pop().unwrap(), t_report)
        };
        assert_eq!(
            u_report["all_evidence_grounded"], true,
            "{name}: the run's own text grounds on the original: {u_report}"
        );
        assert_eq!(
            t_report["all_evidence_grounded"], u_report["all_evidence_grounded"],
            "{name}: the tagged document answers as the original does"
        );
        assert_eq!(
            t_report["checks"], u_report["checks"],
            "{name}: the per-claim outcome — status, evidence, match method — is the same"
        );
        assert_eq!(
            t_report["checks"][0]["status"], "grounded",
            "{name}: {t_report}"
        );
        let mut u_rest = u_report.clone();
        let mut t_rest = t_report.clone();
        for r in [&mut u_rest, &mut t_rest] {
            assert!(
                r.as_object_mut()
                    .unwrap()
                    .remove("document_fingerprint")
                    .is_some(),
                "{name}: the report names the grounding file it read"
            );
        }
        assert_eq!(
            t_rest, u_rest,
            "{name}: apart from the fingerprint of the grounding file, the two reports are one"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// -------------------------------------------------------------------------------------------
// 2. The projections are the original's
// -------------------------------------------------------------------------------------------

/// **The Markdown, the HTML and the grounding artifact of a tagged document are its untagged
/// original's** (§S3, second acceptance; scope §4.3 pinned).
///
/// # The comparison, and why it is this one
///
/// The scope says *byte for byte apart from the assurance block*, and the assurance block lives
/// on the representation — none of the three projections carries one. What each projection does
/// carry that names its input rather than describes it is a digest: `source_sha256` and
/// `representation_sha256` on the Markdown and HTML artifacts, `source.sha256` on the grounding
/// artifact. Those differ by construction — the tagged PDF is different bytes, and its
/// representation carries a different assurance block, which is exactly the clause the scope
/// exempts — so the comparison is the canonical JSON with those members deleted, and nothing
/// else: the text, the anchor map, the coverage census, the identity (the same profile hash,
/// because the same build), the elements, spans, pages and producer are compared byte for byte.
/// The test also asserts the deleted digests really differ, so it cannot be comparing a document
/// to itself.
#[test]
fn the_projections_of_a_tagged_document_are_its_originals() {
    for t in twins() {
        let name = t.name;

        let (t_md, u_md) = (markdown(&t.tagged), markdown(&t.untagged));
        assert_ne!(
            t_md.source_sha256, u_md.source_sha256,
            "{name}: two documents"
        );
        assert_ne!(t_md.representation_sha256, u_md.representation_sha256);
        assert_eq!(t_md.markdown, u_md.markdown, "{name}: the Markdown text");
        assert_eq!(
            canonical_without(
                &t_md.to_canonical_bytes().unwrap(),
                &[&["source_sha256"], &["representation_sha256"]]
            ),
            canonical_without(
                &u_md.to_canonical_bytes().unwrap(),
                &[&["source_sha256"], &["representation_sha256"]]
            ),
            "{name}: the Markdown artifact, apart from the two digests that name its input"
        );

        let (t_html, u_html) = (html(&t.tagged), html(&t.untagged));
        assert_ne!(t_html.source_sha256, u_html.source_sha256);
        assert_eq!(t_html.html, u_html.html, "{name}: the HTML text");
        assert_eq!(
            canonical_without(
                &t_html.to_canonical_bytes().unwrap(),
                &[&["source_sha256"], &["representation_sha256"]]
            ),
            canonical_without(
                &u_html.to_canonical_bytes().unwrap(),
                &[&["source_sha256"], &["representation_sha256"]]
            ),
            "{name}: the HTML artifact, apart from the two digests that name its input"
        );

        let (t_g, u_g) = (ground(&t.tagged), ground(&t.untagged));
        assert_ne!(t_g.source.source.sha256, u_g.source.source.sha256);
        assert_eq!(
            t_g.source.elements, u_g.source.elements,
            "{name}: the grounding elements"
        );
        assert_eq!(t_g.source.spans, u_g.source.spans, "{name}: the spans");
        assert_eq!(
            canonical_without(
                &ethos_parser_grounding::to_canonical_bytes(&t_g.source).unwrap(),
                &[&["source", "sha256"]]
            ),
            canonical_without(
                &ethos_parser_grounding::to_canonical_bytes(&u_g.source).unwrap(),
                &[&["source", "sha256"]]
            ),
            "{name}: the grounding artifact, apart from the digest of its source"
        );
        assert_eq!(
            (
                t_g.omission.nodes_omitted,
                t_g.spans_withheld,
                t_g.elements_omitted
            ),
            (
                u_g.omission.nodes_omitted,
                u_g.spans_withheld,
                u_g.elements_omitted
            ),
            "{name}: and what the projection left out"
        );
    }
}

// -------------------------------------------------------------------------------------------
// 3. Declared on the artifact, and not as the author's
// -------------------------------------------------------------------------------------------

/// **An engine-written tree is declared on the artifact and not as the author's** (§S3, third
/// acceptance; scope §4.2).
///
/// S2 holds this on the extract artifact. This holds it on the **representation**, which is what
/// `markdown`, `html` and `ground` read and whose assurance `represent.rs` rebuilds from the
/// extract's — a declaration that did not survive that rebuild would reach no consumer. The
/// tagged representation carries `structure-tree-engine-written`, document-scoped, and not
/// `untagged-structure-tree-absent`; the untagged original carries the reverse; every other
/// limitation is the source's.
#[test]
fn an_engine_written_tree_is_declared_on_the_artifact_and_not_as_authors() {
    for t in twins() {
        let name = t.name;
        let tagged = codes_of(&t.tagged);
        let untagged = codes_of(&t.untagged);
        assert!(
            tagged.contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
            "{name}: {tagged:?}"
        );
        assert!(
            !tagged.contains(&codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
            "{name}: {tagged:?}"
        );
        assert!(
            untagged.contains(&codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
            "{name}: {untagged:?}"
        );
        assert!(
            !untagged.contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
            "{name}: {untagged:?}"
        );
        let declaration = t
            .tagged
            .payload()
            .assurance
            .limitations
            .iter()
            .find(|l| l.code == codes::STRUCTURE_TREE_ENGINE_WRITTEN)
            .unwrap();
        assert_eq!(
            declaration.scope,
            LimitationScope::Document,
            "{name}: the declaration is document-scoped"
        );
        assert!(
            declaration.detail.contains("gutter-columns-v3"),
            "{name}: the declaration names the rule: {}",
            declaration.detail
        );

        let others = |repr: &DocumentRepresentation| -> Vec<Limitation> {
            repr.payload()
                .assurance
                .limitations
                .iter()
                .filter(|l| {
                    l.code != codes::STRUCTURE_TREE_ENGINE_WRITTEN
                        && l.code != codes::UNTAGGED_STRUCTURE_TREE_ABSENT
                })
                .cloned()
                .collect()
        };
        assert_eq!(
            others(&t.tagged),
            others(&t.untagged),
            "{name}: every other declaration is the source's"
        );
    }
}

// -------------------------------------------------------------------------------------------
// 4. Stripping the attribute launders the tag
// -------------------------------------------------------------------------------------------

/// Remove `/A` from every structure element in the document, wherever the element sits, and
/// return how many were removed.
fn strip_owner_attributes(doc: &mut lopdf::Document) -> usize {
    fn walk(object: &mut Object) -> usize {
        match object {
            Object::Dictionary(d) => {
                let mut n = 0;
                if d.get(b"Type").ok() == Some(&Object::Name(b"StructElem".to_vec()))
                    && d.remove(b"A").is_some()
                {
                    n += 1;
                }
                n + d.iter_mut().map(|(_, v)| walk(v)).sum::<usize>()
            }
            Object::Array(items) => items.iter_mut().map(walk).sum(),
            Object::Stream(s) => s.dict.iter_mut().map(|(_, v)| walk(v)).sum(),
            _ => 0,
        }
    }
    doc.objects.values_mut().map(walk).sum()
}

/// **The writer's output with every `/A` removed reads back as an author's tree** (§S3, fourth
/// acceptance) — `derivation: extracted` on every run, and no engine-written tree declared.
///
/// This is the failure decision #23 names, held as a test: the guarantee is engine-local, the
/// attribute is the whole of what makes a written `/Div` this engine's rather than an author's,
/// and a consumer that drops or ignores it reads author structure (scope §9, limitation 1;
/// §7.2 names the previous release as such a consumer). Nothing here is a defect to repair. It is
/// held so that the day it stops being reproducible — a reader that recognises its own tree by
/// some other mark, or one that stops reading `/A` — is noticed rather than discovered.
///
/// The consequence is asserted too, on the fixture whose blocks span several baselines: read as
/// the author's, the `/Div` licenses the declared join across its line breaks, and the laundered
/// document projects a sentence the untagged original never forms.
#[test]
fn stripping_the_attribute_launders_the_tag() {
    for t in twins() {
        let name = t.name;
        let mut doc = lopdf::Document::load_mem(&t.tagged_pdf).expect("lopdf loads");
        let removed = strip_owner_attributes(&mut doc);
        assert!(
            removed >= 2,
            "{name}: the /Document and at least one /Div: {removed}"
        );
        let mut laundered = Vec::new();
        doc.save_to(&mut laundered).expect("saves");
        assert!(
            lopdf::Document::load_mem(&laundered)
                .expect("reloads")
                .objects
                .values()
                .all(|o| !matches!(o, Object::Dictionary(d) if d.has(b"A"))),
            "{name}: no /A survives"
        );

        let repr = represent(&laundered);
        let nodes = &repr.payload().nodes;
        assert!(!nodes.is_empty(), "{name}");
        for (i, node) in nodes.iter().enumerate() {
            if node.kind != NodeKind::TextRun {
                continue;
            }
            match node.structural_locator.as_ref() {
                Some(StructuralLocator::PdfTagged(l)) => {
                    assert_eq!(
                        l.derivation,
                        DerivationClass::Extracted,
                        "{name} node {i}: without the attribute the reader sees author structure"
                    );
                    assert_eq!(l.role_path, ["Document", "Div"], "{name} node {i}");
                }
                other => panic!("{name} node {i} `{}` is not bound: {other:?}", node.text),
            }
        }
        let laundered_codes = codes_of(&repr);
        assert!(
            !laundered_codes.contains(&codes::STRUCTURE_TREE_ENGINE_WRITTEN),
            "{name}: no engine-written tree is declared: {laundered_codes:?}"
        );
        assert!(
            !laundered_codes.contains(&codes::UNTAGGED_STRUCTURE_TREE_ABSENT),
            "{name}: a tree is present, so the untagged declaration is not made either"
        );

        if name == "leading-gap-two-blocks" {
            let original = markdown(&t.untagged).markdown;
            let laundered_md = markdown(&repr).markdown;
            assert_ne!(
                laundered_md, original,
                "{name}: read as an author's, the /Div licenses joins the original never made"
            );
            assert!(
                laundered_md.contains("Water finds its level and stone keeps its shape"),
                "{name}: the declared join across the block's line breaks: {laundered_md:?}"
            );
            assert!(
                !original.contains("Water finds its level and stone keeps its shape"),
                "{name}: a sentence the untagged original never forms: {original:?}"
            );
        }
    }
}
