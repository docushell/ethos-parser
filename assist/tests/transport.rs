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

//! `ask` and `check` against a fake OpenAI-compatible server (DESIGN §8.1): a `TcpListener` on
//! 127.0.0.1, port 0, answering from a queue of canned LightOnOCR replies and recording every
//! request. No network beyond loopback: the binary runs with an empty environment (no proxy
//! variable, no key but the test's) and reaches the server through the system `curl`, wrapped
//! by a script that logs curl's argv and caps each request at 3 s.

#![cfg(unix)]

#[path = "../src/digest.rs"]
mod digest;
#[path = "../src/json.rs"]
mod json;

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use json::Json;

const BIN: &str = env!("CARGO_BIN_EXE_ethos-parser-assist");
const CHECK_PAGE: &[u8] = include_bytes!("../data/check-page.pdf");
const KEY: &str = "sk-test-do-not-leak-7f3a";
const REQUEST: &str = "lightonocr-3-grounding-v1";

/// A reply as LightOnOCR-3 writes one for the check page: blocks joined by one blank line.
const REPLY: &str = "![title](117,85,357,113) # Harbor Report\n\n\
    ![text](117,121,734,141) Seven lanterns guard the quiet northern bay.\n\n\
    ![table](117,165,589,267) <table>\n<tr><td>14</td><td>27</td><td>39</td></tr>\n\
    <tr><td>152</td><td>268</td><td>371</td></tr>\n\
    <tr><td>4085</td><td>5190</td><td>6243</td></tr>\n</table>";

// ---- the fake server ------------------------------------------------------------------------

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.0 == name)
            .map(|h| h.1.as_str())
    }
}

/// What the server does with the next chat request.
enum Answer {
    Status(u16, String),
    /// Never answers: the client gives up.
    Hang,
    /// Closes the connection without a reply.
    Close,
    /// A 307 to another address.
    Redirect(String),
    /// Announces a reply over the 64 MB cap, and sends none of it.
    Oversize,
}

fn ok(body: String) -> Answer {
    Answer::Status(200, body)
}

/// An OpenAI chat completion as mlx_vlm.server and vLLM write one.
fn completion(model: &str, finish: &str, content: &str) -> String {
    format!(
        "{{\"id\": \"chatcmpl-1\", \"object\": \"chat.completion\", \"created\": 1763200000, \
         \"model\": {}, \"choices\": [{{\"index\": 0, \"message\": {{\"role\": \"assistant\", \
         \"content\": {}}}, \"finish_reason\": {}}}], \"usage\": {{\"prompt_tokens\": 1200, \
         \"completion_tokens\": 412, \"total_tokens\": 1612}}, \"timings\": {{\"prompt_ms\": 1700.5}}}}",
        json::quote(model),
        json::quote(content),
        json::quote(finish)
    )
}

fn models(ids: &[&str]) -> String {
    let data: Vec<String> = ids
        .iter()
        .map(|id| format!("{{\"id\": {}, \"object\": \"model\"}}", json::quote(id)))
        .collect();
    format!("{{\"object\": \"list\", \"data\": [{}]}}", data.join(", "))
}

struct Server {
    port: u16,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn posts(&self) -> usize {
        let requests = self.requests.lock().unwrap();
        requests.iter().filter(|r| r.method == "POST").count()
    }
}

/// A server listing `listed` at `GET /v1/models` and answering chat requests from `queue`.
fn serve(listed: &[&str], queue: Vec<Answer>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let (seen, listed) = (requests.clone(), models(listed));
    let mut queue = VecDeque::from(queue);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { break };
            let Some(req) = read_request(&mut s) else {
                continue;
            };
            let answer = if req.method == "GET" && req.path == "/v1/models" {
                Answer::Status(200, listed.clone())
            } else {
                queue
                    .pop_front()
                    .unwrap_or(Answer::Status(500, "{}".into()))
            };
            seen.lock().unwrap().push(req);
            match answer {
                Answer::Status(status, body) => {
                    let _ = write!(
                        s,
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
                Answer::Hang => {
                    let _ = s.read_to_end(&mut Vec::new());
                }
                Answer::Close => {}
                Answer::Redirect(to) => {
                    let _ = write!(
                        s,
                        "HTTP/1.1 307 X\r\nLocation: {to}\r\nContent-Length: 0\r\n\r\n"
                    );
                }
                Answer::Oversize => {
                    let _ = write!(s, "HTTP/1.1 200 X\r\nContent-Length: 100000000\r\n\r\n");
                }
            }
        }
    });
    Server { port, requests }
}

fn read_request(s: &mut TcpStream) -> Option<Request> {
    s.set_read_timeout(Some(Duration::from_secs(30))).ok()?;
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if s.read(&mut byte).ok()? == 0 {
            return None;
        }
        head.push(byte[0]);
    }
    let head = String::from_utf8(head).ok()?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, path) = (first.next()?.to_owned(), first.next()?.to_owned());
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let mut req = Request {
        method,
        path,
        headers,
        body: Vec::new(),
    };
    // curl asks first for a body over 1 MB.
    if req
        .header("expect")
        .is_some_and(|e| e.eq_ignore_ascii_case("100-continue"))
    {
        s.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").ok()?;
    }
    let len = req.header("content-length").and_then(|v| v.parse().ok());
    req.body = vec![0; len.unwrap_or(0)];
    s.read_exact(&mut req.body).ok()?;
    Some(req)
}

/// A port nothing listens on.
fn dead_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

// ---- running the binary ---------------------------------------------------------------------

struct Tools {
    /// Holds `curl`: a script logging its argv and environment (a builtin: PATH holds no `env`), then running the real curl
    /// capped at 3 s.
    curl: PathBuf,
    /// Holds nothing: no curl.
    none: PathBuf,
}

