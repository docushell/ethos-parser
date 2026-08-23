# 13 — v1.2 slices

**Status:** implementation authority for v1.2 · **Scope document:** `12-V12-SCOPE.md`
**This is the code-review map for v1.2.** Every v1.2 PR belongs to exactly one slice.

**v1 is not done.** S7's gate is measured and **missed**, and 489‰ is a published comparator
rather than a live floor — the chase is **parked**, which is not a pass (`00-NORTH-STAR.md` #10,
`09-V1-MILESTONES.md` S7, `table-gate-v1.md`).

> **Note added by v2-S19.** Every `64‰` in this document is the **four-document** measurement that
> was current while v1.2 ran. v2-S19 grew the gate corpus to **twelve** documents and re-measured:
> the macro reads **70‰**, and the band — 0‰..590‰, median 0‰, **nine of twelve at zero** — shows
> the macro was never the right summary of this detector. **The per-slice acceptance lines below
> are left exactly as they were measured**, because they record what was true when each slice
> shipped and rewriting them would erase the evidence that the number moved. Read them as history;
> read `table-gate-v1.md` for the current number. **v1.1 is complete** at 0.14.1. v1.2 began because
the owner asked for the next roadmap row, and nothing in it closes v1.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v1.2 scope + this document | — | **done** |
| **S1** | MCP over stdio: `extract`, `ground`, `node_get` | S0 | **done** |
| **S2** | Python SDK — thin, over the same library or CLI | S1 | **done** |
| **S3** | Node SDK | S2 | **done** |
| **S4** | LangChain tool, locators in `artifact` never `content` | S2, S3 | **done** |
| **S5** | Optional `liteparse` → `ethos.grounding.v1` adapter | S1 | **done — refused** |

---

## S0 — v1.2 scope and slice map

- **Goal:** v1.2 exists as an ordered list of bounded changes **before** any of it is implemented,
  and the handle law is written down **before** the first host that can violate it.

- **In:** `12-V12-SCOPE.md` (what v1.2 is, what it is not, the handle law, §16.7's hazard quoted
  rather than paraphrased); this document.

- **Out:** any code.

- **Acceptance tests:**
  - [x] Both documents exist and name S2–S5 as **not started**
  - [x] The handle law — **mint / opaque / re-validate**, and *fails closed* — is written down in
        `12-V12-SCOPE.md` §3, with §16.7's hazard quoted rather than paraphrased away
  - [x] The corollary that no tool argument may carry document geometry is stated as something
        **checkable**, and S1 checks it
  - [x] `08-V1-SCOPE.md` and `09-V1-MILESTONES.md` still say S7 is missed

- **Why first:** so the first adapter cannot quietly acquire a `bbox` argument "for convenience"
  while nobody has written down that it may not.

---

## S1 — MCP over stdio

- **Status: done.** `engine mcp` at **0.15.0**. Three tools, no new crate, no new dependency, and
  `deny.toml`'s network bans untouched.

- **Goal:** the first adapter, and the one §16.7 puts first — reachable from every host on its list
  including the licence-failing ones, without dragging their licences toward us.

### The transport is a pipe

**stdio, newline-delimited JSON-RPC.** One request object per line in, one response per line out.
That is MCP's own stdio transport, so it is the native format rather than a dialect invented here,
and it needs no HTTP, no SSE, no socket and no async runtime. `deny.toml` bans `tokio`, `hyper`,
`reqwest`, `ureq` and `rustls` among others; this slice stays inside that ban rather than filing an
ADR to widen it, and `cargo deny check` is the standing proof.

The protocol surface is five methods — `initialize`, `notifications/initialized`, `tools/list`,
`tools/call`, `ping` — implemented as a loop over stdin with the `serde_json` the workspace already
depends on. No MCP framework is vendored: the ones available pull an async runtime, which would
cost the ban to save a few dozen lines.

### The three tools, and why not more

| tool | argument | returns |
| --- | --- | --- |
| `extract` | `path` | `DocumentRepresentation v0`, byte-identical to `engine extract` |
| `ground` | `representation` | `ethos.grounding.v1` |
| `node_get` | `representation`, `node_id` | the node record from **that** artifact |

`markdown` and `html` are **not** here. They would be a few lines each and neither is needed to
prove anything this slice claims; the version gate is the handle round-trip, and a tool that exists
because it was cheap is a surface to keep honest forever. They arrive when a caller needs them.

`verify` is not here either. It relays the pinned Ethos CLI, and a relay wrapped in a second
protocol is a second place for a verdict to be re-derived — `07-VERIFY-BOUNDARY.md` is exactly the
thing this version must not bend for a host's convenience.

