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

//! `ethos-parser` — the ethos-parser command line.
//!
//! **Four subcommands at v0. Nine now**, and this paragraph said four until v2-S13.5. The v0 four
//! are `classify`, `extract`, `ground` and `grounding-check`; `verify` arrived at v0.1, `markdown`
//! at v1.1-S1, `html` at v1.1-S4, `mcp` at v1.2-S1 and `overlay` with the image work. The `Command`
//! enum below is the list that cannot go stale, and `crates/ethos-parser-core/src/verifier.rs` has said
//! *"the other eight subcommands"* since v0.1 — two files in one workspace disagreeing about a
//! number a reader can count is exactly what `docs/04-ARCHITECTURE.md` §2 repaired at v2-S13.3 and
//! this one was missed by.
//!
//! The CLI is a **thin shell** over the library so the two cannot diverge: it
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

mod mcp;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use ethos_parser_core::diagnostics::{DiagnosticsRun, Stage};
use ethos_parser_core::verifier::{RelayRequest, VerifierBinary};
use ethos_parser_core::{EngineError, Profile};
use ethos_parser_pdf::exit::{exit_code, COULD_NOT_READ};
use ethos_parser_pdf::Document;

#[derive(Parser)]
#[command(
    name = "ethos-parser",
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
    /// from is where the declaration lives. What the schema's own limits will not hold is reported
    /// on stderr, and spans and tables also through the artifact's `capabilities`: an element whose
    /// text or page-less locator is too long is omitted, too many tables or a cell or grid too
    /// large withholds the tables, too many spans withholds the spans, and a document with more
    /// pages or elements than the schema admits — or an artifact larger than a verifier accepts —
    /// is refused with exit 2 and no artifact.
    Ground(GroundArgs),

    /// Project a `DocumentRepresentation v0` into `ethos.markdown.v1` (v1.1-S1).
    ///
    /// **Markdown and its Anchor Map, always together.** They are fields of one artifact, not two
    /// files, so there is no `--md-only` and no way for a caller to end up with a Markdown string
    /// whose bytes cannot be inverted back to evidence. `docs/01-CONTRACT.md` §12 refused a
    /// Markdown projection for the whole of v1 on Workbench rule 8 — a projection between what a
    /// retriever ranks and what a citation binds is where a locator dies silently — and checklist
    /// O8 records that the rule prefers no projection at all to one without the map. This
    /// subcommand exists because the map makes the objection payable, not because it lapsed.
    ///
    /// stdout is canonical JSON. A consumer that wants a `.md` file writes
    /// `.markdown` out itself, and owns the fact that doing so discards the map.
    ///
    /// Exit codes: **0** projected · **2** could not read, or the representation does not hash to
    /// its declared digest.
    Markdown(MarkdownArgs),

    /// Project a `DocumentRepresentation v0` into `ethos.html.v1` (v1.1-S4).
    ///
    /// **HTML and its Anchor Map, always together**, on exactly the discipline `markdown` runs
    /// under: one artifact, two segment kinds, a map that tiles every byte, and a census that
    /// accounts for every source character. There is no `--html-only`.
    ///
    /// **Not a rendering of the Markdown.** It is projected from the representation, because a
    /// Markdown-to-HTML pass would be a second projection whose map nobody built. The difference
    /// that earns it a subcommand is tables: GFM cannot say `rowspan`, so `markdown` expands a
    /// merged cell and counts what that cost, while this emits one `<td colspan="2">` and carries
    /// the merge the document drew.
    ///
    /// stdout is canonical JSON. A consumer that wants an `.html` file writes `.html` out itself,
    /// and owns the fact that doing so discards the map.
    ///
    /// Exit codes: **0** projected · **2** could not read, or the representation does not hash to
    /// its declared digest.
    Html(HtmlArgs),

    /// Serve the engine over MCP on stdin/stdout (v1.2-S1).
    ///
    /// **stdio, newline-delimited JSON-RPC** — MCP's own stdio transport, so it is a pipe rather
    /// than a socket: no HTTP, no SSE, no TLS, no async runtime, and `deny.toml`'s network bans
    /// stay in force.
    ///
    /// Three tools — `extract`, `ground`, `node_get` — each calling the same library entry point
    /// the matching subcommand calls, so an artifact returned here is the artifact this CLI
    /// prints, byte for byte.
    ///
    /// **No tool accepts a locator.** The memo's §16.7 hazard is that MCP tools are
    /// model-controlled, so a tool taking a `page` or a `bbox` the engine then trusts makes the
    /// model the citation authority in one step. Every locator is minted by the engine inside an
    /// artifact, travels back as an opaque handle, and is re-validated against that artifact on
    /// the way in — a handle this engine did not mint is an error, never a best guess. See
    /// `docs/history/12-V12-SCOPE.md` §3.
    ///
    /// Exit codes: **0** the stream closed cleanly · **2** stdin or stdout failed.
    Mcp,

    /// Draw what was detected onto a copy of the document (v1-S6).
    ///
    /// Emits a PDF — **the one subcommand whose stdout is not canonical JSON** — carrying an
    /// annotation over every table box, image placement and flagged run the extract found, plus a
    /// per-page note counting what was found and what has no rectangle to draw. That last part is
    /// the point: an overlay that drew only the boxes it had would make a partly-read document
    /// look fully read.
    ///
    /// **It annotates; it does not edit.** No content stream is touched, no text is removed, and
    /// the document's own annotations are kept. This is not a redaction tool.
    ///
    /// Exit codes: **0** the overlay was written · **2** the document could not be read.
    Overlay(OverlayArgs),

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

    /// Relay a citation-verification run to the pinned Ethos CLI.
    ///
    /// **The engine does not verify — it invokes a verifier** (`docs/07-VERIFY-BOUNDARY.md`
    /// Stage 1). This spawns `ethos verify`, forwards its report bytes to stdout **verbatim**,
    /// and maps its exit status. Nothing here reads the report: no field is re-computed, no
    /// verdict is formed, and the engine has no opinion about whether a claim is supported.
    ///
    /// Exit codes: **0** the verifier was satisfied · **1** it was not, and you asked it to say
    /// so with `--fail-on-ungrounded` · **2** the run did not happen — no verifier, a spawn
    /// failure, or a usage error the verifier itself refused. **2 means no report was produced**,
    /// and it is kept apart from 1 for the reason the other subcommands keep it apart: a caller
    /// must be able to tell "the check failed" from "the check did not run".
    ///
    /// The verifier is located the way the oracle harness locates it: `ETHOS_BIN` first and
    /// authoritatively, then the sibling repo build, then `PATH`. Absence is a named error,
    /// never a skip and never a default-pass.
    Verify(VerifyArgs),
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

    /// Process at most N pages; the rest are quarantined and declared. Unbounded by default.
    ///
    /// **This is the only bound a caller has on how much memory one extract costs**, and until
    /// v2-S15 there was none. Peak resident memory tracks PAGE COUNT rather than file size —
    /// 3.0 to 6.6 MiB per page across the gate corpus. `nist-sp-800-171r3` is 120 pages and
    /// 1.5 MB and peaks at 404 MiB; `nist-sp-800-37r2` is 1.4x the file at 2.2 MB but 1.5x the
    /// pages, and peaks at 915 MiB — 2.3x. Every page's extract is retained because it IS the
    /// artifact, so the only thing that bounds the cost is admitting fewer pages. A host handing
    /// this untrusted input could not previously do that: `page_budget` defaults to `Unlimited`
    /// and nothing on this command could lower it.
    ///
    /// **It does not bound everything.** `--max-pages 0` on a 733-page document still costs
    /// 222 MiB, because the structure tree is read over the whole document before the budget is
    /// consulted. For sizing, budget ~7 MiB per admitted page plus ~0.35 MiB per page in the
    /// document; on the corpus's worst case that over-predicts by 44%, which is the safe
    /// direction. Readings and instruments: `docs/measurements/memory-ceiling/`.
    ///
    /// The pages left out are not silently dropped. Each is quarantined with
    /// `resource_limit_pages` and the artifact declares the limitation, which is the same
    /// machinery `PageBudget::AtMost` has always driven — this flag only lets a caller reach it.
    ///
    /// Changing this changes `profile_sha256`, exactly as `classify --sample-pages` does, so a
    /// bounded artifact and an unbounded one are correctly non-comparable rather than quietly
    /// different.
    #[arg(long, value_name = "N")]
    max_pages: Option<u32>,
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
struct VerifyArgs {
    /// The `ethos.grounding.v1` artifact, as `ethos-parser ground` emits it.
    path: PathBuf,

