# 26 — `locate(representation, quote)` — scope

**Scope authority for North Star decision #30** and for `OPEN-WORK.md` §3's item 6.7. Slice detail
belongs in a milestones document written beside this one (§10); every `locate` PR belongs to exactly
one slice there.

**Written 2026-09-18 against local `main` at `a18b2b0`, workspace 0.58.0 unreleased.** Nothing here
reopens v2.2, and nothing here changes what any reader reads out of any document.

**This is a scope, not a decision.** Decision #30 is the decision; what follows is the shape, and
every place the shape and the row pull against each other is named rather than smoothed (§11). The
six questions only the owner can answer are in §12, each with the evidence beside it and no decision
invented.

---

## 1. The one sentence

**`locate` answers where a string lies in a representation, and answers nothing else.**

It is the engine's own question — `07-VERIFY-BOUNDARY.md` §2 states it as *"What does this document
contain, and exactly where?"* (line 22) — asked by a caller who holds a string instead of a node id.

## 2. What decision #30 binds, quoted

`00-NORTH-STAR.md` row 30 (line 62) sets four bounds and one re-refusal condition. Quoted, because
each one decides something below:

> - It takes a representation and a string — no claim, no citation, no verification profile — so
>   `07-VERIFY-BOUNDARY.md` §2's *the engine has no claim input* still holds.
> - It returns locations and nothing else: no verdict, no boolean, no score and no evidence tier. A
>   string that occurs nowhere is an empty answer with the same exit code, never a refusal.
> - Its match rule is the engine's own, versioned and stated. It reimplements none of the verifier's
>   quote semantics, and whether a location supports a claim stays the verifier's to say.
> - The forbidden-token greps stay green, and every locator it returns obeys the v1.2 handle law.
>
> **What would re-refuse it:** a caller-visible field or exit code that answers whether a claim
> holds, or a match rule copied from the verifier.

And the sentence the whole design is arranged around, `07-VERIFY-BOUNDARY.md` §2 lines 29–31:

> **The engine has no claim input.** That is the cleanest way to state the boundary: a component
> that never receives a claim cannot decide whether one holds. If a design starts wanting a claim
> parameter, the boundary is being crossed.

**A string is not a claim, and this document does not argue that it is close enough.** The bound
that carries the weight is the second: there is no field and no exit code anywhere below that
answers *whether*. Exit 0 and an empty `occurrences` array is the whole of the not-found answer, and
§7 names by name every convenience that would turn it into a verdict.

The handle law, `history/12-V12-SCOPE.md` §3 lines 46–50, applies verbatim:

| | Obligation | What it forbids |
| --- | --- | --- |
| **Mint** | Every locator a caller sees was produced by the engine, inside an artifact it already emits | An adapter computing a page, a box or a cell of its own |
| **Opaque** | A locator travels back as the bytes the engine handed out — a node id string, copied | A caller *composing* a locator, or an adapter documenting its shape as an input format |
| **Re-validate** | On the way in, the engine checks the handle against **that** artifact, and one it did not mint **fails closed** | Returning a best guess, the nearest node, or an empty result that reads like "no match" |

Plus its corollary at lines 56–63, *No tool argument may name geometry*, and lines 65–72, *Locators
travel in the artifact, never in the prose*. Row 24 (line 56) amends only one of the three — what
re-validation may skip, and when — and §6.2 says how.

## 3. Input and output

### 3.1 Input

**A `DocumentRepresentation v0` and a string. Nothing else.** The representation arrives exactly as
it arrives at `ground`, `markdown` and `html` today: as a path on the CLI, and as a path or an inline
object over MCP (`mcp.rs:352-359`). It is fingerprint-checked before a single scalar is compared, by
`DocumentRepresentation::verify_fingerprint` (`representation.rs:2719-2728`), the way
`run_ground` checks it at `main.rs:906` and `run_markdown` at `main.rs:772`.

**Both shapes of representation are accepted.** PDF and page-less office alike: `locate` reads
`payload().nodes`, `payload().tables` and the geometry sidecar, all of which every representation
carries, and it reads no locator and no page. It therefore never learns a format, which is what
`04-ARCHITECTURE.md`'s crate-boundary table asks of `ethos-parser-core` (line 38): no PDF concept,
no office concept.

### 3.2 Output — a new artifact type

**`ethos.parser.locations.v0`, `schema_version` `0.1.0`.** Following the engine's own family —
`ethos.parser.representation.v0` (`representation.rs:64`), `ethos.parser.classification.v0`
(`classify.rs:54`), `ethos.parser.extract.v0` (`extract.rs:42`), `ethos.parser.overlay.v0`
(`overlay.rs:54`), `ethos.parser.tags.v0` (`tagging.rs:2269`). The bare `ethos.*` prefix is used for
shapes the verifier's side consumes or mirrors — `ethos.grounding.v1`, `ethos.grounding_validation.v1`
— and for the two projections named alongside them; `locate`'s answer is consumed by nobody on that
side, so it takes the `ethos.parser.*` prefix.

**No existing shape fits, and reusing one would cost more than a new type.** `ethos.grounding.v1` is
`additionalProperties: false` and requires a `bbox` on every element and span (`01-CONTRACT.md` §11,
lines 429–456) — an occurrence with no measured box could not appear in it at all, which is the one
thing typed absence exists to prevent. The Markdown and HTML artifacts are projections of a whole
record with a map that must tile every byte (`markdown.rs:311-359`); an occurrence list tiles
nothing. The validation report is the checker's.

The wire shape, and the reasons are in §5:

```json
{
  "artifact_type": "ethos.parser.locations.v0",
  "schema_version": "0.1.0",
  "parser_version": "0.59.0",
  "profile_sha256": "sha256:…",
  "source_sha256": "sha256:…",
  "representation_sha256": "sha256:…",
  "locate_rule": "locate-scalar-exact-v1",
  "quote_scalars": 5,
  "searched": { "nodes": 4, "blocks": 2, "scalars": 14 },
  "occurrences": [
    { "synthesized": 0,
      "parts": [
        { "node": "s1", "char_start": 1, "char_end": 3,
          "geometry": { "state": "measured", "value": { "x0": 7200, "y0": 9200, "x1": 10800, "y1": 11000 } } },
        { "node": "s2", "char_start": 0, "char_end": 2,
          "geometry": { "state": "absent", "value": "no_ink_to_measure" } }
      ] }
  ],
  "occurrences_withheld": null
}
```

**What applies, stated rather than assumed:**

