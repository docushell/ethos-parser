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
//! Four subcommands at v0; `classify` is implemented as of M2. The CLI is a **thin shell** over
//! the library so the two cannot diverge: it parses arguments, opens the document once, calls
//! `engine_pdf`, prints canonical bytes, and maps the result to an exit code. No classification
//! logic lives here.

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use engine_core::{EngineError, Profile};
use engine_pdf::exit::{exit_code, COULD_NOT_READ};
use engine_pdf::Document;

#[derive(Parser)]
#[command(
    name = "engine",
    about = "Deterministic PDF classification and evidence extraction",
    long_about = None,
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report what a document contains: counts and named reason codes, never a verdict.
    ///
    /// Exit codes: 0 simple, 1 needs attention, 2 could not read.
    Classify(ClassifyArgs),

    /// Extract position-aware text runs with native locators.
    ///
    /// Exit codes: 0 extracted, 2 could not extract. There is no exit 1 here — "needs
    /// attention" is a classify concept, and overloading it would make a caller's `&&` chain
    /// mean two different things depending on which subcommand ran.
    Extract(ExtractArgs),

    /// Project a representation into `ethos.grounding.v1`. **Not implemented until M5.**
    Ground(PathArg),

    /// Validate a grounding artifact against its schema and source bytes.
    /// **Not implemented until M6.**
    GroundingCheck(PathArg),
}

#[derive(clap::Args)]
struct ClassifyArgs {
    /// The PDF to classify.
    path: PathBuf,

    /// Override how many pages are sampled. Defaults to the profile's value (8).
    ///
    /// Changing this changes `profile_sha256`, so artifacts produced at different sample counts
    /// are correctly non-comparable.
    #[arg(long, value_name = "N")]
    sample_pages: Option<u32>,
}

#[derive(clap::Args)]
struct ExtractArgs {
    /// The PDF to extract.
    path: PathBuf,
}

#[derive(clap::Args)]
struct PathArg {
    /// Input path.
    #[arg(value_name = "PATH")]
    _path: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Classify(args) => run_classify(args),
        Command::Extract(args) => run_extract(args),
        Command::Ground(_) => not_implemented("ground", "M5"),
        Command::GroundingCheck(_) => not_implemented("grounding-check", "M6"),
    }
}

fn run_classify(args: ClassifyArgs) -> ExitCode {
    let mut profile = Profile::default();
    if let Some(n) = args.sample_pages {
        profile.classify_sample_pages = n;
    }

    // Opened once. M3's `extract` will take this same handle rather than reopening
    // (docs/04-ARCHITECTURE.md §2.1) — two loads can disagree, and a classifier that saw a
    // different object graph from the extractor is a silent divergence with no diagnostic.
    let result =
        Document::open(&args.path, &profile).and_then(|doc| engine_pdf::classify(&doc, &profile));

    match &result {
        Ok(classification) => match classification.to_canonical_bytes() {
            Ok(bytes) => {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(&bytes);
                let _ = out.write_all(b"\n");
                let _ = out.flush();
                ExitCode::from(exit_code(&result) as u8)
            }
            Err(e) => fail(&e),
        },
        Err(e) => fail(e),
    }
}

fn run_extract(args: ExtractArgs) -> ExitCode {
    let profile = Profile::default();

    // Opened once, exactly as `classify` opens it. The same handle serves both stages
    // (docs/04-ARCHITECTURE.md §2.1); nothing below the CLI opens a file.
    let result =
        Document::open(&args.path, &profile).and_then(|doc| engine_pdf::extract(&doc, &profile));

    match result {
        Ok(artifact) => match artifact.to_canonical_bytes() {
            Ok(bytes) => {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(&bytes);
                let _ = out.write_all(b"\n");
                let _ = out.flush();
                ExitCode::from(EXTRACTED as u8)
            }
            Err(e) => fail(&e),
        },
        Err(e) => fail(&e),
    }
}

/// Extraction succeeded.
///
/// Deliberately not reusing `SIMPLE`: exit 0 means different things for the two subcommands, and
/// naming them separately keeps that visible.
const EXTRACTED: i32 = 0;

/// Report a failure on stderr and exit 2.
///
/// The artifact never appears on stdout in this path: a partial or absent classification must not
/// be mistaken for a real one by something reading the pipe.
fn fail(e: &EngineError) -> ExitCode {
    eprintln!("engine: {} [{}]", e, e.code());
    ExitCode::from(COULD_NOT_READ as u8)
}

/// A subcommand that exists in the surface but has no implementation yet.
///
/// Exits 2 (could-not-read) rather than 0, and says which milestone owns it. Printing usage and
/// exiting 0 would let a script conclude the work happened.
fn not_implemented(name: &str, milestone: &str) -> ExitCode {
    eprintln!(
        "engine: `{name}` is not implemented — it lands at {milestone}.\n\
         See docs/05-MILESTONES.md. Exiting {COULD_NOT_READ} (could-not-read) rather than \
         pretending the work happened."
    );
    ExitCode::from(COULD_NOT_READ as u8)
}
