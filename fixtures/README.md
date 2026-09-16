# Fixtures

Mostly a manifest, not fixture bytes. `manifest.json` names every fixture by path and `sha256`, so
a fixture changing is a visible event in this repository rather than a silent one.

If a root is missing or a hash does not match, the harness **fails with a named error**. It never
skips and never quietly uses a different file.

## The four roots

| Root | Default | Override | What it holds |
| --- | --- | --- | --- |
| `conformance` | `../ethos/fixtures` | `ETHOS_FIXTURES` | The 15 Ethos-owned fixtures. The oracle criterion counts exactly these |
| `benchmark` | `../ethos/benchmarks/gate-zero/corpus` | `ETHOS_BENCH_CORPUS` | 4 large real-world PDFs. Not part of the oracle count |
| `engine` | `fixtures/engine` | `ETHOS_PARSER_FIXTURES` | 45 CC0 PDFs authored here, for behaviours the Ethos corpus does not cover |
| `gate` | `fixtures/gate` | `ETHOS_GATE_CORPUS` | 8 tagged public documents for the table gate — **committed here**, because a corpus you publish numbers about has to be one anyone can re-measure |

`fixtures/office/` holds 16 small office packages for the v2 readers and the mutation harness, and
`fixtures/labelled/table-truth.json` holds the hand-tagged table cells the gate scores against.

The benchmark root exists because two acceptance tests name documents the conformance corpus does
not contain: a 492-page PDF for the bounded-cost check, and a real simple document for the
exit-code-0 case. Keeping them in their own root means they cannot inflate the 15.

## Two fixtures worth knowing before you debug against them

- **`synthetic/table-regular-grid` did not open at all until v0.1.** Its cross-reference entries are
  19 bytes where the PDF spec requires exactly 20. PDFium repairs that; `lopdf` rejects it. v0
  exited 2 and declared the limitation; v0.1 pads the entries under published preconditions and
  reads the file, declaring `xref-entry-padded` on the artifact. Against an older build, that exit-2
  is correct behaviour rather than a bug.
- **`failure/memory-limit-simulated` is byte-identical to `synthetic/simple-text`.** Both hash to
  `f2f6ab91…`. The limit is simulated by configuration, not by a different document, so a test
  asserting "different fixture means different bytes" would be wrong here.

## Engine-owned fixtures

Each one exists because the Ethos corpus genuinely cannot cover the behaviour. `manifest.json`
marks the owner, and `counts.engine_owned` is the number a test checks. The ones whose purpose is
not obvious from the name:

