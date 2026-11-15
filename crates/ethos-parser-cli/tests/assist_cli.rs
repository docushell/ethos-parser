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

//! `extract --assist-answers` (decision #145): a drafting model's replies attached to the record
//! as `Proposed` blocks, read from a file, on PDFs built here and replies made up here. Every
//! refusal is held in `ethos_parser_core::assist`'s own tests too; these hold the command line.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ethos_parser_core::{Sha256Hex, ASSIST_RENDERER, ASSIST_REQUEST_V1};
use serde_json::{json, Value};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_path_buf()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ethos-parser-assist-{label}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn engine(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(args)
        .output()
        .expect("the engine binary runs")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A PDF of the pages given as (page dictionary keys, content stream), drawing with Helvetica.
fn pdf(pages: &[(&str, &str)]) -> Vec<u8> {
    let n = pages.len();
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + 2 * i)).collect();
    let font = 3 + 2 * n;
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")),
    ];
    for (i, (keys, stream)) in pages.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R {keys} /Resources << /Font << /F1 {font} 0 R >> >> \
             /Contents {} 0 R >>",
            4 + 2 * i
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }
    objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".into());
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", n * 2 + 4).as_bytes());
    for o in &offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            n * 2 + 4
        )
        .as_bytes(),
    );
    out
}

/// Three 300 x 400 point pages of made-up text; the third is cropped to 200 x 300 and turned a
/// quarter, so it shows 300 x 200.
fn three_pages() -> Vec<u8> {
    let text = |t: &str| format!("BT /F1 12 Tf 40 300 Td ({t}) Tj ET");
    pdf(&[
        ("/MediaBox [0 0 300 400]", &text("A made-up first page.")),
        ("/MediaBox [0 0 300 400]", &text("A made-up second page.")),
        (
            "/MediaBox [0 0 300 400] /CropBox [50 50 250 350] /Rotate 90",
            &text("A made-up third page."),
        ),
    ])
}

/// Made-up replies to pages 1 and 3 of `source`.
fn replies(source: &[u8]) -> Value {
    let pixels = Sha256Hex::of_bytes(b"made-up pixels").to_string();
    json!({
        "ethos_assist_answers": 1,
        "source_sha256": Sha256Hex::of_bytes(source).to_string(),
        "renderer": ASSIST_RENDERER,
        "request": ASSIST_REQUEST_V1,
        "model": {"id": "example/made-up-model", "version": "not-reported"},
        "pages": [
            {
                "page": 1, "points": [30000, 40000], "dpi": 200, "width": 833, "height": 1111,
                "pixels_sha256": pixels, "finish_reason": "stop",
                "reply": "![title](100,50,900,90) # Made-up heading\n\n\
                          ![text](100,100,900,200) A made-up sentence."
            },
            {
                "page": 3, "points": [30000, 20000], "dpi": 200, "width": 833, "height": 555,
                "pixels_sha256": pixels, "finish_reason": "length",
                "reply": "![table](50,50,950,950) <table><tr><td>7</td></tr></table>"
            }
        ]
    })
}

/// The PDF and its replies file, written to `dir`.
fn write(dir: &Path, source: &[u8], answers: &Value) -> (String, String) {
    let pdf = dir.join("doc.pdf");
    let file = dir.join("doc.answers.json");
    std::fs::write(&pdf, source).expect("write");
    std::fs::write(&file, serde_json::to_vec(answers).expect("JSON")).expect("write");
    (
        pdf.to_str().expect("utf-8").into(),
        file.to_str().expect("utf-8").into(),
    )
}

fn json_of(out: &Output) -> Value {
    assert_eq!(out.status.code(), Some(0), "{}", stderr(out));
    serde_json::from_slice(&out.stdout).expect("a JSON artifact")
}

