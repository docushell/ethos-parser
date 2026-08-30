# 12 — v1.2 scope: adoption

**Scope authority for v1.2.** Slice detail is in [`13-V12-MILESTONES.md`](13-V12-MILESTONES.md), and
every v1.2 PR belongs to exactly one slice.

**v1 is not done** — its table number is measured and missed. v1.2 is the next roadmap row and it
started because the owner asked for it, **not** because that gate cleared. Nothing here closes v1.

---

## 1. The one sentence

**A locator survives every adapter round trip, or the adapter does not ship.**

Everything below is machinery for that sentence.

## 2. Why this version exists — and why its first host is also its worst hazard

v0 through v1.1 built the record and its projections, and nobody reaches them. The audience for a
grounding artifact is a pipeline, and pipelines arrive through hosts.

**MCP first, and it is not close.** It is the only host whose native return type carries a locator
under an *enforced* schema; every other host on the list is effectively an untyped dictionary.

And in the same breath, the reason it could be the worst choice instead of the best: **MCP tools are
model-controlled — the model chooses the arguments.** If any tool accepts a locator as a free-text
argument the engine then trusts, the model has become the citation authority in a single step.

That is not a hazard a system prompt fixes. A model that can type `{"page": 3, "bbox": [10,10,90,40]}`
into a tool the engine believes has *become* the thing this repository exists to prevent — and it
will look exactly like a citation, because it is shaped like one.

## 3. The handle law

The mitigation is structural rather than advisory: **the engine mints every locator, returns it as an
opaque handle, and re-validates it on the way back in.**

Three obligations. An adapter satisfies all three or it does not ship.

| | Obligation | What it forbids |
| --- | --- | --- |
| **Mint** | Every locator a caller sees was produced by the engine, inside an artifact it already emits | An adapter computing a page, a box or a cell of its own |
| **Opaque** | A locator travels back as the bytes the engine handed out — a node id string, copied | A caller *composing* a locator, or an adapter documenting its shape as an input format |
| **Re-validate** | On the way in, the engine checks the handle against **that** artifact, and one it did not mint **fails closed** | Returning a best guess, the nearest node, or an empty result that reads like "no match" |

**Fails closed means an error, not an empty answer.** A tool that returns `{}` for a forged locator
has told the model its guess was merely unlucky. A tool that errors has told it the guess was not
admissible.

### No tool argument may name geometry

No `page`, no `bbox`, no `x`/`y`/`w`/`h`, no row and column pair. Not as optional fields, not "for
convenience", not behind a flag. If a caller needs a node it passes the id the engine minted; if it
needs a region, that slice has not shipped.

This is checkable rather than aspirational: a test reads the tool schemas the server advertises and
fails on those field names.

### Locators travel in the artifact, never in the prose

A summary a model reads may carry counts. It may not carry a box. In MCP that means
`structuredContent`; in a LangChain tool it means the artifact rather than the content string.

Both halves shipped, and the second is checked against the first — the LangChain tools' content is
compared **byte for byte** against what the MCP server emits, so the two adapters cannot drift into
two different sentences about the same document.

## 4. What v1.2 is

| | |
| --- | --- |
| **Adapters, not features** | Every slice wraps stages that already exist |
| **Thin over the library** | The CLI is a shell over the library, and so is every adapter, so they cannot diverge |
| **Locator-safe by construction** | §3's three obligations, enforced by types and tests rather than by documentation |
| **Bounded by the dependency policy** | The network, TLS and async surface stays banned; an adapter that needs it does not ship in this version |

## 5. What v1.2 is not

- **Not a new parse feature.** No detector moves, no rule id moves, and no capability flips because
  an adapter arrived. **An adapter that changes what a document *says* is not an adapter.**
- **Not a network surface.** The dependency policy bans the reachable HTTP and TLS surface, and v1.2
  does not file an ADR to undo that. MCP over **stdio** is a pipe, not a socket.
- **Not a verifier, still.** No adapter emits `grounded`, an evidence tier, or a verdict of its own.
- **Not a session.** Every call is self-contained, and passing the artifact back **is** the session.
  No process-global document cache makes call 2 depend on call 1.
- **Not permission to reopen v1 or v1.1.** Every detection and projection rule keeps its id and its
  numbers.
- **Not a platform.** One MCP server reaches every platform on the list without dragging their
  licences toward us. **That is the whole answer**, and it is why this version's first slice is worth
  more than four separate integrations.

## 6. Slices

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and the milestones | done |
| **S1** | MCP over stdio: `extract`, `ground`, `node_get`, with the handle law enforced | done |
| **S2** | Python SDK | done |
| **S3** | Node SDK | done |
| **S4** | LangChain tools, locators in the artifact and never in the content | done |
| **S5** | An optional `liteparse` → `ethos.grounding.v1` adapter | **done — refused** |

**v1.2 is complete.** S5's *"if it is worth it"* was measured, and the answer is no adapter: that
parser's output cannot name its own producer, and its boxes are loose em boxes this schema cannot
declare. Both walls are in `ethos.grounding.v1` itself, so no adapter could clear them per document.
[`06-STEAL-REFUSE.md`](../06-STEAL-REFUSE.md) carries the measurement, and a test pins the refusal.

## 7. Identity

An adapter does not change what a document says, so **no adapter adds a profile field**.

- **No capability flag for a transport or a language binding.** A capability describes what this
  profile can read out of a *document*. Adding one would put a flag on every artifact that no
  consumer can act on. If a caller ever must see an adapter on the wire, that is a decision with a
  proof test behind it, not a default.
- **No new rule id.** The projection and table rules are untouched.
- **The profile hash still moves on every release**, on `parser_version` alone. That is the same
  shape an earlier slice had where nothing else moved either — because **a version that claimed
  otherwise is the one lie that field cannot afford.**

## 8. Standing rules, carried forward

Repeated because an adapter is where they get bent:

1. **No public confidence field.** Not on a tool result, not in a summary string.
2. **No box, and no role, derived from a font size.**
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count.
4. **No invented coordinate, identifier, fingerprint or page number** — and an adapter inventing one
   on a model's behalf is the same violation wearing a protocol.
5. **A gap is never presented as a success** — a forged handle errors.
6. **Byte identity across runs** — an artifact returned through an adapter is the artifact the CLI
   prints, byte for byte, and a test asserts it.
