# ethos-parser

A document parser that shows its work.

Most parsers hand you text and ask you to trust it. This one hands you text **plus a pointer back
to the exact spot in the file it came from**, and a written list of everything it could not read.

It answers one question: *what is in this document, and where exactly?* It does not answer whether
a claim is true — that is a verifier's job. Put the two together and you can answer the question
that actually matters: **did this AI answer really come from this document?**

## What that looks like

Run `extract` on a PDF and every piece of text carries a pointer back to where it was drawn:

```json
{
  "id": "s1",
  "kind": "text_run",
  "text": "Measured",
  "native_locator": {
    "pdf": { "page": 1, "origin_x": 7200, "origin_y": 7200, "advance": 9600 }
  }
}
```

Coordinates are whole numbers in centipoints (1/100 of a point), so `7200` means 72 pt. There are
no floating-point numbers anywhere in the output, which is why two runs over the same file always
produce byte-identical results.

A Word file gets the same treatment, addressed the way Word addresses itself:

```json
{
  "text": "Evidence, not extraction.",
  "native_locator": { "docx": { "part": "word/document.xml", "paragraph": 1, "run": 1 } }
}
```

A spreadsheet, by sheet, row and column — with the column kept as the letter the file wrote:

```json
{
  "text": "Rows & columns are S3.",
  "native_locator": {
    "xlsx": { "part": "xl/worksheets/sheet1.xml", "sheet": "Ledger", "row": 1, "column": "A" }
  }
}
```

Notice what is missing from those last two: a page number. A `.docx` has no pages — the page breaks
you see on screen belong to whatever printed it. So `pages` comes back empty rather than filled
with a guess.

## Benchmarks

Scored on two public benchmarks with each benchmark's own evaluator. ethos-parser runs locally,
with no OCR and no AI model. Higher is better.

