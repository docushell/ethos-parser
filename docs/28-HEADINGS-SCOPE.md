# 28 — Inferred headings scope: a heading the type says is one, where the document says nothing

**Scope authority for decision #29.** North Star row 29 of 2026-09-17 reversed the checklist refusal
**L29** and narrowed **P14** for headings alone: *a heading may be inferred from font size or font
name in a document that declares no structure*, as `Computed`. This document settles what #29 left
to a scope document. [`OPEN-WORK.md`](OPEN-WORK.md) §5 asks for it in these words: *"A scope
first — rule id, where the inference is declared, how Markdown and HTML emit it — then the rule,
measured as an MHS band with the worst document named and a false-positive check."*

**§10 is the slice authority, and no separate milestones document is written.** The slices below are
short and their reasoning is the scope's, where auto-tagging's were long enough to need
[`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md) and `locate`'s needed
[`27-LOCATE-MILESTONES.md`](27-LOCATE-MILESTONES.md). Said rather than left as an omission.

**Written 2026-09-18 against `main` at `a18b2b0`, workspace 0.58.0, unreleased.** Every number in it
is measured on the verified 0.58.0 `aarch64-apple-darwin` release binary
(`target/aarch64-apple-darwin/release/ethos-parser`, the binary
[`measurements/opendataloader-bench/README.md`](measurements/opendataloader-bench/README.md) pins),
over the eight gate PDFs and the 200 `opendataloader-bench` documents, unless it names another
source. **The corpus census of §7.1.1 was measured the same day by the parent session with the build
at `a18b2b0`** and is attributed there. **Three numbers this document needs do not exist yet and it
says so by name** rather than estimating them (§7.4).

**One caveat rides on every measurement below and is not a footnote.** The signal the shipped rule
reads — the *rendered* em height — is not on the wire today, so every number in §7.2 and §7.3 was
taken on the **ink-box height** as a proxy for it. Within one font the proxy is exact; across fonts
it carries the font's ascent-to-descent envelope, and §7.3 measures what that costs. **The numbers
below bound the proxy, not the rule.** They say the acceptance bar of §7.5 is reachable. They do not
say the rule meets it, and nothing here claims otherwise.

---

## 1. The one sentence

**Where a document declares no structure, a line whose type is larger than the document's own body
type is a heading, and every surface that says so says the engine measured it.**

Everything below is machinery for the second half, plus the measurement that decides whether the
first half is honest enough to ship.

## 2. Row 29, read closely

| Rider | What it means here | Where it is held |
| --- | --- | --- |
| **`Computed`, under a rule id in the profile** | A new profile field `heading_inference_rule`, default `type-size-v1`; `profile_sha256` moves (§4) | `profile.rs`, the pin test |
| **Every surface that emits one says it was inferred, wherever that surface can say so** | The representation: a field whose only meaning is inference, plus the document-scoped `headings-inferred-from-type`. The projections: the rule id in the artifact that carries the string. The raw `.md` byte: **it cannot say, and §5.3 says why rather than inventing a way** | `limitations.rs`, `markdown_rule`, `html_rule` |
| **A declared heading always wins, and inference runs only where the document declares no structure at all** | Two mechanisms, and the second is load-bearing: the reader never sets the field on a document that declares structure, so `heading_level`'s existing precedence never has to arbitrate (§6) | `structure.rs`, `markdown.rs::heading_level` |
| **P14 stands for every other role; D1 stays undecided; a region is never a heading; this row reads type, not position** | §7 of the refusals table. The rule reads neither `region` nor `block`, and a test bans both names from its source, the shape `reading_order.rs`'s own guard already takes | §8 |
| **Measured before it ships, as row 18 requires: MHS as a band with the worst document named, plus a false-positive check whose bound the scope sets before the code** | §7.5 sets the bound at **5% per document**, with the reasoning, before any code | §7, §9 |
| **What would re-refuse it: a measured false-positive rate above that bound** | §7.5, made numeric and per-document rather than pooled | §7.5 |

**What row 29 does not say.** It does not say the engine acquires a document outline: no hierarchy,
no numbering, no table of contents (§5.2, §8). It does not say the inference reaches any format but
PDF — and for every other format the type system already forbids it (§6.2). And it does not say a
heading may be *written into a file*: `tag` writes `/Div` and nothing else (docs/23 §3.2), and
crossing the two features is refused by name (§8).

## 3. What is inferred, and from what

### 3.1 The signal is the rendered em, and the field spelled `font_size` cannot carry it

**Decided: the rule reads `em_scale_pt` — the vertical scale of the text rendering matrix, `Tfs`
composed with the text matrix and the CTM per PDF 32000-1 §9.4.4** — which
[`content.rs`](../crates/ethos-parser-pdf/src/content.rs):119 already computes on every shown run
and [`extract.rs`](../crates/ethos-parser-pdf/src/extract.rs):434 already passes to `ink_box`. It is
the third side of the triangle the box already uses, it is font-independent, and it is the only
number in this engine that means *the height of one em as the page draws it*.

**Rejected: `TextRunAttributes.font_size`**
([`representation.rs`](../crates/ethos-parser-core/src/representation.rs):1503), which is the field
row 29's words most obviously name. It is the raw `Tf` operand:
[`content.rs`](../crates/ethos-parser-pdf/src/content.rs):105–110 says so in as many words — *"Not
the rendered size, and not what an ink box may be built from. A page may set `Tf /F 1` and carry the
type size in the text matrix or the CTM, in which case this is 1 and the glyphs are 10pt"* — and
[`fonts.rs`](../crates/ethos-parser-pdf/src/fonts.rs):388–396 records the defect that taught it:
a producer emitting `10 0 0 10 0 0 Tm` with `/F1 1 Tf` had every box measured at a tenth of its
size. `extract.rs`:485 quantizes the operand onto the wire unchanged.

**Measured 2026-09-18.** Distinct values of `font_size` per document, over the 200 bench documents:

| distinct values | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| documents | **87** | 28 | 41 | 21 | 10 | 7 | 4 | 1 | 1 |

83 of those 87 carry the single value **100**, the `Tf /F 1` idiom; 3 carry 996 and 1 carries 1091.
Of the **107** documents whose ground truth holds a heading — the only documents MHS scores — the
field has **one distinct value on 41** and more than one on 66. On four gate documents it has
exactly one value, 100, on every one of their non-blank runs: `irs-fw9` (974 runs),
`irs-f1040sd-2025` (963), `nist-sp-800-207` (85,709), `nist-sp-800-218` (57,722).

**So the field row 29 names carries no signal on 43.5% of the deployment corpus and none at all on
half the gate corpus.** That is the first conflict between the row's wording and the code, and §11
settles it on the measurement: *font size* is read as *the type size the document draws*, not as the
field currently spelled `font_size`, which carries no signal on 43.5% of the corpus.

**Rejected: the ink box's height**, which is on the wire and varies everywhere (0 of 200 bench
documents lack a measured box). It is the run's pen advance over its font's ascent-to-descent
envelope (contract §5.3), so `height = (ascent − descent)/1000 × em` and the envelope term is the
font's, not the type's. **§7.3 measures what that costs and it is the worst number in this
document**: a 29.37% false-positive rate on `nist-sp-800-218`, every one of them a second font at
ordinary body type. Two further grounds, each sufficient alone: geometry sits **outside the
fingerprint** deliberately (contract §4), so a projection deciding structure from it would be the
seam D4 §4 warns about pointed the wrong way; and **7.9%** of bench text nodes carry no box at all
(bench README, 8,696 of 109,500 at 0.58.0).

**Rejected in this slice: the font's own name and flags.** `/BaseFont` is read
([`fonts.rs`](../crates/ethos-parser-pdf/src/fonts.rs):675–685) only to build a limitation's detail
string, and `/FontDescriptor /Flags` is read at `fonts.rs`:664 only to answer symbolic-versus-
nonsymbolic for *encoding*. `Font` (`fonts.rs`:167–190) keeps neither the name nor a weight, and
neither reaches the wire: `TextRunAttributes.font_id` is the **resource** name the document chose
(`T1_0`, `TT1`, `F1`), which says nothing about weight or family. So the second signal row 29 names
is not on the wire either. §7.2 measures a cheap proxy for it — *a line set in a font the body text
is not set in* — and finds it is the **stronger** of the two signals on recall and the weaker by a
factor of three on false positives. It is scoped as a second slice gated on its own measurement
(§10 S5), and §11 settles that ordering on the false-positive band it costs.

**Rejected: a bold rendered by the content stream** — `Tr 2` stroke-and-fill fake bold, or a doubled
`Tj` at a small offset. Nothing reads it and reading it would be inferring weight from painting.

### 3.2 The baseline is the document's own char-weighted modal em

**Decided: the reference is the char-weighted mode of the rendered em over every non-blank,
non-artifact run of the document**, binned to 10 centipoints — the `BIN` constant
[`blocks.rs`](../crates/ethos-parser-pdf/src/blocks.rs):101 already uses.

**Why a statistic of the document and not a point size.**
[`19-BLOCK-SUBDIVISION-SCOPE.md`](19-BLOCK-SUBDIVISION-SCOPE.md) §4.2 is the finding this reuses:
*"each document sets its break at a near-constant pitch of its own"*, and a constant fitted to the
corpus mix cleaved it in half across a 68-point band. Type behaves the same way — a 14pt heading is
display type in a 10pt document and body type in a 17pt one — so the threshold is a multiple of a
number measured on the document under test, exactly as the leading-gap cut's is.

**Why char-weighted.** Body prose is most of a document's characters and a small fraction of its
lines; weighting by lines would let a title page, a running head or a table of small captions become
the reference. Weighting by characters is what `markdown.rs`'s own `pitch_reference`
([`markdown.rs`](../crates/ethos-parser-core/src/markdown.rs):967) already does for glyph width.

**Rejected: per page.** A page holding nothing but a part title would take its own heading as the
body and find no heading at all. The document-wide mode is the only reference with body text behind
it on every page.

**Rejected: per band, the analogue of the leading cut's normaliser.** A band is a column and a
heading commonly spans the gutter above two columns, so the heading would frequently sit in a band
of its own and be its own reference. Reopens if a corpus shows a document whose columns are set at
materially different type sizes.

**Rejected: the adaptive second mode**, which `19-BLOCK-SUBDIVISION-SCOPE.md` §11 tested for the
leading cut and which lost on real labels by 12.6 points. Nothing here re-opens that argument on new
evidence, and starting with the harder rule is what §11.4 there forbids.

### 3.3 The unit is the line, and the whole line must clear the threshold

**Decided: the unit is the line** — the runs sharing `(page, region, artifact, origin_y)`, which is
`markdown.rs`'s existing `LineKey` ([`markdown.rs`](../crates/ethos-parser-core/src/markdown.rs):939–957),
read for equality only and never for order, as that type's own rustdoc requires.

**Why the line.** It is the unit the projections already open a block on. Measured on
`bench/01030000000001`: the Markdown projection emits **60 blocks** for a page of 39 candidate
lines plus furniture — one block per line on untagged prose — so `heading_level` is already called
once per line and needs no new grouping to hang a `#` on.

**Rejected: the run.** A heading drawn as four `Tj`s would emit four `#`s, which is the
68,112-block defect of 0.44.0 (bench README, *"Every text run was its own block"*) in a new place.

**Rejected: the `block` of the leading-gap cut.** Two reasons, and the second is the row's own.
It is **absent wherever the rule declined**, which `TextRunAttributes.block`'s rustdoc
(`representation.rs`:1548–1556) calls *"ordinary rather than a corner"* — a page of uniform body
text has none — so it is not a unit that always exists. And row 29's rider says in as many words
that *this row reads type, not position*; reading a role off the cut is what D4 §11 rule 1 and
`19-BLOCK-SUBDIVISION-SCOPE.md` §6 refuse, and #29 narrowed neither.

**The whole line, not its tallest run.** A line fires only when **every** run of it that has a
measurable em clears the threshold. Measured cause: `bench/01030000000007` carries a one-character
line at **4.63×** the body box — a drop cap or a display glyph — which a maximum test makes a
heading and a minimum test does not.

### 3.4 The rule, in integers

A line is an inferred heading when all of these hold:

1. every run of the line with a measurable rendered em satisfies `5 · em ≥ 6 · body_em` — that is
   **1.20×**, written as integers because contract §4 admits no float, in the `CUT_NUM`/`CUT_DEN`
   shape of [`blocks.rs`](../crates/ethos-parser-pdf/src/blocks.rs):119–122;
2. at least one run of the line has a measurable rendered em;
3. the line's text is not whitespace-only;
4. the page did not mark the line an `/Artifact` (§14.8.2.2 — the author said it is furniture, and
   `markdown.rs::is_page_artifact`:812 already answers it);
5. no run of the line is owned by a table the detector accepted (`plan_tables`'s `owner` map,
   `markdown.rs`:1585–1592 — a cell is not a heading);
6. the document declares no structure (§6).

**Why 1.20× and not a measured optimum.** There is no measured optimum to take: §7.3's band moves
by 0.1 points between 1.15× and 1.25× on the proxy, and the one document that decides the bound
(`nist-sp-800-218`) is insensitive to the threshold because its errors come from the envelope rather
than from the multiplier. 1.20× is the round number inside the interval the evidence does not
distinguish, and **§7.5's bound, not this constant, is what the rule ships on**. Stating it this way
rather than quoting a fitted number is `19-BLOCK-SUBDIVISION-SCOPE.md` §8 rule 2: *no fixed
multiplier presented as a measurement*.

**No minimum line length**, and the alternative is recorded with its cost rather than shipped:
raising the minimum from 4 to 8 stripped characters moves the size clause's proxy precision 40.3% →
44.9% and its recall 55.6% → 54.2% (§7.2). It is a constant fitted to one corpus, which is §4.2's
error there committed deliberately, and clauses 4 and 5 already remove most short fragments. Reopens
if §7.3's band is missed and the length test is what closes it — in which case it is measured
per document and published, not tuned.

**No confidence, no near-miss, no score.** A line clears the threshold or it does not. Contract §9.

### 3.5 One level, and the corpus says a second would be unmeasurable

**Decided: an inferred heading is level 1 and nothing else.** The projections emit `#` and `<h1>`.
No hierarchy is claimed, and the artifact says so.

Two measured reasons, and either is sufficient.

**The metric cannot see a level.** The harness's own
`evaluator_heading_level.py` docstring says it *"builds a flat section tree that treats all heading
levels as equivalent"*, and the code is that: `_parse_markdown_structure` appends every heading to
`root.children` whatever its `#` depth and tags it the constant `"heading"`, and
`HeadingConfig.rename` compares `node1.tag != node2.tag`. **The level is never compared**, so a
claimed level buys exactly zero on MHS.

**The corpus's own labels claim no hierarchy.** All **193** ground-truth headings across the 107
documents are level 1; `##` through `######` occur **0** times. There is nothing in this corpus to
validate a level against, and a rule scored by nothing is a rule fitted to nothing.

A level would need a second rule — an ordering of the document's display sizes into six buckets —
with its own threshold, its own band and no corpus able to refuse it. YAGNI, and §11 records it as
reopening on a corpus whose ground truth uses more than one level.

## 4. The rule id, where it lives, and what it costs

**Decided: a new profile field, `heading_inference_rule`, default `type-size-v1`**, placed after
`struct_tree_rule` ([`profile.rs`](../crates/ethos-parser-core/src/profile.rs):1258) and before
`markdown_rule` (1269) — beside the field naming the rule that reads the document's own tree,
because this rule runs exactly where that one found nothing.

**Why its own field and not a fold into `struct_tree_rule`.** `struct_tree_rule` names the reading
of the document's own tree; this rule runs only where there is no tree. One id covering both would
make every *tagged* document's artifact non-comparable across a change to a rule that never ran on
it. That is the `html_rule`-versus-`markdown_rule` argument at `profile.rs`:1270–1276, applied
once more: separate when they can move independently, and these can.

**The rule-id fields that exist today**, read off `Profile` (`profile.rs`:1213–1310) so the new one
is placed against a full list rather than a guess: `reading_order_rule`, `table_detection` (a struct
of three), `struct_tree_rule`, `markdown_rule`, `html_rule`, `form_annotation_rule`,
`cmap_data_version`, `font_metrics_data_version`, `text_code_rule`, `observation_rule`. The new
field sits between the third and the fourth.

**Office profiles carry `NOT_RUN`**, as they already do for `struct_tree_rule`, `markdown_rule` and
`html_rule` (`profile.rs`:1406–1408) — `profile.rs`:815 is the constant and its rustdoc is the
argument: a declared state, never an empty string and never a PDF rule id borrowed for the shape.

**No new capability flag.** D4 §10 refused one on the ground that *"a second flag would be a flag no
consumer can act on independently"*. Here the rule id itself is the switch, exactly as
`READING_ORDER_RULE_V0` is the switch for the cut (`profile.rs`:1237–1241): a profile naming a rule
id that means *not run* has turned the inference off and says so in the one place a reader looks.
Contract §7.1 rule 1 is satisfied without a flag because there is no capability claim to make —
`structural_locators` already claims *this profile looks* at the document's own structure, and this
rule makes no claim about the document at all.

**What it costs, accepted with the decision.** `profile_sha256` moves, so **every golden
regenerates** — contract §2, *anything that can change a byte of output belongs in the profile, or
it is a bug*. `the_default_profile_is_pinned` (`profile.rs`:2214) is re-pinned from its own output
with a move-log paragraph, as every prior move; `every_profile_field_is_hash_sensitive`
(`profile.rs`:1907) destructures the new field, so omitting it fails to compile;
`docs/draft-schemas/profile.draft.json` follows.

**`markdown_rule` and `html_rule` move too**, to `markdown-blocks-v8` and `html-blocks-v8`, because
`heading_level` gains a source and both projections call it. That is not a new argument: `html.rs`
:121–124 records the v2.2-S0 precedent verbatim — *"`heading_level` gained its second source… Both
projection ids move together here and that is not a contradiction of them being separate."* The
family name `-blocks-` is kept rather than renamed to `-headings-`, because the table at
`markdown.rs`:126–138 numbers by slice and two of its rows already disagree with their values; a
second naming scheme would be a third way to read it.

## 5. Where the inference is declared on the wire

### 5.1 The representation: one field, absent where false, plus a document-scoped declaration

**Decided: the reader decides, and the representation carries the verdict.**
`TextRunAttributes` gains

```rust
/// Set where this profile's heading-inference rule read this run's type as a heading's, on a
/// document that declares no structure. `Computed`, never the author's — the profile's
/// `heading_inference_rule` names the rule, and the artifact declares
/// `headings-inferred-from-type` with the count and the body reference it measured.
#[serde(default, skip_serializing_if = "std::ops::Not::not")]
pub inferred_heading: bool,
```

**Why the reader and not the projections.** The signal is `em_scale_pt`, which exists only inside
the PDF crate's interpreter; nothing on the wire carries the rendered em (§3.1). A projection
inferring from the wire alone would have to read the box height, which §3.1 refuses on three
grounds. So the reader is where the rule can be honest, and the profile field naming the rule then
names a reader rule, which is what every other rule id there names.

**Why a verdict and not the measurement.** The alternative — a `rendered_em: Option<i64>` on every
run, with the inference in the projections — was considered and costs 12–18 bytes on **every** run
of **every** document, tagged or not, inside the fingerprint, on an engine whose run time is linear
in emitted bytes (D4 §6). The verdict costs nothing where it is false, which is the `region` and
`block` idiom (D4 §4: *"the common case costs zero bytes and zero time"*). It also puts the
inference where the limitation slot can declare it; a bare measurement would leave the artifact with
nothing to say.

**Why `default` here is safe where `derivation`'s was not**, said explicitly because decision #20
turns on the opposite case. `PdfTaggedLocator.derivation` carries no default because an intermediary
stripping the key would launder a `Computed` address into `Extracted` — a default to the
**highest**-trust value. Here the default is `false`: an intermediary stripping the key turns an
inferred heading into no heading, which is the conservative direction and unrecoverable only in the
sense that nothing was claimed. §11 settles this against #20's spell-it-out rule, on the ground that
nonetheless put a `DerivationClass` on this field as #25 put one on the locator.

**Why on the attributes and not on `Node`.** D4 §4's correction, reused without change: `attributes`
is *"facts only this node's kind has"*, only runs have type, and a form field, an annotation and an
image would carry a field that is structurally always absent. It is also what keeps every office
format out by construction — their nodes carry `OfficeRun`, `OfficeCell`, `OfficeParagraph` and the
rest, none of which has this field, so **they cannot acquire an inferred heading by accident**.

**Rejected: a new `StructuralLocator` variant**, which is where roles live and would have reused
`heading_level`'s existing match. It is refused on measurement. `structural_locator` is a single
`Option`, and on the population this feature exists for the slot is **occupied**: docs/23's
amendment records that 146 of the 200 bench documents are PyPDF2 page splits that *"kept their
marked-content ids and lost the tree"*, so their runs carry `PdfMcid`. Minting a variant would mean
replacing an `Extracted` marked-content id with a `Computed` role, which contract §6 rule 1 forbids
outright.

**Rejected: reusing `PdfTagged` with a `Computed` derivation and a synthetic role path.**
`PdfTaggedLocator.mcid` is required and there is no id to put in it; inventing one is contract §10's
fabrication.

**The declaration: a new code, `headings-inferred-from-type`**, document-scoped, in
`ethos_parser_core::assurance::codes` and built in
[`limitations.rs`](../crates/ethos-parser-pdf/src/limitations.rs) beside
`structure_tree_engine_written` (`limitations.rs`:546). Its detail states the count of lines the
rule fired on, the rule id, the body reference in centipoints that the rule measured on this
document, and the fact it exists to state: **the document declared no structure, so every heading in
this artifact is this engine's measurement of type and none is the author's.** It is a disclosure in
the limitation slot on the precedent `structure-tree-engine-written` set and
`tagged-table-without-geometric-table` set before it — and like both it still names something
missing: the author's structure.

`untagged-structure-tree-absent` (`assurance.rs`:349) stays declared on the same document, and is
the precondition rather than a duplicate: it says the catalog declares no `/StructTreeRoot`, which
remains true.

### 5.2 The Markdown projection

`heading_level` ([`markdown.rs`](../crates/ethos-parser-core/src/markdown.rs):639) gains a third
arm after its two declared sources:

```rust
if text_run_attributes(node).is_some_and(|a| a.inferred_heading) {
    return Some(1);
}
```

The `#` is emitted at block open, as `syntax`, by the code already at `markdown.rs`:1850–1853 —
unchanged, because the marker for a declared heading and the marker for an inferred one are the same
bytes.

**Its precedence is structural, not conventional.** The two declared arms return first, and the
reader never sets the field on a document that declares structure (§6), so the two sources can never
both be present. Either mechanism alone would satisfy row 29's rider; both are in place because a
projection-level precedence is a convention a later edit can reorder, and a reader-level gate is
not.

Rule 2 of `to_markdown`'s rustdoc (`markdown.rs`:1536) changes from *"A heading when the tree says
so"* to name both sources. `html.rs`:307 changes likewise.

### 5.3 What a consumer of the `.md` alone learns, which is nothing, and why that stays true

**A `#` carries no derivation, and this projection has no honest place to put one.** Stated plainly
because the alternative is to invent a mechanism.

**What the artifact says.** `MarkdownArtifact.markdown_rule` is `markdown-blocks-v8`, and that id's
own documentation is the statement that this projection may infer a heading from type. A consumer
holding the artifact reads it in one field, beside `representation_sha256`, which reaches the
representation where §5.1's field and limitation are. That is the whole disclosure and it is
document-scoped.

**What the anchor map carries.** The `#` and the space after it are one `Syntax` segment with no
`node_ids`, byte-identical in shape to a declared heading's marker. A quote touching it is not
invertible, which is already the consumer's rule (`markdown.rs`:84).

**What the bare string says: nothing.** And that is not new — the string says nothing today about a
hyphen rejoin, about a blank line the exporter invented, or about a GFM separator row that claims a
header the document never wrote. The `.md` alone has never been the evidence; the artifact is.

Three mechanisms were considered for saying more, and each is refused by name:

| Mechanism | Answer | Why |
| --- | --- | --- |
| A third `SegmentKind` — `inferred`, or `source-ish` | refused | `markdown.rs`:245–247 refuses a third value in as many words: it *"would be the confidence field `docs/01-CONTRACT.md` §9 forbids, and it would put the consumer back in the position this whole version exists to remove"* |
| `node_ids` on the `#` syntax segment, naming the run whose type produced it | refused | `Segment.node_ids` is documented as *"Present on `SegmentKind::Source` and absent on `SegmentKind::Syntax`, because syntax came from no node"* (`markdown.rs`:280–286). A syntax segment naming a node is the one shape a consumer inverting a quote must never see |
| A `StructuralErasure` code in `Coverage` | refused | That census is *"one named class of **structure** the projection could not carry"* (`markdown.rs`:445–455). An inferred heading is structure the projection **added**; filing it there would put it under a heading meaning its opposite |

**Considered and not proposed: a count on `MarkdownArtifact`.** It would let a consumer holding only
the Markdown artifact learn *how many* headings were inferred. The representation's limitation
already carries the count and `representation_sha256` reaches it, so the field would be a second
place for one number to live — and it is a new key on a frozen artifact shape, which moves
`MARKDOWN_SCHEMA_VERSION`. YAGNI. Reopens on a named consumer that holds the Markdown artifact and
cannot reach the representation.

### 5.4 The HTML projection

`<h1>` where `heading_level` returns 1, by the code already at
[`html.rs`](../crates/ethos-parser-core/src/html.rs):555–560. No attribute, no comment, no
`data-inferred`: the fragment has no framing of its own by design (`html.rs`:82–87), and a byte a
consumer might read as content is worse than a byte that says nothing. The disclosure is `html_rule`
in the same artifact, exactly as §5.3.

## 6. Coexistence with declared headings

### 6.1 PDF: the test is `structure::read` returning `Ok(None)`

**The exact predicate: `structure::read(doc)` returns `Ok(None)`**, which happens when the catalog
declares no `/StructTreeRoot` ([`structure.rs`](../crates/ethos-parser-pdf/src/structure.rs):269–271).
That is the same predicate that declares `untagged-structure-tree-absent`, so the gate and the
declaration can never disagree about one document.

**Not "the tree reaches no run".** A tree that cites nothing is still a declaration, and docs/23
§3.6 row 1 refuses a partial fill on exactly this ground: filling around an author's elements
*"invents a relation the author did not state"*. A document with a tree and unbound runs already
declares `structure-mcid-unbound` with a count, which is the honest answer and stays it.

**Not `/MarkInfo`.** `structure.rs` never reads it and a test pins that (`structure.rs`:62); a
document claiming Tagged PDF conformance with no tree declares nothing this rule must yield to, and
one with a tree and no `/MarkInfo` declares plenty.

**A document this engine tagged: the inference runs, and the reasoning is already in the tree.**
After `tag`, a document has a `/StructTreeRoot` whose every binding reads back `Computed`
(docs/23 §4.1) and which declares `structure-tree-engine-written`. Read literally, row 29's *"the
document declares no structure at all"* would stop the inference there — and that would break
something already tested: docs/23 §4.3 requires the Markdown, HTML and grounding of an engine-tagged
document to equal the untagged original's byte for byte, held by
`the_projections_of_a_tagged_document_are_its_originals` (docs/24 S3).

**Decided: a tree whose every binding is `Computed` is no declaration**, so the inference runs on it
and the equality survives. This is not a new argument; it is `group_key`'s, verbatim
(`markdown.rs`:878–887): *"A sequence this engine wrote is no declaration… the producer of the
sequence is this engine, and the licence does not transfer."* The predicate is therefore
`structure::read` returning `Ok(None)` **or** a tree reporting `engine_written` and no
`Extracted` binding. §11 settles this reading of the rider, on the measured cost of the
one, because the alternative is a failing test rather than a smaller feature.

### 6.2 Every other format: declared, or without an input, and out of scope either way

| Format | What it declares | Why the inference does not run |
| --- | --- | --- |
| **EPUB** | The XHTML element name, verbatim, on **every** block — `EpubBlockAttributes.element` (`representation.rs`:1880–1895). `<h1>`..`<h6>` project as headings today | Every EPUB declares structure, so the gate closes. And `EpubBlockAttributes` carries no type size, so there is no input |
| **ODT, ODP** | The **fact** of a heading and not its level: `OdfBlockKind::{Paragraph, Heading}` (`representation.rs`:1648–1653), from `<text:h>` versus `<text:p>`. `text:outline-level` is deliberately unread (`representation.rs`:1629–1632) | Declares structure, so the gate closes. **And a live gap, named in §11:** a declared heading that projects as a paragraph, because `heading_level` reads only EPUB and PDF (`markdown.rs`:629–634) |
| **DOCX** | Nothing the engine can see — the reader keeps no `<w:pStyle>` (`markdown.rs`:636–638) | No input: `OfficeRunAttributes` has one field and it is not a type size |
| **XLSX, ODS, PPTX, RTF** | Nothing about headings | No input, same reason |

**The first slice is PDF only, and every later slice is too until an office reader carries a type
size.** This is not a policy line; it is the type system's, and D4 §5's sentence applies unchanged:
*"the profile says why, and the type system makes it unnecessary to say twice."*

## 7. What is measured before the rule ships

### 7.1 The two instruments

| Instrument | What it answers | Corpus |
| --- | --- | --- |
| `docs/measurements/headings/mhs.py`, plus the bench harness's own `score.py` | Does MHS move, and what is its band with the worst document named | the 200 `opendataloader-bench` documents, 107 scored |
| `docs/measurements/headings/falsepos.py` | On documents that declare headings, how often does the rule fire where the author's tree says the text is not a heading | the **eleven** documents of §7.1.1 |

#### 7.1.1 The population that can answer the false-positive question — eleven documents

**Measured 2026-09-18 with the build at `a18b2b0` by the parent session**, counting runs whose
`structural_locator.pdf_tagged.role_path` holds `H` or `H1`..`H6`, and the distinct
`(mcid, role_path)` pairs as an approximation of heading elements:

| corpus | PDFs | carry a tree | **declare a heading role** | heading items, per document |
| --- | ---: | ---: | ---: | --- |
| `fixtures/gate` | 8 | 8 | **8** | `-37r2` ~276, `-161r1` ~147, `-171r3` ~87, `-53Ar5` ~23, `-207` ~18, `irs-fw9` ~28, `irs-f1040sd-2025` ~8, `-218` ~2 |
| `ethos-oracle/benchmarks/gate-zero` | 10 | 4 | **3** | `nist-sp-800-53r5` ~187, `cfpb-home-loan-toolkit` ~63, `irs-form-1040-2025` ~15 |
| `fixtures/engine` | 59 | 11 | **0** | — |
| `ethos-oracle/fixtures` | 35 | 0 | 0 | — |
| `opendataloader-bench` | 200 | 0 | 0 | — |

Heading-carrying runs behind those item counts: `-37r2` 56,121, `-161r1` 12,768, `-171r3` 3,831,
`-53Ar5` 1,798, `-207` 992, `-218` 68, `irs-fw9` 31, `irs-f1040sd-2025` 11; `nist-sp-800-53r5`
10,790, `cfpb-home-loan-toolkit` 1,036, `irs-form-1040-2025` 27. These are **run** counts over whole
`role_path`s and are not the line counts of §7.3, which exclude `/Artifact` and whitespace-only runs
and resolve each line to one innermost block-level role — a different unit, deliberately, and the
two are not compared.

**Three things this census settles.**

**The false-positive check has a population of eleven documents**, and it is the only such set in
reach: 8 gate plus 3 gate-zero, from **2 to 276** heading items. Every other corpus this repository
can read is useless for the question — `fixtures/engine` has 11 trees and **not one heading role**,
and `ethos-oracle/fixtures` and the bench corpus have no tree at all.

**Two of the eleven have denominators too small to carry a rate**, and §7.5 says so where it sets the
bound: `irs-f1040sd-2025` at ~8 items and `nist-sp-800-218` at ~2. On a document with two declared
heading items a single false positive moves the rate by tens of points, so a per-document bound
applied to them measures noise. §7.5 handles that by counting, not by excluding them.

**The 48 untagged engine fixtures plus 35 oracle fixtures are where the rule fires in the test
suite**, and none of them can label it. That is why the bound is measured on the eleven and the
acceptance tests of §10 S1 are hand-written fixtures rather than corpus documents.

Neither exists in the tree. The probes behind §7.2 and §7.3 were written for this scope and are not
committed; **S3 and S4 commit the instruments, never the corpora**, as
`measurements/auto-tagging/README.md` already requires of its own.

**`score.py` needs two one-line changes and they are part of S4**, because it cannot publish the
band decision #18 asks for: line 63–64 call `band()` for NID and TEDS and not for MHS, and line 70
prints the sentence *"MHS is 0 by construction: headings come from a tag tree, and 0 of 200
documents have one."* Add `band(mhs, "MHS")`; delete the sentence, which decision #29 falsified.

**The baseline, measured at 0.58.0 and quoted from the bench README rather than re-derived:**
**MHS 0.0000**, n = **107**. Its mechanism, read off the evaluator: `evaluate_heading_level` returns
`(None, None)` when the ground truth holds no heading — which excludes 93 documents from the
denominator — and `(0.0, 0.0)` when the ground truth holds one and the prediction holds none, which
is every one of the 107 today.

### 7.2 The recall ceiling, on the proxy — what the rule can reach at best

Measured 2026-09-18. Unit: the line, whitespace-only runs skipped, lines under 4 stripped characters
skipped. Signal: the line's box height over the document's char-weighted modal line box height,
binned to 10 centipoints. Labels: the harness's own ground-truth `#` lines, matched to an extract
line by alphanumeric-squashed containment. **107 documents, 180 of the 193 ground-truth headings
located** in the extract, 13 unlocated because the extracted text differs from the label (for
example `7variants ofsj observer models` against `7 Variants of SJ Observer Models` — a missing
space, not a missing run). **3,682 candidate lines.**

| rule | recall of located headings | lines fired | precision |
| --- | ---: | ---: | ---: |
| size ≥ 1.15× | **55.6%** | 248 | 40.3% |
| size ≥ 1.20× | 52.8% | 233 | 40.8% |
| a font the body text is not set in | **82.8%** | 443 | 33.6% |
| size ≥ 1.15× **or** a foreign font | **91.7%** | 534 | 30.9% |

**The ratio a real heading's line sits at**: band **0.749..3.748**, median **1.200**; p10 1.000,
p25 1.011, p75 1.557, p90 2.270. **54 of the 175 distinct lines carrying a located heading — 30.9% —
sit at or below 1.05×**, where no size threshold can reach them and a threshold that could would
fire on everything. Some sit *below* body height, which is the envelope term of §3.1 in the other
direction.

**The two signals fail in different places, and four documents show it:**

| document | ground-truth heading | its size ratio | its fonts | the body's font |
| --- | --- | ---: | --- | --- |
| `01030000000007` | Narratives in Chuj | 1.327 | `T1_0` — **the body font** | `T1_0`, 1,930 chars |
| `01030000000053` | Barriers to Filipino Women's Participation | 1.394 | `T1_0` (42 chars) | `C2_0`, 1,143 |
| `01030000000001` | 7 Variants of SJ Observer Models | **1.011** | `T1_1` (30 chars — the font's entire use in the document) | `T1_0`, 2,709 |
| `01030000000013` | 4 Al-Sadu Symbols and Social Significance | **1.018** | `T1_1` (33) + `T1_2` (7) | `T1_0`, 1,958 |

On the first two the type is larger and the font unremarkable; on the last two the type is the body's
and the font is one the body never uses. **That is row 29's *font size or font name* measured**, and
it says the disjunction is the recall. §7.3 says what the disjunction costs.

Raising the minimum stripped characters from 4 to 8: the size clause moves to 54.2% recall at 44.9%
precision, the disjunction to 88.8% at 34.0%.

**What "precision" here is and is not.** It is against the harness's ground-truth Markdown, which
labels 193 headings across 200 pages of scholarly typesetting and labels only the paper's own
section headings — not a figure caption, a table title, an author line, a journal stamp or a running
head. A fired line the ground truth does not label may be a heading the ground truth omits. **So
this column is a lower bound on precision, and it is not the false-positive rate row 29 asks for.**
That rate is defined against a document's own declared structure, and §7.3 measures it there.

### 7.3 The false-positive rate, measured where the document declares headings

**Method, and it needs no tree-stripped twin.** The rule's inputs — the rendered em and the font —
do not depend on the structure tree, so the rate at which the rule would call a heading something
the author declared otherwise is fixed by the untouched original. That is docs/23 §7.2's argument
reused: *"the rate … is fully determined by the untouched original."* The labels are already on the
wire: the innermost **block-level** role of the run's `pdf_tagged` locator, `standard_role_path`
after the `/RoleMap`, walking upward past inline-level elements — the correction
`19-BLOCK-SUBDIVISION-SCOPE.md` §3 error 2 records, and the join `paragraphs.py` already performs.
A **false positive** is a line the rule fires on whose declared block-level role is not `H` or
`H1`..`H6`. The denominator is the lines carrying a declared block-level role, with whitespace-only
and `/Artifact` lines excluded first, as every instrument here excludes them.

**A twin is still needed for one thing**, and it is a test rather than a measurement: a gate
document with its `/StructTreeRoot` removed through `lopdf` inside the test, to prove the shipped
build actually fires there and that the predicate the instrument computes is the predicate the code
computes. `stripping_the_attribute_launders_the_tag` (docs/24 S3) is the shape.

**Measured 2026-09-18, five of the eleven documents §7.1.1 names.** `nist-sp-800-207`,
`nist-sp-800-161r1` and `nist-sp-800-53Ar5` were not run: their artifacts do not fit the probe's
memory without the streaming `paragraphs.py` already has. The three gate-zero documents were not run
either: the probe took the eight gate PDFs by path before the census existed. Lines of at least 8
stripped characters, `/Artifact` excluded.

**The naive form — the line's *maximum* box height against the document's modal:**

| document | labelled lines | declared heading lines | recall | false positives | FP rate |
| --- | ---: | ---: | ---: | ---: | ---: |
| `irs-fw9` | 656 | 19 | 94.7% | 7 | 1.07% |
| `irs-f1040sd-2025` | 105 | 6 | 16.7% | 0 | 0.00% |
| `nist-sp-800-218` | 1,798 | 7 | 100.0% | **528** | **29.37%** |

**The worst case has one cause and it is the envelope, not the rule.** `nist-sp-800-218`'s
char-weighted box-height distribution has two populations: **1010 cp, 59.1% of its characters, font
`TT0`**, and **1330 cp, 28.7%, font `TT1`**. 1330/1010 = **1.317**. `TT1`'s ordinary body text is
1.32× the modal box height because its ascent-to-descent envelope is taller, not because its type
is bigger — so a box-height threshold fires on all 27,182 of its characters. **This is the
measurement that rejects the box height as the signal** (§3.1) and the reason the shipped rule reads
`em_scale_pt`.

**Per-font-normalised — each line's char-weighted mean of (its run's box height ÷ that font's own
modal box height), over fonts carrying at least 200 characters.** Within one font that ratio *is* the
ratio of rendered ems and carries no envelope term, so this is the closest an artifact alone comes to
the shipped signal:

| document | size ≥ 1.25× alone | | size ≥ 1.25× **or** foreign font | |
| --- | ---: | ---: | ---: | ---: |
| | recall | FP rate | recall | FP rate |
| `irs-fw9` | 94.7% | 0.76% | 100.0% | 4.57% |
| `irs-f1040sd-2025` | 0.0% | 0.00% | 16.7% | 0.00% |
| `nist-sp-800-218` | 100.0% | **4.84%** | 100.0% | 10.51% |
| `nist-sp-800-171r3` | 3.3% | 0.13% | 100.0% | **15.76%** |
| `nist-sp-800-37r2` | 3.8% | 0.72% | 7.7% | 6.68% |
| **band, worst named** | | **0.00%..4.84%, `nist-sp-800-218`** | | **0.00%..15.76%, `nist-sp-800-171r3`** |

**Read this table for the false-positive column and not for the recall column.** Recall here is
against the producers' own `/H` labels, and `19-BLOCK-SUBDIVISION-SCOPE.md` §9.1 already established
that these producers write structure differently from one another — `nist-sp-800-171r3` declares 180
heading lines and sets them at body type, which is not the rule failing but the producer's
typography. The false-positive column is the one row 29 asks for and it is the one the bound is set
against.

**The finding, in one line: the size clause sits inside a 5% band and the font clause is three times
outside it.**

### 7.4 What is unmeasured, and what each needs

1. **The shipped rule's recall and false-positive rate.** Unmeasured. Needs slice S1 — the rendered
   em reaching the rule — because every number in §7.2 and §7.3 is on the box-height proxy, whose
   envelope term is precisely what produced §7.3's worst case. The proxy says the bound is
   reachable; it does not say the rule meets it.
2. **MHS under the rule.** Unmeasured. Needs S2 and a bench run. It will rise above 0.0000 on
   arithmetic alone — any heading at all leaves the evaluator's `(0.0, 0.0)` early exit — so
   **"MHS above 0.0000" is not an acceptance bar** and §7.5 does not use it as one. C1's success
   criterion the owner's task list carries reads that way, and §11 settles that §7.5's four bars
   govern instead.
3. **Six of the eleven documents §7.1.1 names.** `nist-sp-800-207`, `nist-sp-800-161r1` and
   `nist-sp-800-53Ar5` need the streaming reader `paragraphs.py` already has; `nist-sp-800-53r5`,
   `cfpb-home-loan-toolkit` and `irs-form-1040-2025` need only adding to the instrument's path list.
   `nist-sp-800-161r1` (~147 heading items) and `nist-sp-800-53r5` (~187) are the two largest
   denominators in the set and therefore the two whose rates will bind; `nist-sp-800-207` is the one
   document `19-BLOCK-SUBDIVISION-SCOPE.md` §9.1 proved carries real paragraph labels.

### 7.5 The acceptance bar, set before the code

**The rule ships only if all four hold.**

1. **The false-positive rate is at most 5% on every one of the eleven documents §7.1.1 names**,
   published as a band with the worst document named. Per-document and not pooled, because decision
   #18 forbids a macro over a population this shape and `19-BLOCK-SUBDIVISION-SCOPE.md` §4.2
   measured a 68-point per-document spread for the analogous rule.

   **And the two small denominators are counted, never excluded.** `irs-f1040sd-2025` (~8 heading
   items) and `nist-sp-800-218` (~2) cannot carry a rate: one false positive moves `-218`'s by tens
   of points, so the bound applied to them measures noise rather than the rule. They are reported as
   **absolute counts beside their rates**, they are named in the band as the documents whose rate is
   not readable, and a breach on one of them is an owner decision rather than an automatic refusal.
   The nine documents with three or more heading items carry the bound. Excluding the two would be
   `19-BLOCK-SUBDIVISION-SCOPE.md` §8 rule 4's failure — *a band the rule cannot read says so*
   rather than being folded into an average, and dropping it entirely is the version of that rule
   nobody may take.
2. **MHS rises, and falls on no document**, against the 0.58.0 per-document baseline. Vacuous at the
   first measurement, because every scored document is at 0.0000 and nothing can fall; binding at
   every measurement after. Said rather than left implied.
3. **No document that declares structure changes at all** beyond its profile hash: same Markdown,
   same HTML, same grounding, byte for byte. This is the rider *a declared heading always wins* as a
   test rather than a claim, and §7.1.1 gives it a population: the 8 gate documents, the 4
   tree-carrying gate-zero documents, the 11 tree-carrying engine fixtures, and the 129 tagged
   round-trip documents.
4. **The engine-tagged round trip still holds**: docs/23 §4.3's equality between a tagged document's
   projections and its untagged original's (§6.1).

**Why 5%, reasoned rather than asserted.**

- **It is the shape of the evidence, not a number chosen to pass.** The size clause's measured band
  on five gate documents is 0.00%..4.84%. The bound sits just above the worst measured value, so a
  rule that behaves as the proxy suggests clears it and a rule that behaves materially worse does
  not. It is also not slack: adding the font clause takes the band to 15.76%, three times the bound,
  so the bound discriminates between the two candidate rules — which is the whole job of a bound set
  before the code.
- **Why not 0%.** The block cut shipped at 100% precision over 719 chances and that is the standard
  to aim at, but its denominator was one document's paragraph pairs. This rule's denominator is
  every line of every tagged document, and `19-BLOCK-SUBDIVISION-SCOPE.md` §9.1 measured producers
  whose `/P` is one per line (79.2% of `nist-sp-800-218`'s line pairs cross a `/P`). Such a producer
  labels a real display line `/P`, and a 0% bound would refuse the rule on the producer's labelling
  rather than on the rule's error.
- **Why not 10%.** At 10% the rule could emit, on `nist-sp-800-171r3`'s 3,954 labelled lines, up to
  395 headings the author's tree contradicts — more than twice the 180 heading lines that document
  declares. **A rule that can fabricate more headings than the document declares is not evidence**,
  and contract §10's line between a recorded event and a fabrication is where that lands.

**What re-refuses the rule**, which is row 29's own condition made numeric: a measured
false-positive rate above 5% on any of the **nine** documents of §7.1.1 that carry three or more
declared heading items, on the shipped signal. And, after it ships, the same measurement on any
corpus this repository acquires — the check is one run of the instrument, worth running on every new
corpus for the reason `19-BLOCK-SUBDIVISION-SCOPE.md` §10 gives for `layout_attrs.py`, and the more
so here because **eleven documents from two producers is a thin population** and §7.1.1 shows
nothing else in reach can widen it.

## 8. What is refused, each by name

| Input or idea | Answer | Why |
| --- | --- | --- |
| A document that declares any structure | **no inference at all** | Row 29's rider. The reader gates on `structure::read` returning `Ok(None)` (§6.1), so there is nothing for a declared heading to win against |
| A heading level beyond `h1` | refused, on measurement | The harness's evaluator treats all levels as equivalent and all 193 ground-truth headings are level 1 (§3.5). Reopens on a corpus whose ground truth uses more than one level |
| A heading from `TextRunAttributes.font_size` | refused, on measurement | One distinct value on 87 of 200 bench documents and on all four gate documents measured (§3.1). The field is the `Tf` operand, not the type size |
| A heading from the ink box's height | refused, on measurement | The box carries the font's envelope: 29.37% false positives on `nist-sp-800-218`, from one font 1.32× taller at the same type (§7.3). Also outside the fingerprint, also absent on 7.9% of bench text nodes |
| A heading from a `region` or a `block` | refused | Row 29 says this row reads type, not position; D4 §11 rule 1 and `19-BLOCK-SUBDIVISION-SCOPE.md` §6. A test bans both field names from the rule's source, the shape `reading_order.rs`:1213 already uses for `font_size` |
| A heading from a text prefix — `Chapter 3`, `1.2 Scope`, `TABLE 4` | refused | P14 is *"style and role inferred from font names **or text prefixes**"*, and row 29 narrowed it for type only. Reopens on an owner decision, recorded there first |
| Any other role — paragraph, list, caption, section, figure, table | refused | P14 stands (row 29 rider 3). **D1 stays undecided** and nothing here is an argument for it |
| A bold rendered by the content stream — `Tr 2`, a doubled `Tj` | refused | Nothing reads it, and reading it would infer weight from painting rather than from a declaration (§3.1) |
| A heading on a line the page marked `/Artifact` | refused | §14.8.2.2 — the author said it is furniture, which is a declaration about that line |
| A heading inside a table the detector accepted | refused | A cell is not a heading, and the erasure a GFM header row already claims is enough of a structural claim in one projection |
| A confidence, a score, a near-miss or an ordering of candidates | refused | Contract §9. A line clears the threshold or it does not |
| An office format | refused, by the type system | No office attribute variant carries a type size (§6.2). Not a policy line |
| `tag` writing `/H1`, `/H`, or any heading element | refused | docs/23 §3.2 and §5: the writer writes `/Div` under `/Document` and no other role. **Crossing the two features is refused here so that a later slice cannot do it quietly**, and taking it is D1 plus an owner decision |
| An inferred heading in `ethos.grounding.v1` | not refused — impossible | The shape carries no roles at all (contract §11 item 3) |
| An outline, a table of contents, a numbering, a nesting | refused | Not asked for by row 29, and each is a second rule with no corpus to refuse it. YAGNI |

## 9. Documents and tests that say the opposite today

Listed with line numbers because each is a standing statement of the refusal row 29 reversed, and
each must be amended by dated correction rather than deleted — decision #2's rule about the record.

| Where | What it says | What it becomes |
| --- | --- | --- |
| `markdown.rs`:94–95 (module header) | *"A heading is a heading because the structure tree said so. No font size is consulted anywhere in this file — checklist L29 is REFUSE"* | Both halves become false: a font size is consulted in the reader and the projection reads its verdict. Amended with row 29 named |
| `markdown.rs`:620–625 (`heading_level`) | *"No font size is consulted. Checklist L29 is REFUSE, and a heading inferred from 14pt bold is a claim about layout that no code in this repository makes"* | The sentence *"a heading is a heading because the document said so"* narrows to *"or because this engine measured its type, and the artifact says which"* |
| `markdown.rs`:116, 119–121 (the rule-id rustdoc) | *"headings still only from the structure tree"*, and *"A run that projected headings from font sizes and a run that refused to would disagree"* — the second is now the **argument for** moving the id | The first is amended; the second is kept and quoted in §4 |
| `markdown.rs`:686–687 (`list_role`) | *"the same mistake as reading a heading off a font size"* | Narrowed: a list is still refused, and the analogy loses its second half |
| `markdown.rs`:3167–3174, `a_big_font_is_not_a_heading` | Asserts that a 2400-centipoint run with no role projects as a paragraph | **Rewritten, never deleted.** It becomes *a big font is not a heading where the document declares structure*, with a new sibling asserting that it is one where the document declares none. A deleted refusal test is a refusal nobody can see was reversed |
| `html.rs`:307 | *"`<h1>`–`<h6>` when the structure tree says so, `<p>` otherwise. No font size is read"* | Names both sources |
| `structure.rs`:19–33 (module header) | *"no heading is deduced from a font size, no table from a `\"Table 3:\"` prefix. That inference is the parity checklist's P14"* | **Stays true of `structure.rs`**, and gains one sentence: the inference lives outside this module by design, and this module's job — reading what the document declared — is what gates it |
| `representation.rs`:935–936 (`StructuralLocator`) | *"Nothing here is inferred: no role is guessed from a font size"* | **Stays true**, and is the reason §5.1 refuses a new locator variant |
| `docs/CAPABILITY.md`:24, 51 | *"**Headings project only where the document declares a level**"*, and *"Paragraph structure comes from the tag tree or not at all"* | Row 24 gains the inferred case with its bound and its band; row 51 is untouched — **a paragraph is still refused** |
| `docs/measurements/opendataloader-bench/README.md`:207–215 | Already carries the dated reversal. Its *"The 0.0000 above stands as measured at 0.58.0 until the rule ships and MHS is re-measured"* is the sentence S4 discharges | A dated appendix with the new band |
| `docs/06-STEAL-REFUSE.md`:55 (P14) | Already narrowed, 2026-09-17 | No change |
| `docs/16-D4-SCOPE.md`:273–276 (standing rule 1) | Already narrowed, 2026-09-17 | No change |

**One test is a hazard rather than a statement.** `reading_order.rs`:1213,
`the_rule_reads_no_font_size_and_counts_nothing`, bans the string `font_size` from the
reading-order rule's own source. It scans `reading_order.rs` only and is unaffected — and it is the
model for the guard §8 asks for: the heading rule's source should ban `region` and `block` by the
same word-scan, with the same honest caveat that comment carries about what a word-scan can and
cannot prove.

## 10. Slices

In the shape of [`24-AUTO-TAGGING-MILESTONES.md`](24-AUTO-TAGGING-MILESTONES.md): every PR belongs
to exactly one, and S1 lands before S2 for that document's own reason — *a reader tested against a
projection that shares its mistake is a test that cannot fail.*

| Slice | Theme | Moves the profile? |
| --- | --- | --- |
| **S0** | This document | no |
| **S1** | The reader: the rendered em reaching the rule, `inferred_heading` on the wire, `heading_inference_rule` in the profile, the declaration, the gate, the fixtures | **yes** |
| **S2** | The projections: `heading_level`'s third arm, `markdown_rule` and `html_rule` to `-v8`, the rewritten refusal test | **yes** |
| **S3** | The false-positive instrument, the tree-stripped twin test, and §7.5's bound checked | no |
| **S4** | MHS re-measured, `score.py`'s band, and every document of §9 | no |
| **S5** | *Conditional.* The font-name clause, its own band, its own bound | **yes** |

### S1 — the reader

**Files.** `crates/ethos-parser-pdf/src/headings.rs` (new: the rule, its constants, its guard test),
`crates/ethos-parser-pdf/src/extract.rs` (call it after `arrange_page`, where `region` and `block`
are filled, and gate it on `structure::read`'s answer), `crates/ethos-parser-pdf/src/nodes.rs`
(`TextRun.inferred_heading`), `crates/ethos-parser-pdf/src/represent.rs` (carry it onto the wire),
`crates/ethos-parser-core/src/representation.rs` (`TextRunAttributes.inferred_heading`),
`crates/ethos-parser-core/src/profile.rs` (`HEADING_INFERENCE_RULE_V1`, the field, the default, every
office profile's `NOT_RUN`, the pin), `crates/ethos-parser-core/src/assurance.rs`
(`codes::HEADINGS_INFERRED_FROM_TYPE`), `crates/ethos-parser-pdf/src/limitations.rs`
(`headings_inferred_from_type`), `fixtures/engine/make_fixtures.py` and `fixtures/manifest.json`,
both draft schemas, `docs/PUBLIC-API.md`.

**Acceptance.**

- `a_display_line_on_an_untagged_page_is_an_inferred_heading`: a hand-written fixture — one line at
  twice the body em above five body lines, no `/StructTreeRoot` — sets `inferred_heading` on exactly
  the display line's runs and on no other, and the artifact declares
  `headings-inferred-from-type` naming 1 line, the rule id and the body reference.
- `the_same_page_with_a_tree_infers_nothing`: the same page plus a minimal `/StructTreeRoot` sets
  the field nowhere and declares neither the new code nor `untagged-structure-tree-absent`.
- `a_tf_operand_of_one_still_finds_the_heading`: the fixture rewritten to carry its type size in
  `Tm` with `/F1 1 Tf`, so `font_size` is 100 on every run, reads identically. **This is §3.1's
  measurement as a test**, and it is the one test that would catch a later edit reaching for the
  operand.
- `a_tall_glyph_does_not_make_a_line_a_heading`: a body line with one 4× glyph sets the field
  nowhere (§3.3).
- `an_artifact_line_is_never_an_inferred_heading`, and `a_table_cell_is_never_one`.
- `the_rule_reads_no_region_and_no_block`: the §9 word-scan over `headings.rs`.
- `every_profile_field_is_hash_sensitive` destructures the new field;
  `the_default_profile_is_pinned` re-pinned from its own output with a move-log paragraph.
- Every tagged gate document's extract artifact is byte-identical to 0.58.0's apart from the profile
  hash and the new profile field — acceptance bar 3, at the representation.

**Amended 2026-09-18, on landing: S1 is done.** The rule is `crates/ethos-parser-pdf/src/headings.rs`,
wired into extraction; `inferred_heading` is on both the extract and the representation wire,
absent where false; `heading_inference_rule: type-size-v1` is on the profile (re-pinned to
`sha256:e62a6fad…`, with the move-log paragraph decision #30 had owed since `locate_rule`); and
three fixtures hold it — `heading-display-line`, its twin under a `/StructTreeRoot` citing
nothing, and the same page drawn under `/F1 1 Tf`. What was measured, and four places the
implementation differs from or adds to the text above:

- **Acceptance bar 3 is met.** All eight gate documents — 1.7 GB of representations — are
  byte-identical to the previous build's except `profile_sha256` and the fingerprint covering it
  (118 to 125 differing bytes each, every one inside those two digests). No existing geometry
  digest moved, and the rule fires on **none** of the 58 pre-existing engine fixtures that
  extract (the 59th, `tagged-cycle`, is refused by design) — the five engine-tagged ones the
  gate's second arm opens included — and on exactly the two new fixtures built for it.
- **The line is formed by the caller, not in `headings.rs`.** §3.3's line key includes the band,
  and §9's guard bans that word from the rule's file — the two are reconciled by the caller
  grouping runs into lines and the rule reading each line's runs by type alone. The guard holds
  truthfully, and `EM_BIN` is re-declared rather than imported for the same reason. Each page
  reduces its lines to three facts (the minimum measurable em, whether any run is text, whether
  any is excluded), because the verdict waits for the fold: the body em is a mode over every page.
- **The gate is the whole of §6.1, both arms.** No `/StructTreeRoot`, or a tree every element of
  which this engine's writer created — tested by tagging the heading page with `write_tags` and
  requiring the same inference as its original. One author element closes the gate, which is
  stricter than §6.1's *no `Extracted` binding* only on a mixed tree, a shape the writer never
  produces.
- **§9's table of statements #29 falsifies missed one, and it rides on the wire.**
  `untagged-structure-tree-absent`'s detail said *"Nothing is inferred to fill the gap: a heading
  guessed from a type size … would be indistinguishable on the wire from structure the author
  actually wrote"*. Both halves stopped being true, so the detail now says what is: no role path is
  inferred and no table is read from a caption, and a heading inferred from type is marked
  `inferred_heading` and declared — distinguishable on the wire, which was the objection. Every
  untagged PDF's artifact carries that detail, so its bytes move on documents where the rule fires
  nowhere too; a tagged document's do not.

### S2 — the projections

**Files.** `crates/ethos-parser-core/src/markdown.rs` (`heading_level`, the rule id, the rewritten
test and its new sibling, the module header), `crates/ethos-parser-core/src/html.rs` (the rule id,
its rustdoc), `crates/ethos-parser-core/src/profile.rs` (the two defaults), the CLI's projection
tests, `docs/CAPABILITY.md`.

**Acceptance.**

- `an_inferred_heading_projects_as_a_level_one_heading`: on S1's fixture, `markdown` is
  `# Display line\n\n…` and `html` is `<h1>Display line</h1>`, and the `#` plus its space is one
  `Syntax` segment with no `node_ids`.
- Every existing declared-heading test passes **unchanged**:
  `a_heading_role_from_the_tree_projects_as_a_heading`,
  `every_xhtml_heading_level_projects_at_its_own_depth`,
  `an_element_that_merely_looks_like_a_heading_stays_a_paragraph`,
  `a_non_heading_role_stays_a_paragraph`, and their two HTML twins.
- `a_big_font_is_not_a_heading_where_the_document_declares_structure`: the rewritten test.
- `the_projections_of_a_tagged_document_are_its_originals` (docs/23 §4.3) still passes — acceptance
  bar 4, and the test that §6.1's reading of the rider exists to keep passing.
- Each gate document's Markdown and HTML are byte-identical to 0.58.0's apart from the profile hash.

### S3 — the false-positive instrument and the bound

**Files.** `docs/measurements/headings/falsepos.py` and `README.md` (new),
`crates/ethos-parser-pdf/tests/` (the tree-stripped twin). The instrument takes the eleven documents
of §7.1.1 by path — the three gate-zero ones live under `ethos-oracle/benchmarks/gate-zero`, outside
`fixtures/manifest.json`'s roots, so they are named in the instrument and never in a `cargo test`
fixture list.

**Acceptance.** The band exists over all **eleven** documents of §7.1.1 — the eight gate PDFs and
the three gate-zero documents that declare a heading role — is published with its worst document
named, and **every one of the nine with three or more heading items is at or below 5%**, with
`irs-f1040sd-2025` and `nist-sp-800-218` reported as counts beside their unreadable rates. If one of
the nine is not, the rule does not ship and §7.5's reopening sentence applies — the slice's honest
outcome is a refusal with a number, as [`22-WORD-BOXES-SCOPE.md`](22-WORD-BOXES-SCOPE.md)'s was.

### S4 — MHS, and the documents

**Files.** `docs/measurements/opendataloader-bench/score.py` (the MHS band, the deleted sentence) and
`README.md` (a dated appendix), `docs/measurements/headings/README.md`, this document's amendments
block, and every row of §9.

**Acceptance.** MHS is published as a band with the worst document named; no document's MHS is below
its 0.58.0 value; `OPEN-WORK.md` §5 moves the item from *ready* to *shipped* and §4 loses or keeps
§11's settled calls.

### S5 — the font-name clause, conditional

**Not started, and not startable on this document's evidence.** §7.2 measures that it is where the
recall is (82.8% alone against the size clause's 55.6%, 91.7% in disjunction) and §7.3 that it is
three times outside the bound (0.00%..15.76% against 0.00%..4.84%). It needs a signal better than
*a font the body is not set in* — the font's own `/BaseFont` and `/FontDescriptor /Flags`, neither of
which reaches the wire today (§3.1) — and its own bound, set before its own code. §11 records
whether to scope it now or after S4's numbers.

## 11. What this document settled, and the one question left

**Settled here, each on a rule the repository already has or on a number measured above** — none is
a North Star reversal, and each says what would reopen it:

| | Settled | On what | Reopens on |
| --- | --- | --- | --- |
| *Font size* means **the type size the document draws**, read as the rendered em in the reader — not the field spelled `font_size` | The field has one distinct value on 87 of 200 bench documents, 41 of the 107 MHS-scored ones and all four gate documents measured (§3.1). Reading the row the other way ships a rule that reads a constant | A decision that `font_size` should carry the drawn size instead, which `OPEN-WORK.md` §4 already holds open as its own question |
| A tree **this engine wrote** is not the document declaring structure | `group_key`'s own argument, and docs/23 §4.3's measured equality between a tagged document's projections and its original's — which "yes" would break, weakening a test decision #25 rests on (§6.1) | An owner decision to amend that equality |
| `inferred_heading` is **absent where false**, with no `DerivationClass` of its own | D4's `region` and the block cut's `block` are optional attributes absent where the rule declined, and this is the same shape of layout-derived attribute; the class is carried by the field's identity, the profile's rule id and the document-scoped declaration (§5.1). The alternative, #25's constant-valued `derivation`, is recorded beside it — this is the second most arguable call here | A consumer that meets the field without this document and cannot tell what class it is |
| The **size clause alone** ships; the font clause is S5, conditional on its own measurement | Measured: size alone 55.6% recall at a 0.00%–4.84% false-positive band; the disjunction 91.7% at 0.00%–15.76%, three times §7.5's bound. Raising the bound to 20% would let the rule contradict the author's tree on 790 of `nist-sp-800-171r3`'s 3,954 labelled lines (§7.2, §7.3) | S5's own measurement on the shipped signal, with its own bound set before its code |
| §7.5's **four bars** govern, not "MHS above 0.0000" | MHS rises above zero on arithmetic alone, because the evaluator returns zero only when the prediction holds no heading at all, so that criterion is met by any rule that fires — including a wrong one (§7.4 item 2) | Nothing: a bar a wrong rule passes is not a bar |
| ODT's declared heading that projects as a paragraph is **out of scope** | It is a reader slice of its own — read `text:outline-level`, carry it, add the match arm, and settle what an absent attribute means in ODF (§6.2) | Recorded in `OPEN-WORK.md` §6 as a known defect, so it is not found later as a contradiction |

**The one question left, and it is already on the owner's list.** **Which roadmap version carries
this?** `OPEN-WORK.md` §4 holds it in these words: *"The local plan filed D2 under v2.2's steps,
whose gate #27 closed; decision #28 did not list it under v2.3."* The evidence: v2.2 is **met** on
#27, and adding work to a met version reopens a closed gate; v2.3 is *distribution*, and a heading
rule is not distribution; a row of its own is what #28's amended rule now permits, with a decision
recorded first. **This document names no version, and no slice below waits on the answer** — the
rule, its bound and its measurements are the same whichever row carries it.

**One number this document could not chase, recorded rather than left silent.** 13 of the 193
ground-truth headings could not be matched to any extract line (§7.2), on text differences such as a
missing space. Whether those are a reader defect, a ground-truth artefact or a matcher weakness is
unexamined, and S4's instrument should bucket them rather than drop them.

## 12. Standing rules, carried forward

1. **A heading is what the document declared, or what this engine measured and said so.** Never one
   passing for the other, on any surface that can tell them apart.
2. **A region and a block are where, never what** — P14, decision #19, and row 29's own rider. This
   rule reads neither, and a test says so.
3. **P14 stands for every other role.** No paragraph, list, caption, section, figure or table is
   inferred from type. D1 is untouched.
4. **No fixed multiplier presented as a measurement** —
   `19-BLOCK-SUBDIVISION-SCOPE.md` §8 rule 2. 1.20× is a round number inside an interval the
   evidence does not distinguish, and §7.5's bound is what the rule ships on.
5. **Every number ships as a band with the worst document named** — decision #18.
6. **A refusal is recorded with what reopens it**, so each is taken again on purpose rather than
   re-proposed from the row that named it.
7. **Anything that can change a byte of output is in the profile** — contract §2. Three ids move
   here and every golden regenerates.
8. **No confidence field**, on a heading or anywhere else — contract §9.