- **Canonical JSON, whole.** UTF-8, no whitespace between tokens, keys sorted by code point at write
  time, integers only, minimal escaping, **no Unicode normalization** (`01-CONTRACT.md` §4, lines
  78–86). stdout is canonical bytes and nothing else (`main.rs:32-36`).
- **Integer quanta.** A geometry value is the representation's own `QRect` in integer centipoints;
  `char_start`, `char_end`, `quote_scalars`, `searched.*` and `synthesized` are counts. No float
  reaches the wire, and a non-integer anywhere is a hard error.
- **`deny_unknown_fields` on every struct of it**, including the nested ones — the checklist line
  that exists because *"`deny_unknown_fields` is **not** recursive"* (`01-CONTRACT.md` line 534).
- **`geometry` is `GeometryPresence`, serialized as the representation serializes it** —
  `{"state":"measured","value":{…}}` or `{"state":"absent","value":"no_ink_to_measure"}`,
  from `#[serde(rename_all = "snake_case", tag = "state", content = "value", deny_unknown_fields)]`
  at `derivation.rs:203-210`. Not `Option<QRect>`, for the reason recorded there: `None` collapses
  three different situations into one.
- **`occurrences` is in reading order, and that is not a ranking.** Arrays are semantic —
  *"Element order *is* reading order"* (`01-CONTRACT.md` line 85) — and the order here is the order
  `payload().nodes` is already in. §7 refuses ranking by name.
- **No self-digest.** The artifact carries `representation_sha256` and `source_sha256` and no
  `*_c14n_sha256` of its own, exactly as `MarkdownArtifact` does (`markdown.rs:515-535`): nothing
  binds to an occurrence list, and the record it answers about is the thing with an identity.
- **It is a companion to the record, never a substitute.** That is why it restates no page and no
  node text: a consumer holding an occurrence necessarily holds the representation it names, the
  way a consumer holding an anchor map holds the Markdown.

## 4. The match rule — `locate-scalar-exact-v1`

### 4.1 What counts as an occurrence

| Question | The rule | Why, and what was refused |
| --- | --- | --- |
| **Unit** | Unicode **scalar values** | The unit `char_offsets` already uses and the consuming validator already slices by (`01-CONTRACT.md` lines 448–450). Refused: UTF-8 bytes and UTF-16 code units — row 20 (line 52) records the exact defect a UTF-16 count produces on an astral character, and `22-WORD-BOXES-SCOPE.md`'s amendment measured **280,617** of 2,447,419 spans whose offset a byte cursor would have written differently (line 148) |
| **Comparison** | **Code-point-exact.** No normalisation, no case folding, no whitespace folding | `01-CONTRACT.md` §4 line 82: *"**No Unicode normalization** — extracted text is preserved exactly as extracted"*. A folding rule would have to define its table version on the wire or be row 20's refused shape — *"that definition exists only in a Rust doc comment"* (line 52) |
| **Searched string** | Each **block's** text, as `markdown::geometric_blocks` builds it | `GeometricBlock.text` is *"the members' own text, concatenated"*, and *"A space the page drew is a run of its own, with its own characters, so joining member texts reproduces exactly what the document drew and invents no separator"* (`markdown.rs:1218-1231`). It is the **same string** `ethos.grounding.v1`'s element text is (`grounding/lib.rs:763`, `838`), so an occurrence's offsets are offsets into a string the grounding artifact already publishes |
| **Separator** | **None.** Nothing is inserted between two runs | Any inserted character would be a synthesized character this engine authored at match time, which `01-CONTRACT.md` §10 (line 416) requires to be *flagged where it is created* — and there is no document to flag it on |
| **Crossing nodes** | **Yes, inside one block** | The measured reason it is safe: a block boundary opens only at whitespace wider than its band's own leading, at **100% precision** on the one labelled gate document — *"never firing mid-paragraph across 719 chances"*, at 63.7% recall (`CAPABILITY.md`, Layout blocks; `19-BLOCK-SUBDIVISION-SCOPE.md`). One document, and the band rides with the number |
| **Crossing blocks** | **No.** A match spanning two blocks is not an occurrence | A block boundary is a gap the page drew. `untagged-shredded-line`'s own comment says it: *"a rule that joined on absence alone would produce `YarrowSeparate` across a gap the page plainly drew"* (`make_fixtures.py:663-664`). Joining without a separator asserts an adjacency the page did not draw; joining with one invents a character. Both are fabrication |
| **Case** | Sensitive | See Comparison. A folded rule reopens only as a named `-v2` (§12) |
| **Empty quote** | **Refused**, exit 2, `EngineError::Unsupported` naming it, nothing on stdout | An empty string occurs at every offset of every block, so *"every place it occurs"* has no answer with a meaning. This is **not** the not-found path and must not be read as one: decision #30's *"a string that occurs nowhere is an empty answer with the same exit code"* governs a string that occurs nowhere, and the empty string occurs everywhere. Refused: exit 0 with an empty list, which would tell a caller that a string occurring at every position occurs at none — a false negative about a document, which is the one class of statement this repository refuses outright |
| **Quote longer than every block** | No occurrence, empty answer, exit 0 | Nothing special; it is the not-found path |
| **Quote longer than 16,384 UTF-8 bytes** | Refused, exit 2, naming the limit | The number is `ethos.grounding.v1`'s own longest admissible string, `check::limits::MAX_STRING_BYTES` (`ethos-parser-grounding/src/check.rs:61`), so a longer quote is longer than any element text a citation could carry. **It is re-declared in core rather than imported**, because `ethos-parser-grounding` depends on `ethos-parser-core` and not the other way round (`04-ARCHITECTURE.md`'s crate table), and a `pub use` the wrong way would invert the graph. `LOCATE_MAX_QUOTE_BYTES` therefore sits beside the rule id, and a test **in the grounding crate**, which can see both, asserts the two numbers are equal — so they cannot drift without a red test |
| **Overlapping matches** | **All start positions**, overlapping included | The question is every place the string occurs. `aa` in `aaa` is two occurrences, at 0 and 1. Refused: leftmost-non-overlapping, which drops a real occurrence by an order the caller cannot reproduce from the string alone |
| **Cap** | Counted in full; **over the cap, the count travels and the locators do not** (§4.3) | |
| **What is searched** | Every node's text, in exactly one block each | `geometric_blocks` makes a node of another kind, or one a table already claims, *standalone* (`markdown.rs:1260-1263`), so it is a one-member block — nothing is skipped. The one string that is not searched is a **table cell's own concatenation**, which is neither a node's text nor a block's (§12) |
| **Synthesized characters** | Searched, and **declared per occurrence** | `01-CONTRACT.md` §10 line 412: *"Unflagged, a verifier will one day match a quote against a space the source does not contain."* `locate` is that place. So an occurrence states how many of its scalars the reader synthesized; it is not refused, because refusing it would be a judgement about the match. Stated always, never omitted where zero — decision #20 (line 52) |

