# Fixtures

**This directory holds a manifest, not fixture bytes.**

The conformance corpus lives in the Ethos tree and is used **read-only**. `manifest.json`
references each fixture by path and `sha256`. Copying would invite drift; a hash-pinned manifest
makes a fixture change a visible event in this repo.

## Resolution — three roots

| Root | Default | Override | What it holds |
| --- | --- | --- | --- |
| `conformance` | `../ethos/fixtures` | `ETHOS_FIXTURES` | The 15-fixture conformance corpus. **The M6 oracle criterion counts exactly these** |
| `benchmark` | `../ethos/benchmarks/gate-zero/corpus` | `ETHOS_BENCH_CORPUS` | Large real-world PDFs that M2's acceptance names. Not part of the oracle count |
| `engine` | `fixtures/engine` | `ETHOS_ENGINE_FIXTURES` | CC0 PDFs authored **here**, for behaviours the Ethos corpus does not cover. Never part of the oracle count |

The second root exists because M2's load-bearing acceptance tests name documents the conformance
corpus does not contain. The bounded-cost A/B needs a 492-page PDF (`nist-sp-800-53r5`); the
exit-code-0 case needs a real simple document (`irs-form-1040-2025`). Both live in Ethos's benchmark
corpus, not `fixtures/`. Keeping them in a separate root means they cannot inflate the 15.

If a root is missing or a hash does not verify, the harness **fails with a named error**. It never
skips, and it never silently uses a different file.

## Contents — 15 conformance + 3 benchmark

| Group | Root | Count | Notes |
| --- | --- | --- | --- |
| `synthetic/` | conformance | 9 | Ethos-authored, each with committed goldens |
| `failure/` | conformance | 5 | Fail-closed cases |
| `foreign/` | conformance | 1 | OpenDataLoader round trip |
| benchmark | benchmark | 3 | `nist-sp-800-53r5` (492 pp), `nist-sp-800-63b` (80 pp), `irs-form-1040-2025` (2 pp) |

Two entries are worth knowing before you debug against them:

- **`synthetic/table-regular-grid` did not open under `lopdf` until v0.1.** Its xref entries are 19
  bytes where PDF 32000-1 §7.5.4 requires exactly 20. PDFium repairs it; `lopdf` rejects it. One
  valid document in ~26 on this corpus. v0 exited 2 and **declared** the limitation; v0.1 pads the
  entries under published preconditions and reads it, declaring `xref-entry-padded` on every
  artifact (`docs/01-CONTRACT.md` §8.1). If you are debugging against an older build, that exit-2
  is expected behaviour there, not a bug. **It is also the v1-S2 unruled golden**: six `Tm`/`Tj`
  pairs and zero path operators, so it was invisible to the ruled detector and now yields its 3×2
  grid under `unruled-align-v1`.
- **`failure/memory-limit-simulated` is byte-identical to `synthetic/simple-text`.** Observed, not
  assumed — both hash to `f2f6ab91…`. The limit is simulated by configuration, not by a distinct
  document. A test asserting "different fixture ⇒ different bytes" would be wrong here.

## Engine-owned fixtures

The manifest marks every entry with an `owner`. The 15 conformance entries are `ethos`; **eleven**
are `engine` — authored here, under CC0, by `engine/make_fixtures.py`, each for a behaviour the
Ethos corpus genuinely cannot cover.

