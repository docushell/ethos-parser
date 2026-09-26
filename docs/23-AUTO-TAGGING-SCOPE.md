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

**Since this was written.** The body stands as written and revised on 2026-09-16. Below, dated,
are what building the writer changed, what its two reviews found, and each number §7 asked for.
The body is not rewritten; read it with these.

- **§3.4, the placement rule as built (S2, 2026-09-16; recorded on review 2026-09-17).** The code
  is the rule, and it differs from this document's wording in two places. First, a sequence opens
  before the text-positioning operators (`Td`, `TD`, `Tm`, `T*`) that immediately precede its
  first text-showing operator, not immediately before that operator, so the first line's position
  sits inside the sequence (`engine-tagged-blocks` was written that way). Second, the `q … Q` step
  widens over any operators between the pair and the text object that draw nothing, not only over
  a pair that encloses the text object exactly: `q 0 g BT … ET Q` widens. Neither change encloses
  anything that draws. `plan_page`'s rustdoc states the rule as built.
- **§3.4, what is left outside: one exception, as built.** *A text-showing operator whose run the
  reader dropped stays outside* holds when every run the operator showed was dropped. A `TJ` that
  showed one kept string and one dropped string is enclosed with the kept run's block, for three
  reasons: an operator is the smallest thing a sequence can hold, the splice inserts only at token
  boundaries, and leaving the operator outside would leave its kept run unbound, which §3.7
  refuses. The dropped string's glyphs therefore sit inside the sequence. The dropped run itself is
  still not claimed: it is in no artifact, and `broken-font-encoding` still counts it.
- **§3.5 and §3.6, the filters (narrowed while building S2, 2026-09-16).** `LZWDecode` and
  `ASCII85Decode` are refused by name, like every filter other than none and `FlateDecode`. They
  are not decoded through `lopdf` with its error surfaced, because in `lopdf` 0.44.0 there is no
  error to surface: it returns the partial output of an LZW stream it could not decode as a
  success, and it decodes an ASCII85 stream missing its `~>` end marker anyway. A `/DecodeParms`
  carrying a predictor is refused on the same ground. Row 5 of §3.6 and §9 item 4 therefore read
  *a filter other than none or `FlateDecode`*. Over the 293 documents of §7.1, no document was
  refused for a filter or a stream that does not decode to its end.
- **§3.5, removal and numbering (tightened 2026-09-17, `8500ab3`, and on its review the same
  day).** A superseded stream is removed only when *no object* references it, not merely no page:
  a stream something else names is kept, and a `/Contents` array given by reference goes with its
  streams. And no object the writer adds takes a number that a reference in the source names and
  the file does not hold. §7.1's first run found why. `form-orphan-widget`'s widget names
  `/Parent 9 0 R` in a file holding objects 1 to 7, the writer's second new object took number 9,
  and the widget's parent became the tree's root. The self-check refused that output because a
  limitation moved.
  - **The first fix numbered every new object above the highest number any reference names.** Its
    review showed that one dangling `/X 10000000 0 R` then produced a cross-reference table ten
    million entries long. qpdf 12.3.2 read that output's page as blank, Ghostscript 10.06 could not
    open it, and this engine's own reader, and so the self-check, read it correctly.
  - **As built,** the writer numbers from the highest object the file holds, counting object-stream
    members the cross-reference table never lists. It raises that floor only past a dangling
    number its own allocation would reach; a dangling reference still reads as null (§7.3.10), and
    the fixture tags.
- **§3.6, two refusals the table did not have.** Both are named in their messages and counted in
  §7.1 below; read §9 item 4's list with both added.
  1. *A text-showing operator whose runs the cut placed in two blocks*, added while building S2. A
     `TJ` is one operation holding as many runs as it has strings. When its strings straddle a
     gutter it is an operator for two elements, and a sequence holds operators for one. The
     refusal names the operation and both blocks.
  2. *An object carrying `/StructParents` or `/StructParent` with no tree*, added on review
     2026-09-17 (`8500ab3`); a null value counts as no key. Either key indexes a `/ParentTree` (§14.7.4.4), and the tree the
     writer adds would answer it with elements that do not hold that object's content: a page
     left unrewritten with `/StructParents 0` would name the first rewritten page's ids. The
     document is refused before anything is read, naming the object and the key. It was measured
     before it landed, over the 293 documents: 54 carry either key; the 48 with no tree were all
     refused already by row 2, and the other 6 carry a tree.
