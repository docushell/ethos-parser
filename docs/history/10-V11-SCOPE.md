# 10 — v1.1 scope: Safe Markdown

**Scope authority for v1.1.** Slice detail is in [`11-V11-MILESTONES.md`](11-V11-MILESTONES.md), and
every v1.1 PR belongs to exactly one slice.

**v1 is not done.** Its table number is measured and missed. v1.1 is the next roadmap row and it
started because the owner asked for it, **not** because that gate cleared. Nothing here closes v1,
and no slice here may be cited as evidence that it did.

**Superseded by decision #18 (2026-08-30):** v1 closed on four capability clauses and a published
band — the engine reads the tables a document declares, detects ruled tables where the producer
drew the rules, emits nothing where neither holds, and fabricates nothing — not on the macro. The
paragraph above was true when written and did not survive the decision; it is kept as written. See
[`00-NORTH-STAR.md`](../00-NORTH-STAR.md) row 18, which is where v1's status is stated.

---

## 1. The one sentence

**A Markdown quote is either invertible back to canonical evidence, or explicitly unquotable.**

Everything below is machinery for that sentence. S4 makes it true of a second projection without
weakening it: replace "Markdown" with "HTML" and every law, test and erasure count still has to
hold, or `ethos.html.v1` does not ship.

## 2. Why this version exists

The contract says it *does not define a Markdown projection*, and gives the reason: **retrieval
operates on the evidence record itself, and any projection between what is ranked and what is cited
is where a locator dies silently.**

Here is that failure concretely. A pipeline chunks Markdown, embeds the chunks, ranks them, and hands
the winning chunk to a model. The model quotes it. The citation then has to bind back to *the
document* — a run, a cell, a page — and the Markdown has thrown that away. **Nobody notices, because
the quote is real text and the answer looks right.**

Competitors ship Markdown for retrieval and lose the cell. This repository declined to ship Markdown
for seven v1 slices for exactly that reason. **v1.1 does not lift the objection; it pays it**, by
making the map a structural precondition rather than a companion file somebody can forget.

**The gate is not "pretty Markdown."** Prettiness is not a property this version optimizes. Two
things are: a Markdown-quoted citation verifies end to end, and coverage completeness is asserted
rather than described.

## 3. What v1.1 is

| | |
| --- | --- |
| **Two artifacts** | `ethos.markdown.v1` and `ethos.html.v1` — canonical JSON, integer fields only |
| **Each carrying both halves** | the string **and** the anchor map that inverts it |
| **Plus a census** | `coverage`, accounting for every source character the representation offered — and the two artifacts of one document must agree on it exactly |
| **Produced by** | a projection of `DocumentRepresentation v0`, each under its own versioned rule id |
| **Proved by** | a quote lifted out of the projection that grounds through the existing verifier, and one carrying invented bytes that does not |

**The second artifact is not a rendering of the first.** It is projected from the representation,
because a Markdown-to-HTML pass would be a second projection whose map nobody built. What earns it a
slice rather than a stylesheet is tables: Markdown has no `rowspan`, so it must expand every merged
cell and count what that cost, while HTML carries the merge the document drew.

## 4. The four laws

Each is a way this version could fail quietly, so a reviewer checks these first.

### Law 1 — never one without the other

Markdown is never emitted without its map, and a map is never emitted without its Markdown. They are
**fields of one artifact**, not two files with a naming convention. There is no `--md-only`, and
adding one is a contract change rather than a convenience flag.

*Why structural rather than procedural:* a companion file is a thing a caller forgets, a pipeline
strips, or a cache drops. A field is not.

### Law 2 — the map tiles every byte

Every UTF-8 byte of the string belongs to **exactly one** segment. Segments are half-open byte
ranges, sorted, contiguous, with no overlap and no gap, covering the whole string.

**Incomplete coverage is a failed test, not a footnote.** A map with a hole is worse than no map,
because the hole is exactly where an unquotable byte hides.

### Law 3 — two segment kinds, and only two

| kind | means | invertible? |
| --- | --- | --- |
| `source` | bytes that came from canonical node text | **yes** — it names the node ids |
| `syntax` | markup the exporter invented — hashes, blank lines, fences | **no**, and it says so |

