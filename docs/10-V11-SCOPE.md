# 10 — v1.1 scope: Safe Markdown

**Status:** scope authority for v1.1 · **Slice detail:** `11-V11-MILESTONES.md`
**This is the code-review map for v1.1.** Every v1.1 PR belongs to exactly one slice.

**v1 is not done.** Its gate — table-cell accuracy above 0.489 — is measured and **missed at 64‰**
(`table-gate-v1.md`, `09-V1-MILESTONES.md` S7). v1.1 is the next row of `02-ROADMAP.md` and it
started because the owner asked for it, **not** because the gate cleared. Nothing in this document
closes v1, and no slice here may be cited as evidence that it did.

---

## 1. The one sentence

**A Markdown quote is either invertible to canonical evidence, or explicitly unquotable.**

Everything below is machinery for that sentence.

## 2. Why this version exists at all — Workbench rule 8

`01-CONTRACT.md` §12 says this contract *does not define a Markdown projection*, and gives the
reason: **retrieval operates on the evidence record itself; any projection between what is ranked
and what is cited is where a locator dies silently.**

That is the failure mode, stated concretely. A pipeline chunks Markdown, embeds the chunks, ranks
them, and hands the winning chunk to a model. The model quotes it. The citation then has to bind
back to *the document* — a run, a cell, a page — and the Markdown has thrown that away. Nobody
notices, because the quote is real text and the answer looks right. The parity checklist calls this
**O8: Markdown only with the Anchor Map**, and it records the fallback plainly — **rule 8 prefers
no projection at all.**

Competitors ship Markdown for RAG and lose the cell. This repository declined to ship Markdown for
seven v1 slices for exactly that reason. v1.1 does not lift the objection; it **pays** it, by making
the map a structural precondition rather than a companion file somebody can forget.

**The gate is not "pretty GFM."** Prettiness is not a property this version optimizes. Two things
are:

1. a Markdown-quoted citation verifies end-to-end, and
2. coverage completeness is asserted, not described.

## 3. What v1.1 is

| | |
| --- | --- |
| **One artifact** | `ethos.markdown.v1` — canonical JSON, c14n, integer fields only |
| **Carrying both halves** | the `markdown` string **and** the `anchor_map` that inverts it |
| **Plus a census** | `coverage`, which accounts for every source character the representation offered |
| **Produced by** | a projection of `DocumentRepresentation v0`, under a versioned `markdown_rule` |
| **Proved by** | a quote lifted out of the Markdown that grounds through the *existing* Ethos verifier |

## 4. The four laws

These are the ones a reviewer checks first, because each is a way the version fails quietly.

### Law 1 — never one without the other

Markdown is never emitted without its map, and a map is never emitted without its Markdown. They
are **fields of one artifact**, not two files with a naming convention. A CLI that writes a `.md`
must write the map in the same invocation or refuse with exit 2. There is no `--md-only`, and
adding one is a contract change, not a convenience flag.

*Why structural rather than procedural:* a companion file is a thing a caller forgets, a pipeline
strips, or a cache drops. A field is not.

### Law 2 — the map total-tiles the bytes

Every UTF-8 byte of the `markdown` string belongs to **exactly one** segment. Segments are
`[start, end)` byte offsets, sorted, contiguous, no overlap, no gap, covering `0..markdown.len()`.

**Incomplete coverage is a failed test, not a footnote.** A map with a hole is worse than no map,
because the hole is exactly where an unquotable byte hides.

### Law 3 — two segment kinds, and only two

| kind | means | invertible? |
| --- | --- | --- |
| `source` | bytes that came from canonical node text | **yes** — names the representation node id(s) |
| `syntax` | markup the exporter invented (`#`, blank lines, fences) | **no**, and it says so |

There is no third kind, and in particular there is no "probably source" — that would be a
confidence field wearing a different hat (`01-CONTRACT.md` §9).

A consumer's rule is therefore mechanical: **a quote that touches a `syntax` byte is not
invertible.** It may still be a fine thing to show a human; it is not a citation.

### Law 4 — coverage is a census, not a summary

