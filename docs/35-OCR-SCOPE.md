# 35 — OCR scope: a scanned page is read, and says so

**Status: accepted, built (S1-S4).** Written 2026-11-10 against `main` at `0e4eade`, workspace
0.66.0-dev.1. The owner took every recommendation of the Phase 0 measurements ("Go with all your
recommendations, start Phase 1") and the follow-ups in §9, recorded as North Star decision #120.
Where the build departs from the text below, the paragraph will say **Amended** and why.

Source: the Phase 0 and Phase 1 measurements of 2026-11-08 to 2026-11-10 (census, determinism,
accuracy, speed), summarised in §3. The private corpus named there is used as a bug-finder only and
is not redistributable; every other number is from ParseBench (`run-llama/ParseBench`, Apache-2.0)
or from this repository's own fixtures.

---

## 1. The one sentence

**With `--ocr`, a page on which the deterministic reader found no text layer at all, and on which
exactly one image was painted, is read by a pinned recognizer whose output is the same bytes on
every supported CPU**; each word it reads is a run of derivation `Recognized`, under the OCR
profile, on that page and nowhere else. **Amended (decision #156):** or several images, which the
recognizer composes into one raster with the rectangles filled between them (§14). **Amended
(decision #161):** or a page that showed a few words over a scan, its drawn runs kept and its
recognized words written only outside their boxes (§15).

## 2. Why now

North Star #11 kept OCR out until a deterministic ONNX lane existed; #45 put a Tesseract pass
outside the engine, in the benchmark adapter. Phase 0 measured what an engine lane would buy and
whether it could be deterministic; Phase 1 built the part that was in doubt. Both answers are yes,
and the owner moved OCR ahead of assist (#120, amending #15's order; by #15's own rule the lane is
renumbered **v3** and assist **v4**).

## 3. The measurement

**Census** (3,817 pages: ParseBench, opendataloader-bench, this repository's fixtures). 165 pages
need OCR: ParseBench 161, opendataloader-bench 1, fixtures 3. 115 are one decodable image (46 JPEG,
1 Flate then JPEG, 21 Flate, 5 CCITT, 42 image files); 19 are composites (12 mixed raster content,
6 strips, 1 soft mask); 12 need a renderer; 1 is JPEG 2000; 18 are other. No document mixes scanned
and digital pages.

**Accuracy** (Phase 0, reference Python pipeline). ParseBench, OCR on the 146 documents the engine
reads no text from or refuses: overall 52.10 → 55.88 with PP-OCRv5 mobile, no page down (Tesseract
55.08, Apple Vision 56.10). Six clean 200-dpi pages, word F1: PP-OCRv5 mobile .947, Tesseract .951.
A private corpus of 290 English pages: .917 (Tesseract .911). Synthetic phone photos, hard
condition: .890 (Tesseract .304).

**Determinism** (Phase 1). rten 0.26 adds sums per vector lane, so its bits depend on the CPU's
vector width (4 NEON, 8 AVX2, 16 AVX-512). With `rten-vecmath`'s sums made order-fixed (16 partial
sums whatever the width, one fixed tree), PP-OCRv5 mobile gives bit-identical outputs on Apple NEON
and Intel AVX2 at one, four and the default thread count on 222 pages (detection) and 3,062 line
crops (recognition). Against stock rten no detected box and no recognized text changed. The whole
Rust pipeline gives identical output on Linux x86-64 (AVX2), Linux arm64, Windows x86-64 with
AVX-512, and macOS arm64 (22 fixture pages, 1,405 lines). x86-64 without FMA still differs and is
refused. ONNX Runtime was not used: its bits change with the thread count.

**The Rust pipeline matches the Python one.** Clean pages .947 (Python .947); private corpus .917
(Python .917); photos equal or better in all six conditions.

**Cost.** About 4 s per 200-dpi page at four threads on an Apple M4 Pro, against 56 ms for a
digital page; up to 1.85 GB peak. 12.7 MB of models (Apache-2.0). **Phase 1**, with a page's lines
read in parallel: 1.9 s per page and 1.35 GB peak over the 22 fixture pages.

**On two threads, the size of a small server** (measured 2026-11-16, Apple M4 Pro, 12 cores, an
otherwise quiet machine). The recognizer runs two thread pools: `RTEN_NUM_THREADS` sizes rten's,
which runs the models, and `RAYON_NUM_THREADS` sizes the one that reads a page's lines in parallel.
The child inherits both from the engine's environment, so `RTEN_NUM_THREADS=2
RAYON_NUM_THREADS=2 ethos-parser markdown --ocr …` holds it to two. Six full-page scans
(`ocr/tests/data/nist-171r3-p55.png` and five of ParseBench's `text_ocr` scans) took 0.44 to 1.20 s
a page at the default thread count and 0.92 to 3.92 s on two, about two to three times as long;
peak memory barely moved, 0.27 to 0.74 GB a page either way, so memory, not threads, sizes the
server. The output is byte-identical at the default count, two threads and one.

## 4. What is read

- **The trigger is a reason code, not a score** (`docs/07-VERIFY-BOUNDARY.md` §7). A PDF page is
  read when it carries `classify`'s `scanned` (no text-showing operator at all, and an image) and
  exactly one image placement. The coverage threshold the census used to sort pages is not a
  condition: one image on a page with no text layer is read whatever its size. **Amended
  (decision #156):** or several placements, where everything the page paints is an image or a
  rectangle filled in a known colour (§14). **Amended (decision #161):** or a page that showed
  text, under 200 characters of it, every run boxed, over images that paint at least 40% of its
  visible box together (§15).
- **The image is decoded by the recognizer**, not the engine: DCT and Flate streams, Flate with
  PNG and TIFF predictors; DeviceGray, DeviceRGB, DeviceCMYK, ICC-based by component count,
  Indexed, CalGray and CalRGB; 1, 2, 4, 8 and 16 bits per component; `/Decode` and `/ImageMask`.
  **Amended:** CCITT was listed here and is not decoded — no decoder is in the dependency tree, and
  one is a new dependency or about 500 lines. A CCITT page (5 of ParseBench's) is refused by name,
  `filter-not-decoded:CCITTFaxDecode`, as JBIG2 and JPEG 2000 are.
- **Its placement is any of the eight axis-aligned orientations** of the unit square, after the
  page's `/Rotate`. The raster is turned upright before it is read and every box is mapped back.
- **An image file is a one-page document.** PNG and JPEG, known by their signatures at byte 0, are
  read whole; the page is the file's own pixel grid, one pixel to one point, which the document
  limitation `image-resolution-not-read` states. The source is the file — `image/png` or
  `image/jpeg` and its SHA-256 — and no image node is written, because the page is the image.
  Without `--ocr` an image file is refused by name, with the flag that reads it. **Amended
  (decision #132):** the page is built from the size the recognizer answers, so that size must be
  the one the file's own header states, read before the file is sent — a PNG's `IHDR`, or a JPEG's
  first frame header, found past tables, fill bytes and stray bytes as libjpeg finds it, where a
  height of 0, which a DNL marker sets later, states none. A header that states no size is refused
  `image-size-not-read`, and a size the recognizer answers otherwise `image-size-mismatch`, as a
  PDF image is against its dictionary.
  **Grounding is not extended:** `ethos.grounding.v1` admits PDF sources and the page-less office
  ones only (decision #16), so `ground` refuses an image file by name.

## 5. How, in this code

- **`ocr/` is its own workspace** with its own `rust-toolchain.toml` (1.98.0; rten needs 1.94).
  The engine's pin (1.88) does not move, so nothing the engine writes without `--ocr` moves.
  `ocr/vendor/rten-vecmath` is rten-vecmath 0.26.0 with its sums order-fixed. `ocr/models` holds the
  four model files — detection, recognition, its alphabet and, since decision #153, the
  page-orientation classifier (§13) — built into the binary.
- **`ethos-parser-ocr` speaks over stdin and stdout**: one JSON header line and the image's raw
  bytes per request; one JSON line per answer, carrying the recognizer's identity (its version, the
  models' SHA-256, the runtime and the pipeline rule id) and the lines with their words, or a named
  refusal. It refuses an x86-64 CPU without AVX2 and FMA. It reads a page's lines in parallel:
  rten's bits do not depend on the thread count.
- **The engine finds it beside its own executable**, or at `ETHOS_PARSER_OCR`. With `--ocr` and no
  recognizer, `extract` fails with a named error; it never falls back to an artifact without OCR
  (`docs/07-VERIFY-BOUNDARY.md` §8.6).
- **The engine pins the identity it expects.** An answer from any other recognizer is refused, so
  one profile can never hold two recognizers' output.

## 6. Where it goes on the wire, and what moves

- **Each recognized word is a `text_run` node** with derivation `Recognized`. No new kind: a run
  of text at a place on a page is the fact `text_run` carries, and the North Star's standing rule
  is not to add a kind for a fact an existing node carries. What differs is provenance, which the
  derivation and the locator state. So the reading-order, line, block, heading and table rules read
  recognized words as they read drawn ones — which is how Phase 0's 55.88 was measured: its OCR
  words were written into a text layer that the engine then read.
- **A new locator variant, `recognized`**: the run's place on its page as a text layer states it —
  the foot of its line under the word's left edge, and the word box's width as the advance —
  computed through the placement; the image it came from (its object and generation for a PDF,
  none for an image file or a page composed of several, §14); and the word's quadrilateral in the image's own pixel grid, as the
  recognizer reported it. The run's box is its node's geometry. Its size is the height of the
  word's line, as a text layer written by Tesseract states it. A line that does not run left to
  right on the page — turned within the image, or upside down — is one run with no advance.
- **Derivation `Recognized`**, always, and never beside `Extracted`: the seal refuses a page that
  holds both, so contract §6's placement rule is enforced mechanically, not by convention.
- **The OCR profile** adds the recognizer's identity, so its `profile_sha256` differs from the base
  profile's (contract §6 rule 4). Without `--ocr` every artifact is unchanged but the default
  profile's hash, which the new field moved (§10.2). **Amended (decision #132):** but a page
  the trigger takes, which a default artifact counts under `scanned-page-not-recognized` (§7); no
  rule id and no profile field moved for it.
- **Projections** need no rule of their own: Markdown, HTML, grounding and `locate` read recognized
  runs as runs, and each span's box is its word's page-space box. Grounding an image file is
  refused (§4).
- **No recognizer score is written.** Confidence is a diagnostic at most and never a filter; this
  slice has no use for it.

## 7. Refusals and counted absences

A page that needs OCR and is not read is counted under a named reason, never dropped in silence.
**Amended (decision #132, owner 2026-11-14):** that includes a read without `--ocr`. A page the
trigger takes — no text-showing operator, exactly one image placed and loaded, or several it
composes (§14), no inline image; or a few drawn words over such a scan (§15) —
is counted under the default profile by the reason the request for it would be refused for (a
filter, a mask, a colour space, a placement), the same name `--ocr` writes, and as
`ocr-not-requested` where the request would be sent. A page the trigger itself refuses — several
images it does not compose, an inline image, an image not loaded — is not counted without the
flag. With the flag or
without it, `extract`, `markdown` and `html` print one line on stderr naming how many pages were
not read and each reason, and offer the flag only for the pages not requested — `--ocr` on the
command that read the source, `extract --ocr` or `markdown --source FILE --ocr` on a record, which
the flag cannot read — since the Markdown and HTML artifacts carry no limitation; stdout is
unchanged. A recognizer's reason is written there with its control characters as spaces, so the
line stays one line. MCP prints nothing: `extract` returns the record, which carries every
limitation, but `markdown` sends a model the text without it (decision #36), so a scan left
unread goes unsaid there — a hint there is a follow-up. **A known limit:** the trigger has no size condition, so a
page that places one 1 x 1 spacer image and shows no text is taken, counted `ocr-not-requested`
and, under `--ocr`, sent; `--ocr` reads it under the same trigger, and nothing is drawn from size.
Under `--ocr`, the reasons:
several images on a page that paints something a composition does not hold,
`several-images:<what>` (§14); an inline image; a stream whose filter the recognizer does not decode
(CCITT, JBIG2, JPEG 2000 — the first filter that is neither Flate nor a final DCT, named by the
engine by the recognizer's own rule before the colour space, which a JPEG 2000 stream may leave to
its own header, and not sent); a placement that is not axis-aligned; a colour space or bit depth it
does not read; an image a soft mask or a mask shapes, since the protocol carries the image alone —
**amended (decision #156):** on a composed page, a soft mask that is a gray image with no `/Matte`
is carried and drawn (§14). A
recognizer that refuses the CPU fails `--ocr` by name for the whole document, as a
missing one does. A word whose box's centre falls outside the page's visible box is not written and
is counted; one whose centre is on the page and whose box runs past its edge is written without its
box (`measured_off_page`), as a drawn run is; and one whose box encloses nothing — read in the
padding past a short line's end, a lone `−`, so both its edges fall on the line's last column — keeps
its text and reports no box (`not_reported_by_reader`), as a drawn run without an advance does,
where it refused the whole document (decision #125). Two declared limitations ride on every OCR
artifact: text turned against the rest of its page is not turned (since decision #153 a whole page
turned a quarter or a half is, where the orientation model reads it so in every view, §13); a
recognized word has no font, so emphasis (bold, italic) and a font's own size are not read on
recognized pages.

## 8. What it does to `tag`, `overlay`, `ground` and the table rules

`tag` writes no structure for recognized text and says so. `overlay` draws recognized runs like any
others, and their derivation says what they are. The table, heading and reading-order rules read
recognized runs as runs; a rule that reads a font reads none. A recognizer's order is no evidence
of a table (decision #125), but a form's fields — labels each ending in their one colon beside
their values — are one in any order (decision #152), so a scanned invoice's fields read as the
drawn invoice's do.

## 9. What the owner decided

- 2026-11-10, "Go with all your recommendations, start Phase 1": build OCR now, ahead of v3;
  PP-OCRv5 mobile; rten; byte-identical on every CPU; a separate binary on its own toolchain; scans
  first, photos later.
- Contract §6: "Keep §6 as is" — pages with a scanner's stray text layer are not read.
  **Reopened 2026-11-22**, "Yes, build and measure": a scan carrying a few drawn words is read
  beside them (decision #161, §15).
- Numbering: "Renumber: OCR v3".
- CI: "Yes, push a CI branch now" (a throwaway branch; its results are in §3).
- Image files: "Yes, include image files".
- Models: "Commit them".
- Toolchain: "Install it".
- Releases, 2026-11-14: every release archive carries `ethos-parser-ocr` beside `ethos-parser`,
  built on its own toolchain and fingerprinted with one `--ocr` read (`docs/RELEASING.md` §8).
- Orientation, 2026-11-16, "as per your recommendation": a small dedicated page-orientation model,
  its Apache-2.0 licence checked, pinned and built into the recognizer, not the recognizer's
  scores (North Star row 11 stands); 2026-11-17, its download approved (§13).

## 10. The acceptance bar, set before the code

1. **Same bytes everywhere.** For the same request, `ethos-parser-ocr` answers identically on Linux
   x86-64 with and without AVX-512, Linux arm64, Windows x86-64, macOS arm64 and macOS x86-64, at
   one thread and at the default. An x86-64 CPU without FMA is refused by name. **Met** (§3: five
   runners, 22 pages, 1,405 lines).
2. **Nothing moves without `--ocr`.** Every gate artifact is byte-identical to `0e4eade`'s.
   **Amended:** but the profile hash. The default profile gained `recognizer: not_pinned`, so its
   `profile_sha256` moved (`0903edb3…` -> `8c2f8672…`) and every artifact's with it, as every
   release's does through `parser_version`. With that hash masked, the eight gate documents and
   opendataloader-bench's 200 extract byte-identical to `0e4eade`'s.
3. **With `--ocr`, a document with no scanned page gains no node**; only the profile moves.
   **Met:** the same 208 documents under `--ocr` hold the same nodes, and differ by the profile and
   its two declared OCR limitations alone.
4. **ParseBench with `--ocr` scores at least 55.0 overall** (52.10 without), and no page scores
   lower than without it. **Amended: missed — 53.94, 75 pages up and none down.** The bar was set
   from Phase 0's 55.88, which read every textless page by rendering it; this scope reads one
   image a page decodes. What the build does not reach is composite pages and pages that need a
   renderer (about 1.2 points, Phase 0's figures) and the image files' layout boxes, which need
   `ground` to admit an image source (about 0.75): two decisions for the owner, neither taken here.
5. **The placement rule is enforced**: a test that puts a `Recognized` node beside `Extracted` text
   fails the seal. **Met**, and the test fails when the seal's check is removed.
6. **Two runs are byte-identical**, as for every artifact, and so are runs at one thread and at the
   default. **Met** on six ParseBench scans and three of its image files, `extract --ocr`.
7. **Speed and memory are reported** per page against the 4 s and 1.85 GB of §3, with a target of
   at most 4 s median and 2 GB peak for a 200-dpi page on the M4 Pro. **Met:** 1.9 s per page and
   1.35 GB peak (§3).

## 11. Slices

| Slice | What | Done when |
| --- | --- | --- |
| S1 | `ocr/` workspace: decoders, pipeline, protocol, identity, models, tests | §10.1 in CI — **done** |
| S2 | Engine: `--ocr`, trigger, request, recognized `text_run` nodes, profile, refusals, projections | §10.2, .3, .5, .6 — **done** (§10.2 amended) |
| S3 | Image files as one-page documents | PNG and JPEG fixtures read — **done** |
| S4 | ParseBench adapter on `--ocr`; docs renumbered (OCR v3, assist v4) | §10.4, .7 — **done**, §10.4 missed |

## 12. What this scope does not decide

Photos (perspective, lighting, a photo's own orientation; a scanned page's quarter and half turns
are decided, §13); composite pages that paint anything but images and filled rectangles (pages of
images and fills are decided, §14); CCITT, JPEG 2000 and JBIG2; grounding an image file; a scan whose own text layer is long but
wrong (pages with a few drawn words are decided, §15); tables,
headings and emphasis on recognized pages; languages other than English. How the recognizer is
released is decided (§9): beside `ethos-parser` in every release archive.

## 13. A page scanned on its side or upside down (2026-11-17, decision #153)

A page fed to the scanner sideways or upside down was read as it stood: a NIST page turned a
quarter came out as `uoecn pssi ploin iud`. The recognizer now stands each page upright first.

**The model**: PaddleOCR's document-orientation classifier PP-LCNet_x1_0_doc_ori, in the ONNX
export PaddlePaddle publishes on Hugging Face (`PaddlePaddle/PP-LCNet_x1_0_doc_ori_onnx`, commit
`7330ab7`, Apache-2.0 on its card as on the Paddle original, SHA-256 `af9a0a4f…`; 6.8 MB). Its
digest is the recognizer identity's new `ori_sha256`, which the engine pins as it pins the others.
It reads the page as PaddleX's image classifier does — RGB, the short side resized to 256 as cv2's
INTER_LINEAR does, the centre 224-pixel square, ImageNet's normalisation — and labels it `0`, `90`,
`180` or `270`: how many quarter turns clockwise the page stands. The recognizer turns it back on
top of the turn the request declares, reads it, and maps every box back to the stored raster.

**Labels only, and only where every view agrees.** Taken alone, as PaddleOCR's own pipeline
takes it, the top label turned four upright pages of ParseBench's OCR lane — a line chart
(180°), a folded typed letter (270°), a handwritten page and a Japanese page — and the lane fell
54.66 → 54.63. Each of those reads otherwise in some view. So a page whose centre square reads
upright is read as it stands, and any other is turned by `k` only where the centre square and the
whole page squeezed to the same square, each turned 0 to 3 quarters more, read `k` and then each
quarter more: eight labels. No score is kept or compared (row 11).

**Measured**: the fixture page turned 0 to 3 quarters is found so in every view, and a stored
turned page sent with no turn declared reads as the declared one does, its quads in the stored
grid; every other fixture answer is unchanged but for the identity. ParseBench's `--ocr` lane
54.66 → 54.67: one page up (+0.16, a layout page scanned turned), none down. Opendataloader-bench
reads no image. The private real-document set: no Markdown changed (250 runs), resume scores
identical. A review's made-up pages — blank, black, gradients, pictures over the centre — stay as
they stand, and so does a body turned between an upright head and foot that only the centre
square reads turned; inverted, dark, yellowed, landscape and rescaled text pages are found, a
narrow column strip turned is not. An upright page
costs one classification (about 6 ms); a page that looks turned costs eight and three turned
copies (under 120 ms at 2550 × 3300 and 670 ms at 5100 × 6600, against 1.3 and 1.9 s to read it).

**Known limits**: a page the model does not read so in every view is read as it stands. A body
turned under an upright header — a landscape table printed on a portrait page — is turned whole,
so the header and folio are then read sideways. A turned page keeps its own frame: its runs lie as
the stored raster places them, each line one run with no advance, so the engine's layout reads
fewer of its paragraphs and none of its tables (a turned invoice reads its fields in order, not as
the table its upright copy makes). The rule id stays `ppocrv5-mobile-v1`, which no release
carries yet; `ori_sha256` tells a recognizer before this decision from one after it.


## 14. A page painted with several images (2026-11-19, decision #156)

A scan is often stored as more than one image: a background and the one-bit masks its text is
stencilled with (mixed raster content), or a page cut into strips. Such a page was refused
`several-images`. ParseBench-2's census found eleven among its 508 text pages, the cheapest of its
fixes that need no model. The owner chose a compositor in the recognizer over a renderer: hayro's
vello_cpu picks SIMD paths at run time, so its pixels on other CPUs are unmeasured
(`assist/README.md`), and §10.1 asks the same bytes everywhere.

- **What the engine sends.** The content interpreter keeps what the page painted, in paint order:
  each image a `Do` placed and each single rectangle a fill painted, as much of each as the page's
  rectangular clip and every open form's `/BBox` show, with the colour a stencil mask paints.
  Anything else refuses the page by name, `several-images:<what>`: a stroke (`stroke`); a path
  other than one rectangle (`path-not-rectangular`); a shading (`shading`); an inline image
  (`inline-image`); a clip of another shape, or a form's box its matrix turns by other than a
  quarter (`clip-not-rectangular`); a graphics state that may blend, soften or remap what is
  painted — a blend mode, a soft mask, a fill alpha, a transfer function
  (`graphics-state-not-read`); a layer membership dictionary (`layer-not-read`); a colour set in a
  space whose components do not read as a device colour's, or with another number of operands
  than the space has (`fill-colour-not-read`); an annotation a viewer draws over the page
  (`annotation`). What a layer switched off holds — a `BDC /OC`'s content, or an image whose own
  `/OC` is off — is not painted. A page whose request is refused names the request's reason, as
  one image's does: a filter the recognizer does not decode, a soft mask that is not a gray image
  without `/Matte` or that a stencil carries, a placement not axis-aligned or past a trillion
  pixels, or `composite-too-large` (below).
- **The raster** lies over the page's visible box at the finest density an image on it holds: at
  least a pixel a point, at most 300 dpi and 40 megapixels. Each image is sent once however often
  it is painted, and a paint over no area is left out. A page whose paints cover more than 400
  million pixels between them, or that holds more than 65,536 paints, is refused
  `composite-too-large`.
- **The recognizer draws it** (`ocr/src/compose.rs`) on white, in integers: a raster pixel takes
  the stored sample under its centre; a stencil paints its colour where it is black; a soft mask
  weighs its image over what lies beneath with one rounding; each image is decoded once. Then the
  raster is read as one image is, §13's orientation model included, and each word lands on the
  page through the visible box, citing no one image (§6).
- **Colours** are read as gray, RGB or CMYK by their space's count of components — a device
  space, `/CalGray`, `/CalRGB`, or an ICC profile — as an image's are (§4); a calibrated space's
  gamma or matrix is not applied.

**Reviewed** with made-up PDFs before it was measured: the first build read text a viewer does not
show through an image's own `/OC`, a transfer function, a form box turned 53°, a stencil's soft
mask, four operands in an RGB space, and an annotation drawn over the page; sent an image's stream
once per paint (4 GB for one image painted a thousand times); and refused a whole page for one
image painted over no area. Each is fixed above, with a test that fails without it.

**Measured** on `0f6c8a3` with this decision, with parse-bench 2.1.1:

| | Before | After |
| --- | ---: | ---: |
| ParseBench-2, `--ocr`, overall | 57.18 | **57.54** |
| — text | 61.74 | 62.51 |
| — formatting | 77.57 | 78.93 |
| ParseBench, `--ocr`, overall | 54.69 | **55.14** |
| — content faithfulness | 75.48 | 77.39 |
| — semantic formatting | 52.82 | 53.15 |

Nine of the census's eleven pages are read; the other two are refused by name (`JBIG2Decode`,
`CCITTFaxDecode`). ParseBench gains on 14 content pages, 2 formatting pages and 1 grounding page and
loses on none; ParseBench-2's text subset gains on 8 documents' text and 11 documents' formatting
and loses on none (one formatting part, lists, falls on a page that had no text before). Without `--ocr` nothing moves: both ParseBench lanes score the same
on every page, opendataloader-bench's 200 documents and the private real-document set's 250 write
the same Markdown, and the resume scores are identical.

**Known limits.** A page of one image keeps the path it had (§4): a fill, clip, layer or annotation
over a single scanned image is not read, since sending such a page through the composition would
refuse scans read today (a stroked border); that is a decision for the owner. JBIG2, CCITT and JPEG
2000 masks, common in scanners' mixed raster output, are refused by name. A composed raster carries
no orientation from a placement, so the orientation model reads it as it stands; on a made-up
cropped page turned 270° it read the raster wrongly. Samples are nearest, so a mask finer than
300 dpi is sampled down. The rule id stays `ppocrv5-mobile-v1`, which no release carries; a
recognizer from before this decision has the same identity and fails the read on the new request
kind, `composite`, rather than one page (both binaries ship together, §9).

## 15. A scan carrying a few drawn words (2026-11-22, decision #161)

**The rule.** Under `--ocr`, the trigger also takes a page that showed text when all of it is under
200 characters, every run of it has a box (a run that drew no ink, a run of spaces, covers
nothing and is not asked for one), and the page's upright images paint at least 40% of its
visible box between them (`thresholds::STRAY_TEXT_MAX_CHARS`, `STRAY_TEXT_MIN_IMAGE_PERCENT`) —
each as much of it as the page's rectangular clips and form boxes show, a place two of them paint
counted once, and an image a viewer does not show — drawn in a layer switched off, or switched off
by its own `/OC` — not at all (PR 58's review). The image is
read as any scan is, alone or composed with the others (§14): text the page shows no longer ends
a composition, and is not painted into it. The drawn runs are kept as drawn. A recognized word
is written only where its box lies on the page and shares no area with a drawn run's box; a word
with no box, one past the page's edge, or one beside a drawn run with no box, is left out and
counted under `recognized-words-under-drawn-text`. A line the recognizer reads turned, written as
one run, is written beside drawn text in runs each clear of it — the line split where the next
word's box would cross a drawn run's — so a word clear of the drawn text is not left out with one
that is not (PR 58's review). A page of one image, a few drawn words or none, is read only as far
as the page's clips show the image: a word read outside is counted under
`recognized-words-outside-visible-box`, as one past the page's edge is (PR 58's review).

**Why the seal moved, and how far.** Contract §6 let a recognizer author only where the page had
no text layer at all, and the seal refused any page holding both derivations. A scanner's stamp,
a court filing's header, an archive's title, a signature line: a few drawn words over a scan kept
every word of the scan from the record, and nothing said so. The seal now refuses a recognized
run whose box shares area with an extracted run's, or that has no box where the page holds
extracted text — the one test the reader applies before it writes (`QRect::overlaps`). Rule 1
stands: nothing overwrites `Extracted`; and a recognized run stays `Recognized`, under the OCR
profile, so the guard `07-VERIFY-BOUNDARY` §8.3 names — the derivation class and profile
isolation — holds for every run.

**Without `--ocr`.** A page the widened trigger takes is counted under
`scanned-page-not-recognized` as `--ocr` would leave it (decision #132): as `ocr-not-requested`
where the recognizer would read its image, and the stderr line offers the flag; by the refusal's
own name where it would not, such as `filter-not-decoded:CCITTFaxDecode`. The Markdown and HTML
are unchanged. The line now reads "scanned page(s) were not
read", since such a page shows text.

**Measured** on `7bca4fe` with and without this decision, the same recognizer, parse-bench 2.1.1
(and 2.1.0, the board's, for ParseBench-2):

| | Before | After |
| --- | ---: | ---: |
| ParseBench-2, `--ocr`, overall | 57.96 | **58.52** |
| — text | 62.51 | 63.71 |
| — formatting | 78.97 | 81.16 |
| ParseBench-2, `--ocr`, overall at 2.1.0 | 57.94 | **58.50** |
| ParseBench, `--ocr`, overall | 55.14 | **55.53** |
| — content faithfulness | 77.39 | 78.97 |
| — semantic formatting | 53.17 | 53.52 |

- **Without `--ocr` no Markdown moves** on any of 6,562 PDFs: ParseBench's 2,037, ParseBench-2's
  4,296, opendataloader-bench's 200 and the private real-document set's 29 (37 are refused before
  and after). The report moves: 65 documents now count one page under
  `scanned-page-not-recognized` that said nothing before — 51 as `ocr-not-requested`, 14 by the
  filter or colour space that keeps the image from the recognizer.
- **With `--ocr`, 44 documents' Markdown moves** — 23 distinct pages: 21 in both ParseBench sets,
  one table page of ParseBench-2's financial set whose rows are an image under a drawn title, and
  one opendataloader-bench survey page whose chart labels, which its reference holds, are an
  image (that benchmark is scored without `--ocr`, so its score does not move). Of the other 21
  documents the trigger takes, 14 are refused as without `--ocr`, 3 are refused
  `colour-space-not-read:jpeg`, and on 4 the recognizer finds no word.
- **Per page.** ParseBench: content gains on 21 pages and falls on one (below, −0.018); formatting
  gains on 4 and falls on none; tables, charts and layout do not move. ParseBench-2: every one of
  the 22 documents gains in words found; the parts that scored a near-empty page as clean fall
  where words now stand — integrity on 3 documents, critical words on 2, order on 2.
- **Words left out** beside drawn text: 59 on 6 pages, 41 of them on one page whose drawn numbers
  sit over a scanned article.
- **Re-measured after PR 58's review** on `0ee4b4c`, parse-bench 2.1.0:
  - ParseBench `--ocr` is 55.17 → 55.56 (content 77.52 → 79.12), and ParseBench-2 `--ocr` is
    58.19 → 58.76.
  - opendataloader-bench `--ocr` is 0.8287 → 0.8306.
  - One page moves that did not before the review: a page of turned text, whose words the split
    keeps (content 0.357 → 0.488).
  - The clip and overlap rule changes no page these sets hold: the same 65 documents report one
    unread page without `--ocr`.
  - The second round (hidden images, a one-image page read only as far as its clips show it)
    moves no page either; nor does the third, which checks each word of a turned line against
    what a viewer shows before the line is joined, a word no viewer shows dropped and counted on
    its own; nor the fourth, which keeps a turned line whole only where its own box's centre is
    shown too.

**Known limits.** The recognizer reads no Arabic script: on the one page that falls, a drawn
browser header over a scanned Arabic page, the scan's words come out as Latin letters it made up,
as an all-image Arabic scan's already do. Recognized lines change what the projection reads as
the page's body size, so a drawn run beside them can read differently — on one page a drawn `8`
is now written as a superscript; the run itself is unchanged. A born-digital page set on a
page-size background image, with under 200 characters, is taken too: its image is read, and
whatever text the picture holds is written beside the drawn text. A scan whose own text layer is
long but wrong is not taken (the census's F3, which would replace extracted text). A clip of any
shape other than a rectangle does not narrow an image's share. A turned line split beside drawn
text is read in the page's order, so one read upward comes out last part first. JPEG 2000,
CCITT, JBIG2 and the colour spaces the recognizer does not read are refused by name, as on any
scan.
