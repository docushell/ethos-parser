# ethos-parser-assist

The assist lane's own process (decisions #143 and #144). It renders a PDF's pages, sends each to a
model server you run, and writes the replies to a file. The engine reads that file; it does not
call this binary yet.

**Install.** From 0.66.0 every release archive carries this binary beside `ethos-parser`, built
for Linux, Windows and both macOS architectures, and no model: the model runs in a server you
start (below). On Windows `ask` and `check` are refused `curl-not-run`, because the page reaches
`curl` through `/dev/fd` (see the transport); `render` and `identity` work. To build it yourself,
see the last section.

```text
ethos-parser-assist identity                             one JSON line: {"assist","version","renderer","request"}
ethos-parser-assist ask SERVER FILE PAGES [MODEL] <PDF   asks for pages PAGES (`1,3,7`); writes the replies to FILE
ethos-parser-assist check SERVER [MODEL]                 is SERVER ready for the lane?
ethos-parser-assist render PDF PAGE OUT.png              page PAGE (from 1) as a PNG; prints "dpi width height"
```

`render` exits 1 on a refusal (unreadable file, no such page, a page with no pixel size) and 2 on
bad usage.

## `ask`

- **The request.** Each page is rendered by the page rule below and sent to
  `SERVER/v1/chat/completions` as decision #123 asked it: one PNG data URL, the prompt
  `grounding`, temperature 0, `max_tokens` 12,288, not streamed.
- **The model.** MODEL is `ID[@VERSION]`, and ID is sent as the request's `model`. mlx-vlm loads
  whatever model the request names, so name the served one exactly. Without MODEL, the one model
  `GET /v1/models` lists is used. If it lists none or several, `ask` is refused `model-not-chosen`,
  with the list; mlx-vlm lists the models in the Hugging Face cache too.
- **FILE, the replies file.** Canonical JSON: keys sorted, no whitespace. It holds:
  - `ethos_assist_answers` 1, `source_sha256` (the PDF's bytes), `renderer`, `request`, and
    `model`: `id` as the server reports it, `version`, VERSION or `not-reported`, and `asked`, the
    ID the request named (the engine reads it and does not record it);
  - per page asked, in page order: `page`, `points` (the rendered box in centipoints), `dpi`,
    `width`, `height` and `pixels_sha256`, then `reply` with `finish_reason`, or `error`.
  - Digests are `sha256:<hex>`. `pixels_sha256` is the digest of the PNG sent: the bytes `render`
    writes for that page.
  - The server's address, the key and timings are never written, so the same replies give the
    same bytes.
- **Resume.** FILE is replaced after each page, through a new `FILE.N.part` renamed over it. Run
  `ask` again with the same
  arguments: a page with a reply is kept, and a page with an `error` is asked again. A file for
  another PDF is refused, as are a model the server reports differently and another VERSION
  (`assist-answers-other-source`, `assist-answers-other-model`). Once FILE holds a reply, MODEL's
  ID must be its `asked` or its `id`, so a server's alias resumes under either name; any other is
  refused before a page is passed over as answered. A file without `asked` was asked by its `id`.
- **Exit codes.**
  - 0: every page asked has a reply or a named error.
  - 3: done, but some pages could not be rendered (`page-not-rendered` on stderr, one line a
    page); they are not in FILE, and the rest are.
  - 4: the server cannot be reached (`server-unreachable`); FILE keeps the pages done.
  - 5: refused, by name.
  - 1: FILE cannot be written.
  - 2: bad usage.
- **Names.** A page's `error` is `timeout`, `http N`, `reply-unreadable`, `reply-too-large` (over
  64 MiB), `curl-exit-N`, `curl-killed` or `body-not-written`. A reply cut at `max_tokens` is kept
  with `finish_reason: "length"`, and stderr names it `reply-capped`. The refusals are
  `curl-not-found`, `curl-not-run`, `server-not-http`, `key-not-a-header-value`,
  `key-needs-https`, `not-a-pdf`,
  `pages-unreadable`, `page-not-in-document`, `model-not-chosen` and the
  `assist-answers-*` names the engine uses. Progress, one line a page, is on stderr.

