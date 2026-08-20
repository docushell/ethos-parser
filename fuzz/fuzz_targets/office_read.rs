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

//! The office router: arbitrary bytes into `engine_office::read`.
//!
//! **The obligation.** `06-STEAL-REFUSE.md`'s **A11** — *mutation testing every fixture +
//! `cargo-fuzz` per format*, sourced from Anydoc and due at **v0**. v2-S2 deferred the office half
//! in one clause: *"a cargo-fuzz campaign — `A11`'s mutation lane for this format waits for a
//! second one"*. There are now eight formats and no office byte had ever been fuzzed.
//!
//! **It was not hypothetical.** v2-S9's first adversarial finding was a **panic** in the
//! percent-decoder, reachable from any `href` in a crafted package document, and the review record
//! says it survived to review *precisely because office code is unfuzzed*. A reader whose entire
//! contract is a named refusal must not have an input that takes the process down.
//!
//! # The oracle
//!
//! **No panic. That is the whole of it, and the type system supplies the rest.**
//!
//! `read` returns `Result<DocumentRepresentation, EngineError>`, so *"every failure is a named
//! `EngineError`"* is not something this target can check — it is something the signature makes
//! true. A `Malformed`, `Unsupported`, `MissingPart` or `ResourceLimit` is a **pass**; almost
//! everything the fuzzer produces should be one, because fail-closed is the design. What is a
//! failure: an unwrap, an index out of bounds, an arithmetic overflow in a debug build, an
//! allocation the resource limits should have refused, or a hang.
//!
//! # One target, not eight
//!
//! `read` is the single entry point every format shares and the one `engine extract` calls, so a
//! corpus seeded with one valid package of each shape reaches every reader through it. Eight
//! harnesses would divide that corpus eight ways and explore each branch on a fraction of the
//! budget, which is libFuzzer's coverage feedback working against itself. A format measured
//! unreachable from here is the argument for splitting one out; the shape of the module tree is
//! not.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // No profile argument, unlike the two PDF targets: `read` selects the profile from what the
    // package turns out to be, which is the behaviour under test. A caller cannot pass one in and
    // neither can this.
    if let Ok(repr) = engine_office::read(data) {
        // Sealing already happened inside `read`. Re-deriving the fingerprint and canonicalizing
        // are nearly free and turn "it produced something" into "it produced something internally
        // consistent" — and they put the c14n encoder on the path, which is where a string a
        // package supplied is range-checked one last time.
        let _ = repr.verify_fingerprint();
        let _ = repr.to_canonical_bytes();
    }
});