/// Written once, before any test spawns a process, so no exec meets a file still open for
/// writing (ETXTBSY on Linux).
fn tools() -> &'static Tools {
    static TOOLS: OnceLock<Tools> = OnceLock::new();
    TOOLS.get_or_init(|| {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("assist-transport");
        let path = std::env::var_os("PATH").unwrap_or_default();
        let real = std::env::split_paths(&path)
            .map(|d| d.join("curl"))
            .find(|c| c.is_file())
            .expect("the tests need the system curl");
        let (curl, none) = (root.join("curl"), root.join("none"));
        std::fs::create_dir_all(&curl).unwrap();
        std::fs::create_dir_all(&none).unwrap();
        let script = curl.join("curl");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$ETHOS_TEST_CURL_ARGV\"\n\
                 export -p >> \"$ETHOS_TEST_CURL_ARGV.env\"\nexec '{}' \"$@\" --max-time 3\n",
                real.display()
            ),
        )
        .unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        std::fs::set_permissions(&script, perms).unwrap();
        Tools { curl, none }
    })
}

/// A fresh directory for one test.
fn scratch(name: &str) -> PathBuf {
    tools();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("assist-transport")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
    /// curl's argv, one argument a line, every call.
    argv: String,
    /// curl's environment, every call.
    env: String,
}

/// The binary, run in `dir` with nothing in its environment but `PATH` (`curl_dir`), `TMPDIR`
/// (`dir`) and `env`.
fn run_with(dir: &Path, curl_dir: &Path, env: &[(&str, &str)], args: &[&str], stdin: &[u8]) -> Run {
    let log = dir.join("curl-argv.txt");
    let env_log = dir.join("curl-argv.txt.env");
    let _ = std::fs::remove_file(&log);
    let _ = std::fs::remove_file(&env_log);
    let mut cmd = Command::new(BIN);
    cmd.args(args)
        .env_clear()
        .env("PATH", curl_dir)
        .env("TMPDIR", dir)
        .env("ETHOS_TEST_CURL_ARGV", &log)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .envs(env.iter().copied());
    let mut child = cmd.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(stdin);
    let out = child.wait_with_output().unwrap();
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        argv: std::fs::read_to_string(&log).unwrap_or_default(),
        env: std::fs::read_to_string(&env_log).unwrap_or_default(),
    }
}

fn run(dir: &Path, key: Option<&str>, args: &[&str], stdin: &[u8]) -> Run {
    let env: Vec<(&str, &str)> = key
        .map(|k| ("ETHOS_PARSER_ASSIST_KEY", k))
        .into_iter()
        .collect();
    run_with(dir, &tools().curl, &env, args, stdin)
}

/// `ask SERVER answers.json PAGES [MODEL] <pdf` in `dir`.
fn ask(dir: &Path, server: &str, pages: &str, model: Option<&str>, pdf: &[u8]) -> Run {
    let mut args = vec!["ask", server, "answers.json", pages];
    args.extend(model);
    run(dir, None, &args, pdf)
}

fn answers(dir: &Path) -> Json {
    let text = std::fs::read(dir.join("answers.json")).unwrap();
    let v = Json::parse(&text).expect("the replies file is JSON");
    assert_eq!(
        v.canonical().as_bytes(),
        text,
        "the replies file is canonical"
    );
    v
}

fn page(v: &Json, n: i64) -> &Json {
    let pages = v.get("pages").and_then(Json::arr).unwrap();
    pages
        .iter()
        .find(|p| p.get("page").and_then(Json::int) == Some(n))
        .unwrap_or_else(|| panic!("no page {n}"))
}

fn str_of<'a>(v: &'a Json, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Json::str)
}

