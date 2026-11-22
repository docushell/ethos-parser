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
//! **Four subcommands at v0. Twelve now**, and this paragraph said four until v2-S13.5, nine until
//! auto-tagging S2, ten until `locate` and eleven until `elements`. The v0 four are `classify`,
//! `extract`, `ground` and `grounding-check`; `verify` arrived at v0.1, `markdown` at v1.1-S1,
//! `html` at v1.1-S4, `mcp` at v1.2-S1, `overlay` with the image work, `tag` — the writer of
//! `docs/23-AUTO-TAGGING-SCOPE.md` — at auto-tagging S2, `locate` — the query of
//! `docs/26-LOCATE-SCOPE.md` — at v2.3, and `elements` with decision #137. The `Command` enum below is the list that cannot go stale, and
//! `crates/ethos-parser-core/src/verifier.rs` said *"the other eight subcommands"* from v0.1 until
//! auto-tagging S2 moved it with this paragraph — two files in one workspace disagreeing about a
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
//! omission note, the `--ocr` hint, and `--diagnostics` all go to stderr, so a caller redirecting
//! stdout to a file gets an artifact whose bytes are identical across runs
//! (`docs/04-ARCHITECTURE.md` §2).

#![forbid(unsafe_code)]

mod assist;
mod mcp;
mod recognizer;

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
    about = "Deterministic document parsing: PDFs, office files and scans into JSON, Markdown and \
             HTML, each extracted character traced to its source",
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
    ///
    /// **Exit 0 is not a prediction that `extract` will succeed.** Classification counts
    /// operators, images, paths and annotations over the sampled pages; it reads no structure
    /// tree and interprets no text, and a malformed tree is what `extract` refuses a document
    /// for. Every classification declares this as `classify-reads-no-structure-tree`, and 1 of
    /// 297 PDFs measured classifies cleanly and is refused by `extract`.
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

    /// Project a `DocumentRepresentation v0` into `ethos.markdown.v1`.
    ///
    /// **Markdown and its Anchor Map, always together.** They are fields of one artifact, not two
    /// files, so there is no `--md-only` and no way for a caller to end up with a Markdown string
    /// whose bytes cannot be traced back to evidence. MCP's `markdown` tool is the one exception:
    /// it hands a model the string alone, to read, and a quote taken from it binds through
    /// `locate` instead. The map is required because a projection between what a retriever ranks
    /// and what a citation binds is where a locator is lost without anyone noticing, so this engine
    /// writes no Markdown without one (`docs/01-CONTRACT.md` §12).
    ///
    /// stdout is canonical JSON. A consumer that wants a `.md` file writes
    /// `.markdown` out itself, and owns the fact that doing so discards the map.
    ///
    /// **A record holding a drafting model's replies** (`extract --assist-answers`): a page where
    /// the engine read under 30% of the words the model's blocks would write
    /// is written as the model read it, in place of the engine's runs; on every other page the
    /// model's tables and charts are added where they stand on the page, and one equal to an
    /// engine table it overlaps is left out. Each insertion is one block quote, opening `>
    /// **Proposed:** WHAT by ID@VERSION; model text, not read from the document.` and closing `>
    /// **End of proposed text.**`, the model's blocks between them one quote deeper, every line
    /// opening with `> >`; the model's text there is escaped — `&` and `<` as references, every
    /// `[` and `]` after a `\`, a run of backticks or tildes that could open a fence after a `\` —
    /// and its code fenced by more backticks than it holds, so none of it leaves its quote nor
    /// stands where the label and end line stand, and no link or image forms. A table is
    /// written from its cells, never as the model's HTML. Lines the guard found contradicted or
    /// unsupported are left out, a table cell emptied; a chart's estimated values are kept and its
    /// label says so. The model's bytes are `proposed` segments of the map, never citable, and the
    /// artifact's `proposed` census says page by page what was written and left out. Without that
    /// key, nothing here changes.
    ///
    /// Exit codes: **0** projected · **2** could not read, or the representation does not hash to
    /// its declared digest.
    Markdown(MarkdownArgs),

    /// Project a `DocumentRepresentation v0` into `ethos.html.v1`.
    ///
    /// **HTML and its Anchor Map, always together**, on exactly the discipline `markdown` runs
    /// under: one artifact, the same segment kinds, a map that tiles every byte, and a census that
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
    /// **A record holding a drafting model's replies** is projected as `markdown` projects it,
    /// each page or table the model proposed inside one `<blockquote
    /// class="ethos-proposed" data-derivation="proposed" data-model="ID"
    /// data-model-version="VERSION">` opening with a `<p>` that says so and closing with `<p>`
    /// **End of proposed text.**, the model's blocks between in a `<blockquote>` of their own, its
    /// text escaped; a model table keeps its spans, a cell holding
    /// an estimate is marked `data-estimate="true"` and an emptied one `data-dropped="contradicted"`
    /// or `"unsupported"`.
    ///
    /// Exit codes: **0** projected · **2** could not read, or the representation does not hash to
    /// its declared digest.
    Html(HtmlArgs),

    /// Report where a string lies in a `DocumentRepresentation v0`.
    ///
    /// **It answers where, and nothing else.** No verdict, no boolean, no score and no evidence
    /// tier: a string that occurs nowhere is exit 0 with an empty `occurrences` array, the same
    /// artifact a found one produces. Whether a location supports a claim stays the verifier's to
    /// say (`docs/07-VERIFY-BOUNDARY.md` §2), and this takes no claim — a representation and a
    /// string, and a string is not a claim.
    ///
    /// The match rule is this engine's own, versioned as `locate_rule` on the profile:
    /// code-point-exact on Unicode scalars, no normalization, no case folding, no whitespace
    /// folding, searching each **block** of the reading-order cut. So a match may join runs
    /// inside one block and may **not** join across two — a block boundary is a gap the page
    /// drew, and joining it would assert an adjacency the document does not have. The artifact
    /// declares what was searched (`searched.nodes`, `searched.blocks`, `searched.scalars`) for
    /// exactly that reason.
    ///
    /// **Exit codes: 0 answered · 2 could not read or refused. There is no exit 1, ever.** An
    /// exit 1 meaning *not found* would be one step from a verdict, which this subcommand never
    /// gives. And, as `extract --help` says, overloading 1 would make a caller's `&&` chain mean
    /// two different things depending on which subcommand ran.
    Locate(LocateArgs),

    /// Project a `DocumentRepresentation v0` into `ethos.parser.elements.v0`.
    ///
    /// **One flat list of the document's items**, each with its page, its box, its text, the node
    /// ids it covers and a label the record states: `text`, `section-header`, `page-header`,
    /// `page-footer`, `caption`, `list-item`, `table` (with its grid), `picture` or `form-field`
    /// (with its name, label, type, value and a check box's state, and no box: a field's declared
    /// rectangle is not a measured one). No label is inferred here: a heading is a tagged one or
    /// one the reader inferred from its type, as Markdown writes it with `#`; a header or footer
    /// is the reader's page furniture; a `caption` and a `list-item` are roles the document
    /// tagged, or a DOCX paragraph's declared numbering. A DOCX paragraph is one element. Boxes
    /// are integer centipoints from the top-left of the page; a source with no pages has neither
    /// pages nor boxes. A table the next page goes on with is one `table` element on its first
    /// page: the next page's rows follow its own, each of their cells carries its own `page` and
    /// box, and `continued` names the table records joined in.
    ///
    /// **Nothing is dropped silently**: a node no element carries — a run with no measured box, an
    /// annotation — is listed under `omitted` with its reason, and one line on stderr counts them.
    ///
    /// Takes a representation, or `--source FILE` to read a document and project it in one process,
    /// as `markdown --source` does. stdout is canonical JSON.
    ///
    /// Exit codes: **0** projected · **2** could not read, or the representation does not hash to
    /// its declared digest.
    Elements(ElementsArgs),

    /// Serve the engine over MCP on stdin/stdout.
    ///
    /// **stdio, newline-delimited JSON-RPC** — MCP's own stdio transport, so it is a pipe rather
    /// than a socket: no HTTP, no SSE, no TLS, no async runtime, and `deny.toml`'s network bans
    /// stay in force.
    ///
    /// Five tools — `extract`, `ground`, `node_get`, `locate` and `markdown` — each calling the
    /// same library entry point the matching subcommand calls, so an artifact returned here is the
    /// artifact this CLI prints, byte for byte. `markdown` returns no artifact: only the text of
    /// the one `markdown` prints, for a model to read.
    ///
    /// **No tool accepts a locator.** MCP tools are model-controlled, so a tool taking a `page` or
    /// a `bbox` the engine then trusts would make the model the citation authority in one step.
    /// Every locator is minted by the engine inside an artifact, travels back as an opaque handle,
    /// and is re-validated against that artifact on the way in — a handle this engine did not
    /// mint is an error, never a best guess. See `docs/history/12-V12-SCOPE.md` §3.
    ///
    /// **Each request is answered by a process of its own**, running this binary, so a call's
    /// memory goes back to the system when its answer is out, and a call that aborts fails alone.
    ///
    /// Exit codes: **0** the stream closed cleanly · **2** stdin or stdout failed.
    Mcp(McpArgs),

    /// Draw what was detected onto a copy of the document.
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

    /// Write this engine's own structure tree into a copy of an untagged PDF.
    ///
    /// Emits a PDF — the second subcommand, after `overlay`, whose stdout is not canonical JSON,
    /// and the only one whose output is a document rather than an artifact — carrying one
    /// `/Document` element over one `/Div` per block of the reading-order cut, every element
    /// marked `/A << /O /EthosParser /Derivation /Computed /Rule (…) >>`, each block's text
    /// wrapped in marked-content sequences inserted into the page's content at token boundaries,
    /// a `/ParentTree`, and the `/EthosParserTags` provenance stamp. `extract` on the result binds
    /// every run to a computed `Document/Div` address and declares
    /// `structure-tree-engine-written`; the text record is unchanged, and the writer proves that
    /// on its own output before a byte is printed (`docs/23-AUTO-TAGGING-SCOPE.md` §3.7).
    ///
    /// **It fills absence only.** A document that already carries `/StructTreeRoot` — an
    /// author's tree, or this subcommand's own output — is refused, as are marked-content ids
    /// without a tree, a `/StructParents` or `/StructParent` key without one, a `TJ` whose strings
    /// — or a string whose glyphs, set an em or more apart — the cut placed in two blocks, a page
    /// the tokeniser cannot account for, and a filter the strict decoder does not cover (§3.6 and
    /// the amendments under the scope's header). No `/MarkInfo` is written: the result is not a
    /// Tagged PDF, and a reader that does not read the owner attribute sees author structure (§9).
    /// Not exposed over MCP or the SDKs (§5).
    ///
    /// Exit codes: **0** the tagged PDF was written · **2** the document could not be read, or
    /// was refused.
    Tag(TagArgs),

    /// Validate a grounding artifact: structure, and optionally its binding to source bytes.
    ///
    /// **Structure and binding only** — no claims, no verdict, no `grounded`, no evidence tier.
    /// The engine validates; it never verifies (`docs/07-VERIFY-BOUNDARY.md`).
    ///
    /// Exit codes: **0** valid (and matched, or not checked) · **1** invalid structure, or a
    /// source that does not bind · **2** the input could not be read at all. The Ethos verifier's
    /// own `ethos grounding check` returns 2 for both of the last two; the engine keeps them
    /// apart, because a caller that cannot tell a failing check from an unreadable file is the
    /// defect this project refuses. Both agree on zero versus non-zero, which is what a shell
    /// predicate reads.
    GroundingCheck(GroundingCheckArgs),

    /// Relay a citation-verification run to the Ethos verifier, a separate program.
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
    /// The verifier is the binary `ETHOS_BIN` names, or else `ethos` on `PATH`; never one found
    /// relative to the working directory. `ETHOS_BIN` is authoritative: set to something that is
    /// not a file, it is an error, not a fallback. The binary is pinned by its version and SHA-256
    /// before it runs, and one that will not report its version is not used. Absence is a named
    /// error, never a skip and never a default-pass.
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
    /// The document to extract: PDF, DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF or EPUB; with `--ocr`,
    /// also a PNG or JPEG.
    path: PathBuf,

    /// Process at most N pages; the rest are quarantined and declared. Unbounded by default.
    ///
    /// **This is the only bound a caller has on how much memory one extract costs.** Peak resident
    /// memory tracks PAGE COUNT rather than file size — 3.0 to 6.8 MiB per page across the gate
    /// corpus (the eight public PDFs in `fixtures/gate/`) at 0.58.0. `nist-sp-800-171r3` is 120
    /// pages and 1.5 MB and peaks at 411 MiB; `nist-sp-800-37r2` is 1.4x the file at 2.2 MB but
    /// 1.5x the pages, and peaks at 915 MiB — 2.2x. Every page's extract is retained because it IS
    /// the artifact, so the only thing that bounds the cost is admitting fewer pages.
    ///
    /// **It does not bound everything.** `--max-pages 0` on a 733-page document still costs
    /// 221 MiB. Measured at 0.58.0, 193 MiB of that is the source bytes and the parsed object
    /// graph, which `classify` pays too and which no budget on this command reaches; the
    /// structure tree, read over the whole document before the budget is consulted, is 28 MiB of
    /// it. For sizing, budget 7 MiB, plus 5.4 MiB per admitted page, plus 0.33 MiB per page in
    /// the document — each the worst coefficient measured on the gate corpus. That over-predicts
    /// every point measured: by 3.6% at the tightest, the 733-page document at `--max-pages 128`;
    /// by 12.9% on its full extract; and by 15% to 129% on every other document's. Readings and
    /// instruments: `docs/measurements/memory-ceiling/` §15. A document built to cost more than
    /// its pages is refused by name first: a page may hold 1,048,576 content operations and 64 MiB
    /// of decoded content, and dense pages are read one at a time (§17).
    ///
    /// The pages left out are not silently dropped. Each is quarantined with
    /// `resource_limit_pages` and the artifact declares the limitation, through the same
    /// machinery `PageBudget::AtMost` drives in the library.
    ///
    /// Changing this changes `profile_sha256`, exactly as `classify --sample-pages` does, so a
    /// bounded artifact and an unbounded one are correctly non-comparable rather than quietly
    /// different.
    ///
    /// **A PDF's pages only.** Bytes the office reader takes are refused under this flag, by name:
    /// that reader has no page budget, and an unbounded artifact is not an answer to a bounded
    /// request.
    #[arg(long, value_name = "N")]
    max_pages: Option<u32>,

    /// Read every page that shows no text and places one image — a scan — with the recognizer,
    /// `ethos-parser-ocr` (beside this binary, or named by `ETHOS_PARSER_OCR`). Its words are runs
    /// of derivation `recognized`, under the OCR profile, so the artifact's `profile_sha256`
    /// differs from one made without it (`docs/35-OCR-SCOPE.md`). Pages that show text are read as
    /// they always are; nothing recognized is ever merged with them. A PNG or JPEG file is read as
    /// one page, its own pixel grid. The recognizer reads English only.
    #[arg(long)]
    ocr: bool,

    /// Attach a drafting model's replies to the record, read from FILE: its blocks, under the
    /// `assist` key, each of derivation `proposed` — a model's text, never evidence, and never a
    /// node — under a profile pinning the model, so the record's `profile_sha256` differs from one
    /// made without it. The output is a function of the PDF and FILE alone. FILE
    /// is refused by name where it does not answer this PDF: another source digest, renderer,
    /// request, model or page; a model id that is a file path or names this engine; over 64 MiB,
    /// or a reply over 2,048 block markers. A reply to a page `--max-pages` cut is left out, and
    /// its number kept under `assist.pages_left_out`. A PDF only.
    #[arg(long, value_name = "FILE")]
    assist_answers: Option<PathBuf>,

    /// The model the replies must come from, as `ID` or `ID@VERSION`: FILE naming another is
    /// refused. A version left out is not compared. With `--assist`, the model asked, and its
    /// `@VERSION` the one FILE records; without it, the one model SERVER lists.
    #[arg(long, value_name = "ID[@VERSION]", requires = "assist_answers")]
    assist_model: Option<String>,

    /// Ask the model server at SERVER (an `http://` or `https://` address of an OpenAI-compatible
    /// server) to read this PDF's pages, then attach its replies as `--assist-answers FILE` does.
    /// The asking is done by `ethos-parser-assist` (beside this binary, or named
    /// by `ETHOS_PARSER_ASSIST`), which renders each page, sends it, and writes FILE after each
    /// one, so a stopped run resumes where it stopped; this binary opens no connection. FILE is
    /// then read back, so the output is a function of the PDF and FILE alone: a later
    /// `--assist-answers FILE` gives the same bytes. A FILE reused from an earlier run keeps its
    /// pages, which are attached with the new ones; only a page with no reply is asked. No timeout
    /// is set: the assist binary bounds each request, and an assist binary that hangs otherwise
    /// hangs this command. An API key is read from
    /// `ETHOS_PARSER_ASSIST_KEY` only, by the assist binary, and never written. Refused by name:
    /// `assist-server-unreachable`, and the assist binary's refusals as `assist-` and its name
    /// (`assist-curl-not-found`, `assist-key-needs-https`, `assist-model-not-chosen`, ...) or as
    /// the `assist-answers-*` name the merge would give. Where no page is asked or none could be
    /// drawn, and FILE does not exist, the record is the engine's alone and stderr says so. Setup:
    /// `assist/README.md`.
    #[arg(long, value_name = "SERVER", requires = "assist_answers")]
    assist: Option<String>,

    /// The pages `--assist` asks for. `trigger`, the default, asks for each page where the record
    /// shows a scan, under 20 distinct words, a figure, a table, a picture covering a quarter of
    /// the visible page or more, or U+FFFD in a run; `all` asks for every page
    /// the record lists; a LIST of page numbers in digits, as `1,3,7`, for those pages. A page the
    /// record does not list, as one `--max-pages` cut, is never asked, and stderr counts it.
    #[arg(long, value_name = "trigger|all|LIST", requires = "assist", value_parser = assist_pages)]
    assist_pages: Option<AssistPages>,
}