### The handle law, made mechanical

`node_get` is the whole gate, and it is deliberately the smallest form of it:

1. The caller passes back a `representation` — the artifact `extract` **minted**, either inline or
   as a path to bytes the engine wrote.
2. The engine **re-validates** it: `verify_fingerprint()` first, exactly as `ground`, `markdown`
   and `html` do. A representation whose payload does not hash to its own declared digest is not a
   record this engine will speak for, so a model that edited the JSON on the way through fails here
   rather than getting an answer about a document that never existed.
3. `node_id` is looked up **among that artifact's own nodes**. Found → the node record. Not found →
   a tool error.

**A forged id fails closed.** Not the nearest node, not an empty object, not `null` — an error,
because an empty answer tells a model its guess was unlucky and an error tells it the guess was not
admissible.

**No tool argument carries geometry.** No `page`, no `bbox`, no `x`/`y`/`w`/`h`, no row/column pair,
in any of the three schemas. A test reads the schemas the server advertises and fails on those field
names, so the rule is enforced against the wire rather than against a reviewer's memory.

### Where the locators live

`structuredContent` carries the canonical artifact — the same bytes `engine extract` prints, parsed
as JSON. `content` carries a **short summary**: counts, and nothing else. No box, no id, no cell.

That split is §16.7's, and it is the difference between a locator a pipeline can bind and a locator
a model can edit. A test asserts that no coordinate reaches `content`.

### Statelessness is not a limitation here

Every `tools/call` is self-contained. The server keeps no document cache, so call 2 never depends
on call 1 having happened — **passing the artifact back is the session**. MCP's own July 2026
statelessness change suits a deterministic engine exactly, and it means the handle law has no
back door: there is no server-side table of "documents I have seen" whose keys a model could
enumerate.

- **In:** `engine-cli/src/mcp.rs` (protocol plumbing only — the parsing lives in the library, as it
  does for every other subcommand); the `engine mcp` subcommand; `0.15.0` and the moved profile
  hash; `12`/`13`; CHANGELOG; README.

- **Out:** HTTP or SSE MCP, Tokio, TLS, sockets. A Python or Node SDK. A LangChain tool. A
  liteparse mapper. `markdown`/`html`/`verify` tools. `capabilities.mcp`. A fifth crate. A tag.

- **Acceptance tests:**
  - [x] `engine mcp` answers `initialize` and `tools/list`, and `Cargo.lock` contains no `tokio`
  - [x] `extract` through MCP is **byte-identical** to `engine extract` on the same document
  - [x] `node_get` with a minted id returns that node; a forged id is an **error**, not an empty
        result
  - [x] `node_get` on a representation whose payload was edited fails the fingerprint check
  - [x] No tool's `inputSchema` mentions `page`, `bbox`, `x`, `y`, `width`, `height`, `row` or
        `column`
  - [x] No coordinate appears in any `content` string
  - [x] `cargo deny check` still passes with the network bans in force
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables; the
        markdown and html goldens still green
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt

- **Depends on:** S0.

---

## S2 — Python SDK

- **Status: done.** `packages/python/` at **0.16.0**. Three functions, **no runtime dependency**,
  no new crate, no Rust changed but the version.

- **Goal:** a thin Python surface over the same library or CLI, so it cannot diverge from what the
  CLI prints.

- **The standing constraint:** §3's handle law. A locator is returned, never accepted as prose.

### The decision that makes divergence impossible rather than unlikely

**It wraps the CLI with `subprocess`, and that is the whole design.** `extract` and `ground` run
the subcommands a shell would run and hand back the bytes those subcommands printed, parsed by the
stdlib `json` module. There is no second serialization anywhere in the package, so byte-identity
with the CLI is a **tautology** rather than a promise — `tests/test_cli_surface.py` re-canonicalizes
what `extract` returned and compares it against the CLI's stdout byte for byte, and the only way
that could fail is if Python's JSON parser lost something.

PyO3 was the alternative and it was refused. It would reach the library by a **second path**, which
is a second thing that can disagree with the first — plus a wheel matrix, a fifth build surface,
and a route by which a Rust dependency could arrive on the Python side of the fence. The cost of
the chosen shape is one process spawn per call, which nobody has measured a need to avoid.

The same reasoning refuses an MCP client here: MCP is a **process** and this is a **library**.
They are two callers of one binary, not layers.

### The surface, and the one function with nothing behind it

