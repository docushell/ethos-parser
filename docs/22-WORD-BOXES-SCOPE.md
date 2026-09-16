# 22 — Word boxes: refused as asked for, and what measuring them found

**Word boxes are refused for now, on measurement.** The verifier this engine is oracled against
would carry them and answer every claim as it does today, except a claim that names a word span's
`span_id` or a page-only `value` equal to a word's text — and those change verdict, from not found
or mismatch to grounded, rather than tightening a box that was already there (§4). A five-word quote
cited by element loses its precision between the element and its runs, not between a run and a word,
on all seven documents measured (§5).

**The measurement found two things worth more than what it was asked about.** The `char_offsets`
capability, promised since v0 and deferred at every step since, can be turned on with data the
projection already holds, and doing so removes the `missing_char_offsets` limit from the report on
every artifact that carries spans (§8). And the reader has defects a sub-run box would have inherited
— six are recorded in §9, the widest of them typing 56,091 non-whitespace characters of visible
rotated text on the eight gate documents as drawing nothing.

Nothing here was built. §7 names what reopens word boxes.

**Since this was written.** The body stands as measured at 0.57.0 on 2026-09-15; each change it led
to is recorded below with the version it ships in.

- **§9 items 1 and 2, rotated text — fixed for 0.58.0, in one change** rather than the two §9 and
  §11 ask for, because both are one data path and a split would ship a build that ignores the CTM or
  `/Rotate` on purpose. A run's box follows the pen's travel as a vector through its text matrix, CTM
  and `/Rotate`, on the side its glyph tops point; a baseline along neither axis gets a new
  `GeometryAbsence::NotAxisAligned`; upright boxes are bit for bit unchanged. Against 0.57.0: the
  31,699 visible `no_ink_to_measure` runs on the seven documents and 4,685 on `nist-sp-800-53Ar5` are
  measured, every one a vertical box; the 52,644 `nist-sp-800-53Ar5` note runs turn vertical; no run
  of any corpus is `not_axis_aligned`; `nist-sp-800-207` grounds 6,332 elements where it grounded
  3,532, the new ones pieces of its vertical `/Artifact` note. Corrections to §9: negative `Tf` or
  `Tz`, and a text-matrix turn the CTM cancels, also typed text `no_ink_to_measure`; a mirroring CTM
  laid the box the wrong way along its baseline, and upside-down text put it on the wrong side; the
  only `/Rotate` text measured, conformance `rotation-90`, turns 1.04 pt past its page, so it becomes
  `measured_off_page` and loses its horizontal grounding element instead of gaining a box.
  `PdfLocator::advance` still measures before rotation, a known defect left open.