/// Which pages `extract --assist` asks for.
#[derive(Clone)]
enum AssistPages {
    Trigger,
    All,
    List(Vec<u32>),
}

/// `--assist-pages`: `trigger`, `all`, or page numbers from 1 in digits, comma-separated, sorted
/// and made unique.
fn assist_pages(s: &str) -> Result<AssistPages, String> {
    match s {
        "trigger" => Ok(AssistPages::Trigger),
        "all" => Ok(AssistPages::All),
        _ => s
            .split(',')
            .map(|p| {
                // Digits only: `parse` would take a sign.
                let p = p.trim();
                let digits = p.bytes().all(|b| b.is_ascii_digit());
                p.parse::<u32>().ok().filter(|&n| digits && n >= 1)
            })
            .collect::<Option<std::collections::BTreeSet<u32>>>()
            .map(|pages| AssistPages::List(pages.into_iter().collect()))
            .ok_or_else(|| "`trigger`, `all`, or page numbers from 1, as `1,3,7`".into()),
    }
}

#[derive(clap::Args)]
struct LocateArgs {
    /// The `DocumentRepresentation v0` to search, as `ethos-parser extract` emits it.
    path: PathBuf,

    /// A file holding the quote. **Its bytes are the quote, verbatim.**
    ///
    /// No trim, no trailing-newline strip, no BOM removal. Stripping one trailing LF would make
    /// `printf %s` and `echo` agree and would make a quote that genuinely ends in a newline
    /// unaskable — a silent edit of the caller's input. A shell caller holding the quote in a
    /// variable writes `--quote-file <(printf %s "$q")`; the bounded read accepts a non-regular
    /// file.
    ///
    /// **Not on argv**, and the decisive reason is not privacy: argv cannot carry every string a
    /// representation can contain. A NUL cannot appear in an argument at all, a newline survives
    /// only through correct quoting, and a quoting mistake changes the string *silently* — which
    /// changes what was searched with nothing on the wire saying so.
    ///
    /// The read is bounded by the quote's own 16,384-byte ceiling, which is the longest string
    /// `ethos.grounding.v1` admits. A larger file is refused by name.
    #[arg(long, value_name = "FILE")]
    quote_file: PathBuf,
}

