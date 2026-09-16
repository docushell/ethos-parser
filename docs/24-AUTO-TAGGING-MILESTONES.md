# 24 — Auto-tagging slices

**Implementation authority for v2.2's second half.** Scope is in
[`23-AUTO-TAGGING-SCOPE.md`](23-AUTO-TAGGING-SCOPE.md), and every auto-tagging PR belongs to exactly
one slice below. The document stands under the assumption that scope §12's proposed rows are taken
as written; if the owner changes one, the affected slice is re-cut here before code moves.

**Written 2026-09-16 against `main` at `b4b4aa9`, 0.58.0 unreleased.** v2.2's first half shipped
as D4 S0–S4 at 0.42.0 and the block cut at 0.55.0; nothing here reopens either.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | The scope document and this one | done |
| **S1** | The reader: `/O` under `/A` and through `/ClassMap`, `derivation` on every `pdf_tagged` locator, `struct-tree-v2`, the hand-written fixtures | — |
| **S2** | The writer: strict decode, the tokeniser with positions, the placement rule, the tree, the self-check, `tag` | — |
| **S3** | The round trip: write, read back, project, ground, verify; the mutations that must fail | — |
| **S4** | The measurements of scope §7, and the documents that move with them | — |

S1 lands before S2 on purpose: a reader tested against a PDF a human wrote in the writer's exact
shape is a reader whose test cannot pass because the writer and the reader share a mistake. S2 is
then tested against S1's reader, and S3 tests the pair.

---

## S1 — The reader

