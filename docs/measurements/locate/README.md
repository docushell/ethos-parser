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

---

# D1 S3 — What a quote finds over `fixtures/gate`, and what the call costs

**Run 2026-09-18** on the same host, against the same tree, with the release binary. Instrument:
[`occurrences.py`](occurrences.py). Raw readings, the quote each document was asked for included:
[`gate-occurrences.json`](gate-occurrences.json).

```
cargo build --release
python3 docs/measurements/locate/occurrences.py /tmp/locate-s3
```

Two quotes per document, because one would answer the wrong question. **A realistic quote** is one
of the document's own drawn lines, truncated to 60 scalars — citation-shaped, and lifted from the
document rather than invented. **`the`** is not a citation and is not presented as one; it is the
largest occurrence list a caller could stumble into with a short quote, which is what a band needs
at its top end.

## 7. Occurrences

| document | repr MB | nodes | blocks | realistic: occurrences | its parts | `the`: occurrences | its parts | `the` artifact |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 | 0.7 | 1 098 | 423 | **1** | 1 | 37 | 37 | 5 608 |
| irs-fw9 | 0.9 | 1 097 | 358 | **1** | 3 | 340 | 340 | 47 815 |
| nist-sp-800-218 | 31.6 | 61 739 | 8 662 | **1** | 32 | 650 | 1 431 | 178 520 |
| nist-sp-800-207 | 46.9 | 91 004 | 7 292 | **1** | 49 | 1 117 | 2 705 | 334 164 |
| nist-sp-800-171r3 | 90.1 | 173 463 | 16 868 | **1** | 46 | 1 452 | 3 133 | 392 614 |
| nist-sp-800-37r2 | 227.4 | 436 051 | 43 185 | **1** | 43 | 4 447 | 11 668 | 1 438 790 |
| nist-sp-800-161r1 | 298.1 | 569 564 | 54 703 | **1** | 18 | 5 663 | 13 220 | 1 648 681 |
| **nist-sp-800-53Ar5** | **1 037.9** | **1 804 101** | **57 187** | **1** | **53** | **8 924** | **25 693** | **3 159 979** |

**A citation-length quote occurs exactly once, on all eight.** That is the number a caller cares
about and it is the same on a two-page form and a 733-page catalogue. The band for a short quote is
**37 to 8 924 occurrences**, and the worst document is `nist-sp-800-53Ar5` — named, per decision
#18 — at 8 924 occurrences and a 3.16 MB artifact.

**The cap is four orders of magnitude away.** 8 924 is the largest count any document in this corpus
produces, against a ceiling of 1 000 000 (§2). No document here comes close to it, which is the
honest framing of that ceiling: it bounds a pathological input, not a large one.

**One occurrence is 53 node parts on the largest document.** The NIST producers shred a line into
per-glyph runs — 1 804 101 nodes carrying 1 928 849 scalars, about 1.07 scalars a run — so 60
scalars of prose cross dozens of them. This is the measurement behind §5's design: an occurrence
that named one node could not describe these documents at all, and a caller could not have assembled
the answer from the runs, because knowing which runs are adjacent *is* the block rule. The same
shredding is why `the` costs 2.9 parts an occurrence on that document: a three-letter word crosses
runs.

**`nist-sp-800-161r1` took ten candidates**, where the other seven were answered on the first. Its
leading long lines are cover and front-matter lines whose letter spacing puts each piece in a block
of its own, so a 60-scalar quote drawn from one of them is not in any single block. The instrument
proposes and the engine decides — see §9.

## 8. What a call costs

| document | repr MB | wall ms | engine ms | MB/s |
| --- | ---: | ---: | ---: | ---: |
| irs-f1040sd-2025 | 0.7 | 20.0 | 9.5 | 74 |
| irs-fw9 | 0.9 | 23.6 | 12.7 | 71 |
| nist-sp-800-218 | 31.6 | 420.2 | 401.3 | 79 |
| nist-sp-800-207 | 46.9 | 601.7 | 581.0 | 81 |
| nist-sp-800-171r3 | 90.1 | 1 120.0 | 1 098.1 | 82 |
| nist-sp-800-37r2 | 227.4 | 2 845.9 | 2 821.6 | 81 |
| nist-sp-800-161r1 | 298.1 | 3 715.7 | 3 685.8 | 81 |
| nist-sp-800-53Ar5 | 1 037.9 | 13 513.3 | 13 384.3 | 78 |

Wall clock is measured around the process; `engine ms` is the subcommand's own `--diagnostics`
figure, so the difference — about 20 ms, flat — is process start-up.

**`locate` costs what loading the representation costs, and the search is a rounding error beside
it.** The two quotes cost the same on every document (401 ms and 392 ms on `nist-sp-800-218`;
13 384 ms and 13 302 ms on the largest), though one returns 1 occurrence and the other 8 924. The
throughput is flat at **78–82 MB/s of representation** across three orders of magnitude of input,
so the figure to plan with is one term: **a call costs about 20 ms plus the record's size at
80 MB/s.**

**The consequence for a caller is the library, not the subcommand.** Ten quotes against the
733-page record cost ten reads of a gigabyte through the CLI and one through
`ethos_parser_core::locate`, which takes the sealed representation by reference. The subcommand is
the right shape for one question and the wrong shape for a hundred; `docs/PUBLIC-API.md`'s
thin-shell row names the library call.

## 9. What the instrument does, and two things it got wrong first

It cannot lift a citation-length quote out of one run, because **no document in this corpus has a
run 60 scalars long** — `nist-sp-800-218`'s first three runs are `NI`, `S` and `T`. So a
citation-length quote necessarily spans runs, and the instrument **proposes and lets the engine
decide**: each candidate is one drawn line's own text, handed to `locate`, and kept only if it was
found. It does not reimplement the block rule, which would be a second implementation of the answer
this measurement is about.

Both of its first two attempts were wrong in ways worth recording, because both would have produced
a number:

1. **A fixed prefix of the artifact is not its nodes.** Canonical JSON sorts keys, so `geometry`
   precedes `representation`: four megabytes of `nist-sp-800-218` are geometry rows and no nodes at
   all. The scan now reads forward to `"nodes":[` first.
2. **Candidate starts spread blindly across the runs land mid-line**, and a 60-scalar quote from
   mid-line runs past the line's end — which is a block boundary, because a block never crosses a
   baseline. Forty such candidates found nothing on `nist-sp-800-218` and said nothing about the
   document. Candidates are lines now, which is also the finding worth carrying out of this
   section: **a quote a caller can locate is a quote that lies inside one drawn line.**