#[derive(clap::Args)]
struct ElementsArgs {
    /// A `DocumentRepresentation v0` JSON file, as `ethos-parser extract` emits.
    #[arg(required_unless_present = "source", conflicts_with = "source")]
    path: Option<PathBuf>,

    /// A source document — PDF or office — to extract and project in one process. The artifact is
    /// byte-identical to `extract` piped into this subcommand.
    #[arg(long, value_name = "FILE")]
    source: Option<PathBuf>,
}

#[derive(clap::Args)]
struct GroundingCheckArgs {
    /// The `ethos.grounding.v1` JSON to validate.
    path: PathBuf,

    /// The PDF the artifact claims to describe.
    ///
    /// Supplying it turns `source_binding` from `not_checked` into `matched` or `mismatched`.
    /// Omitting it is not a pass — it is a question that was not asked. Named to match the Ethos
    /// verifier's `ethos grounding check`, so one command line serves both: this project's oracle
    /// test runs the two checkers on the same files and requires the same answer.
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

    /// Leave out repeated page furniture, counted, as `markdown --drop-furniture` does.
    #[arg(long)]
    drop_furniture: bool,

    /// Keep a table's header row that the next page repeats as a row of the joined table, as
    /// `markdown --keep-repeated-header` does.
    #[arg(long)]
    keep_repeated_header: bool,

