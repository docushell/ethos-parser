# ethos-engine — Node SDK

A thin Node surface over the `engine` CLI (**v1.2-S3**). Three functions, `node:` builtins only,
and **not published** — `private: true`, and this slice does not change that.

```js
import { extract, ground, nodeGet } from "ethos-engine";

const representation = extract("contract.pdf");
const grounding = ground(representation);

const nodeId = representation.representation.nodes[0].id;   // minted by the engine
const node = nodeGet(representation, nodeId);
```

| function | shells out to | returns |
| --- | --- | --- |
| `extract(pdfPath)` | `engine extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `engine ground <path>` | `ethos.grounding.v1` |
| `nodeGet(representation, nodeId)` | nothing — the checks are ported | that artifact's node record |

## This is not a second design

[`packages/python/`](../python/) shipped the same three functions at v1.2-S2 and **it is the
contract**. If this package disagreed with it about a signature, an error name, or what `ground`
accepts, this package would be the one that is wrong. The differences are the two the languages
force — `nodeGet` rather than `node_get`, and classes that throw rather than raise — and nothing
else. A test pins both SDKs to the same version string.

## It cannot diverge from what the CLI prints

`extract` and `ground` run the subcommands a shell would run and hand back the bytes those
subcommands printed, parsed with `JSON.parse`. There is no second serialization anywhere in this
package, so there is nowhere for an artifact to change: `test/cli-surface.test.js`
re-canonicalizes what `extract` returned and compares it byte for byte against the CLI's stdout.

That is also why there is no native addon. napi or neon would reach the library by a second path,
which is a second thing that can disagree with the first — plus a prebuild matrix across
platforms and ABI versions, for a saving nobody has measured a need for.

## The handle law

`docs/12-V12-SCOPE.md` §3, unchanged from MCP and Python: **the engine mints every locator,
returns it as an opaque handle, and re-validates it on the way back in.**

- **A locator is returned, never accepted as prose.** No function here takes a page, a box, an
  `x`/`y`, a width, a height or a row/column pair — not optionally, not in an options object, not
  behind a flag. A test reads the parameter names out of `Function.prototype.toString` and fails
  on those names.
- **A handle travels back as the bytes that were handed out.** `nodeGet` takes a node id string
  copied out of a representation the engine returned.
- **A handle the engine did not mint fails closed.** `nodeGet` throws `NodeNotFound`, never `null`
  and never `{}`. An edited artifact throws `FingerprintMismatch` *before* any lookup happens,
  because its payload no longer hashes to its own declared digest.

`nodeGet` is the one function with no subcommand behind it — `engine node-get` does not exist and
this slice does not add it, since MCP already carries the tool. So its checks are ported, and
[`src/c14n.js`](src/c14n.js) is c14n v1 in JavaScript, running the same parity vectors
`crates/engine-core/src/c14n.rs` and `packages/python/src/ethos_engine/_c14n.py` run.

**One thing JavaScript cannot follow, stated rather than papered over.** The language has a single
number type, so `JSON.parse("1.0")` yields the same value as `JSON.parse("1")` and no port can
separate them — the Rust and Python c14n reject float-*shaped* text and this one cannot. What all
three reject identically is a value that is genuinely not a whole number: `1.5` throws at every
depth. The unreachable half costs nothing in practice, because the engine never prints `1.0`.

## What is not here

`markdown` and `html` exist on the CLI and are not wrapped: neither proves anything this slice
claims, and a function that exists because it was cheap is a surface to keep honest forever.
`verify` is absent for a stronger reason — it relays the pinned Ethos CLI, and
`docs/07-VERIFY-BOUNDARY.md` is the boundary a host's convenience must not bend. There is no MCP
client here either: MCP is a process, this is a library.

## LangChain tools (v1.2-S4)

The same three functions as callable tools, behind an **optional peer** so the default import still
pulls nothing:

```bash
npm install @langchain/core
```

```js
import { tools } from "ethos-engine/langchain";

const withTools = model.bindTools(tools());   // or a LangGraph ToolNode
```

**Locators travel in the tool `artifact`, never in `content`.** Each tool declares
`responseFormat: "content_and_artifact"`, so a `ToolMessage` carries the record in `.artifact` and
a summary in `.content` — counts, and nothing a pipeline would bind to:

| tool | `content` | `artifact` |
| --- | --- | --- |
| `extract` | `{n} page(s), {m} node(s). Locators are in the artifact.` | `DocumentRepresentation v0` |
| `ground` | `{n} element(s) with a measured box; {m} omitted for having none.` | `ethos.grounding.v1` |
| `node_get` | ``1 node, kind `{kind}`.`` | the node record |

A box in `content` is a locator a model can edit and then cite, which is the hazard the whole
version is arranged against. The summaries are MCP's own and the test compares them **byte for
byte** against what `engine mcp` emits; the argument schemas are the ones `tools/list` advertises,
verbatim — including `node_id` rather than `nodeId`, because the tool argument is the wire and one
wire has one name.

A forged id, an edited payload or an unreadable document **throws** — MCP's `isError: true` in this
framework's currency. No tool sets trust state, there is no `verify` tool, and there is no
LangGraph adapter: a bindable tool is already what LangGraph binds.

Importing `ethos-engine/langchain` without the peer is a named failure carrying the install
command; the tests skip with that same command when it is absent.

## Running it

The `engine` binary is a prerequisite; nothing here downloads or vendors one. The core suite has
nothing to install — no runtime dependency means no lockfile and no `npm install`.

```bash
cargo build --release --locked && ETHOS_ENGINE=target/release/engine node --test packages/node
```

The LangChain tests need the optional peer and skip with the install command without it:

```bash
npm install --no-save @langchain/core
```

`ETHOS_ENGINE` is checked **first and authoritatively** — a path named there and not present is an
error, not a reason to go looking for some other build — then `engine` on `PATH`. That is the
precedent `VerifierBinary::resolve` sets for `ETHOS_BIN`, and the reason is the same: resolving to
a binary nobody chose means returning artifacts from a parser nobody chose.

The suite additionally checks `engine --version` against `Cargo.toml` and refuses a binary from
another version. A stale `target/release/engine` would otherwise be preferred over nothing and
answer every question plausibly — and a byte-identity check that compares the SDK against the CLI
using the same stale binary is self-consistent, so it would go green about the wrong engine.