| function | shells out to | returns |
| --- | --- | --- |
| `extract(pdf_path)` | `engine extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `engine ground <path>` | `ethos.grounding.v1` |
| `node_get(representation, node_id)` | **nothing** | the node record from **that** artifact |

`ground` takes the artifact `extract` returned — the object itself, or a path to bytes this engine
wrote — which is what the MCP tool of the same name takes. **It does not take a quote, and it does
not take a page**, because `engine ground` takes neither: it projects a representation into
`ethos.grounding.v1`, and a locator-shaped argument would be the corollary violation in its purest
form.

`node_get` has **no `engine node-get` subcommand** and this slice did not add one. MCP already
carries the tool; a third CLI verb would exist for symmetry, which is not a reason. So its checks
are **ported**, in the order `mcp.rs` runs them:

1. the value is a representation — `artifact_type` under `ethos.engine.representation.`;
2. `representation` is re-canonicalized and re-hashed and must equal `representation_c14n_sha256`
   — an artifact edited on the way through is refused **before any lookup happens**;
3. `node_id` is looked up among **that artifact's own nodes**, and a miss raises.

That is why `src/ethos_engine/_c14n.py` exists: it is c14n v1 in Python, ~100 lines, running
`engine-core/src/c14n.rs`'s **own parity vectors**. A fingerprint that were merely *nearly* the
engine's would be worse than none — it would accept an artifact the engine refuses, or refuse one
the engine minted, and either way a caller would be told something false about a document. The
load-bearing proof is not the vectors but the whole artifact: `extract`'s output, re-canonicalized
in Python, reproduces the CLI's bytes exactly, which means the port agrees with the Rust on real
input and not merely on five hand-written values.

### The handle law in Python

- `node_get(rep, minted_id)` returns that node — **the object the artifact carries**, not a copy.
- `node_get(rep, "s-forged")` raises `NodeNotFound`, naming what was refused. **Never `None`,
  never `{}`.**
- `node_get(edited_rep, minted_id)` raises `FingerprintMismatch`, naming both digests.
- **No public function signature contains** `page`, `bbox`, `x`, `y`, `width`, `height`, `row` or
  `column` — read off `inspect.signature`, against the same banned list `mcp.rs` uses.

`ground` deliberately does **not** repeat the fingerprint check in Python. The engine runs it, and
the SDK surfaces the engine's own refusal with its stderr intact: a second check here is a second
thing that could drift from the first, which is the failure mode this whole slice is arranged
against.

### What is deliberately absent

`markdown()` and `html()` exist on the CLI and are not wrapped — neither proves anything this slice
claims, and a function that exists because it was cheap is a surface to keep honest forever.
`verify()` is absent for a stronger reason: it relays the pinned Ethos CLI, and a Python function
of that name would look like this package had an opinion about whether a claim is supported.
`07-VERIFY-BOUNDARY.md` is exactly what an SDK's convenience must not bend.

- **In:** `packages/python/` (`pyproject.toml`, `src/ethos_engine/`, tests, README); `0.16.0` and
  the moved profile hash; `12`/`13`; CHANGELOG; README; `docs/README.md`.

- **Out:** PyO3, maturin, a native extension, a fifth crate. Any runtime dependency. An
  `engine node-get` CLI verb. `markdown`/`html`/`verify` Python functions. An MCP Python client.
  `capabilities.python`. A Node SDK, a LangChain tool, a liteparse mapper. Publication to PyPI. A
  tag.

- **Acceptance tests:**
  - [x] `packages/python/` exists and `import ethos_engine` works
  - [x] Runtime dependencies are **empty** — asserted both from `pyproject.toml` and by walking
        every `import` statement in the package against `sys.stdlib_module_names`
  - [x] `extract(pdf)` re-canonicalizes to `engine extract`'s stdout **byte for byte**
  - [x] `ground` matches `engine ground` from the object and from a path
  - [x] `node_get` with a minted id returns that node; a forged id **raises**; an edited payload
        **raises** at the fingerprint
  - [x] No public signature names `page`, `bbox`, `x`, `y`, `width`, `height`, `row` or `column`
  - [x] No `markdown` / `html` / `verify` / `mcp` Python API
  - [x] The c14n port matches `engine-core`'s own parity vectors, and rejects floats at every depth
  - [x] `ETHOS_ENGINE` is authoritative; a missing binary is a **named failure**, never a skip
  - [x] No PyO3, no fifth crate, no Tokio; `cargo deny check` still passes
  - [x] Workspace **0.16.0**, profile hash
        `sha256:1b7a4208734b9ed52f1c0b2b725322bc854ffac229c019c01226203c67c5a81a`
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables; the
        markdown and html goldens still green
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt
  - [x] `pytest` green in `packages/python`

- **Depends on:** S1.

---

## S3 — Node SDK

- **Status: done.** `packages/node/` at **0.17.0**. Three functions, **no runtime dependency**,
  no new crate, no Rust changed but the version.

- **Goal:** the same surface for Node, on the same terms.

- **The standing constraint:** §3's handle law. A locator is returned, never accepted as prose.

### This is not a second design

**S2 is the contract.** If Node disagreed with Python about a signature, an error name, or what
`ground` accepts, Node would be the one that is wrong — so the differences are exactly the two the
languages force, and nothing else:

| Python | Node | why |
| --- | --- | --- |
| `node_get` | `nodeGet` | naming convention; the same function |
| `raise NodeNotFound(...)` | `throw new NodeNotFound(...)` | classes, and the same six names |

`extract`, `ground`, `REPRESENTATION_ARTIFACT_TYPE`, the error taxonomy, the banned-argument list,
the binary-resolution order and the fixture the tests run against are all shared. A test pins both
SDKs and `package.json` to the workspace version, because two adapters at different versions over
one binary is exactly the disagreement a version string exists to make legible.

### The shape, decided once at S2 and reused

**It spawns the CLI.** `extract` and `ground` run the subcommands a shell would run and hand back
the bytes those subcommands printed, parsed with `JSON.parse`. There is no second serialization
anywhere in the package, so byte-identity is a **tautology** rather than a promise —
`test/cli-surface.test.js` re-canonicalizes what `extract` returned and compares it against the
CLI's stdout byte for byte.

**No native addon.** napi and neon were refused for the reason S2 refused PyO3: a second path to
the library is a second thing that can disagree with the first, and this one would add a prebuild
matrix across platforms and ABI versions to buy it. **No TypeScript, no bundler, no test
framework** — Node 18 has `node:test` and `node:assert/strict`, and a `.d.ts` that needed a
generator would be a build step this package does not have. Runtime dependencies are **empty**,
asserted from `package.json` and again by reading every `import` specifier in the sources.

### The surface

| function | shells out to | returns |
| --- | --- | --- |
| `extract(pdfPath)` | `engine extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `engine ground <path>` | `ethos.grounding.v1` |
| `nodeGet(representation, nodeId)` | **nothing** | the node record from **that** artifact |