/// `render`'s PNG of `pdf`'s page `n`, and its `dpi width height` line.
fn rendered(dir: &Path, pdf: &[u8], n: usize) -> (Vec<u8>, String) {
    let src = dir.join("rendered.pdf");
    std::fs::write(&src, pdf).unwrap();
    let out = dir.join("rendered.png");
    let r = run(
        dir,
        None,
        &[
            "render",
            src.to_str().unwrap(),
            &n.to_string(),
            out.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    (std::fs::read(out).unwrap(), r.stdout.trim().to_owned())
}

/// A PDF of made-up pages, each `(width, height)` points with one line of text.
fn pdf(pages: &[(u32, u32)]) -> Vec<u8> {
    let mut objs = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        String::new(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut kids = Vec::new();
    for (i, &(w, h)) in pages.iter().enumerate() {
        let content = format!("BT /F1 12 Tf 8 8 Td (Page {}) Tj ET", i + 1);
        kids.push(format!("{} 0 R", objs.len() + 1));
        objs.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] \
             /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            objs.len() + 2
        ));
        objs.push(format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len()
        ));
    }
    objs[1] = format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    );
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objs.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
    for o in offsets {
        out.extend(format!("{o:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .bytes(),
    );
    out
}

fn three_pages() -> Vec<u8> {
    pdf(&[(144, 72), (72, 144), (100, 100)])
}

// ---- ask ------------------------------------------------------------------------------------

/// The request is decision #123's, byte for byte, with `render`'s PNG; the reply is stored
/// verbatim, with the page's size and digests, in the replies file's shape (DESIGN §2).
#[test]
fn ask_sends_decision_123s_request_and_stores_the_reply() {
    let dir = scratch("request");
    let server = serve(
        &[],
        vec![ok(completion("LightOnOCR-3-0.8B", "stop", REPLY))],
    );
    let r = ask(
        &dir,
        &server.url(),
        "1",
        Some("LightOnOCR-3-0.8B@36d8636"),
        CHECK_PAGE,
    );
    assert_eq!(r.code, 0, "{}", r.stderr);

    let (png, size) = rendered(&dir, CHECK_PAGE, 1);
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 1, "one page, one request, no listing");
    let req = &requests[0];
    assert_eq!(
        (req.method.as_str(), req.path.as_str()),
        ("POST", "/v1/chat/completions")
    );
    assert_eq!(req.header("content-type"), Some("application/json"));
    assert_eq!(req.header("authorization"), None, "no key, no header");
    let want = format!(
        "{{\"model\":\"LightOnOCR-3-0.8B\",\"max_tokens\":12288,\"temperature\":0.0,\
         \"stream\":false,\"messages\":[{{\"role\":\"user\",\"content\":[{{\"type\":\
         \"image_url\",\"image_url\":{{\"url\":\"data:image/png;base64,{}\"}}}},{{\"type\":\
         \"text\",\"text\":\"grounding\"}}]}}]}}",
        digest::base64(&png)
    );
    assert!(
        req.body == want.as_bytes(),
        "the request body is decision #123's"
    );

    let v = answers(&dir);
    assert_eq!(v.get("ethos_assist_answers").and_then(Json::int), Some(1));
    assert_eq!(
        str_of(&v, "source_sha256"),
        Some(digest::sha256(CHECK_PAGE).as_str())
    );
    let identity = run(&dir, None, &["identity"], b"").stdout;
    let identity = Json::parse(identity.as_bytes()).unwrap();
    assert_eq!(str_of(&v, "renderer"), str_of(&identity, "renderer"));
    assert_eq!(str_of(&v, "request"), Some(REQUEST));
    let model = v.get("model").unwrap();
    assert_eq!(str_of(model, "asked"), Some("LightOnOCR-3-0.8B"));
    assert_eq!(str_of(model, "id"), Some("LightOnOCR-3-0.8B"));
    assert_eq!(str_of(model, "version"), Some("36d8636"));
    let p = page(&v, 1);
    let ints = |k: &str| p.get(k).and_then(Json::int).unwrap();
    let points: Vec<i64> = p
        .get("points")
        .and_then(Json::arr)
        .unwrap()
        .iter()
        .filter_map(Json::int)
        .collect();
    assert_eq!(points, [61200, 79200], "US Letter, in centipoints");
    assert_eq!(
        format!("{} {} {}", ints("dpi"), ints("width"), ints("height")),
        size
    );
    assert_eq!(
        str_of(p, "pixels_sha256"),
        Some(digest::sha256(&png).as_str())
    );
    assert_eq!(str_of(p, "finish_reason"), Some("stop"));
    assert_eq!(str_of(p, "reply"), Some(REPLY));
    assert_eq!(p.get("error"), None);

    let file = std::fs::read_to_string(dir.join("answers.json")).unwrap();
    for out in [&file, &r.stdout] {
        assert!(
            !out.contains("127.0.0.1"),
            "the server's address is never written"
        );
    }
    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.starts_with("ethos-parser-assist-") || n.ends_with(".part"))
        .collect();
    assert!(left.is_empty(), "the request body is removed: {left:?}");
}

/// The key goes in a header from curl's stdin: never curl's argv or environment, the file,
/// stdout or stderr.
#[test]
fn the_key_reaches_the_server_only_in_a_header() {
    let dir = scratch("key");
    let server = serve(
        &["LightOnOCR-3-0.8B"],
        vec![ok(completion("LightOnOCR-3-0.8B", "stop", REPLY))],
    );
    let r = run(
        &dir,
        Some(KEY),
        &["ask", &server.url(), "answers.json", "1"],
        CHECK_PAGE,
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 2, "the listing, then the page");
    let bearer = format!("Bearer {KEY}");
    for req in requests.iter() {
        assert_eq!(req.header("authorization"), Some(bearer.as_str()));
    }
    assert_eq!(r.argv, "-q\n-K\n-\n".repeat(2), "curl's argv is fixed");
    let file = std::fs::read_to_string(dir.join("answers.json")).unwrap();
    assert!(r.env.contains("PATH="), "curl's environment is logged");
    for (what, text) in [
        ("argv", &r.argv),
        ("environment", &r.env),
        ("file", &file),
        ("stdout", &r.stdout),
        ("stderr", &r.stderr),
    ] {
        assert!(!text.contains(KEY), "the key is in curl's {what}");
    }

    let c = run(&dir, Some(KEY), &["check", &server.url()], b"");
    assert!(!c.stdout.contains(KEY) && !c.stderr.contains(KEY) && !c.argv.contains(KEY));

    let bad = run(
        &dir,
        Some("sk-1\nurl = \"file:///etc/passwd\""),
        &["ask", &server.url(), "answers.json", "1"],
        CHECK_PAGE,
    );
    assert_eq!(bad.code, 5);
    assert!(
        bad.stderr.contains("key-not-a-header-value"),
        "{}",
        bad.stderr
    );
    assert!(bad.argv.is_empty(), "refused before curl runs");
}

/// The same replies give the same bytes: no address, key or timing is written. Pages come back
/// sorted, and a page not asked is not in the file.
#[test]
fn two_runs_write_the_same_bytes() {
    let mut files = Vec::new();
    for run_no in 0..2 {
        let dir = scratch(&format!("same-{run_no}"));
        let server = serve(
            &["LightOnOCR-3-0.8B"],
            vec![
                ok(completion(
                    "LightOnOCR-3-0.8B",
                    "stop",
                    "![text](1,2,3,4) one",
                )),
                ok(completion(
                    "LightOnOCR-3-0.8B",
                    "stop",
                    "![text](5,6,7,8) three",
                )),
            ],
        );
        let r = ask(&dir, &server.url(), "3,1", None, &three_pages());
        assert_eq!(r.code, 0, "{}", r.stderr);
        assert_eq!(server.posts(), 2);
        let v = answers(&dir);
        let pages: Vec<i64> = v
            .get("pages")
            .and_then(Json::arr)
            .unwrap()
            .iter()
            .filter_map(|p| p.get("page").and_then(Json::int))
            .collect();
        assert_eq!(pages, [1, 3]);
        assert_eq!(str_of(page(&v, 1), "reply"), Some("![text](1,2,3,4) one"));
        assert_eq!(
            str_of(v.get("model").unwrap(), "version"),
            Some("not-reported")
        );
        files.push(std::fs::read(dir.join("answers.json")).unwrap());
    }
    assert!(files[0] == files[1], "two runs, two different files");
}

