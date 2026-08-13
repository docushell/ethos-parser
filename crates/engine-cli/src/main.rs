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
//! Four subcommands, all implemented as of v0: `classify`, `extract`, `ground`,
//! `grounding-check`. The CLI is a **thin shell** over the library so the two cannot diverge: it
//! parses arguments, opens the document once, calls the library, prints canonical bytes, and maps
//! the result to an exit code. No classification, extraction, projection or validation logic lives
//! here, and `docs/PUBLIC-API.md` names the library entry point behind each subcommand.
//!
//! # stdout is the artifact
//!
//! Every subcommand writes canonical bytes to stdout and nothing else. Failures, the grounding
//! omission note, and `--diagnostics` all go to stderr, so a caller redirecting stdout to a file
//! gets an artifact whose bytes are identical across runs (`docs/04-ARCHITECTURE.md` §2).

#![forbid(unsafe_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use engine_core::diagnostics::{DiagnosticsRun, Stage};
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

    /// Write volatile run diagnostics — timing, host, input path, resident memory — to stderr.
    ///
    /// Off by default, and it changes nothing on stdout. Diagnostics are the only place volatile
    /// data is allowed to exist (`docs/01-CONTRACT.md` §4): they are never canonicalized, never
    /// fingerprinted, and never part of an artifact. Turning this on does not make two runs
    /// produce different artifacts — that is the property being protected.
    #[arg(long, global = true)]
    diagnostics: bool,
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

    /// Project a `DocumentRepresentation v0` into `ethos.grounding.v1`.
    ///
    /// Exit codes: 0 projected, 2 could not read or refused. Nodes with no measurable ink box
    /// are omitted from the artifact and reported on stderr — the grounding schema is
    /// `additionalProperties: false` and cannot carry the count, so the representation it came
    /// from is where the declaration lives.
    Ground(GroundArgs),

    /// Validate a grounding artifact: structure, and optionally its binding to source bytes.
    ///
    /// **Structure and binding only** — no claims, no verdict, no `grounded`, no evidence tier.
    /// The engine validates; it never verifies (`docs/07-VERIFY-BOUNDARY.md`).
    ///
    /// Exit codes: **0** valid (and matched, or not checked) · **1** invalid structure, or a
    /// source that does not bind · **2** the input could not be read at all. Ethos returns 2 for
    /// both of the last two; the engine keeps them apart, because a caller that cannot tell a
    /// failing check from an unreadable file is the defect this project refuses. Both agree on
    /// zero versus non-zero, which is what a shell predicate reads.
    GroundingCheck(GroundingCheckArgs),
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
struct GroundingCheckArgs {
    /// The `ethos.grounding.v1` JSON to validate.
    path: PathBuf,

    /// The PDF the artifact claims to describe.
    ///
    /// Supplying it turns `source_binding` from `not_checked` into `matched` or `mismatched`.
    /// Omitting it is not a pass — it is a question that was not asked. Named to match the Ethos
    /// CLI so the oracle harness reads the same either way.
    #[arg(long, value_name = "PDF")]
    source_artifact: Option<PathBuf>,
}

#[derive(clap::Args)]
struct GroundArgs {
    /// A `DocumentRepresentation v0` JSON file, as `engine extract` emits.
    path: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let diag = cli.diagnostics;
    match cli.command {
        Command::Classify(args) => {
            let path = args.path.clone();
            timed(Stage::Classify, diag, &path, || run_classify(args))
        }
        Command::Extract(args) => {
            let path = args.path.clone();
            timed(Stage::Extract, diag, &path, || run_extract(args))
        }
        Command::Ground(args) => {
            let path = args.path.clone();
            timed(Stage::Ground, diag, &path, || run_ground(args))
        }
        Command::GroundingCheck(args) => {
            let path = args.path.clone();
            timed(Stage::GroundingCheck, diag, &path, || {
                run_grounding_check(args)
            })
        }
    }
}

