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

//! The PDF entry point: arbitrary bytes into `Document::open_bytes`, then `classify`.
//!
//! **An `EngineError` is a pass.** The engine is expected to refuse almost everything the fuzzer
//! produces — that is what fail-closed means. The only failure this target can report is a
//! **panic**, which is a release blocker (`docs/05-MILESTONES.md` M7).
//!
//! `classify` runs on whatever opens, because the bug class worth hunting lives past the parse:
//! an index into a page list, a slice of a content stream, an arithmetic conversion on a number
//! a real document would never carry. `open_bytes` alone would leave all of it unreached.

#![no_main]

use libfuzzer_sys::fuzz_target;

use ethos_parser_core::Profile;
use ethos_parser_pdf::Document;

fuzz_target!(|data: &[u8]| {
    // The default profile, so what is fuzzed is what ships. A profile with the sample count
    // raised would explore more pages per input and hide behind a knob no caller uses.
    let profile = Profile::default();

    if let Ok(doc) = Document::open_bytes(data, &profile) {
        if let Ok(classification) = ethos_parser_pdf::classify(&doc, &profile) {
            // Canonicalization is on the path too: it is where integers are range-checked and
            // where a value that survived the parser can still be rejected. Skipping it would
            // leave the last gate in the chain unfuzzed.
            let _ = classification.to_canonical_bytes();
        }
    }
});
