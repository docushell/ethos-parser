# 12 — v1.2 scope: adoption

**Status:** scope authority for v1.2 · **Slice detail:** `13-V12-MILESTONES.md`
**This is the code-review map for v1.2.** Every v1.2 PR belongs to exactly one slice.

**v1 is not done.** Its table number is measured and **missed at 64‰**; the **> 0.489 chase is
parked** rather than passed (`00-NORTH-STAR.md` #10)
(`table-gate-v1.md`, `09-V1-MILESTONES.md` S7). **v1.1 is complete** at 0.14.1. v1.2 is the next
row of `02-ROADMAP.md` and it started because the owner asked for it, **not** because the gate
cleared. Nothing in this document closes v1, and no slice here may be cited as evidence that it
did.

---

## 1. The one sentence

**A locator survives every adapter round-trip, or the adapter does not ship.**

Everything below is machinery for that sentence.

## 2. Why this version exists — and why its first host is also its worst hazard

v0 through v1.1 built the record and its projections. Nobody reaches them: the audience for
`ethos.grounding.v1` is a pipeline, and pipelines arrive through hosts. `02-ROADMAP.md`'s v1.2 row
is **adoption**, and the parser expansion memo §16.7 ranks the hosts. Its verdict is unambiguous:

> **MCP first, and it is not close.** It is the only host whose native return type carries a
> locator under an *enforced* schema (`outputSchema` + `structuredContent`); every other host on
> the list is effectively `Dict[str, Any]`.

And in the same breath, the reason it could be the worst choice instead of the best:

> **Its one real hazard, stated plainly:** MCP tools are model-controlled — the model chooses the
> arguments. If any tool accepts a locator as a free-text argument that the engine then trusts, the
> model has become the citation authority in a single step.

That is not a hazard a system prompt fixes. A model that can type `{"page": 3, "bbox": [10, 10, 90,
40]}` into a tool the engine believes has *become* the thing this repository exists to prevent —
and it will look exactly like a citation, because it is shaped like one.

## 3. The handle law

The memo states the mitigation in one sentence, and it is structural rather than advisory:

> The mitigation is structural: **the engine mints every locator, returns it as an opaque handle,
> and re-validates it on the way back in.** Get that wrong and MCP is the worst option on the list
> rather than the best.

Three obligations, and an adapter satisfies all three or it does not ship:

| | obligation | what it forbids |
| --- | --- | --- |
| **Mint** | every locator a caller ever sees was produced by the engine, inside an artifact it already emits | an adapter computing a page, a box or a cell of its own |
| **Opaque** | a locator travels back as the bytes the engine handed out — a node id string, copied | a caller *composing* a locator, or an adapter documenting its shape as an input format |
| **Re-validate** | on the way in, the engine checks the handle against **that** artifact, and a handle it did not mint **fails closed** | returning a best guess, the nearest node, or an empty result that reads like "no match" |

**Fails closed means an error, not an empty answer.** A tool that returns `{}` for a forged
locator has told the model its guess was merely unlucky. A tool that errors has told it the guess
was not admissible.

### The corollary about tool arguments

**No tool argument schema may contain document geometry.** No `page`, no `bbox`, no `x`/`y`/`w`/`h`,
no row/column pair. Not as optional fields, not "for convenience", not behind a flag. If a caller
needs a node, it passes the id the engine minted; if it needs a region, that slice has not shipped.

This is checkable rather than aspirational, and v1.2-S1 checks it: a test reads the tool schemas
the server advertises and fails on those field names.

### The corollary about where locators travel

Locators live in the **artifact**, never in the prose a model reads and edits. In MCP that is
`structuredContent`; in a LangChain tool it is `artifact` and not `content` (§16.7, and the memo's
LangChain row says so directly). A summary a human or a model reads may carry counts. It may not
carry a box.

Both halves have shipped: v1.2-S1 in `structuredContent`, v1.2-S4 in
`response_format="content_and_artifact"`. The second is checked against the first — the LangChain
tools' `content` is compared **byte-for-byte** with what `engine mcp` emits — so the two adapters
cannot drift into two different sentences about the same document.

## 4. What v1.2 is

| | |
| --- | --- |
| **Adapters, not features** | every slice wraps stages that already exist and shipped under v0–v1.1 |
| **Thin over the library** | the CLI is a shell over `engine-core` / `engine-pdf` / `engine-grounding`, and so is every adapter, so they cannot diverge |
| **Locator-safe by construction** | §3's three obligations, enforced by the type and by tests rather than by documentation |
| **Bounded by `deny.toml`** | the network/TLS/async surface stays banned; an adapter that needs it does not ship in this version |

## 5. What v1.2 is not

- **Not a new parse feature.** No detector moves, no rule id moves, no capability flips because an
  adapter arrived. An adapter that changes what a document *says* is not an adapter.
- **Not a network surface.** `deny.toml` bans `tokio`, `hyper`, `reqwest`, `ureq`, `rustls` and the
  rest of the reachable HTTP/TLS surface, and v1.2 does not file an ADR to undo that. MCP over
  **stdio** is a pipe, not a socket.
- **Not a verifier, still.** `07-VERIFY-BOUNDARY.md` is unchanged. No adapter emits `grounded`, an
  evidence tier, or a verdict of its own; `verify` relays the pinned Ethos CLI's bytes or it does
  not exist in that adapter.
- **Not a session.** MCP's July 2026 statelessness change suits a deterministic engine exactly:
  every call is self-contained, and passing the artifact back **is** the session. No process-global
  document cache makes call 2 depend on call 1.
- **Not permission to reopen v1 or v1.1.** `ruled-rects-v2`, `stroke-ruled-v1`, `unruled-align-v1`,
  `markdown-blocks-v2` and `html-blocks-v2` keep their ids and their numbers.
- **Not a platform.** RAGFlow, Dify, n8n and Windmill are refused in §16.7 on licence or structure.
  One MCP server reaches all of them without dragging their licences toward us; **that is the whole
  answer**, and it is why this version's first slice is worth more than four integrations.

## 6. Slices

| Slice | Theme | State |
| --- | --- | --- |
| **v1.2-S0** | This document and `13-V12-MILESTONES.md` | **done** |
| **v1.2-S1** | MCP over stdio: `extract`, `ground`, `node_get`, with the handle law enforced | **done** |
| **v1.2-S2** | Python SDK — thin, over the same library or CLI | **done** |
| **v1.2-S3** | Node SDK | **done** |
| **v1.2-S4** | LangChain tool, locators in `artifact` and never in `content` | **done** |
| **v1.2-S5** | Optional `liteparse` → `ethos.grounding.v1` adapter | **done — refused** |

**v1.2 is complete at 0.19.0.** S5's *"if it is worth it"* was measured and the answer is no
adapter: LiteParse's output cannot name its own producer and its boxes are loose em boxes this
schema cannot declare. Both walls are in `ethos.grounding.v1` itself, so no adapter could clear them
per document. `13-V12-MILESTONES.md` S5 and `06-STEAL-REFUSE.md` carry the measurement;
`engine-grounding/tests/liteparse_refusal.rs` pins it.

## 7. Identity

An adapter does not change what a document says, so **no adapter adds a profile field**:

- **No `capabilities.mcp`, no `capabilities.python`, no `capabilities.node`, no
  `capabilities.langchain`, no `capabilities.liteparse`.** A capability describes what this profile can read out of a
  *document*; a transport does not, and neither does a language binding or a framework binding.
  Adding one would put a flag on every artifact that no consumer can act on, and
  `01-CONTRACT.md`'s capability discipline exists to stop exactly that. If a caller ever must see
  an adapter on the wire, that is a decision with a proof test behind it, not a default.
- **No new rule id.** `markdown_rule`, `html_rule` and the three table rules are untouched.
- Workspace `0.14.1` → **`0.15.0`** (S1) → **`0.16.0`** (S2) → **`0.17.0`** (S3) → **`0.18.0`**
  (S4) → **`0.19.0`** (S5), and the profile hash moves **with the version alone** every time — the same shape v1-S7b
  (0.9.0) had, where nothing but `parser_version` moved and the hash moved anyway, because a
  version that claimed otherwise is the one lie that field cannot afford.

| version | slice | what moved |
| --- | --- | --- |
| `0.15.0` | S1, MCP over stdio | `parser_version` |
| `0.16.0` | S2, Python SDK | `parser_version` |
| `0.17.0` | S3, Node SDK | `parser_version` |
| `0.18.0` | S4, LangChain tools | `parser_version` |
| `0.19.0` | S5, the liteparse adapter, refused | `parser_version` |

## 8. Standing rules, carried forward

Unchanged from `08-V1-SCOPE.md` §6 and `10-V11-SCOPE.md` §8, and repeated because an adapter is
where they get bent:

1. **No public confidence field.** Not on a tool result, not in a summary string
2. **No box — and no role — derived from a font size**
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count
4. **No invented coordinate, identifier, fingerprint or pagination** — and an adapter inventing one
   on a model's behalf is the same violation wearing a protocol
5. **A gap is never presented as a success** — a forged handle errors
6. **Byte-identity across runs** — an artifact returned through an adapter is the artifact the CLI
   prints, byte for byte, and v1.2-S1 asserts it
