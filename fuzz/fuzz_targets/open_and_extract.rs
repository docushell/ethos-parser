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

//! The deep path: open, extract, represent, seal.
//!
//! A second target rather than a branch in the first, because libFuzzer's coverage feedback is
//! per-target: one binary that sometimes classifies and sometimes extracts splits its corpus
//! between two very different code paths and explores both worse than either alone.
//!
//! This is the target that reaches the content-stream interpreter, the CMap and encoding tables,
//! font-metric resolution and geometry quantization — the parts that do arithmetic on numbers a
//! document supplies. As in the sibling target, an `EngineError` is a pass and a panic is not.

#![no_main]

use libfuzzer_sys::fuzz_target;

use ethos_parser_core::Profile;
use ethos_parser_pdf::Document;

fuzz_target!(|data: &[u8]| {
    let profile = Profile::default();

    if let Ok(doc) = Document::open_bytes(data, &profile) {
        if let Ok(extract) = ethos_parser_pdf::extract(&doc, &profile) {
            if let Ok(repr) = ethos_parser_pdf::to_representation(&extract, &profile) {
                // Sealing hashes the payload and canonicalizes it. Verifying the fingerprint the
                // engine just computed is nearly free and turns "it produced something" into
                // "it produced something internally consistent".
                let _ = repr.verify_fingerprint();
                let _ = repr.to_canonical_bytes();
            }
        }
    }
});