/// **The replies attach as `Proposed` blocks beside the engine's record, and nothing else moves**:
/// the record differs from the engine-only one in its profile hash, its `assist` key and its
/// fingerprint, and its `ground` is the engine-only one but for those two hashes (design §8.4); its
/// Markdown and HTML are too, but for their `proposed` census, as these replies hold no table or
/// chart to add and swap no page (decision #147).
/// Pages 1 and 3 were asked, so page 2 is absent; page 3 is cropped and turned, and the size it was
/// rendered at is the turned crop's. Page 3's reply was cut at the cap, so it is `truncated`
/// (decision #146), its block kept in the record.
#[test]
fn replies_attach_as_proposed_blocks_and_move_nothing_else() {
    let dir = scratch("attach");
    let source = three_pages();
    let (pdf, answers) = write(&dir, &source, &replies(&source));

    let engine_only = engine(&["extract", &pdf]);
    let assisted = engine(&["extract", &pdf, "--assist-answers", &answers]);
    let (mut plain, mut with) = (json_of(&engine_only), json_of(&assisted));

    let assist = with["representation"]
        .as_object_mut()
        .expect("a payload")
        .remove("assist")
        .expect("an assist key");
    let pages: Vec<(&str, &str, usize)> = assist["pages"]
        .as_array()
        .expect("pages")
        .iter()
        .map(|p| {
            (
                p["page"].as_str().expect("an id"),
                p["outcome"].as_str().expect("an outcome"),
                p["blocks"].as_array().expect("blocks").len(),
            )
        })
        .collect();
    assert_eq!(pages, [("p1", "read", 2), ("p3", "truncated", 1)]);
    assert_eq!(assist["pages"][1]["blocks"][0]["id"], "m3");
    assert_eq!(assist["pages"][1]["blocks"][0]["derivation"], "proposed");
    assert_eq!(assist["model"]["id"], "example/made-up-model");

    for v in [&mut plain, &mut with] {
        v["representation"]["identity"]["profile_sha256"] = Value::Null;
        v["representation_c14n_sha256"] = Value::Null;
    }
    assert_eq!(
        plain, with,
        "only the hash, the assist key and the fingerprint differ"
    );
    assert_ne!(
        json_of(&engine_only)["representation"]["identity"]["profile_sha256"],
        json_of(&assisted)["representation"]["identity"]["profile_sha256"]
    );

    // Projected and grounded, it is the engine's but for the two hashes the record carries and the
    // projections' census of the blocks they leave out; `ground` reads nodes alone.
    let hashes = |out: &Output| {
        let v = json_of(out);
        [
            v["representation"]["identity"]["profile_sha256"].clone(),
            v["representation_c14n_sha256"].clone(),
        ]
        .map(|h| h.as_str().expect("a hash").to_owned())
    };
    let (plain_hashes, with_hashes) = (hashes(&engine_only), hashes(&assisted));
    let (plain_path, with_path) = (dir.join("plain.json"), dir.join("with.json"));
    std::fs::write(&plain_path, &engine_only.stdout).expect("write");
    std::fs::write(&with_path, &assisted.stdout).expect("write");
    for command in ["markdown", "html", "ground"] {
        let run = |path: &Path| {
            let out = engine(&[command, path.to_str().expect("utf-8")]);
            assert_eq!(out.status.code(), Some(0), "{command}: {}", stderr(&out));
            String::from_utf8(out.stdout).expect("utf-8")
        };
        let mut with = run(&with_path);
        for (theirs, ours) in with_hashes.iter().zip(&plain_hashes) {
            with = with.replace(theirs.as_str(), ours);
        }
        let mut with: Value = serde_json::from_str(&with).expect("JSON");
        let census = with
            .as_object_mut()
            .expect("an artifact")
            .remove("proposed");
        assert_eq!(census.is_some(), command != "ground", "{command}");
        let plain: Value = serde_json::from_str(&run(&plain_path)).expect("JSON");
        assert!(plain.get("proposed").is_none(), "{command}");
        assert_eq!(plain, with, "{command}");
    }
}

