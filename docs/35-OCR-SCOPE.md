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
profile, on that page and nowhere else.

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

## 4. What is read

- **The trigger is a reason code, not a score** (`docs/07-VERIFY-BOUNDARY.md` §7). A PDF page is
  read when it carries `classify`'s `scanned` (no text-showing operator at all, and an image) and
  exactly one image placement. The coverage threshold the census used to sort pages is not a
  condition: one image on a page with no text layer is read whatever its size.
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
  Without `--ocr` an image file is refused by name, with the flag that reads it. **Grounding is not
  extended:** `ethos.grounding.v1` admits PDF sources and the page-less office ones only (decision
  #16), so `ground` refuses an image file by name.

## 5. How, in this code

- **`ocr/` is its own workspace** with its own `rust-toolchain.toml` (1.98.0; rten needs 1.94).
  The engine's pin (1.88) does not move, so nothing the engine writes without `--ocr` moves.
  `ocr/vendor/rten-vecmath` is rten-vecmath 0.26.0 with its sums order-fixed. `ocr/models` holds the
  three model files, built into the binary.
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
  none for an image file); and the word's quadrilateral in the image's own pixel grid, as the
  recognizer reported it. The run's box is its node's geometry. Its size is the height of the
  word's line, as a text layer written by Tesseract states it. A line that does not run left to
  right on the page — turned within the image, or upside down — is one run with no advance.
- **Derivation `Recognized`**, always, and never beside `Extracted`: the seal refuses a page that
  holds both, so contract §6's placement rule is enforced mechanically, not by convention.
- **The OCR profile** adds the recognizer's identity, so its `profile_sha256` differs from the base
  profile's (contract §6 rule 4). Without `--ocr` every artifact is unchanged but the default
  profile's hash, which the new field moved (§10.2).
- **Projections** need no rule of their own: Markdown, HTML, grounding and `locate` read recognized
  runs as runs, and each span's box is its word's page-space box. Grounding an image file is
  refused (§4).
- **No recognizer score is written.** Confidence is a diagnostic at most and never a filter; this
  slice has no use for it.

## 7. Refusals and counted absences

A page that needs OCR and is not read is counted under a named reason, never dropped in silence:
several images on the page; an inline image; a stream whose filter the recognizer does not decode
(CCITT, JBIG2, JPEG 2000); a placement that is not axis-aligned; a colour space or bit depth it does
not read; an image a soft mask or a mask shapes, since the protocol carries the image alone. A recognizer that refuses the CPU fails `--ocr` by name for the whole document, as a
missing one does. A word whose box's centre falls outside the page's visible box is not written and
is counted; one whose centre is on the page and whose box runs past its edge is written without its
box (`measured_off_page`), as a drawn run is; and one whose box encloses nothing — read in the
padding past a short line's end, a lone `−`, so both its edges fall on the line's last column — keeps
its text and reports no box (`not_reported_by_reader`), as a drawn run without an advance does,
where it refused the whole document (decision #125). Two declared limitations ride on every OCR artifact: text in a rotated or upside-down
orientation within an upright image is not turned; a recognized word has no font, so emphasis
(bold, italic) and a font's own size are not read on recognized pages.

## 8. What it does to `tag`, `overlay`, `ground` and the table rules

`tag` writes no structure for recognized text and says so. `overlay` draws recognized runs like any
others, and their derivation says what they are. The table, heading and reading-order rules read
recognized runs as runs; a rule that reads a font reads none.

## 9. What the owner decided

- 2026-11-10, "Go with all your recommendations, start Phase 1": build OCR now, ahead of v3;
  PP-OCRv5 mobile; rten; byte-identical on every CPU; a separate binary on its own toolchain; scans
  first, photos later.
- Contract §6: "Keep §6 as is" — pages with a scanner's stray text layer are not read.
- Numbering: "Renumber: OCR v3".
- CI: "Yes, push a CI branch now" (a throwaway branch; its results are in §3).
- Image files: "Yes, include image files".
- Models: "Commit them".
- Toolchain: "Install it".
- Releases, 2026-11-14: every release archive carries `ethos-parser-ocr` beside `ethos-parser`,
  built on its own toolchain and fingerprinted with one `--ocr` read (`docs/RELEASING.md` §8).

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

Photos (perspective, lighting, orientation); composite pages (mixed raster content, strips, soft
masks); CCITT, JPEG 2000 and JBIG2; grounding an image file; pages with a stray text layer (contract §6 as it stands); tables,
headings and emphasis on recognized pages; languages other than English. How the recognizer is
released is decided (§9): beside `ethos-parser` in every release archive.
