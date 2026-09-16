# 23 — Auto-tagging scope: a tag this engine writes, and reads back as its own

**Scope authority for v2.2's second half.** Decision #23 of 2026-09-07 reopened auto-tagging and
named the shape its answer has to take: an attribute object under `/A` carrying a private owner in
`/O`, a tag that fills absence and never replaces one, and a writer that is its own subcommand.
This document settles everything #23 left to a scope document, and §12 lists what it asks the owner
to record before the milestones document below it is authoritative.

**Written 2026-09-16 against `main` at `b4b4aa9`, workspace 0.58.0, unreleased**, and revised the
same day after a three-lens review whose 23 confirmed findings are folded in below (the placement
rule, `/MarkInfo`, the decoder, the projections, the refusals and the measurement plan all moved).
Every number in it is measured on that build unless it names another. The milestones are
[`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md).

---

## 1. The one sentence

**The block cut already knows where the blocks are. This writes them into the file, marked as
computed, and reads them back as computed.**

Everything below is machinery for making the second half of that sentence true rather than hoped.

## 2. The gate clause, read closely

v2.2's second clause is *a tag this engine writes is one it can read back and ground against*.
Three verbs, and each is a test:

| Verb | What it means here | Where it is proved |
| --- | --- | --- |
| **writes** | The engine emits a PDF that carries a structure tree it built from its own `gutter-columns-v3` cut, and nothing an author declared is touched | `tag` on an untagged document; refusal on a tagged one |
| **reads back** | `extract` on that PDF binds every tagged run to the element that cites it, and the locator says the element was **`Computed`** — never author structure | `pdf_tagged.derivation` on every node; `structure-tree-engine-written` on the artifact, and `untagged-structure-tree-absent` not on it |
| **grounds against** | `ground` on that representation is an artifact the engine's checker and the pinned Ethos verifier accept, and a quote from it verifies exactly as it did on the untagged original | the round-trip test, relayed through `ethos verify` |

**What the clause does not say** is that another reader learns anything. #23 already states the
guarantee is engine-local, and nothing here widens it: a reader that has never heard of this engine
sees a `/Div` and reads author structure. §9 declares that on the artifact and §7 measures what it
costs.

## 3. What is written

### 3.1 The unit is the block, and only the block

One structure element per **`(page, region, block)`** of `gutter-columns-v3` — the band-block the
leading-gap cut placed each run in, as `TextRunAttributes.block` already says on the wire since
0.55.0. A band the rule declined (uniform body text, no gap over 1.6× its own leading) is one block;
a page the vertical cut did not divide is one band. So a page of plain prose gets exactly one
element, and that is the honest answer: the rule found one block.

**Why not the D4 region.** A region is a column band and says nothing about where text within it
begins and ends; tagging it would put a whole column in one element and the block cut, the only
finer measurement the engine has, would still have no consumer.

**Why not lines.** A line is a baseline the reader grouped by equality of `origin_y`. One element
per line would be a line number wearing a structure element's name — the failure
[`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md) §1 named for `horizontal_cut`,
moved into the file.

**The block is the consumer #21 removed and #23 restored.** Nothing outside tests reads
`TextRunAttributes.block` today ([`OPEN-WORK.md`](OPEN-WORK.md) §2.2). This is the consumer.

### 3.2 The roles: `/Document` above, `/Div` for every block, nothing else

The root element is `/Document`. Every block is a **`/Div`** — PDF 32000-1 §14.8.4.2, Table 333
(grouping elements): *a generic block-level element or group of elements*. That is exactly what
the cut measured: a block-level stretch of text bounded by whitespace, and nothing about what it
says.

**Not `/P`** (§14.8.4.3, Table 335, the paragraph-like elements). Writing `/P` asserts a
paragraph, and the rule finds **63.7% of real paragraph breaks at 100% precision** on the one
document that can carry a label ([`blocks.rs`](../crates/ethos-parser-pdf/src/blocks.rs)): a
block never splits a paragraph, and a missed break is a block holding more than one. #23 names the
P14 tension in as many words and leans on the attribute to hold it; this document does not lean on
the attribute where a role that claims less is available. This is written under P14 and decision
#19 as they stand. `/P` is what plan decision **D1** (roles inferred from geometry) would licence;
D1 is not taken here and is not asked for in §12 — if the owner takes it, the role constant moves,
`ethos.parser.tags.v0` bumps (§8), and §7.2's instrument becomes the re-refusal test.

