# T1 — What the unruled rule refuses, and whether it was right to

**Run 2026-09-10 against 0.54.0** over the 200 `opendataloader-bench` documents, with the
benchmark's own `ground-truth/reference.json` as the label. The instrument is
[`refusals.py`](refusals.py); it reads the engine's output and computes nothing the engine does
not already disclose.

**This document ends in a method failure, not a threshold.** The measurement ran, the numbers are
real, and they cannot answer the question that was asked. §4 says what would.

---

## 1. Why this was run

`table-gate-v1.md` reports fabrication **0** and a geometric macro F1 whose median is **0**. The
opendataloader-bench record reports **TEDS 0.1038**, 37 of 42 documents at zero. Those are two
views of one mechanism: `unruled-table-candidate-refused` fires on **199 of 200** documents.

The engine is not failing to find tables. It builds candidates and refuses them. So the scoping
question is not *"can a detector be built"* but *"what may a refused candidate emit"* — and that
cannot be answered by counting refusals, because a refusal on a page with no table is the rule
working correctly. It needs the refusals crossed with ground truth.

## 2. What was found

**The engine emits a table on 5 of the 42 documents whose ground truth holds one.** That
reproduces the record's 5-of-42, from a different direction.

**One precondition accounts for every refusal.**

| variant | refusals | page holds a table | page holds none |
| --- | ---: | ---: | ---: |
| `gutter_below_floor` | **199** | 42 | 157 |
| `faces_without_text` | 0 | — | — |
| `lattice_too_large` | 0 | — | — |
| `emission_order_not_row_major` | 0 | — | — |

The other three variants exist, are wired, and **never fire on this corpus**. Every refusal here
is a column gutter under the 1 200-centipoint floor.

**And the gutter cannot tell a table from prose.** Column gaps that failed the floor, split by
whether that page actually holds a table:

| | n | min | p25 | median | p75 | max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| page **has** a table | 42 | 153 | 326 | **416** | 564 | 848 |
| page has **no** table | 157 | 192 | 319 | **416** | 620 | 1186 |

Identical medians. Overlapping quartiles. Overlapping ranges.

## 3. The obvious reading, and why it is wrong

The obvious reading is *"the floor is in the wrong place, and moving it cannot help because the
two populations are the same"*. The first half does not follow and the second is measuring the
wrong thing.

[`unruled.rs`](../../../crates/ethos-parser-pdf/src/unruled.rs)'s `gutter_fault` is
`.windows(2).map(...).find(|gap| *gap < floor)` — **the first adjacent pair below the floor**, over
lines sorted ascending. Not the smallest, not the typical, not the one nearest a table. The
leftmost.

Every document in this corpus is a **single page**. On any single page containing prose, the
leftmost sub-12pt gap between column lines is word spacing inside a text block — and it will be
found and reported whether or not a table sits elsewhere on that page.

**So both rows of the table above are measuring the same thing: the leftmost word gap on a page.**
They agree because they are the same quantity, not because tables and prose are indistinguishable.
The identical medians are an artifact of which fault the engine chooses to disclose.

This is the failure mode [`19-BLOCK-SUBDIVISION-SCOPE.md`](../../19-BLOCK-SUBDIVISION-SCOPE.md) §3
records three of: *a guard that reads its own subject wrongly passes forever*. Recorded here before
anything was built on it.

## 4. What this settles, and what it asks for

**Settled: the floor is not the lever.** No single-number change to `COLUMN_GUTTER_MIN` can be
justified from this data, in either direction, because this data does not measure the candidate's
gutter. Any proposal that starts *"lower the floor to N"* is unscoped until §4's disclosure exists.

**Settled: three of the four preconditions are untested on real documents.** They fire zero times
across 200 documents. Whatever they do, this corpus does not exercise it.

**Asked for — the smallest change that would answer the question.** The refusal currently
discloses one fault per page. To decide what a refused candidate may emit, it needs to disclose the
candidate:

1. **How many faults**, not just the first. One near-miss and forty word gaps are different pages
   and currently produce identical prose.
2. **The gutter distribution of the candidate lattice**, not one sample from it — the same
   band-versus-sample distinction `19` §5 had to make for leading, where local windows lost 5–20 pp
   against a band-wide mode.
