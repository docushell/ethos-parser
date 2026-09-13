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

## Build it

There is no published binary yet. Build from source:

```bash
cargo build --release --locked
```

Then:

```bash
target/release/ethos-parser extract contract.pdf > repr.json
target/release/ethos-parser ground repr.json > grounding.json
```

## Commands

| Command | What it does | Exit codes |
| --- | --- | --- |
| `classify` | Counts and reason codes for a PDF — is it scanned? is the layout hard? | 0 / 1 / 2 |
| `extract` | The document as text runs with locators (`DocumentRepresentation v0`) | 0 / 2 |
| `ground` | Turn a representation into `ethos.grounding.v1`, the citation format | 0 / 2 |
| `markdown` | Markdown plus a map from every character back to a node | 0 / 2 |
| `html` | The same thing in HTML, projected from the document rather than from the Markdown | 0 / 2 |
| `grounding-check` | Validate a grounding file's structure and its link to the source bytes | 0 / 1 / 2 |
| `verify` | Hand a citation check to the pinned Ethos verifier and relay its answer | 0 / 1 / 2 |
| `overlay` | An annotated copy of the PDF showing what was found — and what has no box | 0 / 2 |
| `mcp` | Serve `extract`, `ground` and `node_get` to an agent over stdio | 0 / 2 |

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

`markdown` never gives you a bare Markdown string. It gives you the string **and** an anchor map
that says where every character came from:

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

**MCP** (`ethos-parser mcp`) serves three tools over a plain pipe — `extract`, `ground` and
`node_get`. No HTTP, no socket, no async runtime.

The catch with MCP is that the model picks the arguments. A tool that accepted a page number or a
bounding box would make the model the citation authority in one step, and the result would look
exactly like a real citation. So no tool argument anywhere names a coordinate. The engine mints
every locator itself, hands it back as an opaque id, and re-checks it on the way in: `node_get`
verifies the artifact's fingerprint — once per distinct byte string per server process — then
looks up the id among the nodes parsed from *those* bytes. An
id the engine did not mint is an error, never a nearest match.

**Python and Node SDKs** ([`packages/python/`](packages/python/),
[`packages/node/`](packages/node/)) are the same three functions. They spawn the CLI and parse its
stdout — no native bindings, no runtime dependencies, so they cannot disagree with the binary about
what a document says. LangChain tools ship with both, behind an optional install. Neither package is
published yet.

## What it will never do

- Emit a confidence score, a grade, or any single field meaning "this document is good."
- Delete hidden text, headers, footers or anything else without saying so.
- Invent a coordinate, an id, a fingerprint, or a page number.
- Verify a citation. It validates structure and relays a verifier's answer; it never forms one.
- Publish a benchmark table comparing itself to other parsers.

Everything the engine could not do is stated — as a named limitation, a typed absence, or a counted
bucket. A gap is never dressed up as a success.

## Where things stand

Version 0.55.0. PDF and eight office formats read; Markdown, HTML, MCP and both SDKs ship.

**Tables, stated as a capability rather than as an average.** The engine does four things, and the
fourth is the one to read first:

- It **reads the tables a document declares** in its structure tree.
- It **detects ruled tables** where the producer actually drew the rules.
- It **emits nothing** where neither holds.
- It **fabricates nothing** — 0 cells, measured on every run, not asserted.

The numbers, on twelve tagged public documents (2,068 pages, 172 tagged tables, 15,755 tagged cell
slots). Reading the tables a document declares recovers **157 of the 172** gold tables and takes
combined cell-slot recall to **502‰**. The geometric detectors alone score a macro cell-slot F1 of
**70‰** — but the band is 0‰–590‰ with a median of 0‰, and **ten of the twelve score exactly zero**,
because those documents never drew a grid to detect. Two documents supply that entire average, which
is why the band is printed here and the macro is not offered on its own.

That shape is the honest finding: the geometric rules work where a producer drew the grid and produce
nothing where it did not. The method, the per-document table and the caveats are in
[`docs/table-gate-v1.md`](docs/table-gate-v1.md) — quote table numbers from there or not at all.

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

Apache-2.0. `NOTICE` records the encoding data in the tree and reserves a slot for the Adobe CMap
resources (BSD-3-Clause) if they ever land — none are present today. No AGPL dependencies —
`deny.toml` enforces it and CI proves the rule fires by flipping a crate to AGPL and requiring the
build to reject it.