/// A run that stopped resumes: a page with a reply is kept, a page with an error is asked again.
/// A timeout and a non-200 reply are recorded by name.
#[test]
fn a_stopped_run_resumes_and_retries_its_errors() {
    let dir = scratch("resume");
    let pdf = three_pages();
    let first = serve(
        &[],
        vec![
            ok(completion("m", "stop", "![text](1,1,2,2) first")),
            Answer::Status(503, "{\"error\": \"busy\"}".into()),
            Answer::Hang,
        ],
    );
    let r = ask(&dir, &first.url(), "1,2,3", Some("m"), &pdf);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let v = answers(&dir);
    assert_eq!(str_of(page(&v, 2), "error"), Some("http 503"));
    assert_eq!(str_of(page(&v, 3), "error"), Some("timeout"));
    for n in [2, 3] {
        assert_eq!(page(&v, n).get("reply"), None);
        assert_eq!(page(&v, n).get("finish_reason"), None);
    }
    assert!(r.stderr.contains("page 3: timeout"), "{}", r.stderr);

    let second = serve(
        &[],
        vec![
            ok(completion("m", "stop", "![text](1,1,2,2) second")),
            ok(completion("m", "stop", "![text](1,1,2,2) third")),
        ],
    );
    let r = ask(&dir, &second.url(), "1,2,3", Some("m"), &pdf);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(second.posts(), 2, "page 1 is not asked again");
    assert!(r.stderr.contains("page 1: answered before; kept"));
    let v = answers(&dir);
    let replies: Vec<_> = (1..=3).map(|n| str_of(page(&v, n), "reply")).collect();
    assert_eq!(
        replies,
        [
            Some("![text](1,1,2,2) first"),
            Some("![text](1,1,2,2) second"),
            Some("![text](1,1,2,2) third")
        ]
    );
}

/// Another model on resume is refused, by the server's id or by the version named, and the
/// file is left as it was.
#[test]
fn a_model_change_on_resume_is_refused() {
    let dir = scratch("model-change");
    let pdf = three_pages();
    let one = serve(
        &[],
        vec![ok(completion("m-one", "stop", "![text](1,1,2,2) a"))],
    );
    assert_eq!(ask(&dir, &one.url(), "1", Some("m-one@v1"), &pdf).code, 0);
    let before = std::fs::read(dir.join("answers.json")).unwrap();

    // The same name, served by another model.
    let two = serve(
        &[],
        vec![ok(completion("m-two", "stop", "![text](1,1,2,2) b"))],
    );
    let r = ask(&dir, &two.url(), "1,2", Some("m-one@v1"), &pdf);
    assert_eq!(r.code, 5);
    assert!(
        r.stderr.contains("assist-answers-other-model"),
        "{}",
        r.stderr
    );
    // The id the server reports is known only from a reply: page 2 was asked, and not kept.
    assert_eq!(two.posts(), 1);

    let three = serve(&[], vec![]);
    let r = ask(&dir, &three.url(), "2", Some("m-one@v2"), &pdf);
    assert_eq!(r.code, 5);
    assert!(
        r.stderr.contains("assist-answers-other-model"),
        "{}",
        r.stderr
    );
    assert_eq!(three.posts(), 0, "refused before any page is asked");
    assert!(std::fs::read(dir.join("answers.json")).unwrap() == before);
}

/// **A version left out is not compared**, as `extract --assist-answers --assist-model ID` does
/// not compare it: a file of `m-one@v1`'s replies resumes under `m-one`, and keeps `v1`; another
/// version named is still refused.
#[test]
fn a_version_left_out_is_not_compared_on_resume() {
    let dir = scratch("version-left-out");
    let pdf = three_pages();
    let one = serve(
        &[],
        vec![ok(completion("m-one", "stop", "![text](1,1,2,2) a"))],
    );
    assert_eq!(ask(&dir, &one.url(), "1", Some("m-one@v1"), &pdf).code, 0);
    let two = serve(
        &["m-one"],
        vec![ok(completion("m-one", "stop", "![text](1,1,2,2) b"))],
    );
    for model in [Some("m-one"), None] {
        let r = ask(&dir, &two.url(), "1,2", model, &pdf);
        assert_eq!(r.code, 0, "{model:?}: {}", r.stderr);
    }
    assert_eq!(two.posts(), 1, "page 2 asked once, page 1 kept");
    let v = answers(&dir);
    assert_eq!(str_of(v.get("model").unwrap(), "version"), Some("v1"));
    let r = ask(&dir, &two.url(), "3", Some("m-one@v2"), &pdf);
    assert_eq!(r.code, 5);
    assert!(
        r.stderr.contains("assist-answers-other-model"),
        "{}",
        r.stderr
    );
}

