# `stroke-ruled-v1` — built at S7b, parked, then SUPERSEDED by v1-S8

> **Status: superseded. The rule is now in the tree and running.** v1-S8 revived this patch, found
> that both of the failures recorded below have a single cause, fixed it, and shipped the result as
> `crates/ethos-parser-pdf/src/stroke_ruled.rs` at engine **0.10.0**.
>
> **The cause was not in the band preconditions this file defends — it was one line in `extract`,
> which discarded every non-horizontal segment before the rule ever saw one.** So "where are the
> columns" was decided entirely by where horizontal rules happen to *end*: too strict for page 13
> (whose blank first cell leaves one baseline ruling three cells of four) and too loose for the
> Closing Disclosure pages (where two unrelated rules ending at the same x imply a boundary nobody
> drew). Reading the page's vertical ink fixes both at once.
>
> | | this patch | shipped at S8 |
> | --- | --- | --- |
> | `cfpb` page 13, the 8 × 4 worksheet | **refused** | **7 × 4 emitted** |
> | `cfpb` pages 22, 23, 24-second | emitted, gold tags none | **refused** |
> | `cfpb` cell-F1 | 210‰ (**worse than 246‰**) | **259‰** |
> | `irs-form-1040-2025` | **4 tables** | **0 tables** |
> | macro | 125‰, from the canary | 64‰, from `cfpb` alone |
>
> The 1040 is held at zero by a second precondition: a face whose four edges are a **form field's**
> four edges is that field's box. Measured, a 1040 face and its widget `/Rect` are the same box edge
> for edge, while page 13 — also a fillable worksheet, carrying 25 widgets — insets its fields well
> inside larger printed cells.
>
> The clippy gap this file names as "the slice's one real incompleteness" is closed:
> `stroke-ruled-table-candidate-refused` is wired through `limitations.rs` with an accumulator, and
> `stroke-ruled-tables-not-detected` is retired rather than left to lie.
>
> **The patch is kept for the record, not for reuse.** It no longer applies to HEAD, and the
> measurement below is only checkable against `683031b`. Current account:
> `docs/table-gate-v1.md` and `docs/history/09-V1-MILESTONES.md` S8.

---

Parked at **v1-S7b**, against `683031b`. This directory held a complete, working detection rule
that the repository deliberately did not run.

It is here because throwing it away would mean the next person pays the build cost again to learn
the same thing, and because the measurement below is only checkable if the code that produced it
still exists.

## What it is

A third table-detection rule, beside `ruled-rects-v2` (grids from filled rectangles) and
`unruled-align-v1` (grids from text alignment). This one reads the **ruling lines** a page strokes.

`crate::content::flush_subpath` needs four or five points spanning two distinct x and two distinct
y before it calls a subpath a rectangle, so a two-point `m`/`l` stroked with `S` produces nothing
at all. That is the `stroke-ruled-tables-not-detected` limitation the engine has declared since
v1-S2 (`174e27f`; v1-S1 declared the broader `unruled-tables-not-detected`, retired at S2), and
v1-S7b measured what it costs: **every one of the nine tagged tables
`cfpb-home-loan-toolkit` misses sits on a page that strokes segments — 103 cells, 65% of that
document's gold.** Its page 13 draws an 8 × 4 loan worksheet as 32 horizontal rules and nothing
else.

The rule, as built:

1. **Rule-rows** — horizontal segments sharing a baseline within `LATTICE_TOLERANCE`.
2. **A row must tile** — at least two segments running end to end. Scattered underlining implies
   nothing.
3. **A band** — consecutive rows whose every endpoint is already a column line of the first. A row
   may rule fewer cells than the band has columns (a blank cell does) but may not introduce a
   boundary. Requiring exact equality instead split page 13's worksheet into a 4 × 4 and a 3 × 4 at
   its one blank-first-cell row.
4. **Rows are the regions between rules** — *n* baselines bound *n − 1* rows. A form ruled under
   each cell does not draw its first row's top edge and this rule will not supply one.
5. **Coherence** — every face's own bottom edge must be drawn.
6. **2 × 2 minimum**, matching `unruled-align-v1` — **not** `ruled-rects-v2`, which requires only
   two *faces* and does emit 1 × 2 grids (cfpb pages 16 and 17). The 4096-face cap is shared with
   the ruled rule; `unruled.rs` still keeps its own copy of the same number, so the patch's comment
   claiming one constant rather than three overstates what it did.

