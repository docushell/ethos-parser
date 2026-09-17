# 11 — v1.1 slices

**Implementation authority for v1.1.** Scope is in [`10-V11-SCOPE.md`](10-V11-SCOPE.md), and every
v1.1 PR belongs to exactly one slice.

**v1 is not done.** Its gate is measured and missed, and the chase is parked, which is not a pass.
v1.1 began because the owner asked for the next roadmap row, and nothing in it closes v1.

**Superseded by decision #18 (2026-08-30):** v1 closed on four capability clauses and a published
band — the engine reads the tables a document declares, detects ruled tables where the producer
drew the rules, emits nothing where neither holds, and fabricates nothing — not on the macro. The
paragraph above was true when written and did not survive the decision; it is kept as written. See
[`00-NORTH-STAR.md`](../00-NORTH-STAR.md) row 18, which is where v1's status is stated.

Numbers quoted per slice below were measured on the **four-document** gate corpus current at the
time. The corpus later grew to twelve and the macro reads 70‰;
[`table-gate-v1.md`](../table-gate-v1.md) has the current number. The per-slice figures are left as
measured, because rewriting them would erase the evidence that the number moved.

| Slice | Theme | State |
| --- | --- | --- |
| **S0** | The scope document and this one | done |
| **S1** | Linear Markdown, the anchor map, coverage, the verify golden | done |
| **S2** | Tables and lists, with the erasure declared | done |
| **S3** | The hyphenation join, in the export only, counted | done |
| **S4** | HTML, under the same four laws | done |

---

## S0 — Scope and slice map

**Goal:** v1.1 exists as an ordered list of bounded changes **before** any of it is implemented.

**Why first:** so a Markdown serializer cannot quietly acquire a heading heuristic, a second node-id
space, or an unmapped byte while nobody has written down that it may not.

## S1 — Linear Markdown, with the map that makes it citable

**Goal:** the smallest Markdown that is honest. Linear text only, always paired with a map that
inverts it, always with a census of what did not make it, and one end-to-end path where a quote
taken out of the Markdown grounds through the **existing** verifier.

**The artifact** is one canonical JSON carrying the string, the map, the census, and the usual
identity fields. Segment offsets are **UTF-8 byte offsets, half-open** — bytes rather than character
indices because the consumer's job is to slice the string it was given, and **saying which, in the
field name and in the docs, is the difference between a map and a hint.**

**Decision 1: the two halves are fields, not files.** A companion `.md` is a thing a pipeline strips.
No code path in the workspace produces one without the other, pinned by a test that greps the CLI
surface for a Markdown-only exit.

**Decision 2: the map tiles, and the tiling is checked on construction *and* on parse.** Unsorted,
overlapping, gapped, out-of-range, or empty-with-non-empty-string all fail closed with a named error.
Re-checking on parse means **a hand-edited file cannot smuggle a hole past the type.**

**Decision 3: what counts as a source character.** The census counts **characters of node text**, not
bytes and not glyphs, because the thing being conserved is *the text the representation offered*.

**Decision 4: which nodes project, and why the rest are buckets.** Only text runs. The others are
dropped **by kind**, each with its own bucket, because each is a different fact:

| Node kind | Why not projected |
| --- | --- |
| Form field | Its value lives in the form tree; no content stream draws it. v1-S4 exists to keep it distinguishable from page text, and pasting it into a Markdown body would undo exactly that |
| Annotation | A reviewer's note is markup *over* the document. A consumer who cannot tell it from the page's own words cannot cite either safely |
| Image | A placement, not a picture. Zero characters, and a bucket that is almost always zero — present so the census is exhaustive **by construction** rather than by the reader trusting that images have no text |

**Page artifacts are projected, not dropped.** A running head is still a text run, so it lands in the
Markdown with its node id. **A reader deleting running heads has silently edited the document**; the
flag lives in the representation and a consumer that wants them gone can drop them *itself, knowing
it did*.

**Decision 5: tables are a declared erasure, not a dropped bucket.** A cell's text is a concatenation
of runs that are **already nodes**, so projecting the runs loses no character and a character bucket
for tables would be double-counting. What *is* lost is the **grid** — which run sat in which cell —
and that gets a limitation rather than a count of zero. **Naming it as a character bucket would have
been the more comfortable lie: a number that reads like a disclosure while the thing actually erased
has no number at all.**

**Decision 6: headings come from the tree or not at all.** A node becomes a heading when its role
path says so, after the document's own role map is applied. **No font size is consulted** — a heading
inferred from 14 pt bold is a claim about layout that no code here makes.

