# 11 — v1.1 slices

**Status:** implementation authority for v1.1 · **Scope document:** `10-V11-SCOPE.md`
**This is the code-review map for v1.1.** Every v1.1 PR belongs to exactly one slice.

**v1 is not done.** S7's gate is measured and **missed at 64‰** against a 489‰ floor
(`09-V1-MILESTONES.md` S7, `table-gate-v1.md`). v1.1 began because the owner asked for the next
roadmap row, and nothing in it closes v1.

| Slice | Theme | Depends on | State |
| --- | --- | --- | --- |
| **S0** | v1.1 scope + this document | — | **done** |
| **S1** | Linear Markdown + Anchor Map + coverage + the verify golden | S0 | **done** |
| **S2** | Tables and lists as Markdown, erasure declared (A14), still with the map | S1 | **done** |
| **S3** | HTML and/or export-only cosmetics, under the same map law | S1 | **not started** |

---

## S0 — v1.1 scope and slice map

- **Goal:** v1.1 exists as an ordered list of bounded changes **before** any of it is implemented,
  the same way `08-V1-SCOPE.md` and `09-V1-MILESTONES.md` bound v1.

- **In:** `10-V11-SCOPE.md` (what v1.1 is, what it is not, the four laws, Workbench rule 8 stated
  where a later slice will read it); this document.

- **Out:** any code.

- **Acceptance tests:**
  - [x] Both documents exist and name S2 and S3 as **not started**
  - [x] Rule 8 — *retrieval that ranks a projection while citations bind the source is where
        locators die silently* — is written down in `10-V11-SCOPE.md` §2, with O8's fallback
        (*prefer no projection at all*) quoted rather than paraphrased away
  - [x] `08-V1-SCOPE.md` and `09-V1-MILESTONES.md` still say S7 is missed, and now point here

- **Why first:** so a Markdown serializer cannot quietly acquire a heading heuristic, a second node
  id space, or an unmapped byte while nobody has written down that it may not.

---

## S1 — Linear Markdown, with the map that makes it citable

- **Goal:** the smallest Markdown that is honest. Linear text only, always paired with a map that
  inverts it, always with a census of what did not make it, and one end-to-end path where a quote
  taken out of the Markdown grounds through the **existing** Ethos verifier.

- **Status: done.** `ethos.markdown.v1` ships under `markdown-linear-v1` at **0.11.0**.

- **The artifact.** One canonical JSON, c14n, integer fields only:

  ```json
  {
    "artifact_type": "ethos.markdown.v1",
    "schema_version": "1.0.0",
    "parser_version": "0.11.0",
    "profile_sha256": "sha256:…",
    "source_sha256": "sha256:…",
    "representation_sha256": "sha256:…",
    "markdown_rule": "markdown-linear-v1",
    "markdown": "Hello Ethos\n",
    "anchor_map": {
      "segments": [
        {"kind": "source", "start": 0, "end": 11, "node_ids": ["…"]},
        {"kind": "syntax", "start": 11, "end": 12}
      ]
    },
    "coverage": {
      "source_chars_in_representation": 11,
      "source_chars_emitted": 11,
      "source_chars_dropped": 0,
      "dropped": []
    }
  }
  ```

  `start`/`end` are **UTF-8 byte offsets** into `markdown`, half-open, integers. Byte offsets rather
  than character indices because the consumer's job is to slice the string it was given, and every
  language's slice takes bytes or takes chars — saying which, in the field name and in this
  document, is the difference between a map and a hint.

- **Decision 1: the two halves are fields, not files.** A companion `.md` is a thing a pipeline
  strips. `markdown` and `anchor_map` are fields of one hashed artifact, and no code path in the
  workspace produces one without the other. Pinned by a test that greps the CLI surface for a
  Markdown-only exit.

- **Decision 2: the map tiles, and the tiling is checked on construction *and* on parse.**
  `AnchorMap::new` refuses a segment list that does not cover `0..markdown.len()` exactly — unsorted,
  overlapping, gapped, out-of-range, or empty-with-non-empty-markdown all fail closed with a named
  error. Parsing an artifact re-runs the same check, so a hand-edited file cannot smuggle a hole
  past the type.