    /// Citations file (JSON): an array of claims, or `{"document_fingerprint": …, "claims": […]}`.
    ///
    /// Named to match the Ethos CLI, like `--source-artifact` on `grounding-check`, so the two
    /// invocations read the same and a caller can compare them directly.
    #[arg(long, value_name = "FILE")]
    citations: PathBuf,

    /// Exit 1 after writing the report when the verifier is not satisfied.
    ///
    /// Without it the verifier writes its report and exits 0 whatever it found, and the engine
    /// forwards that unchanged — the report is still produced, so an ungrounded claim is never a
    /// silent skip either way. With it, a shell predicate can gate on the outcome.
    #[arg(long)]
    fail_on_ungrounded: bool,

    /// Verification config (JSON). Forwarded unread.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Write the report here instead of stdout. Forwarded unread.
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,
}

#[derive(clap::Args)]
struct HtmlArgs {
    /// A `DocumentRepresentation v0` JSON file, as `ethos-parser extract` emits.
    ///
    /// **A representation, not a PDF**, and not a Markdown artifact either — see the subcommand's
    /// own documentation for why this projects from the record rather than from the other
    /// projection.
    path: PathBuf,
}

#[derive(clap::Args)]
struct MarkdownArgs {
    /// A `DocumentRepresentation v0` JSON file, as `ethos-parser extract` emits.
    ///
    /// **A representation, not a PDF.** The same input `ethos-parser ground` takes, deliberately: one
    /// subcommand that silently means two different things is how a caller ends up unsure which
    /// profile produced the artifact it is holding.
    path: PathBuf,
}