**Files.** `crates/ethos-parser-core/src/representation.rs` (`PdfTaggedLocator.derivation`),
`crates/ethos-parser-core/src/profile.rs` (`STRUCT_TREE_RULE_V2`, the default profile, the pin),
`crates/ethos-parser-core/src/assurance.rs` (`codes::STRUCTURE_TREE_ENGINE_WRITTEN`),
`crates/ethos-parser-pdf/src/structure.rs` (`STRUCT_ATTRIBUTE_OWNER`, crate-private through S1, its
spelling pinned by a test against scope §3.3; owner read under `/A` and `/C`, derivation on
bindings, counts), `crates/ethos-parser-pdf/src/limitations.rs` (`structure_tree_engine_written`),
`crates/ethos-parser-pdf/src/extract.rs` (declare it), `crates/ethos-parser-pdf/src/represent.rs` (a
tagged table's derivation from its element), `fixtures/engine/make_fixtures.py` and
`fixtures/manifest.json` (the fixtures below), `crates/ethos-parser-pdf/tests/robustness.rs` (the
pinned survivors), `docs/draft-schemas/document-representation.draft.json`,
`docs/draft-schemas/profile.draft.json`, `docs/PUBLIC-API.md` (the field), and every test that
constructs a `PdfTaggedLocator` by hand (`markdown.rs`, `representation.rs`, the grounding and
projection tests).

**What lands.**

1. `PdfTaggedLocator` gains `derivation: DerivationClass`, serialised on every locator with no
   `skip_serializing_if` and no `serde(default)` (decision #20). `deny_unknown_fields` stays, so a
   0.58.0 representation that binds a run to a tree no longer parses under this build and vice
   versa — scope §8 — and the release note says so.
2. `structure.rs` reads, on every structure element, the attribute objects under `/A` (dictionary,
   array, array with revision numbers, through an indirect reference to any of them) and the ones
   reached through `/C` — a name or an array of names — resolved through the root's `/ClassMap`
   with the lookup shape `read_role_map` uses. Over that union: an object with `/O /EthosParser`
   marks the element engine-written; under that owner `/Derivation /Computed` is required and
   anything else is `EngineError::Malformed` naming the element; an `/A` object decides, a `/C`
   object decides when `/A` carries none, a class absent from `/ClassMap` contributes nothing. A
   binding made directly under an engine-written element carries `Computed`; every other binding
   carries `Extracted`. `StructureTree` gains `engine_written: Option<EngineWritten { elements:
   u32, rules: BTreeSet<String> }>` for the declaration; `TaggedTable` gains `derivation`, and
   `represent.rs` uses it instead of the constant it writes today. `/MarkInfo` is never read, and
   the module header says so.
3. `struct_tree_rule` moves to `struct-tree-v2`; `the_default_profile_is_pinned` is re-pinned from
   its own output with a move-log paragraph, as every prior move; the two draft schemas follow.
4. `extract.rs` declares `structure-tree-engine-written` (document scope) when the tree reports
   engine-written elements, with the element count, the bound-run count, the rule names and the
   sentence that the input carried no author structure tree in its detail.
   `untagged-structure-tree-absent` is not declared for such a document: a tree was read.
5. Four hand-written fixtures on the `leading-gap-two-blocks` page (six lines, one 28 pt gap;
   branch `feat/block-cut-leftovers`), in the writer's exact tree shape — the catalog stamp of
   scope §3.3 excepted, which the reader never reads — and without `/MarkInfo`:
   - `engine-tagged-blocks`: `/Document` carrying the attribute under `/A`, two `/Div` children
     each carrying it, `/K [0]` and `/K [1]`, `/Pg`, `/ParentTree` and `/ParentTreeNextKey 1`,
     `/StructParents 0` on the page, the block's `Tj`s in the page's one shared text object with
     each block's sequence `/Div << /MCID n >> BDC … EMC` opened and closed inside it (scope
     §3.4: a text object shared between two blocks is split at the operators, inside it, and
     §3.5: the stream is the untagged page's with the tags inserted and nothing else changed);
   - `engine-tagged-classmap`: the same tree with `/C /EthosBlock` on every element and
     `/ClassMap << /EthosBlock << /O /EthosParser /Derivation /Computed /Rule (gutter-columns-v3)
     >> >>` on the root, no `/A` anywhere;
   - `engine-tagged-mixed`: `/A << /O /Layout /Placement /Block >>` on every element and the
     engine class through `/C`;
   - `engine-tagged-nested-frames`: one block whose first line sits inside `/Span BMC … EMC` and
     whose second sits inside `/OC /oc1 BDC … EMC` (given by name through `/Properties`, an
     `/OCG` in the resources), each with the written `/Div` sequence opened inside the frame and
     closed before its `EMC`, so the reader binds both lines.
   And two more, added by S1's review, on the `form-field-value` page: `tagged-widget-objr`
   (`/Document` → `/Form` whose `/K` is an `/OBJR` citing the widget, `/StructParent 0` on the
   widget and the matching `/ParentTree` entry, no attribute, no `/MarkInfo`) and
   `engine-tagged-widget-objr` (the same tree with the attribute under `/A` on both elements), so
   the locator an `/OBJR` mints is read in both classes — only the `computed` case proves the
   `/OBJR` arm carries the citing element's class rather than a constant.
   Each bumps `engine_owned` in `fixtures/manifest.json` and pins its robustness survivors by that
   test's own procedure.

**Acceptance.**

- `an_engine_written_tag_reads_back_as_computed`: on `engine-tagged-blocks`, all six runs bind
  `pdf_tagged` with `role_path == ["Document", "Div"]` and `derivation == Computed`, runs 1–3 to
  mcid 0 and 4–6 to mcid 1; `Node.derivation` is `Extracted` on all six; the artifact declares
  `structure-tree-engine-written` naming 3 elements, 6 runs and `gutter-columns-v3`, and declares
  none of `untagged-structure-tree-absent`, `structure-mcid-unbound`,
  `structure-item-without-content`.
- `the_attribute_is_read_through_the_class_map_too`: `engine-tagged-classmap` and
  `engine-tagged-mixed` read back identically to `engine-tagged-blocks`.
- `a_nested_frame_does_not_hide_the_binding`: on `engine-tagged-nested-frames` both lines bind
  `Computed` under mcids the tree cites, and `mcid-property-list-by-name` is declared for the
  named `/OC` list exactly as it is today.
- `an_authors_tag_still_reads_back_as_extracted`: on `tagged-structure-roles` every `pdf_tagged`
  locator says `derivation: extracted` and the engine-written declaration is absent.
- `the_owner_without_its_derivation_is_refused`: the fixture with `/Derivation` removed from one
  attribute, or set to `/Extracted`, edited through `lopdf` in the test, is
  `EngineError::Malformed` naming the element.
- `an_attribute_under_another_owner_is_an_authors`: `/O /EthosParser` renamed reads back with
  `derivation: extracted` on every run — #23's *the row goes with it* demonstrated, and the test's
  comment says it documents a failure mode, not a feature.
- `an_attribute_behind_a_reference_is_read`: `/A 12 0 R` resolving to the same dictionary reads
  identically to the inline form.
- `mark_info_is_never_read`: the fixture with no `/MarkInfo`, with `<< /Marked true >>` and with
  `<< /Marked false >>` binds identically.
- `every_profile_field_is_hash_sensitive` still destructures exhaustively (no new field there), and
  the pin test carries the move.
- A representation from the fixture round-trips through `serde_json` and `verify_fingerprint`; the
  Python and Node SDK suites still pass on their committed fixtures (they carry no `pdf_tagged`
  example today — verify, and add one if they do).

## S2 — The writer

**Files.** `crates/ethos-parser-pdf/src/tagging.rs` (new: the strict decoder, the tokeniser with
positions, the placement rule, the tree, the stamp, the self-check), `crates/ethos-parser-pdf/src/content.rs`
(`ShownText.op_index`, crate-private), `crates/ethos-parser-pdf/src/extract.rs` (a crate-private
entry returning the per-run `(page, operator index)` side table beside the artifact, surviving
`reorder_page`), `crates/ethos-parser-pdf/src/lib.rs` (`write_tags`, `TAGS_ARTIFACT_TYPE`),
`crates/ethos-parser-cli/src/main.rs` (`tag`), `crates/ethos-parser-cli/tests/public_api.rs`,
`docs/PUBLIC-API.md`, `docs/04-ARCHITECTURE.md` §2, `docs/CAPABILITY.md`. S2 decides whether
`STRUCT_ATTRIBUTE_OWNER` stays crate-private in `structure.rs`, where S1 left it, or is re-exported
from `lib.rs` beside `write_tags` — with a `public_api.rs` entry and a `PUBLIC-API.md` row if so.

**What lands.**

1. `ShownText` records the index of the operation that showed it; the crate-private extraction
   entry returns, beside the unchanged `ExtractArtifact`, one `(page, operator index)` per run in
   the artifact's final order. `TextRun` and `ExtractArtifact::to_canonical_bytes` do not change,
   and a test asserts the public `TextRun` has no such field.
2. `tagging.rs` reads each page's `/Contents` itself: an entry that is not a stream, a filter
   outside none/`FlateDecode`/`LZWDecode`/`ASCII85Decode`, or a Flate stream that does not inflate to
   the end of its input with a valid check, is `EngineError::Unsupported` or `Malformed` naming the
   page and the cause; the decoded streams are joined with `\n` as `get_page_content` joins them,
   and a test asserts the joined buffer equals `get_page_content`'s on every fixture where the
   latter succeeds.
3. The tokeniser with byte positions mirrors `lopdf::content::Content::decode` — numbers, names,
   strings with nested parentheses and escapes, hex strings, arrays, dictionaries, comments, and
   `BI … ID … EI` as one opaque token ending by `lopdf`'s own rules — and refuses a page unless
   every byte is placed in a token and the operator sequence equals `lopdf`'s decode of the same
   bytes, count and names in order (`content-tokeniser-disagreement`, naming the page and the
   first differing operator).
4. The placement rule of scope §3.4, over a per-operator nesting stack of `BT`/`ET`, `q`/`Q`,
   `BMC`/`BDC`/`EMC`: sequences contain only this block's text-showing operators and non-drawing
   operators, end before a painting operator, a foreign, dropped or empty run or an `/Artifact`
   sequence, never contain an existing sequence boundary, and are widened outward over the enclosing
   `BT … ET` and then a `q … Q` that encloses exactly that while the rule allows; adjacent
   sequences of one block separated only by non-drawing operators merge. `BDC` whose property list
   is a name resolves through the page's `/Properties` (inherited): an `/MCID` found, or a name that
   resolves to nothing, refuses the document by scope §3.6's third row. Ids are assigned in stream
   order, dense from 0, per page.
5. The tree: one `/Document`, one `/Div` per block in reading order with `/K` in ascending id
   order, every element with the attribute of scope §3.3 under `/A`, `/ParentTree` as a flat
   `/Nums` with one entry per rewritten page, `/ParentTreeNextKey`, `/StructParents`, the catalog
   stamp, and no `/MarkInfo` (one already present is left as found). Each rewritten page gets one
   `FlateDecode` stream built by the writer at a fixed level, `/Contents` pointing at it, and the
   superseded streams no other page references are removed from the object map directly.
6. The refusals of scope §3.6, each an `EngineError` whose `detail` names the reason, before any
   byte is written.
7. The self-check of scope §3.7: open the bytes, `extract`, compare page by page and run by run on
   `(text, origin_x, origin_y, advance, font_size, char_codes, synthesized, region, block)`, on the
   binding, on the per-page counters, on the limitations, and on the tokenised output's sequence
   contents; refuse with the first difference or the failing condition named.
8. `ethos-parser tag <pdf>` writes the bytes to stdout; exit 0 or 2; `timed(Stage::Extract, …)` as
   `overlay`. `write_tags(&Document, &Profile)` and `TAGS_ARTIFACT_TYPE` join the frozen surface
   with a thin-shell row.

**Acceptance.**

- `the_writer_emits_the_readers_fixture_shape`: `write_tags` on `leading-gap-two-blocks` produces a
  document whose structure tree, walked with `structure::read`, has the same elements, roles,
  attributes and bindings as `engine-tagged-blocks`; on the nested-frames page's untagged twin
  (write it beside the fixture) the same holds against `engine-tagged-nested-frames`.
- `the_tokeniser_accounts_for_every_byte_and_agrees_with_lopdf`: over every PDF in
  `fixtures/engine`, `fixtures/gate` and the oracle corpus, every byte is placed, the operator
  sequence equals `lopdf`'s decode, and re-slicing the bytes at the recorded positions reproduces
  each operator's text; a page with a filtered inline image (write a fixture) resynchronises at the
  same byte as `lopdf`.
- `a_tagged_document_is_byte_identical_across_runs`: two `write_tags` calls on one input agree
  byte for byte, on the two-column fixture and on one bench document.
- `a_tagged_document_is_refused`: `tagged-structure-roles`, with the fills-absence reason.
- `ids_without_a_tree_are_refused`: a fixture with `/P << /MCID 0 >> BDC` and no `/StructTreeRoot`
  (write it), and `engine-untagged-mcid-by-name`: an untagged page with `/P /MC0 BDC` whose
  `/MC0` carries `/MCID` (write it), both refused by name; the same page with `/MC0` carrying no
  `/MCID` is tagged, the sequence placed inside the frame.
- `an_artifact_run_stays_outside_the_tree`: a fixture with `/Artifact BMC … EMC` furniture and no
  tree (write it) is tagged, its furniture run still binds `pdf_artifact`, no element cites it, and
  the artifact sequence is inside no written sequence.
- `a_shared_content_stream_is_not_edited_in_place`: two pages sharing one content stream through a
  single `/Contents` reference, with different page boxes so their cuts differ (write it), tag
  cleanly to two distinct streams and pass the self-check.
- `the_self_check_refuses_a_moved_run`: with the writer's output mutated in the test to shift one
  `Td` operand, the self-check refuses naming that run (this tests the check, not the writer).
- `tag_cli.rs`: the CLI's bytes equal the library's; exit codes 0 and 2; stdout is a PDF and not
  JSON.

## S3 — The round trip

**Files.** `crates/ethos-parser-cli/tests/tag_roundtrip.rs` (new), `markdown.rs` (`group_key`
reads `derivation`), and whatever the projections' tests need.

**What lands.** `group_key` returns `None` for a `pdf_tagged` locator whose `derivation` is
`Computed`, so the projections take the undeclared path on an engine-tagged document (scope §4.3).
No projection rule id moves; the reason is recorded beside the change.

**Acceptance.**

- `a_written_tag_is_read_back_and_grounds`: for `leading-gap-two-blocks`, `two-column-15-lines`,
  `untagged-shredded-line` and `markdown-two-blocks`: tag; extract the tagged bytes; every run
  binds `pdf_tagged` with `derivation: computed`; the text record equals the source's run for run;
  `ground` succeeds and `grounding_check` says valid; with `ETHOS_BIN` set, `ethos verify` on a
  claim quoting one run's text returns the same report on the tagged and the untagged document
  (relayed, bytes untouched — the verify boundary is not crossed, because nothing here reads the
  report beyond the oracle harness's existing helper).
- `the_projections_of_a_tagged_document_are_its_originals`: on each of those four, `markdown`,
  `html` and `ground` of the tagged representation equal the untagged original's apart from the
  assurance block — scope §4.3 pinned.
- `an_engine_written_tree_is_declared_on_the_artifact_and_not_as_authors`: the tagged
  representation's limitations carry `structure-tree-engine-written` and not
  `untagged-structure-tree-absent`; the untagged original's carry the reverse.
- `stripping_the_attribute_launders_the_tag`: the writer's output with every `/A` removed (edited
  through `lopdf` in the test) reads back `derivation: extracted` on every run and declares no
  engine-written tree — the failure #23 names, held as a test so that the day it stops being
  reproducible is noticed.
- `a_second_write_is_refused`: `write_tags` on its own output is refused.

## S4 — The measurements and the documents

**Files.** `docs/measurements/auto-tagging/roundtrip.py`, `paragraphs.py`, `README.md`;
`docs/23-AUTO-TAGGING-SCOPE.md` (numbers filled in, in an amendments block under the header);
`docs/02-ROADMAP.md` (the v2.2 row); `docs/OPEN-WORK.md`; `docs/CAPABILITY.md` (a `Tag` row in
Can, and the engine-local limit in Cannot); `docs/01-CONTRACT.md` §5.1 (the structural locator
carries a derivation) and §6 (the `Computed` row names a written tag read back);
`docs/16-D4-SCOPE.md` §7 and `docs/00-NORTH-STAR.md` §5 (the *owed* / *allowed* / *not built*
sentences, corrected by dated appendix, never by rewriting a numbered row); `README.md`.

**What lands.**

1. `roundtrip.py`: scope §7.1 over the untagged engine fixtures, the untagged oracle fixtures and
   the 200 bench documents — every count and distribution it lists, the raw-token scan for reals,
   the self-check verdicts, the projection equality, `grounding-check` on every tagged
   representation, `ethos verify` on the named subset. Results in the README as tables; the
   instrument committed, never the corpus.
2. `paragraphs.py`: scope §7.2 on `nist-sp-800-207` as shipped — the `(page, region, block)` to
   author-`/P` join through `qpdf --json`, both shares, worst page named — and the consumer
   demonstration with the 0.58.0 release binary on the writer's output of one untagged fixture.
3. The wire cost of S1's field on the eight gate documents: artifact bytes at 0.58.0 against this
   branch at the same version string.
4. The documents above, each stating what shipped and what is still the owner's.

**Acceptance.** The numbers exist and are in the tree; the scope document's amendments block
records each with its date; `OPEN-WORK.md` moves clause two from *not started* to *built, owner
decision pending on §12*.

---

## What is not a slice

- Recording scope §12's rows in the North Star: the owner's, by decision #2's rule that a
  reversal is recorded there first.
- Publishing 0.59.0: `RELEASING.md`.
- Exposure over MCP and the SDKs: refused in scope §5 until an owner decision.
- Any role other than `/Div`: D1.
- Resolving `/Properties` in the reader, or reading a written sequence as a declaration in the
  projections: each a separate reader decision with its own measurement.
