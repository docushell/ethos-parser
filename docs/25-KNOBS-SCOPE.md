# 25 — Five knobs: password, page sets, `continue_on_page_error`, batch, colour — measured

**Scope authority for the local plan's item 6.9.** Five knobs were named in one row and never
scoped. This document measures each, says what the tree already does, and puts a recommendation and
its reopening condition beside each, so the owner decides from numbers rather than from the row.

**No roadmap row is created here.** The plan files 6.9 under "v2.3", which is not a roadmap version:
creating one is decision D4 in [`OPEN-WORK.md`](OPEN-WORK.md) §4, and it has not been taken.
[`02-ROADMAP.md`](02-ROADMAP.md) says no new version numbers get invented, and nothing below needs
one. Every recommendation holds whichever way D4 goes.

**Recorded 2026-09-17.** The owner took D4, so v2.3 is a roadmap version (North Star row #28), and
accepted §8's proposals as recommended (row #31). The body stands as written.

**Measured 2026-09-16 against `main` at `b4b4aa9` (0.58.0, unreleased)** with the verified 0.58.0
`aarch64-apple-darwin` binary and qpdf 12.3.2, over 297 PDFs in five corpora, 2,370 pages. The
instruments, the corpus list, the binary's digest and what was not measured are in
[`measurements/knobs/`](measurements/knobs/README.md). Every number below is from a result file
there.

---

## 1. The one sentence

**Of the five, one is already half-open without saying so, one is a shell loop, and three have no
document in any corpus this repository can reach that would exercise them.**

## 2. The corpora, and what the engine does with them today

| corpus | PDFs | pages | `extract` exit 0 | exit 2 | pages failed inside an artifact |
| --- | ---: | ---: | ---: | ---: | ---: |
| `fixtures/gate` | 8 | 1,466 | 8 | 0 | 0 |
| `fixtures/engine` | 44 | 44 | 43 | 1 | 0 |
| `ethos-oracle/fixtures` | 35 | 52 + 1 unreadable | 32 | 3 | 0 |
| `gate-zero/corpus` | 10 | 608 | 10 | 0 | 0 |
| opendataloader-bench | 200 | 200 | 200 | 0 | 0 |
| **all** | **297** | **2,370** | **293** | **4** | **0** |

The four refusals are `tagged-cycle` (a `/K` cycle in the structure tree), `corrupt-header-valid`
(a cross-reference table that will not parse), `invalid-header` (no signature at byte 0) and
`password-protected` (encrypted). Each is refused before or above the page loop; none is a page
failing inside a document that otherwise reads. Every artifact's terminal state is `complete`.

## 3. Password

### 3.1 What exists

An encrypted document is refused by name. `Document::open_bytes` asks `lopdf` `is_encrypted()`
after the load ([`document.rs:129`](../crates/ethos-parser-pdf/src/document.rs)) and returns
`EngineError::Encrypted`, its own variant ([`error.rs:61`](../crates/ethos-parser-core/src/error.rs)),
code `encrypted`, `RefusalCode::Encrypted` ([`assurance.rs:986`](../crates/ethos-parser-core/src/assurance.rs)),
exit 2 from `classify` and `extract` ([`lib.rs:117`](../crates/ethos-parser-pdf/src/lib.rs));
`lopdf`'s `Decryption` and `InvalidPassword` map to the same variant (`document.rs:267`). That is
[`01-CONTRACT.md`](01-CONTRACT.md) §8's row — a distinct exit code, never the signal for
"complex" — and the L16 defect [`06-STEAL-REFUSE.md`](06-STEAL-REFUSE.md) records in a competitor.

The load is `lopdf::Document::load_mem` (`document.rs:112`). `lopdf` 0.44.0 carries
`LoadOptions::with_password`, `authenticate_*` and `decrypt`; the engine calls none. **It also tries
the empty password on its own**: on a document whose user password is empty, the reader
authenticates, decrypts every object and removes `/Encrypt` from the trailer before the engine's
check runs (`lopdf-0.44.0/src/reader.rs`, `load_encrypted_document`). `Document::was_encrypted()`
would say so; nothing reads it. Nothing on MCP or the SDKs names a password: `extract` over MCP
takes `path` alone ([`mcp.rs:328`](../crates/ethos-parser-cli/src/mcp.rs)), and both SDKs' `extract`
take one path ([`__init__.py:219`](../packages/python/src/ethos_parser/__init__.py),
[`index.js:130`](../packages/node/src/index.js)). The office readers refuse an encrypted part by
name (`lib.rs:812`, `epub.rs:293`) and are outside this knob.

### 3.2 Measured

| | count | of |
| --- | ---: | ---: |
| documents `qpdf --is-encrypted` calls encrypted | **1** | 297 |
| of those, opened by the empty user password (`--requires-password` exit 3) | **0** | 1 |
| of those, a secret required (exit 0) | 1 | 1 |
| engine refusals with code `encrypted`, `classify` and `extract` | 1 and 1 | 297 |

The one is `failure/password-protected` — AESv3, `R = 6`, made by the Ethos maintainers with qpdf
for exactly this test. The engine and qpdf agree on it and disagree on nothing. Because the corpora
hold no empty-user-password document, the probe made five from `synthetic/simple-text` with qpdf
([`results/probes.txt`](measurements/knobs/results/probes.txt)):

| variant | `--requires-password` | `classify` / `extract` | artifact |
| --- | --- | --- | --- |
| owner password only, AES-256 (`R = 6`) | 3: the empty password opens it | 0 / 0 | 1 node, `Hello Ethos`, `complete`; **no limitation mentions encryption**; differs from the unencrypted artifact only in `source.sha256` and the c14n digest |
| owner only, RC4-128 (`R = 3`) | 3 | 0 / 0 | the same |
| owner only, RC4-40 (`R = 2`) | 3 | 0 / 0 | the same |
| user password `secret`, AES-256 | 0: a secret is required | 2 / 2, `[encrypted]` | none |
| user password `secret`, RC4-128 | 0 | 2 / 2, `[encrypted]` | none |

**The only case a password knob could open without a secret is open today, silently.** The knob
would open exactly the other case, and here that case is one fixture written to be refused.

### 3.3 Recommendation

**Defer the knob. Declare the case that is already open.**

- The knob is one `LoadOptions::with_password` call at `document.rs:112`, outside `profile_sha256`:
  a password changes nothing once the document is open, and the probe shows node, geometry and
  page records identical to the unencrypted file's. What it costs is a secret on a surface —
  visible to `ps` on argv, logged by a host inside an MCP tool call, a keyword argument an SDK must
  pass to the child some other way — **and there is no document here to pay it for.** Reopens when
  a caller names a corpus this repository can pin whose documents need a user password the caller
  holds. Then the CLI reads it from a file descriptor or an environment variable, never argv; the
  SDKs set that for the child; and there is **no MCP line**, because
  [`mcp.rs:439`](../crates/ethos-parser-cli/src/mcp.rs) exposes no knobs and a secret is worse
  than a knob.
- **The silent open is a gap in the record whatever happens to the knob.** Contract §8 gives an
  encrypted source its own signal; an owner-only file is encrypted on disk and gets none. The
  artifact binds to the ciphertext's digest and every locator is correct, so nothing on the wire is
  wrong — but a consumer cannot tell from it that the bytes it names are ciphertext, and
  `was_encrypted()` is one call away. The precedent is §8.1: a repaired open is recorded three ways
  and never silent. Proposal 1. No corpus document changes under any answer.

## 4. Page sets

### 4.1 What exists

`PageBudget` is `Unlimited | AtMost(u32)` ([`profile.rs:1033`](../crates/ethos-parser-core/src/profile.rs)),
adjacently tagged with `deny_unknown_fields` so a knob cannot be dropped and re-hash to the same
digest, on the profile because it changes output (`profile.rs:1197`). `extract --max-pages N` is
the one caller that sets it ([`main.rs:555`](../crates/ethos-parser-cli/src/main.rs)); its rustdoc
(`main.rs:218`) records that changing it moves `profile_sha256` exactly as `classify --sample-pages`
does. `admits(page)` is `page <= n`: **a prefix, and nothing else.** Pages past it are `quarantined`
with `resource-limit-pages` ([`extract.rs:1095`](../crates/ethos-parser-pdf/src/extract.rs),
[`limitations.rs:502`](../crates/ethos-parser-pdf/src/limitations.rs)), the artifact is `partial`,
and every authorized page still appears in `page_states` (contract §7.1 rule 2). `classify` takes
the tighter of the budget and its 8-page sample and names whichever binds
([`classify.rs:203`](../crates/ethos-parser-pdf/src/classify.rs)). A set is representable without
touching a locator: `PageRecord.index` is the document's own page number, and its rustdoc says the
two differ "the moment a page is quarantined or fails"
([`representation.rs:1929`](../crates/ethos-parser-core/src/representation.rs)); page ids are
minted per emitted page ([`represent.rs:65`](../crates/ethos-parser-pdf/src/represent.rs)). MCP
and the SDKs expose neither `--max-pages` nor any budget (`mcp.rs:439`).

### 4.2 Measured

| | count | of |
| --- | ---: | ---: |
| documents over 50 pages, where a set and a prefix differ materially | **7** | 297 |
| documents over 8 pages, where `classify`'s sample already stops short | 9 | 297 |
| single-page documents | 262 | 297 |
| longest | 733 pages (`nist-sp-800-53Ar5`) | |

The seven are five gate documents (59 to 733 pages) and two gate-zero ones (80 and 492), all NIST
publications. The engine reads each whole today in under 13 s; the reason to bound one is memory,
which `--max-pages` bounds at ~7 MiB per admitted page plus a floor (`main.rs:229`).

### 4.3 Recommendation

**Defer, with a shape.** A set-valued budget is a third `PageBudget` variant — `Pages(sorted,
deduplicated list)` — whose `admits` is a membership test; the quarantine, the limitation, the
`partial` state and `PageRecord.index` need no change, and the variant moves `profile_sha256` as
`AtMost` does, so a set artifact and a prefix artifact are non-comparable by construction. The
memory argument does not carry to it: the floor the page loop cannot lower is the structure tree,
read over the whole document above the budget (`extract.rs:987`), and a set pays it as a prefix
does. **What is missing is a caller.** Nothing in the tree, MCP or the SDKs consumes a prefix
budget today, and no document here needs a non-prefix subset. Reopens when one is named — a host
that must cite page 412 of a 733-page document without holding 3.7 GiB — and lands with the lines
`--max-pages` has: the CLI flag and **no MCP or SDK line**, for the reason `--max-pages` has none.

## 5. `continue_on_page_error`

### 5.1 What exists

It is not a flag, and the design is first-error-fails-document. `PageState::Failed(code)` exists
([`assurance.rs:733`](../crates/ethos-parser-core/src/assurance.rs)), `pages_failed` is tallied
from it (`assurance.rs:869`), and **no production code constructs it** — only tests. The page loop
runs every page in parallel, collects every outcome, then walks them in page order and returns the
first error ([`extract.rs:1090-1123`](../crates/ethos-parser-pdf/src/extract.rs),
`let mut y = result?`), discarding the pages that succeeded. A content-stream decode error names
its page (`extract.rs:311`); an operator or font error does not.

Ids are minted per page against a local allocator and rebased onto the document sequence in page
order, holes included, so the same document under the same profile yields the same ids
([`ids.rs`](../crates/ethos-parser-core/src/ids.rs) module docs; `extract.rs:1124`). A page that
dropped out would consume no ordinal — its node count is what failing did not produce — so under a
continue mode every later page's ids are deterministic and shifted relative to a run in which the
page succeeds, and no hole can be reserved for it. The record still says which page is missing:
`page_states` carries `failed` with a code, `pages[]` carries no record for it, and
`PageRecord.index` keeps later pages at their own numbers.

### 5.2 Measured

| | count | of |
| --- | ---: | ---: |
| artifacts with `pages_failed > 0` | **0** | 293 |
| artifacts whose terminal state is not `complete` | 0 | 293 |
| refusals that are a page failing inside a readable document | **0** | 4 |
| documents where `classify` exits 0 and `extract` exits 2 | 1 (`tagged-cycle`) | 297 |

No corpus document exercises the mode, so the behaviour is shown on a three-page file whose page 2
runs `foo` ([`synthetic_page_error.py`](measurements/knobs/synthetic_page_error.py)): `extract`
exits 2 `[unsupported]` naming the operator; `--max-pages 1` exits 0 with page 1's one run, pages 2
and 3 `quarantined`, terminal `partial`; `--max-pages 2` exits 2; the bisection in
`engine_census.py failing-page` lands on page 2 in 4 probes; `classify` exits 0, because it
interprets no content stream. **Page 1's clean extract is thrown away with the document, today.**

### 5.3 Recommendation

**Refuse for now, on measurement, and record the design so it is not re-derived.** Zero of 293
artifacts and zero of 4 refusals are the case the mode serves. Building it means a page-scoped
limitation code for every `EngineError` variant, because contract §7.1 rule 3 says a page state's
reason is a code and not prose; the `Failed` arm in the page loop; and a decision that a page's
error reaches the artifact where today it reaches stderr — none of it hard, all of it untestable
on any document this repository can pin. A mode that turns a refusal into a `partial` artifact also
changes what exit 2 means, which the contract's fail-closed table owns. Reopens on a document in a
corpus this repository can redistribute that fails on one page and reads on the rest; the bisection
instrument finds the page. **The `tagged-cycle` row is a separate finding**: `classify` says
"simple" of a document `extract` refuses, because only `extract` reads the structure tree —
Proposal 5.

## 6. Batch

### 6.1 What exists

Nothing named batch. The CLI takes one path per invocation; MCP serves one document per call; each
SDK call spawns one process ([`__init__.py:423`](../packages/python/src/ethos_parser/__init__.py),
[`engine.js:279`](../packages/node/src/engine.js)). Inside one document the pages already run in
parallel under rayon (`extract.rs:1090`). The plan's §4 V3 verified that the competitor's
`batch-parse` is a sequential `for` loop with no pool and no per-file timeout; this document did
not repeat that reading.

### 6.2 Measured

Wall time of `extract` to `/dev/null`, three repetitions, minimum and (median), on a shared host
with load average 2.0 to 3.1 of 12 cores ([`results/batch.txt`](measurements/knobs/results/batch.txt)):

| set | sequential loop | `xargs -P 4` | `xargs -P 8` |
| --- | ---: | ---: | ---: |
| 200 single-page bench PDFs | 3.50 s (3.67) | 1.77 s (1.78), **1.98x** | 1.69 s (1.75), 2.07x |
| 7 gate documents, 2 to 327 pages | 8.48 s (8.49) | 3.72 s (3.75), **2.28x** | — |

Every pass exited 0 on every file. Two readings, and they say the same thing: **about 2x, from a
shell, quoted loosely because the host was shared.** On single-page documents the engine leaves
eleven cores idle and process start-up is a visible share of the 18 ms each invocation costs; on
multi-page ones rayon already holds the cores.

### 6.3 Recommendation

**Refuse a batch mode.** Its value is a number, and it is 2x on this machine for a loop the shell
provides in one line. A mode inside the binary would own a pool, a per-file timeout and an error
policy — three things the SDKs already own (`ETHOS_PARSER_TIMEOUT`, one process per call) — and it
would be the "supervisor" the plan's earlier draft imagined and its own V3 refuted. No MCP or SDK
line: the caller's loop *is* the batch layer. Reopens on a measured workload where process
start-up dominates the parse — documents smaller than the 18 ms ones here — and even then the first
candidate is a list of paths on stdin, not a pool.

## 7. Colour

### 7.1 What exists

Twelve colour operators are recognised by name ([`ops.rs:171`](../crates/ethos-parser-pdf/src/ops.rs)),
consumed as acknowledged no-ops, and keep no state
([`content.rs:618`](../crates/ethos-parser-pdf/src/content.rs)): "we do not interpret `rg`" is a
decision someone wrote down. No colour space is resolved anywhere. Font name and `/Flags` are read
(`fonts.rs:664`) and reach the wire as `font_id` and a `flags` list (`representation.rs:1377`),
serialization only, as the plan says. Honest colour is not the operand of `rg`; it is that operand
under the colour space in force, which `cs` selects by name from the page's `/ColorSpace`
resources: an ICC profile to read, an `Indexed` lookup table, a `Separation` or `DeviceN` tint
transform — a PDF function to evaluate — or a `Pattern`, which is not a colour at all.

### 7.2 Measured

Declared in page `/Resources /ColorSpace`, inherited resources followed, over the 294 readable
documents ([`results/qpdf-census.tsv`](measurements/knobs/results/qpdf-census.tsv)):

| family | entries (page, name) | documents |
| --- | ---: | ---: |
| `ICCBased` | 2,902 | 82 |
| `Indexed` over `ICCBased` | 763 | 10 |
| `Pattern` over `DeviceRGB` | 13 | 13 |
| `Indexed` over `DeviceRGB` | 7 | 4 |
| `Separation` over `DeviceCMYK` | 5 | 5 |
| none declared | — | 190 |

`CalRGB`, `CalGray`, `Lab` and `DeviceN`: **0** in page resources; `DeviceN` appears in 4
documents' form-XObject resources. **91 of 294 documents** declare a colour space that needs
resolution: 6 of 8 gate, 3 of 10 gate-zero, 82 of 200 bench, 0 of 79 fixtures. Image XObjects add
882 colour-space declarations on 108 documents. `/Pattern` resource dictionaries: 1 document;
`/Shading`: 6. Operator use was not counted (the README says why), so a document painting
`DeviceRGB` with `rg` and declaring nothing is among the 190.

### 7.3 Recommendation

**Refuse.** Three reasons, in order of weight. **No consumer is named** — not the verifier, whose
`ethos.grounding.v1` has no field for it, not a projection, not a host. **A colour on a text run is
a presentation fact**, and the one use anybody proposes for it — a heading is red, a link is blue
— is P14 by name, the line decision #19 drew. And **the cost is a number**: 31% of documents need
an ICC profile read or a tint transform evaluated before their colour is honest, and an engine
that emitted `rg`'s operands on those documents would state a colour the page does not show —
"extracted" in name, invented in fact. It is not a knob: it would be a wire field on every run,
inside the fingerprint, moving `profile_sha256` and growing every artifact, with no MCP or SDK
line because there is nothing to ask for. Reopens on a named consumer with a field to put it in,
and then only as a resolved colour with `Computed` derivation and a typed absence for every space
this engine cannot resolve.

## 8. Proposals for the owner

Numbered as proposals; none is decided here.

1. **Declare the empty-user-password open.** A document `lopdf` decrypts with the empty password is
   read today with nothing on the artifact saying its bytes are ciphertext (§3.2). Options: a
   document-scoped declaration beside `xref-entry-padded`; a limitation, which misdescribes it
   (nothing was withheld); or silence, recorded here as chosen. Zero corpus documents change.
2. **Password knob: defer**, reopening on a pinnable corpus that requires one; if taken, the secret
   never travels on argv or over MCP (§3.3).
3. **Page sets: defer**, reopening on a named caller; a third `PageBudget` variant, no adapter line
   (§4.3).
4. **`continue_on_page_error`: refuse on measurement** — 0 of 293 artifacts, 0 of 4 refusals —
   with §5.1 as the recorded design and a redistributable failing-page document as what reopens it.
5. **Record the `classify`/`extract` disagreement.** `classify` exits 0 on `tagged-cycle`, which
   `extract` refuses; it reads neither the structure tree nor a content stream (§5.2). Either
   `classify` reads the tree, or its help stops implying its exit 0 predicts `extract`'s, or this
   is filed in [`OPEN-WORK.md`](OPEN-WORK.md) §6 as a known gap.
6. **Batch: refuse** as a mode of the binary (§6.3).
7. **Colour: refuse** as an emitted field, P14 named beside the cost (§7.3).
8. **Retire 6.9 as one row.** After 1–7 its five items have five dispositions; `OPEN-WORK.md` §3
   should carry them separately, or drop the refused ones, in the commit that records the decisions.

## 9. Standing rules this document keeps

1. **A gap is never presented as a success.** A page nobody read is `quarantined` or `failed` by
   name; a document opened through a password it was not given says so, or the owner has chosen
   silence on the record.
2. **No invented colour, boundary or identifier.** A colour the engine did not resolve is not a
   colour it may emit; a page that failed leaves no reserved ids.
3. **Anything that changes a byte of output is in the profile**, and refusals are recorded with
   their reopening conditions, so each of the five is taken again on purpose rather than
   re-proposed from the row that named them.