#[derive(clap::Args)]
struct GroundArgs {
    /// A `DocumentRepresentation v0` JSON file, as `ethos-parser extract` emits.
    path: PathBuf,
}

#[derive(clap::Args)]
struct OverlayArgs {
    /// The PDF to annotate.
    path: PathBuf,
}

/// A ceiling on the bytes one invocation will read off disk.
///
/// **There was none** until v2-S15: every entry point called `std::fs::read` on a caller-supplied
/// path, so the whole file was resident before any check beyond the five-byte magic number ran.
/// The office crate has had `zip::MAX_INFLATED_BYTES` since v2-S13 for exactly this reason; the
/// path that reads the file in the first place had nothing.
///
/// 2 GiB is far above any document this engine is meant for and far below "whatever the caller
/// names". For a regular file it is a refusal by name rather than an allocation on the caller's
/// behalf, which is the difference between a diagnosable exit 2 and an OOM kill with no stderr; a
/// source with no size is refused by the bounded read, having held up to the ceiling.
///
/// This bounds the SOURCE. The dominant cost of an extract is not the file — peak memory tracks
/// page count at 3.0 to 6.6 MiB per page, which is what `extract --max-pages` exists to bound.
/// The two ceilings are complementary and neither subsumes the other.
///
/// They also do not COMPOSE into a memory bound, which is worth stating plainly: nothing maps a
/// permitted 2 GiB input onto a peak-memory figure, and the page budget leaves a floor that grows
/// with the document's page count. A caller who needs a hard memory ceiling does not have one
/// today. See `docs/measurements/memory-ceiling/` §5.
pub(crate) const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Read a caller-supplied file, refusing one that is over [`MAX_SOURCE_BYTES`].
pub(crate) fn read_source(path: &std::path::Path) -> Result<Vec<u8>, EngineError> {
    read_source_within(path, MAX_SOURCE_BYTES)
}

