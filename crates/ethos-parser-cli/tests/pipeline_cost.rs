//! Where a PDF-to-Markdown call's time goes, stage by stage. An instrument, not a test.
//!
//! ```text
//! ETHOS_BENCH=~/ethos-external-benchmarks/opendataloader-bench \
//!   cargo test -p ethos-parser-cli --release --test pipeline_cost -- --ignored --nocapture
//! ```
//!
//! **Why it exists.** `docs/measurements/liteparse-head-to-head/README.md` measures 0.035 s per
//! document for this engine against 0.026 s for `lit`, over the CLI path a caller uses today:
//! `extract` then `markdown`, two processes with the representation serialised to JSON, written,
//! read and parsed between them. That number cannot say how much of itself is the engine and how
//! much is the plumbing, and the answer decides whether there is anything worth changing — so it
//! is measured here rather than argued about.
//!
//! It is `#[ignore]` because it needs a corpus this repository does not own, and `--release`
//! because a debug build measures the profile nobody ships.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ethos_parser_core::Profile;
use ethos_parser_pdf::Document;

fn corpus() -> Vec<PathBuf> {
    let bench = std::env::var("ETHOS_BENCH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").unwrap())
                .join("ethos-external-benchmarks/opendataloader-bench")
        });
    let mut pdfs: Vec<PathBuf> = std::fs::read_dir(bench.join("pdfs"))
        .expect("corpus: set ETHOS_BENCH")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        .collect();
    pdfs.sort();
    pdfs
}

/// Milliseconds per document, mean over the corpus.
fn per_doc(total: Duration, n: usize) -> f64 {
    total.as_secs_f64() * 1000.0 / n as f64
}

#[test]
#[ignore = "instrument: needs the opendataloader-bench corpus, and --release to mean anything"]
fn where_a_pdf_to_markdown_call_spends_its_time() {
    let docs = corpus();
    assert!(!docs.is_empty(), "no PDFs under $ETHOS_BENCH/pdfs");
    let profile = Profile::default();
    let profile_sha256 = profile.profile_sha256().unwrap();

    let (mut read, mut open, mut extract, mut represent, mut markdown) = (
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
    );
    let (mut serialise, mut parse, mut verify) = (Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut json_bytes = 0usize;

    for path in &docs {
        let t = Instant::now();
        let bytes = std::fs::read(path).unwrap();
        read += t.elapsed();

        let t = Instant::now();
        let doc = Document::open_bytes(&bytes, &profile).unwrap();
        open += t.elapsed();

        let t = Instant::now();
        let artifact = ethos_parser_pdf::extract(&doc, &profile).unwrap();
        extract += t.elapsed();

        let t = Instant::now();
        let repr = ethos_parser_pdf::to_representation(&artifact, &profile).unwrap();
        represent += t.elapsed();

        // What the two-process path pays between the stages, and nothing else: the representation
        // out to JSON and back, and the fingerprint the second process checks because it is
        // reading a record it did not build.
        let t = Instant::now();
        let json = serde_json::to_vec(&repr).unwrap();
        serialise += t.elapsed();
        json_bytes += json.len();

        let t = Instant::now();
        let reread: ethos_parser_core::DocumentRepresentation =
            serde_json::from_slice(&json).unwrap();
        parse += t.elapsed();

        let t = Instant::now();
        reread.verify_fingerprint().unwrap();
        verify += t.elapsed();

        let t = Instant::now();
        let _ = ethos_parser_core::to_markdown(
            &repr,
            &profile.parser_version,
            &profile_sha256,
            &profile.markdown_rule,
        )
        .unwrap();
        markdown += t.elapsed();
    }

    let n = docs.len();
    let engine = read + open + extract + represent + markdown;
    let plumbing = serialise + parse + verify;
    println!("\n  {n} documents, milliseconds per document (mean), release build\n");
    for (label, d) in [
        ("read the file", read),
        ("open the document", open),
        ("extract", extract),
        ("to_representation", represent),
        ("to_markdown", markdown),
    ] {
        println!("    {label:22}  {:.2}", per_doc(d, n));
    }
    println!("    {:22}  {:.2}", "— engine subtotal", per_doc(engine, n));
    for (label, d) in [
        ("serialise to JSON", serialise),
        ("parse it back", parse),
        ("verify_fingerprint", verify),
    ] {
        println!("    {label:22}  {:.2}", per_doc(d, n));
    }
    println!(
        "    {:22}  {:.2}",
        "— plumbing subtotal",
        per_doc(plumbing, n)
    );
    println!(
        "\n    representation {:.0} KB per document; the two-process path also pays two process \
         starts (7.6 ms measured) and writes that JSON to disk.",
        json_bytes as f64 / n as f64 / 1024.0
    );
}
