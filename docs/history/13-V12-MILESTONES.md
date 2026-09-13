# 13 — v1.2 slices

**Implementation authority for v1.2.** Scope is in [`12-V12-SCOPE.md`](12-V12-SCOPE.md), and every
v1.2 PR belongs to exactly one slice.

**v1 is not done.** Its gate is measured and missed, and the chase is parked, which is not a pass.
v1.2 began because the owner asked for the next roadmap row, and nothing in it closes v1.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | The scope document and this one | done |
| **S1** | MCP over stdio: `extract`, `ground`, `node_get` | done |
| **S2** | Python SDK | done |
| **S3** | Node SDK | done |
| **S4** | LangChain tools | done |
| **S5** | An optional `liteparse` → `ethos.grounding.v1` adapter | **done — refused** |

**v1.2 is complete.** S0–S4 shipped adapters; S5 measured one and refused it.

---

## S0 — Scope and slice map

**Goal:** v1.2 exists as an ordered list of bounded changes **before** any of it is implemented, and
**the handle law is written down before the first host that can violate it.**

**Why first:** so the first adapter cannot quietly acquire a bounding-box argument "for convenience"
while nobody has written down that it may not.

## S1 — MCP over stdio

**Goal:** the first adapter, and the one that reaches every host on the list — including the
licence-failing ones — without dragging their licences toward us.

**The transport is a pipe.** Newline-delimited JSON-RPC on stdin and stdout: MCP's own stdio
transport, so it is the native format rather than a dialect invented here, and it needs no HTTP, no
server-sent events, no socket and no async runtime. **This slice stays inside the dependency ban
rather than filing an ADR to widen it**, and `cargo deny` is the standing proof.

The protocol surface is five methods, implemented as a loop over stdin with the JSON library the
workspace already had. **No MCP framework is vendored:** the ones available pull an async runtime,
which would cost the ban to save a few dozen lines.

**Three tools, and why not more:**

| Tool | Argument | Returns |
| --- | --- | --- |
| `extract` | `path` | The representation, byte-identical to the CLI's |
| `ground` | `representation` | The grounding artifact |
| `node_get` | `representation`, `node_id` | The node record from **that** artifact |

`markdown` and `html` are **not** here. They would be a few lines each and neither proves anything
this slice claims — **a tool that exists because it was cheap is a surface to keep honest forever.**

`verify` is not here either. It relays the pinned verifier, and **a relay wrapped in a second
protocol is a second place for a verdict to be re-derived.**

**The handle law, made mechanical.** `node_get` is the whole gate, in its smallest form:

1. The caller passes back the artifact `extract` **minted**.
2. The engine **re-validates** it — fingerprint first. A representation whose payload does not hash
   to its own declared digest is not a record this engine will speak for, so **a model that edited
   the JSON on the way through fails here rather than getting an answer about a document that never
   existed.**
3. The id is looked up **among that artifact's own nodes**. Found returns the node; not found is an
   error.

**A forged id fails closed** — not the nearest node, not an empty object, not null. **An empty
answer tells a model its guess was unlucky; an error tells it the guess was not admissible.**

**No tool argument carries geometry**, in any of the three schemas. A test reads the schemas the
server advertises and fails on those field names, so the rule is enforced **against the wire rather
than against a reviewer's memory.**

**Locators live in the structured output; the text a model reads carries counts.** That split is the
difference between a locator a pipeline can bind and a locator a model can edit, and a test asserts
no coordinate reaches the text side.

**Statelessness is not a limitation here.** Every call is self-contained and the server keeps no
document cache, so **passing the artifact back is the session.** That also means the handle law has
no back door: there is no server-side table of documents whose keys a model could enumerate.
**Amended 2026-09-13 by [decision 24](../00-NORTH-STAR.md):** the server now remembers the SHA-256
of path-form bytes that already verified, so a repeat call skips re-verification. It keeps no
document, and no argument or reply can name a digest.

## S2 — Python SDK

**Goal:** a thin Python surface that cannot diverge from what the CLI prints.

**It wraps the CLI, and that is the whole design.** `extract` and `ground` run the subcommands a
shell would run and hand back the bytes those subcommands printed. There is no second serialization
anywhere in the package, so **byte-identity with the CLI is a tautology rather than a promise** — a
test re-canonicalizes what `extract` returned and compares it byte for byte against the CLI's
stdout.

**A native extension was refused.** It would reach the library by a **second path**, which is a
second thing that can disagree with the first — plus a wheel matrix, a fifth build surface, and a
route by which a Rust dependency could arrive on the Python side of the fence. The cost of the
chosen shape is one process spawn per call, which nobody has measured a need to avoid.

The same reasoning refuses an MCP client here: **MCP is a process and this is a library. They are
two callers of one binary, not layers.**

**`ground` takes the artifact `extract` returned.** It does not take a quote and it does not take a
page, because the CLI subcommand takes neither — **a locator-shaped argument would be the corollary
violation in its purest form.**

