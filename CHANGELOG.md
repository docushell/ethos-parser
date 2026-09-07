# Changelog

All notable changes to ethos-parser, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

**Nothing is tagged or published.** Version numbers are in-tree; creating a tag or a release is a
separate, deliberate act. Nothing is on crates.io, npm or PyPI.

**Every version moves `profile_sha256`**, because `parser_version` is a profile field — so artifacts
from two builds are correctly non-comparable even when nothing else changed. That is the mechanism
working, not a regression.

Entries through 0.1.0 are grouped by **milestone** rather than by version, because a milestone was
the unit of work that had acceptance criteria. The per-slice reasoning behind each entry lives in the
milestone documents ([`05`](docs/history/05-MILESTONES.md), [`09`](docs/history/09-V1-MILESTONES.md),
[`11`](docs/history/11-V11-MILESTONES.md), [`13`](docs/history/13-V12-MILESTONES.md),
[`15`](docs/history/15-V2-MILESTONES.md)); this file records what changed.

---

## [0.53.0] — a median used as a hard bound split words in half

**Found by running OmniDocBench end2end for the first time.** An English chemistry page scored a
flat **1.0** on text edit distance, and the output explained why: `coordination` came out as
`coordi` and `nation`, `There` as `T` and `here`, `Figure` as `F` and `igure`. Not a decoding
failure — a join failure, mid-word.

**The cause.** [`ink_reach`](crates/ethos-parser-core/src/markdown.rs) capped a run's reach at
`glyphs × reference`, where `reference` is the font's **median** advance per glyph. A median is a
central estimate, so **half of all runs exceed it by construction** — and the join epsilon is 12
centipoints, so a fraction of one percent over the median is enough to manufacture a gap that is
not there.

Measured on `docstructbench_llm-raw-scihub-o.O-chem.200700133.pdf_6`:

| | |
| --- | ---: |
| `coordi` advance | 2 509 over 6 glyphs (418/glyph) |
| font median | 415/glyph |
| old cap | 6 × 415 = **2 488** — 21 centipoints short of the true advance |
| true gap to `nation` | **8** centipoints |
| gap after truncation | **29** — over the 12-centipoint epsilon |