    /// On a document that declares no author structure (an untagged PDF), write a list item for
    /// each layout unit opening with a list marker, two or more in a row, counted, as
    /// `markdown --infer-lists` does.
    #[arg(long)]
    infer_lists: bool,
    /// Leave out text the reader flagged as hidden, counted, as `markdown --drop-hidden` does.
    #[arg(long)]
    drop_hidden: bool,

    /// Write the text without the elements for its style, counted, as `markdown --unstyled` does.
    #[arg(long)]
    unstyled: bool,
}

#[derive(clap::Args)]
struct MarkdownArgs {
    /// A `DocumentRepresentation v0` JSON file, as `ethos-parser extract` emits.
    ///
    /// **A representation, not a PDF.** The same input `ethos-parser ground` takes, deliberately: one
    /// subcommand that silently means two different things is how a caller ends up unsure which
    /// profile produced the artifact it is holding. `--source` is how a caller asks for the other
    /// thing, and it says so in the command line rather than in the bytes.
    #[arg(required_unless_present = "source", conflicts_with = "source")]
    path: Option<PathBuf>,

    /// A source document — PDF or office — to extract and project in one process.
    ///
    /// **The same two stages, without the record on disk between them.** The artifact is
    /// byte-identical to `extract` piped into this subcommand (`the_source_path_equals_the_two_step_path`
    /// asserts it), because it is the same two library calls under the same default profile; what
    /// it skips is serialising the representation to JSON, writing it, reading it back, parsing it,
    /// re-hashing it to verify a fingerprint this process computed moments ago, and a second
    /// process start. Measured over the 200 opendataloader-bench documents, those cost 3.1 ms and
    /// 7.6 ms per document against 5.4 ms of engine work
    /// (`docs/measurements/liteparse-head-to-head/README.md` §3).
    ///
    /// **Nothing is inferred from the flag.** The format is still decided by the file's own bytes,
    /// exactly as `extract` decides it, and a file that states no format this engine reads is
    /// refused by the same message. The representation is not emitted: a caller who needs the
    /// record — to ground a claim, to locate a quote, to keep — runs `extract` and keeps it.
    #[arg(long, value_name = "FILE")]
    source: Option<PathBuf>,

    /// With `--source`, read the source through the recognizer, as `extract` does with the same
    /// flag: its scanned pages, or the image file. The output is byte-identical to that extract
    /// piped into this subcommand.
    // `requires = "source"` alone does not refuse `markdown RECORD --ocr`; `conflicts_with` does.
    #[arg(long, requires = "source", conflicts_with = "path")]
    ocr: bool,

    /// Leave out repeated page furniture: a line the reader flagged as a page's header or footer,
    /// or a tagged page's `/Artifact` — which may be a watermark — whose text another page
    /// repeats on the same side and across the same place, numbers read alike. A running head
    /// and a folio go, a whole line at a time; a numbered chapter title at the top of each
    /// chapter's page goes too. A Word document's header and footer parts go whole, their tables
    /// with them — so a resume whose name sits only in its page header loses it. Counted under
    /// the census bucket `page-furniture-not-projected-v1`; the representation keeps every run.
    #[arg(long)]
    drop_furniture: bool,

    /// Keep a table's header row that the next page repeats, where the table goes on there as
    /// one, as a row of the joined table. By default it is left out, as a reader skips it,
    /// counted under `repeated-table-header-not-projected-v1`; the representation keeps it. Wins
    /// over `--drop-furniture`.
    #[arg(long)]
    keep_repeated_header: bool,

    /// On a document that declares no author structure (an untagged PDF), write a list item for
    /// each layout unit opening with a list marker — a bullet, or a number or a letter closed by
    /// `.` or `)` — two or more in a row, with the marker kept as the page drew it after `- `,
    /// nested by where the marker stands. An inferred heading stays a heading. Inferred, not
    /// declared: counted under the census code `list-items-inferred-from-markers-v1`, and the
    /// representation is unchanged.
    #[arg(long)]
    infer_lists: bool,
    /// Leave out every run the reader flagged with a finding: drawn invisible (`Tr 3`, or `Tr 7`
    /// with nothing painted through its clip), off the visible page, under a point, in a layer the
    /// document switches off, or filled in the colour painted beneath it. A scan's text layer
    /// stays — invisible text inside placed images covering half its page or more together — and
    /// an off-page line goes only where its box is known and misses the visible page (a line
    /// crossing the media box's edge has none, and stays). Counted under the
    /// census bucket `flagged-text-not-projected-v1`; the representation keeps every run and its
    /// findings.
    #[arg(long)]
    drop_hidden: bool,

    /// Write every run's text without the marks for its style: no `**`, `*`, `~~`, `<u>`,
    /// `<sup>` or `<sub>`. The text, headings, lists and tables are as without it, and every
    /// source segment is the one the styled text has, so each byte keeps its anchor. For a reader
    /// that compares text and has no use for its look. Counted under the census code
    /// `inline-styles-not-written-v1`, once per run a mark would have marked. A styled run
    /// opening a line with `#`, `>`, `-` or `1.` reads as block syntax without its marks, as a
    /// plain run does.
    #[arg(long)]
    unstyled: bool,
}