/// Read a caller-supplied file, refusing one that is over `max` bytes.
///
/// `metadata` first, so an oversized file is refused without being read. **The read is bounded
/// too**, because `metadata` only knows a regular file's size: `/dev/zero`, a pipe or a device has
/// none, and was read without limit until memory ran out. Reading one byte past the ceiling is how
/// such a source is caught, after holding up to the ceiling. A regular file's buffer is reserved
/// from its length, fallibly, as `fs::read` reserved it.
fn read_source_within(path: &std::path::Path, max: u64) -> Result<Vec<u8>, EngineError> {
    use std::io::Read as _;
    let over = || EngineError::ResourceLimit {
        limit: format!("source bytes for `{}`", path.display()),
        configured: max.to_string(),
    };
    let io = |e: std::io::Error| EngineError::Io {
        detail: format!("{}: {e}", path.display()),
    };
    let size = std::fs::metadata(path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.len());
    if size.is_some_and(|len| len > max) {
        return Err(over());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size.unwrap_or(0) as usize)
        .map_err(|e| io(e.into()))?;
    std::fs::File::open(path)
        .and_then(|f| f.take(max + 1).read_to_end(&mut bytes))
        .map_err(io)?;
    if bytes.len() as u64 > max {
        return Err(over());
    }
    Ok(bytes)
}

