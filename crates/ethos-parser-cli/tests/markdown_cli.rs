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

//! `ethos-parser markdown` — the projection, and the one test that justifies the whole version.
//!
//! # The claim v1.1 makes, and how it is checked
//!
//! **A Markdown quote is either invertible to canonical evidence, or explicitly unquotable.**
//!
//! [`a_markdown_quote_verifies_end_to_end`] is that sentence as an executable. It runs the real
//! binaries — `extract`, `ground`, `markdown`, `verify` — lifts a substring out of a **`source`**
//! segment, and watches the pinned Ethos CLI ground it. Then it lifts a substring that touches a
//! **`syntax`** segment and watches the same verifier refuse.
//!
//! The second half is the one that means something. On a two-node document the Markdown contains
//! `First line\n\nSecond line`, and `line\n\nSecond` is **real text in the Markdown that the
//! document never drew**. A consumer reading the `.md` alone cannot tell it from a sentence on the
//! page. With the map it is mechanical, and the verifier — which knows nothing about Markdown and
//! reads only the grounding artifact — agrees by failing to ground it.
//!
//! Nothing here re-derives anything from the verifier's report
//! (`docs/07-VERIFY-BOUNDARY.md`): the report is parsed only to read the field the verifier itself
//! wrote.
//!
//! # The verifier is required, and its absence is a failure
//!
//! Same posture as `verify_relay.rs`: a suite that quietly passed on a machine with no verifier
//! would be asserting nothing at all.

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

/// The verifier, resolved exactly as the oracle and relay harnesses resolve it.
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
        "ethos-parser-v11-markdown-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// An engine-owned CC0 fixture, for the behaviours the Ethos corpus does not carry.
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