3. **The candidate's extent** — how many lines and faces it implied before the fault, which is what
   says whether anything was there worth emitting.

That is a change inside `unruled.rs` and its limitation detail. **It emits no table, changes no
grid, and moves no threshold**, so it does not touch decision D1 — it is what makes D1 answerable.

**Not asked for: a confidence score.** Counting faults and reporting a distribution is disclosure.
Grading how close the candidate came is the thing `01-CONTRACT.md` §9 refuses, and none of the
above requires it.

## 4b. T1b — the lattice, rebuilt out-of-band, and the answer is worse than §4 assumed

§4 asked for a wider refusal disclosure. **Do not build it.** The instrument that would have
justified it, [`lattice.py`](lattice.py), was written as a probe instead — reproducing
`unruled::fold` over the run origins the artifact already carries, rather than enriching a
limitation detail that sits inside `representation_c14n_sha256` on 199 of 200 documents. That is
`structelem.py`'s rule: changing the product to justify changing the product is not a measurement.

**Over 194 pages** (the 5 documents where the engine accepts a ruled table are excluded, not
approximated, because `leftover` is then not every run):

| | table page (n=37) | prose page (n=157) |
| --- | ---: | ---: |
| column lines, median | 180 | 157 |
| **faces implied, median** | **6 672** | **4 628** |
| gaps at/over the floor, median | **0** | **1** |
| widest gap on the page, median | 1 161 | **1 288** |
| pages with ≥1 gap at/over the floor | **48%** | **56%** |

**The signal is absent, and on three of five measures it is inverted.** Table pages carry *fewer*
clear gutters than prose pages and a *narrower* widest gap. No threshold over this lattice
separates the two populations, so no disclosure of it would have helped.

### Why, and it is not the floor

`MAX_FACES` is **4 096** ([`unruled.rs:176`](../../../crates/ethos-parser-pdf/src/unruled.rs)). The
median candidate is **4 628 faces on a prose page and 6 672 on a table page**. Both are past the
ceiling that exists to refuse them.

`LatticeTooLarge` nonetheless fires **zero** times, because `detect` checks the gutter floor
**before** the size cap — the module header numbers the preconditions 1-6, but the execution order
is 3, 2, 6, 3, 4, and step 2 returns first on essentially every page.

So the refusal a consumer sees — *"two adjacent column lines sat 416 centipoints apart"* — is the
**less informative of two that both apply**. The honest description is the other one: a 6 672-face
lattice folded from every run on the page, which was never a table candidate. It is a
word-position index.

### What this settles

**The lever is the candidate, not the disclosure and not the floor.** `fold` over every run's
x-origin on a whole page — header, footer, body and all — cannot produce a table candidate. 180
column lines at a 150-centipoint tolerance is a histogram of where words start.

**A table candidate has to be built inside a bounded region.** That is what the block cut (plan
5.5) produces, and it makes the block cut a **prerequisite** for the table work rather than a
sibling of it. It is also the strongest argument yet for deciding D1: without blocks there is no
region to build a candidate in, and the unruled rule has nowhere to stand.

**Two contained repairs are worth making regardless**, and neither needs a decision:

1. **Check the size cap before the gutter floor.** A 6 672-face lattice should be refused as
   oversized, which is true and useful, rather than as a word gap, which is true and misleading.
   Same refusals, better reason, and `LatticeTooLarge` stops being dead code in practice.
2. **State in `table-gate-v1.md` that the unruled rule is page-scoped.** Its measured band already
   says the detector finds almost nothing; this says why, which the band does not.
## 4c. T2 — the unruled rule cannot detect a real table, and the reason is structural

§4b concluded that a candidate needs a bounded region and that the block cut would supply one. The
block cut shipped (`gutter-columns-v3`), so the prediction was testable.
[`block_scoped.py`](block_scoped.py) and [`preconditions.py`](preconditions.py) test it, plus two
further repairs. **Four experiments, and none of them emits a table.**

### The prediction held, and it did not help

Scoping the candidate to a block collapses the lattice by 30–40×:

| scope | median faces | under `MAX_FACES` |
| --- | ---: | ---: |
| page, table present | 6 435 | 35% |
| page, prose | 4 628 | 42% |
| **block, table present** | **154** | **96%** |
| **block, prose** | **208** | **95%** |

So candidates now *reach* the preconditions that test whether a grid is there. Which gate decides:

| gate | page scope | block scope |
| --- | ---: | ---: |
| `gutter_below_floor` | 82 | **649** |
| `lattice_too_large` | 117 | 29 |
| `faces_without_text` | 0 | 10 |
| no candidate | 1 | 346 |
| **would emit** | **0** | **0** |

The bottleneck moved from the size cap to the gutter floor. Nothing was gained.

### Three structural blockers, each independently fatal

**1. The fold tolerance and the gutter floor contradict each other.** `fold` groups x-origins within
`ALIGN_TOLERANCE` = 150 centipoints, so **every word start becomes its own column line** — a table
cell holding three words produces three columns. The gutter floor then refuses any adjacent pair
closer than 1 200 centipoints, which word starts always are. Measured inside the largest block of
each table page: **median 100 column lines at 150cp against 31 at the floor, a 3.6× over-count.**
One constant creates what the other forbids.

**2. Folding at the floor removes that, and occupancy then refuses everything.** With the fold at
1 200/600, `gutter_below_floor` disappears by construction — and **all 656 candidates die at
`faces_without_text`**. Zero emit.

**3. Occupancy does not discriminate, at any threshold.** The rule demands every face hold a run.
Measured occupancy of block candidates folded at the floor:

| | n | median | at 100% |
| --- | ---: | ---: | ---: |
| block on a table page | 123 | **54%** | **0** |
| block on a prose page | 533 | **54%** | **0** |

Identical medians. And relaxing it is worse than useless — at every threshold **more prose blocks
pass than table blocks**, because prose blocks outnumber them 4.3×:

| threshold | table blocks | prose blocks | best-case precision |
| --- | ---: | ---: | ---: |
| ≥90% | 2 | 5 | 29% |
| ≥80% | 7 | 19 | 27% |
| ≥70% | 22 | 71 | 24% |
| ≥60% | 41 | 175 | 19% |

**4. Cell grouping does not rescue it.** Merging horizontally adjacent runs into cells before folding
— the obvious answer to blocker 1 — yields **3 candidates that would emit at a 4pt cell gap, of
which 2 are on prose pages**, and 1 at 8pt. The handful that pass are majority-wrong. That is the
fabrication this rule's strictness exists to refuse, arrived at from the other direction.

### What this settles

**Fabrication-0 and emits-nothing are one property of this rule, not two.** The strictness that
guarantees the first guarantees the second, and no constant in it can be moved to separate them:
blocker 1 is a contradiction between two constants, blocker 3 is an absence of signal in the
underlying quantity. Text alignment does not distinguish a table from prose on this corpus.

**So §5.3 of the plan is answered negatively.** There is nothing useful for a refused candidate to
emit, because the candidate carries no information about whether a table is there. Emitting cell
rectangles from a 54%-occupied word-lattice would be emitting the shape of the prose.

**This corroborates [`table-gate-v1.md`](../../table-gate-v1.md) v2-S22 with a mechanism.** That
section already concluded *"the alignment rule is not a gap to close in v1"* and that recovering the
missed tables *"needs a derivation that reads a table's geometry from something other than drawn
grid ink"*, leaving retire-or-rework as a version-boundary question for the owner. This says why, at
four named gates, on a corpus this repository does not own. **The question is unchanged and now
answerable on evidence.**

**What it does not say.** Nothing here touches the ruled or stroke-ruled rules, which do emit and
are exact where a producer draws the grid, nor the tagged rule, which reads what the document
declares. The gap is untagged tables whose producer drew no rules, and it stays open.

## 4d. T3 — reworking the unruled rule, and why the ruled rule is the better target

The owner chose **rework** over retire. This measures what a rework could reach, and finds the
choice was offered on a menu that was wrong: **the unruled rule is not where the tables are.**

### First, T2's own weakness, resolved