### 4.2 Where the verifier's semantics differ, and that this is not them

Read only from this repository's records: `07-VERIFY-BOUNDARY.md`, and
`22-WORD-BOXES-SCOPE.md` §4 (lines 228–267), which measured the pinned Ethos v0.6.0 (`8adda91`)
against five variants of one artifact. The sibling tree was not read for this document.

| | the verifier, as this tree records it | `locate-scalar-exact-v1` |
| --- | --- | --- |
| Input | a typed claim, a grounding artifact and a verification profile (`07-VERIFY-BOUNDARY.md` line 25) | a representation and a string |
| What it resolves against | **`ethos.grounding.v1`** — elements and spans, after the projection's omissions | the **representation** — every node, omitted or not |
| Order of resolution | *"A claim resolves to the element before any span"*; cited by page and box, only elements are searched; cited by page alone, a page's elements are scanned before its spans (§4 lines 237–239) | no precedence at all: every occurrence is returned, none is preferred |
| Text comparison | a span's own box comes back only where *"a page-only `value` claim equals a span's text exactly"* — otherwise `text mismatch` (§4 lines 241–242, and the table's row 4) | exact, on scalars, and it stops there |
| Crossing a boundary | *"The adjacency join that lets a quote cross a boundary requires `element_id`"* and unions element boxes (§4 lines 239–241) | joins inside a block, unions nothing (§5) |
| Table cells | *"A `table_cell` claim always resolves through its table"* (§4 line 242) | no table resolution; a cell's runs are one-member blocks (§12) |
| Character offsets | *"`char_start` and `char_end` are checked by the grounding validator and never read by verification"* (§4 lines 233–234) | offsets are the answer |
| Output | `evidence.bbox` with its page and text, inside a report (§4 line 230) | node ids, offsets, and the representation's own geometry |

**Stated plainly: this rule is not the verifier's, is not derived from it, and is not a prediction of
it.** The five differences above are each a place where the verifier would answer differently, and
three of them — element-before-span, the `element_id`-gated adjacency join, and table resolution —
mean a string `locate` finds may be one the verifier does not bind, and a string `locate` does not
find may be one the verifier binds through a table. **A location is not a verdict, and nothing here
converts one into the other.** `07-VERIFY-BOUNDARY.md`'s *Never* clause (lines 127–128) is the rule
this section exists to satisfy: *"Linking the same verifier is not a second integration.
Reimplementing its semantics is."*

**Refused as the match rule: the Markdown projection's whitespace rule.** `history/11-V11-MILESTONES.md`
line 88–94 pins it — *"NFC-normalized, with internal whitespace collapsed to one space, and
trimmed"* — and it is the natural `-v2`. It is refused for `-v1` because it searches a string the
representation does not contain: the same document's Markdown holds `block\n\nSecond`, which that
milestone calls *"real text in the Markdown that the document never drew"* (line 108). A caller who
wants that rule is asking for a search over `ethos.markdown.v1`, which is a different question with
a different artifact already in the tree.

### 4.3 The cap, and what happens at it

**MCP is a long-lived process reading untrusted documents repeatedly** (`mcp.rs:436-437`), and a
one-character quote on a gate document would produce occurrences in the millions.

**The rule: count every occurrence; over the cap, emit the count and no locators**, with
`occurrences_withheld` carrying `{occurrences, limit, limitation_code:
"occurrences-withheld-over-limit"}` and `occurrences` empty. Exit **0** — the string was found, and
a count is one of the four honest signals `00-NORTH-STAR.md` §3 (line 80) permits: *"a count, a
named reason, a typed absence, or a declared limitation."*

All of them are withheld, never the excess alone. That is `SpansWithheld`'s own rule, quoted from
`grounding/lib.rs:633-648`: *"truncating to the cap would ground some of a document's runs and
silently drop the rest"*. A truncated occurrence list is the same shape of lie.

**The cap is 1,000,000**: `ethos.grounding.v1`'s own `check::limits::MAX_ELEMENTS`
(`ethos-parser-grounding/src/check.rs:58`), the number it puts on the population an occurrence most
resembles — re-declared in core as `LOCATE_MAX_OCCURRENCES` for the dependency reason above, with
the same equality test holding the two together.

**The number is provisional until S1 measures the artifact's size at it**, and that measurement is
S1's acceptance rather than a later slice's, so no surface ships on an unmeasured cap. Grounding
artifacts already reach 130.46 MB (`22-WORD-BOXES-SCOPE.md` line 156) and `ethos.grounding.v1`'s
input ceiling is 256 MiB (`check.rs:56`), so the arithmetic is answerable; this document does not
answer it. If the measured size at 1,000,000 exceeds what a caller can hold, S1 lowers the cap and
records the number it lowered it to.

## 5. What an occurrence carries

An occurrence is **`parts`** — one per node its scalars touch, in order — and a `synthesized` count.

| Field | What it is | Why |
| --- | --- | --- |
| `parts[].node` | A node id, **copied verbatim** out of the representation searched | The Mint and Opaque obligations. A node id is the one locator this engine already mints inside an artifact it already emits, and it is the id the grounding artifact's spans are keyed by: `Span.id` is `node.id.as_str()` (`grounding/lib.rs:845-846`). So a caller holding both artifacts joins an occurrence to its grounding element through `span.element`, with no mapping table and no new identity |
| `parts[].char_start`, `char_end` | Unicode scalars into **that node's own `text`**, start inclusive, end exclusive | The strongest available form of re-resolvable: node id → `Node.text` (`representation.rs:1988`) → slice by scalars. It needs nothing but the representation. **The load-bearing invariant:** the parts' slices, concatenated in order, equal the quote exactly — and §9's test T12 asserts it |
| `parts[].geometry` | The representation's own `GeometryPresence` for that node, **copied** | Decision #30 names boxes. Copied rather than derived, and a test asserts every emitted value equals `repr.geometry_at(index)` (`representation.rs:2682-2684`), so it cannot disagree with the sidecar — *"a value that can be asserted independently of what it describes is a value that can disagree with it"* (`representation.rs:2152-2155`) |
| `synthesized` | How many of this occurrence's scalars the reader synthesized | §4.1's last row |