/// Run a subcommand, and — only when asked — describe the run on stderr.
///
/// The wrapper exists so the timing region is the *whole* subcommand including its output write,
/// and so no early return inside a subcommand can skip the report. The exit code is passed
/// through untouched: a diagnostics failure must never change what a caller's `&&` chain sees,
/// because then observing the engine would change it.
///
/// Everything reported is assembled by `engine_core::diagnostics` — this function measures a
/// region and chooses a stream, which is the whole of what a shell is allowed to do.
fn timed(stage: Stage, enabled: bool, path: &Path, run: impl FnOnce() -> ExitCode) -> ExitCode {
    let observation = DiagnosticsRun::begin(stage);
    let code = run();
    if enabled {
        // Read after the run, not before: for a subcommand that failed to open the file this is
        // the only size available, and asking twice would be one more thing to keep in agreement.
        let bytes = std::fs::metadata(path).ok().map(|m| m.len());
        let d = observation.finish(Some(path.display().to_string()), bytes);
        match d.to_json_line() {
            Ok(line) => {
                let mut err = std::io::stderr().lock();
                let _ = err.write_all(&line);
                let _ = err.flush();
            }
            // Reported, not fatal, and not on stdout. A diagnostic that could fail a run would be
            // a reason not to turn diagnostics on.
            Err(e) => eprintln!("engine: diagnostics unavailable: {e} [{}]", e.code()),
        }
    }
    code
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
    //
    // The happy-path output is the REPRESENTATION, not the stage artifact: `05-MILESTONES.md`
    // M5 makes `DocumentRepresentation v0` the canonical record, and it is what `engine ground`
    // consumes. The stage artifact remains the library's return type, so M3's acceptance suite
    // still asserts on the thing the parser actually produces.
    let result = Document::open(&args.path, &profile)
        .and_then(|doc| engine_pdf::extract(&doc, &profile))
        .and_then(|extract| engine_pdf::to_representation(&extract, &profile));

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

/// Read a representation from disk and project it.
///
/// Thin, like every other subcommand: it reads bytes, calls the library, prints canonical bytes,
/// and maps the outcome to an exit code. The projection itself lives in `engine-grounding`.
fn run_ground(args: GroundArgs) -> ExitCode {
    let bytes = match std::fs::read(&args.path) {
        Ok(b) => b,
        Err(e) => {
            return fail(&EngineError::Io {
                detail: format!("{}: {e}", args.path.display()),
            })
        }
    };

    let repr: engine_core::DocumentRepresentation = match serde_json::from_slice(&bytes) {
        Ok(r) => r,
        Err(e) => {
            return fail(&EngineError::Malformed {
                what: "representation".into(),
                detail: e.to_string(),
            })
        }
    };

    // The fingerprint is checked before anything is projected. A representation whose payload
    // does not hash to its declared digest is not a record this engine will speak for, and
    // projecting it anyway would launder the disagreement into a fresh-looking artifact.
    if let Err(e) = repr.verify_fingerprint() {
        return fail(&e);
    }

    let projection = match engine_grounding::project(&repr) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };

    match engine_grounding::to_canonical_bytes(&projection.source) {
        Ok(out) => {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(&out);
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();

            // On stderr, deliberately: stdout is the artifact and must stay byte-identical
            // across runs. A consumer that wants this durably reads the representation's
            // `geometry-absent-not-groundable` limitation, which carries the same count.
            if projection.omission.is_lossy() {
                eprintln!(
                    "engine: {} of {} node(s) omitted from the grounding artifact — no measurable \
                     ink box [{}]. The nodes remain in the representation with their text and \
                     native locators; the grounding schema requires a bbox and this engine does \
                     not fabricate one.",
                    projection.omission.nodes_omitted,
                    projection.omission.nodes_total,
                    projection.omission.limitation_code,
                );
            }
            ExitCode::from(PROJECTED as u8)
        }
        Err(e) => fail(&e),
    }
}

/// Projection succeeded.
const PROJECTED: i32 = 0;

/// Validate a grounding artifact and print the report.
///
/// Thin, like the rest: read bytes, call the library, print canonical bytes, map the outcome to
/// an exit code. **The report is printed on every outcome the checker can describe**, including
/// the failing ones — a caller that gets a non-zero exit and no report has to guess why, and the
/// report is the machine-readable part. Only an input that could not be read at all produces no
/// report, because there is nothing to report about.
fn run_grounding_check(args: GroundingCheckArgs) -> ExitCode {
    let grounding = match std::fs::read(&args.path) {
        Ok(b) => b,
        Err(e) => {
            return fail(&EngineError::Io {
                detail: format!("{}: {e}", args.path.display()),
            })
        }
    };

    let source = match &args.source_artifact {
        None => None,
        Some(p) => match std::fs::read(p) {
            Ok(b) => Some(b),
            Err(e) => {
                return fail(&EngineError::Io {
                    detail: format!("{}: {e}", p.display()),
                })
            }
        },
    };

    let report = match engine_grounding::grounding_check(&grounding, source.as_deref()) {
        Ok(r) => r,
        // A non-PDF source, or an input past the accepted ceiling. Ethos refuses these before
        // writing any report and so does this: "these bytes are not the source" would be a
        // different and wrong statement about a file that is not a document at all.
        Err(e) => return fail(&e),
    };

    match report.to_canonical_bytes() {
        Ok(bytes) => {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(&bytes);
            let _ = out.write_all(b"\n");
            let _ = out.flush();
            ExitCode::from(report.exit_code() as u8)
        }
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