| Fixture | Slice | What only it proves |
| --- | --- | --- |
| `show-text-quote-operators` | M3 | The `'` and `"` operators, whose omission is pdf-inspector's disqualifying defect |
| `horizontal-scaling-tz` | M3 | `Tz` changes the advance |
| `synthesized-space-tj` | M3 | A `TJ` gap wide enough that a space was intended and never written |
| `measured-ink-box` | M3 | A `/FontDescriptor` with real ascent/descent, so ink is **measured** |
| `absent-font-metrics` | M5 | A descriptor that exists and declares **no ink extent**, with the advance known |
| `broken-font-encoding` | v0.1 | `/Differences` pointing at glyph names no table carries, so the run is dropped rather than turned into mojibake |
| `ruled-table-grid` | v1-S1 | A 3×3 grid **drawn** with `re`, one merged cell and one empty cell — the ruled golden |
| `ruled-table-overlap` | v1-S1 | Two rectangles claiming one lattice face, so the cross-check must report mismatch and repair nothing |
| `unruled-near-miss` | v1-S2 | Columns that align on two rows and miss on the third by five points: **no table**, plus a named refusal |
| `both-table-rules` | v1-S2 | One painted grid and one aligned-text grid on a page, so the artifact carries two tables under two rule ids |
| `ruled-wins-shared-region` | v1-S2 | A painted grid whose text is *also* a clean alignment grid — one table comes out, and it is the ruled one |

**Why the corpus cannot cover the table fixtures.** No fixture in the Ethos conformance corpus
contains a single path operator, so nothing there can exercise ruled detection at all. And
`synthetic/table-regular-grid` covers exactly one unruled shape — a clean, complete grid. The near
miss, the two-rules page and the shared region are the cases where a detector goes wrong, and none
of them exists upstream.

**Why `absent-font-metrics` is not redundant**, since the answer is narrower than it first looks
and the first draft of this paragraph got it wrong:

Geometry can be absent for more than one reason, and it is tempting to say this fixture is the only
one pairing a **known advance** with an absent box. That is false, and measured: `/Widths` has
always been supplied to every engine fixture, so `show-text-quote-operators`,
`horizontal-scaling-tz` and `synthesized-space-tj` already produce seven fully-addressable runs
(page + origin + advance) with no groundable box between them.

What only this fixture covers is one specific route through `resolve_font_ink`: a
`/FontDescriptor` that **resolves and answers nothing**. The other three carry no descriptor at
all and exit at the first gate; `synthetic/simple-text` supplies no `/Widths`, so its advance is
unknown and `extract` never calls `ink_box` in the first place — it cannot stand in for a metrics
test it never reaches. Here the reader opens a descriptor, finds no `/Ascent`, no `/Descent`, no
`/FontBBox` and no embedded program, and still refuses to invent a box. That is the shape most
likely to tempt a `height = font_size` fallback, which is the defect the whole typed-absence design
exists to prevent.

It is a **dedicated** fixture rather than a reused one on purpose: the three M3 fixtures happen to
take an absence path today, but each exists to prove something else, and a later edit to one of
them — adding a descriptor to test something new — would silently stop testing omission.

**The 15-fixture oracle criterion counts only `owner: "ethos"` entries.** Engine-owned fixtures are
additional test assets and never inflate that number.

### `two-column-14-lines` and `two-column-15-lines` are a pair, and only useful as one

Neither fixture proves anything alone. They are the **same page** — two columns, written right
column first — differing by one line in the left column, and they sit on either side of
pdf-inspector's `min_lines < 15` boundary, where fourteen lines produce row-interleaved order and
fifteen produce column-major (`docs/03-V0-SCOPE.md` §3.2). `gutter-columns-v1` must read both the
same way, and `one_added_line_does_not_reorder_the_page` asserts exactly that.

So: **edit them together or not at all.** Changing the line count in one without the other, or
letting their columns creep closer than the rule's 12pt gutter floor, leaves a test that still
passes while testing nothing. The line counts are not numbers the engine knows — the rule measures
whitespace — they are chosen to sit where a line-counting rule would give itself away.

## Regenerating the engine-owned fixtures

```bash
python3 fixtures/engine/make_fixtures.py fixtures/engine
```

Deterministic: no timestamps, no ids, no compression, so two runs produce byte-identical files and
re-pinning a hash in `manifest.json` is a review of an intended change rather than of churn. A
regeneration that moves an existing fixture's digest is a signal to stop and find out why.