/// Ethos's `max_file_bytes`, which caps `grounding check`'s `--source-artifact`: 256 MiB.
const GROUNDING_CHECK_MAX_SOURCE_BYTES: u64 = 256 * 1024 * 1024;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let diag = cli.diagnostics;
    match cli.command {
        Command::Overlay(args) => {
            let path = args.path.clone();
            timed(Stage::Extract, diag, &path, || run_overlay(args))
        }
        Command::Classify(args) => {
            let path = args.path.clone();
            timed(Stage::Classify, diag, &path, || run_classify(args))
        }
        Command::Extract(args) => {
            let path = args.path.clone();
            timed(Stage::Extract, diag, &path, || run_extract(args))
        }
        Command::Markdown(args) => {
            let path = args.path.clone();
            timed(Stage::Ground, diag, &path, || run_markdown(args))
        }
        Command::Html(args) => {
            let path = args.path.clone();
            timed(Stage::Ground, diag, &path, || run_html(args))
        }
        // Not `timed`: a server has no one input path and no one stage, and inventing a
        // diagnostics row for the whole session would put a duration on a pipe.
        Command::Mcp => run_mcp(),
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
        Command::Verify(args) => {
            let path = args.path.clone();
            timed(Stage::Verify, diag, &path, || run_verify(args))
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
/// Everything reported is assembled by `ethos_parser_core::diagnostics` — this function measures a
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
    let result = Document::open(&args.path, &profile)
        .and_then(|doc| ethos_parser_pdf::classify(&doc, &profile));

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

/// `ethos-parser overlay` — the annotated PDF (v1-S6).
///
/// The document is opened once and the extract taken from that same handle, so the overlay cannot
/// describe a different parse from the one `ethos-parser extract` would report. `build` re-checks the
/// binding on the digest anyway, because "the caller passed the right file" is an assumption and
/// the digest is a fact.
fn run_overlay(args: OverlayArgs) -> ExitCode {
    let profile = Profile::default();
    let result = Document::open(&args.path, &profile).and_then(|doc| {
        let extract = ethos_parser_pdf::extract(&doc, &profile)?;
        ethos_parser_pdf::build_overlay(&doc, &extract, &profile)
    });

    match result {
        Ok(bytes) => {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(&bytes);
            let _ = out.flush();
            ExitCode::from(EXTRACTED as u8)
        }
        Err(e) => fail(&e),
    }
}

fn run_extract(args: ExtractArgs) -> ExitCode {
    // **Dispatch by content, never by extension** (v2-S2, Anydoc's A4). A `.docx` renamed
    // `report.bin` still reads and a `.docx` full of something else does not, because an
    // extension is a claim anybody can make and a magic number is one only the file can.
    //
    // One subcommand and one artifact type, per `docs/history/14-V2-SCOPE.md` §4: there is no
    // `ethos.parser.docx.v0`, and every downstream path — c14n, fingerprint, `node_get` — is
    // unchanged. What differs is the profile the reader runs under, which is what makes the two
    // artifacts provably non-comparable.
    let head = match read_source(&args.path) {
        Ok(bytes) => bytes,
        Err(e) => return fail(&e),
    };
    // **One question, not six ordered ones** (v2-S3). Asking `is_docx` first and `is_xlsx`
    // second would make a package containing both main parts resolve to whichever line came
    // first; `ethos_parser_office::read` decides on the package's own evidence and refuses the
    // ambiguous case by name, so the answer does not depend on the order of this file. These
    // six `||`s only decide whether the office reader is the one to ask, and the v2-S10 branch
    // below them decides nothing about format at all — it is their negation.
    //
    // The fourth line is the ODF **family**, not one member of it (v2-S6). An OpenDocument package
    // declares its own type, so "this is OpenDocument" is knowable before "this is a kind we read"
    // — and asking the narrow question here is what used to send an `.odp` to the PDF reader, to
    // be refused for having no `%PDF-` header. Fail-closed, and naming the wrong cause.
    //
    // The fifth line is not a container question at all (v2-S8). An `.rtf` is a brace-group byte
    // stream that begins `{\rtf`; before this slice it answered `false` to every predicate here
    // and fell through to the PDF reader, to be refused for having no `%PDF-` header — fail-closed,
    // and naming the wrong cause, which is the same defect v2-S5 recorded for an `.ods`.
    //
    // And the last line is the **container**, not a format inside it (v2-S8). A ZIP is definitively
    // not a PDF, so sending one to the PDF reader can only produce a message about a missing
    // `%PDF-` header — which is what an `.epub` used to get. The office router's own refusal names
    // what the package is and is not, so an unread ZIP now fails closed for the cause it actually
    // has. This is the third time the same defect has been fixed for a different format, and it is
    // fixed here for the shape rather than for one more member of it.
    let mut profile = Profile::default();
    if let Some(n) = args.max_pages {
        profile.page_budget = ethos_parser_core::PageBudget::AtMost(n);
    }
    emit_representation(representation_for_bytes(&head, &profile))
}

/// Route bytes to the reader their own signatures name, and return the canonical
/// representation.
///
/// **The one place format dispatch lives** (0.38.0). Until this function existed
/// the routing was private to `run_extract`, and the MCP surface had quietly
/// reintroduced the wrong-cause refusal three CLI slices retired one format at a
/// time: `mcp extract` called the PDF reader directly, so a DOCX handed over MCP
/// was refused for lacking a `%PDF-` header — exactly the defect v2-S6 fixed for
/// an `.ods`, v2-S8 for an `.rtf`, and v2-S10 for untyped bytes. Both surfaces
/// now ask this function, so a fix here is a fix everywhere and the two can never
/// diverge again.
pub(crate) fn representation_for_bytes(
    head: &[u8],
    profile: &Profile,
) -> Result<ethos_parser_core::DocumentRepresentation, EngineError> {
    if ethos_parser_office::is_docx(head)
        || ethos_parser_office::is_xlsx(head)
        || ethos_parser_office::is_pptx(head)
        || ethos_parser_office::is_opendocument(head)
        || ethos_parser_office::is_rtf(head)
        || ethos_parser_office::zip::looks_like_zip(head)
    {
        return ethos_parser_office::read(head);
    }

    // **v2-S10: the branch that was missing, and the last member of the shape S8 named.**
    //
    // Everything above is a signature the bytes state. What falls past all six is bytes that state
    // **no format at all** — a `.csv`, a letter, a log line, a `.txt` — and until this slice they
    // were handed to the PDF reader anyway, to be refused for having no `%PDF-` header. That was
    // fail-closed and it named the **wrong cause**, exactly as an `.ods` did before v2-S6, an
    // `.rtf` before v2-S8 and an `.epub` before the line above it. S8 fixed the *container* shape
    // for its whole class rather than one more member; this is the class S8 left, and it is the
    // last one there is: bytes carrying no signature, no container and no declaration.
    //
    // **This is not a seventh predicate and nothing new is sniffed.** It is the negation of the
    // six above plus the one question the PDF reader already answers about itself. No comma is
    // counted, no line is measured, no extension is read, and no signature is looked for past
    // byte 0. Comma-separated text cannot be told from prose without a reader, and a detector that
    // guessed would claim every comma file — so this engine refuses to name a format it did not
    // measure rather than naming one it cannot. `docs/history/15-V2-MILESTONES.md` S10 argues it in full.
    //
    // A **truncated** PDF is deliberately not here: it aimed at the PDF reader, so the PDF
    // reader's own message is the honest cause for it, down to the zero-byte case.
    if !ethos_parser_pdf::aims_at_the_pdf_reader(head) {
        return Err(no_format_stated());
    }

    // Opened once, exactly as `classify` opens it — and from the bytes the router
    // already read, so the file is read exactly once end to end. The same handle
    // serves both stages (docs/04-ARCHITECTURE.md §2.1); nothing below the CLI
    // opens a file.
    //
    // The happy-path output is the REPRESENTATION, not the stage artifact: `05-MILESTONES.md`
    // M5 makes `DocumentRepresentation v0` the canonical record, and it is what `ethos-parser ground`
    // consumes. The stage artifact remains the library's return type, so M3's acceptance suite
    // still asserts on the thing the parser actually produces.
    let doc = Document::open_bytes(head, profile)?;
    let extract = ethos_parser_pdf::extract(&doc, profile)?;
    ethos_parser_pdf::to_representation(&extract, profile)
}

/// The refusal for bytes that state no format at all (v2-S10).
///
/// **It quotes nothing from the file, and that is the assertion rather than a style choice.** A
/// `.csv` and a letter containing a shopping list produce byte-identical stderr here, which is a
/// test in `no_format_cli.rs` — and it is a test only an implementation that sniffed nothing can
/// pass. The moment this message differs between two such files, something measured one of them.
///
/// It names what was **looked for** rather than what the file might be. Naming a format would be
/// an invented identifier (**A4**, and `docs/history/14-V2-SCOPE.md`'s standing rule 4): nobody measured
/// this file to be a CSV, a log or a letter, and `SourceIdentity` has two fields and no room to
/// record that a type was asserted rather than read — which is `docs/history/13-V12-MILESTONES.md`'s
/// v1.2-S5 finding, that *an identity that can be asserted is an identity that can disagree with
/// what it describes*.
///
/// `%PDF-` is absent from the text on purpose. These bytes never aimed at the PDF reader, so a
/// missing PDF header is not their cause — that is the defect this branch exists to end.
fn no_format_stated() -> EngineError {
    EngineError::Unsupported {
        what: "media type".into(),
        detail: "these bytes state no format this engine reads. Three signatures were looked for \
                 at byte 0 and none is present: a ZIP local file header, the container a DOCX, \
                 XLSX, PPTX, ODT, ODS, ODP or EPUB arrives in; an RTF brace group; and a PDF \
                 header. Nothing else was consulted — not the file's name, and no signature at any \
                 other offset. A file this engine has no reader for is refused by name rather than \
                 handed to a reader it never named, because a refusal that guesses a format is a \
                 claim nobody measured. `docs/CAPABILITY.md` lists what is read and what is not."
            .into(),
    }
}

/// Print a sealed representation as canonical JSON, or map the failure to an exit code.
///
/// Shared by both readers so the two cannot drift in how they emit: **one serializer**, which is
/// `docs/history/14-V2-SCOPE.md` §4's whole point.
fn emit_representation(
    result: Result<ethos_parser_core::DocumentRepresentation, EngineError>,
) -> ExitCode {
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

/// Read a representation from disk and project it into Markdown plus its Anchor Map.
///
/// Thin, like every other subcommand: read bytes, call the library, print canonical bytes, map the
/// outcome to an exit code. The projection lives in `ethos_parser_core::markdown` — it is a projection of
/// the representation and has nothing to do with PDF, so `ethos-parser-pdf` never learns Markdown
/// (`docs/04-ARCHITECTURE.md` §1).
fn run_markdown(args: MarkdownArgs) -> ExitCode {
    let bytes = match read_source(&args.path) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };

    let repr: ethos_parser_core::DocumentRepresentation = match serde_json::from_slice(&bytes) {
        Ok(r) => r,
        Err(e) => {
            return fail(&EngineError::Malformed {
                what: "representation".into(),
                detail: e.to_string(),
            })
        }
    };

    // Checked before anything is projected, exactly as `ground` does. A representation whose
    // payload does not hash to its declared digest is not a record this engine will speak for, and
    // projecting it anyway would launder the disagreement into a fresh-looking artifact whose
    // anchor map named node ids nobody can now confirm.
    if let Err(e) = repr.verify_fingerprint() {
        return fail(&e);
    }

    let profile = Profile::default();
    let profile_sha256 = match profile.profile_sha256() {
        Ok(h) => h,
        Err(e) => {
            return fail(&EngineError::Malformed {
                what: "profile".into(),
                detail: e.to_string(),
            })
        }
    };

    let artifact = match ethos_parser_core::to_markdown(
        &repr,
        &profile.parser_version,
        &profile_sha256,
        &profile.markdown_rule,
    ) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };

    match artifact.to_canonical_bytes() {
        Ok(out) => {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(&out);
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();
            ExitCode::from(PROJECTED as u8)
        }
        Err(e) => fail(&e),
    }
}

/// Serve MCP over stdin/stdout until the stream closes.
///
/// Thin, like every other subcommand: the protocol plumbing is `mcp.rs` and every tool in it calls
/// the same library entry point the matching subcommand calls (`docs/04-ARCHITECTURE.md` §1).
fn run_mcp() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match mcp::serve(stdin.lock(), stdout.lock()) {
        Ok(()) => ExitCode::from(PROJECTED as u8),
        Err(e) => fail(&EngineError::Io {
            detail: format!("mcp stdio: {e}"),
        }),
    }
}