#[derive(clap::Args)]
struct GroundArgs {
    /// A `DocumentRepresentation v0` JSON file, as `ethos-parser extract` emits.
    path: PathBuf,
}

#[derive(clap::Args)]
struct McpArgs {
    /// Answer one request as the server's own process for it: the server's ledger line, then the
    /// request, on stdin. Hidden: it is how `mcp` runs a call, not a way to run one.
    #[arg(long, hide = true)]
    call: bool,
}

#[derive(clap::Args)]
struct OverlayArgs {
    /// The PDF to annotate.
    path: PathBuf,
}

#[derive(clap::Args)]
struct TagArgs {
    /// The untagged PDF to write a structure tree into.
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
/// page count at 3.0 to 6.8 MiB per page, which is what `extract --max-pages` exists to bound.
/// The two ceilings are complementary and neither subsumes the other.
///
/// They also do not COMPOSE into a memory bound, which is worth stating plainly: nothing maps a
/// permitted 2 GiB input onto a peak-memory figure, and the page budget leaves a floor that grows
/// with the document's page count. A caller who needs a hard memory ceiling does not have one
/// today. See `docs/measurements/memory-ceiling/` §5 and §15.
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
        // Under `Extract`, as `overlay` is: the writer runs extraction itself and reports under
        // the stage whose work it wrote into the file.
        Command::Tag(args) => {
            let path = args.path.clone();
            timed(Stage::Extract, diag, &path, || run_tag(args))
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
            // Whichever input the caller named is the one the diagnostics report read.
            let path = args
                .path
                .clone()
                .or_else(|| args.source.clone())
                .unwrap_or_default();
            timed(Stage::Ground, diag, &path, || run_markdown(args))
        }
        Command::Html(args) => {
            let path = args.path.clone();
            timed(Stage::Ground, diag, &path, || run_html(args))
        }
        // `Stage::Ground`, as `markdown` and `html` are: it reads a representation. A stage is
        // not a subcommand, and `diagnostics.rs`'s own test says so.
        Command::Locate(args) => {
            let path = args.path.clone();
            timed(Stage::Ground, diag, &path, || run_locate(args))
        }
        Command::Elements(args) => {
            let path = args
                .path
                .clone()
                .or_else(|| args.source.clone())
                .unwrap_or_default();
            timed(Stage::Ground, diag, &path, || run_elements(args))
        }
        // Not `timed`: a server has no one input path and no one stage, and inventing a
        // diagnostics row for the whole session would put a duration on a pipe.
        Command::Mcp(args) if args.call => run_mcp_call(),
        Command::Mcp(_) => run_mcp(),
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
            Err(e) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "engine: diagnostics unavailable: {e} [{}]",
                    e.code()
                );
            }
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
    // different object graph from the extractor is a silent divergence with no diagnostic. Read
    // through `read_source`, as `extract` is: `Document::open` reads a path with no ceiling.
    let result = read_source(&args.path)
        .and_then(|bytes| Document::open_bytes(&bytes, &profile))
        .and_then(|doc| ethos_parser_pdf::classify(&doc, &profile));

    match &result {
        Ok(classification) => match classification.to_canonical_bytes() {
            Ok(bytes) => match write_stdout(&bytes, true) {
                Ok(()) => ExitCode::from(exit_code(&result) as u8),
                Err(e) => fail(&e),
            },
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
    let result = read_source(&args.path)
        .and_then(|bytes| Document::open_bytes(&bytes, &profile))
        .and_then(|doc| {
            let extract = ethos_parser_pdf::extract(&doc, &profile)?;
            ethos_parser_pdf::build_overlay(&doc, &extract, &profile)
        });

    match result {
        Ok(bytes) => match write_stdout(&bytes, false) {
            Ok(()) => ExitCode::from(EXTRACTED as u8),
            Err(e) => fail(&e),
        },
        Err(e) => fail(&e),
    }
}

/// `ethos-parser tag` — the tagged PDF (auto-tagging S2).
///
/// The document is opened once through the bounded read and handed to `write_tags`, which runs
/// extraction itself: the placement rule needs to know which operator showed each run, and that
/// mapping lives beside the artifact and never on it (`docs/23-AUTO-TAGGING-SCOPE.md` §6), so no
/// artifact parsed from JSON can reach it. Nothing is printed on a refusal: a partial PDF on
/// stdout would be a document nobody wrote.
fn run_tag(args: TagArgs) -> ExitCode {
    let profile = Profile::default();
    let result = read_source(&args.path)
        .and_then(|bytes| Document::open_bytes(&bytes, &profile))
        .and_then(|doc| ethos_parser_pdf::write_tags(&doc, &profile));

    match result {
        Ok(bytes) => match write_stdout(&bytes, false) {
            Ok(()) => ExitCode::from(EXTRACTED as u8),
            Err(e) => fail(&e),
        },
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
        // The office readers take no profile, so a budget handed to them would be dropped.
        if routes_to_office(&head) {
            return fail(&EngineError::Unsupported {
                what: "option".into(),
                detail: "--max-pages bounds the pages of a PDF; these bytes go to the office \
                         reader, which has no page budget, so the bound cannot be applied"
                    .into(),
            });
        }
        profile.page_budget = ethos_parser_core::PageBudget::AtMost(n);
    }
    // A model reads rendered pages of a PDF; office and image bytes have none it was asked about.
    if args.assist_answers.is_some()
        && (routes_to_office(&head) || ethos_parser_pdf::image_media_type(&head).is_some())
    {
        return fail(&EngineError::Unsupported {
            what: "assist-not-pdf".into(),
            detail: "--assist-answers attaches a model's replies to a PDF's pages; these bytes \
                     are not a PDF"
                .into(),
        });
    }
    let repr = if args.ocr {
        profile = ocr_profile(&profile);
        recognized_representation(&head, &profile)
    } else {
        representation_for_bytes(&head, &profile)
    };
    let repr = match &args.assist_answers {
        Some(path) => repr.and_then(|r| {
            if let Some(server) = &args.assist {
                let mut pages = match &args.assist_pages {
                    None | Some(AssistPages::Trigger) => ethos_parser_core::assist_trigger(&r),
                    Some(AssistPages::All) => r.payload().pages.iter().map(|p| p.index).collect(),
                    Some(AssistPages::List(pages)) => pages.clone(),
                };
                // A page the document has and the record does not list — cut by `--max-pages`, or
                // not read — is not asked: a reply to it would be left out. One past the end is
                // the assist binary's to refuse by name.
                let p = r.payload();
                let unlisted = |n: &u32| {
                    *n <= p.assurance.coverage.pages_authorized
                        && !p.pages.iter().any(|page| page.index == *n)
                };
                let left_out = pages.iter().filter(|n| unlisted(n)).count();
                if left_out > 0 {
                    pages.retain(|n| !unlisted(n));
                    let _ = writeln!(
                        std::io::stderr(),
                        "engine: {left_out} page(s) the record does not list are not asked; a \
                         reply to one would be left out [assist-pages-left-out]"
                    );
                }
                if !pages.is_empty() {
                    assist::ask(server, path, &pages, args.assist_model.as_deref(), &head)?;
                }
                if !path.exists() {
                    let _ = writeln!(
                        std::io::stderr(),
                        "engine: {} page(s) asked and no reply written, so the record is the \
                         engine's alone [assist-nothing-attached]",
                        pages.len()
                    );
                    return Ok(r);
                }
            }
            // Read no further than the budget the library refuses past, and by the same name.
            let budget = ethos_parser_core::ASSIST_ANSWERS_MAX_BYTES;
            let answers = read_source_within(path, budget as u64).map_err(|e| match e {
                EngineError::ResourceLimit { .. } => EngineError::Malformed {
                    what: ethos_parser_core::ASSIST_ANSWERS_OVER_BUDGET.into(),
                    detail: format!("{} holds over {budget} bytes", path.display()),
                },
                e => e,
            })?;
            ethos_parser_core::attach(r, &profile, &answers, args.assist_model.as_deref())
        }),
        None => repr,
    };
    emit_representation(repr.inspect(|r| hint_ocr(r, OCR_ON_SOURCE)))
}

/// How [`hint_ocr`] asks for the recognizer on the command that read the source.
const OCR_ON_SOURCE: &str = "--ocr";
/// How it asks on a record, which `--ocr` cannot read: the source read again.
const OCR_ON_RECORD: &str = "`extract --ocr` or `markdown --source FILE --ocr`";

/// Name on stderr the scanned pages a read left unread (decision #132): one line, the count, each
/// reason, and `ask` for the pages the recognizer was not asked to read. The record counts each
/// under `scanned-page-not-recognized`, and the Markdown and HTML artifacts carry no limitation,
/// so without this a scan projects to empty Markdown and says nothing; stdout stays the artifact.
///
/// A limitation has no field for the reason: it is the first code in backticks of the `detail`
/// that `scanned_page_not_recognized` writes. A recognizer's reason is its own text, so a control
/// character in it is written as a space, keeping the line one line.
fn hint_ocr(repr: &ethos_parser_core::DocumentRepresentation, ask: &str) {
    let mut reasons = std::collections::BTreeMap::<String, usize>::new();
    for l in &repr.payload().assurance.limitations {
        if l.code == ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED {
            let reason = l.detail.split('`').nth(1).unwrap_or_default();
            *reasons
                .entry(reason.replace(char::is_control, " "))
                .or_default() += 1;
        }
    }
    if reasons.is_empty() {
        return;
    }
    let pages: usize = reasons.values().sum();
    let why: Vec<String> = reasons.iter().map(|(r, n)| format!("{r} ({n})")).collect();
    let not_asked = match reasons.get("ocr-not-requested") {
        Some(n) => format!("; {ask} reads the {n} not requested"),
        None => String::new(),
    };
    let _ = writeln!(
        std::io::stderr(),
        "engine: {pages} page(s) show no text and were not read: {}{not_asked} [{}]",
        why.join(", "),
        ethos_parser_core::codes::SCANNED_PAGE_NOT_RECOGNIZED
    );
}

/// The OCR profile, with any page budget `base` sets.
fn ocr_profile(base: &Profile) -> Profile {
    Profile {
        page_budget: base.page_budget,
        ..Profile::ocr_v0()
    }
}

/// `--ocr`: the recognizer found and pinned, and a PDF read through it under `profile`, the OCR
/// profile. Office bytes are refused by name: they have no scanned pages to read.
fn recognized_representation(
    head: &[u8],
    profile: &Profile,
) -> Result<ethos_parser_core::DocumentRepresentation, EngineError> {
    if routes_to_office(head) {
        return Err(EngineError::Unsupported {
            what: "option".into(),
            detail: "--ocr reads the scanned pages of a PDF; these bytes go to the office reader, \
                     which has no pages to scan"
                .into(),
        });
    }
    let mut process = recognizer::Process::find()?;
    representation_for(head, profile, Some(&mut process))
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
    representation_for(head, profile, None)
}

/// [`representation_for_bytes`], reading a PDF's scanned pages through `recognizer` when one is
/// given (`--ocr`, `docs/35-OCR-SCOPE.md`).
fn representation_for(
    head: &[u8],
    profile: &Profile,
    recognizer: Option<&mut dyn ethos_parser_pdf::Recognize>,
) -> Result<ethos_parser_core::DocumentRepresentation, EngineError> {
    if routes_to_office(head) {
        return ethos_parser_office::read(head);
    }
    // v3 (`docs/35-OCR-SCOPE.md` §4): a PNG or JPEG states its format at byte 0, and is a page only
    // a recognizer reads, so it is read with `--ocr` and refused by name without it.
    if let Some(media_type) = ethos_parser_pdf::image_media_type(head) {
        let Some(recognizer) = recognizer else {
            return Err(EngineError::Unsupported {
                what: "media type".into(),
                detail: format!(
                    "these bytes are an image file ({media_type}), and an image file's text is \
                     read only by the recognizer: run with --ocr"
                ),
            });
        };
        let extract = ethos_parser_pdf::extract_image(head, profile, recognizer)?;
        return ethos_parser_pdf::into_representation(extract, profile);
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
    // **This is not another predicate and nothing new is sniffed.** It is the negation of the
    // office six and the image signature above (v3), plus the one question the PDF reader already
    // answers about itself. No comma is
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
    let extract = match recognizer {
        Some(r) => ethos_parser_pdf::extract_recognized(&doc, profile, r)?,
        None => ethos_parser_pdf::extract(&doc, profile)?,
    };
    ethos_parser_pdf::into_representation(extract, profile)
}

/// Whether `representation_for_bytes` hands these bytes to the office reader.
fn routes_to_office(head: &[u8]) -> bool {
    ethos_parser_office::is_docx(head)
        || ethos_parser_office::is_xlsx(head)
        || ethos_parser_office::is_pptx(head)
        || ethos_parser_office::is_opendocument(head)
        || ethos_parser_office::is_rtf(head)
        || ethos_parser_office::zip::looks_like_zip(head)
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
        detail: "these bytes state no format this engine reads. Five signatures were looked for \
                 at byte 0 and none is present: a ZIP local file header, the container a DOCX, \
                 XLSX, PPTX, ODT, ODS, ODP or EPUB arrives in; an RTF brace group; a PDF header; \
                 and a PNG's or a JPEG's, an image file `--ocr` reads. Nothing else was consulted — not the file's name, and no signature at any \
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
        Ok(artifact) => {
            // Streamed, not assembled: one buffer holding the whole artifact was the run's peak.
            // Nothing reachable refuses after the first byte (`write_canonical_to`).
            let mut out = std::io::stdout().lock();
            let written = artifact.write_canonical_to(&mut out).and_then(|()| {
                out.write_all(b"\n")
                    .and_then(|()| out.flush())
                    .map_err(EngineError::from)
            });
            match written {
                Ok(()) => ExitCode::from(EXTRACTED as u8),
                // `write_stdout`'s words: the library does not know its writer is stdout.
                Err(EngineError::Io { detail }) => fail(&EngineError::Io {
                    detail: format!("writing to stdout: {detail}"),
                }),
                Err(e) => fail(&e),
            }
        }
        Err(e) => fail(&e),
    }
}

/// Read a representation from disk and project it into Markdown plus its Anchor Map.
///
/// Thin, like every other subcommand: read bytes, call the library, print canonical bytes, map the
/// outcome to an exit code. The projection lives in `ethos_parser_core::markdown` — it is a projection of
/// the representation and has nothing to do with PDF, so `ethos-parser-pdf` never learns Markdown
/// (`docs/04-ARCHITECTURE.md` §1).
/// Report where a string lies in a representation.
///
/// Thin, like the rest: read the representation, re-validate it, read the quote's bytes, call the
/// library, print canonical bytes. **The fingerprint is checked before the quote is even read**,
/// so a record this engine will not speak for is refused before anything is searched in it.
fn run_locate(args: LocateArgs) -> ExitCode {
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
    if let Err(e) = repr.verify_fingerprint() {
        return fail(&e);
    }

    // **The quote's own ceiling bounds the read**, one byte past it so an over-long quote is
    // refused by name with the number rather than as a source-size limit. `read_source`'s 2 GiB
    // is the ceiling on a *source*, and a quote is not a source: pointing this at a document
    // should cost a refusal, not a two-gigabyte allocation on the caller's behalf.
    let quote_bytes = match read_source_within(
        &args.quote_file,
        ethos_parser_core::LOCATE_MAX_QUOTE_BYTES as u64 + 1,
    ) {
        Ok(b) => b,
        Err(EngineError::ResourceLimit { .. }) => {
            return fail(&EngineError::Unsupported {
                what: "locate".into(),
                detail: format!(
                    "the quote file is larger than the {}-byte limit, which is the longest string \
                     `ethos.grounding.v1` admits — so it is longer than any element text a \
                     citation could carry",
                    ethos_parser_core::LOCATE_MAX_QUOTE_BYTES
                ),
            })
        }
        Err(e) => return fail(&e),
    };

    let quote = match String::from_utf8(quote_bytes) {
        Ok(q) => q,
        Err(e) => {
            return fail(&EngineError::Malformed {
                what: "locate quote".into(),
                detail: format!(
                    "the quote file is not UTF-8: the first invalid byte is at offset {}. A \
                     representation's text is UTF-8, so a byte sequence that is not UTF-8 cannot \
                     occur in one and there is nothing to search for",
                    e.utf8_error().valid_up_to()
                ),
            })
        }
    };

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

    let found = match ethos_parser_core::locate(
        &repr,
        &profile.parser_version,
        &profile_sha256,
        &profile.locate_rule,
        &quote,
    ) {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };

    match found.to_canonical_bytes() {
        Ok(out) => match write_stdout(&out, true) {
            // **Answered.** Not "found": an empty `occurrences` array is an answer, and there is
            // no other code for it.
            Ok(()) => ExitCode::from(PROJECTED as u8),
            Err(e) => fail(&e),
        },
        Err(e) => fail(&e),
    }
}

/// The record a projection will read, from whichever of the two inputs the caller named.
///
/// **The fingerprint is checked on one path and not the other, and the asymmetry is the point.** A
/// representation read from a file is a record this process did not build: it may have been edited,
/// truncated or produced by another build, and `01-CONTRACT.md` makes the digest the thing that
/// says otherwise — so it is verified before a byte is projected, exactly as `ground` verifies it.
/// A representation built from a source document two statements ago has nothing to disagree with;
/// re-hashing 250 KB to compare it with a digest this process just computed would measure the
/// hashing code, not the record. The projected bytes are identical either way, which is asserted
/// rather than claimed (`markdown_cli.rs::the_source_path_equals_the_two_step_path`).
fn representation_to_project(
    path: Option<&std::path::Path>,
    source: Option<&std::path::Path>,
    ocr: bool,
) -> Result<ethos_parser_core::DocumentRepresentation, EngineError> {
    if let Some(source) = source {
        let bytes = read_source(source)?;
        if ocr {
            return recognized_representation(&bytes, &ocr_profile(&Profile::default()));
        }
        return representation_for_bytes(&bytes, &Profile::default());
    }
    // clap holds the other arm: exactly one of the two is present.
    let path = path.expect("clap requires a representation path when --source is absent");
    let bytes = read_source(path)?;
    let repr: ethos_parser_core::DocumentRepresentation =
        serde_json::from_slice(&bytes).map_err(|e| EngineError::Malformed {
            what: "representation".into(),
            detail: e.to_string(),
        })?;
    repr.verify_fingerprint()?;
    Ok(repr)
}

fn run_markdown(args: MarkdownArgs) -> ExitCode {
    let repr =
        match representation_to_project(args.path.as_deref(), args.source.as_deref(), args.ocr) {
            Ok(r) => r,
            Err(e) => return fail(&e),
        };
    hint_ocr(
        &repr,
        if args.source.is_some() {
            OCR_ON_SOURCE
        } else {
            OCR_ON_RECORD
        },
    );

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

    let artifact = match ethos_parser_core::to_markdown_with(
        &repr,
        &profile.parser_version,
        &profile_sha256,
        &profile.markdown_rule,
        projection_options(
            args.drop_furniture,
            args.infer_lists,
            args.drop_hidden,
            args.keep_repeated_header,
            args.unstyled,
        ),
    ) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };

    match artifact.to_canonical_bytes() {
        Ok(out) => match write_stdout(&out, true) {
            Ok(()) => ExitCode::from(PROJECTED as u8),
            Err(e) => fail(&e),
        },
        Err(e) => fail(&e),
    }
}

