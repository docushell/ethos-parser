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
| `engine` | `fixtures/engine` | `ETHOS_PARSER_FIXTURES` | 37 CC0 PDFs authored here, for behaviours the Ethos corpus does not cover |
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