`ground` takes the artifact `extract` returned — the object itself, or a path to bytes this engine
wrote — which is what Python's `ground` and the MCP tool take. **It does not take a quote and it
does not take a page**, because `engine ground` takes neither. When handed an object it writes
**c14n bytes** to a temp file, not `JSON.stringify` output: a second serialization is the one thing
this package exists not to have. It does **not** repeat the fingerprint check in JavaScript — the
engine runs it, and the SDK surfaces the engine's own refusal with its stderr intact.

`nodeGet` has **no `engine node-get` subcommand** and this slice did not add one. Its checks are
ported in the order `mcp.rs` and Python run them: the value is a representation; the payload is
re-canonicalized and re-hashed and must equal `representation_c14n_sha256`, **before any lookup**;
`nodeId` is looked up among that artifact's own nodes, and a miss throws.

### c14n, ported a second time — and where JavaScript cannot follow

`src/c14n.js` runs `engine-core/src/c14n.rs`'s **own parity vectors**, as `_c14n.py` does. Two
hazards are specific to this language and both are handled rather than hoped:

- **Key order.** `Array.prototype.sort` compares UTF-16 code units, which disagrees with Rust's
  `String: Ord` above the BMP — a key starting U+1F4A1 sorts *before* one starting U+FFFD under
  code-unit order and *after* under code-point order. The port sorts by code point explicitly, and
  a test asserts the default sort would have got that pair wrong.
- **Lone surrogates.** `Buffer.from` encodes one as U+FFFD, which is a silent repair of evidence.
  The port throws instead, matching what Rust's type system makes impossible.

**One divergence is real and is written down rather than papered over.** JavaScript has a single
number type, so `JSON.parse("1.0")` yields the same value as `JSON.parse("1")` and nothing can
separate them; Rust and Python reject float-*shaped* text and this port cannot. What all three
reject identically is a value that is genuinely not a whole number — `1.5` throws at every depth,
which is the property c14n needs. The unreachable half is unreachable in practice: the engine never
prints `1.0`, because Rust c14n forbids it and the CLI's stdout *is* c14n bytes. A test pins the
divergence so it stays a named property rather than a surprise.