- **Decision 3: what counts as a source character.** `coverage` counts **characters of node text**,
  not bytes and not glyphs, because the thing being conserved is *the text the representation
  offered*. `source_chars_in_representation` is the sum of `text.chars().count()` over every node.
  Emitted is what landed in a `source` segment; dropped is the rest, in named buckets.

- **Decision 4: which nodes project, and why the rest are buckets.** Only `text_run` nodes. The
  others are dropped **by kind**, each with its own bucket, because each is a different fact:

  | node kind | bucket | why |
  | --- | --- | --- |
  | `form_field` | `form-field-values-not-projected-v1` | a field's `/V` lives in the AcroForm tree; no content stream draws it. v1-S4 exists to keep it distinguishable from page text, and pasting it into a Markdown body would undo exactly that |
  | `annotation` | `annotation-text-not-projected-v1` | a reviewer's note is markup *over* the document. A consumer who cannot tell it from the page's own words cannot cite either safely |
  | `image` | `image-nodes-carry-no-text-v1` | a placement, not a picture. Zero characters, and a bucket that is almost always `0` — present so the census is exhaustive by construction rather than by the reader trusting that images have no text |

  **Page artifacts are projected, not dropped.** A running head carries
  `structural_locator: pdf_artifact` and is still a `text_run`, so it lands in the Markdown with its
  node id. Checklist O21/O22 is explicit that a reader deleting running heads has silently edited
  the document; the flag lives in the representation and a consumer that wants them gone can drop
  them *itself*, knowing it did.

- **Decision 5: tables are a declared erasure, not a dropped bucket.** A table cell's text is a
  concatenation of runs that are **already nodes** (`01-CONTRACT.md`, and the `tables` array's own
  doc comment). So projecting the runs loses **no character** — `coverage` is unaffected, and a
  `tables-not-projected` character bucket would be double-counting.

  What *is* lost is the **grid**: which run sat in which cell. That is a structural erasure and it
  gets a limitation, `markdown-table-structure-not-projected`, rather than a count of zero. Naming
  it as a character bucket would have been the more comfortable lie — a number that reads like a
  disclosure while the thing actually erased has no number at all. S2 is where a GFM table earns
  the right to flatten spans, and A14 obliges it to say what the flattening cost.

- **Decision 6: headings come from the tree or not at all.** A node projects as `# `…`###### ` when
  its `pdf_tagged` role path ends in `H`, or `H1`…`H6`, after the document's own `/RoleMap` is
  applied. Anything else is a paragraph. **No font size is consulted** — checklist L29 is REFUSE,
  and a heading inferred from 14pt bold is a claim about layout that no code here makes.

  Measured: **no fixture in either corpus carries a heading role**, so on everything committed today
  this branch never fires and every document projects as paragraphs. The branch is proved by a unit
  test over a hand-built representation instead of by a PDF nobody has, which is the honest way to
  test a path the corpus cannot reach.

- **Decision 7: the whitespace rule is the table gate's, and it is pinned.** Node text is emitted
  **NFC-normalized, with internal runs of Unicode whitespace collapsed to a single `U+0020`, and
  trimmed at the ends**. The same three clauses `table-gate-v1.md` publishes, for the same reason:
  a quote that has to match must be produced by a rule the consumer can reproduce.

  This means a `source` segment's bytes are not always byte-identical to `node.text` — they are its
  normalization. `the_published_whitespace_rule_is_the_one_the_projection_runs` pins each clause,
  and the artifact's own doc comment says so, because a map that claimed exact bytes and delivered
  normalized ones would be the subtlest possible lie.

- **Decision 8: where the code lives.** Anchor-map types and the projection are in **`engine-core`** —
  it is a projection of the representation and has nothing to do with PDF. `engine-pdf` does not
  learn Markdown. `engine-grounding` does not grow a Markdown schema. `engine-cli` gains
  `engine markdown` as a thin call. **No fifth crate** (`04-ARCHITECTURE.md` §1).

- **Decision 9: the CLI takes a representation, not a PDF.** `engine markdown <representation.json>`,
  the same shape as `engine ground`, which also reads the file `engine extract` writes. One input
  kind, documented, rather than a subcommand that silently means two different things. The
  fingerprint is checked before anything is projected, exactly as `ground` does: a representation
  that does not hash to its declared digest is not a record this engine will speak for.