## `check`

`check` asks for a committed test page, `data/check-page.pdf`, and prints a line a step. The page
is made-up content: a heading, a sentence and a ruled 3 x 3 table of numbers. For example (times
and token counts vary):
```text
server    answers: GET /v1/models 200 in 12 ms
model     LightOnOCR-3-0.8B   <- every output records this id
version   36d8636925327c671bd8e479c517a004f7566bd9   <- named with the model
page      1 test page, 612 x 792 pt -> 1964 x 2541 px at 231 dpi (hayro 0.7.1)
reply     stop, 412 tokens, 7.9 s; 3 blocks: title, text, table
reads     9 of 9 words, 9 of 9 numbers; table 3 x 3 as drawn
ready     replies parse as lightonocr-3-grounding-v1
```
- **Exit codes.** 0 when ready; 1 when the reply holds no block marker (`reply-format-not-read`:
  a general chat model) or the server answered with an error; 2 when the server cannot be
  reached, `curl` is missing or cannot run, or the key would go unencrypted (`key-needs-https`).
- **A path as the id.** mlx-vlm reports `--model` verbatim, and every output records the id, so
  `check` warns when the id starts with `/` or `~`.

## The transport and the key

Requests go through the system `curl`, run as `curl -q -K -`. The binary links no HTTP crate.
That is the one exception to the engine's no-network rule, scoped to this binary in
[`docs/37-ASSIST-TRANSPORT.md`](../docs/37-ASSIST-TRANSPORT.md).
- **The key.** A cloud API's key is read from `ETHOS_PARSER_ASSIST_KEY` and nowhere else. It reaches
  curl in its config, on its stdin. So it is never in argv, in curl's environment, on disk, in FILE
  or in any output. A local server needs no key. It is sent only to an `https://` server or to
  loopback: with a key set, an `http://` server elsewhere is refused `key-needs-https` before curl
  runs. Without a key, `http://` to another machine is allowed.
- **The page.** It reaches curl on a second pipe, as `/dev/fd/2`, so no file ever holds it, even
  when a run is killed. That makes `ask` and `check` Unix-only for now (macOS, Linux): elsewhere
  they are refused `curl-not-run`.
- **Proxies.** A loopback server — `localhost`, any address of 127.0.0.0/8 written in dotted
  decimal, or `[::1]` — is never reached through a proxy, whatever `http_proxy`, `HTTPS_PROXY` or
  `ALL_PROXY` say. Any other server is reached as your proxy settings and `NO_PROXY` say.
- **Limits.** A reply over 64 MiB is refused `reply-too-large`. curl follows no redirect, and
  SERVER is never read as a curl glob.

## Setup: a model server

### Mac (MLX)

These are Phase 4a's commands (2026-11-14), which were run on an M4 Pro, with one change:
- **The change.** Phase 4a converted the model to an absolute path (`--mlx-path ~/lighton-mlx`)
  and served that path. mlx-vlm then reports the path as the id, and every output would record it.
- **Here,** the model is converted under `~` and served by a relative name, so the id holds no
  path. mlx-vlm 0.7.12's source returns the request's `model` as the reply's. This form has not
  been run.
- **Two more notes.** `<snapshot dir>` is the directory `hf download` prints. `--host 127.0.0.1`
  matters, since mlx-vlm listens on every interface by default.
```bash
uv venv --python 3.12 ~/lighton && uv pip install --python ~/lighton mlx-vlm==0.7.12 mlx==0.32.4 transformers==5.22.2 pillow==13.0.0 fastapi uvicorn
~/lighton/bin/hf download lightonai/LightOnOCR-3-0.8B --revision 36d8636925327c671bd8e479c517a004f7566bd9
cd ~ && ~/lighton/bin/python -m mlx_vlm convert --hf-path <snapshot dir> --mlx-path LightOnOCR-3-0.8B
cd ~ && HF_HUB_OFFLINE=1 ~/lighton/bin/python -m mlx_vlm.server --host 127.0.0.1 --port 8089 --model LightOnOCR-3-0.8B --max-tokens 12288
ethos-parser-assist check http://127.0.0.1:8089 LightOnOCR-3-0.8B@36d8636925327c671bd8e479c517a004f7566bd9
ethos-parser-assist ask http://127.0.0.1:8089 doc.answers.json 1,2,3 LightOnOCR-3-0.8B@36d8636925327c671bd8e479c517a004f7566bd9 < doc.pdf
```
A later step gives `ethos-parser extract` the `--assist` flag, which runs `ask` for you.