/// Read a representation, or a source, and project its elements (decision #137). Thin, as
/// `run_markdown` is: the projection is `ethos_parser_core::to_elements`.
fn run_elements(args: ElementsArgs) -> ExitCode {
    let repr = match representation_to_project(args.path.as_deref(), args.source.as_deref(), false)
    {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    hint_ocr(&repr, OCR_ON_RECORD);
    let artifact = match ethos_parser_core::to_elements(&repr, &Profile::default().parser_version) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    if let Err(e) = artifact
        .to_canonical_bytes()
        .and_then(|out| write_stdout(&out, true))
    {
        return fail(&e);
    }
    // On stderr, as `ground`'s omission note is: stdout is the artifact, whose `omitted` names them.
    if !artifact.omitted.is_empty() {
        let nodes: usize = artifact.omitted.iter().map(|o| o.node_ids.len()).sum();
        let reasons: Vec<String> = artifact
            .omitted
            .iter()
            .map(|o| {
                let reason = serde_json::to_value(o.reason).unwrap_or_default();
                format!("{} {}", reason.as_str().unwrap_or_default(), o.count)
            })
            .collect();
        let _ = writeln!(
            std::io::stderr(),
            "engine: {nodes} of {} node(s) omitted from the elements ({}). They remain in the \
             representation; the artifact's `omitted` names each one.",
            repr.payload().nodes.len(),
            reasons.join(", "),
        );
    }
    ExitCode::from(PROJECTED as u8)
}

/// The projection options a `markdown` or `html` command line asks for (decisions #134, #138,
/// #139, #151, #158).
fn projection_options(
    drop_furniture: bool,
    infer_lists: bool,
    drop_hidden: bool,
    keep_repeated_header: bool,
    unstyled: bool,
) -> ethos_parser_core::ProjectionOptions {
    let mut options = ethos_parser_core::ProjectionOptions::default();
    options.drop_furniture = drop_furniture;
    options.infer_lists = infer_lists;
    options.drop_flagged = drop_hidden;
    options.keep_repeated_header = keep_repeated_header;
    options.unstyled = unstyled;
    options
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

/// Answer one line of an MCP session, in the process `run_mcp` started for it.
fn run_mcp_call() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match mcp::serve_call(stdin.lock(), stdout.lock()) {
        Ok(()) => ExitCode::from(PROJECTED as u8),
        Err(e) => fail(&EngineError::Io {
            detail: format!("mcp call: {e}"),
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
    hint_ocr(&repr, OCR_ON_RECORD);

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

    let artifact = match ethos_parser_core::to_html_with(
        &repr,
        &profile.parser_version,
        &profile_sha256,
        &profile.html_rule,
        projection_options(
            args.drop_furniture,
            args.infer_lists,
            args.drop_hidden,
            args.keep_repeated_header,
            args.unstyled,
        ),
    ) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };

    match artifact.to_canonical_bytes() {
        Ok(out) => match write_stdout(&out, true) {
            Ok(()) => ExitCode::from(PROJECTED as u8),
            Err(e) => fail(&e),
        },
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
            if let Err(e) = write_stdout(&out, true) {
                return fail(&e);
            }

            // On stderr, deliberately: stdout is the artifact and must stay byte-identical
            // across runs. A consumer that wants this durably reads the representation's
            // `geometry-absent-not-groundable` limitation, which carries the same count. Best
            // effort, as `fail`'s report is: the artifact is already written.
            let mut err = std::io::stderr();
            if projection.omission.is_lossy() {
                let _ = writeln!(
                    err,
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
                let _ = writeln!(
                    err,
                    "engine: {} span(s) withheld — more than the {} `ethos.grounding.v1` admits \
                     [{}]. The artifact carries its elements only (`capabilities.spans` and \
                     `char_offsets` false): every block is still grounded, at block rather than \
                     run granularity.",
                    w.spans, w.limit, w.limitation_code,
                );
            }
            if let Some(o) = projection.elements_omitted {
                let _ = writeln!(
                    err,
                    "engine: {} element(s) omitted from the grounding artifact, and {} span(s) \
                     with them — text longer than the {} bytes, or a locator longer than the {}, \
                     that `ethos.grounding.v1` admits [{}]. The text remains in the \
                     representation; nothing is truncated.",
                    o.elements, o.spans, o.text_limit, o.locator_limit, o.limitation_code,
                );
            }
            if let Some(t) = projection.tables_withheld {
                let _ = writeln!(
                    err,
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

    // Ethos reads the source only once it has judged the artifact valid, so an invalid artifact is
    // reported whatever the source path holds, and a source it cannot read, or over its 256 MiB,
    // refuses only a valid artifact, with no report.
    let checked = match &args.source_artifact {
        None => ethos_parser_grounding::grounding_check(&grounding, None),
        Some(p) => ethos_parser_grounding::grounding_check_reading_source(&grounding, || {
            read_source_within(p, GROUNDING_CHECK_MAX_SOURCE_BYTES)
        }),
    };
    let report = match checked {
        Ok(r) => r,
        // A source that is not a PDF, cannot be read or is too large, or an input past the accepted
        // ceiling. Ethos refuses these before writing any report and so does this: "these bytes are
        // not the source" would be a different and wrong statement about a file that is not a
        // document at all.
        Err(e) => return fail(&e),
    };

    match report.to_canonical_bytes() {
        Ok(bytes) => match write_stdout(&bytes, true) {
            Ok(()) => ExitCode::from(report.exit_code() as u8),
            Err(e) => fail(&e),
        },
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
    // No working-directory sibling: a binary found by where the caller happens to stand is one
    // anybody who controls a parent directory can plant. The pin or PATH, nothing else.
    let binary = match VerifierBinary::resolve(explicit.as_deref(), None) {
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
    if let Err(e) = write_stdout(&relayed.stdout, false) {
        return fail(&e);
    }
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

/// Write an artifact to stdout, or say why it could not be written. A pipe closed early or a
/// full disk leaves a truncated artifact behind, and exit 0 would call that a success.
fn write_stdout(bytes: &[u8], newline: bool) -> Result<(), EngineError> {
    let mut out = std::io::stdout().lock();
    out.write_all(bytes)
        .and_then(|()| {
            if newline {
                out.write_all(b"\n")
            } else {
                Ok(())
            }
        })
        .and_then(|()| out.flush())
        .map_err(|e| EngineError::Io {
            detail: format!("writing to stdout: {e}"),
        })
}

/// Report a failure on stderr and exit 2.
///
/// The artifact never appears on stdout in this path: a partial or absent classification must not
/// be mistaken for a real one by something reading the pipe. The report is best effort: when
/// stderr is a closed pipe `eprintln!` panics, and `panic = "abort"` turned that into SIGABRT,
/// exit 134, in place of this exit 2 (`2>&1 | head` closes both streams at once).
fn fail(e: &EngineError) -> ExitCode {
    let _ = writeln!(std::io::stderr(), "engine: {} [{}]", e, e.code());
    ExitCode::from(COULD_NOT_READ as u8)
}