**No union box.** An occurrence covering three runs carries three boxes and no fourth. A union would
be measured arithmetic (`union_of`, `grounding/lib.rs:666-673`) and would still claim area over text
outside the quote — the first and last runs' unmatched characters. **And no sub-run box**, which is
not a design choice here but a refusal already on the record: word boxes are refused on measurement
(`22-WORD-BOXES-SCOPE.md` §7 lines 318–334; `02-ROADMAP.md` line 63). `locate` therefore binds no
box tighter than a run, and says so.

**No `page`, no re-stated node text, no element id.** A page is resolvable from the node the
occurrence names and would be a second assertion that can disagree with the first (§8). An
element id would be a second id space: the grounding projection mints `e{n}` only for blocks that
survived its omissions (`grounding/lib.rs:829-830`), so a block has no id in the representation at
all — a finding worth stating, because it is why the answer is addressed by node ids and could not
have been addressed by blocks.

**Not a second evidence tier.** Everything an occurrence carries is a copy of something the
representation already says, checked against it by a test. It adds no address, no coordinate and no
identifier the record did not already contain — `00-NORTH-STAR.md` §7's anti-goal (line 148).

## 6. The three surfaces, in one slice

Decision #30: *"It ships as a CLI subcommand, an MCP tool and SDK functions in one slice."* §10's
S2 is that slice.

### 6.1 The CLI subcommand

```
ethos-parser locate <representation> --quote-file <FILE>
```

**The eleventh subcommand.** `enum Command` at `main.rs:75-230` has ten today — `classify`,
`extract`, `ground`, `markdown`, `html`, `mcp`, `overlay`, `tag`, `grounding-check`, `verify` — and
that enum is *"the list that cannot go stale"* (`main.rs:21`). §10 lists every document that counts
them.

**The quote is passed as a file, and not on argv.** Two reasons, the second decisive:

1. `25-KNOBS-SCOPE.md` §3.3 (lines 113–117) refuses a secret on argv — *"visible to `ps` on argv,
   logged by a host inside an MCP tool call"* — and a caller's quote is content it may not want in
   a process table either. This is an argument by analogy and it is the weaker one.
2. **argv cannot carry every string a representation can contain.** A NUL cannot appear in an
   argument at all, a newline survives only through correct quoting, and a shell-quoting mistake
   changes the string *silently* — which changes what was searched with nothing on the wire saying
   so. A rule whose input cannot be expressed faithfully is not a rule a caller can reproduce.

**The bytes are read verbatim: no trim, no trailing-newline strip, no BOM removal.** Stripping one
trailing LF would make `printf %s` and `echo` agree and would make a quote that genuinely ends in a
newline unaskable — a silent edit of the caller's input, which is the silent repair
`history/12-V12-SCOPE.md` §8 rule 3 forbids. A shell caller who has the quote in a variable writes
`--quote-file <(printf %s "$q")`; the CLI's bounded read accepts a non-regular file, having held up
to the ceiling (`main.rs:390-424`).

The file is read through `read_source` (`main.rs:386-388`), so `MAX_SOURCE_BYTES` applies before the
16,384-byte quote limit refuses it by name. Non-UTF-8 bytes are a named refusal, exit 2: a
representation's text is a Rust `String` and a byte sequence that is not UTF-8 cannot occur in it,
so there is nothing to search for.

**Exit codes: 0 answered, 2 could not read or refused. There is no exit 1, ever.**

An exit 1 meaning *not found* is precisely decision #30's re-refusal condition — *"a caller-visible
field or exit code that answers whether a claim holds"* — one composition away from `locate … ;
if [ $? -eq 1 ]`. The precedent for refusing the code outright is `extract`'s
(`main.rs:89-91`): *"overloading it would make a caller's `&&` chain mean two different things
depending on which subcommand ran."* Here it would make the chain mean a verdict.

`--diagnostics` works as it does everywhere, through `timed` (`main.rs:488-508`), under
`Stage::Ground` — it reads a representation, as `markdown` and `html` do (`main.rs:451-458`).

### 6.2 The MCP tool

A fourth tool, `locate`, beside `extract`, `ground` and `node_get` (`mcp.rs:325-388`).

```json
{ "name": "locate",
  "inputSchema": { "type": "object", "additionalProperties": false,
                   "required": ["representation", "quote"],
                   "properties": { "representation": { "type": ["object", "string"] },
                                   "quote": { "type": "string" } } } }
