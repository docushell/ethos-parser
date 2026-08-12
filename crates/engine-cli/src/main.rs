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

//! `engine` — the ethos-engine command line.
//!
//! Four subcommands at v0: `classify`, `extract`, `ground`, `grounding-check`
//! (`docs/04-ARCHITECTURE.md` §2). The CLI is a thin shell over the library so the two cannot
//! diverge: it parses arguments, calls the library, and maps errors to exit codes.
//!
//! **Status: M0 skeleton.** No subcommand is implemented. Every invocation fails closed with
//! exit code 2 and a named reason — never a panic, never a `todo!()`, never a silent success.

#![forbid(unsafe_code)]

use std::io::Write;

/// Simple: no reason codes fired.
pub const EXIT_SIMPLE: i32 = 0;
/// Needs attention: the document was read, and at least one reason code fired.
pub const EXIT_NEEDS_ATTENTION: i32 = 1;
/// Could not read: the document did not open, or a fail-closed rule triggered.
pub const EXIT_COULD_NOT_READ: i32 = 2;

fn main() -> std::process::ExitCode {
    let mut err = std::io::stderr();
    let _ = writeln!(
        err,
        "engine: not implemented at milestone M0.\n\
         \n\
         M0 is the repo skeleton: toolchain, dependency policy, fixture manifest, and a\n\
         deliberately failing oracle harness. The contract types land at M1, classification at\n\
         M2, extraction at M3. See docs/05-MILESTONES.md.\n\
         \n\
         Exiting {EXIT_COULD_NOT_READ} (could-not-read), because failing closed with a named\n\
         reason is the only honest thing an unimplemented parser can do."
    );
    std::process::ExitCode::from(EXIT_COULD_NOT_READ as u8)
}