- **§3.7, the self-check as built (widened on review 2026-09-17, `5c26a72`).** Until then the
  per-page counters of the third bullet reached the check only as document totals inside five
  limitations. They are now compared page by page, together with each page's box, rotation, and
  image, table, tagged-table and object records. The run comparison also cannot see an id filed
  under the wrong `/Div`: every written element reads back as `Document/Div`, computed, with no
  identity of its own. So the check now also walks the written tree against the plan: one
  `/Document`; one `/Div` per planned block, on its page, citing exactly its ids; and a parent tree
  that maps every id, under the page's `/StructParents` key, to the `/Div` citing it. Since that
  commit's own review, it also checks each element's `/Type` and its `/P`.
- **§4.2, the wire cost (measured 2026-09-17).** On the eight gate documents, branch against the
  0.58.0 release build, same version string, `derivation` costs 3.2% (`irs-fw9`) to 4.7%
  (`nist-sp-800-218`) of the artifact, median 4.3%: 25 bytes per `pdf_tagged` locator, and 41.2 MB
  of `nist-sp-800-53Ar5`'s 1.04 GB. The rest of each difference is the constant 1,425 bytes of the
  `block-subdivision-leading-gap-only` declaration. Table in
  [`measurements/auto-tagging/README.md`](measurements/auto-tagging/README.md) §2.
- **§7.1, the round trip (measured 2026-09-17)**, with the writer at `2e70eba` and re-run identical
  apart from timings at `0c7a3c9`, over 293 documents:
  the 58 engine fixtures, the 35 oracle fixtures and the 200 `opendataloader-bench` documents.
  Instrument [`measurements/auto-tagging/roundtrip.py`](measurements/auto-tagging/roundtrip.py);
  method, tables and per-document results in its README §2.
  - **Outcomes.** 129 tagged: bench 54, engine 44, oracle 31. 164 refused:
    - row 2: 99;
    - the stale-key refusal above: 48, all bench, all also carrying row 2's inline ids;
    - row 1: 12;
    - row 3: 1;
    - row 4: 1;
    - no text to tag: 1;
    - not a PDF `extract` opens: 2.

    No document was refused under row 5, row 6, the two-block operator or the self-check (row 7).
    All 146 refused bench documents are PyPDF2 page splits that kept their marked-content ids and
    lost the tree. That is §3.6 row 2's reopening count, and it is the owner's to weigh.
  - **The tree.** 582 elements (129 `/Document`, 453 `/Div`) and 776 sequences. 74 of 453 blocks
    (16.3%) are more than one sequence; the most in one block is 60, on `bench/01030000000199`.
    §7.1 asked for these bucketed by cause. The writer holds the cause in its plan and prints it
    nowhere, so only the distribution is counted, and §12's open question is answered with that
    distribution rather than with causes.
  - **Property lists and inline images.** 15 originals declare `mcid-property-list-by-name`: 9
    were tagged, and 6 refused (4 under row 2, 1 under row 1, 1 under row 3). 5 declare
    `inline-images-not-emitted`, and all 5 were tagged.
  - **Reads back.** On the outputs, `extract` binds 41,208 of 41,209 runs `pdf_tagged`, computed,
    `Document/Div`; the remaining run is page furniture, still `pdf_artifact`.
    `structure-tree-engine-written` is declared on 129 documents, `untagged-structure-tree-absent`
    on none.
  - **Projects (§4.3).** `ground`, `markdown` and `html` each equal the original's on 129 of 129.
    The instrument's first run, at `5b4b7bb` without S3's `group_key` change, found 102, 97 and 97
    of 128 different, which is §4.3's prediction measured.
  - **Grounds.** `grounding-check` finds all 129 valid and matched. `ethos verify` agrees on 122 of
    122, all grounded; on the other 7, no element contains the chosen run on either side.
  - **Bytes.** A second `tag` is byte-identical on 129 of 129. Bytes added: median 716 (44.3%),
    from -125,323 to 3,821; 15 documents shrank. `qpdf --check` exits 0 on all 129 outputs at
    `0c7a3c9` (and on 128 originals; the other exits 3 with warnings, its output 0).
  - **Reals, §3.5's reopening condition.** 6 of 4,116 reals outside content streams do not survive
    `f32`, in 2 documents, both PyPDF2: `/FontMatrix` `0.00100000005` → `0.001` and `/Matrix`
    `-1.60399354` → `-1.6039935`. That is non-zero on one producer, below the ninth significant
    digit, and invisible to the text record. Whether PyPDF2 is *a producer that matters* is the
    owner's to say, and nothing here decides it.
- **§7.2, the consumer (measured 2026-09-17).** `ethos-parser` 0.58.0 as released, run as
  `extract` on the 129 outputs, binds all 41,208 written runs `pdf_tagged` under `Document/Div`,
  and not one of those locators carries `derivation`. Every engine-written `/Div` reads as author
  structure: the launder #21 refused, produced by a shipped reader with no test-only flag.
