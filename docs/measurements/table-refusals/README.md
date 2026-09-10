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