**The fix is one token.** The cap is now `(glyphs + 1) × reference`: the same **one glyph of
slack** that `ink_sequenced` already allows on the overlap side. The bound was one-sided — a run
could overlap the next by a whole glyph and yet was refused a single centipoint of reach beyond a
median. No constant is introduced and the multiplier stays 1, which is what
[`ink_reach`'s own doc comment](crates/ethos-parser-core/src/markdown.rs) requires of it.

The `AC` cell-pitch run that the cap exists to refuse — advance 12 053 over 2 glyphs against a
median of 330, **eighteen times over** — is nowhere near the widened bound and is still refused.

**Corpus effect**, over the 981 OmniDocBench documents:

| | 0.52.0 | 0.53.0 |
| --- | ---: | ---: |
| median characters per block | 3.0 | **4.0** |
| median share of blocks ≤2 chars | 41% | **31%** |
| documents >50% tiny blocks | 306 | **253** |

Nothing else moves: groundability, artifact count, limitation codes and hard failures are all
identical.

**No golden changed, and that is the finding underneath the finding.** The engine corpus never
exercised a run whose advance sits just above the font median, so the whole test suite was blind to
this. A regression test now holds it — `a_run_wider_than_the_median_glyph_still_joins`, verified to
**fail against the old cap** rather than merely pass against the new one.

**What this does NOT fix.** Words are still separate blocks where the page drew a space between
them: the gap there is ~300 centipoints against a 12-centipoint epsilon, and bridging it is the
pitch-relative epsilon that 0.47.0 measured and declined — *"Latin has a trough to site it in and
CJK has none."* That refusal stands. The page above is materially better and still fragmented.

`markdown_rule` moves `markdown-blocks-v5` → `-v6` and `html_rule` `html-blocks-v5` → `-v6`,
together, because the change is in the function both projections call. Block-level grounding
elements move with them, since `geometric_blocks` is the same code.

---

## [0.52.0] — a table refused by hand and derived instead

**A reader changed.** 0.51.0 named its own limit: a width was found by asking the font's own
decoder what a code means, so coverage stopped at the ASCII range `StandardEncoding` carries. That
entry, and decision #22's row, both said widening it *"needs the Annex D glyph-name column, which
is its own measurement rather than a guess bolted on"*.

**The measurement was made, and it refused the obvious version.**
[`docs/21`](docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md) classified all 1 238 remaining absences by
resolving each node's `font_id` back to the `/BaseFont` its document declares:

| Class | Nodes | Share |
| --- | ---: | ---: |
| the font is **not** one of the standard 14 | 1 110 | 89.7% |
| not a text node | 77 | 6.2% |
| Core-14, every code ASCII | 44 | 3.6% |
| **Core-14 + a code `WinAnsiEncoding` defines** | **7** | **0.6%** |

**Seven nodes.** 96–224 entries of hand-transcribed specification data for 0.03% is the trade this
repository refuses elsewhere — `deny.toml`'s header makes the same argument about the same kind of
table, and `docs/20` §4 rejected pdf.js's metrics partly for carrying two verified `xHeight`
transcription defects.

**So the table is derived, not transcribed.**
[`vendor/generate-winansi-glyph-names.py`](vendor/generate-winansi-glyph-names.py) emits an entry
only where three independent sources agree: this repository's own `WIN_ANSI` code-to-text column,
Adobe's Glyph List — passed in by path and **not vendored**, because it is a tool used once rather
than data the build reads — and the glyph repertoire of `vendor/afm/`, which disambiguates the
AGL's several names for one codepoint and proves the chosen name is a real Adobe glyph rather than
a plausible-looking typo. A name whose AGL codepoint disagrees with `WIN_ANSI` is a hard failure,
because two vendored tables disagreeing is a reason to stop rather than to pick a winner.

**Two codes are refused by the generator: `0xA0` and `0xAD`.** Annex D notes that
`WinAnsiEncoding` also encodes `space` at `0xA0` and `hyphen` at `0xAD`, while `WIN_ANSI` decodes
them to U+00A0 and U+00AD, which is right for *text* and leaves no AFM glyph at that codepoint.
Both readings are real and they disagree, so neither is emitted — a width from the wrong reading is
a plausible number for a glyph the document did not ask for. **216 of 218** populated codes carry a
name.

**The guarantee is re-checked in-repo with no external source.** Two tests assert that every name
in the table is a glyph some vendored AFM carries — `eacutte` for `eacute` fails — and that the
table is populated at exactly the codes `WIN_ANSI` is, minus those two.

**It recovered exactly the seven nodes predicted.** Over the same 37 documents, measured ink boxes
move **23 885 → 23 892 of 25 123** and typed-absent **1 238 → 1 231**; the "Core-14 and a code
`WinAnsiEncoding` defines" class is now zero. Prediction and outcome agree to the node.

Seven is a property of *this* corpus — English scientific PDFs in unembedded Times and Helvetica.
The table is general: a population writing Latin-1 accented text in standard-14 faces gets far
more, and gets it without another decision.

**Profile.** `font_metrics_data_version` moves `core14-afm-1` → `core14-afm-2`, because it names
the metric data **and the join used to reach it** — the AFM bytes are unchanged and the route to
them is not. `cmap_data_version` deliberately does **not** move beside it: this table turns a code
into a width, never into different text, and a field that moved for both would stop telling the two
apart.

---

## [0.51.0] — the metrics §9.6.2.2 expects a reader to hold

**A reader changed, and it is the largest groundability move since 0.46.0.** A PDF may name
`/BaseFont /Helvetica` with no `/Widths` and no `/FontDescriptor`. The specification permits that
**because** a conforming reader is expected to hold the standard-14 metrics — so they are known and
merely absent from the file, which is the standing `fonts.rs` already gave an omitted `/DW`:
*"Reading a normative default is reading the document, not guessing at it."* This build holds them.

Adobe's 14 AFM files are vendored **pristine** in `vendor/afm/`, with `MustRead.html` beside them,
and embedded verbatim by `include_str!`. Decision **#22** of `docs/00-NORTH-STAR.md` is where the
licence was accepted; `docs/20-STANDARD-14-METRICS-SCOPE.md` is the measurement behind it.

**Measured before and after on the same 37 OmniDocBench documents**, with the same instrument —
every document declaring a Core-14 face with no `/Widths` and no `/FontDescriptor`:

| | 0.50.0 | 0.51.0 |
| --- | ---: | ---: |
| geometry entries | 25 123 | 25 123 |
| measured | 12 937 (51.5%) | **23 885 (95.1%)** |
| typed-absent | 12 186 (48.5%) | **1 238 (4.9%)** |
| documents declaring `font-widths-absent` | 66 | **0** |

**10 948 of 12 186 absent ink boxes recover — 89.8%.** A node with no ink box is omitted from
`ethos.grounding.v1` entirely, so those are runs that had text and could not be quoted and now can.
The geometry-entry count is **identical on both sides**: no text is gained or lost, and no node
appears or disappears. Only whether each one can be cited.

The spike in `docs/20` predicted 83.4% over 36 documents. It is not the same denominator — that
count was ungroundable *nodes* over a set found by the limitation code, this one is geometry
entries over a set found by scanning font dictionaries — so the two are close rather than
comparable, and the shipped number is the one measured on the shipped code.

**Verified against Adobe's published numbers by hand**, not merely for presence: the conformance
fixture `synthetic/simple-text` draws `Hello Ethos` in 24pt Helvetica, whose AFM widths sum to
5113/1000 em. 5113 ÷ 1000 × 24 × 100 = **12 271 centipoints**, which is exactly the advance the
engine now reports.

**What did not change, and is refused on purpose.** Supplying Helvetica's metrics for a font the
document calls `Arial` is a metric *substitution*, not a reading, and `afm::for_base_font` matches
the Core-14 names exactly and nothing else. The residual 1 238 absences are that refusal working,
plus codes outside this profile's encoding tables: a width is found by asking the font's own
decoder what a code means, so coverage stops at the ASCII range `StandardEncoding` carries. Widening
it needs the Annex D glyph-name column, which is its own measurement rather than a guess bolted on —
now made, and **refused**: it would reach **7 of those 1 238 nodes**. See
[`docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md`](docs/21-STANDARD-14-ASCII-COVERAGE-SCOPE.md).

**The licence is the real cost, and it is not OSI-approved.** APAFML requires that `MustRead.html`
travel with the files under that exact filename, that per-file copyright lines survive, that any
modification be prominently noted in the modified file — and, a fourth obligation the scope
document's quotation had truncated, **that the licence paragraph itself not be modified**. So
`MustRead.html` is vendored byte-exact, original classic-Mac CR line endings included. Nothing in
`vendor/afm/` is modified, so nothing there carries a modification note.

`APAFML` is **deliberately absent from `deny.toml`**. That allowlist governs crate licences in the
resolved dependency graph; these are data files and never enter it, exactly as `deny.toml` already
records for Adobe's CMap data. An entry `cargo deny` could never match is the "just in case" entry
that file's own header forbids. Stated rather than hidden: **a non-OSI-approved licence is present
and CI is green, because no tool here can check it.** The review is decision #22, `NOTICE`, and
`vendor/afm/README.md`.

**Provenance.** No copy of Adobe's original distribution is reachable today, so the files were taken
from `gettalong/hexapdf` and corroborated byte-for-byte against `yob/pdf-reader` — all 14 identical
— and against `UglyToad/PdfPig`, which agrees once one transformation is undone: it holds them with
`CR` replaced by `LF` rather than `CRLF` collapsed, which is why it was not used as the source. Each
file's sha256 and the pinned source commits are in `vendor/afm/README.md`.

**Profile.** A new `font_metrics_data_version` field names the metrics source, beside
`cmap_data_version` and deliberately separate from it: one names the tables that turn a code into a
character, the other the tables that turn a character into an advance. `profile_sha256` therefore
moves for two reasons, and artifacts from before and after are correctly non-comparable.

**Two fixtures moved, because the change made one of them vacuous.** `absent-font-metrics` named
`/Helvetica`, so after this it is measured and could no longer demonstrate typed absence — five
tests were left asserting the recovered path instead of the absent one. Its face is now `/ArialMT`.
A new fixture `absent-font-widths` carries `/ArialMT` with no `/Widths` and no `/FontDescriptor`,
which is the only remaining shape that reaches `font-widths-absent` and an absent advance at once.
The engine corpus is 43 fixtures, and the mutation survivor pin moves 65 -> 66 for the new
fixture's `junk-after-eof`, which survives on every fixture that opens at all.

---

## [0.50.0] — one refusal was answering for two different absences

**No artifact byte changes.** Only the text of an error, and only for documents that already
produced nothing.

`/Identity-H` and `/GBK-EUC-H` both reach the same refusal in `load_simple_encoding`, and it said:

> Predefined CMaps (the Adobe CJK set) are not vendored; a document needing one is refused rather
> than decoded approximately.

That is true of `/GBK-EUC-H`. It is **false of an identity CMap**: PDF 32000-1 §9.7.4.2 makes that
mapping the identity, so the code *is* the CID and nothing about the CMap is missing. What is
absent is the step after it — CID to Unicode — which here has no source at all, because the font
supplies no `/ToUnicode`. Adobe publishes such a mapping per registry and ordering and this profile
carries none; and where the descendant's `/CIDSystemInfo` ordering is `Adobe-Identity-0` the CIDs
are the subset font's own, so no published table decodes them either.

**8 of the 20 OmniDocBench documents that produce no artifact are that kind.** The old sentence
would have sent a reader after a dataset that could not have helped them — and nearly did: the gap
analysis that prompted this listed "vendor the Adobe CMaps" as fixing 20 documents when it fixes 4.

`load_cid_widths` already drew this distinction correctly for *widths*, with the reasoning spelled
out under "Why this refuses every encoding but Identity". The decoding path never got it.

### Why MINOR and not PATCH

`docs/RELEASING.md` §4 says PATCH only when output is byte-identical for the same input, and stderr
is bytes. Every artifact this build emits is byte-identical to 0.49.0's — verified across all 981
corpus documents: exit codes unchanged, markdown unchanged at 2 646 129 characters, no-artifact
count 18 both sides. A reviewer who reads "output" as the artifact alone would call this a PATCH,
and that reading is defensible. It errs the other way because 0.42.1's entry records the cost of
erring toward PATCH.

---

## [0.49.0] — a grounding element is the block now, not the glyph run

`ethos.grounding.v1` offers two granularities: coarse citable **elements** and finer **spans**
inside them, with every span naming its element. v0 could populate only one of them — with no
grouping in the engine, a run *was* the element and *was* the span. The code said so, and said what
would fix it: *"When grouping lands at v1 the element becomes the block and the span stays the run,
and this shape is already the right one."*

Grouping landed at 0.44.0 (marked content) and 0.47.0 (baseline ink). This connects it.

### What a consumer gets

A reader highlighting one quoted sentence on `irs-fw9` used to hold **970 glyph-run rectangles and
no rectangle for the sentence**. It now holds 334 elements over those same 970 spans, and
element 1 is `"Form  W-9"` with a box spanning both its runs.

| corpus | runs per citable element |
| --- | ---: |
| `fixtures/gate` (tagged US federal publishing) | **13.55** |
| `nist-sp-800-207` | **19.72** |
| OmniDocBench `v1_0` (untagged) | **1.24** |

**The same statistic means opposite things on the two corpora, and that is the honest reading.**
Where the producer declares marked-content groups the element is a real block; where nothing is
declared only the baseline join fires and the gain is small — the fragmentation that makes block
assembly hard on untagged input limits this too.

### What is measured and what is not

The grouping is `markdown::geometric_blocks`, which **calls** the clauses both projections already
join on rather than restating them, so the grounding artifact and the projections cannot disagree
about what one piece of ink is. It is `where`, never `what` — decision #19 — and it cannot cross a
baseline, so decision #21's territory is untouched.

- An element's **box is the union** of its members' measured boxes. A union of measured rectangles
  is measured; nothing is inferred.
- An element's **text is its members' own characters concatenated**, with no separator logic at
  all — because a space the page drew is a run with its own text. It is absent from `spans` only
  because it has no ink box to be cited by.
- A run a **table** already claims stays its own element, so no run is grounded twice.
- A block whose every member is ungroundable produces **no element**, and its members are counted
  in the omission report exactly as before.

### A rationale that had outlived its fact

`char_offsets` stays `false`, and the reason it gave is now spent: it said an offset "would always
be `0..len`" because element and span were the same object. An element now holds several spans and
an offset into its text carries real information. The capability is **not** flipped here — it is
`grounding-aligned` and the consuming validator enforces it, so it belongs in its own slice with
its own evidence — but the justification is corrected rather than left standing. A rationale that
has outlived its fact is the defect this repository keeps finding in itself.

### Tests

Two existing tests pinned the 1:1 shape and were rewritten to assert something **stronger**: that
every span sits in a real element and every element holds at least one span, and that an element's
box is exactly the union of its spans' boxes. Four unit tests cover the grouping itself, including
the drawn space as a member and the table-owned run that must not join.

No rule id moves — there is no grounding rule id, which is itself worth noticing.

---

## [0.48.0] — a legal hex string was fatal, and a symbolic font was decoded through the wrong table in silence

Two correctness fixes found by reading the OmniDocBench census rather than the code. Neither
changes a rule's **definition**, so no projection rule id moves; `parser_version` moves
`profile_sha256` as it always does.

### A hexadecimal string containing white space was refused

PDF 32000-1 §7.3.4.3: white-space characters **shall be ignored** inside a hexadecimal string. So
`<0009 000d 0020 00a0>` is exactly `<0009000d002000a0>`, and the `bfrange` array form
`[<0066 0066 006C><0066 006C>…]` is three ligature destinations. `hex_of` required every character
between the brackets to be a hex digit and reported `malformed` — and because `extract`'s page fold
returns the first page error, **one stray space in one font's `ToUnicode` CMap cost the entire
page**.

Two documents of 981 hit it. `scihub_s12935-018-0683-z.pdf_0` went from **no artifact at all to
4 970 characters** — a page carrying a table and a full abstract. Corpus effect: documents with no
artifact **20 → 18**, non-empty Markdown **733 → 735**, and **zero** documents lost a character.

Reading white space as a syntax error was not a stricter reading of the specification. It was a
wrong one.

### A symbolic font was decoded through `StandardEncoding` and the artifact did not say so

§9.6.6.2 gives `StandardEncoding` as the fallback for a **nonsymbolic** font. A symbolic font's
built-in encoding belongs to its own font program, which this profile does not read. Applied
anyway, a TeX math font decodes to the wrong characters — CMEX10 code 90 is `integraldisplay` and
arrives as `Z`, code 88 is `summationdisplay` and arrives as `X` — while the run reports
`scalar_code_mismatch: false`, because one code did produce one scalar. It was simply the wrong
one.

**The decode is unchanged, and that is a measured decision rather than a deferral.** 42 of 981
documents carry such a font, and the symbolic flag does not separate the two populations that
condition covers:

| font | what it is | decode through `StandardEncoding` |
| --- | --- | --- |
| `MathematicalPiLTStd-1`, `CGMathsBase`, `MTEX` | genuinely symbolic | **wrong** |
| `Europa-Bold`, `NewBaskervilleStd-Roman`, `EhrhardtExpMT` | ordinary prose that sets the bit | **right** |

Checked directly: the `Europa-Bold` document projects *"Older components such as carbon resistors
are really not worth keeping…"* — correct English. Refusing on the flag would have dropped correct
text from most of the 42 to fix a minority, which is `O21` inverted. Separating them needs the
embedded font program's own encoding, which this profile does not read.

So the fix is the disclosure, because **the defect was the silence, not the substitution**. New
document-scoped limitation `symbolic-font-builtin-encoding-assumed`, on **36 of 981** documents —
42 predicted, minus 2 that produce no artifact at all, minus 4 that name
`/BaseEncoding /WinAnsiEncoding` inside an encoding dictionary and are therefore decoded exactly as
the document asked. **Zero** documents changed a character, a node count or an exit code.

### What this is not

It is **not** a step toward vendoring the Adobe predefined CMaps. Measured, that buys less than the
`predefined-cmaps-not-vendored` limitation implies: of the 20 documents that produce no artifact,
**4** name `/GBK-EUC-H` and would be fixed by it; **8** name `/Identity-H`, which is not a
predefined CJK CMap and needs CID→Unicode tables instead; 3 need the embedded font program's
encoding; 5 were malformed `ToUnicode`, 2 of them fixed above. The error text for the
`Identity-H` group currently blames the unvendored CJK set, which is misdirection and is worth
correcting before anyone acts on it.

---

## [0.47.0] — an untagged PDF projected one block per run, and the median block was two characters

Until this release a run the document declared nothing about joined with nothing. `group_key`
returned `None` for any run without a structural locator, on the standing rule that **absence is
never a group** — written after an earlier draft read `mcid: None` as a group and welded
`nist-sp-800-207`'s vertical margin stamp into `Thispublicationisavailable…` across 156 pt of white
space, 59 times per document.

The rule was right and its scope was wrong. On a corpus where nothing is tagged, *every* run took
that path. Measured over all 981 born-digital PDFs of OmniDocBench's `v1_0` `ori_pdfs`:

| | before | after |
| --- | ---: | ---: |
| median characters per Markdown block | **2.0** | **3.0** |
| median share of blocks ≤2 characters | 60% | 41% |
| documents ≥95% such blocks | 163 of 733 | **41** |
| **share of all extracted text in those documents** | **52%** | **7%** |
| documents ≥200 characters *and* <50% tiny blocks | 277 | **351** |

### What changed

Two runs now join when they are **the next ink along one baseline**: same page, same region, same
stream (page furniture is not body text), same `origin_y`, drawn after rather than over, and no gap
the page drew. Absence is still never a group — what licenses the join is not the missing
declaration but the ink.

**`advance` is not an ink width, and that is the whole of the safety argument.** A table cell is
commonly drawn as one run whose advance is the *cell pitch*, so `origin_x + advance` lands inside
the next cell and a gap test reads ~0 across 120 pt of white space.
`docstructbench_llm-raw-scihub-o.O-ceat.200600410` draws `AC` at `origin_x` 31 181 with an advance
of 12 053 — 6 026 per glyph, against that font's median of ~330 — and the next cell's `AA` begins
at 43 229. Read naively they join and emit `ACAA`, a token the page draws nowhere.

So a run's reach is capped at `glyphs × the document's own median advance-per-glyph for that
(font, size)`, measured on the document being parsed. The bound this buys is **provable rather than
measured**: acceptance requires `next.origin_x ≤ prev.origin_x + glyphs × reference + 12`, so reach
per glyph can never exceed one reference glyph plus `12 / glyphs`, however badly `advance` lies. A
font seen once has nothing to corroborate against and is refused.

**No new constant.** The only number is the existing 12-centipoint quantization epsilon, now named
`INK_EPSILON_CENTIPOINTS` instead of a bare literal. The two other multipliers are 1 — one glyph's
width per glyph, one glyph of permitted overlap. A pitch-relative gap epsilon (`gap ≤ k × pitch`,
k ≈ 0.18) was measured and **declined**: Latin has a trough to site it in and CJK has none, because
CJK draws no word spaces, so it would be a measurement on one script and a tuned knob on the other.
That is why the `newspaper` family — 111 documents, the worst — is **not** fixed by this release.

### Two new census codes, counted apart on purpose

`baseline-run-joins-abutted-v1` and `baseline-run-joins-spaced-v1`, beside `mcid-run-joins-v1`. The
abutted form asserts two runs are one word; the spaced form only reproduces a space the page drew.
A join this engine measured is a weaker claim than one the producer declared, and pooling them
would erase exactly that difference. Read together on one document they are a derivation profile:
on a tagged document the producer's declaration dominates, on an untagged one every boundary
removed was removed on geometry alone.

The fallback emits `source`, never `source_continuing`, so **a `source` segment gains a node id
only from a producer-declared join or a hyphen closed up — never from geometry.** The seam between
two runs this engine joined stays addressable to the byte.

### What did not change, verified rather than asserted

- **No text moved.** The coverage census balances on all 961 artifacts: 1 623 979 emitted +
  106 184 dropped = 1 730 163 in representation.
- **42 of 48 fixtures are byte-identical**, including all 40 engine fixtures and both tagged IRS
  forms — everything there is declared, so the fallback never fires.
  `fixtures/engine/markdown-two-blocks/document.pdf` still projects as two blocks, which matters
  because it is the only end-to-end evidence for `capabilities.markdown` and `capabilities.html`.
- **No table was flattened.** Pipe counts are identical in all six changed gate documents. A
  table's runs reset the join state, as they did at v2.2-S1 — the reset measured at TEDS 0.104 → 0.000
  when an earlier draft was tried outside the projection.
- **The welding disaster does not reproduce.** `Thispublicationisavailable`, `NISTSP`, `ZEROTRUST`,
  `207ZERO` and `from:https` occur **zero** times on `nist-sp-800-207` before and after. The blocks
  the join creates there are `IST`, `-207`, `ER`, `T A`, `RCHITECTU` and `vii` — partial
  reassembly of the *horizontal* running head, longest 9 characters.
- **Zero fabricated tokens** (`ACAA`, `8DBBACAA`, `000000.26`) across all 961 documents.

### The suite could not see any of this, so a fixture was added

Every engine fixture stacks its runs on distinct baselines, so a rule keyed on "same baseline, next
ink along it" changed nothing in the CLI suite and passed it unchanged — the same blindness the
0.44.0 slice recorded. `fixtures/engine/untagged-shredded-line/document.pdf` is the tripwire: four
runs at one baseline in a font with real ink metrics and no structure tree, three abutting exactly
(12 points per glyph: 72+36=108, 108+24=132, 132+12=144) and a fourth 40 points further on. It
projected `Yar` / `ro` / `w` / `Separate` and now projects `Yarrow` / `Separate`.

Three mutants were watched failing, each caught by exactly the test written for it: dropping every
guard in `ink_sequenced`, dropping the reach cap (caught **only** by
`a_run_whose_advance_is_the_column_pitch_is_not_joined`), and making `LineKey` ignore the baseline
(caught **only** by `runs_with_no_declaration_join_only_along_one_baseline`).

### Both projection rule ids move, again

`markdown_rule` `markdown-blocks-v4` → `-v5` and `html_rule` `html-blocks-v4` → `-v5`, together,
because the clauses live in `markdown.rs` and `html.rs` calls them rather than restating them.
`profile_sha256` moves with them. MINOR rather than PATCH: the emitter produces different bytes for
the same input, which is `docs/RELEASING.md` §4's test.

### Not fixed, and named so the number is not mistaken for the whole

`newspaper` (111 documents) still reads at a median 90% sub-3-character blocks. CJK inter-glyph
tracking sits above the 12-centipoint epsilon by construction, and reaching it needs the
pitch-relative epsilon this slice measured and declined. The epsilon's own justification also does
not cover this population — it was measured on *declared* pairs of *one Latin document*, and the
13–150 centipoint band that is empty there is not empty on undeclared CJK pairs. That is now stated
on the constant rather than inherited silently.

---

## [0.46.1] — five of the six XHTML heading levels were reached by no test at all

`xhtml_heading_level` maps `h1`…`h6` to levels 1…6, and always did. Replacing the **h2–h6** arms
with `None` and running the whole workspace failed **zero** of roughly 1 300 tests.

Every `<h2>`–`<h6>` in every EPUB could have projected as a paragraph — in both syntaxes — and the
suite would have stayed green. That is 0.43.0's headline feature, verified at one level out of six.

### Why the gap existed, which is the interesting part

The **PDF** half of the same function has been guarded at every level since it shipped, by
`a_heading_role_from_the_tree_projects_as_a_heading` — a hand-built representation, for the reason
its own doc comment gives: *"no fixture in either corpus carries a heading role."*

The **XHTML** half, added at v2.2-S0, got no such test. Its only coverage was one end-to-end
assertion over `fixtures/office/book-spine/book.epub`, and that publication contains an `<h1>` and
no other heading. So the coverage was as complete as the fixture happened to be — which is this
repository's recurring defect wearing its politest face: not a guard that reads its subject
wrongly, but a guard that reads only the part of its subject the corpus supplied.

A fixture could not have closed it cleanly. Six levels through a real publication means a fixture
edit, a digest move and a golden move for a fact none of those are about. The end-to-end path was
already proved at `h1`; what was missing is that the **level follows the element**, and that is a
mapping, so it is now tested as one.

### Added

- **Four tests**, two per projection: all six levels at their own depth, and the near misses that
  the exact-match doc comment always promised and nothing checked — `hgroup` (which the comment
  names), `h7`, `h0`, `h11`, `header`, `hr`, `h`, and `H1`/`H2` (XHTML is XML and case-sensitive,
  so these are different elements and must not become headings).
- **`epub_repr_of`**, the page-less test builder whose absence *was* the gap: with no way to make
  an `EpubBlock` node, every test of that path had to go through a whole publication.

Three mutants were watched failing in both projections: `h2`–`h6` to `None` (the exact defect that
passed before), one level off by one (`h4` → 3), and an exact match replaced by a prefix test —
which is what turns `hgroup` into a heading, the case the doc comment warns about.

**The seal caught two errors in the new builder while it was being written**, and both were the
invariant working rather than being in the way: a page-less node parented by an invented page id,
and a payload whose geometry sidecar contradicted its own assurance block.

### Fixed

- A shipped error message in `representation.rs` loses a run of **eighteen stray spaces** mid-
  sentence, carried since 0.42.1 — *"the payload does not declare `…`"* was rendering as
  `declare` + 18 spaces + the code. Surfaced by hitting the error legitimately from a test.

### Unchanged

**Nothing this engine emits moves.** The mapping was already correct; only its coverage changed.
`profile_sha256` moves to `fa7e5994` because `parser_version` is a profile field and a build is a
build — the mechanism working, not a behaviour change.

---

## [0.46.0] — a composite font's widths were read from a key the format never puts them on

`load_widths` asked every font for `/Widths` and `/FirstChar`. That is the **simple** font shape.
PDF 32000-1 §9.7.4.3 puts a composite font's widths on its **descendant CIDFont**, as `/W` spans
with `/DW` as the default, and a `/Type0` dictionary carries no `/Widths` at all.

So every composite font fell through to "no width information" and reported an **unknown advance**
while the document supplied a perfectly good one. Measured on the 200-document
`opendataloader-bench` corpus:

| | |
| --- | --- |
| documents where a `/Type0` font was declared width-absent | **50** |
| …of which the file carried `/W` or `/DW` | **50** |
| composite fonts declared width-absent | **72** |
| …matched by a descendant carrying `/W` or `/DW` | **72** |

A 100% false-positive rate. The engine said "this document does not say" about a document that
said it plainly, on one file in four.

### What it cost, which is the part that matters

No width means no ink box, and no ink box means the node is **omitted from
`ethos.grounding.v1`** — that schema requires a bbox on every element, and fabricating one is
forbidden. So a seventh of the corpus could not be quoted, which is the one thing this engine
exists to make possible.

| | before | after |
| --- | --- | --- |
| text nodes omitted from grounding | 14 683 of 109 500 (**13.41%**) | **8 770** (8.01%) |
| documents declaring `font-widths-absent` | 51 | **1** |
| NID on `opendataloader-bench` | 0.8471 | **0.8490** |

**No text is gained or lost and the node count is identical** — 109 500 on both sides. Only what
can be expressed downstream changed. NID moves as a side effect: measurable advances let 0.44.0's
block assembly judge ink-contiguity it previously had to guess at.

### The same misattribution, one layer along, found while writing this entry

The number that belongs in the row above — *"…because the font gave no ascent/descent/BBox: 6 519
→ 131"* — **was itself misattributed**, and this entry very nearly repeated it. `extract.rs`'s
`_ =>` arm fills `NotReportedByReader` when EITHER the font metrics are missing OR the advance is,
and the sentence beneath it said, of all of them, *"their font supplies no usable ascent/descent
and no `/FontBBox`"*. A claim about ink envelopes, made over a bucket half of which was about
widths.

The two are separable with no new wire type — a run with no advance already carries
`advance: None` — so `geometry-absent-not-groundable` now counts them apart. What the corpus
actually holds, of the 8 770 still omitted:

| | nodes | is this a gap in this reader? |
| --- | --- | --- |
| no ink envelope | **130** | **yes** — the real metrics gap |
| no advance | **1** | yes |
| nothing to measure — whitespace runs | 5 592 | no; there was never a box |
| measured, and drawn off the page | 3 047 | no; the document's own choice (D4-S5) |

So the reader's own limitation is **130 nodes in 109 500 — 0.12%**, where before this slice the
artifact said 6 519 and named the wrong cause for **6 388** of them. Everything else omitted from
grounding is a property of the documents.

The one document still declaring `font-widths-absent` is a Type1 `Times-Roman` with no `/Widths` —
a genuine standard-14 case, and the only place the AFM sentence was ever true. The message that
was wrong 50 times in 51 is now right 1 time in 1.

### Added

- **`WidthSource::Cid`** — `/W` spans and `/DW`, keyed by CID. **Both** `/W` forms are read: `c [w1
  w2 …]` and `c_first c_last w`. Both occur in the wild — 1 143 and 810 entries respectively on
  that corpus — and a parser that implemented one would read the other's numbers as CIDs and build
  silently wrong spans.
- **`fixtures/engine/composite-font-cid-widths`** and **`composite-font-non-identity-cmap`**.
  Manifest `engine_owned` 39 → 41, fixtures 66 → 68, pinned survivors 62 → 64.

### Claimed only where the CID is knowable

`/W` is keyed by CID; `advance_glyph_space` is handed a character **code**. The map between them is
the `/Encoding` CMap, and this profile parses none — the standing
`composite-font-codes-from-tounicode` interim. Under `Identity-H` and `Identity-V` the map is the
identity by definition, so the code **is** the CID. Under anything else the advance stays absent,
because a width looked up with the wrong key is a plausible number for the wrong glyph, and a
plausible number is the one failure a consumer cannot detect.

The restriction costs nothing measurable: **all 78** composite fonts on that corpus declare
`Identity-H`. That is the claim `split_codes` already made in prose — *"right for Identity-H, which
is what real documents overwhelmingly use"* — and this is the first slice to put a number on it.

### Why 1 294 tests passed while this was true

**`grep -rl CIDFontType fixtures/` matched nothing**, in either owned corpus. The composite-width
path was exercised by no test at all. That is verbatim the argument v1-S6 used to justify
`image-xobject-drawn` — *"the corpus contains NO image XObject anywhere, so without this the whole
image path is untested"* — and it was available for four versions before anyone applied it here.

Five mutants were watched failing, at two layers: the `/DW` default replaced with zero, the `/W`
range made exclusive at its end, `Identity-V` dropped, the encoding refusal dropped, and the
`/Type0` dispatch removed (the original defect, restored).

**One of them found a defect in this slice's own test.** With `/DW 1000` in the fixture — which is
also §9.7.4.3's value for an omitted key — a reader that ignored `/DW` entirely still produced the
right number, and the mutant survived. The fixture now declares `/DW 900`. And the first draft of
the unit test walked the span table with its own copy of the lookup, so the exclusive-range mutant
survived there too; it now goes through `advance_glyph_space`. A guard that reads its own subject
through a private copy of that subject is this repository's recurring defect, and writing one
inside the slice that repairs an instance of it would have been a poor joke.

### Changed

- `geometry-absent-not-groundable` splits `NotReportedByReader` into "no ink envelope" and "no
  advance". Prose and counts only; no new type reaches the wire.
- `profile_sha256` moves to `cf5ee039`, on `parser_version` alone. No rule id moves.

---

## [0.45.0] — a page whose whole content was one `Do` came out blank, and said so nowhere

`extract` walks a page's `Do` operators and asks each XObject what it is. A `/Subtype /Image`
becomes a node. A `/Subtype /Form` returns `None` — this profile does not descend into form
XObjects — and until now `None` was the end of it: the placement was discarded and **nothing on
the artifact recorded that it had happened**.

So a page whose entire content stream is `q /Xf1 Do Q`, which is the shape a page-slicing tool
produces, extracted to zero nodes with `pages_failed: 0`. Nothing distinguished it from a page
that draws nothing at all.

### Added

- **`form-xobjects-not-descended`**, document-scoped, carrying a count. It fires only on documents
  that actually drew one, and says how many.
- **`fixtures/engine/form-xobject-text-drawn`** — the third member of v1-S6's `Do` pair, and the
  one placement that produces **no node of any kind**. It writes the same `Do` as
  `image-xobject-drawn` and changes the `/Subtype`, so the pair isolates exactly that. The form is
  deliberately well-formed, drawing its text through the page's own font object: a malformed one
  would also produce zero nodes, and then the fixture would prove that a broken stream is skipped
  rather than that a working one is not descended into. Manifest counts `engine_owned` 38 → 39,
  fixtures 65 → 66, pinned survivors 61 → 62 (`junk-after-eof`, as every engine fixture does).

### Why the limitation that already existed did not cover this

`form-xobject-text-not-descended` is **profile-scoped**: it rides on every artifact this engine
writes, including artifacts for documents containing no XObject at all. It states the policy and
never what the policy cost on this document. Both are now emitted, and the fixture asserts the
scopes apart — present on the form fixture, absent on the two image fixtures, while the
profile-scoped one is on all three.

This is the argument that already produced `unresolved_xobjects` and `inline_images`, two arms
away in the same interpreter: *"no image nodes" must not be able to mean "there were images and
the reader lost them".* Form XObjects were the one case it had not been applied to.

### Changed

- **The count is taken outside `capabilities.images`.** What it declares is text this reader did
  not read, not a picture it declined to emit; the capability now gates only the node. No PDF
  profile ships with that flag false, which is exactly why the branch is written down rather than
  discovered later.
- `profile_sha256` moves to `0a532bf7`. **No rule id moves, and that is worth stating**: the
  profile names the rules that decide what an artifact contains, and the set of limitations it
  declares is not one of them. A reader who found only `parser_version` different could otherwise
  conclude nothing had changed.

### Guards

Two mutants were watched failing, and they fail at different assertions: a counter that never
increments loses the document-scoped limitation, and a limitation pushed unconditionally
(`> 0` → `>= 0`) survives the positive half and dies on the negative one. The negative half is
what makes the positive one mean anything — a code that appeared on every document would be the
profile-scoped one under a second name.

`the_counter_list_is_complete` in `ethos-parser-office` caught the new accumulator on its own and
demanded it be listed. That derivation was added at v2-S13.1 against a hand-list that had shipped
short; this is the first real addition it has caught.

### Not in this slice

Descending into form XObjects. That is a reader change with its own resource-recursion, graphics-
state and cycle questions, and the count is what makes its absence legible in the meantime.

---

## [0.44.0] — a text run was its own block, and 68 112 of them averaged two characters

`nist-sp-800-207` projected as **68 112 Markdown blocks with a mean length of two characters**.
"NIST Special Publication 800-207" arrived as forty of them. `<p>Y</p><p>arr</p><p>o</p><p>w</p>`
is the same defect in HTML.

Both projections were **correct for the purpose they document** — a quote binds, the anchor map
tiles, and v1.1's gate holds — and unusable for the one their names imply. No consumer can read
that: not a person, not a retriever, not a model.

### Changed

- **One block per marked-content sequence the document itself declared.** 68 112 → **4 698**
  blocks, mean length 35. The grouping is the producer's own `BDC`/`MCID` marks, so joining
  claims nothing this engine inferred.
- **`markdown_rule` `markdown-blocks-v3` → `-v4` and `html_rule` `html-blocks-v3` → `-v4`**, both
  again, because the change is in `heading_level`'s neighbour — a rule both projections call.
  `profile_sha256` moves to `4bf3acf9`.
- **No representation changes.** `extract` output is byte-identical at equal version; this is a
  projection rule and the wire the projections read did not move.

### The rule, and why its default is to break

Four clauses. A space the page drew — in either run's bytes, **or as a whitespace-only run of its
own** — joins with one `syntax` space. A line break inside one sequence joins with one.
Ink-contiguity joins with nothing and coalesces into one `source` segment. **Anything else breaks
the block**, which is `on_different_lines`' own posture: *a missed join reads as two words the page
drew; a wrong join invents one.*

`region` is in the key because 58 of 974 groups span two of them, and joining across one welds over
a gutter — the clause `hyphen_tail` needed at D4-S3.

### Two designs this rejected, both measured before anything shipped

- **Reading absence as a group.** All 4 826 page-artifact runs on `nist-sp-800-207` carry
  `mcid: None`; grouping them per page welds the vertical DOI stamp, the running head and the folio
  across 156 pt of white space. `recalcuConfidential` again, 59 times per document.
- **Joining by default and inserting a space on evidence.** The space this corpus most often draws
  is a **run of its own**, dropped by the empty-text `continue` before any join state exists:
  `...subject to backup` + ` ` + `withholding` becomes `backupwithholding`, 3 292 word-boundary
  welds on one document. And `SynthesisReason::TjGap` does not cover the rest — it fires **zero**
  times on four of the five gate documents.

### Added

- **`mcid-run-joins-v1`** — every join, counted. Declaring 14 863 list-item joins while committing
  63 414 prose joins in silence was the asymmetry that made this mandatory. Not a `GFM_*` code:
  GFM does not cause it.

### Guards

**No existing test could observe any of this** — every CLI engine fixture is locator-less and the
shared unit builder hard-codes `mcid: 0`, so the whole suite passed unchanged while the output
moved by a factor of fifteen. Eight tests now hold the rule, and four mutants were watched failing.
One **survived at first**: the page-artifact test placed its runs 224 pt apart, so the gap clause
broke them and the key was never consulted. Making them ink-contiguous put the key under test.

### Unchanged, and checked rather than assumed

The census balances and is numerically identical — every join byte is `syntax`. **No two adjacent
`source` segments name different node ids**, still 0 of 171 418. A document that declares nothing
projects byte-identically: `markdown-two-blocks` 2 → 2, `book-spine.epub` 10 → 10,
`simple-paragraphs.docx` 4 → 4. Both projections report the same 63 414 joins, so they have not
drifted.

**No role is claimed.** This reports where the producer put a `BDC`; it decides nothing about where
a block is. `docs/19-BLOCK-SUBDIVISION-SCOPE.md` §9 measured mcid as line-like, so block size is
producer-dependent and the word "paragraph" appears nowhere in the rule.

---

## [0.43.0] — a heading the document declares reaches the projection

`heading_level` read one source. A PDF's tagged `/H1`..`/H6` became `# ` and `<h1>`; an EPUB whose
XHTML says `<h1>` in as many words became a paragraph. The reader had carried the element name for
exactly this purpose since v2-S9 — *"XHTML has no such distinction, so it is left false and the
element's own name is carried beside it instead"* ([`epub.rs`](crates/ethos-parser-office/src/epub.rs))
— and the projection never read it.

**This is not L29 arriving by the back door.** `<h1>` is the document stating a heading and its
level, the same kind of statement `/H1` is, and both are `Extracted`. No font size is consulted here
or anywhere else. The rule this repository keeps is not *"only PDFs have headings"* — it is *"a
heading is a heading because the document said so"*, which is what
[`markdown.rs`](crates/ethos-parser-core/src/markdown.rs)'s module header has always said.

### Changed

- **`markdown_rule` moves `markdown-blocks-v2` -> `markdown-blocks-v3`, and `html_rule` moves
  `html-blocks-v2` -> `html-blocks-v3`.** The first time both move together. They are separate ids
  so that they *can* move apart, which was never a promise that they always would: this change went
  through `heading_level`, which both projections call. `profile_sha256` moves with them and on
  `parser_version`, to `ecc17874`.
- **No representation changes**, for any format. This is a projection rule, and the wire the
  projections read did not move — so `ethos.parser.extract.v0` and `ethos.parser.representation.v0`
  are byte-identical at equal version, and only `ethos.markdown.v1` and `ethos.html.v1` differ, only
  for EPUB.

### Not in this slice, and each for its own reason

- **ODT, ODS and ODP.** `OdfBlockKind` is `Paragraph | Heading`: the *fact* of a heading is on the
  wire and its **level is not**, because the reader does not read `text:outline-level`. Emitting `#`
  for a block the file marks `outline-level="3"` would be a false claim about structure, so nothing
  is emitted. Closing it means reading the attribute, carrying it, and settling what an absent
  `text:outline-level` means in ODF — a reader slice with a wire change, not a projection fix.
- **DOCX.** Earlier still: the reader keeps no `<w:pStyle>`, so no heading reaches the wire at all
  and the projection cannot see one. Resolving a style name to a level means reading `styles.xml`
  and following style inheritance.

**`docs/CAPABILITY.md` said "office documents project too" with no caveat and now carries one**, so
the gap is stated where a reader looks rather than discovered by projecting a book.

### Guards

Neither projection had any test over an office representation at all, which is why this defect was
invisible to a suite of 1 380: `an_epubs_own_heading_element_projects_as_a_heading` and
`an_epubs_own_heading_element_projects_as_an_h_element` assert the `<h1>`, and both also assert that
the same fixture's `<p>` and `<td>` do **not** become headings — a rule matching any element
beginning with `h` passes the first assertion and fails the second.
`adding_the_epub_source_leaves_tagged_pdf_headings_alone` covers the cheapest way for this to have
gone wrong, which is the new arm shadowing the old one.
## [0.42.1] — ink the document draws off its own page

**Six of two hundred DP-Bench documents produced no artifact at all**, exiting 2 on the
box-within-page check. The seal refuses an out-of-page box on the stated grounds that it *"means
the measurement or the coordinate transform is wrong"*. For these six it means neither.

`01030000000029.pdf` sets `9.9626 0 0 9.9626 -435.1181 674.3054 Tm` against
`/MediaBox [0 0 510.236 737.008]`. The content stream itself places the text at x = −435.1181 pt,
and the engine reported `x0 = -43512` centipoints — that number, correctly transformed and
quantized. **The measurement was right and the transform was right**: the document draws an entire
column off-canvas, because the page was extracted from a wider original. All six are that shape,
and none is marginal — every off-page origin measured lands between −435 pt and −84 pt, never a
boundary nudge at the page edge.

**This is v1-S6.2 met from the other direction.** That slice found the same seal error over
rectangles drawn around whitespace and answered it by not claiming a box; its note that *"no run
with visible text is out of place anywhere"* was true of the two NIST documents in front of it and
is false in general. Here the run draws glyphs, the font supplies metrics, and the box is real.

### Fixed

- **`GeometryAbsence::MeasuredOffPage`** — the box was measured and the **document** places it
  outside its own page box, so no page-relative rectangle exists to report. The PDF reader decides
  this while the page is still in scope, and the run keeps its text, its origin, its region and the
  `off-page-text` finding this engine already raised for exactly that content before deciding to
  refuse the document over the box. Nothing is clamped — that fabricates a coordinate the document
  does not contain — and nothing is dropped, which would be a silent erasure.
- **The absence is counted, not merely spelled.** `check_structure` requires the geometry
  declaration whenever any node is non-groundable, so a document whose only absences were off-page
  boxes would have sealed with no limitation naming them and been refused — the annotation defect
  of 0.40.0, one reason over. `geometry-absent-not-groundable` now splits by three reasons where
  v1-S6.2 split by two, and says "Three" only when the third is non-zero, so a document with no
  off-page box carries the sentence it always carried, byte for byte.

### Unchanged, deliberately

- **The seal's invariant.** Its job is to catch an engine that computed a coordinate it cannot
  justify. A reachable case that is not that is a reason to teach the producer a new spelling,
  never to widen the one check standing between a transform bug and a plausible-looking artifact.
- **Every artifact 0.42.0 could produce.** All eight gate documents are byte-identical across this
  change, measured before the version moved: a box outside its page previously refused the whole
  document, so no document that sealed under 0.42.0 has one. What changed is which documents seal
  at all.
- **No rule id and no capability flag.** `parser_version` moves and `profile_sha256` with it,
  because `measured_off_page` is a value 0.42.0 could never emit and because "this engine could not
  read that document" and "this engine refused it" are different facts about a build.

### Added

- **`fixtures/engine/ink-past-the-media-box`** — a run with real ink metrics at negative x. The
  three neighbouring fixtures each stop one step short: `crop-box-smaller-than-media` puts a
  measured box outside the *crop* box, `off-page-and-offset-box` puts an *origin* outside it, and
  `whitespace-past-the-page-edge` puts a box outside the media box around *nothing*. Its second run
  is on the page, so the absence is proved per-run rather than a page-wide give-up.
- **`every_measured_box_this_reader_emits_survives_the_seal`** — `PageGeometry::contains` restates
  `check_box_within_page`, and two spellings of one invariant is what goes stale. The sweep runs
  every engine fixture through extract and seal, so a `contains` loosened relative to the seal
  fails here. All three new guards were watched failing under a mutation that reverts the fix.

### Documentation

- `docs/01-CONTRACT.md` §5.2 said **four** absence variants and listed four; the code had five
  before this change and has six now. Both the table and the count are current.
- `docs/draft-schemas/geometry.draft.json` enumerated **three**, having missed `no_ink_to_measure`
  (v1-S6.2) and `not_reported_by_structure_tree` (v2-S24). All six are declared.


### What else this version carried, unbilled until now

**0.42.1 shipped twenty-one commits and described one.** Everything above is the last of them. The
other twenty landed between the 0.42.0 release commit and this one, and no `CHANGELOG` entry named
any of them — so this section is the bill, written late, rather than a silent omission left to
`git log`.

**Robustness, and one repair of a repair.**

- **A ZIP's declared uncompressed size was reserved before a byte was inflated**, so a hostile
  header could ask for an allocation the archive never justifies. The first fix was itself a **20x
  memory regression** and is repaired here too — the sequence is in the history because a fix that
  costs twenty times the memory it saves is worth recording as a step, not smoothed away.
- **A ZIP comment containing `PK\x05\x06` displaced the end-of-central-directory record**, so an
  archive with those four bytes in its comment was read from the wrong place.
- **A poisoned font cache aborted the run**, and an id rebase panicked on three of eight node
  kinds.
- **Every CLI entry point read the whole file before checking its size**, so a size ceiling that
  existed was enforced after the memory had already been spent.
- **Neither SDK had a timeout, and the Node SDK buffered stdout without limit.**

**Bounds, including a new flag.**

- **`extract --max-pages`** — memory tracked page count and no caller could bound it. **This is a
  feature**, and it is what makes the version number below wrong.

**A guard that had never executed.** Three SDK version guards existed and none of them ran; the
number they were guarding had drifted **six minors**, with `0.36.1` sitting in a `0.42.0` tree.

**Performance, all of it byte-identical at equal version.** The central directory was walked twice
to read one part; the canonicalization emit path stopped cloning the payload to hand it over; the
test suite was compiling unoptimized and the table gate paid **13.6x** for it; the dependency cache
never refreshed, so the optimization it existed for never landed; peak RSS is medianed now, because
one sample of it was not a measurement.

**Documents.** [`docs/18-INTERNING-SCOPE.md`](docs/18-INTERNING-SCOPE.md) — role-path interning
measured and refused — and decision 20, *a constant today is a discriminator tomorrow*.

### The version number is wrong, and is left standing

**0.42.1 should have been 0.43.0.** `--max-pages` is a feature; the ZIP end-of-central-directory
repair widens the set of archives this engine accepts, which is a reader change by the precedent set
at 0.33.0 and 0.38.0; and the font-cache repair changes which documents produce an artifact at all.
The sentence *"a PATCH … no reader changed"* was true of the slice it was written about and false of
the version it was attached to.

**It is not renumbered**, and the reasoning is worth stating rather than assuming. Nothing here is
tagged or published, so no consumer holds a `0.42.1` to be confused by; `0.43.0` is already claimed
by the slice after this one; and `profile_sha256` `686e85cb` is pinned to the string `0.42.1` in
`profile.rs`, so a renumber moves a digest to correct a label. **Recording that the label is wrong
costs nothing and loses nothing. Moving it would spend a version to hide a mistake**, which is the
opposite of what the version field is for.


---

## [0.42.0] — the cut stops discarding its own grouping

`gutter-columns-v1` divided a page into column bands, subdivided each band, and then returned only
the permutation. So a consumer received the runs of a two-column page in the right order and could
not tell the page had two columns: the engine measured the page's structure and then declined to
say so.

**This is the first half of v2.2**, by decision #19. Auto-tagging is the second half, and it was
always going to need this one first — you cannot write a tag for an untagged document without first
deciding where its blocks are. No version number was invented; v2.2 already existed and this is the
half nobody had scoped. [`docs/16-D4-SCOPE.md`](docs/16-D4-SCOPE.md) is the scope.

### Added

- **`region` on every text run** — which region of its page the reading-order cut placed it in,
  1-based in reading order, on `TextRunAttributes`. **Layout is where text sits; structure is what
  text means.** A region is `Computed` from whitespace this engine measured and is never a
  paragraph, a heading, a section or anything a role can be read from — roles keep coming from the
  document's own structure tree or from nowhere. That separation is the capability, and it is the
  thing model-based extractors conflate by construction.
- **`ci/bench.py`** — median wall time and artifact size over the gate corpus, reusing the engine's
  own `--diagnostics` rather than a clock of its own. It exists because local performance was a
  priority nothing measured. Its first finding: **the engine is linear in what it emits**, flat at
  0.015 s/MB of output across every large gate document, while cost per megabyte of *input* varies
  more than threefold. `nist-sp-800-53Ar5` is 7.5 MB in and 932 MB out, and that expansion is the
  run time.

### Changed

- **`reading_order_rule` moves `gutter-columns-v1` → `gutter-columns-v2`**, so `profile_sha256`
  moves. **The cut did not change** — same constants, same recursion, and the fifteen ordering tests
  were not edited — so two artifacts either side list the same runs in the same sequence. The id
  moves because the artifact gained a field: one naming `-v1` promises no region, and a reader who
  could not tell them apart could not tell an undivided page from an older build.
- **Markdown and HTML stop joining a hyphenated word across a column gutter.** `hyphen_tail`
  already declined to weld across a page, a heading, a list item, a cell and page furniture; a
  column boundary was one no clause could see. `recalcu-` at the foot of the left column and
  `Confidential` at the head of the right projected as **`recalcuConfidential`**, a word the page
  draws nowhere and no citation can ground. Both projections share the guard, so one clause fixes
  each.
- `reading-order-geometric-only` now also declares what a region does **not** say: it is a column
  band, so runs stacked in one column share a region however many paragraphs separate them; it is a
  flat ordinal over a recursive cut, so it never says why a boundary exists or how deeply it nests;
  and absent means no division, which is not the same claim as single-column.

### Fixed

- **`$defs/text_run_attributes` forbade `findings`**, a field the engine has emitted since v1-S6,
  under `additionalProperties: false`. Two committed fixtures produce it. The published schema said
  the engine's own output was invalid, and no test validates an artifact against these drafts.

### Refused

- **Declared document splits (D1)**, on measurement rather than principle. Of five candidate
  signals three declare navigation or numbering rather than a boundary — an outline is a bookmark,
  page labels are numbering, an attachment is a separate file. The two that would be honest do not
  occur: across all 45 PDF fixtures `/Collection`, `/EmbeddedFiles`, `/PageLabels`, `/Part` and
  `/DocumentFragment` appear **0** times, and all eight gate documents declare exactly one top-level
  structure element. A detector with no positive case is an assertion.
  [`docs/17-D1-SCOPE.md`](docs/17-D1-SCOPE.md) carries both reopening conditions.
- **A visible block separator in the projections.** A region boundary is a *column* boundary, and
  GFM `---` after a paragraph line is a setext heading underline while `<hr>` is by definition a
  *thematic* break. Both would read a semantic claim off a geometric fact.

### Notes

- **What this does not close.** A region opens only on a vertical cut, so paragraph structure on an
  untagged single-column page is still unavailable — recorded in `CAPABILITY.md`'s Cannot table
  rather than left implied.
- **Throughput**, on a quiet machine at `--repeat 3`: all eight gate documents within +3.9% / −2.1%
  against artifact growth of 0.03% to 2.1%. The undivided page allocates nothing.
- `ci/gate.sh` now locates the pinned oracle, so nine tests in `html_cli`, `markdown_cli` and
  `verify_relay` stop failing locally for a reason unrelated to anyone's change. The header's claim
  that the built-in fallback worked was wrong: it resolves against the caller's working directory,
  and `cargo test` sets that to the crate root.

---

## [0.41.0] — the engine becomes `ethos-parser`

Renamed from `ethos-engine`, and the name was wrong in two ways that only get more expensive to fix.

The mechanical one: `engine` and `engine-core` are both taken on crates.io, so the five crates could
never have been published under the names they had. **Any release required renaming them anyway**,
and doing the product rename separately would have meant two migrations.

The larger one: `ethos-engine` reads as the verifier's internals, and this is not that. The parser
and the verifier are separate products that compose. **A document parser is useful to anyone with
documents; a citation verifier is useful to a narrower set.** Naming the broader tool after the
narrower one told most of its potential readers they had found an accessory.

### Changed

- **All ten `ethos.engine.*` artifact types are now `ethos.parser.*`** — representation,
  classification, extract, overlay, and one per office format. That is why this is a minor rather
  than a rename.
- The MCP server name, the environment variables, and the CLI binary all follow.
- **The profile hash moves for two different reasons.** A PDF artifact moves on `parser_version`
  alone — its backend name is the library that reads the bytes, and renaming the crate calling it
  does not touch that. The eight office profiles move for two reasons, because their backend name
  really was the crate name.

### Deliberately unchanged

- **`ethos.grounding.v1`** — the verifier's format, owned elsewhere.
- **`ETHOS_BIN`, `ETHOS_FIXTURES`, `ETHOS_BENCH_CORPUS`** — they address the verifier's tree, so
  renaming them would have pointed the oracle harness at itself.
- **The bare English word "engine" in prose.** This is still an engine, and 2,826 sentences saying so
  would have been mangled by a substitution that cannot tell a product name from a common noun. An
  audit caught four places where a first pass renamed a manifest key and left the documentation
  describing a root that does not exist.
- **The archived patch in `docs/attic/`** keeps its pre-rename paths, because rewriting an archived
  patch would make it claim to apply to a tree that did not exist when it was written.

## [0.40.2] — the overlay note counts the tables it cannot draw

### Fixed

The overlay's per-page note exists so that **an overlay drawing only the boxes it has cannot make a
partly-read document look fully read.** Since tagged tables became first-class records carrying
absent geometry — found, real, and undrawable by construction — the note counted only the geometric
population. On one document's page it reported "0 tables … 0 marked items have no rectangle this
overlay can draw" about a page carrying two tagged tables. **Both numbers were wrong, and wrong in
the direction that reassures.**

Both now cover both populations, and the note names the third cause alongside the two it listed.
Nothing in the representation moves.

## [0.40.1] — the assurance envelope guards the parse door too

### Fixed

The assurance type says of itself that an artifact claiming completion while carrying a quarantined
page "is not a bug this type can have", and its constructor earns that by deriving coverage from the
page states and the terminal state from the coverage. **Reading took the wire's word for all three.**
A derived deserializer over five public fields is not a constructor, and the fingerprint check cannot
close the gap — **it binds a payload to itself, not to the truth of what the payload asserts.**

Demonstrated rather than argued: a real artifact was edited to quarantine a page while its coverage
and terminal state kept claiming completion, its digest recomputed with the published canonical-JSON
rules — no secret is involved, the canonicalizer was reimplemented in twenty lines and checked
against an untouched artifact first — and `ground` accepted it, exit 0, projecting from a record
whose own pages contradict it.

Deserialization now re-derives both figures the way the constructor does and refuses a disagreement,
naming which it found. **Nothing this engine emits changes** — the serializer is untouched — and all
sixty artifacts across both corpora round-trip unchanged, which is the check that the new door
refuses only forgeries.

## [0.40.0] — a node whose kind has no ink box stops refusing to seal

### Fixed

The structure check requires the geometry-absent limitation to be declared exactly when some node is
non-groundable — and an annotation, a form field and an image are each non-groundable by
construction, because **their rectangle is a number the author wrote into a dictionary, not ink this
engine measured.** The producer triggered that declaration on ink-absent *text runs* only. The two
populations disagreed and the seal enforced the wider one, so **a PDF whose fonts supply real metrics
and which carries a single annotation was refused outright** — exit 2, no representation, no
grounding, no Markdown, no HTML.

**Real files escaped by luck rather than by design.** One whitespace-only run or one metric-less font
supplies a text-run absence that fires the declaration for an unrelated reason — one document has
11,421 of the former — and every fixture carrying an annotation or an image also has an unmeasurable
lone text run, **so the combination was never built.** Reproduced by adding one annotation to a
fixture whose text does measure: exit 0 before, exit 2 after.

The same change corrects the ink sentence's denominator, which counted every node while its numerator
counted text runs — reporting "1 of 3 text nodes" about a document with one text node. **"This kind
has no ink" and "this reader could not measure the ink" are different statements**, and the artifact
already keeps them apart everywhere else.

Six of fifty-two fixtures move, every one carrying a non-text node. The office readers are untouched
and their artifacts byte-identical, **which is the check that this is a producer fix and not a change
to what the seal means.**

## [0.39.0] — the office formats stop being stranded

The largest capability gap the audit named, closed from both sides. The verifier-side revision that
was recorded as *owned elsewhere rather than refused* landed first — the grounding schema now admits
page-less media types under a version-gated union — and this slice takes it.

### Added

`ground` projects a page-less office representation into that shape:

- **`pages: []`**, because a page-less source states no page and synthesizing one is the invented
  pagination the law refuses.
- **Every element under its own node id**, because a page-less artifact carries no spans and the id a
  consumer joins back by has to live on the element.
- **The native locator serialized canonically beside the text** — opaque to the verifier, exactly
  reversible by a consumer holding the representation.
- **No geometry anywhere**, so nothing is omitted for lacking a box.

**A PDF projection is byte-identical to what this engine has emitted since M5.** The engine's own
validator mirrors the verifier's page-less rules code for code.

The refusal tests that pinned the old wall flipped into emission tests the way they were built to,
and the test-only subset validator learned the union's applicators rather than waving them through.
**Proven end to end with real binaries on both sides:** a DOCX extracted here, grounded here, checked
by the sibling verifier, and verified — grounded at element scope, **which is the precision a
page-less address can honestly claim.**

## [0.38.3] — the payload is walked once per artifact

### Changed

The emit tail the previous version named. The seal was already canonicalizing the whole payload to
hash it; printing then walked the same payload again — on a 932 MB artifact, most of the remaining
wall clock.

The seal now keeps the bytes it hashed, and the print splices them into the envelope through a joiner
that sorts keys and refuses duplicates exactly as the full serializer would — **so the spliced emit
is the full serialization by construction, and by a test that pins the two routes byte-equal.**

**The cache is derived state, not identity:** it never serializes, and equality remains the five wire
fields, **because a minted artifact must equal its own parsed round-trip.** A parsed artifact carries
no cache and takes the full pass unchanged.

Measured: one document falls from ~50 s to ~40 s. The extraction batch's whole journey now reads
100.4 s → ~40 s **at byte-identical output per version.**

## [0.38.2] — pages extract in parallel, and the artifact cannot tell

### Changed

The workspace's first threading dependency (reviewed against the dependency policy before addition)
parallelizes the per-page half of extraction. Each page runs the body the sequential loop always ran —
**carved out verbatim** — against the shared read-only handle with a page-local id allocator. A
sequential fold then walks the results in page order, rebases every id onto the document-global
sequence, folds each counter with the same saturating arithmetic in the same order, and returns the
first error in page order, **which is where the sequential loop always stopped.**

**The subtlety the byte oracle caught before commit:** a refused table candidate consumes an id it
never ships, and the artifact keeps that hole. So the fold rebases by offset and replays allocation
counts rather than renumbering emitted entities, **which would have closed every hole and shifted
every id after it.** Even the id-overflow refusal still fires at the page it always fired at, because
the replay allocates through the same guarded path.

**Proof is byte comparison at equal version** across the gate documents, the 932 MB artifact included.
**The honest number is modest and says where the next slice lives:** the 492-page document falls from
~57 s to ~50 s, because the emit tail now dominates. Amdahl's receipt, named rather than rounded up.

## [0.38.1] — the clustering lead, measured and refused

### Measured, not shipped

The audit's highest-confidence ruled-rule recommendation — cluster a page's rectangles into connected
components and judge each alone, so a stray painted box stops refusing the clean grid beside it — was
implemented in full, run against the twelve-document gate, and **refused on its numbers**:

| | before | clustering |
| --- | --- | --- |
| Macro cell-F1 | **70‰** | **63‰** |
| The corpus's best document | 590‰ | 490‰ |
| Geometric tables emitted | 18 | 112 |
| False-positive slots added | — | ~2,000 across six documents |
| Fabrication | 0 | 0 |

The best document **split into five single-dimension fragments**, because the page-global lattice was
the very mechanism unifying its form rows — and sixty-five furniture grids arrived on one document
alone. **Fabrication stayed 0, which is the "worse than a zero" shape: real text arranged into grids
that are not there.**

A single-dimension refusal was probed against the artifacts and not written: **the ten prose stacks it
clears and the fragments it would delete are the same predicate.**

The code is reverted and the per-document table is kept, with the finding the next attempt has to
answer: **any per-region ruled rule needs a region-merging step strong enough to reunify a form's rows
before it can afford to judge regions alone.**

## [0.38.0] — numeric character references resolve everywhere, and the router is shared

### Fixed

**A decision recorded rather than made, now made.** `&#233;` is a scalar written another way — no DTD
required — and six of the eight office readers refused it by name while the EPUB reader resolved it
through a hardened resolver **nine lines away**. **Every valid document containing one character
reference was unreadable in six of eight formats** — an unbounded per-document cost held against a
bounded one-time price.

All six now resolve references in text **and in the names attributes carry**: a sheet named with an
escaped accent is an address, and **refusing the whole workbook over a well-formed name was a
wrong-cause refusal.** The six rule ids move a version because the behaviour they name moved, **which
is the whole function of a rule id.**

**Named entities beyond the five predefined stay refused.** `&nbsp;` is an HTML name an XML parser
without a DTD cannot resolve, and **a name that silently became an empty string would be a character
dropped from evidence.**

The same change ends the format-dispatch fork: the MCP `extract` tool called the PDF reader directly,
**so a DOCX over MCP was refused for lacking a PDF header** — the wrong-cause refusal three earlier
slices each retired on the CLI surface while the MCP surface silently kept it. Routing now lives in
one function both surfaces call, **so a fix there is a fix everywhere**, and the PDF path opens from
the bytes the router already read, so a file is read exactly once end to end.

## [0.37.2] — the detector quadratics go

### Changed

The second half of the performance repair, on the same proof: **gate artifacts byte-identical at equal
version**, each rewrite argued equivalent at the site.

- The ruled rule's coherence precondition was cubic — every face re-scanned every rectangle, and a
  property of the rectangle alone was recomputed per pair. **Covering a face whose edges are lattice
  lines is an interval condition on the line indices**, so each rectangle now marks its covered block
  in one difference grid.
- Lattice-line lookups become binary searches with the linear scans' exact first- and last-match
  semantics.
- The cross-check's overlap test becomes a sweep that finds the same pairs and re-sorts them into the
  order the wire has always recorded — **4,096 cells made the all-pairs form 16.7M tests per table.**

Measured: the densest gate document falls 2.04 s → 1.75 s; the largest, 59.9 s → 56.7 s.

**The run-to-cell assignment scan stays quadratic deliberately:** cells can overlap, a run inside two
cells belongs to both, and a bucketed rewrite that preserved that faithfully was not worth its risk.

## [0.37.1] — the emit path stops building every artifact twice

### Changed

A performance repair **with a byte-identity proof**: at equal version, every artifact is byte-for-byte
what the previous build emitted. The proof is not a sentence — a 932 MB artifact and a second one were
compared byte-for-byte at every step, and the canonical-JSON property suite gained an equivalence law.
Measured: a 492-page extract fell from **100.4 s to 59.9 s**.

- **Canonical serialization streams.** The old route built a full document object model and then
  canonicalized it, allocating a map insert and a key string per field per node before writing one
  byte. Byte-equivalence with the old route is a property test, **refusal messages included**, and
  objects still sort at write time so the map-ordering hazard cannot reach this path.
- **The artifact-per-serialization clone is gone.** A serde attribute made the whole representation —
  every run's text and codes — clone once per serialization to re-emit the same five fields under the
  same names. A hand-written serializer emits the wire shape without it; the wire struct still owns
  parsing, where the structural checks live, and a test pins the shapes equal.
- **Fonts parse once per document** rather than per page, cached by object id **and resource name** —
  the name is part of the key because it is baked into a font's error strings, and **one object under
  two names must not share those bytes.**
- **The hot loop stops allocating per glyph**, and runs move instead of cloning. The structure-tree
  reconciliation reads a per-page index built **after** the reading-order pass, because an index built
  a line earlier is exactly the stale-index bug the join's own comment warns about — caught by the
  byte oracle during this work, before commit.

**What the adversarial review on this diff caught, fixed before commit.** A malformed-but-parseable
tagged cell can cite the same marked-content id twice, and the new index would have bound the doubled
citation twice where the old scan bound it once — proven by byte-comparison of crafted fixtures
against a pre-change binary. The streaming serializer's docs also over-claimed equivalence: it is
deliberately **stricter** on three inputs no workspace type produces, which now **refuse loudly**
instead of emitting invalid JSON or silently dropping a field.

**Named misses, left open honestly.** The payload is still serialized twice per artifact and pages are
still sequential — both are the next slices. A benchmark harness is still absent: a criterion
dependency would put ~30 crates in front of the licence gate, **which is a vetting decision, not a
patch.** The measurements are from `/usr/bin/time` on the gate corpus, method stated so they can be
recomputed.

## [0.37.0] — v2-S24: tagged tables

### Added

A **fourth** detection rule, emitting a table for each one the structure tree *declares* that no
geometric detector matched.

- **`Extracted`, where a geometric table is `Computed`** — the document stated the grid. **This
  inverts the usual intuition: the tagged table is the stronger claim.**
- **Geometry typed-absent** under a variant meaning *this source has no box*, distinct from *the
  reader could not measure one*. No box is invented; the representation schema moves for it.
- **A not-applicable cross-check, never `ok`** — it compares two derivations, and a tagged table
  supplies only one. **A stray `ok` would be the check passing a comparison it never ran.**
- **Omitted from grounding**, which requires a box, and disclosed by name.

**Measured: combined micro recall 4‰ → 502‰** (7,924 of 15,755 gold slots), 157 of 172 gold tables
emitted, 15,593 cells, **fabrication 0**. The geometric gate stays at 70‰ macro and 4‰ micro,
deliberately — **scoring a tree-derived table against the tree it came from is circular**, so the two
are measured apart. **The gate being unmoved is the proof the detector did not move.**

## [0.36.3] — v2-S23: the coverage two slices retired

### Changed

No reader moved. Two coverage claims resolved rather than left ambiguous: one verified still reachable
on the wire through a geometric-only fault, and one — a backend's catalog-scan recovery — **argued for
deletion rather than papered over with a fixture**, since it is a backend leniency the engine never
guaranteed.

## [0.36.2] — v2-S22: why ten documents produce nothing

### Added

A per-gold-table diagnostic that walks all 172 tagged tables and records, for each page: what ink it
carries, which rule built a candidate, and which precondition rejected it. **Deliberate-run, never in
CI**, and it cross-checks its own emitted tables against the extractor's before reading a single
refusal, **so a drift in the mirror is a test failure rather than a wrong number.**

**The finding: the alignment rule emitted 0 tables across all 172 gold tables.** All 17 detections are
on the two documents that draw their grids. **The engine ships a rule id, a profile field and a slice
of machinery that has never produced a table on a real document**, and a test now asserts that count
stays zero so the claim cannot lapse silently.

**And it names the precondition, uniformly.** On every gold page in all twelve documents the alignment
rule refuses at the column-gutter floor — **the text's own columns sit closer than 12 pt, which is
prose spacing, not a table gap.** That is step 2 of the rule, so it never reaches the whole-page
lattice earlier analysis blamed.

**Micro recall is stated beside the macro for the first time: 4‰**, 70 of 15,755 slots.

No detector, rule id, tolerance or profile field changed.

## [0.36.1] — v2-S21: the mutation that missed

### Fixed

A mutation kind had claimed since M7 to reach the file trailer's cross-reference pointer and instead
seeked a fixed fraction of file length — **landing 373 KB short on the largest fixture.** It now seeks
from the trailer, **shown rather than asserted.**

**All eighteen newly-refusing mutants give the same named reason.** Two fixtures leave the flip
entirely. What stopped being covered is said rather than left implicit.

The manifest did not move, **which is the one coupling an earlier slice warned about.**

## [0.36.0] — v2-S20: the nine grids the engine already rejects

### Changed

**A minor that REMOVES output.** The ruled rule now declines a grid its own **structural** cross-check
rejects, rather than emitting it with a disagreement recorded beside it.

**Why the field beside the grid was not enough:** the Markdown and HTML projections draw **every**
table the artifact carries and consult no check. **So "keep and declare" delivered nine grids to a
consumer and delivered the contradiction to nobody.**

**All three options were measured on all twelve documents and the gate cannot tell them apart** — the
macro reads 70‰ under every one. **So the decision is argued from the artifact, not from the gate.**

**The cost is twelve cell slots and every one is the empty string** — blank faces agreeing with blank
tagged cells. **No character of extracted text is lost anywhere in the corpus.** As a rate: 1,028
cells emitted and 11,307 slots predicted, to get 12 right — **one per 941 wrong.**

**Only the structural half gates, and that was measured rather than reasoned.** Gating on the whole
check refused a 2 × 2 whose only defect is one edge a **single centipoint** out — what a 1 pt stroked
rule looks like. The structural half is arithmetic on indices the rule assigned and admits no
tolerance; the geometric half compares exact boxes against a lattice built *with* one, **so it fires
on the slop that tolerance exists to absorb.**

**The gate is in one rule because it can only fire in one rule** — the other two build a cell per face
from the same lines the box comes from, so gating them would be dead code, and a test asserts it.

A shipped fixture changed its job: it used to prove the engine emits a self-contradicting grid and
says so; it now proves the engine refuses one and says why.

## [0.35.0] — v2-S19: the corpus that was never grown

### Added

**The gate corpus goes from four documents to twelve** — 2,068 pages, 172 tagged tables, 15,755 cell
slots — committed to an engine-owned root, **because a corpus you publish numbers about has to be one
anybody can re-measure.** The four had lived in a tree this repository does not own.

**Admission is a rule now, not a judgement**, with five conditions. **Personal documents are refused
on two grounds:** a benchmark corpus is committed and has numbers published about it, **which is
publication, and a digest is not anonymisation** — and a résumé's layout is not what this will meet,
so admitting one would move the number **without anyone being able to say whether the detector
improved or the corpus got easier.**

### Measured

**Macro 64‰ → 70‰ across a threefold corpus.** Read alone that is stability. **The band refuses that
reading:** 0‰..590‰, median 0‰, **ten of twelve at exactly zero**, two documents supplying all 849
averaged points, and removing one dropping the macro to 23‰.

**So neither number was ever a property of this engine.** What twelve documents establish is the
**shape**, and it is bimodal.

**Fabrication is 0 across all twelve.** Cross-check disagreements went 0 → 9, **all nine on one
document** — which detects nine tables against four tagged, contributing 11,295 false-positive slots.
**It is not fabrication:** the detector arranged real text into a grid that is not there. **And the
engine already knew** — there are exactly nine detected tables there, so the cross-check was rejecting
every one.

**A gap closed as a side effect:** the document carrying the largest share of the gate number had no
manifest entry at all. An earlier slice pinned that gap and said the day it closed the guard would
fail and bring whoever closed it back to that paragraph. **That is what happened.**

**Decision #18 was written here and NOT decided.**

## [0.34.3] — v2-S18: the gate that has never been green

### Fixed

**`cargo fmt --check` had been red since v2-S14, across four slices that each recorded a green** —
because nothing ran it. The finding underneath the defect: **no "green" in this repository had ever
been a fact.** The local gate script is now guarded against the workflow in both directions, and the
guard was **watched failing six ways** and restored each time.

## [0.34.2] — v2-S17: the two guards outside `src`

### Fixed

**No line of `crates/*/src` changed at all.** The interesting one is a guard **passing while covering
four fifths of its subject** — a floor set one below its own population, which is the shape an earlier
slice exists for.

The fix derives the floor from the enum rather than hardcoding it, and asserts *which* stages were
walked rather than how many. **Covering the verify stage was refused on principle, not on cost:** it
would report the pinned verifier's behaviour as evidence about this engine, and a byte-identical relay
**cannot have injected anything at all.**

## [0.34.1] — v2-S16: the instrument four slices rebuilt wrong

### Added

A committed code-line extractor, because **three of four ad-hoc rebuilds were wrong, each in a way the
others could not see** — which is the existence case for making it a deliverable.

**One pass over each file producing two strings of identical length**, with a seven-axis control that
**runs on every invocation rather than behind a flag** — each axis observed failing under a deliberate
break before it was trusted. It lives beside the other scanners rather than in a crate, and **it is
not a new CI job.**

**One precondition it claimed did not survive measurement**, and neither is asserted now — the
instrument does not depend on either. The one it does have, it checks.

## [0.34.0] — v2-S15: the `neither detector` cluster

### Changed

**A minor, because two of the cluster's fifteen sites are emitted wire strings** — an artifact this
build emits differs from the previous build's for the same bytes.

**The cluster moved as one**, because repairing the comments alone would have left the wire saying one
thing and the comments another.

**They were wrong at birth, not rotted.** There had been three detectors since v1-S8 while the strings
said "neither". **The substance is true and stays true** — no detector reads the header tag — and the
code was correct.

**The new wording carries no ordinal**, so a fourth detector cannot re-rot it. **The blast radius was
measured before the wording was chosen:** the representation hash moves, the profile hash does not,
the oracle does not, and neither mutation harness does.

## [0.33.1] — v2-S14.1: the guards that were never written

### Added

Two guards that two comments had named as existing. **Each was broken on purpose and watched go red.**
The first asserts the claim rather than something adjacent to it, and **the fixture is built so a lazy
implementation cannot pass it** — the table's runs are interleaved with a second column's, so the
remap is not the identity.

**One handed finding did not survive the re-check**, in the file cited as its own evidence. **That
made seven consecutive slices in which a recorded finding disagreed with the code.**

## [0.33.0] — v2-S14: the CRC-32 question, answered

### Fixed

**A minor, because a reader changed.** The ZIP reader verified a part's declared length and never its
checksum, **so a corrupted part that still inflated to the right size was read as though intact.**

**The measurement came first and decided the shape:** the refusal shipped only after the false-refusal
rate was measured at **zero over 40 valid packages and 2,370 entries**, because **refusing a valid
archive would be a regression dressed up as a hardening.** The instrument was negative-controlled, and
measured with **the engine's own decompressor** — a mirror implementation would have measured a
different thing.

**The error is named** distinctly from a length or signature failure, so a caller can switch on the
cause.

**Detection does not verify, and finding that out cost a regression.** The first version broke routing:
the entry reader runs during *detection*, so a corrupt part made the router fail closed **naming the
wrong cause** about a document that plainly is what it says it is. Measured, not reasoned.

Mutation survivors fell 36 → 31, emptying one class entirely. **The emptied class is pinned rather than
deleted**, so a mutant reappearing there reads as a regression rather than as noise.

## [0.32.5] — v2-S13.5: the two sweeps that never ran

### Fixed

Two read-only sweeps an earlier slice planned and did not run. They found that the verify-boundary
document had been claiming to hold the settled decisions *verbatim, identically* **while holding
fourteen of seventeen**, and that **two comments named tests that have never existed.**

Two clusters are named and deferred whole rather than half-repaired, **because half a repair is
worse.**

## [0.32.4] — v2-S13.4: the gate's verb, and what "embedded assets" meant

### Changed

The owner settled two of the three standing questions, recorded as decisions #16 and #17.

- **#16 — *ground* means *bind*.** Read literally the gate contradicts itself: grounding requires
  pages and *no synthesised pages* is the same sentence. Read as *bind* it is met.
- **#17 — counted satisfies v2.** Reading an office asset is explicitly not v2 — an office image has
  no page and no coordinate system, **so a node for it is a contract change rather than a reader
  change.**

## [0.32.3] — v2-S13.3: the statements that stopped being true

### Fixed

The prose half of an earlier confirmed list of false statements. **One cluster had to be one decision**
rather than a site-by-site edit. **Two errors this slice introduced were caught by an adversarial
pass**, recorded because a repair that introduces defects is worth knowing about. 1,789 candidate lines
examined; 15 confirmed mismatches.

## [0.32.2] — v2-S13.2: the roadmap reordered

### Changed

An owner decision, recorded rather than argued. The ladder after v2 becomes **v2.2 → v3 → v4**.

**OCR was renumbered rather than only resequenced**, because `parser_version` sits inside
`profile_sha256` and **a later build carrying a lower number defeats the one job that field has.**
v2.1 is now a gap, and nothing ever shipped under it. Accessibility's conditional gate is withdrawn —
**a conditional parallel lane cannot also be the mandatory next row.**

No code, no reader, no profile field.

## [0.32.1] — v2-S13.1: the guards that check nothing

### Fixed

**Not one non-comment line in any `crates/*/src` file moved.** Guards that read their own subject
wrongly, including **one token that had been dead for twenty-two commits.**

**The repair is a rule, not a number** — a module-path rule rather than a second exemption array —
and **verified by breaking it, twice.** Twelve findings confirmed and five more found by three
independent sweeps, **so "five" is a result rather than a mood.**

## [0.32.0] — v2-S13: A11's other half

### Added

**Mutation testing for the office packages**, an obligation due since v0. Every one of the sixteen
packages damaged **twelve** ways — 148 mutants, survivors pinned in five explained classes.

**A second harness rather than a second manifest root**, because the existing harness feeds every
manifest entry to the PDF opener, **so office entries would have been refused as non-PDF while every
assertion still passed.**

**The kinds are not the same six, and that is a result rather than a shortcut.** Five carry over with
their mechanics rewritten around the ZIP; one reduces to nothing for a package and is **dropped rather
than faked**; five are new, because a container has hazards a byte stream does not. **Every pair a
kind cannot apply to is pinned and counted.**

**The finding, and the reason to have built this:** the ZIP reader never verified a checksum.
Escalated rather than settled, because it is a reader change with a cost — see 0.33.0.

## [0.31.1] — v2-S12.1: the guards that were never there

### Fixed

A CI job that did not build what it claimed to. **Verified by breaking it:** with the added line
removed, the test fails and names the reason. Forty-four sites now take a floor that had not moved
while the corpus tripled underneath it.

**Twenty-eight confirmed false statements were named rather than left to be rediscovered**, and one
correction to an earlier extrapolated rate is carried with its conditions.

## [0.31.0] — v2-S12: the office readers get fuzzed

### Added

A fuzz target on the single entry point all eight office readers share. **Not hypothetical:** an
earlier slice's adversarial review found a **panic** in a decoder.

**One target rather than eight, and the evidence that one is enough** — a measured campaign showed a
corpus seeded with one valid package of each shape reaches all eight readers. **Eight harnesses would
divide one corpus eight ways and explore each branch on a fraction of the budget.**

**The campaign ran 3,808,191 executions over about 57 minutes** under a memory sanitizer with debug
assertions, and **found nothing — which is a result rather than a pass.**

**No fuzz run is added to CI's required jobs**, and the measured budget is stated either way.

## [0.30.0] — v2-S11: embedded assets, counted

### Fixed

**No office asset was being read — that alone would be an omission. What made it a defect is that the
media entries were uncounted, in no bucket at all.** One reader's unread-part prefixes did not match
its media directory, so a document with forty embedded images declared **zero** unread parts for them.

**It was also unexercised, so step one was a fixture and a failing test.** None of the three OOXML
fixtures contained a single media entry — **the blindness could not have been caught by anything that
existed.**

**A second bucket, not a wider one.** The existing code's message says its parts *carry text*, and a
picture does not. **One number cannot honestly answer *how much* for two kinds of erasure.**

**A media part is identified by where the package puts it**, which the package specification itself
names — **never by sniffing bytes and never by an extension.**

## [0.29.1] — v2-S10.2: the docs that stopped describing the code

### Fixed

Eleven statements, found by a search rather than from a handed list — **and one of the ten handed was
wrong, which is the finding.**

**Two decisions, not typos.** The draft-schema worked examples are now regenerated under one rule for
both files: they had published **two different digests for the same representation of the same
document**, and at most one could ever have been right. And three harnesses now honour the fixture-root
environment variable, **which is a behaviour change.**

**Known and owed, not fixed here:** nothing guards those identity blocks.

## [0.29.0] — v2-S10: CSV, as an argued refusal

### Changed

**No reader. The format row closes on an argument.**

**The parse is not the problem. One field is.** Every field's text would be real bytes, every ordinal a
true line count, and an empty `pages` array simply true. **Exactly one thing would be false** — the
record would claim the file *is* a CSV when nobody measured that, and the identity structure has two
fields and **no room to say "asserted".**

**The naive reason is the wrong one and is recorded so it is not re-derived.** It is not that a CSV
parse would produce nonsense on prose. It is that **the engine could never tell a wrong assertion from
a right one, so fail-closed is unreachable from inside that design.**

**What shipped is a fallthrough refusal, not a CSV detector.** A `.csv` is no longer told it lacks a
PDF header, and a `.csv` and a letter containing a shopping list get **byte-identical stderr** — **the
test only an implementation that sniffed nothing can pass.**

**The reopening preconditions are written down**, so a later slice inherits a decision rather than a
mood. Neither is met, neither is a day's work, and neither was started.

*(v2-S10.1 repaired seven claims two earlier slices left false, with no version. It recorded itself
nowhere — its own finding, arriving about itself.)*

## [0.28.1] — v2-S9.1: the erasure counters that could wrap

### Fixed

Counters that could overflow, repaired across every site — **and the lesson that a site list is not a
search.**

## [0.28.0] — v2-S9: EPUB

### Added

An EPUB block binds, and `pages` is still `[]`.

**The first time the no-pages law had to be argued rather than applied.** A navigation document may
carry a page list naming **the pages of a print edition** — real identifiers the file writes down. It
is refused because **a page record is a page with a width and a height**, and a publisher's label
about somebody else's paper has no geometry to validate against.

**Reading order is the spine, and the archive is the trap.** The fixture proves it rather than
asserting it, by storing the second spine document first.

**No style sheet is read at all**, which is the general case behind three named gaps — including a
line break between two CJK characters, **the widest gap this slice knowingly leaves.**

**Detection is exact rather than prefixed.** The container rule is shared with the OpenDocument family
and the family question is asked separately.

**An adversarial review ran before this shipped and six findings were real**, including a vacuous test
that compared two compile-time constants.

## [0.27.0] — v2-S8: RTF

### Added

An RTF paragraph binds, and `pages` is still `[]`.

**The first format here that is not a package.** No parts, no manifest, no name for itself. **The
invariant grew a fourth rule instead** — a constant part name would have let the existing rule run
unchanged **and would have been a string the document does not contain.**

**The page, said out loud** — a page break and a paper width in the plainest language any of these
formats use. Both are print arithmetic.

**A byte escape above 0x7F is declared, never guessed.** Its meaning depends on a code page this
reader does not read. **Emitting a Latin-1 character would be mojibake presented as a success.**

**One defect worth recording, because it made the whole skip rule silently off.** The router's last
line also changed, and it is not about RTF: it became the *container* question.

## [0.26.0] — v2-S7: ODP

### Added

A block binds on the draw page, and `pages` is still `[]`.

**The sharpest refusal in v2.** A presentation lists draw pages — discrete, ordered, named, counted out
loud by anybody describing a deck — with a master's page width beside them. **A page record needed
nothing computed at all, which had never been true before.** Refused on the conversion rule's own three
words: *it invents pagination.*

**This is where the PPTX argument stops transferring:** ODP puts every draw page in **one** part, so
there is no part name to lean on.

**The shape set is named, and the consequence stated rather than hidden** — no corpus of real files was
available to measure which other elements matter.

**The frame rule, measured a third time, and the atom decides again.** The outcome matches ODT and the
reason matches neither.

**Speaker notes are the erasure rule inverted**, so they are counted rather than spliced: **splicing
would be a silent *extra*, which is worse than a silent drop because a consumer cannot tell it from
evidence.**

## [0.25.0] — v2-S6: ODS

### Added

A cell binds at the position the file states.

**The address this format does not write.** OpenDocument writes **no row number and no column letter
anywhere.** A repeat count is the file saying *"and n more of these"* — **a statement of position, so
reading it is reading.** A covered cell advances the cursor and yields no node.

**The column is a number here and a string in XLSX**, because each carries what its own file states.

**The frame-alternative rule looks different here, and that is the finding:** a frame floats *over* the
sheet, so its words belong to no cell, and **there is no address at which "one displayed phrase becomes
one node" could be true.**

The wrong-cause PDF message is fixed for this family.

## [0.24.0] — v2-S5: ODT

### Added

A paragraph binds, and **the one v2 format whose file contains a page break still declares no pages.**

A soft page break records where the *producing application's* layout fell, and it moves when the font
stack, paper size or producer changes. **It is read, recognised and discarded.**

**The atom is the paragraph, not the span** — a span is formatting, and addressing by it would make the
address depend on where the author changed a font.

**ODF's own whitespace rule is applied, and that is reading.**

**An adversarial review inverted the rule.** Character data was reaching a block through elements that
are not text — an image's alt text, an embedded object's base64 **spliced mid-sentence**, a heading's
generated number, a field's cached value, furigana. **So character data reaches a block only when every
element between them is an allowlisted inline one.**

Three defects came with it: **suffix matching let a conforming foreign element shift every later
address**; a mathematics annotation shares a local name with a comment, **so an inline formula was
declared as an unread reviewer's remark**; and a one-slot skip flag under-declared nested regions,
**which is the direction the erasure rule exists to prevent.**

**Four more were found before it shipped**, including a footnote concatenated into the sentence citing
it — **a sealed, error-free, byte-identical artifact stating a phrase the document does not contain.**

## [0.23.0] — v2-S4: PPTX

### Added

A slide's text binds, and **a slide is a part rather than a page** — which is the whole of this slice's
argument. **This is the first v2 format where inventing a page would not even feel like inventing
one.**

**The locator carries no slide number at all.** A caller that wants deck position reads the
presentation part, **where it is a fact about the presentation rather than a claim baked into every
citation.**

**The shape component was going to be the shape's own id, and measurement changed it.** Across 18 real
decks — 329 slides, 3,335 shapes — the id is present every time and **unique only most of the time.**
**Addressing by it would have given one address two answers on real files.**

**What a top-level-only reader would miss, measured:** 88.2% of text elements. Descending into groups
brings it to 91.2%, **and the remaining 8.8% is counted rather than dropped** — including a slide-number
field, whose **cached** value goes stale the moment the deck is reordered.

**Alternate-content branches: one phrase, one node — and the counters still advance through the rest**,
because counting only what was read would leave every later address one short. The same holds for a
self-closing paragraph element two common libraries write: **the address must not turn on how a deck
was serialized.**

**One rule moved to a module of its own**, because **three of the nine defects the previous slice's
review found were two copies of one rule disagreeing.** Both existing suites passed unaltered across
the move, **which is what made it a move rather than a rewrite.**

**Three more defects found before shipping**, all *a right address pointing at the wrong thing.*

## [0.22.0] — v2-S3: XLSX

### Added

A cell binds, and the artifact carries **two parts** — the first this engine has ever produced.

**Both the part and the sheet, because they answer different questions.** **The row is a number and the
column is a string, and the asymmetry is the file's** — turning a column letter into a number is
**arithmetic the file never performed and a value it never contains.**

**The part the shortcut would have skipped:** the workbook part contains no part names at all. The
sequential-filename shortcut **is wrong in ordinary files**, and every failure mode **attaches the
wrong sheet name to the right cells — a locator that is confidently wrong, which is strictly worse than
one that is absent.**

**The invariant did not have to change** — the part check was already a bijection, not a
cardinality-of-one rule.

**A formula is not a second authority:** no evaluator, and a field says whether the text is a stored
value, a cached result, or the formula's source.

**The coordinate declaration stays inert, deliberately** — measured: nothing acts on it for a page-less
artifact, and **a mode enum would have moved every PDF artifact's hash to respell a value nothing
reads.**

### Fixed

**Reviewing against the standing rules found eight defects before shipping, and fixing them surfaced a
ninth. Every one was a *silent* failure**, and three were in code whose own comment described the
hazard it had. Three of them produced **a sealed, byte-identical, error-free artifact with the wrong
text at the right address.**

## [0.21.0] — v2-S2: DOCX

### Added

The office crate exists, `extract` reads a `.docx`, and **`pages` is `[]` on the artifact.**

**The invariant now splits on the locator family** — the one thing a node cannot fake. A page-less
node's parent is a part id, `pages` must be empty, and **a measured box is refused outright, because
there is no page to check it against.** Integrity comes from a part-id-to-name **bijection**, so
nothing needs a second list.

**The locator carries a part name and two document-order positions.** No page, no box, no coordinates.
**If a field would have to be computed by laying the document out, it does not belong here.**

**A new attributes variant rather than the PDF one with fields blanked** — **a zero font size would be
three claims the document never made.**

**One new dependency, and the asymmetry is the whole argument.** ZIP is read in-crate; XML is
deliberately **not** hand-rolled, because entities, namespaces and encodings are exactly where a
hand-rolled reader silently gets **text** wrong, **and text is the evidence.**

**Measured, not feared:** the first version dropped an escaped ampersand silently. The fixture carries
one because of that.

## [0.20.0] — v2-S1: the grounding contract for a page-less source

### Changed

**A contract decision with no reader.** The grounding schema stays PDF-only in this repository —
revising it is a change to the **verifier's** contract, so that option was recorded as **blocked on a
revision owned elsewhere, not refused.**

**The finding: the page assumption is not where the scope document thought.** The seal refuses a node
whose parent is not a declared page, **so a page-less document cannot become a representation at all.**
Reaching the gate is upstream of grounding.

**One guard is about the dependency graph rather than the code:** no renderer in the lock file,
**because that refusal lapses as a transitive dependency before it lapses as a design decision.**

*(v2-S0 scoped v2 with no code and no version.)*

## [0.19.0] — v1.2-S5: the liteparse adapter, measured and REFUSED

### Changed

**No mapper, no subcommand, no foreign parser in the tree**, and the refusal is pinned by a test.

**Two walls, both in the grounding schema itself**, so no adapter could clear them per document. Their
output **cannot name its own producer**, and the schema leaves **nowhere to record that an identity was
asserted rather than measured** — **an identity that can be asserted is one that can disagree with what
it describes.** And their boxes are loose em boxes the schema cannot declare, **which would reproduce
that defect inside this repository's own artifact type.**

**What was *not* a wall is the more useful half:** the predicted coordinate blocker **dissolved**.

**No refusing subcommand**, because it would be a permanent public surface that does nothing, and
refusing *per document* would need a parser built on a guessed schema — **which would refuse real
output as malformed when the truth is that this engine guessed.**

**v1.2 is complete.**

## [0.18.0] — v1.2-S4: LangChain tools

### Added

Three tools per language, behind an optional extra and an optional peer, **so the default import still
reaches nothing but the standard library.**

**Locators travel in the artifact, never in the content.** Without the content-and-artifact response
format the artifact is stringified into the content, **and a box there is a locator a model can edit
and then cite.**

**MCP is the oracle for the summary strings, not this slice's opinion** — compared byte-for-byte, on
one fixture where nothing is omitted and one where everything is.

**The summary names the kind and never the id:** a kind is a category, an id is a handle, **and a handle
in the one channel a model can rewrite is the hazard itself.**

**No graph adapter and no trust state.** A failure **raises**, because an empty result would tell a
model its guess was merely unlucky.

### Fixed

**A false green.** Both SDK suites preferred a release build and took the first file that existed — on
a tree with a stale one that was a much older binary, **and every earlier assertion passed against
it**, because the byte-identity checks are self-consistent whichever binary they use. **They proved
what they claim, about the wrong engine.** Both now check the version.

## [0.17.0] — v1.2-S3: the Node SDK

### Added

The same three functions. **This is not a second design** — Python is the contract, and the differences
are exactly the two the language forces.

**No native addon, no TypeScript, no bundler, no test framework.** Runtime dependencies empty, asserted
from the manifest and by reading every import specifier.

**Canonical JSON ported a second time**, with two language-specific hazards handled: the default sort
compares UTF-16 units and disagrees with Rust above the BMP, and the runtime's encoder turns a lone
surrogate into a replacement character — **a silent repair of evidence**, so the port throws instead.

**One divergence is real and written down:** float-*shaped* text cannot be told from an integer here.
**The unreachable half is unreachable in practice**, because the CLI's stdout *is* canonical bytes.

## [0.16.0] — v1.2-S2: the Python SDK

### Added

Three functions over the CLI, standard library only.

**It wraps the CLI, and that is the whole design** — there is no second serialization anywhere, so
**byte-identity is a tautology rather than a promise.**

**A native extension was refused:** it would reach the library by a **second path**, which is a second
thing that can disagree with the first.

**The node lookup's checks are ported**, running the Rust implementation's own parity vectors. **A
fingerprint that were merely *nearly* the engine's would be worse than none.** The load-bearing proof
is the whole artifact reproducing the CLI's bytes, **not the five hand-written vectors.**

**`ground` deliberately does not repeat the fingerprint check** — a second check is a second thing that
could drift from the first.

## [0.15.0] — v1.2-S1: MCP over stdio

### Added

Three tools over newline-delimited JSON-RPC on a pipe. **No framework is vendored:** the ones available
pull an async runtime, **which would cost the network ban to save a few dozen lines.**

**The handle law, made mechanical.** The engine mints every locator, hands it back opaque, and
re-validates it — fingerprint first, then lookup among **that** artifact's nodes. **A forged id fails
closed: an empty answer tells a model its guess was unlucky; an error tells it the guess was not
admissible.**

**No tool argument carries geometry**, and a test reads the advertised schemas so the rule is enforced
**against the wire rather than against a reviewer's memory.**

**Statelessness is not a limitation here** — passing the artifact back **is** the session, so there is
no server-side table of documents whose keys a model could enumerate.

*(v1.2-S0 wrote the handle law down before the first host that could break it, with no code.)*

## [0.14.1] — v1.1-S4: HTML, under the same four laws

### Added

`ethos.html.v1`. **The Markdown rule id did not move**, so the Markdown a document produces is
byte-for-byte what the previous release emitted.

**Why this is a slice and not a stylesheet: GFM cannot say `rowspan`, and HTML can.**

**The two dropped erasure codes are dropped because HTML does not commit them, not because of their
names.** Three of six describe faults in the *record* and are kept. The span code is **recomputed**,
not dropped — a merge costs HTML nothing *unless the grid cannot hold it*.

**No header cell appears anywhere**, because a header row would be this exporter deciding what the
document meant. **GFM had no such choice, which is why the earlier slice owed a code for it and this
one does not.**

**Entities are emitted whole as `source`**, unlike the pipe escape: an entity **replaces** the
character, so the census is told it stands for **one** character rather than four. **Inverting a source
segment on this artifact therefore means HTML-unescaping it.**

**A fragment, deliberately.** **Embedding a fragment is one concatenation; unwrapping a document is a
parse.**

**One shared census function**, so the two artifacts **cannot** disagree about what a document
contains.

## [0.13.0] — v1.1-S3: the hyphenation join

### Added

A word the page broke across a line reads as one word in the Markdown. **The export joins; the
evidence record does not** — extract still emits both halves with the hyphen, **because this project
does not guess in the record.**

**Two clauses were found by measuring, and each was a real defect.** Without the baseline clause the
rule welded two fragments of *one line* — **the only place it fired on the entire benchmark corpus, and
it fired wrongly.** Without the furniture clause it welded a running head onto body text, producing **a
word on no page.** That also took away the per-run handle a consumer needs to drop furniture itself.

**So there is a quote that reads perfectly and does not ground.** The map holds one source segment
naming **both** runs, the removed hyphen sits in a named character bucket, and the census balances.

**A character bucket rather than a structural erasure**, and that is the whole test for which census a
disclosure belongs in.

**The golden needed a third fixture**, because the existing one's font declares no ink metrics, so the
verifier would find nothing and refuse every quote. **A model handed that Markdown would cite it
without hesitation, and the page never drew it.**

## [0.12.0] — v1.1-S2: GFM tables and lists

### Added

**A cell's runs are emitted once**, so the census balances and a table-free document comes out
byte-for-byte as before.

**The record had to carry the link, because a source segment must name a node.** A consumer holding
only the string can get back two ways and both are wrong — **re-run the geometry, which can drift from
the detector, or match the text, which is a guess the moment two cells hold the same word.** **The law
forced the field.**

**The erasures are a second census with integers**, because a dropped-character bucket for them would
read `0`. **The header claim is counted once per table, not once per cell** — **counting its cells would
make a wide table look like a worse lie than a narrow one when both told exactly one.**

**Trailing empties stay**, and **the escape is syntax while the character it escapes is source.**

**Lists come from the tree or not at all**, and the marker is always a dash: **a doubled marker is ugly;
deleting the document's own characters to make it pretty is the erasure this rule is about.**

### Removed

The blanket table limitation is **deleted, not reworded** — **a limitation that outlives the gap it
describes is worse than none, because a reader acts on it.**

## [0.11.0] — v1.1-S0/S1: Safe Markdown

### Added

`ethos.markdown.v1` — the string, the anchor map, and a character census, **as fields of one artifact.**
A companion file is a thing a pipeline strips; a field is not.

**The map tiles, and the tiling is checked on construction *and* on parse**, so a hand-edited file
cannot smuggle a hole past the type.

**Two segment kinds and only two.** No "probably source" — **that would be a confidence field wearing a
different hat.**

**Headings come from the tree or not at all.** Measured: **no fixture in either corpus carries a heading
role**, so the branch is proved by a unit test over a hand-built representation — **the honest way to
test a path the corpus cannot reach.**

**The verify golden is the whole point of the slice**, and step five is what means something: a string
that is **real text in the Markdown that the document never drew.** A consumer quoting the `.md` alone
cannot tell it from a sentence the page contains; with the map it is mechanical.

## [0.10.0] — v1-S8: stroke-ruled tables

### Added

The third detection rule, taken out of the attic and shipped.

**Both of the parked rule's failures were one defect: it never read the page's vertical ink**, so
"where are the columns" was decided by where horizontal rules happen to end. The coherence step moved
onto the lines, **which fixes both symptoms at once.**

**And the tax form stays silent on a principle, not a special case:** a face whose four edges are a form
field's four edges is that field's box. **No new constant.**

**Measured:** one document 246‰ → 259‰, macro 61‰ → 64‰, the tax form held at **0** tables, fabrication
**0**, cross-check disagreements **0**.

**It comes back one row short, and says so.** A shape match is unreachable while the rule against
invented coordinates holds — **the top edge is not in the file.** The old limitation is **retired and
replaced** by one stating the offset — the third code to hold that position, **each removed rather than
reworded.**

**What it still gets wrong, reported rather than tuned away:** 136 false-positive slots against 12,
mostly bands whose interior column lines *are* stroked. **A producer who does not tag a table is not
evidence that no table is there**, so they are kept and the cost reported — page precision 1000‰ →
928‰.

## [0.9.0] — v1-S7b: detector calibration, the gate at 61‰

### Changed

**The prescribed calibration was a no-op and was not taken.** The gutter floor read as a cliff is the
*first* sub-floor gap, not the smallest; disabling it entirely leaves the corpus **scoring
identically.** **A rule-version event that versions nothing is worse than none.**

**The candidate handed to the alignment rule is the whole page. The floor is not wrong; it is
unreachable.**

**Segmentation and marked-content grouping were both built, measured and reverted.** Segmentation
emitted 141 tables against 10 and got 72 of 1,302 cells right; it **routes the gutter floor around
itself**, removing a fabrication guard. Grouping alone **changed every number by nothing**; combined
with bands it produced the project's first non-zero fabrication count.

**What all five repairs missed:** **the alignment rule emits zero tables on the entire corpus.** Five
slices went into a rule the gate never exercised.

### Fixed

**Asking the other question found a real defect.** The ruled rule required every face to be covered by
*some* painted rectangle — and a page-background panel answers yes for all of them at once, while the
same rule separately discarded that panel as the table's own border. **One rectangle cannot be both the
only evidence a face exists and not a cell.**

Detection precision 900‰ → **1000‰**, cross-check disagreements 2 → **0**, the gate 43‰ → **61‰**, **no
true positive lost.** The ruled rule also gained a way to declare a refusal at all, **which it had
never had.**

*(v1-S7a built the labelled set and the harness with no version bump, deliberately. It changed no
detector and improved no number: **it built the instrument, pointed it at the corpus, and reported what
it saw. The number it reported was bad.**)*

## [0.8.2] — v1-S6.2: a run that draws no ink has no ink box

### Fixed

**Two real NIST documents produced no artifact at all**, exiting 2 on the box-within-page check — on
491 of one document's 492 pages. **Two of the three real benchmark documents were unreadable, and had
been since the check was written.**

**Every offending box is whitespace**, measured across all of them rather than sampled — spaces at a
one-point font size past the right edge, a producer idiom. **No run with visible text is out of place
anywhere.** The transform was never wrong; the seal was refusing two documents over rectangles drawn
around nothing.

**The contract had named the gap and left it unfilled.** Typed absence exists so that *could not
measure*, *nothing to measure* and *not asked to measure* are three answers — and there was no variant
for the second. Added, and deliberately not counted toward the ink limitation: **the reader could
measure; there was nothing there.**

**The hard refusal stays.** It caught the previous slice's crop-box regression, **and softening it would
have let that ship silently.**

**Blast radius:** 150,425 nodes lose a meaningless box; **zero** conformance fixtures change. **A
citation anchored to a rectangle around three spaces was never evidence.**

## [0.8.1] — v1-S6.1: code width comes from the font, not its decoder

### Fixed

A **simple** font declaring a two-byte Unicode codespace had its single-byte codes read two at a time.
The specification is unambiguous, and **the doc comment one line above already said so; the code did
not do it.**

**Measured, not estimated:** 2,426 mis-split fonts in one document, and **8,417 text runs omitted** from
another's artifact — with what survived visibly damaged.

**Why six slices passed green over it:** every conformance fixture uses a font with no Unicode map.
**The shape that breaks appears in zero owned fixtures and in every real document.**

**The dishonesty is the worse half.** The lost runs were declared as a *broken font encoding on the
document* — **a false statement about a conformant document.** **A declaration that misattributes is
worse than no declaration**, and this one would have sent someone to fix a document with nothing wrong
with it.

**Composite fonts are declared rather than left to be discovered**, being correct for the encoding real
documents overwhelmingly use and unverified for anything else.

## [0.8.0] — v1-S6: images, findings, and the overlay

### Added

- **Findings are observations, and the run stays** — flagged, in reading order, with text and origin
  intact. **A competitor deletes low-contrast text and returns a page that looks clean.**
- **The render mode was already tracked and read by nothing**, so the defect was **silent mixing, not
  data loss.** What was missing is that anyone could tell.
- **An image node is a placement and a digest, never a picture and never a caption.** Hashed **as
  stored**, so the fingerprint cannot depend on this engine's decompressor. **An artifact is a record
  about a document, not a second copy of it.**
- **The painted rect is the matrix, not the pixel count** — **a 4000×3000 photograph scaled into a 2 cm
  thumbnail is 2 cm of page.**
- **A third kind of box, kept apart from the other two.** A rotated placement is typed not-axis-aligned
  rather than given a bounding box, **which would claim page area the picture does not cover.**
- **The overlay marks and never edits**, with a per-page note counting what has **no** rectangle —
  **the criterion is that absence is visible, not just presence.**

### Fixed

**The page transform discarded the box's origin**, so every coordinate on a page whose media box does
not start at the origin was shifted. Not one document in either corpus has such a box — measured across
all 67 available — **which is why it survived six slices.** It had to be fixed before an off-page
finding could be honest: **a fabricated security finding is worse than none.**

### Not shipped

**Page rasters, by decision.** No renderer clears the dependency policy, and shelling out would put an
unpinned binary between the document and the artifact. **The setting records the not-emitted state
anyway**, so a renderer arriving later is a hash event rather than a silent change of meaning.

## [0.7.0] — v1-S5: multi-column reading order

### Added

`gutter-columns-v1`. **A new id, not a bump** — **bumping in place is the one move that makes two
artifacts look comparable while their orders disagree.**

**The evidence is whitespace, and only whitespace.** Nothing counts lines, runs or characters. **The
guard is vertical overlap, not a width**, which separates two columns from a heading above an indented
list. Its cost is stated rather than hidden.

**Separate constants from the table rule, even where the number is equal** — **a tuning pass on table
detection must not silently reorder every multi-column document.**

**One order.** The run array *is* the reading order; **a second sequence would reproduce the defect it
was meant to fix.** **Tables are atoms**, so a cut cannot shred a grid into fake columns.

### Fixed

**A cut leaked its sort**, so an uncut block came back ordered by baseline — **which on a real
two-column booklet turned pages that were already column-major into line-by-line row-major reading.**
Measured, fixed and pinned.

**Classify is untouched.** **A reading-order rule is not a page-complexity detector.**

## [0.6.0] — v1-S4: forms and annotations

### Added

Widgets and annotations as typed, distinguishable nodes. **Annotation text is never page text.**

**Measured before writing anything:** extraction reads page content streams and nothing else, so those
strings were **never** reaching a text run. **The defect did not exist here and this slice is purely
additive** — worth recording, because the opposite finding would have made it a repair.

**Walked from the page, not from the form.** A field names no page; its *widget* does. That gives every
node a real parent, one node per widget rather than a field plus a clone, and **makes an orphan
detectable as a field no page walk reached.**

**Two new node kinds, and the rule that permitted them.** Earlier slices refused kinds for facts an
existing node already carried — and **a field's value and an annotation's comment are carried by
nothing**, because no content stream draws them.

**A new locator variant, not a fabricated origin.** The declared rectangle is a different type from
measured geometry, because **measured means ink and a declared rect is a number the author wrote.**

**Flag bits this profile has no name for are kept as raw bit positions**, because a flag nobody named is
still something the document said. **A hidden annotation is flagged and kept** — honouring a rendering
instruction by deleting content is an undeclared edit.

**The tax form measured:** 199 widgets, **0 tables still**, and its 126 text fields report absent rather
than empty string — **a blank form is not a form filled in with nothing.**

## [0.5.0] — v1-S3: tagged-PDF structure trees

### Added

`struct-tree-v1`, binding a run **only on exact `(page, id)` equality** — nothing fuzzy.

**Four locator states, because they are four different facts**, and collapsing any two loses something
real. **Artifact runs stay in the node list, flagged** — a reader that deletes running heads has
silently edited the document, **and the edit is undetectable downstream.**

**A second cross-check under its own new id**, not the existing one widened: **one id meaning both would
leave a reader unable to tell which pair of derivations disagreed.**

**Three things this slice deliberately does not do:** no reordering (with a guard test asserting the
module contains no sort); no new node kinds; and **no using tags to fix a detector** — **a tagged grid
does not rescue an untagged one.**

**Absence is named on both sides and invented on neither**, through four conditional limitations —
including one for content this profile does not resolve, because **an unread id is not an absent one.**

## [0.4.0] — v1-S2: unruled tables

### Added

`unruled-align-v1`, **a separate rule id rather than a version bump**: *the document drew this grid* and
*a detector inferred it* are claims of very different strength, **and one id could not tell them
apart.** Every table now says which rule found it.

**The detection setting became a structure**, because a single string could not distinguish "looked and
found none" from "never looked".

**Coherence is the alignment analogue of the ruled rule's coverage precondition.** Measured on the tax
form: its text implies **23,276 faces** on one page. It still yields **0 tables**, and now says it
looked and refused.

**Fold, do not grow.** Growing until a gutter appears *chains* — measured, two lines of word-split prose
came out as a 2×3 table. **The cost is the declared price of not fabricating.**

**Emission order is evidence, not a threshold.** **Every alternative discriminator is a number tuned
until the fixtures fall the right side of it.**

### Removed

The blanket limitation, **deleted rather than reworded**, replaced by one profile-scoped code and one
**conditional** document-scoped one that appears only where a candidate was actually built and refused.

## [0.3.0] — v1-S1: ruled tables, cell slots, and the cross-check

### Added

Axis-aligned path capture, the cell-position and occupancy model, a ruled-grid detector, and the
geometric-versus-structural cross-check.

**Table and cell node kinds were considered and deliberately not added** — a cell's text is a
concatenation of runs that are *already* nodes, **so emitting cells as nodes too would put the same text
in two places.**

**Two findings, both measured rather than assumed.** The conformance fixture named for a grid contains
**no path operators at all**, so it was an S2 fixture wearing an S1 name. And a first version of the
detector **fabricated a 662-cell table** on a tax form: **building one lattice from every rectangle on a
page turns any document that *contains* boxes into a grid.** The fix is the coherence precondition.

**Overlaps are deliberately not excluded by it:** an overlap is a real disagreement and belongs in the
cross-check where it is reported, **not in a precondition where it would be silently dropped.**

**Looked-but-none is a real answer** — an empty array with the key present, not a fabricated one-cell
table around the page.

## [0.2.0] — v0.1: verify, encoding, and the xref decision

### Added

- **`verify` — invoking a verifier, still not verifying.** Report bytes are relayed **verbatim**, and
  the engine has no type for a report, a claim or a result, **so there is nothing that could re-derive
  one.** A missing verifier exits 2 with **no report**: never a skip, never a stub, never a
  default-pass. The verifier is pinned by version **and binary digest**, so a rebuild is as visible as
  a version change.
- **Encoding-issue detection.** A font that cannot map a code drops its run and declares the gap with a
  count, **instead of failing the whole document** — and never emits a substitution character. A
  document that decodes *nothing* is still refused outright.
- **The xref decision, written down.** One bounded, declared repair, under five published preconditions
  chosen so that **nothing an offset points at can move.** General recovery was rejected: it makes "the
  engine read it" stop implying "the document said it", **and the whole artifact contract rests on that
  implication.**

The oracle partition moved from 11/4 to **12/3**, in the open.

### Fixed

Two guards that were passing for the wrong reason.

## [0.1.0] — v0, frozen

**Not tagged.** The freeze is in-tree.

### M7 — CLI and library freeze, exit criteria as CI jobs

**No new capability.** M7 makes the exit criteria checkable and the public surface a decision.

- **`--diagnostics`**: opt-in, stderr-only, off by default. **stdout is byte-identical with the flag
  and without.** Not an artifact, deliberately — **an object that looked like an artifact would
  eventually be consumed like one, and then a timing would be inside somebody's hash.** A test walks
  every key of every artifact at every depth, because checking that two runs match would have passed
  for a host field on a machine where the host never changes.
- **The public API is now a list**, and a test fails if the crate roots and the document disagree.
  Narrowing the PDF crate's machinery surfaced **five dead items the compiler could not previously
  see**, two of them genuinely unread state.
- **Fuzz and mutation layers.** One triage finding: a mutation that "applied" to two benchmark
  documents by matching bytes inside a compressed stream, **which would have gone green while proving
  nothing.**
- **One fail-closed hardening**, found by that triage: the content interpreter kept text shown before
  an unrecognised operator. **No artifact was ever wrong** — but the guarantee belonged to the call
  site rather than to the type, **and the test meant to cover it passed for the wrong reason.**

### M6 — the validator, and agreement with the oracle

`grounding-check` validating **structure and source binding only**, agreeing byte-identically with the
verifier across all 15 fixtures on structure, source binding, representation hash and counts.

**The scope line said "JSON Schema validation only" and that was corrected here:** the schema is
necessary and not sufficient, and **a schema-only checker would disagree with the oracle it is required
to match.**

**Double-run byte identity** over the whole path, producing identical **files** rather than merely
identical payloads. **Oracle absence is loud** — never a skip, never a stub.

### M5 — the representation, and the grounding projection

`DocumentRepresentation v0` and the grounding adapter.

**Geometry-absent nodes are omitted from the grounding artifact and counted**, with the representation
still holding the node and its locator. **No node is ever emitted with a fabricated box.**

**Omission is only ever for missing measurable geometry, and this is structural rather than reviewed:**
the call site takes **a typed absence, not a boolean**, so it cannot be reached from a quality
judgement.

**A zero-area box is a hard error in the engine** — stricter than the oracle. **Stricter-on-emission is
the safe direction and the only one permitted.**

**Lossiness is asserted, not assumed**, so nobody later mistakes a grounding round trip for proof the
representation is intact.

**One engine-authored CC0 fixture** with unusable font metrics, because the upstream corpus has none.

### M4 — capabilities, typed absence, the L1 gate

**An artifact that does not declare its capabilities has not reached "extracted", regardless of how good
its text is.**

**Every declared capability has a passing test** — a capability asserted true with no test is a build
failure. **Partial processing is terminal and visible.** **Absence is never `1.0`.** And
**capability-limited beats negative**: absence of extractable content is never evidence of absence in
the source.

### M3 — extract: runs, locators, fail-closed operators

Content-stream interpretation with the operator set **enumerated explicitly**, a native locator on every
run, measured ink boxes or typed absence, synthesized flags, and glyph codes with the ligature caveat.

**Both quote-form show-text operators are handled** — the disqualifying defect in a surveyed parser,
where the text vanishes silently and surrounding runs merge with corrupt geometry. **An unknown operator
fails closed by name.** **Horizontal scaling is applied**, which that parser does not implement anywhere.
**No box is derived from a font size.**

### M2 — classify: reason codes, two axes, three exit codes

**The caller owns the policy; the engine owns the observation.**

**Bounded cost is the load-bearing test:** a 492-page document scans **8** pages, and classify time is
flat in page count — 246× the pages for 1.7× the time. **The two phases are timed separately**, because
the backend parses the whole object graph eagerly.

**Three exit codes, one fixture each, all distinguishable** — including a missing file.

**The simplest fixture's behaviour is recorded, not tuned to match anyone.** One competitor calls it
text-based with zero text pages; another demands OCR. **Neither is the target.**

**The reasons with no sound detector are never emitted, and the artifact says so** — silence would let a
caller read an empty list as evidence of absence.

### M1 — contract types, canonical JSON, quanta

**After M1 the artifact shape stopped being negotiable and started being a compile error.**

**Integers only**; any non-integer anywhere is a hard error. **Keys sorted by code point at write time**,
tested with the map-ordering feature forced on — **and the test first proves the feature is active, so
it cannot pass vacuously.** **Profile sensitivity asserted field by field**, and the default profile
pinned by bytes and digest: **the first proves a change is detectable, the second proves it was
intended.**

**Every nested object in a hashed type denies unknown fields** — the derive does not recurse, **and
guarding only the outer struct leaves a dropped nested knob re-hashing to the unmodified digest.**

### M0 — skeleton, toolchain, dependency policy, a failing harness

**The first commit contained the oracle test and one fixture, failing.**

The gate was two conditions, because a bare test run exited non-zero **by design**. **Resolving it by
making the bare command green was forbidden** — every mechanical route destroys the milestone, and the
test's own diagnostic said so.

**The AGPL probe fails with exit code 4 naming the crate**, proving the gate fired for *that* reason —
**any non-zero exit would also match a config typo.** The oracle harness **errors loudly if the verifier
is absent**, and its environment variable is authoritative rather than a hint: **resolving silently to a
verifier nobody chose is worse than finding none, because the operator believes they know which one
answered.**