| Fixture | Proves |
| --- | --- |
| `show-text-quote-operators` | The `'` and `"` text operators, whose omission is a known defect in another parser |
| `horizontal-scaling-tz` | `Tz` changes the advance |
| `synthesized-space-tj` | A `TJ` gap wide enough that a space was intended but never written |
| `measured-ink-box` | A font descriptor with real ascent and descent, so the ink box is genuinely measured |
| `absent-font-metrics` | A descriptor that resolves and declares no ink extent — the one shape most likely to tempt a `height = font_size` fallback |
| `broken-font-encoding` | Glyph names no table carries, so the run is dropped rather than turned into mojibake |
| `ruled-table-grid` | A 3×3 grid drawn with rectangles, one merged cell, one empty cell |
| `ruled-table-overlap` | Two rectangles claiming one grid cell — the rule refuses the grid and says why |
| `unruled-near-miss` | Columns that align on two rows and miss on the third by five points: no table, plus a named refusal |
| `background-panel-not-a-grid` | A background panel that covers every cell an early rule inferred, so it called decoration a 7×7 table |
| `both-table-rules` | One painted grid and one aligned-text grid on a page, under two rule ids |
| `ruled-wins-shared-region` | A painted grid whose text also aligns cleanly — one table comes out, and it is the ruled one |
| `stroke-ruled-worksheet` | A grid drawn as stroked lines rather than filled boxes, including an undrawn top edge |
| `stroke-ruled-columns-not-drawn` | The same horizontal ink with no vertical rules — must be refused, because lines that merely end at the same x are not a boundary |
| `stroke-ruled-field-boxes` | A stroked grid whose cells are really four form-field boxes |
| `tagged-structure-roles`, `tagged-rolemap`, `tagged-table-agrees`, `tagged-table-disagrees`, `tagged-cycle` | The four structural-locator states, role remapping, agreeing and disagreeing tagged grids, and a cycle that must be survived rather than spun on |
| `form-field-value`, `annotation-contents`, `form-orphan-widget`, `form-xfa-stub` | A field value nothing draws, a hidden annotation that is still a node, a broken parent chain declared rather than repaired, and an XFA packet declared and never parsed |
| `two-column-14-lines`, `two-column-15-lines` | The ±1-line pair — see below |
| `image-xobject-drawn`, `image-declared-not-drawn`, `form-xobject-text-drawn` | See below |
| `invisible-render-mode` | Text under render mode 3: present, flagged, never filtered out |
| `off-page-and-offset-box` | A media box whose origin is not `(0,0)`, plus a smaller crop box — the coordinate repair and the off-page finding in one page |
| `composite-font-cid-widths`, `composite-font-non-identity-cmap` | A `/Type0` font whose widths live on its descendant CIDFont as `/W` and `/DW`, and the same descendant under a CMap this profile cannot read — see below |
| `leading-gap-two-blocks` | Six lines in one column, three at a 14 pt leading, a 28 pt gap, three more: the leading-gap half of the block cut opens exactly two blocks, and the fixture states the numbers it is cut against — see below |
| `engine-tagged-blocks`, `engine-tagged-classmap`, `engine-tagged-mixed`, `engine-tagged-nested-frames` | The leading-gap page carrying the structure tree the auto-tagging writer emits, written by hand before the writer exists — the attribute under `/A`, through `/ClassMap`, beside a foreign owner, and inside existing marked-content frames — see below |

Why the upstream corpus cannot cover the table cases: **no fixture in it contains a single path
operator**, so nothing there exercises ruled detection at all.

### The two-column pair only works as a pair

`two-column-14-lines` and `two-column-15-lines` are the same page, written right column first,
differing by one line. They sit on either side of a boundary where a *line-counting* rule would
change its answer, and `gutter-columns-v1` must read both the same way.

**Edit them together or not at all.** Changing the line count in one, or letting the columns creep
closer than the 12 pt gutter floor, leaves a test that still passes while testing nothing.

### The `Do` trio answers three different questions

The first two carry the same image object in the same resource dictionary, and the only difference
between them is one `Do` operator. The third writes that same `Do` and changes the `/Subtype`:

| | `classify` | `extract` |
| --- | --- | --- |
| `image-xobject-drawn` | `embedded-images`, `image_count: 1` | one image node |
| `image-declared-not-drawn` | `embedded-images`, `image_count: 1` | **zero** image nodes |
| `form-xobject-text-drawn` | no image resource | **zero** image nodes, **zero** runs from the form, and `form-xobjects-not-descended` **count 1** |

Classify counts the images a page's resources *declare*. Extract emits a node per image a page
actually *paints*. Neither is wrong, and expecting them to agree means misreading one of them.

The third is not a fourth image case; it is the one placement that produces **no node of any
kind**. This profile does not descend into form XObjects, so the form's text is neither an image
node nor a run — and until v2.2-S2 nothing on the artifact said the placement had happened. A page
whose whole content is `q /Xf1 Do Q` came out empty with `pages_failed: 0`. The profile-scoped
`form-xobject-text-not-descended` could not close that gap, because it rides on **every** artifact
this engine writes, including ones for documents containing no XObject at all: it states the
policy, not the cost. `form-xobjects-not-descended` is document-scoped and carries a count, and the
fixture asserts both halves — present here, absent on the two above.

### The leading-gap fixture is the first one authored for the block cut