There is no third kind, and in particular no "probably source" — that would be a confidence field
wearing a different hat.

A consumer's rule is therefore mechanical: **a quote that touches a `syntax` byte is not
invertible.** It may still be fine to show a human; it is not a citation.

### Law 4 — coverage is a census, not a summary

Emitted plus dropped must equal what the representation held, asserted in CI on real fixtures. Every
dropped character sits in a **named bucket with a count**. "Some content was omitted" is not a
disclosure; `annotation-text-not-projected-v1: 37` is.

## 5. What v1.1 is not

- **Not a second record.** The input is the representation, and the output mints **no new node ids** —
  a `source` segment names ids that already exist. A projection that mints identifiers is a parallel
  record, and two records of one document drift.
- **Not a layout engine.** No heading inferred from a font size, ever. A heading is a heading because
  the document's structure tree said so, or it is a paragraph.
- **Not a verifier.** The engine still does not verify. It *invokes* the pinned verifier on a quote
  that came through the map and relays what it says, re-deriving nothing.
- **Not a quality contest.** No bake-off, no "better Markdown than X", no ranking.
- **Not a place to put cosmetics — except in the export, counted.** S3 ships exactly one: a word the
  page broke across a line comes out closed up. It does **not** touch the representation, which still
  holds both halves with the hyphen verbatim; the joined bytes are one `source` segment naming both
  runs; and the removed hyphen is a named bucket with a count. The cost is stated rather than hidden:
  **the joined word does not ground**, because no element contains it. Dot-leader removal and drop-cap
  merging are not implemented and have no fixture — not a plan, just absent.
- **Not permission to reopen v1.** No detector is retuned here. Every detection rule keeps its id and
  its numbers.

## 6. Slices

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | This document and the milestones | done |
| **S1** | Linear Markdown, the anchor map, coverage, and the verify golden | done |
| **S2** | Tables and lists, with the erasure declared, still with the map | done |
| **S3** | The hyphenation join, in the export only, counted | done |
| **S4** | HTML, under the same four laws | done |

**S3 shipped the cosmetic and not HTML, and the row was split rather than half-ticked.** They are two
products: HTML is a second artifact owing the same four laws, and a hyphenation join is a few lines in
one function. Doing both under one label would have made "S3 is done" unreadable.

v1.1 is complete.

## 7. Identity

A projection that changes what comes out is a profile change, on the same discipline as every rule id
before it:

- **`markdown_rule`** — a versioned string, moved by each slice that changed the output. Unchanged at
  S4, so artifacts either side of that hash carry byte-identical Markdown.
- **`html_rule`** — a **separate** versioned string. Separate because the two projections say
  different things about the same table, and one id covering both would make every artifact
  non-comparable each time either moved.
- **`capabilities.markdown` and `capabilities.html`** — `true` only with a proof test; `false` obliges
  a declared limitation.
- **A profile predating S1**, with no `markdown_rule`, is **refused rather than defaulted**. A field
  defaulted in is a claim the run never made.

## 8. Standing rules, carried forward

Repeated because a projection is where they get bent:

1. **No public confidence field, ever.** A segment kind is typed vocabulary, never a score.
2. **No box derived from a font size** — and no *role* derived from one either.
3. **No silent drop and no silent repair.** A dropped character is a named bucket with a count.
4. **No invented coordinate, identifier, fingerprint or page number.** A projection mints no ids.
5. **Fail closed, and distinguishably.**
6. **Byte identity is a test.**
7. **The Ethos tree is read-only.**
8. **No verification.** Markdown does not acquire a verdict.

## PR review checklist

- [ ] Markdown and map are fields of one artifact; no code path emits one alone
- [ ] The map tiles the whole string exactly — sorted, contiguous, no overlap, no gap
- [ ] Every `source` segment names node ids that exist
- [ ] Emitted plus dropped equals what was in the representation, asserted on a real fixture
- [ ] Every dropped bucket is named and counted
- [ ] No new node id is minted anywhere in the projection
- [ ] No heading comes from a font size
- [ ] The rule id is on the profile, and a profile without it fails closed
- [ ] A `true` capability has a named proof test
- [ ] Double-run byte identity holds
- [ ] Nothing is re-derived from a verifier report
- [ ] v1's numbers are untouched, and S7 still reads as missed
