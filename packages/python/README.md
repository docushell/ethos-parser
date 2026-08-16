# ethos-engine — Python SDK

A thin Python surface over the `engine` CLI (**v1.2-S2**). Three functions, stdlib only, and
**not on PyPI** — this slice does not publish it.

```python
import ethos_engine

representation = ethos_engine.extract("contract.pdf")
grounding = ethos_engine.ground(representation)

node_id = representation["representation"]["nodes"][0]["id"]   # minted by the engine
node = ethos_engine.node_get(representation, node_id)
```

| function | shells out to | returns |
| --- | --- | --- |
| `extract(pdf_path)` | `engine extract <path>` | `DocumentRepresentation v0` |
| `ground(representation)` | `engine ground <path>` | `ethos.grounding.v1` |
| `node_get(representation, node_id)` | nothing — the checks are ported | that artifact's node record |

## It cannot diverge from what the CLI prints

`extract` and `ground` run the subcommands a shell would run and hand back the bytes those
subcommands printed, parsed as JSON. There is no second serialization anywhere in this package,
so there is nowhere for an artifact to change: `tests/test_cli_surface.py` re-canonicalizes what
`extract` returned and compares it byte for byte against the CLI's stdout.

That is also why there is no native extension. PyO3 would reach the library by a second path,
which is a second thing that can disagree with the first — plus a wheel matrix and a fifth build
surface, for a saving nobody has measured a need for.

## The handle law

`docs/12-V12-SCOPE.md` §3, unchanged from the MCP adapter: **the engine mints every locator,
returns it as an opaque handle, and re-validates it on the way back in.**

- **A locator is returned, never accepted as prose.** No function here takes a page, a box, an
  `x`/`y`, a width, a height or a row/column pair — not optionally, not keyword-only, not behind
  a flag. A test reads the signatures and fails on those names.
- **A handle travels back as the bytes that were handed out.** `node_get` takes a node id string
  copied out of a representation the engine returned.
- **A handle the engine did not mint fails closed.** `node_get` raises `NodeNotFound`, never
  `None` and never `{}`. An edited artifact raises `FingerprintMismatch` *before* any lookup
  happens, because its payload no longer hashes to its own declared digest.

`node_get` is the one function with no subcommand behind it — `engine node-get` does not exist
and this slice does not add it, since MCP already carries the tool. So its checks are ported:
`src/ethos_engine/_c14n.py` is c14n v1 in Python, running the same parity vectors
`crates/engine-core/src/c14n.rs` runs.

## What is not here

`markdown` and `html` exist on the CLI and are not wrapped: neither proves anything this slice
claims, and a function that exists because it was cheap is a surface to keep honest forever.
`verify` is absent for a stronger reason — it relays the pinned Ethos CLI, and
`docs/07-VERIFY-BOUNDARY.md` is the boundary a host's convenience must not bend. There is no MCP
client here either: MCP is a process, this is a library, and they are two callers of one binary
rather than layers.

## Installing and running

The `engine` binary is a prerequisite; nothing here downloads or vendors one.

```bash
cargo build --release --locked                 # from the repository root
pip install -e 'packages/python[dev]'
ETHOS_ENGINE=target/release/engine pytest packages/python
```

`ETHOS_ENGINE` is checked **first and authoritatively** — a path named there and not present is
an error, not a reason to go looking for some other build — then `engine` on `PATH`. That is the
precedent `VerifierBinary::resolve` sets for `ETHOS_BIN`, and the reason is the same: resolving
to a binary nobody chose means returning artifacts from a parser nobody chose.
