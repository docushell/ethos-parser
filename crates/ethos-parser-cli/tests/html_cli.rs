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

//! `ethos-parser html` — the second projection, under the same four laws (v1.1-S4).
//!
//! # What this suite has to establish that `markdown_cli.rs` does not
//!
//! The laws are the same laws, so most of this file is the same sweep over the same fixtures. The
//! part that is *not* a repeat is the reason `ethos.html.v1` exists at all:
//!
//! **GFM cannot say `rowspan`.** v1.1-S2 had to expand every merged cell into the slots it covered
//! and count what that cost, because a table that reads `| North | merged span |  |` has quietly
//! become a table with an extra empty cell. HTML says it — one `<td colspan="2">`, and the covered
//! slot emits nothing — so on the same fixture the two artifacts carry different erasure censuses,
//! and [`the_two_projections_disagree_about_the_merge_and_say_so`] is that difference asserted
//! rather than described.
//!
//! [`an_html_quote_verifies_end_to_end`] is the capability proof `capabilities.html` names. It is
//! the same sentence v1.1-S1's golden makes, in the second projection: a quote lifted out of a
//! `source` segment grounds through the pinned Ethos CLI, and one that carries a tag does not.
//!
//! # The verifier is required, and its absence is a failure
//!
//! Same posture as every other golden here: a suite that quietly passed on a machine with no
//! verifier would be asserting nothing at all.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

mod common;

// -------------------------------------------------------------------------------------------
// Harness
// -------------------------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

/// The verifier, resolved exactly as the oracle and Markdown harnesses resolve it.
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
         FAILURE, never a skip (docs/04-ARCHITECTURE.md §4)."
    );
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-v11-html-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn engine_fixture(name: &str) -> PathBuf {
    let p = repo_root().join(format!("fixtures/engine/{name}/document.pdf"));
    assert!(p.is_file(), "fixture missing: {}", p.display());
    p
}

/// The conformance corpus, resolved through `fixtures/manifest.json`.
///
/// **The manifest declares the root and the `ETHOS_FIXTURES` override; this harness used to
/// hardcode `../ethos/fixtures` and ignore both** (v2-S10.2). That made it green only where the
/// corpus happens to be a sibling checkout of this one — and CI is not such a place: the workflow
/// checks the verifier out at `ethos-oracle/` and points `ETHOS_FIXTURES` there, so every
/// `conformance` call here resolved to a path that does not exist. `classify_cli.rs`,
/// `diagnostics.rs`, `grounding.rs` and `library_surface.rs` have always read the declaration.
fn conformance(rel: &str) -> PathBuf {
    let m: Value = serde_json::from_slice(
        &std::fs::read(repo_root().join("fixtures/manifest.json")).expect("manifest"),
    )
    .expect("valid JSON");
    let decl = &m["roots"]["conformance"];
    let root = match decl["env"].as_str().and_then(|e| std::env::var(e).ok()) {
        Some(v) => PathBuf::from(v),
        None => repo_root().join(decl["default"].as_str().expect("default")),
    };
    let p = root.join(rel);
    assert!(p.is_file(), "fixture missing: {}", p.display());
    p
}