Measured: **no fixture in either corpus carries a heading role**, so on everything committed today
this branch never fires. It is proved by a unit test over a hand-built representation instead of by a
PDF nobody has, **which is the honest way to test a path the corpus cannot reach.**

**Decision 7: the whitespace rule is the table gate's, and it is pinned.** Text is emitted
NFC-normalized, with internal whitespace collapsed to one space, and trimmed. **A quote that has to
match must be produced by a rule the consumer can reproduce.**

This means a `source` segment's bytes are not always byte-identical to the node's text — they are its
normalization, and both the test and the artifact's own documentation say so, **because a map that
claimed exact bytes and delivered normalized ones would be the subtlest possible lie.**

**Decision 8: where the code lives.** The map types and the projection are in the core crate — it is
a projection of the representation and has nothing to do with PDF. **No fifth crate.**

**Decision 9: the CLI takes a representation, not a PDF**, the same shape as `ground`. One input kind
rather than a subcommand that silently means two different things — and the fingerprint is checked
before anything is projected.

**The verify golden, which is the whole point of the slice.** Not a unit test — the real binaries:
extract, ground, project, then lift a substring out of a `source` segment and watch the pinned
verifier call it **grounded**, and lift one touching a `syntax` segment and watch it **not**.

**Step five is the one that means something.** On a two-node document the Markdown contains `First
line\n\nSecond line`, and the string `line\n\nSecond` is **real text in the Markdown that the
document never drew.** A consumer quoting from the `.md` alone cannot tell it from a sentence the
page contains. With the map it is mechanical — the quote touches a `syntax` byte — and the verifier,
which knows nothing about Markdown, agrees by failing to ground it.

## S2 — Tables and lists as Markdown

**Goal:** a GFM table that says what it cost.

**The problem it inherits:** real documents merge cells and GFM has no `rowspan`. Flattening one is
an erasure, and the artifact must **name it and quantify it** — not footnote it in a README.

**Decision 1: a cell's runs are emitted once.** A table is projected at the position of the first run
one of its cells claims, and those runs are then not also emitted as paragraphs. The characters move
from linear source to cell source; they are not duplicated and not dropped, so the census still
balances and a document with no tables comes out byte-for-byte as it did at S1.

**Decision 2: the record had to carry the link, because a `source` segment must name a node.** The
cell record arrived at S1 holding a cell's text and **no way back** to the runs it is a concatenation
of. The detector always knew and the conversion threw it away.

A consumer holding only the string can get back two ways and both are wrong: **re-run the geometry,
which is a second copy of the detector's rule that can drift from the first, or match the text, which
is a guess the moment two cells hold the same word.** So the cell record gained node ids and the
representation moved a version. This is not detection — it is a fact the detector computed, carried
across a boundary that used to drop it — and the seal now refuses a record whose cell names a run it
does not declare. **The law forced the field:** without it a GFM cell could not be emitted as source
at all.

**Decision 3: the erasures are a second census, with integers.** GFM has no row or column spans and
no headerless table. Those are **structural** erasures — the text is all still there, so a
dropped-character bucket for them would read `0`, which is the example of a disclosure that discloses
nothing. They are counted as codes with counts:

| What is counted | Note |
| --- | --- |
| Slots a merge covered that GFM cannot say it covered | Summed over the table's cells |
| The header the delimiter row asserts | **Once per table**, not once per cell in row 0 |
| A run two cells both claimed | Kept by the first, so one node's characters are not counted twice |
| A cell outside the declared grid or on a taken slot | Its runs still project, as paragraphs |
| A table with zero rows or columns | — |
| A body run appended to an already-open list item | The one guess this slice makes |

**Once per table** is pinned by a test: the erasure is one claim — *this table has a header* — made
once about one table. **Counting its cells would make a wide table look like a worse lie than a
narrow one when both told exactly one.**

**Decision 4: trailing empties stay.** A serializer that truncates an empty last row makes a prettier
table and a different document.

**Decision 5: the escape is syntax, the character it escapes is source.** A cell holding `A|B` is
written `A\|B`. Emitting that as one source segment would claim the document drew a backslash it
never drew, and a consumer slicing it would get two characters where the page has one. So the
backslash is `syntax` and the pipe stays `source`: **a quote containing the pipe still inverts, one
reaching back over the escape does not.**

**Decision 6: lists come from the tree or not at all.** No bullet glyph, no hanging indent, no font
name — the same refusal made about font-size headings, one structure level up.

