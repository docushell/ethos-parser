# Codes a font's declared map leaves unmapped — scope (decision #43)

**Status: shipped 2026-10-05 as `declared-font-codes-v3`.** A run's codes become characters through
the font's `/ToUnicode` CMap, or its simple encoding where it has none. Where neither maps a code,
`-v2` omitted the run and counted it under `broken-font-encoding`. `-v3` reads such a code through
what the font itself states, and only such a code: nothing the document declared is overridden.

## 1. Why

On ParseBench, 217 of its 2,037 documents omitted text — 21,600 runs — and on the eight
tree-stripped gate documents, six omitted 8,261. Two causes, neither of them a damaged document:

1. **`Q` did not restore the text state** (`crates/ethos-parser-pdf/src/content.rs`). The text
   state is part of the graphics state (PDF 32000-1 §9.3.1). A producer that set a font inside
   `q … Q` for one space — Office's clip groups do — had every later string decoded through that
   font's `/ToUnicode`, which maps none of their codes. The European Medicines Agency's research
   tables lost "Perform an analysis of clinical trials…" this way. A bug, not a policy: fixed.
2. **Maps that are incomplete where the font is not.** A simple font names a glyph in
   `/Differences` — `uni092A094D0930`, `f_f` — that its `/ToUnicode` omits, or that this
   profile's sixty-name glyph table does not hold; a composite font's `/ToUnicode` covers part of
   its glyphs while its embedded TrueType program's cmap names every one.

## 2. The rule

[`crates/ethos-parser-pdf/src/font_fallback.rs`](../crates/ethos-parser-pdf/src/font_fallback.rs)
builds, once per font, the map of codes the font states:

1. **`/Differences` glyph names**, read by the Adobe Glyph List specification's rules that need no
   list: a suffix after the first full stop dropped, components joined by underscores read one by
   one, each `uniXXXX…` (four upper-case hex digits per character), `uXXXX` to `uXXXXXX`, or a name
   this profile's glyph table holds. **Never the base encoding's table**: a font that ships a
   `/ToUnicode` often names `/MacRomanEncoding` as a formality and draws its own glyphs at those
   codes.
2. **An embedded TrueType program's own tables**: a composite font's CID through its
   `/CIDToGIDMap` (under `/Identity-H` or `/Identity-V` only, where the code is the CID), a
   symbolic simple font's code through the program's symbol cmap; the glyph read back through the
   program's Unicode cmap — the lowest codepoint that selects it — or its `post` glyph name by
   rule 1.

A control or private-use value is never produced. Every run read this way is declared on the
artifact under `unmapped-codes-read-from-font`, with its count; a run with a code none of these
maps is still omitted and counted under `broken-font-encoding`.

## 3. Measured (2026-10-05)

Both changes together, with `ruled-rects-v9` (§4):

| | before | after |
|---|---:|---:|
| ParseBench tables | 0.4092 | 0.4148 |
| ParseBench charts | 0.0977 | 0.0984 |
| ParseBench content faithfulness | 0.6627 | 0.6633 |
| ParseBench semantic formatting | 0.4063 | 0.4080 |
| ParseBench visual grounding | 0.4687 | 0.4695 |
| opendataloader-bench NID | 0.8819 | 0.8825 |
| opendataloader-bench TEDS | 0.4283 | 0.4322 |
| opendataloader-bench MHS | 0.5458 | 0.5467 |

Omitted runs across ParseBench's 2,037 documents: 21,600 on 217 → 13,750 on 181 with the `Q` fix →
10,895 on 143. 5,511 runs, on 50 documents, are read through the font, 2,656 of them on one table page that was
refused whole before — its `/Differences` names every glyph `uniXXXX` — and now scores 0.1302 GriTS
where it scored 0. On the gate documents 8,261 → 0, all of it the `Q` fix. No opendataloader-bench document falls on any
measure.

**The gate documents' tables**, against 2d226f1 on the eight tree-stripped documents, all of it the
`Q` fix (the font reading and `-v9` change none): 500 the same; 7 the same shape with the characters
they were missing; 18 larger and 8 new — rows of `nist-sp-800-171r3`'s control appendix that a
dropped run used to break (p117: 27 rows → 37). 6 are gone. Four were built from text with
characters missing (`171r3` p103, p107 and two on p112) and the real table on each page holds their
rows now. Two are losses: `171r3` p110's PM rows, read before only as a 3 × 4 table of broken text
and now read in full with no table found; and `nist-sp-800-161r1` p239's questionnaire grid, whose
blank answer column held a letter only while the leaked text state drew the first column's glyphs
across its line — placed where the document places them, the column is empty, and `-v8`'s fourth
clause refuses a grid with a column holding no letter or digit.

**What still falls, and why.** Two ParseBench pages read in scripts drawn in visual order lose
content score although their characters are now right: Farsi (right to left) and Hindi (vowel signs
drawn before their consonant). The text is in drawing order, as every run is — the engine's
limitation for those scripts, not the font's.

**What stays omitted** (10,895 runs): fonts that state nothing — subset programs stripped of their
cmap and glyph names, CID-keyed CFF programs, names only the full Adobe Glyph List knows
(`scedilla`), codes MacRoman's sources dispute. Only OCR would read those.

## 4. `ruled-rects-v9`

Reading the EMA tables' text exposed a clause of `ruled-rects-v8` that was too wide: it refused a
grid where any two cells of a row held three or more lines on shared baselines — and two cells of
wrapped prose beside a one-line label share their baselines too. `-v9` asks it of every cell of the
row that holds text, which is what rows the page did not rule look like
([`31-TABLE-TRACKS-SCOPE.md`](31-TABLE-TRACKS-SCOPE.md) §13).

## 5. Not done

- **The full Adobe Glyph List**: still not vendored (`docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`
  §6); a derived subset would read the names above.
- **CFF and Type 1 programs**: `skrifa` reads TrueType and OpenType; a bare CFF's glyph names are
  not read.
- **Logical order for complex scripts.**