```

**What it must not accept**, and each is checked against the advertised wire rather than against a
reviewer's memory by `mcp.rs:828-866` and `mcp_stdio.rs:172-202`: `page`, `bbox`, `box`, `rect`,
`x`, `y`, `w`, `h`, `width`, `height`, `row`, `column`, `col`, `span`, `offset`, `coords`, `region`.
`quote` is on neither list and is not a locator: it is content, and the engine mints the locators.
`additionalProperties: false`, or *"extra arguments, which is where a locator sneaks in"*
(`mcp.rs:886-888`).

**The tool count moves from three to four, deliberately, through the line that says so.**
`mcp.rs:875-881` asserts `tools.len() == 3` with the message *"three is the number this server has
argued for … A fourth is a decision, and it arrives through this line."* This document is that
decision; the slice edits that line and `mcp.rs:832-837`'s floor.

**Row 24's memo applies with no new code and no new key.** `locate` calls
`representation_arg(args, ledger)` (`mcp.rs:560-581`), the same function `ground` and `node_get`
call, so a path-form representation goes through `ledger.load(bytes)` (`mcp.rs:669-698`): it is
parsed, hashed, and verified **unless these exact bytes already verified in this process**. An
inline object is verified every call, because *"a `Value`'s serialization is a build property"*
(`mcp.rs:76-77`).

**The quote is never part of the key, and no answer is ever remembered.** The ledger remembers a
verification, not a reply. Row 24's own amendment is the constraint: *"no call's answer depends on an
earlier call; an earlier call may make a later call on byte-identical input cheaper."* A ledger keyed
on `(bytes, quote)` returning a cached occurrence list would break it. `mcp_stdio.rs:585`'s
`every_reply_in_a_session_is_the_reply_a_fresh_server_gives` must cover a `locate` call, which is
how that stays true rather than asserted.

**The model-facing summary is counts and nothing else.** `"3 occurrence(s) in 2 block(s)."`, or
`"0 occurrence(s)."` — no node id, no box, no page, no digest.
`the_artifact_through_mcp_is_the_artifact_the_cli_prints` (`mcp_stdio.rs:236-241`) already fails on
`bbox`, `[`, `x0`, `origin`, `sha256:` and `s1` in that field, and the split is
`history/12-V12-SCOPE.md`'s: *"A summary a model reads may carry counts. It may not carry a box."*
The summary must not read as a verdict; a count is not one, and `"not found"` as a phrase is
refused in §7.

**A forged or edited representation is a tool error, not an empty result** — `isError: true`
(`mcp.rs:418-422`). *"An empty result tells a model its guess was unlucky; an error tells it the
guess was not admissible."* A quote that occurs nowhere is the opposite case and is `isError: false`
with an empty array, which is the distinction decision #30 draws.

### 6.3 The SDK functions

```python
ethos_parser.locate(representation, quote)   # returns the parsed locations artifact
```
```js
locate(representation, quote)                // returns the parsed locations artifact
```

**Both shell out to the subcommand, and neither ports the match rule.** That is the asymmetry with
`node_get`, and it is the right way round. `node_get` has no subcommand, so its checks were *ported*
and the packages carry a canonical-JSON port whose agreement with Rust had to be proved on real
input (`history/13-V12-MILESTONES.md` lines 117–129). **A ported match rule would be a second
implementation of the answer**, and two implementations of a text-matching rule that can disagree is
exactly what `locate` must not be. So: one process spawn, the engine's own bytes back
(`python/__init__.py:219-231`, `node/src/index.js:130-132`).

**The quote reaches the child through a file, not argv**, by the contextmanager
`_representation_path` already uses (`python/__init__.py:260-282`) and Node's
`withRepresentationPath` (`node/src/index.js:161`). Written as raw UTF-8 bytes, removed afterwards.
`--` separates the paths from the flags, as `ground` already does.

**Timeouts unchanged.** `_run` applies the existing wall clock — 600 s, `ETHOS_PARSER_TIMEOUT`, `0`
to wait forever (`python/__init__.py:392-401`, `423-461`) — and Node's `run` the same. No new
environment variable, no per-call budget.

**Python is the contract and Node follows it** (`node/src/index.js:18-24`): the only permitted
differences are the name's casing and a throw instead of a raise. `locate` needs neither.

**No LangChain tool in this slice.** The two that exist wrap `extract` and `ground`, and the
`ground` tool exists to relay the server's own summary verbatim
(`history/13-V12-MILESTONES.md` lines 109–111). A `locate` tool would be a model-controlled surface
whose argument is a free-text string the model composes — which is not a locator and so not a handle-law
violation, but it is also not asked for by decision #30, and *"a tool that exists because it was cheap
is a surface to keep honest forever"* (`history/13-V12-MILESTONES.md` line 59). Refused for this
slice; it reopens on a named host.

## 7. What is refused, by name

Each with the reason, and each the shape of a convenience somebody will propose.

| Refused | Reason |
| --- | --- |
| A `claim` or `claims` parameter, on any surface | `07-VERIFY-BOUNDARY.md` §2 line 30: *"If a design starts wanting a claim parameter, the boundary is being crossed."* Decision #30's first bound |
| A `citation` parameter — a `{page, element_id}` object | It is a claim's other half, and every field in it is geometry or a foreign id: the handle law's *No tool argument may name geometry* forbids the first and the Mint obligation the second |
| A verification profile, a `--profile`, or a verifier pin argument | #30's first bound. `VerifierPin::NotPinned` is a real statement and `locate` consults no verifier (`PUBLIC-API.md` lines 123–125) |
| `found`, `present`, `is_present`, `ok`, `grounded`, `verified` — any boolean | #30's second bound, and `07-VERIFY-BOUNDARY.md` §8 anti-pattern 5: *"`is_grounded()`, `looks_verified`, `quick_check()` … Each is individually harmless and collectively the whole failure"* |
| A `confidence`, `score`, `similarity`, `match_quality` or `trust` field | `01-CONTRACT.md` §9 (lines 367–371), and `ci/forbidden-tokens.sh confidence` fails the build on the substring (`forbidden-tokens.sh:46`) |
| `best_match`, `top_match`, a `rank`, a `limit` that keeps the "best" N | A preference among locations is a judgement about which one supports something. Reading order is the order, and the cap withholds all or none (§4.3) |
| An exit code, on any surface, that differs between found and not found | #30's re-refusal condition, stated as an exit code. §6.1 |
| `--fail-on-missing`, `--fail-if-absent`, or any gate flag | The same thing wearing `verify --fail-on-ungrounded`'s clothes. That flag gates a **verifier's** answer (`main.rs:308-314`); there is no answer here to gate |
| A match rule copied from the verifier, or tuned to agree with it | #30's third bound and its re-refusal condition. §4.2 |
| Normalised, case-folded, whitespace-folded or fuzzy matching in `-v1` | §4.1. Each reopens as a named `-v2` (§12), never as a flag on `-v1` |
| A union box for an occurrence, or a sub-run box | §5. The second is a standing refusal on measurement |
| An element id minted by `locate` | §5. A second id space for a block the representation does not name |
| A `page`, `node_id` or `block` argument scoping the search | The handle law for the first; YAGNI for the others — a scoped search is a second question nobody has asked |
| A ledger entry keyed on the quote, or any cached answer | Row 24's amendment (§6.2) |
| A subcommand that composes `locate` and `verify` | `07-VERIFY-BOUNDARY.md` §8 anti-pattern 4: re-deriving verifier outputs is *"how a second authority is born by accident"* |
| The phrase "not found" in the model-facing summary | It is one word from a verdict, and a count says the same thing without being one |
| A capability flag for `locate` on the profile | `history/12-V12-SCOPE.md` §7 lines 119–120: *"A capability describes what this profile can read out of a document."* `locate` reads nothing new out of one |

## 8. Identity and compatibility

**`locate_rule` is a field of `Profile`, and `profile_sha256` moves with it.** The nearest two
precedents and the contract's own blanket rule all point the same way, and this document follows
them rather than arguing an exception:

- `01-CONTRACT.md` §2 line 45: *"**Anything that can change a byte of output belongs in the
  profile, or it is a bug.**"* The locations artifact is output, and `locate_rule` decides what comes
  out of it.
- `markdown_rule` and `html_rule` are on the profile, and `markdown_rule`'s rustdoc gives the reason
  in words that apply here unchanged (`profile.rs:1259-1264`): *"On the profile because it decides
  what comes out … an artifact whose hash could not tell them apart would claim a comparability it
  lacks."* Neither of those rules changes a representation byte either, which is the one argument for
  keeping `locate_rule` off the profile — and it did not carry for them.
- Rows 19 and 22 each paid the regeneration cost with the decision recording it.

**The cost is zero in this release and a full regeneration after it.** 0.59.0 already re-pins the
profile: auto-tagging S1 moved `struct_tree_rule` to `struct-tree-v2` (`profile.rs:478`), and every
version moves `profile_sha256` anyway because `parser_version` is a profile field. Deciding the other
way now would make the correction later cost a regeneration of every golden on its own. So the field
lands in 0.59.0 with the rest.

**What moves with it:** `Profile`'s field and its rustdoc, `docs/draft-schemas/profile.draft.json`,
and the re-pin in `the_default_profile_is_pinned` — one commit, in S1, with the pin taken from the
test's own output.

**What does not move: the bytes of any existing artifact's content.** `locate` adds no field to a
representation, a classification, an extract artifact, a grounding artifact, a Markdown artifact or
an HTML artifact, and changes no reader's answer about any document. Every such artifact's
`profile_sha256` string moves, as it does in every release, and the release note says so in the
sentence it already has to write.

**`identity.profile_sha256` on the locations artifact is the profile the engine was running**, as
`MarkdownArtifact`'s is. It does not claim to restate the profile that produced the record —
`representation_sha256` binds that exactly — so it does not extend the recorded office defect, where
*"Office markdown and html stamp the PDF default profile's `profile_sha256`, not the profile that
produced the representation"* (`OPEN-WORK.md` §6).

**One note that must ride on the artifact's own documentation:** geometry is outside the
representation's fingerprint by design (`01-CONTRACT.md` line 100; `representation.rs:2011-2022`).
A box `locate` echoes is bound to the record by a digest that never covered it. The sidecar's own
rustdoc says this already — *"nobody should read a verified fingerprint as covering a bbox it never
touched"* — and `locate`'s must say it too, because `locate` is the first surface where a box travels
with a digest beside it.

**No page on an occurrence, settled here.** Decision #30 names node ids, character offsets and
boxes, and does not name a page; a node states its own (`representation.rs:1984`), so restating it
would be a second assertion that can disagree with the first — the argument
`representation.rs:2152-2155` makes in as many words. The asymmetry with `ethos.grounding.v1`, which
restates `page` on every element and span, is real and is left standing: that shape is the
verifier's input and is built to be read without the record beside it, where an occurrence list is a
companion to a record the caller necessarily holds. What would reopen it: a named consumer that holds
occurrences without the representation.

**What the release note says**, drafted:

> ### Added
> - `ethos-parser locate <representation> --quote-file <FILE>` — an eleventh subcommand, an
>   `ethos.parser.locations.v0` artifact, an MCP `locate` tool and `locate()` in both SDKs. It
>   answers where a string lies in a representation: every occurrence, as node ids, character
>   offsets in Unicode scalars, and the representation's own geometry for the nodes touched. The
>   match rule is `locate-scalar-exact-v1`, code-point-exact, on the profile and named on every
>   artifact; matches join runs inside one block of the reading-order cut and never across two. Exit
>   0 whether the string occurs or not; a string that occurs nowhere is an empty answer. **It emits
>   no verdict, no boolean and no score, and its match rule is not the verifier's** — North Star
>   decision #30, and `docs/26-LOCATE-SCOPE.md` §4.2 records the five places the verifier resolves a
>   quote differently.
>
> ### Unchanged
> - No representation, classification, extract, grounding, Markdown or HTML artifact changes a byte
>   of its content; `profile_sha256` moves for this release as it does for every release, and
>   `locate_rule` is one of the fields it now covers.

## 9. The acceptance tests

Named, each with the fixture it uses, preferring fixtures that exist. Every path is
`fixtures/engine/<name>/document.pdf` unless stated.

| | Test | Fixture, and the fact it rests on |
| --- | --- | --- |
| **T1** | found: one occurrence, one part | `measured-ink-box` — one run, text `Measured` (`make_fixtures.py:642`). Quote `Measured` → one occurrence, one part, `char_start` 0, `char_end` 8, geometry `state: measured` (the only fixture that takes the measured branch) |
| **T2** | not found: exit 0, `occurrences: []`, and **no** `isError` over MCP | `measured-ink-box`, quote `Absent` |
| **T3** | several occurrences, and the order is reading order | `untagged-shredded-line` — runs `Yar`, `ro`, `w`, `Separate`; the first three abut exactly and form one block whose text is `Yarrow` (`make_fixtures.py:665-670`). Quote `r` → **two** occurrences: block scalars 2..3, wholly in node `Yar` at its own 2..3; and 3..4, wholly in node `ro` at 0..1 |
| **T4** | a quote crossing runs | Same fixture, quote `arrow` → one occurrence, **three** parts: `Yar` 1..3, `ro` 0..2, `w` 0..1; and their slices concatenated equal `arrow` |
| **T5** | a quote crossing elements is **not** an occurrence | `markdown-two-blocks` — two runs, `First block` and `Second block`, 60 pt apart (`make_fixtures.py:671-674`). Quote `First blockSecond block` → `occurrences: []`, exit 0. Same fixture also covers T3's negative: `untagged-shredded-line`'s `YarrowSeparate` → `[]`, the string the fixture's own comment was written about |
| **T6** | text that exists only after a projection | `markdown-hyphen-break` — `The rate may be recalcu-` and `lated at closing` (`make_fixtures.py:698-701`). Quote `recalculated` → `[]`. And `markdown-two-blocks` with the Markdown's own `block\n\nSecond` → `[]`. Both are the strings `history/11-V11-MILESTONES.md` lines 104–111 built those fixtures to mark unquotable |
| **T7** | the empty quote is refused | Any fixture. Exit 2, nothing on stdout, the error names the empty quote; over MCP `isError: true`. And the test asserts this is **not** T2's path, by comparing the two exit codes |
| **T8** | a synthesized character inside an occurrence is declared | `synthesized-space-tj` — `[(one) -500 (two)] TJ` (`make_fixtures.py:775-777`), where the reader synthesizes a space. A quote spanning it reports `synthesized: 1`; T1's reports `synthesized: 0` |
| **T9** | right-to-left text: visual order is what is searched | `rtl-hebrew-visual-order` — four CIDs drawn in visual order, so the run's text is the logical word reversed (`make_fixtures.py:1114-1126`, `_RTL_TOUNICODE` at `make_fixtures.py:1657-1669`, and `extraction.rs:4723-4736`). Quote `שלום` (logical) → `[]`; quote `םולש` (as drawn) → one occurrence. The test's message names `CAPABILITY.md`'s row and `OPEN-WORK.md` §4's pending decision, so the test records the limitation rather than hiding it |
| **T10** | an astral scalar counts as one | **No fixture carries a non-BMP scalar** — checked across `fixtures/engine/make_fixtures.py`. So this is a unit test over a **hand-built representation**, which is the precedent `history/11-V11-MILESTONES.md` line 84–86 sets for a path the corpus cannot reach: *"proved by a unit test over a hand-built representation instead of by a PDF nobody has, which is the honest way to test a path the corpus cannot reach."* It pins all three units at once: a quote holding U+1F600 is 1 scalar, 4 UTF-8 bytes and 2 UTF-16 code units, and the offsets are in scalars. §12 records the standing item asking whether a fixture should be authored instead |
| **T11** | a combining mark is two scalars and is not folded | Hand-built representation, same reason: `rtl-hebrew-visual-order` carries Hebrew **letters** and no niqqud, so no committed fixture holds a combining mark. `e` + U+0301 must not match a precomposed `é`, and the message says the rule is `01-CONTRACT.md` §4's *no Unicode normalization* |
| **T12** | **the invariant**: the parts' own slices, concatenated, equal the quote | Every fixture above, asserted in one loop. This is the test that makes an occurrence re-resolvable, and it is the one that fails if the offsets are ever written against the block's text instead of the node's |
| **T13** | every emitted geometry equals the sidecar's | Every fixture above: `part.geometry == repr.geometry_at(node_index)`, so nothing is re-derived |
| **T14** | the adapter round trip | `locate` over MCP re-canonicalized equals the CLI's stdout byte for byte, on the `the_artifact_through_mcp_is_the_artifact_the_cli_prints` pattern (`mcp_stdio.rs:213-232`); and the model-facing summary contains none of `bbox`, `[`, `x0`, `origin`, `sha256:`, `s1` |
| **T15** | the SDK round trip | Python and Node `locate()` return the parsed form of those same bytes, and re-canonicalizing what came back reproduces the CLI's stdout — the tautology `history/13-V12-MILESTONES.md` lines 96–101 describes. Plus: a quote containing a newline and a quote containing a NUL both survive the adapter, which is the test that argv could not have passed |
| **T16** | the fingerprint is checked before anything is searched | An edited payload with its declared digest kept — `tampered()`'s shape (`mcp.rs:954-967`) — is refused on all three surfaces, with no occurrence returned and the message naming both digests |
| **T17** | the handle law's wire checks still hold with four tools | `mcp.rs:828-866` and `mcp_stdio.rs:172-202` pass unchanged; the two count assertions move from 3 to 4 |
| **T18** | office representations answer too | `fixtures/office/simple-paragraphs/document.docx` — a quote inside one paragraph's run is found, with geometry typed-absent rather than omitted |
| **T19** | the greps stay green | `ci/forbidden-tokens.sh confidence` and `ci/forbidden-tokens.sh verification` exit 0 with the new module in the tree (`forbidden-tokens.sh:46`, `57`) |
| **T20** | the frozen surface and the thin shell | `public_api.rs`'s `the_public_surface_is_the_frozen_one` and `the_public_api_document_names_every_frozen_item` cover the new exports; `library_surface.rs` gains `locate_is_reachable_and_canonical_from_the_library`, and a CLI-equals-library test, both named in `PUBLIC-API.md`'s mapping table |

**Two tests it is worth saying do not exist.** There is no test that `locate` agrees with the
verifier about anything, and there must not be: agreement would be the property #30 refuses. And
there is no test that every occurrence is one the verifier would bind, for the same reason.

## 10. Slices, and what else must move

In the shape of `24-AUTO-TAGGING-MILESTONES.md`, smallest first. The milestones document owed
alongside this one carries them with their file lists.

| Slice | Theme | Acceptance |
| --- | --- | --- |
| **S0** | This document and the milestones document | Both committed before any code, which `02-ROADMAP.md` line 101 requires and which the block cut is the one recorded exception to |
| **S1** | The core query: `crates/ethos-parser-core/src/locate.rs` — `locate()`, `Locations`, `Occurrence`, `OccurrencePart`, `OccurrencesWithheld`, `LOCATE_RULE_V1`, `LOCATE_MAX_QUOTE_BYTES`, `LOCATE_MAX_OCCURRENCES`, `LOCATIONS_ARTIFACT_TYPE`, `LOCATIONS_SCHEMA_VERSION`, calling `markdown::geometric_blocks` rather than restating it; `locate_rule` on `Profile` with the draft schema and the re-pin; and **the artifact's measured size at the cap** | T1–T13, T18 as library tests; T19; the equality test in the grounding crate; and the cap confirmed or lowered on a measured number |
| **S2** | **All three surfaces, in one slice** (decision #30): the `locate` subcommand and its exit codes; the fourth MCP tool with the two count assertions moved; `locate()` in both SDKs | T14–T17, T20; the CLI-equals-library test |
| **S3** | The measurements that are not the cap's: occurrence counts for a realistic quote over `fixtures/gate`, and the documents that move with them | Recorded in `docs/measurements/locate/`, with the worst document named |

**Why the core query lands before the surfaces.** S1's own tests are the ones that can be wrong in a
way no adapter test would catch — the offsets, the block boundary, the invariant — and an adapter
tested against a query that was never tested alone is an adapter whose test can pass because both
halves share a mistake. That is S1-before-S2's reasoning in `24-AUTO-TAGGING-MILESTONES.md` line 19,
applied here.

**Where the code lives, and why not elsewhere.** `ethos-parser-core`, as a new module beside
`markdown` and `html`. `locate` is a query over the representation with no PDF and no office concept
in it, which is the crate-boundary rule (`04-ARCHITECTURE.md` line 38), and it is the same reasoning
`run_markdown` records: *"it is a projection of the representation and has nothing to do with PDF, so
`ethos-parser-pdf` never learns Markdown"* (`main.rs:749-751`). **No fifth crate**
(`history/11-V11-MILESTONES.md` line 96–97). It is not in `ethos-parser-grounding`, which *"projects
the representation, never a document"* and whose output is the verifier's input — putting a text
search in the crate that feeds the verifier is the wrong neighbourhood for it whatever the code does.

**Documents and files that must move with S2**, because each states a subcommand count or a surface
that is about to change:

- `crates/ethos-parser-cli/src/main.rs:17-25` — *"Four subcommands at v0. Ten now"*.
- `crates/ethos-parser-core/src/verifier.rs:41` — *"the other nine subcommands"*.
- `crates/ethos-parser-cli/tests/diagnostics.rs:132` — *"ten subcommands map to five stages"*.
- `docs/PUBLIC-API.md:233` (*"The CLI's **ten** subcommands"*), `:251` (*"It maps five of the ten"*)
  and the mapping table at `:260-266`, which gains a `locate` row.
- `docs/04-ARCHITECTURE.md:19` and `:79` — the tree comment and *"binds all ten"*.
- `docs/README.md:7` — *"`tag`, the tenth subcommand"*.
- `docs/CAPABILITY.md` — a **Can** row, and the standing statement that `locate` binds no box
  tighter than a run.
- `docs/02-ROADMAP.md:29` — v2.3's row, marking `locate` built.
- `docs/OPEN-WORK.md:112` — item 6.7's status.
- `docs/draft-schemas/` — a `locations.draft.json`, beside the others.
- `CHANGELOG.md` — §8's draft.

**What does not move:** `ci/gate.sh` and `.github/workflows/ci.yml`. `cargo test --workspace`
(`gate.sh:134-135`) picks up new test binaries, and `ci/sdk-suites.sh` (`gate.sh:140-141`) runs both
SDK suites already; the gate stays at nine steps, so the parity test
`v0_exit_criteria.rs:881` needs nothing. Said explicitly because adding a check to one and not the
other is a failure this repository has a test for.

## 11. Tensions named

Four places where decision #30's bounds and the existing code or the handle law pull against each
other. None is resolved by wording; each is resolved by a choice above, and the choice is named.

1. **"Every place the string occurs" against "no invented adjacency."** The row promises *every*
   place; §4.1 refuses matches crossing a block boundary. Both cannot be maximal. The choice: the
   refusal wins, because a match across a block boundary is a match against a string the page never
   drew, and the artifact must therefore **declare what it searched** — `searched.blocks` and
   `searched.nodes` are on the wire for exactly that, so the promise is narrowed visibly rather than
   silently.
2. **The handle law's Mint obligation against there being no block id.** The handle law requires
   every locator a caller sees to have come out of an artifact the engine already emits. A block —
   the unit the match rule searches — has no id in the representation, and the grounding projection's
   `e{n}` is minted only for blocks that survived its omissions (`grounding/lib.rs:829-830`). So an
   occurrence cannot name the string it indexes. The choice: address by node ids and offsets into
   each node's own text (§5), which the record does mint, and let the join to a grounding element run
   through `Span.id` (`grounding/lib.rs:845-846`). Minting a block id here would be the second id
   space §7 refuses.
3. **Contract §2's blanket profile rule against "no existing artifact's bytes move."** Settled in
   §8: the rule id joins the profile, because the two nearest precedents and the contract's own
   sentence all point that way, and what "does not move" means is the *content* of every existing
   artifact — every `profile_sha256` string moves in every release regardless.
4. **The handle law forbids an `offset` *argument*, and `locate` returns offsets.** Not a conflict —
   the ban is on arguments a model could compose into a locator (`mcp.rs:847-850`) — but it is worth
   stating, because the first reading of the banned list looks like it forbids the answer. Returned
   offsets are minted; accepted ones would be composed. `Span.char_start` has been on the wire since
   0.58.0 on exactly that footing.

## 12. What this document settled, and the one question left

**Settled here, with what would reopen each** — none is a North Star reversal, and each follows a
rule the repository already has:

| | Settled | On what | Reopens on |
| --- | --- | --- | --- |
| `locate_rule` on `Profile` | yes | Contract §2's blanket rule and `markdown_rule`'s precedent (§8) | A contract change about what the profile covers |
| A page on an occurrence | no | #30 names boxes and not pages; a node states its own (§8) | A named consumer holding occurrences without the record |
| The cap | 1,000,000, provisional until S1 measures the artifact at it | `MAX_ELEMENTS`, re-declared in core (§4.3) | S1's own measurement, which may lower it |
| A non-BMP or combining-mark fixture | not authored; T10 and T11 are hand-built representation tests | The precedent at `history/11-V11-MILESTONES.md` lines 84–86 (§9) | Recorded as a standing item in `OPEN-WORK.md` §5, beside the `<mc:AlternateContent>` one, for the same reason: a unit test leaves the mutation suite and the digest lists unreached |
| A table cell's concatenated text | not searched by `-v1` | `geometric_blocks` makes a table-owned run standalone, so a cell's runs are one-member blocks (§4.1) | A caller that cites a cell by its text; the verifier resolves a `table_cell` claim through its table, so the two sides are furthest apart here and it is the next `-v2` candidate |

**The one question left, and it is already on the owner's list.** `02-ROADMAP.md` line 101 gives
every version a scope and a milestones document before code, and `OPEN-WORK.md` §4 carries the
standing question of whether v2.3 gets **one pair for the version** or **a scope per item that
changes the engine**. This document is written as the second — a per-item scope, numbered 26 after
`25-KNOBS-SCOPE.md`, with a milestones document of its own. If the owner takes the first, §3 to §9
become the `locate` section of v2.3's scope and the slices merge into its milestones; nothing in the
design changes either way, which is why the code is not waiting on the answer.

---

## Standing rules this document keeps

Repeated because a new query surface is where they get bent.

1. **No claim, no verdict, no `grounded`, no evidence tier** — `07-VERIFY-BOUNDARY.md`, and the grep
   is the proof rather than this sentence.
2. **No public confidence field**, on an artifact, a tool result or a summary string.
3. **No invented coordinate, identifier, fingerprint or page number.** Everything an occurrence
   carries is a copy of something the record already states, checked against it by a test.
4. **Typed absence, never a sentinel.** A node with no measured box carries its reason, not
   `[0,0,0,0]` and not a missing key.
5. **No silent drop and no silent repair.** A withheld occurrence list carries its count and its
   code; a quote is searched as the caller wrote it, byte for byte.
6. **A gap is never presented as a success.** A forged or edited representation errors; a string that
   occurs nowhere does not.
7. **Byte identity across surfaces.** The artifact an adapter returns is the artifact the CLI prints,
   and a test asserts it.
