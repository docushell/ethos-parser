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