- **The verify golden, which is the whole point of the slice.** Not a unit test — the real binaries:

  1. `engine extract` a fixture → representation
  2. `engine ground` it → `ethos.grounding.v1`
  3. `engine markdown` the same representation → `ethos.markdown.v1`
  4. lift a substring out of a **`source`** segment, make it a `quote` claim → `engine verify`
     reports it **grounded**
  5. lift a substring that touches a **`syntax`** segment → the verifier does **not** ground it

  Step 5 is the one that means something. On a two-node document the Markdown contains
  `First line\n\nSecond line`, and the string `line\n\nSecond` is *real text in the Markdown that
  the document never drew*. A consumer quoting from the `.md` alone cannot tell it from a sentence
  the page contains. With the map it is mechanical: the quote touches a `syntax` byte, so it is not
  invertible, and the verifier — reading only the grounding artifact, which knows nothing about
  Markdown — agrees by failing to ground it.

  Nothing is re-derived from the report (`07-VERIFY-BOUNDARY.md`). The engine asks and relays.

- **In:** `engine-core/src/markdown.rs` (types + projection); `markdown_rule` and
  `capabilities.markdown` on the profile; schema; `engine markdown`; PUBLIC-API; the verify golden.

- **Out:** GFM tables, task lists, `![]()`, HTML. Font-based heading detection. Any change to
  `extract`, the detectors, overlay or rasters. A Markdown quality comparison. A fifth crate.

- **Acceptance tests:**
  - [x] `engine markdown` on `simple-text` writes `ethos.markdown.v1`; two runs byte-identical
  - [x] The map tiles the markdown string — property test over generated segment lists **and** an
        exhaustive assert on the real fixtures
  - [x] `source_chars_emitted + source_chars_dropped == source_chars_in_representation`
  - [x] A `source` substring grounds through `engine verify`; a `syntax`-touching substring does not
  - [x] No CLI path prints Markdown without a map — asserted, not assumed
  - [x] `capabilities.markdown: true` has a named proof test; a profile without `markdown_rule`
        fails closed
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables
  - [x] `cargo test --workspace --locked` — **652 pass, 0 fail** — clippy `-D warnings`, `deny`,
        both grep gates, fmt

- **Depends on:** S0, and v1-S8 for the representation it projects.

---

## S2 — Tables and lists as Markdown

- **Goal:** a GFM table that says what it cost.

- **The problem it inherits:** the gold declares no spans at all on the v1 corpus, but real
  documents do, and GFM has no rowspan. Flattening one is an erasure, and **A14 requires the
  artifact to name it and quantify it** — not a footnote in a README.

- **Status: done.** `markdown-blocks-v1` at **0.12.0**. `markdown-linear-v1` is superseded and
  `markdown-table-structure-not-projected` is **deleted**, not reworded.

- **Decision 1: a cell's runs are emitted once.** A table is projected as GFM at the position of
  the first run one of its cells claims, and those runs are then **not** also emitted as
  paragraphs. The characters move from linear source to cell source; they are not duplicated and
  they are not dropped, so the census still balances and a document with no tables comes out
  byte-for-byte as it did at S1 — asserted on `simple-text` and `markdown-two-blocks` as literals,
  map geometry included.

- **Decision 2: the record had to carry the link, because a `source` segment must name a node.**
  `TableCellRecord` arrived at S1 holding a cell's text and **no** way back to the runs it is a
  concatenation of. The detector has always known — `DetectedCell::run_indices` — and the
  conversion threw it away. A consumer holding only the string can get back two ways and both are
  wrong: re-run the geometry, which is a second copy of the detector's rule that can drift from
  the first (checklist A12), or match the text, which is a guess the moment two cells hold the
  same word.

  So `TableCellRecord` gains `node_ids` and the representation goes to **0.5.0**. This is not
  detection — it is a fact the detector computed, carried across a boundary that used to drop it —
  and `DocumentRepresentation::seal` now refuses a record whose cell names a run it does not
  declare. Without it a GFM cell **cannot be emitted as `source` at all**, because `AnchorMap`
  refuses a source segment that names no node. The law forced the field.

