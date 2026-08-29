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

//! **The CLI is a thin shell** — proved without the CLI.
//!
//! `docs/05-MILESTONES.md` M7: *"every subcommand behaviour is reachable through the library,
//! proved by library-level tests that do not invoke the binary."* Nothing in this file spawns a
//! process. `CARGO_BIN_EXE_ethos-parser` does not appear, and
//! `no_test_in_this_file_spawns_the_binary` asserts that about the source rather than trusting it.
//!
//! # Why it must be its own file
//!
//! `classify_cli.rs` and `grounding.rs` already assert that the binary's stdout equals the
//! library's bytes. That is the *other* half, and on its own it is circular: it proves the two
//! agree, not that the library alone can produce the artifact. If a subcommand ever grew a step
//! the CLI performed itself — a fixup, a default, a bit of reordering — those tests would still
//! pass, because both sides would include it. These would not.
//!
//! # Why it lives in `ethos-parser-cli`
//!
//! Because it is the only crate that depends on all three libraries. `ethos-parser-grounding` may never
//! depend on `ethos-parser-pdf` (`docs/04-ARCHITECTURE.md` §1), so a test spanning extract → project
//! has nowhere else to go. The crate is the host; the binary is not involved.

use std::path::PathBuf;

use ethos_parser_core::{DocumentRepresentation, Profile};
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

fn engine_fx(name: &str) -> PathBuf {
    path_in("engine", &format!("{name}/document.pdf"))
}

fn conformance(rel: &str) -> PathBuf {
    path_in("conformance", rel)
}

/// Canonical JSON is JSON, and it parses. Anything else is not an artifact.
fn parse_canonical(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).expect("canonical bytes are valid JSON")
}

// -------------------------------------------------------------------------------------------
// One test per subcommand
// -------------------------------------------------------------------------------------------

/// `ethos-parser classify` — reachable as `Document::open` → `classify` → `to_canonical_bytes`.
#[test]
fn classify_is_reachable_and_canonical_from_the_library() {
    let profile = Profile::default();
    let doc = Document::open(&engine_fx("measured-ink-box"), &profile).expect("opens");
    let classification = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");

    let bytes = classification
        .to_canonical_bytes()
        .expect("canonicalizes without the CLI");
    let v = parse_canonical(&bytes);

    assert_eq!(
        v["identity"]["artifact_type"],
        "ethos.parser.classification.v0"
    );
    assert!(v["pages"].is_array(), "per-page counts are present");
    assert!(
        v["needs_attention"].is_boolean(),
        "the derived boolean is present"
    );

    // The exit-code mapping is library-owned too, or an embedding caller has to re-derive the
    // rule the binary uses — which is exactly how two callers come to disagree.
    let code = ethos_parser_pdf::exit::exit_code(&Ok(classification));
    assert!(
        code == ethos_parser_pdf::exit::SIMPLE || code == ethos_parser_pdf::exit::NEEDS_ATTENTION,
        "a document that classified maps to 0 or 1, never {code}"
    );
}

