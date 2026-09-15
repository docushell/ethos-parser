# Rotated text, as it was measured

The instruments behind the fix for [`../../22-WORD-BOXES-SCOPE.md`](../../22-WORD-BOXES-SCOPE.md)
§9 items 1 and 2 — a run's box built from the pen's travel as a vector through its text matrix, CTM
and `/Rotate`, and `GeometryAbsence::NotAxisAligned` for a baseline along neither axis. Committed so
the numbers below can be re-derived rather than believed.

**These are measurement scripts, not product code.** Nothing imports them and they are not on the
gate.

**Run 2026-09-15**: `ethos-parser` 0.57.0 as released (base) against the change, built at the same
version string so no identity field can hide a difference.

## Running them

```bash
M=docs/measurements/rotated-text
# BASE and NEW are two release builds of ethos-parser. For every PDF: extract, classify and overlay
# on the PDF; ground, markdown and html on each binary's own extract; byte-compare every artifact
# and exit code, and keep the documents that differ.
python3 $M/run.py BASE NEW /tmp/rot --jobs 6 \
  fixtures/gate/{irs-f1040sd-2025,irs-fw9,nist-sp-800-218,nist-sp-800-207,nist-sp-800-171r3,nist-sp-800-37r2,nist-sp-800-161r1}.pdf \
  fixtures/engine/*/document.pdf $(find ../ethos-oracle/fixtures -name '*.pdf') \
  ../ethos-oracle/benchmarks/gate-zero/corpus/*.pdf ~/ethos-external-benchmarks/opendataloader-bench/pdfs/*.pdf
# The 950 MiB one, alone and without overlay:
python3 $M/run.py BASE NEW /tmp/rot-53Ar5 --jobs 1 --no-overlay fixtures/gate/nist-sp-800-53Ar5.pdf
# Every kept document, node by node (streams the extract, so 53Ar5 fits):
for d in /tmp/rot/*/ /tmp/rot-53Ar5/*/; do python3 $M/rotcheck.py $d; done
```

`run.py` calls `reprdiff.py` and `grounddiff.py` beside it. `rotcheck.py` adds what this change
needs: each geometry transition split by whether the new box has the vertical signature (the origin
on a horizontal edge, between the x edges), node fields compared, the limitation's clauses, Markdown
and HTML compared after normalising `representation_sha256`, and grounding compared by node id and by
member spans, so element renumbering is not counted as change.

## What it found

**340 documents** — 7 gate, 44 engine, the 35 PDFs under the oracle's `fixtures`, the 10-document
gate-zero corpus, 200 opendataloader-bench, and 44 one-page probes — plus `nist-sp-800-53Ar5` alone.
**39 changed**: extract, ground, and (where the limitation text moved) markdown and html through
`representation_sha256` only. Overlay and classify changed on none. On every changed document, no
node's text, order, parent, attributes, findings or native locator moved — `advance` included — and
no table, page or other assurance field.

| Transition | Count | Where |
| --- | ---: | --- |
| `no_ink_to_measure` → measured, advance ≤ 0, vertical box | 31,699 | `irs-fw9` 4, `nist-sp-800-207` 2,800, `-218` 954, `-37r2` 10,498, `-161r1` 17,443 |
| same | 4,685 | `nist-sp-800-53Ar5` |
| same | 30,773 | gate-zero `nist-sp-800-53r5` 25,970, `-63b` 4,802, `irs-form-1040-2025` 1 |
| same | 74 | opendataloader-bench `01030000000076` 73, `01030000000185` 1 |
| measured upright → measured vertical | 52,644 | `nist-sp-800-53Ar5` margin note, all with the vertical signature |
| → `not_axis_aligned` | 0 | any corpus (1 on the new fixture, 2 probes) |
| measured → `measured_off_page`, no `off-page-text` finding | 2 | conformance `rotation-90` and its gate-zero copy |

