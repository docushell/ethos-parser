# ethos-parser — Node SDK

Three functions over the `ethos-parser` CLI. `node:` builtins only, no runtime dependencies, and not
published (`private: true`).

```js
import { extract, ground, nodeGet } from "ethos-parser";

const representation = extract("contract.pdf");
const grounding = ground(representation);

const nodeId = representation.representation.nodes[0].id;   // minted by the engine
const node = nodeGet(representation, nodeId);
```

| Function | Runs | Returns |
| --- | --- | --- |
| `extract(path)` | `ethos-parser extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `ethos-parser ground <path>` | `ethos.grounding.v1` |
| `nodeGet(representation, nodeId)` | nothing — the checks are ported | that artifact's node record |

## It is the Python SDK in another language

[`packages/python/`](../python/) is the contract. If this package disagreed with it about a
signature, an error name, or what `ground` accepts, this package would be the one that is wrong. The
only differences are the ones JavaScript forces — `nodeGet` rather than `node_get`, classes that
throw rather than exceptions that raise. A test pins both to the same version string.

## Why it wraps the CLI

`extract` and `ground` run the same subcommands you would run in a shell and hand back exactly what
those subcommands printed, parsed with `JSON.parse`. There is no second serializer here, so there is
nowhere for the two to drift apart — a test re-canonicalizes what `extract` returned and compares it
byte for byte against the CLI's stdout.

That is also why there is no native addon. napi or neon would reach the engine by a second path, and
a second path is a second thing that can disagree with the first — plus a prebuild matrix across
platforms and ABI versions.

## Locators are handles, not arguments

The engine mints every locator, returns it as an opaque id, and re-validates it on the way back in.

- **No function here takes a coordinate.** No page, no box, no `x`/`y`, no row/column pair — not
  optionally, not in an options object, not behind a flag. A test reads the parameter names out of
  `Function.prototype.toString` and fails on those names.
- **`nodeGet` takes an id you copied out of a representation the engine returned.**
- **A forged id fails closed.** `nodeGet` throws `NodeNotFound`, never `null` and never `{}`. An
  edited artifact throws `FingerprintMismatch` *before* any lookup happens, because its payload no
  longer hashes to its own declared digest.

[`src/c14n.js`](src/c14n.js) runs the same parity vectors as the Rust and Python implementations,
sorting keys by code point because JavaScript's default sort compares UTF-16 units and disagrees
with Rust above the BMP.

**One thing JavaScript cannot do, stated rather than papered over.** It has a single number type, so
`JSON.parse("1.0")` and `JSON.parse("1")` give the same value and no port can tell them apart. Rust
and Python reject float-*shaped* text; this one cannot. All three reject a value that genuinely is
not a whole number — `1.5` throws everywhere — and the engine never prints `1.0`, so the gap costs
nothing in practice.

`markdown`, `html` and `verify` are deliberately not wrapped.

## LangChain tools

An optional peer, so the plain import still pulls nothing:

```bash
npm install @langchain/core
```

```js
import { tools } from "ethos-parser/langchain";

const withTools = model.bindTools(tools());   // or a LangGraph ToolNode
```

**Locators travel in the tool's `artifact`, never in `content`.** Each tool declares
`responseFormat: "content_and_artifact"`, so a `ToolMessage` carries the record in `.artifact` and
only counts in `.content`:

| Tool | `content` | `artifact` |
| --- | --- | --- |
| `extract` | `{n} page(s), {m} node(s). Locators are in the artifact.` | `DocumentRepresentation v0` |
| `ground` | `ethos-parser mcp`'s own reply text: `{n} element(s) with a measured box; {m} omitted for having none.`, then a clause for each thing the schema's limits took — spans withheld, elements omitted, tables withheld | `ethos.grounding.v1` |
| `node_get` | ``1 node, kind `{kind}`.`` | the node record |

A box in `content` is a locator the model can edit and then cite, which is the whole hazard. The
argument schemas are MCP's own, verbatim — including `node_id` rather than `nodeId`, because the tool
argument is the wire and one wire has one name — and so are the summaries' words: `ground`'s is the
text `ethos-parser mcp` itself replied with, since it states facts the artifact does not carry, and
`extract`'s is compared byte for byte in a test. `node_get` spells the kind the way the artifact
does (`text_run`). The `ground` tool starts `ethos-parser mcp` and sends it a path, never the
object; if the record is refused it runs `ethos-parser ground` on the same path, so the error thrown
is `ground()`'s own.

Importing `ethos-parser/langchain` without the peer fails with the install command in the message.
There is no `verify` tool and no LangGraph adapter.

## Running the tests

The `ethos-parser` binary is a prerequisite; nothing here downloads one. The core suite has nothing
to install — no runtime dependency means no lockfile and no `npm install`.

```bash
cargo build --release --locked && ETHOS_PARSER=target/release/ethos-parser node --test packages/node
```

The LangChain tests need the optional peer and skip with the install command without it:

```bash
npm install --no-save @langchain/core
```

`ETHOS_PARSER` is checked first and taken literally — a path named there that does not exist is an
error, not a reason to go hunting for some other build. Then `ethos-parser` on `PATH`, then the
workspace `target/`. The suite also refuses a binary whose version does not match `Cargo.toml`,
because a stale build answers every question plausibly and would make a byte-identity check go green
about the wrong engine.