### The handle law in Node

- `nodeGet(rep, mintedId)` returns that node — **the same object the artifact carries**, not a copy.
- `nodeGet(rep, "s-forged")` throws `NodeNotFound`, naming what was refused. **Never `null`, never
  `undefined`, never `{}`.**
- `nodeGet(editedRep, mintedId)` throws `FingerprintMismatch`, naming both digests.
- **No exported function parameter is named** `page`, `bbox`, `box`, `rect`, `x`, `y`, `w`, `h`,
  `width`, `height`, `row`, `column`, `col`, `span`, `offset`, `coords` or `region` — the same list
  `mcp.rs` and Python use. `Function.prototype.length` says nothing about names, so the test reads
  the real header out of `Function.prototype.toString` and asserts the names it recovered, so a
  parse that returned nothing could not make the ban vacuous.

### What is deliberately absent

`markdown()` and `html()` — neither proves anything this slice claims. `verify()` — it relays the
pinned Ethos CLI, and a JavaScript function of that name would look like this package had an
opinion about whether a claim is supported. No MCP client: MCP is a process, this is a library.

- **In:** `packages/node/` (`package.json`, `src/`, `test/`, README); the `0.17.0` bump and the
  moved profile hash; the Python SDK's `__version__` pin, which the bump requires; `12`/`13`;
  CHANGELOG; README; `docs/README.md`.

- **Out:** napi, neon, node-gyp, prebuild, any native addon. TypeScript, a bundler, Jest, Vitest,
  a generated `.d.ts`. Any runtime dependency. An `engine node-get` CLI verb.
  `markdown`/`html`/`verify` JS functions. An MCP client. `capabilities.node`. Publication to npm.
  A LangChain tool, a liteparse mapper. A tag.

- **Acceptance tests:**
  - [x] `packages/node/` exists and the package resolves through its own `exports` map
  - [x] Runtime dependencies are **empty** — asserted from `package.json` (`dependencies`,
        `optionalDependencies`, `peerDependencies`) and by reading every `import` specifier
  - [x] `extract(pdf)` re-canonicalizes to `engine extract`'s stdout **byte for byte**
  - [x] `ground` matches `engine ground` from the object and from a path
  - [x] `nodeGet` with a minted id returns that node; a forged id **throws**; an edited payload
        **throws** at the fingerprint
  - [x] No exported parameter names a coordinate, read off `Function.prototype.toString`
  - [x] No `markdown` / `html` / `verify` / `mcp` JS API
  - [x] The c14n port matches `engine-core`'s own parity vectors, rejects non-integers at every
        depth, sorts by code point rather than code unit, and refuses a lone surrogate
  - [x] `ETHOS_ENGINE` is authoritative; a missing binary is a **named failure**, never a skip
  - [x] No napi/neon, no fifth crate, no Tokio; `cargo deny check` still passes
  - [x] Workspace **0.17.0**, Python `__version__` **0.17.0**, `package.json` **0.17.0**, profile
        hash `sha256:83cd55301d2423d54033e449b2bcdbd07b5a5c926c441dc456cb93edc7788774`
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables; the
        markdown and html goldens still green
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt
  - [x] `node --test` green in `packages/node`; `pytest` still green in `packages/python`

- **Depends on:** S2.

---

## S4 — LangChain tools

- **Status: done.** `ethos_engine.langchain` and `ethos-engine/langchain` at **0.18.0**. Three
  tools per language, no new crate, no Rust changed but the version, and **neither SDK's empty
  runtime install moved**.

- **Goal:** a callable tool for LangChain (Python and JS), running unchanged inside LangGraph.

- **The standing constraint:** §16.7's LangChain row — **locators travel in the tool `artifact`,
  never in `content`** — and the tool must not set trust state.

### The split is the whole slice

`12-V12-SCOPE.md` §3's second corollary says where locators live, and MCP already implemented it.
LangChain implements the same sentence in its own types, so the table is a translation and not a
design:

| MCP | LangChain |
| --- | --- |
| `structuredContent` | the tool's **`artifact`** — the SDK object, unaltered |
| `content` text | the tool's **`content`** — counts, and nothing a pipeline would bind to |

`response_format="content_and_artifact"` (`responseFormat` in JS) is what makes both sides real.
Without it the artifact is stringified into `content`, and a box in `content` is a locator a model
can edit and then cite — which is the failure this whole version is arranged to prevent. Both
optional pins are bounded on **both** sides for that one reason: a major bump is where the
contract could change, and it should break loudly rather than silently downgrade the split.