- **§9 item 3, word spacing on two-byte codes — fixed for 0.58.0.** `Tw` now reaches only a simple
  font's code 32: every composite font this engine advances is Identity-H or Identity-V, whose codes
  are all two bytes. p26 advances 1500, not 2500. Against the build before it, over 327 PDFs (the
  eight gate documents, 44 engine and 35 oracle fixtures, gate-zero, 200 opendataloader-bench, the
  24 `probes.py` probes and 6 `Tw` probes generated out of tree), one real document moved:
  `cfpb-home-loan-toolkit` page 17 shows `=` as `<0020>` under `-0.017 Tw` at 11 pt, so that run
  advances 0.187 pt further and the nine runs after it on the line move right onto the origins an
  independent reader draws, to the centipoint; its Markdown and HTML differ only in
  `representation_sha256`, and its tables and grounding counts are unchanged. That reader is
  PyMuPDF, run out of tree because it is AGPL (decision #14), as were the census and the 65 pt
  below. Corrections to §9: the eight gate and 200 bench documents show a two-byte 0x0020 under
  non-zero `Tw` in 20 strings on 2 documents, each unmapped by its `/ToUnicode` and so never
  advanced — the live case sat outside that count. The defect was wider than `<0020>`: a byte 0x20
  the `/ToUnicode`-derived split read alone took `Tw` too, a lone zero-width `<0020>` got a
  `Measured` box made only of `Tw`, negative `Tw` typed a drawn glyph `no_ink_to_measure`, and the
  shift outlived a run later dropped as undecodable. Newly found, not fixed: a dropped run stops the
  pen at its refused code — `content.rs` returns before advancing that code and the codes after it —
  so later runs on the line sit left of where the page draws them (a `Tw` probe's next run at 55 pt,
  drawn at 65 pt).
- **§9 item 4, a `TJ`-gap space on the wrong run — fixed for 0.58.0.** A `TJ` number writes its
  flagged space onto the last run only if a string with codes was drawn since the pen was last
  placed (`BT`, `Td`, `TD`, `T*`, `Tm`, the line move of `'` and `"`, or a `cm` or `Q` that changes
  the CTM). A string later dropped as undecodable still counts, so no two words fuse across lost
  text. Against the build before it, over 322 PDFs (the eight gate documents, 44 engine and 35
  oracle fixtures, gate-zero, 200 opendataloader-bench, 25 probes), 60 runs in 6 documents lose
  their space (`nist-sp-800-161r1` 44, `nist-sp-800-171r3` 3, `cfpb-home-loan-toolkit` 8, and bench
  `01030000000001`, `…02` and `…04` 2, 1 and 2), plus probe p25; no box or id moves, nor the count
  of extracted nodes or of grounding elements and spans. 'i ncluded', 'Y OUR' and 'Gar cía' now
  read whole. One Markdown and HTML block break is new: `171r3`'s 'ad d' becomes 'ad' ‖ 'd'. Two
  cfpb tagged cells change, and their labels were regenerated to match; every table score is
  unchanged. Corrections to §9: the shape in real documents is not another line's run but a
  same-line `Tm` placing the pen again before a large negative number (p25's shape occurs in no
  measured document). 47 of the seven's 189 synthesized spaces were that shape, every one inside a
  word or beside a drawn space, so 47 of item 5's 189 runs no longer carry one.
- **§9 item 6, a Type 3 font's height — refused, not mapped, for 0.58.0.** A Type 3 font keeps its
  ascent/descent envelope only where its `/FontMatrix` leaves the vertical at the 1000-unit default
  (b = 0, d = 0.001 at single precision, f = 0; an absent or non-array matrix keeps it), whichever
  source supplied it — descriptor, embedded program or decision #22's standard-14 fill; elsewhere
  its runs that draw ink are `not_reported_by_reader` (whitespace stays `no_ink_to_measure`) and
  their advances are unchanged. Correction to §9: the direction it implies, carrying ascent and
  descent through the matrix, was wrong. Nothing in a Type 3 font says which units its descriptor
  uses: LibreOffice 7.5 and 7.6 write thousandths of text space under a 1/UPEM matrix (read in its
  source, measured only on a probe built from it), where mapping would shrink an exact 12 pt box to
  5.86 pt; Ghostscript writes a glyph-space `/FontBBox`, and pdf.js reads one each way. So p20's
  0.9 pt box is withdrawn, and so would a right LibreOffice-era one be. Against the build before it,
  over 351 PDFs (the seven smaller gate documents, 44 engine and 35 oracle fixtures, gate-zero, 200
  opendataloader-bench, the 24 probes and 31 Type 3 probes and producer files, three of those qpdf
  QDF rewrites of the Ghostscript ones), 17 documents changed, every one a synthetic probe: 16 runs
  measured and one `no_ink_to_measure` (a quarter-turned matrix) become `not_reported_by_reader`.
  Every real file is byte-identical, `ci/artifact-bytes.py` too over all 272 fixture artifacts; the
  real Type 3 fonts either sit at the default (matplotlib, the oracle ligature fixture) or carried
  no envelope (Skia in bench `01030000000163` and a Chrome emoji print, Ghostscript). The
  not-groundable limitation's "no `/FontBBox`" is false for these runs and is kept, since rewording
  it moves nearly every PDF artifact.
- **§9 item 5, codes that decode to several characters — a documentation defect, and the contract
  sentence below it, corrected for 0.58.0 with no engine change.** Correction to §9: the boxes are
  not the defect. Each code advances the pen once, so a run's box spans the ligature glyph's
  advance and not its letters', and every such run is its own advance wide — all 29 in
  `nist-sp-800-53Ar5` (26 exactly, 3 within the centipoint that quantizing the two edges separately
  allows) and all 608 in 79 of the 200 `opendataloader-bench` documents (473 exactly, 135 within
  it), measured on this branch's build with `ligatures.py`, which now counts each measured box
  against its own run's advance and sums over a directory. What was wrong is the writing. §5.3
  claimed *"measured ink boxes only, from the embedded font program or the font descriptor"* while
  the box is the ascent-to-descent envelope §5.3 itself names as the failure to avoid, stretched
  over the pen; it now describes the box as this branch builds it — the pen's travel along the
  run's baseline, each code advancing once by its own width from `/Widths`, `/W` and `/DW` or a
  vendored standard-14 AFM, over that envelope on the side the glyph tops point — and records,
  without deciding, that nothing on the wire declares the box's kind and that §6's versioned rule
  for it is not in the profile. The 0.55.0 entry's *"flags exactly that"* is corrected the same
  way: `scalar_code_mismatch` compares two counts, so a synthesized character sets it too — 15,164
  of the 15,772 flagged bench runs are synthesized-only. Pinned by the conformance ligature run's
  advance (9600) and a unit test that shows one advance per code for a `/Differences` ligature. The
  box's own width is measured, not pinned: no fixture in the tree carries a *measured* box on a
  multi-character code — the conformance ligature is Type 3 and takes the envelope gate above — so
  a reader that spanned the box over the run's letters would pass every test and is caught only by
  `ligatures.py` (mutated, probe `p18-ligature`'s box reads 3000 centipoints against a 2500
  advance). Closing it needs a new fixture. No emitted byte moves: 323 PDFs byte-identical across
  extract, classify, overlay, ground, markdown, html and exit codes, and all 272 fixture artifacts
  identical.
- **§8, `char_offsets` — built and turned on, and ships in 0.58.0 with the owner's acceptance.** §8
  makes the flip the owner's call, and the owner accepted it on 2026-09-16. Every span a PDF
  grounding artifact carries now says where its text lies in its element's, `char_start` inclusive
  and `char_end` exclusive, in Unicode scalars. The offsets come from a scalar cursor over
  **every** member of the block, boxless runs included, and the artifact claims the capability as
  `char_offsets && spans_emitted`. Against the build before it, over 328 PDFs (the seven smaller
  gate documents, 44 engine and 35 oracle fixtures, gate-zero, 200 opendataloader-bench, 24
  `probes.py` probes and 8 built out of tree for this question), 324 grounding artifacts gained an
  offset pair on each of their 2,447,419 spans and nothing else: stripping `char_start`/`char_end`
  and resetting the flag gives the base bytes on all 324, and no node, box, id, element text or
  span text moves anywhere — 0 changed nodes over the whole corpus. Recomputed
  independently out of tree from the **base** representation, 0 of the 2,447,419 disagree. 280,617
  spans carry an offset pair a UTF-8 byte cursor would have written differently and 6,974 hold
  non-ASCII text; 13,089 carry a space the reader synthesized. 36,745 of the 220,826 elements hold
  boxless text — 32,365 at an edge, 22,068 in the interior, 17,688 both — the members a boxed-only
  cursor would skip. Both checkers call every one of the 324 valid and source-matched.
  `nist-sp-800-53Ar5`, past the million-span cap, carries neither spans nor offsets and both
  checkers accept it; all 16 Office grounding artifacts are byte-identical. On `irs-fw9` Ethos
  returns all five checks with identical status and evidence and drops `missing_char_offsets` with
  the `capability_limited` warning it was the only cause of. The cost: artifacts grow 29.9%
  (`nist-sp-800-161r1` 53.37 MB → 69.23 MB, the largest 130.46 MB, none within 10% of the 256 MiB
  ceiling) and `grounding-check` on `161r1` goes 0.29 s → 0.53 s (Ethos 0.72 s → 1.07 s).
  Corrections to §8: the offsets are **not** found by searching the element string, as `variants.py`
  did — the projection knows each member's position and a search is ambiguous wherever a run's text
  repeats. That is an argument, not a measurement — a search from the previous span's end agrees on
  all 2,447,419 spans, and neither checker can tell the two apart — so a unit test pins the member
  cursor. The proof cannot live in `crates/ethos-parser-pdf/tests/capabilities.rs`, which has no
  grounding dependency, so that table's row names
  `char_offsets_index_the_element_text_in_unicode_scalars` in `ethos-parser-cli`'s oracle suite,
  where the real `extract` and `ground` binaries write the artifact and the pinned Ethos decides;
  and §8 understates the reach — correcting the stale limitation detail changes one field of every
  **Office** representation, and every format's Markdown and HTML move their profile and
  representation digests.

---

## 1. The question

A text run carries one box. A quote that cites part of a run can be bound to no box tighter than the
whole run, and a quote cited by element is bound to the element's. The PDF reader already keeps the
per-code pen advances a sub-run box would be built from — `ShownText::code_advances`, added at
0.55.0 and described there as *"what a sub-run extent — a word box — has to be built from"* — and
the grounding schema already has `spans` with `char_start`, `char_end` and a `bbox` of their own.

**Would boxes for words, emitted as spans, bind citations more tightly — and can this engine emit
them honestly?** That splits into four questions: can a sub-run extent be computed with the run
box's own meaning (§3); would the verifier use one (§4); how often does a word or a quote actually
sit inside a run (§5); and what does the tree already promise or refuse (§6, §10).

## 2. What was measured

`ethos-parser` 0.57.0 and the pinned Ethos v0.6.0 (`8adda91`), over the eight `fixtures/gate`
documents. The corpus figures in §5 cover seven of them — `irs-f1040sd-2025`, `irs-fw9`,
`nist-sp-800-218`, `nist-sp-800-207`, `nist-sp-800-171r3`, `nist-sp-800-37r2` and
`nist-sp-800-161r1`: 733 pages and 1,329,323 text runs. The eighth, `nist-sp-800-53Ar5`, is another
733 pages whose representation is 950 MiB; it enters §8 and §9, where each figure names it. Synthetic
one-page probes each exercise one text-state feature, and claims were verified against variants of
`irs-fw9`'s grounding artifact. Every instrument is in
[`measurements/word-boxes/`](measurements/word-boxes/README.md).

It was run as five independent investigations — the reader, the wire, the verifier, the corpus, the
tree's record — then checked by a sixth that re-derived the numbers the decision rests on, and this
document was reviewed against the sources and the instruments before it was committed. Corrections
from both are folded in.

## 3. What a run is, and what its box means

**A run is one string operand.** Each `Tj`, `'` or `"` string, and each string element inside a `TJ`
array, becomes its own run; a `TJ` number only moves the text matrix between runs, and nothing merges
adjacent runs afterwards (`content.rs:500-520`, `815-834`; `extract.rs:339-475`). Probe p12,
`[(W) 80 (ord) -300 (next) -100 (word)] TJ`, extracts as four runs: `W`, `ord `, `next`, `word`.

**The box is not glyph ink.** Horizontally it is the pen: the run's origin to origin plus the summed
advance, from `/Widths`, a CID `/W` and `/DW`, or a vendored standard-14 AFM, with `Tc`, `Tw`, `Tz`,
the text matrix and the CTM's x-scale applied. Vertically it is the font's ascent and descent — from
the embedded program, the descriptor, its `/FontBBox`, or the AFM — scaled by the rendered em
(`fonts.rs:337-390`; `extract.rs:385-388`: *"from the font's ascent/descent envelope stretched over
the run's advance — not from glyph outlines"*). §9 records that `docs/01-CONTRACT.md` §5.3 describes
it otherwise.

**For upright horizontal text with known widths, a sub-run extent is exact arithmetic on data the
reader holds**: prefix sums of the same per-code deltas that sum to the run's advance
(`content.rs:727-747`, checked by `debug_assert`s at `extract.rs:366-382`), quantized as the run box
is, with the run's own top and bottom. It is `Computed` under `docs/01-CONTRACT.md` §6, as the run
box is. The one place it can differ from the run box is the run's right edge, where a sum of
quantized pieces and a quantized sum may round apart.

**It cannot be computed** inside a code that decodes to several characters, for a space the reader
synthesized from a `TJ` gap (it has no code and no advance), or in a run holding any code with no
width — such a run has no box at all (probe p27). **Nor is it honest wherever the run box is not**,
and §9 shows the run box is wrong for rotated text.

## 4. What the verifier does with a finer span

**Ethos v0.6.0 reports a location in one place: `evidence.bbox`, with its page and text**
(`verify_types.rs:224-240`). No span offset appears in a report; the only character offset one can
carry is the hardened profile's context-echo boundary, an index into the echoed text
(`verify_types.rs:272-281`). `char_start` and `char_end` are checked by the grounding validator and
never read by verification, and `capabilities.char_offsets` does one thing: add or omit the
`missing_char_offsets` capability limit (`ethos-verify` `lib.rs:140-142`).

**A claim resolves to the element before any span.** Cited by page and box, only elements are
searched (`lib.rs:930-951`). Cited by page alone, a page's elements are scanned before its spans
(`lib.rs:975-1011`), and a span's text always lies inside its element's. The adjacency join that lets
a quote cross a boundary requires `element_id` (`lib.rs:1135`) and unions element boxes
(`lib.rs:1281`). A span's own box comes back in two cases only: the claim names that span's
`span_id`, or a page-only `value` claim equals a span's text exactly. A `table_cell` claim always
resolves through its table (`lib.rs:887-899`).

Seven of the nine claims `runv.py` verifies, against five variants of `irs-fw9`'s artifact — **A** as
emitted, 970 spans, one per measured run; **B** A with offsets on those spans; **C** B plus 6,198 word
spans (7,168 in all); **D** C without offsets; **E** C with one word's box moved to another line:

| claim | A | B | C | D | E |
| --- | --- | --- | --- | --- | --- |
| `required.` by `element_id` `e8` | element box | element box | element box | element box | element box |
| `required.` by page and the word's own box | element box | element box | element box | element box | element box |
| `required.` by page only | element box | element box | element box | element box | element box |
| `required.` as a page-only `value` | text mismatch | text mismatch | **word box** | **word box** | **moved box** |
| `required.` by its word span's `span_id` | span not found | span not found | **word box** | **word box** | **moved box** |
| `An entry is required.` by the run's `span_id` | run box | run box | run box | run box | run box |
| `is required.` by one word span's `span_id` | span not found | span not found | text mismatch | text mismatch | text mismatch |
| capability limits | `missing_char_offsets` | **none** | none | `missing_char_offsets` | none |

The element box is `[6160, 9717, 57464, 11390]`; the word box in C is `[19580, 9734, 22308, 10550]`.

**So a word box changes a result only for a claim that names the word's span, or a page-only value
equal to it, and there it changes the verdict** — no quote of two or more words binds to anything
finer than an element or a run. **The verifier takes span geometry on trust:** E, whose word box
lies on another line, passes `ethos grounding check` and is echoed back as an `exact_span`. And the
two decisions separate cleanly — B clears the capability limit with no word boxes, and D resolves
every claim exactly as C does.

## 5. What the corpus says about need

**Runs are mostly not word-shaped.** Of 1,329,323 runs, 886,018 (66.7%) are one character and 46,612
(3.5%) hold interior whitespace; 8.96% of non-whitespace characters sit in multi-word runs
(`census.py`). Of 1,280,432 runs in grounded blocks, 1,070,511 (83.6%) are a fragment of a word
(`runs.py`). It is bimodal by producer: on the five NIST documents a word takes a median two or three
runs, while `irs-fw9` draws whole lines — a median 29 characters and 5 words per run.

**How many words need a cut inside a run is a definition, not a measurement** (`words.py`, 317,164
words in grounded blocks). If a covering run may hold whitespace outside the word — its space glyph
inside the word's box — **85,542 (27.0%)** need one. If it may not, **181,479 (57.2%)**. On
`nist-sp-800-171r3` the two readings give 17.2% and 52.9%. Any future design has to choose, and say so.

**Where the precision goes depends on the claim's shape** (`areas.py`). A word or a five-word quote
cited by element gets a box whose area is, at the median, the first figure below times the union of
the runs drawing it; trimming those boundary runs to the claim would shrink that union by the second
— **an estimate**, apportioning a run's width by character count:

| document | word: element ÷ runs | word: runs ÷ trimmed | five words: element ÷ runs | five words: runs ÷ trimmed |
| --- | --- | --- | --- | --- |
| irs-f1040sd-2025 | 1.1× | 2.0× | 2.1× | 1.06× |
| irs-fw9 | 4.7× | 12.8× | 3.9× | 2.02× |
| nist-sp-800-218 | 13.5× | 1.15× | 4.9× | 1.00× |
| nist-sp-800-207 | 71.7× | 1.26× | 14.2× | 1.03× |
| nist-sp-800-171r3 | 33.4× | 1.07× | 9.1× | 1.00× |
| nist-sp-800-37r2 | 44.1× | 1.09× | 10.4× | 1.00× |
| nist-sp-800-161r1 | 38.6× | 1.08× | 7.7× | 1.00× |

For a five-word quote the loss is between element and runs on all seven. For a single word it is too
on the NIST documents, and the reverse on the two IRS forms, where runs are whole lines. §4 is what
decides: the element box is what a claim cited by element gets, whatever the spans hold.

**Every claim this repository verifies cites by element.** Eleven claims —
`verify_relay.rs:170-171`; `markdown_cli.rs:943-944`, `1156`, `1161`, `1355`, `1363`, `1373`;
`html_cli.rs:655`, `663` — all name an `element_id`, so each is answered with its element's box
whatever the quote spans.

## 6. What the tree has promised and refused

**No decision, roadmap row or `06-STEAL-REFUSE.md` row promises or refuses word boxes.** The only
tracked mentions are the 0.55.0 CHANGELOG entry and the `code_advances` comments, both saying the
data is crate-internal and on no wire. **`char_offsets` is the promise that was made.** M4 set it
false because *"v0 emits no element/span hierarchy for an offset to index into; it lands at M5"*
(`profile.rs:2179-2180`); M5 kept it false because v0 did no line grouping, so an element and a span
were the same object (`2be20f5`); `docs/history/08-V1-SCOPE.md:164` moved it to *"Whichever slice
makes elements coarser than spans"*; and when that slice came, it was deferred again: it *"belongs
in its own slice with its own evidence"* (`crates/ethos-parser-grounding/src/lib.rs:684-692`). This
document is that evidence (§8).

## 7. The refusal, and what would reopen it

**Refused: word-level spans with boxes, emitted by this engine, on the evidence in §4 and §5.** It
reopens when either of the first two conditions below holds **and the owner chooses to resume**; the
third comes first whichever it is.

1. **The verifier resolves a quote to spans inside the element it matched** — a word box, or a union
   of them, returned for a claim cited by element or by page. Testable at any new pin by re-running
   `runv.py`: rows one to three of §4's table change.
2. **A claims file from a consumer of this engine's artifacts cites by `span_id`.** Then §4's two
   cases stop being narrow.
3. **§9's reader defects are fixed first.** A word box cannot be cut from a run that has no box, and
   the boxes it would be cut from are wrong for rotated text.

**Neither condition 1 nor 2 is met at the v0.6.0 pin:** §4 shows the first, and every claim this
repository verifies cites by `element_id`. Whether a consumer outside it cites by `span_id` is not
something the tree can show.

## 8. Recommended instead: turn on `char_offsets`

**The owner's call, recommended on this evidence**, as its own slice:

- **It needs no new data.** The projection already builds each element's text by concatenating its
  runs, and a span's offsets are where its text lies in that string. They placed for every span on
  four gate documents — 970, 963, 56,768 and 82,909 — and both `ethos grounding check` and the
  engine's own checker accept every artifact; one offset shifted is `invalid_offsets` at `/spans/1`
  from both (`variants.py`, then both checkers, as the instruments README runs them).
- **It clears `missing_char_offsets` from the report on every artifact that carries spans** — verified
  against Ethos, the emitted artifact reports `capability_limited` with `missing_char_offsets` and the
  same artifact with offsets reports neither (§4, A against B). **An artifact past the 1,000,000-span
  cap keeps it:** `nist-sp-800-53Ar5`'s spans are withheld, its report carries `missing_spans` as well,
  and an artifact may not claim offsets without spans (`check.rs:741`).
- **It tightens no box.** Said plainly so nobody expects it to: B resolves every claim to the box A
  does.
- **What it changes.** `capabilities.char_offsets` flips in the profile, moving `profile_sha256`, and
  the `char-offsets-not-emitted` limitation leaves it. The projection writes offsets, stops refusing a
  profile that claims them (`crates/ethos-parser-grounding/src/lib.rs:890-898`), and must write the
  artifact's capability as `char_offsets && spans_emitted` rather than copying the profile's
  (`lib.rs:973-975`), or a span-capped artifact claims offsets without spans and both checkers refuse
  it. An emitter change — a MINOR.
- **What it corrects**, because each still gives the spent reason that there is no grouping:
  `crates/ethos-parser-core/src/assurance.rs:488-497`, the field's documentation at
  `profile.rs:614-621`, the assertion and its message at `profile.rs:3151-3157`, and the `why_not` at
  `crates/ethos-parser-pdf/tests/capabilities.rs:147-158`. The note at
  `crates/ethos-parser-grounding/src/lib.rs:684-692` is retired rather than corrected: it already
  records the reason as spent, and explains only why the flip was pending.
- **What it must prove.** A `true` capability names a proof test in
  `crates/ethos-parser-pdf/tests/capabilities.rs` (the rule at `profile.rs:603-605`). Offsets index
  the element's text in Unicode scalars, not codes and not UTF-8 bytes, so the test places a span in a
  run holding multi-byte text and a synthesized space and fails on a byte offset. The tests that pin
  `false` move with it: the mutation at `profile.rs:2013-2016` mutates toward `false` afterwards, and
  `crates/ethos-parser-cli/tests/grounding.rs:305` inverts.

## 9. Defects found on the way

**Not caused by this question, and none fixed here.** Each belongs in its own change.

1. **Rotated text is typed as drawing nothing.** The advance is read from the text matrix's `e` alone
   (`content.rs:730-732`), so a run whose text matrix turns the advance 90° or more, or mirrors it,
   advances 0 or less and is given `no_ink_to_measure` — the absence that says there is no ink. Probes
   p07 (90°), p08 (180°) and p15 (mirrored) show it. **On the seven documents 31,699 visible runs carry
   it: 48,692 of 1,730,148 non-whitespace characters (2.81%), on 579 of 733 pages**, and an independent
   reader reports every one as non-horizontal text; **`nist-sp-800-53Ar5` adds 4,685 runs, 7,399
   characters, on 88 pages** (`census.py`). A smaller rotation goes the other way: at 45° (p09) the
   advance is foreshortened, 1768 for 2500, and the box is `Measured` and upright. **This is the one to
   fix first**: it hides real text from grounding behind a false reason.
2. **A rotation outside the text matrix gives an upright `Measured` box.** A 90° CTM (p10) or a page
   `/Rotate` (p17) still produces a horizontal box, and both grounding checkers accept it. For p10 the
   CTM maps the advance onto the page's y axis, so from the font's ascent and descent the glyphs
   occupy x 142.82–152.07 and y 325–350, top-left; the box is x 150–175, y 342.82–352.07. **It is live
   on `nist-sp-800-53Ar5`:** its margin note is drawn under a 90° CTM — page 51's content sets
   `0 1 -1 0 23.8 0 cm` — and 52,644 runs at that note's origin x come back `Measured` and upright, on
   642 pages (`margin.py`, a count by position rather than by direction; the independent reader reads
   the note as vertical). No page of the seven declares `/Rotate`; a rotated CTM was not counted there.
3. **Word spacing is applied to two-byte codes.** `content.rs:729` tests `code == 32` whatever the
   code's width, so a composite font's `<0020>` receives `Tw`, against PDF 32000-1 §9.3.3 and the
   engine's own documentation at `text_state.rs:212`. Probe p26 advances 2500 where the specification
   gives 1500. Not counted in the corpus.
4. **A synthesized space can land on another line's run.** It is attached to whichever run was shown
   last (`content.rs:828`), so a `TJ` array opening with a large negative number puts the space on a
   run from an earlier `BT`…`ET`: probe p25 gives `ab ` at y 350 for a gap before `cd` at y 300. Not
   counted: 189 runs on the seven carry a synthesized space, and none was checked for this.
5. **Codes that decode to several characters carry measured boxes.** Twenty-nine runs in
   `nist-sp-800-53Ar5` — `ti`, `nti`, `onti`, `ati`, `ft`, `aft` — have one code standing for two or
   more characters and a `Measured` box (`ligatures.py`). The 0.55.0 entry's *"fires zero times over
   the four gate fixtures' 155 000 nodes"* is true of those four; the seven have 189 runs whose only
   mismatch is a synthesized space, and the eighth has these 29.
6. **A Type 3 font's height ignores its `/FontMatrix`.** Ascent and descent are divided by 1000
   (`fonts.rs:373-374`) while the font's widths go through its matrix (`fonts.rs:318-324`), so probe
   p20 — `/FontMatrix` 0.01, a glyph 70 units tall, 7 pt at 10 pt — gets a box 0.9 pt tall. Not
   counted in the corpus.

**And a sentence in the contract.** `docs/01-CONTRACT.md` §5.3 names *"loose em boxes —
ascent-to-descent, not ink"* as the failure to avoid, and says this engine *"emits measured ink boxes
only"*. The box is the font's ascent-to-descent envelope over the pen advance (§3), as the code
comments and the 0.47.0 CHANGELOG entry (*"`advance` is not an ink width"*) already say. It is
declared as what it is nowhere in the contract or on the wire; the contract should say what is
emitted.

## 10. What any future word-box design keeps

- **A word boundary is where the text holds a space, not a threshold.** A gap sized to word spaces
  was declined at 0.47.0: Latin had a trough and CJK, which draws no word spaces, had none
  (`CHANGELOG.md:1121-1122`). Re-measured at 0.54.0 over 749,409 same-baseline pairs, the Latin
  distribution has no trough either (`CHANGELOG.md:689-695`). On `irs-f1040sd-2025`, 236 of its 390
  whitespace-free same-baseline joins have a gap of at least a quarter box height, and 224 of those lie
  between two runs of dots (`joins.py`). Neither a space nor a gap is a role read off geometry, so P14
  refuses neither; the measurement refuses the gap.
- **The box keeps the run box's meaning** — pen extent and font envelope — and names itself `Computed`
  with a rule id in the profile.
- **Per-code advances are not on the wire today, and nothing forces them there.** A word box computed
  at projection would need them on `TextRunAttributes`: a representation schema version, and more
  bytes on every PDF run. Decision #20 does not require them: it binds values the contract requires an
  artifact to state, and the contract does not require these.
- **The span cap is all-or-nothing.** Past 1,000,000 spans every span is withheld. A span for every
  word of every run, beside the run spans, would take `nist-sp-800-161r1` to 1,017,409; for the words
  of multi-word runs only, to 538,695 (`spans.py`).
- **Span geometry is trusted by the verifier** (§4, E), so a wrong word box is echoed back as exact. A
  fixture that checks a word box against an independent reading belongs in the first slice.

## 11. What this changes in the tree

This document, its instruments, and two rows in [`02-ROADMAP.md`](02-ROADMAP.md). Nothing in the
engine, the contract or the profile moves here: §8 and each item of §9 are their own changes, and §9's
first item is recommended to go first.
