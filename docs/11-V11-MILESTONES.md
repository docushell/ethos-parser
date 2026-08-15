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
| **S2** | Tables and lists as Markdown, erasure declared (A14), still with the map | S1 | **not started** |
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

## S2 — Tables and lists as Markdown — **not started**

- **Goal:** a GFM table that says what it cost.

- **The problem it inherits:** the gold declares no spans at all on the v1 corpus, but real
  documents do, and GFM has no rowspan. Flattening one is an erasure, and **A14 requires the
  artifact to name it and quantify it** — not a footnote in a README.

- **Not started.** Starts when the owner asks.

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