/// **A file of errors only holds no reply to keep** (review): it resumes under another version
/// or another model named, and takes that model's id and version.
#[test]
fn a_file_of_errors_only_takes_the_model_named() {
    let dir = scratch("errors-only");
    let pdf = three_pages();
    let busy = || Answer::Status(503, "{\"error\": \"busy\"}".into());
    let one = serve(&[], vec![busy()]);
    assert_eq!(ask(&dir, &one.url(), "1", Some("m-one@v1"), &pdf).code, 0);

    let two = serve(&[], vec![busy()]);
    let r = ask(&dir, &two.url(), "1", Some("m-one@v2"), &pdf);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let model = answers(&dir).get("model").cloned().unwrap();
    assert_eq!(str_of(&model, "version"), Some("v2"));

    let three = serve(
        &[],
        vec![ok(completion("m-two", "stop", "![text](1,1,2,2) a"))],
    );
    let r = ask(&dir, &three.url(), "1", Some("m-two"), &pdf);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let model = answers(&dir).get("model").cloned().unwrap();
    assert_eq!(str_of(&model, "id"), Some("m-two"));
    assert_eq!(str_of(&model, "version"), Some("not-reported"));
}

/// Saved replies are kept only for the model they are from (review): another name is refused
/// before any page is passed over as answered, even when every page asked is answered and no
/// version is named. A server's alias resumes under the name asked or the id it reported.
#[test]
fn saved_replies_are_kept_only_for_their_model() {
    let dir = scratch("model-asked");
    let pdf = three_pages();
    // `alias` served as `org/model`, as vLLM's `--served-model-name` or a cloud API may answer.
    let one = serve(
        &[],
        vec![ok(completion("org/model", "stop", "![text](1,1,2,2) a"))],
    );
    assert_eq!(ask(&dir, &one.url(), "1", Some("alias"), &pdf).code, 0);
    let model = answers(&dir).get("model").cloned().unwrap();
    assert_eq!(str_of(&model, "asked"), Some("alias"));
    assert_eq!(str_of(&model, "id"), Some("org/model"));
    let before = std::fs::read(dir.join("answers.json")).unwrap();

    let none = serve(&[], vec![]);
    let r = ask(&dir, &none.url(), "1", Some("other"), &pdf);
    assert_eq!(r.code, 5, "{}", r.stderr);
    assert!(
        r.stderr.contains("assist-answers-other-model"),
        "{}",
        r.stderr
    );
    assert!(std::fs::read(dir.join("answers.json")).unwrap() == before);
    for name in ["alias", "org/model"] {
        let r = ask(&dir, &none.url(), "1", Some(name), &pdf);
        assert_eq!(r.code, 0, "{name}: {}", r.stderr);
        assert!(r.stderr.contains("page 1: answered before; kept"));
    }
    assert_eq!(none.posts(), 0);

    let two = serve(
        &[],
        vec![ok(completion("org/model", "stop", "![text](1,1,2,2) b"))],
    );
    assert_eq!(ask(&dir, &two.url(), "1,2", Some("alias"), &pdf).code, 0);
    assert_eq!(two.posts(), 1);
}

/// A reply cut at `max_tokens` is kept as written, `finish_reason: length`, and named.
#[test]
fn a_capped_reply_passes_through_as_length() {
    let dir = scratch("length");
    let looped = "![table](1,1,9,9) <table><tr><td>1</td></tr><tr><td>1</td></tr><tr><td>1";
    let server = serve(&[], vec![ok(completion("m", "length", looped))]);
    // A scheme in capitals is still http.
    let url = server.url().replacen("http", "HTTP", 1);
    let r = ask(&dir, &url, "1", Some("m"), CHECK_PAGE);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert!(r.stderr.contains("page 1: reply-capped"), "{}", r.stderr);
    let v = answers(&dir);
    assert_eq!(str_of(page(&v, 1), "finish_reason"), Some("length"));
    assert_eq!(str_of(page(&v, 1), "reply"), Some(looped));
}

/// Each way a request fails has its own name.
#[test]
fn transport_failures_are_named() {
    let dir = scratch("failures");
    let pdf = three_pages();

    let down = format!("http://127.0.0.1:{}", dead_port());
    let r = ask(&dir, &down, "1", Some("m"), &pdf);
    assert_eq!(r.code, 4, "{}", r.stderr);
    assert!(r.stderr.contains("server-unreachable"));
    assert!(!dir.join("answers.json").exists(), "nothing was answered");

    let server = serve(&[], vec![ok(completion("m", "stop", "x"))]);
    let r = run_with(
        &dir,
        &tools().none,
        &[],
        &["ask", &server.url(), "answers.json", "1", "m"],
        &pdf,
    );
    assert_eq!(r.code, 5);
    assert!(r.stderr.contains("curl-not-found"), "{}", r.stderr);

    let elsewhere = serve(&[], vec![]);
    let server = serve(
        &[],
        vec![
            ok("<html>not a completion</html>".into()),
            ok("{\"choices\": []}".into()),
            Answer::Close,
        ],
    );
    let r = ask(&dir, &server.url(), "1,2,3", Some("m"), &pdf);
    assert_eq!(r.code, 0, "{}", r.stderr);
    let v = answers(&dir);
    for n in [1, 2] {
        assert_eq!(str_of(page(&v, n), "error"), Some("reply-unreadable"));
    }
    // curl's "empty reply from server".
    assert_eq!(str_of(page(&v, 3), "error"), Some("curl-exit-52"));

    // A redirect is not followed: the page stays on the server named.
    std::fs::remove_file(dir.join("answers.json")).unwrap();
    let to = format!("{}/v1/chat/completions", elsewhere.url());
    let server = serve(&[], vec![Answer::Redirect(to), Answer::Oversize]);
    let r = ask(&dir, &server.url(), "1,2", Some("m"), &pdf);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(elsewhere.requests.lock().unwrap().len(), 0);
    let v = answers(&dir);
    assert_eq!(str_of(page(&v, 1), "error"), Some("http 307"));
    assert_eq!(str_of(page(&v, 2), "error"), Some("reply-too-large"));
}