The marker is always a dash, never a number: the representation carries no list numbering, and
choosing an ordered marker without one would be this exporter deciding the document meant a numbered
list. Where the document *did* draw its own number it drew it as a label, which is source text — so
the item reads `- 1. First item`. **A doubled marker is ugly; deleting the document's own characters
to make it pretty is the erasure this rule is about.**

**The one guess, counted.** The standard pairs one label with one body, so a body run directly after
its label is the same item. A *second* body run is different: two sibling items have identical role
paths, so nothing distinguishes "the rest of this item" from "the next item". The projection joins,
and a code says how often.

**Two fixtures had to be authored.** One is a stroked 3×3 whose font declares **real ink metrics**,
so its cell runs reach the grounding artifact and a cell quote can be verified end to end — the
existing ruled fixture has a merge and an empty cell but declares no metrics, so the verifier would
find nothing and refuse both halves of the golden, **which proves nothing about cells.** The other is
a tagged list tree, because **neither corpus tags a list anywhere.**

**The cell-quote golden**, one structure level up from S1's: a quote from the merged cell's origin
grounds, and `North | merged span` — real text in the Markdown that the page never drew, because the
document painted a ruling line and not a pipe — comes back **`text_mismatch`**. **The reason is
pinned, not just the verdict:** a not-found result would mean the citation pointed at nothing and the
test would pass without the map having demonstrated anything.

## S3 — The hyphenation join, in the export only

**Goal:** a word the page broke across a line reads as one word in the Markdown, and the evidence
record is untouched.

**Why the row was split rather than half-ticked.** S3 was written as "HTML and/or cosmetics", and
those are two products: HTML is a second artifact owing the same four laws, and a hyphenation join is
a few lines in one function. Shipping both under one label would have made "S3 is done" unreadable.

**Dot-leaders and drop-caps are not implemented and have no fixture.** Inventing a collapse rule with
nothing to measure it against is the speculative work the standing rules refuse.

**The rule:** adjacent runs, same page, **on different baselines**, the first ending in a hyphen with
a letter in front and the second starting with a letter. The hyphen goes; nothing else does. Not
across a cell, a list item, a heading, or the page-furniture boundary; not where the dash is its own
word; and pairwise, so a word broken twice joins once.

**Two clauses were found by measuring, and each was a real defect.**

*The baseline clause is the whole rule.* Written as "adjacent runs, same page", it joins two
fragments of *one line*. A real document draws `non-escrowed` as a string of tiny runs at a single
baseline, so the rule produced **`nonescr`** — a compound hyphen the author wrote, deleted, and a
word that is not one. **It was the only place the rule fired on the entire benchmark corpus, and it
fired wrongly.**

A hyphen inside a line is a hyphen the author wrote. Only a hyphen at a line's end is a candidate for
having been put there by the break. The test is deliberately conservative, **because a missed join
reads as the two words the page drew, and a wrong join invents one.**

*The furniture clause covers the one block boundary no other clause can see.* The heading and list
checks both bail unless the locator is tagged, so a page-artifact run passes every other guard. A page
whose last body line ends in a soft hyphen and whose footer is the next node in reading order
projected **`recalcuConfidential`** — a word on no page, welded from two streams the document itself
declared separate.

That also broke a promise rule 1 makes out loud: artifacts are kept *so a consumer that wants them
gone drops them itself, knowing it did*, and the per-run source segment is the only handle for that.
**One segment spanning body text and a footer takes the handle away.** The test is equality, not
exclusion — a two-line running head hyphenates like any paragraph.

**The export joins; the evidence record does not.** Extract still emits both halves with the hyphen,
because telling a soft break-hyphen from a real compound one needs a dictionary, **and this project
does not guess in the record.**

So there is a quote that reads perfectly and **does not ground**. That is the correct answer and not
a verifier defect, and the artifact does not leave it to be inferred: the map holds one source segment
over the joined letters naming **both** runs, so the citable strings are recoverable; the removed
hyphen sits in a named character bucket; and the census still balances.

**A character bucket rather than a structural erasure**, and that is the whole test for which census a
disclosure belongs in: **a hyphen *is* a character of node text and it really is not in the Markdown.**
The GFM erasures are counted separately precisely because their characters are all still there.

**The golden, and the third fixture it needed.** S1's golden refused a quote spanning a blank line;
S2's refused one carrying table chrome. Both are *punctuation* a careful reader might squint at. S3
produces something harder — a joined word that is ordinary English in the middle of an ordinary
sentence:

```text
     page:  The rate may be recalcu-
            lated at closing
 markdown:  The rate may be recalculated at closing
```