§4c compared blocks on table pages against blocks on prose pages, and said so: *"ground truth says
whether a `Table` is on that page, not whether it is this block"*. The reference file carries
coordinates, so [`gt_boxes.py`](gt_boxes.py) resolves it — normalized × page dimensions lands in the
same top-left centipoints the engine uses.

**It confirms §4c and inverts it.** Runs *inside* a real table box, folded at the floor: occupancy
median **37%**, against prose blocks' **54%**. Same column count. A real table is **sparser** than
prose, because it holds short cell contents scattered across many x-positions while prose packs
words densely along each line. Occupancy does not merely fail to discriminate; it points the wrong
way.

### What the real tables actually look like

Reading one table's runs out of the artifact shows three instrument problems, not an absent signal:

- **Runs are split mid-word.** `'Y'` + `'outh Federations of Cambodia'`; `'Cambodian W'` +
  `'omen for Peace and'`. Each fragment contributes its own column line.
- **Columns are right-aligned.** One numeric column produced origins at 30934, 31184, 31593, 32093 —
  four spurious column lines for one real column, because `fold` sees only left edges.
- **Cells wrap across lines.** `'Union of '` then `'(UYFC)'` is one cell on two baselines, so
  lattice rows are not table rows.

Column 2 sat at x=8752 on 8 of 15 rows, dead consistent. The signal is there and the lattice cannot
see it.

### Fixing two of the three helps, and not enough

[`edge_alignment.py`](edge_alignment.py) joins mid-word fragments and folds **right** edges as well
as left, then counts alignment positions supported by at least half the rows
([`column_support.py`](column_support.py) is the unfixed version, for contrast).

| threshold | inside a table box | prose block | precision |
| --- | ---: | ---: | ---: |
| ≥2 strong columns | **48%** (19/40) | 23% (96/423) | 17% |
| ≥3 | 18% | 8% | 18% |
| ≥4 | 12% | 6% | 17% |

**A 2:1 likelihood ratio, where every earlier statistic was inverted.** That is real progress. It is
also nowhere near enough: prose blocks outnumber table blocks **10.6:1**, so 2:1 lands at **17%
precision** — five emissions in six would be wrong. Reaching 70% precision against that base rate
needs roughly **20:1**.

### And the tables are not there anyway

| of the 42 documents whose ground truth holds a Table | docs |
| --- | ---: |
| the **ruled** rule built a lattice from painted rectangles and refused it | **30** |
| only the unruled rule ever fired — nothing grid-shaped was drawn | 12 |
| already emit a table | 5 |

**The unruled rule's entire addressable population is 12 of 42.** Even a perfect unruled rule leaves
30 documents untouched, and its measured ceiling is 17% precision.

### The ruled rule fails on one precondition, 30 times out of 30

Every one of those 30 documents is refused by the same clause, and the artifact names it:

> **a cell the ink does not draw** — *"The rectangles implied a grid whose cells they do not all
> draw. A ruled grid must be explained by the ink face by face… A rectangle merely ENCLOSING the grid
> does not count."* — `9 rectangles implied 15 cells`

Across the 30, a median of **57%** of implied cells are drawn (p25 30%, p75 93%).

**This is structurally the same precondition as the unruled rule's occupancy — and epistemically the
opposite.** The unruled rule asks whether inferred alignment explains an inferred grid. The ruled
rule asks whether **ink the document actually painted** explains a grid **that same ink implied**.
Completing a partially-drawn grid from the lines its own producer drew is interpolation within
stated geometry. Inferring columns from word positions is not.

And the shortfall has an obvious cause: a producer that draws row separators but no column
separators, or an outer border plus horizontal rules, has fully specified its grid in *lines* while
drawing few of its *cells*. The precondition counts faces.

### The recommendation

**Rework the ruled rule's coverage precondition, not the unruled rule.** It reaches 30 of 42
documents against 12, it fails on one named clause rather than three, its evidence is drawn ink
rather than inferred alignment, and its 57% median coverage is a shortfall with a nameable cause
rather than a 2:1 signal against a 10.6:1 base rate.