**`node_get` has no subcommand behind it** and this slice did not add one. MCP already carries the
tool; a third CLI verb would exist for symmetry, which is not a reason. So its checks are **ported**,
in the order the MCP server runs them: the value is a representation; the payload is re-canonicalized
and re-hashed and must match, **before any lookup happens**; the id is looked up among that
artifact's own nodes.

That is why the package carries a canonical-JSON port, running the Rust implementation's **own
parity vectors**. **A fingerprint that were merely *nearly* the engine's would be worse than none** —
it would accept an artifact the engine refuses, or refuse one the engine minted, and either way a
caller would be told something false about a document. The load-bearing proof is not the vectors but
the whole artifact: extract's output, re-canonicalized in Python, reproduces the CLI's bytes exactly,
**which means the port agrees with the Rust on real input and not merely on five hand-written
values.**

**`ground` deliberately does not repeat the fingerprint check in Python.** The engine runs it and the
SDK surfaces the engine's own refusal with its stderr intact: **a second check here is a second thing
that could drift from the first.**

**What is deliberately absent.** `markdown` and `html` prove nothing this slice claims. `verify` is
absent for a stronger reason: it relays the pinned verifier, and **a Python function of that name
would look like this package had an opinion about whether a claim is supported.**

## S3 — Node SDK

**Goal:** the same surface for Node, on the same terms.

**This is not a second design.** Python is the contract. If Node disagreed with it about a signature,
an error name, or what `ground` accepts, **Node would be the one that is wrong** — so the differences
are exactly the two the languages force: the function name's casing, and classes that throw rather
than exceptions that raise. A test pins both SDKs and the package manifest to the workspace version,
**because two adapters at different versions over one binary is exactly the disagreement a version
string exists to make legible.**

**No native addon**, for the reason Python refused one, plus a prebuild matrix across platforms and
ABI versions. **No TypeScript, no bundler, no test framework** — the runtime has a test module built
in, and generated type definitions would be a build step this package does not have. Runtime
dependencies are empty, asserted from the manifest and again by reading every import specifier in
the sources.

**When handed an object, `ground` writes canonical bytes to a temp file** rather than the language's
own stringifier: **a second serialization is the one thing this package exists not to have.**

**Canonical JSON, ported a second time — and two hazards specific to this language:**

- **Key order.** The default sort compares UTF-16 code units, which disagrees with Rust above the
  BMP. The port sorts by code point explicitly, and a test asserts the default sort would have got a
  particular pair wrong.
- **Lone surrogates.** The runtime's encoder turns one into a replacement character, which is **a
  silent repair of evidence**. The port throws instead, matching what Rust's type system makes
  impossible.

**One divergence is real and written down rather than papered over.** JavaScript has a single number
type, so float-*shaped* text like `1.0` cannot be told from `1`; Rust and Python reject it and this
port cannot. What all three reject identically is a value that genuinely is not a whole number, which
is the property canonical JSON needs. **The unreachable half is unreachable in practice** — the
engine never prints `1.0`, because the CLI's stdout *is* canonical bytes. A test pins the divergence
so it stays a named property rather than a surprise.

**The parameter-name ban is read off the function's own source text**, because the language's
arity property says nothing about names — and the test asserts the names it recovered, **so a parse
that returned nothing could not make the ban vacuous.**

## S4 — LangChain tools

**Goal:** a callable tool for LangChain in both languages, running unchanged inside a graph.

**The split is the whole slice.** MCP already implemented the rule that locators travel in the
artifact and never in the prose; LangChain implements the same sentence in its own types, so the
mapping is a translation and not a design. The framework's content-and-artifact response format is
what makes both sides real — **without it the artifact is stringified into the content, and a box in
the content is a locator a model can edit and then cite.** Both optional dependency pins are bounded
on **both** sides for that one reason: a major bump is where the contract could change, and **it
should break loudly rather than silently downgrade the split.**

**MCP is the oracle for the summary strings, not this slice's opinion.** The suites compare them
byte-for-byte against what the MCP server emits, on one fixture where nothing is omitted and one
where everything is. That is what stops a second adapter inventing a richer sentence than the first —
and it is also the proof that the omitted count, computed out here as nodes minus elements, equals
the engine's own count. **Counting geometry rows instead would re-encode the groundability rule in
two more languages, and a count derived from a different question than the one being asked is a count
that goes wrong the first time a second absence variant appears.**

**The `node_get` summary names the kind and never the id**: a kind is a category, an id is a handle,
**and a handle in the one channel a model can rewrite is the hazard itself.** The kind is spelled the
way the *artifact* spells it, not the way the server prints its internal debug form — reshaping it
would be the adapter inventing a name for a thing it did not read.

**One wire shape, checked against the wire.** The three argument schemas are plain JSON Schema,
verbatim from what the MCP server advertises, and a test in each language asserts them against the
live listing rather than against a reviewer's memory. The argument name keeps MCP's spelling in both
languages even where the Node function parameter differs: **the tool argument is the wire, and one
wire has one name.**

