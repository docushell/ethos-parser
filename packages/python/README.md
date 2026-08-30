# ethos-parser — Python SDK

Three functions over the `ethos-parser` CLI. Standard library only, no runtime dependencies, and
not on PyPI yet.

```python
import ethos_parser

representation = ethos_parser.extract("contract.pdf")
grounding = ethos_parser.ground(representation)

node_id = representation["representation"]["nodes"][0]["id"]   # minted by the engine
node = ethos_parser.node_get(representation, node_id)
```

| Function | Runs | Returns |
| --- | --- | --- |
| `extract(path)` | `ethos-parser extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `ethos-parser ground <path>` | `ethos.grounding.v1` |
| `node_get(representation, node_id)` | nothing — the checks are ported | that artifact's node record |

## Why it wraps the CLI

`extract` and `ground` run the same subcommands you would run in a shell and hand back exactly what
those subcommands printed, parsed as JSON. There is no second serializer in this package, so there
is nowhere for the two to drift apart. A test re-canonicalizes what `extract` returned and compares
it byte for byte against the CLI's stdout.

That is also why there is no PyO3 extension. A native binding would reach the engine by a second
path, and a second path is a second thing that can disagree with the first — plus a wheel matrix, for
a speed-up nobody has measured a need for.

## Locators are handles, not arguments

The engine mints every locator, returns it as an opaque id, and re-validates it on the way back in.

- **No function here takes a coordinate.** No page, no box, no `x`/`y`, no row/column pair — not
  optionally, not keyword-only, not behind a flag. A test reads the signatures and fails on those
  names.
- **`node_get` takes an id you copied out of a representation the engine returned.**
- **A forged id fails closed.** `node_get` raises `NodeNotFound`, never `None` and never `{}`. An
  edited artifact raises `FingerprintMismatch` *before* any lookup happens, because its payload no
  longer hashes to its own declared digest.

`node_get` is the one function with no subcommand behind it, so its checks are ported:
`src/ethos_parser/_c14n.py` runs the same parity vectors as the Rust implementation.

`markdown`, `html` and `verify` are deliberately not wrapped.

## LangChain tools

An optional extra, so the plain install still pulls nothing:

```bash
pip install 'ethos-parser[langchain]'
```

```python
from ethos_parser.langchain import tools

llm_with_tools = llm.bind_tools(tools())      # or a LangGraph ToolNode
```

**Locators travel in the tool's `artifact`, never in `content`.** Each tool declares
`response_format="content_and_artifact"`, so a `ToolMessage` carries the record in `.artifact` and
only counts in `.content`:

| Tool | `content` | `artifact` |
| --- | --- | --- |
| `extract` | `{n} page(s), {m} node(s). Locators are in the artifact.` | `DocumentRepresentation v0` |
| `ground` | `{n} element(s) with a measured box; {m} omitted for having none.` | `ethos.grounding.v1` |
| `node_get` | ``1 node, kind `{kind}`.`` | the node record |

A box in `content` is a locator the model can edit and then cite, which is the whole hazard. The
summaries are MCP's own, compared byte for byte in a test, so the three adapters cannot drift into
three different sentences about one document.

There is no `verify` tool and no LangGraph adapter — a `StructuredTool` is already what LangGraph
binds.

## Running the tests

The `ethos-parser` binary is a prerequisite; nothing here downloads one.

```bash
cargo build --release --locked                 # from the repository root
pip install -e 'packages/python[dev]'
ETHOS_PARSER=target/release/ethos-parser pytest packages/python
```

`ETHOS_PARSER` is checked first and taken literally — a path named there that does not exist is an
error, not a reason to go hunting for some other build. Then `ethos-parser` on `PATH`, then the
workspace `target/`. The suite also refuses a binary whose version does not match `Cargo.toml`,
because a stale build answers every question plausibly and would make a byte-identity check go green
about the wrong engine.