/// **Two runs with one PDF and one replies file give identical bytes.**
#[test]
fn the_same_pdf_and_replies_give_identical_bytes() {
    let dir = scratch("twice");
    let source = three_pages();
    let (pdf, answers) = write(&dir, &source, &replies(&source));
    let run = || engine(&["extract", &pdf, "--assist-answers", &answers]);
    let (a, b) = (run(), run());
    assert_eq!(a.status.code(), Some(0), "{}", stderr(&a));
    assert_eq!(a.stdout, b.stdout);
}

/// **A replies file that does not answer this PDF is refused by name, with nothing on stdout**,
/// and so is one naming another model than `--assist-model`; the turned crop's size, untouched by
/// the turn, is another page.
#[test]
fn replies_that_do_not_answer_this_pdf_are_refused_by_name() {
    let dir = scratch("refused");
    let source = three_pages();
    let mut cases: Vec<(&str, Value, Vec<&str>)> = Vec::new();
    let mut unknown = replies(&source);
    unknown["server"] = json!("http://127.0.0.1:8089");
    cases.push(("assist-answers-unreadable", unknown, vec![]));
    let mut other = replies(&source);
    other["source_sha256"] = json!(Sha256Hex::of_bytes(b"another PDF").to_string());
    cases.push(("assist-answers-other-source", other, vec![]));
    let mut renderer = replies(&source);
    renderer["renderer"] = json!("another renderer 1.0; pages-400dpi-5mp-v1");
    cases.push(("assist-answers-other-renderer", renderer, vec![]));
    let mut request = replies(&source);
    request["request"] = json!("another-request-v1");
    cases.push(("assist-answers-other-request", request, vec![]));
    let mut path = replies(&source);
    path["model"]["id"] = json!("~/made-up-model");
    cases.push(("assist-answers-model-is-path", path, vec![]));
    let mut processor = replies(&source);
    processor["model"]["id"] = json!("Ethos-Parser");
    cases.push(("assist-answers-model-is-processor", processor, vec![]));
    let mut past = replies(&source);
    past["pages"][1]["page"] = json!(4);
    cases.push(("assist-answers-other-page", past, vec![]));
    let mut unturned = replies(&source);
    unturned["pages"][1]["points"] = json!([20000, 30000]);
    cases.push(("assist-answers-other-page", unturned, vec![]));
    cases.push((
        "assist-answers-other-model",
        replies(&source),
        vec!["--assist-model", "example/other-model"],
    ));
    for (name, answers, extra) in cases {
        let (pdf, file) = write(&dir, &source, &answers);
        let mut args = vec!["extract", pdf.as_str(), "--assist-answers", file.as_str()];
        args.extend(extra);
        let out = engine(&args);
        assert_ne!(out.status.code(), Some(0), "{name}");
        assert!(out.stdout.is_empty(), "{name}: stdout carries nothing");
        assert!(stderr(&out).contains(name), "{name}: {}", stderr(&out));
    }

    // The model named, with or without its version, is the file's.
    let (pdf, file) = write(&dir, &source, &replies(&source));
    for named in [
        "example/made-up-model",
        "example/made-up-model@not-reported",
    ] {
        let out = engine(&[
            "extract",
            &pdf,
            "--assist-answers",
            &file,
            "--assist-model",
            named,
        ]);
        assert_eq!(out.status.code(), Some(0), "{named}: {}", stderr(&out));
    }
    // A file over the byte budget is refused by name, and one past any source's size too: neither
    // is read whole. Both are sparse, so they take no disk.
    let big = dir.join("big.answers.json");
    for len in [
        ethos_parser_core::ASSIST_ANSWERS_MAX_BYTES as u64 + 1,
        3 * 1024 * 1024 * 1024,
    ] {
        std::fs::File::create(&big)
            .and_then(|f| f.set_len(len))
            .expect("a sparse file");
        let out = engine(&[
            "extract",
            &pdf,
            "--assist-answers",
            big.to_str().expect("utf-8"),
        ]);
        assert_ne!(out.status.code(), Some(0), "{len}");
        assert!(out.stdout.is_empty(), "{len}");
        assert!(
            stderr(&out).contains("assist-answers-over-budget"),
            "{len}: {}",
            stderr(&out)
        );
    }
    std::fs::remove_file(&big).expect("remove");

    // And `--assist-model` alone asks nothing.
    let out = engine(&["extract", &pdf, "--assist-model", "example/made-up-model"]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
}

/// **A reply to a page `--max-pages` cut is left out, and its number kept**: the record lists no
/// page for its blocks to hang on, and refusing the whole file would refuse the pages it does list.
#[test]
fn a_reply_to_a_page_the_budget_cut_is_left_out_and_counted() {
    let dir = scratch("budget");
    let source = three_pages();
    let (pdf, answers) = write(&dir, &source, &replies(&source));
    let out = engine(&[
        "extract",
        &pdf,
        "--max-pages",
        "1",
        "--assist-answers",
        &answers,
    ]);
    let assist = &json_of(&out)["representation"]["assist"];
    assert_eq!(assist["pages"].as_array().expect("pages").len(), 1);
    assert_eq!(assist["pages"][0]["page"], "p1");
    assert_eq!(assist["pages_left_out"], json!([3]));
}

/// **Office and image bytes are refused under `--assist-answers`, by name**: a model was asked about
/// a PDF's rendered pages, and these have none.
#[test]
fn office_and_image_bytes_are_refused_under_assist() {
    let dir = scratch("not-pdf");
    let docx = repo_root().join("fixtures/office/simple-paragraphs/document.docx");
    let png = dir.join("page.png");
    std::fs::write(
        &png,
        b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\0\xc8\0\0\0\x64 the rest",
    )
    .expect("write");
    let (_, file) = write(&dir, b"", &replies(b""));
    for path in [docx, png] {
        let out = engine(&[
            "extract",
            path.to_str().expect("utf-8"),
            "--assist-answers",
            &file,
        ]);
        assert_ne!(out.status.code(), Some(0));
        assert!(out.stdout.is_empty());
        assert!(stderr(&out).contains("assist-not-pdf"), "{}", stderr(&out));
    }
}

/// **Under `--ocr` the model's pin stacks on the OCR profile**: the record is read under it, and
/// the replies attach to that record. The recognizer is `tests/fake-ocr.sh`; these pages show text,
/// so it reads none of them.
#[cfg(unix)]
#[test]
fn under_ocr_the_pin_stacks_on_the_ocr_profile() {
    let dir = scratch("ocr");
    let source = three_pages();
    let (pdf, answers) = write(&dir, &source, &replies(&source));
    let identity = serde_json::to_value(ethos_parser_core::RecognizerIdentity::ethos_parser_ocr())
        .expect("serializes");
    let out = Command::new(env!("CARGO_BIN_EXE_ethos-parser"))
        .args(["extract", "--ocr", &pdf, "--assist-answers", &answers])
        .env(
            "ETHOS_PARSER_OCR",
            repo_root().join("crates/ethos-parser-cli/tests/fake-ocr.sh"),
        )
        .env("FAKE_OCR_IDENTITY", identity.to_string())
        .output()
        .expect("the engine binary runs");
    let record = json_of(&out);
    let pin: ethos_parser_core::AssistPin = serde_json::from_value(json!({
        "rule": ethos_parser_core::ASSIST_RULE_V1,
        "guard": ethos_parser_core::ASSIST_GUARD_V1,
        "renderer": ASSIST_RENDERER,
        "request": ASSIST_REQUEST_V1,
        "model": "example/made-up-model",
        "model_version": "not-reported",
    }))
    .expect("a pin");
    let profile = ethos_parser_core::Profile::ocr_v0().with_assist(pin);
    assert_eq!(
        record["representation"]["identity"]["profile_sha256"],
        profile.profile_sha256().expect("hashes").to_string()
    );
}

/// **The guard drops exactly the planted wrong lines, on a page built here** (decision #146,
/// design §8.3): a table read with four wrong values (a digit in an amount, an id and a date, and a
/// comma for the page's point) loses those four cells and binds the rest, the amount the engine
/// split by a space among them; a chart's bar values the page does not print are kept as
/// estimates; an invented line is unsupported; a region the engine read nothing in, or only text
/// it drew invisibly, stays unchecked. Page 2 prints the wrong values where page 1's table stands, so only page 1's runs
/// may bind page 1's blocks; its reply was cut in a loop, which drops its last block.
/// Two made-up pages with planted wrong lines, and replies to them: page 1 a ruled table of values,
/// ids and dates, a chart with labelled bars and a paragraph; page 2 the same table read wrong, its
/// reply cut at the cap inside a looping table.
fn planted() -> (Vec<u8>, Value) {
    let at = |x: u32, y: u32, t: &str| format!("BT /F1 10 Tf {x} {y} Td ({t}) Tj ET\n");
    let grid = |rows: &[[&str; 4]]| {
        let mut s = String::new();
        for (y, row) in (0..).map(|r| 700 - 20 * r).zip(rows) {
            for (x, t) in (0..).map(|c| 72 + 120 * c).zip(row) {
                s += &at(x, y, t);
            }
        }
        s
    };
    let page1 = grid(&[
        ["Item", "Amount", "Code", "Date"],
        ["Alpha", "1,234", "A-113", "2024-03-01"],
        ["Beta", "5.678", "B-220", "2024-04-15"],
        ["Gamma", "9 012", "C-310", "2024-05-30"],
    ]) + "0.5 g 100 400 30 60 re f 150 400 30 90 re f 200 400 30 100 re f 0 g\n"
        + &at(100, 520, "Sales by quarter")
        + &at(107, 465, "42")
        + &at(105, 385, "Q1")
        + &at(155, 385, "Q2")
        + &at(205, 385, "Q3")
        + &at(72, 300, "The made-up paragraph says nothing of note here.")
        // Text drawn invisibly (render mode 3) where the page shows nothing: no reading.
        + "BT 3 Tr /F1 10 Tf 72 100 Td (Figure 3 shows 99 trends) Tj ET\n";
    let page2 = grid(&[
        ["Item", "Amount", "Code", "Date"],
        ["Alpha", "1,243", "A-118", "2024-03-07"],
        ["Beta", "5,678", "B-220", "2024-04-15"],
    ]);
    let source = pdf(&[
        ("/MediaBox [0 0 612 792]", &page1),
        ("/MediaBox [0 0 612 792]", &page2),
    ]);
    let mut answers = replies(&source);
    answers["pages"] = json!([
        {
            "page": 1, "points": [61200, 79200], "dpi": 200, "width": 1700, "height": 2200,
            "pixels_sha256": Sha256Hex::of_bytes(b"made-up pixels").to_string(),
            "finish_reason": "stop",
            "reply": "![table](100,90,850,215) <table>\
                <tr><th>Item</th><th>Amount</th><th>Code</th><th>Date</th></tr>\
                <tr><td>Alpha</td><td>1,243</td><td>A-118</td><td>2024-03-07</td></tr>\
                <tr><td>Beta</td><td>5,678</td><td>B-220</td><td>2024-04-15</td></tr>\
                <tr><td>Gamma</td><td>9012</td><td>C-310</td><td>2024-05-30</td></tr></table>\n\n\
                ![chart](140,320,460,530) <table><tr><th>Quarter</th><th>Sales</th></tr>\
                <tr><td>Q1</td><td>42</td></tr><tr><td>Q2</td><td>57</td></tr>\
                <tr><td>Q3</td><td>61</td></tr></table>\n\n\
                ![text](100,600,900,640) The made-up paragraph says nothing of note here.\n\
                Quarterly revenue rose sharply\n\n\
                ![text](100,800,900,950) Figure 3 shows 12 trends"
        },
        {
            "page": 2, "points": [61200, 79200], "dpi": 200, "width": 1700, "height": 2200,
            "pixels_sha256": Sha256Hex::of_bytes(b"made-up pixels").to_string(),
            "finish_reason": "length",
            "reply": format!(
                "![text](100,100,900,200) Item Amount\n\n![table](100,300,900,900) <table>{}",
                "<tr><td>0</td></tr>\n".repeat(8)
            )
        }
    ]);
    (source, answers)
}

#[test]
fn the_guard_drops_exactly_the_planted_wrong_lines() {
    let (source, answers) = planted();
    let dir = scratch("guard");
    let (pdf, file) = write(&dir, &source, &answers);
    let record = json_of(&engine(&["extract", &pdf, "--assist-answers", &file]));
    let pages = &record["representation"]["assist"]["pages"];

    let verdict = |block: &Value| {
        let g = &block["guard"];
        (g["lines"].clone(), g["bound"].clone(), g["flagged"].clone())
    };
    let blocks = &pages[0]["blocks"];
    assert_eq!(pages[0]["outcome"], "read");
    assert_eq!(
        verdict(&blocks[0]),
        (
            json!(16),
            json!(12),
            json!([
                {"line": 6, "mark": "contradicted", "value": "1,243"},
                {"line": 7, "mark": "contradicted", "value": "118"},
                {"line": 8, "mark": "contradicted", "value": "2024-03-07"},
                {"line": 10, "mark": "contradicted", "value": "5,678"}
            ])
        ),
        "the table"
    );
    assert_eq!(
        verdict(&blocks[1]),
        (
            json!(8),
            json!(6),
            json!([
                {"line": 6, "mark": "estimate", "value": "57"},
                {"line": 8, "mark": "estimate", "value": "61"}
            ])
        ),
        "the chart"
    );
    assert_eq!(
        verdict(&blocks[2]),
        (
            json!(2),
            json!(1),
            json!([{"line": 2, "mark": "unsupported"}])
        ),
        "the paragraph"
    );
    assert_eq!(
        verdict(&blocks[3]),
        (json!(1), json!(0), Value::Null),
        "the region the engine read nothing in"
    );
    assert_eq!(blocks[3]["guard"]["engine_words"], 0);

    assert_eq!(pages[1]["outcome"], "loop");
    assert_eq!(pages[1]["blocks"].as_array().expect("blocks").len(), 2);
}

/// **The projections show what the guard keeps** (decision #147): the model's table is added with
/// its four planted wrong cells emptied, and its chart with its bar values labelled estimates; its
/// paragraphs, the engine's to read, are left out, and so is the looping table its cut reply ends
/// in; each counted, and the same bytes run after run.
#[test]
fn the_projections_show_what_the_guard_keeps() {
    let (source, answers) = planted();
    let dir = scratch("projected");
    let (pdf, file) = write(&dir, &source, &answers);
    let record = dir.join("record.json");
    let out = engine(&["extract", &pdf, "--assist-answers", &file]);
    json_of(&out);
    std::fs::write(&record, &out.stdout).expect("write");
    let record = record.to_str().expect("utf-8");
    let markdown = engine(&["markdown", record]);
    assert_eq!(markdown.stdout, engine(&["markdown", record]).stdout);
    let markdown = json_of(&markdown);
    let html = json_of(&engine(&["html", record]));
    let label = |what: &str| {
        format!(
            "> **Proposed:** {what} by example/made-up-model@not-reported; model text, not read \
             from the document."
        )
    };
    let text = markdown["markdown"].as_str().expect("markdown");
    // Each group one block quote, closed by its end line, the model's rows one quote deeper.
    assert!(text.contains(&format!(
        "{}\n>\n> > | Item | Amount | Code | Date |\n> > | --- | --- | --- | --- |\n> > | Alpha \
         |  |  |  |\n> > | Beta |  | B-220 | 2024-04-15 |\n> > | Gamma | 9012 | C-310 | \
         2024-05-30 |\n>\n> **End of proposed text.**\n\n",
        label("table")
    )));
    assert!(text.contains(&format!(
        "{}\n>\n> > | Quarter | Sales |\n> > | --- | --- |\n> > | Q1 | 42 |\n> > | Q2 | 57 |\n\
         > > | Q3 | 61 |\n>\n> **End of proposed text.**",
        label("chart values estimated")
    )));
    assert!(!text.contains("Quarterly revenue") && !text.contains("| 0 |"));
    let census = |artifact: &Value| {
        artifact["proposed"]["pages"]
            .as_array()
            .expect("pages")
            .iter()
            .map(|p| {
                (
                    p["swapped"].clone(),
                    p["blocks"].clone(),
                    p["left_out"].clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    let not_projected = |n: u32| json!({"code": "proposed-block-not-projected-v1", "count": n});
    let truncated = json!({"code": "proposed-block-truncated-v1", "count": 1});
    let contradicted = json!({"code": "proposed-value-contradicted-v1", "count": 4});
    assert_eq!(
        census(&html),
        [
            (
                json!(false),
                json!(2),
                json!([not_projected(2), contradicted])
            ),
            (json!(false), json!(0), json!([not_projected(1), truncated])),
        ]
    );
    assert_eq!(census(&markdown), census(&html));
    let html = html["html"].as_str().expect("html");
    assert_eq!(html.matches("data-dropped=\"contradicted\"").count(), 4);
    assert_eq!(html.matches("data-estimate=\"true\"").count(), 2);
    let end = "</blockquote>\n<p><strong>End of proposed text.</strong></p>\n</blockquote>\n";
    assert_eq!(html.matches(end).count(), 2);
}

/// **The record names the renderer and request the assist binary uses** (decisions #143-#145):
/// the replies file is refused unless its `renderer` and `request` equal these two constants, so
/// they must be the strings `ethos-parser-assist` writes. That binary lives in its own workspace
/// (`assist/`), so this reads its source rather than linking it.
#[test]
fn the_record_names_what_the_assist_binary_writes() {
    let main = std::fs::read_to_string(repo_root().join("assist/src/main.rs"))
        .expect("assist/src/main.rs is in the repository");
    for (name, value) in [
        ("RENDERER", ASSIST_RENDERER),
        ("REQUEST", ASSIST_REQUEST_V1),
    ] {
        let line = format!("pub const {name}: &str = \"{value}\";");
        assert!(
            main.contains(&line),
            "assist/src/main.rs must declare `{line}`, as the engine reads it"
        );
    }
}

/// **No model file in the repository** (the owner, 2026-11-15; Phase 4b design, test 8.6): the
/// assist lane's model runs in a server the user starts. The recognizer's models in `ocr/models/`,
/// built into its binary (decision #120), are the one exception. `ci/release-artifacts.sh` holds
/// each release archive to its three binaries, LICENSE, NOTICE and README.md.
#[test]
fn the_repository_tracks_no_model_file() {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git ls-files: {}", stderr(&out));
    let files = String::from_utf8(out.stdout).expect("UTF-8 paths");
    let files: Vec<&str> = files.split('\0').filter(|f| !f.is_empty()).collect();
    assert!(
        files.len() > 100,
        "git ls-files listed {} files",
        files.len()
    );
    let weights = [
        "onnx",
        "safetensors",
        "gguf",
        "npz",
        "mlx",
        "pt",
        "pth",
        "ckpt",
    ];
    let models: Vec<&str> = files
        .into_iter()
        .filter(|f| {
            let ext = Path::new(f).extension().and_then(|e| e.to_str());
            let big = || std::fs::metadata(repo_root().join(f)).is_ok_and(|m| m.len() > 1 << 20);
            match ext {
                Some(e) if weights.contains(&e) => !f.starts_with("ocr/models/"),
                Some("bin") => big(),
                _ => false,
            }
        })
        .collect();
    assert!(models.is_empty(), "model files are tracked: {models:?}");
}