/// A loopback server is never reached through a proxy, whatever the environment says: the page
/// does not leave the machine.
#[test]
fn a_loopback_server_is_never_proxied() {
    let dir = scratch("proxy");
    let proxy = serve(&["m"], vec![]);
    let to = proxy.url();
    for var in ["http_proxy", "HTTPS_PROXY", "ALL_PROXY", "all_proxy"] {
        let target = serve(
            &["m"],
            vec![ok(completion("m", "stop", "![text](1,1,2,2) a"))],
        );
        let env = [(var, to.as_str()), ("ETHOS_PARSER_ASSIST_KEY", KEY)];
        let args = ["ask", &target.url(), "answers.json", "1"];
        let r = run_with(&dir, &tools().curl, &env, &args, CHECK_PAGE);
        assert_eq!(r.code, 0, "{var}: {}", r.stderr);
        assert_eq!(target.posts(), 1, "{var}");
        let _ = std::fs::remove_file(dir.join("answers.json"));
    }
    // Nor any other address of 127.0.0.0/8 (review). Nothing listens at 127.0.0.2, so curl, gone
    // there itself, finds no server: refused on Linux, timed out on macOS.
    let env = [
        ("http_proxy", to.as_str()),
        ("ETHOS_PARSER_ASSIST_KEY", KEY),
    ];
    let server = format!("http://127.0.0.2:{}", dead_port());
    let args = ["ask", &server, "answers.json", "1", "m"];
    let r = run_with(&dir, &tools().curl, &env, &args, CHECK_PAGE);
    assert!(matches!(r.code, 0 | 4), "{}", r.stderr);
    assert!(
        r.stderr.contains("server-unreachable") || r.stderr.contains("page 1: timeout"),
        "{}",
        r.stderr
    );
    assert_eq!(
        proxy.requests.lock().unwrap().len(),
        0,
        "the proxy saw a request"
    );
}

/// The key never crosses the network unencrypted (review): with a key, an `http://` server off
/// loopback is refused by name before curl runs, for the listing, a page and `check` alike. The
/// addresses are this machine's by another name than loopback's, so a run that is not refused
/// still sends nothing off it.
#[test]
fn a_key_is_never_sent_over_http_off_loopback() {
    let dir = scratch("key-in-clear");
    let port = dead_port();
    for server in [
        format!("http://0.0.0.0:{port}"),
        format!("HTTP://[::]:{port}"),
        format!("http://127.0.0.1@0.0.0.0:{port}"),
    ] {
        let server = server.as_str();
        let page = run(
            &dir,
            Some(KEY),
            &["ask", server, "answers.json", "1", "m"],
            CHECK_PAGE,
        );
        let listing = run(
            &dir,
            Some(KEY),
            &["ask", server, "answers.json", "1"],
            CHECK_PAGE,
        );
        for r in [&page, &listing] {
            assert_eq!(r.code, 5, "{server}: {}", r.stderr);
            assert!(
                r.stderr.contains("key-needs-https"),
                "{server}: {}",
                r.stderr
            );
        }
        let c = run(&dir, Some(KEY), &["check", server], b"");
        assert_eq!(c.code, 2, "{server}: {}", c.stdout);
        assert!(
            c.stdout.contains("key-needs-https"),
            "{server}: {}",
            c.stdout
        );
        for r in [&page, &listing, &c] {
            assert!(r.argv.is_empty(), "{server}: curl ran");
            assert!(!r.stdout.contains(KEY) && !r.stderr.contains(KEY));
        }
    }
    assert!(!dir.join("answers.json").exists());
}

/// SERVER is a URL, never a curl glob: `{a,b}` is one address, asked once.
#[test]
fn server_is_not_a_glob() {
    let dir = scratch("glob");
    let server = serve(&[], vec![ok(completion("m", "stop", "![text](1,1,2,2) a"))]);
    let r = ask(
        &dir,
        &format!("{}/{{a,b}}", server.url()),
        "1",
        Some("m"),
        CHECK_PAGE,
    );
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(server.posts(), 1);
}

/// No file holds the page, even when the run is killed mid-request: the body reaches curl on a
/// pipe.
#[test]
fn a_killed_run_leaves_no_page_on_disk() {
    let dir = scratch("killed");
    let server = serve(&[], vec![Answer::Hang]);
    let log = dir.join("curl-argv.txt");
    let mut child = Command::new(BIN)
        .args(["ask", &server.url(), "answers.json", "1", "m"])
        .env_clear()
        .env("PATH", &tools().curl)
        .env("TMPDIR", &dir)
        .env("ETHOS_TEST_CURL_ARGV", &log)
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(CHECK_PAGE).unwrap();
    let start = std::time::Instant::now();
    while server.posts() == 0 && start.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(server.posts(), 1, "the page was sent");
    child.kill().unwrap();
    child.wait().unwrap();
    let mut left: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    left.sort();
    assert_eq!(left, ["curl-argv.txt", "curl-argv.txt.env"]);
}

/// FILE is replaced through a temporary name no file holds yet: a link planted there is not
/// followed, and two runs never share one.
#[test]
fn the_replies_file_is_never_written_through_a_link() {
    let dir = scratch("part-link");
    let victim = dir.join("victim.txt");
    std::fs::write(&victim, "precious\n").unwrap();
    for name in ["answers.json.part", "answers.json.0.part"] {
        std::os::unix::fs::symlink(&victim, dir.join(name)).unwrap();
    }
    let server = serve(&[], vec![ok(completion("m", "stop", "![text](1,1,2,2) a"))]);
    let r = ask(&dir, &server.url(), "1", Some("m"), CHECK_PAGE);
    assert_eq!(r.code, 0, "{}", r.stderr);
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "precious\n");
    let file = std::fs::symlink_metadata(dir.join("answers.json")).unwrap();
    assert!(file.is_file());
    assert_eq!(
        str_of(page(&answers(&dir), 1), "reply"),
        Some("![text](1,1,2,2) a")
    );
    assert!(
        !dir.join("answers.json.1.part").exists(),
        "renamed into place"
    );
}

