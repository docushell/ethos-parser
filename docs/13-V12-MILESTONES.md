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
| **S2** | Python SDK — thin, over the same library or CLI | S1 | **not started** |
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

## S2 — Python SDK — **not started**

- **Goal:** a thin Python surface over the same library or CLI, so it cannot diverge from what the
  CLI prints.
- **The standing constraint:** §3's handle law. A locator is returned, never accepted as prose.
- **Not started.** Starts when the owner asks.

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