- **§7.2, the misread rate (measured 2026-09-17)** with `ethos-parser` 0.58.0 as released (the
  `aarch64-apple-darwin` build) and qpdf 12.3.2, over the eight gate documents as shipped, by
  [`measurements/auto-tagging/paragraphs.py`](measurements/auto-tagging/paragraphs.py). The method,
  the exclusions and every table are in [`measurements/auto-tagging/README.md`](measurements/auto-tagging/README.md)
  §1.
  - **The labelled document.** `nist-sp-800-207` is the one document whose `/P` labels are
    paragraphs. There the join binds 63,306 of 90,817 runs to a `/P`, and it agrees with the
    engine's `role_path` on all 81,558 tagged runs.
  - **(i) Blocks holding more than one author `/P`.** 23 of the 259 blocks holding a `/P`-bound run
    hold two or more author `/P`: 8.9%. Outside any `/Table` it is 11 of 219 (5.0%). Among blocks
    holding only table-cell paragraphs it is 12 of 40 (30.0%): those rows share baselines, so they
    are one block by construction. The 23 blocks carry 96 author boundaries, 69 of them in table
    blocks. Per page the share runs 0.0% .. 100.0%, median 0.0%. The worst page by count is 51,
    with 4 of 12 blocks (region 2 block 3 is a table holding 6 `/P`). The two pages at 100% are 36,
    the one band the rule declined, with 3 `/P` in it, and 54, a table whose 44 cell paragraphs
    are one block.
  - **(ii) Author `/P` split across blocks.** 12 of 343 `/P` elements fall in two or more blocks:
    3.5%, and all 12 cross a page break. Within a page the cut splits no paragraph (0 of 343). Per
    page it runs 0.0% .. 50.0%, median 0.0%; the worst page is 34.
  - **The other seven gate documents.** Across all eight, (i) runs from 8.9%
    (`nist-sp-800-207`) to 81.8% (`nist-sp-800-53Ar5`), median 58.7% (`nist-sp-800-171r3`), and
    (ii) from 0.0% (`irs-f1040sd-2025`, `irs-fw9`) to 3.5% (`nist-sp-800-207`), median 0.3%
    (`nist-sp-800-218`). That is producer behaviour, not recall: their `/P` is a line or a page
    wrap.
  - **The quoted recall.** The 63.7% recall at 100% precision this document quotes from
    `blocks.rs` (§3.2, §7.2, §9) reproduces exactly on `probe3b.py` at 0.58.0, and it is bounded
    from both sides. On the same 135 boundaries the shipped cut finds 79 (58.5%). Of the seven
    boundaries the simulation finds and the shipped cut misses, five are whitespace-only runs,
    empty paragraphs Word writes that each halve a gap, and two are on page 36's declined band.
    Over every raw `/P` it finds 129 of 189 (68.3%) and still cuts 0 of 1,022 mid-paragraph
    pairs.

- **§3.5, the read path (2026-09-17, after the merge).** The leniencies this section gives as the
  writer's reason for decoding strictly are refused on the read path too: `extract` and the table
  diagnostics run this section's tokeniser, require its operators to be `lopdf`'s, and check that a
  `FlateDecode` stream's deflate data reaches its end, all before a page is interpreted;
  `classify` counts nothing for a page either check refuses (`extract.rs::page_operations`). So the
  sentence above — that §3.7 cannot catch a dropped tail later, because the re-extraction "would
  drop the same tail" — is superseded: the re-extraction refuses it. The writer's own conditions
  are unchanged and it stays the stricter of the two: it also refuses a failed Adler-32 check,
  bytes after the deflate data, and every filter other than none and `FlateDecode`, each of which
  the reader still takes from `lopdf`. **A corpus example arrived the next day**: of OmniDocBench's
  981 born-digital `v1_0` pages, one is refused by the tokeniser on the read path, where the build
  before the check wrote an artifact of a page whose 125 718 content bytes were dropped after the
  fourth ([`measurements/omnidocbench/README.md`](measurements/omnidocbench/README.md)).