/// What `ask` will not do, refused by name before any page is asked.
#[test]
fn refusals_are_named() {
    let dir = scratch("refusals");
    let pdf = three_pages();
    let server = serve(&["a", "b"], vec![]);
    let url = server.url();
    for (pages, model, stdin, name) in [
        ("4", Some("m"), &pdf[..], "page-not-in-document"),
        ("0", Some("m"), &pdf[..], "pages-unreadable"),
        ("1,x", Some("m"), &pdf[..], "pages-unreadable"),
        ("1", Some("m"), &b"not a PDF"[..], "not-a-pdf"),
        (
            "1",
            None,
            &pdf[..],
            "model-not-chosen: the server lists 2: a, b",
        ),
    ] {
        let r = ask(&dir, &url, pages, model, stdin);
        assert_eq!(r.code, 5, "{name}: {}", r.stderr);
        assert!(r.stderr.contains(name), "{name}: {}", r.stderr);
    }
    let r = ask(&dir, "ftp://127.0.0.1/", "1", Some("m"), &pdf);
    assert_eq!(r.code, 5);
    assert!(r.stderr.contains("server-not-http"));
    assert_eq!(server.posts(), 0);
    assert!(!dir.join("answers.json").exists());

    // A file this run did not write.
    std::fs::write(dir.join("answers.json"), "{\"pages\": [").unwrap();
    let r = ask(&dir, &url, "1", Some("m"), &pdf);
    assert!(r.code == 5 && r.stderr.contains("assist-answers-unreadable"));
    let other = serve(&[], vec![ok(completion("m", "stop", "![text](1,1,2,2) a"))]);
    std::fs::remove_file(dir.join("answers.json")).unwrap();
    assert_eq!(ask(&dir, &other.url(), "1", Some("m"), CHECK_PAGE).code, 0);
    let r = ask(&dir, &url, "1", Some("m"), &pdf);
    assert!(
        r.code == 5 && r.stderr.contains("assist-answers-other-source"),
        "{}",
        r.stderr
    );
    assert_eq!(server.posts(), 0);
}

// ---- check ----------------------------------------------------------------------------------

#[test]
fn check_is_ready_for_a_grounding_reply() {
    let dir = scratch("check-ready");
    let server = serve(
        &["LightOnOCR-3-0.8B"],
        vec![ok(completion("LightOnOCR-3-0.8B", "stop", REPLY))],
    );
    let r = run(&dir, None, &["check", &server.url()], b"");
    assert_eq!(r.code, 0, "{}{}", r.stdout, r.stderr);
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert!(lines[0].starts_with("server    answers: GET /v1/models 200 in "));
    assert_eq!(
        lines[1],
        "model     LightOnOCR-3-0.8B   <- every output records this id"
    );
    assert_eq!(
        lines[2],
        "version   not reported by the server; record one with --assist-model ID@VERSION"
    );
    let (_, size) = rendered(&dir, CHECK_PAGE, 1);
    let [dpi, w, h] = <[&str; 3]>::try_from(size.split(' ').collect::<Vec<_>>()).unwrap();
    assert_eq!(
        lines[3],
        format!("page      1 test page, 612 x 792 pt -> {w} x {h} px at {dpi} dpi (hayro 0.7.1)")
    );
    assert!(lines[4].starts_with("reply     stop, 412 tokens, "));
    assert!(lines[4].ends_with(" s; 3 blocks: title, text, table"));
    assert_eq!(
        lines[5],
        "reads     9 of 9 words, 9 of 9 numbers; table 3 x 3 as drawn"
    );
    assert_eq!(lines[6], format!("ready     replies parse as {REQUEST}"));
    assert_eq!(lines.len(), 7);
    assert!(!r.stdout.contains("127.0.0.1"));
    let requests = server.requests.lock().unwrap();
    let body = String::from_utf8_lossy(&requests[1].body);
    assert!(body.starts_with("{\"model\":\"LightOnOCR-3-0.8B\",\"max_tokens\":12288,"));
}

#[test]
fn check_names_a_reply_it_cannot_read() {
    let dir = scratch("check-prose");
    let prose = "The image shows a report titled Harbor Report with a small table.";
    let server = serve(
        &["chat-model"],
        vec![ok(completion("chat-model", "stop", prose))],
    );
    let r = run(&dir, None, &["check", &server.url()], b"");
    assert_eq!(r.code, 1);
    assert!(r.stdout.contains("reply     stop, 412 tokens, "));
    assert!(r
        .stdout
        .lines()
        .last()
        .unwrap()
        .starts_with("not ready reply-format-not-read: "));

    let server = serve(&["m"], vec![Answer::Status(500, "{}".into())]);
    let r = run(&dir, None, &["check", &server.url()], b"");
    assert_eq!(r.code, 1);
    assert_eq!(r.stdout.lines().last(), Some("reply     http 500"));
}

#[test]
fn check_cannot_reach_a_server_that_is_not_there() {
    let dir = scratch("check-down");
    let r = run(
        &dir,
        None,
        &["check", &format!("http://127.0.0.1:{}", dead_port())],
        b"",
    );
    assert_eq!(r.code, 2);
    assert!(
        r.stdout.starts_with("server    server-unreachable"),
        "{}",
        r.stdout
    );
    let r = run_with(
        &dir,
        &tools().none,
        &[],
        &["check", "http://127.0.0.1:1"],
        b"",
    );
    assert_eq!(r.code, 2);
    assert!(r.stdout.contains("curl-not-found"), "{}", r.stdout);
}