| tool | `content` | `artifact` |
| --- | --- | --- |
| `extract` | `{n} page(s), {m} node(s). Locators are in the artifact.` | `DocumentRepresentation v0` |
| `ground` | `{n} element(s) with a measured box; {m} omitted for having none.` | `ethos.grounding.v1` |
| `node_get` | ``1 node, kind `{kind}`.`` | the node record |

**MCP is the oracle for those strings, not this slice's opinion.** The suites compare `content`
byte-for-byte against what `engine mcp` emits on the same document, on one fixture where nothing
is omitted and one where everything is. That is what stops a second adapter inventing a richer
sentence than the first — and it is also the proof that `ground`'s omitted count, computed out
here as **nodes minus elements**, equals the engine's own `omission.nodes_omitted`. Counting
geometry rows instead would re-encode `GeometryPresence::is_groundable` in two more languages, and
a count derived from a different question than the one being asked is a count that goes wrong the
first time a second absence variant appears.

`node_get`'s summary names the **kind** and never the id: a kind is a category, an id is a handle,
and a handle in the one channel a model can rewrite is the hazard itself. The kind is spelled the
way the **artifact** spells it (`text_run`), not the way `engine mcp` prints Rust's `Debug`
(`TextRun`) — reshaping it would be the adapter inventing a name for a thing it did not read.

### One wire shape, checked against the wire

The three argument schemas are **plain JSON Schema, verbatim from what `engine mcp` advertises**,
and a test in each language asserts them against `tools/list` rather than against a reviewer's
memory. That is where the geometry ban lives too: no `page`, no `bbox`, no `x`/`y`, no row/column
pair, in any of them.

`node_id` keeps MCP's spelling in **both** languages even though the Node SDK's function parameter
is `nodeId`. The tool argument is the wire, and one wire has one name.

JSON Schema rather than pydantic models or zod: it is what both frameworks accept, it lets the
three adapters be compared object to object, and it keeps the Node package needing nothing but
`@langchain/core`.

### The install stays empty

| package | how LangChain arrives | subpath |
| --- | --- | --- |
| `packages/python` | optional extra `[langchain]`; `dependencies` stays `[]` | `ethos_engine.langchain` |
| `packages/node` | optional peer `@langchain/core`; `dependencies` stays absent | `ethos-engine/langchain` |

`import ethos_engine` and `import "ethos-engine"` still reach nothing but the stdlib, and a test in
each language asserts it by reading the import graph of the **default entry point only**. Importing
the subpath without the dependency is a **named** failure carrying the install command — in JS via
a dynamic `import` inside a try/catch at module load, so it fails exactly where Python's
`ImportError` fails, rather than as Node's generic module-not-found.

### No LangGraph adapter, and no trust state

§16.7 refused a separate LangGraph integration and this slice honours that: a `StructuredTool` is
already what `bind_tools` and a `ToolNode` take, so a graph, a node or a checkpointer here would be
a second surface that proves nothing. **No `langgraph` package is depended on**, in either
language, and a test asserts it.

No tool result, summary or annotation says `grounded`, `verified`, an evidence tier, a score or a
degree of belief; `07-VERIFY-BOUNDARY.md` is unchanged, and there is no `verify` tool. A failure —
a forged id, an edited payload, an unreadable document — **raises**, which is MCP's `isError: true`
in this framework's currency. Nothing catches what the SDK raises, because an empty result would
tell a model its guess was merely unlucky.

### A false green the slice found and closed

Both SDK suites preferred `target/release/engine` over `target/debug/engine` and took the first
file that existed. On a tree with a stale release build that was a **`0.11.0`** binary, and every
S2 and S3 assertion passed against it: the byte-identity checks compare the SDK against the CLI
using the same binary, so they are self-consistent whichever one it is. They proved what they
claim, about the wrong engine.

Both locators now read `engine --version` and refuse a binary that is not this workspace's — an
explicit `ETHOS_ENGINE` pin errors rather than being silently overridden, and the search skips a
build from another version and names what it found. Measured, not feared: it is what tripped this
slice's first `engine mcp` call.

- **In:** `packages/python/src/ethos_engine/langchain.py` + the `[langchain]` extra;
  `packages/node/src/langchain.js` + the `./langchain` export and optional peer; the test-harness
  version guard in both suites; `0.18.0` and the moved profile hash; both SDK version pins;
  `12`/`13`; CHANGELOG; README; `docs/README.md`.

