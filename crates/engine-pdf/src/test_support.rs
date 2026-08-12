// Copyright 2026 The ethos-engine maintainers
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

//! Fixture resolution for tests, matching the oracle harness (`docs/04-ARCHITECTURE.md` §4).
//!
//! Two roots, each independently overridable, resolved from `fixtures/manifest.json` exactly as
//! `crates/engine-cli/tests/oracle.rs` resolves them. **A missing corpus is a failure, never a
//! skip** — a harness that skips reports green on a machine where it never ran.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/crates/engine-pdf
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("manifest dir has two ancestors")
        .to_path_buf()
}

fn manifest() -> serde_json::Value {
    let p = repo_root().join("fixtures/manifest.json");
    let bytes = std::fs::read(&p)
        .unwrap_or_else(|e| panic!("fixture manifest unreadable at {}: {e}", p.display()));
    serde_json::from_slice(&bytes).expect("fixture manifest is valid JSON")
}

/// Resolve a declared corpus root by name, honouring its environment override.
fn root(name: &str) -> PathBuf {
    let m = manifest();
    let decl = &m["roots"][name];
    assert!(!decl.is_null(), "manifest declares no root `{name}`");

    if let Some(var) = decl["env"].as_str() {
        if let Ok(v) = std::env::var(var) {
            return PathBuf::from(v);
        }
    }
    repo_root().join(decl["default"].as_str().expect("root declares a default"))
}

fn read(root_name: &str, rel: &str) -> Vec<u8> {
    let path = root(root_name).join(rel);
    std::fs::read(&path).unwrap_or_else(|e| {
        let env = manifest()["roots"][root_name]["env"]
            .as_str()
            .unwrap_or("(none)")
            .to_string();
        panic!(
            "fixture `{rel}` not found in the `{root_name}` corpus at {}: {e}\n\
             Set {env} to override. Corpora are read-only and never copied into this repo; a \
             missing corpus is a failure, never a skip.",
            path.display()
        )
    })
}

/// Read a fixture from the 15-fixture conformance corpus.
pub fn conformance_fixture(rel: &str) -> Vec<u8> {
    read("conformance", rel)
}

/// Read a document from the benchmark corpus (the large real-world PDFs M2 names).
pub fn bench_fixture(rel: &str) -> Vec<u8> {
    read("benchmark", rel)
}

/// Read a fixture from the engine-owned CC0 set (`fixtures/engine`).
pub fn engine_fixture(rel: &str) -> Vec<u8> {
    read("engine", rel)
}