**What stays true about the unruled rule.** §4c's conclusion is unchanged: it cannot work as
designed. T3 adds that even reworked it addresses 29% of the population at 17% precision. Retiring
it is now better supported than reworking it — but that is the owner's call, and it is no longer the
question that matters.

## 4e. T4 — 5.7 was the wrong fix. The lattice invents rows the page did not draw

5.7 was scoped as *"group rectangles into spatially connected candidate grids"*, on the reasoning
that the page-wide lattice extent is what defeats tracing. Designing it against real pages —
rectangles are not on the wire, so this needed a throwaway dump, since removed — showed the premise
is wrong.

### What a refused table page actually paints

`01030000000045.pdf`, one ground-truth table, currently refused. Its nine rectangles:

```
row 1   y=[20777,23412]   x=[5400,8352]  [8352,27347]  [27347,37273]
row 2   y=[26047,28682]   x=[5400,8352]  [8352,27347]  [27347,37273]
row 3   y=[30338,31893]   x=[5400,8352]  [8352,27347]  [27347,37273]
```

**That is a complete 3 × 3 cell grid with every cell drawn.** Nothing is missing, nothing is
scattered, and there is no page furniture to cluster away. `01030000000046.pdf` (35 rectangles,
7 columns) and `01030000000047.pdf` (28, 7 columns) are the same shape.

### Why it is refused, exactly

The rows have **gaps between them**: row 1 ends at 23412, row 2 begins at 26047. So `Lattice::build`
clusters six y edges into five bands, of which **two are the whitespace between drawn cells**:

| band | y | covered |
| --- | --- | --- |
| 1 | 20777–23412 | yes, 3 cells |
| 2 | **23412–26047** | **nothing** |
| 3 | 26047–28682 | yes, 3 cells |
| 4 | **28682–30338** | **nothing** |
| 5 | 30338–31893 | yes, 3 cells |

Five bands × three columns = 15 faces, of which the nine drawn cells cover nine. **That is the
refusal's own arithmetic — `9 rectangles implied 15 cells`** — and the six uncovered faces are all
inter-cell whitespace. Tracing fails for the same reason: the vertical line at x=8352 has edges only
where cells are, so its union has gaps of 2 635 and 1 656 centipoints, far past
`LATTICE_TOLERANCE`.

**The rule is refusing a perfectly drawn grid because it inserted rows the page never drew.**

### So 5.7 is a different change

Not clustering. **A band no rectangle occupies is not a row of the grid** — it is the space between
cells — and dropping such bands turns this page's 15 implied faces into 9 implied faces, all
covered, which emits a 3 × 3 table.

**The guard still holds under it.** `background-panel-not-a-grid` paints three scattered bars into a
7 × 7 lattice. Dropping empty bands leaves at most the three rows and three columns the bars touch —
9 faces of which 3 are painted, 6 uncovered — so it is still refused, and tracing still cannot carry
a line across it.

### What it costs, and why it is not this increment

`Lattice` is `{ xs, ys }` with rows and columns derived as `len − 1`, and **a gap between rows cannot
be expressed as a line list**: dropping either line bounding an empty band merges the two real rows
around it and moves their geometry. The lattice has to carry explicit bands — `rows: Vec<(i64,i64)>`,
`cols: Vec<(i64,i64)>` — which touches 28 references and the five methods `rows`, `columns`,
`bounds`, `face` and `span_of`.

That is the honest scope of 5.7, and it is a structural change to the type rather than a precondition
tweak. **Emitting a 5 × 3 grid with six empty slots instead is the shortcut and it is refused**: the
document drew a 3 × 3, and a table claiming five rows where two are whitespace is a grid this engine
invented.

## 5. Reproducing

```bash
cargo build --release --locked
ETHOS_BENCH_CORPUS=~/ethos-external-benchmarks/opendataloader-bench \
  docs/measurements/table-refusals/refusals.py
```

The engine is deterministic and the script only reads its output, so two runs agree exactly. The
script prints any refusal line its patterns fail to match and calls the taxonomy incomplete when
that count is non-zero, because a parser that silently drops a variant reports a cleaner corpus
than the one it read.

**Corpus licence.** opendataloader-bench is a separate checkout and is not mirrored into this
repository. `ground-truth/reference.json` is read, never copied.