Segments are captured as their own evidence — a `PathSegment` beside `PathRect` — because a box
says *this cell is here* and a line says *this edge is here*. Collapsing a segment into a
zero-height rectangle would have let the ruled lattice consume it, and a zero-area cell is a cell
nobody drew.

## What it measured

| | `ruled-rects-v2` | `+ stroke-ruled-v1` |
| --- | --- | --- |
| detected / matched | 8 / 8 | 21 / 15 |
| precision | 1000‰ | 714‰ |
| cells emitted | 36 | 272 |
| **fabricated** | 0 | **0** |
| cross-check disagreements | 0 | 0 |
| `cfpb-home-loan-toolkit` | 24 TP / 12 FP → **246‰** | 41 TP / 189 FP → **210‰** |
| `irs-form-1040-2025` | 0 TP → 0‰ | 12 TP / 30 FP → **292‰** |
| **MACRO cell-F1** | **61‰** | **125‰** |

The capability is real: 17 `cfpb` cells and 12 `1040` cells no rule had ever found, and NIST plus
all **three** gold negatives at zero.

### What it actually emits on `cfpb-home-loan-toolkit`, and the surprise in it

| Page | emitted | gold there |
| --- | --- | --- |
| 6 | 5 × 2 | 7 × 2 |
| **7** | **7 × 2, 14 cells** | **7 × 2, 14 cells** |
| 22 | 5 × 4 and 7 × 4 | none |
| 23 | 3 × 3 | 5 × 3 |
| 24 | 9 × 4 and 7 × 3 | none |
| 25 | 4 × 2 and 8 × 6 | 2 × 2 |

Page 7 is an exact shape hit. **Page 13 is not on this list at all** — the 8 × 4 loan worksheet
that motivated the whole slice is *refused by the rule's own step 5*. Its eight baselines include
one carrying three segments rather than four, so one face has no rule under it and coherence
declines the band. An earlier draft of this file claimed page 13 "comes out at exactly 8 × 4"; that
was the shape of the **ink**, measured before the emission path existed, and it is not what the
rule produces. The correction matters: the rule refuses the table it was built for.

It also relocates the over-detection. The 189 false positives are concentrated on pages 22, 24 and
25 — Closing Disclosure pages that stroke hundreds of segments and carry little or no tagged table
— not on scattered page furniture as first written.

## Why it is parked rather than shipped

1. **It regresses the document it was built for**, 246‰ → 210‰ — 17 more right cells bought with
   177 more wrong ones, and it **refuses page 13**, the table the slice existed to find. Of the
   false positives, **156 are text appearing in no gold cell at all** — genuine over-detection,
   overwhelmingly from the Closing Disclosure pages 22, 24 and 25. *(That 156 came from a scratch
   probe that classified every emitted cell against the gold; it is not printed by any committed
   test, so treat it as a recorded observation rather than something the harness reproduces.)*
2. **Four canary tests fail.** `irs-form-1040-2025` yields 4 tables where every slice since v1-S1
   has held it at 0. That guard exists because of the 662-cell fabrication, and its message says
   flipping it *"is a deliberate decision needing its own evidence"*. **The decision was taken and
   it is to keep 1040 at zero tables.**
3. **The macro gain comes entirely from the canary document.** 1040 going 0‰ → 292‰ is what doubles
   the average, while `cfpb` — where the rule genuinely helps — gets worse. A metric that rewards
   flipping the guard should not be what decides to flip it.

Fabrication stayed 0 and the cross-check clean throughout, so this is over-detection rather than
invention. A real distinction, and not a defence: 156 cells of real text in regions nobody tags as
tables is the failure the gold negatives exist to catch.

## Reviving it

```bash
git apply docs/attic/stroke-ruled-v1/stroke-ruled-v1.patch
```

Then measure, which is the only way to know whether any of the above still holds:

```bash
cargo test -p ethos-parser-pdf --lib accuracy --locked -- --nocapture
```

**The fixture corpora are not found automatically from an arbitrary worktree.** `fixtures/manifest.json`
resolves its defaults relative to the repository root — `../ethos/fixtures` and
`../ethos/benchmarks/gate-zero/corpus` — which only works for a tree sitting beside the sibling
`ethos` checkout. From anywhere else, set `ETHOS_FIXTURES` and `ETHOS_BENCH_CORPUS` first, exactly
as `.github/workflows/ci.yml` does.