/// Read a representation from disk and project it into HTML plus its Anchor Map.
///
/// The same seven steps `run_markdown` takes, against `ethos_parser_core::to_html`. Thin by the same
/// rule: `docs/04-ARCHITECTURE.md` §1 puts no logic in the CLI, and the projection is a fact about
/// the representation rather than about PDF, so `ethos-parser-pdf` never learns HTML either.
fn run_html(args: HtmlArgs) -> ExitCode {
    let bytes = match read_source(&args.path) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };

    let repr: ethos_parser_core::DocumentRepresentation = match serde_json::from_slice(&bytes) {
        Ok(r) => r,
        Err(e) => {
            return fail(&EngineError::Malformed {
                what: "representation".into(),
                detail: e.to_string(),
            })
        }
    };

    // Checked before anything is projected, exactly as `markdown` and `ground` do.
    if let Err(e) = repr.verify_fingerprint() {
        return fail(&e);
    }

    let profile = Profile::default();
    let profile_sha256 = match profile.profile_sha256() {
        Ok(h) => h,
        Err(e) => {
            return fail(&EngineError::Malformed {
                what: "profile".into(),
                detail: e.to_string(),
            })
        }
    };

    let artifact = match ethos_parser_core::to_html(
        &repr,
        &profile.parser_version,
        &profile_sha256,
        &profile.html_rule,
    ) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };

    match artifact.to_canonical_bytes() {
        Ok(out) => {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(&out);
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();
            ExitCode::from(PROJECTED as u8)
        }
        Err(e) => fail(&e),
    }
}