/// `extract` a fixture to disk and return the representation path.
fn extract_to(dir: &Path, pdf: &Path) -> PathBuf {
    let out = engine(&["extract", pdf.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "extract must succeed");
    let p = dir.join("repr.json");
    std::fs::write(&p, &out.stdout).expect("write representation");
    p
}

/// The parsed `ethos.markdown.v1` for a representation.
fn markdown_of(repr: &Path) -> Value {
    let out = engine(&["markdown", repr.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "markdown must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("canonical JSON")
}

// -------------------------------------------------------------------------------------------
// The artifact
// -------------------------------------------------------------------------------------------

/// The shape, on the smallest real document there is.
#[test]
fn markdown_on_simple_text_is_the_artifact_the_scope_document_describes() {
    let dir = scratch("shape");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let a = markdown_of(&repr);

    assert_eq!(a["artifact_type"], "ethos.markdown.v1");
    assert_eq!(a["schema_version"], "1.1.0");
    assert_eq!(a["markdown_rule"], "markdown-blocks-v10");
    assert_eq!(a["markdown"], "Hello Ethos\n");

    // Every artifact carries the four identity fields plus both bindings.
    for key in [
        "parser_version",
        "profile_sha256",
        "source_sha256",
        "representation_sha256",
    ] {
        assert!(a.get(key).is_some(), "missing `{key}`");
    }

    let segs = a["anchor_map"]["segments"].as_array().expect("segments");
    assert_eq!(segs.len(), 2);
    assert_eq!(segs[0]["kind"], "source");
    assert_eq!(segs[0]["start"], 0);
    assert_eq!(segs[0]["end"], 11);
    assert!(!segs[0]["node_ids"].as_array().unwrap().is_empty());
    assert_eq!(segs[1]["kind"], "syntax");
    assert!(
        segs[1].get("node_ids").is_none(),
        "syntax names no node, so the key is absent rather than an empty array"
    );
}

/// **Law 2, on the real fixtures.** Every byte belongs to exactly one segment.
///
/// The engine-owned table and list fixtures are in the sweep because v1.1-S2 is where a segment
/// stops being one run per block: a GFM row interleaves chrome and cell text a dozen times a
/// line, and the map has to tile all of it.
#[test]
fn the_map_tiles_the_markdown_on_every_fixture() {
    let dir = scratch("tiles");
    for (i, path) in [
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
    ]
    .iter()
    .enumerate()
    {
        let rel = path.display().to_string();
        let sub = dir.join(format!("f{i}"));
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(&sub, path);
        let a = markdown_of(&repr);
        let md = a["markdown"].as_str().expect("markdown is a string");
        let segs = a["anchor_map"]["segments"].as_array().expect("segments");

        let mut cursor = 0usize;
        let mut rebuilt = String::new();
        for s in segs {
            let start = s["start"].as_u64().unwrap() as usize;
            let end = s["end"].as_u64().unwrap() as usize;
            assert_eq!(start, cursor, "{rel}: gap or overlap at {start}");
            assert!(end > start, "{rel}: empty segment");
            rebuilt.push_str(&md[start..end]);
            cursor = end;
        }
        assert_eq!(
            cursor,
            md.len(),
            "{rel}: the map must cover the whole string"
        );
        assert_eq!(
            rebuilt, md,
            "{rel}: the segments must reconstruct the string"
        );
    }
}

/// **Law 4, on the real fixtures.** The census balances and every bucket is named.
#[test]
fn the_coverage_census_balances_on_every_fixture() {
    let dir = scratch("coverage");
    for (i, path) in [
        conformance("synthetic/simple-text/document.pdf"),
        conformance("synthetic/two-lines/document.pdf"),
        conformance("synthetic/heading-export/document.pdf"),
        // v1.1-S3: the first document whose census carries a character the projection itself
        // removed, so the sweep has to see it balance rather than only the goldens.
        conformance("synthetic/hyphenated-line-break/document.pdf"),
        engine_fixture("markdown-table-cells"),
        engine_fixture("markdown-hyphen-break"),
        engine_fixture("tagged-list-items"),
    ]
    .iter()
    .enumerate()
    {
        let rel = path.display().to_string();
        let sub = dir.join(format!("c{i}"));
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(&sub, path);
        let a = markdown_of(&repr);
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
            let code = b["code"].as_str().expect("every bucket is named");
            assert!(
                !code.is_empty(),
                "{rel}: an unnamed bucket is not a disclosure"
            );
        }

        // v1.1-S2's second census. Always present — an empty array says "nothing was flattened",
        // which is a different statement from an absent key saying "this artifact does not track
        // flattening".
        let erasures = c["structural_erasures"]
            .as_array()
            .unwrap_or_else(|| panic!("{rel}: `structural_erasures` must be present"));
        for e in erasures {
            assert!(
                !e["code"]
                    .as_str()
                    .expect("every erasure is named")
                    .is_empty(),
                "{rel}: an unnamed erasure is not a disclosure"
            );
            assert!(
                e["count"].as_u64().expect("a count, not an adjective") > 0,
                "{rel}: a zero-count erasure is disclosure-shaped noise; the codes are documented \
                 whether or not a document trips them"
            );
        }
    }
}

/// A form field's value is **dropped and counted**, never pasted into the body.
///
/// v1-S4 exists to keep a widget's `/V` distinguishable from page text; a projection that inlined
/// it would undo exactly that, and silently.
#[test]
fn a_form_field_value_is_a_named_bucket_rather_than_body_text() {
    let dir = scratch("field");
    let pdf = repo_root().join("fixtures/engine/form-field-value/document.pdf");
    let repr = extract_to(&dir, &pdf);
    let a = markdown_of(&repr);

    let md = a["markdown"].as_str().unwrap();
    assert!(
        !md.contains("Wendell"),
        "the field value must not appear in the Markdown body: {md:?}"
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
    let a = engine(&["markdown", repr.to_str().unwrap()]);
    let b = engine(&["markdown", repr.to_str().unwrap()]);
    assert_eq!(a.stdout, b.stdout, "two runs must agree byte for byte");
}

/// **Law 1, asserted rather than assumed.** No CLI path prints Markdown without a map.
///
/// A grep over the surface rather than over the implementation: the failure this guards against is
/// somebody adding a convenience flag, and a flag is visible in `--help`.
#[test]
fn no_cli_path_emits_markdown_without_its_map() {
    let help = engine(&["markdown", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    let text = String::from_utf8_lossy(&help.stdout);

    // **The OPTIONS block, not the prose.** The subcommand's own documentation says the words
    // "no `--md-only`", so a naive substring search over the whole help output matches this
    // repository explaining the rule and fails. What matters is whether such a flag is actually
    // declared, and clap lists every declared flag under `Options:`.
    let options = text
        .split("Options:")
        .nth(1)
        .expect("clap always prints an Options block");
    let flags: Vec<&str> = options
        .split_whitespace()
        .filter(|w| w.starts_with("--"))
        .map(|w| w.trim_end_matches(','))
        .collect();
    assert!(
        flags
            .iter()
            .all(|f| matches!(*f, "--help" | "--version" | "--diagnostics" | "--source")),
        "`ethos-parser markdown` grew a flag: {flags:?}. Any option that could suppress the Anchor Map \
         is the one thing this version exists to prevent (docs/history/10-V11-SCOPE.md law 1), so a new \
         flag here is a deliberate decision that needs its own evidence."
    );

    // **`--source` is the one flag admitted here, and this is the evidence the sentence above
    // asks for.** It does not touch what is emitted: it decides where the record comes from — a
    // file, or this process — and [`the_source_path_equals_the_two_step_path`] asserts the two
    // produce the same bytes, map included, on three fixtures and in both projections. Law 1 is
    // about a projection leaving its map behind, and a flag that cannot change the artifact
    // cannot do that. A flag that *suppressed* anything still fails this test, which is why the
    // list is a list and not a wildcard.
    let source_help = options
        .split("--source")
        .nth(1)
        .expect("the flag is declared, so clap documents it");
    assert!(
        source_help.contains("byte-identical"),
        "--source must document the equality that admits it, in the text a caller reads: {source_help:?}"
    );

    // And the artifact itself always carries both halves.
    let dir = scratch("paired");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let a = markdown_of(&repr);
    assert!(a.get("markdown").is_some() && a.get("anchor_map").is_some());
}

/// A representation whose payload does not hash to its declared digest is refused, not projected.
#[test]
fn a_tampered_representation_is_refused() {
    let dir = scratch("tampered");
    let repr = extract_to(&dir, &conformance("synthetic/simple-text/document.pdf"));
    let mut v: Value =
        serde_json::from_slice(&std::fs::read(&repr).unwrap()).expect("representation");
    v["representation"]["nodes"][0]["text"] = Value::String("Goodbye Ethos".into());
    let bad = dir.join("tampered.json");
    std::fs::write(&bad, serde_json::to_vec(&v).unwrap()).unwrap();

    let out = engine(&["markdown", bad.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "projecting a representation that does not hash to its digest would launder the \
         disagreement into a fresh-looking artifact"
    );
    assert!(out.stdout.is_empty(), "and nothing is written on refusal");
}

// -------------------------------------------------------------------------------------------
// v1.1-S2 — the grid, and what it cost
// -------------------------------------------------------------------------------------------

/// **A table is a table, and every character of it appears once.**
///
/// The S1 behaviour this replaces emitted each cell run as its own paragraph and declared the
/// lost grid as `markdown-table-structure-not-projected`. The grid is here now; what GFM cannot
/// hold is declared instead, with a number.
#[test]
fn a_table_projects_as_gfm_and_says_what_the_flattening_cost() {
    let dir = scratch("gfm");
    let repr = extract_to(&dir, &engine_fixture("markdown-table-cells"));
    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown");

    assert_eq!(
        md,
        "| Region | A\\|B | Total |\n| --- | --- | --- |\n| North | merged span |  |\n|  |  |  |\n",
        "the fixture's 3x3: a header row, a merge whose covered slot is empty, an escaped pipe, \
         and a last row the document drew and never wrote in"
    );

    // Every cell string appears exactly once. A run emitted as GFM *and* as a paragraph would
    // count one node's characters twice, and the census would be arithmetic about no document.
    for text in ["Region", "Total", "North", "merged span"] {
        assert_eq!(
            md.matches(text).count(),
            1,
            "`{text}` must appear exactly once in {md:?}"
        );
    }

    // **The empty trailing row is still there.** Truncating it is the competitor erasure A14
    // names: a prettier table and a different document.
    assert!(
        md.contains("|  |  |  |\n"),
        "the last row is empty and declared, and it stays: {md:?}"
    );

    let erasures = a["coverage"]["structural_erasures"]
        .as_array()
        .expect("structural erasures");
    let count = |code: &str| -> u64 {
        erasures
            .iter()
            .find(|e| e["code"] == code)
            .and_then(|e| e["count"].as_u64())
            .unwrap_or(0)
    };
    assert_eq!(
        count("gfm-span-slots-unrepresentable-v1"),
        1,
        "the 1x2 merge held two slots and GFM can say one: {erasures:?}"
    );
    assert_eq!(
        count("gfm-row-zero-separator-v1"),
        1,
        "one table, one delimiter row, one header claim the document never made"
    );

    // And the character census is untouched by any of it.
    let c = &a["coverage"];
    assert_eq!(
        c["source_chars_emitted"].as_u64().unwrap() + c["source_chars_dropped"].as_u64().unwrap(),
        c["source_chars_in_representation"].as_u64().unwrap()
    );
    assert_eq!(
        c["source_chars_dropped"].as_u64().unwrap(),
        0,
        "flattening a merge loses slots, not characters — a dropped-character count here would \
         read 0 and disclose nothing, which is why the erasure has its own census"
    );
}

/// A tagged list becomes list items; an untagged document grows none.
#[test]
fn lists_come_from_the_tree_and_nowhere_else() {
    let dir = scratch("lists");
    let repr = extract_to(&dir, &engine_fixture("tagged-list-items"));
    let a = markdown_of(&repr);
    assert_eq!(
        a["markdown"].as_str().expect("markdown"),
        "- 1. First item\n- 2. Second item\n  - a. Nested item\n- 3. Third item continued\n\
         \n\
         Closing paragraph\n",
        "the document's own `1.` survives beside the `- ` this exporter added, the nested `/L` \
         indents, the two-run body joins, and the unmarked run ends the list"
    );

    let joins = a["coverage"]["structural_erasures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["code"] == "gfm-list-item-run-joins-v1")
        .and_then(|e| e["count"].as_u64())
        .unwrap_or(0);
    assert_eq!(
        joins, 1,
        "one item's body is two marked runs, and nothing in a role path distinguishes that from \
         two sibling items — so the join is made and counted"
    );

    // **The dangerous-looking case, and it is in the conformance corpus.**
    // `synthetic/list-items` draws a literal `-` and a literal `2.` and tags NOTHING. Its
    // projection therefore *looks* exactly like a Markdown list — and every byte of it is
    // `source`, because the document really did draw those characters. An exporter that had
    // inferred a list here would have produced the same string with `- ` marked `syntax`, and a
    // consumer quoting the line would have been told, wrongly, that it could not cite the bullet.
    let corpus = dir.join("corpus-list");
    std::fs::create_dir_all(&corpus).expect("scratch");
    let untagged_list = extract_to(&corpus, &conformance("synthetic/list-items/document.pdf"));
    let c = markdown_of(&untagged_list);
    let md = c["markdown"].as_str().unwrap();
    assert!(
        md.starts_with("- Verify cited evidence"),
        "the document's own hyphen leads the line: {md:?}"
    );
    let first = &c["anchor_map"]["segments"][0];
    assert_eq!(
        first["kind"], "source",
        "and it is SOURCE, not a marker this exporter added"
    );
    assert_eq!(
        first["start"].as_u64(),
        Some(0),
        "the source segment starts at byte 0 — there is no syntax in front of the hyphen"
    );
    assert!(
        c["coverage"]["structural_erasures"]
            .as_array()
            .unwrap()
            .is_empty(),
        "nothing was flattened: no table, no tagged list"
    );

    // **The untagged case, on the smallest real document there is.**
    let sub = dir.join("untagged");
    std::fs::create_dir_all(&sub).expect("scratch");
    let plain = extract_to(&sub, &conformance("synthetic/simple-text/document.pdf"));
    let b = markdown_of(&plain);
    assert_eq!(
        b["markdown"].as_str().unwrap(),
        "Hello Ethos\n",
        "no tree, no list markers — reading one off a bullet glyph or a hanging indent is the \
         refusal L29 makes about font-size headings, one structure level up"
    );
}

/// **The undeclared join, end to end** (v2.2-S5).
///
/// `untagged-shredded-line` is the only engine fixture whose runs share a baseline, and it exists
/// because every other one stacks them: before it, a rule keyed on "same baseline, next ink along
/// it" changed nothing in this suite and passed it unchanged. That is the same blindness the
/// 0.44.0 slice recorded, and this is the tripwire for it.
///
/// Three runs abut exactly — `Yar`+`ro`+`w` at 12 points per glyph — and the fourth starts 40
/// points after the third ends, so the gap the page drew still breaks the block. The census names
/// the two joins under the geometric code and not under `mcid-run-joins-v1`, because nothing in
/// the document declared them.
#[test]
fn runs_the_document_declared_nothing_about_join_along_one_baseline() {
    let dir = scratch("undeclared-join");
    let repr = extract_to(&dir, &engine_fixture("untagged-shredded-line"));
    let a = markdown_of(&repr);
    assert_eq!(a["markdown"].as_str().unwrap(), "Yarrow\n\nSeparate\n");

    let erasure = |code: &str| {
        a["coverage"]["structural_erasures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["code"] == code)
            .map(|e| e["count"].as_u64().unwrap())
            .unwrap_or(0)
    };
    assert_eq!(erasure("baseline-run-joins-abutted-v1"), 2);
    assert_eq!(erasure("baseline-run-joins-spaced-v1"), 0);
    assert_eq!(
        erasure("mcid-run-joins-v1"),
        0,
        "nothing in this document declares a marked-content sequence"
    );

    // The seam stays addressable: a join this engine measured never merges two nodes into one
    // segment, which is what separates it from a join the producer declared.
    for seg in a["anchor_map"]["segments"].as_array().unwrap() {
        if seg["kind"] == "source" {
            assert_eq!(
                seg["node_ids"].as_array().unwrap().len(),
                1,
                "a geometric join merged two nodes into one segment: {seg}"
            );
        }
    }
}

/// **The S1 fixtures come out byte-for-byte as they did**, modulo the identity fields.
///
/// S2 changed how a table and a list project. It did not change how a paragraph projects, and a
/// slice that quietly reflowed every document while adding a feature would be impossible to
/// review. Pinned as literals rather than as a diff against a stored artifact, so the assertion
/// says what the answer is.
#[test]
fn a_document_with_no_table_and_no_list_projects_exactly_as_it_did_at_s1() {
    let dir = scratch("unchanged");
    for (i, (path, want, segments)) in [
        (
            conformance("synthetic/simple-text/document.pdf"),
            "Hello Ethos\n",
            vec![("source", 0u64, 11u64), ("syntax", 11, 12)],
        ),
        (
            engine_fixture("markdown-two-blocks"),
            "First block\n\nSecond block\n",
            vec![
                ("source", 0, 11),
                ("syntax", 11, 13),
                ("source", 13, 25),
                ("syntax", 25, 26),
            ],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let sub = dir.join(format!("u{i}"));
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(&sub, &path);
        let a = markdown_of(&repr);
        assert_eq!(
            a["markdown"].as_str().unwrap(),
            want,
            "{}: the body must not have moved",
            path.display()
        );
        let got: Vec<(&str, u64, u64)> = a["anchor_map"]["segments"]
            .as_array()
            .unwrap()
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
            segments,
            "{}: the map geometry must not have moved either — no pipes, no new segments",
            path.display()
        );
        assert!(
            a["coverage"]["structural_erasures"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{}: nothing was flattened, so the erasure census is empty",
            path.display()
        );
    }
}

// -------------------------------------------------------------------------------------------
// v1.1-S3 — the one cosmetic, and the line it does not cross
// -------------------------------------------------------------------------------------------

/// A word the page broke across a line reads as one word — **in the export, and nowhere else**.
///
/// The whole slice in one test. `extract` still emits `hyphen-` and `ated` as two `Extracted`
/// runs — `hyphenated_line_breaks_are_not_rejoined_and_that_is_the_policy` is that half, and it is
/// a *policy*, because telling a soft break-hyphen from a compound one needs a dictionary. What
/// changes here is the projection: `docs/history/10-V11-SCOPE.md` §5 puts a cosmetic in the export or
/// nowhere.
///
/// So the assertions come in pairs. The Markdown reads `hyphenated`; the two runs the map names
/// still hold `hyphen-` and `ated`, hyphen and all. That gap is the difference between a cosmetic
/// and a rewrite, and it is why the hyphen is a **counted** character rather than a quiet one.
#[test]
fn a_word_broken_across_a_line_is_joined_in_the_export_and_nowhere_else() {
    let dir = scratch("hyphen");
    let repr = extract_to(
        &dir,
        &conformance("synthetic/hyphenated-line-break/document.pdf"),
    );
    let a = markdown_of(&repr);

    assert_eq!(
        a["markdown"].as_str().unwrap(),
        "hyphenated\n",
        "the export closes the word up; the hyphen is not in the string"
    );

    // **One `source` segment over the joined letters, naming both runs.** Not two adjacent
    // segments: the hyphen that marked where the halves met is gone, so there is no offset at
    // which the first run stops being the answer, and splitting would invent one.
    let segs = a["anchor_map"]["segments"].as_array().expect("segments");
    assert_eq!(
        segs.len(),
        2,
        "the joined word, then the trailing newline: {segs:?}"
    );
    assert_eq!(segs[0]["kind"], "source");
    assert_eq!(
        (segs[0]["start"].as_u64(), segs[0]["end"].as_u64()),
        (Some(0), Some(10))
    );
    let ids: Vec<&str> = segs[0]["node_ids"]
        .as_array()
        .expect("node_ids")
        .iter()
        .map(|v| v.as_str().expect("an id"))
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "these bytes came from two runs and the map has to say both: {ids:?}"
    );

    // **The other half of the pair.** The runs the map just named still hold what the page drew.
    let doc: Value =
        serde_json::from_slice(&std::fs::read(&repr).unwrap()).expect("representation");
    let nodes = doc["representation"]["nodes"].as_array().expect("nodes");
    let text_of = |id: &str| -> String {
        nodes
            .iter()
            .find(|n| n["id"].as_str() == Some(id))
            .and_then(|n| n["text"].as_str())
            .unwrap_or_else(|| panic!("the map names `{id}`, which must be a node"))
            .to_string()
    };
    assert_eq!(text_of(ids[0]), "hyphen-", "the record keeps the hyphen");
    assert_eq!(text_of(ids[1]), "ated");
    assert!(
        !nodes
            .iter()
            .any(|n| n["text"].as_str() == Some("hyphenated")),
        "no node holds the joined word, so a quote of it is READABLE and NOT CITABLE — that is \
         the correct answer for a word the page drew in two pieces, not a verifier defect. The \
         citable strings are the two the map names."
    );

    // **The dropped hyphen has a number.** A cosmetic that removed a character the document drew
    // and said nothing would be exactly the silent edit A14 exists to forbid.
    let c = &a["coverage"];
    let b = c["dropped"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["code"] == "hyphenation-rejoin-dropped-v1")
        .expect("the hyphen is a named bucket, not a rounding");
    assert_eq!(b["chars"].as_u64(), Some(1));
    assert_eq!(b["nodes"].as_u64(), Some(1));
    assert_eq!(
        c["source_chars_emitted"].as_u64().unwrap() + c["source_chars_dropped"].as_u64().unwrap(),
        c["source_chars_in_representation"].as_u64().unwrap(),
        "and the census still balances with it"
    );

    // A cosmetic is not a structural erasure: the thing removed IS a character of node text.
    assert!(c["structural_erasures"].as_array().unwrap().is_empty());
}

/// A hyphen the projection must leave alone: `simple-text` grows no joins.
///
/// The companion to the test above, and the reason the rule needs a letter on each side. Without
/// one, any run ending in a dash would swallow the next block.
#[test]
fn a_document_with_no_line_break_hyphen_declares_no_join() {
    let dir = scratch("nojoin");
    for name in ["simple-text", "two-lines"] {
        let sub = dir.join(name);
        std::fs::create_dir_all(&sub).expect("scratch");
        let repr = extract_to(
            &sub,
            &conformance(&format!("synthetic/{name}/document.pdf")),
        );
        let a = markdown_of(&repr);
        assert!(
            !a["coverage"]["dropped"]
                .as_array()
                .unwrap()
                .iter()
                .any(|b| b["code"] == "hyphenation-rejoin-dropped-v1"),
            "{name}: nothing was joined, so the bucket is absent rather than reading 0"
        );
    }
}

/// The profile says which rule ran, and it no longer declares a limitation that stopped being
/// true.
#[test]
fn the_profile_names_the_block_rule_and_has_retired_the_linear_one() {
    let dir = scratch("profile");
    let repr = extract_to(&dir, &engine_fixture("markdown-table-cells"));
    let doc: Value =
        serde_json::from_slice(&std::fs::read(&repr).unwrap()).expect("representation");

    let codes: Vec<&str> = doc["representation"]["assurance"]["limitations"]
        .as_array()
        .expect("limitations")
        .iter()
        .filter_map(|l| l["code"].as_str())
        .collect();
    assert!(
        codes.contains(&"markdown-table-spans-flattened"),
        "the narrower limitation that IS true must be declared: {codes:?}"
    );
    assert!(
        !codes.contains(&"markdown-table-structure-not-projected"),
        "`the grid is not carried` is false the moment a table becomes GFM, and a reader ACTS on \
         a stale limitation — this one would send them to `tables` on the representation for a \
         grid the Markdown now has. Deleted, not reworded, the way v1-S2 and v1-S8 retired theirs."
    );

    assert_eq!(markdown_of(&repr)["markdown_rule"], "markdown-blocks-v10");
}

// -------------------------------------------------------------------------------------------
// The golden — the sentence v1.1 exists to make true
// -------------------------------------------------------------------------------------------

/// **A Markdown quote is either invertible to canonical evidence, or explicitly unquotable.**
///
/// The real binaries, end to end, with the pinned verifier deciding. See this file's module
/// documentation for why the second half is the half that matters.
#[test]
fn a_markdown_quote_verifies_end_to_end() {
    // **Resolved up front so its absence is a NAMED failure**, not a confusing "expected exit 0,
    // got 2" three assertions later. Same posture as `verify_relay.rs`: a suite that quietly
    // passed on a machine with no verifier would be asserting nothing at all.
    let verifier = ethos_binary();
    assert!(
        verifier.is_file(),
        "the pinned Ethos CLI is a test-time dependency of this golden: {}",
        verifier.display()
    );

    let dir = scratch("golden");
    // **An engine-owned fixture, and it had to be authored for this.** The golden needs two runs
    // that BOTH reach `ethos.grounding.v1` — a syntax join to span, and a verifier that can find
    // the element either quote names. Every pre-existing fixture with measurable ink metrics has
    // exactly one run, so there was no join; the fixtures with two runs declare no metrics, so
    // their `elements` array is empty and the golden would pass vacuously against a verifier that
    // found nothing either way.
    let pdf = repo_root().join("fixtures/engine/markdown-two-blocks/document.pdf");
    let repr = extract_to(&dir, &pdf);

    // The grounding artifact the verifier will read. It knows nothing about Markdown.
    let grounded_out = engine(&["ground", repr.to_str().unwrap()]);
    assert_eq!(grounded_out.status.code(), Some(0), "ground must succeed");
    let grounding = dir.join("grounding.json");
    std::fs::write(&grounding, &grounded_out.stdout).expect("write grounding");
    let fingerprint = format!(
        "sha256:{}",
        ethos_parser_core::sha256_hex_bytes(&std::fs::read(&grounding).unwrap())
    );

    // The projection, and its map.
    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown").to_string();
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
    let source_quote = md[ss..se].to_string();
    assert!(
        !source_quote.trim().is_empty(),
        "the quote must be substantive"
    );

    // A quote that TOUCHES a syntax segment: real text in the Markdown, drawn by nothing.
    let syntax = segs
        .iter()
        .find(|s| s["kind"] == "syntax" && s["end"].as_u64().unwrap() < md.len() as u64)
        .expect("the join between two runs is a syntax segment");
    let (ys, ye) = (
        syntax["start"].as_u64().unwrap() as usize,
        syntax["end"].as_u64().unwrap() as usize,
    );
    // Widen across the join so the quote is a plausible-looking sentence rather than bare
    // whitespace — this is the string a consumer reading the `.md` alone would happily cite.
    let spanning = md[ys.saturating_sub(4)..(ye + 6).min(md.len())].to_string();
    assert!(
        spanning.contains(&md[ys..ye]),
        "the spanning quote must actually include the invented bytes"
    );
    assert!(
        !md[ss..se].contains(&spanning),
        "and it must not be wholly inside the source segment"
    );

    // The citation names the element the SOURCE quote came from. Read off the grounding artifact
    // rather than hardcoded: an id that does not exist comes back `element_not_found`, which would
    // make both halves of this test pass for a reason that has nothing to do with the Anchor Map.
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

    let grounded_claims = write_claims("from-source.json", &source_quote);
    let spanning_claims = write_claims("from-syntax.json", &spanning);

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
    // `all_evidence_grounded` is the boolean the verifier writes; this test forms no opinion of
    // its own about whether a quote is supported, which is the entire point of the boundary.
    let grounded_of = |report: &Value| -> bool {
        let v = report
            .get("all_evidence_grounded")
            .and_then(Value::as_bool)
            .unwrap_or_else(|| panic!("the report must carry `all_evidence_grounded`: {report}"));
        // And the per-claim status agrees, so a summary field alone cannot carry the assertion.
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
        grounded_of(&verify(&grounded_claims)),
        "a quote taken from a SOURCE segment is text the document drew, so it must ground. \
         Quote: {source_quote:?}"
    );

    // **And it must fail for the RIGHT reason.** `element_not_found` would mean the citation
    // pointed at nothing and this test would pass without the Anchor Map having demonstrated
    // anything at all. `text_mismatch` is the verifier saying: I found the element you cited, and
    // the page did not draw the words you quoted.
    let spanning_report = verify(&spanning_claims);
    let reason = spanning_report["checks"][0]["reason"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        reason, "text_mismatch",
        "the spanning quote must be refused because the TEXT is not on the page, not because the \
         citation missed: {spanning_report}"
    );

    assert!(
        !grounded_of(&spanning_report),
        "a quote spanning a SYNTAX segment is text this exporter invented — the page never drew \
         it — so it must NOT ground. That is the whole claim of v1.1: from the `.md` alone this \
         string is indistinguishable from a real sentence, and only the Anchor Map says which it \
         is. Quote: {spanning:?}"
    );
}

/// **The same sentence, one structure level up: a quote lifted out of a GFM CELL.**
///
/// v1.1-S1's golden proved it for a paragraph, where the invented bytes are a blank line. A table
/// is where the claim gets harder and where it matters more, because a GFM row is *mostly*
/// invented: pipes, spaces, dashes and an escape, wrapped around text the page really drew. From
/// the `.md` alone `North | merged span` is indistinguishable from a sentence; only the map says
/// it is chrome.
///
/// The cell that grounds here is the **origin of the merge**, which is the case A14 is about:
/// GFM flattened the span, the artifact counted the slots it cost, and the text that survived the
/// flattening still inverts to the run that drew it.
#[test]
fn a_quote_from_a_gfm_cell_verifies_end_to_end() {
    let verifier = ethos_binary();
    assert!(
        verifier.is_file(),
        "the pinned Ethos CLI is a test-time dependency of this golden: {}",
        verifier.display()
    );

    let dir = scratch("cell-golden");
    // **Authored for this, like `markdown-two-blocks` was.** `ruled-table-grid` already has a
    // merge and an empty cell, but its font declares no ink metrics, so no cell run reaches
    // `ethos.grounding.v1` — the verifier would find nothing and refuse both halves, which
    // proves nothing about cells.
    let repr = extract_to(&dir, &engine_fixture("markdown-table-cells"));

    let grounded_out = engine(&["ground", repr.to_str().unwrap()]);
    assert_eq!(grounded_out.status.code(), Some(0), "ground must succeed");
    let grounding = dir.join("grounding.json");
    std::fs::write(&grounding, &grounded_out.stdout).expect("write grounding");
    let fingerprint = format!(
        "sha256:{}",
        ethos_parser_core::sha256_hex_bytes(&std::fs::read(&grounding).unwrap())
    );

    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown").to_string();
    let segs = a["anchor_map"]["segments"].as_array().expect("segments");

    // The merged cell's text, located through the MAP rather than by searching the string: the
    // point of the artifact is that a consumer does not have to guess which bytes are citable.
    let cell = "merged span";
    let at = md
        .find(cell)
        .expect("the merged cell's text is in the table");
    let segment = segs
        .iter()
        .find(|s| {
            s["kind"] == "source"
                && s["start"].as_u64().unwrap() as usize == at
                && s["end"].as_u64().unwrap() as usize == at + cell.len()
        })
        .expect("and it is exactly one source segment, naming the run that drew it");
    assert_eq!(
        segment["node_ids"].as_array().map(Vec::len),
        Some(1),
        "one run, one id — and an id that already existed upstream"
    );

    // The chrome around it. `North | merged span` is real text in the Markdown; the page drew a
    // ruling line there, not a pipe.
    let row_start = md.find("| North").expect("the body row");
    let spanning = md[row_start + 2..at + cell.len()].to_string();
    assert!(
        spanning.contains('|'),
        "the spanning quote must actually include the invented chrome: {spanning:?}"
    );

    let grounding_doc: Value =
        serde_json::from_slice(&std::fs::read(&grounding).unwrap()).expect("grounding JSON");
    // Read off the artifact rather than hardcoded: a citation naming an id that does not exist
    // comes back `element_not_found`, and both halves would then pass for a reason that has
    // nothing to do with the Anchor Map.
    let element_id = grounding_doc["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .find(|e| e["text"].as_str() == Some(cell))
        .and_then(|e| e["id"].as_str())
        .expect("the merged cell's run is an element of the grounding artifact")
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
        grounded_of(&verify(&write_claims("from-cell.json", cell))),
        "a quote copied out of a GFM cell is text the document drew — the merge GFM could not \
         represent did not touch it — so it must ground. Quote: {cell:?}"
    );

    let spanning_report = verify(&write_claims("from-chrome.json", &spanning));
    let reason = spanning_report["checks"][0]["reason"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        reason, "text_mismatch",
        "the spanning quote must be refused because the TEXT is not on the page, not because the \
         citation missed — `element_not_found` would make this test pass without the Anchor Map \
         having demonstrated anything: {spanning_report}"
    );
    assert!(
        !grounded_of(&spanning_report),
        "a quote carrying table chrome is text this exporter invented. The document painted a \
         ruling line; it did not draw a `|`. Quote: {spanning:?}"
    );
}

/// **The same sentence again, and this time the invented bytes are a word.**
///
/// S1's golden refused a quote spanning a blank line; S2's refused one carrying table chrome. Both
/// are *punctuation* a reader might squint at. v1.1-S3 produces something harder: a joined word
/// that is ordinary English, in the middle of an ordinary sentence, with nothing about it to
/// squint at.
///
/// The fixture's page draws two lines:
///
/// ```text
///     The rate may be recalcu-
///     lated at closing
/// ```
///
/// and the projection closes that up to `The rate may be recalculated at closing`. **A model handed
/// that Markdown would cite it without hesitation, and the page never drew it.** `extract` still
/// holds `recalcu-` and `lated` as two `Extracted` runs — that is the standing policy
/// `hyphenated_line_breaks_are_not_rejoined_and_that_is_the_policy` pins — so the joined word is on
/// no element of `ethos.grounding.v1`.
///
/// This test makes the whole trade executable, with the pinned Ethos CLI deciding:
///
/// | quote | expected |
/// | --- | --- |
/// | `The rate may be recalcu-` — the first half, as the page drew it | **grounded** |
/// | `lated at closing` — the second half | **grounded** |
/// | `recalculated` — the word only the export contains | **`text_mismatch`** |
///
/// The third is the one that means something, and it must fail for the RIGHT reason:
/// `element_not_found` would mean the citation pointed at nothing, and the test would pass without
/// the cosmetic having been examined at all.
///
/// **`recalculated` is never asserted grounded, and that is not a gap.** It is what
/// `docs/history/10-V11-SCOPE.md` §5 buys: the export may repair, the evidence record may not, and the
/// map names the two strings that *are* citable.
#[test]
fn the_joined_word_does_not_ground_and_both_halves_do() {
    let verifier = ethos_binary();
    assert!(
        verifier.is_file(),
        "the pinned Ethos CLI is a test-time dependency of this golden: {}",
        verifier.display()
    );

    let dir = scratch("hyphen-golden");
    // **Authored for this, like `markdown-two-blocks` and `markdown-table-cells` were.** The
    // Ethos corpus already carries the shape — `synthetic/hyphenated-line-break` is `hyphen-` then
    // `ated` — but its font declares no ink metrics, so both runs take the typed-absent path and
    // `elements` comes out empty. A golden there would watch the verifier find nothing and refuse
    // every quote, which proves nothing about the join.
    let repr = extract_to(&dir, &engine_fixture("markdown-hyphen-break"));

    let grounded_out = engine(&["ground", repr.to_str().unwrap()]);
    assert_eq!(grounded_out.status.code(), Some(0), "ground must succeed");
    let grounding = dir.join("grounding.json");
    std::fs::write(&grounding, &grounded_out.stdout).expect("write grounding");
    let fingerprint = format!(
        "sha256:{}",
        ethos_parser_core::sha256_hex_bytes(&std::fs::read(&grounding).unwrap())
    );

    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown").to_string();
    assert_eq!(
        md, "The rate may be recalculated at closing\n",
        "the export closes the word up"
    );

    // The joined bytes are ONE source segment naming BOTH runs — the map's own account of what
    // happened, read here rather than assumed.
    let segs = a["anchor_map"]["segments"].as_array().expect("segments");
    let joined = segs
        .iter()
        .find(|s| s["kind"] == "source")
        .expect("a source segment");
    let ids: Vec<String> = joined["node_ids"]
        .as_array()
        .expect("node_ids")
        .iter()
        .map(|v| v.as_str().expect("an id").to_string())
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "these bytes came from two runs and the map says so: {ids:?}"
    );

    // The two halves, read off the GROUNDING artifact rather than hardcoded: an id that does not
    // exist comes back `element_not_found`, and every half of this test would then pass for a
    // reason that has nothing to do with the join.
    let grounding_doc: Value =
        serde_json::from_slice(&std::fs::read(&grounding).unwrap()).expect("grounding JSON");
    let elements: Vec<(String, String)> = grounding_doc["elements"]
        .as_array()
        .expect("elements")
        .iter()
        .filter_map(|e| {
            Some((
                e["id"].as_str()?.to_string(),
                e["text"].as_str()?.to_string(),
            ))
        })
        .collect();
    assert_eq!(
        elements.len(),
        2,
        "both halves must reach the grounding artifact, or this golden is vacuous: {elements:?}"
    );
    let (first_id, first_half) = elements[0].clone();
    let (second_id, second_half) = elements[1].clone();
    assert_eq!(first_half, "The rate may be recalcu-");
    assert_eq!(second_half, "lated at closing");

    // And the joined word is on NEITHER of them. That is the fact the third claim below asks the
    // verifier to confirm independently.
    assert!(
        !elements.iter().any(|(_, t)| t.contains("recalculated")),
        "no element carries the joined word — the page drew it in two pieces: {elements:?}"
    );

    let write_claims = |name: &str, text: &str, element: &str| -> PathBuf {
        let p = dir.join(name);
        let body = serde_json::json!({
            "document_fingerprint": fingerprint,
            "claims": [{
                "kind": "quote",
                "text": text,
                "citation": { "page": "p1", "element_id": element }
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

    // **The export joined; the evidence record did not** — so both halves are still citable,
    // exactly as they were before this slice existed.
    assert!(
        grounded_of(&verify(&write_claims(
            "first-half.json",
            &first_half,
            &first_id
        ))),
        "the first half is text the document drew, hyphen and all, so it must still ground"
    );
    assert!(
        grounded_of(&verify(&write_claims(
            "second-half.json",
            &second_half,
            &second_id
        ))),
        "and so must the second"
    );

    // **The word only the export contains.** Cited against an element that DOES exist, so the
    // verifier finds it and judges the text.
    let joined_report = verify(&write_claims("joined-word.json", "recalculated", &first_id));
    let reason = joined_report["checks"][0]["reason"].as_str().unwrap_or("");
    assert_eq!(
        reason, "text_mismatch",
        "the joined word must be refused because the TEXT is not on the page, not because the \
         citation missed — `element_not_found` would make this test pass without the cosmetic \
         having been examined at all: {joined_report}"
    );
    assert!(
        !grounded_of(&joined_report),
        "`recalculated` reads like ordinary English in an ordinary sentence, and the page never \
         drew it: it drew `recalcu-` and `lated` on two lines. This is the cost of the S3 \
         cosmetic, and the correct answer — not a defect in the verifier. What the artifact owes \
         a consumer is that the map names the two strings that ARE citable, and it does."
    );
}

// -------------------------------------------------------------------------------------------

/// **An EPUB's own heading element reaches the projection** (v2.2-S0).
///
/// `heading_level` read one source — a PDF's tagged `/H1`..`/H6` — so an EPUB whose XHTML says
/// `<h1>` in as many words projected as a paragraph, and `docs/CAPABILITY.md`'s *"office documents
/// project too"* carried no caveat saying so. The reader had always carried the element name for
/// exactly this: *"XHTML has no such distinction, so it is left false and the element's own name is
/// carried beside it instead"* (`crates/ethos-parser-office/src/epub.rs`).
///
/// This is not L29. `<h1>` is the document declaring a heading and its level, the same kind of
/// statement `/H1` is, and no font size is consulted to reach it.
#[test]
fn an_epubs_own_heading_element_projects_as_a_heading() {
    let dir = scratch("epub-heading");
    let repr = extract_to(
        &dir,
        &repo_root().join("fixtures/office/book-spine/book.epub"),
    );
    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown string");

    assert!(
        md.contains("# Evidence, not extraction."),
        "the `<h1>` the publication declares must project as a heading:\n{md}"
    );

    // The other half, and the reason this is not just a `starts_with('h')` test: the same fixture
    // carries `p`, `pre`, `td` and `li` blocks, and none of them is a heading. A rule that turned
    // any element into one would pass the assertion above and be wrong about everything else.
    assert!(
        md.contains("Rows & columns bind to a block and never to a page."),
        "a `<p>` must still be there:\n{md}"
    );
    assert!(
        !md.contains("# Rows & columns"),
        "a `<p>` must NOT become a heading:\n{md}"
    );
    assert!(
        !md.contains("# Left cell"),
        "a `<td>` must NOT become a heading:\n{md}"
    );
}

/// **An ODT's own `text:outline-level` projects as a heading** (v2.4).
///
/// The end-to-end half of `markdown::tests::every_odf_outline_level_projects_at_its_own_depth`.
/// The unit test reaches `h2`..`h6`, which no package in the corpus carries; this one proves the
/// level survives the two hops between `<text:h text:outline-level="1">` and `# `.
#[test]
fn an_odts_own_outline_level_projects_as_a_heading() {
    let dir = scratch("odt-heading");
    let repr = extract_to(
        &dir,
        &repo_root().join("fixtures/office/text-paragraphs/document.odt"),
    );
    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown string");

    assert!(
        md.contains("# Evidence, not extraction."),
        "the `<text:h text:outline-level=\"1\">` the document declares must project as a \
         heading:\n{md}"
    );
    assert_eq!(
        md.matches('#').count(),
        1,
        "one `#` and no other: five `<text:p>` follow it, and a `<text:p>` is not a \
         heading:\n{md}"
    );
    assert_eq!(a["markdown_rule"], "markdown-blocks-v10");
    // **The negative, and it is the one that earns its place.** A heading that projected at its
    // own depth flattened nothing — this is the only guard that a predicate which forgot to
    // short-circuit on a level it CAN write would fail.
    assert_eq!(
        a["coverage"]["structural_erasures"]
            .as_array()
            .map(Vec::len),
        Some(0),
        "nothing was flattened: {}",
        a["coverage"]["structural_erasures"]
    );
}

/// **An ODP `<text:h>` that states no level projects as a paragraph.**
///
/// The refusal, end to end, and the reason the test above cannot stand alone: a rule that emitted
/// `# ` for every `OdfBlockKind::Heading` would pass it and be wrong here. The deck writes a bare
/// `<text:h>` — a heading whose depth comes from an outline style in `styles.xml`, which the
/// reader declares it did not open — so `# ` would be level one on no evidence.
#[test]
fn an_odp_heading_with_no_stated_level_projects_as_a_paragraph() {
    let dir = scratch("odp-heading");
    let repr = extract_to(
        &dir,
        &repo_root().join("fixtures/office/presentation-pages/presentation.odp"),
    );
    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown string");

    assert!(
        md.contains("Evidence, not extraction."),
        "the text is there:\n{md}"
    );
    assert!(
        !md.contains('#'),
        "and no `#` anywhere: the deck states the fact of a heading and never its level:\n{md}"
    );
    // **And it says so.** Before the erasure declaration this projection dropped two declared
    // headings to body text and the artifact carried no trace of it.
    assert_eq!(
        *a["coverage"]["structural_erasures"]
            .as_array()
            .expect("an array"),
        vec![serde_json::json!({"code": "heading-level-unresolved-v1", "count": 2})],
        "two bare `<text:h>`, one count each, and no other code"
    );
}

/// **A tagged PDF's headings are untouched by the EPUB source being added.**
///
/// The two sources sit in one function, so the cheapest way for this change to have gone wrong is
/// for the new arm to shadow the old one. `irs-fw9` is tagged and carries real `/H` roles.
#[test]
fn adding_the_epub_source_leaves_tagged_pdf_headings_alone() {
    let dir = scratch("epub-heading-pdf");
    let repr = extract_to(&dir, &repo_root().join("fixtures/gate/irs-fw9.pdf"));
    let a = markdown_of(&repr);
    let md = a["markdown"].as_str().expect("markdown string");

    let headings = md.lines().filter(|l| l.starts_with('#')).count();
    assert!(
        headings > 0,
        "a tagged PDF must still project its own headings: {headings}"
    );
}

// -------------------------------------------------------------------------------------------
// Decision #29: a heading inferred from type
// -------------------------------------------------------------------------------------------

/// **An inferred heading projects as a level-one heading, in both syntaxes, and its marker is
/// syntax** (`docs/28-HEADINGS-SCOPE.md` S2).
///
/// On S1's fixture — one display line at twice the body type, no `/StructTreeRoot` — the reader
/// flagged the display line's run, and both projections render it: `# ` in Markdown, `<h1>` in
/// HTML. The `# ` is one `syntax` segment naming no node, byte-identical in shape to a declared
/// heading's marker, so a quote touching it does not invert: the rule a consumer already has
/// covers an inferred heading unchanged. What says the `#` was inferred is the artifact's own
/// `markdown_rule`, `-v8`, and the representation it names — the `.md` alone cannot say, and
/// the scope's §5.3 is why nothing is invented to make it.
#[test]
fn an_inferred_heading_projects_as_a_level_one_heading() {
    let dir = scratch("inferred-heading");
    let repr = extract_to(&dir, &engine_fixture("heading-display-line"));

    let md = markdown_of(&repr);
    let text = md["markdown"].as_str().expect("a Markdown string");
    assert!(
        text.starts_with("# Display line\n\n"),
        "the display line is a level-one heading and nothing precedes it: {text:?}"
    );
    assert_eq!(
        text.matches('#').count(),
        1,
        "one heading, and no body line became one"
    );
    assert_eq!(md["markdown_rule"], "markdown-blocks-v10");

    let first = &md["anchor_map"]["segments"][0];
    assert_eq!(
        first["kind"], "syntax",
        "the marker is the exporter's, not the page's"
    );
    assert_eq!(first["start"], 0);
    assert_eq!(first["end"], 2, "`# ` and nothing more");
    assert!(
        first.get("node_ids").is_none(),
        "a syntax segment names no node: {first}"
    );

    let out = engine(&["html", repr.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let html: Value = serde_json::from_slice(&out.stdout).expect("canonical JSON");
    let text = html["html"].as_str().expect("an HTML string");
    assert!(
        text.starts_with("<h1>Display line</h1>\n"),
        "the same line, the same level, in the other syntax: {text:?}"
    );
    assert_eq!(html["html_rule"], "html-blocks-v10");
}

/// **`--source` is the same two stages in one process, and the bytes say so.**
///
/// The flag exists because the two-step path spends more on carrying the record between two
/// processes than on reading the document: 7.6 ms of process start and 3.1 ms of serialising,
/// writing, reading, parsing and re-hashing a 250 KB representation, against 5.4 ms of engine work
/// per document (`docs/measurements/liteparse-head-to-head/README.md` §3). None of that is the
/// projection, so none of it may change the projection — which is what this asserts, on a fixture
/// carrying a table, an inferred heading and an artifact run, over both projections.
#[test]
fn the_source_path_equals_the_two_step_path() {
    for fixture in [
        "heading-display-line",
        "leading-gap-two-blocks",
        "two-column-15-lines",
    ] {
        let pdf = engine_fixture(fixture);
        let pdf = pdf.to_str().unwrap();
        let dir = scratch("source-equals");
        let repr_path = dir.join(format!("{fixture}.json"));

        let extracted = engine(&["extract", pdf]);
        assert_eq!(extracted.status.code(), Some(0), "extract {fixture}");
        std::fs::write(&repr_path, &extracted.stdout).expect("write the record");

        for projection in ["markdown", "html"] {
            if projection == "html" {
                // `html` still takes a record only; the flag landed on `markdown` alone, where the
                // cost was measured. This arm asserts that, so the asymmetry is deliberate rather
                // than forgotten.
                let refused = engine(&["html", "--source", pdf]);
                assert_ne!(refused.status.code(), Some(0), "html has no --source yet");
                continue;
            }
            let two_step = engine(&[projection, repr_path.to_str().unwrap()]);
            let one_step = engine(&[projection, "--source", pdf]);
            assert_eq!(
                two_step.status.code(),
                Some(0),
                "{projection} from the record"
            );
            assert_eq!(one_step.status.code(), Some(0), "{projection} --source");
            assert_eq!(
                two_step.stdout, one_step.stdout,
                "{projection} of {fixture} differs between the two paths"
            );
            assert!(
                one_step.stderr.is_empty(),
                "nothing on stderr: {:?}",
                one_step.stderr
            );
        }
    }
}

/// **Neither input, or both, is a usage error — and a source that states no format is the
/// extractor's own refusal.**
///
/// The flag names which of the two things the caller means, so nothing is guessed from the bytes
/// beyond the format question `extract` already asks. A `.csv` handed to `--source` gets the
/// message v2-S10 wrote for bytes that state no format, not a message about JSON.
#[test]
fn the_two_inputs_are_named_never_guessed() {
    let dir = scratch("source-refusals");
    let formatless = dir.join("shopping.csv");
    std::fs::write(&formatless, b"eggs,2\nmilk,1\n").expect("write the formatless file");

    let neither = engine(&["markdown"]);
    assert_eq!(
        neither.status.code(),
        Some(2),
        "a subcommand with no input is a usage error"
    );

    let pdf = engine_fixture("leading-gap-two-blocks");
    let both = engine(&[
        "markdown",
        pdf.to_str().unwrap(),
        "--source",
        pdf.to_str().unwrap(),
    ]);
    assert_eq!(both.status.code(), Some(2), "two inputs is a usage error");

    let no_format = engine(&["markdown", "--source", formatless.to_str().unwrap()]);
    assert_eq!(no_format.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&no_format.stderr);
    assert!(
        stderr.contains("these bytes state no format this engine reads"),
        "the extractor's own refusal, not a JSON parse error: {stderr}"
    );

    // And the record path still refuses a representation whose payload does not hash to its digest.
    let repr = dir.join("tampered.json");
    let extracted = engine(&["extract", pdf.to_str().unwrap()]);
    let mut json: Value = serde_json::from_slice(&extracted.stdout).expect("a record");
    json["representation"]["nodes"][0]["text_run"]["text"] = Value::String("edited".into());
    std::fs::write(&repr, serde_json::to_vec(&json).unwrap()).expect("write the tampered record");
    let tampered = engine(&["markdown", repr.to_str().unwrap()]);
    assert_eq!(
        tampered.status.code(),
        Some(2),
        "a tampered record is still refused"
    );
}