**The patched tree does not pass clippy, and CI runs it with `-D warnings`.**
`Refusal::kind`, `explanation` and `detail` are defined and never called, so
`cargo clippy --workspace --all-targets --locked -- -D warnings` fails on dead code. That is not a
lint to silence: it is the slice's one real incompleteness. The other two rules route their
refusals into a declared limitation — `ruled-table-candidate-refused`,
`unruled-table-candidate-refused` — and this one never got that wiring, so a page whose ruling
lines imply a grid the ink does not explain is refused **silently**. Anyone reviving this should
wire it through `crates/ethos-parser-pdf/src/limitations.rs` rather than reach for `#[allow(dead_code)]`;
standing rule 3 wants the disclosure either way.

The canary failures live in a different target, which `--lib` cannot reach:

```bash
cargo test -p ethos-parser-pdf --test extraction --locked
```

Expect four failures there —
`the_tax_form_does_not_become_an_alignment_lattice`, `form_fields_never_feed_the_table_detectors`,
`observing_images_changes_no_earlier_slices_answer`,
`reading_the_structure_tree_changes_no_earlier_slices_answer`. They are the point, not an
oversight: they are what says 1040 stopped being silent.

> **The bundled `stroke-ruled-v1.patch` predates the `ethos-engine` → `ethos-parser` rename and
> still names the old `crates/engine-*` paths. It is kept byte-for-byte as it was written, because
> rewriting an archived patch would make it claim to apply to a tree that did not exist when it was
> authored. The prose below uses today's crate names; the patch does not.**

The patch adds `crates/ethos-parser-pdf/src/stroke_ruled.rs` — the 348-line module that **is** the rule —
and modifies `content.rs`, `extract.rs`, `tables.rs` and `lib.rs` in `ethos-parser-pdf` plus `profile.rs`
and `lib.rs` in `ethos-parser-core`. `git apply --check` first; `git apply -R` backs it out. The six
pre-image blob hashes in the patch header are the real applicability test — they match `683031b`
today, and when they stop matching the module is the part worth keeping.

Wiring it properly is not a morning of hand-waving; it is at least: a
`codes::STROKE_RULED_TABLE_CANDIDATE_REFUSED` and a `lim::` builder mirroring the two in
`limitations.rs`, with an accumulator in `extract.rs`; the `TableDetection` third field, its schema
and a new digest; **retracting `stroke-ruled-tables-not-detected`**, which the patch leaves in place
so a patched engine emits stroke-ruled tables while every artifact still declares it does not; and
a capability proof test.

Runtimes on the machine this was measured on: the accuracy harness ~150 s, the extraction target
~300 s. Neither is hung.

**It is not wired into the profile.** `TableDetection` has `ruled` and `unruled` and
`deny_unknown_fields`; shipping this would need a third field, a schema change and a new
`profile_sha256`. That was left undone deliberately — there was no point paying for a profile
change before the rule had earned it.

## Verified

Re-applied to `683031b` in a clean worktree and measured: the patch applies, it builds, and **every
figure in the table above reproduced exactly** — both columns, including the baseline. The four
canary failures are the four named above and no others. Two gaps in the revival instructions were
found that way and are now written into them: the fixture-path resolution, and the clippy failure.

## What a future attempt should know

**The band preconditions are sound and they reproduce** — but they refuse page 13 and accept
pages 22, 24 and 25. The problem is *which regions become bands*, and step 5 is stricter than the
documents are tidy: one row of a real worksheet ruled three cells instead of four and lost the
whole table. Every remaining tightening or loosening considered was a threshold fitted to these
four documents, which `docs/history/08-V1-SCOPE.md` §3 forbids, so the attempt stopped rather than tune.

The fuller record is `docs/table-gate-v1.md`: page 13's exact baselines and x-coordinates (enough
to write a unit test with no corpus at all), the per-page table of all nine missed cfpb tables, and
the forecast that even a *perfect* stroke-ruled rule capturing all 103 cells reaches only 852‰ cfpb
and 213‰ macro — still short of the 489‰ floor, because NIST and 1040 carry half the average
between them and stay at zero.

Two things to settle **before** writing code, because they are what decides the outcome:

- **The 1040 question.** Are a tax form's ruled entry boxes tables? The tree tags one table on that
  document and this rule finds four grids. Answer it first; the rule's fate follows from it.
- **The coherence/first-row pair.** A table ruled only under its cells does not draw its first
  row's top edge, so step 4 loses that row; and step 5 refuses the whole band if any single face
  lacks a rule, which is what kills page 13. Those two together are why the rule both under- and
  over-shoots. On page 13 that alone shifts
  every row index by one against the gold, which the cell-slot join charges twice — see the
  row-offset note in `docs/table-gate-v1.md`.

The lead does not go away: **103 `cfpb` cells, 65% of that document's gold, are still behind ink
this engine discards.**