### Linux (vLLM, NVIDIA GPU): not run here

These commands are from ParseBench's LightOnOCR-3 provider. **They have not been run**: this
project has no Linux GPU. vLLM 0.30.0 or later is needed, since earlier versions loop.
```bash
docker run --gpus all -p 127.0.0.1:8089:8000 vllm/vllm-openai:v0.30.0 --model lightonai/LightOnOCR-3-0.8B \
  --revision 36d8636925327c671bd8e479c517a004f7566bd9 --served-model-name lightonai/LightOnOCR-3-0.8B \
  --gpu-memory-utilization 0.85 --limit-mm-per-prompt '{"image": 1}' --max-model-len 24576
ethos-parser-assist check http://127.0.0.1:8089 lightonai/LightOnOCR-3-0.8B@36d8636925327c671bd8e479c517a004f7566bd9
```
A cloud API that serves the model works the same way, with `ETHOS_PARSER_ASSIST_KEY` set.

## Why a workspace of its own

- hayro 0.7.1 needs Rust 1.92, and the engine is pinned to 1.88. This workspace uses the
  recognizer's toolchain, 1.98.0 (`rust-toolchain.toml`).
- In `ocr/`, hayro's crates would share `image`, `png` and `zune-jpeg` with the recognizer, so a
  change here could move `--ocr` output. So `assist/` has its own `Cargo.lock` and `deny.toml`, and
  the root and `ocr/` lockfiles do not change.

## The page rule, `pages-400dpi-5mp-v1`

This is decision #123's Ghostscript rule, rendered with hayro:

- dpi = min(400, floor(sqrt(5e6 / inches))), with the inches taken from the page's size in whole
  points.
- Pixel sizes are rounded as Ghostscript rounds them. hayro floors, which leaves 1 px short.
- A page still over 5e6 pixels is shrunk with Lanczos.
- hayro renders the CropBox, intersected with the MediaBox and rotated, on white, written as an
  RGB PNG.

The `renderer` string names this build, hayro's version and the rule. A unit test ties it to
`Cargo.toml` and `Cargo.lock`.

**Measured** (2026-11-15, Apple M4 Pro): page 1 of the 379 ParseBench PDFs sampled in Phase 4a
renders byte for byte the same as Phase 4a's renderer did, with the same `dpi width height` line.
vello_cpu picks SIMD paths at run time, so pixels on other CPUs are unmeasured.

## Build and check

```bash
cd assist
cargo build --release --locked
cargo fmt --check
cargo clippy --release --all-targets --locked -- -D warnings
cargo test --release --locked
```

The tests need `curl` on `PATH` and nothing beyond loopback: `tests/transport.rs` runs `ask` and
`check` against a fake server on 127.0.0.1, with an empty environment.

`ci/assist.sh` runs the last four lines in `ci/gate.sh` and in CI. Both also run
`cargo deny --manifest-path assist/Cargo.toml check` from the repository root. cargo-deny then reads `assist/deny.toml`, which allows Apache-2.0, MIT, BSD-3-Clause, Zlib and
Unicode-3.0, has no GPL or AGPL, and copies the root's network bans.

hayro embeds data that is not a crate: the Foxit standard-14 fonts from PDFium (BSD-3-Clause),
Adobe's CMaps (BSD-3-Clause), a CGATS ICC profile (CC0) and LAB.icc (LCMS2). Their notices are in
`NOTICE`, which every release archive carries, with every crate `Cargo.lock` locks and its licence;
a unit test holds that list to the lockfile, so a dependency change updates `NOTICE` too.