- **Out:** `langgraph` / `@langchain/langgraph` in either language. A default-install dependency.
  A `verify`, `markdown` or `html` tool. An MCP client. A custom agent, chain, retriever or
  `Document` loader. LlamaIndex, Haystack, RAGFlow, Dify, n8n, Windmill, Langflow.
  `capabilities.langchain`. A liteparse mapper. Publication. A tag.

- **Acceptance tests:**
  - [x] Python `dependencies` still `[]`; Node `dependencies` still absent; `@langchain/core` is
        an **optional** peer and `langchain-core` an extra
  - [x] `import ethos_engine` / `import "ethos-engine"` reaches no framework, asserted off the
        default entry point's import graph
  - [x] Three tools per language: `extract`, `ground`, `node_get`
  - [x] `artifact` is the SDK object; `content` is MCP's counts, **byte-for-byte against
        `engine mcp`** on an omitting and a non-omitting document
  - [x] No summary carries `bbox`, `[`, `x0`, `origin`, `sha256:`, the **real** minted id or the
        **real** fingerprint — read off the artifact rather than hardcoded
  - [x] Forged `node_id` raises; an edited representation raises at the fingerprint; an unreadable
        document raises
  - [x] The argument schemas **equal** the ones `engine mcp` advertises; none names a coordinate
  - [x] No trust-state word on a result, summary, description or schema; `status` is `success`
  - [x] No LangGraph dependency in either language
  - [x] Workspace **0.18.0**, both SDKs and `package.json` **0.18.0**, profile hash
        `sha256:2bf3e74e6a3b972669cbc03afda66c4949b870d9695a7739dfc255a859485fee`
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables; the
        markdown and html goldens still green
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt
  - [x] `pytest` green in `packages/python`; `node --test` green in `packages/node` with the peer
        present, and skipping the S4 file with the install command when it is absent

- **Depends on:** S2, S3.

---

## S5 — optional `liteparse` → `ethos.grounding.v1` adapter

- **Status: done — and the answer is NO ADAPTER.** `0.19.0`. No mapper, no subcommand, no foreign
  parser in the tree. The refusal is pinned by
  `crates/engine-grounding/tests/liteparse_refusal.rs`.

- **Goal:** map a foreign parser's output into the grounding shape, **if it is worth it**.

- **The standing constraint:** whatever it maps is **not** `Extracted` — this engine did not read
  those bytes, and an adapter that laundered someone else's output into the derivation class this
  repository reserves for its own reader would be the worst defect in the tree.

### "If it is worth it" is a gate, and it was measured

The slice ran the question down rather than assuming either answer. **Two of the four hazards are
real and both are properties of `ethos.grounding.v1` itself**, so no adapter could clear them by
being careful about a particular document.

**Wall 1 — the producer cannot name itself.** Checklist §8, measured from `output/json.rs:46-65`:
LiteParse *"emits `page, width, height, text, text_items` **and nothing else**"*. That is checklist
**L20**, the missing versioned output contract this repository was built to attack — so a LiteParse
artifact does not say what produced it. `ethos.grounding.v1` **requires** `producer: {name,
version}`, both non-empty.

A caller-supplied version is not a way out. `additionalProperties: false` runs the length of that
schema, so there is nowhere to record that an identity was **asserted** rather than **measured**,
and a claimed version would be indistinguishable from one the engine read. This repository's own
precedent runs the other way and says why: `VerifierBinary::identify` pins a verifier by version
**and binary digest**, because an identity that can be asserted is an identity that can disagree
with what it describes.