`leading-gap-two-blocks` is six single-run lines in one column, in a font declaring real ink
metrics so every run reaches the grounding artifact: three lines at a 14 pt leading, a 28 pt gap,
three more at 14 pt. In the rule's own units (`blocks.rs`: centipoints, top-left origin) the
baselines are 2000, 3400, 4800, 7600, 9000 and 10400, so:

| | |
| --- | --- |
| gaps | 1400, 1400, 2800, 1400, 1400 |
| modal leading | 1400 — four of five gaps, so the share is 4/5 against a floor of 1/4, and 1400 clears the 600 floor |
| threshold | 8/5 × 1400 = 2240 |
| cuts | one, at the 2800 gap |
| blocks | runs 1–3 are block 1, runs 4–6 are block 2, `region` absent on all six |

**It is not the first fixture whose page the cut divides, and the census that found that is the
reason it states its numbers.** Measured with the 0.58.0 binary over the 44 engine fixtures and the
14 conformance fixtures that open: three engine fixtures already come out in two blocks by the
leading-gap half — `both-table-rules` (the gap between its two tables), `rotated-and-mirrored-text`
(seven turned runs scattered down a page) and `stroke-ruled-worksheet` (a worksheet's row pitch) —
each by accident of a layout authored for something else, and four more only by the vertical cut,
one block per column band: the two-column pair, `unruled-near-miss` and the oracle's
`synthetic/two-columns`. None of the seven states its leading or its gap, so none can say which gap
opened a block or that the threshold was cleared on purpose, and a change to `CUT_NUM`/`CUT_DEN` or
to the modal-leading guard would move them without any fixture saying why. The two-run fixtures
(`markdown-two-blocks`, `markdown-hyphen-break`) cannot cut at all: one gap is its own modal leading,
and a gap never clears 1.6 times itself. That pair is the negative half of the wire tests in
`crates/ethos-parser-pdf/tests/extraction.rs`, and this fixture is the positive half.

### The engine-tagged family is the writer's tree shape, written by hand first

`engine-tagged-blocks` is `leading-gap-two-blocks` — the same six lines, baselines and metrics
font — carrying the structure tree the auto-tagging writer emits (`docs/23-AUTO-TAGGING-SCOPE.md`
§3.3–§3.4): `/Document` over one `/Div` per block, every element carrying
`/A << /O /EthosParser /Derivation /Computed /Rule (gutter-columns-v3) >>`, each block's three `Tj`s
in one `/Div << /MCID n >> BDC … EMC` opened and closed inside the page's single text object (§3.4:
a text object shared between two blocks is split at the operators, inside it), a `/ParentTree`,
`/StructParents` on the page, and no `/MarkInfo`. The content stream is the untagged page's with
the `BDC`/`EMC` tokens inserted and nothing else changed — §3.5's rule for the writer, which the
generator asserts. The catalog stamp of §3.3 (`/EthosParserTags`) is deliberately absent: the
reader never consults it, and its `SourceSha256` and `ParserVersion` are the writer's to fill; S2
checks the stamp on the writer's own output. It was written by hand **before the writer exists**,
on purpose: a reader tested against a file the writer produced could pass on a mistake the two
share, and this file cannot have one. The reader must bind all six runs `pdf_tagged` under
`Document/Div` with `derivation: computed`, runs 1–3 to mcid 0 and 4–6 to mcid 1, and declare
`structure-tree-engine-written` rather than `untagged-structure-tree-absent`.

The three siblings hold the same page under the same tree and move only where the attribute sits,
which is what the reader has to be indifferent to:

| Fixture | Where the attribute is |
| --- | --- |
| `engine-tagged-classmap` | on the root's `/ClassMap` as `/EthosBlock`, reached through `/C` on every element; no `/A` anywhere |
| `engine-tagged-mixed` | a foreign owner inline (`/A << /O /Layout /Placement /Block >>`) and the engine's class through `/C` — `/A` decides only when it carries the engine's owner |
| `engine-tagged-nested-frames` | under `/A` as in `-blocks`, but block 1's first line sits inside `/Span BMC … EMC` and its second inside `/OC /oc1 BDC … EMC` given by name, the written `/Div` sequence opened inside each frame; block 1 is ids 0, 1, 2 and block 2 is id 3 |