**`rotation-90`.** The pen runs from user x 36 to 145.044, which `/Rotate 90` lays along display y, so
the turned box is [6827, 3600, 8492, 14504] against a page 14400 tall — 1.04 pt past it. Ethos's own
`layout.json` for the fixture has [6824, 3742, 8489, 14488], past the edge too. The run keeps its text
and origin and loses its horizontal grounding element; the limitation's off-page sentence now says
its origin is on the page.

**Grounding.** Spans added equal each document's newly measured runs, and on no corpus was a span
or an element removed except `rotation-90`'s one. Elements: `irs-fw9` 334 → 335 (spans 970 → 974),
`nist-sp-800-207` 3,532 → 6,332, `-218` 6,899 → 7,853, `-37r2` 26,518 → 37,016, `-161r1` 30,202 →
47,412 (spans 498,561 → 516,004, and 5 declared blocks mixing a vertical label with horizontal text
grow to cover it), `nist-sp-800-53Ar5` 49,525 → 54,172, with its 642 per-page note elements
re-boxed to x 26.90–35.90, where the independent reader below puts the note. Most new elements are
one vertical run, because the undeclared join needs a shared `origin_y`, which a vertical line never
has: on `nist-sp-800-207`, `-218` and `-37r2` every one is, and the 2,800 on `-207` are pieces of its
vertical `/Artifact` note. A declared marked-content group joins its vertical runs all the same:
`irs-fw9`'s one new element has 4 spans, `nist-sp-800-161r1` has 15 with 3–16, and gate-zero
`nist-sp-800-63b` has 9 with 7–27.
Every changed artifact checked — all of the above, gate-zero's three and both opendataloader
documents — is `valid` and `matched` under both `ethos grounding check` (v0.6.0) and
`ethos-parser grounding-check`, with identical reports.

**Probes.** All 39 predicted boxes and absences on the p- and q-probes matched the reference
arithmetic exactly; the upright probes, the fake italic and the cancelled flip did not change.

**Independent reader, run out of tree.** The reader was PyMuPDF. It is AGPL, and decision #14 keeps
AGPL out of this repository, instruments included, so its three scripts are not committed. Its
text trace classifies every character by baseline direction.
- Over the seven gate documents, `nist-sp-800-53Ar5`, gate-zero and opendataloader-bench it finds no
  character on a −x or off-axis baseline, and the only `/Rotate` text is `rotation-90`'s 13
  characters.
- Its vertical non-whitespace characters equal, page by page, those in 0.57.0's runs of advance ≤ 0
  on `irs-fw9`, `nist-sp-800-207`, `-218`, `-37r2`, `-161r1` and opendataloader `01030000000076` and
  `01030000000185`, so none of that text was CTM-turned.
- After the change the same count sits, page by page, in runs with a vertical measured box: 43,
  4,592, 1,476, 15,023, 27,558, 133 and 34.
- On `nist-sp-800-53Ar5` it is 60,043 of 60,066, equal on every page but page 47. There, 23
  characters of a turned table header (`Assessor /`, `Assessment Team`) are absent from the extract
  itself, on both builds, since no node was added or removed.

**Cost.** `extract` on `nist-sp-800-161r1`, three runs each: 3.59–3.65 s and 1.11 GiB peak on
both binaries. On `nist-sp-800-53Ar5`, two each: 12.24–12.35 s and 3.63–3.66 GiB on both.

**Also moved, outside any artifact.** `ground`'s stderr note, `N of M node(s) omitted from the
grounding artifact`, changes its count by exactly each document's geometry transitions (for example
`irs-fw9` 127 → 123 and `nist-sp-800-161r1` 71,003 → 53,560). It appears on both `rotation-90` copies.
No exit code and no other command's stderr moved.

**Not changed, deliberately.** `PdfLocator::advance` still measures along the text matrix's x before
rotation; on a CTM- or `/Rotate`-turned page it disagrees with the box. Recorded as a known defect,
not decided here.