**A model handed that Markdown would cite it without hesitation, and the page never drew it.** So the
golden runs the four real binaries: each half **grounds**, and the joined word comes back
**`text_mismatch`** — cited against an element that *does* exist, so the verifier finds it and judges
the text. A not-found result would have passed the test without examining the cosmetic at all.

The existing hyphen fixture could not carry this, because its font declares no ink metrics, so the
grounding artifact comes out empty and the golden would watch the verifier refuse every quote. Two
runs 30 points apart in a metrics font is what makes the question askable at all.

## S4 — HTML, under the same four laws

**Goal:** an HTML projection carrying the same map discipline. **It gets the four laws or it does not
ship.** The Markdown rule id did not move: the Markdown a document produces is byte-for-byte what the
previous release emitted.

**Why this is a slice and not a stylesheet.** A second artifact would not be worth it if it were the
first with angle brackets. It is worth one for exactly one reason: **GFM cannot say `rowspan`, and
HTML can.** Where Markdown must expand a merged cell and count what that cost, HTML emits one
`<td colspan="2">` and the covered slot emits nothing.

**The two dropped erasure codes are dropped because HTML does not commit them — not because of their
names.** Three of the six describe faults in the *record* rather than limits of GFM, and HTML meets
those identically, so it keeps them. The span code is **recomputed** rather than dropped: a merge
costs HTML nothing *unless the grid cannot hold it*, and on a fixture whose row declares both a
spanning cell and an ordinary cell in the next slot, the span is clamped and the lost merge is
declared. Only the header code is unconditionally absent, because HTML asserts no header.

**No header cell appears anywhere in the projection.** No detector reads the header tag and the
representation carries no header declaration, so a header row would be this exporter deciding what
the document meant. **GFM had no such choice, which is why S2 owed a code for it and S4 does not.**

**The list-join erasure is on *both* artifacts**, and its GFM-shaped name is historical rather than
descriptive: the erasure belongs to the tagged tree and carries the name of the slice that first met
it. Renaming it would change what the Markdown artifact says under a rule id this slice deliberately
does not move, and **a rule id that stayed put while its output changed is the one dishonesty a
version id exists to prevent.**

**What is reused rather than rewritten:** everything that decides *what* to emit. The census is closed
by one shared function, so **the two artifacts cannot disagree** about how many characters a document
contains or which are dropped and why — asserted on every fixture, not just described. Their
structural erasures do differ, and that difference is the point.

**Two things are more than spelling**, and both follow from using the span instead of expanding it.
Lists need state: a Markdown item is a line, an HTML one is an element inside a list that must be
opened, nested and closed — tracked per level, because a tree that skips a depth otherwise closes an
item that was never opened. And a merge that collides with another cell's origin has to be clamped,
because GFM expands every merge and so never had to resolve the collision at all.

**Entities are emitted whole as `source`, and that differs from the pipe escape on purpose.** The
escape adds a byte beside a real one, so the backslash is syntax and the pipe is source. An entity
**replaces** the character — `&lt;` contains no `<` to label — so it is source, and the census is told
it stands for **one** character rather than four.

That last part is load-bearing. The census counts characters of node text, not emitted bytes; letting
an entity inflate the count would push emitted past what the representation holds. **Inverting a
source segment on this artifact therefore means HTML-unescaping it** — a total, lossless transform,
said out loud here, in the schema and in the module.

**A fragment, deliberately.** No document wrapper and no indentation. Those are bytes no document
drew; the map would tile them honestly as syntax, but they would sit inside quotes a consumer is
likely to lift. **Embedding a fragment is one concatenation; unwrapping a document is a parse.**

---

## Standing rules for every v1.1 slice

1. **No public confidence field.** A segment kind is typed vocabulary, never a score.
2. **No box, and no role, derived from a font size.**
3. **No silent drop and no silent repair** — a dropped character is a named bucket with a count.
4. **No invented coordinate, identifier, fingerprint or page number** — a projection mints no ids.
5. **Fail closed, and distinguishably.**
6. **Byte identity is a test.**
7. **The Ethos tree is read-only.**
8. **No verification** — Markdown does not acquire a verdict.

## What is still open across the whole engine

Listed here so a reader does not mistake a shipped projection for a finished engine:

- **The v1 table gate is missed**, and the chase for the comparator is parked. Two NIST documents
  score 0‰ — they draw no table rulings at all, and several of their tagged tables are multi-page,
  which a page-granular join cannot match even in principle.
- **Page rasters** — no renderer this project may depend on.
- **Low-contrast detection** — no profile this build can produce reads colour at all.
- **Structure-order reading** — reading order is geometric; tag order is not a sorter.
- **Undrawn table edges are not supplied** — a form ruled under each cell comes back one row short.