/// Read a representation from disk and project it.
///
/// Thin, like every other subcommand: it reads bytes, calls the library, prints canonical bytes,
/// and maps the outcome to an exit code. The projection itself lives in `ethos-parser-grounding`.
fn run_ground(args: GroundArgs) -> ExitCode {
    let bytes = match read_source(&args.path) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };

    let repr: ethos_parser_core::DocumentRepresentation = match serde_json::from_slice(&bytes) {
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

    let projection = match ethos_parser_grounding::project(&repr) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };

    match ethos_parser_grounding::to_canonical_bytes(&projection.source) {
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
            if let Some(w) = projection.spans_withheld {
                eprintln!(
                    "engine: {} span(s) withheld — more than the {} `ethos.grounding.v1` admits \
                     [{}]. The artifact carries its elements only (`capabilities.spans: false`): \
                     every block is still grounded, at block rather than run granularity.",
                    w.spans, w.limit, w.limitation_code,
                );
            }
            if let Some(o) = projection.elements_omitted {
                eprintln!(
                    "engine: {} element(s) omitted from the grounding artifact, and {} span(s) \
                     with them — text longer than the {} bytes, or a locator longer than the {}, \
                     that `ethos.grounding.v1` admits [{}]. The text remains in the \
                     representation; nothing is truncated.",
                    o.elements, o.spans, o.text_limit, o.locator_limit, o.limitation_code,
                );
            }
            if let Some(t) = projection.tables_withheld {
                eprintln!(
                    "engine: {} table(s) withheld — {} over the {} tables the schema admits, {} \
                     cell(s) longer than its {} bytes, {} grid(s) with more cells than it admits \
                     [{}]. The artifact carries no tables (`capabilities.tables: false`); \
                     withholding them moved no element or span.",
                    t.tables,
                    t.tables.saturating_sub(t.table_limit),
                    t.table_limit,
                    t.oversized_cells,
                    t.string_limit,
                    t.oversized_grids,
                    t.limitation_code,
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
    let grounding = match read_source(&args.path) {
        Ok(b) => b,
        Err(e) => return fail(&e),
    };

    // Ethos judges the artifact before it reads the source, so an artifact it refuses is reported
    // whatever the source path holds, and a source it cannot read, or over its 256 MiB, refuses
    // only a valid artifact. A failed read is therefore kept until the verdict is known. The one
    // difference left: this opens the source even for an artifact that turns out invalid, which a
    // path that blocks on open, such as a FIFO with no writer, can tell apart.
    let (source, unread) = match &args.source_artifact {
        None => (None, None),
        Some(p) => match read_source_within(p, GROUNDING_CHECK_MAX_SOURCE_BYTES) {
            Ok(b) => (Some(b), None),
            Err(e) => (None, Some(e)),
        },
    };

    let report = match ethos_parser_grounding::grounding_check(&grounding, source.as_deref()) {
        Ok(r) => r,
        // A non-PDF source, or an input past the accepted ceiling. Ethos refuses these before
        // writing any report and so does this: "these bytes are not the source" would be a
        // different and wrong statement about a file that is not a document at all.
        Err(e) => return fail(&e),
    };
    if let (ethos_parser_grounding::Structure::Valid, Some(e)) = (report.structure, &unread) {
        return fail(e);
    }

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

/// Spawn the pinned verifier and relay what it says.
///
/// Thin, and thinner than the rest: this one does not even look at the bytes it prints. It
/// resolves a binary, forwards the flags the caller gave, writes stdout through unchanged, and
/// returns the mapped exit code. `ethos_parser_core::verifier` owns all of that so an embedding caller
/// gets the same behaviour without a process boundary of its own.
fn run_verify(args: VerifyArgs) -> ExitCode {
    // `ETHOS_BIN` is read here rather than in the library: which environment variable pins the
    // verifier is a property of how this tool is deployed, not of the relay.
    let explicit = std::env::var_os("ETHOS_BIN").map(PathBuf::from);
    let repo_relative = std::env::current_dir().ok();

    let binary = match VerifierBinary::resolve(explicit.as_deref(), repo_relative.as_deref()) {
        Ok(b) => b,
        // Exit 2 with nothing on stdout. A caller that got a report here would have one the
        // engine invented, which is the single outcome this subcommand exists to make impossible.
        Err(e) => return fail(&e),
    };

    let request = RelayRequest {
        grounding: &args.path,
        citations: &args.citations,
        adapter: ethos_parser_core::GROUNDING_ADAPTER,
        fail_on_ungrounded: args.fail_on_ungrounded,
        config: args.config.as_deref(),
        out: args.out.as_deref(),
    };

    let relayed = match ethos_parser_core::relay(&binary, &request) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };

    // Verbatim, in both directions. The report goes to stdout because it is the artifact of this
    // subcommand; the verifier's own diagnostics go to stderr because they are its, not ours.
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&relayed.stdout);
    let _ = out.flush();
    if !relayed.stderr.is_empty() {
        let mut err = std::io::stderr().lock();
        let _ = err.write_all(&relayed.stderr);
        let _ = err.flush();
    }

    ExitCode::from(relayed.exit as u8)
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
