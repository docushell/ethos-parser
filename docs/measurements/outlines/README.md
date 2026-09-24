# `/Outlines` — what the corpus actually declares

**Measured 2026-09-23.** The evidence for [`../../29-OUTLINES-SCOPE.md`](../../29-OUTLINES-SCOPE.md)
§2.1, which is the argument for reopening [`../../17-D1-SCOPE.md`](../../17-D1-SCOPE.md) §5's bar:
*a detector with no positive case is an assertion*.

**Nothing was built when this was measured**, which is the point: the instrument opened the trees
with `lopdf` so the scope's numbers came from somewhere other than the code they would judge.

**The reader shipped on 2026-09-24** as `outlines-v1`, and it reproduces every figure below —
2 273 entries, the per-document counts and depths, 0 unresolved, 69 undecodable titles. That is
`29-OUTLINES-SCOPE.md` §8 bar 1 met against constants set before the code existed rather than
re-derived from it, which is the only version of that check worth running.

## What ran

[`outlinescan.rs`](outlinescan.rs), pinning `lopdf = { version = "0.44.0", default-features =
false }` — the engine's own declaration, so it walks a catalog exactly as the reader would. **Not a
workspace member and not a cargo target**: it is the source of a throwaway binary, kept here because
a measurement whose method cannot be re-run is not reproducible
([`../../17-D1-SCOPE.md`](../../17-D1-SCOPE.md) §8). Drop it into a scratch crate with that one
dependency to re-run it.

```bash
outlinescan $(find fixtures -name '*.pdf' | sort) > outlines.json
```

Corpus: all **70** PDFs under `fixtures/`. Data: [`outlines.json`](outlines.json), one record per
document.

## The numbers

| | |
| --- | ---: |
| PDFs carrying an `/Outlines` with at least one entry | **6 of 70** |
| total entries | **2 273** |
| maximum declared depth | **5** |
| destinations resolving to a page in this document | **2 273** |
| destinations **not** resolving | **0** |
| entries whose target page precedes the previous entry's | **2** |
| empty titles | **0** |
| **walks abandoned early** (repeated id, unresolvable `/First`/`/Next`) | **0** |

| document | entries | depth | resolved | backward | pages |
| --- | ---: | ---: | ---: | ---: | ---: |
| `nist-sp-800-53Ar5` | 1 251 | 5 | 1 251 | 0 | 733 |
| `nist-sp-800-161r1` | 433 | 5 | 433 | 0 | 327 |
| `nist-sp-800-37r2` | 347 | 3 | 347 | 1 | 183 |
| `nist-sp-800-171r3` | 160 | 3 | 160 | 0 | 120 |
| `nist-sp-800-207` | 69 | 3 | 69 | 0 | 59 |
| `nist-sp-800-218` | 13 | 2 | 13 | 1 | 36 |

**2 273 is exact, not a lower bound.** The walk counts its own early exits and they are zero, and
the total was confirmed independently by counting objects carrying both `/Title` and `/Parent` —
2 273 that way too, from qpdf's object dump rather than lopdf's tree walk.

All six are NIST special publications, which is the corpus's shape rather than a finding about
outlines: `17-D1-SCOPE.md`:54 already recorded *"8, in six documents"*. The **2 273** and the
resolution rate are what was missing.

**Destination resolution** follows §12.3.2: an explicit array, a name or byte string through
`/Names`→`/Dests` or the catalog's older `/Dests`, and `/A` with an `/S /GoTo` action. A first array
element that is an integer is a **remote** destination's page number, in another file, and is not
resolved — none occurs here.

## The title-decoding finding

The instrument decodes a `/Title` per §7.9.2.2 — UTF-16BE behind a byte-order mark, PDFDocEncoding
otherwise — and **refuses the `0x80`–`0x9F` block rather than guessing it**, because that block is
where PDFDocEncoding, Latin-1 and Windows-1252 disagree.

**69 titles carry 70 such occurrences** — one title in `nist-sp-800-171r3` holds two `0x85`.

| byte | occurrences | Annex D.2 | this repository's only 0x80–0x9F table (`WIN_ANSI`, CP1252) |
| --- | ---: | --- | --- |
| `0x85` | 59 | **U+2013** en dash | U+2026 ellipsis — **wrong** |
| `0x84` | 9 | **U+2014** em dash | U+201E — **wrong** |
| `0x90` | 2 | **U+2019** right quote | **undefined** |

The Annex D.2 column was read with qpdf 12.3.2's decoder and agrees with the strings' own context:
`03.08.09 System Backup – Cryptographic Protection`, `PREPARE TASKS AND OUTCOMES—SYSTEM LEVEL`,
`APPENDIX F: RESPONSE TO EXECUTIVE ORDER 14028’s CALL`.

**The wrong table is one file away and would pass a printability check.** An earlier draft of this
page glossed `0x85` as an ellipsis — the CP1252 reading, taken from the table already in the tree.
It turns `Backup – Cryptographic` into `Backup … Cryptographic`, and CP1252 has no `0x90` at all.

**Why it matters to a reader and not only to this instrument.**
[`forms.rs`](../../../crates/ethos-parser-pdf/src/forms.rs):490's `decode_text` is the text-string
decoder this repository already has, and its non-UTF-16 branch is
`bytes.iter().map(char::from)` — Latin-1, so `0x85` becomes **U+0085**, a C1 control character.
Those 69 titles would carry invisible control characters inside a string that still reads as
well-formed. Not a refusal, not a `U+FFFD`, just wrong.

**This is why `29-OUTLINES-SCOPE.md` §4 leaves those 69 titles ABSENT AND COUNTED rather than decoded.**
Vendoring Annex D.2's block would fix them, but
[`../../21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](../../21-STANDARD-14-ASCII-COVERAGE-SCOPE.md) §4
refused a hand-transcribed Annex D table once, and §5's reopening condition — cross-validation
against the vendored `WinAnsiEncoding` column — **cannot be met for this block**, because 0x80–0x9F
is exactly where the two tables disagree. So decoding is an optional later slice, not a precondition.

Noted in passing: `decode_text`'s doc comment says *"a byte outside it becomes `U+FFFD`"*, which its
non-UTF-16 branch cannot produce. True of the UTF-16 branch, false of the other.

## What this does not measure

**Whether a title appears in the text of the page its destination resolves to** — the PI-B
cross-check. That join needs the engine's own per-page text and is deliberately deferred: the
decoding gap above would corrupt it, since an undecoded `0x85` cannot match a page that draws an
**en dash**, so the count would measure the decoder rather than the document. It is `29-OUTLINES-SCOPE.md` slice S3, after `S-ENC`, and neither has shipped.

**A rate for exactly this join was published in `06-STEAL-REFUSE.md` row PI-A on 2026-09-22 and
withdrawn on 2026-09-23** (`f185611`): it came from the PageIndex review's reader agent and nothing
in this directory produces it. It is not re-derived here for the reason above.

**Also not measured, and owed before `S-ENC` is argued:** how many annotation and form-field strings
across the 70 fixtures carry a byte in 0x80–0x9F. `decode_text` is live on those today, so the
block's blast radius is wider than these 69 titles and is currently unstated.