/// MLX reports `--model` verbatim, so a path would be recorded in every output. The warning
/// follows the rule the engine refuses replies by (`assist-answers-model-is-path`); an id with
/// an organisation, as the Hub names models, is not a path.
#[test]
fn check_warns_when_the_id_is_a_path() {
    for (label, id, warned) in [
        ("check-path-abs", "/Users/someone/lighton-mlx", true),
        ("check-path-home", "~/lighton-mlx", true),
        ("check-path-dot", "./lighton-mlx", true),
        ("check-path-drive", "C:lighton-mlx", true),
        ("check-path-backslash", "models\\lighton-mlx", true),
        ("check-path-hub", "lightonai/LightOnOCR-3-0.8B", false),
    ] {
        let dir = scratch(label);
        let server = serve(&[id], vec![ok(completion(id, "stop", REPLY))]);
        let r = run(&dir, None, &["check", &server.url(), id], b"");
        assert_eq!(r.code, 0, "{id}: {}", r.stdout);
        assert_eq!(
            r.stdout.contains("\nwarning   the id is a path"),
            warned,
            "{id}: {}",
            r.stdout
        );
    }
}

// ---- extract --assist, through the engine ----------------------------------------------------

/// The engine, `ethos-parser`: `ETHOS_PARSER_BIN`, or the repository's debug build, which the gate
/// and CI build before this suite runs.
fn engine_bin() -> PathBuf {
    let bin = std::env::var_os("ETHOS_PARSER_BIN").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/debug/ethos-parser"),
        PathBuf::from,
    );
    assert!(
        bin.is_file(),
        "{} is not built: run `cargo build -p ethos-parser-cli` at the repository root, or name \
         the engine by ETHOS_PARSER_BIN",
        bin.display()
    );
    bin
}

/// `ethos-parser ARGS` in `dir`, with this build as its assist binary, the logging curl alone on
/// `PATH`, and nothing else in its environment but `key`.
fn engine(dir: &Path, key: Option<&str>, args: &[&str]) -> Run {
    let log = dir.join("curl-argv.txt");
    let _ = std::fs::remove_file(&log);
    let out = Command::new(engine_bin())
        .args(args)
        .env_clear()
        .env("PATH", &tools().curl)
        .env("TMPDIR", dir)
        .env("ETHOS_TEST_CURL_ARGV", &log)
        .env("ETHOS_PARSER_ASSIST", BIN)
        .envs(key.map(|k| ("ETHOS_PARSER_ASSIST_KEY", k)))
        .current_dir(dir)
        .output()
        .unwrap();
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        argv: std::fs::read_to_string(&log).unwrap_or_default(),
        env: String::new(),
    }
}

/// **`extract --assist` end to end** (decision #149): the engine spawns this binary, which asks
/// the fake server for each page the trigger chose (all three: each holds two words) with the key
/// in a header, and the record is the bytes `--assist-answers FILE` gives with no server at all;
/// a second run resumes, asking nothing. A server that is not there is the engine's refusal, by
/// name, with nothing on stdout.
#[test]
fn extract_assist_asks_the_server_and_records_what_answers_file_gives() {
    let dir = scratch("extract");
    std::fs::write(dir.join("doc.pdf"), three_pages()).unwrap();
    let replies = (0..3)
        .map(|_| ok(completion("LightOnOCR-3-0.8B", "stop", REPLY)))
        .collect();
    let server = serve(&["LightOnOCR-3-0.8B"], replies);
    let url = server.url();
    let args = [
        "extract",
        "doc.pdf",
        "--assist",
        url.as_str(),
        "--assist-answers",
        "doc.answers.json",
    ];
    let asked = engine(&dir, Some(KEY), &args);
    assert_eq!(asked.code, 0, "{}", asked.stderr);
    assert_eq!(server.posts(), 3, "one request a page");
    for req in server.requests.lock().unwrap().iter() {
        let bearer = format!("Bearer {KEY}");
        assert_eq!(req.header("authorization"), Some(bearer.as_str()));
    }
    let file = std::fs::read_to_string(dir.join("doc.answers.json")).unwrap();
    for out in [&asked.stdout, &asked.stderr, &asked.argv, &file] {
        assert!(!out.contains(KEY), "the key is never written");
    }
    assert!(!asked.stdout.contains("127.0.0.1") && !file.contains("127.0.0.1"));
    let record = Json::parse(asked.stdout.as_bytes()).expect("a record");
    let pages = record
        .get("representation")
        .and_then(|r| r.get("assist"))
        .and_then(|a| a.get("pages"))
        .and_then(Json::arr)
        .expect("the replies are attached");
    assert_eq!(pages.len(), 3);

    let merged = engine(
        &dir,
        None,
        &["extract", "doc.pdf", "--assist-answers", "doc.answers.json"],
    );
    assert_eq!(merged.code, 0, "{}", merged.stderr);
    assert_eq!(
        merged.stdout, asked.stdout,
        "a function of the PDF and FILE"
    );
    let resumed = engine(&dir, Some(KEY), &args);
    assert_eq!(resumed.code, 0, "{}", resumed.stderr);
    assert_eq!(server.posts(), 3, "every page was answered before");
    assert_eq!(resumed.stdout, asked.stdout);

    let gone = format!("http://127.0.0.1:{}", dead_port());
    let mut unreachable = args;
    unreachable[3] = &gone;
    unreachable[5] = "other.answers.json";
    let r = engine(&dir, None, &unreachable);
    assert_ne!(r.code, 0);
    assert!(r.stdout.is_empty());
    assert!(
        r.stderr.contains("assist-server-unreachable"),
        "{}",
        r.stderr
    );
}