**Wall 2 — the boxes are loose and the schema cannot say so.** Checklist **L18**, measured
(§18.2 #5): their bbox is a union of `FPDFText_GetLooseCharBox` — em boxes, ascent-to-descent,
**not ink**. `01-CONTRACT.md` §5.3 is directly on point:

> When a box *is* emitted, the artifact says **what kind of box it is** … v0 emits measured ink
> boxes only … **If a future version emits loose boxes, it declares those separately.**

There is no field for that declaration in `ethos.grounding.v1` and no room to add one. Loose boxes
in this schema would be L18 — *"sold as precise positioning, with nothing in the output saying which
it is"* — reproduced inside this repository's own `artifact_type`. **That is worse than shipping
nothing, because the result would look like evidence.**

### What was NOT a wall, which is the more useful half of the measurement

The memo predicted the blocker would be geometry: an adapter *"would declare
`coordinate_origin: unknown` unless it also reads the source PDF"*. **That one dissolves.**

| hazard | predicted | measured |
| --- | --- | --- |
| coordinate origin | `unknown`, unless the PDF is read | **fine.** Their space is top-left, 72 DPI, `CropBox`→`MediaBox` (§18.2 #3); this engine's visible box is `/CropBox` clipped to media, media where none is declared. Same box, same origin |
| unit conversion | — | **fine.** 72 DPI is one point per unit, so points × 100 is centipoints exactly |
| floats on the wire (L22) | lossy | **survivable.** A value that will not land on an integer centipoint is an omission with a count — the honesty `project()` already uses for a node with no measurable ink box |
| producer identity | not raised | **fatal** (wall 1) |
| box semantics | raised as L18, not as an adapter blocker | **fatal** (wall 2) |

So the refusal is narrow and specific: **provenance and box semantics, not geometry.** Recording
which hazards were false is what makes the decision re-openable on evidence rather than on mood.

### Why there is no refusing subcommand

The brief allowed shipping a CLI that refuses. There is none, for two reasons.

A subcommand that can only ever exit 2 is a permanent public surface that does nothing, which is the
argument S1 used to refuse `markdown` and `html` MCP tools — *a tool that exists because it was
cheap is a surface to keep honest forever*. And to refuse **per document** it would have to parse
LiteParse JSON, of which this tree has no sample: the shape is known only as a field list in a memo.
A parser built on that guess would refuse real LiteParse output as *malformed* when the truth is
*this engine guessed your schema*, which is a worse artifact than no command.

The refusal is executable in the way this repository makes rules executable — as a guard test
against the thing the decision was made about. `liteparse_refusal.rs` asserts that `producer`
requires a non-empty name and version, that no property anywhere in the schema declares box
semantics, and that `additionalProperties: false` leaves no room to add one. Relax any of those and
the test fails, and S5 is reopened **deliberately**.

- **In:** `crates/engine-grounding/tests/liteparse_refusal.rs`; the REFUSE row and the measurement
  in `06-STEAL-REFUSE.md`; `0.19.0` and the moved profile hash; both SDK version pins; `12`/`13`;
  CHANGELOG; README; `docs/README.md`.

- **Out:** any mapper, any `adapt-liteparse` subcommand, any liteparse fixture or parser. A
  `DocumentRepresentation` on a foreign path. `capabilities.liteparse`. An `unknown` origin in the
  grounding schema — widening it is a change to the **verifier's** contract, not an adapter's
  business. A liteparse / PDFium / AGPL dependency. Docling, LlamaIndex, Haystack. MCP, LangChain,
  Python or Node surface for a mapper that does not exist. A tag.

- **Acceptance tests:**
  - [x] No `ethos.grounding.v1` is emitted on a foreign path, because there is no foreign path
  - [x] No `DocumentRepresentation` is produced from foreign bytes
  - [x] `project()` and `engine ground` are **untouched** — no diff in `engine-grounding/src`
  - [x] The refusal is pinned to the schema: `producer` requires a non-empty name and version; no
        property declares box semantics; `additionalProperties: false` on the artifact, `element`
        and `span`; `coordinate_system` admits no `unknown` origin
  - [x] No MCP / LangChain / Python / Node surface for the mapper
  - [x] No liteparse, PDFium or AGPL dependency; `cargo deny check` passes
  - [x] No confidence field anywhere (the grep gate is the standing proof)
  - [x] Workspace **0.19.0**, both SDKs **0.19.0**, profile hash
        `sha256:3ad382d0bfb4cbc53cdd74f8dae44e4807c5764452e34517ca384503fdc96f5f`
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables; the
        markdown and html goldens still green
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt

- **Depends on:** S1.

---

**v1.2 is complete at 0.19.0.** S0–S4 shipped adapters; S5 measured one and refused it. **v1 is
still not done** — the S7 table-cell gate is measured and missed at 64‰ — and no slice in this
document may be cited as evidence that it is.

---

## Standing rules for every v1.2 slice

Carried from `08-V1-SCOPE.md` §6, `10-V11-SCOPE.md` §8 and `12-V12-SCOPE.md` §8:

1. **No public confidence field** — including on a tool result or in a summary string
2. **No box, and no role, derived from a font size**
3. **No silent drop and no silent repair**
4. **No invented coordinate, identifier, fingerprint or pagination** — and an adapter inventing one
   on a model's behalf is that violation wearing a protocol
5. **A gap is never presented as a success** — a forged handle errors
6. **Byte-identity across runs**, and across adapters: what a tool returns is what the CLI prints