`coverage.source_chars_emitted + coverage.source_chars_dropped == coverage.source_chars_in_representation`,
asserted in CI on real fixtures. Every dropped character sits in a **named bucket** with a count.
"Some content was omitted" is not a disclosure; `annotation-text-not-projected-v1: 37` is.

This is checklist **A14 — declared erasure**: if something is removed, the artifact says so *and
says how much*.

## 5. What v1.1 is not

- **Not a second IR.** The input is `DocumentRepresentation`. The output grows **no new node ids** —
  a `source` segment names ids that already exist. A projection that mints identifiers is a parallel
  record, and two records of one document drift.
- **Not a layout engine.** No font-size heading inference, ever (checklist **L29: REFUSE**). A
  heading is a heading because the document's structure tree said so, or it is a paragraph.
- **Not a verifier.** `07-VERIFY-BOUNDARY.md` is unchanged: the engine still does not verify. S1
  *invokes* the pinned Ethos CLI on a quote that came through the map, and relays what it says.
  Nothing is re-derived from the report.
- **Not a quality contest.** No bake-off, no "better Markdown than X", no ranking. `03-V0-SCOPE.md`
  §6's posture is unchanged.
- **Not a place to put cosmetics.** Hyphenation joining, dot-leader removal and drop-cap merging
  never touch the representation. If they ever appear they are export-only, and each is either
  `syntax` or an invertible emit — never a silent rewrite of node text. They are S3 at the earliest.
- **Not permission to reopen v1.** No detector is retuned in this version. `ruled-rects-v2`,
  `stroke-ruled-v1` and `unruled-align-v1` keep their ids and their numbers.

## 6. Slices

| Slice | Theme | State |
| --- | --- | --- |
| **v1.1-S0** | This document and `11-V11-MILESTONES.md` | **done** |
| **v1.1-S1** | Linear Markdown + Anchor Map + coverage + the verify golden | **done** |
| **v1.1-S2** | Tables and lists as Markdown, with the erasure declared (A14), still with the map | **not started** |
| **v1.1-S3** | HTML and/or export-only cosmetics, under the same map law | **not started** |

S2 and S3 start when the owner asks. Not before.

## 7. Identity

A projection that changes what comes out is a profile change, on the same discipline as every rule
id before it:

- `profile.markdown_rule` — a versioned string, `markdown-linear-v1` at S1, `markdown-blocks-v1` at S2.
- `capabilities.markdown` — `true` only with a proof test; `false` obliges a declared limitation.
- A profile JSON predating S1 — one with no `markdown_rule` — is **refused**, not defaulted. The
  same posture `table_detection.stroke_ruled` took at v1-S8: a field defaulted in is a claim the run
  never made.
- Workspace `0.10.0` → **`0.11.0`**, and the profile hash moves with it.

## 8. Standing rules, carried forward

Unchanged from `08-V1-SCOPE.md` §6, and repeated because a projection is where they get bent:

1. **No public confidence field, ever.** A segment kind is typed vocabulary, never a score.
2. **No box derived from a font size** — and no *role* derived from one either (L29).
3. **No silent drop, and no silent repair.** A dropped character is a named bucket with a count.
4. **No invented coordinate, identifier, fingerprint or pagination.** A projection mints no ids.
5. **Fail closed, and distinguishably.**
6. **Byte identity is a test.** Two runs of `engine markdown` are byte-identical.
7. **The Ethos tree is read-only.**
8. **No verification.** Markdown does not acquire a verdict.

## PR review checklist

- [ ] Markdown and map are fields of one artifact; no code path emits one alone
- [ ] The map tiles `0..markdown.len()` exactly — sorted, contiguous, no overlap, no gap
- [ ] Every `source` segment names representation node ids that exist
- [ ] `emitted + dropped == in_representation`, asserted on a real fixture
- [ ] Every dropped bucket is named and counted
- [ ] No new node id is minted anywhere in the projection
- [ ] No heading comes from a font size
- [ ] `markdown_rule` on the profile; a profile without it fails closed
- [ ] `capabilities.markdown: true` has a named proof test
- [ ] Double-run byte identity
- [ ] Nothing is re-derived from an Ethos report
- [ ] v1's numbers are untouched, and S7 still reads as missed