fn extract_to(dir: &Path, pdf: &Path) -> PathBuf {
    let out = engine(&["extract", pdf.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "extract must succeed");
    let p = dir.join("repr.json");
    std::fs::write(&p, &out.stdout).expect("write representation");
    p
}

/// The parsed `ethos.html.v1` for a representation.
fn html_of(repr: &Path) -> Value {
    let out = engine(&["html", repr.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "html must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("canonical JSON")
}

/// The parsed `ethos.markdown.v1`, for the tests that compare the two projections.
fn markdown_of(repr: &Path) -> Value {
    let out = engine(&["markdown", repr.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "markdown must succeed");
    serde_json::from_slice(&out.stdout).expect("canonical JSON")
}

/// Every fixture both projections are swept over.
fn sweep() -> Vec<PathBuf> {
    vec![
        conformance("synthetic/simple-text/document.pdf"),
        conformance("synthetic/two-lines/document.pdf"),
        conformance("synthetic/two-columns/document.pdf"),
        conformance("synthetic/heading-export/document.pdf"),
        conformance("synthetic/hyphenated-line-break/document.pdf"),
        conformance("synthetic/table-regular-grid/document.pdf"),
        engine_fixture("ruled-table-grid"),
        engine_fixture("markdown-table-cells"),
        engine_fixture("markdown-hyphen-break"),
        engine_fixture("tagged-list-items"),
        engine_fixture("ruled-table-overlap"),
        engine_fixture("form-field-value"),
    ]
}

// -------------------------------------------------------------------------------------------
// The artifact
// -------------------------------------------------------------------------------------------

/// The shape, on the smallest real document there is.
#[test]
fn html_on_simple_text_is_the_artifact_the_scope_document_describes() {
    let dir = scratch("shape");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let a = html_of(&repr);

    assert_eq!(a["artifact_type"], "ethos.html.v1");
    assert_eq!(a["schema_version"], "1.0.0");
    assert_eq!(a["html_rule"], "html-blocks-v10");
    assert_eq!(a["html"], "<p>Hello Ethos</p>\n");

    for key in [
        "parser_version",
        "profile_sha256",
        "source_sha256",
        "representation_sha256",
    ] {
        assert!(a.get(key).is_some(), "missing `{key}`");
    }

    // `<p>` · `Hello Ethos` · `</p>\n`. The tags are bytes this exporter invented and say so.
    let segs = a["anchor_map"]["segments"].as_array().expect("segments");
    let got: Vec<(&str, u64, u64)> = segs
        .iter()
        .map(|s| {
            (
                s["kind"].as_str().unwrap(),
                s["start"].as_u64().unwrap(),
                s["end"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![("syntax", 0, 3), ("source", 3, 14), ("syntax", 14, 19)]
    );
    assert!(
        segs[0].get("node_ids").is_none(),
        "syntax names no node, so the key is absent rather than an empty array"
    );
}

/// **Law 2, on the real fixtures.** Every byte belongs to exactly one segment.
#[test]
fn the_map_tiles_the_html_on_every_fixture() {
    let dir = scratch("tiles");
    for (i, path) in sweep().iter().enumerate() {
        let rel = path.display().to_string();
        let sub = dir.join(format!("f{i}"));
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(&sub, path);
        let a = html_of(&repr);
        let html = a["html"].as_str().expect("html is a string");
        let segs = a["anchor_map"]["segments"].as_array().expect("segments");

        let mut cursor = 0usize;
        let mut rebuilt = String::new();
        for s in segs {
            let start = s["start"].as_u64().unwrap() as usize;
            let end = s["end"].as_u64().unwrap() as usize;
            assert_eq!(start, cursor, "{rel}: gap or overlap at {start}");
            assert!(end > start, "{rel}: empty segment");
            let kind = s["kind"].as_str().unwrap();
            assert!(
                kind == "source" || kind == "syntax",
                "{rel}: law 3 admits exactly two kinds, not `{kind}`"
            );
            rebuilt.push_str(&html[start..end]);
            cursor = end;
        }
        assert_eq!(cursor, html.len(), "{rel}: the map must cover the string");
        assert_eq!(rebuilt, html, "{rel}: the segments must reconstruct it");
    }
}

/// **Law 4, on the real fixtures.** The census balances and every bucket is named.
#[test]
fn the_coverage_census_balances_on_every_fixture() {
    let dir = scratch("coverage");
    for (i, path) in sweep().iter().enumerate() {
        let rel = path.display().to_string();
        let sub = dir.join(format!("c{i}"));
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(&sub, path);
        let a = html_of(&repr);
        let c = &a["coverage"];

        let total = c["source_chars_in_representation"].as_u64().unwrap();
        let emitted = c["source_chars_emitted"].as_u64().unwrap();
        let dropped = c["source_chars_dropped"].as_u64().unwrap();
        assert_eq!(emitted + dropped, total, "{rel}: the census must balance");

        let sum: u64 = c["dropped"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["chars"].as_u64().unwrap())
            .sum();
        assert_eq!(sum, dropped, "{rel}: the buckets must sum to the total");
        for b in c["dropped"].as_array().unwrap() {
            assert!(
                !b["code"]
                    .as_str()
                    .expect("every bucket is named")
                    .is_empty(),
                "{rel}: an unnamed bucket is not a disclosure"
            );
        }
        assert!(
            c["structural_erasures"].is_array(),
            "{rel}: `structural_erasures` must be present, even when empty"
        );
    }
}

/// **Both projections account for exactly the same characters, on every fixture.**
///
/// The strongest statement this suite makes about the pair. Two artifacts of one document that
/// disagreed about how many characters it contains would leave a consumer no way to tell which
/// had lost something, and the shared `census` is what stops that being possible.
#[test]
fn the_two_projections_agree_about_every_character() {
    let dir = scratch("agree");
    for (i, path) in sweep().iter().enumerate() {
        let rel = path.display().to_string();
        let sub = dir.join(format!("a{i}"));
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(&sub, path);
        let (h, m) = (html_of(&repr), markdown_of(&repr));

        for field in [
            "source_chars_in_representation",
            "source_chars_emitted",
            "source_chars_dropped",
        ] {
            assert_eq!(
                h["coverage"][field], m["coverage"][field],
                "{rel}: the two projections disagree about `{field}`"
            );
        }
        assert_eq!(
            h["coverage"]["dropped"], m["coverage"]["dropped"],
            "{rel}: and they must drop the same characters for the same named reasons"
        );
    }
}

/// **The reason this artifact exists.** HTML carries the merge; GFM had to flatten and count it.
#[test]
fn the_two_projections_disagree_about_the_merge_and_say_so() {
    let dir = scratch("merge");
    let repr = extract_to(&dir, &engine_fixture("markdown-table-cells"));
    let (h, m) = (html_of(&repr), markdown_of(&repr));

    let html = h["html"].as_str().expect("html");
    assert!(
        html.contains("<td colspan=\"2\">merged span</td>"),
        "the merged cell is ONE cell that says how wide it is: {html}"
    );
    assert_eq!(
        html.matches("merged span").count(),
        1,
        "and its text appears once — the slot it covers emits nothing at all"
    );
    assert!(
        !html.contains("<th"),
        "no header is invented: no detector reads `/TH`"
    );

    let code_of = |a: &Value, code: &str| -> u64 {
        a["coverage"]["structural_erasures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["code"] == code)
            .and_then(|e| e["count"].as_u64())
            .unwrap_or(0)
    };

    assert_eq!(
        code_of(&m, "gfm-span-slots-unrepresentable-v1"),
        1,
        "the Markdown artifact still declares the slot the merge cost it"
    );
    assert_eq!(
        code_of(&h, "gfm-span-slots-unrepresentable-v1"),
        0,
        "and the HTML artifact does NOT — it did not commit that erasure, and a code copied over \
         for symmetry would read as a disclosure while disclosing nothing"
    );
    assert_eq!(
        code_of(&m, "gfm-row-zero-separator-v1"),
        1,
        "GFM's delimiter row asserts a header the document never declared"
    );
    assert_eq!(
        code_of(&h, "gfm-row-zero-separator-v1"),
        0,
        "HTML asserts none, because every cell is a `<td>`"
    );
}

/// The one erasure HTML **does** commit, declared rather than quietly inherited.
///
/// Two sibling `/LI`s have identical role paths, so nothing in the representation distinguishes
/// "the rest of this item" from "the next item". Both projections guess, both guess the same way,
/// and both say how often.
#[test]
fn the_list_item_join_is_declared_on_both_projections() {
    let dir = scratch("listjoin");
    let repr = extract_to(&dir, &engine_fixture("tagged-list-items"));
    let (h, m) = (html_of(&repr), markdown_of(&repr));

    let count = |a: &Value| -> u64 {
        a["coverage"]["structural_erasures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["code"] == "gfm-list-item-run-joins-v1")
            .and_then(|e| e["count"].as_u64())
            .unwrap_or(0)
    };
    assert_eq!(count(&m), 1);
    assert_eq!(
        count(&h),
        1,
        "HTML joins the same two runs, so it owes the same count: {:?}",
        h["coverage"]["structural_erasures"]
    );

    // And the tree's nesting is real structure rather than an indent.
    let html = h["html"].as_str().unwrap();
    assert!(
        html.contains("<ul>"),
        "a tagged list becomes a list: {html}"
    );
    assert_eq!(
        html.matches("<ul>").count(),
        html.matches("</ul>").count(),
        "every list this exporter opened, it closed: {html}"
    );
    assert_eq!(html.matches("<li>").count(), html.matches("</li>").count());
}

/// An untagged document grows no list here either. Checklist L29 and its neighbours, in HTML.
#[test]
fn nothing_is_inferred_from_a_font_or_a_glyph() {
    let dir = scratch("infer");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let html = html_of(&repr)["html"].as_str().unwrap().to_string();
    for tag in [
        "<ul>", "<li>", "<h1>", "<h2>", "<table>", "class=", "style=",
    ] {
        assert!(
            !html.contains(tag),
            "`{tag}` appears on an untagged one-run document: {html}"
        );
    }
}

/// A form field's value is dropped and counted, never pasted into the body — as in Markdown.
#[test]
fn a_form_field_value_is_a_named_bucket_rather_than_body_text() {
    let dir = scratch("field");
    let repr = extract_to(&dir, &engine_fixture("form-field-value"));
    let a = html_of(&repr);

    assert!(
        !a["html"].as_str().unwrap().contains("Wendell"),
        "the field value must not appear in the HTML body"
    );
    let b = a["coverage"]["dropped"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["code"] == "form-field-values-not-projected-v1")
        .expect("and it is declared, with a count");
    assert!(b["chars"].as_u64().unwrap() > 0);
}

/// Double-run byte identity — standing rule 6.
#[test]
fn two_runs_are_byte_identical() {
    let dir = scratch("identity");
    let repr = extract_to(&dir, &conformance("synthetic/two-lines/document.pdf"));
    let a = engine(&["html", repr.to_str().unwrap()]);
    let b = engine(&["html", repr.to_str().unwrap()]);
    assert_eq!(a.stdout, b.stdout, "two runs must agree byte for byte");
}

/// **Law 1, asserted rather than assumed.** No CLI path prints HTML without a map.
#[test]
fn no_cli_path_emits_html_without_its_map() {
    let help = engine(&["html", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    let text = String::from_utf8_lossy(&help.stdout);

    // The OPTIONS block, not the prose: the subcommand's own documentation says the words "no
    // `--html-only`", so searching the whole output would match this test's own subject.
    let options = text.split("Options:").nth(1).unwrap_or("");
    for flag in ["--html-only", "--no-map", "--raw", "--bare"] {
        assert!(
            !options.contains(flag),
            "`{flag}` exists on `ethos-parser html`, and law 1 says the map is not optional"
        );
    }

    // And the artifact itself cannot be constructed without one: every emitted artifact carries
    // a non-empty map for a non-empty string.
    let dir = scratch("law1");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let a = html_of(&repr);
    assert!(!a["html"].as_str().unwrap().is_empty());
    assert!(!a["anchor_map"]["segments"].as_array().unwrap().is_empty());
}

/// A representation whose payload does not hash to its declared digest is refused.
#[test]
fn a_tampered_representation_is_refused() {
    let dir = scratch("tamper");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let mut doc: Value = serde_json::from_slice(&std::fs::read(&repr).unwrap()).expect("json");
    doc["representation"]["nodes"][0]["text"] = Value::String("Tampered".into());
    let bad = dir.join("bad.json");
    std::fs::write(&bad, serde_json::to_vec(&doc).unwrap()).expect("write");

    let out = engine(&["html", bad.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "projecting a record that does not hash to its own digest would launder the disagreement"
    );
}

// -------------------------------------------------------------------------------------------
// The golden — the capability proof
// -------------------------------------------------------------------------------------------

/// **An HTML quote is either invertible to canonical evidence, or explicitly unquotable.**
///
/// v1.1-S1's sentence, in the second projection, with the pinned Ethos CLI deciding. The fixture
/// is `markdown-two-blocks` because its font declares real ink metrics, so both runs reach
/// `ethos.grounding.v1` as elements a verifier can actually find — without that the golden would
/// watch the verifier find nothing and refuse both halves, which proves nothing.
///
/// The refused quote is the one HTML makes available and Markdown does not: **`block</p>\n<p>`**
/// is real text in the `.html`, and the page drew none of it. From the file alone nothing marks
/// those bytes as invented; the map does, mechanically, and the verifier — which knows nothing
/// about HTML and reads only the grounding artifact — agrees by failing to ground it.
///
/// The reason is pinned, not just the verdict: `element_not_found` would mean the citation pointed
/// at nothing and the test would pass without the Anchor Map having demonstrated anything.
#[test]
fn an_html_quote_verifies_end_to_end() {
    let verifier = ethos_binary();
    assert!(
        verifier.is_file(),
        "the pinned Ethos CLI is a test-time dependency of this golden: {}",
        verifier.display()
    );

    let dir = scratch("golden");
    let repr = extract_to(&dir, &engine_fixture("markdown-two-blocks"));

    let grounded_out = engine(&["ground", repr.to_str().unwrap()]);
    assert_eq!(grounded_out.status.code(), Some(0), "ground must succeed");
    let grounding = dir.join("grounding.json");
    std::fs::write(&grounding, &grounded_out.stdout).expect("write grounding");
    let fingerprint = format!(
        "sha256:{}",
        ethos_parser_core::sha256_hex_bytes(&std::fs::read(&grounding).unwrap())
    );

    let a = html_of(&repr);
    let html = a["html"].as_str().expect("html").to_string();
    let segs = a["anchor_map"]["segments"].as_array().expect("segments");

    // A quote taken from a SOURCE segment: bytes the document actually drew.
    let source = segs
        .iter()
        .find(|s| s["kind"] == "source")
        .expect("at least one source segment");
    let (ss, se) = (
        source["start"].as_u64().unwrap() as usize,
        source["end"].as_u64().unwrap() as usize,
    );
    let source_quote = html[ss..se].to_string();
    assert!(
        !source_quote.trim().is_empty(),
        "the quote must be substantive"
    );

    // A quote that carries TAGS: real text in the file, drawn by nothing.
    let tagged = {
        let at = html.find("</p>").expect("a closing tag between the blocks");
        html[at.saturating_sub(5)..(at + 8).min(html.len())].to_string()
    };
    assert!(
        tagged.contains("</p>"),
        "the spanning quote must actually include the invented bytes: {tagged:?}"
    );
    assert!(
        !html[ss..se].contains(&tagged),
        "and it must not be wholly inside the source segment"
    );

    let grounding_doc: Value =
        serde_json::from_slice(&std::fs::read(&grounding).unwrap()).expect("grounding JSON");
    let element_id = grounding_doc["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .find(|e| e["text"].as_str() == Some(source_quote.as_str()))
        .and_then(|e| e["id"].as_str())
        .expect("the source segment's text is an element of the grounding artifact")
        .to_string();

    let write_claims = |name: &str, text: &str| -> PathBuf {
        let p = dir.join(name);
        let body = serde_json::json!({
            "document_fingerprint": fingerprint,
            "claims": [{
                "kind": "quote",
                "text": text,
                "citation": { "page": "p1", "element_id": element_id }
            }]
        });
        std::fs::write(&p, serde_json::to_vec_pretty(&body).unwrap()).expect("write claims");
        p
    };

    let verify = |claims: &Path| -> Value {
        let out = engine(&[
            "verify",
            grounding.to_str().unwrap(),
            "--citations",
            claims.to_str().unwrap(),
        ]);
        assert_eq!(
            out.status.code(),
            Some(0),
            "the relay itself succeeds whatever the verdict: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        common::verification_report(&out.stdout)
    };

    // **The verifier's own field, read and not re-derived** (`docs/07-VERIFY-BOUNDARY.md`).
    let grounded_of = |report: &Value| -> bool {
        let v = report
            .get("all_evidence_grounded")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| panic!("the report must carry `all_evidence_grounded`: {report}"));
        let statuses: Vec<&str> = report["checks"]
            .as_array()
            .expect("checks")
            .iter()
            .filter_map(|c| c["status"].as_str())
            .collect();
        assert!(
            !statuses.is_empty(),
            "the report must judge the claim: {report}"
        );
        assert_eq!(
            v,
            statuses.iter().all(|s| *s == "grounded"),
            "the summary and the per-claim statuses must agree: {report}"
        );
        v
    };

    assert!(
        grounded_of(&verify(&write_claims(
            "from-source.html.json",
            &source_quote
        ))),
        "a quote taken from a SOURCE segment is text the document drew, so it must ground. \
         Quote: {source_quote:?}"
    );

    let tagged_report = verify(&write_claims("from-tags.html.json", &tagged));
    let reason = tagged_report["checks"][0]["reason"].as_str().unwrap_or("");
    assert_eq!(
        reason, "text_mismatch",
        "the tagged quote must be refused because the TEXT is not on the page, not because the \
         citation missed — `element_not_found` would make this test pass without the Anchor Map \
         having demonstrated anything: {tagged_report}"
    );
    assert!(
        !grounded_of(&tagged_report),
        "a quote carrying a tag is text this exporter invented. The document drew a line break; \
         it did not draw `</p>`. Quote: {tagged:?}"
    );
}

/// The joined hyphen word is **not** citable on this artifact either, and the halves still are.
///
/// The S3 trade, restated for the second projection so it cannot quietly hold on one artifact and
/// not the other. `recalculated` is never asserted grounded — that is the cost of the cosmetic,
/// paid identically here.
#[test]
fn the_joined_word_is_not_citable_on_the_html_either() {
    let dir = scratch("hyphen");
    let repr = extract_to(&dir, &engine_fixture("markdown-hyphen-break"));
    let a = html_of(&repr);

    assert_eq!(
        a["html"].as_str().unwrap(),
        "<p>The rate may be recalculated at closing</p>\n"
    );

    let doc: Value =
        serde_json::from_slice(&std::fs::read(&repr).unwrap()).expect("representation");
    let nodes = doc["representation"]["nodes"].as_array().expect("nodes");
    assert!(
        !nodes
            .iter()
            .any(|n| n["text"].as_str() == Some("recalculated")),
        "no node holds the joined word, so it is READABLE on both artifacts and citable on \
         neither — the map names the two strings that are"
    );

    let joined = a["anchor_map"]["segments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "source")
        .expect("a source segment");
    assert_eq!(
        joined["node_ids"].as_array().map(Vec::len),
        Some(2),
        "and the HTML map names both runs, exactly as the Markdown one does"
    );
}

// -------------------------------------------------------------------------------------------

/// **An EPUB's own heading element reaches the HTML projection too** (v2.2-S0).
///
/// `html.rs` and `markdown.rs` share one `heading_level`, so this is the same fact in the other
/// syntax — and asserting it in both is the point rather than duplication: the two rule ids are
/// separate precisely so they *can* move apart, and only a test in each says they did not.
#[test]
fn an_epubs_own_heading_element_projects_as_an_h_element() {
    let dir = scratch("epub-heading-html");
    let repr = extract_to(
        &dir,
        &repo_root().join("fixtures/office/book-spine/book.epub"),
    );
    let a = html_of(&repr);
    let html = a["html"].as_str().expect("html string");

    assert!(
        html.contains("<h1>Evidence, not extraction.</h1>"),
        "the `<h1>` the publication declares must project as `<h1>`:\n{html}"
    );
    assert!(
        !html.contains("<h1>Rows &amp; columns"),
        "a `<p>` must NOT become a heading:\n{html}"
    );
}

/// **An ODT's own `text:outline-level` reaches the HTML projection** (v2.4).
///
/// The end-to-end half of `html::tests::every_odf_outline_level_projects_as_its_own_h_element`:
/// that one builds the nodes, this one runs a real package through the reader, the wire and the
/// projection. Both are needed — the unit test reaches `h2`..`h6`, which no package in the corpus
/// carries, and this one proves the level survives the two hops between the element and the tag.
#[test]
fn an_odts_own_outline_level_projects_as_an_h_element() {
    let dir = scratch("odt-heading-html");
    let repr = extract_to(
        &dir,
        &repo_root().join("fixtures/office/text-paragraphs/document.odt"),
    );
    let a = html_of(&repr);
    let html = a["html"].as_str().expect("html string");

    assert!(
        html.contains("<h1>Evidence, not extraction.</h1>"),
        "the `<text:h text:outline-level=\"1\">` the document declares must project as \
         `<h1>`:\n{html}"
    );
    assert_eq!(
        html.matches("<h").count(),
        1,
        "one heading and no other: five `<text:p>` follow it, and a `<text:p>` is not a \
         heading:\n{html}"
    );
    assert_eq!(a["html_rule"], "html-blocks-v10");
    // **The negative, and it is the one that earns its place.** A heading that projected at its
    // own depth flattened nothing — the only guard that a predicate which forgot to short-circuit
    // on a level it CAN write would fail.
    assert_eq!(
        a["coverage"]["structural_erasures"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "nothing was flattened: {}",
        a["coverage"]["structural_erasures"]
    );
}

/// **An ODP `<text:h>` that states no level projects as a paragraph, not as `<h1>`.**
///
/// The refusal, end to end. The presentation fixture writes a bare `<text:h>`: it is a heading,
/// and the level it displays at comes from an outline style in `styles.xml`, which the reader
/// declares it did not open. `<h1>` here would be level one on no evidence, which is the whole of
/// what `OfficeParagraphAttributes::outline_level`'s `None` means.
#[test]
fn an_odp_heading_with_no_stated_level_projects_as_a_paragraph() {
    let dir = scratch("odp-heading-html");
    let repr = extract_to(
        &dir,
        &repo_root().join("fixtures/office/presentation-pages/presentation.odp"),
    );
    let a = html_of(&repr);
    let html = a["html"].as_str().expect("html string");

    assert!(
        html.contains("<p>Evidence, not extraction.</p>"),
        "a `<text:h>` with no `text:outline-level` has no depth to project:\n{html}"
    );
    assert!(
        !html.contains("<h"),
        "no heading element anywhere: the deck states the fact of a heading and never its \
         level:\n{html}"
    );
    // **Kept at the same count as the Markdown artifact's**, because this projection commits the
    // same flattening. Two artifacts of one document that disagreed about how many headings it
    // lost would both be wrong to cite.
    assert_eq!(
        *a["coverage"]["structural_erasures"]
            .as_array()
            .expect("an array"),
        vec![serde_json::json!({"code": "heading-level-unresolved-v1", "count": 2})],
        "two bare `<text:h>`, one count each, and no other code"
    );
}

/// **The undeclared join, in HTML** (v2.2-S5).
///
/// The twin of `runs_the_document_declared_nothing_about_join_along_one_baseline`. Both
/// projections call the same clauses out of `crate::markdown`, and a document that reads as one
/// block there must read as one `<p>` here — so this fixture is the tripwire for both.
#[test]
fn runs_the_document_declared_nothing_about_join_into_one_paragraph() {
    let dir = scratch("undeclared-join-html");
    let repr = extract_to(&dir, &engine_fixture("untagged-shredded-line"));
    let a = html_of(&repr);
    assert_eq!(
        a["html"].as_str().unwrap(),
        "<p>Yarrow</p>\n<p>Separate</p>\n"
    );
}
