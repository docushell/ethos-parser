# D1 S1 — What a locations artifact costs at its ceiling, and the defect measuring it found

**Run 2026-09-18 against the tree this commit lands** (local `main` at `1b5a9c0` plus D1 S1), on
macOS 26.6.2, Mac16,8 arm64 (48 GiB, 12 cores). The instrument is a test rather than a script,
because the artifact it measures has no CLI subcommand yet — S2 is where one arrives:

```
cargo test -p ethos-parser-core --lib -- --ignored --nocapture the_ceiling
```

It is `the_ceiling_is_a_measured_number_and_the_excess_withholds_every_locator` in
`crates/ethos-parser-core/src/locate.rs`, `#[ignore]`d for the reason `accuracy.rs`'s generator is:
a gate step that allocates half a gigabyte to re-derive a number already written down here buys
nothing this page does not already say.

---

## 1. Why this was run

`docs/26-LOCATE-SCOPE.md` §5 took `LOCATE_MAX_OCCURRENCES` from `ethos.grounding.v1`'s element
ceiling by argument rather than by measurement, and said so: *"The number is provisional until S1
measures the artifact's size at it."* `docs/27-LOCATE-MILESTONES.md` S1 made that its acceptance —
*"no surface ships on an unmeasured cap"* — so this ran before the CLI, the MCP tool and the SDKs
exist to ship on it.

## 2. What the artifact costs

| shape | occurrences | bytes | per occurrence |
| --- | --- | --- | --- |
| one part, two-character node id | 1 000 000 | 133 778 286 | 133.77 |
| **three parts**, five- and six-character node ids | 10 000 | 3 269 396 | **326.93** |
| the excess, withheld | 1 000 001 | **564** | — |

**127.6 MiB at the ceiling for the cheapest occurrence there is** — one part, in a node whose id is
two characters, carrying one measured box. That is the floor, and the second row is the slope that
matters: a quote crossing three runs costs 2.4x as much per occurrence, so a million of those would
be **about 312 MiB** by the same figure. That number is a projection and is marked as one: a
million three-part occurrences need three million runs inside one block, and at that point the
representation rather than the answer is the thing being measured.

The third row is the shape past the ceiling. The count travels, every locator is withheld — all of
them, never the excess alone — and the whole artifact is 564 bytes. A truncated list would locate
some of a document's occurrences and drop the rest without saying which.

## 3. What producing it costs

| | debug | release |
| --- | --- | --- |
| wall clock, all three measurements | 9.3 s | 2.09 s |
| peak RSS, each phase scoped so the figure is one measurement's own | — | 527 450 112 B (**503 MiB**) |

Peak RSS is read from outside the process by `/usr/bin/time -l` on the built test binary, not on
`cargo test`, which would have measured the compiler. **It is ~4x the artifact**, because the
occurrence list, the million-scalar representation and the canonical bytes are alive at the same
moment. A caller who asks for a million occurrences is asking for that, and §5 says what follows.

## 4. The defect the measurement found

The first implementation re-counted every block member's scalars from the block's start on each
match — `occurrence_at` called `chars().count()` per member, per occurrence. That is
O(occurrences x block scalars), and the ceiling's own worst case (one block of a million scalars
holding a million matches) is on the order of 10^12 scalar counts. **It does not finish**, so there
was no number to measure.

The fix is a per-block prefix of member scalar starts, computed once, with the member a match
begins in found by `partition_point` rather than walked to. Cost became linear in the document plus
the answer, which is what made §2 and §3 measurable at all.

This is the argument for the rule that produced it: a cap defended by argument alone would have
shipped on a code path that could not reach it.

## 5. The decision

**The cap stays at 1 000 000.** Confirmed, not lowered, for four reasons:

1. It is `ethos.grounding.v1`'s `MAX_ELEMENTS`, and the reason the two numbers are equal is the
   reason the number is what it is (§5 of the scope). A test in the grounding crate —
   `locates_ceilings_are_the_grounding_artifacts_own` — fails if either moves.
2. 127.6 MiB is inside the family of sizes this engine already writes.
   `docs/measurements/memory-ceiling/README.md` records a **950 MB** representation for the largest
   gate document, and that artifact is not optional.
3. Withholding is all-or-nothing by design. A lower cap does not make a large document cheaper; it
   converts documents that would have received every locator into documents that receive none.
   That is a worse answer, not a safer one.
4. The cost is now linear in the document and in the answer, so the ceiling bounds the artifact
   rather than the wall clock.

**What this is not** is a statement that asking for a million occurrences is a good idea. §3's 503
MiB is the price of the pathological shape, and a caller who meets it is a caller quoting a single
character. The engine answers; the ceiling is where it stops handing out locators and starts
handing out a count.

## 6. What this page does not measure

Occurrence counts for a realistic quote over `fixtures/gate`, the band with the worst document
named, and the wall time of a `locate` call on the largest of them. Those are S3
(`docs/27-LOCATE-MILESTONES.md`), and they need the CLI S2 adds.