- **§8, the extract artifact (corrected 2026-09-18, found reviewing the 0.59.0 release note).**
  Its last clause — *a 0.58.0 build, whose record did not deny unknown fields, ignores the key* —
  is true of the tagged-table record and never of an extract artifact. A tagged table's cells are
  tagged runs, every run's `pdf_tagged` locator carries `derivation` as well, and 0.58.0's
  `TextRun` denies unknown fields, so each build refuses the other's extract artifact of any
  tagged PDF, and 0.58.0 also refuses one whose runs carry decision #29's `inferred_heading`.
  `EXTRACT_SCHEMA_VERSION`'s rustdoc said the same and is corrected with this. §8's statement
  about the representation stands, and was run both ways against the 0.58.0 release binary on
  `tagged-structure-roles`: `unknown field derivation` one way, `missing field derivation` the
  other, each exit 2.

With §7's numbers published, clause two meets the condition §12's proposed row #27 names. The rows
themselves remain the owner's to record.

**Recorded 2026-09-17.** The owner accepted §12's three rows as proposed. They are North Star rows
#25–#27, and v2.2's gate is met on #27.

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
| An encrypted document | refused (`EngineError::Encrypted`) | One that needs a password, as every subcommand refuses it; one the empty user password opens, which the reader reads and declares (decision #31), because a rewritten copy would carry neither its encryption nor its permissions |
| A digital signature: a dictionary carrying `/ByteRange` | refused, naming the object | The signature covers the source's bytes, and §3.5's full serialisation moves them, so the copy would carry a signature that no longer verifies. An incremental update would keep it |
| A real outside `f32`'s range, in any object or the trailer | refused, naming where | `lopdf` reads it as infinite and its writer prints the token `inf`, which qpdf refuses: the copy would not be a PDF |
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
  **Amended 2026-09-18:** `locate` is an eleventh subcommand and a fourth MCP tool (decision #30),
  so the population this counted has moved. The count is left as it stood when this was written
  rather than restated, on `verify_relay.rs`'s rule about ordinals — a recount asserted in passing
  is how a sentence acquires a second wrong number. Nothing in the reasoning changes: `tag` is
  still not exposed, for the reason above.
- **Not `--max-pages`.** §3.6.

## 6. Where it sits

| Surface | Shape |
| --- | --- |
| CLI | `ethos-parser tag <pdf>` — stdout is the tagged PDF, the second subcommand after `overlay` whose stdout is not canonical JSON. Exit **0** written, **2** could not read or refused. The tenth subcommand |
| Library | `ethos_parser_pdf::write_tags(&Document, &Profile) -> Result<Vec<u8>, EngineError>` and `TAGS_ARTIFACT_TYPE`, frozen in [`PUBLIC-API.md`](PUBLIC-API.md) with a thin-shell row. The writer runs extraction itself, as `overlay` does in-process, because the artifact does not say which operator showed a run: that mapping — `(page, operator index in the joined buffer)` per run, surviving `reorder_page` — lives in a crate-private side table from a crate-private extraction entry point, never on `TextRun`, so `ExtractArtifact` does not change and no artifact parsed from JSON can reach the placement rule |
| Reader | `structure.rs` and the wire field of §4.2; `ethos_parser_pdf::limitations::structure_tree_engine_written`; `codes::STRUCTURE_TREE_ENGINE_WRITTEN` |
| Profile | `struct_tree_rule: struct-tree-v2` |
| Fixtures | Hand-written PDFs in the exact tree shape the writer emits (no catalog stamp: the reader never reads it, and its `SourceSha256` and `ParserVersion` are the writer's to fill, so S2 checks the stamp on the writer's own output), so the reader is tested independently of the writer and a writer regression cannot hide a reader one: `engine-tagged-blocks` (the attribute under `/A`), a `/ClassMap`-only twin (`/C` on every element, the class on the root), a mixed twin (`/A` holding a foreign owner such as `/O /Layout`, `/C` carrying the engine class), and a page whose block has one line inside `/Span BMC … EMC` and one inside an optional-content sequence given by name; and a widget cited by `/OBJR` under an author's tree and under the engine's, so the locator an `/OBJR` mints is read in both classes. For the writer's refusals: an untagged page with `/P /MC0 BDC` whose `/MC0` carries `/MCID`; two pages sharing one content stream through a single reference |

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
- **The extract artifact follows the same rule.** `TaggedTableRecord.derivation` — the class of
  §4.1 on the tagged-table record of `ethos.parser.extract.v0`, which `represent.rs` reads to fill
  the representation's `TableRecord` — is required with no default, and `EXTRACT_SCHEMA_VERSION`
  stays at 0.4.0 on the precedent of 0.55.0's `TextRun.block`, the release note naming it. The
  record denies unknown fields from this slice on, so this build refuses a 0.58.0 extract that
  carries a tagged table (`missing field derivation`) and later shapes refuse each other
  symmetrically; a 0.58.0 build, whose record did not deny unknown fields, ignores the key. The
  artifact is a draft library surface with no stored fixtures.
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