- **Decision 3: the erasures are a second census, with integers.** GFM has no `rowspan`, no
  `colspan`, and no headerless table. Those are **structural** erasures: the text is all still
  there, so a dropped-character bucket for them would read `0`, which is A14's own example of a
  disclosure that discloses nothing. `coverage.structural_erasures` carries them as
  `{code, count}`, sorted, non-zero only:

  | code | counts |
  | --- | --- |
  | `gfm-span-slots-unrepresentable-v1` | slots a merge covered that GFM cannot say it covered — `rowspan × colspan - 1`, summed over the table's cells |
  | `gfm-row-zero-separator-v1` | **once per table**, because the delimiter row makes row 0 a header on every renderer and neither detector reads `/TH` |
  | `gfm-cell-run-claimed-twice-v1` | a run two cells both claimed, kept by the first so one node's characters are not counted twice |
  | `gfm-cell-not-placed-v1` | a cell outside the declared grid or on a taken slot; its runs still project, as paragraphs |
  | `gfm-table-not-projected-v1` | a table with zero rows or columns |
  | `gfm-list-item-run-joins-v1` | a body run appended to an already-open item |

  **Once per table, not once per cell in row 0** — pinned by a test. The erasure is one claim,
  *this table has a header*, made once about one table. Counting its cells would make a wide table
  look like a worse lie than a narrow one when both told exactly one.

- **Decision 4: trailing empties stay.** A serializer that truncates an empty last row or column
  makes a prettier table and a different document, and A14 names that erasure specifically.
  `markdown-table-cells` draws a row nobody wrote in, and it comes out as `|  |  |  |`.