**[ParseBench](https://github.com/run-llama/ParseBench)** — about 2,000 human-verified pages from
enterprise documents, scored on five dimensions. Against the open-source local parsers on its
leaderboard, ethos-parser places **2nd of 10**; AI-model services score higher.

| Parser | Overall | Tables | Charts | Content | Formatting | Grounding |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| LlamaParse Agentic Plus (highest overall, cloud) | 86.61 | 93.37 | 90.19 | 88.74 | 86.54 | 74.21 |
| PyMuPDF4LLM | 53.49 | 72.00 | 2.19 | 79.34 | 52.04 | 61.89 |
| **ethos-parser 0.64.0** | **50.67** | 58.75 | 17.26 | 67.46 | 51.27 | 58.59 |
| Warp Ingest | 40.18 | 57.45 | 7.04 | 70.81 | 45.81 | 19.78 |
| LiteParse (no OCR) | 36.90 | 42.40 | 1.80 | 70.00 | 38.50 | 31.80 |
| OpenDataLoader | 29.40 | 35.18 | 0.92 | 66.06 | 34.07 | 10.77 |
| pdf-inspector | 26.59 | 26.64 | 5.29 | 56.06 | 35.07 | 9.91 |
| MarkItDown | 18.63 | 15.77 | 2.02 | 64.54 | 0.91 | 9.90 |
| PyMuPDF (HTML) | 16.62 | 0.00 | 0.00 | 55.63 | 18.26 | 9.20 |
| PyMuPDF (Text) | 16.02 | 0.00 | 0.00 | 68.28 | 0.95 | 10.86 |
| pypdf | 14.87 | 0.00 | 0.00 | 62.50 | 0.91 | 10.92 |

**[opendataloader-bench](https://github.com/opendataloader-project/opendataloader-bench)** — 200
real-world PDFs, scored on reading order (NID), tables (TEDS) and headings (MHS). ethos-parser
places **9th of 13**.

| Engine | Overall | Reading order | Tables | Headings | Speed (s/page) |
| --- | ---: | ---: | ---: | ---: | ---: |
| opendataloader [hybrid] | 0.907 | 0.934 | 0.928 | 0.821 | 0.463 |
| nutrient | 0.885 | 0.925 | 0.708 | 0.819 | 0.008 |
| docling | 0.882 | 0.898 | 0.887 | 0.824 | 0.762 |
| marker | 0.861 | 0.890 | 0.808 | 0.796 | 53.932 |
| unstructured [hi_res] | 0.841 | 0.904 | 0.588 | 0.749 | 3.008 |
| edgeparse | 0.837 | 0.894 | 0.717 | 0.706 | 0.036 |
| opendataloader | 0.831 | 0.902 | 0.489 | 0.739 | 0.015 |
| mineru | 0.831 | 0.857 | 0.873 | 0.743 | 5.962 |
| **ethos-parser 0.64.0** | **0.791** | 0.888 | 0.583 | 0.589 | 0.036 |
| pymupdf4llm | 0.732 | 0.885 | 0.401 | 0.412 | 0.091 |
| unstructured | 0.686 | 0.882 | 0.000 | 0.388 | 0.077 |
| markitdown | 0.589 | 0.844 | 0.273 | 0.000 | 0.114 |
| liteparse | 0.576 | 0.866 | 0.000 | 0.000 | 1.061 |

The ethos-parser rows are measured by this project with each benchmark's own scorer: ParseBench at
`1f5b37a` through [`docs/measurements/parsebench/`](docs/measurements/parsebench/),
opendataloader-bench at `7af1d8f` through
[`docs/measurements/opendataloader-bench/`](docs/measurements/opendataloader-bench/). Every other
row is copied from the benchmark's published results: ParseBench's `leaderboard.csv` at `1f5b37a`,
opendataloader-bench's `evaluation.json` files at `7af1d8f`. Other engines' speeds are from an
Apple M4, ethos-parser's from an Apple M4 Pro. On charts, ethos-parser reads only the values a
chart prints; it never estimates a value from the drawing.

## Install

Every release carries a prebuilt binary for Linux, Windows and macOS (Apple silicon and Intel),
plus a `SHA256SUMS.txt`. A release is assembled only when all four binaries print byte-identical
artifacts over the gate documents. On macOS or Linux, set `T` to your platform and run:

```bash
T=aarch64-apple-darwin   # or x86_64-apple-darwin, x86_64-unknown-linux-gnu
curl -fLO https://github.com/docushell/ethos-parser/releases/latest/download/SHA256SUMS.txt
A=$(grep -o "ethos-parser-[^ ]*-$T\.tar\.gz" SHA256SUMS.txt)
curl -fLO "https://github.com/docushell/ethos-parser/releases/latest/download/$A"
shasum -a 256 -c SHA256SUMS.txt --ignore-missing   # Linux: sha256sum -c SHA256SUMS.txt --ignore-missing
tar -xzf "$A"
"./${A%.tar.gz}/ethos-parser" --version
```

`OK` beside your archive is the digest check passing; the one line it calls improperly formatted
is the blank line in the file's header. On Windows, take the `x86_64-pc-windows-msvc` archive from
the [releases page](https://github.com/docushell/ethos-parser/releases), compare `Get-FileHash`
with `SHA256SUMS.txt`, extract it with `tar -xzf`, and run `ethos-parser.exe`. The macOS binaries
are not notarized: `curl` leaves them runnable, but a copy downloaded through a browser is
quarantined and killed on launch — once its digest checks out,
`xattr -d com.apple.quarantine ethos-parser-*/ethos-parser` clears it.

Or build from source, with the toolchain `rust-toolchain.toml` pins:

```bash
cargo build --release --locked
```

Then, with the binary on your `PATH`:

```bash
ethos-parser extract contract.pdf > repr.json
ethos-parser ground repr.json > grounding.json
```

## Commands

| Command | What it does | Exit codes |
| --- | --- | --- |
| `classify` | Counts and reason codes for a PDF — is it scanned? is the layout hard? | 0 / 1 / 2 |
| `extract` | The document as text runs with locators (`DocumentRepresentation v0`) | 0 / 2 |
| `ground` | Turn a representation into `ethos.grounding.v1`, the citation format | 0 / 2 |
| `markdown` | Markdown plus a map from every character back to a node. Takes a representation, or `--source <FILE>` to read a document and project it in one process — the same bytes, without the record on disk between the two stages | 0 / 2 |
| `html` | The same thing in HTML, projected from the document rather than from the Markdown | 0 / 2 |
| `grounding-check` | Validate a grounding file's structure and its link to the source bytes | 0 / 1 / 2 |
| `verify` | Hand a citation check to the pinned Ethos verifier and relay its answer | 0 / 1 / 2 |
| `overlay` | An annotated copy of the PDF showing what was found — and what has no box | 0 / 2 |
| `tag` | A copy of an untagged PDF with a structure tree written over it from the block cut — one `/Div` per block, marked as computed, so `extract` reads it back as this engine's own. Fills absence only: a PDF that already has a tree is refused ([`docs/23-AUTO-TAGGING-SCOPE.md`](docs/23-AUTO-TAGGING-SCOPE.md)) | 0 / 2 |
| `locate` | Where a string lies in a representation — every occurrence, as node ids, character offsets and the record's own boxes. Never whether anything is true: a string that occurs nowhere is an empty answer, and there is no exit 1 ([`docs/26-LOCATE-SCOPE.md`](docs/26-LOCATE-SCOPE.md)) | 0 / 2 |
| `mcp` | Serve `extract`, `ground`, `node_get`, `locate` and `markdown` to an agent over stdio | 0 / 2 |

Exit codes always mean the same thing: **0** it worked, **1** it ran and the answer is no, **2** it
could not run at all. Add `--diagnostics` to any command for timing and host details on stderr;
stdout never changes.

## Formats it reads

PDF, DOCX, XLSX, PPTX, ODT, ODS, ODP, RTF and EPUB — all into the same record, with the same
fingerprint and the same JSON shape.

The format is decided by looking at the bytes, never the file extension. A `report.bin` that is
really a PDF still reads; a `.docx` full of something else is a clear error. Anything the engine
has no reader for is refused by name:

```
$ ethos-parser extract data.csv
engine: unsupported media type: these bytes state no format this engine reads. Three signatures
were looked for at byte 0 and none is present: a ZIP local file header ... an RTF brace group ...
and a PDF header. Nothing else was consulted — not the file's name ...
```

CSV is deliberately absent. A CSV parse would produce real text, but the engine would have to
*claim* the file is a CSV without ever having measured that, and there is nowhere in the record to
say "I was told this, I did not check it." See [`docs/CAPABILITY.md`](docs/CAPABILITY.md) for what
would reopen it.

## Markdown you can still cite

The `markdown` command never gives you a bare Markdown string. It gives you the string **and** an
anchor map that says where every character came from:

```json
{
  "markdown": "Measured\n",
  "anchor_map": {
    "segments": [
      { "start": 0, "end": 8, "kind": "source", "node_ids": ["s1"] },
      { "start": 8, "end": 9, "kind": "syntax" }
    ]
  }
}
```

Characters 0–8 are the document's own text and map back to node `s1`. Character 8 is the newline
this exporter added. Every byte is one or the other, which gives you a mechanical rule: **a quote
that touches a `syntax` byte is not a citation.** A coverage census counts anything that did not
make it into the Markdown, by reason.

`html` works the same way. It exists as its own command rather than a stylesheet because of tables:
Markdown has no `rowspan`, so it has to flatten a merged cell and count the cost, while HTML can
emit `<td colspan="2">` and keep the merge the document drew.

## Using it from code

**MCP** (`ethos-parser mcp`) serves five tools over a plain pipe — `extract`, `ground`,
`node_get`, `locate` and `markdown`. No HTTP, no socket, no async runtime. To add it to Claude
Code:

```bash
claude mcp add ethos-parser -- /absolute/path/to/ethos-parser mcp
```

Any other MCP host takes the same command, with `mcp` as its one argument.

`markdown` is how a model reads a document: give it a file's `path`, and it returns the text the
`markdown` command prints, after a count of what that text leaves out. It is the one place the
engine hands over Markdown without its anchor map, on purpose (decision #36 in
[`docs/00-NORTH-STAR.md`](docs/00-NORTH-STAR.md)): the text is for reading, and a passage the
model wants to cite goes back through `locate`, which returns the locators the engine minted.

The catch with MCP is that the model picks the arguments. A tool that accepted a page number or a
bounding box would make the model the citation authority in one step, and the result would look
exactly like a real citation. So no tool argument anywhere names a coordinate. The engine mints
every locator itself, hands it back as an opaque id, and re-checks it on the way in: `node_get`
verifies the artifact's fingerprint — for a path, once per distinct byte string while it is among the
last 64 this process verified; for an inline artifact, every call — then looks up the id among the
nodes parsed from *those* bytes. An
id the engine did not mint is an error, never a nearest match.

**Python and Node SDKs** ([`packages/python/`](packages/python/),
[`packages/node/`](packages/node/)) expose `extract`, `ground`, `node_get` and `locate` as
functions. They spawn the CLI and parse its
stdout — no native bindings, no runtime dependencies, so they cannot disagree with the binary about
what a document says. LangChain tools ship with both, behind an optional install. Neither package is
published yet.

## What it will never do

- Emit a confidence score, a grade, or any single field meaning "this document is good."
- Delete hidden text, headers, footers or anything else without saying so.
- Invent a coordinate, an id, a fingerprint, or a page number.
- Verify a citation. It validates structure and relays a verifier's answer; it never forms one.

Everything the engine could not do is stated — as a named limitation, a typed absence, or a counted
bucket. A gap is never dressed up as a success.

## Where things stand

Version 0.64.0. PDF and eight office formats read; Markdown, HTML, MCP and both SDKs ship.

**Tables** come from eight named, versioned rules: the tables a document declares in its structure
tree, ruled grids, columns of aligned text, and the numbers a chart prints. None invents a cell:
fabrication is **0** on the twelve-document labelled set, measured on every run
([`docs/table-gate-v1.md`](docs/table-gate-v1.md)).

For the full honest inventory, read [`docs/CAPABILITY.md`](docs/CAPABILITY.md).

## Tests

```bash
cargo test --workspace --locked
```

Nothing is excluded and nothing is skipped — a test enforces that. The oracle test compares this
engine's answers against the real Ethos CLI, and filtering it out to get a green build would delete
the only proof that the two read a document the same way. It needs the Ethos repo for its fixtures
and binary, resolved from `../ethos` or from `ETHOS_FIXTURES`, `ETHOS_BENCH_CORPUS` and `ETHOS_BIN`.
A missing corpus is a failure, never a skip.

The SDK suites need a built binary rather than a build of their own:

```bash
pip install -e 'packages/python[dev]' && pytest packages/python
```

```bash
node --test packages/node
```

Both read `ETHOS_PARSER` first, then `PATH`, then the workspace `target/` — and both refuse a
binary whose version does not match `Cargo.toml`, so a stale build cannot answer questions
plausibly and go green about the wrong engine.

Run the whole gate exactly as CI runs it with `ci/gate.sh`.

## Contributing

Every commit needs a `Signed-off-by` trailer. Point git at the tracked hooks once and they will add
and check it for you:

```bash
git config core.hooksPath .githooks
```

Git only parses trailers in the last paragraph, so keep `Signed-off-by` and any `Co-Authored-By`
together with no blank line between them.

Start with [`docs/README.md`](docs/README.md) for the map of the documentation.

## Licence

Apache-2.0. `NOTICE` records the encoding data in the tree, Adobe's CID-to-Unicode mappings among
it (BSD-3-Clause, decision #78), and reserves a slot for the Adobe predefined CMaps if they ever
land — none are present today. No AGPL dependencies —
`deny.toml` enforces it and CI proves the rule fires by flipping a crate to AGPL and requiring the
build to reject it.
