# 13 — v1.2 slices

**Status:** implementation authority for v1.2 · **Scope document:** `12-V12-SCOPE.md`
**This is the code-review map for v1.2.** Every v1.2 PR belongs to exactly one slice.

**v1 is not done.** S7's gate is measured and **missed at 64‰** against a 489‰ floor
(`09-V1-MILESTONES.md` S7, `table-gate-v1.md`). **v1.1 is complete** at 0.14.1. v1.2 began because
the owner asked for the next roadmap row, and nothing in it closes v1.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v1.2 scope + this document | — | **done** |
| **S1** | MCP over stdio: `extract`, `ground`, `node_get` | S0 | **done** |
| **S2** | Python SDK — thin, over the same library or CLI | S1 | **done** |
| **S3** | Node SDK | S1 | **not started** |
| **S4** | LangChain tool, locators in `artifact` never `content` | S2, S3 | **not started** |
| **S5** | Optional `liteparse` → `ethos.grounding.v1` adapter | S1 | **not started** |

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

## S3 — Node SDK — **not started**

- **Goal:** the same surface for Node, on the same terms.
- **Not started.** Starts when the owner asks.

---

## S4 — LangChain tool — **not started**

- **Goal:** a callable tool for LangChain (Python and JS), running unchanged inside LangGraph.
- **The standing constraint:** §16.7's LangChain row — **locators travel in the tool `artifact`,
  never in `content`**, and the tool must not set trust state.
- **Not started.** Starts when the owner asks.

---

## S5 — optional `liteparse` → `ethos.grounding.v1` adapter — **not started**

- **Goal:** map a foreign parser's output into the grounding shape, if it is worth it.
- **The standing constraint:** whatever it maps is **not** `Extracted` — this engine did not read
  those bytes, and an adapter that laundered someone else's output into the derivation class this
  repository reserves for its own reader would be the worst defect in the tree.
- **Not started.** Starts when the owner asks.

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