**The install stays empty.** LangChain is an optional extra in Python and an optional peer in Node,
so the default import still reaches nothing but the standard library — asserted by reading the import
graph of the **default entry point only**. Importing the subpath without it is a **named** failure
carrying the install command, and in Node that means a dynamic import inside a try/catch at module
load, so it fails exactly where Python's import error does rather than as a generic
module-not-found.

**No graph adapter and no trust state.** A bindable tool is already what a graph takes, so a node or
a checkpointer here would be a second surface that proves nothing. No result, summary or annotation
says grounded, verified, an evidence tier or a score, and there is no `verify` tool. A failure
**raises**, which is the framework's currency for the server's error flag — **nothing catches what
the SDK raises, because an empty result would tell a model its guess was merely unlucky.**

**A false green this slice found and closed.** Both SDK suites preferred a release build over a debug
one and took the first file that existed. On a tree with a stale release build that was a much older
binary — **and every earlier assertion passed against it**, because the byte-identity checks compare
the SDK against the CLI using the same binary and are self-consistent whichever one it is. **They
proved what they claim, about the wrong engine.** Both locators now read the binary's version and
refuse one that is not this workspace's. Measured, not feared: it is what tripped this slice's first
MCP call.

## S5 — The liteparse adapter, measured and refused

**The answer is no adapter.** No mapper, no subcommand, no foreign parser in the tree. The refusal is
pinned by a test.

**The standing constraint:** whatever such an adapter mapped would **not** be `Extracted` — this
engine did not read those bytes, and **an adapter that laundered someone else's output into the
derivation class this repository reserves for its own reader would be the worst defect in the tree.**

**"If it is worth it" is a gate, and it was measured.** Two of the four hazards are real, and **both
are properties of the grounding schema itself**, so no adapter could clear them by being careful
about a particular document.

**Wall 1 — the producer cannot name itself.** Their output emits page, width, height, text and text
items **and nothing else**, so it does not say what produced it. The schema **requires** a producer
name and version, both non-empty.

A caller-supplied version is not a way out. `additionalProperties: false` runs the length of that
schema, so **there is nowhere to record that an identity was asserted rather than measured**, and a
claimed version would be indistinguishable from one the engine read. This repository's own precedent
runs the other way and says why: a verifier is pinned by version **and binary digest**, because **an
identity that can be asserted is an identity that can disagree with what it describes.**

**Wall 2 — the boxes are loose and the schema cannot say so.** Their boxes are em boxes, ascent to
descent, **not ink** — and the contract requires an emitted box to declare its kind. There is no
field for that declaration and no room to add one. **Loose boxes in this schema would reproduce that
defect inside this repository's own artifact type, which is worse than shipping nothing, because the
result would look like evidence.**

**What was *not* a wall, which is the more useful half of the measurement.** The prediction was that
geometry would block it — an unknown coordinate origin unless the adapter also read the PDF. **That
dissolves:**

| Hazard | Predicted | Measured |
| --- | --- | --- |
| Coordinate origin | Unknown, unless the PDF is read | **Fine.** Their space and this engine's visible box are the same box with the same origin |
| Unit conversion | — | **Fine.** Their DPI makes the conversion exact |
| Floats on the wire | Lossy | **Survivable.** A value that will not land on an integer centipoint is an omission with a count — the honesty the projection already uses |
| Producer identity | Not raised | **Fatal** |
| Box semantics | Raised, but not as an adapter blocker | **Fatal** |

So the refusal is narrow and specific: **provenance and box semantics, not geometry.** Recording
which hazards were false is what makes the decision re-openable on evidence rather than on mood.

**Why there is no refusing subcommand.** The brief allowed shipping a CLI that refuses. There is
none, for two reasons. A subcommand that can only ever exit 2 is a permanent public surface that does
nothing — the same argument that refused the extra MCP tools. And to refuse *per document* it would
have to parse their JSON, **of which this tree has no sample**: the shape is known only as a field
list. A parser built on that guess would refuse real output as *malformed* when the truth is *this
engine guessed your schema*, **which is a worse artifact than no command.**

The refusal is executable the way this repository makes rules executable — as a guard test against
the thing the decision was made about. It asserts that the producer field requires a non-empty name
and version, that no property anywhere declares box semantics, and that the schema leaves no room to
add one. **Relax any of those and the test fails, and this slice is reopened deliberately.**

---

## Standing rules for every v1.2 slice

1. **No public confidence field** — including on a tool result or in a summary string.
2. **No box, and no role, derived from a font size.**
3. **No silent drop and no silent repair.**
4. **No invented coordinate, identifier, fingerprint or page number** — and an adapter inventing one
   on a model's behalf is that violation wearing a protocol.
5. **A gap is never presented as a success** — a forged handle errors.
6. **Byte identity across runs, and across adapters** — what a tool returns is what the CLI prints.