The first two must read identically to `-blocks`, bindings and declarations alike. The third
binds all six runs `computed` and declares the named `/OC` list as `mcid-property-list-by-name`,
exactly as any named list is declared today. The failure modes — the owner without its
`/Derivation`, the owner renamed, the attribute behind a reference, a `/MarkInfo` added — are
made in the tests by editing `-blocks` through `lopdf`, so each is one edit away from the file
that reads correctly (`crates/ethos-parser-pdf/tests/tagging_read.rs`).

The pair `tagged-widget-objr` / `engine-tagged-widget-objr` holds the one binding the family above
cannot: a locator minted for an **object** the tree cites by `/OBJR` rather than for a
marked-content id. Both are the `form-field-value` page — one label, one widget, nothing marked —
under `/Document` → `/Form` whose `/K` is `<< /Type /OBJR /Obj 10 0 R /Pg 3 0 R >>`, with
`/StructParent 0` on the widget, `/ParentTree << /Nums [0 8 0 R] >>` on the root, and no
`/MarkInfo`. The first carries no attribute and is an author's tree; the second carries the
engine's attribute on both elements, a shape the writer never produces (it tags blocks, not
widgets) and nothing forbids. The form-field node's locator must read `derivation: extracted` on
the first and `computed` on the second: only the `computed` case proves the `/OBJR` arm carries
the citing element's class rather than a constant.

**Object 6 is the tree, so the descriptor cannot take its usual number.** `build_pdf` refuses a
fixture that wants both, and the list fixture went without metrics for that reason; this family
carries the same Helvetica descriptor as its tree's last extra object (10, or 11 where the `/OCG`
takes 10) and names it through `FONT_EXTRA`, so all six runs keep the measured ink boxes the
untagged page has.

### The composite-font pair is about a shape neither corpus had

Before v2.2-S3, `grep -rl CIDFontType fixtures/` matched **nothing** — in either owned corpus. So
the composite-width path was exercised by no test at all, and `load_widths` spent its entire life
asking a `/Type0` dictionary for `/Widths`, a key PDF 32000-1 §9.7.4.3 never puts there. 1 294
tests passed while every composite font in existence reported an unknown advance.

That is the same argument v1-S6 used for `image-xobject-drawn` — *"the corpus contains NO image
XObject anywhere, so without this the whole image path is untested"* — and it is worth stating
that the argument was available for four versions before anyone applied it here.

| | `/Encoding` | `extract` |
| --- | --- | --- |
| `composite-font-cid-widths` | `/Identity-H` | advance **2880** centipoints, ink box **2880** wide |
| `composite-font-non-identity-cmap` | `/UniJIS-UCS2-H` | text still `ABCE`, advance **absent** |

The pair differs **only** in the encoding: same descendant, same `/W`, same `/DW`. `/W` is keyed
by CID and the code→CID map is the `/Encoding` CMap, which this profile does not parse — so under
Identity the code *is* the CID and the widths are readable, and under anything else a width would
be a plausible number for the wrong glyph.

`2880` is `500 + 750 + 900 + 250` glyph units at 12 pt, and the four numbers come from four
different code paths: `/W`'s array form twice, then `/DW`, then `/W`'s range form. **`/DW` is 900
and not 1000 on purpose** — 1000 is also §9.7.4.3's value for an omitted `/DW`, so at 1000 a
mutant that ignored the key survived the fixture. The omitted case is covered by a unit test in
`fonts.rs`, where a dictionary can be built without one.

## Regenerating

```bash
python3 fixtures/engine/make_fixtures.py fixtures/engine
python3 fixtures/office/make_fixtures.py
```

Both are deterministic — no timestamps, no ids, no compression — so two runs produce identical
files. If a regeneration moves an existing fixture's digest, stop and find out why.