- **Decision 5: the escape is syntax, the character it escapes is source.** GFM ends a cell at
  `|`, so a cell holding `A|B` is written `A\|B`. Emitting `A\|B` as one source segment would
  claim the document drew a backslash it never drew, and a consumer slicing that segment would get
  two characters where the page has one. So the backslash is `syntax` and the pipe stays `source`:
  a quote containing the pipe still inverts, one reaching back over the escape does not. `\` is
  escaped for the same reason — otherwise a cell whose text is literally `\|` would split at a
  character the document merely printed.

- **Decision 6: lists come from the tree or not at all.** A run whose role path ends in `Lbl`,
  `LBody` or `LI` **with an `/L` above it** becomes a list item; depth is the number of `/L` in the
  path. No bullet glyph, no hanging indent, no font name — the refusal L29 makes about font-size
  headings, one structure level up. An `Lbl` under `/TOCI` is not a list item, and that is pinned.

  The marker is always `- `, never `1. `: the representation carries no `/ListNumbering`, and
  choosing an ordered marker without one would be this exporter deciding the document meant a
  numbered list. Where the document *did* draw its own number it drew it as an `/Lbl`, which is
  source text — so the item reads `- 1. First item`. A doubled marker is ugly; deleting the
  document's own characters to make it pretty is the erasure A14 is about.

  **The one guess, counted.** PDF 32000 pairs one `/Lbl` with one `/LBody`, so a body run directly
  after its label is the same item by the standard. A *second* body run is different: two sibling
  `/LI`s have identical role paths, so nothing distinguishes "the rest of this item" from "the next
  item". The projection joins, and `gfm-list-item-run-joins-v1` says how often.

- **Two fixtures had to be authored**, for the reason `markdown-two-blocks` had to be at S1:

  - `markdown-table-cells` — a stroked 3×3 whose font declares **real ink metrics**, so its cell
    runs reach `ethos.grounding.v1` and a cell quote can be verified end to end. `ruled-table-grid`
    already has a merge and an empty cell, but declares no metrics, so the verifier would find
    nothing and refuse both halves of the golden — which proves nothing about cells. Carries the
    merge, the pipe, and the empty last row.
  - `tagged-list-items` — an `/L` / `/LI` / `/Lbl` / `/LBody` tree with a nested `/L` and one item
    whose body is two marked runs. **Neither corpus tags a list anywhere**, so this is the only
    document that reaches the branch at all. It carries no `/FontDescriptor` because `build_pdf`
    refuses a fixture that wants both a descriptor and a structure tree — they both claim object 6.

- **The cell-quote golden.** Same four binaries as S1's, one structure level up. A quote copied out
  of the **merged cell's origin** grounds; `North | merged span` — real text in the Markdown that
  the page never drew, because the document painted a ruling line and not a `|` — comes back
  **`text_mismatch`**. The reason is pinned, not just the verdict: `element_not_found` would mean
  the citation pointed at nothing and the test would pass without the Anchor Map having
  demonstrated anything.

- **In:** GFM tables and tagged lists in `engine-core/src/markdown.rs`; `node_ids` on
  `TableCellRecord` and the seal-time check that they resolve; `structural_erasures` on the
  artifact; `markdown-blocks-v1`; `markdown-table-spans-flattened` replacing
  `markdown-table-structure-not-projected`; two fixtures; schema, PUBLIC-API, CHANGELOG.

- **Out:** HTML. Hyphenation joining, dot-leaders, drop-caps — those are S3. Font-inferred lists or
  headings. Any change to `extract`, the three detection rules, or the table gate. A synthesized
  "Column 1" header the document did not write. Truncating an empty cell to look tidier. A fifth
  crate. A tag.

- **Acceptance tests:**
  - [x] Four laws on the S1 fixtures **and** every table fixture: the map tiles, two kinds only,
        one artifact, `emitted + dropped == in_representation`
  - [x] `simple-text` and `markdown-two-blocks` unchanged — body **and** map geometry, as literals
  - [x] S1's verify golden still grounds the source quote and still refuses the spanning one with
        `text_mismatch`
  - [x] A cell's characters appear **once**, not as a paragraph and again in the grid
  - [x] `gfm-span-slots-unrepresentable-v1` non-zero on the merged fixture, and the origin cell's
        text still grounds
  - [x] No trailing empty row or column dropped
  - [x] Untagged `simple-text` grows no `- ` markers; an `/Lbl` outside an `/L` is not a list item
  - [x] The default profile declares `markdown-table-spans-flattened` and **not**
        `markdown-table-structure-not-projected`
  - [x] Cell-quote golden: cell text grounds, table chrome does not, reason pinned
  - [x] Oracle still 12 / 3; table gate still **64‰**; `irs-form-1040-2025` still 0 tables;
        fabrication 0
  - [x] `cargo test --workspace --locked`, clippy `-D warnings`, `deny`, both grep gates, fmt

- **Depends on:** S1.

---

## S3 — HTML and/or export-only cosmetics — **not started**

- **Goal:** either an HTML projection under the same map law, or hyphenation joining / dot-leader
  removal / drop-cap merging as export-only transforms — each `syntax` or an invertible emit,
  never a rewrite of node text.

- **The standing constraint:** checklist O9. HTML gets the same map discipline or it does not ship.

- **Not started.** Starts when the owner asks.

---

## Standing rules for every v1.1 slice

Carried from `08-V1-SCOPE.md` §6 and `10-V11-SCOPE.md` §8, repeated because a projection is where
they get bent:

1. **No public confidence field.** A segment kind is typed vocabulary, never a score
2. **No box — and no role — derived from a font size**
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count
4. **No invented coordinate, identifier, fingerprint or pagination** — a projection mints no ids
5. **Fail closed, and distinguishably**
6. **Byte identity is a test**
7. **The Ethos tree is read-only**
8. **No verification** — Markdown does not acquire a verdict

## What is still open across the whole engine

Unchanged by v1.1, and listed here so a reader of this document does not mistake a shipped
projection for a finished engine:

- **The v1 table gate is missed at 64‰** against a 489‰ floor. `nist-sp-800-63b` and
  `nist-sp-800-53r5` both score 0‰ — they draw no table rulings at all, and several of their tagged
  tables are multi-page, which the page-granular join cannot match even in principle.
- **Page rasters** (`page_screenshots`) — no renderer this project may depend on.
- **Low-contrast detection** — no profile this build can produce reads colour at all.
- **Structure-order** — reading order is geometric; `/K` order is not a sorter.
- **Undrawn table edges are not supplied** — a form ruled under each cell comes back one row short.