/// `ethos-parser extract` — reachable, and it is the **representation** the subcommand emits.
#[test]
fn extract_and_represent_are_reachable_and_canonical_from_the_library() {
    let profile = Profile::default();
    let doc = Document::open(&engine_fx("measured-ink-box"), &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");

    repr.verify_fingerprint()
        .expect("the library seals what it produces");

    let bytes = repr.to_canonical_bytes().expect("canonicalizes");
    let v = parse_canonical(&bytes);
    assert_eq!(v["artifact_type"], "ethos.parser.representation.v0");
    assert!(
        !v["representation"]["nodes"].as_array().unwrap().is_empty(),
        "the fixture has text, so the representation has nodes"
    );
}

/// `ethos-parser ground` — reachable as parse → `verify_fingerprint` → `project` → canonical bytes.
///
/// Including the fingerprint check, because that is a *behaviour of the subcommand*: `ground`
/// refuses a representation whose payload does not hash to its declared digest. A library caller
/// that skipped it would be projecting a record the engine will not speak for.
#[test]
fn project_is_reachable_and_canonical_from_the_library() {
    let profile = Profile::default();
    let doc = Document::open(&engine_fx("measured-ink-box"), &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");

    // Round-trip through canonical bytes, exactly as the subcommand receives it from disk.
    let on_disk = repr.to_canonical_bytes().expect("canonicalizes");
    let reparsed: DocumentRepresentation =
        serde_json::from_slice(&on_disk).expect("a representation parses back");
    reparsed
        .verify_fingerprint()
        .expect("the fingerprint survives the round trip");

    let projection = ethos_parser_grounding::project(&reparsed).expect("projects");
    let bytes = ethos_parser_grounding::to_canonical_bytes(&projection.source).expect("canonicalizes");
    let v = parse_canonical(&bytes);

    assert_eq!(v["artifact_type"], "ethos.grounding.v1");
    assert!(v["pages"].is_array());

    // The omission report is on the projection, not on stderr: the CLI prints it, but the count
    // is a library value, so a caller embedding the engine can read it without parsing text.
    let _ = projection.omission.is_lossy();
}

/// `ethos-parser grounding-check` — reachable, including the report's own exit-code mapping.
#[test]
fn grounding_check_is_reachable_and_canonical_from_the_library() {
    let profile = Profile::default();
    let pdf = engine_fx("measured-ink-box");
    let source = std::fs::read(&pdf).expect("fixture readable");

    let doc = Document::open(&pdf, &profile).expect("opens");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");
    let projection = ethos_parser_grounding::project(&repr).expect("projects");
    let grounding =
        ethos_parser_grounding::to_canonical_bytes(&projection.source).expect("canonicalizes");

    // With the source: the binding is answered.
    let matched = ethos_parser_grounding::grounding_check(&grounding, Some(&source)).expect("checks");
    let v = parse_canonical(&matched.to_canonical_bytes().expect("canonicalizes"));
    assert_eq!(v["structure"], "valid");
    assert_eq!(v["source_binding"], "matched");
    assert_eq!(matched.exit_code(), 0);

    // Without it: not a pass, a question that was not asked.
    let unchecked = ethos_parser_grounding::grounding_check(&grounding, None).expect("checks");
    let v = parse_canonical(&unchecked.to_canonical_bytes().expect("canonicalizes"));
    assert_eq!(v["source_binding"], "not_checked");
    assert_eq!(unchecked.exit_code(), 0);

    // A different document: mismatched, and non-zero.
    let other = std::fs::read(conformance("synthetic/two-lines/document.pdf")).expect("readable");
    let mismatched = ethos_parser_grounding::grounding_check(&grounding, Some(&other)).expect("checks");
    let v = parse_canonical(&mismatched.to_canonical_bytes().expect("canonicalizes"));
    assert_eq!(v["source_binding"], "mismatched");
    assert_ne!(
        mismatched.exit_code(),
        0,
        "a source that does not bind is not a pass"
    );
}

// -------------------------------------------------------------------------------------------
// The whole path, and the properties that hang off it
// -------------------------------------------------------------------------------------------

/// The full happy path in one function, with no process boundary anywhere.
///
/// This is what an embedding caller writes. If it ever stops being writable, the CLI has grown
/// logic — and that is the failure this milestone exists to prevent.
#[test]
fn the_whole_happy_path_runs_inside_one_process() {
    let profile = Profile::default();
    let pdf = engine_fx("measured-ink-box");
    let source = std::fs::read(&pdf).expect("readable");

    let doc = Document::open_bytes(&source, &profile).expect("opens");
    let _classification = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");
    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");
    let projection = ethos_parser_grounding::project(&repr).expect("projects");
    let grounding =
        ethos_parser_grounding::to_canonical_bytes(&projection.source).expect("canonicalizes");
    let report = ethos_parser_grounding::grounding_check(&grounding, Some(&source)).expect("checks");

    assert_eq!(report.exit_code(), 0, "the happy path ends valid and bound");

    // One document load served classify and extract — `docs/04-ARCHITECTURE.md` §2.1. A library
    // caller gets that property for free precisely because opening is the caller's decision.
    assert_eq!(
        doc.source_sha256().hex(),
        ethos_parser_core::sha256_hex_bytes(&source)
    );
}

/// **Every artifact carrying geometry declares its `coordinate_system`** — `docs/03-V0-SCOPE.md`
/// §5, and the CI job `v0-coordinates`.
///
/// The qualifier is load-bearing in both directions, so both are asserted. An artifact with boxes
/// and no declared frame is uninterpretable — a reader cannot tell centipoints from points, or
/// top-left from bottom-left, and PDF's own origin is the opposite of the one used here. But a
/// classification carries *no* geometry, and declaring a coordinate system on it would be
/// describing a frame nothing in the document is expressed in.
#[test]
fn every_geometry_bearing_artifact_declares_its_coordinate_system() {
    let profile = Profile::default();
    let doc = Document::open(&engine_fx("measured-ink-box"), &profile).expect("opens");

    let classification = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");
    let c = parse_canonical(&classification.to_canonical_bytes().expect("canonical"));
    assert!(
        !c.to_string().contains("coordinate_system"),
        "a classification carries no geometry, so it must not declare a frame"
    );

    let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
    let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");
    let r = parse_canonical(&repr.to_canonical_bytes().expect("canonical"));
    assert_eq!(
        r["representation"]["coordinate_system"]["unit"],
        "centipoint"
    );
    assert_eq!(
        r["representation"]["coordinate_system"]["origin"],
        "top-left"
    );

    let projection = ethos_parser_grounding::project(&repr).expect("projects");
    let g = parse_canonical(
        &ethos_parser_grounding::to_canonical_bytes(&projection.source).expect("canonical"),
    );
    assert_eq!(g["coordinate_system"]["unit"], "centipoint");
    assert_eq!(g["coordinate_system"]["origin"], "top-left");

    // And the frame is part of identity: a profile that changed it would produce artifacts that
    // are correctly non-comparable rather than silently reinterpreted.
    let profile_json = serde_json::to_value(&profile).expect("profile serializes");
    assert_eq!(profile_json["coordinate_system"]["unit"], "centipoint");
    assert_eq!(profile_json["coordinate_system"]["origin"], "top-left");
}

/// Double-run byte identity holds at the library level too, not only across the binary.
#[test]
fn two_library_runs_produce_identical_bytes() {
    let profile = Profile::default();
    let source = std::fs::read(engine_fx("measured-ink-box")).expect("readable");

    let once = |()| -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let doc = Document::open_bytes(&source, &profile).expect("opens");
        let classification = ethos_parser_pdf::classify(&doc, &profile).expect("classifies");
        let extract = ethos_parser_pdf::extract(&doc, &profile).expect("extracts");
        let repr = ethos_parser_pdf::to_representation(&extract, &profile).expect("represents");
        let projection = ethos_parser_grounding::project(&repr).expect("projects");
        (
            classification.to_canonical_bytes().expect("canonical"),
            repr.to_canonical_bytes().expect("canonical"),
            ethos_parser_grounding::to_canonical_bytes(&projection.source).expect("canonical"),
        )
    };

    assert_eq!(
        once(()),
        once(()),
        "two library runs must agree byte for byte"
    );
}

/// A library caller gets the same named refusal the binary maps to exit 2.
#[test]
fn the_library_refuses_what_the_binary_refuses() {
    let profile = Profile::default();

    // A malformation v0.1's bounded repair does NOT cover. `table-regular-grid` used to sit here
    // and now opens (docs/01-CONTRACT.md §12); this one still refuses, which is what keeps the
    // repair from having quietly become general recovery.
    let hostile =
        std::fs::read(conformance("failure/corrupt-header-valid/document.pdf")).expect("readable");
    let e = Document::open_bytes(&hostile, &profile).expect_err("must refuse");
    assert_eq!(e.code(), "malformed");
    assert_eq!(
        ethos_parser_pdf::exit::exit_code(&Err(e)),
        ethos_parser_pdf::exit::COULD_NOT_READ,
        "the library's own mapping sends this to 2, without the CLI deciding"
    );

    // And the three open failures stay distinguishable in-process.
    let codes: Vec<&str> = [
        "failure/password-protected/document.pdf",
        "failure/corrupt-header-valid/document.pdf",
        "failure/invalid-header/document.pdf",
    ]
    .iter()
    .map(|rel| {
        let bytes = std::fs::read(conformance(rel)).expect("readable");
        Document::open_bytes(&bytes, &profile)
            .err()
            .unwrap_or_else(|| panic!("{rel} must fail"))
            .code()
    })
    .collect();
    assert!(
        codes.contains(&"encrypted") && codes.contains(&"malformed"),
        "the taxonomy must stay distinguishable to a library caller: {codes:?}"
    );
}

// -------------------------------------------------------------------------------------------
// Guard the guard
// -------------------------------------------------------------------------------------------

/// **This file must not spawn the binary.**
///
/// The whole claim is "without the CLI". A `Command::new(env!("CARGO_BIN_EXE_ethos-parser"))` added
/// here later would keep every assertion above passing while quietly destroying what they prove.
#[test]
fn no_test_in_this_file_spawns_the_binary() {
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/library_surface.rs"),
    )
    .expect("this file is readable");

    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    // Split so the needles do not match this very list. The obvious version of this test fails
    // on itself, which is a fine way to discover that the scan is real.
    let banned = [
        concat!("CARGO_BIN", "_EXE"),
        concat!("std::process", "::Command"),
        concat!("Command", "::new"),
    ];

    for banned in banned {
        assert!(
            !code.contains(banned),
            "`{banned}` appears in library_surface.rs. These tests prove the library reaches \
             every subcommand behaviour on its own; spawning the binary here would make them \
             prove the opposite of what they claim."
        );
    }
}