**Not `/NonStruct`**, the row below `/Div` in the same Table 333, which the specification defines
by contrast with it: *a grouping element having no inherent structural significance* that a
conforming processor shall not interpret or export. That claims less than was measured: the cut did
find block-level layout, and a role that tells every consumer to skip it would hide a measurement
the engine stands behind. `/Div` is the entry of the grouping table that says *block* without
saying *what kind* and without telling a processor to suppress it.

No heading, list, table, caption or span is ever written. Those are roles, roles come from an author
or from nowhere (P14, decision #19), and this engine has no author's evidence for an untagged
document by definition.

### 3.3 The attribute, which is the whole of what #23 asked for

Every element the writer creates, the root included, carries one attribute object:

```
/A << /O /EthosParser /Derivation /Computed /Rule (gutter-columns-v3) >>
```

| Key | Value | Why it is there |
| --- | --- | --- |
| `/O` | `/EthosParser` | The owner, §14.7.5. A private name: Annex E asks such names to carry a registered prefix, and this engine registers none, so the name is a convention only this engine reads — which is what #23 means by *engine-local*, said in the file |
| `/Derivation` | `/Computed` | The contract's own class (`01-CONTRACT.md` §6), as a name. The reader requires this exact value under this owner; any other shape under `/EthosParser` is refused as malformed rather than read as author structure or ignored |
| `/Rule` | `(gutter-columns-v3)` | The rule whose cut the element is, as the profile spells it. A later writer under a later cut says so on each element, so a reader can name the rule without consulting the catalog |

The attribute travels **per element**, because #23 binds the row to it: *if the attribute is ever
dropped or ignored, that distinction is gone and this row goes with it.* §7.2 measures exactly that
drop. A single document-level stamp would not survive a consumer that copies elements. The writer
attaches it under `/A` only; the reader also accepts it through `/C` and the root's `/ClassMap`
(§4.1), because a tool that factors repeated attribute dictionaries into a class is the *ignored*
case #23 names, and the engine reads its own attribute wherever the specification lets it be placed.

One document-level record rides with it, on the catalog, the shape
[`overlay.rs`](../crates/ethos-parser-pdf/src/overlay.rs) already stamps:

```
/EthosParserTags << /ArtifactType (ethos.parser.tags.v0) /SourceSha256 (sha256:…)
                    /ProfileSha256 (sha256:…) /ParserVersion (0.59.0) >>
```

It says which bytes the tags were computed from and under which profile. It is provenance, not the
derivation: the reader never consults it to decide what an element is.

### 3.4 Marked content: sequences that hold this block's text and nothing that draws

Each block's text-showing operators are wrapped in marked-content sequences
`/Div << /MCID n >> BDC … EMC`, and the block's element lists every sequence it owns in `/K`.

**What a sequence may enclose.** Two kinds of operator, and no third: text-showing operators
(`Tj`, `TJ`, `'`, `"`) whose runs the cut placed in this block, and operators that draw nothing —
graphics state, text object, text state and positioning, colour, path construction, clipping
(`W`, `W*`, `n`), marked-content points (`MP`, `DP`) and compatibility markers. A sequence ends
before anything else, by name: a text-showing operator that produced no run of this block (another
block's run, a run the reader dropped under `broken-font-encoding`, an empty run), any painting
operator (`Do` for any XObject, `BI … EI`, `sh`, and `S`, `s`, `f`, `F`, `f*`, `B`, `B*`, `b`,
`b*`), and any `/Artifact` sequence. So text inside a form XObject, an inline image, a figure or
page furniture is never inside a written sequence, and a block interrupted by one of them becomes
two sequences.

**Existing marked content is a barrier, never something a sequence contains.** The reader binds a
run by the id of the **innermost** open sequence
([`content.rs`](../crates/ethos-parser-pdf/src/content.rs) `current_mcid`), so text inside an
existing id-less sequence — an optional-content layer `/OC /oc0 BDC` given by name, a `/Span`
with `/ActualText` — that a written sequence merely enclosed would read back with no id and never
bind. A written sequence therefore opens inside the innermost existing sequence that holds the
block's operator and closes before that sequence's `EMC`; a block whose operators sit in two
existing sequences gets two written ones. Nothing straddles a text object, a saved graphics state
or an existing sequence, whichever way the specification's nesting rule is read.

**As wide as those rules allow, and no wider.** A sequence is first placed at the depth of the
block's operators (`BDC` immediately before the first, `EMC` immediately after the last) and then
widened outward step by step — to the enclosing `BT … ET`, then to a `q … Q` that encloses exactly
that — as long as everything newly enclosed draws nothing, belongs to no other block and sits
inside the same existing sequence. Two such sequences of one block separated only by non-drawing
operators merge into one. A block written as one `BT … ET` per line therefore becomes one sequence,
and so does a block whose lines share one text object; a text object shared between two blocks is
split at the operators, inside it.

**Why not one sequence per operator.** It would be legal and simple, and it would destroy the
projections. [`markdown.rs`](../crates/ethos-parser-core/src/markdown.rs)'s block rule joins two
runs either because *the document declared them one sequence* or, where neither run carries a
declaration, by ink along one baseline. A run carrying a declaration the previous run does not share
breaks. One id per operator therefore projects an engine-tagged page as one Markdown block per
`Tj` — the 68 112-block defect of 0.44.0, reintroduced by the writer. §4.3 says how the projections
read a written sequence; §7.1 records how many sequences each block needed and why, so the cost of
these rules is a number and not a guess.

**Why not one sequence per block, always.** A block's operators are not always contiguous in the
stream, and two sequences may not share one id: the parent tree indexes a page's ids by position,
so an id names one sequence.

**Ids are per page, dense from zero, in stream order.** That is what `/ParentTree` indexes by. Each
page that received a sequence gets `/StructParents`, the root gets `/ParentTree` and
`/ParentTreeNextKey`. Every element carries `/Type /StructElem`, `/S`, `/P`, `/Pg` where it has
content, and `/K`. The tree is the shape §14.7 describes, so a reader that does consult the parent
tree finds what it expects; this engine's own reader walks `/K` and needs none of it.

**No `/MarkInfo`.** `/Marked true` is the Tagged PDF conformance claim (§14.8.1), and the file the
writer emits does not meet it: paths, images, form XObjects and dropped runs are neither in the tree
nor marked `/Artifact`. The writer adds no `/MarkInfo`, leaves one the catalog already carries
byte-for-byte as found, and the reader never reads the key — it looks for `/StructTreeRoot` and
nothing else, which a test pins.

**What is left outside a sequence, and why.** A run the page marked `/Artifact` — page furniture
with no tree, which a producer can write — stays outside: §14.8.2.2 puts artifacts outside the
structure, and the author said so. A text-showing operator whose run the reader dropped stays
outside: the engine did not read it, so it does not claim it. Text inside a form XObject is not
descended by this profile and is not tagged; the artifact already declares that. A whitespace-only
run is a run, is in a block, and is tagged like any other.

### 3.5 The bytes: decoded strictly, spliced, never re-encoded

**The page's content is read by the writer itself, not through `get_page_content`.** That helper
appends a stream's still-encoded bytes when a decode fails and skips a `/Contents` entry that does
not resolve to a stream, silently either way. The writer resolves `/Contents`, refuses an entry
that is not a stream, decodes each stream with a decoder that surfaces failure — `FlateDecode`
through the writer's own inflate, refusing a stream whose decode does not run to the end of its
input or whose check fails; `LZWDecode` and `ASCII85Decode` through `lopdf` with its error
surfaced; any other filter refused by name — and joins the streams with the same `\n` separator
`get_page_content` uses, so the buffer is byte-for-byte the one `extract` interpreted and the
operator indices below mean the same thing on both sides. The separator bytes are therefore
emitted inside the single stream the page ends up with; that is equivalent to the document's own
content only under §7.7.3.3's rule that a stream boundary falls between tokens, and a producer
that breaks it tokenises differently in the two views.

**Tokenised with byte positions, and every byte accounted for.** The writer's tokeniser mirrors
what `lopdf::content::Content::decode` accepts — numbers, names, strings with nested parentheses
and escapes, hex strings, arrays, dictionaries, comments — and an inline image `BI … ID … EI` is
one opaque token whose data ends by the same rules `lopdf` applies (the computed byte length for
unfiltered data in a colour space it knows, otherwise the first whitespace-`EI`-whitespace window),
so the two resynchronise at the same byte by construction and no `BDC`/`EMC` ever lands inside
image data. Two conditions hold before any tag is written, and a page that fails either is refused
by name: the tokeniser places every byte of the buffer in a token, and its operator sequence —
count and names in order — equals `lopdf`'s decode of the same bytes. `lopdf`'s decoder is lenient
and stops silently at the first operation it cannot parse; together the two conditions prove it
parsed the whole page, which is what makes the operator indices trustworthy. §3.7 cannot catch
this later, because the re-extraction uses the same lenient decoder over bytes the splice
preserved and would drop the same tail.

**Inserted, never re-encoded.** The `BDC` and `EMC` operators are inserted at token boundaries;
every byte of the content the document wrote survives unchanged. The alternative — decoding to
operations and encoding them back — would send every operand through `lopdf`'s writer, whose
reals are `f32` and whose string and name escapes are its own, and a coordinate that moved by a
last bit is a coordinate the engine changed while claiming to have added structure.

**The page ends up with one new stream.** The writer builds it itself: `lopdf::Stream::new` on a
dictionary carrying `/Filter /FlateDecode`, deflated in the writer at a fixed level, and the page's
`/Contents` set to a reference to it. `change_page_content` is not used, because it mutates the
referenced stream in place when `/Contents` is a single reference and a stream two pages share
would then carry the last page's bytes for both; `Stream::compress` is not used, because it leaves
a short stream raw. After every page is rewritten, the superseded stream objects that no other page
references are removed from the document's object map directly — not with `delete_object`, which
walks every object per call, and not with `prune_objects`, which would also drop the document's
pre-existing orphans, more than §9 promises.

**Every other object is re-serialised.** The whole document is then written once from a fresh
clone, as the overlay does and for the reason its module header gives: `lopdf` mutates what it
saves, so byte identity holds per fresh copy and every build takes one. That write re-encodes
every object the writer did not touch: a real is narrowed to `f32` and reprinted shortest-round-trip
(`595.303937007874` becomes `595.30396`), names and literal strings are re-escaped, untouched
streams keep their filters and bytes. The text record is unaffected — `extract` reads page boxes
and matrices through the same `f32` on both sides, so the origins it compares are the same numbers
— and for the same reason §3.7 cannot detect the narrowing; it is declared (§9) and measured
(§7.1: on the 54 PDFs the repository holds, 27 202 reals outside content streams, none changed at
0.58.0's precision, measured during review on 2026-09-16).

**Not an incremental update.** PDF §7.5.6 would let the original bytes stand as a prefix, with only
the page dictionaries, the new content streams, the tree, the catalog and the trailer appended, and
every untouched object byte-identical by construction. The cost is the superseded content streams
left in the prefix, not the fonts or images, so the file does not double. It is refused for now on
one ground only: `lopdf`'s incremental writer is the less exercised of its two, and the full
serialisation is the path the overlay has proved across the corpora. It reopens if §7.1's real
narrowing count is non-zero on a producer that matters, or if a consumer needs the untagged bytes
addressable inside the tagged file; `SourceSha256` on the catalog stamp records what the tags were
computed from either way.

### 3.6 What is refused, each by name

| Input | Answer | Why |
| --- | --- | --- |
| A document with `/StructTreeRoot` | refused | #23: a written tag fills absence only. That includes a tree that reaches only some of the content: filling a partial tree means choosing where under the author's elements the new ones go, and every such choice invents a relation the author did not state. Reopens with a measured case where the untagged remainder of a tagged document is worth more than the risk of misfiling it |
| A run bound `pdf_mcid` — an inline `/MCID` in the content stream, and no tree | refused | The ids already have a meaning the document lost; writing another id inside the same sequence, or reusing one, would either shadow it or claim the parent tree already indexes it. Reopens with a corpus count showing this shape is common |
| A `BDC` whose property list is a name that resolves, through the page's `/Properties` (inherited through `/Parent`), to a dictionary carrying `/MCID`; or a name that resolves to nothing | refused | The reader declares such lists `mcid-property-list-by-name` and does not resolve them, so the artifact cannot say whether an id is there. The writer resolves the name itself before writing: an id found is the row above, and a name that resolves to nothing is an id unknown rather than absent. A named list without `/MCID` — an optional-content layer — is not refused; it is an existing sequence under §3.4 |
| An encrypted document | refused | As every subcommand refuses it today (`EngineError::Encrypted`) |
| A `/Contents` entry that is not a stream; a stream under a filter other than none, `FlateDecode`, `LZWDecode` or `ASCII85Decode`; a stream whose decode does not run to the end | refused, naming the page and the filter | §3.5. The reader's raw-bytes fallback cannot be relied on to fail: a corrupt Flate stream decodes partially and is accepted |
| A page whose bytes the writer's tokeniser cannot fully account for, or whose operator sequence `lopdf`'s decoder reads differently | refused, naming the page and the first differing operator | §3.5; an inline image whose data contains a false `EI` window is a known cause |
| A document whose re-extraction differs from the source's, or whose tagged runs do not all bind | refused, naming the first differing run | §3.7 |

No page budget: a tag written under `--max-pages` would leave a tree that reaches some pages, and
the document would then be refused by the first row above for the rest of its life. The writer
reads the whole document or none of it.

### 3.7 The writer checks its own output before emitting it

After serialising, the writer opens its own bytes, runs `extract` on them, and compares against the
extract it wrote from:

- every page, in order, the same sequence of runs — text, origin, advance, font size, codes,
  synthesized characters, region and block equal, run for run;
- every run the writer put in a sequence now binds as `pdf_tagged` with `derivation: computed` and
  role path `Document/Div`; every run that was an artifact is still an artifact;
- the per-page counters agree — inline images, form XObjects not descended, unresolved XObjects,
  property lists by name, dropped runs — so a drift in where the two tokenisations resynchronised
  is caught even where no run text moved;
- the artifact declares `structure-tree-engine-written` and not `untagged-structure-tree-absent`,
  and declares neither `structure-mcid-unbound` nor `structure-item-without-content`; every other
  limitation is the source's;
- on the tokenised output, no written sequence encloses a painting operator, a text-showing
  operator of another block or of no block, an `/Artifact` sequence or a boundary of an existing
  sequence — §3.4 proved on every document, not on the fixtures alone.

A difference is a refusal, with the first differing run or the failing condition named. This is the
gate clause's *reads back* verified on every document rather than on the fixtures, at the price of
a second extraction. The cost is accepted: the writer is not on any hot path, and a tagged PDF that
this engine could not read back as its own is the one artifact this feature must never emit.

## 4. What is read back

### 4.1 `structure.rs` reads the owner, under `/A` and through `/ClassMap`

`attribute_dicts` today reads `/A` only to find `/RowSpan` and `/ColSpan` on cells, resolves no
indirect reference, and is never consulted for a `/Div` or `/P` — the North Star's *already parses
this exact shape* was two of three ([`OPEN-WORK.md`](OPEN-WORK.md) §7). It now reads, on every
element, the attribute objects under `/A` — the dictionary, the array, and the array with revision
numbers, following an indirect reference to any of them — and the ones reached through `/C`, a name
or an array of names resolved through the root's `/ClassMap` with the same lookup `read_role_map`
already performs. Over that union one predicate applies: an object whose `/O` is `/EthosParser`
marks the element engine-written, and under that owner `/Derivation /Computed` is required; an
object this engine owns in a shape it does not write is refused as malformed, on the same
fail-closed grounds a cycling `/K` is. An `/EthosParser` object under `/A` decides; one reached
only through `/C` decides when `/A` carries none; a class name absent from `/ClassMap` contributes
nothing.

A binding made under an element that carries the owner attribute is **`Computed`**; every other
binding is **`Extracted`**, as every binding has been since v1-S3. The innermost element decides —
the one whose `/K` cites the content — so a computed `/Div` under an author `/Document` would read
as computed and an author `/P` under a computed `/Div` as author. Neither shape is written today
and both are declared rather than smoothed.

The rule that decides which runs come out bound, and now what class the binding carries, moves
from `struct-tree-v1` to **`struct-tree-v2`**; `profile_sha256` moves with it.

### 4.2 The wire: `derivation` on every `pdf_tagged` locator, and one document-scoped declaration

`PdfTaggedLocator` gains **`derivation: DerivationClass`**, written on every tagged locator of
every document — `extracted` for author structure, `computed` for this engine's own. Decision #20
settles the shape: a per-node value the contract requires the artifact to state is spelled out even
where it is constant, and omitting it to restore `extracted` at read time would build the field #20
names as somebody else's bug, defaulting to the highest-trust class. The cost is one key on every
tagged run of every tagged document; §7.1 measures it on the gate corpus, and #20 accepted 18% for
four such fields on the same argument.

The artifact also declares, as a document-scoped limitation, **`structure-tree-engine-written`**:
how many elements carry the owner attribute, how many runs bound under them, the rule names the
attribute carries, and the fact its detail exists to state — the input carried no author structure
tree, so every role path in this artifact is this engine's own cut read back and none is the
author's. It is a disclosure in the limitation slot on the precedent of
`tagged-table-without-geometric-table` (v2-S24), and like that code it still names something
missing: an author's structure. **`untagged-structure-tree-absent` is not declared** on such a
document: its detail says the catalog declares no `/StructTreeRoot` and no role path exists, and
both would be false; `extract` emits it only when `structure::read` finds no tree, and that stays
as it is.

`Node.derivation` on a text run stays **`Extracted`**. The text was read from the document's own
encoding; what was computed is the address, and the address says so. `may_be_overwritten_by` is
untouched, which is what closes row #21: the laundering it feared was a computed address reading as
an author's, and the address now carries its class on the wire.

### 4.3 The projections read a written sequence as no declaration at all

[`markdown.rs`](../crates/ethos-parser-core/src/markdown.rs), `html.rs` and the grounding
projection group runs by marked-content sequence, on one licence: *the producer said these runs are
one thing*. On an engine-tagged document the producer of the sequence is this engine, and the
licence does not transfer. v2.2-S5 refused to weld runs across a baseline on geometry alone and kept
the seam between geometrically joined runs addressable; a written sequence read as a declaration
would reach the same weld through the file, and the anchor map — which carries node ids and no
derivation — could not say so. So `group_key` treats a `pdf_tagged` locator whose `derivation` is
`computed` exactly as it treats a run with no declaration: the undeclared path applies, and the
Markdown, the HTML and the grounding elements of an engine-tagged document are those of its
untagged original, byte for byte apart from the assurance block. A test holds that equality.

This is also what keeps the grounding honest. An element's text is its members' text concatenated
verbatim, and a block whose lines end without a space glyph would read `R1R2R3` as one element while
its Markdown reads `R1 R2 R3`; a quote taken from the Markdown would then fail to ground on the
tagged document and ground on the untagged one. Leaving the elements as they are keeps every
existing verify result exactly as it was, which is what *grounds against* promises.

The projection rule ids do not move: no input that existed before this change projects
differently, and the new input class projects as its untagged twin. What the tag adds for a
consumer is the read-back of §4.2 and the tree in the file; what it adds for the projections is
nothing, on purpose.

## 5. What this does not do

- **No role from geometry.** No `/P`, `/H1`, `/L`, `/Table`. P14, decision #19, and the
  *not scheduled* table in [`02-ROADMAP.md`](02-ROADMAP.md). D1 is untouched (§3.2).
- **No PDF/UA, no Tagged PDF conformance, no accessibility studio.** A `/Div` tree with a private
  attribute is not a conformance claim, and the writer sets no `/MarkInfo`, no `/Lang`, no
  `/ViewerPreferences` and no XMP flag.
- **No tagging of a tagged document, and no partial fill.** §3.6.
- **Not a side effect of `extract`.** #23. `extract` never writes a byte of the document.
- **Not exposed over MCP or the SDKs.** Six of the nine subcommands are not, `overlay` among them,
  and the writer is the one whose output is a document rather than an artifact. An MCP tool is
  model-controlled, and a tool that emits a mutated copy of a document on a model's say-so is a
  different hazard class from one that reads. Plan §8's rule that every knob needs an MCP and SDK
  line is met by this line: not exposed, on purpose, until an owner decision says otherwise.
- **Not `--max-pages`.** §3.6.

## 6. Where it sits

| Surface | Shape |
| --- | --- |
| CLI | `ethos-parser tag <pdf>` — stdout is the tagged PDF, the second subcommand after `overlay` whose stdout is not canonical JSON. Exit **0** written, **2** could not read or refused. The tenth subcommand |
| Library | `ethos_parser_pdf::write_tags(&Document, &Profile) -> Result<Vec<u8>, EngineError>` and `TAGS_ARTIFACT_TYPE`, frozen in [`PUBLIC-API.md`](PUBLIC-API.md) with a thin-shell row. The writer runs extraction itself, as `overlay` does in-process, because the artifact does not say which operator showed a run: that mapping — `(page, operator index in the joined buffer)` per run, surviving `reorder_page` — lives in a crate-private side table from a crate-private extraction entry point, never on `TextRun`, so `ExtractArtifact` does not change and no artifact parsed from JSON can reach the placement rule |
| Reader | `structure.rs` and the wire field of §4.2; `ethos_parser_pdf::limitations::structure_tree_engine_written`; `codes::STRUCTURE_TREE_ENGINE_WRITTEN` |
| Profile | `struct_tree_rule: struct-tree-v2` |
| Fixtures | Hand-written PDFs in the exact shape the writer emits, so the reader is tested independently of the writer and a writer regression cannot hide a reader one: `engine-tagged-blocks` (the attribute under `/A`), a `/ClassMap`-only twin (`/C` on every element, the class on the root), a mixed twin (`/A` holding a foreign owner such as `/O /Layout`, `/C` carrying the engine class), and a page whose block has one line inside `/Span BMC … EMC` and one inside an optional-content sequence given by name. For the writer's refusals: an untagged page with `/P /MC0 BDC` whose `/MC0` carries `/MCID`; two pages sharing one content stream through a single reference |

## 7. What is measured before the row closes

### 7.1 The round trip, on every untagged document the repository can reach

Instrument: `docs/measurements/auto-tagging/roundtrip.py`. Over the untagged engine fixtures, the
untagged oracle fixtures, and the 200 `opendataloader-bench` documents:

- documents tagged, and documents refused, by reason — every row of §3.6 counted, so the cost of
  each refusal is a number;
- elements written, sequences written, and the distribution of sequences per block bucketed by
  cause — text object shared with another block, graphics-state nesting, an existing sequence, a
  painting operator, a foreign or dropped run, an artifact — with the worst document named; the
  in-repository corpus cannot measure the existing-sequence cause (no untagged PDF in it carries a
  `BMC`/`BDC`), the bench corpus can;
- property lists by name resolved per document, and how many resolved to an id;
- documents carrying `inline-images-not-emitted`, and of those how many tagged and how many refused;
- bytes added to the document;
- reals outside content streams that do not survive the `f32` round trip, counted by key
  (`/MediaBox`, `/CropBox`, `/Rect`, `/BBox`, `/Matrix`, `/FontMatrix`, `/Widths`), from a raw
  token scan of the source bytes, with the worst producer named;
- the self-check's verdict on every document, which is the *reads back* clause counted;
- Markdown, HTML and grounding artifacts of the tagged document against the untagged original —
  identical apart from the assurance block, which is §4.3 counted;
- `ground` then `grounding-check` on every tagged representation, and `ethos verify` on a claim
  quoting one run of the first block of each of a named subset, the same claim against the
  untagged original, the two reports equal.

Also the cost of §4.2 on the eight gate documents: artifact bytes at 0.58.0 against the same
documents with the field, since every one of them is tagged.

### 7.2 #23's re-refusal condition, made measurable

#23 is re-refused by *a measured case where a consumer treats an engine-written `/P` as author
structure and produces a citation the document does not support.* The role is `/Div`, the consumer
is named, and the two halves are measured separately.

**The consumer that ignores `/A` is the previous release.** `ethos-parser 0.58.0 extract`, under
`struct-tree-v1`, reads `/A` only for spans on cells and never on a `/Div`; run on the writer's
output it binds every engine-written element as `pdf_tagged` on an `Extracted` run — the launder
#21 refused, produced by a shipped reader with no test-only flag. That demonstration runs on any
untagged fixture and is the first half.

**The misread rate needs no twin.** Every written `/Div` is exactly one `(page, region, block)`,
§3.7 verifies region and block equal run for run on the writer's output, and the cut never consults
an mcid — so the rate at which a `/Div` would span an author's paragraph is fully determined by the
untouched original. On `nist-sp-800-207`, the one document with real `/P` labels, one `extract` of
the document as shipped gives `(page, region, block)` and the `pdf_tagged` mcid on every run; the
author's `/P` elements' `/K` ids per page come from `qpdf --json` of the tree; the join on
`(page, mcid)` gives (i) the share of blocks whose runs fall under two or more `/P` elements and
(ii) the share of `/P` elements whose runs fall in two or more blocks, worst page named. The first
is the rate at which a consumer reading `/Div` as a paragraph would cite across a boundary the
author drew; [`blocks.rs`](../crates/ethos-parser-pdf/src/blocks.rs)'s 63.7% recall at 100%
precision on 135 boundaries already bounds it, and the instrument re-aggregates that measurement
per block rather than re-deriving it. No tree-stripped twin is tagged: such a twin still carries the
author's `/MCID` operators and is refused by §3.6's second row, and exercising the writer on it
would need a content-stream rewrite instrument this document does not ask for.

A consumer that reads `/A` is this engine, and §3.7 proves that path on every write.

## 8. Identity

- **`struct-tree-v1` → `struct-tree-v2`** on the profile: the reading rule changed, it reports a
  class it did not report before, and `profile_sha256` moves.
- **`ethos.parser.tags.v0`** is the writer's artifact type, stamped on the catalog, and is what
  moves when the written shape changes — the role, the attribute keys, the placement rule. The
  writer's rule is not a profile field, on the overlay's precedent: the profile names what the tags
  were computed from (`ProfileSha256` on the stamp), the artifact type names the written shape.
- **`structure-tree-engine-written`** is a new `codes` constant and a wire addition, alongside
  `pdf_tagged.derivation`.
- `reading_order_rule` does not move: the cut is unchanged, and the writer reads its output. No
  projection rule id moves (§4.3).
- **A representation from either side of this change is refused by the other.** `PdfTaggedLocator`
  denies unknown fields and, by decision #20, `derivation` carries no default: a 0.58.0 build refuses
  a representation that carries the field (`unknown field derivation`), and this build refuses a
  0.58.0 one that binds any run to a tree (`missing field derivation`), from the CLI as
  `Malformed: representation` and from every MCP tool that takes a representation by path, before
  fingerprint verification, because parsing precedes it. Office and untagged-PDF representations
  carry no such locator and are unaffected. `schema_version` stays at 0.6.0, on the precedent of
  0.55.0 and 0.58.0 — a MINOR whose wire change is named in the release note and refused by the
  parser, not a shape bump — and the release note says this as 0.58.0's said it for
  `not_axis_aligned`.
- **MINOR**, by [`RELEASING.md`](RELEASING.md) §4: a reader changes, a wire field is added, every
  tagged artifact's bytes move. It lands in the version after 0.58.0.

## 9. Declared limitations

Stated on the artifact where an artifact exists, and here where the output is a document:

1. **The guarantee is engine-local** (#23). A reader that does not read `/A` sees author
   structure. `structure-tree-engine-written` is the artifact's statement; the file's is the
   attribute itself, which such a reader has already skipped. §7.2 names the previous release as
   that reader.
2. **A block is not a paragraph.** 63.7% recall at 100% precision on one document, no indent
   branch; carried into the file as `/Div`, never `/P`.
3. **A block may be several sequences**, split at a shared text object, an existing sequence, a
   painting operator, a foreign or dropped run or an artifact. §3.4, measured in §7.1.
4. **Refused whole, by name:** a document with a tree, a document with marked-content ids and no
   tree — inline, or by name through `/Properties` — a filter outside the three the writer decodes,
   a stream that does not decode to its end, a page the tokeniser cannot fully account for. §3.6.
5. **The output is a re-serialisation.** No byte of any content stream is altered and the text
   record is proved unchanged on every run; every other object is re-encoded by `lopdf` — reals
   narrowed to `f32`, names and strings re-escaped — and the page's streams become one
   `FlateDecode` stream carrying the `\n` separators the concatenation added. Declared, and
   measured in §7.1; not checked by §3.7, which reads through the same `f32`.
6. **Not a Tagged PDF.** A structure tree inside an otherwise untagged file: no `/MarkInfo`, and
   content outside the sequences is not marked `/Artifact`.
7. **Representations from the previous release are refused by this build's parser, and vice
   versa.** Re-extract under the current build. §8.

## 10. Slices

The milestones document carries them. In one line each: the reader, the wire field and the
hand-written fixtures; the writer, its decoder and tokeniser, its self-check and the `tag`
subcommand; the round-trip and mutation tests; the measurements of §7 and the documents.

## 11. Standing rules, carried forward

1. **A block is where, never what.** P14 and decision #19, now inside the file as well as on the
   wire — and the projections read it as nothing at all (§4.3).
2. **Nothing is written into a PDF that cannot declare how it was derived** —
   [`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md) §8 rule 5, and the attribute is
   that declaration, read wherever the specification lets it be placed.
3. **A written tag fills absence only, and writing is its own subcommand.** #23's two constraints.
4. **No byte of any content stream is altered, and the text record is proved unchanged on every
   run**; every other object is re-serialised, and §9 says what that changes.
5. **Byte identity across runs**, for the tagged PDF as for every artifact.
6. **Every number ships as a band with the worst document named** (decision #18).

## 12. What this document asks the owner to record

None of these is decided by this document; each is proposed as a North Star row, numbered from #25,
and the milestones stand under the assumption that they are taken as written here.

| Proposed row | What it records |
| --- | --- |
| **#25** | This scope: owner `/EthosParser`, the attribute of §3.3 read under `/A` and `/C`, `/Div` under `/Document`, the block as the unit, the placement rule of §3.4, strictly decoded and spliced bytes with one serialisation, `pdf_tagged.derivation` plus `structure-tree-engine-written` on the wire, `struct-tree-v2`, the projections reading a written sequence as no declaration, and the parse break of §8 |
| **#26** | The writer is not exposed over MCP or the SDKs until an owner decision says otherwise (§5) |
| **#27** | The local plan's three v2.2 "done" conditions, reconciled with the roadmap's two clauses: (1) *a refused table candidate emits something a retrieval pipeline can use* is retired on the plan's own §5.3 T2 finding that there is nothing useful for a refused candidate to emit; (2) *TEDS re-measured and published as a band* is an instrument under decision #18, run at 0.58.0 (`docs/measurements/opendataloader-bench/`), not a gate; (3) *the block cut has a consumer* is met by this scope (§3.1). The roadmap's two clauses are the whole gate, and clause two is met when §3.7 holds on the fixtures and §7's numbers are published — or the plan's definition is retired, which comes to the same |

D1 stays where [`OPEN-WORK.md`](OPEN-WORK.md) §4 holds it, as its own pending decision; §3.2 says
what moves if it is taken. One question this document leaves open on purpose, because a
measurement should answer it: whether §3.4's split rate on real producers is low enough that no
second placement strategy is worth its code.
